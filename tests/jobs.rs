use audio_forensic::{
    AnalysisJob, AnalysisJobError, AnalysisOptions, AnalysisProgress, AnalysisReport,
    CancellationToken, FileStatus, MediaSource, StartJobError, analyze_source,
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom},
    sync::{Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

// Admission is deliberately process-wide. Serialize only this test binary's
// independent scenarios so ordinary parallel cargo test remains supported.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn wav() -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(4036u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(4000u32.to_le_bytes());
    for i in 0..2000 {
        bytes.extend(((i * 71 % 32000) as i16 - 16000).to_le_bytes());
    }
    bytes
}

fn finish(job: &mut AnalysisJob) -> Result<AnalysisReport, AnalysisJobError> {
    let limit = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(result) = job.try_finish() {
            assert!(job.is_finished());
            assert!(job.try_finish().is_none(), "result must be consumed once");
            return result;
        }
        assert!(Instant::now() < limit, "job did not finish");
        thread::sleep(Duration::from_millis(1));
    }
}

fn start() -> AnalysisJob {
    AnalysisJob::spawn_source(
        Box::new(Cursor::new(wav())),
        "generated",
        Default::default(),
    )
    .unwrap_or_else(|error| panic!("could not start: {error}"))
}

#[test]
fn job_preserves_exact_report_and_retains_terminal_progress() {
    let _serial = TEST_LOCK.lock().unwrap();
    let expected = analyze_source(
        Box::new(Cursor::new(wav())),
        "generated",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    let mut job = start();
    let report = finish(&mut job).unwrap();
    assert_eq!(report.status, FileStatus::Analyzed);
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    job.cancel();
    assert_eq!(
        job.progress(),
        Some(AnalysisProgress::Finished {
            status: FileStatus::Analyzed
        })
    );
    // Keeping a completed handle does not retain background admission.
    assert_eq!(finish(&mut start()).unwrap().status, FileStatus::Analyzed);
}

struct GatedSource {
    cursor: Cursor<Vec<u8>>,
    entered: mpsc::Sender<()>,
    release: Option<Mutex<mpsc::Receiver<()>>>,
    dropped: mpsc::Sender<()>,
    panic_on_read: bool,
}

impl Read for GatedSource {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if let Some(release) = self.release.take() {
            let _ = self.entered.send(());
            let _ = release.into_inner().unwrap().recv();
        }
        assert!(!self.panic_on_read, "generated host-source panic");
        self.cursor.read(bytes)
    }
}

impl Seek for GatedSource {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(position)
    }
}

impl MediaSource for GatedSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.cursor.get_ref().len() as u64)
    }
}

impl Drop for GatedSource {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}

#[test]
fn busy_cancel_and_drop_keep_input_bounded_until_io_returns() {
    let _serial = TEST_LOCK.lock().unwrap();
    for drop_handle in [false, true] {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let source = GatedSource {
            cursor: Cursor::new(wav()),
            entered: entered_tx,
            release: Some(Mutex::new(release_rx)),
            dropped: dropped_tx,
            panic_on_read: false,
        };
        let mut job = AnalysisJob::spawn_source(Box::new(source), "blocked", Default::default())
            .unwrap_or_else(|e| panic!("{e}"));
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!job.is_finished());
        assert!(job.try_finish().is_none());
        assert!(job.progress().is_some());
        assert!(matches!(
            AnalysisJob::spawn_path("unused.wav", Default::default()),
            Err(StartJobError::Busy)
        ));
        if drop_handle {
            // Teardown must return even while the source cannot yet respond.
            let (done_tx, done_rx) = mpsc::channel();
            let teardown = thread::spawn(move || {
                drop(job);
                done_tx.send(()).unwrap();
            });
            done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            teardown.join().unwrap();
            assert!(matches!(
                AnalysisJob::spawn_path("unused.wav", Default::default()),
                Err(StartJobError::Busy)
            ));
            assert!(dropped_rx.try_recv().is_err());
            release_tx.send(()).unwrap();
        } else {
            job.cancel();
            assert!(job.try_finish().is_none());
            release_tx.send(()).unwrap();
            assert_eq!(finish(&mut job).unwrap().status, FileStatus::Cancelled);
        }
        dropped_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // The source destructor runs just before the admission guard drops.
        let limit = Instant::now() + Duration::from_secs(5);
        let mut recovered = loop {
            match AnalysisJob::spawn_source(
                Box::new(Cursor::new(wav())),
                "next",
                Default::default(),
            ) {
                Ok(job) => break job,
                Err(StartJobError::Busy) if Instant::now() < limit => thread::yield_now(),
                Err(e) => panic!("admission did not recover: {e}"),
            }
        };
        assert_eq!(finish(&mut recovered).unwrap().status, FileStatus::Analyzed);
    }
}

#[test]
fn source_panic_is_a_host_error_and_releases_admission() {
    let _serial = TEST_LOCK.lock().unwrap();
    let (entered, _) = mpsc::channel();
    let (dropped, dropped_rx) = mpsc::channel();
    let source = GatedSource {
        cursor: Cursor::new(wav()),
        entered,
        release: None,
        dropped,
        panic_on_read: true,
    };
    let mut job = AnalysisJob::spawn_source(Box::new(source), "panic", Default::default())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(matches!(
        finish(&mut job),
        Err(AnalysisJobError::WorkerPanicked)
    ));
    dropped_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!matches!(
        job.progress(),
        Some(AnalysisProgress::Finished { .. })
    ));
    assert_eq!(finish(&mut start()).unwrap().status, FileStatus::Analyzed);
}

#[test]
fn path_open_failures_unsupported_and_deadline_are_file_reports() {
    let _serial = TEST_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("generated.wav");
    let mut missing =
        AnalysisJob::spawn_path(&path, Default::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(finish(&mut missing).unwrap().status, FileStatus::Failed);
    std::fs::write(&path, wav()).unwrap();
    let mut valid =
        AnalysisJob::spawn_path(&path, Default::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(finish(&mut valid).unwrap().status, FileStatus::Analyzed);
    for (bytes, options, status) in [
        (
            vec![0; 100],
            AnalysisOptions::default(),
            FileStatus::Unsupported,
        ),
        (
            wav(),
            AnalysisOptions {
                deadline: Duration::ZERO,
                ..Default::default()
            },
            FileStatus::TimedOut,
        ),
    ] {
        let mut job = AnalysisJob::spawn_source(Box::new(Cursor::new(bytes)), "failure", options)
            .unwrap_or_else(|e| panic!("{e}"));
        let report = finish(&mut job).unwrap();
        assert_eq!(report.status, status);
        assert!(report.coverage.is_none() && report.detectors.is_empty());
        assert_eq!(job.progress(), Some(AnalysisProgress::Finished { status }));
    }
}
