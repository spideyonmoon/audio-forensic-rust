//! Streaming primitives. Two passes reproduce Python's global relative activity gate
//! without retaining audio or a full spectrogram. FFT windows are symmetric Hann,
//! unnormalized, and f32. The strict right boundary matches the reference STFT.
use crate::model::{ChannelMeasurements, SpectralMeasurements};
use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::sync::Arc;

pub const WINDOW: usize = 4096;
pub const HOP: usize = 2048;
pub const CUTOFF_DB: f32 = -65.0;

pub struct StreamingStft {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    pending: Vec<f32>,
    buffer: Vec<Complex32>,
    scratch: Vec<Complex32>,
    magnitudes: Vec<f32>,
}

impl Default for StreamingStft {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingStft {
    pub fn new() -> Self {
        let fft = FftPlanner::new().plan_fft_forward(WINDOW);
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        let window = (0..WINDOW)
            .map(|i| {
                (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (WINDOW - 1) as f64).cos()) as f32
            })
            .collect();
        Self {
            fft,
            window,
            pending: Vec::with_capacity(WINDOW),
            buffer: vec![Complex32::default(); WINDOW],
            scratch,
            magnitudes: vec![0.0; WINDOW / 2 + 1],
        }
    }

    /// One-sample lookahead intentionally excludes a frame ending exactly at EOF.
    pub fn push(&mut self, sample: f32, mut visit: impl FnMut(&[f32])) {
        self.push_spectrum(sample, |mags, _| visit(mags));
    }

    /// Borrow the existing positive-frequency complex bins without another FFT.
    pub(crate) fn push_spectrum(
        &mut self,
        sample: f32,
        mut visit: impl FnMut(&[f32], &[Complex32]),
    ) {
        if self.pending.len() == WINDOW {
            for (i, x) in self.pending.iter().enumerate() {
                self.buffer[i] = Complex32::new(x * self.window[i], 0.0);
            }
            self.fft
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            for (out, x) in self.magnitudes.iter_mut().zip(&self.buffer) {
                *out = x.norm();
            }
            visit(&self.magnitudes, &self.buffer[..self.magnitudes.len()]);
            self.pending.copy_within(HOP.., 0);
            self.pending.truncate(WINDOW - HOP);
        }
        self.pending.push(sample);
    }
}

#[derive(Default)]
pub(crate) struct PcmStats {
    count: u64,
    peak: f64,
    sum: f64,
    sum_square: f64,
    zeros: u64,
    full_scale: u64,
    nonzero: u64,
    trailing: [u64; 32],
}

impl PcmStats {
    pub fn push(&mut self, x: f64, integer: Option<i32>, bits: Option<u32>) {
        self.count += 1;
        self.peak = self.peak.max(x.abs());
        self.sum += x;
        self.sum_square += x * x;
        self.zeros += u64::from(x == 0.0);
        let upper = bits.map(|n| 1.0 - 2f64.powi(1 - n as i32)).unwrap_or(1.0);
        self.full_scale += u64::from(x <= -1.0 || x >= upper);
        if let Some(word) = integer.filter(|v| *v != 0) {
            self.nonzero += 1;
            self.trailing[word.trailing_zeros() as usize] += 1;
        }
    }

    pub fn finish(
        self,
        channel_index: usize,
        spectral: SpectralMeasurements,
    ) -> ChannelMeasurements {
        let exact = self
            .trailing
            .iter()
            .position(|n| *n > 0)
            .map(|n| 32 - n as u32);
        let threshold = 8.max(self.nonzero / 10_000);
        let mut cumulative = 0;
        let effective = if self.nonzero < 500 {
            None
        } else {
            self.trailing
                .iter()
                .position(|n| {
                    cumulative += n;
                    cumulative >= threshold
                })
                .map(|n| 32 - n as u32)
        };
        ChannelMeasurements {
            channel_index,
            samples: self.count,
            peak: self.peak,
            rms: (self.sum_square / self.count.max(1) as f64).sqrt(),
            dc_offset: self.sum / self.count.max(1) as f64,
            zero_samples: self.zeros,
            full_scale_samples: self.full_scale,
            exact_used_bits: exact,
            effective_bits: effective,
            spectral,
        }
    }
}

pub(crate) struct SpectralStats {
    rate: u32,
    global_peak: f32,
    frames: u64,
    active: u64,
    sum: Vec<f64>,
    square_sum: Vec<f64>,
    cutoffs: Vec<u64>,
}

