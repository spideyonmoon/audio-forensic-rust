//! BS.1770 programme energy, with exact two-pass gating and fixed storage.
//! Only rates with integral 100 ms hops are supported; no rounded time grid.
use crate::model::{AnalysisInterval, DetectorStatus, LoudnessAnalysis};

const OFFSET: f64 = -0.691;
pub(crate) const ABSOLUTE_GATE_LUFS: f64 = -70.0;

#[derive(Clone)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn push(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

fn filters(rate: u32) -> [Biquad; 2] {
    // Bilinear K-weighting parameterization used by libebur128. At 48 kHz
    // these reproduce the two stages in BS.1770 Annex 1, Tables 1 and 2.
    let k = (std::f64::consts::PI * 1681.974450955533 / rate as f64).tan();
    let q = 0.7071752369554196;
    let vh = 10f64.powf(3.999843853973347 / 20.);
    let vb = vh.powf(0.4996667741545416);
    let d = 1. + k / q + k * k;
    let shelf = Biquad {
        b: [
            (vh + vb * k / q + k * k) / d,
            2. * (k * k - vh) / d,
            (vh - vb * k / q + k * k) / d,
        ],
        a: [2. * (k * k - 1.) / d, (1. - k / q + k * k) / d],
        z: [0.; 2],
    };
    let k = (std::f64::consts::PI * 38.13547087602444 / rate as f64).tan();
    let q = 0.5003270373238773;
    let d = 1. + k / q + k * k;
    let highpass = Biquad {
        b: [1., -2., 1.],
        a: [2. * (k * k - 1.) / d, (1. - k / q + k * k) / d],
        z: [0.; 2],
    };
    [shelf, highpass]
}

fn level(power: f64) -> Option<f64> {
    (power > 0.).then(|| OFFSET + 10. * power.log10())
}

#[derive(Default)]
struct Mean {
    n: u64,
    value: f64,
}

impl Mean {
    fn push(&mut self, x: f64) {
        self.n += 1;
        self.value += (x - self.value) / self.n as f64;
    }
}

pub(crate) struct LoudnessMeter {
    rate: u32,
    filters: Vec<[Biquad; 2]>,
    hop: u64,
    frames: u64,
    chunk_energy: f64,
    history: [f64; 30],
    chunks: u64,
    absolute: Mean,
    gated: Mean,
    relative_gate: Option<f64>,
    momentary_max: f64,
    short_term_max: f64,
    range: crate::loudness_range::RangeMeter,
}

impl LoudnessMeter {
    pub(crate) fn new(
        rate: u32,
        channels: usize,
        relative_gate: Option<f64>,
        range_gate: Option<f64>,
    ) -> Self {
        Self {
            rate,
            filters: vec![filters(rate); channels],
            hop: u64::from(rate / 10),
            frames: 0,
            chunk_energy: 0.,
            history: [0.; 30],
            chunks: 0,
            absolute: Mean::default(),
            gated: Mean::default(),
            relative_gate,
            momentary_max: 0.,
            short_term_max: 0.,
            range: crate::loudness_range::RangeMeter::new(range_gate),
        }
    }

    pub(crate) fn relative_gate(&self) -> Option<f64> {
        (self.absolute.n > 0).then_some(self.absolute.value * 0.1)
    }

    pub(crate) fn range_gate(&self) -> Option<f64> {
        self.range.relative_gate()
    }

    // The decoder calls this in interleaved channel order, once per sample.
    pub(crate) fn push(&mut self, channel: usize, x: f64) {
        if self.rate % 10 != 0 {
            return;
        }
        let [shelf, highpass] = &mut self.filters[channel];
        let y = highpass.push(shelf.push(x));
        // Sum independently filtered channel powers, never a mono downmix.
        self.chunk_energy += y * y;
        if channel + 1 != self.filters.len() {
            return;
        }
        self.frames += 1;
        if self.frames % self.hop != 0 {
            return;
        }
        self.history[(self.chunks % 30) as usize] = self.chunk_energy / self.hop as f64;
        self.chunk_energy = 0.;
        self.chunks += 1;
        if self.chunks >= 4 {
            // Re-sum only four positive chunks, avoiding running-window drift
            // or cancellation after loud content followed by long silence.
            let power = (0..4)
                .map(|i| self.history[((self.chunks - 1 - i) % 30) as usize])
                .sum::<f64>()
                / 4.;
            self.momentary_max = self.momentary_max.max(power);
            if power > 10f64.powf((ABSOLUTE_GATE_LUFS - OFFSET) / 10.) {
                self.absolute.push(power);
                if self.relative_gate.is_some_and(|gate| power > gate) {
                    self.gated.push(power);
                }
            }
        }
        if self.chunks >= 30 {
            let power = self.history.iter().sum::<f64>() / 30.;
            self.short_term_max = self.short_term_max.max(power);
            self.range.push(power);
        }
    }

    pub(crate) fn finish(self, analyzed_frames: u64) -> LoudnessAnalysis {
        let supported = self.rate % 10 == 0;
        let complete = self.chunks.saturating_sub(3);
        let end = if complete > 0 {
            self.chunks * self.hop
        } else {
            0
        };
        let integrated = (self.gated.n > 0).then(|| level(self.gated.value).unwrap());
        LoudnessAnalysis {
            status: if !supported { DetectorStatus::Unsupported }
                else if integrated.is_some() { DetectorStatus::Measured }
                else { DetectorStatus::Inconclusive },
            channel_weights: vec![1.; self.filters.len()],
            analyzed_frames,
            interval: (end > 0).then_some(AnalysisInterval { start_frame: 0, end_frame: end }),
            hop_frames: supported.then_some(self.hop),
            block_frames: supported.then_some(4 * self.hop),
            trailing_frames: analyzed_frames - end,
            complete_blocks: complete,
            absolute_gated_blocks: self.absolute.n,
            relative_gated_blocks: self.gated.n,
            relative_gate_lufs: self.relative_gate.and_then(level),
            integrated_lufs: integrated,
            momentary_max_lufs: level(self.momentary_max),
            short_term_max_lufs: level(self.short_term_max),
            short_term_windows: self.chunks.saturating_sub(29),
            range: self.range.finish(supported, (self.chunks >= 30).then_some(AnalysisInterval { start_frame: 0, end_frame: end })),
            caveats: vec![
                "Programme power sums independently K-weighted native channels with unit weights; mono has no dual-mono correction.".into(),
                "Only complete windows on a 100 ms grid contribute. Maxima are grid-sampled, not continuous-time maxima; filter state starts at zero at frame zero.".into(),
                "Rates not divisible by ten are unsupported by this loudness measurement; other analysis remains available. Silence and insufficient coverage produce null levels.".into(),
                "Loudness is a listening-level measurement, not source history or a mastering-quality verdict. True peak and range have separate reports; certified meter conformance is unverified.".into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_match_itu_48khz_tables() {
        let [shelf, highpass] = filters(48000);
        for (a, b) in shelf
            .b
            .iter()
            .zip([1.53512485958697, -2.69169618940638, 1.19839281085285])
        {
            assert!((a - b).abs() < 1e-13);
        }
        for (a, b) in shelf.a.iter().zip([-1.69065929318241, 0.73248077421585]) {
            assert!((a - b).abs() < 1e-13);
        }
        for (a, b) in highpass.a.iter().zip([-1.99004745483398, 0.99007225036621]) {
            assert!((a - b).abs() < 1e-13);
        }
        assert_eq!(highpass.b, [1., -2., 1.]);
    }
}
