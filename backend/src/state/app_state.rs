use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use dashmap::DashMap;
use tracing_subscriber::{reload, EnvFilter};

use crate::config::AppConfig;
use crate::state::limiters::Limiters;
use crate::utils::encrypt::KeyRing;
use crate::AppState;

/// Build the full application state.
pub async fn build(
    cfg: AppConfig,
    db: sqlx::SqlitePool,
    log_reload_handle: reload::Handle<EnvFilter, tracing_subscriber::Registry>,
) -> Arc<AppState> {
    let encryption = KeyRing::from_secrets(&cfg.encryption_secrets())
        .unwrap_or_else(|| KeyRing::from_secret("coride-default-encryption-secret"));
    if encryption.key_count() > 1 {
        tracing::info!(
            keys = encryption.key_count(),
            "Loaded multiple encryption secrets for key rotation"
        );
    }

    // HTTP clients: default (no proxy)
    let http_clients = DashMap::new();
    http_clients.insert(None, reqwest::Client::new());
    let default_client = http_clients.get(&None).unwrap().clone();

    let retention_days = Arc::new(AtomicU32::new(cfg.log.retention_days));

    let state = Arc::new(AppState {
        config: cfg,
        db,
        limiters: Limiters::new(),
        http_client: default_client,
        http_clients,
        encryption,
        channel_semaphores: DashMap::new(),
        key_cursors: DashMap::new(),
        login_rate_limiters: DashMap::new(),
        log_reload_handle,
        retention_days,
        rate_limits: tokio::sync::RwLock::new(Arc::default()),
    });

    if let Err(e) = state.reload_rate_limits().await {
        tracing::warn!(error = %e, "Failed to load rate limit configuration");
    }

    state
}
