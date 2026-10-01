//! Bounded streaming counterpart of Python's foreign-Nyquist measurements.
//! All matching hypotheses are retained. Filters and processing can mimic them.
use crate::{
    dsp::WINDOW,
    model::{DetectorStatus, ResamplingAnalysis, ResamplingCandidate},
};

pub(crate) const SOURCE_RATES: [u32; 4] = [44100, 48000, 88200, 96000];
pub(crate) const MIN_FRAMES: u64 = 8;

struct Mirror {
    rate: u32,
    pairs: Vec<(usize, usize, f64)>,
    sum: f64,
}

pub(crate) struct ResamplingStats {
    rate: u32,
    active: u64,
    sums: Vec<f64>,
    mirrors: Vec<Mirror>,
}

impl ResamplingStats {
    pub fn new(rate: u32) -> Self {
        let bin_hz = rate as f64 / WINDOW as f64;
        let len = WINDOW / 2 + 1;
        let mirrors = SOURCE_RATES
            .into_iter()
            .filter(|src| *src as f64 / 2.0 + 1200.0 <= rate as f64 / 2.0 - 200.0)
            .map(|src| {
                let nyquist = src as f64 / 2.0;
                let c = (nyquist / bin_hz).round_ties_even() as isize;
                let g = (300.0 / bin_hz) as isize;
                let half = ((1800.0 / bin_hz) as isize)
                    .min(len as isize - 4 - c - g)
                    .min(c - g);
                let pairs = if half as f64 * bin_hz < 700.0 {
                    vec![]
                } else {
                    (c - g - half..c - g)
                        .map(|lo| {
                            let target = nyquist * 2.0 / bin_hz - lo as f64;
                            (lo as usize, target.floor() as usize, target.fract())
                        })
                        .collect()
                };
                Mirror {
                    rate: src,
                    pairs,
                    sum: 0.0,
                }
            })
            .collect();
        Self {
            rate,
            active: 0,
            sums: vec![0.0; len],
            mirrors,
        }
    }

    /// Called only for frames accepted by the shared spectral activity mask.
    pub fn push(&mut self, mags: &[f32]) {
        if self.mirrors.is_empty() {
            return;
        }
        self.active += 1;
        for (sum, value) in self.sums.iter_mut().zip(mags) {
            *sum += *value as f64;
        }
        for mirror in &mut self.mirrors {
            // Online centered covariance avoids cancellation for nearly flat bands.
            let (mut mean_l, mut mean_h, mut var_l, mut var_h, mut covariance) =
                (0.0, 0.0, 0.0, 0.0, 0.0);
            for (i, &(lo, hi, frac)) in mirror.pairs.iter().enumerate() {
                let l = (mags[lo] as f64 + 1e-12).ln();
                let h = (mags[hi] as f64 * (1.0 - frac) + mags[hi + 1] as f64 * frac + 1e-12).ln();
                let dl = l - mean_l;
                let dh = h - mean_h;
                mean_l += dl / (i + 1) as f64;
                mean_h += dh / (i + 1) as f64;
                var_l += dl * (l - mean_l);
                var_h += dh * (h - mean_h);
                covariance += dl * (h - mean_h);
            }
            mirror.sum += covariance / ((var_l.max(0.0) * var_h.max(0.0)).sqrt() + 1e-12);
        }
    }

