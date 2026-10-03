use audio_forensic::dsp::{HOP, StreamingStft, WINDOW};
use audio_forensic::model::DetectorStatus;
use audio_forensic::{AnalysisOptions, CancellationToken, FileStatus, analyze_source};
use std::{io::Cursor, time::Duration};

fn wav(bits: u16, channels: u16, rate: u32, format: u16, samples: &[i32]) -> Vec<u8> {
    let bytes = bits / 8;
    let mut data = vec![];
    for x in samples {
        data.extend_from_slice(&x.to_le_bytes()[..bytes as usize]);
    }
    let mut out = vec![];
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(format.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * channels as u32 * bytes as u32).to_le_bytes());
    out.extend((channels * bytes).to_le_bytes());
    out.extend(bits.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

fn analyze(data: Vec<u8>, options: AnalysisOptions) -> audio_forensic::AnalysisReport {
    analyze_source(
        Box::new(Cursor::new(data)),
        "memory.wav",
        &options,
        &CancellationToken::default(),
    )
}

#[test]
fn preserves_integer_precision_at_16_24_and_32_bits() {
    for bits in [16, 24, 32] {
        let values = [1, -1, 0, 13, -127, 0];
        let samples: Vec<_> = values.into_iter().cycle().take(12000).collect();
        let report = analyze(wav(bits, 2, 44100, 1, &samples), AnalysisOptions::default());
        assert_eq!(
            report.status,
            FileStatus::Analyzed,
            "{:?}",
            report.diagnostics
        );
        assert_eq!(report.coverage.as_ref().unwrap().analyzed_frames, 6000);
        for channel in report.channels {
            assert_eq!(channel.exact_used_bits, Some(bits as u32));
            assert_eq!(channel.effective_bits, Some(bits as u32));
        }
    }
}

#[test]
fn reports_padding_without_claiming_ancestry() {
    let samples = vec![123 * 256; 12000];
    let report = analyze(wav(24, 2, 48000, 1, &samples), AnalysisOptions::default());
    assert_eq!(report.status, FileStatus::Analyzed);
    assert_eq!(report.channels[0].effective_bits, Some(16));
    assert_eq!(report.detectors[0].status, DetectorStatus::Hit);
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    assert!(report.evidence_index.is_none());
}

#[test]
fn a_single_low_bit_prevents_an_exact_padding_claim() {
    let mut samples = vec![123 * 256; 12000];
    samples[10] += 1;
    let report = analyze(wav(24, 1, 48000, 1, &samples), AnalysisOptions::default());
    assert_eq!(report.channels[0].effective_bits, Some(16));
    assert_eq!(report.channels[0].exact_used_bits, Some(24));
    assert_eq!(report.detectors[0].status, DetectorStatus::NotDetected);
}

#[test]
fn silence_and_short_audio_have_no_fabricated_spectral_measurements() {
    for count in [100, 10000] {
        let report = analyze(
            wav(16, 1, 44100, 1, &vec![0; count]),
            AnalysisOptions::default(),
        );
        assert_eq!(report.status, FileStatus::Analyzed);
        let c = &report.channels[0];
        assert!(c.effective_bits.is_none());
        assert!(c.spectral.cutoff_p95_hz.is_none());
        assert_eq!(c.spectral.active_frames, 0);
        assert_eq!(report.detectors[1].status, DetectorStatus::Inconclusive);
    }
}

#[test]
fn strict_stft_boundary_matches_reference() {
    for count in [
        WINDOW - 1,
        WINDOW,
        WINDOW + 1,
        WINDOW + HOP,
        WINDOW + HOP + 1,
    ] {
        let mut stft = StreamingStft::new();
        let mut frames = 0;
        for _ in 0..count {
            stft.push(0.5, |_| frames += 1);
        }
        let expected = count.saturating_sub(WINDOW).div_ceil(HOP);
        assert_eq!(frames, expected, "count {count}");
    }
}

#[test]
fn limit_applies_to_every_measurement_and_does_not_check_full_length() {
    let mut samples = vec![1; 8000];
    samples.extend(vec![32767; 8000]);
    let options = AnalysisOptions {
        max_seconds: Some(1.0),
        ..Default::default()
    };
    let report = analyze(wav(16, 1, 8000, 1, &samples), options);
    assert_eq!(report.status, FileStatus::Analyzed);
    let coverage = report.coverage.unwrap();
    assert_eq!(coverage.analyzed_frames, 8000);
    assert_eq!(coverage.end_seconds, 1.0);
    assert!(!coverage.reached_end);
    assert!(coverage.length_matches_header.is_none());
    assert_eq!(report.channels[0].peak, 1.0 / 32768.0);
    assert_eq!(report.channels[0].samples, 8000);
}

#[test]
fn invalid_limits_fail_without_decoding() {
    for max_seconds in [0.0, -1.0, f64::INFINITY, f64::NAN, 90000.0] {
        let report = analyze(
            wav(16, 1, 44100, 1, &[1; 100]),
            AnalysisOptions {
                max_seconds: Some(max_seconds),
                ..Default::default()
            },
        );
        assert_eq!(report.status, FileStatus::Failed);
    }
}

#[test]
fn cancellation_and_deadline_are_explicit_outcomes() {
    let cancel = CancellationToken::default();
    cancel.cancel();
    let report = analyze_source(
        Box::new(Cursor::new(vec![])),
        "cancel.wav",
        &AnalysisOptions::default(),
        &cancel,
    );
    assert_eq!(report.status, FileStatus::Cancelled);
    let report = analyze(
        vec![],
        AnalysisOptions {
            deadline: Duration::ZERO,
            ..Default::default()
        },
    );
    assert_eq!(report.status, FileStatus::TimedOut);
}

#[test]
fn unsupported_surround_is_not_downmixed() {
    let report = analyze(
        wav(16, 3, 48000, 1, &[1; 12000]),
        AnalysisOptions::default(),
    );
    assert_eq!(report.status, FileStatus::Unsupported);
    assert!(report.channels.is_empty());
}

#[test]
fn nonfinite_float_is_a_failure_and_finite_float_skips_integer_checks() {
    let good = vec![0.25f32.to_bits() as i32; 10000];
    let report = analyze(wav(32, 1, 44100, 3, &good), AnalysisOptions::default());
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{:?}",
        report.diagnostics
    );
    assert!(!report.stream.unwrap().integer_pcm);
    assert_eq!(report.detectors[0].status, DetectorStatus::Unsupported);
    let bad = vec![f32::NAN.to_bits() as i32; 10000];
    assert_eq!(
        analyze(wav(32, 1, 44100, 3, &bad), AnalysisOptions::default()).status,
        FileStatus::Failed
    );
}

