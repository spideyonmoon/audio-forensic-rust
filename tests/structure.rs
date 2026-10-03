use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_path,
};
use serde_json::Value;
use std::path::Path;

fn run_pcm(samples: &[f32], channels: u16, rate: u32, max_seconds: Option<f64>) -> AnalysisReport {
    let bytes = samples.len() as u32 * 4;
    let mut out = vec![];
    out.extend(b"RIFF");
    out.extend((36 + bytes).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(3u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * channels as u32 * 4).to_le_bytes());
    out.extend((channels * 4).to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    for x in samples {
        out.extend(x.to_le_bytes());
    }
    let report = audio_forensic::analyze_source(
        Box::new(std::io::Cursor::new(out)),
        "structure.wav",
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
fn structure_matches_python_numpy_and_independent_pcm_oracles() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/structure_reference.json")).unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let name = case["file"].as_str().unwrap();
        let report = analyze_path(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(report.status, FileStatus::Analyzed);
        assert_eq!(report.schema_version, audio_forensic::model::SCHEMA_VERSION);
        assert_eq!(report.policy_version, audio_forensic::model::POLICY_VERSION);
        assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
        assert_eq!(report.evidence_index, None);
        assert_eq!(
            report.coverage.as_ref().unwrap().decoded_pcm_sha256,
            case["pcm_sha256"]
        );
        assert!(
            !report
                .unimplemented_detectors
                .iter()
                .any(|d| d == "aucdtect")
        );
        let bin_hz = case["sample_rate"].as_f64().unwrap() / 4096.0;
        for (ch, oracle) in case["channels"].as_array().unwrap().iter().enumerate() {
            let expected = &oracle["expected"];
            let actual = serde_json::to_value(&report.spectral_structure[ch]).unwrap();
            for key in [
                "stft_frames",
                "active_frames",
                "bound_frames",
                "flat_frames",
                "phase_frame_pairs",
                "phase_bin_pairs",
            ] {
                assert_eq!(actual[key], expected[key], "{name} ch{ch} {key}");
            }
            for (key, tolerance) in [
                ("average_scatter_bound_hz", bin_hz),
                ("modal_scatter_bound_hz", bin_hz),
                ("phase_band_start_hz", 1e-9),
                ("high_band_phase_entropy_bits", 0.001),
            ] {
                match (actual[key].as_f64(), expected[key].as_f64()) {
                    (Some(a), Some(b)) => assert!(
                        (a - b).abs() <= tolerance,
                        "{name} ch{ch} {key}: {a} != {b}"
                    ),
                    (None, None) => (),
                    _ => panic!("{name} ch{ch} {key}: applicability differs"),
                }
            }
            // Original bound arithmetic is meaningful on these non-flat, short controls.
            let bound = report.spectral_structure[ch]
                .average_scatter_bound_hz
                .unwrap();
            let pinned = oracle["reference"]["average_bound_hz"].as_f64().unwrap();
            assert!(
                (bound - pinned).abs() <= bin_hz,
                "{name}: Rust {bound}, pinned Python {pinned}"
            );
            assert_eq!(
                report.spectral_structure[ch].active_frames,
                report.channels[ch].spectral.active_frames
            );
            assert!(
                report.spectral_structure[ch]
                    .interval
                    .as_ref()
                    .unwrap()
                    .end_frame
                    < report.coverage.as_ref().unwrap().analyzed_frames
            );
        }
        let roundtrip: AnalysisReport =
            serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
        assert_eq!(roundtrip.spectral_structure.len(), report.channels.len());
        assert!(
            report
                .detectors
                .iter()
                .filter(|d| matches!(
                    d.id.as_str(),
                    "spectral_scatter_bound" | "high_band_phase_entropy"
                ))
                .all(|d| !matches!(d.status, DetectorStatus::Hit | DetectorStatus::NotDetected))
        );
    }
}

#[test]
fn native_stereo_phase_survives_antiphase_and_silent_channels_stay_empty() {
    let mut state = 213u32;
    let mono: Vec<_> = (0..16385)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as i32 as f32 / i32::MAX as f32 * 0.2
        })
        .collect();
    for silent in [false, true] {
        let stereo: Vec<_> = mono
            .iter()
            .flat_map(|x| [*x, if silent { 0.0 } else { -*x }])
            .collect();
        let report = run_pcm(&stereo, 2, 48000, None);
        let a = &report.spectral_structure[0];
        let b = &report.spectral_structure[1];
        assert_eq!(a.scatter_status, DetectorStatus::Measured);
        assert_eq!(a.phase_status, DetectorStatus::Measured);
        if silent {
            assert_eq!(b.active_frames, 0);
            assert_eq!(b.scatter_status, DetectorStatus::Inconclusive);
            assert_eq!(b.phase_status, DetectorStatus::Inconclusive);
            assert_eq!(b.average_scatter_bound_hz, None);
            assert_eq!(b.high_band_phase_entropy_bits, None);
        } else {
            assert_eq!(a.average_scatter_bound_hz, b.average_scatter_bound_hz);
            assert!(
                (a.high_band_phase_entropy_bits.unwrap() - b.high_band_phase_entropy_bits.unwrap())
                    .abs()
                    < 1e-12
            );
        }
    }
}

#[test]
fn phase_and_scatter_report_short_silent_and_unsupported_coverage() {
    for length in [4096, 4097, 10240, 10241] {
        let report = run_pcm(&vec![0.0; length], 1, 48000, None);
        let stats = &report.spectral_structure[0];
        assert_eq!(stats.scatter_status, DetectorStatus::Inconclusive);
        assert_eq!(stats.phase_status, DetectorStatus::Inconclusive);
        assert_eq!(stats.average_scatter_bound_hz, None);
        assert_eq!(stats.high_band_phase_entropy_bits, None);
        let frames = length.saturating_sub(4096).div_ceil(2048) as u64;
        assert_eq!(stats.stft_frames, frames);
        assert_eq!(
            stats.interval.as_ref().map(|i| i.end_frame),
            (frames > 0).then_some(frames.saturating_sub(1) * 2048 + 4096)
        );
    }
    let report = run_pcm(&vec![0.1; 20000], 1, 16000, Some(0.5));
    let stats = &report.spectral_structure[0];
    assert_eq!(report.coverage.as_ref().unwrap().analyzed_frames, 8000);
    assert_eq!(stats.phase_status, DetectorStatus::Unsupported);
    assert_eq!(stats.high_band_phase_entropy_bits, None);
    assert!(stats.interval.as_ref().unwrap().end_frame < 8000);
}
