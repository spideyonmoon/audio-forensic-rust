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
        "rolloff.wav",
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

fn shaped(slope: f64) -> Vec<f64> {
    let mut bins = vec![Complex::default(); 4096];
    for k in 1..2048 {
        let hz = k as f64 * 48000.0 / 4096.0;
        let db = slope * ((hz - 12000.0) / 1000.0).clamp(-2.0, 8.0);
        bins[k] = Complex::from_polar(20.0 * 10f64.powf(db / 20.0), (k * 7919) as f64);
        bins[4096 - k] = bins[k].conj();
    }
    FftPlanner::new().plan_fft_inverse(4096).process(&mut bins);
    let peak = bins.iter().map(|x| x.re.abs()).fold(0.0, f64::max);
    (0..48000).map(|i| 0.7 * bins[i % 4096].re / peak).collect()
}

#[test]
fn slopes_follow_generated_eq_and_preserve_antiphase_gain() {
    for expected in [-4., 0., 4.] {
        let mono = shaped(expected);
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, -x * 0.5]).collect();
        let r = run(&stereo, 2, 48000, None);
        let a = &r.rolloff[0];
        let b = &r.rolloff[1];
        assert_eq!(a.status, DetectorStatus::Measured);
        assert_eq!(b.status, DetectorStatus::Measured);
        assert!(
            (a.slope_db_per_khz.unwrap() - expected).abs() < 0.02,
            "{:?}",
            a
        );
        assert!((a.slope_db_per_khz.unwrap() - b.slope_db_per_khz.unwrap()).abs() < 1e-6);
        assert_eq!(a.lower_band.bin_count, a.lower_band.eligible_bins);
        assert_eq!(a.upper_band.bin_count, a.upper_band.eligible_bins);
        let roundtrip: AnalysisReport =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(roundtrip.rolloff[0].slope_db_per_khz, a.slope_db_per_khz);
    }
}

#[test]
fn rate_geometry_requires_complete_endpoint_bands_and_reports_actual_bins() {
    for rate in [
        8000u32, 24500, 32000, 36500, 36501, 44100, 48000, 96000, 384000,
    ] {
        let mut mono = vec![0.; 16385];
        for i in (1024..mono.len()).step_by(4096) {
            mono[i] = 0.5;
        }
        let r = run(&mono, 1, rate, None);
        let s = &r.rolloff[0];
        if rate <= 36500 {
            assert_eq!(s.status, DetectorStatus::Unsupported);
            assert_eq!(s.slope_db_per_khz, None);
        } else {
            assert_eq!(s.status, DetectorStatus::Measured);
            assert!(s.slope_db_per_khz.unwrap().abs() < 1e-6);
            for b in [&s.lower_band, &s.upper_band] {
                assert!(b.lower_bin_hz.unwrap() >= b.requested_lower_hz);
                assert!(b.upper_bin_hz.unwrap() <= b.requested_upper_hz);
                assert_eq!(
                    b.mean_bin_hz,
                    Some((b.lower_bin_hz.unwrap() + b.upper_bin_hz.unwrap()) / 2.)
                );
            }
        }
    }
}

#[test]
fn silent_quiet_sparse_and_missing_channel_energy_abstain() {
    let mut comb = vec![0.; 48000];
    for i in (1024..comb.len()).step_by(2048) {
        comb[i] = 0.5;
    }
    let sine: Vec<_> = (0..48000)
        .map(|i| 0.5 * (std::f64::consts::TAU * 12000. * i as f64 / 48000.).sin())
        .collect();
    for mono in [
        vec![0.; 48000],
        shaped(-4.).iter().map(|&x| x * 1e-7).collect(),
        sine,
        comb,
    ] {
        let stereo: Vec<_> = mono.iter().flat_map(|&x| [x, 0.]).collect();
        let r = run(&stereo, 2, 48000, None);
        for s in &r.rolloff {
            assert_eq!(s.status, DetectorStatus::Inconclusive);
            assert_eq!(s.slope_db_per_khz, None);
        }
    }
}

#[test]
fn prefixes_strict_frame_boundary_and_inactive_gaps_are_explicit() {
    let mut mono = vec![0.; 16385];
    for i in (1024..mono.len()).step_by(4096) {
        mono[i] = 0.5;
    }
    for (length, frames) in [(4096, 0), (8193, 3), (10241, 4)] {
        let r = run(&mono, 1, 48000, Some(length as f64 / 48000.));
        let s = &r.rolloff[0];
        assert_eq!(s.active_frames, frames);
        assert_eq!(s.stft_frames, frames);
        assert_eq!(s.slope_db_per_khz.is_some(), frames >= 4);
        assert_eq!(
            s.interval.as_ref().map(|i| i.end_frame),
            if frames == 0 {
                None
            } else {
                Some((frames - 1) * 2048 + 4096)
            }
        );
    }
    mono.resize(48000, 0.);
    let r = run(&mono, 1, 48000, None);
    assert!(r.rolloff[0].active_frames < r.rolloff[0].stft_frames);
    assert_eq!(r.rolloff[0].status, DetectorStatus::Measured);
    assert!(r.rolloff[0].slope_db_per_khz.unwrap().abs() < 1e-6);
}
