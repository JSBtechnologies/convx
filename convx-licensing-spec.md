# convx Licensing Specification

**Version:** 1.0.0  
**Tiers:** Standard, Pro (Team: future)

---

## Pricing Summary

| Tier | Price | Model | Target |
|------|-------|-------|--------|
| **Standard** | $29 | One-time, lifetime | Developers, creators, ML engineers |
| **Pro** | $49/year | Annual subscription | Power users who want sync + mobile |
| **Team** | $8/seat/month | Subscription | Teams (future) |

---

## Feature Matrix

| Feature | Standard ($29) | Pro ($49/yr) |
|---------|----------------|--------------|
| **Core Conversion** | | |
| Image formats (PNG, JPG, WebP, GIF, HEIC, AVIF, etc.) | ✅ | ✅ |
| Video formats (MP4, MOV, WebM, AVI, MKV, GIF) | ✅ | ✅ |
| Audio formats (MP3, WAV, FLAC, M4A, AAC, OGG) | ✅ | ✅ |
| Document formats (PDF, DOCX, TXT, MD, HTML) | ✅ | ✅ |
| Quality/resize controls | ✅ | ✅ |
| **Batch Processing** | | |
| Unlimited batch | ✅ | ✅ |
| Parallel processing (`--jobs`) | ✅ | ✅ |
| Watch mode | ✅ | ✅ |
| Pipeline configs (`convx.yaml`) | ✅ | ✅ |
| **ML Features** | | |
| Background removal (`--remove-bg`) | ✅ | ✅ |
| Upscaling (`--upscale 2x/4x`) | ✅ | ✅ |
| Image denoising (`--denoise`) | ✅ | ✅ |
| Audio denoising (`--denoise`) | ✅ | ✅ |
| Face restoration (`--restore-faces`) | ✅ | ✅ |
| Transcription (`convx transcribe`) | ✅ | ✅ |
| OCR (`convx ocr`) | ✅ | ✅ |
| GPU acceleration | ✅ | ✅ |
| **MCP Integration** | | |
| MCP server | ✅ | ✅ |
| AI agent tools | ✅ | ✅ |
| **Interfaces** | | |
| CLI | ✅ | ✅ |
| Desktop app (Mac, Windows, Linux) | ✅ | ✅ |
| Web app | ❌ | ✅ |
| Mobile app (iOS, Android) | ❌ | ✅ |
| **Cloud & Sync** | | |
| Cloud sync | ❌ | ✅ |
| Conversion history (synced) | ❌ | ✅ |
| Presets sync | ❌ | ✅ |
| Cross-device continuity | ❌ | ✅ |
| **Devices** | | |
| Device limit | 3 | 5 |
| **Support** | | |
| Community (GitHub) | ✅ | ✅ |
| Email support | ❌ | ✅ |
| Priority issues | ❌ | ✅ |

---

## License Key Format

### Structure

```
CONVX-{TIER}-{PAYLOAD}-{SIGNATURE}

Examples:
CONVX-STD-A1B2C3D4E5F6-X9Y8Z7
CONVX-PRO-G7H8I9J0K1L2-W6V5U4
```

### Components

| Component | Description | Example |
|-----------|-------------|---------|
| Prefix | Always `CONVX` | `CONVX` |
| Tier | `STD` or `PRO` | `STD` |
| Payload | Base32-encoded data | `A1B2C3D4E5F6` |
| Signature | HMAC verification | `X9Y8Z7` |

### Payload Contents (Encoded)

```rust
#[derive(Serialize, Deserialize)]
struct LicensePayload {
    /// License tier
    tier: Tier,
    
    /// Email hash (first 8 chars of SHA256)
    email_hash: String,
    
    /// Issue timestamp (Unix epoch)
    issued_at: u64,
    
    /// Expiration timestamp (None for Standard)
    expires_at: Option<u64>,
    
    /// License ID (for revocation checking)
    license_id: String,
}
```

---

## License File

### Location

```
~/.convx/license.json
```

### Structure

