//! Streaming Pearson correlation between two native-channel band RMS envelopes.
use crate::{
    dsp::{HOP, WINDOW},
    model::{AnalysisInterval, BandEnvelope, DetectorStatus, EnvelopeAnalysis},
};
use std::ops::RangeInclusive;

pub(crate) const MIN_FRAMES: u64 = 10;
pub(crate) const ABSOLUTE_POWER: f64 = 1e-12;
pub(crate) const RELATIVE_POWER: f64 = 1e-8;
pub(crate) const MIN_STD: f64 = 1e-8;
pub(crate) const MIN_CV: f64 = 1e-3;

pub(crate) struct EnvelopeStats {
    rate: u32,
    frames: u64,
    active: u64,
    bands: [Option<RangeInclusive<usize>>; 2],
    normalization: f64,
    power: [f64; 2],
    broadband: f64,
    mean: [f64; 2],
    m2: [f64; 2],
    covariance: f64,
}

impl EnvelopeStats {
    pub fn new(rate: u32) -> Self {
        let bin_hz = f64::from(rate) / WINDOW as f64;
        let bands = [(1000.0, 8000.0), (16000.0, 22000.0)].map(|(lo, hi)| {
            let start = (lo / bin_hz).ceil() as usize;
            let end = (hi / bin_hz).floor() as usize;
            (hi < f64::from(rate) / 2.0 && end > start && end < WINDOW / 2).then_some(start..=end)
        });
        let energy: f64 = (0..WINDOW)
            .map(|i| {
                let w = (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (WINDOW - 1) as f64).cos())
                    as f32;
                f64::from(w).powi(2)
            })
            .sum();
        Self {
            rate,
            frames: 0,
            active: 0,
            bands,
            normalization: 1.0 / (WINDOW as f64 * energy),
            power: [0.0; 2],
            broadband: 0.0,
            mean: [0.0; 2],
            m2: [0.0; 2],
            covariance: 0.0,
        }
    }

    pub fn push(&mut self, magnitudes: &[f32], active: bool) {
        self.frames += 1;
        if !active {
            return;
        }
        self.active += 1;
        let mut powers = [0.0; 2];
        let mut broadband = 0.0;
        for (bin, &magnitude) in magnitudes.iter().enumerate() {
            let power = f64::from(magnitude).powi(2);
            broadband += if bin == 0 || bin == WINDOW / 2 {
                power
            } else {
                2.0 * power
            };
            for (index, band) in self.bands.iter().enumerate() {
                if band.as_ref().is_some_and(|b| b.contains(&bin)) {
                    powers[index] += 2.0 * power;
                }
            }
        }
        self.broadband += broadband * self.normalization;
        let mut delta = [0.0; 2];
        let mut values = [0.0; 2];
        for i in 0..2 {
            powers[i] *= self.normalization;
            self.power[i] += powers[i];
            values[i] = powers[i].sqrt();
            delta[i] = values[i] - self.mean[i];
            self.mean[i] += delta[i] / self.active as f64;
            self.m2[i] += delta[i] * (values[i] - self.mean[i]);
        }
        self.covariance += delta[0] * (values[1] - self.mean[1]);
    }

    fn band(&self, index: usize) -> BandEnvelope {
        let (lower, upper) = [(1000.0, 8000.0), (16000.0, 22000.0)][index];
        let range = self.bands[index].as_ref();
        let data = range.is_some() && self.active > 0;
        let power = data.then(|| self.power[index] / self.active as f64);
        let mean = data.then_some(self.mean[index]);
        let std = data.then(|| (self.m2[index].max(0.0) / self.active as f64).sqrt());
        let energy = power.is_some_and(|p| {
            p > ABSOLUTE_POWER.max(RELATIVE_POWER * self.broadband / self.active as f64)
        });
        let varying = std
            .zip(mean)
            .is_some_and(|(s, m)| s > MIN_STD && s > MIN_CV * m);
        let bin_hz = f64::from(self.rate) / WINDOW as f64;
        BandEnvelope {
            status: if range.is_none() {
                DetectorStatus::Unsupported
            } else if self.active >= MIN_FRAMES && energy && varying {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            requested_lower_hz: lower,
            requested_upper_hz: upper,
            lower_bin_hz: range.map(|r| *r.start() as f64 * bin_hz),
            upper_bin_hz: range.map(|r| *r.end() as f64 * bin_hz),
            bin_count: range.map_or(0, |r| r.end() - r.start() + 1),
            mean_square: power,
            mean_rms: mean,
            std_rms: std,
            energy_eligible: energy,
            variation_eligible: varying,
        }
    }

    pub fn finish(self, channel_index: usize) -> EnvelopeAnalysis {
        let mid = self.band(0);
        let high = self.band(1);
        let measured =
            mid.status == DetectorStatus::Measured && high.status == DetectorStatus::Measured;
        EnvelopeAnalysis {
            channel_index,
            status: if mid.status == DetectorStatus::Unsupported
                || high.status == DetectorStatus::Unsupported
            {
                DetectorStatus::Unsupported
            } else if measured {
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
            coefficient: measured
                .then(|| (self.covariance / (self.m2[0] * self.m2[1]).sqrt()).clamp(-1.0, 1.0)),
            mid_band: mid,
            high_band: high,
        }
    }
}
