# Windows Twitch authorization persistence

The repeated device authorization was an application bug, not an unsigned-binary or extraction-directory requirement: the Rust monitoring OAuth session lived only in memory. Twitch website cookies use the existing WebView2 profile separately and are not read, exported, or copied by this feature.

## Storage boundary

`credential_store.rs` stores a generic Windows Credential Manager entry under the fixed target `com.modpackdad.mpdtabber.poc/TwitchOAuth/v1`. `CRED_PERSIST_LOCAL_MACHINE` preserves it across logons for the **same Windows user on the same machine**, without enterprise roaming. The public client ID and validated Twitch user ID are part of the protected payload. No token reaches SQLite, manager state, JavaScript, error strings, or source logs. The serialized temporary Win32 buffers are explicitly wiped. Credential and Session types intentionally have no Debug implementation.

No new Cargo dependency or lockfile change is needed: the narrow native FFI links Windows system libraries. Non-Windows secure persistence reports unavailable, with no plaintext fallback. The controller should continue the existing memory-only login on those systems and accurately report the limitation.

This vault is not a security boundary against other software running as the same Windows user. The implementation does not claim otherwise.

## Controller integration contract

1. Add `mod credential_store;` to main. Own one `CredentialStore::new()` for the controller lifetime. All auth tasks must use clones of this same store, not independent instances.
2. Start restoring once at startup: call `begin_epoch(auth_epoch)`, then `ScopedStore::load()`. Missing entry means disconnected normally. Decode/vault errors are visible and must not trigger a login popup automatically.
3. Create `Session::from_stored(PUBLIC_CLIENT_ID, stored)` and retain that Session in an `Arc<tokio::sync::Mutex<_>>` even if the following operation has a transient error. Run `Twitch::restore_session(&mut session, &scope)` asynchronously. Only after success expose connected login and start normal monitoring. Startup validation rejects wrong application/account IDs; user ID comparison permits legitimate username changes.
4. Retry transient restore failures with bounded backoff using the **same Session**. Never reload an older vault snapshot over a rotated in-memory refresh token after a save failure. Do not delete on network, rate-limit, server, or vault errors. If `ApiError.reconnect` is true, delete using the matching scoped store, clear the session, and ask for a new connection.
5. Connect/reconnect: increment `auth_epoch` and call `begin_epoch` before spawning new OAuth work. On successful current-generation device authorization, save `session.stored()` via the scoped `SessionStore::save` before reporting that sign-in will be remembered. A storage failure must be visible; do not claim persistence. Retain the new session and retry its save (or allow a clearly labeled memory-only connection).
6. Replace `poll` with `poll_with_store`. Refresh rotations are saved before any subsequent validate/Helix request. A failed save retains the new token and its pending flag for the next retry. A failed post-refresh validation is retried before using Helix. Maintain hourly validation even when monitoring is stopped/no channels are configured (calling poll with an empty list is sufficient); this is a Twitch requirement.
7. Explicit Disconnect: increment epoch, call `CredentialStore::forget(new_epoch)` before releasing the old connection. This invalidates stale writers and deletes under the same lock. Display a delete failure and allow retry. All responses must still be checked against controller generation. For stale asynchronous revocation reports, use `ScopedStore::delete`, which cannot delete a newer connection. A stale save returns an error and cannot resurrect a disconnected credential.
8. Ordinary app shutdown and transient monitoring errors must **not** call forget/delete. Do not change the app identifier, SQLite paths, client ID, or WebView2 profile identity. Existing installations will need one final authorization because no OAuth credentials were previously saved.

The public-client device grant uses official Twitch endpoints, no client secret. Stored authorization can still expire/revoke and then needs user consent. Public-client refresh tokens have a documented 30-day validity. Persistence does not establish channel-points, streaks, or watch-time credit.

## Evidence and remaining acceptance

Executed on Windows, using a unique `TEST-ONLY/<pid>-<nonce>` Credential Manager entry containing only fake tokens; native write/read/delete and idempotent deletion passed. Production target was not read or modified. Fake-backend tests verify stale refresh cannot resurrect Disconnect or delete a newer login. Local HTTP OAuth fixtures cover startup account validation, wrong account/client rejection, refresh rotation before validation failure, secure-store failure retry, pending validation retry, revocation versus transient server errors. They are not evidence of real Twitch login.

`cargo test --locked -p mpd-twitch` and `cargo clippy --locked -p mpd-twitch --all-targets -- -D warnings` pass using an isolated target directory. Controller integration, real authorization, app restart, disconnect/restart, and offline-start/recovery need end-to-end verification after integration. macOS/Linux persistence is not implemented.

## Official references checked 2026-09-24

- [Twitch token validation](https://dev.twitch.tv/docs/authentication/validate-tokens/): startup/hourly validation and revoked-session handling.
- [Twitch refresh tokens](https://dev.twitch.tv/docs/authentication/refresh-tokens/): public-client secret exemption, rotated-token storage, public-client refresh expiry.
- [Twitch device flow](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow).
- [Microsoft CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew).
- [Microsoft CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw): generic blob size and persistence scope.
