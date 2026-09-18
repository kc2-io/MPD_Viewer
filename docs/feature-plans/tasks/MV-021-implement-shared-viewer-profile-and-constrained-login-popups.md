# MV-021 — Implement shared viewer profile and constrained login popups

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-001  
**Independent reviewer:** `mpdv_security`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- src-tauri/src/player.rs
- src-tauri/src/viewer_auth.rs (new, if useful)
- Necessary module registration in src-tauri/src/main.rs
- src-tauri/src/model.rs and controller.rs for bounded auth commands/events only
- Native auth/popup tests

## Focused read scope

- Approved ADR-001
- Existing profile paths/OS support
- src-tauri/capabilities/*
- S5,S7,S8,S9

## Work sequence

1. Reuse the identified persistent app-owned playback profile for all viewers and login popups. Preserve existing sessions when possible.
2. Implement official popup handling using native opener/features/environment relationships and the audited destination policy.
3. Keep auth windows unprivileged and out of playback capacity accounting; bound and track their lifecycle.
4. Provide manager-origin sign-in and explicit safe viewer-session reset primitives if supported; preserve OAuth Disconnect as a different action.
5. Respect normal storage/cookie restrictions and expose unsupported-runtime failures. Do not fake a login/Turbo success state.
6. Avoid logging auth query strings, cookies, passwords, or raw identity-provider responses.

## Acceptance criteria

- Official login works on each claimed supported runtime with the user entering credentials.
- Second viewer uses the intended profile; persistence and reset behavior are documented.
- Arbitrary popup/navigation attempts are denied and auth windows have no native app authority.

## Required checks and evidence

- Destination parser tests including lookalike hosts, credentials-in-URL, non-HTTPS and redirect changes
- Native popup/related-view and second-viewer checks
- Reset and app-exit cleanup
- Negative command/telemetry checks from auth window

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No cookie import from Chrome/Firefox.
- No automatic consent/privacy bypass.
- No token persistence redesign.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
