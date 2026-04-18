-- Migration 011: Add channel_id to quotas table for per-channel user quotas

ALTER TABLE quotas ADD COLUMN channel_id INTEGER;
