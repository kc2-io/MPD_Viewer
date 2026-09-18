# Hosted player black-window correction

The Windows diagnostic build opened `https://parent.mpdviewer.com/` with channel
configuration in the URL fragment. The actual public hosted page, fetched on
September 18, 2026, reads `location.search` instead. No channel reached its
bootstrap, leaving an empty black grid rather than a Twitch player.

## Bounded native compatibility adapter

Production hosted viewers now send one assigned channel in the query string,
with fractional volume and `pauseInactive=false`. A deliberately invalid active
channel sentinel keeps the legacy page's active-player handler muted. The native
adapter requests volume before mute through the documented
SDK methods. It never calls play, reload, or the legacy volume helper during an
audio update. Demo and local fallback retain their existing fragment protocol.

The native initialization script runs only in the assigned top-level origin,
path and channel. It observes same-document events, binds reports to the native
session, blocks the legacy assignment-command vocabulary before the hosted
listener, and preserves unrelated Twitch SDK messages. Reports remain advisory;
manager capabilities and selection authority are unchanged. Readiness timeout
shows an error instead of silently leaving an empty viewer indefinitely.

This explicitly supports the existing legacy query host. It is not the full
versioned wrapper negotiation/grid/chat work described in the feature plan.
The hosted provenance snapshot and deployed website were not changed.

## Evidence and limits

- 97 source tests passed: 42 JavaScript, 14 configuration, 41 release helpers.
  Twelve adapter regressions execute the preserved host and adapter together
  with a fake SDK, capture ordering and queued messages. They are not media tests.
- 24 Rust tests passed, including production query and demo/local URL regressions.
- Clippy passed with the two pre-existing controller style warnings.
- Independent source/security review approved the adapter after correcting
  unrelated oversized SDK-message handling and preserving the existing titles.
- A temporary Windows/Tauri/WebView2 harness used the actual `Host::open`, live
  HTTPS host, current Twitch SDK, and an isolated diagnostic profile. Monstercat
  produced real READY/OFFLINE reports and its offline UI was observed by the
  owner. A live Shroud viewer produced real READY/PLAYING reports while muted.
- The temporary example initially lacked the Windows common-controls manifest;
  adding Tauri's standard manifest allowed the harness to run. The normal app
  already receives that manifest through its build. The harness is not shipped.
- **Native volume acceptance remains unresolved:** Twitch reported volume 0.5
  despite a requested 0.25, including a later native update and a direct public
  SDK volume-only request. A direct public SDK pause request did take effect.
  The adapter reports actual observations rather than substituting requested
  values. No automatic unmute, forced playback, internal Twitch API, or altered
  volume units were used to bypass this discrepancy. Global volume/mute behavior
  still needs further native acceptance with user interaction.

These checks establish that the missing-channel initialization fault is corrected
and real Windows playback can start. They do not establish viewer login/Turbo,
all channel/network conditions, macOS/Linux runtime playback, native grid/chat,
or complete release acceptance. Release flags remain disabled; no tag, release,
or website deployment is part of this change.
