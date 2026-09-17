//! Public-client OAuth and complete batched Helix observations.
//! Credentials deliberately have no Debug/Serialize implementation and stay in Rust memory.
use std::{collections::{HashMap, HashSet}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;

const OAUTH: &str = "https://id.twitch.tv/oauth2";
const HELIX: &str = "https://api.twitch.tv/helix";

#[derive(Clone, Debug)]
pub struct ApiError {
    pub message: String,
    pub retry_after: Duration,
    pub reconnect: bool,
}
impl ApiError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), retry_after: Duration::from_secs(30), reconnect: false }
    }
    fn network(_: reqwest::Error) -> Self { Self::new("Network request failed; existing players are retained.") }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.message) }
}
impl std::error::Error for ApiError {}

#[derive(Clone)]
pub struct Twitch { http: Client }

#[derive(Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub struct Session {
    client_id: String,
    access: String,
    refresh: String,
    expires: Instant,
    validated: Instant,
    pub login: String,
}

#[derive(Deserialize)]
struct TokenReply { access_token: String, refresh_token: String, expires_in: u64 }
#[derive(Deserialize)]
struct Validated { client_id: String, #[serde(default)] login: String, expires_in: u64 }
#[derive(Deserialize)]
struct Stream { id: String, user_login: String }
#[derive(Deserialize, Default)]
struct Pagination { cursor: Option<String> }
#[derive(Deserialize)]
struct Page { data: Vec<Stream>, #[serde(default)] pagination: Pagination }

fn checked(response: Response) -> Result<Response, ApiError> {
    if response.status().is_success() { return Ok(response); }
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(ApiError { message: "Twitch authorization is no longer valid. Reconnect Twitch.".into(),
            retry_after: Duration::from_secs(60), reconnect: true });
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let reset = response.headers().get("Ratelimit-Reset").and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok()).map(|v| v.saturating_sub(now) + 1);
        let retry = response.headers().get("Retry-After").and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        return Err(ApiError { message: "Twitch rate limit reached; waiting before retry.".into(),
            retry_after: Duration::from_secs(reset.into_iter().chain(retry).max().unwrap_or(60).max(1)), reconnect: false });
    }
    Err(ApiError::new(format!("Twitch returned HTTP {}. No channels were marked offline.", status.as_u16())))
}

impl Twitch {
    pub fn new() -> Result<Self, ApiError> {
        let http = Client::builder().timeout(Duration::from_secs(20))
            .user_agent("MPD-Tabber-POC/0.1")
            .redirect(reqwest::redirect::Policy::none()).build().map_err(ApiError::network)?;
        Ok(Self { http })
    }

    pub async fn device_code(&self, client_id: &str) -> Result<DeviceCode, ApiError> {
        let response = self.http.post(format!("{OAUTH}/device"))
            .form(&[("client_id", client_id), ("scopes", "")])
            .send().await.map_err(ApiError::network)?;
        checked(response)?.json().await.map_err(|_| ApiError::new("Invalid device-code response from Twitch."))
    }

