//! Bounded quiet-block levels and spectral color, without source-depth inference.
use crate::model::{
    AnalysisInterval, DetectorStatus, NoiseFloorAnalysis, QuietBlock, QuietColorBand,
};
use rustfft::{Fft, FftPlanner, num_complex::Complex64};
use std::{ops::RangeInclusive, sync::Arc};

pub(crate) const MAX_BLOCKS: usize = 300;
pub(crate) const MIN_NONZERO: usize = 20;
pub(crate) const ABSOLUTE_POWER: f64 = 1e-24;
pub(crate) const RELATIVE_POWER: f64 = 1e-12;

// Strict rational edges, avoiding floating-point membership at exact bin boundaries.
fn band_bins(rate: u32, block: usize, high: bool) -> RangeInclusive<usize> {
    let n = block as u64;
    let (lower, upper, denominator) = if high {
        (33 * n, 45 * n, 100)
    } else {
        (150 * n, 2000 * n, u64::from(rate))
    };
    (lower / denominator + 1) as usize..=((upper - 1) / denominator) as usize
}

pub(crate) struct FloorSurvey {
    rate: u32,
    block: usize,
    samples: u64,
    sum: f64,
    scale: f64,
    underflow: usize,
    rms: Vec<f64>,
}

impl FloorSurvey {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            block: (rate / 10) as usize,
            samples: 0,
            sum: 0.,
            scale: 0.,
            underflow: 0,
            rms: Vec::with_capacity(MAX_BLOCKS),
        }
    }

    pub fn push(&mut self, x: f64) {
        if self.rms.len() == MAX_BLOCKS {
            return;
        }
        // Scaled sum of squares keeps very small finite float PCM distinct from silence.
        let amplitude = x.abs();
        if amplitude > self.scale {
            self.sum = 1. + self.sum * (self.scale / amplitude).powi(2);
            self.scale = amplitude;
        } else if amplitude > 0. {
            self.sum += (amplitude / self.scale).powi(2);
        }
        self.samples += 1;
        if self.samples % self.block as u64 == 0 {
            let rms = self.scale * (self.sum / self.block as f64).sqrt();
            if self.scale > 0. && rms == 0. {
                self.underflow += 1;
            }
            self.rms.push(rms);
            self.sum = 0.;
            self.scale = 0.;
        }
    }

    pub fn into_collector(self, channel_index: usize, analyzed_frames: u64) -> FloorCollector {
        let mut order: Vec<_> = self
            .rms
            .iter()
            .enumerate()
            .filter(|(_, x)| **x > 0.)
            .collect();
        order.sort_by(|a, b| a.1.total_cmp(b.1).then(a.0.cmp(&b.0)));
        let measured = order.len() >= MIN_NONZERO;
        let percentile = |q: f64| {
            if !measured {
                return None;
            }
            let pos = (order.len() - 1) as f64 * q;
            let lo = pos.floor() as usize;
            let hi = pos.ceil() as usize;
            Some(*order[lo].1 + (*order[hi].1 - *order[lo].1) * pos.fract())
        };
        let low = percentile(0.015);
        let high = percentile(0.99);
        let mut selected: Vec<_> = if measured {
            order
                .iter()
                .take((order.len() / 10).max(3))
                .map(|(i, rms)| QuietBlock {
                    block_index: *i,
                    rms: **rms,
                })
                .collect()
        } else {
            vec![]
        };
        selected.sort_by_key(|b| b.block_index);
        let low_bins = band_bins(self.rate, self.block, false);
        let high_bins = band_bins(self.rate, self.block, true);
        let band = |lower: f64, upper: f64, range: RangeInclusive<usize>| {
            let bins: Vec<_> = range.collect();
            QuietColorBand {
                requested_lower_hz: lower,
                requested_upper_hz: upper,
                lower_bin_hz: bins
                    .first()
                    .map(|&k| k as f64 * self.rate as f64 / self.block as f64),
                upper_bin_hz: bins
                    .last()
                    .map(|&k| k as f64 * self.rate as f64 / self.block as f64),
                bin_count: bins.len(),
                mean_bin_power: None,
                energy_eligible: false,
            }
        };
        let report = NoiseFloorAnalysis {
            channel_index,
            status: if measured {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            interval: (!self.rms.is_empty()).then_some(AnalysisInterval {
                start_frame: 0,
                end_frame: (self.rms.len() * self.block) as u64,
            }),
            block_frames: self.block,
            inspected_frames: self.samples,
            trailing_frames: self.samples % self.block as u64,
            coverage_capped: analyzed_frames > self.samples,
            complete_blocks: self.rms.len(),
            zero_blocks: self.rms.len() - order.len() - self.underflow,
            rms_underflow_blocks: self.underflow,
            nonzero_blocks: order.len(),
            nonzero_rms_p015: low,
            nonzero_rms_p99: high,
            nonzero_rms_p015_dbfs: low.map(|x| 20. * x.log10()),
            nonzero_rms_p99_dbfs: high.map(|x| 20. * x.log10()),
            selected_blocks: selected,
            color_status: DetectorStatus::Inconclusive,
            low_band: band(150., 2000., low_bins.clone()),
            high_band: band(
                self.rate as f64 * 0.33,
                self.rate as f64 * 0.45,
                high_bins.clone(),
            ),
            high_minus_low_db: None,
        };
        let fft = FftPlanner::new().plan_fft_forward(self.block);
        let scratch = vec![Complex64::default(); fft.get_inplace_scratch_len()];
        let window: Vec<_> = (0..self.block)
            .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (self.block - 1) as f64).cos())
            .collect();
        let scale = 2. / (self.block as f64 * window.iter().map(|x| x * x).sum::<f64>());
        FloorCollector {
            report,
            low_bins,
            high_bins,
            position: 0,
            selected_cursor: 0,
            fft,
            scratch,
            window,
            buffer: vec![Complex64::default(); self.block],
            scale,
            low_power: 0.,
            high_power: 0.,
            total_power: 0.,
        }
    }
}

