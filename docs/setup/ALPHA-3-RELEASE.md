# MPD Viewer v0.1.0-alpha.3

The owner requested a release with signed Windows binaries after PR #16 merged.
Continue the approved Windows-only alpha scope: timestamped Authenticode-signed
Windows x64 executable in a ZIP plus source/player-source archives, metadata and
checksums. All four native build gates, protected signing/publication environments,
immutable tags and publisher/timestamp verification remain required. Stable releases
remain disabled. No website deployment or macOS/Linux binary publication.

Base main: e9fb3d2d11c8989e71e3e122cc87774482636f8f. PR #16 and this main commit have
successful source and all four native checks. Preparation changes workspace/Tauri
versions, Cargo-generated local workspace lockfile versions and release notes only.
Third-party dependencies, application identity and release controls are unchanged.

Since alpha.2: drag channel rows/grips to adjust ranking while keeping up/down
arrows, insertion feedback and safe saves (PR #15); viewer counts in channel rows
and active viewer title bars, with stale/unavailable handling (PR #16). Automated
tests and local native compilation passed. Browser tests use mocked native state;
actual Windows dragging and live native title updates remain acceptance items.
The user reported alpha.2 working well; that does not establish those new behaviors.

Sequence: independently review and merge this version PR after all protected
checks pass; create annotated v0.1.0-alpha.3 from clean main matching origin/main;
verify all release builds, approve Windows signing, download and independently
verify the signed candidate before publication approval, then verify the published
asset set, checksums, metadata and Windows signature. Never move a tag, overwrite
a release, bypass a failed gate or substitute an unsigned executable.
