# Full-page viewer controls and native window mute plan

Date: 2026-09-25
Base: `origin/main` at `cc6e7e6`
Branch/worktree: `am/MPDViewer-bitrate-volume` / `MPDViewer-bitrate-volume`

## Objective

Make the manager accurately reflect the two viewer modes:

- The default full-page mode opens a first-party Twitch channel page in a native
  webview. It must not show the preferred-quality (bitrate) or volume controls,
  because MPD Viewer cannot apply those settings to the page.
- The explicit `--embedded-viewer` mode and bundled demo keep their existing
  quality, volume, and embed mute controls.
- Full-page viewers gain native whole-webview mute controls where the platform
  engine publicly supports them: a persisted global mute and a runtime mute for
  each current player session. These controls mute audio output without editing,
  scripting, or claiming telemetry from Twitch's document.

The existing app identifier, preferences path/schema, browser profile, Twitch
client ID, player selection policy, and hosted-source snapshot remain unchanged.

## Verified baseline and platform boundary

- `ViewerMode::TwitchPage` is already the default; only the exact
  `--embedded-viewer` flag selects the embed.
- `ViewerCapabilities.media_controls` already hides the combined quality,
  volume, and embed-mute block for live full-page viewers. The manager currently
  offers no usable mute control in that mode.
- Windows WebView2 exposes `ICoreWebView2_8::SetIsMuted`, and Linux WebKitGTK
  exposes `WebViewExt::set_is_muted`. Both silence the webview while leaving
  playback running. Tauri's `WebviewWindow::with_webview` provides the bounded
  platform-handle access; direct bindings are target-gated so other builds do
  not acquire irrelevant platform dependencies.
- Public macOS `WKWebView` has no equivalent page-output mute property. Do not
  use private selectors, Twitch DOM automation, injected media-element scripts,
  or OS-wide process muting. Advertise native mute only when compiled for a
  supported engine, and explain the limitation in the manager otherwise.

## Behavior contract

| Surface | Quality/bitrate | Volume | Global mute | Per-stream mute |
|---|---|---|---|---|
| Bundled demo | Existing control | Existing control | Existing embed mute | No separate window control |
| `--embedded-viewer` | Existing control | Existing control | Existing embed mute | No separate window control |
| Full-page Windows/Linux | Hidden | Hidden | Native webview mute | Native webview mute for the exact session |
| Full-page macOS | Hidden | Hidden | Hidden with an honest unsupported note | Hidden |

Additional rules:

1. Reuse `Settings.muted` as the persisted global mute preference so upgrades
   need no schema migration and the preference has one meaning across launch
   modes. Do not change or discard the stored volume/quality values merely
   because those controls are hidden. The existing embedded/demo checkbox edits
   this same global preference; changing launch mode never clears it.
2. Per-stream mute is runtime state attached to `PlayerSession`, not a favorite
   or Twitch account preference. A newly opened/replaced session starts without
   an individual override and derives its effective state from global mute.
3. Effective full-page mute is `settings.muted || session.window_muted`. Turning
   global mute off restores each session's individual choice. While global mute
   is on, per-session controls show the global state and are disabled rather than
   implying that one stream can override it.
4. A per-stream action carries the numeric session ID. Rust rejects an unknown,
   closing, demo, embedded, or stale session; channel text alone is not authority.
5. `SetAudio` remains the embedded/demo volume-and-mute operation and is rejected
   for a live full-page backend. A separate explicit window-mute action handles
   full-page global and per-session requests. Full-page per-session actions never
   write the persisted global preference.
6. Native mute never changes playback intent, selection, timers, focus, browser
   profile, Twitch settings, or page DOM. Errors stay visible and do not report a
   successful state transition.
7. Controller state distinguishes persisted/requested values from the last
   native-applied value for every page window. `with_webview` completion is
   awaited with a timeout; a queued callback is not treated as success. Timeout
   cancellation and the native setter are serialized under one state lock, so a
   callback that has not started cannot mutate audio after timeout/rollback and
   one already applying must publish its result. `PlayerView` reports only
   last-confirmed native-applied state as effective state.
8. A global change is transactional at controller level. Save the preference
   only after every current page window confirms the target effective mute. If a
   window fails, restore already changed windows to their recorded prior values,
   leave `Settings.muted` unchanged, retain the confirmed result of each rollback,
   and show a bounded error naming failed/rollback-failed sessions. Retrying the
   same UI operation attempts reconciliation again. A storage failure after all
   native changes uses the same rollback policy.
9. A per-session change updates its runtime override and reported applied state
   only after the exact window confirms success. Failure leaves the override
   unchanged. Rust rejects per-session actions while global mute is enabled,
   matching the disabled UI.
