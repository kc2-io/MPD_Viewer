//! A single device-authorization window in the existing viewer browser profile.
use tauri::Manager;
#[cfg(windows)]
use tauri::{WebviewUrl, WebviewWindowBuilder};

pub fn ensure_supported() -> Result<(), String> {
    if cfg!(windows) { Ok(()) }
    else { Err("Connecting Twitch inside the viewer is currently available on Windows only.".into()) }
}

pub fn label(epoch: u64) -> String { format!("viewer-auth-{epoch}") }

fn official_host(url: &url::Url, host: &str) -> bool {
    url.scheme() == "https" && url.host_str() == Some(host)
        && url.username().is_empty() && url.password().is_none() && url.port().is_none()
}

fn official(url: &url::Url) -> bool { official_host(url, "www.twitch.tv") }

pub struct Activation {
    url: url::Url, code: String,
    return_url: std::sync::Mutex<Option<url::Url>>,
}
impl Activation {
    pub fn parse(text: &str, code: &str) -> Result<Self, String> {
        let invalid = || "Twitch returned an unexpected activation link. Connect again.".to_owned();
        if text.len() > 1024 || code.is_empty() || code.len() > 32 || !code.bytes().all(|c| c.is_ascii_alphanumeric()) {
            return Err(invalid());
        }
        let url = url::Url::parse(text).map_err(|_| invalid())?;
        let result = Self { url, code: code.to_owned(), return_url: std::sync::Mutex::new(None) };
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
    fn authorization_query(&self, url: &url::Url) -> bool {
        // Twitch's activation page supplies this continuation. Bind it to our
        // app and pending user code; never accept extra scopes or foreign returns.
        let mut seen = std::collections::HashSet::new();
        for (key, value) in url.query_pairs() {
            if !seen.insert(key.clone()) { return false; }
            let valid = match key.as_ref() {
                "client_id" => value == crate::model::DEFAULT_CLIENT_ID,
                "user_code" => value == self.code,
                "scope" => value.is_empty(),
                "device_code" => !value.is_empty() && value.len() <= 1024,
                "force_verify" => matches!(value.as_ref(), "true" | "false"),
                "response_type" => !value.is_empty() && value.len() <= 64
                    && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'),
                "redirect_uri" => url::Url::parse(&value).is_ok_and(|target|
                    official(&target) && target.fragment().is_none()),
                _ => false,
            };
            if !valid { return false; }
        }
        seen.len() == 7
    }
    #[cfg(any(windows, test))]
    fn is_return(&self, url: &url::Url) -> bool {
        official(url) && url.fragment().is_none()
            && (url.path() != "/activate" || self.activation_query(url, false))
            && self.return_url.lock().is_ok_and(|expected| expected.as_ref() == Some(url))
    }
    #[cfg(any(windows, test))]
    fn allows(&self, url: &url::Url) -> bool {
        let authorization_route = (official_host(url, "id.twitch.tv") && url.path() == "/oauth2/authorize")
            || (official_host(url, "auth.twitch.tv") && url.path() == "/authorize");
        if authorization_route {
            if url.as_str().len() > 4096 || url.fragment().is_some() || !self.authorization_query(url) {
                return false;
            }
            // This first-party return was checked above. Keep exactly one target
            // for this window/attempt; do not turn its origin into a path wildcard.
            let Some(target) = url.query_pairs().find(|(key, _)| key == "redirect_uri")
                .and_then(|(_, value)| url::Url::parse(&value).ok()) else { return false; };
            let Ok(mut expected) = self.return_url.lock() else { return false; };
            return match expected.as_ref() {
                Some(current) => current == &target,
                None => { *expected = Some(target); true },
            };
        }
        official(url) && match url.path() {
            "/login" | "/" => true,
            // An exact return must never bypass the existing same-code check.
            "/activate" => url.fragment().is_none() && self.activation_query(url, false),
            _ => self.return_url.lock().is_ok_and(|expected| expected.as_ref() == Some(url)),
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
    let activation = std::sync::Arc::new(activation);
    let completion = activation.clone();
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
        .on_page_load(move |window, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished
                && completion.is_return(payload.url()) {
                if let Some(controller) = window.app_handle().try_state::<crate::controller::Handle>() {
                    let tx = controller.tx.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = tx.send(crate::controller::Message::AuthReturnLoaded { epoch }).await;
                    });
                }
            }
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
    fn authorization_url() -> url::Url {
        let mut url: url::Url = "https://id.twitch.tv/oauth2/authorize".parse().unwrap();
        url.query_pairs_mut().extend_pairs([
            ("client_id", crate::model::DEFAULT_CLIENT_ID), ("user_code", "ABCDEFGH"),
            ("device_code", "opaque-test-device-code"), ("scope", ""),
            ("response_type", "device_code"), ("force_verify", "false"),
            ("redirect_uri", "https://www.twitch.tv/activate"),
        ]);
        url
    }
    #[test]
    fn authorization_continuation_is_bound_and_has_no_extra_authority() {
        let activation = Activation::parse(GOOD, "ABCDEFGH").unwrap();
        let good = authorization_url();
        assert!(activation.allows(&good));
        let keys: Vec<_> = good.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
        // Every observed field is mandatory, unique, and checked independently.
        for (key, _) in &keys {
            let mut missing = good.clone();
            missing.set_query(None);
            missing.query_pairs_mut().extend_pairs(keys.iter().filter(|(k, _)| k != key));
            assert!(!activation.allows(&missing), "missing {key}");
            let mut duplicate = good.clone();
            duplicate.query_pairs_mut().append_pair(key, &keys.iter().find(|(k, _)| k == key).unwrap().1);
            assert!(!activation.allows(&duplicate), "duplicate {key}");
        }
        for (key, bad) in [
            ("client_id", "other-app"), ("user_code", "OTHER"), ("scope", "chat:edit"),
            ("device_code", ""), ("response_type", ""), ("response_type", "invalid value"),
            ("force_verify", "yes"), ("redirect_uri", "https://evil.example/"),
            ("redirect_uri", "https://www.twitch.tv.evil.example/activate"),
            ("redirect_uri", "http://www.twitch.tv/activate"),
            ("redirect_uri", "https://user@www.twitch.tv/activate"),
            ("redirect_uri", "https://www.twitch.tv:444/activate"),
            ("redirect_uri", "https://www.twitch.tv/activate#fragment"),
        ] {
            let mut url = good.clone();
            url.set_query(None);
            url.query_pairs_mut().extend_pairs(keys.iter().map(|(k, v)| (k.as_str(), if k == key { bad } else { v.as_str() })));
            assert!(!activation.allows(&url), "invalid {key}");
        }
        for text in [
            good.as_str().replace("https:", "http:"),
            good.as_str().replace("id.twitch.tv", "id.twitch.tv.evil.example"),
            good.as_str().replace("id.twitch.tv", "user@id.twitch.tv"),
            good.as_str().replace("id.twitch.tv", "id.twitch.tv:444"),
            good.as_str().replace("/oauth2/authorize", "/oauth2/token"),
            format!("{good}&state=extra"), format!("{good}#fragment"),
            format!("{good}&device_code={}", "x".repeat(4096)),
        ] { assert!(!activation.allows(&text.parse().unwrap())); }
    }
    #[test]
    fn authentication_frontend_uses_the_same_bound_query_policy() {
        let activation = Activation::parse(GOOD, "ABCDEFGH").unwrap();
        let mut frontend = authorization_url();
        frontend.set_host(Some("auth.twitch.tv")).unwrap();
        frontend.set_path("/authorize");
        assert!(activation.allows(&frontend));
        for text in [
            frontend.as_str().replace("auth.twitch.tv", "auth.twitch.tv.evil.example"),
            frontend.as_str().replace("auth.twitch.tv", "user@auth.twitch.tv"),
            frontend.as_str().replace("auth.twitch.tv", "auth.twitch.tv:444"),
            frontend.as_str().replace("https:", "http:"),
            frontend.as_str().replace("/authorize?", "/authorize/redirect?"),
            frontend.as_str().replace("user_code=ABCDEFGH", "user_code=OTHER"),
            frontend.as_str().replace(crate::model::DEFAULT_CLIENT_ID, "other-client"),
            frontend.as_str().replace("scope=", "scope=chat%3Aedit"),
            format!("{frontend}&user_code=ABCDEFGH"), format!("{frontend}#fragment"),
            "https://auth.twitch.tv/authorize".into(),
        ] { assert!(!activation.allows(&text.parse().unwrap())); }
    }
    #[test]
    fn return_navigation_is_exact_and_bound_to_an_accepted_authorization() {
        let activation = Activation::parse(GOOD, "ABCDEFGH").unwrap();
        let callback: url::Url = "https://www.twitch.tv/test-only/complete".parse().unwrap();
        let authorize = |target: &str| {
            let mut url = authorization_url();
            let pairs: Vec<_> = url.query_pairs().map(|(k, v)|
                (k.to_string(), if k == "redirect_uri" { target.to_owned() } else { v.into_owned() })).collect();
            url.set_query(None);
            url.query_pairs_mut().extend_pairs(pairs);
            url
        };
        assert!(!activation.allows(&callback));
        assert!(!activation.is_return(&callback));
        let mut invalid = authorize(callback.as_str());
        invalid.query_pairs_mut().append_pair("scope", "chat:edit");
        assert!(!activation.allows(&invalid));
        assert!(!activation.allows(&callback));
        let valid = authorize(callback.as_str());
        assert!(activation.allows(&valid));
        assert!(activation.allows(&callback));
        assert!(activation.is_return(&callback));
        assert!(!activation.is_return(&"https://www.twitch.tv/login".parse().unwrap()));
        assert!(activation.allows(&valid));
        assert!(!activation.allows(&authorize("https://www.twitch.tv/another-return")));
        for text in [
            format!("{callback}?extra=1"), format!("{callback}#fragment"), format!("{callback}/child"),
            "https://www.twitch.tv/test-only/other".into(),
            "https://www.twitch.tv/another-return".into(),
            callback.as_str().replace("www.twitch.tv", "www.twitch.tv.evil.example"),
            callback.as_str().replace("https:", "http:"),
            callback.as_str().replace("www.twitch.tv", "user@www.twitch.tv"),
            callback.as_str().replace("www.twitch.tv", "www.twitch.tv:444"),
        ] { assert!(!activation.allows(&text.parse().unwrap())); }
        let next_attempt = Activation::parse(GOOD, "ABCDEFGH").unwrap();
        assert!(!next_attempt.allows(&callback));
    }
    #[test]
    fn recorded_return_cannot_bypass_activation_query_checks() {
        for target in [
            "https://www.twitch.tv/activate?device-code=OTHER",
            "https://www.twitch.tv/activate?extra=unexpected",
            "https://www.twitch.tv/activate?public=false",
        ] {
            let activation = Activation::parse(GOOD, "ABCDEFGH").unwrap();
            let mut authorize = authorization_url();
            let pairs: Vec<_> = authorize.query_pairs().map(|(k, v)|
                (k.to_string(), if k == "redirect_uri" { target.to_owned() } else { v.into_owned() })).collect();
            authorize.set_query(None);
            authorize.query_pairs_mut().extend_pairs(pairs);
            assert!(activation.allows(&authorize));
            assert!(!activation.allows(&target.parse().unwrap()));
            assert!(!activation.is_return(&target.parse().unwrap()));
        }
    }
    #[test]
    fn window_identity_changes_between_attempts() {
        assert_ne!(label(1), label(2));
        assert!(!label(1).starts_with("player-"));
        assert_ne!(label(1), "main");
    }
}