```json
{
  "key": "CONVX-STD-A1B2C3D4E5F6-X9Y8Z7",
  "tier": "standard",
  "email": "user@example.com",
  "issued_at": "2025-01-26T00:00:00Z",
  "expires_at": null,
  "device_id": "d8f7e6a5b4c3",
  "activated_at": "2025-01-26T12:34:56Z"
}
```

---

## Implementation

### Core Types

```rust
// src/license/mod.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Standard,
    Pro,
}

impl Tier {
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "STD" => Some(Tier::Standard),
            "PRO" => Some(Tier::Pro),
            _ => None,
        }
    }
    
    pub fn code(&self) -> &'static str {
        match self {
            Tier::Standard => "STD",
            Tier::Pro => "PRO",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Tier::Standard => "Standard",
            Tier::Pro => "Pro",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub tier: Tier,
    pub email: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub device_id: String,
    pub activated_at: DateTime<Utc>,
}

impl License {
    /// Check if license is valid (not expired)
    pub fn is_valid(&self) -> bool {
        match self.expires_at {
            Some(expires) => Utc::now() < expires,
            None => true, // Standard never expires
        }
    }
    
    /// Check if license is Pro tier
    pub fn is_pro(&self) -> bool {
        self.tier == Tier::Pro
    }
    
    /// Days until expiration (None if never expires)
    pub fn days_remaining(&self) -> Option<i64> {
        self.expires_at.map(|expires| {
            (expires - Utc::now()).num_days()
        })
    }
}

#[derive(Debug, Clone)]
pub enum LicenseStatus {
    /// No license found
    None,
    
    /// Valid license
    Valid(License),
    
    /// License expired
    Expired(License),
    
    /// License revoked
    Revoked(License),
    
    /// Invalid license key
    Invalid(String),
}
```

### License Manager

