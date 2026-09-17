# Native acceptance checklist

**Every native result below is pending.** Fill in actual observations; do not infer success from the mock-browser suite.

For each run, record date, OS/version, CPU architecture, Rust toolchain, Cargo.lock commit/hash, webview version, wrapper URL, authentication state, resource measurements, and logs with credentials removed.

| Test | Procedure / expected observation | Windows | macOS | Linux |
|---|---|---|---|---|
| Build | Compile workspace and execute all 15 core tests. | Pending | Pending | Pending |
| Cold launch | Manager opens, default capacity 3, volume 25%, no playback before Start. | Pending | Pending | Pending |
| Persistence | Add/reorder favorites, change volume/mute/cap; restart and confirm settings. Live status must not restore as fresh. | Pending | Pending | Pending |
| Demo selection | Fresh demo: Bravo/Charlie/Delta selected, Alpha offline. | Pending | Pending | Pending |
| Preemption | Alpha goes live: Delta closes before Alpha opens; retained session IDs unchanged. | Pending | Pending | Pending |
| Capacity | Test K=1,3,6; shrink 6 to 2; account for closing windows until destruction. No replacement should overshoot capacity. | Pending | Pending | Pending |
| Reordering | Reorder selected channels without reloading them. Move a live waiting channel above a selected channel. | Pending | Pending | Pending |
| Racing changes | Rapidly change priority/cap/source controls while opens/closes and API calls complete. No stale session resurrects. | Pending | Pending | Pending |
| Pause/Resume | Pause keeps current assignments and does not pause video. Resume reconciles latest ranking. Explicit cap shrink/remove still applies. | Pending | Pending | Pending |
| Skip | Manually close a player; it stays skipped for the current broadcast. Undo restores eligibility. | Pending | Pending | Pending |
| Telemetry | Demo freeze causes observed playback to become unknown. Reports never change ranking or API presence. | Pending | Pending | Pending |
| Device OAuth | Register Public app, authorize with no extra scopes, handle cancel/expiry/denial/reconnect. No secret enters the UI. | Pending | Pending | Pending |
| Credential maintenance | Validate hourly, refresh before expiry, serialize refresh operations, recover from revocation without saving tokens. | Pending | Pending | Pending |
| Real API | At least two genuine live favorites and an offline favorite; verify status and batched pagination beyond 100 logins. | Pending | Pending | Pending |
| API failure | Disconnect network or inject a failed page; existing sessions stay, affected observations stale, no false offline. | Pending | Pending | Pending |
| Rate limit | Controlled/mock HTTP integration test for 429/reset headers, not deliberate production abuse. Honor delay and bounded retries. | Pending | Pending | Pending |
| Offline debounce | One successful miss retains a selected stream; two misses confirm offline. Failed requests do not advance miss count. | Pending | Pending | Pending |
| Local wrapper | Test local HTTP and record actual acceptance/errors. This does not substitute for HTTPS acceptance. | Pending | Pending | Pending |
| HTTPS wrapper | At `https://parent.mpdviewer.com/`, serve the matching wrapper; rebuild; validate actual parent, CSP, SDK, native telemetry. Site deployment is owner-reported; verification pending. | Pending | Pending | Pending |
| Actual playback | K=1,3,6 with real channels. Separate loading/ready/playing/blocked/paused; do not equate assigned with playing. | Pending | Pending | Pending |
| Volume and mute | 0%,25%,100%, muted/unmuted; no transient SDK default loudness; retained players not reloaded. | Pending | Pending | Pending |
| Autoplay | Newly opened players may require clicks. Report block honestly; do not force user gestures or repeat unpause. | Pending | Pending | Pending |
| User pause | Manually pause in Twitch controls. Heartbeats and ranking changes must not resume retained player. | Pending | Pending | Pending |
| Inactive/minimized | Switch focus and minimize player/manager windows. Observe actual video and telemetry. Do not spoof visibility. | Pending | Pending | Pending |
| Ads/consent/login | Record behavior without treating every overlay or ad as a crash. Account-dependent playback is unsupported until verified. | Pending | Pending | Pending |
| Sleep/wake | Suspend and resume. Stale telemetry/status should be honest; no duplicate windows or polling storms. | Pending | Pending | Pending |
| Renderer/window loss | Close or fault a player in controlled testing. Current implementation treats unexpected window disappearance as skip; document this limit. | Pending | Pending | Pending |
| Stop | Stops channel polling, closes all managed player windows. Authentication maintenance may continue. | Pending | Pending | Pending |
| Quit | Close manager; confirm native process and all player webviews exit. Never close unrelated browser tabs. | Pending | Pending | Pending |
| Permission boundary | Player/iframe attempts at get_state or dispatch fail; telemetry cannot target another session. Verify actual native ACL, not just JSON. | Pending | Pending | Pending |
| Navigation | Block arbitrary top-level navigation, popups, downloads, and custom-scheme launches from player content. | Pending | Pending | Pending |
| Packaging | Native release build and clean-machine install; configure signing separately. | Pending | Pending | Pending |

## Measurements

Run a short smoke test and a longer soak at K=3; expand only after stability. Record CPU, GPU, memory, bandwidth, active processes, player session IDs, and post-stop cleanup. Do not generalize one operating system's result to another.

## Decision rule

If actual inactive/minimized playback or the compliant HTTPS embed fails on a required platform, record the failure and reconsider the playback adapter. Do not silently switch the product to visible tiles or claim the runtime has passed. The tabbed workspace and Leptos UI remain subsequent work, not proof-of-concept prerequisites.
