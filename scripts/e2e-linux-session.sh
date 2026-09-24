#!/usr/bin/env bash
set -euo pipefail
# Called inside this job's private X server and D-Bus session. Never touch another
# desktop or search by process name; only the exact window-manager child is ours.
openbox --sm-disable >/dev/null 2>&1 &
window_manager_pid=$!
trap 'kill "$window_manager_pid" 2>/dev/null || true; wait "$window_manager_pid" 2>/dev/null || true' EXIT
python scripts/e2e-runtime-manifest.py
python scripts/e2e-no-driver-probe.py run
npm test --prefix tests/e2e
