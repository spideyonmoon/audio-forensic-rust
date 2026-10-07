//! Versioned, uncalibrated interpretation of saved, bound reference inputs.
//! Does not modify native measurement reports or infer calibrated probabilities.
use crate::{
    FileStatus,
    model::AnalysisInterval,
    reference_inputs::{REFERENCE_INPUT_METHOD, REFERENCE_INPUT_VERSION, ReferenceAnalysis},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const METHOD: &str = "python-reference-c6ecce2-v1";
pub const COMMIT: &str = "c6ecce2296256b516709d87088896d1be913908c";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub version: u32,
    pub status: String,
    pub value: Option<f64>,
    pub unit: String,
    pub domain: String,
    pub channel_indices: Vec<usize>,
    pub intervals: Vec<AnalysisInterval>,
    pub sample_rate: u32,
    pub algorithm: String,
    pub caveats: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Scores {
    pub lossy: i32,
    pub natural: i32,
    pub net: i32,
    pub max: i32,
    pub heuristic: i32,
    pub main: i32,
    pub raw_lossy_pct: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTrace {
    pub id: String,
    pub python_source: String,
    pub input_feature_ids: Vec<String>,
    pub operands: BTreeMap<String, Option<f64>>,
    pub predicate: String,
    pub state: String,
    pub effect: String,
    pub before: Scores,
    pub after: Scores,
    pub causal_veto_rule: Option<String>,
    pub result: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub matched: Option<bool>,
    pub display: String,
    pub rule_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceAssessment {
    pub assessment_version: u32,
    pub method_id: String,
    pub contract_version: u32,
    pub calibration_status: String,
    pub reference_commit: String,
    pub status: String,
    pub input_binding: serde_json::Value,
    pub features: BTreeMap<String, Feature>,
    pub scores: Option<Scores>,
    pub reference_label: Option<String>,
    pub display_summary: String,
    pub source_candidates: Vec<Candidate>,
    pub depth_candidates: Vec<Candidate>,
    pub rules: Vec<RuleTrace>,
    pub deviations: Vec<String>,
    pub missing_inputs: Vec<String>,
    pub caveats: Vec<String>,
    pub legacy_outputs: serde_json::Value,
}

fn empty(status: &str, why: &str) -> ReferenceAssessment {
    ReferenceAssessment { assessment_version:1, method_id:METHOD.into(), contract_version:1,
        calibration_status:"uncalibrated".into(), reference_commit:COMMIT.into(), status:status.into(),
        input_binding:serde_json::Value::Null, features:BTreeMap::new(), scores:None, reference_label:None,
        display_summary:why.into(), source_candidates:vec![], depth_candidates:vec![], rules:vec![],
        deviations:(1..=12).map(|n|format!("D{n:02}")).collect(), missing_inputs:vec![],
        caveats:vec!["Uncalibrated pinned reference method; scores are not probabilities or proof of authenticity, codec ancestry, original depth or physical source.".into(), "No indicators does not establish lossless ancestry. Native measurements retain INCONCLUSIVE ancestry and a null evidence index.".into()], legacy_outputs:serde_json::Value::Null }
}

/// Evaluate only the supplied saved result. No decoding, I/O, or mutation occurs.
pub fn assess_reference(input: &ReferenceAnalysis) -> ReferenceAssessment {
    let m = &input.measurement;
    if m.status != FileStatus::Analyzed {
        return empty(
            match m.status {
                FileStatus::Failed => "failed",
                FileStatus::Unsupported => "unsupported",
                FileStatus::Cancelled => "cancelled",
                FileStatus::TimedOut => "timed_out",
                _ => unreachable!(),
            },
            "Reference interpretation unavailable: analysis did not complete.",
        );
    }
    if m.stream.as_ref().is_some_and(|s| {
        matches!(
            s.codec.to_ascii_uppercase().as_str(),
            "DSD" | "DSD_LSBF" | "DSD_MSBF"
        )
    }) {
        return empty(
            "unsupported",
            "Native DSD: PCM ancestry interpretation is inapplicable (D08).",
        );
    }
    let (Some(r), Some(c), Some(s)) = (&input.reference_inputs, &m.coverage, &m.stream) else {
        return empty(
            "inconclusive",
            "Reference interpretation requires bound reference inputs, coverage and stream identity.",
        );
    };
    if m.schema_version != "0.18.0"
        || !(8000..=384000).contains(&s.sample_rate)
        || !(1..=2).contains(&s.channels)
        || c.start_seconds != 0.
        || c.end_seconds != c.analyzed_frames as f64 / s.sample_rate as f64
        || m.policy_version != "observations-only-v18"
        || r.version != REFERENCE_INPUT_VERSION
        || r.method != REFERENCE_INPUT_METHOD
        || r.reference_commit != COMMIT
        || r.decoded_pcm_sha256 != c.decoded_pcm_sha256
        || r.hash_sample_encoding != c.hash_sample_encoding
        || r.analyzed_frames != c.analyzed_frames
        || r.reached_end != c.reached_end
        || r.sample_rate != s.sample_rate
        || r.processing_passes != 3
        || r.pass_pcm_sha256.len() != 3
        || r.pass_pcm_sha256.iter().any(|h| h != &c.decoded_pcm_sha256)
        || r.basis.interval.start_frame != 0
        || r.basis.interval.end_frame != c.analyzed_frames
    {
        return empty(
            "failed",
            "Unknown versions or mismatched PCM, coverage or input binding.",
        );
    }
    let mut out = empty("available", "");
    out.input_binding = serde_json::json!({"measurement_schema":m.schema_version,"measurement_policy":m.policy_version,"coverage":c,"stream":s,"adapter_version":r.version,"adapter_method":r.method,"source":m.source});
    if matches!(
        s.codec.to_ascii_uppercase().as_str(),
        "DSD" | "DSD_LSBF" | "DSD_MSBF"
    ) {
        out.status = "unsupported".into();
        out.display_summary =
            "Native DSD: PCM ancestry interpretation is inapplicable (D08).".into();
        return out;
    }
    let full = vec![r.basis.interval.clone()];
    let base = r.base.interval.clone().into_iter().collect::<Vec<_>>();
    let cap = vec![r.source.captured_interval.clone()];
    let mut put = |id: &str,
                   value: Option<f64>,
                   unit: &str,
                   domain: &str,
                   intervals: &[AnalysisInterval],
                   algorithm: &str| {
        let value = value.filter(|v| v.is_finite());
        out.features.insert(id.into(),Feature{id:id.into(),version:2,status:if value.is_some(){"available"}else{"unavailable"}.into(),value,unit:unit.into(),domain:domain.into(),channel_indices:(0..s.channels).collect(),intervals:intervals.to_vec(),sample_rate:s.sample_rate,algorithm:algorithm.into(),caveats:vec!["D01/D02/D03: see bound reference-input adapter for numerical applicability and cap semantics.".into()]});
    };
    for (id, v) in [
        ("cutoff", &r.base.cutoff_p95_hz),
        ("variance", &r.base.cutoff_variance_hz2),
        ("sharpness", &r.base.sharpness_db_per_bin),
        ("cliff", &r.base.cliff_depth_db),
        ("hf", &r.base.hf_magnitude_ratio),
        ("noise", &r.base.noise_above_cutoff_db),
        ("entropy", &r.base.entropy_bits),
        ("banding", &r.spectral.banding),
        ("side", &r.spectral.side_anomaly),
        ("sparsity", &r.spectral.sparsity),
        ("envelope", &r.spectral.envelope_correlation),
        ("slope", &r.spectral.rolloff_db_per_khz),
    ] {
        put(id, v.value, &v.unit, "reference_mid", &base, &r.method);
    }
    put(
        "rate",
        Some(s.sample_rate as f64),
        "Hz",
        "metadata",
        &full,
        "parsed stream",
    );
    // Pinned source returns false outside this method's spectral geometry.
    let ny = s.sample_rate as f64 / 2.;
    put(
        "dsd_spectrum",
        if s.sample_rate <= 48000 || (30000. / (s.sample_rate as f64 / 4096.)) as usize >= 2049 {
            Some(0.)
        } else {
            r.spectral.dsd_like_spectrum.map(|b| b as u8 as f64)
        },
        "boolean",
        "reference_mid",
        &base,
        &r.method,
    );
    let bin = s.sample_rate as f64 / 4096.;
    let comb_geometry =
        ny >= 16000. && (20000f64.min(ny - 100.) / bin) as usize > (16000. / bin) as usize + 80;
    put(
        "comb",
        if !comb_geometry {
            Some(0.)
        } else {
            r.spectral.comb_pattern.map(|b| b as u8 as f64)
        },
        "boolean",
        "reference_mid",
        &base,
        &r.method,
    );
    let resample_geometry = crate::detectors::resampling::SOURCE_RATES
        .iter()
        .any(|src| *src as f64 / 2. + 1200. <= ny - 200.);
    let resample_available = !resample_geometry || r.spectral.resampling.active_frames >= 8;
    put(
        "resample",
        resample_available.then_some(
            r.spectral
                .ordered_resampling_hit
                .as_ref()
                .map_or(0., |h| h.source_rate as f64),
        ),
        "Hz (0=no hit)",
        "reference_mid",
        &base,
        &r.method,
    );
    put(
        "resample_wall",
        resample_available.then_some(
            r.spectral
                .ordered_resampling_hit
                .as_ref()
                .is_some_and(|h| h.mode == "wall") as u8 as f64,
        ),
        "boolean",
        "reference_mid",
        &base,
        &r.method,
    );
    put(
        "bound",
        r.scatter.average_bound_hz.value,
        "Hz",
        "reference_mid",
        &base,
        &r.method,
    );
    put(
        "phase",
        r.scatter.physical_phase.high_band_phase_entropy_bits,
        "bits",
        "reference_mid",
        &base,
        "adjacent energy-qualified phase; D03",
    );
    for (id, v, interval) in [
        (
            "void",
            &r.source.void_profile.rms_dbfs,
            &r.source.void_profile.interval,
        ),
        (
            "hiss",
            &r.source.cassette_profile.std_dbfs,
            &r.source.cassette_profile.interval,
        ),
        (
            "hiss_corr",
            &r.source.cassette_profile.absolute_lag_correlation,
            &r.source.cassette_profile.interval,
        ),
        (
            "vinyl_noise",
            &r.source.vinyl_profile.rms_dbfs,
            &r.source.vinyl_profile.interval,
        ),
        (
            "vinyl_corr",
            &r.source.vinyl_profile.absolute_lag_correlation,
            &r.source.vinyl_profile.interval,
        ),
        (
            "vinyl_variance",
            &r.source.vinyl_profile.temporal_rms_std_db,
            &r.source.vinyl_profile.interval,
        ),
        (
            "preecho",
            &r.source.transients.preceding_energy_percent,
            &r.source.transients.interval,
        ),
        (
            "clicks",
            &r.source.transients.click_candidates_per_minute,
            &r.source.transients.interval,
        ),
    ] {
        put(
            id,
            v.value,
            &v.unit,
            "reference_mid",
            std::slice::from_ref(interval),
            &r.source.method,
        );
    }
    // No qualifying attacks/runs is an explicit false gate, not a fabricated measurement.
    put(
        "attacks",
        Some(r.source.transients.eligible_attacks as f64),
        "count",
        "reference_mid",
        &cap,
        &r.source.method,
    );
    put(
        "silence_seconds",
        Some(r.source.silence_frames as f64 / s.sample_rate as f64),
        "s",
        "reference_mid",
        &full,
        &r.source.method,
    );
    put(
        "silence_ratio",
        r.source.silence_hf_ratio.value,
        "ratio",
        "reference_mid",
        &full,
        "qualified silence throughout prefix (first 30s captured), music 10–40s or entire short prefix; D03",
    );
    put(
        "aac",
        r.aac_winner.score.value,
        "ratio",
        "reference_mid",
        std::slice::from_ref(&r.aac_winner.interval),
        "reference mono/mid/side winner; see bound basis",
    );
    put(
        "vorbis",
        r.vorbis_winner.score.value,
        "ratio",
        "reference_mid",
        std::slice::from_ref(&r.vorbis_winner.interval),
        "reference reconstructed native lanes; see bound winner",
    );
    for (id, v) in [
        ("header_duration", r.header.duration_mismatch),
        ("header_bitrate", r.header.legacy_bitrate_mismatch),
    ] {
        put(
            id,
            v.map(|b| b as u8 as f64),
            "boolean",
            "metadata",
            &full,
            "header audit only; D07",
        );
    }
    put(
        "mqa",
        m.mqa
            .as_ref()
            .map(|q| !q.candidates.is_empty())
            .map(|b| b as u8 as f64),
        "boolean",
        "native_integer_pcm",
        &full,
        "candidate only; D06",
    );
    if s.channels == 1 {
        let side = out.features.get_mut("side").unwrap();
        side.value = Some(0.);
        side.status = "inapplicable".into();
        side.domain = "reference_side".into();
        side.algorithm =
            "Explicit pinned mono side=0 policy input; no measured side channel".into();
    } else {
        out.features.get_mut("side").unwrap().domain = "reference_side".into();
    }
    if !valid_domains_and_scopes(r, s.channels) {
        return empty(
            "failed",
            "Reference adapter domains or intervals do not match the versioned contract.",
        );
    }
    evaluate(&mut out, &s.codec, &r.segments.probes);
    depth(&mut out, s.bits_per_sample, s.integer_pcm, r);
    known_codec_audit(&mut out, &s.codec, &m.source);
    out
}

struct Eval<'a> {
    out: &'a mut ReferenceAssessment,
    scores: Scores,
    missing: BTreeSet<String>,
}
impl Eval<'_> {
    #[allow(clippy::too_many_arguments)] // Explicit frozen rule metadata and causal veto.
    fn run(
        &mut self,
        id: &str,
        source: &str,
        predicate: &str,
        effect: &str,
        veto: Option<bool>,
        cause: &str,
        f: impl FnOnce(&mut Read<'_>) -> Option<i32>,
    ) -> Option<i32> {
        let mut read = Read {
            features: &self.out.features,
            used: BTreeMap::new(),
        };
        let (value, state) = match veto {
            Some(true) => (Some(0), "vetoed"),
            None => (None, "missing_input"),
            Some(false) => {
                let v = f(&mut read);
                (
                    v,
                    match v {
                        None => "missing_input",
                        Some(0) => "not_triggered",
                        _ => "applied",
                    },
                )
            }
        };
        let before = self.scores.clone();
        if let Some(n) = value {
            match effect {
                "lossy" => self.scores.lossy += n,
                "natural" => self.scores.natural += n,
                "add" => self.scores.main += n,
                "floor" if n > 0 => self.scores.main = self.scores.main.max(n),
                _ => (),
            }
        }
        if value.is_none() && effect != "label_only" {
            let n = self.missing.len();
            for (key, v) in &read.used {
                if v.is_none() {
                    self.missing.insert(key.clone());
                }
            }
            if veto.is_none() {
                self.missing.insert(format!("{cause}: unresolved veto"));
            }
            if self.missing.len() == n {
                self.missing
                    .insert(format!("{id}: unavailable prerequisite"));
            }
        }
        self.out.rules.push(RuleTrace {
            id: id.into(),
            python_source: format!("audio_forensic.py:{source}"),
            input_feature_ids: read.used.keys().cloned().collect(),
            operands: read.used,
            predicate: predicate.into(),
            state: state.into(),
            effect: match effect {
                "lossy" | "natural" => "add".into(),
                "add" if value.is_some_and(|v| v < 0) => "subtract".into(),
                _ => effect.into(),
            },
            before,
            after: self.scores.clone(),
            causal_veto_rule: (!matches!(veto, Some(false))).then(|| cause.into()),
            result: value,
        });
        value
    }
    fn rule(
        &mut self,
        id: &str,
        source: &str,
        predicate: &str,
        effect: &str,
        f: impl FnOnce(&mut Read<'_>) -> Option<i32>,
    ) -> Option<i32> {
        self.run(id, source, predicate, effect, Some(false), "", f)
    }
    fn excluded(&mut self, id: &str, source: &str, predicate: &str, keys: &[&str]) {
        let operands = keys
            .iter()
            .map(|k| {
                (
                    k.to_string(),
                    self.out.features.get(*k).and_then(|f| f.value),
                )
            })
            .collect();
        self.out.rules.push(RuleTrace {
            id: id.into(),
            python_source: format!("audio_forensic.py:{source}"),
            input_feature_ids: keys.iter().map(|s| s.to_string()).collect(),
            operands,
            predicate: predicate.into(),
            state: "excluded_by_contract".into(),
            effect: "override".into(),
            before: self.scores.clone(),
            after: self.scores.clone(),
            causal_veto_rule: None,
            result: None,
        });
    }
}
struct Read<'a> {
    features: &'a BTreeMap<String, Feature>,
    used: BTreeMap<String, Option<f64>>,
}
impl Read<'_> {
    fn get(&mut self, key: &str) -> Option<f64> {
        let v = self.features.get(key).and_then(|f| f.value);
        self.used.insert(key.into(), v);
        v
    }
}
fn yes(b: bool, n: i32) -> Option<i32> {
    Some(if b { n } else { 0 })
}
fn candidate(
    out: &mut ReferenceAssessment,
    id: &str,
    matched: Option<bool>,
    display: &str,
    rules: &[&str],
) {
    out.source_candidates.push(Candidate {
        id: id.into(),
        matched,
        display: display.into(),
        rule_ids: rules.iter().map(|s| s.to_string()).collect(),
    });
}

fn evaluate(out: &mut ReferenceAssessment, codec: &str, probes: &[crate::model::SegmentProbe]) {
    use crate::reference_inputs::{reference_fingerprint, segment_vote};
    let rate = out.features["rate"].value.unwrap_or(0.) as u32;
    let ny = rate as f64 / 2.;
    let mut e = Eval {
        out,
        scores: Scores {
            max: 14,
            ..Default::default()
        },
        missing: BTreeSet::new(),
    };
    e.rule(
        "R01",
        "2219-2221",
        "cutoff < 0.85*Nyquist AND cutoff < 18500; +2",
        "lossy",
        |r| yes(r.get("cutoff")? < ny * 0.85 && r.get("cutoff")? < 18500., 2),
    );
    e.rule(
        "R02",
        "2224-2232",
        "sharpness >15 OR (cliff>35 AND cutoff<0.93N): +3; else >8 OR cliff>20: +1",
        "lossy",
        |r| {
            let s = r.get("sharpness")?;
            if s > 15. {
                return Some(3);
            }
            let c = r.get("cutoff")?;
            let d = if c < ny * 0.93 { r.get("cliff")? } else { 0. };
            Some(if d > 35. {
                3
            } else if s > 8. || d > 20. {
                1
            } else {
                0
            })
        },
    );
    e.rule("R03", "2233-2235", "hf < 0.005; +1", "lossy", |r| {
        yes(r.get("hf")? < 0.005, 1)
    });
    e.rule(
        "R04",
        "2236-2241",
        "noise < -70: +3; else < -40: +1",
        "lossy",
        |r| {
            let n = r.get("noise")?;
            Some(if n < -70. {
                3
            } else if n < -40. {
                1
            } else {
                0
            })
        },
    );
    e.rule(
        "R05",
        "2242-2244",
        "cutoff < 0.85N AND variance <1000; +1",
        "lossy",
        |r| {
            if r.get("cutoff")? >= ny * 0.85 {
                return Some(0);
            }
            yes(r.get("variance")? < 1000., 1)
        },
    );
    e.rule(
        "R06",
        "2245-2250",
        "banding>0.92 AND cutoff<0.80N: +1; side>0.60: +2",
        "lossy",
        |r| {
            let b = if r.get("cutoff")? < ny * 0.8 {
                (r.get("banding")? > 0.92) as i32
            } else {
                0
            };
            Some(b + 2 * (r.get("side")? > 0.60) as i32)
        },
    );
    e.rule(
        "R07",
        "2252-2265",
        "non-DSD: hf>0.05 with cutoff>0.85N, noise>-50, entropy>8.5 with cutoff>0.85N; +1 each",
        "natural",
        |r| {
            if r.get("dsd_spectrum")? != 0. {
                return Some(0);
            }
            let mut n = (r.get("noise")? > -50.) as i32;
            if r.get("cutoff")? > ny * 0.85 {
                n += (r.get("hf")? > 0.05) as i32 + (r.get("entropy")? > 8.5) as i32;
            }
            Some(n)
        },
    );
    e.rule(
        "R08",
        "2267-2277",
        "sharpness<5; variance>100000 non-DSD else >10000 with cutoff>0.85N; side<0.2; +1 each",
        "natural",
        |r| {
            let n = (r.get("sharpness")? < 5.) as i32 + (r.get("side")? < 0.2) as i32;
            Some(
                n + if r.get("cutoff")? > ny * 0.85 {
                    (r.get("variance")? > 10000.) as i32
                } else {
                    0
                },
            )
        },
    );
    e.scores.net = (e.scores.lossy - e.scores.natural).max(0);
    e.scores.raw_lossy_pct = (e.scores.lossy as f64 / 14. * 100.).min(100.);
    let scaled = (e.scores.net as f64 * 45. / 14.).round_ties_even() as i32;
    e.rule(
        "R09",
        "2463-2468",
        "net=max(0,lossy-natural); Python ties-to-even round(net*45/14)",
        "add",
        |_| Some(scaled),
    );
    // R09 is pure integer arithmetic: no binary rounding ambiguity except exact 0.5 ties.
    let main = (e.scores.net as f64 * 45. / 14.).round_ties_even() as i32;
    e.scores.main = main;
    if let Some(t) = e.out.rules.last_mut() {
        t.after = e.scores.clone();
        t.state = "applied".into();
    }
    e.rule(
        "R10",
        "2476-2493",
        "ordered resample hit; +45",
        "add",
        |r| yes(r.get("resample")? > 0., 45),
    );
    e.excluded(
        "R11",
        "2498-2504",
        "D07 excludes duration +20 and bitrate +25; raw operands retained",
        &["header_duration", "header_bitrate"],
    );
    let psycho = |r: &mut Read<'_>| -> Option<bool> {
        let c = r.get("cutoff")?;
        Some(
            c < 21000.
                || (c < 20000.
                    && [20200., 19550., 18850., 17450., 16800., 15400., 11100.]
                        .iter()
                        .any(|f| (c - f).abs() <= 300.)),
        )
    };
    e.rule(
        "R12",
        "1769-1800;2506-2511",
        "cutoff<21000 OR MP3 profile; eligible preceding-energy percent>10:+15 else >=5:+10",
        "add",
        |r| {
            if !psycho(r)? || r.get("attacks")? == 0. {
                return Some(0);
            }
            let p = r.get("preecho")?;
            Some(if p > 10. {
                15
            } else if p >= 5. {
                10
            } else {
                0
            })
        },
    );
    e.excluded(
        "R13",
        "1801-1820;2506-2511",
        "D05 excludes negated absolute band-correlation +15/>0.5 or +10/>=0.3; no mirroring claim",
        &[],
    );
    let comb = e
        .rule(
            "R14",
            "1821-1855;2506-2511",
            "psycho gate AND >=2 qualifying comb peaks; +10",
            "add",
            |r| {
                if !psycho(r)? {
                    return Some(0);
                }
                yes(r.get("comb")? != 0., 10)
            },
        )
        .map(|n| n != 0);
    let mut hiss = None;
    let cass=e.rule("R15","2093-2145","cutoff<19000; hiss>-55 AND corr<0.2:+30; -6<slope<-3:+20 else slope<-10:-20; no comb:+15; 50<std<300:+15 else std<30:-10; clamp>=0","label_only",|r|{
        if r.get("cutoff")?>=19000.{hiss=Some(false);return Some(0)}
        let c=r.get("cutoff")?;let lo=c+if c<16000.{1000.}else{500.};
        hiss=if 20000f64.min(ny-100.)<=lo{Some(false)}else{r.get("hiss").and_then(|v|if v<= -55.{Some(false)}else{r.get("hiss_corr").map(|a|a<0.2)})};
        let slope=r.get("slope")?;let std=r.get("variance")?.sqrt();
        Some(((if hiss?{30}else{0})+(if slope> -6.&&slope< -3.{20}else if slope< -10.{-20}else{0})+(if !comb?{15}else{0})+(if std>50.&&std<300.{15}else if std<30.{-10}else{0})).max(0))
    });
    let cassette = cassette_decision(cass, hiss, comb, &e.out.features);
    e.rule(
        "R16",
        "2515-2521",
        "cassette score>=30 AND hiss; -40 and selective downstream veto (D10)",
        "add",
        |_| yes(cassette?, -40),
    );
    if cassette.is_none() {
        e.missing.insert("R15: cassette decision".into());
    }
    let fake = e
        .rule(
            "R17",
            "2536-2540",
            "no resample; rate>=88200; 0<cutoff<0.6N; cliff>25; measured void<-80; +20",
            "add",
            |r| {
                if rate < 88200 {
                    return Some(0);
                }
                let c = r.get("cutoff")?;
                if c <= 0. || c >= ny * 0.6 || r.get("cliff")? <= 25. || r.get("resample")? > 0. {
                    return Some(0);
                }
                yes(r.get("void")? < -80., 20)
            },
        )
        .map(|n| n > 0);
    let cutoff = e.out.features["cutoff"].value;
    let fp = reference_fingerprint(cutoff, rate);
    let wall = policy_segment_wall(
        cutoff,
        e.out.features["cliff"].value,
        e.out.features["void"].value,
        e.out.features["resample_wall"].value.map(|v| v != 0.),
        rate,
    );
    let majority = wall
        .filter(|_| {
            probes
                .iter()
                .filter(|p| p.eligible)
                .all(|p| p.cutoff_hz.is_some())
        })
        .and_then(|w| segment_vote(probes, w).majority_on_eligible_probes);
    let mut derived = e.out.features["cutoff"].clone();
    derived.id = "segment_majority".into();
    derived.unit = "boolean".into();
    derived.value = majority.map(|v| v as u8 as f64);
    derived.status = if majority.is_some() {
        "available"
    } else {
        "unavailable"
    }
    .into();
    derived.intervals = probes.iter().map(|p| p.interval.clone()).collect();
    derived.algorithm =
        "P04 qualified adaptive segment vote, duplicate offsets retained (D03)".into();
    e.out.features.insert(derived.id.clone(), derived);
    e.run("R18","2542-2575","adaptive wall=max(16500,cutoff+400) if cliff>30 AND (void<-85 OR fingerprint) AND not resample wall AND cutoff<22500; positive even-half vote qualifies; +55","add",cassette,"R16",|r|yes(r.get("segment_majority")?!=0.,55));
    let mut anomalies = BTreeMap::new();
    let anomaly_inputs_available = cutoff.is_some_and(|c| {
        let ceiling = (c - 2000.).min(ny * 0.85);
        probes.iter().filter(|p| p.eligible).all(|p| {
            p.cutoff_hz.is_some_and(|pc| {
                pc <= 0.
                    || pc >= ceiling
                    || (p.cliff_db.is_some()
                        && (!(c > ny * 0.93 && pc > 11000.)
                            || (p.high_band_relative_db.is_some() && p.peak_dbfs.is_some())))
            })
        })
    });
    if let Some(c) = cutoff {
        let ceiling = (c - 2000.).min(ny * 0.85);
        for p in probes.iter().filter(|p| p.eligible) {
            if let (Some(pc), Some(cl)) = (p.cutoff_hz, p.cliff_db) {
                let void = c > ny * 0.93
                    && pc > 11000.
                    && pc < ceiling
                    && p.high_band_relative_db.is_some_and(|v| v < -110.)
                    && p.peak_dbfs.is_some_and(|v| v > -40.);
                if (pc > 0. && pc < ceiling && cl > 25.) || void {
                    anomalies.insert(p.interval.start_frame, (pc, cl, void));
                }
            }
        }
    }
    let segment_veto = or(cassette, majority);
    e.run(
        "R19",
        "2576-2612",
        "no majority/cassette; one unique anomalous region; cliff>35 and nearest fingerprint; +25",
        "add",
        segment_veto,
        "R16/R18",
        |_| {
            if !anomaly_inputs_available {
                return None;
            }
            cutoff?;
            Some(
                if anomalies.len() == 1
                    && anomalies.values().any(|(c, d, _)| {
                        *d > 35. && reference_fingerprint(Some(*c), rate).is_some()
                    })
                {
                    25
                } else {
                    0
                },
            )
        },
    );
    e.run(
        "R20",
        "2613-2628",
        "2+ unique regions:+30; 4+:+40; any void:+25; median fingerprint:+15",
        "add",
        segment_veto,
        "R16/R18",
        |_| {
            if !anomaly_inputs_available {
                return None;
            }
            cutoff?;
            if anomalies.len() < 2 {
                return Some(0);
            }
            let mut c = anomalies.values().map(|v| v.0).collect::<Vec<_>>();
            c.sort_by(f64::total_cmp);
            let mid = c.len() / 2;
            let median = if c.len() % 2 == 0 {
                (c[mid - 1] + c[mid]) / 2.
            } else {
                c[mid]
            };
            Some(
                (if c.len() >= 4 { 40 } else { 30 })
                    + 25 * anomalies.values().any(|v| v.2) as i32
                    + 15 * reference_fingerprint(Some(median), rate).is_some() as i32,
            )
        },
    );
    let dirty = e
        .run(
            "R21",
            "1648-1711;2630-2636",
            "qualifying silence>=2s and measured HF ratio>0.3; +50 and early return",
            "add",
            cassette,
            "R16",
            |r| {
                if r.get("silence_seconds")? < 2. || ny - 100. < 16000. {
                    return Some(0);
                }
                yes(r.get("silence_ratio")? > 0.3, 50)
            },
        )
        .map(|n| n > 0);
    let mut vinyl = Some(false);
    e.run("R22","1717-1762;2635-2637","noise<-70 AND cutoff<22500 AND cliff>25:+20; else noise>=-70 AND corr<0.3 AND temporal std<5: vinyl -40, 5<=clicks<=50:-10","add",or(cassette,dirty),"R16/R21",|r|{
        let c=r.get("cutoff")?;let geometry=(c>0.&&c<ny*0.93&&ny-100.-(c+800.)>=400.)||(c>0.&&c<ny-2100.);
        if !geometry{return Some(0)}
        let n=r.get("vinyl_noise")?;
        if n< -70.{return yes(c<22500.&&r.get("cliff")?>25.,20)}
        vinyl=r.get("vinyl_corr").and_then(|v|if v>=0.3{Some(false)}else{r.get("vinyl_variance").map(|v|v<5.)});
        if vinyl?{let clicks=r.get("clicks")?;Some(-40-if (5. ..=50.).contains(&clicks){10}else{0})}else{Some(0)}
    });
    if cassette.is_none() || dirty.is_none() {
        vinyl = None;
    }
    // Missing profile energy can change whether the vinyl veto applies.
    if e.out
        .rules
        .last()
        .is_some_and(|t| t.state == "missing_input")
    {
        vinyl = None;
    }
    e.run("R23","2638-2644","no cassette, no majority, wall<=16500, cutoff>0.85N, no resample, 0<=silence ratio<0.15; -30","add",cassette,"R16",|r|{if r.get("cutoff")?<=ny*0.85||r.get("resample")?>0.||wall?>16500.||majority?||r.get("silence_seconds")?<2.||ny-100.<16000.{return Some(0)}let s=r.get("silence_ratio")?;yes((0. ..0.15).contains(&s),-30)});
    let analog = or(cassette, vinyl);
    e.run(
        "R24",
        "2646-2655",
        "fingerprint AND (void<-85 OR cliff>30); +10",
        "add",
        analog,
        "R16/R22",
        |r| {
            if fp.is_none() {
                return Some(0);
            }
            if r.get("cliff")? > 30. {
                return Some(10);
            }
            yes(r.get("void")? < -85., 10)
        },
    );
    e.run(
        "R25",
        "2664-2666",
        "rate>=40000 AND 0<bound<16500; +25",
        "add",
        analog,
        "R16/R22",
        |r| {
            if rate < 40000 {
                return Some(0);
            }
            let b = r.get("bound")?;
            yes(b > 0. && b < 16500., 25)
        },
    );
    e.rule(
        "R26",
        "2671-2675",
        "cutoff<0.85N AND cutoff<22500 AND cliff>25 AND phase>4.5; +10",
        "add",
        |r| {
            let c = r.get("cutoff")?;
            if c >= ny * 0.85 || c >= 22500. || r.get("cliff")? <= 25. {
                return Some(0);
            }
            yes(r.get("phase")? > 4.5, 10)
        },
    );
    e.rule(
        "R27",
        "2677-2681",
        "cutoff<0.95N AND sparsity>0.30; +10",
        "add",
        |r| {
            if r.get("cutoff")? >= ny * 0.95 {
                return Some(0);
            }
            yes(r.get("sparsity")? > 0.30, 10)
        },
    );
    e.rule(
        "R28",
        "2683-2693",
        "cutoff>16500 AND 0<bound<cutoff-2000 AND envelope<0.15; +15",
        "add",
        |r| {
            let c = r.get("cutoff")?;
            if c <= 16500. {
                return Some(0);
            }
            let b = r.get("bound")?;
            if b <= 0. || b >= c - 2000. {
                return Some(0);
            }
            yes(r.get("envelope")? < 0.15, 15)
        },
    );
    e.run(
        "R29",
        "2695-2711",
        "44.1/48kHz, no source veto; AAC>=0.10:+55 else >=0.06:+15",
        "add",
        analog,
        "R16/R22",
        |r| {
            if ![44100, 48000].contains(&rate) {
                return Some(0);
            }
            let a = r.get("aac")?;
            Some(if a >= 0.10 {
                55
            } else if a >= 0.06 {
                15
            } else {
                0
            })
        },
    );
    e.rule(
        "R30",
        "2713-2729",
        "44.1/48kHz; Vorbis>=0.03: main=max(main,55), independent of source veto",
        "floor",
        |r| {
            if ![44100, 48000].contains(&rate) {
                return Some(0);
            }
            yes(r.get("vorbis")? >= 0.03, 55)
        },
    );
    let before = e.scores.clone();
    e.scores.main = e.scores.main.clamp(0, 100);
    e.scores.heuristic = e.scores.main;
    e.out.rules.push(RuleTrace{id:"R31".into(),python_source:"audio_forensic.py:2371-2409;2731-2733".into(),input_feature_ids:vec![],operands:BTreeMap::new(),predicate:"clamp 0..100; parsed lossy codec first; >=86 before resample/fake; >=55, >=31, >=11, else".into(),state:if e.missing.is_empty(){"applied"}else{"missing_input"}.into(),effect:"clamp".into(),before,after:e.scores.clone(),causal_veto_rule:None,result:e.missing.is_empty().then_some(e.scores.main)});
    e.excluded(
        "R32",
        "_apply_mqa_override;build_report",
        "D06 excludes MQA known codec / main=100 certainty override",
        &["mqa"],
    );
    let resample = e.out.features["resample"].value;
    let known = matches!(
        codec.to_ascii_uppercase().as_str(),
        "AAC"
            | "MP3"
            | "MPEG AUDIO"
            | "OPUS"
            | "VORBIS"
            | "WMA"
            | "AC-3"
            | "E-AC-3"
            | "MUSEPACK"
            | "MPC"
            | "ATRAC"
            | "ATRAC3"
    );
    let (label, summary, legacy) = verdict(
        e.scores.main,
        known,
        resample.unwrap_or(0.),
        fake.unwrap_or(false),
        rate,
        cutoff.unwrap_or(0.),
    );
    candidate(
        e.out,
        "cassette",
        cassette,
        "Cassette-like source candidate under the uncalibrated reference method.",
        &["R15", "R16"],
    );
    candidate(
        e.out,
        "vinyl",
        vinyl,
        "Vinyl-like noise candidate under the uncalibrated reference method.",
        &["R21", "R22"],
    );
    candidate(
        e.out,
        "bandwidth",
        fake,
        "Bandwidth-limited high-rate candidate; recording history is unverified.",
        &["R17"],
    );
    candidate(
        e.out,
        "resampling",
        resample.map(|v| v > 0.),
        "Sample-rate conversion candidate under the reference method.",
        &["R10"],
    );
    let injection = e
        .out
        .rules
        .iter()
        .find(|t| t.id == "R28")
        .map(|t| t.state.as_str());
    let injected = match injection {
        Some("applied") => Some(true),
        Some("not_triggered") => Some(false),
        _ => None,
    };
    candidate(
        e.out,
        "independent_hf",
        injected,
        "Independent high-frequency envelope candidate; added noise is one possible explanation.",
        &["R28"],
    );
    e.out.legacy_outputs = serde_json::json!({"qualification":"Audit-only Python wording; not validated claims or calibrated confidence. Before/after ledgers accumulate only available rule effects; when partial they are not composite scores.","cassette_score":cass,"cassette_score_only_alias":cass.map(|n|n>=30),"cassette_hiss":hiss,"segment_wall_hz":wall,"segment_majority":majority,"anomalous_regions":anomalies,"reference_label":if e.missing.is_empty(){Some(label)}else{None},"primary_verdict":if e.missing.is_empty(){Some(legacy)}else{None},"net_confidence_pct":if e.missing.is_empty(){Some(e.scores.main)}else{None}});
    if e.missing.is_empty() {
        e.out.scores = Some(e.scores.clone());
        e.out.reference_label = Some(label.into());
        e.out.display_summary = format!("Uncalibrated reference method: {summary}");
    } else {
        e.out.status = "partial".into();
        e.out.display_summary="Reference interpretation is partial: required rule inputs are unavailable. Completed candidate observations remain available.".into();
        e.out.missing_inputs = e.missing.into_iter().collect();
    }
    audit_interpretations(e.out, ny);
    enrich_trace(e.out, probes, cass, hiss, wall, vinyl);
}
fn or(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}
fn and(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    or(a.map(|v| !v), b.map(|v| !v)).map(|v| !v)
}
fn policy_segment_wall(
    cutoff: Option<f64>,
    cliff: Option<f64>,
    void: Option<f64>,
    resample_wall: Option<bool>,
    rate: u32,
) -> Option<f64> {
    let ny = rate as f64 / 2.;
    let geometry = cutoff.map(|c| c > 0. && c < ny * 0.93 && ny - 100. - (c + 800.) >= 400.);
    let verified = if geometry == Some(false) {
        Some(false)
    } else {
        void.map(|v| v < -85.)
    };
    let fingerprint = crate::reference_inputs::reference_fingerprint(cutoff, rate).is_some();
    let armed = and(
        and(cliff.map(|d| d > 30.), or(verified, Some(fingerprint))),
        and(resample_wall.map(|w| !w), cutoff.map(|c| c < 22500.)),
    );
    armed.and_then(|a| {
        if a {
            cutoff.map(|c| 16500f64.max(c + 400.))
        } else {
            Some(16500.)
        }
    })
}
fn cassette_decision(
    score: Option<i32>,
    hiss: Option<bool>,
    comb: Option<bool>,
    features: &BTreeMap<String, Feature>,
) -> Option<bool> {
    if hiss == Some(false) {
        return Some(false);
    }
    if let Some(s) = score {
        return Some(s >= 30 && hiss?);
    }
    let value = |key: &str| features.get(key).and_then(|f| f.value);
    let slope = value("slope").map(|v| {
        if v > -6. && v < -3. {
            20
        } else if v < -10. {
            -20
        } else {
            0
        }
    });
    let flutter = value("variance").map(|v| {
        let s = v.sqrt();
        if s > 50. && s < 300. {
            15
        } else if s < 30. {
            -10
        } else {
            0
        }
    });
    let comb = comb.map(|c| if c { 0 } else { 15 });
    let low = if hiss == Some(true) { 30 } else { 0 }
        + slope.unwrap_or(-20)
        + flutter.unwrap_or(-10)
        + comb.unwrap_or(0);
    let high = 30 + slope.unwrap_or(20) + flutter.unwrap_or(15) + comb.unwrap_or(15);
    if high < 30 {
        Some(false)
    } else if hiss == Some(true) && low >= 30 {
        Some(true)
    } else {
        None
    }
}
fn known_codec_audit(out: &mut ReferenceAssessment, codec: &str, source: &str) {
    if out.scores.is_none() {
        return;
    }
    let codec = codec.to_ascii_uppercase();
    if !matches!(
        codec.as_str(),
        "AAC"
            | "MP3"
            | "MPEG AUDIO"
            | "OPUS"
            | "VORBIS"
            | "WMA"
            | "AC-3"
            | "E-AC-3"
            | "MUSEPACK"
            | "MPC"
            | "ATRAC"
            | "ATRAC3"
    ) {
        return;
    }
    let ext = std::path::Path::new(source)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s.to_ascii_lowercase()))
        .unwrap_or_default();
    let mut label = if codec.len() <= 3 {
        codec.clone()
    } else {
        codec
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                format!("{}{}", c.next().unwrap(), c.as_str().to_ascii_lowercase())
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    if format!(".{}", codec.to_ascii_lowercase()) != ext {
        label.push_str(&format!(" in {}", ext.to_ascii_uppercase()));
    }
    let fp = crate::reference_inputs::reference_fingerprint(
        out.features["cutoff"].value,
        out.features["rate"].value.unwrap() as u32,
    )
    .filter(|f| f.codec.to_ascii_uppercase().contains(&codec));
    let mut text = format!("ℹ Natively Lossy Format ({label})");
    if let Some(f) = fp {
        text.push_str(&format!(
            " — matches measured {} {} encoder profile",
            f.codec, f.profile
        ));
    } else if out.scores.as_ref().unwrap().net >= 6 {
        text.push_str(" — severe degradation detected.");
    }
    out.legacy_outputs["primary_verdict"] = text.into();
    out.legacy_outputs["known_lossy_codec"] = codec.into();
}
fn enrich_trace(
    out: &mut ReferenceAssessment,
    probes: &[crate::model::SegmentProbe],
    cass: Option<i32>,
    hiss: Option<bool>,
    wall: Option<f64>,
    vinyl: Option<bool>,
) {
    let template = out.features["cutoff"].clone();
    for (id, value) in [
        ("cassette_score", cass.map(|n| n as f64)),
        ("cassette_hiss", hiss.map(|b| b as u8 as f64)),
        ("adaptive_wall", wall),
        ("vinyl_candidate", vinyl.map(|b| b as u8 as f64)),
    ] {
        let mut f = template.clone();
        f.id = id.into();
        f.value = value;
        f.status = if value.is_some() {
            "available"
        } else {
            "unavailable"
        }
        .into();
        f.unit = match id {
            "cassette_score" => "points",
            "adaptive_wall" => "Hz",
            _ => "boolean",
        }
        .into();
        f.algorithm = METHOD.into();
        let scopes = match id {
            "cassette_score" | "cassette_hiss" => {
                vec!["hiss", "hiss_corr", "slope", "variance", "comb"]
            }
            "vinyl_candidate" => vec![
                "silence_ratio",
                "vinyl_noise",
                "vinyl_corr",
                "vinyl_variance",
                "clicks",
            ],
            _ => vec!["cutoff", "cliff", "void", "resample_wall"],
        };
        f.intervals = scopes
            .iter()
            .filter_map(|k| out.features.get(*k))
            .flat_map(|x| x.intervals.clone())
            .collect();
        out.features.insert(id.into(), f);
    }
    for (n, p) in probes.iter().enumerate() {
        for (suffix, value) in [
            ("cutoff", p.cutoff_hz),
            ("cliff", p.cliff_db),
            ("void", p.high_band_relative_db),
            ("peak", p.peak_dbfs),
            ("eligible", Some(p.eligible as u8 as f64)),
        ] {
            let mut f = template.clone();
            f.id = format!("segment_{n}_{suffix}");
            f.unit = match suffix {
                "cutoff" => "Hz",
                "cliff" | "void" => "dB relative",
                "peak" => "dBFS",
                _ => "boolean",
            }
            .into();
            f.value = value;
            f.status = if value.is_some() {
                "available"
            } else {
                "unavailable"
            }
            .into();
            f.intervals = vec![p.interval.clone()];
            f.algorithm = "P04 qualified two-second segment adapter".into();
            out.features.insert(f.id.clone(), f);
        }
    }
    for t in &mut out.rules {
        let extra: &[&str] = match t.id.as_str() {
            "R09" => &[
                "cutoff",
                "variance",
                "sharpness",
                "cliff",
                "hf",
                "noise",
                "entropy",
                "banding",
                "side",
                "dsd_spectrum",
            ],
            "R15" => &["comb", "rate"],
            "R16" => &["cassette_score", "cassette_hiss"],
            "R18" | "R19" | "R20" => &[
                "cassette_score",
                "cassette_hiss",
                "cutoff",
                "cliff",
                "void",
                "resample_wall",
                "adaptive_wall",
                "segment_majority",
            ],
            "R21" | "R22" => &[
                "cassette_score",
                "cassette_hiss",
                "silence_seconds",
                "silence_ratio",
            ],
            "R23" => &[
                "cassette_score",
                "cassette_hiss",
                "adaptive_wall",
                "segment_majority",
                "resample",
            ],
            "R24" | "R25" | "R29" => {
                &["cassette_score", "cassette_hiss", "vinyl_candidate", "rate"]
            }
            "R31" => &["resample", "cutoff", "cliff", "void", "rate"],
            _ => &[],
        };
        for key in extra {
            if let Some(f) = out.features.get(*key) {
                t.operands.insert((*key).into(), f.value);
            }
        }
        if matches!(t.id.as_str(), "R18" | "R19" | "R20") {
            for (k, f) in out
                .features
                .iter()
                .filter(|(k, _)| k.starts_with("segment_"))
            {
                t.operands.insert(k.clone(), f.value);
            }
        }
        t.input_feature_ids = t.operands.keys().cloned().collect();
    }
}
fn verdict(
    main: i32,
    known: bool,
    resample: f64,
    fake: bool,
    rate: u32,
    cutoff: f64,
) -> (&'static str, &'static str, String) {
    if known {
        return (
            "CAUTION",
            "parsed codec identifies a natively lossy stream; ancestry is a separate question.",
            "ℹ Natively Lossy Format (parsed codec)".into(),
        );
    }
    if main >= 86 {
        return (
            "LIKELY_LOSSY",
            "strong lossy-pattern candidate; source history remains unverified.",
            "✗  Lossy transcode detected — fake lossless (high certainty)".into(),
        );
    }
    if resample > 0. {
        return (
            "SUSPICIOUS",
            "sample-rate conversion candidate; this does not establish lossy ancestry.",
            format!(
                "⚠  Sample-rate counterfeit — upsampled from {} kHz (fake hi-res)",
                resample / 1000.
            ),
        );
    }
    if fake {
        return (
            "SUSPICIOUS",
            "bandwidth-limited high-rate candidate; source history remains unverified.",
            format!(
                "⚠  Fake hi-res — {} kHz container but bandwidth ends at {:.1} kHz (upsampled)",
                rate as f64 / 1000.,
                cutoff / 1000.
            ),
        );
    }
    if main >= 55 {
        (
            "SUSPICIOUS",
            "strong lossy indicators; a transcode is a candidate interpretation.",
            "⚠  Strong lossy indicators — probable transcode".into(),
        )
    } else if main >= 31 {
        (
            "CAUTION",
            "minor spectral indicators; legitimate processing remains possible.",
            "~  Minor spectral quirks — possibly legitimate".into(),
        )
    } else if main >= 11 {
        (
            "LIKELY_GENUINE",
            "few lossy indicators; lossless ancestry is unverified.",
            "✓  Consistent with genuine lossless source".into(),
        )
    } else {
        (
            "GENUINE",
            "no strong lossy indicators; source history is unverified.",
            "✓  No strong lossy indicators detected — source history unverified".into(),
        )
    }
}

