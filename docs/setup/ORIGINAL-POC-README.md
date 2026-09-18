# MPD Tabber — proof of concept

**Source version:** 0.1.0 + hosted-parent configuration update · **Prepared:** September 17, 2026

A Rust/Tauri desktop proof of concept for selecting the highest-priority live Twitch favorites and maintaining a configurable number of managed viewing sessions. Default capacity: **3**. Volume and mute are separate controls.

> **Verification status:** source has been implemented, but the native Rust application has **not been compiled or executed** in the authoring environment. Rust/Cargo are unavailable here; no native build was attempted for this configuration update. **51 JavaScript/Python/browser-mock checks passed**; 21 included Rust tests (15 core and 6 SQLite-settings tests) remain unrun. Actual OAuth, Twitch playback, native window lifecycle, and Windows/macOS/Linux compatibility remain acceptance gates. This is a source handoff, not a verified executable or finished product.

The app owner's public Client ID remains preconfigured. Real Twitch players now use `https://parent.mpdviewer.com/`; Demo continues to use the bundled localhost wrapper. See [the hosted-parent update](docs/HOSTED-PARENT-UPDATE.md) for deployment requirements and [the Client ID update](docs/CLIENT-ID-UPDATE.md) for authentication details.

## What is included

- Pure Rust priority selection, stable-session reconciliation, offline debouncing, and stale-status rules.
- A single-owner Rust controller, SQLite settings, and managed native player windows.
- Reorderable favorites, enable/disable, capacity, global volume, mute, Start, Pause/Resume, Stop, Focus, Skip/Undo, and Retry.
- An offline-data **Demo** mode with explicitly simulated player events.
- A real Twitch API adapter: public-client device authorization, in-memory credentials, refresh/validation, batched live-status polling, pagination, and rate-limit handling.
- An official Twitch player wrapper with readiness, playback, blocked-autoplay, visibility, volume, and mute telemetry.
- The owner-provided HTTPS player origin preconfigured for Twitch mode, plus a bundled localhost wrapper for Demo.
- Narrow Tauri permissions separating manager actions from untrusted player telemetry.
- Build scripts, tests, an unexecuted cross-platform CI workflow, and a native acceptance checklist.

MPD Tabber reports selection and observed player state, **not viewer-count credit**. It does not spoof activity, bypass autoplay restrictions, automate chat, or claim that any tab/volume setting produces counted viewers.

## Start here

Install a current stable Rust toolchain and the [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/). Windows requires the C++ build tools and WebView2; macOS requires the appropriate Xcode tooling; Linux requires the desktop development libraries described in that guide. Node is **not required to build or launch this POC**; it is used only by the presentation tests.

From the extracted project directory, run:

```sh
cargo run -p mpd-tabber --features custom-protocol
```

Equivalent launch scripts:

```powershell
# Windows PowerShell, from the project directory
.\scripts\run.ps1
```

```sh
# macOS / Linux, from the project directory
sh scripts/run.sh
```

The first build needs network access to retrieve Cargo dependencies. There is no pre-resolved `Cargo.lock`: dependency resolution could not run here. After the first successful native build, review and commit the generated lockfile and use `--locked` for repeatable builds.

Build a release executable with:

```sh
cargo build -p mpd-tabber --release --features custom-protocol
```

Optional native installer build, **not verified here**:

```sh
cargo install tauri-cli --version "^2" --locked
cargo tauri build
```

Installer signing and macOS notarization are not configured. The native build must be performed and tested on each claimed platform. Do not disable system protections to force an unsuccessful build or playback test through.

## Demo walkthrough

On a fresh installation, Demo is selected and playback is stopped. Once the application successfully builds:

1. Click **Load demo**. It adds `alpha_demo`, `bravo_demo`, `charlie_demo`, and `delta_demo`; Bravo, Charlie, and Delta start as simulated live.
2. Leave capacity at **3**, then click **Start**. The intended result is three explicitly simulated native player windows.
3. Set Alpha to **Go live**. Because Alpha is first, Delta should close before Alpha opens. Bravo and Charlie should keep their session IDs.
4. Reorder favorites with arrows or drag-and-drop, lower the limit, and change volume/mute. Inspect the session cards and activity log.
5. In a demo player, exercise **Autoplay blocked**, **Paused**, **Player error**, and **Pause telemetry**. A stale last-known playing report must become unknown rather than remain counted as observed playback.
6. Pause automation to retain existing assignments. Resume to apply the current priority selection. **Stop** should close every managed player. Closing the manager should end the application and all its player windows.

These are intended native behaviors to verify, **not claims that the native scenario has already passed**. The included mock-browser tests exercise UI commands and wrapper events independently of the Rust controller.

Demo players do not load Twitch media. An existing API connection can still perform credential maintenance until disconnected; disconnect Twitch when a completely offline session is desired. Simulated broadcast states are not restored on restart.

