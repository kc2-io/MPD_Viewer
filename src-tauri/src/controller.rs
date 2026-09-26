//! Single-owner controller. Network results are generation-tagged; native window
//! destruction is acknowledged before replacement windows consume capacity.
use std::{collections::{HashMap, HashSet, VecDeque}, sync::Arc, time::{Duration, Instant}};
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use crate::{model::*, player::{self, Host}, storage::Store};
use mpd_core::Presence;
use mpd_twitch::{ApiError, Session, SessionStore, Stream, Twitch};
use crate::credential_store::{CredentialStore, ScopedStore};

type Credentials = Arc<Mutex<Session>>;

fn monitored_poll_interval(minutes: u32) -> Duration {
    Duration::from_secs(u64::from(minutes) * 60)
}
fn stale_after(minutes: u32) -> Duration {
    monitored_poll_interval(minutes).saturating_mul(3).max(Duration::from_secs(90))
}
fn scheduled_poll(now: Instant, last_check: Option<Instant>, not_before: Instant, minutes: u32) -> Instant {
    let due = last_check.map_or(now, |checked| checked + monitored_poll_interval(minutes));
    due.max(now).max(not_before)
}
fn rescheduled_poll(now: Instant, last_check: Option<Instant>, not_before: Instant, next_poll: Instant, minutes: u32) -> Instant {
    // A failed request or manual Check now can already be queued exactly at the
    // rate-limit boundary. Changing the normal cadence must not postpone it.
    if not_before > now && next_poll <= not_before { not_before }
    else { scheduled_poll(now,last_check,not_before,minutes) }
}

// A small async seam keeps failed restoration's *same* in-memory refresh token
// alive. No Tauri handle, UI state, logging, or credential serialization crosses it.
pub(crate) struct RecoveryError { message: String, reconnect: bool, delay: Duration }
impl From<ApiError> for RecoveryError {
    fn from(error: ApiError) -> Self { Self { message: error.message, reconnect: error.reconnect, delay: error.retry_after } }
}
impl RecoveryError {
    fn storage(message: String) -> Self { Self { message, reconnect: false, delay: Duration::from_secs(30) } }
}
trait RecoveryDriver {
    type Credentials;
    fn load(&self) -> Result<Option<Self::Credentials>, RecoveryError>;
    fn recover(&self, credentials: &mut Self::Credentials, validate: bool) -> impl std::future::Future<Output=Result<String,RecoveryError>> + Send;
}
pub(crate) struct RecoveryOutcome<T> { credentials: Option<T>, result: Result<Option<String>, RecoveryError> }
async fn recover_authorization<D: RecoveryDriver>(driver: &D, credentials: Option<D::Credentials>, validate: bool) -> RecoveryOutcome<D::Credentials> {
    let mut credentials = match credentials {
        Some(value) => Some(value),
        None => match driver.load() {
            Ok(value) => value,
            Err(error) => return RecoveryOutcome { credentials: None, result: Err(error) },
        },
    };
    let result = match credentials.as_mut() {
        Some(value) => driver.recover(value, validate).await.map(Some),
        None => Ok(None),
    };
    RecoveryOutcome { credentials, result }
}
struct NativeRecovery { twitch: Twitch, scope: ScopedStore }
impl RecoveryDriver for NativeRecovery {
    type Credentials = Credentials;
    fn load(&self) -> Result<Option<Credentials>, RecoveryError> {
        self.scope.load().map_err(RecoveryError::storage)?.map(|stored| {
            Session::from_stored(DEFAULT_CLIENT_ID, stored).map(|session| Arc::new(Mutex::new(session))).map_err(RecoveryError::from)
        }).transpose()
    }
    async fn recover(&self, credentials: &mut Credentials, validate: bool) -> Result<String, RecoveryError> {
        let mut session = credentials.lock().await;
        if validate { self.twitch.restore_session(&mut session, &self.scope).await?; }
        else { self.scope.save(&session.stored()).map_err(RecoveryError::storage)?; }
        Ok(session.login.clone())
    }
}
fn accept_recovery(current_epoch: u64, result_epoch: u64, pending: bool) -> bool {
    current_epoch == result_epoch && pending
}
struct PendingRecovery { credentials: Option<Credentials>, validate: bool, next_attempt: Instant, backoff: u64, inflight: bool }
impl PendingRecovery {
    fn new(credentials: Option<Credentials>, validate: bool) -> Self {
        Self { credentials, validate, next_attempt: Instant::now(), backoff: 30, inflight: false }
    }
}

fn close_tracked_auth_window(tracked: &mut Option<u64>, close: impl FnOnce(u64) -> Result<(), String>) -> Result<(), String> {
    if let Some(epoch) = *tracked {
        close(epoch)?;
        *tracked = None;
    }
    Ok(())
}
fn auth_completion_ready(epoch: u64, tracked: Option<u64>, returned: Option<u64>, pending: bool, connected: bool) -> bool {
    !pending && connected && tracked == Some(epoch) && returned == Some(epoch)
}

pub enum Message {
    Action(Action, oneshot::Sender<Result<(), String>>),
    Report(String, Report),
    Destroyed(String),
    AuthCode { epoch: u64, code: String, url: String },
    AuthReturnLoaded { epoch: u64 },
    Authorized { epoch: u64, result: Result<Session, ApiError> },
    Recovered { epoch: u64, outcome: RecoveryOutcome<Credentials> },
    Polled { id: u64, generation: u64, epoch: u64, monitored: bool,
        result: Result<HashMap<String, Stream>, ApiError> },
}
#[derive(Clone)]
pub struct Handle {
    pub tx: mpsc::Sender<Message>,
    pub view: watch::Receiver<View>,
}
struct PlayerSession {
    id: u64, login: String, label: String, closing: bool,
    report: Option<Report>, reported: Option<Instant>,
    rate_start: Instant, rate_count: u8, quality_dirty: bool,
    window_mute_override: bool, window_muted: Option<bool>,
}

impl PlayerSession {
    fn accept_report(&mut self, report: Report) {
        if matches!(report.state, Playback::Loading | Playback::Ready) { self.quality_dirty = true; }
        self.reported = Some(Instant::now()); self.report = Some(report);
    }
}

#[derive(Clone, Debug)]
struct MuteTarget { id: u64, label: String, before: bool, after: bool }
struct MuteTransaction { confirmed: HashMap<u64, bool>, error: Option<String> }

fn apply_mute_transaction(targets: &[MuteTarget], mut apply: impl FnMut(&str, bool) -> Result<(), String>) -> MuteTransaction {
    let mut confirmed: HashMap<_, _> = targets.iter().map(|target| (target.id, target.before)).collect();
    let mut changed = Vec::new();
    for target in targets {
        match apply(&target.label, target.after) {
            Ok(()) => {
                confirmed.insert(target.id, target.after);
                if target.before != target.after { changed.push(target); }
            }
            Err(error) => {
                let mut rollback_failed = Vec::new();
                for previous in changed.into_iter().rev() {
                    if let Err(rollback_error) = apply(&previous.label, previous.before) {
                        rollback_failed.push(format!("{} ({rollback_error})", previous.id));
                    } else { confirmed.insert(previous.id, previous.before); }
                }
                let rollback = if rollback_failed.is_empty() { String::new() }
                    else { format!(" Rollback also failed for sessions {}.", rollback_failed.join(", ")) };
                return MuteTransaction { confirmed,
                    error: Some(format!("Could not change page-window mute for session {}: {error}.{rollback}", target.id)) };
            }
        }
    }
    MuteTransaction { confirmed, error: None }
}

