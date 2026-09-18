# MV-041 — Extract surface presentation while preserving standalone behavior

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-022, MV-031  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- src-tauri/src/player.rs
- src-tauri/src/presentation.rs or presentation/ (new)
- src-tauri/src/controller.rs
- src-tauri/src/main.rs
- src-tauri/src/model.rs
- Scoped presenter/lifecycle tests and caller capability adjustments

## Focused read scope

- Approved CONTRACTS.md
- Current close/destroy/reconcile/report/sweep paths
- Existing standalone behavior tests

## Work sequence

1. Split static wrapper hosting from surface creation/audio/focus/closure.
2. Introduce logical session, native surface instance, and container tracking using the frozen minimal interface.
3. Convert direct get_webview_window assumptions to adapter operations and actual surface ownership.
4. Bind reports to native surface identity and reject old epochs. Keep telemetry advisory.
5. Introduce fake adapter delayed callbacks and distinguish programmatic close reasons from manual Skip.
6. Leave all viewers standalone in this task and prove behavior equivalence before adding grid.

## Acceptance criteria

- Current priority, capacity, skip, retry, audio, auth/chat, Stop/Quit behavior remains intact.
- Late surface events cannot alter another session or resurrect a stopped one.
- No new grid UI ships in this refactor commit.

## Required checks and evidence

- Fake presenter delayed create/destroy and stale report tests
- Existing core selection tests unchanged
- Standalone real login/chat/playback smoke
- Manual close vs programmatic close semantics

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No scheduling policy changes or generalized docking library.
- No unrelated formatting churn across the repository.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
