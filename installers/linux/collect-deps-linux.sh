#!/usr/bin/env bash
#
# collect-deps-linux.sh — Collect portable/static dependency builds for bundled Linux installer.
#
# Usage: bash collect-deps-linux.sh <deps-dir>
#
# Populates the deps directory with:
#   bin/         — ffmpeg, ffprobe, vips, pandoc, pdftoppm
#   lib/         — shared .so files
#   LibreOffice/ — stripped headless LibreOffice
#   python/      — portable Python (indygreg build)
#   wheels/      — offline Python wheel files
#
# Environment overrides:
#   SKIP_FFMPEG=1         Skip FFmpeg collection
#   SKIP_VIPS=1           Skip libvips collection
#   SKIP_PANDOC=1         Skip Pandoc collection
#   SKIP_POPPLER=1        Skip Poppler collection
#   SKIP_LIBREOFFICE=1    Skip LibreOffice collection
#   SKIP_PYTHON=1         Skip Python collection
#   SKIP_WHEELS=1         Skip Python wheel collection

set -euo pipefail

DEPS_DIR="${1:?Usage: collect-deps-linux.sh <deps-dir>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

BIN_DIR="$DEPS_DIR/bin"
LIB_DIR="$DEPS_DIR/lib"
TEMP_DIR="${TMPDIR:-/tmp}/convx-deps-dl"

mkdir -p "$BIN_DIR" "$LIB_DIR" "$TEMP_DIR"

# Versions
VIPS_VERSION="8.16.1"
PANDOC_VERSION="3.6.4"
PYTHON_VERSION="3.13.12"
LO_VERSION="25.2.7"

download_file() {
  local url="$1"
  local dest="$2"
  if [[ -f "$dest" ]]; then
    echo "    Using cached: $(basename "$dest")"
    return
  fi
  echo "    Downloading: $url"
  curl -fSL -o "$dest" "$url"
}

# ── 1. FFmpeg (static build from johnvansickle.com) ─────────────

if [[ "${SKIP_FFMPEG:-0}" != "1" ]]; then
  echo "==> Collecting FFmpeg..."

  FFMPEG_TAR="$TEMP_DIR/ffmpeg-static.tar.xz"
  ARCH="$(uname -m)"
  if [[ "$ARCH" == "x86_64" ]]; then
    FFMPEG_ARCH="amd64"
  else
    FFMPEG_ARCH="arm64"
  fi

  download_file "https://johnvansickle.com/ffmpeg/builds/ffmpeg-git-${FFMPEG_ARCH}-static.tar.xz" "$FFMPEG_TAR"

  FFMPEG_EXTRACT="$TEMP_DIR/ffmpeg"
  if [[ ! -d "$FFMPEG_EXTRACT" ]]; then
    mkdir -p "$FFMPEG_EXTRACT"
    tar -xf "$FFMPEG_TAR" -C "$FFMPEG_EXTRACT" --strip-components=1
  fi

  cp "$FFMPEG_EXTRACT/ffmpeg" "$BIN_DIR/ffmpeg"
  cp "$FFMPEG_EXTRACT/ffprobe" "$BIN_DIR/ffprobe"
  chmod 755 "$BIN_DIR/ffmpeg" "$BIN_DIR/ffprobe"
  echo "    Copied ffmpeg + ffprobe (static)"
fi

# ── 2. libvips (pre-built from GitHub) ──────────────────────────

if [[ "${SKIP_VIPS:-0}" != "1" ]]; then
  echo "==> Collecting libvips..."

  # Try to use system vips and bundle its dependencies
  VIPS_BIN="$(command -v vips 2>/dev/null || true)"
  if [[ -n "$VIPS_BIN" && -f "$VIPS_BIN" ]]; then
    cp "$VIPS_BIN" "$BIN_DIR/vips"
    chmod 755 "$BIN_DIR/vips"

    # Copy required shared libraries
    if command -v ldd >/dev/null 2>&1; then
      ldd "$VIPS_BIN" 2>/dev/null | grep "=> /" | awk '{print $3}' | while read -r lib; do
        base="$(basename "$lib")"
        # Skip system libs
        case "$base" in
          libc.so*|libm.so*|libdl.so*|libpthread.so*|librt.so*|ld-linux*|linux-vdso*) continue ;;
        esac
        if [[ ! -f "$LIB_DIR/$base" ]]; then
          cp "$lib" "$LIB_DIR/" 2>/dev/null || true
        fi
      done
    fi

    # Use patchelf to set RPATH if available
    if command -v patchelf >/dev/null 2>&1; then
      patchelf --set-rpath '$ORIGIN/../lib' "$BIN_DIR/vips" 2>/dev/null || true
    fi
    echo "    Copied vips + shared libs"
  else
    echo "    Warning: vips not found. Install libvips-tools: sudo apt-get install libvips-tools"
  fi
