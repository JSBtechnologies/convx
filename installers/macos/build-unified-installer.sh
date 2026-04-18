#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# Workspace builds output to repo-root target/, not src-tauri/target/
APP_BUNDLE_DIR="$REPO_ROOT/target/release/bundle/macos"
APP_BUNDLE_PATH="$APP_BUNDLE_DIR/convx.app"
SCRIPTS_DIR="$REPO_ROOT/installers/macos/pkg/scripts"
RESOURCES_DIR="$REPO_ROOT/installers/macos/pkg/resources"
DIST_DIR="$REPO_ROOT/installers/macos/dist"

usage() {
  cat <<'EOF'
Usage: bash installers/macos/build-unified-installer.sh [APP_VERSION]

Builds a unified macOS .pkg installer that places convx.app into /Applications.

Environment overrides:
  SKIP_TAURI_BUILD=1            Skip rebuilding the Tauri app bundle
  SKIP_DEPS_COLLECTION=1        Skip dependency collection (use existing)
  KEEP_PKG_WORK_DIR=1           Keep temporary pkg work directory for debugging
  MACOS_APP_SIGN_IDENTITY=...   Optional codesign identity for .app bundle
  MACOS_PKG_SIGN_IDENTITY=...   Optional productbuild signing identity for .pkg
  MACOS_NOTARY_PROFILE=...      Optional xcrun notarytool keychain profile
                                (requires signed package)

Examples:
  bash installers/macos/build-unified-installer.sh
  SKIP_TAURI_BUILD=1 bash installers/macos/build-unified-installer.sh 0.1.0
  MACOS_PKG_SIGN_IDENTITY="Developer ID Installer: Example Inc. (TEAMID)" \
    MACOS_NOTARY_PROFILE="AC_PROFILE" \
    bash installers/macos/build-unified-installer.sh
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

APP_VERSION="${1:-}"
SKIP_TAURI_BUILD="${SKIP_TAURI_BUILD:-0}"
KEEP_PKG_WORK_DIR="${KEEP_PKG_WORK_DIR:-0}"
MACOS_APP_SIGN_IDENTITY="${MACOS_APP_SIGN_IDENTITY:-}"
MACOS_PKG_SIGN_IDENTITY="${MACOS_PKG_SIGN_IDENTITY:-}"
MACOS_NOTARY_PROFILE="${MACOS_NOTARY_PROFILE:-}"
if [[ -z "$APP_VERSION" ]]; then
  APP_VERSION="$(/usr/bin/python3 - "$REPO_ROOT" <<'PY'
import json,sys
from pathlib import Path
p=Path(sys.argv[1]) / 'convx-app' / 'src-tauri' / 'tauri.conf.json'
print(json.loads(p.read_text())['version'])
PY
)"
fi

PKG_WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/convx-pkg.XXXXXX")"
PKG_ROOT="$PKG_WORK_DIR/root"
COMPONENT_PKG="$PKG_WORK_DIR/convx-component.pkg"
DIST_XML="$PKG_WORK_DIR/distribution.xml"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1"
    exit 1
  fi
}

require_cmd pkgbuild
require_cmd productbuild
require_cmd /usr/bin/python3

if [[ -n "$MACOS_NOTARY_PROFILE" ]]; then
  require_cmd xcrun
fi

cleanup_pkg_work_dir() {
  if [[ "$KEEP_PKG_WORK_DIR" != "1" && -n "${PKG_WORK_DIR:-}" && -d "$PKG_WORK_DIR" ]]; then
    rm -rf "$PKG_WORK_DIR"
  fi
}

trap cleanup_pkg_work_dir EXIT

check_bundle_permissions() {
  if [[ -d "$APP_BUNDLE_PATH" ]]; then
    OWNER="$(stat -f "%Su" "$APP_BUNDLE_PATH" 2>/dev/null || echo "")"
    if [[ -n "$OWNER" && "$OWNER" != "$(id -un)" ]]; then
      echo "Detected non-user-owned bundle: $APP_BUNDLE_PATH (owner: $OWNER)"
      echo "Fix permissions once, then retry:"
      echo "  sudo chown -R $(id -un):staff '$APP_BUNDLE_PATH'"
      exit 1
    fi
  fi
}

check_bundle_permissions

if [[ "$SKIP_TAURI_BUILD" != "1" ]]; then
  echo "Building fresh Tauri app bundle..."
  (
    cd "$REPO_ROOT/convx-app"
    cargo tauri build
  )
fi

check_bundle_permissions

mkdir -p "$DIST_DIR" "$PKG_ROOT/Applications"

APP_PATH=""
if [[ -d "$APP_BUNDLE_PATH" ]]; then
  APP_PATH="$APP_BUNDLE_PATH"
else
  APP_PATH="$(ls -d "$APP_BUNDLE_DIR"/*.app 2>/dev/null | head -n 1 || true)"
fi

if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
  echo "No .app bundle found. Building Tauri app bundle..."
  (
    cd "$REPO_ROOT/convx-app"
    cargo tauri build
  )

  APP_PATH="$(ls -d "$APP_BUNDLE_DIR"/*.app 2>/dev/null | head -n 1 || true)"
  if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
    echo "Failed to produce .app bundle from Tauri build."
    exit 1
  fi
fi

rm -rf "$PKG_ROOT/Applications/convx.app"
cp -R "$APP_PATH" "$PKG_ROOT/Applications/"

