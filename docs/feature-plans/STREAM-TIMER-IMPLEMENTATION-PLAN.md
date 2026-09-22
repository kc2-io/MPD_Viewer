# MPD Viewer — Per-channel stream timer implementation plan

**Audience:** Claude implementation agent and independent reviewer<br>
**Prepared:** September 21, 2026<br>
**Verified planning baseline:** `b167bdc` (`main`, matching `origin/main`, clean when inspected)<br>
**Status:** Plan only. No application behavior, release, hosted site, or wrapper was changed.

## 0. Claude execution brief

Implement this as one bounded feature task in a dedicated worktree. Before writing code, read `AGENTS.md`, `CODEX-START-HERE.md`, `docs/HOSTED-SOURCE-PLAN-ADDENDUM.md`, and `docs/feature-plans/AGENT-RULES.md`; then inspect the actual branch, HEAD, worktree status, and recent timer-related changes. The baseline above records what this plan reviewed, not permission to reset or overwrite a newer checkout. The coordinator must provide the implementation base commit, worktree, and exclusive lease listed in section 6.

Return a small reviewable patch. Do not delegate shared-file edits, deploy, publish, tag, or push unless a separate instruction explicitly authorizes it.

## 1. Outcome

Add an optional timer to every favorite channel. A favorite with no timer keeps the existing unlimited behavior. A favorite with a timer may hold an assigned viewer for that many minutes, then yields to the next eligible live favorite so watch time can be spread across more live favorites than the configured viewer limit.

The Rust controller remains the sole authority for assignment, timer expiry, and rotation. The manager UI only edits the saved duration and presents controller state. Player telemetry remains advisory and must not be able to extend, expire, or reset a timer.

This task does not change the Twitch player wrapper, the hosted-source provenance snapshot, the public Twitch client ID, browser profile identity, app identifier, preference path, viewer limit, or release/deployment configuration.

## 2. Product contract to implement

Use these decisions unless the owner answers the open questions in section 12 differently before coding.

### Saved setting

- Add `watch_minutes: Option<u32>` to each saved favorite.
- `None` means **Always / no timer**. It means “this assignment does not yield because of a timer,” not “ignore the favorite order or viewer limit.” Normal offline handling, disabling, manual skip, cap reduction, and higher-priority preemption still apply.
- Accept whole minutes from 1 through 1,440. Reject zero, negative, fractional, non-finite, and out-of-range input in Rust even if the UI also validates it.
- New and existing favorites default to `None` so an upgrade has exactly the old scheduling behavior until the user opts in.
- Keep settings schema 1 if the field is backward-compatible via a field-level serde default. Do not discard or reset an old settings row merely because it lacks the new field.

### What consumes time

- Measure **managed assignment time**, not claimed Twitch watch time. Start the budget after `Host::open` succeeds for the logical assignment.
- Count loading, buffering, autoplay-blocked, and user-paused player states. The app cannot securely or reliably infer credited watch time from remote playback telemetry.
- Count only while automation mode is `Running` and the logical session is assigned and not closing. `Pause` freezes remaining time because it pauses automatic selection. Resume continues the remaining budget.
- Use monotonic duration accounting, never the wall clock. Detect a long controller-tick gap consistent with machine sleep and do not consume the whole suspended interval in one jump.
- A renderer retry or later layout recreation must not grant a fresh budget to the same logical assignment. Timer state belongs to the controller assignment/rotation turn, not a webview instance.

### Expiry and rotation

- On expiry, scan forward in the current favorite order, wrapping at the end.
- Skip candidates that are offline or not fresh enough to open, disabled, manually skipped for the current broadcast, already assigned or closing, pending as another replacement, or in an open-failure backoff state.
- If a genuine waiting candidate exists, programmatically close the expired session, retain its capacity reservation until destruction is acknowledged, then open the chosen candidate. Never exceed the viewer limit by opening the replacement early.
- If no waiting candidate exists, keep the expired stream open. Show it as **Time reached · waiting for another live channel**. Do not close/reopen it, repeatedly increment rounds, or reset its duration. Re-evaluate when eligibility, order, limit, failure state, or favorite settings change.
- A waiting candidate becoming live later should cause an overdue assignment to yield after the successful poll is applied; it should not wait for another full timer.
- Rotation wraps. With one slot and two timed live channels A then B, the sequence is A, B, A, B, with each receiving its configured duration.
- An unlimited channel reached by rotation does not timer-yield. For example, with one slot and live A (10 minutes), B (20 minutes), C (Always), the expected sequence is A, B, C until normal non-timer selection rules remove or preempt C.

### Multiple viewer slots

