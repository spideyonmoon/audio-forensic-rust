//! Process-local serial admission. No I/O, JNI or joins under the registry lock.
use audio_forensic::CancellationToken;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    thread,
};

pub const CONTROL_LIMIT: usize = 64 * 1024;
pub const DOCUMENT_LIMIT: u64 = 64 * 1024 * 1024;
pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, Clone, Serialize)]
pub struct HostError {
    pub code: String,
    pub message: String,
    pub attempt_id: Option<String>,
    pub phase: String,
}
impl HostError {
    pub fn new(code: &str, message: &str) -> Self {
        let mut end = message.len().min(4096);
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        Self {
            code: code.into(),
            message: message[..end].into(),
            attempt_id: None,
            phase: "request".into(),
        }
    }
    pub fn at(mut self, r: &Request, phase: &str) -> Self {
        self.attempt_id = Some(r.attempt_id.clone());
        self.phase = phase.into();
        self
    }
}
pub type Result<T> = std::result::Result<T, HostError>;
pub fn fail(code: &str, message: &str) -> HostError {
    HostError::new(code, message)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    pub id: String,
    pub attempt_id: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedInput {
    pub root: PathBuf,
    pub payload: Descriptor,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub kind: String,
    pub version: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub kind: String,
    pub job_id: String,
    pub attempt_id: String,
    pub selection_id: String,
    pub generation: u64,
    pub item_ids: Vec<String>,
    pub feature: Option<String>,
    pub leases: Vec<Lease>,
    #[serde(default)]
    pub saved: Vec<SavedInput>,
    pub scope: Scope,
    pub deadline_ms: u64,
    pub output_dir: PathBuf,
    pub available_memory_bytes: u64,
    pub low_memory: bool,
    pub reserved_output_bytes: u64,
    #[serde(default)]
    pub same_track_asserted: bool,
    #[serde(default)]
    pub spectrogram: bool,
    #[serde(default)]
    pub png: bool,
    #[serde(default = "publication")]
    pub preset: String,
}
fn publication() -> String {
    "publication".into()
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scope {
    Full,
    Prefix { seconds: f64 },
}
pub fn uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
impl Request {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > CONTROL_LIMIT {
            return Err(fail("size_limit", "Control request exceeds 64 KiB"));
        }
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }
        let header: Version = serde_json::from_slice(bytes)
            .map_err(|_| fail("invalid_request", "Invalid request version header"))?;
        if header.version != 1 {
            return Err(fail("unsupported_version", "Expected host version 1"));
        }
        let r: Self = serde_json::from_slice(bytes)
            .map_err(|_| fail("invalid_request", "Invalid UTF-8 request JSON or fields"))?;
        r.validate().map_err(|e| e.at(&r, "request"))?;
        Ok(r)
    }
    fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(fail("unsupported_version", "Expected host version 1"));
        }
        let bad = || {
            fail(
                "invalid_request",
                "Invalid operation, identity, scope, lease or reservation",
            )
        };
        if ![&self.job_id, &self.attempt_id, &self.selection_id]
            .into_iter()
            .all(|s| uuid(s))
            || self.item_ids.is_empty()
            || self.item_ids.len() > 32
            || !self.item_ids.iter().all(|s| uuid(s))
            || self
                .item_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != self.item_ids.len()
            || !(1..=1_200_000).contains(&self.deadline_ms)
            || !self.output_dir.is_absolute()
            || !(1..=512 * 1024 * 1024).contains(&self.reserved_output_bytes)
            || !["standard", "publication", "large"].contains(&self.preset.as_str())
        {
            return Err(bad());
        }
        if let Scope::Prefix { seconds } = self.scope {
            if !seconds.is_finite() || seconds <= 0.0 || seconds > 1_200.0 {
                return Err(bad());
            }
        }
        match (self.kind.as_str(), self.feature.as_deref()) {
            ("probe", None) | ("feature", Some("forensics" | "spectrogram")) => {
                if self.leases.len() != 1 || self.item_ids.len() != 1 || !self.saved.is_empty() {
                    return Err(bad());
                }
            }
            ("feature", Some("compare")) => {
                if !self.same_track_asserted
                    || !(2..=32).contains(&self.saved.len())
                    || !self.leases.is_empty()
                    || self.item_ids.len() != self.saved.len()
                {
                    return Err(bad());
                }
            }
            ("feature", Some("render")) => {
                if self.saved.len() != 1
                    || !self.leases.is_empty()
                    || self.item_ids.len() != 1
                    || !self.png
                {
                    return Err(bad());
                }
            }
            _ => return Err(bad()),
        }
        for l in &self.leases {
            if !uuid(&l.id)
                || l.attempt_id != self.attempt_id
                || !l.path.is_absolute()
                || l.bytes > 700 * 1024 * 1024
                || l.sha256.len() != 64
                || !l.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || l.name.len() > 1024
            {
                return Err(bad());
            }
        }
        if self
            .saved
            .iter()
            .any(|s| !s.root.is_absolute() || s.payload.bytes > DOCUMENT_LIMIT)
            || self.saved.iter().map(|s| s.payload.bytes).sum::<u64>() > 128 * 1024 * 1024
        {
            return Err(fail("size_limit", "Saved document limits exceeded"));
        }
        Ok(())
    }
}

