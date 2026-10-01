//! Bounded native-channel implementation of the pinned Vorbis long-block search.
//! Captures at most twelve 3071-sample spans per channel; never a full audio track.
use super::mdct::Mdct;
use crate::model::{AnalysisInterval, DetectorStatus, VorbisAnalysis, VorbisProbe};

pub(crate) const SIZE: usize = 2048;
pub(crate) const HOP: usize = 1024;
pub(crate) const SPAN: usize = SIZE + HOP - 1;
pub(crate) const CAP_SECONDS: u64 = 180;
pub(crate) const HIT_SCORE: f64 = 0.03;

pub(crate) fn positions(frames: u64, rate: u32) -> Vec<u64> {
    if !matches!(rate, 44100 | 48000) {
        return vec![];
    }
    let length = frames.min(CAP_SECONDS * u64::from(rate));
    if length < (SIZE * 8) as u64 {
        return vec![];
    }
    let mut result: Vec<_> = (0..12)
        .map(|i| (SIZE as u64 + (length - 3 * SIZE as u64) * i / 11) / HOP as u64 * HOP as u64)
        .collect();
    result.dedup();
    result
}

struct Channel {
    position: u64,
    first: usize,
    spans: Vec<Vec<f64>>,
}

pub(crate) struct VorbisCollector {
    rate: u32,
    limit: u64,
    positions: Vec<u64>,
    channels: Vec<Channel>,
}

impl VorbisCollector {
    pub fn new(frames: u64, rate: u32, channels: usize) -> Self {
        let positions = positions(frames, rate);
        Self {
            rate,
            limit: frames.min(CAP_SECONDS * u64::from(rate)),
            channels: (0..channels)
                .map(|_| Channel {
                    position: 0,
                    first: 0,
                    spans: positions.iter().map(|_| Vec::with_capacity(SPAN)).collect(),
                })
                .collect(),
            positions,
        }
    }

    pub fn push(&mut self, channel: usize, sample: f64) {
        let state = &mut self.channels[channel];
        while state.first < self.positions.len()
            && state.position >= self.positions[state.first] + SPAN as u64
        {
            state.first += 1;
        }
        for index in state.first..self.positions.len() {
            if state.position < self.positions[index] {
                break;
            }
            state.spans[index].push(sample);
        }
        state.position += 1;
    }

