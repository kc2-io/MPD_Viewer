# MV-022 — Make API connection and viewer sign-in distinct in the manager

**Status:** Planned, not executed.  
**Agent:** `mpdv_web`  
**Model / reasoning:** `gpt-5.6-terra` / `medium`  
**Dependencies:** MV-010, MV-021  
**Independent reviewer:** `mpdv_qa`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- ui/index.html
- ui/app.js
- ui/view-model.js
- ui/style.css
- Tests for manager auth UI

## Focused read scope

- Approved native auth action/view contract
- Current connected_as and device-auth rendering
- Auth diagnosis report

## Work sequence

1. Label API state as live-status/monitoring authorization, not proof of signed-in playback.
2. Add user-triggered viewer sign-in entry and honest Unknown/user-confirmed state where no official verification exists.
3. Expose explicit viewer-session reset only with a clear confirmation and explanation of what is cleared.
4. Keep API Disconnect separate; do not display an unverified Turbo-active badge.
5. Show actionable popup/runtime/login failures without exposing secrets.

## Acceptance criteria

- A user can distinguish both authentication concepts before starting viewers.
- Viewer-session reset never silently disconnects monitoring or deletes favorites.
- No API-connected status implies ad-free playback.

## Required checks and evidence

- Manager command dispatch and permission checks
- Exact confirmation semantics for reset
- Unknown/error states and keyboard accessibility
- No credential fields or cookie contents in UI state

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No native-auth implementation in this UI task.
- No automatic Turbo detection claim.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
