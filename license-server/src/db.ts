import type {
  ActivationRow,
  AuditLogRow,
  DeviceFingerprint,
  DiscountCodeRow,
  LicenseKeyRow,
  OrderRow,
  OrgLicenseRow,
  OrgMemberRow,
  OrgRow,
  Supabase,
} from './types';

// ─── License key operations ─────────────────────────────────────────────

export async function findKeyByValue(supabase: Supabase, key: string): Promise<LicenseKeyRow | null> {
  const { data, error } = await supabase
    .from('license_keys')
    .select('*')
    .eq('key', key)
    .single();
  // PGRST116 = "not found" (single row expected but none returned)
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findKeyByValue failed: ${error.message}`);
  }
  if (error) return null;
  return data as LicenseKeyRow;
}

export async function findActiveActivation(
  supabase: Supabase,
  keyId: number,
  deviceId: string,
): Promise<ActivationRow | null> {
  const { data, error } = await supabase
    .from('activations')
    .select('*')
    .eq('key_id', keyId)
    .eq('device_id', deviceId)
    .eq('deactivated', false)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findActiveActivation failed: ${error.message}`);
  }
  if (error) return null;
  return data as ActivationRow;
}

export async function findAnyActiveActivation(
  supabase: Supabase,
  keyId: number,
): Promise<ActivationRow | null> {
  const { data, error } = await supabase
    .from('activations')
    .select('*')
    .eq('key_id', keyId)
    .eq('deactivated', false)
    .limit(1)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findAnyActiveActivation failed: ${error.message}`);
  }
  if (error) return null;
  return data as ActivationRow;
}

export async function countActiveActivations(supabase: Supabase, keyId: number): Promise<number> {
  const { count, error } = await supabase
    .from('activations')
    .select('*', { count: 'exact', head: true })
    .eq('key_id', keyId)
    .eq('deactivated', false);
  if (error) return 0;
  return count ?? 0;
}

export async function insertActivation(
  supabase: Supabase,
  keyId: number,
  device: DeviceFingerprint,
  activatedAt: string,
  recheckAfter: string,
): Promise<void> {
  // Use upsert to handle re-activation after deactivation
  // (UNIQUE constraint on key_id + device_id would block a plain insert)
  const { error } = await supabase
    .from('activations')
    .upsert({
      key_id: keyId,
      device_id: device.device_id,
      device_name: device.device_name,
      tier1_hash: device.tier1_hash,
      tier2_hash: device.tier2_hash,
      platform: device.platform,
      schema_version: device.schema_version,
      activated_at: activatedAt,
      recheck_after: recheckAfter,
      deactivated: false,
    }, { onConflict: 'key_id,device_id' });
  if (error) throw new Error(`insertActivation failed: ${error.message}`);
}

export async function deactivateDevice(
  supabase: Supabase,
  keyId: number,
  deviceId: string,
): Promise<boolean> {
  const { data, error } = await supabase
    .from('activations')
    .update({ deactivated: true })
    .eq('key_id', keyId)
    .eq('device_id', deviceId)
    .eq('deactivated', false)
    .select('id');
  if (error) return false;
  return (data?.length ?? 0) > 0;
}

export async function deactivateAllDevices(supabase: Supabase, keyId: number): Promise<void> {
  const { error } = await supabase
    .from('activations')
    .update({ deactivated: true })
    .eq('key_id', keyId)
    .eq('deactivated', false);
  if (error) throw new Error(`deactivateAllDevices failed: ${error.message}`);
}

export async function updateRecheckAfter(
  supabase: Supabase,
  keyId: number,
  deviceId: string,
  recheckAfter: string,
): Promise<void> {
  const { error } = await supabase
    .from('activations')
    .update({ recheck_after: recheckAfter })
    .eq('key_id', keyId)
    .eq('device_id', deviceId)
    .eq('deactivated', false);
  if (error) throw new Error(`updateRecheckAfter failed: ${error.message}`);
}

export async function insertLicenseKey(
  supabase: Supabase,
  key: string,
  tier: string,
  email: string | null,
): Promise<void> {
  const { error } = await supabase
    .from('license_keys')
    .insert({ key, tier, email });
  if (error) throw new Error(`insertLicenseKey failed: ${error.message}`);
}

export async function insertLicenseKeyReturningId(
  supabase: Supabase,
  key: string,
  tier: string,
  email: string | null,
): Promise<number> {
  const { data, error } = await supabase
    .from('license_keys')
    .insert({ key, tier, email })
    .select('id')
    .single();
  if (error || !data) throw new Error(`insertLicenseKeyReturningId failed: ${error?.message}`);
  return data.id;
}

// ─── Order operations ───────────────────────────────────────────────────

export async function findOrderByLsId(
  supabase: Supabase,
  lsOrderId: string,
): Promise<OrderRow | null> {
  const { data, error } = await supabase
    .from('orders')
    .select('*')
    .eq('ls_order_id', lsOrderId)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findOrderByLsId failed: ${error.message}`);
  }
  if (error) return null;
  return data as OrderRow;
}

