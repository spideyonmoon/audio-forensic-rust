use audio_forensic::{
    AnalysisOptions, CancellationToken, FileStatus, analyze_source, analyze_source_product,
    analyze_source_with_reference_inputs, analyze_source_with_spectrogram,
    analyze_source_with_tool_statistics,
    metadata::{MetadataStatus, read_metadata_source},
    product::{ProductReport, compare_products, comparison_key, read_product_json, render_product},
    reference_assessment::assess_reference,
    reference_inputs::ReferenceAnalysis,
    spectrogram_png::{CanvasOptions, CanvasSize},
};
use std::{io::Cursor, sync::OnceLock};

fn noise(seconds: usize) -> Vec<u8> {
    let source = include_bytes!("fixtures/clip_noise.wav");
    assert_eq!(&source[36..40], b"data");
    let mut bytes = source[..44].to_vec();
    for _ in 0..seconds / 2 {
        bytes.extend_from_slice(&source[44..]);
    }
    let n = (bytes.len() - 44) as u32;
    bytes[4..8].copy_from_slice(&(36 + n).to_le_bytes());
    bytes[40..44].copy_from_slice(&n.to_le_bytes());
    bytes
}
fn product() -> ProductReport {
    static P: OnceLock<ProductReport> = OnceLock::new();
    P.get_or_init(|| {
        analyze_source_product(
            Box::new(Cursor::new(noise(18))),
            "generated.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            true,
            |_| {},
        )
    })
    .clone()
}
fn reassess(p: &mut ProductReport) {
    p.reference_assessment = assess_reference(&ReferenceAnalysis {
        measurement: p.measurement_report.clone(),
        reference_inputs: p.reference_inputs.clone(),
    });
}

#[test]
fn combined_collectors_preserve_native_and_independent_products() {
    let p = product();
    p.validate().unwrap();
    assert_eq!(p.reference_assessment.status, "available");
    let bytes = noise(18);
    let options = AnalysisOptions::default();
    let cancel = CancellationToken::default();
    let native = analyze_source(
        Box::new(Cursor::new(bytes.clone())),
        "generated.wav",
        &options,
        &cancel,
    );
    let reference = analyze_source_with_reference_inputs(
        Box::new(Cursor::new(bytes.clone())),
        "generated.wav",
        &options,
        &cancel,
        |_| {},
    );
    let tools = analyze_source_with_tool_statistics(
        Box::new(Cursor::new(bytes.clone())),
        "generated.wav",
        &options,
        &cancel,
        |_| {},
    );
    let spectral = analyze_source_with_spectrogram(
        Box::new(Cursor::new(bytes)),
        "generated.wav",
        &options,
        &cancel,
        |_| {},
    );
    assert_eq!(
        serde_json::to_value(&p.measurement_report).unwrap(),
        serde_json::to_value(native).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&p.reference_inputs).unwrap(),
        serde_json::to_value(reference.reference_inputs).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&p.byproducts).unwrap(),
        serde_json::to_value(tools.byproducts).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&p.tool_statistics).unwrap(),
        serde_json::to_value(tools.tool_statistics).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&p.artifacts.spectrogram).unwrap(),
        serde_json::to_value(spectral.spectrogram).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&p.artifacts.presentation).unwrap(),
        serde_json::to_value(spectral.presentation).unwrap()
    );
    if let Ok(path) = std::env::var("P07_PRODUCT_OUTPUT") {
        let json = serde_json::to_vec(&p).unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(&json)
            .unwrap();
    }
}

