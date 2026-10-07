use audio_forensic::{
    AnalysisOptions, CancellationToken, FileStatus, analyze_path_with_spectrogram,
    analyze_source_with_spectrogram,
    spectrogram::{ArtifactStatus, RenderError},
    spectrogram_png::{CanvasOptions, CanvasSize, MAX_CANVAS_RGB_BYTES},
};
use std::{io::Cursor, time::Duration};

fn control(stereo: bool, prefix: bool) -> audio_forensic::spectrogram::SpectrogramAnalysis {
    let p = analyze_path_with_spectrogram(
        if stereo {
            "tests/fixtures/activity16.wav"
        } else {
            "tests/fixtures/noise16.wav"
        },
        &AnalysisOptions {
            max_seconds: prefix.then_some(0.5),
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(
        p.measurement.status,
        FileStatus::Analyzed,
        "{:?}",
        p.measurement.diagnostics
    );
    p
}

#[test]
fn complete_canvas_contains_calibrated_endpoints_metadata_cutoff_and_original_measurement() {
    let p = control(false, false);
    let original = serde_json::to_value(&p.measurement).unwrap();
    let image = p
        .render_canvas(
            &CanvasOptions::default(),
            &CancellationToken::default(),
            Duration::from_secs(20),
        )
        .unwrap();
    let d = &image.descriptor;
    assert_eq!((d.width, d.height), (2560, 1440));
    assert_eq!(image.rgb.len(), 2560 * 1440 * 3);
    assert_eq!(d.frequency_ticks.first().unwrap().value, 0.0);
    assert_eq!(
        d.frequency_ticks.last().unwrap().value,
        f64::from(p.measurement.stream.as_ref().unwrap().sample_rate) / 2.
    );
    assert_eq!(d.time_ticks.first().unwrap().label, "0:00");
    assert_eq!(
        d.time_ticks.last().unwrap().value,
        p.measurement.coverage.as_ref().unwrap().end_seconds
    );
    assert!(d.time_ticks.windows(2).all(|v| v[0].label != v[1].label));
    assert!(d.stream_line.contains("kbps (audio average)"));
    assert!(d.analysis_line.contains("Hann 1024 / hop 512"));
    assert!(d.analysis_line.contains("FULL STREAM"));
    assert_eq!(d.legend_min_db, -140.);
    assert_eq!(d.legend_max_db, 0.);
    let src = p.measurement.channels[0].spectral.cutoff_p95_hz.unwrap();
    assert_eq!(d.cutoffs[0].frequency_hz, src);
    assert!(d.cutoffs[0].label.contains("native Ch"));
    assert_eq!(serde_json::to_value(&p.measurement).unwrap(), original);
    let x = 2256usize;
    let top = 300usize;
    let bottom = 1180usize;
    assert_eq!(
        &image.rgb[(top * 2560 + x) * 3..(top * 2560 + x) * 3 + 3],
        &[255, 240, 160]
    );
    assert_eq!(
        &image.rgb[(bottom * 2560 + x) * 3..(bottom * 2560 + x) * 3 + 3],
        &[0, 0, 0]
    );
}

#[test]
fn publication_png_is_lossless_rgb_with_dpi_context_and_no_overwrite() {
    let p = control(false, false);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("canvas.png");
    let options = CanvasOptions {
        size: CanvasSize::Standard,
        title: Some("Calibrated spectrum — generated control".into()),
    };
    let export = p.write_png_new(
        &path,
        &options,
        &CancellationToken::default(),
        Duration::from_secs(20),
    );
    assert_eq!(
        export.status,
        ArtifactStatus::Available,
        "{:?}",
        export.reason
    );
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let decoder = png::Decoder::new(Cursor::new(&bytes));
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!((info.width, info.height), (1600, 900));
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    assert_eq!(info.pixel_dims.unwrap().xppu, 11811);
    assert_eq!(info.srgb, Some(png::SrgbRenderingIntent::Perceptual));
    let caption = &info.utf8_text[0];
    assert_eq!(caption.keyword, "Alfred canvas");
    let meta: serde_json::Value = serde_json::from_str(&caption.get_text().unwrap()).unwrap();
    assert_eq!(meta["render_version"], 1);
    assert_eq!(meta["title"], options.title.unwrap());
    let mut decoded = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut decoded).unwrap();
    let image = p
        .render_canvas(
            &CanvasOptions {
                size: CanvasSize::Standard,
                title: Some("Calibrated spectrum — generated control".into()),
            },
            &CancellationToken::default(),
            Duration::from_secs(20),
        )
        .unwrap();
    assert_eq!(&decoded[..frame.buffer_size()], image.rgb);
    let conflict = p.write_png_new(
        &path,
        &CanvasOptions::default(),
        &CancellationToken::default(),
        Duration::from_secs(20),
    );
    assert_eq!(conflict.status, ArtifactStatus::Failed);
    assert!(conflict.path.is_none());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn partial_scope_and_legacy_saved_wrappers_keep_bitrate_honest() {
    let p = control(false, true);
    p.validate_binding().unwrap();
    let image = p
        .render_canvas(
            &CanvasOptions {
                size: CanvasSize::Standard,
                ..Default::default()
            },
            &CancellationToken::default(),
            Duration::from_secs(20),
        )
        .unwrap();
    assert!(image.descriptor.analysis_line.contains("ANALYZED PREFIX"));
    assert_eq!(image.descriptor.time_ticks.last().unwrap().value, 0.5);
    if let Some(i) = p.presentation.as_ref().unwrap().bitrate_interval.as_ref() {
        assert!(i.end_frame <= p.measurement.coverage.as_ref().unwrap().analyzed_frames);
    }
    let mut value = serde_json::to_value(&p).unwrap();
    value.as_object_mut().unwrap().remove("presentation");
    let old: audio_forensic::spectrogram::SpectrogramAnalysis =
        serde_json::from_value(value).unwrap();
    let image = old
        .render_canvas(
            &CanvasOptions {
                size: CanvasSize::Standard,
                ..Default::default()
            },
            &CancellationToken::default(),
            Duration::from_secs(20),
        )
        .unwrap();
    assert!(image.descriptor.stream_line.contains("bitrate unavailable"));
    let mut mutated = p;
    mutated.presentation.as_mut().unwrap().decoded_pcm_sha256 = "a".repeat(64);
    assert!(mutated.validate_binding().is_err());
}

#[test]
fn maximum_canvas_title_and_controls_are_bounded_and_failed_paths_are_null() {
    let p = control(false, false);
    let token = CancellationToken::default();
    let options = CanvasOptions {
        size: CanvasSize::Large,
        title: Some("Long Unicode title é — ".repeat(1000)),
    };
    let image = p
        .render_canvas(&options, &token, Duration::from_secs(20))
        .unwrap();
    assert_eq!(image.rgb.len(), MAX_CANVAS_RGB_BYTES);
    assert!(image.descriptor.title_truncated);
    assert!(image.descriptor.title.chars().count() <= 257);
    token.cancel();
    assert!(matches!(
        p.render_canvas(&options, &token, Duration::from_secs(20)),
        Err(RenderError::Cancelled)
    ));
    assert!(matches!(
        p.render_canvas(&options, &CancellationToken::default(), Duration::ZERO),
        Err(RenderError::TimedOut)
    ));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cancel.png");
    let export = p.write_png_new(&path, &options, &token, Duration::from_secs(20));
    assert!(export.path.is_none());
    assert!(!path.exists());
    let missing = p.write_png_new(
        dir.path().join("absent/canvas.png"),
        &CanvasOptions::default(),
        &CancellationToken::default(),
        Duration::from_secs(20),
    );
    assert_eq!(missing.status, ArtifactStatus::Failed);
    assert!(missing.path.is_none());
}

fn float_wav(stereo: bool, silent: bool) -> Vec<u8> {
    let channels = if stereo { 2u16 } else { 1 };
    let mut pcm = Vec::new();
    for _ in 0..8192 {
        let x = if silent { 0f32 } else { 0.5 };
        pcm.extend(x.to_le_bytes());
        if stereo {
            pcm.extend((-x).to_le_bytes());
        }
    }
    let n = pcm.len() as u32;
    let mut out = b"RIFF".to_vec();
    out.extend((36 + n).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(3u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(8000u32.to_le_bytes());
    out.extend((8000u32 * u32::from(channels) * 4).to_le_bytes());
    out.extend((channels * 4).to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(b"data");
    out.extend(n.to_le_bytes());
    out.extend(pcm);
    out
}

#[test]
fn silence_and_mid_cancellation_are_visible_without_fabricated_cutoff() {
    for (stereo, silent) in [(false, true), (true, false)] {
        let p = analyze_source_with_spectrogram(
            Box::new(Cursor::new(float_wav(stereo, silent))),
            "generated.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            |_| {},
        );
        let image = p
            .render_canvas(
                &CanvasOptions {
                    size: CanvasSize::Standard,
                    ..Default::default()
                },
                &CancellationToken::default(),
                Duration::from_secs(20),
            )
            .unwrap();
        if silent {
            assert!(image.descriptor.cutoffs.is_empty());
        } else {
            // The mid cancels, but each native channel still has its measured cutoff.
            assert_eq!(image.descriptor.cutoffs.len(), 1);
            assert_eq!(image.descriptor.cutoffs[0].channel_indices, [0, 1]);
            assert_eq!(
                image.descriptor.cutoffs[0].frequency_hz,
                p.measurement.channels[0].spectral.cutoff_p95_hz.unwrap()
            );
        }
        assert!(
            image
                .descriptor
                .time_ticks
                .windows(2)
                .all(|v| v[0].label != v[1].label)
        );
        if stereo {
            assert!(
                image
                    .descriptor
                    .warnings
                    .iter()
                    .any(|v| v.contains("native channels contain signal"))
            );
        }
        let presentation = p.presentation.unwrap();
        assert_eq!(
            presentation.bitrate_bps,
            Some(if stereo { 512000. } else { 256000. })
        );
    }
}

#[test]
fn distinct_native_channel_cutoffs_are_never_averaged_into_mid() {
    let mut p = control(true, false);
    assert_eq!(p.measurement.channels.len(), 2);
    p.measurement.channels[0].spectral.cutoff_p95_hz = Some(1000.);
    p.measurement.channels[1].spectral.cutoff_p95_hz = Some(1005.);
    let image = p
        .render_canvas(
            &CanvasOptions {
                size: CanvasSize::Standard,
                ..Default::default()
            },
            &CancellationToken::default(),
            Duration::from_secs(20),
        )
        .unwrap();
    assert_eq!(image.descriptor.cutoffs.len(), 2);
    assert_eq!(image.descriptor.cutoffs[0].frequency_hz, 1005.);
    assert_eq!(image.descriptor.cutoffs[1].frequency_hz, 1000.);
    assert!(image.descriptor.analysis_line.contains("stereo mid"));
}
