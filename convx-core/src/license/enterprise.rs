//! Enterprise configuration for silent activation, centralized settings,
//! and conversion audit logging.
//!
//! Precedence for license key:
//!   1. `CONVX_LICENSE_KEY` env var
//!   2. `~/.convx/enterprise-config.json`
//!   3. System-level paths (MDM deployment):
//!      - macOS: `/Library/Application Support/convx/enterprise-config.json`
//!      - Linux: `/etc/convx/enterprise-config.json`
//!      - Windows: `%PROGRAMDATA%\convx\enterprise-config.json`

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Enterprise configuration loaded from JSON file or env.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<EnterpriseSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_endpoint: Option<String>,
}

/// Centralized settings pushed from the org admin dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_quality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite_existing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_notifications: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_formats: Option<Vec<String>>,
    /// If true, settings are locked — users cannot override locally.
    #[serde(default)]
    pub locked: bool,
}

/// Metadata-only conversion audit event (never includes file content).
#[derive(Debug, Serialize)]
pub struct ConversionAuditEvent {
    pub input_format: String,
    pub output_format: String,
    pub input_size: u64,
    pub output_size: u64,
    pub duration_ms: u64,
    pub platform: String,
    pub timestamp: String,
}

impl EnterpriseConfig {
    /// Load enterprise configuration from all known locations.
    /// Returns None if no enterprise config is found.
    pub fn load() -> Option<Self> {
        // 1. Check user-level config
        if let Some(path) = user_config_path() {
            if let Some(config) = Self::load_from(&path) {
                return Some(config);
            }
        }

        // 2. Check system-level config (MDM deployment)
        for path in system_config_paths() {
            if let Some(config) = Self::load_from(&path) {
                return Some(config);
            }
        }

        None
    }

    fn load_from(path: &PathBuf) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Save enterprise config to the user config path.
    pub fn save(&self) -> Result<(), String> {
        let path = user_config_path().ok_or("Cannot determine config path")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }
}

/// Try to auto-activate using enterprise config.
///
/// Returns:
/// - `Some(Ok(...))` if activation succeeded or already active
/// - `Some(Err(...))` if activation was attempted but failed
/// - `None` if no enterprise key is configured
pub fn auto_activate() -> Option<Result<super::ActivateOutcome, String>> {
    // 1. Check env var
    if let Ok(key) = std::env::var("CONVX_LICENSE_KEY") {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(super::activate(trimmed));
        }
    }

    // 2. Check enterprise config file
    if let Some(config) = EnterpriseConfig::load() {
        if let Some(key) = config.license_key {
            let trimmed = key.trim();
            if !trimmed.is_empty() {
                return Some(super::activate(trimmed));
            }
        }
    }

    None
}

/// Fetch org settings from the license server.
/// Called after activation if the key belongs to an org.
pub fn fetch_org_settings(key: &str) -> Result<Option<EnterpriseSettings>, String> {
    let api_base =
        std::env::var("CONVX_LICENSE_API").unwrap_or_else(|_| "https://api.convx.dev".to_string());

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{}/v1/org/settings-by-key", api_base))
        .query(&[("key", key)])
        .send()
        .map_err(|e| e.to_string())?;

    if resp.status() == 404 {
        // Not an org key
        return Ok(None);
    }

    if !resp.status().is_success() {
        return Err(format!("Server returned {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

    if let Some(settings) = body.get("settings") {
        let enterprise_settings: EnterpriseSettings =
            serde_json::from_value(settings.clone()).map_err(|e| e.to_string())?;
        Ok(Some(enterprise_settings))
    } else {
        Ok(None)
    }
}

/// Fire-and-forget audit event to the enterprise endpoint.
/// Never blocks or fails the calling operation.
pub fn send_audit_event(config: &EnterpriseConfig, event: ConversionAuditEvent) {
    let endpoint = match &config.audit_endpoint {
        Some(url) => url.clone(),
        None => {
            let api_base = std::env::var("CONVX_LICENSE_API")
                .unwrap_or_else(|_| "https://api.convx.dev".to_string());
            match &config.org_id {
                Some(org_id) => format!("{}/v1/org/{}/audit", api_base, org_id),
                None => return, // No org, no audit
            }
        }
    };

    // Fire and forget in a separate thread
    std::thread::spawn(move || {
        let _ = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .and_then(|client| {
                client
                    .post(&endpoint)
                    .json(&serde_json::json!({
                        "action": "convert",
                        "metadata": event,
                    }))
                    .send()
            });
    });
}

// ─── Path helpers ────────────────────────────────────────────────────────

fn user_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".convx").join("enterprise-config.json"))
}

fn system_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(
            "/Library/Application Support/convx/enterprise-config.json",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/etc/convx/enterprise-config.json"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(programdata) = std::env::var("PROGRAMDATA") {
            paths.push(
                PathBuf::from(programdata)
                    .join("convx")
                    .join("enterprise-config.json"),
            );
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_config_deserialize() {
        let json = r#"{
            "license_key": "CONVX-TEST-TEST-TEST-TEST",
            "org_id": "abc-123",
            "settings": {
                "default_quality": 90,
                "default_format": "webp",
                "locked": true
            },
            "audit_endpoint": "https://api.convx.dev/v1/org/abc-123/audit"
        }"#;

        let config: EnterpriseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.license_key.unwrap(), "CONVX-TEST-TEST-TEST-TEST");
        assert_eq!(config.org_id.unwrap(), "abc-123");
        assert!(config.settings.unwrap().locked);
    }

    #[test]
    fn enterprise_config_minimal() {
        let json = r#"{ "license_key": "CONVX-AAAA-BBBB-CCCC-DDDD" }"#;
        let config: EnterpriseConfig = serde_json::from_str(json).unwrap();
        assert!(config.org_id.is_none());
        assert!(config.settings.is_none());
    }

    #[test]
    fn auto_activate_returns_none_without_config() {
        // In test env with no CONVX_LICENSE_KEY and no config file
        std::env::remove_var("CONVX_LICENSE_KEY");
        let result = auto_activate();
        assert!(result.is_none());
    }
}
