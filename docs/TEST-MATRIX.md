# Verification matrix

Use the root [`justfile`](../justfile) for project commands. Install
[`just`](https://github.com/casey/just) through a trusted package manager or the
project's documented upstream installation methods, then run `just` to list
recipes. Recipes use Bash and require Python 3.11+ available as `python3`, Node
22.12+, and the same Rust and native prerequisites as the underlying existing
scripts and workflows. Windows users need a Bash-capable environment for these
root recipes.

No recipe silently changes a lockfile, launches Twitch, or turns a missing tool
into success. Cargo verification may download locked crates that are absent from
the local cache. E2E Node installation, registry auditing, GUI, and runtime work
use separately named recipes.

## Lanes

| Recipe | Evidence | Expected use |
|---|---|---|
| `just verify` | Offline source/configuration/release-policy tests, action pins, diff hygiene | Every change, including instructions and docs |
| `just verify-core` | Locked deterministic `mpd-core` and `mpd-twitch` unit tests | Scheduling, timer, selection, or Twitch-client logic |
| `just verify-native` | Locked workspace tests, Clippy, and optimized native build on the current host | Rust, Tauri, controller, permissions, storage, or native window work |
| `just verify-e2e` | E2E JavaScript syntax/unit checks and production/test isolation policy | Desktop E2E harness or workflow changes |
| `just verify-all` | All non-runtime lanes above; Cargo may fetch missing locked crates | PR-ready broad changes when host prerequisites exist |
| `just e2e-install` | Locked Node dependency installation; network access | Initial/updated E2E harness environment only |
| `just e2e-audit` | Current registry vulnerability audit; network access | Dependency or release review |
| `just e2e-build` | Instrumented unsigned native GUI test binary | Local desktop GUI preparation |
| `just e2e-run` | Actual desktop GUI harness using caller-supplied environment | Explicit local native E2E acceptance only |
| `just state` | Read-only local/GitHub repository, release, and PR state | Start of release/coordination work |

`just verify-native` matches the compile/test commands in native CI, but a local
single-platform pass does not replace the hosted platform matrix.

## Change-to-evidence routing

- Prose/instructions only: `just verify`. Confirm links and factual live-state
  claims separately.
- `crates/core` or `crates/twitch`: `just verify` and `just verify-core`.
- Native/controller/storage/auth/permissions: `just verify` and
  `just verify-native`, plus focused native runtime evidence for the behavior.
- Manager, wrapper, or adapter JavaScript/CSS: `just verify`; run the focused
  browser suite already associated with that feature when its Playwright/browser
  prerequisites are available. Label it mocked browser evidence.
- Desktop E2E harness: `just verify`, `just verify-e2e`, and the relevant hosted
  GUI matrix. Use `just e2e-run` only with the documented isolated environment.
- Workflows/release/signing: `just verify` plus targeted workflow/helper tests,
  independent adversarial review, and hosted checks from the changed revision.
- Visible UI: attach sanitized screenshot/video evidence from the changed
  revision, with run/commit identity and expiry. A missing runtime environment is
  a named draft-PR gate, not permission to reuse stale evidence.

Record commands and outcomes exactly. A skipped, cancelled, unavailable, or
mocked lane must be described as such.

## Hosted change impact

Pull-request and `main`-push workflows always run the lightweight source and
change-impact jobs. The shared `just ci-impact` classifier compares the actual
checked-out revision with its event base using the complete Git history,
NUL-delimited paths, and rename detection disabled so both sides of a move are
classified.

A change is non-build-only only when every changed path is one of:

- A Markdown file anywhere in the repository.
- A file below `docs/` or `.github/ISSUE_TEMPLATE/`.
- Root `LICENSE`, `.github/CODEOWNERS`, or the pull-request template.

Only an exact successful `full=false` result uses lightweight mode. Invalid or
missing revisions, classifier errors, empty diffs, unknown paths, workflow or
action-pin changes, and missing/failed classifier output all select the full
suite. Scheduled and manually dispatched Desktop E2E runs always select full
execution.

In lightweight mode, the four required Native check names each run a small
Ubuntu job that reports native compilation as not applicable and publishes no
binary. Desktop E2E runs one `GUI (Not-required)` decision job and produces no
GUI evidence. These are explicit successful applicability decisions, not claims
that native or GUI tests ran.

Full pull-request Desktop E2E remains the Windows x64, macOS arm64, and Linux
x64 fixture matrix. Scheduled runs, opted-in manual runs, and the
`desktop-e2e-extended` label add Intel macOS; this policy does not turn ordinary
pull requests into the extended matrix.
