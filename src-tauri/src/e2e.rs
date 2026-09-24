//! Test-build-only startup isolation. No production entry points or commands.
use std::{path::PathBuf, sync::OnceLock};
use tauri::{Manager, WebviewWindowBuilder};

struct Configuration {
    root: PathBuf,
    run_id: String,
    port: u16,
    manager: tauri::utils::config::WindowConfig,
    fixture_port: u16,
    scenario: String,
}
static CONFIG: OnceLock<Configuration> = OnceLock::new();

pub fn prepare(context: &mut tauri::Context<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var("MPD_E2E_ROOT")?);
    let run_id = std::env::var("MPD_E2E_RUN_ID")?;
    if !root.is_absolute() || !root.is_dir() || run_id.len() != 32
        || !run_id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err("E2E requires an existing absolute test root and a 32-character lowercase hex run ID".into());
    }
    let root = root.canonicalize()?;
    if std::fs::read_to_string(root.join(".mpd-e2e-root"))?.trim() != run_id {
        return Err("E2E root ownership marker does not match".into());
    }
    let port: u16 = std::env::var("TAURI_WEBDRIVER_PORT")?.parse()?;
    if port < 1024 { return Err("E2E driver requires an unprivileged explicit port".into()); }
    let windows = &mut context.config_mut().app.windows;
    if windows.len() != 1 || windows[0].label != "main" { return Err("Unexpected manager window configuration".into()); }
    let manager = windows.remove(0);
    let scenario = std::env::var("MPD_E2E_SCENARIO").unwrap_or_else(|_| "demo".into());
    if !matches!(scenario.as_str(), "demo" | "web" | "auth") { return Err("Unknown E2E scenario".into()); }
    let fixture_port = start_fixture(&root)?;
    CONFIG.set(Configuration { root, run_id, port, manager, fixture_port, scenario }).map_err(|_| "E2E already initialized")?;
    Ok(())
}
fn config() -> &'static Configuration { CONFIG.get().expect("E2E isolation must precede any native startup") }
pub fn preferences() -> PathBuf { config().root.join("preferences.sqlite3") }
pub fn root() -> &'static std::path::Path { &config().root }
pub fn vault_target() -> String { format!("MPD_Viewer-E2E/{}/TwitchOAuth", config().run_id) }
pub fn port() -> u16 { config().port }
pub fn twitch() -> Result<mpd_twitch::Twitch, mpd_twitch::ApiError> { mpd_twitch::Twitch::local_fixture(config().fixture_port) }
pub fn scenario() -> &'static str { &config().scenario }
pub fn url(path: &str) -> url::Url { url::Url::parse(&format!("http://localhost:{}{path}", config().fixture_port)).expect("local fixture URL") }
pub fn fail_open(login: &str) -> bool {
    std::fs::read(config().root.join("fixture-state.json")).ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|state| state.get("fail_open").and_then(|value| value.as_array()).cloned())
        .is_some_and(|logins| logins.iter().any(|value| value.as_str() == Some(login)))
}
pub fn is_fixture(target: &url::Url) -> bool { target.origin() == url("/").origin() }
pub fn seed_settings(store: &mut crate::storage::Store) -> Result<(), String> {
    if scenario() == "demo" || config().root.join(".settings-seeded").exists() { return Ok(()); }
    let settings = crate::model::Settings { demo: false, limit: 2,
        favorites: ["alpha_fixture", "bravo_fixture", "charlie_fixture"].into_iter().map(|login|
            crate::model::Favorite { login: login.into(), enabled: true, watch_minutes: None }).collect(), ..Default::default() };
    store.save(&settings)?;
    std::fs::write(config().root.join(".settings-seeded"), b"1").map_err(|e| e.to_string())
}
pub fn session() -> mpd_twitch::Session {
    let bytes = serde_json::to_vec(&serde_json::json!({"version":1,"client_id":crate::model::DEFAULT_CLIENT_ID,
        "access":"fixture-access","refresh":"fixture-refresh","user_id":"123"})).unwrap();
    mpd_twitch::Session::from_stored(crate::model::DEFAULT_CLIENT_ID, mpd_twitch::StoredSession::decode(&bytes).unwrap()).unwrap()
}

