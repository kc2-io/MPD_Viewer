//! Native OAuth vault. No credentials are stored in SQLite, logs, or webview state.
use mpd_twitch::{SessionStore, StoredSession};
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "e2e-tests"))]
const TARGET: &str = "com.modpackdad.mpdtabber.poc/TwitchOAuth/v1";
trait Backend: Send + Sync {
    fn load(&self) -> Result<Option<StoredSession>, String>;
    fn save(&self, value: &StoredSession) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}
struct State { epoch: u64, backend: Box<dyn Backend> }
#[derive(Clone)]
pub struct CredentialStore { state: Arc<Mutex<State>> }
#[derive(Clone)]
pub struct ScopedStore { state: Arc<Mutex<State>>, epoch: u64 }
impl CredentialStore {
    pub fn new() -> Self {
        #[cfg(feature = "e2e-tests")]
        let target = crate::e2e::vault_target();
        #[cfg(feature = "e2e-tests")]
        let target = target.as_str();
        #[cfg(not(feature = "e2e-tests"))]
        let target = TARGET;
        Self { state: Arc::new(Mutex::new(State { epoch: 0, backend: Box::new(PlatformVault::new(target)) })) }
    }
    /// Advance before starting any new Connect/restore task. Older tasks can no longer write.
    pub fn begin_epoch(&self, epoch: u64) -> Result<ScopedStore, String> {
        let mut state = self.state.lock().map_err(|_| "Twitch credential store lock failed.")?;
        if epoch < state.epoch { return Err("Twitch connection was superseded.".into()); }
        state.epoch = epoch;
        Ok(ScopedStore { state: self.state.clone(), epoch })
    }
    /// Advance AND delete atomically for explicit Disconnect. Report deletion failures.
    pub fn forget(&self, epoch: u64) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Twitch credential store lock failed.")?;
        if epoch < state.epoch { return Err("Twitch connection was superseded.".into()); }
        state.epoch = epoch;
        state.backend.delete()
    }
}
impl ScopedStore {
    fn with_backend<T>(&self, f: impl FnOnce(&dyn Backend) -> Result<T, String>) -> Result<T, String> {
        let state = self.state.lock().map_err(|_| "Twitch credential store lock failed.")?;
        if state.epoch != self.epoch { return Err("Twitch connection was superseded.".into()); }
        f(state.backend.as_ref())
    }
    pub fn load(&self) -> Result<Option<StoredSession>, String> { self.with_backend(|vault| vault.load()) }
    /// Revocation deletion is scoped: a stale poll must not delete a newer login.
    pub fn delete(&self) -> Result<(), String> { self.with_backend(|vault| vault.delete()) }
}
impl SessionStore for ScopedStore {
    fn save(&self, value: &StoredSession) -> Result<(), String> { self.with_backend(|vault| vault.save(value)) }
}

#[cfg(not(windows))]
struct PlatformVault;
#[cfg(not(windows))]
impl PlatformVault { fn new(_: &str) -> Self { Self } }
#[cfg(not(windows))]
impl Backend for PlatformVault {
    fn load(&self) -> Result<Option<StoredSession>, String> { Err("Remembering Twitch authorization is currently supported only on Windows.".into()) }
    fn save(&self, _: &StoredSession) -> Result<(), String> { Err("Secure Twitch credential storage is unavailable on this platform.".into()) }
    fn delete(&self) -> Result<(), String> { Ok(()) }
}

