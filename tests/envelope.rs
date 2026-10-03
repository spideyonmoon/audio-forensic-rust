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
        "envelopes.wav",
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

fn tones(rate: usize, seconds: usize, mode: i32) -> Vec<f64> {
    (0..rate * seconds)
        .map(|i| {
            let t = i as f64 / rate as f64;
            let modulation = 0.12 * (std::f64::consts::TAU * t).sin();
            let low = if mode == 0 { 0.2 } else { 0.2 + modulation };
            let high = match mode {
                -1 => 0.2 - modulation,
                0 => 0.2,
                2 => 0.,
                _ => low,
            };
            low * (std::f64::consts::TAU * 3000. * t).sin()
                + high * (std::f64::consts::TAU * 18000. * t).sin()
        })
        .collect()
}

#[test]
fn signed_correlation_preserves_native_antiphase_and_gain() {
    for mode in [-1, 1] {
        let mono = tones(48000, 3, mode);
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -x * 0.5]).collect();
        let r = run(&stereo, 2, 48000, None);
        for e in &r.envelope {
            assert_eq!(e.status, DetectorStatus::Measured);
            assert!(
                (e.coefficient.unwrap() - mode as f64).abs() < 0.001,
                "{e:?}"
            );
            assert!(e.mid_band.energy_eligible && e.high_band.variation_eligible);
        }
        assert!(
            (r.envelope[0].coefficient.unwrap() - r.envelope[1].coefficient.unwrap()).abs() < 1e-10
        );
        let roundtrip: AnalysisReport =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(
            roundtrip.envelope[0].active_frames,
            r.envelope[0].active_frames
        );
        assert!(
            (roundtrip.envelope[0].coefficient.unwrap() - r.envelope[0].coefficient.unwrap()).abs()
                < 1e-14
        );
    }
}

#[test]
fn silent_constant_quiet_and_missing_band_envelopes_abstain() {
    for mono in [
        vec![0.; 48000],
        tones(48000, 1, 0),
        tones(48000, 1, 2),
        tones(48000, 1, 1).iter().map(|&x| x * 1e-6).collect(),
    ] {
        let r = run(&mono, 1, 48000, None);
        assert_eq!(r.envelope[0].status, DetectorStatus::Inconclusive);
        assert_eq!(r.envelope[0].coefficient, None);
    }
}

#[test]
fn complete_bands_and_minimum_active_frame_count_are_required() {
    for rate in [8000u32, 32000, 44000, 44001, 44100, 48000, 96000, 384000] {
        let mono = tones(rate as usize, 1, 1);
        let r = run(&mono, 1, rate, None);
        let e = &r.envelope[0];
        assert_eq!(
            e.status,
            if rate <= 44000 {
                DetectorStatus::Unsupported
            } else {
                DetectorStatus::Measured
            }
        );
        if rate > 44000 {
            assert!(e.coefficient.unwrap() > 0.999);
            assert!(e.high_band.lower_bin_hz.unwrap() >= 16000.);
            assert!(e.high_band.upper_bin_hz.unwrap() <= 22000.);
        }
    }
    let mono = tones(48000, 1, 1);
    for frames in [9u64, 10] {
        let length = 4096 + (frames - 1) * 2048 + 1;
        let r = run(&mono, 1, 48000, Some(length as f64 / 48000.));
        let e = &r.envelope[0];
        assert_eq!(e.active_frames, frames);
        assert_eq!(e.coefficient.is_some(), frames == 10);
        assert_eq!(e.interval.as_ref().unwrap().end_frame, length - 1);
    }
}

#[test]
fn inactive_gaps_and_a_silent_native_channel_are_explicit() {
    let mut mono = tones(48000, 3, 1);
    mono[48000..96000].fill(0.);
    let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, 0.]).collect();
    let r = run(&stereo, 2, 48000, None);
    assert!(r.envelope[0].active_frames < r.envelope[0].stft_frames);
    assert_eq!(r.envelope[0].status, DetectorStatus::Measured);
    assert!(r.envelope[0].coefficient.unwrap() > 0.999);
    assert_eq!(r.envelope[1].active_frames, 0);
    assert_eq!(r.envelope[1].coefficient, None);
    assert_eq!(r.envelope[1].mid_band.mean_rms, None);
}
