import { constantTimeEqual } from '../crypto';
import { findKeyByValue } from '../db';
import type { DownloadLink, DownloadPlatform, DownloadTokenRequest, Env, Supabase } from '../types';

const DOWNLOAD_BASE = 'https://convx.dev/dl';
const TOKEN_TTL_SECONDS = 300; // 5 minutes

const PLATFORM_FILES: Record<DownloadPlatform, { filename: string; label: string }> = {
  macos: { filename: 'ConvX.pkg', label: 'Download for Mac' },
  windows: { filename: 'ConvX-Setup.exe', label: 'Download for Windows' },
  linux: { filename: 'convx-linux.AppImage', label: 'Download for Linux' },
};

/**
 * Generate signed download URLs for all platforms.
 * Token is an opaque base64url string encoding: expires:platform:keyId:hmac_sha256
 */
export async function handleDownloadToken(
  body: DownloadTokenRequest,
  env: Env,
  supabase: Supabase,
): Promise<Response> {
  const { key, turnstile_token } = body;
  if (!key) {
    return Response.json({ error: 'Missing license key' }, { status: 400 });
  }

  // Verify Turnstile token
  if (env.TURNSTILE_SECRET_KEY) {
    if (!turnstile_token) {
      return Response.json({ error: 'Captcha verification required' }, { status: 400 });
    }
    const turnstileOk = await verifyTurnstile(turnstile_token, env.TURNSTILE_SECRET_KEY);
    if (!turnstileOk) {
      return Response.json({ error: 'Captcha verification failed' }, { status: 403 });
    }
  }

  const normalized = key.trim().toUpperCase();
  const row = await findKeyByValue(supabase, normalized);

  if (!row) {
    return Response.json({ error: 'Invalid license key' }, { status: 404 });
  }

  if (row.revoked) {
    return Response.json({ error: 'License key has been revoked' }, { status: 403 });
  }

  const expires = Math.floor(Date.now() / 1000) + TOKEN_TTL_SECONDS;

  const downloads: DownloadLink[] = [];
  for (const [platform, info] of Object.entries(PLATFORM_FILES)) {
    const token = await createOpaqueToken(expires, platform, row.id, env.LEMONSQUEEZY_WEBHOOK_SECRET);
    downloads.push({
      platform: platform as DownloadPlatform,
      url: `${DOWNLOAD_BASE}/${platform}?s=${token}`,
      label: info.label,
      filename: info.filename,
    });
  }

  return Response.json({ downloads } satisfies { downloads: DownloadLink[] });
}

/**
 * Create an opaque base64url token: base64url(expires:platform:keyId:hmac)
 */
async function createOpaqueToken(
  expires: number,
  platform: string,
  keyId: number,
  secret: string,
): Promise<string> {
  const hmac = await computeHmac(keyId, platform, expires, secret);
  const payload = `${expires}:${platform}:${keyId}:${hmac}`;
  return toBase64Url(payload);
}

/**
 * Parse and verify an opaque token. Returns { valid, platform, expired } or null.
 */
export function parseOpaqueToken(token: string): { expires: number; platform: string; keyId: number; hmac: string } | null {
  try {
    const decoded = fromBase64Url(token);
    const parts = decoded.split(':');
    if (parts.length !== 4) return null;
    const [expiresStr, platform, keyIdStr, hmac] = parts;
    const expires = parseInt(expiresStr, 10);
    const keyId = parseInt(keyIdStr, 10);
    if (isNaN(expires) || isNaN(keyId)) return null;
    if (!['macos', 'windows', 'linux'].includes(platform)) return null;
    return { expires, platform, keyId, hmac };
  } catch {
    return null;
  }
}

export async function verifyOpaqueToken(
  token: string,
  requestPlatform: string,
  secret: string,
): Promise<{ valid: boolean; reason?: string }> {
  const parsed = parseOpaqueToken(token);
  if (!parsed) return { valid: false, reason: 'invalid' };

  if (Math.floor(Date.now() / 1000) > parsed.expires) {
    return { valid: false, reason: 'expired' };
  }

  if (parsed.platform !== requestPlatform) {
    return { valid: false, reason: 'platform_mismatch' };
  }

  const expected = await computeHmac(parsed.keyId, parsed.platform, parsed.expires, secret);
  if (!constantTimeEqual(parsed.hmac, expected)) {
    return { valid: false, reason: 'tampered' };
  }

  return { valid: true };
}

/**
 * Verify endpoint for nginx auth_request (GET).
 * Expects X-Original-URI header containing the original request URI.
 */
export async function handleDownloadVerify(
  request: Request,
  env: Env,
): Promise<Response> {
  const originalUri = request.headers.get('x-original-uri');
  if (!originalUri) {
    return new Response('Forbidden', { status: 403 });
  }

  // Parse /dl/{platform}?s={token} from original URI
  const match = originalUri.match(/^\/dl\/(macos|windows|linux)\?s=([A-Za-z0-9_-]+)$/);
  if (!match) {
    return new Response('Forbidden', { status: 403 });
  }

  const [, platform, token] = match;
  const result = await verifyOpaqueToken(token, platform, env.LEMONSQUEEZY_WEBHOOK_SECRET);

  if (!result.valid) {
    return new Response('Forbidden', { status: 403 });
  }

  return new Response('OK', { status: 200 });
}

async function computeHmac(
  keyId: number,
  platform: string,
  expires: number,
  secret: string,
): Promise<string> {
  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const message = `${keyId}:${platform}:${expires}`;
  const sig = await crypto.subtle.sign('HMAC', key, encoder.encode(message));
  return Array.from(new Uint8Array(sig))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

function toBase64Url(str: string): string {
  return btoa(str).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(str: string): string {
  const padded = str.replace(/-/g, '+').replace(/_/g, '/');
  const pad = padded.length % 4;
  const b64 = pad ? padded + '='.repeat(4 - pad) : padded;
  return atob(b64);
}

async function verifyTurnstile(token: string, secretKey: string): Promise<boolean> {
  const res = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ secret: secretKey, response: token }),
  });
  const data = await res.json() as { success: boolean };
  return data.success;
}
