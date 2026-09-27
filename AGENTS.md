# MPD Viewer repository contract

This is the canonical instruction file for Codex, Claude Code, OpenCode,
Hermes, and other coding agents. Tool-specific files may adapt discovery, but
must not duplicate or contradict this contract.

## Start with actual state

- Inspect the current worktree, branch, dirty files, remotes, and relevant live
  GitHub state before writing. Never overwrite a newer checkout or unrelated
  user changes.
- `docs/PROJECT-STATE.md` is a dated snapshot, not live authority. Use it for
  orientation, then verify facts that can change.
- Read only the references relevant to the task. Do not load the historical
  handoffs or the entire feature-planning packet for a small change.
- Before editing a scoped area, read its instructions even if the client does
  not discover nested files automatically:
  - `crates/core/AGENTS.md` for selection and timer policy.
  - `src-tauri/AGENTS.md` for native/controller work.
  - `web/AGENTS.md` for hosted-source or wrapper work.
  - `.github/AGENTS.md` for CI, signing, tags, releases, or repository policy.
  - `tests/e2e/AGENTS.md` for desktop GUI automation and evidence.
- Use `docs/PRODUCT-CONTRACT.md` for durable product boundaries and
  `docs/TEST-MATRIX.md` to choose verification. Proposed contracts under
  `docs/feature-plans/` are not implemented behavior unless current code and
  state records say so.

## Default delivery workflow

Classify the request before acting:

- Analysis, diagnosis, explanation, research, planning, and review are
  read-only unless the user separately asks for changes.
- An implementation or fix request authorizes edits for that task in the
  assigned branch/worktree, relevant tests, commits of only task-authored
  changes, pushing that task branch, and opening or updating its pull request.
- That standing authority never includes committing unrelated dirty files,
  pushing directly to `main`, force-pushing, merging, tagging, publishing a
  release, deploying, changing repository/provider settings, or administering
  Twitch, DNS, signing, or cloud resources. Those require an explicit request.
- Missing credentials or permissions are reported, never bypassed.

For an implementation request, continue to PR-ready completion without waiting
for the user to repeat the normal steps:

1. Inspect existing behavior, instructions, and related tests. Infer reasonable
   acceptance criteria; ask only when a missing product choice would materially
   change the result.
2. Use the assigned worktree. When `agent-manager` is available, inspect active
   sessions/reservations, reserve writable files before editing, and release
   them when done. Otherwise state explicit file ownership for parallel work.
3. Use a short reviewed plan for consequential or multi-area work. Do not create
   a plan document for a trivial, well-bounded edit.
4. Implement the smallest coherent change. Preserve user work and avoid
   opportunistic cleanup.
5. Run the applicable `just` recipes from `docs/TEST-MATRIX.md`. Do not weaken a
   gate, hide a failure, mutate lockfiles incidentally, or describe a mock as
   native evidence.
6. Obtain the independent review required below, address actionable findings,
   and re-review blocking corrections.
7. Commit only the intended files, push the task branch, and open or update a PR
   against `main`. Use the repository PR template.
8. For visible UI changes, attach sanitized screenshot/video evidence from the
   changed revision. Label mocked layout evidence and do not present it as live
   Twitch or native acceptance.
9. Monitor required checks to a terminal result. Diagnose failures; retry only
   evidence-backed transient failures with a stated reason and bounded attempts.
   Skipped or cancelled checks are not success.
10. Report the PR URL, commit, exact checks and outcomes, review result,
    evidence/artifact links and expiry, and remaining limitations.

If required independent or runtime evidence is unavailable, open a draft PR and
name the missing gate. Do not claim PR-ready completion.

## Independent review without prompting

- Trivial prose-only edits may use self-review plus relevant validation.
- Production code and automation changes require an independent read-only
  review of the actual diff against its base revision.
- Consequential plans require independent plan review before implementation.
- Authentication, authorization, native lifecycle, IPC/permissions, workflow,
  signing, and release changes require an adversarial reviewer, preferably from
  another capable model/provider when available.
- Reviewers report findings and do not edit in the review pass. The owning agent
  resolves them or records an evidence-backed disagreement. Re-review blocking
  fixes. Material behavior changes after review invalidate that review.
- Use one reviewer by default. Add reviewers only for distinct high-risk
  domains; do not launch duplicate or unbounded review loops.
- Record review outcome in the PR. Create a lasting review document only when it
  captures a durable architectural decision.

## Product and safety invariants

- Rust is the selection/controller authority. Remote content renders assigned
  state and never chooses favorites or exceeds native selection.
- Preserve the existing application identifier, preference locations,
  browser-profile identity, public Twitch client ID, and HTTPS parent hostname
  unless a specifically approved migration says otherwise.
- Full Twitch channel pages are the default viewer. Embedded mode is an explicit
  build/runtime fallback with different supported controls; do not promise
  embedded volume, quality, chat, or telemetry behavior for full-page viewers.
- Monitoring OAuth and viewer website sessions are distinct. Never claim login,
  Turbo, rewards, viewer credit, or session persistence without the required
  live/native evidence.
- Use official Twitch surfaces and supported APIs. No ad blocking, artificial
  engagement, credential scraping, cookie import/export, browser-security
  disabling, or script injection into full Twitch pages.
- `web/parent.mpdviewer.com/index.html` is a provenance snapshot. Do not silently
  replace the player wrapper with it, modify it as production source, deploy it,
  or infer protocol compatibility from filenames.
- Never put tokens, cookies, signing keys, certificate passwords, or unredacted
  authentication material in source, logs, screenshots, artifacts, or chat.

## Build and release boundaries

- Use the committed lockfile and toolchain. GitHub Actions references remain
  pinned to full commit SHAs.
- PR builds are unsigned and receive no release environments, signing secrets,
  or OIDC tokens.
- Release tags are immutable
  `vMAJOR.MINOR.PATCH[-alpha.N|-beta.N|-rc.N]` and must match committed versions.
- Record source checks, native compilation, GUI/runtime evidence, signing, and
  live Twitch observations separately.
- Do not change expected-success gates to make an unverified release pass.
- Never merge, tag, publish, deploy, or alter release/signing scope without an
  explicit task authorizing that external action.

## Coordination and handoff

- One writer owns a source file at a time, including across worktrees. Shared
  native lifecycle/protocol files, permissions, workflows, and `ui/app.js` are
  serialized. Read-only reviewers do not take write ownership.
- Prefer one implementer and one reviewer. Parallel writers require disjoint
  file ownership and independently mergeable work.
- Keep existing instruction, model, reasoning, and approval settings unless the
  user explicitly changes them. Describe capability/risk needs rather than
  hard-coding provider model names into repository policy.
- Final handoffs are factual and concise: changed behavior, files, exact checks,
  review outcome, runtime evidence, PR/commit, limitations, and next external
  gate. Do not substitute promises or raw reasoning transcripts for evidence.
