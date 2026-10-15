//! `relay.toml`: where the relay listens, keeps its data and finds its push key.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Public API address. Behind Caddy on the same host this is a loopback address.
    pub listen: SocketAddr,
    /// The database lives here (and, from Phase 1, sealed message files).
    pub data_dir: PathBuf,
    /// Unix socket that `relayctl` talks to.
    #[serde(default = "default_admin_socket")]
    pub admin_socket: PathBuf,
    /// Group given read and write on the admin socket (the operators). Unset: owner only.
    #[serde(default)]
    pub admin_group: Option<String>,
    /// Browser origin allowed to read `/v1/health` (the website's /status page).
    #[serde(default)]
    pub health_cors_origin: Option<String>,
    /// Take the client address from the last `X-Forwarded-For` entry when the
    /// connection comes from loopback, i.e. from the reverse proxy on this host.
    #[serde(default = "default_true")]
    pub trust_loopback_proxy: bool,
    /// Apple Push Notification service. Unset: pushes are refused, everything else runs.
    #[serde(default)]
    pub apns: Option<ApnsConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApnsConfig {
    /// The `.p8` auth key from developer.apple.com, readable only by the relay's user.
    pub key_path: PathBuf,
    pub key_id: String,
    pub team_id: String,
    /// The app's bundle ID.
    pub topic: String,
    /// Replaces both APNs hosts. Tests only.
    #[serde(default)]
    pub endpoint_override: Option<String>,
}

fn default_admin_socket() -> PathBuf {
    PathBuf::from("/run/closing-relay/admin.sock")
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("can't read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}
