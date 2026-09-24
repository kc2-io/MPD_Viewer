# Multiplatform desktop E2E test plan

Date: 2026-09-24. Status: implementation and hosted GUI validation in progress in
PR #22. See [implementation/evidence record](../setup/DESKTOP-E2E.md).
Planning baseline: main `7556f866eae58c0b993bb1fe64cab4c9fab32778`
(v0.1.0-alpha.4). Grid development remains paused.

## Goal and evidence boundary

Exercise MPD Viewer's actual Tauri GUI, Rust controller, SQLite persistence and
native viewer windows on GitHub-hosted Windows, macOS and Linux. User actions
must go through rendered controls; assertions must observe resulting UI/windows.
Do not replace `dispatch`/`get_state` with a mocked JavaScript bridge in this suite.

Controlled channel/API responses and local viewer documents make CI repeatable.
Such runs demonstrate application behavior under fixtures, not live Twitch login,
stream playback, chat posting, Turbo, points, streaks or credited watch time.
OS dialogs, hardware video/audio and signed-distribution behavior also require
separate acceptance evidence. Test builds are not the released signed binaries.

## Current starting point

- `.github/workflows/ci.yml` builds/tests Windows x64, macOS arm64/x64 and Linux
  x64, but does not launch the GUI. Keep these existing gates.
- `tests/*.browser.*` and `tests/browser_test.py` provide useful browser tests
  with mocked native calls. Reuse scenarios/selectors, not their mocked bridge.
- Demo mode provides deterministic live/offline channels and simulated players.
  It uses `player-*` windows even when the default viewer mode is full-page.
  Demo coverage alone therefore misses the new `twitch-page-*` lifecycle.
- Settings currently use the application config directory; monitoring credentials
  use a fixed Windows Credential Manager target. Isolate both before local E2E.
- Windows has Connect Twitch and secure authorization persistence. Native Connect
  Twitch is currently unsupported on macOS/Linux, as is secure persistence;
  tests must assert those limitations rather than invent authentication parity.

## Driver and platform decision

Prototype WebdriverIO with `@wdio/tauri-service`, explicitly selecting its embedded
WebDriver provider. Current Tauri documentation recommends this route and documents
Windows, Linux and macOS support [1,2]. It requires a test-only native plugin.
The older direct `tauri-driver` route supports Windows/Linux, not macOS [1].

Documentation contains differing defaults/setup descriptions. The spike must
verify published package availability, exact compatible npm/Cargo versions,
toolchain/MSRV and actual driver behavior; pin successful versions in lockfiles.
Do not depend on an implicit provider default or perform broad dependency upgrades.

| Lane | Runner | Native renderer | Proposed cadence |
|---|---|---|---|
| Windows x64 | windows-2025 | WebView2 | Every relevant PR and main push |
| macOS arm64 | macos-15 | WKWebView | Every relevant PR and main push |
| Linux x64 | ubuntu-22.04 | WebKitGTK under Xvfb | Every relevant PR and main push |
| macOS Intel | macos-15-intel | WKWebView | Scheduled, manual and release-candidate validation |

Use one desktop session and serial GUI specs per job. On Linux provision Xvfb,
a session bus and a lightweight window manager if required for focus/close tests.
Record display dimensions, scale, runner image and renderer versions. A hosted
runner must prove screenshot capture, typing, pointer actions, window selection,
close/resize and app restart. Do not infer desktop availability from compilation.

Gate the driver choice on two-window switching and drag behavior. Verify whether
input is native or DOM-synthesized; a synthetic drag is not evidence of OS pointer
behavior. Prefer an external native driver on Windows/Linux or a narrowly scoped
native automation supplement where needed. Record macOS limitations explicitly.
No paid driver or self-hosted infrastructure is assumed. If a platform cannot pass
the spike, report the concrete blocker; browser-only substitution is not a pass.

## Test build and fixture architecture

1. Add an optional, default-off `e2e-tests` Cargo feature for automation plugins,
   fixtures and isolation configuration. Build with the bundled frontend and
   custom protocol, using the same controller/presentation paths as production.
   Keep all test dependencies optional and feature-gated. Test binaries are
   unsigned diagnostics and must never enter release packaging.
2. Require an explicit per-run test root before initializing storage or profiles,
   or constructing any webview. The manager is declaratively configured in
   `tauri.conf.json`, so prove initialization order rather than assuming setup()
   runs early enough. Manager, viewers and auth fixtures must all use test storage.
   Use it for SQLite, WebView data, logs and fixture files. Namespace any native
   vault target with a random run identifier. Reuse only within that spec's
   restart sequence; separate specs start fresh. Fail closed if isolation is
   incomplete. Preserve production app ID, public client ID and normal paths.
