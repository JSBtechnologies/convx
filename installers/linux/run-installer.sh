#!/usr/bin/env bash

set -euo pipefail

LOG_FILE="${CONVX_INSTALLER_LOG:-/tmp/convx-installer.log}"
SKIP_BOOTSTRAP="${CONVX_SKIP_BOOTSTRAP:-0}"
mkdir -p "$(dirname "$LOG_FILE")"
exec >>"$LOG_FILE" 2>&1

echo "[run-installer-linux] started at $(date)"

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

if [[ "$SKIP_BOOTSTRAP" != "1" ]]; then
	echo "Installing prerequisites..."
	bash "$REPO_ROOT/installers/bootstrap-linux.sh"
else
	echo "Skipping prerequisites bootstrap (CONVX_SKIP_BOOTSTRAP=1)"
fi

echo "Linux app installer/package flow depends on distro packaging."
echo "Use your distro package artifact (deb/rpm/AppImage) after prerequisites are installed."
echo "[run-installer-linux] completed at $(date)"