export async function insertOrder(
  supabase: Supabase,
  lsOrderId: string,
  lsCustomerId: string | null,
  lsOrderNumber: number | null,
  userEmail: string,
  userName: string | null,
  productName: string | null,
  currency: string,
  totalUsd: number,
  status: string,
  licenseKeyId: number,
  rawPayload: string | null,
): Promise<number> {
  const { data, error } = await supabase
    .from('orders')
    .insert({
      ls_order_id: lsOrderId,
      ls_customer_id: lsCustomerId,
      ls_order_number: lsOrderNumber,
      user_email: userEmail,
      user_name: userName,
      product_name: productName,
      currency,
      total_usd: totalUsd,
      status,
      license_key_id: licenseKeyId,
      raw_payload: rawPayload ? JSON.parse(rawPayload) : null,
    })
    .select('id')
    .single();
  if (error || !data) throw new Error(`insertOrder failed: ${error?.message}`);
  return data.id;
}

export async function updateOrderDiscountCode(
  supabase: Supabase,
  orderId: number,
  discountCodeId: number,
): Promise<void> {
  const { error } = await supabase
    .from('orders')
    .update({ discount_code_id: discountCodeId })
    .eq('id', orderId);
  if (error) throw new Error(`updateOrderDiscountCode failed: ${error.message}`);
}

export async function markOrderEmailSent(
  supabase: Supabase,
  orderId: number,
): Promise<void> {
  const { error } = await supabase
    .from('orders')
    .update({ email_sent: true, email_sent_at: new Date().toISOString() })
    .eq('id', orderId);
  if (error) throw new Error(`markOrderEmailSent failed: ${error.message}`);
}

// ─── Discount code operations ───────────────────────────────────────────

export async function insertDiscountCode(
  supabase: Supabase,
  code: string,
  orderId: number,
  discountCents: number,
  expiresAt: string | null,
): Promise<number> {
  const { data, error } = await supabase
    .from('discount_codes')
    .insert({
      code,
      order_id: orderId,
      discount_cents: discountCents,
      expires_at: expiresAt,
    })
    .select('id')
    .single();
  if (error || !data) throw new Error(`insertDiscountCode failed: ${error?.message}`);
  return data.id;
}

export async function findDiscountByCode(
  supabase: Supabase,
  code: string,
): Promise<DiscountCodeRow | null> {
  const { data, error } = await supabase
    .from('discount_codes')
    .select('*')
    .eq('code', code)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findDiscountByCode failed: ${error.message}`);
  }
  if (error) return null;
  return data as DiscountCodeRow;
}

export async function redeemDiscountCode(
  supabase: Supabase,
  code: string,
  email: string,
): Promise<boolean> {
  const { data, error } = await supabase
    .from('discount_codes')
    .update({
      redeemed: true,
      redeemed_at: new Date().toISOString(),
      redeemed_by: email,
    })
    .eq('code', code)
    .eq('redeemed', false)
    .select('id');
  if (error) return false;
  return (data?.length ?? 0) > 0;
}

// ─── Organization operations ────────────────────────────────────────────

export async function createOrg(
  supabase: Supabase,
  name: string,
  slug: string,
  billingEmail: string,
  plan: string = 'team',
  maxSeats: number = 25,
): Promise<OrgRow> {
  const { data, error } = await supabase
    .from('organizations')
    .insert({ name, slug, billing_email: billingEmail, plan, max_seats: maxSeats })
    .select('*')
    .single();
  if (error || !data) throw new Error(`createOrg failed: ${error?.message}`);
  return data as OrgRow;
}

export async function findOrgById(supabase: Supabase, orgId: string): Promise<OrgRow | null> {
  const { data, error } = await supabase
    .from('organizations')
    .select('*')
    .eq('id', orgId)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findOrgById failed: ${error.message}`);
  }
  if (error) return null;
  return data as OrgRow;
}

