# MV-051 — Run regression and real-runtime acceptance matrix

**Status:** Planned, not executed.  
**Agent:** `mpdv_qa`  
**Model / reasoning:** `gpt-5.6-terra` / `high`  
**Dependencies:** MV-050  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- tests/ (targeted new and updated tests)
- docs/mpd-viewer-v0.2/reports/acceptance.md
- docs/MANUAL-TESTS.md and redacted evidence manifests

## Focused read scope

- All feature acceptance criteria
- Existing core/configuration/browser suites
- Recorded OS/runtime/wrapper compatibility matrix

## Work sequence

1. Rerun baseline and focused suites after integration; record actual command output, not inherited counts.
2. Run real native chat/login/profile/Turbo comparisons with user-performed account interactions.
3. Exercise 1,2,3,4,6 selected viewers across modes, paused/blocked playback, cap changes and selection swaps.
4. Exercise sleep/wake, network loss, minimize/restore, failed moves, stop/quit, shared profile and reset.
5. Verify upgrade preferences/branding and real CSP/ACL enforcement.
6. Record support per OS/runtime. Any unavailable target remains blocked/unverified; it does not inherit another platform's pass.

## Acceptance criteria

- Each required gate has an actual passed/failed/blocked result and evidence location.
- No critical regression or privilege issue remains.
- Ad observations and authentication claims reflect native evidence, not model inference.

## Required checks and evidence

- Full native/Rust suite and targeted Node/Python/browser suites
- Manual native Windows/macOS/Linux matrix
- Old/new wrapper-client compatibility tests
- Upgrade and shutdown cleanup checks

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No production-code fixes without a new file lease and owning-task review.
- No treating mock no-sandbox/CSP-stripped harness behavior as native proof.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
