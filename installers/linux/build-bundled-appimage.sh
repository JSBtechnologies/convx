#!/usr/bin/env bash
#
# build-bundled-appimage.sh — Inject bundled deps into Tauri AppImage and rebuild.
#
# Usage: bash build-bundled-appimage.sh <appimage-path> <deps-dir> [output-path]
#
# Extracts the Tauri AppImage, injects deps into usr/libexec/convx-deps/,
# and rebuilds using appimagetool.

set -euo pipefail

ORIG_DIR="$(pwd)"
APPIMAGE_ARG="${1:?Usage: build-bundled-appimage.sh <appimage-path> <deps-dir> [output-path]}"
DEPS_ARG="${2:?Usage: build-bundled-appimage.sh <appimage-path> <deps-dir> [output-path]}"
OUTPUT_ARG="${3:-}"

# Resolve to absolute paths before we cd anywhere
APPIMAGE="$(cd "$(dirname "$APPIMAGE_ARG")" && pwd)/$(basename "$APPIMAGE_ARG")"
DEPS_DIR="$(cd "$DEPS_ARG" && pwd)"

if [[ -n "$OUTPUT_ARG" ]]; then
  # Ensure parent directory exists, then resolve
  mkdir -p "$(dirname "$OUTPUT_ARG")"
  OUTPUT="$(cd "$(dirname "$OUTPUT_ARG")" && pwd)/$(basename "$OUTPUT_ARG")"
else
  OUTPUT=""
fi

if [[ ! -f "$APPIMAGE" ]]; then
  echo "Error: AppImage not found at $APPIMAGE"
  exit 1
fi

if [[ ! -d "$DEPS_DIR" ]]; then
  echo "Error: Deps directory not found at $DEPS_DIR"
  exit 1
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/convx-appimage.XXXXXX")"
APPDIR="$WORK_DIR/squashfs-root"

echo "==> Extracting AppImage..."
chmod +x "$APPIMAGE"
cd "$WORK_DIR"

# --appimage-extract works without FUSE (unlike running the AppImage itself)
"$APPIMAGE" --appimage-extract

if [[ ! -d "$APPDIR" ]]; then
  echo "Error: Failed to extract AppImage"
  rm -rf "$WORK_DIR"
  exit 1
fi

# Inject deps
DEPS_DEST="$APPDIR/usr/libexec/convx-deps"
echo "==> Injecting bundled dependencies into $DEPS_DEST..."
mkdir -p "$DEPS_DEST"
cp -R "$DEPS_DIR"/* "$DEPS_DEST/"

# Set proper permissions
find "$DEPS_DEST/bin" -type f -exec chmod 755 {} + 2>/dev/null || true
find "$DEPS_DEST/python" -name "python3*" -type f -exec chmod 755 {} + 2>/dev/null || true
find "$DEPS_DEST/LibreOffice/program" -name "soffice*" -type f -exec chmod 755 {} + 2>/dev/null || true

DEPS_SIZE=$(du -sm "$DEPS_DEST" | awk '{print $1}')
echo "    Injected ${DEPS_SIZE} MB of dependencies"

# Download appimagetool if not available
APPIMAGETOOL="$(command -v appimagetool 2>/dev/null || true)"
if [[ -z "$APPIMAGETOOL" ]]; then
  echo "==> Downloading appimagetool..."
  ARCH="$(uname -m)"
  APPIMAGETOOL="$WORK_DIR/appimagetool"
  curl -fSL -o "$APPIMAGETOOL" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
  chmod +x "$APPIMAGETOOL"
fi

# Determine output path
if [[ -z "$OUTPUT" ]]; then
  BASENAME="$(basename "$APPIMAGE" .AppImage)"
  OUTPUT="$(dirname "$APPIMAGE")/${BASENAME}-bundled.AppImage"
fi

echo "==> Rebuilding AppImage..."
ARCH="$(uname -m)" "$APPIMAGETOOL" "$APPDIR" "$OUTPUT" 2>&1

echo "==> Built bundled AppImage: $OUTPUT"
echo "    Size: $(du -sh "$OUTPUT" | awk '{print $1}')"

rm -rf "$WORK_DIR"
