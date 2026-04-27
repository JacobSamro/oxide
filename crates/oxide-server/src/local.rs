// Local (published) package store. Tarballs live on disk under <local_path>/<safe_name>/<v>.tgz.
// Metadata is rebuilt on demand from LocalPackageVersion + LocalPackageDistTag rows so an
// admin tweaking dist-tags or unpublishing a version is reflected immediately.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use rusqlite::params;
use serde_json::{json, Value};

use crate::db::Db;

pub struct LocalStore {
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LocalPackage {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub workspace_id: Option<i64>,
}

impl LocalStore {
    pub fn new(root: impl Into<PathBuf>) -> Self { Self { root: root.into() } }

    pub fn tarball_path(&self, name: &str, version: &str) -> PathBuf {
        let safe = name.replace('/', "_2F_");
        self.root.join(safe).join(format!("{version}.tgz"))
    }

    pub fn lookup(db: &Db, name: &str) -> Result<Option<LocalPackage>> {
        db.with(|c| {
            let row = c.query_row(
                "SELECT id, name, ownerId, workspaceId FROM LocalPackage WHERE name = ?1",
                params![name],
                |r| Ok(LocalPackage {
                    id: r.get(0)?, name: r.get(1)?, owner_id: r.get(2)?, workspace_id: r.get(3).ok(),
                }),
            ).optional_anyhow()?;
            Ok(row)
        })
    }

    /// Build a registry-style metadata document from the local tables.
    /// `tarball_base_url` is used to construct dist.tarball URLs.
    pub fn build_metadata(db: &Db, name: &str, tarball_base_url: &str) -> Result<Value> {
        let pkg = Self::lookup(db, name)?.ok_or_else(|| anyhow!("not a local package"))?;

        let versions: Vec<(String, String, String, i64)> = db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT version, metadata, tarballSha, tarballSize
                   FROM LocalPackageVersion WHERE packageId = ?1 ORDER BY id ASC",
            )?;
            let rows = stmt.query_map(params![pkg.id], |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            )))?
            .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })?;

        let mut versions_obj = serde_json::Map::new();
        for (ver, meta_str, sha, size) in &versions {
            let mut v: Value = serde_json::from_str(meta_str)
                .with_context(|| format!("parse stored metadata for {name}@{ver}"))?;
            // Fill in dist; we are the source of truth, not whatever the publisher sent.
            let tarball_url = format!("{}/{}/-/{}-{}.tgz",
                tarball_base_url.trim_end_matches('/'),
                name,
                last_path_segment(name),
                ver,
            );
            let dist = json!({ "tarball": tarball_url, "integrity": sha, "size": size });
            if let Some(o) = v.as_object_mut() { o.insert("dist".into(), dist); }
            versions_obj.insert(ver.clone(), v);
        }

        let dist_tags: Vec<(String, String)> = db.with(|c| {
            let mut stmt = c.prepare("SELECT tag, version FROM LocalPackageDistTag WHERE packageId = ?1")?;
            let rows = stmt.query_map(params![pkg.id], |r| Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })?;
        let mut tags_obj = serde_json::Map::new();
        for (t, v) in &dist_tags { tags_obj.insert(t.clone(), Value::String(v.clone())); }
        // Always include `latest` if it isn't set explicitly — pick the highest published.
        if !tags_obj.contains_key("latest") {
            if let Some(latest) = pick_latest(&versions.iter().map(|v| v.0.as_str()).collect::<Vec<_>>()) {
                tags_obj.insert("latest".into(), Value::String(latest.to_string()));
            }
        }

        let mut doc = serde_json::Map::new();
        doc.insert("_id".into(), Value::String(name.to_string()));
        doc.insert("name".into(), Value::String(name.to_string()));
        doc.insert("dist-tags".into(), Value::Object(tags_obj));
        doc.insert("versions".into(), Value::Object(versions_obj));
        Ok(Value::Object(doc))
    }

    pub async fn write_tarball(&self, name: &str, version: &str, data: Bytes) -> Result<()> {
        let path = self.tarball_path(name, version);
        if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await?; }
        let tmp = crate::storage::tmp_path(&path);
        tokio::fs::write(&tmp, &data).await?;
        tokio::fs::rename(&tmp, &path).await?;
        Ok(())
    }
}

fn last_path_segment(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn pick_latest<'a>(versions: &[&'a str]) -> Option<&'a str> {
    let mut parsed: Vec<(semver::Version, &str)> = versions.iter()
        .filter_map(|v| semver::Version::parse(v).ok().map(|p| (p, *v)))
        .collect();
    parsed.sort_by(|a, b| a.0.cmp(&b.0));
    parsed.last().map(|(_, v)| *v)
}

// rusqlite's `Result` doesn't have `optional()` returning anyhow::Result, so a tiny shim:
trait OptExt<T> {
    fn optional_anyhow(self) -> Result<Option<T>>;
}
impl<T> OptExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional_anyhow(self) -> Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
