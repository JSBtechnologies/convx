# convx Installer Bootstrap

This folder contains prerequisite bootstrap scripts used by installer flows.

Goal: keep purchase → download → install → first conversion as low-friction as possible.

## Recommended path by OS

- macOS: build/use unified PKG via [installers/macos/build-unified-installer.sh](macos/build-unified-installer.sh)
- Windows: build/use bootstrapper EXE via [installers/windows/build-bootstrapper.ps1](windows/build-bootstrapper.ps1)

## What gets installed

- `ffmpeg`
- `libvips`

## Scripts

- macOS: [installers/bootstrap-macos.sh](bootstrap-macos.sh)
- Windows: [installers/bootstrap-windows.ps1](bootstrap-windows.ps1)

Installer wrappers:

- Windows bootstrapper template (Inno Setup): [installers/windows/convx-bootstrapper.iss](windows/convx-bootstrapper.iss)
- Windows bootstrapper build script: [installers/windows/build-bootstrapper.ps1](windows/build-bootstrapper.ps1)
- macOS unified PKG build script: [installers/macos/build-unified-installer.sh](macos/build-unified-installer.sh)
- macOS run wrapper: [installers/macos/run-installer.sh](macos/run-installer.sh)

## Usage

### Shared environment variables

- `CONVX_INSTALLER_LOG` — override installer/bootstrap log path
	- default: `/tmp/convx-installer.log` on macOS
	- default: `%TEMP%\\convx-installer.log` on Windows
- `CONVX_SKIP_BOOTSTRAP=1` — skip dependency bootstrap in wrapper scripts (`run-installer.sh`)

### macOS

Run in terminal:

`bash installers/bootstrap-macos.sh`

### Windows (PowerShell as Admin)

`powershell -ExecutionPolicy Bypass -File installers/bootstrap-windows.ps1`

## Build a single Windows bootstrapper EXE

1) Build the Tauri MSI:

`cd convx-app && cargo tauri build`

2) Build the bootstrapper (requires Inno Setup / `iscc` in PATH):

`powershell -ExecutionPolicy Bypass -File installers/windows/build-bootstrapper.ps1`

Optional explicit version override:

`powershell -ExecutionPolicy Bypass -File installers/windows/build-bootstrapper.ps1 -AppVersion 0.1.0`

Optional explicit output directory:

`powershell -ExecutionPolicy Bypass -File installers/windows/build-bootstrapper.ps1 -OutputDir C:\build\out`

The output EXE includes:

- dependency install step
- MSI installation step
- post-install app launch

Output filename format:

- `convx-setup-bootstrapper-<version>.exe`

## Build a unified macOS installer PKG

1) Build unified installer package (auto-builds app bundle if needed):

`bash installers/macos/build-unified-installer.sh`

By default this script rebuilds the Tauri app so the package always contains latest UI/code changes.

Optional (advanced) skip rebuild:

`SKIP_TAURI_BUILD=1 bash installers/macos/build-unified-installer.sh`

Optional (advanced) keep temporary packaging workspace for debugging:

`KEEP_PKG_WORK_DIR=1 bash installers/macos/build-unified-installer.sh`

Optional package signing:

`MACOS_PKG_SIGN_IDENTITY="Developer ID Installer: Example Inc. (TEAMID)" bash installers/macos/build-unified-installer.sh`

Optional notarization (requires signing + configured `xcrun notarytool` keychain profile):

`MACOS_PKG_SIGN_IDENTITY="Developer ID Installer: Example Inc. (TEAMID)" MACOS_NOTARY_PROFILE="AC_PROFILE" bash installers/macos/build-unified-installer.sh`

If you previously ran `cargo tauri build` with `sudo`, you may get a permission error removing old bundle files. Fix once:

`sudo chown -R $(id -un):staff convx-app/src-tauri/target/release/bundle/macos`

Output filename format:

- `convx-installer-<version>.pkg`

Installer behavior:

- installs app to `/Applications`
- overwrites previous `/Applications/convx.app` install
- does **not** perform long dependency installs during package scripts (to avoid installer hangs)
- dependency setup is handled on first launch via guided in-app wizard
- launches app after install

## macOS DMG wrapper flow (fallback)

1) Build DMG:

`cd convx-app && cargo tauri build`

2) Run wrapper:

`bash installers/macos/run-installer.sh`

## Manifest

Pinned minimum expectations and install commands live in:

- [installers/dependency-manifest.json](dependency-manifest.json)

This is the source of truth for future installer/bootstrapper tooling.

## Post-install MCP setup (optional)

After installation, convx can run as an MCP server over stdio.

Run manually:

`convx mcp`

Or configure in an MCP client:

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

If `convx` is not on PATH yet, use the full binary path for your installation.
