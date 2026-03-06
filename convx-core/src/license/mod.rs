//! License management for ConvX.
//!
//! Public API consumed by CLI, Desktop, and MCP surfaces:
//!
//! ```rust,ignore
//! use convx::license::{check_license, activate, deactivate, license_info, LicenseStatus};
//!
//! match check_license() {
//!     LicenseStatus::Valid { .. } => { /* proceed */ }
//!     LicenseStatus::NotActivated => { /* prompt for key */ }
//!     other => { /* show appropriate message */ }
//! }
//! ```

pub mod api;
pub mod crypto;
pub mod enterprise;
pub mod fingerprint;
pub mod keyfile;

pub use enterprise::{EnterpriseConfig, EnterpriseSettings};
pub use fingerprint::DebugFingerprint;

use api::ApiError;
use fingerprint::DeviceFingerprint;
use keyfile::{LicenseFile, ValidationResult};

/// Overall license status — the one enum every surface checks on startup.
#[derive(Debug)]
pub enum LicenseStatus {
    /// License is valid. Carry on.
    Valid {
        device_name: String,
        recheck_after: chrono::DateTime<chrono::Utc>,
    },

    /// License needs a recheck but is still within the grace period.
    /// The tool should work but attempt a background recheck.
    GracePeriod { days_overdue: i64 },

    /// Grace period exhausted. Recheck required.
    Expired,

    /// Key was revoked (refund, etc). License file has been removed.
    Revoked,

    /// Signature didn't verify — file was tampered.
    Tampered,

    /// License is bound to a different device.
    DeviceMismatch { stored_device: String },

    /// No license file on disk.
    NotActivated,

    /// Something unexpected went wrong reading the license.
    Error(String),
}

/// Check the license and optionally revalidate with the server.
///
/// This is the main entry point called on every launch:
/// 1. Read `~/.convx/license.json`
/// 2. Verify signature and device fingerprint locally
/// 3. If past `recheck_after` and online, revalidate with server
/// 4. Return the status
pub fn check_license() -> LicenseStatus {
    let license = match LicenseFile::load() {
        Ok(Some(license)) => license,
        Ok(None) => return LicenseStatus::NotActivated,
        Err(e) => return LicenseStatus::Error(e.to_string()),
    };

    match license.validate() {
        ValidationResult::Valid {
            device_name,
            recheck_after,
            ..
        } => LicenseStatus::Valid {
            device_name,
            recheck_after,
        },

        ValidationResult::NeedsRecheck {
            key: _,
            days_overdue,
        } => {
            // Attempt a background recheck
            match attempt_recheck(&license) {
                RecheckOutcome::Renewed { recheck_after } => LicenseStatus::Valid {
                    device_name: license.device.device_name.clone(),
                    recheck_after,
                },
                RecheckOutcome::Revoked => {
                    let _ = LicenseFile::remove();
                    LicenseStatus::Revoked
                }
                RecheckOutcome::Offline => {
                    // Couldn't reach server — still in grace period
                    LicenseStatus::GracePeriod { days_overdue }
                }
                RecheckOutcome::Failed(msg) => {
                    // Non-transient error during recheck; allow grace
                    tracing::warn!(error = %msg, "License recheck failed");
                    LicenseStatus::GracePeriod { days_overdue }
                }
            }
        }

        ValidationResult::Expired => {
            // One last try to recheck
            match attempt_recheck(&license) {
                RecheckOutcome::Renewed { recheck_after } => LicenseStatus::Valid {
                    device_name: license.device.device_name.clone(),
                    recheck_after,
                },
                RecheckOutcome::Revoked => {
                    let _ = LicenseFile::remove();
                    LicenseStatus::Revoked
                }
                _ => LicenseStatus::Expired,
            }
        }

        ValidationResult::Tampered => LicenseStatus::Tampered,

        ValidationResult::DeviceMismatch { stored_name } => LicenseStatus::DeviceMismatch {
            stored_device: stored_name,
        },

        ValidationResult::NotFound => LicenseStatus::NotActivated,

        ValidationResult::Corrupt(msg) => LicenseStatus::Error(msg),
    }
}

