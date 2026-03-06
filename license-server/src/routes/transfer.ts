import { buildSignaturePayload, daysFromNow, rfc3339, signPayload } from '../crypto';
import { deactivateAllDevices, findKeyByValue, findOrgLicenseByKeyId, insertActivation, insertAuditLog } from '../db';
import type { Env, Supabase, TransferRequest, TransferResponse } from '../types';

export async function handleTransfer(body: TransferRequest, env: Env, supabase: Supabase): Promise<Response> {
  const { key: rawKey, new_device } = body;

  if (!rawKey || !new_device?.device_id) {
    return Response.json({ error: 'Missing key or new_device' }, { status: 400 });
  }

  const key = rawKey.trim().toUpperCase();
  const licenseKey = await findKeyByValue(supabase, key);
  if (!licenseKey) {
    return Response.json({ error: 'Invalid license key' }, { status: 404 });
  }

  if (licenseKey.revoked) {
    return Response.json({ error: 'This license key has been revoked' }, { status: 403 });
  }

  // Deactivate all existing devices for this key
  await deactivateAllDevices(supabase, licenseKey.id);

  // Activate on the new device
  const recheckDays = parseInt(env.RECHECK_DAYS || '30', 10);
  const activatedAt = rfc3339(new Date());
  const recheckAfter = rfc3339(daysFromNow(recheckDays));

  await insertActivation(supabase, licenseKey.id, new_device, activatedAt, recheckAfter);

  // Audit log for org keys
  const orgLicense = await findOrgLicenseByKeyId(supabase, licenseKey.id);
  if (orgLicense) {
    await insertAuditLog(supabase, orgLicense.org_id, licenseKey.id, licenseKey.email, 'transfer', {
      device_id: new_device.device_id,
      device_name: new_device.device_name,
      platform: new_device.platform,
    });
  }

  const payload = buildSignaturePayload(
    key,
    new_device.device_id,
    new_device.schema_version,
    activatedAt,
    recheckAfter,
  );
  const signature = signPayload(payload, env.ED25519_PRIVATE_KEY);

  const response: TransferResponse = {
    transferred: true,
    activated_at: activatedAt,
    recheck_after: recheckAfter,
    signature,
  };

  return Response.json(response);
}
