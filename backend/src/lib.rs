pub mod config;
pub mod db;
pub mod middleware;
pub mod router;
pub mod service;
pub mod state;
pub mod utils;

use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::{RwLock, Semaphore};
use tracing_subscriber::{reload, EnvFilter};

use crate::config::AppConfig;
use crate::state::limiters::Limiters;
use crate::state::rate_limits::{RateLimitRule, RateLimitSnapshot};
use crate::utils::encrypt::KeyRing;

/// Global application state shared across all request handlers.
pub struct AppState {
    pub config: AppConfig,
    pub db: SqlitePool,
    /// Rate limiters and concurrency counters (global / per-user / per-channel).
    pub limiters: Limiters,
    /// Default HTTP client (no proxy).
    pub http_client: reqwest::Client,
    /// HTTP clients keyed by optional proxy URL (None = default client).
    pub http_clients: DashMap<Option<String>, reqwest::Client>,
    /// AES-256-GCM key ring for encrypting channel API keys at rest.
    pub encryption: KeyRing,
    /// Per-channel concurrency semaphores used for the channel queue.
    pub channel_semaphores: DashMap<i64, Arc<Semaphore>>,
    /// Round-robin cursors for channel API key selection.
    pub key_cursors: DashMap<i64, AtomicU64>,
    /// Per-IP rate limiters for the login endpoint.
    pub login_rate_limiters: DashMap<String, Arc<crate::state::limiters::DirectLimiter>>,
    /// Handle for runtime log level reloading.
    pub log_reload_handle: reload::Handle<EnvFilter, tracing_subscriber::Registry>,
    /// Log retention in days; updated at runtime by the settings API.
    pub retention_days: Arc<AtomicU32>,
    /// Cached rate limit configuration.
    pub rate_limits: RwLock<Arc<RateLimitSnapshot>>,
}

impl AppState {
    /// Reload rate limit rules from the database.
    pub async fn reload_rate_limits(&self) -> Result<(), sqlx::Error> {
        let snapshot = crate::state::rate_limits::load_snapshot(&self.db).await?;
        *self.rate_limits.write().await = Arc::new(snapshot);
        Ok(())
    }

    /// Current rate limit snapshot (cheap Arc clone).
    pub async fn rate_limit_snapshot(&self) -> Arc<RateLimitSnapshot> {
        self.rate_limits.read().await.clone()
    }

    /// Global rule: database config wins, config.yaml is the fallback.
    pub async fn global_rate_rule(&self) -> RateLimitRule {
        if let Some(rule) = self.rate_limit_snapshot().await.global.clone() {
            return rule;
        }
        RateLimitRule {
            qps: self.config.global_rate_limit.qps,
            concurrency: self.config.global_rate_limit.concurrency,
            action: self.config.global_rate_limit.action.clone(),
        }
    }

    /// Next round-robin cursor for a channel's API keys.
    pub fn next_key_cursor(&self, channel_id: i64) -> u64 {
        let counter = self
            .key_cursors
            .entry(channel_id)
            .or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Current log retention in days.
    pub fn retention_days(&self) -> u32 {
        self.retention_days
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
