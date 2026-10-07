//! Separate pinned-source observations. No source-history scores.
use crate::{
    detectors::resampling::ResamplingStats, model::ResamplingAnalysis, reference_inputs::InputValue,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralInputs {
    pub domain: String,
    pub active_frames: u64,
    pub banding: InputValue,
    pub side_hf_ratio: InputValue,
    pub side_anomaly: InputValue,
    pub side_compared_frames: u64,
    pub top_nyquist_ratio: InputValue,
    pub lpf_detected: Option<bool>,
    pub lpf_last_bin_hz: InputValue,
    pub lpf_legacy_label: Option<String>,
    pub dsd_spectral_ratio: InputValue,
    pub dsd_like_spectrum: Option<bool>,
    pub resampling: ResamplingAnalysis,
    pub ordered_resampling_hit: Option<OrderedResamplingHit>,
    pub sparsity: InputValue,
    pub sparse_bins: u64,
    pub tested_bins: u64,
    pub envelope_correlation: InputValue,
    pub rolloff_db_per_khz: InputValue,
    pub comb_lags: Vec<CombLag>,
    pub comb_qualifying_peaks: usize,
    pub comb_pattern: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedResamplingHit {
    pub source_rate: u32,
    pub mode: String,
    pub strength: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombLag {
    pub lag: usize,
    pub correlation: InputValue,
    pub lower_neighbor: InputValue,
    pub upper_neighbor: InputValue,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderObservations {
    pub duration_difference_seconds: InputValue,
    pub duration_mismatch: Option<bool>,
    pub total_file_bitrate_ratio: InputValue,
    pub legacy_extension_gate: bool,
    pub legacy_bitrate_mismatch: Option<bool>,
    pub interpretation: String,
}
/// Caller supplies declared metadata explicitly; prefix duration is never compared as full EOF.
pub fn header_observations(
    decoded_seconds: f64,
    reached_end: bool,
    claimed_seconds: Option<f64>,
    claimed_kbps: Option<f64>,
    file_bytes: Option<u64>,
    extension: &str,
) -> HeaderObservations {
    let claimed = claimed_seconds.filter(|v| v.is_finite() && *v > 0.0);
    let delta = claimed
        .filter(|_| reached_end && decoded_seconds.is_finite() && decoded_seconds > 0.0)
        .map(|c| (c - decoded_seconds).abs());
    let ratio = claimed
        .filter(|c| *c > 1.0)
        .zip(claimed_kbps.filter(|b| b.is_finite() && *b > 0.0))
        .zip(file_bytes)
        .map(|((c, b), n)| n as f64 * 8.0 / c / 1000.0 / b);
    let gate = matches!(
        extension.to_ascii_lowercase().trim_start_matches('.'),
        "mp3" | "m4a" | "aac" | "ogg" | "opus" | "wma"
    );
    HeaderObservations{duration_difference_seconds:value(delta,"s"),duration_mismatch:delta.map(|v|v>0.5),total_file_bitrate_ratio:value(ratio,"ratio"),legacy_extension_gate:gate,legacy_bitrate_mismatch:if gate{ratio.map(|r|!(0.8..=1.35).contains(&r))}else{None},interpretation:"Integrity observations only (D07). Total file size includes metadata; extension is not codec identity or ancestry evidence.".into()}
}
pub(crate) fn value(v: Option<f64>, unit: &str) -> InputValue {
    InputValue::new(
        v,
        unit,
        "Insufficient frames, energy, geometry or supplied input (D03)",
    )
}
#[derive(Default)]
pub(crate) struct Moments {
    n: u64,
    mean: f64,
    m2: f64,
}
impl Moments {
    pub fn push(&mut self, x: f64) {
        self.n += 1;
        let d = x - self.mean;
        self.mean += d / self.n as f64;
        self.m2 += d * (x - self.mean);
    }
    pub fn std(&self) -> f64 {
        (self.m2 / self.n.max(1) as f64).max(0.0).sqrt()
    }
}
#[derive(Default)]
struct Pair {
    n: u64,
    a: f64,
    b: f64,
    aa: f64,
    bb: f64,
    ab: f64,
}
impl Pair {
    fn push(&mut self, a: f64, b: f64) {
        self.n += 1;
        let da = a - self.a;
        let db = b - self.b;
        self.a += da / self.n as f64;
        self.b += db / self.n as f64;
        self.aa += da * (a - self.a);
        self.bb += db * (b - self.b);
        self.ab += da * (b - self.b);
    }
    fn corr(&self) -> Option<f64> {
        (self.n >= 10 && self.aa / self.n as f64 >= 1e-16 && self.bb / self.n as f64 >= 1e-16)
            .then(|| (self.ab / (self.aa * self.bb).sqrt()).clamp(-1.0, 1.0))
    }
}
pub(crate) struct SpectralCollector {
    rate: u32,
    cutoff: Option<f64>,
    count: u64,
    sums: Vec<f32>,
    logs: Vec<Moments>,
    region: std::ops::Range<usize>,
    region_peak: f32,
    sparse: u64,
    tested: u64,
    pair: Pair,
    resampling: ResamplingStats,
    side_mid: f64,
    side_side: f64,
    side_count: u64,
}
impl SpectralCollector {
    pub fn new(rate: u32, cutoff: Option<f64>, maxima: Vec<f32>) -> Self {
        let hi = cutoff
            .map(|c| (c * 4096.0 / rate as f64) as usize)
            .unwrap_or(0)
            .min(2049);
        let lo = hi.saturating_sub((1500.0 * 4096.0 / rate as f64) as usize);
        let peak = maxima[lo..hi].iter().copied().fold(0.0, f32::max);
        Self {
            rate,
            cutoff,
            count: 0,
            sums: vec![0.0; 2049],
            logs: (0..hi - lo).map(|_| Moments::default()).collect(),
            region: lo..hi,
            region_peak: peak,
            sparse: 0,
            tested: 0,
            pair: Pair::default(),
            resampling: ResamplingStats::new(rate),
            side_mid: 0.0,
            side_side: 0.0,
            side_count: 0,
        }
    }
    pub fn side(&mut self, mid: &[f32], side: &[f32]) {
        let first = (10000.0 * 4096.0 / self.rate as f64) as usize;
        if first >= 2049 {
            return;
        }
        self.side_mid += mid[first..].iter().map(|v| *v as f64).sum::<f64>();
        self.side_side += side[first..].iter().map(|v| *v as f64).sum::<f64>();
        self.side_count += 1;
    }
    pub fn push(&mut self, m: &[f32]) {
        self.count += 1;
        for (s, x) in self.sums.iter_mut().zip(m) {
            *s += x;
        }
        self.resampling.push(m);
        for (mom, x) in self.logs.iter_mut().zip(&m[self.region.clone()]) {
            mom.push((20.0f32 * (*x / (self.region_peak + 1e-12) + 1e-12).log10()) as f64);
        }
        let hi = self
            .cutoff
            .map(|c| (c * 4096.0 / self.rate as f64) as usize)
            .unwrap_or(0)
            .min(m.len());
        let peak = m.iter().copied().fold(0.0, f32::max) + 1e-12;
        if hi >= 10 {
            self.sparse += m[..hi]
                .iter()
                .filter(|x| 20.0f32 * (**x / peak + 1e-12).log10() < -95.0)
                .count() as u64;
            self.tested += hi as u64;
        }
        let idx = |hz: f64| ((hz * 4096.0 / self.rate as f64) as usize).min(m.len());
        let energy = |l: usize, h: usize| -> Option<f64> {
            (h > l).then(|| {
                (m[l..h].iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / (h - l) as f64).sqrt()
            })
        };
        if let (Some(a), Some(b)) = (
            energy(idx(1000.0), idx(8000.0)),
            energy(idx(16000.0), idx(22000.0)),
        ) {
            self.pair.push(a, b);
        }
    }
    pub fn finish(self) -> SpectralInputs {
        let ready = self.count >= 4;
        let bin = self.rate as f64 / 4096.0;
        let ny = self.rate as f64 / 2.0;
        let avg: Vec<f32> = self
            .sums
            .iter()
            .map(|s| *s / self.count.max(1) as f32)
            .collect();
        let peak = avg.iter().copied().fold(0.0, f32::max) + 1e-12;
        let db: Vec<f32> = avg
            .iter()
            .map(|x| 20.0 * (*x / peak + 1e-12).log10())
            .collect();
        let top = (ny * 0.9 / bin) as usize;
        let total = avg.iter().map(|x| *x as f64).sum::<f64>();
        let top_ratio = (ready && total > 1e-12)
            .then(|| avg[top..].iter().map(|x| *x as f64).sum::<f64>() / (total + 1e-12));
        let lpf = top_ratio.map(|r| r < 0.00005);
        let last = (lpf == Some(true))
            .then(|| (1..=top).rev().find(|i| db[*i] > -40.0))
            .flatten();
        let dsd_ratio = if ready && self.rate > 48000 && ((30000.0 / bin) as usize) < avg.len() {
            let lo = (15000.0 / bin) as usize;
            let hi = (20000.0 / bin) as usize;
            let a = avg[lo..hi].iter().map(|x| *x as f64).sum::<f64>() / (hi - lo) as f64;
            let b = avg[(30000.0 / bin) as usize..]
                .iter()
                .map(|x| *x as f64)
                .sum::<f64>()
                / (avg.len() - (30000.0 / bin) as usize) as f64;
            (a > 1e-12).then_some((b + 1e-12) / (a + 1e-12))
        } else {
            None
        };
        let band = |f: f64| {
            let lo = ((f - 250.0) / bin) as usize;
            let hi = ((f + 250.0) / bin) as usize;
            (ready && f + 250.0 < ny && hi > lo)
                .then(|| db[lo..hi].iter().map(|x| *x as f64).sum::<f64>() / (hi - lo) as f64)
        };
        let slope = band(12000.0).zip(band(18000.0)).map(|(a, b)| (b - a) / 6.0);
        let lo = (16000.0 / bin) as usize;
        let hi = ((20000.0f64.min(ny - 100.0) / bin) as usize).min(2049);
        let mut lags = vec![];
        let mut peaks = 0;
        if ready && hi > lo + 80 {
            let mut spec: Vec<f32> = avg[lo..hi]
                .iter()
                .map(|x| 20.0 * (*x + 1e-12).log10())
                .collect();
            let mean = spec.iter().map(|x| *x as f64).sum::<f64>() as f32 / spec.len() as f32;
            for x in &mut spec {
                *x -= mean;
            }
            let den = spec.iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
            let ac = |l: usize| {
                if l > 0 && l < spec.len() - 1 && den > 1e-12 {
                    Some(
                        spec[..spec.len() - l]
                            .iter()
                            .zip(&spec[l..])
                            .map(|(a, b)| (*a as f64) * (*b as f64))
                            .sum::<f64>()
                            / (den + 1e-12),
                    )
                } else {
                    None
                }
            };
            for l in [64, 128, 192] {
                let (a, b, c) = (ac(l), ac(l - 3), ac(l + 3));
                if a.is_some_and(|a| {
                    a > 0.25 && a > 2.0 * b.unwrap_or(0.0).abs().max(c.unwrap_or(0.0).abs())
                }) {
                    peaks += 1;
                }
                lags.push(CombLag {
                    lag: l,
                    correlation: value(a, "correlation"),
                    lower_neighbor: value(b, "correlation"),
                    upper_neighbor: value(c, "correlation"),
                });
            }
        }
        let comb = lags
            .iter()
            .any(|l| l.correlation.value.is_some())
            .then_some(peaks >= 2);
        let resampling = self.resampling.finish(0);
        let hit = resampling.candidates.iter().find_map(|c| {
            c.modes.first().map(|m| OrderedResamplingHit {
                source_rate: c.source_rate,
                mode: m.clone(),
                strength: match m.as_str() {
                    "notch" => {
                        c.edge_relative_db
                            .unwrap()
                            .min(c.above_relative_db.unwrap())
                            - c.notch_relative_db.unwrap()
                    }
                    "wall" => c.edge_relative_db.unwrap() - c.ceiling_relative_db.unwrap(),
                    _ => c.mirror_correlation.unwrap(),
                },
            })
        });
        let ratio = (self.side_count > 0 && self.side_mid > 1e-12).then(|| {
            self.side_side
                / (self.side_mid
                    + 1e-12 * (self.side_count as f64 * (2049 - (10000.0 / bin) as usize) as f64))
        });
        SpectralInputs {
            domain: "pinned f32 active mid STFT; side uses unmasked aligned stride-4 frames".into(),
            active_frames: self.count,
            banding: value(
                (ready && self.logs.len() >= 4 && self.region_peak > 1e-12).then(|| {
                    (1.0 - self.logs.iter().map(Moments::std).sum::<f64>()
                        / self.logs.len() as f64
                        / 15.0)
                        .clamp(0.0, 1.0)
                }),
                "ratio",
            ),
            side_hf_ratio: value(ratio, "ratio"),
            side_anomaly: value(
                ratio.map(|r| {
                    if r < 0.02 {
                        1.0
                    } else if r < 0.08 {
                        0.6
                    } else {
                        0.0
                    }
                }),
                "source scalar",
            ),
            side_compared_frames: self.side_count,
            top_nyquist_ratio: value(top_ratio, "ratio"),
            lpf_detected: lpf,
            lpf_last_bin_hz: value(last.map(|i| i as f64 * bin), "Hz"),
            lpf_legacy_label: lpf.filter(|v| *v).map(|_| {
                last.map(|i| format!("~{}kHz", (i as f64 * bin / 1000.0) as usize))
                    .unwrap_or_else(|| "< 1kHz".into())
            }),
            dsd_spectral_ratio: value(dsd_ratio, "ratio"),
            dsd_like_spectrum: dsd_ratio.map(|r| r > 1.5),
            resampling,
            ordered_resampling_hit: hit,
            sparsity: value(
                (ready && self.tested > 0).then(|| self.sparse as f64 / self.tested as f64),
                "fraction",
            ),
            sparse_bins: self.sparse,
            tested_bins: self.tested,
            envelope_correlation: value(self.pair.corr(), "signed Pearson"),
            rolloff_db_per_khz: value(slope, "dB/kHz"),
            comb_lags: lags,
            comb_qualifying_peaks: peaks,
            comb_pattern: comb,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64, t: f64) {
        assert!((a - b).abs() <= t, "{a} != {b}, tolerance {t}");
    }
    #[test]
    fn pinned_spectral_controls() {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/reference_profiles.json"))
                .unwrap();
        for c in oracle["spectral"].as_array().unwrap() {
            let rate = c["rate"].as_u64().unwrap() as u32;
            let kind = c["kind"].as_str().unwrap();
            let mut rows = vec![];
            for t in 0..32 {
                let row: Vec<f32> = (0..2049)
                    .map(|k| {
                        let mut v =
                            (0.05 + ((k * 17 + t * 31 + k * t * 3) % 997) as f64 / 997.0) as f32;
                        let hz = k as f64 * rate as f64 / 4096.0;
                        if kind == "wall" && hz > 16000.0 {
                            v *= 1e-6;
                        }
                        if kind == "notch" && (hz - 22050.0).abs() < 350.0 {
                            v *= 1e-4;
                        }
                        if kind == "constant" {
                            v = 1.0;
                        }
                        v
                    })
                    .collect();
                rows.push(row);
            }
            let mut maxima = vec![0.0f32; 2049];
            for row in &rows {
                for (p, x) in maxima.iter_mut().zip(row) {
                    *p = p.max(*x);
                }
            }
            let mut s = SpectralCollector::new(rate, c["cutoff"].as_f64(), maxima);
            for row in &rows {
                s.push(row);
            }
            let r = s.finish();
            near(
                r.banding.value.unwrap(),
                c["banding"].as_f64().unwrap(),
                1e-3,
            );
            assert_eq!(r.lpf_detected, Some(c["lpf"][0].as_bool().unwrap()));
            if r.lpf_detected == Some(true) {
                assert_eq!(r.lpf_legacy_label.as_deref(), c["lpf"][1].as_str());
            }
            if let Some(d) = r.dsd_like_spectrum {
                assert_eq!(d, c["dsd"].as_bool().unwrap());
            }
            if let Some(v) = r.sparsity.value {
                near(v, c["sparsity"].as_f64().unwrap(), 1e-12);
            }
            if let Some(v) = r.envelope_correlation.value {
                near(v, c["envelope"].as_f64().unwrap(), 1e-3);
            }
            if let Some(h) = r.ordered_resampling_hit {
                assert_eq!(h.source_rate, c["resample"][0].as_u64().unwrap() as u32);
                assert_eq!(h.mode, c["resample"][1].as_str().unwrap());
                near(h.strength, c["resample"][2].as_f64().unwrap(), 0.05);
            } else {
                assert!(c["resample"].is_null());
            }
            for (i, l) in r.comb_lags.iter().enumerate() {
                for (got, index) in [
                    (&l.correlation, i),
                    (&l.lower_neighbor, 3 + i * 2),
                    (&l.upper_neighbor, 4 + i * 2),
                ] {
                    if let Some(v) = got.value {
                        if let Some(e) = c["lags"][index].as_f64() {
                            near(v, e, 0.003);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn header_and_side_boundaries() {
        for (ratio, expected) in [(0.8, false), (1.35, false), (0.799, true), (1.351, true)] {
            let r = header_observations(
                10.5,
                true,
                Some(10.0),
                Some(8.0),
                Some((ratio * 10000.0) as u64),
                "m4a",
            );
            assert_eq!(r.duration_mismatch, Some(false));
            assert_eq!(r.legacy_bitrate_mismatch, Some(expected));
        }
        assert_eq!(
            header_observations(2.0, false, Some(10.0), None, None, "wav").duration_mismatch,
            None
        );
        assert_eq!(
            header_observations(10.5001, true, Some(10.0), None, None, "wav").duration_mismatch,
            Some(true)
        );
        for (scale, score) in [(0.01, 1.0), (0.04, 0.6), (0.1, 0.0)] {
            let mut s = SpectralCollector::new(48000, Some(16000.0), vec![1.0; 2049]);
            s.side(&vec![1.0; 2049], &vec![scale; 2049]);
            near(s.finish().side_anomaly.value.unwrap(), score, 0.0);
        }
        let s = SpectralCollector::new(8000, None, vec![0.0; 2049]).finish();
        assert!(s.banding.value.is_none());
        assert!(s.envelope_correlation.value.is_none());
        assert!(s.comb_pattern.is_none());
    }
}
