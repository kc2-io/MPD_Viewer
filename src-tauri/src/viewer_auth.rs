//! Explicit website sign-in, separate from the monitoring API's device grant.
//! Uses the existing default app browser profile; never inspects credentials.
#[cfg(windows)]
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(any(windows, test))]
fn allowed_navigation(url: &url::Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some("www.twitch.tv")
        && url.username().is_empty() && url.password().is_none() && url.port().is_none()
        && matches!(url.path(), "/login" | "/")
}

#[cfg(windows)]
pub fn open(app: &tauri::AppHandle) -> Result<(), String> {
    const LABEL: &str = "viewer-auth";
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show().map_err(|_| "Could not show Twitch sign-in.")?;
        window.unminimize().map_err(|_| "Could not restore Twitch sign-in.")?;
        return window.set_focus().map_err(|_| "Could not focus Twitch sign-in.".into());
    }
    let handle = app.clone();
    // No custom data directory: retain the profile already used by player windows.
    // This label has no capability and receives no player adapter or auth scripts.
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(
        "https://www.twitch.tv/login".parse().expect("Static Twitch URL is valid")))
        .title("Twitch sign-in · www.twitch.tv")
        .inner_size(600.0, 760.0).min_inner_size(480.0, 600.0)
        .on_navigation(move |url| {
            let allowed = allowed_navigation(url);
            if !allowed {
                if let Some(window) = handle.get_webview_window(LABEL) {
                    let _ = window.set_title("Twitch sign-in · unsupported navigation blocked");
                }
            }
            allowed
        })
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .build().map_err(|_| "Could not open Twitch sign-in. Try again after closing any existing sign-in window.".to_owned())?;
    Ok(())
}

#[cfg(not(windows))]
pub fn open(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("Viewer website sign-in is currently supported for testing on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::allowed_navigation;
    #[test]
    fn permits_only_official_website_sign_in_and_its_home_return() {
        for value in ["https://www.twitch.tv/login", "https://www.twitch.tv/",
            "https://www.twitch.tv/login?popup=true", "https://www.twitch.tv:443/login"] {
            assert!(allowed_navigation(&value.parse().unwrap()));
        }
    }
    #[test]
    fn denies_foreign_destinations_and_unreviewed_routes() {
        for value in ["http://www.twitch.tv/login", "https://twitch.tv/login",
            "https://www.twitch.tv.evil.example/login", "https://evil.example/login",
            "https://user@www.twitch.tv/login", "https://user:secret@www.twitch.tv/login",
            "https://www.twitch.tv:444/login", "https://id.twitch.tv/login",
            "https://www.twitch.tv/settings", "https://www.twitch.tv/signup",
            "https://www.twitch.tv/login/other", "https://www.twitch.tv/%6cogin",
            "javascript:alert(1)", "file:///C:/test.html", "about:blank"] {
            assert!(!allowed_navigation(&value.parse().unwrap()), "{value}");
        }
    }
}
