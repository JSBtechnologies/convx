import { buildSignaturePayload, daysFromNow, rfc3339, signPayload } from '../crypto';
import {
  countActiveActivations,
  countOrgActiveSeats,
  findActiveActivation,
  findAnyActiveActivation,
  findKeyByValue,
  findOrgById,
  findOrgLicenseByKeyId,
  insertActivation,
  insertAuditLog,
} from '../db';
import type { ActivateRequest, ActivateResponse, Env, Supabase } from '../types';

export async function handleActivate(body: ActivateRequest, env: Env, supabase: Supabase): Promise<Response> {
  const { key: rawKey, device } = body;

  if (!rawKey || !device?.device_id) {
    return Response.json({ error: 'Missing key or device' }, { status: 400 });
  }

  const key = rawKey.trim().toUpperCase();
  const licenseKey = await findKeyByValue(supabase, key);
  if (!licenseKey) {
    return Response.json({ error: 'Invalid license key' }, { status: 404 });
  }

  if (licenseKey.revoked) {
    return Response.json({ error: 'This license key has been revoked' }, { status: 403 });
  }

  // Check if this exact device is already activated for this key
  const existing = await findActiveActivation(supabase, licenseKey.id, device.device_id);
  if (existing) {
    // Idempotent: return existing activation data with a fresh signature
    const recheckDays = parseInt(env.RECHECK_DAYS || '30', 10);
    const recheckAfter = rfc3339(daysFromNow(recheckDays));

    const payload = buildSignaturePayload(
      key,
      device.device_id,
      device.schema_version,
      existing.activated_at,
      recheckAfter,
    );
    const signature = signPayload(payload, env.ED25519_PRIVATE_KEY);

    const response: ActivateResponse = {
      key,
      device_id: device.device_id,
      activated_at: existing.activated_at,
      recheck_after: recheckAfter,
      signature,
    };

    return Response.json(response);
  }

  // Check if this is an org key — enforce org seat limits
  const orgLicense = await findOrgLicenseByKeyId(supabase, licenseKey.id);
  if (orgLicense) {
    const org = await findOrgById(supabase, orgLicense.org_id);
    if (org) {
      const activeSeats = await countOrgActiveSeats(supabase, org.id);
      if (activeSeats >= org.max_seats) {
        await insertAuditLog(supabase, org.id, licenseKey.id, licenseKey.email, 'seat_limit_hit', {
          device_name: device.device_name,
          active: activeSeats,
          max: org.max_seats,
        });
        return Response.json(
          { error: 'Seat limit reached', active: activeSeats, max: org.max_seats },
          { status: 409 },
        );
      }
    }
  } else {
    // Non-org key: standard per-key device limit
    const activeCount = await countActiveActivations(supabase, licenseKey.id);
    if (activeCount >= licenseKey.max_devices) {
      const otherActivation = await findAnyActiveActivation(supabase, licenseKey.id);
      return Response.json(
        { active_device_name: otherActivation?.device_name ?? 'another device' },
        { status: 409 },
      );
    }
  }

  // Activate on this device
  const recheckDays = parseInt(env.RECHECK_DAYS || '30', 10);
  const now = new Date();
  const activatedAt = rfc3339(now);
  const recheckAfter = rfc3339(daysFromNow(recheckDays));

  await insertActivation(supabase, licenseKey.id, device, activatedAt, recheckAfter);

  // Audit log (for org keys)
  if (orgLicense) {
    await insertAuditLog(supabase, orgLicense.org_id, licenseKey.id, licenseKey.email, 'activate', {
      device_id: device.device_id,
      device_name: device.device_name,
      platform: device.platform,
    });
  }

  const payload = buildSignaturePayload(
    key,
    device.device_id,
    device.schema_version,
    activatedAt,
    recheckAfter,
  );
  const signature = signPayload(payload, env.ED25519_PRIVATE_KEY);

  const response: ActivateResponse = {
    key,
    device_id: device.device_id,
    activated_at: activatedAt,
    recheck_after: recheckAfter,
    signature,
  };

  return Response.json(response);
}
