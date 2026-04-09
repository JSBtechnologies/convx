<p align="center">
  <img src="icon-convx-512.png" alt="ConvX" width="128" height="128">
</p>

<h1 align="center">ConvX</h1>

<p align="center">
  <strong>Local-first universal file converter for CLI, Desktop, and AI agents.</strong>
</p>

<p align="center">
  <a href="https://github.com/JSBtechnologies/convx/actions/workflows/ci.yml"><img src="https://github.com/JSBtechnologies/convx/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License: Apache-2.0"></a>
  <a href="https://github.com/JSBtechnologies/convx/releases"><img src="https://img.shields.io/github/v/release/JSBtechnologies/convx?label=release" alt="Release"></a>
</p>

<p align="center">
  <a href="#installation">Installation</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#supported-formats">Formats</a> &middot;
  <a href="#presets">Presets</a> &middot;
  <a href="#mcp-server">MCP Server</a> &middot;
  <a href="#desktop-app">Desktop App</a>
</p>

---

ConvX converts images, video, audio, documents, data files, and ebooks — all processed entirely on your machine. No uploads, no cloud, no waiting. One Rust core powers three interfaces: a command-line tool, a native desktop app, and an MCP server for AI-assisted workflows.

**Open source. Offline. No activation required.**

## Key Features

- **100% local processing** — your files never leave your machine
- **53 formats across 6 categories** — images, video, audio, documents, data, and ebooks
- **Three interfaces** — CLI, desktop app (macOS/Windows), and MCP server
- **18 built-in presets** — one-command conversions for Discord, Twitter, Instagram, and more
- **Batch processing** — convert thousands of files in parallel with `--jobs`
- **File watching** — auto-convert files as they appear in a directory
- **Smart size targeting** — iteratively optimize output to hit a target file size
- **Built-in dependency management** — `convx check` verifies your system is ready
- **MCP-native** — governed file conversion for AI agent workflows

## Supported Formats

| Category | Formats | Count |
|----------|---------|-------|
| **Image** | PNG, JPG, WebP, GIF, BMP, TIFF, ICO, SVG, HEIC, HEIF, AVIF | 11 |
| **Video** | MP4, MOV, WebM, AVI, MKV, WMV, FLV, M4V, MPEG, TS | 10 |
| **Audio** | MP3, WAV, FLAC, M4A, AAC, OGG, WMA, AIFF, Opus, AC3 | 10 |
| **Document** | PDF, DOCX, DOC, PPTX, XLSX, TXT, Markdown, HTML | 8 |
| **Data** | CSV, JSON, YAML, XML, Parquet, JSONL, TSV, Arrow, SQLite, NPY, NPZ, HDF5 | 12 |
| **Ebook** | EPUB, MOBI | 2 |

**Cross-category conversions** are also supported:

- Video -> GIF (animated frame extraction)
- Video -> Audio (extract audio track)
- Images -> PDF (embed in document)
- PDF -> PNG/JPG (page rendering)
- Data -> HTML/PDF/Markdown (styled table export)
- EPUB -> PDF

## Installation

### Desktop App (Recommended)