# Create symlinks for CLI and MCP inside the app bundle (unified binary)
MACOS_BIN_DIR="$PKG_ROOT/Applications/convx.app/Contents/MacOS"

# Find the Tauri binary name (e.g. convx-app)
TAURI_BIN="$(ls "$MACOS_BIN_DIR" | grep -v '\.dylib' | head -1)"
if [[ -n "$TAURI_BIN" ]]; then
  echo "Creating convx-cli symlink -> $TAURI_BIN"
  ln -sf "$TAURI_BIN" "$MACOS_BIN_DIR/convx-cli"
  echo "Creating convx-mcp symlink -> $TAURI_BIN"
  ln -sf "$TAURI_BIN" "$MACOS_BIN_DIR/convx-mcp"
else
  echo "Warning: No Tauri binary found in $MACOS_BIN_DIR"
fi

# Collect and bundle all dependencies
SKIP_DEPS_COLLECTION="${SKIP_DEPS_COLLECTION:-0}"
if [[ "$SKIP_DEPS_COLLECTION" != "1" ]]; then
  echo ""
  echo "Collecting bundled dependencies..."
  APP_RESOURCES="$PKG_ROOT/Applications/convx.app/Contents/Resources"
  bash "$REPO_ROOT/installers/macos/collect-deps.sh" "$APP_RESOURCES"
  echo ""
fi

# Codesign the .app bundle if identity is provided
if [[ -n "$MACOS_APP_SIGN_IDENTITY" ]]; then
  echo "Signing .app bundle with: $MACOS_APP_SIGN_IDENTITY"

  # Sign the main binary (symlinks convx-cli and convx-mcp point to it)
  if [[ -n "$TAURI_BIN" && -f "$MACOS_BIN_DIR/$TAURI_BIN" ]]; then
    codesign --force --options runtime --sign "$MACOS_APP_SIGN_IDENTITY" "$MACOS_BIN_DIR/$TAURI_BIN"
    echo "  Signed $TAURI_BIN"
  fi

  # Sign the top-level .app bundle (without --deep to avoid re-signing bundled frameworks)
  codesign --force --options runtime --sign "$MACOS_APP_SIGN_IDENTITY" \
    "$PKG_ROOT/Applications/convx.app"
  echo "  Signed convx.app"
fi

PKG_NAME="ConvX-Setup.pkg"
OUT_PATH="$DIST_DIR/$PKG_NAME"

# Generate a component plist and disable bundle relocation.
# Without this, macOS Installer "relocates" the .app to wherever an existing
# copy lives (e.g. the build directory) instead of /Applications.
COMPONENT_PLIST="$PKG_WORK_DIR/component.plist"
pkgbuild --analyze --root "$PKG_ROOT" "$COMPONENT_PLIST"
# Set BundleIsRelocatable=false for all bundle components
/usr/bin/python3 - "$COMPONENT_PLIST" <<'PYEOF'
import plistlib, sys
path = sys.argv[1]
with open(path, "rb") as f:
    pl = plistlib.load(f)
for comp in pl:
    comp["BundleIsRelocatable"] = False
with open(path, "wb") as f:
    plistlib.dump(pl, f)
PYEOF

pkgbuild \
  --root "$PKG_ROOT" \
  --component-plist "$COMPONENT_PLIST" \
  --scripts "$SCRIPTS_DIR" \
  --identifier "com.convx.app" \
  --version "$APP_VERSION" \
  --install-location "/" \
  "$COMPONENT_PKG"

cat > "$DIST_XML" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="1">
  <title>ConvX</title>
  <options customize="never" require-scripts="false"/>
  <choices-outline>
    <line choice="default">
      <line choice="com.convx.app"/>
    </line>
  </choices-outline>
  <choice id="default"/>
  <choice id="com.convx.app" visible="false">
    <pkg-ref id="com.convx.app"/>
  </choice>
  <pkg-ref id="com.convx.app" version="$APP_VERSION" onConclusion="none">convx-component.pkg</pkg-ref>
</installer-gui-script>
EOF

PRODUCTBUILD_ARGS=(
  --distribution "$DIST_XML"
  --package-path "$PKG_WORK_DIR"
  --resources "$RESOURCES_DIR"
)

if [[ -n "$MACOS_PKG_SIGN_IDENTITY" ]]; then
  PRODUCTBUILD_ARGS+=(--sign "$MACOS_PKG_SIGN_IDENTITY")
fi

PRODUCTBUILD_ARGS+=("$OUT_PATH")

productbuild "${PRODUCTBUILD_ARGS[@]}"

if [[ -n "$MACOS_NOTARY_PROFILE" ]]; then
  if [[ -z "$MACOS_PKG_SIGN_IDENTITY" ]]; then
    echo "MACOS_NOTARY_PROFILE provided but package is unsigned. Set MACOS_PKG_SIGN_IDENTITY first."
    exit 1
  fi

  echo "Submitting package for notarization..."
  xcrun notarytool submit "$OUT_PATH" --keychain-profile "$MACOS_NOTARY_PROFILE" --wait

  echo "Stapling notarization ticket..."
  xcrun stapler staple "$OUT_PATH"
fi

echo "✅ Built unified macOS installer: $OUT_PATH"
if [[ "$KEEP_PKG_WORK_DIR" == "1" ]]; then
  echo "ℹ️ Kept temporary build directory: $PKG_WORK_DIR"
fi
