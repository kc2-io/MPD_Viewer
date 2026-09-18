# MV-060 — Package compatible release and deployment handoff

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** MV-051  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- Release/build metadata as approved
- Versioned wrapper build/package scripts and manifest
- README.md and release/setup documentation
- docs/mpd-viewer-v0.2/reports/release-checklist.md

## Focused read scope

- Final acceptance report
- Approved model/agent and architecture records
- Existing hosted site version and immutable artifact policy

## Work sequence

1. Package MPD Viewer binaries/source and matching immutable hosted wrapper assets with hashes/build IDs.
2. Keep the old root/v1 site available; produce the explicit native/wrapper compatibility matrix and rollback instructions.
3. Review client ID, parent hostname, legacy identity preservation, and developer-application display-name checklist.
4. List exact tested platforms/runtimes and limitations; do not label untested native builds supported.
5. Create deployment instructions. Publish remotely only after an explicit separate instruction identifying the destination.

## Acceptance criteria

- Artifact names/visible branding use MPD Viewer.
- Wrapper assets form a compatible, versioned set with rollback.
- Release notes distinguish implemented features, tested behavior and blocked support gates.

## Required checks and evidence

- Clean install/upgrade checks for packaged targets
- Artifact manifest/hash validation
- Versioned URL and CSP checks
- Rollback compatibility review

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No unapproved deployment, DNS/account changes, or silent old-client replacement.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
