use std::sync::Arc;

use axum::{
    extract::State,
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use coride_api::config;
use coride_api::db::{self, models};
use coride_api::middleware::{admin_auth, auth, ip_filter, rate_limit, user_auth};
use coride_api::router::{admin_routes, proxy_routes};
use coride_api::state::app_state as state_builder;
use coride_api::AppState;

#[tokio::main]
async fn main() {
    // 1. Load configuration
    let cfg = config::load().await;

    // 2. Initialize logging (stdout + daily rotating file)
    let log_level = cfg.log.level.clone();
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("coride_api={log_level}")));

    // Create log directory
    std::fs::create_dir_all("log").ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "log", "coride-api.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::Layer::default().with_target(false))
        .with(
            tracing_subscriber::fmt::Layer::default()
                .with_target(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!(port = cfg.server.port, "Starting CoRide-API");

    // 3. Initialize database pool (directory + file creation handled by init_pool)
    let db_pool = db::init_pool(&cfg.database.path, cfg.database.pool_size).await;

    // 4. Run migrations
    db::run_migrations(&db_pool).await;

    // 5. Initialize system app profiles
    models::init_system_app_profiles(&db_pool).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Failed to initialize app profiles");
    });

    // 6. Ensure admin user exists
    db::ensure_admin(&db_pool, &cfg.admin.username, &cfg.admin.password).await;

    // 7. Build application state
    let state = state_builder::build(cfg.clone(), db_pool.clone()).await;

    // 8. Build proxy routes
    let proxy_routes = Router::new()
        .route("/v1/chat/completions", post(proxy_routes::chat_completions))
        .route("/v1/completions", post(proxy_routes::completions))
        .route("/v1/messages", post(proxy_routes::messages))
        .route("/v1/models", get(proxy_routes::list_models))
        .route("/v1/user/info", get(proxy_routes::user_info))
        .layer(from_fn_with_state(state.clone(), ip_filter::ip_filter))
        .layer(from_fn_with_state(state.clone(), rate_limit::rate_limit_middleware))
        .layer(from_fn_with_state(state.clone(), auth::auth_middleware));

    // 9. Build admin routes
    let admin_public = Router::new()
        // Auth (public)
        .route("/admin/auth/login", post(admin_routes::login));

    // User-facing routes (JWT auth, not admin-only)
    let user_routes = Router::new()
        .route("/user/info", get(admin_routes::get_user_info))
        .route("/user/keys", get(admin_routes::list_user_keys))
        .route("/user/keys", post(admin_routes::create_user_key))
        .route("/user/keys/{id}", put(admin_routes::update_user_key))
        .route("/user/keys/{id}", delete(admin_routes::delete_user_key))
        .route("/user/stats/dashboard", get(admin_routes::user_dashboard_stats))
        .route("/user/stats/usage", get(admin_routes::user_usage_stats))
        .route("/user/logs", get(admin_routes::user_logs))
        .layer(from_fn_with_state(state.clone(), user_auth::user_auth_middleware));

    let admin_protected = Router::new()
        // Auth (protected)
        .route("/admin/auth/me", get(admin_routes::get_me))
        // Users
        .route("/admin/users", get(admin_routes::list_users))
        .route("/admin/users", post(admin_routes::create_user))
        .route("/admin/users/{id}", put(admin_routes::update_user))
        .route("/admin/users/{id}/reset-key", post(admin_routes::reset_user_key))
        .route("/admin/users/{id}", delete(admin_routes::delete_user_endpoint))
        // Channels
        .route("/admin/channels", get(admin_routes::list_channels))
        .route("/admin/channels", post(admin_routes::create_channel))
        .route("/admin/channels/{id}", put(admin_routes::update_channel))
        .route("/admin/channels/{id}", delete(admin_routes::delete_channel_endpoint))
        .route("/admin/channels/{id}/test", post(admin_routes::test_channel))
        .route("/admin/channels/{id}/stats", get(admin_routes::channel_stats))
        // Models
        .route("/admin/models", get(admin_routes::list_models_endpoint))
        .route("/admin/models", post(admin_routes::create_model_endpoint))
        .route("/admin/models/{id}", put(admin_routes::update_model))
        .route("/admin/models/{id}", delete(admin_routes::delete_model_endpoint))
        // Quotas
        .route("/admin/quotas", get(admin_routes::list_quotas))
        .route("/admin/quotas", post(admin_routes::create_quota))
        .route("/admin/quotas/{id}", put(admin_routes::update_quota))
        .route("/admin/quotas/{id}", delete(admin_routes::delete_quota))
        // Rate limits
        .route("/admin/ratelimits", get(admin_routes::list_ratelimits))
        .route("/admin/ratelimits", post(admin_routes::create_ratelimit))
        .route("/admin/ratelimits/{id}", put(admin_routes::update_ratelimit))
        .route("/admin/ratelimits/{id}", delete(admin_routes::delete_ratelimit))
        // App profiles
        .route("/admin/app-profiles", get(admin_routes::list_app_profiles))
        .route("/admin/app-profiles", post(admin_routes::create_app_profile))
        .route("/admin/app-profiles/{id}", put(admin_routes::update_app_profile))
        .route("/admin/app-profiles/{id}", delete(admin_routes::delete_app_profile_endpoint))
        // System config
        .route("/admin/system/config", get(admin_routes::get_system_config))
        .route("/admin/system/config/log-level", put(admin_routes::set_log_level))
        .route("/admin/system/config/rate-limit", put(admin_routes::set_global_rate_limit))
        // Quota warnings
        .route("/admin/quotas/warnings", get(admin_routes::quota_warnings))
        // Stats
        .route("/admin/stats/dashboard", get(admin_routes::dashboard_stats))
        .route("/admin/stats/usage", get(admin_routes::usage_stats))
        .route("/admin/logs/export", get(admin_routes::export_logs_csv))
        // Logs
        .route("/admin/logs", get(admin_routes::list_logs))
        // User keys (admin view)
        .route("/admin/keys", get(admin_routes::list_all_user_keys))
        .route("/admin/keys/{user_id}", get(admin_routes::list_user_keys_by_user_id))
        // Traffic plans
        .route("/admin/traffic-plan/global", get(admin_routes::get_global_traffic_plan))
        .route("/admin/traffic-plan/global", put(admin_routes::upsert_global_traffic_plan))
        .route("/admin/traffic-plan/global", delete(admin_routes::delete_global_traffic_plan))
        .route("/admin/traffic-plan/channels", get(admin_routes::list_channel_traffic_plans))
        .route("/admin/traffic-plan/channel/{id}", get(admin_routes::get_channel_traffic_plan))
        .route("/admin/traffic-plan/channel/{id}", put(admin_routes::upsert_channel_traffic_plan))
        .route("/admin/traffic-plan/channel/{id}", delete(admin_routes::delete_channel_traffic_plan))
        // IP access control
        .route("/admin/ip/blacklist", get(admin_routes::list_blacklist))
        .route("/admin/ip/blacklist", post(admin_routes::add_blacklist))
        .route("/admin/ip/blacklist/{id}", delete(admin_routes::delete_blacklist))
        .route("/admin/ip/whitelist/user/{user_id}", get(admin_routes::list_user_whitelist))
        .route("/admin/ip/whitelist", post(admin_routes::add_whitelist))
        .route("/admin/ip/whitelist/entry/{id}", delete(admin_routes::delete_whitelist))
        .layer(from_fn_with_state(state.clone(), admin_auth::admin_auth_middleware));

    // 10. Combine all routes
    let app = Router::new()
        .route("/health", get(health_check))
        .merge(proxy_routes)
        .merge(admin_public)
        .merge(user_routes)
        .merge(admin_protected)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ]))
        .with_state(state.clone());

    // 11. Start log cleanup background task
    let cleanup_pool = db_pool.clone();
    let retention_days = cfg.log.retention_days as i32;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Run every hour
        loop {
            interval.tick().await;
            match models::cleanup_old_logs(&cleanup_pool, retention_days).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        tracing::info!(deleted = deleted, "Cleaned up old request logs");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to clean up old request logs");
                }
            }
        }
    });

    // 12. Start channel health check background task
    let health_pool = db_pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // Run every 5 minutes
        loop {
            interval.tick().await;
            match coride_api::service::health::check_channels_health(&health_pool).await {
                Ok((disabled, recovered)) => {
                    if !disabled.is_empty() {
                        tracing::warn!(channels = ?disabled, "Disabled unhealthy channels");
                    }
                    if !recovered.is_empty() {
                        tracing::info!(channels = ?recovered, "Recovered healthy channels");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to check channel health");
                }
            }
        }
    });

    // 13. Start server with graceful shutdown
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await
        .unwrap_or_else(|e| panic!("Failed to bind to {addr}: {e}"));

    tracing::info!(%addr, "CoRide-API server listening");

    // Graceful shutdown: listen for Ctrl+C (works on all platforms)
    let shutdown_signal = async {
        tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        tracing::info!("Shutting down gracefully...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();
}

async fn health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1")
        .fetch_optional(&state.db)
        .await
        .is_ok();

    if db_ok {
        Json(serde_json::json!({
            "status": "ok",
            "database": "connected",
        }))
    } else {
        Json(serde_json::json!({
            "status": "degraded",
            "database": "disconnected",
        }))
    }
}
