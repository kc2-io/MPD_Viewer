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

The test-only macOS observer posts a no-op through Tauri's native event proxy
every 100 ms, with at most one pending task, to wake headless WebKit loading.
Driverless probes use this same event-loop wake-up without registering the driver
or changing page permissions, focus, visibility or background-throttling policy.

The bundled fixture experiment gives each viewer's five local asset requests a numeric
`e2e_session` query. The script bytes, origin and CSP are unchanged; only the
test-feature HTML/URL varies. This separates simultaneous fixture resource loads
while investigating WebKit's stalled deferred scripts. A pass with this variation
does not establish that ordinary production same-URL loading is fixed.

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

- Local Windows: normal 84 Rust tests passed; test-build manager-origin regression
  passed; GUI smoke, fake auth and driverless native IPC probes passed.
- Local extended Windows harness: 25 cases passed, including a real one-minute
  timer and API-outage recovery. An intentional assertion failure separately
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
