//! Attempt-local atomic publication. Shared storage reserves quota/free disk and
//! owns retention and deletion, including closed/obsolete attempt directories.
use crate::transport::{DOCUMENT_LIMIT, Descriptor, Result, fail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub fn io(_: std::io::Error) -> crate::transport::HostError {
    fail("io_error", "Private file operation failed")
}
pub fn child(root: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || name.contains(['/', '\\', ':'])
    {
        return Err(fail(
            "invalid_request",
            "Expected a private relative filename",
        ));
    }
    let root = fs::canonicalize(root).map_err(io)?;
    let path = root.join(name);
    if path.exists() && fs::canonicalize(&path).map_err(io)?.parent() != Some(root.as_path()) {
        return Err(fail(
            "invalid_request",
            "Payload escapes its private directory",
        ));
    }
    Ok(path)
}
pub fn hash_file(path: &Path, limit: u64) -> Result<(u64, String)> {
    let file = File::open(path).map_err(io)?;
    let mut reader = file.take(limit + 1);
    let mut buffer = [0u8; 256 * 1024];
    let mut hash = Sha256::new();
    let mut count = 0;
    loop {
        let n = reader.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        hash.update(&buffer[..n]);
    }
    if count > limit {
        return Err(fail("size_limit", "Payload exceeds its document limit"));
    }
    Ok((count, format!("{:x}", hash.finalize())))
}
pub fn load(root: &Path, d: &Descriptor) -> Result<Vec<u8>> {
    let path = child(root, &d.path)?;
    if d.bytes > DOCUMENT_LIMIT {
        return Err(fail("size_limit", "Saved document exceeds 64 MiB"));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io)?
        .take(DOCUMENT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() as u64 != d.bytes || format!("{:x}", Sha256::digest(&bytes)) != d.sha256 {
        return Err(fail("input_changed", "Saved payload size/hash mismatch"));
    }
    Ok(bytes)
}
struct LimitedWriter {
    file: File,
    left: u64,
}
impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.left {
            return Err(std::io::Error::other("document limit"));
        }
        let n = self.file.write(bytes)?;
        self.left -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
pub struct Output {
    pub dir: PathBuf,
    budget: u64,
    used: u64,
    committed: bool,
}
impl Output {
    pub fn new(root: &Path, attempt: &str, budget: u64) -> Result<Self> {
        let dir = child(root, attempt)?;
        fs::create_dir(&dir)
            .map_err(|_| fail("storage_full", "Cannot allocate new attempt directory"))?;
        Ok(Self {
            dir,
            budget,
            used: 0,
            committed: false,
        })
    }
    pub fn json(
        &mut self,
        name: &str,
        kind: &str,
        version: &str,
        value: &impl Serialize,
    ) -> Result<Descriptor> {
        let temp = child(&self.dir, &format!("{name}.partial"))?;
        let mut w = LimitedWriter {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(io)?,
            left: DOCUMENT_LIMIT.min(self.budget.saturating_sub(self.used)),
        };
        serde_json::to_writer(&mut w, value).map_err(|_| {
            fail(
                "storage_full",
                "Cannot serialize within reserved document space",
            )
        })?;
        w.file.sync_all().map_err(io)?;
        drop(w);
        let target = child(&self.dir, name)?;
        fs::rename(temp, &target).map_err(io)?;
        self.descriptor(name, kind, version)
    }
    pub fn descriptor(&mut self, name: &str, kind: &str, version: &str) -> Result<Descriptor> {
        let (bytes, sha256) = hash_file(&child(&self.dir, name)?, DOCUMENT_LIMIT)?;
        let total = self
            .used
            .checked_add(bytes)
            .ok_or_else(|| fail("storage_full", "Output quota exceeded"))?;
        if total > self.budget {
            return Err(fail("storage_full", "Output quota exceeded"));
        }
        self.used = total;
        Ok(Descriptor {
            kind: kind.into(),
            version: version.into(),
            path: name.into(),
            bytes,
            sha256,
        })
    }
    pub fn png_space(&self) -> bool {
        self.budget.saturating_sub(self.used) >= 32 * 1024 * 1024
    }
    pub fn commit(&mut self, manifest: &impl Serialize) -> Result<Descriptor> {
        let d = self.json("manifest.json", "alfred-result", "1", manifest)?;
        // Android/Linux: persist directory entries as well as file contents.
        #[cfg(unix)]
        File::open(&self.dir).map_err(io)?.sync_all().map_err(io)?;
        self.committed = true;
        Ok(d)
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        // Only this newly-created attempt is ours; never delete a snapshot/root.
        if !self.committed {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}