Treat expiration as a bounded rotation round, not a set of independent manual skips.

- Only replace an expired session when doing so admits a live favorite that is genuinely waiting outside the current assigned/reserved set.
- Serialize close-confirm-open transitions through the controller. A closing session and a pending replacement both reserve one slot.
- If several timers expire together but fewer channels are waiting than expired sessions, rotate only enough sessions to admit the waiting channels. Leave the other expired sessions open in the waiting state.
- Do not immediately reopen a channel displaced earlier in the same rotation wave merely to satisfy a second simultaneous expiry. Advance/wrap the round only after a newly admitted timed channel has received its turn or the eligible set materially changes.
- Break equal-deadline decisions deterministically by current favorite rank.
- Preserve the existing strict priority result when every favorite is unlimited. Timer-aware selection is allowed to defer an expired higher-ranked favorite; that temporary deferral is the feature, not a change to saved rank.

A conforming three-channel/two-slot example is:

```text
All live and timed: A, B, C; viewer limit 2
Initial:           A + B
First rotation:    B + C  (A yielded; do not bounce A straight back)
Next rotation:     A + C
Next rotation:     A + B
```

The exact internal representation may be a rotation generation plus yielded/pending sets, or an equivalently tested state machine. Do not reuse the existing `skipped` map: manual skip must remain visible, undoable, and scoped to user intent, while timer deferral has different reset rules.

### Reset and mutation rules

- `Stop`, process restart, and a fresh `Start` reset transient countdowns, overdue flags, pending replacements, and rotation rounds. They do not alter saved durations. Start again from normal priority order and full budgets.
- A new Twitch broadcast ID for a channel starts a fresh timer turn. A transient stale poll or the first missing poll does not.
- Removing or disabling a favorite clears its transient timer state. Re-enabling starts fresh if selected again.
- Editing a channel's timer resets only that channel's current budget from the moment the save succeeds. Changing it to Always removes its deadline. A failed save changes neither settings nor runtime state.
- Reordering clears timer deferrals/round bookkeeping so the new order controls the next choice, but preserves elapsed budget for an active session that remains assigned.
- Increasing the viewer limit fills newly available capacity without resetting existing timers. Decreasing it uses deterministic selection and retains closing capacity reservations.
- Manual Skip remains broadcast-scoped, takes precedence over timer rotation, and is the only path that produces the existing `SKIPPED` state and Undo control.
- A user closing a viewer window keeps the current manual-skip behavior. A timer-driven close must never be mistaken for a manual close.
- Retry preserves remaining time for the same logical turn. It must not become a way to obtain unlimited time by recreating a renderer.

## 3. Architecture and data ownership

### Persisted model — `src-tauri/src/model.rs`, `src-tauri/src/storage.rs`

1. Extend `Favorite` with the optional duration and a field-level default.
2. Add a narrowly shaped manager action such as:

   ```rust
   SetTimer { login: String, minutes: Option<u32> }
   ```

3. Validate channel existence and the 1–1,440 bound in Rust. Preserve `Action`'s `deny_unknown_fields` behavior.
4. Add old-JSON and new-JSON storage tests. Verify an old row loads as Always and saving it retains all other settings.
5. Do not persist `Instant`, remaining seconds, rotation rounds, overdue state, or pending transitions.

### Pure scheduling policy — `crates/core/src/lib.rs`

Move the timer/rotation decision rules into pure, clock-free policy code. Pass elapsed durations and explicit state into it so tests never sleep.

Keep these concerns distinct:

- base eligibility from enabled state, presence freshness, broadcast identity, manual skip, and viewer limit;
- an active turn's configured budget and accumulated running duration;
- timer deferral for the current rotation round;
- pending close/replacement reservations;
- open failures/backoff supplied by the controller.

The existing `select` behavior must remain a regression oracle for an all-Always list. Either extend it behind an explicit timer-aware input or add a new scheduler and prove identical results when no durations are set. Do not let viewer count or player telemetry enter the policy.

### Controller runtime — `src-tauri/src/controller.rs`

Add controller-owned runtime state keyed by normalized login plus current broadcast/turn identity. Suggested concepts, not mandatory names:

```text
TimerTurn      = configured budget, consumed duration, last running tick, overdue
RotationRound  = generation, yielded channels, admitted channels
Replacement    = source session, target login/broadcast, transition phase
CloseReason    = manual/window, stop, offline, preemption, timer rotation, retry
```

Important integration details:

