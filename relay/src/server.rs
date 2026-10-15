//! Starting and stopping the relay: the public listener and the admin socket.

use std::net::SocketAddr;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};

use tokio::net::{TcpListener, UnixListener};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::apns::Apns;
use crate::config::Config;
use crate::state::AppState;
use crate::{admin, db, http};

pub struct Running {
    pub http_addr: SocketAddr,
    pub admin_socket: PathBuf,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

pub async fn start(cfg: Config) -> Result<Running, String> {
    std::fs::create_dir_all(&cfg.data_dir)
        .map_err(|e| format!("can't create {}: {e}", cfg.data_dir.display()))?;
    let db_path = cfg.data_dir.join("relay.db");
    let conn = db::open(&db_path).map_err(|e| format!("can't open {}: {e}", db_path.display()))?;
    let apns = match &cfg.apns {
        Some(a) => Some(Apns::new(a)?),
        None => {
            tracing::warn!("relay.toml has no [apns] section: pushes are off");
            None
        }
    };

    let tcp =
        TcpListener::bind(cfg.listen).await.map_err(|e| format!("can't listen on {}: {e}", cfg.listen))?;
    let http_addr = tcp.local_addr().map_err(|e| e.to_string())?;
    let admin_socket = cfg.admin_socket.clone();
    let unix = bind_admin_socket(&admin_socket, cfg.admin_group.as_deref())?;

    let st = AppState::new(cfg, conn, apns);
    let (stop, stopped) = watch::channel(false);
    let mut tasks = Vec::new();

    let app = http::router(st.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let mut until = stopped.clone();
    tasks.push(tokio::spawn(async move {
        let shutdown = async move {
            let _ = until.wait_for(|s| *s).await;
        };
        if let Err(e) = axum::serve(tcp, app).with_graceful_shutdown(shutdown).await {
            tracing::error!("public listener stopped: {e}");
        }
    }));

    let mut until = stopped;
    let admin_st = st.clone();
    tasks.push(tokio::spawn(async move {
        tokio::select! {
            _ = admin::serve(unix, admin_st) => {}
            _ = until.wait_for(|s| *s) => {}
        }
    }));

    tracing::info!(%http_addr, admin = %admin_socket.display(), "relay started");
    Ok(Running { http_addr, admin_socket, stop, tasks })
}

impl Running {
    pub async fn stop(self) {
        let _ = self.stop.send(true);
        for task in self.tasks {
            let _ = task.await;
        }
        if is_socket(&self.admin_socket) {
            let _ = std::fs::remove_file(&self.admin_socket);
        }
    }
}

fn bind_admin_socket(path: &Path, group: Option<&str>) -> Result<UnixListener, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    }
    if is_socket(path) {
        std::fs::remove_file(path).map_err(|e| format!("can't remove the old {}: {e}", path.display()))?;
    }
    let listener = UnixListener::bind(path)
        .map_err(|e| format!("can't open the admin socket {}: {e}", path.display()))?;
    let mode = match group {
        Some(name) => {
            let gid = group_id(name).ok_or_else(|| format!("no group named {name} on this host"))?;
            std::os::unix::fs::chown(path, None, Some(gid))
                .map_err(|e| format!("can't give {} to group {name}: {e}", path.display()))?;
            0o660
        }
        None => 0o600,
    };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| format!("can't set permissions on {}: {e}", path.display()))?;
    Ok(listener)
}

fn is_socket(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_socket())
}

/// Looks a group up in /etc/group (the relay runs on plain Linux hosts, no NSS needed).
fn group_id(name: &str) -> Option<u32> {
    let groups = std::fs::read_to_string("/etc/group").ok()?;
    groups.lines().find_map(|line| {
        let mut fields = line.split(':');
        let group = fields.next()?;
        let _password = fields.next()?;
        let gid = fields.next()?;
        (group == name).then(|| gid.parse().ok()).flatten()
    })
}
