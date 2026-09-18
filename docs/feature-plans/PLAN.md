# MPD Viewer — Feature and Subagent Implementation Plan

**Prepared:** September 17, 2026  
**Status:** Proposed implementation plan; no application changes or live-account tests were performed for this plan.  
**Baseline reviewed:** `MPD-Tabber-POC-Hosted-Parent.zip` (SHA-256 `542075ad3983cc9b1ae819d3220b2f1838859dbd877bbb534fb37b601200fb07`).  
**Suggested milestone:** 0.2.0, subject to the version already in the working checkout.

## 1. Outcome and boundaries

Deliver these five changes to the existing Rust/Tauri application:

| Request | Intended outcome |
|---|---|
| Rename | The application presents itself as **MPD Viewer**. |
| New tagline | The manager displays exactly **View fav channels in priority**. |
| Chat | Every active viewer has its own channel's official Twitch chat, enabled by default. |
| Turbo/ad investigation | Distinguish API authorization from viewer website sign-in, support the legitimate embedded login flow where the runtime allows it, and establish why ads appear using real evidence. |
| Grid or standalone | Users can choose one genuine grid window containing their viewers or separate native viewer windows. |

Keep priority selection, the user-defined session limit (default 3), audio settings, pause/stop behavior, favorites, public Client ID, and HTTPS parent intact. Do not turn this into a browser-engine rewrite or a Leptos migration. Chat bots, cookie import, ad blocking, automated engagement, and claims about counted viewers are out of scope.

The source archive is the review baseline, not proof of the user's current checkout or deployed page. The user reports actual login and ads, so the first task must inspect local changes and runtime evidence rather than repeat the older handoff's unverified status as a present-day fact. The deployed site could not be retrieved in this planning session; its actual contents remain unverified here.

## 2. Architectural decisions

### 2.1 Preserve identity while changing branding

Change visible branding now. Preserve `com.modpackdad.mpdtabber.poc`, the established preference path, and any existing browser-profile identity during this milestone. Keep the Rust package name `mpd-tabber` initially so existing commands and automation continue working. Internal names are compatibility identifiers, not customer-facing branding.

A separate, explicit migration may rename internal identifiers later. Do not perform a global replacement of `tabber` across paths, bundle IDs, schemas, package names, databases, and profile stores.

### 2.2 Add chat without replacing the working video adapter

Retain `Twitch.Player` and add an official chat iframe beside it. This preserves the existing volume and playback-event integration and makes chat collapse independent of video construction. Twitch documents the chat URL and required parent parameter [S1].

Evaluate the combined `Twitch.Embed` experience only if the authentication spike demonstrates a concrete reason to prefer it; Twitch also documents this official alternative and its login popups [S2]. Do not have separate agents introduce both approaches.

### 2.3 Treat Turbo as an authentication investigation

An authorized Helix API account, the user's normal browser session, a chat login, and an embedded video session are different observations. Do not derive one from another.

In the reviewed source, `Action::OpenAuth` opens the device-activation page in the system browser. `Host::open` denies every `window.open` request, and the wrapper only constructs a video player. These are concrete gaps to investigate, not conclusive proof of the cause of the observed ads. See `SOURCE-REVIEW.md`.

The target is working website authentication and observed account behavior—not suppression of advertising. The product must not invent a `turbo_active` state from API authorization or lack of an ad during a short test.

### 2.4 Separate a viewer session from the window containing it

Introduce a presentation boundary between the controller and native webviews. A selected channel is a logical session; a webview is its playback surface; a window is a container. In grid mode several surfaces share a container. In standalone mode each has its own container.

Prefer moving the existing surface between containers without a new video player. Tauri documents a desktop `reparent` API, while child-webview construction is currently behind its `unstable` feature [S5, S6]. These APIs justify a small feasibility experiment, not an unconditional portability claim.

If retained-surface moves fail but native grid construction works, an explicitly reported **close-and-recreate transition** is acceptable: preserve logical selection, await old playback destruction, then create the new surface. A brief reload must be visible to the user. If child grids themselves are unreliable, record a separate architecture decision for an unprivileged hosted grid page or another adapter. Do not silently call tiled standalone windows a grid, broaden manager permissions, or ship an unsupported mode as complete.

## 3. Feature A — Rename and tagline

### Scope

Update manager title and header, standalone viewer titles, future grid title, packaged display name, user-facing error strings, current setup documentation, and newly generated artifact names. Set the packaged `productName` to `MPD Viewer`; keep the POC/version qualifier in a separate label or About text.

