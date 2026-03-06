# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**convx** is a local-first file conversion tool with three surfaces sharing a single Rust core:
- **CLI** (`convx-core/src/main.rs`) — clap-based command-line tool
- **Desktop App** (`convx-app/`) — Tauri v2 + Quasar v2 (Vue 3)
- **MCP Server** (`convx-core/src/mcp/main.rs`) — Model Context Protocol server

A separate **license server** (`license-server/`) runs on Cloudflare Workers with Supabase.

## Build & Development Commands

### Rust Core (`convx-core/`)
```bash
cargo build                        # build library + CLI
cargo build --release              # release build
cargo test                         # unit tests
cargo test -- --ignored            # integration tests (need fixtures + ffmpeg/libvips)
cargo test test_name               # run a single test
cargo fmt --check                  # format check (CI enforced)
cargo clippy -- -D warnings        # lint (CI enforced)
```

Integration tests require fixtures: `cd convx-core && ./tests/generate_fixtures.sh`
System deps: `ffmpeg`, `libvips` (macOS: `brew install ffmpeg vips`)
Optional Python deps for ML/data formats: `pip install pyarrow numpy h5py` (managed via `~/.convx/venv`)

### Desktop App (`convx-app/`)
```bash
cd convx-app
npm install                        # install frontend deps
npm run dev                        # Quasar SPA dev server (port 1420, no Tauri)
npm run build                      # production frontend build
npm run lint                       # ESLint
npm run tauri:dev                  # full Tauri desktop dev (requires cargo-tauri v2)
npm run tauri:bundle               # production desktop build
```

Tauri CLI: `cargo install tauri-cli --version "^2"`

### License Server (`license-server/`)
```bash
cd license-server
npm install
npm run dev                        # wrangler local dev
npm run deploy                     # deploy to Cloudflare Workers
```

Secrets are set via `wrangler secret put <NAME>` (see `wrangler.toml` for list).

## Architecture

### Cargo Workspace
Root `Cargo.toml` defines workspace members: `convx-core` and `convx-app/src-tauri`. The Tauri backend depends on `convx-core` via path: `convx-core = { path = "../../convx-core" }`.

### convx-core
- **`engine.rs`** — `ConvxEngine` orchestrator holding 6 converter trait objects
- **`converters/`** — `image.rs`, `video.rs`, `audio.rs`, `document.rs`, `data.rs`, `ebook.rs` — each implements the `Converter` trait
- **`types/`** — `format.rs` (54-format enum across 6 categories: Image, Video, Audio, Document, Data, Ebook), `options.rs` (per-domain option structs), `result.rs`, `error.rs` (ConvxError), `preset.rs`
- **`license/`** — `mod.rs` (LicenseStatus enum), `api.rs` (server calls), `crypto.rs` (Ed25519), `keyfile.rs` (~/.convx/license.json), `fingerprint.rs` (device ID), `enterprise.rs`
- **`presets/`** — built-in conversion presets (discord, twitter, heic-to-jpg, parquet-to-csv, data-to-pdf, etc.)
- **Library name is `convx`** (not `convx_core`): `use convx::ConvxEngine`
- Feature flag `cli` (default) gates clap/rayon/glob/indicatif deps; the library can be used without it

### convx-app (Tauri + Quasar)

**Bridge Pattern** (`src/services/bridge/`):
- `index.ts` — detects Tauri via `window.__TAURI_INTERNALS__`, returns `TauriBridge` or `MockBridge`
- `tauri.ts` — real IPC calls via `@tauri-apps/api/core` invoke
- `mock.ts` — browser-safe mock with simulated responses for SPA dev without Tauri

**Tauri Backend** (`src-tauri/`):
- `lib.rs` — registers all Tauri commands and manages `ConvxState` (engine + cancel flag)
- `commands.rs` — IPC command handlers (conversion, formats, dependencies, license, enterprise); includes path security validation blocking sensitive system roots

**Frontend** (Vue 3 + TypeScript):
- Pinia stores: `conversion.ts`, `settings.ts`, `history.ts`
- Pages: `ConvertPage`, `HistoryPage`, `SettingsPage` in `MainLayout`
- Composable: `useConvert.ts` for conversion flow logic
- Progress via Tauri event `'conversion-progress'` with stage + percent

### License Server
Cloudflare Workers + Supabase. Routes: `activate`, `validate`, `transfer`, `deactivate`, `admin`, `download`, `discount`, `webhook` (LemonSqueezy), `org` (enterprise). Ed25519 signing via `@noble/ed25519`.

## Critical Gotchas

- **Pinia initialization**: `src/stores/index.ts` MUST exist and export a Quasar store plugin wrapping `createPinia()`. Without it, stores silently fail and the app renders blank.
- **Tauri v2 CSP**: `tauri.conf.json` must include `script-src 'unsafe-inline' 'unsafe-eval'` for Vite dev mode.
- **Tauri v2 API imports**: Use `@tauri-apps/api/core` (not the old `@tauri-apps/api/tauri`).
- **Dynamic imports for Tauri APIs**: Always use `await import('@tauri-apps/api/...')` to prevent crashes when running as SPA without Tauri.
- **Quasar entry point**: Uses `<!-- quasar:entry-point -->` comment (not `<div id="q-app">`).
- **Clear `.quasar/` cache** after modifying `src/stores/index.ts` or `quasar.config.ts`.
- **Quasar has no Tauri mode**: Tauri v2 is integrated manually (no `quasar mode add tauri`).

## CI Pipeline

Runs on push/PR (`.github/workflows/ci.yml`):
1. `cargo fmt --check` (in `convx-core/`)
2. `cargo clippy -- -D warnings` (in `convx-core/`)
3. `cargo test` — unit tests
4. `cargo test -- --ignored` — integration tests (with fixture generation)
