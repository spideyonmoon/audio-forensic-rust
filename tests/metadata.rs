use audio_forensic::{
    AnalysisOptions, CancellationToken, MediaSource, analyze_source,
    metadata::{
        MAX_KEY_BYTES, MAX_TEXT_BYTES, MAX_VALUE_BYTES, MetadataReport, MetadataStatus,
        ReplayGainStatus, audit_metadata_replaygain, audit_replaygain, read_metadata_source,
    },
};
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

fn read(bytes: Vec<u8>, name: &str) -> MetadataReport {
    read_metadata_source(
        Box::new(Cursor::new(bytes)),
        name,
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    )
}

fn flac_blocks(blocks: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let input = include_bytes!("fixtures/noise16.flac");
    let mut end = 4;
    loop {
        let last = input[end] & 128 != 0;
        let len = u32::from_be_bytes([0, input[end + 1], input[end + 2], input[end + 3]]) as usize;
        end += 4 + len;
        if last {
            break;
        }
    }
    let mut out = input[..42].to_vec();
    out[4] = 0;
    for (index, (kind, data)) in blocks.iter().enumerate() {
        out.push(kind | if index + 1 == blocks.len() { 128 } else { 0 });
        out.extend_from_slice(&(data.len() as u32).to_be_bytes()[1..]);
        out.extend(data);
    }
    out.extend(&input[end..]);
    out
}
fn comments(vendor: &[u8], entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = (vendor.len() as u32).to_le_bytes().to_vec();
    out.extend(vendor);
    out.extend((entries.len() as u32).to_le_bytes());
    for entry in entries {
        out.extend((entry.len() as u32).to_le_bytes());
        out.extend(entry);
    }
    out
}
fn flac(entries: &[&str]) -> Vec<u8> {
    flac_blocks(&[(
        4,
        comments(
            b"generated vendor",
            &entries
                .iter()
                .map(|s| s.as_bytes().to_vec())
                .collect::<Vec<_>>(),
        ),
    )])
}
fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    if data.len() % 2 != 0 {
        out.push(0);
    }
    out
}
fn wav(before: &[Vec<u8>], after: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"RIFF\0\0\0\0WAVE".to_vec();
    let fmt = [
        1u16.to_le_bytes().as_slice(),
        1u16.to_le_bytes().as_slice(),
        48000u32.to_le_bytes().as_slice(),
        96000u32.to_le_bytes().as_slice(),
        2u16.to_le_bytes().as_slice(),
        16u16.to_le_bytes().as_slice(),
    ]
    .concat();
    out.extend(chunk(b"fmt ", &fmt));
    for c in before {
        out.extend(c);
    }
    out.extend(chunk(b"data", &vec![0; 96000]));
    for c in after {
        out.extend(c);
    }
    let len = out.len() as u32 - 8;
    out[4..8].copy_from_slice(&len.to_le_bytes());
    out
}
fn info(entries: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut out = b"INFO".to_vec();
    for (key, value) in entries {
        out.extend(chunk(key, value));
    }
    chunk(b"LIST", &out)
}

#[test]
fn flac_named_unknown_duplicates_empty_and_unicode_are_lossless() {
    let report = read(
        flac(&[
            "Title=বাংলা 🎵",
            "TITLE=second",
            "ARTIST=a",
            "ALBUM=b",
            "DATE=2026",
            "ALBUMARTIST=c",
            "BPM=99",
            "COMMENT=d",
            "commentQuality=e",
            "REPLAYGAIN_TRACK_GAIN=+2 dB",
            "REPLAYGAIN_ALBUM_GAIN=-1 dB",
            "X-Strange= quote \" \\ \n =tail",
            "Empty=",
        ]),
        "fake.m4a",
    );
    assert_eq!(report.status, MetadataStatus::Available);
    assert_eq!(report.named_value("title"), Some("বাংলা 🎵"));
    assert_eq!(report.named_tags["title"], [1, 2]);
    assert_eq!(report.named_tags.len(), 11);
    assert_eq!(report.entries[12].key, "X-Strange");
    assert_eq!(
        report.entries[12].value.as_deref(),
        Some(" quote \" \\ \n =tail")
    );
    assert_eq!(report.entries[13].value.as_deref(), Some(""));
    let technical = report.technical.as_ref().unwrap();
    assert_eq!(technical.codec, "FLAC");
    assert_eq!(technical.declared_precision_bits, Some(16));
    assert!(technical.declared_duration_seconds.unwrap() > 0.0);
    assert_eq!(technical.declared_bit_rate_bps, None);
    assert_eq!(technical.derived_pcm_bit_rate_bps, None);
    assert!(report.text_limits.complete);
    let text = serde_json::to_string(&report).unwrap();
    let roundtrip: MetadataReport = serde_json::from_str(&text).unwrap();
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        serde_json::to_value(roundtrip).unwrap()
    );
}

