# MPD Viewer repository rules

Rust is the selection/controller authority. Preserve the existing app identifier,
preferences paths, browser-profile identity and public Twitch client ID.
The pasted hosted source in web/parent.mpdviewer.com is a provenance snapshot:
do not silently substitute it for player-wrapper or deploy it.
Read docs/HOSTED-SOURCE-PLAN-ADDENDUM.md before grid, chat or protocol work.
Read docs/feature-plans/AGENT-RULES.md and assign exclusive ownership for shared files.

Build/release rules:
- Never put tokens, cookies, signing keys or certificate passwords in source/logs.
- PR builds are unsigned and cannot access release environments or OIDC tokens.
- Release tags are immutable vMAJOR.MINOR.PATCH[-alpha.N|-beta.N|-rc.N].
- Tagged releases require a committed lockfile, a pinned toolchain and action SHAs.
- Do not make the repository public, push tags, publish releases, or deploy the site
  without an explicit task authorizing that operation.
- Record native compilation, signing and runtime tests separately; mocks are not
  evidence of working authentication, Turbo, chat or a native grid.
- Do not change expected-success gates to pass an unverified release.
