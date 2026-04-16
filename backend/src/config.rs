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
/// Environment variables (all prefixed with `LP_`):
/// - `LP_PORT`          -- server port
/// - `LP_DB_PATH`       -- database file path
/// - `LP_ADMIN_USERNAME`-- admin username
/// - `LP_ADMIN_PASSWORD`-- admin password
/// - `LP_JWT_SECRET`    -- JWT signing secret
/// - `LP_LOG_LEVEL`     -- log level (trace/debug/info/warn/error)
pub async fn load() -> AppConfig {
    dotenvy::dotenv().ok();

    let config_path = "config/config.yaml";
    let raw = tokio::fs::read_to_string(config_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read config file '{config_path}': {e}"));

    let mut config: AppConfig =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("Failed to parse config YAML: {e}"));

    // Environment variable overrides
    if let Ok(port) = env::var("LP_PORT") {
        config.server.port = port.parse().expect("LP_PORT must be a valid u16");
    }
    if let Ok(db_path) = env::var("LP_DB_PATH") {
        config.database.path = db_path;
    }
    if let Ok(username) = env::var("LP_ADMIN_USERNAME") {
        config.admin.username = username;
    }
    if let Ok(password) = env::var("LP_ADMIN_PASSWORD") {
        config.admin.password = password;
    }
    if let Ok(secret) = env::var("LP_JWT_SECRET") {
        config.jwt.secret = secret;
    }
    if let Ok(level) = env::var("LP_LOG_LEVEL") {
        config.log.level = level;
    }

    config
}
