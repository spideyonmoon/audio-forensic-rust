//! Interpretation of existing observations, separate from detector arithmetic.
use crate::{
    AnalysisReport, FileStatus,
    model::{DetectorStatus, POLICY_VERSION, SCHEMA_VERSION},
};
use serde::Serialize;
use std::{collections::BTreeMap, fmt};

/// Version of this derived assessment, independent of the measurement schema.
pub const ASSESSMENT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentAvailability {
    Available,
    InputUnavailable,
    UnsupportedReport,
    InvalidCoverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDomain {
    SpectralPatterns,
    CodecTransforms,
    SampleRateHistory,
    IntegerPrecision,
    SourceProfileMeasurements,
    ListeningLevels,
    ChannelRelationships,
    Uninterpreted,
}

impl EvidenceDomain {
    pub fn title(self) -> &'static str {
        match self {
            Self::SpectralPatterns => "Spectral patterns",
            Self::CodecTransforms => "Codec transform patterns",
            Self::SampleRateHistory => "Sample-rate observations",
            Self::IntegerPrecision => "Integer precision",
            Self::SourceProfileMeasurements => "Source-profile measurements",
            Self::ListeningLevels => "Listening levels",
            Self::ChannelRelationships => "Channel relationships",
            Self::Uninterpreted => "Uninterpreted observations",
        }
    }

    pub fn limitation(self) -> &'static str {
        match self {
            Self::SpectralPatterns => {
                "Cutoffs, walls, sparsity and related spectra can share the same filtering cause; they do not identify a codec or source medium."
            }
            Self::CodecTransforms => {
                "AAC and Vorbis patterns are provisional candidates. Multiple matches do not identify a unique codec or establish multiple encoding stages; no match does not prove lossless history."
            }
            Self::SampleRateHistory => {
                "Rate hypotheses are not verified original rates or evidence of lossy encoding. Their wall evidence can overlap the spectral group."
            }
            Self::IntegerPrecision => {
                "Unused integer bits describe this interval. Dither, gain and processing can change exercised bits without establishing original recording depth."
            }
            Self::SourceProfileMeasurements => {
                "Quietness, band noise and transient measurements can include music and processing effects; they do not establish vinyl, cassette or digital origin."
            }
            Self::ListeningLevels => {
                "Loudness, range, crest and peak estimates describe levels; they do not establish ancestry or a mastering-quality grade."
            }
            Self::ChannelRelationships => {
                "Channel correlation describes the measured signal; it does not establish fake stereo or source history."
            }
            Self::Uninterpreted => {
                "Unknown families or detector versions are retained for inspection and excluded from interpretation."
            }
        }
    }
}

/// Counts of detector records, not independent evidence or calibrated votes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ObservationCounts {
    pub hit: usize,
    pub measured: usize,
    pub not_detected: usize,
    pub inconclusive: usize,
    pub unsupported: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceGroup {
    pub domain: EvidenceDomain,
    /// Indices into the original report.detectors; resolve there for exact
    /// channels/bases, intervals, values, thresholds and detector caveats.
    pub detector_indices: Vec<usize>,
    pub counts: ObservationCounts,
    /// Each matching method appears once across native channels and AAC bases.
    /// This list is descriptive; its length is never used as a score.
    pub provisional_patterns: Vec<String>,
    pub limitation: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct InferenceBoundary {
    pub subject: &'static str,
    pub verdict: &'static str,
    pub reason: &'static str,
}

/// Derived non-MQA interpretation. Store it alongside the original report if
/// serialized: detector indices refer to that report's unchanged record order.
/// No probabilities, weighted scores or source-history labels are produced.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceAssessment {
    pub assessment_version: u32,
    pub availability: AssessmentAvailability,
    pub analyzed_frames: Option<u64>,
    pub reached_end: Option<bool>,
    pub groups: Vec<EvidenceGroup>,
    /// MQA observations are preserved in the original report, not interpreted.
    pub deferred_mqa_indices: Vec<usize>,
    pub conclusions: Vec<InferenceBoundary>,
    pub caveats: Vec<&'static str>,
}

fn domain(family: &str) -> EvidenceDomain {
    match family {
        "spectral_wall" | "spectral_measurements" | "spectral_structure" => {
            EvidenceDomain::SpectralPatterns
        }
        "transform_grid" => EvidenceDomain::CodecTransforms,
        "sample_rate_history" => EvidenceDomain::SampleRateHistory,
        "bit_depth" => EvidenceDomain::IntegerPrecision,
        "noise_measurements" | "transient_measurements" => {
            EvidenceDomain::SourceProfileMeasurements
        }
        "listening_level" => EvidenceDomain::ListeningLevels,
        "channel_measurements" => EvidenceDomain::ChannelRelationships,
        _ => EvidenceDomain::Uninterpreted,
    }
}

fn pattern(id: &str) -> Option<&'static str> {
    match id {
        "aac_quantization_lattice_mono"
        | "aac_quantization_lattice_mid"
        | "aac_quantization_lattice_side" => Some("AAC quantization lattice"),
        "vorbis_transform_grid" => Some("Vorbis transform grid"),
        "segment_wall" => Some("repeated or mixed spectral walls"),
        "resampling_candidates" => Some("sample-rate hypotheses"),
        "integer_precision" => Some("unused low-order integer bits"),
        _ => None,
    }
}

