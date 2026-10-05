use audio_forensic::{
    AnalysisOptions, AnalysisProgress, CancellationToken, FileStatus,
    analyze_path_with_reference_inputs, analyze_source, analyze_source_with_reference_inputs,
};
use serde_json::Value;
use std::{io::Cursor, path::Path};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn oracle() -> Value {
    serde_json::from_str(include_str!("fixtures/reference_inputs.json")).unwrap()
}
fn near(a: f64, b: f64, tol: f64, label: &str) {
    assert!((a - b).abs() <= tol, "{label}: {a} != {b} ({tol})");
}
fn wav(samples: &[f64], channels: u16, rate: u32) -> Vec<u8> {
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
fn pcm(name: &str) -> Vec<f64> {
    use symphonia::core::{audio::SampleBuffer, errors::Error, io::MediaSourceStream};
    let stream = MediaSourceStream::new(
        Box::new(std::fs::File::open(fixture(name)).unwrap()),
        Default::default(),
    );
    let mut format = symphonia::default::get_probe()
        .format(
            &Default::default(),
            stream,
            &Default::default(),
            &Default::default(),
        )
        .unwrap()
        .format;
    let params = format.default_track().unwrap().codec_params.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &Default::default())
        .unwrap();
    let mut out = vec![];
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("{e}"),
        };
        let audio = decoder.decode(&packet).unwrap();
        let mut buf = SampleBuffer::<f64>::new(audio.capacity() as u64, *audio.spec());
        buf.copy_interleaved_ref(audio);
        out.extend_from_slice(buf.samples());
    }
    out
}

#[test]
fn base_and_source_scatter_match_frozen_generated_controls() {
    for case in oracle()["cases"].as_array().unwrap() {
        let result = analyze_path_with_reference_inputs(
            fixture(case["file"].as_str().unwrap()),
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            |_| {},
        );
        assert_eq!(
            result.measurement.status,
            FileStatus::Analyzed,
            "{:?}",
            result.measurement.diagnostics
        );
        let inputs = result.reference_inputs.unwrap();
        assert_eq!(inputs.version, 1);
        assert_eq!(inputs.processing_passes, 3);
        assert_eq!(
            inputs.decoded_pcm_sha256,
            case["pcm_sha256"].as_str().unwrap()
        );
        assert!(
            inputs
                .pass_pcm_sha256
                .iter()
                .all(|x| *x == inputs.decoded_pcm_sha256)
        );
        let e = &case["features"];
        let base = serde_json::to_value(&inputs.base).unwrap();
        assert_eq!(inputs.base.stft_frames, e["frames"].as_u64().unwrap());
        assert_eq!(
            inputs.base.active_frames,
            e["active_frames"].as_u64().unwrap()
        );
        if inputs.base.active_frames < 4 {
            assert!(inputs.base.cutoff_p95_hz.value.is_none());
            assert!(inputs.scatter.average_bound_hz.value.is_none());
            continue;
        }
        for (key, tol) in [
            ("cutoff_p95_hz", 0.01),
            ("cutoff_variance_hz2", 0.01),
            ("sharpness_db_per_bin", 0.02),
            ("cliff_depth_db", 0.02),
            ("hf_magnitude_ratio", 1e-6),
            ("entropy_bits", 1e-5),
            ("noise_above_cutoff_db", 0.02),
        ] {
            near(
                base[key]["value"].as_f64().unwrap(),
                e[key].as_f64().unwrap(),
                tol,
                key,
            );
        }
        let scatter = &e["scatter"];
        assert_eq!(
            inputs.scatter.sampling_stride,
            scatter["stride"].as_u64().unwrap()
        );
        assert_eq!(
            inputs.scatter.sampled_frames,
            scatter["sampled_frames"].as_u64().unwrap()
        );
        for (actual, key, tol) in [
            (
                inputs.scatter.legacy_average_bound_hz,
                "average_bound_hz",
                inputs.sample_rate as f64 / 4096.0,
            ),
            (
                inputs.scatter.legacy_histogram_mode_hz,
                "histogram_mode_hz",
                inputs.sample_rate as f64 / 4096.0,
            ),
            (
                inputs.scatter.legacy_phase_entropy_bits,
                "legacy_phase_entropy_bits",
                0.001,
            ),
        ] {
            near(actual.unwrap(), scatter[key].as_f64().unwrap(), tol, key);
        }
    }
}

#[test]
fn f32_transform_winners_match_source_and_tie_order() {
    for case in oracle()["transforms"].as_array().unwrap() {
        let signal = pcm(case["file"].as_str().unwrap());
        let layout = case["layout"].as_str().unwrap();
        let (samples, channels) = if layout == "mono" {
            (signal, 1)
        } else {
            (
                signal
                    .into_iter()
                    .flat_map(|x| match layout {
                        "dual_mono" => [x, x],
                        "antiphase" => [x, -x],
                        "right_only" => [0.0, x],
                        _ => panic!(),
                    })
                    .collect(),
                2,
            )
        };
        let result = analyze_source_with_reference_inputs(
            Box::new(Cursor::new(wav(&samples, channels, 44100))),
            "basis.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            |_| {},
        );
        assert_eq!(
            result.measurement.status,
            FileStatus::Analyzed,
            "{:?}",
            result.measurement.diagnostics
        );
        let inputs = result.reference_inputs.unwrap();
        if layout == "antiphase" {
            assert!(inputs.basis.mid_cancelled);
            assert!(inputs.base.cutoff_p95_hz.value.is_none());
        }
        if case["method"] == "aac" {
            assert_eq!(inputs.aac_winner.basis.as_deref(), case["winner"].as_str());
            for (actual, expected) in inputs
                .aac_bases
                .iter()
                .zip(case["scores"].as_array().unwrap())
            {
                let score = expected.as_f64().unwrap();
                if score <= 0.0 {
                    assert!(actual.lattice_score.is_none());
                } else {
                    near(actual.lattice_score.unwrap(), score, 1e-12, "AAC score");
                }
            }
        } else {
            assert_eq!(
                inputs.vorbis_winner.basis.as_deref(),
                case["winner"].as_str()
            );
            near(
                inputs.vorbis_winner.score.value.unwrap(),
                case["score"].as_f64().unwrap(),
                1e-4,
                "Vorbis score",
            );
            assert_eq!(
                inputs.vorbis_winner.supporting_probes.unwrap(),
                case["support"].as_u64().unwrap() as usize
            );
            assert_eq!(
                inputs.vorbis_winner.tested_probes.unwrap(),
                case["tested"].as_u64().unwrap() as usize
            );
        }
    }
}

