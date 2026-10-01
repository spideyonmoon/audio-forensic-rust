//! Two-band spectral slope observations, without analog-source interpretation.
use crate::{
    dsp::{HOP, WINDOW},
    model::{AnalysisInterval, DetectorStatus, RolloffAnalysis, RolloffBand},
};

pub(crate) const MIN_FRAMES: u64 = 4;
pub(crate) const ABSOLUTE_BIN_AMPLITUDE: f64 = 1e-6;
pub(crate) const RELATIVE_BIN_AMPLITUDE: f64 = 1e-4;
const BINS: usize = WINDOW / 2 + 1;

pub(crate) struct RolloffStats {
    rate: u32,
    frames: u64,
    active: u64,
    magnitude_sum: Vec<f64>,
    amplitude_scale: f64,
}

impl RolloffStats {
    pub fn new(rate: u32) -> Self {
        let window_sum: f64 = (0..WINDOW)
            .map(|i| {
                f64::from(
                    (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (WINDOW - 1) as f64).cos())
                        as f32,
                )
            })
            .sum();
        Self {
            rate,
            frames: 0,
            active: 0,
            magnitude_sum: vec![0.0; BINS],
            amplitude_scale: 2.0 / window_sum,
        }
    }

    pub fn push(&mut self, magnitudes: &[f32], active: bool) {
        self.frames += 1;
        if active {
            self.active += 1;
            for (sum, &value) in self.magnitude_sum.iter_mut().zip(magnitudes) {
                *sum += f64::from(value);
            }
        }
    }

    fn band(&self, center: f64, reference_peak: f64) -> RolloffBand {
        let lower = center - 250.0;
        let upper = center + 250.0;
        let bin_hz = f64::from(self.rate) / WINDOW as f64;
        let start = (lower / bin_hz).ceil() as usize;
        let end = (upper / bin_hz).floor() as usize;
        let supported =
            upper < f64::from(self.rate) / 2.0 && start > 0 && end < WINDOW / 2 && end > start;
        let mut eligible = 0;
        let mut sum_db = 0.0;
        let mut minimum: Option<f64> = None;
        if supported && self.active > 0 {
            let floor = ABSOLUTE_BIN_AMPLITUDE.max(reference_peak * RELATIVE_BIN_AMPLITUDE);
            for &sum in &self.magnitude_sum[start..=end] {
                let amplitude = sum / self.active as f64 * self.amplitude_scale;
                minimum = Some(minimum.unwrap_or(amplitude).min(amplitude));
                if amplitude > floor {
                    eligible += 1;
                    sum_db += 20.0 * (amplitude / reference_peak).log10();
                }
            }
        }
        let bins = if supported { end - start + 1 } else { 0 };
        let measured = supported && self.active >= MIN_FRAMES && eligible == bins;
        RolloffBand {
            status: if !supported {
                DetectorStatus::Unsupported
            } else if measured {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            requested_center_hz: center,
            requested_lower_hz: lower,
            requested_upper_hz: upper,
            lower_bin_hz: supported.then_some(start as f64 * bin_hz),
            upper_bin_hz: supported.then_some(end as f64 * bin_hz),
            mean_bin_hz: supported.then_some((start + end) as f64 / 2.0 * bin_hz),
            bin_count: bins,
            eligible_bins: eligible,
            minimum_bin_amplitude: minimum,
            mean_relative_level_db: measured.then(|| sum_db / bins as f64),
        }
    }

    pub fn finish(self, channel_index: usize) -> RolloffAnalysis {
        let reference_peak = if self.active > 0 {
            self.magnitude_sum.iter().copied().fold(0.0, f64::max) / self.active as f64
                * self.amplitude_scale
        } else {
            0.0
        };
        let lower = self.band(12000.0, reference_peak);
        let upper = self.band(18000.0, reference_peak);
        let slope = lower
            .mean_relative_level_db
            .zip(upper.mean_relative_level_db)
            .map(|(lo, hi)| {
                (hi - lo) * 1000.0 / (upper.mean_bin_hz.unwrap() - lower.mean_bin_hz.unwrap())
            });
        RolloffAnalysis {
            channel_index,
            status: if lower.status == DetectorStatus::Unsupported
                || upper.status == DetectorStatus::Unsupported
            {
                DetectorStatus::Unsupported
            } else if slope.is_some() {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            interval: (self.frames > 0).then_some(AnalysisInterval {
                start_frame: 0,
                end_frame: self.frames.saturating_sub(1) * HOP as u64 + WINDOW as u64,
            }),
            stft_frames: self.frames,
            active_frames: self.active,
            reference_peak_amplitude: (self.active > 0).then_some(reference_peak),
            lower_band: lower,
            upper_band: upper,
            slope_db_per_khz: slope,
        }
    }
}