fn start_fixture(root: &std::path::Path) -> Result<u16, Box<dyn std::error::Error>> {
    let port_file = root.join(".fixture-port");
    let port: u16 = if port_file.exists() { std::fs::read_to_string(&port_file)?.parse()? } else { 0 };
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
    let port = listener.local_addr()?.port();
    std::fs::write(port_file, port.to_string())?;
    let server = tiny_http::Server::from_listener(listener, None).map_err(|e| std::io::Error::other(e.to_string()))?;
    let root = root.to_owned();
    std::thread::spawn(move || for request in server.incoming_requests() {
        let host = format!("localhost:{port}");
        if request.headers().iter().find(|h| h.field.equiv("Host")).map(|h| h.value.as_str()) != Some(host.as_str()) {
            let _ = request.respond(tiny_http::Response::empty(403)); continue;
        }
        let target = url::Url::parse(&format!("http://{host}{}", request.url())).unwrap();
        let state: serde_json::Value = std::fs::read(root.join("fixture-state.json")).ok()
            .and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let (body, mime, status) = match target.path() {
            "/oauth/device" => (serde_json::json!({"device_code":"fixture-device","user_code":"E2ETEST1",
                "verification_uri":"https://www.twitch.tv/activate?device-code=E2ETEST1","expires_in":600,"interval":5}).to_string(), "application/json", 200),
            "/oauth/token" if scenario() == "auth" && !root.join(".authorized").exists() =>
                (r#"{"message":"authorization_pending"}"#.into(), "application/json", 400),
            "/oauth/token" => (r#"{"access_token":"fixture-access","refresh_token":"fixture-refresh","expires_in":3600}"#.into(), "application/json", 200),
            "/oauth/validate" => (serde_json::json!({"client_id":crate::model::DEFAULT_CLIENT_ID,
                "user_id":"123","login":"fixture_user","expires_in":3600}).to_string(), "application/json", 200),
            "/helix/streams" if state["fail_api"] == true => ("{}".into(), "application/json", 503),
            "/helix/streams" => {
                let data: Vec<_> = target.query_pairs().filter(|(k, _)| k == "user_login").enumerate()
                    .filter(|(_, (_, login))| !state["offline"].as_array().is_some_and(|xs| xs.iter().any(|x| x.as_str()==Some(login))))
                    .map(|(i,(_,login))| serde_json::json!({"id":format!("fixture-{login}"),"user_login":login,
                        "viewer_count":state["count"].as_u64().unwrap_or((i as u64+1)*100)})).collect();
                (serde_json::json!({"data":data,"pagination":{}}).to_string(), "application/json", 200)
            },
            "/authorize" => {
                let _ = std::fs::write(root.join(".authorized"), b"1");
                ("<h1>Fixture authorized</h1>".into(), "text/html", 200)
            },
            "/activation" => ("<!doctype html><title>MPD E2E authorization fixture</title><h1>Local authorization fixture</h1><a id='authorize' href='/authorize'>Authorize simulation</a>".into(), "text/html", 200),
            "/fixture.js" => ("document.querySelector('#channel').textContent=location.pathname.split('/').pop(); localStorage.setItem('mpd-e2e-profile',localStorage.getItem('mpd-e2e-profile')||'remembered'); document.querySelector('#profile').textContent=localStorage.getItem('mpd-e2e-profile');".into(), "text/javascript", 200),
            path if path.starts_with("/web/") => ("<!doctype html><html><head><title>MPD E2E local viewer</title><script src='/fixture.js' defer></script></head><body><h1>MPD E2E local viewer</h1><p id='channel'></p><p id='profile'></p><p>No Twitch video, viewers or reward credit.</p></body></html>".into(), "text/html", 200),
            _ => ("Not found".into(), "text/plain", 404),
        };
        let mut response = tiny_http::Response::from_string(body).with_status_code(status);
        for (name,value) in [("Content-Type",mime),("Cache-Control","no-store"),("Content-Security-Policy","default-src 'self'; script-src 'self'; connect-src ipc: http://ipc.localhost https://ipc.localhost; frame-src 'none'; object-src 'none'")] {
            response.add_header(tiny_http::Header::from_bytes(name,value).unwrap());
        }
        let _ = request.respond(response);
    });
    Ok(port)
}

pub fn isolate<'a>(builder: WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle>) -> WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle> {
    #[cfg(target_os = "macos")]
    {
        let mut identifier = [0u8; 16];
        for (i, byte) in identifier.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&config().run_id[i*2..i*2+2], 16).expect("validated run ID");
        }
        builder.data_store_identifier(identifier)
    }
    #[cfg(not(target_os = "macos"))]
    { builder.data_directory(config().root.join("webview")) }
}