```rust
// src/license/manager.rs

use crate::license::{License, LicenseStatus, Tier};
use crate::ConvxError;
use std::path::PathBuf;
use std::fs;

pub struct LicenseManager {
    license_path: PathBuf,
    api_base: String,
    public_key: String,
}

impl LicenseManager {
    pub fn new() -> Self {
        let license_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".convx")
            .join("license.json");
        
        Self {
            license_path,
            api_base: "https://api.convx.dev".to_string(),
            public_key: include_str!("../../keys/public.pem").to_string(),
        }
    }
    
    /// Get current license status
    pub fn status(&self) -> LicenseStatus {
        match self.load() {
            Ok(license) => {
                if license.is_valid() {
                    LicenseStatus::Valid(license)
                } else {
                    LicenseStatus::Expired(license)
                }
            }
            Err(_) => LicenseStatus::None,
        }
    }
    
    /// Load license from disk
    pub fn load(&self) -> Result<License, ConvxError> {
        let content = fs::read_to_string(&self.license_path)
            .map_err(|_| ConvxError::LicenseNotFound)?;
        
        let license: License = serde_json::from_str(&content)
            .map_err(|_| ConvxError::LicenseCorrupted)?;
        
        // Verify signature
        if !self.verify_key(&license.key)? {
            return Err(ConvxError::LicenseInvalid);
        }
        
        Ok(license)
    }
    
    /// Activate a license key
    pub async fn activate(&self, key: &str) -> Result<License, ConvxError> {
        // 1. Verify key format and signature locally
        if !self.verify_key(key)? {
            return Err(ConvxError::LicenseInvalid);
        }
        
        // 2. Parse key to get tier and check format
        let tier = self.parse_tier(key)?;
        
        // 3. Call API to activate (registers device)
        let device_id = self.get_device_id();
        let response = self.api_activate(key, &device_id).await?;
        
        // 4. Create license struct
        let license = License {
            key: key.to_string(),
            tier,
            email: response.email,
            issued_at: response.issued_at,
            expires_at: response.expires_at,
            device_id,
            activated_at: chrono::Utc::now(),
        };
        
        // 5. Save to disk
        self.save(&license)?;
        
        Ok(license)
    }
    
    /// Deactivate license on this device
    pub async fn deactivate(&self) -> Result<(), ConvxError> {
        let license = self.load()?;
        
        // Call API to deactivate
        self.api_deactivate(&license.key, &license.device_id).await?;
        
        // Remove local license file
        fs::remove_file(&self.license_path)
            .map_err(|e| ConvxError::IoError { reason: e.to_string() })?;
        
        Ok(())
    }
    
    /// Save license to disk
    fn save(&self, license: &License) -> Result<(), ConvxError> {
        // Ensure directory exists
        if let Some(parent) = self.license_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(license)?;
        fs::write(&self.license_path, content)?;
        
        Ok(())
    }
    
    /// Verify license key signature
    fn verify_key(&self, key: &str) -> Result<bool, ConvxError> {
        // Parse key parts
        let parts: Vec<&str> = key.split('-').collect();
        if parts.len() != 4 || parts[0] != "CONVX" {
            return Ok(false);
        }
        
        let tier_code = parts[1];
        let payload = parts[2];
        let signature = parts[3];
        
        // Verify tier code
        if Tier::from_code(tier_code).is_none() {
            return Ok(false);
        }
        
        // Verify HMAC signature
        let message = format!("CONVX-{}-{}", tier_code, payload);
        let valid = self.verify_hmac(&message, signature)?;
        
        Ok(valid)
    }
    
    /// Parse tier from key
    fn parse_tier(&self, key: &str) -> Result<Tier, ConvxError> {
        let parts: Vec<&str> = key.split('-').collect();
        if parts.len() < 2 {
            return Err(ConvxError::LicenseInvalid);
        }
        
        Tier::from_code(parts[1])
            .ok_or(ConvxError::LicenseInvalid)
    }
    
    /// Generate unique device ID
    fn get_device_id(&self) -> String {
        // Combine: machine ID + username + home dir hash
        let machine_id = machine_uid::get()
            .unwrap_or_else(|_| "unknown".to_string());
        
        let username = whoami::username();
        
        let combined = format!("{}-{}", machine_id, username);
        
        // Hash to fixed length
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..6]) // First 12 hex chars
    }
    
    fn verify_hmac(&self, message: &str, signature: &str) -> Result<bool, ConvxError> {
        // HMAC verification using public key
        // Implementation depends on crypto library choice
        todo!("Implement HMAC verification")
    }
    
    async fn api_activate(&self, key: &str, device_id: &str) -> Result<ActivateResponse, ConvxError> {
        let client = reqwest::Client::new();
        
        let response = client
            .post(format!("{}/v1/licenses/activate", self.api_base))
            .json(&serde_json::json!({
                "key": key,
                "device_id": device_id,
                "app_version": env!("CARGO_PKG_VERSION"),
                "os": std::env::consts::OS,
            }))
            .send()
            .await
            .map_err(|e| ConvxError::NetworkError { reason: e.to_string() })?;
        
        if response.status() == 404 {
            return Err(ConvxError::LicenseNotFound);
        }
        
        if response.status() == 403 {
            return Err(ConvxError::DeviceLimitReached);
        }
        
        if !response.status().is_success() {
            return Err(ConvxError::ActivationFailed {
                reason: response.text().await.unwrap_or_default(),
            });
        }
        
        response.json().await
            .map_err(|e| ConvxError::NetworkError { reason: e.to_string() })
    }
    
    async fn api_deactivate(&self, key: &str, device_id: &str) -> Result<(), ConvxError> {
        let client = reqwest::Client::new();
        
        client
            .post(format!("{}/v1/licenses/deactivate", self.api_base))
            .json(&serde_json::json!({
                "key": key,
                "device_id": device_id,
            }))
            .send()
            .await
            .map_err(|e| ConvxError::NetworkError { reason: e.to_string() })?;
        
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ActivateResponse {
    email: String,
    issued_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    devices_used: u32,
    devices_limit: u32,
}
```

### Feature Gating

