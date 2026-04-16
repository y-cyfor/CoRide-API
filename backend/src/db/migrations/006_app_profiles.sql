-- Migration 006: App profiles table (application伪装 presets)

CREATE TABLE IF NOT EXISTS app_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(64) UNIQUE NOT NULL,
    identifier VARCHAR(64) UNIQUE NOT NULL,
    user_agent VARCHAR(512) NOT NULL,
    extra_headers TEXT,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
