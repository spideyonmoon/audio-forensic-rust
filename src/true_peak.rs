//! Four-phase FIR estimate using BS.1770-5 Annex 2 coefficients.
use crate::model::{AnalysisInterval, DetectorStatus, TruePeakAnalysis};

// Columns in the published table are phases; rows run newest to oldest.
// Flattened by row this is a symmetric 48-tap interpolation FIR.
const COEFFICIENTS: [[f64; 4]; 12] = [
    [
        0.001708984375,
        -0.0291748046875,
        -0.0189208984375,
        -0.00830078125,
    ],
    [0.010986328125, 0.029296875, 0.0330810546875, 0.014892578125],
    [
        -0.0196533203125,
        -0.0517578125,
        -0.0582275390625,
        -0.026611328125,
    ],
    [0.033203125, 0.089111328125, 0.1015625, 0.047607421875],
    [
        -0.0594482421875,
        -0.16650390625,
        -0.2003173828125,
        -0.102294921875,
    ],
    [
        0.1373291015625,
        0.465087890625,
        0.77978515625,
        0.97216796875,
    ],
    [
        0.97216796875,
        0.77978515625,
        0.465087890625,
        0.1373291015625,
    ],
    [
        -0.102294921875,
        -0.2003173828125,
        -0.16650390625,
        -0.0594482421875,
    ],
    [0.047607421875, 0.1015625, 0.089111328125, 0.033203125],
    [
        -0.026611328125,
        -0.0582275390625,
        -0.0517578125,
        -0.0196533203125,
    ],
    [0.014892578125, 0.0330810546875, 0.029296875, 0.010986328125],
    [
        -0.00830078125,
        -0.0189208984375,
        -0.0291748046875,
        0.001708984375,
    ],
];

#[derive(Default)]
pub(crate) struct TruePeakMeter {
    history: [f64; 12],
    frames: u64,
    sample_peak: f64,
    interpolated_peak: f64,
}

impl TruePeakMeter {
    fn filter(&mut self, x: f64) {
        self.history.copy_within(0..11, 1);
        self.history[0] = x;
        let mut values = [0.; 4];
        for (sample, coefficients) in self.history.iter().zip(COEFFICIENTS) {
            for (value, coefficient) in values.iter_mut().zip(coefficients) {
                *value += sample * coefficient;
            }
        }
        for value in values {
            self.interpolated_peak = self.interpolated_peak.max(value.abs());
        }
    }

    pub(crate) fn push(&mut self, x: f64) {
        self.frames += 1;
        self.sample_peak = self.sample_peak.max(x.abs());
        self.filter(x);
    }

    pub(crate) fn finish(mut self, channel_index: usize, rate: u32) -> TruePeakAnalysis {
        // Finish the entire FIR response without reading any later real audio.
        for _ in 0..11 {
            self.filter(0.);
        }
        let peak = self.sample_peak.max(self.interpolated_peak);
        TruePeakAnalysis {
            channel_index, status: DetectorStatus::Measured,
            interval: AnalysisInterval { start_frame: 0, end_frame: self.frames },
            oversampling_factor: 4, oversampled_rate_hz: rate * 4,
            filter_taps: 48, zero_padding_frames: 11,
            sample_peak: self.sample_peak, interpolated_peak: self.interpolated_peak,
            estimated_peak: peak,
            estimated_peak_dbtp: (peak > 0.).then(|| 20. * peak.log10()),
            caveats: vec![
                "Four-times FIR interpolation estimates waveform peaks; it is not an exact continuous-time maximum or a clipping/audibility verdict.".into(),
                "The analyzed prefix is extended with zeros at both boundaries and the complete filter tail is measured. No audio beyond the shared prefix is read; boundary ringing can affect the result.".into(),
                "The estimate is the larger of the sample peak and FIR peak. Native channels and DC are preserved. Silence has zero linear peak and null dBTP.".into(),
                "The published 48 kHz filter is applied at every native rate with the same normalized response. Below 48 kHz the output rate is below the Annex 2 192 kHz recommendation. Certified meter conformance is unverified.".into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_fir_is_symmetric_and_flush_captures_last_sample() {
        let h: Vec<_> = COEFFICIENTS.into_iter().flatten().collect();
        assert!(h.iter().zip(h.iter().rev()).all(|(a, b)| a == b));
        let mut m = TruePeakMeter::default();
        m.push(0.5);
        let r = m.finish(0, 48000);
        assert_eq!(r.interpolated_peak, 0.5 * 0.97216796875);
        assert_eq!(r.estimated_peak, 0.5);
        assert_eq!(r.interval.end_frame, 1);
    }
}