10. On supported platforms, a full-page window is constructed hidden on
    `about:blank`, its initial effective global mute is confirmed through the
    native API, and only then does it navigate to Twitch and become visible.
    Failure destroys the hidden blank window and returns an open error, so a
    persisted muted launch cannot start page audio or appear successfully opened.
    The unmuted path is confirmed the same way.

## Implementation sequence and ownership

Shared controller/model/UI files have one production owner: the coordinator in
this worktree. Review agents are read-only, satisfying the exclusive-file rules.

1. **Capability and model contract** — coordinator, high reasoning.
   - Extend `ViewerCapabilities` with an explicit native-window-mute capability.
   - Add a session-bound full-page mute action and expose requested/effective
     window mute state on `PlayerView` without reusing wrapper telemetry fields.
   - Keep serialized defaults backward compatible for browser fixtures and saved
     schema-1 settings.
2. **Native adapter** — coordinator, high reasoning.
   - Isolate platform code in a small native-window-audio module.
   - Windows: obtain the WebView2 core object and cast to `ICoreWebView2_8` before
     calling its mute setter.
   - Linux WebKitGTK: call the public `set_is_muted` API.
   - macOS and other unsupported targets: return an explicit unsupported result
     and expose the capability as false; add no private API dependency.
   - Create a supported full-page window hidden on `about:blank`, wait for native
     mute completion, then navigate and show only after success. Destroy and
     reject the session on failure.
3. **Controller semantics** — coordinator, high reasoning.
   - Track individual mute on each live `PlayerSession`.
   - Apply global changes to every non-closing full-page window; apply individual
     changes only to the validated session. Track each confirmed native value,
     use the transactional rollback/persistence policy above, and surface partial
     failures without rendering requested state as effective.
   - Preserve the embed's current audio and quality delivery paths exactly.
4. **Manager UI** — coordinator, medium reasoning.
   - Keep the quality and volume inputs absent from layout/accessibility flow in
     full-page mode.
   - Replace the old note with concise mode-accurate guidance.
   - Show a global native-window mute checkbox only when supported, and add a
     `Mute page` / `Unmute page` button to each full-page player card.
   - Preserve focus during periodic refresh and avoid rebuilding cards solely due
     to unrelated state.
5. **Verification** — coordinator, high reasoning.
   - Rust tests: capability matrix, action deserialization/validation, stale
     session rejection, global/individual precedence, new-session inheritance,
     callback timeout/failure, one-of-many global failure and rollback, storage
     failure rollback, restart persistence, failed initial-mute destruction with
     no published session/capacity leak, and unchanged embedded audio/quality
     behavior. Use an injectable native-mute seam for deterministic failures.
   - Mocked browser tests: full-page controls hide quality/volume, supported and
     unsupported mute UI, global disable/restore semantics, session IDs in
     actions, and embedded/demo regression coverage.
   - Native E2E: default full-page fixture exercises confirmed muted and unmuted
     launch plus global and per-session mute without replacing windows; embedded
     launch still exposes its media controls. Treat native API confirmation as
     runtime mute evidence while not claiming audible-output measurement.
   - Run workspace tests, source checks, clippy, native build, browser checks,
     E2E static checks, and the available native GUI matrix. Record mock, compile,
     and runtime evidence separately.
6. **Independent review and delivery**.
   - A lower-cost plan reviewer (`gpt-6-luna`, medium) checks scope, platform
     claims, state semantics, testability, and missing acceptance criteria.
   - After implementation, an independent code reviewer (`gpt-6-sol`, high)
     reviews controller authority, native API safety, stale-session handling,
     cross-platform compilation, UI truthfulness, and tests. Findings return to
     the coordinator for correction and re-review.
   - Commit and push the bounded patch, then open a PR against `main`. Do not
     merge it, tag a release, deploy the hosted site, or change signing settings.

## Acceptance criteria

- A default live full-page manager renders no preferred-quality/bitrate selector
  and no volume slider.
- On supported platforms it renders a global window-mute control and one control
  for each current player; each calls a native webview mute API, not Twitch DOM.
- Global mute affects current and newly opened full-page windows and persists;
  per-session choices survive global mute toggles for that session only.
- Global-on disables individual buttons without discarding their overrides;
  global-off restores each override. Global preference survives an app restart
  and launch-mode change, while per-session overrides do not.
- Newly opened supported page windows remain hidden until their initial muted or
   unmuted state is confirmed before Twitch navigation. Partial callback/storage failures stay visible,
  retain the old preference, and show only confirmed per-window applied state.
- A stale session action cannot affect a replacement window, and unsupported
  platforms never display a control that cannot work.
- Demo and `--embedded-viewer` retain their existing volume, quality, mute,
  playback, and telemetry behavior.
- Relevant local checks pass, independent review has no unresolved blockers,
  the branch is clean, and the opened PR's required CI checks finish green.