Set the tagline to exactly:

> View fav channels in priority

Keep the existing public Client ID `ha94kk20cfu1tp74pgg8isgi88cpo7` and parent origin `https://parent.mpdviewer.com/` unchanged. Updating the Twitch developer application's display name is an external administrative action, not something the source rename accomplishes; include it on the owner's release checklist.

### Files and ownership

`ui/index.html`; visible strings in `ui/app.js`; `src-tauri/tauri.conf.json`; title/startup strings in `src-tauri/src/player.rs` and `main.rs`; wrapper HTML title; current README; UI fixtures and branding tests. Changes in shared Rust files must be literal-only and merged before native feature work.

Historical verification documents may retain MPD Tabber to accurately identify old artifacts. Maintain an allowlist of intentional legacy identifiers rather than asserting that no occurrence of `tabber` remains anywhere.

### Acceptance

A fresh install and upgrade both show the new name and exact tagline. Existing favorites, order, volume, mute, limit, custom Client ID, and saved browser session are preserved. Existing development commands still work. Installer upgrade behavior is verified on each packaged target; changing a visible product name must not be assumed harmless to all installer formats.

**Owner:** `mpdv_mechanical` · Luna / low. **Independent review:** Terra / medium.

## 4. Feature B — Chat in each viewer

### User experience

Show the selected channel's chat by default. On a sufficiently wide surface, video and chat sit side by side. An initial chat width around 340 logical pixels is a product choice, not a Twitch requirement. The video area—not merely the outer window—must remain at least 400 × 300 pixels, following Twitch's player requirements [S3].

At narrow widths, use a deliberate stacked layout with adequate height. Permit the user to collapse and restore chat; do not reload or recreate the video to do so. Never silently remove chat to fit more grid cells. Grid geometry must account for the chat panel and application chrome.

Use Twitch's own chat interface for messages, emotes, login, moderation restrictions, and account state. No chat OAuth scopes, IRC connection, or bespoke chat renderer are needed for this embed-based implementation.

### Implementation

Build the chat URL from the already validated channel and the real wrapper hostname using a URL builder. The expected form is `https://www.twitch.tv/embed/<channel>/chat?parent=<actual-hostname>`. No native token or cookie belongs in that URL.

Add `https://www.twitch.tv` to the wrapper's narrowly scoped frame policy. Review response-header CSP as well as HTML meta CSP; changing the latter cannot loosen a stricter server policy. Keep the privileged manager's `frame-src 'none'` isolation. Review native frame-navigation rules, which currently admit the wrapper and video-player origin but not chat.

If a sandbox attribute is used, follow Twitch's documented chat capabilities rather than inventing a restrictive subset that silently breaks storage or sign-in [S1]. Do not broadly enable arbitrary popup URLs: native popup policy is owned by the authentication task.

A failed chat load must not stop video. Do not infer a successfully authenticated chat from an iframe `load` event. Cross-origin chat contents are not available to the wrapper for arbitrary inspection.

### Acceptance

Channel A's video always has channel A's chat; priority replacement creates the replacement channel's chat. Collapse/restore does not change the video instance or call `play`. Anonymous chat is displayed where Twitch permits it; actual posting is verified by a user in an authorized test channel after native sign-in. Ads, consent prompts, and failed chat resources do not create reload loops.

Demo mode contains an explicitly simulated chat panel and makes no Twitch video/chat requests.

**Owner:** `mpdv_web` · Terra / medium. **Review:** Sol / high for iframe policy and login integration.

## 5. Feature C — Viewer authentication and Turbo diagnostics

### 5.1 Reproduce before changing the browser engine

Record the real OS, app build, resolved Tauri/Wry version, webview runtime, wrapper build/origin, and exact sequence used to log in. Record whether the observed content is a Twitch-served ad or a promotion already present in the broadcast. Do not record passwords, cookie values, authorization headers, or unredacted authentication traces.

Compare a controlled set of observations using the same channel and account:

| Case | Purpose |
|---|---|
| Normal browser, normal Twitch page | Establish account behavior outside the application. |
| Normal browser, hosted wrapper | Separate embed behavior from native-runtime behavior. |
| Native viewer with only API authorization | Demonstrate that monitoring connection alone is not website sign-in. |
| Native viewer after Twitch website sign-in in its profile | Test the proposed browser-session fix. |
| Second viewer and app restart | Test sharing and persistence. |
| Grid after it exists | Detect layout-dependent regressions. |

