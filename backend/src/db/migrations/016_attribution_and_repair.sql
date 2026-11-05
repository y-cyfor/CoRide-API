-- Migration 016: request attribution, auto-disable marker, quota repair
--
-- user_id/key_id let the UI attribute traffic to a user (and a specific key)
-- without scanning/`LIKE`-matching the raw API key column.
ALTER TABLE request_logs ADD COLUMN user_id INTEGER;
ALTER TABLE request_logs ADD COLUMN key_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_request_logs_user_id ON request_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_user_created ON request_logs(user_id, created_at);

-- Marks channels that were disabled by the health checker so that only those are
-- probed for automatic recovery. Manually disabled channels stay disabled.
ALTER TABLE channels ADD COLUMN auto_disabled BOOLEAN NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_quotas_user_channel ON quotas(user_id, channel_id);
CREATE INDEX IF NOT EXISTS idx_user_keys_value_status ON user_keys(key_value, status);
CREATE INDEX IF NOT EXISTS idx_ip_whitelist_user ON ip_whitelist(user_id);

-- One-time repair: channels configured with a *request* quota were wrongly charged
-- in tokens by older builds, which left them permanently exhausted (quota_used far
-- above quota_limit with no reset path). Such rows can never serve a request, so
-- clearing them is strictly an improvement.
UPDATE channels
SET quota_used = 0
WHERE quota_type = 'requests'
  AND quota_limit IS NOT NULL
  AND quota_used > quota_limit;

-- Give channels that already have a quota cycle but no period an initial period so
-- that the automatic reset can start working for existing deployments.
UPDATE channels
SET quota_period_start = COALESCE(quota_period_start, created_at, CURRENT_TIMESTAMP),
    quota_period_end = CASE quota_cycle
        WHEN 'hourly'  THEN datetime(COALESCE(quota_period_start, created_at, CURRENT_TIMESTAMP), '+1 hour')
        WHEN 'daily'   THEN datetime(COALESCE(quota_period_start, created_at, CURRENT_TIMESTAMP), '+1 day')
        WHEN 'weekly'  THEN datetime(COALESCE(quota_period_start, created_at, CURRENT_TIMESTAMP), '+7 day')
        WHEN 'monthly' THEN datetime(COALESCE(quota_period_start, created_at, CURRENT_TIMESTAMP), '+30 day')
        ELSE NULL
    END
WHERE quota_type IS NOT NULL
  AND quota_cycle IN ('hourly', 'daily', 'weekly', 'monthly')
  AND quota_period_end IS NULL;
