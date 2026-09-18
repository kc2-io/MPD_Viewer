# MV-001 — Freeze minimal authentication and presentation contracts

**Status:** Planned, not executed.  
**Agent:** `mpdv_architect`  
**Model / reasoning:** `gpt-6-astra` / `high`  
**Dependencies:** MV-010, MV-020, MV-040  
**Independent reviewer:** `mpdv_security`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- docs/mpd-viewer-v0.2/CONTRACTS.md
- docs/mpd-viewer-v0.2/decisions/ADR-001-viewer-auth.md
- docs/mpd-viewer-v0.2/decisions/ADR-002-presentation.md
- Task file-lease and dependency ledger

## Focused read scope

- Auth and grid spike reports
- Current native model/controller/player code
- CONTRACTS.md proposals
- S1–S9

## Work sequence

1. Choose the official chat embed implementation, popup/profile strategy, and platform support policy.
2. Freeze logical session vs native surface vs container identity, per-surface telemetry binding, and close semantics.
3. Define backward-compatible wrapper negotiation and settings migration ownership.
4. Decide retained or deliberate recreate mode switching by supported runtime; define rollback and failure visibility.
5. Lease shared native, wrapper, and manager paths to nonoverlapping tasks. Keep implementation interfaces small.

## Acceptance criteria

- Subsequent agents have explicit actions/events/state, permission boundaries, defaults, and error behavior.
- Each unresolved native limitation is a gate or an explicit fallback decision—not an assumption.
- No conflicting simultaneous leases on shared files.

## Required checks and evidence

- Review state-transition scenarios in CONTRACTS.md
- Check planned protocol against the existing deny_unknown_fields report
- Independent security and native-engineer review

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No broad refactor in the contract task.
- No final support claim based on a documentation page alone.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
