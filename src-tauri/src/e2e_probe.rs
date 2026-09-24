//! Driverless policy probe. This module is compiled only with `e2e-tests`.
//! Calls originate in the real fixture webview and cross its ordinary Tauri IPC.
//! No test command, driver evaluator, extra capability, or result HTTP route exists.
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;

const HASH_KEY: &str = "mpd_e2e_policy";

pub fn enabled() -> bool {
    std::env::var("MPD_E2E_POLICY_PROBE").as_deref() == Ok("1")
}

/// Attach only to the local fixture viewers of a driverless probe process.
pub fn script(label: &str, session: u64) -> String {
    let config = serde_json::json!({"label": label, "session": session,
        "wrapper": label.starts_with("player-"), "hashKey": HASH_KEY});
    SCRIPT.replace("__PROBE_CONFIG__", &config.to_string())
}

const SCRIPT: &str = r#"
(() => {
  const cfg = __PROBE_CONFIG__;
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
  async function run() {
    // Run after ordinary wrapper initialization has consumed its configuration hash.
    const deadline = Date.now() + 10000;
    while (document.readyState !== 'complete' || !window.__TAURI_INTERNALS__?.invoke) {
      if (Date.now() >= deadline) throw new Error('native_ipc_not_ready');
      await sleep(50);
    }
    const invoke = (command, args) => Promise.race([
      window.__TAURI_INTERNALS__.invoke(command, args),
      new Promise((_, reject) => setTimeout(() => reject(new Error('probe_timeout')), 3000))
    ]);
    const checks = {};
    async function denied(name, command, args, kind) {
      try { await invoke(command, args); checks[name] = false; }
      catch (error) {
        // A missing bridge, unknown command, timeout or arbitrary exception is NOT denial.
        const message = typeof error === 'string' ? error : String(error?.message || '');
        checks[name] = kind === 'session'
          ? message === 'Session does not belong to this window.'
          : /\bnot allowed\b|\bdenied\b/i.test(message)
            && !/unknown command|not found|timeout|not ready/i.test(message);
      }
    }
    await denied('get_state_denied', 'get_state', {}, 'acl');
    await denied('dispatch_denied', 'dispatch', { action: { type: 'clear_error' } }, 'acl');
    const report = { session: cfg.session, state: 'ready', visible: true, volume: null, muted: null };
    if (cfg.wrapper) {
      try { await invoke('player_report', { report }); checks.own_report_allowed = true; }
      catch (_) { checks.own_report_allowed = false; }
      await denied('wrong_session_denied', 'player_report', {
        report: { ...report, session: cfg.session + 1000000 }
      }, 'session');
    } else {
      await denied('player_report_denied', 'player_report', { report }, 'acl');
    }
    return { label: cfg.label, checks };
  }
  run().catch(() => ({ label: cfg.label, checks: { native_ipc_ready: false } })).then(result => {
    // Preserve the wrapper's startup parameters. Hash observation grants no new native IPC.
    const hash = new URLSearchParams(location.hash.slice(1));
    hash.set(cfg.hashKey, JSON.stringify(result));
    location.hash = hash.toString();
  });
})();
"#;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeResult {
    label: String,
    checks: std::collections::BTreeMap<String, bool>,
}

fn parse_result(url: &url::Url, label: &str) -> Result<Option<ProbeResult>, String> {
    let values: Vec<_> = url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes())
        .filter(|(key, _)| key == HASH_KEY).collect();
    if values.is_empty() { return Ok(None); }
    if values.len() != 1 || values[0].1.len() > 2048 { return Err("Invalid probe result envelope".into()); }
    let result: ProbeResult = serde_json::from_str(&values[0].1).map_err(|_| "Invalid probe result JSON")?;
    if result.label != label { return Err("Probe window identity mismatch".into()); }
    let expected: &[&str] = if label.starts_with("player-") {
        &["dispatch_denied", "get_state_denied", "own_report_allowed", "wrong_session_denied"]
    } else {
        &["dispatch_denied", "get_state_denied", "player_report_denied"]
    };
    if result.checks.len() != expected.len() || expected.iter().any(|name| result.checks.get(*name) != Some(&true)) {
        return Err(format!("Native IPC policy check failed for {label}"));
    }
    Ok(Some(result))
}

