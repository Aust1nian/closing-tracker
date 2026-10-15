//! What every request handler shares.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::apns::Apns;
use crate::config::Config;
use crate::ratelimit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    db: Arc<Mutex<Connection>>,
    pub apns: Option<Arc<Apns>>,
    /// `/v0/push-check` registrations per client address.
    pub push_check_limit: Arc<RateLimiter>,
    pub started: Instant,
}

impl AppState {
    pub fn new(cfg: Config, db: Connection, apns: Option<Apns>) -> AppState {
        AppState {
            cfg: Arc::new(cfg),
            db: Arc::new(Mutex::new(db)),
            apns: apns.map(Arc::new),
            push_check_limit: Arc::new(RateLimiter::new(10, Duration::from_secs(3600))),
            started: Instant::now(),
        }
    }

    /// Runs `f` on the database from a blocking thread.
    pub async fn db<T, F>(&self, f: F) -> rusqlite::Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = db.lock().unwrap_or_else(|p| p.into_inner());
            f(&mut conn)
        })
        .await
        .expect("database task panicked")
    }
}
