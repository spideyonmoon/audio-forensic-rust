//! Frozen, grouped evaluation of provisional AAC/Vorbis observations.
//! Labels are externally reviewed declarations, never inferred from reports.
use crate::{
    AnalysisReport, FileStatus, assess_evidence,
    evidence::AssessmentAvailability,
    model::{DetectorStatus, POLICY_VERSION, SCHEMA_VERSION},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EVALUATION_POLICY: &str = "codec-pattern-presence-v1";
pub const MAX_CASES: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    Aac,
    Vorbis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Development,
    Validation,
    LockedTest,
    Challenge,
}

/// Absence requires a reviewed chain, not merely an unencoded immediate parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageLabel {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    GeneratedControl,
    ReviewedHistory,
    RecordedAddedStage,
    UnknownHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCase {
    pub id: String,
    pub source_group: String,
    pub split: Split,
    pub codec: Codec,
    pub label: StageLabel,
    pub provenance: Provenance,
    /// Declared codec/processing stratum, e.g. aac_256_trim137_s16.
    pub processing: String,
    /// Human review/recipe reference and SHA-256 of its retained UTF-8 receipt.
    pub evidence_ref: String,
    pub evidence_sha256: String,
    /// Independent decoder's hash in the core's canonical PCM representation.
    /// Required for labeled cases; optional only for unknown challenge inputs.
    pub expected_pcm_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationManifest {
    pub manifest_version: u32,
    /// Exact measurement engine version; every submitted report must match.
    pub report_engine_version: String,
    pub cases: Vec<EvaluationCase>,
}

/// Freeze before inspecting selected evaluation reports. The digest detects
/// changes, but is not a signature or proof that a human kept test data unseen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenEvaluation {
    pub evaluation_policy: String,
    pub evaluator_version: String,
    pub manifest_sha256: String,
    pub manifest: EvaluationManifest,
}

#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    #[error("invalid evaluation plan: {0}")]
    InvalidPlan(String),
    #[error("invalid evaluation report for {id}: {reason}")]
    InvalidReport { id: String, reason: String },
    #[error("evaluation incomplete; missing cases: {0:?}")]
    MissingCases(Vec<String>),
}

