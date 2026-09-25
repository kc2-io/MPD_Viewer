# MPD Viewer

**View fav channels in priority**

Rust / Tauri desktop proof of concept for Windows, Linux and macOS. Maintains a prioritized list of favorite Twitch channels and user-configurable viewer capacity and audio settings.

## Viewer behavior

Full Twitch channel pages are the default. See [web viewer controls, appearance and
authorization](docs/setup/WEB-VIEWER.md). The same binary retains the embedded mode
behind `--embedded-viewer`. Optional per-channel timers rotate assigned sessions to
the next eligible live favorite; Always preserves normal priority. Windows
monitoring authorization is saved securely for subsequent launches.

## Historical preparation status

This repository snapshot contains the POC, the owner-supplied hosted-player HTML, feature/subagent plans, and a new build/sign/release configuration. It has **not** been compiled or signed in the preparation environment. The remote GitHub repository has **not** been created by this handoff. The intended repository is **private `kc2-io/MPD_Viewer`**.

`Cargo.lock` and exact Rust/action pins must be generated, reviewed and committed before enabling releases. The uploaded POC did not include a resolved lockfile; a fabricated one is not supplied. Native CI intentionally stops when the lockfile is absent.

The supplied deployed host and native POC use different bootstrap/audio/telemetry protocols. See [the integration addendum](docs/HOSTED-SOURCE-PLAN-ADDENDUM.md). A green build or a valid signature is not proof that this mismatch, Twitch viewer sign-in, Turbo behavior, chat or grid integration has been fixed.

## Start here

For the Codex continuation, open this directory as a local project and read
[CODEX-START-HERE.md](CODEX-START-HERE.md), then use
[the first-chat prompt](CODEX-START-PROMPT.md). Full context and existing Git
history are included; no remote or authentication is installed by this handoff.

- [GitHub setup and reference-workflow comparison](docs/setup/GITHUB-SETUP.md)
- [Signing configuration](docs/setup/SIGNING.md)
- [Tags, release gates and expected artifacts](docs/setup/RELEASING.md)
- [Preparation verification and outstanding gates](docs/setup/VERIFICATION.md)
- [Original POC instructions](docs/setup/ORIGINAL-POC-README.md)
- [Feature tasks and subagent packet](docs/feature-plans/)
- [Repository guidance for agents](AGENTS.md)

## Local source checks

Python 3.11+ and Node 22:

```sh
bash scripts/ci-check.sh
```

After installing Rust and platform-specific [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), resolve and commit dependencies as described in setup, then:

```sh
cargo test --locked --workspace --features mpd-tabber/custom-protocol
cargo run --locked -p mpd-tabber --features custom-protocol
```

The binary/crate remains `mpd-tabber`; the visible product is **MPD Viewer**. The existing bundle identifier, preferences path, public Twitch Client ID and parent URL are preserved. This avoids treating a display-name change as a browser-profile/data migration.

## Workflows

| Workflow | Purpose | Privileges |
|---|---|---|
| CI | Source checks, four native build/test targets, explicitly unsigned diagnostic ZIPs | Read-only token; no signing credentials |
| Bootstrap dependency lock | Produce a real initial Cargo lock and resolved Rust pin for review | Read-only; manual main-branch run; no automatic commit |
| Tagged release | Validate tag/source/pins, build without secrets, sign/package, verify complete assets, publish | Separate signing and publication environments; disabled until configured |

Tags are annotated `vMAJOR.MINOR.PATCH` or `vMAJOR.MINOR.PATCH-{alpha,beta,rc}.N`. Tag, workspace and Tauri versions must agree. No tag or release is created by merely opening this source archive. No workflow deploys the parent website.

The source imports retain their existing license and provenance. Credentials, local reference-workflow audit output and signing material are excluded from Git.

## Mobile interaction POC

An Android-emulator-ready, media-free mobile POC lives in
[`mobile-poc/`](mobile-poc/README.md). It reuses the Rust selection core, shows
four assignments in a phone-sized 2×2 grid, and includes an audio + chat concept.
Read the [mobile architecture investigation](docs/mobile/ARCHITECTURE-INVESTIGATION-2026-09-25.md)
before treating the layout as a real Twitch player: official embed dimensions
prevent four compliant videos on a typical phone, and the public embed API does
not expose a supported audio-only mode.