```rust
// src/license/features.rs

use crate::license::{LicenseManager, LicenseStatus, Tier};
use crate::ConvxError;

/// Features that require specific license tiers
#[derive(Debug, Clone, Copy)]
pub enum Feature {
    // Standard features (require any license)
    Convert,
    Batch,
    RemoveBg,
    Upscale,
    Denoise,
    RestoreFaces,
    Transcribe,
    Ocr,
    Mcp,
    Gpu,
    
    // Pro features (require Pro tier)
    WebApp,
    MobileApp,
    CloudSync,
    HistorySync,
    PresetsSync,
}

impl Feature {
    /// Minimum tier required for this feature
    pub fn required_tier(&self) -> Tier {
        match self {
            // All Standard features
            Feature::Convert
            | Feature::Batch
            | Feature::RemoveBg
            | Feature::Upscale
            | Feature::Denoise
            | Feature::RestoreFaces
            | Feature::Transcribe
            | Feature::Ocr
            | Feature::Mcp
            | Feature::Gpu => Tier::Standard,
            
            // Pro-only features
            Feature::WebApp
            | Feature::MobileApp
            | Feature::CloudSync
            | Feature::HistorySync
            | Feature::PresetsSync => Tier::Pro,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Feature::Convert => "File conversion",
            Feature::Batch => "Batch processing",
            Feature::RemoveBg => "Background removal",
            Feature::Upscale => "Image upscaling",
            Feature::Denoise => "Denoising",
            Feature::RestoreFaces => "Face restoration",
            Feature::Transcribe => "Transcription",
            Feature::Ocr => "OCR",
            Feature::Mcp => "MCP server",
            Feature::Gpu => "GPU acceleration",
            Feature::WebApp => "Web app",
            Feature::MobileApp => "Mobile app",
            Feature::CloudSync => "Cloud sync",
            Feature::HistorySync => "History sync",
            Feature::PresetsSync => "Presets sync",
        }
    }
}

/// Check if a feature is available
pub fn check_feature(feature: Feature) -> Result<(), ConvxError> {
    let manager = LicenseManager::new();
    
    match manager.status() {
        LicenseStatus::None => {
            Err(ConvxError::LicenseRequired {
                feature: feature.name().to_string(),
            })
        }
        
        LicenseStatus::Expired(license) => {
            Err(ConvxError::LicenseExpired {
                tier: license.tier.name().to_string(),
                expired_at: license.expires_at.unwrap(),
            })
        }
        
        LicenseStatus::Revoked(_) => {
            Err(ConvxError::LicenseRevoked)
        }
        
        LicenseStatus::Invalid(key) => {
            Err(ConvxError::LicenseInvalid)
        }
        
        LicenseStatus::Valid(license) => {
            let required = feature.required_tier();
            let has_tier = match (license.tier, required) {
                (Tier::Pro, _) => true,
                (Tier::Standard, Tier::Standard) => true,
                (Tier::Standard, Tier::Pro) => false,
            };
            
            if has_tier {
                Ok(())
            } else {
                Err(ConvxError::FeatureRequiresPro {
                    feature: feature.name().to_string(),
                })
            }
        }
    }
}

/// Require a feature or exit with error message
#[macro_export]
macro_rules! require_feature {
    ($feature:expr) => {
        if let Err(e) = $crate::license::features::check_feature($feature) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
}
```

### Error Types

```rust
// src/error.rs (additions)

#[derive(Debug, thiserror::Error)]
pub enum ConvxError {
    // ... existing errors ...
    
    // License errors
    #[error("No license found. Purchase at https://convx.dev")]
    LicenseNotFound,
    
    #[error("License file is corrupted. Please re-activate.")]
    LicenseCorrupted,
    
    #[error("Invalid license key")]
    LicenseInvalid,
    
    #[error("License expired on {expired_at}. Renew at https://convx.dev/renew")]
    LicenseExpired {
        tier: String,
        expired_at: chrono::DateTime<chrono::Utc>,
    },
    
    #[error("License has been revoked. Contact support@convx.dev")]
    LicenseRevoked,
    
    #[error("License required for {feature}. Purchase at https://convx.dev")]
    LicenseRequired { feature: String },
    
    #[error("{feature} requires Pro. Upgrade at https://convx.dev/pro")]
    FeatureRequiresPro { feature: String },
    
    #[error("Device limit reached. Deactivate another device first.")]
    DeviceLimitReached,
    
    #[error("Activation failed: {reason}")]
    ActivationFailed { reason: String },
}
```