#[test]
fn saved_dispatch_exact_integers_missing_fields_and_stale_bindings() {
    let p = product();
    assert_eq!(p.field_aliases.len(), 155);
    let json = serde_json::to_string(&p).unwrap();
    let read = read_product_json(&json).unwrap();
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(&read[0]).unwrap()
    );
    let mut value = serde_json::to_value(&p).unwrap();
    value["metadata"]["technical"]["file_size_bytes"] = u64::MAX.into();
    value["artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("presentation");
    value["artifacts"].as_object_mut().unwrap().remove("title");
    value["byproducts"] = serde_json::Value::Null;
    value["tool_statistics"] = serde_json::Value::Null;
    let read = read_product_json(&value.to_string()).unwrap();
    assert_eq!(
        read[0].metadata.technical.as_ref().unwrap().file_size_bytes,
        Some(u64::MAX)
    );
    assert!(read[0].artifacts.presentation.is_none());
    assert!(
        render_product(&read[0])
            .unwrap()
            .contains("18446744073709551615")
    );
    for (path, bad) in [
        (
            vec!["product_schema_version"],
            serde_json::json!("alfred-product-v1"),
        ),
        (
            vec!["reference_assessment", "assessment_version"],
            serde_json::json!(2),
        ),
        (
            vec!["reference_inputs", "decoded_pcm_sha256"],
            serde_json::json!("wrong"),
        ),
        (
            vec!["reference_assessment", "scores", "main"],
            serde_json::json!(55),
        ),
        (
            vec!["metadata", "technical", "selected_track_id"],
            serde_json::json!(99),
        ),
        (
            vec!["tool_statistics", "method_id"],
            serde_json::json!("wrong"),
        ),
        (
            vec!["byproducts", "coverage", "analyzed_frames"],
            serde_json::json!(99),
        ),
        (
            vec!["artifacts", "presentation", "presentation_version"],
            serde_json::json!(99),
        ),
    ] {
        let mut v = serde_json::to_value(&p).unwrap();
        let mut target = &mut v;
        for key in path {
            target = &mut target[key];
        }
        *target = bad;
        assert!(read_product_json(&v.to_string()).is_err());
    }
    assert!(read_product_json("[]").is_err());
    let mut v = serde_json::to_value(&p).unwrap();
    v.as_object_mut().unwrap().remove("metadata");
    assert!(read_product_json(&v.to_string()).is_err());
    let text = render_product(&p).unwrap();
    for s in [
        "uncalibrated",
        "Native ancestry: INCONCLUSIVE",
        "native_integer_pcm",
        "interval",
        "channel_index",
        "legacy_loudness_profile",
        "reference_mid",
    ] {
        assert!(text.contains(s), "{s}");
    }
}

#[test]
fn partial_failure_comparison_ties_incompatibility_and_missing_tuple() {
    let p = product();
    let fail = ProductReport::input_failure(audio_forensic::AnalysisReport::input_failure(
        "missing.wav",
        "generated missing",
    ));
    let result = compare_products(&[fail.clone(), p.clone(), p.clone()]).unwrap();
    assert_eq!(result.winner_input_index, Some(1));
    assert_eq!(
        result
            .ranking
            .iter()
            .map(|r| r.input_index)
            .collect::<Vec<_>>(),
        vec![1, 2, 0]
    );
    assert_eq!(
        compare_products(&[fail.clone(), fail])
            .unwrap()
            .winner_input_index,
        None
    );
    let mut partial = p.clone();
    partial
        .reference_inputs
        .as_mut()
        .unwrap()
        .base
        .cutoff_p95_hz
        .value = None;
    reassess(&mut partial);
    assert_eq!(partial.reference_assessment.status, "partial");
    partial.validate().unwrap();
    assert_eq!(
        compare_products(&[partial.clone(), p.clone()])
            .unwrap()
            .winner_input_index,
        Some(1)
    );
    assert_eq!(
        compare_products(&[partial.clone(), partial])
            .unwrap()
            .status,
        "unavailable"
    );
    let mut bad_domain = p.clone();
    bad_domain
        .tool_statistics
        .as_mut()
        .unwrap()
        .measurements
        .as_mut()
        .unwrap()
        .dr
        .input_domain = "foreign domain".into();
    assert!(compare_products(&[p.clone(), bad_domain]).is_err());
    let mut missing = p.clone();
    missing.tool_statistics = None;
    assert!(comparison_key(&missing).legacy_dr_integer.is_none());
    assert_eq!(
        compare_products(&[p.clone(), missing]).unwrap().status,
        "incompatible"
    );
    let mut prefix = p.clone();
    prefix
        .measurement_report
        .coverage
        .as_mut()
        .unwrap()
        .reached_end = false;
    prefix.reference_inputs.as_mut().unwrap().reached_end = false;
    prefix.byproducts.as_mut().unwrap().coverage = prefix.measurement_report.coverage.clone();
    prefix.tool_statistics.as_mut().unwrap().coverage = prefix.measurement_report.coverage.clone();
    prefix.artifacts.spectrogram.as_mut().unwrap().coverage =
        prefix.measurement_report.coverage.clone();
    reassess(&mut prefix);
    prefix.validate().unwrap();
    let result = compare_products(&[p, prefix]).unwrap();
    assert_eq!(result.status, "incompatible");
    assert!(result.ranking.iter().all(|r| r.rank.is_none() && !r.winner));
}

