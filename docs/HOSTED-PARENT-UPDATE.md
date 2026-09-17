# MPD Tabber — hosted parent setup

**Date:** September 17, 2026  
**Based on:** the supplied MPD-Tabber-POC-Configured.zip  
**Configured player URL:** `https://parent.mpdviewer.com/`

## What changed

The native player URL and its matching telemetry-only capability now use the owner-provided HTTPS host. Demo still uses the bundled localhost wrapper. The public Client ID remains `ha94kk20cfu1tp74pgg8isgi88cpo7`. The player-wrapper source itself is unchanged from the previous package.

`src-tauri/player-origin.json`:

```json
{
  "url": "https://parent.mpdviewer.com/"
}
```

The player capability keeps only `report-playback` on `player-*` windows, permitting `http://localhost:*/*` and `https://parent.mpdviewer.com/*`. Manager permissions are not granted to remote content.

The native window opens the actual hosted page as its top-level document. The wrapper constructs Twitch.Player with `parent: [location.hostname]`, producing `parent.mpdviewer.com`. It is not sufficient to name that domain from an unrelated local page.

## Required site contents

The separate `MPD-Tabber-Player-Wrapper.zip` contains exactly three files at its root:

```text
index.html
player.js
style.css
```

Serve those files at the site root:

```text
https://parent.mpdviewer.com/           -> index.html
https://parent.mpdviewer.com/player.js
https://parent.mpdviewer.com/style.css
```

A landing page alone will not implement the POC's fragment parameters, audio bridge, or telemetry. A compatible custom wrapper is possible but was not inspected here. Do not overwrite an existing custom implementation without comparing it first. No website upload or modification was performed in this update.

Use the final URL directly, with a valid HTTPS certificate and no login wall or redirect to another host. Return HTML, JavaScript, and CSS using their appropriate content types, rather than returning the root HTML for every missing asset. Keep assets from one wrapper version together and invalidate stale caches when updating.

Keep the supplied Content Security Policy. Review any host-added CSP rather than assuming the page meta policy can override it. The wrapper loads the official script at `https://player.twitch.tv/js/embed/v1.js` and embeds `https://player.twitch.tv`; native telemetry additionally needs the IPC transports already listed in the supplied policy. Do not add analytics or arbitrary third-party scripts to this dedicated origin. It should not receive OAuth access tokens, refresh tokens, or client secrets.

The supplied CSS reserves at least 400 × 300 pixels for the player. Twitch separately requires unobscured approved player elements. Do not overlay the video with application UI or use hidden-player tricks to force autoplay.

Reference: [Twitch embedding requirements](https://dev.twitch.tv/docs/embed/), [Twitch player reference](https://dev.twitch.tv/docs/embed/video-and-clips/).

## Apply to an existing checkout

The new source archive is already configured. To preserve other changes in your own checkout and apply just this setting, run from the project root:

```powershell
py -3 scripts/set-player-url.py https://parent.mpdviewer.com/
cargo run -p mpd-tabber --features custom-protocol
```

On macOS/Linux use `python3` instead of `py -3`. Configuration is compiled into the application, so an already-built executable must be rebuilt. No database migration is needed for this player URL.

## Browser smoke test

Open the following in an ordinary browser; replace `modpackdad` with a channel that is known to be live when testing:

```text
https://parent.mpdviewer.com/#channel=modpackdad&session=1&volume=25&muted=false&demo=false
```

A correct wrapper should display the requested channel and **Parent: parent.mpdviewer.com · https:**. When the official SDK is ready, requested audio is applied before a single initial playback attempt. Autoplay may be blocked; use the Twitch controls or **Try playback** rather than disabling browser protections. An offline channel will not prove live playback.

The ordinary browser is expected to say **Standalone page · no native telemetry**. Visiting the bare root without its fragment is expected to show **Invalid channel or session configuration**. Neither message alone means the hosting setup is broken.

The fragment carries channel/session/audio options, not account credentials. The native app supplies a unique session ID for each player. Browser tests do not authorize the app with Twitch.

## Native smoke test

After a successful native build, stop Demo and close its managed windows. Select Twitch, connect using the existing public Client ID, and authorize. Confirm the playback diagnostics show the HTTPS wrapper, add a known live favorite, and start with one player. Check actual video, requested audio, and recent advisory telemetry before expanding to three players.

Then verify priority replacement, unchanged session retention, blocked autoplay, manual pause, inactive/minimized windows, Stop, and Quit using `MANUAL-TESTS.md`. The remote wrapper should be unable to invoke manager commands. A hosting failure should surface as unavailable playback/telemetry, not silent localhost playback.

## What was and was not verified

51 non-native checks passed after this update: 12 Node presentation checks, 14 Python configuration checks, and 25 existing mock-browser checks. Five new configuration checks cover root-URL acceptance, synchronized compiled URL/permissions, applying the host without modifying manager access, removing it with `--local`, and rejecting insecure input without modifying files.

A further HTTPS-origin route-mock browser suite was added, but this environment blocked navigation with `ERR_BLOCKED_BY_ADMINISTRATOR` before any assertions ran. It is not a passing test and would not prove real hosting or playback even if it ran, because its page responses, Twitch SDK, and IPC are mocks.

The owner reported deploying the site. Attempts to fetch it here failed, including a container DNS resolution failure. Therefore this update does **not** verify the deployed HTML/assets, DNS propagation, certificate, response headers, actual Twitch player, or native integration. It does not establish that the site fails in the owner's browser. No Rust toolchain was available; no executable was built and the 21 existing native tests remain unrun.

HTTPS plus the actual parent domain addresses specific documented embed-origin requirements. It is not Twitch approval or evidence of viewer-count credit; size, visibility, autoplay, other platform requirements, and all native acceptance gates remain separate.

## Wrapper file hashes

SHA-256 of the unchanged local wrapper files, for comparing with your deployment:

```text
8bab8073497bfad9b6ec772346e901b4cf14f24d7b8a344773cf2b613011ac8f  index.html
4c258ecaa7c9293f1e522702a47a5bdcd5b2dca55e9ce8025e013a9d8f2ab45a  player.js
b25717e092d1a4751323c5c9ea3bc2d27b4030218652d49b6817ff79727ddde5  style.css
```