---

## CLI Commands

### License Commands

```rust
// src/main.rs (additions)

#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Activate a license
    Activate {
        /// License key
        key: String,
    },
    
    /// Show license status
    License,
    
    /// Deactivate license from this device
    Deactivate,
}

async fn handle_activate(key: &str) -> Result<(), ConvxError> {
    let manager = LicenseManager::new();
    
    println!("Activating license...");
    
    let license = manager.activate(key).await?;
    
    println!();
    println!("✅ {} license activated", license.tier.name());
    println!();
    println!("   Email: {}", license.email);
    
    if let Some(expires) = license.expires_at {
        println!("   Expires: {}", expires.format("%Y-%m-%d"));
    } else {
        println!("   Expires: Never");
    }
    
    println!();
    println!("Thank you for purchasing convx!");
    
    Ok(())
}

fn handle_license() -> Result<(), ConvxError> {
    let manager = LicenseManager::new();
    
    match manager.status() {
        LicenseStatus::None => {
            println!();
            println!("  No license found");
            println!();
            println!("  Purchase at https://convx.dev");
            println!("  Activate with: convx activate <key>");
            println!();
        }
        
        LicenseStatus::Valid(license) => {
            println!();
            println!("  License: {} ✅", license.tier.name());
            println!("  Email: {}", license.email);
            
            if let Some(days) = license.days_remaining() {
                println!("  Expires: {} days", days);
            } else {
                println!("  Expires: Never");
            }
            
            println!("  Device: {}", license.device_id);
            println!();
        }
        
        LicenseStatus::Expired(license) => {
            println!();
            println!("  License: {} ⚠️ EXPIRED", license.tier.name());
            println!("  Email: {}", license.email);
            println!("  Expired: {}", license.expires_at.unwrap().format("%Y-%m-%d"));
            println!();
            println!("  Renew at https://convx.dev/renew");
            println!();
        }
        
        LicenseStatus::Revoked(_) => {
            println!();
            println!("  License: REVOKED ❌");
            println!();
            println!("  Contact support@convx.dev");
            println!();
        }
        
        LicenseStatus::Invalid(_) => {
            println!();
            println!("  License: INVALID ❌");
            println!();
            println!("  Re-activate with: convx activate <key>");
            println!();
        }
    }
    
    Ok(())
}

async fn handle_deactivate() -> Result<(), ConvxError> {
    let manager = LicenseManager::new();
    
    println!("Deactivating license...");
    
    manager.deactivate().await?;
    
    println!("✅ License deactivated from this device");
    println!();
    println!("You can activate on another device with your license key.");
    
    Ok(())
}
```

### CLI Usage Examples

```bash
# Activate Standard license
$ convx activate CONVX-STD-A1B2C3D4E5F6-X9Y8Z7

Activating license...

✅ Standard license activated

   Email: user@example.com
   Expires: Never

Thank you for purchasing convx!

# Activate Pro license
$ convx activate CONVX-PRO-G7H8I9J0K1L2-W6V5U4

Activating license...

✅ Pro license activated

   Email: user@example.com
   Expires: 2026-01-26

Thank you for purchasing convx!

# Check license status
$ convx license

  License: Standard ✅
  Email: user@example.com
  Expires: Never
  Device: d8f7e6a5b4c3

# Deactivate
$ convx deactivate

Deactivating license...
✅ License deactivated from this device

You can activate on another device with your license key.
```

---

## No License Experience

When running without a license:

