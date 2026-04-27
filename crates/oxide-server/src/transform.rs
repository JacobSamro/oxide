use anyhow::Result;
use bytes::Bytes;
use serde_json::Value;

/// Rewrite all `dist.tarball` URLs in the metadata to point at our public URL.
/// Returns rewritten JSON bytes; falls back to original bytes if structure unexpected.
pub fn rewrite_tarball_urls(raw: &[u8], public_url: &str) -> Result<Bytes> {
    let mut v: Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return Ok(Bytes::copy_from_slice(raw)),
    };
    if let Some(versions) = v.get_mut("versions").and_then(|x| x.as_object_mut()) {
        for (_ver, vobj) in versions.iter_mut() {
            if let Some(dist) = vobj.get_mut("dist").and_then(|d| d.as_object_mut()) {
                if let Some(t) = dist.get("tarball").and_then(|t| t.as_str()) {
                    if let Some(rewritten) = rewrite_one(t, public_url) {
                        dist.insert("tarball".into(), Value::String(rewritten));
                    }
                }
            }
        }
    }
    Ok(Bytes::from(serde_json::to_vec(&v)?))
}

fn rewrite_one(orig: &str, public_url: &str) -> Option<String> {
    // Find "/<pkg>/-/" segment and graft our public URL on.
    let public = public_url.trim_end_matches('/');
    let idx = orig.find("/-/")?;
    // Walk back to the host root: take everything from the path that starts the package name.
    let after_scheme = orig.split("://").nth(1)?;
    let path_start = after_scheme.find('/')?;
    let path = &after_scheme[path_start..];
    let _ = idx; // sanity that "/-/" exists
    Some(format!("{public}{path}"))
}

/// Build npm abbreviated metadata (Accept: application/vnd.npm.install-v1+json) from full metadata.
pub fn abbreviate(raw: &[u8]) -> Result<Bytes> {
    let v: Value = serde_json::from_slice(raw)?;
    let name = v.get("name").cloned().unwrap_or(Value::Null);
    let dist_tags = v.get("dist-tags").cloned().unwrap_or_else(|| serde_json::json!({}));
    let modified = v.get("modified").cloned();

    let mut versions = serde_json::Map::new();
    if let Some(orig_versions) = v.get("versions").and_then(|x| x.as_object()) {
        for (ver, vobj) in orig_versions {
            let mut o = serde_json::Map::new();
            for k in [
                "name", "version", "deprecated", "dependencies", "optionalDependencies",
                "devDependencies", "bundleDependencies", "peerDependencies", "peerDependenciesMeta",
                "bin", "directories", "dist", "engines", "_hasShrinkwrap", "hasInstallScript",
                "cpu", "os", "funding",
            ] {
                if let Some(val) = vobj.get(k) { o.insert(k.into(), val.clone()); }
            }
            versions.insert(ver.clone(), Value::Object(o));
        }
    }

    let mut out = serde_json::Map::new();
    out.insert("name".into(), name);
    out.insert("dist-tags".into(), dist_tags);
    if let Some(m) = modified { out.insert("modified".into(), m); }
    out.insert("versions".into(), Value::Object(versions));
    Ok(Bytes::from(serde_json::to_vec(&Value::Object(out))?))
}

pub fn gzip(bytes: &[u8]) -> Result<Bytes> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut enc = GzEncoder::new(Vec::with_capacity(bytes.len() / 4), Compression::new(5));
    enc.write_all(bytes)?;
    Ok(Bytes::from(enc.finish()?))
}

pub fn brotli_compress(bytes: &[u8]) -> Result<Bytes> {
    let mut out = Vec::with_capacity(bytes.len() / 4);
    let mut writer = brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
    use std::io::Write;
    writer.write_all(bytes)?;
    drop(writer);
    Ok(Bytes::from(out))
}
