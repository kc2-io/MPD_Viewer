#!/usr/bin/env bash
set -euo pipefail
python3 scripts/release-tools.py check-version
node --test tests/view-model.test.mjs tests/timer.test.mjs tests/viewer-count.test.mjs tests/quality.test.cjs tests/hosted-source-characterization.test.cjs tests/hosted-player-adapter.test.cjs tests/chat.test.cjs
python3 tests/configuration_test.py
python3 -m unittest discover -s tests -p 'release_test.py' -v
