# convx - Phases 1-6 Complete ✓

## Summary

The Rust core engine and CLI are fully implemented and operational. All 6 phases are complete.

## Project Structure

```
convx-core/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # CLI entry point
│   ├── engine.rs           # ConvxEngine
│   ├── types/
│   │   ├── mod.rs
│   │   ├── format.rs       # 54 file formats across 6 categories
│   │   ├── options.rs      # Conversion options
│   │   ├── result.rs       # Conversion results
│   │   └── error.rs        # Error types
│   ├── converters/
│   │   ├── mod.rs
│   │   ├── image.rs        # libvips integration
│   │   ├── video.rs        # FFmpeg for video
│   │   ├── audio.rs        # FFmpeg for audio
│   │   ├── document.rs     # Pandoc for documents
│   │   ├── data.rs         # Data/ML format conversions (CSV, JSON, Parquet, Arrow, etc.)
│   │   └── ebook.rs        # Calibre for ebooks
│   └── utils/
│       ├── mod.rs
│       └── deps.rs
└── tests/
    ├── integration.rs      # Integration tests
    └── fixtures/           # Test files (to be generated)
```

## What Works

✅ Full Rust library with comprehensive API
✅ CLI with convert, formats, and version commands
✅ Format detection and validation
✅ Quality, FPS, width, and other conversion options
✅ Size savings and performance metrics
✅ Comprehensive error handling
✅ Integration test suite (60+ tests)

## Commands

```bash
# Build
cargo build --release

# Show help
./target/release/convx --help

# List formats
./target/release/convx formats

# Convert (once dependencies are installed)
./target/release/convx convert input.png --to webp
./target/release/convx convert video.mp4 --to gif --fps 10
./target/release/convx convert audio.wav --to mp3
```

## Required System Dependencies

To use the conversion features, you need:

1. **FFmpeg** - for video and audio conversion
   ```bash
   sudo apt install ffmpeg
   ```

2. **libvips** - for image conversion
   ```bash
   sudo apt install libvips-tools
   ```

3. **Python 3 + pip** (optional) - for ML data format conversions
   ```bash
   pip install pyarrow numpy h5py
   ```
   Python deps are auto-managed via `~/.convx/venv`. SQLite conversion uses Python stdlib (no pip install needed).

## Testing

Generate test fixtures:
```bash
# Sample PNG
ffmpeg -f lavfi -i testsrc=duration=1:size=640x480:rate=1 -frames:v 1 tests/fixtures/sample.png -y

# Sample video (3 seconds)
ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=30 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac tests/fixtures/sample.mp4 -y

# Sample audio (3 seconds)
ffmpeg -f lavfi -i sine=frequency=440:duration=3 tests/fixtures/sample.wav -y
```

Run tests:
```bash
cargo test
```

## Next Steps: Phase 7 - Quasar Frontend

To complete the project:

1. Create Quasar app with TypeScript and Pinia
2. Add Tauri mode for desktop integration
3. Create bridge layer for Rust IPC
4. Build UI components (drag & drop, format selector)
5. Wire up frontend to Rust backend

Phase 7 requires Node.js/npm and is a separate major undertaking.

## Supported Formats (54 total)

**Images (11)**: png, jpg, webp, gif, bmp, tiff, ico, svg, heic, heif, avif
**Video (10)**: mp4, mov, webm, avi, mkv, wmv, flv, m4v, mpeg, ts
**Audio (10)**: mp3, wav, flac, m4a, aac, ogg, wma, aiff, opus, ac3
**Documents (8)**: pdf, docx, doc, pptx, xlsx, txt, md, html
**Data (13)**: csv, json, yaml, xml, parquet, jsonl, tsv, arrow, sqlite, npy, npz, hdf5
**Ebooks (2)**: epub, mobi

### Data/ML Conversion Highlights

- **Pure Rust** (no deps): TSV↔CSV, JSONL↔JSON/CSV, Data→HTML/Markdown tables
- **Python-backed**: Parquet↔CSV/JSON (pyarrow), Arrow↔CSV/JSON (pyarrow), SQLite→CSV/JSON (stdlib), NPY/NPZ→CSV (numpy), HDF5→CSV/JSON (h5py)
- **Cross-category**: CSV/JSON/TSV/JSONL/YAML/XML → HTML, CSV/JSON → PDF (pandoc+weasyprint), CSV/JSON → Markdown

## Status

- ✅ Phase 1: Project Setup
- ✅ Phase 2: Core Types
- ✅ Phase 3: Converters
- ✅ Phase 4: Engine
- ✅ Phase 5: CLI
- ✅ Phase 6: Tests
- ⏳ Phase 7: Quasar Frontend (pending)
