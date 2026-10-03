use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
};

fn run(samples: &[f64], channels: u16, rate: u32, max_seconds: Option<f64>) -> AnalysisReport {
    let bytes = samples.len() as u32 * 8;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(3u16.to_le_bytes());
    wav.extend(channels.to_le_bytes());
    wav.extend(rate.to_le_bytes());
    wav.extend((rate * u32::from(channels) * 8).to_le_bytes());
    wav.extend((channels * 8).to_le_bytes());
    wav.extend(64u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    for sample in samples {
        wav.extend(sample.to_le_bytes());
    }
    let report = analyze_source(
        Box::new(std::io::Cursor::new(wav)),
        "noise.wav",
        &AnalysisOptions {
            max_seconds,
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(report.evidence_index, None);
    report
}

#[test]
fn band_dynamics_match_analytic_tone_and_known_level_steps() {
    let rate = 48000usize;
    let mono: Vec<_> = (0..3 * rate)
        .map(|i| {
            let amplitude = [0.01, 0.02, 0.04][i / rate];
            amplitude * (std::f64::consts::TAU * 1500.0 * i as f64 / 4096.0).sin()
        })
        .collect();
    let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -x]).collect();
    let r = run(&stereo, 2, rate as u32, None);
    for n in &r.noise {
        for c in &n.high_band.correlations {
            assert_eq!(c.status, DetectorStatus::Measured);
            // A tone multiplied by periodic Hann has this normalized circular
            // autocorrelation. Symmetric Hann differs by O(1/N).
            let angle = std::f64::consts::TAU * f64::from(c.lag_frames) / 4096.0;
            let expected = (1500.0 * angle).cos() * (2.0 + angle.cos()) / 3.0;
            assert!((c.coefficient.unwrap() - expected).abs() < 0.003);
        }
        let t = &n.high_band.temporal_variation;
        assert_eq!(t.status, DetectorStatus::Measured);
        assert_eq!(t.blocks.len(), 3);
        assert_eq!(t.eligible_blocks, 3);
        let expected = 20.0 * 2.0f64.log10() * (2.0f64 / 3.0).sqrt();
        assert!((t.level_std_db.unwrap() - expected).abs() < 1e-5);
        for b in &t.blocks {
            assert!(b.interval.start_frame >= u64::from(b.block_index) * rate as u64);
            assert!(b.interval.end_frame <= (u64::from(b.block_index) + 1) * rate as u64);
            assert_eq!(b.interval.start_frame % 2048, 0);
            assert_eq!(
                b.interval.end_frame,
                b.interval.start_frame + (b.stft_frames - 1) * 2048 + 4096
            );
        }
    }
    assert_eq!(
        r.noise[0].high_band.correlations[0].coefficient,
        r.noise[1].high_band.correlations[0].coefficient
    );
}

#[test]
fn band_dynamics_keep_silence_and_partial_blocks_inconclusive() {
    let rate = 48000usize;
    for gain in [0.0, 1e-7] {
        let mono: Vec<_> = (0..2 * rate)
            .map(|i| gain * (std::f64::consts::TAU * 1500.0 * i as f64 / 4096.0).sin())
            .collect();
        let r = run(&mono, 1, rate as u32, None);
        let b = &r.noise[0].high_band;
        assert!(
            b.correlations
                .iter()
                .all(|c| c.status == DetectorStatus::Inconclusive && c.coefficient.is_none())
        );
        assert_eq!(b.temporal_variation.status, DetectorStatus::Inconclusive);
        assert_eq!(b.temporal_variation.level_std_db, None);
    }
    let mut mono: Vec<_> = (0..3 * rate)
        .map(|i| 0.01 * (std::f64::consts::TAU * 1500.0 * i as f64 / 4096.0).sin())
        .collect();
    let r = run(&mono, 1, rate as u32, Some(1.99));
    assert_eq!(r.noise[0].high_band.temporal_variation.blocks.len(), 1);
    assert_eq!(r.noise[0].high_band.temporal_variation.level_std_db, None);
    mono[rate..2 * rate].fill(0.0);
    let r = run(&mono, 1, rate as u32, None);
    let t = &r.noise[0].high_band.temporal_variation;
    assert_eq!(t.blocks.len(), 3);
    assert_eq!(t.eligible_blocks, 2);
    assert_eq!(t.level_std_db, None); // never erase a silent second to report stability
    assert_eq!(t.blocks[1].mean_square, Some(0.0));
    let r = run(&vec![0.0; 16000], 1, 8000, None);
    assert!(
        r.noise[0]
            .high_band
            .correlations
            .iter()
            .all(|c| c.status == DetectorStatus::Unsupported)
    );
    assert_eq!(
        r.noise[0].high_band.temporal_variation.status,
        DetectorStatus::Unsupported
    );
}

#[test]
fn temporal_history_stops_at_180_seconds_while_whole_stream_power_continues() {
    // Low rate keeps this duration-boundary regression inexpensive.
    let rate = 8000usize;
    let mono: Vec<_> = (0..182 * rate)
        .map(|i| if i < 180 * rate { 0.0 } else { 0.1 })
        .collect();
    let r = run(&mono, 1, rate as u32, None);
    let n = &r.noise[0];
    for b in [&n.high_band, &n.above_cutoff_band] {
        assert_eq!(b.temporal_variation.blocks.len(), 180);
        assert_eq!(b.temporal_variation.search_limit_frames, 180 * rate as u64);
        assert!(
            b.temporal_variation
                .blocks
                .iter()
                .all(|p| p.interval.end_frame <= 180 * rate as u64)
        );
    }
    assert_eq!(n.interval.end_frame, 182 * rate as u64);
    assert!(n.stft_interval.as_ref().unwrap().end_frame > 180 * rate as u64);
}

#[test]
fn runs_use_exact_threshold_duration_boundaries_and_native_channels() {
    // Exact -40 dBFS samples split runs; a lookahead sample cannot erase a window.
    let mut mono = vec![0.0; 24000];
    mono.push(0.01);
    mono.extend(vec![0.0; 23999]); // too short to qualify
    mono.push(-0.01);
    mono.extend(vec![0.001; 24001]); // flushed at EOF
    let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, 0.25]).collect();
    let report = run(&stereo, 2, 48000, None);
    let n = &report.noise[0];
    assert_eq!(
        (n.quiet_runs, n.quiet_samples, n.longest_quiet_run_frames),
        (2, 48001, 24001)
    );
    assert_eq!(
        n.quiet_intervals
            .iter()
            .map(|i| (i.start_frame, i.end_frame))
            .collect::<Vec<_>>(),
        vec![(0, 24000), (48001, 72002)]
    );
    let expected = (0..mono.len() - 4096)
        .step_by(2048)
        .filter(|&i| (i + 4096 <= 24000) || (i >= 48001 && i + 4096 <= 72002))
        .count();
    assert_eq!(n.quiet_stft_frames, expected as u64);
    assert_eq!(report.noise[1].quiet_runs, 0);
    assert_eq!(
        report.noise[1].high_band.quiet_status,
        DetectorStatus::Inconclusive
    );
    assert_eq!(report.noise[1].high_band.quiet_mean_square, None);
    let roundtrip: AnalysisReport =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(roundtrip.noise[0].quiet_samples, 48001);
    assert!(
        report
            .detectors
            .iter()
            .filter(|d| d.family == "noise_measurements")
            .all(|d| !matches!(d.status, DetectorStatus::Hit | DetectorStatus::NotDetected))
    );
}

