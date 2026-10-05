//! Host notifications, independent of the serialized measurement report.
use crate::FileStatus;
use std::time::{Duration, Instant};

/// A synchronous notification on the thread performing analysis.
/// Frame counts are per pass, not elapsed-time estimates. Expected lengths can
/// be unknown or inaccurate; only the final report establishes actual coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalysisProgress {
    WaitingForWorker,
    ReadingMetadata,
    Decoding {
        pass: u8,
        processed_frames: u64,
        expected_frames: Option<u64>,
    },
    AnalyzingDetectors,
    Finished {
        status: FileStatus,
    },
}

pub(crate) struct Progress<'a> {
    callback: &'a mut dyn FnMut(AnalysisProgress),
    last_update: Instant,
}

impl<'a> Progress<'a> {
    pub(crate) fn new(callback: &'a mut dyn FnMut(AnalysisProgress)) -> Self {
        Self {
            callback,
            last_update: Instant::now(),
        }
    }

    pub(crate) fn emit(&mut self, event: AnalysisProgress) {
        (self.callback)(event);
        self.last_update = Instant::now();
    }

    pub(crate) fn frames(
        &mut self,
        pass: u8,
        processed_frames: u64,
        expected_frames: Option<u64>,
        force: bool,
    ) {
        if force || self.last_update.elapsed() >= Duration::from_millis(100) {
            self.emit(AnalysisProgress::Decoding {
                pass,
                processed_frames,
                expected_frames,
            });
        }
    }
}
