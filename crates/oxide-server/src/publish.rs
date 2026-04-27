// PUT /:package — accepts an npm "publish" document and stores the tarball + metadata.
//
// The publish document looks roughly like:
//   { "_id": "...", "name": "...",
//     "dist-tags": { "latest": "1.2.3" },
//     "versions": { "1.2.3": { ...package.json... , "dist": { ... } } },
//     "_attachments": { "<name>-1.2.3.tgz": { "data": "<base64 tarball>", "length": N } } }
//
// We support the common case: a single new version per request (the npm CLI batches one).

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use bytes::Bytes;
use rusqlite::params;
use serde_json::Value;
use sha2::{Digest, Sha512};

use crate::auth::AuthUser;
use crate::db::Db;
use crate::local::LocalStore;

pub struct PublishOutcome {
    pub package_name: String,
    pub version: String,
}

pub async fn handle_publish(
    db: Db,
    store: &LocalStore,
    user: AuthUser,
    package_name: String,
    body: Bytes,
) -> Result<PublishOutcome> {
    // Parsing is cheap; tarball SHA + write are not. Run on a blocking task.
    let parsed: Parsed = tokio::task::spawn_blocking(move || parse(&package_name, &body))
        .await
        .context("publish task panicked")??;

    // Auth/ownership
    let pkg_id = ensure_package(&db, &parsed.name, user.id)?;

    // Tarball: validate length + integrity, then persist.
    let computed_integrity = npm_integrity_sha512(&parsed.tarball);
    if let Some(declared) = &parsed.declared_integrity {
        if declared != &computed_integrity {
            bail!("tarball integrity mismatch: declared={declared}, computed={computed_integrity}");
        }
    }
    let tarball_size = parsed.tarball.len() as i64;
    store.write_tarball(&parsed.name, &parsed.version, parsed.tarball.clone()).await?;

    // Metadata + dist-tags rows.
    insert_version(&db, pkg_id, &parsed, &computed_integrity, tarball_size, user.id)?;
    if let Some(tags) = &parsed.dist_tags {
        for (tag, version) in tags {
            upsert_dist_tag(&db, pkg_id, tag, version)?;
        }
    } else {
        // Default: bump `latest` to the published version.
        upsert_dist_tag(&db, pkg_id, "latest", &parsed.version)?;
    }

    Ok(PublishOutcome { package_name: parsed.name, version: parsed.version })
}

struct Parsed {
    name: String,
    version: String,
    version_meta: Value,         // the package.json subset for this version (cleaned)
    tarball: Bytes,
    declared_integrity: Option<String>,
    dist_tags: Option<Vec<(String, String)>>,
}

fn parse(url_name: &str, body: &[u8]) -> Result<Parsed> {
    let mut doc: Value = serde_json::from_slice(body).context("publish body is not JSON")?;
    let name = doc.get("name").and_then(|v| v.as_str()).unwrap_or(url_name).to_string();

    // Pull the single version. npm sends exactly one in `versions` per publish.
    let versions = doc.get_mut("versions").and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow!("missing 'versions'"))?;
    if versions.is_empty() { bail!("publish body has no versions"); }
    let (version, version_meta) = versions.iter().next()
        .map(|(k, v)| (k.clone(), v.clone())).unwrap();

    // Validate version is parseable so we never end up with garbage in the index.
    semver::Version::parse(&version).with_context(|| format!("invalid semver: {version}"))?;

    let declared_integrity = version_meta
        .get("dist").and_then(|d| d.get("integrity")).and_then(|i| i.as_str())
        .map(|s| s.to_string());

    // _attachments is an object keyed by filename. There's only one.
    let attachments = doc.get("_attachments").and_then(|a| a.as_object())
        .ok_or_else(|| anyhow!("missing '_attachments'"))?;
    let (_filename, att) = attachments.iter().next()
        .ok_or_else(|| anyhow!("no attachment"))?;
    let b64 = att.get("data").and_then(|d| d.as_str())
        .ok_or_else(|| anyhow!("attachment has no 'data'"))?;
    let tarball = base64::engine::general_purpose::STANDARD
        .decode(b64).context("base64-decode tarball")?;
    if let Some(expected_len) = att.get("length").and_then(|n| n.as_u64()) {
        if expected_len as usize != tarball.len() {
            bail!("attachment length mismatch (declared {expected_len}, got {})", tarball.len());
        }
    }

    let dist_tags = doc.get("dist-tags").and_then(|t| t.as_object()).map(|tags| {
        tags.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<Vec<_>>()
    });

    // Strip _attachments-style metadata from the version blob before we store it.
    let cleaned_meta = sanitize_version_meta(version_meta);

    Ok(Parsed { name, version, version_meta: cleaned_meta, tarball: Bytes::from(tarball),
                declared_integrity, dist_tags })
}

fn sanitize_version_meta(mut v: Value) -> Value {
    if let Some(o) = v.as_object_mut() {
        // npm publish bodies sometimes include a `_npmUser`, `_npmVersion`, etc. — keep them out.
        o.remove("_attachments");
        o.remove("_id");
        // Wipe a stale `dist` block — we'll rebuild on read with our own tarball URL + size.
        o.remove("dist");
    }
    v
}

fn npm_integrity_sha512(bytes: &[u8]) -> String {
    let mut h = Sha512::new();
    h.update(bytes);
    let digest = h.finalize();
    format!("sha512-{}", base64::engine::general_purpose::STANDARD.encode(digest))
}

fn ensure_package(db: &Db, name: &str, user_id: i64) -> Result<i64> {
    db.with(|c| {
        if let Ok(id) = c.query_row(
            "SELECT id FROM LocalPackage WHERE name = ?1",
            params![name], |r| r.get::<_, i64>(0),
        ) {
            // Existing package: any future per-package ACL goes here.
            // For now we allow re-publishes only by the original owner.
            let owner: i64 = c.query_row(
                "SELECT ownerId FROM LocalPackage WHERE id = ?1",
                params![id], |r| r.get(0))?;
            if owner != user_id {
                anyhow::bail!("package {name} is owned by another user");
            }
            return Ok(id);
        }
        c.execute(
            "INSERT INTO LocalPackage (name, ownerId) VALUES (?1, ?2)",
            params![name, user_id],
        )?;
        Ok(c.last_insert_rowid())
    })
}

fn insert_version(
    db: &Db, package_id: i64, p: &Parsed,
    integrity: &str, size: i64, user_id: i64,
) -> Result<()> {
    db.with(|c| {
        let meta_str = serde_json::to_string(&p.version_meta)?;
        let res = c.execute(
            "INSERT INTO LocalPackageVersion
                (packageId, version, metadata, tarballSha, tarballSize, publishedBy)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![package_id, p.version, meta_str, integrity, size, user_id],
        );
        match res {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
                anyhow::bail!("version {} already published", p.version)
            }
            Err(e) => Err(e.into()),
        }
    })
}

fn upsert_dist_tag(db: &Db, package_id: i64, tag: &str, version: &str) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO LocalPackageDistTag (packageId, tag, version) VALUES (?1, ?2, ?3)
             ON CONFLICT(packageId, tag) DO UPDATE SET version = excluded.version",
            params![package_id, tag, version],
        )?;
        Ok(())
    })
}
