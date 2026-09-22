# Dark mode implementation code review

**Verdict:** Not GTG yet.

The implementation is well structured and most automated checks pass, but it has one release-blocking validation gap and three code-quality defects that should be corrected before approval.

## Required fixes

### P1 — Complete native and signed-in acceptance

The implementation proceeded even though its own feasibility report says signed-in-profile behavior and native WebView2 theme propagation were not tested:

- `docs/dark-mode/twitch-chat-feasibility.md:3-17`
- `docs/dark-mode/acceptance.md:3-7`

This matters because the light chat URL has no explicit light parameter. Twitch may apply a signed-in user's remembered dark preference. If that happens, light-system mode would still show dark chat and the feature would not meet its requirement.

Before approval:

1. Run the Windows/WebView2 acceptance matrix.
2. Test a signed-in profile whose Twitch preference is dark.
3. Verify cold start in light and dark modes.
4. Verify live light → dark → light transitions without restarting the app.
5. Verify the chat-only reload preserves login state, assigned channel, video/player identity, manual pause, autoplay-blocked state, volume, mute, quality, session, and collapse state.
6. Record the results in `docs/dark-mode/acceptance.md` with redacted evidence.

If the plain chat URL remains dark in light-system mode for a signed-in profile, the current separate-chat implementation does not satisfy the feature. Do not claim completion, inject styles into Twitch's cross-origin DOM, broaden navigation, or silently migrate to combined `Twitch.Embed`. Escalate the architecture decision.

### P2 — Make the Rust chat URL gate canonical

`src-tauri/src/player.rs::allowed_chat_url` currently decodes and sorts query pairs. This accepts forms that the JavaScript never emits, despite comments and the contract promising two exact URL shapes.

Examples currently accepted or potentially accepted include:

- `darkpopout&parent=...` with reversed parameter order.
- Percent-encoded keys such as `p%61rent`.
- Encoded spellings that bypass the literal `darkpopout=` raw-query check.

Required correction:

1. Preserve the existing scheme, host, credentials, port, path, channel and fragment checks.
2. Compare the raw query against only these canonical forms:
   - `parent={expected-parent}`
   - `parent={expected-parent}&darkpopout`
3. Reject reversed order, encoded keys, encoded parent spellings, `darkpopout=`, `darkpopout=1`, duplicates, empty segments, unknown parameters and extra parameters.
4. Remove the positive test for the reversed `darkpopout&parent=...` order.
5. Add explicit negative tests for every non-canonical form above.
6. Reconcile `docs/dark-mode/theme-contract.md` so it no longer simultaneously says encoded lookalikes are rejected and accepted as equivalent.

The goal is the smallest possible navigation allowance, byte-for-byte compatible with `player-wrapper/chat.js`.

### P2 — Correct light-theme contrast and its tests

The light palette in `ui/style.css` uses several boundaries and text combinations below the targets specified by the feature plan.

Measured examples:

| Combination | Contrast | Required |
|---|---:|---:|
| `--line:#c2d0d8` against white controls/panels | 1.58:1 | 3:1 for meaningful UI boundaries |
| `--field-bg:#f4f7f9` against white | 1.08:1 | Must remain visually distinguishable, with a 3:1 control boundary |
| `--error-line:#d29aa5` against `--error-bg:#fbe9ec` | 2.02:1 | 3:1 when the boundary conveys the state |
| Running text `#0c7a54` against `#bfe8d6` | 4.00:1 | 4.5:1 for normal text |

Required correction:

1. Introduce distinct stronger light-theme tokens for interactive control borders and other meaningful boundaries instead of relying on the subtle panel-divider color everywhere.
2. Adjust the running pill foreground or background to reach at least 4.5:1.
3. Review error, authentication, tag, input, select, button, panel and status boundaries in both themes.
4. Preserve 4.5:1 for normal text and 3:1 for large text, focus indicators and meaningful UI component boundaries.
5. Extend `tests/theme.browser.cjs` to assert border contrast, not only text against field fill.
6. Actually focus each tested control before reading its `outlineColor`; the current test reads the unfocused computed outline and can pass using `currentColor` instead of the `:focus-visible` style.
7. Cover buttons, text inputs, selects, error state, authentication panel, running status and the chat toolbar.

### P2 — Reset chat loading/error state on every theme navigation

`player-wrapper/chat.js` creates one timeout for the initial iframe load. After the first successful load clears it, a theme change assigns a new `frame.src` without showing loading feedback or starting a new timeout.

Consequences:

