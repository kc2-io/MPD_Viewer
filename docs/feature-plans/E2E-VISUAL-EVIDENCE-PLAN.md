# E2E visual evidence and pull-request reporting plan

Date: 2026-09-25

Planning baseline: `87458ac` (`origin/main`)

Status: hosted capture accepted; read-only reporter merged; comment rollout under review

## Outcome

Every `Desktop E2E` matrix job will retain a bounded, silent screen recording,
checkpoint screenshots, JUnit, manifests and sanitized logs as one downloadable
GitHub Actions artifact. A reviewer will see one bot-managed comment on the pull
request with the tested revision, each platform's job result, direct artifact
links, the Actions run link and the evidence expiration date. The same artifacts
remain available from the workflow-run summary for main, scheduled and manually
dispatched runs.

The comment links to GitHub-hosted evidence; it does not embed screenshots or
video. Actions artifact URLs are authenticated ZIP downloads rather than stable
image/video origins. Inline media would require a separate publishing surface
such as Pages or a writable evidence branch, would expose untrusted PR-produced
images publicly, and is not authorized by this plan.

This is diagnostic fixture evidence. It does not establish live Twitch playback,
login, Turbo, OS-native input, audio, signing or release acceptance.

## Verified starting point

- `.github/workflows/desktop-e2e.yml` runs ordinary, read-only `pull_request`
  jobs on Windows x64, macOS arm64 and Linux x64. Extended runs add macOS x64.
- Each matrix job already stages evidence under `if: always()` and uploads
  `Desktop-E2E-<platform>-<run>-attempt<attempt>` for seven days.
- `tests/e2e/support.mjs` already captures bounded PNGs for selected success
  milestones and `tests/e2e/run.mjs` captures every assertion failure.
- `scripts/e2e-stage-evidence.py` admits only top-level JSON, XML, log and PNG
  files, rejects links/sensitive names, and caps file count, file size and total
  bytes. Profiles, SQLite, vault contents and credentials are outside the
  evidence directory and must remain outside it.
- There is no recorder, no run summary and no PR reporter. The existing `CI`
  workflow uploads unsigned binaries but does not execute GUI tests; it is not a
  visual-evidence target.
- The local checkout is clean at the planning baseline. Local `gh` credentials
  were invalid when checked, so current hosted-run behavior was not re-read via
  the API during planning. Repository policy and the current checkout identify
  the repository as public; implementation must recheck the live settings before
  any hosted rollout.

## Architecture and trust boundary

```text
untrusted pull_request workflow                 trusted default-branch reporter

Desktop E2E matrix                              workflow_run: Desktop E2E activity
  fixture-only native app                         no PR checkout
  silent desktop recorder                         no artifact download or parsing
  PNG checkpoints                                 read run/jobs/artifact metadata
  bounded staging                                 verify current PR/head association
  upload artifact ------------------------------> update one marked PR comment
  write job summary                               links back to GitHub artifacts/run
```

The capture workflow keeps only `contents: read` and receives no secrets, OIDC or
release environment. It continues to use `pull_request`, never
`pull_request_target`. A separate `.github/workflows/desktop-e2e-report.yml` runs
from the default branch on selected `workflow_run` activity with only `contents: read`,
`actions: read`, `pull-requests: read` and, only after a read-only rollout,
`pull-requests: write`.

The reporter must not check out the pull-request ref, run pull-request code,
download/unzip artifacts, or render artifact contents into Markdown. It checks
out only the exact default-branch commit that received the event, with persisted
credentials disabled, to run a small standard-library Python reporter. Its parsing, escaping and
stale-run behavior have unit tests. It does not install packages or use a cache.

## Evidence contract

Each platform artifact contains only top-level files:

