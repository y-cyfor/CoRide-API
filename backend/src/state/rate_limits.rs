use std::collections::HashMap;

use sqlx::SqlitePool;

/// One rate limit rule loaded from `rate_limit_configs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRule {
    pub qps: u32,
    pub concurrency: u32,
    pub action: String,
}

impl RateLimitRule {
    pub fn should_queue(&self) -> bool {
        self.action == "queue"
    }
}

/// Snapshot of every configured rate limit.
///
/// The proxy path reads this snapshot (cheap `RwLock<Arc<_>>` read) instead of hitting
/// the database on every request. It is refreshed after admin edits and periodically.
#[derive(Debug, Clone, Default)]
pub struct RateLimitSnapshot {
    pub global: Option<RateLimitRule>,
    pub channels: HashMap<i64, RateLimitRule>,
    pub users: HashMap<i64, RateLimitRule>,
}

impl RateLimitSnapshot {
    pub fn channel(&self, channel_id: i64) -> Option<&RateLimitRule> {
        self.channels.get(&channel_id)
    }

    pub fn user(&self, user_id: i64) -> Option<&RateLimitRule> {
        self.users.get(&user_id)
    }
}

/// Load all rate limit rules.
///
/// Invalid rows (negative qps, unknown target type) are ignored with a warning instead
/// of failing the whole refresh.
pub async fn load_snapshot(pool: &SqlitePool) -> Result<RateLimitSnapshot, sqlx::Error> {
    let rows: Vec<(String, Option<i64>, i32, i32, String)> = sqlx::query_as(
        "SELECT target_type, target_id, qps, concurrency, action FROM rate_limit_configs",
    )
    .fetch_all(pool)
    .await?;

    let mut snapshot = RateLimitSnapshot::default();
    for (target_type, target_id, qps, concurrency, action) in rows {
        let rule = RateLimitRule {
            qps: qps.max(0) as u32,
            concurrency: concurrency.max(0) as u32,
            action: if action == "queue" { "queue".into() } else { "reject".into() },
        };

        match target_type.as_str() {
            "global" => snapshot.global = Some(rule),
            "channel" => match target_id {
                Some(id) => {
                    snapshot.channels.insert(id, rule);
                }
                None => tracing::warn!("Ignoring channel rate limit without target_id"),
            },
            "user" => match target_id {
                Some(id) => {
                    snapshot.users.insert(id, rule);
                }
                None => tracing::warn!("Ignoring user rate limit without target_id"),
            },
            other => tracing::warn!(target_type = other, "Ignoring unknown rate limit target"),
        }
    }

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_action_is_detected() {
        let rule = RateLimitRule {
            qps: 1,
            concurrency: 1,
            action: "queue".into(),
        };
        assert!(rule.should_queue());

        let reject = RateLimitRule {
            action: "reject".into(),
            ..rule
        };
        assert!(!reject.should_queue());
    }

    #[test]
    fn snapshot_lookups_are_scoped() {
        let mut snapshot = RateLimitSnapshot::default();
        snapshot.channels.insert(
            1,
            RateLimitRule {
                qps: 5,
                concurrency: 2,
                action: "reject".into(),
            },
        );
        assert_eq!(snapshot.channel(1).map(|r| r.qps), Some(5));
        assert!(snapshot.channel(2).is_none());
        assert!(snapshot.user(1).is_none());
    }
}
