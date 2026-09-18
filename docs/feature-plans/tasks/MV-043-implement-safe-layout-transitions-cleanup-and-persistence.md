# MV-043 — Implement safe layout transitions, cleanup, and persistence

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-042  
**Independent reviewer:** `mpdv_security`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- src-tauri/src/controller.rs
- src-tauri/src/model.rs
- src-tauri/src/storage.rs
- src-tauri/src/main.rs (window events)
- Presentation transition implementation
- Transition and preference migration tests

## Focused read scope

- CONTRACTS.md transition sequence
- Grid/standalone presenter implementations
- Existing skip/capacity/Stop semantics

## Work sequence

1. Add controller-owned layout revisions and pending/committed state.
2. Implement retained-surface moves where supported and explicit close-confirm-recreate fallback where approved.
3. Preserve logical selection and desired audio; never exceed reserved capacity by opening duplicates.
4. Make Stop/exit, preemption, and cap reduction supersede moves; reject stale callbacks.
5. Differentiate grid close (Stop), standalone user close (Skip), and empty container cleanup (neither).
6. Persist layout preference and valid geometry with backward-compatible defaults. Preserve existing app/config/profile identities; restore no playback automatically.

## Acceptance criteria

- Switching both directions never accidentally skips channels or leaves duplicate playback.
- Stop during every transition stage leaves no viewer/popup orphan and no later reopen.
- Old preferences load as standalone; saved valid layout survives restart without autoplay.
- Failed movement or creation produces an explicit recoverable state.

## Required checks and evidence

- Transition permutation/property-style tests
- Close-grid, stop, preempt, lower-cap and repeated-toggle races
- Simulated lost destroy event and registry sweep
- Paused playback and recreation disclosure
- Missing monitor/off-screen restore and old settings fixtures

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No mixed dock/standalone mode unless separately approved.
- No silent removal of the background-playback uncertainty.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
