//! Two-pass bounded high-pass envelope observations; no source-medium inference.
use crate::model::{AnalysisInterval, DetectorStatus, TransientAnalysis, TransientEvent};

pub(crate) const CAP_SECONDS: u64 = 180;
pub(crate) const MAX_EVENTS: usize = 128;
pub(crate) const ABSOLUTE_THRESHOLD: f64 = 1e-4;
pub(crate) const MEDIAN_MULTIPLIER: f64 = 3.0;
const HISTOGRAM_STEPS: usize = 880; // -180..40 dB, in 0.25 dB steps.

pub(crate) struct Biquad {
    b0: f64,
    b1: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn highpass(rate: u32, q: f64) -> Self {
        Self::highpass_at(rate, 1000.0, q)
    }

    pub(crate) fn highpass_at(rate: u32, cutoff: f64, q: f64) -> Self {
        let omega = std::f64::consts::TAU * cutoff / f64::from(rate);
        let alpha = omega.sin() / (2.0 * q);
        let norm = 1.0 + alpha;
        let b0 = (1.0 + omega.cos()) / (2.0 * norm);
        Self {
            b0,
            b1: -2.0 * b0,
            a1: -2.0 * omega.cos() / norm,
            a2: (1.0 - alpha) / norm,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub(crate) fn lowpass_at(rate: u32, cutoff: f64, q: f64) -> Self {
        let omega = std::f64::consts::TAU * cutoff / f64::from(rate);
        let alpha = omega.sin() / (2.0 * q);
        let norm = 1.0 + alpha;
        let b0 = (1.0 - omega.cos()) / (2.0 * norm);
        Self {
            b0,
            b1: 2.0 * b0,
            a1: -2.0 * omega.cos() / norm,
            a2: (1.0 - alpha) / norm,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub(crate) fn push(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b0 * x - self.a2 * y;
        // Avoid persistent subnormal arithmetic after an impulse decays. This
        // -600 dBFS state floor is far below every measurement/threshold floor.
        if self.z1.abs() < 1e-30 {
            self.z1 = 0.0;
        }
        if self.z2.abs() < 1e-30 {
            self.z2 = 0.0;
        }
        y
    }
}

struct Envelope {
    rate: u32,
    filters: [Biquad; 2],
    ring: Vec<f64>,
    cursor: usize,
    sum: f64,
    samples: u64,
}

impl Envelope {
    fn new(rate: u32) -> Self {
        Self {
            rate,
            filters: [
                Biquad::highpass(rate, 1.0 / (2.0 * (std::f64::consts::PI / 8.0).cos())),
                Biquad::highpass(rate, 1.0 / (2.0 * (3.0 * std::f64::consts::PI / 8.0).cos())),
            ],
            ring: vec![0.0; (rate as usize / 2000) | 1],
            cursor: 0,
            sum: 0.0,
            samples: 0,
        }
    }

    fn warmup(&self) -> u64 {
        u64::from(self.rate).div_ceil(50)
    }

    fn push(&mut self, sample: f64) -> Option<(u64, f64)> {
        if self.samples >= CAP_SECONDS * u64::from(self.rate) {
            return None;
        }
        let mut x = sample;
        for filter in &mut self.filters {
            x = filter.push(x);
        }
        let rectified = x.abs();
        self.sum += rectified - self.ring[self.cursor];
        self.ring[self.cursor] = rectified;
        self.cursor = (self.cursor + 1) % self.ring.len();
        // Periodic rebasing bounds accumulated rolling-sum roundoff.
        if self.cursor == 0 {
            self.sum = self.ring.iter().sum();
        }
        self.samples += 1;
        let half = self.ring.len() as u64 / 2;
        if self.samples < self.ring.len() as u64 {
            return None;
        }
        let center = self.samples - 1 - half;
        (center >= self.warmup()).then(|| {
            (
                center,
                self.sum.max(0.0) / self.ring.len() as f64 * std::f64::consts::FRAC_PI_2,
            )
        })
    }
}

pub(crate) struct TransientSurvey {
    envelope: Envelope,
    histogram: Vec<u64>,
    envelope_samples: u64,
}

impl TransientSurvey {
    pub fn new(rate: u32) -> Self {
        Self {
            envelope: Envelope::new(rate),
            histogram: vec![0; HISTOGRAM_STEPS + 3],
            envelope_samples: 0,
        }
    }

    pub fn push(&mut self, sample: f64) {
        let Some((_, level)) = self.envelope.push(sample) else {
            return;
        };
        let bin = if level == 0.0 {
            0
        } else if level <= 1e-9 {
            1
        } else if level >= 100.0 {
            HISTOGRAM_STEPS + 2
        } else {
            2 + (((20.0 * level.log10() + 180.0) * 4.0) as usize).min(HISTOGRAM_STEPS - 1)
        };
        self.histogram[bin] += 1;
        self.envelope_samples += 1;
    }

    fn rank_bounds(&self, rank: u64) -> (f64, Option<f64>) {
        let mut count = 0;
        for (i, &n) in self.histogram.iter().enumerate() {
            count += n;
            if count <= rank {
                continue;
            }
            return match i {
                0 => (0.0, Some(0.0)),
                1 => (0.0, Some(1e-9)),
                i if i == HISTOGRAM_STEPS + 2 => (100.0, None),
                i => {
                    let db = -180.0 + (i - 2) as f64 * 0.25;
                    (
                        10.0f64.powf(db / 20.0),
                        Some(10.0f64.powf((db + 0.25) / 20.0)),
                    )
                }
            };
        }
        unreachable!("median rank is below the envelope sample count")
    }

    pub fn into_detector(self, channel_index: usize) -> TransientDetector {
        let rate = self.envelope.rate;
        let half = self.envelope.ring.len() as u64 / 2;
        let peak_start = self.envelope.warmup() + 1;
        let peak_end = self.envelope.samples.saturating_sub(half + 1);
        let interval = (peak_end > peak_start).then_some(AnalysisInterval {
            start_frame: peak_start,
            end_frame: peak_end,
        });
        let (lower, upper) = if self.envelope_samples > 0 {
            let (a, b) = self.rank_bounds((self.envelope_samples - 1) / 2);
            let (c, d) = self.rank_bounds(self.envelope_samples / 2);
            (Some((a + c) / 2.0), b.zip(d).map(|(x, y)| (x + y) / 2.0))
        } else {
            (None, None)
        };
        let measured = interval
            .as_ref()
            .is_some_and(|i| i.end_frame - i.start_frame >= u64::from(rate).div_ceil(2))
            && upper.is_some();
        let threshold = upper
            .filter(|_| measured)
            .map(|m| ABSOLUTE_THRESHOLD.max(MEDIAN_MULTIPLIER * m));
        let report = TransientAnalysis {
            channel_index,
            status: if measured {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            interval: AnalysisInterval {
                start_frame: 0,
                end_frame: self.envelope.samples,
            },
            eligible_peak_interval: interval,
            envelope_samples: self.envelope_samples,
            highpass_cutoff_hz: 1000.0,
            filter_order: 4,
            smoothing_frames: self.envelope.ring.len() as u32,
            warmup_frames: self.envelope.warmup(),
            minimum_peak_distance_frames: (rate / 100).max(1),
            baseline_median_lower: lower,
            baseline_median_upper: upper,
            envelope_threshold: threshold,
            peak_count: measured.then_some(0),
            peaks_per_minute: None,
            events_truncated: false,
            events: vec![],
        };
        TransientDetector {
            envelope: Envelope::new(rate),
            report,
            picker: PeakPicker::default(),
            last_accepted: None,
        }
    }
}

#[derive(Default)]
struct PeakPicker {
    previous: Option<f64>,
    plateau_start: u64,
    rising: bool,
}

impl PeakPicker {
    /// Plateau midpoint matches ordinary local-maximum geometry. The first and
    /// last envelope points never qualify without both neighboring samples.
    fn push(&mut self, frame: u64, value: f64) -> Option<(u64, f64)> {
        let mut peak = None;
        if let Some(previous) = self.previous {
            if value > previous {
                self.plateau_start = frame;
                self.rising = true;
            } else if value < previous {
                if self.rising {
                    peak = Some((
                        self.plateau_start + (frame - 1 - self.plateau_start) / 2,
                        previous,
                    ));
                }
                self.rising = false;
            }
        }
        self.previous = Some(value);
        peak
    }
}

pub(crate) struct TransientDetector {
    envelope: Envelope,
    report: TransientAnalysis,
    picker: PeakPicker,
    last_accepted: Option<u64>,
}

impl TransientDetector {
    pub fn selection_available(&self) -> bool {
        self.report.envelope_threshold.is_some()
    }

    pub fn push(&mut self, sample: f64) -> Option<TransientEvent> {
        let threshold = self.report.envelope_threshold?;
        let (frame, value) = self.envelope.push(sample)?;
        let (frame, level) = self.picker.push(frame, value)?;
        if level <= threshold
            || self.last_accepted.is_some_and(|last| {
                frame - last < u64::from(self.report.minimum_peak_distance_frames)
            })
        {
            return None;
        }
        self.last_accepted = Some(frame);
        *self.report.peak_count.as_mut().unwrap() += 1;
        if self.report.events.len() < MAX_EVENTS {
            self.report.events.push(TransientEvent {
                frame,
                envelope_peak: level,
            });
        } else {
            self.report.events_truncated = true;
        }
        Some(TransientEvent {
            frame,
            envelope_peak: level,
        })
    }

    pub fn finish(mut self) -> TransientAnalysis {
        if let Some(count) = self.report.peak_count {
            let interval = self.report.eligible_peak_interval.as_ref().unwrap();
            self.report.peaks_per_minute = Some(
                count as f64 * 60.0 * f64::from(self.envelope.rate)
                    / (interval.end_frame - interval.start_frame) as f64,
            );
        }
        self.report
    }
}
