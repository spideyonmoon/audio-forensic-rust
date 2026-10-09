use audio_forensic::{
    AnalysisOptions, AnalysisProgress, AnalysisReport, CancellationToken, FileStatus, MediaSource,
    analyze_path, analyze_source, analyze_source_with_progress,
    metadata::{MetadataStatus, read_metadata_source},
};
use serde_json::Value;
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/alac")
            .join(name),
    )
    .unwrap()
}
fn run(bytes: Vec<u8>, options: &AnalysisOptions) -> AnalysisReport {
    analyze_source(
        Box::new(Cursor::new(bytes)),
        "wrong-extension.wav",
        options,
        &CancellationToken::default(),
    )
}
fn position(bytes: &[u8], kind: &[u8; 4], which: usize) -> usize {
    bytes
        .windows(4)
        .enumerate()
        .filter(|(_, v)| *v == kind)
        .nth(which)
        .unwrap()
        .0
        + 4
}
fn word(bytes: &mut [u8], at: usize, n: u32) {
    bytes[at..at + 4].copy_from_slice(&n.to_be_bytes());
}
fn with_dependencies(mut bytes: Vec<u8>, values: &[u8]) -> Vec<u8> {
    let start = position(&bytes, b"stbl", 0) - 8;
    let end = start + u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap()) as usize;
    let added = 12 + values.len();
    for kind in [*b"moov", *b"trak", *b"mdia", *b"minf", *b"stbl"] {
        let at = position(&bytes, &kind, 0) - 8;
        let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        word(&mut bytes, at, size + added as u32);
    }
    let mut atom = (added as u32).to_be_bytes().to_vec();
    atom.extend(b"sdtp");
    atom.extend([0; 4]);
    atom.extend(values);
    bytes.splice(end..end, atom);
    bytes
}

#[test]
fn container_compatibility_unknown_dependencies_and_cookie_precision_are_pcm_exact() {
    for bits in [16, 24] {
        let original = fixture(&format!("48000-{bits}-2-tail.m4a"));
        let baseline = run(original.clone(), &AnalysisOptions::default());
        let p = position(&original, b"stsz", 0);
        let samples = u32::from_be_bytes(original[p + 8..p + 12].try_into().unwrap()) as usize;
        let mut supported = with_dependencies(original.clone(), &vec![0; samples]);
        let p = position(&supported, b"alac", 0);
        supported[p + 18..p + 20].copy_from_slice(&16u16.to_be_bytes());
        for prefix in [None, Some(0.03)] {
            let options = AnalysisOptions {
                max_seconds: prefix,
                ..Default::default()
            };
            let reference = run(original.clone(), &options);
            let actual = run(supported.clone(), &options);
            assert_eq!(
                actual.status,
                FileStatus::Analyzed,
                "{:?}",
                actual.diagnostics
            );
            assert_eq!(actual.stream.as_ref().unwrap().bits_per_sample, Some(bits));
            assert_eq!(
                actual.coverage.as_ref().unwrap().decoded_pcm_sha256,
                reference.coverage.as_ref().unwrap().decoded_pcm_sha256
            );
        }
        assert_eq!(baseline.status, FileStatus::Analyzed);
        failure(
            with_dependencies(original.clone(), &vec![0; samples - 1]),
            FileStatus::Failed,
            "dependency count/length",
        );
        let mut claims = vec![0; samples];
        claims[0] = 0x10;
        failure(
            with_dependencies(original.clone(), &claims),
            FileStatus::Unsupported,
            "nonzero sample dependencies",
        );
        let mut version = supported.clone();
        let p = position(&version, b"sdtp", 0);
        version[p] = 1;
        failure(version, FileStatus::Unsupported, "full-box version/flags");
        let mut precision = supported;
        let p = position(&precision, b"alac", 0);
        precision[p + 18..p + 20].copy_from_slice(&8u16.to_be_bytes());
        failure(precision, FileStatus::Failed, "precision/channels disagree");
    }
}
fn failure(bytes: Vec<u8>, status: FileStatus, text: &str) {
    for prefix in [None, Some(0.03)] {
        let r = run(
            bytes.clone(),
            &AnalysisOptions {
                max_seconds: prefix,
                ..Default::default()
            },
        );
        assert_eq!(r.status, status, "{text}: {:?}", r.diagnostics);
        assert!(r.coverage.is_none());
        assert!(
            r.diagnostics.iter().any(|d| d.message.contains(text)),
            "{:?}",
            r.diagnostics
        );
    }
}

