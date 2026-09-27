# Twitch full-page volume and quality research

Research performed: 2026-09-25

Recorded: 2026-09-27

Repository state reviewed through: `4ae6228` (`v0.1.0-alpha.8`)

## Question

Can MPD Viewer set the volume and video bitrate or quality of a stream while
continuing to open the full Twitch channel page, without injecting JavaScript or
automating Twitch's page, and while staying within Twitch's documented integration
rules?

## Conclusion

No documented Twitch URL parameter sets arbitrary volume or video quality on a
full channel page such as:

```text
https://www.twitch.tv/{channel}
```

Twitch documents programmatic volume and quality controls only for its official
embedded player JavaScript API. The embed URL itself can request an initial muted
state, but it cannot set an arbitrary volume or quality. Consequently, MPD Viewer
cannot restore centralized volume and quality controls for full Twitch pages using
URL parameters alone.

The supported full-page behavior is:

- Let the viewer use Twitch's visible volume and **Settings > Quality** controls.
- Use a native whole-webview mute control on platforms whose public browser-engine
  API supports it. This silences output without inspecting or modifying Twitch's
  document.
- Retain centralized volume and preferred-quality controls only for the official
  embedded viewer and the simulated demo.

This is a technical review of documented interfaces, not a legal certification.

## Documented Twitch interfaces

### Full Twitch channel pages

Twitch does not document volume, mute, resolution, quality, or bitrate query
parameters for `www.twitch.tv/{channel}`. MPD Viewer's full-page mode therefore
launches the normalized channel URL without media-control parameters and grants
the remote page no MPD command capability.

The absence of a documented parameter does not prove that Twitch has no internal
implementation detail with similar behavior. It does mean MPD Viewer should not
depend on one as a supported public contract.

### Official player iframe URL

For `https://player.twitch.tv/`, Twitch documents these optional player URL
parameters:

- `autoplay`
- `muted`
- `time` for VOD content

The iframe also requires a channel, video, or collection and the genuine `parent`
domain. There is no documented `volume`, `quality`, `resolution`, or `bitrate` URL
parameter. Therefore a URL can request initial mute, but not 25% volume or 360p.

The source is Twitch's
[Embedding Video and Clips documentation](https://dev.twitch.tv/docs/embed/video-and-clips/).

### Official embedded-player API

The same Twitch documentation exposes supported JavaScript methods on an official
embedded player:

- `setMuted(boolean)`
- `setVolume(number)` with a value from 0.0 through 1.0
- `getQualities()`
- `setQuality(string)` using a quality identifier Twitch advertised for that
  player

This is the supported programmable route, but it requires using the Twitch embed.
It is not a supported API for controlling a top-level Twitch website document.
Using Twitch's documented SDK in an application-owned wrapper is different from
injecting code into `twitch.tv`, but it does not satisfy a requirement to remain on
the full channel page.

### Quality is not an exact bitrate setting

Twitch exposes available renditions such as Source, 720p60, or 360p rather than an
API for selecting an exact number of kilobits per second. Available renditions can
vary between broadcasts.

The embedded player reports the current playback bitrate through the
`playbackRate` field of `getPlaybackStats()`, but that field is observational; it
is not a bitrate setter. Twitch's viewer guidance likewise describes choosing a
quality rendition through the player's Settings menu. See Twitch's
[channel-page player guide](https://help.twitch.tv/s/article/a-tour-of-your-channel-page)
and [playback troubleshooting guide](https://help.twitch.tv/s/article/playback-issue-troubleshooting).

## Native controls that do not modify Twitch

Browser engines provide a narrower native control: mute the audio output of a
whole webview.

- Windows WebView2 exposes `CoreWebView2.IsMuted` / `ICoreWebView2_8::SetIsMuted`.
- Linux WebKitGTK exposes `webkit_web_view_set_is_muted`.
- The reviewed public macOS `WKWebView` API has no equivalent per-webview output
  mute property.

These APIs provide mute/unmute, not arbitrary volume, and they do not select a
video rendition. Microsoft describes the WebView2 boundary in its
[audio API overview](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/overview-features-apis?tabs=win32cpp),
and WebKitGTK documents its
[WebView mute property](https://webkitgtk.org/reference/webkit2gtk/stable/property.WebView.is-muted.html).

MPD Viewer now implements this native full-page mute boundary on Windows and
Linux in [`window_audio.rs`](../src-tauri/src/window_audio.rs). It deliberately
does not use Twitch DOM access, injected media-element code, or private browser
selectors. The complete behavior and failure contract is recorded in the
[full-page window mute plan](feature-plans/FULL-PAGE-WINDOW-MUTE-PLAN.md), and the
implementation was merged by
[PR #31](https://github.com/kc2-io/MPD_Viewer/pull/31).

Operating-system audio-session attenuation was not selected as a replacement for
the volume slider. Webview renderer/audio processes may be shared or reassigned,
making per-stream ownership unreliable; behavior is platform-specific; and the
Twitch UI would not reflect the externally applied volume. It would still provide
no quality control.

## Twitch policy boundary

Twitch's [embedding requirements](https://dev.twitch.tv/docs/embed/) require an
HTTPS host, the genuine `parent` value, approved and unobscured Twitch player
elements, and the documented minimum dimensions. The
[Twitch Developer Services Agreement](https://legal.twitch.com/en/legal/developer-agreement)
also requires the official embedded player for embedded Twitch video and prohibits
modifying or interfering with the player, its functionality, Twitch marks, or
advertisements.

Within those documented boundaries:

- The official embed SDK is the clearest supported route for programmable volume
  and rendition selection.
- Native whole-webview mute is outside the Twitch document and does not replace or
  obscure its player controls.
- Supplying a false `parent`, automating Twitch's DOM, injecting media-control
  scripts into the full page, calling undocumented private endpoints, rewriting
  playlists, or intercepting HLS renditions would not be an acceptable substitute.

Loading the full Twitch website in a native webview is not documented as a
programmable player integration. MPD Viewer therefore treats Twitch as the owner
of the page and its controls and does not claim that the webview container itself
constitutes Twitch approval.

## Decision matrix

| Viewer route | URL mute | Arbitrary volume | Quality selection | MPD approach |
|---|---:|---:|---:|---|
| Full `www.twitch.tv/{channel}` page | Not documented | Not documented | Not documented | Twitch UI plus supported native whole-window mute |
| Official `player.twitch.tv` iframe | Initial `muted` only | Official SDK | Official SDK | Existing `--embedded-viewer` path |
| Simulated demo | MPD-owned | MPD-owned | MPD-owned | Existing test/demo controls |

## Repository implications

The current architecture matches this result:

- Full-page mode constructs only `https://www.twitch.tv/{channel}` in
  [`player.rs`](../src-tauri/src/player.rs).
- Full-page windows do not receive the embedded player audio or quality commands.
- The manager hides volume and preferred-quality controls in full-page mode and
  exposes native mute only when the platform reports support.
- The exact `--embedded-viewer` launch flag retains the official embed path with
  centralized volume, mute, and preferred-quality controls.
- No hosted-site deployment, Twitch registration change, private API, or page
  injection is needed for this conclusion.

If Twitch later publishes a full-page media-control API or supported URL contract,
it can be reconsidered against the same requirements. Until then, adding
centralized full-page volume or quality controls would imply an unsupported
mechanism and should remain out of scope.
