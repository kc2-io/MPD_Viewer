# Feature-plan addendum: supplied hosted player

Date: September 17, 2026. This addendum qualifies the previous MPD Viewer feature plan; it does not claim any planned feature has been implemented. Importing source is complete locally; actual repository integration and website deployment are distinct actions.

## New evidence

The supplied host already implements a CSS grid containing several official Twitch players. It uses a different protocol from the uploaded native POC. Therefore, **protocol alignment comes before chat and production layout integration**. Do not blindly replace the deployed page with the earlier single-player wrapper, and do not assume native child-webview reparenting is the only feasible grid implementation.

## Updated work order and agent scopes

| Task | Agent / reasoning | Bounded deliverable | Exit criterion |
|---|---|---|---|
| MV-000 baseline | Coordinator / high | Compare current checkout with imported snapshot, uploaded POC and server deployment source | Record which wrapper and native adapter actually run; no assumptions from filenames alone |
| MV-031 protocol, advanced ahead of feature integration | Architect + native engineer / high | One versioned bootstrap/commands/telemetry contract and compatibility fixtures | Define query versus fragment, 0–1 versus percent, mute, session generation, sender identity, failure reporting and negotiated capabilities |
| MV-040 layout experiment, broadened | Native engineer / high | Compare one host document with N players against one native webview per player in a grid container | Choose using actual playback, chat, login, resize, IPC, crash-isolation and transition evidence |
| Host behavior correction | Web engineer / medium–high | Working-copy fixes for focus/playback separation, reorder, layout count, finite input validation and schema bounds | All selected players follow the explicit playback policy; manual pause survives audio changes; retention does not reload a player |
| MV-020 / MV-021 auth investigation | Native engineer / high | Official website-session and popup test through the selected viewer profile | Account sharing/persistence observed independently of monitoring OAuth; no claims about Turbo without evidence |
| MV-030 chat | Web engineer / medium–high | Official chat associated with each player, compatible video/chat geometry | Correct channel, video minimum area, independent collapse and no video recreation |
| MV-050 security + MV-051 acceptance | Independent reviewer and QA / high | Negative IPC/schema tests and native runtime evidence | Hosted page cannot change favorites, launch arbitrary resources or impersonate an unrelated assignment; actual cleanup verified |

Use the established agent profiles only after the coordinator checks that their model identifiers and reasoning settings are available in the actual execution environment. Do not repeat broad repository analysis in each task. Assign exclusive ownership of shared native lifecycle/protocol files.

## Architecture decision to resolve, not assume

**Candidate A: one webview hosting a DOM grid.** Build on this supplied page. Standalone mode loads the same host with one assignment per window. Switching between grid and standalone may deliberately recreate players; close old playback first, reserve capacity correctly, preserve logical assignments and disclose the reload. A Twitch iframe/player instance must not be presumed movable between independent webview documents without recreation.

**Candidate B: one webview per player, mounted in a native grid container.** Keep the earlier reparenting experiment where preserving a renderer is an important requirement and the runtime supports it. This adds lifecycle and platform complexity and needs real native evidence.

The comparison must include resources, shared session recognition, crash scope, chat controls, minimum-size behavior and whether seamless moves are actually a requirement. The existing CSS grid is a useful starting point, not proof that a secure native grid feature is complete.

## Protocol and behavior contract

Rust remains the selection authority. The hosted document renders native assignments; it does not decide favorite priority or exceed the selected count. For a multi-player webview, maintain a native mapping from host instance to its permitted session IDs and generations. Individual channel strings in web messages are not authority. Bound telemetry to that mapping and reject stale or foreign sessions.

Separate **keyboard/visual focus**, **audio policy**, and **playback intent**. The previously agreed default is that assigned streams remain eligible to play, with configured per-player global volume and a separate mute value; changing focus must not pause the others. Focus-only audio or pause-inactive behavior can be explicit later options, not undocumented defaults. Do not silently resume user-paused or autoplay-blocked streams.

For a hosted document that has native telemetry capability, allow only the required native command on explicitly allowed origins and surfaces. Either use a deliberate native-only bridge or validate browser message origin and source; do not expose a general all-origin command listener. Remove unnecessary analytics from the working production wrapper in a separate reviewed change, while preserving the import snapshot.

Negotiate protocol support and keep immutable wrapper versions available for supported old clients. The current snapshot must not be assumed compatible with the uploaded POC. Demo stays independent of real Twitch media requests.

## Stop conditions and evidence

Native API mismatch, unsupported browser session behavior or missing tools must produce a specific recorded blocker, not an unbounded retry loop or a claimed pass. Mock tests characterize messages and decisions; native window/profile behavior and Twitch playback require separate observations. No website change, deployment or remote push is authorized merely by this addendum.
