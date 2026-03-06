//! Cryptographic primitives for the ConvX license system.
//!
//! - Ed25519 signature verification (public key embedded at compile time)
//! - Proprietary HMAC-SHA256 fingerprint hashing with version-tagged mixing
//!
//! The public key is embedded in the binary at compile time.
//! The private key lives exclusively on the license server.
//! This module only *verifies* — it never signs.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Embedded Ed25519 public key (32 bytes, hex-encoded at build time).
///
/// Generate a keypair once with:
/// ```text
/// openssl genpkey -algorithm ed25519 -out convx-license-private.pem
/// openssl pkey -in convx-license-private.pem -pubout -outform DER | tail -c 32 | xxd -p
/// ```
///
/// Set `CONVX_LICENSE_PUBKEY` env var at build time for release builds.
/// In dev builds without the env var, a dummy key is used (signature
/// verification will always fail — use `--skip-license` for local dev).
const PUBLIC_KEY_HEX: &str = match option_env!("CONVX_LICENSE_PUBKEY") {
    Some(key) => key,
    // 32 zero-bytes in hex — signature verification will always fail,
    // which is the safe default for dev builds.
    None => "0000000000000000000000000000000000000000000000000000000000000000",
};

// ═══════════════════════════════════════════════════════════════════════════
// Proprietary fingerprint hashing
// ═══════════════════════════════════════════════════════════════════════════
//
// This section implements the proprietary device fingerprint hashing scheme.
// To reproduce a fingerprint hash, an attacker would need to:
//   1. Know the HMAC key (compiled into the binary, not public)
//   2. Know the tier-specific domain separation tags
//   3. Know the order-dependent mixing sequence
//   4. Know the schema version rotation
//
// Together these form a "fingerprint protocol" that is binary-specific.

/// HMAC key for fingerprint computation. This is the proprietary secret
/// compiled into every ConvX binary. Different from the Ed25519 keys.
///
/// This isn't a cryptographic secret protecting data — it's an
/// anti-tampering measure ensuring fingerprints can only be computed
/// by authentic ConvX binaries.
const HMAC_KEY: &[u8] = b"cvx::fp::v2::7e4a9f1c::b3d82e6a::01f7c5d9";

/// Domain separation tags per tier. These ensure that even if two tiers
/// have identical input components, they produce different hashes.
const TIER_DOMAINS: [&[u8]; 3] = [
    b"convx::composite::v2", // tier 0 = composite of tier1+tier2
    b"convx::silicon::v2",   // tier 1 = immutable hardware
    b"convx::system::v2",    // tier 2 = stable system
];

/// Proprietary tiered HMAC hash.
///
/// Takes a tier number (0-2) and an array of component strings, and
/// produces a deterministic hex-encoded hash using HMAC-SHA256 with
/// domain separation and order-dependent mixing.
///
/// The mixing process:
/// 1. Initialize HMAC-SHA256 with the compiled-in key
/// 2. Feed the domain separation tag for this tier
/// 3. Feed each component with a position-dependent separator
/// 4. Apply a final round of mixing with the tier number
///
/// This is intentionally more complex than SHA256(concat(parts, salt))
/// to make reproduction harder without access to this source.
pub(crate) fn proprietary_tier_hash(tier: u8, components: &[&str]) -> String {
    // Round 1: HMAC-SHA256 with domain separation
    let mut mac = HmacSha256::new_from_slice(HMAC_KEY).expect("HMAC accepts any key length");

    // Domain tag
    let domain = TIER_DOMAINS.get(tier as usize).unwrap_or(&TIER_DOMAINS[0]);
    mac.update(domain);
    mac.update(&[0xFF]); // separator

    // Feed components with position-dependent separators.
    // The separator includes the component index XOR'd with the tier number,
    // making the hash depend on the exact ordering.
    for (i, component) in components.iter().enumerate() {
        let pos_tag = (i as u8) ^ tier ^ 0xA5; // position tag with constant mixing
        mac.update(&[pos_tag]);
        mac.update(component.as_bytes());
        mac.update(&[0x00]); // null terminator per component
    }

    // Component count — changing the number of components changes the hash
    // even if the extra components are empty strings
    mac.update(&[components.len() as u8]);

    let round1 = mac.finalize().into_bytes();

    // Round 2: Mix round1 result with a SHA-256 of the tier+key.
    // This adds a layer that depends on both the HMAC output and
    // a hash of the key material, making it harder to work backwards.
    let mut round2 = Sha256::new();
    round2.update(round1);
    round2.update([tier]);
    round2.update(HMAC_KEY);
    round2.update([0xDE, 0xAD]); // magic bytes

    hex::encode(round2.finalize())
}

// ═══════════════════════════════════════════════════════════════════════════
// Ed25519 signature verification
// ═══════════════════════════════════════════════════════════════════════════

use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH};

pub(crate) fn verifying_key() -> Result<VerifyingKey, CryptoError> {
    let bytes = hex::decode(PUBLIC_KEY_HEX).map_err(|_| CryptoError::InvalidPublicKey)?;
    if bytes.len() != PUBLIC_KEY_LENGTH {
        return Err(CryptoError::InvalidPublicKey);
    }
    let mut key_bytes = [0u8; PUBLIC_KEY_LENGTH];
    key_bytes.copy_from_slice(&bytes);
    VerifyingKey::from_bytes(&key_bytes).map_err(|_| CryptoError::InvalidPublicKey)
}

/// Verify an Ed25519 signature over `payload`.
pub(crate) fn verify_signature(payload: &[u8], signature_b64: &str) -> Result<(), CryptoError> {
    use ed25519_dalek::Verifier;

    let sig_bytes = base64_decode(signature_b64).map_err(|_| CryptoError::InvalidSignature)?;

    if sig_bytes.len() != 64 {
        return Err(CryptoError::InvalidSignature);
    }
    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(&sig_bytes);

    let signature = Signature::from_bytes(&sig_array);
    let key = verifying_key()?;

    key.verify(payload, &signature)
        .map_err(|_| CryptoError::SignatureMismatch)
}

/// SHA-256 helper used for signature payload canonicalization.
pub(crate) fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|_| ())
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid embedded public key")]
    InvalidPublicKey,
    #[error("Invalid signature encoding")]
    InvalidSignature,
    #[error("Signature verification failed — license file may be tampered")]
    SignatureMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_deterministic() {
        let a = sha256_hex(b"hello");
        let b = sha256_hex(b"hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn tier_hash_deterministic() {
        let h1 = proprietary_tier_hash(1, &["cpu-id", "platform-uuid"]);
        let h2 = proprietary_tier_hash(1, &["cpu-id", "platform-uuid"]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn tier_hash_order_sensitive() {
        let h1 = proprietary_tier_hash(1, &["cpu-id", "platform-uuid"]);
        let h2 = proprietary_tier_hash(1, &["platform-uuid", "cpu-id"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn tier_hash_tier_sensitive() {
        // Same components, different tier number → different hash
        let h1 = proprietary_tier_hash(1, &["test-component"]);
        let h2 = proprietary_tier_hash(2, &["test-component"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn tier_hash_component_count_matters() {
        // ["a", ""] vs ["a"] should differ even though the content is similar
        let h1 = proprietary_tier_hash(1, &["a", ""]);
        let h2 = proprietary_tier_hash(1, &["a"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn tier_hash_output_is_hex_sha256() {
        let h = proprietary_tier_hash(0, &["test"]);
        assert_eq!(h.len(), 64); // hex-encoded SHA-256
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
