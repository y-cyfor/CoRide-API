use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::db::models::{self, Quota};

#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    #[error("Quota exceeded: {0} limit reached")]
    Exceeded(String),
    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),
}

/// Outcome of a user quota check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaVerdict {
    /// Whether the request may proceed.
    pub allowed: bool,
    /// True when the exhausted quota is scoped to a single channel, in which case the
    /// proxy should fall back to another channel instead of failing the request.
    pub channel_scoped: bool,
    pub message: String,
}

impl QuotaVerdict {
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            channel_scoped: false,
            message: String::new(),
        }
    }
}

/// Compute the current period for a quota cycle.
///
/// `None`/`permanent` cycles have no period and never reset.
pub fn compute_period(cycle: Option<&str>) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let now = Utc::now();
    match cycle.map(str::trim) {
        Some("hourly") => (Some(now), Some(now + chrono::Duration::hours(1))),
        Some("daily") => (Some(now), Some(now + chrono::Duration::days(1))),
        Some("weekly") => (Some(now), Some(now + chrono::Duration::weeks(1))),
        // 30-day months keep the arithmetic simple and monotonic.
        Some("monthly") => (Some(now), Some(now + chrono::Duration::days(30))),
        _ => (None, None),
    }
}

/// Backwards-compatible alias used by the quota API handlers.
pub fn init_quota_period(cycle: &str) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    compute_period(Some(cycle))
}

/// Validate a quota cycle name.
pub fn is_valid_cycle(cycle: &str) -> bool {
    matches!(cycle, "hourly" | "daily" | "weekly" | "monthly" | "permanent")
}

/// Validate a quota type name.
pub fn is_valid_quota_type(quota_type: &str) -> bool {
    matches!(quota_type, "requests" | "tokens")
}

/// How many units of a channel quota one request consumes.
///
/// Request quotas count calls; token quotas count estimated/actual tokens.
pub fn channel_quota_cost(quota_type: Option<&str>, tokens: i64) -> i64 {
    match quota_type.map(str::trim) {
        Some("requests") => 1,
        Some("tokens") => tokens.max(0),
        // Unknown/absent quota type: never charge, but still allow the request. The
        // channel is only limited when `quota_limit` is set *and* a type is known.
        _ => 0,
    }
}

/// Decide whether a quota still has room for one more request.
///
/// * request quotas stop once `used` reaches the limit;
/// * token quotas stop once usage reaches the limit, because the exact cost is unknown
///   before the upstream answers (the post-flight deduction may overshoot by one call).
pub fn quota_allows(quota_type: &str, used: i64, limit: i64, cost: i64) -> bool {
    match quota_type {
        "requests" => used + cost.max(1) <= limit,
        "tokens" => used < limit,
        _ => true,
    }
}

/// Check if a user has remaining quota for a specific channel.
///
/// Channel-scoped quotas take priority over the user's global quota.
pub async fn check_user_quota(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
    tokens: i64,
) -> Result<QuotaVerdict, QuotaError> {
    let quota = match models::get_user_quota_for_channel(pool, user_id, channel_id).await? {
        Some(q) => q,
        None => return Ok(QuotaVerdict::allowed()),
    };

    // Reset before evaluating so an expired period never blocks a request.
    reset_quota_if_expired(pool, &quota).await?;
    let quota = match models::get_quota_by_id(pool, quota.id).await? {
        Some(q) => q,
        None => return Ok(QuotaVerdict::allowed()),
    };

    let channel_scoped = quota.channel_id.is_some();
    let cost = if quota.quota_type == "requests" { 1 } else { tokens };
    let exceeded = !quota_allows(&quota.quota_type, quota.used, quota.total_limit, cost);

    if exceeded {
        Ok(QuotaVerdict {
            allowed: false,
            channel_scoped,
            message: format!(
                "{} quota exceeded: {}/{}",
                quota.quota_type, quota.used, quota.total_limit
            ),
        })
    } else {
        Ok(QuotaVerdict::allowed())
    }
}

/// Deduct user quota for a specific channel after a successful request.
pub async fn deduct_user_quota(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
    tokens: i64,
) -> Result<(), QuotaError> {
    let quota = match models::get_user_quota_for_channel(pool, user_id, channel_id).await? {
        Some(q) => q,
        None => return Ok(()), // No quota to deduct from
    };

    let amount = match quota.quota_type.as_str() {
        "requests" => 1,
        "tokens" => tokens.max(0),
        _ => return Ok(()),
    };

    if amount == 0 {
        return Ok(());
    }

    let ok = models::increment_quota_used(pool, quota.id, amount).await?;
    if !ok {
        tracing::warn!(
            quota_id = quota.id,
            amount,
            limit = quota.total_limit,
            "Quota deduction would exceed the limit; recording the request anyway"
        );
    }
    Ok(())
}

/// Check whether a channel has room for one more request.
///
/// Resets the channel period first so expired quotas never block traffic.
pub async fn check_channel_quota(
    pool: &SqlitePool,
    channel_id: i64,
    cost: i64,
) -> Result<bool, QuotaError> {
    reset_channel_quota_if_expired(pool, channel_id).await?;

    let Some((quota_type, quota_limit, quota_used)) =
        models::channel_quota_state(pool, channel_id).await?
    else {
        return Ok(true);
    };

    // No limit configured => unlimited.
    let Some(limit) = quota_limit else {
        return Ok(true);
    };

    let quota_type = quota_type.unwrap_or_default();
    Ok(quota_allows(&quota_type, quota_used, limit, cost))
}

