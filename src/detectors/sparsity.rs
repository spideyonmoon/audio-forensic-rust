//! Bounded below-cutoff relative-magnitude counts, without codec interpretation.
use crate::{
    dsp::{HOP, WINDOW},
    model::{AnalysisInterval, DetectorStatus, SparsityAnalysis},
};

pub(crate) const MIN_FRAMES: u64 = 4;
pub(crate) const MIN_BINS: usize = 10;
pub(crate) const PEAK_AMPLITUDE_FLOOR: f64 = 1e-6;
pub(crate) const RELATIVE_DB: f64 = -95.0;

pub(crate) struct SparsityStats {
    rate: u32,
    frames: u64,
    active: u64,
    eligible: u64,
    sparse: Vec<u64>,
    amplitude_scale: f64,
    relative_magnitude: f64,
}

impl SparsityStats {
    pub fn new(rate: u32) -> Self {
        let sum: f64 = (0..WINDOW)
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
            eligible: 0,
            sparse: vec![0; WINDOW / 2 + 1],
            amplitude_scale: 2.0 / sum,
            relative_magnitude: 10.0f64.powf(RELATIVE_DB / 20.0),
        }
    }

    pub fn push(&mut self, magnitudes: &[f32], active: bool) {
        self.frames += 1;
        if !active {
            return;
        }
        self.active += 1;
        let peak = magnitudes.iter().copied().fold(0.0f32, f32::max) as f64;
        if peak * self.amplitude_scale <= PEAK_AMPLITUDE_FLOOR {
            return;
        }
        self.eligible += 1;
        let threshold = peak * self.relative_magnitude;
        for (count, &magnitude) in self.sparse.iter_mut().zip(magnitudes) {
            *count += u64::from(f64::from(magnitude) < threshold);
        }
    }

    pub fn finish(self, channel_index: usize, cutoff_hz: Option<f64>) -> SparsityAnalysis {
        let bin_hz = f64::from(self.rate) / WINDOW as f64;
        // Defer the global p95 cutoff until the existing spectral pass finishes.
        // Explicitly exclude DC, Nyquist, and every bin at/above that cutoff.
        let bins = cutoff_hz.map_or(0, |cutoff| {
            (1..WINDOW / 2)
                .take_while(|&k| k as f64 * bin_hz < cutoff)
                .count()
        });
        let measured = self.eligible >= MIN_FRAMES && bins >= MIN_BINS;
        let sparse = measured.then(|| self.sparse[1..=bins].iter().sum::<u64>());
        let total = measured.then_some(self.eligible * bins as u64);
        SparsityAnalysis {
            channel_index,
            status: if measured {
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
            eligible_frames: self.eligible,
            below_peak_floor_frames: self.active - self.eligible,
            cutoff_p95_hz: cutoff_hz,
            lower_bin_hz: (bins > 0).then_some(bin_hz),
            upper_bin_hz: (bins > 0).then_some(bins as f64 * bin_hz),
            bin_count: bins,
            sparse_bin_observations: sparse,
            total_bin_observations: total,
            fraction: sparse.zip(total).map(|(s, n)| s as f64 / n as f64),
        }
    }
}
