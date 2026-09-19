//! A single device-authorization window in the existing viewer browser profile.
use tauri::Manager;
#[cfg(windows)]
use tauri::{WebviewUrl, WebviewWindowBuilder};

pub fn ensure_supported() -> Result<(), String> {
    if cfg!(windows) { Ok(()) }
    else { Err("Connecting Twitch inside the viewer is currently available on Windows only.".into()) }
}

pub fn label(epoch: u64) -> String { format!("viewer-auth-{epoch}") }

fn official(url: &url::Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some("www.twitch.tv")
        && url.username().is_empty() && url.password().is_none() && url.port().is_none()
}

pub struct Activation { url: url::Url, code: String }
impl Activation {
    pub fn parse(text: &str, code: &str) -> Result<Self, String> {
        let invalid = || "Twitch returned an unexpected activation link. Connect again.".to_owned();
        if text.len() > 1024 || code.is_empty() || code.len() > 32 || !code.bytes().all(|c| c.is_ascii_alphanumeric()) {
            return Err(invalid());
        }
        let url = url::Url::parse(text).map_err(|_| invalid())?;
        let result = Self { url, code: code.to_owned() };
        if !official(&result.url) || result.url.path() != "/activate" || result.url.fragment().is_some()
            || !result.activation_query(&result.url, true) { return Err(invalid()); }
        Ok(result)
    }
    fn activation_query(&self, url: &url::Url, required: bool) -> bool {
        let mut public = 0;
        let mut code = 0;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "public" if value == "true" => public += 1,
                "device-code" if value == self.code => code += 1,
                _ => return false,
            }
        }
        public <= 1 && code <= 1 && (!required || code == 1)
    }
    #[cfg(any(windows, test))]
    fn allows(&self, url: &url::Url) -> bool {
        official(url) && match url.path() {
            "/login" | "/" => true,
            "/activate" => url.fragment().is_none() && self.activation_query(url, false),
            _ => false,
        }
    }
}

pub fn close(app: &tauri::AppHandle, epoch: u64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label(epoch)) {
        window.destroy().map_err(|_| "Could not close the previous Twitch connection window. Close it and try again.".to_owned())?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn open(app: &tauri::AppHandle, epoch: u64, activation: Activation) -> Result<(), String> {
    let label = label(epoch);
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|_| "Could not show Twitch connection.")?;
        window.unminimize().map_err(|_| "Could not restore Twitch connection.")?;
        return window.set_focus().map_err(|_| "Could not focus Twitch connection.".into());
    }
    let handle = app.clone();
    let window_label = label.clone();
    // Preserve the default viewer profile. No capability, adapter, or auth script.
    WebviewWindowBuilder::new(app, &label, WebviewUrl::External(activation.url.clone()))
        .title("Connect Twitch · www.twitch.tv")
        .inner_size(600.0, 760.0).min_inner_size(480.0, 600.0)
        .on_navigation(move |url| {
            let allowed = activation.allows(url);
            if !allowed {
                if let Some(window) = handle.get_webview_window(&window_label) {
                    let _ = window.set_title("Connect Twitch · unsupported navigation blocked");
                }
            }
            allowed
        })
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .build().map_err(|_| "Could not open Twitch connection. Close any previous connection window and try again.".to_owned())?;
    Ok(())
}

#[cfg(not(windows))]
pub fn open(_app: &tauri::AppHandle, _epoch: u64, _activation: Activation) -> Result<(), String> {
    ensure_supported()
}

#[cfg(test)]
mod tests {
    use super::*;
    const GOOD: &str = "https://www.twitch.tv/activate?public=true&device-code=ABCDEFGH";
    #[test]
    fn activation_is_bound_to_the_pending_device_code() {
        assert!(Activation::parse(GOOD, "ABCDEFGH").is_ok());
        for text in [GOOD.replace("ABCDEFGH", "DIFFERENT"), format!("{GOOD}&device-code=ABCDEFGH"),
            format!("{GOOD}&public=true"), format!("{GOOD}&next=https://evil.example"),
            format!("{GOOD}#fragment"), GOOD.replace("public=true", "public=false"),
            GOOD.replace("https:", "http:"), GOOD.replace("www.twitch.tv", "www.twitch.tv.evil.example"),
            GOOD.replace("www.twitch.tv", "user@www.twitch.tv"), GOOD.replace("www.twitch.tv", "www.twitch.tv:444"),
            GOOD.replace("/activate", "/login")] {
            assert!(Activation::parse(&text, "ABCDEFGH").is_err());
        }
        assert!(Activation::parse(GOOD, "").is_err());
    }
    #[test]
    fn accepts_live_activation_shape_without_optional_public_flag() {
        let live = "https://www.twitch.tv/activate?device-code=ABCDEFGH";
        let activation = Activation::parse(live, "ABCDEFGH").unwrap();
        assert!(activation.allows(&live.parse().unwrap()));
        assert!(Activation::parse("https://www.twitch.tv/activate", "ABCDEFGH").is_err());
        assert!(Activation::parse("https://www.twitch.tv/activate?public=true", "ABCDEFGH").is_err());
        assert!(Activation::parse(live, "OTHER").is_err());
        assert!(Activation::parse(&format!("{live}&device-code=ABCDEFGH"), "ABCDEFGH").is_err());
        assert!(Activation::parse(&format!("{live}&next=https://evil.example"), "ABCDEFGH").is_err());
    }
    #[test]
    fn navigation_stays_on_reviewed_routes_and_the_same_attempt() {
        let activation = Activation::parse(GOOD, "ABCDEFGH").unwrap();
        for text in [GOOD, "https://www.twitch.tv/login", "https://www.twitch.tv/", "https://www.twitch.tv/activate"] {
            assert!(activation.allows(&text.parse().unwrap()));
        }
        for text in ["https://www.twitch.tv/activate?device-code=OTHER", "https://www.twitch.tv/activate?public=true&public=true",
            "https://www.twitch.tv/settings", "https://id.twitch.tv/oauth2/authorize", "https://evil.example/login",
            "http://www.twitch.tv/login", "https://www.twitch.tv:444/login", "https://user@www.twitch.tv/login",
            "https://www.twitch.tv.evil.example/login", "file:///C:/test.html", "about:blank"] {
            assert!(!activation.allows(&text.parse().unwrap()));
        }
    }
    #[test]
    fn window_identity_changes_between_attempts() {
        assert_ne!(label(1), label(2));
        assert!(!label(1).starts_with("player-"));
        assert_ne!(label(1), "main");
    }
}