#[test]
fn quiet_intervals_are_bounded_but_all_runs_are_counted() {
    let mono: Vec<_> = (0..18)
        .flat_map(|_| std::iter::repeat_n(0.0, 24000).chain([0.2]))
        .collect();
    let report = run(&mono, 1, 48000, None);
    let n = &report.noise[0];
    assert_eq!(n.quiet_runs, 18);
    assert_eq!(n.quiet_samples, 18 * 24000);
    assert_eq!(n.quiet_intervals.len(), 16);
    assert!(n.quiet_intervals_truncated);
    let expected = (0..mono.len() - 4096)
        .step_by(2048)
        .filter(|&i| i / 24001 == (i + 4095) / 24001 && (i + 4095) % 24001 < 24000)
        .count();
    assert_eq!(n.quiet_stft_frames, expected as u64);
    assert_eq!(n.high_band.quiet_mean_square, Some(0.0));
    assert_eq!(n.high_band.quiet_rms_dbfs, None);
}

#[test]
fn short_prefixes_silence_and_unavailable_bands_abstain_explicitly() {
    for len in [4096usize, 4097, 10240, 10241] {
        let r = run(&vec![0.0; len], 1, 48000, None);
        let n = &r.noise[0];
        let frames = len.saturating_sub(4096).div_ceil(2048) as u64;
        assert_eq!(n.stft_frames, frames);
        assert_eq!(
            n.stft_interval.as_ref().map(|x| x.end_frame),
            (frames > 0).then(|| (frames - 1) * 2048 + 4096)
        );
        assert_eq!(
            n.high_band.status,
            if frames >= 4 {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            }
        );
        assert_eq!(n.high_band.mean_square, (frames >= 4).then_some(0.0));
        assert_eq!(n.high_band.rms_dbfs, None);
        assert_eq!(n.high_band.quiet_status, DetectorStatus::Inconclusive);
        assert_eq!(n.above_cutoff_band.status, DetectorStatus::Unsupported);
    }
    for (length, expected) in [(24000, 0), (24001, 1)] {
        let r = run(&vec![0.0; 48001], 1, 48001, Some(length as f64 / 48001.0));
        assert_eq!(r.noise[0].quiet_runs, expected);
        assert_eq!(r.noise[0].minimum_quiet_run_frames, 24001);
        assert!(!r.coverage.unwrap().reached_end);
    }
    for rate in [8000, 16000, 32000] {
        let r = run(&vec![0.0; rate as usize], 1, rate, None);
        assert_eq!(r.noise[0].high_band.status, DetectorStatus::Unsupported);
        assert_eq!(
            r.noise[0].high_band.quiet_status,
            DetectorStatus::Unsupported
        );
        assert_eq!(r.noise[0].high_band.bin_count, 0);
        assert_eq!(r.noise[0].quiet_runs, 1);
    }
}

#[test]
fn band_normalization_matches_sine_power_and_antiphase_is_preserved() {
    for rate in [44100, 48000, 96000, 192000, 384000] {
        let bin = (18000.0 * 4096.0 / f64::from(rate)).round();
        let mono: Vec<_> = (0..rate)
            .map(|i| 0.004 * (std::f64::consts::TAU * bin * f64::from(i) / 4096.0).sin())
            .collect();
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -x]).collect();
        let r = run(&stereo, 2, rate, None);
        for n in &r.noise {
            assert_eq!(n.high_band.status, DetectorStatus::Measured);
            assert_eq!(n.high_band.quiet_status, DetectorStatus::Measured);
            assert!((n.high_band.mean_square.unwrap() - 0.004f64.powi(2) / 2.0).abs() < 1e-11);
            assert_eq!(n.high_band.mean_square, n.high_band.quiet_mean_square);
        }
        assert_eq!(
            r.noise[0].high_band.mean_square,
            r.noise[1].high_band.mean_square
        );
    }
}
