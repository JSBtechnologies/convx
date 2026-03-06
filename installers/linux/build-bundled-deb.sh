#!/usr/bin/env bash
#
# build-bundled-deb.sh — Repack Tauri .deb with bundled dependencies.
#
# Usage: bash build-bundled-deb.sh <deb-path> <deps-dir> [output-path]
#
# Extracts the Tauri .deb, adds deps to /opt/convx/deps/, adds postinst script,
# and repacks.

set -euo pipefail

DEB_PATH="${1:?Usage: build-bundled-deb.sh <deb-path> <deps-dir> [output-path]}"
DEPS_DIR="${2:?Usage: build-bundled-deb.sh <deb-path> <deps-dir> [output-path]}"
OUTPUT="${3:-}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ ! -f "$DEB_PATH" ]]; then
  echo "Error: .deb not found at $DEB_PATH"
  exit 1
fi

if [[ ! -d "$DEPS_DIR" ]]; then
  echo "Error: Deps directory not found at $DEPS_DIR"
  exit 1
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/convx-deb.XXXXXX")"
DEB_ROOT="$WORK_DIR/deb"

echo "==> Extracting .deb package..."
mkdir -p "$DEB_ROOT"
dpkg-deb -R "$DEB_PATH" "$DEB_ROOT"

# Inject deps
DEPS_DEST="$DEB_ROOT/opt/convx/deps"
echo "==> Injecting bundled dependencies..."
mkdir -p "$DEPS_DEST"
cp -R "$DEPS_DIR"/* "$DEPS_DEST/"

find "$DEPS_DEST/bin" -type f -exec chmod 755 {} + 2>/dev/null || true
find "$DEPS_DEST/python" -name "python3*" -type f -exec chmod 755 {} + 2>/dev/null || true
find "$DEPS_DEST/LibreOffice/program" -name "soffice*" -type f -exec chmod 755 {} + 2>/dev/null || true

DEPS_SIZE=$(du -sm "$DEPS_DEST" | awk '{print $1}')
echo "    Injected ${DEPS_SIZE} MB of dependencies"

# Add/update postinst script
echo "==> Adding postinst script..."
mkdir -p "$DEB_ROOT/DEBIAN"
cp "$SCRIPT_DIR/postinst" "$DEB_ROOT/DEBIAN/postinst"
chmod 755 "$DEB_ROOT/DEBIAN/postinst"

# Update installed size in control file
INSTALLED_KB=$(du -sk "$DEB_ROOT" | awk '{print $1}')
if [[ -f "$DEB_ROOT/DEBIAN/control" ]]; then
  sed -i "s/^Installed-Size:.*/Installed-Size: $INSTALLED_KB/" "$DEB_ROOT/DEBIAN/control"
fi

# Determine output path
if [[ -z "$OUTPUT" ]]; then
  BASENAME="$(basename "$DEB_PATH" .deb)"
  OUTPUT="$(dirname "$DEB_PATH")/${BASENAME}-bundled.deb"
fi

echo "==> Rebuilding .deb package..."
dpkg-deb -b "$DEB_ROOT" "$OUTPUT"

echo "==> Built bundled .deb: $OUTPUT"
echo "    Size: $(du -sh "$OUTPUT" | awk '{print $1}')"

rm -rf "$WORK_DIR"
