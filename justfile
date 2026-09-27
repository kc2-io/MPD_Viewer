set shell := ["bash", "-euo", "pipefail", "-c"]

# List the available project commands.
default:
    @just --list

# Show local and live repository state without changing it.
state:
    git status --short --branch
    git remote -v
    gh repo view --json nameWithOwner,url,isPrivate,defaultBranchRef
    gh release list --limit 5
    gh pr list --state open --limit 20
    gh variable get RELEASES_ENABLED --json name,value,updatedAt
    gh variable get STABLE_RELEASES_ENABLED --json name,value,updatedAt

# Run the offline source, configuration, release-policy, and harness unit checks.
verify-source:
    bash scripts/ci-check.sh

# Verify pinned GitHub Action references and their manifest.
verify-pins:
    python3 scripts/pin-actions.py --check

# Check the working diff for whitespace errors.
verify-diff:
    git diff HEAD --check

# Safe default verification: offline source/policy checks, action pins, and diff hygiene.
verify: verify-source verify-pins verify-diff

# Run deterministic core scheduling and Twitch client unit tests.
verify-core:
    cargo test --locked -p mpd-core -p mpd-twitch

# Match the native CI compile/test lane on the current host.
verify-native:
    python3 scripts/release-tools.py require-lock
    cargo metadata --locked --format-version 1 > /dev/null
    cargo test --locked --workspace --features mpd-tabber/custom-protocol
    cargo clippy --locked --workspace --all-targets --features mpd-tabber/custom-protocol
    cargo build --locked --release -p mpd-tabber --features custom-protocol

# Check desktop E2E source and the production/test isolation policies without launching a GUI.
verify-e2e:
    npm run check --prefix tests/e2e
    python3 scripts/e2e-runner-test.py
    python3 scripts/e2e-release-boundary.py

# Run every non-runtime verification lane; Cargo may fetch locked crates if absent.
verify-all: verify verify-core verify-native verify-e2e

# Install the locked desktop E2E Node dependencies. This accesses the network.
e2e-install:
    npm ci --prefix tests/e2e

# Audit the locked desktop E2E dependency tree. This accesses the network.
e2e-audit:
    npm audit --prefix tests/e2e --audit-level=high

# Build the explicitly instrumented, unsigned desktop E2E binary.
e2e-build:
    cargo build --locked -p mpd-tabber --features custom-protocol,e2e-tests

# Run the desktop GUI harness with its documented binary/driver/display environment.
e2e-run:
    test -n "${MPD_E2E_BINARY:-}" || { echo "MPD_E2E_BINARY is required; see tests/e2e/README.md" >&2; exit 2; }
    npm test --prefix tests/e2e

# Run MPD Viewer in its default full-page Twitch mode.
run:
    cargo run --locked -p mpd-tabber --features custom-protocol

# Run MPD Viewer with the embedded viewer fallback.
run-embedded:
    cargo run --locked -p mpd-tabber --features custom-protocol -- --embedded-viewer

# Create an immutable annotated release tag locally after explicit authorization.
tag-release version:
    python3 scripts/tag-release.py "{{ version }}"

# Do not run this after tag-release has already created the local tag.
# Create and push an immutable annotated release tag in one step after explicit authorization.
tag-release-push version:
    python3 scripts/tag-release.py "{{ version }}" --push