fi

# ── 3. Pandoc (static binary from GitHub) ───────────────────────

if [[ "${SKIP_PANDOC:-0}" != "1" ]]; then
  echo "==> Collecting Pandoc..."

  ARCH="$(uname -m)"
  if [[ "$ARCH" == "x86_64" ]]; then
    PANDOC_ARCH="amd64"
  else
    PANDOC_ARCH="arm64"
  fi

  PANDOC_TAR="$TEMP_DIR/pandoc.tar.gz"
  download_file "https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-linux-${PANDOC_ARCH}.tar.gz" "$PANDOC_TAR"

  PANDOC_EXTRACT="$TEMP_DIR/pandoc"
  if [[ ! -d "$PANDOC_EXTRACT" ]]; then
    mkdir -p "$PANDOC_EXTRACT"
    tar -xzf "$PANDOC_TAR" -C "$PANDOC_EXTRACT" --strip-components=1
  fi

  PANDOC_BIN="$(find "$PANDOC_EXTRACT" -name pandoc -type f | head -1)"
  if [[ -n "$PANDOC_BIN" ]]; then
    cp "$PANDOC_BIN" "$BIN_DIR/pandoc"
    chmod 755 "$BIN_DIR/pandoc"
    echo "    Copied pandoc (static)"
  else
    echo "    Warning: pandoc binary not found in archive"
  fi
fi

# ── 4. Poppler (pdftoppm) ──────────────────────────────────────

if [[ "${SKIP_POPPLER:-0}" != "1" ]]; then
  echo "==> Collecting Poppler (pdftoppm)..."

  PDFTOPPM_BIN="$(command -v pdftoppm 2>/dev/null || true)"
  if [[ -n "$PDFTOPPM_BIN" && -f "$PDFTOPPM_BIN" ]]; then
    cp "$PDFTOPPM_BIN" "$BIN_DIR/pdftoppm"
    chmod 755 "$BIN_DIR/pdftoppm"

    # Bundle shared library dependencies
    if command -v ldd >/dev/null 2>&1; then
      ldd "$PDFTOPPM_BIN" 2>/dev/null | grep "=> /" | awk '{print $3}' | while read -r lib; do
        base="$(basename "$lib")"
        case "$base" in
          libc.so*|libm.so*|libdl.so*|libpthread.so*|librt.so*|ld-linux*|linux-vdso*) continue ;;
        esac
        if [[ ! -f "$LIB_DIR/$base" ]]; then
          cp "$lib" "$LIB_DIR/" 2>/dev/null || true
        fi
      done
    fi

    if command -v patchelf >/dev/null 2>&1; then
      patchelf --set-rpath '$ORIGIN/../lib' "$BIN_DIR/pdftoppm" 2>/dev/null || true
    fi
    echo "    Copied pdftoppm + shared libs"
  else
    echo "    Warning: pdftoppm not found. Install: sudo apt-get install poppler-utils"
  fi
fi

# ── 5. LibreOffice (from official .deb tarball, stripped) ───────

