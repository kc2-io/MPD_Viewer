# Review of the supplied hosted player

Date: September 17, 2026. Scope: reconstructed user-pasted HTML in `web/parent.mpdviewer.com/index.html`, compared with the uploaded `MPD-Tabber-POC-Hosted-Parent.zip`. This is not a review of the user's current Git checkout or a verified live deployment. No production source was changed in this import.

## Correction to earlier assumptions

The supplied host is a **single-file, multiple-player implementation with a CSS grid**. It is not the earlier three-file, single-player wrapper. Separate `player.js` and `style.css` files must not be assumed to exist. Earlier smoke tests using fragments do not match this host. The earlier importer was too restrictive for this actual source.

## Integration comparison

| Contract | Uploaded native POC | Supplied hosted document | Consequence |
|---|---|---|---|
| Channel bootstrap | Writes `#channel=...&session=...` using `url.set_fragment` in `src-tauri/src/player.rs` | Reads `location.search` and repeated `channel` query keys | An otherwise unmodified POC launch supplies no channels to this document. The mock test reproduces zero player construction. |
| Volume | Integer percent, e.g. 25; calls `window.mpdSetAudio(percent, muted)` | Fraction 0–1; `setVolume` and a `setVolume` message | There is no `mpdSetAudio`; passing 25 as the host query value clamps to 1, not 0.25. |
| Audio policy | Global volume and separate mute for every assigned player | Active-only audio; no independent global mute command | Default behavior does not implement the agreed global-player controls. |
| Playback policy | Maintain selected streams, respecting explicit player pause | `pauseInactive=true` by default | On READY, the host attempts to play the active channel and pause all others. |
| Reporting | A registered `player_report` command, session ID and structured state, bound to the native window | Raw `{type, event, channel}` messages through multiple transports | Payload and transport do not implement the existing native command contract. |
| Layout | One native window per assigned session | Several Twitch.Player instances in a single DOM grid | Reassess grid architecture before adding native child-webview complexity. |

The Tauri command API is documented using registered commands and `invoke`; a generic `window.ipc.postMessage` call with a custom payload is not evidence of compatibility with this project's `player_report` command. [Tauri command documentation](https://v2.tauri.app/develop/calling-rust/).

For a direct WebView2 adapter, native `PostWebMessageAsJson` delivers to `window.chrome.webview`'s message event. This document registers only `window.addEventListener('message', ...)`, so that particular inbound transport is also not wired. A native adapter that explicitly executes an appropriately scoped page command would be a different integration. [Microsoft WebView2 documentation](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2.postwebmessageasjson).

## Behavior demonstrated by offline characterization

1. `pauseInactive=false` at bootstrap requests SDK autoplay for each constructed player; actual autoplay is not simulated. Switching it from true to false later does not resume a previously paused inactive player.
2. Changing the active player's volume calls `applyState`, which calls `play` when `pauseInactive` is true. This can undo a manual pause.
3. `add()` runs `relayout()` before registering the new player. The second player is laid out with one column; the fifth is laid out using a four-player count.
4. `setChannels` retains player objects, which is useful, but does not rearrange retained DOM cells to match a new priority order.
5. Removing the active player through `remove` alone leaves no active replacement. `setChannels` does perform its separate fallback selection.
6. Incoming commands do not check `e.origin` or `e.source`. A sender with a usable window reference can issue commands irrespective of origin. The test supplies an unrelated origin/source and successfully removes a player; it does not establish remote exploitability without that window reference.
7. Invalid volume text becomes `NaN`; a non-array `channels` value throws; channel values and list lengths lack a bounded schema.
8. A fragment-only POC URL creates no players, and the legacy audio function is absent.

For cross-document messaging, validate both the sender and the message schema and use an explicit target origin where appropriate. Sending no credentials does not make unauthorized playback commands safe. Native IPC needs its own capability checks and session binding; an origin string inside a JSON payload is not authentication. [MDN postMessage security guidance](https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage).

## Static review findings not established by browser tests

The CSS enforces 400 by 300 minimum cells while the page hides overflow. A small window or too many cells can clip content. There is no demonstrated viewport-capacity policy. Twitch documents minimum video dimensions and visibility-related autoplay requirements, so test the actual visible video area, including space needed for chat, rather than treating minimum CSS values as proof. [Twitch player documentation](https://dev.twitch.tv/docs/embed/video-and-clips/).

The page has no chat iframe, account-state management, protocol version, session generation, per-command acknowledgement IDs, or explicit SDK-load failure UI. `ONLINE` is forwarded as its own event and must not be promoted to evidence of playback. `hostReady` reports initialization, not authenticated or playing video.

The source includes a third-party Cloudflare script and no CSP meta element. HTTP security headers were not supplied; server CSP must not be inferred absent. Keep the imported source intact, then separately review third-party script execution on an origin granted native telemetry capabilities. Do not load analytics into a privileged viewer simply because it exists in this snapshot.

This source does not establish why the owner sees ads. API authorization, website session recognition and Turbo playback remain separate tests. Do not use hidden playback, ad blocking, cookie extraction, or browser-security bypasses as fixes.

## Verification for this import

- 18 Node characterization tests passed using local mocks; external scripts were not executed.
- The inline application JavaScript compiled in Node's VM.
- The manifest hash matches the normalized HTML.
- Source changes are additive: native code, capabilities, deployed assets and the original `player-wrapper/` remain unchanged.
- No Rust build, native webview run, real Twitch session, chat, Turbo or live deployment check was performed.

See `HOSTED-SOURCE-PLAN-ADDENDUM.md` for the bounded next steps.
