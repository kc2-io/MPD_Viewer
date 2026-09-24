# Twitch channel-page viewer proof of concept

Date: September 23, 2026

Status: **Source and mocked-UI proof passed; Windows/WebView2 rewards evidence pending.**

## Decision under test

The normal build opens `https://www.twitch.tv/{channel}` as the top-level document in an
MPD-managed WebView. There is no runtime viewer-mode setting and no UI control for changing
viewer backends. The former Twitch embed remains available only as an explicitly selected
build feature.

The internal feature contract is:

```text
default build                         twitch-page-viewer
alternate developer build            twitch-embed-viewer
both or neither                       compile error
```

Demo mode still uses the bundled local wrapper; it is a simulated data source, not a second
production Twitch mode.

## What the source proof establishes

- A real assignment resolves to the exact initial URL `https://www.twitch.tv/{normalized-login}`.
- Real channel pages use `twitch-page-{session}` labels. The existing Tauri capability matches
  only `player-*`, and its remote URL list does not contain `www.twitch.tv`, so Twitch pages get
  no native command permission.
- The full-page backend installs no initialization script. Wrapper adapters remain compiled only
  for `twitch-embed-viewer`.
- Navigation permits credential-free HTTPS on `www.twitch.tv` and the official
  `player.twitch.tv` subframe origin. It rejects arbitrary hosts, lookalike hosts, credentials,
  explicit ports, `id.twitch.tv`, custom schemes, and local files.
- Popups and downloads remain denied. Login is expected to use the existing separate Connect
  window and the default persistent MPD Viewer browser profile.
- Rust remains authoritative for selection, capacity, focus, retry, skip, and window lifetime.
- The manager does not claim playback telemetry and hides embed-only quality/audio controls for
  the full channel-page backend. Twitch owns page playback, rewards, and sign-in UI.
- No hosted source, Twitch client ID, application identifier, preference path, browser-profile
  override, capability file, workflow, release setting, or deployment was changed.

## Checks run in the Linux development environment

Passed:

```text
git diff --check
node --check ui/app.js
node --test tests/view-model.test.mjs
python3 tests/configuration_test.py                       17 tests
bash scripts/ci-check.sh                                 all JS/config/release checks
MPD_TEST_BROWSER=chromium ... tests/viewer-auth.browser.cjs
                                                         mocked UI PASS
cargo metadata --no-deps --format-version 1
rustfmt --edition 2021 --check ...                       parsed edited Rust; formatting drift remains
```

Blocked rather than passed:

- Linux `cargo check -p mpd-tabber`: host lacks `libdbus-1-dev` before the application crate
  compiles.
- Cross-target Windows `cargo check`: the Rust Windows standard library and locked Windows crates
  were obtained, but this Linux host lacks the MSVC `lib.exe`/compiler toolchain required by
  `ring`.
- Native WebView2 launch, profile persistence, Twitch sign-in, playback, Channel Points, Watch
  Streaks, concurrent credit, and minimized/background behavior cannot be inferred from Chromium
  or source tests.

## Windows build checks

Run from this branch using a normal Visual Studio/MSVC Rust environment:

```powershell
cargo test --locked --workspace --features mpd-tabber/custom-protocol
cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol
cargo build --locked -p mpd-tabber --features custom-protocol

cargo check --locked --workspace --no-default-features `
  --features mpd-tabber/custom-protocol,mpd-tabber/twitch-embed-viewer
```

The first three commands test the default channel-page backend. The final command proves that the
non-default embed backend still type-checks. Do not use `--all-features`; selecting both backends
is intentionally rejected.

## Signed-in WebView2 result matrix

Use an enrolled channel where the tester can see the Channel Points balance. Do not capture
credentials, cookies, MFA prompts, private chat, OAuth URLs, or raw account data.

| Check | Result | Redacted evidence/notes |
|---|---|---|
| Diagnostics says `twitch-page`; initial document is `https://www.twitch.tv/{channel}` | PENDING | |
| Connect completes and the channel page recognizes the same signed-in account | PENDING | |
| Website session survives closing and restarting MPD Viewer | PENDING | |
| Ordinary timed Channel Points increase while the page is visible | PENDING | |
| Active-watching bonus can be claimed manually in Twitch UI | PENDING | |
| A qualifying consecutive broadcast advances the Watch Streak | PENDING | |
| One unfocused but visible managed page continues to receive credit | PENDING | |
| A minimized/background page continues to receive credit | PENDING | |
| Two simultaneous managed channel pages each receive ordinary credit | PENDING | |
| In-page navigation cannot silently leave a window assigned to the wrong channel | PENDING | |
| Stop/Skip/Retry close or replace only the intended page | PENDING | |
| No embed, duplicate playback, native IPC access, download, or arbitrary popup occurs | PENDING | |

For the ordinary-points check, record the displayed balance and wall-clock time before and after
at least two normal Twitch earning intervals. Do not count a manually claimed bonus as ordinary
watch credit. For concurrent pages, record each channel separately; do not infer success from one.

Watch Streak validation may require waiting for the next qualifying broadcast. Until Twitch shows
the updated streak in its own UI, record it as pending, not passed.

## Stop conditions

- If the channel page or its player is blocked by the navigation policy, record the exact official
  hostname needed; do not broaden to arbitrary subdomains without review.
- If Connect authorization succeeds but the channel page is signed out, the assumed shared
  profile is disproved. Investigate Tauri/WebView2 profile configuration without importing cookies.
- If visible full channel pages do not earn ordinary points, this architecture does not meet the
  feature goal. Do not add DOM automation, private Twitch APIs, user-agent spoofing, cookie copying,
  automated reward claiming, or a silent embed fallback.
- If only one of several simultaneous pages earns credit, document that Twitch limitation before
  changing MPD selection or scheduling behavior.
- If Twitch's client-side navigation lets a managed window move to another channel while Rust still
  assigns the original channel, treat that as a failed selection-authority check. Do not inject a
  DOM watcher as a shortcut; define an enforceable native navigation boundary first.

## Current conclusion

The modular, capability-free full-page path is viable at source and manager-UI level. It is not yet
proven to preserve Channel Points or Watch Streaks. The result remains pending until the Windows
matrix above contains direct Twitch/WebView2 observations.
