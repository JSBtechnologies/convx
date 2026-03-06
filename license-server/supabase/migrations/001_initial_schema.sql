-- Phase 1: Migrate from D1 (SQLite) to Supabase (Postgres)
-- This recreates the existing schema with Postgres-native types

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE license_keys (
  id          BIGSERIAL PRIMARY KEY,
  key         TEXT      NOT NULL UNIQUE,
  email       TEXT,
  tier        TEXT      NOT NULL DEFAULT 'standard',
  max_devices INTEGER   NOT NULL DEFAULT 1,
  revoked     BOOLEAN   NOT NULL DEFAULT FALSE,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE activations (
  id              BIGSERIAL PRIMARY KEY,
  key_id          BIGINT      NOT NULL REFERENCES license_keys(id),
  device_id       TEXT        NOT NULL,
  device_name     TEXT        NOT NULL,
  tier1_hash      TEXT        NOT NULL,
  tier2_hash      TEXT        NOT NULL,
  platform        TEXT        NOT NULL,
  schema_version  INTEGER     NOT NULL,
  activated_at    TIMESTAMPTZ NOT NULL,
  recheck_after   TIMESTAMPTZ NOT NULL,
  deactivated     BOOLEAN     NOT NULL DEFAULT FALSE,
  UNIQUE(key_id, device_id)
);

CREATE TABLE orders (
  id                BIGSERIAL PRIMARY KEY,
  ls_order_id       TEXT      NOT NULL UNIQUE,
  ls_customer_id    TEXT,
  ls_order_number   INTEGER,
  user_email        TEXT      NOT NULL,
  user_name         TEXT,
  product_name      TEXT,
  currency          TEXT      NOT NULL DEFAULT 'USD',
  total_usd         INTEGER   NOT NULL,
  status            TEXT      NOT NULL DEFAULT 'paid',
  license_key_id    BIGINT    REFERENCES license_keys(id),
  discount_code_id  BIGINT,
  email_sent        BOOLEAN   NOT NULL DEFAULT FALSE,
  email_sent_at     TIMESTAMPTZ,
  raw_payload       JSONB,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE discount_codes (
  id              BIGSERIAL PRIMARY KEY,
  code            TEXT      NOT NULL UNIQUE,
  order_id        BIGINT    REFERENCES orders(id),
  discount_cents  INTEGER   NOT NULL DEFAULT 1400,
  redeemed        BOOLEAN   NOT NULL DEFAULT FALSE,
  redeemed_at     TIMESTAMPTZ,
  redeemed_by     TEXT,
  expires_at      TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_activations_key_id ON activations(key_id);
CREATE INDEX idx_activations_key_device ON activations(key_id, device_id) WHERE deactivated = FALSE;
CREATE INDEX idx_orders_ls_order_id ON orders(ls_order_id);
CREATE INDEX idx_discount_codes_code ON discount_codes(code);
