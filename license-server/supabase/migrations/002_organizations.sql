-- Phase 2: Organization & Team Model

CREATE TABLE organizations (
  id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  name            TEXT NOT NULL,
  slug            TEXT NOT NULL UNIQUE,
  billing_email   TEXT NOT NULL,
  plan            TEXT NOT NULL DEFAULT 'team',  -- team, business, enterprise
  max_seats       INTEGER NOT NULL DEFAULT 25,
  settings        JSONB NOT NULL DEFAULT '{}',
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE org_members (
  id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  user_email      TEXT NOT NULL,
  role            TEXT NOT NULL DEFAULT 'member',  -- admin, member
  auth_user_id    UUID REFERENCES auth.users(id),
  invited_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  accepted_at     TIMESTAMPTZ,
  UNIQUE(org_id, user_email)
);

CREATE TABLE org_licenses (
  id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  license_key_id  BIGINT NOT NULL REFERENCES license_keys(id),
  assigned_to     TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE audit_log (
  id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  org_id          UUID REFERENCES organizations(id),
  license_key_id  BIGINT REFERENCES license_keys(id),
  user_email      TEXT,
  action          TEXT NOT NULL,  -- activate, deactivate, transfer, convert, revoke, seat_limit_hit
  metadata        JSONB NOT NULL DEFAULT '{}',
  ip_address      TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_org_members_org_id ON org_members(org_id);
CREATE INDEX idx_org_members_auth_user ON org_members(auth_user_id);
CREATE INDEX idx_org_licenses_org_id ON org_licenses(org_id);
CREATE INDEX idx_org_licenses_key_id ON org_licenses(license_key_id);
CREATE INDEX idx_audit_log_org_id ON audit_log(org_id);
CREATE INDEX idx_audit_log_created ON audit_log(created_at DESC);

-- RLS: org admins see only their own org's data
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_licenses ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;

-- Organizations: members can read their own org
CREATE POLICY "Org members read own org" ON organizations FOR SELECT
  USING (id IN (SELECT org_id FROM org_members WHERE auth_user_id = auth.uid()));

-- Org members: can read members in their own org
CREATE POLICY "Org members read own members" ON org_members FOR SELECT
  USING (org_id IN (SELECT org_id FROM org_members WHERE auth_user_id = auth.uid()));

-- Org admins can insert/update members
CREATE POLICY "Org admins manage members" ON org_members FOR ALL
  USING (org_id IN (
    SELECT org_id FROM org_members
    WHERE auth_user_id = auth.uid() AND role = 'admin'
  ));

-- Org licenses: members can read
CREATE POLICY "Org members read licenses" ON org_licenses FOR SELECT
  USING (org_id IN (SELECT org_id FROM org_members WHERE auth_user_id = auth.uid()));

-- Audit log: members can read their org's audit log
CREATE POLICY "Org members read audit" ON audit_log FOR SELECT
  USING (org_id IN (SELECT org_id FROM org_members WHERE auth_user_id = auth.uid()));

-- Service role bypass (for the CF Worker using service key)
-- The service role key automatically bypasses RLS, so no additional policies needed
-- for server-side operations.