#[test]
fn generated_rate_precision_channel_layout_matrix_is_pcm_exact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/alac");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        for prefix in [false, true] {
            let rate = case["rate"].as_u64().unwrap();
            let options = AnalysisOptions {
                max_seconds: prefix.then_some(1000.25 / rate as f64),
                ..Default::default()
            };
            let report = analyze_path(
                root.join(case["file"].as_str().unwrap()),
                &options,
                &CancellationToken::default(),
            );
            assert_eq!(
                report.status,
                FileStatus::Analyzed,
                "{} prefix={prefix}: {:?}",
                case["file"],
                report.diagnostics
            );
            let stream = report.stream.as_ref().unwrap();
            assert_eq!(stream.codec, "alac");
            assert_eq!(stream.sample_rate, u32::try_from(rate).unwrap());
            assert_eq!(
                stream.bits_per_sample,
                Some(case["bits"].as_u64().unwrap() as u32)
            );
            assert_eq!(stream.channels, case["channels"].as_u64().unwrap() as usize);
            assert!(stream.integer_pcm);
            let coverage = report.coverage.as_ref().unwrap();
            assert_eq!(
                coverage.decoded_pcm_sha256,
                case[if prefix {
                    "prefix_pcm_sha256"
                } else {
                    "pcm_sha256"
                }]
                .as_str()
                .unwrap()
            );
            assert_eq!(coverage.analyzed_frames, if prefix { 1000 } else { 5001 });
            assert_eq!(coverage.reached_end, !prefix);
            assert_eq!(report.schema_version, "0.18.0");
            assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
            assert!(report.evidence_index.is_none());
        }
    }
}

