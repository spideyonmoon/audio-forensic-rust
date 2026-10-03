//! Causal high-band energy before the existing high-pass envelope peak centers.
//! Descriptive context only; no codec pre-echo inference.
use super::transients::{Biquad, CAP_SECONDS, MAX_EVENTS};
use crate::model::{
    AnalysisInterval, DetectorStatus, PrecedingEnergyAnalysis, PrecedingEnergyEvent, TransientEvent,
};

pub(crate) const ABSOLUTE_POWER: f64 = 1e-12;
pub(crate) const BASELINE_MULTIPLIER: f64 = 3.0;
const BINS: usize = 960; // -200..40 dB power, 0.25 dB steps.

struct BandFilter {
    sections: [Biquad; 4],
}
impl BandFilter {
    fn new(rate: u32) -> Self {
        let q = [
            1.0 / (2.0 * (std::f64::consts::PI / 8.0).cos()),
            1.0 / (2.0 * (3.0 * std::f64::consts::PI / 8.0).cos()),
        ];
        Self {
            sections: [
                Biquad::highpass_at(rate, 10000.0, q[0]),
                Biquad::highpass_at(rate, 10000.0, q[1]),
                Biquad::lowpass_at(rate, 20000.0, q[0]),
                Biquad::lowpass_at(rate, 20000.0, q[1]),
            ],
        }
    }
    fn push(&mut self, mut x: f64) -> f64 {
        for section in &mut self.sections {
            x = section.push(x);
        }
        x * x
    }
}

pub(crate) struct EnergySurvey {
    rate: u32,
    samples: u64,
    filter: Option<BandFilter>,
    histogram: Vec<u64>,
    count: u64,
}

impl EnergySurvey {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            samples: 0,
            filter: (rate > 40000).then(|| BandFilter::new(rate)),
            histogram: if rate > 40000 {
                vec![0; BINS + 3]
            } else {
                vec![]
            },
            count: 0,
        }
    }
    pub fn push(&mut self, x: f64) {
        if self.samples >= CAP_SECONDS * u64::from(self.rate) {
            return;
        }
        let frame = self.samples;
        self.samples += 1;
        let Some(filter) = &mut self.filter else {
            return;
        };
        let power = filter.push(x);
        if frame < u64::from(self.rate).div_ceil(50) {
            return;
        }
        let bin = if power == 0.0 {
            0
        } else if power <= 1e-20 {
            1
        } else if power >= 1e4 {
            BINS + 2
        } else {
            2 + (((10.0 * power.log10() + 200.0) * 4.0) as usize).min(BINS - 1)
        };
        self.histogram[bin] += 1;
        self.count += 1;
    }
    fn rank(&self, rank: u64) -> (f64, Option<f64>) {
        let mut count = 0;
        for (bin, &n) in self.histogram.iter().enumerate() {
            count += n;
            if count <= rank {
                continue;
            }
            return match bin {
                0 => (0.0, Some(0.0)),
                1 => (0.0, Some(1e-20)),
                i if i == BINS + 2 => (1e4, None),
                i => {
                    let db = -200.0 + (i - 2) as f64 * 0.25;
                    (10f64.powf(db / 10.0), Some(10f64.powf((db + 0.25) / 10.0)))
                }
            };
        }
        unreachable!("rank within baseline sample count")
    }
    pub fn into_collector(
        self,
        channel_index: usize,
        selection_available: bool,
    ) -> EnergyCollector {
        let warmup = u64::from(self.rate).div_ceil(50);
        let (lower, upper) = if self.count > 0 {
            let (a, b) = self.rank((self.count - 1) / 2);
            let (c, d) = self.rank(self.count / 2);
            (Some((a + c) / 2.0), b.zip(d).map(|(x, y)| (x + y) / 2.0))
        } else {
            (None, None)
        };
        let supported = self.filter.is_some();
        let threshold = upper
            .filter(|_| selection_available)
            .map(|p| ABSOLUTE_POWER.max(BASELINE_MULTIPLIER * p));
        EnergyCollector {
            rate: self.rate,
            samples: 0,
            filter: supported.then(|| BandFilter::new(self.rate)),
            ring: if supported {
                vec![0.0; self.rate.div_ceil(4) as usize]
            } else {
                vec![]
            },
            report: PrecedingEnergyAnalysis {
                channel_index,
                status: if supported {
                    DetectorStatus::Inconclusive
                } else {
                    DetectorStatus::Unsupported
                },
                interval: AnalysisInterval {
                    start_frame: 0,
                    end_frame: self.samples,
                },
                baseline_interval: (self.count > 0).then_some(AnalysisInterval {
                    start_frame: warmup,
                    end_frame: self.samples,
                }),
                baseline_samples: self.count,
                baseline_median_lower_power: lower,
                baseline_median_upper_power: upper,
                threshold_power: threshold,
                requested_lower_hz: 10000.0,
                requested_upper_hz: 20000.0,
                filter_order: 8,
                warmup_frames: warmup,
                lookback_start_frames: warmup,
                lookback_end_frames: u64::from(self.rate).div_ceil(100),
                history_frames: if supported {
                    u64::from(self.rate).div_ceil(4)
                } else {
                    0
                },
                selected_peak_count: selection_available.then_some(0),
                eligible_event_count: 0,
                startup_ineligible_count: 0,
                history_expired_count: 0,
                above_baseline_count: threshold.map(|_| 0),
                above_baseline_fraction: None,
                events_truncated: false,
                events: vec![],
            },
        }
    }
}

