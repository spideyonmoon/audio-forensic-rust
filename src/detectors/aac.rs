//! AAC long-window lattice observations, ported from the pinned Python reference.
//! Pass one retains bounded hop energies; pass two captures only selected spans.
use super::mdct::Mdct;
use crate::model::{AacAnalysis, AacBasis, AacProbe, AnalysisInterval, DetectorStatus};

pub(crate) const SIZE: usize = 2048;
const HOP: usize = SIZE / 2;
const STEP: usize = 8;
const PHASES: usize = HOP / STEP;
const SCALES: usize = 8;
const SPAN: usize = SIZE + HOP - STEP;
pub(crate) const CAP_SECONDS: u64 = 180;
pub(crate) const MAX_ANCHORS: usize = 16;
pub(crate) const HIT_SCORE: f64 = 0.10;
const SWB: [usize; 50] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928, 1024,
];

// K/12 + sqrt(K/180) * ndtri(.01 + .99*ndtr(-(K/12)/sqrt(K/180))).
// Fixed band sizes avoid a runtime special-functions dependency. Checked against SciPy.
fn gamma(k: usize) -> f64 {
    match k {
        4 => 0.034627743940669975,
        8 => 0.18216111782169564,
        12 => 0.3998539305495409,
        16 => 0.6397932766660791,
        20 => 0.8912209245221232,
        24 => 1.1505381548589444,
        28 => 1.4158093824133844,
        32 => 1.685792280407484,
        96 => 6.301075723711526,
        _ => unreachable!("fixed scalefactor-band table"),
    }
}

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    for k in 1..100 {
        term *= (x / (2.0 * k as f64)).powi(2);
        sum += term;
        if term < sum * f64::EPSILON {
            break;
        }
    }
    sum
}

fn kbd_window() -> Vec<f64> {
    let mut cumulative = Vec::with_capacity(HOP + 1);
    let mut sum = 0.0;
    for i in 0..=HOP {
        let position = 2.0 * i as f64 / HOP as f64 - 1.0;
        // The common I0(beta) denominator cancels during normalization.
        sum += bessel_i0(std::f64::consts::PI * 4.0 * (1.0 - position * position).sqrt());
        cumulative.push(sum);
    }
    let rising: Vec<_> = cumulative[..HOP].iter().map(|x| (x / sum).sqrt()).collect();
    rising.iter().chain(rising.iter().rev()).copied().collect()
}

// scan() visits native samples in interleaved frame order. Transform only this
// detector's copy, in f64, leaving exact integer/native-channel paths untouched.
struct BasisFrame {
    channels: usize,
    left: f64,
    reference_f32: bool,
}

impl BasisFrame {
    fn push(&mut self, channel: usize, sample: f64) -> Option<[f64; 2]> {
        let sample = if self.reference_f32 {
            f64::from(sample as f32)
        } else {
            sample
        };
        if self.channels == 1 {
            Some([sample, 0.0])
        } else if channel == 0 {
            self.left = sample;
            None
        } else {
            Some(if self.reference_f32 {
                [
                    f64::from((self.left as f32 + sample as f32) * 0.5),
                    f64::from((self.left as f32 - sample as f32) * 0.5),
                ]
            } else {
                [(self.left + sample) * 0.5, (self.left - sample) * 0.5]
            })
        }
    }
}

#[derive(Default)]
struct Energy {
    previous: f64,
    current: f64,
    blocks: Vec<f64>,
}

pub(crate) struct AacSelector {
    rate: u32,
    frame: BasisFrame,
    position: u64,
    energies: Vec<Energy>,
}

impl AacSelector {
    pub fn new(rate: u32, channels: usize) -> Self {
        Self {
            rate,
            frame: BasisFrame {
                channels,
                left: 0.0,
                reference_f32: false,
            },
            position: 0,
            energies: (0..channels).map(|_| Energy::default()).collect(),
        }
    }

    /// Pinned Python rounding is opt-in; native selection remains f64.
    pub(crate) fn new_reference(rate: u32, channels: usize) -> Self {
        let mut selector = Self::new(rate, channels);
        selector.frame.reference_f32 = true;
        selector
    }

