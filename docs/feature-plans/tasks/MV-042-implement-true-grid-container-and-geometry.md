# MV-042 — Implement true grid container and geometry

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-041  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- src-tauri/src/presentation/grid.rs (or approved equivalent)
- src-tauri/src/layout.rs (new pure geometry)
- Necessary presenter wiring and feature gates
- Grid geometry/native smoke tests

## Focused read scope

- Grid feasibility ADR
- Surface/profile/caller contracts
- Chat panel minimum dimensions
- S3,S5,S6,S9

## Work sequence

1. Create a dedicated native grid window hosting each selected surface at its own bounds.
2. Keep manager privileges out of all hosted surfaces; preserve profile and popup behavior.
3. Implement deterministic priority-ordered geometry using logical units and display scaling.
4. Account for video, chat and chrome minimums. Use the proven scroll/clipping or clear insufficient-space strategy from the ADR.
5. Test geometry in isolation and real native interaction. Do not silently hide chat, reduce capacity, or start off-screen playback tricks.

## Acceptance criteria

- Grid is one native container with per-channel video and chat surfaces.
- No video is shrunk below the required minimum to fit an arbitrary count.
- Focus, chat keyboard input, resize and native popup work as tested.

## Required checks and evidence

- Geometry tests for 1,2,3,4,6 plus overflow counts
- Different DPI/scaling and screen work areas
- Keyboard focus and chat input in neighboring cells
- Hostile neighboring-surface report test

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No global switching state machine yet.
- No OS-level tiling presented as the completed feature.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
