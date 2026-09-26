# Player status placement and live-status rescan plan

Status: finalized after independent read-only review

Base: `87458ac` (`origin/main` on 2026-09-25)

Implementation branch: `am/MDP-Viewer-UI-tweaks`

## Objective

Keep the useful player-session status visible without requiring users to scroll,
and let users choose how often MPD Viewer rescans Twitch live status. A fresh or
upgraded install without this preference uses a one-minute rescan interval.

“Rescan” in this task means the controller-owned Twitch/Helix live-status poll.
It does not change player telemetry heartbeats, assignment timers, authorization
retries, or the manual **Check now** action.

## Current-state constraints

- Rust remains authoritative for polling, selection, validation, and persisted
  settings. JavaScript only presents state and dispatches a bounded action.
- Preserve the application identifier, preferences path/schema compatibility,
  browser profile, Twitch client ID, favorites, session limit, audio semantics,
  assignment timers, and manual pause behavior.
- `ui/app.js` and shared controller/model files are coordinator-serialized.
- Open PR #28 (`am/MPDViewer-tweaks`, commit `aa1e441`) contains newer UI cleanup
  based on alpha.6, while this task starts on alpha.7. Preserve that work rather
  than recreating removed status/arrow UI. Cherry-pick that commit onto this
  alpha.7 branch without rewriting or force-updating its branch, then make this
  task's serialized UI edits. The new PR to `main` is the combined successor;
  after verifying the prior diff is present, close PR #28 as superseded with a
  link to the new PR so two overlapping changes are not left open.
- The hosted-source snapshot and player wrapper are out of scope. No site deploy,
  release, tag, or Twitch-registration change is authorized.

## User-visible design

1. Move the complete **Player sessions** panel above the three-column session
   summary and the priority/settings area, directly after the global error alert.
   Keep the existing cards, empty state, actions, and playback disclaimer intact.
2. Add **Live-status rescan interval (minutes)** to Session settings as a numeric
   field with a whole-minute range of 1–60 and a default of `1`.
3. Describe that the setting controls automatic Twitch checks and that **Check
   now** remains available. Demo-mode controls remain immediate and do not issue
   Twitch requests.
4. Save the field on the native `change` event (blur or Enter), matching the
   existing session-limit interaction. While the field is focused, periodic
   snapshots do not overwrite a partial edit. Client validation keeps an invalid
   value available for correction, exposes the error through the existing alert,
   and never dispatches it to Rust. Associate the field with explanatory hint
   text using `aria-describedby`.
5. Render the live-monitor summary from the saved interval (for example,
   `Every 1 min` or `Every 10 min`) instead of the fixed `Every 30s` text only
   while Twitch monitoring is running normally. Preserve the more important
   state labels `Not running`, `Demo source`, `Checking…`, and `Selection paused`;
   the settings field remains the authoritative visible interval in those states.
6. Keep the layout responsive at desktop and narrow widths; the moved status
   panel must not introduce horizontal scrolling.

## Controller and persistence design

- Add `rescan_minutes: u32` to `Settings`, using serde defaults so existing schema
  1 JSON without the field loads as `1`. Keep schema 1 because this is an
  additive backward-compatible preference.
- Validate `rescan_minutes` in the inclusive range 1–60. Add a
  deny-unknown-fields `SetRescan { minutes: u32 }` action and persist it before
  updating the active schedule.
- After a successful monitored poll, schedule the next one at
  `rescan_minutes * 60` seconds. Keep the existing unmonitored authorization
  maintenance cadence independent.
- When the interval changes during monitoring, calculate the next due time from
  the last successful check plus the new interval, clamped to now and to the
  server/API `not_before` backoff. This makes a shorter interval take effect
  promptly and a longer interval avoid an unintended immediate poll.
- Make the stale-age threshold interval-aware so a deliberate interval longer
  than one minute does not become stale before its next scheduled scan. Use
  `max(90 seconds, 3 * configured interval)`, preserving the current three-cycle
  tolerance at the old 30-second cadence. Apply the same helper both when Start
  assesses an earlier check and in the one-second stale tick. A failed poll still
  marks data stale immediately and applies exponential/server-requested backoff.