pub struct State {
    pub request: Request,
    pub cancel: CancellationToken,
    pub latest: Mutex<Value>,
    pub terminal: Mutex<Option<Value>>,
}
impl State {
    pub fn progress(&self, value: Value) {
        *lock(&self.latest) = value;
    }
}
#[derive(Default)]
struct Registry {
    sequence: i64,
    active: Option<i64>,
    slot: Option<(i64, Arc<State>)>,
    #[cfg(test)]
    fail_spawn: bool,
}
#[derive(Clone, Default)]
pub struct Bridge {
    registry: Arc<Mutex<Registry>>,
}
impl Bridge {
    pub fn describe(&self) -> Value {
        let r = lock(&self.registry);
        json!({"version":1,"engine_version":"0.32.0","payloads":["metadata-v1","audio-forensic-product-v1","spectrogram-v1","audio-forensic-comparison-v1"],"active_handle":r.active})
    }
    pub fn start(&self, bytes: &[u8]) -> Result<i64> {
        let request = Request::parse(bytes)?;
        self.spawn(request, crate::operation::run)
    }
    fn spawn(
        &self,
        request: Request,
        run: impl FnOnce(&State) -> Result<Value> + Send + 'static,
    ) -> Result<i64> {
        let state = Arc::new(State {
            request,
            cancel: CancellationToken::default(),
            latest: Mutex::new(json!({"phase":"starting"})),
            terminal: Mutex::new(None),
        });
        let handle = {
            let mut r = lock(&self.registry);
            if r.active.is_some() || r.slot.is_some() {
                return Err(
                    fail("busy", "Close the previous operation after source release")
                        .at(&state.request, "start"),
                );
            }
            let h = r
                .sequence
                .checked_add(1)
                .ok_or_else(|| fail("resource_limit", "Handle sequence exhausted"))?;
            r.sequence = h;
            r.active = Some(h);
            r.slot = Some((h, state.clone()));
            h
        };
        let registry = self.registry.clone();
        let worker = state.clone();
        let work = move || {
            // All source-owning stack frames unwind before admission is released.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&worker)))
                .unwrap_or_else(|_| Err(fail("worker_panicked", "Native worker panicked")))
                .map_err(|e| e.at(&worker.request, "worker"));
            let value = match result {
                Ok(v) => json!({"status":"completed","result":v}),
                Err(e) => json!({"status":"failed","error":e}),
            };
            *lock(&worker.terminal) = Some(value);
            // Terminal visibility is gated by active=None, after the source is gone.
            lock(&registry).active = None;
        };
        #[cfg(test)]
        let injected = std::mem::take(&mut lock(&self.registry).fail_spawn);
        #[cfg(not(test))]
        let injected = false;
        let started = if injected {
            Err(std::io::Error::other("injected spawn failure"))
        } else {
            thread::Builder::new()
                .name("alfred-native".into())
                .spawn(work)
        };
        if started.is_err() {
            let mut r = lock(&self.registry);
            r.active = None;
            r.slot = None;
            return Err(fail("thread_start_failed", "Cannot create native worker")
                .at(&state.request, "start"));
        }
        Ok(handle)
    }
    fn lookup(&self, handle: i64) -> Result<(Arc<State>, bool)> {
        let r = lock(&self.registry);
        match &r.slot {
            Some((h, s)) if *h == handle => Ok((s.clone(), r.active == Some(handle))),
            _ => Err(fail("stale_handle", "Handle is no longer available")),
        }
    }
    pub fn poll(&self, handle: i64) -> Result<Value> {
        let (s, active) = self.lookup(handle)?;
        Ok(
            json!({"version":1,"handle":handle,"attempt_id":s.request.attempt_id,"selection_id":s.request.selection_id,"generation":s.request.generation,"item_ids":s.request.item_ids,
            "progress":*lock(&s.latest),"terminal":if active { None } else { lock(&s.terminal).clone() },"leases_released":!active}),
        )
    }
    pub fn cancel(&self, handle: i64) -> Result<Value> {
        let (s, _) = self.lookup(handle)?;
        s.cancel.cancel();
        Ok(json!({"cancel_requested":true}))
    }
    pub fn close(&self, handle: i64) -> Value {
        let removed = {
            let mut r = lock(&self.registry);
            if r.slot.as_ref().is_some_and(|(h, _)| *h == handle) {
                r.slot.take()
            } else {
                None
            }
        };
        if let Some((_, s)) = removed {
            s.cancel.cancel();
        }
        json!({"closed":true})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    fn request() -> Request {
        serde_json::from_value(json!({"version":1,"kind":"probe","job_id":"00000000-0000-0000-0000-000000000001",
            "attempt_id":"00000000-0000-0000-0000-000000000002","selection_id":"00000000-0000-0000-0000-000000000003","generation":u64::MAX,
            "item_ids":["00000000-0000-0000-0000-000000000004"],"feature":null,"leases":[],"scope":{"kind":"full"},"deadline_ms":1000,
            "output_dir":"unused","available_memory_bytes":0,"low_memory":false,"reserved_output_bytes":1})).unwrap()
    }
    fn wait(b: &Bridge) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while b.describe()["active_handle"] != Value::Null {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn close_keeps_admission_until_source_drop_and_stale_tokens_never_return() {
        let b = Bridge::default();
        let (entered_tx, entered) = mpsc::channel();
        let (release_tx, release) = mpsc::channel();
        struct SourceDrop(mpsc::Sender<()>, mpsc::Receiver<()>);
        impl Drop for SourceDrop {
            fn drop(&mut self) {
                self.0.send(()).unwrap();
                self.1.recv().unwrap();
            }
        }
        let h = b
            .spawn(request(), move |s| {
                let _source = SourceDrop(entered_tx, release);
                while !s.cancel.is_cancelled() {
                    thread::yield_now();
                }
                Ok(json!({}))
            })
            .unwrap();
        assert_eq!(
            b.spawn(request(), |_| Ok(json!({}))).unwrap_err().code,
            "busy"
        );
        b.cancel(h).unwrap();
        b.cancel(h).unwrap();
        b.close(h);
        b.close(h);
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(b.poll(h).unwrap_err().code, "stale_handle");
        assert_eq!(b.cancel(h).unwrap_err().code, "stale_handle");
        assert_eq!(
            b.spawn(request(), |_| Ok(json!({}))).unwrap_err().code,
            "busy"
        );
        release_tx.send(()).unwrap();
        wait(&b);
        let next = b.spawn(request(), |_| Ok(json!({}))).unwrap();
        assert!(next > h);
        wait(&b);
        let p = b.poll(next).unwrap();
        assert_eq!(p, b.poll(next).unwrap());
        assert_eq!(p["generation"], u64::MAX);
        assert_eq!(
            b.spawn(request(), |_| Ok(json!({}))).unwrap_err().code,
            "busy"
        );
        b.close(next);
    }
    #[test]
    fn panic_spawn_failure_and_sequence_exhaustion_recover() {
        let b = Bridge::default();
        lock(&b.registry).fail_spawn = true;
        assert_eq!(
            b.spawn(request(), |_| Ok(json!({}))).unwrap_err().code,
            "thread_start_failed"
        );
        let h = b
            .spawn(request(), |_| panic!("injected worker panic"))
            .unwrap();
        wait(&b);
        assert_eq!(
            b.poll(h).unwrap()["terminal"]["error"]["code"],
            "worker_panicked"
        );
        b.close(h);
        let h = b.spawn(request(), |_| Ok(json!({}))).unwrap();
        wait(&b);
        b.close(h);
        lock(&b.registry).sequence = i64::MAX;
        assert_eq!(
            b.spawn(request(), |_| Ok(json!({}))).unwrap_err().code,
            "resource_limit"
        );
        assert_eq!(b.describe()["active_handle"], Value::Null);
    }
}
