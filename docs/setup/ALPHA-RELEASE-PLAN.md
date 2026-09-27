# Multiplatform manual-test alpha release scope

The owner explicitly requested a new alpha with artifacts for manual testing on
Windows, macOS and Linux. The repository still has no Apple Developer ID or
notarization credentials, so this scope makes the platform trust differences
visible instead of representing unsigned files as production-ready packages.

`.github/release-scope.json` records `multiplatform-alpha`. Helpers accept this
reduced-signing scope only for alpha versions. Beta, release-candidate and stable
versions still require the `full` scope and all original signing/notarization
gates. `STABLE_RELEASES_ENABLED` remains false.

## Trust and artifact contract

- Windows x64 is Authenticode-signed and timestamped through the existing
  protected `release-windows` environment. Its publisher, status and timestamp
  must pass the independent workflow check before packaging.
- macOS Apple Silicon and Intel artifacts are app bundles in ZIP files. They are
  explicitly named `UNSIGNED`, are not notarized, do not use the protected Apple
  signing environment, and are only for manual alpha testing.
- Linux x64 DEB and AppImage artifacts have SHA-256 checksums but no OS-native
  publisher signature. Their filenames explicitly include `UNSIGNED` for this
  manual-test alpha.
- Source archives, metadata and checksums remain supporting verification files.

Published `v0.1.0-alpha.8` assets, which illustrate the current alpha contract:

- `MPD_Viewer-v0.1.0-alpha.8-Windows-x64.zip`
- `MPD_Viewer-v0.1.0-alpha.8-macOS-arm64-UNSIGNED.zip`
- `MPD_Viewer-v0.1.0-alpha.8-macOS-x64-UNSIGNED.zip`
- `MPD_Viewer-v0.1.0-alpha.8-Linux-x64-UNSIGNED.deb`
- `MPD_Viewer-v0.1.0-alpha.8-Linux-x64-UNSIGNED.AppImage`
- `MPD_Viewer-v0.1.0-alpha.8-source.zip`
- `MPD_Viewer-v0.1.0-alpha.8-player-sources.zip`
- `BUILD-METADATA.json`
- `SHA256SUMS.txt`

Publication requires all four native release builds, signed Windows packaging,
both unsigned macOS app bundles, both Linux packages, the exact asset set, a
download/hash round trip, live repository/tag rechecks and protected publication
approval. A failed or partial matrix remains unpublished. No workflow may replace
a failed signed Windows binary with an unsigned one.

`v0.1.0-alpha.7` was the first valid multiplatform manual-test alpha under this
scope, and `v0.1.0-alpha.8` was published under the same scope. Both are
preserved. Determine any future candidate from current source and live releases;
no existing tag or release is moved or overwritten.

## Sequence and acceptance boundary

For a separately authorized future alpha, prepare and merge the protected
version PR, then run the extended Desktop E2E matrix on the exact merged commit.
On clean `main` equal to `origin/main`, choose one mutually exclusive command:
`just tag-release VERSION` creates a local tag without publishing, while
`just tag-release-push VERSION` creates and pushes the tag in one step and
therefore requires explicit push authorization. The push recipe rejects a tag
already created by the local-only recipe. Approve the Windows signing and
publication environments only after their candidates and workflow state are
reviewed. Verify the published checksums and Windows signature independently
after download.

The release provides binaries for manual platform testing; compilation and local
fixture E2E are not claims of live Twitch playback, login, Turbo/reward credit,
native OS input fidelity or cross-platform credential persistence. Known product
limits remain in the generated release notes. No parent-site deployment is part
of this release.
