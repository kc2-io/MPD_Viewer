# DM-000 — Baseline and standalone Twitch chat theming feasibility

**Status:** Candidate exact mapping is evidence-backed for fresh profiles; signed-in/remembered-theme and native live-change confirmation remains a recorded blocked item for the native platform owner.
**Date:** 2026-09-21
**Checkout:** `main` at `b167bdcd58a81e9125f9fb0a7574cf53eefbf2ed`; worktree dirty state: two untracked planning docs only (`Feature_Dark_Mode.md`, `docs/feature-plans/STREAM-TIMER-IMPLEMENTATION-PLAN.md`). No tracked file modifications at start.
**Environment:** Linux x86_64, no DISPLAY/WAYLAND_DISPLAY, `xvfb-run` present, **no webkit2gtk dev runtime** (native Linux app run blocked), Node v24.19.0, Rust 1.98.1, Python 3.11.2. Twitch host reachable over HTTPS (HTTP 200).

This file is the DM-000 output. It records *web-observable* evidence (real, unmocked Twitch responses) and is **not** native OS-theme evidence. Native confirmation is recorded in `docs/dark-mode/acceptance.md`.

## Decision gate result

The plan required testing fresh-profile light/dark, signed-in profiles, and live OS changes against the configured HTTPS parent.

- **Supported (deterministic, fresh profile):**
  - light → `https://www.twitch.tv/embed/{channel}/chat?parent={parent}` (exactly today's URL)
  - dark → `https://www.twitch.tv/embed/{channel}/chat?parent={parent}&darkpopout`
- **Blocked (needs a native platform owner + normal Twitch sign-in):** signed-in-profile behavior (account-remembered theme), identity/session after chat-only reload, and live OS light↔dark↔light switching over a real WebView2 session. Credentials were not created, requested, or recorded.

## Direct evidence gathered

### Official documentation (fetched 2026-09-21, dev.twitch.tv/docs/embed/chat/)

The standalone chat iframe template is `https://www.twitch.tv/embed/<channel>/chat?parent=<parent>`. Parameter table lists only `channel`, `parent`, `height`, `width`, `sandbox`. **No theme parameter exists for the separate chat embed.** The combined `Twitch.Embed` API documents `theme: "light" | "dark"`, but adopting it would replace the separate `Twitch.Player` + chat architecture and is out of scope (requires a separate approved plan).

### Ecosystem/history

- `darkpopout` has been the long-standing mechanism for forcing the separate chat embed dark (Twitch developer forum answers 2020, 2021, 2022; GitHub ecosystem projects twitchls, multitwitch).
- Twitch staff statement (2018 new-chat-embed EOL announcement): embed intent is for developers to use the light version or force dark "via the ?darkpopout query param ... to match the style of the surrounding page".
- A 2023 Twitch forum response caveat: `darkpopout=1` forces dark, "otherwise user preference should apply/be recalled" — i.e. a signed-in profile with a remembered dark preference may render the plain URL dark. This is the residual risk that keeps the native spike required.
- `twitchdev/issues#1054` (2025) requests documenting `darkpopout`; it remains undocumented in the official table.

### Direct HTTP fetch (real service, unmocked)

`https://www.twitch.tv/embed/alpha/chat?parent=parent.mpdviewer.com`, with and without `&darkpopout` / `&darkpopout=1`, returned byte-identical shell HTML (181085 bytes) — theme application is client-side in the JS bundles. Server-side bytes are not a theme determinant.

### Headless Chromium observation of the real embed (fresh profile, no cookies)

Playwright chromium, fresh browser context, real Twitch resources, configured parent. `colorScheme` emulation simulates our own page's media; the cross-origin chat iframe's theme is Twitch-controlled.

| URL shape | Media | Observed Twitch theme |
|---|---|---|
| `parent` only | light | `tw-root--theme-light` (bg `rgb(247,247,248)` / white) |
| `parent` only | dark | `tw-root--theme-light` (unchanged) |
| `parent&darkpopout` | light | `tw-root--theme-dark` (bg `rgb(14,14,16)` / `rgb(24,24,27)`) |
| `parent&darkpopout` | dark | `tw-root--theme-dark` (unchanged) |
| `parent&darkpopout=1` | light / dark | `tw-root--theme-dark` |
| `parent&theme=light` | both | `tw-root--theme-light` — `theme=` is **ignored** |
| `parent&theme=dark` | both | `tw-root--theme-light` — `theme=` is **ignored** |

Conclusions for DM-001:

1. The exact light mapping is the current `parent`-only URL; the exact dark mapping adds the single flag `darkpopout` (flag form and `=1` both force dark).
2. `theme=` has no effect on the separate chat embed and is rejected.
3. The chat iframe does **not** inherit `prefers-color-scheme` from the embedding page, which is why the theme must cross via the chat URL — matching contract rule 3.
4. `darkpopout` is the only mechanism that produces dark deterministically without user action.

## Baseline checks

- `bash scripts/ci-check.sh`: **pass** (6 Node suites, 15 configuration tests, 51 release tests — counts as recorded, not copied from the plan).
- `git diff --check`: pass.
- Rust toolchain resolution: 1.98.1 (rust-toolchain pinned). Native cargo build/run blocked below.

## Native spike status

- **Blocked here:** native Linux run requires `webkit2gtk-4.1` dev libraries (missing) plus a display/OS-theme controller; Windows/WebView2 is the release target and is not available on this machine. The application-owned webview theme-propagation check and the signed-in/live-change chat checks are therefore **blocked** pending the native platform owner running `docs/dark-mode/acceptance.md`.

## Protected-state changes

None. `web/parent.mpdviewer.com/index.html` untouched, no deployment, no Twitch registration, DNS, release, tag, or signing change. No credentials read or recorded.