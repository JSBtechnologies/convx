$ErrorActionPreference = "Stop"

$LogFile = $env:CONVX_INSTALLER_LOG
if ([string]::IsNullOrWhiteSpace($LogFile)) {
  $LogFile = Join-Path $env:TEMP "convx-installer.log"
}

$LogDir = Split-Path -Parent $LogFile
if (-not [string]::IsNullOrWhiteSpace($LogDir) -and -not (Test-Path $LogDir)) {
  New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}

Start-Transcript -Path $LogFile -Append | Out-Null

try {

Write-Host "convx dependency bootstrap (Windows)"
Write-Host "------------------------------------"

function Test-Command($Name) {
  return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [string[]]$Arguments = @(),
    [Parameter(Mandatory = $true)][string]$FailureMessage
  )

  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$FailureMessage (exit code: $LASTEXITCODE)"
  }
}

# System packages via winget or chocolatey
if (Test-Command winget) {
  Write-Host "Using winget to install dependencies..."

  $WingetPackages = @(
    @{ Name = "FFmpeg";      Id = "Gyan.FFmpeg" },
    @{ Name = "libvips";     Id = "libvips.libvips" },
    @{ Name = "Pandoc";      Id = "JohnMacFarlane.Pandoc" },
    @{ Name = "LibreOffice"; Id = "TheDocumentFoundation.LibreOffice" },
    @{ Name = "Python";      Id = "Python.Python.3.13" }
  )

  foreach ($pkg in $WingetPackages) {
    Write-Host "Installing $($pkg.Name)..."
    Invoke-Checked -FilePath "winget" -Arguments @(
      "install", "-e", "--id", $pkg.Id,
      "--accept-package-agreements", "--accept-source-agreements"
    ) -FailureMessage "$($pkg.Name) install failed via winget"
  }

  # Poppler for Windows (no winget package — use chocolatey or manual)
  if (Test-Command choco) {
    Invoke-Checked -FilePath "choco" -Arguments @("install", "-y", "poppler") -FailureMessage "Poppler install failed via chocolatey"
  } else {
    Write-Host "WARNING: Poppler (pdftoppm) not available via winget. Install manually or via chocolatey."
  }
} elseif (Test-Command choco) {
  Write-Host "Using chocolatey to install dependencies..."
  Invoke-Checked -FilePath "choco" -Arguments @(
    "install", "-y", "ffmpeg", "vips", "pandoc", "poppler",
    "libreoffice-fresh", "python3"
  ) -FailureMessage "Dependency install failed via chocolatey"
} else {
  Write-Host "Neither winget nor chocolatey were found."
  Write-Host "Install manually:"
  Write-Host "  FFmpeg: https://ffmpeg.org/download.html"
  Write-Host "  libvips: https://www.libvips.org/install.html"
  Write-Host "  Pandoc: https://pandoc.org/installing.html"
  Write-Host "  LibreOffice: https://www.libreoffice.org/download/"
  exit 1
}

# Python venv + pip packages
Write-Host "Setting up Python virtual environment..."
$VenvDir = Join-Path $env:USERPROFILE ".convx\venv"
if (-not (Test-Path (Join-Path $VenvDir "Scripts\python.exe"))) {
  $null = New-Item -ItemType Directory -Path (Split-Path $VenvDir) -Force
  Invoke-Checked -FilePath "python" -Arguments @("-m", "venv", $VenvDir) -FailureMessage "Failed to create Python venv"
}

$VenvPip = Join-Path $VenvDir "Scripts\pip.exe"
Write-Host "Installing Python packages..."
Invoke-Checked -FilePath $VenvPip -Arguments @("install", "--upgrade", "pip") -FailureMessage "pip upgrade failed"
Invoke-Checked -FilePath $VenvPip -Arguments @("install", "pandas", "openpyxl", "weasyprint", "pdf2docx", "mobi") -FailureMessage "Python package install failed"

# Verify
Write-Host "Verifying installation..."
if (-not (Test-Command ffmpeg)) { throw "ffmpeg is still not available on PATH" }
if (-not (Test-Command vips)) { throw "vips is still not available on PATH" }
if (-not (Test-Command pandoc)) { throw "pandoc is still not available on PATH" }
if (-not (Test-Command soffice)) { Write-Host "WARNING: LibreOffice (soffice) not on PATH yet — may need restart" }

ffmpeg -version | Select-Object -First 1 | Out-Host
vips --version | Out-Host
pandoc --version | Select-Object -First 1 | Out-Host

$VenvPython = Join-Path $VenvDir "Scripts\python.exe"
& $VenvPython -c "import pandas, openpyxl, weasyprint, pdf2docx, mobi; print('Python modules OK')" | Out-Host

Write-Host "convx prerequisites installed successfully"

} finally {
  Stop-Transcript | Out-Null
}
