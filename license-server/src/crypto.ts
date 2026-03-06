import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { sha256 } from '@noble/hashes/sha256';
import { bytesToHex } from '@noble/hashes/utils';

// @noble/ed25519 v2 requires setting the sha512 hash function
ed.etc.sha512Sync = sha512;

/**
 * Build the canonical signature payload matching the Rust client.
 *
 * From keyfile.rs lines 140-150:
 *   canonical = "{key}::{device_id}::{schema_version}::{activated_at}::{recheck_after}"
 *   payload = sha256_hex(canonical).as_bytes()  // UTF-8 bytes of the hex string
 */
export function buildSignaturePayload(
  key: string,
  deviceId: string,
  schemaVersion: number,
  activatedAt: string,
  recheckAfter: string,
): Uint8Array {
  const canonical = `${key}::${deviceId}::${schemaVersion}::${activatedAt}::${recheckAfter}`;
  const hash = sha256(new TextEncoder().encode(canonical));
  const hexStr = bytesToHex(hash);
  // The Rust client does sha256_hex(...).into_bytes() which gives UTF-8 bytes of the hex string
  return new TextEncoder().encode(hexStr);
}

/**
 * Sign a payload with Ed25519 using the private key seed.
 * Returns the signature as a base64 string.
 */
export function signPayload(payload: Uint8Array, privateKeyHex: string): string {
  const privateKey = hexToBytes(privateKeyHex);
  const signature = ed.sign(payload, privateKey);
  return bytesToBase64(signature);
}

/**
 * Generate an RFC 3339 timestamp matching chrono's to_rfc3339() format.
 * Chrono uses "+00:00" suffix, not "Z".
 */
export function rfc3339(date: Date): string {
  return date.toISOString().replace('Z', '+00:00');
}

/**
 * Get a date N days from now.
 */
export function daysFromNow(days: number): Date {
  const d = new Date();
  d.setUTCDate(d.getUTCDate() + days);
  return d;
}

/**
 * Constant-time string comparison that does not leak length information.
 */
export function constantTimeEqual(a: string, b: string): boolean {
  const len = Math.max(a.length, b.length);
  let result = a.length ^ b.length;
  for (let i = 0; i < len; i++) {
    result |= (a.charCodeAt(i) || 0) ^ (b.charCodeAt(i) || 0);
  }
  return result === 0;
}

// ─── Helpers ─────────────────────────────────────────────────────────────

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}
