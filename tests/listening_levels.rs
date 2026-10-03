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
        "listening-levels.wav",
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

fn tone(rate: u32, frames: usize, amplitude: f64, cycles_per_sample: f64, phase: f64) -> Vec<f64> {
    (0..frames)
        .map(|i| {
            let fade = ((i.min(frames - 1 - i) as f64) / (rate as f64 * 0.01)).min(1.);
            amplitude * fade * (std::f64::consts::TAU * cycles_per_sample * i as f64 + phase).sin()
        })
        .collect()
}

#[test]
fn phase_sensitive_peak_controls_recover_intersample_levels() {
    for (divisor, degrees, amplitude) in [
        (4., 0., 0.5),
        (4., 45., 0.5),
        (6., 60., 0.5),
        (8., 67.5, 0.5),
        (4., 45., 1.41),
    ] {
        let x = tone(
            48000,
            4800,
            amplitude,
            1. / divisor,
            degrees * std::f64::consts::PI / 180.,
        );
        let r = run(&x, 1, 48000, None);
        let p = &r.true_peak[0];
        let error = p.estimated_peak_dbtp.unwrap() - 20. * amplitude.log10();
        assert!(
            (-0.4..=0.2).contains(&error),
            "{divisor}/{degrees}: {error}"
        );
        assert!(p.estimated_peak >= p.sample_peak);
        assert_eq!(p.status, DetectorStatus::Measured);
        if degrees == 45. {
            assert!(p.estimated_peak / p.sample_peak > 1.4);
        }
    }
}

#[test]
fn native_gain_silence_tails_and_rate_coverage() {
    for rate in [8000, 11025, 44100, 48000, 96000, 192000, 384000] {
        let x: Vec<_> = tone(rate, 800, 0.5, 0.25, std::f64::consts::FRAC_PI_4)
            .into_iter()
            .flat_map(|x| [x, -x * 0.5])
            .collect();
        let r = run(&x, 2, rate, None);
        assert!((r.true_peak[0].estimated_peak - 2. * r.true_peak[1].estimated_peak).abs() < 1e-14);
        for p in &r.true_peak {
            assert_eq!(p.oversampled_rate_hz, 4 * rate);
            assert_eq!(p.interval.end_frame, 800);
            assert_eq!(p.zero_padding_frames, 11);
        }
    }
    let p = run(&[0., 0., 0.5], 1, 48000, None).true_peak.remove(0);
    assert_eq!(p.interpolated_peak, 0.5 * 0.97216796875);
    let p = run(&[0.; 10], 1, 48000, None).true_peak.remove(0);
    assert_eq!(p.estimated_peak, 0.);
    assert_eq!(p.estimated_peak_dbtp, None);
    let p = run(&[f64::MIN_POSITIVE], 1, 48000, None)
        .true_peak
        .remove(0);
    assert_eq!(p.estimated_peak, f64::MIN_POSITIVE);
    assert!(p.estimated_peak_dbtp.unwrap().is_finite());
}

#[test]
fn prefix_finishes_its_own_filter_tail_and_ignores_later_samples() {
    let mut x = vec![0.; 300];
    x.extend([0.4, 0.4, -0.4, -0.4]);
    let expected = run(&x, 1, 8000, None).true_peak.remove(0);
    x.extend(vec![0.9; 100]);
    let actual = run(&x, 1, 8000, Some(304.1 / 8000.)).true_peak.remove(0);
    assert_eq!(actual.estimated_peak, expected.estimated_peak);
    assert_eq!(actual.interpolated_peak, expected.interpolated_peak);
    assert_eq!(actual.interval.end_frame, 304);
}

fn levels(db: &[f64]) -> Vec<f64> {
    db.iter()
        .flat_map(|db| {
            (0..160000).map(move |i| {
                10f64.powf(db / 20.) * (std::f64::consts::TAU * 1000. * i as f64 / 8000.).sin()
            })
        })
        .collect()
}

#[test]
fn generated_ebu_level_steps_and_gain_have_expected_ranges() {
    for (db, expected) in [
        (vec![-20., -30.], 10.),
        (vec![-20., -15.], 5.),
        (vec![-40., -20.], 20.),
        (vec![-50., -35., -20., -35., -50.], 15.),
    ] {
        let x = levels(&db);
        let r = run(&x, 1, 8000, None);
        let a = &r.loudness.as_ref().unwrap().range;
        assert_eq!(a.status, DetectorStatus::Measured);
        assert!((a.range_lu.unwrap() - expected).abs() < 0.03, "{:?}", a);
        assert!(a.range_upper_lu.unwrap() - a.range_lower_lu.unwrap() <= 0.02000001);
        assert_eq!(a.histogram_overflow_windows, 0);
        let parsed: AnalysisReport =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(
            parsed.loudness.unwrap().range.relative_gated_windows,
            a.relative_gated_windows
        );
    }
}

#[test]
fn range_requires_two_complete_gated_windows_and_honors_prefix() {
    let mut x = levels(&[-20., -30.]);
    for frames in [23999, 24000, 24799, 24800] {
        let r = run(&x, 1, 8000, Some((frames as f64 + 0.1) / 8000.));
        let a = r.loudness.unwrap().range;
        assert_eq!(a.range_lu.is_some(), frames >= 24800);
    }
    let prefix = run(&x, 1, 8000, Some(20.)).loudness.unwrap().range;
    x.truncate(160000);
    let eof = run(&x, 1, 8000, None).loudness.unwrap().range;
    assert_eq!(prefix.range_lu, eof.range_lu);
    assert_eq!(prefix.relative_gated_windows, eof.relative_gated_windows);
    let silent = run(&vec![0.; 32000], 1, 8000, None).loudness.unwrap().range;
    assert_eq!(silent.status, DetectorStatus::Inconclusive);
    assert_eq!(silent.relative_gate_lufs, None);
    let unsupported = run(&vec![0.; 44100], 1, 11025, None)
        .loudness
        .unwrap()
        .range;
    assert_eq!(unsupported.status, DetectorStatus::Unsupported);
}
