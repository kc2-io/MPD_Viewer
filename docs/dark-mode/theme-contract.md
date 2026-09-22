# DM-001 — Frozen theme contract

**Status:** Frozen for implementation. DM-000 feasibility record: `docs/dark-mode/twitch-chat-feasibility.md`.
**Authority:** The operating system light/dark preference is the only theme authority. There is no manual selector, saved field, database migration, controller action, or wrapper-protocol message.

## Mode and persistence

- Mode: `system` only. Nothing is persisted; theme state is never written to storage, settings, or preferences.
- Theme must never affect: channel selection, capacity, playback state, audio, quality, telemetry, authentication, or persisted preferences. Rust remains the selection/controller authority.

## Mechanism

- Primary: CSS `prefers-color-scheme`. App-owned stylesheets declare `color-scheme: light dark` so native `form` controls, scrollbars, focus/selection follow the OS.
- Cross-boundary: `matchMedia('(prefers-color-scheme: dark)')` is used exactly once per chat install to map the OS theme into the official chat iframe URL. No other JS/theme bridge exists. If a supported webview fails to propagate `prefers-color-scheme`, that is a blocked defect and a separate minimal native bridge must be designed — not silently worked around in JS.

## Semantic tokens

Semantic custom properties, defined on `:root` with a dark default and overridden under `@media (prefers-color-scheme: light)`:

- `--bg`, `--panel`, `--line`, `--text`, `--muted`, `--accent`, `--green`, `--red`
- Input/select surfaces and placeholder, button surface + hover, focus ring (accent with adequate contrast in both themes), disabled opacity, tag/dot colors, scrollable backgrounds.
- The dark palette is the current one and must remain visually stable. The light palette uses darker, accessible cyan for text/focus/borders (brand cyan is preserved as a fill/logo color where legible).

Every visible state must have an intentional value in both themes; no dark-on-dark or light-on-light fallback may remain. WCAG 2.1 AA: 4.5:1 normal text, 3:1 large text and meaningful UI boundaries/focus indicators.

## Exact verified chat URL shapes

Frozen from DM-000 (fresh-profile deterministic):

- Light: `https://www.twitch.tv/embed/{channel}/chat?parent={hostname}`
- Dark: `https://www.twitch.tv/embed/{channel}/chat?parent={hostname}&darkpopout`

`theme=` is ignored by the separate chat embed and is rejected. Only the two shapes above may navigate.

Residual (non-blocking for this implementation, blocking for release acceptance): a signed-in profile's remembered theme may render the plain (light) URL dark. The native owner confirms identity/session behavior under `docs/dark-mode/acceptance.md`; the chat-only reload disclosure below always applies.

## Startup and live-change behavior

- Cold start: chat iframe URL reflects the OS theme at install time (one `matchMedia` snapshot).
- Live change: a real media-query `change` event that differs from the currently applied theme updates **only the existing chat iframe's URL**. Repeated/duplicate events are no-ops (deduplicated; zero navigation when the theme did not change).
- Timing: preference is immediate and deduplicated so a restored panel cannot show a stale theme. When the panel is collapsed the iframe still reloads on theme change (hidden iframes belong to the same document and their state is retained); the change applies immediately and is correct when restored.
- A system-theme change must never recreate or navigate the video element, replace the Twitch player object, call `play`, resume a manually paused/autoplay-blocked player, change volume/mute/quality, or change the native session.

## Chat-only reload disclosure

Changing the chat theme reloads only the chat iframe. This may discard an unsent cross-origin chat draft; the app does not claim draft retention. Video, player object, session, pause, audio, quality, collapse state, and channel are preserved.

## Native allowlist rule (`allowed_chat_url`)

Frozen rule for Rust:

- Scheme `https`, host exactly `www.twitch.tv`, no username/password, no port, no fragment.
- Path exactly `/embed/{assigned-channel}/chat`.
- The raw query must be byte-for-byte exactly one of two canonical strings:
  - `parent={exact current parent hostname}`
  - `parent={exact current parent hostname}&darkpopout`
- Anything else is rejected: reversed parameter order, percent-encoded keys or parent
  spellings, `darkpopout=`, `darkpopout=1`, duplicate keys, unknown `theme=`/extra
  parameters, empty `&&` segments, leading/trailing `&`, case-spelled keys.
  `chat.js` emits only the canonical shapes, so the smallest possible navigation
  allowance matches its output exactly.

## Fallback / stop behavior

- If `matchMedia` is unavailable or throws, chat installs with the light URL shape (same as today's behavior) and never navigates; no retry loop or timer-driven reload.
- If the OS theme events don't fire in a supported webview, app chrome still renders correctly on cold start per OS preference; live propagation is a blockable defect recorded under native acceptance.
- If native acceptance finds the signed-in remembered-theme case unacceptable at release time, the fix must go through an approved plan (no silent style injection into the Twitch frame, no broaden-website navigation, no combined-embed migration without a separate ADR/plan).

## Redirect/verification guard

`?darkpopout` must not be silently stripped or appended after navigation by any second navigation. A single `iframe.src` assignment per real transition.

## Evidence required per supported platform

For each advertised platform (see `docs/dark-mode/acceptance.md`): cold-start light and dark observed; live light→dark and dark→light transitions observed with no player recreation; chat-only reload bounded to one navigation; channel and `parent` unchanged; collapsed chat stays collapsed; demo makes zero Twitch requests.