fn apply_persisted_mute_transaction(
    targets: &[MuteTarget],
    mut apply: impl FnMut(&str, bool) -> Result<(), String>,
    persist: impl FnOnce() -> Result<(), String>,
) -> MuteTransaction {
    let applied = apply_mute_transaction(targets, &mut apply);
    if applied.error.is_some() { return applied; }
    if let Err(storage_error) = persist() {
        let mut confirmed: HashMap<_, _> = targets.iter().map(|target| (target.id, target.after)).collect();
        let mut rollback_failed = Vec::new();
        for target in targets.iter().rev() {
            match apply(&target.label, target.before) {
                Ok(()) => { confirmed.insert(target.id, target.before); }
                Err(error) => rollback_failed.push(format!("{} ({error})", target.id)),
            }
        }
        let rollback_error = if rollback_failed.is_empty() { String::new() }
            else { format!(" Rollback also failed for sessions {}.", rollback_failed.join(", ")) };
        return MuteTransaction { confirmed,
            error: Some(format!("Could not save global page mute: {storage_error}.{rollback_error}")) };
    }
    applied
}

fn session_mute_label(players: &HashMap<u64, PlayerSession>, id: u64) -> Result<String, String> {
    let player = players.get(&id).ok_or("That player session is no longer active.")?;
    if player.closing { return Err("That player session is closing.".into()); }
    player.window_muted.ok_or("That player does not support native window mute.")?;
    Ok(player.label.clone())
}

