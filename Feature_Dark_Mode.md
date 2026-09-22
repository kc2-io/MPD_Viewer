# MPD Viewer system dark/light mode plan

**Status:** Ready for Claude Code execution; feature work not started  
**Prepared against:** clean `main` at `b167bdcd58a81e9125f9fb0a7574cf53eefbf2ed` on 2026-09-21  
**Baseline check:** `bash scripts/ci-check.sh` passed (6 Node suites, 15 configuration tests, 51 release tests)  
**Goal:** Follow the operating-system light/dark preference at startup and while the app is running. Apply it to the manager, bundled/Demo viewer chrome, native-injected hosted-viewer chrome, and the official embedded Twitch chat.

## Coordinator instructions

Act as the implementation coordinator. Read, in order:

1. `AGENTS.md`
2. `CODEX-START-HERE.md`
3. `docs/HOSTED-SOURCE-PLAN-ADDENDUM.md`
4. `docs/feature-plans/AGENT-RULES.md`
5. `docs/setup/CHAT-2026-09-18.md`
6. This plan

Verify the checkout, branch, dirty state, and current source before writing. The commit above is planning evidence, not permission to overwrite a newer checkout. Preserve user changes.

Use one coordinator and at most three workers, with at most two production-code writers at once. Give every worker a base commit, separate worktree, exclusive writable-file lease, bounded task packet, and required handoff. Only the coordinator integrates branches. Do not let an agent repair findings from its own independent review pass.

Use the most capable available Claude model/high reasoning for architecture, Twitch feasibility, native navigation security, and final review; a balanced coding model/medium reasoning for chat behavior and browser tests; and a fast/simple model for mechanical CSS-token work. Verify the actual Claude Code model and subagent types at execution time instead of copying possibly stale model identifiers.

## Current-state evidence

- `ui/style.css` is hard-coded dark (`color-scheme:dark`) and has dark literals outside its existing variables.
- `player-wrapper/style.css` and `player-wrapper/chat.css` are hard-coded dark.
- `src-tauri/hosted-player-adapter.js` creates a dark-only inline failure notice.
- `player-wrapper/chat.js` creates the separate official chat iframe with only the required `parent` query parameter. The same helper serves the bundled wrapper and is injected into the hosted top-level document by `src-tauri/src/player.rs`; it is the single theme handoff point for chat.
- `src-tauri/src/player.rs::allowed_chat_url` permits only the assigned HTTPS Twitch chat URL with exactly one `parent` query pair. Any theme query must receive an exact, reviewed allowlist change.
- `src-tauri/tauri.conf.json`, the player `WebviewWindowBuilder`, and the authentication window builder do not force a native theme. Keep the native default that follows the operating system unless runtime evidence proves it insufficient.
- `web/parent.mpdviewer.com/index.html` is an immutable provenance snapshot. Do not edit, replace, or deploy it. App-owned injected CSS may override visible host-shell colors at runtime.
- The current official standalone Twitch chat documentation lists `parent` but no theme option. The combined `Twitch.Embed` API documents `theme: "light" | "dark"`, but adopting it would replace the established separate `Twitch.Player` plus chat architecture. Relevant current references:
  - <https://dev.twitch.tv/docs/embed/chat/>
  - <https://dev.twitch.tv/docs/embed/everything/>

## Feature contract

1. The operating system is the only theme authority. There is no manual theme selector, saved theme field, database migration, controller action, or wrapper-protocol message.
2. App-owned content follows the system theme on cold start and on a live system-theme change without an app restart.
3. CSS `prefers-color-scheme` is the primary mechanism. JavaScript `matchMedia('(prefers-color-scheme: dark)')` is used only where the theme must cross into the Twitch chat URL.
4. Keep Rust as selection/controller authority. Theme detection must not affect channel selection, capacity, playback state, audio, quality, telemetry, authentication, or persisted preferences.
5. A system-theme change must not recreate or navigate the video element, replace the Twitch player object, call `play`, resume a manually paused/autoplay-blocked player, change volume/mute/quality, or change the native session.
6. If Twitch requires navigation to change the separate chat theme, only the chat iframe may reload. Preserve its channel, `parent`, panel visibility/collapse state, and surrounding video state. Document that this may discard an unsent cross-origin chat draft; do not claim draft retention.
7. Demo mode follows the system palette and continues to make zero Twitch requests.
8. Do not inspect or modify Twitch's cross-origin iframe DOM. Do not weaken CSP, native capabilities, origin checks, popup rules, or the chat navigation gate.
9. Do not change the app identifier, preferences/profile paths, public Twitch client ID, parent hostname, or visible product/tagline.
10. No hosted-site deployment, DNS/Twitch-registration change, release, or tag is authorized by this plan.

