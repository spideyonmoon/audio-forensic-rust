//! P04c inputs on the pinned Python f32 basis. No ancestry scores or verdicts.
//! Native detector records are never accepted as reference inputs by these APIs.
use crate::{
    AnalysisReport, FileStatus,
    decode::Failure,
    detectors::{
        aac::{AacCollector, AacSelector},
        segments::{ClipAnalyzer, WALLS},
        structure::StructureStats,
        vorbis::VorbisCollector,
    },
    dsp::{HOP, StreamingStft, WINDOW},
    model::{
        AacAnalysis, AacBasis, AnalysisInterval, CodecWallCandidate, SegmentProbe,
        SpectralStructureAnalysis, VorbisAnalysis,
    },
};
use rustfft::num_complex::Complex32;
use serde::{Deserialize, Serialize};

pub const REFERENCE_INPUT_VERSION: u32 = 2;
pub const REFERENCE_INPUT_METHOD: &str = "python-c6ecce2-p04abc-f32-v2";
const BINS: usize = WINDOW / 2 + 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValue {
    pub value: Option<f64>,
    pub unit: String,
    pub unavailable_reason: Option<String>,
}
impl InputValue {
    pub(crate) fn new(value: Option<f64>, unit: &str, reason: &str) -> Self {
        let value = value.filter(|x| x.is_finite());
        Self {
            value,
            unit: unit.into(),
            unavailable_reason: value.is_none().then(|| reason.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisScope {
    pub domain: String,
    pub interval: AnalysisInterval,
    pub native_peak: f64,
    pub mid_peak: f32,
    pub side_peak: Option<f32>,
    /// Exactly zero mid with nonzero native PCM; no source-history implication.
    pub mid_cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseInputs {
    pub domain: String,
    pub interval: Option<AnalysisInterval>,
    pub stft_frames: u64,
    pub active_frames: u64,
    pub cutoff_p95_hz: InputValue,
    pub cutoff_variance_hz2: InputValue,
    pub sharpness_db_per_bin: InputValue,
    pub cliff_depth_db: InputValue,
    pub hf_magnitude_ratio: InputValue,
    pub entropy_bits: InputValue,
    /// Raw FFT magnitude RMS dB, distinct from filtered time-domain void.
    pub noise_above_cutoff_db: InputValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterInputs {
    pub domain: String,
    pub interval: Option<AnalysisInterval>,
    pub active_frames: u64,
    pub sampling_stride: u64,
    pub sampled_frames: u64,
    pub flat_sampled_frames: u64,
    pub bound_frames: u64,
    pub average_bound_hz: InputValue,
    pub histogram_mode_hz: InputValue,
    /// Unchanged source results, including flat-floor artifacts and gap bridging.
    /// Audit only: P05 must use the gated fields above / physical_phase below.
    pub legacy_average_bound_hz: Option<f64>,
    pub legacy_histogram_mode_hz: Option<f64>,
    pub legacy_phase_entropy_bits: Option<f64>,
    pub legacy_phase_frame_pairs: u64,
    pub legacy_phase_bin_pairs: u64,
    pub legacy_phase_histogram: Vec<u64>,
    /// Adjacent actual frames, energy-qualified bins >=10 kHz, excludes Nyquist.
    pub physical_phase: SpectralStructureAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformWinner {
    pub domain: String,
    pub interval: AnalysisInterval,
    pub basis: Option<String>,
    pub score: InputValue,
    pub supporting_probes: Option<usize>,
    pub tested_probes: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentPlan {
    pub requested_probes: usize,
    pub clip_frames: u64,
    pub offsets: Vec<u64>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentInputs {
    pub domain: String,
    pub plan: SegmentPlan,
    /// All planned intervals, including inactive clips and duplicate offsets.
    pub probes: Vec<SegmentProbe>,
    pub classic_wall_hz: f64,
    pub classic_vote: SegmentVote,
    /// Depends on P04a resampling and P04b time-domain void; never STFT noise.
    pub adaptive_wall_hz: InputValue,
    pub nearest_reference_wall: Option<CodecWallCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentVote {
    pub active_probes: usize,
    pub eligible_probes: usize,
    pub eligible_walled_probes: usize,
    pub majority_on_eligible_probes: Option<bool>,
    /// Source's silent-only gate/count/majority retained for trace, not a verdict.
    pub legacy_walled_probes: usize,
    pub legacy_majority: Option<bool>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceInputs {
    pub header: crate::reference_spectral::HeaderObservations,
    pub spectral: crate::reference_spectral::SpectralInputs,
    pub source: crate::reference_source::SourceInputs,
    pub version: u32,
    pub method: String,
    pub reference_commit: String,
    pub deviation_ids: Vec<String>,
    pub sample_rate: u32,
    pub analyzed_frames: u64,
    pub reached_end: bool,
    pub decoded_pcm_sha256: String,
    pub hash_sample_encoding: String,
    /// Native measurement still uses two passes; this adapter verifies a third.
    pub processing_passes: u8,
    pub pass_pcm_sha256: Vec<String>,
    pub basis: BasisScope,
    pub base: BaseInputs,
    pub scatter: ScatterInputs,
    pub segments: SegmentInputs,
    pub aac_winner: TransformWinner,
    pub aac_bases: Vec<AacAnalysis>,
    pub vorbis_winner: TransformWinner,
    pub vorbis_reconstructed_channels: Vec<VorbisAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceAnalysis {
    pub measurement: AnalysisReport,
    pub reference_inputs: Option<ReferenceInputs>,
}
impl ReferenceAnalysis {
    pub(crate) fn new(measurement: AnalysisReport, inputs: Option<ReferenceInputs>) -> Self {
        let reference_inputs = (measurement.status == FileStatus::Analyzed)
            .then_some(inputs)
            .flatten();
        Self {
            measurement,
            reference_inputs,
        }
    }
}

/// Native samples are rounded individually before the f32 M/S operations.
pub fn reference_basis(left: f64, right: Option<f64>) -> (f32, Option<f32>) {
    let left = left as f32;
    match right {
        Some(right) => {
            let right = right as f32;
            ((left + right) * 0.5, Some((left - right) * 0.5))
        }
        None => (left, None),
    }
}

fn interval(frames: u64) -> Option<AnalysisInterval> {
    (frames > 0).then_some(AnalysisInterval {
        start_frame: 0,
        end_frame: frames.saturating_sub(1) * HOP as u64 + WINDOW as u64,
    })
}

struct Frame {
    channels: usize,
    left: f64,
}
impl Frame {
    fn push(&mut self, ch: usize, sample: f64) -> Option<(f32, Option<f32>)> {
        if self.channels == 1 {
            Some(reference_basis(sample, None))
        } else if ch == 0 {
            self.left = sample;
            None
        } else {
            Some(reference_basis(self.left, Some(sample)))
        }
    }
}

pub(crate) struct ReferenceSurvey {
    rate: u32,
    frame: Frame,
    stft: StreamingStft,
    spectral_peak: f32,
    native_peak: f64,
    mid_peak: f32,
    side_peak: f32,
    aac: AacSelector,
}
impl ReferenceSurvey {
    pub fn new(rate: u32, channels: usize) -> Self {
        Self {
            rate,
            frame: Frame {
                channels,
                left: 0.0,
            },
            stft: StreamingStft::new(),
            spectral_peak: 0.0,
            native_peak: 0.0,
            mid_peak: 0.0,
            side_peak: 0.0,
            aac: AacSelector::new_reference(rate, channels),
        }
    }
    pub fn push(&mut self, ch: usize, x: f64) {
        self.native_peak = self.native_peak.max(x.abs());
        self.aac.push(ch, x);
        if let Some((mid, side)) = self.frame.push(ch, x) {
            self.mid_peak = self.mid_peak.max(mid.abs());
            self.side_peak = self.side_peak.max(side.unwrap_or(0.0).abs());
            self.stft.push(mid, |m| {
                self.spectral_peak = self
                    .spectral_peak
                    .max(m.iter().copied().fold(0.0, f32::max))
            });
        }
    }
    pub fn into_second(self, frames: u64) -> ReferenceSecond {
        let channels = self.frame.channels;
        ReferenceSecond {
            rate: self.rate,
            frames,
            frame: Frame {
                channels,
                left: 0.0,
            },
            stft: StreamingStft::new(),
            base: BaseStats::new(self.rate, self.spectral_peak),
            maxima: vec![0.0; BINS],
            quiet: crate::detectors::noise_floor::FloorSurvey::new(self.rate),
            physical: StructureStats::new(self.rate),
            phase: LegacyPhase::new(self.rate),
            aac: self.aac.into_collector(frames),
            vorbis: VorbisCollector::new(frames, self.rate, channels),
            basis: BasisScope {
                domain: if channels == 1 {
                    "f32_mono"
                } else {
                    "f32_mid_side"
                }
                .into(),
                interval: AnalysisInterval {
                    start_frame: 0,
                    end_frame: frames,
                },
                native_peak: self.native_peak,
                mid_peak: self.mid_peak,
                side_peak: (channels == 2).then_some(self.side_peak),
                mid_cancelled: self.mid_peak == 0.0 && self.native_peak > 0.0,
            },
            peak: self.spectral_peak,
        }
    }
}

pub(crate) struct ReferenceSecond {
    rate: u32,
    frames: u64,
    frame: Frame,
    stft: StreamingStft,
    base: BaseStats,
    maxima: Vec<f32>,
    quiet: crate::detectors::noise_floor::FloorSurvey,
    physical: StructureStats,
    phase: LegacyPhase,
    aac: AacCollector,
    vorbis: VorbisCollector,
    basis: BasisScope,
    peak: f32,
}
impl ReferenceSecond {
    pub fn push(&mut self, ch: usize, x: f64) {
        self.aac.push(ch, x);
        let native_mid = if self.frame.channels == 1 {
            x
        } else {
            (self.frame.left + x) * 0.5
        };
        if let Some((mid, side)) = self.frame.push(ch, x) {
            self.quiet.push(native_mid);
            if let Some(side) = side {
                // Pinned Vorbis widens BEFORE reconstructing, unlike P03 Pearson.
                self.vorbis.push(0, f64::from(mid) + f64::from(side));
                self.vorbis.push(1, f64::from(mid) - f64::from(side));
            } else {
                self.vorbis.push(0, f64::from(mid));
            }
            self.stft.push_spectrum(mid, |m, s| {
                let active = self.base.push(m);
                if active {
                    for (p, x) in self.maxima.iter_mut().zip(m) {
                        *p = p.max(*x);
                    }
                }
                self.physical.push(m, s, active);
                self.phase.push(s, active);
            });
        }
    }
    pub fn into_third(
        self,
        mut control: impl FnMut() -> Result<(), Failure>,
    ) -> Result<ReferenceThird, Failure> {
        control()?;
        let aac = self.aac.finish(&mut control)?;
        let vorbis = self.vorbis.finish(&mut control)?;
        let base = self.base.finish();
        let channels = self.frame.channels;
        let count = base.active_frames;
        let spectral = crate::reference_spectral::SpectralCollector::new(
            self.rate,
            base.cutoff_p95_hz.value,
            self.maxima,
        );
        Ok(ReferenceThird {
            rate: self.rate,
            frames: self.frames,
            frame: self.frame,
            stft: StreamingStft::new(),
            peak: self.peak,
            active: 0,
            scatter: ScatterStats::new(self.rate, count),
            spectral,
            source: crate::reference_source::SourceCollector::new(self.rate, self.frames),
            quiet: self.quiet.into_collector(0, self.frames),
            side_stft: StreamingStft::new(),
            side_mags: vec![0.0; BINS],
            stft_index: 0,
            position: 0,
            bit_counts: vec![[0; 33]; channels],
            integer_only: true,
            segments: ReferenceSegments::new(self.frames, self.rate),
            base,
            physical: self.physical.finish(0),
            phase: self.phase,
            basis: self.basis,
            aac,
            vorbis,
        })
    }
}

pub(crate) struct ReferenceThird {
    rate: u32,
    frames: u64,
    frame: Frame,
    stft: StreamingStft,
    peak: f32,
    active: u64,
    scatter: ScatterStats,
    spectral: crate::reference_spectral::SpectralCollector,
    source: crate::reference_source::SourceCollector,
    quiet: crate::detectors::noise_floor::FloorCollector,
    side_stft: StreamingStft,
    side_mags: Vec<f32>,
    stft_index: u64,
    position: u64,
    bit_counts: Vec<[u64; 33]>,
    integer_only: bool,
    segments: ReferenceSegments,
    base: BaseInputs,
    physical: SpectralStructureAnalysis,
    phase: LegacyPhase,
    basis: BasisScope,
    aac: Vec<AacAnalysis>,
    vorbis: Vec<VorbisAnalysis>,
}
impl ReferenceThird {
    pub fn push(&mut self, ch: usize, x: f64, integer: Option<i32>) {
        if self.position < self.rate as u64 * 30 {
            if let Some(word) = integer {
                if word != 0 {
                    self.bit_counts[ch][word.trailing_zeros() as usize] += 1;
                }
            } else {
                self.integer_only = false;
            }
        }
        let native_mid = if self.frame.channels == 1 {
            x
        } else {
            (self.frame.left + x) * 0.5
        };
        if let Some((mid, side)) = self.frame.push(ch, x) {
            self.position += 1;
            self.quiet.push(native_mid);
            self.source.push(mid);
            if let Some(side) = side {
                self.side_stft
                    .push(side, |m| self.side_mags.copy_from_slice(m));
            }
            self.segments.push(mid);
            self.stft.push(mid, |m| {
                if side.is_some() && self.frames >= 8192 && self.stft_index % 4 == 0 {
                    self.spectral.side(m, &self.side_mags);
                }
                self.stft_index += 1;
                let peak = m.iter().copied().fold(0.0, f32::max);
                if peak > (self.peak + 1e-12) * 1e-3 {
                    self.spectral.push(m);
                    if self.active % self.scatter.stride == 0 {
                        self.scatter.push(m);
                    }
                    self.active += 1;
                }
            });
        }
    }
    pub fn finish(
        self,
        control: impl FnMut() -> Result<(), Failure>,
    ) -> Result<ReferenceInputs, Failure> {
        let cutoff = self.base.cutoff_p95_hz.value;
        let cliff = self.base.cliff_depth_db.value;
        let nearest = reference_fingerprint(cutoff, self.rate);
        let plan = self.segments.plan;
        let probes = self.segments.probes;
        let classic_vote = segment_vote(&probes, 16500.0);
        let aac_winner = aac_winner(&self.aac, self.frames);
        let vorbis_winner = vorbis_winner(&self.vorbis, self.frame.channels, self.frames);
        let spectral = self.spectral.finish();
        let bits = self.bit_counts.iter().map(|counts| {
            let nonzero: u64 = counts.iter().sum(); let mut sum = 0;
            let v = if self.integer_only && nonzero >= 500 { counts.iter().position(|n| {sum += n; sum >= 8.max(nonzero/10000)}).map(|i|(32-i) as f64) } else {None};
            InputValue::new(v,"bits","Requires 500 nonzero exact integer samples in first 30s; float conversion is not inferred")
        }).collect();
        let source = self
            .source
            .finish(cutoff, cliff, self.quiet.finish(), bits, control)?;
        let resample_wall = if spectral.resampling.candidates.is_empty()
            || spectral.resampling.candidates.iter().any(|c| c.eligible)
        {
            Some(
                spectral
                    .ordered_resampling_hit
                    .as_ref()
                    .is_some_and(|h| h.mode == "wall"),
            )
        } else {
            None
        };
        let void_verified = source.void_profile.rms_dbfs.value;
        let adaptive =
            adaptive_segment_wall(cutoff, cliff, void_verified, resample_wall, self.rate);
        Ok(ReferenceInputs {
            header: crate::reference_spectral::header_observations(
                self.frames as f64 / self.rate as f64,
                false,
                None,
                None,
                None,
                "",
            ),
            spectral,
            source,
            version: REFERENCE_INPUT_VERSION,
            method: REFERENCE_INPUT_METHOD.into(),
            reference_commit: "c6ecce2296256b516709d87088896d1be913908c".into(),
            deviation_ids: vec!["D01".into(), "D02".into(), "D03".into()],
            sample_rate: self.rate,
            analyzed_frames: self.frames,
            reached_end: false,
            decoded_pcm_sha256: String::new(),
            hash_sample_encoding: String::new(),
            processing_passes: 3,
            pass_pcm_sha256: vec![],
            basis: self.basis,
            base: self.base,
            scatter: self.scatter.finish(self.active, self.physical, self.phase),
            segments: SegmentInputs {
                domain: "f32_mid_to_f64_hann_2s_fft".into(),
                plan,
                probes,
                classic_wall_hz: 16500.0,
                classic_vote,
                adaptive_wall_hz: adaptive,
                nearest_reference_wall: nearest,
            },
            aac_winner,
            aac_bases: self.aac,
            vorbis_winner,
            vorbis_reconstructed_channels: self.vorbis,
        })
    }
}

struct BaseStats {
    rate: u32,
    peak: f32,
    frames: u64,
    active: u64,
    cutoffs: Vec<u64>,
    sum: Vec<f32>,
    squares: Vec<f64>,
    total: f64,
    hf: f64,
}
impl BaseStats {
    fn new(rate: u32, peak: f32) -> Self {
        Self {
            rate,
            peak,
            frames: 0,
            active: 0,
            cutoffs: vec![0; BINS],
            sum: vec![0.0; BINS],
            squares: vec![0.0; BINS],
            total: 0.0,
            hf: 0.0,
        }
    }
    fn push(&mut self, m: &[f32]) -> bool {
        self.frames += 1;
        let peak = m.iter().copied().fold(0.0, f32::max);
        if peak <= (self.peak + 1e-12) * 1e-3 {
            return false;
        }
        self.active += 1;
        let cutoff = m
            .iter()
            .rposition(|x| 20.0 * (x / (peak + 1e-12) + 1e-12).log10() > -65.0)
            .unwrap_or(0);
        self.cutoffs[cutoff] += 1;
        let start = (15000.0 * WINDOW as f64 / self.rate as f64) as usize;
        for (i, &x) in m.iter().enumerate() {
            self.sum[i] += x;
            self.squares[i] += f64::from(x * x);
            self.total += f64::from(x);
            if i >= start {
                self.hf += f64::from(x);
            }
        }
        true
    }
    fn finish(self) -> BaseInputs {
        let ready = self.frames >= 4 && self.active >= 4;
        let rank = (self.active.saturating_sub(1)) as f64 * 0.95;
        let at = |rank: u64| {
            let mut count = 0;
            self.cutoffs
                .iter()
                .position(|n| {
                    count += n;
                    count > rank
                })
                .unwrap_or(0)
        };
        let bin = self.rate as f64 / WINDOW as f64;
        let cutoff = (at(rank.floor() as u64) as f64 * (1.0 - rank.fract())
            + at(rank.ceil() as u64) as f64 * rank.fract())
            * bin;
        let mean = self
            .cutoffs
            .iter()
            .enumerate()
            .map(|(i, n)| i as f64 * bin * (*n as f64))
            .sum::<f64>()
            / self.active.max(1) as f64;
        let variance = self
            .cutoffs
            .iter()
            .enumerate()
            .map(|(i, n)| (i as f64 * bin - mean).powi(2) * (*n as f64))
            .sum::<f64>()
            / self.active.max(1) as f64;
        let avg: Vec<f32> = self
            .sum
            .iter()
            .map(|x| x / self.active.max(1) as f32)
            .collect();
        let ref_peak = avg.iter().copied().fold(0.0, f32::max) + 1e-12;
        let db: Vec<f32> = avg
            .iter()
            .map(|x| 20.0 * (x / ref_peak + 1e-12).log10())
            .collect();
        let lo = ((cutoff - 2500.0).max(0.0) / bin) as usize;
        let hi = (((cutoff + 625.0) / bin) as usize).min(BINS);
        let sharp = (hi > lo + 1).then(|| {
            db[lo..hi]
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0, f32::max) as f64
        });
        let left = ((cutoff - 400.0).max(0.0) / bin) as usize;
        let right = (((cutoff + 400.0) / bin) as usize).min(BINS - 1);
        let cliff = (right > left).then(|| f64::from(db[left] - db[right]));
        let total = avg.iter().map(|x| f64::from(*x)).sum::<f64>() as f32 + 1e-12;
        let entropy = avg
            .iter()
            .filter(|x| **x > 0.0)
            .map(|x| {
                let p = x / total;
                f64::from(-p * p.log2())
            })
            .sum::<f64>();
        let first = ((cutoff / bin) as usize).min(BINS - 1);
        let noise = 20.0
            * ((self.squares[first..].iter().sum::<f64>()
                / (self.active.max(1) as f64 * (BINS - first) as f64))
                .sqrt()
                + 1e-12)
                .log10();
        let val = |x: Option<f64>, unit: &str| {
            InputValue::new(
                if ready { x } else { None },
                unit,
                "Requires four active reference STFT frames and nonempty numerical support",
            )
        };
        BaseInputs {
            domain: "f32_mono_or_mid_hann4096_hop2048_strict".into(),
            interval: interval(self.frames),
            stft_frames: self.frames,
            active_frames: self.active,
            cutoff_p95_hz: val(Some(cutoff), "Hz"),
            cutoff_variance_hz2: val(Some(variance), "Hz^2"),
            sharpness_db_per_bin: val(sharp, "dB/bin"),
            cliff_depth_db: val(cliff, "dB"),
            hf_magnitude_ratio: val(
                (self.rate > 30000).then_some(self.hf / (self.total + 1e-12)),
                "ratio",
            ),
            entropy_bits: val(Some(entropy), "bits"),
            noise_above_cutoff_db: val(Some(noise), "raw FFT magnitude dB"),
        }
    }
}

struct LegacyPhase {
    start: usize,
    previous: Option<Vec<f32>>,
    hist: [u64; 36],
    pairs: u64,
}
impl LegacyPhase {
    fn new(rate: u32) -> Self {
        Self {
            start: (10000 * WINDOW / rate as usize).min(BINS - 1),
            previous: None,
            hist: [0; 36],
            pairs: 0,
        }
    }
    fn push(&mut self, s: &[Complex32], active: bool) {
        if !active {
            return;
        }
        let next: Vec<f32> = s[self.start..].iter().map(|x| x.im.atan2(x.re)).collect();
        if let Some(previous) = &self.previous {
            for (&a, &b) in previous.iter().zip(&next) {
                let delta = f64::from(b) - f64::from(a);
                let wrapped = delta.sin().atan2(delta.cos());
                let bin = (((wrapped + std::f64::consts::PI) / std::f64::consts::TAU * 36.0)
                    as usize)
                    .min(35);
                self.hist[bin] += 1;
            }
            self.pairs += 1;
        }
        self.previous = Some(next);
    }
    fn entropy(&self) -> Option<f64> {
        (self.pairs >= 2 && BINS - self.start >= 2).then(|| hist_entropy(&self.hist))
    }
}
fn hist_entropy(hist: &[u64]) -> f64 {
    let total = hist.iter().sum::<u64>() as f64 + 1e-12;
    hist.iter()
        .filter(|n| **n > 0)
        .map(|n| {
            let p = *n as f64 / total;
            -p * p.log2()
        })
        .sum()
}

struct ScatterStats {
    rate: u32,
    stride: u64,
    sampled: u64,
    flat: u64,
    raw: Vec<u64>,
    gated: Vec<u64>,
    log: Vec<f32>,
    scatter: Vec<f32>,
}
impl ScatterStats {
    fn new(rate: u32, active: u64) -> Self {
        Self {
            rate,
            stride: if active > 2500 { active / 2500 + 1 } else { 1 },
            sampled: 0,
            flat: 0,
            raw: vec![0; BINS],
            gated: vec![0; BINS],
            log: vec![0.0; BINS],
            scatter: vec![0.0; BINS],
        }
    }
    fn push(&mut self, m: &[f32]) {
        self.sampled += 1;
        let peak = m.iter().copied().fold(0.0, f32::max) + 1e-12;
        for (out, &x) in self.log.iter_mut().zip(m) {
            let db = (20.0 * (x / peak + 1e-12).log10()).max(-110.0) / 10.0;
            *out = (f64::from(db) * std::f64::consts::LN_10) as f32;
        }
        // D03 flat gate uses centered f64 local differences, avoiding f32 m2-m1²
        // floor residue. The source moments remain separately auditable.
        let mut organic_max = 0.0f64;
        let mut physical = vec![0.0; BINS];
        for (i, value) in physical.iter_mut().enumerate() {
            let local: [f64; 5] = std::array::from_fn(|j| {
                f64::from(self.log[(i + j).saturating_sub(2).min(BINS - 1)])
            });
            let mean = local.iter().sum::<f64>() / 5.0;
            *value = (local.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 5.0).sqrt();
            let m1 = mean as f32;
            let m2 = (local
                .iter()
                .map(|x| f64::from((*x as f32) * (*x as f32)))
                .sum::<f64>()
                / 5.0) as f32;
            self.scatter[i] = (m2 - m1 * m1).max(0.0).sqrt();
        }
        for (i, value) in self.log.iter_mut().enumerate() {
            *value = ((0..5)
                .map(|j| f64::from(self.scatter[(i + j).saturating_sub(2).min(BINS - 1)]))
                .sum::<f64>()
                / 5.0) as f32;
            organic_max = organic_max.max(
                (0..5)
                    .map(|j| physical[(i + j).saturating_sub(2).min(BINS - 1)])
                    .sum::<f64>()
                    / 5.0,
            );
        }
        let maximum = self.log.iter().copied().fold(0.0, f32::max);
        let threshold = 0.6f32.min(maximum * 0.25);
        let bound = self.log.iter().rposition(|x| *x >= threshold).unwrap_or(0);
        self.raw[bound] += 1;
        if organic_max <= 1e-6 {
            self.flat += 1;
        } else {
            self.gated[bound] += 1;
        }
    }
    fn bounds(&self, counts: &[u64]) -> (Option<f64>, Option<f64>) {
        let total = counts.iter().sum::<u64>();
        if total == 0 {
            return (None, None);
        }
        let bin = self.rate as f64 / WINDOW as f64;
        let avg = counts
            .iter()
            .enumerate()
            .map(|(i, n)| i as f64 * (*n as f64) * bin)
            .sum::<f64>()
            / total as f64;
        let first = counts.iter().position(|n| *n > 0).unwrap() as f64 * bin;
        let last = counts.iter().rposition(|n| *n > 0).unwrap() as f64 * bin;
        let (lo, hi) = if first == last {
            (first - 0.5, last + 0.5)
        } else {
            (first, last)
        };
        let mut hist = [0u64; 20];
        for (i, &count) in counts.iter().enumerate() {
            if count == 0 {
                continue;
            }
            let index = (((i as f64 * bin - lo) / (hi - lo) * 20.0) as usize).min(19);
            hist[index] += count;
        }
        let winner = hist
            .iter()
            .position(|n| *n == *hist.iter().max().unwrap())
            .unwrap();
        (
            Some(avg),
            Some(lo + (winner as f64 + 0.5) * (hi - lo) / 20.0),
        )
    }
    fn finish(
        self,
        active: u64,
        physical: SpectralStructureAnalysis,
        phase: LegacyPhase,
    ) -> ScatterInputs {
        let bound_frames = self.gated.iter().sum::<u64>();
        let (avg, mode) = self.bounds(&self.gated);
        let (raw_avg, raw_mode) = self.bounds(&self.raw);
        let ready = bound_frames >= 4;
        ScatterInputs {
            domain: "f32_mid_active_rows_source_stride_and_20bin_mode".into(),
            interval: physical.interval.clone(),
            active_frames: active,
            sampling_stride: self.stride,
            sampled_frames: self.sampled,
            flat_sampled_frames: self.flat,
            bound_frames,
            average_bound_hz: InputValue::new(
                if ready { avg } else { None },
                "Hz",
                "Requires four nonflat sampled frames",
            ),
            histogram_mode_hz: InputValue::new(
                if ready { mode } else { None },
                "Hz",
                "Requires four nonflat sampled frames",
            ),
            legacy_average_bound_hz: if active >= 4 { raw_avg } else { None },
            legacy_histogram_mode_hz: if active >= 4 { raw_mode } else { None },
            legacy_phase_entropy_bits: if active >= 4 { phase.entropy() } else { None },
            legacy_phase_frame_pairs: phase.pairs,
            legacy_phase_bin_pairs: phase.hist.iter().sum(),
            legacy_phase_histogram: phase.hist.to_vec(),
            physical_phase: physical,
        }
    }
}

/// CPython Random(42).randint(0, max), including its rejection/getrandbits policy.
fn random42(max: u64) -> u64 {
    let mut mt = [0u32; 624];
    mt[0] = 19650218;
    for i in 1..624 {
        mt[i] = 1812433253u32
            .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
            .wrapping_add(i as u32);
    }
    let mut i = 1;
    for _ in 0..624 {
        mt[i] = (mt[i] ^ (mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1664525)).wrapping_add(42);
        i += 1;
        if i == 624 {
            mt[0] = mt[623];
            i = 1;
        }
    }
    for _ in 0..623 {
        mt[i] = (mt[i] ^ (mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1566083941))
            .wrapping_sub(i as u32);
        i += 1;
        if i == 624 {
            mt[0] = mt[623];
            i = 1;
        }
    }
    mt[0] = 0x80000000;
    let mut index = 624;
    let mut word = || {
        if index == 624 {
            for i in 0..624 {
                let y = (mt[i] & 0x80000000) | (mt[(i + 1) % 624] & 0x7fffffff);
                mt[i] = mt[(i + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908b0df } else { 0 };
            }
            index = 0;
        }
        let mut y = mt[index];
        index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    };
    let n = max + 1;
    let k = 64 - n.leading_zeros();
    loop {
        let r = if k <= 32 {
            u64::from(word() >> (32 - k))
        } else {
            u64::from(word()) | (u64::from(word() >> (64 - k)) << 32)
        };
        if r < n {
            return r;
        }
    }
}

/// Duration adapts probe count; short inputs never reduce the source minimum.
pub fn reference_segment_plan(frames: u64, rate: u32) -> SegmentPlan {
    let count = if rate == 0 {
        9
    } else {
        (frames / (u64::from(rate) * 15)).clamp(9, 36) as usize
    };
    let clip = 2 * u64::from(rate);
    let reason = if rate <= 33000 {
        Some("Nyquist does not exceed classic 16.5 kHz wall")
    } else if frames < clip * count as u64 {
        Some("Requires two seconds per requested probe")
    } else {
        None
    };
    let mut offsets = vec![];
    if reason.is_none() {
        let step = (frames - clip) / (count as u64 - 1);
        offsets.extend((0..count - 2).map(|i| i as u64 * step));
        offsets.push(random42(frames - clip));
        offsets.push(frames - clip);
        offsets.sort_unstable();
    }
    SegmentPlan {
        requested_probes: count,
        clip_frames: clip,
        offsets,
        unavailable_reason: reason.map(str::to_owned),
    }
}

struct ReferenceSegments {
    plan: SegmentPlan,
    position: u64,
    next: usize,
    ring: Vec<f32>,
    clip: Vec<f64>,
    analyzer: Option<ClipAnalyzer>,
    probes: Vec<SegmentProbe>,
}
impl ReferenceSegments {
    fn new(frames: u64, rate: u32) -> Self {
        let plan = reference_segment_plan(frames, rate);
        let enabled = !plan.offsets.is_empty();
        let len = if enabled {
            plan.clip_frames as usize
        } else {
            0
        };
        Self {
            plan,
            position: 0,
            next: 0,
            ring: vec![0.0; len],
            clip: vec![0.0; len],
            analyzer: enabled.then(|| ClipAnalyzer::new(rate)),
            probes: vec![],
        }
    }
    fn push(&mut self, x: f32) {
        if self.ring.is_empty() {
            return;
        }
        let len = self.ring.len();
        self.ring[(self.position % len as u64) as usize] = x;
        self.position += 1;
        while self
            .plan
            .offsets
            .get(self.next)
            .is_some_and(|off| off + self.plan.clip_frames == self.position)
        {
            let start = self.plan.offsets[self.next];
            for (i, value) in self.clip.iter_mut().enumerate() {
                *value = f64::from(self.ring[((self.position % len as u64) as usize + i) % len]);
            }
            self.probes
                .push(self.analyzer.as_mut().unwrap().measure(&self.clip, start));
            self.next += 1;
        }
    }
}

fn majority(walled: usize, total: usize) -> bool {
    if total % 2 == 0 {
        walled > 0 && walled >= total / 2
    } else {
        walled > total / 2
    }
}
/// Pure vote over reference clips. Fewer than three broad-band eligible probes
/// cannot produce a product majority. Legacy source vote is trace only.
pub fn segment_vote(probes: &[SegmentProbe], wall: f64) -> SegmentVote {
    let supported = wall.is_finite() && wall > 0.0;
    let active = probes.iter().filter(|p| p.active).count();
    let eligible = probes.iter().filter(|p| p.eligible).count();
    let walled = probes
        .iter()
        .filter(|p| p.eligible && p.cutoff_hz.is_some_and(|c| c <= wall))
        .count();
    let raw = probes
        .iter()
        .filter(|p| p.active && p.cutoff_hz.is_some_and(|c| c <= wall))
        .count();
    SegmentVote {
        active_probes: active,
        eligible_probes: eligible,
        eligible_walled_probes: walled,
        majority_on_eligible_probes: (supported && eligible >= 3)
            .then(|| majority(walled, eligible)),
        legacy_walled_probes: raw,
        legacy_majority: (supported && active > 0).then(|| majority(raw, active)),
        unavailable_reason: (!(supported && eligible >= 3))
            .then(|| "Requires finite positive wall and three broad-band eligible probes".into()),
    }
}

pub fn reference_fingerprint(cutoff: Option<f64>, rate: u32) -> Option<CodecWallCandidate> {
    let cutoff = cutoff.filter(|c| c.is_finite() && *c > 0.0 && *c < (rate as f64 / 2.0) * 0.98)?;
    let mut best: Option<CodecWallCandidate> = None;
    for &(codec, profile, hz, tol) in WALLS {
        let d = (cutoff - hz).abs();
        if d <= tol && best.as_ref().is_none_or(|b| d < b.distance_hz) {
            best = Some(CodecWallCandidate {
                codec: codec.into(),
                profile: profile.into(),
                reference_wall_hz: hz,
                tolerance_hz: tol,
                distance_hz: d,
            });
        }
    }
    best
}

/// Pinned adaptive threshold with explicitly supplied P04a/P04b inputs. None
/// means unknown; missing dependencies are never substituted with clear zeros.
pub fn adaptive_segment_wall(
    cutoff: Option<f64>,
    cliff: Option<f64>,
    verified_time_void: Option<f64>,
    resampling_wall: Option<bool>,
    rate: u32,
) -> InputValue {
    let cutoff = cutoff.filter(|x| x.is_finite());
    let cliff = cliff.filter(|x| x.is_finite());
    let wall = match (cutoff, cliff) {
        (Some(c), Some(d)) if c >= 22500.0 || d <= 30.0 => Some(16500.0),
        (Some(c), Some(_)) => {
            let fingerprint = reference_fingerprint(Some(c), rate).is_some();
            let void = verified_time_void.filter(|x| x.is_finite());
            match (resampling_wall, void) {
                (Some(true), _) => Some(16500.0),
                (Some(false), _) if fingerprint => Some(16500.0f64.max(c + 400.0)),
                (Some(false), Some(v)) => Some(if v < -85.0 {
                    16500.0f64.max(c + 400.0)
                } else {
                    16500.0
                }),
                (_, Some(v)) if v >= -85.0 && !fingerprint => Some(16500.0),
                _ => None,
            }
        }
        _ => None,
    };
    InputValue::new(
        wall,
        "Hz",
        "Requires base cutoff/cliff and applicable P04a resampling / P04b time-domain void inputs",
    )
}

fn aac_winner(bases: &[AacAnalysis], frames: u64) -> TransformWinner {
    let mut winner: Option<&AacAnalysis> = None;
    for b in bases {
        if b.lattice_score
            .is_some_and(|s| s.is_finite() && winner.is_none_or(|p| s > p.lattice_score.unwrap()))
        {
            winner = Some(b);
        }
    }
    TransformWinner {
        domain: "f32_mono_mid_side_widened_f64_kbd_mdct".into(),
        interval: AnalysisInterval {
            start_frame: 0,
            end_frame: bases.first().map_or(frames, |b| b.search_limit_frames),
        },
        basis: winner.map(|b| {
            match b.basis {
                AacBasis::Mono => "M",
                AacBasis::Mid => "M",
                AacBasis::Side => "S",
            }
            .into()
        }),
        score: InputValue::new(
            winner.and_then(|b| b.lattice_score),
            "flagged fraction of 49 bands",
            "Unsupported rate or fewer than four eligible probes/bands",
        ),
        supporting_probes: winner.map(|b| b.eligible_probes),
        tested_probes: winner.map(|b| b.probes.len()),
    }
}
fn vorbis_winner(bases: &[VorbisAnalysis], channels: usize, frames: u64) -> TransformWinner {
    let mut winner: Option<&VorbisAnalysis> = None;
    for b in bases {
        if b.zero_excess_score.is_some_and(|s| {
            s.is_finite() && winner.is_none_or(|p| s > p.zero_excess_score.unwrap())
        }) {
            winner = Some(b);
        }
    }
    TransformWinner {
        domain: "f32_mid_side_widened_then_reconstructed_f64_L_R_vorbis_mdct".into(),
        interval: AnalysisInterval {
            start_frame: 0,
            end_frame: bases.first().map_or(frames, |b| b.search_limit_frames),
        },
        basis: winner.map(|b| {
            if channels == 1 {
                "M"
            } else if b.channel_index == 0 {
                "L"
            } else {
                "R"
            }
            .into()
        }),
        score: InputValue::new(
            winner.and_then(|b| b.zero_excess_score),
            "zero excess fraction",
            "Unsupported rate or fewer than four active probes",
        ),
        supporting_probes: winner.map(|b| b.supporting_probes),
        tested_probes: winner.map(|b| b.active_probes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn oracle() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/reference_inputs.json")).unwrap()
    }
    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} != {b} (tolerance {tol})");
    }

    #[test]
    fn basis_rounding_and_python_random42_offsets() {
        let o = oracle();
        for v in o["basis_vectors"].as_array().unwrap() {
            let (m, s) = reference_basis(v["left"].as_f64().unwrap(), v["right"].as_f64());
            assert_eq!(f64::from(m), v["mid"].as_f64().unwrap());
            assert_eq!(f64::from(s.unwrap()), v["side"].as_f64().unwrap());
            assert_eq!(
                f64::from(m) + f64::from(s.unwrap()),
                v["reconstructed_left"].as_f64().unwrap()
            );
            assert_eq!(
                f64::from(m) - f64::from(s.unwrap()),
                v["reconstructed_right"].as_f64().unwrap()
            );
        }
        for v in o["offsets"].as_array().unwrap() {
            let p = reference_segment_plan(
                v["frames"].as_u64().unwrap(),
                v["rate"].as_u64().unwrap() as u32,
            );
            assert_eq!(p.requested_probes, v["count"].as_u64().unwrap() as usize);
            assert_eq!(
                p.offsets,
                v["offsets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn source_scatter_uses_final_active_count_and_twenty_bin_mode() {
        let mut stats = ScatterStats::new(48000, 5003);
        for i in (0..5003).step_by(3) {
            let mut mags: Vec<f32> = (0..BINS)
                .map(|n| 0.01 + (n as f32 * 0.031 + i as f32 * 0.073).sin().abs())
                .collect();
            for x in &mut mags[600 + i % 1100..] {
                *x *= 1e-6;
            }
            stats.push(&mags);
        }
        let e = &oracle()["long_scatter"];
        assert_eq!(stats.stride, e["stride"].as_u64().unwrap());
        assert_eq!(stats.sampled, e["sampled_frames"].as_u64().unwrap());
        let (avg, mode) = stats.bounds(&stats.raw);
        near(
            avg.unwrap(),
            e["average_bound_hz"].as_f64().unwrap(),
            48000.0 / 4096.0,
        );
        near(
            mode.unwrap(),
            e["histogram_mode_hz"].as_f64().unwrap(),
            48000.0 / 4096.0,
        );
        let mut flat = ScatterStats::new(48000, 4);
        for _ in 0..4 {
            flat.push(&vec![1.0; BINS]);
        }
        assert_eq!(flat.flat, 4);
        assert_eq!(flat.gated.iter().sum::<u64>(), 0);
        // Source returns a Nyquist-looking bound for constant log-power.
        assert_eq!(flat.raw[BINS - 1], 4);
    }

    #[test]
    fn physical_phase_retains_activity_gaps_energy_gates_and_nyquist_exclusion() {
        let mut phase = LegacyPhase::new(48000);
        let mut physical = StructureStats::new(48000);
        let mut mags = vec![0.0; BINS];
        mags[1200] = 1.0;
        mags[1201] = 0.7;
        mags[BINS - 1] = 1.0;
        for i in 0..7 {
            let active = i % 2 == 0;
            let s: Vec<_> = mags
                .iter()
                .map(|m| Complex32::from_polar(*m, i as f32 * 0.21))
                .collect();
            physical.push(&mags, &s, active);
            phase.push(&s, active);
        }
        assert_eq!(phase.pairs, 3);
        assert_eq!(
            phase.hist.iter().sum::<u64>(),
            3 * (BINS - phase.start) as u64
        );
        let p = physical.finish(0);
        assert_eq!(p.phase_frame_pairs, 0);
        assert!(p.high_band_phase_entropy_bits.is_none());
        assert!(phase.entropy().is_some());
    }

    #[test]
    fn segment_ring_preserves_overlaps_duplicates_and_fixed_storage() {
        let rate = 48000;
        let frames = 18 * rate as u64;
        let mut collector = ReferenceSegments::new(frames, rate);
        // Force overlapping/duplicate source intervals without changing production
        // planning: this directly validates the bounded ring's index arithmetic.
        collector.plan.offsets = vec![0, 0, 1234, frames - 2 * rate as u64];
        let mut independent = ClipAnalyzer::new(rate);
        let sample =
            |i: u64| ((i as f64 * 0.3723).sin() * 0.2 + (i as f64 * 1.8321).sin() * 0.1) as f32;
        for i in 0..frames {
            collector.push(sample(i));
        }
        assert_eq!(collector.probes.len(), 4);
        for (&off, actual) in collector.plan.offsets.iter().zip(&collector.probes) {
            let clip: Vec<_> = (off..off + 2 * rate as u64)
                .map(|i| f64::from(sample(i)))
                .collect();
            let expected = independent.measure(&clip, off);
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
        assert_eq!(collector.ring.len(), 2 * rate as usize);
        assert_eq!(collector.clip.len(), 2 * rate as usize);
        let capped = ReferenceSegments::new(384000 * 600, 384000);
        assert_eq!(capped.ring.len(), 768000);
        assert_eq!(capped.clip.len(), 768000);
        assert_eq!(capped.plan.requested_probes, 36);
        let bytes = capped.ring.capacity() * 4
            + capped.clip.capacity() * 8
            + capped.analyzer.as_ref().unwrap().payload_bytes();
        println!("384kHz segment payload bytes: {bytes}");
        assert!(bytes <= 46_080_016);
    }

    #[test]
    fn adaptive_dependencies_and_vote_boundaries_remain_explicit() {
        assert_eq!(
            adaptive_segment_wall(Some(15860.0), Some(31.0), None, Some(false), 48000).value,
            Some(16500.0)
        );
        assert_eq!(
            adaptive_segment_wall(Some(19350.0), Some(31.0), None, Some(false), 48000).value,
            Some(19750.0)
        );
        assert!(
            adaptive_segment_wall(Some(18000.0), Some(31.0), None, Some(false), 48000)
                .value
                .is_none()
        );
        assert!(
            adaptive_segment_wall(Some(18000.0), Some(31.0), Some(-86.0), None, 48000)
                .value
                .is_none()
        );
        assert_eq!(
            adaptive_segment_wall(Some(18000.0), Some(31.0), Some(-85.0), Some(false), 48000).value,
            Some(16500.0)
        );
        assert_eq!(
            adaptive_segment_wall(
                Some(18000.0),
                Some(31.0),
                Some(-85.00001),
                Some(false),
                48000
            )
            .value,
            Some(18400.0)
        );
        assert_eq!(
            adaptive_segment_wall(Some(18000.0), Some(30.0), None, None, 48000).value,
            Some(16500.0)
        );
        for (w, n, expected) in [(0, 4, false), (2, 4, true), (1, 3, false), (2, 3, true)] {
            assert_eq!(majority(w, n), expected);
        }
        assert!(
            segment_vote(&[], 16500.0)
                .majority_on_eligible_probes
                .is_none()
        );
        assert_eq!(
            reference_fingerprint(Some(19550.0), 44100).unwrap().codec,
            "AAC"
        );
    }
}
