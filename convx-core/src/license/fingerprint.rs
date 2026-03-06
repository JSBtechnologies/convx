//! Hardware-based device fingerprinting — proprietary composite method.
//!
//! Collects immutable and semi-stable hardware identifiers per platform,
//! combines them through a proprietary tiered HMAC scheme. The resulting
//! fingerprint survives OS updates and minor hardware changes but changes
//! when the device is genuinely different.
//!
//! ## Tier model
//!
//! - **Tier 1 (immutable silicon):** Platform/SMBIOS UUID, CPU hardware ID,
//!   CPU microarchitecture details, TPM endorsement (Windows). These are
//!   burned into hardware at manufacturing. Must match for same-device.
//!
//! - **Tier 2 (stable system):** Motherboard serial, OS machine ID, model
//!   identifier, total physical RAM, primary GPU device ID, chassis serial.
//!   Tolerates one component drifting (e.g. RAM upgrade, OS reinstall).
//!
//! - **Tier 3 (soft signals):** Hostname, OS version, boot disk UUID.
//!   Informational only, never used in the binding hash.
//!
//! ## Security model
//!
//! - MAC addresses are intentionally excluded — trivially spoofable
//! - CPU/board/platform UUIDs require physical hardware changes to alter
//! - The hash computation uses HMAC-SHA256 with a compiled-in key and
//!   version-tagged mixing that would require binary reverse engineering
//!   to reproduce
//! - Individual signal values are never stored or transmitted — only the
//!   composite tier hashes leave the device

use crate::license::crypto::proprietary_tier_hash;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    /// Composite device ID: HMAC(tier1 || tier2)
    pub device_id: String,
    /// Human-readable device name (hostname + model)
    pub device_name: String,
    /// Tier-1 hash (immutable silicon)
    pub tier1_hash: String,
    /// Tier-2 hash (stable system)
    pub tier2_hash: String,
    /// Platform: "macos", "windows", "linux"
    pub platform: String,
    /// Fingerprint schema version — allows us to rotate the hash method
    pub schema_version: u8,
    /// When this fingerprint was collected
    pub collected_at: chrono::DateTime<Utc>,
}

/// Raw signals collected from the hardware.
/// Each field is a string regardless of the underlying data type —
/// we hash them uniformly.
#[derive(Debug, Default)]
pub(crate) struct RawSignals {
    // ── Tier 1: Immutable silicon ──────────────────────────────────
    /// Platform UUID (IOPlatformUUID on macOS, SMBIOS UUID on Windows/Linux)
    pub platform_uuid: String,
    /// CPU hardware identifier (ProcessorId on Windows, brand+stepping+features elsewhere)
    pub cpu_id: String,
    /// CPU microarchitecture details: family, model, stepping, microcode revision
    pub cpu_microarch: String,
    /// TPM endorsement key public hash (Windows only, very stable)
    pub tpm_hash: String,

    // ── Tier 2: Stable system ──────────────────────────────────────
    /// Motherboard / logic board serial number
    pub board_serial: String,
    /// OS-level machine identifier (machine-id on Linux, MachineGuid on Windows)
    pub machine_id: String,
    /// Hardware model identifier (e.g. "Mac14,2", "HP EliteBook 840 G9")
    pub model: String,
    /// Total physical RAM in bytes (string representation)
    pub total_ram: String,
    /// Primary GPU device identifier
    pub gpu_id: String,
    /// Chassis / enclosure serial number
    pub chassis_serial: String,
    /// System firmware / BIOS version
    pub firmware_version: String,

    // ── Tier 3: Soft / display-only ────────────────────────────────
    pub hostname: String,
    pub os_version: String,
    pub boot_disk_uuid: String,
}

/// Current fingerprint schema version. Bump this when changing the
/// hash computation so old fingerprints can be migrated.
const SCHEMA_VERSION: u8 = 2;

