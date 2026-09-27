# Desktop E2E instructions

Read the root `AGENTS.md`, this directory's `README.md`, and
`docs/setup/DESKTOP-E2E.md` before changing the harness or workflow.

- The harness may exercise production Rust/controller behavior only through the
  explicitly gated `e2e-tests` build. Ordinary builds must exclude its driver,
  fixture, and credential hooks.
- Create isolated marked roots/profiles and clean up only owned processes and
  paths. Never upload profiles, databases, credentials, raw URLs, or unbounded
  logs.
- Preserve nonzero failure status. Retries may cover only narrowly classified
  infrastructure failures and must never mask policy/assertion failures.
- Screenshots and videos are revision-bound fixture evidence, not proof of live
  Twitch playback, authentication, rewards, or OS-input fidelity.
- Run `just verify-e2e` for harness source changes. `just e2e-run` requires the
  documented native binary, driver, display, and evidence environment.
