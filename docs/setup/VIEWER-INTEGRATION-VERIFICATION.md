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
recovery instruction referring to a disabled button. Regression tests cover the
first two relevant state-machine cases (including original failure reproduction).

## Executed checks

- Windows `cargo test --locked --workspace --features mpd-tabber/custom-protocol`:
  **84 tests passed** (39 core, 34 app, 9 Twitch, 2 vault integration).
- Node source suites including timer labels: **92 passed**.
- Python configuration: **17 passed**; release-helper tests: **51 passed**.
- Edge/Playwright mocked native bridge: timer **4 passed**; unified auth UI passed
  including runtime control visibility, theme guidance and failed-delete retry.
- Browser theme suite: manager, wrapper and hosted source both palettes/contrast
  and live switching passed. Remote content is mocked; this is not native evidence.
- Clippy and release-profile Windows build are being finalized after integration.

## Native acceptance status

Pending combined-build checks: startup with existing profile, successful real
Twitch authorization then restart without a prompt, explicit Disconnect/restart,
full-page remembered Dark Theme, short real assignment deadline and automation
pause/resume, and embedded launch-flag smoke test. Offline recovery and suspend
accounting have deterministic tests but no native outage/sleep observation yet.
No macOS/Linux runtime acceptance is claimed. CI platform builds are separate.

See [web viewer behavior and known assignment boundary](WEB-VIEWER.md),
[authorization evidence](AUTH-PERSISTENCE.md) and [timer evidence](STREAM-TIMER-VERIFICATION.md).
Existing profile identity and preferences are preserved. No tag, release, website
deployment, signing configuration change or force push is part of this task.
