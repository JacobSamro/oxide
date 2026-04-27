// Token + password auth for publish.
//
// `npm publish` carries an `Authorization: Bearer <token>` header. We look the token up
// in the Token table and resolve to a User. `npm login` exchanges name+password for a
// fresh token; we validate the password against User.passwordHash (bcrypt, same hashes
// the Bun-side admin UI writes).

use anyhow::{anyhow, Result};
use axum::http::HeaderMap;
use bcrypt::verify;
use rand::RngCore;
use rusqlite::params;

use crate::db::Db;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub name: String,
}

pub fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let v = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let s = v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer "))?;
    if s.is_empty() { None } else { Some(s.to_string()) }
}

pub fn user_from_token(db: &Db, token: &str) -> Result<AuthUser> {
    let user = db.with(|c| {
        let row = c.query_row(
            "SELECT u.id, u.email, u.name
               FROM Token t JOIN User u ON u.id = t.userId
              WHERE t.id = ?1",
            params![token],
            |r| Ok(AuthUser { id: r.get(0)?, email: r.get(1)?, name: r.get(2)? }),
        );
        Ok(row)
    })??;
    let _ = db.touch_token(token);
    Ok(user)
}

pub fn verify_password(db: &Db, name_or_email: &str, password: &str) -> Result<AuthUser> {
    let row: (i64, String, String, String) = db.with(|c| {
        let r = c.query_row(
            "SELECT id, email, name, passwordHash FROM User WHERE email = ?1 OR name = ?1 LIMIT 1",
            params![name_or_email],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        Ok(r)
    })?;
    if !verify(password, &row.3).unwrap_or(false) {
        return Err(anyhow!("invalid credentials"));
    }
    Ok(AuthUser { id: row.0, email: row.1, name: row.2 })
}

pub fn issue_token(db: &Db, user_id: i64, label: Option<&str>) -> Result<String> {
    // 32 random bytes hex-encoded → 64 chars. Opaque to callers; we never hash on retrieval.
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let token = hex::encode(buf);
    db.with(|c| {
        c.execute(
            "INSERT INTO Token (id, userId, name) VALUES (?1, ?2, ?3)",
            params![token, user_id, label],
        )?;
        Ok(())
    })?;
    Ok(token)
}

pub fn revoke_token(db: &Db, token: &str) -> Result<()> {
    db.with(|c| {
        c.execute("DELETE FROM Token WHERE id = ?1", params![token])?;
        Ok(())
    })
}
