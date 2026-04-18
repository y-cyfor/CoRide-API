use serde::Serialize;
use sqlx::SqlitePool;
use chrono::Timelike;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip)]
    pub password_hash: String,
    pub role: String,
    pub api_key: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_models: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Channel {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub base_url: String,
    pub api_keys: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_headers: Option<String>,
    pub status: String,
    pub weight: i32,
    pub timeout: i32,
    pub retry_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_limit: Option<i64>,
    pub quota_used: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_cycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_period_start: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_period_end: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_profile_id: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Model {
    pub id: i64,
    pub channel_id: i64,
    pub source_name: String,
    pub proxy_name: String,
    pub enabled: bool,
    #[sqlx(rename = "is_default")]
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct AppProfile {
    pub id: i64,
    pub name: String,
    pub identifier: String,
    pub user_agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Quota {
    pub id: i64,
    pub user_id: i64,
    pub quota_type: String,
    pub total_limit: i64,
    pub used: i64,
    pub cycle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RateLimitConfig {
    pub id: i64,
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<i64>,
    pub qps: i32,
    pub concurrency: i32,
    pub action: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RequestLog {
    pub id: i64,
    pub user_api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<i64>,
    pub model: String,
    pub endpoint: String,
    pub status_code: i32,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub elapsed_ms: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// === User CRUD ===

pub async fn get_user_by_api_key(pool: &SqlitePool, api_key: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE api_key = ? AND status = 'active'")
        .bind(api_key)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_by_id(pool: &SqlitePool, id: i64) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<User>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_users(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<Vec<User>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password_hash: &str,
    role: &str,
    api_key: &str,
    status: &str,
    enabled_models: Option<&str>,
    note: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, role, api_key, status, enabled_models, note) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(api_key)
    .bind(status)
    .bind(enabled_models)
    .bind(note)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_user(
    pool: &SqlitePool,
    id: i64,
    username: &str,
    role: &str,
    status: &str,
    enabled_models: Option<&str>,
    note: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET username = ?, role = ?, status = ?, enabled_models = ?, note = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(username)
    .bind(role)
    .bind(status)
    .bind(enabled_models)
    .bind(note)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_user_status(pool: &SqlitePool, id: i64, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_user_api_key(pool: &SqlitePool, id: i64, new_key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET api_key = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(new_key)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// === Channel CRUD ===

pub async fn get_active_channels(pool: &SqlitePool) -> Result<Vec<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE status = 'active'")
        .fetch_all(pool)
        .await
}

pub async fn get_channel_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_channel_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<Channel>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_channels(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<Vec<Channel>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn create_channel(
    pool: &SqlitePool,
    name: &str,
    channel_type: &str,
    base_url: &str,
    api_keys: &str,
    custom_headers: Option<&str>,
    weight: i32,
    timeout: i32,
    retry_count: i32,
    quota_type: Option<&str>,
    quota_limit: Option<i64>,
    quota_cycle: Option<&str>,
    app_profile_id: Option<i64>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO channels (name, type, base_url, api_keys, custom_headers, weight, timeout, retry_count, quota_type, quota_limit, quota_cycle, app_profile_id, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active')",
    )
    .bind(name)
    .bind(channel_type)
    .bind(base_url)
    .bind(api_keys)
    .bind(custom_headers)
    .bind(weight)
    .bind(timeout)
    .bind(retry_count)
    .bind(quota_type)
    .bind(quota_limit)
    .bind(quota_cycle)
    .bind(app_profile_id)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_channel(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    channel_type: &str,
    base_url: &str,
    api_keys: &str,
    custom_headers: Option<&str>,
    weight: i32,
    timeout: i32,
    retry_count: i32,
    quota_type: Option<&str>,
    quota_limit: Option<i64>,
    quota_cycle: Option<&str>,
    app_profile_id: Option<i64>,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE channels SET name=?, type=?, base_url=?, api_keys=?, custom_headers=?, weight=?, timeout=?, retry_count=?, quota_type=?, quota_limit=?, quota_cycle=?, app_profile_id=?, status=?, updated_at=CURRENT_TIMESTAMP WHERE id=?",
    )
    .bind(name)
    .bind(channel_type)
    .bind(base_url)
    .bind(api_keys)
    .bind(custom_headers)
    .bind(weight)
    .bind(timeout)
    .bind(retry_count)
    .bind(quota_type)
    .bind(quota_limit)
    .bind(quota_cycle)
    .bind(app_profile_id)
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_channel(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get a channel's API key by round-robin based on current quota_used.
pub fn get_channel_api_key(channel: &Channel) -> Option<String> {
    let keys: Vec<String> = serde_json::from_str(&channel.api_keys).ok()?;
    if keys.is_empty() {
        return None;
    }
    let idx = (channel.quota_used as usize) % keys.len();
    Some(keys[idx].clone())
}

// === Model CRUD ===

pub async fn get_model_by_proxy_name(pool: &SqlitePool, proxy_name: &str) -> Result<Option<Model>, sqlx::Error> {
    sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE proxy_name = ? AND enabled = 1",
    )
    .bind(proxy_name)
    .fetch_optional(pool)
    .await
}

pub async fn get_model_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Model>, sqlx::Error> {
    sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_models_by_channel(pool: &SqlitePool, channel_id: i64) -> Result<Vec<Model>, sqlx::Error> {
    sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE channel_id = ? ORDER BY created_at",
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
}

pub async fn get_model_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<Model>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, Model>(
        "SELECT * FROM models ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_models(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<Vec<Model>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, Model>(
        "SELECT * FROM models ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn create_model(
    pool: &SqlitePool,
    channel_id: i64,
    source_name: &str,
    proxy_name: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO models (channel_id, source_name, proxy_name, enabled, is_default) VALUES (?, ?, ?, true, false)",
    )
    .bind(channel_id)
    .bind(source_name)
    .bind(proxy_name)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_model(
    pool: &SqlitePool,
    id: i64,
    source_name: &str,
    proxy_name: &str,
    enabled: bool,
    is_default: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE models SET source_name=?, proxy_name=?, enabled=?, is_default=? WHERE id=?",
    )
    .bind(source_name)
    .bind(proxy_name)
    .bind(enabled)
    .bind(is_default)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_model(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM models WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Find active channels that have a model matching the given proxy name.
pub async fn find_channels_for_model(
    pool: &SqlitePool,
    proxy_name: &str,
) -> Result<Vec<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>(
        "SELECT c.* FROM channels c
         INNER JOIN models m ON m.channel_id = c.id
         WHERE m.proxy_name = ? AND m.enabled = 1 AND c.status = 'active'
         ORDER BY c.weight DESC",
    )
    .bind(proxy_name)
    .fetch_all(pool)
    .await
}

/// Alias for find_channels_for_model -- get channels serving a given model name.
pub async fn get_channel_by_model(
    pool: &SqlitePool,
    model_name: &str,
) -> Result<Vec<Channel>, sqlx::Error> {
    find_channels_for_model(pool, model_name).await
}

// === Quota CRUD ===

pub async fn get_active_quotas(pool: &SqlitePool, user_id: i64) -> Result<Vec<Quota>, sqlx::Error> {
    sqlx::query_as::<_, Quota>(
        "SELECT * FROM quotas WHERE user_id = ? AND enabled = true ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_quota_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Quota>, sqlx::Error> {
    sqlx::query_as::<_, Quota>("SELECT * FROM quotas WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_quota_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<Quota>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, Quota>(
        "SELECT * FROM quotas ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_quotas(
    pool: &SqlitePool,
    user_id: Option<i64>,
    page: i64,
    page_size: i64,
) -> Result<Vec<Quota>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    if let Some(uid) = user_id {
        sqlx::query_as::<_, Quota>(
            "SELECT * FROM quotas WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(uid)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Quota>(
            "SELECT * FROM quotas ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}

pub async fn create_quota(
    pool: &SqlitePool,
    user_id: i64,
    quota_type: &str,
    total_limit: i64,
    cycle: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO quotas (user_id, quota_type, total_limit, cycle, enabled) VALUES (?, ?, ?, ?, true)",
    )
    .bind(user_id)
    .bind(quota_type)
    .bind(total_limit)
    .bind(cycle)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_quota(
    pool: &SqlitePool,
    id: i64,
    total_limit: i64,
    enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE quotas SET total_limit=?, enabled=? WHERE id=?")
        .bind(total_limit)
        .bind(enabled)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_quota(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM quotas WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn increment_quota_used(pool: &SqlitePool, quota_id: i64, amount: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE quotas SET used = used + ? WHERE id = ?")
        .bind(amount)
        .bind(quota_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reset_quota_if_cycle_expired(pool: &SqlitePool, quota_id: i64) -> Result<(), sqlx::Error> {
    // Reset used to 0 if the current cycle has passed
    sqlx::query(
        "UPDATE quotas SET used = 0, period_start = CURRENT_TIMESTAMP, period_end = CASE cycle
            WHEN 'hourly' THEN datetime('now', '+1 hour')
            WHEN 'daily' THEN datetime('now', '+1 day')
            WHEN 'weekly' THEN datetime('now', '+7 days')
            WHEN 'monthly' THEN datetime('now', '+1 month')
            ELSE period_end
        END WHERE id = ? AND cycle != 'permanent' AND period_end IS NOT NULL AND period_end < CURRENT_TIMESTAMP",
    )
    .bind(quota_id)
    .execute(pool)
    .await?;
    Ok(())
}

// === Channel Quota ===

/// Get active quota for a user on a specific channel.
/// Returns the channel-level quota if exists, otherwise falls back to user-level (channel_id IS NULL).
/// Returns None if no quota restriction applies.
pub async fn get_user_quota_for_channel(
    pool: &SqlitePool,
    user_id: i64,
    channel_id: i64,
) -> Result<Option<Quota>, sqlx::Error> {
    // Try channel-level quota first
    let channel_quota: Option<Quota> = sqlx::query_as(
        "SELECT * FROM quotas WHERE user_id = ? AND channel_id = ? AND enabled = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    if channel_quota.is_some() {
        return Ok(channel_quota);
    }

    // Fallback to user-level quota (channel_id IS NULL)
    sqlx::query_as(
        "SELECT * FROM quotas WHERE user_id = ? AND channel_id IS NULL AND enabled = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn check_channel_quota(
    pool: &SqlitePool,
    channel_id: i64,
) -> Result<bool, sqlx::Error> {
    let row: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT quota_limit, quota_used FROM channels WHERE id = ?",
    )
    .bind(channel_id)
    .fetch_one(pool)
    .await?;

    match (row.0, row.1) {
        (Some(limit), Some(used)) => Ok(used < limit),
        _ => Ok(true), // no limit set => not exceeded
    }
}

pub async fn increment_channel_quota(
    pool: &SqlitePool,
    channel_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE channels SET quota_used = quota_used + ? WHERE id = ?",
    )
    .bind(amount)
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Check if an IP is in the global blacklist.
pub async fn is_ip_blacklisted(pool: &SqlitePool, ip: &str) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ip_blacklist WHERE ip_address = ?",
    )
    .bind(ip)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

/// Check if a user has an IP whitelist and if the given IP is in it.
/// Returns Ok(true) if whitelist exists and IP matches.
/// Returns Ok(false) if whitelist exists but IP doesn't match.
/// Returns Ok(None) if user has no whitelist (no restriction).
pub async fn check_user_ip_whitelist(
    pool: &SqlitePool,
    user_id: i64,
    ip: &str,
) -> Result<Option<bool>, sqlx::Error> {
    let has_whitelist: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ip_whitelist WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if has_whitelist == 0 {
        return Ok(None);
    }

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ip_whitelist WHERE user_id = ? AND ip_address = ?",
    )
    .bind(user_id)
    .bind(ip)
    .fetch_one(pool)
    .await?;

    Ok(Some(count > 0))
}

// === AppProfile CRUD ===

pub async fn get_app_profile_by_id(pool: &SqlitePool, id: i64) -> Result<Option<AppProfile>, sqlx::Error> {
    sqlx::query_as::<_, AppProfile>(
        "SELECT * FROM app_profiles WHERE id = ? AND enabled = 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_app_profile_by_identifier(pool: &SqlitePool, identifier: &str) -> Result<Option<AppProfile>, sqlx::Error> {
    sqlx::query_as::<_, AppProfile>(
        "SELECT * FROM app_profiles WHERE identifier = ? AND enabled = 1",
    )
    .bind(identifier)
    .fetch_optional(pool)
    .await
}

pub async fn get_app_profile_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<AppProfile>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, AppProfile>(
        "SELECT * FROM app_profiles ORDER BY is_system DESC, created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_app_profiles(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<Vec<AppProfile>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, AppProfile>(
        "SELECT * FROM app_profiles ORDER BY is_system DESC, created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn create_app_profile(
    pool: &SqlitePool,
    name: &str,
    identifier: &str,
    user_agent: &str,
    extra_headers: Option<&str>,
    description: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO app_profiles (name, identifier, user_agent, extra_headers, description, enabled, is_system) VALUES (?, ?, ?, ?, ?, true, false)",
    )
    .bind(name)
    .bind(identifier)
    .bind(user_agent)
    .bind(extra_headers)
    .bind(description)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_app_profile(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    user_agent: &str,
    extra_headers: Option<&str>,
    description: Option<&str>,
    enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE app_profiles SET name=?, user_agent=?, extra_headers=?, description=?, enabled=? WHERE id=? AND is_system=false",
    )
    .bind(name)
    .bind(user_agent)
    .bind(extra_headers)
    .bind(description)
    .bind(enabled)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_app_profile(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM app_profiles WHERE id = ? AND is_system = false")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn init_system_app_profiles(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_profiles")
        .fetch_one(pool)
        .await?;
    if existing > 0 {
        return Ok(());
    }

    let profiles = [
        ("Claude Code", "claude-code", "claude-code-sdk/1.0", r#"{"anthropic-version":"2023-06-01","anthropic-beta":"message-batches-2024-09-24"}"#, "Claude Code CLI default headers"),
        ("OpenCode", "opencode", "opencode/1.0", r#"{}"#, "OpenCode CLI default headers"),
        ("OpenClaw", "openclaw", "openclaw/1.0", r#"{}"#, "OpenClaw CLI default headers"),
        ("Generic OpenAI", "generic-openai", "openai-sdk/1.0", r#"{}"#, "Standard OpenAI SDK headers"),
        ("Generic Anthropic", "generic-anthropic", "anthropic-sdk/1.0", r#"{"anthropic-version":"2023-06-01"}"#, "Standard Anthropic SDK headers"),
    ];

    for (name, identifier, user_agent, extra_headers, desc) in profiles {
        sqlx::query(
            "INSERT INTO app_profiles (name, identifier, user_agent, extra_headers, description, enabled, is_system) VALUES (?, ?, ?, ?, ?, true, true)",
        )
        .bind(name)
        .bind(identifier)
        .bind(user_agent)
        .bind(extra_headers)
        .bind(desc)
        .execute(pool)
        .await?;
    }

    tracing::info!("Initialized 5 system app profiles");
    Ok(())
}

// === RateLimitConfig CRUD ===

pub async fn get_rate_limit_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<RateLimitConfig>, sqlx::Error> {
    sqlx::query_as::<_, RateLimitConfig>("SELECT * FROM rate_limit_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_rate_limit_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<RateLimitConfig>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, RateLimitConfig>(
        "SELECT * FROM rate_limit_configs ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_ratelimits(pool: &SqlitePool) -> Result<Vec<RateLimitConfig>, sqlx::Error> {
    sqlx::query_as::<_, RateLimitConfig>("SELECT * FROM rate_limit_configs ORDER BY target_type, created_at")
        .fetch_all(pool)
        .await
}

pub async fn create_ratelimit(
    pool: &SqlitePool,
    target_type: &str,
    target_id: Option<i64>,
    qps: i32,
    concurrency: i32,
    action: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO rate_limit_configs (target_type, target_id, qps, concurrency, action) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(target_type)
    .bind(target_id)
    .bind(qps)
    .bind(concurrency)
    .bind(action)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_ratelimit(
    pool: &SqlitePool,
    id: i64,
    qps: i32,
    concurrency: i32,
    action: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE rate_limit_configs SET qps=?, concurrency=?, action=? WHERE id=?")
        .bind(qps)
        .bind(concurrency)
        .bind(action)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_ratelimit(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM rate_limit_configs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// === RequestLog CRUD ===

pub async fn insert_request_log(
    pool: &SqlitePool,
    user_api_key: &str,
    channel_id: Option<i64>,
    model: &str,
    endpoint: &str,
    status_code: i32,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    elapsed_ms: i32,
    request_body: Option<&str>,
    response_body: Option<&str>,
    error_message: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO request_logs (user_api_key, channel_id, model, endpoint, status_code, prompt_tokens, completion_tokens, total_tokens, elapsed_ms, request_body, response_body, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_api_key)
    .bind(channel_id)
    .bind(model)
    .bind(endpoint)
    .bind(status_code)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(elapsed_ms)
    .bind(request_body)
    .bind(response_body)
    .bind(error_message)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn get_request_log_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<RequestLog>, sqlx::Error> {
    sqlx::query_as::<_, RequestLog>("SELECT * FROM request_logs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_request_log(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM request_logs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_request_log_list(
    pool: &SqlitePool,
    page: u32,
    page_size: u32,
) -> Result<Vec<RequestLog>, sqlx::Error> {
    let offset = (page - 1) * page_size;
    sqlx::query_as::<_, RequestLog>(
        "SELECT * FROM request_logs ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_logs(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
    user_id: Option<&str>,
    model: Option<&str>,
    status_code_prefix: Option<&str>,
) -> Result<Vec<RequestLog>, sqlx::Error> {
    let offset = (page - 1) * page_size;

    // Use parameterized queries to prevent SQL injection
    let user_pattern = user_id.map(|u| format!("%{}%", u));
    let model_exact = model.map(|m| m.to_string());
    let status_pattern = status_code_prefix.map(|s| format!("{}%", s));

    let query = sqlx::query_as::<_, RequestLog>(
        "SELECT * FROM request_logs WHERE 1=1
         AND (? IS NULL OR user_api_key LIKE ?)
         AND (? IS NULL OR model = ?)
         AND (? IS NULL OR status_code LIKE ?)
         ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(user_pattern.as_deref())
    .bind(user_pattern.as_deref())
    .bind(model_exact.as_deref())
    .bind(model_exact.as_deref())
    .bind(status_pattern.as_deref())
    .bind(status_pattern.as_deref())
    .bind(page_size)
    .bind(offset);

    query.fetch_all(pool).await
}

pub async fn cleanup_old_logs(pool: &SqlitePool, retention_days: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM request_logs WHERE created_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", retention_days))
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// === TrafficPlan ===

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct TrafficPlan {
    pub id: i64,
    pub channel_id: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct TrafficPlanSlot {
    pub id: i64,
    pub plan_id: i64,
    pub start_hour: i32,
    pub end_hour: i32,
    pub app_profile_id: i64,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrafficPlanDetail {
    pub id: i64,
    pub channel_id: Option<i64>,
    pub slots: Vec<TrafficPlanSlotWithProfile>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrafficPlanSlotWithProfile {
    pub id: i64,
    pub start_hour: i32,
    pub end_hour: i32,
    pub app_profile_id: i64,
    pub app_profile_name: String,
    pub app_profile_identifier: String,
    pub weight: i32,
}

// -- TrafficPlan CRUD --

pub async fn get_global_traffic_plan(pool: &SqlitePool) -> Result<Option<TrafficPlan>, sqlx::Error> {
    sqlx::query_as::<_, TrafficPlan>(
        "SELECT * FROM traffic_plans WHERE channel_id IS NULL LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_traffic_plan_by_channel(
    pool: &SqlitePool,
    channel_id: i64,
) -> Result<Option<TrafficPlan>, sqlx::Error> {
    sqlx::query_as::<_, TrafficPlan>(
        "SELECT * FROM traffic_plans WHERE channel_id = ? LIMIT 1",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
}

pub async fn create_traffic_plan(
    pool: &SqlitePool,
    channel_id: Option<i64>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO traffic_plans (channel_id) VALUES (?)",
    )
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn delete_traffic_plan(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM traffic_plans WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_traffic_plan_by_channel(
    pool: &SqlitePool,
    channel_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM traffic_plans WHERE channel_id = ?")
        .bind(channel_id)
        .execute(pool)
        .await?;
    Ok(())
}

// -- TrafficPlanSlot CRUD --

pub async fn get_slots_by_plan_id(
    pool: &SqlitePool,
    plan_id: i64,
) -> Result<Vec<TrafficPlanSlot>, sqlx::Error> {
    sqlx::query_as::<_, TrafficPlanSlot>(
        "SELECT * FROM traffic_plan_slots WHERE plan_id = ? ORDER BY start_hour, id",
    )
    .bind(plan_id)
    .fetch_all(pool)
    .await
}

pub async fn insert_traffic_plan_slot(
    pool: &SqlitePool,
    plan_id: i64,
    start_hour: i32,
    end_hour: i32,
    app_profile_id: i64,
    weight: i32,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO traffic_plan_slots (plan_id, start_hour, end_hour, app_profile_id, weight) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(plan_id)
    .bind(start_hour)
    .bind(end_hour)
    .bind(app_profile_id)
    .bind(weight)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn delete_slots_by_plan_id(
    pool: &SqlitePool,
    plan_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM traffic_plan_slots WHERE plan_id = ?")
        .bind(plan_id)
        .execute(pool)
        .await?;
    Ok(())
}

// -- Plan detail helpers --

pub async fn get_traffic_plan_detail(
    pool: &SqlitePool,
    plan: &TrafficPlan,
) -> Result<TrafficPlanDetail, sqlx::Error> {
    let slots: Vec<(i64, i32, i32, i64, i32, String, String)> = sqlx::query_as(
        "SELECT s.id, s.start_hour, s.end_hour, s.app_profile_id, s.weight,
                p.name, p.identifier
         FROM traffic_plan_slots s
         JOIN app_profiles p ON p.id = s.app_profile_id
         WHERE s.plan_id = ?
         ORDER BY s.start_hour, s.id",
    )
    .bind(plan.id)
    .fetch_all(pool)
    .await?;

    let slot_details = slots
        .into_iter()
        .map(|(id, sh, eh, ap_id, w, name, ident)| TrafficPlanSlotWithProfile {
            id,
            start_hour: sh,
            end_hour: eh,
            app_profile_id: ap_id,
            app_profile_name: name,
            app_profile_identifier: ident,
            weight: w,
        })
        .collect();

    Ok(TrafficPlanDetail {
        id: plan.id,
        channel_id: plan.channel_id,
        slots: slot_details,
        created_at: plan.created_at,
    })
}

pub async fn list_all_traffic_plans(
    pool: &SqlitePool,
) -> Result<Vec<TrafficPlanDetail>, sqlx::Error> {
    let plans: Vec<TrafficPlan> = sqlx::query_as::<_, TrafficPlan>(
        "SELECT * FROM traffic_plans WHERE channel_id IS NOT NULL ORDER BY channel_id",
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for plan in &plans {
        result.push(get_traffic_plan_detail(pool, plan).await?);
    }
    Ok(result)
}

/// Resolve the app profile to use for a given channel based on traffic plans.
/// Priority: per-channel plan → global plan → legacy channel.app_profile_id → None
pub async fn resolve_app_profile_for_channel(
    pool: &SqlitePool,
    channel: &Channel,
) -> Result<Option<AppProfile>, sqlx::Error> {
    // Try per-channel plan first, then global
    let plan = if let Some(p) = get_traffic_plan_by_channel(pool, channel.id).await? {
        Some(p)
    } else {
        get_global_traffic_plan(pool).await?
    };

    let plan = match plan {
        Some(p) => p,
        None => {
            // No plan at all, fall back to legacy
            if let Some(profile_id) = channel.app_profile_id {
                return get_app_profile_by_id(pool, profile_id).await;
            }
            return Ok(None);
        }
    };

    let current_hour = chrono::Utc::now().hour() as i32;

    let slots: Vec<TrafficPlanSlot> = sqlx::query_as(
        "SELECT * FROM traffic_plan_slots WHERE plan_id = ? AND start_hour <= ? AND end_hour > ?",
    )
    .bind(plan.id)
    .bind(current_hour)
    .bind(current_hour)
    .fetch_all(pool)
    .await?;

    if slots.is_empty() {
        if let Some(profile_id) = channel.app_profile_id {
            return get_app_profile_by_id(pool, profile_id).await;
        }
        return Ok(None);
    }

    // Weighted random selection using mixed entropy seed
    let total_weight: i32 = slots.iter().map(|s| s.weight).sum();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Mix multiple entropy sources: timestamp (seconds + nanos), process ID, thread address
    let thread_addr = format!("{:p}", &std::thread::current())
        .chars()
        .take(16)
        .fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64));
    let seed: u64 = (now.as_secs().wrapping_mul(0x517cc1b727220a95))
        ^ (now.subsec_nanos() as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (std::process::id() as u64).wrapping_mul(0xc4ceb9fe1a85ec53)
        ^ thread_addr;
    let pick = (seed % total_weight as u64) as i32;
    let mut remaining = pick;
    for slot in &slots {
        remaining -= slot.weight;
        if remaining < 0 {
            return get_app_profile_by_id(pool, slot.app_profile_id).await;
        }
    }
    // Fallback
    get_app_profile_by_id(pool, slots[0].app_profile_id).await
}

// === User API Keys ===

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct UserKey {
    pub id: i64,
    pub user_id: i64,
    pub key_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_models: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct UserKeyWithUsername {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub key_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_models: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_user_key(
    pool: &SqlitePool,
    user_id: i64,
    key_value: &str,
    name: Option<&str>,
    enabled_models: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO user_keys (user_id, key_value, name, enabled_models) VALUES (?, ?, ?, ?) RETURNING id"
    )
    .bind(user_id)
    .bind(key_value)
    .bind(name)
    .bind(enabled_models)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn get_user_key_by_value(pool: &SqlitePool, key_value: &str) -> Result<Option<UserKey>, sqlx::Error> {
    sqlx::query_as::<_, UserKey>(
        "SELECT * FROM user_keys WHERE key_value = ? AND status = 'active'"
    )
    .bind(key_value)
    .fetch_optional(pool)
    .await
}

pub async fn list_user_keys_by_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<UserKey>, sqlx::Error> {
    sqlx::query_as::<_, UserKey>(
        "SELECT * FROM user_keys WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_all_user_keys(pool: &SqlitePool) -> Result<Vec<UserKeyWithUsername>, sqlx::Error> {
    sqlx::query_as::<_, UserKeyWithUsername>(
        "SELECT uk.*, u.username FROM user_keys uk JOIN users u ON u.id = uk.user_id ORDER BY uk.created_at DESC"
    )
    .fetch_all(pool)
    .await
}

pub async fn update_user_key(
    pool: &SqlitePool,
    key_id: i64,
    user_id: i64,
    name: Option<&str>,
    enabled_models: Option<&str>,
    status: Option<&str>,
) -> Result<(), sqlx::Error> {
    let q = if let Some(s) = status {
        sqlx::query(
            "UPDATE user_keys SET name = COALESCE(?, name), enabled_models = COALESCE(?, enabled_models), status = COALESCE(?, status) WHERE id = ? AND user_id = ?"
        ).bind(name).bind(enabled_models).bind(s).bind(key_id).bind(user_id)
    } else {
        sqlx::query(
            "UPDATE user_keys SET name = COALESCE(?, name), enabled_models = COALESCE(?, enabled_models) WHERE id = ? AND user_id = ?"
        ).bind(name).bind(enabled_models).bind(key_id).bind(user_id)
    };
    q.execute(pool).await?;
    Ok(())
}

pub async fn delete_user_key(pool: &SqlitePool, key_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_keys WHERE id = ? AND user_id = ?")
        .bind(key_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