fn invalid(reason: &str) -> EvaluationError {
    EvaluationError::InvalidPlan(reason.into())
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest_manifest(manifest: &EvaluationManifest) -> String {
    sha256(&serde_json::to_vec(manifest).expect("manifest contains no fallible JSON values"))
}
fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl FrozenEvaluation {
    pub fn freeze(manifest: EvaluationManifest) -> Result<Self, EvaluationError> {
        validate_manifest(&manifest)?;
        Ok(Self {
            evaluation_policy: EVALUATION_POLICY.into(),
            evaluator_version: env!("CARGO_PKG_VERSION").into(),
            manifest_sha256: digest_manifest(&manifest),
            manifest,
        })
    }
    pub fn validate(&self) -> Result<(), EvaluationError> {
        if self.evaluation_policy != EVALUATION_POLICY
            || self.evaluator_version != env!("CARGO_PKG_VERSION")
        {
            return Err(invalid("unsupported frozen policy/evaluator version"));
        }
        validate_manifest(&self.manifest)?;
        if self.manifest_sha256 != digest_manifest(&self.manifest) {
            return Err(invalid("manifest fingerprint changed"));
        }
        Ok(())
    }
}

fn validate_manifest(m: &EvaluationManifest) -> Result<(), EvaluationError> {
    if m.manifest_version != 1
        || m.cases.is_empty()
        || m.cases.len() > MAX_CASES
        || m.report_engine_version.trim().is_empty()
        || m.report_engine_version.len() > 128
    {
        return Err(invalid(
            "require version 1, an engine version, and 1..=10000 cases",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::new();
    let mut hashes = BTreeMap::new();
    for c in &m.cases {
        if !identifier(&c.id)
            || !identifier(&c.source_group)
            || !identifier(&c.processing)
            || c.evidence_ref.trim().is_empty()
            || c.evidence_ref.len() > 4096
            || !is_hash(&c.evidence_sha256)
        {
            return Err(invalid(
                "invalid case/group/processing identifier or evidence reference/hash",
            ));
        }
        if !ids.insert(&c.id) {
            return Err(invalid("duplicate case ID"));
        }
        if groups
            .insert(&c.source_group, c.split)
            .is_some_and(|s| s != c.split)
        {
            return Err(invalid("source group occurs in multiple splits"));
        }
        match (c.label, c.provenance) {
            (StageLabel::Absent, Provenance::GeneratedControl | Provenance::ReviewedHistory)
            | (
                StageLabel::Present,
                Provenance::GeneratedControl
                | Provenance::ReviewedHistory
                | Provenance::RecordedAddedStage,
            ) => {}
            (StageLabel::Unknown, _) if c.split == Split::Challenge => {}
            _ => {
                return Err(invalid(
                    "label/provenance is incompatible; unknown labels require challenge split",
                ));
            }
        }
        if let Some(hash) = &c.expected_pcm_sha256 {
            if !is_hash(hash) {
                return Err(invalid("invalid independent PCM hash"));
            }
            if hashes
                .insert(hash, &c.source_group)
                .is_some_and(|g| g != &c.source_group)
            {
                return Err(invalid(
                    "identical expected PCM assigned to different source groups",
                ));
            }
        } else if c.label != StageLabel::Unknown {
            return Err(invalid("labeled cases require independent PCM hash"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternOutcome {
    Hit,
    NoHit,
    Abstain,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseOutcome {
    pub case_id: String,
    pub outcome: PatternOutcome,
    pub reason: &'static str,
    pub file_status: FileStatus,
    /// Digest of typed report JSON; not the original on-disk byte digest.
    pub normalized_report_sha256: String,
    pub detector_indices: Vec<usize>,
    pub analyzed_frames: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OutcomeCounts {
    pub hit: usize,
    pub no_hit: usize,
    pub abstain: usize,
}
impl OutcomeCounts {
    fn add(&mut self, outcome: PatternOutcome) {
        match outcome {
            PatternOutcome::Hit => self.hit += 1,
            PatternOutcome::NoHit => self.no_hit += 1,
            PatternOutcome::Abstain => self.abstain += 1,
        }
    }
    fn total(&self) -> usize {
        self.hit + self.no_hit + self.abstain
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupOutcomes {
    pub source_group: String,
    pub counts: OutcomeCounts,
}

/// Equal weight per declared group within a codec/processing/label/provenance
/// stratum. Rates include abstentions in the denominator. No pooled accuracy or
/// binomial confidence interval treats related derivatives as independent trials.
#[derive(Debug, Clone, Serialize)]
pub struct StratumOutcomes {
    pub codec: Codec,
    pub processing: String,
    pub label: StageLabel,
    pub provenance: Provenance,
    pub groups: Vec<GroupOutcomes>,
    pub file_counts: OutcomeCounts,
    pub group_mean_hit_fraction: f64,
    pub group_mean_no_hit_fraction: f64,
    pub group_mean_abstain_fraction: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationSummary {
    pub evaluation_policy: &'static str,
    pub evaluator_version: &'static str,
    pub manifest_sha256: String,
    pub split: Split,
    pub cases: Vec<CaseOutcome>,
    /// Unknown challenge labels remain in cases and never enter these strata.
    pub labeled_strata: Vec<StratumOutcomes>,
    pub unknown_cases_excluded: usize,
    pub limitations: Vec<&'static str>,
}

/// Feed one report at a time. Storage is O(manifest cases), never O(audio length)
/// or O(sum of report sizes). A summary is unavailable until every selected case
/// has exactly one report. Other splits cannot be submitted accidentally.
pub struct EvaluationSession<'a> {
    plan: &'a FrozenEvaluation,
    split: Split,
    outcomes: BTreeMap<String, CaseOutcome>,
    pcm_groups: BTreeMap<String, String>,
}

impl<'a> EvaluationSession<'a> {
    pub fn new(plan: &'a FrozenEvaluation, split: Split) -> Result<Self, EvaluationError> {
        plan.validate()?;
        if !plan.manifest.cases.iter().any(|c| c.split == split) {
            return Err(invalid("selected split is empty"));
        }
        Ok(Self {
            plan,
            split,
            outcomes: BTreeMap::new(),
            pcm_groups: plan
                .manifest
                .cases
                .iter()
                .filter_map(|c| {
                    c.expected_pcm_sha256
                        .as_ref()
                        .map(|h| (h.clone(), c.source_group.clone()))
                })
                .collect(),
        })
    }

    pub fn submit(&mut self, id: &str, report: &AnalysisReport) -> Result<(), EvaluationError> {
        let bad = |reason: &str| EvaluationError::InvalidReport {
            id: id.into(),
            reason: reason.into(),
        };
        let case = self
            .plan
            .manifest
            .cases
            .iter()
            .find(|c| c.id == id && c.split == self.split)
            .ok_or_else(|| bad("case absent from selected split"))?;
        if self.outcomes.contains_key(id) {
            return Err(bad("duplicate report"));
        }
        if report.engine_version != self.plan.manifest.report_engine_version
            || report.schema_version != SCHEMA_VERSION
            || report.policy_version != POLICY_VERSION
            || report.ancestry_verdict != "INCONCLUSIVE"
            || report.evidence_index.is_some()
        {
            return Err(bad(
                "report engine/schema/policy or inference contract mismatch",
            ));
        }
        if report.status == FileStatus::Analyzed {
            if let (Some(expected), Some(coverage)) = (&case.expected_pcm_sha256, &report.coverage)
            {
                if expected != &coverage.decoded_pcm_sha256 {
                    return Err(bad("independent PCM hash mismatch"));
                }
            }
        }
        let (outcome, reason, indices) = observe(report, case.codec);
        let normalized_report_sha256 =
            sha256(&serde_json::to_vec(report).map_err(|e| bad(&e.to_string()))?);
        if report.status == FileStatus::Analyzed {
            if let Some(coverage) = &report.coverage {
                if coverage.reached_end
                    && coverage.requested_max_seconds.is_none()
                    && is_hash(&coverage.decoded_pcm_sha256)
                {
                    if self
                        .pcm_groups
                        .get(&coverage.decoded_pcm_sha256)
                        .is_some_and(|g| g != &case.source_group)
                    {
                        return Err(bad("decoded PCM duplicates another declared source group"));
                    }
                    self.pcm_groups.insert(
                        coverage.decoded_pcm_sha256.clone(),
                        case.source_group.clone(),
                    );
                }
            }
        }
        self.outcomes.insert(
            id.into(),
            CaseOutcome {
                case_id: id.into(),
                outcome,
                reason,
                file_status: report.status.clone(),
                normalized_report_sha256,
                detector_indices: indices,
                analyzed_frames: report.coverage.as_ref().map(|c| c.analyzed_frames),
            },
        );
        Ok(())
    }

    pub fn finish(self) -> Result<EvaluationSummary, EvaluationError> {
        let selected: Vec<_> = self
            .plan
            .manifest
            .cases
            .iter()
            .filter(|c| c.split == self.split)
            .collect();
        let missing: Vec<_> = selected
            .iter()
            .filter(|c| !self.outcomes.contains_key(&c.id))
            .map(|c| c.id.clone())
            .collect();
        if !missing.is_empty() {
            return Err(EvaluationError::MissingCases(missing));
        }
        type Key = (Codec, String, StageLabel, Provenance);
        let mut strata = BTreeMap::<Key, BTreeMap<String, OutcomeCounts>>::new();
        let mut unknown = 0;
        for c in &selected {
            if c.label == StageLabel::Unknown {
                unknown += 1;
                continue;
            }
            strata
                .entry((c.codec, c.processing.clone(), c.label, c.provenance))
                .or_default()
                .entry(c.source_group.clone())
                .or_default()
                .add(self.outcomes[&c.id].outcome);
        }
        let labeled_strata = strata
            .into_iter()
            .map(|((codec, processing, label, provenance), groups)| {
                let mut s = StratumOutcomes {
                    codec,
                    processing,
                    label,
                    provenance,
                    groups: vec![],
                    file_counts: OutcomeCounts::default(),
                    group_mean_hit_fraction: 0.0,
                    group_mean_no_hit_fraction: 0.0,
                    group_mean_abstain_fraction: 0.0,
                };
                let n = groups.len() as f64;
                for (source_group, counts) in groups {
                    let denominator = counts.total() as f64 * n;
                    s.group_mean_hit_fraction += counts.hit as f64 / denominator;
                    s.group_mean_no_hit_fraction += counts.no_hit as f64 / denominator;
                    s.group_mean_abstain_fraction += counts.abstain as f64 / denominator;
                    s.file_counts.hit += counts.hit;
                    s.file_counts.no_hit += counts.no_hit;
                    s.file_counts.abstain += counts.abstain;
                    s.groups.push(GroupOutcomes {
                        source_group,
                        counts,
                    });
                }
                s
            })
            .collect();
        Ok(EvaluationSummary {
            evaluation_policy: EVALUATION_POLICY,
            evaluator_version: env!("CARGO_PKG_VERSION"),
            manifest_sha256: self.plan.manifest_sha256.clone(),
            split: self.split,
            cases: selected
                .iter()
                .map(|c| self.outcomes[&c.id].clone())
                .collect(),
            labeled_strata,
            unknown_cases_excluded: unknown,
            limitations: vec![
                "Evaluates provisional pattern presence, not ancestry, source medium or original bit depth.",
                "Labels, evidence receipts and group independence require external review; this evaluator does not authenticate their truth.",
                "Generated controls, reviewed histories and recorded added stages remain separate strata. Unknown cases are excluded from labeled rates.",
                "Rates weight declared source groups equally within each stratum and include abstentions. No calibrated probability, population uncertainty interval or overall accuracy is supplied.",
                "Full-file reports can contain narrower detector spans. Resolve detector indices in the exact source report.",
                "Freezing detects plan changes, not prior exposure, retuning, related unidentified sources or deliberate relabeling.",
            ],
        })
    }
}

fn observe(r: &AnalysisReport, codec: Codec) -> (PatternOutcome, &'static str, Vec<usize>) {
    use PatternOutcome::{Abstain, Hit, NoHit};
    if r.status != FileStatus::Analyzed {
        return (Abstain, "input_unavailable", vec![]);
    }
    if assess_evidence(r).availability != AssessmentAvailability::Available {
        return (Abstain, "invalid_report_scope", vec![]);
    }
    let coverage = r.coverage.as_ref().unwrap();
    let stream = r.stream.as_ref().unwrap();
    if !coverage.reached_end
        || coverage.requested_max_seconds.is_some()
        || coverage.start_seconds != 0.0
        || coverage.analysis_passes != 2
        || coverage.analyzed_frames == 0
        || coverage.length_matches_header == Some(false)
        || coverage.decoder_verification == Some(false)
        || !is_hash(&coverage.decoded_pcm_sha256)
        || !(8000..=384000).contains(&stream.sample_rate)
        || !coverage.end_seconds.is_finite()
        || (coverage.end_seconds - coverage.analyzed_frames as f64 / stream.sample_rate as f64)
            .abs()
            > 1e-9
        || stream
            .declared_frames
            .is_some_and(|n| n != coverage.analyzed_frames)
        || coverage.hash_sample_encoding
            != if stream.integer_pcm {
                "s32le_msb_aligned"
            } else {
                "f64le"
            }
    {
        return (Abstain, "incomplete_or_invalid_full_file_coverage", vec![]);
    }
    let slots: Vec<(&str, Option<usize>)> = match codec {
        Codec::Aac if stream.channels == 1 => vec![("aac_quantization_lattice_mono", Some(0))],
        Codec::Aac => vec![
            ("aac_quantization_lattice_mid", None),
            ("aac_quantization_lattice_side", None),
        ],
        Codec::Vorbis => (0..stream.channels)
            .map(|c| ("vorbis_transform_grid", Some(c)))
            .collect(),
    };
    let indices: Vec<_> = r
        .detectors
        .iter()
        .enumerate()
        .filter(|(_, d)| match codec {
            Codec::Aac => d.id.starts_with("aac_quantization_lattice_"),
            Codec::Vorbis => d.id == "vorbis_transform_grid",
        })
        .map(|(i, _)| i)
        // At most two expected slots. Keep one extra index to report duplicates
        // without retaining arbitrarily many invalid externally supplied records.
        .take(3)
        .collect();
    if indices.len() != slots.len()
        || slots.iter().any(|(id, channel)| {
            indices
                .iter()
                .filter(|&&i| {
                    let d = &r.detectors[i];
                    d.id == *id
                        && d.channel_index == *channel
                        && d.version == 1
                        && d.family == "transform_grid"
                })
                .count()
                != 1
        })
    {
        return (Abstain, "missing_duplicate_or_unknown_detector", indices);
    }
    if indices.iter().any(|&i| {
        let d = &r.detectors[i];
        matches!(d.status, DetectorStatus::Hit | DetectorStatus::NotDetected)
            && (d.intervals.is_empty() || d.intervals.iter().any(|p| p.start_frame >= p.end_frame))
    }) {
        return (Abstain, "missing_detector_scope", indices);
    }
    if indices
        .iter()
        .any(|&i| r.detectors[i].status == DetectorStatus::Hit)
    {
        return (Hit, "target_pattern_observed", indices);
    }
    if indices
        .iter()
        .all(|&i| r.detectors[i].status == DetectorStatus::NotDetected)
    {
        return (NoHit, "all_target_bases_no_hit", indices);
    }
    (Abstain, "target_basis_inconclusive_or_unsupported", indices)
}
