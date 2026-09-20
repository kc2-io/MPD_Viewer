# Channel viewer counts

Base: `a22bda6a6b3ff652b882de784bd2c2f77afaedbe` (main after PR #15).

## Plan and ownership

Show Twitch audience counts beneath each channel in the manager and in the native
title bar of its active viewer window. Use existing authenticated Get Streams
polling; do not add API requests, scopes, website deployment, player reloads, or
remote-page permissions.

- Coordinator: Twitch/core/controller/model/player, manager UI/CSS, CI test list,
  documentation and integration.
- UI test agent: exclusively `tests/viewer-count.test.mjs` and
  `tests/viewer-count.browser.cjs`, with the native bridge mocked.
- Independent reviewer: read-only source review and verification. Inherited
  session model/effort; no model overrides.

## Behavior

Twitch's [Get Streams response](https://dev.twitch.tv/docs/api/reference/#get-streams)
includes `viewer_count`. The existing 30-second monitoring poll now retains it.
It remains optional, ephemeral metadata; preferences and selection policy do not
use it. Known zero displays as `0 viewers`; missing metadata is hidden.

The manager renders `1,234 viewers` beside live status. Native viewer titles use
`channel · 1,234 viewers · MPD Viewer`. Title synchronization runs on the existing
one-second controller tick and changes the title only when it differs. This also
recovers if the hosted page changes its document title. It never reloads playback.

Failed/stale observations retain the last count with `(stale)`. The first
successful observation missing a channel clears its count immediately, while the
existing two-observation offline selection policy remains unchanged. Disabling a
channel clears its count; re-enabling waits for another observation. Demo never
fabricates a viewer count. Generation and authentication-epoch guards continue to
reject obsolete results. Counts update normally during paused selection because
monitoring continues. Stopping or disconnecting marks retained observations stale.

Channel metadata wraps within its column at narrow widths. Count refreshes during
a rank drag follow the existing DOM freeze and render when the drag finishes.

## Verification

- `cargo test --locked --workspace --features mpd-tabber/custom-protocol`: 40 passed.
- `cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol`:
  passed; three pre-existing controller style warnings remain.
- `node --test tests/view-model.test.mjs tests/viewer-count.test.mjs tests/hosted-source-characterization.test.cjs tests/hosted-player-adapter.test.cjs tests/chat.test.cjs`:
  55 passed.
- `python tests/configuration_test.py`: 15 passed.
- `python scripts/release-tools.py check-version`: unchanged `0.1.0-alpha.2`.
- `node tests/viewer-count.browser.cjs`: six passed in Edge 153.0.4234.32, mocked
  native bridge. Covers zero/singular/grouped/missing/stale counts, updates, demo,
  actual mouse dragging during refresh, arrows, and 860px layout boundaries.
- `node tests/ranking.browser.cjs`: all 15 ranking browser regressions passed in
  Edge 153.0.4234.32 with a mocked native bridge.
- `python -m unittest discover -s tests -p 'release_test.py' -v`: 51 passed.
- `cargo build --locked --release -p mpd-tabber --features custom-protocol`: Windows
  x64 compilation succeeded. The local preview is unsigned; no runtime result is
  inferred from compilation.
- Independent review: no remaining source blockers after timer throttling and
  narrow-layout fixes.

## Native acceptance still required

Browser mocks and title-format unit tests do not prove live Twitch observations or
native title updates. With the unsigned Windows preview, connect and start a live
favorite, check count in the channel list and viewer title, and compare again
after a successful poll. Confirm title updates retain playback/chat and user pause,
and verify stale labeling during a temporary network failure. The native title bar
is not visible while the viewer is fullscreen. macOS/Linux runtime behavior remains
unverified. No release, version tag, signing operation, or website deployment is
part of this feature.
