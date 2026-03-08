<#
.SYNOPSIS
Collects portable/static dependency builds for bundled Windows installer.

.DESCRIPTION
Downloads and stages all dependencies into a deps\ directory:
  deps\bin\       ffmpeg.exe, ffprobe.exe, vips.exe, pandoc.exe, pdftoppm.exe
  deps\lib\       DLLs for vips, poppler
  deps\LibreOffice\program\soffice.exe  (LibreOffice Portable, stripped)
  deps\python\    Embeddable Python (+ pip support)
  deps\wheels\    Offline .whl files

.PARAMETER DepsDir
Output directory (defaults to .\deps).

.PARAMETER SkipFfmpeg
Skip FFmpeg download.

.PARAMETER SkipVips
Skip libvips download.

.PARAMETER SkipPandoc
Skip Pandoc download.

.PARAMETER SkipPoppler
Skip Poppler (pdftoppm) download.

.PARAMETER SkipLibreOffice
Skip LibreOffice Portable download.

.PARAMETER SkipPython
Skip Python download.

.PARAMETER SkipWheels
Skip Python wheel download.
#>
[CmdletBinding()]
param(
  [string]$DepsDir = "",
  [switch]$SkipFfmpeg,
  [switch]$SkipVips,
  [switch]$SkipPandoc,
  [switch]$SkipPoppler,
  [switch]$SkipLibreOffice,
  [switch]$SkipPython,
  [switch]$SkipWheels
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($DepsDir)) {
  $DepsDir = Join-Path $PSScriptRoot "deps"
}

$BinDir = Join-Path $DepsDir "bin"
$LibDir = Join-Path $DepsDir "lib"
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
New-Item -ItemType Directory -Path $LibDir -Force | Out-Null

$TempDir = Join-Path $env:TEMP "convx-deps-dl"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

# Versions
$VIPS_VERSION = "8.16.1"
$PANDOC_VERSION = "3.6.4"
$POPPLER_VERSION = "24.08.0-0"
$PYTHON_VERSION = "3.13.12"
$LO_VERSION = "25.2.7"

function Download-File {
  param([string]$Url, [string]$Dest)
  if (Test-Path $Dest) {
    Write-Host "    Using cached: $(Split-Path -Leaf $Dest)"
    return
  }
  Write-Host "    Downloading: $Url"
  Invoke-WebRequest -Uri $Url -OutFile $Dest -UseBasicParsing
}

# ── 1. FFmpeg (static GPL build from BtbN) ─────────────────────

if (-not $SkipFfmpeg) {
  Write-Host "==> Collecting FFmpeg..."

  $FfmpegZip = Join-Path $TempDir "ffmpeg.zip"
  $FfmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
  Download-File $FfmpegUrl $FfmpegZip

  $FfmpegExtract = Join-Path $TempDir "ffmpeg"
  if (-not (Test-Path $FfmpegExtract)) {
    Expand-Archive -Path $FfmpegZip -DestinationPath $FfmpegExtract -Force
  }

  $FfmpegBinDir = Get-ChildItem -Path $FfmpegExtract -Recurse -Directory -Filter "bin" | Select-Object -First 1
  if ($FfmpegBinDir) {
    Copy-Item (Join-Path $FfmpegBinDir.FullName "ffmpeg.exe") $BinDir -Force
    Copy-Item (Join-Path $FfmpegBinDir.FullName "ffprobe.exe") $BinDir -Force
    Write-Host "    Copied ffmpeg.exe + ffprobe.exe"
  } else {
    Write-Host "    Warning: ffmpeg bin dir not found in archive"
  }
}

# ── 2. libvips (pre-built from GitHub) ─────────────────────────

if (-not $SkipVips) {
  Write-Host "==> Collecting libvips..."

  $VipsZip = Join-Path $TempDir "vips.zip"
  $VipsUrl = "https://github.com/libvips/build-win64-mxe/releases/download/v${VIPS_VERSION}/vips-dev-w64-all-${VIPS_VERSION}.zip"
  Download-File $VipsUrl $VipsZip

  $VipsExtract = Join-Path $TempDir "vips"
  if (-not (Test-Path $VipsExtract)) {
    Expand-Archive -Path $VipsZip -DestinationPath $VipsExtract -Force
  }

  # vips ships with bin/vips.exe and all its DLLs in bin/
  $VipsBinDir = Get-ChildItem -Path $VipsExtract -Recurse -Directory -Filter "bin" | Select-Object -First 1
  if ($VipsBinDir) {
    Copy-Item (Join-Path $VipsBinDir.FullName "vips.exe") $BinDir -Force
    # Copy DLLs to bin/ (alongside exe for immediate discovery) and lib/ (for Python native lib path)
    Get-ChildItem -Path $VipsBinDir.FullName -Filter "*.dll" | ForEach-Object {
      Copy-Item $_.FullName $BinDir -Force
      Copy-Item $_.FullName $LibDir -Force
    }
    Write-Host "    Copied vips.exe + DLLs"
  } else {
    Write-Host "    Warning: vips bin dir not found in archive"
  }
}

