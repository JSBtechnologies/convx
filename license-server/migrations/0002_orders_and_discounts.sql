-- Orders from LemonSqueezy webhooks
CREATE TABLE IF NOT EXISTS orders (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  ls_order_id       TEXT    NOT NULL UNIQUE,
  ls_customer_id    TEXT,
  ls_order_number   INTEGER,
  user_email        TEXT    NOT NULL,
  user_name         TEXT,
  product_name      TEXT,
  currency          TEXT    NOT NULL DEFAULT 'USD',
  total_usd         INTEGER NOT NULL,
  status            TEXT    NOT NULL DEFAULT 'paid',
  license_key_id    INTEGER REFERENCES license_keys(id),
  discount_code_id  INTEGER,
  email_sent        INTEGER NOT NULL DEFAULT 0,
  email_sent_at     TEXT,
  raw_payload       TEXT,
  created_at        TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Agent Toolbox discount codes generated with each ConvX purchase
CREATE TABLE IF NOT EXISTS discount_codes (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  code            TEXT    NOT NULL UNIQUE,
  order_id        INTEGER REFERENCES orders(id),
  discount_cents  INTEGER NOT NULL DEFAULT 1400,
  redeemed        INTEGER NOT NULL DEFAULT 0,
  redeemed_at     TEXT,
  redeemed_by     TEXT,
  expires_at      TEXT,
  created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_orders_ls_order_id ON orders(ls_order_id);
CREATE INDEX IF NOT EXISTS idx_orders_email ON orders(user_email);
CREATE INDEX IF NOT EXISTS idx_discount_codes_code ON discount_codes(code);
CREATE INDEX IF NOT EXISTS idx_discount_codes_order ON discount_codes(order_id);
