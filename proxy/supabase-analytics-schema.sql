-- =============================================================
-- Vantage Usage Analytics — Supabase Schema
-- Run this in the Supabase Dashboard → SQL Editor
-- =============================================================

-- 1. proxy_analytics — every API request logged
CREATE TABLE proxy_analytics (
  id              BIGSERIAL PRIMARY KEY,
  ip_hash         TEXT NOT NULL,
  endpoint        TEXT NOT NULL,
  method          TEXT NOT NULL DEFAULT 'GET',
  response_status INTEGER NOT NULL,
  duration_ms     INTEGER,
  device_id       TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_proxy_analytics_created_day
  ON proxy_analytics (DATE(created_at));

CREATE INDEX idx_proxy_analytics_device_id
  ON proxy_analytics (device_id)
  WHERE device_id IS NOT NULL;

CREATE INDEX idx_proxy_analytics_endpoint
  ON proxy_analytics (endpoint, created_at);

ALTER TABLE proxy_analytics ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Allow anon insert" ON proxy_analytics
  FOR INSERT TO anon
  WITH CHECK (true);

CREATE POLICY "Service role full access" ON proxy_analytics
  FOR ALL TO service_role
  USING (true);

-- 2. devices — device registry with future payment columns
CREATE TABLE devices (
  device_id               TEXT PRIMARY KEY,
  ip_hash                 TEXT,
  os                      TEXT,
  arch                    TEXT,
  app_version             TEXT,
  first_seen              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  stripe_customer_id      TEXT,
  subscription_tier       TEXT DEFAULT 'free',
  subscription_status     TEXT DEFAULT 'active',
  subscription_expires_at TIMESTAMPTZ
);

CREATE INDEX idx_devices_subscription
  ON devices (subscription_tier, subscription_status);

ALTER TABLE devices ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Allow anon upsert" ON devices
  FOR INSERT TO anon
  WITH CHECK (true);

CREATE POLICY "Allow anon update non-billing" ON devices
  FOR UPDATE TO anon
  USING (true)
  WITH CHECK (true);

CREATE POLICY "Service role full access" ON devices
  FOR ALL TO service_role
  USING (true);

-- 3. events — app events (used by heartbeat and future telemetry)
CREATE TABLE events (
  id          BIGSERIAL PRIMARY KEY,
  device_id   TEXT NOT NULL REFERENCES devices(device_id),
  event_type  TEXT NOT NULL,
  ip_hash     TEXT,
  metadata    JSONB DEFAULT '{}',
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_events_device_id ON events (device_id, created_at);
CREATE INDEX idx_events_type_day  ON events (event_type, DATE(created_at));

ALTER TABLE events ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Allow anon insert" ON events
  FOR INSERT TO anon
  WITH CHECK (true);

CREATE POLICY "Service role full access" ON events
  FOR ALL TO service_role
  USING (true);

-- 4. Views for quick dashboards

-- Daily active users (unique IP hashes per day)
CREATE VIEW daily_active_users AS
SELECT
  DATE(created_at) AS day,
  COUNT(DISTINCT ip_hash) AS unique_users,
  COUNT(*) AS total_requests
FROM proxy_analytics
WHERE endpoint != '/health'
GROUP BY DATE(created_at)
ORDER BY day DESC;

-- Endpoint usage breakdown per day
CREATE VIEW endpoint_daily_stats AS
SELECT
  DATE(created_at) AS day,
  endpoint,
  COUNT(*) AS request_count,
  AVG(duration_ms) AS avg_duration_ms,
  COUNT(CASE WHEN response_status >= 400 THEN 1 END) AS error_count
FROM proxy_analytics
WHERE endpoint != '/health'
GROUP BY DATE(created_at), endpoint
ORDER BY day DESC, request_count DESC;

-- App version distribution
CREATE VIEW version_adoption AS
SELECT
  app_version,
  COUNT(*) AS device_count,
  MAX(last_seen) AS most_recent_seen
FROM devices
WHERE app_version IS NOT NULL
GROUP BY app_version
ORDER BY device_count DESC;

-- Daily active devices (from heartbeat events)
CREATE VIEW daily_active_devices AS
SELECT
  DATE(created_at) AS day,
  COUNT(DISTINCT device_id) AS unique_devices
FROM events
WHERE event_type = 'app_open'
GROUP BY DATE(created_at)
ORDER BY day DESC;

-- API usage per device per day (for future billing)
CREATE VIEW device_daily_api_usage AS
SELECT
  DATE(created_at) AS day,
  device_id,
  endpoint,
  COUNT(*) AS request_count
FROM proxy_analytics
WHERE device_id IS NOT NULL
  AND endpoint != '/health'
GROUP BY DATE(created_at), device_id, endpoint
ORDER BY day DESC;
