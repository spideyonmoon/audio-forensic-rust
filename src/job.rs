//! Bounded background execution for hosts without an asynchronous runtime.
use crate::{
    AnalysisOptions, AnalysisProgress, AnalysisReport, CancellationToken, MediaSource,
    analyze_path_with_progress, analyze_source_with_progress,
};
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

static BACKGROUND_ACTIVE: AtomicBool = AtomicBool::new(false);

struct Permit;

impl Permit {
    fn acquire() -> Result<Self, StartJobError> {
        BACKGROUND_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| StartJobError::Busy)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        BACKGROUND_ACTIVE.store(false, Ordering::Release);
    }
}

/// Failure to start a background job; no analysis report was produced.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StartJobError {
    #[error("a background analysis is still running")]
    Busy,
    #[error("cannot start analysis worker: {0}")]
    Spawn(#[source] io::Error),
}

/// A host/source panic is distinct from a structured unsuccessful file report.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnalysisJobError {
    #[error("analysis worker panicked")]
    WorkerPanicked,
}

/// One background analysis with replaceable progress and one consumable result.
///
/// At most one unfinished background job is admitted per process. Busy callers
/// must retry later; there is no hidden input queue. Synchronous analysis still
/// shares the core worker budget, so this job may wait for an existing call.
/// Dropping this handle requests cancellation and detaches, without waiting for
/// I/O. Admission remains occupied until the worker releases its source, even
/// after the handle is dropped. Cancellation cannot interrupt blocked source I/O.
pub struct AnalysisJob {
    cancel: CancellationToken,
    progress: Arc<Mutex<Option<AnalysisProgress>>>,
    worker: Option<JoinHandle<AnalysisReport>>,
}

impl AnalysisJob {
    /// Move a seekable source into a background job. On start failure the supplied
    /// source is dropped; obtain a fresh source when retrying. Its Read/Seek and
    /// Drop implementations must obey the source API's cooperative I/O contract.
    pub fn spawn_source(
        source: Box<dyn MediaSource>,
        name: impl Into<String>,
        options: AnalysisOptions,
    ) -> Result<Self, StartJobError> {
        let name = name.into();
        Self::spawn(move |cancel, progress| {
            analyze_source_with_progress(source, &name, &options, cancel, progress)
        })
    }

    /// Open and analyze a path on the worker. Opening is outside the decode
    /// deadline, as in the synchronous path API; open errors are failed reports.
    pub fn spawn_path(
        path: impl Into<PathBuf>,
        options: AnalysisOptions,
    ) -> Result<Self, StartJobError> {
        let path = path.into();
        Self::spawn(move |cancel, progress| {
            analyze_path_with_progress(path, &options, cancel, progress)
        })
    }

    fn spawn(
        run: impl FnOnce(&CancellationToken, &mut dyn FnMut(AnalysisProgress)) -> AnalysisReport
        + Send
        + 'static,
    ) -> Result<Self, StartJobError> {
        let permit = Permit::acquire()?;
        let cancel = CancellationToken::default();
        let worker_cancel = cancel.clone();
        let progress = Arc::new(Mutex::new(None));
        let worker_progress = Arc::clone(&progress);
        let worker = thread::Builder::new()
            .name("audio-analysis".into())
            .spawn(move || {
                // The permit outlives run and its source, including on unwind.
                let _permit = permit;
                run(&worker_cancel, &mut |event| {
                    *worker_progress.lock().unwrap_or_else(|e| e.into_inner()) = Some(event);
                })
            })
            .map_err(StartJobError::Spawn)?;
        Ok(Self {
            cancel,
            progress,
            worker: Some(worker),
        })
    }

    /// Request cooperative cancellation. A completed report is not changed.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Latest notification, or None before the worker emits its first event.
    /// Intermediate events may be replaced before polling; this is not an event
    /// log. A Finished notification can precede result availability briefly.
    /// A panicked worker need not emit Finished; always poll the result too.
    pub fn progress(&self) -> Option<AnalysisProgress> {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// True when the worker has exited, including after taking its result.
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// Take the result once, without waiting for ongoing analysis. None means
    /// either still running or already consumed (distinguish with is_finished).
    /// Cancelled, timed-out, unsupported and failed inputs remain ordinary
    /// reports. Only a worker panic returns AnalysisJobError.
    pub fn try_finish(&mut self) -> Option<Result<AnalysisReport, AnalysisJobError>> {
        if !self.is_finished() {
            return None;
        }
        self.worker
            .take()
            .map(|worker| worker.join().map_err(|_| AnalysisJobError::WorkerPanicked))
    }
}

impl Drop for AnalysisJob {
    fn drop(&mut self) {
        self.cancel();
    }
}
