# Setup verification record

> Current connected setup: [CONTINUATION-2026-09-18.md](CONTINUATION-2026-09-18.md). The earlier handoff status below is historical; releases remain disabled.

Prepared September 17, 2026 (America/Los_Angeles). This record concerns the new repository/CI/signing/release source, not the verification reports from earlier POC turns.

## What was actually done

Restored the previously supplied Git bundle with its two existing commits, imported the existing feature/subagent planning packet, and added repository bootstrap, build/sign/release workflows and helpers locally. Display branding was updated without changing the internal app ID, crate names, saved settings, Client ID or hosted origin. The user-supplied hosted HTML remains byte-for-byte identical to its normalized import.

The old workflow is archived as `docs/setup/baseline-check.yml.txt`. Historical docs are retained; their prior test claims should not be confused with the newly executed checks below.

## Executed checks

| Check | Actual result | Scope |
|---|---|---|
| Node presentation + hosted-source characterization | 30 passed (12 + 18) | JavaScript/pure mocks, including known hosted/native incompatibilities |
| Python configuration checks | 14 passed | URL/origin/capability/default-ID configuration |
| New release helper and guard checks | 30 passed | Tags, metadata/version, missing-lock handling, package names and bytes, mocked action resolution, security assertions |
| **Total counted tests** | **74 passed** | Offline only |
| Workflow YAML parse and job-dependency name checks | Passed for 3 workflows | Syntax/structure only, not GitHub Actions expression validation |
| Python AST, shell `bash -n`, JavaScript `node --check`, TOML and Git whitespace checks | Passed | Static syntax only |
| Three expected-block commands | Correctly stopped | Missing Cargo.lock; disabled release flag; unpinned action tags |

The final exact source-check output is in `source-checks.log`. The packaging unit test uses fixture bytes, not a signed Windows executable. Mac codesign, Authenticode, notarization, GitHub API permissions and real GitHub Action jobs are not mocked into being declared successful.

## Not executed / outstanding

| Gate | Status |
|---|---|
| GitHub connection, authenticated reference workflow inspection | Not available; plugin still not installed at last check |
| Private `kc2-io/MPD_Viewer` creation or source push | Not performed |
| Actual branch/tag/environment protection configuration | Not performed; plan compatibility unverified |
| Reference-repo secret/variable/provider configuration parity | Not inspected; no values copied |
| Cargo dependency resolution / Cargo.lock | Missing in supplied baseline; generation not possible here |
| Exact Rust toolchain and full-SHA Action pins | Helpers supplied; actual resolutions/pins not performed |
| Native Rust compilation/tests/clippy, packaging | Not run; Rust/Cargo absent |
| GitHub native build matrix | Not run |
| Windows Azure identity trust, signing and timestamp validation | Not configured/run |
| Apple certificate/key import, signing, notarization/stapling | Not configured/run |
| Linux DEB/AppImage creation | Not run |
| Actual release uploads/download hash verification | Not run |
| Annotated release tag or published release | None created |
| Deployed parent website | Not modified |
| Application runtime, chat, login/Turbo, real grid playback | Not tested or fixed by this infrastructure work |

No executable, installer, DMG, DEB, AppImage, genuine signed artifact or signature key is included in this handoff. The bundle/ZIP contains **source and setup automation**, not built release artifacts.

## Known blockers and safe defaults

CI requires a committed real Cargo.lock; missing-lock failures are intentional. Bootstrap the lock and resolve/pin the toolchain/actions before signing. Provider variables/secrets and OIDC trust must be configured using authorized existing sources. GitHub plan support for protections must be checked. Release flags remain disabled, and no workflow silently downgrades a signed release to unsigned.

The proposed workflow action versions and native labels were checked against official documentation during preparation, but installation, execution and exact resolutions still require a networked authorized environment. Packaging code and provider/API integration may need corrections discovered by the first real runs.

The original/native and supplied/hosted player protocol mismatch remains. Infrastructure success does not establish application readiness or Twitch policy compliance.

## Repository provenance

Parent commit: `cb8f577` (supplied hosted-source import); earlier commit: `1f0f947` (supplied POC archive snapshot). A new local setup commit follows those commits. The working repository has branch `main` and no remote. This preserves history without claiming any commit exists on GitHub.

The setup commit is attributed to the assistant, not signed as or impersonating the owner. Release tags are to be made later by the authorized maintainer's configured Git identity. No Git signing keys were generated.
