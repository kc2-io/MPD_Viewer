use std::net::TcpListener;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;
use crate::model::{Settings, ViewerCapabilities, ViewerCount};

use crate::viewer_mode::{self, ViewerMode};

const TWITCH_PAGE_ROOT: &str = "https://www.twitch.tv/";

pub struct Host {
    local: Url,
    mode: ViewerMode,
    production: Option<Url>,
}
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
                    #[cfg(feature = "e2e-tests")]
                    let asset_name = path.to_owned(); // Only the fixed asset allowlist above.
                    #[cfg(feature = "e2e-tests")]
                    {
                        let session = request.url().split_once('?').and_then(|(_, query)| query.strip_prefix("e2e_session="))
                            .filter(|value| value.len() <= 20 && value.bytes().all(|b| b.is_ascii_digit())).unwrap_or("none");
                        eprintln!("E2E asset begin {asset_name} session={session}");
                    }
                    #[cfg(feature = "e2e-tests")]
                    let body = if mime.starts_with("text/html") { crate::e2e::scope_wrapper_assets(body, request.url()) } else { body.into() };
                    let mut response = tiny_http::Response::from_string(body);
                    for (name, value) in [("Content-Type", mime), ("Cache-Control", "no-store"),
                        ("X-Content-Type-Options", "nosniff"), ("Referrer-Policy", "strict-origin-when-cross-origin")] {
                        if let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) { response.add_header(header); }
                    }
                    #[cfg(feature = "e2e-tests")]
                    response.add_header(tiny_http::Header::from_bytes("Content-Security-Policy",
                        "default-src 'self'; script-src 'self'; style-src 'self'; connect-src ipc: http://ipc.localhost https://ipc.localhost; frame-src 'none'; object-src 'none'").unwrap());
                    #[cfg(not(feature = "e2e-tests"))]
                    let _ = request.respond(response);
                    #[cfg(feature = "e2e-tests")]
                    {
                        let respond_ok = request.respond(response).is_ok();
                        eprintln!("E2E asset end {asset_name}: respond_ok={respond_ok}");
                    }
                } else { let _ = request.respond(tiny_http::Response::empty(404)); }
            }
        })?;
        {
            let config: serde_json::Value = serde_json::from_str(include_str!("../player-origin.json"))?;
            let production = config.get("url").and_then(|v| v.as_str()).map(Url::parse).transpose()?;
            if let Some(url) = &production {
                if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
                    return Err(std::io::Error::other("Production player URL must be HTTPS with no credentials.").into());
                }
            }
            Ok(Self { local: Url::parse(&format!("http://localhost:{port}/index.html"))?, production, mode: viewer_mode::selected() })
        }
    }

    pub fn label(&self, id: u64, demo: bool) -> String {
        if demo || self.mode == ViewerMode::Embedded { format!("player-{id}") }
        else { format!("twitch-page-{id}") }
    }

    pub fn capabilities(&self, demo: bool) -> ViewerCapabilities {
        if demo {
            return ViewerCapabilities::wrapper("bundled-demo");
        }
        match self.mode {
            ViewerMode::TwitchPage => ViewerCapabilities::twitch_page(),
            ViewerMode::Embedded => ViewerCapabilities::wrapper("twitch-embed"),
        }
    }

    pub fn diagnostic_origin(&self, demo: bool) -> String {
        if demo { return self.local.as_str().to_owned(); }
        match self.mode {
            ViewerMode::TwitchPage => TWITCH_PAGE_ROOT.to_owned(),
            ViewerMode::Embedded => self.production.as_ref().unwrap_or(&self.local).as_str().to_owned(),
        }
    }

    fn player_url(&self, login: &str, id: u64, settings: &Settings) -> Url {
        if settings.demo {
            let mut url = self.local.clone();
            #[cfg(feature = "e2e-tests")]
            url.query_pairs_mut().append_pair("e2e_session", &id.to_string());
            let fragment = url::form_urlencoded::Serializer::new(String::new())
                .append_pair("channel", login).append_pair("session", &id.to_string())
                .append_pair("volume", &settings.volume.to_string()).append_pair("muted", &settings.muted.to_string())
                .append_pair("demo", "true").append_pair("quality", &settings.preferred_quality).finish();
            url.set_fragment(Some(&fragment));
            return url;
        }
        if self.mode == ViewerMode::TwitchPage {
            #[cfg(feature = "e2e-tests")]
            return crate::e2e::url(&format!("/web/{login}"));
            #[cfg(not(feature = "e2e-tests"))]
            {
            let mut url = Url::parse(TWITCH_PAGE_ROOT).expect("static Twitch channel-page origin");
            url.path_segments_mut().expect("Twitch root is a base URL").push(login);
            return url;
            }
        }
        #[cfg(feature = "e2e-tests")]
        {
            // Test builds never load the hosted player or Twitch SDK.
            let mut simulated = settings.clone(); simulated.demo = true;
            return self.player_url(login, id, &simulated);
        }
        #[cfg(not(feature = "e2e-tests"))]
        {
            let mut url = self.production.as_ref().unwrap_or(&self.local).clone();
            if self.production.is_some() {
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
                    .append_pair("demo", "false").append_pair("quality", &settings.preferred_quality).finish();
                url.set_fragment(Some(&fragment));
            }
            url
        }
    }
    pub fn open(&self, app: &AppHandle, login: &str, id: u64, settings: &Settings) -> Result<String, String> {
        #[cfg(feature = "e2e-tests")]
        if crate::e2e::fail_open(login) { return Err("Simulated native viewer-open failure".into()); }
        let url = self.player_url(login, id, settings);
        #[cfg(not(feature = "e2e-tests"))]
        let chat_parent = url.host_str().unwrap_or_default().to_owned();
        #[cfg(not(feature = "e2e-tests"))]
        let chat_channel = login.to_owned();
        let label = self.label(id, settings.demo);
        let origin = url.origin();
        #[cfg(not(feature = "e2e-tests"))]
        let demo = settings.demo;
        #[cfg(not(feature = "e2e-tests"))]
        let mode = self.mode;
        let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url.clone()))
            .title(window_title(login, settings.demo, None))
            .inner_size(1180.0, 720.0).min_inner_size(430.0, 480.0)
            .focused(false);
        #[cfg(feature = "e2e-tests")]
        let builder = crate::e2e::isolate(builder);
        #[cfg(feature = "e2e-tests")]
        let builder = if crate::e2e_probe::enabled() {
            builder.initialization_script(crate::e2e_probe::script(&label, id))
        } else { builder };
        #[cfg(not(feature = "e2e-tests"))]
        let builder = if self.mode == ViewerMode::Embedded && !settings.demo && self.production.is_some() {
            let mut adapter = format!("({})({}, {});", include_str!("../hosted-player-adapter.js"), serde_json::json!({
                "origin": url.origin().ascii_serialization(), "path": url.path(),
                "channel": login, "session": id, "volume": settings.volume, "muted": settings.muted,
                "quality": settings.preferred_quality
            }), include_str!("../../player-wrapper/quality.js"));
            adapter.push_str(&format!("({})({});", include_str!("../../player-wrapper/chat.js"), serde_json::json!({
                "origin": url.origin().ascii_serialization(), "path": url.path(),
                "channel": login, "demo": false, "mode": "hosted",
                "css": include_str!("../../player-wrapper/chat.css")
            })));
            builder.initialization_script(adapter)
        } else { builder };
        builder
            .on_navigation(move |target| {
                #[cfg(feature = "e2e-tests")]
                return target.origin() == origin;
                #[cfg(not(feature = "e2e-tests"))]
                allowed_navigation(target, &origin, demo, mode, &chat_channel, &chat_parent)
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .on_download(|_, _| false)
            .build().map_err(|e| format!("Could not create player: {e}"))?;
        Ok(label)
    }
}

