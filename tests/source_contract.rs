use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, MediaSource, analyze_source,
};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Cursor, ErrorKind, Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Preflight,
    First,
    Second,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    ReadError(Phase, u64, ErrorKind),
    SeekError(Phase),
    Mutate(usize),
    Truncate,
    Cancel(Phase, u64),
    InterruptUntilCancelled(Phase, u64, usize),
}

#[derive(Default, Debug)]
struct Evidence {
    reads: usize,
    seeks: usize,
    triggers: usize,
    rewound_for_second: bool,
}

struct Source {
    cursor: Cursor<Vec<u8>>,
    phase: Phase,
    zero_seeks: usize,
    chunk: usize,
    length: Option<u64>,
    seekable: bool,
    action: Option<Action>,
    cancel: CancellationToken,
    evidence: Arc<Mutex<Evidence>>,
}

impl Source {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            length: Some(bytes.len() as u64),
            cursor: Cursor::new(bytes),
            phase: Phase::Preflight,
            zero_seeks: 0,
            chunk: 256,
            seekable: true,
            action: None,
            cancel: CancellationToken::default(),
            evidence: Arc::default(),
        }
    }

    fn triggered(&mut self) {
        self.action = None;
        self.evidence.lock().unwrap().triggers += 1;
    }
}

impl Read for Source {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.evidence.lock().unwrap().reads += 1;
        if out.is_empty() {
            return Ok(0);
        }
        let mut limit = out.len().min(self.chunk);
        match self.action {
            Some(Action::InterruptUntilCancelled(phase, at, remaining)) if phase == self.phase => {
                if self.cursor.position() >= at {
                    if remaining == 1 {
                        self.cancel.cancel();
                        self.triggered();
                    } else {
                        self.action =
                            Some(Action::InterruptUntilCancelled(phase, at, remaining - 1));
                    }
                    return Err(io::Error::from(ErrorKind::Interrupted));
                }
                limit = limit.min((at - self.cursor.position()) as usize);
            }
            Some(Action::ReadError(phase, at, kind)) if phase == self.phase => {
                if self.cursor.position() >= at {
                    self.triggered();
                    return Err(io::Error::new(kind, "generated source read error"));
                }
                limit = limit.min((at - self.cursor.position()) as usize);
            }
            Some(Action::Cancel(phase, at)) if phase == self.phase => {
                if self.cursor.position() >= at {
                    self.triggered();
                    self.cancel.cancel();
                } else {
                    limit = limit.min((at - self.cursor.position()) as usize);
                }
            }
            _ => {}
        }
        self.cursor.read(&mut out[..limit])
    }
}

impl Seek for Source {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.evidence.lock().unwrap().seeks += 1;
        // Native WAV seeks to byte 44 for the independent second scan. This is
        // a source-I/O control, separate from the decoder's internal packets.
        let second = self.phase == Phase::First && pos == SeekFrom::Start(44);
        if second {
            self.phase = Phase::Second;
            self.evidence.lock().unwrap().rewound_for_second = true;
        }
        if let Some(Action::SeekError(phase)) = self.action {
            if phase == self.phase {
                self.triggered();
                return Err(io::Error::other("generated source seek error"));
            }
        }
        if second {
            match self.action {
                Some(Action::Mutate(offset)) => {
                    self.cursor.get_mut()[offset] ^= 1;
                    self.triggered();
                }
                Some(Action::Truncate) => {
                    let len = self.cursor.get_ref().len();
                    self.cursor.get_mut().truncate(len - 2);
                    self.triggered();
                }
                _ => {}
            }
        }
        let result = self.cursor.seek(pos)?;
        if pos == SeekFrom::Start(0) {
            self.zero_seeks += 1;
            if self.zero_seeks == 2 {
                self.phase = Phase::First;
            }
        }
        Ok(result)
    }
}

impl MediaSource for Source {
    fn is_seekable(&self) -> bool {
        self.seekable
    }
    fn byte_len(&self) -> Option<u64> {
        self.length
    }
}

fn wav() -> (Vec<u8>, String) {
    let words: Vec<i16> = (0..4_096)
        .map(|i| ((i * 71) % 32_000 - 16_000) as i16)
        .collect();
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend((36 + words.len() as u32 * 2).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(8_000_u32.to_le_bytes());
    bytes.extend(16_000_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((words.len() as u32 * 2).to_le_bytes());
    let mut hash = Sha256::new();
    for word in words {
        bytes.extend(word.to_le_bytes());
        hash.update((i32::from(word) << 16).to_le_bytes());
    }
    (bytes, format!("{:x}", hash.finalize()))
}

fn run(source: Source, options: &AnalysisOptions) -> (AnalysisReport, Arc<Mutex<Evidence>>) {
    let evidence = source.evidence.clone();
    let cancel = source.cancel.clone();
    let report = analyze_source(
        Box::new(source),
        "display-name-not-a-path.bin",
        options,
        &cancel,
    );
    assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
    assert!(report.evidence_index.is_none());
    if let Some(directory) = std::env::var_os("SOURCE_CONTRACT_OUTPUT_DIR") {
        use std::{
            fs::OpenOptions,
            io::Write,
            path::Path,
            sync::atomic::{AtomicUsize, Ordering},
        };
        static NUMBER: AtomicUsize = AtomicUsize::new(0);
        let directory = Path::new(&directory).canonicalize().unwrap();
        let private = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("corpus/local")
            .canonicalize()
            .unwrap();
        assert!(directory.starts_with(private));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!(
                "{:03}.json",
                NUMBER.fetch_add(1, Ordering::Relaxed)
            )))
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
            .unwrap();
        file.write_all(b"\n").unwrap();
    }
    (report, evidence)
}

