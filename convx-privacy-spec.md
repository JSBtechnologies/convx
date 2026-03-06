# convx Privacy Specification

**Version:** 1.0.0  
**Principle:** Your files. Your machine. Your business.

---

## Core Privacy Principles

1. **Local-first** — Files never leave your machine (local version)
2. **No training** — We never use your files to train AI models
3. **No selling** — We never sell or share your data
4. **Minimal collection** — We only collect what's necessary
5. **Transparent** — Clear policies, no buried opt-outs
6. **User control** — Delete your data anytime

---

## Data Handling by Product

### convx Local ($39)

| Data Type | Collected? | Details |
|-----------|------------|---------|
| Files you process | ❌ **Never** | 100% local processing |
| File names | ❌ **Never** | Not transmitted |
| File contents | ❌ **Never** | Not transmitted |
| Conversion history | ❌ **Never** | Stored locally only |
| License key | ✅ Yes | Validate purchase |
| Device ID | ✅ Yes | Enforce device limit |
| App version | ✅ Yes | Compatibility/support |
| OS type | ✅ Yes | Compatibility/support |
| Email | ✅ Yes | License delivery, support |

**Summary:** We verify your license. That's it. Your files never touch our servers.

---

### convx Cloud ($9/mo) — If Built

| Data Type | Collected? | Retention | Details |
|-----------|------------|-----------|---------|
| Files uploaded | ✅ Yes | **24 hours max** | Auto-deleted |
| Processed output | ✅ Yes | **24 hours max** | Auto-deleted |
| File metadata | ✅ Yes | **24 hours max** | Name, size only |
| Account email | ✅ Yes | Until account deleted | Authentication |
| Conversion count | ✅ Yes | Aggregated | Usage limits |
| Payment info | ❌ **Never** | Handled by Stripe/Paddle | We never see card numbers |

**What we DON'T do with cloud files:**
- ❌ Train AI models
- ❌ Share with third parties
- ❌ Analyze contents
- ❌ Keep after processing
- ❌ Use for any purpose other than requested conversion

---

## The Privacy Pledge

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│                  The convx Privacy Pledge                   │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   1. We NEVER train AI models on your files                 │
│                                                             │
│   2. We NEVER sell your data                                │
│                                                             │
│   3. We NEVER share your files with third parties           │
│                                                             │
│   4. Local version: Files NEVER leave your machine          │
│                                                             │
│   5. Cloud version: Files auto-deleted within 24 hours      │
│                                                             │
│   6. We collect ONLY what's necessary for the service       │
│                                                             │
│   7. You can delete ALL your data anytime                   │
│                                                             │
│   Questions? privacy@convx.dev                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation

### Local Version: Zero Telemetry by Default

```rust
// src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Send anonymous crash reports (opt-in only)
    /// Default: false
    pub crash_reports_enabled: bool,
    
    /// Send anonymous usage statistics (opt-in only)
    /// Default: false  
    pub telemetry_enabled: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            crash_reports_enabled: false,  // OFF by default
            telemetry_enabled: false,       // OFF by default
        }
    }
}
```

### Local Version: What License Validation Sends

```rust
// src/license/manager.rs

#[derive(Serialize)]
struct LicenseActivationRequest {
    // What we send
    key: String,           // License key
    device_id: String,     // Machine identifier (hashed)
    app_version: String,   // e.g., "1.0.0"
    os: String,            // e.g., "macos", "windows", "linux"
    
    // What we DON'T send
    // - No file names
    // - No file paths
    // - No conversion history
    // - No usage patterns
    // - No personal files
}
```

### Local Version: Device ID Generation

```rust
// src/license/device.rs

use sha2::{Sha256, Digest};

/// Generate a device ID that:
/// - Is unique to this machine
/// - Is NOT reversible to personal info
/// - Is stable across app reinstalls
pub fn generate_device_id() -> String {
    // Get machine-specific identifiers
    let machine_id = machine_uid::get()
        .unwrap_or_else(|_| "unknown".to_string());
    
    // Hash to prevent reverse engineering
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(b"convx-device-salt"); // Additional entropy
    
    let hash = hasher.finalize();
    
    // Return first 12 hex characters (enough for uniqueness)
    hex::encode(&hash[..6])
}

// Result: "a3f8c2d1e9b4" - not reversible to user identity
```

