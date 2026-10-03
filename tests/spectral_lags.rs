use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
};
use rustfft::{FftPlanner, num_complex::Complex};

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
        "spectral-lags.wav",
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

fn shaped(periodic: bool) -> Vec<f64> {
    let mut bins = vec![Complex::default(); 4096];
    for k in 1..2048 {
        let db = if periodic {
            8.0 * (std::f64::consts::TAU * k as f64 / 64.0).cos()
        } else {
            -24.0 * k as f64 / 2048.0
        };
        bins[k] = Complex::from_polar(10f64.powf(db / 20.0), (k * 7919) as f64);
        bins[4096 - k] = bins[k].conj();
    }
    FftPlanner::new().plan_fft_inverse(4096).process(&mut bins);
    let peak = bins.iter().map(|x| x.re.abs()).fold(0.0, f64::max);
    (0..64000).map(|i| 0.7 * bins[i % 4096].re / peak).collect()
}

#[test]
fn periodic_and_eq_spectra_are_measurements_with_native_gain_and_sign() {
    for periodic in [true, false] {
        let mono = shaped(periodic);
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -x * 0.5]).collect();
        let r = run(&stereo, 2, 64000, None);
        let a = &r.spectral_lags[0];
        let b = &r.spectral_lags[1];
        assert_eq!(a.status, DetectorStatus::Measured);
        assert_eq!(b.status, DetectorStatus::Measured);
        assert!(a.lags[0].target.coefficient.unwrap() > 0.2);
        for (x, y) in a.lags.iter().zip(&b.lags) {
            assert_eq!(x.target.lag_hz, x.requested_lag_hz);
            assert!((x.target.coefficient.unwrap() - y.target.coefficient.unwrap()).abs() < 1e-10);
            assert_eq!(x.target.pair_count, a.bin_count - x.target.lag_bins);
            for neighbour in &x.neighbours {
                assert!(neighbour.coefficient.unwrap().abs() <= 1.0);
            }
        }
        let d = r
            .detectors
            .iter()
            .find(|d| d.id == "high_band_spectral_lags")
            .unwrap();
        assert_eq!(d.status, DetectorStatus::Measured);
        let json = serde_json::to_string(&r).unwrap();
        let roundtrip: AnalysisReport = serde_json::from_str(&json).unwrap();
        assert!(
            (roundtrip.spectral_lags[0].lags[0]
                .target
                .coefficient
                .unwrap()
                - a.lags[0].target.coefficient.unwrap())
            .abs()
                < 1e-14
        );
    }
}

#[test]
fn full_band_resolution_and_individual_lag_applicability_are_explicit() {
    for rate in [
        8000, 32000, 40000, 40001, 44100, 48000, 96000, 192000, 202271, 384000,
    ] {
        let r = run(&shaped(true), 1, rate, None);
        let s = &r.spectral_lags[0];
        let supported = rate > 40000 && s.bin_count >= 81;
        assert_eq!(
            s.status,
            if supported {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Unsupported
            }
        );
        if rate > 40000 {
            assert!(s.lower_bin_hz.unwrap() >= 16000.);
            assert!(s.upper_bin_hz.unwrap() < 20000.);
        } else {
            assert_eq!(s.lower_bin_hz, None);
        }
        for probe in &s.lags {
            for lag in std::iter::once(&probe.target).chain(&probe.neighbours) {
                assert_eq!(lag.coefficient.is_some(), supported && lag.pair_count >= 16);
                assert_eq!(lag.lag_hz, lag.lag_bins as f64 * rate as f64 / 4096.);
            }
        }
    }
}

#[test]
fn flat_silent_tiny_and_sparse_tone_spectra_abstain() {
    let mut flat = vec![0.; 64000];
    for i in (1024..flat.len()).step_by(4096) {
        flat[i] = 0.5;
    }
    let tone: Vec<_> = (0..64000)
        .map(|i| 0.5 * (std::f64::consts::TAU * 18000. * i as f64 / 64000.).sin())
        .collect();
    for mono in [
        vec![0.; 64000],
        shaped(true).iter().map(|x| x * 1e-8).collect(),
        flat,
        tone,
    ] {
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, 0.]).collect();
        let r = run(&stereo, 2, 64000, None);
        for s in &r.spectral_lags {
            assert_eq!(s.status, DetectorStatus::Inconclusive);
            assert!(s.lags.iter().all(|p| p.target.coefficient.is_none()));
        }
    }
}

#[test]
fn strict_prefix_minimum_frames_and_later_content_are_explicit() {
    let mono = shaped(true);
    for (length, count) in [(4096, 0), (8193, 3), (10241, 4)] {
        let r = run(&mono, 1, 64000, Some(length as f64 / 64000.));
        let s = &r.spectral_lags[0];
        assert_eq!(s.stft_frames, count);
        assert_eq!(s.active_frames, count);
        assert_eq!(s.lags[0].target.coefficient.is_some(), count >= 4);
        assert_eq!(
            s.interval.as_ref().map(|i| i.end_frame),
            (count > 0).then(|| (count - 1) * 2048 + 4096)
        );
    }
    let mut changed = mono.clone();
    changed[16384..].fill(0.);
    let a = run(&mono, 1, 64000, Some(0.256));
    let b = run(&changed, 1, 64000, Some(0.256));
    assert_eq!(
        a.coverage.as_ref().unwrap().decoded_pcm_sha256,
        b.coverage.as_ref().unwrap().decoded_pcm_sha256
    );
    assert_eq!(
        serde_json::to_value(&a.spectral_lags).unwrap(),
        serde_json::to_value(&b.spectral_lags).unwrap()
    );
}

#[test]
fn inactive_gaps_keep_original_window_coverage_and_channel_counts() {
    let mut mono = shaped(true);
    mono[20000..44000].fill(0.);
    let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, 0.]).collect();
    let r = run(&stereo, 2, 64000, None);
    let a = &r.spectral_lags[0];
    let b = &r.spectral_lags[1];
    assert_eq!(a.status, DetectorStatus::Measured);
    assert!(a.active_frames < a.stft_frames);
    assert_eq!(b.active_frames, 0);
    assert_eq!(b.status, DetectorStatus::Inconclusive);
    assert_eq!(a.stft_frames, b.stft_frames);
    assert_eq!(
        a.interval.as_ref().unwrap().end_frame,
        b.interval.as_ref().unwrap().end_frame
    );
}
