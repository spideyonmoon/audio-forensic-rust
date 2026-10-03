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
        "loudness.wav",
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

fn tone(rate: u32, frames: usize, gain: f64) -> Vec<f64> {
    (0..frames)
        .map(|i| gain * (std::f64::consts::TAU * 997. * i as f64 / rate as f64).sin())
        .collect()
}

#[test]
fn itu_reference_tone_and_channel_power_addition() {
    let x = tone(48000, 48000, 0.1);
    let mono = run(&x, 1, 48000, None).loudness.unwrap();
    assert!((mono.integrated_lufs.unwrap() + 23.01).abs() < 0.02);
    assert_eq!(mono.status, DetectorStatus::Measured);
    for polarity in [-1., 1.] {
        let stereo: Vec<_> = x.iter().flat_map(|&v| [v, polarity * v]).collect();
        let l = run(&stereo, 2, 48000, None).loudness.unwrap();
        assert!(
            (l.integrated_lufs.unwrap() - mono.integrated_lufs.unwrap() - 10. * 2f64.log10()).abs()
                < 1e-10
        );
        assert_eq!(l.channel_weights, [1., 1.]);
    }
    let silent_right: Vec<_> = x.iter().flat_map(|&v| [v, 0.]).collect();
    let l = run(&silent_right, 2, 48000, None).loudness.unwrap();
    assert!((l.integrated_lufs.unwrap() - mono.integrated_lufs.unwrap()).abs() < 1e-10);
}

#[test]
fn full_windows_prefix_and_trailing_samples_are_explicit() {
    let rate = 8000;
    let x = tone(rate, 24800, 0.1);
    for frames in [1, 3199, 3200, 3201, 3999, 4000, 23999, 24000, 24800] {
        let r = run(&x, 1, rate, Some((frames as f64 + 0.1) / rate as f64));
        let l = r.loudness.unwrap();
        assert_eq!(l.analyzed_frames, frames);
        assert_eq!(l.complete_blocks, (frames / 800).saturating_sub(3));
        assert_eq!(l.short_term_windows, (frames / 800).saturating_sub(29));
        let end = if frames >= 3200 {
            frames / 800 * 800
        } else {
            0
        };
        assert_eq!(l.interval.map(|i| i.end_frame).unwrap_or(0), end);
        assert_eq!(l.trailing_frames, frames - end);
        assert_eq!(l.integrated_lufs.is_some(), frames >= 3200);
        assert_eq!(l.short_term_max_lufs.is_some(), frames >= 24000);
    }
}

#[test]
fn silence_and_sub_gate_signals_do_not_invent_integrated_levels() {
    for gain in [0., 1e-6, 1e-200] {
        let l = run(&tone(8000, 24000, gain), 1, 8000, None)
            .loudness
            .unwrap();
        assert_eq!(l.status, DetectorStatus::Inconclusive);
        assert_eq!(l.absolute_gated_blocks, 0);
        assert_eq!(l.relative_gated_blocks, 0);
        assert_eq!(l.relative_gate_lufs, None);
        assert_eq!(l.integrated_lufs, None);
        assert_eq!(l.momentary_max_lufs.is_some(), gain == 1e-6);
    }
}

#[test]
fn relative_gate_rejects_quiet_programme_and_gain_is_in_db() {
    let mut x = tone(8000, 32000, 0.1);
    x.extend(tone(8000, 32000, 0.001));
    let r = run(&x, 1, 8000, None);
    let a = r.loudness.as_ref().unwrap();
    assert_eq!(a.complete_blocks, 77);
    assert_eq!(a.absolute_gated_blocks, 77);
    assert!(a.relative_gated_blocks < 44);
    assert!(a.relative_gated_blocks >= 37);
    let quiet: Vec<_> = x.iter().map(|x| x * 0.5).collect();
    let b = run(&quiet, 1, 8000, None).loudness.unwrap();
    assert_eq!(a.relative_gated_blocks, b.relative_gated_blocks);
    for (a, b) in [
        (a.integrated_lufs, b.integrated_lufs),
        (a.momentary_max_lufs, b.momentary_max_lufs),
        (a.short_term_max_lufs, b.short_term_max_lufs),
        (a.relative_gate_lufs, b.relative_gate_lufs),
    ] {
        assert!((a.unwrap() - b.unwrap() - 20. * 2f64.log10()).abs() < 1e-9);
    }
    let json = serde_json::to_string(&r).unwrap();
    let parsed: AnalysisReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.loudness.unwrap().complete_blocks, 77);
    let d = r
        .detectors
        .iter()
        .find(|d| d.id == "programme_loudness")
        .unwrap();
    assert_eq!(d.channel_index, None);
    assert_eq!(d.family, "listening_level");
    assert!(!r.unimplemented_detectors.contains(&"true_peak".to_string()));
}

#[test]
fn filter_rate_range_and_unsupported_fractional_hops() {
    for rate in [8000, 11020, 22050, 44100, 48000, 96000, 192000, 384000] {
        let l = run(&tone(rate, (rate / 2) as usize, 0.1), 1, rate, None)
            .loudness
            .unwrap();
        assert_eq!(l.status, DetectorStatus::Measured);
        assert!(l.integrated_lufs.unwrap().is_finite());
        assert_eq!(l.complete_blocks, 2);
    }
    for rate in [8001, 11025, 44101, 383999] {
        let l = run(&vec![0.1; (rate / 2) as usize], 1, rate, None)
            .loudness
            .unwrap();
        assert_eq!(l.status, DetectorStatus::Unsupported);
        assert_eq!(l.hop_frames, None);
        assert_eq!(l.integrated_lufs, None);
        assert!(l.interval.is_none());
        assert_eq!(l.trailing_frames, u64::from(rate / 2));
    }
}

#[test]
fn prefix_loudness_excludes_later_loud_audio() {
    let x = tone(8000, 24000, 0.01);
    let expected = run(&x, 1, 8000, None).loudness.unwrap();
    let mut with_loud_tail = x;
    with_loud_tail.extend(tone(8000, 8000, 0.9));
    let prefix = run(&with_loud_tail, 1, 8000, Some(3.)).loudness.unwrap();
    assert_eq!(prefix.integrated_lufs, expected.integrated_lufs);
    assert_eq!(prefix.momentary_max_lufs, expected.momentary_max_lufs);
    assert_eq!(prefix.short_term_max_lufs, expected.short_term_max_lufs);
}
