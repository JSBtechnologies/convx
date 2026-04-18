# Contributing to ConvX

Thank you for your interest in contributing. ConvX is a local-first file converter built in Rust with a Tauri + Quasar desktop app. This guide covers everything you need to get started.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Running Tests](#running-tests)
- [Code Style](#code-style)
- [Submitting Changes](#submitting-changes)
- [Adding a New Format](#adding-a-new-format)
- [Reporting Bugs](#reporting-bugs)

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold it.

## Getting Started

1. Fork the repository and clone your fork
2. Set up your development environment (see below)
3. Create a feature branch: `git checkout -b feature/my-feature`
4. Make your changes, add tests, and ensure CI passes
5. Open a pull request against `main`

For non-trivial changes, open an issue first to discuss the approach before investing time in an implementation.

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- System dependencies for your platform

**macOS:**
```bash
brew install ffmpeg vips pandoc poppler python@3
brew install --cask libreoffice
python3 -m venv ~/.convx/venv
~/.convx/venv/bin/pip install pandas openpyxl weasyprint pdf2docx "PyMuPDF==1.23.26" mobi pyarrow numpy h5py
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get install ffmpeg libvips-tools libvips-dev pandoc poppler-utils python3 python3-venv
python3 -m venv ~/.convx/venv
~/.convx/venv/bin/pip install pandas openpyxl weasyprint pdf2docx "PyMuPDF==1.23.26" mobi pyarrow numpy h5py
```

### Build the Rust core

```bash
cd convx-core
cargo build
./target/debug/convx check   # verify system deps are found
```

### Desktop app (optional)

```bash
cd convx-app
npm install
npm run dev          # SPA dev server (no Tauri, port 1420)
npm run tauri:dev    # full desktop app with hot reload
```

Requires `cargo install tauri-cli --version "^2"` for Tauri commands.

## Running Tests

```bash
cd convx-core

# Generate test fixtures (requires ffmpeg, ~5 seconds)
./tests/generate_fixtures.sh

# Unit tests — no external deps needed, runs fast
cargo test

# Integration tests — require fixtures and system tools
cargo test -- --ignored
```

Unit tests cover format detection, option handling, codec selection, and converter logic. Integration tests perform real conversions against fixtures. When adding a new converter or format, add both.

## Code Style

CI enforces these — your PR will not merge without them passing:

```bash
cargo fmt --check        # formatting
cargo clippy -- -D warnings   # lint
```

Run `cargo fmt` and `cargo clippy --fix` before pushing. No exceptions.

## Submitting Changes

- **One PR per logical change.** Don't bundle unrelated fixes.
- **Add tests** for new behavior. Integration tests for new conversions; unit tests for logic.
- **Update the relevant docs** in `docs/` if your change affects architecture or behavior.
- **Keep commit messages clear** — describe what and why, not how.
- CI must pass (fmt + clippy + unit + integration tests).

## Adding a New Format

The converter pipeline has a clear extension pattern:

1. **`convx-core/src/types/format.rs`** — Add the format variant to the `Format` enum and update `detect()`, `category()`, and `extension()`.
2. **`convx-core/src/converters/`** — Either add to an existing converter (e.g., a new image type goes in `image.rs`) or create a new file implementing the `Converter` trait.
3. **`convx-core/src/engine.rs`** — Register the new conversion path in `can_convert()` and route it in `convert()`.
4. **Tests** — Add a unit test for `can_convert` logic and an integration test (marked `#[ignore]`) for an actual conversion.
5. **README.md** — Update the Supported Formats table.

See `docs/core-engine.md` for a detailed walkthrough of the engine architecture.

## Reporting Bugs

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml). Include your OS, the ConvX version (`convx version`), the output of `convx check`, and the exact command that failed with its full output.
