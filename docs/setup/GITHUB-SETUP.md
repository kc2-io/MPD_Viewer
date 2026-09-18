# Private GitHub repository setup

## Scope and actual state

Target: **`kc2-io/MPD_Viewer`**, private. `kc2-io` follows the user's named reference projects; the bootstrap supports `--owner` to use a different verified account/organization.

This is a prepared local Git repository, not a completed remote setup. In this environment GitHub is not connected, GitHub CLI/Rust are unavailable, and container requests to GitHub failed name resolution. No repo, settings, secrets, Azure federated credentials, Apple credentials, tags, Actions runs or releases have been created. No website was changed.

The current workflow files in `kc2-io/BotOrNot` and `kc2-io/mpd-bot` could not be retrieved with authenticated access. The setup follows the requested general build -> sign/package -> tagged-release convention, but **does not claim to duplicate those current files or secret names**. Comparison remains a gate.

## What is already in Git source

Four native targets (Windows x64, macOS arm64, macOS x64, Linux x64), unprivileged pull-request artifacts, signing/publication job separation, private-repository checks, immutable-tag setup, a lock bootstrap, exact-pin resolver, release helper/tests, feature plans, and the untouched supplied HTML import.

Visible branding is changed to MPD Viewer and `View fav channels in priority`. The historical internal bundle ID and crate names remain. No chat/authentication/grid application changes are part of this infrastructure commit.

## Preferred connected execution

Connect GitHub with access to both reference repositories and permission to create a private repository in the selected owner. Inspect their actual `.github/workflows/`, Actions variables, environment/secret **names**, repository settings and signing provider configuration. Never try to extract secret values or copy third-party repo material indiscriminately.

Map the verified existing Windows signing convention into `release-windows`. Existing repository-scoped settings do not automatically apply to a new repository. Any existing organization secret/variable access should be expanded only to this named repository, not to all repositories. The private plan's branch/tag/environment protection support must also be verified.

## Equivalent local CLI path

Requirements: Git, authenticated `gh`, Python 3.11+, current stable Rust. Native compilation additionally requires the platform's Tauri prerequisites. The commands below are real actions when run, not actions performed in this handoff.

Restore the bundle to retain the supplied import history:

```powershell
git clone -b main .\MPD_Viewer-Repository.bundle MPD_Viewer
cd MPD_Viewer
git remote remove origin
```

The ZIP alternatively contains source only, without `.git`. Prefer the bundle to avoid inventing replacement history. Do not overwrite a newer working checkout.

Inspect reference workflows before any remote changes:

```powershell
py -3 scripts/bootstrap-github.py inspect
```

Their exact bytes and blob SHAs go to ignored `.audit-local/`; they are not automatically included in this repository or overwritten remotely. Read those files and adjust provider/environment names as necessary before continuing. The helper does not perform that semantic comparison for you.

Resolve reproducibility inputs locally:

```powershell
py -3 scripts/bootstrap-github.py prepare
```

This generates a real Cargo.lock when absent, preserves existing resolved dependencies, pins the selected installed stable Rust version, and resolves each allowlisted official Action tag to its actual commit SHA. Review the results and commit them with your Git identity:

```powershell
git add Cargo.lock rust-toolchain.toml .github
git commit -m "build: pin dependencies, Rust and GitHub Actions"
```

Run available native tests, then create the repository:

```powershell
py -3 scripts/bootstrap-github.py create
```

The helper refuses a dirty checkout, a non-main branch, or an existing `origin`. It does not replace repositories, force-push, delete anything, or enable releases. It reads the reference workflows again before creating anything.

It requests private creation, verifies the API privacy field before any source push, configures read-only workflow defaults and disables release flags, pushes `main`, and requests the following protections:

- PR-only changes to main, required source/native checks, strict up-to-date checks, resolved conversations, no force-push/delete and a linear history. Zero other-person approvals supports a solo maintainer; review requirements should be increased when collaborators join.
- `release-windows`, `release-macos`, `release-publish` environments, owner review, and tag-only `v*` deployment policies. Self-approval is allowed intentionally for a solo maintainer.
- `v*` tag update/deletion restrictions. The owner/reviewed maintainers still need controlled tag-creation permission.

All protection requests are **unverified until GitHub accepts and returns the intended settings**. The helper stops on any unsupported/denied setting rather than silently downgrading it. It leaves any already-created private repository and any already-pushed source in place. Inspect partial state before retrying; the `create` command is intentionally not an unsafe overwrite/resume operation.

Private-repository protection features depend on GitHub plan. No subscription, payment method, spending limit, visibility change, or plan upgrade is performed. If the plan rejects a required protection, explicitly choose a supported alternative before using signing credentials. Do not simply delete the protections to make setup pass.

## Bootstrap without local Rust

After private creation/push, manually run **Bootstrap dependency lock** on main. It produces `Cargo.lock` and `rust-toolchain.toml` as an artifact, not a commit. Review and commit them, run `pin-actions.py --write` with authenticated `gh`, review/commit the action pins, then rerun CI. The missing-lock native CI failure is intentional, not a passing build.

## Finish before first release

1. Confirm the new repo reports private, audit actual workflow/settings parity with the references, and verify branch/tag/environment protections.
2. Commit real dependency/toolchain/action pins. Fix any Rust compile, packaging or platform-specific errors exposed by the first Actions runs.
3. Follow SIGNING.md using existing authorized signing resources where appropriate. Verify expected publisher/team and narrowly scoped trust.
4. Perform a reviewed prerelease build/sign/notarize/download verification; do not label the POC production-ready merely because artifacts exist.

`RELEASES_ENABLED` and `STABLE_RELEASES_ENABLED` are both initialized to `false`. Enable prereleases only after signing/trust and native builds have been verified. Stable releases need separate application acceptance. There is no automatic website deployment or public GitHub Pages setup.

## Reference documentation

- [GitHub CLI repo create](https://cli.github.com/manual/gh_repo_create)
- [GitHub repository environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)
- [GitHub Actions secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use)
- [Cargo lockfile guidance](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html)
