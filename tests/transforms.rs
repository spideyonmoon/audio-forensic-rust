use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_path,
};
use serde_json::Value;
use std::path::Path;

fn fixture_pcm(file: &str) -> Vec<f64> {
    use symphonia::core::{audio::SampleBuffer, errors::Error, io::MediaSourceStream};
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file);
    let source = MediaSourceStream::new(
        Box::new(std::fs::File::open(path).unwrap()),
        Default::default(),
    );
    let mut probed = symphonia::default::get_probe()
        .format(
            &Default::default(),
            source,
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
    let params = probed.format.default_track().unwrap().codec_params.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &Default::default())
        .unwrap();
    let mut result = vec![];
    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("{e}"),
        };
        let audio = decoder.decode(&packet).unwrap();
        assert_eq!(audio.spec().channels.count(), 1);
        let mut samples = SampleBuffer::<f64>::new(audio.capacity() as u64, *audio.spec());
        samples.copy_interleaved_ref(audio);
        result.extend_from_slice(samples.samples());
    }
    result
}

fn float_wav(samples: &[f64], channels: u16, rate: u32) -> Vec<u8> {
    let bytes = samples.len() as u32 * 4;
    let mut out = vec![];
    out.extend(b"RIFF");
    out.extend((36 + bytes).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(3u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * channels as u32 * 4).to_le_bytes());
    out.extend((channels * 4).to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    for x in samples {
        out.extend((*x as f32).to_le_bytes());
    }
    out
}

fn run_pcm(samples: &[f64], channels: u16) -> AnalysisReport {
    let report = audio_forensic::analyze_source(
        Box::new(std::io::Cursor::new(float_wav(samples, channels, 44100))),
        "transformed.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    report
}

#[test]
fn vorbis_search_tolerates_trim_gain_and_preserves_native_channels() {
    let encoded = fixture_pcm("grid_q6_44100.flac");
    let shifted: Vec<_> = encoded[137..].iter().map(|x| x * 0.37).collect();
    assert_eq!(run_pcm(&shifted, 1).vorbis[0].status, DetectorStatus::Hit);
    let stereo: Vec<_> = encoded.iter().flat_map(|x| [*x, 0.0]).collect();
    let report = run_pcm(&stereo, 2);
    assert_eq!(report.vorbis[0].status, DetectorStatus::Hit);
    assert_eq!(report.vorbis[1].status, DetectorStatus::Inconclusive);
    let anti_phase: Vec<_> = encoded.iter().flat_map(|x| [*x, -*x]).collect();
    let report = run_pcm(&anti_phase, 2);
    assert!(
        report
            .vorbis
            .iter()
            .all(|v| v.status == DetectorStatus::Hit)
    );
}

#[test]
fn tonal_quiet_and_silent_controls_do_not_invent_a_vorbis_grid() {
    let length = 44100;
    for mode in ["tone", "harmonic", "quiet", "silence"] {
        let samples: Vec<_> = (0..length)
            .map(|i| {
                let t = std::f64::consts::TAU * 440.0 * i as f64 / 44100.0;
                match mode {
                    "tone" => 0.2 * t.sin(),
                    "harmonic" => (1..20).map(|k| 0.2 / k as f64 * (t * k as f64).sin()).sum(),
                    "quiet" => 1e-7 * t.sin(),
                    _ => 0.0,
                }
            })
            .collect();
        let report = run_pcm(&samples, 1);
        assert_ne!(report.vorbis[0].status, DetectorStatus::Hit, "{mode}");
        if matches!(mode, "quiet" | "silence") {
            assert_eq!(report.vorbis[0].zero_excess_score, None);
        }
    }
}

fn run(file: &str, max_seconds: Option<f64>) -> AnalysisReport {
    let report = analyze_path(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(file),
        &AnalysisOptions {
            max_seconds,
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{file}: {:?}",
        report.diagnostics
    );
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(report.evidence_index, None);
    report
}

#[test]
fn resampling_statistics_match_python_without_claiming_original_rate() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/transform_reference.json")).unwrap();
    for case in oracle["resampling"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let report = run(file, None);
        assert_eq!(
            report.coverage.as_ref().unwrap().decoded_pcm_sha256,
            case["pcm_sha256"]
        );
        let result = &report.resampling[0];
        assert_eq!(
            result.active_frames,
            case["active_frames"].as_u64().unwrap()
        );
        let actual = serde_json::to_value(&result.candidates).unwrap();
        assert_eq!(
            actual.as_array().unwrap().len(),
            case["candidates"].as_array().unwrap().len()
        );
        for (index, candidate) in case["candidates"].as_array().unwrap().iter().enumerate() {
            assert_eq!(actual[index]["source_rate"], candidate["source_rate"]);
            for key in [
                "edge_relative_db",
                "notch_relative_db",
                "above_relative_db",
                "ceiling_relative_db",
                "mirror_correlation",
            ] {
                match (actual[index][key].as_f64(), candidate[key].as_f64()) {
                    (Some(a), Some(b)) => {
                        // f32 FFT roundoff becomes visible at these very low
                        // relative levels. This tolerance does not apply to
                        // audible bands or usable mirror correlations.
                        let tolerance = if key == "mirror_correlation" {
                            0.003
                        } else if b < -120.0 {
                            0.2
                        } else {
                            0.05
                        };
                        assert!(
                            (a - b).abs() < tolerance,
                            "{file}: {key} Rust {a} Python {b}"
                        );
                    }
                    (None, None) => (),
                    _ => panic!("{file}: {key} availability differs"),
                }
            }
        }
        if let Some(hit) = case["original_hit"].as_array() {
            assert_eq!(result.status, DetectorStatus::Hit);
            let candidate = result
                .candidates
                .iter()
                .find(|c| c.source_rate == hit[0].as_u64().unwrap() as u32)
                .unwrap();
            assert!(
                candidate
                    .modes
                    .iter()
                    .any(|mode| mode == hit[1].as_str().unwrap())
            );
        } else {
            assert_ne!(result.status, DetectorStatus::Hit, "{file}");
        }
    }
}

#[test]
fn vorbis_grid_matches_pinned_encoder_and_clean_reference_cases() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/transform_reference.json")).unwrap();
    for case in oracle["vorbis"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let report = run(file, None);
        assert_eq!(
            report.coverage.as_ref().unwrap().decoded_pcm_sha256,
            case["pcm_sha256"]
        );
        let result = &report.vorbis[0];
        let expected = case["score"].as_f64().unwrap();
        let actual = result.zero_excess_score.unwrap();
        assert!(
            (actual - expected).abs() < 0.0001,
            "{file}: Rust {actual} Python {expected}"
        );
        assert_eq!(
            result.supporting_probes,
            case["support"].as_u64().unwrap() as usize,
            "{file}"
        );
        assert_eq!(
            result.active_probes,
            case["active"].as_u64().unwrap() as usize
        );
        assert_eq!(
            result.status,
            if case["encoded"].as_bool().unwrap() {
                DetectorStatus::Hit
            } else {
                DetectorStatus::NotDetected
            }
        );
        assert!(
            result
                .probes
                .iter()
                .all(|p| p.interval.end_frame <= result.search_limit_frames)
        );
    }
}

#[test]
fn insufficient_prefixes_and_unsupported_rates_abstain() {
    let short = run("grid_q6_44100.flac", Some(0.1));
    assert_eq!(short.vorbis[0].status, DetectorStatus::Inconclusive);
    assert_eq!(short.vorbis[0].zero_excess_score, None);
    assert!(short.vorbis[0].probes.is_empty());
    assert_eq!(short.resampling[0].status, DetectorStatus::Unsupported);
    let short = run("rate_mirror.flac", Some(0.15));
    assert_eq!(short.resampling[0].status, DetectorStatus::Inconclusive);
    assert!(
        short.resampling[0]
            .candidates
            .iter()
            .all(|c| c.modes.is_empty())
    );
    let high = run("rate_full96.flac", None);
    assert_eq!(high.vorbis[0].status, DetectorStatus::Unsupported);
    assert!(high.vorbis[0].probes.is_empty());
}
