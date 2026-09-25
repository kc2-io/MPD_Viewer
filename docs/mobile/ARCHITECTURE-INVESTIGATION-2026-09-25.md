# Mobile architecture investigation — 2026-09-25

## Recommendation

Keep the existing Rust scheduling/controller core and use **Tauri 2 with one
mobile webview containing a DOM grid**. Treat phones and tablets as different
presentation classes:

- Tablet landscape can target four real 400×300 Twitch video embeds in a 2×2
  grid, before adding application chrome or chat.
- A phone can show at least four assignments in a 2×2 overview, but one selected
  video must expand to a compliant player size. Four simultaneously visible,
  Twitch-compliant videos do not fit a normal phone viewport.
- The requested audio + chat experience is represented in the POC, but it must
  remain a product experiment until Twitch exposes or approves a supported
  audio-only route. Do not hide, cover, move off-screen or shrink a video embed
  and call it audio-only.

This choice shares the current UI technologies, keeps selection in Rust, avoids
duplicating policy in Swift/Kotlin, and matches the hosted-source addendum's
Candidate A. Tauri supports Android and iOS from one codebase, using the system
Android WebView and WKWebView respectively. Its mobile multi-window support is
not the right grid mechanism on phones: Android typically puts additional
activities on the back stack, while iPhone scenes commonly replace the current
UI rather than appearing side by side.

Primary references:

