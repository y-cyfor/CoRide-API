-- Performance indexes for high-frequency query patterns
-- BUG-042: quotas(user_id, enabled) — user quota lookups
CREATE INDEX IF NOT EXISTS idx_quotas_user_enabled ON quotas(user_id, enabled);
-- BUG-043: rate_limit_configs(target_type, target_id) — rate limit matching
CREATE INDEX IF NOT EXISTS idx_rate_limit_target ON rate_limit_configs(target_type, target_id);
-- BUG-044: models(proxy_name, enabled) — model name reverse lookup
CREATE INDEX IF NOT EXISTS idx_models_proxy_enabled ON models(proxy_name, enabled);
-- BUG-045: models(channel_id, enabled) — channel-model join
CREATE INDEX IF NOT EXISTS idx_models_channel_enabled ON models(channel_id, enabled);