/// Deduct channel quota after a successful request.
pub async fn deduct_channel_quota(
    pool: &SqlitePool,
    channel_id: i64,
    quota_type: Option<&str>,
    tokens: i64,
) -> Result<(), QuotaError> {
    let cost = channel_quota_cost(quota_type, tokens);
    if cost == 0 {
        return Ok(());
    }
    let ok = models::increment_channel_quota(pool, channel_id, cost).await?;
    if !ok {
        tracing::warn!(
            channel_id,
            cost,
            "Channel quota deduction would exceed the limit; recording the request anyway"
        );
    }
    Ok(())
}

/// Reset the channel quota when its period has elapsed.
///
/// Channels created by older builds have no `quota_period_end`; those get an initial
/// period so that the reset starts working instead of blocking forever.
pub async fn reset_channel_quota_if_expired(
    pool: &SqlitePool,
    channel_id: i64,
) -> Result<(), QuotaError> {
    // (quota type, limit, cycle, period end, channel created_at)
    type ChannelQuotaRow = (
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<DateTime<Utc>>,
        DateTime<Utc>,
    );
    let row: Option<ChannelQuotaRow> = sqlx::query_as(
        "SELECT quota_type, quota_limit, quota_cycle, quota_period_end, created_at
         FROM channels WHERE id = ?",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    let Some((quota_type, quota_limit, quota_cycle, period_end, created_at)) = row else {
        return Ok(());
    };

    // No quota configured, or a non-repeating cycle.
    if quota_type.is_none() || quota_limit.is_none() {
        return Ok(());
    }

    let cycle = match quota_cycle.as_deref().map(str::trim) {
        Some("hourly") | Some("daily") | Some("weekly") | Some("monthly") => {
            quota_cycle.as_deref().unwrap().trim().to_string()
        }
        _ => return Ok(()), // permanent / unknown
    };

    let now = Utc::now();

    match period_end {
        Some(end) if end >= now => Ok(()),
        Some(_) => {
            let new_end = period_end_from(&cycle, now);
            sqlx::query(
                "UPDATE channels SET quota_used = 0, quota_period_start = ?, quota_period_end = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(now)
            .bind(new_end)
            .bind(channel_id)
            .execute(pool)
            .await?;
            tracing::info!(channel_id, cycle = %cycle, "Channel quota period reset");
            Ok(())
        }
        None => {
            // Legacy row without a period: start one now, keep the current usage so
            // admins still see it, and let the next boundary reset it.
            let new_end = period_end_from(&cycle, now);
            let start = if created_at < now { created_at } else { now };
            sqlx::query(
                "UPDATE channels SET quota_period_start = ?, quota_period_end = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(start)
            .bind(new_end)
            .bind(channel_id)
            .execute(pool)
            .await?;
            tracing::info!(channel_id, cycle = %cycle, "Initialised missing channel quota period");
            Ok(())
        }
    }
}

fn period_end_from(cycle: &str, from: DateTime<Utc>) -> DateTime<Utc> {
    match cycle {
        "hourly" => from + chrono::Duration::hours(1),
        "daily" => from + chrono::Duration::days(1),
        "weekly" => from + chrono::Duration::weeks(1),
        "monthly" => from + chrono::Duration::days(30),
        _ => from,
    }
}

/// Reset quota used if the current cycle has expired.
async fn reset_quota_if_expired(pool: &SqlitePool, quota: &Quota) -> Result<(), QuotaError> {
    if quota.cycle == "permanent" {
        return Ok(());
    }
    models::reset_quota_if_cycle_expired(pool, quota.id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_quota_costs_one_call() {
        assert_eq!(channel_quota_cost(Some("requests"), 12_345), 1);
    }

    #[test]
    fn token_quota_costs_tokens() {
        assert_eq!(channel_quota_cost(Some("tokens"), 12_345), 12_345);
        assert_eq!(channel_quota_cost(Some("tokens"), -5), 0);
    }

    #[test]
    fn unlimited_channel_never_costs() {
        assert_eq!(channel_quota_cost(None, 999), 0);
        assert_eq!(channel_quota_cost(Some(""), 999), 0);
    }

    #[test]
    fn permanent_cycles_have_no_period() {
        assert_eq!(compute_period(Some("permanent")), (None, None));
        assert_eq!(compute_period(None), (None, None));
    }

    #[test]
    fn cyclic_quotas_get_a_future_end() {
        for cycle in ["hourly", "daily", "weekly", "monthly"] {
            let (start, end) = compute_period(Some(cycle));
            let start = start.expect("start");
            let end = end.expect("end");
            assert!(end > start, "{cycle} must end after it starts");
            assert_eq!(start.offset(), &chrono::Utc);
        }
    }

    #[test]
    fn validators_reject_typos() {
        assert!(is_valid_cycle("daily"));
        assert!(!is_valid_cycle("day"));
        assert!(is_valid_quota_type("tokens"));
        assert!(!is_valid_quota_type("token"));
    }

    #[test]
    fn request_quota_allows_exactly_the_limit() {
        assert!(quota_allows("requests", 0, 2, 1));
        assert!(quota_allows("requests", 1, 2, 1));
        assert!(!quota_allows("requests", 2, 2, 1));
    }

    #[test]
    fn token_quota_blocks_once_usage_reaches_the_limit() {
        assert!(quota_allows("tokens", 0, 30, 18));
        assert!(quota_allows("tokens", 18, 30, 18));
        assert!(!quota_allows("tokens", 36, 30, 18));
    }

    #[test]
    fn unknown_quota_types_never_block() {
        assert!(quota_allows("", 100, 10, 1));
        assert!(quota_allows("bogus", 100, 10, 1));
    }
}
