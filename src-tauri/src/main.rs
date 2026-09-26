#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(test)]
mod http_pool_regression;
mod controller;
mod credential_store;
mod viewer_mode;
mod model;
mod player;
mod window_audio;
mod storage;
mod viewer_auth;
#[cfg(feature = "e2e-tests")]
mod e2e;
#[cfg(feature = "e2e-tests")]
mod e2e_probe;

use controller::{Controller, Handle, Message};
use model::{Action, Report, View};
use tauri::{Manager, State, WebviewWindow, WindowEvent};
use tokio::sync::{mpsc, oneshot, watch};

#[tauri::command]
fn get_state(window: WebviewWindow, state: State<'_, Handle>) -> Result<View, String> {
    if window.label() != "main" { return Err("Manager command denied.".into()); }
    let view = state.view.borrow().clone();
    Ok(view)
}

#[tauri::command]
async fn dispatch(window: WebviewWindow, state: State<'_, Handle>, action: Action) -> Result<(), String> {
    if window.label() != "main" { return Err("Manager command denied.".into()); }
    let (tx, rx) = oneshot::channel();
    state.tx.send(Message::Action(action, tx)).await.map_err(|_| "Application is shutting down.")?;
    rx.await.map_err(|_| "Controller unavailable.")?
}

#[tauri::command]
fn player_report(window: WebviewWindow, state: State<'_, Handle>, report: Report) -> Result<(), String> {
    if !window.label().starts_with("player-") || window.label() != format!("player-{}", report.session) {
        return Err("Session does not belong to this window.".into());
    }
    if report.volume.is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v)) {
        return Err("Invalid volume report.".into());
    }
    // Bounded queue; hostile subframes cannot allocate an unbounded event backlog.
    state.tx.try_send(Message::Report(window.label().into(), report)).map_err(|_| "Telemetry queue is full.".into())
}

fn main() {
    viewer_mode::initialize(std::env::args_os().skip(1));
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    #[cfg(feature = "e2e-tests")]
    e2e::prepare(&mut context).expect("E2E isolation configuration required");
    #[cfg(feature = "e2e-tests")]
    if std::env::args().any(|arg| arg == "--e2e-cleanup") {
        credential_store::CredentialStore::new().forget(0).expect("E2E scoped vault cleanup failed");
        return;
    }
    let builder = tauri::Builder::default();
    #[cfg(feature = "e2e-tests")]
    let builder = if e2e_probe::enabled() { builder }
        else { builder.plugin(tauri_plugin_wdio_webdriver::init_with_port(e2e::port())) };
    builder
        .invoke_handler(tauri::generate_handler![get_state, dispatch, player_report])
        .setup(|app| {
            #[cfg(not(feature = "e2e-tests"))]
            let path = app.path().app_config_dir()?.join("preferences.sqlite3");
            #[cfg(feature = "e2e-tests")]
            let path = e2e::preferences();
            #[allow(unused_mut)]
            let mut store = storage::Store::open(&path)?;
            #[cfg(feature = "e2e-tests")]
            e2e::seed_settings(&mut store)?;
            let settings = store.load()?;
            let host = player::Host::start()?;
            #[cfg(not(feature = "e2e-tests"))]
            let twitch = mpd_twitch::Twitch::new()?;
            #[cfg(feature = "e2e-tests")]
            let twitch = e2e::twitch()?;
            let (tx, rx) = mpsc::channel(256);
            let (publish, view) = watch::channel(View::default());
            app.manage(Handle { tx: tx.clone(), view });
            let controller = Controller::new(app.handle().clone(), tx, publish, store, host, settings, twitch);
            tauri::async_runtime::spawn(controller.run(rx));
            #[cfg(feature = "e2e-tests")]
            e2e::create_manager(app.handle())?;
            #[cfg(feature = "e2e-tests")]
            e2e_probe::install(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, WindowEvent::CloseRequested { .. }) {
                // Never leave invisible background player windows after manager exit.
                window.app_handle().exit(0);
            } else if matches!(event, WindowEvent::Destroyed) {
                if let Some(handle) = window.app_handle().try_state::<Handle>() {
                    let _ = handle.tx.try_send(Message::Destroyed(window.label().into()));
                }
            }
        })
        .run(context)
        .expect("MPD Tabber failed to start");
}
