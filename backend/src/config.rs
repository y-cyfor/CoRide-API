use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub admin: AdminConfig,
    pub jwt: JwtConfig,
    pub log: LogConfig,
    pub proxy: ProxyConfig,
    pub global_rate_limit: GlobalRateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub cors_allowed_origins: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub retention_days: u32,
    pub max_records: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxyConfig {
    pub timeout: u64,
    pub max_retries: u32,
    pub log_request_body: bool,
    pub log_response_body: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalRateLimitConfig {
    pub qps: u32,
    pub concurrency: u32,
    pub action: String,
}

/// Load configuration from `config/config.yaml` with environment variable overrides.
///
/// Environment variables (all prefixed with `CORIDE_` or legacy `LP_`):
/// - `CORIDE_PORT` / `LP_PORT`          -- server port
/// - `CORIDE_DB_PATH` / `LP_DB_PATH`    -- database file path
/// - `CORIDE_ADMIN_USERNAME` / `LP_ADMIN_USERNAME` -- admin username
/// - `CORIDE_ADMIN_PASSWORD` / `LP_ADMIN_PASSWORD` -- admin password
/// - `CORIDE_JWT_SECRET` / `LP_JWT_SECRET`    -- JWT signing secret
/// - `CORIDE_LOG_LEVEL` / `LP_LOG_LEVEL`     -- log level
pub async fn load() -> AppConfig {
    dotenvy::dotenv().ok();

    let config_path = "config/config.yaml";
    let raw = tokio::fs::read_to_string(config_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read config file '{config_path}': {e}"));

    let mut config: AppConfig =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("Failed to parse config YAML: {e}"));

    // Helper: prefer CORIDE_ prefix, fall back to LP_ prefix
    let env = |coride_key: &str, lp_key: &str| -> Option<String> {
        env::var(coride_key).ok().or_else(|| env::var(lp_key).ok())
    };

    // Environment variable override
    if let Some(port) = env("CORIDE_PORT", "LP_PORT") {
        config.server.port = port.parse().expect("CORIDE_PORT must be a valid u16");
    }
    if let Some(db_path) = env("CORIDE_DB_PATH", "LP_DB_PATH") {
        config.database.path = db_path;
    }
    if let Some(username) = env("CORIDE_ADMIN_USERNAME", "LP_ADMIN_USERNAME") {
        config.admin.username = username;
    }
    if let Some(password) = env("CORIDE_ADMIN_PASSWORD", "LP_ADMIN_PASSWORD") {
        config.admin.password = password;
    }
    if let Some(secret) = env("CORIDE_JWT_SECRET", "LP_JWT_SECRET") {
        config.jwt.secret = secret;
    }
    if let Some(level) = env("CORIDE_LOG_LEVEL", "LP_LOG_LEVEL") {
        config.log.level = level;
    }

    config
}
