//! Read, write, and verify `~/.convx/license.json`.

use crate::license::crypto::{sha256_hex, verify_signature};
use crate::license::fingerprint::{DeviceFingerprint, Match};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// On-disk license file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// The license key (e.g. "CONVX-XXXX-XXXX-XXXX-XXXX")
    pub key: String,

    /// Device fingerprint captured at activation
    pub device: DeviceFingerprint,

    /// When activation occurred
    pub activated_at: DateTime<Utc>,

    /// Server-issued: tool should recheck the license after this date.
    /// If offline past this date, a grace period applies.
    pub recheck_after: DateTime<Utc>,

    /// Ed25519 signature over the canonical payload (base64-encoded).
    /// Signs: SHA-256(key || device_id || schema_version || activated_at_iso || recheck_after_iso)
    pub signature: String,
}

/// Result of validating the on-disk license against the current device.
#[derive(Debug)]
pub enum ValidationResult {
    /// License is valid and the device matches.
    Valid {
        key: String,
        device_name: String,
        recheck_after: DateTime<Utc>,
    },

    /// License exists but needs a recheck (past recheck_after, within grace).
    NeedsRecheck { key: String, days_overdue: i64 },

    /// Grace period exhausted — recheck_after + GRACE_DAYS has passed.
    Expired,

    /// Signature is invalid — file was tampered with.
    Tampered,

    /// The device fingerprint doesn't match (different machine).
    DeviceMismatch { stored_name: String },

    /// No license file on disk.
    NotFound,

    /// File exists but couldn't be parsed.
    Corrupt(String),
}

/// Number of days past `recheck_after` before the license hard-stops.
const GRACE_DAYS: i64 = 7;

impl LicenseFile {
    /// Standard location: `~/.convx/license.json`
    pub fn path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".convx").join("license.json"))
    }

    /// Read the license file from disk. Returns `None` if it doesn't exist.
    pub fn load() -> Result<Option<Self>, KeyfileError> {
        let path = Self::path().ok_or(KeyfileError::NoHomeDir)?;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|e| KeyfileError::ReadFailed {
            path: path.clone(),
            reason: e.to_string(),
        })?;

        let license: LicenseFile =
            serde_json::from_str(&content).map_err(|e| KeyfileError::ParseFailed {
                path,
                reason: e.to_string(),
            })?;

        Ok(Some(license))
    }

    /// Write the license file to disk, creating `~/.convx/` if necessary.
    pub fn save(&self) -> Result<(), KeyfileError> {
        let path = Self::path().ok_or(KeyfileError::NoHomeDir)?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| KeyfileError::WriteFailed {
                path: parent.to_path_buf(),
                reason: e.to_string(),
            })?;
        }

        let content =
            serde_json::to_string_pretty(self).map_err(|e| KeyfileError::WriteFailed {
                path: path.clone(),
                reason: e.to_string(),
            })?;

        std::fs::write(&path, content).map_err(|e| KeyfileError::WriteFailed {
            path,
            reason: e.to_string(),
        })?;

        // Restrict permissions on Unix (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let path = Self::path().unwrap();
            let _ = std::fs::set_permissions(&path, perms);
        }

        Ok(())
    }

    /// Remove the license file from disk.
    pub fn remove() -> Result<(), KeyfileError> {
        let path = Self::path().ok_or(KeyfileError::NoHomeDir)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| KeyfileError::WriteFailed {
                path,
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }

    /// Canonical payload bytes that the signature covers.
    /// Both client and server must compute this identically.
    ///
    /// Includes the schema_version to prevent replay attacks where an
    /// attacker uses a signature from an older fingerprint scheme.
    pub fn signature_payload(&self) -> Vec<u8> {
        let canonical = format!(
            "{}::{}::{}::{}::{}",
            self.key,
            self.device.device_id,
            self.device.schema_version,
            self.activated_at.to_rfc3339(),
            self.recheck_after.to_rfc3339(),
        );
        sha256_hex(canonical.as_bytes()).into_bytes()
    }

    /// Full validation: signature check, device match, expiry check.
    pub fn validate(&self) -> ValidationResult {
        // 1. Verify the cryptographic signature
        let payload = self.signature_payload();
        if verify_signature(&payload, &self.signature).is_err() {
            return ValidationResult::Tampered;
        }

        // 2. Check device fingerprint
        let current = match DeviceFingerprint::collect() {
            Ok(fp) => fp,
            Err(_) => {
                return ValidationResult::Corrupt(
                    "Failed to collect device fingerprint".to_string(),
                )
            }
        };

        match current.compare(&self.device) {
            Match::Exact => {}
            Match::Drifted => {
                // Accept but tier 2 changed — OS reinstall, minor HW swap.
                // On next server recheck, report the drift so the stored
                // fingerprint can be updated.
                tracing::info!(
                    "Device fingerprint tier 2 drifted — accepting (tier 1 still matches)"
                );
            }
            Match::Different => {
                return ValidationResult::DeviceMismatch {
                    stored_name: self.device.device_name.clone(),
                };
            }
        }

        // 3. Check recheck window
        let now = Utc::now();
        if now <= self.recheck_after {
            return ValidationResult::Valid {
                key: self.key.clone(),
                device_name: self.device.device_name.clone(),
                recheck_after: self.recheck_after,
            };
        }

        let overdue = (now - self.recheck_after).num_days();
        if overdue <= GRACE_DAYS {
            return ValidationResult::NeedsRecheck {
                key: self.key.clone(),
                days_overdue: overdue,
            };
        }

        ValidationResult::Expired
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeyfileError {
    #[error("Cannot determine home directory")]
    NoHomeDir,

    #[error("Cannot read license file at {path}: {reason}")]
    ReadFailed { path: PathBuf, reason: String },

    #[error("Cannot write license file at {path}: {reason}")]
    WriteFailed { path: PathBuf, reason: String },

    #[error("License file at {path} is malformed: {reason}")]
    ParseFailed { path: PathBuf, reason: String },
}
