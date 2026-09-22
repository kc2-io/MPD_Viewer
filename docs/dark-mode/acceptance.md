# DM-050 — Native acceptance matrix (system dark/light mode)

Status: **BLOCKED — pending platform owner.** This machine (Linux, no display, no webkit2gtk,
no dbus/GTK dev libs) cannot compile or run the native Tauri build. Per
`Feature_Dark_Mode.md` DM-050 and AGENT-RULES, a blocked platform is not a pass. Run every
claimed supported platform separately; record Windows/WebView2 (current release target) at
minimum, never infer it from Chromium/Edge.

## Code state under acceptance

Base: `main` (clean aside from this feature's files). Feature changes:
`ui/style.css`, `player-wrapper/{style.css,chat.css,chat.js}`, `player-wrapper/*` hosted source
untouched, `src-tauri/hosted-player-adapter.js`, `src-tauri/src/player.rs`
(`allowed_chat_url` now admits exactly light `?parent={host}` and dark
`?parent={host}&darkpopout`, flag-first order, raw-query guard).

Browser-level evidence (automated, already passing here): all Node suites (89), configuration
(15), release (51), `chat.browser.cjs` (hosted/bundled/demo), `theme.browser.cjs`.

## Environment / result table (owner fills in)

| Platform | WebView | Build | Cold-start light | Cold-start dark | Live OS transition | Only chat reloads | Channel preserved | Signed-in remembered-theme | Result |
|---|---|---|---|---|---|---|---|---|---|
| Windows | WebView2 | `cargo build --locked --release -p mpd-tabber --features custom-protocol` | | | | | | | |
| macOS | WKWebView | | | | | | | | |
| Linux | WebKitGTK | | | | | | | | |

## Manual steps (per available platform, per DM-050)

1. Launch with OS in light mode; inspect manager, native window chrome, bundled Demo, hosted
   viewer chrome, and official chat. When the OS is light, the chat frame must load the light URL
   shape `.../chat?parent={host}`; when the OS is dark, it must load the dark `&darkpopout` shape.
   (A signed-in profile that remembers a dark preference may still render the light URL dark;
   that residual risk is tracked below, not a failure of this step.)
2. With manager and viewer open, switch OS to dark, then back to light, no app restart.
   Expect: app chrome recolors live; the chat iframe navigates once per real transition to its
   themed URL shape; no second navigation; message: the reload may drop an unsent draft
   (documented, not retained).
3. Repeat with chat expanded and collapsed, player playing, player manually paused, autoplay
   blocked if reproducible, and (only if the owner opts in) chat signed in. No messages sent.
4. Verify only the chat frame reloads; video element, playback position, pause, audio, quality,
   collapse, session are untouched.
5. After reload verify assigned channel and signed-in/out behavior match pre-reload (identity).
6. Exercise narrow and wide viewer geometry and all manager form controls.
7. Redacted screenshots + this table. Never capture credentials, cookies, MFA, private
   messages, or raw account data.

## Known residual risk to re-check explicitly

Signed-in Twitch profile with a remembered dark preference may render the plain light URL dark
(DM-000 record). If unacceptable at release, any fix requires an approved plan
(theme-contract.md §Fallback / stop behavior): no style injection into the Twitch frame, no
broadened navigation, no combined-embed migration without a separate ADR/plan.

## Promotion gate

- Every advertised platform column above must be passed, not blocked.
- Redacted screenshots attached to this record's evidence path before release.
- Contract deviations require a reviewed plan change, not an expected-success-gate edit.