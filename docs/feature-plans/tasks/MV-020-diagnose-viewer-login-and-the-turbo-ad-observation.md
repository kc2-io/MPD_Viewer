# MV-020 — Diagnose viewer login and the Turbo ad observation

**Status:** Planned, not executed.  
**Agent:** `mpdv_architect`  
**Model / reasoning:** `gpt-6-astra` / `high`  
**Dependencies:** MV-000  
**Independent reviewer:** `mpdv_security`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- docs/mpd-viewer-v0.2/reports/auth-turbo-spike.md
- A disposable auth-spike worktree; experimental changes are NOT merged

## Focused read scope

- src-tauri/src/player.rs
- src-tauri/src/controller.rs: OpenAuth/Connect/Disconnect
- src-tauri/src/main.rs
- player-wrapper/*
- Actual wrapper and runtime evidence
- S1,S2,S5,S7,S8,S16

## Work sequence

1. Reproduce and distinguish monitoring OAuth, normal-browser login, chat identity, and native video session.
2. Record the existing deny-all popup and navigation policies; determine the actual profile sharing before proposing new paths.
3. Run the comparison matrix in PLAN.md with the user performing login/MFA. Collect only redacted errors and observations.
4. Experiment with official login popup handling and related-view configuration. Identify exact required origins and any storage-access limitations.
5. Decide whether retaining Twitch.Player plus separate official chat is viable. Recommend combined Twitch.Embed only with evidence and an approved ADR.
6. Return a supported, unsupported, or blocked result per runtime. Identify the likely cause only to the confidence supported by evidence.

## Acceptance criteria

- A reproducible sequence and falsifiable diagnosis are provided, or the exact missing evidence is named.
- No claim that API authorization proves website sign-in or Turbo.
- Popup/profile requirements are concrete inputs to MV-001/MV-021.

## Required checks and evidence

- Normal Twitch page vs hosted wrapper vs native viewer comparisons
- Second-viewer and restart checks
- Negative popup/navigation tests
- Actual ads or missing sign-in documented without credentials

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No ad blocking, unsupported APIs, browser-security flags, cookie copying, or password capture.
- No production patches or website deployment from spike code.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