3. Start with existing demo controls. Add a separate full-page fixture scenario
   that supplies deterministic monitoring responses and substitutes a local page
   at the viewer URL boundary, retaining full-page labels, capability decisions,
   native construction and destruction callbacks. Reuse scheduling and timers.
   A synthetic page must identify itself visibly as a fixture.
   Seed synthetic monitoring independently of the unsupported macOS/Linux Connect
   UI; this does not implement or certify real authorization on those platforms.
4. Keep network adapters replaceable only in the test build. Restrict fixture
   origins to the run's exact loopback server; do not add arbitrary URL overrides
   to production. Deny/record unexpected external app requests, including Twitch
   and analytics. Allow build-tool downloads before the test phase.
5. The automation server listens only on loopback with a per-run endpoint/access
   mechanism supported by the verified driver. Review its injection scope and
   disable automation access on untrusted viewer/login pages. If the driver
   cannot do that, use an external driver or separate non-instrumented security
   probe rather than giving remote pages a privileged bridge.
6. Drive ordinary actions through buttons, fields and pointer/keyboard input.
   Backend hooks may seed external fixtures or advance a test clock, but must not
   perform the behavior under test. Avoid general-purpose evaluator endpoints
   accessible to application web content. Use bounded readiness waits, not sleeps
   as proof of success. Test IDs supplement accessible names where needed.
7. Release exclusion must be checked: normal builds have no automation plugin,
   fixture override or test listener; test flags cannot activate them. Verify the
   resolved dependency/features and launch a normal build as a negative probe.
   This is additional verification, not a change to signing/release permissions.

## Scenario matrix

| Group | Actions and observable acceptance |
|---|---|
| Startup | Fresh manager renders; remains stopped; correct defaults; Start opens expected native fixture viewers. |
| Favorites | Add/remove/enable channels, invalid input, arrow ranking; correct priority and order persist after restart. |
| Drag ranking | Pointer drag by grip/row changes visible order; interactive controls do not start a drag; rank persists. Record native vs synthetic input. |
| Capacity/lifecycle | Two viewers, cap reduction, preemption, manual close/skip, Retry, Stop and manager exit; observe actual native handles/counts and eventual destruction, not only manager state. |
| Timers | Save Always/10/custom minutes, reject invalid values, expiry selects next eligible channel, pause/resume accounting, no replacement capacity overshoot. |
| Counts/errors | Controlled live/viewer-count updates, stale/unavailable state, recoverable open/network failures and retry UI. |
| Modes | Default full-page fixture has web-mode guidance and hides central media controls; `--embedded-viewer` retains embedded controls. Demo alone is insufficient. |
| Appearance | Manager light/dark render and theme transition without destroying viewers. Fixture theme is not Twitch's remembered Dark Theme acceptance. |
| Persistence | Favorites, rank, timer and limit survive full process restart; app does not auto-start playback. Local fixture profile sentinel survives only its test profile. |
| Auth/recovery | Fake OAuth service responses exercise Windows UI/recovery flows; real Windows vault stores only fake credentials under test target, restores then deletes on Disconnect. Assert unsupported Connect and persistence on macOS/Linux. |
| Security | Real viewer caller denied get_state/dispatch; full-page player_report and wrong-session/stale wrapper reports denied; blocked navigation/popup/download cases follow existing policies; no credentials in manager state/logs/artifacts. |

Start with startup, two-window lifecycle and preference restart on all three OSes.
Then add the remaining groups. Run one real one-minute timer expiry per platform
in the extended suite. A controlled clock can cover longer/race scenarios, but
must use the actual scheduler and be labeled separately from elapsed-time tests.
Retain exhaustive race/ordering coverage in existing Rust tests.

Security probes must travel through the real native IPC boundary. Driver execute
or backend shortcuts cannot establish denial from an unprivileged page. A local
fixture has a different origin from Twitch: maintain separate production-policy
tests and explicit evidence of which origin/capability combination was exercised.
Native theme changes and close events need native actions where WebDriver cannot
produce them; do not replace these assertions with CSS class or JS event changes.

## GitHub Actions delivery

- Add a dedicated `Desktop E2E` workflow with read-only repository permissions,
  ordinary `pull_request`, main push and `workflow_dispatch` events. Never use
  `pull_request_target` to run PR code. No signing environments, OIDC or secrets.
