//! Phase 0 round trip against a mock APNs: health, registration, rate limit,
//! the operator's test push with a verifiable provider token, and a restart.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use closing_relay::config::{ApnsConfig, Config};
use closing_relay::server;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const TOPIC: &str = "tightline.systems.closingtracker";

#[derive(Clone)]
struct Mock {
    seen: Arc<Mutex<Vec<(String, HeaderMap, Value)>>>,
    status: StatusCode,
    reason: Option<&'static str>,
}

async fn apns_endpoint(
    State(mock): State<Mock>,
    UrlPath(token): UrlPath<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, HeaderMap, String) {
    mock.seen.lock().unwrap().push((token, headers, body));
    let mut out = HeaderMap::new();
    out.insert("apns-id", "11111111-2222-3333-4444-555555555555".parse().unwrap());
    let body = mock.reason.map(|r| json!({ "reason": r }).to_string()).unwrap_or_default();
    (mock.status, out, body)
}

async fn start_mock(status: StatusCode, reason: Option<&'static str>) -> (String, Mock) {
    let mock = Mock { seen: Arc::default(), status, reason };
    let app = Router::new().route("/3/device/{token}", post(apns_endpoint)).with_state(mock.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, mock)
}

struct Keys {
    private_path: PathBuf,
    public_pem: String,
}

fn make_key(dir: &Path) -> Keys {
    let secret = p256::SecretKey::random(&mut rand_core::OsRng);
    let private_path = dir.join("AuthKey_TEST.p8");
    std::fs::write(&private_path, secret.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes()).unwrap();
    let public_pem = secret.public_key().to_public_key_pem(LineEnding::LF).unwrap();
    Keys { private_path, public_pem }
}

fn config(dir: &Path, key: &Path, apns_url: &str) -> Config {
    Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.join("data"),
        admin_socket: dir.join("admin.sock"),
        admin_group: None,
        health_cors_origin: Some("https://tightline.systems".into()),
        trust_loopback_proxy: true,
        apns: Some(ApnsConfig {
            key_path: key.to_path_buf(),
            key_id: "KEYID12345".into(),
            team_id: "TEAMID1234".into(),
            topic: TOPIC.into(),
            endpoint_override: Some(apns_url.into()),
        }),
    }
}

async fn admin(socket: &Path, req: Value) -> Value {
    let mut stream = UnixStream::connect(socket).await.unwrap();
    stream.write_all(format!("{req}\n").as_bytes()).await.unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).await.unwrap();
    serde_json::from_str(&line).unwrap()
}

fn token(byte: &str) -> String {
    byte.repeat(32)
}

async fn register(http: &reqwest::Client, base: &str, body: Value, from: &str) -> StatusCode {
    let resp = http
        .put(format!("{base}/v0/push-check"))
        .header("x-forwarded-for", from)
        .json(&body)
        .send()
        .await
        .unwrap();
    StatusCode::from_u16(resp.status().as_u16()).unwrap()
}