/// Activate a license key on this device.
pub fn activate(key: &str) -> Result<ActivateOutcome, String> {
    let device = DeviceFingerprint::collect().map_err(|e| e.to_string())?;

    match api::activate(key, &device) {
        Ok(resp) => {
            let license = LicenseFile {
                key: resp.key,
                device: device.clone(),
                activated_at: resp.activated_at,
                recheck_after: resp.recheck_after,
                signature: resp.signature,
            };
            license.save().map_err(|e| e.to_string())?;

            Ok(ActivateOutcome::Activated {
                device_name: device.device_name,
            })
        }
        Err(ApiError::AlreadyActivated { device_name }) => {
            Ok(ActivateOutcome::AlreadyActive { device_name })
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Transfer the license from its current device to this one.
pub fn transfer(key: &str) -> Result<(), String> {
    let device = DeviceFingerprint::collect().map_err(|e| e.to_string())?;

    let resp = api::transfer(key, &device).map_err(|e| e.to_string())?;

    let license = LicenseFile {
        key: key.to_string(),
        device,
        activated_at: resp.activated_at,
        recheck_after: resp.recheck_after,
        signature: resp.signature,
    };

    license.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Deactivate this device, freeing the slot.
pub fn deactivate() -> Result<(), String> {
    let license = LicenseFile::load()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No active license found".to_string())?;

    api::deactivate(&license.key, &license.device.device_id).map_err(|e| e.to_string())?;

    LicenseFile::remove().map_err(|e| e.to_string())?;
    Ok(())
}

/// Try auto-activation from enterprise config (env var or config file).
/// Returns None if no enterprise key is configured.
pub fn auto_activate() -> Option<Result<ActivateOutcome, String>> {
    enterprise::auto_activate()
}

/// Load enterprise config if present.
pub fn get_enterprise_config() -> Option<EnterpriseConfig> {
    // Check env var first
    if std::env::var("CONVX_LICENSE_KEY").is_ok() {
        let config = EnterpriseConfig {
            license_key: std::env::var("CONVX_LICENSE_KEY").ok(),
            org_id: std::env::var("CONVX_ORG_ID").ok(),
            settings: None,
            audit_endpoint: None,
        };
        return Some(config);
    }

    EnterpriseConfig::load()
}

/// Collect and display device fingerprint diagnostic info.
/// Used by `convx fingerprint` debug command.
pub fn fingerprint_debug() -> Result<DebugFingerprint, String> {
    DeviceFingerprint::collect_debug().map_err(|e| e.to_string())
}

/// Get current license info without revalidating.
pub fn license_info() -> Option<LicenseInfo> {
    let license = LicenseFile::load().ok()??;

    Some(LicenseInfo {
        key_masked: mask_key(&license.key),
        device_name: license.device.device_name.clone(),
        platform: license.device.platform.clone(),
        activated_at: license.activated_at,
        recheck_after: license.recheck_after,
    })
}

/// Require a valid license or exit.
/// Call this at the top of CLI/MCP entry points.
pub fn require_license() -> Result<(), String> {
    match check_license() {
        LicenseStatus::Valid { .. } | LicenseStatus::GracePeriod { .. } => Ok(()),
        LicenseStatus::NotActivated => Err(
            "ConvX is not activated. Run `convx activate <KEY>` to get started.\n\
             Purchase a license at https://convx.dev"
                .to_string(),
        ),
        LicenseStatus::Expired => Err(
            "Your license needs to be re-verified but the server couldn't be reached.\n\
             Please check your internet connection and try again."
                .to_string(),
        ),
        LicenseStatus::Revoked => Err("This license has been revoked.\n\
             If you believe this is an error, contact support@convx.dev"
            .to_string()),
        LicenseStatus::Tampered => Err("License file integrity check failed.\n\
             Run `convx activate <KEY>` to re-activate, or contact support@convx.dev"
            .to_string()),
        LicenseStatus::DeviceMismatch { stored_device } => Err(format!(
            "This license is activated on a different device: \"{}\"\n\
             Run `convx activate <KEY>` to transfer it to this device.",
            stored_device
        )),
        LicenseStatus::Error(msg) => Err(format!("License error: {}", msg)),
    }
}

// ─── Supporting types ──────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ActivateOutcome {
    /// Successfully activated on this device.
    Activated { device_name: String },
    /// Key is already active on another device — user can choose to transfer.
    AlreadyActive { device_name: String },
}

pub struct LicenseInfo {
    pub key_masked: String,
    pub device_name: String,
    pub platform: String,
    pub activated_at: chrono::DateTime<chrono::Utc>,
    pub recheck_after: chrono::DateTime<chrono::Utc>,
}

// ─── Internal helpers ──────────────────────────────────────────────────────

enum RecheckOutcome {
    Renewed {
        recheck_after: chrono::DateTime<chrono::Utc>,
    },
    Revoked,
    Offline,
    Failed(String),
}

fn attempt_recheck(license: &LicenseFile) -> RecheckOutcome {
    match api::validate(license) {
        Ok(resp) => {
            if resp.revoked {
                return RecheckOutcome::Revoked;
            }
            if !resp.valid {
                return RecheckOutcome::Revoked;
            }

            // Update the license file with the fresh recheck window
            let mut updated = license.clone();
            updated.recheck_after = resp.recheck_after;
            updated.signature = resp.signature;

            if updated.save().is_err() {
                tracing::warn!("Failed to write updated license file after recheck");
            }

            RecheckOutcome::Renewed {
                recheck_after: resp.recheck_after,
            }
        }
        Err(e) if e.is_transient() => RecheckOutcome::Offline,
        Err(e) => RecheckOutcome::Failed(e.to_string()),
    }
}

fn mask_key(key: &str) -> String {
    // "CONVX-AAAA-BBBB-CCCC-DDDD" → "CONVX-****-****-****-DDDD"
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() < 2 {
        return "****".to_string();
    }

    let last = parts.last().unwrap_or(&"****");
    let masked_middle: Vec<String> = parts[1..parts.len() - 1]
        .iter()
        .map(|_| "****".to_string())
        .collect();

    let mut result = vec![parts[0].to_string()];
    result.extend(masked_middle);
    result.push(last.to_string());
    result.join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_key_typical() {
        assert_eq!(
            mask_key("CONVX-AAAA-BBBB-CCCC-DDDD"),
            "CONVX-****-****-****-DDDD"
        );
    }

    #[test]
    fn mask_key_short() {
        assert_eq!(mask_key("CONVX-DDDD"), "CONVX-DDDD");
    }

    #[test]
    fn mask_key_no_dashes() {
        assert_eq!(mask_key("ABCDEF"), "****");
    }
}
