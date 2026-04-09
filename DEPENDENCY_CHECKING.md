# Built-in Dependency Checking

## Overview

convx now includes automatic dependency checking built into the tool itself. No separate installation script needed!

## How It Works

### 1. Check Command

Run the check command to see if dependencies are installed:

```bash
./target/release/convx check
```

**If dependencies are missing:**
```
Missing dependencies: FFmpeg, libvips

Please install them:

On macOS:
  brew install ffmpeg vips

On Windows:
  winget install Gyan.FFmpeg libvips.libvips

For other systems, visit:
  - FFmpeg: https://ffmpeg.org/download.html
  - libvips: https://www.libvips.org/install.html
```

**If dependencies are installed:**
```
✓ All dependencies are installed!

FFmpeg: ffmpeg version 4.4.2
libvips: libvips 8.12.2
```

### 2. Automatic Checking During Conversion

When you try to convert a file, convx automatically checks for dependencies:

```bash
./target/release/convx convert input.png --to webp
```

If dependencies are missing, you'll get the same helpful installation instructions before the conversion fails.

## Implementation

The dependency checker is implemented in `src/utils/deps.rs`:

- `DependencyChecker::check_ffmpeg()` - Checks for FFmpeg
- `DependencyChecker::check_vips()` - Checks for libvips
- `DependencyChecker::check_pandoc()` - Checks for Pandoc (document conversions)
- `DependencyChecker::python_has_module(name)` - Checks for Python modules (pyarrow, numpy, h5py)
- `DependencyChecker::convx_python()` - Returns Python from `~/.convx/venv` or system
- `DependencyChecker::check_all()` - Checks all deps and returns helpful errors
- `DependencyChecker::get_versions()` - Gets version information

### Optional Python Dependencies (for ML/data formats)

These are checked on-demand when converting ML data formats:

| Module | Required for | Install |
|--------|-------------|---------|
| `pyarrow` | Parquet ↔ CSV/JSON, Arrow ↔ CSV/JSON | `pip install pyarrow` |
| `numpy` | NPY/NPZ → CSV | `pip install numpy` |
| `h5py` | HDF5 → CSV/JSON | `pip install h5py` |

Python deps are managed via `~/.convx/venv`. SQLite conversions use Python stdlib (`sqlite3`).

## Benefits

✅ **No manual setup** - Just build and run, convx tells you what to install
✅ **Clear instructions** - Platform-specific installation commands
✅ **Early failure** - Catches missing deps before conversion starts
✅ **Version info** - See what versions are installed
✅ **Python venv** - ML deps auto-managed in `~/.convx/venv`

## For Developers

To add a new dependency check:

1. Add a check method in `src/utils/deps.rs`:
```rust
pub fn check_new_tool() -> Result<(), ConvxError> {
    match Command::new("tool").arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(ConvxError::ToolNotFound),
    }
}
```

2. Add it to `check_all()`:
```rust
if Self::check_new_tool().is_err() {
    missing.push("ToolName");
}
```

3. Add installation instructions to the error message.
