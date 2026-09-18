# Connected setup verification — September 18, 2026

This record supersedes the historical disconnected-environment status in the
handoff and older setup documents. It does not supersede their release gates.

## Checkout, identity, and source

- Actual checkout: `D:\MPD_Viewer-Codex-Project\MPD_Viewer`.
- Initial state: clean `main`, no remotes, `e356f44`; all four supplied commits
  preserved. No history replacement, reset, force push, or website deployment.
- GitHub CLI 2.101.0 downloaded from the official `cli/cli` release; archive
  SHA-256 `bc6c814367b193cd8e713611d61e36013c0ef843b8f516458fe3eda039192794`.
  Browser authorization completed as `ipl31`, active admin of `kc2-io`.
  The portable CLI is in the continuation task's `work/tools/gh/bin/gh.exe`;
  it was not installed on the system PATH.
- The authenticated organization inventory and target lookup were checked before
  creation. `kc2-io/MPD_Viewer` was absent. It was created **private**, privacy was
  verified before pushing, and `origin` now points to
  <https://github.com/kc2-io/MPD_Viewer>.
- Setup commit: `0ed080a706838fb292fa652d6833607db79d4294`.
  Real Cargo.lock (479 packages), Rust 1.98.1, and seven official Action tag
  resolutions are committed. `.github/action-pins.json` records the resolutions.
- HTML checkout line endings are now LF, preserving the hosted snapshot's exact
  provenance bytes on Windows. The action-pin guard covers both workflow file
  extensions and rejects unsupported `uses` syntax instead of silently skipping it.
- Internal crate, bundle identifier, preferences paths, Twitch client ID, app
  version `0.1.0`, and supplied hosted HTML content are unchanged.

## Fresh validation

| Check | Actual result |
|---|---|
| `bash scripts/ci-check.sh` (real GitHub source job) | 76 passed: 30 JavaScript, 14 configuration, 32 release-helper |
| Independent review | Initial two findings fixed; revised diff and 76 source tests passed |
| `cargo test --locked --workspace --features mpd-tabber/custom-protocol` | Passed on Windows x64: 21 Rust tests |
| `cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol` | Passed; two existing controller style warnings |
| `cargo build --locked --release -p mpd-tabber --features custom-protocol` | Passed on Windows x64 |
| Native host, version, action pins, committed-lock and whitespace checks | Passed |
| Local executable Authenticode | `NotSigned`, as expected for a diagnostic build |
| Native signature verifier | Accepted the genuine timestamped mpd-bot reference; rejected unsigned MPD Viewer and a mismatched expected publisher |
| Local Windows binary packaging | x64 PE, MPD Viewer 0.1.0 resources, exact executable bytes preserved in diagnostic ZIP |
| Native UI/Twitch/login/Turbo/grid acceptance | Not run |

Local executable SHA-256:
`7ba3944cc9cf757ab021af7a76a4d35957847c274aaf32062456852f89bbb459`.
The local diagnostic ZIP was staged by `release-tools.py stage-ci`; it is not a
signed release or installer.

Actual GitHub matrix run:
<https://github.com/kc2-io/MPD_Viewer/actions/runs/35386136378>.
**Success:** source checks and all four native jobs passed. Each native job ran
21 Rust tests, clippy, and an optimized executable build. The same two existing
clippy style warnings appeared on all hosts. All four artifacts were downloaded;
their outer ZIP digests matched GitHub's published SHA-256 values. Inner diagnostic
ZIPs were hashed, their executable architectures verified, and Windows Authenticode
was confirmed `NotSigned`. The artifact manifest is
`CI-ARTIFACTS-2026-09-18.json` beside this record.

These are diagnostic executable ZIPs: no signed Windows release, macOS app/DMG,
or Linux DEB/AppImage was produced. CI artifacts have seven-day retention.