| Evidence | Required behavior |
|---|---|
| `video-*.mp4` or `video-*.webm` | Silent recording of the disposable desktop only while the GUI suite runs. Segment output so a timeout still leaves finalized, playable evidence. Freeze one cross-platform format after the spike. |
| `checkpoint-*.png` | Capture all native windows after each completed case, plus existing named milestones and failure screenshots. Retain the eight-window and PNG-signature checks. |
| `results.xml` | Existing JUnit result; test failure remains a failing job. |
| `manifest.json`, `runner-manifest.json` | Existing test/build/runtime boundaries plus recorder name/version, format, dimensions, frame rate, segment count, duration and capture result. |
| `application-*.log`, `capture.log` | Existing sanitized/bounded logs and recorder diagnostics without raw environment, URLs, tokens or profile data. |
| `upload-summary.json`, `SHA256SUMS.txt` | Staged file names/sizes/rejections and hashes for extraction/transport diagnostics. These PR-generated hashes are not provenance. The GitHub-provided artifact digest is the external integrity value. |

Use no microphone or system audio. Start recording after the isolated fixture
environment exists and stop it before cleanup exits. Record the whole disposable
desktop so manager and child Tauri windows are visible; WebDriver page recording
alone would omit native multi-window lifecycle.

The recorder owns exact child handles/PIDs and stops only those children. It must
receive the runner's interrupt, stop in `finally`, and never kill by process name.
It has recorder-specific shutdown rather than reusing the app process helper:
send FFmpeg `q` on its stdin, wait about three seconds, then escalate only that
exact child. Do not detach it unless signals are explicitly forwarded. Record
graceful, escalated and lost shutdown distinctly. On interrupt, finalize the
recorder before slower app/profile cleanup. Give the workflow step the harness
deadline plus a measured finalization margin; the current equal 10/20-minute
harness and step deadlines are insufficient.

Use short finalized segments (target 60 seconds) and a container that tolerates
abrupt cutoff, then validate each candidate with `ffprobe` before staging. A
corrupt, zero-frame, over-limit or symlinked video is rejected and recorded in
the manifest.

Initial encoding target for the hosted spike is at most 1280 x 800, 6-10 frames
per second, no audio and approximately 600-900 kbit/s. The implementation spike
selects the exact codec/container using tools already on each runner image where
possible. GitHub runner labels roll forward, so record image version plus recorder
and `ffprobe` path, version and executable hash. Any downloaded tool is pinned by
version and checksum. Candidate backends are FFmpeg desktop capture on Windows/Linux
and the native macOS screen recorder followed by the same bounded validation.
Do not add an unpinned recorder action or silently replace a failed platform
recording with a passing placeholder.

The first hosted PR run proved that none of the assigned images supplies FFmpeg.
Provision release `b6.1.1` from `eugeneware/ffmpeg-static` using the repository's
standard-library installer. It selects one fixed FFmpeg/FFprobe pair for each
supported OS/architecture, verifies committed SHA-256 values on the compressed
release assets before bounded decompression into `RUNNER_TEMP`, and checks the
platform capture device plus `libx264` before any build or GUI work. No setup
action, package-manager mutation, cache or secret is involved. The release bundle
contains different underlying FFmpeg builds by platform, so the runtime manifest
records the actual executable path, version and hash and hosted playback remains
an acceptance gate.

Replace the staging script's alphabetical, global 8 MiB/128-file treatment with
reserved per-type quotas and priority: JUnit/manifests first, sanitized logs next,
video next and checkpoints last. Core reports cannot be evicted by screenshots.
Name checkpoints only by sequence/window (`checkpoint-017-w0.png`); case names
belong in the manifest and cannot trip the sensitive-name filter. Preserve the
current text/PNG per-file limit, allow only the selected video extension, start
with 32 MiB per segment, and derive the total/platform and full-matrix byte budgets
from the 20-minute extended run before making them gates. A 128 MiB total is not
enough at the high end of the proposed bitrate. Configure artifact upload with
`compression-level: 0` for already-compressed media. Retention stays seven days
initially and the PR comment states the API-provided expiration.

The harness saves and restores the exact active native window around every
checkpoint. It enforces its own checkpoint quota, always preserves failure and
named milestone images, and records any skipped routine checkpoint. Staging does
not silently turn a passing GUI suite red merely because optional routine images
exceeded a quota; required report/video absence is handled by the separate
evidence validator.

