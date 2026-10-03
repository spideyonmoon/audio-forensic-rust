use audio_forensic::{AnalysisOptions, CancellationToken, FileStatus, analyze_source};
use std::io::Cursor;

fn report(bytes: Vec<u8>) -> audio_forensic::AnalysisReport {
    analyze_source(
        Box::new(Cursor::new(bytes)),
        "untrusted",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    )
}
fn original() -> Vec<u8> {
    include_bytes!("fixtures/noise16.flac").to_vec()
}
fn flac(kind: u8, payload: &[u8]) -> Vec<u8> {
    let input = original();
    let mut offset = 4;
    loop {
        let last = input[offset] & 128 != 0;
        let n = u32::from_be_bytes([0, input[offset + 1], input[offset + 2], input[offset + 3]])
            as usize;
        offset += 4 + n;
        if last {
            break;
        }
    }
    let mut out = input[..42].to_vec();
    out[4] = 0;
    out.push(kind | 128);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes()[1..]);
    out.extend_from_slice(payload);
    out.extend_from_slice(&input[offset..]);
    out
}
fn comments(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = 0u32.to_le_bytes().to_vec();
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        out.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        out.extend_from_slice(entry);
    }
    out
}
fn picture() -> Vec<u8> {
    let mut out = 3u32.to_be_bytes().to_vec();
    out.extend_from_slice(&9u32.to_be_bytes());
    out.extend_from_slice(b"image/png");
    out.extend_from_slice(&0u32.to_be_bytes());
    for field in [1u32, 1, 8, 0, 3] {
        out.extend_from_slice(&field.to_be_bytes());
    }
    out.extend_from_slice(&[1, 2, 3]);
    out
}
fn encoded(bytes: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    for group in bytes.chunks(3) {
        let n = ((group[0] as u32) << 16)
            | ((group.get(1).copied().unwrap_or(0) as u32) << 8)
            | group.get(2).copied().unwrap_or(0) as u32;
        for shift in [18, 12, 6, 0] {
            out.push(ALPHABET[((n >> shift) & 63) as usize]);
        }
        if group.len() < 3 {
            *out.last_mut().unwrap() = b'=';
        }
        if group.len() == 1 {
            let len = out.len();
            out[len - 2] = b'=';
        }
    }
    out
}
fn wave(chunk: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(4 + 8 + payload.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(chunk);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}
fn failed(bytes: Vec<u8>, text: &str) {
    let r = report(bytes);
    assert_eq!(r.status, FileStatus::Failed, "{:?}", r.diagnostics);
    assert!(
        r.diagnostics.iter().any(|d| d.message.contains(text)),
        "{:?}",
        r.diagnostics
    );
    assert_eq!(r.ancestry_verdict, "INCONCLUSIVE");
    assert!(r.evidence_index.is_none());
}

#[test]
fn embedded_flac_picture_lengths_fail_before_dependency_allocation() {
    let mut x = 3u32.to_be_bytes().to_vec();
    x.extend_from_slice(&u32::MAX.to_be_bytes());
    failed(flac(6, &x), "embedded length exceeds block");
    let mut x = picture();
    x[37..41].copy_from_slice(&u32::MAX.to_be_bytes());
    failed(flac(6, &x), "embedded length exceeds block");
}
#[test]
fn nested_base64_picture_and_unicode_key_obey_the_same_bounds() {
    let mut bad = 3u32.to_be_bytes().to_vec();
    bad.extend_from_slice(&u32::MAX.to_be_bytes());
    for key in ["METADATA_BLOCK_PICTURE=", "METADATA_BLOCK_PICTURE="] {
        let mut x = key.as_bytes().to_vec();
        x.extend(encoded(&bad));
        failed(flac(4, &comments(&[x])), "embedded length exceeds block");
    }
}
#[test]
fn comment_lengths_counts_and_short_application_blocks_fail_structurally() {
    failed(
        flac(4, &u32::MAX.to_le_bytes()),
        "embedded length exceeds block",
    );
    let mut x = 0u32.to_le_bytes().to_vec();
    x.extend(u32::MAX.to_le_bytes());
    failed(flac(4, &x), "comment count exceeds block");
    failed(flac(2, &[]), "short application block");
    failed(flac(3, &[0]), "seek table length");
}
#[test]
fn valid_artwork_tags_and_nested_artwork_preserve_exact_audio() {
    let baseline = report(original());
    assert_eq!(baseline.status, FileStatus::Analyzed);
    let mut nested = b"metadata_block_picture=".to_vec();
    nested.extend(encoded(&picture()));
    for bytes in [
        flac(6, &picture()),
        flac(4, &comments(&[b"TITLE=generated".to_vec(), nested])),
    ] {
        let r = report(bytes);
        assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
        assert_eq!(
            r.coverage.unwrap().decoded_pcm_sha256,
            baseline.coverage.as_ref().unwrap().decoded_pcm_sha256
        );
    }
}
#[test]
fn aggregate_metadata_and_record_budgets_are_explicit() {
    let entries = vec![b"A=".to_vec(); 4097];
    let r = report(flac(4, &comments(&entries)));
    assert_eq!(r.status, FileStatus::Unsupported);
    assert!(r.diagnostics[0].message.contains("4096"));
    // STREAMINFO + an enormous declared block exceeds the aggregate budget
    // before any allocation or reliance on the missing payload.
    let mut bytes = original()[..42].to_vec();
    bytes[4] = 0;
    bytes.extend_from_slice(&[129, 255, 255, 255]);
    let r = report(bytes);
    assert_eq!(r.status, FileStatus::Unsupported);
    assert!(r.diagnostics[0].message.contains("16 MiB"));
    let mut bytes = original()[..42].to_vec();
    bytes[4] = 0;
    for _ in 0..1024 {
        bytes.extend_from_slice(&[1, 0, 0, 0]);
    }
    let r = report(bytes);
    assert_eq!(r.status, FileStatus::Unsupported);
    assert!(r.diagnostics[0].message.contains("1024"));
}
#[test]
fn wav_info_and_extensible_channel_fields_fail_before_parsing() {
    let mut list = b"INFOINAM".to_vec();
    list.extend(u32::MAX.to_le_bytes());
    failed(wave(b"LIST", &list), "WAV INFO length exceeds list");
    let mut fmt = vec![0; 40];
    fmt[..2].copy_from_slice(&65534u16.to_le_bytes());
    fmt[2..4].copy_from_slice(&u16::MAX.to_le_bytes());
    let r = report(wave(b"fmt ", &fmt));
    assert_eq!(r.status, FileStatus::Unsupported);
    fmt[2..4].copy_from_slice(&1u16.to_le_bytes());
    fmt[24..28].copy_from_slice(&6u32.to_le_bytes());
    let r = report(wave(b"fmt ", &fmt));
    assert_eq!(r.status, FileStatus::Unsupported);
    assert!(r.diagnostics[0].message.contains("subtype"));
    failed(wave(b"LIST", &[0; 3]), "short WAV list");
}

#[test]
fn missing_format_zero_alignment_and_partial_frames_return_failures() {
    failed(wave(b"data", &[0; 4]), "data precedes format");
    let mut bytes = include_bytes!("fixtures/noise16.wav").to_vec();
    // Generated fixture has the ordinary 16-byte PCM fmt chunk first.
    assert_eq!(&bytes[12..16], b"fmt ");
    bytes[32..34].copy_from_slice(&0u16.to_le_bytes());
    failed(bytes, "block alignment");
    let mut bytes = include_bytes!("fixtures/noise16.wav").to_vec();
    let mut pos = 12;
    while &bytes[pos..pos + 4] != b"data" {
        let n = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        pos += 8 + n + n % 2;
    }
    let n = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap());
    bytes[pos + 4..pos + 8].copy_from_slice(&(n - 1).to_le_bytes());
    failed(bytes, "partial PCM frame");
}
