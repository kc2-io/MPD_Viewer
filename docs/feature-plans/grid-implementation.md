# Native grid implementation plan

Date: 2026-10-01. Base: `f2dc03cc2e7ca11271c946ebf98d39f51b840495`.
Author: Astra, with independent review amendments. This is an implementation
plan, not a claim of native or live Twitch acceptance.

## Scope and competitive context

Add a genuine single-window Grid presentation alongside Standalone. Preserve
full Twitch pages as the default backend and retain explicit embedded mode.
Rust continues to select favorites, own assignments, and enforce capacity and
timers. Retained live switching is in scope; stopped-only selection is an
intermediate milestone. No framework rewrite, DOM multi-player protocol,
website deployment, arbitrary docking, or automatic playback on relaunch.

[Viewington](https://viewington.com/watch) supplies comparison questions about
adaptive multiview, ordering, offline changes, and control discoverability.
It does not prescribe MPD's framework, embed backend, layout, or selection
policy. Profiles/sharing are outside this change. Its controls do not establish
full-page controls, login, rewards, or performance.

## Implementation sequence and gates

1. Disposable native spike: use the locked Tauri 2.11.6/runtime-wry 2.11.4 and
   deliberately enable `unstable`. Prove child creation, bidirectional retained
   reparent, bounds/focus, and event-thread close completion locally. Extend
   native fixtures to document/state/profile retention and counts 1/2/3/4/6.
   Record missing Windows/macOS/live Twitch evidence; never infer portability.
2. Add a small presenter and pure geometry module. Separate logical session
   identity from surface identity and owning container. Use WindowBuilder and
   WebviewBuilder for viewers, with the same production profile defaults,
   navigation/download/popup restrictions, and embedded bootstrap. E2E children
   use the existing isolated profile. Preserve standalone behavior first.
3. Add defaulted `Settings.viewer_layout` (Standalone upgrade default), bounded
   `set_layout` action, capability/pending state, and manager layout control.
   Geometry uses actual desired selected count, logical units, backend minimums,
   deterministic priority order, and monitor work area. Never alter the limit
   or mark a channel failed merely because it cannot fit.
4. Implement retained live transitions as controller-owned transactions:
   revision/cancellation token, source and target placements, tagged completion,
   deferred reply, and successful preference commit. Create target containers
   outside synchronous UI callbacks. Execute mutations on the event thread,
   checking cancellation immediately before mutation. Keep the controller able
   to process Stop/exit, capacity reduction, and replacement. Latest layout
   request supersedes older requests; stale completions cannot resurrect media.
5. Integrate per-child closure/focus/mute, geometry recovery, close semantics,
   tests, adversarial implementation review, native fixture evidence, and a PR.

## Lifecycle and authority

Manager commands bind to the actual calling manager webview, not a shared
window. Only wrapper child labels may report their own session; full pages have
no report/manage capabilities. Reports bind to the active surface incarnation.
Retained movement preserves that identity; any future recreation needs a new
surface identity. No recreation fallback is included in this change.

Tauri worker-thread Webview::close removes its registry entry before native
destruction. Close on the event thread and emit a tagged closure completion
after synchronous native removal; preserve capacity on dispatch failure or
uncertain completion. Registry absence alone is not closure evidence.

Tauri reparent updates its window accessor before dispatcher success; runtime
failure may drop the old native handle. Maintain explicit placements. Known
pre-mutation failures preserve sources. Uncertain reparent/rollback failures
trigger Stop-all cleanup, including both source and target containers, rather
than speculative recreation. Close failures retain reservations and retry with
bounded backoff. Source container cleanup must be empty and cannot imply Skip.

Standalone user close means Skip; grid close means Stop viewers and monitoring;
manager exit closes all. Focus neither pauses neighbors nor resumes paused
media. Native child mute must be retested; macOS keeps its unsupported result.
Grid titles describe the group; standalone titles retain channel/viewer counts.

## Timer and overflow policies

Moving surfaces remain logically assigned. Running assignment time, including
transition downtime, continues; automation Pause freezes time. Account monotonic
elapsed time exactly once, preserve broadcast turn state, and do not call
rotation.opened for a presentation move. Stop/exit and reductions supersede
transitions; settle cancelled native work before ordinary selection reconciliation.

Preflight actual selected count before Start/switch; reject no-fit without
changing assignments or the committed preference. Test backend cell minimums
(initial full-page floor 430 x 480; embedded must also fit video/chat/chrome).
For later growth, resize, DPI change, or monitor loss, first enlarge/reposition
within a valid work area. If no fit is possible, stop all viewers and monitoring
with a reachable manager error. Do not silently lower capacity, hide playback,
switch backend/layout, or restart. Retain favorites, capacity, and layout
preference. Standard Stop semantics apply to timer state. Failed closes remain
reserved. This bounded policy is explicitly part of the proposed implementation;
arbitrary-count scrolling requires separate proof.

## Validation and delivery

Unit/fake tests cover settings migration, geometry/no-fit, retained identity,
unchanged selection and turn time, target creation/partial move/rollback failure,
Stop/capacity reduction/repeated toggles/stale callbacks, close-failure capacity,
close semantics, and no-fit recovery.

Native fixture tests establish one grid container and N children, actual bounds,
retained document state/profile, focus, resize, IPC denial, confirmed close before
replacement, and no orphan surfaces. Live Twitch pause/chat/profile/audio
continuity remains separate. Sanitize revision-bound screenshot/video evidence.

Run `just verify`, `just verify-native`, `just verify-e2e`; `just verify-core` if
core changes. Build/run isolated native E2E and monitor hosted checks to terminal
outcomes. Obtain independent adversarial review of the actual diff and re-review
blocking corrections. Commit task files, push the assigned branch, open a draft
PR if any required platform/live gate is unavailable. No merge/deploy/release.

## Execution record

The locked-runtime Linux spike proved native child creation, retained movement in
both directions, and event-thread close. Production now uses separate container
and child identities, presentation transactions, defaulted layout preferences,
manager controls, and controller-owned Stop/cleanup. Embedded cells use 800 x 540;
full-page cells currently use the proposed 430 x 480 floor. Linux needs a native
GtkFixed adapter because the locked Wry GtkBox host ignores child coordinates.
This does not manipulate production page DOM.

Independent review identified and corrected preparation mutation, orphaned partial
opens, uncertain move cleanup, late supersession, Stop dominance, and cleanup retry
intent. Final re-review and changed-revision checks are recorded in the PR.
Native local-fixture evidence covers two retained surfaces where they fit, state/turn retention,
IPC isolation, cancellation, injected failure cleanup, and no-fit Stop. Geometry
unit tests additionally cover counts 1/2/3/4/6/12; this is not native acceptance
for all those counts.

Hosted Linux exposed off-thread monitor conversion in the locked runtime: the
complete monitor observation now runs on the native event thread. Hosted small
macOS desktops correctly reject two embedded cells; fixture evidence explicitly
checks that rejection and reports one retained embedded child there, while the
full-page probe still requires two. This is not two-child embedded acceptance
on a small desktop. Exact Linux Openbox and small-display fixtures verify these
corrections.

Live Twitch pause/chat/profile/audio continuity, full-page sizing, DPI/monitor-loss
acceptance, and platform-specific native results remain distinct gates. Keep the
PR draft until the required evidence is available. No release or deployment is
part of this task.