## Required decision gate: standalone Twitch chat theming

Complete this gate before production implementation.

The current separate chat iframe has no officially documented theme parameter. `darkpopout=1` is a candidate seen in Twitch ecosystem guidance, but it is not present in the current official standalone-chat parameter table. Do not treat it as supported merely because it is familiar.

In a disposable spike worktree, use the configured HTTPS parent and the existing app-owned browser profile to test:

- fresh profile, OS light, `parent` only;
- fresh profile, OS dark, candidate exact dark parameter;
- signed-in profile, OS light, `parent` only;
- signed-in profile, OS dark, candidate exact dark parameter;
- live light-to-dark and dark-to-light changes;
- chat identity/session behavior after the expected chat-only reload.

Record the exact URL shape, OS, native webview/runtime, whether chat visibly matches, whether Twitch recalls an account-specific theme, and whether login remains usable. Do not record credentials, cookies, messages, or unredacted account data.

Decision:

- If one exact standalone-chat URL mapping reliably produces light and dark in both profile states, freeze it in a short decision record and proceed. Permit only that exact mapping in Rust.
- If light mode is controlled by remembered Twitch state, the candidate is ignored, or results are inconsistent, stop the chat portion and escalate. Do not silently inject styles into Twitch, broaden URL navigation, or migrate to combined `Twitch.Embed`.
- A combined-embed migration requires a separate approved ADR and plan covering player API compatibility, audio/quality behavior, playback/session retention, authentication, geometry, and protocol/security regression. It is not an automatic fallback in this feature.

The feature is not complete if app chrome changes theme but Twitch chat cannot meet the contract.

## Execution graph

```text
DM-000 baseline + Twitch feasibility gate
              |
              v
       DM-001 freeze contract
          /           \
         v             v
DM-010 app palettes  DM-020 chat behavior/tests
          \           /
           v         v
        coordinator integration
                  |
                  v
       DM-030 native URL/security
                  |
                  v
       DM-040 independent review
                  |
                  v
       DM-050 native acceptance
```

DM-010 and DM-020 may run in parallel because their production leases do not overlap. DM-030 starts only after the chat URL decision and integration of DM-020. DM-040 is review-only on its first pass. Route findings back to the owning task, then re-review.

## File-lease ledger

The coordinator must replace `TBD` with the assigned worktree, base commit, agent/model class, and status before a worker starts.

| Task | Exclusive writable lease | Complexity | Worktree/base/status |
|---|---|---|---|
| DM-000 | `docs/dark-mode/twitch-chat-feasibility.md` and disposable spike files that will not merge | High architecture/runtime | main `b167bdc` / coordinator / DONE 2026-09-21 |
| DM-001 | `docs/dark-mode/theme-contract.md` and lease ledger only | High architecture | main `b167bdc` / coordinator / DONE 2026-09-21 |
| DM-010 | `ui/style.css`, `player-wrapper/style.css`, `player-wrapper/chat.css`, optional new `tests/theme.browser.cjs` | Simple-to-medium CSS | main `b167bdc` / coordinator / DONE 2026-09-21 |
| DM-020 | `player-wrapper/chat.js`, `tests/chat.test.cjs`, `tests/chat.browser.cjs` | Medium-high JS/browser | main `b167bdc` / coordinator / DONE 2026-09-21 |
| Integration | conflict resolution only; `scripts/ci-check.sh` if a new test must be wired in | Coordinator | main / coordinator / DONE 2026-09-21 (no ci-check change needed; browser suites are opt-in) |
| DM-030 | `src-tauri/src/player.rs`, `src-tauri/hosted-player-adapter.js`, their adjacent Rust tests, `tests/hosted-player-adapter.test.cjs` | High native/security | main `b167bdc` / coordinator / DONE-locally 2026-09-21 (native cargo compile blocked: missing libdbus-1-dev/webkit2gtk; allowlist logic verified by standalone url-crate mirror) |
| DM-040 | Read-only; coordinator may later lease a new QA-only fixture | High independent review | feature/dark-mode `45ed672` / independent reviewer / DONE — two read-only passes, no blocking findings (pre-fix pass flagged P1/P2/quality items that are all addressed; post-fix pass closed with no blocking, 6 non-blocking cosmetic notes) |
| DM-050 | `docs/dark-mode/acceptance.md` and evidence paths only | High native QA | main / coordinator / BLOCKED (no native runtime on this machine) |