## Real Twitch mode

### Authorize MPD Tabber

The public Client ID supplied for MPD Tabber is now the built-in default:

```text
ha94kk20cfu1tp74pgg8isgi88cpo7
```

Confirm in the Twitch developer console that this registration uses the **Public** client type; that setting has not been inspected here. Public-client device authorization does not require a client secret. Do not embed or enter a client secret.

The Client ID field is filled from native settings. Fresh installs, missing fields, and previously saved empty or whitespace-only IDs use the default. Existing nonempty custom IDs are preserved. Favorites, ordering, volume, capacity, and mode preferences are not reset. The migrated default is written to SQLite on the next successful settings save.

Stop playback and wait until managed windows are closed. Select **Twitch** as the data source, leave the prefilled Client ID in place, and click **Connect Twitch**. Use the displayed activation code and **Open authorization** button to complete Twitch authorization in the normal browser. Then add actual favorite channels and press **Start**. A client ID alone does not authorize an account.

The source requests no extra OAuth scopes because the manually supplied favorites workflow only needs stream status. Acceptance of the empty-scope device grant must be verified with the actual registered app. The supplied Client ID has been configured, but the registration has not been validated against Twitch; no account was connected and no live API call was tested in this update.

Client IDs are public application identifiers, unlike secrets or access tokens. See [Twitch application registration](https://dev.twitch.tv/docs/authentication/register-app/) and the OAuth references below.

Tokens are held in native process memory only. Reauthorize after restarting. API authorization does **not** establish a signed-in Twitch website session in the embedded player; account-dependent playback is unverified. Popup-based website sign-in is not implemented.

References: [Twitch OAuth/device flow](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/), [token validation](https://dev.twitch.tv/docs/authentication/validate-tokens/), [Get Streams](https://dev.twitch.tv/docs/api/reference/#get-streams).

### Player origin: hosted HTTPS wrapper

Real Twitch player windows are now configured to load:

```text
https://parent.mpdviewer.com/
```

That page must actually serve the POC's `player-wrapper/index.html`, with `player.js` and `style.css` alongside it. A placeholder/landing page, an HTTP redirect to Twitch, or a separate local page that merely declares this hostname as `parent` is not this integration.

The existing wrapper uses `parent: [location.hostname]`, which derives `parent.mpdviewer.com` when loaded from the configured page. It does not spoof the parent. Native credentials are never passed into the wrapper.

This source package already contains the origin configuration. To apply the same change to a separate existing checkout without replacing other work, run:

```powershell
py -3 scripts/set-player-url.py https://parent.mpdviewer.com/
cargo run -p mpd-tabber --features custom-protocol
```

On macOS/Linux, replace `py -3` with `python3`. Rebuilding is required. The script updates both `src-tauri/player-origin.json` and the player-only capability in `src-tauri/capabilities/players.json`; it does not deploy or modify the website. Manager permissions are unchanged.

Demo still uses the application-owned static server on IPv4 loopback, independent of the HTTPS host. In Twitch mode, an unavailable configured HTTPS host does not cause a silent fallback to localhost.

**Deployment status:** the owner reported deploying this hostname. This environment could not retrieve it: the web fetch failed and a container DNS lookup could not resolve it. This does not establish that the site is unavailable to the owner. Actual HTML, TLS certificate, response headers, assets, and playback could not be inspected. See [the hosting instructions](docs/HOSTED-PARENT-UPDATE.md) and the supplied standalone wrapper ZIP.

The root without a URL fragment will intentionally display a configuration error when it is serving this wrapper, because a channel and session ID are required. Use the smoke-test URL in the hosting instructions. An ordinary browser displays **Standalone page · no native telemetry**; that is expected outside the native app.

To explicitly restore the development-only localhost experiment, then rebuild:

```sh
python3 scripts/set-player-url.py --local
```

Use a valid HTTPS certificate, serve the final URL directly, and keep the wrapper's CSP intact. Additional HTTP response CSPs are cumulative and must permit the documented Twitch SDK, its iframe, and the native telemetry transport; do not strip security headers blindly. No extra analytics/scripts, login interstitials, or worker caches should be added to this player origin without review.

HTTPS and a truthful parent address the documented origin requirements; they do not prove general platform-policy approval, viewer credit, autoplay, or inactive/minimized playback. These remain separate native acceptance checks.

References: [Twitch embed requirements](https://dev.twitch.tv/docs/embed/), [video player SDK](https://dev.twitch.tv/docs/embed/video-and-clips/).

## Behavior and deliberate POC limits

| Area | POC implementation / limitation |
|---|---|
| Browser shell | Tauri 2. Browser-engine suitability is still provisional. |
| Playback layout | Separate native windows, as specified for the first feasibility milestone. No tabbed workspace yet. |
| Management UI | Small HTML/CSS/JavaScript frontend. Rust owns the core, API client, persistence, and controller. Leptos migration is deferred deliberately. |
| Defaults | 3 slots, 25% volume, unmuted, Demo source, no automatic start, MPD Tabber public Client ID prefilled; Twitch wrapper at `https://parent.mpdviewer.com/`. |
| Resource bounds | 1–1,000 slots and up to 2,000 favorites are validation bounds, not performance claims or Twitch counting rules. Start testing with 1–3 windows. |
| Identity | Normalized channel login, not stable Twitch user IDs. Rename migration and existence validation are deferred; a syntactically valid nonexistent channel may appear offline. |
| Polling | Normally every 30 seconds; up to 100 logins per request; complete pagination; one in-flight poll. |
| Offline | Two successful missing observations; failed/partial API calls never mark channels offline. |
| Stale data | Retain existing last-known-live sessions; do not open new sessions from stale or pending-offline observations. |
| Preemption | Retain unchanged sessions. Close outgoing windows before consuming their capacity with replacements. |
| Pause | Freezes automatic selection, not playback. Monitoring may continue. Explicit remove/disable/skip/cap reductions still apply. |
| Manual close | Skip that broadcast; Undo restores eligibility. Unexpected native window loss is currently treated the same way, not automatically classified as a renderer crash. |
| Retry | Explicitly closes/recreates a failed player when running. There is no automatic reload storm or silent unpause. |
| Stop | Stops channel monitoring and closes managed windows. An authenticated stopped app may still validate/refresh credentials hourly. |
| Credentials | Memory-only; no keychain integration or token persistence. Memory is not guaranteed to be zeroized. |
| Persistence | SQLite stores validated preferences only; current live status and skipped broadcasts are not restored. Import/export UI is deferred. |
| Observed playback | Based on bounded, recent wrapper telemetry. No report / stale report is not proof of playing. |
| Lifecycle | Native close/exit logic is implemented but must be tested; forced process kill, sleep/wake, and renderer crashes are not certified. |

## Tests and verification

Run the pure Rust tests without desktop dependencies:

```sh
cargo test -p mpd-core
```

Compile/test the workspace on a machine with the native prerequisites:

```sh
cargo test --workspace --features mpd-tabber/custom-protocol
cargo fmt --all
cargo clippy --workspace --all-targets --features mpd-tabber/custom-protocol
```

Run the non-native tests:

```sh
node --test tests/view-model.test.mjs
python3 tests/configuration_test.py
python3 -m pip install playwright
python3 -m playwright install chromium
python3 tests/browser_test.py
```

The browser test can use an already installed Chromium with `MPD_TEST_CHROMIUM=/path/to/chromium`. It uses in-memory pages and mocked native IPC/Twitch SDK events. It removes CSP and module loading **only in its instrumentation**, so it does not validate production CSP, native IPC permissions, actual iframe loads, network behavior, or playback. Production files retain their CSP and normal loading.

An additional `python3 tests/hosted_wrapper_test.py` suite loads unchanged wrapper assets under the configured HTTPS origin using intercepted browser responses and mocked SDK/IPC. It retains CSP but does not contact the deployed host or establish TLS. Its execution here was blocked by the browser environment before assertions (`ERR_BLOCKED_BY_ADMINISTRATOR`); it is **not** included in the 51 passing checks.

See **[VERIFICATION.md](docs/VERIFICATION.md)** for exact results and **[MANUAL-TESTS.md](docs/MANUAL-TESTS.md)** for the remaining native acceptance matrix. The GitHub Actions workflow is included but has not been run or uploaded to a repository.

## Source layout

```text
crates/core/       Pure priority selection and reconciliation rules; Rust tests
crates/twitch/     Device OAuth, token maintenance, and Helix polling
src-tauri/src/    Native commands, single-owner controller, windows, SQLite
src-tauri/        Build configuration, icons, and explicit IPC permissions
ui/               Management frontend; no selection policy
player-wrapper/   Official player integration and isolated demo controls
scripts/          Launch helpers and HTTPS-origin configuration
tests/           Node, configuration, and mocked browser tests
docs/            Verification, security, manual tests, and mock-data previews
```

Screenshots in `docs/` are **browser-rendered previews using mock data**, not screenshots of a compiled desktop app or real Twitch playback.

## Next gate

Run a native build, resolve any compiler/runtime integration issues, commit the resulting dependency lockfile, and complete the three-stream playback matrix on the target operating systems. Only then decide whether Tauri's webviews meet the background-playback requirement and whether to proceed to a tabbed workspace/Leptos UI.

No Twitch approval, viewer-count behavior, resource-efficiency claim, or cross-platform playback guarantee is asserted by this POC. See [SECURITY.md](docs/SECURITY.md).