#[test]
fn text_limits_preserve_utf8_and_account_for_all_omissions() {
    let mut tags = vec![format!("{}={}", "é".repeat(150), "🎵".repeat(5000)).into_bytes()];
    tags.extend((0..1090).map(|i| format!("K{i}=value").into_bytes()));
    let report = read(flac_blocks(&[(4, comments(b"v", &tags))]), "large.flac");
    assert_eq!(report.status, MetadataStatus::Available);
    assert_eq!(report.entries.len(), 1024);
    assert_eq!(report.text_limits.total_entries, 1092);
    assert_eq!(report.text_limits.omitted_entries, 68);
    assert_eq!(report.text_limits.truncated_entries, 1);
    let entry = &report.entries[1];
    assert_eq!(entry.key.len(), MAX_KEY_BYTES);
    assert_eq!(entry.value.as_ref().unwrap().len(), MAX_VALUE_BYTES);
    assert!(entry.key_truncated && entry.value_truncated);
    assert_eq!(
        report.text_limits.total_text_utf8_bytes,
        report.text_limits.retained_text_utf8_bytes + report.text_limits.omitted_text_utf8_bytes
    );
    assert!(!report.text_limits.complete);
    let many = (0..100)
        .map(|i| format!("K{i}={}", "a".repeat(16384)).into_bytes())
        .collect::<Vec<_>>();
    let report = read(flac_blocks(&[(4, comments(b"v", &many))]), "budget.flac");
    assert!(report.text_limits.omitted_entries > 0);
    assert!(report.text_limits.retained_text_utf8_bytes <= MAX_TEXT_BYTES as u64);
    assert_eq!(
        report.text_limits.total_text_utf8_bytes,
        report.text_limits.retained_text_utf8_bytes + report.text_limits.omitted_text_utf8_bytes
    );
    assert!(report.entries.iter().all(
        |e| e.key.len() <= MAX_KEY_BYTES && e.value.as_ref().unwrap().len() <= MAX_VALUE_BYTES
    ));
}

