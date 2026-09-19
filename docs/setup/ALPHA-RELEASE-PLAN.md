# Signed Windows alpha release scope

The owner authorized an alpha release, then explicitly approved releasing only the artifacts we can sign. `v0.1.0-alpha.1` therefore publishes a Windows x64 ZIP containing an Authenticode-signed and timestamped executable. macOS and Linux binary packages are excluded. Source archives, metadata and checksums remain supporting review/verification files; they are not represented as signed executables.

This supersedes the previous requirement to wait for Apple credentials for this alpha. `.github/release-scope.json` records `windows-alpha`; helpers reject that scope for beta, release-candidate or stable versions. Those releases require restoring `full` scope and satisfying all original platform requirements. `STABLE_RELEASES_ENABLED` remains false.

All four native builds remain required. Windows packaging must succeed after independent publisher, Authenticode status and timestamp validation. Only the alpha's macOS/Linux packaging jobs may be skipped. Publication checks the explicit job results and the exact asset set, uploads a draft, downloads and hashes every file, rechecks repository identity/visibility and the immutable tag, then publishes with prerelease status and `latest=false`. Environment reviewers, tag protections, OIDC scope and signing failures remain enforced.

Expected alpha assets:

- `MPD_Viewer-v0.1.0-alpha.1-Windows-x64.zip`
- `MPD_Viewer-v0.1.0-alpha.1-source.zip`
- `MPD_Viewer-v0.1.0-alpha.1-player-sources.zip`
- `BUILD-METADATA.json`
- `SHA256SUMS.txt`

Version preparation changes only Cargo workspace/Tauri versions and the three local workspace Cargo.lock entries to `0.1.0-alpha.1`; third-party dependencies and app identity are unchanged. Signing setup readback confirms exact repository/environment OIDC trust and the existing certificate-profile-scoped signer role. Actual MPD Viewer signing and publication results must be recorded after execution, not inferred from this plan.

Known application limits remain in the release notes: unsupported chat sign-in popups, unverified posting/login/Turbo/native grid, and the unresolved SDK-reported volume mismatch. Windows live video/chat display and pause retention were observed before this release. Cross-platform compilation does not establish playback/login acceptance. No parent website deployment is included.

Sequence: independently review this patch, pass the protected version PR checks, merge, enable only prereleases, create the annotated alpha tag on clean main equal to origin/main, run the protected release workflow, verify its signed Windows artifact before publication approval, then verify the published download's hashes and signature. Never move the tag, overwrite a release or substitute an unsigned runtime binary after a failure.
