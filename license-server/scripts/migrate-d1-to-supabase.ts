/**
 * One-time migration script: D1 (SQLite) → Supabase (Postgres)
 *
 * Usage:
 *   D1_DATABASE_ID=xxx SUPABASE_URL=xxx SUPABASE_SERVICE_KEY=xxx npx tsx scripts/migrate-d1-to-supabase.ts
 *
 * Prerequisites:
 *   1. Run the Supabase migration (001_initial_schema.sql) first
 *   2. Export D1 data using `wrangler d1 export convx-licenses --remote --output=d1-export.json`
 *   3. Or use the Wrangler D1 HTTP API to read rows
 *
 * This script reads from a D1 JSON export and inserts into Supabase.
 */

import { createClient } from '@supabase/supabase-js';
import { readFileSync } from 'fs';

const SUPABASE_URL = process.env.SUPABASE_URL!;
const SUPABASE_SERVICE_KEY = process.env.SUPABASE_SERVICE_KEY!;

if (!SUPABASE_URL || !SUPABASE_SERVICE_KEY) {
  console.error('Set SUPABASE_URL and SUPABASE_SERVICE_KEY environment variables');
  process.exit(1);
}

const supabase = createClient(SUPABASE_URL, SUPABASE_SERVICE_KEY, {
  auth: { persistSession: false },
});

interface D1Export {
  license_keys: Array<{
    id: number;
    key: string;
    email: string | null;
    tier: string;
    max_devices: number;
    revoked: number; // SQLite integer boolean
    created_at: string;
  }>;
  activations: Array<{
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
    deactivated: number; // SQLite integer boolean
  }>;
  orders: Array<{
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
    email_sent: number; // SQLite integer boolean
    email_sent_at: string | null;
    raw_payload: string | null;
    created_at: string;
  }>;
  discount_codes: Array<{
    id: number;
    code: string;
    order_id: number | null;
    discount_cents: number;
    redeemed: number; // SQLite integer boolean
    redeemed_at: string | null;
    redeemed_by: string | null;
    expires_at: string | null;
    created_at: string;
  }>;
}