fn verify_worker_released() {
    let (bytes, hash) = wav();
    let (report, _) = run(
        Source::new(bytes),
        &AnalysisOptions {
            deadline: Duration::from_secs(5),
            ..Default::default()
        },
    );
    assert_eq!(
        report.status,
        FileStatus::Analyzed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.coverage.unwrap().decoded_pcm_sha256, hash);
}

fn assert_failed(report: &AnalysisReport, expected_message: &str) {
    assert_eq!(
        report.status,
        FileStatus::Failed,
        "{:?}",
        report.diagnostics
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "decode_error" && d.message.contains(expected_message)),
        "{:?}",
        report.diagnostics
    );
    assert!(report.coverage.is_none());
    assert!(report.channels.is_empty());
    assert!(report.detectors.is_empty());
}

#[test]
fn short_reads_and_unknown_or_overestimated_lengths_preserve_exact_pcm() {
    for chunk in [1, 3, 7, 256] {
        for length in [None, Some(8_236), Some(u64::MAX)] {
            let (bytes, hash) = wav();
            let mut source = Source::new(bytes);
            source.chunk = chunk;
            source.length = length;
            let (report, evidence) = run(source, &AnalysisOptions::default());
            assert_eq!(
                report.status,
                FileStatus::Analyzed,
                "chunk={chunk} length={length:?}: {:?}",
                report.diagnostics
            );
            let coverage = report.coverage.unwrap();
            assert_eq!(coverage.analyzed_frames, 4_096);
            assert_eq!(coverage.decoded_pcm_sha256, hash);
            assert_eq!(coverage.length_matches_header, Some(true));
            assert!(evidence.lock().unwrap().rewound_for_second);
        }
    }
}

#[test]
fn preflight_probe_and_each_pass_read_failures_are_structured_and_release_worker() {
    for (phase, at, expected) in [
        (Phase::Preflight, 0, "generated source read error"),
        (Phase::First, 0, "generated source read error"),
        (Phase::First, 512, "generated source read error"),
        (Phase::Second, 512, "generated source read error"),
    ] {
        let (bytes, _) = wav();
        let mut source = Source::new(bytes);
        source.action = Some(Action::ReadError(phase, at, ErrorKind::PermissionDenied));
        let (report, evidence) = run(source, &AnalysisOptions::default());
        assert_failed(&report, expected);
        assert_eq!(evidence.lock().unwrap().triggers, 1);
        verify_worker_released();
    }
}

#[test]
fn interrupted_reads_are_retried_without_losing_pcm() {
    for phase in [Phase::Preflight, Phase::First, Phase::Second] {
        let (bytes, hash) = wav();
        let mut source = Source::new(bytes);
        source.action = Some(Action::ReadError(
            phase,
            if phase == Phase::Preflight { 0 } else { 512 },
            ErrorKind::Interrupted,
        ));
        let (report, evidence) = run(source, &AnalysisOptions::default());
        assert_eq!(
            report.status,
            FileStatus::Analyzed,
            "{phase:?}: {:?}",
            report.diagnostics
        );
        assert_eq!(report.coverage.unwrap().decoded_pcm_sha256, hash);
        assert_eq!(evidence.lock().unwrap().triggers, 1);
    }
}

#[test]
fn repeated_interrupts_remain_cancellable_and_raw_eof_errors_are_failures() {
    for phase in [Phase::First, Phase::Second] {
        for action in [
            Action::InterruptUntilCancelled(phase, 512, 8),
            Action::ReadError(phase, 512, ErrorKind::UnexpectedEof),
        ] {
            let (bytes, _) = wav();
            let mut source = Source::new(bytes);
            source.action = Some(action);
            let (report, evidence) = run(source, &AnalysisOptions::default());
            if matches!(action, Action::ReadError(..)) {
                assert_failed(&report, "generated source read error");
            } else {
                assert_eq!(
                    report.status,
                    FileStatus::Cancelled,
                    "{:?}",
                    report.diagnostics
                );
                assert!(report.coverage.is_none());
                assert!(evidence.lock().unwrap().reads < 100);
            }
            assert_eq!(evidence.lock().unwrap().triggers, 1);
            verify_worker_released();
        }
    }
}

