# MPD Viewer v0.1.0-alpha.7

The owner authorized a new alpha release with artifacts for manual testing on
Windows, macOS and Linux. This is the first explicit multiplatform manual-test
alpha scope; the trust model and exact filenames are recorded in
`ALPHA-RELEASE-PLAN.md`.

The candidate is based on `origin/main` after the valid signed Windows-only
alpha.6 release. It keeps alpha.6's application behavior and adds the packaging,
metadata and workflow controls needed to distribute clearly labeled macOS and
Linux manual-test artifacts. The invalid empty alpha.5 release remains untouched
because release tags are immutable.

This preparation changes workspace/Tauri versions to alpha.7, Cargo-generated
local workspace lock versions, the committed alpha scope, release packaging,
release security tests and documentation. It preserves the app identifier,
preferences/profile identity, public Twitch client ID, parent host and action
pins. It does not deploy the website or alter Twitch registration.

Before tagging, the protected PR checks and an extended four-platform Desktop E2E
run must pass on the exact merged commit. The tagged workflow must build all four
native targets, sign and verify Windows, bundle clearly marked unsigned macOS apps
and Linux packages, and publish only the exact verified set after environment
approval. Record the PR, E2E run, tagged release run, artifact hashes and manual
platform results as separate evidence.
