use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, MediaSource, analyze_source,
};
use sha2::{Digest, Sha256};
use std::io::{self, Cursor, Read, Seek, SeekFrom};

fn crc(bytes: &[u8], width: u32, polynomial: u32) -> u32 {
    let mut value = 0;
    for byte in bytes {
        value ^= u32::from(*byte) << (width - 8);
        for _ in 0..8 {
            value = (value << 1)
                ^ if value & (1 << (width - 1)) != 0 {
                    polynomial
                } else {
                    0
                };
            value &= (1 << width) - 1;
        }
    }
    value
}

fn generated(channels: usize, unknown: bool, checksum: bool) -> (Vec<u8>, Vec<Vec<u8>>, Vec<u8>) {
    let lengths = [256usize, 256, 256, 256, 73];
    let words: Vec<_> = (0..1097 * channels)
        .map(|i| ((i * 71 + (i % channels) * 919) % 32000) as i16 - 16000)
        .collect();
    let pcm: Vec<_> = words
        .iter()
        .flat_map(|&x| (i32::from(x) << 16).to_le_bytes())
        .collect();
    let mut frames = Vec::new();
    let mut start = 0;
    for (number, length) in lengths.into_iter().enumerate() {
        let mut frame = vec![
            255,
            248,
            0x64,
            (((channels - 1) as u8) << 4) | 8,
            number as u8,
            (length - 1) as u8,
        ];
        frame.push(crc(&frame, 8, 7) as u8);
        for ch in 0..channels {
            frame.push(2); // Verbatim subframe, no wasted bits.
            for i in 0..length {
                frame.extend(words[(start + i) * channels + ch].to_be_bytes());
            }
        }
        frame.extend((crc(&frame, 16, 0x8005) as u16).to_be_bytes());
        frames.push(frame);
        start += length;
    }
    let mut prefix = b"fLaC\x80\0\0\x22".to_vec();
    prefix.extend(256u16.to_be_bytes());
    prefix.extend(256u16.to_be_bytes());
    for n in [
        frames.iter().map(Vec::len).min().unwrap(),
        frames.iter().map(Vec::len).max().unwrap(),
    ] {
        prefix.extend(&(n as u32).to_be_bytes()[1..]);
    }
    let fields = (8000u64 << 44)
        | (((channels - 1) as u64) << 41)
        | (15 << 36)
        | if unknown { 0 } else { 1097 };
    prefix.extend(fields.to_be_bytes());
    // Independent Python packing/MD5 and strict FFmpeg confirm these generated controls.
    let md5 = if !checksum {
        [0; 16]
    } else if channels == 1 {
        [
            0xb0, 0x80, 0x99, 0x36, 0xda, 0x9e, 0x67, 0x78, 0x4b, 0x9c, 0xe9, 0x09, 0x7a, 0x15,
            0x5e, 0x3b,
        ]
    } else {
        [
            0x76, 0x5d, 0xc5, 0xce, 0x23, 0xe2, 0xbb, 0xaa, 0x88, 0xf4, 0x48, 0x5f, 0x53, 0xaf,
            0x96, 0xbf,
        ]
    };
    prefix.extend(md5);
    (prefix, frames, pcm)
}

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
    let report = analyze_source(
        Box::new(ShortSource(Cursor::new(bytes))),
        "generated.bin",
        &AnalysisOptions {
            max_seconds: prefix.then_some(0.05),
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    assert!(report.evidence_index.is_none());
    report
}

fn joined(prefix: &[u8], frames: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = prefix.to_vec();
    bytes.extend(frames.iter().flatten());
    bytes
}

fn exact(
    bytes: Vec<u8>,
    pcm: &[u8],
    frames: usize,
    channels: usize,
    prefix: bool,
) -> AnalysisReport {
    let report = run(bytes, prefix);
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{:?}",
        report.diagnostics
    );
    let coverage = report.coverage.as_ref().unwrap();
    assert_eq!(coverage.analyzed_frames, frames as u64);
    assert_eq!(
        coverage.decoded_pcm_sha256,
        format!("{:x}", Sha256::digest(&pcm[..frames * channels * 4]))
    );
    assert_eq!(coverage.reached_end, !prefix);
    assert_eq!(coverage.analysis_passes, 2);
    report
}