    pub fn push(&mut self, channel: usize, sample: f64) {
        if !matches!(self.rate, 44100 | 48000)
            || self.position >= CAP_SECONDS * u64::from(self.rate)
        {
            return;
        }
        let Some(samples) = self.frame.push(channel, sample) else {
            return;
        };
        self.position += 1;
        for (energy, sample) in self.energies.iter_mut().zip(samples) {
            energy.current += (sample * 32768.0).powi(2);
            if self.position % HOP as u64 == 0 {
                if self.position >= SIZE as u64 {
                    energy.blocks.push(energy.previous + energy.current);
                }
                energy.previous = energy.current;
                energy.current = 0.0;
            }
        }
    }

    pub fn into_collector(self, frames: u64) -> AacCollector {
        let limit = frames.min(CAP_SECONDS * u64::from(self.rate));
        let channels = self.frame.channels;
        let bases = self
            .energies
            .into_iter()
            .enumerate()
            .map(|(index, energy)| {
                let npos = limit.saturating_sub(SIZE as u64) / HOP as u64;
                let energies = &energy.blocks[..energy.blocks.len().min(npos as usize)];
                let floor =
                    (SIZE as f64 * 100.0).max(energies.iter().copied().fold(0.0, f64::max) * 1e-6);
                let mut candidates: Vec<_> = energies.iter().copied().enumerate().collect();
                candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                let mut selected = Vec::<(u64, f64)>::new();
                if limit >= (SIZE * 4) as u64 {
                    for (index, energy) in candidates {
                        if energy < floor {
                            break;
                        }
                        let anchor = index as u64 * HOP as u64;
                        // Retain the reference's conservative boundary test, even
                        // though the stepped phase search needs seven fewer samples.
                        if anchor < HOP as u64 / 2
                            || anchor - HOP as u64 / 2 + (SIZE + HOP - 1) as u64 > limit
                        {
                            continue;
                        }
                        if selected
                            .iter()
                            .all(|(other, _)| other.abs_diff(anchor) > SIZE as u64)
                        {
                            selected.push((anchor, energy));
                        }
                        if selected.len() == MAX_ANCHORS {
                            break;
                        }
                    }
                }
                selected.sort_by_key(|(anchor, _)| *anchor);
                BasisCapture {
                    basis: if channels == 1 {
                        AacBasis::Mono
                    } else if index == 0 {
                        AacBasis::Mid
                    } else {
                        AacBasis::Side
                    },
                    anchors: selected
                        .iter()
                        .map(|&(anchor, energy)| AacProbe {
                            anchor_frame: anchor,
                            interval: AnalysisInterval {
                                start_frame: anchor - HOP as u64 / 2,
                                end_frame: anchor - HOP as u64 / 2 + SPAN as u64,
                            },
                            anchor_rms: (energy / SIZE as f64).sqrt() / 32768.0,
                            selected_eligible_bands: None,
                            selected_flagged_bands: None,
                        })
                        .collect(),
                    spans: selected.iter().map(|_| Vec::with_capacity(SPAN)).collect(),
                    first: 0,
                }
            })
            .collect();
        AacCollector {
            rate: self.rate,
            limit,
            frame: self.frame,
            position: 0,
            bases,
        }
    }
}

struct BasisCapture {
    basis: AacBasis,
    anchors: Vec<AacProbe>,
    spans: Vec<Vec<f64>>,
    first: usize,
}

pub(crate) struct AacCollector {
    rate: u32,
    limit: u64,
    frame: BasisFrame,
    position: u64,
    bases: Vec<BasisCapture>,
}

#[derive(Clone, Copy, Default)]
struct Vote {
    eligible: u8,
    flagged: u8,
}

