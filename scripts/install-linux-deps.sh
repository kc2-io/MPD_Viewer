#!/usr/bin/env bash
set -euo pipefail
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev   libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev   patchelf libfuse2 file xdg-utils
