# Windows Installer Overhaul + Setup Screen Personality — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix Windows installer so it works reliably, and replace the technical setup screen with grouped capability labels and quirky copy.

**Architecture:** Drop the Inno-wraps-MSI pattern. Inno Setup directly places the Tauri exe + WebView2 loader + bundled deps. On the Rust side, `install_pip_module()` uses bundled pip directly when available instead of trying to create a venv. The frontend groups 14 individual deps into 4 capability buckets with personality-driven labels.

**Tech Stack:** Inno Setup 6, PowerShell, Rust (deps.rs + commands.rs), Vue 3 + TypeScript

---

### Task 1: Fix `deps.rs` — use bundled pip directly, skip venv when unnecessary

**Files:**
- Modify: `convx-core/src/utils/deps.rs:228-257` (`install_pip_module`)

**Step 1: Modify `install_pip_module` to prefer bundled pip**

Replace the current `install_pip_module` method (lines 228-257) with logic that:
- Checks if bundled pip exists first (`convx_pip()` already has fallback logic)
- Only calls `ensure_venv()` if NO bundled pip is available
- Uses whichever pip is found

```rust
/// Installs a pip module into the convx venv or via bundled Python.
/// Prefers bundled pip (Windows bundled installer) over venv to avoid
/// issues with embeddable Python lacking the venv module.
pub fn install_pip_module(module: &str) -> Result<(), String> {
    // Try bundled pip first (works on Windows embeddable Python)
    let pip = if let Some(bundled_pip) = Self::bundled_pip_path() {
        bundled_pip
    } else {
        // Fall back to venv approach (macOS/Linux, bootstrapper installs)
        Self::ensure_venv()?;
        Self::convx_pip()
            .ok_or_else(|| "pip not found in venv after creation".to_string())?
    };

    let mut args = vec!["install".to_string()];

    // Use bundled wheels for offline install if available
    if let Some(wheels) = Self::bundled_wheels_dir() {
        args.push("--find-links".to_string());
        args.push(wheels.to_string_lossy().to_string());
    }

    args.push(module.to_string());

    let output = Command::new(&pip)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run pip install: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pip install {} failed: {}", module, stderr));
    }

    Ok(())
}
```

**Step 2: Add `bundled_pip_path` helper**

Add this method above `install_pip_module`:

```rust
/// Returns bundled pip path if it exists (e.g. Windows bundled installer).
/// This avoids needing to create a venv with embeddable Python.
fn bundled_pip_path() -> Option<String> {
    if let Some(res) = Self::bundled_resources_dir() {
        let bundled_pip = if cfg!(windows) {
            res.join("python").join("Scripts").join("pip.exe")
        } else {
            res.join("python").join("bin").join("pip3")
        };
        if bundled_pip.exists() {
            return Some(bundled_pip.to_string_lossy().to_string());
        }
    }
    None
}
```

**Step 3: Build and run unit tests**

Run: `cd convx-core && cargo build && cargo test`
Expected: All existing tests pass, no compilation errors.

**Step 4: Commit**

```bash
git add convx-core/src/utils/deps.rs
git commit -m "fix(deps): use bundled pip directly, skip venv on Windows embeddable Python"
```

---

### Task 2: Update `commands.rs` — return grouped dependency categories

**Files:**
- Modify: `convx-app/src-tauri/src/commands.rs:334-381` (`get_missing_dependencies`)
- Modify: `convx-app/src-tauri/src/commands.rs:385-578` (`install_single_dependency`)

**Step 1: Change `get_missing_dependencies` to return category groups**

Replace lines 334-381 with logic that returns category-level names instead of individual deps. The backend checks all deps in each category and reports the category as missing if any dep in it is missing.

