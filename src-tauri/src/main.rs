#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod controller;
mod model;
mod player;
mod storage;
mod viewer_auth;

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
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_state, dispatch, player_report])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("preferences.sqlite3");
            let store = storage::Store::open(&path)?;
            let settings = store.load()?;
            let host = player::Host::start()?;
            let twitch = mpd_twitch::Twitch::new()?;
            let (tx, rx) = mpsc::channel(256);
            let (publish, view) = watch::channel(View::default());
            app.manage(Handle { tx: tx.clone(), view });
            let controller = Controller::new(app.handle().clone(), tx, publish, store, host, settings, twitch);
            tauri::async_runtime::spawn(controller.run(rx));
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
        .run(tauri::generate_context!())
        .expect("MPD Tabber failed to start");
}
