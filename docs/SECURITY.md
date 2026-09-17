# Security notes and POC boundaries

This is a source-level design/implementation record, not a completed security audit. Native enforcement and platform-specific behavior remain untested.

## Trust separation

The bundled `main` management window receives only the `manage` permission (`get_state`, `dispatch`). The remote player windows receive only `report-playback`. All custom commands are registered in the build-time AppManifest rather than relying on permissive defaults.

Every manager command checks the caller label. A player report must match its caller's `player-<session>` label. Reports are typed, finite/range-checked for volume, queued through a bounded channel, and rate-limited per session by the controller. They report advisory status only: a player cannot select channels, change preferences, read credentials, or open files through that command.

On platforms where iframe callers are not distinguishable from their containing webview, treat all player telemetry as untrusted. A compromised wrapper/subframe may falsify its own displayed playback state, but that state never drives stream selection or claims of viewer credit.

References: [Tauri capabilities](https://v2.tauri.app/security/capabilities/), [AppManifest](https://docs.rs/tauri-build/latest/tauri_build/struct.AppManifest.html).

## Credentials and privacy

Twitch OAuth access/refresh tokens remain in native memory and are not serialized, logged, passed to player URLs, or saved in SQLite. SQLite contains channel preferences, the public client ID, and nonsecret settings. Device activation codes are displayed intentionally during authorization.

Disconnect drops application ownership of credentials, but an in-flight request may retain its own reference until completion. Memory is not explicitly zeroized. OS/browser caches, process dumps, swap, and Twitch-managed cookies are outside this POC's guarantees. A compromised OS/local account is outside the threat model.

## Local and hosted player content

The loopback HTTP server exposes only the three bundled static wrapper assets. It binds to IPv4 loopback, checks the Host header, rejects non-GET requests, and has no control or credential endpoints. The manager uses the bundled custom protocol, not the loopback origin.

The player capability admits `http://localhost:*/*` for Demo/development and `https://parent.mpdviewer.com/*` for the owner-hosted Twitch wrapper. No arbitrary HTTPS host or subdomain wildcard is admitted. It grants no filesystem, shell, process, or management commands. Hosting the player wrapper creates a dependency on the integrity and availability of the owner-controlled website. Do not add analytics, arbitrary scripts, or broad native permissions to that origin.

The macOS plist declares local networking for the local experiment. It does not set `NSAllowsArbitraryLoads`, bypass certificate validation, or disable browser security. This plist's actual effect must be checked in a native macOS bundle. Reference: [Apple NSAllowsLocalNetworking](https://developer.apple.com/documentation/bundleresources/information-property-list/nsapptransportsecurity/nsallowslocalnetworking).

Network/TLS validation remains enabled. Arbitrary navigation, popups, and downloads are denied in managed player windows. Native enforcement, CSP compatibility, and origin matching must still be tested; static configuration tests cannot prove them.

## Platform and product boundaries

No visibility spoofing, synthetic viewing activity, account farms, ad blocking, player patching, or autoplay bypass is implemented. A manually paused player remains paused unless the user explicitly starts it or retries that session. Telemetry heartbeats do not simulate interaction.

Real Twitch playback and platform-policy suitability remain unverified. The source makes no promise of counted viewers, revenue, or any particular tab/volume accounting behavior. Review Twitch's current requirements before public distribution and do not use the application to evade platform restrictions.

References: [Twitch embedding](https://dev.twitch.tv/docs/embed/), [player SDK](https://dev.twitch.tv/docs/embed/video-and-clips/).
