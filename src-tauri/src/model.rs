use serde::{Deserialize, Serialize};

/// MPD Tabber's public application identifier; not an OAuth token or client secret.
pub const DEFAULT_CLIENT_ID: &str = "ha94kk20cfu1tp74pgg8isgi88cpo7";

#[derive(Clone, Serialize, Deserialize)]
pub struct Favorite { pub login: String, pub enabled: bool }

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema: u32,
    pub favorites: Vec<Favorite>,
    pub limit: usize,
    pub volume: u8,
    pub muted: bool,
    pub demo: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { schema: 1, favorites: vec![], limit: 3, volume: 25,
            muted: false, demo: true }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 { return Err("Unsupported settings version.".into()); }
        if self.limit == 0 || self.limit > 1000 { return Err("Set a tab limit between 1 and 1000 (POC safety bound).".into()); }
        if self.volume > 100 { return Err("Volume must be 0–100%.".into()); }
        if self.favorites.len() > 2000 { return Err("POC supports at most 2000 saved favorites.".into()); }
        let mut seen = std::collections::HashSet::new();
        for f in &self.favorites {
            let login = mpd_core::normalize_login(&f.login).map_err(str::to_owned)?;
            if login != f.login || !seen.insert(login) { return Err("Invalid or duplicate favorite.".into()); }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Stopped, Running, Paused }

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Add { input: String },
    Remove { login: String },
    Move { login: String, position: usize },
    Enable { login: String, enabled: bool },
    SetLimit { limit: usize },
    SetAudio { volume: u8, muted: bool },
    SetDemo { demo: bool },
    DemoLive { login: String, live: bool },
    LoadDemo,
    Start,
    Pause,
    Stop,
    Refresh,
    Connect {},
    Disconnect,
    Focus { login: String },
    Skip { login: String },
    UndoSkip { login: String },
    Retry { login: String },
    ClearError,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Playback { Loading, Ready, Buffering, Playing, Paused, Blocked, Offline, Ended, Error }

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub session: u64,
    pub state: Playback,
    pub visible: bool,
    pub volume: Option<f64>,
    pub muted: Option<bool>,
}

#[derive(Clone, Serialize)]
pub struct FavoriteView {
    pub login: String,
    pub enabled: bool,
    pub presence: String,
    pub skipped: bool,
    pub demo_live: bool,
    pub open_error: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct PlayerView {
    pub login: String,
    pub session: u64,
    pub closing: bool,
    pub state: Playback,
    pub report_age_seconds: Option<u64>,
    pub visible: Option<bool>,
    pub volume: Option<f64>,
    pub muted: Option<bool>,
}
#[derive(Clone, Serialize)]
pub struct View {
    pub mode: Mode,
    pub settings: Settings,
    pub favorites: Vec<FavoriteView>,
    pub players: Vec<PlayerView>,
    pub connected_as: Option<String>,
    pub auth_pending: bool,
    pub user_code: Option<String>,
    pub last_check_seconds: Option<u64>,
    pub polling: bool,
    pub next_check_seconds: u64,
    pub player_origin: String,
    pub error: Option<String>,
    pub events: Vec<String>,
}
impl Default for View {
    fn default() -> Self {
        Self { mode: Mode::Stopped, settings: Settings::default(), favorites: vec![], players: vec![],
            connected_as: None, auth_pending: false, user_code: None, last_check_seconds: None,
            polling: false, next_check_seconds: 0, player_origin: String::new(), error: None, events: vec![] }
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;
    #[test]
    fn connect_rejects_application_id_overrides_and_unknown_fields() {
        assert!(serde_json::from_str::<Action>(r#"{"type":"connect"}"#).is_ok());
        assert!(serde_json::from_str::<Action>(r#"{"type":"connect","client_id":"other"}"#).is_err());
        assert!(serde_json::from_str::<Action>(r#"{"type":"connect","url":"https://evil.example"}"#).is_err());
    }
}
