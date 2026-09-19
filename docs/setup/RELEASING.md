# Tagged releases

> Current alpha scope: [ALPHA-RELEASE-PLAN.md](ALPHA-RELEASE-PLAN.md). The owner approved signed Windows-only alpha distribution; historical full-platform requirements below still apply to other release stages.

## Gate status

No release tag, native release build, signature, notarization, remote artifact or GitHub release has been produced by this source handoff. Release execution is disabled until explicitly configured.

## Version and tag contract

Keep `Cargo.toml` workspace version, `src-tauri/tauri.conf.json` version, and the tag (minus `v`) identical. Review/commit corresponding workspace-package version entries in Cargo.lock using Cargo, not ad-hoc text edits. Supported tags:

```text
v0.1.0-rc.1
v0.1.0-beta.2
v0.1.0
```

Use an annotated tag identifying a reviewed commit on main. Build metadata and alternate prefixes are rejected by this first implementation. Tags never move; failed releases are investigated rather than repaired by force-moving a tag.

`RELEASES_ENABLED=true` permits configured releases; stable versions also require `STABLE_RELEASES_ENABLED=true`. Both start false. The gate requires a committed nonempty Cargo.lock, a numeric pinned Rust version and full commit-SHA Action references. Repository identity and visibility must match `.github/release-policy.json`: `kc2-io/MPD_Viewer`, public, following the owner's explicit visibility change on September 18, 2026. Missing or mismatched policy data stops the release; live identity and visibility are checked again before draft creation and immediately before publication.

## Release sequence

1. Complete setup, signing configuration, workflow review and native build validation. Keep the reference repositories unchanged.
2. Prepare and merge a version PR. For the POC, a prerelease such as `0.1.0-rc.1` is preferable to implying production acceptance. This is a proposed first version, not a tag already created.
3. On clean local main identical to origin/main, create/push explicitly:

```powershell
py -3 scripts/tag-release.py v0.1.0-rc.1 --push
```

Without `--push`, the helper creates only the local tag. It never edits versions, moves tags, creates a repository or enables release flags.

4. GitHub validates the approved repository identity and visibility, tag form/version/annotation, main ancestry and pins. Source checks and all four native builds run without signing secrets.
5. Approve the narrowly scoped environments when supported/configured. Windows signs/verifies; macOS signs/notarizes/staples both architectures; Linux creates native packages.
6. Publication requires the complete matrix. It checks the exact file set, creates a draft, uploads all assets, downloads and hashes them, rechecks remote identity/visibility/tag, then publishes the verified draft. A failed verification leaves the draft unpublished. It does not overwrite an existing release, even an existing draft.
7. Download release assets from the public release; verify hashes, Windows publisher/timestamp, macOS notarization/Gatekeeper, Linux package installation and actual playback/lifecycle on representative machines. Record real outcomes separately from CI compilation.

## Expected release assets

For example `v0.1.0-rc.1`:

| File suffix/name | Contents / verification |
|---|---|
| `-Windows-x64.zip` | Timestamped Authenticode-signed executable, license, instructions; requires WebView2; ZIP distribution, no installer |
| `-macOS-arm64.dmg` | Apple Silicon app and signed/notarized/stapled disk image |
| `-macOS-x64.dmg` | Intel app and signed/notarized/stapled disk image |
| `-Linux-x64.deb` | Debian-family package; checksum, not native signed |
| `-Linux-x64.AppImage` | Linux AppImage; checksum, not native signed |
| `-source.zip` | Exact tagged source archive |
| `-player-sources.zip` | Both bundled wrapper and supplied hosted HTML, origin config and protocol addendum; not a deploy action |
| `BUILD-METADATA.json` | Repository, tag, commit, run identity, Cargo.lock hash and binary/source hashes |
| `SHA256SUMS.txt` | Hashes of final distributed artifacts and metadata |

The first seven names begin `MPD_Viewer-v<version>`. The example version is illustrative. Source archives are generated from the same tagged Git tree; no untracked local audit files or signing keys are included.

## CI artifacts are not releases

PR artifacts use `MPD_Viewer-<platform>-pr<N>-UNSIGNED`; main artifacts use the commit instead of pr<N>. They are seven-day diagnostic binary ZIPs, not a signed macOS app, Linux installer, or release. Internal native transfers are retained one day; the release pipeline uses only artifacts from its own run. GitHub release assets remain attached to the public release. Source and diagnostic artifacts are now publicly accessible.

## Recovery

Do not rerun by overwriting a release, deleting safety checks or moving tags. Inspect a failed draft and provider logs without exposing credentials. Fix source/configuration in a reviewed commit and normally create the next prerelease number. Existing drafts are left for an explicit maintainer decision; helpers never delete releases automatically.

## Limits of the initial configuration

Unsigned PR builds are expected. Signed release jobs, packaging commands and branch/environment APIs have not been run here. Exact provider settings must be compared with current BotOrNot/mpd-bot. No auto-updater, GitHub Pages, parent-host deployment, billing changes, Linux native signature or Windows installer is configured. Never call the POC's known hosted/native protocol mismatch fixed because a signed artifact exists.
