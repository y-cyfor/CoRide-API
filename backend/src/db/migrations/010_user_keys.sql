-- User API Keys table
-- Each user can have multiple API keys with independent model access settings

CREATE TABLE IF NOT EXISTS user_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    key_value VARCHAR(128) UNIQUE NOT NULL,
    name VARCHAR(64),
    enabled_models TEXT,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_user_keys_user_id ON user_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_user_keys_key_value ON user_keys(key_value);
CREATE INDEX IF NOT EXISTS idx_user_keys_status ON user_keys(status);
