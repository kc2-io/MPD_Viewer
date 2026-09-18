# MPD Viewer — Proposed Shared Contracts

**Status:** Architecture input to MV-001, not implemented API definitions. Resolve native feasibility before freezing signatures. Keep changes proportional to this small POC.

## Invariants

1. Favorite priority and live-status observations determine selection; display mode never does.
2. A channel has at most one managed active playback surface. Opening and closing surfaces reserve capacity until ownership is resolved.
3. Authentication popups are not playback sessions, but remain tracked for cleanup.
4. Logical session identity survives a layout move. Recreated renderers get a new surface instance/epoch.
5. Only the controller authorizes session creation, replacement, layout changes, and cleanup.
6. Remote content has no management authority. Its telemetry is advisory even when its caller is verified.
7. Stop/exit overrides pending layout transactions. No delayed callback may resurrect a viewer.
8. API authorization, browser website sign-in, chat identity, video entitlement, and observed playback are distinct concepts.
9. Credentials and browser-cookie values are not protocol messages, UI state, logs, or diagnostic exports.
10. Manager UI is not a host for untrusted Twitch content.

## Domain types and ownership

The contract agent should define the smallest useful equivalent of:

```text
ViewerSession {
  session_id, channel, active_surface, placement,
  desired_audio, last_observed_playback, lifecycle
}
SurfaceRecord {
  surface_id, instance_epoch, session_id, native_webview_label,
  container_id, profile_id, closing_reason
}
LayoutState {
  desired_mode, committed_mode, revision, phase, transaction_error
}
LayoutMode = Standalone | Grid
CloseReason = UserSkip | Stop | Preemption | Retry | LayoutReplacement | EmptyContainer | Failure
```

Persist layout preference, chat defaults, and sensible window geometry. Do not persist transient native labels or treat restored session IDs as live sessions. Keep the existing settings identity and public Client ID.

Use one owner for `model.rs`/`storage.rs` schema changes. Prefer backward-compatible optional/defaulted settings fields for this milestone unless a real migration requirement warrants a new schema version. Do not broaden deserialization globally to accommodate arbitrary payloads.

## Presentation adapter

Separate static asset hosting from surface lifecycle. Names are illustrative:

```text
create_surface(session, surface_epoch, placement, audio, profile) -> pending operation
move_surface(surface, target_placement, layout_revision) -> result/event
apply_audio(surface, desired_audio) -> result
apply_bounds(surface, logical_rect) -> result
focus_surface(surface) -> result
close_surface(surface, reason) -> result/event
surface_exists(surface) -> bool
```

Keep `create` and destruction acknowledgements explicit. A fake adapter must reproduce delayed open/move/destroy completion. Native runtime callbacks feed controller messages; they do not mutate selection themselves. Whether the native API is synchronous or async, avoid blocking Tauri's UI loop and test documented platform creation constraints.

A retained-surface move should not navigate, replace DOM, reconstruct the Twitch player, or clear chat drafts. A fallback recreation must be explicit and must not claim to preserve an unreadable cross-origin chat draft. Show a reload warning where unavoidable.

## Native caller identity and message routing

Do not accept the current grid container's window label as identity for all children. Resolve the native caller webview to the active `SurfaceRecord`, then match session ID and instance epoch. The payload is a claim, not identity.

Commands:

- Manager commands: bundled management webview only; validate action shape and state.
- Player hello/report: hosted or Demo wrapper surface only; exact approved origin, bounded payload, native surface association, rate limiting.
- Authentication windows: no custom app commands and no telemetry.

On runtimes where iframe calls are indistinguishable from their containing surface, assume the iframe can forge observations for that same surface. This remains safe only while telemetry cannot trigger selection, layout, file access, credential access, or privileged retry behavior. Do not treat `visible: true` as proof of viewport intersection or viewer credit.

Never grant `manage` to `workspace`, `player-*`, or all windows to resolve an IPC error. If a trusted toolbar is added to a grid, isolate it by a separately verified native webview and capability.

## Protocol compatibility

The existing `Report` has `deny_unknown_fields` and this shape:

```text
session, state, visible, volume, muted
```

Keep v1 reports exactly compatible while v1 native clients remain supported. Introduce a separate additive hello negotiation or versioned endpoint with a native-known surface binding. Example metadata:

```text
protocol_version, wrapper_build_id, supported_features
```

A v2 report may add surface/instance information only after negotiation. Treat unknown/incompatible major versions as a compatibility error. Do not repeatedly retry unsupported commands or reload the player as a workaround. Existing wrappers without negotiation must be either explicitly supported as legacy or rejected with a clear upgrade instruction.

The server must not store user credentials. Fragment/channel/session parameters are not credentials but should still be excluded from unnecessary analytics. Keep the dedicated parent host free of unrelated scripts.

## Authentication profile contract

Profile identity is stable across grid and standalone surfaces and across login popups. It is independent of user-facing app name and channel identity. Do not silently generate a profile per viewer. Detect the existing profile before selecting a new path.

Popup handling must preserve the native opener configuration/environment/related-view relationship. Validate the popup's initial and subsequent destinations; a permitted first URL is not permission for arbitrary later navigation. Use observed official-flow evidence to define exact allowed origins and paths where feasible.

A website session reset is a separate explicit manager action. It stops viewers, closes related popups, safely clears only the app-owned viewer profile through supported APIs, preserves favorite settings, and leaves API credentials unchanged unless the user separately disconnects. Never wipe a locked profile directory or another application's browser profile.

`ViewerAccountStatus` may honestly remain unknown. Do not add an authoritative Turbo flag without a documented verification surface. Human test confirmation must be distinguished from application-observed state.

## Chat contract

One official chat iframe for each viewer's validated channel. Video API integration remains unchanged unless MV-020 produces an approved decision to use the combined embed. Chat show/hide changes layout only. Failure is localized to chat; video continues.

Default chat on. Preserve at least 400 × 300 for the video portion and reserve additional space for chat/chrome. Chat's actual signed-in UI and sending behavior are native acceptance steps, not inferred from iframe load or API OAuth.

## Layout transition sequence

1. Receive requested mode and allocate a new layout revision.
2. Take the current desired selection and reserve affected surfaces. Coalesce repeated requests.
3. Construct target container(s) without duplicating playback.
4. Reparent existing surfaces if supported. Otherwise close old surface, await destruction, then create its replacement with a new epoch.
5. Apply geometry and audio. Preserve manual pause where technically supported; otherwise report behavior rather than repeatedly issuing play.
6. Check for stop/preemption/newer revision before committing results.
7. Retire only empty old containers. Do not emit UserSkip for a move or programmatic container cleanup.
8. Persist the committed display preference. On failure, preserve a usable source when possible or show an explicit degraded/failed transition with recovery controls.

A grid-window close in the all-grid milestone issues Stop; it is not a series of manual channel skips. Manager exit closes all surfaces and login windows. Optional mixed-mode docking would need a new close-semantics decision before implementation.

## Required contract tests

Test callback order permutations: open then stop, move then priority replacement, close-grid during move, reduce cap while moving, repeated grid/standalone requests, old epoch report after recreation, failed reparent, target window creation failure, missing destruction event, manually paused playback, login popup during move, and missing monitor on restore.

Prove the existing top-K results are unchanged across both layouts. Prove no extra playback is started while an outgoing surface still reserves capacity. A temporarily excess count during a user-requested cap reduction is a draining state, not permission to open additional sessions.
