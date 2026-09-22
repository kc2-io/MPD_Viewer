# Dark-mode status — Windows verification

Updated: 2026-09-22

## Verdict

**Not GTG: strict P1 native acceptance remains open.** Source fixes and independent
re-review are complete. Windows compilation is now verified; it must not be
conflated with completion of the native acceptance matrix.

## Branch and ownership

- Branch: `feature/dark-mode`, incoming HEAD `46add96a23b72e7ec4845b44e42d9e3c84286361`.
- Base: `origin/main` at `b167bdcd58a81e9125f9fb0a7574cf53eefbf2ed`.
- Isolated Windows linked worktree: `work/dark-mode-acceptance`.
- The incoming final commit already tracks the review and stream-timer plan. Both
  are preserved; stream-timer implementation is outside this task.
- Coordinator owns corrections; independent reviewer performed read-only review
  and re-review with no remaining blocking code/test findings.

## Corrections during Windows review

- Restored mandatory signed-in remembered-dark acceptance. Light URL alone is not
  a pass: visibly dark chat under light OS is a P1 failure requiring an architecture decision.
- Fixed the hosted browser test's incorrectly passed callback argument. It now
  asserts exact expected colors through light → dark → light and retained actual
  document, video-container and chat-frame nodes (not native SDK-object evidence).
- Improved light manager footer and Demo-label contrast and added assertions
  against their actual backgrounds, including both Demo gradient endpoints.
- Removed a new unnecessary `format!` in the canonical navigation test.
- Canonical native URL gate, per-navigation loading/error reset, profile identity,
  native capabilities and hosted provenance remain intact.

## Fresh Windows verification

| Check | Result |
|---|---|
| Six Node suites listed in `scripts/ci-check.sh` | 89 passed |
| `python scripts/release-tools.py check-version` | 0.1.0-alpha.3, passed |
| `python tests/configuration_test.py` | 15 passed |
| `python -m unittest discover -s tests -p release_test.py` | 51 passed (mock release tests, no publication) |
| `node tests/theme.browser.cjs` | manager, bundled wrapper, hosted: passed after corrections |
| `node tests/chat.browser.cjs` | hosted, bundled, Demo: passed |
| `cargo test --locked --workspace --features mpd-tabber/custom-protocol` | 42 passed on Windows |
| `cargo build --locked --release -p mpd-tabber --features custom-protocol` | passed on Windows |
| `cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol` | completed with 3 pre-existing controller warnings |
| Same Clippy command with `-- -D warnings` | fails on pre-existing controller warnings; not waived or labeled green |
| `cargo fmt --all -- --check` | fails on repository-wide formatting drift; no unrelated mass reformat performed |
| `git diff --check` for follow-up changes | passed |
| `git diff --check origin/main...46add96` | incoming plan Markdown hard-break whitespace warnings |

The source-check script's constituent commands were run directly in PowerShell;
no claim that the Bash entrypoint itself ran on Windows. Controller source is
unchanged from `origin/main`. Browser mocks are not signed-in/native evidence.

## Native Windows evidence

- Windows build 10.0.26200; this app's WebView2 executable is version 153.0.4234.48.
- Test executable is unsigned, version 0.1.0-alpha.3 (feature branch; not a release).
- SHA-256: `9b2f4cfce14db386774b8bce95d945f238604a92a5c90947b993b3cb18f6745b`.
- Native manager cold-started in light mode; light palette and window chrome observed.
- User completed normal Twitch authorization and started two live native viewers.
- Native signed-in light chat and video observed. No credentials/profile data were read.
- User reported chat reopened dark after the requested switch to Windows Dark.
- Subsequent native observation showed chat light with video paused; exact remaining
  matrix and user confirmations are recorded in `acceptance.md`.

## Open acceptance work

Finish the strict Windows matrix in `acceptance.md`, including confirmed remembered
Twitch Dark preference, both cold starts, both live transitions, and native identity,
state and navigation-count preservation evidence. Unobserved checks remain pending.
macOS/WKWebView and Linux/WebKitGTK runtime acceptance remain blocked on those platforms.
No release, tag or site deployment is part of this task.