| Platform | Verified CI artifact |
|---|---|
| Linux-x64 | [Unsigned ZIP](https://github.com/kc2-io/MPD_Viewer/actions/runs/35386136378/artifacts/10565115614) |
| macOS-x64 | [Unsigned ZIP](https://github.com/kc2-io/MPD_Viewer/actions/runs/35386136378/artifacts/10564725652) |
| Windows-x64 | [Unsigned ZIP](https://github.com/kc2-io/MPD_Viewer/actions/runs/35386136378/artifacts/10564492304) |
| macOS-arm64 | [Unsigned ZIP](https://github.com/kc2-io/MPD_Viewer/actions/runs/35386136378/artifacts/10563684155) |

The follow-up documentation-only commit records this tested source commit and
skips a redundant native CI run. It changes no executable source, dependencies,
workflows, release gate, or signing configuration.

## Reference repositories actually inspected

Both repositories are public; MPD Viewer remains private. Inspected current
workflow files, their blob SHAs, repository permissions/settings, branch rules,
environment deployment policies, and secret/variable names. References were
read-only throughout. No secret values were requested or copied.

| Convention | BotOrNot | mpd-bot | MPD Viewer |
|---|---|---|---|
| Release workflow blob | `2cea761ab943db4bc2d2256a38d827bf1cb47397` | `5d641aa956c9bec4d91670e2a91e367ec583223a` | Reviewed separate build/sign/publish jobs |
| Build stack | .NET Windows | Rust cross-platform CI, Windows release | Rust/Tauri four-platform CI/release preparation |
| Windows signer | Azure Artifact Signing + OIDC | Same provider | Prepared same provider, not authorized or executed |
| Signing environment | `artifact-signing` | `artifact-signing` | `release-windows` |
| Signing profile variable | `AZURE_SIGNING_PROFILE` | `AZURE_SIGNING_PROFILE` | `AZURE_CERTIFICATE_PROFILE` |
| Publisher variable | `AZURE_SIGNING_PUBLISHER` | Absent | `WINDOWS_SIGNER_SUBJECT` (exact certificate subject) |
| Published release evidence | [successful run](https://github.com/kc2-io/BotOrNot/actions/runs/35245721883) | [successful run](https://github.com/kc2-io/mpd-bot/actions/runs/35138124047) | No release/tag created |

Both references have the same non-secret tenant, subscription, regional endpoint,
signing account and certificate profile, but **different Azure client IDs**.
Do not blindly reuse one application's trust. Resolve the authorized application
and narrow repository/environment trust with the Azure administrator after the
protection gate is satisfied. Other MPD Viewer Azure variable names match the
reference names. No Apple signing configuration was present in the inspected
reference environments; Apple credentials/team remain an independent prerequisite.

Downloaded, without executing, the existing
[mpd-bot v0.2.0 release](https://github.com/kc2-io/mpd-bot/releases/tag/v0.2.0).
ZIP SHA-256 matched its GitHub asset digest:
`affc59c20335110b2326ed01ea978d3cd728145cd151c4ce8fefb7c040936a83`.
Windows reported its executable's Authenticode signature **Valid**, with a
Microsoft timestamp and signer subject:

```text
CN=Kenneth Caruso, O=Kenneth Caruso, L=Fall City, S=wa, C=US
```

That is evidence for the reference release, not an MPD Viewer signature. Confirm
the selected current certificate profile still expects that exact subject before
setting MPD Viewer's `WINDOWS_SIGNER_SUBJECT`.

## Actual remote writes and protection blocker

Verified repository private; configured issues on, wiki/projects off, squash-only
merge, delete merged branches, no auto-merge, default workflow permissions `read`,
and workflow PR approvals disabled. Source/history pushed normally to `main`.
Both `RELEASES_ENABLED` and `STABLE_RELEASES_ENABLED` are **false**.

The organization reports plan `free`. Actual attempted protection writes:

| API request | Actual outcome |
|---|---|
| Main branch protection, required source/native checks, no force/delete, PR-only | HTTP 403, plan upgrade/public visibility required |
| Immutable `v*` tag ruleset | HTTP 403, plan upgrade/public visibility required |
| Required reviewer environments (`release-windows`, `release-macos`, `release-publish`) | HTTP 422, billing plan does not support required reviewers |

The failed environment requests **partially created three empty environments**.
Readback shows no protection rules and no deployment branch policy. They are not
approved signing environments. No environment variables or secrets, Azure trust,
or Apple material was provisioned. No gates were weakened to accept this state.

GitHub's [branch protection documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
and [ruleset documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
describe private-repository plan restrictions. The
[environment protection documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments#required-reviewers)
states that required reviewers are public-repository-only on Free, Pro and Team.
Therefore, a Team upgrade alone does not satisfy all currently documented gates.
No plan, billing or repository-visibility change was made.

## Remaining gates, in order

1. Choose a supported private-repository protection arrangement. Retaining the
   current GitHub required-reviewer design requires an eligible plan; an alternative
   design needs explicit approval and independent review. Do not remove gates,
   switch public, or change billing automatically. Keep releases disabled meanwhile.
2. After that choice, reconcile the existing empty environments (do not rerun the
   create helper), apply required checks/reviews/tag policies, and read them back.
3. Configure the existing authorized Azure signing profile with the appropriate
   application's narrow Certificate Profile Signer access and a federated credential:

   ```text
   issuer: https://token.actions.githubusercontent.com
   audience: api://AzureADTokenExchange
   subject: repo:kc2-io/MPD_Viewer:environment:release-windows
   ```

   Use the provider UI and GitHub environment settings, never chat for credentials.
   Do not modify trust for either reference repository or broaden organization-wide
   access. No existing Azure admin login or new federation was verified this turn.
4. Supply the Apple Developer ID and issuer-based notarization configuration listed
   in `SIGNING.md` through secure provider/environment interfaces. No Apple enrollment
   or purchase is authorized. Actual app/DMG signing and notarization remain untested.
5. Native CI has passed; complete actual runtime and release-package
   verification. CI diagnostic ZIPs are not signed app bundles/installers.
6. Prepare a separately reviewed version change and obtain specific prerelease
   authorization before enabling the relevant flag, creating/pushing any release
   tag, or publishing. Stable acceptance remains separate.

No release tag, GitHub release, signed MPD Viewer artifact, Azure resource/trust,
Apple credential, or parent website deployment was created by this continuation.
