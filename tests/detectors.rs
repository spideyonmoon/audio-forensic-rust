use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_path, analyze_source,
};
use std::{io::Cursor, path::Path};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn wav24(data: &[u8], channels: u16, rate: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * channels as u32 * 3).to_le_bytes());
    out.extend((channels * 3).to_le_bytes());
    out.extend(24u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    // RIFF chunks must end on an even byte boundary.
    if data.len() % 2 == 1 {
        out.push(0);
        let size = (out.len() - 8) as u32;
        out[4..8].copy_from_slice(&size.to_le_bytes());
    }
    out
}

fn analyze(data: Vec<u8>, max_seconds: Option<f64>) -> AnalysisReport {
    let report = analyze_source(
        Box::new(Cursor::new(data)),
        "synthetic.wav",
        &AnalysisOptions {
            max_seconds,
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(report.evidence_index, None);
    report
}

#[test]
fn clip_measurements_match_pinned_python_and_independent_numpy() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/detector_reference.json")).unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let report = analyze_path(
            fixture(case["file"].as_str().unwrap()),
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(
            report.status,
            FileStatus::Analyzed,
            "{:?}",
            report.diagnostics
        );
        let segment = &report.segments[0];
        assert_eq!(segment.status, DetectorStatus::Inconclusive); // Only one two-second clip.
        let probe = &segment.probes[0];
        assert_eq!(
            probe.wall_observed,
            case["wall_observed"].as_bool().unwrap()
        );
        for (key, actual, tolerance) in [
            ("cutoff_hz", probe.cutoff_hz, 0.001),
            ("cliff_db", probe.cliff_db, 0.0001),
            ("high_band_relative_db", probe.high_band_relative_db, 0.0001),
            ("peak_dbfs", probe.peak_dbfs, 1e-10),
            (
                "above_cutoff_relative_db",
                probe.above_cutoff_relative_db,
                0.0001,
            ),
            (
                "occupied_band_fraction",
                probe.occupied_band_fraction,
                1e-10,
            ),
        ] {
            match (actual, case[key].as_f64()) {
                (Some(a), Some(b)) => assert!(
                    (a - b).abs() <= tolerance,
                    "{} {key}: {a} vs {b}",
                    case["file"]
                ),
                (None, None) => (),
                _ => panic!("{} {key}: availability differs", case["file"]),
            }
        }
    }
}

fn clips(names: &[&str]) -> Vec<u8> {
    let mut data = vec![];
    for name in names {
        data.extend_from_slice(&std::fs::read(fixture(name)).unwrap()[44..]);
    }
    wav24(&data, 1, 44100)
}

#[test]
fn full_band_lowpass_and_mixed_probes_remain_observations() {
    for (names, walls, pattern) in [
        (vec!["clip_noise.wav"; 3], 0, "no_walls_observed"),
        (vec!["clip_high_wall.wav"; 3], 3, "repeated_walls"),
        (
            vec!["clip_wall.wav", "clip_noise.wav", "clip_noise.wav"],
            1,
            "isolated_wall",
        ),
        (
            vec![
                "clip_wall.wav",
                "clip_noise.wav",
                "clip_noise.wav",
                "clip_noise.wav",
                "clip_wall.wav",
            ],
            2,
            "mixed_probes",
        ),
    ] {
        let report = analyze(clips(&names), None);
        let segment = &report.segments[0];
        assert_eq!(segment.probes.len(), names.len());
        assert_eq!(segment.wall_probes, walls);
        assert_eq!(segment.pattern, pattern);
        for (i, probe) in segment.probes.iter().enumerate() {
            assert_eq!(probe.interval.start_frame, i as u64 * 88200);
            assert_eq!(probe.interval.end_frame, (i + 1) as u64 * 88200);
        }
        assert!(
            report
                .detectors
                .iter()
                .filter(|d| d.id == "segment_wall" || d.id == "codec_wall_candidates")
                .all(|d| d.family == "spectral_wall")
        );
    }
}