### Optional Telemetry (Opt-In Only)

```rust
// src/telemetry.rs

/// Telemetry is OFF by default.
/// User must explicitly enable in settings.

#[derive(Serialize)]
struct AnonymousEvent {
    // What we collect IF user opts in
    event: String,           // e.g., "conversion_completed"
    format_from: String,     // e.g., "png"
    format_to: String,       // e.g., "webp"
    duration_ms: u64,        // Processing time
    app_version: String,
    os: String,
    
    // What we NEVER collect, even with telemetry on
    // - File names
    // - File paths
    // - File contents
    // - User identity
}

pub fn track_event(event: AnonymousEvent) {
    let config = Config::load();
    
    // Only send if explicitly enabled
    if !config.privacy.telemetry_enabled {
        return;
    }
    
    // Send to analytics (no user identifier attached)
    analytics::send(event);
}
```

### First Run: Privacy Notice

```rust
// src/main.rs

fn show_first_run_notice() {
    println!();
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│                                                         │");
    println!("│  Welcome to convx!                                      │");
    println!("│                                                         │");
    println!("│  Privacy notice:                                        │");
    println!("│  • Your files are processed 100% locally                │");
    println!("│  • Nothing is uploaded to our servers                   │");
    println!("│  • We only contact our server to validate your license  │");
    println!("│  • Telemetry is OFF by default                          │");
    println!("│                                                         │");
    println!("│  Learn more: https://convx.dev/privacy                  │");
    println!("│                                                         │");
    println!("└─────────────────────────────────────────────────────────┘");
    println!();
}
```

---

## Cloud Version Implementation (If Built)

### File Upload: Encryption

```rust
// convx-cloud/src/upload.rs

use aes_gcm::{Aes256Gcm, Key, Nonce};

pub async fn handle_upload(file: UploadedFile, user_id: &str) -> Result<FileRecord> {
    // 1. Generate unique encryption key for this file
    let file_key = generate_random_key();
    
    // 2. Encrypt file at rest
    let encrypted_data = encrypt_aes256(file.data, &file_key)?;
    
    // 3. Store encrypted file
    let storage_path = format!("uploads/{}/{}", user_id, uuid::new_v4());
    storage::put(&storage_path, encrypted_data).await?;
    
    // 4. Store file key (encrypted with user's key)
    let record = FileRecord {
        id: uuid::new_v4(),
        user_id: user_id.to_string(),
        storage_path,
        file_key_encrypted: encrypt_with_user_key(&file_key, user_id)?,
        original_name: file.name,  // Stored for user convenience only
        size_bytes: file.size,
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(24),  // Auto-delete
        deleted_at: None,
    };
    
    db::files::insert(&record).await?;
    
    Ok(record)
}
```

### Automatic File Deletion

```rust
// convx-cloud/src/jobs/cleanup.rs

/// Runs every hour via cron
pub async fn cleanup_expired_files() -> Result<CleanupReport> {
    let mut report = CleanupReport::default();
    
    // Find files older than 24 hours
    let expired_files = db::files::query()
        .where_expired()
        .where_not_deleted()
        .fetch_all()
        .await?;
    
    for file in expired_files {
        // Delete from storage
        storage::delete(&file.storage_path).await?;
        
        // Mark as deleted in database
        db::files::mark_deleted(file.id).await?;
        
        report.files_deleted += 1;
        report.bytes_freed += file.size_bytes;
    }
    
    // Log for audit (no file contents, just counts)
    info!(
        "Cleanup complete: {} files deleted, {} bytes freed",
        report.files_deleted,
        report.bytes_freed
    );
    
    Ok(report)
}
```

