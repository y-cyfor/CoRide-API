-- Migration 007: Add app_profile_id foreign key to channels

ALTER TABLE channels ADD COLUMN app_profile_id INTEGER REFERENCES app_profiles(id);
