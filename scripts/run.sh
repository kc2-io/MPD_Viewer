#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
command -v cargo >/dev/null 2>&1 || { echo "Rust/Cargo is required. See README.md." >&2; exit 1; }
exec cargo run -p mpd-tabber --features custom-protocol