impl DeviceFingerprint {
    /// Collect hardware signals and build the fingerprint for this device.
    pub fn collect() -> Result<Self, FingerprintError> {
        let raw = collect_signals()?;

        // Require at least one Tier-1 signal to proceed
        if raw.platform_uuid.is_empty() && raw.cpu_id.is_empty() {
            return Err(FingerprintError::InsufficientSignals);
        }

        let tier1 = proprietary_tier_hash(
            1,
            &[
                &raw.platform_uuid,
                &raw.cpu_id,
                &raw.cpu_microarch,
                &raw.tpm_hash,
            ],
        );

        let tier2 = proprietary_tier_hash(
            2,
            &[
                &raw.board_serial,
                &raw.machine_id,
                &raw.model,
                &raw.total_ram,
                &raw.gpu_id,
                &raw.chassis_serial,
                &raw.firmware_version,
            ],
        );

        let composite = proprietary_tier_hash(0, &[&tier1, &tier2]);

        let device_name = build_device_name(&raw);
        let platform = current_platform().to_string();

        Ok(Self {
            device_id: composite,
            device_name,
            tier1_hash: tier1,
            tier2_hash: tier2,
            platform,
            schema_version: SCHEMA_VERSION,
            collected_at: Utc::now(),
        })
    }

    /// Collect raw signals and return them for diagnostic purposes.
    /// Used by `convx fingerprint` debug command.
    pub fn collect_debug() -> Result<DebugFingerprint, FingerprintError> {
        let raw = collect_signals()?;
        let fp = Self::collect()?;

        Ok(DebugFingerprint {
            fingerprint: fp,
            signals: DebugSignals {
                platform_uuid: redact_middle(&raw.platform_uuid),
                cpu_id: raw.cpu_id.clone(),
                cpu_microarch: raw.cpu_microarch.clone(),
                tpm_hash: if raw.tpm_hash.is_empty() {
                    "(not available)".to_string()
                } else {
                    redact_middle(&raw.tpm_hash)
                },
                board_serial: redact_middle(&raw.board_serial),
                machine_id: redact_middle(&raw.machine_id),
                model: raw.model.clone(),
                total_ram: raw.total_ram.clone(),
                gpu_id: raw.gpu_id.clone(),
                chassis_serial: redact_middle(&raw.chassis_serial),
                firmware_version: raw.firmware_version.clone(),
                hostname: raw.hostname.clone(),
                os_version: raw.os_version.clone(),
                boot_disk_uuid: redact_middle(&raw.boot_disk_uuid),
            },
        })
    }