#[tokio::test]
async fn phase0_round_trip() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = tempfile::tempdir().unwrap();
    let keys = make_key(dir.path());
    let (apns_url, mock) = start_mock(StatusCode::OK, None).await;
    let running = server::start(config(dir.path(), &keys.private_path, &apns_url)).await.unwrap();
    let base = format!("http://{}", running.http_addr);
    let http = reqwest::Client::new();

    // Health, readable by the website's /status page.
    let resp = http.get(format!("{base}/v1/health?t=1")).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(resp.headers()["access-control-allow-origin"], "https://tightline.systems");
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let health: Value = resp.json().await.unwrap();
    assert_eq!(health["ok"], true);

    // Registration validates its input.
    let phone = token("ab");
    let ok = json!({ "token": phone.to_uppercase(), "env": "production", "label": "Austin's iPhone\u{7}" });
    assert_eq!(register(&http, &base, ok, "198.51.100.10").await, StatusCode::NO_CONTENT);
    for bad in [
        json!({ "token": "abc", "env": "production" }),
        json!({ "token": token("zz"), "env": "production" }),
        json!({ "token": token("ab"), "env": "staging" }),
        json!({ "token": token("ab"), "env": "production", "name": "extra field" }),
    ] {
        assert_eq!(register(&http, &base, bad, "198.51.100.11").await, StatusCode::BAD_REQUEST);
    }

    let listed = admin(&running.admin_socket, json!({ "cmd": "push_tokens" })).await;
    let tokens = listed["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0]["token_prefix"], "abababab");
    assert_eq!(tokens[0]["label"], "Austin's iPhone", "control characters are dropped");

    // The operator's test push reaches APNs with a valid provider token.
    let sent = admin(&running.admin_socket, json!({ "cmd": "push_test" })).await;
    assert_eq!(sent["ok"], true, "{sent}");
    assert_eq!(sent["status"], 200);
    {
        let seen = mock.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let (path_token, headers, body) = &seen[0];
        assert_eq!(path_token, &phone);
        assert_eq!(headers["apns-topic"], TOPIC);
        assert_eq!(headers["apns-push-type"], "alert");
        assert_eq!(headers["apns-priority"], "10");
        assert_eq!(body["aps"]["alert"]["body"], "Test alert: you’re set up.");

        let bearer = headers["authorization"].to_str().unwrap().strip_prefix("bearer ").unwrap();
        let header = jsonwebtoken::decode_header(bearer).unwrap();
        assert_eq!(header.alg, Algorithm::ES256);
        assert_eq!(header.kid.as_deref(), Some("KEYID12345"));
        let mut validation = Validation::new(Algorithm::ES256);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        let key = DecodingKey::from_ec_pem(keys.public_pem.as_bytes()).unwrap();
        let claims = jsonwebtoken::decode::<Value>(bearer, &key, &validation).unwrap().claims;
        assert_eq!(claims["iss"], "TEAMID1234");
        let iat = claims["iat"].as_i64().unwrap();
        assert!((closing_relay::now_unix() - iat).abs() < 60);
    }

    // A prefix that matches nothing is refused without calling Apple.
    let missing = admin(&running.admin_socket, json!({ "cmd": "push_test", "to": "cdcd" })).await;
    assert_eq!(missing["ok"], false);
    assert_eq!(mock.seen.lock().unwrap().len(), 1);

    // Ten registrations an hour per address; another address is unaffected.
    for i in 0..9 {
        let body = json!({ "token": token(&format!("{:02x}", i + 1)), "env": "sandbox" });
        assert_eq!(register(&http, &base, body, "198.51.100.10").await, StatusCode::NO_CONTENT);
    }
    let body = json!({ "token": token("ee"), "env": "sandbox" });
    assert_eq!(register(&http, &base, body.clone(), "198.51.100.10").await, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(register(&http, &base, body, "198.51.100.99").await, StatusCode::NO_CONTENT);

    // Registrations survive a restart.
    running.stop().await;
    let running = server::start(config(dir.path(), &keys.private_path, &apns_url)).await.unwrap();
    let status = admin(&running.admin_socket, json!({ "cmd": "status" })).await;
    assert_eq!(status["ok"], true);
    assert_eq!(status["apns"], true);
    assert_eq!(status["push_check_tokens"], 11);
    running.stop().await;
}

#[tokio::test]
async fn apple_refusal_is_reported() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = tempfile::tempdir().unwrap();
    let keys = make_key(dir.path());
    let (apns_url, _mock) = start_mock(StatusCode::BAD_REQUEST, Some("BadDeviceToken")).await;
    let running = server::start(config(dir.path(), &keys.private_path, &apns_url)).await.unwrap();
    let base = format!("http://{}", running.http_addr);
    let http = reqwest::Client::new();

    let body = json!({ "token": token("cd"), "env": "sandbox" });
    assert_eq!(register(&http, &base, body, "203.0.113.5").await, StatusCode::NO_CONTENT);
    let sent = admin(&running.admin_socket, json!({ "cmd": "push_test", "to": "CDCD" })).await;
    assert_eq!(sent["ok"], false);
    assert_eq!(sent["status"], 400);
    assert_eq!(sent["reason"], "BadDeviceToken");
    assert_eq!(sent["env"], "sandbox");
    running.stop().await;
}

#[tokio::test]
async fn bad_key_fails_at_start() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("AuthKey_BAD.p8");
    std::fs::write(&key, "-----BEGIN PRIVATE KEY-----\nnot a key\n-----END PRIVATE KEY-----\n").unwrap();
    let err = server::start(config(dir.path(), &key, "http://127.0.0.1:9")).await.err().unwrap();
    assert!(err.contains("isn't an APNs .p8 key"), "{err}");
}

/// Network: Apple's production server must answer this client over HTTP/2.
/// No provider token is sent, so Apple refuses with 403; the protocol is the point.
#[tokio::test]
#[ignore = "talks to api.push.apple.com"]
async fn real_apns_speaks_http2() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let resp = reqwest::Client::new()
        .post(format!("https://api.push.apple.com/3/device/{}", token("00")))
        .header("apns-topic", TOPIC)
        .json(&json!({ "aps": {} }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.version(), reqwest::Version::HTTP_2);
    assert_eq!(resp.status().as_u16(), 403);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["reason"], "MissingProviderToken");
}