```rust
/// Returns list of missing dependency *categories* for the setup wizard.
/// Categories group related deps so the UI doesn't expose implementation details.
///
/// Categories:
///   "media"    — ffmpeg, vips
///   "document" — libreoffice, pandoc, poppler
///   "data"     — pip:pandas, pip:openpyxl, pip:pyarrow, pip:numpy, pip:h5py
///   "formats"  — pip:weasyprint, pip:pdf2docx, pip:mobi
#[tauri::command]
pub fn get_missing_dependencies() -> Vec<String> {
    let mut missing = Vec::new();

    // Media: ffmpeg + vips
    if DependencyChecker::check_ffmpeg().is_err() || DependencyChecker::check_vips().is_err() {
        missing.push("media".to_string());
    }

    // Document: libreoffice + pandoc + poppler
    if DependencyChecker::libreoffice_executable().is_none()
        || DependencyChecker::pandoc_executable().is_none()
        || DependencyChecker::pdftoppm_executable().is_none()
    {
        missing.push("document".to_string());
    }

    // Data: pandas, openpyxl, pyarrow, numpy, h5py
    if DependencyChecker::python3_executable().is_none()
        || !DependencyChecker::python_has_module("pandas")
        || !DependencyChecker::python_has_module("openpyxl")
        || !DependencyChecker::python_has_module("pyarrow")
        || !DependencyChecker::python_has_module("numpy")
        || !DependencyChecker::python_has_module("h5py")
    {
        missing.push("data".to_string());
    }

    // Formats: weasyprint, pdf2docx, mobi
    if DependencyChecker::python3_executable().is_none()
        || !DependencyChecker::python_has_module("weasyprint")
        || !DependencyChecker::python_has_module("pdf2docx")
        || !DependencyChecker::python_has_module("mobi")
    {
        missing.push("formats".to_string());
    }

    missing
}
```

**Step 2: Update `install_single_dependency` to handle category names**

The Windows `#[cfg(target_os = "windows")]` block and macOS block both need to handle category-based install. Replace the function body with a dispatcher that expands categories to individual installs.

Add a helper at the top of commands.rs (or just above `install_single_dependency`):

```rust
fn category_deps(category: &str) -> Vec<&'static str> {
    match category {
        "media" => vec!["ffmpeg", "vips"],
        "document" => vec!["libreoffice", "pandoc", "poppler"],
        "data" => vec!["pip:pandas", "pip:openpyxl", "pip:pyarrow", "pip:numpy", "pip:h5py"],
        "formats" => vec!["pip:weasyprint", "pip:pdf2docx", "pip:mobi"],
        _ => vec![category],
    }
}
```

Then update `install_single_dependency` to expand categories:

```rust
#[tauri::command]
pub fn install_single_dependency(name: String) -> JsDependencyStatus {
    let deps = category_deps(&name);
    let mut errors = Vec::new();

    for dep in deps {
        let result = install_one_dep(dep);
        if !result.ok {
            errors.push(result.message);
        }
    }

    if errors.is_empty() {
        JsDependencyStatus {
            ok: true,
            message: format!("{} ready", name),
        }
    } else {
        JsDependencyStatus {
            ok: false,
            message: errors.join("; "),
        }
    }
}
```

Extract the current per-dep install logic into `fn install_one_dep(name: &str) -> JsDependencyStatus` — this is the existing body of `install_single_dependency` with the `pip:` prefix check, brew/winget dispatch, etc. No logic changes, just moved into a helper.

**Step 3: Build**

Run: `cd convx-core && cargo build`
Expected: Clean compile.

**Step 4: Commit**

```bash
git add convx-app/src-tauri/src/commands.rs
git commit -m "feat(commands): group dependencies into categories for setup wizard"
```

---

### Task 3: Update `DependencySetupWizard.vue` — grouped capabilities + quirky copy

**Files:**
- Modify: `convx-app/src/components/DependencySetupWizard.vue`

**Step 1: Replace FRIENDLY_NAMES with category labels**

Replace lines 165-180:

```typescript
const FRIENDLY_NAMES: Record<string, string> = {
  media: 'Tuning the pixel engine',
  document: 'Sharpening the document toolkit',
  data: 'Crunching the data gears',
  formats: 'Wiring up the format translators',
};
```

**Step 2: Replace LOADING_MESSAGES with personality-driven copy**

Replace lines 182-191:

```typescript
const LOADING_MESSAGES = [
  'Warming up the conversion engine...',
  'Teaching your computer new tricks...',
  'Calibrating the format wizardry...',
  'Your files never leave this machine.',
  'Almost ready to transform things...',
  'No cloud. No uploads. Just speed.',
  'Unpacking the good stuff...',
  'Locking in the magic...',
];
```

**Step 3: Update the failed-state install command for Windows**

In the `installCommand` computed (lines 217-235), the current logic splits by `pip:` prefix and `brew`. Update it to handle categories. For Windows, just point to docs since the bundled installer should handle everything:

