# MV-031 — Version the wrapper protocol and preserve old clients

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-021, MV-030  
**Independent reviewer:** `mpdv_security`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- src-tauri/src/main.rs (hello/report endpoints)
- src-tauri/src/model.rs (negotiation types)
- src-tauri/build.rs
- src-tauri/permissions/app.toml
- src-tauri/capabilities/players.json
- player-wrapper/player.js (negotiation only)
- src-tauri/src/player.rs (asset map if needed)
- scripts/set-player-url.py and compatibility/configuration tests

## Focused read scope

- CONTRACTS.md protocol decision
- Old v1 Report and three-asset server
- Existing hosted update script and remote permissions

## Work sequence

1. Implement the approved bounded hello/version handshake without breaking v1 deny_unknown_fields payloads.
2. Keep old and new client/wrapper combinations explicit. Reject incompatible versions with an actionable error, not repeated reloads.
3. Bind telemetry/hello to native caller ownership under the approved contract. Register every new command in the build manifest and capabilities.
4. Keep Demo bundled assets and deployed asset manifest synchronized. Retain restrictive manager isolation.
5. Prepare immutable version-path deployment packaging on the existing parent host; retain the v1 root for old clients.

## Acceptance criteria

- Old client/new wrapper and new client/old wrapper behavior matches the compatibility table.
- No widening of manager permissions to remote pages.
- Native and wrapper agree on supported feature/version metadata.

## Required checks and evidence

- Protocol compatibility fixtures
- Unknown field/major-version/missing hello cases
- Configuration script exact-origin checks
- Native denial of unauthorized hello/report calls
- Real header/meta CSP agreement

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No replacement of root hosted assets without authorization.
- No migration to a new parent hostname.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
