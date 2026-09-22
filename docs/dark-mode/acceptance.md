# DM-050 — Native acceptance matrix (system dark/light mode)

Status: **IN PROGRESS — Windows native acceptance, 2026-09-22.** Windows compilation and
42 workspace tests passed. The native manager cold-started in light mode with the light
palette and light title bar. Signed-in chat and the remaining native matrix are pending;
this is not GTG. macOS/WKWebView and Linux/WebKitGTK runtime remain untested. A blocked
platform is not a pass; never infer native behavior from Chromium/Edge.

## Code state under acceptance

Base: `origin/main` at `b167bdc`; branch `feature/dark-mode`, incoming HEAD `46add96`.
Windows verification uses an isolated linked worktree. Feature changes:
`ui/style.css`, `player-wrapper/{style.css,chat.css,chat.js}`, `player-wrapper/*` hosted source
untouched, `src-tauri/hosted-player-adapter.js`, `src-tauri/src/player.rs`
(`allowed_chat_url` admits exactly light `?parent={host}` and dark
`?parent={host}&darkpopout`; reversed order is rejected).

Browser-level evidence (automated, already passing here): all Node suites (89), configuration
(15), release (51), `chat.browser.cjs` (hosted/bundled/demo), `theme.browser.cjs`.

## Environment / result table (owner fills in)

| Platform | WebView | Build | Cold-start light | Cold-start dark | Live OS transition | Only chat reloads | Channel preserved | Signed-in remembered-theme | Result |
|---|---|---|---|---|---|---|---|---|---|
| Windows 10.0.26200 | WebView2 153.0.4234.48 | Passed, unsigned | Observed: manager, hosted viewer and chat light | Pending | User confirmed light → dark → light | No visible playback interruption; exact navigation count pending | Observed retained channel | Pending: user reports preference unknown | P1 OPEN |
| macOS | WKWebView | Not run here | Blocked | Blocked | Blocked | Blocked | Blocked | Blocked | BLOCKED |
| Linux | WebKitGTK | Not run here | Blocked | Blocked | Blocked | Blocked | Blocked | Blocked | BLOCKED |

### Windows session observations, 2026-09-22

- Built after the follow-up contrast corrections; executable SHA-256:
  `9b2f4cfce14db386774b8bce95d945f238604a92a5c90947b993b3cb18f6745b`.
- Existing release was closed before launch. Existing profile and preferences were used;
  no credential/cookie extraction or profile copying occurred.
- User completed normal Twitch authorization and started two live viewers. Native
  light chat and a signed-in message composer were observed; no messages were sent.
- User paused one video, hid chat and switched Windows to Dark. User reported expanded
  chat was dark. User then confirmed chat returned to light, the paused stream stayed
  paused and the other stream continued. Native observation also showed light chat and
  the paused player. These are UI/user observations, not SDK-object or navigation traces.
- User explicitly reported Twitch's remembered appearance was unknown. That does not
  satisfy the mandatory remembered-dark P1 test.
- Still unverified natively: dark cold start, bundled Demo, exact one-chat-navigation
  count, underlying video/player object and session identity, volume/mute/quality
  preservation, autoplay-blocked behavior, and narrow/wide geometry acceptance.
- Screenshots viewed during testing contain account/chat information and are not
  committed. Public evidence is this redacted written record; sanitized screenshot
  artifacts are still required before promotion.

## Manual steps (per available platform, per DM-050)

1. Launch with OS in light mode; inspect manager, native window chrome, bundled Demo, hosted
   viewer chrome, and official chat. When the OS is light, the chat frame must load the light URL
   shape `.../chat?parent={host}` AND visibly render light chat; when the OS is dark,
   it must load the dark `&darkpopout` shape AND visibly render dark chat. Repeat cold start
   in dark mode. Signed-in chat with a remembered dark preference is mandatory for P1:
   dark chat in light-system mode is an acceptance failure, even if its URL is correct.
2. With manager and viewer open, switch OS to dark, then back to light, no app restart.
   Expect: app chrome recolors live; the chat iframe navigates once per real transition to its
   themed URL shape; no second navigation; message: the reload may drop an unsent draft
   (documented, not retained).
3. Repeat with chat expanded and collapsed, player playing, player manually paused, autoplay
   blocked if reproducible, and chat signed in with a remembered dark preference. The user
   completes any authentication in Twitch's normal UI. If unavailable, mark P1 blocked,
   never waived or passed. No messages sent.
4. Verify only the chat frame reloads; video element, playback position, pause, audio, quality,
   collapse, session are untouched.
5. After reload verify assigned channel and signed-in/out behavior match pre-reload (identity).
6. Exercise narrow and wide viewer geometry and all manager form controls.
7. Redacted screenshots + this table. Never capture credentials, cookies, MFA, private
   messages, or raw account data.

## Mandatory architecture stop condition

If a signed-in Twitch profile with a remembered dark preference renders the plain light URL
dark, the separate-chat implementation fails P1. Escalate the architecture decision;
do not waive the requirement, inject styles into Twitch's cross-origin DOM, broaden
navigation, or migrate to combined Twitch.Embed without a separately reviewed plan.

## Promotion gate

- Every advertised platform column above must be passed, not blocked.
- Redacted screenshots attached to this record's evidence path before release.
- Contract deviations require a reviewed plan change, not an expected-success-gate edit.