fn official_twitch_navigation(url: &Url) -> bool {
    url.scheme() == "https" && matches!(url.host_str(), Some("www.twitch.tv" | "player.twitch.tv"))
        && url.username().is_empty() && url.password().is_none() && url.port().is_none()
}

fn allowed_navigation(target: &Url, origin: &url::Origin, demo: bool, mode: ViewerMode, channel: &str, parent: &str) -> bool {
    if demo {
        return target.origin() == origin.clone()
            || (target.scheme() == "https" && target.host_str() == Some("player.twitch.tv"));
    }
    if mode == ViewerMode::TwitchPage {
        // This is a top-level first-party Twitch document, not an embed. Keep
        // arbitrary sites, credential-bearing URLs, ports and custom schemes out.
        return official_twitch_navigation(target);
    }
    target.origin() == origin.clone()
        || (target.scheme() == "https" && target.host_str() == Some("player.twitch.tv"))
        || allowed_chat_url(target, channel, parent)
}

// Only the assigned official chat frame is allowed; it receives no native
// capability. The OS theme may add the single `darkpopout` flag (frozen in
// docs/dark-mode/theme-contract.md) to the otherwise exact light shape. The
// query is compared byte-for-byte, so only the two raw strings the chat
// helper emits can navigate; encoded or reordered spellings never match.
fn allowed_chat_url(url: &Url, channel: &str, parent: &str) -> bool {
    if url.scheme() != "https" || url.host_str() != Some("www.twitch.tv")
        || !url.username().is_empty() || url.password().is_some() || url.port().is_some()
        || url.path() != format!("/embed/{channel}/chat") || url.fragment().is_some() {
        return false;
    }
    let Some(query) = url.query() else { return false };
    query == format!("parent={parent}") || query == format!("parent={parent}&darkpopout")
}