- A setting change never cancels an in-flight poll. If no poll is in flight,
  reschedule from the last successful check as described above. If one is in
  flight, let it finish; successful completion schedules exactly one next poll
  using the latest persisted interval, while failure schedules the existing
  backoff. No `last_check` means the change is due now, still clamped by
  `not_before`. Paused mode continues live-status polling as it does today;
  stopped mode keeps its independent authorization-maintenance cadence.
- **Check now** requests an immediate poll subject to the existing `not_before`
  rate-limit boundary and does not rewrite the preference.

## File ownership and subagent execution

At most one production-code writer is used for this compact, shared-file change.

| Owner | Model / reasoning | Writable lease | Deliverable |
|---|---|---|---|
| Primary coordinator | `gpt-6-sol`, high | `docs/feature-plans/UI-SESSION-STATUS-RESCAN-PLAN.md`, `src-tauri/src/model.rs`, `src-tauri/src/controller.rs`, `src-tauri/src/storage.rs`, `ui/index.html`, `ui/app.js`, `ui/style.css`, relevant tests, `docs/ui-preview.png` | Integrate preserved UI work, implement, test, capture screenshot, commit/push, open PR |
| Plan reviewer subagent | `gpt-6-luna`, medium, read-only | none | Challenge scope, migration, bounds, scheduling/backoff semantics, acceptance tests, and PR strategy; return findings to coordinator |
| Implementation reviewer subagent | `gpt-6-sol`, high, read-only | none | Independently inspect the final diff for correctness, regressions, accessibility, persistence, scheduling, and test gaps |

If an assigned model is unavailable, use the nearest available lower-cost model
for plan review and retain high reasoning for the independent code review. No
subagent edits coordinator-serialized files.

## Verification

1. Rust unit tests:
   - default and legacy settings load `rescan_minutes == 1`;
   - valid values round-trip; 0 and values above 60 are rejected without
     overwriting the last good settings;
   - malformed/unknown action payloads are rejected;
   - interval-to-duration, rescheduling, stale threshold, and backoff clamping
     cover 1/60-minute bounds, exact due time, shorter/longer changes, no previous
     check, `not_before` later than due, in-flight completion, and immediate
     failure staleness without real-time sleeps.
2. Browser UI tests:
   - Player sessions precedes summary/priority content in DOM and rendered Y
     position;
   - the labeled/described input renders authoritative state, dispatches
     `set_rescan` on blur/Enter, preserves partial focused edits across ordinary
     refreshes, and rejects invalid values without dispatch;
   - live-monitor text uses correct singular/plural interval labels;
   - desktop and the narrowest supported viewport have measured no horizontal
     overflow, and the field remains keyboard-operable.
3. Native/E2E coverage:
   - save a non-default interval through the real UI/controller, restart, and
     verify it is restored from the native store without opening viewers (a page
     reload alone is not restart evidence);
   - run the smallest available native build/E2E smoke that exercises settings.
4. Regression checks: run the existing Rust workspace tests, source checks,
   JavaScript tests, browser suites affected by `ui/app.js`, and formatting/lint
   commands available in the repository. Record native versus mocked evidence
   separately.
5. Regenerate `docs/ui-preview.png` through the browser preview harness, inspect
   it for the new top placement and rescan field, commit it, and embed the image
   in the pull-request body using its branch-hosted raw URL.

## Acceptance criteria

- On initial render, player session cards or their empty state appear above the
  summary and main priority/settings content, without scrolling to the old lower
  location and without duplicated IDs or panels.
- Fresh and upgraded preferences default to a one-minute rescan interval.
- A user can save any whole interval from 1 through 60 minutes; invalid values
  cannot reach or overwrite Rust settings.
- The controller uses the saved value for monitored Twitch polls, while manual
  checks, API backoff, auth maintenance, demo behavior, and player telemetry keep
  their existing boundaries.
- The saved value survives an application restart and the summary accurately
  displays it.
- Relevant automated checks pass, the screenshot is visually inspected, and an
  independent reviewer has no unresolved blocking finding.
- The branch is pushed and a PR to `main` is open with test evidence, the
  screenshot, limitations, and the relationship to PR #28 documented. No merge,
  deployment, tag, or release is performed.
