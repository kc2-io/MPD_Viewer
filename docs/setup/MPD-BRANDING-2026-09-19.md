# Modpackdad branding

User-requested follow-up to the unified connection branch, base `00642d4ecaee5fb97f7d4607d9571b0396c280d1`. Coordinator owns edits; `setup_review` independently reviewed read-only with inherited model/effort settings and no override.

The supplied transparent MPD logo is preserved byte-for-byte at `ui/assets/mpd-logo.png` (SHA-256 `cf5f093a3c1b31e62c3b8629f8560c70dc565c4aeb19bc198ed171cfa4fd71f6`). The manager displays it at 56×56 using `object-fit: contain`, replacing the M placeholder. Its original aspect ratio and artwork are unchanged.

The app-owned manager, bundled player and injected chat toolbar use the logo's cyan `#00D0FF`, near-black `#0A1116`, dark slate panels `#111E25`, light text `#EAF6FA`, and muted text `#A8BDC6`. Existing green/red status meaning remains. Official Twitch iframe content and the hosted source snapshot/site were not modified.

Desktop icons were packaged with Tauri CLI 2.11.4. The original was proportionally fitted into a transparent 960×960 canvas and padded 32px on each side to make the committed 1024×1024 `src-tauri/icons/source.png`; no artwork was redrawn or recolored. Regenerate formats with `npx --yes @tauri-apps/cli@2.11.4 icon src-tauri/icons/source.png --output <temporary-output-directory>`, then copy `icon.png`, `icon.ico` and `icon.icns` into the configured icon directory. No new runtime dependency or bundle identity change.

Verification:

- User source and committed UI asset SHA-256 match.
- Manager browser check passed existing unified connection assertions and confirmed the logo loaded at its original 995px width. Screenshots at 1280px, 860px and 430px were captured; desktop/narrow previews were visually reviewed and no horizontal overflow was detected. These use simulated state, not live account data.
- `node tests/chat.browser.cjs`: hosted, bundled and Demo passed CSP/layout/context-retention checks, including 430px player geometry, no resumed playback, and no Twitch requests in Demo. Twitch responses are mocked.
- `cargo build --locked --release -p mpd-tabber --features custom-protocol`: optimized Windows build passed with replacement desktop icons.
- Independent review approved; representative text/background contrast exceeds 4.5:1, and `git diff --check` passes.

The unsigned branding preview is separate from the running earlier test build. Real combined Twitch activation acceptance remains pending under the unified-connection evidence record. No new signed release, tag or website deployment is included.
