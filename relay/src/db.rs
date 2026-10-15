//! The relay's SQLite database: WAL mode, schema versioned by `user_version`.

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

/// Each entry moves the schema up one version. Never edit a shipped entry; add one.
const MIGRATIONS: &[&str] = &[
    // 1 · Phase 0 pipeline check: APNs tokens registered by test builds.
    // Dropped when the device registry arrives in Phase 1.
    "CREATE TABLE push_check_tokens (
        token         TEXT PRIMARY KEY,
        env           TEXT NOT NULL CHECK (env IN ('production', 'sandbox')),
        label         TEXT NOT NULL DEFAULT '',
        registered_at INTEGER NOT NULL,
        last_seen     INTEGER NOT NULL
    ) STRICT;",
];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    let mode: String = conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(rusqlite::Error::InvalidParameterName(format!("journal_mode is {mode}, not WAL")));
    }
    // FULL syncs the WAL on every commit, so an acknowledged write survives power loss.
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}
