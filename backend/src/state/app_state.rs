use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorLimiter,
};

use crate::config::AppConfig;
use crate::AppState;

/// Build the global rate limiter from config.
pub fn build_global_qps_limiter(cfg: &AppConfig) -> Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>> {
    let qps = if cfg.global_rate_limit.qps > 0 {
        cfg.global_rate_limit.qps as u32
    } else {
        u32::MAX
    };
    let quota = Quota::per_second(qps.try_into().unwrap());
    Arc::new(
        GovernorLimiter::<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>::direct(quota),
    )
}

/// Build the full application state.
pub async fn build(cfg: AppConfig, db: sqlx::SqlitePool) -> Arc<AppState> {
    let global_qps_limiter = build_global_qps_limiter(&cfg);
    let http_client = reqwest::Client::new();

    let encryption_key = crate::utils::encrypt::derive_key(&cfg.jwt.secret);

    Arc::new(AppState {
        config: cfg,
        db,
        global_qps_limiter,
        channel_rate_limiters: DashMap::new(),
        user_rate_limiters: DashMap::new(),
        http_client,
        login_rate_limiters: DashMap::new(),
        encryption_key,
    })
}

/// Create or replace a per-channel QPS limiter.
/// If `qps` is 0 the entry is removed (unlimited).
pub fn upsert_channel_limiter(
    limiters: &DashMap<i64, Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    channel_id: i64,
    qps: u32,
) {
    if qps == 0 {
        limiters.remove(&channel_id);
        return;
    }
    let non_zero = std::num::NonZeroU32::new(qps)
        .unwrap_or_else(|| std::num::NonZeroU32::new(1).unwrap());
    let limiter = Arc::new(GovernorLimiter::direct(Quota::per_second(non_zero)));
    limiters.insert(channel_id, limiter);
}

/// Create or replace a per-user QPS limiter.
/// If `qps` is 0 the entry is removed (unlimited).
pub fn upsert_user_limiter(
    limiters: &DashMap<String, Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    user_key: String,
    qps: u32,
) {
    if qps == 0 {
        limiters.remove(&user_key);
        return;
    }
    let non_zero = std::num::NonZeroU32::new(qps)
        .unwrap_or_else(|| std::num::NonZeroU32::new(1).unwrap());
    let limiter = Arc::new(GovernorLimiter::direct(Quota::per_second(non_zero)));
    limiters.insert(user_key, limiter);
}

/// If the quota cycle has expired, reset `used` to 0 and advance the period.
///
/// Returns the new `period_end` if a reset occurred, `None` otherwise.
/// This is a pure function — the caller is responsible for persisting
/// the updated values back to the database.
pub fn reset_quota_if_expired(
    used: &mut i64,
    cycle: &str,
    period_end: &mut Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    if cycle == "permanent" {
        return None;
    }

    let now = Utc::now();
    if let Some(end) = *period_end {
        if now >= end {
            *used = 0;
            let new_end = match cycle {
                "hourly" => now + chrono::Duration::hours(1),
                "daily" => now + chrono::Duration::days(1),
                "weekly" => now + chrono::Duration::weeks(1),
                "monthly" => now + chrono::Duration::days(30),
                _ => return None,
            };
            *period_end = Some(new_end);
            return Some(new_end);
        }
    }

    None
}
