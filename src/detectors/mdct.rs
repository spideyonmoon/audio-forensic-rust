//! Princen-Bradley fold followed by an orthonormal DCT-IV, using a 2N complex FFT.
//! The phase factors implement the cosine definition; no new runtime dependency.
use rustfft::{Fft, FftPlanner, num_complex::Complex64};
use std::sync::Arc;

pub(crate) struct Mdct {
    window: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
    before: Vec<Complex64>,
    after: Vec<Complex64>,
    buffer: Vec<Complex64>,
    scratch: Vec<Complex64>,
    coefficients: Vec<f64>,
}

impl Mdct {
    pub fn new(window: Vec<f64>) -> Self {
        let n = window.len() / 2;
        assert!(n > 0 && n % 2 == 0);
        let fft = FftPlanner::new().plan_fft_forward(2 * n);
        let scratch = vec![Complex64::default(); fft.get_inplace_scratch_len()];
        let phase = |x: f64| Complex64::from_polar(1.0, -std::f64::consts::PI * x / (2 * n) as f64);
        Self {
            window,
            fft,
            before: (0..n).map(|i| phase(i as f64)).collect(),
            after: (0..n)
                .map(|i| phase(i as f64 + 0.5) * (2.0 / n as f64).sqrt())
                .collect(),
            buffer: vec![Complex64::default(); 2 * n],
            scratch,
            coefficients: vec![0.0; n],
        }
    }

    pub fn process(&mut self, samples: &[f64]) -> &[f64] {
        let n = self.coefficients.len();
        assert_eq!(samples.len(), n * 2);
        let x = |i: usize| samples[i] * self.window[i];
        self.buffer.fill(Complex64::default());
        for i in 0..n / 2 {
            self.buffer[i] = self.before[i] * (-x(n + n / 2 - 1 - i) - x(n + n / 2 + i));
            self.buffer[n / 2 + i] = self.before[n / 2 + i] * (x(i) - x(n - 1 - i));
        }
        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        for (i, value) in self.coefficients.iter_mut().enumerate() {
            *value = (self.buffer[i] * self.after[i]).re;
        }
        &self.coefficients
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fft_mdct_matches_the_cosine_definition() {
        for n in [4, 16, 64] {
            let window: Vec<_> = (0..n * 2)
                .map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / (2 * n) as f64).sin())
                .collect();
            let samples: Vec<_> = (0..n * 2)
                .map(|i| (i as f64 * 1.37).sin() * 0.3 + (i as f64 * 0.21).cos())
                .collect();
            let mut mdct = Mdct::new(window.clone());
            let actual = mdct.process(&samples);
            for (k, a) in actual.iter().enumerate() {
                let direct: f64 = (0..2 * n)
                    .map(|i| {
                        samples[i]
                            * window[i]
                            * (std::f64::consts::PI / n as f64
                                * (i as f64 + 0.5 + n as f64 / 2.0)
                                * (k as f64 + 0.5))
                                .cos()
                    })
                    .sum();
                assert!(
                    (a - direct * (2.0 / n as f64).sqrt()).abs() < 1e-11,
                    "N={n}, k={k}"
                );
            }
        }
    }
}
