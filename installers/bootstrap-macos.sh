#!/usr/bin/env bash

set -euo pipefail

LOG_FILE="${CONVX_INSTALLER_LOG:-/tmp/convx-installer.log}"
mkdir -p "$(dirname "$LOG_FILE")"
exec >>"$LOG_FILE" 2>&1

echo "[bootstrap-macos] started at $(date)"

echo "convx dependency bootstrap (macOS)"
echo "-----------------------------------"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script is for macOS only."
  exit 1
fi

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew not found. Install Homebrew first: https://brew.sh"
  exit 1
fi

echo "Installing system packages via Homebrew..."
brew install ffmpeg vips pandoc poppler python@3

echo "Installing applications via Homebrew Cask..."
brew install --cask libreoffice

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
echo "[bootstrap-macos] completed at $(date)"
