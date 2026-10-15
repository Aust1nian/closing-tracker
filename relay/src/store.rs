//! Queries. Every function takes a plain connection and runs on a blocking thread.

use rusqlite::{Connection, params};

use crate::apns::Env;

/// The stub sits on a public address, so it keeps only the most recent tokens.
const PUSH_CHECK_KEEP: i64 = 50;

#[derive(Debug, Clone)]
pub struct PushCheckToken {
    pub token: String,
    pub env: Env,
    pub label: String,
    pub registered_at: i64,
    pub last_seen: i64,
}

pub fn upsert_push_check(
    db: &mut Connection,
    token: &str,
    env: Env,
    label: &str,
    now: i64,
) -> rusqlite::Result<()> {
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO push_check_tokens (token, env, label, registered_at, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT (token) DO UPDATE
         SET env = excluded.env, label = excluded.label, last_seen = excluded.last_seen",
        params![token, env.as_str(), label, now],
    )?;
    tx.execute(
        "DELETE FROM push_check_tokens WHERE token NOT IN
         (SELECT token FROM push_check_tokens ORDER BY last_seen DESC LIMIT ?1)",
        params![PUSH_CHECK_KEEP],
    )?;
    tx.commit()
}

/// Newest first.
pub fn list_push_check(db: &mut Connection) -> rusqlite::Result<Vec<PushCheckToken>> {
    let mut stmt = db.prepare(
        "SELECT token, env, label, registered_at, last_seen
         FROM push_check_tokens ORDER BY last_seen DESC, token",
    )?;
    let rows = stmt.query_map([], |row| {
        let env: String = row.get(1)?;
        Ok(PushCheckToken {
            token: row.get(0)?,
            env: Env::parse(&env).unwrap_or(Env::Production),
            label: row.get(2)?,
            registered_at: row.get(3)?,
            last_seen: row.get(4)?,
        })
    })?;
    rows.collect()
}
