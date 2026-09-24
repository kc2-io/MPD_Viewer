//! Public-client OAuth and complete batched Helix observations.
//! Credentials have no Debug implementation and are exposed only to a native secure store.
use std::{collections::{HashMap, HashSet}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};

const OAUTH: &str = "https://id.twitch.tv/oauth2";
const HELIX: &str = "https://api.twitch.tv/helix";

#[derive(Clone, Debug)]
pub struct ApiError {
    pub message: String,
    pub retry_after: Duration,
    pub reconnect: bool,
    invalid_access: bool,
}
impl ApiError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), retry_after: Duration::from_secs(30), reconnect: false, invalid_access: false }
    }
    fn network(_: reqwest::Error) -> Self { Self::new("Network request failed; existing players are retained.") }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.message) }
}
impl std::error::Error for ApiError {}

#[derive(Clone)]
pub struct Twitch { http: Client, oauth: String }

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
    user_id: String,
    persist_pending: bool,
    validation_pending: bool,
}

/// Native-only secret payload. Never send this value to IPC, logs, or preferences.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredSession {
    version: u8,
    client_id: String,
    access: String,
    refresh: String,
    user_id: String,
}
impl StoredSession {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|_| "Could not encode saved Twitch authorization.".into())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes)
            .map_err(|_| "Saved Twitch authorization is unreadable. Reconnect Twitch.".to_string())?;
        if value.version != 1 || value.client_id.is_empty() || value.access.is_empty()
            || value.refresh.is_empty() || value.user_id.is_empty() {
            return Err("Saved Twitch authorization is invalid. Reconnect Twitch.".into());
        }
        Ok(value)
    }
}
/// Implementations must serialize save with disconnect and reject stale auth generations.
pub trait SessionStore: Send + Sync {
    fn save(&self, session: &StoredSession) -> Result<(), String>;
}
struct MemoryOnly;
impl SessionStore for MemoryOnly {
    fn save(&self, _: &StoredSession) -> Result<(), String> { Ok(()) }
}
impl Session {
    pub fn from_stored(client_id: &str, stored: StoredSession) -> Result<Self, ApiError> {
        if stored.client_id != client_id {
            return Err(ApiError { message: "Saved Twitch authorization belongs to a different app. Reconnect Twitch.".into(),
                retry_after: Duration::from_secs(60), reconnect: true, invalid_access: false });
        }
        Ok(Self { client_id: stored.client_id, access: stored.access, refresh: stored.refresh,
            user_id: stored.user_id, login: String::new(), expires: Instant::now(),
            validated: Instant::now(), persist_pending: false, validation_pending: true })
    }
    pub fn stored(&self) -> StoredSession {
        StoredSession { version: 1, client_id: self.client_id.clone(), access: self.access.clone(),
            refresh: self.refresh.clone(), user_id: self.user_id.clone() }
    }
    fn persist(&mut self, store: &dyn SessionStore) -> Result<(), ApiError> {
        // Keep pending on failure; never lose the newly rotated refresh token in memory.
        store.save(&self.stored()).map_err(|_| ApiError::new(
            "Could not securely save Twitch authorization; retrying. Keep MPD Viewer open."))?;
        self.persist_pending = false;
        Ok(())
    }
}

#[derive(Deserialize)]
struct TokenReply { access_token: String, refresh_token: String, expires_in: u64 }
#[derive(Deserialize)]
struct Validated { client_id: String, #[serde(default)] login: String, #[serde(default)] user_id: String, expires_in: u64 }
#[derive(Deserialize)]
pub struct Stream {
    pub id: String,
    pub user_login: String,
    // Missing metadata must not become a fabricated zero or discard live status.
    #[serde(default)]
    pub viewer_count: Option<u32>,
}
#[derive(Deserialize, Default)]
struct Pagination { cursor: Option<String> }
#[derive(Deserialize)]
struct Page { data: Vec<Stream>, #[serde(default)] pagination: Pagination }

fn checked(response: Response) -> Result<Response, ApiError> {
    if response.status().is_success() { return Ok(response); }
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(ApiError { message: "Twitch authorization is no longer valid. Reconnect Twitch.".into(),
            retry_after: Duration::from_secs(60), reconnect: true, invalid_access: true });
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let reset = response.headers().get("Ratelimit-Reset").and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok()).map(|v| v.saturating_sub(now) + 1);
        let retry = response.headers().get("Retry-After").and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        return Err(ApiError { message: "Twitch rate limit reached; waiting before retry.".into(),
            retry_after: Duration::from_secs(reset.into_iter().chain(retry).max().unwrap_or(60).max(1)), reconnect: false, invalid_access: false });
    }
    Err(ApiError::new(format!("Twitch returned HTTP {}. No channels were marked offline.", status.as_u16())))
}

