//! Versioned Python byproducts, separate from native measurements and policy.
use crate::{
    AnalysisReport, FileStatus,
    decode::Failure,
    model::{AnalysisInterval, Coverage, Diagnostic, StreamInfo},
};
use serde::{Deserialize, Serialize};

pub const BYPRODUCT_VERSION: u32 = 1;
pub const BYPRODUCT_METHOD: &str = "python-reference-c6ecce2-byproducts-v1";
pub const MAX_SILENCE_SECTIONS: usize = 1024;
/// At most 24 hours of positive 100 ms RMS scalars; never retain PCM.
pub const MAX_RMS_BLOCKS: usize = 864_000;
const CLIP_THRESHOLD: f32 = 1.0 - 1.0 / 32768.0;
const SILENCE_THRESHOLD: f32 = 0.001;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ByproductAvailability {
    Available,
    Unavailable,
    Inapplicable,
    ResourceLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayValue {
    pub availability: ByproductAvailability,
    pub value: Option<f64>,
    pub unit: String,
    pub display: Option<String>,
    pub reason: Option<String>,
}

impl DisplayValue {
    fn measured(value: Option<f64>, unit: &str, places: usize, reason: &str) -> Self {
        let value = value.filter(|v| v.is_finite());
        Self {
            availability: if value.is_some() {
                ByproductAvailability::Available
            } else {
                ByproductAvailability::Unavailable
            },
            value,
            unit: unit.into(),
            display: value.map(|v| format!("{v:.places$}")),
            reason: value.is_none().then(|| reason.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencePhase {
    pub availability: ByproductAvailability,
    pub reason: Option<String>,
    pub block_frames: u64,
    pub complete_blocks: u64,
    pub valid_blocks: u64,
    pub trailing_frames: u64,
    pub denominator_strict_minimum: f64,
    pub mean_correlation: Option<f64>,
    pub display: Option<String>,
    /// Pinned wording for audit only; it is not a validated diagnosis.
    pub legacy_verdict: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceCeilingCount {
    pub threshold_amplitude: f64,
    pub examined_channel_samples: u64,
    pub samples_at_or_above_threshold: u64,
    pub display: String,
    /// Pinned clipping wording for audit only, not proof of audible distortion.
    pub legacy_verdict: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceSection {
    pub interval: AnalysisInterval,
    pub duration_seconds: f64,
    pub ends_near_analyzed_end: bool,
    pub display: String,
    /// The reference's EOF marker is retained only in this audit field.
    pub legacy_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceSilence {
    pub threshold_amplitude: f64,
    pub minimum_run_frames: u64,
    pub total_runs: u64,
    pub total_silent_frames: u64,
    pub total_percent: DisplayValue,
    pub sections: Vec<SilenceSection>,
    pub omitted_sections: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceNoiseFloor {
    pub block_frames: u64,
    pub complete_blocks: u64,
    pub positive_blocks: u64,
    pub trailing_frames: u64,
    pub percentile: f64,
    pub retained_block_limit: usize,
    /// Linear interpolation on strictly positive complete-block RMS amplitudes.
    pub percentile_rms: Option<f64>,
    pub level_dbfs: DisplayValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceByproducts {
    pub basis: String,
    pub interval: AnalysisInterval,
    pub mid_peak: f64,
    pub side_peak: Option<f64>,
    pub native_signal_present: bool,
    pub mid_cancelled_with_native_signal: bool,
    pub phase: ReferencePhase,
    pub ceiling_count: ReferenceCeilingCount,
    pub silence: ReferenceSilence,
    pub noise_floor_fallback: ReferenceNoiseFloor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeChannelDisplay {
    pub channel_index: usize,
    pub sample_peak_dbfs: DisplayValue,
    pub rms_dbfs: DisplayValue,
    pub dc_offset_amplitude: DisplayValue,
    pub sample_crest_linear: DisplayValue,
    pub sample_crest_db: DisplayValue,
    pub estimated_true_peak_dbtp: DisplayValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeLevelDisplay {
    pub method_id: String,
    pub analyzed_interval: AnalysisInterval,
    pub loudness_interval: Option<AnalysisInterval>,
    pub loudness_range_interval: Option<AnalysisInterval>,
    pub channels: Vec<NativeChannelDisplay>,
    pub integrated_lufs: DisplayValue,
    pub range_lu: DisplayValue,
    pub momentary_max_lufs: DisplayValue,
    pub short_term_max_lufs: DisplayValue,
    pub fixed_minus16_delta_db: DisplayValue,
    pub fixed_minus14_delta_db: DisplayValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByproductReport {
    pub byproduct_version: u32,
    pub contract_version: u32,
    pub method_id: String,
    pub engine_version: String,
    pub source: String,
    pub status: FileStatus,
    pub stream: Option<StreamInfo>,
    pub coverage: Option<Coverage>,
    pub reference: Option<ReferenceByproducts>,
    pub native_levels: Option<NativeLevelDisplay>,
    pub diagnostics: Vec<Diagnostic>,
    pub caveats: Vec<String>,
}

/// Both products come from the same worker, source snapshot and two PCM passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByproductAnalysis {
    pub measurement: AnalysisReport,
    pub byproducts: ByproductReport,
}

impl ByproductAnalysis {
    pub(crate) fn new(measurement: AnalysisReport, reference: Option<ReferenceByproducts>) -> Self {
        let success = measurement.status == FileStatus::Analyzed;
        let byproducts = ByproductReport {
            byproduct_version: BYPRODUCT_VERSION,
            contract_version: 1,
            method_id: BYPRODUCT_METHOD.into(),
            engine_version: measurement.engine_version.clone(),
            source: measurement.source.clone(),
            status: measurement.status.clone(),
            stream: measurement.stream.clone(),
            coverage: if success { measurement.coverage.clone() } else { None },
            reference: if success { reference } else { None },
            native_levels: if success { native_level_display(&measurement) } else { None },
            diagnostics: measurement.diagnostics.clone(),
            caveats: vec![
                "Reference byproducts use f32 mono or mid/side and reconstructed f32 L/R; native measurements remain separate. Complete floor(sr/10)-sample blocks only; phase averages valid blocks equally, not the whole-stream native coefficient.".into(),
                "Ceiling counts use the fixed 16-bit amplitude threshold at every precision. This is not a plateau detector or proof of clipping/audibility; legacy verdicts are uncalibrated audit wording.".into(),
                "Mid silence and positive-block RMS p5 are content measurements, not recording noise or source-history proof. Mid cancellation can hide native-channel signal. Silence percentage uses analyzed frames (D02).".into(),
                "Levels retain the native meter, gates/windows/tails and FIR true-peak estimate (D04); no FFmpeg 48 kHz equivalence or certified meter claim. Deltas are fixed method targets -16/-14, not current platform policies (D11).".into(),
                "Silence retains at most 1024 sections with exact all-run totals. RMS percentile retains at most 864000 positive scalars; exceeding the cap makes only that fallback unavailable. No PCM is retained.".into(),
            ],
        };
        Self {
            measurement,
            byproducts,
        }
    }
}

fn amplitude_db(x: f64) -> Option<f64> {
    (x > 0.0)
        .then(|| 20.0 * x.log10())
        .filter(|v| v.is_finite())
}

/// Map a successful supported measurement report; unavailable meters stay null.
/// This does not calculate astats/SoX aggregates or DR (P03a).
pub fn native_level_display(report: &AnalysisReport) -> Option<NativeLevelDisplay> {
    if report.status != FileStatus::Analyzed
        || report.schema_version != crate::model::SCHEMA_VERSION
        || report.policy_version != crate::model::POLICY_VERSION
    {
        return None;
    }
    let coverage = report.coverage.as_ref()?;
    let l = report.loudness.as_ref()?;
    let unavailable = "Native meter unavailable; retain its original applicability, gate and window counts in the measurement report.";
    let integrated = l.integrated_lufs.filter(|v| v.is_finite());
    let delta = |target: f64| {
        let mut d = DisplayValue::measured(integrated.map(|v| target - v), "dB", 1, unavailable);
        d.display = d.value.map(|v| format!("{v:+.1} dB"));
        d
    };
    Some(NativeLevelDisplay {
        method_id: "native-level-display-v1".into(),
        analyzed_interval: AnalysisInterval {
            start_frame: 0,
            end_frame: coverage.analyzed_frames,
        },
        loudness_interval: l.interval.clone(),
        loudness_range_interval: l.range.interval.clone(),
        channels: report
            .channels
            .iter()
            .map(|c| NativeChannelDisplay {
                channel_index: c.channel_index,
                sample_peak_dbfs: DisplayValue::measured(
                    amplitude_db(c.peak),
                    "dBFS",
                    2,
                    "Zero peak has no finite dBFS level.",
                ),
                rms_dbfs: DisplayValue::measured(
                    amplitude_db(c.rms),
                    "dBFS",
                    2,
                    "Zero/underflowed RMS has no finite dBFS level.",
                ),
                dc_offset_amplitude: DisplayValue::measured(
                    Some(c.dc_offset),
                    "full_scale_amplitude",
                    2,
                    "Nonfinite DC offset.",
                ),
                sample_crest_linear: DisplayValue::measured(
                    c.crest_factor_linear,
                    "linear_ratio",
                    2,
                    "Native sample crest unavailable.",
                ),
                sample_crest_db: DisplayValue::measured(
                    c.crest_factor_db,
                    "dB",
                    2,
                    "Native sample crest unavailable.",
                ),
                estimated_true_peak_dbtp: DisplayValue::measured(
                    report
                        .true_peak
                        .iter()
                        .find(|t| t.channel_index == c.channel_index)
                        .and_then(|t| t.estimated_peak_dbtp),
                    "dBTP_estimate",
                    2,
                    "Native FIR true-peak level unavailable.",
                ),
            })
            .collect(),
        integrated_lufs: DisplayValue::measured(integrated, "LUFS", 2, unavailable),
        range_lu: DisplayValue::measured(l.range.range_lu, "LU", 2, unavailable),
        momentary_max_lufs: DisplayValue::measured(l.momentary_max_lufs, "LUFS", 2, unavailable),
        short_term_max_lufs: DisplayValue::measured(l.short_term_max_lufs, "LUFS", 2, unavailable),
        fixed_minus16_delta_db: delta(-16.0),
        fixed_minus14_delta_db: delta(-14.0),
    })
}

/// Fixed scalar state plus capped RMS/section vectors; only enabled by the new API.
pub(crate) struct ByproductCollector {
    rate: u32,
    channels: usize,
    frames: u64,
    mid_peak: f64,
    side_peak: f64,
    native_signal: bool,
    clipped: u64,
    run_start: Option<u64>,
    runs: u64,
    silent_frames: u64,
    sections: Vec<AnalysisInterval>,
    block_frames: u64,
    in_block: u64,
    blocks: u64,
    valid_blocks: u64,
    correlation_sum: f64,
    means: [f64; 2],
    centered: [f64; 2],
    covariance: f64,
    mid_square_sum: f64,
    positive_blocks: u64,
    rms: Vec<f64>,
    rms_limit: usize,
    rms_overflow: bool,
}

impl ByproductCollector {
    pub(crate) fn new(rate: u32, channels: usize) -> Self {
        Self {
            rate,
            channels,
            frames: 0,
            mid_peak: 0.0,
            side_peak: 0.0,
            native_signal: false,
            clipped: 0,
            run_start: None,
            runs: 0,
            silent_frames: 0,
            sections: Vec::new(),
            block_frames: u64::from((rate / 10).max(1)),
            in_block: 0,
            blocks: 0,
            valid_blocks: 0,
            correlation_sum: 0.0,
            means: [0.0; 2],
            centered: [0.0; 2],
            covariance: 0.0,
            mid_square_sum: 0.0,
            positive_blocks: 0,
            rms: Vec::new(),
            rms_limit: MAX_RMS_BLOCKS,
            rms_overflow: false,
        }
    }

    pub(crate) fn push(&mut self, left: f64, right: Option<f64>) {
        self.native_signal |= left != 0.0 || right.is_some_and(|r| r != 0.0);
        let left = left as f32;
        // Match the reference operation order: convert lanes, add/subtract, /2,
        // then reconstruct in f32 before widening for centered statistics.
        let (mid, side) = right.map_or((left, None), |r| {
            let r = r as f32;
            ((left + r) / 2.0, Some((left - r) / 2.0))
        });
        let l = side.map_or(mid, |s| mid + s);
        let r = side.map(|s| mid - s);
        self.mid_peak = self.mid_peak.max(f64::from(mid.abs()));
        self.side_peak = self.side_peak.max(f64::from(side.unwrap_or(0.0).abs()));
        self.clipped += u64::from(l.abs() >= CLIP_THRESHOLD)
            + u64::from(r.is_some_and(|r| r.abs() >= CLIP_THRESHOLD));
        if mid.abs() < SILENCE_THRESHOLD {
            self.run_start.get_or_insert(self.frames);
        } else {
            self.end_run();
        }
        self.frames += 1;
        self.in_block += 1;
        let x = f64::from(mid);
        self.mid_square_sum += x * x;
        if let Some(r) = r {
            let pair = [f64::from(l), f64::from(r)];
            let d = [pair[0] - self.means[0], pair[1] - self.means[1]];
            for ch in 0..2 {
                self.means[ch] += d[ch] / self.in_block as f64;
                self.centered[ch] += d[ch] * (pair[ch] - self.means[ch]);
            }
            self.covariance += d[0] * (pair[1] - self.means[1]);
        }
        if self.in_block == self.block_frames {
            self.blocks += 1;
            let denom = (self.centered[0] * self.centered[1]).sqrt();
            if self.channels == 2 && denom > 1e-12 {
                self.valid_blocks += 1;
                self.correlation_sum += self.covariance / denom;
            }
            let rms = (self.mid_square_sum / self.block_frames as f64).sqrt();
            if rms > 0.0 {
                self.positive_blocks += 1;
                if self.rms.len() < self.rms_limit && !self.rms_overflow {
                    if self.rms.len() == self.rms.capacity() {
                        self.rms
                            .reserve_exact((self.rms_limit - self.rms.len()).min(4096));
                    }
                    self.rms.push(rms);
                } else {
                    self.rms_overflow = true;
                    self.rms.clear();
                }
            }
            self.in_block = 0;
            self.means = [0.0; 2];
            self.centered = [0.0; 2];
            self.covariance = 0.0;
            self.mid_square_sum = 0.0;
        }
    }

    fn end_run(&mut self) {
        if let Some(start) = self.run_start.take() {
            if self.frames - start >= u64::from(self.rate / 2) {
                self.runs += 1;
                self.silent_frames += self.frames - start;
                if self.sections.len() < MAX_SILENCE_SECTIONS {
                    self.sections.push(AnalysisInterval {
                        start_frame: start,
                        end_frame: self.frames,
                    });
                }
            }
        }
    }

    pub(crate) fn finish(
        mut self,
        reached_end: bool,
        mut control: impl FnMut() -> Result<(), Failure>,
    ) -> Result<ReferenceByproducts, Failure> {
        control()?;
        self.end_run();
        let avg = (self.valid_blocks > 0).then(|| self.correlation_sum / self.valid_blocks as f64);
        let phase_reason = if self.channels == 1 {
            Some("Mono has no reference side/L-R pair; no fabricated coefficient of one.")
        } else if self.blocks == 0 {
            Some("No complete floor(sr/10)-sample block.")
        } else if avg.is_none() {
            Some("No block with centered L/R denominator strictly above 1e-12.")
        } else {
            None
        };
        let phase = ReferencePhase {
            availability: if self.channels == 1 {
                ByproductAvailability::Inapplicable
            } else if avg.is_some() {
                ByproductAvailability::Available
            } else {
                ByproductAvailability::Unavailable
            },
            reason: phase_reason.map(str::to_owned),
            block_frames: self.block_frames,
            complete_blocks: self.blocks,
            valid_blocks: self.valid_blocks,
            trailing_frames: self.in_block,
            denominator_strict_minimum: 1e-12,
            mean_correlation: avg,
            display: avg.map(|v| format!("{v:.3}")),
            legacy_verdict: avg.map(|v| phase_verdict(v).0.into()),
            summary: avg.map(|v| phase_verdict(v).1.into()),
        };
        let percentile_rms = if self.frames >= u64::from(self.rate)
            && self.positive_blocks >= 5
            && !self.rms_overflow
        {
            let rank = (self.rms.len() - 1) as f64 * 0.05;
            let lower = rank.floor() as usize;
            let fraction = rank - lower as f64;
            let lo = *self.rms.select_nth_unstable_by(lower, f64::total_cmp).1;
            control()?;
            let hi = if fraction == 0.0 {
                lo
            } else {
                *self.rms.select_nth_unstable_by(lower + 1, f64::total_cmp).1
            };
            Some(if fraction < 0.5 {
                lo + (hi - lo) * fraction
            } else {
                hi - (hi - lo) * (1.0 - fraction)
            })
        } else {
            None
        };
        control()?;
        let reason = if self.rms_overflow {
            "Positive RMS scalar limit exceeded; no truncated percentile is emitted."
        } else if self.frames < u64::from(self.rate) {
            "Reference requires at least one analyzed second."
        } else {
            "Reference requires at least five strictly positive complete-block RMS values."
        };
        let mut floor =
            DisplayValue::measured(percentile_rms.and_then(amplitude_db), "dBFS", 2, reason);
        if self.rms_overflow {
            floor.availability = ByproductAvailability::ResourceLimit;
        }
        let mut sections = Vec::with_capacity(self.sections.len());
        for interval in self.sections {
            control()?;
            let start = interval.start_frame as f64 / self.rate as f64;
            let end = interval.end_frame as f64 / self.rate as f64;
            let near_end = interval.end_frame >= self.frames.saturating_sub(2);
            let base = format!(
                "{:02}:{:02} → {:02}:{:02} ({:.1}s)",
                (start / 60.0).floor() as u64,
                (start % 60.0).floor() as u64,
                (end / 60.0).floor() as u64,
                (end % 60.0).floor() as u64,
                end - start
            );
            sections.push(SilenceSection {
                interval,
                duration_seconds: end - start,
                ends_near_analyzed_end: near_end,
                display: format!(
                    "{base}{}",
                    if near_end {
                        if reached_end {
                            " → EOF"
                        } else {
                            " → analyzed end"
                        }
                    } else {
                        ""
                    }
                ),
                legacy_display: format!("{base}{}", if near_end { " → EOF" } else { "" }),
            });
        }
        let mut percent = DisplayValue::measured(
            (self.frames > 0).then(|| self.silent_frames as f64 / self.frames as f64 * 100.0),
            "percent",
            1,
            "No analyzed samples.",
        );
        percent.display = percent.value.map(|v| format!("{v:.1}%"));
        Ok(ReferenceByproducts {
            basis: if self.channels == 1 {
                "mono_f32"
            } else {
                "stereo_mid_side_f32_reconstructed_lr_f32"
            }
            .into(),
            interval: AnalysisInterval {
                start_frame: 0,
                end_frame: self.frames,
            },
            mid_peak: self.mid_peak,
            side_peak: (self.channels == 2).then_some(self.side_peak),
            native_signal_present: self.native_signal,
            mid_cancelled_with_native_signal: self.channels == 2
                && self.mid_peak == 0.0
                && self.native_signal,
            phase,
            ceiling_count: ReferenceCeilingCount {
                threshold_amplitude: f64::from(CLIP_THRESHOLD),
                examined_channel_samples: self.frames * self.channels as u64,
                samples_at_or_above_threshold: self.clipped,
                display: self.clipped.to_string(),
                legacy_verdict: clipping_verdict(self.clipped),
                summary: format!(
                    "{} reconstructed channel sample(s) at or above the fixed 16-bit ceiling; audible distortion is unverified.",
                    self.clipped
                ),
            },
            silence: ReferenceSilence {
                threshold_amplitude: f64::from(SILENCE_THRESHOLD),
                minimum_run_frames: u64::from(self.rate / 2),
                total_runs: self.runs,
                total_silent_frames: self.silent_frames,
                total_percent: percent,
                omitted_sections: self.runs - sections.len() as u64,
                sections,
            },
            noise_floor_fallback: ReferenceNoiseFloor {
                block_frames: self.block_frames,
                complete_blocks: self.blocks,
                positive_blocks: self.positive_blocks,
                trailing_frames: self.in_block,
                percentile: 5.0,
                retained_block_limit: self.rms_limit,
                percentile_rms,
                level_dbfs: floor,
            },
        })
    }
}

fn phase_verdict(v: f64) -> (&'static str, &'static str) {
    if v >= 0.9 {
        (
            "Mono-compatible",
            "Reference windows: strongly positive L/R correlation.",
        )
    } else if v >= 0.5 {
        (
            "Normal stereo",
            "Reference windows: positive L/R correlation.",
        )
    } else if v >= 0.0 {
        (
            "Wide stereo",
            "Reference windows: weak/nonnegative L/R correlation.",
        )
    } else if v >= -0.3 {
        (
            "⚠ Possible fake stereo / heavy M-S processing",
            "Reference windows: negative L/R correlation; processing history is unknown.",
        )
    } else {
        (
            "⚠ Phase cancellation — check mono fold-down",
            "Reference windows: strongly negative L/R correlation; check mono fold-down.",
        )
    }
}

fn clipping_verdict(n: u64) -> String {
    if n == 0 {
        "✓ No clipped samples".into()
    } else if n < 10 {
        format!("~ {n} clipped sample(s) — minor")
    } else {
        let digits = n.to_string();
        let mut grouped = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        format!("⚠ {grouped} clipped samples — audible distortion likely")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: Option<f64>, expected: &serde_json::Value, tolerance: f64) {
        match (actual, expected.as_f64()) {
            (Some(a), Some(e)) => assert!(
                (a - e).abs() <= tolerance.max(e.abs() * 1e-12),
                "{a} != {e}"
            ),
            (None, None) => {}
            _ => panic!("nullability disagreement: {actual:?}, {expected}"),
        }
    }

    #[test]
    fn frozen_pinned_python_vectors_match_counts_windows_and_audit_text() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/byproduct_reference.json"))
                .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let rate = case["rate"].as_u64().unwrap() as u32;
            let channels = case["channels"].as_u64().unwrap() as usize;
            let mut c = ByproductCollector::new(rate, channels);
            let limit = case["prefix_frames"].as_u64().unwrap_or(u64::MAX);
            for segment in case["segments"].as_array().unwrap() {
                for _ in 0..segment["repeat"].as_u64().unwrap() {
                    for pair in segment["pattern"].as_array().unwrap() {
                        if c.frames < limit {
                            c.push(
                                pair[0].as_f64().unwrap(),
                                if channels == 2 {
                                    pair[1].as_f64()
                                } else {
                                    None
                                },
                            );
                        }
                    }
                }
            }
            let b = c
                .finish(case["prefix_frames"].is_null(), || Ok(()))
                .unwrap();
            let e = &case["expected"];
            let frames = e["frames"].as_u64().unwrap();
            assert_eq!(b.interval.end_frame, frames, "{}", case["name"]);
            assert_eq!(b.phase.complete_blocks, frames / u64::from(rate / 10));
            assert_eq!(b.phase.trailing_frames, frames % u64::from(rate / 10));
            assert_eq!(b.phase.valid_blocks, e["valid_blocks"].as_u64().unwrap());
            close(b.phase.mean_correlation, &e["correlation"], 1e-10);
            assert_eq!(
                b.phase.display.as_deref().unwrap_or(""),
                e["phase"][0].as_str().unwrap(),
                "{}",
                case["name"]
            );
            assert_eq!(
                b.phase.legacy_verdict.as_deref().unwrap_or(""),
                e["phase"][1].as_str().unwrap()
            );
            assert_eq!(b.ceiling_count.display, e["clipping"][0].as_str().unwrap());
            assert_eq!(
                b.ceiling_count.legacy_verdict,
                e["clipping"][1].as_str().unwrap()
            );
            assert_eq!(
                b.ceiling_count.examined_channel_samples,
                frames * channels as u64
            );
            assert_eq!(
                b.silence.total_percent.display.as_deref().unwrap_or(""),
                e["silence_pct"].as_str().unwrap()
            );
            assert_eq!(
                b.silence.total_runs,
                e["runs"].as_array().unwrap().len() as u64
            );
            for (s, run) in b.silence.sections.iter().zip(e["runs"].as_array().unwrap()) {
                assert_eq!(s.interval.start_frame, run[0].as_u64().unwrap());
                assert_eq!(s.interval.end_frame, run[1].as_u64().unwrap());
            }
            let actual: Vec<_> = b
                .silence
                .sections
                .iter()
                .map(|s| s.legacy_display.clone())
                .collect();
            let expected: Vec<_> = e["sections"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
            assert_eq!(actual, expected);
            close(
                b.noise_floor_fallback.percentile_rms,
                &e["percentile_rms"],
                0.0,
            );
            assert_eq!(
                b.noise_floor_fallback.positive_blocks,
                e["positive_blocks"].as_u64().unwrap()
            );
            assert_eq!(
                b.noise_floor_fallback
                    .level_dbfs
                    .display
                    .as_deref()
                    .unwrap_or(""),
                e["floor"].as_str().unwrap(),
                "{}",
                case["name"]
            );
            if case["name"] == "antiphase" {
                assert!(b.mid_cancelled_with_native_signal);
            }
            let encoded = serde_json::to_string(&b).unwrap();
            let roundtrip: ReferenceByproducts = serde_json::from_str(&encoded).unwrap();
            assert_eq!(roundtrip.interval.end_frame, frames);
        }
    }

    #[test]
    fn phase_threshold_branches_and_count_formatting_are_exact() {
        for (value, expected) in [
            (0.9, "Mono-compatible"),
            (0.5, "Normal stereo"),
            (0.0, "Wide stereo"),
            (-0.3, "⚠ Possible fake stereo / heavy M-S processing"),
            (-0.3000000001, "⚠ Phase cancellation — check mono fold-down"),
        ] {
            assert_eq!(phase_verdict(value).0, expected);
        }
        assert_eq!(
            clipping_verdict(1234567),
            "⚠ 1,234,567 clipped samples — audible distortion likely"
        );
        assert_eq!(
            DisplayValue::measured(Some(1.125), "dB", 2, "")
                .display
                .as_deref(),
            Some("1.12")
        );
        assert_eq!(
            DisplayValue::measured(Some(1.375), "dB", 2, "")
                .display
                .as_deref(),
            Some("1.38")
        );
        assert_eq!(
            DisplayValue::measured(Some(f64::INFINITY), "dB", 2, "nonfinite").value,
            None
        );
    }

    #[test]
    fn bounded_sections_keep_exact_totals_and_rms_cap_never_emits_truncated_percentile() {
        let mut c = ByproductCollector::new(8000, 1);
        c.rms_limit = 5;
        for _ in 0..1026 {
            for _ in 0..4000 {
                c.push(0.0, None);
            }
            c.push(0.25, None);
        }
        assert!(c.rms.capacity() <= 5);
        let b = c.finish(false, || Ok(())).unwrap();
        assert_eq!(b.silence.sections.len(), MAX_SILENCE_SECTIONS);
        assert_eq!(b.silence.total_runs, 1026);
        assert_eq!(b.silence.omitted_sections, 2);
        assert_eq!(b.silence.total_silent_frames, 1026 * 4000);
        assert_eq!(
            b.noise_floor_fallback.level_dbfs.availability,
            ByproductAvailability::ResourceLimit
        );
        assert_eq!(b.noise_floor_fallback.percentile_rms, None);
        assert!(b.noise_floor_fallback.positive_blocks > 5);
    }

    #[test]
    fn finishing_is_cancellable_and_zero_frames_stay_unavailable() {
        let c = ByproductCollector::new(8000, 2);
        assert!(matches!(
            c.finish(false, || Err(Failure::Cancelled)),
            Err(Failure::Cancelled)
        ));
        let b = ByproductCollector::new(8000, 2)
            .finish(true, || Ok(()))
            .unwrap();
        assert_eq!(b.phase.mean_correlation, None);
        assert_eq!(b.silence.total_percent.value, None);
        assert_eq!(b.noise_floor_fallback.percentile_rms, None);
    }
}
