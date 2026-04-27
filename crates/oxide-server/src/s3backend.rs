// Minimal S3 client built on rusty-s3 (presigning) + reqwest (transfer).
// Used as an optional tarball storage backend; the FS backend remains the default.

use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use futures::Stream;
use reqwest::Client;
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use url::Url;

use crate::settings::S3Settings;

pub struct S3Backend {
    bucket: Bucket,
    creds: Credentials,
    prefix: String,
    http: Client,
}

impl S3Backend {
    pub fn from_settings(s: &S3Settings) -> Result<Self> {
        anyhow::ensure!(s.enabled, "s3 not enabled");
        anyhow::ensure!(!s.bucket.is_empty(), "bucket required");
        anyhow::ensure!(!s.access_key.is_empty() && !s.secret_key.is_empty(), "credentials required");

        let endpoint = if s.endpoint.is_empty() {
            // Default to AWS regional endpoint.
            Url::parse(&format!("https://s3.{}.amazonaws.com", s.region))?
        } else {
            Url::parse(&s.endpoint).context("invalid s3 endpoint")?
        };
        let style = if s.path_style { UrlStyle::Path } else { UrlStyle::VirtualHost };
        let bucket = Bucket::new(endpoint, style, s.bucket.clone(), s.region.clone())
            .context("constructing s3 bucket")?;
        let creds = Credentials::new(s.access_key.clone(), s.secret_key.clone());
        let http = Client::builder().timeout(Duration::from_secs(60)).build()?;
        let prefix = s.path_prefix.trim_matches('/').to_string();
        let prefix = if prefix.is_empty() { String::new() } else { format!("{prefix}/") };
        Ok(Self { bucket, creds, prefix, http })
    }

    fn key(&self, package: &str, file: &str) -> String {
        let safe = package.replace('/', "_2F_");
        format!("{}{}/{}", self.prefix, safe, file)
    }

    pub async fn put(&self, package: &str, file: &str, body: Bytes) -> Result<()> {
        let key = self.key(package, file);
        let action = self.bucket.put_object(Some(&self.creds), &key);
        let url = action.sign(Duration::from_secs(300));
        let res = self.http.put(url).body(body).send().await?;
        anyhow::ensure!(res.status().is_success(), "s3 put failed: {}", res.status());
        Ok(())
    }

    /// Returns a streaming GET response when the object exists, or None on 404.
    pub async fn get_stream(&self, package: &str, file: &str)
        -> Result<Option<(u64, impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static)>>
    {
        let key = self.key(package, file);
        let action = self.bucket.get_object(Some(&self.creds), &key);
        let url = action.sign(Duration::from_secs(300));
        let res = self.http.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND { return Ok(None); }
        anyhow::ensure!(res.status().is_success(), "s3 get failed: {}", res.status());
        let len = res.content_length().unwrap_or(0);
        Ok(Some((len, res.bytes_stream())))
    }
}
