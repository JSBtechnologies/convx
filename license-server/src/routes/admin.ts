import { constantTimeEqual } from '../crypto';
import { insertLicenseKey } from '../db';
import { generateConvxKey } from '../keygen';
import type { AdminGenerateRequest, Env, Supabase } from '../types';

export async function handleAdminGenerate(
  body: AdminGenerateRequest,
  env: Env,
  request: Request,
  supabase: Supabase,
): Promise<Response> {
  // Auth check (constant-time comparison)
  const authHeader = request.headers.get('Authorization');
  const expected = `Bearer ${env.ADMIN_SECRET}`;
  if (!authHeader || !constantTimeEqual(authHeader, expected)) {
    return Response.json({ error: 'Unauthorized' }, { status: 401 });
  }

  const count = Math.min(body.count ?? 1, 100);
  const tier = body.tier ?? 'standard';
  const email = body.email ?? null;

  const keys: string[] = [];
  for (let i = 0; i < count; i++) {
    const key = generateConvxKey();
    await insertLicenseKey(supabase, key, tier, email);
    keys.push(key);
  }

  return Response.json({ keys, count: keys.length, tier, email });
}
