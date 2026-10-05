//! Provisional wall observations on deterministic, non-overlapping two-second probes.
//! Measurements follow the Python clip transform; sampling and gates are versioned
//! policy changes. Walls and codec matches belong to one evidence family.
use crate::model::{
    AnalysisInterval, CodecWallCandidate, DetectorStatus, SegmentAnalysis, SegmentProbe,
};
use rustfft::{Fft, FftPlanner, num_complex::Complex64};
use std::sync::Arc;

pub(crate) const SILENT_PEAK_DB: f64 = -50.0;
pub(crate) const MIN_CLIFF_DB: f64 = 30.0;
pub(crate) const MAX_VOID_DB: f64 = -75.0;
pub(crate) const MIN_OCCUPANCY: f64 = 0.30;
pub(crate) const MAX_PROBES: usize = 36;

// Values retained from the pinned Python CODEC_WALLS, not specification claims.
pub(crate) const WALLS: &[(&str, &str, f64, f64)] = &[
    ("MP3 (LAME)", "320 kbps", 20220.0, 150.0),
    ("MP3 (LAME)", "320 kbps @48k", 20510.0, 150.0),
    ("MP3 (LAME)", "256 kbps", 19530.0, 150.0),
    ("MP3 (LAME)", "256 kbps @48k", 19760.0, 150.0),
    ("MP3 (LAME)", "192 kbps", 18840.0, 150.0),
    ("MP3 (LAME)", "192 kbps @48k", 19010.0, 150.0),
    ("MP3 (LAME)", "160 kbps", 17460.0, 150.0),
    ("MP3 (LAME)", "128 kbps", 16770.0, 150.0),
    ("MP3 (LAME)", "96 kbps", 15410.0, 150.0),
    ("MP3 (LAME)", "64 kbps", 11270.0, 250.0),
    ("AAC", "~192 kbps", 19350.0, 150.0),
    ("AAC", "~192 kbps @48k", 19560.0, 150.0),
    ("AAC", "~128 kbps", 17280.0, 150.0),
    ("AAC", "~96 kbps", 15860.0, 180.0),
    ("Vorbis", "q4 (~128 kbps)", 19000.0, 150.0),
    ("Vorbis", "q4 @48k", 19180.0, 150.0),
    ("Vorbis", "q2 (~96 kbps)", 16575.0, 150.0),
    (
        "Opus",
        "CELT 20 kHz band limit (any bitrate)",
        20460.0,
        260.0,
    ),
];

fn candidates(cutoff: f64, rate: u32) -> Vec<CodecWallCandidate> {
    // No validated profile tables at other analysis rates. Retain overlapping
    // hypotheses at 44.1/48k; the observed rate need not be the historical rate.
    if !matches!(rate, 44100 | 48000) || cutoff >= rate as f64 * 0.49 {
        return vec![];
    }
    let mut out: Vec<_> = WALLS
        .iter()
        .filter_map(|&(codec, profile, hz, tolerance)| {
            let distance = (cutoff - hz).abs();
            (distance <= tolerance).then(|| CodecWallCandidate {
                codec: codec.into(),
                profile: profile.into(),
                reference_wall_hz: hz,
                tolerance_hz: tolerance,
                distance_hz: distance,
            })
        })
        .collect();
    out.sort_by(|a, b| a.distance_hz.total_cmp(&b.distance_hz));
    out
}

pub(crate) fn offsets(frames: u64, rate: u32) -> Vec<u64> {
    let length = u64::from(rate) * 2;
    if rate < 32000 || frames < length {
        return vec![];
    }
    let count = (frames / (u64::from(rate) * 15))
        .clamp(9, MAX_PROBES as u64)
        .min(frames / length) as usize;
    if count == 1 {
        return vec![0];
    }
    (0..count)
        .map(|i| (((frames - length) as u128 * i as u128) / (count - 1) as u128) as u64)
        .collect()
}

pub(crate) struct ClipAnalyzer {
    rate: u32,
    window: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
    buffer: Vec<Complex64>,
    scratch: Vec<Complex64>,
    mag: Vec<f64>,
    db: Vec<f64>,
}

