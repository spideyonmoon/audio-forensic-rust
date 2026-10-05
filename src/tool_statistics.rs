//! Bounded independent implementations of pinned tool statistics, not ancestry.
use crate::{
    AnalysisReport, FileStatus,
    byproducts::{ByproductAnalysis, ByproductAvailability, ByproductReport, ReferenceByproducts},
    decode::Failure,
    model::{Coverage, Diagnostic, StreamInfo},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TOOL_STATISTICS_VERSION: u32 = 1;
pub const TOOL_STATISTICS_METHOD: &str = "ffmpeg-7.1.1-sox-14.4.2-v1";
const ENTROPY_BINS: usize = 8192;
const DR_BINS: usize = 32768;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolValue {
    pub availability: ByproductAvailability,
    pub value: Option<f64>,
    pub unit: String,
    pub display: Option<String>,
    pub reason: Option<String>,
    pub source_label: String,
    /// Tool-precision text, including nonfinite/omitted legacy results.
    pub legacy_text: Option<String>,
}

fn text(x: f64, places: usize) -> String {
    if x.is_nan() {
        "nan".into()
    } else {
        format!("{x:.places$}")
    }
}

fn value(x: f64, unit: &str, label: &str, places: usize, reason: Option<&str>) -> ToolValue {
    let finite = x.is_finite() && reason.is_none();
    ToolValue {
        availability: if finite {
            ByproductAvailability::Available
        } else {
            ByproductAvailability::Inapplicable
        },
        value: finite.then_some(x),
        unit: unit.into(),
        display: finite.then(|| text(x, places)),
        reason: (!finite).then(|| {
            reason
                .unwrap_or("Nonfinite result in this input domain")
                .into()
        }),
        source_label: label.into(),
        legacy_text: Some(text(x, places)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstatsChannel {
    pub channel_index: usize,
    pub frames: u64,
    pub minimum_samples: u64,
    pub maximum_samples: u64,
    pub absolute_peak_samples: u64,
    pub zero_crossings: u64,
    pub noise_floor_windows: u64,
    pub fields: BTreeMap<String, ToolValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstatsReport {
    pub input_domain: String,
    pub window_frames: u64,
    pub channels: Vec<AstatsChannel>,
    pub overall: BTreeMap<String, ToolValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoxReport {
    pub input_domain: String,
    pub channel_sample_count: u64,
    pub conversion_clips: u64,
    pub fields: BTreeMap<String, ToolValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrChannel {
    pub channel_index: usize,
    pub numeric_dr: ToolValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrReport {
    pub input_domain: String,
    pub block_frames: u64,
    pub complete_blocks_before_tail: u64,
    pub trailing_frames: u64,
    pub selected_block_target: u64,
    pub channels: Vec<DrChannel>,
    pub overall: ToolValue,
    pub overall_integer_label: Option<String>,
    /// Pinned Python selects the first positive-only matching channel text.
    pub legacy_python_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeasurements {
    pub astats: AstatsReport,
    pub sox: SoxReport,
    pub dr: DrReport,
    /// Pinned last printed astats selection; crest_factor_db is raw linear here.
    pub legacy_loudness_profile: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatisticsReport {
    pub statistics_version: u32,
    pub contract_version: u32,
    pub method_id: String,
    pub engine_version: String,
    pub source: String,
    pub status: FileStatus,
    pub stream: Option<StreamInfo>,
    pub coverage: Option<Coverage>,
    pub measurements: Option<ToolMeasurements>,
    pub diagnostics: Vec<Diagnostic>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnalysis {
    pub measurement: AnalysisReport,
    pub byproducts: ByproductReport,
    pub tool_statistics: ToolStatisticsReport,
}

impl ToolAnalysis {
    pub(crate) fn new(
        report: AnalysisReport,
        reference: Option<ReferenceByproducts>,
        tools: Option<ToolMeasurements>,
    ) -> Self {
        let success = report.status == FileStatus::Analyzed;
        let tool_statistics = ToolStatisticsReport {
            statistics_version: TOOL_STATISTICS_VERSION, contract_version: 1,
            method_id: TOOL_STATISTICS_METHOD.into(), engine_version: report.engine_version.clone(),
            source: report.source.clone(), status: report.status.clone(), stream: report.stream.clone(),
            coverage: if success { report.coverage.clone() } else { None },
            measurements: if success { tools } else { None }, diagnostics: report.diagnostics.clone(),
            caveats: vec![
                "Pinned astats uses native lanes, signed-maximum integer normalization and 50 ms windows. Overall and channel aggregates differ. Its amplitude dynamic range is not DR; noise floor is a rolling signal maximum, not isolated recording noise.".into(),
                "SoX stat uses interleaved signed32 conversion, saturation and cross-channel deltas. Counts and conversion clips do not alter native PCM or establish audible clipping. Float precision/conversion is explicit.".into(),
                "DR reproduces pinned f32 three-second blocks and histogram/tail quirks, including exact-multiple no-data. Overall DR and its truncated label differ from the legacy first matching channel label. No crest/LRA substitute or calibrated quality grade.".into(),
                "Legacy tool text/last-occurrence fields are audit data. Correct crest linear/dB fields remain separate (D04). Finite nonphysical upstream sentinels are suppressed with reasons. No runtime tools, resampling, PCM retention or source-history conclusion.".into(),
            ],
        };
        let ByproductAnalysis {
            measurement,
            byproducts,
        } = ByproductAnalysis::new(report, reference);
        Self {
            measurement,
            byproducts,
            tool_statistics,
        }
    }
}

fn db(x: f64) -> f64 {
    20.0 * x.log10()
}

struct AstLane {
    n: u64,
    min: f64,
    max: f64,
    min_count: u64,
    max_count: u64,
    min_run: f64,
    max_run: f64,
    min_runs: f64,
    max_runs: f64,
    last: f64,
    last_nonzero: f64,
    crossings: u64,
    min_nonzero: f64,
    abs_peak: f64,
    abs_count: u64,
    sum: f64,
    squares: f64,
    ewma: f64,
    lo: f64,
    hi: f64,
    noise: f64,
    noise_count: u64,
    entropy: f64,
    ring: Vec<f64>,
    pos: usize,
    sorted: Vec<f64>,
    front: usize,
    back: usize,
    hist: Vec<u64>,
    mult: f64,
}

impl AstLane {
    fn new(rate: u32) -> Self {
        let window = ((0.05 * f64::from(rate) + 0.5) as usize).max(1);
        Self {
            n: 0,
            min: f64::MAX,
            max: -f64::MAX,
            min_count: 0,
            max_count: 0,
            min_run: 0.0,
            max_run: 0.0,
            min_runs: 0.0,
            max_runs: 0.0,
            last: f64::NAN,
            last_nonzero: 0.0,
            crossings: 0,
            min_nonzero: f64::MAX,
            abs_peak: 0.0,
            abs_count: 0,
            sum: 0.0,
            squares: 0.0,
            ewma: 0.0,
            lo: f64::MAX,
            hi: -f64::MAX,
            noise: f64::NAN,
            noise_count: 0,
            entropy: 0.0,
            ring: vec![0.0; window],
            pos: 0,
            sorted: vec![-1.0; window],
            front: 0,
            back: 0,
            hist: vec![0; ENTROPY_BINS],
            mult: (-1.0 / 0.05 / f64::from(rate)).exp(),
        }
    }

    fn push(&mut self, x: f64) {
        let a = x.abs();
        if a > self.abs_peak {
            self.abs_peak = a;
            self.abs_count = 1;
        } else if a == self.abs_peak {
            self.abs_count += 1;
        }
        if x < self.min {
            self.min = x;
            self.min_count = 1;
            self.min_run = 1.0;
            self.min_runs = 0.0;
        } else if x == self.min {
            self.min_count += 1;
            self.min_run = if x == self.last {
                self.min_run + 1.0
            } else {
                1.0
            };
        } else if self.last == self.min {
            self.min_runs += self.min_run * self.min_run;
        }
        if x > self.max {
            self.max = x;
            self.max_count = 1;
            self.max_run = 1.0;
            self.max_runs = 0.0;
        } else if x == self.max {
            self.max_count += 1;
            self.max_run = if x == self.last {
                self.max_run + 1.0
            } else {
                1.0
            };
        } else if self.last == self.max {
            self.max_runs += self.max_run * self.max_run;
        }
        if x != 0.0 {
            self.min_nonzero = self.min_nonzero.min(a);
            self.crossings += u64::from((x > 0.0) != (self.last_nonzero > 0.0));
            self.last_nonzero = x;
        }
        self.sum += x;
        self.squares += x * x;
        self.ewma = self.ewma * self.mult + (1.0 - self.mult) * x * x;
        self.last = x;
        let drop = self.ring[self.pos];
        self.ring[self.pos] = x;
        self.pos = (self.pos + 1) % self.ring.len();
        self.hist[(a.min(1.0) * 8191.0).round_ties_even() as usize] += 1;
        if self.n >= self.ring.len() as u64 {
            self.lo = self.lo.min(self.ewma);
            self.hi = self.hi.max(self.ewma);
        }
        self.n += 1;
        let noise = self.noise_window(a, drop.abs());
        if self.n >= self.ring.len() as u64 {
            if self.noise.is_nan() || noise < self.noise {
                self.noise = noise;
                self.noise_count = 1;
            } else if noise == self.noise {
                self.noise_count += 1;
            }
        }
    }

    fn noise_window(&mut self, amplitude: f64, expired: f64) -> f64 {
        let size = self.sorted.len();
        let previous = |i: usize| (i + size - 1) % size;
        let mut empty = self.front == self.back && self.sorted[self.front] == -1.0;
        if !empty && expired == self.sorted[self.front] {
            self.sorted[self.front] = -1.0;
            if self.front != self.back {
                self.front = previous(self.front);
            }
            // Pinned cursor equality declares empty even when a value remains.
            // The residual slot then affects insertion/output; do not fix it.
            empty = self.front == self.back;
        }
        if !empty && amplitude >= self.sorted[self.front] {
            loop {
                self.sorted[self.front] = -1.0;
                if self.front == self.back {
                    empty = true;
                    break;
                }
                self.front = previous(self.front);
            }
        }
        while !empty && amplitude >= self.sorted[self.back] {
            self.sorted[self.back] = -1.0;
            if self.back == self.front {
                empty = true;
                break;
            }
            self.back = (self.back + 1) % size;
        }
        if !empty {
            self.back = previous(self.back);
        }
        self.sorted[self.back] = amplitude;
        self.sorted[self.front]
    }

    fn finish(&mut self, check: &mut impl FnMut() -> Result<(), Failure>) -> Result<(), Failure> {
        if self.n < self.ring.len() as u64 {
            self.lo = self.squares / self.n as f64;
            self.hi = self.lo;
        }
        for (i, count) in self.hist.iter().enumerate() {
            if i % 1024 == 0 {
                check()?;
            }
            let p = *count as f64 / self.n as f64;
            if p > 1e-8 {
                self.entropy -= p * p.log2() / 13.0;
            }
        }
        Ok(())
    }

    fn fields(&self) -> BTreeMap<String, ToolValue> {
        let mut m = common_fields(
            self.sum / self.n as f64,
            self.abs_peak,
            self.squares / self.n as f64,
            self.hi,
            self.lo,
            (self.min_runs + self.max_runs) / (self.min_count + self.max_count) as f64,
            (self.min_count + self.max_count) as f64,
            self.noise,
            self.entropy,
            self.n == self.ring.len() as u64,
        );
        let crest = if self.squares == 0.0 {
            1.0
        } else {
            self.abs_peak / (self.squares / self.n as f64).sqrt()
        };
        let silence = (self.squares == 0.0).then_some("Crest is undefined for zero power");
        m.insert(
            "crest_linear".into(),
            value(crest, "ratio", "Crest factor", 6, silence),
        );
        let mut crest_db = value(
            db(crest),
            "dB",
            "Derived 20 log10(Crest factor)",
            6,
            silence,
        );
        crest_db.legacy_text = None;
        m.insert("crest_db".into(), crest_db);
        m.insert(
            "absolute_peak_count".into(),
            value(
                self.abs_count as f64,
                "absolute-peak samples",
                "Abs Peak count",
                6,
                None,
            ),
        );
        m.insert(
            "amplitude_range_db".into(),
            value(
                db(2.0 * self.abs_peak / self.min_nonzero),
                "dB",
                "Dynamic range",
                6,
                None,
            ),
        );
        m.insert(
            "zero_crossings_rate".into(),
            value(
                self.crossings as f64 / self.n as f64,
                "crossings/sample",
                "Zero crossings rate",
                6,
                None,
            ),
        );
        m
    }
}

#[allow(clippy::too_many_arguments)]
fn common_fields(
    dc: f64,
    peak: f64,
    power: f64,
    hi: f64,
    lo: f64,
    flat: f64,
    peaks: f64,
    noise: f64,
    entropy: f64,
    sentinel: bool,
) -> BTreeMap<String, ToolValue> {
    let mut m = BTreeMap::new();
    for (key, x, unit, label) in [
        ("dc_offset", dc, "amplitude", "DC offset"),
        ("peak_dbfs", db(peak), "dBFS", "Peak level dB"),
        ("rms_dbfs", db(power.sqrt()), "dBFS", "RMS level dB"),
        ("rms_peak_dbfs", db(hi.sqrt()), "dBFS", "RMS peak dB"),
        ("rms_trough_dbfs", db(lo.sqrt()), "dBFS", "RMS trough dB"),
        ("flat_factor_db", db(flat), "dB", "Flat factor"),
        ("peak_count", peaks, "extreme samples", "Peak count"),
        ("noise_floor_dbfs", db(noise), "dBFS", "Noise floor dB"),
        ("entropy", entropy, "normalized entropy", "Entropy"),
    ] {
        let reason = (sentinel && key.starts_with("rms_") && key != "rms_dbfs")
            .then_some("Exactly one 50 ms window leaves upstream extrema sentinels");
        let mut v = value(x, unit, label, 6, reason);
        if key == "rms_trough_dbfs" && lo == 1.0 {
            v.legacy_text = None;
        }
        m.insert(key.into(), v);
    }
    m
}

#[derive(Default)]
struct SoxState {
    n: u64,
    clips: u64,
    min: f64,
    max: f64,
    sum: f64,
    norm: f64,
    squares: f64,
    last: f64,
    delta: f64,
    deltas: f64,
    delta_squares: f64,
}

impl SoxState {
    fn push(&mut self, word: i32, clipped: bool) {
        let x = f64::from(word) / f64::from(i32::MAX);
        if self.n == 0 {
            self.min = x;
            self.max = x;
            self.last = x;
        }
        self.min = self.min.min(x);
        self.max = self.max.max(x);
        self.sum += x;
        self.norm += x.abs();
        self.squares += x * x;
        let d = (x - self.last).abs();
        self.delta = self.delta.max(d);
        self.deltas += d;
        self.delta_squares += d * d;
        self.last = x;
        self.n += 1;
        self.clips += u64::from(clipped);
    }

    fn finish(self, rate: u32, channels: usize, domain: String) -> SoxReport {
        let n = self.n as f64;
        let peak = self.max.abs().max(self.min.abs());
        let frequency =
            (self.delta_squares / self.squares).sqrt() * f64::from(rate) / std::f64::consts::TAU;
        let mut fields = BTreeMap::new();
        for (key, x, unit, label, places, reason) in [
            ("samplesRead", n, "channel samples", "Samples read", 0, None),
            (
                "lengthSeconds",
                n / f64::from(rate) / channels as f64,
                "seconds",
                "Length (seconds)",
                6,
                None,
            ),
            (
                "scaledBy",
                f64::from(i32::MAX),
                "signed32 scale",
                "Scaled by",
                1,
                None,
            ),
            (
                "maximumAmplitude",
                self.max,
                "amplitude",
                "Maximum amplitude",
                6,
                None,
            ),
            (
                "minimumAmplitude",
                self.min,
                "amplitude",
                "Minimum amplitude",
                6,
                None,
            ),
            (
                "midlineAmplitude",
                self.min / 2.0 + self.max / 2.0,
                "amplitude",
                "Midline amplitude",
                6,
                None,
            ),
            (
                "meanNorm",
                self.norm / n,
                "amplitude",
                "Mean    norm",
                6,
                None,
            ),
            (
                "meanAmplitude",
                self.sum / n,
                "amplitude",
                "Mean    amplitude",
                6,
                None,
            ),
            (
                "rmsAmplitude",
                (self.squares / n).sqrt(),
                "amplitude",
                "RMS     amplitude",
                6,
                None,
            ),
            (
                "maximumDelta",
                self.delta,
                "amplitude",
                "Maximum delta",
                6,
                None,
            ),
            ("minimumDelta", 0.0, "amplitude", "Minimum delta", 6, None),
            (
                "meanDelta",
                self.deltas / (n - 1.0),
                "amplitude",
                "Mean    delta",
                6,
                (self.n < 2).then_some("Requires two interleaved samples"),
            ),
            (
                "rmsDelta",
                (self.delta_squares / (n - 1.0)).sqrt(),
                "amplitude",
                "RMS     delta",
                6,
                (self.n < 2).then_some("Requires two interleaved samples"),
            ),
            (
                "roughFrequency",
                frequency.trunc(),
                "Hz (interleaved estimate)",
                "Rough   frequency",
                0,
                (self.squares == 0.0 || self.n < 2)
                    .then_some("Requires nonzero power and two interleaved samples"),
            ),
            (
                "volumeAdjustment",
                1.0 / peak,
                "ratio",
                "Volume adjustment",
                3,
                (peak == 0.0).then_some("Undefined at zero amplitude; tool omits field"),
            ),
        ] {
            let mut v = value(x, unit, label, places, reason);
            if self.n == 1 && matches!(key, "meanDelta" | "rmsDelta") {
                // Frozen SoX 14.4.2 Windows printf spelling for the 0/0 case.
                v.legacy_text = Some("1.#INF00".into());
            }
            if key == "roughFrequency" && !frequency.is_finite() {
                v.legacy_text = Some("-2147483648".into());
            }
            if key == "volumeAdjustment" && peak == 0.0 {
                v.legacy_text = None;
            }
            fields.insert(key.into(), v);
        }
        SoxReport {
            input_domain: domain,
            channel_sample_count: self.n,
            conversion_clips: self.clips,
            fields,
        }
    }
}

struct DrLane {
    rms: Vec<u64>,
    peaks: Vec<u64>,
    sum: f32,
    peak: f32,
    n: u64,
    blocks: u64,
}
impl DrLane {
    fn new() -> Self {
        Self {
            rms: vec![0; DR_BINS + 1],
            peaks: vec![0; DR_BINS + 1],
            sum: 0.0,
            peak: 0.0,
            n: 0,
            blocks: 0,
        }
    }
    fn push(&mut self, x: f32, block: u64) {
        self.sum += x * x;
        self.peak = self.peak.max(x.abs());
        self.n += 1;
        if self.n == block {
            self.end_block();
        }
    }
    fn end_block(&mut self) {
        let rms = (2.0f32 * self.sum / self.n as f32).sqrt();
        self.rms[(rms * DR_BINS as f32)
            .round_ties_even()
            .clamp(0.0, DR_BINS as f32) as usize] += 1;
        self.peaks[(self.peak * DR_BINS as f32)
            .round_ties_even()
            .clamp(0.0, DR_BINS as f32) as usize] += 1;
        self.n = 0;
        self.sum = 0.0;
        self.peak = 0.0;
        self.blocks += 1;
    }
    fn finish(
        mut self,
        target: u64,
        check: &mut impl FnMut() -> Result<(), Failure>,
    ) -> Result<f32, Failure> {
        self.end_block();
        let mut first = false;
        let mut peak = DR_BINS;
        for i in (0..=DR_BINS).rev() {
            if i % 1024 == 0 {
                check()?;
            }
            if self.peaks[i] > 0 {
                if first || self.peaks[i] > 1 {
                    peak = i;
                    break;
                }
                first = true;
            }
        }
        let mut sum = 0.0f32;
        let mut count = 0;
        for i in (0..=DR_BINS).rev() {
            if i % 1024 == 0 {
                check()?;
            }
            if count >= target {
                break;
            }
            if self.rms[i] > 0 {
                let a = i as f32 / DR_BINS as f32;
                sum += a * a * self.rms[i] as f32;
                count += self.rms[i];
            }
        }
        Ok(20.0f32 * (peak as f32 / DR_BINS as f32 / (sum / target as f32).sqrt()).log10())
    }
}

/// Six significant digits, matching upstream %g for finite DR text.
fn general(x: f32) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    let sci = format!("{x:.5e}");
    let (mantissa, exponent) = sci.split_once('e').expect("scientific formatter");
    let exp: i32 = exponent.parse().expect("exponent");
    let trim = |s: String| {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            s
        }
    };
    if !(-4..6).contains(&exp) {
        format!("{}e{exp:+03}", trim(mantissa.into()))
    } else {
        trim(format!(
            "{:.p$}",
            sci.parse::<f64>().expect("rounded DR"),
            p = (5 - exp) as usize
        ))
    }
}

pub(crate) struct ToolCollector {
    rate: u32,
    bits: Option<u32>,
    integer: bool,
    lanes: Vec<AstLane>,
    sox: SoxState,
    dr: Vec<DrLane>,
}
impl ToolCollector {
    pub(crate) fn new(info: &StreamInfo) -> Self {
        let channels = info.channels;
        Self {
            rate: info.sample_rate,
            bits: info.bits_per_sample,
            integer: info.integer_pcm,
            lanes: (0..channels)
                .map(|_| AstLane::new(info.sample_rate))
                .collect(),
            sox: SoxState::default(),
            dr: (0..channels).map(|_| DrLane::new()).collect(),
        }
    }
    pub(crate) fn push(&mut self, ch: usize, x: f64, word: Option<i32>) {
        let norm = match word {
            Some(w) if self.bits.is_some_and(|b| b <= 16) => f64::from(w >> 16) / 32767.0,
            Some(w) => f64::from(w) / f64::from(i32::MAX),
            None => x,
        };
        self.lanes[ch].push(norm);
        self.dr[ch].push(x as f32, 3 * u64::from(self.rate));
        let (sample, clip) = match word {
            Some(w) => (w, false),
            None => {
                let scaled = x * 2147483648.0;
                let round = if self.bits == Some(32) {
                    scaled
                } else if scaled < 0.0 {
                    scaled - 0.5
                } else {
                    scaled + 0.5
                };
                let clip = if self.bits == Some(32) {
                    !(-2147483648.0..=2147483648.0).contains(&scaled)
                } else {
                    scaled <= -2147483648.5 || scaled > 2147483648.0
                };
                (round as i32, clip)
            }
        };
        self.sox.push(sample, clip);
    }
    pub(crate) fn finish(
        mut self,
        mut check: impl FnMut() -> Result<(), Failure>,
    ) -> Result<ToolMeasurements, Failure> {
        for lane in &mut self.lanes {
            check()?;
            lane.finish(&mut check)?;
        }
        let n = self.lanes[0].n;
        let count = self.lanes.len();
        let mut dc = 0.0f64;
        let mut peak = 0.0f64;
        let mut squares = 0.0;
        let mut hi = -f64::MAX;
        let mut lo = f64::MAX;
        let mut runs = 0.0;
        let mut peaks = 0u64;
        let mut noise = 0.0;
        let mut entropy = 0.0;
        let mut channels = Vec::new();
        for (i, a) in self.lanes.iter().enumerate() {
            if a.sum.abs() > dc.abs() {
                dc = a.sum;
            }
            peak = peak.max(a.abs_peak);
            squares += a.squares;
            hi = hi.max(a.hi);
            lo = lo.min(a.lo);
            runs += a.min_runs + a.max_runs;
            peaks += a.min_count + a.max_count;
            noise = if noise > a.noise { noise } else { a.noise };
            entropy += a.entropy;
            channels.push(AstatsChannel {
                channel_index: i,
                frames: n,
                minimum_samples: a.min_count,
                maximum_samples: a.max_count,
                absolute_peak_samples: a.abs_count,
                zero_crossings: a.crossings,
                noise_floor_windows: a.noise_count,
                fields: a.fields(),
            });
        }
        let mut overall = common_fields(
            dc / n as f64,
            peak,
            squares / (n as f64 * count as f64),
            hi,
            lo,
            runs / peaks as f64,
            peaks as f64 / count as f64,
            noise,
            entropy / count as f64,
            n == self.lanes[0].ring.len() as u64,
        );
        overall.insert(
            "absolute_peak_count".into(),
            value(
                self.lanes.iter().map(|a| a.abs_count).sum::<u64>() as f64 / count as f64,
                "absolute-peak samples (lane mean)",
                "Abs Peak count",
                6,
                None,
            ),
        );
        let mut legacy = BTreeMap::new();
        for (name, key) in [
            ("peak_db", "peak_dbfs"),
            ("rms_db", "rms_dbfs"),
            ("rms_peak_db", "rms_peak_dbfs"),
            ("rms_trough_db", "rms_trough_dbfs"),
            ("noise_floor_db", "noise_floor_dbfs"),
            ("dynamic_range_db", "amplitude_range_db"),
            ("crest_factor_db", "crest_linear"),
            ("flat_factor", "flat_factor_db"),
            // The pinned unanchored "Peak count:" regex also matches the later
            // "Abs Peak count:" line, and therefore selects this field.
            ("peak_count", "absolute_peak_count"),
            ("sox_entropy", "entropy"),
            ("dc_offset", "dc_offset"),
            ("zero_crossings_rate", "zero_crossings_rate"),
        ] {
            let mut last = None;
            for fields in channels
                .iter()
                .map(|c| &c.fields)
                .chain(std::iter::once(&overall))
            {
                if let Some(t) = fields.get(key).and_then(|v| v.legacy_text.as_ref()) {
                    last = Some(t);
                }
            }
            legacy.insert(
                name.into(),
                last.and_then(|s| s.parse::<f64>().ok())
                    .filter(|v| v.is_finite())
                    .map(|v| format!("{v:.2}")),
            );
        }
        let domain = if self.integer {
            if self.bits.is_some_and(|b| b <= 16) {
                "s16 / 32767"
            } else {
                "MSB-aligned s32 / 2147483647"
            }
        } else if self.bits == Some(32) {
            "native f32 widened to f64"
        } else {
            "native f64"
        };
        let astats = AstatsReport {
            input_domain: domain.into(),
            window_frames: self.lanes[0].ring.len() as u64,
            channels,
            overall,
        };
        let sox_domain = if self.integer {
            "native MSB-aligned signed32"
        } else if self.bits == Some(32) {
            "f32: scaled truncation/saturation to signed32"
        } else {
            "f64: scaled half-away rounding/saturation to signed32"
        };
        let sox = self.sox.finish(self.rate, count, sox_domain.into());
        let complete = self.dr[0].blocks;
        let tail = self.dr[0].n;
        let target = (0.2f32 * complete as f32).round_ties_even() as u64;
        let reason = if tail == 0 {
            Some("Pinned drmeter has no data at exact block multiples")
        } else if target == 0 {
            Some("Pinned top-20% block target rounds to zero")
        } else {
            None
        };
        let mut dr_channels = Vec::new();
        let mut sum = 0.0f32;
        let mut legacy_label = None;
        for (i, lane) in self.dr.into_iter().enumerate() {
            check()?;
            let x = if reason.is_some() {
                f32::NAN
            } else {
                lane.finish(target, &mut check)?
            };
            sum += x;
            let mut numeric_dr = value(f64::from(x), "dB (drmeter)", "Channel DR", 6, reason);
            numeric_dr.legacy_text = (tail != 0).then(|| general(x));
            if let Some(s) = &numeric_dr.legacy_text {
                let leading: String = s
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if legacy_label.is_none() {
                    legacy_label = leading
                        .parse::<f64>()
                        .ok()
                        .map(|v| format!("DR{}", v.trunc() as i64));
                }
            }
            dr_channels.push(DrChannel {
                channel_index: i,
                numeric_dr,
            });
        }
        let mean = sum / count as f32;
        let mut overall_dr = value(f64::from(mean), "dB (drmeter)", "Overall DR", 6, reason);
        overall_dr.legacy_text = (tail != 0).then(|| general(mean));
        if legacy_label.is_none() && mean.is_finite() && mean >= 0.0 {
            let leading: String = general(mean)
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            legacy_label = leading
                .parse::<f64>()
                .ok()
                .map(|v| format!("DR{}", v.trunc() as i64));
        }
        let dr = DrReport {
            input_domain: "native lanes converted to f32, three-second RMS/peak histograms".into(),
            block_frames: 3 * u64::from(self.rate),
            complete_blocks_before_tail: complete,
            trailing_frames: tail,
            selected_block_target: target,
            channels: dr_channels,
            overall: overall_dr,
            overall_integer_label: mean
                .is_finite()
                .then(|| format!("DR{}", mean.trunc() as i64)),
            legacy_python_label: legacy_label.unwrap_or_else(|| "N/A".into()),
        };
        check()?;
        Ok(ToolMeasurements {
            astats,
            sox,
            dr,
            legacy_loudness_profile: legacy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_rate_arrays_stay_fixed_after_many_windows() {
        let mut lane = AstLane::new(384000);
        let mut dr = DrLane::new();
        let capacities = (
            lane.ring.capacity(),
            lane.sorted.capacity(),
            lane.hist.capacity(),
            dr.rms.capacity(),
            dr.peaks.capacity(),
        );
        assert_eq!(
            (lane.ring.len() + lane.sorted.len()) * 8 + lane.hist.len() * 8,
            372736
        );
        assert_eq!((dr.rms.len() + dr.peaks.len()) * 8, 524304);
        for i in 0..100000 {
            let x = (i % 31) as f64 / 32.0;
            lane.push(x);
            dr.push(x as f32, 24);
        }
        assert_eq!(
            capacities,
            (
                lane.ring.capacity(),
                lane.sorted.capacity(),
                lane.hist.capacity(),
                dr.rms.capacity(),
                dr.peaks.capacity()
            )
        );
        assert_eq!(dr.blocks, 4166);
    }

    #[test]
    fn odd_rate_circular_expiration_matches_frozen_tool_edge() {
        let mut lane = AstLane::new(11025);
        for _ in 0..4000 {
            for x in [0.5, -0.125, 0.0] {
                lane.push(x);
            }
        }
        lane.finish(&mut || Ok(())).unwrap();
        assert_eq!(lane.noise, 0.0);
        assert_eq!(lane.fields()["noise_floor_dbfs"].value, None);
        assert_eq!(
            lane.fields()["noise_floor_dbfs"].legacy_text.as_deref(),
            Some("-inf")
        );
    }

    #[test]
    fn reductions_are_cancellable_and_propagate_deadline() {
        let mut lane = AstLane::new(8000);
        lane.push(0.25);
        assert!(matches!(
            lane.finish(&mut || Err(Failure::Cancelled)),
            Err(Failure::Cancelled)
        ));
        let mut dr = DrLane::new();
        for _ in 0..97 {
            dr.push(0.25, 24);
        }
        let mut checks = 0;
        assert!(matches!(
            dr.finish(1, &mut || {
                checks += 1;
                if checks == 2 {
                    Err(Failure::TimedOut)
                } else {
                    Ok(())
                }
            }),
            Err(Failure::TimedOut)
        ));
        assert_eq!(checks, 2);
    }

    #[test]
    fn dr_general_precision_and_scientific_text_are_explicit() {
        for (x, expected) in [
            (12.345678f32, "12.3457"),
            (0.0, "0"),
            (-3.0103, "-3.0103"),
            (0.00000125, "1.25e-06"),
            (9999999.0, "1e+07"),
        ] {
            assert_eq!(general(x), expected);
        }
    }

    #[test]
    fn one_sample_sox_preserves_legacy_text_but_suppresses_undefined_deltas() {
        let mut state = SoxState::default();
        state.push(536870912, false);
        let report = state.finish(8000, 1, "generated signed32".into());
        assert_eq!(report.fields.len(), 15);
        for key in ["meanDelta", "rmsDelta"] {
            assert_eq!(report.fields[key].value, None);
            assert_eq!(report.fields[key].legacy_text.as_deref(), Some("1.#INF00"));
        }
    }
}