fn failed(bytes: Vec<u8>, prefix: bool) {
    let report = run(bytes, prefix);
    assert_eq!(
        report.status,
        FileStatus::Failed,
        "{:?}",
        report.diagnostics
    );
    assert!(report.coverage.is_none() && report.channels.is_empty() && report.detectors.is_empty());
    assert!(!report.diagnostics.is_empty());
}

#[test]
fn full_and_prefix_preserve_exact_pcm_when_totals_or_md5_are_absent() {
    for channels in [1, 2] {
        for unknown in [false, true] {
            for checksum in [false, true] {
                let (header, frames, pcm) = generated(channels, unknown, checksum);
                let bytes = joined(&header, &frames);
                let full = exact(bytes.clone(), &pcm, 1097, channels, false);
                assert_eq!(
                    full.coverage.unwrap().decoder_verification,
                    checksum.then_some(true)
                );
                assert_eq!(
                    full.stream.unwrap().declared_frames,
                    (!unknown).then_some(1097)
                );
                let prefix = exact(bytes, &pcm, 400, channels, true);
                assert!(prefix.coverage.unwrap().decoder_verification.is_none());
            }
        }
    }
}

#[test]
fn variable_blocks_use_sample_numbers_and_preserve_exact_pcm() {
    for channels in [1, 2] {
        let (mut header, mut frames, pcm) = generated(channels, true, true);
        header[8..10].copy_from_slice(&73u16.to_be_bytes());
        for (index, frame) in frames.iter_mut().enumerate() {
            let number = char::from_u32(index as u32 * 256).unwrap();
            let mut encoded = [0; 4];
            let number = number.encode_utf8(&mut encoded).as_bytes();
            let mut changed = vec![255, 249, frame[2], frame[3]];
            changed.extend(number);
            changed.push(frame[5]);
            changed.push(crc(&changed, 8, 7) as u8);
            changed.extend(&frame[7..frame.len() - 2]);
            changed.extend((crc(&changed, 16, 0x8005) as u16).to_be_bytes());
            *frame = changed;
        }
        for (offset, size) in [
            (12, frames.iter().map(Vec::len).min().unwrap()),
            (15, frames.iter().map(Vec::len).max().unwrap()),
        ] {
            header[offset..offset + 3].copy_from_slice(&(size as u32).to_be_bytes()[1..]);
        }
        let bytes = joined(&header, &frames);
        exact(bytes.clone(), &pcm, 1097, channels, false);
        exact(bytes, &pcm, 400, channels, true);
        frames.remove(1);
        failed(joined(&header, &frames), true);
    }
}

#[test]
fn missing_corrupt_duplicate_and_reordered_frames_fail_inside_the_interval() {
    for unknown in [false, true] {
        let (header, frames, _) = generated(2, unknown, false);
        for index in [0, 1] {
            let mut changed = frames.clone();
            changed.remove(index);
            failed(joined(&header, &changed), true);
            let mut changed = frames.clone();
            let last = changed[index].len() - 1;
            changed[index][last] ^= 1;
            failed(joined(&header, &changed), true);
            let mut changed = frames.clone();
            changed[index][6] ^= 1;
            failed(joined(&header, &changed), true);
        }
        let mut changed = frames.clone();
        changed.swap(0, 1);
        failed(joined(&header, &changed), false);
        let mut changed = frames.clone();
        changed.insert(1, frames[0].clone());
        failed(joined(&header, &changed), false);
    }
}

#[test]
fn unmeasured_late_damage_and_wrong_md5_do_not_poison_a_prefix() {
    for unknown in [false, true] {
        let (mut header, mut frames, pcm) = generated(1, unknown, true);
        header[26] ^= 1;
        exact(joined(&header, &frames), &pcm, 400, 1, true);
        failed(joined(&header, &frames), false);
        let last = frames[3].len() - 1;
        frames[3][last] ^= 1;
        exact(joined(&header, &frames), &pcm, 400, 1, true);
        failed(joined(&header, &frames), false);
    }
}

