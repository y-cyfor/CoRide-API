-- Migration 005: Quotas table (user-level quotas)

CREATE TABLE IF NOT EXISTS quotas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    quota_type VARCHAR(16) NOT NULL,
    total_limit BIGINT NOT NULL,
    used BIGINT NOT NULL DEFAULT 0,
    cycle VARCHAR(16) NOT NULL DEFAULT 'permanent',
    period_start DATETIME,
    period_end DATETIME,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
