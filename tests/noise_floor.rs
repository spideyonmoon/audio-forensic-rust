use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
};

fn run(samples: &[f64], channels: u16, rate: u32, limit: Option<f64>) -> AnalysisReport {
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
    for &sample in samples {
        wav.extend(sample.to_le_bytes());
    }
    let r = analyze_source(
        Box::new(std::io::Cursor::new(wav)),
        "noise-floor.wav",
        &AnalysisOptions {
            max_seconds: limit,
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
    assert_eq!(r.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(r.evidence_index, None);
    r
}

#[test]
fn original_quiet_indices_survive_interleaved_zero_blocks() {
    let mut x = Vec::new();
    for i in 0..50 {
        let value = if i % 2 == 0 {
            0.
        } else {
            (25 - i / 2) as f64 / 128.
        };
        x.extend(vec![value; 800]);
    }
    let r = run(&x, 1, 8000, None);
    let n = &r.noise_floor[0];
    assert_eq!(n.status, DetectorStatus::Measured);
    assert_eq!(
        (n.complete_blocks, n.zero_blocks, n.nonzero_blocks),
        (50, 25, 25)
    );
    assert_eq!(
        n.selected_blocks
            .iter()
            .map(|b| b.block_index)
            .collect::<Vec<_>>(),
        [45, 47, 49]
    );
    assert!((n.nonzero_rms_p015.unwrap() - 1.36 / 128.).abs() < 1e-14);
    assert!((n.nonzero_rms_p99.unwrap() - 24.76 / 128.).abs() < 1e-14);
    assert_eq!(n.color_status, DetectorStatus::Inconclusive);
    assert_eq!(n.high_minus_low_db, None);
    assert_eq!(n.interval.as_ref().unwrap().end_frame, 40000);
}

#[test]
fn native_antiphase_color_and_gain_remain_measurable() {
    // Both tones complete integer cycles within each 100 ms block.
    let x: Vec<_> = (0..96000)
        .flat_map(|i| {
            let t = i as f64 / 48000.;
            let v = 0.02 * (std::f64::consts::TAU * 1000. * t).sin()
                + 0.01 * (std::f64::consts::TAU * 18000. * t).sin();
            [v, -v * 0.5]
        })
        .collect();
    let r = run(&x, 2, 48000, None);
    for n in &r.noise_floor {
        assert_eq!(n.status, DetectorStatus::Measured);
        assert_eq!(n.color_status, DetectorStatus::Measured);
        let expected =
            10. * (0.25 * n.low_band.bin_count as f64 / n.high_band.bin_count as f64).log10();
        assert!((n.high_minus_low_db.unwrap() - expected).abs() < 1e-6);
        assert!(
            (n.low_band.mean_bin_power.unwrap() * n.low_band.bin_count as f64
                - 0.0002 / if n.channel_index == 0 { 1. } else { 4. })
            .abs()
                < 1e-10
        );
    }
    assert!(
        (r.noise_floor[0].nonzero_rms_p015_dbfs.unwrap()
            - r.noise_floor[1].nonzero_rms_p015_dbfs.unwrap()
            - 20. * 2f64.log10())
        .abs()
            < 1e-10
    );
    let parsed: AnalysisReport = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(
        parsed.noise_floor[0].selected_blocks[0].block_index,
        r.noise_floor[0].selected_blocks[0].block_index
    );
    assert_eq!(
        r.detectors
            .iter()
            .filter(|d| d.id == "quiet_block_profile")
            .count(),
        2
    );
}

#[test]
fn silent_channels_and_nineteen_nonzero_blocks_abstain() {
    let x: Vec<_> = (0..16000)
        .flat_map(|i| [if i < 15200 { 0.125 } else { 0. }, 0.])
        .collect();
    let r = run(&x, 2, 8000, None);
    for n in &r.noise_floor {
        assert_eq!(n.status, DetectorStatus::Inconclusive);
        assert_eq!(n.nonzero_rms_p015, None);
        assert!(n.selected_blocks.is_empty());
        assert_eq!(n.low_band.mean_bin_power, None);
    }
    assert_eq!(r.noise_floor[0].nonzero_blocks, 19);
    assert_eq!(r.noise_floor[1].zero_blocks, 20);
}

#[test]
fn complete_blocks_prefix_tails_and_ties_are_explicit() {
    for rate in [8000, 8001, 44100, 96000, 384000] {
        let block = (rate / 10) as usize;
        let x = vec![0.125; block * 21 + 7];
        for count in [0, 19, 20] {
            let limit = (block as f64 * count as f64 + 1.5) / rate as f64;
            let r = run(&x, 1, rate, Some(limit));
            let n = &r.noise_floor[0];
            assert_eq!(n.complete_blocks, count);
            // Same flooring convention as the shared decoder limit.
            assert_eq!(n.inspected_frames, (limit * rate as f64).floor() as u64);
            assert_eq!(n.trailing_frames, n.inspected_frames % block as u64);
            assert!(!n.coverage_capped);
            assert_eq!(n.interval.is_some(), count > 0);
            assert_eq!(
                n.status,
                if count >= 20 {
                    DetectorStatus::Measured
                } else {
                    DetectorStatus::Inconclusive
                }
            );
            if count == 20 {
                assert_eq!(
                    n.selected_blocks
                        .iter()
                        .map(|b| b.block_index)
                        .collect::<Vec<_>>(),
                    [0, 1, 2]
                );
            }
        }
    }
}

#[test]
fn survey_stops_at_three_hundred_blocks() {
    let x: Vec<_> = (0..800 * 301 + 3)
        .map(|i| if i < 800 * 300 { 0.125 } else { 0.001 })
        .collect();
    let r = run(&x, 1, 8000, None);
    let n = &r.noise_floor[0];
    assert!(n.coverage_capped);
    assert_eq!(n.complete_blocks, 300);
    assert_eq!(n.inspected_frames, 240000);
    assert_eq!(n.trailing_frames, 0);
    assert_eq!(n.nonzero_rms_p015, Some(0.125));
    assert_eq!(n.selected_blocks.len(), 30);
    assert_eq!(n.selected_blocks.last().unwrap().block_index, 29);
}

#[test]
fn tiny_float_pcm_is_not_mislabeled_as_exact_silence() {
    let r = run(&vec![1e-200; 16000], 1, 8000, None);
    let n = &r.noise_floor[0];
    assert_eq!(n.zero_blocks, 0);
    assert_eq!(n.rms_underflow_blocks, 0);
    assert_eq!(n.nonzero_rms_p015, Some(1e-200));
    assert_eq!(n.color_status, DetectorStatus::Inconclusive);
    let mut x = vec![0.; 16000];
    for i in (0..16000).step_by(800) {
        x[i] = f64::from_bits(1);
    }
    let r = run(&x, 1, 8000, None);
    let n = &r.noise_floor[0];
    assert_eq!(n.zero_blocks, 0);
    assert_eq!(n.rms_underflow_blocks, 20);
    assert_eq!(n.nonzero_blocks, 0);
    assert_eq!(n.status, DetectorStatus::Inconclusive);
}

#[test]
fn strict_band_edges_use_exact_rational_bin_membership() {
    // k=264 and k=360 are exactly 33% and 45% of an 800-sample FFT,
    // even though rounded Hz calculations can straddle those boundaries.
    for rate in [8001, 44101, 383990] {
        let n = (rate / 10) as usize;
        let r = run(&vec![0.125; n * 20], 1, rate, None);
        let p = &r.noise_floor[0];
        let first = (33 * n / 100) + 1;
        let last = (45 * n - 1) / 100;
        assert_eq!(p.high_band.bin_count, last - first + 1);
        assert_eq!(
            p.high_band.lower_bin_hz,
            Some(first as f64 * rate as f64 / n as f64)
        );
        assert_eq!(
            p.high_band.upper_bin_hz,
            Some(last as f64 * rate as f64 / n as f64)
        );
    }
}
