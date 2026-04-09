# Windows Installer Overhaul + Setup Screen Personality

**Date:** 2026-03-08
**Status:** Approved

## Problem

The bundled Windows installer (`ConvX-Setup.exe`) has three cascading failures:

1. **convx.exe not found** — Inno Setup defaults to `Program Files (x86)` on 64-bit Windows (missing `ArchitecturesInstallIn64BitMode`). The Tauri MSI installs to a different path. CreateProcess fails with code 2.
2. **Venv creation fails** — The bundled embeddable Python distribution doesn't include the `venv` module. `ensure_venv()` in `deps.rs` fails, so `pip.exe` never exists at `~/.convx/venv/Scripts/pip.exe`.
3. **Setup screen exposes internals** — Users see "pandas (Python)", "openpyxl (Python)" etc. during setup. This leaks implementation details and feels technical.

## Solution

### Part 1: Inno Setup as sole installer (drop MSI wrapper)

**Current flow:** Inno Setup extracts MSI to temp -> runs `msiexec` -> hopes paths align.

**New flow:** Inno Setup directly places all files. No MSI involved at runtime.

Changes to `convx-bundled.iss`:
- Add `ArchitecturesInstallIn64BitMode=x64compatible`
- Replace `[Run] msiexec` with direct `[Files]` entries for the Tauri exe + WebView2 resources
- Replace the pip venv step with bundled Python direct install into `deps\python\`
- Remove MSI as a build input; instead take the raw Tauri build output directory

New build script step: `collect-deps-windows.ps1` (or a separate script) extracts the Tauri exe and resources from the build output into the deps staging directory.

### Part 2: Fix `deps.rs` — skip venv on bundled installs

`install_pip_module()` currently calls `ensure_venv()` unconditionally, which fails on embeddable Python.

New logic:
- If bundled pip exists at `<exe_dir>/deps/python/Scripts/pip.exe`, use it directly
- Only attempt venv creation when no bundled pip is found (bootstrapper/manual install)
- `python_has_module()` should also check bundled Python, not just venv

### Part 3: Setup screen — grouped capabilities, no package names

Replace 14 individual dependency rows with 4 capability groups:

| Internal deps | User sees |
|---|---|
| ffmpeg, vips | Tuning the pixel engine |
| libreoffice, pandoc, poppler | Sharpening the document toolkit |
| pip:pandas, pip:openpyxl, pip:pyarrow, pip:numpy, pip:h5py | Crunching the data gears |
| pip:weasyprint, pip:pdf2docx, pip:mobi | Wiring up the format translators |

Loading messages refreshed:
- "Warming up the conversion engine..."
- "Teaching your computer new tricks..."
- "Calibrating the format wizardry..."
- "Your files never leave this machine."
- "Almost ready to transform things..."

## Files to modify

- `installers/windows/convx-bundled.iss` — rewrite to direct file placement
- `installers/windows/build-bundled-installer.ps1` — update for new input format
- `installers/windows/collect-deps-windows.ps1` — add Tauri exe extraction step
- `convx-core/src/utils/deps.rs` — fix `install_pip_module()` to skip venv when bundled pip exists
- `convx-app/src/components/DependencySetupWizard.vue` — grouped capabilities, new copy
- `convx-app/src-tauri/src/commands.rs` — update `getMissingDependencies` to return groups
