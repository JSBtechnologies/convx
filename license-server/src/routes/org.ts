import {
  countOrgActiveSeats,
  createOrg,
  deactivateDevice,
  findKeyByValue,
  findOrgById,
  findOrgLicenseByKeyId,
  findOrgMemberById,
  insertAuditLog,
  insertLicenseKeyReturningId,
  insertOrgLicense,
  insertOrgMember,
  listAuditLog,
  listOrgActiveSeats,
  listOrgLicenses,
  listOrgMembers,
  removeOrgMember,
  updateOrgMemberRole,
  updateOrgSettings,
} from '../db';
import { constantTimeEqual } from '../crypto';
import { generateConvxKey } from '../keygen';
import type {
  CreateOrgRequest,
  Env,
  OrgGenerateKeysRequest,
  OrgInviteMemberRequest,
  OrgRevokeRequest,
  OrgSettingsUpdate,
  Supabase,
} from '../types';

function requireAdmin(request: Request, env: Env): Response | null {
  const authHeader = request.headers.get('Authorization');
  const expected = `Bearer ${env.ADMIN_SECRET}`;
  if (!authHeader || !constantTimeEqual(authHeader, expected)) {
    return Response.json({ error: 'Unauthorized' }, { status: 401 });
  }
  return null;
}

// POST /v1/org
export async function handleCreateOrg(
  body: CreateOrgRequest,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const { name, slug, billing_email, plan, max_seats } = body;
  if (!name || !slug || !billing_email) {
    return Response.json({ error: 'Missing name, slug, or billing_email' }, { status: 400 });
  }

  const planSeats: Record<string, number> = {
    team: 25,
    business: 100,
    enterprise: 999999,
  };

  const orgPlan = plan ?? 'team';
  const seats = max_seats ?? planSeats[orgPlan] ?? 25;

  const org = await createOrg(supabase, name, slug, billing_email, orgPlan, seats);

  await insertAuditLog(supabase, org.id, null, billing_email, 'org_created', {
    plan: orgPlan,
    max_seats: seats,
  });

  return Response.json(org, { status: 201 });
}

// GET /v1/org/:id/seats
export async function handleGetOrgSeats(
  orgId: string,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const [active, seats] = await Promise.all([
    countOrgActiveSeats(supabase, orgId),
    listOrgActiveSeats(supabase, orgId),
  ]);

  return Response.json({
    active,
    max_seats: org.max_seats,
    seats: seats.map((s) => ({
      device_id: s.device_id,
      device_name: s.device_name,
      platform: s.platform,
      activated_at: s.activated_at,
      key_id: s.key_id,
    })),
  });
}

// POST /v1/org/:id/keys
export async function handleGenerateOrgKeys(
  orgId: string,
  body: OrgGenerateKeysRequest,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const count = Math.min(body.count ?? 1, 100);
  const tier = body.tier ?? 'standard';

  const keys: string[] = [];
  for (let i = 0; i < count; i++) {
    const key = generateConvxKey();
    const keyId = await insertLicenseKeyReturningId(supabase, key, tier, org.billing_email);
    await insertOrgLicense(supabase, orgId, keyId);
    keys.push(key);
  }

  await insertAuditLog(supabase, orgId, null, null, 'keys_generated', {
    count,
    tier,
  });

  return Response.json({ keys, count: keys.length, tier, org_id: orgId });
}

// POST /v1/org/:id/revoke
export async function handleRevokeOrgDevice(
  orgId: string,
  body: OrgRevokeRequest,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const { key_id, device_id } = body;
  if (!key_id || !device_id) {
    return Response.json({ error: 'Missing key_id or device_id' }, { status: 400 });
  }

  // Verify key belongs to this org
  const orgLicense = await findOrgLicenseByKeyId(supabase, key_id);
  if (!orgLicense || orgLicense.org_id !== orgId) {
    return Response.json({ error: 'Key does not belong to this organization' }, { status: 403 });
  }

  const deactivated = await deactivateDevice(supabase, key_id, device_id);

  if (deactivated) {
    await insertAuditLog(supabase, orgId, key_id, null, 'revoke', {
      device_id,
    });
  }

  return Response.json({ deactivated });
}

// GET /v1/org/:id/settings
export async function handleGetOrgSettings(
  orgId: string,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  return Response.json({ settings: org.settings, plan: org.plan, max_seats: org.max_seats });
}

// PUT /v1/org/:id/settings
export async function handleUpdateOrgSettings(
  orgId: string,
  body: OrgSettingsUpdate,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  // Validate settings keys
  const allowedSettingsKeys = [
    'default_quality', 'default_format', 'output_directory',
    'overwrite_existing', 'show_notifications', 'allowed_formats', 'locked',
  ];
  const unknownKeys = Object.keys(body).filter((k) => !allowedSettingsKeys.includes(k));
  if (unknownKeys.length > 0) {
    return Response.json({ error: `Unknown settings keys: ${unknownKeys.join(', ')}` }, { status: 400 });
  }

  const updated = await updateOrgSettings(supabase, orgId, body);

  await insertAuditLog(supabase, orgId, null, null, 'settings_updated', {
    settings: body,
  });

  return Response.json({ settings: updated.settings });
}

