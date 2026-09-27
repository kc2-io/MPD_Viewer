# MPD Viewer product contract

These are durable approved boundaries. They describe required product behavior
and authority, not a claim that every proposed feature is implemented. Check
`PROJECT-STATE.md` and current code for implementation status.

## Identity and compatibility

- The visible product is **MPD Viewer** with the tagline
  **View fav channels in priority**.
- Preserve `com.modpackdad.mpdtabber.poc`, existing preference locations,
  browser-profile identity, the registered public Twitch client ID, and
  `https://parent.mpdviewer.com/` unless an explicit migration is approved.
- Backward compatibility and user data take priority over internal renaming.

## Authority and viewer modes

- Rust owns favorites, ranking, selection, capacity, assignments, timer policy,
  persistence, native surfaces, and session/generation identity.
- Remote documents render native assignments. A channel string or window label
  received from web content is never sufficient authority.
- Full Twitch channel pages are the default. They use Twitch's own supported
  playback, appearance, volume, and quality controls.
- Embedded mode is an explicit fallback selected with `--embedded-viewer`. Its
  wrapper chat, telemetry, volume, and preferred-quality features must not be
  advertised as capabilities of full-page mode.
- Keep keyboard/visual focus, audio policy, and playback intent distinct.
  Changing focus or mute must not silently resume a user-paused or
  autoplay-blocked stream.

## Twitch, authentication, and user trust

- Monitoring authorization and the website session used by viewers are separate
  identities and lifecycles. Disconnecting monitoring tokens does not imply
  deletion of browser-profile website data.
- Use Twitch's official pages, player, and chat with normal browser security.
  No ad blocking, artificial activity, unsupported private APIs, credential
  capture, cookie import/export, or browser-security bypasses.
- Do not inject application scripts into full Twitch pages to control unsupported
  volume, quality, bitrate, rewards, or account behavior.
- Do not promise Turbo recognition, ads, channel points, streaks, drops, viewer
  credit, or account persistence. Record only what was observed in a stated live
  native environment.
- Login/popup surfaces have no manager capabilities. Navigation, popups,
  downloads, origins, IPC commands, and telemetry are narrowly allowlisted.

## Timers and lifecycle

- Timers limit a selected favorite's turn only when another eligible favorite
  can use the capacity. Stop and close always win over rotation.
- A timed assignment with no eligible live alternative remains selected and
  receives a fresh turn rather than appearing permanently overdue. This is an
  approved target contract; `PROJECT-STATE.md` identifies whether its current
  implementation has merged.
- Failed opens, deferred favorites, stale reports, capacity changes, and multiple
  simultaneous expirations must preserve ranking/fairness and never create
  duplicate playback.
- Mode changes and replacements close the old surface before opening a
  replacement and reject stale callbacks. If recreation is required, preserve
  logical assignment and disclose the reload.

## Hosted source and future architecture

- `web/parent.mpdviewer.com/index.html` is a provenance snapshot of supplied
  source, not a deployment input or an automatically compatible wrapper.
- Protocol, chat, authentication, or grid changes must follow
  `docs/HOSTED-SOURCE-PLAN-ADDENDUM.md` and use actual native evidence.
- The grid, multi-surface protocol, reset APIs, and other contracts under
  `docs/feature-plans/` are proposed designs until implemented and verified.
  Do not present them as current behavior.

## Evidence

- Unit and browser-mock tests establish logic, rendering, and message contracts
  only within their fixtures.
- Native compilation does not establish runtime playback, login, audio, window
  lifecycle, signing, or cross-platform acceptance.
- Fixture desktop E2E establishes native application/controller behavior under
  controlled local content, not live Twitch behavior.
- Signing, packaging, downloaded-artifact verification, and live manual
  acceptance are separate evidence and must be reported separately.