struct Source {
    cursor: Cursor<Vec<u8>>,
    short: bool,
    fail_seek: Arc<AtomicBool>,
    cancel: Option<CancellationToken>,
}
impl Read for Source {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if let Some(c) = self.cancel.take() {
            c.cancel();
        }
        let n = if self.short {
            out.len().min(7)
        } else {
            out.len()
        };
        self.cursor.read(&mut out[..n])
    }
}
impl Seek for Source {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        if self.fail_seek.load(Ordering::Relaxed) {
            return Err(io::Error::other("F01 injected seek failure"));
        }
        self.cursor.seek(pos)
    }
}
impl MediaSource for Source {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

#[test]
fn unknown_length_short_reads_and_second_pass_seek_failures_are_explicit() {
    for layout in ["front", "tail"] {
        let bytes = fixture(&format!("96000-24-2-{layout}.m4a"));
        let r = analyze_source(
            Box::new(Source {
                cursor: Cursor::new(bytes.clone()),
                short: true,
                fail_seek: Arc::default(),
                cancel: None,
            }),
            "opaque-source",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
        let m = read_metadata_source(
            Box::new(Source {
                cursor: Cursor::new(bytes.clone()),
                short: true,
                fail_seek: Arc::default(),
                cancel: None,
            }),
            "opaque-source",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(m.status, MetadataStatus::Available);
        assert_eq!(m.technical.unwrap().file_size_bytes, None);
        let fail = Arc::new(AtomicBool::new(false));
        let set = fail.clone();
        let r = analyze_source_with_progress(
            Box::new(Source {
                cursor: Cursor::new(fixture("seek-long.m4a")),
                short: false,
                fail_seek: fail,
                cancel: None,
            }),
            "source",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            move |p| {
                if matches!(
                    p,
                    AnalysisProgress::Decoding {
                        pass: 1,
                        processed_frames: 100000,
                        ..
                    }
                ) {
                    set.store(true, Ordering::Relaxed);
                }
            },
        );
        assert_eq!(r.status, FileStatus::Failed);
        assert!(r.coverage.is_none());
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.message.contains("F01 injected seek failure"))
        );
    }
}

#[test]
fn cancellation_and_deadline_suppress_audio_results() {
    let bytes = fixture("8000-16-1-tail.m4a");
    for pass in [1, 2] {
        let cancel = CancellationToken::default();
        let signal = cancel.clone();
        let r = analyze_source_with_progress(
            Box::new(Cursor::new(bytes.clone())),
            "source",
            &AnalysisOptions::default(),
            &cancel,
            move |p| {
                if matches!(p,AnalysisProgress::Decoding {pass:p,processed_frames:0,..} if p==pass)
                {
                    signal.cancel();
                }
            },
        );
        assert_eq!(r.status, FileStatus::Cancelled);
        assert!(r.coverage.is_none());
    }
    let cancel = CancellationToken::default();
    let r = analyze_source(
        Box::new(Source {
            cursor: Cursor::new(bytes.clone()),
            short: true,
            fail_seek: Arc::default(),
            cancel: Some(cancel.clone()),
        }),
        "source",
        &AnalysisOptions::default(),
        &cancel,
    );
    assert_eq!(r.status, FileStatus::Cancelled);
    let r = run(
        bytes,
        &AnalysisOptions {
            deadline: Duration::ZERO,
            ..Default::default()
        },
    );
    assert_eq!(r.status, FileStatus::TimedOut);
    assert!(r.coverage.is_none());
}

#[test]
fn malformed_truncated_and_unbounded_container_controls() {
    let original = fixture("8000-16-1-tail.m4a");
    for kind in [*b"stts", *b"stsc", *b"stco"] {
        let mut bytes = original.clone();
        let p = position(&bytes, &kind, 0);
        word(&mut bytes, p + 4, u32::MAX);
        failure(bytes, FileStatus::Unsupported, "table exceeds");
    }
    let mut bytes = original.clone();
    let p = position(&bytes, b"stsz", 0);
    word(&mut bytes, p + 8, u32::MAX);
    failure(bytes, FileStatus::Unsupported, "sample count");
    let mut bytes = original.clone();
    let p = position(&bytes, b"alac", 1);
    word(&mut bytes, p + 4, u32::MAX);
    failure(bytes, FileStatus::Unsupported, "block/packet");
    let mut bytes = original.clone();
    let p = position(&bytes, b"stco", 0);
    word(&mut bytes, p + 8, u32::MAX);
    failure(bytes, FileStatus::Failed, "exceeds mdat");
    let mut bytes = original.clone();
    let p = position(&bytes, b"stts", 0);
    word(&mut bytes, p + 12, 0);
    failure(bytes, FileStatus::Failed, "packet timing");
    let mut bytes = original.clone();
    let p = position(&bytes, b"stsc", 0);
    word(&mut bytes, p + 8, 0);
    failure(bytes, FileStatus::Failed, "sample-to-chunk");
    let mut bytes = original.clone();
    let p = position(&bytes, b"stts", 0);
    word(&mut bytes, p - 8, 7);
    failure(bytes, FileStatus::Failed, "smaller than header");
    let mut bytes = original.clone();
    bytes.truncate(bytes.len() - 10);
    failure(bytes, FileStatus::Failed, "truncated MP4");
    let mut bytes = original.clone();
    let p = position(&bytes, b"mdat", 0);
    bytes[p..p + 100].fill(255);
    let r = run(bytes, &AnalysisOptions::default());
    assert_eq!(r.status, FileStatus::Failed);
    assert!(r.coverage.is_none());
    let mut bytes = original;
    let p = position(&bytes, b"moov", 0);
    word(&mut bytes, p - 8, 17 * 1024 * 1024);
    // The size must fit the actual source before its allocation limit is tested.
    bytes.resize(p - 8 + 17 * 1024 * 1024, 0);
    failure(bytes, FileStatus::Unsupported, "moov exceeds");
}

#[test]
fn codec_fragmentation_precision_tracks_and_edits_are_not_implicit_support() {
    let original = fixture("8000-16-1-tail.m4a");
    let mut bytes = original.clone();
    let p = position(&bytes, b"alac", 0);
    bytes[p - 4..p].copy_from_slice(b"mp4a");
    failure(bytes, FileStatus::Unsupported, "not ALAC");
    let mut bytes = original.clone();
    bytes.extend([0, 0, 0, 8, b'm', b'o', b'o', b'f']);
    failure(bytes, FileStatus::Unsupported, "fragmented");
    for (offset, value) in [(9, 32), (13, 6)] {
        let mut bytes = original.clone();
        let p = position(&bytes, b"alac", 1);
        bytes[p + offset] = value;
        failure(bytes, FileStatus::Unsupported, "16/24-bit mono/stereo");
    }
    let mut bytes = original.clone();
    let p = position(&bytes, b"hdlr", 0);
    bytes[p + 8..p + 12].copy_from_slice(b"vide");
    failure(bytes, FileStatus::Unsupported, "not audio");
    let mut bytes = original.clone();
    let p = position(&bytes, b"elst", 0);
    word(&mut bytes, p + 12, 1);
    failure(bytes, FileStatus::Unsupported, "nonidentity edit");
    let r = run(
        original,
        &AnalysisOptions {
            track_id: Some(1),
            ..Default::default()
        },
    );
    assert_eq!(r.status, FileStatus::Failed);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.message.contains("track is not present"))
    );
}