async function migrate() {
  const exportPath = process.argv[2] || 'd1-export.json';
  console.log(`Reading D1 export from: ${exportPath}`);

  let data: D1Export;
  try {
    data = JSON.parse(readFileSync(exportPath, 'utf-8'));
  } catch (err) {
    console.error(`Failed to read export file: ${err}`);
    console.error('\nTo export D1 data, run:');
    console.error('  wrangler d1 export convx-licenses --remote --output=d1-export.json');
    console.error('\nOr create a JSON file with this structure:');
    console.error('  { "license_keys": [...], "activations": [...], "orders": [...], "discount_codes": [...] }');
    process.exit(1);
  }

  // 1. Migrate license_keys
  if (data.license_keys?.length) {
    console.log(`Migrating ${data.license_keys.length} license keys...`);
    const rows = data.license_keys.map((r) => ({
      id: r.id,
      key: r.key,
      email: r.email,
      tier: r.tier,
      max_devices: r.max_devices,
      revoked: r.revoked === 1, // SQLite int → Postgres bool
      created_at: r.created_at,
    }));

    // Insert in batches of 100
    for (let i = 0; i < rows.length; i += 100) {
      const batch = rows.slice(i, i + 100);
      const { error } = await supabase.from('license_keys').insert(batch);
      if (error) {
        console.error(`  Error at batch ${i}: ${error.message}`);
      } else {
        console.log(`  Inserted ${Math.min(i + 100, rows.length)}/${rows.length}`);
      }
    }

    // Reset sequence to max id
    const maxId = Math.max(...data.license_keys.map((r) => r.id));
    await supabase.rpc('setval_license_keys', { val: maxId }).catch(() => {
      console.log(`  Note: Run manually: SELECT setval('license_keys_id_seq', ${maxId});`);
    });
  }

  // 2. Migrate activations
  if (data.activations?.length) {
    console.log(`Migrating ${data.activations.length} activations...`);
    const rows = data.activations.map((r) => ({
      id: r.id,
      key_id: r.key_id,
      device_id: r.device_id,
      device_name: r.device_name,
      tier1_hash: r.tier1_hash,
      tier2_hash: r.tier2_hash,
      platform: r.platform,
      schema_version: r.schema_version,
      activated_at: r.activated_at,
      recheck_after: r.recheck_after,
      deactivated: r.deactivated === 1,
    }));

    for (let i = 0; i < rows.length; i += 100) {
      const batch = rows.slice(i, i + 100);
      const { error } = await supabase.from('activations').insert(batch);
      if (error) {
        console.error(`  Error at batch ${i}: ${error.message}`);
      } else {
        console.log(`  Inserted ${Math.min(i + 100, rows.length)}/${rows.length}`);
      }
    }

    const maxId = Math.max(...data.activations.map((r) => r.id));
    console.log(`  Note: Run manually: SELECT setval('activations_id_seq', ${maxId});`);
  }

  // 3. Migrate orders
  if (data.orders?.length) {
    console.log(`Migrating ${data.orders.length} orders...`);
    const rows = data.orders.map((r) => ({
      id: r.id,
      ls_order_id: r.ls_order_id,
      ls_customer_id: r.ls_customer_id,
      ls_order_number: r.ls_order_number,
      user_email: r.user_email,
      user_name: r.user_name,
      product_name: r.product_name,
      currency: r.currency,
      total_usd: r.total_usd,
      status: r.status,
      license_key_id: r.license_key_id,
      discount_code_id: r.discount_code_id,
      email_sent: r.email_sent === 1,
      email_sent_at: r.email_sent_at,
      raw_payload: r.raw_payload ? JSON.parse(r.raw_payload) : null,
      created_at: r.created_at,
    }));

    for (let i = 0; i < rows.length; i += 100) {
      const batch = rows.slice(i, i + 100);
      const { error } = await supabase.from('orders').insert(batch);
      if (error) {
        console.error(`  Error at batch ${i}: ${error.message}`);
      } else {
        console.log(`  Inserted ${Math.min(i + 100, rows.length)}/${rows.length}`);
      }
    }

    const maxId = Math.max(...data.orders.map((r) => r.id));
    console.log(`  Note: Run manually: SELECT setval('orders_id_seq', ${maxId});`);
  }

  // 4. Migrate discount_codes
  if (data.discount_codes?.length) {
    console.log(`Migrating ${data.discount_codes.length} discount codes...`);
    const rows = data.discount_codes.map((r) => ({
      id: r.id,
      code: r.code,
      order_id: r.order_id,
      discount_cents: r.discount_cents,
      redeemed: r.redeemed === 1,
      redeemed_at: r.redeemed_at,
      redeemed_by: r.redeemed_by,
      expires_at: r.expires_at,
      created_at: r.created_at,
    }));

    for (let i = 0; i < rows.length; i += 100) {
      const batch = rows.slice(i, i + 100);
      const { error } = await supabase.from('discount_codes').insert(batch);
      if (error) {
        console.error(`  Error at batch ${i}: ${error.message}`);
      } else {
        console.log(`  Inserted ${Math.min(i + 100, rows.length)}/${rows.length}`);
      }
    }

    const maxId = Math.max(...data.discount_codes.map((r) => r.id));
    console.log(`  Note: Run manually: SELECT setval('discount_codes_id_seq', ${maxId});`);
  }

  console.log('\nMigration complete!');
  console.log('\nPost-migration steps:');
  console.log('  1. Reset all sequences (see notes above)');
  console.log('  2. Set wrangler secrets: SUPABASE_URL, SUPABASE_SERVICE_KEY');
  console.log('  3. Deploy: wrangler deploy');
  console.log('  4. Test all endpoints against the new backend');
  console.log('  5. Remove D1 binding from wrangler.toml (already done)');
}

migrate().catch(console.error);