pub(crate) struct FloorCollector {
    report: NoiseFloorAnalysis,
    low_bins: RangeInclusive<usize>,
    high_bins: RangeInclusive<usize>,
    position: u64,
    selected_cursor: usize,
    fft: Arc<dyn Fft<f64>>,
    scratch: Vec<Complex64>,
    window: Vec<f64>,
    buffer: Vec<Complex64>,
    scale: f64,
    low_power: f64,
    high_power: f64,
    total_power: f64,
}

impl FloorCollector {
    pub fn push(&mut self, x: f64) {
        let Some(selected) = self.report.selected_blocks.get(self.selected_cursor) else {
            return;
        };
        let n = self.report.block_frames;
        let block_index = self.position / n as u64;
        let offset = (self.position % n as u64) as usize;
        self.position += 1;
        if block_index != selected.block_index as u64 {
            return;
        }
        self.buffer[offset] = Complex64::new(x * self.window[offset], 0.);
        if offset + 1 != n {
            return;
        }
        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        for (k, value) in self.buffer[..=n / 2].iter().enumerate() {
            let power = value.norm_sqr() * self.scale;
            self.total_power += if k == 0 || (n % 2 == 0 && k == n / 2) {
                power / 2.
            } else {
                power
            };
            if self.low_bins.contains(&k) {
                self.low_power += power;
            }
            if self.high_bins.contains(&k) {
                self.high_power += power;
            }
        }
        self.selected_cursor += 1;
    }

    pub fn finish(mut self) -> NoiseFloorAnalysis {
        let count = self.report.selected_blocks.len();
        if count > 0 && self.selected_cursor == count {
            let floor = ABSOLUTE_POWER.max(RELATIVE_POWER * self.total_power / count as f64);
            for (band, sum) in [
                (&mut self.report.low_band, self.low_power),
                (&mut self.report.high_band, self.high_power),
            ] {
                if band.bin_count >= 2 {
                    let power = sum / (count * band.bin_count) as f64;
                    band.mean_bin_power = Some(power);
                    band.energy_eligible = power > floor;
                }
            }
            if self.report.low_band.energy_eligible && self.report.high_band.energy_eligible {
                self.report.color_status = DetectorStatus::Measured;
                self.report.high_minus_low_db = Some(
                    10. * (self.report.high_band.mean_bin_power.unwrap()
                        / self.report.low_band.mean_bin_power.unwrap())
                    .log10(),
                );
            }
        }
        self.report
    }
}
