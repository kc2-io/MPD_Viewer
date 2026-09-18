# MV-044 — Expose layout selection and state in the manager

**Status:** Planned, not executed.  
**Agent:** `mpdv_web`  
**Model / reasoning:** `gpt-5.6-terra` / `medium`  
**Dependencies:** MV-022, MV-043  
**Independent reviewer:** `mpdv_qa`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- ui/index.html
- ui/app.js
- ui/view-model.js
- ui/style.css
- Manager layout tests and fixtures

## Focused read scope

- Committed/pending layout action contract
- Grid close and insufficient-space behavior
- Current auth/chat UI

## Work sequence

1. Add Standalone/Grid setting with current, pending, failed and supported states.
2. Keep rank controls distinct from presentation order; focus a grid viewer through its surface.
3. Show brief reload disclosure for recreate-only platforms and explanatory space errors.
4. Reflect the committed setting only after backend success; display real failure/rollback states.
5. Keep controls keyboard-accessible and avoid granting layout authority to remote wrapper content.

## Acceptance criteria

- User can switch modes and understand in-progress/error/reload conditions.
- A failed backend change does not misleadingly show success.
- Selection count and volume semantics remain unchanged.

## Required checks and evidence

- Action dispatch and pending/committed UI tests
- Unavailable mode and insufficient-space state
- Focus, keyboard and resize usability
- Priority order vs display mode separation

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No cross-platform support claims based on UI availability alone.
- No unrelated visual redesign.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
