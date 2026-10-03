use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, MediaSource, analyze_source,
};
use sha2::{Digest, Sha256};
use std::io::{self, Cursor, Read, Seek, SeekFrom};

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend(tag);
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    if data.len() % 2 != 0 {
        out.push(0);
    }
}

fn wave(bits: u16, valid: Option<u16>, channels: u16, floating: bool) -> (Vec<u8>, Vec<u8>) {
    let mut samples = Vec::new();
    let mut expected = Vec::new();
    for i in 0..1025 * usize::from(channels) {
        if floating {
            let x = [0.0f64, -0.0, 0.125, -0.25, 0.5, -0.875, 1.0, -1.0][i % 8];
            if bits == 32 {
                samples.extend((x as f32).to_le_bytes());
            } else {
                samples.extend(x.to_le_bytes());
            }
            expected.extend(x.to_le_bytes());
        } else {
            let precision = valid.unwrap_or(bits);
            let peak = 1i64 << (precision - 1);
            let x = [-peak, peak - 1, -1, 0, 1, peak / 2, -peak / 2][i % 7];
            let stored = x << (bits - precision);
            if bits == 8 {
                samples.push((stored + 128) as u8);
            } else {
                samples.extend(&stored.to_le_bytes()[..usize::from(bits / 8)]);
            }
            expected.extend(((x << (32 - precision)) as i32).to_le_bytes());
        }
    }
    let format = if valid.is_some() {
        65534u16
    } else if floating {
        3
    } else {
        1
    };
    let mut fmt = format.to_le_bytes().to_vec();
    fmt.extend(channels.to_le_bytes());
    fmt.extend(8000u32.to_le_bytes());
    fmt.extend((8000u32 * u32::from(channels * bits / 8)).to_le_bytes());
    fmt.extend((channels * bits / 8).to_le_bytes());
    fmt.extend(bits.to_le_bytes());
    if let Some(valid) = valid {
        fmt.extend(22u16.to_le_bytes());
        fmt.extend(valid.to_le_bytes());
        fmt.extend((if channels == 1 { 4u32 } else { 3 }).to_le_bytes());
        fmt.extend((if floating { 3u32 } else { 1 }).to_le_bytes());
        fmt.extend([0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
    }
    let mut body = b"WAVE".to_vec();
    chunk(&mut body, b"fmt ", &fmt);
    chunk(&mut body, b"data", &samples);
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((body.len() as u32).to_le_bytes());
    bytes.extend(body);
    (bytes, expected)
}

// Unknown source byte_len is distinct from an unknown data length in the WAV.
// Short reads exercise read_exact, ring refill and both decoder passes.
struct ShortSource(Cursor<Vec<u8>>);
impl Read for ShortSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out.len().min(7);
        self.0.read(&mut out[..n])
    }
}
impl Seek for ShortSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}
impl MediaSource for ShortSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

