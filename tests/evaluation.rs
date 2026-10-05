use audio_forensic::{
    AnalysisReport, FileStatus,
    evaluation::{
        Codec, EvaluationCase, EvaluationManifest, EvaluationSession, FrozenEvaluation,
        PatternOutcome as O, Provenance as P, Split, StageLabel as L,
    },
    model::{AnalysisInterval, Coverage, DetectorResult, DetectorStatus as S, StreamInfo},
};

fn case(id: &str, group: &str, label: L) -> EvaluationCase {
    EvaluationCase {
        id: id.into(),
        source_group: group.into(),
        split: Split::Development,
        codec: Codec::Aac,
        label,
        provenance: P::GeneratedControl,
        processing: "native".into(),
        evidence_ref: "test recipe".into(),
        evidence_sha256: "e".repeat(64),
        expected_pcm_sha256: Some(if group == "g1" { "a" } else { "b" }.repeat(64)),
    }
}
fn plan(cases: Vec<EvaluationCase>) -> FrozenEvaluation {
    FrozenEvaluation::freeze(EvaluationManifest {
        manifest_version: 1,
        report_engine_version: env!("CARGO_PKG_VERSION").into(),
        cases,
    })
    .unwrap()
}
fn report(hash: &str, statuses: [S; 2]) -> AnalysisReport {
    let mut r = AnalysisReport::input_failure("generated", "test");
    r.status = FileStatus::Analyzed;
    r.diagnostics.clear();
    r.stream = Some(StreamInfo {
        track_id: 0,
        codec: "pcm".into(),
        sample_rate: 48000,
        channels: 2,
        bits_per_sample: Some(16),
        declared_frames: Some(48000),
        integer_pcm: true,
    });
    r.coverage = Some(Coverage {
        start_seconds: 0.0,
        end_seconds: 1.0,
        analyzed_frames: 48000,
        reached_end: true,
        requested_max_seconds: None,
        length_matches_header: Some(true),
        decoder_verification: None,
        analysis_passes: 2,
        decoded_pcm_sha256: hash.repeat(64),
        hash_sample_encoding: "s32le_msb_aligned".into(),
    });
    r.detectors = [
        "aac_quantization_lattice_mid",
        "aac_quantization_lattice_side",
    ]
    .into_iter()
    .zip(statuses)
    .map(|(id, status)| DetectorResult {
        id: id.into(),
        version: 1,
        family: "transform_grid".into(),
        status,
        channel_index: None,
        intervals: vec![AnalysisInterval {
            start_frame: 100,
            end_frame: 200,
        }],
        measurements: Default::default(),
        thresholds: Default::default(),
        caveats: vec![],
    })
    .collect();
    r
}
fn outcome(r: &AnalysisReport) -> O {
    let p = plan(vec![case("a", "g1", L::Present)]);
    let mut s = EvaluationSession::new(&p, Split::Development).unwrap();
    s.submit("a", r).unwrap();
    s.finish().unwrap().cases[0].outcome
}

#[test]
fn frozen_plan_rejects_leakage_unreviewed_negatives_and_mutation() {
    let a = case("a", "g1", L::Absent);
    let mut b = case("b", "g1", L::Present);
    b.split = Split::LockedTest;
    let m = |cases| EvaluationManifest {
        manifest_version: 1,
        report_engine_version: "0.22.0".into(),
        cases,
    };
    assert!(FrozenEvaluation::freeze(m(vec![a.clone(), b])).is_err());
    let mut b = case("b", "g2", L::Present);
    b.expected_pcm_sha256 = a.expected_pcm_sha256.clone();
    assert!(FrozenEvaluation::freeze(m(vec![a.clone(), b])).is_err());
    for provenance in [P::UnknownHistory, P::RecordedAddedStage] {
        let mut c = a.clone();
        c.provenance = provenance;
        assert!(FrozenEvaluation::freeze(m(vec![c])).is_err());
    }
    let mut c = a.clone();
    c.label = L::Unknown;
    assert!(FrozenEvaluation::freeze(m(vec![c.clone()])).is_err());
    c.split = Split::Challenge;
    c.provenance = P::UnknownHistory;
    c.expected_pcm_sha256 = None;
    assert!(FrozenEvaluation::freeze(m(vec![c])).is_ok());
    let mut p = plan(vec![a]);
    p.manifest.cases[0].label = L::Present;
    assert!(p.validate().is_err());
    let mut p = plan(vec![case("a", "g1", L::Present)]);
    p.evaluation_policy = "other".into();
    assert!(p.validate().is_err());
}

