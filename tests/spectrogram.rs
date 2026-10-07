use audio_forensic::{
    AnalysisOptions, AnalysisProgress, CancellationToken, FileStatus, MediaSource,
    analyze_path_with_spectrogram, analyze_source, analyze_source_with_spectrogram,
    spectrogram::{
        ArtifactStatus, DisplayBasis, FLOOR_DB, FREQUENCY_ROWS, RenderError, SpectrogramAnalysis,
    },
};
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    time::Duration,
};

fn wav(samples: &[f32], channels: u16, rate: u32) -> Vec<u8> {
    let bytes = samples.len() as u32 * 4;
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((36 + bytes).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(3u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * u32::from(channels) * 4).to_le_bytes());
    out.extend((channels * 4).to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    for &x in samples {
        out.extend(x.to_le_bytes());
    }
    out
}

fn analyze(samples: &[f32], channels: u16, rate: u32, seconds: Option<f64>) -> SpectrogramAnalysis {
    analyze_source_with_spectrogram(
        Box::new(Cursor::new(wav(samples, channels, rate))),
        "generated.wav",
        &AnalysisOptions {
            max_seconds: seconds,
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    )
}

#[test]
fn tone_axes_power_hash_and_native_report_match_at_low_and_high_rates() {
    let samples: Vec<_> = (0..8192)
        .map(|i| (0.5 * (std::f64::consts::TAU * 64.0 * i as f64 / 1024.0).sin()) as f32)
        .collect();
    for rate in [8000, 44100, 96000, 384000] {
        let p = analyze(&samples, 1, rate, None);
        assert_eq!(
            p.measurement.status,
            FileStatus::Analyzed,
            "{:?}",
            p.measurement.diagnostics
        );
        p.spectrogram.validate().unwrap();
        let d = p.spectrogram.data.as_ref().unwrap();
        assert_eq!(d.basis, DisplayBasis::Mono);
        assert_eq!(d.frequency_step_hz * 512.0, f64::from(rate) / 2.0);
        for bins in d.power_db.chunks_exact(FREQUENCY_ROWS) {
            let bin = bins
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0;
            assert_eq!(bin, 64);
            assert!(
                (bins
                    .iter()
                    .map(|&v| 10f64.powf(f64::from(v) / 10.0))
                    .sum::<f64>()
                    - 0.125)
                    .abs()
                    < 2e-6
            );
        }
        let hash = format!(
            "{:x}",
            Sha256::digest(
                samples
                    .iter()
                    .flat_map(|&x| f64::from(x).to_le_bytes())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(
            p.spectrogram.coverage.as_ref().unwrap().decoded_pcm_sha256,
            hash
        );
        assert_eq!(p.measurement.coverage.as_ref().unwrap().analysis_passes, 2);
        let native = analyze_source(
            Box::new(Cursor::new(wav(&samples, 1, rate))),
            "generated.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(
            serde_json::to_value(native).unwrap(),
            serde_json::to_value(&p.measurement).unwrap()
        );
    }
}

#[test]
fn silence_short_strict_eof_and_antiphase_are_explicit() {
    for frames in [100, 1024, 1025, 8192] {
        let p = analyze(&vec![0.; frames], 1, 8000, None);
        assert_eq!(p.measurement.status, FileStatus::Analyzed);
        p.spectrogram.validate().unwrap();
        if frames <= 1024 {
            assert_eq!(p.spectrogram.status, ArtifactStatus::Unavailable);
            assert!(p.spectrogram.data.is_none());
        } else {
            assert!(
                p.spectrogram
                    .data
                    .unwrap()
                    .power_db
                    .iter()
                    .all(|&v| v == FLOOR_DB)
            );
        }
    }
    let samples: Vec<_> = (0..8192)
        .flat_map(|i| {
            let x = if i % 2 == 0 { 0.5 } else { -0.5 };
            [x, -x]
        })
        .collect();
    let p = analyze(&samples, 2, 8000, None);
    let d = p.spectrogram.data.unwrap();
    assert_eq!(d.basis, DisplayBasis::StereoMid);
    assert_eq!(d.channel_indices, [0, 1]);
    assert!(d.mid_cancelled_with_native_signal);
    assert_eq!(d.native_peak, 0.5);
    assert!(
        d.warnings
            .iter()
            .any(|v| v.contains("native channels contain signal"))
    );
    assert!(d.power_db.iter().all(|&v| v == FLOOR_DB));
    assert_eq!(p.measurement.channels[0].peak, 0.5);
    assert_eq!(p.measurement.ancestry_verdict, "INCONCLUSIVE");
    assert!(p.measurement.evidence_index.is_none());
}

#[test]
fn burst_and_impulse_time_positions_and_prefix_do_not_include_tail() {
    let mut samples = vec![0f32; 24000];
    for (i, x) in samples.iter_mut().enumerate().take(14000).skip(10000) {
        *x = (0.5 * (std::f64::consts::TAU * 64.0 * i as f64 / 1024.0).sin()) as f32;
    }
    let p = analyze(&samples, 1, 8000, None);
    let d = p.spectrogram.data.as_ref().unwrap();
    let active: Vec<_> = d
        .power_db
        .chunks_exact(FREQUENCY_ROWS)
        .enumerate()
        .filter(|(_, bins)| bins[64] > -40.0)
        .map(|(i, _)| i)
        .collect();
    let first = d.time_columns[active[0]].window_support.start_frame + 512;
    let last = d.time_columns[*active.last().unwrap()]
        .window_support
        .end_frame
        - 512;
    // Window centers locate onset/offset within one displayed bucket (one hop).
    assert!(first.abs_diff(10000) <= 512);
    assert!(last.abs_diff(14000) <= 512);
    let prefix = analyze(&samples, 1, 8000, Some(1.0));
    assert!(!prefix.spectrogram.coverage.as_ref().unwrap().reached_end);
    assert_eq!(
        prefix.spectrogram.data.as_ref().unwrap().interval.end_frame,
        8000
    );
    assert!(
        prefix
            .spectrogram
            .data
            .unwrap()
            .power_db
            .iter()
            .all(|&v| v == FLOOR_DB)
    );
    samples.fill(0.0);
    samples[12000] = 0.75;
    let p = analyze(&samples, 1, 8000, None);
    let d = p.spectrogram.data.unwrap();
    let best = d
        .power_db
        .chunks_exact(FREQUENCY_ROWS)
        .enumerate()
        .max_by(|a, b| a.1[64].total_cmp(&b.1[64]))
        .unwrap()
        .0;
    let center = d.time_columns[best].window_support.start_frame + 512;
    assert!(center.abs_diff(12000) <= 512);
}

#[test]
fn raster_is_oriented_and_create_new_export_reports_failures_without_overwrite() {
    let p = analyze(&vec![0.0; 8192], 1, 8000, None);
    let token = CancellationToken::default();
    let rgb = p
        .spectrogram
        .render_rgb(&token, Duration::from_secs(10))
        .unwrap();
    assert_eq!(
        rgb.len(),
        p.spectrogram.data.as_ref().unwrap().time_columns.len() * 513 * 3
    );
    assert!(rgb.iter().all(|&v| v == 0));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new.ppm");
    let export = p
        .spectrogram
        .write_ppm_new(&path, &token, Duration::from_secs(10));
    assert_eq!(export.status, ArtifactStatus::Available);
    assert_eq!(export.path.as_ref(), Some(&path));
    let saved = std::fs::read(&path).unwrap();
    assert!(saved.starts_with(b"P6\n"));
    let conflict = p
        .spectrogram
        .write_ppm_new(&path, &token, Duration::from_secs(10));
    assert_eq!(conflict.status, ArtifactStatus::Failed);
    assert!(conflict.path.is_none());
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    let bad = p.spectrogram.write_ppm_new(
        temp.path().join("absent/new.ppm"),
        &token,
        Duration::from_secs(10),
    );
    assert!(bad.path.is_none());
    assert_eq!(bad.status, ArtifactStatus::Failed);
    token.cancel();
    assert!(matches!(
        p.spectrogram.render_rgb(&token, Duration::from_secs(10)),
        Err(RenderError::Cancelled)
    ));
    let missing = temp.path().join("cancel.ppm");
    let cancelled = p
        .spectrogram
        .write_ppm_new(&missing, &token, Duration::from_secs(10));
    assert_eq!(cancelled.status, ArtifactStatus::Cancelled);
    assert!(!missing.exists());
    assert!(matches!(
        p.spectrogram
            .render_rgb(&CancellationToken::default(), Duration::ZERO),
        Err(RenderError::TimedOut)
    ));
    let mut bright = p.spectrogram.clone();
    let d = bright.data.as_mut().unwrap();
    d.power_db[512] = 0.;
    d.power_db[0] = -70.;
    let rgb = bright
        .render_rgb(&CancellationToken::default(), Duration::from_secs(10))
        .unwrap();
    assert_eq!(&rgb[..3], &[255, 240, 160]);
    let width = bright.data.as_ref().unwrap().time_columns.len();
    assert_eq!(
        &rgb[(512 * width) * 3..(512 * width) * 3 + 3],
        &[160, 32, 64]
    );
}

#[test]
fn saved_descriptor_rejects_unknown_versions_bad_dimensions_axes_and_nonfinite_data() {
    let p = analyze(&vec![0.25; 8192], 1, 8000, None);
    let saved = serde_json::to_vec(&p).unwrap();
    let loaded: SpectrogramAnalysis = serde_json::from_slice(&saved).unwrap();
    loaded.spectrogram.validate().unwrap();
    loaded.validate_binding().unwrap();
    let mut mismatched = loaded.clone();
    mismatched
        .measurement
        .coverage
        .as_mut()
        .unwrap()
        .decoded_pcm_sha256 = "1".repeat(64);
    assert!(mismatched.validate_binding().is_err());
    for change in 0..8 {
        let mut a = loaded.spectrogram.clone();
        match change {
            0 => a.artifact_version = 2,
            1 => a.data.as_mut().unwrap().frequency_rows = usize::MAX,
            2 => a.data.as_mut().unwrap().power_db.push(0.),
            3 => a.data.as_mut().unwrap().time_columns[0].stft_frames = 100,
            4 => a.data.as_mut().unwrap().frequency_step_hz = 1.,
            5 => a.data.as_mut().unwrap().power_db[0] = f32::NAN,
            6 => a.coverage.as_mut().unwrap().analyzed_frames = u64::MAX,
            _ => a.data.as_mut().unwrap().channel_indices = vec![1],
        }
        assert!(
            a.render_rgb(&CancellationToken::default(), Duration::from_secs(10))
                .is_err(),
            "mutation {change}"
        );
    }
}

#[test]
fn failure_cancellation_and_deadline_suppress_artifacts_and_emit_one_terminal() {
    for phase in 0..4 {
        let token = CancellationToken::default();
        let mut events = Vec::new();
        let p = analyze_source_with_spectrogram(
            Box::new(Cursor::new(wav(&vec![0.25; 8192], 1, 8000))),
            "generated.wav",
            &AnalysisOptions::default(),
            &token,
            |e| {
                let stop = match phase {
                    0 => matches!(e, AnalysisProgress::WaitingForWorker),
                    1 => matches!(e, AnalysisProgress::Decoding { pass: 1, .. }),
                    2 => matches!(e, AnalysisProgress::Decoding { pass: 2, .. }),
                    _ => matches!(e, AnalysisProgress::AnalyzingDetectors),
                };
                if stop {
                    token.cancel();
                }
                events.push(e);
            },
        );
        assert_eq!(p.measurement.status, FileStatus::Cancelled);
        assert_eq!(p.spectrogram.status, ArtifactStatus::Cancelled);
        assert!(p.spectrogram.data.is_none());
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, AnalysisProgress::Finished { .. }))
                .count(),
            1
        );
    }
    let p = analyze_source_with_spectrogram(
        Box::new(Cursor::new(wav(&vec![0.25; 8192], 1, 8000))),
        "deadline.wav",
        &AnalysisOptions {
            deadline: Duration::ZERO,
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(p.spectrogram.status, ArtifactStatus::TimedOut);
    assert!(p.spectrogram.coverage.is_none());
    let p = analyze_source_with_spectrogram(
        Box::new(Cursor::new(b"invalid".to_vec())),
        "invalid.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    assert_ne!(p.measurement.status, FileStatus::Analyzed);
    assert!(p.spectrogram.data.is_none());
    let dir = tempfile::tempdir().unwrap();
    let p = analyze_path_with_spectrogram(
        dir.path().join("missing.wav"),
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(p.spectrogram.status, ArtifactStatus::Failed);
}

struct UnknownLength(Cursor<Vec<u8>>);
impl Read for UnknownLength {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let n = b.len().min(37);
        self.0.read(&mut b[..n])
    }
}
impl Seek for UnknownLength {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(p)
    }
}
impl MediaSource for UnknownLength {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

struct ChangingSource {
    cursor: Cursor<Vec<u8>>,
    zero_seeks: usize,
    changed: bool,
}
impl Read for ChangingSource {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let take = out.len().min(256);
        self.cursor.read(&mut out[..take])
    }
}
impl Seek for ChangingSource {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        if position == SeekFrom::Start(0) {
            self.zero_seeks += 1;
        }
        if self.zero_seeks >= 2 && position == SeekFrom::Start(44) && !self.changed {
            // Valid f32 mantissa change, keeping headers/count/PCM type intact.
            self.cursor.get_mut()[44] ^= 1;
            self.changed = true;
        }
        self.cursor.seek(position)
    }
}
impl MediaSource for ChangingSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.cursor.get_ref().len() as u64)
    }
}

