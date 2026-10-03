//! Scaled online centered statistics for simultaneous native-channel PCM pairs.
use crate::model::{AnalysisInterval, DetectorStatus, StereoCorrelationAnalysis};

pub(crate) const MIN_PAIRS: u64 = 2;
pub(crate) const MIN_STD: f64 = 1e-8;
pub(crate) const MIN_RELATIVE_STD: f64 = 1e-6;

#[derive(Default)]
pub(crate) struct StereoStats {
    count: u64,
    scale: [f64; 2],
    mean: [f64; 2],
    m2: [f64; 2],
    covariance: f64,
}

impl StereoStats {
    pub fn push(&mut self, left: f64, right: f64) {
        let samples = [left, right];
        let mut factors = [1.0; 2];
        for i in 0..2 {
            if samples[i].abs() > self.scale[i] {
                factors[i] = self.scale[i] / samples[i].abs();
                self.scale[i] = samples[i].abs();
                self.mean[i] *= factors[i];
                self.m2[i] *= factors[i] * factors[i];
            }
        }
        self.covariance *= factors[0] * factors[1];
        self.count += 1;
        let values = [0, 1].map(|i| {
            if self.scale[i] > 0.0 {
                samples[i] / self.scale[i]
            } else {
                0.0
            }
        });
        let delta = [values[0] - self.mean[0], values[1] - self.mean[1]];
        for i in 0..2 {
            self.mean[i] += delta[i] / self.count as f64;
            self.m2[i] += delta[i] * (values[i] - self.mean[i]);
        }
        self.covariance += delta[0] * (values[1] - self.mean[1]);
    }

    pub fn finish(self, channels: usize) -> StereoCorrelationAnalysis {
        let data = channels == 2 && self.count > 0;
        let std = [0, 1].map(|i| (self.m2[i].max(0.0) / self.count.max(1) as f64).sqrt());
        let eligible =
            [0, 1].map(|i| data && self.scale[i] * std[i] > MIN_STD && std[i] > MIN_RELATIVE_STD);
        let measured = self.count >= MIN_PAIRS && eligible.iter().all(|&v| v);
        StereoCorrelationAnalysis {
            status: if channels != 2 { DetectorStatus::Unsupported } else if measured { DetectorStatus::Measured } else { DetectorStatus::Inconclusive },
            channel_indices: if channels == 2 { vec![0, 1] } else { vec![] },
            interval: data.then_some(AnalysisInterval { start_frame: 0, end_frame: self.count }),
            pair_count: self.count,
            mean: [0, 1].map(|i| data.then_some(self.scale[i] * self.mean[i])),
            std: [0, 1].map(|i| data.then_some(self.scale[i] * std[i])),
            variation_eligible: eligible,
            coefficient: measured.then(|| (self.covariance / self.m2[0].sqrt() / self.m2[1].sqrt()).clamp(-1.0, 1.0)),
            caveats: vec!["Signed zero-lag Pearson correlation of centered simultaneous native-channel PCM pairs over the shared prefix, including silence and DC. No downmix, lag search, filtering, activity selection or channel alignment is applied.".into(),
                "Each channel must have standard deviation above 1e-8 full-scale amplitude and 1e-6 of its own sample peak. These are numerical applicability gates, not calibrated stereo-quality thresholds. At least two pairs are required.".into(),
                "Correlation depends on content, phase, delay, gain and channel processing. It does not identify fake stereo, lossy processing, authenticity or mastering quality.".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_centered_covariance_tracks_changing_peaks_and_dc() {
        let pairs = [
            [0.0, 0.0],
            [0.01, -0.02],
            [0.4, -0.8],
            [-0.2, 0.4],
            [1.0, -2.0],
        ];
        let mut s = StereoStats::default();
        for [x, y] in pairs {
            s.push(x + 0.5, y - 0.25);
        }
        let r = s.finish(2);
        assert_eq!(r.status, DetectorStatus::Measured);
        assert!((r.coefficient.unwrap() + 1.0).abs() < 1e-14);
        let mean = pairs.iter().map(|p| p[0] + 0.5).sum::<f64>() / pairs.len() as f64;
        let std = (pairs
            .iter()
            .map(|p| (p[0] + 0.5 - mean).powi(2))
            .sum::<f64>()
            / pairs.len() as f64)
            .sqrt();
        assert!((r.mean[0].unwrap() - mean).abs() < 1e-14);
        assert!((r.std[0].unwrap() - std).abs() < 1e-14);
    }

    #[test]
    fn tiny_nonzero_variation_and_constant_channels_abstain_explicitly() {
        let mut s = StereoStats::default();
        for i in 0..100 {
            s.push(if i % 2 == 0 { 1e-200 } else { -1e-200 }, 0.5);
        }
        let r = s.finish(2);
        assert_eq!(r.status, DetectorStatus::Inconclusive);
        assert_eq!(r.std[0], Some(1e-200));
        assert_eq!(r.std[1], Some(0.0));
        assert_eq!(r.coefficient, None);
    }
}
