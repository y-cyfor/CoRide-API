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
    pub channel_queue: ChannelQueueConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    #[serde(default)]
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
    #[serde(default)]
    pub max_records: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_timeout_ms")]
    pub timeout: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default)]
    pub log_request_body: bool,
    #[serde(default)]
    pub log_response_body: bool,
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalRateLimitConfig {
    pub qps: u32,
    pub concurrency: u32,
    #[serde(default = "default_action")]
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelQueueConfig {
    #[serde(default = "default_channel_concurrency")]
    pub max_concurrent: u32,
    #[serde(default = "default_channel_queue_retries")]
    pub max_retries: u32,
    #[serde(default = "default_delay_min")]
    pub delay_min_ms: u64,
    #[serde(default = "default_delay_max")]
    pub delay_max_ms: u64,
}

/// Encryption settings for channel API keys at rest.
#[derive(Debug, Clone, Deserialize)]
#[derive(Default)]
pub struct SecurityConfig {
    /// Secrets used to derive AES-256-GCM keys. The first entry encrypts new values;
    /// every entry is tried when decrypting, which allows rotating the secret without
    /// losing existing data. Empty => fall back to `jwt.secret` (legacy behaviour).
    #[serde(default)]
    pub encryption_secrets: Vec<String>,
}


/// Background channel health probing.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_health_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: i32,
    #[serde(default = "default_health_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: default_health_interval_secs(),
            failure_threshold: default_failure_threshold(),
            timeout_secs: default_health_timeout_secs(),
        }
    }
}

impl Default for ChannelQueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: default_channel_concurrency(),
            max_retries: default_channel_queue_retries(),
            delay_min_ms: default_delay_min(),
            delay_max_ms: default_delay_max(),
        }
    }
}

fn default_proxy_timeout_ms() -> u64 {
    300_000
}
fn default_max_retries() -> u32 {
    1
}
fn default_action() -> String {
    "reject".to_string()
}
fn default_channel_concurrency() -> u32 {
    2
}
fn default_channel_queue_retries() -> u32 {
    5
}
fn default_delay_min() -> u64 {
    100
}
fn default_delay_max() -> u64 {
    2_000
}
fn default_true() -> bool {
    true
}
fn default_health_interval_secs() -> u64 {
    300
}
fn default_failure_threshold() -> i32 {
    3
}
fn default_health_timeout_secs() -> u64 {
    10
}

impl AppConfig {
    /// Secrets used for encrypting upstream API keys, in priority order.
    ///
    /// Falls back to the JWT secret so that existing deployments keep working.
    pub fn encryption_secrets(&self) -> Vec<String> {
        if self.security.encryption_secrets.is_empty() {
            vec![self.jwt.secret.clone()]
        } else {
            self.security.encryption_secrets.clone()
        }
    }

    /// Longest upstream timeout permitted for a single request, in milliseconds.
    pub fn max_proxy_timeout_ms(&self) -> u64 {
        self.proxy.timeout.max(1_000)
    }
}

// LP_ prefix is deprecated, use CORIDE_ instead

/// Load configuration from `config/config.yaml` with environment variable overrides.
///
/// Environment variables (all prefixed with `CORIDE_` or legacy `LP_`):
/// - `CORIDE_PORT` / `LP_PORT`          -- server port
/// - `CORIDE_DB_PATH` / `LP_DB_PATH`    -- database file path
/// - `CORIDE_ADMIN_USERNAME` / `LP_ADMIN_USERNAME` -- admin username
/// - `CORIDE_ADMIN_PASSWORD` / `LP_ADMIN_PASSWORD` -- admin password
/// - `CORIDE_JWT_SECRET` / `LP_JWT_SECRET`    -- JWT signing secret
/// - `CORIDE_LOG_LEVEL` / `LP_LOG_LEVEL`     -- log level
/// - `CORIDE_ENCRYPTION_SECRETS`             -- comma-separated key ring for API key encryption
/// - `CORIDE_TRUSTED_PROXIES`                -- comma-separated proxy IPs allowed to set X-Real-IP
/// - `CORIDE_HEALTH_CHECK_ENABLED`           -- `true` / `false`
pub async fn load() -> AppConfig {
    load_from("config/config.yaml").await
}

/// Same as [`load`] but with an explicit path (used by tests).
pub async fn load_from(config_path: &str) -> AppConfig {
    dotenvy::dotenv().ok();

    let raw = tokio::fs::read_to_string(config_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read config file '{config_path}': {e}"));

    let mut config: AppConfig =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("Failed to parse config YAML: {e}"));

    // Helper: prefer CORIDE_ prefix, fall back to LP_ prefix
    // LP_ prefix is deprecated, use CORIDE_ instead
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
    if let Some(secrets) = env("CORIDE_ENCRYPTION_SECRETS", "LP_ENCRYPTION_SECRETS") {
        config.security.encryption_secrets = secrets
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(proxies) = env("CORIDE_TRUSTED_PROXIES", "LP_TRUSTED_PROXIES") {
        config.proxy.trusted_proxies = proxies
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(enabled) = env("CORIDE_HEALTH_CHECK_ENABLED", "LP_HEALTH_CHECK_ENABLED") {
        config.health_check.enabled = matches!(
            enabled.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
server:
  port: 8000
  host: "0.0.0.0"
database:
  path: "./data/coride.db"
  pool_size: 10
admin:
  username: "admin"
  password: "admin123"
jwt:
  secret: "secret"
  expires_in: 86400
log:
  level: "info"
  retention_days: 30
proxy:
  timeout: 300000
  max_retries: 1
  log_request_body: false
  log_response_body: false
global_rate_limit:
  qps: 100
  concurrency: 50
  action: "reject"
channel_queue:
  max_concurrent: 2
  max_retries: 5
  delay_min_ms: 100
  delay_max_ms: 2000
"#;

    #[test]
    fn old_config_without_new_sections_still_parses() {
        let cfg: AppConfig = serde_yaml::from_str(MINIMAL).expect("legacy config must parse");
        assert!(cfg.security.encryption_secrets.is_empty());
        assert!(cfg.health_check.enabled);
        assert_eq!(cfg.health_check.failure_threshold, 3);
        // Legacy behaviour: fall back to the JWT secret.
        assert_eq!(cfg.encryption_secrets(), vec!["secret".to_string()]);
    }

    #[test]
    fn explicit_encryption_secrets_win() {
        let mut cfg: AppConfig = serde_yaml::from_str(MINIMAL).unwrap();
        cfg.security.encryption_secrets = vec!["a".into(), "b".into()];
        assert_eq!(cfg.encryption_secrets(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn health_check_config_is_overridable() {
        let yaml = format!("{MINIMAL}\nhealth_check:\n  enabled: false\n  interval_secs: 30\n  failure_threshold: 5\n  timeout_secs: 3\n");
        let cfg: AppConfig = serde_yaml::from_str(&yaml).unwrap();
        assert!(!cfg.health_check.enabled);
        assert_eq!(cfg.health_check.interval_secs, 30);
        assert_eq!(cfg.health_check.failure_threshold, 5);
        assert_eq!(cfg.health_check.timeout_secs, 3);
    }
}