    /// Check whether a stored fingerprint matches the current device.
    pub fn compare(&self, other: &DeviceFingerprint) -> Match {
        if self.tier1_hash == other.tier1_hash && self.tier2_hash == other.tier2_hash {
            Match::Exact
        } else if self.tier1_hash == other.tier1_hash {
            Match::Drifted
        } else {
            Match::Different
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// Both tiers match — same device, nothing changed.
    Exact,
    /// Tier 1 matches but tier 2 drifted (OS reinstall, minor HW change).
    /// Should be accepted but logged server-side.
    Drifted,
    /// Different device entirely.
    Different,
}

/// Diagnostic output for `convx fingerprint` command.
#[derive(Debug, Serialize)]
pub struct DebugFingerprint {
    pub fingerprint: DeviceFingerprint,
    pub signals: DebugSignals,
}

#[derive(Debug, Serialize)]
pub struct DebugSignals {
    pub platform_uuid: String,
    pub cpu_id: String,
    pub cpu_microarch: String,
    pub tpm_hash: String,
    pub board_serial: String,
    pub machine_id: String,
    pub model: String,
    pub total_ram: String,
    pub gpu_id: String,
    pub chassis_serial: String,
    pub firmware_version: String,
    pub hostname: String,
    pub os_version: String,
    pub boot_disk_uuid: String,
}

fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn build_device_name(raw: &RawSignals) -> String {
    let host = if raw.hostname.is_empty() {
        "Unknown".to_string()
    } else {
        raw.hostname.clone()
    };

    if raw.model.is_empty() {
        host
    } else {
        format!("{} ({})", host, raw.model)
    }
}

/// Redact middle portion of sensitive IDs for debug output.
/// "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX" → "XXXXXXXX-****-****-****-XXXXXXXXXXXX"
fn redact_middle(s: &str) -> String {
    if s.is_empty() {
        return "(not available)".to_string();
    }
    let len = s.len();
    if len <= 8 {
        return format!("{}****", &s[..2.min(len)]);
    }
    let show = len / 4;
    format!("{}…{}", &s[..show], &s[len - show..])
}

// ═══════════════════════════════════════════════════════════════════════════
// Platform-specific collectors
// ═══════════════════════════════════════════════════════════════════════════

fn collect_signals() -> Result<RawSignals, FingerprintError> {
    #[cfg(target_os = "macos")]
    {
        collect_macos()
    }
    #[cfg(target_os = "windows")]
    {
        collect_windows()
    }
    #[cfg(target_os = "linux")]
    {
        collect_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(FingerprintError::UnsupportedPlatform)
    }
}

// ─── macOS ─────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn collect_macos() -> Result<RawSignals, FingerprintError> {
    let mut raw = RawSignals::default();

    // ── Tier 1 ─────────────────────────────────────────────────────

    // IOPlatformUUID — hardware UUID burned into Mac firmware at manufacturing.
    // This alone uniquely identifies a Mac. It survives OS reinstalls,
    // disk wipes, and all software changes.
    let ioreg_output =
        run_cmd("ioreg", &["-rd1", "-c", "IOPlatformExpertDevice"]).unwrap_or_default();

    raw.platform_uuid = extract_ioreg_value(&ioreg_output, "IOPlatformUUID").unwrap_or_default();

    // CPU: brand string + physical core count + logical core count + features.
    // This is stable per CPU model — changes only when the CPU is replaced
    // (which on Apple Silicon means a new machine entirely).
    let cpu_brand = sysctl_value("machdep.cpu.brand_string").unwrap_or_default();
    let phys_cores = sysctl_value("hw.physicalcpu").unwrap_or_default();
    let logical_cores = sysctl_value("hw.logicalcpu").unwrap_or_default();

    // On Intel Macs: stepping, family, extended model give microarch details
    // On Apple Silicon: just the brand string is sufficient (e.g. "Apple M2 Pro")
    let cpu_family = sysctl_value("machdep.cpu.family").unwrap_or_default();
    let cpu_model = sysctl_value("machdep.cpu.model").unwrap_or_default();
    let cpu_stepping = sysctl_value("machdep.cpu.stepping").unwrap_or_default();

    raw.cpu_id = format!(
        "{}::p{}::l{}",
        cpu_brand.trim(),
        phys_cores.trim(),
        logical_cores.trim()
    );

    raw.cpu_microarch = format!(
        "family={}::model={}::stepping={}",
        cpu_family.trim(),
        cpu_model.trim(),
        cpu_stepping.trim()
    );

    // ── Tier 2 ─────────────────────────────────────────────────────

    // IOPlatformSerialNumber — Apple's logic board serial.
    // On newer Macs this may be truncated, but is still a stable identifier.
    raw.board_serial =
        extract_ioreg_value(&ioreg_output, "IOPlatformSerialNumber").unwrap_or_default();

    // Hardware model identifier (e.g. "Mac14,2" for M2 MacBook Air)
    raw.model = sysctl_value("hw.model")
        .unwrap_or_default()
        .trim()
        .to_string();

    // Machine ID — on macOS the platform UUID serves double duty
    raw.machine_id = raw.platform_uuid.clone();

    // Total physical RAM — stable unless user physically upgrades (soldered on Apple Silicon)
    raw.total_ram = sysctl_value("hw.memsize")
        .unwrap_or_default()
        .trim()
        .to_string();

    // Primary GPU identifier from system_profiler
    raw.gpu_id = run_cmd(
        "system_profiler",
        &["SPDisplaysDataType", "-detailLevel", "mini"],
    )
    .and_then(|out| {
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Chipset Model:") || trimmed.starts_with("Chip:") {
                return trimmed.split_once(':').map(|(_, v)| v.trim().to_string());
            }
        }
        None
    })
    .unwrap_or_default();

    // Chassis serial — on Macs this is typically the same as the board serial
    // but we collect it separately for consistency across platforms
    raw.chassis_serial = raw.board_serial.clone();

    // Boot ROM / firmware version
    raw.firmware_version = run_cmd("ioreg", &["-rd1", "-c", "IOPlatformExpertDevice"])
        .and_then(|out| extract_ioreg_value(&out, "boot-rom-version"))
        // Sometimes the value is hex-encoded in ioreg; fallback to system_profiler
        .or_else(|| {
            run_cmd(
                "system_profiler",
                &["SPHardwareDataType", "-detailLevel", "mini"],
            )
            .and_then(|out| {
                for line in out.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("System Firmware Version:")
                        || trimmed.starts_with("Boot ROM Version:")
                    {
                        return trimmed.split_once(':').map(|(_, v)| v.trim().to_string());
                    }
                }
                None
            })
        })
        .unwrap_or_default();

    // ── Tier 3 ─────────────────────────────────────────────────────

    raw.hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_default();

    raw.os_version = sysctl_value("kern.osproductversion")
        .or_else(|| run_cmd("sw_vers", &["-productVersion"]).map(|s| s.trim().to_string()))
        .unwrap_or_default();

    // Boot volume UUID
    raw.boot_disk_uuid = run_cmd("diskutil", &["info", "-plist", "/"])
        .and_then(|out| {
            // Simple extraction from plist — look for VolumeUUID
            let mut found_key = false;
            for line in out.lines() {
                let trimmed = line.trim();
                if trimmed.contains("VolumeUUID") {
                    found_key = true;
                    continue;
                }
                if found_key && trimmed.starts_with("<string>") {
                    return Some(
                        trimmed
                            .trim_start_matches("<string>")
                            .trim_end_matches("</string>")
                            .to_string(),
                    );
                }
            }
            None
        })
        .unwrap_or_default();

    Ok(raw)
}

