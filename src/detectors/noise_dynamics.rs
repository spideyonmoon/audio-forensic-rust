//! Periodic band-limited autocorrelation and bounded temporal band-power history.
use crate::{
    dsp::WINDOW,
    model::{
        AnalysisInterval, BandCorrelation, BandPowerBlock, BandTemporalVariation, DetectorStatus,
    },
};
use std::ops::RangeInclusive;

pub(crate) fn append_observations(report: &mut crate::model::AnalysisReport) {
    use crate::model::DetectorResult;
    for n in &report.noise {
        for (id, band) in [
            ("high_band", &n.high_band),
            ("above_cutoff_band", &n.above_cutoff_band),
        ] {
            for c in &band.correlations {
                report.detectors.push(DetectorResult {
                    id: format!("{id}_circular_correlation_lag_{}", c.lag_frames),
                    version: 1, family: "noise_measurements".into(), status: c.status.clone(),
                    channel_index: Some(n.channel_index), intervals: n.stft_interval.iter().cloned().collect(),
                    measurements: c.coefficient.map(|v| [("coefficient".into(), v)].into()).unwrap_or_default(),
                    thresholds: [("lag_samples".into(), c.lag_frames as f64),
                        ("minimum_stft_frames".into(), super::noise::MIN_BAND_FRAMES as f64),
                        ("absolute_mean_square_floor_exclusive".into(), MIN_POWER),
                        ("relative_broadband_power_floor_exclusive".into(), RELATIVE_POWER_FLOOR)].into(),
                    caveats: vec!["Signed circular autocorrelation of band-masked Hann windows, weighted by window energy. Wraparound, windowing and band limits affect it; it is not full-stream Pearson correlation or a source-medium label.".into()],
                });
            }
            let t = &band.temporal_variation;
            report.detectors.push(DetectorResult {
                id: format!("{id}_temporal_variation"), version: 1, family: "noise_measurements".into(),
                status: t.status.clone(), channel_index: Some(n.channel_index),
                intervals: t.blocks.iter().map(|b| b.interval.clone()).collect(),
                measurements: t.level_std_db.map(|v| [("level_std_db".into(), v)].into()).unwrap_or_default(),
                thresholds: [("maximum_seconds".into(), CAP_SECONDS as f64),
                    ("minimum_complete_blocks".into(), 2.0),
                    ("absolute_mean_square_floor_exclusive".into(), MIN_POWER),
                    ("relative_broadband_power_floor_exclusive".into(), RELATIVE_POWER_FLOOR)].into(),
                caveats: vec!["Population standard deviation of one-second band-power levels. Only windows wholly within completed seconds of the capped prefix contribute. Every included block must exceed both energy floors; silence/quiet blocks are not dropped to create an artificially stable result.".into(),
                    "Block powers can contain music, noise and spectral leakage; stationarity does not confirm vinyl, cassette or any source history.".into()],
            });
        }
    }
}

pub(crate) const CAP_SECONDS: usize = 180;
pub(crate) const MIN_POWER: f64 = 1e-12;
pub(crate) const RELATIVE_POWER_FLOOR: f64 = 1e-8;
const BINS: usize = WINDOW / 2 + 1;

struct SecondPower {
    sums: Vec<f64>,
    frames: u64,
    start: u64,
    end: u64,
}

pub(crate) struct TemporalSpectra {
    rate: u32,
    seconds: Vec<SecondPower>,
}

fn broadband_power(sums: &[f64], frames: u64, normalization: f64) -> f64 {
    if frames == 0 {
        return 0.0;
    }
    // DC and Nyquist do not receive the one-sided doubling of interior bins.
    (sums[1..BINS - 1].iter().sum::<f64>() + 0.5 * (sums[0] + sums[BINS - 1])) * normalization
        / frames as f64
}

fn eligible(power: f64, broadband: f64) -> bool {
    power > MIN_POWER.max(broadband * RELATIVE_POWER_FLOOR)
}