// GET /v1/org/:id/audit
export async function handleGetOrgAudit(
  orgId: string,
  url: URL,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const action = url.searchParams.get('action') ?? undefined;
  const limit = parseInt(url.searchParams.get('limit') ?? '50', 10);
  const offset = parseInt(url.searchParams.get('offset') ?? '0', 10);

  const entries = await listAuditLog(supabase, orgId, { action, limit, offset });

  return Response.json({ entries, count: entries.length });
}

// POST /v1/org/:id/members
export async function handleInviteOrgMember(
  orgId: string,
  body: OrgInviteMemberRequest,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const { email, role } = body;
  if (!email) {
    return Response.json({ error: 'Missing email' }, { status: 400 });
  }

  const validRoles = ['admin', 'member'];
  const memberRole = role ?? 'member';
  if (!validRoles.includes(memberRole)) {
    return Response.json({ error: `Invalid role. Must be one of: ${validRoles.join(', ')}` }, { status: 400 });
  }

  const member = await insertOrgMember(supabase, orgId, email, memberRole);

  await insertAuditLog(supabase, orgId, null, email, 'member_invited', {
    role: memberRole,
  });

  return Response.json(member, { status: 201 });
}

// GET /v1/org/:id/members
export async function handleListOrgMembers(
  orgId: string,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const members = await listOrgMembers(supabase, orgId);
  return Response.json({ members });
}

// PUT /v1/org/:id/members — update member role
export async function handleUpdateOrgMemberRole(
  orgId: string,
  body: { member_id: string; role: string },
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const { member_id, role } = body;
  if (!member_id || !role) {
    return Response.json({ error: 'Missing member_id or role' }, { status: 400 });
  }

  const validRoles = ['admin', 'member'];
  if (!validRoles.includes(role)) {
    return Response.json({ error: `Invalid role. Must be one of: ${validRoles.join(', ')}` }, { status: 400 });
  }

  // Verify member belongs to this org
  const member = await findOrgMemberById(supabase, member_id);
  if (!member || member.org_id !== orgId) {
    return Response.json({ error: 'Member does not belong to this organization' }, { status: 403 });
  }

  await updateOrgMemberRole(supabase, member_id, role);

  await insertAuditLog(supabase, orgId, null, null, 'member_role_updated', {
    member_id,
    role,
  });

  return Response.json({ updated: true });
}

// DELETE /v1/org/:id/members
export async function handleRemoveOrgMember(
  orgId: string,
  body: { member_id: string },
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const { member_id } = body;
  if (!member_id) {
    return Response.json({ error: 'Missing member_id' }, { status: 400 });
  }

  // Verify member belongs to this org
  const member = await findOrgMemberById(supabase, member_id);
  if (!member || member.org_id !== orgId) {
    return Response.json({ error: 'Member does not belong to this organization' }, { status: 403 });
  }

  await removeOrgMember(supabase, member_id);

  await insertAuditLog(supabase, orgId, null, null, 'member_removed', {
    member_id,
  });

  return Response.json({ removed: true });
}

// GET /v1/org/:id/licenses
export async function handleListOrgLicenses(
  orgId: string,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  const authErr = requireAdmin(request, env);
  if (authErr) return authErr;

  const org = await findOrgById(supabase, orgId);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  const licenses = await listOrgLicenses(supabase, orgId);
  return Response.json({ licenses });
}

// GET /v1/org/settings-by-key?key=CONVX-XXXX-...
// Resolves a license key to its org and returns enterprise settings.
// Returns 404 if the key doesn't exist or isn't associated with an org.
export async function handleGetOrgSettingsByKey(
  url: URL,
  supabase: Supabase,
): Promise<Response> {
  const rawKey = url.searchParams.get('key');
  if (!rawKey) {
    return Response.json({ error: 'Missing key parameter' }, { status: 400 });
  }

  const key = rawKey.trim().toUpperCase();

  // key → license_keys row
  const keyRow = await findKeyByValue(supabase, key);
  if (!keyRow) {
    return Response.json({ error: 'Key not found' }, { status: 404 });
  }

  // license_keys.id → org_licenses row
  const orgLicense = await findOrgLicenseByKeyId(supabase, keyRow.id);
  if (!orgLicense) {
    return Response.json({ error: 'Key is not associated with an organization' }, { status: 404 });
  }

  // org_licenses.org_id → organization row
  const org = await findOrgById(supabase, orgLicense.org_id);
  if (!org) {
    return Response.json({ error: 'Organization not found' }, { status: 404 });
  }

  return Response.json({ settings: org.settings });
}
