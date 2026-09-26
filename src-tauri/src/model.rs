use serde::{Deserialize, Serialize};

/// MPD Tabber's public application identifier; not an OAuth token or client secret.
pub const DEFAULT_CLIENT_ID: &str = "ha94kk20cfu1tp74pgg8isgi88cpo7";

#[derive(Clone, Serialize, Deserialize)]
pub struct Favorite { pub login: String, pub enabled: bool, #[serde(default)] pub watch_minutes: Option<u32> }

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema: u32,
    pub favorites: Vec<Favorite>,
    pub limit: usize,
    pub rescan_minutes: u32,
    pub volume: u8,
    pub muted: bool,
    pub preferred_quality: String,
    pub demo: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { schema: 1, favorites: vec![], limit: 3, rescan_minutes: 1, volume: 25,
            muted: false, preferred_quality: "auto".into(), demo: true }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["auto","source","160p","180p","240p","360p","480p","720p","1080p","1440p","2160p"].contains(&self.preferred_quality.as_str()) {
            return Err("Choose a supported preferred video quality.".into());
        }
        if self.schema != 1 { return Err("Unsupported settings version.".into()); }
        if self.limit == 0 || self.limit > 1000 { return Err("Set a tab limit between 1 and 1000 (POC safety bound).".into()); }
        if !(1..=60).contains(&self.rescan_minutes) { return Err("Set a live-status rescan interval between 1 and 60 whole minutes.".into()); }
        if self.volume > 100 { return Err("Volume must be 0–100%.".into()); }
        if self.favorites.len() > 2000 { return Err("POC supports at most 2000 saved favorites.".into()); }
        let mut seen = std::collections::HashSet::new();
        for f in &self.favorites {
            if f.watch_minutes.is_some_and(|m| !(1..=1440).contains(&m)) { return Err("Timer must be 1–1440 whole minutes or Always.".into()); }
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
    SetRescan { minutes: u32 },
    SetTimer { login: String, minutes: Option<u32> },
    SetAudio { volume: u8, muted: bool },
    SetQuality { quality: String },
    SetWindowMute { session: Option<u64>, muted: bool },
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

/// Last observed audience size. A missing value is never represented as zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ViewerCount { pub count: u32, pub stale: bool }
impl ViewerCount {
    pub fn from_presence(presence: Option<&mpd_core::Presence>, enabled: bool, demo: bool) -> Option<Self> {
        if !enabled || demo { return None; }
        let p = presence?;
        if p.broadcast_id.is_none() || p.missing_polls > 0 { return None; }
        p.viewer_count.map(|count| Self { count, stale: !p.fresh })
    }
    pub fn label(self) -> String {
        let digits = self.count.to_string();
        let mut grouped = String::new();
        for (index, ch) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) { grouped.push(','); }
            grouped.push(ch);
        }
        format!("{grouped} viewer{}{}", if self.count == 1 { "" } else { "s" },
            if self.stale { " (stale)" } else { "" })
    }
}