No worker may edit `web/parent.mpdviewer.com/index.html`. Shared controller/model/storage/permission files are outside all leases. A worker needing an out-of-scope file must stop and ask the coordinator to re-plan ownership; it must not expand its own lease.

## DM-000 — Baseline and Twitch feasibility

**Agent:** Architecture/runtime subagent  
**Complexity:** High; use the most capable available model with high reasoning  
**Reviewer:** Native/security reviewer  
**Dependencies:** None

### Work

1. Recheck the current branch, commit, dirty files, configured player origin, Tauri version, target OS, and installed webview version.
2. Run the baseline checks listed below and record actual results rather than copying this plan's counts.
3. Confirm that `matchMedia('(prefers-color-scheme: dark)')` changes live in the manager, bundled player, and configured hosted top-level webview on the available native runtime.
4. Execute the standalone Twitch chat decision gate above in a disposable spike. Do not merge spike code.
5. Produce `docs/dark-mode/twitch-chat-feasibility.md` with redacted evidence and one of: supported exact mapping, unsupported, or blocked with the exact missing environment/user action.

### Exit criteria

- The exact current checkout and environment are recorded.
- App-owned webview theme propagation is observed natively or explicitly blocked.
- The standalone-chat URL decision is evidence-backed.
- No hosted snapshot, deployed site, user profile data, or production source was changed.

## DM-001 — Freeze the theme contract

**Agent:** Coordinator or architecture subagent  
**Complexity:** High  
**Dependencies:** DM-000

Write a concise `docs/dark-mode/theme-contract.md` that freezes:

- `system` as the only mode and no persistence;
- light and dark semantic tokens;
- the exact verified chat URL shapes;
- startup and live-change behavior;
- whether a theme change immediately reloads hidden chat or defers it until restore (choose one deterministic behavior and test it);
- chat-only reload/draft-loss disclosure;
- the exact `allowed_chat_url` rule;
- fallback/stop behavior when OS theme events or Twitch theming are unavailable;
- the evidence required per supported platform.

Prefer immediate, deduplicated chat updates so a restored panel cannot show a stale theme. A repeated event for the already-applied theme must be a no-op. If the feasibility evidence favors deferral, document the reason and add equivalent restore tests.

Do not create a settings field, a native management command, or a new privileged bridge merely to distribute theme state. If a supported webview fails to propagate the OS preference, stop and design a separate minimal native theme bridge with an exclusive Rust/JS lease and security review.

## DM-010 — App-owned light/dark palettes

**Agent:** Frontend styling subagent  
**Complexity:** Simple-to-medium; a fast model is appropriate after DM-001 is frozen  
**Dependencies:** DM-001  
**Writable scope:** Only the DM-010 lease

### Work

1. Refactor the manager, bundled viewer, and shared chat-shell CSS to semantic custom properties. Keep the existing dark palette visually stable.
2. Add a complete light palette under an explicit `prefers-color-scheme` branch and declare native-control support with `color-scheme: light dark` (or an equivalent explicit per-branch value).
3. Replace dark-only literals for surfaces, text, muted text, borders, buttons, inputs/selects, pills, player cards, diagnostics, errors, auth panels, toolbar, chat status/note, Demo content, hover/focus/disabled states, and scrollable backgrounds.
4. Keep video letterboxing dark where that is a media requirement; do not mistake the video canvas for app chrome.
5. Preserve brand cyan where it remains legible. Use a darker accessible cyan for light-theme text/focus/borders when necessary rather than changing the logo asset.
6. Ensure the native-injected hosted shell is readable even before or after the chat iframe loads. Do not edit the hosted snapshot.
7. Add focused browser coverage for computed theme colors if that coverage can be isolated in a new test file without taking another task's lease.

### Acceptance

- Both themes have intentional values for every visible state; no dark-on-dark or light-on-light fallback remains.
- Text and controls meet WCAG 2.1 AA contrast targets: 4.5:1 for normal text, 3:1 for large text and meaningful UI boundaries/focus indicators.
- Form controls, focus rings, selection/accent states, scrollbars where supported, errors, and disabled states remain recognizable.
- Responsive geometry and the 400 x 300 minimum video area remain unchanged.
- No JavaScript, Rust, settings, snapshot, or deployment changes.

## DM-020 — Theme flow into official chat

**Agent:** Web/chat behavior subagent  
**Complexity:** Medium-high; balanced coding model with careful browser reasoning  
**Dependencies:** DM-001  
**Writable scope:** Only the DM-020 lease

### Work