#[test]
fn png_saved_title_preset_collision_and_separate_export_failure() {
    let mut p = product();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.png");
    p.artifacts.title = Some("Saved title".into());
    let options = CanvasOptions {
        size: CanvasSize::Standard,
        title: None,
    };
    let export = p.export_png(
        &path,
        &options,
        &CancellationToken::default(),
        std::time::Duration::from_secs(60),
    );
    assert_eq!(export.path.as_deref(), Some(path.as_path()));
    let before = std::fs::read(&path).unwrap();
    let failed = p.export_png(
        &path,
        &options,
        &CancellationToken::default(),
        std::time::Duration::from_secs(60),
    );
    assert!(failed.path.is_none());
    assert_eq!(before, std::fs::read(path).unwrap());
    assert_eq!(p.measurement_report.status, FileStatus::Analyzed);
    p.validate().unwrap();
    assert!(p.diagnostics.iter().any(|d| d.code == "artifact_export"));
}

#[test]
fn cancellation_third_pass_deadline_and_prefix_all_products() {
    let cancel = CancellationToken::default();
    let token = cancel.clone();
    let p = analyze_source_product(
        Box::new(Cursor::new(noise(2))),
        "cancel.wav",
        &AnalysisOptions::default(),
        &cancel,
        true,
        |event| {
            if matches!(
                event,
                audio_forensic::AnalysisProgress::Decoding { pass: 3, .. }
            ) {
                token.cancel();
            }
        },
    );
    assert_eq!(p.measurement_report.status, FileStatus::Cancelled);
    assert!(p.reference_inputs.is_none());
    assert!(p.byproducts.as_ref().unwrap().reference.is_none());
    assert!(p.tool_statistics.as_ref().unwrap().measurements.is_none());
    p.validate().unwrap();
    let options = AnalysisOptions {
        deadline: std::time::Duration::ZERO,
        ..Default::default()
    };
    let p = analyze_source_product(
        Box::new(Cursor::new(noise(2))),
        "timeout.wav",
        &options,
        &CancellationToken::default(),
        true,
        |_| {},
    );
    assert_eq!(p.measurement_report.status, FileStatus::TimedOut);
    p.validate().unwrap();
    let options = AnalysisOptions {
        max_seconds: Some(0.125),
        ..Default::default()
    };
    let p = analyze_source_product(
        Box::new(Cursor::new(noise(2))),
        "prefix.wav",
        &options,
        &CancellationToken::default(),
        true,
        |_| {},
    );
    p.validate().unwrap();
    let c = p.measurement_report.coverage.as_ref().unwrap();
    assert_eq!(
        c.analyzed_frames,
        (0.125 * f64::from(p.measurement_report.stream.as_ref().unwrap().sample_rate)).floor()
            as u64
    );
    assert!(c.end_seconds <= 0.125);
    assert!(!c.reached_end);
    assert_eq!(
        p.byproducts
            .as_ref()
            .unwrap()
            .coverage
            .as_ref()
            .unwrap()
            .analyzed_frames,
        c.analyzed_frames
    );
    assert_eq!(
        p.tool_statistics
            .as_ref()
            .unwrap()
            .coverage
            .as_ref()
            .unwrap()
            .analyzed_frames,
        c.analyzed_frames
    );
    assert_eq!(
        p.reference_inputs.as_ref().unwrap().analyzed_frames,
        c.analyzed_frames
    );
    assert_eq!(
        p.artifacts
            .spectrogram
            .as_ref()
            .unwrap()
            .coverage
            .as_ref()
            .unwrap()
            .analyzed_frames,
        c.analyzed_frames
    );
}

#[test]
fn info_does_not_read_pcm_payload() {
    use std::io::{Read, Seek, SeekFrom};
    struct HeadersOnly(Cursor<Vec<u8>>);
    impl Read for HeadersOnly {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            let pos = self.0.position();
            assert!(pos < 44 || b.is_empty(), "PCM read at {pos}");
            let n = b.len().min(44usize.saturating_sub(pos as usize));
            self.0.read(&mut b[..n])
        }
    }
    impl Seek for HeadersOnly {
        fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
            self.0.seek(p)
        }
    }
    impl audio_forensic::MediaSource for HeadersOnly {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.0.get_ref().len() as u64)
        }
    }
    let m = read_metadata_source(
        Box::new(HeadersOnly(Cursor::new(noise(2)))),
        "spy.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(m.status, MetadataStatus::Available);
    assert_eq!(m.technical.unwrap().declared_duration_seconds, Some(2.));
}
