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
        "sparsity.wav",
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

fn tone(n: usize, frequency: f64) -> Vec<f64> {
    (0..n)
        .map(|i| 0.4 * (std::f64::consts::TAU * frequency * i as f64 / 48000.).sin())
        .collect()
}

#[test]
fn flat_impulses_have_no_sparse_bins_at_every_supported_rate() {
    let mut mono = vec![0.; 16385];
    // Exactly one nonzero sample in each window: flat FFT magnitude, no comb.
    for i in (1024..mono.len()).step_by(4096) {
        mono[i] = 0.5;
    }
    for rate in [8000, 32000, 44100, 48000, 96000, 384000] {
        let r = run(&mono, 1, rate, None);
        let s = &r.sparsity[0];
        assert_eq!(s.status, DetectorStatus::Measured);
        assert_eq!(s.eligible_frames, 7);
        assert_eq!(s.bin_count, 2047);
        assert_eq!(s.sparse_bin_observations, Some(0));
        assert_eq!(s.total_bin_observations, Some(7 * 2047));
        assert_eq!(s.fraction, Some(0.));
        assert_eq!(s.lower_bin_hz, Some(rate as f64 / 4096.));
        assert_eq!(s.upper_bin_hz, Some(2047. * rate as f64 / 4096.));
    }
}

#[test]
fn natural_tones_are_sparse_with_native_antiphase_and_gain() {
    let mono = tone(48000, 6000.);
    let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -0.5 * x]).collect();
    let r = run(&stereo, 2, 48000, None);
    for s in &r.sparsity {
        assert_eq!(s.status, DetectorStatus::Measured);
        assert!(s.fraction.unwrap() > 0.9, "{s:?}");
        assert!(s.upper_bin_hz.unwrap() < s.cutoff_p95_hz.unwrap());
    }
    assert_eq!(r.sparsity[0].fraction, r.sparsity[1].fraction);
    assert!(
        r.detectors
            .iter()
            .filter(|d| d.id == "below_cutoff_sparsity")
            .all(|d| d.status == DetectorStatus::Measured && d.family == "spectral_measurements")
    );
    let parsed: AnalysisReport = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(
        parsed.sparsity[0].sparse_bin_observations,
        r.sparsity[0].sparse_bin_observations
    );
    assert!((parsed.sparsity[0].fraction.unwrap() - r.sparsity[0].fraction.unwrap()).abs() < 1e-14);
}

#[test]
fn silence_quiet_and_insufficient_bandwidth_abstain() {
    for mono in [
        vec![0.; 48000],
        tone(48000, 6000.).iter().map(|x| x * 1e-7).collect(),
        vec![0.25; 48000],
    ] {
        let r = run(&mono, 1, 48000, None);
        let s = &r.sparsity[0];
        assert_eq!(s.status, DetectorStatus::Inconclusive, "{s:?}");
        assert_eq!(s.fraction, None);
        assert_eq!(s.sparse_bin_observations, None);
        assert_eq!(s.total_bin_observations, None);
        assert_eq!(
            s.active_frames,
            s.eligible_frames + s.below_peak_floor_frames
        );
    }
}

#[test]
fn strict_prefix_and_minimum_frames_preserve_coverage() {
    let mono = tone(48000, 6000.);
    for count in [0u64, 3, 4] {
        let length = if count == 0 {
            4096
        } else {
            4096 + (count - 1) * 2048 + 1
        };
        let r = run(&mono, 1, 48000, Some(length as f64 / 48000.));
        let s = &r.sparsity[0];
        assert_eq!(s.stft_frames, count);
        assert_eq!(s.eligible_frames, count);
        assert_eq!(s.fraction.is_some(), count >= 4);
        assert_eq!(
            s.interval.as_ref().map(|x| x.end_frame),
            (count > 0).then_some(length - 1)
        );
    }
}

#[test]
fn global_cutoff_gaps_and_silent_channel_are_explicit() {
    let mut mono = tone(3 * 48000, 1000.);
    mono[48000..96000].fill(0.);
    mono[96000..].copy_from_slice(&tone(48000, 18000.));
    let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, 0.]).collect();
    let r = run(&stereo, 2, 48000, None);
    let s = &r.sparsity[0];
    assert!(s.active_frames < s.stft_frames);
    assert!(s.cutoff_p95_hz.unwrap() >= 18000.);
    assert_eq!(s.cutoff_p95_hz, r.channels[0].spectral.cutoff_p95_hz);
    assert_eq!(
        s.total_bin_observations,
        Some(s.eligible_frames * s.bin_count as u64)
    );
    assert_eq!(r.sparsity[1].eligible_frames, 0);
    assert_eq!(r.sparsity[1].cutoff_p95_hz, None);
    assert_eq!(r.sparsity[1].fraction, None);
}
