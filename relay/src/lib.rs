//! Closing Tracker relay: carries sealed messages between a shop's phones, watches
//! closing deadlines and sends pushes. It can read routing data and closing status,
//! never content.
//!
//! Phase 0 is the pipeline stub: `/v1/health`, a temporary push-check registration
//! (`/v0/push-check`) and an operator test push through `relayctl`.

#![forbid(unsafe_code)]

pub mod admin;
pub mod apns;
pub mod config;
pub mod db;
pub mod http;
pub mod ratelimit;
pub mod server;
pub mod state;
pub mod store;

/// Seconds since the Unix epoch.
pub fn now_unix() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
