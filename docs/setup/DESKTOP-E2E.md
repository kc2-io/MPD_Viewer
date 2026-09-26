# Desktop GUI end-to-end testing

Implementation and rollout evidence: [PR #22](https://github.com/kc2-io/MPD_Viewer/pull/22).
The PR records the tested commit, hosted run attempts and repeatability outcome;
the existence of this workflow alone does not establish acceptance.
The design is in `docs/feature-plans/MULTIPLATFORM-E2E-PLAN.md`.

## What runs

`Desktop E2E` launches a native Tauri binary with its actual Rust controller,
SQLite preferences and native viewer windows. It uses deterministic demo data
and a separate local full-page fixture path. No Twitch account, stream or signing
credential is used. Existing compilation and release gates remain unchanged.

The primary PR matrix is Windows x64, macOS Apple Silicon and Linux x64. Manual,
scheduled and PRs labeled `desktop-e2e-extended` also run Intel macOS and extended
cases. That label is for test selection, not an approval or release bypass.

Run instructions and the precise test-driver limitations are in
[`tests/e2e/README.md`](../../tests/e2e/README.md). `MPD_E2E_EXTENDED=1` adds a real
elapsed one-minute rotation and longer recovery checks. Every job captures bounded
JUnit, JSON manifests, native application logs and screenshots. Test profiles and
credential stores are never artifacts.

## Visual evidence rollout

The first rollout records the disposable desktop only while the GUI harness runs,
without microphone or system audio. It uses bounded one-minute H.264 segments and
captures numbered per-case PNGs while preserving the previously active native
window. Recorder failure is reported separately from the GUI result during this
advisory phase. Every platform artifact also carries JUnit, runtime/test manifests,
sanitized logs, hashes, staging decisions and an independent media-validation
report. The job summary links the exact GitHub artifact and its GitHub-provided
digest; retention remains seven days.

Hosted runner images do not currently include FFmpeg. The workflow therefore
downloads the fixed `eugeneware/ffmpeg-static` `b6.1.1` FFmpeg and FFprobe assets
for its exact OS/architecture, verifies committed SHA-256 values before bounded
decompression under `RUNNER_TEMP`, and checks the required capture backend and
H.264 encoder. Tool paths, versions and executable hashes are retained in the
runtime and test manifests.

The pull-request reporter is deliberately split from the untrusted PR workflow.
It runs reviewed default-branch code on `workflow_run`, reads only GitHub job and
artifact metadata, and never downloads PR artifacts. The first deployment used
`contents: read`, `actions: read` and `pull-requests: read`; its hosted dry run was
observed before the separately reviewed rollout granted only `pull-requests: write`.
The reporter now creates or updates one bot-owned PR comment while the same evidence
remains downloadable from the Actions run and job summary.
Association-less runs are matched against the unique PR that was active when the
run began, and both base and head are rechecked before a comment write. Editing a
PR triggers a fresh capture; the comment identifies the base SHA as of reporting.

Design, threat model, implementation packets and acceptance gates are recorded in
[`E2E-VISUAL-EVIDENCE-PLAN.md`](../feature-plans/E2E-VISUAL-EVIDENCE-PLAN.md).

## Driver decision and limits

Published `tauri-plugin-wdio-webdriver` 1.4.0 was inspected before integration.
Its embedded server supports native windows on all three OS families, but its
desktop input is DOM-synthesized. Dropdown and HTML5 drag helpers are therefore
explicitly synthetic GUI tests of the actual application handlers, not OS pointer
acceptance. Native close/theme/resize tests call native window APIs through a narrow,
test-only file mailbox; they do not claim titlebar or OS Settings interaction.
The native resize API plus observed viewport dimensions avoids a driver 1.4.0
`setWindowRect` race: duplicate move/resize events can invoke Tauri's consumed
one-shot listener and panic. A process crash remains a test failure.

The harness uses the direct WebdriverIO client instead of `@wdio/tauri-service`:
the service installs mocking/evaluation wrappers that this suite does not need.
Application actions still cross ordinary Tauri IPC into the production controller.

The embedded server is loopback-only but unauthenticated, exposes evaluation,
and installs handlers on every webview. On macOS it also replaces the WebKit UI
delegate. Consequently, instrumented builds load only authored local fixtures,
and must not be used for live Twitch login or for production popup-policy claims.
Separate driverless processes exercise original native IPC permissions from two
wrapper and two full-page fixture windows, including positive own-session wrapper
reports and negative manager/wrong-session calls. These are local-origin checks,
not proof of behavior on a live Twitch origin.

Concurrent wrapper loading exposed a tiny_http worker-pool starvation defect.
The project vendors version 0.12.0 with a minimal queue-accounting correction used
by both normal and test builds. Its provenance is in `vendor/tiny_http/MPD-PATCH.md`;
a bounded regression exercises the exact vendored worker module. Unsuccessful
per-session asset-URL and macOS no-op wake experiments were removed, so the GUI
suite again uses the ordinary bundled asset URLs and event-loop behavior.

## Isolation and production boundary

The non-default `e2e-tests` feature is required for the driver, fixture services,
profile/vault overrides, native test mailbox and policy probe. Startup requires an
absolute marked test directory and a random identifier before constructing any
webview; the declarative manager is constructed manually with equivalent settings
after isolation. Missing isolation fails before app startup.

Windows/Linux browser data and SQLite are scoped to the test root. macOS uses a
unique WKWebView data-store UUID because WKWebView does not support a custom data
directory. That isolated store is outside the root and is disposed of with the
GitHub-hosted VM. Local macOS runs may leave that named store behind; they do not
use the normal application store. Each scenario must have a distinct identifier,
reused only for its own restart checks.

Fake Windows credentials use a namespaced target. The test-only cleanup command
deletes that target; it cannot address the production credential entry. Windows
Connect/restore/Disconnect checks use synthetic OAuth replies. macOS/Linux check
their current unsupported Connect behavior rather than reporting auth parity.

CI checks resolved production Cargo graphs with and without default features.
The driver and fixture features must be absent. A separate normal-binary probe
runs only on disposable hosted accounts: supplying E2E flags/environment must
neither create a driver listener on the selected port nor modify the marked test
root. This is a bounded negative probe, not proof of GUI readiness or of absence
of all network listeners (the ordinary local player server is legitimate).

## Validation and rollout

- Local Windows: normal 85 Rust tests passed; test-build manager-origin regression
  passed; GUI smoke, fake auth and driverless native IPC probes passed.
- Local extended Windows harness: 26 cases passed, including a real one-minute
  timer with paused accounting, injected native-open failure/Retry, and API-outage
  recovery. An intentional assertion failure separately
  produced nonzero status, screenshots, JUnit and cleanup.
- Hosted experiments found stale row references and a fresh-document driver race.
  Ranking observations now use one read-only DOM snapshot. Fresh windows wait for
  their final document title using the driver's atomic getter before its two-step
  script evaluator; expected initialized content is then checked independently.
- Independent review covered startup/recovery ordering, bounded shutdown,
  scenario-specific identities, production exclusion, driver limitations and CI.
  Unresolved process shutdown fails the suite and preserves its isolated root.
- Record four-platform extended acceptance and five consecutive clean primary
  matrices in PR #22 with run/attempt links, exact commit and measured duration.
  Include normal-build runtime-probe results and bounded failure evidence.

Do not add required branch checks until the plan's repeatability and independent
review gates are met. No grid feature, stable release or website deployment is
part of this work.

## Deferred runtime coverage

The initial rollout does not complete every scenario in the original matrix.
Follow-up native security work must cover stale-report handling after assignment
teardown and production-origin navigation, popup and download policies. Current
policy unit tests remain required; bundled/local driverless IPC checks do not
substitute for those runtime cases. Native OS input fidelity and live Twitch
acceptance remain separate from this fixture harness.