async fn action(app: &AppHandle, action: crate::model::Action) -> Result<(), String> {
    let handle = app.try_state::<crate::controller::Handle>().ok_or("Controller not initialized")?;
    let (tx, rx) = oneshot::channel();
    handle.tx.send(crate::controller::Message::Action(action, tx)).await
        .map_err(|_| "Controller action channel closed")?;
    rx.await.map_err(|_| "Controller did not acknowledge action")?
}

async fn run(app: &AppHandle) -> Result<Vec<ProbeResult>, String> {
    use crate::model::Action;
    match crate::e2e::scenario() {
        "demo" => action(app, Action::LoadDemo).await?,
        "web" => {},
        _ => return Err("Policy probe requires demo or web fixtures".into()),
    }
    action(app, Action::SetLimit { limit: 2 }).await?;
    action(app, Action::Start).await?;
    let prefix = if crate::e2e::scenario() == "demo" { "player-" } else { "twitch-page-" };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let windows = app.webview_windows();
        let viewers: Vec<_> = windows.iter().filter(|(label, _)| label.starts_with(prefix)).collect();
        if viewers.len() > 2 { return Err("Policy probe exceeded viewer capacity".into()); }
        let mut results = Vec::new();
        for (label, window) in &viewers {
            let url = window.url().map_err(|_| "Could not observe native fixture URL")?;
            if let Some(result) = parse_result(&url, label)? { results.push(result); }
        }
        if results.len() == 2 {
            results.sort_by(|left, right| left.label.cmp(&right.label));
            return Ok(results);
        }
        if Instant::now() >= deadline { return Err("Timed out waiting for two native IPC probe results".into()); }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Call after creating the manager and installing the ordinary controller state.
pub fn install(app: &AppHandle) {
    if !enabled() { return; }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = tokio::time::timeout(Duration::from_secs(40), run(&app)).await
            .unwrap_or_else(|_| Err("Driverless policy probe timed out".into()));
        let passed = result.is_ok();
        let evidence = serde_json::json!({
            "schema": 1, "passed": passed, "driver_registered": false,
            "scenario": crate::e2e::scenario(), "origin": "local-fixture",
            "boundary": "original Tauri IPC from real native fixture webviews",
            "results": result.as_ref().ok(), "error": result.as_ref().err(),
        });
        let written = std::fs::write(crate::e2e::root().join("policy-probe.json"), evidence.to_string()).is_ok();
        app.exit(if passed && written { 0 } else { 1 });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_all_expected_policy_checks_and_identity() {
        let mut url = url::Url::parse("http://localhost:41234/web/fixture").unwrap();
        let evidence = serde_json::json!({"label":"twitch-page-1", "checks":{
            "dispatch_denied":true,"get_state_denied":true,"player_report_denied":true}});
        url.set_fragment(Some(&url::form_urlencoded::Serializer::new(String::new())
            .append_pair(HASH_KEY, &evidence.to_string()).finish()));
        assert!(parse_result(&url,"twitch-page-1").unwrap().is_some());
        assert!(parse_result(&url,"twitch-page-2").is_err());
        assert!(parse_result(&url,"player-1").is_err());
        url.set_fragment(None);
        assert!(parse_result(&url,"twitch-page-1").unwrap().is_none());
    }
    #[test]
    fn probe_uses_original_ipc_and_preserves_existing_fragment() {
        let source = script("player-3", 3);
        assert!(source.contains("window.__TAURI_INTERNALS__.invoke(command, args)"));
        assert!(source.contains("new URLSearchParams(location.hash.slice(1))"));
        assert!(!source.contains("__PROBE_CONFIG__"));
        assert!(!source.contains("fetch("));
    }
}
