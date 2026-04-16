-- Migration 003: Channels table (includes quota fields)

CREATE TABLE IF NOT EXISTS channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(64) UNIQUE NOT NULL,
    type VARCHAR(32) NOT NULL,
    base_url VARCHAR(512) NOT NULL,
    api_keys TEXT NOT NULL,
    custom_headers TEXT,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    weight INTEGER NOT NULL DEFAULT 1,
    timeout INTEGER NOT NULL DEFAULT 300000,
    retry_count INTEGER NOT NULL DEFAULT 1,
    quota_type VARCHAR(16),
    quota_limit BIGINT,
    quota_used BIGINT NOT NULL DEFAULT 0,
    quota_cycle VARCHAR(16),
    quota_period_start DATETIME,
    quota_period_end DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
