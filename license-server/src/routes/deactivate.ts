import { deactivateDevice, findKeyByValue, findOrgLicenseByKeyId, insertAuditLog } from '../db';
import type { DeactivateRequest, DeactivateResponse, Supabase } from '../types';

export async function handleDeactivate(body: DeactivateRequest, supabase: Supabase): Promise<Response> {
  const { key: rawKey, device_id } = body;

  if (!rawKey || !device_id) {
    return Response.json({ error: 'Missing key or device_id' }, { status: 400 });
  }

  const key = rawKey.trim().toUpperCase();
  const licenseKey = await findKeyByValue(supabase, key);
  if (!licenseKey) {
    return Response.json({ error: 'Invalid license key' }, { status: 404 });
  }

  const deactivated = await deactivateDevice(supabase, licenseKey.id, device_id);

  // Audit log for org keys
  if (deactivated) {
    const orgLicense = await findOrgLicenseByKeyId(supabase, licenseKey.id);
    if (orgLicense) {
      await insertAuditLog(supabase, orgLicense.org_id, licenseKey.id, licenseKey.email, 'deactivate', {
        device_id,
      });
    }
  }

  const response: DeactivateResponse = { deactivated };
  return Response.json(response);
}