```typescript
const installCommand = computed(() => {
  const failed = depStates.filter((d) => d.status === 'failed').map((d) => d.name);

  if (os.value === 'macos') {
    const parts: string[] = [];
    const brewDeps: string[] = [];
    const pipDeps: string[] = [];

    for (const cat of failed) {
      if (cat === 'media') brewDeps.push('ffmpeg', 'vips');
      else if (cat === 'document') brewDeps.push('pandoc', 'poppler');
      else if (cat === 'data') pipDeps.push('pandas', 'openpyxl', 'pyarrow', 'numpy', 'h5py');
      else if (cat === 'formats') pipDeps.push('weasyprint', 'pdf2docx', 'mobi');
    }

    if (brewDeps.length) parts.push(`brew install ${brewDeps.join(' ')}`);
    if (pipDeps.length)
      parts.push(`~/.convx/venv/bin/pip install ${pipDeps.join(' ')}`);
    return parts.join(' && ') || 'Try reinstalling convx from the .pkg installer';
  }

  if (os.value === 'linux') {
    const parts: string[] = [];
    const aptDeps: string[] = [];
    const pipDeps: string[] = [];

    for (const cat of failed) {
      if (cat === 'media') aptDeps.push('ffmpeg', 'libvips-tools');
      else if (cat === 'document') aptDeps.push('pandoc', 'poppler-utils');
      else if (cat === 'data') pipDeps.push('pandas', 'openpyxl', 'pyarrow', 'numpy', 'h5py');
      else if (cat === 'formats') pipDeps.push('weasyprint', 'pdf2docx', 'mobi');
    }

    if (aptDeps.length) parts.push(`sudo apt-get install -y ${aptDeps.join(' ')}`);
    if (pipDeps.length)
      parts.push(`~/.convx/venv/bin/pip install ${pipDeps.join(' ')}`);
    return parts.join(' && ') || 'See https://convx.dev/docs for your platform';
  }

  // Windows: bundled installer should handle everything
  return 'Try reinstalling ConvX. See https://convx.dev/docs for help.';
});
```

**Step 4: Update mock bridge to return categories**

In `convx-app/src/services/bridge/mock.ts`, the `getMissingDependencies` mock already returns `[]` — no change needed.

**Step 5: Run frontend lint**

Run: `cd convx-app && npm run lint`
Expected: No errors.

**Step 6: Commit**

```bash
git add convx-app/src/components/DependencySetupWizard.vue
git commit -m "feat(wizard): group deps into categories with quirky setup copy"
```

---

### Task 4: Rewrite `convx-bundled.iss` — direct file placement, no MSI

**Files:**
- Modify: `installers/windows/convx-bundled.iss`

**Step 1: Rewrite the ISS file**

The new ISS takes a `TauriDir` parameter (path to the Tauri build output directory containing convx.exe) instead of `AppMsi`. It copies files directly.

```iss
; ConvX Windows bundled installer (Inno Setup)
;
; Usage:
;   iscc /DTauriDir="C:\path\to\tauri-output" /DDepsDir="C:\path\to\deps" convx-bundled.iss

#define MyAppName "ConvX"
#define MyAppPublisher "JSB Technologies"
#define MyAppExeName "convx.exe"
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef TauriDir
  #error TauriDir define is required. Path to directory containing convx.exe from Tauri build.
#endif
#ifndef DepsDir
  #error DepsDir define is required. Example: /DDepsDir="C:\path\to\deps"
#endif

[Setup]
AppId={{E11F2AA0-46DB-4E79-BB2A-4F6F6A65A6EA}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\convx
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=ConvX-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
LicenseFile=..\EULA.txt
DiskSpanning=no

[Files]
; Tauri app binary + resources
Source: "{#TauriDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

; MCP wrapper
Source: "convx-mcp.cmd"; DestDir: "{app}"; Flags: ignoreversion

; Bundled dependencies
Source: "{#DepsDir}\bin\*"; DestDir: "{app}\deps\bin"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\lib\*"; DestDir: "{app}\deps\lib"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\LibreOffice\*"; DestDir: "{app}\deps\LibreOffice"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\python\*"; DestDir: "{app}\deps\python"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\wheels\*"; DestDir: "{app}\deps\wheels"; Flags: ignoreversion recursesubdirs

[Run]
; Install Python packages from bundled wheels using bundled pip (no venv needed)
Filename: "{app}\deps\python\python.exe"; Parameters: "-m pip install --no-index --find-links ""{app}\deps\wheels"" pandas openpyxl weasyprint pdf2docx PyMuPDF mobi pyarrow numpy h5py"; StatusMsg: "Setting up conversion tools..."; Flags: runhidden waituntilterminated runasoriginaluser
; Launch app
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\deps\bin;{app}\deps\lib"; Check: NeedsAddPath('{app}\deps\bin')

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\.convx\venv"
```

