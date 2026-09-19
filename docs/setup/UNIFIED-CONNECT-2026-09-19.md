# Unified Twitch connection and fixed public application ID

Base: `b289bd352b2d032f494983a14a8f4643477157cb` (PR #10). Branch: `feat/unified-twitch-connect`. Worktree: `D:\MPD_Viewer-Codex-Project\MPD_Viewer`. Coordinator owns every changed file; `setup_review` independently reviewed read-only using inherited session model/effort settings, with no override.

## Behavior

On Windows, **Connect Twitch** requests a device grant using MPD Viewer's fixed public client ID and automatically opens Twitch's supplied, prefilled activation link in the existing viewer browser profile. The user signs in and approves access directly in Twitch. Rust continues polling for the device-flow result. No credentials or auth-page content are inspected or injected.

The separate viewer sign-in button and editable client-ID field are removed. `Connect {}` accepts no parameters; even a direct manager IPC call containing `client_id` is rejected. Settings no longer deserialize or serialize a client-ID field. Legacy overrides are ignored, while favorites, ordering, enabled flags, limits, volume, mute, Demo and schema remain intact. Saved preferences shed the obsolete field on their next normal save. The registered public client ID itself is unchanged.

While a grant is pending, the same button becomes **Continue in Twitch** and focuses or reopens that attempt without requesting another grant. Cancel/Disconnect aborts the grant and closes its connection window. Closing only the Twitch window leaves the pending attempt available to continue until expiry. Switching to Demo cancels a pending connection. Successful API authorization leaves Twitch's page available for the user to finish and close; it does not reload/resume players or assert website/video identity. A later reconnect closes that completed window first.

Monitoring credentials remain in Rust memory and require reconnection after quitting; secure token persistence is separate work. Website sessions may persist independently. Other platforms return an explicit unsupported error before requesting a device grant. This Windows preview does not establish cross-platform login support.

## Security and lifecycle

The initial URL must be HTTPS, exact `www.twitch.tv`, `/activate`, with no credentials, non-default port or fragment. It must contain exactly one `device-code` matching Twitch's returned user code. The optional `public` parameter may occur once and must equal `true`; no additional keys are accepted. Further navigation permits official `/login`, `/`, and `/activate`, plus the constrained `id.twitch.tv/oauth2/authorize` continuation described below; activation query values remain bound to the current attempt. The code-less `/activate` path permits a completion page. New windows and downloads remain denied. No capability or profile configuration changes are made.

Window labels include the authorization epoch and match neither manager nor player capabilities. Pending state and matching epochs gate incoming auth results. Malformed links and open failures abort/invalidate the attempt. Window cleanup has separate tracking, so a failed close retains its old identity for retry even after the grant is invalidated; a new grant cannot start until cleanup succeeds. Error text excludes raw URLs/codes.

## Executed verification

- `cargo test --locked --workspace --features mpd-tabber/custom-protocol`: 26 tests passed, including legacy settings migration, forged client-ID rejection, activation URL binding, navigation restrictions, and failed-close/retry tracking.
- `cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol`: passed with the same two pre-existing controller style warnings.
- `cargo build --locked --release -p mpd-tabber --features custom-protocol`: optimized Windows executable built successfully; unsigned test build.
- `node tests/viewer-auth.browser.cjs` with installed Edge/Playwright: passed. Real manager HTML/modules with a mocked IPC bridge cover Demo, initial/pending/continue/cancel/reconnect UI, removed controls, fixed Connect payload, retained player cards and absence of JavaScript errors. This is not native authentication evidence.
- Independent reviewer: 49 JavaScript, 14 configuration and 51 release checks passed (114 total); final source review approved after the failed-close correction. Coordinator reran configuration checks and `git diff --check` successfully.
- Disposable native fixture, separate test profile, Tauri 2.11.5/WebView2: actual ACL denied `get_state`, `dispatch` and `player_report` from an auth-window label. The production window module opened/refocused one window, reopened it after destruction, and exited with the window open. Exit code 0; WebView2 emitted a class-unregister shutdown warning.
- Disposable native controller harness ran the actual controller/modules with isolated identity and in-memory preferences. Passed: cancel before a device reply; ignore stale AuthCode/Authorized events; repeated Connect preserves its epoch; malformed current activation link invalidates the attempt and rejects later completion; native Demo rejects Connect. Exit code 0. Synthetic auth events characterize lifecycle only; no account authorization or session recognition was inferred.

## User acceptance and delivery boundary

The unified unsigned preview has been launched using the existing app profile. Real Twitch activation/approval and viewer recognition for this combined flow await the user's observation. Earlier PR #10 separately established user-reported website-session sharing and restart persistence; that does not substitute for testing this new activation route. Fresh-profile login/MFA and video identity/Turbo remain unverified.

No new release, tag, website deployment, Twitch registration change, cookie import/export or automated chat is included. Files changed: native model/controller/storage/auth window; manager HTML/JS; chat help text; browser/configuration fixtures and checks; this evidence record. Existing preferences paths, app identifier, capabilities, player lifecycle policy and hosted source snapshot are preserved.

## Activation-link compatibility correction

The first implementation incorrectly required the `public=true` parameter shown in Twitch's documentation example. On September 19, 2026, a live device-code response from the unchanged app registration returned the exact HTTPS `www.twitch.tv/activate` destination with only a matching `device-code` parameter. Only structural metadata was inspected; code/token values were not logged or saved.

The parser now accepts this actual response shape as well as the documented example. A matching code remains mandatory; missing/mismatched/duplicate codes, false/duplicate public flags, unknown keys, foreign destinations, userinfo, non-default ports and fragments remain rejected. Navigation and account permissions are unchanged.

Verification: 27 Rust workspace tests passed, including the new actual-shape regression and negative cases; optimized Windows build passed; independent read-only security review approved. A disposable Rust diagnostic called the actual Twitch client and production activation parser with a fresh live response and passed without performing authorization or printing codes. This establishes live response compatibility, not completed website login or viewer identity. No release/tag/site deployment.

## Authorization continuation correction

After the activation-link correction, the user reproduced an unsupported-navigation message when pressing Activate. A disposable native diagnostic observed only structural route metadata: HTTPS `id.twitch.tv/oauth2/authorize`, default port, no fragment, and query keys `client_id`, `device_code`, `force_verify`, `redirect_uri`, `response_type`, `scope`, and `user_code`. No query values, credentials, cookies or raw authentication URLs were recorded. Twitch's anonymously retrieved public activation-page JavaScript confirms that its verification response supplies the authorization URI which the page navigates to.

The navigation policy now permits this exact endpoint with exactly those seven unique fields. The client ID must be MPD Viewer's fixed ID, the user code must match the pending attempt, and scope must be empty. Device code and response type are bounded; force_verify must be a Boolean string. The redirect must parse as HTTPS on exact `www.twitch.tv`, without credentials, a nondefault port or a fragment. Other navigation remains restricted; this does not permit arbitrary Twitch paths or third-party redirects. No controller, profile, capability, popup or download permissions change.

Verification: the optimized Windows build passed. 28 Rust workspace tests passed, including missing/duplicate fields, wrong app/user codes, added scopes, unsafe destinations and malformed parameters. Independent read-only review approved the implementation. The positive unit fixture characterizes policy, not the actual private query values. The updated disposable native viewer was opened for user testing; completed authorization and the final return route are not yet verified. No release, tag or website deployment.


### Twitch authorization frontend

The next native attempt passed the identity-server endpoint but encountered another HTTPS `/authorize` route with the same seven parameter names. Its hostname was redacted by the diagnostic's too-small fixed hostname list, so that trace alone does not identify the host. Anonymous inspection of Twitch's public `https://auth.twitch.tv/authorize` page and its `auth-main-34b9f641fa883b71e87c.js` bundle confirms a dedicated `/authorize` frontend which calls the identity server for authorization data. The policy now also supports that exact first-party host/path with the identical seven-field validation. It does not enable `/authorize/redirect`, other paths, or wildcard hosts.

Verification: 29 Rust tests passed; optimized Windows build passed; independent review approved the actual patch. The diagnostic now recognizes the fixed `auth.twitch.tv` hostname and emits a constant marker if the complete frontend predicate passes. The new native build was opened for another user test. Full authorization and the final return route remain unverified pending that test. No diagnostic code is part of the production patch.


### Exact authorization return

The next native diagnostic confirmed that the actual session uses `auth.twitch.tv/authorize` and passes its complete bound-parameter policy. A later step then tried a query-free, fragment-free HTTPS return on `www.twitch.tv`; the diagnostic redacted its path segments. Rather than guessing that path or permitting arbitrary first-party paths, the viewer now records the exact already-validated `redirect_uri` from the first accepted authorization navigation, in memory for that window only. Repeated authorization navigation must retain the same return URL. One exact matching return is additionally allowed, including its path and query identity; a fresh attempt/window has no recorded return. No raw return URL is logged or persisted.

Existing `/login`, `/`, and `/activate` handling remains explicit. In particular, the `/activate` code/query guard takes precedence over a captured return, so return binding cannot bypass same-code checks or admit unknown activation parameters. Mutex failures deny the new return allowance.

Verification: 31 Rust workspace tests and the optimized Windows build passed. Return tests cover pre-authorization rejection, invalid authorization installing no return, immutable callback identity, altered paths/queries/fragments/authorities, isolation between attempts and activation-guard precedence. Independent review approved after correcting that precedence issue. The updated native diagnostic was opened for user testing and emits only a fixed marker when the exact return passes. Completion of the full native flow remains pending user observation.