### User-Initiated Immediate Deletion

```rust
// convx-cloud/src/api/files.rs

/// DELETE /api/files/:id
pub async fn delete_file(
    user_id: &str,
    file_id: Uuid,
) -> Result<DeleteResponse> {
    // Verify ownership
    let file = db::files::get(file_id).await?;
    if file.user_id != user_id {
        return Err(Error::Forbidden);
    }
    
    // Delete from storage immediately
    storage::delete(&file.storage_path).await?;
    
    // Mark as deleted
    db::files::mark_deleted(file_id).await?;
    
    Ok(DeleteResponse {
        deleted: true,
        message: "File permanently deleted".to_string(),
    })
}
```

### Delete All User Data

```rust
// convx-cloud/src/api/account.rs

/// DELETE /api/account
/// GDPR Article 17: Right to erasure
pub async fn delete_account(user_id: &str) -> Result<DeleteAccountResponse> {
    // 1. Delete all files from storage
    let files = db::files::get_all_for_user(user_id).await?;
    for file in files {
        storage::delete(&file.storage_path).await.ok(); // Continue on error
    }
    
    // 2. Delete all database records
    db::files::delete_all_for_user(user_id).await?;
    db::conversions::delete_all_for_user(user_id).await?;
    db::settings::delete_for_user(user_id).await?;
    
    // 3. Cancel subscription (if any)
    if let Some(subscription) = db::subscriptions::get_for_user(user_id).await? {
        stripe::subscriptions::cancel(&subscription.stripe_id).await?;
    }
    
    // 4. Delete user account
    db::users::delete(user_id).await?;
    
    // 5. Log deletion (for compliance audit, no personal data)
    audit_log::record(AuditEvent::AccountDeleted {
        timestamp: Utc::now(),
        // Note: user_id is now deleted, this is just a record that deletion occurred
    });
    
    Ok(DeleteAccountResponse {
        deleted: true,
        message: "All data permanently deleted. Sorry to see you go!".to_string(),
    })
}
```

### Export User Data

```rust
// convx-cloud/src/api/account.rs

/// GET /api/account/export
/// GDPR Article 20: Right to data portability
pub async fn export_account_data(user_id: &str) -> Result<DataExport> {
    let user = db::users::get(user_id).await?;
    let files = db::files::get_all_for_user(user_id).await?;
    let conversions = db::conversions::get_all_for_user(user_id).await?;
    let settings = db::settings::get_for_user(user_id).await?;
    
    let export = DataExport {
        exported_at: Utc::now(),
        user: UserExport {
            email: user.email,
            created_at: user.created_at,
        },
        files: files.into_iter().map(|f| FileExport {
            name: f.original_name,
            size_bytes: f.size_bytes,
            created_at: f.created_at,
            // Note: actual file contents available via separate download
        }).collect(),
        conversion_history: conversions.into_iter().map(|c| ConversionExport {
            from_format: c.from_format,
            to_format: c.to_format,
            created_at: c.created_at,
        }).collect(),
        settings: settings,
    };
    
    Ok(export)
}
```

---

## GDPR Compliance

### Requirements Checklist

| GDPR Article | Requirement | Implementation |
|--------------|-------------|----------------|
| Art. 5 | Data minimization | Collect only what's necessary |
| Art. 6 | Lawful basis | Contract (service provision) |
| Art. 7 | Consent | Explicit opt-in for telemetry |
| Art. 12 | Transparent information | Clear privacy policy |
| Art. 13 | Information at collection | First-run notice |
| Art. 15 | Right of access | `/api/account/export` |
| Art. 17 | Right to erasure | `/api/account` DELETE |
| Art. 20 | Data portability | `/api/account/export` |
| Art. 25 | Privacy by design | Local-first, encryption |
| Art. 32 | Security | AES-256, TLS 1.3 |
| Art. 33 | Breach notification | Incident response plan |

### Data Processing Agreement (Cloud)