A short ad-free observation is not proof of entitlement handling. Use a repeatable observation protocol and record interruptions and timing. An unavailable login prompt or blocked browser is a legitimate blocked test, not a pass.

### 5.2 One persistent playback profile

All viewer surfaces, their chat frames, and authorized login popups should use one stable application-owned browser profile. Do not assume a new custom profile is needed before checking the current runtime's default: changing the path can appear to log the user out.

Windows WebView2 stores browser state in its user-data/profile storage [S7]. Configure and verify the same profile/environment for related views. macOS and Linux need equivalent native-runtime handling; Tauri's custom macOS data-store identifier has an OS-version constraint [S5]. Do not silently increase the supported minimum macOS version to obtain it.

Sharing a profile is necessary design work but not a universal cure for third-party-cookie restrictions. Test storage-access prompts and partitioning behavior under normal platform security settings. Do not disable cross-origin security, falsify browser identity, or automatically grant blanket third-party storage access.

### 5.3 Constrained sign-in popups

Replace deny-all popups with a narrow, tested policy for the official Twitch login flow. Preserve opener relationships and supplied native window features/configuration. Tauri documents different related-view/environment requirements on each platform [S8]. Merely opening the login URL in another unrelated webview is not sufficient evidence that the flow works.

Login windows receive **no manager or telemetry permissions**, cannot download arbitrary files, and do not count as viewing sessions. Validate parsed HTTPS origins and actual redirect destinations; do not use substring matches or `*.twitch.tv` as a substitute for an audited navigation policy. Any additional identity-provider navigation must be supported by the observed official flow and reviewed separately.

User-entered passwords and MFA stay in Twitch-controlled pages. No form scraping, DOM credential injection, cookie export/import, or token-to-cookie conversion. Auth popups are tracked so application exit and explicit session reset cannot orphan them.

### 5.4 Honest account UI

Replace the ambiguous single connection indicator with two concepts:

- **Live-status API:** the account authorized for monitoring.
- **Viewer website sign-in:** open the official sign-in experience, with state shown only to the extent it can actually be established.

Allow `Unknown / verify in Twitch` instead of inventing a native authenticated flag. A user-confirmed sign-in may be labeled as user-confirmed, not automatically verified. An account shown in chat is evidence about chat, not sufficient proof that the video recognizes Turbo.

Provide a deliberate reset of the application-owned viewer session if runtime support permits it. Explain that reset closes viewers and signs this app's browser profile out; require confirmation. Preserve favorite settings and keep API Disconnect separate. Do not delete a live browser-profile directory as a shortcut.

### Acceptance and fallback

Official login completes in a supported native runtime; another viewer shares the intended session; restart behavior is documented; all popups remain unprivileged. Real-account comparison establishes whether the original ad observation is resolved or remains reproducible. Report an unsupported embed/runtime combination honestly and offer a user-initiated open-in-browser compatibility path where useful. Do not promise full automatic window/volume management in an unmanaged external browser.

**Diagnosis:** Astra / high. **Native implementation:** Sol / high. **UI:** Terra / medium. **Security review:** a separate Astra / high agent.

## 6. Feature D — Grid and standalone presentation

### 6.1 Product contract

Keep standalone as the upgrade default. Add a manager setting **Viewer layout: Standalone | Grid**. Persist the preference but never automatically start playback after relaunch.

Grid means one native application window containing all selected viewer surfaces. Maintain priority order left-to-right, top-to-bottom. Layout changes do not change favorite rankings. Reordering should move retained surfaces without reconstructing them.

Global mode switching is required. Arbitrary docking trees, dragging between windows, and simultaneous mixed docked/standalone placement are optional later work—not blockers for this milestone.

### 6.2 Model and presentation boundary

Define the following contracts before coding:

| Concept | Meaning |
|---|---|
| `ViewerSessionId` | Stable logical assignment to a selected channel. |
| `SurfaceId` / instance epoch | Identifies the concrete webview/renderer incarnation. |
| `ContainerId` | Standalone window or shared grid window. |
| `Placement` | Standalone container or grid cell. |
| `LayoutRevision` | Rejects callbacks from superseded layout transactions. |
| `BrowserProfileId` | Shared browser session identity, independent of container. |
| `CloseReason` | User skip, stop, preemption, replacement, container move, or failure. |

The exact Rust names are proposals. `CONTRACTS.md` defines behavior that the contract agent must translate into the smallest workable implementation.