fn valid_domains_and_scopes(r: &crate::reference_inputs::ReferenceInputs, channels: usize) -> bool {
    if r.basis.domain
        != if channels == 1 {
            "f32_mono"
        } else {
            "f32_mid_side"
        }
        || r.base.domain != "f32_mono_or_mid_hann4096_hop2048_strict"
        || r.scatter.domain != "f32_mid_active_rows_source_stride_and_20bin_mode"
        || r.segments.domain != "f32_mid_to_f64_hann_2s_fft"
        || r.aac_winner.domain != "f32_mono_mid_side_widened_f64_kbd_mdct"
        || r.vorbis_winner.domain != "f32_mid_side_widened_then_reconstructed_f64_L_R_vorbis_mdct"
        || r.spectral.domain
            != "pinned f32 active mid STFT; side uses unmasked aligned stride-4 frames"
        || r.source.method != "python-c6ecce2-source-f32-v1; D02/D03/D05"
    {
        return false;
    }
    let valid =
        |i: &AnalysisInterval| i.start_frame <= i.end_frame && i.end_frame <= r.analyzed_frames;
    r.base.interval.as_ref().is_none_or(valid)
        && r.scatter.interval.as_ref().is_none_or(valid)
        && valid(&r.aac_winner.interval)
        && valid(&r.vorbis_winner.interval)
        && valid(&r.source.captured_interval)
        && valid(&r.source.effective_bits_interval)
        && valid(&r.source.void_profile.interval)
        && valid(&r.source.cassette_profile.interval)
        && valid(&r.source.vinyl_profile.interval)
        && valid(&r.source.transients.interval)
        && r.source.quiet_profile.interval.as_ref().is_none_or(valid)
        && r.source.captured_interval.end_frame <= r.sample_rate as u64 * 180
        && r.source.cassette_profile.interval.end_frame <= r.sample_rate as u64 * 60
        && r.source.effective_bits_interval.end_frame <= r.sample_rate as u64 * 30
        && r.source.effective_bits_by_channel.len() == channels
        && r.segments.probes.len() <= 36
        && r.segments.probes.iter().all(|p| {
            valid(&p.interval)
                && (!p.eligible
                    || [
                        p.cutoff_hz,
                        p.cliff_db,
                        p.peak_dbfs,
                        p.high_band_relative_db,
                    ]
                    .iter()
                    .all(|v| v.is_none_or(f64::is_finite)))
        })
}