#[derive(Clone, Serialize)]
pub struct FavoriteView {
    pub watch_minutes: Option<u32>,
    pub login: String,
    pub enabled: bool,
    pub presence: String,
    pub viewer_count: Option<ViewerCount>,
    pub skipped: bool,
    pub demo_live: bool,
    pub open_error: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct PlayerView {
    pub timer: TimerView,
    pub login: String,
    pub session: u64,
    pub closing: bool,
    pub state: Playback,
    pub report_age_seconds: Option<u64>,
    pub visible: Option<bool>,
    pub volume: Option<f64>,
    pub muted: Option<bool>,
    pub window_muted: Option<bool>,
    pub window_mute_override: Option<bool>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TimerView {
    Unlimited,
    Counting { remaining_seconds: u64 },
    AutomationPaused { remaining_seconds: u64 },
    WaitingForAlternative,
    ClosingForRotation,
}

#[derive(Clone, Serialize)]
pub struct ViewerCapabilities {
    pub backend: String,
    pub telemetry: bool,
    pub media_controls: bool,
    pub window_mute_controls: bool,
    pub twitch_channel_page: bool,
}
impl ViewerCapabilities {
    pub fn twitch_page() -> Self {
        Self { backend: "twitch-page".into(), telemetry: false,
            media_controls: false, window_mute_controls: crate::window_audio::supported(),
            twitch_channel_page: true }
    }
    pub fn wrapper(backend: &str) -> Self {
        Self { backend: backend.into(), telemetry: true,
            media_controls: true, window_mute_controls: false, twitch_channel_page: false }
    }
    pub fn selected() -> Self {
        match crate::viewer_mode::selected() {
            crate::viewer_mode::ViewerMode::TwitchPage => Self::twitch_page(),
            crate::viewer_mode::ViewerMode::Embedded => Self::wrapper("twitch-embed"),
        }
    }
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
    pub viewer: ViewerCapabilities,
    pub player_origin: String,
    pub error: Option<String>,
    pub events: Vec<String>,
}
impl Default for View {
    fn default() -> Self {
        Self { mode: Mode::Stopped, settings: Settings::default(), favorites: vec![], players: vec![],
            connected_as: None, auth_pending: false, user_code: None, last_check_seconds: None,
            polling: false, next_check_seconds: 0, viewer: ViewerCapabilities::selected(),
            player_origin: String::new(), error: None, events: vec![] }
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
    #[test]
    fn rescan_rejects_malformed_or_extra_fields_at_the_manager_boundary() {
        assert!(serde_json::from_str::<Action>(r#"{"type":"set_rescan","minutes":1}"#).is_ok());
        for value in ["-1","1.5","4294967296","\"NaN\"","true"] {
            assert!(serde_json::from_str::<Action>(&format!(r#"{{"type":"set_rescan","minutes":{value}}}"#)).is_err());
        }
        assert!(serde_json::from_str::<Action>(r#"{"type":"set_rescan","minutes":1,"seconds":30}"#).is_err());
    }
    #[test]
    fn window_mute_accepts_only_the_bounded_global_or_session_shape() {
        assert!(serde_json::from_str::<Action>(r#"{"type":"set_window_mute","muted":true}"#).is_ok());
        assert!(serde_json::from_str::<Action>(r#"{"type":"set_window_mute","session":7,"muted":false}"#).is_ok());
        assert!(serde_json::from_str::<Action>(r#"{"type":"set_window_mute","login":"alpha","muted":true}"#).is_err());
        assert!(serde_json::from_str::<Action>(r#"{"type":"set_window_mute","session":"7","muted":true}"#).is_err());
    }
}


#[cfg(test)]
mod viewer_count_tests {
    use super::*;
    #[test]
    fn display_distinguishes_live_stale_unavailable_disabled_and_demo() {
        let mut p = mpd_core::Presence::default();
        assert_eq!(ViewerCount::from_presence(Some(&p), true, false), None);
        p.observe_stream(Some("a1"), Some(0));
        assert_eq!(ViewerCount::from_presence(Some(&p), true, false), Some(ViewerCount { count: 0, stale: false }));
        assert_eq!(ViewerCount::from_presence(Some(&p), false, false), None);
        assert_eq!(ViewerCount::from_presence(Some(&p), true, true), None);
        p.stale();
        assert_eq!(ViewerCount::from_presence(Some(&p), true, false), Some(ViewerCount { count: 0, stale: true }));
        p.observe_stream(None, None);
        assert_eq!(ViewerCount::from_presence(Some(&p), true, false), None);
        p.observe_stream(Some("a2"), Some(20));
        assert_eq!(ViewerCount::from_presence(Some(&p), true, false), Some(ViewerCount { count: 20, stale: false }));
        p.observe_stream(Some("a2"), None);
        assert_eq!(ViewerCount::from_presence(Some(&p), true, false), None);
    }
    #[test]
    fn title_labels_group_counts_and_mark_stale_without_rounding() {
        for (count, expected) in [(0,"0 viewers"), (1,"1 viewer"), (999,"999 viewers"),
            (1000,"1,000 viewers"), (1234567,"1,234,567 viewers"), (u32::MAX,"4,294,967,295 viewers")] {
            assert_eq!(ViewerCount { count, stale: false }.label(), expected);
            assert_eq!(ViewerCount { count, stale: true }.label(), format!("{expected} (stale)"));
        }
    }
}
