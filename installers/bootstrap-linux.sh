#!/usr/bin/env bash

set -euo pipefail

LOG_FILE="${CONVX_INSTALLER_LOG:-/tmp/convx-installer.log}"
mkdir -p "$(dirname "$LOG_FILE")"
exec >>"$LOG_FILE" 2>&1

echo "[bootstrap-linux] started at $(date)"

echo "convx dependency bootstrap (Linux)"
echo "----------------------------------"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This script is for Linux only."
  exit 1
fi

if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update
  sudo apt-get install -y --no-install-recommends \
    ffmpeg libvips-tools pandoc poppler-utils \
    libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress \
    python3 python3-venv
elif command -v dnf >/dev/null 2>&1; then
  sudo dnf install -y ffmpeg vips vips-tools pandoc poppler-utils \
    libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress \
    python3
elif command -v pacman >/dev/null 2>&1; then
  sudo pacman -S --noconfirm ffmpeg libvips pandoc poppler libreoffice-still python
else
  echo "No supported package manager detected."
  echo "Install manually:"
  echo "  - FFmpeg: https://ffmpeg.org/download.html"
  echo "  - libvips: https://www.libvips.org/install.html"
  echo "  - Pandoc: https://pandoc.org/installing.html"
  echo "  - Poppler: https://poppler.freedesktop.org/"
  echo "  - LibreOffice: https://www.libreoffice.org/download/"
  exit 1
fi

echo "Setting up Python virtual environment..."
VENV_DIR="$HOME/.convx/venv"
if [[ ! -f "$VENV_DIR/bin/python3" ]]; then
  mkdir -p "$(dirname "$VENV_DIR")"
  python3 -m venv "$VENV_DIR"
fi

echo "Installing Python packages..."
"$VENV_DIR/bin/pip" install --upgrade pip
"$VENV_DIR/bin/pip" install pandas openpyxl weasyprint pdf2docx "PyMuPDF==1.23.26" mobi pyarrow numpy h5py

echo "Verifying installation..."
ffmpeg -version | head -n 1
vips --version
pandoc --version | head -n 1
pdftoppm -v 2>&1 | head -n 1
soffice --version
"$VENV_DIR/bin/python3" --version
"$VENV_DIR/bin/python3" -c "import pandas, openpyxl, weasyprint, pdf2docx, fitz, mobi, pyarrow, numpy, h5py; print('Python modules OK')"

echo "convx prerequisites installed successfully"
echo "[bootstrap-linux] completed at $(date)"