## Pull-request comment and run summary

The unprivileged matrix job assigns an `id` to `actions/upload-artifact`, then
writes an `if: always()` job summary containing its job result, platform, source
revision, screenshot/video counts, evidence boundary and, when upload succeeded,
the action's `artifact-url` and `artifact-digest`. A failed upload is shown as
failed rather than producing an empty link. This provides evidence links for PR,
main, scheduled and manual runs without any write permission.

The reporter handles `requested`, `in_progress` and `completed` activity for an
upstream run whose event is `pull_request` and repository is the expected
repository. A name match is insufficient because PR code can add an impostor
workflow. Require path `.github/workflows/desktop-e2e.yml` and require its numeric
workflow ID to match the ID GitHub returns for that default-branch file.

When `workflow_run.pull_requests` identifies exactly one PR, validate it against
the current PR. When GitHub supplies no association (which must be tested for fork
PRs), query open PRs by the exact head owner/branch and accept exactly one result
only after its head SHA, head repository ID and base repository ID match the run
and current repository. Use the branch only as an API selector, never in shell or
comment text. Ambiguous/no matches fail closed. Serialize reporter activity per
head repository ID and head branch with `cancel-in-progress: false`; the reporter
then rebuilds current-head state instead of trusting event arrival order. This
avoids dropping pending state transitions without needlessly serializing unrelated
pull requests.

It skips an event when the resolved run is not the newest Desktop E2E run/attempt
for the current PR head. A requested/in-progress event replaces an old green body
with an explicit running state; every completed conclusion, including cancelled,
timed out, action required, startup failure, stale, skipped and neutral, is shown
truthfully. The hosted spike must freeze the comparison fields for ordinary and
fork PRs and for GitHub's “re-run failed jobs” behavior, where jobs/artifacts may
come from different attempts.

The reporter queries job and artifact metadata through the Actions API and builds
links from GitHub-returned numeric run/artifact IDs. It never prints PR-controlled
job/artifact names. Exact known job labels and exact artifact-name regular
expressions map to reporter-owned constant platform labels; all other values are
ignored. No artifact bytes are trusted. It also compares Git tree/blob metadata
for the workflow, E2E harness and evidence scripts at PR head versus the default
branch. A fixed warning banner appears when that test infrastructure differs,
because the results were produced by PR-controlled code and are not independent
attestation. The comment includes:

- an exact first-line hidden versioned marker plus confirmation that the existing
  comment author has the fixed `github-actions[bot]` login, bot type and numeric
  user ID; when `performed_via_github_app` is present, its slug must match the
  Actions app before update;
- PR head SHA, tested Actions SHA, run/attempt, conclusion and completion time;
- one row per expected platform with job conclusion and artifact link, including
  a clear missing/expired-artifact state;
- run-summary and rerun links, GitHub artifact digest when the API supplies it,
  seven-day retention/expiration, and fixed disclaimers that the fixture results
  came from the PR revision and do not establish live Twitch behavior.

Page through all comments. If no marked bot comment exists, create it. Otherwise
rewrite the entire body of the oldest exact match so new commits do not flood the
conversation; never create a second matching comment or edit a user/other-bot
comment that copied the marker. A same-repository writer can still impersonate
the shared Actions app and is already trusted to change workflows. A missing PR
association, stale head, unexpected repository/workflow, API error or insufficient
permission makes the reporter fail or skip with an explicit job summary. Main,
schedule and manual runs receive job summaries only.

No `${{ github.event.* }}` or `github.head_ref` value is interpolated inside a
`run:` block. The reporter reads `GITHUB_EVENT_PATH` and passes fixed environment
values to its script. Static policy tests enforce that restriction, the explicit
default-branch checkout, `persist-credentials: false`, no artifact download, no
cache/package installation, and no `pull_request_target`, secrets, OIDC or
environments.

## Implementation packets and exclusive ownership

Use one coordinator and at most three workers. At most two workers write code in
parallel. The coordinator assigns these exact leases before work and is the only
writer for workflow files and final integration.

