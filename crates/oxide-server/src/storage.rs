use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Atomic write: write to <path>.tmp.<uuid>, fsync, rename.
pub async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.with_context(|| format!("mkdir {parent:?}"))?;
    }
    let tmp = tmp_path(path);
    {
        let mut f = fs::File::create(&tmp).await?;
        f.write_all(bytes).await?;
        f.flush().await?;
        f.sync_all().await?;
    }
    fs::rename(&tmp, path).await.context("rename")?;
    Ok(())
}

pub fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(format!(".tmp.{}", uuid::Uuid::new_v4()));
    PathBuf::from(s)
}

pub async fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path).await {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub async fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).await?;
    Ok(())
}