#[test]
fn malformed_text_and_hard_preflight_limits_are_explicit() {
    let tags = [
        b"no separator".to_vec(),
        b"=empty-key".to_vec(),
        b"TITLE=bad\xffvalue".to_vec(),
    ];
    let report = read(flac_blocks(&[(4, comments(b"v", &tags))]), "malformed.flac");
    assert_eq!(report.status, MetadataStatus::Available);
    assert_eq!(report.text_limits.malformed_entries, 2);
    assert_eq!(report.text_limits.invalid_utf8_entries, 1);
    assert!(!report.text_limits.complete);
    assert!(!report.observations.text_scan_complete);
    assert_eq!(report.named_value("title"), Some("bad�value"));
    let mut bad = comments(b"v", &[]);
    bad[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
    let report = read(flac_blocks(&[(4, bad)]), "bad.flac");
    assert_eq!(report.status, MetadataStatus::Failed);
    assert!(report.diagnostics[0].message.contains("count"));
    let too_large = vec![b'A'; 1024 * 1024 + 1];
    let report = read(
        flac_blocks(&[(4, comments(&too_large, &[]))]),
        "oversized.flac",
    );
    assert_eq!(report.status, MetadataStatus::Unsupported);
    assert!(!report.text_limits.complete);
}

#[test]
fn encoder_and_mqa_claims_remain_editable_unscored_text() {
    let report = read(
        flac(&[
            "COMMENT=mp3tag only",
            "X=MqA Studio",
            "Y=libmp3lame 320 kbps",
            "Z= xing",
        ]),
        "renamed.mp3",
    );
    assert_eq!(
        report.observations.encoder_signatures,
        ["320 kbps", "lame", "libmp3lame", "xing"]
    );
    assert!(
        report
            .observations
            .encoder_summary
            .as_ref()
            .unwrap()
            .contains("unknown")
    );
    assert_eq!(report.observations.legacy_encoder_trace, None);
    assert!(report.observations.mqa_metadata_claimed);
    assert!(
        !read(flac(&["X=notmqa mqax MQASTUDIOS", "TITLE=MQA"]), "x.flac")
            .observations
            .mqa_metadata_claimed
    );
    assert!(
        read(flac(&["X=mqaencoder"]), "x.flac")
            .observations
            .mqa_metadata_claimed
    );
    let clean = read(flac(&["COMMENT=mp3tag"]), "x.flac");
    assert!(clean.observations.encoder_signatures.is_empty());
    let measured = analyze_source(
        Box::new(Cursor::new(flac(&["X=MQA Studio LAME"]))),
        "tag-only.flac",
        &AnalysisOptions {
            max_seconds: Some(0.01),
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(measured.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(measured.evidence_index, None);
    assert_eq!(measured.mqa.as_ref().unwrap().sync_matches, 0);
}

#[test]
fn artwork_is_a_bounded_descriptor_and_preserves_exact_audio() {
    let mut pic = 3u32.to_be_bytes().to_vec();
    for text in [b"image/png".as_slice(), b"cover".as_slice()] {
        pic.extend((text.len() as u32).to_be_bytes());
        pic.extend(text);
    }
    for n in [640u32, 480, 24, 0, 3] {
        pic.extend(n.to_be_bytes());
    }
    pic.extend([1, 2, 3]);
    let bytes = flac_blocks(&[(6, pic)]);
    let report = read(bytes.clone(), "picture.flac");
    assert_eq!(report.artwork.len(), 1);
    let art = &report.artwork[0];
    assert_eq!(
        (art.picture_type, art.width, art.height, art.byte_length),
        (Some(3), Some(640), Some(480), 3)
    );
    let options = AnalysisOptions {
        max_seconds: Some(0.01),
        ..Default::default()
    };
    let cancel = CancellationToken::default();
    let original = analyze_source(
        Box::new(Cursor::new(
            include_bytes!("fixtures/noise16.flac").to_vec(),
        )),
        "baseline.flac",
        &options,
        &cancel,
    );
    let tagged = analyze_source(
        Box::new(Cursor::new(bytes)),
        "picture.flac",
        &options,
        &cancel,
    );
    assert_eq!(
        original.coverage.unwrap().decoded_pcm_sha256,
        tagged.coverage.unwrap().decoded_pcm_sha256
    );
}

struct NoAudio {
    bytes: Cursor<Vec<u8>>,
    audio: std::ops::Range<u64>,
    read_bytes: Arc<AtomicU64>,
}
impl Read for NoAudio {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let start = self.bytes.position();
        assert!(!self.audio.contains(&start), "metadata read PCM");
        let cap = if start < self.audio.start {
            (self.audio.start - start).min(out.len() as u64) as usize
        } else {
            out.len()
        };
        let n = self.bytes.read(&mut out[..cap])?;
        self.read_bytes.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}
impl Seek for NoAudio {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(pos)
    }
}
impl MediaSource for NoAudio {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.bytes.get_ref().len() as u64)
    }
}

#[test]
fn wav_info_after_pcm_is_read_without_decoding_or_reading_audio() {
    let before = info(&[(b"INAM", b"First\0"), (b"IART", b"Artist\0")]);
    let after = info(&[
        (b"INAM", b"Second\0"),
        (b"ZZZZ", "বাংলা\0".as_bytes()),
        (b"ISFT", b"MQA encoder\0"),
    ]);
    let bytes = wav(&[before.clone()], &[after]);
    let start = 44 + before.len() as u64;
    let count = Arc::new(AtomicU64::new(0));
    let source = NoAudio {
        bytes: Cursor::new(bytes),
        audio: start..start + 96000,
        read_bytes: count.clone(),
    };
    let report = read_metadata_source(
        Box::new(source),
        "actually.wav",
        &AnalysisOptions {
            max_seconds: Some(0.01),
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(report.status, MetadataStatus::Available);
    assert!(count.load(Ordering::Relaxed) < 300);
    assert_eq!(report.named_value("title"), Some("First"));
    assert_eq!(report.named_tags["title"], [0, 2]);
    assert_eq!(report.entries[3].value.as_deref(), Some("বাংলা"));
    assert!(report.observations.mqa_metadata_claimed);
    let tech = report.technical.unwrap();
    assert_eq!(tech.declared_frames, Some(48000));
    assert_eq!(tech.declared_duration_seconds, Some(1.0));
    assert_eq!(tech.derived_pcm_bit_rate_bps, Some(768000));
    assert_eq!(tech.declared_bit_rate_bps, None);
}

#[test]
fn absent_fields_unknown_totals_xml_and_unsupported_chunks_stay_explicit() {
    let report = read(wav(&[], &[]), "missing.wav");
    assert!(report.named_tags.is_empty());
    assert!(report.entries.is_empty());
    assert!(report.text_limits.complete);
    assert_eq!(report.named_value("title"), None);
    let mut bytes = flac(&[]);
    for b in &mut bytes[22..26] {
        *b = 0;
    }
    // Preserve precision bits in packed STREAMINFO while clearing the 36-bit total.
    bytes[21] &= 0xf0;
    let report = read(bytes, "unknown.flac");
    assert_eq!(report.technical.unwrap().declared_frames, None);
    let report = read(
        wav(
            &[chunk(b"iXML", b"<x>text</x>"), chunk(b"id3 ", b"opaque")],
            &[],
        ),
        "xml.wav",
    );
    assert_eq!(report.entries[0].value.as_deref(), Some("<x>text</x>"));
    assert_eq!(report.opaque_metadata.len(), 1);
    assert!(!report.observations.text_scan_complete);
    let mut bad_list = b"INFOZZZZ".to_vec();
    bad_list.extend(100u32.to_le_bytes());
    let report = read(wav(&[chunk(b"LIST", &bad_list)], &[]), "bad.wav");
    assert_eq!(report.status, MetadataStatus::Failed);
}

#[test]
fn metadata_cancellation_deadline_and_invalid_track_emit_no_success() {
    let cancel = CancellationToken::default();
    cancel.cancel();
    let report = read_metadata_source(
        Box::new(Cursor::new(wav(&[], &[]))),
        "c",
        &AnalysisOptions::default(),
        &cancel,
    );
    assert_eq!(report.status, MetadataStatus::Cancelled);
    assert!(report.entries.is_empty());
    for (options, status) in [
        (
            AnalysisOptions {
                deadline: Duration::ZERO,
                ..Default::default()
            },
            MetadataStatus::TimedOut,
        ),
        (
            AnalysisOptions {
                track_id: Some(9),
                ..Default::default()
            },
            MetadataStatus::Failed,
        ),
    ] {
        assert_eq!(
            read_metadata_source(
                Box::new(Cursor::new(wav(&[], &[]))),
                "x",
                &options,
                &CancellationToken::default()
            )
            .status,
            status
        );
    }
    assert_eq!(
        read(b"unknown".to_vec(), "x.flac").status,
        MetadataStatus::Unsupported
    );
}

#[test]
fn frozen_python_replaygain_vectors_match_exact_legacy_tuples() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/replaygain_reference.json")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let actual = audit_replaygain(
            case["stored"].as_str().unwrap(),
            case["lufs"].as_str().unwrap(),
        );
        assert_eq!(
            serde_json::to_value(&actual.legacy_outputs).unwrap(),
            case["expected"],
            "{case}"
        );
    }
    for (stored, lufs, expected) in [
        ("0 dB", "-18", "matches"),
        ("0", "-19", "minor_discrepancy"),
        ("0", "-21", "discrepancy"),
    ] {
        assert_eq!(
            audit_replaygain(stored, lufs).comparison.as_deref(),
            Some(expected)
        );
    }
    for lufs in ["nan", "inf", "-inf", "invalid"] {
        let audit = audit_replaygain("2", lufs);
        assert_eq!(audit.status, ReplayGainStatus::InvalidLoudness);
        assert_eq!(audit.absolute_delta_db, None);
    }
}

