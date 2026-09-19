# Viewer website sign-in — Windows preview

Task: bounded manager-origin sign-in from MV-020/MV-021. Base `6a4633205f1d89642313bee0e223788429efc1b7`, branch `feat/viewer-website-signin`, worktree `D:\MPD_Viewer-Codex-Project\MPD_Viewer`. The coordinator owns all edits; `setup_review` independently reviews read-only using inherited session settings. No model override was applied; an exact provider model/effort identifier was not exposed.

## Behavior and scope

The manager now distinguishes live-status monitoring authorization from **Sign in for viewing**. The latter opens official Twitch website login in the existing app-owned Windows browser profile. Clicking again focuses the same window without restarting its page. Users verify their account directly in Twitch, close the sign-in window, and explicitly Retry existing players if necessary. There is no automatic reload, playback resume or inferred login state.

Monitoring Disconnect clears in-memory API tokens; it does not sign out Twitch's persistent website session. The app identifier, preferences, public client ID, viewer profile and capability files remain unchanged. Demo rejects viewer sign-in in both UI and Rust. Non-Windows returns an explicit unsupported-for-testing error.

The new `viewer-auth` window matches neither the manager nor player capability. Its top-level navigation permits only HTTPS `www.twitch.tv` at `/login` or `/`, without userinfo or non-default ports. Other routes, popups and downloads are denied. No credential/cookie access, auth-page scripts or raw auth URL logging is added. Login challenges requiring other routes may be blocked; those flows require separate observation and review.

This is a dedicated manager-origin website login, not MV-021's proposed related-popup implementation. Player popups remain denied. Session reset, other platforms, video account identity and Turbo acceptance remain outside this bounded preview. The hosted source snapshot and deployed parent website are unchanged.

Files: `src-tauri/src/{viewer_auth.rs,main.rs,model.rs,controller.rs}`, `ui/{index.html,app.js}`, `player-wrapper/chat.js`, `tests/viewer-auth.browser.cjs`, and this record.

## Executed checks

- `cargo test --locked --workspace --features mpd-tabber/custom-protocol`: 27 passed, including positive/negative sign-in URL cases.
- `cargo build --locked -p mpd-tabber --features custom-protocol`: Windows debug application built successfully; unsigned diagnostic only.
- `node tests/viewer-auth.browser.cjs`: passed with Playwright supplied through `NODE_PATH` and installed Microsoft Edge. Exercises the real manager HTML/modules with a mocked native bridge: Demo disabled, monitoring identity label, one bounded sign-in action, retained player cards and no JavaScript errors. This is not native authentication evidence.
- Source checks: 49 existing JavaScript tests, 14 configuration tests, 51 release unit tests, version check and `git diff --check` passed. Independent reviewer reran JavaScript/configuration checks and approved the corrected final patch.
- `cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol`: passed with two pre-existing controller style warnings.
- Disposable native harness compiled with Tauri 2.11.5 and custom protocol, using an isolated diagnostic identifier. A controlled bundled page labeled `viewer-auth` attempted `get_state`, `dispatch`, and `player_report`; real Tauri ACL rejected all three before sentinel handlers. No test script was injected into Twitch. The harness then called the production sign-in module, confirmed one window after repeated open/focus, closed/reopened it, and exited with the auth window open. Process exited successfully. WebView2 printed a class-unregister warning during shutdown; it did not prevent process exit. This tests ACL/lifecycle, not credentials or identity.

## Actual website-session observations

A separate disposable native diagnostic retained the real app identifier/default profile, without loading preferences or a monitoring controller. It opened official Twitch login and a muted Monstercat viewer. The user completed Twitch sign-in directly. Closing the login window opened a second viewer. The user then reported seeing emotes for channels they subscribe to or follow. This is user-reported evidence of chat-session recognition, not a programmatically verified identity or video/Turbo result.

The user closed both diagnostic viewers. Their process was confirmed absent. A new viewer-only process opened Monstercat without a sign-in window, using the same profile; the user reported that recognition appeared to persist after this restart. This is user-observed chat-session persistence on this Windows installation, not programmatic identity verification. Credentials, cookies and account content were not inspected or logged.

No release/tag publication is part of this patch. The published alpha.1 executable does not contain this new manager control. Cross-platform CI compilation is separate from actual native website-login acceptance. Full MV-020/MV-021 acceptance remains open, including video identity, other runtimes and any requested related-popup/reset work.
