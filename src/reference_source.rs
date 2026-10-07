//! Capped time-domain reference profiles, distinct from native circular STFT statistics.
use crate::{
    decode::Failure,
    model::{AnalysisInterval, NoiseFloorAnalysis},
    reference_inputs::InputValue,
    reference_spectral::value,
};
use rustfft::{FftPlanner, num_complex::Complex32, num_complex::Complex64};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredProfile {
    pub interval: AnalysisInterval,
    pub fft_length: usize,
    pub lower_hz: f64,
    pub upper_hz: f64,
    pub lag_frames: usize,
    pub rms_dbfs: InputValue,
    pub std_dbfs: InputValue,
    pub absolute_lag_correlation: InputValue,
    pub temporal_rms_std_db: InputValue,
    pub complete_seconds: usize,
    pub unavailable_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransientInputs {
    pub interval: AnalysisInterval,
    pub envelope_method: String,
    pub attacks: usize,
    pub eligible_attacks: usize,
    pub affected_attacks: usize,
    pub preceding_energy_percent: InputValue,
    pub legacy_all_attack_percent: InputValue,
    pub hf_median_power: InputValue,
    pub click_candidates: usize,
    pub click_candidates_per_minute: InputValue,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInputs {
    pub method: String,
    pub captured_interval: AnalysisInterval,
    pub void_profile: FilteredProfile,
    pub cassette_profile: FilteredProfile,
    pub vinyl_profile: FilteredProfile,
    pub transients: TransientInputs,
    pub quiet_profile: NoiseFloorAnalysis,
    pub quiet_profile_domain: String,
    pub effective_bits_by_channel: Vec<InputValue>,
    pub effective_bits_interval: AnalysisInterval,
    pub silence_hf_ratio: InputValue,
    pub silence_frames: u64,
    pub silence_hf_captured_frames: usize,
    pub fake_hires_bandwidth_candidate: Option<bool>,
    pub excluded_rules: Vec<String>,
}

pub(crate) struct SourceCollector {
    rate: u32,
    frames: u64,
    pcm: Vec<f32>,
    silence: Vec<f32>,
    pending: Vec<f32>,
    run: u64,
    sil_frames: u64,
}
impl SourceCollector {
    pub fn new(rate: u32, frames: u64) -> Self {
        Self {
            rate,
            frames,
            pcm: Vec::with_capacity(frames.min(rate as u64 * 180) as usize),
            silence: Vec::new(),
            pending: Vec::new(),
            run: 0,
            sil_frames: 0,
        }
    }
    pub fn push(&mut self, x: f32) {
        if self.pcm.len() < self.rate as usize * 180 {
            self.pcm.push(x);
        }
        if x.abs() < 0.01 {
            self.run += 1;
            if self.silence.len() + self.pending.len() < self.rate as usize * 30 {
                self.pending.push(x);
            }
        } else {
            self.end_run();
        }
    }
    fn end_run(&mut self) {
        if self.run >= self.rate as u64 / 2 {
            self.sil_frames += self.run;
            self.silence.append(&mut self.pending);
        }
        self.pending.clear();
        self.run = 0;
    }
    pub fn finish(
        mut self,
        cutoff: Option<f64>,
        cliff: Option<f64>,
        quiet: NoiseFloorAnalysis,
        bits: Vec<InputValue>,
        mut control: impl FnMut() -> Result<(), Failure>,
    ) -> Result<SourceInputs, Failure> {
        self.end_run();
        control()?;
        let n = self.pcm.len();
        let ny = self.rate as f64 / 2.0;
        let lo = cutoff
            .filter(|c| *c > 0.0 && *c < ny * 0.93 && ny - 100.0 - (*c + 800.0) >= 400.0)
            .map(|c| c + 800.0);
        let void = profile(
            &self.pcm,
            self.rate,
            lo,
            Some(ny - 100.0),
            false,
            &mut control,
        )?;
        let vinyl = if void.rms_dbfs.value.is_some() {
            void.clone()
        } else {
            profile(
                &self.pcm,
                self.rate,
                cutoff
                    .filter(|c| *c > 0.0 && *c < ny - 2100.0)
                    .map(|c| c + 1000.0),
                Some(ny - 100.0),
                false,
                &mut control,
            )?
        };
        let hiss_lo = cutoff
            .filter(|c| *c < 19000.0)
            .map(|c| c + if c < 16000.0 { 1000.0 } else { 500.0 });
        let cassette = profile(
            &self.pcm[..n.min(self.rate as usize * 60)],
            self.rate,
            hiss_lo,
            Some(20000.0f64.min(ny - 100.0)),
            true,
            &mut control,
        )?;
        let transients = transients(&self.pcm, self.rate, &mut control)?;
        let music = if self.frames >= self.rate as u64 * 40 {
            &self.pcm[self.rate as usize * 10..self.rate as usize * 40]
        } else {
            &self.pcm[..]
        };
        let ratio = if self.sil_frames >= self.rate as u64 * 2 {
            let a = hf_energy(music, self.rate, &mut control)?;
            let b = hf_energy(&self.silence, self.rate, &mut control)?;
            a.zip(b)
                .filter(|(a, _)| *a > 1.2e-8)
                .map(|(a, b)| b / (a + 1e-12))
        } else {
            None
        };
        let fake = cutoff
            .zip(cliff)
            .zip(void.rms_dbfs.value)
            .map(|((c, d), v)| {
                self.rate >= 88200 && c > 0.0 && c < ny * 0.6 && d > 25.0 && v < -80.0
            });
        Ok(SourceInputs{method:"python-c6ecce2-source-f32-v1; D02/D03/D05".into(),captured_interval:AnalysisInterval{start_frame:0,end_frame:n as u64},void_profile:void,cassette_profile:cassette,vinyl_profile:vinyl,transients,quiet_profile:quiet,quiet_profile_domain:"first 30s exact native f64 mono/mean of native channels; corrected zero-block indices".into(),effective_bits_by_channel:bits,effective_bits_interval:AnalysisInterval{start_frame:0,end_frame:self.frames.min(self.rate as u64*30)},silence_hf_ratio:value(ratio,"ratio"),silence_frames:self.sil_frames,silence_hf_captured_frames:self.silence.len(),fake_hires_bandwidth_candidate:fake,excluded_rules:vec!["D05: negating absolute band correlation is not frequency mirroring; +10/+15 excluded".into()]})
    }
}
/// scipy.fft.next_fast_len(n), complex transform: factors 2,3,5,7,11.
fn fast_len(n: usize) -> usize {
    let mut k = n.max(1);
    loop {
        let mut q = k;
        for p in [2, 3, 5, 7, 11] {
            while q % p == 0 {
                q /= p;
            }
        }
        if q == 1 {
            return k;
        }
        k += 1;
    }
}
fn filtered(
    x: &[f32],
    rate: u32,
    lo: f64,
    hi: f64,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<(Vec<f32>, usize), Failure> {
    let n = fast_len(x.len());
    let mut planner = FftPlanner::<f32>::new();
    control()?;
    let forward = planner.plan_fft_forward(n);
    let inverse = planner.plan_fft_inverse(n);
    let scratch_len = forward
        .get_inplace_scratch_len()
        .max(inverse.get_inplace_scratch_len());
    if scratch_len > 2 * n {
        return Err(Failure::ResourceLimit(
            "FFT scratch exceeds 2N complex samples".into(),
        ));
    }
    let mut data = vec![Complex32::default(); n];
    let mut scratch = vec![
        Complex32::default();
        forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len())
    ];
    for (dst, src) in data.iter_mut().zip(x) {
        dst.re = *src;
    }
    control()?;
    forward.process_with_scratch(&mut data, &mut scratch);
    control()?;
    for (i, z) in data.iter_mut().enumerate() {
        let f = i.min(n - i) as f64 * rate as f64 / n as f64;
        if f < lo || f > hi {
            *z = Complex32::default();
        }
    }
    inverse.process_with_scratch(&mut data, &mut scratch);
    control()?;
    Ok((data[..x.len()].iter().map(|z| z.re / n as f32).collect(), n))
}
fn mean(x: &[f32]) -> f64 {
    x.iter().map(|v| *v as f64).sum::<f64>() / x.len().max(1) as f64
}
fn variance(x: &[f32], m: f64) -> f64 {
    x.iter().map(|v| (*v as f64 - m).powi(2)).sum::<f64>() / x.len().max(1) as f64
}
fn correlation(a: &[f32], b: &[f32]) -> Option<f64> {
    let (ma, mb) = (mean(a), mean(b));
    let (va, vb) = (variance(a, ma), variance(b, mb));
    (va > 1e-20 && vb > 1e-20).then(|| {
        (a.iter()
            .zip(b)
            .map(|(a, b)| (*a as f64 - ma) * (*b as f64 - mb))
            .sum::<f64>()
            / a.len() as f64
            / (va * vb).sqrt())
        .abs()
        .min(1.0)
    })
}
pub(crate) fn profile(
    x: &[f32],
    rate: u32,
    lo: Option<f64>,
    hi: Option<f64>,
    cassette: bool,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<FilteredProfile, Failure> {
    let lag = if cassette {
        50usize.max((100.0 * rate as f64 / 44100.0).round_ties_even() as usize)
    } else {
        25usize.max((50.0 * rate as f64 / 44100.0).round_ties_even() as usize)
    };
    let mut out = FilteredProfile {
        interval: AnalysisInterval {
            start_frame: 0,
            end_frame: x.len() as u64,
        },
        fft_length: 0,
        lower_hz: lo.unwrap_or(0.0),
        upper_hz: hi.unwrap_or(0.0),
        lag_frames: lag,
        rms_dbfs: value(None, "dBFS"),
        std_dbfs: value(None, "dBFS"),
        absolute_lag_correlation: value(None, "absolute Pearson"),
        temporal_rms_std_db: value(None, "dB"),
        complete_seconds: x.len() / rate as usize,
        unavailable_reason: Some("Missing cutoff, empty band or insufficient samples".into()),
    };
    if x.is_empty()
        || !lo
            .zip(hi)
            .is_some_and(|(l, h)| l >= 0.0 && h > l && h < rate as f64 / 2.0)
    {
        return Ok(out);
    }
    let (y, n) = filtered(x, rate, lo.unwrap(), hi.unwrap(), control)?;
    out.fft_length = n;
    let power = y.iter().map(|x| f64::from(*x * *x)).sum::<f64>() / y.len() as f64;
    let rms = power.sqrt();
    let std = variance(&y, mean(&y)).sqrt();
    out.rms_dbfs = value(Some(20.0 * (rms + 1e-12).log10()), "dBFS");
    out.std_dbfs = value(Some(20.0 * (std + 1e-12).log10()), "dBFS");
    if y.len() > lag * 2 {
        out.absolute_lag_correlation = value(
            correlation(&y[..y.len() - lag], &y[lag..]),
            "absolute Pearson",
        );
    }
    if out.complete_seconds >= 2 {
        let levels: Vec<f64> = y
            .chunks_exact(rate as usize)
            .map(|s| {
                20.0 * ((s.iter().map(|v| f64::from(*v * *v)).sum::<f64>() / s.len() as f64).sqrt()
                    + 1e-12)
                    .log10()
            })
            .collect();
        let avg = levels.iter().sum::<f64>() / levels.len() as f64;
        out.temporal_rms_std_db = value(
            (std > 1e-10).then(|| {
                (levels.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / levels.len() as f64).sqrt()
            }),
            "dB",
        );
    }
    out.unavailable_reason = None;
    Ok(out)
}
fn hf_energy(
    x: &[f32],
    rate: u32,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<Option<f64>, Failure> {
    if x.len() < 1024 || rate as f64 / 2.0 - 100.0 <= 16000.0 {
        return Ok(None);
    }
    control()?;
    let n = x.len();
    let fft = FftPlanner::<f64>::new().plan_fft_forward(n);
    let mut data: Vec<Complex64> = x
        .iter()
        .enumerate()
        .map(|(i, v)| {
            Complex64::new(
                *v as f64 * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (n - 1) as f64).cos()),
                0.0,
            )
        })
        .collect();
    if fft.get_inplace_scratch_len() > 2 * n {
        return Err(Failure::ResourceLimit(
            "HF FFT scratch exceeds 2N complex samples".into(),
        ));
    }
    let mut scratch = vec![Complex64::default(); fft.get_inplace_scratch_len()];
    fft.process_with_scratch(&mut data, &mut scratch);
    control()?;
    Ok(Some(
        data[..=n / 2]
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let f = *i as f64 * rate as f64 / n as f64;
                f >= 16000.0 && f <= 22000.0f64.min(rate as f64 / 2.0 - 100.0)
            })
            .map(|(_, v)| v.norm_sqr())
            .sum::<f64>()
            / (n as f64).powi(2),
    ))
}
fn smooth(
    x: &[f64],
    rate: u32,
    seconds: f64,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<Vec<f64>, Failure> {
    if x.is_empty() {
        return Ok(vec![]);
    }
    let width = ((seconds * rate as f64) as usize) | 1;
    let half = width / 2;
    let mut sum = 0.0;
    for k in 0..width {
        sum += x[k.saturating_sub(half).min(x.len() - 1)].abs();
    }
    let mut out = Vec::with_capacity(x.len());
    for i in 0..x.len() {
        if i % 65536 == 0 {
            control()?;
        }
        out.push(sum / width as f64 * std::f64::consts::FRAC_PI_2);
        let remove = i.saturating_sub(half);
        let add = (i + half + 1).min(x.len() - 1);
        sum += x[add].abs() - x[remove].abs();
    }
    Ok(out)
}
fn median(x: &mut [f64]) -> Option<f64> {
    if x.is_empty() {
        return None;
    }
    let n = x.len();
    let (_, m, _) = x.select_nth_unstable_by(n / 2, |a, b| a.total_cmp(b));
    let upper = *m;
    if n % 2 == 1 {
        Some(upper)
    } else {
        Some((x[..n / 2].iter().copied().fold(f64::NEG_INFINITY, f64::max) + upper) * 0.5)
    }
}
fn peaks(
    x: &[f64],
    height: f64,
    distance: usize,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<Vec<usize>, Failure> {
    let mut p = vec![];
    let mut i = 1;
    while i + 1 < x.len() {
        if i % 65536 == 0 {
            control()?;
        }
        if x[i - 1] < x[i] {
            let start = i;
            while i + 1 < x.len() && x[i + 1] == x[start] {
                i += 1;
                if i % 65536 == 0 {
                    control()?;
                }
            }
            if i + 1 < x.len() && x[i + 1] < x[start] && x[start] >= height {
                p.push((start + i) / 2);
            }
        }
        i += 1;
    }
    p.sort_by(|a, b| x[*b].total_cmp(&x[*a]).then(b.cmp(a)));
    let mut selected = std::collections::BTreeSet::new();
    for (index, p) in p.into_iter().enumerate() {
        if index % 65536 == 0 {
            control()?;
        }
        if !selected
            .range(
                p.saturating_sub(distance.saturating_sub(1))
                    ..=p.saturating_add(distance.saturating_sub(1)),
            )
            .any(|_| true)
        {
            selected.insert(p);
        }
    }
    Ok(selected.into_iter().collect())
}
// Fourth-order Butterworth via analog poles, bilinear transform. f64 SOS filtering.
fn butter(
    x: &[f32],
    rate: u32,
    lo: f64,
    hi: Option<f64>,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<Vec<f64>, Failure> {
    let fs = rate as f64;
    let warp = |f: f64| 2.0 * fs * (std::f64::consts::PI * f / fs).tan();
    let w = warp(lo);
    let mut poles = vec![];
    for k in 0..4 {
        let p = Complex64::from_polar(1.0, std::f64::consts::PI * (2 * k + 5) as f64 / 8.0);
        if let Some(h) = hi {
            let wh = warp(h);
            let b = wh - w;
            let a = p * b / 2.0;
            let root = (a * a - Complex64::new(w * wh, 0.0)).sqrt();
            poles.push(a + root);
            poles.push(a - root);
        } else {
            poles.push(Complex64::new(w, 0.0) / p);
        }
    }
    let digital: Vec<_> = poles
        .iter()
        .map(|p| (Complex64::new(2.0 * fs, 0.0) + p) / (Complex64::new(2.0 * fs, 0.0) - p))
        .collect();
    let mut upper: Vec<_> = digital.iter().copied().filter(|p| p.im > 0.0).collect();
    upper.sort_by(|a, b| a.norm_sqr().total_cmp(&b.norm_sqr()));
    let mut sections = vec![];
    for (i, p) in upper.iter().enumerate() {
        let sign = if hi.is_some() && i < upper.len() / 2 {
            1.0
        } else {
            -1.0
        };
        sections.push([1.0, 2.0 * sign, 1.0, 1.0, -2.0 * p.re, p.norm_sqr()]);
    }
    let omega = hi
        .map(|h| 2.0 * (warp(lo) * warp(h)).sqrt().atan2(2.0 * fs))
        .unwrap_or(std::f64::consts::PI);
    let z = Complex64::from_polar(1.0, -omega);
    let gain = sections
        .iter()
        .map(|s| (s[0] + s[1] * z + s[2] * z * z) / (s[3] + s[4] * z + s[5] * z * z))
        .product::<Complex64>()
        .norm();
    for b in &mut sections[0][..3] {
        *b /= gain;
    }
    let mut state = vec![[0.0; 2]; sections.len()];
    x.iter()
        .enumerate()
        .map(|(index, v)| {
            if index % 65536 == 0 {
                control()?;
            }
            let mut y = *v as f64;
            for (s, z) in sections.iter().zip(&mut state) {
                let out = s[0] * y + z[0];
                z[0] = s[1] * y - s[4] * out + z[1];
                z[1] = s[2] * y - s[5] * out;
                y = out;
            }
            Ok(y)
        })
        .collect()
}
pub(crate) fn transients(
    x: &[f32],
    rate: u32,
    control: &mut impl FnMut() -> Result<(), Failure>,
) -> Result<TransientInputs, Failure> {
    control()?;
    let raw: Vec<f64> = x.iter().map(|x| *x as f64).collect();
    let env = smooth(&raw, rate, 0.001, control)?;
    drop(raw);
    let attacks = peaks(
        &env,
        10f64.powf(-3.0 / 20.0),
        (rate as f64 * 0.05) as usize,
        control,
    )?;
    drop(env);
    control()?;
    let hp = butter(x, rate, 1000.0, None, control)?;
    let mut env = smooth(&hp, rate, 0.0005, control)?;
    drop(hp);
    let mut copy = env.clone();
    let med = median(&mut copy).unwrap_or(0.0);
    drop(copy);
    let clicks = peaks(&env, med * 3.0, (rate as f64 * 0.01) as usize, control)?.len();
    env.clear();
    drop(env);
    control()?;
    let mut eligible = 0;
    let mut affected = 0;
    let mut baseline = None;
    if rate as f64 / 2.0 - 100.0 > 10000.0 {
        let hf = butter(
            x,
            rate,
            10000.0,
            Some(20000.0f64.min(rate as f64 / 2.0 - 100.0)),
            control,
        )?;
        let mut squares: Vec<_> = hf.iter().map(|x| x * x).collect();
        baseline = median(&mut squares);
        drop(squares);
        let pre = (rate as f64 * 0.02) as usize;
        let post = (rate as f64 * 0.01) as usize;
        for p in &attacks {
            if *p < pre + post {
                continue;
            }
            eligible += 1;
            let e = hf[p - pre..p - post].iter().map(|x| x * x).sum::<f64>() / (pre - post) as f64;
            if baseline.is_some_and(|b| e > b * 3.0) {
                affected += 1;
            }
        }
    }
    control()?;
    Ok(TransientInputs{interval:AnalysisInterval{start_frame:0,end_frame:x.len() as u64},envelope_method:"nearest-edge centered odd-width mean(abs(x))*pi/2; Butterworth4 causal SOS; D03 eligible denominator".into(),attacks:attacks.len(),eligible_attacks:eligible,affected_attacks:affected,preceding_energy_percent:value((eligible>0&&baseline.is_some_and(|b|b>1e-24)).then(||100.0*affected as f64/eligible as f64),"percent"),legacy_all_attack_percent:value((!attacks.is_empty()&&baseline.is_some()).then(||100.0*affected as f64/attacks.len() as f64),"percent"),hf_median_power:value(baseline,"normalized power"),click_candidates:clicks,click_candidates_per_minute:value((!x.is_empty()&&med>1e-12).then(||clicks as f64*60.0*rate as f64/x.len() as f64),"events/minute")})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64, t: f64) {
        assert!((a - b).abs() <= t, "{a} != {b}, tolerance {t}");
    }
    #[test]
    fn pinned_filtered_and_transient_controls() {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/reference_profiles.json"))
                .unwrap();
        for c in oracle["profiles"].as_array().unwrap() {
            let rate = c["rate"].as_u64().unwrap() as u32;
            let kind = c["kind"].as_str().unwrap();
            let n = (c["seconds"].as_f64().unwrap() * rate as f64) as usize;
            let mut x: Vec<f32> = (0..n.min(rate as usize * 180))
                .map(|i| {
                    let u = (((i as u64) * 1664525 + 1013904223) & 0xffffffff) as f64
                        / 4294967296.0
                        - 0.5;
                    let v = (u * 0.1) as f32;
                    if kind == "zero" {
                        0.0
                    } else if kind == "events" {
                        v * 0.001
                    } else {
                        v
                    }
                })
                .collect();
            if kind == "events" {
                for sec in [0.01, 0.7, 1.5, 2.3] {
                    let p = (rate as f64 * sec) as usize;
                    for v in &mut x[p..p + (rate as f64 * 0.004) as usize] {
                        *v = 0.9;
                    }
                }
            }
            let r = profile(
                &x,
                rate,
                c["band"][0].as_f64(),
                c["band"][1].as_f64(),
                false,
                &mut || Ok(()),
            )
            .unwrap();
            assert_eq!(r.fft_length, c["fft_length"].as_u64().unwrap() as usize);
            near(
                r.rms_dbfs.value.unwrap(),
                c["rms_db"].as_f64().unwrap(),
                0.05,
            );
            near(
                r.std_dbfs.value.unwrap(),
                c["std_db"].as_f64().unwrap(),
                0.05,
            );
            if let Some(v) = r.absolute_lag_correlation.value {
                near(v, c["autocorrelation"].as_f64().unwrap(), 0.003);
            }
            if let Some(v) = r.temporal_rms_std_db.value {
                near(v, c["temporal_std"].as_f64().unwrap(), 0.05);
            }
            let t = transients(&x, rate, &mut || Ok(())).unwrap();
            assert_eq!(
                t.attacks,
                c["peaks"].as_array().unwrap().len(),
                "{kind}/{rate} attacks"
            );
            assert_eq!(
                t.click_candidates,
                c["clicks"].as_u64().unwrap() as usize,
                "{kind}/{rate} clicks"
            );
            assert_eq!(t.eligible_attacks, c["eligible"].as_u64().unwrap() as usize);
            assert_eq!(t.affected_attacks, c["affected"].as_u64().unwrap() as usize);
            if let Some(v) = t.preceding_energy_percent.value {
                near(v, c["preceding_pct"].as_f64().unwrap(), 0.1);
            }
            if kind == "zero" {
                assert!(r.absolute_lag_correlation.value.is_none());
                assert!(t.preceding_energy_percent.value.is_none());
            }
        }
    }
    #[test]
    fn invalid_bands_and_cancellation() {
        let r = profile(
            &[0.0; 100],
            8000,
            Some(3000.0),
            Some(2000.0),
            false,
            &mut || Ok(()),
        )
        .unwrap();
        assert!(r.rms_dbfs.value.is_none());
        assert!(r.unavailable_reason.is_some());
        assert!(
            profile(
                &[1.0; 100],
                48000,
                Some(1000.0),
                Some(2000.0),
                false,
                &mut || Err(Failure::Decode("cancel control".into()))
            )
            .is_err()
        );
    }
}