- [Tauri 2 mobile support](https://v2.tauri.app/)
- [Tauri system webview versions](https://v2.tauri.app/reference/webview-versions/)
- [Tauri mobile multi-window behavior](https://v2.tauri.app/learn/mobile-multiwindow/)
- [Tauri mobile prerequisites](https://v2.tauri.app/start/prerequisites/)

## Twitch constraints that shape the product

Twitch requires HTTPS embed domains, an accurate `parent` domain, unobscured
approved player elements, and compliance with the documented minimum geometry.
The interactive video player requires at least **400×300 CSS pixels**. Twitch
also states that video does not autoplay on mobile without user interaction.
The combined video + chat embed has the same 400×300 minimum and changes from
side-by-side to stacked layout at narrow sizes.

The documented player controls include play/pause, mute, volume and quality,
but no audio-only rendition. Therefore the statement that supported audio-only
is unavailable is an inference from the current public embed surface, not a
claim about Twitch's private clients. The official chat-only iframe is supported,
but pairing it with audio still needs a legitimate audio source.

Primary references:

- [Twitch embedded-experience requirements](https://dev.twitch.tv/docs/embed/)
- [Twitch video size, mobile autoplay and player controls](https://dev.twitch.tv/docs/embed/video-and-clips/)
- [Twitch combined video + chat embed](https://dev.twitch.tv/docs/embed/everything/)
- [Twitch chat-only embed](https://dev.twitch.tv/docs/embed/chat/)

These rules create two release-blocking facts:

1. Four real Twitch players in a 2×2 arrangement require at least an 800×600
   content rectangle. "Even if they are small" cannot override Twitch's minimum
   size rule.
2. A no-video audio mode cannot be implemented by visually hiding an official
   player. Besides the policy issue, mobile visibility/media policies may pause
   it, so it would not be dependable.

## Options considered

| Option | Reuse / authority | Four-up behavior | Cost and risk | Decision |
|---|---|---|---|---|
| Tauri 2, one bundled UI webview plus one DOM player grid | Reuses Rust core and current HTML/CSS/JS. Rust passes an allowlisted assignment set. | Correct fit for tablet; phone uses four-assignment overview plus expanded compliant video. | Must solve HTTPS hosted wrapper, protocol, cookies/popups, resource pressure and background lifecycle with native evidence. | **Recommended.** |
| Tauri mobile multi-window / one webview per player | Rust ownership is natural, but every player is a native activity/scene. | Mobile OS does not compose four phone windows into an in-app grid. | High lifecycle and profile complexity; poor handset UX. | Reject for phone grid; possible later tablet experiment only. |
| Native SwiftUI + Jetpack Compose with Rust FFI | Maximum lifecycle and platform integration; Rust core can remain shared. | Still embeds Twitch in WKWebView/Android WebView and inherits all Twitch limits. | Two presentation implementations, two native bridges and significantly more test/release work. | Contingency if Tauri mobile runtime fails acceptance. |
| Flutter or React Native with WebView packages and Rust FFI | One UI codebase. | Same embed geometry and audio constraints. | Replaces working UI stack without removing the hard media risk; adds FFI/plugin lifecycle work. | Not justified for the first mobile build. |
| PWA / Capacitor shell | Fastest visual delivery. | Can render the same responsive grid. | Weakens Rust authority or requires WASM/server restructuring; background and secure native storage need plugins. | Useful only as a layout prototype. |

CEF and Electron are not mobile candidates for this task. A browser extension is
also not an iOS/Android application architecture.

## Functional coverage plan

| Desktop capability | Mobile route | Current evidence / remaining gate |
|---|---|---|
| Favorites, priority, enable/disable, capacity | Reuse `mpd-core` and controller actions | POC uses real `mpd-core` selection and retains session IDs on reorder. |
| Start, pause automation, stop, skip/retry, timers | Port controller state machine behind the mobile main webview | POC covers start/pause/stop; full controller integration and timers remain. |
| Twitch live-status OAuth | Reuse `mpd-twitch`; open system/browser authorization deliberately | Network/auth flow is not in this media-free POC. Mobile secure token storage remains. |
| Preferences | Keep the existing application identifier and schema; add Keychain/Keystore-backed sensitive storage | The POC preserves `com.modpackdad.mpdtabber.poc` but uses in-memory demo state. |
| Website viewer sign-in and chat identity | One stable WebView profile; constrain related popup navigation | Must be tested separately on Android and iOS. Monitoring OAuth is still distinct. |
| Global mute/volume and quality | Versioned hosted wrapper commands after user gesture | UI and bounded Rust state are present; real SDK behavior is untested on mobile. |
| Four simultaneous videos | 2×2 tablet grid at no less than 400×300 per video | POC validates layout only. Live playback/resource testing remains. |
| Four assignments on phone | 2×2 overview with one expanded video when selected | POC validates the 2×2 overview at 390 and 412 CSS-pixel widths. |
| Audio + chat, no video | Await a supported Twitch audio route or revise requirement | POC validates interaction only and labels all content simulated. |
| App background playback | Native Android/iOS lifecycle experiment | Unverified; never infer from desktop or browser mocks. |

## Production boundary and protocol

The checked-in `web/parent.mpdviewer.com/index.html` remains a provenance
snapshot and was not changed or deployed. Real Twitch media needs a new,
versioned HTTPS mobile wrapper on the approved parent host. Rust must pass the
complete assignment set and bind every telemetry report to the native-known
session/generation. The hosted document may render assignments; it may not pick
favorites, change capacity, or invoke manager commands.

The minimum protocol work before live media is:

1. Define a `mobile-grid-v1` hello/capability handshake and immutable wrapper
   build identifier.
2. Use query parameters only for bounded bootstrap data or establish a narrow
   native-to-host command after an origin/path check. Do not grant the remote
   page general manager capability.
3. Keep per-session `session_id`, generation, channel, desired volume, mute and
   playback intent in Rust. Channel strings sent from the page are claims, not
   authority.
4. Separate visual focus, audio focus and playback intent. A layout change must
   not silently resume a user-paused player.
5. Permit only exact Twitch video/chat navigation required for the assigned
   channel and actual parent hostname. Authentication popups receive no native
   commands.

## POC delivered here

`mobile-poc/` is a separate Tauri 2 package and generated Android Studio project.
It intentionally does not modify desktop lifecycle/protocol files or the hosted
source snapshot.

Implemented:

- Rust-owned demo favorite list and top-priority selection through `mpd-core`.
- Default four-session capacity, retained session IDs on priority reorder,
  start/pause/resume/stop, live/offline toggles, capacity, global volume/mute and
  bounded channel normalization.
- Phone-first 2×2 grid, safe-area handling and tablet sizing.
- Audio + chat interaction concept with four selectable audio sessions.
- Strict local CSP with `frame-src 'none'`; the POC makes no Twitch requests.
- Generated Android project using the existing application identifier.

Not implemented or claimed:

- Real Twitch video/audio/chat, monitoring authorization, website login, Turbo,
  viewer credit, background playback, secure credential persistence, timers,
  push notifications, store packaging or production signing.
- iOS generation/build/runtime; Tauri exposes its iOS CLI only on macOS and the
  current host is Linux.
- Android runtime execution; this host has no `/dev/kvm`, so an emulator cannot
  be run with normal acceleration here.

## Next acceptance experiment

1. On an Android host with KVM, install the debug APK and exercise rotation,
   pause/resume/background/kill, keyboard, safe areas and memory pressure.
2. On macOS, generate the iOS project, build for an iPhone and iPad simulator,
   then repeat the lifecycle checks.
3. Build a new local working wrapper (not the provenance snapshot), deploy it
   only after explicit authorization, and test one real video after a user tap.
4. Test four 400×300 videos on a tablet landscape emulator/device. Record CPU,
   memory, thermals, audio focus, cookie sharing, crash isolation and resize.
5. Test official combined embed login/chat popups on both platforms with a
   dedicated profile. Do not inspect or export cookies.
6. Resolve audio-only with Twitch-supported evidence. If none exists, adopt
   "four assignments + one expanded video/chat" on phones and remove the
   unsupported audio-only promise.
