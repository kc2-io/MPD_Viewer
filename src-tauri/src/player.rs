use std::net::TcpListener;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;
use crate::model::Settings;

pub struct Host { pub local: Url, pub production: Option<Url> }
impl Host {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let server = tiny_http::Server::from_listener(listener, None)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        std::thread::Builder::new().name("player-assets".into()).spawn(move || {
            for request in server.incoming_requests() {
                let host = request.headers().iter().find(|h| h.field.equiv("Host")).map(|h| h.value.as_str());
                let localhost = format!("localhost:{port}");
                if host != Some(localhost.as_str()) || request.method() != &tiny_http::Method::Get {
                    let _ = request.respond(tiny_http::Response::from_string("Forbidden").with_status_code(403));
                    continue;
                }
                let path = request.url().split('?').next().unwrap_or("");
                let asset: Option<(&str, &str)> = match path {
                    "/" | "/index.html" => Some((include_str!("../../player-wrapper/index.html"), "text/html; charset=utf-8")),
                    "/player.js" => Some((include_str!("../../player-wrapper/player.js"), "text/javascript; charset=utf-8")),
                    "/style.css" => Some((include_str!("../../player-wrapper/style.css"), "text/css; charset=utf-8")),
                    _ => None,
                };
                if let Some((body, mime)) = asset {
                    let mut response = tiny_http::Response::from_string(body);
                    for (name, value) in [("Content-Type", mime), ("Cache-Control", "no-store"),
                        ("X-Content-Type-Options", "nosniff"), ("Referrer-Policy", "strict-origin-when-cross-origin")] {
                        if let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) { response.add_header(header); }
                    }
                    let _ = request.respond(response);
                } else { let _ = request.respond(tiny_http::Response::empty(404)); }
            }
        })?;
        let config: serde_json::Value = serde_json::from_str(include_str!("../player-origin.json"))?;
        let production = config.get("url").and_then(|v| v.as_str()).map(Url::parse).transpose()?;
        if let Some(url) = &production {
            if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
                return Err(std::io::Error::other("Production player URL must be HTTPS with no credentials.").into());
            }
        }
        Ok(Self { local: Url::parse(&format!("http://localhost:{port}/index.html"))?, production })
    }
    pub fn base(&self, demo: bool) -> &Url {
        if demo { &self.local } else { self.production.as_ref().unwrap_or(&self.local) }
    }
    pub fn open(&self, app: &AppHandle, login: &str, id: u64, settings: &Settings) -> Result<String, String> {
        let mut url = self.base(settings.demo).clone();
        let fragment = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("channel", login).append_pair("session", &id.to_string())
            .append_pair("volume", &settings.volume.to_string()).append_pair("muted", &settings.muted.to_string())
            .append_pair("demo", &settings.demo.to_string()).finish();
        url.set_fragment(Some(&fragment));
        let label = format!("player-{id}");
        let origin = url.origin();
        WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
            .title(format!("{login} · MPD Viewer{}", if settings.demo { " · SIMULATED" } else { "" }))
            .inner_size(820.0, 550.0).min_inner_size(430.0, 480.0)
            .focused(false)
            .on_navigation(move |target| {
                // No navigation to arbitrary sites and no OS custom-scheme launches.
                // Twitch's own player origin is also allowed for iframe navigation.
                target.origin() == origin || (target.scheme() == "https" && target.host_str() == Some("player.twitch.tv"))
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .on_download(|_, _| false)
            .build().map_err(|e| format!("Could not create player: {e}"))?;
        Ok(label)
    }
}

pub fn audio(app: &AppHandle, label: &str, settings: &Settings) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        let script = format!("window.mpdSetAudio && window.mpdSetAudio({}, {});", settings.volume, settings.muted);
        window.eval(&script).map_err(|e| e.to_string())?;
    }
    Ok(())
}