pub struct Controller {
    app: AppHandle, tx: mpsc::Sender<Message>, publish: watch::Sender<View>,
    store: Store, host: Host, twitch: Twitch, settings: Settings, mode: Mode,
    presence: HashMap<String, Presence>, skipped: HashMap<String, String>,
    demo_live: HashMap<String, String>, players: HashMap<u64, PlayerSession>,
    failed: HashMap<String, String>, next_id: u64, generation: u64, auth_epoch: u64,
    credentials: Option<Credentials>, connected_as: Option<String>,
    credential_store: CredentialStore, auth_recovery: Option<PendingRecovery>,
    auth_task: Option<tokio::task::JoinHandle<()>>, auth_pending: bool,
    user_code: Option<String>, auth_url: Option<String>, auth_window_epoch: Option<u64>, auth_return_epoch: Option<u64>,
    polling: Option<u64>, poll_id: u64, next_poll: Instant, not_before: Instant,
    last_check: Option<Instant>, backoff: u64, error: Option<String>,
    events: VecDeque<String>, started: Instant,
    rotation: mpd_core::timer::Rotation, timer_tick: Instant, close_retry: HashMap<u64,Instant>,
}
impl Controller {
    pub fn new(app: AppHandle, tx: mpsc::Sender<Message>, publish: watch::Sender<View>,
        store: Store, host: Host, settings: Settings, twitch: Twitch) -> Self {
        Self { app, tx, publish, store, host, twitch, settings, mode: Mode::Stopped,
            presence: HashMap::new(), skipped: HashMap::new(), demo_live: HashMap::new(),
            players: HashMap::new(), failed: HashMap::new(), next_id: 0, generation: 0,
            auth_epoch: 0, credentials: None, connected_as: None, auth_task: None,
            credential_store: CredentialStore::new(), auth_recovery: None,
            auth_pending: false, user_code: None, auth_url: None, auth_window_epoch: None, auth_return_epoch: None, polling: None,
            poll_id: 0, next_poll: Instant::now(), not_before: Instant::now(),
            last_check: None, backoff: 30, error: None, rotation: Default::default(), timer_tick: Instant::now(), close_retry: HashMap::new(), events: VecDeque::new(), started: Instant::now() }
    }
    fn log(&mut self, text: impl Into<String>) {
        self.events.push_front(format!("+{}s  {}", self.started.elapsed().as_secs(), text.into()));
        self.events.truncate(60);
    }
    fn mark_stale(&mut self) { for p in self.presence.values_mut() { p.stale(); } }
    fn request_poll(&mut self) { self.next_poll = Instant::now().max(self.not_before); }
    fn changed(&mut self) { self.generation += 1; self.request_poll(); }
    fn save(&mut self, proposed: Settings) -> Result<(), String> {
        self.store.save(&proposed)?; self.settings = proposed; Ok(())
    }
    fn record_confirmed_mutes(&mut self, confirmed: &HashMap<u64, bool>) {
        for (id, muted) in confirmed {
            if let Some(player) = self.players.get_mut(id) { player.window_muted = Some(*muted); }
        }
    }
    fn set_window_mute(&mut self, session: Option<u64>, muted: bool) -> Result<(), String> {
        let capabilities = self.host.capabilities(self.settings.demo);
        if !capabilities.twitch_channel_page || !capabilities.window_mute_controls {
            return Err("Native page-window mute is unavailable for the active viewer mode and platform.".into());
        }
        if let Some(id) = session {
            if self.settings.muted { return Err("Turn off global page mute before changing one player.".into()); }
            let label = session_mute_label(&self.players, id)?;
            player::window_mute(&self.app, &label, muted)?;
            let player = self.players.get_mut(&id).ok_or("That player session is no longer active.")?;
            player.window_mute_override = muted; player.window_muted = Some(muted);
            self.log(format!("{} page audio for session {id}.", if muted { "Muted" } else { "Unmuted" }));
            return Ok(());
        }
        let targets: Vec<_> = self.players.values().filter(|player| !player.closing)
            .filter_map(|player| player.window_muted.map(|before| MuteTarget {
                id: player.id, label: player.label.clone(), before,
                after: muted || player.window_mute_override,
            })).collect();
        let mut proposed = self.settings.clone(); proposed.muted = muted;
        let app = self.app.clone();
        let applied = apply_persisted_mute_transaction(&targets,
            |label, target| player::window_mute(&app, label, target),
            || self.store.save(&proposed));
        self.record_confirmed_mutes(&applied.confirmed);
        if let Some(error) = applied.error { return Err(error); }
        self.settings = proposed;
        self.log(format!("{} all page-window audio.", if muted { "Muted" } else { "Unmuted" }));
        Ok(())
    }
    fn update_demo(&mut self) {
        if !self.settings.demo { return; }
        for f in &self.settings.favorites {
            let p = self.presence.entry(f.login.clone()).or_default();
            p.observe(self.demo_live.get(&f.login).map(String::as_str));
            // Demo switches are explicit observations, not flaky network samples.
            if !self.demo_live.contains_key(&f.login) { p.observe(None); }
        }
        self.last_check = Some(Instant::now());
    }
    fn skip(&mut self, login: &str) {
        if let Some(id) = self.presence.get(login).and_then(|p| p.broadcast_id.clone()) {
            self.skipped.insert(login.into(), id); self.log(format!("Skipped {login} for this broadcast."));
        }
    }
    fn disconnected(&mut self, forget: bool) -> Result<(), String> {
        let close = close_tracked_auth_window(&mut self.auth_window_epoch, |epoch| crate::viewer_auth::close(&self.app, epoch));
        self.auth_epoch += 1;
        let vault = if forget { self.credential_store.forget(self.auth_epoch) }
            else { self.credential_store.begin_epoch(self.auth_epoch).map(|_| ()) };
        self.auth_recovery = None;
        self.polling = None;
        if let Some(task) = self.auth_task.take() { task.abort(); }
        self.auth_pending = false; self.credentials = None; self.connected_as = None;
        self.user_code = None; self.auth_url = None; self.auth_return_epoch = None; self.mark_stale();
        vault.and(close)
    }
    fn close_completed_connection(&mut self) {
        if auth_completion_ready(self.auth_epoch, self.auth_window_epoch, self.auth_return_epoch,
            self.auth_pending, self.credentials.is_some()) {
            if close_tracked_auth_window(&mut self.auth_window_epoch, |epoch|
                crate::viewer_auth::close(&self.app, epoch)).is_err() {
                self.error = Some("Twitch connected, but its window could not close automatically. You can close it manually.".into());
            }
        }
    }
    fn open_connection(&self) -> Result<(), String> {
        let (Some(url), Some(code)) = (&self.auth_url, &self.user_code) else { return Ok(()); };
        let activation = crate::viewer_auth::Activation::parse(url, code)?;
        crate::viewer_auth::open(&self.app, self.auth_epoch, activation)
    }
    fn action(&mut self, action: Action) -> Result<(), String> {
        self.advance_timers();
        match action {
            Action::Add { input } => {
                if input.len() > 512 { return Err("Channel input is too long.".into()); }
                let login = mpd_core::normalize_login(&input).map_err(str::to_owned)?;
                if self.settings.favorites.iter().any(|f| f.login == login) { return Err("That channel is already a favorite.".into()); }
                let mut s = self.settings.clone(); s.favorites.push(Favorite { login: login.clone(), enabled: true, watch_minutes: None });
                self.save(s)?; self.presence.insert(login.clone(), Presence::default());
                self.changed(); self.log(format!("Added {login}."));
            }
            Action::Remove { login } => {
                let mut s = self.settings.clone(); s.favorites.retain(|f| f.login != login); self.save(s)?;
                self.presence.remove(&login); self.skipped.remove(&login); self.demo_live.remove(&login);
                self.failed.remove(&login); self.rotation.remove(&login); self.changed();
            }
            Action::Move { login, position } => {
                let mut s = self.settings.clone();
                let index = s.favorites.iter().position(|f| f.login == login).ok_or("Unknown favorite.")?;
                let favorite = s.favorites.remove(index);
                s.favorites.insert(position.min(s.favorites.len()), favorite); self.save(s)?;
                self.rotation.clear_round();
            }
            Action::Enable { login, enabled } => {
                let mut s = self.settings.clone();
                s.favorites.iter_mut().find(|f| f.login == login).ok_or("Unknown favorite.")?.enabled = enabled;
                self.save(s)?;
                self.rotation.remove(&login);
                // Disabled channels are not polled; re-enabling waits for a new count.
                if let Some(p) = self.presence.get_mut(&login) { p.viewer_count = None; }
                self.changed();
            }
            Action::SetLimit { limit } => { let mut s = self.settings.clone(); s.limit = limit; self.save(s)?; self.rotation.clear_round(); }
            Action::SetRescan { minutes } => {
                let mut s = self.settings.clone(); s.rescan_minutes = minutes; self.save(s)?;
                // An in-flight result schedules from the latest saved setting when
                // it completes. Otherwise apply the new interval immediately.
                if !self.settings.demo && self.mode != Mode::Stopped && self.polling.is_none() {
                    self.next_poll = rescheduled_poll(Instant::now(), self.last_check, self.not_before,
                        self.next_poll, self.settings.rescan_minutes);
                }
            }
            Action::SetTimer { login, minutes } => {
                let mut s=self.settings.clone();
                s.favorites.iter_mut().find(|f| f.login==login).ok_or("Unknown favorite.")?.watch_minutes=minutes;
                self.save(s)?; self.advance_timers(); self.rotation.configure(&login,minutes);
                self.log(format!("Timer for {login}: {}.",minutes.map_or("Always".into(),|m| format!("{m} minutes assigned time"))));
            }
            Action::SetAudio { volume, muted } => {
                if !self.host.capabilities(self.settings.demo).media_controls {
                    return Err("Volume controls are unavailable for full Twitch pages.".into());
                }
                let mut s = self.settings.clone(); s.volume = volume; s.muted = muted; self.save(s)?;
                for p in self.players.values() { player::audio(&self.app, &p.label, &self.settings)?; }
            }
            Action::SetQuality { quality } => {
                if !self.host.capabilities(self.settings.demo).media_controls {
                    return Err("Preferred quality is unavailable for full Twitch pages.".into());
                }
                let mut s = self.settings.clone(); s.preferred_quality = quality; self.save(s)?;
                for p in self.players.values_mut() { p.quality_dirty = true; }
            }
            Action::SetWindowMute { session, muted } => self.set_window_mute(session, muted)?,
            Action::SetDemo { demo } => {
                // Idempotent updates must not restart recovery while a token is rotating.
                if demo == self.settings.demo { return Ok(()); }
                if self.mode != Mode::Stopped || !self.players.is_empty() {
                    return Err("Stop and wait for all players to close before changing data sources.".into());
                }
                if demo { self.disconnected(false)?; }
                let mut s = self.settings.clone(); s.demo = demo; self.save(s)?;
                self.presence.clear(); self.skipped.clear(); self.failed.clear(); self.last_check = None;
                if !demo && cfg!(windows) { self.begin_recovery(None, true); }
                self.changed(); self.log(if demo { "Demo data source selected." } else { "Twitch data source selected." });
            }
            Action::DemoLive { login, live } => {
                if !self.settings.demo { return Err("Simulation controls only work in Demo mode.".into()); }
                if !self.settings.favorites.iter().any(|f| f.login == login) { return Err("Unknown favorite.".into()); }
                if live { self.next_id += 1; self.demo_live.insert(login, format!("demo-{}", self.next_id)); }
                else { self.demo_live.remove(&login); }
                self.update_demo();
            }
            Action::LoadDemo => {
                if !self.settings.demo { return Err("Switch to Demo mode first.".into()); }
                let mut s = self.settings.clone();
                for login in ["alpha_demo", "bravo_demo", "charlie_demo", "delta_demo"] {
                    if !s.favorites.iter().any(|f| f.login == login) { s.favorites.push(Favorite { login: login.into(), enabled: true, watch_minutes: None }); }
                }
                self.save(s)?;
                for login in ["bravo_demo", "charlie_demo", "delta_demo"] {
                    self.next_id += 1; self.demo_live.insert(login.into(), format!("demo-{}", self.next_id));
                }
                self.update_demo(); self.log("Demo loaded: Bravo, Charlie, and Delta are simulated live; Alpha is offline.");
            }
            Action::Start => {
                if !self.settings.demo && (self.credentials.is_none() || self.auth_pending) { return Err("Connect Twitch first, or use Demo mode.".into()); }
                if self.mode==Mode::Stopped {
                    if !self.players.is_empty() { return Err("Wait for all previous players to close before starting.".into()); }
                    self.rotation=Default::default();
                }
                self.timer_tick=Instant::now();
                self.mode = Mode::Running; self.changed();
                if self.settings.demo { self.update_demo(); }
                else if self.last_check.is_none_or(|t| t.elapsed() > stale_after(self.settings.rescan_minutes)) { self.mark_stale(); }
                self.log("Monitoring started.");
            }
            Action::Pause => { if self.mode == Mode::Running { self.mode = Mode::Paused; self.log("Automatic selection paused; existing players retained."); } }
            Action::Stop => {
                self.mode = Mode::Stopped; self.rotation=Default::default(); self.generation += 1; self.mark_stale();
                self.next_poll = Instant::now() + Duration::from_secs(60);
                self.log("Stopped; closing all managed players.");
            }
            Action::Refresh => {
                if self.settings.demo { self.update_demo(); } else { self.request_poll(); }
            }
            Action::Connect {} => {
                if self.settings.demo { return Err("Switch to Twitch mode to connect.".into()); }
                crate::viewer_auth::ensure_supported()?;
                if self.auth_pending && self.auth_recovery.is_none() {
                    if let Err(error) = self.open_connection() {
                        let _ = self.disconnected(false);
                        return Err(error);
                    }
                    return Ok(());
                }
                self.disconnected(false)?; self.auth_pending = true; self.error = None;
                let epoch = self.auth_epoch; let tx = self.tx.clone(); let twitch = self.twitch.clone();
                self.auth_task = Some(tokio::spawn(async move {
                    let result = async {
                        let code = twitch.device_code(DEFAULT_CLIENT_ID).await?;
                        let _ = tx.send(Message::AuthCode { epoch, code: code.user_code.clone(), url: code.verification_uri.clone() }).await;
                        twitch.complete_device(DEFAULT_CLIENT_ID, code).await
                    }.await;
                    let _ = tx.send(Message::Authorized { epoch, result }).await;
                }));
            }
            Action::Disconnect => { self.disconnected(true)?; self.log("Monitoring disconnected. API tokens were cleared; Twitch website sign-in is unchanged."); }
            Action::Focus { login } => {
                if let Some(p) = self.players.values().find(|p| p.login == login && !p.closing) {
                    if let Some(w) = self.app.get_webview_window(&p.label) { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
                }
            }
            Action::Skip { login } => self.skip(&login),
            Action::UndoSkip { login } => { self.skipped.remove(&login); }
            Action::Retry { login } => {
                self.failed.remove(&login);
                let ids: Vec<_> = self.players.values().filter(|p| p.login == login && !p.closing).map(|p| p.id).collect();
                for id in ids { self.close(id); }
            }
            Action::ClearError => self.error = None,
        }
        Ok(())
    }
    fn close(&mut self, id: u64) {
        if self.close_retry.get(&id).is_some_and(|t| *t>Instant::now()) { return; }
        if let Some(p) = self.players.get_mut(&id) {
            if p.closing { return; }
            p.closing = true;
            if let Some(w) = self.app.get_webview_window(&p.label) {
                if let Err(error) = w.destroy() {
                    p.closing = false; self.close_retry.insert(id,Instant::now()+Duration::from_secs(30)); self.error = Some(format!("Could not close {}: {error}", p.login));
                }
            }
            // Keep the capacity reservation until Destroyed or a registry sweep.
        }
    }
    fn destroyed(&mut self, label: &str) {
        let id = self.players.values().find(|p| p.label == label).map(|p| p.id);
        if let Some(id) = id {
            self.close_retry.remove(&id);
            if let Some(p) = self.players.remove(&id) {
                if !p.closing && self.mode != Mode::Stopped { self.skip(&p.login); }
                self.log(format!("Closed {} (session {}).", p.login, p.id));
            }
        }
    }
    fn advance_timers(&mut self) {
        let now=Instant::now(); let elapsed=now.saturating_duration_since(self.timer_tick); self.timer_tick=now;
        let assigned=self.players.values().filter(|p| !p.closing).map(|p| p.login.clone()).collect();
        let before: HashSet<_>=self.rotation.turns.iter().filter(|(_,t)| t.overdue()).map(|(l,_)| l.clone()).collect();
        self.rotation.advance(elapsed,self.mode==Mode::Running,&assigned);
        let reached: Vec<_>=self.rotation.turns.iter().filter(|(l,t)| t.overdue() && !before.contains(*l)).map(|(l,_)| l.clone()).collect();
        for login in reached { self.log(format!("Timer reached for {login}; waiting for an eligible live alternative.")); }
    }
    fn reconcile(&mut self) {
        self.advance_timers();
        let ranked: Vec<_> = self.settings.favorites.iter().map(|f| mpd_core::Favorite { login: f.login.clone(), enabled: f.enabled }).collect();
        // Presence freshness is deliberately not part of turn identity: one missed
        // poll and temporary API failures retain assignment time.
        let eligible: Vec<_> = ranked.iter().filter(|f| f.enabled).filter_map(|f| {
            let id=self.presence.get(&f.login)?.broadcast_id.as_ref()?;
            (self.skipped.get(&f.login)!=Some(id)).then(|| (f.login.clone(),id.clone()))
        }).collect();
        self.rotation.observe(eligible.clone());
        self.rotation.turns.retain(|login,turn| eligible.iter().any(|(l,b)| l==login && b==&turn.broadcast));
        for p in self.players.values().filter(|p| !p.closing && self.mode!=Mode::Stopped) {
            if let Some((_,broadcast))=eligible.iter().find(|(l,_)| l==&p.login) {
                let minutes=self.settings.favorites.iter().find(|f| f.login==p.login).and_then(|f| f.watch_minutes);
                self.rotation.opened(&p.login,broadcast,minutes);
            }
        }
        let existing: HashSet<_> = self.players.values().filter(|p| !p.closing).map(|p| p.login.clone()).collect();
        let before=self.rotation.clone();
        let desired = match self.mode {
            Mode::Stopped => vec![],
            Mode::Running => self.rotation.desired(&ranked,&self.presence,&existing,&self.skipped,
                &self.failed.keys().cloned().collect(),self.settings.limit,
                !self.players.values().any(|p| p.closing) && self.close_retry.values().all(|t| *t<=Instant::now())),
            Mode::Paused => ranked.iter().filter(|f| f.enabled && existing.contains(&f.login))
                .filter(|f| {
                    let broadcast = self.presence.get(&f.login).and_then(|p| p.broadcast_id.as_ref());
                    self.skipped.get(&f.login).is_none() || self.skipped.get(&f.login) != broadcast
                }).take(self.settings.limit).map(|f| f.login.clone()).collect(),
        };
        let closing: Vec<_> = self.players.values().filter(|p| !desired.contains(&p.login)).map(|p| p.id).collect();
        for id in closing { self.close(id); }
        if before.pending.is_none() {
            if let Some((source,target))=self.rotation.pending.clone() {
                if self.players.values().any(|p| p.login==source && !p.closing) { self.rotation=before; }
                else { self.log(format!("Timer reached: rotating {source} to {target}.")); }
            }
        }
        if self.mode != Mode::Running || self.players.values().any(|p| p.closing) { return; }
        // Re-evaluate after each failed open, excluding it before truncation. Each
        // login is attempted at most once until explicit Retry; no tick retry storm.
        for _ in 0..=ranked.len() {
            let existing: HashSet<_>=self.players.values().map(|p| p.login.clone()).collect();
            let desired=self.rotation.desired(&ranked,&self.presence,&existing,&self.skipped,
                &self.failed.keys().cloned().collect(),self.settings.limit,false);
            if self.players.len()>=self.settings.limit { break; }
            let Some(login)=desired.into_iter().find(|l| !existing.contains(l)) else { break; };
            self.next_id+=1; let id=self.next_id; let label=self.host.label(id,self.settings.demo);
            match self.host.open(&self.app,&login,id,&self.settings) {
                Ok(_) => {
                    self.advance_timers();
                    self.players.insert(id,PlayerSession {id,login:login.clone(),label,closing:false,
                        report:None,reported:None,rate_start:Instant::now(),rate_count:0,quality_dirty:true,
                        window_mute_override:false,
                        window_muted:self.host.capabilities(self.settings.demo).window_mute_controls.then_some(self.settings.muted)});
                    if let Some(broadcast)=self.presence.get(&login).and_then(|p| p.broadcast_id.as_deref()) {
                        let minutes=self.settings.favorites.iter().find(|f| f.login==login).and_then(|f| f.watch_minutes);
                        self.rotation.opened(&login,broadcast,minutes);
                    }
                    self.log(format!("Opened {login} (session {id}); assignment timer started or resumed."));
                }
                Err(error) => {
                    self.failed.insert(login.clone(),error.clone()); self.error=Some(error);
                    self.log(format!("Could not open {login}; waiting for Retry."));
                    if self.rotation.pending.as_ref().is_some_and(|(_,target)| target==&login) {
                        let source=self.rotation.pending.as_ref().unwrap().0.clone();
                        let candidates: HashSet<_>=mpd_core::select(&ranked,&self.presence,&HashSet::new(),&self.skipped,usize::MAX).into_iter()
                            .filter(|l| !existing.contains(l) && !self.failed.contains_key(l) && !self.rotation.deferred(l)).collect();
                        let order: Vec<_>=ranked.iter().map(|f| f.login.clone()).collect();
                        if let Some(next)=mpd_core::timer::next_waiting(&order,&source,&candidates) { self.rotation.pending=Some((source,next)); }
                        else { self.rotation.failed_target(); }
                    }
                }
            }
        }
    }
    fn begin_recovery(&mut self, credentials: Option<Credentials>, validate: bool) {
        self.auth_pending = true;
        self.auth_recovery = Some(PendingRecovery::new(credentials, validate));
    }
    fn maybe_recover(&mut self) {
        let Some(pending) = self.auth_recovery.as_mut() else { return; };
        if pending.inflight || Instant::now() < pending.next_attempt { return; }
        let scope = match self.credential_store.begin_epoch(self.auth_epoch) {
            Ok(scope) => scope,
            Err(error) => { self.error = Some(error); pending.next_attempt = Instant::now()+Duration::from_secs(30); return; }
        };
        pending.inflight = true;
        let credentials = pending.credentials.take(); let validate = pending.validate;
        let driver = NativeRecovery { twitch: self.twitch.clone(), scope };
        let epoch = self.auth_epoch; let tx = self.tx.clone();
        self.auth_task = Some(tokio::spawn(async move {
            let outcome = recover_authorization(&driver, credentials, validate).await;
            let _ = tx.send(Message::Recovered { epoch, outcome }).await;
        }));
    }
    fn recovered(&mut self, epoch: u64, outcome: RecoveryOutcome<Credentials>) {
        if !accept_recovery(self.auth_epoch, epoch, self.auth_recovery.is_some()) { return; }
        match outcome.result {
            Ok(login) => {
                self.auth_recovery = None; self.auth_pending = false; self.auth_task = None;
                self.connected_as = login;
                self.credentials = outcome.credentials;
                self.error = None; self.not_before = Instant::now(); self.backoff = 30; self.request_poll();
                if self.connected_as.is_some() { self.log("Twitch authorization connected and remembered securely on this Windows account."); }
                self.close_completed_connection();
            }
            Err(error) if error.reconnect => {
                self.error = Some(error.message);
                if let Err(vault_error) = self.disconnected(true) { self.error = Some(vault_error); }
            }
            Err(error) => {
                let pending = self.auth_recovery.as_mut().unwrap();
                pending.credentials = outcome.credentials; pending.inflight = false;
                pending.next_attempt = Instant::now()+error.delay.max(Duration::from_secs(pending.backoff));
                pending.backoff = (pending.backoff*2).min(300);
                self.error = Some(format!("{} Retrying automatically. Cancel, then Connect Twitch starts a new authorization.", error.message));
            }
        }
    }
    fn maybe_poll(&mut self) {
        if self.polling.is_some() || Instant::now() < self.next_poll || Instant::now() < self.not_before { return; }
        let Some(credentials) = self.credentials.clone() else { return; };
        let monitored = !self.settings.demo && self.mode != Mode::Stopped;
        let logins: Vec<_> = if monitored { self.settings.favorites.iter().filter(|f| f.enabled).map(|f| f.login.clone()).collect() } else { vec![] };
        self.poll_id += 1; let id = self.poll_id; self.polling = Some(id);
        let generation = self.generation; let epoch = self.auth_epoch;
        let scope = match self.credential_store.begin_epoch(epoch) {
            Ok(scope) => scope,
            Err(error) => { self.polling = None; self.error = Some(error); return; }
        };
        let tx = self.tx.clone(); let twitch = self.twitch.clone();
        tokio::spawn(async move {
            let result = {
                let mut session = credentials.lock().await;
                if cfg!(windows) { twitch.poll_with_store(&mut session, &logins, &scope).await }
                else { twitch.poll(&mut session, &logins).await }
            };
            let _ = tx.send(Message::Polled { id, generation, epoch, monitored, result }).await;
        });
    }
    fn report(&mut self, label: String, report: Report) {
        let Some(p) = self.players.get_mut(&report.session) else { return; };
        if p.label != label || p.closing { return; }
        if p.rate_start.elapsed() >= Duration::from_secs(1) { p.rate_start = Instant::now(); p.rate_count = 0; }
        if p.rate_count >= 20 { return; }
        p.rate_count += 1;
        p.accept_report(report);
        // Reports are advisory only: NEVER change live status, ranking, or opening policy.
    }
    fn sync_quality(&mut self) {
        for p in self.players.values_mut().filter(|p| !p.closing && p.quality_dirty && p.report.is_some()) {
            // The document reports only after its setter is installed. A new
            // loading/ready report re-arms this after reload; read current settings.
            match player::quality(&self.app, &p.label, &self.settings) {
                Ok(()) => p.quality_dirty = false,
                Err(error) => self.error = Some(format!("Could not apply video quality to {}: {error}", p.login)),
            }
        }
    }
    fn sync_viewer_titles(&self) {
        for p in self.players.values().filter(|p| !p.closing) {
            let enabled = self.settings.favorites.iter().any(|f| f.login == p.login && f.enabled);
            let count = ViewerCount::from_presence(self.presence.get(&p.login), enabled, self.settings.demo);
            if let Some(window) = self.app.get_webview_window(&p.label) {
                let title = player::window_title(&p.login, self.settings.demo, count);
                // Hosted pages may change document.title after loading. Compare the
                // actual native title on each controller tick, rather than caching it.
                if window.title().ok().as_deref() != Some(title.as_str()) {
                    let _ = window.set_title(&title);
                }
            }
        }
    }
    fn snapshot(&self) -> View {
        let mut players: Vec<_> = self.players.values().map(|p| PlayerView {
            timer: if p.closing && self.rotation.pending.as_ref().is_some_and(|(source,_)| source==&p.login) { TimerView::ClosingForRotation }
                else { match self.rotation.turns.get(&p.login).and_then(|turn| turn.remaining()) {
                    None => TimerView::Unlimited,
                    Some(remaining) if remaining.is_zero() => TimerView::WaitingForAlternative,
                    Some(remaining) => { let remaining_seconds=remaining.as_secs()+u64::from(remaining.subsec_nanos()>0);
                        if self.mode==Mode::Paused { TimerView::AutomationPaused {remaining_seconds} }
                        else { TimerView::Counting {remaining_seconds} }
                    }
                } },
            login: p.login.clone(), session: p.id, closing: p.closing,
            state: p.report.as_ref().map_or(Playback::Loading, |r| r.state),
            report_age_seconds: p.reported.map(|t| t.elapsed().as_secs()),
            visible: p.report.as_ref().map(|r| r.visible), volume: p.report.as_ref().and_then(|r| r.volume),
            muted: p.report.as_ref().and_then(|r| r.muted),
            window_muted: p.window_muted,
            window_mute_override: p.window_muted.map(|_| p.window_mute_override),
        }).collect();
        players.sort_by_key(|p| self.settings.favorites.iter().position(|f| f.login == p.login).unwrap_or(usize::MAX));
        View { mode: self.mode, settings: self.settings.clone(),
            favorites: self.settings.favorites.iter().map(|f| {
                let p = self.presence.get(&f.login);
                let presence = match p {
                    None => "unknown",
                    Some(p) if !p.fresh => "stale",
                    Some(p) if p.broadcast_id.is_some() && p.missing_polls > 0 => "checking offline",
                    Some(p) if p.broadcast_id.is_some() => "live",
                    Some(_) => "offline",
                };
                FavoriteView { watch_minutes: f.watch_minutes, login: f.login.clone(), enabled: f.enabled, presence: presence.into(),
                    viewer_count: ViewerCount::from_presence(p, f.enabled, self.settings.demo),
                    skipped: self.skipped.get(&f.login).is_some_and(|id| p.and_then(|p| p.broadcast_id.as_ref()) == Some(id)),
                    demo_live: self.demo_live.contains_key(&f.login), open_error: self.failed.get(&f.login).cloned() }
            }).collect(), players, connected_as: self.connected_as.clone(), auth_pending: self.auth_pending,
            user_code: self.user_code.clone(), last_check_seconds: self.last_check.map(|t| t.elapsed().as_secs()),
            polling: self.polling.is_some(), next_check_seconds: self.next_poll.saturating_duration_since(Instant::now()).as_secs(),
            viewer: self.host.capabilities(self.settings.demo),
            player_origin: self.host.diagnostic_origin(self.settings.demo),
            error: self.error.clone(), events: self.events.iter().cloned().collect() }
    }
    pub async fn run(mut self, mut rx: mpsc::Receiver<Message>) {
        self.log("Ready. Playback starts only when you press Start.");
        #[cfg(feature = "e2e-tests")]
        if crate::e2e::scenario() == "web" {
            self.credentials = Some(Arc::new(Mutex::new(crate::e2e::session())));
            self.connected_as = Some("fixture_user".into());
        }
        if cfg!(windows) && !self.settings.demo && self.credentials.is_none() { self.begin_recovery(None, true); }
        self.publish.send_replace(self.snapshot());
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                message = rx.recv() => match message {
                    None => break,
                    Some(Message::Action(action, reply)) => {
                        let result = self.action(action);
                        if let Err(error) = &result { self.error = Some(error.clone()); }
                        let _ = reply.send(result);
                    }
                    Some(Message::Report(label, report)) => self.report(label, report),
                    Some(Message::Destroyed(label)) => self.destroyed(&label),
                    Some(Message::AuthCode { epoch, code, url }) => {
                        if epoch == self.auth_epoch && self.auth_pending {
                            self.auth_window_epoch = Some(epoch);
                            match crate::viewer_auth::Activation::parse(&url, &code)
                                .and_then(|activation| crate::viewer_auth::open(&self.app, epoch, activation)) {
                                Ok(()) => { self.user_code = Some(code); self.auth_url = Some(url); }
                                Err(error) => { let _ = self.disconnected(false); self.error = Some(error); }
                            }
                        }
                    }
                    Some(Message::AuthReturnLoaded { epoch }) => {
                        if epoch == self.auth_epoch && self.auth_window_epoch == Some(epoch) {
                            self.auth_return_epoch = Some(epoch);
                            self.close_completed_connection();
                        }
                    }
                    Some(Message::Authorized { epoch, result }) => {
                        if epoch == self.auth_epoch && self.auth_pending {
                            self.auth_pending = false; self.user_code = None; self.auth_url = None;
                            match result {
                                Ok(session) => {
                                    let credentials = Arc::new(Mutex::new(session));
                                    if cfg!(windows) { self.begin_recovery(Some(credentials), false); }
                                    else {
                                        self.connected_as = Some(credentials.lock().await.login.clone()); self.credentials = Some(credentials);
                                        self.error = Some("This platform keeps Twitch authorization only for this run; secure persistence is currently available on Windows.".into());
                                        self.request_poll(); self.log("Monitoring connected for this run. Verify your account in the Twitch window.");
                                        self.close_completed_connection();
                                    }
                                }
                                Err(error) => { let _ = self.disconnected(false); self.error = Some(error.message); },
                            }
                        }
                    }
                    Some(Message::Recovered { epoch, outcome }) => self.recovered(epoch, outcome),
                    Some(Message::Polled { id, generation, epoch, monitored, result }) => {
                        if self.polling == Some(id) { self.polling = None; }
                        if epoch == self.auth_epoch {
                            match result {
                                Ok(online) => {
                                    self.backoff = 30;
                                    if generation == self.generation {
                                        self.next_poll = Instant::now() + if monitored {
                                            monitored_poll_interval(self.settings.rescan_minutes)
                                        } else { Duration::from_secs(60) };
                                        if monitored && self.mode != Mode::Stopped && !self.settings.demo {
                                            for f in self.settings.favorites.iter().filter(|f| f.enabled) {
                                                let stream = online.get(&f.login);
                                                self.presence.entry(f.login.clone()).or_default().observe_stream(
                                                    stream.map(|s| s.id.as_str()), stream.and_then(|s| s.viewer_count));
                                            }
                                            self.last_check = Some(Instant::now());
                                        }
                                    } else { self.request_poll(); }
                                }
                                Err(error) => {
                                    self.mark_stale(); self.error = Some(error.message.clone());
                                    let delay = error.retry_after.max(Duration::from_secs(self.backoff));
                                    self.not_before = Instant::now() + delay; self.next_poll = self.not_before;
                                    self.backoff = (self.backoff * 2).min(300);
                                    if error.reconnect {
                                        if let Err(vault_error) = self.disconnected(true) { self.error = Some(vault_error); }
                                    }
                                }
                            }
                        }
                    }
                },
                _ = tick.tick() => {
                    // Lost window events cannot leak capacity reservations indefinitely.
                    let gone: Vec<_> = self.players.values().filter(|p| self.app.get_webview_window(&p.label).is_none()).map(|p| p.label.clone()).collect();
                    for label in gone { self.destroyed(&label); }
                    if !self.settings.demo && self.last_check.is_some_and(|t| t.elapsed() > stale_after(self.settings.rescan_minutes)) { self.mark_stale(); }
                    // Native title work stays bounded to this timer, not player reports.
                    self.sync_viewer_titles();
                    self.sync_quality();
                }
            }
            self.reconcile(); self.maybe_recover(); self.maybe_poll(); self.publish.send_replace(self.snapshot());
        }
    }
}

