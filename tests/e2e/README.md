# Desktop GUI E2E harness

This suite launches the real Tauri app with `e2e-tests` and drives rendered
controls through WebdriverIO's W3C client. It does not replace Tauri dispatch,
state, or the Rust controller with JavaScript mocks.

```powershell
npm ci --prefix tests/e2e
$env:MPD_E2E_BINARY = (Resolve-Path target/debug/MPD_Viewer.exe).Path
$env:MPD_E2E_OUTPUT_DIR = "$env:RUNNER_TEMP/desktop-e2e-evidence"
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
