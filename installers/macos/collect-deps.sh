#!/usr/bin/env bash
#
# collect-deps.sh — Collect and bundle all dependencies for the convx .pkg installer.
#
# Usage: bash collect-deps.sh <app-bundle-resources-dir>
#
# Populates the Resources/ directory inside convx.app with:
#   bin/         — ffmpeg, ffprobe, vips, pandoc, pdftoppm
#   lib/         — shared libraries for vips/poppler
#   LibreOffice/ — stripped headless LibreOffice
#   Python.framework/ — bundled Python from python.org
#   wheels/      — offline Python wheel files
#
# Environment overrides:
#   SKIP_FFMPEG=1         Skip FFmpeg collection
#   SKIP_PANDOC=1         Skip Pandoc collection
#   SKIP_VIPS=1           Skip libvips collection
#   SKIP_POPPLER=1        Skip Poppler collection
#   SKIP_LIBREOFFICE=1    Skip LibreOffice collection
#   SKIP_PYTHON=1         Skip Python.framework collection
#   SKIP_WHEELS=1         Skip Python wheel collection

set -euo pipefail

RESOURCES_DIR="${1:?Usage: collect-deps.sh <app-bundle-resources-dir>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

BIN_DIR="$RESOURCES_DIR/bin"
LIB_DIR="$RESOURCES_DIR/lib"

mkdir -p "$BIN_DIR" "$LIB_DIR"

# ── Helper: resolve and copy dylibs recursively ───────────────────

copy_dylibs() {
  local binary="$1"
  local dest_lib="$2"
  local visited=()

  _copy_deps() {
    local file="$1"
    for dep in $(otool -L "$file" 2>/dev/null | awk 'NR>1 {print $1}' | grep -v '^/usr/lib' | grep -v '^/System' | grep -v '@rpath' | grep -v '@executable_path'); do
      local base
      base="$(basename "$dep")"
      if [[ ! -f "$dest_lib/$base" ]]; then
        if [[ -f "$dep" ]]; then
          cp "$dep" "$dest_lib/"
          chmod 755 "$dest_lib/$base"
          # Rewrite the ID
          install_name_tool -id "@loader_path/../lib/$base" "$dest_lib/$base" 2>/dev/null || true
          _copy_deps "$dest_lib/$base"
        fi
      fi
    done

    # Rewrite all references in the file to use @loader_path
    for dep in $(otool -L "$file" 2>/dev/null | awk 'NR>1 {print $1}' | grep -v '^/usr/lib' | grep -v '^/System' | grep -v '@'); do
      local base
      base="$(basename "$dep")"
      install_name_tool -change "$dep" "@loader_path/../lib/$base" "$file" 2>/dev/null || true
    done
  }

  _copy_deps "$binary"
}

# ── 1. FFmpeg (static binary) ─────────────────────────────────────

if [[ "${SKIP_FFMPEG:-0}" != "1" ]]; then
  echo "==> Collecting FFmpeg..."

  # Prefer static build if available, otherwise use Homebrew
  FFMPEG_STATIC="/opt/homebrew/bin/ffmpeg"
  FFPROBE_STATIC="/opt/homebrew/bin/ffprobe"

  if [[ -f "$FFMPEG_STATIC" ]]; then
    cp "$FFMPEG_STATIC" "$BIN_DIR/ffmpeg"
    chmod 755 "$BIN_DIR/ffmpeg"
    copy_dylibs "$BIN_DIR/ffmpeg" "$LIB_DIR"
    echo "    Copied ffmpeg from Homebrew"
  else
    echo "    Warning: ffmpeg not found, skipping"
  fi

  if [[ -f "$FFPROBE_STATIC" ]]; then
    cp "$FFPROBE_STATIC" "$BIN_DIR/ffprobe"
    chmod 755 "$BIN_DIR/ffprobe"
    copy_dylibs "$BIN_DIR/ffprobe" "$LIB_DIR"
    echo "    Copied ffprobe from Homebrew"
  fi
fi

# ── 2. libvips ────────────────────────────────────────────────────

if [[ "${SKIP_VIPS:-0}" != "1" ]]; then
  echo "==> Collecting libvips..."

  VIPS_BIN="$(command -v vips 2>/dev/null || echo /opt/homebrew/bin/vips)"
  if [[ -f "$VIPS_BIN" ]]; then
    cp "$VIPS_BIN" "$BIN_DIR/vips"
    chmod 755 "$BIN_DIR/vips"
    copy_dylibs "$BIN_DIR/vips" "$LIB_DIR"
    echo "    Copied vips + dylibs"
  else
    echo "    Warning: vips not found, skipping"
  fi
