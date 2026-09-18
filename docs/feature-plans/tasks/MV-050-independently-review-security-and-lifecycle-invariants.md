# MV-050 — Independently review security and lifecycle invariants

**Status:** Planned, not executed.  
**Agent:** `mpdv_security`  
**Model / reasoning:** `gpt-6-astra` / `high`  
**Dependencies:** MV-010, MV-031, MV-043, MV-044  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- None: read-only reviewer returns findings; coordinator writes docs/mpd-viewer-v0.2/reports/security-review.md

## Focused read scope

- All changed native command, profile, popup, capability, wrapper and presentation paths
- Patch diffs and acceptance evidence
- S8,S9

## Work sequence

1. Trace the actual native caller from auth popup, Twitch iframe, wrapper surface, and manager to each command.
2. Attempt cross-surface spoofing, capability union escalation, hostile redirects and popup abuse.
3. Review profile preservation/reset, redaction, unknown protocol behavior, and hosted-v1 compatibility.
4. Trace Stop/preempt/cap-decrease races and programmatic destruction. Check for duplicate playback and orphan popups.
5. Return prioritized file/line findings with reproducible tests. Fixes go back to the owning implementation task, then are re-reviewed.

## Acceptance criteria

- No open critical/high privilege, credential, data-loss, or duplicate-playback issue.
- Native checks are distinguished from static review and mocks.
- Review is independent of the code-author run.

## Required checks and evidence

- Native negative IPC and popup checks on claimed platforms
- Diff-to-contract review
- Reproduction of each substantive finding or clear static proof

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No implementation patches during this review run.
- No password/cookie requests or storage-security bypass.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
