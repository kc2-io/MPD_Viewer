# Viewer integration delivery plan — 2026-09-24

Base: current origin/main 2ea2960, merged with tested spike a3f3bd2 on integration
branch feat/viewer-next at 2e163e6. Preserve prior commits, profile/app identity,
preferences, client ID, hosted provenance and native capability separation.

## Work areas and exclusive ownership

1. Timer worker (high-complexity scheduling/lifecycle): work/viewer-timer.
   Own crates/core, src-tauri/src/{controller,model,storage}.rs, ui/{app.js,index.html,
   style.css,view-model.js}, timer tests/fixtures and timer docs only. Implement
   existing STREAM-TIMER-IMPLEMENTATION-PLAN with optional per-channel minutes,
   default Always, convenient 10-minute setting, assigned-time countdown and
   safe close-before-open rotation. No auth implementation changes.
2. Auth worker (high-complexity credentials): work/viewer-auth-persistence.
   Own crates/twitch, new src-tauri/src/credential_store.rs, auth-persistence tests
   and docs only. Investigate memory-only OAuth; implement Windows protected
   refresh credential persistence, restore/validate APIs, refresh rotation and
   test seam. No controller/main/Cargo edits; give coordinator explicit wiring.
3. Research worker (bounded native/platform investigation, read-only): inspect
   documented full-page theming and media controls; cite Twitch and WebView2/Tauri
   APIs. No undocumented Twitch APIs, DOM automation, spoofing or cookie handling.
4. Coordinator: serialize integrations and own native viewer/main/Cargo/capability
   changes, runtime --embedded-viewer flag, native theme propagation and any supported
   media controls. At most two production writers active; wait for one worker to
   finish before native implementation. Auth controller wiring happens after timer
   lease ends. Keep topic commits separate.
5. Independent reviewer: read-only security/lifecycle review of final integration;
   implementation findings return to owner for correction and re-review.

## Decisions and gates

- Full Twitch page is default in one binary. Hidden launch flag --embedded-viewer
  selects the existing embed path; no settings/UI switch or persisted backend.
- Follow OS theme using supported native browser mechanisms where possible.
  Respect native Twitch appearance behavior; do not style Twitch cross-origin DOM.
- Web-page volume/quality controls are conditional on a documented supported route.
  If no such route exists, retain Twitch's own controls and record the limitation.
- Timer measures running assigned time, not Twitch credited watch time. Pause of
  automation freezes it; playback pause does not. Runtime rounds are not persisted.
- Credentials never enter preferences, webview messages or logs. Restore must validate
  before use, preserve transient-failure recovery and stop revoked credentials.
  Explicit disconnect removes saved credential. No raw cookie/profile inspection.
- No release, tag, signing change or website deployment. Merge/integration proceeds
  through existing protections and current task authorization; no force pushes.

## Verification and delivery

Run focused policy/storage/auth tests, full Rust workspace tests, source checks,
mocked UI/browser checks and Windows release-profile build. Verify runtime flag,
native themes, saved authorization after restart and a short native timer scenario
separately from mocks. Request only user-owned Twitch authentication interaction.
Record unavailable macOS/Linux checks honestly. Deliver reviewed commits/PR and
an unsigned Windows test artifact with exact source identity and test evidence.