```markdown
## Data Processing Summary

**Data Controller:** The user
**Data Processor:** convx (your company)

**Purpose of Processing:**
- File format conversion
- Image/audio enhancement
- Text extraction (OCR, transcription)

**Data Processed:**
- Files uploaded by user
- Account information (email)

**Retention Period:**
- Files: 24 hours maximum
- Account: Until user deletes account

**Sub-processors:**
- [Cloud Provider]: Infrastructure
- Stripe/Paddle: Payment processing (no file access)

**Security Measures:**
- Encryption at rest (AES-256)
- Encryption in transit (TLS 1.3)
- Access controls
- Automatic deletion
- Audit logging
```

---

## CCPA Compliance (California)

### Requirements Checklist

| CCPA Right | Implementation |
|------------|----------------|
| Right to know | Privacy policy + data export |
| Right to delete | Account deletion |
| Right to opt-out of sale | We don't sell data (state this clearly) |
| Right to non-discrimination | Same service regardless of privacy choices |

### Required Disclosures

```markdown
## California Privacy Notice

**Categories of Personal Information Collected:**
- Identifiers (email, device ID)
- Commercial information (purchase history)

**Categories of Personal Information Sold:**
- NONE. We do not sell personal information.

**Categories of Personal Information Disclosed for Business Purpose:**
- Payment information to payment processors (Stripe/Paddle)

**Your Rights:**
- Request access to your data
- Request deletion of your data
- Opt-out of sale (N/A - we don't sell)
- Non-discrimination for exercising rights

**Contact:**
privacy@convx.dev
```

---

## Security Measures

### Local Version

| Measure | Implementation |
|---------|----------------|
| License key storage | Encrypted in keychain/credential manager |
| No network for processing | Files never transmitted |
| Code signing | Signed binaries (notarized on macOS) |

### Cloud Version (If Built)

| Measure | Implementation |
|---------|----------------|
| TLS 1.3 | All connections encrypted |
| AES-256-GCM | Files encrypted at rest |
| Per-file keys | Each file has unique encryption key |
| Access logging | All file access logged |
| No logs of file contents | Only metadata logged |
| Secure deletion | Files overwritten, not just unlinked |
| SOC 2 compliance | If pursuing enterprise customers |

---

## Incident Response Plan

### If a Breach Occurs (Cloud Version)

```markdown
## Incident Response Procedure

1. **Identify** - Detect and confirm the breach
2. **Contain** - Stop ongoing unauthorized access
3. **Assess** - Determine what data was affected
4. **Notify** - 
   - Users within 72 hours (GDPR requirement)
   - Authorities if required
5. **Remediate** - Fix the vulnerability
6. **Document** - Record incident and response

## Notification Template

Subject: Security Incident Notification

We detected unauthorized access to our systems on [DATE].

**What happened:** [Brief description]

**What data was affected:** [Specific data types]

**What we're doing:** [Remediation steps]

**What you can do:** [User actions if any]

**Contact:** security@convx.dev
```

---

## Privacy Policy (User-Facing)

```markdown
# convx Privacy Policy

*Last updated: [DATE]*

## The Short Version

- **Local version:** Your files never leave your computer. We only verify your license.
- **Cloud version:** Your files are encrypted, processed, and deleted within 24 hours. We never train AI on your files.
- **Both versions:** We never sell your data. Ever.

---

## Local Version (convx Desktop/CLI)

### What We Collect

| Data | Why | Stored Where |
|------|-----|--------------|
| License key | Verify purchase | Your computer |
| Device ID | Enforce device limit | Our server |
| Email | License delivery | Our server |
| App version, OS | Compatibility | Our server |

### What We DON'T Collect

- Your files (never transmitted)
- File names
- Conversion history
- Usage patterns

### Telemetry

Telemetry is **OFF by default**. If you choose to enable it, we collect anonymous usage statistics (feature used, processing time) with no file information.

---

## Cloud Version (convx Cloud)

### What We Collect

| Data | Why | Retention |
|------|-----|-----------|
| Files you upload | Process them | Deleted in 24 hours |
| Email | Account | Until you delete |
| Conversion count | Usage limits | Aggregated only |

### What We DON'T Do

- ❌ Train AI models on your files
- ❌ Sell or share your files
- ❌ Access file contents (automated processing only)
- ❌ Keep files longer than 24 hours

### Security

- Encrypted in transit (TLS 1.3)
- Encrypted at rest (AES-256)
- Automatic deletion after 24 hours
- You can delete files immediately anytime

---

## Your Rights

### Access Your Data
Request a copy of all data we have about you.

### Delete Your Data
Delete your account and all associated data.

### Export Your Data
Download your data in a portable format.

### Contact
privacy@convx.dev

---

## Changes to This Policy

We'll notify you of significant changes via email or in-app notification.

---

## Contact

**Email:** privacy@convx.dev
**Address:** [Your business address]
```

