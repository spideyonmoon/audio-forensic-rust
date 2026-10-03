use crate::model::{AnalysisInterval, DetectorStatus, LoudnessRangeAnalysis};

const BIN: f64 = 0.01;
const BINS: usize = 12000;
const LOW: f64 = -70.;

pub(crate) struct RangeMeter {
    absolute_count: u64,
    absolute_mean: f64,
    gate: Option<f64>,
    bins: Vec<u64>,
    gated: u64,
    overflow: u64,
}

impl RangeMeter {
    pub(crate) fn new(gate: Option<f64>) -> Self {
        Self {
            absolute_count: 0,
            absolute_mean: 0.,
            gate,
            bins: if gate.is_some() {
                vec![0; BINS]
            } else {
                vec![]
            },
            gated: 0,
            overflow: 0,
        }
    }

    pub(crate) fn relative_gate(&self) -> Option<f64> {
        (self.absolute_count > 0).then_some(self.absolute_mean * 0.01)
    }

    pub(crate) fn push(&mut self, power: f64) {
        // Tech 3342 uses inclusive gates, unlike integrated LUFS.
        if power < 10f64.powf((LOW + 0.691) / 10.) {
            return;
        }
        self.absolute_count += 1;
        self.absolute_mean += (power - self.absolute_mean) / self.absolute_count as f64;
        if !self.gate.is_some_and(|gate| power >= gate) {
            return;
        }
        self.gated += 1;
        let loudness = -0.691 + 10. * power.log10();
        let index = ((loudness - LOW) / BIN).floor() as usize;
        if let Some(bin) = self.bins.get_mut(index) {
            *bin += 1;
        } else {
            self.overflow += 1;
        }
    }

    fn percentile(&self, percent: u128) -> f64 {
        // Nearest order statistic, half upward, as in the reference MATLAB.
        let rank = ((u128::from(self.gated - 1) * percent + 50) / 100) as u64;
        let mut count = 0;
        for (i, n) in self.bins.iter().enumerate() {
            count += n;
            if count > rank {
                return LOW + (i as f64 + 0.5) * BIN;
            }
        }
        unreachable!("rank is within the non-overflow histogram")
    }

    pub(crate) fn finish(
        self,
        supported: bool,
        interval: Option<AnalysisInterval>,
    ) -> LoudnessRangeAnalysis {
        let measured = supported && self.gated >= 2 && self.overflow == 0;
        let p10 = measured.then(|| self.percentile(10));
        let p95 = measured.then(|| self.percentile(95));
        let range = p10.zip(p95).map(|(low, high)| high - low);
        LoudnessRangeAnalysis {
            status: if !supported { DetectorStatus::Unsupported } else if measured { DetectorStatus::Measured }
                else { DetectorStatus::Inconclusive }, interval,
            absolute_gated_windows: self.absolute_count, relative_gated_windows: self.gated,
            relative_gate_lufs: self.gate.map(|p| -0.691 + 10. * p.log10()), histogram_bin_width_lu: BIN,
            histogram_overflow_windows: self.overflow, p10_lufs: p10, p95_lufs: p95, range_lu: range,
            range_lower_lu: range.map(|v| (v - BIN).max(0.)), range_upper_lu: range.map(|v| v + BIN),
            caveats: vec![
                "Complete 3 s windows every 100 ms; inclusive -70 LUFS and -20 LU relative gates; nearest-rank 10th/95th percentiles.".into(),
                "Gates use unquantized powers in two passes. A fixed 0.01 LU histogram approximates percentiles; bounds cover histogram quantization, not perceptual uncertainty.".into(),
                "No synthetic silence is appended, unlike the Tech 3342 file-meter tail recommendation. Coverage follows complete windows inside the shared prefix; certified EBU meter conformance is unverified.".into(),
                "Fewer than two gated windows or any histogram overflow abstains. Range is descriptive, not a DR score, compression diagnosis or mastering-quality grade.".into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inclusive_gates_and_histogram_overflow_are_explicit() {
        let threshold = 10f64.powf((-70. + 0.691) / 10.);
        let mut r = RangeMeter::new(Some(threshold));
        r.push(threshold);
        r.push(threshold);
        let a = r.finish(true, None);
        assert_eq!(a.absolute_gated_windows, 2);
        assert_eq!(a.relative_gated_windows, 2);
        assert_eq!(a.range_lu, Some(0.));
        let mut r = RangeMeter::new(Some(threshold));
        r.push(1e6);
        r.push(1.);
        let a = r.finish(true, None);
        assert_eq!(a.histogram_overflow_windows, 1);
        assert_eq!(a.status, DetectorStatus::Inconclusive);
        assert_eq!(a.range_lu, None);
    }
}