| Packet | Agent/model budget | Exclusive lease | Deliverable |
|---|---|---|---|
| VE-01 recorder spike | `gpt-6-sol`, high reasoning | Isolated, unmerged experiment only: temporary workflow/harness changes in its own worktree | Prove start/stop/interrupt and playable silent segments on each hosted OS; report exact tools/versions and measured size. Its overlapping workflow changes are experimental evidence, never merged. No production Rust changes. |
| VE-02 harness capture | `gpt-6-sol`, medium reasoning | `tests/e2e/video.mjs`, `tests/e2e/run.mjs`, `tests/e2e/support.mjs`, `tests/e2e/specs.mjs`, `tests/e2e/README.md`, `tests/e2e/package*.json`, `scripts/e2e-linux-session.sh` | Integrate the accepted recorder, exact-window restoration, bounded per-case screenshots and manifest fields without weakening assertions or cleanup. Depends on VE-01. |
| VE-03 staging/reporting | `gpt-6-sol`, medium reasoning | `scripts/e2e-stage-evidence.py`, `scripts/e2e-runtime-manifest.py`, new evidence validator/summary/PR-report scripts, and new dedicated test files only | Type-specific evidence validation, checksums, summaries, and strictly metadata-only reporter logic. Do not add new cases to shared `scripts/e2e-runner-test.py`. |
| VE-04 integration | coordinator, high reasoning | `.github/workflows/desktop-e2e.yml`, new reporter workflow, `.github/action-pins.json`, `scripts/pin-actions.py`, `scripts/ci-check.sh`, this plan and `docs/setup/DESKTOP-E2E.md` | Freeze interfaces, integrate workers serially, preserve pinned actions/permissions, run checks and perform the two-merge hosted rollout. |
| VE-05 adversarial review | Claude Code CLI, strongest available review model, read-only | No file lease | Attack fork safety, workflow privilege boundary, shell/Markdown injection, stale-run races, cleanup, artifact leakage/cost and failure semantics. Findings return to the owning packet; reviewer does not self-fix. |

Use `gpt-6-luna` at medium or lower effort only for bounded mechanical inventory
or documentation checks; do not spend a frontier/high-reasoning worker on those.
The coordinator freezes two versioned interfaces before the writers start: the
evidence manifest/staging schema between VE-02/VE-03 and the reporter's trusted
event/API inputs between VE-03/VE-04. The coordinator may reduce active workers
when integration touches shared workflow state. No worker changes Rust/controller,
release workflows, signing, parent-site source or deployment.

## Execution phases

### 1. Revalidate and spike

1. Recheck clean worktree, `origin/main`, live repository visibility, Actions
   retention cap, artifact storage/quota, fork approval/token/secret settings and
   default workflow-token policy. Inspect current Desktop E2E runs before writes;
   refresh local GitHub authentication through normal browser flow if needed
   without printing credentials.
2. In an isolated experiment branch/worktree, probe recorder availability on
   Windows x64, macOS arm64, Linux x64 and extended macOS x64. Capture the existing
   controlled fixture suite; record CPU, elapsed time, resolution, duration, bytes
   and whether a forced failure/cancellation/timeout leaves playable segments.
   Creating a real fork PR is a separate external action and requires explicit
   authorization at execution time.
3. Select and document one output contract. Stop if macOS runner permissions or a
   missing codec prevents reliable capture; do not call a screenshot slideshow a
   real-time recording without an explicit scope decision.

### 2. Implement capture and staging

1. Add recorder lifecycle with owned-process cleanup and no audio. Preserve the
   original test exit code and record capture status without changing that code.
   Add a separate `if: always()` `Validate visual evidence` step that reports its
   own conclusion. During rollout it is diagnostic; making it a required gate is
   a later explicit decision after the five-run stability evidence.
2. Capture checkpoint PNGs after every test wrapper completion and on failure.
   Save/restore the active window; bound filenames, window count and per-type file
   quota; preserve failure/milestone images; keep screenshots supplementary to
   assertions. Compare checkpoints enabled/disabled at the same revision.