# ── 3. Pandoc (static binary from GitHub) ──────────────────────

if (-not $SkipPandoc) {
  Write-Host "==> Collecting Pandoc..."

  $PandocZip = Join-Path $TempDir "pandoc.zip"
  $PandocUrl = "https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-windows-x86_64.zip"
  Download-File $PandocUrl $PandocZip

  $PandocExtract = Join-Path $TempDir "pandoc"
  if (-not (Test-Path $PandocExtract)) {
    Expand-Archive -Path $PandocZip -DestinationPath $PandocExtract -Force
  }

  $PandocExe = Get-ChildItem -Path $PandocExtract -Recurse -Filter "pandoc.exe" | Select-Object -First 1
  if ($PandocExe) {
    Copy-Item $PandocExe.FullName $BinDir -Force
    Write-Host "    Copied pandoc.exe"
  } else {
    Write-Host "    Warning: pandoc.exe not found in archive"
  }
}

# ── 4. Poppler (pdftoppm from oschwartz10612 release) ──────────

if (-not $SkipPoppler) {
  Write-Host "==> Collecting Poppler (pdftoppm)..."

  $PopplerZip = Join-Path $TempDir "poppler.zip"
  $PopplerUrl = "https://github.com/oschwartz10612/poppler-windows/releases/download/v${POPPLER_VERSION}/Release-${POPPLER_VERSION}.zip"
  Download-File $PopplerUrl $PopplerZip

  $PopplerExtract = Join-Path $TempDir "poppler"
  if (-not (Test-Path $PopplerExtract)) {
    Expand-Archive -Path $PopplerZip -DestinationPath $PopplerExtract -Force
  }

  $PopplerBinDir = Get-ChildItem -Path $PopplerExtract -Recurse -Directory -Filter "bin" | Select-Object -First 1
  if ($PopplerBinDir) {
    Copy-Item (Join-Path $PopplerBinDir.FullName "pdftoppm.exe") $BinDir -Force
    # Copy poppler DLLs to bin/ (alongside exe) and lib/ (for Python native lib path)
    Get-ChildItem -Path (Join-Path $PopplerBinDir.FullName "..") -Recurse -Filter "*.dll" | ForEach-Object {
      if (-not (Test-Path (Join-Path $BinDir $_.Name))) {
        Copy-Item $_.FullName $BinDir -Force
      }
      if (-not (Test-Path (Join-Path $LibDir $_.Name))) {
        Copy-Item $_.FullName $LibDir -Force
      }
    }
    Write-Host "    Copied pdftoppm.exe + DLLs"
  } else {
    Write-Host "    Warning: poppler bin dir not found in archive"
  }
}

# ── 5. LibreOffice (portable/standard install, stripped) ───────

if (-not $SkipLibreOffice) {
  Write-Host "==> Collecting LibreOffice..."

  $LoDir = Join-Path $DepsDir "LibreOffice"
  $LoMsi = Join-Path $TempDir "libreoffice.msi"
  $LoUrl = "https://download.documentfoundation.org/libreoffice/stable/${LO_VERSION}/win/x86_64/LibreOffice_${LO_VERSION}_Win_x86-64.msi"
  Download-File $LoUrl $LoMsi

  # Extract MSI contents (without installing)
  $LoExtract = Join-Path $TempDir "libreoffice"
  if (-not (Test-Path $LoExtract)) {
    Write-Host "    Extracting LibreOffice MSI (this takes a while)..."
    Start-Process -FilePath "msiexec.exe" -ArgumentList "/a `"$LoMsi`" TARGETDIR=`"$LoExtract`" /qn" -Wait -NoNewWindow
  }

  # Find the program directory
  $LoProgramDir = Get-ChildItem -Path $LoExtract -Recurse -Directory -Filter "program" |
    Where-Object { Test-Path (Join-Path $_.FullName "soffice.exe") } |
    Select-Object -First 1

  if ($LoProgramDir) {
    New-Item -ItemType Directory -Path $LoDir -Force | Out-Null
    Copy-Item -Path $LoProgramDir.FullName -Destination $LoDir -Recurse -Force

    # Strip unnecessary components
    $StripDirs = @("python-core-*", "wizards", "gallery", "template", "autocorr", "wordbook")
    foreach ($pattern in $StripDirs) {
      Get-ChildItem -Path $LoDir -Recurse -Directory -Filter $pattern -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue }
    }

    $LoSizeMB = [math]::Round((Get-ChildItem -Path $LoDir -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB)
    Write-Host "    Extracted LibreOffice (${LoSizeMB} MB)"
  } else {
    Write-Host "    Warning: soffice.exe not found in extracted MSI"
  }
}

