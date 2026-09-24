# Desktop GUI E2E harness

This suite launches the real Tauri app with `e2e-tests` and drives rendered
controls through WebdriverIO's W3C client. It does not replace Tauri dispatch,
state, or the Rust controller with JavaScript mocks.

```powershell
npm ci --prefix tests/e2e
$env:MPD_E2E_BINARY = (Resolve-Path target/debug/mpd-tabber.exe).Path
$env:MPD_E2E_OUTPUT_DIR = Join-Path $env:TEMP ("mpd-e2e-evidence-" + [guid]::NewGuid().ToString("N"))
npm test --prefix tests/e2e
```

Use the actual compiled executable filename on each OS. Build separately with
`cargo build --locked -p mpd-tabber --features custom-protocol,e2e-tests`.
Linux requires a desktop session, for example Xvfb plus a session bus/window manager.
`MPD_E2E_EXTENDED=1` includes a real elapsed one-minute rotation. Set
`MPD_E2E_FORCE_FAILURE=1` only for a diagnostic run proving nonzero status,
failure reports, screenshots, and cleanup; never leave it enabled in required CI.

The harness creates a fresh temporary root and exact ownership marker before
launch, supplies `MPD_E2E_ROOT`, `MPD_E2E_RUN_ID`, `MPD_E2E_SCENARIO` and
`TAURI_WEBDRIVER_PORT`, and preserves that root only across its own restart test.
It deletes only that marked root, and terminates only the app child/tree it owns.
Do not point this runner at a production executable: isolation is enforced by the
optional native test build before any storage or webview construction.

The published service 1.4.0 was inspected. Its worker installs mocking command
overrides and wraps script evaluation, so this harness uses `webdriverio.remote`
directly against the embedded plugin instead. No extra JavaScript Tauri plugin or
mock bridge is installed. The npm lockfile pins the client dependency tree.

Evidence contains a bounded JUnit report, manifest, sanitized application logs
and per-window PNGs. Never upload the temporary root, SQLite database, browser
profile, credentials or raw environment. App child environments use an OS/display
allowlist. No Twitch account is used, and external navigation is disabled by the
native test build.

The embedded driver synthesizes DOM input. These tests establish rendered GUI,
native-window lifecycle and backend behavior under fixtures, **not native OS
pointer/drag fidelity, titlebar close events, Twitch login/playback or rewards**.
The manifest enumerates exclusions rather than silently skipping acceptance.
Platform runtime results must be recorded separately from this harness source.

Dropdown and HTML5 drag coverage uses explicit synthetic DOM input helpers because
the embedded driver's `option.click()` does not change a select, and its mouse
events do not initiate HTML5 drag. These helpers target rendered controls and
trigger their existing handlers; they never call the native action bridge.
Native theme and close coverage uses the narrow file mailbox to call native
window APIs, so it proves theme propagation / CloseRequested handling, not an OS
settings click or a titlebar click. The full-page negative IPC probe invokes
real Tauri IPC from the instrumented fixture caller and checks permission denial;
it does not replace a non-instrumented production-origin security probe.

Windows fake OAuth coverage uses only local activation and synthetic tokens,
then restarts, verifies Windows Credential Manager restoration and Disconnect.
Other platforms assert the existing unsupported Connect behavior. Cleanup invokes
`--e2e-cleanup` with the exact owned root/run ID, deleting only that namespaced
fake vault record even after failure. The macOS WebKit data store uses an isolated
UUID outside the test root; hosted runner teardown removes it. Local macOS users
should account for that isolated store remaining after the harness root is removed.

`webdriverio` is pinned to 9.32.0. A scoped override pins
`@puppeteer/browsers` to 3.2.3 because WDIO's 2.x dependency retains vulnerable
`extract-zip`; 3.2.3 removes that dependency. Node >=22.12 is required. The harness
uses a prebuilt app and existing embedded driver, never browser downloads. The
imported client and actual Windows session were exercised with the override;
`npm audit` reported zero findings when the lockfile was generated.

The suite also launches separate driverless policy-probe processes. Those native
fixture probes exercise actual caller IPC permissions without the
WebDriver plugin being registered. The probe writes only bounded booleans/errors
to `policy-demo.json` / `policy-web.json`; a nonzero native exit fails the suite.
They use bundled/local test origins, not a live Twitch domain. Fixture seeding in
that security probe is not GUI-action evidence.

Each demo/web/auth/policy case gets a different random run ID as well as a fresh
root, so Windows vault targets and macOS UUID profiles cannot cross case
boundaries. The same ID/root is retained only for that case's restart assertions.

The manifest distinguishes `harnessCommit` (and dirty status) from the compiled
binary. It records the binary SHA-256 and fails if its bytes change during a run.
Set `MPD_E2E_BUILD_COMMIT` to a verified 40-character native source commit when
building elsewhere; otherwise that field is null. A local harness worktree commit
alone is not evidence of the compiled application's source revision.

Shutdown retains each owned child handle until exit is confirmed. Unix cleanup
escalates an unresponsive owned process group from SIGTERM to SIGKILL with bounded
waits. If shutdown or scoped credential cleanup remains unresolved, the runner
fails and retains the marked test root for recovery instead of deleting an active
profile. Reports record cleanup success and whether the root was removed.

Fresh-document readiness first polls W3C `getTitle()` for the exact final HTML
title, then checks the expected rendered content. Source inspection of
`tauri-plugin-wdio-webdriver` 1.4.0 (`src/platform/executor.rs`, `get_title` and
`execute_script`) found that getTitle uses one document-title evaluation, while
execute/sync stores a result in a window-global and polls it in another evaluation.
Initial navigation can discard that global and time out even when the final page
subsequently renders. This source-based race mitigation adds no action retries,
arbitrary readiness sleeps or larger command timeouts. Platform evidence must
still demonstrate it fixes the observed hosted failures.

Viewer readiness diagnostics retain only the final bounded observation per window:
WebDriver result type, document readyState/title, expected fixture channel, demo
hidden flag, bridge text, heading, visibility/focus, initialization function types
and an allowlisted set of resource basenames/status/timings (no resource URLs). These appear in the manifest and readiness
failure message. Complete-document, expected-channel and visible-demo/fixture
marker assertions are unchanged; a screenshot alone is not substituted for them.

On a failed demo/embedded readiness assertion only, a diagnostic may click the
manager's existing Focus button once for the exact allowlisted fixture channel
read from the failing window URL fragment. It preserves the original observation
and samples readiness for at most five seconds after focusing that same native
window. The `afterFocus` diagnostic records any recovery or diagnostic error, but
**always rethrows the original failure**; it is not a passing-test retry or a
normal-suite focus workaround. No raw URL or fragment is retained.
