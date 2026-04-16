-- Migration 009: Traffic plan tables (time-based app profile distribution)

-- Traffic plan: NULL channel_id means global plan, otherwise per-channel override
CREATE TABLE IF NOT EXISTS traffic_plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id INTEGER NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE
);

-- Each plan has one or more time slots distributing traffic among app profiles
CREATE TABLE IF NOT EXISTS traffic_plan_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER NOT NULL,
    start_hour INTEGER NOT NULL CHECK(start_hour >= 0 AND start_hour < 24),
    end_hour INTEGER NOT NULL CHECK(end_hour >= 0 AND end_hour <= 24),
    app_profile_id INTEGER NOT NULL,
    weight INTEGER NOT NULL DEFAULT 100 CHECK(weight > 0),
    FOREIGN KEY (plan_id) REFERENCES traffic_plans(id) ON DELETE CASCADE,
    FOREIGN KEY (app_profile_id) REFERENCES app_profiles(id)
);

CREATE INDEX IF NOT EXISTS idx_traffic_plans_channel ON traffic_plans(channel_id);
CREATE INDEX IF NOT EXISTS idx_traffic_plan_slots_plan ON traffic_plan_slots(plan_id);