if [[ "${SKIP_LIBREOFFICE:-0}" != "1" ]]; then
  echo "==> Collecting LibreOffice (headless)..."

  LO_DIR="$DEPS_DIR/LibreOffice"
  ARCH="$(uname -m)"
  if [[ "$ARCH" == "x86_64" ]]; then
    LO_DIR_ARCH="x86_64"
    LO_FILE_ARCH="x86-64"
  else
    LO_DIR_ARCH="aarch64"
    LO_FILE_ARCH="aarch64"
  fi

  LO_TAR="$TEMP_DIR/libreoffice.tar.gz"
  LO_DEB_NAME="LibreOffice_${LO_VERSION}_Linux_${LO_FILE_ARCH}_deb"
  download_file "https://download.documentfoundation.org/libreoffice/stable/${LO_VERSION}/deb/${LO_DIR_ARCH}/${LO_DEB_NAME}.tar.gz" "$LO_TAR"

  LO_EXTRACT="$TEMP_DIR/libreoffice"
  if [[ ! -d "$LO_EXTRACT" ]]; then
    mkdir -p "$LO_EXTRACT"
    tar -xzf "$LO_TAR" -C "$LO_EXTRACT" --strip-components=1
  fi

  # Extract all .deb packages
  LO_ROOT="$TEMP_DIR/lo-root"
  mkdir -p "$LO_ROOT"
  find "$LO_EXTRACT" -name "*.deb" | while read -r deb; do
    dpkg-deb -x "$deb" "$LO_ROOT" 2>/dev/null || true
  done

  # Find the program directory
  LO_PROGRAM="$(find "$LO_ROOT" -type d -name "program" -path "*/libreoffice*" | head -1)"
  if [[ -n "$LO_PROGRAM" ]]; then
    LO_INSTALL="$(dirname "$LO_PROGRAM")"
    mkdir -p "$LO_DIR"

    # Copy program/ and other essential dirs
    cp -R "$LO_PROGRAM" "$LO_DIR/"

    # Copy share/registry (needed for headless operation)
    if [[ -d "$LO_INSTALL/share" ]]; then
      cp -R "$LO_INSTALL/share" "$LO_DIR/"
    fi

    # Create soffice wrapper at expected location
    if [[ -f "$LO_DIR/program/soffice" ]]; then
      chmod 755 "$LO_DIR/program/soffice"
    fi
    ln -sf "program/soffice" "$LO_DIR/soffice" 2>/dev/null || true

    # Strip unnecessary components
    rm -rf "$LO_DIR/share/gallery" 2>/dev/null || true
    rm -rf "$LO_DIR/share/template" 2>/dev/null || true
    rm -rf "$LO_DIR/share/autocorr" 2>/dev/null || true
    rm -rf "$LO_DIR/share/wordbook" 2>/dev/null || true
    rm -rf "$LO_DIR/share/wizards" 2>/dev/null || true
    rm -rf "$LO_DIR/program/python-core-"* 2>/dev/null || true
    find "$LO_DIR" -name "*.pyc" -delete 2>/dev/null || true

    LO_SIZE=$(du -sm "$LO_DIR" | awk '{print $1}')
    echo "    Extracted LibreOffice (${LO_SIZE} MB)"
  else
    echo "    Warning: LibreOffice program dir not found in extracted debs"
  fi
fi

# ── 6. Python (indygreg standalone build) ───────────────────────