#[test]
fn probes_respect_prefix_and_preserve_silent_stereo_channel() {
    let report = analyze(
        clips(&["clip_noise.wav", "clip_wall.wav", "clip_wall.wav"]),
        Some(2.0),
    );
    assert_eq!(report.segments[0].probes.len(), 1);
    assert_eq!(report.segments[0].wall_probes, 0);
    assert_eq!(report.segments[0].status, DetectorStatus::Inconclusive);
    let mono = std::fs::read(fixture("clip_wall.wav")).unwrap();
    let mut stereo = vec![];
    for _ in 0..3 {
        for word in mono[44..].chunks_exact(3) {
            stereo.extend(word);
            stereo.extend([0, 0, 0]);
        }
    }
    let report = analyze(wav24(&stereo, 2, 44100), None);
    assert_eq!(report.segments[0].wall_probes, 3);
    assert_eq!(report.segments[1].active_probes, 0);
    assert_eq!(report.segments[1].status, DetectorStatus::Inconclusive);
}

fn mqa_wave(frames: usize, start: usize) -> Vec<u8> {
    let mut data = vec![];
    for i in 0..frames {
        let mut bit = 0;
        for (value, width, offset) in [
            (0xbe0498c88u64, 36, start),
            (9, 4, start + 38),
            (17, 5, start + 64),
        ] {
            if i >= offset && i < offset + width {
                bit = ((value >> (width - 1 - (i - offset))) & 1) as i32;
            }
        }
        // Both samples negative; XOR carries source bit plane 10 (MSB-aligned 18).
        let right = -0x200000i32;
        let left = right ^ (bit << 10);
        data.extend_from_slice(&left.to_le_bytes()[..3]);
        data.extend_from_slice(&right.to_le_bytes()[..3]);
    }
    wav24(&data, 2, 44100)
}

#[test]
fn mqa_survives_decoder_packets_and_signed_pcm_without_confirming_ancestry() {
    // Observe the demuxer's actual packet boundary, then straddle it with either
    // the marker or its payload. Do not assume a decoder block size.
    let source = symphonia::core::io::MediaSourceStream::new(
        Box::new(Cursor::new(mqa_wave(10000, 0))),
        Default::default(),
    );
    let mut probe = symphonia::default::get_probe()
        .format(
            &Default::default(),
            source,
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
    let boundary = probe.format.next_packet().unwrap().dur as usize;
    assert!((69..9900).contains(&boundary));
    for start in [boundary - 50, boundary - 12] {
        let report = analyze(mqa_wave(10000, start), None);
        let mqa = report.mqa.unwrap();
        assert_eq!(mqa.status, DetectorStatus::Hit);
        assert_eq!(mqa.sync_matches, 1);
        let candidate = &mqa.candidates[0];
        assert_eq!(candidate.sync_end_frame, start as u64 + 35);
        assert_eq!(candidate.source_bit_plane, 10);
        assert_eq!(candidate.rate_field_hz, Some(96000));
        assert_eq!(candidate.provenance_code, Some(17));
        assert!(candidate.payload_complete);
        assert!(!mqa.metadata_evaluated);
        let joint = report
            .detectors
            .iter()
            .find(|d| d.id == "mqa_signalling_candidates")
            .unwrap();
        assert_eq!(joint.channel_index, None);
        assert_eq!(joint.intervals[0].end_frame, 10000);
    }
}

#[test]
fn mqa_does_not_read_payload_or_marker_beyond_prefix() {
    let report = analyze(mqa_wave(44100, 4090), Some(4126.25 / 44100.0));
    let mqa = report.mqa.unwrap();
    assert_eq!(mqa.scanned_frames, 4126);
    assert_eq!(mqa.sync_matches, 1);
    assert_eq!(mqa.candidates[0].rate_code, None);
    assert!(!mqa.candidates[0].payload_complete);
    let report = analyze(mqa_wave(44100, 4090), Some(4090.25 / 44100.0));
    assert_eq!(report.mqa.unwrap().sync_matches, 0);
}