/// Group related current-policy detector records without counting channels,
/// AAC bases, wall matches or sample-rate hypotheses as independent votes.
/// Unsupported contracts and unsuccessful files produce no interpreted groups.
/// This is not a replacement for validating an externally supplied JSON report.
pub fn assess_evidence(report: &AnalysisReport) -> EvidenceAssessment {
    let mut assessment = EvidenceAssessment {
        assessment_version: ASSESSMENT_VERSION,
        availability: AssessmentAvailability::Available,
        analyzed_frames: None,
        reached_end: None,
        groups: vec![],
        deferred_mqa_indices: vec![],
        conclusions: vec![
            InferenceBoundary {
                subject: "Lossy ancestry",
                verdict: "INCONCLUSIVE",
                reason: "No independently validated aggregation policy is available; neither pattern hits nor their absence establishes ancestry.",
            },
            InferenceBoundary {
                subject: "Source medium",
                verdict: "INCONCLUSIVE",
                reason: "Source-profile measurements are implemented, but vinyl/cassette/digital classification lacks independently characterized validation.",
            },
            InferenceBoundary {
                subject: "Original bit depth",
                verdict: "INCONCLUSIVE",
                reason: "Exercised integer precision is measured; original recording depth cannot be established from it alone.",
            },
        ],
        caveats: vec![
            "Related detector records are grouped for interpretation, not summed. Groups are not proven statistically independent; no evidence score is assigned.",
            "Counts describe detector records, including channels/bases. Inconclusive and unsupported records remain visible and are never treated as negative evidence.",
            "The analyzed prefix bounds every finding; individual detectors may cover only their listed intervals. Read the original records for applicability and caveats.",
            "MQA interpretation is deferred; existing signalling observations remain in the original report.",
        ],
    };
    if report.schema_version != SCHEMA_VERSION || report.policy_version != POLICY_VERSION {
        assessment.availability = AssessmentAvailability::UnsupportedReport;
        return assessment;
    }
    if report.status != FileStatus::Analyzed {
        assessment.availability = AssessmentAvailability::InputUnavailable;
        return assessment;
    }
    let (Some(coverage), Some(stream)) = (&report.coverage, &report.stream) else {
        assessment.availability = AssessmentAvailability::InvalidCoverage;
        return assessment;
    };
    if !(1..=2).contains(&stream.channels)
        || report.detectors.iter().any(|d| {
            d.channel_index.is_some_and(|c| c >= stream.channels)
                || d.intervals
                    .iter()
                    .any(|i| i.start_frame > i.end_frame || i.end_frame > coverage.analyzed_frames)
        })
    {
        assessment.availability = AssessmentAvailability::InvalidCoverage;
        return assessment;
    }
    assessment.analyzed_frames = Some(coverage.analyzed_frames);
    assessment.reached_end = Some(coverage.reached_end);
    let mut groups = BTreeMap::<EvidenceDomain, EvidenceGroup>::new();
    for (index, detector) in report.detectors.iter().enumerate() {
        if detector.family == "mqa_signalling" {
            assessment.deferred_mqa_indices.push(index);
            continue;
        }
        let domain = if detector.version == 1 {
            domain(&detector.family)
        } else {
            EvidenceDomain::Uninterpreted
        };
        let group = groups.entry(domain).or_insert_with(|| EvidenceGroup {
            domain,
            detector_indices: vec![],
            counts: ObservationCounts::default(),
            provisional_patterns: vec![],
            limitation: domain.limitation(),
        });
        group.detector_indices.push(index);
        match detector.status {
            DetectorStatus::Hit => group.counts.hit += 1,
            DetectorStatus::Measured => group.counts.measured += 1,
            DetectorStatus::NotDetected => group.counts.not_detected += 1,
            DetectorStatus::Inconclusive => group.counts.inconclusive += 1,
            DetectorStatus::Unsupported => group.counts.unsupported += 1,
        }
        if detector.status == DetectorStatus::Hit && domain != EvidenceDomain::Uninterpreted {
            if let Some(name) = pattern(&detector.id) {
                if !group.provisional_patterns.iter().any(|s| s == name) {
                    group.provisional_patterns.push(name.into());
                }
            }
        }
    }
    assessment.groups = groups.into_values().collect();
    for group in &mut assessment.groups {
        group.provisional_patterns.sort();
    }
    assessment
}

impl fmt::Display for EvidenceAssessment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Evidence interpretation: {:?}", self.availability)?;
        if let Some(frames) = self.analyzed_frames {
            writeln!(
                f,
                "  Scope: {frames} native frames; reached stream end: {}. Detector intervals may be narrower.",
                self.reached_end == Some(true)
            )?;
        }
        for group in &self.groups {
            let c = &group.counts;
            writeln!(
                f,
                "  {}: {} hit, {} measured, {} no match, {} inconclusive, {} unsupported records",
                group.domain.title(),
                c.hit,
                c.measured,
                c.not_detected,
                c.inconclusive,
                c.unsupported
            )?;
            if !group.provisional_patterns.is_empty() {
                writeln!(
                    f,
                    "    Provisional patterns: {}",
                    group.provisional_patterns.join(", ")
                )?;
            }
            writeln!(f, "    {}", group.limitation)?;
        }
        for conclusion in &self.conclusions {
            writeln!(
                f,
                "  {}: {}. {}",
                conclusion.subject, conclusion.verdict, conclusion.reason
            )?;
        }
        for caveat in &self.caveats {
            writeln!(f, "  {caveat}")?;
        }
        Ok(())
    }
}
