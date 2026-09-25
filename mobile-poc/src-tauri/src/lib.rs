//! Emulator-oriented mobile shell. Media is deliberately simulated; scheduling is real.
use mpd_core::{Favorite as CoreFavorite, Presence};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};
use tauri::{State, WebviewWindow};

const DEFAULT_LIMIT: usize = 4;
const MAX_LIMIT: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RunMode {
    Stopped,
    Running,
    Paused,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DisplayMode {
    Grid,
    AudioChat,
}

#[derive(Clone)]
struct Favorite {
    login: String,
    enabled: bool,
    live: bool,
}

#[derive(Clone, Serialize)]
struct FavoriteView {
    login: String,
    enabled: bool,
    live: bool,
    selected: bool,
}

#[derive(Clone, Serialize)]
struct SessionView {
    login: String,
    session: u64,
}

#[derive(Clone, Serialize)]
struct Snapshot {
    mode: RunMode,
    display: DisplayMode,
    limit: usize,
    volume: u8,
    muted: bool,
    favorites: Vec<FavoriteView>,
    sessions: Vec<SessionView>,
    events: Vec<String>,
    media: &'static str,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Add { input: String },
    Remove { login: String },
    Move { login: String, position: usize },
    Enable { login: String, enabled: bool },
    DemoLive { login: String, live: bool },
    SetLimit { limit: usize },
    SetAudio { volume: u8, muted: bool },
    SetDisplay { display: String },
    Start,
    Pause,
    Stop,
    LoadDemo,
}

struct MobileController {
    mode: RunMode,
    display: DisplayMode,
    limit: usize,
    volume: u8,
    muted: bool,
    favorites: Vec<Favorite>,
    sessions: HashMap<String, u64>,
    next_session: u64,
    events: Vec<String>,
}

impl Default for MobileController {
    fn default() -> Self {
        let mut value = Self {
            mode: RunMode::Stopped,
            display: DisplayMode::Grid,
            limit: DEFAULT_LIMIT,
            volume: 25,
            muted: true,
            favorites: Vec::new(),
            sessions: HashMap::new(),
            next_session: 0,
            events: Vec::new(),
        };
        value.load_demo();
        value
    }
}

impl MobileController {
    fn load_demo(&mut self) {
        self.favorites = [
            "monstercat",
            "twitch",
            "bobross",
            "criticalrole",
            "riotgames",
            "rocketleague",
        ]
        .into_iter()
        .map(|login| Favorite {
            login: login.to_owned(),
            enabled: true,
            live: true,
        })
        .collect();
        self.sessions.clear();
        self.mode = RunMode::Stopped;
        self.log("Demo favorites loaded. No Twitch media was requested.");
    }

    fn log(&mut self, value: impl Into<String>) {
        self.events.insert(0, value.into());
        self.events.truncate(8);
    }

    fn desired(&self) -> Vec<String> {
        let ranked: Vec<_> = self
            .favorites
            .iter()
            .map(|favorite| CoreFavorite {
                login: favorite.login.clone(),
                enabled: favorite.enabled,
            })
            .collect();
        let presence: HashMap<_, _> = self
            .favorites
            .iter()
            .map(|favorite| {
                let mut value = Presence::default();
                value.observe(favorite.live.then_some("mobile-poc-broadcast"));
                (favorite.login.clone(), value)
            })
            .collect();
        let existing = self.sessions.keys().cloned().collect();
        mpd_core::select(&ranked, &presence, &existing, &HashMap::new(), self.limit)
    }

    fn reconcile(&mut self) {
        if self.mode != RunMode::Running {
            return;
        }
        let desired = self.desired();
        let desired_set: HashSet<_> = desired.iter().cloned().collect();
        self.sessions.retain(|login, _| desired_set.contains(login));
        for login in desired {
            if !self.sessions.contains_key(&login) {
                self.next_session += 1;
                self.sessions.insert(login, self.next_session);
            }
        }
    }

    fn action(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::Add { input } => {
                let login = mpd_core::normalize_login(&input).map_err(str::to_owned)?;
                if self
                    .favorites
                    .iter()
                    .any(|favorite| favorite.login == login)
                {
                    return Err("That channel is already a favorite.".into());
                }
                self.favorites.push(Favorite {
                    login: login.clone(),
                    enabled: true,
                    live: true,
                });
                self.log(format!("Added {login} as a simulated live favorite."));
                self.reconcile();
            }
            Action::Remove { login } => {
                let before = self.favorites.len();
                self.favorites.retain(|favorite| favorite.login != login);
                if self.favorites.len() == before {
                    return Err("Favorite not found.".into());
                }
                self.sessions.remove(&login);
                self.log(format!("Removed {login}."));
                self.reconcile();
            }
            Action::Move { login, position } => {
                let Some(index) = self
                    .favorites
                    .iter()
                    .position(|favorite| favorite.login == login)
                else {
                    return Err("Favorite not found.".into());
                };
                if position >= self.favorites.len() {
                    return Err("Priority position is out of range.".into());
                }
                let favorite = self.favorites.remove(index);
                self.favorites.insert(position, favorite);
                self.log(format!("Moved {login} to priority {}.", position + 1));
                self.reconcile();
            }
            Action::Enable { login, enabled } => {
                let favorite = self.favorite_mut(&login)?;
                favorite.enabled = enabled;
                self.log(format!(
                    "{} {login}.",
                    if enabled { "Enabled" } else { "Disabled" }
                ));
                self.reconcile();
            }
            Action::DemoLive { login, live } => {
                let favorite = self.favorite_mut(&login)?;
                favorite.live = live;
                self.log(format!(
                    "{login} is now simulated {}.",
                    if live { "live" } else { "offline" }
                ));
                self.reconcile();
            }
            Action::SetLimit { limit } => {
                if !(1..=MAX_LIMIT).contains(&limit) {
                    return Err(format!(
                        "Choose between 1 and {MAX_LIMIT} streams for this POC."
                    ));
                }
                self.limit = limit;
                self.log(format!("Stream capacity set to {limit}."));
                self.reconcile();
            }
            Action::SetAudio { volume, muted } => {
                if volume > 100 {
                    return Err("Volume must be 0–100%.".into());
                }
                self.volume = volume;
                self.muted = muted;
                self.log(format!(
                    "Audio set to {volume}%{}.",
                    if muted { " (muted)" } else { "" }
                ));
            }
            Action::SetDisplay { display } => {
                self.display = match display.as_str() {
                    "grid" => DisplayMode::Grid,
                    "audio_chat" => DisplayMode::AudioChat,
                    _ => return Err("Unknown display mode.".into()),
                };
                self.log(match self.display {
                    DisplayMode::Grid => "Showing the four-up grid.",
                    DisplayMode::AudioChat => "Showing the audio + chat interaction concept.",
                });
            }
            Action::Start => {
                self.mode = RunMode::Running;
                self.reconcile();
                self.log("Demo monitoring started.");
            }
            Action::Pause => {
                if self.mode == RunMode::Running {
                    self.mode = RunMode::Paused;
                    self.log("Automation paused; current assignments retained.");
                }
            }
            Action::Stop => {
                self.mode = RunMode::Stopped;
                self.sessions.clear();
                self.log("Stopped and closed all simulated sessions.");
            }
            Action::LoadDemo => self.load_demo(),
        }
        Ok(())
    }

    fn favorite_mut(&mut self, login: &str) -> Result<&mut Favorite, String> {
        self.favorites
            .iter_mut()
            .find(|favorite| favorite.login == login)
            .ok_or_else(|| "Favorite not found.".into())
    }

    fn snapshot(&self) -> Snapshot {
        let selected: HashSet<_> = self.sessions.keys().collect();
        let favorites = self
            .favorites
            .iter()
            .map(|favorite| FavoriteView {
                login: favorite.login.clone(),
                enabled: favorite.enabled,
                live: favorite.live,
                selected: selected.contains(&favorite.login),
            })
            .collect();
        let mut sessions: Vec<_> = self
            .sessions
            .iter()
            .map(|(login, session)| SessionView {
                login: login.clone(),
                session: *session,
            })
            .collect();
        sessions.sort_by_key(|session| {
            self.favorites
                .iter()
                .position(|favorite| favorite.login == session.login)
                .unwrap_or(usize::MAX)
        });
        Snapshot {
            mode: self.mode,
            display: self.display,
            limit: self.limit,
            volume: self.volume,
            muted: self.muted,
            favorites,
            sessions,
            events: self.events.clone(),
            media: "simulated",
        }
    }
}

