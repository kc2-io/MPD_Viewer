# MPD Tabber public Client ID configuration update

Date: September 17, 2026. Based on POC source version 0.1.0.

## Configuration

The application now defaults to `ha94kk20cfu1tp74pgg8isgi88cpo7`, the Client ID supplied by the app owner. This is an application identifier, not an access token or client secret.

The native constant is `DEFAULT_CLIENT_ID` in `src-tauri/src/model.rs`. Existing empty or whitespace-only saved IDs are upgraded on load in `src-tauri/src/storage.rs`; nonempty custom IDs are preserved. The effective default is persisted on the next settings save. Other preferences are unchanged.

No account was authorized and no request was sent to Twitch with this ID. Its registration and Public client type remain unverified. No secret is stored or requested. The existing player-origin setup is unchanged.

## Changed behavior and checks

- The connection form receives the Client ID from native settings; the UI does not hardcode a second copy.
- Three additional mocked browser checks cover prefill, dispatch with the default ID, and a custom override.
- One additional Python source-configuration check verifies the supplied ID and default wiring. This is not a Rust runtime test.
- Six new Rust/SQLite regression tests cover a fresh install, missing field, empty field, whitespace-only field, custom ID preservation, and saving the migrated default. They require a native Rust build and have not been run here.

See `VERIFICATION.md` and `test-output.txt` for actual executed results. Browser checks use mocked native IPC and do not prove OAuth, Rust compilation, or real playback.

## Next local step

Build and launch using the README. Select **Twitch**, leave the prefilled ID, choose **Connect Twitch**, and complete authorization in the normal browser. Confirm **Public** client type in the Twitch developer console before testing the device flow. Configuring the ID does not remove the separate player-origin/playback acceptance gates.

## References

- [Twitch application registration and public Client IDs](https://dev.twitch.tv/docs/authentication/register-app/)
- [Twitch public-client device authorization](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/)