Key changes:
- `ArchitecturesAllowed=x64compatible` + `ArchitecturesInstallIn64BitMode=x64compatible`
- No MSI — files copied directly from `TauriDir`
- Pip runs via `python.exe -m pip` instead of `Scripts\pip.exe` (more reliable with embeddable Python)
- StatusMsg says "Setting up conversion tools..." not "Configuring components..."

**Step 2: Commit**

```bash
git add installers/windows/convx-bundled.iss
git commit -m "fix(installer): direct file placement, drop MSI wrapper, add x64 arch"
```

---

### Task 5: Update `build-bundled-installer.ps1` — accept TauriDir instead of MSI

**Files:**
- Modify: `installers/windows/build-bundled-installer.ps1`

**Step 1: Rewrite the build script**

The script now finds and extracts the Tauri build output (exe + resources) instead of passing an MSI.

```powershell
<#
.SYNOPSIS
Builds the convx Windows bundled installer EXE (Inno Setup + deps).

.PARAMETER TauriDir
Optional explicit path to directory containing convx.exe from Tauri build.

.PARAMETER DepsDir
Optional path to deps directory (defaults to .\deps).

.PARAMETER AppVersion
Optional version override (defaults to tauri.conf.json version).

.PARAMETER OutputDir
Optional output directory for generated EXE.
#>
[CmdletBinding()]
param(
  [string]$TauriDir = "",
  [string]$DepsDir = "",
  [string]$AppVersion = "",
  [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path "$PSScriptRoot/../.."
$issPath = Resolve-Path "$PSScriptRoot/convx-bundled.iss"

# Resolve version
if ([string]::IsNullOrWhiteSpace($AppVersion)) {
  $tauriConfigPath = "$repoRoot/convx-app/src-tauri/tauri.conf.json"
  if (Test-Path $tauriConfigPath) {
    $tauriConfig = Get-Content $tauriConfigPath -Raw | ConvertFrom-Json
    if ($tauriConfig.version) {
      $AppVersion = [string]$tauriConfig.version
    }
  }
  if ([string]::IsNullOrWhiteSpace($AppVersion)) {
    $AppVersion = "0.1.0"
  }
}

# Resolve Tauri output directory
if ([string]::IsNullOrWhiteSpace($TauriDir)) {
  # Look for convx.exe in the Tauri release build output
  $TauriBuildDir = "$repoRoot/target/release"
  $TauriExe = Get-ChildItem -Path $TauriBuildDir -Filter "convx.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $TauriExe) {
    Write-Error "convx.exe not found in $TauriBuildDir. Build first: cd convx-app && cargo tauri build"
  }
  $TauriDir = $TauriBuildDir
}
if (-not (Test-Path (Join-Path $TauriDir "convx.exe"))) {
  Write-Error "convx.exe not found in TauriDir: $TauriDir"
}

# Resolve deps directory
if ([string]::IsNullOrWhiteSpace($DepsDir)) {
  $DepsDir = Join-Path $PSScriptRoot "deps"
}
if (-not (Test-Path $DepsDir)) {
  Write-Error "Deps directory not found at: $DepsDir. Run collect-deps-windows.ps1 first."
}

# Resolve output directory
if (-not [string]::IsNullOrWhiteSpace($OutputDir)) {
  if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
  }
  $OutputDir = (Resolve-Path $OutputDir).Path
}

# Find Inno Setup compiler
$iscc = Get-Command iscc -ErrorAction SilentlyContinue
if (-not $iscc) {
  Write-Error "Inno Setup compiler (iscc) not found in PATH. Install Inno Setup first."
}

Write-Host "Building bundled ConvX installer..."
Write-Host "  Tauri dir: $TauriDir"
Write-Host "  Deps: $DepsDir"
Write-Host "  Version: $AppVersion"

Push-Location $PSScriptRoot
try {
  $isccArgs = @(
    "/DTauriDir=$TauriDir",
    "/DDepsDir=$DepsDir",
    "/DAppVersion=$AppVersion"
  )

  if (-not [string]::IsNullOrWhiteSpace($OutputDir)) {
    $isccArgs += "/DOutputDir=$OutputDir"
  }

  $isccArgs += "$issPath"

  & iscc @isccArgs
  if ($LASTEXITCODE -ne 0) {
    Write-Error "Inno Setup compilation failed with exit code $LASTEXITCODE"
  }

  Write-Host "Bundled installer built successfully (version $AppVersion)"
} finally {
  Pop-Location
}
```