impl ClipAnalyzer {
    #[cfg(test)]
    pub(crate) fn payload_bytes(&self) -> usize {
        (self.window.capacity() + self.mag.capacity() + self.db.capacity()) * 8
            + (self.buffer.capacity() + self.scratch.capacity()) * 16
    }
    pub(crate) fn new(rate: u32) -> Self {
        let n = rate as usize * 2;
        let fft = FftPlanner::new().plan_fft_forward(n);
        let scratch = vec![Complex64::default(); fft.get_inplace_scratch_len()];
        Self {
            rate,
            window: (0..n)
                .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (n - 1) as f64).cos())
                .collect(),
            fft,
            buffer: vec![Complex64::default(); n],
            scratch,
            mag: vec![0.0; n / 2 + 1],
            db: vec![0.0; n / 2 + 1],
        }
    }

    pub(crate) fn measure(&mut self, samples: &[f64], start: u64) -> SegmentProbe {
        let peak = samples.iter().map(|x| x.abs()).fold(0.0, f64::max);
        let peak_db = (peak > 0.0).then(|| 20.0 * peak.log10());
        let active = peak_db.is_some_and(|p| p >= SILENT_PEAK_DB);
        let mut result = SegmentProbe {
            interval: AnalysisInterval {
                start_frame: start,
                end_frame: start + samples.len() as u64,
            },
            peak_dbfs: peak_db,
            cutoff_hz: None,
            cliff_db: None,
            high_band_relative_db: None,
            above_cutoff_relative_db: None,
            occupied_band_fraction: None,
            active,
            eligible: false,
            wall_observed: false,
            codec_candidates: vec![],
        };
        if !active {
            return result;
        }
        for (i, x) in samples.iter().enumerate() {
            self.buffer[i] = Complex64::new(x * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        for (m, x) in self.mag.iter_mut().zip(&self.buffer) {
            *m = x.norm();
        }
        let reference = self.mag.iter().copied().fold(0.0, f64::max) + 1e-12;
        for (db, m) in self.db.iter_mut().zip(&self.mag) {
            *db = 20.0 * (m / reference + 1e-12).log10();
        }
        let bin_hz = self.rate as f64 / samples.len() as f64;
        let cutoff = self.db.iter().rposition(|db| *db > -65.0).unwrap_or(0) as f64 * bin_hz;
        result.cutoff_hz = Some(cutoff);
        let lo_a = ((cutoff - 450.0) / bin_hz) as isize;
        let lo_b = ((cutoff - 350.0) / bin_hz) as isize;
        let hi_a = ((cutoff + 350.0) / bin_hz) as usize;
        let hi_b = ((cutoff + 450.0) / bin_hz) as usize;
        if lo_a > 0 && lo_b > lo_a && hi_b < self.db.len() && hi_b > hi_a {
            let mean = |slice: &[f64]| slice.iter().sum::<f64>() / slice.len() as f64;
            result.cliff_db =
                Some(mean(&self.db[lo_a as usize..lo_b as usize]) - mean(&self.db[hi_a..hi_b]));
        }
        let nyquist = self.rate as f64 / 2.0;
        let relative_band = |low: f64, high: f64| -> Option<f64> {
            let lo = (low / bin_hz).ceil() as usize;
            let hi = ((high / bin_hz).floor() as usize + 1).min(self.mag.len());
            if high < low || lo >= hi {
                return None;
            }
            let power = self.mag[lo..hi]
                .iter()
                .map(|m| (m / reference).powi(2))
                .sum::<f64>()
                / (hi - lo) as f64;
            Some(20.0 * (power.sqrt() + 1e-15).log10())
        };
        result.high_band_relative_db = relative_band(18500.0, nyquist - 500.0);
        result.above_cutoff_relative_db = relative_band(cutoff + 800.0, nyquist - 100.0);
        let low = (1000.0 / bin_hz) as usize;
        let high = (((cutoff - 1000.0).max(0.0) / bin_hz) as usize).min(self.db.len());
        if high > low {
            let occupied = self.db[low..high].iter().filter(|db| **db > -45.0).count() as f64
                / (high - low) as f64;
            result.occupied_band_fraction = Some(occupied);
            result.eligible = cutoff >= 8000.0 && occupied >= MIN_OCCUPANCY;
        }
        result.wall_observed = result.eligible
            && cutoff < 22500.0f64.min(nyquist * 0.98)
            && result.cliff_db.is_some_and(|db| db > MIN_CLIFF_DB)
            && result
                .above_cutoff_relative_db
                .is_some_and(|db| db < MAX_VOID_DB);
        if result.wall_observed {
            result.codec_candidates = candidates(cutoff, self.rate);
        }
        result
    }
}

struct ChannelCollector {
    position: u64,
    next: usize,
    samples: Vec<f64>,
    probes: Vec<SegmentProbe>,
}

pub(crate) struct SegmentCollector {
    rate: u32,
    offsets: Vec<u64>,
    channels: Vec<ChannelCollector>,
    analyzer: Option<ClipAnalyzer>,
}

impl SegmentCollector {
    pub fn new(frames: u64, rate: u32, channels: usize) -> Self {
        let offsets = offsets(frames, rate);
        let enabled = !offsets.is_empty();
        Self {
            rate,
            offsets,
            channels: (0..channels)
                .map(|_| ChannelCollector {
                    position: 0,
                    next: 0,
                    samples: if enabled {
                        Vec::with_capacity(rate as usize * 2)
                    } else {
                        vec![]
                    },
                    probes: vec![],
                })
                .collect(),
            analyzer: enabled.then(|| ClipAnalyzer::new(rate)),
        }
    }

    pub fn push(&mut self, channel: usize, sample: f64) {
        let state = &mut self.channels[channel];
        if let Some(&offset) = self.offsets.get(state.next) {
            if state.position >= offset {
                state.samples.push(sample);
                if state.samples.len() == self.rate as usize * 2 {
                    state.probes.push(
                        self.analyzer
                            .as_mut()
                            .unwrap()
                            .measure(&state.samples, offset),
                    );
                    state.samples.clear();
                    state.next += 1;
                }
            }
        }
        state.position += 1;
    }

    pub fn finish(self) -> Vec<SegmentAnalysis> {
        self.channels
            .into_iter()
            .enumerate()
            .map(|(channel, state)| {
                let active = state.probes.iter().filter(|p| p.active).count();
                let eligible = state.probes.iter().filter(|p| p.eligible).count();
                let walled = state.probes.iter().filter(|p| p.wall_observed).count();
                let (status, pattern) = if self.rate < 32000 {
                    (DetectorStatus::Unsupported, "unsupported_rate")
                } else if eligible < 3 {
                    (DetectorStatus::Inconclusive, "insufficient_probes")
                } else if walled == 0 {
                    (DetectorStatus::NotDetected, "no_walls_observed")
                } else if walled == 1 {
                    (DetectorStatus::Measured, "isolated_wall")
                } else if walled >= eligible.div_ceil(2) {
                    (DetectorStatus::Hit, "repeated_walls")
                } else {
                    (DetectorStatus::Hit, "mixed_probes")
                };
                SegmentAnalysis {
                    channel_index: channel,
                    status,
                    active_probes: active,
                    eligible_probes: eligible,
                    wall_probes: walled,
                    pattern: pattern.into(),
                    probes: state.probes,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probe_intervals_are_bounded_unique_and_include_the_end() {
        for rate in [32000, 44100, 48000, 96000, 384000] {
            for seconds in [1, 2, 5, 18, 31, 600, 100_000] {
                let frames = rate as u64 * seconds;
                let positions = offsets(frames, rate);
                assert!(positions.len() <= MAX_PROBES);
                if frames < rate as u64 * 2 {
                    assert!(positions.is_empty());
                    continue;
                }
                assert_eq!(positions[0], 0);
                if positions.len() > 1 {
                    assert_eq!(positions.last().unwrap() + rate as u64 * 2, frames);
                }
                assert!(positions.windows(2).all(|p| p[1] >= p[0] + rate as u64 * 2));
            }
        }
    }
    #[test]
    fn overlapping_profiles_remain_ambiguous() {
        let hits = candidates(19550.0, 44100);
        assert!(hits.iter().any(|c| c.codec == "AAC"));
        assert!(hits.iter().any(|c| c.codec == "MP3 (LAME)"));
        assert!(candidates(21000.0, 44100).is_empty());
        assert!(candidates(19550.0, 96000).is_empty());
    }
    #[test]
    fn silence_tones_and_short_inputs_abstain() {
        for mode in 0..3 {
            let rate = 44100;
            let frames = if mode == 2 { 100 } else { rate * 6 };
            let mut collector = SegmentCollector::new(frames as u64, rate, 1);
            for i in 0..frames {
                let x = if mode == 1 {
                    0.5 * (std::f64::consts::TAU * 12000.0 * i as f64 / rate as f64).sin()
                } else {
                    0.0
                };
                collector.push(0, x);
            }
            let result = collector.finish().remove(0);
            assert_eq!(result.status, DetectorStatus::Inconclusive);
            assert_eq!(result.wall_probes, 0);
        }
    }
}