#[test]
fn paired_bases_deduplicate_hits_and_preserve_partial_abstention() {
    for (statuses, expected) in [
        ([S::Hit, S::Hit], O::Hit),
        ([S::Hit, S::Inconclusive], O::Hit),
        ([S::NotDetected, S::NotDetected], O::NoHit),
        ([S::NotDetected, S::Inconclusive], O::Abstain),
        ([S::Unsupported, S::NotDetected], O::Abstain),
        ([S::Measured, S::Measured], O::Abstain),
    ] {
        let r = report("a", statuses);
        let before = serde_json::to_vec(&r).unwrap();
        assert_eq!(outcome(&r), expected);
        assert_eq!(serde_json::to_vec(&r).unwrap(), before);
    }
    let r = report("a", [S::Hit, S::Hit]);
    let mut duplicate = r.clone();
    duplicate.detectors.push(duplicate.detectors[0].clone());
    assert_eq!(outcome(&duplicate), O::Abstain);
    let mut missing = r.clone();
    missing.detectors.pop();
    assert_eq!(outcome(&missing), O::Abstain);
    let mut future = r.clone();
    future.detectors[0].version += 1;
    assert_eq!(outcome(&future), O::Abstain);
    let mut invalid = r.clone();
    invalid.detectors[0].intervals[0].end_frame = 48001;
    assert_eq!(outcome(&invalid), O::Abstain);
    let mut empty = r;
    empty.detectors[0].intervals.clear();
    assert_eq!(outcome(&empty), O::Abstain);
}

#[test]
fn failures_prefixes_and_bad_contracts_do_not_become_negatives() {
    for status in [
        FileStatus::Failed,
        FileStatus::Unsupported,
        FileStatus::Cancelled,
        FileStatus::TimedOut,
    ] {
        let mut r = report("a", [S::Hit, S::Hit]);
        r.status = status;
        assert_eq!(outcome(&r), O::Abstain);
    }
    let mut prefix = report("a", [S::Hit, S::Hit]);
    prefix.coverage.as_mut().unwrap().reached_end = false;
    assert_eq!(outcome(&prefix), O::Abstain);
    let p = plan(vec![case("a", "g1", L::Present)]);
    let mut s = EvaluationSession::new(&p, Split::Development).unwrap();
    let mismatch = report("b", [S::Hit, S::Hit]);
    assert!(s.submit("a", &mismatch).is_err());
    let mut r = report("a", [S::Hit, S::Hit]);
    r.engine_version = "other".into();
    assert!(s.submit("a", &r).is_err());
    r.engine_version = env!("CARGO_PKG_VERSION").into();
    r.evidence_index = Some(1.0);
    assert!(s.submit("a", &r).is_err());
    assert!(s.finish().is_err());
}

#[test]
fn group_weight_is_equal_despite_unequal_variant_counts() {
    let p = plan(vec![
        case("a", "g1", L::Present),
        case("b", "g1", L::Present),
        case("c", "g1", L::Present),
        case("d", "g2", L::Present),
    ]);
    let mut s = EvaluationSession::new(&p, Split::Development).unwrap();
    for id in ["a", "b", "c"] {
        s.submit(id, &report("a", [S::Hit, S::Hit])).unwrap();
    }
    s.submit("d", &report("b", [S::NotDetected, S::NotDetected]))
        .unwrap();
    let result = s.finish().unwrap();
    let rates = &result.labeled_strata[0];
    assert_eq!(rates.file_counts.hit, 3);
    assert_eq!(rates.file_counts.no_hit, 1);
    assert_eq!(rates.groups.len(), 2);
    assert_eq!(rates.group_mean_hit_fraction, 0.5);
    assert_eq!(rates.group_mean_no_hit_fraction, 0.5);
}

#[test]
fn unknowns_are_excluded_and_split_missing_and_duplicate_submissions_fail() {
    let mut c = case("a", "g1", L::Unknown);
    c.split = Split::Challenge;
    c.provenance = P::UnknownHistory;
    let p = plan(vec![c, case("b", "g2", L::Absent)]);
    let mut s = EvaluationSession::new(&p, Split::Challenge).unwrap();
    let r = report("a", [S::Hit, S::Hit]);
    assert!(s.submit("b", &r).is_err());
    s.submit("a", &r).unwrap();
    assert!(s.submit("a", &r).is_err());
    let result = s.finish().unwrap();
    assert_eq!(result.unknown_cases_excluded, 1);
    assert!(result.labeled_strata.is_empty());
    assert!(EvaluationSession::new(&p, Split::LockedTest).is_err());
    assert!(
        EvaluationSession::new(&p, Split::Development)
            .unwrap()
            .finish()
            .is_err()
    );
}

