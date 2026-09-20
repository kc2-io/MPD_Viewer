# Drag channel ranking

Base: released main `6fc9a5428cc3ad9bc5f63c28b13363af98d8ed3a`. Branch `feat/drag-channel-ranking`. Worktree `D:\MPD_Viewer-Codex-Project\MPD_Viewer`.

## Outcome and interaction

Drag a channel row or its visible grip to change priority. A cyan insertion marker shows the before/after position. Keep the up/down buttons, their keyboard operation and first/last boundaries. Drop dispatches the existing Rust `move` command once; Rust persists order and remains the selection authority. Cancelled, self and external drops do not change order. Keep the list stable during dragging and handle stale order or failed saves visibly. No new layout/grid feature or player lifecycle change.

## Task ownership (ready before implementation)

1. Coordinator: exclusive production ownership of `ui/app.js`, `ui/style.css`, `ui/index.html`, `src-tauri/tauri.conf.json`, feature documentation, and any small CI test wiring. Disable native file-drop interception only for manager `main`; preserve all other config/security/profile settings. Replace the minimal drag handler with insertion semantics, internal-drag validation, cleanup and accessible feedback. Preserve arrows and prevent row controls initiating a drag.
2. Browser-test agent: exclusive ownership of new `tests/ranking.browser.cjs`. Real Edge/Playwright manager UI with a mocked native bridge. Cover down/up and first/last drops, no-op/cancel/external drops, periodic refresh stability, failed dispatch, arrow controls and ranking feedback. Do not modify production or shared fixtures. Run once coordinator implementation is ready; report browser-only evidence accurately.
3. Independent reviewer: read-only review of production/config/test changes after implementation. Check off-by-one positions, stale updates, disabled channels, control interactions, external drop handling, and unchanged Rust/IPC authority. Route findings to coordinator; do not implement review findings.

## Acceptance and delivery

- Browser interaction tests pass with actual mouse dragging as well as edge-case event tests.
- Existing manager/browser and source/config tests pass; Windows optimized build succeeds.
- Record native drag behavior separately from browser mocks. Provide an unsigned native build for user verification; no release/tag/website deployment.
- Open a feature PR with plan, verification and any remaining native/cross-platform limits. Preserve settings and current app identity.


## Implementation and executed checks

The existing HTML drag handlers could not run normally on Windows because the default native Tauri file-drop handler intercepted them. The manager window now sets `dragDropEnabled:false`, as required by [Tauri's configuration documentation](https://v2.tauri.app/reference/config/#dragdropenabled). The setting is scoped to main; authentication/player window construction and capabilities are unchanged. Document-wide handlers prevent file or URL drops from navigating away from the manager.

Rows expose a visible grip and a cyan before/after insertion line. Mouse dragging dispatches exactly one existing `move` action with the post-removal index; no backend, storage schema or selection changes were needed. Up/down buttons remain usable by mouse and keyboard. Disabled channels remain reorderable. Internal drag state and current login order guard drops; list nodes remain stable through polling and pending saves. A failed save displays an error. If saving succeeds but refreshing fails, further ranking stays blocked until an authoritative snapshot arrives, avoiding moves against stale positions. Visible polite live feedback reports progress, results and cancellation.

Coordinator owns all production/configuration/documentation changes and the configuration regression. `ranking_browser_tests` owns only the new browser test, plus disposable visual captures. `setup_review` reviewed read-only; its refresh-recovery finding was corrected and re-reviewed. Agents used inherited session model/effort settings with no overrides. No release/tag/website deployment is included.

Executed:
- `node tests/ranking.browser.cjs`, installed Edge 153.0.4234.32 via Playwright: 15 cases passed. Real mouse grip/text drags, before/after/first/last movement, no-ops, Escape, external/control drops, arrows, disabled channels, periodic polling, pending save, stale order, in-flight old-read race, save failure and saved-but-refresh-failed recovery. Native bridge is mocked.
- `node tests/viewer-auth.browser.cjs`: passed after the final refresh changes, preserving the existing manager connection UI behavior with a mocked bridge.
- `node --test tests/view-model.test.mjs tests/hosted-source-characterization.test.cjs tests/hosted-player-adapter.test.cjs tests/chat.test.cjs`: reviewer ran all 49 checks successfully.
- `python tests/configuration_test.py`: 15 passed, including a new regression for the manager's Windows HTML drag configuration.
- `cargo test --locked --workspace --features mpd-tabber/custom-protocol`: 32 passed.
- `cargo build --locked --release -p mpd-tabber --features custom-protocol`: optimized Windows build passed after the final implementation changes.
- Browser captures at 1280px and the native minimum 860px: grips, controls, insertion line and feedback visible; no horizontal overflow/clipping.
- Independent source/test review approved; `git diff --check` passed.

Browser/native limits: real WebView2 drag routing and a restart proving persisted ranking have not been exercised for this patch. The unsigned local Windows preview is ready for that acceptance test. Cross-platform native drag behavior is not established by the browser checks or Windows compilation. Existing Rust move/storage behavior is reused, but mocked browser tests do not prove persisted preferences.
