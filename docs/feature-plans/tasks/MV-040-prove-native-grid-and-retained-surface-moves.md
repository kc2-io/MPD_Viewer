# MV-040 — Prove native grid and retained-surface moves

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-000  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- docs/mpd-viewer-v0.2/reports/grid-spike.md
- A disposable grid-spike worktree; experimental changes are NOT merged

## Focused read scope

- src-tauri/src/player.rs
- src-tauri/src/main.rs
- src-tauri/src/controller.rs
- src-tauri/capabilities/*
- src-tauri/Cargo.toml
- S5,S6,S8,S9

## Work sequence

1. Prototype two hosted child webviews inside a separate native grid window; retain the privileged manager separately.
2. Check tested Tauri feature/version requirements; record dependency pins and any unsupported OS constraints.
3. Test bounds, display scaling, focus, keyboard interaction, popup ownership, minimize/restore, and back-and-forth reparenting.
4. Observe whether DOM, playback, session/profile, and chat draft survive a move; never infer this only from a method returning Ok.
5. Test the native caller identity needed for telemetry and safe scrolling/clipping of child views.
6. Return a capability matrix: retained move, recreate move, grid supported/unsupported. Propose an ADR fallback instead of a broad engine rewrite.

## Acceptance criteria

- At least one real native reproduction or a specific blocked-environment report.
- No claim that the current unstable API is portable without platform evidence.
- Grid means one window, not tiled standalone windows.

## Required checks and evidence

- Native window-count and surface-count observation
- Bidirectional reparent with a paused and playing surface
- Caller identity and cross-surface report attempt
- Low-space/overflow geometry and multi-monitor cases

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No full docking framework.
- No manager permissions on remote grid children.
- No release merge of throwaway prototype.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