fn band_votes(coefficients: &[f64]) -> [Vote; SCALES] {
    let mut powers = [0.0f64; 49];
    let mut maxima = [0.0f64; 49];
    for (band, edges) in SWB.windows(2).enumerate() {
        let values = &coefficients[edges[0]..edges[1]];
        powers[band] = values.iter().map(|x| x * x).sum::<f64>() / values.len() as f64;
        maxima[band] = values.iter().map(|x| x.abs()).fold(1e-12, f64::max);
    }
    let floor = 1.0f64.max(powers.iter().copied().fold(0.0, f64::max) * 1e-6);
    let powered: [f64; HOP] = std::array::from_fn(|i| coefficients[i].abs().powf(0.75));
    let mut votes = [Vote::default(); SCALES];
    for (band, edges) in SWB.windows(2).enumerate() {
        if powers[band] <= floor {
            continue;
        }
        let sdz = 16.0 + (4.0 / 3.0) * maxima[band].log2();
        let count = edges[1] - edges[0];
        for (sf, vote) in votes.iter_mut().enumerate() {
            let smin = 0.3 * sdz;
            let smax = 0.7 * sdz;
            let scale = (-3.0 * (smin + sf as f64 / 7.0 * (smax - smin)) / 16.0).exp2();
            let mut error = 0.0;
            let mut populated = 0;
            for coefficient in &powered[edges[0]..edges[1]] {
                let scaled = coefficient * scale;
                error += (scaled.round_ties_even() - scaled).powi(2);
                populated += usize::from(scaled >= 0.5);
            }
            if populated * 2 >= count {
                vote.eligible += 1;
                vote.flagged += u8::from(error < gamma(count));
            }
        }
    }
    votes
}

impl AacCollector {
    pub fn push(&mut self, channel: usize, sample: f64) {
        if self.position >= self.limit {
            return;
        }
        let Some(samples) = self.frame.push(channel, sample) else {
            return;
        };
        for (basis, sample) in self.bases.iter_mut().zip(samples) {
            while basis.first < basis.anchors.len()
                && self.position >= basis.anchors[basis.first].interval.end_frame
            {
                basis.first += 1;
            }
            for index in basis.first..basis.anchors.len() {
                if self.position < basis.anchors[index].interval.start_frame {
                    break;
                }
                basis.spans[index].push(sample * 32768.0);
            }
        }
        self.position += 1;
    }

