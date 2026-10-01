//! Streaming spectral scatter and high-band phase measurements, without source labels.
use crate::{
    dsp::{HOP, WINDOW},
    model::{AnalysisInterval, DetectorStatus, SpectralStructureAnalysis},
};
use rustfft::num_complex::Complex32;

const BINS: usize = WINDOW / 2 + 1;
pub(crate) const MIN_FRAMES: u64 = 4;
pub(crate) const MIN_SCATTER: f64 = 1e-6;
pub(crate) const PHASE_RELATIVE_MAG: f64 = 1e-4;
pub(crate) const PHASE_ABSOLUTE_MAG: f64 = 1e-8;
pub(crate) const PHASE_HIST_BINS: usize = 36;

pub(crate) struct StructureStats {
    rate: u32,
    frames: u64,
    active: u64,
    bounds: Vec<u64>,
    flat: u64,
    log_power: Vec<f64>,
    scatter: Vec<f64>,
    phase_start: usize,
    previous_phase: Vec<f64>,
    previous_usable: Vec<bool>,
    previous_active: bool,
    phase_histogram: [u64; PHASE_HIST_BINS],
    phase_frame_pairs: u64,
}

impl StructureStats {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            frames: 0,
            active: 0,
            bounds: vec![0; BINS],
            flat: 0,
            log_power: vec![0.0; BINS],
            scatter: vec![0.0; BINS],
            // Actual frequencies >=10 kHz, excluding the real-valued Nyquist bin.
            phase_start: (10000 * WINDOW).div_ceil(rate as usize).min(WINDOW / 2),
            previous_phase: vec![0.0; BINS],
            previous_usable: vec![false; BINS],
            previous_active: false,
            phase_histogram: [0; PHASE_HIST_BINS],
            phase_frame_pairs: 0,
        }
    }

    pub fn push(&mut self, magnitudes: &[f32], spectrum: &[Complex32], active: bool) {
        self.frames += 1;
        if !active {
            // Never compare across an inactive gap as if it were one hop.
            self.previous_active = false;
            return;
        }
        self.active += 1;
        let peak = magnitudes.iter().copied().fold(0.0f32, f32::max) as f64;
        if let Some(bound) = self.scatter_bound(magnitudes, peak) {
            self.bounds[bound] += 1;
        } else {
            self.flat += 1;
        }
        self.accumulate_phase(magnitudes, spectrum, peak);
    }

    fn scatter_bound(&mut self, magnitudes: &[f32], peak: f64) -> Option<usize> {
        for (value, &mag) in self.log_power.iter_mut().zip(magnitudes) {
            let db = (20.0 * (f64::from(mag) / (peak + 1e-12) + 1e-12).log10()).max(-110.0);
            *value = db / 10.0 * std::f64::consts::LN_10;
        }
        for (i, value) in self.scatter.iter_mut().enumerate() {
            let local: [f64; 5] =
                std::array::from_fn(|j| self.log_power[(i + j).saturating_sub(2).min(BINS - 1)]);
            let mean = local.iter().sum::<f64>() / 5.0;
            // Centered f64 differences avoid cancellation on the clamped floor.
            *value = (local.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 5.0).sqrt();
        }
        // Reuse the consumed log-power buffer for the second five-bin average.
        for (i, value) in self.log_power.iter_mut().enumerate() {
            *value = (0..5)
                .map(|j| self.scatter[(i + j).saturating_sub(2).min(BINS - 1)])
                .sum::<f64>()
                / 5.0;
        }
        let maximum = self.log_power.iter().copied().fold(0.0f64, f64::max);
        if maximum <= MIN_SCATTER {
            return None;
        }
        let threshold = 0.6f64.min(maximum * 0.25);
        self.log_power.iter().rposition(|&x| x >= threshold)
    }

    fn accumulate_phase(&mut self, magnitudes: &[f32], spectrum: &[Complex32], peak: f64) {
        let floor = PHASE_ABSOLUTE_MAG.max(peak * PHASE_RELATIVE_MAG);
        let mut histogram = [0u64; PHASE_HIST_BINS];
        let mut count = 0;
        for i in self.phase_start..WINDOW / 2 {
            let usable = f64::from(magnitudes[i]) > floor;
            let phase = f64::from(spectrum[i].im).atan2(f64::from(spectrum[i].re));
            if self.previous_active && usable && self.previous_usable[i] {
                let mut delta = phase - self.previous_phase[i];
                if delta > std::f64::consts::PI {
                    delta -= std::f64::consts::TAU;
                }
                if delta < -std::f64::consts::PI {
                    delta += std::f64::consts::TAU;
                }
                let bin = (((delta + std::f64::consts::PI) / std::f64::consts::TAU
                    * PHASE_HIST_BINS as f64) as usize)
                    .min(PHASE_HIST_BINS - 1);
                histogram[bin] += 1;
                count += 1;
            }
            self.previous_phase[i] = phase;
            self.previous_usable[i] = usable;
        }
        if count >= 2 {
            self.phase_frame_pairs += 1;
            for (total, value) in self.phase_histogram.iter_mut().zip(histogram) {
                *total += value;
            }
        }
        self.previous_active = true;
    }

    pub fn finish(self, channel_index: usize) -> SpectralStructureAnalysis {
        let bound_frames: u64 = self.bounds.iter().sum();
        let bin_hz = f64::from(self.rate) / WINDOW as f64;
        let measured = bound_frames >= MIN_FRAMES;
        let average = measured.then(|| {
            self.bounds
                .iter()
                .enumerate()
                .map(|(i, &count)| i as f64 * count as f64)
                .sum::<f64>()
                / bound_frames as f64
                * bin_hz
        });
        // Exact FFT-bin mode, choosing the lowest bin in a tie. No data-dependent
        // histogram centers or claim that this is a probabilistic source bound.
        let mode = measured.then(|| {
            let max_count = self.bounds.iter().copied().max().unwrap();
            self.bounds.iter().position(|&n| n == max_count).unwrap() as f64 * bin_hz
        });
        let phase_samples: u64 = self.phase_histogram.iter().sum();
        let phase_supported = WINDOW / 2 - self.phase_start >= 2;
        let phase_measured = phase_supported && self.phase_frame_pairs >= 3;
        let entropy = phase_measured.then(|| {
            self.phase_histogram
                .iter()
                .filter(|&&n| n > 0)
                .map(|&n| {
                    let p = n as f64 / phase_samples as f64;
                    -p * p.log2()
                })
                .sum()
        });
        SpectralStructureAnalysis {
            channel_index,
            scatter_status: if measured {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            phase_status: if !phase_supported {
                DetectorStatus::Unsupported
            } else if phase_measured {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            interval: (self.frames > 0).then_some(AnalysisInterval {
                start_frame: 0,
                end_frame: self.frames.saturating_sub(1) * HOP as u64 + WINDOW as u64,
            }),
            stft_frames: self.frames,
            active_frames: self.active,
            bound_frames,
            flat_frames: self.flat,
            average_scatter_bound_hz: average,
            modal_scatter_bound_hz: mode,
            phase_band_start_hz: phase_supported.then_some(self.phase_start as f64 * bin_hz),
            phase_frame_pairs: self.phase_frame_pairs,
            phase_bin_pairs: phase_samples,
            high_band_phase_entropy_bits: entropy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spectrum(magnitudes: &[f32], phase: f32) -> Vec<Complex32> {
        magnitudes
            .iter()
            .map(|&m| Complex32::from_polar(m, phase))
            .collect()
    }

    #[test]
    fn flat_spectra_abstain_instead_of_inventing_a_nyquist_bound() {
        let mags = vec![1.0; BINS];
        let mut stats = StructureStats::new(48000);
        for i in 0..4 {
            stats.push(&mags, &spectrum(&mags, i as f32 * 0.3), true);
        }
        let report = stats.finish(0);
        assert_eq!(report.flat_frames, 4);
        assert_eq!(report.bound_frames, 0);
        assert_eq!(report.scatter_status, DetectorStatus::Inconclusive);
        assert_eq!(report.average_scatter_bound_hz, None);
        assert_eq!(report.modal_scatter_bound_hz, None);
        // A deterministic non-boundary phase increment has one occupied bin.
        assert_eq!(report.phase_status, DetectorStatus::Measured);
        assert_eq!(report.high_band_phase_entropy_bits, Some(0.0));
    }

    #[test]
    fn phase_pairs_never_bridge_inactive_gaps_or_include_empty_bins() {
        let mut mags = vec![1.0; BINS];
        let mut stats = StructureStats::new(48000);
        for i in 0..9 {
            stats.push(&mags, &spectrum(&mags, i as f32 * 0.3), i != 4);
        }
        let report = stats.finish(0);
        assert_eq!(report.phase_frame_pairs, 6);
        assert_eq!(report.phase_bin_pairs, 6 * (2048 - 854));
        assert_eq!(report.high_band_phase_entropy_bits, Some(0.0));

        mags.fill(1e-12);
        mags[100] = 1.0;
        let mut stats = StructureStats::new(48000);
        for i in 0..10 {
            stats.push(&mags, &spectrum(&mags, i as f32), true);
        }
        let report = stats.finish(0);
        assert_eq!(report.phase_frame_pairs, 0);
        assert_eq!(report.phase_bin_pairs, 0);
        assert_eq!(report.phase_status, DetectorStatus::Inconclusive);
        assert_eq!(report.high_band_phase_entropy_bits, None);
    }

    #[test]
    fn phase_band_energy_must_be_shared_and_excludes_nyquist() {
        let mut stats = StructureStats::new(48000);
        for i in 0..8 {
            let mut mags = vec![1e-12; BINS];
            mags[100] = 1.0;
            mags[900 + i % 2] = 1.0; // No shared usable high-frequency bin.
            mags[2048] = 1.0; // Nyquist's real coefficient must not vote.
            stats.push(&mags, &spectrum(&mags, i as f32), true);
        }
        assert_eq!(stats.finish(0).phase_bin_pairs, 0);
        let mut stats = StructureStats::new(16000);
        let mags = vec![1.0; BINS];
        for i in 0..8 {
            stats.push(&mags, &spectrum(&mags, i as f32), true);
        }
        let report = stats.finish(0);
        assert_eq!(report.phase_status, DetectorStatus::Unsupported);
        assert_eq!(report.phase_band_start_hz, None);
        assert_eq!(report.high_band_phase_entropy_bits, None);
    }

    #[test]
    fn all_frames_contribute_past_reference_subsampling_boundary() {
        let mags: Vec<_> = (0..BINS).map(|i| 0.1 + (i % 7) as f32 * 0.01).collect();
        let complex = spectrum(&mags, 0.3);
        let mut stats = StructureStats::new(8000);
        for _ in 0..2501 {
            stats.push(&mags, &complex, true);
        }
        assert_eq!(stats.bounds.len(), BINS);
        assert_eq!(stats.previous_phase.len(), BINS);
        assert_eq!(stats.phase_histogram.len(), 36);
        let report = stats.finish(0);
        assert_eq!(report.stft_frames, 2501);
        assert_eq!(report.bound_frames, 2501);
        assert_eq!(report.scatter_status, DetectorStatus::Measured);
        assert_eq!(
            report.interval.unwrap().end_frame,
            2500 * HOP as u64 + WINDOW as u64
        );
    }
}