#[test]
fn replaygain_uses_actual_native_prefix_and_rejects_stale_or_unequal_inputs() {
    let bytes = flac(&["REPLAYGAIN_TRACK_GAIN=0 dB"]);
    let metadata = read(bytes.clone(), "pair.flac");
    let options = AnalysisOptions {
        max_seconds: Some(1.0),
        ..Default::default()
    };
    let mut measured = analyze_source(
        Box::new(Cursor::new(bytes)),
        "pair.flac",
        &options,
        &CancellationToken::default(),
    );
    let audit = audit_metadata_replaygain(&metadata, &measured);
    assert_eq!(audit.status, ReplayGainStatus::Available);
    assert_eq!(
        audit.analyzed_frames,
        Some(measured.coverage.as_ref().unwrap().analyzed_frames)
    );
    assert_eq!(
        audit.measured_lufs,
        measured.loudness.as_ref().unwrap().integrated_lufs
    );
    assert_eq!(
        audit.decoded_pcm_sha256.as_ref(),
        Some(&measured.coverage.as_ref().unwrap().decoded_pcm_sha256)
    );
    measured.source = "other.flac".into();
    assert_eq!(
        audit_metadata_replaygain(&metadata, &measured).status,
        ReplayGainStatus::UnavailableInput
    );
    measured.source = "pair.flac".into();
    measured.status = audio_forensic::FileStatus::Failed;
    assert_eq!(
        audit_metadata_replaygain(&metadata, &measured).absolute_delta_db,
        None
    );
}
