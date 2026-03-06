# ConvX License Module v2 — Build, Fix & Test Prompt

## Context

You are working on ConvX, a local-first file conversion tool at `/Users/jeffriebudde/convx/`. The crate is `convx-core` at `/Users/jeffriebudde/convx/convx-core/`.

The license module at `src/license/` was just upgraded with hardened device fingerprinting and proprietary HMAC-based hashing. The changes have NOT been compiled yet. Your job is to make it compile cleanly with zero warnings, run all tests, and verify the new `convx fingerprint` CLI command works on this machine.

## What Changed (scope of new additions)

### 1. `Cargo.toml` — Added `hmac = "0.12"` dependency

### 2. `src/license/crypto.rs` — Proprietary HMAC-SHA256 fingerprint hashing
- Replaced the old `tiered_hash()` (plain SHA256 + salt) with `proprietary_tier_hash()`
- Uses HMAC-SHA256 with a compiled-in key (`HMAC_KEY`)
- Domain separation tags per tier (`TIER_DOMAINS`)
- Position-dependent separators (index XOR tier XOR constant)
- Two-round mixing: HMAC round → SHA256 finalization round
- Removed the old `FINGERPRINT_SALT` constant and `tiered_hash()` function
- The old `tiered_hash` is no longer used anywhere — `fingerprint.rs` now calls `proprietary_tier_hash`
- Ed25519 signature verification and `sha256_hex` are unchanged
- Tests cover: determinism, order sensitivity, tier sensitivity, component count sensitivity, output format

### 3. `src/license/fingerprint.rs` — Expanded hardware signal collection
- `DeviceFingerprint` struct now includes `schema_version: u8` field (set to `2`)
- `RawSignals` struct expanded from 7 fields to 14 fields:
  - **New Tier 1:** `cpu_microarch` (family/model/stepping/microcode), `tpm_hash` (Windows TPM 2.0 endorsement key)
  - **New Tier 2:** `total_ram`, `gpu_id`, `chassis_serial`, `firmware_version`
  - **New Tier 3:** `boot_disk_uuid`
- macOS collector: added `sysctl_value()` helper, collects from `system_profiler SPDisplaysDataType`, `diskutil info -plist /`, firmware from ioreg + system_profiler fallback
- Windows collector: added `win_class_name()` mapping, PowerShell-first approach with wmic fallback, filters out "To Be Filled By O.E.M." junk values, TPM via `Get-TpmEndorsementKeyInfo`
- Linux collector: added `/proc/meminfo` parsing, `lspci -mm` for GPU, `findmnt` for boot UUID, expanded `/proc/cpuinfo` parsing (vendor_id, siblings, microcode)
- New `collect_debug()` method returns `DebugFingerprint` with redacted signal values
- New types: `DebugFingerprint`, `DebugSignals` (both `Serialize`)
- New helper: `redact_middle()` for safe display of sensitive IDs
- Calls `proprietary_tier_hash()` instead of the old `tiered_hash()`
- Tests cover: collection succeeds, determinism, compare exact/drifted/different, debug collection

### 4. `src/license/keyfile.rs` — Schema version in signature payload
- `signature_payload()` now includes `self.device.schema_version` in the canonical string:
  `"{}::{}::{}::{}::{}"` → key, device_id, schema_version, activated_at, recheck_after
- Added `tracing::info!()` log on tier 2 drift acceptance

### 5. `src/license/mod.rs` — New public exports and fingerprint_debug function
- Added `pub use fingerprint::DebugFingerprint;`
- Added `pub fn fingerprint_debug() -> Result<DebugFingerprint, String>`
- Fixed unused variable: `NeedsRecheck { key, .. }` → `NeedsRecheck { key: _, .. }`

### 6. `src/main.rs` — New `convx fingerprint` CLI command
- Added `Commands::Fingerprint` variant to the `Commands` enum
- Added match arm that calls `license::fingerprint_debug()`
- Human-readable output shows: schema version, truncated device ID, device name, platform, truncated tier hashes, all collected signals (redacted)
- JSON output via `--json` flag for machine-readable diagnostics
- This command does NOT require a license (not in the `needs_license` match)

## Build & Fix Instructions

1. **Run `cargo build` first.** Fix any compilation errors. The most likely issues:
   - Import mismatches if `proprietary_tier_hash` visibility or signature doesn't match
   - The `hmac` crate version may need `Mac` trait import adjustments (hmac 0.12 uses `hmac::Mac`)
   - `DeviceFingerprint` now has `schema_version: u8` — any place constructing a `DeviceFingerprint` directly needs this field. Check `api.rs` response deserialization — Serde should handle it if the server sends it, but if there's manual construction anywhere, add the field.
   - The `ActivateRequest` in `api.rs` serializes a `DeviceFingerprint` which now includes `schema_version` — this is fine for the API (just sends an extra field).

2. **Run `cargo build` with zero warnings.** Fix any unused imports, unused variables, dead code warnings. Common ones:
   - The `NeedsRecheck { key: _ }` fix is already in place
   - Check for any unused `use` statements from the old crypto module (e.g. if something was importing the removed `tiered_hash` or `FINGERPRINT_SALT`)

