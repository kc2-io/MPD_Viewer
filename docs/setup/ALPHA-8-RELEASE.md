# MPD Viewer v0.1.0-alpha.8

The owner authorized a new alpha release with downloadable artifacts for manual
testing on Windows, macOS and Linux. It retains the approved multiplatform alpha
trust model: signed and timestamped Windows x64, explicitly unsigned and
unnotarized macOS arm64/x64 app ZIPs, and explicitly unsigned Linux x64 DEB and
AppImage packages with SHA-256 checksums.

The candidate is based on `origin/main` at
`1b8e27cb2f9de67e00b6cdec700461d0b30e915e`, after alpha.7. Since alpha.7 it adds
native mute controls for full-page viewer windows, moves current-session status
to a more visible manager location, displays the configured rescan interval,
adds cross-platform GUI video/screenshot evidence, refreshes locked application
and GitHub Actions dependencies, and hardens Windows toolchain installation and
driverless policy-probe startup.

This preparation changes the workspace and Tauri versions to alpha.8, updates
only Cargo-generated local workspace versions in the lockfile, refreshes current
alpha documentation, and updates generated release notes. It preserves the app
identifier, preferences/profile identity, public Twitch client ID, parent host,
release scope and signing policy. It does not deploy the website or alter Twitch
registration.

Before tagging, the protected version PR checks and an extended four-platform
Desktop E2E run must pass on the exact merged commit. The tagged workflow must
build all four native targets, sign and verify Windows, bundle clearly marked
unsigned macOS apps and Linux packages, and publish only the exact verified set
after environment approval. The published asset hashes and Windows trust state
must be verified separately from later manual runtime testing.