    pub async fn complete_device(&self, client_id: &str, code: DeviceCode) -> Result<Session, ApiError> {
        let deadline = Instant::now() + Duration::from_secs(code.expires_in.min(3600));
        let mut interval = Duration::from_secs(code.interval.max(5));
        loop {
            tokio::time::sleep(interval).await;
            if Instant::now() >= deadline { return Err(ApiError::new("Authorization code expired. Connect again.")); }
            let response = self.http.post(format!("{OAUTH}/token"))
                .form(&[("client_id", client_id), ("device_code", code.device_code.as_str()),
                    ("scopes", ""), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code")])
                .send().await.map_err(ApiError::network)?;
            if response.status().is_success() {
                let reply: TokenReply = response.json().await.map_err(|_| ApiError::new("Invalid token response."))?;
                let mut session = Session { client_id: client_id.into(), access: reply.access_token,
                    refresh: reply.refresh_token, expires: Instant::now() + Duration::from_secs(reply.expires_in),
                    validated: Instant::now(), login: String::new() };
                self.validate(&mut session).await?;
                return Ok(session);
            }
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                interval = interval.max(checked(response).unwrap_err().retry_after);
                continue;
            }
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            let message = body.get("message").or_else(|| body.get("error")).and_then(|v| v.as_str()).unwrap_or("");
            match message {
                "authorization_pending" => continue,
                "slow_down" => { interval += Duration::from_secs(5); continue; }
                _ => return Err(ApiError::new("Device authorization failed or was denied. Check that the app is registered as a Public client, then reconnect.")),
            }
        }
    }

    async fn validate(&self, session: &mut Session) -> Result<(), ApiError> {
        let response = self.http.get(format!("{OAUTH}/validate"))
            .header("Authorization", format!("OAuth {}", session.access))
            .send().await.map_err(ApiError::network)?;
        let value: Validated = checked(response)?.json().await.map_err(|_| ApiError::new("Invalid token validation response."))?;
        if value.client_id != session.client_id { return Err(ApiError::new("Token/client ID mismatch. Reconnect Twitch.")); }
        session.login = value.login;
        session.expires = Instant::now() + Duration::from_secs(value.expires_in);
        session.validated = Instant::now();
        Ok(())
    }

    async fn refresh(&self, session: &mut Session) -> Result<(), ApiError> {
        let response = self.http.post(format!("{OAUTH}/token"))
            .form(&[("grant_type", "refresh_token"), ("refresh_token", session.refresh.as_str()),
                ("client_id", session.client_id.as_str())])
            .send().await.map_err(ApiError::network)?;
        if response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::UNAUTHORIZED {
            return Err(ApiError { message: "Refresh token expired or was revoked. Reconnect Twitch.".into(),
                retry_after: Duration::from_secs(60), reconnect: true });
        }
        let reply: TokenReply = checked(response)?.json().await.map_err(|_| ApiError::new("Invalid refresh response. Reconnect Twitch."))?;
        // Replace the single-use refresh token before making any further request.
        session.access = reply.access_token;
        session.refresh = reply.refresh_token;
        session.expires = Instant::now() + Duration::from_secs(reply.expires_in);
        self.validate(session).await
    }

    async fn streams_once(&self, session: &Session, logins: &[String]) -> Result<HashMap<String, String>, ApiError> {
        let mut online = HashMap::new();
        for batch in logins.chunks(100) {
            let requested: HashSet<_> = batch.iter().map(String::as_str).collect();
            let mut cursor: Option<String> = None;
            let mut cursors = HashSet::new();
            loop {
                let mut query = vec![("first", "100".to_owned())];
                query.extend(batch.iter().map(|s| ("user_login", s.clone())));
                if let Some(value) = &cursor { query.push(("after", value.clone())); }
                let response = self.http.get(format!("{HELIX}/streams")).query(&query)
                    .header("Client-Id", &session.client_id).bearer_auth(&session.access)
                    .send().await.map_err(ApiError::network)?;
                let page: Page = checked(response)?.json().await.map_err(|_| ApiError::new("Invalid streams response; observation discarded."))?;
                for stream in page.data {
                    let login = stream.user_login.to_ascii_lowercase();
                    if requested.contains(login.as_str()) { online.insert(login, stream.id); }
                }
                match page.pagination.cursor.filter(|s| !s.is_empty()) {
                    Some(next) if cursors.insert(next.clone()) => cursor = Some(next),
                    Some(_) => return Err(ApiError::new("Repeated pagination cursor; observation discarded.")),
                    None => break,
                }
            }
        }
        // Atomic snapshot: an error in any batch/page discards all partial results.
        Ok(online)
    }

    /// Caller must serialize this method for each Session (refresh tokens are single-use).
    pub async fn poll(&self, session: &mut Session, logins: &[String]) -> Result<HashMap<String, String>, ApiError> {
        if session.expires.saturating_duration_since(Instant::now()) < Duration::from_secs(60) {
            self.refresh(session).await?;
        } else if session.validated.elapsed() >= Duration::from_secs(3600) {
            if let Err(error) = self.validate(session).await {
                if error.reconnect { self.refresh(session).await?; } else { return Err(error); }
            }
        }
        match self.streams_once(session, logins).await {
            Err(error) if error.reconnect => {
                self.refresh(session).await?;
                self.streams_once(session, logins).await
            }
            result => result,
        }
    }
}