#[test]
fn mono_and_vorbis_use_native_geometry_without_codec_cross_votes() {
    let mut r = report("a", [S::NotDetected, S::NotDetected]);
    r.stream.as_mut().unwrap().channels = 1;
    r.detectors.truncate(1);
    r.detectors[0].id = "aac_quantization_lattice_mono".into();
    r.detectors[0].channel_index = Some(0);
    assert_eq!(outcome(&r), O::NoHit);
    let mut v = r.detectors[0].clone();
    v.id = "vorbis_transform_grid".into();
    v.status = S::Hit;
    r.detectors.push(v);
    assert_eq!(outcome(&r), O::NoHit);
    let mut c = case("a", "g1", L::Present);
    c.codec = Codec::Vorbis;
    let p = plan(vec![c]);
    let mut s = EvaluationSession::new(&p, Split::Development).unwrap();
    s.submit("a", &r).unwrap();
    let result = s.finish().unwrap();
    assert_eq!(result.cases[0].outcome, O::Hit);
    assert_eq!(result.cases[0].detector_indices, [1]);
}

#[test]
fn abstentions_stay_in_denominators_and_provenance_stays_separate() {
    let mut added = case("added", "g1", L::Present);
    added.provenance = P::RecordedAddedStage;
    let p = plan(vec![
        case("hit", "g1", L::Present),
        case("skip", "g1", L::Present),
        added,
    ]);
    let mut s = EvaluationSession::new(&p, Split::Development).unwrap();
    s.submit("hit", &report("a", [S::Hit, S::Hit])).unwrap();
    s.submit("skip", &report("a", [S::Inconclusive, S::Unsupported]))
        .unwrap();
    s.submit("added", &report("a", [S::NotDetected, S::NotDetected]))
        .unwrap();
    let summary = s.finish().unwrap();
    assert_eq!(summary.labeled_strata.len(), 2);
    let generated = &summary.labeled_strata[0];
    assert_eq!(generated.provenance, P::GeneratedControl);
    assert_eq!(generated.group_mean_hit_fraction, 0.5);
    assert_eq!(generated.group_mean_abstain_fraction, 0.5);
    assert_eq!(summary.labeled_strata[1].group_mean_no_hit_fraction, 1.0);
}

#[test]
fn challenge_pcm_leakage_is_rejected_even_without_expected_hashes() {
    let mut a = case("a", "g1", L::Unknown);
    a.split = Split::Challenge;
    a.provenance = P::UnknownHistory;
    a.expected_pcm_sha256 = None;
    let mut b = a.clone();
    b.id = "b".into();
    b.source_group = "g2".into();
    let p = plan(vec![a, b]);
    let mut s = EvaluationSession::new(&p, Split::Challenge).unwrap();
    let r = report("a", [S::Hit, S::Hit]);
    s.submit("a", &r).unwrap();
    assert!(s.submit("b", &r).is_err());
    assert!(s.finish().is_err());
}

#[test]
fn manifest_paths_bounds_hashes_and_required_fields_are_checked() {
    let original = plan(vec![case("a", "g1", L::Present)]).manifest;
    for id in ["../outside", "a/b", "", "a.b"] {
        let mut m = original.clone();
        m.cases[0].id = id.into();
        assert!(FrozenEvaluation::freeze(m).is_err());
    }
    let mut m = original.clone();
    m.cases[0].expected_pcm_sha256 = None;
    assert!(FrozenEvaluation::freeze(m).is_err());
    let mut m = original.clone();
    m.cases[0].evidence_sha256 = "F".repeat(64);
    assert!(FrozenEvaluation::freeze(m).is_err());
    let mut m = original.clone();
    m.cases.push(m.cases[0].clone());
    assert!(FrozenEvaluation::freeze(m).is_err());
    let mut json = serde_json::to_value(original).unwrap();
    json["cases"][0]["typo"] = true.into();
    assert!(serde_json::from_value::<EvaluationManifest>(json).is_err());
}