fn audit_interpretations(out: &mut ReferenceAssessment, ny: f64) {
    use crate::reference_labels as l;
    let get = |s: &str| out.features.get(s).and_then(|f| f.value);
    let legit = get("cutoff").map(|c| c > ny * 0.85);
    let labels = serde_json::json!({
        "variance":get("variance").zip(legit).map(|(v,b)|l::variance(v,b)),
        "sharpness":get("sharpness").map(l::sharpness),"hf":get("hf").map(l::hf_ratio),
        "banding":get("banding").map(l::banding),"noise":get("noise").map(l::nf),
        "side":get("side").map(l::side),"entropy":get("entropy").zip(legit).map(|(v,b)|l::entropy(v,b)),
        "bound":get("bound").map(|v|l::bound(ny,v)),
        "phase":get("phase").zip(legit).map(|(v,b)|l::phase_entropy(v,b)),
        "sparsity":get("sparsity").zip(legit).map(|(v,b)|l::sparsity(v,b)),
        "envelope":get("envelope").map(l::ultra_corr),"aac":get("aac").map(l::mdct)
    });
    out.legacy_outputs["interpretations"] = labels;
    let score = out
        .rules
        .last()
        .map(|r| r.after.clone())
        .unwrap_or_default();
    out.rules.push(RuleTrace{id:"R34".into(),python_source:"audio_forensic.py:1250-1326;2084-2089;2738-2757".into(),input_feature_ids:out.features.keys().cloned().collect(),operands:out.features.iter().map(|(k,v)|(k.clone(),v.value)).collect(),predicate:"Exact pinned scalar interpretation thresholds/text are audit-only; absent values remain null (D03/D09). D04/D11 native units/normalization/ReplayGain do not add ancestry points; D12 comparison workflow belongs to P07.".into(),state:"applied".into(),effect:"label_only".into(),before:score.clone(),after:score,causal_veto_rule:None,result:None});
}

