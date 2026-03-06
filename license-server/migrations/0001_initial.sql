-- License keys issued to customers
CREATE TABLE IF NOT EXISTS license_keys (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  key         TEXT    NOT NULL UNIQUE,
  email       TEXT,
  tier        TEXT    NOT NULL DEFAULT 'standard',
  max_devices INTEGER NOT NULL DEFAULT 1,
  revoked     INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Active device bindings
CREATE TABLE IF NOT EXISTS activations (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  key_id          INTEGER NOT NULL REFERENCES license_keys(id),
  device_id       TEXT    NOT NULL,
  device_name     TEXT    NOT NULL,
  tier1_hash      TEXT    NOT NULL,
  tier2_hash      TEXT    NOT NULL,
  platform        TEXT    NOT NULL,
  schema_version  INTEGER NOT NULL,
  activated_at    TEXT    NOT NULL,
  recheck_after   TEXT    NOT NULL,
  deactivated     INTEGER NOT NULL DEFAULT 0,
  UNIQUE(key_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_activations_key ON activations(key_id);
CREATE INDEX IF NOT EXISTS idx_license_keys_key ON license_keys(key);