#[test]
fn partial_tail_and_skipped_bytes_cannot_be_a_clean_eof_without_md5_or_total() {
    let (header, frames, pcm) = generated(1, true, false);
    let mut bytes = joined(&header, &frames);
    bytes.pop();
    failed(bytes, false);
    let mut bytes = joined(&header, &frames);
    bytes.extend(b"generated-junk");
    failed(bytes, false);
    let mut bytes = header.clone();
    bytes.extend(b"generated-junk");
    bytes.extend(frames.iter().flatten());
    failed(bytes, true);
    let mut bytes = header;
    bytes.extend(&frames[0]);
    bytes.extend(b"generated-junk");
    bytes.extend(frames[1..].iter().flatten());
    failed(bytes, true);
    // With neither a declared total nor MD5, a byte-complete shorter stream is
    // legitimate. No original duration/checksum can be inferred from it.
    let (header, _, _) = generated(1, true, false);
    let report = exact(joined(&header, &frames[..4]), &pcm, 1024, 1, false);
    let coverage = report.coverage.unwrap();
    assert!(coverage.length_matches_header.is_none() && coverage.decoder_verification.is_none());
}

#[test]
fn valid_crc_does_not_allow_hidden_frames_or_zero_byte_padding() {
    for channels in [1, 2] {
        for checksum in [false, true] {
            let (header, frames, pcm) = generated(channels, true, checksum);
            let mut hidden = frames[0].clone();
            hidden[2] = 0x65; // Valid frame at another rate, ignored as a boundary.
            hidden[6] = crc(&hidden[..6], 8, 7) as u8;
            let footer = hidden.len() - 2;
            let checksum = (crc(&hidden[..footer], 16, 0x8005) as u16).to_be_bytes();
            hidden[footer..].copy_from_slice(&checksum);
            for extra in [hidden, vec![0; 16]] {
                let mut bytes = header.clone();
                bytes.extend(&frames[0]);
                bytes.extend(extra);
                bytes.extend(frames[1..].iter().flatten());
                failed(bytes.clone(), false);
                failed(bytes, true);
            }
            let mut bytes = joined(&header, &frames);
            bytes.extend([0; 16]);
            exact(bytes.clone(), &pcm, 400, channels, true);
            failed(bytes, false);
        }
    }
}

#[test]
fn frame_rate_must_match_streaminfo_even_when_all_frame_headers_agree() {
    let (header, mut frames, _) = generated(1, true, false);
    for frame in &mut frames {
        frame[2] = 0x65; // 16 kHz frames contradict the 8 kHz STREAMINFO.
        frame[6] = crc(&frame[..6], 8, 7) as u8;
        let footer = frame.len() - 2;
        let checksum = (crc(&frame[..footer], 16, 0x8005) as u16).to_be_bytes();
        frame[footer..].copy_from_slice(&checksum);
    }
    failed(joined(&header, &frames), false);
    failed(joined(&header, &frames), true);
}

fn rate_header(frame: &[u8], code: u8, extra: &[u8], inherit_precision: bool) -> Vec<u8> {
    let mut out = frame[..6].to_vec();
    out[2] = 0x60 | code;
    if inherit_precision {
        out[3] &= 0xf0;
    }
    out.extend(extra);
    out.push(crc(&out, 8, 7) as u8);
    out.extend(&frame[7..frame.len() - 2]);
    out.extend((crc(&out, 16, 0x8005) as u16).to_be_bytes());
    out
}

#[test]
fn equivalent_rate_and_precision_headers_preserve_pcm() {
    let encodings: &[(u8, &[u8])] = &[
        (0, &[]),            // STREAMINFO rate.
        (4, &[]),            // Fixed 8 kHz code.
        (12, &[8]),          // Explicit kHz.
        (13, &[0x1f, 0x40]), // Explicit Hz.
        (14, &[3, 0x20]),    // Explicit tens of Hz.
    ];
    for channels in [1, 2] {
        for inherit_precision in [false, true] {
            let (mut header, frames, pcm) = generated(channels, true, false);
            // Frame byte sizes are optional; extended headers change them.
            header[12..18].fill(0);
            for &(code, extra) in encodings {
                let changed: Vec<_> = frames
                    .iter()
                    .map(|f| rate_header(f, code, extra, inherit_precision))
                    .collect();
                exact(joined(&header, &changed), &pcm, 1097, channels, false);
                exact(joined(&header, &changed), &pcm, 400, channels, true);
            }
            let mixed: Vec<_> = frames
                .iter()
                .zip(encodings)
                .map(|(f, &(code, extra))| rate_header(f, code, extra, inherit_precision))
                .collect();
            exact(joined(&header, &mixed), &pcm, 1097, channels, false);
            exact(joined(&header, &mixed), &pcm, 400, channels, true);
        }
    }
}