fi

# ── 3. Pandoc (static binary) ────────────────────────────────────

if [[ "${SKIP_PANDOC:-0}" != "1" ]]; then
  echo "==> Collecting Pandoc..."

  PANDOC_BIN="$(command -v pandoc 2>/dev/null || echo /opt/homebrew/bin/pandoc)"
  if [[ -f "$PANDOC_BIN" ]]; then
    cp "$PANDOC_BIN" "$BIN_DIR/pandoc"
    chmod 755 "$BIN_DIR/pandoc"
    # Pandoc from Homebrew on macOS is typically statically linked
    echo "    Copied pandoc"
  else
    echo "    Warning: pandoc not found, skipping"
  fi
fi

# ── 4. Poppler (pdftoppm) ────────────────────────────────────────

if [[ "${SKIP_POPPLER:-0}" != "1" ]]; then
  echo "==> Collecting Poppler (pdftoppm)..."

  PDFTOPPM_BIN="$(command -v pdftoppm 2>/dev/null || echo /opt/homebrew/bin/pdftoppm)"
  if [[ -f "$PDFTOPPM_BIN" ]]; then
    cp "$PDFTOPPM_BIN" "$BIN_DIR/pdftoppm"
    chmod 755 "$BIN_DIR/pdftoppm"
    copy_dylibs "$BIN_DIR/pdftoppm" "$LIB_DIR"
    echo "    Copied pdftoppm + dylibs"
  else
    echo "    Warning: pdftoppm not found, skipping"
  fi
fi

# ── 4b. WeasyPrint native deps (GLib, Pango, Cairo, etc.) ────