- Initially run the three primary lanes for all PRs; optimize path selection only
  after proving required-check reporting cannot leave checks pending or skip
  relevant changes. Keep existing protected checks unchanged during rollout.
- Run extended cases and Intel macOS on a schedule/manual dispatch; execute the
  four-platform extended suite against the exact release candidate before tagging
  once the suite is accepted. Add this to the release checklist without weakening
  existing tag/build/signing/publication gates.
- Pin action SHAs, tool versions and npm lockfile; use `npm ci` and locked Cargo
  builds. Cache dependencies/build outputs by OS, architecture, toolchain, lockfile
  and feature set. Never cache profiles, credentials or test result state.
- Initial ceilings: 45 minutes per job including cold build, 10 minutes for PR
  GUI specs, 20 for extended specs. Measure before tightening. Do not hide failures
  with blanket retries or `continue-on-error`; a retry is diagnostic and retains
  the original failure.
- Always upload a bounded JUnit report, screenshots for each app window on failure,
  driver/app logs and a manifest containing commit, platform/runtime/driver versions,
  fixture mode, test list and skipped checks. Artifact names include platform and
  run attempt; initial retention 7 days. No browser profile, SQLite vault contents,
  auth URL query strings or credential dumps. Video is optional after reliable
  screenshots; it is not a dependency for the first milestone.
- Finally blocks terminate only this run's recorded app/driver/fixture processes,
  clean only verified test-root paths and scoped vault entries, and preserve
  sanitized evidence even when a spec times out.

## Isolated implementation work packets

Use one coordinator, at most three workers and at most two production writers.
Assign exact leases/worktrees before work, following AGENT-RULES.md. Select available
models by complexity at execution time; do not change global model settings.

| Packet | Owner/complexity | Exclusive scope and deliverable | Dependency |
|---|---|---|---|
| E2E-01 | Native engineer, high | Disposable driver spike; three-OS launch/click/two-window/restart/input evidence; dependency decision | None |
| E2E-02 | Native engineer, high | Test-only startup, storage/profile/vault isolation, network/viewer fixture seams; Cargo/native files | E2E-01 |
| E2E-03 | UI/test engineer, medium-high | `tests/e2e/`, fixtures, WDIO package/config/specs; UI test IDs by explicit lease | E2E-01 contract; integrate after E2E-02 |
| E2E-04 | CI engineer, medium-high | `.github/workflows/desktop-e2e.yml`, runner scripts, reports; no release changes | E2E-01 findings; can overlap E2E-02/03 |
| E2E-05 | Independent reviewer, high | Isolation, plugin capability, release exclusion and lifecycle review; report only | Integrated E2E-02/03/04 |
| E2E-06 | Coordinator/QA, high | Hosted repeated runs, exact coverage matrix, rollout decision and maintained instructions | Review findings resolved |

Native lifecycle/controller/permissions files have a single writer. Coordinator
serializes Cargo lock changes and integrates native hooks before dependent specs.
Review fixes return to their owner, then receive independent re-review.

## Completion and rollout gates

1. Spike has real hosted GUI evidence on Windows, macOS arm64 and Linux; Intel
   extended lane is separately demonstrated. Each missing capability is named.
2. Initial smoke and then agreed scenario groups pass at the same commit. Force a
   test failure during harness development to verify nonzero CI status, useful
   artifacts and cleanup; restore the test before merging.
3. Observe five consecutive clean primary-matrix runs without assertion retries,
   plus one successful Intel extended run. This is a rollout threshold, not proof
   that future flakes cannot occur. Resolve nondeterminism before required status.
4. Independent review confirms test isolation, no production automation surface,
   no remote-manager capability expansion and no release packaging of test builds.
5. Only then propose adding the three named GUI checks to branch protection.
   Document runtime/cost, supported input operations, exclusions and reproduction
   commands. Do not mark skipped or fixture-only scenarios as native/live passes.

Planning does not authorize grid work, a new release, paid infrastructure or live
Twitch account automation. This document creates no workflows or application changes.

## References checked for planning

1. [Tauri WebDriver overview](https://v2.tauri.app/develop/tests/webdriver/)
2. [WebdriverIO Tauri service](https://webdriver.io/docs/desktop-testing/tauri/)
3. [WebdriverIO platform support](https://webdriver.io/docs/desktop-testing/tauri/platform-support/)
4. [Tauri GitHub Actions GUI example](https://v2.tauri.app/develop/tests/webdriver/ci/)
5. [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)

References describe available tooling, not a successful MPD Viewer E2E run.