impl TemporalSpectra {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            seconds: Vec::new(),
        }
    }

    /// Only windows fully inside one of the first 180 one-second blocks enter.
    /// Frame completion still follows the shared STFT's strict EOF convention.
    pub fn frame(&mut self, end: u64) -> Option<&mut [f64]> {
        let start = end - WINDOW as u64;
        let second = start / u64::from(self.rate);
        if second >= CAP_SECONDS as u64 || end > (second + 1) * u64::from(self.rate) {
            return None;
        }
        while self.seconds.len() <= second as usize {
            self.seconds.push(SecondPower {
                sums: vec![0.0; BINS],
                frames: 0,
                start: 0,
                end: 0,
            });
        }
        let block = &mut self.seconds[second as usize];
        if block.frames == 0 {
            block.start = start;
        }
        block.frames += 1;
        block.end = end;
        Some(&mut block.sums)
    }

    pub fn finish_band(
        &self,
        range: Option<RangeInclusive<usize>>,
        all_power: &[f64],
        frames: u64,
        samples: u64,
        normalization: f64,
    ) -> (Vec<BandCorrelation>, BandTemporalVariation) {
        let total: f64 = range
            .clone()
            .map(|r| all_power[r].iter().sum())
            .unwrap_or(0.0);
        let mean = if frames > 0 {
            total * normalization / frames as f64
        } else {
            0.0
        };
        let usable = frames >= super::noise::MIN_BAND_FRAMES
            && eligible(mean, broadband_power(all_power, frames, normalization));
        let status = |sufficient| {
            if range.is_none() {
                DetectorStatus::Unsupported
            } else if sufficient {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Inconclusive
            }
        };
        let correlations = [(25, 50.0), (50, 100.0)]
            .into_iter()
            .map(|(minimum, base)| {
                let lag = minimum.max((base * f64::from(self.rate) / 44100.0).round() as u32);
                let coefficient = range.clone().filter(|_| usable).map(|bins| {
                    let covariance: f64 = bins
                        .map(|k| {
                            all_power[k]
                                * (std::f64::consts::TAU * k as f64 * f64::from(lag)
                                    / WINDOW as f64)
                                    .cos()
                        })
                        .sum();
                    (covariance / total).clamp(-1.0, 1.0)
                });
                BandCorrelation {
                    lag_frames: lag,
                    lag_seconds: f64::from(lag) / f64::from(self.rate),
                    status: status(usable),
                    coefficient,
                }
            })
            .collect();

        let blocks: Vec<_> = self
            .seconds
            .iter()
            .enumerate()
            .take((samples / u64::from(self.rate)).min(CAP_SECONDS as u64) as usize)
            .filter(|(_, block)| block.frames > 0)
            .map(|(i, block)| {
                let mean_square = range.clone().map(|r| {
                    block.sums[r].iter().sum::<f64>() * normalization / block.frames as f64
                });
                BandPowerBlock {
                    block_index: i as u32,
                    interval: AnalysisInterval {
                        start_frame: block.start,
                        end_frame: block.end,
                    },
                    stft_frames: block.frames,
                    mean_square,
                    rms_dbfs: mean_square.filter(|&p| p > 0.0).map(|p| 10.0 * p.log10()),
                    eligible: mean_square.is_some_and(|p| {
                        eligible(p, broadband_power(&block.sums, block.frames, normalization))
                    }),
                }
            })
            .collect();
        let eligible_blocks = blocks.iter().filter(|b| b.eligible).count();
        // Do not drop zero/quiet blocks and accidentally report stable noise.
        let sufficient = blocks.len() >= 2 && eligible_blocks == blocks.len();
        let level_std_db = sufficient.then(|| {
            let mean =
                blocks.iter().map(|b| b.rms_dbfs.unwrap()).sum::<f64>() / blocks.len() as f64;
            (blocks
                .iter()
                .map(|b| (b.rms_dbfs.unwrap() - mean).powi(2))
                .sum::<f64>()
                / blocks.len() as f64)
                .sqrt()
        });
        (
            correlations,
            BandTemporalVariation {
                status: status(sufficient),
                search_limit_frames: CAP_SECONDS as u64 * u64::from(self.rate),
                eligible_blocks,
                level_std_db,
                blocks,
            },
        )
    }
}