if [[ "${SKIP_WEASYPRINT_LIBS:-0}" != "1" ]]; then
  echo "==> Collecting WeasyPrint native libraries (GLib, Pango, Cairo)..."

  BREW_LIB="/opt/homebrew/lib"
  if [[ ! -d "$BREW_LIB" ]]; then
    BREW_LIB="/usr/local/lib"
  fi

  if [[ -d "$BREW_LIB" ]]; then
    # Libraries WeasyPrint loads via ctypes/cffi
    WEASY_LIBS=(
      "libgobject-2.0"
      "libglib-2.0"
      "libpango-1.0"
      "libpangocairo-1.0"
      "libpangoft2-1.0"
      "libcairo"
      "libcairo-gobject"
      "libgdk_pixbuf-2.0"
      "libharfbuzz"
      "libfontconfig"
      "libfreetype"
      "libfribidi"
      "libgio-2.0"
      "libgmodule-2.0"
      "libintl"
      "libpixman-1"
      "libpng16"
    )

    WEASY_COUNT=0
    for libname in "${WEASY_LIBS[@]}"; do
      # Find the dylib (could be .0.dylib, .dylib, etc.)
      DYLIB="$(ls "$BREW_LIB"/${libname}*.dylib 2>/dev/null | head -1)"
      if [[ -n "$DYLIB" && -f "$DYLIB" ]]; then
        BASENAME="$(basename "$DYLIB")"
        if [[ ! -f "$LIB_DIR/$BASENAME" ]]; then
          cp "$DYLIB" "$LIB_DIR/"
          chmod 755 "$LIB_DIR/$BASENAME"
          WEASY_COUNT=$((WEASY_COUNT + 1))
        fi
        # Also copy any versioned symlinks
        for link in "$BREW_LIB"/${libname}*.dylib; do
          LBASE="$(basename "$link")"
          if [[ ! -f "$LIB_DIR/$LBASE" && -f "$link" ]]; then
            cp "$link" "$LIB_DIR/" 2>/dev/null || true
          fi
        done
      fi
    done

    # Resolve transitive dylib dependencies for all copied libs
    for lib in "$LIB_DIR"/*.dylib; do
      copy_dylibs "$lib" "$LIB_DIR"
    done

    echo "    Copied $WEASY_COUNT WeasyPrint native libraries"
  else
    echo "    Warning: Homebrew lib dir not found, skipping WeasyPrint native deps"
  fi
fi

# ── 5. LibreOffice (stripped headless) ────────────────────────────

if [[ "${SKIP_LIBREOFFICE:-0}" != "1" ]]; then
  echo "==> Collecting LibreOffice (headless)..."

  LO_DEST="$RESOURCES_DIR/LibreOffice"
  if [[ -d "/Applications/LibreOffice.app" ]]; then
    bash "$SCRIPT_DIR/strip-libreoffice.sh" "$LO_DEST"
  else
    echo "    Warning: LibreOffice.app not found, skipping"
  fi
fi

# ── 6. Python.framework (from python.org) ────────────────────────

if [[ "${SKIP_PYTHON:-0}" != "1" ]]; then
  echo "==> Collecting Python.framework..."

  PY_DEST="$RESOURCES_DIR/Python.framework"
  PYTHON_ORG_FW="/Library/Frameworks/Python.framework"
  PYTHON_VERSION="${PYTHON_BUNDLE_VERSION:-3.13.2}"

  PY_SRC=""

  # Prefer python.org framework (relocatable, universal2)
  if [[ -d "$PYTHON_ORG_FW/Versions" ]]; then
    PY_SRC="$PYTHON_ORG_FW"
    echo "    Found python.org framework at $PYTHON_ORG_FW"
  fi

  # Download from python.org if not installed locally
  if [[ -z "$PY_SRC" ]]; then
    echo "    python.org framework not found locally, downloading Python ${PYTHON_VERSION}..."

    PY_PKG_URL="https://www.python.org/ftp/python/${PYTHON_VERSION}/python-${PYTHON_VERSION}-macos11.pkg"
    PY_DOWNLOAD_DIR="$(mktemp -d "${TMPDIR:-/tmp}/convx-python-dl.XXXXXX")"
    PY_PKG_FILE="$PY_DOWNLOAD_DIR/python.pkg"
    PY_EXTRACT_DIR="$PY_DOWNLOAD_DIR/extracted"

    echo "    Downloading $PY_PKG_URL ..."
    curl -fSL -o "$PY_PKG_FILE" "$PY_PKG_URL" 2>&1 || {
      echo "    Error: Failed to download Python ${PYTHON_VERSION} from python.org"
      echo "    Install Python from https://www.python.org/downloads/ and retry"
      rm -rf "$PY_DOWNLOAD_DIR"
      PY_PKG_FILE=""
    }

    if [[ -n "$PY_PKG_FILE" && -f "$PY_PKG_FILE" ]]; then
      mkdir -p "$PY_EXTRACT_DIR"

      # Extract the framework pkg payload
      echo "    Extracting Python.framework from installer..."
      pkgutil --expand "$PY_PKG_FILE" "$PY_EXTRACT_DIR/pkg" 2>&1

      # Find the framework payload inside the expanded pkg
      FW_PAYLOAD=""
      for payload_dir in "$PY_EXTRACT_DIR/pkg"/Python_Framework*.pkg "$PY_EXTRACT_DIR/pkg"/Python_Framework*; do
        if [[ -f "$payload_dir/Payload" ]]; then
          FW_PAYLOAD="$payload_dir/Payload"
          break
        fi
      done

      if [[ -n "$FW_PAYLOAD" ]]; then
        mkdir -p "$PY_EXTRACT_DIR/framework"
        # Payload is a gzipped cpio archive — extracts framework contents directly
        # (Versions/, Headers/, Resources/, Python at top level)
        (
          cd "$PY_EXTRACT_DIR/framework"
          gunzip -dc < "$FW_PAYLOAD" | cpio -id 2>/dev/null || \
            cpio -id < "$FW_PAYLOAD" 2>/dev/null || true
        )

        # The payload extracts framework contents directly (Versions/ at top level)
        if [[ -d "$PY_EXTRACT_DIR/framework/Versions" ]]; then
          PY_SRC="$PY_EXTRACT_DIR/framework"
          echo "    Extracted Python.framework from installer"
        elif [[ -d "$PY_EXTRACT_DIR/framework/Library/Frameworks/Python.framework" ]]; then
          PY_SRC="$PY_EXTRACT_DIR/framework/Library/Frameworks/Python.framework"
          echo "    Extracted Python.framework from installer"
        else
          echo "    Warning: Could not locate Python.framework in extracted pkg"
        fi
      else
        echo "    Warning: Could not find Python_Framework payload in pkg"
      fi
    fi
  fi

  if [[ -n "$PY_SRC" && -d "$PY_SRC" ]]; then
    # Copy the framework
    cp -R "$PY_SRC" "$PY_DEST"

    # Strip unnecessary components to save space
    find "$PY_DEST" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
    find "$PY_DEST" -type d -name "test" -exec rm -rf {} + 2>/dev/null || true
    find "$PY_DEST" -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/idlelib 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/tkinter 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/turtle* 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/ensurepip 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/distutils 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/lib2to3 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/share 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/bin/idle* 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/bin/2to3* 2>/dev/null || true
    # Remove static library (large, not needed for venv)
    rm -f "$PY_DEST"/Versions/*/lib/python*/config-*/libpython*.a 2>/dev/null || true
    # Remove Tk/Tcl frameworks (large, not needed for headless use)
    rm -rf "$PY_DEST"/Versions/*/Frameworks/Tk.framework 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/Frameworks/Tcl.framework 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/tkinter 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/tk* 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/tcl* 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/Tk* 2>/dev/null || true
    # Remove IDLE app bundle
    rm -rf "$PY_DEST"/Versions/*/Resources/Python.app 2>/dev/null || true
    # Remove header files (not needed at runtime)
    rm -rf "$PY_DEST"/Versions/*/Headers 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/include 2>/dev/null || true
    # Remove .pyc optimized files and debug symbols
    find "$PY_DEST" -name "*.pyc" -delete 2>/dev/null || true
    find "$PY_DEST" -name "*.pyo" -delete 2>/dev/null || true
    # Remove documentation
    rm -rf "$PY_DEST"/Versions/*/lib/python*/doc 2>/dev/null || true
    rm -rf "$PY_DEST"/Versions/*/lib/python*/pydoc* 2>/dev/null || true

    PY_SIZE=$(du -sm "$PY_DEST" | awk '{print $1}')
    echo "    Bundled Python.framework (${PY_SIZE} MB)"
  else
    echo "    Warning: Python.framework not available, skipping"
    echo "    Install from https://www.python.org/downloads/ and retry"
  fi

  # Clean up download temp dir
  rm -rf "${PY_DOWNLOAD_DIR:-/nonexistent}" 2>/dev/null || true