impl SpectralStats {
    pub fn new(rate: u32, global_peak: f32) -> Self {
        Self {
            rate,
            global_peak,
            frames: 0,
            active: 0,
            sum: vec![0.0; WINDOW / 2 + 1],
            square_sum: vec![0.0; WINDOW / 2 + 1],
            cutoffs: vec![0; WINDOW / 2 + 1],
        }
    }

    pub fn push(&mut self, mags: &[f32]) -> bool {
        self.frames += 1;
        let peak = mags.iter().copied().fold(0.0f32, f32::max);
        if peak <= (self.global_peak + 1e-12) * 1e-3 {
            return false;
        }
        self.active += 1;
        let cutoff = mags
            .iter()
            .rposition(|m| 20.0 * (m / (peak + 1e-12) + 1e-12).log10() > CUTOFF_DB)
            .unwrap_or(0);
        self.cutoffs[cutoff] += 1;
        for (i, m) in mags.iter().enumerate() {
            self.sum[i] += *m as f64;
            self.square_sum[i] += (*m as f64).powi(2);
        }
        true
    }

    fn rank(&self, rank: u64) -> usize {
        let mut count = 0;
        for (i, n) in self.cutoffs.iter().enumerate() {
            count += n;
            if count > rank {
                return i;
            }
        }
        self.cutoffs.len() - 1
    }

    pub fn finish(self) -> SpectralMeasurements {
        let mut result = SpectralMeasurements {
            frames: self.frames,
            active_frames: self.active,
            ..Default::default()
        };
        if self.active == 0 {
            return result;
        }
        let bin_hz = self.rate as f64 / WINDOW as f64;
        let rank = (self.active - 1) as f64 * 0.95;
        let lo = self.rank(rank.floor() as u64) as f64;
        let hi = self.rank(rank.ceil() as u64) as f64;
        let cutoff = (lo + (hi - lo) * rank.fract()) * bin_hz;
        let avg: Vec<f64> = self.sum.iter().map(|x| x / self.active as f64).collect();
        let ref_peak = avg.iter().copied().fold(0.0f64, f64::max) + 1e-12;
        let db: Vec<f64> = avg
            .iter()
            .map(|v| 20.0 * (v / ref_peak + 1e-12).log10())
            .collect();
        let index = |hz: f64| ((hz.max(0.0) / bin_hz) as usize).min(avg.len() - 1);
        let left = index(cutoff - 400.0);
        let right = index(cutoff + 400.0);
        let sharp_lo = index(cutoff - 2500.0);
        let sharp_hi = ((cutoff + 625.0) / bin_hz) as usize;
        let sharp_slice = &db[sharp_lo..sharp_hi.min(db.len()).max(sharp_lo)];
        let total: f64 = avg.iter().sum();
        let hf_start = ((15_000.0 / bin_hz) as usize).min(avg.len());
        let cutoff_index = index(cutoff);
        let noise_power = self.square_sum[cutoff_index..].iter().sum::<f64>()
            / (self.active as f64 * (avg.len() - cutoff_index) as f64);
        let mean = self
            .cutoffs
            .iter()
            .enumerate()
            .map(|(i, n)| i as f64 * bin_hz * *n as f64)
            .sum::<f64>()
            / self.active as f64;
        let variance = self
            .cutoffs
            .iter()
            .enumerate()
            .map(|(i, n)| (i as f64 * bin_hz - mean).powi(2) * *n as f64)
            .sum::<f64>()
            / self.active as f64;
        result.cutoff_p95_hz = Some(cutoff);
        result.cutoff_variance_hz2 = Some(variance);
        result.cliff_depth_db = Some(if right > left {
            db[left] - db[right]
        } else {
            0.0
        });
        result.sharpness_db_per_bin = Some(
            sharp_slice
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0, f64::max),
        );
        result.hf_magnitude_ratio = Some(avg[hf_start..].iter().sum::<f64>() / (total + 1e-12));
        result.entropy_bits = Some(
            avg.iter()
                .filter(|x| **x > 0.0)
                .map(|x| {
                    let p = x / (total + 1e-12);
                    -p * p.log2()
                })
                .sum(),
        );
        result.noise_above_cutoff_db = Some(20.0 * (noise_power.sqrt() + 1e-12).log10());
        result
    }
}
