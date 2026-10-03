use audio_forensic::{AnalysisOptions, CancellationToken, FileStatus, MediaSource, analyze_source};
use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

type Gate = Arc<(Mutex<bool>, Condvar)>;
struct BlockingSource {
    cursor: Cursor<Vec<u8>>,
    started: Gate,
    release: Gate,
}
impl Read for BlockingSource {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let (lock, signal) = &*self.started;
        *lock.lock().unwrap() = true;
        signal.notify_all();
        let (lock, signal) = &*self.release;
        let mut released = lock.lock().unwrap();
        while !*released {
            released = signal.wait(released).unwrap();
        }
        self.cursor.read(out)
    }
}
impl Seek for BlockingSource {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(p)
    }
}
impl MediaSource for BlockingSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.cursor.get_ref().len() as u64)
    }
}
struct CountingSource {
    cursor: Cursor<Vec<u8>>,
    reads: Arc<AtomicUsize>,
}
impl Read for CountingSource {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.cursor.read(out)
    }
}
impl Seek for CountingSource {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(p)
    }
}
impl MediaSource for CountingSource {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.cursor.get_ref().len() as u64)
    }
}
struct ReleaseOnDrop(Gate);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        let (lock, signal) = &*self.0;
        *lock.lock().unwrap() = true;
        signal.notify_all();
    }
}
fn wav() -> Vec<u8> {
    let mut x = Vec::new();
    x.extend(b"RIFF");
    x.extend(236u32.to_le_bytes());
    x.extend(b"WAVEfmt ");
    x.extend(16u32.to_le_bytes());
    x.extend(1u16.to_le_bytes());
    x.extend(1u16.to_le_bytes());
    x.extend(48000u32.to_le_bytes());
    x.extend(96000u32.to_le_bytes());
    x.extend(2u16.to_le_bytes());
    x.extend(16u16.to_le_bytes());
    x.extend(b"data");
    x.extend(200u32.to_le_bytes());
    for _ in 0..100 {
        x.extend(100i16.to_le_bytes());
    }
    x
}

#[test]
fn queued_calls_cancel_and_expire_before_reading_or_allocating_analysis_state() {
    let started: Gate = Arc::new((Mutex::new(false), Condvar::new()));
    let release: Gate = Arc::new((Mutex::new(false), Condvar::new()));
    let release_guard = ReleaseOnDrop(release.clone());
    let owner = BlockingSource {
        cursor: Cursor::new(wav()),
        started: started.clone(),
        release,
    };
    let task = std::thread::spawn(move || {
        analyze_source(
            Box::new(owner),
            "owner.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        )
    });
    let (lock, signal) = &*started;
    let (ready, timeout) = signal
        .wait_timeout_while(lock.lock().unwrap(), Duration::from_secs(5), |ready| {
            !*ready
        })
        .unwrap();
    assert!(!timeout.timed_out() && *ready);
    drop(ready);
    let reads = Arc::new(AtomicUsize::new(0));
    let cancel = CancellationToken::default();
    let trigger = cancel.clone();
    let cancelling = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        trigger.cancel();
    });
    let r = analyze_source(
        Box::new(CountingSource {
            cursor: Cursor::new(wav()),
            reads: reads.clone(),
        }),
        "queued.wav",
        &AnalysisOptions::default(),
        &cancel,
    );
    cancelling.join().unwrap();
    assert_eq!(r.status, FileStatus::Cancelled);
    assert_eq!(reads.load(Ordering::Relaxed), 0);
    let r = analyze_source(
        Box::new(CountingSource {
            cursor: Cursor::new(wav()),
            reads: reads.clone(),
        }),
        "queued-deadline.wav",
        &AnalysisOptions {
            deadline: Duration::from_millis(10),
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::TimedOut);
    assert_eq!(reads.load(Ordering::Relaxed), 0);
    drop(release_guard);
    assert_eq!(task.join().unwrap().status, FileStatus::Analyzed);
    let r = analyze_source(
        Box::new(CountingSource {
            cursor: Cursor::new(wav()),
            reads: reads.clone(),
        }),
        "after-owner.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed);
    assert!(reads.load(Ordering::Relaxed) > 0);
}
