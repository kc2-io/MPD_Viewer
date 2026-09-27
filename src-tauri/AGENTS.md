# Native application instructions

Read the root `AGENTS.md` and `docs/PRODUCT-CONTRACT.md` first.

- Rust remains authoritative for selection, assignments, session identity,
  lifecycle, persistence, permissions, and native window state.
- Preserve the stable bundle/application identity, preferences, browser profile,
  public Twitch client ID, and parent origin unless an approved migration exists.
- Treat remote strings, window labels, URLs, and web messages as untrusted input.
  Bind reports to the native surface/session/generation and reject stale or
  foreign callbacks.
- Login and popup surfaces receive no manager capabilities. Keep navigation,
  downloads, popups, and IPC least-privileged.
- Do not treat compilation or mocked bridges as proof of native login,
  persistence, audio, playback, or Twitch behavior.
- Run `just verify-native` for native/controller changes and add targeted runtime
  evidence when the acceptance criterion concerns an actual webview or OS API.