---

## Marketing: Privacy as Feature

### Website Copy

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│           🔒 Privacy-First File Conversion                  │
│                                                             │
│   Unlike cloud services, convx processes everything         │
│   on YOUR computer.                                         │
│                                                             │
│   • Your files never leave your machine                     │
│   • We never see your content                               │
│   • We never train AI on your data                          │
│   • No upload. No waiting. No privacy concerns.             │
│                                                             │
│   Your files. Your machine. Your business.                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Comparison Table (Marketing)

```
┌─────────────────────────────────────────────────────────────┐
│                    Privacy Comparison                        │
├─────────────────────┬──────────┬────────────┬───────────────┤
│                     │  convx   │ Cloud      │ Adobe         │
│                     │  Local   │ Services   │               │
├─────────────────────┼──────────┼────────────┼───────────────┤
│ Files stay local    │    ✅    │     ❌     │      ❌       │
│ No AI training      │    ✅    │     ❓     │      ❌*      │
│ No data selling     │    ✅    │     ❓     │      ❓       │
│ Works offline       │    ✅    │     ❌     │      ❌       │
│ Clear privacy policy│    ✅    │     ❓     │      ❌       │
└─────────────────────┴──────────┴────────────┴───────────────┘

* Adobe updated terms in 2024 to allow AI training on user content
```

---

## CLI: Privacy Commands

```bash
# Show privacy info
$ convx privacy

Privacy Settings
────────────────
Telemetry: Disabled (default)
Crash reports: Disabled (default)

Data stored locally:
  ~/.convx/license.json (license info)
  ~/.convx/config.toml (preferences)
  ~/.convx/history.db (conversion history - local only)

Data sent to convx servers:
  • License key (on activation)
  • Device ID (on activation)
  • App version (on activation)

Your files are NEVER uploaded.

Learn more: https://convx.dev/privacy

# Enable telemetry (opt-in)
$ convx privacy telemetry on
Telemetry enabled. Anonymous usage stats will be sent.

# Disable telemetry
$ convx privacy telemetry off
Telemetry disabled. No usage data will be sent.

# Clear local history
$ convx privacy clear-history
Conversion history cleared.

# Full data export
$ convx privacy export
Exported to ~/convx-data-export.json
```

---

## Summary

| Aspect | Local | Cloud |
|--------|-------|-------|
| Files leave machine | ❌ Never | ✅ For processing |
| File retention | N/A | 24 hours max |
| AI training | ❌ Never | ❌ Never |
| Data selling | ❌ Never | ❌ Never |
| Telemetry default | Off | Off |
| Delete account | N/A | Yes, instant |
| GDPR compliant | ✅ | ✅ |
| CCPA compliant | ✅ | ✅ |

---

## Definition of Done

- [ ] Privacy policy on website
- [ ] First-run privacy notice
- [ ] `convx privacy` command works
- [ ] Telemetry OFF by default
- [ ] License validation sends minimal data
- [ ] (Cloud) Files auto-delete after 24h
- [ ] (Cloud) User can delete immediately
- [ ] (Cloud) Account deletion works
- [ ] (Cloud) Data export works
