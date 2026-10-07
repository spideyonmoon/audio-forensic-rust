//! Preserve caller I/O failures across decoder probing and retry transient reads.
use crate::{decode::Failure, model::CancellationToken};
use std::{
    io::{self, ErrorKind, Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use symphonia::core::io::MediaSource;

#[derive(Clone, Default)]
pub(crate) struct SourceFault(Arc<Mutex<Option<Failure>>>);

impl SourceFault {
    fn save(&self, failure: Failure) {
        // Caller implementations can panic; do not mask an earlier source error
        // with a later decoder fallback. No lock is held while calling source I/O.
        let mut fault = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if fault.is_none() {
            *fault = Some(failure);
        }
    }

    pub fn check<T>(&self, result: Result<T, Failure>) -> Result<T, Failure> {
        let fault = self.0.lock().unwrap_or_else(|error| error.into_inner());
        match fault.as_ref() {
            Some(failure) => Err(failure.clone()),
            None => result,
        }
    }
}

pub(crate) struct GuardedSource {
    inner: Box<dyn MediaSource>,
    fault: SourceFault,
    cancel: CancellationToken,
    deadline: Duration,
    start: Instant,
    discovered_len: Option<u64>,
}

impl GuardedSource {
    pub fn new(
        inner: Box<dyn MediaSource>,
        cancel: CancellationToken,
        deadline: Duration,
        start: Instant,
    ) -> (Self, SourceFault) {
        let fault = SourceFault::default();
        (
            Self {
                inner,
                fault: fault.clone(),
                cancel,
                deadline,
                start,
                discovered_len: None,
            },
            fault,
        )
    }

    /// A seek-to-end container preflight established this stable source extent.
    /// MP4's locked reader requires a length even for seekable caller sources.
    pub(crate) fn record_length(&mut self, bytes: u64) {
        self.discovered_len = Some(bytes);
    }

    fn control(&self) -> io::Result<()> {
        let failure = if self.cancel.is_cancelled() {
            Some(Failure::Cancelled)
        } else if self.start.elapsed() >= self.deadline {
            Some(Failure::TimedOut)
        } else {
            None
        };
        if let Some(failure) = failure {
            let message = failure.to_string();
            self.fault.save(failure);
            return Err(io::Error::other(message));
        }
        Ok(())
    }

    fn remember<T>(&self, result: io::Result<T>) -> io::Result<T> {
        if let Err(error) = &result {
            self.fault
                .save(Failure::Decode(format!("Source I/O: {error}")));
        }
        result
    }
}

impl Read for GuardedSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            self.control()?;
            let result = self.inner.read(out);
            if matches!(&result, Err(error) if error.kind() == ErrorKind::Interrupted) {
                continue;
            }
            return self.remember(result);
        }
    }
}

impl Seek for GuardedSource {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.control()?;
        let result = self.inner.seek(position);
        self.remember(result)
    }
}

impl MediaSource for GuardedSource {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len().or(self.discovered_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct InterruptedRead(Arc<AtomicUsize>);
    impl Read for InterruptedRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
                std::thread::sleep(Duration::from_millis(40));
                Err(io::Error::from(ErrorKind::Interrupted))
            } else {
                // Fail promptly if the guard incorrectly retries past deadline.
                Err(io::Error::other("read retried past deadline"))
            }
        }
    }
    impl Seek for InterruptedRead {
        fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
            Ok(0)
        }
    }
    impl MediaSource for InterruptedRead {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            None
        }
    }

    #[test]
    fn read_retry_checks_the_same_cooperative_deadline() {
        let reads = Arc::new(AtomicUsize::new(0));
        let (mut source, fault) = GuardedSource::new(
            Box::new(InterruptedRead(reads.clone())),
            CancellationToken::default(),
            Duration::from_millis(20),
            Instant::now(),
        );
        assert!(source.read(&mut [0]).is_err());
        assert!(matches!(
            fault.check(Ok::<_, Failure>(())),
            Err(Failure::TimedOut)
        ));
        // Scheduling may expire the deadline before the first read. A read that
        // did run blocks past it; no second underlying read may start afterward.
        assert!(reads.load(Ordering::Relaxed) <= 1);
    }
}
