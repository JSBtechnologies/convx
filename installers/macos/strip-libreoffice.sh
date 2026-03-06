#!/usr/bin/env bash
#
# strip-libreoffice.sh — Create a minimal headless LibreOffice for bundling.
#
# Usage: bash strip-libreoffice.sh <output-dir>
#
# Copies LibreOffice.app from /Applications and strips it down to the minimum
# needed for `soffice --headless --convert-to` (DOC/PPTX/XLSX conversions).
# Reduces ~794 MB → ~300-350 MB.

set -euo pipefail

DEST="${1:?Usage: strip-libreoffice.sh <output-dir>}"
LO_SRC="/Applications/LibreOffice.app"

if [[ ! -d "$LO_SRC" ]]; then
  echo "Error: LibreOffice not found at $LO_SRC"
  echo "Install it first: brew install --cask libreoffice"
  exit 1
fi

echo "Stripping LibreOffice to minimal headless build..."

# Create the output structure
mkdir -p "$DEST"

# Copy the entire app first, then strip
cp -R "$LO_SRC/Contents/MacOS" "$DEST/MacOS"
cp -R "$LO_SRC/Contents/Frameworks" "$DEST/Frameworks"

# Only copy essential Resources
mkdir -p "$DEST/Resources"

# Core resources needed for conversion
for item in registry fonts; do
  if [[ -d "$LO_SRC/Contents/Resources/$item" ]]; then
    cp -R "$LO_SRC/Contents/Resources/$item" "$DEST/Resources/"
  fi
done

# Copy filter configuration if it exists
if [[ -d "$LO_SRC/Contents/Resources/filter" ]]; then
  cp -R "$LO_SRC/Contents/Resources/filter" "$DEST/Resources/"
fi

# Copy basic (needed by some filters)
if [[ -d "$LO_SRC/Contents/Resources/basic" ]]; then
  cp -R "$LO_SRC/Contents/Resources/basic" "$DEST/Resources/"
fi

# Copy xslt (may be needed by import/export filters)
if [[ -d "$LO_SRC/Contents/Resources/xslt" ]]; then
  cp -R "$LO_SRC/Contents/Resources/xslt" "$DEST/Resources/"
fi

# Copy the en.lproj (keep English only)
if [[ -d "$LO_SRC/Contents/Resources/en.lproj" ]]; then
  cp -R "$LO_SRC/Contents/Resources/en.lproj" "$DEST/Resources/"
fi

# Copy share directory (contains type detection, filter configs)
if [[ -d "$LO_SRC/Contents/Resources/share" ]]; then
  cp -R "$LO_SRC/Contents/Resources/share" "$DEST/Resources/"
fi

# ── Remove large unnecessary components ───────────────────────────

# Remove Python framework from Frameworks (not needed for headless)
rm -rf "$DEST/Frameworks/LibreOfficePython.framework"

# Remove help files
rm -rf "$DEST/Resources/help"

# Remove gallery/clipart
rm -rf "$DEST/Resources/gallery"

# Remove templates
rm -rf "$DEST/Resources/template"

# Remove wizards
rm -rf "$DEST/Resources/wizards"

# Remove config (GUI profiles/settings)
rm -rf "$DEST/Resources/config"

# Remove Java support
rm -rf "$DEST/Resources/java"
rm -rf "$DEST/Frameworks/"*java* 2>/dev/null || true

# Remove extensions (dictionaries etc) — not needed for conversion
rm -rf "$DEST/Resources/extensions"
rm -rf "$DEST/Resources/share/extensions" 2>/dev/null || true

# Remove non-English language packs
find "$DEST/Resources" -name "*.lproj" -not -name "en.lproj" -type d -exec rm -rf {} + 2>/dev/null || true

# Remove autocorrect data (not needed for headless conversion)
rm -rf "$DEST/Resources/share/autocorr" 2>/dev/null || true

# Remove wordbook (not needed)
rm -rf "$DEST/Resources/share/wordbook" 2>/dev/null || true

# Create a convenience soffice symlink at the root
if [[ -f "$DEST/MacOS/soffice" ]]; then
  ln -sf MacOS/soffice "$DEST/soffice"
fi

# Report size
ORIGINAL_SIZE=$(du -sm "$LO_SRC" | awk '{print $1}')
STRIPPED_SIZE=$(du -sm "$DEST" | awk '{print $1}')
echo "Original:  ${ORIGINAL_SIZE} MB"
echo "Stripped:  ${STRIPPED_SIZE} MB"
echo "Saved:     $((ORIGINAL_SIZE - STRIPPED_SIZE)) MB"
echo "Output:    $DEST"
