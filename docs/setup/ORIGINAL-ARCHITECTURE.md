# MPD Tabber: Architecture and Development Plan

**Status:** Architecture proposal. The browser engine remains provisional until the playback feasibility prototype passes.

**Target platforms:** Windows, Linux, and macOS.

**Primary language:** Rust.

## Executive Recommendation

Build a standalone Rust desktop application with a browser-independent scheduling core and a replaceable playback adapter. The first implementation candidate is **Tauri 2, a Rust-based Leptos interface, and Twitch’s official embedded player**.

Make the final browser-engine decision **after a small, packaged playback prototype on Windows, macOS, and Linux**. Reliably playing Twitch—including volume control and inactive-tab behavior—is the main technical risk. The favorites list and priority algorithm are comparatively straightforward.

This design preserves the requested behavior rather than assuming a particular framework can deliver it.

## 1. Product Definition

> MPD Tabber maintains a configurable number of Twitch viewing sessions, automatically selecting live channels from a user-managed priority list.

A **tab** means an MPD Tabber–managed viewing session. In the standalone implementation, these are tabs inside the application, not tabs in an existing Chrome or Firefox window.

### 1.1 MVP Behavior

| Area | Behavior |
|---|---|
| Favorites | Add streamers by channel name or Twitch channel URL. Remove or temporarily disable them. |
| Priority | Reorder at any time using drag-and-drop or keyboard controls. Changes take effect immediately. |
| Tab limit | User-defined positive integer, default **3**. No hardcoded assumption that Twitch counts only two or three. |
| Selection | Maintain the highest-priority live favorites, up to the configured limit. |
| Preemption | When a higher-priority favorite goes live, replace the lowest-priority selected favorite. |
| Volume | A global **0–100% player-volume setting**, applied to every managed player, with a separate mute control. |
| Persistence | Save favorites, priority order, tab limit, volume, and other preferences locally. |
| Start | Begin monitoring and opening the selected streams. |
| Pause automation | Keep existing players, but stop automatically changing the selection. |
| Stop | Close managed players and stop monitoring. |
| Exit | Shut down all application-owned playback. Never affect unrelated browser tabs. |

**Proposed defaults beyond the stated requirements:** 25% volume, a 30-second live-status polling interval, and no automatic playback until the user presses Start. These are product choices, not Twitch viewer-count recommendations.

### 1.2 Selection Example

With priority order **A → B → C → D → E**, and a limit of **3**:

| Event | Selected channels |
|---|---|
| B, C, D, and E are live | B, C, D |
| A goes live | A, B, C |
| The user moves E to the top | E, A, B |
| A goes offline | E, B, C |
| The user reduces the limit to 2 | E, B |

Channels that remain selected must keep their existing player sessions. A reorder must not reload every stream.

### 1.3 Product Boundary: Playback Is Not Viewer-Count Confirmation

MPD Tabber should report **selected channels and observed playback**, not claim that it has produced a particular number of counted viewers. The Twitch interfaces referenced in this proposal expose player state and aggregate stream information; they do not give this application a per-session “your view has been credited” confirmation.

Assumptions about tab count and volume can remain user preferences without becoming claims the application makes.