#[test]
fn contradictory_and_reserved_rates_never_produce_measurements() {
    let encodings: &[(u8, &[u8])] = &[
        (1, &[]),
        (2, &[]),
        (3, &[]),
        (5, &[]),
        (6, &[]),
        (7, &[]),
        (8, &[]),
        (9, &[]),
        (10, &[]),
        (11, &[]),
        (12, &[16]),
        (13, &[0x3e, 0x80]),
        (14, &[6, 0x40]),
        (12, &[0]),
        (13, &[0, 0]),
        (14, &[0, 0]),
        (15, &[]),
    ];
    for channels in [1, 2] {
        for unknown in [false, true] {
            let (mut header, frames, _) = generated(channels, unknown, false);
            header[12..18].fill(0);
            for &(code, extra) in encodings {
                let changed: Vec<_> = frames
                    .iter()
                    .map(|f| rate_header(f, code, extra, false))
                    .collect();
                failed(joined(&header, &changed), false);
                failed(joined(&header, &changed), true);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum OnRewind {
    SeekError,
    ReadError,
    MetadataChange,
    Cancel,
}

struct RewindSource {
    cursor: Cursor<Vec<u8>>,
    zero_seeks: usize,
    action: OnRewind,
    cancel: CancellationToken,
    triggered: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Read for RewindSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.zero_seeks >= 3 && matches!(self.action, OnRewind::ReadError) {
            self.triggered
                .store(true, std::sync::atomic::Ordering::SeqCst);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "FLAC rewind read control",
            ));
        }
        let n = out.len().min(7);
        self.cursor.read(&mut out[..n])
    }
}
impl Seek for RewindSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        if pos == SeekFrom::Start(0) {
            self.zero_seeks += 1;
            if self.zero_seeks == 3 {
                match self.action {
                    OnRewind::SeekError => {
                        self.triggered
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                        return Err(io::Error::other("FLAC rewind seek control"));
                    }
                    OnRewind::MetadataChange => {
                        self.cursor.get_mut()[26] ^= 1;
                        self.triggered
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    OnRewind::Cancel => {
                        self.cancel.cancel();
                        self.triggered
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    OnRewind::ReadError => {}
                }
            }
        }
        self.cursor.seek(pos)
    }
}
impl MediaSource for RewindSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

#[test]
fn flac_reprobe_preserves_io_errors_metadata_identity_and_cancellation() {
    for action in [
        OnRewind::SeekError,
        OnRewind::ReadError,
        OnRewind::MetadataChange,
        OnRewind::Cancel,
    ] {
        let (header, frames, pcm) = generated(1, true, false);
        let cancel = CancellationToken::default();
        let triggered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let report = analyze_source(
            Box::new(RewindSource {
                cursor: Cursor::new(joined(&header, &frames)),
                zero_seeks: 0,
                action,
                cancel: cancel.clone(),
                triggered: triggered.clone(),
            }),
            "rewind-control.flac",
            &AnalysisOptions::default(),
            &cancel,
        );
        assert!(triggered.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(
            report.status,
            if matches!(action, OnRewind::Cancel) {
                FileStatus::Cancelled
            } else {
                FileStatus::Failed
            },
            "{:?}",
            report.diagnostics
        );
        assert!(report.coverage.is_none() && report.detectors.is_empty());
        let diagnostic = &report.diagnostics[0].message;
        assert!(
            diagnostic.contains(match action {
                OnRewind::SeekError => "FLAC rewind seek control",
                OnRewind::ReadError => "FLAC rewind read control",
                OnRewind::MetadataChange => "STREAMINFO",
                OnRewind::Cancel => "cancelled",
            }),
            "{diagnostic}"
        );
        // A failed/cancelled FLAC call must release the shared worker.
        exact(joined(&header, &frames), &pcm, 1097, 1, false);
    }
}
