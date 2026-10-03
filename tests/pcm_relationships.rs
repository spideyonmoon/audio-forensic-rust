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
    for &x in samples {
        wav.extend(x.to_le_bytes());
    }
    let r = analyze_source(
        Box::new(std::io::Cursor::new(wav)),
        "pcm-relationships.wav",
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

fn sine(phase: f64) -> Vec<f64> {
    (0..8192)
        .map(|i| 0.5 * (std::f64::consts::TAU * i as f64 / 64.0 + phase).sin())
        .collect()
}

#[test]
fn sample_crest_matches_tone_square_dc_and_impulse_without_quality_labels() {
    let mut pulse = vec![0.0; 8192];
    pulse[0] = 0.5;
    for (mono, expected) in [
        (sine(0.), 2f64.sqrt()),
        (
            (0..8192)
                .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
                .collect(),
            1.0,
        ),
        (vec![0.25; 8192], 1.0),
        (pulse, 8192f64.sqrt()),
    ] {
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -x * 0.5]).collect();
        let r = run(&stereo, 2, 48000, None);
        for c in &r.channels {
            assert_eq!(c.crest_factor_status, DetectorStatus::Measured);
            assert!((c.crest_factor_linear.unwrap() - expected).abs() < 1e-12 * expected);
            assert!((c.crest_factor_db.unwrap() - 20.0 * expected.log10()).abs() < 1e-10);
        }
    }
    let r = run(&vec![0.; 8192], 1, 48000, None);
    assert_eq!(
        r.channels[0].crest_factor_status,
        DetectorStatus::Inconclusive
    );
    assert_eq!(r.channels[0].crest_factor_linear, None);
    assert_eq!(r.channels[0].crest_factor_db, None);
}

#[test]
fn stereo_centering_preserves_gain_dc_phase_and_sign() {
    let a = sine(0.);
    for phase in [
        0.,
        std::f64::consts::FRAC_PI_3,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    ] {
        let b = sine(phase);
        let samples: Vec<_> = a
            .iter()
            .zip(&b)
            .flat_map(|(&x, &y)| [x + 0.125, y * 0.5 - 0.25])
            .collect();
        let r = run(&samples, 2, 48000, None);
        let s = r.stereo_correlation.as_ref().unwrap();
        assert_eq!(s.status, DetectorStatus::Measured);
        assert!((s.coefficient.unwrap() - phase.cos()).abs() < 1e-12);
        assert!((s.mean[0].unwrap() - 0.125).abs() < 1e-14);
        assert!((s.std[0].unwrap() - 0.5 / 2f64.sqrt()).abs() < 1e-13);
        assert_eq!(s.pair_count, 8192);
        assert_eq!(s.interval.as_ref().unwrap().end_frame, 8192);
        let rt: AnalysisReport = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert!(
            (rt.stereo_correlation.unwrap().coefficient.unwrap() - s.coefficient.unwrap()).abs()
                < 1e-14
        );
    }
}

#[test]
fn mono_constant_quiet_and_dc_dominated_variation_abstain() {
    let a = sine(0.);
    let mono = run(&a, 1, 8001, None);
    let s = mono.stereo_correlation.unwrap();
    assert_eq!(s.status, DetectorStatus::Unsupported);
    assert_eq!(s.pair_count, 0);
    assert!(s.interval.is_none());
    for b in [
        vec![0.; a.len()],
        vec![0.25; a.len()],
        a.iter().map(|x| x * 1e-8).collect(),
        a.iter().map(|x| 0.5 + x * 1e-7).collect(),
    ] {
        let samples: Vec<_> = a.iter().zip(b).flat_map(|(&x, y)| [x, y]).collect();
        let r = run(&samples, 2, 48000, None);
        let s = r.stereo_correlation.unwrap();
        assert_eq!(s.status, DetectorStatus::Inconclusive);
        assert_eq!(s.coefficient, None);
        assert_eq!(s.variation_eligible, [true, false]);
    }
}

#[test]
fn tiny_float_rms_and_crest_survive_squaring_underflow() {
    let mono: Vec<_> = (0..8192)
        .map(|i| if i % 2 == 0 { 1e-200 } else { -1e-200 })
        .collect();
    let samples: Vec<_> = mono.iter().flat_map(|&x| [x, -x * 0.5]).collect();
    let r = run(&samples, 2, 48000, None);
    for (i, c) in r.channels.iter().enumerate() {
        assert!((c.rms / (1e-200 / (i + 1) as f64) - 1.0).abs() < 1e-14);
        assert_eq!(c.zero_samples, 0);
        assert_eq!(c.crest_factor_linear, Some(1.0));
    }
    let s = r.stereo_correlation.unwrap();
    assert_eq!(s.status, DetectorStatus::Inconclusive);
    assert!(s.std[0].unwrap() > 0.0);
    assert_eq!(s.coefficient, None);
}

#[test]
fn minimum_pairs_prefix_and_inactive_samples_use_original_coverage() {
    let mut samples = vec![0.25, -0.5, -0.25, 0.5];
    samples.resize(400, 0.);
    let one = run(&samples, 2, 8000, Some(1. / 8000.));
    assert_eq!(
        one.stereo_correlation.unwrap().status,
        DetectorStatus::Inconclusive
    );
    let two = run(&samples, 2, 8000, Some(2. / 8000.));
    assert!((two.stereo_correlation.unwrap().coefficient.unwrap() + 1.).abs() < 1e-14);
    let full = run(&samples, 2, 8000, None);
    assert_eq!(full.stereo_correlation.as_ref().unwrap().pair_count, 200);
    assert!((full.stereo_correlation.unwrap().coefficient.unwrap() + 1.).abs() < 1e-14);
    assert!((full.channels[0].crest_factor_linear.unwrap() - 100f64.sqrt()).abs() < 1e-12);
    let mut changed = samples.clone();
    changed[4..].fill(8.);
    let prefix = run(&changed, 2, 8000, Some(2. / 8000.));
    assert_eq!(
        prefix.coverage.unwrap().decoded_pcm_sha256,
        two.coverage.unwrap().decoded_pcm_sha256
    );
    assert_eq!(
        prefix.channels[0].crest_factor_linear,
        two.channels[0].crest_factor_linear
    );
}

#[test]
fn rms_rebases_for_later_larger_peaks() {
    let mut samples = vec![1e-200; 8192];
    samples[4096] = 0.5;
    let r = run(&samples, 1, 48000, None);
    assert!((r.channels[0].rms - 0.5 / 8192f64.sqrt()).abs() < 1e-15);
    assert!((r.channels[0].crest_factor_linear.unwrap() - 8192f64.sqrt()).abs() < 1e-12);
}
