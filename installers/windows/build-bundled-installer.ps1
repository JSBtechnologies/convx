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
  # Check default install locations
  $defaultPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\iscc.exe",
    "C:\Program Files\Inno Setup 6\iscc.exe"
  )
  foreach ($p in $defaultPaths) {
    if (Test-Path $p) {
      $iscc = $p
      break
    }
  }
  if (-not $iscc) {
    Write-Error "Inno Setup compiler (iscc) not found in PATH or default locations. Install Inno Setup first."
  }
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

  $isccPath = if ($iscc -is [string]) { $iscc } else { "iscc" }
  & $isccPath @isccArgs
  if ($LASTEXITCODE -ne 0) {
    Write-Error "Inno Setup compilation failed with exit code $LASTEXITCODE"
  }

  Write-Host "Bundled installer built successfully (version $AppVersion)"
} finally {
  Pop-Location
}