#[cfg(test)]
mod poll_schedule_tests {
    use super::{monitored_poll_interval, rescheduled_poll, scheduled_poll, stale_after};
    use std::time::{Duration,Instant};

    #[test]
    fn configured_intervals_and_stale_tolerance_cover_bounds() {
        assert_eq!(monitored_poll_interval(1),Duration::from_secs(60));
        assert_eq!(monitored_poll_interval(60),Duration::from_secs(3600));
        assert_eq!(stale_after(1),Duration::from_secs(180));
        assert_eq!(stale_after(60),Duration::from_secs(10800));
    }

    #[test]
    fn rescheduling_handles_exact_shorter_longer_missing_and_backoff_deadlines() {
        let base=Instant::now();
        let now=base+Duration::from_secs(120);
        assert_eq!(scheduled_poll(now,Some(base),base,2),now);
        assert_eq!(scheduled_poll(now,Some(base),base,1),now);
        assert_eq!(scheduled_poll(now,Some(base),base,10),base+Duration::from_secs(600));
        assert_eq!(scheduled_poll(now,None,base,10),now);
        let backoff=base+Duration::from_secs(700);
        assert_eq!(scheduled_poll(now,Some(base),backoff,10),backoff);
    }

    #[test]
    fn interval_change_never_postpones_a_retry_already_queued_at_backoff() {
        let base=Instant::now();let now=base+Duration::from_secs(120);
        let backoff=base+Duration::from_secs(700);
        assert_eq!(rescheduled_poll(now,Some(base),backoff,backoff,60),backoff);
        let ordinary=base+Duration::from_secs(180);
        assert_eq!(rescheduled_poll(now,Some(base),base,ordinary,10),base+Duration::from_secs(600));
    }
}

