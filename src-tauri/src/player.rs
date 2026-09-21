use std::net::TcpListener;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;
use crate::model::{Settings, ViewerCount};

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
                    "/quality.js" => Some((include_str!("../../player-wrapper/quality.js"), "text/javascript; charset=utf-8")),
                    "/chat.js" => Some((include_str!("../../player-wrapper/chat.js"), "text/javascript; charset=utf-8")),
                    "/chat.css" => Some((include_str!("../../player-wrapper/chat.css"), "text/css; charset=utf-8")),
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
    fn player_url(&self, login: &str, id: u64, settings: &Settings) -> Url {
        let mut url = self.base(settings.demo).clone();
        if !settings.demo && self.production.is_some() {
            // The deployed host consumes queries and fractional volume, unlike
            // the bundled demo wrapper. An invalid channel sentinel keeps the
            // legacy active-player handler muted until the native adapter sets
            // volume BEFORE unmuting. It never selects another channel.
            url.set_fragment(None);
            let retained: Vec<(String, String)> = url.query_pairs()
                .filter(|(key, _)| !matches!(key.as_ref(), "channel" | "active" | "volume" | "pauseInactive"))
                .map(|(key, value)| (key.into_owned(), value.into_owned())).collect();
            url.query_pairs_mut().clear().extend_pairs(retained)
                .append_pair("channel", login).append_pair("active", "__mpd-native-pending__")
                .append_pair("volume", &(f64::from(settings.volume) / 100.0).to_string())
                .append_pair("pauseInactive", "false");
        } else {
            let fragment = url::form_urlencoded::Serializer::new(String::new())
                .append_pair("channel", login).append_pair("session", &id.to_string())
                .append_pair("volume", &settings.volume.to_string()).append_pair("muted", &settings.muted.to_string())
                .append_pair("demo", &settings.demo.to_string())
                .append_pair("quality", &settings.preferred_quality).finish();
            url.set_fragment(Some(&fragment));
        }
        url
    }
    pub fn open(&self, app: &AppHandle, login: &str, id: u64, settings: &Settings) -> Result<String, String> {
        let url = self.player_url(login, id, settings);
        let mut adapter = if !settings.demo && self.production.is_some() {
            format!("({})({}, {});", include_str!("../hosted-player-adapter.js"), serde_json::json!({
                "origin": url.origin().ascii_serialization(), "path": url.path(),
                "channel": login, "session": id, "volume": settings.volume, "muted": settings.muted,
                "quality": settings.preferred_quality
            }), include_str!("../../player-wrapper/quality.js"))
        } else { String::new() };
        if !settings.demo && self.production.is_some() {
            adapter.push_str(&format!("({})({});", include_str!("../../player-wrapper/chat.js"), serde_json::json!({
                "origin": url.origin().ascii_serialization(), "path": url.path(),
                "channel": login, "demo": false, "mode": "hosted",
                "css": include_str!("../../player-wrapper/chat.css")
            })));
        }
        let chat_parent = url.host_str().unwrap_or_default().to_owned();
        let chat_channel = login.to_owned();
        let label = format!("player-{id}");
        let origin = url.origin();
        WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
            .title(window_title(login, settings.demo, None))
            .inner_size(1180.0, 720.0).min_inner_size(430.0, 480.0)
            .focused(false)
            .initialization_script(adapter)
            .on_navigation(move |target| {
                // No navigation to arbitrary sites and no OS custom-scheme launches.
                // Twitch's own player origin is also allowed for iframe navigation.
                target.origin() == origin || (target.scheme() == "https" && target.host_str() == Some("player.twitch.tv"))
                    || allowed_chat_url(target, &chat_channel, &chat_parent)
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .on_download(|_, _| false)
            .build().map_err(|e| format!("Could not create player: {e}"))?;
        Ok(label)
    }
}

// Only the assigned official chat frame is allowed; it receives no native capability.
fn allowed_chat_url(url: &Url, channel: &str, parent: &str) -> bool {
    let pairs: Vec<_> = url.query_pairs().collect();
    url.scheme() == "https" && url.host_str() == Some("www.twitch.tv")
        && url.username().is_empty() && url.password().is_none() && url.port().is_none()
        && url.path() == format!("/embed/{channel}/chat") && url.fragment().is_none()
        && pairs.len() == 1 && pairs[0].0 == "parent" && pairs[0].1 == parent
}

pub fn window_title(login: &str, demo: bool, count: Option<ViewerCount>) -> String {
    let audience = if demo { String::new() } else {
        count.map(|value| format!(" · {}", value.label())).unwrap_or_default()
    };
    format!("{login}{audience} · MPD Viewer{}", if demo { " · SIMULATED" } else { "" })
}

pub fn quality(app: &AppHandle, label: &str, settings: &Settings) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        let value = serde_json::to_string(&settings.preferred_quality).map_err(|e| e.to_string())?;
        window.eval(format!("window.mpdSetQuality && window.mpdSetQuality({value});")).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn audio(app: &AppHandle, label: &str, settings: &Settings) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        let script = format!("window.mpdSetAudio && window.mpdSetAudio({}, {});", settings.volume, settings.muted);
        window.eval(&script).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_navigation_is_bound_to_assignment_and_parent() {
        let valid = "https://www.twitch.tv/embed/alpha/chat?parent=parent.mpdviewer.com";
        assert!(allowed_chat_url(&Url::parse(valid).unwrap(), "alpha", "parent.mpdviewer.com"));
        for invalid in [
            valid.replace("https:", "http:"), valid.replace("www.twitch.tv", "evil.example"),
            valid.replace("/alpha/", "/beta/"), valid.replace("/chat?", "/chat/other?"),
            valid.replace("parent.mpdviewer.com", "evil.example"), format!("{valid}&parent=evil.example"),
            format!("{valid}&extra=true"), format!("{valid}#fragment"),
            valid.replace("www.twitch.tv", "user@www.twitch.tv"),
            valid.replace("www.twitch.tv", "www.twitch.tv:444"),
        ] { assert!(!allowed_chat_url(&Url::parse(&invalid).unwrap(), "alpha", "parent.mpdviewer.com"), "{invalid}"); }
    }
    fn host(production: Option<&str>) -> Host {
        Host { local: Url::parse("http://localhost:4321/index.html").unwrap(),
            production: production.map(|url| Url::parse(url).unwrap()) }
    }
    #[test]
    fn hosted_bootstrap_replaces_old_assignments_and_converts_volume() {
        let host = host(Some("https://parent.mpdviewer.com/?channel=old&channel=other&active=old&volume=1&pauseInactive=true&keep=yes#old"));
        let settings = Settings { demo: false, volume: 25, muted: true, ..Settings::default() };
        let url = host.player_url("alpha", 7, &settings);
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(pairs.iter().filter(|(k, _)| k == "channel").count(), 1);
        for (key, value) in [("channel", "alpha"), ("active", "__mpd-native-pending__"),
            ("volume", "0.25"), ("pauseInactive", "false"), ("keep", "yes")] {
            assert!(pairs.iter().any(|(k, v)| k == key && v == value));
        }
        assert!(url.fragment().is_none());
        assert!(mpd_core::normalize_login("__mpd-native-pending__").is_err());
    }
    #[test]
    fn demo_keeps_bundled_fragment_protocol() {
        let host = host(Some("https://parent.mpdviewer.com/"));
        let url = host.player_url("alpha", 7, &Settings::default());
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.fragment(), Some("channel=alpha&session=7&volume=25&muted=false&demo=true&quality=auto"));
    }
    #[test]
    fn local_live_fallback_keeps_bundled_protocol() {
        let host = host(None);
        let settings = Settings { demo: false, ..Settings::default() };
        let url = host.player_url("alpha", 7, &settings);
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.fragment(), Some("channel=alpha&session=7&volume=25&muted=false&demo=false&quality=auto"));
    }
}


#[cfg(test)]
mod viewer_title_tests {
    use super::*;
    #[test]
    fn count_title_keeps_channel_and_brand_and_never_fakes_demo_counts() {
        assert_eq!(window_title("alpha", false, Some(ViewerCount { count: 1234, stale: false })),
            "alpha · 1,234 viewers · MPD Viewer");
        assert_eq!(window_title("alpha", false, Some(ViewerCount { count: 0, stale: true })),
            "alpha · 0 viewers (stale) · MPD Viewer");
        assert_eq!(window_title("alpha", false, None), "alpha · MPD Viewer");
        assert_eq!(window_title("alpha", true, Some(ViewerCount { count: 1234, stale: false })),
            "alpha · MPD Viewer · SIMULATED");
    }
}