fi

# ── 7. Python wheels ─────────────────────────────────────────────

if [[ "${SKIP_WHEELS:-0}" != "1" ]]; then
  echo "==> Collecting Python wheels..."

  WHEELS_DIR="$RESOURCES_DIR/wheels"
  mkdir -p "$WHEELS_DIR"

  MODULES=(pandas openpyxl weasyprint pdf2docx mobi pyarrow numpy h5py)

  # Detect current platform
  ARCH="$(uname -m)"
  if [[ "$ARCH" == "arm64" ]]; then
    PLATFORM="macosx_11_0_arm64"
  else
    PLATFORM="macosx_10_9_x86_64"
  fi

  # Download wheels for all modules
  pip3 download \
    --only-binary=:all: \
    --platform "$PLATFORM" \
    --python-version 3 \
    --dest "$WHEELS_DIR" \
    "${MODULES[@]}" 2>&1 || {
    echo "    Warning: Some wheels failed to download. Trying without --only-binary..."
    pip3 download \
      --dest "$WHEELS_DIR" \
      "${MODULES[@]}" 2>&1 || echo "    Warning: wheel download had errors"
  }

  WHEEL_COUNT=$(find "$WHEELS_DIR" -name "*.whl" | wc -l | tr -d ' ')
  WHEEL_SIZE=$(du -sm "$WHEELS_DIR" | awk '{print $1}')
  echo "    Downloaded ${WHEEL_COUNT} wheels (${WHEEL_SIZE} MB)"
fi

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "=== Dependency collection complete ==="
echo ""
du -sh "$BIN_DIR" 2>/dev/null | awk '{print "  bin/:          " $1}'
du -sh "$LIB_DIR" 2>/dev/null | awk '{print "  lib/:          " $1}'
du -sh "$RESOURCES_DIR/LibreOffice" 2>/dev/null | awk '{print "  LibreOffice/:  " $1}' || true
du -sh "$RESOURCES_DIR/Python.framework" 2>/dev/null | awk '{print "  Python.framework/: " $1}' || true
du -sh "$RESOURCES_DIR/wheels" 2>/dev/null | awk '{print "  wheels/:       " $1}' || true
echo ""
du -sh "$RESOURCES_DIR" | awk '{print "  TOTAL:         " $1}'
