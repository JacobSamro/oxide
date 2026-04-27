// Writable SQLite connection shared across handlers. settings.rs already opens a
// read-only handle for runtime config; this one supports tokens, local packages, and publish.
//
// We use parking_lot::Mutex (no poisoning, slightly faster) and call into it from short
// synchronous critical sections. SQLite ops at this throughput stay sub-ms; for the heavy
// publish path we still wrap in spawn_blocking so a slow disk doesn't stall the runtime.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OpenFlags};

#[derive(Clone)]
pub struct Db {
    inner: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).ok(); }
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ).with_context(|| format!("opening db {path:?}"))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        Ok(Self { inner: Arc::new(Mutex::new(conn)) })
    }

    /// Run `f` against the locked connection. Keep these short.
    pub fn with<F, T>(&self, f: F) -> Result<T>
    where F: FnOnce(&Connection) -> Result<T>
    {
        let g = self.inner.lock();
        f(&g)
    }

    pub fn touch_token(&self, token: &str) -> Result<()> {
        self.with(|c| {
            c.execute("UPDATE Token SET lastUsedAt = datetime('now') WHERE id = ?1", params![token])?;
            Ok(())
        })
    }
}
