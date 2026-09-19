//! Single-owner controller. Network results are generation-tagged; native window
//! destruction is acknowledged before replacement windows consume capacity.
use std::{collections::{HashMap, HashSet, VecDeque}, sync::Arc, time::{Duration, Instant}};
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use crate::{model::*, player::{self, Host}, storage::Store};
use mpd_core::Presence;
use mpd_twitch::{ApiError, Session, Twitch};

type Credentials = Arc<Mutex<Session>>;
pub enum Message {
    Action(Action, oneshot::Sender<Result<(), String>>),
    Report(String, Report),
    Destroyed(String),
    AuthCode { epoch: u64, code: String, url: String },
    Authorized { epoch: u64, result: Result<Session, ApiError> },
    Polled { id: u64, generation: u64, epoch: u64, monitored: bool,
        result: Result<HashMap<String, String>, ApiError> },
}
#[derive(Clone)]
pub struct Handle {
    pub tx: mpsc::Sender<Message>,
    pub view: watch::Receiver<View>,
}
struct PlayerSession {
    id: u64, login: String, label: String, closing: bool,
    report: Option<Report>, reported: Option<Instant>,
    rate_start: Instant, rate_count: u8,
}

pub struct Controller {
    app: AppHandle, tx: mpsc::Sender<Message>, publish: watch::Sender<View>,
    store: Store, host: Host, twitch: Twitch, settings: Settings, mode: Mode,
    presence: HashMap<String, Presence>, skipped: HashMap<String, String>,
    demo_live: HashMap<String, String>, players: HashMap<u64, PlayerSession>,
    failed: HashMap<String, String>, next_id: u64, generation: u64, auth_epoch: u64,
    credentials: Option<Credentials>, connected_as: Option<String>,
    auth_task: Option<tokio::task::JoinHandle<()>>, auth_pending: bool,
    user_code: Option<String>, auth_url: Option<String>,
    polling: Option<u64>, poll_id: u64, next_poll: Instant, not_before: Instant,
    last_check: Option<Instant>, backoff: u64, error: Option<String>,
    events: VecDeque<String>, started: Instant,
}
impl Controller {
    pub fn new(app: AppHandle, tx: mpsc::Sender<Message>, publish: watch::Sender<View>,
        store: Store, host: Host, settings: Settings, twitch: Twitch) -> Self {
        Self { app, tx, publish, store, host, twitch, settings, mode: Mode::Stopped,
            presence: HashMap::new(), skipped: HashMap::new(), demo_live: HashMap::new(),
            players: HashMap::new(), failed: HashMap::new(), next_id: 0, generation: 0,
            auth_epoch: 0, credentials: None, connected_as: None, auth_task: None,
            auth_pending: false, user_code: None, auth_url: None, polling: None,
            poll_id: 0, next_poll: Instant::now(), not_before: Instant::now(),
            last_check: None, backoff: 30, error: None, events: VecDeque::new(), started: Instant::now() }
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
    fn disconnected(&mut self) {
        self.auth_epoch += 1;
        if let Some(task) = self.auth_task.take() { task.abort(); }
        self.auth_pending = false; self.credentials = None; self.connected_as = None;
        self.user_code = None; self.auth_url = None; self.mark_stale();
    }
    fn action(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::Add { input } => {
                if input.len() > 512 { return Err("Channel input is too long.".into()); }
                let login = mpd_core::normalize_login(&input).map_err(str::to_owned)?;
                if self.settings.favorites.iter().any(|f| f.login == login) { return Err("That channel is already a favorite.".into()); }
                let mut s = self.settings.clone(); s.favorites.push(Favorite { login: login.clone(), enabled: true });
                self.save(s)?; self.presence.insert(login.clone(), Presence::default());
                self.changed(); self.log(format!("Added {login}."));
            }
            Action::Remove { login } => {
                let mut s = self.settings.clone(); s.favorites.retain(|f| f.login != login); self.save(s)?;
                self.presence.remove(&login); self.skipped.remove(&login); self.demo_live.remove(&login);
                self.failed.remove(&login); self.changed();
            }
            Action::Move { login, position } => {
                let mut s = self.settings.clone();
                let index = s.favorites.iter().position(|f| f.login == login).ok_or("Unknown favorite.")?;
                let favorite = s.favorites.remove(index);
                s.favorites.insert(position.min(s.favorites.len()), favorite); self.save(s)?;
            }
            Action::Enable { login, enabled } => {
                let mut s = self.settings.clone();
                s.favorites.iter_mut().find(|f| f.login == login).ok_or("Unknown favorite.")?.enabled = enabled;
                self.save(s)?; self.changed();
            }
            Action::SetLimit { limit } => { let mut s = self.settings.clone(); s.limit = limit; self.save(s)?; }
            Action::SetAudio { volume, muted } => {
                let mut s = self.settings.clone(); s.volume = volume; s.muted = muted; self.save(s)?;
                for p in self.players.values() { player::audio(&self.app, &p.label, &self.settings)?; }
            }
            Action::SetDemo { demo } => {
                if self.mode != Mode::Stopped || !self.players.is_empty() {
                    return Err("Stop and wait for all players to close before changing data sources.".into());
                }
                let mut s = self.settings.clone(); s.demo = demo; self.save(s)?;
                self.presence.clear(); self.skipped.clear(); self.failed.clear(); self.last_check = None;
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
                    if !s.favorites.iter().any(|f| f.login == login) { s.favorites.push(Favorite { login: login.into(), enabled: true }); }
                }
                self.save(s)?;
                for login in ["bravo_demo", "charlie_demo", "delta_demo"] {
                    self.next_id += 1; self.demo_live.insert(login.into(), format!("demo-{}", self.next_id));
                }
                self.update_demo(); self.log("Demo loaded: Bravo, Charlie, and Delta are simulated live; Alpha is offline.");
            }
            Action::Start => {
                if !self.settings.demo && self.credentials.is_none() { return Err("Authorize monitoring first, or use Demo mode.".into()); }
                self.mode = Mode::Running; self.changed();
                if self.settings.demo { self.update_demo(); }
                else if self.last_check.map_or(true, |t| t.elapsed() > Duration::from_secs(90)) { self.mark_stale(); }
                self.log("Monitoring started.");
            }
            Action::Pause => { if self.mode == Mode::Running { self.mode = Mode::Paused; self.log("Automatic selection paused; existing players retained."); } }
            Action::Stop => {
                self.mode = Mode::Stopped; self.generation += 1; self.mark_stale();
                self.next_poll = Instant::now() + Duration::from_secs(3600);
                self.log("Stopped; closing all managed players.");
            }
            Action::Refresh => {
                if self.settings.demo { self.update_demo(); } else { self.request_poll(); }
            }
            Action::Connect { client_id } => {
                let client_id = client_id.trim().to_owned();
                if client_id.is_empty() || client_id.len() > 128 || !client_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    return Err("Enter the Client ID from your Twitch Public application.".into());
                }
                let mut s = self.settings.clone(); s.client_id = client_id.clone(); self.save(s)?;
                self.disconnected(); self.auth_pending = true;
                let epoch = self.auth_epoch; let tx = self.tx.clone(); let twitch = self.twitch.clone();
                self.auth_task = Some(tokio::spawn(async move {
                    let result = async {
                        let code = twitch.device_code(&client_id).await?;
                        let _ = tx.send(Message::AuthCode { epoch, code: code.user_code.clone(), url: code.verification_uri.clone() }).await;
                        twitch.complete_device(&client_id, code).await
                    }.await;
                    let _ = tx.send(Message::Authorized { epoch, result }).await;
                }));
            }
            Action::Disconnect => { self.disconnected(); self.log("Monitoring disconnected. API tokens were cleared; Twitch website sign-in is unchanged."); }
            Action::OpenViewerLogin => {
                if self.settings.demo { return Err("Switch to Twitch mode to sign in for viewing.".into()); }
                crate::viewer_auth::open(&self.app)?;
                self.log("Opened Twitch website sign-in. Verify your account in Twitch; API monitoring is separate.");
            }
            Action::OpenAuth => {
                let text = self.auth_url.as_ref().ok_or("No authorization is pending.")?;
                let url = url::Url::parse(text).map_err(|_| "Invalid Twitch authorization URL.")?;
                if url.scheme() != "https" || !matches!(url.host_str(), Some("www.twitch.tv" | "twitch.tv"))
                    || url.path() != "/activate" || !url.username().is_empty() || url.password().is_some() {
                    return Err("Rejected unexpected authorization URL.".into());
                }
                webbrowser::open(url.as_str()).map_err(|_| "Could not open your browser. Visit Twitch's activation page manually.")?;
            }
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
        if let Some(p) = self.players.get_mut(&id) {
            if p.closing { return; }
            p.closing = true;
            if let Some(w) = self.app.get_webview_window(&p.label) {
                if let Err(error) = w.destroy() {
                    p.closing = false; self.error = Some(format!("Could not close {}: {error}", p.login));
                }
            }
            // Keep the capacity reservation until Destroyed or a registry sweep.
        }
    }
    fn destroyed(&mut self, label: &str) {
        let id = self.players.values().find(|p| p.label == label).map(|p| p.id);
        if let Some(id) = id {
            if let Some(p) = self.players.remove(&id) {
                if !p.closing && self.mode != Mode::Stopped { self.skip(&p.login); }
                self.log(format!("Closed {} (session {}).", p.login, p.id));
            }
        }
    }
    fn reconcile(&mut self) {
        let ranked: Vec<_> = self.settings.favorites.iter().map(|f| mpd_core::Favorite { login: f.login.clone(), enabled: f.enabled }).collect();
        let existing: HashSet<_> = self.players.values().filter(|p| !p.closing).map(|p| p.login.clone()).collect();
        let desired = match self.mode {
            Mode::Stopped => vec![],
            Mode::Running => mpd_core::select(&ranked, &self.presence, &existing, &self.skipped, self.settings.limit),
            Mode::Paused => ranked.iter().filter(|f| f.enabled && existing.contains(&f.login))
                .filter(|f| {
                    let broadcast = self.presence.get(&f.login).and_then(|p| p.broadcast_id.as_ref());
                    self.skipped.get(&f.login).is_none() || self.skipped.get(&f.login) != broadcast
                }).take(self.settings.limit).map(|f| f.login.clone()).collect(),
        };
        let closing: Vec<_> = self.players.values().filter(|p| !desired.contains(&p.login)).map(|p| p.id).collect();
        for id in closing { self.close(id); }
        if self.mode != Mode::Running || self.players.values().any(|p| p.closing) { return; }
        for login in desired {
            if self.players.len() >= self.settings.limit { break; }
            if self.players.values().any(|p| p.login == login) || self.failed.contains_key(&login) { continue; }
            self.next_id += 1; let id = self.next_id;
            self.players.insert(id, PlayerSession { id, login: login.clone(), label: format!("player-{id}"),
                closing: false, report: None, reported: None, rate_start: Instant::now(), rate_count: 0 });
            match self.host.open(&self.app, &login, id, &self.settings) {
                Ok(_) => self.log(format!("Opened {login} (session {id}).")),
                Err(error) => { self.players.remove(&id); self.failed.insert(login, error.clone()); self.error = Some(error); }
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
        let tx = self.tx.clone(); let twitch = self.twitch.clone();
        tokio::spawn(async move {
            let result = {
                let mut session = credentials.lock().await;
                twitch.poll(&mut session, &logins).await
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
        p.reported = Some(Instant::now()); p.report = Some(report);
        // Reports are advisory only: NEVER change live status, ranking, or opening policy.
    }
    fn snapshot(&self) -> View {
        let mut players: Vec<_> = self.players.values().map(|p| PlayerView {
            login: p.login.clone(), session: p.id, closing: p.closing,
            state: p.report.as_ref().map_or(Playback::Loading, |r| r.state),
            report_age_seconds: p.reported.map(|t| t.elapsed().as_secs()),
            visible: p.report.as_ref().map(|r| r.visible), volume: p.report.as_ref().and_then(|r| r.volume),
            muted: p.report.as_ref().and_then(|r| r.muted),
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
                FavoriteView { login: f.login.clone(), enabled: f.enabled, presence: presence.into(),
                    skipped: self.skipped.get(&f.login).is_some_and(|id| p.and_then(|p| p.broadcast_id.as_ref()) == Some(id)),
                    demo_live: self.demo_live.contains_key(&f.login), open_error: self.failed.get(&f.login).cloned() }
            }).collect(), players, connected_as: self.connected_as.clone(), auth_pending: self.auth_pending,
            user_code: self.user_code.clone(), last_check_seconds: self.last_check.map(|t| t.elapsed().as_secs()),
            polling: self.polling.is_some(), next_check_seconds: self.next_poll.saturating_duration_since(Instant::now()).as_secs(),
            player_origin: self.host.base(self.settings.demo).as_str().to_owned(),
            error: self.error.clone(), events: self.events.iter().cloned().collect() }
    }
    pub async fn run(mut self, mut rx: mpsc::Receiver<Message>) {
        self.log("Ready. Playback starts only when you press Start.");
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
                        if epoch == self.auth_epoch && code.len() <= 32 && url.len() <= 1024 {
                            self.user_code = Some(code); self.auth_url = Some(url);
                        }
                    }
                    Some(Message::Authorized { epoch, result }) => {
                        if epoch == self.auth_epoch {
                            self.auth_pending = false; self.user_code = None; self.auth_url = None;
                            match result {
                                Ok(session) => { self.connected_as = Some(session.login.clone()); self.credentials = Some(Arc::new(Mutex::new(session)));
                                    self.error = None; self.request_poll(); self.log("Monitoring authorized. API tokens stay in memory; viewer website sign-in is separate."); }
                                Err(error) => self.error = Some(error.message),
                            }
                        }
                    }
                    Some(Message::Polled { id, generation, epoch, monitored, result }) => {
                        if self.polling == Some(id) { self.polling = None; }
                        if epoch == self.auth_epoch {
                            match result {
                                Ok(online) => {
                                    self.backoff = 30;
                                    if generation == self.generation {
                                        self.next_poll = Instant::now() + Duration::from_secs(if monitored { 30 } else { 3600 });
                                        if monitored && self.mode != Mode::Stopped && !self.settings.demo {
                                            for f in self.settings.favorites.iter().filter(|f| f.enabled) {
                                                self.presence.entry(f.login.clone()).or_default().observe(online.get(&f.login).map(String::as_str));
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
                                    if error.reconnect { self.disconnected(); }
                                }
                            }
                        }
                    }
                },
                _ = tick.tick() => {
                    // Lost window events cannot leak capacity reservations indefinitely.
                    let gone: Vec<_> = self.players.values().filter(|p| self.app.get_webview_window(&p.label).is_none()).map(|p| p.label.clone()).collect();
                    for label in gone { self.destroyed(&label); }
                    if !self.settings.demo && self.last_check.is_some_and(|t| t.elapsed() > Duration::from_secs(90)) { self.mark_stale(); }
                }
            }
            self.reconcile(); self.maybe_poll(); self.publish.send_replace(self.snapshot());
        }
    }
}
