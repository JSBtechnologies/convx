/**
 * Generate an Ed25519 keypair for the ConvX license system.
 *
 * Usage:
 *   npx tsx scripts/generate-keypair.ts
 *
 * Output:
 *   - Private key hex (store as Worker secret: ED25519_PRIVATE_KEY)
 *   - Public key hex (set as build env: CONVX_LICENSE_PUBKEY)
 */

import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// @noble/ed25519 v2 requires sha512
ed.etc.sha512Sync = sha512;

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

const privateKey = ed.utils.randomPrivateKey();
const publicKey = ed.getPublicKey(privateKey);

console.log('═══════════════════════════════════════════════════');
console.log('ConvX License Keypair Generator');
console.log('═══════════════════════════════════════════════════');
console.log();
console.log('PRIVATE KEY (hex) — store as Cloudflare Worker secret:');
console.log(`  wrangler secret put ED25519_PRIVATE_KEY`);
console.log(`  Value: ${bytesToHex(privateKey)}`);
console.log();
console.log('PUBLIC KEY (hex) — set as build-time env for convx-core:');
console.log(`  CONVX_LICENSE_PUBKEY=${bytesToHex(publicKey)}`);
console.log();
console.log('To build convx-core with this public key:');
console.log(`  CONVX_LICENSE_PUBKEY=${bytesToHex(publicKey)} cargo build --release -p convx-core`);
console.log();
console.log('IMPORTANT: Save both values securely. The private key cannot be recovered.');
console.log('═══════════════════════════════════════════════════');
