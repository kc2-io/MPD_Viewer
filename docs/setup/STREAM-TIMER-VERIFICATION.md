# Per-channel assignment timers — implementation and verification

Each favorite defaults to **Always**. The row offers a quick **10 min** button and
an explicit **Save** for any whole duration from 1 through 1,440 minutes. Settings
schema 1 remains unchanged; older favorites load without timers. Timers are
assignment time, not a Twitch watch-time/points/streak measurement.

A timer starts after opening succeeds. Loading, blocked and paused video count;
Pause automation freezes all budgets. Stop/restart starts fresh turns. Retry keeps
its existing budget. Controller intervals longer than five seconds are treated as
suspension and omitted rather than charging machine-sleep time in one jump.

An expired channel stays open until a fresh eligible alternative is waiting.
Rotation scans forward and wraps in favorite order. Closing viewers reserve their
slots until the exact native window destruction or registry sweep is observed.
Multiple simultaneous expirations rotate only genuinely waiting channels; yielded
channels cannot immediately bounce back before an admitted timed turn expires.
Oldest deadlines go first, with favorite rank resolving equal deadlines.

Normal higher-priority preemption still applies. Manual Skip/closing remains a
separate broadcast-scoped action with Undo. Timer expiry never creates SKIPPED.
A failed open is excluded until explicit Retry, other eligible targets are tried,
and the expired source is recovered if no alternative can open. Close failures
are visible and retry no more often than once per 30 seconds.

## Automated evidence (Windows, September 24, 2026)

- `cargo test -p mpd-core`: 39 passed. Deterministic clock/policy coverage includes
  all-Always subset/cap equivalence, one-channel wait, one-slot wrap, three-channel
  two-slot and five-channel three-slot waves, fresh/stale candidates, unlimited
  destination, failed candidates, recovery source, pause/sleep, retry, new
  broadcast, timer edits, reordering, changed eligible set and cap increase.
- `cargo test -p mpd-tabber --features custom-protocol`: 23 passed, including old
  settings migration, duration round trip, invalid-save preservation and malformed
  action rejection. Windows native compilation is established; this is not a
  running-WebView2 timer observation.
- The existing six Node suites plus `tests/timer.test.mjs`: 92 passed.
- `node tests/timer.browser.cjs`: 4 passed in headless Edge with a mocked native
  bridge. Covers exact Save/10-minute/Always actions, focus during polling, failed
  save display, invalid values, narrow layouts and drag-row stability while the
  player countdown updates.
- `python tests/configuration_test.py`: 17 passed.
- `python -m unittest discover -s tests -p release_test.py`: 51 passed.
- `cargo fmt --check`: fails on pre-existing compact Rust formatting beginning in
  `crates/core/src/lib.rs`; no whole-repository reformat was performed. The new
  `timer.rs` was formatted with rustfmt.
- `bash scripts/ci-check.sh`: version check passed, then this host's Bash could not
  find `node`. Its Node, Python configuration and release checks were run directly
  with the commands above instead.

## Native acceptance still required

Run one-minute Demo timers through overdue wait, live-arrival replacement, wrap,
multiple-slot waves, automation pause/resume, Stop while closing, manual close,
Retry and restart. Also cross one real Twitch assignment deadline. Observe native
capacity and Destroyed/registry behavior, including sleep/resume, separately from
these deterministic tests. There is no production sub-minute setting or timer
acceleration flag. No native elapsed-time/sleep evidence is claimed here.

No player wrapper, hosted source, authentication, app/profile identity, native
capability, release configuration, or deployment is changed by this timer patch.
Independent scheduling/lifecycle review is required before acceptance.

## Pending-target review correction

A native close can outlast a live-status poll. Every selection now revalidates the
reserved replacement using fresh-open eligibility, retargets to the next fresh
candidate, or recovers the still-fresh overdue source. The actual next open is
pinned to that reservation. Successfully bypassed targets are deferred for the
new turn, so a first-miss channel returning on the next poll cannot preempt its
replacement. If all alternatives become stale, cancellation leaves no permanent
bypass deferral. Four deterministic regressions cover these transitions and the
actual-open/pending-target identity. Native lifecycle evidence remains pending.

## Combined Windows native observation

The `cc60b37` unsigned build passed a one-minute real-channel rotation at capacity
one and a paused timer remaining at 1:00 across repeated observations. Both test
timers were restored to Always and capacity to three. Native retry/failure/sleep
scenarios above remain unobserved; their deterministic tests are separate evidence.
