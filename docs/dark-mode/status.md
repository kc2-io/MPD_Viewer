# Status — Dark mode feature (`dark_mode_code_review.md`)

Generated: 2026-09-21

## Verdict

**`dark_mode_code_review.md` fully addressed — independent re-review closed with no blocking findings.**

The feature is complete in code and all automated evidence is green. It is **not releasable yet**:
one acceptance gate (native Windows/WebView2, P1) remains **BLOCKED** on this machine and must be
run by the platform owner. GTG is intentionally kept open.

## Branch / commits

- Branch: `feature/dark-mode`
- `45ed672` — Add OS theme support to manager, viewer chrome, and official chat
- `2a56ae7` — Record DM-040 re-review result in lease ledger
- Base: `main` `b167bdc`
- User files NOT committed (still untracked, untouched):
  - `dark_mode_code_review.md`
  - `docs/feature-plans/STREAM-TIMER-IMPLEMENTATION-PLAN.md`
- No push performed (not authorized).

## Verification results

| Check | Result |
|---|---|
| `node --test` (all Node suites) | **89/89 pass** (incl. 15 chat + new theme-navigation timeout/reset tests, hosted-player-adapter, hosted-source characterization) |
| `scripts/ci-check.sh` | **Pass** (JS + configuration suites, 51 release tests OK) |
| `tests/chat.browser.cjs` | **PASS** hosted / bundled / demo (real CSP) |
| `tests/theme.browser.cjs` | **PASS** manager / wrapper / hosted (offline palette, AA bounds + focus indicators, live OS switch without reload, external-request allowlist) |
| `git diff --check` | Clean on committed tree |
| Native `cargo build/test/clippy` | **Blocked** (no webkit2gtk / libdbus-1-dev / display); `allowed_chat_url` verified via standalone offline url-crate mirror |

## Review items and resolution

- **P2a (canonical Rust gate)** — `src-tauri/src/player.rs::allowed_chat_url` accepts exactly
  `?parent={host}` and `?parent={host}&darkpopout` (byte-for-byte raw-query compare); encoded,
  reordered, flagged, extra-key, case, and syntax-split spellings all rejected by tests.
- **P2b (light-theme contrast)** — distinct interactive and panel boundaries in both themes;
  every asserted pair AA ≥ 3:1 (non-text) / 4.5:1 (text) vs its real backdrop, including controls,
  focus indicators (measured on focused elements), running pill, error/auth/tag, wrapper lines,
  and chat toolbar.
- **P2c (chat navigation reset)** — `player-wrapper/chat.js` `navigate()` clears the previous
  timeout, shows loading feedback, sets a fresh 20 s timeout, and assigns `frame.src` exactly once;
  repeated same-theme events are no-ops.
- **Quality** — deterministic offline palette suite, route-mocked hosted shell using the preserved
  `web/parent.mpdviewer.com/index.html` (never edited; provenance intact), no unplanned external
  hosts, `color-scheme: light dark` declared on the injected hosted document, focus-aware probes.
- **Hygiene** — dedicated feature branch; commit limited to feature files; `dark_mode_code_review.md`
  and the STREAM-TIMER plan excluded; `web/parent.mpdviewer.com/index.html` and
  `scripts/ci-check.sh` show zero feature diff; no tokens/keys in the committed diff.
- **acceptance.md wording (lines 30–34)** — corrected: light OS loads the light URL shape, dark OS
  loads the `&darkpopout` shape; signed-in remembered-preference residual risk is tracked, not a
  failure of the step.

## Independent re-review

Two read-only passes by an independent reviewer:

1. First pass (pre-fix): flagged the P1/P2/quality items above → all addressed.
2. Final pass: **no blocking findings**; 6 non-blocking cosmetic notes only (trailing-whitespace
   hard breaks in `Feature_Dark_Mode.md`, a stray leading space in `player-wrapper/chat.css`,
   decorative-only `--line` divider at 1.44:1 in light, programmatic-focus outline probe caveat,
   channel-path percent-encoding note, DM-050 blocked status being an open gate rather than a defect).

## Blocked / open gate (NOT done)

- **P1 / DM-050 native acceptance** — requires Windows/WebView2 (current release target) plus a
  signed-in Twitch profile with a remembered dark preference; cold-start light/dark, live OS
  transitions, only-chat-reloads, channel/pause/volume/quality/session/collapse preservation,
  signed-in behavior. Recorded as **BLOCKED** in `docs/dark-mode/acceptance.md`; macOS/Linux must
  never be inferred from Chromium. A blocked platform is not a pass; the promotion gate stays open.

## What to do next

1. Platform owner runs `docs/dark-mode/acceptance.md` matrix on Windows/WebView2 (and any other
   advertised platform), attaches redacted screenshots.
2. If the signed-in profile renders the plain light URL dark and that is unacceptable, follow the
   approved-contract escalation path (no style injection, no broadened navigation, no combined
   embed without a separate reviewed plan).
3. Coordinator flips the GTG gate only after the native matrix passes; then the tagged-release
   rules in AGENTS.md apply.