#[test]
fn metadata_binds_native_cookie_and_retains_named_text() {
    for layout in ["front", "tail"] {
        let m = read_metadata_source(
            Box::new(Cursor::new(fixture(&format!("384000-24-2-{layout}.m4a")))),
            "fake.flac",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        assert_eq!(m.status, MetadataStatus::Available, "{:?}", m.diagnostics);
        assert_eq!(m.named_value("title"), Some("Generated F01"));
        assert_eq!(m.named_value("comments"), Some("Known generated PCM"));
        let t = m.technical.as_ref().unwrap();
        assert_eq!(t.codec, "alac");
        assert_eq!(t.selected_track_id, 0);
        assert_eq!(t.declared_sample_rate_hz, Some(384000));
        assert_eq!(t.declared_precision_bits, Some(24));
        assert_eq!(t.declared_frames, Some(5001));
        assert!(m.text_limits.complete);
    }
}

#[test]
fn long_pcm_and_corrupt_partial_frame_lengths_are_structured() {
    let expected: Value = serde_json::from_slice(
        &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/alac/seek.json"))
            .unwrap(),
    )
    .unwrap();
    let r = run(fixture("seek-long.m4a"), &AnalysisOptions::default());
    assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
    assert_eq!(
        r.coverage.as_ref().unwrap().decoded_pcm_sha256,
        expected["pcm_sha256"].as_str().unwrap()
    );
    let mut bytes = fixture("8000-16-1-tail.m4a");
    let p = position(&bytes, b"mdat", 0);
    // First mono SCE: element/tag/reserved, then partial flag at bit 19 and
    // the explicit count at bits 23..55. All-ones exceeds decoder capacity.
    bytes[p + 2] |= 1 << 4;
    for bit in 23..55 {
        bytes[p + bit / 8] |= 1 << (7 - bit % 8);
    }
    let r = run(bytes, &AnalysisOptions::default());
    assert_eq!(r.status, FileStatus::Failed);
    assert!(r.coverage.is_none());
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.message.contains("ALAC packet caused a decoder panic")),
        "{:?}",
        r.diagnostics
    );
}