if [[ "${SKIP_PYTHON:-0}" != "1" ]]; then
  echo "==> Collecting Python..."

  PY_DIR="$DEPS_DIR/python"
  ARCH="$(uname -m)"

  PY_TAR="$TEMP_DIR/python-standalone.tar.gz"
  PY_RELEASE_TAG="20260303"
  PY_BUILD_TAG="${PYTHON_VERSION}+${PY_RELEASE_TAG}"
  if [[ "$ARCH" == "x86_64" ]]; then
    PY_TRIPLE="x86_64-unknown-linux-gnu"
  else
    PY_TRIPLE="aarch64-unknown-linux-gnu"
  fi

  # URL-encode the + as %2B for the filename in the URL
  PY_FILENAME="cpython-${PYTHON_VERSION}%2B${PY_RELEASE_TAG}-${PY_TRIPLE}-install_only_stripped.tar.gz"
  download_file \
    "https://github.com/astral-sh/python-build-standalone/releases/download/${PY_RELEASE_TAG}/${PY_FILENAME}" \
    "$PY_TAR"

  if [[ ! -d "$PY_DIR" ]]; then
    mkdir -p "$PY_DIR"
    tar -xzf "$PY_TAR" -C "$PY_DIR" --strip-components=1
  fi

  # Strip unnecessary components
  find "$PY_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
  find "$PY_DIR" -type d -name "test" -exec rm -rf {} + 2>/dev/null || true
  find "$PY_DIR" -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true
  rm -rf "$PY_DIR"/lib/python*/idlelib 2>/dev/null || true
  rm -rf "$PY_DIR"/lib/python*/tkinter 2>/dev/null || true
  rm -rf "$PY_DIR"/lib/python*/turtle* 2>/dev/null || true
  rm -rf "$PY_DIR"/lib/python*/ensurepip 2>/dev/null || true
  rm -rf "$PY_DIR"/lib/python*/lib2to3 2>/dev/null || true
  rm -rf "$PY_DIR"/include 2>/dev/null || true
  rm -rf "$PY_DIR"/share 2>/dev/null || true
  find "$PY_DIR" -name "*.pyc" -delete 2>/dev/null || true
  find "$PY_DIR" -name "*.pyo" -delete 2>/dev/null || true
  rm -f "$PY_DIR"/lib/python*/config-*/libpython*.a 2>/dev/null || true

  PY_SIZE=$(du -sm "$PY_DIR" | awk '{print $1}')
  echo "    Bundled Python ${PYTHON_VERSION} (${PY_SIZE} MB)"
fi

# ── 7. Python wheels ──────────────────────────────────────────

if [[ "${SKIP_WHEELS:-0}" != "1" ]]; then
  echo "==> Collecting Python wheels..."

  WHEELS_DIR="$DEPS_DIR/wheels"
  mkdir -p "$WHEELS_DIR"

  MODULES=(pandas openpyxl weasyprint pdf2docx mobi pyarrow numpy h5py)

  # Try bundled pip, then system pip
  PY_EXE="$DEPS_DIR/python/bin/python3"
  PIP_EXE="$DEPS_DIR/python/bin/pip3"

  if [[ -f "$PIP_EXE" ]]; then
    PIP_CMD="$PIP_EXE"
  elif command -v pip3 >/dev/null 2>&1; then
    PIP_CMD="pip3"
  else
    echo "    Warning: No pip available, skipping wheel download"
    PIP_CMD=""
  fi

  if [[ -n "$PIP_CMD" ]]; then
    ARCH="$(uname -m)"
    if [[ "$ARCH" == "x86_64" ]]; then
      PLATFORM="manylinux2014_x86_64"
    else
      PLATFORM="manylinux2014_aarch64"
    fi

    echo "    Downloading wheels for: ${MODULES[*]}"
    "$PIP_CMD" download \
      --only-binary=:all: \
      --platform "$PLATFORM" \
      --python-version 3.13 \
      --dest "$WHEELS_DIR" \
      "${MODULES[@]}" 2>&1 || {
      echo "    Warning: Some wheels failed. Trying without --only-binary..."
      "$PIP_CMD" download \
        --dest "$WHEELS_DIR" \
        "${MODULES[@]}" 2>&1 || echo "    Warning: wheel download had errors"
    }

    WHEEL_COUNT=$(find "$WHEELS_DIR" -name "*.whl" | wc -l | tr -d ' ')
    WHEEL_SIZE=$(du -sm "$WHEELS_DIR" | awk '{print $1}')
    echo "    Downloaded ${WHEEL_COUNT} wheels (${WHEEL_SIZE} MB)"
  fi
fi

# ── Summary ──────────────────────────────────────────────────

echo ""
echo "=== Linux dependency collection complete ==="
echo ""
du -sh "$BIN_DIR" 2>/dev/null | awk '{print "  bin/:          " $1}'
du -sh "$LIB_DIR" 2>/dev/null | awk '{print "  lib/:          " $1}'
du -sh "$DEPS_DIR/LibreOffice" 2>/dev/null | awk '{print "  LibreOffice/:  " $1}' || true
du -sh "$DEPS_DIR/python" 2>/dev/null | awk '{print "  python/:       " $1}' || true
du -sh "$DEPS_DIR/wheels" 2>/dev/null | awk '{print "  wheels/:       " $1}' || true
echo ""
du -sh "$DEPS_DIR" | awk '{print "  TOTAL:         " $1}'
