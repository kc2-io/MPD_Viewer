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

Expected `v0.1.0-alpha.7` assets:

- `MPD_Viewer-v0.1.0-alpha.7-Windows-x64.zip`
- `MPD_Viewer-v0.1.0-alpha.7-macOS-arm64-UNSIGNED.zip`
- `MPD_Viewer-v0.1.0-alpha.7-macOS-x64-UNSIGNED.zip`
- `MPD_Viewer-v0.1.0-alpha.7-Linux-x64-UNSIGNED.deb`
- `MPD_Viewer-v0.1.0-alpha.7-Linux-x64-UNSIGNED.AppImage`
- `MPD_Viewer-v0.1.0-alpha.7-source.zip`
- `MPD_Viewer-v0.1.0-alpha.7-player-sources.zip`
- `BUILD-METADATA.json`
- `SHA256SUMS.txt`

Publication requires all four native release builds, signed Windows packaging,
both unsigned macOS app bundles, both Linux packages, the exact asset set, a
download/hash round trip, live repository/tag rechecks and protected publication
approval. A failed or partial matrix remains unpublished. No workflow may replace
a failed signed Windows binary with an unsigned one.

`v0.1.0-alpha.6` was published as the latest valid signed Windows-only alpha
while this scope change was being prepared. It is preserved. The next candidate
is alpha.7; no existing tag or release is moved or overwritten.

## Sequence and acceptance boundary

Prepare and merge the protected alpha.7 version/scope PR, then run the extended
Desktop E2E matrix on the exact merged commit. On clean `main` equal to
`origin/main`, create the annotated alpha.7 tag with `scripts/tag-release.py`.
Approve the Windows signing and publication environments only after their
candidates and workflow state are reviewed. Verify the published checksums and
Windows signature independently after download.

The release provides binaries for manual platform testing; compilation and local
fixture E2E are not claims of live Twitch playback, login, Turbo/reward credit,
native OS input fidelity or cross-platform credential persistence. Known product
limits remain in the generated release notes. No parent-site deployment is part
of this release.
