# Preferred video quality

Base: `69d8cf0255806159e2bed43eee63d91eb2c51dcf` (v0.1.0-alpha.3).

## Scope and ownership

The owner requested a main-screen preferred resolution with nearest-supported
fallback, for example 180p choosing 360p when that is a stream's lowest option.
The UI calls this **Preferred video quality**: these are vertical resolutions,
not a measured bitrate in kbps.

Coordinator owns the settings/controller/native/bootstrap and both player paths,
shared selection helper, UI, storage and adapter integration tests. The test agent
exclusively owns `tests/quality.test.cjs` and `tests/quality.browser.cjs`. Independent
review is read-only. Agents inherited session model/effort without overrides.

## Behavior

Session settings offers Auto (default), Source, 160p, 180p, 240p, 360p, 480p,
720p, 1080p, 1440p and 2160p. Existing settings default to Auto. The validated
preference persists in the existing settings store and applies to all current and
future viewers, without changing application/profile identity or schema version.

Both the hosted native adapter and bundled wrapper use `player-wrapper/quality.js`.
It uses the official [Twitch Player API](https://dev.twitch.tv/docs/embed/video-and-clips/)
`getQualities`, `getQuality`, `setQuality` and `isPaused`. Only an advertised quality
ID is passed to the SDK. The helper handles string IDs and `{group,name}` metadata.

- Choose the nearest advertised numeric resolution; ties prefer lower resolution,
  then lower frame rate at the same resolution.
- 180p selects 360p if 360p is lowest; it selects 160p if that is closer.
- Source selects the advertised source (`chunked`/`source`), or highest known
  resolution if no source ID is advertised.
- A source-only stream uses source. An unlabeled source height cannot be inferred:
  it does not compete with known numeric heights. If no numeric or source option
  is known, use advertised Auto; otherwise leave Twitch's current choice alone.
- Empty/not-ready metadata is retried on existing five-second observation ticks.
  Changed variant lists are reconsidered. Duplicate successful requests are avoided;
  thrown SDK failures can retry. Audio-only/invalid entries are excluded.
- Paused, blocked, offline and ended players retain the pending preference until
  normal playback resumes. Quality code never calls play, pause or reload.

Rust queues preference delivery until the document has reported initialization.
Loading/ready reports re-arm it after reload, and the existing one-second controller
tick delivers the current persisted setting rather than the opening snapshot.
No new IPC command/capability, remote permission, token, external API request, or
website deployment is introduced. The hosted source snapshot remains unchanged.

## Executed verification

- `cargo test --locked --workspace --features mpd-tabber/custom-protocol`: 42 passed,
  including actual SQLite round-trip, legacy defaults, invalid-save rejection and
  document readiness synchronization.
- `node --test tests/view-model.test.mjs tests/viewer-count.test.mjs tests/quality.test.cjs tests/hosted-source-characterization.test.cjs tests/hosted-player-adapter.test.cjs tests/chat.test.cjs`:
  80 passed, including 21 quality-selection cases and hosted adapter integration.
- `node tests/quality.browser.cjs`: seven passed in Edge 153.0.4234.48. Four manager
  checks cover selector/dispatch, mocked settings reconstruction, focus and 860px
  layout. Three bundled-wrapper checks serve actual HTML/scripts/CSP with a mocked
  Twitch SDK: nearest quality, existing player reuse, pause retention, source-only
  fallback and demo isolation. No CSP violations or JS errors.
- `python tests/configuration_test.py`: 15 passed.
- `python -m unittest discover -s tests -p 'release_test.py' -q`: 51 passed.
- `cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol`:
  passed with three pre-existing controller style warnings.
- `cargo build --locked --release -p mpd-tabber --features custom-protocol`: Windows
  x64 release-profile compilation succeeded; copied as an unsigned test preview.
- Version check remains `0.1.0-alpha.3`; no release/version change.
- Independent review cleared the readiness/reload fix; no remaining source blockers.

## Native acceptance

SDK mocks do not prove real Twitch rendition selection or native message delivery.
Test the unsigned Windows preview with multiple live channels: set 180p and inspect
each Twitch quality menu; verify the nearest available rendition, repeat at 720p,
try Source and Auto, pause a viewer before changing preference, resume manually,
reload an existing viewer, then restart and verify the saved setting. Test actual
source-only and delayed availability cases when available. Twitch controls actual
rendition availability and delivery. No native live playback claim is made here.
