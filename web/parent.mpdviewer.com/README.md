# Supplied hosted player — source snapshot

This folder preserves the single HTML document the owner pasted into the conversation on September 17, 2026, identifying it as the source of `https://parent.mpdviewer.com/`.

**This is not a live-site download and is not a replacement for the bundled POC wrapper.** The pasted Markdown table was converted into HTML; server bytes, HTTP headers, TLS, browser login, playback and native integration have not been verified. `SOURCE.json` records the normalization and the reconstructed file's SHA-256 hash.

## What was preserved

The original title and comments still say MPD Tabber. Inline CSS and JavaScript, the Twitch script reference, and the Cloudflare script reference, integrity attribute and beacon configuration are retained. The embedded scripts were not downloaded or executed. The Cloudflare tag might be deployment-injected; the supplied text alone does not establish its origin. Its presence does not mean it should be included in a future privileged production player page.

Markdown table padding, punctuation escapes and link formatting were removed. Rows containing `<br>` were interpreted as blank source lines rather than literal elements inside JavaScript. Whitespace and line-ending normalization mean this is not a byte-for-byte archival claim about the live website.

The supplied page is self-contained apart from its two external script references. It does **not** require separate `player.js` and `style.css` assets. The prior three-file importer is not appropriate for this document.

## Important boundary

Do not deploy this archival snapshot or overwrite `player-wrapper/` automatically. The host and the uploaded POC have incompatible bootstrap, audio and telemetry contracts. See `../../docs/HOSTED-SOURCE-REVIEW.md` and `../../docs/HOSTED-SOURCE-PLAN-ADDENDUM.md` before implementing changes.

The native application source and its configuration are unchanged by this import. The website was not changed. The earlier feature plan remains a plan, not an implemented release.

## Offline characterization

From the repository root:

```sh
node --test tests/hosted-source-characterization.test.cjs
```

These tests deliberately document the supplied behavior, including defects. They execute the inline application script in a JavaScript VM with a fake DOM, Twitch API and IPC. They do not load either external script or prove browser rendering, autoplay, account recognition, Turbo behavior, security enforcement or native playback.

When implementing a corrected player, leave this snapshot immutable and test the new implementation separately. A passing characterization test that demonstrates an unsafe behavior is not a security acceptance test.
