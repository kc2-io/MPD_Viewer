# Twitch web viewer

The normal build opens full Twitch channel pages by default. The older embedded
viewer remains available in the same binary only with `--embedded-viewer`:

```powershell
.\MPD_Viewer.exe --embedded-viewer
```

The choice lasts for that process, is not saved, and is not exposed in settings.
Demo mode still uses the local simulated player in either launch mode. Full Twitch
pages receive no MPD native commands or reporting permission. Existing application,
preferences and WebView2 profile identities are preserved.

## Appearance and player controls

MPD's manager and window chrome follow the operating-system theme. For full Twitch
pages choose the avatar menu, then **Dark Theme**. Twitch owns and remembers that
website preference in the existing browser profile. This is the user-approved web
mode behavior; MPD does not promise automatic OS matching inside Twitch pages.
The native Windows spike was observed changing its full page and chat from light
to dark using this official switch on 2026-09-24. New-build restart verification
is tracked separately in the integration acceptance record.

Use the Twitch player's volume slider and Settings > Quality in each web viewer.
Twitch documents programmatic volume/quality methods for its embedded SDK, not
external control of its full website. MPD therefore keeps central volume/quality
controls for embedded/demo mode only. It does not inject player scripts, access
private APIs, or alter the Twitch page to simulate those controls.

Tauri/WebView2 already passes the native preferred color scheme to pages, including
OS theme changes. That does not override Twitch's own explicit appearance choice.
No cookie copying, local-storage rewriting, CSS injection or appearance automation
is used by the application.

## Assignment boundary

Twitch can navigate within its own website, including links to another channel or
raids. MPD currently manages the original channel assignment: its title, audience
count and timer refer to that assignment, not an observed identity of whatever
Twitch subsequently displays. Use the manager to choose channels; Retry returns
a viewer to its assigned channel. The HTTPS origin allowlist prevents arbitrary
external navigation but does not lock Twitch’s own single-page routing. A native
URL-observer policy and raid acceptance are separate follow-up work; no Twitch DOM
inspection is used to infer or force playback identity.

## Authorization and rotation

On Windows, monitoring authorization is stored in Windows Credential Manager and
validated on startup. An installation upgrading from the old memory-only behavior
needs one final authorization; subsequent successful restarts should restore it.
See [persistence details](AUTH-PERSISTENCE.md). Website sign-in remains Twitch-owned
and separate from permission to use the monitoring API.

A favorite can have an optional rotation timer. It measures time assigned to an
MPD viewer while automation is running, not Twitch credited watch time. Twitch
alone determines points, streaks and other eligibility; a full page is not a
guarantee of credit.

## References

- [Twitch embedded player API](https://dev.twitch.tv/docs/embed/video-and-clips/)
- [Twitch terms of service](https://legal.twitch.com/en/legal/terms-of-service/)
- [WebView2 preferred color scheme](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2profile)

Documentation review identifies supported integration boundaries; it is not a
legal certification. macOS/Linux behavior requires their own native acceptance.