Reference: [Twitch Embedded Video and Clips](https://dev.twitch.tv/docs/embed/video-and-clips/).

## 2. Browser Framework Recommendation

There are two separate decisions:

1. Standalone application versus installed-browser integration.
2. Which browser engine a standalone application uses.

A standalone application does not necessarily need to bundle Chromium.

### 2.1 Options

| Approach | Advantages for MPD Tabber | Main disadvantages | Recommendation |
|---|---|---|---|
| **Tauri 2 with system webviews** | Rust application host; standalone experience; avoids managing a bundled Chromium distribution. | Different browser engines across operating systems; Twitch compatibility must be tested. | **First prototype candidate.** |
| **CEF through Rust bindings** | Chromium-based browser integration across the requested platforms; substantial control over browser lifecycle. | More packaging, subprocess, security-update, and codec responsibility. | **Contingency, not the starting point.** |
| **Browser extension + Rust native host** | Real browser tabs; uses the browser’s existing Twitch session. | Requires an extension and browser-specific distribution; precise volume control still needs a player integration. | **Compatibility fallback.** |
| **Electron + Rust core** | Bundled Chromium approach. | Adds a JavaScript/Node application shell rather than a Rust-first desktop stack. | **Not the default for this project.** |

Tauri uses **WebView2 on Windows**, **WKWebView on macOS**, and **WebKitGTK on Linux**. A successful Windows playback test does not establish macOS or Linux compatibility.

Reference: [Tauri Webview Versions](https://v2.tauri.app/reference/webview-versions/).

CEF has Rust bindings listing Windows, Linux, and macOS support, but platform support alone does not establish Twitch compatibility. Multimedia codec support, the actual binaries, and their redistribution requirements need review before committing to that route.

Reference: [CEF Rust Bindings](https://github.com/tauri-apps/cef-rs).

Browser extensions can manage tabs and communicate with a Rust native process through native messaging. Tab mute is not the same as a general percentage-volume control, so this alternative does not eliminate all player-integration work.

Reference: [Chrome Extensions Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).

Electron’s main-process model is based on Node.js, which is why it is not the first choice for a Rust-first desktop application.

Reference: [Electron Process Model](https://www.electronjs.org/docs/latest/tutorial/process-model).

### 2.2 Rust Implementation Choice

Use **Leptos for the management interface**, keeping application UI code in Rust compiled to WebAssembly. Tauri documents this integration. A small JavaScript interoperability layer will connect to Twitch’s JavaScript player SDK.

Reference: [Tauri and Leptos](https://v2.tauri.app/start/frontend/leptos/).

The implementation consists of:

- Rust for business logic, networking, persistence, desktop integration, and management UI.
- HTML and CSS for presentation.
- A small, isolated bridge to the Twitch player.

This does not mean writing a browser or video decoder in Rust.

## 3. Playback Feasibility Prototype

The prototype must answer three questions before the runtime is selected.

### 3.1 Can the Packaged Application Embed Twitch Correctly?

Twitch’s embed requirements include SSL-backed embedding domains, correct `parent` configuration, approved player elements, and unobscured players. A desktop application’s custom URL scheme must not simply be assumed equivalent to a normal HTTPS website.

Reference: [Twitch Embedding Requirements](https://dev.twitch.tv/docs/embed/).

Evaluate two approaches:

1. **A local or bundled player wrapper**, but only if its production origin can satisfy the documented requirements.
2. **An application-owned HTTPS player wrapper**, loaded as the top-level document of a dedicated player webview. This would be a small static page containing the official player integration—not a backend service holding users’ accounts or tokens.

The second option introduces a real hosting dependency. That must be explicit: “standalone desktop application” would not mean “all application assets are local.”

Do not solve origin problems by spoofing the parent domain, disabling certificate validation, or installing a custom trusted certificate.

### 3.2 Does Inactive-Tab Playback Actually Work?

**This is a release gate, not an implementation detail.**

Twitch documents minimum player dimensions of **400 × 300** and visibility-related autoplay conditions. Hidden and inactive-player behavior must therefore be verified rather than assumed.

Reference: [Twitch Embedded Video and Clips](https://dev.twitch.tv/docs/embed/video-and-clips/).

| Scenario | Required observation |
|---|---|
| Three selected streams | Each player’s actual playback state is visible to the application. |
| Switching application tabs | Retained players are not unnecessarily recreated. |
| Application minimized | Determine whether playback continues, pauses, or is suspended. |
| New stream opened with nonzero volume | Handle autoplay restrictions honestly. |
| Lock/unlock and sleep/wake | Recover without duplicate sessions or retry storms. |
| Ads, consent screens, and login prompts | Do not misclassify them as crashes and repeatedly reload. |

A grid of visible players could be an optional layout, but it must not silently replace the background-tab requirement.

If the intended behavior is unsupported in embedded webviews, change the playback adapter or explicitly revise the experience. Do not fabricate visibility or playback activity.

### 3.3 Can the Application Provide a Secure, Usable Tabbed Interface?

Tauri’s child-webview construction API was identified in the proposal as being behind its `unstable` feature. Confirm its status when implementing; do not assume that composing several independent native webviews into one polished window is already a solved dependency.

Reference: [Tauri WebviewBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html).

Start the prototype with straightforward native playback windows. Then decide between a single-window layout and a manager window plus a tabbed player-workspace window.

**Framework acceptance requires packaged playback, production-origin behavior, and lifecycle tests—not just a page loading in a development build.**

## 4. Application Architecture

Build **one local application**, not a collection of services.

### 4.1 Components

| Component | Responsibility |
|---|---|
| **Domain core** | Favorites, ranking, settings validation, desired selection, and reconciliation rules. |
| **Twitch adapter** | Authentication, channel identity resolution, and live-status queries. |
| **Controller** | Own application state and coordinate asynchronous operations. |
| **Player adapter** | Create, control, observe, and close application-owned player sessions. |
| **Persistence** | Save settings and favorites; import and export preferences. |
| **Management UI** | Display state and submit user commands. Contains no scheduling policy. |

The critical separation is:

> The core decides which channels should be open. The player adapter decides how to open and control them.

Changing from Tauri playback to CEF or a browser extension must not require rewriting the priority algorithm.

### 4.2 Suggested Repository Structure

```text
mpd-tabber/
  Cargo.toml
  crates/
    core/src/
      model.rs
      selection.rs
      reconcile.rs
      protocol.rs

    twitch/src/
      auth.rs
      api.rs
      monitor.rs

    desktop/src/
      controller.rs
      storage.rs
      credentials.rs
      lifecycle.rs
      player/

    ui/src/
      favorites/
      settings/
      workspace/
      status/

  player-wrapper/
  tests/
  docs/
    requirements.md
    architecture.md
    decisions/
    tasks/
```

Four workspace crates are enough initially. Do not create separate crates for every small feature.

### 4.3 Proposed Supporting Libraries

| Purpose | Proposed library or approach |
|---|---|
| Asynchronous work | Tokio |
| HTTP | reqwest |
| Serialization | serde |
| Local database | rusqlite |
| Credentials | Operating-system credential-store adapter |
| Bounded diagnostics | tracing |

The pure core must have **no dependency on Tauri, HTTP, SQLite, browser handles, or real timers**. Its behavior should be testable without opening Twitch.

## 5. Twitch Authentication and Live Monitoring

### 5.1 Authentication: Public-Client Device Flow

Use Twitch’s **public-client device-code grant**.

This avoids shipping a client secret inside a downloadable desktop application. Twitch supports public clients obtaining and refreshing tokens without a client secret through this flow.

Reference: [Twitch OAuth Token Flows](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/).

User experience:

1. Click **Connect Twitch**.
2. Authorize MPD Tabber on Twitch in the normal browser.
3. Return to the application.

The native Rust process handles tokens. The player wrapper must not receive them.

Request the smallest permission set that works for the MVP. `Get Streams` accepts an app or user access token without requiring an additional endpoint-specific scope. Follow-import, chat, and broadcaster permissions should not be requested merely to monitor a manually entered favorites list.

Reference: [Twitch API Reference](https://dev.twitch.tv/docs/api/reference/).

Validate tokens on startup and hourly, as Twitch requires. Serialize refresh operations and safely save replacement credentials; the device-flow documentation describes single-use refresh tokens for public clients.

References:

- [Twitch Token Validation](https://dev.twitch.tv/docs/authentication/validate-tokens/)
- [Twitch OAuth Token Flows](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/)

**Keep API authorization separate from website sign-in.** Do not assume that authorizing the application also signs the embedded player into Twitch. Test anonymous playback and account-dependent playback separately.

### 5.2 Live Status: Polling First

For the MVP, use **batched polling every 30 seconds**, rather than EventSub.

Twitch’s `Get Streams` endpoint accepts up to 100 user IDs per request. Set `first=100`; otherwise, the default result-page size is 20. Complete any pagination before treating missing channels as offline.

Reference: [Twitch API Reference](https://dev.twitch.tv/docs/api/reference/).

Approximate request volume under normal conditions:

| Favorites | Requests per polling cycle | Requests per minute |
|---:|---:|---:|
| 25 | 1 | 2 |
| 100 | 1 | 2 |
| 500 | 5 | 10 |

These are arithmetic estimates excluding retries and identity lookups, not promises about API quotas. The implementation must honor actual rate-limit responses.

### 5.3 Why Not EventSub Immediately?

EventSub WebSockets have subscription-count limits and a separate total-cost limit. The original architecture proposal identified a **total-cost limit of 10**, with subscriptions for arbitrary broadcasters who have not authorized the application potentially consuming that cost. Verify the applicable limits during implementation.

The “300 subscriptions per connection” number must not be interpreted to mean the application can freely monitor hundreds of arbitrary favorites through WebSockets.

Reference: [Managing EventSub Subscriptions](https://dev.twitch.tv/docs/eventsub/manage-subscriptions/).

EventSub can be added later to improve latency where practical. Polling should remain the reconciliation mechanism, especially because events missed during a lost connection are not replayed.

Reference: [Handling EventSub WebSocket Events](https://dev.twitch.tv/docs/eventsub/handling-websocket-events/).

## 6. Selection, Reconciliation, and Failure Handling

This area deserves substantial correctness testing.

### 6.1 Selection Is a Pure Calculation

For a fresh, fully observed state:

```text
desired channels =
    favorites in priority order
    → enabled
    → confirmed live
    → not explicitly skipped
    → first K
```

This calculation must not open windows or make network requests.

### 6.2 Reconciliation Changes Only What Is Necessary

Compare the desired selection with current assignments:

```text
Keep  = current ∩ desired
Close = current − desired
Open  = desired − current
```

Existing players in `Keep` remain intact. A volume change does not recreate sessions. Reordering selected channels changes their presentation order, not their identity.

### 6.3 Enforce Capacity During Transitions

Opening operations reserve capacity. Closing players continue consuming capacity until their closure is confirmed.

**Close an outgoing session before opening its replacement**, avoiding an application-created temporary fourth stream when the limit is three.

Reducing the limit from five to two requires a brief teardown period. The desired count becomes two immediately, while the actual count converges as excess sessions close. The UI must report that transition rather than pretend closure is instantaneous.

### 6.4 Prevent Stale Asynchronous Work From Winning

Every opening operation must carry a session identity and revision.

For example, suppose the application starts opening D, and the user immediately moves A above D. A late “D opened successfully” callback must not restore an obsolete selection.

A single controller owns these state transitions. Browser callbacks and HTTP responses report events to it; they must not independently modify shared application state.

### 6.5 Distinguish Offline, Unknown, and Failed Playback

| Situation | Proposed behavior |
|---|---|
| Fresh observation says a favorite is live | Recalculate selection. |
| Favorite missing from one successful poll | Mark pending offline; do not immediately churn. |
| Missing from two successful polls | Confirm offline and replace if needed. |
| API timeout or failed batch | Mark affected status stale, not offline. |
| Network temporarily unavailable | Retain existing sessions; avoid new opens based on stale data. |
| Autoplay blocked | Keep the assigned tab and show **Click to start playback**. |
| User pauses a player | Respect the pause. |
| Renderer crashes | Attempt bounded recovery. |
| Recovery repeatedly fails | Show an actionable error; stop automatic reloads. |
| User explicitly closes a managed tab | Treat as **Skip this broadcast**, with Undo. |

The manual-close behavior prevents the application from immediately reopening a tab the user deliberately closed.

### 6.6 Separate Assignment From Playback

A channel can be:

- Live but waiting for a slot.
- Assigned but still loading.
- Assigned but awaiting user interaction.
- Assigned and playing.

Twitch’s player API provides readiness, playback, and playback-blocked events, plus independent volume and mute controls. Use those rather than treating “the browser window exists” as successful playback.

Reference: [Twitch Embedded Video and Clips](https://dev.twitch.tv/docs/embed/video-and-clips/).

Example status line:

> Monitoring · 3 assigned · 2 playing · 1 needs a click · Last successful check 12 seconds ago

## 7. Storage and Security

### 7.1 Local Data

Use SQLite for ordered favorites and preferences. Persist the Twitch user ID as identity, alongside the last-known login and display name.

Reordering must be transactional. Import and export must use a versioned JSON format containing preferences only.

Do not restore cached live status as current truth after a restart.

### 7.2 Credentials

Store tokens in the native credential store, not the preferences database or webview storage.

On a Linux installation without an available secret store, provide an explicit memory-only authentication mode rather than silently falling back to plaintext files.

Never include tokens, cookies, or authorization headers in diagnostic exports.

### 7.3 Browser Isolation

The management interface and player content must have different privileges.

**Trusted management UI:** May submit validated favorites and settings commands.

**Player content:** May control its assigned player through a constrained wrapper and report limited telemetry. It must not read credentials, access arbitrary files, run shell commands, or create arbitrary windows.

Tauri’s capability and custom-command exposure behavior needs explicit review. Its documentation includes platform-specific considerations around embedded iframe callers; do not grant broad remote-content permissions.

Reference: [Tauri Capabilities](https://v2.tauri.app/security/capabilities/).

If the player needs an inbound reporting command, that command must be safe even when called by a hostile subframe: bounded, rate-limited, restricted to its own session, and unable to change application policy.

Keep normal TLS validation and browser sandbox protections enabled.

## 8. Development Plan

Develop through small, bounded tasks with explicit acceptance criteria. Do not ask an implementation agent to build the whole application in one pass.

### Phase A: Establish Feasibility

| Task | Scope | Exit criterion |
|---|---|---|
| **TAB-001: Playback prototype** | Minimal packaged application; production origin; three players; volume; inactive and minimized behavior. | Observed results on Windows, macOS, and Linux; runtime decision recorded. |
| **TAB-002: Authentication prototype** | Public-client device flow and refresh. | Usable API token without a distributed secret; minimal permissions verified. |
| **TAB-003: Security contract** | Management/player separation and message protocol. | Explicit command permissions and a documented trust boundary. |

**Do not proceed with a large UI implementation until these results are available.**

### Phase B: Implement the Independently Testable Core

| Task | Scope | Exit criterion |
|---|---|---|
| **TAB-004: Workspace and checks** | Crates, formatting, linting, tests, and dependency checks. | Minimal builds work on target platforms. |
| **TAB-005: Domain models** | Favorites, settings, identities, presence, and playback states. | Input validation and serialization tests. |
| **TAB-006: Priority selection** | Pure selection function. | Deterministic, correct, duplicate-free top-K results. |
| **TAB-007: Persistence** | SQLite, transactional reorder, and import/export. | Preferences survive restart; invalid imports do not partially apply. |

### Phase C: Monitoring and Orchestration

| Task | Scope | Exit criterion |
|---|---|---|
| **TAB-008: Authentication adapter** | Credential storage, validation, and serialized refresh. | Expiration, revocation, cancellation, and storage failures handled. |
| **TAB-009: Twitch API adapter** | Identity resolution and complete batched status queries. | Correct behavior across 100-ID boundaries and failed pages. |
| **TAB-010: Monitor** | Polling, freshness, offline debounce, and backoff. | Failed requests never become false offline observations. |
| **TAB-011: Controller** | Reconciliation using a fake player adapter. | Correct swaps, capacity reservations, and stale-callback handling. |

### Phase D: Actual Playback and User Interface

| Task | Scope | Exit criterion |
|---|---|---|
| **TAB-012: Player wrapper** | Official SDK integration and constrained messages. | Channel, volume, mute, readiness, and playback states work. |
| **TAB-013: Player adapter** | Native lifecycle and real session ownership. | Priority swaps work without orphaned or duplicate players. |
| **TAB-014: Manager UI** | Favorites, ordering, settings, and controls. | Complete management workflow works with keyboard and mouse. |
| **TAB-015: Player workspace** | Tabs, state badges, focus, retry, and skip/undo. | Tab selection does not reload retained players; failures are understandable. |

### Phase E: Hardening and Release

| Task | Scope | Exit criterion |
|---|---|---|
| **TAB-016: Lifecycle and reliability** | Sleep/wake, network changes, crashes, stop, and quit. | No retry storms or residual application-owned playback after normal shutdown. |
| **TAB-017: Security and diagnostics** | Hostile message tests, permissions, and log redaction. | Player content cannot reach privileged operations. |
| **TAB-018: Packaging and documentation** | Native installers, signing, clean installs, and compatibility matrix. | Every claimed release target is tested and limitations are documented. |

### 8.1 Task Execution Rules

Each implementation task must specify:

- Allowed files or modules.
- Prerequisites and dependencies.
- Required tests.
- Explicit non-goals.
- Acceptance criteria and a demonstration.

Concurrent agents must use separate branches or worktrees. Changes outside the assigned area must become explicit dependencies rather than opportunistic refactors.

## 9. Acceptance Tests

### 9.1 Core Correctness

Generate sequences of reorders, live/offline changes, cap changes, failed opens, and delayed callbacks.

| Property | Expected result |
|---|---|
| Selection correctness | Highest-priority eligible channels are selected. |
| Uniqueness | No duplicate assignment for a channel. |
| Session stability | Retained channels are not unnecessarily reopened. |
| Capacity safety | New opens respect reserved and closing capacity. |
| Event ordering | Old callbacks cannot resurrect obsolete assignments. |
| Shutdown | Stop prevents pending work from reopening streams. |

### 9.2 Real Playback

Test **K = 1, 3, and 6**, volume **0%, 25%, and 100%**, and both guest and any supported signed-in experience.

Run shorter smoke tests and longer playback soaks on representative Windows, macOS, and Linux systems. Measure the whole process tree:

- CPU and GPU activity.
- Memory usage.
- Bandwidth consumption.
- Resource cleanup.

Do not publish memory-use or efficiency claims before those measurements.

### 9.3 Product and Platform Review

The application must not fabricate activity, evade restrictions, automate chat, or claim viewer-count credit. Before public distribution, review the intended unattended-viewing behavior against Twitch’s current rules.

**This document is an architecture proposal, not a finding that Twitch has approved the application or its unattended-use model.**

External platform requirements and dependency capabilities must be revalidated during implementation and before release.

## 10. Deferred Features

Keep the first release focused. Defer:

- Follow-list import.
- Pinned channels.
- Fairness rotation.
- Per-channel volume overrides.
- Focus-only audio.
- Cloud synchronization.
- Automatic startup.
- EventSub acceleration.

These features are easier to add after the application can reliably answer:

> Which channels should be open, which are actually playing, and can the user stop all of them cleanly?

## 11. First Milestone and Final Direction

**Build standalone, keep the application code Rust-first, and start with Tauri 2—but keep the browser choice provisional until real Twitch playback passes on all three operating systems.**

The first milestone must demonstrate:

1. Three managed streams.
2. Priority replacement.
3. Working volume control.
4. Honest inactive and minimized playback behavior.
5. Complete cleanup.

Once that is proven, the remaining work becomes a well-bounded desktop application rather than a speculative browser-integration project.
