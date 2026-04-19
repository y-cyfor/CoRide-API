pub mod config;
pub mod db;
pub mod middleware;
pub mod router;
pub mod service;
pub mod state;
pub mod utils;

use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    RateLimiter as GovernorLimiter,
};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::config::AppConfig;

/// Global application state shared across all request handlers.
pub struct AppState {
    pub config: AppConfig,
    pub db: SqlitePool,
    pub global_qps_limiter: Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    pub channel_rate_limiters: DashMap<i64, Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    pub user_rate_limiters: DashMap<String, Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    pub http_client: reqwest::Client,
    /// Per-IP rate limiters for login endpoint (5 attempts per minute).
    pub login_rate_limiters: DashMap<String, Arc<GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>>,
    /// AES-256-GCM key for encrypting channel API keys at rest (derived from JWT secret).
    pub encryption_key: [u8; 32],
}
