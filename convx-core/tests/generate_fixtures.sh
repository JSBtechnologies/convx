#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES_DIR="$ROOT_DIR/tests/fixtures"

mkdir -p "$FIXTURES_DIR"

ffmpeg -f lavfi -i testsrc=duration=1:size=640x480:rate=1 -frames:v 1 "$FIXTURES_DIR/sample.png" -y
ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=30 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac "$FIXTURES_DIR/sample.mp4" -y
ffmpeg -f lavfi -i sine=frequency=440:duration=3 "$FIXTURES_DIR/sample.wav" -y

# Data format fixtures
cat > "$FIXTURES_DIR/sample.csv" <<'CSV'
name,age,city
Alice,30,New York
Bob,25,San Francisco
Charlie,35,Chicago
CSV

cat > "$FIXTURES_DIR/sample.json" <<'JSON'
[
  {"name": "Alice", "age": 30, "city": "New York"},
  {"name": "Bob", "age": 25, "city": "San Francisco"},
  {"name": "Charlie", "age": 35, "city": "Chicago"}
]
JSON

cat > "$FIXTURES_DIR/sample.tsv" <<'TSV'
name	age	city
Alice	30	New York
Bob	25	San Francisco
Charlie	35	Chicago
TSV

cat > "$FIXTURES_DIR/sample.jsonl" <<'JSONL'
{"name": "Alice", "age": 30, "city": "New York"}
{"name": "Bob", "age": 25, "city": "San Francisco"}
{"name": "Charlie", "age": 35, "city": "Chicago"}
JSONL

# Document format fixtures
cat > "$FIXTURES_DIR/sample.md" <<'MD'
# Sample Document

This is a **sample** markdown document for testing.

- Item 1
- Item 2
- Item 3
MD

cat > "$FIXTURES_DIR/sample.html" <<'HTML'
<!DOCTYPE html>
<html><head><title>Sample</title></head>
<body><h1>Sample Document</h1><p>This is a sample HTML document for testing.</p></body>
</html>
HTML

cat > "$FIXTURES_DIR/sample.txt" <<'TXT'
Sample Document

This is a sample plain text document for testing.

Line 1
Line 2
Line 3
TXT

# PPTX fixture (requires python-pptx in venv)
if ~/.convx/venv/bin/python3 -c "import pptx" 2>/dev/null; then
    ~/.convx/venv/bin/python3 -c "
from pptx import Presentation
from pptx.util import Inches
prs = Presentation()
slide = prs.slides.add_slide(prs.slide_layouts[1])
slide.shapes.title.text = 'Sample Presentation'
slide.placeholders[1].text = 'This is a test slide for ConvX.'
prs.save('$FIXTURES_DIR/sample.pptx')
"
    echo "  Created sample.pptx"
else
    echo "  Skipping sample.pptx (python-pptx not installed)"
fi

echo "Fixtures generated in $FIXTURES_DIR"