pub fn window_title(login: &str, demo: bool, count: Option<ViewerCount>) -> String {
    let audience = if demo { String::new() } else {
        count.map(|value| format!(" · {}", value.label())).unwrap_or_default()
    };
    format!("{login}{audience} · MPD Viewer{}", if demo { " · SIMULATED" } else { "" })
}

pub fn quality(app: &AppHandle, label: &str, settings: &Settings) -> Result<(), String> {
    if !label.starts_with("player-") { return Ok(()); }
    if let Some(window) = app.get_webview_window(label) {
        let value = serde_json::to_string(&settings.preferred_quality).map_err(|e| e.to_string())?;
        window.eval(format!("window.mpdSetQuality && window.mpdSetQuality({value});")).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn audio(app: &AppHandle, label: &str, settings: &Settings) -> Result<(), String> {
    if !label.starts_with("player-") { return Ok(()); }
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
        let light = "https://www.twitch.tv/embed/alpha/chat?parent=parent.mpdviewer.com";
        let dark = "https://www.twitch.tv/embed/alpha/chat?parent=parent.mpdviewer.com&darkpopout";
        for valid in [light, dark] {
            assert!(allowed_chat_url(&Url::parse(valid).unwrap(), "alpha", "parent.mpdviewer.com"), "{valid}");
        }
        let parent = "parent.mpdviewer.com";
        for invalid in [
            light.replace("https:", "http:"), light.replace("www.twitch.tv", "evil.example"),
            light.replace("/alpha/", "/beta/"), light.replace("/chat?", "/chat/other?"),
            light.replace(parent, "evil.example"), format!("{light}&parent=evil.example"),
            format!("{light}&extra=true"), format!("{light}#fragment"),
            light.replace("www.twitch.tv", "user@www.twitch.tv"),
            light.replace("www.twitch.tv", "www.twitch.tv:444"),
            // reversed order, extra flag value forms, unknown/theme/extra keys
            format!("https://www.twitch.tv/embed/alpha/chat?darkpopout&parent={parent}"),
            format!("{light}&darkpopout=1"), format!("{light}&darkpopout=1&darkpopout"),
            format!("{light}&darkpopout="), format!("{dark}&theme=dark"), format!("{light}&theme=dark"),
            format!("{light}&darkpopout=&extra=true"),
            // empty segments and leading/trailing separators
            format!("https://www.twitch.tv/embed/alpha/chat?parent={parent}&&darkpopout"),
            format!("https://www.twitch.tv/embed/alpha/chat?&parent={parent}"),
            format!("https://www.twitch.tv/embed/alpha/chat?parent={parent}&"),
            // percent-encoded keys or parent spellings never match the raw forms
            format!("https://www.twitch.tv/embed/alpha/chat?p%61rent={parent}&darkpopout"),
            "https://www.twitch.tv/embed/alpha/chat?parent=parent%2Empdviewer.com".to_string(),
            // case-spelled key, stray '?', and a missing query
            format!("https://www.twitch.tv/embed/alpha/chat?Parent={parent}"),
            format!("https://www.twitch.tv/embed/alpha/chat?parent={parent}?darkpopout"),
            "https://www.twitch.tv/embed/alpha/chat".to_owned(),
        ] { assert!(!allowed_chat_url(&Url::parse(&invalid).unwrap(), "alpha", parent), "{invalid}"); }
    }
    fn host(production: Option<&str>) -> Host {
        Host { local: Url::parse("http://localhost:4321/index.html").unwrap(),
            mode: ViewerMode::Embedded,
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
    fn runtime_mode_keeps_remote_pages_outside_the_wrapper_boundary() {
        let mut h = host(Some("https://parent.mpdviewer.com/"));
        let origin = Url::parse("https://parent.mpdviewer.com/").unwrap().origin();
        let page = Url::parse("https://www.twitch.tv/alpha").unwrap();
        let wrapper = Url::parse("https://parent.mpdviewer.com/").unwrap();
        assert_eq!(h.label(3, false), "player-3");
        assert!(h.capabilities(false).media_controls);
        assert!(!allowed_navigation(&page, &origin, false, h.mode, "alpha", "parent.mpdviewer.com"));
        assert!(allowed_navigation(&wrapper, &origin, false, h.mode, "alpha", "parent.mpdviewer.com"));
        h.mode = ViewerMode::TwitchPage;
        assert_eq!(h.label(3, false), "twitch-page-3");
        assert!(!h.capabilities(false).media_controls);
        assert!(allowed_navigation(&page, &page.origin(), false, h.mode, "alpha", "www.twitch.tv"));
        assert!(!allowed_navigation(&wrapper, &page.origin(), false, h.mode, "alpha", "www.twitch.tv"));
        assert_eq!(h.label(3, true), "player-3");
        assert!(h.capabilities(true).media_controls);
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
    #[test]
    fn default_backend_opens_a_top_level_channel_page_without_embed_parameters() {
        let mut host = host(None);
        host.mode = ViewerMode::TwitchPage;
        let settings = Settings { demo: false, ..Settings::default() };
        let url = host.player_url("alpha", 7, &settings);
        assert_eq!(url.as_str(), "https://www.twitch.tv/alpha");
        assert_eq!(host.label(7, false), "twitch-page-7");
        assert_eq!(host.label(7, true), "player-7");
        let capabilities = host.capabilities(false);
        assert_eq!(capabilities.backend, "twitch-page");
        assert!(!capabilities.telemetry);
        assert!(!capabilities.media_controls);
        assert!(capabilities.twitch_channel_page);
    }
    #[test]
    fn channel_page_navigation_stays_on_credential_free_first_party_https() {
        for valid in ["https://www.twitch.tv/alpha", "https://www.twitch.tv/login",
            "https://www.twitch.tv/directory/category/science-and-technology?sort=VIEWER_COUNT"] {
            assert!(official_twitch_navigation(&Url::parse(valid).unwrap()), "{valid}");
        }
        assert!(official_twitch_navigation(&Url::parse("https://player.twitch.tv/?channel=alpha&parent=www.twitch.tv").unwrap()));
        for invalid in ["http://www.twitch.tv/alpha", "https://evil.example/alpha",
            "https://www.twitch.tv.evil.example/alpha", "https://user@www.twitch.tv/alpha",
            "https://www.twitch.tv:444/alpha", "https://id.twitch.tv/oauth2/authorize",
            "file:///C:/alpha", "about:blank"] {
            assert!(!official_twitch_navigation(&Url::parse(invalid).unwrap()), "{invalid}");
        }
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