fn depth_text(claimed: u32, effective: u32, profile: Option<(f64, bool)>) -> String {
    if effective > 0 && effective <= claimed.saturating_sub(8) {
        return format!(
            "⚠ Upscaled: {claimed}-bit container but only {effective} bits carry signal — clean integer pad from a {effective}-bit source"
        );
    }
    if effective > 0 && effective < claimed {
        return format!(
            "~ {effective} of {claimed} bits exercised — reduced-depth master, bit-shifted gain, or fixed-point chain (not zero-padded)"
        );
    }
    let Some((floor, flat)) = profile else {
        return format!(
            "✓ {claimed}-bit container fully exercised — source depth not independently confirmable"
        );
    };
    // Python formats integral f64 ties to even, including negative 0.5 values.
    let shown = floor.round_ties_even();
    if floor > -86. {
        return format!(
            "✓ {claimed}-bit container fully exercised — noise floor masked by a loud master ({shown:.0} dBFS), source depth not independently confirmable"
        );
    }
    let dr = ((-floor - 1.76) / 6.02).round_ties_even() as i32;
    if floor < -102. {
        let dr = dr.max(claimed as i32);
        return format!(
            "✓ Genuine {claimed}-bit — noise floor at {shown:.0} dBFS confirms content below the 16-bit limit (~{dr}-bit dynamic range)"
        );
    }
    if claimed >= 24 && flat && floor <= -89. {
        return format!(
            "⚠ Effective ~16-bit — {claimed}-bit container but a flat noise floor at {shown:.0} dBFS (the 16-bit dither level) — upsampled from 16-bit"
        );
    }
    if claimed >= 24 {
        let color = if flat {
            "limited dynamic range"
        } else {
            "colored/analog"
        };
        return format!(
            "~ Noise floor {shown:.0} dBFS (~{dr}-bit effective, {color}) — consistent with an analog-sourced or heavily-compressed {claimed}-bit master; source depth unconfirmable"
        );
    }
    format!("✓ {claimed}-bit consistent — noise floor at {shown:.0} dBFS matches the claimed depth")
}