fn run(bytes: Vec<u8>, prefix: bool) -> AnalysisReport {
    let r = analyze_source(
        Box::new(ShortSource(Cursor::new(bytes))),
        "generated.bin",
        &AnalysisOptions {
            max_seconds: prefix.then_some(0.05),
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(r.ancestry_verdict, "INCONCLUSIVE");
    assert!(r.evidence_index.is_none());
    r
}

fn exact(bytes: Vec<u8>, pcm: &[u8], channels: u16, floating: bool, prefix: bool) {
    let r = run(bytes, prefix);
    assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
    let info = r.stream.unwrap();
    assert_eq!(info.channels, usize::from(channels));
    assert_eq!(info.integer_pcm, !floating);
    let coverage = r.coverage.unwrap();
    let frames = if prefix { 400 } else { 1025 };
    let count = frames * usize::from(channels) * if floating { 8 } else { 4 };
    assert_eq!(coverage.analyzed_frames, frames as u64);
    assert_eq!(coverage.reached_end, !prefix);
    assert_eq!(
        coverage.decoded_pcm_sha256,
        format!("{:x}", Sha256::digest(&pcm[..count]))
    );
}

fn rejected(bytes: Vec<u8>, status: FileStatus, text: &str) {
    for prefix in [false, true] {
        let r = run(bytes.clone(), prefix);
        assert_eq!(r.status, status, "{:?}", r.diagnostics);
        assert!(
            r.diagnostics.iter().any(|d| d.message.contains(text)),
            "{:?}",
            r.diagnostics
        );
        assert!(r.coverage.is_none() && r.channels.is_empty() && r.detectors.is_empty());
    }
}

#[test]
fn integer_container_widths_and_valid_bits_preserve_exact_native_pcm() {
    for channels in [1, 2] {
        for (bits, precision) in [(8, 7), (16, 12), (24, 20), (32, 24)] {
            for valid in [None, Some(bits), Some(precision)] {
                let (bytes, pcm) = wave(bits, valid, channels, false);
                for prefix in [false, true] {
                    exact(bytes.clone(), &pcm, channels, false, prefix);
                }
            }
        }
    }
}

#[test]
fn ieee_float_widths_preserve_exact_f64_hashes_and_signed_zero() {
    for channels in [1, 2] {
        for bits in [32, 64] {
            for valid in [None, Some(bits)] {
                let (bytes, pcm) = wave(bits, valid, channels, true);
                for prefix in [false, true] {
                    exact(bytes.clone(), &pcm, channels, true, prefix);
                }
            }
        }
    }
}

#[test]
fn unknown_data_lengths_and_rf64_are_explicitly_unsupported() {
    let (mut bytes, pcm) = wave(16, None, 1, false);
    bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    exact(bytes.clone(), &pcm, 1, false, false);
    bytes[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    rejected(
        bytes.clone(),
        FileStatus::Unsupported,
        "unknown-length WAV data",
    );
    bytes[4..8].copy_from_slice(&(36 + 2050u32).to_le_bytes());
    rejected(
        bytes.clone(),
        FileStatus::Unsupported,
        "unknown-length WAV data",
    );
    for tag in [b"RF64", b"BW64", b"RIFX"] {
        bytes[..4].copy_from_slice(tag);
        rejected(
            bytes.clone(),
            FileStatus::Unsupported,
            if tag == b"RIFX" { "RIFX" } else { "RF64/BW64" },
        );
    }
}

#[test]
fn channel_masks_and_subtype_guids_cannot_silently_change_basis() {
    for channels in [1, 2] {
        let (bytes, pcm) = wave(16, Some(16), channels, false);
        for mask in [0u32, 1, 3, 4, 8, 12, 0x80000001] {
            let mut x = bytes.clone();
            x[40..44].copy_from_slice(&mask.to_le_bytes());
            if matches!((channels, mask), (1, 0 | 1 | 4) | (2, 0 | 3)) {
                exact(x, &pcm, channels, false, false);
            } else {
                rejected(x, FileStatus::Unsupported, "channel mask");
            }
        }
        let mut x = bytes.clone();
        x[48..60].copy_from_slice(&[33, 7, 211, 17, 134, 68, 200, 193, 202, 0, 0, 0]);
        rejected(x, FileStatus::Unsupported, "subtype");
        let mut x = bytes;
        x[59] ^= 1;
        rejected(x, FileStatus::Unsupported, "subtype");
    }
}

#[test]
fn invalid_precision_extension_and_byte_rate_fail_before_analysis() {
    let (bytes, _) = wave(24, Some(20), 1, false);
    for (offset, value, message) in [
        (38, 0u16, "valid bits"),
        (38, 25, "valid bits"),
        (36, 21, "extension size"),
        (36, 65535, "extension size"),
    ] {
        let mut x = bytes.clone();
        x[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        rejected(x, FileStatus::Failed, message);
    }
    let mut x = bytes;
    x[28..32].copy_from_slice(&1u32.to_le_bytes());
    rejected(x, FileStatus::Failed, "byte rate");
    let (mut x, _) = wave(32, Some(32), 1, true);
    x[38..40].copy_from_slice(&24u16.to_le_bytes());
    rejected(x, FileStatus::Failed, "valid bits");
}

#[test]
fn nonzero_padding_fails_only_when_inside_the_analyzed_interval() {
    for (bits, valid) in [(8, 7), (16, 12), (24, 20), (32, 24)] {
        let (bytes, pcm) = wave(bits, Some(valid), 2, false);
        let mut x = bytes.clone();
        x[68] |= 1;
        rejected(x, FileStatus::Failed, "declared precision");
        let mut x = bytes;
        x[68 + 900 * 2 * usize::from(bits / 8)] |= 1;
        exact(x.clone(), &pcm, 2, false, true);
        let r = run(x, false);
        assert_eq!(r.status, FileStatus::Failed);
        assert!(r.coverage.is_none());
    }
}
