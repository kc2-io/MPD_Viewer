//! Driverless native fixtures for child surfaces. Production excludes this module.
use crate::{
    controller::{Handle, Message},
    model::{Action, Mode, View, ViewerLayout},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub fn enabled() -> bool {
    std::env::var("MPD_E2E_GRID_PROBE").as_deref() == Ok("1")
}

pub fn script() -> &'static str {
    r#"
(() => {
  if(window.top!==window)return;
  const instance=crypto.randomUUID();
  window.mpdGridUserState='fixture-ready';
  window.mpdGridInspect=(inspection)=>{
    const hash=new URLSearchParams(location.hash.slice(1));
    hash.set('grid_fixture',JSON.stringify({inspection,instance,state:window.mpdGridUserState,focus:document.hasFocus(),width:innerWidth,height:innerHeight}));
    location.hash=hash.toString();
  };
  addEventListener('load',window.mpdGridInspect,{once:true});
})();
"#
}

fn state(app: &AppHandle) -> View {
    app.state::<Handle>().view.borrow().clone()
}
async fn action(app: &AppHandle, action: Action) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.state::<Handle>()
        .tx
        .send(Message::Action(action, tx))
        .await
        .map_err(|_| "Controller closed")?;
    rx.await.map_err(|_| "Controller did not reply")?
}
async fn wait(
    app: &AppHandle,
    description: &str,
    condition: impl Fn(&View) -> bool,
) -> Result<View, String> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let view = state(app);
        if condition(&view) {
            return Ok(view);
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Timed out: {description}; mode={}, players={}, error={:?}",
                match view.mode {
                    Mode::Stopped => "Stopped",
                    Mode::Running => "Running",
                    Mode::Paused => "Paused",
                },
                view.players.len(),
                view.error
            ));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
