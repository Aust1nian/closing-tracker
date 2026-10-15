//! The public API. Caddy terminates TLS in front of it.

use std::net::{IpAddr, SocketAddr};

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::apns::Env;
use crate::state::AppState;
use crate::{now_unix, store};

/// Spec: a message is capped at 2 MB.
const MAX_BODY: usize = 2 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v0/push-check", put(push_check))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

async fn health(State(st): State<AppState>) -> Response {
    let mut resp = Json(serde_json::json!({
        "ok": true,
        "service": "closing-relay",
        "version": env!("CARGO_PKG_VERSION"),
        "time": now_unix(),
    }))
    .into_response();
    let headers = resp.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(origin) = st.cfg.health_cors_origin.as_deref()
        && let Ok(value) = HeaderValue::from_str(origin)
    {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    resp
}

/// Phase 0 only: a test build registers its APNs token so `relayctl push-test`
/// can reach it. Unauthenticated, so it's small and rate-limited; Phase 1
/// replaces it with the authenticated `PUT /v1/push-token`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PushCheck {
    token: String,
    env: String,
    #[serde(default)]
    label: String,
}

async fn push_check(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<PushCheck>, JsonRejection>,
) -> Response {
    if !st.push_check_limit.allow(client_ip(&st, peer, &headers)) {
        return error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    let Ok(Json(req)) = body else {
        return error(StatusCode::BAD_REQUEST, "bad_request");
    };
    let token = req.token.to_ascii_lowercase();
    if !valid_device_token(&token) {
        return error(StatusCode::BAD_REQUEST, "bad_token");
    }
    let Some(env) = Env::parse(&req.env) else {
        return error(StatusCode::BAD_REQUEST, "bad_env");
    };
    let label: String = req.label.chars().filter(|c| !c.is_control()).take(40).collect();
    let label = label.trim().to_owned();
    let prefix = token[..8].to_owned();
    let now = now_unix();
    match st.db(move |db| store::upsert_push_check(db, &token, env, &label, now)).await {
        Ok(()) => {
            tracing::info!(token = %prefix, env = env.as_str(), "push-check token registered");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            tracing::error!("push-check store failed: {e}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "internal")
        }
    }
}

/// APNs device tokens are hex. Today they're 32 bytes; Apple says they may grow.
fn valid_device_token(token: &str) -> bool {
    (64..=200).contains(&token.len())
        && token.len().is_multiple_of(2)
        && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The client's address: the peer, or, behind the reverse proxy on this host,
/// the last `X-Forwarded-For` entry (the one the proxy itself wrote).
fn client_ip(st: &AppState, peer: SocketAddr, headers: &HeaderMap) -> IpAddr {
    if st.cfg.trust_loopback_proxy && peer.ip().is_loopback() {
        let forwarded = headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .next_back();
        if let Some(ip) = forwarded {
            return ip;
        }
    }
    peer.ip()
}

fn error(status: StatusCode, code: &'static str) -> Response {
    (status, Json(serde_json::json!({ "error": code }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::valid_device_token;

    #[test]
    fn device_tokens() {
        assert!(valid_device_token(&"ab".repeat(32)));
        assert!(valid_device_token(&"0f".repeat(50)));
        assert!(!valid_device_token(&"ab".repeat(31)), "too short");
        assert!(!valid_device_token(&"ab".repeat(101)), "too long");
        assert!(!valid_device_token(&format!("{}a", "ab".repeat(32))), "odd length");
        assert!(!valid_device_token(&"zz".repeat(32)), "not hex");
    }
}
