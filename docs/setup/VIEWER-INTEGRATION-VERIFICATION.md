# Viewer integration verification — 2026-09-24

Integration branch: `feat/viewer-next`, base main `2ea2960`, merged spike `a3f3bd2`.
This is development work, not a signed release or a rewards-credit certification.

## Implemented and reviewed

- Default full Twitch pages; exact `--embedded-viewer` runtime fallback in one binary.
- User-approved Twitch-owned Dark Theme setting, with manager guidance. Native
  spike observed switching page and chat light to dark through the official UI.
- Twitch-owned web volume/quality controls; no supported full-page control API
  found. Central controls remain available for embedded and demo modes.
- Optional Always/10-minute/custom assignment timers; native controller owns time,
  safe close-before-open capacity, failure recovery and bounded rotation waves.
- Windows Credential Manager OAuth persistence, startup validation, rotated-token
  save/retry, epoch-safe cancellation/deletion and hourly validation while stopped.

Independent read-only review covered vault security, controller recovery, runtime
capability separation and timer lifecycle. Review found and corrected a target
freshness change during native close, redundant data-source recovery, and a
recovery instruction referring to a disabled button. Regression tests cover timer target changes and authorization recovery state
(including the original timer failure reproduction).

## Executed checks

- Windows `cargo test --locked --workspace --features mpd-tabber/custom-protocol`:
  **84 tests passed** (39 core, 34 app, 9 Twitch, 2 vault integration).
- Node source suites including timer labels: **92 passed**.
- Python configuration: **17 passed**; release-helper tests: **51 passed**.
- Edge/Playwright mocked native bridge: timer **4 passed**; unified auth UI passed
  including runtime control visibility, theme guidance and failed-delete retry.
- Browser theme suite: manager, wrapper and hosted source both palettes/contrast
  and live switching passed. Remote content is mocked; this is not native evidence.
- Clippy completed successfully with advisory style/dead-code warnings; release-profile
  Windows build succeeded. Authenticode status is NotSigned, as intended.
- PR #20 CI run `35970419952`: source and all four native platforms passed.

## Native Windows acceptance — executed 2026-09-24

Tested the unsigned Windows x64 executable built from code commit `cc60b37`,
SHA256 `85516bd8c27ff978bfc9b606544613a7694880bf23f04ab056a386e84a6d729d`.
Existing application/profile identity was retained. The user performed Twitch
password/authorization interactions; no credentials or cookies were inspected.

- Initial startup retained the existing 30 favorites and three-session limit,
  and stayed stopped until Start.
- After the user's authorization, a full process exit/restart restored the
  connected account without an authorization window. The connection also survived
  the embedded-mode restart and return to default mode.
- Default launch opened the full Twitch channel page. The page and chat retained
  the official Dark Theme setting previously selected through Twitch's own UI;
  visible website sign-in also survived.
- With a temporary one-session limit and a one-minute assignment timer, the active
  channel rotated to the next eligible live favorite. The old viewer disappeared
  and one replacement remained; no duplicate managed session was observed.
- A timer configured while automation was paused stayed at `1:00` across repeated
  observations. Resume accounting is covered by deterministic policy tests; this
  native check establishes paused-clock behavior, not every suspend/resume case.
- `--embedded-viewer` in the same binary exposed central volume/quality controls,
  opened three embedded player/chat layouts and received advisory player telemetry.
- Restored both temporary timers to Always and the limit to three, then returned
  to default web mode. Shutdown removed the managed windows on both test runs.

Explicit deletion of the user's new saved authorization was not performed. Native
Credential Manager deletion was verified with isolated fake credentials, and
controller cancellation/revocation/race paths have deterministic tests. Actual
network-outage recovery, OS sleep/resume, SPA/raid behavior and macOS/Linux GUI
behavior remain unobserved; compilation/CI is not runtime evidence for those cases.
No claim is made about Twitch rewards credit, automatic Windows-to-Twitch theme
matching, or full-page programmatic quality/volume controls.

See [web viewer behavior and known assignment boundary](WEB-VIEWER.md),
[authorization evidence](AUTH-PERSISTENCE.md) and [timer evidence](STREAM-TIMER-VERIFICATION.md).
Existing profile identity and preferences are preserved. No tag, release, website
deployment, signing configuration change or force push is part of this task.