impl Twitch {
    pub fn new() -> Result<Self, ApiError> {
        let http = Client::builder().timeout(Duration::from_secs(20))
            .user_agent("MPD-Tabber-POC/0.1")
            .redirect(reqwest::redirect::Policy::none()).build().map_err(ApiError::network)?;
        Ok(Self { http, oauth: OAUTH.into() })
    }

    pub async fn device_code(&self, client_id: &str) -> Result<DeviceCode, ApiError> {
        let response = self.http.post(format!("{}/device", self.oauth))
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
            let response = self.http.post(format!("{}/token", self.oauth))
                .form(&[("client_id", client_id), ("device_code", code.device_code.as_str()),
                    ("scopes", ""), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code")])
                .send().await.map_err(ApiError::network)?;
            if response.status().is_success() {
                let reply: TokenReply = response.json().await.map_err(|_| ApiError::new("Invalid token response."))?;
                let mut session = Session { client_id: client_id.into(), access: reply.access_token,
                    refresh: reply.refresh_token, expires: Instant::now() + Duration::from_secs(reply.expires_in),
                    validated: Instant::now(), login: String::new(), user_id: String::new(), persist_pending: false, validation_pending: true };
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
        let response = self.http.get(format!("{}/validate", self.oauth))
            .header("Authorization", format!("OAuth {}", session.access))
            .send().await.map_err(ApiError::network)?;
        let value: Validated = checked(response)?.json().await.map_err(|_| ApiError::new("Invalid token validation response."))?;
        if value.client_id != session.client_id || value.user_id.is_empty() || value.login.is_empty()
            || (!session.user_id.is_empty() && value.user_id != session.user_id) {
            return Err(ApiError { message: "Saved Twitch authorization does not match this app or account. Reconnect Twitch.".into(),
                retry_after: Duration::from_secs(60), reconnect: true, invalid_access: false });
        }
        session.user_id = value.user_id;
        session.login = value.login;
        session.expires = Instant::now() + Duration::from_secs(value.expires_in);
        session.validated = Instant::now();
        session.validation_pending = false;
        Ok(())
    }

    async fn refresh(&self, session: &mut Session, store: &dyn SessionStore) -> Result<(), ApiError> {
        let response = self.http.post(format!("{}/token", self.oauth))
            .form(&[("grant_type", "refresh_token"), ("refresh_token", session.refresh.as_str()),
                ("client_id", session.client_id.as_str())])
            .send().await.map_err(ApiError::network)?;
        if response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::UNAUTHORIZED {
            return Err(ApiError { message: "Refresh token expired or was revoked. Reconnect Twitch.".into(),
                retry_after: Duration::from_secs(60), reconnect: true, invalid_access: false });
        }
        let reply: TokenReply = checked(response)?.json().await.map_err(|_| ApiError::new("Invalid refresh response. Reconnect Twitch."))?;
        // Replace the single-use refresh token before making any further request.
        session.access = reply.access_token;
        session.refresh = reply.refresh_token;
        session.expires = Instant::now() + Duration::from_secs(reply.expires_in);
        session.persist_pending = true;
        session.validation_pending = true;
        session.persist(store)?;
        self.validate(session).await
    }

    /// Validate saved credentials on every launch before using them. Transient errors
    /// must retain the vault entry; reconnect errors require its deletion by the caller.
    pub async fn restore_session(&self, session: &mut Session, store: &dyn SessionStore) -> Result<(), ApiError> {
        if session.persist_pending { session.persist(store)?; }
        match self.validate(session).await {
            // Only an invalid/expired access token permits refresh. Identity mismatch is terminal.
            Err(error) if error.invalid_access => {
                self.refresh(session, store).await
            }
            result => result,
        }
    }

    async fn streams_once(&self, session: &Session, logins: &[String]) -> Result<HashMap<String, Stream>, ApiError> {
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
                    if requested.contains(login.as_str()) { online.insert(login, stream); }
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
    pub async fn poll(&self, session: &mut Session, logins: &[String]) -> Result<HashMap<String, Stream>, ApiError> {
        self.poll_with_store(session, logins, &MemoryOnly).await
    }

    pub async fn poll_with_store(&self, session: &mut Session, logins: &[String], store: &dyn SessionStore) -> Result<HashMap<String, Stream>, ApiError> {
        if session.persist_pending { session.persist(store)?; }
        if session.validation_pending {
            self.restore_session(session, store).await?;
        }
        if session.expires.saturating_duration_since(Instant::now()) < Duration::from_secs(60) {
            self.refresh(session, store).await?;
        } else if session.validated.elapsed() >= Duration::from_secs(3600) {
            if let Err(error) = self.validate(session).await {
                if error.invalid_access { self.refresh(session, store).await?; } else { return Err(error); }
            }
        }
        match self.streams_once(session, logins).await {
            Err(error) if error.invalid_access => {
                self.refresh(session, store).await?;
                self.streams_once(session, logins).await
            }
            result => result,
        }
    }
}


#[cfg(test)]
mod stream_tests {
    use super::*;
    #[test]
    fn counts_deserialize_without_conflating_missing_and_zero() {
        let page: Page = serde_json::from_str(r#"{"data":[
            {"id":"1","user_login":"alpha","viewer_count":0},
            {"id":"2","user_login":"beta","viewer_count":1234567},
            {"id":"3","user_login":"gamma"}
        ]}"#).unwrap();
        assert_eq!(page.data[0].viewer_count, Some(0));
        assert_eq!(page.data[1].viewer_count, Some(1234567));
        assert_eq!(page.data[2].viewer_count, None);
        assert_eq!(page.data[2].id, "3");
    }
    #[test]
    fn invalid_counts_discard_the_observation_instead_of_wrapping() {
        for count in ["-1", "1.5", "4294967296", "\"100\""] {
            let json = format!(r#"{{"data":[{{"id":"1","user_login":"alpha","viewer_count":{count}}}]}}"#);
            assert!(serde_json::from_str::<Page>(&json).is_err());
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, sync::{Arc, Mutex}};
    fn saved() -> StoredSession {
        StoredSession::decode(br#"{"version":1,"client_id":"app","access":"old-access","refresh":"old-refresh","user_id":"123"}"#).unwrap()
    }
    #[derive(Default)]
    struct FakeStore { values: Mutex<Vec<Vec<u8>>>, fail: Mutex<bool> }
    impl SessionStore for FakeStore {
        fn save(&self, value: &StoredSession) -> Result<(), String> {
            if *self.fail.lock().unwrap() { return Err("test vault locked".into()); }
            self.values.lock().unwrap().push(value.encode()?);
            Ok(())
        }
    }
    // Loopback HTTP fixture only; production endpoints remain fixed HTTPS and nonredirecting.
    fn server(replies: Vec<(u16, &'static str)>) -> (Twitch, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            for (status, body) in replies {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
                        _ => panic!("OAuth fixture did not receive expected request"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 1024];
                loop {
                    let read = stream.read(&mut chunk).unwrap();
                    assert!(read > 0);
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(end) = request.windows(4).position(|x| x == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&request[..end]);
                        let length = header.lines().find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())).unwrap_or(0);
                        if request.len() >= end + 4 + length { break; }
                    }
                }
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        let mut twitch = Twitch::new().unwrap();
        twitch.oauth = format!("http://{address}");
        (twitch, thread)
    }
    const VALID: &str = r#"{"client_id":"app","login":"tester","user_id":"123","expires_in":3600}"#;
    const ROTATED: &str = r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":3600}"#;
    #[test]
    fn corrupt_or_wrong_client_credentials_are_rejected_without_exposing_tokens() {
        assert!(StoredSession::decode(b"private-garbage-token").is_err());
        assert!(StoredSession::decode(br#"{"version":2,"client_id":"app","access":"x","refresh":"y","user_id":"123"}"#).is_err());
        let error = Session::from_stored("other-app", saved()).err().unwrap();
        assert!(error.reconnect);
        assert!(!error.message.contains("old-access"));
    }
    #[tokio::test]
    async fn restore_validates_before_accepting_account() {
        let (twitch, thread) = server(vec![(200, VALID)]);
        let mut session = Session::from_stored("app", saved()).unwrap();
        assert!(session.login.is_empty());
        twitch.restore_session(&mut session, &FakeStore::default()).await.unwrap();
        assert_eq!(session.login, "tester");
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn different_account_is_terminal_and_never_refreshed() {
        let (twitch, thread) = server(vec![(200, r#"{"client_id":"app","login":"other","user_id":"999","expires_in":3600}"#)]);
        let mut session = Session::from_stored("app", saved()).unwrap();
        let error = twitch.restore_session(&mut session, &FakeStore::default()).await.unwrap_err();
        assert!(error.reconnect && !error.invalid_access);
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn expired_access_refreshes_and_saves_before_validation_network_failure() {
        let (twitch, thread) = server(vec![(401, "{}"), (200, ROTATED), (503, "{}")]);
        let store = FakeStore::default();
        let mut session = Session::from_stored("app", saved()).unwrap();
        let error = twitch.restore_session(&mut session, &store).await.unwrap_err();
        assert!(!error.reconnect);
        let values = store.values.lock().unwrap();
        assert_eq!(values.len(), 1);
        let saved = StoredSession::decode(&values[0]).unwrap();
        assert!(saved.refresh == "new-refresh");
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn failed_vault_write_keeps_rotated_token_for_retry_without_second_refresh() {
        let (twitch, thread) = server(vec![(401, "{}"), (200, ROTATED), (200, VALID)]);
        let store = Arc::new(FakeStore::default());
        *store.fail.lock().unwrap() = true;
        let mut session = Session::from_stored("app", saved()).unwrap();
        let error = twitch.restore_session(&mut session, store.as_ref()).await.unwrap_err();
        assert!(!error.reconnect && session.persist_pending);
        assert!(session.refresh == "new-refresh");
        *store.fail.lock().unwrap() = false;
        twitch.restore_session(&mut session, store.as_ref()).await.unwrap();
        assert!(!session.persist_pending);
        assert_eq!(session.login, "tester");
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn poll_revalidates_rotation_after_transient_validation_failure() {
        let (twitch, thread) = server(vec![(401, "{}"), (200, ROTATED), (503, "{}"), (200, VALID)]);
        let store = FakeStore::default();
        let mut session = Session::from_stored("app", saved()).unwrap();
        assert!(twitch.restore_session(&mut session, &store).await.is_err());
        assert!(session.validation_pending);
        // No channels: validates without requesting Helix or inventing observations.
        assert!(twitch.poll_with_store(&mut session, &[], &store).await.unwrap().is_empty());
        assert!(!session.validation_pending);
        assert_eq!(session.login, "tester");
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn revoked_refresh_is_terminal_but_server_error_is_not() {
        for (status, reconnect) in [(400, true), (503, false)] {
            let (twitch, thread) = server(vec![(401, "{}"), (status, "{}")]);
            let store = FakeStore::default();
            let mut session = Session::from_stored("app", saved()).unwrap();
            let error = twitch.restore_session(&mut session, &store).await.unwrap_err();
            assert_eq!(error.reconnect, reconnect);
            assert!(store.values.lock().unwrap().is_empty());
            thread.join().unwrap();
        }
    }
}
