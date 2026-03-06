import { handleActivate } from './routes/activate';
import { handleAdminGenerate } from './routes/admin';
import { handleDeactivate } from './routes/deactivate';
import { handleValidateDiscount, handleRedeemDiscount } from './routes/discount';
import { handleDownloadToken } from './routes/download';
import {
  handleCreateOrg,
  handleGenerateOrgKeys,
  handleGetOrgAudit,
  handleGetOrgSeats,
  handleGetOrgSettings,
  handleGetOrgSettingsByKey,
  handleInviteOrgMember,
  handleListOrgLicenses,
  handleListOrgMembers,
  handleRemoveOrgMember,
  handleRevokeOrgDevice,
  handleUpdateOrgMemberRole,
  handleUpdateOrgSettings,
} from './routes/org';
import { handleTransfer } from './routes/transfer';
import { handleValidate } from './routes/validate';
import { handleWebhook } from './routes/webhook';
import { getSupabase } from './supabase';
import type { Env } from './types';

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    // CORS preflight
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: corsHeaders(request),
      });
    }

    const supabase = getSupabase(env);

    // GET /v1/org/settings-by-key?key=CONVX-XXXX-...
    if (path === '/v1/org/settings-by-key' && request.method === 'GET') {
      try {
        return withCors(await handleGetOrgSettingsByKey(url, supabase), request);
      } catch (err) {
        console.error('Settings-by-key error:', err);
        return withCors(Response.json({ error: 'Internal server error' }, { status: 500 }), request);
      }
    }

    // ─── Org routes (support GET and POST/PUT) ───────────────────────────
    const orgMatch = path.match(/^\/v1\/org\/([0-9a-f-]+)\/(seats|keys|revoke|settings|audit|members|licenses)$/);
    if (orgMatch) {
      const [, orgId, action] = orgMatch;

      try {
        // GET routes
        if (request.method === 'GET') {
          switch (action) {
            case 'seats':
              return withCors(await handleGetOrgSeats(orgId, env, request, supabase), request);
            case 'settings':
              return withCors(await handleGetOrgSettings(orgId, env, request, supabase), request);
            case 'audit':
              return withCors(await handleGetOrgAudit(orgId, url, env, request, supabase), request);
            case 'members':
              return withCors(await handleListOrgMembers(orgId, env, request, supabase), request);
            case 'licenses':
              return withCors(await handleListOrgLicenses(orgId, env, request, supabase), request);
            default:
              return withCors(Response.json({ error: 'Method not allowed' }, { status: 405 }), request);
          }
        }

        // POST/PUT routes
        if (request.method === 'POST' || request.method === 'PUT') {
          const body = await request.json() as Record<string, unknown>;
          switch (action) {
            case 'keys':
              return withCors(await handleGenerateOrgKeys(orgId, body, env, request, supabase), request);
            case 'revoke':
              return withCors(await handleRevokeOrgDevice(orgId, body as Parameters<typeof handleRevokeOrgDevice>[1], env, request, supabase), request);
            case 'settings':
              return withCors(await handleUpdateOrgSettings(orgId, body, env, request, supabase), request);
            case 'members':
              if (request.method === 'PUT') {
                return withCors(await handleUpdateOrgMemberRole(orgId, body as { member_id: string; role: string }, env, request, supabase), request);
              }
              return withCors(await handleInviteOrgMember(orgId, body as Parameters<typeof handleInviteOrgMember>[1], env, request, supabase), request);
            default:
              return withCors(Response.json({ error: 'Method not allowed' }, { status: 405 }), request);
          }
        }

        // DELETE routes
        if (request.method === 'DELETE') {
          const body = await request.json() as Record<string, unknown>;
          switch (action) {
            case 'members':
              return withCors(await handleRemoveOrgMember(orgId, body as { member_id: string }, env, request, supabase), request);
            default:
              return withCors(Response.json({ error: 'Method not allowed' }, { status: 405 }), request);
          }
        }

        return withCors(Response.json({ error: 'Method not allowed' }, { status: 405 }), request);
      } catch (err) {
        console.error('Org route error:', err);
        return withCors(Response.json({ error: 'Internal server error' }, { status: 500 }), request);
      }
    }

    // POST /v1/org (create org)
    if (path === '/v1/org' && request.method === 'POST') {
      try {
        const body = await request.json();
        return withCors(await handleCreateOrg(body as Parameters<typeof handleCreateOrg>[0], env, request, supabase), request);
      } catch (err) {
        console.error('Create org error:', err);
        return withCors(Response.json({ error: 'Internal server error' }, { status: 500 }), request);
      }
    }

    // All remaining endpoints are POST only
    if (request.method !== 'POST') {
      return withCors(Response.json({ error: 'Method not allowed' }, { status: 405 }), request);
    }

    try {
      // Validate required secrets early — surfaces config issues clearly
      if (!env.SUPABASE_URL || !env.SUPABASE_SERVICE_KEY) {
        console.error('Missing SUPABASE_URL or SUPABASE_SERVICE_KEY');
        return withCors(
          Response.json({ error: 'Server misconfigured: missing database credentials' }, { status: 500 }),
          request,
        );
      }
      if (!env.ED25519_PRIVATE_KEY) {
        console.error('Missing ED25519_PRIVATE_KEY');
        return withCors(
          Response.json({ error: 'Server misconfigured: missing signing key' }, { status: 500 }),
          request,
        );
      }

      let response: Response;

      // Webhook route needs raw body for HMAC verification
      if (path === '/v1/webhook/lemonsqueezy') {
        const rawBody = await request.text();
        response = await handleWebhook(rawBody, env, request, supabase);
        return withCors(response, request);
      }

      // All other routes parse JSON
      let body: unknown;
      try {
        body = await request.json();
      } catch {
        return withCors(Response.json({ error: 'Invalid JSON body' }, { status: 400 }), request);
      }

      switch (path) {
        case '/v1/license/activate':
          response = await handleActivate(body as Parameters<typeof handleActivate>[0], env, supabase);
          break;
        case '/v1/license/validate':
          response = await handleValidate(body as Parameters<typeof handleValidate>[0], env, supabase);
          break;
        case '/v1/license/deactivate':
          response = await handleDeactivate(body as Parameters<typeof handleDeactivate>[0], supabase);
          break;
        case '/v1/license/transfer':
          response = await handleTransfer(body as Parameters<typeof handleTransfer>[0], env, supabase);
          break;
        case '/v1/admin/generate-keys':
          response = await handleAdminGenerate(
            body as Parameters<typeof handleAdminGenerate>[0],
            env,
            request,
            supabase,
          );
          break;
        case '/v1/discount/validate':
          response = await handleValidateDiscount(
            body as Parameters<typeof handleValidateDiscount>[0],
            supabase,
          );
          break;
        case '/v1/discount/redeem':
          response = await handleRedeemDiscount(
            body as Parameters<typeof handleRedeemDiscount>[0],
            supabase,
          );
          break;
        case '/v1/download/token':
          response = await handleDownloadToken(
            body as Parameters<typeof handleDownloadToken>[0],
            env,
            supabase,
          );
          break;
        default:
          response = Response.json({ error: 'Not found' }, { status: 404 });
      }

      return withCors(response, request);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error('Unhandled error:', message, err);
      return withCors(
        Response.json(
          { error: 'Internal server error', detail: message },
          { status: 500 },
        ),
        request,
      );
    }
  },
} satisfies ExportedHandler<Env>;

const ALLOWED_ORIGINS = [
  'https://convx.dev',
  'https://www.convx.dev',
  'https://enterprise.convx.dev',
  'tauri://localhost',
  'https://tauri.localhost',
];

function getAllowedOrigin(request: Request): string | null {
  const origin = request.headers.get('Origin');
  if (!origin) return null;
  if (ALLOWED_ORIGINS.includes(origin)) return origin;
  return null;
}

function corsHeaders(request?: Request): Record<string, string> {
  const origin = request ? getAllowedOrigin(request) : null;
  const headers: Record<string, string> = {
    'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    'Vary': 'Origin',
  };
  if (origin) {
    headers['Access-Control-Allow-Origin'] = origin;
  }
  return headers;
}

function withCors(response: Response, request?: Request): Response {
  const headers = new Headers(response.headers);
  const origin = request ? getAllowedOrigin(request) : null;
  if (origin) {
    headers.set('Access-Control-Allow-Origin', origin);
  }
  headers.set('Vary', 'Origin');
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}