pub fn create_manager(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let builder = WebviewWindowBuilder::from_config(app, &config().manager)?;
    isolate(builder)
        .on_navigation(bundled_manager_url)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .build()?;
    // Bounded native window evidence only. No profiles, tokens or arbitrary URLs.
    let evidence = serde_json::json!({"run_id":config().run_id,"driver_port":config().port,
        "windows": app.webview_windows().keys().collect::<Vec<_>>(), "instrumented":true});
    std::fs::write(config().root.join("native-startup.json"), serde_json::to_vec(&evidence)?)?;
    observe_native(app.clone());
    Ok(())
}

fn bundled_manager_url(url: &url::Url) -> bool {
    url.username().is_empty() && url.password().is_none() && url.port().is_none()
        && matches!((url.scheme(), url.host_str()),
            ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn manager_rejects_loopback_services_and_credential_urls() {
        for valid in ["tauri://localhost/", "http://tauri.localhost/", "https://tauri.localhost/index.html"] {
            assert!(super::bundled_manager_url(&valid.parse().unwrap()));
        }
        for invalid in ["http://localhost:4445/", "http://127.0.0.1/", "http://tauri.localhost:4445/", "https://user@tauri.localhost/", "https://twitch.tv/"] {
            assert!(!super::bundled_manager_url(&invalid.parse().unwrap()));
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeCommand { id: String, action: String, label: String, theme: Option<String>, width: Option<u32>, height: Option<u32> }
fn observe_native(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut previous = Vec::<String>::new();
        loop {
            let mut windows: Vec<String> = app.webview_windows().keys().cloned().collect();
            windows.sort();
            if windows != previous {
                let state = serde_json::json!({"windows":windows,"pid":std::process::id()});
                let _ = std::fs::write(config().root.join("native-state.json"), state.to_string());
                previous = windows;
            }
            let command_path = config().root.join("native-command.json");
            if let Ok(bytes) = std::fs::read(&command_path) {
                let _ = std::fs::remove_file(&command_path);
                if bytes.len() <= 4096 {
                    if let Ok(command) = serde_json::from_slice::<NativeCommand>(&bytes) {
                        if command.id.len() <= 64 && command.label.len() <= 80 {
                            let handle = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                let result = if let Some(window) = handle.get_webview_window(&command.label) {
                                    match command.action.as_str() {
                                        // This calls the native close API and exercises CloseRequested.
                                        // It is not represented as a titlebar mouse-input test.
                                        "close" => window.close().map_err(|e| e.to_string()),
                                        "resize" => match (command.width, command.height) {
                                            (Some(width @ 640..=1600), Some(height @ 480..=1000)) =>
                                                window.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| e.to_string()),
                                            _ => Err("Invalid native size".into()),
                                        },
                                        "theme" => match command.theme.as_deref() {
                                            Some("light") => window.set_theme(Some(tauri::Theme::Light)).map_err(|e| e.to_string()),
                                            Some("dark") => window.set_theme(Some(tauri::Theme::Dark)).map_err(|e| e.to_string()),
                                            _ => Err("Invalid native theme".into()),
                                        },
                                        _ => Err("Unknown native test action".into()),
                                    }
                                } else { Err("Native window not found".into()) };
                                let response = serde_json::json!({"id":command.id,"ok":result.is_ok(),"error":result.err()});
                                let _ = std::fs::write(config().root.join("native-command-result.json"), response.to_string());
                            });
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}
