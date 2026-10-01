//! Bounded quiet-passage and window-normalized band-power measurements.
use crate::{
    dsp::{HOP, WINDOW},
    model::{AnalysisInterval, BandPowerMeasurement, DetectorStatus, NoiseAnalysis},
};
use rustfft::num_complex::Complex32;

pub(crate) const QUIET_THRESHOLD: f64 = 0.01;
pub(crate) const MAX_INTERVALS: usize = 16;
pub(crate) const MIN_BAND_FRAMES: u64 = 4;
const BINS: usize = WINDOW / 2 + 1;

pub(crate) struct NoiseStats {
    rate: u32,
    samples: u64,
    frames: u64,
    power: Vec<f64>,
    quiet_power: Vec<f64>,
    run_power: Vec<f64>,
    run_samples: u64,
    run_frames: u64,
    quiet_frames: u64,
    quiet_samples: u64,
    quiet_runs: u64,
    longest_run: u64,
    intervals: Vec<AnalysisInterval>,
}

impl NoiseStats {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            samples: 0,
            frames: 0,
            power: vec![0.0; BINS],
            quiet_power: vec![0.0; BINS],
            run_power: vec![0.0; BINS],
            run_samples: 0,
            run_frames: 0,
            quiet_frames: 0,
            quiet_samples: 0,
            quiet_runs: 0,
            longest_run: 0,
            intervals: Vec::with_capacity(MAX_INTERVALS),
        }
    }

    /// Must run before push_sample for the STFT's one-sample lookahead.
    /// At this point samples is the exclusive end of the evaluated window.
    pub fn push_spectrum(&mut self, spectrum: &[Complex32]) {
        debug_assert_eq!(self.samples, self.frames * HOP as u64 + WINDOW as u64);
        self.frames += 1;
        let inside_run = self.run_samples >= WINDOW as u64;
        for (i, bin) in spectrum.iter().enumerate() {
            let power = f64::from(bin.re).powi(2) + f64::from(bin.im).powi(2);
            self.power[i] += power;
            if inside_run {
                self.run_power[i] += power;
            }
        }
        self.run_frames += u64::from(inside_run);
    }

    pub fn push_sample(&mut self, sample: f64) {
        if sample.abs() < QUIET_THRESHOLD {
            self.run_samples += 1;
        } else {
            self.finish_run();
        }
        self.samples += 1;
    }

    fn finish_run(&mut self) {
        if self.run_samples >= u64::from(self.rate).div_ceil(2) {
            self.quiet_runs += 1;
            self.quiet_samples += self.run_samples;
            self.longest_run = self.longest_run.max(self.run_samples);
            self.quiet_frames += self.run_frames;
            for (total, power) in self.quiet_power.iter_mut().zip(&self.run_power) {
                *total += power;
            }
            if self.intervals.len() < MAX_INTERVALS {
                self.intervals.push(AnalysisInterval {
                    start_frame: self.samples - self.run_samples,
                    end_frame: self.samples,
                });
            }
        }
        if self.run_frames > 0 {
            self.run_power.fill(0.0);
        }
        self.run_samples = 0;
        self.run_frames = 0;
    }

    fn band(&self, lower: Option<f64>, upper: f64, normalization: f64) -> BandPowerMeasurement {
        let bin_hz = f64::from(self.rate) / WINDOW as f64;
        let start = lower
            .map(|lo| (lo / bin_hz).ceil() as usize)
            .unwrap_or(BINS);
        let end = (upper / bin_hz).floor() as usize;
        // Both bands exclude DC/Nyquist. At least two actual bins must exist.
        let supported = start > 0 && end < WINDOW / 2 && end >= start.saturating_add(1);
        let measure = |values: &[f64], frames: u64| {
            (supported && frames >= MIN_BAND_FRAMES)
                .then(|| values[start..=end].iter().sum::<f64>() * normalization / frames as f64)
        };
        let status = |frames| {
            if !supported {
                DetectorStatus::Unsupported
            } else if frames < MIN_BAND_FRAMES {
                DetectorStatus::Inconclusive
            } else {
                DetectorStatus::Measured
            }
        };
        let mean_square = measure(&self.power, self.frames);
        let quiet_mean_square = measure(&self.quiet_power, self.quiet_frames);
        // Exact zero has a measured linear value; its logarithm is undefined.
        let db = |power: Option<f64>| power.filter(|&p| p > 0.0).map(|p| 10.0 * p.log10());
        BandPowerMeasurement {
            status: status(self.frames),
            quiet_status: status(self.quiet_frames),
            requested_lower_hz: lower,
            requested_upper_hz: upper,
            lower_bin_hz: supported.then_some(start as f64 * bin_hz),
            upper_bin_hz: supported.then_some(end as f64 * bin_hz),
            bin_count: if supported { end - start + 1 } else { 0 },
            mean_square,
            rms_dbfs: db(mean_square),
            quiet_mean_square,
            quiet_rms_dbfs: db(quiet_mean_square),
        }
    }

    pub fn finish(mut self, channel_index: usize, cutoff_hz: Option<f64>) -> NoiseAnalysis {
        self.finish_run();
        // Use the same rounded f32 symmetric Hann coefficients as StreamingStft.
        let window_energy: f64 = (0..WINDOW)
            .map(|i| {
                let w = (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (WINDOW - 1) as f64).cos())
                    as f32;
                f64::from(w).powi(2)
            })
            .sum();
        let normalization = 2.0 / (WINDOW as f64 * window_energy);
        let upper = f64::from(self.rate) / 2.0 - 100.0;
        let high_band = self.band(Some(16000.0), upper.min(22000.0), normalization);
        let above_cutoff_band = self.band(
            cutoff_hz.filter(|&hz| hz > 0.0).map(|hz| hz + 1000.0),
            upper,
            normalization,
        );
        NoiseAnalysis {
            channel_index,
            interval: AnalysisInterval {
                start_frame: 0,
                end_frame: self.samples,
            },
            minimum_quiet_run_frames: u64::from(self.rate).div_ceil(2),
            quiet_runs: self.quiet_runs,
            quiet_samples: self.quiet_samples,
            longest_quiet_run_frames: self.longest_run,
            quiet_intervals_truncated: self.quiet_runs > self.intervals.len() as u64,
            quiet_intervals: self.intervals,
            stft_interval: (self.frames > 0).then(|| AnalysisInterval {
                start_frame: 0,
                end_frame: (self.frames - 1) * HOP as u64 + WINDOW as u64,
            }),
            stft_frames: self.frames,
            quiet_stft_frames: self.quiet_frames,
            high_band,
            above_cutoff_band,
        }
    }
}
