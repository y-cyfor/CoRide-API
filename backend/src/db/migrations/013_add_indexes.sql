-- Migration 013: Add missing indexes for performance

-- Index on request_logs.channel_id for channel usage stats aggregation
CREATE INDEX IF NOT EXISTS idx_request_logs_channel_id ON request_logs(channel_id);

-- Index on quotas.channel_id for per-channel user quota lookup
CREATE INDEX IF NOT EXISTS idx_quotas_channel_id ON quotas(channel_id);