async fn documents(app: &AppHandle, labels: &[String]) -> Result<Vec<serde_json::Value>, String> {
    let inspection = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    let script = format!(
        "window.mpdGridInspect && window.mpdGridInspect({})",
        serde_json::json!(inspection)
    );
    for label in labels {
        app.get_webview(label)
            .ok_or("Missing retained surface")?
            .eval(&script)
            .map_err(|e| e.to_string())?;
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut results = Vec::new();
        for label in labels {
            if let Some(view) = app.get_webview(label) {
                // Read-only inspection may arrive before fixture script setup.
                // Repeat observation until this nonce is acknowledged.
                view.eval(&script).map_err(|e| e.to_string())?;
                if let Ok(url) = view.url() {
                    if let Some((_, json)) =
                        url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes())
                            .find(|(key, _)| key == "grid_fixture")
                    {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                            if value["inspection"] == inspection {
                                results.push(value)
                            }
                        }
                    }
                }
            }
        }
        if results.len() == labels.len() {
            return Ok(results);
        }
        if Instant::now() >= deadline {
            return Err("Native document inspection unavailable".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
fn assert_retained(
    before: &[serde_json::Value],
    after: &[serde_json::Value],
) -> Result<(), String> {
    for (a, b) in before.iter().zip(after) {
        if a["instance"] != b["instance"] || b["state"] != "fixture-paused" {
            return Err("Retained move lost fixture document/state".into());
        }
    }
    Ok(())
}
async fn run(app: &AppHandle) -> Result<serde_json::Value, String> {
    use Action::*;
    let demo = crate::e2e::scenario() == "demo";
    let timed = if demo { "bravo_demo" } else { "alpha_fixture" };
    if demo {
        action(app, LoadDemo).await?;
        action(
            app,
            DemoLive {
                login: "delta_demo".into(),
                live: false,
            },
        )
        .await?;
    } else {
        action(
            app,
            Enable {
                login: "charlie_fixture".into(),
                enabled: false,
            },
        )
        .await?;
    }

    let geometry = crate::presentation::preflight(app, 2, demo);
    if let Err(error) = &geometry {
        if !matches!(error, crate::presentation::GridError::NoFit(message, _)
            if demo && message.starts_with("Grid cannot fit 2 selected viewers"))
        {
            return Err(error.to_string());
        }
    }
    let count = if demo && geometry.is_err() {
        // Small hosted desktops cannot fit two 800x540 embedded cells. Prove
        // the production rejection, then exercise retention with one cell.
        action(
            app,
            SetLayout {
                layout: ViewerLayout::Grid,
            },
        )
        .await?;
        action(app, Start).await?;
        wait(app, "small display rejects two embedded cells", |v| {
            v.mode == Mode::Stopped
                && v.players.is_empty()
                && v.error
                    .as_deref()
                    .is_some_and(|e| e.starts_with("Grid cannot fit 2 selected viewers"))
        })
        .await?;
        action(
            app,
            SetLayout {
                layout: ViewerLayout::Standalone,
            },
        )
        .await?;
        action(
            app,
            Enable {
                login: "charlie_demo".into(),
                enabled: false,
            },
        )
        .await?;
        action(app, ClearError).await?;
        1
    } else {
        2
    };
    action(app, SetLimit { limit: 1000 }).await?;
    action(
        app,
        SetTimer {
            login: timed.into(),
            minutes: Some(1),
        },
    )
    .await?;
    std::fs::write(
        crate::e2e::root().join("fixture-state.json"),
        b"{\"fail_after_child\":true,\"fail_close\":true}",
    )
    .map_err(|e| e.to_string())?;
    action(app, Start).await?;
    wait(
        app,
        "partial opens retain failed cleanup reservations",
        |v| !v.players.is_empty() && v.error.is_some(),
    )
    .await?;
    std::fs::write(
        crate::e2e::root().join("fixture-state.json"),
        b"{\"fail_after_child\":true}",
    )
    .map_err(|e| e.to_string())?;
    // Cleanup retries independently of selection, without Stop or explicit Retry.
    let cleanup_deadline = Instant::now() + Duration::from_secs(40);
    loop {
        let view = state(app);
        if view.players.is_empty()
            && view
                .favorites
                .iter()
                .filter(|f| f.open_error.is_some())
                .count()
                == count
        {
            if view.mode != Mode::Running {
                return Err("Partial-open cleanup stopped monitoring".into());
            }
            break;
        }
        if Instant::now() >= cleanup_deadline {
            return Err("Failed partial-open cleanup did not retry while Running".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    std::fs::write(crate::e2e::root().join("fixture-state.json"), b"{}")
        .map_err(|e| e.to_string())?;
    let logins = if demo {
        vec!["bravo_demo", "charlie_demo"]
    } else {
        vec!["alpha_fixture", "bravo_fixture"]
    };
    for login in logins.into_iter().take(count) {
        action(
            app,
            Retry {
                login: login.into(),
            },
        )
        .await?;
    }
    action(app, ClearError).await?;
    let initial = wait(app, "selected native fixtures", |v| {
        v.players.len() == count
    })
    .await?;
    let labels: Vec<_> = initial
        .players
        .iter()
        .map(|p| {
            format!(
                "{}-{}",
                if demo { "player" } else { "twitch-page" },
                p.session
            )
        })
        .collect();
    documents(app, &labels).await?;
    // Change a document-owned value before reparent, then observe it afterwards.
    for label in &labels {
        app.get_webview(label)
            .unwrap()
            .eval("window.mpdGridUserState='fixture-paused'; document.body.dataset.retained='yes'")
            .map_err(|e| e.to_string())?;
    }
    let before = documents(app, &labels).await?;
    for fault in ["fail_layout_prepare", "fail_layout_after_move"] {
        std::fs::write(
            crate::e2e::root().join("fixture-state.json"),
            serde_json::json!({(fault):true}).to_string(),
        )
        .map_err(|e| e.to_string())?;
        action(
            app,
            SetLayout {
                layout: ViewerLayout::Grid,
            },
        )
        .await?;
        let preserved = wait(app, "source-intact layout failure", |v| {
            v.layout_pending.is_none() && v.error.is_some()
        })
        .await?;
        if preserved.mode != Mode::Running
            || preserved.settings.viewer_layout != ViewerLayout::Standalone
            || app.windows().len() != count + 1
        {
            return Err("Preparation/rollback failure stopped or leaked intact sources".into());
        }
        assert_retained(&before, &documents(app, &labels).await?)?;
        std::fs::write(crate::e2e::root().join("fixture-state.json"), b"{}")
            .map_err(|e| e.to_string())?;
        action(app, ClearError).await?;
    }
    action(
        app,
        SetLayout {
            layout: ViewerLayout::Grid,
        },
    )
    .await?;
    action(
        app,
        SetLayout {
            layout: ViewerLayout::Standalone,
        },
    )
    .await?;
    action(
        app,
        SetLayout {
            layout: ViewerLayout::Grid,
        },
    )
    .await?;
    wait(app, "grid commit", |v| {
        v.layout_pending.is_none() && v.settings.viewer_layout == ViewerLayout::Grid
    })
    .await?;
    let after = documents(app, &labels).await?;
    assert_retained(&before, &after)?;
    // Run original IPC from children while they share the Grid container.
    for (label, player) in labels.iter().zip(&initial.players) {
        app.get_webview(label)
            .unwrap()
            .eval(
                crate::e2e_probe::script(label, player.session)
                    .replace("mpd_e2e_policy", "mpd_e2e_grid_policy"),
            )
            .map_err(|e| e.to_string())?;
    }
    let grid = app
        .windows()
        .into_iter()
        .find(|(label, _)| label.starts_with("viewer-grid-"))
        .ok_or("No native grid container")?
        .1;
    if grid.webviews().len() != count || app.windows().len() != 2 {
        return Err("Grid did not produce one container with the selected children".into());
    }
    let mut bounds = Vec::new();
    for label in &labels {
        let view = app.get_webview(label).unwrap();
        let rect = crate::presentation::bounds(&view)?;
        let size = rect
            .size
            .to_logical::<f64>(grid.scale_factor().map_err(|e| e.to_string())?);
        if size.width < crate::layout::MIN_WIDTH || size.height < crate::layout::MIN_HEIGHT {
            return Err("Native grid cell below minimum".into());
        }
        let pos = rect
            .position
            .to_logical::<f64>(grid.scale_factor().map_err(|e| e.to_string())?);
        bounds.push(serde_json::json!({"label":label,"x":pos.x,"y":pos.y,"width":size.width,"height":size.height}));
    }
    if count == 2 {
        let a = &bounds[0];
        let b = &bounds[1];
        let separated = a["x"].as_f64().unwrap() + a["width"].as_f64().unwrap()
            <= b["x"].as_f64().unwrap() - crate::layout::DIVIDER + 0.5
            || b["x"].as_f64().unwrap() + b["width"].as_f64().unwrap()
                <= a["x"].as_f64().unwrap() - crate::layout::DIVIDER + 0.5
            || a["y"].as_f64().unwrap() + a["height"].as_f64().unwrap()
                <= b["y"].as_f64().unwrap() - crate::layout::DIVIDER + 0.5
            || b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap()
                <= a["y"].as_f64().unwrap() - crate::layout::DIVIDER + 0.5;
        if !separated {
            return Err(format!("Native child cells overlap: {bounds:?}"));
        }
    }
    let original_position = grid.outer_position().map_err(|e| e.to_string())?;
    let original_size = grid.inner_size().map_err(|e| e.to_string())?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let app_clone = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(
            app_clone
                .available_monitors()
                .map(|monitors| {
                    monitors
                        .iter()
                        .map(|m| m.work_area().position)
                        .collect::<Vec<_>>()
                })
                .map_err(|e| e.to_string()),
        );
    })
    .map_err(|e| e.to_string())?;
    let monitors = rx.await.map_err(|e| e.to_string())??;
    let other = monitors.iter().find(|p| {
        (p.x - original_position.x).abs() > 100 || (p.y - original_position.y).abs() > 100
    });
    let target = other.map_or(
        tauri::PhysicalPosition::new(original_position.x + 20, original_position.y + 20),
        |p| tauri::PhysicalPosition::new(p.x + 20, p.y + 20),
    );
    grid.set_position(target).map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let moved = grid.outer_position().map_err(|e| e.to_string())?;
    if other.is_some() && ((moved.x - target.x).abs() > 2 || (moved.y - target.y).abs() > 2) {
        return Err("Native grid did not reach the selected other monitor".into());
    }
    tokio::time::sleep(Duration::from_millis(1800)).await;
    if grid.outer_position().map_err(|e| e.to_string())? != moved {
        return Err("Periodic layout moved a reachable grid back to the manager origin".into());
    }
    if other.is_some() {
        std::fs::write(
            crate::e2e::root().join("grid-other-monitor-ready"),
            b"fixture",
        )
        .map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    grid.maximize().map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    if !grid.is_maximized().map_err(|e| e.to_string())? {
        return Err("Native grid maximize was not observed".into());
    }
    tokio::time::sleep(Duration::from_millis(1800)).await;
    if !grid.is_maximized().map_err(|e| e.to_string())? {
        return Err("Periodic grid layout undid native maximization".into());
    }
    grid.unmaximize().map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    grid.set_size(original_size).map_err(|e| e.to_string())?;
    grid.set_position(original_position)
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    crate::presentation::focus(app, &labels[0])?;
    let manager = app.get_window("main").ok_or("Missing theme manager")?;
    for (theme, marker) in [
        (tauri::Theme::Light, "grid-light-ready"),
        (tauri::Theme::Dark, "grid-dark-ready"),
    ] {
        manager.set_theme(Some(theme)).map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        std::fs::write(crate::e2e::root().join(marker), b"fixture").map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    manager.set_theme(None).map_err(|e| e.to_string())?;
    std::fs::write(crate::e2e::root().join("grid-ready"), b"fixture").map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    action(
        app,
        SetLayout {
            layout: ViewerLayout::Standalone,
        },
    )
    .await?;
    wait(app, "standalone commit", |v| {
        v.layout_pending.is_none() && v.settings.viewer_layout == ViewerLayout::Standalone
    })
    .await?;
    let returned = documents(app, &labels).await?;
    assert_retained(&before, &returned)?;
    let timed_player = state(app)
        .players
        .into_iter()
        .find(|p| p.login == timed)
        .ok_or("Timed assignment disappeared")?;
    if let crate::model::TimerView::Counting { remaining_seconds } = timed_player.timer {
        if remaining_seconds >= 60 {
            return Err("Layout changes reset or froze assignment time".into());
        }
    } else {
        return Err("Timed assignment lost its turn".into());
    }

    if app.windows().len() != count + 1 {
        return Err("Standalone restoration leaked containers".into());
    }
    // Native IPC probes run in the same children and survive container changes.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut ready = true;
        for label in &labels {
            let url = app
                .get_webview(label)
                .unwrap()
                .url()
                .map_err(|e| e.to_string())?;
            let report = url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes())
                .find(|(key, _)| key == "mpd_e2e_grid_policy");
            if let Some((_, json)) = report {
                let value: serde_json::Value =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                if value["checks"].as_object().is_none_or(|checks| {
                    checks.len() != if demo { 4 } else { 3 } || checks.values().any(|v| v != true)
                }) {
                    return Err("Native child IPC isolation failed".into());
                }
            } else {
                ready = false
            }
        }
        if ready {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Native IPC probes did not finish".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Stop supersedes an in-flight retained transaction.
    action(
        app,
        SetLayout {
            layout: ViewerLayout::Grid,
        },
    )
    .await?;
    action(app, Stop).await?;
    if let Err(error) = action(
        app,
        SetLayout {
            layout: ViewerLayout::Standalone,
        },
    )
    .await
    {
        if !error.contains("Stop") {
            return Err(error);
        }
    }
    wait(app, "Stop cancels transition and cleans media", |v| {
        v.layout_pending.is_none() && v.players.is_empty() && v.mode == Mode::Stopped
    })
    .await?;
    // After complete cleanup a large configured limit still permits two cells.
    action(
        app,
        SetLayout {
            layout: ViewerLayout::Grid,
        },
    )
    .await?;
    action(app, Start).await?;
    wait(app, "grid restart", |v| v.players.len() == count).await?;
    action(app, SetLimit { limit: count }).await?;
    if demo {
        action(
            app,
            DemoLive {
                login: "delta_demo".into(),
                live: true,
            },
        )
        .await?;
    } else {
        action(
            app,
            Enable {
                login: "charlie_fixture".into(),
                enabled: true,
            },
        )
        .await?;
    }
    let replacement_parent = app
        .windows()
        .into_keys()
        .find(|label| label.starts_with("viewer-grid-"))
        .ok_or("Missing replacement grid")?;
    let reserved = state(app)
        .players
        .into_iter()
        .find(|p| p.login == timed)
        .ok_or("No timed fixture")?
        .session;
    std::fs::write(
        crate::e2e::root().join("fixture-state.json"),
        b"{\"fail_close\":true}",
    )
    .map_err(|e| e.to_string())?;
    action(
        app,
        Skip {
            login: timed.into(),
        },
    )
    .await?;
    let failed = wait(app, "failed close reserves capacity", |v| v.error.is_some()).await?;
    if failed.players.len() != count || !failed.players.iter().any(|p| p.session == reserved) {
        return Err("Failed close released capacity".into());
    }
    std::fs::write(crate::e2e::root().join("fixture-state.json"), b"{}")
        .map_err(|e| e.to_string())?;
    action(
        app,
        Retry {
            login: timed.into(),
        },
    )
    .await?;
    wait(app, "confirmed close permits replacement", |v| {
        v.players.len() == count && !v.players.iter().any(|p| p.session == reserved)
    })
    .await?;
    let grid = app
        .windows()
        .into_iter()
        .find(|(label, _)| label.starts_with("viewer-grid-"))
        .ok_or("Grid disappeared before close test")?
        .1;
    if grid.label() != replacement_parent {
        return Err("Replacement recreated the active grid container".into());
    }
    grid.close().map_err(|e| e.to_string())?;
    wait(app, "closing grid Stops rather than skipping", |v| {
        v.mode == Mode::Stopped && v.players.is_empty()
    })
    .await?;
    action(
        app,
        UndoSkip {
            login: timed.into(),
        },
    )
    .await?;
    action(
        app,
        Enable {
            login: if demo {
                "delta_demo"
            } else {
                "charlie_fixture"
            }
            .into(),
            enabled: false,
        },
    )
    .await?;
    // Allow Stop's empty-parent retirement to dispatch, then restart before
    // relying on an eventual Destroyed notification for cached ownership.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    action(app, SetLimit { limit: 1000 }).await?;
    action(app, Start).await?;
    wait(app, "grid starts after explicit user request", |v| {
        v.players.len() == count
    })
    .await?;
    // A failed display observation must retain active viewers, even when a new
    // eligible favorite increases the desired count beyond cached admission.
    action(
        app,
        SetTimer {
            login: timed.into(),
            minutes: None,
        },
    )
    .await?;
    let retained: Vec<_> = state(app).players.iter().map(|p| p.session).collect();
    std::fs::write(
        crate::e2e::root().join("fixture-state.json"),
        b"{\"fail_display_observation\":true}",
    )
    .map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(2200)).await;
    let extra = "grid_observation_fixture";
    action(
        app,
        Add {
            input: extra.into(),
        },
    )
    .await?;
    if demo {
        action(
            app,
            DemoLive {
                login: extra.into(),
                live: true,
            },
        )
        .await?;
    }
    tokio::time::sleep(Duration::from_millis(2200)).await;
    let after = state(app);
    if after.mode != Mode::Running
        || after.players.iter().map(|p| p.session).collect::<Vec<_>>() != retained
    {
        return Err("Transient display failure stopped or replaced active viewers".into());
    }
    action(
        app,
        Remove {
            login: extra.into(),
        },
    )
    .await?;
    // Let a mutation time out without Stop, then prove its already-retired
    // token prevents the delayed native callback from running after Stop.
    std::fs::write(
        crate::e2e::root().join("fixture-state.json"),
        b"{\"delay_grid_mutation\":true}",
    )
    .map_err(|e| e.to_string())?;
    wait(app, "native mutation worker times out", |v| {
        v.error
            .as_deref()
            .is_some_and(|e| e.contains("Grid mutation timed out"))
    })
    .await?;
    std::fs::write(crate::e2e::root().join("fixture-state.json"), b"{}")
        .map_err(|e| e.to_string())?;
    action(app, Stop).await?;
    wait(app, "Stop after expired mutation", |v| {
        v.mode == Mode::Stopped && v.players.is_empty()
    })
    .await?;
    let skipped = crate::e2e::root().join("grid-mutation-skipped");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !skipped.exists() {
        if Instant::now() > deadline {
            return Err("Timed-out native mutation remained unfenced".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    action(app, ClearError).await?;
    action(app, Start).await?;
    wait(app, "restart after expired mutation", |v| {
        v.mode == Mode::Running && v.players.len() == count
    })
    .await?;
    std::fs::write(
        crate::e2e::root().join("fixture-state.json"),
        b"{\"slow_display_observation\":true}",
    )
    .map_err(|e| e.to_string())?;
    let marker = crate::e2e::root().join("display-observation-started");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() {
        if Instant::now() > deadline {
            return Err("Delayed display worker was not observed".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let stop_started = Instant::now();
    action(app, Stop).await?;
    if stop_started.elapsed() > Duration::from_secs(2) {
        return Err("Display worker blocked the controller's Stop acknowledgement".into());
    }
    wait(app, "Stop during delayed display work", |v| {
        v.mode == Mode::Stopped && v.players.is_empty()
    })
    .await?;
    std::fs::write(crate::e2e::root().join("fixture-state.json"), b"{}")
        .map_err(|e| e.to_string())?;
    action(app, ClearError).await?;
    action(app, Start).await?;
    wait(app, "restart after cancelled display work", |v| {
        v.mode == Mode::Running && v.players.len() == count
    })
    .await?;
    // Growth beyond available geometry must stop rather than truncate selection.
    for index in 0..20 {
        let login = format!("grid_fixture_{index}");
        action(
            app,
            Add {
                input: login.clone(),
            },
        )
        .await?;
        if demo {
            action(app, DemoLive { login, live: true }).await?;
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if state(app).mode == Mode::Stopped {
            break;
        }
    }
    let stopped = wait(app, "no-fit stops all", |v| {
        v.players.is_empty() && v.mode == Mode::Stopped
    })
    .await?;
    if stopped.settings.limit != 1000
        || stopped.settings.viewer_layout != ViewerLayout::Grid
        || stopped.error.is_none()
    {
        return Err("No-fit changed capacity/preference or hid its reason".into());
    }
    Ok(
        serde_json::json!({"two_embedded_cells_fit": demo.then_some(count==2),"other_monitor_placement_retained":other.is_some(),"reachable_placement_retained":true,"maximized_grid_retained":true,"transient_display_failure_retains_viewers":true,"stop_during_display_work":true,"expired_mutation_fenced":true,"retained_document":true,"retained_fixture_state":true,"grid_children":count,"standalone_restored":true,"preparation_preserves_sources":true,"partial_open_cleanup":true,"partial_move_rollback":true,"configured_limit":1000,"ipc_isolation":true,"stop_during_move":true,"repeated_requests":true,"failed_close_reserves_capacity":true,"grid_close_stops":true,"timer_continuity":true,"no_fit_stops_without_truncation":true,"bounds":bounds,"live_twitch":false}),
    )
}
pub fn install(app: &AppHandle) {
    if !enabled() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = tokio::time::timeout(Duration::from_secs(90), run(&app))
            .await
            .unwrap_or_else(|_| Err("Native grid probe timed out".into()));
        let passed = result.is_ok();
        let evidence = serde_json::json!({"passed":passed,"driver_registered":false,"scenario":crate::e2e::scenario(),"origin":"isolated-local-fixtures","result":result.as_ref().ok(),"error":result.as_ref().err()});
        let written = std::fs::write(
            crate::e2e::root().join("grid-probe.json"),
            evidence.to_string(),
        )
        .is_ok();
        app.exit(if passed && written { 0 } else { 1 });
    });
}