1. In `mpdInstallChat`, derive the initial system theme using one `matchMedia('(prefers-color-scheme: dark)')` object.
2. Build the chat URL from the validated assigned channel and `location.hostname`, then add only the exact theme query shape frozen by DM-001.
3. Subscribe to live theme changes using the runtime-compatible media-query listener. Deduplicate identical state and avoid timers/retry loops that can cause repeated navigation.
4. On a real theme transition, update only the existing chat iframe URL. Do not move the video or its ancestors, replace the panel, recreate the player, call playback/audio/quality methods, or invoke native commands.
5. Preserve collapse state, accessibility attributes, status/error behavior, and the correct assigned channel. Keep Demo purely local with no iframe, theme URL, listener side effect, or Twitch request.
6. Make the VM fixture's `matchMedia` deterministic and capable of firing change events.
7. Extend browser coverage with `page.emulateMedia({ colorScheme: 'light' })`, `dark`, and runtime switching for hosted, bundled, and Demo modes.

### Required assertions

- Initial light and dark iframe URLs exactly match DM-001.
- One chat iframe exists; channel and `parent` remain exact.
- Repeated same-theme events cause zero navigation.
- One real theme transition causes at most one chat navigation.
- The video DOM node, containing grid, Twitch player object, session, pause state, volume, mute, and quality are unchanged.
- No extra `play` call occurs.
- Collapsed chat remains collapsed across a theme transition and is correct when restored.
- Chat load/error remains isolated from video.
- Demo has no Twitch iframe/request.
- Foreign assignment/origin/path/top-frame cases remain rejected.
- CSP violations remain empty in the route-mocked browser test.

## Coordinator integration checkpoint

Merge DM-010 and DM-020 one at a time after reviewing their handoffs and diffs. Resolve conflicts centrally. If a new theme test file should be part of standard source checks, only the coordinator edits `scripts/ci-check.sh`.

Run focused source/browser tests before starting DM-030. Confirm the production chat URL emitted by JavaScript is byte-for-byte compatible with the frozen native navigation rule.

## DM-030 — Native navigation security and injected error theme

**Agent:** Native/security integration subagent  
**Complexity:** High; most capable available model/high reasoning  
**Dependencies:** Integrated DM-020 and the supported decision from DM-000  
**Writable scope:** Only the DM-030 lease

### Work

1. Change `allowed_chat_url` only enough to permit the exact light and dark URL shapes from DM-001. Preserve HTTPS, `www.twitch.tv`, no credentials, default port only, exact assigned channel path, exact parent hostname, and no fragment.
2. Parse query pairs structurally. Reject duplicate `parent`, duplicate theme keys, unknown keys, wrong theme values, extra parameters, encoded lookalikes, foreign channel/parent, userinfo, custom ports, fragments, and non-HTTPS URLs.
3. Keep chat without native capabilities. Do not broaden `players.json`, manager permissions, CSP origins, popup rules, or general Twitch navigation.
4. Theme the adapter-created initialization failure notice in `src-tauri/hosted-player-adapter.js` using app-owned system-responsive styles. Do not change adapter authority, message filtering, startup order, playback calls, or telemetry.
5. Add positive and negative Rust/adapter tests adjacent to the existing coverage.

### Exit criteria

- Only the two frozen chat URL shapes pass.
- Existing protocol, playback, quality, audio, and security tests still pass.
- The hosted failure notice is readable in both themes without touching the provenance snapshot.
- No capability, controller, storage, auth, or deployment file changed.

## DM-040 — Independent security, behavior, and accessibility review

**Agent:** Independent review/QA subagent  
**Complexity:** High  
**Dependencies:** DM-010, DM-020, DM-030 integrated  
**First pass:** Read-only

Review the integrated diff against this plan and test:

- exact URL allowlisting and all negative cases;
- no new IPC/native authority or broadened navigation;
- no edit to `web/parent.mpdviewer.com/index.html` (verify its recorded provenance/hash checks still pass);
- initial and live light/dark behavior in manager, bundled viewer, injected hosted viewer, Demo, failure notices, and official chat;
- video/player identity, manual pause, audio, quality, session, and collapse retention;
- no duplicate event listeners/navigations after repeated installer calls;
- error, focus, hover, disabled, status, form-control, and responsive states;
- contrast with an automated checker plus manual inspection of screenshots;
- test claims are labeled as unit, route-mocked browser, native runtime, or live Twitch evidence.

The reviewer reports findings to the coordinator. The owning implementation agent fixes each accepted finding in its existing lease; the reviewer then rechecks. Severe navigation, permission, duplicate-playback, or state-loss defects block acceptance.