Extract focused presentation code from `Host::open`, `audio`, and controller calls to `get_webview_window`. Keep a concrete adapter and fake test adapter; do not build a general-purpose docking framework.

### 6.3 Native grid feasibility and safety

Prototype two viewer webviews in a dedicated grid window, with no manager privileges. Test resize, focus, chat input, native popups, minimize/restore, and reparenting in both directions. Record which OS/runtime combinations work. Pin tested dependency versions and commit the lockfile; the reviewed archive did not include one.

The current telemetry endpoint identifies the caller by **window** label. That is no longer a valid session boundary when several viewers share a window. Bind reports to the native **webview/surface** identity and active instance epoch. Parent-supplied session numbers alone are not trusted identity.

Tauri capabilities can merge permissions, and Linux cannot always distinguish a third-party iframe from its containing webview [S9]. A hostile frame may be able to submit advisory telemetry for its own surface; it must never manage favorites, read API credentials, alter layout policy, or report another surface as playing.

Keep the local manager separate from the native grid window. Do not place Twitch embeds in the existing privileged manager DOM. If local grid chrome is later added, permission it by its own webview, not by a grant covering all grid children.

### 6.4 Geometry

Use a pure layout calculation from available logical size, viewer count, chat preference, and optional column count. Account for display scale and screen work areas. Test 1, 2, 3, 4, and 6 viewers plus larger-count overflow cases.

Do not shrink video below 400 × 300 to satisfy an arbitrary capacity. When all cells cannot fit, use supported scrolling/clipping with truthful visibility, or explain the space constraint and let the user resize or choose standalone. The feasibility decision must establish how scrolling native child views works; native views do not automatically behave like ordinary CSS-grid children.

No hiding or rendering off-screen to imply visible playback. No automatic cap reduction just because the selected display is small. Respect explicit chat collapse and keep application controls outside Twitch content.

### 6.5 Transition state machine

A layout switch is a controller-owned transaction. Freeze its session snapshot, create the target container, transfer or explicitly recreate surfaces, apply geometry/audio, and retire only empty source containers. Commit the layout preference after the target is usable, with rollback or explicit partial-failure state otherwise.

Stop, exit, preemption, and a cap reduction supersede in-flight moves. A stale callback must never resurrect a stopped or replaced viewer. Old surfaces continue reserving capacity until destruction is observed. Reparenting must not be interpreted as user Skip.

For a recreate transition, close and confirm destruction of old playback before starting the replacement. Keep logical channel identity while assigning a new surface epoch. Do not replay `play` merely because the layout changed; honor a user-paused player whenever its state can be retained or safely restored.

### 6.6 Window semantics

Closing an individual standalone viewer preserves the current **Skip this broadcast** meaning. Closing the grid is a deliberate **Stop viewers and monitoring** action for this global-mode milestone; the manager stays available and Start is explicit. Closing an empty source window during a move is neither Stop nor Skip. Closing the manager exits the application and cleans up viewer and login windows.

On restart restore layout preferences and valid geometry, not stale sessions or live statuses. Recover windows whose saved monitor is missing or whose bounds are off-screen.

**Feasibility and implementation:** Sol / high. **Contract and independent lifecycle/security review:** Astra / high. **Geometry tests and manager controls:** Terra / medium or high as specified in the packets.

## 7. Hosted-wrapper delivery

The native binary and hosted wrapper form a protocol pair. Introduce a bounded compatibility handshake containing a protocol version, wrapper build ID, and supported presentation features. This is diagnostic metadata, not authentication evidence.

The old Rust `Report` rejects unknown fields. Do not simply append v2 fields to the old payload and deploy it to all existing clients. Use an additive handshake and negotiated messages or preserve the exact v1 reporting path for old applications. Unknown versions must produce a useful compatibility error, not a silent reload loop.

Package wrapper assets together under an immutable version path on the **same** parent domain, such as `/releases/viewer-0.2/`, and keep the existing root/v1 assets available for old clients. The actual embedding hostname remains `parent.mpdviewer.com`; paths are not a different parent. Keep a rollback artifact and native/wrapper version matrix.

The source currently serves exactly three bundled wrapper assets. Any newly split JS/CSS file requires corresponding local-server, deployment-manifest, CSP, and test-fixture updates. Do not edit only the website while leaving Demo's bundled wrapper incompatible.

No deployment is authorized by this planning document. Produce the deploy package and checklist; publication requires the owner's explicit deployment instruction and verified destination.

## 8. Subagent routing and reasoning

