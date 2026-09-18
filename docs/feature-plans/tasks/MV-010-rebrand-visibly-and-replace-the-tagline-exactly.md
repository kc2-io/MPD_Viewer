# MV-010 — Rebrand visibly and replace the tagline exactly

**Status:** Planned, not executed.  
**Agent:** `mpdv_mechanical`  
**Model / reasoning:** `gpt-5.6-luna` / `low`  
**Dependencies:** MV-000  
**Independent reviewer:** `mpdv_web`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- ui/index.html
- Visible strings only in ui/app.js
- src-tauri/tauri.conf.json (display fields only)
- Title/startup strings only in src-tauri/src/player.rs and main.rs
- player-wrapper/index.html (title only)
- README.md and relevant UI test fixtures

## Focused read scope

- SOURCE-REVIEW.md branding and persistence entries
- src-tauri/src/storage.rs
- Run scripts and packaging workflow

## Work sequence

1. Use MPD Viewer for visible application branding and the exact tagline View fav channels in priority.
2. Set productName to MPD Viewer; preserve POC/build labeling separately.
3. Keep the stable bundle ID, config/profile paths, Rust package name, commands, public Client ID, and parent hostname.
4. Update literal-dependent fixtures. Add an intentional-legacy-name allowlist; historical verification documents remain accurate.
5. Record the optional owner action to rename the Twitch registration display name; do not change the developer account.

## Acceptance criteria

- Manager, native viewer, installer display fields, and wrapper title show the requested brand.
- Tagline string matches exactly, without rewriting fav to favorite.
- Upgrade fixture retains existing preferences and legacy identifiers.

## Required checks and evidence

- UI/title assertions
- Configuration regression tests including unchanged identifier/client ID/origin
- Native upgrade/install display-name check when installers are available

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No global tabber-to-viewer replacement.
- No Rust crate/binary rename or database/profile migration.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
