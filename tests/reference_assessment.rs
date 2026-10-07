use audio_forensic::reference_assessment::{assess_reference, read_assessment_json};
use audio_forensic::reference_inputs::ReferenceAnalysis;
use audio_forensic::{
    AnalysisOptions, CancellationToken, FileStatus, analyze_path_with_reference_inputs,
};
use std::path::Path;

fn generated() -> ReferenceAnalysis {
    // The pinned plan needs nine two-second probes: extend this generated-only
    // two-second PCM fixture to 18 seconds without changing its sample words.
    let source =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clip_noise.wav"))
            .unwrap();
    assert_eq!(&source[36..40], b"data");
    let mut bytes = source[..44].to_vec();
    for _ in 0..9 {
        bytes.extend_from_slice(&source[44..]);
    }
    let data_bytes = (bytes.len() - 44) as u32;
    bytes[4..8].copy_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes[40..44].copy_from_slice(&data_bytes.to_le_bytes());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("generated-18s.wav");
    std::fs::write(&path, bytes).unwrap();
    analyze_path_with_reference_inputs(
        path,
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    )
}

#[test]
fn bound_generated_audio_and_saved_result_contract() {
    let input = generated();
    assert_eq!(input.measurement.status, FileStatus::Analyzed);
    let before = serde_json::to_value(&input.measurement).unwrap();
    let a = assess_reference(&input);
    assert!(
        a.status == "available",
        "{} {}",
        a.status,
        a.display_summary
    );
    assert_eq!(a.rules.len(), 34);
    assert_eq!(a.features["segment_majority"].unit, "boolean");
    assert_eq!(a.features["segment_0_peak"].unit, "dBFS");
    assert_eq!(a.features["segment_0_cliff"].unit, "dB relative");
    assert_eq!(
        a.rules.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        (1..=34).map(|n| format!("R{n:02}")).collect::<Vec<_>>()
    );
    for id in ["R11", "R13", "R32"] {
        let rule = a.rules.iter().find(|r| r.id == id).unwrap();
        assert_eq!(rule.state, "excluded_by_contract");
        assert_eq!(rule.before, rule.after);
    }
    assert_eq!(serde_json::to_value(&input.measurement).unwrap(), before);
    assert_eq!(input.measurement.ancestry_verdict, "INCONCLUSIVE");
    assert!(input.measurement.evidence_index.is_none());
    let saved = serde_json::to_string(&input).unwrap();
    let decoded: ReferenceAnalysis = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        serde_json::to_value(assess_reference(&decoded)).unwrap(),
        serde_json::to_value(&a).unwrap()
    );
    let json = serde_json::to_string(&a).unwrap();
    assert!(read_assessment_json(&json).is_ok());
    let mut wrong = serde_json::to_value(&a).unwrap();
    wrong["assessment_version"] = 9.into();
    assert!(read_assessment_json(&wrong.to_string()).is_err());
    for kind in 0..8 {
        let mut bad = input.clone();
        let r = bad.reference_inputs.as_mut().unwrap();
        match kind {
            0 => r.version = 999,
            1 => r.method = "native-lookalike".into(),
            2 => r.decoded_pcm_sha256 = "wrong".into(),
            3 => r.analyzed_frames += 1,
            4 => r.base.domain = "native_integer_pcm".into(),
            5 => r.pass_pcm_sha256[1] = "changed".into(),
            6 => r.source.void_profile.interval.end_frame = r.analyzed_frames + 1,
            _ => r.source.method = "unknown".into(),
        }
        let a = assess_reference(&bad);
        assert_eq!(a.status, "failed", "{kind}");
        assert!(a.scores.is_none());
        assert!(a.reference_label.is_none());
    }
    for (status, expected) in [
        (FileStatus::Failed, "failed"),
        (FileStatus::Cancelled, "cancelled"),
        (FileStatus::TimedOut, "timed_out"),
        (FileStatus::Unsupported, "unsupported"),
    ] {
        let mut bad = input.clone();
        bad.measurement.status = status;
        let a = assess_reference(&bad);
        assert_eq!(a.status, expected);
        assert!(a.scores.is_none());
        assert!(a.rules.is_empty());
    }
    let mut dsd = input.clone();
    dsd.measurement.stream.as_mut().unwrap().codec = "DSD".into();
    dsd.reference_inputs = None;
    assert_eq!(assess_reference(&dsd).status, "unsupported");
    let mut missing = input.clone();
    missing
        .reference_inputs
        .as_mut()
        .unwrap()
        .base
        .cutoff_p95_hz
        .value = None;
    let a = assess_reference(&missing);
    assert_eq!(a.status, "partial");
    assert!(a.scores.is_none());
    assert!(a.reference_label.is_none());
    assert!(a.missing_inputs.iter().any(|s| s == "cutoff"));
}

#[test]
fn prefix_and_renamed_extension_preserve_scope_and_codec() {
    let input = analyze_path_with_reference_inputs(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clip_noise.wav"),
        &AnalysisOptions {
            max_seconds: Some(1.),
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    let mut renamed = input.clone();
    renamed.measurement.source = "generated.mp3".into();
    let a = assess_reference(&input);
    let b = assess_reference(&renamed);
    assert_eq!(a.scores, b.scores);
    assert_eq!(a.reference_label, b.reference_label);
    let frames = input.measurement.coverage.as_ref().unwrap().analyzed_frames;
    assert!(
        a.features
            .values()
            .flat_map(|f| &f.intervals)
            .all(|i| i.start_frame <= i.end_frame && i.end_frame <= frames)
    );
    assert_eq!(a.status, "partial");
    assert!(a.scores.is_none());
}