#[cfg(windows)]
use windows_vault::PlatformVault;
#[cfg(windows)]
mod windows_vault {
    use super::*;
    use std::{ffi::c_void, ptr};
    const GENERIC: u32 = 1;
    const LOCAL_MACHINE: u32 = 2; // Persist across logons of this same Windows user, never enterprise roaming.
    const NOT_FOUND: u32 = 1168;
    const MAX_BLOB: usize = 2560;
    #[repr(C)]
    struct FileTime { low: u32, high: u32 }
    #[repr(C)]
    struct Credential {
        flags: u32, kind: u32, target: *mut u16, comment: *mut u16, modified: FileTime,
        blob_size: u32, blob: *mut u8, persist: u32, attribute_count: u32,
        attributes: *mut c_void, alias: *mut u16, username: *mut u16,
    }
    #[link(name = "Advapi32")]
    extern "system" {
        fn CredReadW(target: *const u16, kind: u32, flags: u32, credential: *mut *mut Credential) -> i32;
        fn CredWriteW(credential: *const Credential, flags: u32) -> i32;
        fn CredDeleteW(target: *const u16, kind: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut c_void);
    }
    #[link(name = "Kernel32")]
    extern "system" { fn GetLastError() -> u32; }
    fn wipe(bytes: &mut [u8]) {
        for byte in bytes { unsafe { ptr::write_volatile(byte, 0); } }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
    pub(super) struct PlatformVault { target: Vec<u16> }
    impl PlatformVault {
        pub(super) fn new(target: &str) -> Self { Self { target: target.encode_utf16().chain(Some(0)).collect() } }
    }
    impl Backend for PlatformVault {
        fn load(&self) -> Result<Option<StoredSession>, String> {
            let mut raw = ptr::null_mut();
            // SAFETY: target is NUL terminated; API initializes raw on success.
            if unsafe { CredReadW(self.target.as_ptr(), GENERIC, 0, &mut raw) } == 0 {
                return if unsafe { GetLastError() } == NOT_FOUND { Ok(None) }
                    else { Err("Windows could not read saved Twitch authorization.".into()) };
            }
            if raw.is_null() { return Err("Windows returned an invalid Twitch credential.".into()); }
            // SAFETY: CredReadW owns a valid CREDENTIALW allocation until CredFree.
            let result = unsafe {
                let credential = &mut *raw;
                if credential.blob.is_null() || credential.blob_size == 0 || credential.blob_size as usize > MAX_BLOB {
                    Err("Saved Twitch authorization is invalid. Reconnect Twitch.".into())
                } else {
                    let bytes = std::slice::from_raw_parts_mut(credential.blob, credential.blob_size as usize);
                    let value = StoredSession::decode(bytes).map(Some);
                    wipe(bytes);
                    value
                }
            };
            unsafe { CredFree(raw.cast()); }
            result
        }
        fn save(&self, value: &StoredSession) -> Result<(), String> {
            let mut blob = value.encode()?;
            if blob.len() > MAX_BLOB {
                wipe(&mut blob);
                return Err("Twitch authorization exceeds the Windows vault size limit.".into());
            }
            let mut username: Vec<u16> = "MPD Viewer Twitch OAuth".encode_utf16().chain(Some(0)).collect();
            let credential = Credential { flags: 0, kind: GENERIC, target: self.target.as_ptr().cast_mut(),
                comment: ptr::null_mut(), modified: FileTime { low: 0, high: 0 }, blob_size: blob.len() as u32,
                blob: blob.as_mut_ptr(), persist: LOCAL_MACHINE, attribute_count: 0, attributes: ptr::null_mut(),
                alias: ptr::null_mut(), username: username.as_mut_ptr() };
            // SAFETY: all pointers remain live for the synchronous call; no attributes are provided.
            let success = unsafe { CredWriteW(&credential, 0) } != 0;
            wipe(&mut blob);
            if success { Ok(()) } else { Err("Windows could not securely save Twitch authorization.".into()) }
        }
        fn delete(&self) -> Result<(), String> {
            // SAFETY: target is NUL terminated. Delete only our exact generic-credential target.
            if unsafe { CredDeleteW(self.target.as_ptr(), GENERIC, 0) } != 0 || unsafe { GetLastError() } == NOT_FOUND {
                Ok(())
            } else { Err("Windows could not remove saved Twitch authorization. Retry Disconnect.".into()) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sentinel() -> StoredSession {
        StoredSession::decode(br#"{"version":1,"client_id":"test-app","access":"fake-access","refresh":"fake-refresh","user_id":"123"}"#).unwrap()
    }
    struct Fake(Arc<Mutex<Option<Vec<u8>>>>);
    impl Backend for Fake {
        fn load(&self) -> Result<Option<StoredSession>, String> { self.0.lock().unwrap().as_deref().map(StoredSession::decode).transpose() }
        fn save(&self, value: &StoredSession) -> Result<(), String> { *self.0.lock().unwrap() = Some(value.encode()?); Ok(()) }
        fn delete(&self) -> Result<(), String> { *self.0.lock().unwrap() = None; Ok(()) }
    }
    #[test]
    fn stale_refresh_cannot_resurrect_disconnect_or_delete_new_login() {
        let store = CredentialStore { state: Arc::new(Mutex::new(State { epoch: 0, backend: Box::new(Fake(Arc::new(Mutex::new(None)))) })) };
        let old = store.begin_epoch(1).unwrap();
        old.save(&sentinel()).unwrap();
        store.forget(2).unwrap();
        assert!(old.save(&sentinel()).is_err());
        assert!(store.begin_epoch(2).unwrap().load().unwrap().is_none());
        let new = store.begin_epoch(3).unwrap();
        new.save(&sentinel()).unwrap();
        assert!(old.delete().is_err());
        assert!(new.load().unwrap().is_some());
    }
    #[test]
    #[cfg(windows)]
    fn windows_vault_roundtrip_and_delete_isolated_sentinel() {
        let target = format!("com.modpackdad.mpdtabber.poc/TEST-ONLY/{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let vault = PlatformVault::new(&target);
        struct Cleanup<'a>(&'a PlatformVault);
        impl Drop for Cleanup<'_> { fn drop(&mut self) { let _ = self.0.delete(); } }
        let _cleanup = Cleanup(&vault);
        assert!(vault.load().unwrap().is_none());
        vault.save(&sentinel()).unwrap();
        let loaded = vault.load().unwrap().unwrap();
        assert!(loaded.encode().unwrap() == sentinel().encode().unwrap());
        vault.delete().unwrap();
        assert!(vault.load().unwrap().is_none());
        vault.delete().unwrap();
    }
}