#[test]
fn prefix_native_results_progress_and_failure_suppression() {
    let samples: Vec<f64> = (0..48000)
        .flat_map(|i| {
            [
                0.3 * (i as f64 * 0.137).sin(),
                -0.3 * (i as f64 * 0.137).sin(),
            ]
        })
        .collect();
    let data = wav(&samples, 2, 48000);
    let options = AnalysisOptions {
        max_seconds: Some(0.3),
        ..Default::default()
    };
    let native = analyze_source(
        Box::new(Cursor::new(data.clone())),
        "source.wav",
        &options,
        &CancellationToken::default(),
    );
    let mut events = vec![];
    let product = analyze_source_with_reference_inputs(
        Box::new(Cursor::new(data.clone())),
        "source.wav",
        &options,
        &CancellationToken::default(),
        |e| events.push(e),
    );
    assert_eq!(
        serde_json::to_value(native).unwrap(),
        serde_json::to_value(&product.measurement).unwrap()
    );
    let inputs = product.reference_inputs.unwrap();
    assert_eq!(inputs.analyzed_frames, 14400);
    assert!(!inputs.reached_end);
    assert!(inputs.basis.mid_cancelled);
    assert!(inputs.aac_winner.score.value.is_none());
    assert!(inputs.vorbis_winner.score.value.is_none());
    assert!(inputs.segments.plan.offsets.is_empty());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AnalysisProgress::Decoding { pass: 3, .. }))
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, AnalysisProgress::Finished { .. }))
            .count(),
        1
    );
    let token = CancellationToken::default();
    let callback_token = token.clone();
    let cancelled = analyze_source_with_reference_inputs(
        Box::new(Cursor::new(data)),
        "source.wav",
        &options,
        &token,
        |e| {
            if matches!(
                e,
                AnalysisProgress::Decoding {
                    pass: 3,
                    processed_frames: 0,
                    ..
                }
            ) {
                callback_token.cancel();
            }
        },
    );
    assert_eq!(cancelled.measurement.status, FileStatus::Cancelled);
    assert!(cancelled.reference_inputs.is_none());
    let invalid = analyze_source_with_reference_inputs(
        Box::new(Cursor::new(vec![0; 44])),
        "source.wav",
        &options,
        &CancellationToken::default(),
        |_| {},
    );
    assert_ne!(invalid.measurement.status, FileStatus::Analyzed);
    assert!(invalid.reference_inputs.is_none());
}

#[test]
fn third_pass_source_mutation_and_deadline_are_structured() {
    use std::{
        io::{Read, Seek, SeekFrom},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    struct Source {
        cursor: Cursor<Vec<u8>>,
        changed: Arc<AtomicBool>,
    }
    impl Read for Source {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let n = self.cursor.read(buffer)?;
            if n > 0 && self.changed.load(Ordering::Relaxed) {
                buffer[0] ^= 1;
            }
            Ok(n)
        }
    }
    impl Seek for Source {
        fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
            self.cursor.seek(p)
        }
    }
    impl audio_forensic::MediaSource for Source {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.cursor.get_ref().len() as u64)
        }
    }
    let data = wav(
        &(0..48000)
            .map(|i| 0.1 * (i as f64 * 0.137).sin())
            .collect::<Vec<_>>(),
        1,
        48000,
    );
    let changed = Arc::new(AtomicBool::new(false));
    let notify = changed.clone();
    let result = analyze_source_with_reference_inputs(
        Box::new(Source {
            cursor: Cursor::new(data.clone()),
            changed,
        }),
        "one-source.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        move |e| {
            if matches!(
                e,
                AnalysisProgress::Decoding {
                    pass: 3,
                    processed_frames: 0,
                    ..
                }
            ) {
                notify.store(true, Ordering::Relaxed);
            }
        },
    );
    assert_eq!(
        result.measurement.status,
        FileStatus::Failed,
        "{:?}",
        result.measurement.diagnostics
    );
    assert!(result.reference_inputs.is_none());
    assert!(
        result
            .measurement
            .diagnostics
            .iter()
            .any(|d| d.code == "decode_error")
    );
    let timed = analyze_source_with_reference_inputs(
        Box::new(Cursor::new(data)),
        "one-source.wav",
        &AnalysisOptions {
            deadline: Duration::from_secs(1),
            ..Default::default()
        },
        &CancellationToken::default(),
        |e| {
            if matches!(
                e,
                AnalysisProgress::Decoding {
                    pass: 3,
                    processed_frames: 0,
                    ..
                }
            ) {
                std::thread::sleep(Duration::from_millis(1100));
            }
        },
    );
    assert_eq!(timed.measurement.status, FileStatus::TimedOut);
    assert!(timed.reference_inputs.is_none());
}