#[test]
fn truncation_and_missing_track_fail() {
    let mut data = wav(16, 1, 44100, 1, &[1; 10000]);
    data.truncate(data.len() - 4000);
    let report = analyze(data, AnalysisOptions::default());
    assert_eq!(report.status, FileStatus::Failed, "{:?}", report);
    let report = analyze(
        wav(16, 1, 44100, 1, &[1; 10000]),
        AnalysisOptions {
            track_id: Some(99),
            ..Default::default()
        },
    );
    assert_eq!(report.status, FileStatus::Failed);
}

#[test]
fn anti_phase_stereo_retains_each_channel() {
    let samples: Vec<_> = (0..10000)
        .flat_map(|i| {
            let x = (i % 200) * 100 - 10000;
            [x, -x]
        })
        .collect();
    let report = analyze(wav(16, 2, 44100, 1, &samples), AnalysisOptions::default());
    assert_eq!(report.status, FileStatus::Analyzed);
    assert!(report.channels[0].spectral.active_frames > 0);
    assert_eq!(
        report.channels[0].spectral.cutoff_p95_hz,
        report.channels[1].spectral.cutoff_p95_hz
    );
}

#[test]
#[cfg(feature = "cli")]
fn batch_cli_keeps_results_after_a_bad_file() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.wav");
    std::fs::write(&good, wav(16, 1, 44100, 1, &[1; 10000])).unwrap();
    let missing = dir.path().join("missing.wav");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_audio-forensic"))
        .arg("--json")
        .arg(missing)
        .arg(good)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let reports: Vec<audio_forensic::AnalysisReport> =
        serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reports[0].status, FileStatus::Failed);
    assert_eq!(reports[1].status, FileStatus::Analyzed);
}
