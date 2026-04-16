-- Migration 008: Add channel health check columns
ALTER TABLE channels ADD COLUMN consecutive_failures INTEGER NOT NULL DEFAULT 0;
ALTER TABLE channels ADD COLUMN last_checked DATETIME;
ALTER TABLE channels ADD COLUMN last_success_at DATETIME;