#[cfg(test)]
mod auth_cleanup_tests {
    use super::{close_tracked_auth_window, auth_completion_ready};
    #[test]
    fn auto_close_requires_both_current_attempt_completions_in_either_order() {
        assert!(!auth_completion_ready(7, Some(7), None, true, false));
        // Return page first: still wait for successful API authorization.
        assert!(!auth_completion_ready(7, Some(7), Some(7), true, false));
        assert!(auth_completion_ready(7, Some(7), Some(7), false, true));
        // API first: still wait for the exact return page to finish loading.
        assert!(!auth_completion_ready(7, Some(7), None, false, true));
        assert!(auth_completion_ready(7, Some(7), Some(7), false, true));
        assert!(!auth_completion_ready(7, Some(7), Some(6), false, true));
        assert!(!auth_completion_ready(7, Some(6), Some(7), false, true));
        assert!(!auth_completion_ready(7, Some(7), Some(7), false, false));
        assert!(!auth_completion_ready(7, None, Some(7), false, true));
        assert!(!auth_completion_ready(8, Some(8), Some(7), true, false));
    }
    #[test]
    fn failed_close_retains_the_original_window_for_retry() {
        let mut tracked=Some(7);
        assert!(close_tracked_auth_window(&mut tracked, |epoch| {
            assert_eq!(epoch,7); Err("simulated close failure".into())
        }).is_err());
        assert_eq!(tracked,Some(7));
        close_tracked_auth_window(&mut tracked, |epoch| { assert_eq!(epoch,7); Ok(()) }).unwrap();
        assert_eq!(tracked,None);
        close_tracked_auth_window(&mut tracked, |_| panic!("Already cleaned up")).unwrap();
    }
}

