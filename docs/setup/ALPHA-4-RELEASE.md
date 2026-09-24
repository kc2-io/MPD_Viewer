# MPD Viewer v0.1.0-alpha.4

The owner authorized merging the completed viewer work and cutting the next tag
on 2026-09-24. Remote releases/tags were checked: alpha.3 is latest, and alpha.4
was absent. Continue the established signed Windows-only alpha scope with source
archives, metadata and checksums. Do not publish unsigned macOS/Linux binaries,
change signing controls, enable stable releases, move tags or deploy the website.

The feature integration is PR #20. Its source and all four native platform checks
passed on the implementation tree, with final documentation checks rerunning.
Windows native acceptance is recorded in VIEWER-INTEGRATION-VERIFICATION.md.

Since alpha.3: default full Twitch channel pages with an embedded launch flag;
Twitch-owned remembered Dark Theme and media controls in web mode; optional
per-channel assignment timers; protected Windows monitoring-authorization
persistence; and the earlier preferred-resolution/system-appearance changes.

This preparation changes workspace/Tauri versions, Cargo-generated local workspace
lock versions, release-note text and this record only. Third-party dependencies,
app/profile identity, public client ID, scope and workflow permissions are unchanged.

Sequence: merge protected feature PR, base the reviewed version PR on resulting
main, pass all protected checks, merge the version PR, then create annotated
v0.1.0-alpha.4 on clean main identical to origin/main using scripts/tag-release.py.
Verify all release builds, approve existing Windows signing environment, download
and independently check the signed candidate publisher/timestamp before approving
publication, then verify the published assets, metadata, hashes and signature.
The existing environment reviewer account may approve only after those checks;
no environment protection is removed or bypassed.

Windows runtime checks do not establish rewards credit or macOS/Linux runtime
acceptance. See WEB-VIEWER.md for the Twitch navigation/raid assignment boundary
and supported controls, and feature verification notes for remaining unobserved
network-outage, OS sleep, explicit real-credential deletion and fresh-profile cases.