- A theme-triggered chat reload that stalls can leave the status hidden indefinitely.
- A previous error is not reset to a loading state before retrying.
- The current unit test changes theme before simulating the first completed load, so it misses the real lifecycle.

Required correction:

1. Factor chat navigation into a helper that:
   - Clears the previous timeout.
   - Shows `Loading Twitch chat…`.
   - Starts a fresh 20-second timeout.
   - Assigns the new URL exactly once.
2. Keep the existing `load` and `error` behavior isolated to chat.
3. Preserve theme-event deduplication so the same theme performs no navigation and starts no new timeout.
4. Add tests that:
   - Complete the initial load first.
   - Change theme afterward.
   - Assert loading feedback returns.
   - Assert the new timeout is active.
   - Exercise timeout, error and eventual successful load.
   - Confirm video/player/pause/audio/quality/session/collapse state remains unchanged.

## Quality improvements

### Make the palette browser test deterministic and offline

`tests/theme.browser.cjs` opens the non-Demo bundled wrapper without routing external URLs. It can contact Twitch and may become network-dependent even though it is intended to test only app-owned colors.

- Use Demo mode where sufficient, or route Twitch SDK/chat requests to deterministic mocks.
- Assert that the palette test makes no unplanned external network requests.
- Keep live Twitch behavior in the separately labeled feasibility/native acceptance checks.

### Cover the injected hosted shell

The new palette browser test covers the manager and bundled wrapper but does not assert computed colors for the native-injected hosted shell.

- Add a route-mocked hosted case using the preserved hosted HTML plus injected `chat.js`/`chat.css`.
- Assert dark and light computed colors, live switching, retained video identity and no CSP violations.
- Do not edit `web/parent.mpdviewer.com/index.html`.

### Declare the color scheme on the injected hosted document

Add `color-scheme: light dark` to the injected `.mpd-chat-document` rule in `player-wrapper/chat.css`. The class is placed on the document element, so this lets hosted scrollbars and any native controls follow the system scheme as well as the explicit colors.

### Correct acceptance documentation

`docs/dark-mode/acceptance.md:30-31` says a light-mode launch should show "Dark chat." Correct this to require light chat in light-system mode.

Ensure test counts and evidence labels remain accurate: source/unit, route-mocked browser, real-browser Twitch, native compilation and native runtime are separate evidence categories.

### Restore feature-branch/worktree hygiene

The review environment did not show a separate linked feature worktree. `git worktree list` showed only the root checkout, and the implementation is currently uncommitted directly on `main` at `b167bdc`.

Before integration:

1. Preserve all current user files and unrelated untracked work, especially `docs/feature-plans/STREAM-TIMER-IMPLEMENTATION-PLAN.md`.
2. Put the dark-mode implementation on a dedicated feature branch/worktree without overwriting or cleaning user changes.
3. Keep the dark-mode commit limited to its production code, tests and evidence documents.
4. Review the final diff against the actual merge base before merging.

## Checks already run during review

Passed:

- `git diff --check`
- `bash scripts/ci-check.sh`
- `tests/chat.browser.cjs` using the existing cached Playwright package and installed browser
- `tests/theme.browser.cjs` using the existing cached Playwright package and installed browser
- `cargo test --locked -p mpd-core` — 18 tests passed
- Hosted provenance, capabilities, controller, storage and identity files remain unchanged

Blocked by the review environment:

- Full workspace Rust test/build: missing Linux `dbus-1` and WebKitGTK development libraries.
- Native Windows/WebView2 runtime acceptance.
- Signed-in Twitch theme behavior.

`cargo fmt --all -- --check` also reports extensive repository-wide pre-existing formatting drift, including files outside this feature. Do not mechanically reformat the repository as part of this fix. Keep any formatting changes limited to edited feature code unless a separate formatting task is approved.

## Final acceptance criteria

The patch is GTG only when:

- The four required findings above are corrected and independently re-reviewed.
- Signed-in Windows/WebView2 acceptance demonstrates chat matching both system themes, or the architecture limitation is escalated instead of waived.
- Only the two canonical chat URL query strings pass the native navigation gate.
- Light and dark palettes meet the stated contrast targets, including focused controls and meaningful boundaries.
- Every chat navigation has correct loading, timeout, error and recovery behavior.
- Theme changes never recreate or resume video or change session/audio/quality/pause state.
- Demo and deterministic palette tests make no Twitch requests.
- The hosted provenance snapshot and native capability boundaries remain unchanged.
- Source, browser and available Rust checks pass, with unavailable native platforms recorded as blocked rather than passed.

No code was modified during the architecture review that produced this remediation list.