pub(crate) struct EnergyCollector {
    rate: u32,
    samples: u64,
    filter: Option<BandFilter>,
    ring: Vec<f64>,
    report: PrecedingEnergyAnalysis,
}
impl EnergyCollector {
    pub fn push(&mut self, x: f64) {
        if self.samples >= CAP_SECONDS * u64::from(self.rate) {
            return;
        }
        if let Some(filter) = &mut self.filter {
            let power = filter.push(x);
            self.ring[self.samples as usize % self.report.history_frames as usize] = power;
        }
        self.samples += 1;
    }
    pub fn observe(&mut self, event: &TransientEvent) {
        *self
            .report
            .selected_peak_count
            .as_mut()
            .expect("selection produced an event") += 1;
        let mut context = PrecedingEnergyEvent {
            frame: event.frame,
            status: DetectorStatus::Inconclusive,
            interval: None,
            mean_square: None,
            above_baseline: None,
            unavailable_reason: None,
        };
        if self.filter.is_none() {
            context.status = DetectorStatus::Unsupported;
            context.unavailable_reason = Some("band_unsupported".into());
        } else if event.frame < self.report.warmup_frames + self.report.lookback_start_frames {
            self.report.startup_ineligible_count += 1;
            context.unavailable_reason = Some("startup_or_short_context".into());
        } else {
            let start = event.frame - self.report.lookback_start_frames;
            let end = event.frame - self.report.lookback_end_frames;
            if start < self.samples.saturating_sub(self.report.history_frames) || end > self.samples
            {
                self.report.history_expired_count += 1;
                context.unavailable_reason = Some("history_expired".into());
            } else {
                let mean = (start..end)
                    .map(|frame| self.ring[frame as usize % self.report.history_frames as usize])
                    .sum::<f64>()
                    / (end - start) as f64;
                context.interval = Some(AnalysisInterval {
                    start_frame: start,
                    end_frame: end,
                });
                context.mean_square = Some(mean);
                context.status = DetectorStatus::Measured;
                self.report.eligible_event_count += 1;
                if let Some(threshold) = self.report.threshold_power {
                    let above = mean > threshold;
                    context.above_baseline = Some(above);
                    *self.report.above_baseline_count.as_mut().unwrap() += u64::from(above);
                }
            }
        }
        if self.report.events.len() < MAX_EVENTS {
            self.report.events.push(context);
        } else {
            self.report.events_truncated = true;
        }
    }
    pub fn finish(mut self) -> PrecedingEnergyAnalysis {
        if let Some(count) = self
            .report
            .above_baseline_count
            .filter(|_| self.report.eligible_event_count > 0)
        {
            self.report.status = DetectorStatus::Measured;
            self.report.above_baseline_fraction =
                Some(count as f64 / self.report.eligible_event_count as f64);
        }
        self.report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_peak_context_is_bounded_and_excluded_from_denominator() {
        let mut survey = EnergySurvey::new(48000);
        for _ in 0..24000 {
            survey.push(0.0);
        }
        let mut c = survey.into_collector(0, true);
        for _ in 0..24000 {
            c.push(0.0);
        }
        c.observe(&TransientEvent {
            frame: 1000,
            envelope_peak: 1.0,
        });
        c.observe(&TransientEvent {
            frame: 3000,
            envelope_peak: 1.0,
        });
        c.observe(&TransientEvent {
            frame: 23900,
            envelope_peak: 1.0,
        });
        let r = c.finish();
        assert_eq!(r.selected_peak_count, Some(3));
        assert_eq!(r.startup_ineligible_count, 1);
        assert_eq!(r.history_expired_count, 1);
        assert_eq!(r.eligible_event_count, 1);
        assert_eq!(r.above_baseline_fraction, Some(0.0));
        assert_eq!(r.events[2].mean_square, Some(0.0));
    }
}