export async function updateOrgSettings(
  supabase: Supabase,
  orgId: string,
  settings: Record<string, unknown>,
): Promise<OrgRow> {
  // Merge with existing settings to avoid wiping unset keys on partial updates
  const existing = await findOrgById(supabase, orgId);
  const merged = { ...(existing?.settings as Record<string, unknown> ?? {}), ...settings };

  const { data, error } = await supabase
    .from('organizations')
    .update({ settings: merged, updated_at: new Date().toISOString() })
    .eq('id', orgId)
    .select('*')
    .single();
  if (error || !data) throw new Error(`updateOrgSettings failed: ${error?.message}`);
  return data as OrgRow;
}

// ─── Org member operations ──────────────────────────────────────────────

export async function findOrgMember(
  supabase: Supabase,
  orgId: string,
  email: string,
): Promise<OrgMemberRow | null> {
  const { data, error } = await supabase
    .from('org_members')
    .select('*')
    .eq('org_id', orgId)
    .eq('user_email', email)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findOrgMember failed: ${error.message}`);
  }
  if (error) return null;
  return data as OrgMemberRow;
}

export async function listOrgMembers(supabase: Supabase, orgId: string): Promise<OrgMemberRow[]> {
  const { data, error } = await supabase
    .from('org_members')
    .select('*')
    .eq('org_id', orgId)
    .order('invited_at', { ascending: true });
  if (error) return [];
  return data as OrgMemberRow[];
}

export async function insertOrgMember(
  supabase: Supabase,
  orgId: string,
  email: string,
  role: string = 'member',
  authUserId?: string,
): Promise<OrgMemberRow> {
  const { data, error } = await supabase
    .from('org_members')
    .insert({
      org_id: orgId,
      user_email: email,
      role,
      auth_user_id: authUserId ?? null,
    })
    .select('*')
    .single();
  if (error || !data) throw new Error(`insertOrgMember failed: ${error?.message}`);
  return data as OrgMemberRow;
}

export async function findOrgMemberById(
  supabase: Supabase,
  memberId: string,
): Promise<OrgMemberRow | null> {
  const { data, error } = await supabase
    .from('org_members')
    .select('*')
    .eq('id', memberId)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findOrgMemberById failed: ${error.message}`);
  }
  if (error) return null;
  return data as OrgMemberRow;
}

export async function updateOrgMemberRole(
  supabase: Supabase,
  memberId: string,
  role: string,
): Promise<void> {
  const { error } = await supabase
    .from('org_members')
    .update({ role })
    .eq('id', memberId);
  if (error) throw new Error(`updateOrgMemberRole failed: ${error.message}`);
}

export async function removeOrgMember(supabase: Supabase, memberId: string): Promise<void> {
  const { error } = await supabase
    .from('org_members')
    .delete()
    .eq('id', memberId);
  if (error) throw new Error(`removeOrgMember failed: ${error.message}`);
}

export async function acceptOrgInvite(
  supabase: Supabase,
  orgId: string,
  email: string,
  authUserId: string,
): Promise<void> {
  const { error } = await supabase
    .from('org_members')
    .update({ auth_user_id: authUserId, accepted_at: new Date().toISOString() })
    .eq('org_id', orgId)
    .eq('user_email', email);
  if (error) throw new Error(`acceptOrgInvite failed: ${error.message}`);
}

