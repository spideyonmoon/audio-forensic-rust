//! Signed lag products of a centered high-band log-magnitude spectrum.
//! Descriptive periodicity measurements; no codec/source interpretation.
use crate::{
    dsp::{HOP, WINDOW},
    model::{AnalysisInterval, DetectorStatus, SpectralLag, SpectralLagAnalysis},
};

pub(crate) const MIN_FRAMES: u64 = 4;
pub(crate) const MIN_BINS: usize = 81;
pub(crate) const MIN_PAIRS: usize = 16;
pub(crate) const ABSOLUTE_AMPLITUDE: f64 = 1e-6;
pub(crate) const RELATIVE_AMPLITUDE: f64 = 1e-4;
pub(crate) const MIN_STD_DB: f64 = 0.1;

pub(crate) fn analyze(
    channel_index: usize,
    rate: u32,
    frames: u64,
    active: u64,
    magnitude_sum: &[f64],
) -> SpectralLagAnalysis {
    // Integer ratios avoid rounding ambiguity for exact band-edge bins.
    let start = (16000u64 * WINDOW as u64).div_ceil(u64::from(rate)) as usize;
    let end = (20000u64 * WINDOW as u64).div_ceil(u64::from(rate)) as usize;
    let complete = rate > 40000 && start > 0 && end <= WINDOW / 2;
    let bins = if complete { end - start } else { 0 };
    let supported = complete && bins >= MIN_BINS;
    let bin_hz = f64::from(rate) / WINDOW as f64;
    let window_sum: f64 = (0..WINDOW)
        .map(|i| {
            f64::from(
                (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (WINDOW - 1) as f64).cos()) as f32,
            )
        })
        .sum();
    let scale = 2.0 / window_sum / active.max(1) as f64;
    let reference = magnitude_sum.iter().copied().fold(0.0, f64::max) * scale;
    let floor = ABSOLUTE_AMPLITUDE.max(reference * RELATIVE_AMPLITUDE);
    let mut eligible_bins = 0;
    let mut minimum: Option<f64> = None;
    if complete && active > 0 {
        for &sum in &magnitude_sum[start..end] {
            let amplitude = sum * scale;
            minimum = Some(minimum.unwrap_or(amplitude).min(amplitude));
            eligible_bins += usize::from(amplitude > floor);
        }
    }
    let energy_eligible = supported && active >= MIN_FRAMES && eligible_bins == bins;
    let mut centered = Vec::new();
    let mut energy = 0.0;
    let mut std_db = None;
    if energy_eligible {
        centered = magnitude_sum[start..end]
            .iter()
            .map(|&sum| 20.0 * (sum * scale / reference).log10())
            .collect::<Vec<_>>();
        let mean = centered.iter().sum::<f64>() / bins as f64;
        for value in &mut centered {
            *value -= mean;
            energy += *value * *value;
        }
        std_db = Some((energy / bins as f64).sqrt());
    }
    let variable = std_db.is_some_and(|std| std > MIN_STD_DB);
    let lag_result = |lag: usize| {
        let pairs = bins.saturating_sub(lag);
        let geometry = supported && pairs >= MIN_PAIRS;
        SpectralLag {
            lag_bins: lag,
            lag_hz: lag as f64 * bin_hz,
            pair_count: pairs,
            status: if !geometry {
                DetectorStatus::Unsupported
            } else if variable {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            },
            coefficient: (geometry && variable).then(|| {
                centered[..pairs]
                    .iter()
                    .zip(&centered[lag..])
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    / energy
            }),
        }
    };
    // WINDOW/64 is exact: all three requested separations lie on integer bins.
    let lags = (1..=3)
        .map(|multiple| {
            let lag = multiple * (WINDOW / 64);
            crate::model::SpectralLagProbe {
                multiple,
                requested_lag_hz: multiple as f64 * f64::from(rate) / 64.0,
                target: lag_result(lag),
                neighbours: vec![lag_result(lag - 3), lag_result(lag + 3)],
            }
        })
        .collect::<Vec<_>>();
    SpectralLagAnalysis {
        channel_index,
        status: if !supported {
            DetectorStatus::Unsupported
        } else if variable {
            DetectorStatus::Measured
        } else {
            DetectorStatus::Inconclusive
        },
        interval: (frames > 0).then_some(AnalysisInterval {
            start_frame: 0,
            end_frame: frames.saturating_sub(1) * HOP as u64 + WINDOW as u64,
        }),
        stft_frames: frames,
        active_frames: active,
        requested_lower_hz: 16000.0,
        requested_upper_hz_exclusive: 20000.0,
        lower_bin_hz: complete.then_some(start as f64 * bin_hz),
        upper_bin_hz: complete.then_some(end.saturating_sub(1) as f64 * bin_hz),
        bin_count: bins,
        eligible_bins,
        reference_peak_amplitude: (active > 0).then_some(reference),
        minimum_bin_amplitude: minimum,
        log_magnitude_std_db: std_db,
        lags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spectrum(db: impl Fn(usize) -> f64) -> Vec<f64> {
        (0..=WINDOW / 2).map(|k| 10f64.powf(db(k) / 20.0)).collect()
    }

    #[test]
    fn exact_centered_cosine_products_and_neighbours() {
        // At 64 kHz the selected band is exactly 256 bins (four periods).
        let sums = spectrum(|k| 6.0 * (std::f64::consts::TAU * k as f64 / 64.0).cos());
        let r = analyze(0, 64000, 4, 4, &sums);
        assert_eq!(r.status, DetectorStatus::Measured);
        assert_eq!(r.bin_count, 256);
        assert!((r.log_magnitude_std_db.unwrap() - 6.0 / 2f64.sqrt()).abs() < 1e-12);
        for probe in &r.lags {
            let expected = 1.0 - probe.multiple as f64 / 4.0;
            assert!((probe.target.coefficient.unwrap() - expected).abs() < 1e-12);
            assert_eq!(probe.target.pair_count, 256 - probe.target.lag_bins);
            assert_eq!(probe.neighbours.len(), 2);
            assert_eq!(probe.target.lag_hz, probe.requested_lag_hz);
        }
    }

    #[test]
    fn flat_missing_energy_and_short_spectra_do_not_invent_coefficients() {
        let flat = spectrum(|_| 0.0);
        for (active, sums) in [
            (4, flat.clone()),
            (3, flat.clone()),
            (4, vec![0.0; flat.len()]),
        ] {
            let r = analyze(0, 48000, 4, active, &sums);
            assert_eq!(r.status, DetectorStatus::Inconclusive);
            assert!(r.lags.iter().all(|p| p.target.coefficient.is_none()));
        }
        let mut sparse = spectrum(|k| (k % 64) as f64 / 8.0);
        sparse[1500] = 0.0;
        let r = analyze(0, 48000, 4, 4, &sparse);
        assert_eq!(r.eligible_bins, r.bin_count - 1);
        assert_eq!(r.status, DetectorStatus::Inconclusive);
        assert_eq!(r.log_magnitude_std_db, None);
    }
}