fn depth(
    out: &mut ReferenceAssessment,
    claimed: Option<u32>,
    integer: bool,
    r: &crate::reference_inputs::ReferenceInputs,
) {
    let mut audits = vec![];
    for (id, v, unit, intervals) in [
        (
            "depth_claimed",
            claimed.map(|n| n as f64),
            "bits",
            vec![r.source.effective_bits_interval.clone()],
        ),
        (
            "depth_floor",
            r.source.quiet_profile.nonzero_rms_p015_dbfs,
            "dBFS",
            r.source
                .quiet_profile
                .interval
                .clone()
                .into_iter()
                .collect(),
        ),
        (
            "depth_color",
            r.source.quiet_profile.high_minus_low_db,
            "dB",
            r.source
                .quiet_profile
                .selected_blocks
                .iter()
                .map(|b| AnalysisInterval {
                    start_frame: b.block_index as u64 * r.source.quiet_profile.block_frames as u64,
                    end_frame: (b.block_index as u64 + 1)
                        * r.source.quiet_profile.block_frames as u64,
                })
                .collect(),
        ),
    ] {
        out.features.insert(
            id.into(),
            Feature {
                id: id.into(),
                version: 2,
                status: if v.is_some() {
                    "available"
                } else {
                    "unavailable"
                }
                .into(),
                value: v,
                unit: unit.into(),
                domain: if id == "depth_claimed" {
                    "metadata"
                } else {
                    "native_float_pcm"
                }
                .into(),
                channel_indices: (0..r.source.effective_bits_by_channel.len()).collect(),
                intervals,
                sample_rate: r.sample_rate,
                algorithm: r.source.quiet_profile_domain.clone(),
                caveats: vec![
                    "D02/D03: native f64 channel mean, first 30s, original nonzero-block indices"
                        .into(),
                ],
            },
        );
    }
    for (channel, bits) in r.source.effective_bits_by_channel.iter().enumerate() {
        let id = format!("effective_bits_channel_{channel}");
        out.features.insert(
            id.clone(),
            Feature {
                id: id.clone(),
                version: 2,
                status: if bits.value.is_some() {
                    "available"
                } else {
                    "unavailable"
                }
                .into(),
                value: bits.value,
                unit: "bits".into(),
                domain: "native_integer_pcm".into(),
                channel_indices: vec![channel],
                intervals: vec![r.source.effective_bits_interval.clone()],
                sample_rate: r.sample_rate,
                algorithm: "first-30s exact integer reference counters; D02".into(),
                caveats: vec![],
            },
        );
        let floor = r.source.quiet_profile.nonzero_rms_p015_dbfs;
        let flat = r
            .source
            .quiet_profile
            .high_minus_low_db
            .map(|v| v.abs() < 7.);
        let p = floor.and_then(|f| {
            if !(-102. ..=-86.).contains(&f) || claimed.is_some_and(|c| c < 24) {
                Some((f, flat.unwrap_or(false)))
            } else {
                flat.map(|b| (f, b))
            }
        });
        let effective = bits.value.filter(|v| *v > 0.).map(|v| v as u32);
        let text = claimed
            .zip(effective)
            .filter(|_| integer)
            .map(|(c, b)| depth_text(c, b, p));
        let (matched, display) = match claimed.zip(effective).filter(|_| integer) {
            None => (
                None,
                "Integer source-depth candidate unavailable; no exercised-bit assertion is made."
                    .into(),
            ),
            Some((c, b)) if b <= c.saturating_sub(8) => (
                Some(true),
                format!(
                    "Reference depth candidate: {b} exercised bits in a {c}-bit container, consistent with integer padding; original source depth is unverified."
                ),
            ),
            Some((c, b)) if b < c => (
                Some(true),
                format!(
                    "Reference depth candidate: {b} of {c} bits exercised; reduced depth, gain or fixed-point processing are possible."
                ),
            ),
            Some((c, _)) => {
                let msg = match p {
                    Some((f, _)) if f < -102. => "quiet profile below the reference 16-bit floor",
                    Some((f, true)) if c >= 24 && (-102. ..=-89.).contains(&f) => {
                        "flat quiet profile near the reference 16-bit floor"
                    }
                    Some((f, _)) if f <= -86. => {
                        "exposed quiet profile with limited effective dynamic range"
                    }
                    _ => "noise floor unavailable or masked",
                };
                (
                    Some(false),
                    format!(
                        "Reference depth candidate: {c} bits exercised; {msg}. Original source depth remains unverified."
                    ),
                )
            }
        };
        out.depth_candidates.push(Candidate {
            id: format!("depth_channel_{channel}"),
            matched,
            display,
            rule_ids: vec!["R33".into()],
        });
        audits.push(serde_json::json!({"channel":channel,"legacy_text":text,"profile_floor_dbfs":floor,"profile_flat":flat}));
    }
    out.legacy_outputs["depth"] = serde_json::json!(audits);
    let score = out
        .rules
        .last()
        .map(|r| r.after.clone())
        .unwrap_or_default();
    out.rules.push(RuleTrace{id:"R33".into(),python_source:"audio_forensic.py:806-850".into(),input_feature_ids:out.features.keys().filter(|k|k.starts_with("effective_bits")||k.starts_with("depth_")).cloned().collect(),operands:out.features.iter().filter(|(k,_)|k.starts_with("effective_bits")||k.starts_with("depth_")).map(|(k,v)|(k.clone(),v.value)).collect(),predicate:"Independent depth: pad<=claimed-8; reduced<claimed; masked floor>-86; floor<-102; claimed>=24 AND flat AND floor<=-89; otherwise limited/consistent. Python ties-to-even; D02/D03/D09 qualification.".into(),state:if integer{"applied"}else{"inapplicable"}.into(),effect:"label_only".into(),before:score.clone(),after:score,causal_veto_rule:None,result:None});
    out.rules.sort_by(|a, b| a.id.cmp(&b.id));
}

