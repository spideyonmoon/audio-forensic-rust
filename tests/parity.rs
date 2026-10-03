use audio_forensic::{AnalysisOptions, CancellationToken, FileStatus, analyze_path};
use serde_json::Value;
use std::path::Path;

#[test]
fn streaming_features_match_pinned_python_oracles() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_dir.join("reference.json")).unwrap())
            .unwrap();
    for case in expected["cases"].as_array().unwrap() {
        let name = case["file"].as_str().unwrap();
        let report = analyze_path(
            fixture_dir.join(name),
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(
            report.status,
            FileStatus::Analyzed,
            "{name}: {:?}",
            report.diagnostics
        );
        assert_eq!(
            report.coverage.as_ref().unwrap().decoded_pcm_sha256,
            case["pcm_sha256"].as_str().unwrap(),
            "{name}: lossless PCM must be bit-exact"
        );
        let actual = serde_json::to_value(&report.channels).unwrap();
        for (ch, oracle) in case["channels"].as_array().unwrap().iter().enumerate() {
            for key in ["samples", "effective_bits"] {
                assert_eq!(actual[ch][key], oracle[key], "{name} channel {ch} {key}");
            }
            for key in ["peak", "rms", "dc_offset"] {
                let error =
                    (actual[ch][key].as_f64().unwrap() - oracle[key].as_f64().unwrap()).abs();
                assert!(error < 1e-10, "{name} channel {ch} {key}: {error}");
            }
            for key in ["frames", "active_frames"] {
                assert_eq!(
                    actual[ch]["spectral"][key], oracle["spectral"][key],
                    "{name} {key}"
                );
            }
            for (key, tolerance) in [
                ("cutoff_p95_hz", 0.01),
                ("cutoff_variance_hz2", 0.01),
                ("cliff_depth_db", 0.02),
                ("sharpness_db_per_bin", 0.02),
                ("hf_magnitude_ratio", 1e-6),
                ("entropy_bits", 1e-5),
                ("noise_above_cutoff_db", 0.02),
            ] {
                let a = actual[ch]["spectral"][key].as_f64().unwrap();
                let b = oracle["spectral"][key].as_f64().unwrap();
                assert!(
                    (a - b).abs() <= tolerance,
                    "{name} channel {ch} {key}: Rust {a} Python {b}, tolerance {tolerance}"
                );
            }
        }
    }
}

#[test]
fn flac_truncation_and_wrong_embedded_checksum_fail() {
    let original =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/noise16.flac"))
            .unwrap();
    let mut truncated = original.clone();
    truncated.truncate(truncated.len() / 2);
    let mut bad_checksum = original;
    bad_checksum[26] ^= 1; // STREAMINFO MD5 begins at file byte 26.
    for data in [truncated, bad_checksum] {
        let report = audio_forensic::analyze_source(
            Box::new(std::io::Cursor::new(data)),
            "broken.flac",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(report.status, FileStatus::Failed, "{:?}", report);
    }
}
