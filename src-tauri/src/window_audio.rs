#[cfg(any(windows, target_os = "linux", test))]
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tauri::WebviewWindow;

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(any(windows, target_os = "linux", test))]
enum CallbackState {
    Pending,
    Complete(Result<(), String>),
    Cancelled,
}

#[cfg(any(windows, target_os = "linux", test))]
type CallbackCompletion = Arc<(Mutex<CallbackState>, Condvar)>;

#[cfg(any(windows, target_os = "linux", test))]
fn new_completion() -> CallbackCompletion {
    Arc::new((Mutex::new(CallbackState::Pending), Condvar::new()))
}

/// Holding the state lock across the native setter makes timeout cancellation
/// authoritative: a callback that has not started before cancellation cannot
/// apply later, while a setter already in progress must publish its result.
#[cfg(any(windows, target_os = "linux", test))]
fn complete_if_pending(
    completion: &CallbackCompletion,
    apply: impl FnOnce() -> Result<(), String>,
) {
    let (state, wake) = &**completion;
    let Ok(mut state) = state.lock() else { return };
    if matches!(*state, CallbackState::Pending) {
        *state = CallbackState::Complete(apply());
        wake.notify_one();
    }
}

#[cfg(any(windows, target_os = "linux", test))]
fn wait_for_completion(completion: &CallbackCompletion, timeout: Duration) -> Result<(), String> {
    let (state, wake) = &**completion;
    let state = state
        .lock()
        .map_err(|_| "Native window mute callback failed.".to_owned())?;
    let (mut state, _) = wake
        .wait_timeout_while(state, timeout, |state| {
            matches!(*state, CallbackState::Pending)
        })
        .map_err(|_| "Native window mute callback failed.".to_owned())?;
    match &*state {
        CallbackState::Complete(result) => result.clone(),
        CallbackState::Pending => {
            *state = CallbackState::Cancelled;
            Err("Native window mute did not complete in time.".into())
        }
        CallbackState::Cancelled => Err("Native window mute was cancelled.".into()),
    }
}

pub const fn supported() -> bool {
    cfg!(any(windows, target_os = "linux"))
}

/// Apply whole-webview mute through the native browser engine. A successfully
/// queued callback is not success: the caller waits for platform confirmation.
pub fn set_muted(window: &WebviewWindow, muted: bool) -> Result<(), String> {
    #[cfg(any(windows, target_os = "linux"))]
    {
        let completion = new_completion();
        let callback = completion.clone();
        window
            .with_webview(move |webview| {
                complete_if_pending(&callback, || set_platform_muted(webview, muted));
            })
            .map_err(|error| format!("Could not schedule native window mute: {error}"))?;
        wait_for_completion(&completion, CALLBACK_TIMEOUT)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (window, muted, CALLBACK_TIMEOUT);
        Err("Native page-window mute is unavailable on this platform.".into())
    }
}

#[cfg(target_os = "linux")]
fn set_platform_muted(webview: tauri::webview::PlatformWebview, muted: bool) -> Result<(), String> {
    use webkit2gtk::WebViewExt;
    webview.inner().set_is_muted(muted);
    Ok(())
}

#[cfg(windows)]
fn set_platform_muted(webview: tauri::webview::PlatformWebview, muted: bool) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
    use windows_core::Interface;
    unsafe {
        let core = webview
            .controller()
            .CoreWebView2()
            .map_err(|error| format!("Could not access WebView2: {error}"))?;
        let audio = core.cast::<ICoreWebView2_8>().map_err(|error| {
            format!("This WebView2 runtime does not support window mute: {error}")
        })?;
        audio
            .SetIsMuted(muted)
            .map_err(|error| format!("Could not set WebView2 mute: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;

    #[test]
    fn compile_time_capability_matches_supported_targets() {
        assert_eq!(super::supported(), cfg!(any(windows, target_os = "linux")));
    }

    #[test]
    fn completion_reports_an_applied_native_result() {
        let completion = super::new_completion();
        super::complete_if_pending(&completion, || Ok(()));
        assert!(super::wait_for_completion(&completion, Duration::ZERO).is_ok());
    }

    #[test]
    fn callback_arriving_after_timeout_cannot_apply_mute() {
        let completion = super::new_completion();
        assert!(super::wait_for_completion(&completion, Duration::ZERO).is_err());
        let applied = Arc::new(AtomicBool::new(false));
        let callback_applied = applied.clone();
        super::complete_if_pending(&completion, || {
            callback_applied.store(true, Ordering::SeqCst);
            Ok(())
        });
        assert!(!applied.load(Ordering::SeqCst));
    }
}