**Step 2: Commit**

```bash
git add installers/windows/build-bundled-installer.ps1
git commit -m "fix(build): accept TauriDir instead of MSI for bundled installer"
```

---

### Task 6: Add Windows `ensure_post_install` to `commands.rs`

**Files:**
- Modify: `convx-app/src-tauri/src/commands.rs:584-778`

**Step 1: Add a `#[cfg(target_os = "windows")]` block in `ensure_post_install`**

Currently the function only has `#[cfg(target_os = "macos")]` logic. Add a Windows block that installs missing pip modules using bundled Python (no venv):

```rust
#[cfg(target_os = "windows")]
{
    // Install missing pip modules using bundled Python directly
    let bundled_pip = DependencyChecker::bundled_pip_path();
    let pip_path = bundled_pip.or_else(|| DependencyChecker::convx_pip());

    if let Some(pip) = pip_path {
        let modules = [
            "pandas", "openpyxl", "weasyprint", "pdf2docx", "PyMuPDF",
            "mobi", "pyarrow", "numpy", "h5py",
        ];
        let missing_modules: Vec<&str> = modules
            .iter()
            .filter(|m| !DependencyChecker::python_has_module(m))
            .copied()
            .collect();

        if !missing_modules.is_empty() {
            let mut cmd = Command::new(&pip);
            cmd.arg("install");
            if let Some(wheels) = DependencyChecker::bundled_wheels_dir() {
                cmd.args(["--find-links", &wheels.to_string_lossy()]);
            }
            cmd.args(&missing_modules);
            match cmd.output() {
                Ok(out) if out.status.success() => {
                    repairs.push(format!(
                        "Installed missing modules: {}",
                        missing_modules.join(", ")
                    ));
                }
                _ => {
                    repairs.push(format!(
                        "Could not auto-install modules: {}",
                        missing_modules.join(", ")
                    ));
                }
            }
        }
    }
}
```

Note: `bundled_pip_path` needs to be `pub` (change from `fn` to `pub fn` in deps.rs — do this during Task 1).

**Step 2: Build**

Run: `cd convx-core && cargo build`
Expected: Clean compile.

**Step 3: Commit**

```bash
git add convx-app/src-tauri/src/commands.rs convx-core/src/utils/deps.rs
git commit -m "feat(windows): add ensure_post_install for bundled Python module repair"
```

---

### Task 7: Update CI workflow for new build script parameters

**Files:**
- Modify: `.github/workflows/build-installers.yml`

**Step 1: Find and update the bundled installer build step**

Change the CI step that calls `build-bundled-installer.ps1` to pass `-TauriDir` instead of `-MsiPath`. The Tauri build output directory for Windows is typically `target/release/` (where `convx.exe` lives).

Look for the step that runs something like:
```powershell
.\build-bundled-installer.ps1 -MsiPath $msiPath
```

Replace with:
```powershell
.\build-bundled-installer.ps1 -TauriDir "$env:GITHUB_WORKSPACE\target\release"
```

**Step 2: Commit**

```bash
git add .github/workflows/build-installers.yml
git commit -m "ci: update bundled installer build to use TauriDir"
```

---

### Task 8: Verify full build on Windows machine

**Steps (manual, on Windows):**

1. `cd convx-app && cargo tauri build` — produces `target\release\convx.exe`
2. `cd installers\windows && .\collect-deps-windows.ps1` — collects deps
3. `.\build-bundled-installer.ps1` — builds `ConvX-Setup.exe`
4. Run `ConvX-Setup.exe` — verify:
   - Installs to `C:\Program Files\convx\` (not x86)
   - No "Unable to execute" errors
   - App launches, "Verifying setup" shows 4 grouped items with quirky labels
   - All 4 groups go green
   - "You're all set" screen appears

**Commit if any fixes needed during testing.**