3. **Run ALL tests:**
   ```bash
   cargo test -p convx-core
   ```
   
   **Expected test modules and what they verify:**
   
   **`license::crypto::tests`** (5 tests):
   - `sha256_hex_deterministic` — SHA256 helper produces consistent 64-char hex output
   - `tier_hash_deterministic` — same inputs → same HMAC output
   - `tier_hash_order_sensitive` — swapping component order changes the hash
   - `tier_hash_tier_sensitive` — same components with different tier number → different hash
   - `tier_hash_component_count_matters` — `["a", ""]` ≠ `["a"]`
   - `tier_hash_output_is_hex_sha256` — output is 64-char lowercase hex

   **`license::fingerprint::tests`** (6 tests):
   - `collect_succeeds_on_current_platform` — collects non-empty device_id, tier hashes, device_name; schema_version == 2
   - `fingerprint_is_deterministic` — two consecutive collections produce identical hashes
   - `compare_exact_match` — fingerprint compared to itself returns `Match::Exact`
   - `compare_different` — mutated tier1 + tier2 returns `Match::Different`
   - `compare_drifted` — mutated tier2 only returns `Match::Drifted`
   - `debug_fingerprint_works` — `collect_debug()` succeeds

   **`license::tests`** (3 tests):
   - `mask_key_typical` — standard key masking
   - `mask_key_short` — edge case
   - `mask_key_no_dashes` — edge case

   **Plus any existing tests in other modules** (converters, presets, engine, etc.) — these should still pass. Don't break existing functionality.

4. **Run the new CLI command:**
   ```bash
   cargo run --features cli -- fingerprint
   cargo run --features cli -- fingerprint --json
   ```
   
   **Verify the output:**
   - Human-readable mode shows schema version, truncated hashes, all signal labels
   - JSON mode outputs valid JSON with `fingerprint` and `signals` objects
   - On macOS: `platform_uuid` should not be empty, `cpu_id` should contain the CPU brand string, `model` should be like "Mac14,2" or similar, `total_ram` should be a number, `gpu_id` should contain a chipset name
   - Sensitive values in human-readable mode should be redacted (middle portion replaced with `…`)

5. **Run the existing license CLI commands to verify no regression:**
   ```bash
   cargo run --features cli -- license
   # Should print "ConvX is not activated." (unless a license.json exists)
   
   cargo run --features cli -- --help
   # Should show the new "fingerprint" subcommand in the help output
   ```

6. **Verify the `convx-mcp` binary also compiles:**
   ```bash
   cargo build --bin convx-mcp --no-default-features
   ```
   The MCP binary doesn't use CLI features but still links the license module.

## Fix Guidelines

- **DO NOT change the HMAC_KEY, TIER_DOMAINS, or mixing constants** in `crypto.rs` — these are intentional proprietary values.
- **DO NOT change the signal collection logic** (which commands are run, which sysctl keys are read, etc.) — these were deliberately chosen per-platform.
- **DO NOT remove the `schema_version` field** from `DeviceFingerprint` — it's needed for future fingerprint scheme rotation.
- **DO NOT downgrade `hmac`** below 0.12 — the `Mac` trait API changed between versions.
- If a platform-specific collector function has a compilation issue (e.g. missing `#[cfg]` attribute), fix it but keep the collection logic intact.
- If there are Clippy warnings, fix them, but don't restructure the module architecture.
- All fixes should be minimal and surgical — this is a "make it compile and pass tests" task, not a refactor.

## Completion Criteria

All of these must be true:

- [ ] `cargo build -p convx-core` succeeds with zero errors
- [ ] `cargo build -p convx-core` produces zero warnings (or only expected ones from dependencies)
- [ ] `cargo test -p convx-core` — ALL tests pass, including:
  - [ ] `license::crypto::tests::sha256_hex_deterministic`
  - [ ] `license::crypto::tests::tier_hash_deterministic`
  - [ ] `license::crypto::tests::tier_hash_order_sensitive`
  - [ ] `license::crypto::tests::tier_hash_tier_sensitive`
  - [ ] `license::crypto::tests::tier_hash_component_count_matters`
  - [ ] `license::crypto::tests::tier_hash_output_is_hex_sha256`
  - [ ] `license::fingerprint::tests::collect_succeeds_on_current_platform`
  - [ ] `license::fingerprint::tests::fingerprint_is_deterministic`
  - [ ] `license::fingerprint::tests::compare_exact_match`
  - [ ] `license::fingerprint::tests::compare_different`
  - [ ] `license::fingerprint::tests::compare_drifted`
  - [ ] `license::fingerprint::tests::debug_fingerprint_works`
  - [ ] `license::tests::mask_key_typical`
  - [ ] `license::tests::mask_key_short`
  - [ ] `license::tests::mask_key_no_dashes`
  - [ ] All pre-existing tests in other modules still pass
- [ ] `cargo run --features cli -- fingerprint` produces readable output with non-empty signal values
- [ ] `cargo run --features cli -- fingerprint --json` produces valid parseable JSON
- [ ] `cargo run --features cli -- license` still works (prints not activated or shows existing license)
- [ ] `cargo run --features cli -- --help` shows the `fingerprint` subcommand
- [ ] `cargo build --bin convx-mcp --no-default-features` compiles successfully

## After Completion

Print a summary showing:
1. Any fixes you had to make (file, line, what changed, why)
2. Full test output (test count, all passing)
3. Output of `convx fingerprint` on this machine
4. Output of `convx fingerprint --json` on this machine