#[cfg(test)]
mod quality_sync_tests {
    use super::*;
    #[test]
    fn new_document_readiness_rearms_quality_sync_without_telemetry_flooding() {
        let mut p = PlayerSession { id: 1, login: "alpha".into(), label: "player-1".into(),
            closing: false, report: None, reported: None, rate_start: Instant::now(), rate_count: 0, quality_dirty: true,
            window_mute_override: false, window_muted: None };
        let report = |state| Report { session: 1, state, visible: true, volume: None, muted: None };
        // An early preference waits for a report from the initialized document.
        assert!(p.quality_dirty && p.report.is_none());
        p.accept_report(report(Playback::Ready));
        assert!(p.quality_dirty && p.report.is_some());
        p.quality_dirty = false; // successful native delivery
        p.accept_report(report(Playback::Playing));
        assert!(!p.quality_dirty);
        p.accept_report(report(Playback::Paused));
        assert!(!p.quality_dirty);
        // Reloading the same window creates another READY with its old bootstrap.
        p.accept_report(report(Playback::Ready));
        assert!(p.quality_dirty);
    }
}

#[cfg(test)]
mod window_mute_tests {
    use super::*;
    fn target(id: u64, before: bool, after: bool) -> MuteTarget {
        MuteTarget { id, label: format!("twitch-page-{id}"), before, after }
    }
    #[test]
    fn global_change_confirms_every_target() {
        let targets = [target(1, false, true), target(2, true, true)];
        let mut calls = Vec::new();
        let result = apply_mute_transaction(&targets, |label, muted| { calls.push((label.to_owned(), muted)); Ok(()) });
        assert!(result.error.is_none());
        assert_eq!(result.confirmed, HashMap::from([(1, true), (2, true)]));
        assert_eq!(calls, [("twitch-page-1".into(), true), ("twitch-page-2".into(), true)]);
    }
    #[test]
    fn one_failure_rolls_back_changed_windows_and_never_claims_target_state() {
        let targets = [target(1, false, true), target(2, false, true), target(3, true, true)];
        let mut calls = Vec::new();
        let result = apply_mute_transaction(&targets, |label, muted| {
            calls.push((label.to_owned(), muted));
            if label == "twitch-page-2" { Err("simulated setter failure".into()) } else { Ok(()) }
        });
        assert!(result.error.as_deref().is_some_and(|error| error.contains("session 2")));
        assert_eq!(result.confirmed, HashMap::from([(1, false), (2, false), (3, true)]));
        assert_eq!(calls, [("twitch-page-1".into(), true), ("twitch-page-2".into(), true), ("twitch-page-1".into(), false)]);
    }
    #[test]
    fn rollback_failure_retains_last_confirmed_native_value_and_is_reported() {
        let targets = [target(1, false, true), target(2, false, true)];
        let mut first = true;
        let result = apply_mute_transaction(&targets, |label, muted| {
            if label == "twitch-page-1" && muted && first { first = false; return Ok(()); }
            Err("simulated failure".into())
        });
        assert_eq!(result.confirmed, HashMap::from([(1, true), (2, false)]));
        let error = result.error.unwrap();
        assert!(error.contains("session 2") && error.contains("Rollback also failed for sessions 1"));
    }
    #[test]
    fn storage_failure_rolls_every_window_back_before_reporting_failure() {
        let targets = [target(1, false, true), target(2, false, true)];
        let mut calls = Vec::new();
        let result = apply_persisted_mute_transaction(&targets,
            |label, muted| { calls.push((label.to_owned(), muted)); Ok(()) },
            || Err("simulated storage failure".into()));
        assert_eq!(result.confirmed, HashMap::from([(1, false), (2, false)]));
        assert!(result.error.as_deref().is_some_and(|error| error.contains("simulated storage failure")));
        assert_eq!(calls, [
            ("twitch-page-1".into(), true), ("twitch-page-2".into(), true),
            ("twitch-page-2".into(), false), ("twitch-page-1".into(), false),
        ]);
    }
    #[test]
    fn storage_rollback_keeps_each_last_confirmed_result_when_one_restore_fails() {
        let targets = [target(1, false, true), target(2, false, true)];
        let mut applied = false;
        let result = apply_persisted_mute_transaction(&targets,
            |label, muted| {
                if !muted && label == "twitch-page-2" { return Err("simulated restore failure".into()); }
                applied = true; Ok(())
            },
            || Err("simulated storage failure".into()));
        assert!(applied);
        assert_eq!(result.confirmed, HashMap::from([(1, false), (2, true)]));
        assert!(result.error.as_deref().is_some_and(|error| error.contains("Rollback also failed for sessions 2")));
    }
    fn session(id: u64, closing: bool, window_muted: Option<bool>) -> PlayerSession {
        PlayerSession { id, login: "alpha".into(), label: format!("twitch-page-{id}"), closing,
            report: None, reported: None, rate_start: Instant::now(), rate_count: 0, quality_dirty: true,
            window_mute_override: false, window_muted }
    }
    #[test]
    fn session_validation_rejects_stale_closing_and_unsupported_targets() {
        let mut players = HashMap::new();
        assert!(session_mute_label(&players, 7).unwrap_err().contains("no longer active"));
        players.insert(7, session(7, true, Some(false)));
        assert!(session_mute_label(&players, 7).unwrap_err().contains("closing"));
        players.insert(7, session(7, false, None));
        assert!(session_mute_label(&players, 7).unwrap_err().contains("does not support"));
        players.insert(7, session(7, false, Some(false)));
        assert_eq!(session_mute_label(&players, 7).unwrap(), "twitch-page-7");
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use std::sync::{Mutex as StdMutex, atomic::{AtomicUsize, Ordering}};
    struct FakeRecovery {
        stored: bool, loads: AtomicUsize, calls: AtomicUsize,
        fail: StdMutex<Option<bool>>, phases: StdMutex<Vec<bool>>,
    }
    impl FakeRecovery {
        fn new(stored: bool, failure: Option<bool>) -> Self {
            Self { stored, loads: AtomicUsize::new(0), calls: AtomicUsize::new(0), fail: StdMutex::new(failure), phases: StdMutex::new(vec![]) }
        }
    }
    impl RecoveryDriver for FakeRecovery {
        type Credentials = Arc<AtomicUsize>;
        fn load(&self) -> Result<Option<Self::Credentials>, RecoveryError> {
            self.loads.fetch_add(1,Ordering::SeqCst);
            Ok(self.stored.then(|| Arc::new(AtomicUsize::new(0))))
        }
        async fn recover(&self, credentials: &mut Self::Credentials, validate: bool) -> Result<String,RecoveryError> {
            self.calls.fetch_add(1,Ordering::SeqCst);
            self.phases.lock().unwrap().push(validate);
            // Stand in for rotation: the caller must retain this exact allocation.
            credentials.fetch_add(1,Ordering::SeqCst);
            tokio::task::yield_now().await;
            if let Some(reconnect)=self.fail.lock().unwrap().take() {
                Err(RecoveryError { message:"simulated authorization failure".into(),reconnect,delay:Duration::from_secs(30) })
            } else { Ok("tester".into()) }
        }
    }
    #[tokio::test]
    async fn startup_without_saved_entry_does_not_validate_or_request_login() {
        let driver=FakeRecovery::new(false,None);
        let outcome=recover_authorization(&driver,None,true).await;
        assert!(matches!(outcome.result,Ok(None)));
        assert!(outcome.credentials.is_none());
        assert_eq!(driver.loads.load(Ordering::SeqCst),1);
        assert_eq!(driver.calls.load(Ordering::SeqCst),0);
    }
    #[tokio::test]
    async fn transient_restore_retains_rotated_session_and_retries_without_reloading() {
        let driver=FakeRecovery::new(true,Some(false));
        let first=recover_authorization(&driver,None,true).await;
        assert!(matches!(first.result,Err(RecoveryError { reconnect:false,.. })));
        let retained=first.credentials.as_ref().unwrap().clone();
        assert_eq!(retained.load(Ordering::SeqCst),1);
        let second=recover_authorization(&driver,first.credentials,true).await;
        assert!(matches!(second.result,Ok(Some(_))));
        assert!(Arc::ptr_eq(&retained,second.credentials.as_ref().unwrap()));
        assert_eq!(driver.loads.load(Ordering::SeqCst),1);
        assert_eq!(retained.load(Ordering::SeqCst),2);
    }
    #[tokio::test]
    async fn initial_save_failure_retries_new_credentials_instead_of_loading_old_account() {
        let driver=FakeRecovery::new(true,Some(false));
        let current=Arc::new(AtomicUsize::new(10));
        let first=recover_authorization(&driver,Some(current.clone()),false).await;
        assert!(first.result.is_err());
        let second=recover_authorization(&driver,first.credentials,false).await;
        assert!(second.result.is_ok());
        assert!(Arc::ptr_eq(&current,second.credentials.as_ref().unwrap()));
        assert_eq!(driver.loads.load(Ordering::SeqCst),0);
        assert_eq!(*driver.phases.lock().unwrap(),vec![false,false]);
    }
    #[tokio::test]
    async fn revocation_result_is_terminal_and_late_epoch_cannot_be_accepted() {
        let driver=FakeRecovery::new(true,Some(true));
        let outcome=recover_authorization(&driver,None,true).await;
        assert!(matches!(outcome.result,Err(RecoveryError { reconnect:true,.. })));
        assert!(!accept_recovery(2,1,true));
        assert!(!accept_recovery(2,2,false));
        assert!(accept_recovery(2,2,true));
    }
}