#[test]
fn seek_failures_and_underreported_length_are_explicit() {
    for phase in [Phase::Preflight, Phase::Second] {
        let (bytes, _) = wav();
        let mut source = Source::new(bytes);
        source.action = Some(Action::SeekError(phase));
        let (report, evidence) = run(source, &AnalysisOptions::default());
        assert_failed(&report, "generated source seek error");
        assert_eq!(evidence.lock().unwrap().triggers, 1);
        verify_worker_released();
    }
    let (bytes, _) = wav();
    let mut source = Source::new(bytes);
    source.length = Some(20);
    let (report, _) = run(source, &AnalysisOptions::default());
    assert_failed(&report, "truncated ancillary block");
    verify_worker_released();
}

#[test]
fn mutation_and_truncation_between_passes_never_publish_successful_measurements() {
    for action in [Action::Mutate(44), Action::Mutate(8_234), Action::Truncate] {
        let (bytes, _) = wav();
        let mut source = Source::new(bytes);
        source.action = Some(action);
        let (report, evidence) = run(source, &AnalysisOptions::default());
        assert_failed(
            &report,
            if matches!(action, Action::Truncate) {
                "stream may be truncated"
            } else {
                "Source changed between analysis passes"
            },
        );
        assert_eq!(evidence.lock().unwrap().triggers, 1);
        verify_worker_released();
    }
}

#[test]
fn prefix_contract_detects_inside_changes_and_excludes_unanalyzed_tail() {
    for (offset, expected) in [(44, FileStatus::Failed), (8_234, FileStatus::Analyzed)] {
        let (bytes, _) = wav();
        let mut source = Source::new(bytes);
        source.action = Some(Action::Mutate(offset));
        let (report, evidence) = run(
            source,
            &AnalysisOptions {
                max_seconds: Some(0.1),
                ..Default::default()
            },
        );
        assert_eq!(report.status, expected, "{:?}", report.diagnostics);
        if expected == FileStatus::Failed {
            assert_failed(&report, "Source changed between analysis passes");
        } else {
            let coverage = report.coverage.unwrap();
            assert_eq!(coverage.analyzed_frames, 800);
            assert!(!coverage.reached_end);
            assert!(coverage.length_matches_header.is_none());
        }
        assert_eq!(evidence.lock().unwrap().triggers, 1);
        verify_worker_released();
    }
}

#[test]
fn mid_read_cancellation_on_each_scan_releases_worker() {
    for phase in [Phase::First, Phase::Second] {
        let (bytes, _) = wav();
        let mut source = Source::new(bytes);
        source.action = Some(Action::Cancel(phase, 512));
        let (report, evidence) = run(source, &AnalysisOptions::default());
        assert_eq!(
            report.status,
            FileStatus::Cancelled,
            "{:?}",
            report.diagnostics
        );
        assert!(report.coverage.is_none());
        assert!(report.channels.is_empty());
        assert_eq!(evidence.lock().unwrap().triggers, 1);
        verify_worker_released();
    }
}

#[test]
fn nonseekable_invalid_options_and_precancelled_sources_are_not_read() {
    for case in 0..3 {
        let (bytes, _) = wav();
        let mut source = Source::new(bytes);
        let mut options = AnalysisOptions::default();
        let expected = match case {
            0 => {
                source.seekable = false;
                FileStatus::Unsupported
            }
            1 => {
                options.max_seconds = Some(f64::NAN);
                FileStatus::Failed
            }
            _ => {
                source.cancel.cancel();
                FileStatus::Cancelled
            }
        };
        let (report, evidence) = run(source, &options);
        assert_eq!(report.status, expected);
        assert!(!report.diagnostics.is_empty());
        let evidence = evidence.lock().unwrap();
        assert_eq!(evidence.reads, 0);
        assert_eq!(evidence.seeks, 0);
    }
}

#[test]
fn unsuccessful_float_reports_preserve_known_native_precision() {
    for action in [
        None,
        Some(Action::ReadError(
            Phase::First,
            512,
            ErrorKind::PermissionDenied,
        )),
        Some(Action::ReadError(
            Phase::Second,
            512,
            ErrorKind::PermissionDenied,
        )),
    ] {
        let (mut bytes, _) = wav();
        bytes.truncate(44);
        bytes[4..8].copy_from_slice(&(36 + 4_096 * 4_u32).to_le_bytes());
        bytes[20..22].copy_from_slice(&3_u16.to_le_bytes());
        bytes[28..32].copy_from_slice(&32_000_u32.to_le_bytes());
        bytes[32..34].copy_from_slice(&4_u16.to_le_bytes());
        bytes[34..36].copy_from_slice(&32_u16.to_le_bytes());
        bytes[40..44].copy_from_slice(&(4_096 * 4_u32).to_le_bytes());
        for index in 0..4_096 {
            let value = if action.is_none() && index == 128 {
                f32::NAN
            } else {
                0.25_f32
            };
            bytes.extend(value.to_le_bytes());
        }
        let mut source = Source::new(bytes);
        source.action = action;
        let (report, _) = run(source, &AnalysisOptions::default());
        assert_eq!(
            report.status,
            FileStatus::Failed,
            "{:?}",
            report.diagnostics
        );
        assert!(
            !report.stream.unwrap().integer_pcm,
            "failed float metadata must not claim integer PCM"
        );
        assert!(report.coverage.is_none());
        assert!(report.channels.is_empty());
        verify_worker_released();
    }
}
