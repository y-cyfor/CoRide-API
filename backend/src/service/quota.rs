use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::db::models::{self, Quota};

#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    #[error("Quota exceeded: {0} limit reached")]
    Exceeded(String),
    #[error("No active quota found")]
    NotFound,
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
}

/// Check if a user has remaining quota for a specific channel.
/// Checks channel-level quota first, falls back to user-level quota.
pub async fn check_user_quota(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
    tokens: i32,
) -> Result<(), QuotaError> {
    let quota = match models::get_user_quota_for_channel(pool, user_id, channel_id).await? {
        Some(q) => q,
        None => return Ok(()), // No quota restriction = allowed
    };

    // Reset used if cycle expired
    reset_quota_if_expired(pool, &quota).await?;

    match quota.quota_type.as_str() {
        "requests" => {
            if quota.used >= quota.total_limit {
                return Err(QuotaError::Exceeded(format!("{} requests", quota.total_limit)));
            }
        }
        "tokens" => {
            if quota.used + (tokens as i64) > quota.total_limit {
                return Err(QuotaError::Exceeded(format!("{} tokens", quota.total_limit)));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Deduct user quota for a specific channel after a successful request.
/// Only deducts from the applicable quota (channel-level or user-level).
/// Returns Ok(()) if deducted successfully, or Exceeded if quota would be exceeded.
pub async fn deduct_user_quota(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
    tokens: i32,
) -> Result<(), QuotaError> {
    let quota = match models::get_user_quota_for_channel(pool, user_id, channel_id).await? {
        Some(q) => q,
        None => return Ok(()), // No quota to deduct from
    };

    match quota.quota_type.as_str() {
        "requests" => {
            let ok = models::increment_quota_used(pool, quota.id, 1).await?;
            if !ok {
                return Err(QuotaError::Exceeded(format!("{} requests", quota.total_limit)));
            }
        }
        "tokens" => {
            let ok = models::increment_quota_used(pool, quota.id, tokens as i64).await?;
            if !ok {
                return Err(QuotaError::Exceeded(format!("{} tokens", quota.total_limit)));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Check if a channel has remaining quota.
pub async fn check_channel_quota(pool: &SqlitePool, channel_id: i64) -> Result<(), QuotaError> {
    // First, check and reset quota if cycle expired
    reset_channel_quota_if_expired(pool, channel_id).await?;

    let ok = models::check_channel_quota(pool, channel_id).await?;
    if !ok {
        return Err(QuotaError::Exceeded("channel quota".to_string()));
    }
    Ok(())
}

/// Reset channel quota used if the current cycle has expired.
async fn reset_channel_quota_if_expired(pool: &SqlitePool, channel_id: i64) -> Result<(), QuotaError> {
    // Get channel quota info
    let row: (Option<String>, Option<i64>, Option<String>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT quota_type, quota_limit, quota_cycle, quota_period_end FROM channels WHERE id = ?",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or((None, None, None, None));

    let (quota_type, _quota_limit, quota_cycle, period_end) = row;

    // No quota configured or permanent cycle
    if quota_type.is_none() || quota_cycle.as_deref() == Some("permanent") || quota_cycle.is_none() {
        return Ok(());
    }

    let cycle = quota_cycle.as_deref().unwrap_or("permanent");
    if cycle == "permanent" {
        return Ok(());
    }

    // Check if period_end has passed
    if let Some(end) = period_end {
        if end < Utc::now() {
            // Reset quota and advance period
            let now = Utc::now();
            let new_end = match cycle {
                "hourly" => now + chrono::Duration::hours(1),
                "daily" => now + chrono::Duration::days(1),
                "weekly" => now + chrono::Duration::weeks(1),
                "monthly" => now + chrono::Duration::days(30),
                _ => return Ok(()),
            };

            sqlx::query(
                "UPDATE channels SET quota_used = 0, quota_period_start = ?, quota_period_end = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(now)
            .bind(new_end)
            .bind(channel_id)
            .execute(pool)
            .await?;

            tracing::info!(channel_id, cycle, "Channel quota period reset");
        }
    }

    Ok(())
}

/// Deduct channel quota after a successful request.
pub async fn deduct_channel_quota(pool: &SqlitePool, channel_id: i64, tokens: i32) -> Result<(), QuotaError> {
    let ok = models::increment_channel_quota(pool, channel_id, tokens as i64).await?;
    if !ok {
        return Err(QuotaError::Exceeded("channel quota".to_string()));
    }
    Ok(())
}

/// Reset quota used if the current cycle has expired.
async fn reset_quota_if_expired(pool: &SqlitePool, quota: &Quota) -> Result<(), QuotaError> {
    if quota.cycle == "permanent" {
        return Ok(());
    }

    // Check if period_end has passed
    if let Some(end) = quota.period_end {
        if end < Utc::now() {
            models::reset_quota_if_cycle_expired(pool, quota.id).await?;
        }
    }

    Ok(())
}

/// Initialize quota period dates based on cycle type.
pub fn init_quota_period(cycle: &str) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let now = Utc::now();
    match cycle {
        "hourly" => (Some(now), Some(now + chrono::Duration::hours(1))),
        "daily" => (Some(now), Some(now + chrono::Duration::days(1))),
        "weekly" => (Some(now), Some(now + chrono::Duration::weeks(1))),
        "monthly" => (Some(now), Some(now + chrono::Duration::days(30))),
        "permanent" | _ => (None, None),
    }
}
