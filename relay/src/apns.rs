//! Apple Push Notification service: token (JWT) auth over HTTP/2.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;

use crate::config::ApnsConfig;

const PRODUCTION: &str = "https://api.push.apple.com";
const SANDBOX: &str = "https://api.sandbox.push.apple.com";

/// Apple rejects provider tokens older than an hour and throttles a team that
/// makes new ones more often than every 20 minutes.
const BEARER_LIFETIME: Duration = Duration::from_secs(40 * 60);

/// Which APNs server a device token belongs to. TestFlight and App Store builds
/// use production; builds run from Xcode use sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Env {
    Production,
    Sandbox,
}

impl Env {
    pub fn parse(s: &str) -> Option<Env> {
        match s {
            "production" => Some(Env::Production),
            "sandbox" => Some(Env::Sandbox),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Env::Production => "production",
            Env::Sandbox => "sandbox",
        }
    }
}

/// Apple's answer to one push.
#[derive(Debug)]
pub struct Outcome {
    pub status: u16,
    pub apns_id: Option<String>,
    /// Apple's reason code when it refused, such as `BadDeviceToken`.
    pub reason: Option<String>,
}

impl Outcome {
    pub fn accepted(&self) -> bool {
        self.status == 200
    }
}

pub struct Apns {
    http: reqwest::Client,
    key: EncodingKey,
    key_id: String,
    team_id: String,
    topic: String,
    endpoint_override: Option<String>,
    bearer: Mutex<Option<(String, SystemTime)>>,
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    iat: u64,
}

impl Apns {
    pub fn new(cfg: &ApnsConfig) -> Result<Apns, String> {
        let pem = std::fs::read(&cfg.key_path)
            .map_err(|e| format!("can't read the APNs key {}: {e}", cfg.key_path.display()))?;
        let key = EncodingKey::from_ec_pem(&pem)
            .map_err(|e| format!("{} isn't an APNs .p8 key: {e}", cfg.key_path.display()))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| format!("can't set up the HTTPS client: {e}"))?;
        let apns = Apns {
            http,
            key,
            key_id: cfg.key_id.clone(),
            team_id: cfg.team_id.clone(),
            topic: cfg.topic.clone(),
            endpoint_override: cfg.endpoint_override.clone(),
            bearer: Mutex::new(None),
        };
        apns.bearer()?; // a key that can't sign fails at start, not at the first alert
        Ok(apns)
    }

    fn bearer(&self) -> Result<String, String> {
        let mut cached = self.bearer.lock().unwrap_or_else(|p| p.into_inner());
        let now = SystemTime::now();
        if let Some((token, made)) = cached.as_ref()
            && now.duration_since(*made).is_ok_and(|age| age < BEARER_LIFETIME)
        {
            return Ok(token.clone());
        }
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());
        let iat = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "the system clock is before 1970".to_string())?
            .as_secs();
        let token = jsonwebtoken::encode(&header, &Claims { iss: &self.team_id, iat }, &self.key)
            .map_err(|e| format!("can't sign the APNs provider token: {e}"))?;
        *cached = Some((token.clone(), now));
        Ok(token)
    }

    /// Sends a visible alert. `Err` only when the request couldn't be made;
    /// a refusal from Apple comes back as an `Outcome` with its reason.
    pub async fn send_alert(&self, device_token: &str, env: Env, body: &str) -> Result<Outcome, String> {
        let base = self.endpoint_override.as_deref().unwrap_or(match env {
            Env::Production => PRODUCTION,
            Env::Sandbox => SANDBOX,
        });
        let payload = serde_json::json!({ "aps": { "alert": { "body": body }, "sound": "default" } });
        let resp = self
            .http
            .post(format!("{base}/3/device/{device_token}"))
            .header("authorization", format!("bearer {}", self.bearer()?))
            .header("apns-topic", &self.topic)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("couldn't reach APNs: {e}"))?;
        let status = resp.status().as_u16();
        let apns_id = resp.headers().get("apns-id").and_then(|v| v.to_str().ok()).map(str::to_owned);
        let reason = if status == 200 {
            None
        } else {
            resp.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("reason").and_then(|r| r.as_str()).map(str::to_owned))
        };
        if reason.as_deref() == Some("ExpiredProviderToken") {
            *self.bearer.lock().unwrap_or_else(|p| p.into_inner()) = None;
        }
        Ok(Outcome { status, apns_id, reason })
    }
}
