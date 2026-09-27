# Tagged releases

> Current alpha scope: [ALPHA-RELEASE-PLAN.md](ALPHA-RELEASE-PLAN.md). The owner approved a manual-test multiplatform alpha: signed Windows, explicitly unsigned/unnotarized macOS, and explicitly unsigned Linux packages. Full signing requirements still apply to beta, release-candidate and stable releases.

## Gate status

The live repository has valid releases through the multiplatform manual-test
alpha.8, published September 27, 2026. Alpha.5 is a lightweight tag on
alpha.4-versioned source with an empty
manually created release, so it is not repaired or counted as release evidence.
Do not assume the next version or current live gate state from this document;
verify them with `just state` before release work.

## Version and tag contract

Keep `Cargo.toml` workspace version, `src-tauri/tauri.conf.json` version, and the tag (minus `v`) identical. Review/commit corresponding workspace-package version entries in Cargo.lock using Cargo, not ad-hoc text edits. Supported tags:

```text
v0.1.0-rc.1
v0.1.0-beta.2
v0.1.0-alpha.N
v0.1.0
```

Use an annotated tag identifying a reviewed commit on main. Build metadata and alternate prefixes are rejected by this first implementation. Tags never move; failed releases are investigated rather than repaired by force-moving a tag.

`RELEASES_ENABLED=true` permits configured releases; stable versions also require
`STABLE_RELEASES_ENABLED=true`. At the September 27 verification, prereleases
were enabled and stable releases remained disabled; both values are mutable and
must be checked live. The gate requires a committed nonempty Cargo.lock, a
numeric pinned Rust version and full commit-SHA Action references. Repository
identity and visibility must match `.github/release-policy.json`:
`kc2-io/MPD_Viewer`, public. Missing or mismatched policy data stops the release;
live identity and visibility are checked again before draft creation and
immediately before publication.

## Release sequence

Once the [desktop E2E rollout](DESKTOP-E2E.md) is accepted, run `Desktop E2E`
with extended cases on all four platforms against the exact candidate commit
before tagging. Record its run URL and commit alongside existing native/signing
evidence. Fixture GUI success does not replace live Twitch checks or signing
verification; E2E binaries are unsigned test tools and never release inputs.

1. Obtain explicit authorization for the candidate version and signing/artifact
   scope. Verify current repository, environment, and provider gates without
   changing reference repositories.
2. Prepare and merge a reviewed version PR, then run the extended four-platform
   Desktop E2E matrix against the exact merge commit.
3. On clean local `main` identical to `origin/main`, choose exactly one command.
   To create the tag locally without publishing it:

```text
just tag-release v0.1.0-alpha.N
```

To create and immediately push the tag in one step after separately confirming
the target commit and explicit push authorization:

```text
just tag-release-push v0.1.0-alpha.N
```

Do not run `just tag-release-push` after `just tag-release`; the helper rejects
an existing tag rather than moving it. The helper never edits versions, moves
tags, creates a repository, or enables release flags.

4. GitHub validates the approved repository identity and visibility, tag form/version/annotation, main ancestry and pins. Source checks and all four native builds run without signing secrets.
5. Approve the narrowly scoped environments when supported/configured. Windows signs/verifies. In full scope macOS signs/notarizes/staples both architectures; in the manual-test alpha scope it bundles explicitly unsigned/unnotarized apps without Apple credentials. Linux creates checksum-covered native packages.
6. Publication requires the complete matrix. It checks the exact file set, creates a draft, uploads all assets, downloads and hashes them, rechecks remote identity/visibility/tag, then publishes the verified draft. A failed verification leaves the draft unpublished. It does not overwrite an existing release, even an existing draft.
7. Download release assets from the public release; verify hashes, Windows publisher/timestamp, expected macOS Gatekeeper behavior for the declared trust state, Linux package installation and actual playback/lifecycle on representative machines. Record real outcomes separately from CI compilation.

## Expected release assets

For example `v0.1.0-rc.1`:

| File suffix/name | Contents / verification |
|---|---|
| `-Windows-x64.zip` | Timestamped Authenticode-signed executable, license, instructions; requires WebView2; ZIP distribution, no installer |
| `-macOS-arm64.dmg` / `-macOS-arm64-UNSIGNED.zip` | Full scope: signed/notarized/stapled Apple Silicon DMG. Manual-test alpha: unsigned, unnotarized app ZIP. |
| `-macOS-x64.dmg` / `-macOS-x64-UNSIGNED.zip` | Full scope: signed/notarized/stapled Intel DMG. Manual-test alpha: unsigned, unnotarized app ZIP. |
| `-Linux-x64.deb` / `-Linux-x64-UNSIGNED.deb` | Debian-family package; checksum, not native signed. Manual-test alpha uses the explicit `UNSIGNED` name. |
| `-Linux-x64.AppImage` / `-Linux-x64-UNSIGNED.AppImage` | Linux AppImage; checksum, not native signed. Manual-test alpha uses the explicit `UNSIGNED` name. |
| `-source.zip` | Exact tagged source archive |
| `-player-sources.zip` | Both bundled wrapper and supplied hosted HTML, origin config and protocol addendum; not a deploy action |
| `BUILD-METADATA.json` | Repository, tag, commit, run identity, Cargo.lock hash and binary/source hashes |
| `SHA256SUMS.txt` | Hashes of final distributed artifacts and metadata |

The first seven names begin `MPD_Viewer-v<version>`. The example version is illustrative. Source archives are generated from the same tagged Git tree; no untracked local audit files or signing keys are included.

## CI artifacts are not releases

PR artifacts use `MPD_Viewer-<platform>-pr<N>-UNSIGNED`; main artifacts use the commit instead of pr<N>. They are seven-day diagnostic binary ZIPs, not a signed macOS app, Linux installer, or release. Internal native transfers are retained one day; the release pipeline uses only artifacts from its own run. GitHub release assets remain attached to the public release. Source and diagnostic artifacts are now publicly accessible.

## Recovery

Do not rerun by overwriting a release, deleting safety checks or moving tags. Inspect a failed draft and provider logs without exposing credentials. Fix source/configuration in a reviewed commit and normally create the next prerelease number. Existing drafts are left for an explicit maintainer decision; helpers never delete releases automatically.

## Current limits

Unsigned PR builds remain expected. The release pipeline has published verified
manual-test alphas through alpha.8, including a timestamped signed Windows ZIP
and explicitly unsigned macOS/Linux packages. That evidence does not establish
Apple signing/notarization, stable-release readiness, live Twitch acceptance, or
future provider access. No auto-updater, GitHub Pages, parent-host deployment,
billing change, Linux native signature, or Windows installer is configured.
Never call hosted/native protocol behavior fixed merely because an artifact is
signed or published.