#[cfg(target_os = "macos")]
fn sysctl_value(key: &str) -> Option<String> {
    run_cmd("sysctl", &["-n", key]).map(|s| s.trim().to_string())
}

#[cfg(target_os = "macos")]
fn extract_ioreg_value(output: &str, key: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains(key) {
            if let Some((_before, after)) = trimmed.split_once('=') {
                let value = after.trim().trim_matches('"').trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

// ─── Windows ───────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn collect_windows() -> Result<RawSignals, FingerprintError> {
    let mut raw = RawSignals::default();

    // ── Tier 1 ─────────────────────────────────────────────────────

    // SMBIOS UUID — motherboard firmware UUID, set at manufacturing.
    // Unique per physical machine. Survives OS reinstalls and disk changes.
    raw.platform_uuid = wmic_value("csproduct", "UUID").unwrap_or_default();

    // CPU ProcessorId — a hardware-level identifier from the CPU itself.
    // Combined with brand and core count for a comprehensive CPU fingerprint.
    let proc_id = wmic_value("cpu", "ProcessorId").unwrap_or_default();
    let proc_name = wmic_value("cpu", "Name").unwrap_or_default();
    let proc_cores = wmic_value("cpu", "NumberOfLogicalProcessors").unwrap_or_default();
    let proc_phys = wmic_value("cpu", "NumberOfCores").unwrap_or_default();
    raw.cpu_id = format!(
        "{}::{}::p{}::l{}",
        proc_id, proc_name, proc_phys, proc_cores
    );

    // CPU microarchitecture details
    let cpu_family = wmic_value("cpu", "Family").unwrap_or_default();
    let cpu_stepping = wmic_value("cpu", "Stepping").unwrap_or_default();
    let cpu_revision = wmic_value("cpu", "Revision").unwrap_or_default();
    raw.cpu_microarch = format!(
        "family={}::stepping={}::revision={}",
        cpu_family, cpu_stepping, cpu_revision
    );

    // TPM endorsement key public hash — extremely stable, tied to TPM chip.
    // Only available on machines with TPM 2.0. Not all machines have this.
    raw.tpm_hash = run_cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "try { (Get-TpmEndorsementKeyInfo -HashAlgorithm 'Sha256' -ErrorAction Stop).PublicKeyHash } catch { '' }",
        ],
    )
    .map(|s| s.trim().to_string())
    .unwrap_or_default();

    // ── Tier 2 ─────────────────────────────────────────────────────

    // Motherboard serial number
    raw.board_serial = wmic_value("baseboard", "SerialNumber").unwrap_or_default();

    // MachineGuid — Windows registry value, stable across reboots.
    // Changes on OS reinstall, but tier 1 hardware signals compensate.
    raw.machine_id = run_cmd(
        "reg",
        &[
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ],
    )
    .and_then(|out| {
        out.lines()
            .find(|l| l.contains("MachineGuid"))
            .and_then(|l| l.split_whitespace().last())
            .map(|s| s.trim().to_string())
    })
    .unwrap_or_default();

    // Model identifier
    raw.model = wmic_value("computersystem", "Model").unwrap_or_default();

    // Total physical memory in bytes
    raw.total_ram = run_cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
        ],
    )
    .map(|s| s.trim().to_string())
    .unwrap_or_default();

    // Primary GPU device ID
    raw.gpu_id = run_cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController | Select-Object -First 1).PNPDeviceID",
        ],
    )
    .map(|s| s.trim().to_string())
    .unwrap_or_default();

    // Chassis serial
    raw.chassis_serial = wmic_value("systemenclosure", "SerialNumber").unwrap_or_default();

    // BIOS version
    raw.firmware_version = wmic_value("bios", "SMBIOSBIOSVersion").unwrap_or_default();

    // ── Tier 3 ─────────────────────────────────────────────────────

    raw.hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_default();

    raw.os_version = run_cmd("cmd", &["/c", "ver"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // Boot volume serial number
    raw.boot_disk_uuid = run_cmd("cmd", &["/c", "vol", "C:"])
        .and_then(|out| {
            out.lines()
                .find(|l| l.contains("Serial Number"))
                .and_then(|l| l.split_whitespace().last())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    Ok(raw)
}

#[cfg(target_os = "windows")]
fn wmic_value(alias: &str, property: &str) -> Option<String> {
    // Try PowerShell first (wmic deprecated on newer Windows builds)
    let class_name = win_class_name(alias);
    let ps_cmd = format!(
        "(Get-CimInstance -ClassName {} | Select-Object -First 1).{}",
        class_name, property
    );

    if let Some(val) = run_cmd("powershell", &["-NoProfile", "-Command", &ps_cmd]) {
        let trimmed = val.trim().to_string();
        if !trimmed.is_empty()
            && trimmed != "Default string"
            && trimmed != "None"
            && trimmed != "To Be Filled By O.E.M."
        {
            return Some(trimmed);
        }
    }

    // Fallback to wmic for older Windows
    let output = run_cmd("wmic", &[alias, "get", property, "/value"])?;
    for line in output.lines() {
        if let Some((_key, value)) = line.split_once('=') {
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty()
                && trimmed != "Default string"
                && trimmed != "To Be Filled By O.E.M."
            {
                return Some(trimmed);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn win_class_name(alias: &str) -> &str {
    match alias {
        "csproduct" => "Win32_ComputerSystemProduct",
        "cpu" => "Win32_Processor",
        "baseboard" => "Win32_BaseBoard",
        "computersystem" => "Win32_ComputerSystem",
        "systemenclosure" => "Win32_SystemEnclosure",
        "bios" => "Win32_BIOS",
        _ => alias,
    }
}

// ─── Linux ─────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
#[allow(clippy::field_reassign_with_default)]
fn collect_linux() -> Result<RawSignals, FingerprintError> {
    let mut raw = RawSignals::default();

    // ── Tier 1 ─────────────────────────────────────────────────────

    // SMBIOS product UUID — the most reliable Linux hardware identifier.
    // Requires root on some systems; fallback chain ensures we get something.
    raw.platform_uuid = read_file_trimmed("/sys/class/dmi/id/product_uuid")
        .or_else(|| {
            // On non-root: try dmidecode
            run_cmd("sudo", &["dmidecode", "-s", "system-uuid"]).map(|s| s.trim().to_string())
        })
        .or_else(|| read_file_trimmed("/etc/machine-id"))
        .unwrap_or_default();

    // CPU identification from /proc/cpuinfo — includes model, stepping,
    // microcode revision, and core count. Stable per physical CPU.
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let cpu_model_name = cpuinfo_value(&cpuinfo, "model name").unwrap_or_default();
    let cpu_cores = cpuinfo_value(&cpuinfo, "cpu cores").unwrap_or_default();
    let cpu_siblings = cpuinfo_value(&cpuinfo, "siblings").unwrap_or_default();
    let cpu_vendor = cpuinfo_value(&cpuinfo, "vendor_id").unwrap_or_default();

    raw.cpu_id = format!(
        "{}::{}::p{}::l{}",
        cpu_vendor, cpu_model_name, cpu_cores, cpu_siblings
    );

    let cpu_family = cpuinfo_value(&cpuinfo, "cpu family").unwrap_or_default();
    let cpu_model_num = cpuinfo_value(&cpuinfo, "model").unwrap_or_default();
    let cpu_stepping = cpuinfo_value(&cpuinfo, "stepping").unwrap_or_default();
    let cpu_microcode = cpuinfo_value(&cpuinfo, "microcode").unwrap_or_default();

    raw.cpu_microarch = format!(
        "family={}::model={}::stepping={}::microcode={}",
        cpu_family, cpu_model_num, cpu_stepping, cpu_microcode
    );

    // No TPM hash method on Linux via simple commands — leave empty
    raw.tpm_hash = String::new();

    // ── Tier 2 ─────────────────────────────────────────────────────

    // Board serial
    raw.board_serial = read_file_trimmed("/sys/class/dmi/id/board_serial").unwrap_or_default();

    // systemd machine-id — stable across reboots, changes on OS reinstall
    raw.machine_id = read_file_trimmed("/etc/machine-id").unwrap_or_default();

    // Product name as model identifier
    raw.model = read_file_trimmed("/sys/class/dmi/id/product_name").unwrap_or_default();

    // Total physical RAM from /proc/meminfo
    raw.total_ram = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    // Primary GPU — look for the first PCI VGA or 3D controller
    raw.gpu_id = run_cmd("lspci", &["-mm"])
        .and_then(|out| {
            for line in out.lines() {
                if line.contains("VGA") || line.contains("3D controller") {
                    return Some(line.to_string());
                }
            }
            None
        })
        .or_else(|| {
            // Fallback: read from sysfs
            read_file_trimmed("/sys/class/drm/card0/device/device")
        })
        .unwrap_or_default();

    // Chassis serial
    raw.chassis_serial = read_file_trimmed("/sys/class/dmi/id/chassis_serial").unwrap_or_default();

    // BIOS / firmware version
    raw.firmware_version = read_file_trimmed("/sys/class/dmi/id/bios_version").unwrap_or_default();

    // ── Tier 3 ─────────────────────────────────────────────────────

    raw.hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_default();

    raw.os_version = read_file_trimmed("/etc/os-release")
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| {
                    l.trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string()
                })
        })
        .unwrap_or_default();

    // Root filesystem UUID
    raw.boot_disk_uuid = run_cmd("findmnt", &["-no", "UUID", "/"])
        .map(|s| s.trim().to_string())
        .or_else(|| {
            // Fallback for systems without findmnt
            run_cmd("blkid", &["-o", "value", "-s", "UUID", "/dev/sda1"])
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_default();

    Ok(raw)
}

#[cfg(target_os = "linux")]
fn cpuinfo_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with(key) {
            if let Some((_, v)) = line.split_once(':') {
                let val = v.trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_file_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ─── Shared helpers ────────────────────────────────────────────────────────

fn run_cmd(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    #[error("Could not collect enough hardware signals to build a device fingerprint")]
    InsufficientSignals,

    #[error("Unsupported platform for device fingerprinting")]
    UnsupportedPlatform,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_succeeds_on_current_platform() {
        // This test will pass on macOS/Windows/Linux dev machines
        let fp = DeviceFingerprint::collect();
        assert!(fp.is_ok(), "Failed to collect fingerprint: {:?}", fp.err());

        let fp = fp.unwrap();
        assert!(!fp.device_id.is_empty());
        assert!(!fp.tier1_hash.is_empty());
        assert!(!fp.tier2_hash.is_empty());
        assert!(!fp.device_name.is_empty());
        assert_eq!(fp.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let a = DeviceFingerprint::collect().unwrap();
        let b = DeviceFingerprint::collect().unwrap();

        assert_eq!(a.device_id, b.device_id);
        assert_eq!(a.tier1_hash, b.tier1_hash);
        assert_eq!(a.tier2_hash, b.tier2_hash);
    }

    #[test]
    fn compare_exact_match() {
        let fp = DeviceFingerprint::collect().unwrap();
        assert_eq!(fp.compare(&fp), Match::Exact);
    }

    #[test]
    fn compare_different() {
        let mut a = DeviceFingerprint::collect().unwrap();
        let b = DeviceFingerprint::collect().unwrap();

        a.tier1_hash = "completely_different".to_string();
        a.tier2_hash = "also_different".to_string();
        assert_eq!(a.compare(&b), Match::Different);
    }

    #[test]
    fn compare_drifted() {
        let a = DeviceFingerprint::collect().unwrap();
        let mut b = a.clone();
        b.tier2_hash = "drifted_tier2".to_string();
        assert_eq!(a.compare(&b), Match::Drifted);
    }

    #[test]
    fn debug_fingerprint_works() {
        let debug = DeviceFingerprint::collect_debug();
        assert!(debug.is_ok());
    }
}
