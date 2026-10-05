use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
    assess_evidence,
    evidence::{AssessmentAvailability as A, EvidenceDomain as D},
    model::{AnalysisInterval, DetectorResult, DetectorStatus as S},
};
use std::io::Cursor;

fn report() -> AnalysisReport {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(436u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(32000u32.to_le_bytes());
    bytes.extend(4u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(400u32.to_le_bytes());
    for i in 0..200i16 {
        bytes.extend((i * 17 - 500).to_le_bytes());
    }
    let r = analyze_source(
        Box::new(Cursor::new(bytes)),
        "generated",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed);
    r
}

fn detector(id: &str, family: &str, status: S) -> DetectorResult {
    DetectorResult {
        id: id.into(),
        family: family.into(),
        version: 1,
        status,
        channel_index: Some(0),
        intervals: vec![AnalysisInterval {
            start_frame: 0,
            end_frame: 50,
        }],
        measurements: Default::default(),
        thresholds: Default::default(),
        caveats: vec![],
    }
}

#[test]
fn real_report_preserves_all_records_scopes_and_original_bytes() {
    let r = report();
    let before = serde_json::to_vec(&r).unwrap();
    let a = assess_evidence(&r);
    assert_eq!(a.availability, A::Available);
    assert_eq!(a.analyzed_frames, Some(100));
    assert_eq!(a.reached_end, Some(true));
    let mut indices: Vec<_> = a
        .groups
        .iter()
        .flat_map(|g| g.detector_indices.iter().copied())
        .chain(a.deferred_mqa_indices.iter().copied())
        .collect();
    indices.sort();
    assert_eq!(indices, (0..r.detectors.len()).collect::<Vec<_>>());
    for g in &a.groups {
        let c = &g.counts;
        assert_eq!(
            c.hit + c.measured + c.not_detected + c.inconclusive + c.unsupported,
            g.detector_indices.len()
        );
    }
    assert!(a.conclusions.iter().all(|c| c.verdict == "INCONCLUSIVE"));
    assert_eq!(before, serde_json::to_vec(&r).unwrap());
    let wire = serde_json::to_value(&a).unwrap();
    assert_eq!(wire["assessment_version"], 1);
    assert!(wire.get("evidence_index").is_none());
}

#[test]
fn related_walls_and_multiple_codec_bases_do_not_multiply_inference() {
    let mut r = report();
    r.detectors = vec![
        detector("segment_wall", "spectral_wall", S::Hit),
        detector("codec_wall_candidates", "spectral_wall", S::Measured),
        detector("spectral_features", "spectral_measurements", S::Measured),
        detector("aac_quantization_lattice_mid", "transform_grid", S::Hit),
        detector("aac_quantization_lattice_side", "transform_grid", S::Hit),
        detector("vorbis_transform_grid", "transform_grid", S::Hit),
        detector("resampling_candidates", "sample_rate_history", S::Hit),
        detector("integer_precision", "bit_depth", S::Hit),
    ];
    let a = assess_evidence(&r);
    let spectral = a
        .groups
        .iter()
        .find(|g| g.domain == D::SpectralPatterns)
        .unwrap();
    assert_eq!(spectral.detector_indices, [0, 1, 2]);
    assert_eq!(
        spectral.provisional_patterns,
        ["repeated or mixed spectral walls"]
    );
    let codec = a
        .groups
        .iter()
        .find(|g| g.domain == D::CodecTransforms)
        .unwrap();
    assert_eq!(codec.counts.hit, 3);
    assert_eq!(
        codec.provisional_patterns,
        ["AAC quantization lattice", "Vorbis transform grid"]
    );
    // Repeated channels/records must not alter the interpretation or conclusions.
    r.detectors.extend(r.detectors.clone());
    let doubled = assess_evidence(&r);
    for (left, right) in a.groups.iter().zip(&doubled.groups) {
        assert_eq!(left.provisional_patterns, right.provisional_patterns);
        assert_eq!(
            left.detector_indices.len() * 2,
            right.detector_indices.len()
        );
    }
    assert_eq!(
        serde_json::to_value(a.conclusions).unwrap(),
        serde_json::to_value(doubled.conclusions).unwrap()
    );
}

#[test]
fn absence_abstentions_and_measurements_never_become_authenticity() {
    let mut r = report();
    r.detectors = vec![
        detector(
            "aac_quantization_lattice_mid",
            "transform_grid",
            S::NotDetected,
        ),
        detector(
            "aac_quantization_lattice_side",
            "transform_grid",
            S::Inconclusive,
        ),
        detector("vorbis_transform_grid", "transform_grid", S::Unsupported),
        detector("quiet_block_profile", "noise_measurements", S::Measured),
        detector(
            "highpass_envelope_peaks",
            "transient_measurements",
            S::Measured,
        ),
    ];
    r.coverage.as_mut().unwrap().reached_end = false;
    let a = assess_evidence(&r);
    assert_eq!(a.reached_end, Some(false));
    let codec = a
        .groups
        .iter()
        .find(|g| g.domain == D::CodecTransforms)
        .unwrap();
    assert_eq!(
        (
            codec.counts.not_detected,
            codec.counts.inconclusive,
            codec.counts.unsupported
        ),
        (1, 1, 1)
    );
    assert!(codec.provisional_patterns.is_empty());
    let profile = a
        .groups
        .iter()
        .find(|g| g.domain == D::SourceProfileMeasurements)
        .unwrap();
    assert_eq!(profile.detector_indices, [3, 4]);
    assert_eq!(profile.counts.measured, 2);
    assert!(a.conclusions.iter().all(|c| c.verdict == "INCONCLUSIVE"));
    assert!(a.to_string().contains("reached stream end: false"));
}

#[test]
fn failures_and_unrecognized_contracts_do_not_interpret_stale_hits() {
    let mut r = report();
    r.detectors = vec![detector("integer_precision", "bit_depth", S::Hit)];
    for status in [
        FileStatus::Failed,
        FileStatus::Unsupported,
        FileStatus::Cancelled,
        FileStatus::TimedOut,
    ] {
        r.status = status;
        let a = assess_evidence(&r);
        assert_eq!(a.availability, A::InputUnavailable);
        assert!(a.groups.is_empty() && a.analyzed_frames.is_none());
    }
    r.status = FileStatus::Analyzed;
    for field in ["schema", "policy"] {
        let mut future = r.clone();
        if field == "schema" {
            future.schema_version = "999".into();
        } else {
            future.policy_version = "future-policy".into();
        }
        let a = assess_evidence(&future);
        assert_eq!(a.availability, A::UnsupportedReport);
        assert!(a.groups.is_empty());
    }
}

#[test]
fn mqa_is_deferred_and_unknown_versions_or_families_stay_uninterpreted() {
    let mut r = report();
    let mut future = detector("aac_quantization_lattice_mid", "transform_grid", S::Hit);
    future.version = 2;
    r.detectors = vec![
        future,
        detector("future_test", "unknown_family", S::Hit),
        detector("mqa_signalling_candidates", "mqa_signalling", S::Hit),
    ];
    let a = assess_evidence(&r);
    assert_eq!(a.deferred_mqa_indices, [2]);
    assert_eq!(a.groups.len(), 1);
    assert_eq!(a.groups[0].domain, D::Uninterpreted);
    assert_eq!(a.groups[0].detector_indices, [0, 1]);
    assert!(a.groups[0].provisional_patterns.is_empty());
}

#[test]
fn invalid_interval_channel_or_missing_coverage_abstains() {
    let r = report();
    for case in 0..5 {
        let mut invalid = r.clone();
        invalid.detectors = vec![detector("integer_precision", "bit_depth", S::Hit)];
        match case {
            0 => invalid.detectors[0].channel_index = Some(2),
            1 => invalid.detectors[0].intervals[0].end_frame = 101,
            2 => invalid.detectors[0].intervals[0].start_frame = 60,
            3 => invalid.coverage = None,
            _ => invalid.stream = None,
        }
        let a = assess_evidence(&invalid);
        assert_eq!(a.availability, A::InvalidCoverage);
        assert!(a.groups.is_empty());
    }
}
