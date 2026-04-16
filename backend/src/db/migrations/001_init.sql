-- Migration 001: Initialize core tables
-- request_logs and rate_limit_configs

CREATE TABLE IF NOT EXISTS request_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_api_key VARCHAR(128) NOT NULL,
    channel_id INTEGER,
    model VARCHAR(128) NOT NULL,
    endpoint VARCHAR(256) NOT NULL,
    status_code INTEGER NOT NULL,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    elapsed_ms INTEGER NOT NULL,
    request_body TEXT,
    response_body TEXT,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for high-frequency query fields
CREATE INDEX IF NOT EXISTS idx_request_logs_api_key ON request_logs(user_api_key);
CREATE INDEX IF NOT EXISTS idx_request_logs_model ON request_logs(model);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_request_logs_status_code ON request_logs(status_code);

CREATE TABLE IF NOT EXISTS rate_limit_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_type VARCHAR(32) NOT NULL,
    target_id INTEGER,
    qps INTEGER NOT NULL DEFAULT 0,
    concurrency INTEGER NOT NULL DEFAULT 0,
    action VARCHAR(16) NOT NULL DEFAULT 'reject',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