    pub fn finish<E>(
        self,
        mut control: impl FnMut() -> Result<(), E>,
    ) -> Result<Vec<AacAnalysis>, E> {
        let mut mdct = Mdct::new(kbd_window());
        let mut results = vec![];
        for mut basis in self.bases {
            control()?;
            let mut curves = vec![];
            if basis.anchors.len() >= 4 {
                for span in &basis.spans {
                    let mut curve = vec![Vote::default(); SCALES * PHASES];
                    for phase in 0..PHASES {
                        // Includes transform, powers and all band/scale rounding.
                        if phase % 8 == 0 {
                            control()?;
                        }
                        let votes =
                            band_votes(mdct.process(&span[phase * STEP..phase * STEP + SIZE]));
                        for (sf, vote) in votes.into_iter().enumerate() {
                            curve[sf * PHASES + phase] = vote;
                        }
                    }
                    curves.push(curve);
                }
            }
            let mut best = None;
            let mut score = None;
            let mut eligible_probes = 0;
            for index in 0..SCALES * PHASES {
                let eligible = curves.iter().filter(|c| c[index].eligible >= 16).count();
                // Explicit abstention for sparse spectra, including tones. Python
                // instead returns zero and can maximize over fewer usable probes.
                if eligible < 4 {
                    continue;
                }
                let value = curves
                    .iter()
                    .filter(|c| c[index].eligible >= 16)
                    .map(|c| f64::from(c[index].flagged) / 49.0)
                    .sum::<f64>()
                    / curves.len() as f64;
                if score.is_none_or(|prior| value > prior) {
                    best = Some(index);
                    score = Some(value);
                    eligible_probes = eligible;
                }
            }
            if let Some(index) = best {
                for (probe, curve) in basis.anchors.iter_mut().zip(&curves) {
                    probe.selected_eligible_bands = Some(usize::from(curve[index].eligible));
                    probe.selected_flagged_bands = Some(usize::from(curve[index].flagged));
                }
            }
            let status = if !matches!(self.rate, 44100 | 48000) {
                DetectorStatus::Unsupported
            } else if let Some(value) = score {
                if value >= HIT_SCORE {
                    DetectorStatus::Hit
                } else {
                    DetectorStatus::NotDetected
                }
            } else {
                DetectorStatus::Inconclusive
            };
            results.push(AacAnalysis {
                basis: basis.basis,
                status,
                search_limit_frames: self.limit,
                lattice_score: score,
                phase_offset: best.map(|i| i % PHASES * STEP),
                scalefactor_index: best.map(|i| i / PHASES),
                eligible_probes,
                probes: basis.anchors,
            });
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kbd_and_band_thresholds_match_scipy_reference() {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aac_reference.json")).unwrap();
        let window = kbd_window();
        for (i, expected) in oracle["window"].as_array().unwrap().iter().enumerate() {
            assert!(
                (window[i] - expected.as_f64().unwrap()).abs() < 2e-14,
                "window {i}"
            );
            assert!((window[i].powi(2) + window[(i + HOP) % SIZE].powi(2) - 1.0).abs() < 2e-14);
        }
        for (i, band) in SWB.windows(2).enumerate() {
            assert!(
                (gamma(band[1] - band[0]) - oracle["gamma"][i].as_f64().unwrap()).abs() < 1e-13
            );
        }
        let input: Vec<_> = oracle["mdct_input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let mut mdct = Mdct::new(window);
        for (i, actual) in mdct.process(&input).iter().enumerate() {
            assert!(
                (actual - oracle["mdct"][i].as_f64().unwrap()).abs() < 1e-8,
                "MDCT coefficient {i}"
            );
        }
    }

    #[test]
    fn selection_capture_and_energy_storage_are_bounded() {
        for rate in [44100, 48000, 96000] {
            for frames in [100, 8191, 8192, 48_000] {
                let mut selector = AacSelector::new(rate, 1);
                for i in 0..frames {
                    selector.push(0, 0.1 + i as f64 * 1e-8);
                }
                let mut collector = selector.into_collector(frames);
                let anchors = &collector.bases[0].anchors;
                assert!(anchors.len() <= MAX_ANCHORS);
                assert!(
                    anchors
                        .windows(2)
                        .all(|w| w[1].anchor_frame - w[0].anchor_frame > SIZE as u64)
                );
                assert!(anchors.iter().all(|a| a.interval.end_frame <= frames));
                if frames < 8192 || rate == 96000 {
                    assert!(anchors.is_empty());
                }
                for i in 0..frames {
                    collector.push(0, i as f64);
                }
                for (anchor, span) in collector.bases[0]
                    .anchors
                    .iter()
                    .zip(&collector.bases[0].spans)
                {
                    assert_eq!(span.len(), SPAN);
                    assert_eq!(span[0], anchor.interval.start_frame as f64 * 32768.0);
                    assert_eq!(
                        span[SPAN - 1],
                        (anchor.interval.end_frame - 1) as f64 * 32768.0
                    );
                }
            }
        }
        let mut selector = AacSelector::new(48000, 2);
        let cap = CAP_SECONDS * 48000;
        for _ in 0..cap + 5000 {
            selector.push(0, 0.2);
            selector.push(1, -0.2);
        }
        assert_eq!(selector.position, cap);
        for energy in &selector.energies {
            assert_eq!(energy.blocks.len() as u64, cap / HOP as u64 - 1);
        }
        let collector = selector.into_collector(cap + 5000);
        assert_eq!(collector.limit, cap);
        assert!(collector.bases[0].anchors.is_empty());
        assert_eq!(collector.bases[1].anchors.len(), MAX_ANCHORS);
    }

    #[test]
    fn cancellation_propagates_from_inside_lattice_search() {
        let mut selector = AacSelector::new(44100, 1);
        for i in 0..44100 {
            selector.push(0, (i as f64 * 1.345).sin());
        }
        let mut collector = selector.into_collector(44100);
        for i in 0..44100 {
            collector.push(0, (i as f64 * 1.345).sin());
        }
        let mut calls = 0;
        let result = collector.finish(|| {
            calls += 1;
            if calls == 5 { Err("cancelled") } else { Ok(()) }
        });
        assert!(matches!(result, Err("cancelled")));
        assert_eq!(calls, 5);
    }
}
