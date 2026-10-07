//! Independent forensic product reporting and same-track reference comparison.
use crate::{
    AnalysisReport, CancellationToken, FileStatus,
    byproducts::ByproductReport,
    metadata::{MetadataReport, MetadataStatus, ReplayGainAudit, audit_metadata_replaygain},
    model::Diagnostic,
    reference_assessment::{ReferenceAssessment, assess_reference, read_assessment_json},
    reference_inputs::{ReferenceAnalysis, ReferenceInputs},
    spectrogram::{RasterExport, SpectrogramAnalysis, SpectrogramArtifact},
    spectrogram_png::{CanvasOptions, SpectrogramPresentation},
    tool_statistics::{ToolAnalysis, ToolStatisticsReport},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Write as _, path::Path, time::Duration};

pub const PRODUCT_SCHEMA: &str = "audio-forensic-product-v1";

/// Pinned Python aliases point into the complete typed report; values/units and
/// applicability are never inferred from the alias's legacy name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldAlias {
    pub value_paths: Vec<String>,
    pub unit: String,
    pub domain: String,
    pub scope_paths: Vec<String>,
    pub qualification: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductArtifacts {
    pub spectrogram: Option<SpectrogramArtifact>,
    #[serde(default)]
    pub presentation: Option<SpectrogramPresentation>,
    #[serde(default)]
    pub title: Option<String>,
    pub exports: Vec<RasterExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductReport {
    pub product_schema_version: String,
    pub engine_version: String,
    pub contract_version: u32,
    pub measurement_report: AnalysisReport,
    pub metadata: MetadataReport,
    pub reference_inputs: Option<ReferenceInputs>,
    pub reference_assessment: ReferenceAssessment,
    pub byproducts: Option<ByproductReport>,
    pub tool_statistics: Option<ToolStatisticsReport>,
    pub replaygain_audit: ReplayGainAudit,
    pub artifacts: ProductArtifacts,
    pub diagnostics: Vec<Diagnostic>,
    pub analysis_seconds: Option<f64>,
    pub field_aliases: BTreeMap<String, FieldAlias>,
}

fn equal(a: &impl Serialize, b: &impl Serialize) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

impl ProductReport {
    pub(crate) fn from_collected(
        tool: ToolAnalysis,
        metadata: MetadataReport,
        inputs: Option<ReferenceInputs>,
        spectrum: Option<SpectrogramAnalysis>,
    ) -> Self {
        let reference = ReferenceAnalysis::new(tool.measurement.clone(), inputs);
        let assessment = assess_reference(&reference);
        let replaygain_audit = audit_metadata_replaygain(&metadata, &tool.measurement);
        let title = metadata.named_value("title").map(str::to_owned);
        let (spectrogram, presentation) = match spectrum {
            Some(s) => (Some(s.spectrogram), s.presentation),
            None => (None, None),
        };
        let mut product = Self {
            product_schema_version: PRODUCT_SCHEMA.into(),
            engine_version: tool.measurement.engine_version.clone(),
            contract_version: 1,
            diagnostics: tool.measurement.diagnostics.clone(),
            analysis_seconds: None,
            field_aliases: BTreeMap::new(),
            measurement_report: tool.measurement,
            metadata,
            reference_inputs: reference.reference_inputs,
            reference_assessment: assessment,
            byproducts: Some(tool.byproducts),
            tool_statistics: Some(tool.tool_statistics),
            replaygain_audit,
            artifacts: ProductArtifacts {
                spectrogram,
                presentation,
                title,
                exports: vec![],
            },
        };
        product.refresh_aliases();
        product
    }

    /// Rebuild presentation aliases from stored fields only. This has no policy
    /// effects, decoding or filesystem access.
    pub fn refresh_aliases(&mut self) {
        let snapshot = serde_json::json!({"metadata":self.metadata,
            "measurement_report":self.measurement_report,"byproducts":self.byproducts,
            "tool_statistics":self.tool_statistics,"reference_inputs":self.reference_inputs,
            "reference_assessment":self.reference_assessment,"replaygain_audit":self.replaygain_audit,
            "analysis_seconds":self.analysis_seconds,"artifacts":{"exports":self.artifacts.exports}});
        let mut aliases: BTreeMap<String, FieldAlias> =
            serde_json::from_str(include_str!("../assets/product-field-aliases-v1.json"))
                .expect("reviewed alias map");
        for alias in aliases.values_mut() {
            for path in &mut alias.value_paths {
                if let Some(tag) = path.strip_prefix("tag:") {
                    *path = self
                        .metadata
                        .named_tags
                        .get(tag)
                        .and_then(|v| v.first())
                        .map(|i| format!("/metadata/entries/{i}/value"))
                        .unwrap_or_else(|| format!("/metadata/named_tags/{tag}"));
                }
            }
            alias.status = if alias
                .value_paths
                .iter()
                .any(|path| snapshot.pointer(path).is_some_and(|v| !v.is_null()))
            {
                "available"
            } else {
                "unavailable"
            }
            .into();
            if (alias.unit.contains("audit") || alias.qualification.starts_with("Uncalibrated"))
                && alias.status == "available"
            {
                alias.status = "audit_only".into();
            }
        }
        self.field_aliases = aliases;
    }

    pub fn input_failure(report: AnalysisReport) -> Self {
        let mut metadata = MetadataReport::new(&report.source);
        metadata.diagnostics = report.diagnostics.clone();
        metadata.text_limits.complete = false;
        Self::from_collected(ToolAnalysis::new(report, None, None), metadata, None, None)
    }

    /// Reject unknown versions and stale/mismatched saved results before display.
    /// Re-evaluates only stored reference inputs; never reads the original audio.
    pub fn validate(&self) -> Result<(), String> {
        if self
            .analysis_seconds
            .is_some_and(|s| !s.is_finite() || s < 0.)
        {
            return Err("Invalid elapsed analysis time".into());
        }
        let m = &self.measurement_report;
        if self.product_schema_version != PRODUCT_SCHEMA
            || self.contract_version != 1
            || self.engine_version != m.engine_version
            || m.schema_version != crate::model::SCHEMA_VERSION
            || m.policy_version != crate::model::POLICY_VERSION
            || m.ancestry_verdict != "INCONCLUSIVE"
            || m.evidence_index.is_some()
            || self.metadata.metadata_version != 1
            || self.metadata.contract_version != 1
            || self.metadata.source != m.source
        {
            return Err("Unsupported product/measurement/metadata version or identity".into());
        }
        read_assessment_json(
            &serde_json::to_string(&self.reference_assessment).map_err(|e| e.to_string())?,
        )?;
        let expected = assess_reference(&ReferenceAnalysis {
            measurement: m.clone(),
            reference_inputs: self.reference_inputs.clone(),
        });
        if !equal(&expected, &self.reference_assessment) {
            return Err("Saved reference assessment does not match its bound inputs".into());
        }
        if self.reference_inputs.is_some() && expected.status == "failed" {
            return Err("Invalid saved reference binding".into());
        }
        if m.status != FileStatus::Analyzed && self.reference_inputs.is_some() {
            return Err("Unsuccessful measurement retains stale reference inputs".into());
        }
        if m.status == FileStatus::Analyzed {
            let (Some(s), Some(c)) = (&m.stream, &m.coverage) else {
                return Err("Analyzed measurement lacks stream/coverage".into());
            };
            if !(8000..=384000).contains(&s.sample_rate)
                || !(1..=2).contains(&s.channels)
                || c.start_seconds != 0.
                || c.end_seconds != c.analyzed_frames as f64 / s.sample_rate as f64
                || c.analysis_passes != 2
                || m.channels.len() != s.channels
            {
                return Err("Invalid native stream/coverage".into());
            }
            if self.metadata.status == MetadataStatus::Available {
                let t = self
                    .metadata
                    .technical
                    .as_ref()
                    .ok_or("Missing technical metadata")?;
                if t.selected_track_id != s.track_id
                    || t.declared_sample_rate_hz != Some(s.sample_rate)
                    || t.declared_channels != Some(s.channels as u32)
                {
                    return Err("Metadata belongs to a different stream".into());
                }
            }
        }
        for indices in self.metadata.named_tags.values() {
            if indices.iter().any(|i| *i >= self.metadata.entries.len()) {
                return Err("Invalid named metadata entry index".into());
            }
        }
        if let Some(b) = &self.byproducts {
            if b.byproduct_version != 1
                || b.contract_version != 1
                || b.method_id != crate::byproducts::BYPRODUCT_METHOD
                || b.source != m.source
                || b.status != m.status
                || !equal(&b.stream, &m.stream)
                || !equal(&b.coverage, &successful_coverage(m))
                || (m.status != FileStatus::Analyzed
                    && (b.reference.is_some() || b.native_levels.is_some()))
            {
                return Err("Invalid saved byproduct binding/version".into());
            }
        }
        if let Some(t) = &self.tool_statistics {
            if let (Some(data), Some(stream)) = (&t.measurements, &m.stream) {
                let astats = if stream.integer_pcm {
                    if stream.bits_per_sample.is_some_and(|b| b <= 16) {
                        "s16 / 32767"
                    } else {
                        "MSB-aligned s32 / 2147483647"
                    }
                } else if stream.bits_per_sample == Some(32) {
                    "native f32 widened to f64"
                } else {
                    "native f64"
                };
                let sox = if stream.integer_pcm {
                    "native MSB-aligned signed32"
                } else if stream.bits_per_sample == Some(32) {
                    "f32: scaled truncation/saturation to signed32"
                } else {
                    "f64: scaled half-away rounding/saturation to signed32"
                };
                if data.astats.input_domain != astats
                    || data.sox.input_domain != sox
                    || data.dr.input_domain
                        != "native lanes converted to f32, three-second RMS/peak histograms"
                {
                    return Err("Invalid saved tool statistics domains".into());
                }
            }
            if t.statistics_version != 1
                || t.contract_version != 1
                || t.method_id != crate::tool_statistics::TOOL_STATISTICS_METHOD
                || t.source != m.source
                || t.status != m.status
                || !equal(&t.stream, &m.stream)
                || !equal(&t.coverage, &successful_coverage(m))
                || (m.status != FileStatus::Analyzed && t.measurements.is_some())
            {
                return Err("Invalid saved tool statistics binding/version".into());
            }
        }
        if !equal(
            &self.replaygain_audit,
            &audit_metadata_replaygain(&self.metadata, m),
        ) {
            return Err("Invalid saved ReplayGain audit binding".into());
        }
        if let Some(s) = &self.artifacts.spectrogram {
            SpectrogramAnalysis {
                measurement: m.clone(),
                spectrogram: s.clone(),
                presentation: self.artifacts.presentation.clone(),
            }
            .validate_binding()
            .map_err(|e| e.to_string())?;
        } else if self.artifacts.presentation.is_some() {
            return Err("Presentation without spectrogram".into());
        }
        for e in &self.artifacts.exports {
            if (e.status == crate::spectrogram::ArtifactStatus::Available) != e.path.is_some() {
                return Err("Invalid saved export availability/path".into());
            }
        }
        Ok(())
    }

    pub fn export_png(
        &mut self,
        path: impl AsRef<Path>,
        options: &CanvasOptions,
        cancel: &CancellationToken,
        deadline: Duration,
    ) -> RasterExport {
        if self.artifacts.exports.len() >= 32 {
            return RasterExport {
                status: crate::spectrogram::ArtifactStatus::Failed,
                path: None,
                reason: Some(
                    "Product export history limit is 32; start a new export record".into(),
                ),
            };
        }
        let mut options = options.clone();
        if options.title.is_none() {
            options.title = self.artifacts.title.clone();
        }
        let result = self.validate().and_then(|_| {
            self.artifacts
                .spectrogram
                .clone()
                .ok_or_else(|| "Spectrogram was not collected".into())
        });
        let export = match result {
            Ok(spectrogram) => SpectrogramAnalysis {
                measurement: self.measurement_report.clone(),
                spectrogram,
                presentation: self.artifacts.presentation.clone(),
            }
            .write_png_new(path, &options, cancel, deadline),
            Err(reason) => RasterExport {
                status: crate::spectrogram::ArtifactStatus::Failed,
                path: None,
                reason: Some(reason),
            },
        };
        if options.title.is_some() {
            self.artifacts.title = options.title;
        }
        if let Some(reason) = &export.reason {
            self.diagnostics.push(Diagnostic {
                code: "artifact_export".into(),
                message: reason.clone(),
            });
        }
        self.artifacts.exports.push(export.clone());
        self.refresh_aliases();
        export
    }
}

fn successful_coverage(m: &AnalysisReport) -> Option<crate::model::Coverage> {
    (m.status == FileStatus::Analyzed)
        .then(|| m.coverage.clone())
        .flatten()
}

/// A single object or an array is accepted. Unknown envelopes never fall back to
/// interpreting JSON as an audio filename or as a different report version.
pub fn read_product_json(json: &str) -> Result<Vec<ProductReport>, String> {
    // Typed direct parsing also rejects duplicate struct fields and preserves u64.
    let mut reports: Vec<ProductReport> = if json.trim_start().starts_with('[') {
        serde_json::from_str(json).map_err(|e| e.to_string())?
    } else {
        vec![serde_json::from_str(json).map_err(|e| e.to_string())?]
    };
    if reports.is_empty() {
        return Err("Empty product report batch".into());
    }
    for p in &mut reports {
        p.validate()?;
        p.refresh_aliases();
    }
    Ok(reports)
}

fn dump(text: &mut String, label: &str, value: &impl Serialize) {
    // Pretty structured fields retain every measurement, unit, scope, null and
    // audit alias. JSON escaping prevents tag control characters altering output.
    let _ = writeln!(
        text,
        "{label}:\n{}",
        serde_json::to_string_pretty(value).expect("serializable product")
    );
}

pub fn render_product(p: &ProductReport) -> Result<String, String> {
    p.validate()?;
    let mut text = format!(
        "{}: {:?}\nReference method (uncalibrated): {}\n",
        serde_json::to_string(&p.measurement_report.source).unwrap(),
        p.measurement_report.status,
        p.reference_assessment.display_summary
    );
    text.push_str("Scores are not probabilities or proof of authenticity, source or sound quality.\nNative ancestry: INCONCLUSIVE; evidence index: unavailable.\n");
    dump(
        &mut text,
        "Reference scores",
        &p.reference_assessment.scores,
    );
    dump(
        &mut text,
        "Source candidates",
        &p.reference_assessment.source_candidates,
    );
    dump(
        &mut text,
        "Depth candidates",
        &p.reference_assessment.depth_candidates,
    );
    dump(
        &mut text,
        "Metadata (read-only declarations and raw tags)",
        &p.metadata,
    );
    dump(
        &mut text,
        "Native measurements (channels, units, intervals and caveats)",
        &p.measurement_report,
    );
    dump(
        &mut text,
        "Byproducts / native level display",
        &p.byproducts,
    );
    dump(
        &mut text,
        "Tool statistics (distinct DR and legacy aliases)",
        &p.tool_statistics,
    );
    dump(&mut text, "ReplayGain legacy audit", &p.replaygain_audit);
    dump(&mut text, "Reference inputs", &p.reference_inputs);
    dump(
        &mut text,
        "Reference features / rules / qualified audit",
        &p.reference_assessment,
    );
    // Avoid printing the large spectral matrix; it stays available in product JSON.
    dump(
        &mut text,
        "Spectrogram status",
        &p.artifacts
            .spectrogram
            .as_ref()
            .map(|s| (&s.status, &s.reason, &s.coverage)),
    );
    dump(&mut text, "Spectrogram title", &p.artifacts.title);
    dump(
        &mut text,
        "Spectrogram exports (saved paths; existence not rechecked)",
        &p.artifacts.exports,
    );
    dump(&mut text, "Product diagnostics", &p.diagnostics);
    let mut presentation = p.clone();
    presentation.refresh_aliases();
    dump(
        &mut text,
        "Python field aliases (status, units, domains, scopes and audit qualification)",
        &presentation.field_aliases,
    );
    dump(&mut text, "Elapsed analysis seconds", &p.analysis_seconds);
    Ok(text)
}

pub fn render_batch(reports: &[ProductReport]) -> String {
    let mut out = String::from(
        "Batch summary — uncalibrated reference method\nTrack | Status | Main | DR (legacy first channel) | Native LUFS | Reference status\n",
    );
    for p in reports {
        let dr = p
            .tool_statistics
            .as_ref()
            .and_then(|t| t.measurements.as_ref())
            .map(|t| t.dr.legacy_python_label.as_str());
        let lufs = p
            .measurement_report
            .loudness
            .as_ref()
            .and_then(|l| l.integrated_lufs);
        let main = p.reference_assessment.scores.as_ref().map(|s| s.main);
        let _ = writeln!(
            out,
            "{} | {:?} | {:?} | {:?} | {:?} | {}",
            serde_json::to_string(&p.measurement_report.source).unwrap(),
            p.measurement_report.status,
            main,
            dr,
            lufs,
            p.reference_assessment.status
        );
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonKey {
    pub available: bool,
    pub main_score: Option<i32>,
    pub rate_history_flag: Option<bool>,
    pub cutoff_hz: Option<f64>,
    pub depth_category: Option<u8>,
    pub legacy_dr_integer: Option<i32>,
    pub precision_times_rate: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonEntry {
    pub input_index: usize,
    pub source: String,
    pub rank: Option<usize>,
    pub winner: bool,
    pub key: ComparisonKey,
    pub missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub comparison_schema_version: String,
    pub method_id: String,
    pub status: String,
    pub winner_input_index: Option<usize>,
    pub criteria: Vec<String>,
    pub caveats: Vec<String>,
    pub ranking: Vec<ComparisonEntry>,
}

pub fn comparison_key(p: &ProductReport) -> ComparisonKey {
    let a = &p.reference_assessment;
    let resample = a
        .source_candidates
        .iter()
        .find(|c| c.id == "resampling")
        .and_then(|c| c.matched);
    let bandwidth = a
        .source_candidates
        .iter()
        .find(|c| c.id == "bandwidth")
        .and_then(|c| c.matched);
    let depth = a.legacy_outputs["depth"]
        .as_array()
        .and_then(|v| v.first())
        .and_then(|d| d["legacy_text"].as_str())
        .map(|s| {
            if s.contains('⚠') {
                2
            } else if s.contains('~') {
                1
            } else {
                0
            }
        });
    ComparisonKey {
        available: p.measurement_report.status == FileStatus::Analyzed
            && a.status == "available"
            && a.scores.is_some()
            && a.reference_label
                .as_deref()
                .is_some_and(|v| v != "INCONCLUSIVE"),
        main_score: a.scores.as_ref().map(|s| s.main),
        rate_history_flag: match (resample, bandwidth) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },
        cutoff_hz: a.features.get("cutoff").and_then(|f| f.value),
        depth_category: depth,
        legacy_dr_integer: p
            .tool_statistics
            .as_ref()
            .and_then(|t| t.measurements.as_ref())
            .and_then(|t| {
                t.dr.legacy_python_label
                    .trim()
                    .strip_prefix("DR")
                    .and_then(|v| v.trim().parse().ok())
            }),
        precision_times_rate: p.measurement_report.stream.as_ref().and_then(|s| {
            s.bits_per_sample
                .map(|b| u64::from(b) * u64::from(s.sample_rate))
        }),
    }
}

fn order(a: &ComparisonKey, b: &ComparisonKey) -> std::cmp::Ordering {
    (!a.available)
        .cmp(&(!b.available))
        .then_with(|| {
            a.main_score
                .unwrap_or(999)
                .cmp(&b.main_score.unwrap_or(999))
        })
        .then_with(|| {
            a.rate_history_flag
                .unwrap_or(false)
                .cmp(&b.rate_history_flag.unwrap_or(false))
        })
        .then_with(|| {
            b.cutoff_hz
                .unwrap_or(0.)
                .total_cmp(&a.cutoff_hz.unwrap_or(0.))
        })
        .then_with(|| {
            a.depth_category
                .unwrap_or(0)
                .cmp(&b.depth_category.unwrap_or(0))
        })
        .then_with(|| {
            b.legacy_dr_integer
                .unwrap_or(-1)
                .cmp(&a.legacy_dr_integer.unwrap_or(-1))
        })
        .then_with(|| {
            b.precision_times_rate
                .unwrap_or(0)
                .cmp(&a.precision_times_rate.unwrap_or(0))
        })
}

/// The caller asserts these are variants of ONE track. This does not detect
/// track identity, align samples or assess perceptual quality. Incompatible
/// available candidates retain input order, null ranks and no winner.
pub fn compare_products(reports: &[ProductReport]) -> Result<ComparisonReport, String> {
    if reports.len() < 2 {
        return Err("Comparison requires at least two variants of one track".into());
    }
    for p in reports {
        p.validate()?;
    }
    let mut result = ComparisonReport {
        comparison_schema_version: "audio-forensic-comparison-v1".into(),
        method_id: "python-c6ecce2-reference-tuple-v1".into(), status: "available".into(),
        winner_input_index: None,
        criteria: ["availability", "main score ascending", "rate-history flag ascending", "cutoff Hz descending",
            "legacy first-channel depth category ascending (0/1/2)", "legacy first-channel integer DR descending", "precision bits × rate Hz descending", "stable input order"].map(str::to_owned).to_vec(),
        caveats: vec!["Caller asserts variants of the same track; no alignment or identity verification. Reference-method ranking is not proof of authenticity or perceptual quality.".into(),
            "Unknown tuple fields remain null. Sort-only legacy defaults: main=999, rate flag=false, cutoff=0, depth=0, DR=-1, precision×rate=0. No missing measurement is fabricated.".into(),
            "Compatible candidates require identical versions, reference basis domain and analyzed duration/end status. Different native rates are permitted; unlike coverage/domains are not ranked together.".into()],
        ranking: reports.iter().enumerate().map(|(i,p)| {
            let key = comparison_key(p);
            let missing_fields = [ ("main_score", key.main_score.is_none()), ("rate_history_flag",key.rate_history_flag.is_none()),
                ("cutoff_hz",key.cutoff_hz.is_none()), ("depth_category",key.depth_category.is_none()),
                ("legacy_dr_integer",key.legacy_dr_integer.is_none()), ("precision_times_rate",key.precision_times_rate.is_none())]
                .into_iter().filter(|(_,missing)|*missing).map(|(name,_)|name.into()).collect();
            ComparisonEntry { input_index:i, source:p.measurement_report.source.clone(), rank:None, winner:false, key, missing_fields }
        }).collect(),
    };
    let eligible: Vec<_> = result
        .ranking
        .iter()
        .filter(|e| e.key.available)
        .map(|e| &reports[e.input_index])
        .collect();
    let compatible = eligible
        .windows(2)
        .all(|w| compatibility(w[0]) == compatibility(w[1]));
    if !compatible {
        result.status = "incompatible".into();
        return Ok(result);
    }
    result.ranking.sort_by(|a, b| order(&a.key, &b.key));
    if eligible.is_empty() {
        result.status = "unavailable".into();
    }
    for (i, e) in result.ranking.iter_mut().enumerate() {
        e.rank = Some(i + 1);
        e.winner = i == 0 && e.key.available;
        if e.winner {
            result.winner_input_index = Some(e.input_index);
        }
    }
    Ok(result)
}

fn compatibility(p: &ProductReport) -> serde_json::Value {
    let m = &p.measurement_report;
    serde_json::json!([
        m.schema_version,
        m.policy_version,
        p.reference_assessment.method_id,
        p.reference_inputs
            .as_ref()
            .map(|r| (&r.method, r.version, &r.basis.domain)),
        m.coverage
            .as_ref()
            .map(|c| (c.start_seconds, c.end_seconds, c.reached_end)),
        p.tool_statistics.as_ref().map(|t| (
            &t.method_id,
            t.statistics_version,
            t.measurements.as_ref().map(|m| &m.dr.input_domain)
        ))
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_python_sort_tuple_every_axis_and_stable_ties() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/comparison_reference.json"))
                .unwrap();
        let keys: Vec<ComparisonKey> = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| serde_json::from_value(c["key"].clone()).unwrap())
            .collect();
        let mut indices: Vec<usize> = (0..keys.len()).collect();
        indices.sort_by(|a, b| order(&keys[*a], &keys[*b]));
        assert_eq!(
            serde_json::to_value(indices).unwrap(),
            fixture["expected_order"]
        );
    }
}
