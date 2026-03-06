//! HTTPS communication with the ConvX license server.
//!
//! All network calls go through this module. The rest of the license
//! system is offline-capable by design.

use crate::license::fingerprint::DeviceFingerprint;
use crate::license::keyfile::LicenseFile;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default base URL for the license API.
const DEFAULT_API_BASE: &str = match option_env!("CONVX_LICENSE_API") {
    Some(url) => url,
    None => "https://api.convx.dev/v1/license",
};

/// Resolve the API base URL. Checks `CONVX_LICENSE_API` at runtime first,
/// then falls back to the compile-time default.
fn api_base() -> String {
    std::env::var("CONVX_LICENSE_API").unwrap_or_else(|_| DEFAULT_API_BASE.to_string())
}

const USER_AGENT: &str = concat!("convx/", env!("CARGO_PKG_VERSION"));
const REQUEST_TIMEOUT_SECS: u64 = 10;

// ─── Request / Response types ──────────────────────────────────────────────

#[derive(Serialize)]
struct ActivateRequest {
    key: String,
    device: DeviceFingerprint,
}

#[derive(Deserialize)]
pub struct ActivateResponse {
    pub key: String,
    pub device_id: String,
    pub activated_at: DateTime<Utc>,
    pub recheck_after: DateTime<Utc>,
    /// Base64-encoded Ed25519 signature over the canonical payload.
    pub signature: String,
}

#[derive(Serialize)]
struct ValidateRequest {
    key: String,
    device_id: String,
    tier1_hash: String,
    tier2_hash: String,
}

#[derive(Deserialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub revoked: bool,
    pub recheck_after: DateTime<Utc>,
    pub signature: String,
    /// If the license is on a different device, the server tells us.
    pub active_device_name: Option<String>,
}

#[derive(Serialize)]
struct DeactivateRequest {
    key: String,
    device_id: String,
}

#[derive(Deserialize)]
pub struct DeactivateResponse {
    pub deactivated: bool,
}

#[derive(Serialize)]
struct TransferRequest {
    key: String,
    new_device: DeviceFingerprint,
}

#[derive(Deserialize)]
pub struct TransferResponse {
    pub transferred: bool,
    pub activated_at: DateTime<Utc>,
    pub recheck_after: DateTime<Utc>,
    pub signature: String,
}

// ─── API calls ─────────────────────────────────────────────────────────────

/// Activate a license key on this device.
/// Returns the signed license data to write to disk.
pub fn activate(key: &str, device: &DeviceFingerprint) -> Result<ActivateResponse, ApiError> {
    let body = ActivateRequest {
        key: key.to_string(),
        device: device.clone(),
    };

    let resp = http_post(&format!("{}/activate", api_base()), &body)?;
    let status = resp.status();
    let text = resp.text().map_err(|e| ApiError::Network(e.to_string()))?;

    if status == 409 {
        // Conflict — key already active on another device
        let info: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}));
        let device_name = info
            .get("active_device_name")
            .and_then(|v| v.as_str())
            .unwrap_or("another device")
            .to_string();
        return Err(ApiError::AlreadyActivated { device_name });
    }

    if !status.is_success() {
        return Err(ApiError::Server {
            status: status.as_u16(),
            body: text,
        });
    }

    serde_json::from_str(&text).map_err(|e| ApiError::InvalidResponse(e.to_string()))
}

/// Transfer a license from its current device to this one.
/// The server unbinds the old device and binds the new one.
pub fn transfer(key: &str, new_device: &DeviceFingerprint) -> Result<TransferResponse, ApiError> {
    let body = TransferRequest {
        key: key.to_string(),
        new_device: new_device.clone(),
    };

    let resp = http_post(&format!("{}/transfer", api_base()), &body)?;
    let status = resp.status();
    let text = resp.text().map_err(|e| ApiError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(ApiError::Server {
            status: status.as_u16(),
            body: text,
        });
    }

    serde_json::from_str(&text).map_err(|e| ApiError::InvalidResponse(e.to_string()))
}

/// Periodic revalidation: confirm the key is still valid and get a fresh
/// recheck_after timestamp.
pub fn validate(license: &LicenseFile) -> Result<ValidateResponse, ApiError> {
    let body = ValidateRequest {
        key: license.key.clone(),
        device_id: license.device.device_id.clone(),
        tier1_hash: license.device.tier1_hash.clone(),
        tier2_hash: license.device.tier2_hash.clone(),
    };

    let resp = http_post(&format!("{}/validate", api_base()), &body)?;
    let status = resp.status();
    let text = resp.text().map_err(|e| ApiError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(ApiError::Server {
            status: status.as_u16(),
            body: text,
        });
    }

    serde_json::from_str(&text).map_err(|e| ApiError::InvalidResponse(e.to_string()))
}

/// Voluntarily deactivate this device, freeing the slot.
pub fn deactivate(key: &str, device_id: &str) -> Result<DeactivateResponse, ApiError> {
    let body = DeactivateRequest {
        key: key.to_string(),
        device_id: device_id.to_string(),
    };

    let resp = http_post(&format!("{}/deactivate", api_base()), &body)?;
    let status = resp.status();
    let text = resp.text().map_err(|e| ApiError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(ApiError::Server {
            status: status.as_u16(),
            body: text,
        });
    }

    serde_json::from_str(&text).map_err(|e| ApiError::InvalidResponse(e.to_string()))
}

/// Fetch org settings for a license key.
/// Returns None if the key is not associated with an org.
pub fn fetch_org_settings(key: &str) -> Result<Option<super::EnterpriseSettings>, ApiError> {
    let url = format!(
        "{}/settings-by-key",
        api_base().replace("/v1/license", "/v1/org")
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let resp = client
        .get(&url)
        .query(&[("key", key)])
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                ApiError::Timeout
            } else if e.is_connect() {
                ApiError::Offline
            } else {
                ApiError::Network(e.to_string())
            }
        })?;

    if resp.status().as_u16() == 404 {
        return Ok(None);
    }

    let status = resp.status();
    let text = resp.text().map_err(|e| ApiError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(ApiError::Server {
            status: status.as_u16(),
            body: text,
        });
    }

    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| ApiError::InvalidResponse(e.to_string()))?;

    if let Some(settings) = body.get("settings") {
        let enterprise_settings: super::EnterpriseSettings =
            serde_json::from_value(settings.clone())
                .map_err(|e| ApiError::InvalidResponse(e.to_string()))?;
        Ok(Some(enterprise_settings))
    } else {
        Ok(None)
    }
}

// ─── HTTP helper ───────────────────────────────────────────────────────────

fn http_post<T: Serialize>(url: &str, body: &T) -> Result<reqwest::blocking::Response, ApiError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| ApiError::Network(e.to_string()))?;

    client.post(url).json(body).send().map_err(|e| {
        if e.is_timeout() {
            ApiError::Timeout
        } else if e.is_connect() {
            ApiError::Offline
        } else {
            ApiError::Network(e.to_string())
        }
    })
}

// ─── Errors ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("No internet connection — license check skipped")]
    Offline,

    #[error("License server timed out")]
    Timeout,

    #[error("Network error: {0}")]
    Network(String),

    #[error("License already active on: \"{device_name}\"")]
    AlreadyActivated { device_name: String },

    #[error("License server returned {status}: {body}")]
    Server { status: u16, body: String },

    #[error("Unexpected response from license server: {0}")]
    InvalidResponse(String),
}

impl ApiError {
    /// Returns true if this error means we simply couldn't reach the server
    /// (offline, timeout, DNS failure) — not a definitive rejection.
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Offline | Self::Timeout | Self::Network(_))
    }
}
