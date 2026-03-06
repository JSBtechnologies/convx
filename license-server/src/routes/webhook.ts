import {
  createOrg,
  findOrderByLsId,
  insertAuditLog,
  insertDiscountCode,
  insertLicenseKeyReturningId,
  insertOrgLicense,
  insertOrgMember,
  insertOrder,
  markOrderEmailSent,
  updateOrderDiscountCode,
} from '../db';
import { constantTimeEqual } from '../crypto';
import { sendEnterpriseWelcomeEmail, sendLicenseEmail } from '../email';
import { generateConvxKey, generateDiscountCode } from '../keygen';
import type { Env, LemonSqueezyWebhookPayload, Supabase } from '../types';

// Enterprise product IDs from LemonSqueezy
const ENTERPRISE_PRODUCTS: Record<string, { plan: string; seats: number; keyCount: number }> = {
  // These IDs should match your LemonSqueezy product variant IDs
  'team': { plan: 'team', seats: 25, keyCount: 25 },
  'business': { plan: 'business', seats: 100, keyCount: 100 },
  'enterprise': { plan: 'enterprise', seats: 999999, keyCount: 100 },
};

// Product name patterns to detect enterprise purchases
function detectEnterprisePlan(productName: string | null): { plan: string; seats: number; keyCount: number } | null {
  if (!productName) return null;
  const lower = productName.toLowerCase();
  // Check most specific patterns first
  if (lower.includes('business')) return ENTERPRISE_PRODUCTS['business'];
  if (lower.includes('enterprise')) return ENTERPRISE_PRODUCTS['enterprise'];
  if (lower.includes('team plan') || lower.includes('team')) return ENTERPRISE_PRODUCTS['team'];
  return null;
}

export async function handleWebhook(
  rawBody: string,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  // 1. Verify HMAC-SHA256 signature
  const signature = request.headers.get('X-Signature') ?? '';
  if (!signature) {
    return Response.json({ error: 'Missing signature' }, { status: 401 });
  }

  const valid = await verifySignature(rawBody, signature, env.LEMONSQUEEZY_WEBHOOK_SECRET);
  if (!valid) {
    return Response.json({ error: 'Invalid signature' }, { status: 401 });
  }

  // 2. Parse payload
  let payload: LemonSqueezyWebhookPayload;
  try {
    payload = JSON.parse(rawBody);
  } catch {
    return Response.json({ error: 'Invalid JSON' }, { status: 400 });
  }

  // 3. Only process order_created events
  if (payload.meta.event_name !== 'order_created') {
    return Response.json({ received: true, event: payload.meta.event_name });
  }

  const attrs = payload.data.attributes;
  const lsOrderId = payload.data.id;

  // 4. Idempotency — already processed this order?
  const existing = await findOrderByLsId(supabase, lsOrderId);
  if (existing) {
    return Response.json({ received: true, duplicate: true, order_id: existing.id });
  }

  const productName = attrs.first_order_item?.product_name ?? null;
  const enterprisePlan = detectEnterprisePlan(productName);

  if (enterprisePlan) {
    // ── Enterprise purchase flow ──────────────────────────────────────
    return handleEnterpriseOrder(supabase, env, attrs, lsOrderId, rawBody, enterprisePlan);
  }

  // ── Standard (individual) purchase flow ─────────────────────────────

  // 5. Generate license key
  const licenseKey = generateConvxKey();
  const licenseKeyId = await insertLicenseKeyReturningId(
    supabase,
    licenseKey,
    'standard',
    attrs.user_email,
  );

  // 6. Create order record
  const orderId = await insertOrder(
    supabase,
    lsOrderId,
    String(attrs.customer_id),
    attrs.order_number,
    attrs.user_email,
    attrs.user_name,
    productName,
    attrs.currency,
    attrs.total_usd,
    attrs.status,
    licenseKeyId,
    rawBody,
  );

  // 7. Generate discount code (expires in 1 year)
  const discountCode = generateDiscountCode();
  const expiresAt = new Date();
  expiresAt.setFullYear(expiresAt.getFullYear() + 1);

  const discountCodeId = await insertDiscountCode(
    supabase,
    discountCode,
    orderId,
    1400, // $14 off
    expiresAt.toISOString(),
  );

  // 8. Link discount code to order
  await updateOrderDiscountCode(supabase, orderId, discountCodeId);

  // 9. Send email (best-effort — don't fail the webhook)
  try {
    await sendLicenseEmail(env, attrs.user_email, attrs.user_name, licenseKey, discountCode);
    await markOrderEmailSent(supabase, orderId);
  } catch (err) {
    console.error('Failed to send license email:', err);
  }

  return Response.json({
    received: true,
    order_id: orderId,
  });
}

async function handleEnterpriseOrder(
  supabase: Supabase,
  env: Env,
  attrs: LemonSqueezyWebhookPayload['data']['attributes'],
  lsOrderId: string,
  rawBody: string,
  plan: { plan: string; seats: number; keyCount: number },
): Promise<Response> {
  // 1. Create the organization
  const slug = attrs.user_email.split('@')[0].replace(/[^a-zA-Z0-9-]/g, '') + '-' + Date.now().toString(36);
  const org = await createOrg(
    supabase,
    attrs.user_name || attrs.user_email.split('@')[0],
    slug,
    attrs.user_email,
    plan.plan,
    plan.seats,
  );

  // 2. Generate N license keys and link to org
  const keys: string[] = [];
  let firstKeyId: number | null = null;
  for (let i = 0; i < plan.keyCount; i++) {
    const key = generateConvxKey();
    const keyId = await insertLicenseKeyReturningId(supabase, key, plan.plan, attrs.user_email);
    await insertOrgLicense(supabase, org.id, keyId);
    keys.push(key);
    if (i === 0) firstKeyId = keyId;
  }

  // 3. Create admin member for purchaser
  await insertOrgMember(supabase, org.id, attrs.user_email, 'admin');

  // 4. Create order record (linked to first key)
  const orderId = await insertOrder(
    supabase,
    lsOrderId,
    String(attrs.customer_id),
    attrs.order_number,
    attrs.user_email,
    attrs.user_name,
    attrs.first_order_item?.product_name ?? null,
    attrs.currency,
    attrs.total_usd,
    attrs.status,
    firstKeyId!,
    rawBody,
  );

  // 5. Audit log
  await insertAuditLog(supabase, org.id, null, attrs.user_email, 'org_created', {
    plan: plan.plan,
    seats: plan.seats,
    keys_generated: plan.keyCount,
    order_id: lsOrderId,
  });

  // 6. Generate invite token (base64 of org_id:email)
  const inviteToken = btoa(`${org.id}:${attrs.user_email}`);
  const dashboardUrl = `https://enterprise.convx.dev/signup?token=${inviteToken}`;

  // 7. Send enterprise welcome email (best-effort)
  try {
    await sendEnterpriseWelcomeEmail(
      env,
      attrs.user_email,
      attrs.user_name,
      plan.plan,
      plan.seats,
      keys.slice(0, 5), // Send first 5 keys in email
      dashboardUrl,
    );
    await markOrderEmailSent(supabase, orderId);
  } catch (err) {
    console.error('Failed to send enterprise welcome email:', err);
  }

  return Response.json({
    received: true,
    enterprise: true,
    order_id: orderId,
  });
}

async function verifySignature(
  rawBody: string,
  signature: string,
  secret: string,
): Promise<boolean> {
  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const mac = await crypto.subtle.sign('HMAC', key, encoder.encode(rawBody));
  const computedHex = Array.from(new Uint8Array(mac))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');

  return constantTimeEqual(computedHex, signature);
}