3. Extend staging with exact video suffix, signature/container validation results,
   per-type limits, hashes and tests for links, sensitive names, oversize video,
   total size, corrupt media, absent media and partial segments.
4. Upload evidence under `if: always()` and write the per-platform job summary
   after upload. Preserve seven-day retention and unique run/attempt naming.

### 3. Add the privileged metadata-only reporter

1. Add unit-tested API/comment logic first using recorded synthetic event/API
   fixtures: same-repository and fork PRs, missing association, stale head, rerun,
   copied marker, expired/missing artifacts, hostile names, API pagination/error
   and concurrent completions.
2. Add the `workflow_run` workflow on the default branch with read-only permissions,
   per-head serialized concurrency and an exact default-branch commit checkout.
   Assert statically that it contains no PR checkout, artifact download, shell
   interpolation of PR fields, secret/OIDC/environment access or
   `pull_request_target`.
3. Merge the read-only reporter first and inspect same-repository runs plus synthetic
   fork payloads. In a second, separately reviewed PR, add only
   `pull-requests: write` and comment create/update. If explicitly authorized,
   verify a real fork run. Confirm exact comment ownership, links, head SHA,
   in-progress state, every terminal conclusion and rerun behavior.

### 4. Acceptance and rollout

Run the smallest local checks first, then all source checks and the real hosted
matrix. Required evidence before declaring the plan implemented:

1. Unit/static checks pass, including existing isolation/release-boundary tests,
   action SHA pin checks and workflow YAML validation.
2. A deliberate GUI assertion failure remains red, produces failure PNGs and a
   playable video segment, uploads evidence, updates the PR comment and cleans only
   owned app/recorder processes. Remove the deliberate failure afterward.
3. Success, failure, cancellation/timeout and rerun attempts all produce truthful
   summaries. A cancelled matrix does not get summarized as passed.
4. Each primary platform produces readable video and checkpoint PNGs for the exact
   tested SHA. The four-platform extended run separately covers macOS x64.
5. Synthetic fixtures prove the fork lookup fails closed. When the user separately
   authorizes a real fork PR, it proves the E2E job stays read-only/no-secrets while
   the trusted reporter can safely link its artifacts without consuming their bytes.
6. Download each artifact from both the Actions run and PR comment, verify hashes,
   open screenshots and play all video segments. Confirm links report expiration.
7. Inspect artifacts for profiles, SQLite, cookies, authorization URLs, tokens,
   environment dumps and unintended desktop content such as Server Manager.
   Fixture-only codes/URLs may be visible and are documented; any credential or
   non-fixture leak blocks rollout. Crop to the disposable Xvfb display on Linux.
8. Measure artifact bytes and added job time across five primary matrices. Adjust
   frame rate/bitrate within the evidence contract if needed; do not hide recorder
   failures or weaken GUI assertions to meet cost targets.
9. Independent adversarial re-review has no unresolved high-severity findings.

Roll out capture and the reporter as non-required diagnostic automation first.
During rollout, inspect reporter workflow runs directly because a failed reporter
cannot update its own PR comment. The comment always displays its update time and
tested head. Existing CI,
Desktop E2E result gates, release workflows, environment protections and signing
remain unchanged. If reporting fails, tests and uploaded artifacts remain the
source of truth. Rollback disables only the reporter/capture additions; it must
not delete historical runs/artifacts, rewrite PR comments with false success, or
change test conclusions.

## Planning references