Model names and effort controls were verified against current official documentation. The assignments below are engineering recommendations, not a measured benchmark on this repository. Availability depends on the user's Codex client and account [S10–S14].

| Agent | Model | Effort | Assigned work |
|---|---|---|---|
| `mpdv_architect` | `gpt-6-astra` | `high` | Contracts, auth diagnosis, architecture decisions, integration decisions. |
| `mpdv_native` | `gpt-5.6-sol` | `high` | Profile/popup implementation, presenter refactor, grid and lifecycle logic. |
| `mpdv_web` | `gpt-5.6-terra` | `medium` | Chat, manager controls, scoped wrapper changes. |
| `mpdv_mechanical` | `gpt-5.6-luna` | `low` | Exact rename/tagline and bounded documentation changes. |
| `mpdv_qa` | `gpt-5.6-terra` | `high` | Failure-focused tests, browser/native evidence, compatibility checks. |
| `mpdv_security` | `gpt-6-astra` | `high` | Read-only independent permission, auth, and lifecycle review. |

Escalate an isolated unsolved design or race-condition question to Astra / `xhigh`; do not run every typo fix at maximum effort. Missing native tooling, unavailable model access, or absent account interaction requires a recorded blocker, not more reasoning tokens. If a requested model is unavailable, the coordinator must record an available substitute rather than pretend it ran.

Custom-agent examples are included under `codex-examples/`. They use documented model/effort and agent configuration fields [S15]. They are examples to merge, not permission to replace existing project policies or credentials. No agent has been launched by this planning handoff.

### Scheduling

Begin with MV-000. Then run branding, authentication diagnosis, and grid feasibility in separate worktrees; experimental spike code is not merged into the release branch. Freeze contracts in MV-001 after findings are available.

Implement authentication plumbing and the chat wrapper in parallel only while their writable file sets are disjoint. Integrate and verify standalone chat/sign-in before the presenter refactor. Then build grid presentation, transitions, and manager layout controls. Independent security review and real native acceptance gate the release.

Allow at most three worker threads and at most two production-code writers at once. Give each task an exclusive file lease. `player.rs`, `controller.rs`, `main.rs`, `model.rs`, permission files, and `ui/app.js` are shared hot spots; never assign them to simultaneous production writers. One coordinator merges; no worker rebases another worker's branch.

The task dependency manifest, not numeric task order, controls execution.

## 9. Validation and release gates

| Gate | Required evidence |
|---|---|
| Baseline | Actual current checkout and deployment identified; native build/test commands recorded; old failures separated from new work. |
| Branding | Exact strings and upgrade preservation tested. |
| Chat | Correct per-channel chat, real CSP, keyboard operation, isolated chat failure, no video reconstruction. |
| Account behavior | Native official sign-in, shared profile, restart, documented API/viewer distinction and ad observations. |
| Presentation | Real single-window grid, standalone restoration, retained or explicitly recreated surfaces. |
| Correctness | Capacity accounting, late callbacks, priority changes, cap decrease, stop during transition, no accidental skips. |
| Security | Caller identity and ACLs tested in real native runtimes; hostile frame cannot reach management or another session. |
| Distribution | Matching immutable hosted assets, rollback, upgrade tests, per-platform runtime matrix. |

Automated mocks do not establish native permission enforcement, real Twitch login, Turbo recognition, or background playback. The user must perform account/MFA steps; test agents must never request credentials in chat or collect raw login cookies.

Test Windows, macOS, and Linux as separate runtime targets. Prioritize the user's Windows setup for the first reproducible acceptance run, while preserving cross-platform source and recording support honestly. A platform with blocked tests is not a platform with a passed release gate.

No new feature code was built, deployed, or validated in this planning pass. The deliverable is the scoped execution package and source-grounded architecture above.

## 10. Packet contents and immediate next step

Read `SOURCE-REVIEW.md`, `CONTRACTS.md`, `AGENT-RULES.md`, and `ORCHESTRATOR-PROMPT.md`. `tasks/` contains 16 independently assignable work packets with dependencies, file ownership, tests, non-goals, and handoff requirements. `task-manifest.json` is the machine-readable dependency and model-routing index.

Start by giving the coordinator `ORCHESTRATOR-PROMPT.md` in the current working repository. Its first action is baseline discovery, not a full rewrite or an unreviewed deployment.

## Sources

Source identifiers link to the official references in [SOURCES.md](SOURCES.md). Repository findings are documented separately in [SOURCE-REVIEW.md](SOURCE-REVIEW.md).