/// Explicit version dispatch for a saved assessment. Re-assess bound inputs to verify policy.
pub fn read_assessment_json(json: &str) -> Result<ReferenceAssessment, String> {
    let a: ReferenceAssessment = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if a.assessment_version != 1
        || a.method_id != METHOD
        || a.contract_version != 1
        || a.reference_commit != COMMIT
        || a.calibration_status != "uncalibrated"
    {
        return Err("Unsupported reference assessment version/method".into());
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scalar_case(name: &str) -> (ReferenceAssessment, Vec<crate::model::SegmentProbe>) {
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/reference_assessment_v2.json"
        ))
        .unwrap();
        let c = data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap();
        let mut a = empty("available", "");
        for (k, v) in c["features"].as_object().unwrap() {
            a.features.insert(
                k.clone(),
                Feature {
                    id: k.clone(),
                    version: 2,
                    status: "available".into(),
                    value: v.as_f64(),
                    unit: "scalar".into(),
                    domain: "reference_mid".into(),
                    channel_indices: vec![0],
                    intervals: vec![],
                    sample_rate: 48000,
                    algorithm: "frozen fixture".into(),
                    caveats: vec![],
                },
            );
        }
        (a, serde_json::from_value(c["probes"].clone()).unwrap())
    }
    #[test]
    fn missing_inputs_respect_gates_and_excluded_rules() {
        let (mut a, p) = scalar_case("full_band");
        for k in [
            "preecho", "slope", "void", "hiss", "clicks", "phase", "envelope",
        ] {
            a.features.get_mut(k).unwrap().value = None;
        }
        for k in ["header_duration", "header_bitrate", "mqa"] {
            a.features.get_mut(k).unwrap().value = Some(1.);
        }
        evaluate(&mut a, "PCM", &p);
        assert_eq!(a.status, "available");
        assert_eq!(a.scores.unwrap().main, 0);
        let (mut a, p) = scalar_case("full_band");
        a.features.get_mut("hf").unwrap().value = None;
        evaluate(&mut a, "PCM", &p);
        assert_eq!(a.status, "partial");
        assert!(a.scores.is_none());
        assert!(a.legacy_outputs["net_confidence_pct"].is_null());
        let (mut a, p) = scalar_case("cassette_veto");
        a.features.get_mut("aac").unwrap().value = None;
        evaluate(&mut a, "PCM", &p);
        assert_eq!(a.status, "available");
        assert_eq!(
            a.rules.iter().find(|r| r.id == "R29").unwrap().state,
            "vetoed"
        );
        let (mut a, p) = scalar_case("cassette_without_hiss");
        evaluate(&mut a, "PCM", &p);
        assert_eq!(a.legacy_outputs["cassette_score_only_alias"], true);
        assert_eq!(
            a.source_candidates
                .iter()
                .find(|c| c.id == "cassette")
                .unwrap()
                .matched,
            Some(false)
        );
    }
    #[test]
    fn frozen_verdict_precedence_and_rounding() {
        let d: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/reference_verdict_boundaries.json"
        ))
        .unwrap();
        for c in d["cases"].as_array().unwrap() {
            let (label, _, text) = verdict(
                c["main"].as_i64().unwrap() as i32,
                false,
                c["resample"].as_f64().unwrap(),
                c["fake"].as_bool().unwrap(),
                96000,
                20000.,
            );
            assert_eq!(label, c["label"].as_str().unwrap());
            assert_eq!(text, c["text"].as_str().unwrap());
        }
        for (net, expected) in d["rounding"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                (net as f64 * 45. / 14.).round_ties_even() as i64,
                expected.as_i64().unwrap()
            );
        }
        let (mut a, p) = scalar_case("round_half_net7");
        a.features.get_mut("dsd_spectrum").unwrap().value = Some(1.);
        evaluate(&mut a, "PCM", &p);
        assert_eq!(a.scores.as_ref().unwrap().net, 7);
        assert_eq!(a.scores.unwrap().main, 22);
    }
    #[test]
    fn frozen_python_policy_vectors() {
        let traces: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/reference_assessment_traces_v2.json"
        ))
        .unwrap();
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/reference_assessment_v2.json"
        ))
        .unwrap();
        for case in data["cases"].as_array().unwrap() {
            let mut out = empty("available", "");
            for (k, v) in case["features"].as_object().unwrap() {
                out.features.insert(
                    k.clone(),
                    Feature {
                        id: k.clone(),
                        version: 2,
                        status: "available".into(),
                        value: v.as_f64(),
                        unit: "test".into(),
                        domain: "reference_mid".into(),
                        channel_indices: vec![0],
                        intervals: vec![],
                        sample_rate: 48000,
                        algorithm: "frozen scalar oracle".into(),
                        caveats: vec![],
                    },
                );
            }
            let probes: Vec<crate::model::SegmentProbe> =
                serde_json::from_value(case["probes"].clone()).unwrap();
            evaluate(&mut out, "PCM", &probes);
            let expected = &case["expected"];
            assert_eq!(
                out.status, "available",
                "{} {:?}",
                case["name"], out.missing_inputs
            );
            let score = serde_json::to_value(out.scores.as_ref().unwrap()).unwrap();
            for key in [
                "lossy",
                "natural",
                "net",
                "max",
                "main",
                "heuristic",
                "raw_lossy_pct",
            ] {
                assert_eq!(score[key], expected[key], "{} {key}", case["name"]);
            }
            assert_eq!(
                out.reference_label.as_deref(),
                expected["label"].as_str(),
                "{}",
                case["name"]
            );
            assert_eq!(
                out.legacy_outputs["primary_verdict"], expected["text"],
                "{}",
                case["name"]
            );
            assert_eq!(
                out.legacy_outputs["cassette_score"], expected["cassette"],
                "{}",
                case["name"]
            );
            for entry in traces[case["name"].as_str().unwrap()].as_array().unwrap() {
                let id = match entry[0].as_i64().unwrap() {
                    2468 => "R09",
                    2489 => "R10",
                    2505 => "R14",
                    2514 => "R16",
                    2539 => "R17",
                    2574 => "R18",
                    2608 => "R19",
                    2619 => "R20",
                    2633 => "R22",
                    2650 => "R24",
                    2662 => "R25",
                    2670 => "R26",
                    2677 => "R27",
                    2689 => "R28",
                    2703 | 2706 => "R29",
                    2718 => "R30",
                    2726 => "R31",
                    n => panic!("Unmapped pinned score statement {n}"),
                };
                assert_eq!(
                    out.rules.iter().find(|r| r.id == id).unwrap().after.main as i64,
                    entry[1].as_i64().unwrap(),
                    "{} {id}",
                    case["name"]
                );
            }
        }
    }
    #[test]
    fn frozen_python_depth_vectors() {
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/reference_assessment_v2.json"
        ))
        .unwrap();
        for c in data["depth"].as_array().unwrap() {
            assert_eq!(
                depth_text(
                    c["claimed"].as_u64().unwrap() as u32,
                    c["effective"].as_u64().unwrap() as u32,
                    c["floor"]
                        .as_f64()
                        .map(|f| (f, c["flat"].as_bool().unwrap()))
                ),
                c["text"].as_str().unwrap()
            );
        }
    }
}
