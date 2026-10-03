use audio_forensic::model::{AacBasis, DetectorStatus};
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_path,
};
use serde_json::Value;
use std::path::Path;

fn run_pcm(samples: &[f64], channels: u16, rate: u32) -> AnalysisReport {
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
    for sample in samples {
        out.extend((*sample as f32).to_le_bytes());
    }
    let report = audio_forensic::analyze_source(
        Box::new(std::io::Cursor::new(out)),
        "aac_control.wav",
        &AnalysisOptions::default(),
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

fn run_fixture(file: &str, max_seconds: Option<f64>) -> AnalysisReport {
    let report = analyze_path(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(file),
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
fn aac_scores_anchors_and_pcm_match_the_pinned_python_reference() {
    let oracle: Value = serde_json::from_str(include_str!("fixtures/aac_reference.json")).unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let report = run_fixture(file, None);
        assert_eq!(report.schema_version, audio_forensic::model::SCHEMA_VERSION);
        assert_eq!(report.policy_version, audio_forensic::model::POLICY_VERSION);
        assert!(
            !report
                .unimplemented_detectors
                .iter()
                .any(|d| d == "aac_mdct")
        );
        assert_eq!(
            report.coverage.as_ref().unwrap().decoded_pcm_sha256,
            case["pcm_sha256"]
        );
        let aac = &report.aac[0];
        assert_eq!(aac.basis, AacBasis::Mono);
        assert_eq!(
            aac.status,
            if case["encoded"].as_bool().unwrap() {
                DetectorStatus::Hit
            } else {
                DetectorStatus::NotDetected
            }
        );
        let expected = case["score"].as_f64().unwrap();
        assert!(
            (aac.lattice_score.unwrap() - expected).abs() < 1e-12,
            "{file}: {:?} != {expected}",
            aac.lattice_score
        );
        assert_eq!(aac.probes.len(), case["anchors"].as_array().unwrap().len());
        for (probe, expected) in aac.probes.iter().zip(case["anchors"].as_array().unwrap()) {
            assert_eq!(
                probe.anchor_frame,
                expected["anchor_frame"].as_u64().unwrap()
            );
            assert!((probe.anchor_rms - expected["anchor_rms"].as_f64().unwrap()).abs() < 1e-10);
            assert_eq!(probe.interval.end_frame - probe.interval.start_frame, 3064);
            assert!(probe.interval.end_frame <= report.coverage.as_ref().unwrap().analyzed_frames);
        }
        let detector = report
            .detectors
            .iter()
            .find(|d| d.id == "aac_quantization_lattice_mono")
            .unwrap();
        assert_eq!(detector.status, aac.status);
        assert_eq!(detector.channel_index, Some(0));
        assert_eq!(detector.intervals.len(), aac.probes.len());
        assert_eq!(
            detector.measurements["lattice_score"],
            aac.lattice_score.unwrap()
        );
        let roundtrip: AnalysisReport =
            serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
        assert_eq!(roundtrip.aac[0].lattice_score, aac.lattice_score);
    }
}

#[test]
fn mid_side_analysis_keeps_native_channels_and_quiet_bases_separate() {
    // Deterministic broadband control, not a provenance label.
    let mut state = 913u32;
    let samples: Vec<_> = (0..44100)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as i32 as f64 / i32::MAX as f64 * 0.2
        })
        .collect();
    let mono = run_pcm(&samples, 1, 44100);
    let expected = mono.aac[0].lattice_score.unwrap();
    assert!(expected < 0.06);
    for (name, right_scale) in [
        ("dual_mono", 1.0),
        ("quiet_side", 1.0 - 1e-7),
        ("anti_phase", -1.0),
    ] {
        let stereo: Vec<_> = samples
            .iter()
            .flat_map(|x| [*x, *x * right_scale])
            .collect();
        let report = run_pcm(&stereo, 2, 44100);
        assert_eq!(report.channels.len(), 2);
        assert!(report.channels.iter().all(|c| c.rms > 0.1));
        let (active, quiet) = if name == "anti_phase" { (1, 0) } else { (0, 1) };
        assert_eq!(report.aac[0].basis, AacBasis::Mid);
        assert_eq!(report.aac[1].basis, AacBasis::Side);
        assert!(
            (report.aac[active].lattice_score.unwrap() - expected).abs() < 0.002,
            "{name}"
        );
        assert_eq!(report.aac[quiet].status, DetectorStatus::Inconclusive);
        assert_eq!(report.aac[quiet].lattice_score, None);
        assert!(report.aac[quiet].probes.is_empty());
        assert!(
            report
                .detectors
                .iter()
                .filter(|d| d.id.starts_with("aac_quantization"))
                .all(|d| d.channel_index.is_none())
        );
    }
}

#[test]
fn sparse_quiet_and_transient_controls_abstain() {
    for mode in ["silence", "quiet", "tone", "harmonic", "impulse"] {
        let samples: Vec<_> = (0..44100)
            .map(|i| {
                let t = std::f64::consts::TAU * 440.0 * i as f64 / 44100.0;
                match mode {
                    "tone" => 0.2 * t.sin(),
                    "harmonic" => (1..4).map(|k| 0.1 * (t * k as f64).sin()).sum(),
                    "quiet" => 1e-7 * t.sin(),
                    "impulse" if i == 22050 => 0.8,
                    _ => 0.0,
                }
            })
            .collect();
        let report = run_pcm(&samples, 1, 44100);
        assert_eq!(report.aac[0].status, DetectorStatus::Inconclusive, "{mode}");
        assert_eq!(report.aac[0].lattice_score, None, "{mode}");
    }
}

#[test]
fn aac_respects_prefixes_and_rate_applicability() {
    let short = run_fixture("aac_256_44100.flac", Some(0.1));
    assert_eq!(short.aac[0].status, DetectorStatus::Inconclusive);
    assert!(short.aac[0].probes.is_empty());
    assert_eq!(short.aac[0].search_limit_frames, 4410);
    let prefix = run_fixture("aac_256_44100.flac", Some(0.5));
    assert!(
        prefix.aac[0]
            .probes
            .iter()
            .all(|p| p.interval.end_frame <= 22050)
    );
    assert!(!prefix.coverage.as_ref().unwrap().reached_end);
    let high = run_fixture("rate_full96.flac", None);
    assert_eq!(high.aac[0].status, DetectorStatus::Unsupported);
    assert!(high.aac[0].probes.is_empty());
    assert_eq!(high.aac[0].lattice_score, None);
}