Download the latest installer for your platform from [Releases](https://github.com/JSBtechnologies/convx/releases):

| Platform | Package | Notes |
|----------|---------|-------|
| **macOS** | `convx-x.x.x.pkg` | Universal installer with bundled dependencies |
| **Windows** | `convx-setup.exe` | Bootstrapper with guided dependency setup |

The desktop installer bundles all required dependencies and sets up the CLI automatically.

### CLI Only (From Source)

Requires [Rust](https://rustup.rs/) 1.70+ and system dependencies:

**macOS:**
```bash
brew install ffmpeg vips pandoc poppler python@3 && brew install --cask libreoffice
python3 -m venv ~/.convx/venv
~/.convx/venv/bin/pip install pandas openpyxl weasyprint pdf2docx "PyMuPDF==1.23.26" mobi pyarrow numpy h5py
```

**Build:**
```bash
git clone https://github.com/JSBtechnologies/convx.git
cd convx/convx-core
cargo build --release
./target/release/convx check   # verify dependencies
```

Platform-specific bootstrap scripts are also available in [`installers/`](installers/README.md).

## Quick Start

```bash
# Convert an image
convx convert photo.png --to webp -q 80

# Convert HEIC photos with a preset
convx convert *.heic --preset heic-to-jpg -j 4

# Create a Discord-optimized video (<8 MB)
convx convert clip.mp4 --preset discord

# Extract audio from a video
convx convert interview.mp4 --to mp3

# Convert a spreadsheet to CSV
convx convert report.xlsx --to csv

# Render data as a styled PDF table
convx convert data.csv --preset data-to-pdf

# Watch a folder and auto-convert screenshots
convx watch ~/Screenshots --to webp --filter "*.png,*.jpg"

# Batch convert with parallel workers
convx convert "./photos/*.png" --to webp -j 8 -d ./output --overwrite
```

## CLI Reference

```
convx <COMMAND> [OPTIONS]

Commands:
  convert       Convert one or more files
  formats       List supported formats and conversion targets
  presets       List and inspect built-in presets
  info          Show file metadata (size, format, duration, codecs)
  watch         Watch a directory for auto-conversion
  check         Verify system dependencies are installed
  mcp           Start the MCP server (stdio transport)
  version       Print version

Global flags:
  --json        Emit machine-readable JSON output
```

### convert

```bash
convx convert <INPUT>... [OPTIONS]
  -o, --output <PATH>       Output file path (single file only)
  -d, --output-dir <DIR>    Output directory for batch mode
      --to <FORMAT>         Target format (e.g., webp, mp3, pdf)
  -q, --quality <1-100>     Quality level
  -w, --width <PX>          Resize width (preserves aspect ratio)
      --fps <N>             Frame rate for video/GIF output
      --max-size <BYTES>    Target maximum file size (best-effort)
  -p, --preset <NAME>       Apply a built-in preset
  -j, --jobs <N>            Parallel workers for batch conversion
      --overwrite           Overwrite existing output files
```

### formats

```bash
convx formats                     # List all formats by category
convx formats --from png          # Show what PNG can convert to
convx formats --category video    # Show all video formats
```

### info

```bash
convx info video.mp4
# Output: format, size, duration, resolution, codecs, available targets
```

### watch

```bash
convx watch ~/Downloads --to webp --filter "*.png,*.jpg,*.heic" --overwrite
```

Monitors a directory and automatically converts matching files as they appear.

## Presets

Presets are predefined conversion profiles that apply optimized settings with a single flag.

| Preset | Output | Description |
|--------|--------|-------------|
| `discord` | MP4 | Discord upload-friendly video (<8 MB, CRF 28) |
| `discord-nitro` | MP4 | Higher quality Discord video (<50 MB) |
| `twitter-image` | WebP | Twitter-optimized image (1200px, quality 85) |
| `twitter-gif` | GIF | Twitter-friendly GIF (480px, 15 fps) |
| `instagram-story` | MP4 | Vertical video for Stories (1080x1920, 30 fps) |
| `web-image` | WebP | Web-optimized image (quality 80, strips metadata) |
| `email-friendly` | JPG | Small email attachment (<1 MB, 1200px) |
| `heic-to-jpg` | JPG | Apple HEIC photos to JPG (quality 90) |
| `archive-lossless` | PNG | Lossless archival copy (quality 100) |
| `extract-audio` | MP3 | Extract audio track (192 kbps) |
| `pdf-to-images` | PNG | Render PDF pages as PNG images |
| `markdown-to-pdf` | PDF | Markdown document to PDF |
| `epub-to-pdf` | PDF | EPUB ebook to PDF |
| `json-to-csv` | CSV | JSON array to CSV spreadsheet |
| `parquet-to-csv` | CSV | Parquet columnar data to CSV |
| `csv-to-parquet` | Parquet | CSV to Parquet columnar format |
| `jsonl-to-csv` | CSV | JSON Lines to CSV |
| `data-to-pdf` | PDF | Render tabular data as a styled PDF table |

```bash
convx presets list              # List all presets
convx presets show discord      # Show preset details
convx convert clip.mp4 --preset discord
```

## MCP Server

ConvX includes a [Model Context Protocol](https://modelcontextprotocol.io/) server, enabling AI assistants like Claude and Cursor to perform file conversions directly.

### Setup

Add to your Claude Desktop or Cursor MCP configuration:

```json
{
  "mcpServers": {
    "convx": {
      "command": "convx",
      "args": ["mcp"]
    }
  }
}
```

Or run standalone:

```bash
convx mcp          # stdio transport
convx-mcp          # alternative binary name
```

### Available Tools

| Tool | Description |
|------|-------------|
| `usage-guide` | Built-in guide for effective ConvX MCP usage |
| `convert_file` | Convert a single file with full option control |
| `batch_convert` | Convert multiple files in one call |
| `get_supported_formats` | List all formats grouped by category |
| `get_conversion_targets` | Get valid output formats for an input format |
| `can_convert` | Check if a specific conversion path is supported |
| `get_file_info` | Get file metadata (size, duration, codecs, resolution) |
| `list_presets` | List all built-in presets |
| `get_preset` | Get details for a specific preset |
| `check_dependencies` | Verify system dependencies are installed |

See [MCP_USAGE.md](MCP_USAGE.md) for detailed setup and troubleshooting.

## Desktop App

The ConvX desktop app provides a native GUI for file conversion built with [Tauri v2](https://v2.tauri.app/) and [Quasar](https://quasar.dev/) (Vue 3).

**Features:**
- Drag-and-drop file conversion
- Real-time progress tracking with stage indicators
- Format browser with conversion target discovery
- Built-in dependency setup wizard
- Conversion history with retry support

### Development

```bash
cd convx-app
npm install
npm run dev              # SPA dev server (no Tauri, port 1420)
npm run tauri:dev        # Full desktop app with hot reload
npm run tauri:bundle     # Production build (.app / .exe)
```

Requires `cargo install tauri-cli --version "^2"` for Tauri commands.

## Architecture

```
convx/
├── convx-core/            Rust library, CLI, and MCP server
│   ├── src/
│   │   ├── engine.rs          ConvxEngine — orchestrator for 6 converter backends
│   │   ├── converters/        Image (libvips), Video/Audio (FFmpeg), Document
│   │   │                      (Pandoc + WeasyPrint + LibreOffice), Data (native),
│   │   │                      Ebook (mobi/epub)
│   │   ├── types/             Format enum (53 formats), options, results, errors
│   │   ├── presets/           18 built-in conversion presets
│   │   ├── utils/             Dependency checker, platform detection
│   │   ├── cli.rs             CLI implementation (clap)
│   │   └── mcp_server.rs     MCP server implementation
│   └── tests/                 130 unit tests + integration test suite
├── convx-app/             Desktop app (Tauri v2 + Quasar v2)
│   ├── src/                   Vue 3 frontend (TypeScript)
│   │   ├── services/bridge/   Tauri IPC / Mock bridge pattern
│   │   ├── pages/             Convert, History, Settings
│   │   └── stores/            Pinia state management
│   └── src-tauri/             Rust backend with IPC commands
└── installers/            Platform installers and bootstrap scripts
```

### Conversion Pipeline

```
Input File -> Format Detection -> Converter Selection -> Processing -> Output File
                                      |
                    +-----------------+-----------------+
                    |                 |                  |
              libvips (images)   FFmpeg (A/V)    Pandoc/WeasyPrint
                                                  (documents)
```

The engine supports progress callbacks, cancellation, and iterative size optimization (for `--max-size` targets, it adjusts quality, CRF, dimensions, and bitrate across up to 8 attempts).

## System Dependencies

ConvX orchestrates best-in-class open-source tools under a unified interface:

| Dependency | Used For | Required |
|------------|----------|----------|
| [FFmpeg](https://ffmpeg.org/) | Video, audio, and GIF processing | Yes |
| [libvips](https://www.libvips.org/) | High-performance image processing | Yes |
| [Pandoc](https://pandoc.org/) | Document format conversion | Yes |
| [Poppler](https://poppler.freedesktop.org/) | PDF page rendering (pdftoppm) | Yes |
| [LibreOffice](https://www.libreoffice.org/) | DOC, PPTX, XLSX conversion | Yes |
| [Python 3](https://www.python.org/) | Data/ML format support | Optional |
| [WeasyPrint](https://weasyprint.org/) | HTML/CSS to PDF rendering | Optional |

Python modules (installed in `~/.convx/venv`): `pandas`, `openpyxl`, `weasyprint`, `pdf2docx`, `mobi`, `pyarrow`, `numpy`, `h5py`.

Run `convx check` at any time to verify your setup.

## Testing

```bash
cd convx-core

# Generate test fixtures (requires ffmpeg)
./tests/generate_fixtures.sh

# Run unit tests (130 tests, no external deps needed)
cargo test

# Run integration tests (requires fixtures + system tools)
cargo test -- --ignored

# Lint and format checks (CI-enforced)
cargo clippy -- -D warnings
cargo fmt --check
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Ensure tests pass (`cargo test && cargo clippy -- -D warnings`)
4. Submit a pull request

## License

Apache-2.0. Copyright 2025 JSB Technologies. See [LICENSE](LICENSE).
