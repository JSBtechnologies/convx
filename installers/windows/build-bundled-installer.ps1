<#
.SYNOPSIS
Builds the convx Windows bundled installer EXE (Inno Setup + deps).

.PARAMETER MsiPath
Optional explicit path to the Tauri-generated MSI.

.PARAMETER DepsDir
Optional path to deps directory (defaults to .\deps).

.PARAMETER AppVersion
Optional version override (defaults to tauri.conf.json version).

.PARAMETER OutputDir
Optional output directory for generated EXE.
#>
[CmdletBinding()]
param(
  [string]$MsiPath = "",
  [string]$DepsDir = "",
  [string]$AppVersion = "",
  [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

function Find-Msi {
  param([string]$Root)
  $candidates = Get-ChildItem -Path $Root -Recurse -Filter *.msi -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending
  if ($candidates.Count -eq 0) { return $null }
  return $candidates[0].FullName
}

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

# Resolve MSI path
if ([string]::IsNullOrWhiteSpace($MsiPath)) {
  $MsiPath = Find-Msi "$repoRoot/target/release/bundle"
}
if (-not $MsiPath -or -not (Test-Path $MsiPath)) {
  Write-Error "No MSI found. Build one first: cd convx-app && cargo tauri build"
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
Write-Host "  MSI: $MsiPath"
Write-Host "  Deps: $DepsDir"
Write-Host "  Version: $AppVersion"

Push-Location $PSScriptRoot
try {
  $isccArgs = @(
    "/DAppMsi=$MsiPath",
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