- Advance clocks on the existing bounded controller tick, before reconciliation.
- Stop accumulating as soon as a session begins closing.
- Record the timer-driven close reason explicitly or preserve an equally strong invariant; relying on a generic boolean must not turn a destroy callback into `skip()`.
- Continue reserving capacity until `Destroyed` or the existing registry sweep confirms the surface is gone.
- Reject stale poll/auth/window callbacks exactly as today. A late destroy from an old session must not clear a new channel's timer state.
- Include pending targets in duplicate/capacity checks.
- Exclude failed targets before truncating candidate selection. If the first replacement cannot open, try the next eligible waiting favorite. If none can open, recover the expired source when possible and leave it overdue; do not leave a slot empty forever or retry every second.
- Bound failed-close/open retries with state/backoff and a visible error. Do not create a one-second destroy/open storm.
- Keep demo mode behaviorally equivalent and fast to test. Tests may use seconds or injected durations internally; the saved user setting remains whole minutes.

### Read model and manager UI — `src-tauri/src/model.rs`, `ui/`

Expose presentation state without exposing timestamps. A tagged status is preferable to several contradictory booleans, for example:

```text
Unlimited
Counting { remaining_seconds }
AutomationPaused { remaining_seconds }
WaitingForAlternative
ClosingForRotation
```

Implement an accessible per-row editor:

- A blank value or explicit **Always** choice means no timer.
- A whole-number minutes input permits 1–1,440 and has an accessible label containing the channel login.
- Save on an explicit action or committed change, not on every keystroke.
- Do not optimistically display the new authoritative value before Rust accepts and publishes it.
- Show configured duration in the favorite row and live countdown/status on the corresponding player card.
- Add the timer fields to render keys so countdowns update, without breaking the existing drag operation's DOM-stability guarantees. Do not replace a row under an active drag or in-flight rank save.
- Present “assigned time,” not “credited watch time.” Keep `PLAYBACK ≠ VIEWER CREDIT` messaging intact.
- Ensure narrow layouts wrap the timer control without making enable, rank, retry, skip, and remove controls unreachable.

Put display formatting (for example `12:34 left`, `Paused · 12:34 left`, and overdue text) in `ui/view-model.js` and unit-test boundary values there.

## 4. Required behavior matrix

The implementation is incomplete until these cases have deterministic automated coverage.

| Case | Required result |
|---|---|
| All favorites are Always | Exact existing top-K selection and preemption behavior. |
| One timed live favorite, one slot | It remains open at zero; no close/open churn. |
| Timed A and B, one slot | A yields to B, then B wraps to A. |
| Timed A; B offline at expiry | A stays overdue; B does not open. |
| B becomes live after A is overdue | A yields promptly after the fresh successful observation. |
| A timed, B Always, one slot | A yields to B; B has no timer-driven yield. |
| Three timed channels, two slots | Every channel receives turns; simultaneous expiry does not bounce a just-displaced channel or exceed two reservations. |
| More slots than eligible live channels | Expired channels stay; no artificial cycling. |
| Higher-priority channel becomes live | Existing priority preemption still occurs unless that broadcast is deliberately deferred by the active timer round. |
| First missing poll/stale API data | Existing player is retained and its assignment timer continues. |
| Confirmed offline/new broadcast | Old turn is discarded; a new broadcast gets a full timer. |
| Pause for longer than remaining time | No rotation while paused; resume continues the frozen remainder. |
| Stop during timer close | No replacement opens; all players drain and transient timer state resets. |
| Limit decreases during rotation | No excess replacement opens; closing sessions continue to reserve capacity. |
| Limit increases while channels are overdue | New free slots fill first; close/reopen is avoided. |
| Reorder while overdue | The newly saved order determines the next replacement without resetting retained elapsed time. |
| Timer edited while active | Successful edit starts the new full budget; failed edit changes nothing. |
| Manual close/Skip | Existing broadcast-scoped manual skip and Undo behavior; no timer deferral label. |
| Retry/layout recreation | Same logical turn retains its remaining budget. |
| Replacement open fails | Try a bounded next candidate or recover source; no empty-slot deadlock or retry storm. |
| Destroy event is lost | Registry sweep completes the reservation exactly once; no duplicate replacement. |
| Late callback from old session | Cannot reset, consume, or close the current turn. |
| Machine sleep/large tick gap | Suspended time is not charged as one large active interval. |
| Duration bounds/overflow | Rust rejects invalid values and performs checked conversion to `Duration`. |

## 5. Implementation sequence

Make small commits in this order so failures are attributable.

