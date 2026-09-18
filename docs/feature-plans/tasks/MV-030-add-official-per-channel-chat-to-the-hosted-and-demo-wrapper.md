# MV-030 — Add official per-channel chat to the hosted and Demo wrapper

**Status:** Planned, not executed.  
**Agent:** `mpdv_web`  
**Model / reasoning:** `gpt-5.6-terra` / `medium`  
**Dependencies:** MV-001  
**Independent reviewer:** `mpdv_native`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- player-wrapper/index.html
- player-wrapper/player.js
- player-wrapper/style.css
- Focused chat/browser test fixtures

## Focused read scope

- Approved embed choice and popup policy
- Current Twitch.Player audio/event setup
- S1–S4
- Native Host local asset map

## Work sequence

1. Add an official per-channel chat iframe using validated channel and actual parent hostname; keep the existing video adapter unless ADR-001 changes it.
2. Enable chat by default, implement explicit collapse/restore without player reconstruction, and use responsive side/stack geometry.
3. Adjust only necessary wrapper frame policy. Coordinate native frame-navigation changes through the owning native task.
4. Keep video independent of chat errors and report unsupported chat/login behavior honestly.
5. Add clearly simulated Demo chat with no Twitch requests. Preserve volume-before-first-play and no periodic unpause behavior.

## Acceptance criteria

- Chat channel always matches viewer channel.
- Chat toggle preserves the same player instance, audio, pause, and session.
- Video meets minimum geometry and chat does not obscure it.
- Real native chat posting/login is covered by later acceptance, not inferred from mocks.

## Required checks and evidence

- Chat URL/parent injection tests
- Mock chat failure and channel replacement
- Collapsed/restored player identity and no extra play calls
- Real-CSP browser check
- Demo network-isolation check

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No custom chat client, chatbot, chat OAuth scope, or combined-embed migration without ADR approval.
- No deployment during implementation.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
