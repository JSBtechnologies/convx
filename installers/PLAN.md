# convx Installer Plan (Bootstrap + App)

## Goal

Deliver a frictionless flow:

1. Download installer
2. Accept license
3. Install prerequisites automatically
4. Install app
5. Launch app

## Strategy

### Windows (Primary first)

- Build Tauri MSI (`cargo tauri build`)
- Wrap MSI with an Inno Setup bootstrapper EXE
- Bootstrapper flow:
  1. Show EULA
  2. Run dependency bootstrap (`bootstrap-windows.ps1`)
  3. Install MSI silently
  4. Launch convx

### macOS

- Build Tauri DMG (`cargo tauri build`)
- Use bootstrap shell script to install prerequisites (`bootstrap-macos.sh`)
- Open DMG and guide user through app install
- Keep in-app dependency wizard as fallback safety net

### Linux

- Use distro-native package manager dependency flow (`bootstrap-linux.sh`)
- Keep app packaging separate per distro

## Milestones

- [x] Cross-platform dependency bootstrap scripts
- [x] In-app dependency setup wizard fallback
- [x] Installer manifest for pinned minimums
- [x] CI workflow to build installer artifacts
- [ ] Windows bootstrap EXE production in release pipeline
- [ ] macOS signed/notarized DMG flow
- [ ] Linux package publishing pipeline

## Notes

This approach balances UX and maintainability while preserving local-first behavior.
