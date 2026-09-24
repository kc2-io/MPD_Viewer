# Desktop GUI end-to-end testing

Implementation status: in review in PR #22. Hosted cross-platform acceptance and
repeatability runs are in progress; this record does not claim they have passed.
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

## Driver decision and limits

Published `tauri-plugin-wdio-webdriver` 1.4.0 was inspected before integration.
Its embedded server supports native windows on all three OS families, but its
desktop input is DOM-synthesized. Dropdown and HTML5 drag helpers are therefore
explicitly synthetic GUI tests of the actual application handlers, not OS pointer
acceptance. Native close/theme tests call native window APIs through a narrow,
test-only file mailbox; they do not claim titlebar or OS Settings interaction.

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

## Acceptance record to complete

- Local Windows: normal 84 Rust tests passed; test-build manager-origin regression
  passed; GUI smoke, fake auth and driverless native IPC probes passed.
- Local extended harness: a real one-minute timer rotated, and an intentional
  assertion failure produced nonzero status, screenshots, JUnit and cleanup.
- First hosted experiment: Windows passed its then-current GUI suite; macOS and
  Linux launched the GUI but exposed stale row-element assertions during refresh.
  The assertion was changed to one read-only DOM snapshot; reruns are pending.
- Normal-build runtime probe, four-platform extended acceptance and five clean
  primary-matrix repetitions still need recorded hosted results.

Do not add required branch checks until the plan's repeatability and independent
review gates are met. No grid feature, stable release or website deployment is
part of this work.