#[tauri::command]
fn get_state(
    window: WebviewWindow,
    state: State<'_, Mutex<MobileController>>,
) -> Result<Snapshot, String> {
    if window.label() != "main" {
        return Err("Mobile manager command denied.".into());
    }
    state
        .lock()
        .map_err(|_| "Controller unavailable.".to_owned())
        .map(|value| value.snapshot())
}

#[tauri::command]
fn dispatch(
    window: WebviewWindow,
    state: State<'_, Mutex<MobileController>>,
    action: Action,
) -> Result<Snapshot, String> {
    if window.label() != "main" {
        return Err("Mobile manager command denied.".into());
    }
    let mut value = state
        .lock()
        .map_err(|_| "Controller unavailable.".to_owned())?;
    value.action(action)?;
    Ok(value.snapshot())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(MobileController::default()))
        .invoke_handler(tauri::generate_handler![get_state, dispatch])
        .run(tauri::generate_context!())
        .expect("MPD Viewer mobile POC failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_four_rust_selected_sessions() {
        let mut controller = MobileController::default();
        controller.action(Action::Start).unwrap();
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.sessions.len(), 4);
        assert_eq!(
            snapshot
                .sessions
                .iter()
                .map(|session| session.login.as_str())
                .collect::<Vec<_>>(),
            ["monstercat", "twitch", "bobross", "criticalrole"]
        );
    }

    #[test]
    fn priority_change_retains_existing_session_ids() {
        let mut controller = MobileController::default();
        controller.action(Action::Start).unwrap();
        let before = controller.sessions.clone();
        controller
            .action(Action::Move {
                login: "criticalrole".into(),
                position: 0,
            })
            .unwrap();
        for login in ["monstercat", "twitch", "bobross", "criticalrole"] {
            assert_eq!(controller.sessions.get(login), before.get(login));
        }
    }

    #[test]
    fn audio_chat_is_presentation_only() {
        let mut controller = MobileController::default();
        controller.action(Action::Start).unwrap();
        let sessions = controller.sessions.clone();
        controller
            .action(Action::SetDisplay {
                display: "audio_chat".into(),
            })
            .unwrap();
        assert_eq!(controller.sessions, sessions);
    }

    #[test]
    fn paused_controller_does_not_replace_assignments() {
        let mut controller = MobileController::default();
        controller.action(Action::Start).unwrap();
        controller.action(Action::Pause).unwrap();
        let sessions = controller.sessions.clone();
        controller
            .action(Action::DemoLive {
                login: "monstercat".into(),
                live: false,
            })
            .unwrap();
        assert_eq!(controller.sessions, sessions);
    }
}
