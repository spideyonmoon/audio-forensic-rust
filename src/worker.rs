//! One active core analysis per process; waiting stays cooperative and bounded.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub(crate) static ANALYSIS_WORKER: WorkerBudget = WorkerBudget::new();

pub(crate) struct WorkerBudget {
    active: AtomicBool,
}
pub(crate) struct WorkerPermit<'a> {
    budget: &'a WorkerBudget,
}

impl WorkerBudget {
    const fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }

    pub fn acquire<E>(
        &self,
        mut check: impl FnMut() -> Result<(), E>,
    ) -> Result<WorkerPermit<'_>, E> {
        loop {
            check()?;
            if self
                .active
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(WorkerPermit { budget: self });
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for WorkerPermit<'_> {
    fn drop(&mut self) {
        self.budget.active.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permit_lifetime_and_wait_checks_bound_active_work() {
        let b = WorkerBudget::new();
        let first = b.acquire(|| Ok::<_, &'static str>(())).unwrap();
        let mut calls = 0;
        let waiting = b.acquire(|| {
            calls += 1;
            if calls == 3 { Err("cancelled") } else { Ok(()) }
        });
        assert!(matches!(waiting, Err("cancelled")));
        assert_eq!(calls, 3);
        drop(first);
        assert!(b.acquire(|| Ok::<_, &'static str>(())).is_ok());
    }
}