```bash
$ convx convert image.png --to webp

╭─────────────────────────────────────────────────────────────╮
│                                                             │
│   convx requires a license                                  │
│                                                             │
│   Standard  $29 one-time                                    │
│   ─────────────────────────────────────                     │
│   • Convert any format (image, video, audio, docs)          │
│   • ML features (remove-bg, upscale, denoise, transcribe)   │
│   • MCP server for AI agents                                │
│   • CLI + Desktop app                                       │
│   • Unlimited usage, forever                                │
│                                                             │
│   Pro  $49/year                                             │
│   ─────────────────────────────────────                     │
│   • Everything in Standard                                  │
│   • Web app + Mobile app                                    │
│   • Cloud sync across devices                               │
│   • Priority support                                        │
│                                                             │
│   → Purchase: https://convx.dev                             │
│   → Activate: convx activate <key>                          │
│                                                             │
╰─────────────────────────────────────────────────────────────╯
```

---

## Pro Feature Gating

When Standard user tries Pro feature:

```bash
$ convx sync

╭─────────────────────────────────────────────────────────────╮
│                                                             │
│   Cloud sync requires Pro ($49/year)                        │
│                                                             │
│   Pro includes:                                             │
│   • Web app + Mobile app                                    │
│   • Sync across all your devices                            │
│   • Conversion history                                      │
│   • Priority support                                        │
│                                                             │
│   → Upgrade: https://convx.dev/pro                          │
│                                                             │
╰─────────────────────────────────────────────────────────────╯
```

---

## Expired License

```bash
$ convx convert image.png --to webp

╭─────────────────────────────────────────────────────────────╮
│                                                             │
│   ⚠️  Your Pro license expired on 2025-12-15                │
│                                                             │
│   Your Standard features still work, but you've lost:       │
│   • Web app + Mobile app access                             │
│   • Cloud sync                                              │
│   • Priority support                                        │
│                                                             │
│   → Renew Pro: https://convx.dev/renew                      │
│   → Or continue with Standard features                      │
│                                                             │
╰─────────────────────────────────────────────────────────────╯

Converting image.png → image.webp
✅ Done (1.2 MB → 340 KB, 72% smaller)
```

**Note:** Expired Pro gracefully degrades to Standard, not zero functionality.

---

## Backend API

### Endpoints

```
POST /v1/licenses/activate
POST /v1/licenses/deactivate
GET  /v1/licenses/verify
POST /v1/licenses/check-updates
```

### Activate Request

```json
POST /v1/licenses/activate
{
  "key": "CONVX-STD-A1B2C3D4E5F6-X9Y8Z7",
  "device_id": "d8f7e6a5b4c3",
  "app_version": "1.0.0",
  "os": "macos"
}
```

### Activate Response

```json
{
  "success": true,
  "email": "user@example.com",
  "tier": "standard",
  "issued_at": "2025-01-26T00:00:00Z",
  "expires_at": null,
  "devices_used": 1,
  "devices_limit": 3
}
```

### Error Responses

```json
// Invalid key
{
  "success": false,
  "error": "invalid_key",
  "message": "License key is invalid"
}

// Device limit reached
{
  "success": false,
  "error": "device_limit",
  "message": "Device limit reached (3/3)",
  "devices": [
    { "id": "abc123", "name": "MacBook Pro", "last_seen": "2025-01-25" },
    { "id": "def456", "name": "Desktop", "last_seen": "2025-01-20" },
    { "id": "ghi789", "name": "Laptop", "last_seen": "2025-01-15" }
  ]
}

// Already activated on this device
{
  "success": true,
  "message": "Already activated on this device",
  "email": "user@example.com",
  "tier": "standard"
}
```

---

## Payment Integration

### Recommended: LemonSqueezy

