import { buildSignaturePayload, daysFromNow, rfc3339, signPayload } from '../crypto';
import { findActiveActivation, findKeyByValue, updateRecheckAfter } from '../db';
import type { Env, Supabase, ValidateRequest, ValidateResponse } from '../types';

export async function handleValidate(body: ValidateRequest, env: Env, supabase: Supabase): Promise<Response> {
  const { key: rawKey, device_id, tier1_hash, tier2_hash } = body;

  if (!rawKey || !device_id) {
    return Response.json({ error: 'Missing key or device_id' }, { status: 400 });
  }

  const key = rawKey.trim().toUpperCase();
  const licenseKey = await findKeyByValue(supabase, key);
  if (!licenseKey) {
    return Response.json({ error: 'Invalid license key' }, { status: 404 });
  }

  // Check revocation
  if (licenseKey.revoked) {
    const response: ValidateResponse = {
      valid: false,
      revoked: true,
      recheck_after: rfc3339(new Date()),
      signature: '',
      active_device_name: null,
    };
    return Response.json(response);
  }

  // Find active activation for this device
  const activation = await findActiveActivation(supabase, licenseKey.id, device_id);
  if (!activation) {
    const response: ValidateResponse = {
      valid: false,
      revoked: false,
      recheck_after: rfc3339(new Date()),
      signature: '',
      active_device_name: null,
    };
    return Response.json(response);
  }

  // Log tier2 drift (tier1 must match, tier2 drift is acceptable)
  if (tier1_hash && tier1_hash !== activation.tier1_hash) {
    // Tier 1 mismatch — different hardware entirely
    const response: ValidateResponse = {
      valid: false,
      revoked: false,
      recheck_after: rfc3339(new Date()),
      signature: '',
      active_device_name: activation.device_name,
    };
    return Response.json(response);
  }

  if (tier2_hash && tier2_hash !== activation.tier2_hash) {
    // Tier 2 drifted — OS reinstall or minor HW change, acceptable
    console.log(`Tier 2 drift detected for key=${key} device=${device_id}`);
  }

  // Refresh recheck window
  const recheckDays = parseInt(env.RECHECK_DAYS || '30', 10);
  const recheckAfter = rfc3339(daysFromNow(recheckDays));

  await updateRecheckAfter(supabase, licenseKey.id, device_id, recheckAfter);

  const payload = buildSignaturePayload(
    key,
    device_id,
    activation.schema_version,
    activation.activated_at,
    recheckAfter,
  );
  const signature = signPayload(payload, env.ED25519_PRIVATE_KEY);

  const response: ValidateResponse = {
    valid: true,
    revoked: false,
    recheck_after: recheckAfter,
    signature,
    active_device_name: null,
  };

  return Response.json(response);
}
