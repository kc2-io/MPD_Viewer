# Core policy instructions

Read the root `AGENTS.md` and `docs/PRODUCT-CONTRACT.md` first.

- Keep this crate deterministic and independent of Tauri, browser, storage, and
  network concerns.
- Preserve ranking, capacity, timer fairness, deferral, failure recovery, and
  stop/close precedence unless the task explicitly changes their contract.
- Express policy changes with focused unit tests covering single and multiple
  slots, simultaneous events, stale/deferred sources, failed targets, and
  closing behavior as applicable.
- Run `just verify-core` for changes here. Controller integration may also
  require `just verify-native`.