## DM-050 — Native acceptance matrix

**Agent:** Native QA subagent, coordinated with the user only for normal OS/Twitch UI interaction  
**Complexity:** High  
**Dependencies:** Independent review has no blocking findings

Test each claimed supported platform separately. At minimum, run the current release target's Windows/WebView2 path. Record macOS/WKWebView and Linux/WebKitGTK as passed, failed, or blocked; never infer them from Chromium/Edge.

For each available platform:

1. Launch with OS light mode. Inspect manager, native window chrome, bundled Demo, hosted viewer chrome, and official chat.
2. With manager and viewer open, switch OS to dark, then back to light. No app restart.
3. Repeat with chat expanded and collapsed, player playing, player manually paused, player autoplay-blocked if reproducible, and chat signed in if the user chooses to complete normal Twitch UI authentication.
4. Verify only chat reloads when required; video and its playback state do not.
5. Verify chat still has the assigned channel and expected signed-in/signed-out behavior after reload. Do not send messages as part of automated acceptance.
6. Exercise narrow and wide viewer geometry and manager form controls.
7. Capture redacted screenshots and a concise environment/result table. Do not capture credentials, cookies, MFA, private messages, or raw account data.

A blocked platform is not a pass. If the platform is advertised as supported, leave the feature gate open until it is tested or the support statement is explicitly narrowed through a reviewed product decision.

## Commands and evidence

Run the smallest focused checks during each task, then the full relevant set after integration:

```sh
git status --short --branch
git diff --check

node --test tests/chat.test.cjs tests/hosted-player-adapter.test.cjs
node --test tests/view-model.test.mjs tests/viewer-count.test.mjs tests/quality.test.cjs tests/hosted-source-characterization.test.cjs tests/hosted-player-adapter.test.cjs tests/chat.test.cjs
python3 tests/configuration_test.py
python3 -m unittest discover -s tests -p 'release_test.py' -v
bash scripts/ci-check.sh

cargo fmt --all -- --check
cargo test --locked --workspace --features mpd-tabber/custom-protocol
cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol -- -D warnings
cargo build --locked --release -p mpd-tabber --features custom-protocol
```

Run the Playwright theme suites using the repository's documented installed-browser setup. `node tests/chat.browser.cjs` requires Playwright to be resolvable and currently uses installed Edge. Do not install dependencies, change lockfiles, or write generated screenshots into tracked paths without reviewing the environment and obtaining the appropriate lease. Browser emulation is not native OS-theme evidence.

For a native run:

```sh
cargo run --locked -p mpd-tabber --features custom-protocol
```

Record source tests, browser tests, native compilation, native runtime theme switching, and live Twitch chat observations as separate evidence categories.

## Definition of done

- Manager and all app-owned viewer surfaces correctly render at startup in system light and dark modes.
- Live OS theme changes propagate without restarting the app.
- Official Twitch chat visibly follows both modes using the exact mechanism proven in DM-000.
- Any chat-only reload is bounded, deduplicated, disclosed, and does not change video/player/session/audio/quality/pause state.
- Demo remains network-isolated from Twitch.
- Exact chat channel/parent/navigation restrictions and existing CSP/native capabilities remain least-privilege.
- Both palettes pass contrast and interaction-state review.
- Required source, Rust, and browser checks pass.
- Native evidence is recorded per claimed platform; blocked checks remain explicitly blocked.
- The hosted provenance snapshot is unchanged, and no site deployment, release, or tag occurred.
- An independent reviewer has no unresolved blocking findings.

## Explicit non-goals

- No manual theme setting or saved preference.
- No redesign of selection, controller, player protocol, authentication, Turbo behavior, grid layout, or viewer-count behavior.
- No combined Twitch embed migration without a separate approved ADR/plan.
- No cross-origin Twitch DOM styling, custom chat client, automated chat, new OAuth scopes, popup expansion, or cookie handling.
- No edits to the hosted provenance snapshot and no deployment.
- No release/version/tag/signing changes.

## Required handoff from every subagent

Use the repository format verbatim:

```text
Task / base commit / worktree:
Model and effort actually used:
Files changed:
Behavior implemented:
Checks run: exact commands and outcomes
Checks blocked or not run: reason and needed environment
Native runtime / wrapper version where relevant:
Known limitations and remaining decisions:
Security or migration implications:
Patch/commit and suggested merge order:
```

Do not include raw reasoning transcripts or secrets. A task is complete only when its acceptance criteria passed or its explicitly research-only finding was delivered.
