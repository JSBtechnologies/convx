#!/usr/bin/env bash

set -euo pipefail

LOG_FILE="${CONVX_INSTALLER_LOG:-/tmp/convx-installer.log}"
SKIP_BOOTSTRAP="${CONVX_SKIP_BOOTSTRAP:-0}"
mkdir -p "$(dirname "$LOG_FILE")"
exec >>"$LOG_FILE" 2>&1

echo "[run-installer-macos] started at $(date)"

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUNDLE_ROOT="$REPO_ROOT/convx-app/src-tauri/target/release/bundle/dmg"

DMG_PATH="${1:-}"
if [[ -z "$DMG_PATH" ]]; then
  if [[ -d "$BUNDLE_ROOT" ]]; then
    DMG_PATH="$(ls -t "$BUNDLE_ROOT"/*.dmg 2>/dev/null | head -n 1 || true)"
  fi
fi

if [[ -z "$DMG_PATH" || ! -f "$DMG_PATH" ]]; then
  echo "No DMG found. Build one first:"
  echo "  cd convx-app && cargo tauri build"
  exit 1
fi

if [[ "$SKIP_BOOTSTRAP" != "1" ]]; then
  echo "Installing prerequisites..."
  bash "$REPO_ROOT/installers/bootstrap-macos.sh"
else
  echo "Skipping prerequisites bootstrap (CONVX_SKIP_BOOTSTRAP=1)"
fi

echo "Opening DMG: $DMG_PATH"
open "$DMG_PATH"

echo "✅ Prerequisites installed and installer opened"
echo "[run-installer-macos] completed at $(date)"