Why:
- Handles global taxes automatically
- 5% + payment fees (lower than Gumroad's 10%)
- Webhooks for license generation
- Built-in license key generation
- Subscription management for Pro

### Webhook Flow

```
Customer Purchase → LemonSqueezy → Webhook → convx API → Generate Key → Email to Customer
```

### Webhook Handler

```rust
// convx-api/src/webhooks/lemonsqueezy.rs

async fn handle_order_created(payload: LemonSqueezyWebhook) -> Result<(), Error> {
    let order = payload.data;
    
    // Determine tier from product
    let tier = match order.product_id {
        STANDARD_PRODUCT_ID => Tier::Standard,
        PRO_PRODUCT_ID => Tier::Pro,
        _ => return Err(Error::UnknownProduct),
    };
    
    // Generate license key
    let key = generate_license_key(tier, &order.customer_email);
    
    // Store in database
    db::licenses::create(License {
        key: key.clone(),
        tier,
        email: order.customer_email.clone(),
        order_id: order.id,
        issued_at: Utc::now(),
        expires_at: if tier == Tier::Pro {
            Some(Utc::now() + Duration::days(365))
        } else {
            None
        },
    }).await?;
    
    // Email license key to customer
    email::send_license_key(&order.customer_email, &key, tier).await?;
    
    Ok(())
}
```

---

## Database Schema

```sql
-- licenses table
CREATE TABLE licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(50) UNIQUE NOT NULL,
    tier VARCHAR(20) NOT NULL,  -- 'standard' or 'pro'
    email VARCHAR(255) NOT NULL,
    order_id VARCHAR(100),
    issued_at TIMESTAMP WITH TIME ZONE NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE,
    revoked_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- device activations
CREATE TABLE activations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_id UUID REFERENCES licenses(id),
    device_id VARCHAR(50) NOT NULL,
    device_name VARCHAR(255),
    os VARCHAR(50),
    app_version VARCHAR(20),
    activated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deactivated_at TIMESTAMP WITH TIME ZONE,
    
    UNIQUE(license_id, device_id)
);

-- indexes
CREATE INDEX idx_licenses_email ON licenses(email);
CREATE INDEX idx_licenses_key ON licenses(key);
CREATE INDEX idx_activations_license ON activations(license_id);
CREATE INDEX idx_activations_device ON activations(device_id);
```

---

## Offline Capability

### Standard License

- **Fully offline after activation**
- License file stored locally
- Cryptographic signature verified locally
- No phone-home required

### Pro License

- **Requires periodic online check** (once per 30 days)
- Verifies subscription still active
- Grace period: 7 days offline after last check
- After grace period: degrades to Standard features

```rust
impl License {
    pub fn needs_online_check(&self) -> bool {
        if self.tier != Tier::Pro {
            return false;
        }
        
        let last_check = self.last_online_check.unwrap_or(self.activated_at);
        let days_since = (Utc::now() - last_check).num_days();
        
        days_since > 30
    }
    
    pub fn is_in_grace_period(&self) -> bool {
        if self.tier != Tier::Pro {
            return false;
        }
        
        let last_check = self.last_online_check.unwrap_or(self.activated_at);
        let days_since = (Utc::now() - last_check).num_days();
        
        days_since > 30 && days_since <= 37  // 7 day grace
    }
}
```

---

## Summary

| Aspect | Standard | Pro |
|--------|----------|-----|
| **Price** | $29 one-time | $49/year |
| **License type** | Perpetual | Annual |
| **Devices** | 3 | 5 |
| **Offline** | Full | 30 days + 7 day grace |
| **On expiry** | N/A | Degrades to Standard |
| **Features** | Local + ML + MCP | + Cloud + Web + Mobile |

---

## Files to Create

1. `src/license/mod.rs` - Module exports
2. `src/license/types.rs` - Core types
3. `src/license/manager.rs` - License management
4. `src/license/features.rs` - Feature gating
5. `src/license/crypto.rs` - Key verification
6. Backend API for activation/deactivation
7. LemonSqueezy webhook handler

---

## Definition of Done

- [ ] `convx activate <key>` activates license
- [ ] `convx license` shows status
- [ ] `convx deactivate` removes license
- [ ] Standard features work offline forever
- [ ] Pro features require valid subscription
- [ ] Expired Pro degrades gracefully
- [ ] Device limits enforced
- [ ] LemonSqueezy webhooks generate keys
