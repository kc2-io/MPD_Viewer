# MPD Viewer

**View fav channels in priority**

MPD Viewer is a Rust/Tauri desktop application for Windows, Linux, and macOS. It
maintains a ranked list of favorite Twitch channels and opens the highest-priority
live selections within a configurable viewer capacity.

## Current behavior

Full Twitch channel pages are the default viewer. The same binary retains the
embedded viewer with `--embedded-viewer`. The manager supports ranked drag
ordering, live viewer counts, configurable status rescans, per-channel timers,
system light/dark appearance, and saved Windows monitoring authorization.

Session settings offer Standalone (the upgrade default) and Grid layouts. Grid
places the selected native viewers in priority order within one window. Switching
layouts retains their documents and assignment timers. Closing the grid stops
monitoring; closing a standalone viewer skips its broadcast. If the selected
viewers cannot fit the display, the manager reports the problem and stops viewers
rather than reducing capacity. Resize or select Standalone, then press Start.

Full-page mode relies on Twitch's own supported playback, appearance, volume,
and quality controls. Windows and Linux also expose native whole-page mute;
macOS reports that capability as unsupported. Embedded mode retains its separate
wrapper chat, volume, and preferred-quality controls.

See [project state](docs/PROJECT-STATE.md) for a dated implementation/release
snapshot and [product contract](docs/PRODUCT-CONTRACT.md) for durable boundaries.

## Development

Install Bash, Rust and the platform-specific
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), Python 3.11+
available as `python3`, Node 22.12+, and
[`just`](https://github.com/casey/just). Then list the supported project
commands:

```sh
just
```

Common lanes:

```sh
just verify
just verify-core
just verify-native
just verify-e2e
just verify-all
```

The lanes and their evidence boundaries are defined in the
[verification matrix](docs/TEST-MATRIX.md). `just verify` is the safe default: it
does not install packages, modify lockfiles, launch a GUI, or contact Twitch.

Run the application in its default mode:

```sh
just run
```

Run the embedded fallback:

```sh
just run-embedded
```

The binary/crate remains `mpd-tabber`; the visible product is MPD Viewer. The
bundle identifier and stored settings/profile identities intentionally remain
stable for compatibility.

## Repository workflow

Project-wide agent and contributor expectations are in
[`AGENTS.md`](AGENTS.md). The default implementation workflow includes focused
verification, independent review, a task branch and pull request, required-check
monitoring, and revision-bound UI evidence where applicable. Merge, tags,
releases, deployments, and provider administration still require explicit
authorization.

The repository has:

- Read-only source and native CI on pull requests.
- Native build/test lanes for Windows x64, Linux x64, macOS arm64, and macOS x64.
- Desktop GUI fixture E2E with bounded screenshot/video evidence.
- A fail-closed tagged-release pipeline with separate signing and publication
  environments.

Pull-request artifacts are unsigned diagnostics. Release scope and signing
requirements are documented in [tagged releases](docs/setup/RELEASING.md) and
[signing configuration](docs/setup/SIGNING.md). No workflow deploys the parent
website.

## Architecture and provenance

Rust is authoritative for favorites, selection, assignments, timers, persistence,
permissions, and native window lifecycle. Web content renders only assigned
state.

`web/parent.mpdviewer.com/index.html` is a provenance snapshot of owner-supplied
hosted source. It is not interchangeable with the bundled wrapper and is not a
deployment instruction. Read the
[hosted-source addendum](docs/HOSTED-SOURCE-PLAN-ADDENDUM.md) before protocol,
chat, or grid work.

Historical repository-bootstrap handoffs are retained under `docs/archive/` for
provenance only. They are not current instructions or authorization.
