# MPD Viewer project state

Verified: 2026-09-27 (US/Pacific)<br>
Verified `origin/main`: `4ae62288b2ecbb9e7b792b8738cb6cf6822e4549`

This is an orientation snapshot, not live authority. Before relying on mutable
facts such as open pull requests, repository settings, workflow results, release
flags, or artifacts, verify them with `just state` and the relevant GitHub view.
Update this file when a merged task changes the summarized product state.

## Repository and release

- Repository: public `kc2-io/MPD_Viewer`, default branch `main`.
- Newest published prerelease: [`v0.1.0-alpha.8`](https://github.com/kc2-io/MPD_Viewer/releases/tag/v0.1.0-alpha.8), published 2026-09-27.
- Alpha.8 contains a signed Windows x64 ZIP and explicitly unsigned macOS
  arm64/x64 and Linux DEB/AppImage manual-test artifacts, plus checksums,
  metadata, sources, and player sources.
- Prerelease publication is enabled. Stable releases remain disabled and retain
  the full signing/acceptance requirements in `docs/setup/RELEASING.md`.
- The repository has native CI on Windows x64, macOS arm64/x64, and Linux x64,
  plus desktop GUI fixture E2E on the primary three platforms and extended
  coverage including Intel macOS.

## Implemented product behavior on the verified base

- Full Twitch channel pages are the default viewer; `--embedded-viewer` selects
  the retained embedded fallback.
- Rust owns ranked favorite selection, viewer capacity, timers, assignments,
  persistence, and native window lifecycle.
- The manager supports drag ranking, viewer counts, configurable live-status
  rescans, per-channel timers, system appearance, and persistent Windows
  monitoring authorization.
- Embedded viewers provide official chat and supported wrapper volume/quality
  controls. Full-page viewers use Twitch's own media controls; Windows and Linux
  also provide confirmed native whole-webview mute. macOS reports that native
  full-page mute is unsupported.
- Desktop E2E uses controlled local fixtures and produces bounded screenshots,
  videos, manifests, logs, and hashes. It does not automate live Twitch login,
  playback, rewards, or viewer credit.

See `README.md` and the focused records under `docs/setup/` for details and
evidence boundaries.

## Active and proposed work at verification time

- [PR #38](https://github.com/kc2-io/MPD_Viewer/pull/38) proposes restarting a
  timed assignment when no eligible alternative favorite is live. It is not part
  of the verified base until merged.
- Native grid presentation remains a proposal/experiment in
  `docs/feature-plans/`; it is not implemented product behavior.
- Mobile emulator work exists on feature branches as a proof of concept and is
  not part of desktop `main` or a supported mobile release.

## Known boundaries

- A full Twitch channel page does not expose supported application controls for
  central volume, quality, or exact bitrate. Do not inject scripts to add them.
- Monitoring OAuth is distinct from the browser-profile website session.
  Account recognition, rewards, Turbo, and viewer credit require live evidence
  and remain Twitch decisions.
- The hosted source snapshot is not the deployed-site authority and is not
  protocol-equivalent to every bundled/native path.
- Apple Developer ID signing/notarization, stable-release enablement, automatic
  updating, and website deployment are not part of the current alpha scope.
