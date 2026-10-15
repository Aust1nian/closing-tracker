//! The operator socket. `relayctl` writes one JSON request per line and reads
//! one JSON answer per line. Only the relay host's operators can open it.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::state::AppState;
use crate::store;

/// The spec's test alert.
pub const TEST_ALERT: &str = "Test alert: you’re set up.";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Status,
    PushTokens,
    /// Sends a test alert to the push-check token starting with `to`, or the newest one.
    PushTest {
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        text: Option<String>,
    },
}

pub async fn serve(listener: UnixListener, st: AppState) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(handle(stream, st.clone()));
            }
            Err(e) => {
                tracing::warn!("admin socket accept failed: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

async fn handle(stream: UnixStream, st: AppState) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read.take(64 * 1024)).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let answer = match serde_json::from_str::<Request>(&line) {
            Ok(req) => dispatch(req, &st).await,
            Err(e) => fail(format!("bad request: {e}")),
        };
        let mut out = answer.to_string();
        out.push('\n');
        if write.write_all(out.as_bytes()).await.is_err() {
            return;
        }
    }
}

async fn dispatch(req: Request, st: &AppState) -> Value {
    match req {
        Request::Status => {
            let tokens = st.db(store::list_push_check).await;
            json!({
                "ok": true,
                "version": env!("CARGO_PKG_VERSION"),
                "uptime_s": st.started.elapsed().as_secs(),
                "listen": st.cfg.listen.to_string(),
                "apns": st.apns.is_some(),
                "push_check_tokens": tokens.map(|t| t.len()).unwrap_or(0),
            })
        }
        Request::PushTokens => match st.db(store::list_push_check).await {
            Ok(tokens) => json!({
                "ok": true,
                "tokens": tokens.iter().map(|t| json!({
                    "token_prefix": &t.token[..8],
                    "env": t.env.as_str(),
                    "label": t.label,
                    "registered_at": t.registered_at,
                    "last_seen": t.last_seen,
                })).collect::<Vec<_>>(),
            }),
            Err(e) => fail(format!("database: {e}")),
        },
        Request::PushTest { to, text } => push_test(st, to, text).await,
    }
}

async fn push_test(st: &AppState, to: Option<String>, text: Option<String>) -> Value {
    let Some(apns) = st.apns.clone() else {
        return fail("APNs isn't configured: add the [apns] section to relay.toml".into());
    };
    let tokens = match st.db(store::list_push_check).await {
        Ok(t) => t,
        Err(e) => return fail(format!("database: {e}")),
    };
    let target = match to.map(|p| p.to_ascii_lowercase()) {
        None => match tokens.first() {
            Some(t) => t.clone(),
            None => return fail("no phone has registered yet: open the test app first".into()),
        },
        Some(prefix) => {
            let matches: Vec<_> = tokens.iter().filter(|t| t.token.starts_with(&prefix)).collect();
            match matches.as_slice() {
                [one] => (*one).clone(),
                [] => return fail(format!("no registered token starts with {prefix}")),
                many => {
                    return fail(format!("{} tokens start with {prefix}; give more characters", many.len()));
                }
            }
        }
    };
    let body: String = text.as_deref().unwrap_or(TEST_ALERT).chars().take(178).collect();
    let prefix = &target.token[..8];
    match apns.send_alert(&target.token, target.env, &body).await {
        Ok(outcome) => {
            tracing::info!(token = %prefix, status = outcome.status, reason = ?outcome.reason, "test push");
            json!({
                "ok": outcome.accepted(),
                "token_prefix": prefix,
                "env": target.env.as_str(),
                "status": outcome.status,
                "apns_id": outcome.apns_id,
                "reason": outcome.reason,
            })
        }
        Err(e) => fail(e),
    }
}

fn fail(error: String) -> Value {
    json!({ "ok": false, "error": error })
}
