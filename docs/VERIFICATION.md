# Verification record — MPD Tabber POC 0.1.0 + hosted-parent update

Date: September 17, 2026. All passing non-native suites below were rerun after configuring the supplied HTTPS player URL. The registered Client ID was retained.

## Executed checks

| Suite | Actual outcome | What it establishes |
|---|---:|---|
| `node --test tests/view-model.test.mjs` | 12 passed | Presentation-state labels, telemetry freshness, and observed-playing calculations. |
| `python3 tests/configuration_test.py` | 14 passed | HTTPS URL validation, matching compiled URL/permissions, transactional rejection of invalid configuration, local reset, narrow capabilities, and Client ID/default source wiring. |
| `MPD_TEST_CHROMIUM=/usr/bin/chromium python3 tests/browser_test.py` | 25 passed | Management UI behavior with mocked native IPC; player-wrapper behavior with a mocked Twitch SDK; explicitly simulated demo. |
| **Total** | **51 passed** | Non-native source checks only. |
| `python3 tests/hosted_wrapper_test.py` | Blocked before assertions; not counted | Browser environment returned `ERR_BLOCKED_BY_ADMINISTRATOR` for HTTPS-origin route-mock navigation. |

JavaScript syntax checks also passed. JSON/TOML/plist files were parsed, and embedded asset paths were checked. These are static integrity checks, **not a Rust build**.

`browser-test-results.json` records individual browser checks. `test-output.txt` records the final executed commands and output.

The existing browser checks include Client ID prefill and submission. Five new configuration checks cover a root HTTPS URL, compiled origin/capability agreement, setting both files while preserving manager permissions, local reset, and invalid input leaving configuration unchanged. These checks do not execute Rust, load SQLite, or authorize with Twitch.

The browser checks cover command dispatch for reordering, volume/mute, add validation, pause, and stop; requested audio before the first SDK play call; blocked playback; telemetry that does not repeatedly call play; zero volume versus mute; and demo isolation from Twitch media requests.

## What was NOT executed

| Gate | Status |
|---|---|
| Rust dependency resolution / `Cargo.lock` generation | Not run |
| Native Rust compilation | Not run |
| Included pure Rust core tests (15) | Written, not run |
| New native Rust/SQLite settings tests (6) | Written, not run; covers default/missing/blank/whitespace/custom/saved IDs |
| `cargo fmt` / `cargo clippy` | Not run |
| Windows WebView2 runtime | Not run |
| macOS WKWebView runtime | Not run |
| Linux WebKitGTK runtime | Not run |
| Native command ACL enforcement / IPC origin behavior | Not run |
| SQLite persistence through the native application | Not run |
| Real device authorization / refresh / Helix requests | Not run |
| Real Twitch embedded video, ads, account restrictions | Not run |
| Deployed HTTPS wrapper | Owner reports deployment at `https://parent.mpdviewer.com/`; web fetch and container DNS resolution failed here, so remote contents/TLS/headers could not be inspected |
| Background/minimized playback and sleep/wake | Not run |
| Native capacity transitions / cleanup / crash behavior | Not run |
| Installers, signing, notarization | Not run |
| GitHub Actions matrix | Workflow supplied, not executed |

There is no native executable included in the archive. Compilation or runtime integration errors may remain.

## Environment and limitations

The hosted-parent update environment has Node 22, Python 3.13, Playwright, and system Chromium, but no Rust/Cargo toolchain. No native build was attempted in this update.

The 25 passing browser checks used in-memory documents and mocks; no real browser navigation to Twitch or to a production/loopback wrapper was tested. The harness strips CSP and replaces module loading for instrumentation; native IPC and the Twitch SDK are simulated. Consequently, passing these tests does **not** demonstrate that HTTP/HTTPS loads, CSP, permissions, native window construction, Rust scheduling, or Twitch playback work together.

The test harness uses `--no-sandbox` only for its root-owned headless Chromium process in this container. **The desktop application does not disable the webview sandbox.** Run acceptance tests using normal browser security settings.

`ui-preview.png` and `player-preview.png` are screenshots of that mock/in-memory environment and are labeled accordingly in the README. They are not evidence of real viewing sessions.

## Client ID configuration status

The supplied public identifier is configured in native defaults and the connection form. Its Twitch registration and Public client type have not been inspected or validated; no account was authorized. The empty-settings upgrade is implemented with six native regression tests, but their runtime behavior remains unverified until Rust tests can run. See `CLIENT-ID-UPDATE.md`.


## HTTPS-host configuration status

The source points to the owner's supplied host and grants only player reporting on that exact HTTPS origin. Demo retains localhost. Wrapper source files are unchanged. The deploy ZIP is a local artifact only; no remote files were uploaded or replaced.

The public website could not be retrieved by the web tool (`Cache miss`); the container fetch failed with `Could not resolve host: parent.mpdviewer.com`. This is an environment-limited result, not a finding that the site is broken or its DNS is incorrect.

An additional browser suite was written to load unmodified wrapper assets and retain their CSP using mocked HTTPS responses. Its first navigation failed with `net::ERR_BLOCKED_BY_ADMINISTRATOR` before assertions. The environment restriction was not bypassed. `hosted-wrapper-test-results.json` records a blocked run, zero passed assertions, and the distinction from the 51 successful checks. Run it locally only as an additional mock test, not as a substitute for the deployed-site/native acceptance matrix.

See `HOSTED-PARENT-UPDATE.md` for the root asset layout, smoke-test fragment, and exact file hashes.

## Required next validation

Follow `MANUAL-TESTS.md`. Record OS version, architecture, actual resolved dependencies, webview/runtime version, wrapper origin, and observed results. A Rust build passing does not close the playback feasibility gate; each operating system still needs real playback and lifecycle tests.