    pub fn finish(self, channel_index: usize) -> ResamplingAnalysis {
        let peak =
            self.sums.iter().copied().fold(0.0, f64::max) / self.active.max(1) as f64 + 1e-12;
        let db: Vec<_> = self
            .sums
            .iter()
            .map(|sum| 20.0 * (sum / self.active.max(1) as f64 / peak + 1e-12).log10())
            .collect();
        let bin_hz = self.rate as f64 / WINDOW as f64;
        let band = |lo: f64, hi: f64| -> Option<f64> {
            if self.active == 0 {
                return None;
            }
            let start = (lo.max(0.0) / bin_hz) as usize;
            let end = ((hi / bin_hz) as usize + 1).min(db.len());
            (end > start).then(|| db[start..end].iter().sum::<f64>() / (end - start) as f64)
        };
        let candidates: Vec<_> = self
            .mirrors
            .into_iter()
            .map(|mirror| {
                let nyquist = mirror.rate as f64 / 2.0;
                let edge = band(nyquist - 900.0, nyquist - 350.0);
                let notch = band(nyquist - 150.0, nyquist + 250.0);
                let above = band(
                    nyquist + 450.0,
                    (nyquist + 2000.0).min(self.rate as f64 / 2.0 - 200.0),
                );
                let ceiling = band(nyquist + 450.0, self.rate as f64 / 2.0 - 200.0);
                // Correlation of near-zero FFT noise is not meaningful. The
                // reference only evaluates mirrors when both bands are alive.
                let corr = (self.active > 0
                    && !mirror.pairs.is_empty()
                    && edge.is_some_and(|x| x >= -80.0)
                    && above.is_some_and(|x| x > -85.0))
                .then(|| mirror.sum / self.active as f64);
                let eligible = self.active >= MIN_FRAMES && edge.is_some_and(|x| x >= -80.0);
                let mut modes = vec![];
                if eligible {
                    let e = edge.unwrap();
                    let n = notch.unwrap();
                    let a = above.unwrap();
                    let c = ceiling.unwrap();
                    if e - n >= 20.0 && a - n >= 20.0 {
                        modes.push("notch".into());
                    }
                    if c < e - 40.0 && c < -90.0 {
                        modes.push("wall".into());
                    }
                    if a > -85.0 && corr.is_some_and(|x| x > 0.35) {
                        modes.push("mirror".into());
                    }
                }
                ResamplingCandidate {
                    source_rate: mirror.rate,
                    edge_relative_db: edge,
                    notch_relative_db: notch,
                    above_relative_db: above,
                    ceiling_relative_db: ceiling,
                    mirror_correlation: corr,
                    eligible,
                    modes,
                }
            })
            .collect();
        let status = if candidates.is_empty() {
            DetectorStatus::Unsupported
        } else if candidates.iter().any(|c| !c.modes.is_empty()) {
            DetectorStatus::Hit
        } else if candidates.iter().any(|c| c.eligible) {
            DetectorStatus::NotDetected
        } else {
            DetectorStatus::Inconclusive
        };
        ResamplingAnalysis {
            channel_index,
            status,
            active_frames: self.active,
            candidates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_spectra_have_finite_zero_mirror_correlation() {
        let mut stats = ResamplingStats::new(96000);
        for _ in 0..MIN_FRAMES {
            stats.push(&vec![1.0; WINDOW / 2 + 1]);
        }
        let result = stats.finish(0);
        assert_eq!(result.status, DetectorStatus::NotDetected);
        assert!(
            result
                .candidates
                .iter()
                .all(|c| c.mirror_correlation == Some(0.0))
        );
    }
    #[test]
    fn all_matching_rate_hypotheses_are_retained() {
        let rate = 192000;
        let mags: Vec<_> = (0..=WINDOW / 2)
            .map(|i| {
                let hz = i as f64 * rate as f64 / WINDOW as f64;
                if (hz - 22050.0).abs() < 350.0 || (hz - 24000.0).abs() < 350.0 {
                    1e-4
                } else {
                    1.0
                }
            })
            .collect();
        let mut stats = ResamplingStats::new(rate);
        for _ in 0..MIN_FRAMES {
            stats.push(&mags);
        }
        let result = stats.finish(0);
        for rate in [44100, 48000] {
            let c = result
                .candidates
                .iter()
                .find(|c| c.source_rate == rate)
                .unwrap();
            assert!(c.modes.iter().any(|m| m == "notch"));
        }
    }
}
