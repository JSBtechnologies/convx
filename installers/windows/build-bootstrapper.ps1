<#
.SYNOPSIS
Builds the convx Windows bootstrapper EXE (Inno Setup).

.PARAMETER MsiPath
Optional explicit path to the Tauri-generated MSI.

.PARAMETER AppVersion
Optional version override (defaults to tauri.conf.json version).

.PARAMETER OutputDir
Optional output directory for generated EXE.
#>
[CmdletBinding()]
param(
  [string]$MsiPath = "",
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
$issPath = Resolve-Path "$PSScriptRoot/convx-bootstrapper.iss"

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

if ([string]::IsNullOrWhiteSpace($MsiPath)) {
  # Workspace builds output to repo-root target/, not convx-app/src-tauri/target/
  $MsiPath = Find-Msi "$repoRoot/target/release/bundle"
}

if (-not $MsiPath) {
  Write-Error "No MSI found. Build one first: cd convx-app && cargo tauri build"
}

if (-not (Test-Path $MsiPath)) {
  Write-Error "MSI not found at path: $MsiPath"
}

if (-not [string]::IsNullOrWhiteSpace($OutputDir)) {
  if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
  }
  $OutputDir = (Resolve-Path $OutputDir).Path
}

$iscc = Get-Command iscc -ErrorAction SilentlyContinue
if (-not $iscc) {
  Write-Error "Inno Setup compiler (iscc) not found in PATH. Install Inno Setup first."
}

Push-Location $PSScriptRoot
try {
  $isccArgs = @(
    "/DAppMsi=$MsiPath",
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

  Write-Host "Bootstrapper built successfully (version $AppVersion)"
} finally {
  Pop-Location
}
