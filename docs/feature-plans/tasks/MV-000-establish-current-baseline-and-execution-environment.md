# MV-000 — Establish current baseline and execution environment

**Status:** Planned, not executed.  
**Agent:** `mpdv_native`  
**Model / reasoning:** `gpt-5.6-sol` / `high`  
**Dependencies:** None  
**Independent reviewer:** `mpdv_architect`

Read the repository instructions, `AGENT-RULES.md`, and the relevant section of `PLAN.md` first. The coordinator must assign a base commit, worktree, and exclusive file lease. These scopes are a ceiling, not a requirement to edit every listed file.

## Writable scope

- docs/mpd-viewer-v0.2/reports/baseline.md
- Cargo.lock (generate only through Cargo; review resolved dependencies)
- Minimal baseline build repairs only under an additional coordinator-approved file lease

## Focused read scope

- Repository instructions and git status
- Cargo.toml and src-tauri/Cargo.toml
- .github/workflows/check.yml
- src-tauri/player-origin.json
- Existing tests and verification reports
- Actual deployed wrapper and current native build metadata

## Work sequence

1. Identify the user's current branch and uncommitted changes; do not replace it with the reviewed ZIP.
2. Compare the targeted files in SOURCE-REVIEW.md and record any local fixes or newer features. Check whether the installed binary corresponds to this checkout.
3. Record compiler, target OS/architecture, Tauri/Wry resolved versions, webview runtime, current profile configuration, and actual wrapper assets/headers. Redact account information.
4. Run baseline checks, distinguishing old failures from environment blockers. Generate/commit the application lockfile only after real dependency resolution.
5. Verify available Codex models/efforts, existing AGENTS rules, and the support matrix. Establish the file-lease ledger.

## Acceptance criteria

- Current checkout/deployment fingerprint and baseline outcomes are recorded.
- Missing platforms or account-interaction requirements are explicit.
- No user source/settings/profile is overwritten.

## Required checks and evidence

- cargo fmt --all -- --check
- cargo test -p mpd-core
- cargo test --workspace --all-targets
- cargo clippy --workspace --all-targets -- -D warnings
- node --test tests/view-model.test.mjs
- python3 tests/configuration_test.py
- Existing browser suites when their documented runtime prerequisites are available

A listed check is a requirement, not a claim that it has run. Mark missing platform/tool/account steps as blocked. Do not replace a native acceptance check with a mock and retain the same pass label.

## Explicit non-goals

- No feature implementation beyond separately approved baseline repairs.
- Do not repeat prior test counts without rerunning them.

## Handoff

Return task/base/worktree, actual model/effort, changed files, commands and outcomes, blocked checks, runtime/wrapper version, remaining risks, and patch/commit. Follow the full handoff format in `AGENT-RULES.md`. Do not include raw credentials or hidden reasoning transcripts.