- [GitHub workflow-run event and privilege warning](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run)
- [GitHub secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use)
- [GitHub Actions artifacts REST API](https://docs.github.com/en/rest/actions/artifacts)
- [GitHub pull-request comment API](https://docs.github.com/en/rest/issues/comments)
- [Downloading workflow artifacts](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts)
- [`actions/upload-artifact` outputs and retention](https://github.com/actions/upload-artifact/blob/main/README.md)

These references define platform behavior; they are not evidence that the planned
workflow has run successfully in this repository.

## Adversarial review record

Reviewed 2026-09-25 against baseline `87458ac` and the first draft using Claude
Code CLI 2.1.280 in read-only/plan mode with the requested `opus` alias and high
effort. Text output did not expose the alias's resolved model ID. Verdict:
**ACCEPT WITH CHANGES**. No hosted runner was used by the reviewer.

Accepted corrections:

- fail-closed fork PR lookup using owner/branch only as an API selector plus exact
  head SHA and repository-ID validation;
- verify default-branch workflow path and numeric ID, not spoofable workflow name;
- fixed warning when PR-controlled workflow/harness inputs differ, constant label
  mapping, and no rendering of attacker-controlled job/artifact names;
- recorder-first graceful interrupt, deadline margin, abrupt-cutoff-safe segments
  and recorder-specific process handling;
- prioritized per-type quotas, numeric checkpoint names, exact active-window
  restoration and checkpoint-on/off equivalence testing;
- two-merge reporter rollout so read behavior is observed before granting comment
  write permission;
- per-head reporter serialization, current-head/newest-run reconstruction,
  explicit in-progress/non-success states and rerun-attempt testing;
- stronger comment ownership/pagination, concrete injection/static checks, runner
  tool identity, GitHub artifact digest, visibility/quota revalidation, separate
  evidence-validation status, complete file leases and frozen worker interfaces.

One recommendation was narrowed: a real fork PR is valuable hosted evidence, but
creating it is an external repository action and remains contingent on explicit
authorization. Synthetic fork payload/API tests are required regardless. Inline
media hosting remains deliberately rejected because it broadens publication and
is unnecessary for the requested GitHub artifact downloads.

The reviewer requested hosted validation of fork association, workflow path/ID,
rerun artifact semantics, signal grace, macOS screen-capture permission, recorder
availability/version, capture overhead/flakiness, Retina PNG size, artifact digest
fields, live repository settings and fork comment permissions. Those are now
explicit phase-one or acceptance gates rather than assumed facts.

### Implementation adversarial review

The integrated implementation was reviewed again on 2026-09-25 with Claude Code
CLI 2.1.280 in read-only mode using the requested `opus` alias and high effort.
Its verdict was **ACCEPT WITH CHANGES**. Applied changes include preserving
failure/milestone screenshots ahead of routine checkpoints, hardening malformed
tool/metadata probes, adding fragmented-MP4 packet flushing, using per-head
non-cancelling reporter concurrency, null-safe API parsing, fixed bot identity,
truthful merge-SHA labeling, and reconstructing artifacts across failed-job
reruns. The first reporter rollout remained dry-run and metadata-only.

A final read-only sub-agent audit found two additional hosted blockers: the
reporter needed explicit `pull-requests: read`, and the interrupt path could wait
indefinitely before finalizing FFmpeg. Both are fixed and covered by policy/static
tests. No high-severity local review finding remains. Hosted Windows x64, macOS
arm64 and Linux x64 capture/playback passed on
[run 36215087749](https://github.com/kc2-io/MPD_Viewer/actions/runs/36215087749),
with checksum-valid artifacts and independently probed H.264 video. The native
Linux x64, Windows x64, macOS arm64 and macOS x64 matrix passed on
[run 36215087751](https://github.com/kc2-io/MPD_Viewer/actions/runs/36215087751).
The default-branch dry-run reporter is merged; the separately reviewed
write-enabled comment rollout and the remaining deliberate failure, cancellation,
repeatability and real-fork gates remain acceptance work rather than inferred success.

The second-stage privilege review found that GitHub can omit `pull_requests` from
the workflow-run metadata. The original fallback could therefore rebind a late run
to a newer PR that reused the same fork branch and head commit. The write rollout
now queries historical PRs and requires exactly one matching PR that was active
when the run began, fails closed on missing timestamps or overlapping histories,
and rechecks both base and head immediately before writing. It also reruns capture
when a PR is edited and displays the current base SHA; later default-branch advances
remain visible as the stated base-at-report-time rather than being represented as
part of the tested head identity.