    pub fn finish<E>(
        self,
        mut control: impl FnMut() -> Result<(), E>,
    ) -> Result<Vec<VorbisAnalysis>, E> {
        let window = (0..SIZE)
            .map(|i| {
                (std::f64::consts::FRAC_PI_2
                    * (std::f64::consts::PI * (i as f64 + 0.5) / SIZE as f64)
                        .sin()
                        .powi(2))
                .sin()
            })
            .collect();
        let mut mdct = Mdct::new(window);
        let lo = 1000 * SIZE / self.rate as usize;
        let hi = 19000 * SIZE / self.rate as usize;
        let mut results = vec![];
        for (channel_index, state) in self.channels.into_iter().enumerate() {
            control()?;
            let energies: Vec<_> = state
                .spans
                .iter()
                .map(|span| span[..SIZE].iter().map(|x| x * x).sum::<f64>() / SIZE as f64)
                .collect();
            let floor = 1e-7f64.max(energies.iter().copied().fold(0.0, f64::max) * 1e-6);
            let mut probes: Vec<_> = self
                .positions
                .iter()
                .zip(&energies)
                .map(|(&start, &energy)| VorbisProbe {
                    interval: AnalysisInterval {
                        start_frame: start,
                        end_frame: start + SPAN as u64,
                    },
                    anchor_rms: energy.sqrt(),
                    active: energy > floor,
                    baseline_zero_fraction: None,
                    maximum_zero_fraction: None,
                    selected_phase_zero_fraction: None,
                    supports_selected_phase: false,
                })
                .collect();
            let active = probes.iter().filter(|p| p.active).count();
            let mut curves = vec![];
            if active >= 4 {
                for (i, span) in state.spans.iter().enumerate() {
                    if !probes[i].active {
                        continue;
                    }
                    let mut pooled = [0.0f64; 128];
                    for phase in 0..HOP {
                        if phase % 64 == 0 {
                            control()?;
                        }
                        let coefficients = &mdct.process(&span[phase..phase + SIZE])[lo..hi];
                        let threshold = (coefficients.iter().map(|x| x.abs()).fold(0.0, f64::max)
                            * 1e-5)
                            .max(1e-8);
                        let fraction = coefficients.iter().filter(|x| x.abs() < threshold).count()
                            as f64
                            / coefficients.len() as f64;
                        pooled[phase % 128] = pooled[phase % 128].max(fraction);
                    }
                    let mut sorted = pooled;
                    sorted.sort_by(f64::total_cmp);
                    probes[i].baseline_zero_fraction = Some((sorted[63] + sorted[64]) / 2.0);
                    probes[i].maximum_zero_fraction = Some(sorted[127]);
                    curves.push((i, pooled));
                }
            }
            let mut score = 0.0;
            let mut phase = None;
            let mut supporting = 0;
            for phi in 0..128 {
                let mut support = 0;
                let mut strength = 0.0;
                for &(index, ref curve) in &curves {
                    let baseline = probes[index].baseline_zero_fraction.unwrap();
                    let excess = curve[phi] - baseline;
                    support += usize::from(excess >= 0.03 && curve[phi] >= 3.0 * baseline + 0.01);
                    strength += excess.max(0.0);
                }
                strength /= active.max(1) as f64;
                if support >= 4.max(active.div_ceil(2)) && strength > score {
                    score = strength;
                    phase = Some(phi);
                    supporting = support;
                }
            }
            if let Some(phi) = phase {
                for (index, curve) in curves {
                    let baseline = probes[index].baseline_zero_fraction.unwrap();
                    probes[index].selected_phase_zero_fraction = Some(curve[phi]);
                    probes[index].supports_selected_phase =
                        curve[phi] - baseline >= 0.03 && curve[phi] >= 3.0 * baseline + 0.01;
                }
            }
            let status = if !matches!(self.rate, 44100 | 48000) {
                DetectorStatus::Unsupported
            } else if active < 4 {
                DetectorStatus::Inconclusive
            } else if score >= HIT_SCORE {
                DetectorStatus::Hit
            } else {
                DetectorStatus::NotDetected
            };
            results.push(VorbisAnalysis {
                channel_index,
                status,
                search_limit_frames: self.limit,
                zero_excess_score: (active >= 4).then_some(score),
                phase_mod_128: phase,
                supporting_probes: supporting,
                active_probes: active,
                probes,
            });
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchors_remain_bounded_and_inside_the_capped_prefix() {
        for rate in [44100, 48000, 96000] {
            for length in [0, 100, 16383, 16384, 88200, 48_000_000] {
                let starts = positions(length, rate);
                assert!(starts.len() <= 12);
                assert!(starts.windows(2).all(|w| w[0] < w[1]));
                assert!(starts.iter().all(|start| start % 1024 == 0
                    && start + SPAN as u64 <= length.min(CAP_SECONDS * u64::from(rate))));
                if rate == 96000 || length < 16384 {
                    assert!(starts.is_empty());
                }
            }
        }
    }
    #[test]
    fn processing_obeys_cancellation_between_phase_batches() {
        let mut collector = VorbisCollector::new(44100, 44100, 1);
        for i in 0..44100 {
            collector.push(0, (i as f64 * 1.345).sin());
        }
        let mut calls = 0;
        let result = collector.finish(|| {
            calls += 1;
            if calls == 5 { Err("cancelled") } else { Ok(()) }
        });
        assert!(matches!(result, Err("cancelled")));
        assert_eq!(calls, 5);
    }
}