1. **Model and migration:** optional field, action validation, storage fixtures. No scheduler change yet.
2. **Pure timer scheduler:** state transitions and table-driven tests, including multiple-slot rotation rounds. Prove all-Always equivalence.
3. **Controller integration:** injected/passed elapsed time, close reasons, reservations, failure recovery, events, and read model.
4. **Manager UI:** row editor, player countdown, accessible labels, narrow layout, and DOM/mock tests.
5. **Regression and native exercise:** full automated checks, Demo-mode accelerated scenario, then real native timing/lifecycle checks.
6. **Independent review:** scheduling/lifecycle reviewer inspects races, persistence compatibility, and manual-skip separation before merge.

Do not combine this work with grid, chat, hosted protocol, OAuth, branding, dependency upgrades, or release changes.

## 6. Writable-file lease

Give the implementation agent exclusive ownership of these files for the duration of the task:

- `crates/core/src/lib.rs`
- `src-tauri/src/model.rs`
- `src-tauri/src/controller.rs`
- `src-tauri/src/storage.rs`
- `ui/app.js`
- `ui/view-model.js`
- `ui/index.html`
- `ui/style.css`
- timer-specific tests/fixtures under `tests/`
- timer-specific user documentation in `README.md` and `docs/MANUAL-TESTS.md`

`controller.rs`, `model.rs`, `storage.rs`, and `ui/app.js` are shared hot spots. No other production writer may edit them concurrently. Expand the lease only after reporting why. Do not edit `player-wrapper/`, `web/parent.mpdviewer.com/`, Tauri capabilities, signing workflows, or release files for this feature.

## 7. Automated checks

At minimum run and report exact outcomes for:

```bash
cargo fmt --check
cargo test -p mpd-core
cargo test -p mpd-tabber
node --test tests/view-model.test.mjs
python3 tests/configuration_test.py
bash scripts/ci-check.sh
```

Add focused Rust tests with a fake clock or explicit elapsed-duration input; never make the suite wait real minutes. Add manager DOM coverage to the existing mocked browser harness or a timer-specific Playwright test. Label those as mocked UI evidence, not native timer evidence.

If the host lacks native Tauri libraries, report `cargo test -p mpd-tabber` as blocked with the exact missing prerequisite. Do not weaken the test or count `mpd-core` as native validation.

## 8. Native/manual acceptance

Use Demo mode with an internal/test-only accelerated duration mechanism or a debug fixture; do not add sub-minute production settings merely to speed testing.

Verify separately:

1. Start with one live timed favorite and confirm it stays open after reaching zero.
2. Bring another favorite live and confirm the overdue viewer closes once, capacity remains reserved until destruction, and the next viewer opens once.
3. Exercise one-slot wrap and the three-channel/two-slot rotation example.
4. Pause before expiry, wait, resume, and confirm the frozen remainder.
5. Stop during close and during pending replacement; confirm nothing reopens.
6. Manually close a player and confirm that only this path shows `SKIPPED`/Undo.
7. Restart the app and confirm saved durations remain but playback does not auto-start and transient countdowns do not restore.
8. Run a real Twitch-mode smoke test long enough to cross one configured deadline. Record assignment/lifecycle evidence separately from any claim about playback or viewer credit.

## 9. Logging and diagnostics

Add concise controller events for timer start, pause/resume, overdue-with-no-alternative, rotation source/target, replacement failure, and round wrap. Do not log OAuth material, cookies, URLs containing sensitive values, or high-frequency countdown ticks.

Diagnostics must say **assigned for N minutes** or **timer reached**, never “watched,” “viewer counted,” or “credit earned.”

## 10. Failure and rollback rules

- A settings validation/save failure leaves both saved and active timer configuration unchanged.
- A close failure keeps the source tracked and visible with a recoverable error.
- An open failure does not erase the source's rotation context or permanently consume a slot.
- Stop/exit always wins over timer callbacks and pending replacement work.
- Removing all configured timers restores old behavior without a database downgrade.
- The feature can be reverted without losing favorites because old settings readers should either ignore the additive field or the revert must include an explicit compatibility check. Verify this rather than assuming it.

## 11. Completion handoff

Return the repository-required handoff exactly:

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

Also include the tested timer scenarios, whether native elapsed-time behavior was observed across sleep/resume, and a statement that no hosted source or deployment was changed.

## 12. Owner questions before implementation

The plan recommends answers so the agent can proceed, but these product choices are worth confirming:

1. Does **Always** mean the recommended “no timer once selected, while normal rank/preemption still applies,” or should every live Always channel reserve a slot ahead of all timed channels regardless of rank?
2. Should the countdown use the recommended controller assignment time, or only fresh `Playing` telemetry? The latter is less reliable and lets remote content influence scheduling.
3. Is 1–1,440 whole minutes an acceptable production range, with no timer as the default?