# ── 6. Python (embeddable + pip) ───────────────────────────────

if (-not $SkipPython) {
  Write-Host "==> Collecting Python..."

  $PyDir = Join-Path $DepsDir "python"
  $PyZip = Join-Path $TempDir "python-embed.zip"
  $PyUrl = "https://www.python.org/ftp/python/${PYTHON_VERSION}/python-${PYTHON_VERSION}-embed-amd64.zip"
  Download-File $PyUrl $PyZip

  if (-not (Test-Path $PyDir)) {
    New-Item -ItemType Directory -Path $PyDir -Force | Out-Null
    Expand-Archive -Path $PyZip -DestinationPath $PyDir -Force
  }

  # Enable pip in embeddable Python by uncommenting import site in python*._pth
  $PthFile = Get-ChildItem -Path $PyDir -Filter "python*._pth" | Select-Object -First 1
  if ($PthFile) {
    $content = Get-Content $PthFile.FullName
    $content = $content -replace "^#import site", "import site"
    Set-Content $PthFile.FullName $content
  }

  # Download and install pip
  $GetPipPy = Join-Path $TempDir "get-pip.py"
  if (-not (Test-Path $GetPipPy)) {
    Download-File "https://bootstrap.pypa.io/get-pip.py" $GetPipPy
  }

  $PyExe = Join-Path $PyDir "python.exe"
  Write-Host "    Installing pip..."
  & $PyExe $GetPipPy --no-warn-script-location 2>&1 | Out-Null

  $PySizeMB = [math]::Round((Get-ChildItem -Path $PyDir -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB)
  Write-Host "    Bundled Python ${PYTHON_VERSION} (${PySizeMB} MB)"
}

# ── 7. Python wheels ──────────────────────────────────────────

if (-not $SkipWheels) {
  Write-Host "==> Collecting Python wheels..."

  $WheelsDir = Join-Path $DepsDir "wheels"
  New-Item -ItemType Directory -Path $WheelsDir -Force | Out-Null

  $Modules = @("pandas", "openpyxl", "weasyprint", "pdf2docx", "PyMuPDF", "mobi", "pyarrow", "numpy", "h5py")

  $PyExe = Join-Path $DepsDir "python\python.exe"
  $PipExe = Join-Path $DepsDir "python\Scripts\pip.exe"

  if (Test-Path $PipExe) {
    Write-Host "    Downloading wheels for: $($Modules -join ', ')"
    & $PipExe download `
      --only-binary=:all: `
      --platform win_amd64 `
      --python-version 3.13 `
      --dest $WheelsDir `
      @Modules 2>&1 | ForEach-Object { Write-Host "    $_" }

    $WheelCount = (Get-ChildItem -Path $WheelsDir -Filter "*.whl").Count
    $WheelSizeMB = [math]::Round((Get-ChildItem -Path $WheelsDir -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB)
    Write-Host "    Downloaded ${WheelCount} wheels (${WheelSizeMB} MB)"
  } elseif (Get-Command pip3 -ErrorAction SilentlyContinue) {
    Write-Host "    Using system pip to download wheels..."
    & pip3 download `
      --only-binary=:all: `
      --platform win_amd64 `
      --python-version 3.13 `
      --dest $WheelsDir `
      @Modules 2>&1 | ForEach-Object { Write-Host "    $_" }
  } else {
    Write-Host "    Warning: No pip available, skipping wheel download"
  }
}

# ── Summary ────────────────────────────────────────────────────

Write-Host ""
Write-Host "=== Windows dependency collection complete ==="
Write-Host ""

function Show-DirSize($Path, $Label) {
  if (Test-Path $Path) {
    $sizeMB = [math]::Round((Get-ChildItem -Path $Path -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1MB)
    Write-Host "  ${Label}: ${sizeMB} MB"
  }
}

Show-DirSize $BinDir "bin/"
Show-DirSize $LibDir "lib/"
Show-DirSize (Join-Path $DepsDir "LibreOffice") "LibreOffice/"
Show-DirSize (Join-Path $DepsDir "python") "python/"
Show-DirSize (Join-Path $DepsDir "wheels") "wheels/"

$TotalMB = [math]::Round((Get-ChildItem -Path $DepsDir -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB)
Write-Host ""
Write-Host "  TOTAL: ${TotalMB} MB"