// ─── Org license operations ─────────────────────────────────────────────

export async function findOrgLicenseByKeyId(
  supabase: Supabase,
  licenseKeyId: number,
): Promise<OrgLicenseRow | null> {
  const { data, error } = await supabase
    .from('org_licenses')
    .select('*')
    .eq('license_key_id', licenseKeyId)
    .single();
  if (error && error.code !== 'PGRST116') {
    throw new Error(`findOrgLicenseByKeyId failed: ${error.message}`);
  }
  if (error) return null;
  return data as OrgLicenseRow;
}

export async function listOrgLicenses(supabase: Supabase, orgId: string): Promise<OrgLicenseRow[]> {
  const { data, error } = await supabase
    .from('org_licenses')
    .select('*')
    .eq('org_id', orgId)
    .order('created_at', { ascending: true });
  if (error) return [];
  return data as OrgLicenseRow[];
}

export async function insertOrgLicense(
  supabase: Supabase,
  orgId: string,
  licenseKeyId: number,
  assignedTo?: string,
): Promise<OrgLicenseRow> {
  const { data, error } = await supabase
    .from('org_licenses')
    .insert({
      org_id: orgId,
      license_key_id: licenseKeyId,
      assigned_to: assignedTo ?? null,
    })
    .select('*')
    .single();
  if (error || !data) throw new Error(`insertOrgLicense failed: ${error?.message}`);
  return data as OrgLicenseRow;
}

export async function countOrgActiveSeats(supabase: Supabase, orgId: string): Promise<number> {
  const { data: orgLicenses } = await supabase
    .from('org_licenses')
    .select('license_key_id')
    .eq('org_id', orgId);

  if (!orgLicenses?.length) return 0;

  const keyIds = orgLicenses.map((l: { license_key_id: number }) => l.license_key_id);

  const { count, error } = await supabase
    .from('activations')
    .select('*', { count: 'exact', head: true })
    .in('key_id', keyIds)
    .eq('deactivated', false);

  if (error) return 0;
  return count ?? 0;
}

export async function listOrgActiveSeats(
  supabase: Supabase,
  orgId: string,
): Promise<ActivationRow[]> {
  const { data: orgLicenses } = await supabase
    .from('org_licenses')
    .select('license_key_id')
    .eq('org_id', orgId);

  if (!orgLicenses?.length) return [];

  const keyIds = orgLicenses.map((l: { license_key_id: number }) => l.license_key_id);

  const { data, error } = await supabase
    .from('activations')
    .select('*')
    .in('key_id', keyIds)
    .eq('deactivated', false)
    .order('activated_at', { ascending: false });

  if (error) return [];
  return data as ActivationRow[];
}

// ─── Audit log operations ───────────────────────────────────────────────

export async function insertAuditLog(
  supabase: Supabase,
  orgId: string | null,
  licenseKeyId: number | null,
  userEmail: string | null,
  action: string,
  metadata: Record<string, unknown> = {},
  ipAddress?: string,
): Promise<void> {
  const { error } = await supabase
    .from('audit_log')
    .insert({
      org_id: orgId,
      license_key_id: licenseKeyId,
      user_email: userEmail,
      action,
      metadata,
      ip_address: ipAddress ?? null,
    });
  if (error) console.error(`insertAuditLog failed: ${error.message}`);
}

export async function listAuditLog(
  supabase: Supabase,
  orgId: string,
  options?: { action?: string; limit?: number; offset?: number },
): Promise<AuditLogRow[]> {
  let query = supabase
    .from('audit_log')
    .select('*')
    .eq('org_id', orgId)
    .order('created_at', { ascending: false });

  if (options?.action) {
    query = query.eq('action', options.action);
  }
  if (options?.limit) {
    query = query.limit(options.limit);
  }
  if (options?.offset) {
    query = query.range(options.offset, options.offset + (options?.limit ?? 50) - 1);
  }

  const { data, error } = await query;
  if (error) return [];
  return data as AuditLogRow[];
}
