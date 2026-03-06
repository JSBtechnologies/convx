import type { SupabaseClient } from '@supabase/supabase-js';

// ─── Environment bindings ────────────────────────────────────────────────

export interface Env {
  SUPABASE_URL: string;
  SUPABASE_SERVICE_KEY: string;
  ED25519_PRIVATE_KEY: string;        // hex-encoded 32-byte seed
  ADMIN_SECRET: string;               // bearer token for admin routes
  RECHECK_DAYS: string;               // default "30"
  LEMONSQUEEZY_WEBHOOK_SECRET: string; // HMAC signing secret
  RESEND_API_KEY: string;             // Resend email API key
  EMAIL_FROM: string;                 // e.g. "ConvX <license@convx.dev>"
  TURNSTILE_SECRET_KEY: string;       // Cloudflare Turnstile secret key
}

// ─── Supabase client alias ───────────────────────────────────────────────

export type Supabase = SupabaseClient;

// ─── Device fingerprint (matches Rust DeviceFingerprint) ─────────────────

export interface DeviceFingerprint {
  device_id: string;
  device_name: string;
  tier1_hash: string;
  tier2_hash: string;
  platform: string;
  schema_version: number;
  collected_at: string;
}

// ─── Request bodies ──────────────────────────────────────────────────────

export interface ActivateRequest {
  key: string;
  device: DeviceFingerprint;
}

export interface ValidateRequest {
  key: string;
  device_id: string;
  tier1_hash: string;
  tier2_hash: string;
}

export interface DeactivateRequest {
  key: string;
  device_id: string;
}

export interface TransferRequest {
  key: string;
  new_device: DeviceFingerprint;
}

export interface AdminGenerateRequest {
  count?: number;
  tier?: string;
  email?: string;
}

// ─── Response bodies ─────────────────────────────────────────────────────

export interface ActivateResponse {
  key: string;
  device_id: string;
  activated_at: string;
  recheck_after: string;
  signature: string;
}

export interface ValidateResponse {
  valid: boolean;
  revoked: boolean;
  recheck_after: string;
  signature: string;
  active_device_name: string | null;
}

export interface DeactivateResponse {
  deactivated: boolean;
}

export interface TransferResponse {
  transferred: boolean;
  activated_at: string;
  recheck_after: string;
  signature: string;
}

// ─── LemonSqueezy webhook types ──────────────────────────────────────────

export interface LemonSqueezyWebhookPayload {
  meta: {
    event_name: string;
    custom_data?: Record<string, unknown>;
  };
  data: {
    type: string;
    id: string;
    attributes: LemonSqueezyOrderAttributes;
  };
}

export interface LemonSqueezyOrderAttributes {
  store_id: number;
  customer_id: number;
  identifier: string;
  order_number: number;
  user_name: string;
  user_email: string;
  currency: string;
  subtotal: number;
  discount_total: number;
  tax: number;
  total: number;
  subtotal_usd: number;
  discount_total_usd: number;
  tax_usd: number;
  total_usd: number;
  status: string;
  refunded: boolean | null;
  refunded_at: string | null;
  first_order_item: {
    order_id: number;
    product_id: number;
    variant_id: number;
    product_name: string;
  } | null;
  urls: {
    receipt: string;
  };
  created_at: string;
  updated_at: string;
  test_mode: boolean;
}

// ─── Discount types ─────────────────────────────────────────────────────

export interface ValidateDiscountRequest {
  code: string;
}

export interface ValidateDiscountResponse {
  valid: boolean;
  discount_cents: number;
  discount_formatted: string;
  original_price_cents: number;
  final_price_cents: number;
  product: string;
}

export interface RedeemDiscountRequest {
  code: string;
  email: string;
}

// ─── Download types ─────────────────────────────────────────────────────

export interface DownloadTokenRequest {
  key: string;
  turnstile_token?: string;
}

export type DownloadPlatform = 'macos' | 'windows' | 'linux';

export interface DownloadLink {
  platform: DownloadPlatform;
  url: string;
  label: string;
  filename: string;
}

export interface DownloadTokenResponse {
  downloads: DownloadLink[];
}

// ─── Organization types ─────────────────────────────────────────────────

export interface OrgRow {
  id: string;
  name: string;
  slug: string;
  billing_email: string;
  plan: string;
  max_seats: number;
  settings: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface OrgMemberRow {
  id: string;
  org_id: string;
  user_email: string;
  role: string;
  auth_user_id: string | null;
  invited_at: string;
  accepted_at: string | null;
}

export interface OrgLicenseRow {
  id: string;
  org_id: string;
  license_key_id: number;
  assigned_to: string | null;
  created_at: string;
}

export interface AuditLogRow {
  id: string;
  org_id: string | null;
  license_key_id: number | null;
  user_email: string | null;
  action: string;
  metadata: Record<string, unknown>;
  ip_address: string | null;
  created_at: string;
}

export interface CreateOrgRequest {
  name: string;
  slug: string;
  billing_email: string;
  plan?: string;
  max_seats?: number;
}

export interface OrgGenerateKeysRequest {
  count?: number;
  tier?: string;
}

export interface OrgRevokeRequest {
  key_id: number;
  device_id: string;
}

export interface OrgInviteMemberRequest {
  email: string;
  role?: string;
}

export interface OrgSettingsUpdate {
  [key: string]: unknown;
}

// ─── DB row types (Postgres: booleans are real booleans) ─────────────────

export interface LicenseKeyRow {
  id: number;
  key: string;
  email: string | null;
  tier: string;
  max_devices: number;
  revoked: boolean;
  created_at: string;
}

export interface ActivationRow {
  id: number;
  key_id: number;
  device_id: string;
  device_name: string;
  tier1_hash: string;
  tier2_hash: string;
  platform: string;
  schema_version: number;
  activated_at: string;
  recheck_after: string;
  deactivated: boolean;
}

export interface OrderRow {
  id: number;
  ls_order_id: string;
  ls_customer_id: string | null;
  ls_order_number: number | null;
  user_email: string;
  user_name: string | null;
  product_name: string | null;
  currency: string;
  total_usd: number;
  status: string;
  license_key_id: number | null;
  discount_code_id: number | null;
  email_sent: boolean;
  email_sent_at: string | null;
  raw_payload: Record<string, unknown> | null;
  created_at: string;
}

export interface DiscountCodeRow {
  id: number;
  code: string;
  order_id: number | null;
  discount_cents: number;
  redeemed: boolean;
  redeemed_at: string | null;
  redeemed_by: string | null;
  expires_at: string | null;
  created_at: string;
}