#[test]
fn changed_pcm_between_passes_suppresses_collected_pixels() {
    let p = analyze_source_with_spectrogram(
        Box::new(ChangingSource {
            cursor: Cursor::new(wav(&vec![0.25; 8192], 1, 8000)),
            zero_seeks: 0,
            changed: false,
        }),
        "changed.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(
        p.measurement.status,
        FileStatus::Failed,
        "{:?}",
        p.measurement.diagnostics
    );
    assert!(
        p.measurement
            .diagnostics
            .iter()
            .any(|d| d.message == "Source changed between analysis passes"),
        "{:?}",
        p.measurement.diagnostics
    );
    assert!(p.spectrogram.data.is_none());
    assert!(p.spectrogram.coverage.is_none());
    p.validate_binding().unwrap();
    assert_eq!(
        analyze(&vec![0.25; 8192], 1, 8000, None).measurement.status,
        FileStatus::Analyzed
    );
}

#[test]
fn unknown_source_length_short_reads_bind_same_prefix_and_pcm() {
    let data = wav(&vec![0.25; 16000], 1, 8000);
    let p = analyze_source_with_spectrogram(
        Box::new(UnknownLength(Cursor::new(data))),
        "unknown.wav",
        &AnalysisOptions {
            max_seconds: Some(1.0),
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
    p.spectrogram.validate().unwrap();
    assert_eq!(
        p.spectrogram.coverage.as_ref().unwrap().analyzed_frames,
        8000
    );
    assert_eq!(
        p.spectrogram.coverage.as_ref().unwrap().decoded_pcm_sha256,
        p.measurement.coverage.as_ref().unwrap().decoded_pcm_sha256
    );
}

#[test]
fn flac_unknown_declared_duration_uses_observed_frames_and_same_pcm() {
    // Existing public generated fixture; only the optional STREAMINFO count changes.
    let original = include_bytes!("fixtures/activity16.flac").to_vec();
    let native = analyze_source(
        Box::new(Cursor::new(original.clone())),
        "generated.flac",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(native.status, FileStatus::Analyzed);
    let mut bytes = original;
    let packed = u64::from_be_bytes(bytes[18..26].try_into().unwrap()) & !((1u64 << 36) - 1);
    bytes[18..26].copy_from_slice(&packed.to_be_bytes());
    let p = analyze_source_with_spectrogram(
        Box::new(UnknownLength(Cursor::new(bytes))),
        "unknown.flac",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(
        p.measurement.status,
        FileStatus::Analyzed,
        "{:?}",
        p.measurement.diagnostics
    );
    assert_eq!(p.measurement.stream.as_ref().unwrap().declared_frames, None);
    assert_eq!(
        p.spectrogram.coverage.as_ref().unwrap().decoded_pcm_sha256,
        native.coverage.as_ref().unwrap().decoded_pcm_sha256
    );
    assert_eq!(
        p.spectrogram.coverage.as_ref().unwrap().analyzed_frames,
        native.coverage.as_ref().unwrap().analyzed_frames
    );
    p.validate_binding().unwrap();
    assert_eq!(p.spectrogram.status, ArtifactStatus::Available);
}
