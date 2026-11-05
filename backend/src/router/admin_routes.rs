use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::db::models::{self, LogFilters};
use crate::middleware::auth::AuthContext;
use crate::service::quota;
use crate::utils::ip;
use crate::AppState;

// === Response helpers ===

/// Success envelope: `{"code":0,"message":"ok","data":...}` with HTTP 200.
fn ok_response<T: serde::Serialize>(data: T) -> Response {
    (StatusCode::OK, Json(serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": data,
    })))
        .into_response()
}

/// Error envelope carrying a real HTTP status code.
fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "code": status.as_u16(),
            "message": message,
        })),
    )
        .into_response()
}

fn bad_request(message: &str) -> Response {
    error_response(StatusCode::BAD_REQUEST, message)
}

fn not_found(message: &str) -> Response {
    error_response(StatusCode::NOT_FOUND, message)
}

fn conflict(message: &str) -> Response {
    error_response(StatusCode::CONFLICT, message)
}

fn internal_error(scope: &str, error: impl std::fmt::Display) -> Response {
    tracing::error!(scope, error = %error, "Admin operation failed");
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("{scope} failed: {error}"),
    )
}

// === Request DTOs ===

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<String>,
    pub note: Option<String>,
    pub enabled_models: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub r#type: String,
    pub base_url: String,
    pub api_keys: String,
    pub custom_headers: Option<String>,
    pub weight: Option<i32>,
    pub timeout: Option<i32>,
    pub retry_count: Option<i32>,
    pub quota_type: Option<String>,
    pub quota_limit: Option<i64>,
    pub quota_cycle: Option<String>,
    pub app_profile_id: Option<i64>,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub channel_id: i64,
    pub source_name: String,
    pub proxy_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuotaRequest {
    pub user_id: i64,
    pub quota_type: String,
    pub total_limit: i64,
    pub cycle: String,
    pub channel_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRateLimitRequest {
    pub target_type: String,
    pub target_id: Option<i64>,
    pub qps: i32,
    pub concurrency: i32,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAppProfileRequest {
    pub name: String,
    pub identifier: String,
    pub user_agent: String,
    pub extra_headers: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LogListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub user_id: Option<i64>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub status_code: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatsFilterQuery {
    pub user_api_key: Option<String>,
    pub user_id: Option<i64>,
    pub channel_id: Option<i64>,
    pub model: Option<String>,
    pub days: Option<i64>,
}

// === Update DTOs ===

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub enabled_models: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub base_url: Option<String>,
    pub api_keys: Option<String>,
    pub custom_headers: Option<String>,
    pub weight: Option<i32>,
    pub timeout: Option<i32>,
    pub retry_count: Option<i32>,
    pub quota_type: Option<String>,
    pub quota_limit: Option<i64>,
    pub quota_cycle: Option<String>,
    pub app_profile_id: Option<i64>,
    pub proxy_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub channel_id: Option<i64>,
    pub source_name: Option<String>,
    pub proxy_name: Option<String>,
    pub enabled: Option<bool>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuotaRequest {
    pub quota_type: Option<String>,
    pub total_limit: Option<i64>,
    pub cycle: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRateLimitRequest {
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub qps: Option<i32>,
    pub concurrency: Option<i32>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppProfileRequest {
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub user_agent: Option<String>,
    pub extra_headers: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SetLogLevelRequest {
    pub level: String,
    #[serde(default)]
    pub retention_days: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SetRateLimitRequest {
    pub qps: i32,
    pub concurrency: i32,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct AddBlacklistRequest {
    pub ip_address: String,
}

#[derive(Debug, Deserialize)]
pub struct AddWhitelistRequest {
    pub user_id: i64,
    pub ip_address: String,
}

#[derive(Debug, Deserialize)]
pub struct TrafficPlanSlotRequest {
    pub start_hour: i32,
    pub end_hour: i32,
    pub app_profile_id: i64,
    pub weight: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpsertTrafficPlanRequest {
    pub slots: Vec<TrafficPlanSlotRequest>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserKeyRequest {
    pub name: Option<String>,
    pub enabled_models: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserKeyRequest {
    pub name: Option<String>,
    pub enabled_models: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserStatsQuery {
    pub days: Option<i64>,
    pub channel: Option<i64>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserLogsQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub model: Option<String>,
    pub status_code: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

// === Validation helpers ===

/// Validate a user-supplied model list (`NULL`, `[...]` or empty string).
fn validate_model_list(raw: Option<&str>) -> Result<(), String> {
    let Some(raw) = raw else { return Ok(()) };
    if raw.trim().is_empty() {
        return Ok(());
    }
    match serde_json::from_str::<Vec<String>>(raw) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("enabled_models must be a JSON array of strings: {e}")),
    }
}

/// Validate the channel API keys payload: a non-empty JSON array of strings.
fn validate_api_keys(raw: &str) -> Result<Vec<String>, String> {
    let keys: Vec<String> = serde_json::from_str(raw)
        .map_err(|e| format!("api_keys must be a JSON array of strings: {e}"))?;
    if keys.is_empty() {
        return Err("api_keys must contain at least one key".to_string());
    }
    if keys.iter().any(|key| key.trim().is_empty()) {
        return Err("api_keys must not contain empty values".to_string());
    }
    Ok(keys)
}

/// Validate the optional channel quota configuration.
fn validate_channel_quota(
    quota_type: Option<&str>,
    quota_limit: Option<i64>,
    quota_cycle: Option<&str>,
) -> Result<(), String> {
    match quota_type.map(str::trim).filter(|t| !t.is_empty()) {
        None => Ok(()),
        Some(kind) => {
            if !quota::is_valid_quota_type(kind) {
                return Err(format!(
                    "quota_type must be 'requests' or 'tokens', got '{kind}'"
                ));
            }
            match quota_limit {
                Some(limit) if limit > 0 => {}
                _ => return Err("quota_limit must be greater than 0 when quota_type is set".into()),
            }
            match quota_cycle.map(str::trim) {
                Some(cycle) if quota::is_valid_cycle(cycle) => Ok(()),
                Some(cycle) => Err(format!(
                    "quota_cycle must be hourly/daily/weekly/monthly/permanent, got '{cycle}'"
                )),
                None => Err("quota_cycle is required when quota_type is set".into()),
            }
        }
    }
}

fn validate_base_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("base_url is required".to_string());
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("base_url must start with http:// or https://".to_string());
    }
    Ok(())
}

fn validate_custom_headers(raw: Option<&str>) -> Result<(), String> {
    let Some(raw) = raw else { return Ok(()) };
    if raw.trim().is_empty() {
        return Ok(());
    }
    serde_json::from_str::<HashMap<String, String>>(raw)
        .map(|_| ())
        .map_err(|e| format!("custom_headers must be a JSON object of strings: {e}"))
}

fn validate_username(username: &str) -> Result<(), String> {
    let username = username.trim();
    if username.len() < 3 {
        return Err("username must be at least 3 characters".to_string());
    }
    if username.chars().any(char::is_whitespace) {
        return Err("username must not contain whitespace".to_string());
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.chars().count() < 6 {
        return Err("password must be at least 6 characters".to_string());
    }
    Ok(())
}

/// Map a row of channel data as stored in SQLite.
type ChannelRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    i32,
    i32,
    i32,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<i64>,
    Option<String>,
    String,
);

// === Auth endpoints ===

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Response {
    let pool = &state.db;
    let user = match models::get_user_by_username(pool, &req.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return error_response(StatusCode::UNAUTHORIZED, "Invalid username or password")
        }
        Err(e) => return internal_error("login", e),
    };

    if user.status != "active" {
        return error_response(StatusCode::UNAUTHORIZED, "User account is disabled");
    }

    // bcrypt verification is CPU bound; keep it off the async scheduler.
    let hash = user.password_hash.clone();
    let password = req.password.clone();
    let verified = tokio::task::spawn_blocking(move || {
        bcrypt::verify(&password, &hash).unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    if !verified {
        return error_response(StatusCode::UNAUTHORIZED, "Invalid username or password");
    }

    let token = match crate::utils::jwt::generate_token(
        user.id,
        &user.username,
        &user.role,
        &state.config.jwt.secret,
        state.config.jwt.expires_in as i64,
    ) {
        Ok(token) => token,
        Err(e) => return internal_error("token generation", e),
    };

    let refresh_token = match crate::utils::jwt::generate_token(
        user.id,
        &user.username,
        &user.role,
        &state.config.jwt.secret,
        7 * 24 * 3600,
    ) {
        Ok(token) => token,
        Err(e) => return internal_error("refresh token generation", e),
    };

    ok_response(serde_json::json!({
        "token": token,
        "refreshToken": refresh_token,
        "user": {
            "id": user.id,
            "username": user.username,
            "role": user.role,
        }
    }))
}

pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Response {
    let claims =
        match crate::utils::jwt::verify_token(&req.refresh_token, &state.config.jwt.secret) {
            Ok(claims) => claims,
            Err(_) => return error_response(StatusCode::UNAUTHORIZED, "Invalid refresh token"),
        };

    let user = match models::get_user_by_id(&state.db, claims.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return error_response(StatusCode::UNAUTHORIZED, "User not found"),
        Err(e) => return internal_error("refresh token", e),
    };

    if user.status != "active" {
        return error_response(StatusCode::UNAUTHORIZED, "User account is disabled");
    }

    let token = match crate::utils::jwt::generate_token(
        user.id,
        &user.username,
        &user.role,
        &state.config.jwt.secret,
        state.config.jwt.expires_in as i64,
    ) {
        Ok(token) => token,
        Err(e) => return internal_error("token generation", e),
    };

    ok_response(serde_json::json!({
        "token": token,
        "refreshToken": req.refresh_token,
        "user": {
            "id": user.id,
            "username": user.username,
            "role": user.role,
        }
    }))
}

pub async fn get_me(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = match req.extensions().get::<i64>() {
        Some(id) => *id,
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid token"),
    };

    match models::get_user_by_id(&state.db, user_id).await {
        Ok(Some(user)) => ok_response(serde_json::json!({
            "id": user.id,
            "username": user.username,
            "role": user.role,
            "status": user.status,
            "created_at": user.created_at,
        })),
        Ok(None) => not_found("User not found"),
        Err(e) => internal_error("get_me", e),
    }
}

// === User endpoints ===

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).clamp(1, 200);

    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await {
        Ok(total) => total,
        Err(e) => return internal_error("list users", e),
    };

    let users = match models::list_users(pool, page, page_size).await {
        Ok(users) => users,
        Err(e) => return internal_error("list users", e),
    };

    // Aggregate quotas per user, keeping request and token quotas separate so that the
    // UI never adds "requests" to "tokens".
    let user_ids: Vec<i64> = users.iter().map(|user| user.id).collect();
    let mut quota_map: HashMap<i64, (i64, i64)> = HashMap::new();
    let mut token_map: HashMap<i64, (i64, i64)> = HashMap::new();

    if !user_ids.is_empty() {
        let placeholders = user_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT user_id, quota_type, COALESCE(SUM(total_limit), 0), COALESCE(SUM(used), 0) \
             FROM quotas WHERE user_id IN ({placeholders}) AND enabled = 1 \
             GROUP BY user_id, quota_type"
        );
        let mut query = sqlx::query_as::<_, (i64, String, i64, i64)>(&sql);
        for user_id in &user_ids {
            query = query.bind(user_id);
        }
        if let Ok(rows) = query.fetch_all(pool).await {
            for (user_id, quota_type, limit, used) in rows {
                if quota_type == "tokens" {
                    token_map.insert(user_id, (limit, used));
                } else {
                    quota_map.insert(user_id, (limit, used));
                }
            }
        }
    }

    let mut result = Vec::with_capacity(users.len());
    for user in users {
        let (quota_total, quota_used) = quota_map.get(&user.id).copied().unwrap_or((0, 0));
        let (token_total, token_used) = token_map.get(&user.id).copied().unwrap_or((0, 0));
        let mut obj = serde_json::to_value(&user).unwrap_or_default();
        if let Some(obj) = obj.as_object_mut() {
            obj.insert("quota_total".into(), serde_json::json!(quota_total));
            obj.insert("quota_used".into(), serde_json::json!(quota_used));
            obj.insert("token_quota_total".into(), serde_json::json!(token_total));
            obj.insert("token_quota_used".into(), serde_json::json!(token_used));
        }
        result.push(obj);
    }

    ok_response(serde_json::json!({ "items": result, "total": total }))
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Response {
    if let Err(message) = validate_username(&req.username) {
        return bad_request(&message);
    }
    if let Err(message) = validate_password(&req.password) {
        return bad_request(&message);
    }
    if let Err(message) = validate_model_list(req.enabled_models.as_deref()) {
        return bad_request(&message);
    }

    let role = req.role.as_deref().unwrap_or("user");
    if role != "admin" && role != "user" {
        return bad_request("role must be 'admin' or 'user'");
    }

    let pool = &state.db;
    if let Ok(Some(_)) = models::get_user_by_username(pool, &req.username).await {
        return conflict("Username already exists");
    }

    let password = req.password.clone();
    let password_hash = match tokio::task::spawn_blocking(move || {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
    })
    .await
    {
        Ok(Ok(hash)) => hash,
        Ok(Err(e)) => return internal_error("hash password", e),
        Err(e) => return internal_error("hash password", e),
    };

    let api_key = format!("sk-{}", Uuid::new_v4().simple());

    let user_id = match models::create_user(
        pool,
        &req.username,
        &password_hash,
        role,
        &api_key,
        "active",
        req.enabled_models.as_deref(),
        req.note.as_deref(),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => return internal_error("create user", e),
    };

    // Mirror the primary key so the Key management page lists it (and can revoke it).
    if let Err(e) =
        models::create_user_key(pool, user_id, &api_key, Some("Default Key"), None).await
    {
        tracing::warn!(error = %e, user_id, "Failed to create default key row");
    }

    ok_response(serde_json::json!({
        "id": user_id,
        "api_key": api_key,
    }))
}

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Response {
    if let Err(message) = validate_model_list(req.enabled_models.as_deref()) {
        return bad_request(&message);
    }

    let pool = &state.db;
    type UserRow = (String, String, String, Option<String>, Option<String>);
    let current: Option<UserRow> = match sqlx::query_as(
        "SELECT username, role, status, enabled_models, note FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => return internal_error("load user", e),
    };

    let Some((username, role, status, enabled_models, note)) = current else {
        return not_found("User not found");
    };

    let username = req.username.unwrap_or(username);
    let role = req.role.unwrap_or(role);
    let status = req.status.unwrap_or(status);
    let enabled_models = req.enabled_models.or(enabled_models);
    let note = req.note.or(note);

    if let Err(message) = validate_username(&username) {
        return bad_request(&message);
    }
    if role != "admin" && role != "user" {
        return bad_request("role must be 'admin' or 'user'");
    }
    if status != "active" && status != "disabled" {
        return bad_request("status must be 'active' or 'disabled'");
    }

    match models::update_user(
        pool,
        id,
        &username,
        &role,
        &status,
        enabled_models.as_deref(),
        note.as_deref(),
    )
    .await
    {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) if e.to_string().contains("UNIQUE") => conflict("Username already exists"),
        Err(e) => internal_error("update user", e),
    }
}

pub async fn reset_user_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    let current: Option<String> = match sqlx::query_scalar("SELECT api_key FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
    {
        Ok(key) => key,
        Err(e) => return internal_error("reset user key", e),
    };

    let Some(old_key) = current else {
        return not_found("User not found");
    };

    let new_key = format!("sk-{}", Uuid::new_v4().simple());
    match models::rotate_user_api_key(pool, id, &old_key, &new_key).await {
        Ok(()) => ok_response(serde_json::json!({ "api_key": new_key })),
        Err(e) => internal_error("reset user key", e),
    }
}

pub async fn delete_user_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    if ctx.user_id == id {
        return bad_request("You cannot delete your own account");
    }

    let pool = &state.db;
    // Keep related rows consistent even though SQLite may not enforce cascades for
    // data created before foreign keys were enabled.
    if let Err(e) = sqlx::query("DELETE FROM user_keys WHERE user_id = ?")
        .bind(id)
        .execute(pool)
        .await
    {
        return internal_error("delete user keys", e);
    }
    if let Err(e) = sqlx::query("DELETE FROM quotas WHERE user_id = ?")
        .bind(id)
        .execute(pool)
        .await
    {
        return internal_error("delete user quotas", e);
    }
    if let Err(e) = sqlx::query("DELETE FROM ip_whitelist WHERE user_id = ?")
        .bind(id)
        .execute(pool)
        .await
    {
        return internal_error("delete user whitelist", e);
    }

    match models::delete_user(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete user", e),
    }
}

// === Channel endpoints ===

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).clamp(1, 200);

    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM channels").fetch_one(pool).await {
        Ok(total) => total,
        Err(e) => return internal_error("list channels", e),
    };

    let mut channels = match models::list_channels(pool, page, page_size).await {
        Ok(channels) => channels,
        Err(e) => return internal_error("list channels", e),
    };

    // Per-channel usage statistics.
    let stats_map: HashMap<i64, (i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT channel_id,
                COUNT(*) as total_requests,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                COALESCE(SUM(CASE WHEN created_at >= datetime('now', 'start of day') THEN 1 ELSE 0 END), 0) as today_requests,
                COALESCE(SUM(CASE WHEN created_at >= datetime('now', 'start of day') THEN total_tokens ELSE 0 END), 0) as today_tokens
         FROM request_logs
         WHERE channel_id IS NOT NULL
         GROUP BY channel_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(channel_id, total_requests, total_tokens, today_requests, today_tokens)| {
        (channel_id, (total_requests, total_tokens, today_requests, today_tokens))
    })
    .collect();

    // Health metadata lives on the table but is not exposed by the struct.
    let health_map: HashMap<i64, (i32, Option<String>, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, consecutive_failures, last_checked, last_success_at, auto_disabled FROM channels",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, failures, checked, success, auto_disabled)| {
        (id, (failures, checked, success, auto_disabled))
    })
    .collect();

    let mut items = Vec::with_capacity(channels.len());
    for channel in &mut channels {
        let decryptable = models::decrypt_api_keys_checked(&channel.api_keys, &state.encryption).is_ok();
        channel.api_keys = models::maybe_decrypt_api_keys(&channel.api_keys, &state.encryption);

        let channel_json = match serde_json::to_value(&*channel) {
            Ok(value) => value,
            Err(e) => return internal_error("serialize channel", e),
        };
        let mut enriched = channel_json.as_object().cloned().unwrap_or_default();

        let channel_id = channel.id;
        let (total_requests, total_tokens, today_requests, today_tokens) =
            stats_map.get(&channel_id).copied().unwrap_or((0, 0, 0, 0));
        enriched.insert(
            "stats".to_string(),
            serde_json::json!({
                "total_requests": total_requests,
                "total_tokens": total_tokens,
                "today_requests": today_requests,
                "today_tokens": today_tokens,
            }),
        );

        let (failures, checked, success, auto_disabled) =
            health_map.get(&channel_id).cloned().unwrap_or((0, None, None, false));
        enriched.insert("consecutive_failures".to_string(), serde_json::json!(failures));
        enriched.insert("last_checked".to_string(), serde_json::json!(checked));
        enriched.insert("last_success_at".to_string(), serde_json::json!(success));
        enriched.insert("auto_disabled".to_string(), serde_json::json!(auto_disabled));
        enriched.insert("api_keys_decryptable".to_string(), serde_json::json!(decryptable));

        items.push(Value::Object(enriched));
    }

    ok_response(serde_json::json!({ "items": items, "total": total }))
}

pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateChannelRequest>,
) -> Response {
    if let Err(message) = validate_base_url(&req.base_url) {
        return bad_request(&message);
    }
    if let Err(message) = validate_api_keys(&req.api_keys) {
        return bad_request(&message);
    }
    if let Err(message) = validate_custom_headers(req.custom_headers.as_deref()) {
        return bad_request(&message);
    }
    if let Err(message) = validate_channel_quota(
        req.quota_type.as_deref(),
        req.quota_limit,
        req.quota_cycle.as_deref(),
    ) {
        return bad_request(&message);
    }

    let channel_type = req.r#type.trim();
    if channel_type != "openai" && channel_type != "anthropic" {
        return bad_request("type must be 'openai' or 'anthropic'");
    }

    let weight = req.weight.unwrap_or(1);
    if weight <= 0 {
        return bad_request("weight must be greater than 0");
    }
    let timeout = req.timeout.unwrap_or(300_000);
    if timeout <= 0 {
        return bad_request("timeout must be greater than 0 (milliseconds)");
    }
    let retry_count = req.retry_count.unwrap_or(1).max(1);

    match models::create_channel(
        &state.db,
        req.name.trim(),
        channel_type,
        req.base_url.trim(),
        &req.api_keys,
        req.custom_headers.as_deref(),
        weight,
        timeout,
        retry_count,
        req.quota_type.as_deref().filter(|t| !t.trim().is_empty()),
        req.quota_limit,
        req.quota_cycle.as_deref().filter(|c| !c.trim().is_empty()),
        req.app_profile_id,
        req.proxy_url.as_deref().filter(|p| !p.trim().is_empty()),
        &state.encryption,
    )
    .await
    {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) if e.to_string().contains("UNIQUE") => conflict("Channel name already exists"),
        Err(e) => internal_error("create channel", e),
    }
}

pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateChannelRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<ChannelRow> = match sqlx::query_as(
        "SELECT name, type, base_url, api_keys, custom_headers, weight, timeout, retry_count,
                quota_type, quota_limit, quota_cycle, app_profile_id, proxy_url, status
         FROM channels WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => return internal_error("load channel", e),
    };

    let Some((
        name,
        channel_type,
        base_url,
        stored_keys,
        custom_headers,
        weight,
        timeout,
        retry_count,
        quota_type,
        quota_limit,
        quota_cycle,
        app_profile_id,
        proxy_url,
        status,
    )) = current
    else {
        return not_found("Channel not found");
    };

    // Refuse to re-save ciphertext we cannot read: that would encrypt it a second time
    // and permanently corrupt the stored keys.
    let decrypted = models::decrypt_api_keys_checked(&stored_keys, &state.encryption);
    let (stored_plaintext, decrypt_ok) = match decrypted {
        Ok(plain) => (plain, true),
        Err(stored) => (stored, false),
    };

    if !decrypt_ok && req.api_keys.is_none() {
        return bad_request(
            "Stored API keys cannot be decrypted with the configured encryption secret. \
             Provide the plaintext keys again or restore the correct secret.",
        );
    }

    let next_keys = match req.api_keys {
        Some(keys) => {
            if !decrypt_ok && keys == stored_keys {
                return bad_request(
                    "Stored API keys cannot be decrypted with the configured encryption secret. \
                     Provide the plaintext keys again or restore the correct secret.",
                );
            }
            keys
        }
        None => stored_plaintext,
    };

    if let Err(message) = validate_api_keys(&next_keys) {
        return bad_request(&message);
    }

    let name = req.name.unwrap_or(name);
    let channel_type = req.r#type.unwrap_or(channel_type);
    let base_url = req.base_url.unwrap_or(base_url);
    let weight = req.weight.unwrap_or(weight);
    let timeout = req.timeout.unwrap_or(timeout);
    let retry_count = req.retry_count.unwrap_or(retry_count).max(1);
    let quota_type = req.quota_type.or(quota_type);
    let quota_limit = req.quota_limit.or(quota_limit);
    let quota_cycle = req.quota_cycle.or(quota_cycle);
    let app_profile_id = req.app_profile_id.or(app_profile_id);
    let proxy_url = req.proxy_url.or(proxy_url);
    let status = req.status.unwrap_or(status);
    let custom_headers = req.custom_headers.or(custom_headers);

    if let Err(message) = validate_base_url(&base_url) {
        return bad_request(&message);
    }
    if let Err(message) = validate_custom_headers(custom_headers.as_deref()) {
        return bad_request(&message);
    }
    if let Err(message) = validate_channel_quota(
        quota_type.as_deref(),
        quota_limit,
        quota_cycle.as_deref(),
    ) {
        return bad_request(&message);
    }
    if channel_type != "openai" && channel_type != "anthropic" {
        return bad_request("type must be 'openai' or 'anthropic'");
    }
    if weight <= 0 {
        return bad_request("weight must be greater than 0");
    }
    if timeout <= 0 {
        return bad_request("timeout must be greater than 0 (milliseconds)");
    }
    if status != "active" && status != "disabled" {
        return bad_request("status must be 'active' or 'disabled'");
    }

    match models::update_channel(
        pool,
        id,
        name.trim(),
        &channel_type,
        base_url.trim(),
        &next_keys,
        custom_headers.as_deref(),
        weight,
        timeout,
        retry_count,
        quota_type.as_deref().filter(|t| !t.trim().is_empty()),
        quota_limit,
        quota_cycle.as_deref().filter(|c| !c.trim().is_empty()),
        app_profile_id,
        proxy_url.as_deref().filter(|p| !p.trim().is_empty()),
        &status,
        &state.encryption,
    )
    .await
    {
        Ok(()) => {
            // Re-initialise the quota period when the cycle changed so the reset works.
            if let Err(e) = models::set_channel_quota_period(pool, id, quota_cycle.as_deref()).await {
                tracing::warn!(error = %e, channel_id = id, "Failed to sync quota period");
            }
            // A manual status change clears the auto-disabled marker.
            if status == "active" {
                let _ = sqlx::query(
                    "UPDATE channels SET auto_disabled = 0, consecutive_failures = 0 WHERE id = ?",
                )
                .bind(id)
                .execute(pool)
                .await;
            }
            ok_response(serde_json::json!({ "updated": true }))
        }
        Err(e) if e.to_string().contains("UNIQUE") => conflict("Channel name already exists"),
        Err(e) => internal_error("update channel", e),
    }
}

pub async fn delete_channel_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;

    // Clean up dependent rows in a transaction: SQLite foreign keys are enabled, and
    // traffic plan slots reference the plan (not the channel), so remove explicitly.
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return internal_error("delete channel", e),
    };

    let statements = [
        "DELETE FROM models WHERE channel_id = ?",
        "DELETE FROM traffic_plan_slots WHERE plan_id IN (SELECT id FROM traffic_plans WHERE channel_id = ?)",
        "DELETE FROM traffic_plans WHERE channel_id = ?",
        "DELETE FROM quotas WHERE channel_id = ?",
    ];
    for sql in statements {
        if let Err(e) = sqlx::query(sql).bind(id).execute(&mut *tx).await {
            return internal_error("delete channel dependencies", e);
        }
    }
    if let Err(e) = sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        return internal_error("delete channel", e);
    }
    if let Err(e) = tx.commit().await {
        return internal_error("delete channel", e);
    }

    // Drop cached limiter state for the removed channel.
    state.channel_semaphores.remove(&id);
    state.key_cursors.remove(&id);

    ok_response(serde_json::json!({ "deleted": true }))
}

pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    let channel = match models::get_channel_by_id(pool, id).await {
        Ok(Some(channel)) => channel,
        Ok(None) => return not_found("Channel not found"),
        Err(e) => return internal_error("load channel", e),
    };

    let keys_json = match models::decrypt_api_keys_checked(&channel.api_keys, &state.encryption) {
        Ok(plain) => plain,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Stored API keys cannot be decrypted; check the encryption secret",
            )
        }
    };

    let keys: Vec<String> = match serde_json::from_str(&keys_json) {
        Ok(keys) => keys,
        Err(e) => return bad_request(&format!("Invalid API keys format: {e}")),
    };
    let Some(api_key) = keys.first() else {
        return bad_request("No API key configured");
    };

    let timeout_secs = if channel.timeout > 0 {
        (channel.timeout as u64 / 1000).clamp(3, 120)
    } else {
        15
    };
    let client = crate::service::proxy::build_client_with_proxy(channel.proxy_url.as_deref());
    let test_url = format!("{}/models", channel.base_url.trim_end_matches('/'));

    let mut req = client.get(&test_url).timeout(std::time::Duration::from_secs(timeout_secs));
    if channel.r#type == "anthropic" {
        req = req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }

    let start = std::time::Instant::now();
    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let latency_ms = start.elapsed().as_millis() as u64;
            let body = resp.text().await.unwrap_or_default();
            // Character-safe truncation: upstream errors are often non-ASCII.
            let body_summary: String = body.chars().take(200).collect();

            if status.is_success() {
                ok_response(serde_json::json!({
                    "status": "ok",
                    "http_status": status.as_u16(),
                    "message": "渠道连通性测试成功",
                    "latency_ms": latency_ms,
                    "response_body": body_summary,
                }))
            } else {
                let hint = match status.as_u16() {
                    401 => "API Key 可能无效或未配置，请检查 API Key 是否正确",
                    403 => "该 API Key 无权访问 /models 端点，请确认 Key 权限",
                    404 | 405 => "BaseURL 可能不正确，或该供应商不支持 /models 端点",
                    429 => "请求频率超限，请稍后再试",
                    500..=599 => "上游服务异常，请稍后再试",
                    _ => "请检查 BaseURL 和 API Key 配置是否正确",
                };
                ok_response(serde_json::json!({
                    "status": "warning",
                    "http_status": status.as_u16(),
                    "message": format!("渠道返回状态码 {}，{}", status.as_u16(), hint),
                    "latency_ms": latency_ms,
                    "response_body": body_summary,
                }))
            }
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            error_response(
                StatusCode::BAD_GATEWAY,
                &format!("连接上游服务失败: {e} (耗时 {latency_ms}ms)"),
            )
        }
    }
}

/// GET /admin/channels/{id}/stats — detailed usage stats for a single channel
pub async fn channel_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;

    match models::get_channel_by_id(pool, id).await {
        Ok(None) => return not_found("Channel not found"),
        Err(e) => return internal_error("load channel", e),
        _ => {}
    }

    let total: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE channel_id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap_or((0, 0));

    let today: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0) FROM request_logs
         WHERE channel_id = ? AND created_at >= datetime('now', 'start of day')",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap_or((0, 0));

    let channels = sqlx::query_as::<_, (Option<String>, Option<i64>, i64, Option<String>)>(
        "SELECT quota_type, quota_limit, quota_used, quota_cycle FROM channels WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (quota_type, quota_limit, quota_used, quota_cycle) =
        channels.unwrap_or((None, None, 0, None));
    let quota_percent = match quota_limit {
        Some(limit) if limit > 0 => {
            Some(((quota_used as f64 / limit as f64) * 100.0 * 10.0).round() / 10.0)
        }
        _ => None,
    };

    ok_response(serde_json::json!({
        "total_requests": total.0,
        "total_tokens": total.1,
        "today_requests": today.0,
        "today_tokens": today.1,
        "quota_type": quota_type,
        "quota_limit": quota_limit,
        "quota_used": quota_used,
        "quota_cycle": quota_cycle,
        "quota_percent": quota_percent,
    }))
}

// === Model endpoints ===

pub async fn list_models_endpoint(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).clamp(1, 500);

    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM models").fetch_one(pool).await {
        Ok(total) => total,
        Err(e) => return internal_error("list models", e),
    };

    match models::list_models(pool, page, page_size).await {
        Ok(models) => ok_response(serde_json::json!({ "items": models, "total": total })),
        Err(e) => internal_error("list models", e),
    }
}

pub async fn create_model_endpoint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateModelRequest>,
) -> Response {
    let source_name = req.source_name.trim();
    let proxy_name = req.proxy_name.trim();
    if source_name.is_empty() || proxy_name.is_empty() {
        return bad_request("source_name and proxy_name are required");
    }

    let pool = &state.db;
    match models::get_channel_by_id(pool, req.channel_id).await {
        Ok(None) => return bad_request("channel_id does not exist"),
        Err(e) => return internal_error("create model", e),
        _ => {}
    }

    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM models WHERE channel_id = ? AND proxy_name = ?",
    )
    .bind(req.channel_id)
    .bind(proxy_name)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    if existing > 0 {
        return conflict("This channel already exposes that proxy name");
    }

    match models::create_model(pool, req.channel_id, source_name, proxy_name).await {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) => internal_error("create model", e),
    }
}

pub async fn update_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateModelRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(i64, String, String, bool, bool)> = match sqlx::query_as(
        "SELECT channel_id, source_name, proxy_name, enabled, is_default FROM models WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => return internal_error("load model", e),
    };

    let Some((channel_id, source_name, proxy_name, enabled, is_default)) = current else {
        return not_found("Model not found");
    };

    let channel_id = req.channel_id.unwrap_or(channel_id);
    let source_name = req.source_name.unwrap_or(source_name);
    let proxy_name = req.proxy_name.unwrap_or(proxy_name);
    let enabled = req.enabled.unwrap_or(enabled);
    let is_default = req.is_default.unwrap_or(is_default);

    if source_name.trim().is_empty() || proxy_name.trim().is_empty() {
        return bad_request("source_name and proxy_name are required");
    }

    match models::get_channel_by_id(pool, channel_id).await {
        Ok(None) => return bad_request("channel_id does not exist"),
        Err(e) => return internal_error("update model", e),
        _ => {}
    }

    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM models WHERE channel_id = ? AND proxy_name = ? AND id != ?",
    )
    .bind(channel_id)
    .bind(proxy_name.trim())
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    if duplicate > 0 {
        return conflict("This channel already exposes that proxy name");
    }

    match models::update_model(
        pool,
        id,
        source_name.trim(),
        proxy_name.trim(),
        enabled,
        is_default,
    )
    .await
    {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => internal_error("update model", e),
    }
}

pub async fn delete_model_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match models::delete_model(&state.db, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete model", e),
    }
}

// === Quota endpoints ===

pub async fn list_quotas(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).clamp(1, 200);

    let total: i64 = match pagination.user_id {
        Some(user_id) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM quotas WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await
        }
        None => sqlx::query_scalar("SELECT COUNT(*) FROM quotas").fetch_one(pool).await,
    }
    .unwrap_or(0);

    match models::list_quotas(pool, pagination.user_id, page, page_size).await {
        Ok(quotas) => ok_response(serde_json::json!({ "items": quotas, "total": total })),
        Err(e) => internal_error("list quotas", e),
    }
}

pub async fn create_quota(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateQuotaRequest>,
) -> Response {
    if !quota::is_valid_quota_type(&req.quota_type) {
        return bad_request("quota_type must be 'requests' or 'tokens'");
    }
    if !quota::is_valid_cycle(&req.cycle) {
        return bad_request("cycle must be hourly/daily/weekly/monthly/permanent");
    }
    if req.total_limit < 0 {
        return bad_request("total_limit cannot be negative");
    }

    let pool = &state.db;

    match models::get_user_by_id(pool, req.user_id).await {
        Ok(None) => return bad_request("user_id does not exist"),
        Err(e) => return internal_error("create quota", e),
        _ => {}
    }

    if let Some(channel_id) = req.channel_id {
        match models::get_channel_by_id(pool, channel_id).await {
            Ok(None) => return bad_request("channel_id does not exist"),
            Err(e) => return internal_error("create quota", e),
            _ => {}
        }
    }

    match models::quota_exists(pool, req.user_id, req.channel_id).await {
        Ok(true) => {
            return conflict(
                "A quota already exists for this user/channel; edit the existing quota instead",
            )
        }
        Err(e) => return internal_error("create quota", e),
        Ok(false) => {}
    }

    match models::create_quota(
        pool,
        req.user_id,
        &req.quota_type,
        req.total_limit,
        &req.cycle,
        req.channel_id,
    )
    .await
    {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) => internal_error("create quota", e),
    }
}

pub async fn update_quota(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateQuotaRequest>,
) -> Response {
    if let Some(limit) = req.total_limit
        && limit < 0 {
            return bad_request("total_limit cannot be negative");
        }
    if let Some(cycle) = req.cycle.as_deref()
        && !quota::is_valid_cycle(cycle) {
            return bad_request("cycle must be hourly/daily/weekly/monthly/permanent");
        }
    if let Some(kind) = req.quota_type.as_deref()
        && !quota::is_valid_quota_type(kind) {
            return bad_request("quota_type must be 'requests' or 'tokens'");
        }

    let pool = &state.db;
    let current: Option<(i64, String, bool)> =
        match sqlx::query_as("SELECT total_limit, quota_type, enabled FROM quotas WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
        {
            Ok(row) => row,
            Err(e) => return internal_error("load quota", e),
        };

    let Some((limit, quota_type, enabled)) = current else {
        return not_found("Quota not found");
    };

    let total_limit = req.total_limit.unwrap_or(limit);
    let quota_type = req.quota_type.unwrap_or(quota_type);
    let cycle = req.cycle.clone();
    let enabled = req.enabled.unwrap_or(enabled);

    // Changing type/cycle needs the period (and ideally usage) reset to stay coherent.
    if let Some(cycle) = cycle {
        let (period_start, period_end) = quota::compute_period(Some(&cycle));
        if let Err(e) = sqlx::query(
            "UPDATE quotas SET quota_type = ?, total_limit = ?, cycle = ?, enabled = ?,
                    used = 0, period_start = ?, period_end = ? WHERE id = ?",
        )
        .bind(&quota_type)
        .bind(total_limit)
        .bind(&cycle)
        .bind(enabled)
        .bind(period_start)
        .bind(period_end)
        .bind(id)
        .execute(pool)
        .await
        {
            return internal_error("update quota", e);
        }
    } else if let Err(e) = sqlx::query(
        "UPDATE quotas SET quota_type = ?, total_limit = ?, enabled = ? WHERE id = ?",
    )
    .bind(&quota_type)
    .bind(total_limit)
    .bind(enabled)
    .bind(id)
    .execute(pool)
    .await
    {
        return internal_error("update quota", e);
    }

    ok_response(serde_json::json!({ "updated": true }))
}

pub async fn delete_quota(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match models::delete_quota(&state.db, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete quota", e),
    }
}

// === RateLimit endpoints ===

fn validate_rate_limit(qps: i32, concurrency: i32, action: &str) -> Result<(), String> {
    if qps < 0 || concurrency < 0 {
        return Err("qps and concurrency cannot be negative".to_string());
    }
    if action != "reject" && action != "queue" {
        return Err("action must be 'reject' or 'queue'".to_string());
    }
    Ok(())
}

/// Validate that the target of a rate limit rule exists.
async fn validate_rate_limit_target(
    pool: &sqlx::SqlitePool,
    target_type: &str,
    target_id: Option<i64>,
) -> Result<(), String> {
    match target_type {
        "global" => Ok(()),
        "channel" => {
            let id = target_id.ok_or("target_id is required for channel limits")?;
            match models::get_channel_by_id(pool, id).await {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err("target channel does not exist".to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        "user" => {
            let id = target_id.ok_or("target_id is required for user limits")?;
            match models::get_user_by_id(pool, id).await {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err("target user does not exist".to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        _ => Err("target_type must be 'global', 'channel' or 'user'".to_string()),
    }
}

pub async fn list_ratelimits(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(100).clamp(1, 500);

    match models::list_ratelimits(&state.db, page, page_size).await {
        Ok((configs, total)) => {
            ok_response(serde_json::json!({ "items": configs, "total": total }))
        }
        Err(e) => internal_error("list rate limits", e),
    }
}

pub async fn create_ratelimit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRateLimitRequest>,
) -> Response {
    if let Err(message) = validate_rate_limit(req.qps, req.concurrency, &req.action) {
        return bad_request(&message);
    }
    if let Err(message) =
        validate_rate_limit_target(&state.db, &req.target_type, req.target_id).await
    {
        return bad_request(&message);
    }

    let target_id = if req.target_type == "global" {
        None
    } else {
        req.target_id
    };

    let exists: i64 = match target_id {
        Some(id) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM rate_limit_configs WHERE target_type = ? AND target_id = ?",
            )
            .bind(&req.target_type)
            .bind(id)
            .fetch_one(&state.db)
            .await
        }
        None => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM rate_limit_configs WHERE target_type = 'global'",
            )
            .fetch_one(&state.db)
            .await
        }
    }
    .unwrap_or(0);
    if exists > 0 {
        return conflict("A rate limit for this target already exists; edit it instead");
    }

    match models::create_ratelimit(
        &state.db,
        &req.target_type,
        target_id,
        req.qps,
        req.concurrency,
        &req.action,
    )
    .await
    {
        Ok(id) => {
            if let Err(e) = state.reload_rate_limits().await {
                tracing::warn!(error = %e, "Failed to reload rate limits");
            }
            ok_response(serde_json::json!({ "id": id }))
        }
        Err(e) => internal_error("create rate limit", e),
    }
}

pub async fn update_ratelimit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRateLimitRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(String, Option<i64>, i32, i32, String)> = match sqlx::query_as(
        "SELECT target_type, target_id, qps, concurrency, action FROM rate_limit_configs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => return internal_error("load rate limit", e),
    };

    let Some((target_type, target_id, qps, concurrency, action)) = current else {
        return not_found("Rate limit not found");
    };

    let qps = req.qps.unwrap_or(qps);
    let concurrency = req.concurrency.unwrap_or(concurrency);
    let action = req.action.unwrap_or(action);

    if let Err(message) = validate_rate_limit(qps, concurrency, &action) {
        return bad_request(&message);
    }

    match models::update_ratelimit(pool, id, qps, concurrency, &action).await {
        Ok(()) => {
            let _ = validate_rate_limit_target(pool, &target_type, target_id).await;
            if let Err(e) = state.reload_rate_limits().await {
                tracing::warn!(error = %e, "Failed to reload rate limits");
            }
            ok_response(serde_json::json!({ "updated": true }))
        }
        Err(e) => internal_error("update rate limit", e),
    }
}

pub async fn delete_ratelimit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match models::delete_ratelimit(&state.db, id).await {
        Ok(()) => {
            if let Err(e) = state.reload_rate_limits().await {
                tracing::warn!(error = %e, "Failed to reload rate limits");
            }
            ok_response(serde_json::json!({ "deleted": true }))
        }
        Err(e) => internal_error("delete rate limit", e),
    }
}

// === AppProfile endpoints ===

pub async fn list_app_profiles(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(20).clamp(1, 200);

    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM app_profiles").fetch_one(pool).await {
        Ok(total) => total,
        Err(e) => return internal_error("list app profiles", e),
    };

    match models::list_app_profiles(pool, page, page_size).await {
        Ok(profiles) => ok_response(serde_json::json!({ "items": profiles, "total": total })),
        Err(e) => internal_error("list app profiles", e),
    }
}

pub async fn create_app_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAppProfileRequest>,
) -> Response {
    let name = req.name.trim();
    let identifier = req.identifier.trim();
    if name.is_empty() || identifier.is_empty() {
        return bad_request("name and identifier are required");
    }
    if let Err(message) = validate_custom_headers(req.extra_headers.as_deref()) {
        return bad_request(&message);
    }

    match models::create_app_profile(
        &state.db,
        name,
        identifier,
        req.user_agent.trim(),
        req.extra_headers.as_deref(),
        req.description.as_deref(),
    )
    .await
    {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) if e.to_string().contains("UNIQUE") => {
            conflict("An app profile with that name or identifier already exists")
        }
        Err(e) => internal_error("create app profile", e),
    }
}

pub async fn update_app_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAppProfileRequest>,
) -> Response {
    if let Err(message) = validate_custom_headers(req.extra_headers.as_deref()) {
        return bad_request(&message);
    }

    let pool = &state.db;
    type ProfileRow = (String, String, Option<String>, Option<String>, bool);
    let current: Option<ProfileRow> = match sqlx::query_as(
        "SELECT name, user_agent, extra_headers, description, enabled FROM app_profiles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => return internal_error("load app profile", e),
    };

    let Some((name, user_agent, extra_headers, description, enabled)) = current else {
        return not_found("App profile not found");
    };

    match models::update_app_profile(
        pool,
        id,
        &req.name.unwrap_or(name),
        &req.user_agent.unwrap_or(user_agent),
        req.extra_headers.or(extra_headers).as_deref(),
        req.description.or(description).as_deref(),
        req.enabled.unwrap_or(enabled),
    )
    .await
    {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) if e.to_string().contains("UNIQUE") => conflict("App profile name already exists"),
        Err(e) => internal_error("update app profile", e),
    }
}

pub async fn delete_app_profile_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;

    let is_system: Option<bool> =
        match sqlx::query_scalar("SELECT is_system FROM app_profiles WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
        {
            Ok(value) => value,
            Err(e) => return internal_error("delete app profile", e),
        };

    match is_system {
        None => return not_found("App profile not found"),
        Some(true) => return bad_request("System app profiles cannot be deleted"),
        Some(false) => {}
    }

    // Remove plan slots that reference this profile first (FK constraint).
    let _ = sqlx::query("DELETE FROM traffic_plan_slots WHERE app_profile_id = ?")
        .bind(id)
        .execute(pool)
        .await;

    match models::delete_app_profile(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete app profile", e),
    }
}

// === Dashboard / stats ===

pub async fn dashboard_stats(State(state): State<Arc<AppState>>) -> Response {
    let pool = &state.db;

    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM request_logs")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let today_requests: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM request_logs WHERE created_at >= datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Distinct users (not distinct keys) active in the last day.
    let active_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT COALESCE(CAST(user_id AS TEXT), user_api_key))
         FROM request_logs WHERE created_at >= datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // P95 over the last 7 days keeps the query bounded on large log tables.
    let p95_ms: i64 = sqlx::query_scalar(
        "SELECT COALESCE(elapsed_ms, 0) FROM request_logs
         WHERE created_at >= datetime('now', '-7 day')
         ORDER BY elapsed_ms ASC
         LIMIT 1 OFFSET (
            SELECT COUNT(*) * 95 / 100 FROM request_logs
            WHERE created_at >= datetime('now', '-7 day')
         )",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let success_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM request_logs WHERE status_code < 400")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let failure_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM request_logs WHERE status_code >= 400")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let total_tokens: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(total_tokens), 0) FROM request_logs")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let error_rate = if total_requests > 0 {
        format!("{:.1}%", (failure_count as f64 / total_requests as f64) * 100.0)
    } else {
        "0.0%".to_string()
    };

    ok_response(serde_json::json!({
        "total_requests": total_requests,
        "today_requests": today_requests,
        "active_users": active_users,
        "success_count": success_count,
        "failure_count": failure_count,
        "p95_latency_ms": p95_ms,
        "error_rate": error_rate,
        "total_tokens": total_tokens,
    }))
}

/// SQL fragment restricting logs to one user (legacy rows matched by API key).
const USER_SCOPE_SQL: &str = "(?1 IS NULL OR user_id = ?1 OR user_api_key IN (
        SELECT k.key_value FROM user_keys k WHERE k.user_id = ?1
        UNION ALL SELECT u.api_key FROM users u WHERE u.id = ?1))";

pub async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogListQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 500);

    let filters = LogFilters {
        user_id: query.user_id,
        api_key_pattern: query.api_key.as_deref().filter(|v| !v.trim().is_empty()),
        model: query.model.as_deref().filter(|v| !v.trim().is_empty()),
        status_code_prefix: query
            .status_code
            .as_deref()
            .filter(|v| !v.trim().is_empty()),
        start_time: query.start_time.as_deref().filter(|v| !v.trim().is_empty()),
        end_time: query.end_time.as_deref().filter(|v| !v.trim().is_empty()),
    };

    let total = match models::count_logs(&state.db, &filters).await {
        Ok(total) => total,
        Err(e) => return internal_error("count logs", e),
    };

    match models::list_logs(&state.db, page, page_size, &filters).await {
        Ok(logs) => ok_response(serde_json::json!({ "items": logs, "total": total })),
        Err(e) => internal_error("list logs", e),
    }
}

pub async fn export_logs_csv(State(state): State<Arc<AppState>>) -> Response {
    let pool = &state.db;
    type CsvRow = (i64, String, Option<i64>, String, String, i32, i32, i32, i32, i32, String);
    let logs: Vec<CsvRow> =
        match sqlx::query_as(
            "SELECT id, user_api_key, channel_id, model, endpoint, status_code,
                    prompt_tokens, completion_tokens, total_tokens, elapsed_ms, created_at
             FROM request_logs ORDER BY created_at DESC LIMIT 10000",
        )
        .fetch_all(pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => return internal_error("export logs", e),
        };

    fn csv_escape(value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r')
        {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }

    let mut csv = String::from(
        "id,user_api_key,channel_id,model,endpoint,status_code,prompt_tokens,completion_tokens,total_tokens,elapsed_ms,created_at\n",
    );

    for (id, key, channel_id, model, endpoint, status, prompt, completion, total, elapsed, created) in
        logs
    {
        let masked_key: String = if key.chars().count() > 20 {
            format!("{}...", key.chars().take(17).collect::<String>())
        } else {
            key
        };

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            id,
            csv_escape(&masked_key),
            channel_id
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            csv_escape(&model),
            csv_escape(&endpoint),
            status,
            prompt,
            completion,
            total,
            elapsed,
            csv_escape(&created)
        ));
    }

    (
        StatusCode::OK,
        [
            ("Content-Type", "text/csv; charset=utf-8"),
            ("Content-Disposition", "attachment; filename=request_logs.csv"),
        ],
        csv,
    )
        .into_response()
}

/// Shared filtering for the statistics endpoints.
struct StatsScope {
    user_id: Option<i64>,
    api_key: Option<String>,
    channel_id: Option<i64>,
    model: Option<String>,
    days: i64,
}

impl StatsScope {
    fn since(&self) -> String {
        format!("-{} days", self.days.clamp(1, 365))
    }

    /// Owner filter combining `user_id` and an explicit API key.
    fn owner_clause(offset: usize) -> String {
        format!(
            "(?{offset} IS NULL OR user_id = ?{offset} OR user_api_key IN (
                SELECT k.key_value FROM user_keys k WHERE k.user_id = ?{offset}
                UNION ALL SELECT u.api_key FROM users u WHERE u.id = ?{offset}))"
        )
    }
}

pub async fn usage_stats(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<StatsFilterQuery>,
) -> Response {
    let pool = &state.db;
    let scope = StatsScope {
        user_id: filter.user_id,
        api_key: filter.user_api_key.clone().filter(|v| !v.trim().is_empty()),
        channel_id: filter.channel_id,
        model: filter.model.clone().filter(|v| !v.trim().is_empty()),
        days: filter.days.unwrap_or(7),
    };

    // All queries share the same parameter layout:
    //   ?1 user_id, ?2 api_key, ?3 channel_id, ?4 model, ?5 since
    let where_clause = format!(
        "WHERE {} \
         AND (?2 IS NULL OR user_api_key = ?2) \
         AND (?3 IS NULL OR channel_id = ?3) \
         AND (?4 IS NULL OR model = ?4) \
         AND created_at >= datetime('now', ?5)",
        StatsScope::owner_clause(1)
    );

    let daily_trend: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT strftime('%Y-%m-%d', created_at) AS day, COUNT(*) AS count
         FROM request_logs {where_clause} GROUP BY day ORDER BY day"
    ))
    .bind(scope.user_id)
    .bind(scope.api_key.as_deref())
    .bind(scope.channel_id)
    .bind(scope.model.as_deref())
    .bind(scope.since())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let channel_usage: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT COALESCE(c.name, 'Unknown'), COUNT(*)
         FROM request_logs r LEFT JOIN channels c ON c.id = r.channel_id
         {where_clause}
         GROUP BY r.channel_id ORDER BY COUNT(*) DESC LIMIT 10"
    ))
    .bind(scope.user_id)
    .bind(scope.api_key.as_deref())
    .bind(scope.channel_id)
    .bind(scope.model.as_deref())
    .bind(scope.since())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let top_users: Vec<(Option<i64>, String, i64)> = sqlx::query_as(&format!(
        "SELECT user_id, MIN(user_api_key), COUNT(*) AS count
         FROM request_logs {where_clause}
         GROUP BY COALESCE(CAST(user_id AS TEXT), user_api_key)
         ORDER BY count DESC LIMIT 10"
    ))
    .bind(scope.user_id)
    .bind(scope.api_key.as_deref())
    .bind(scope.channel_id)
    .bind(scope.model.as_deref())
    .bind(scope.since())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Resolve usernames for the top-user chart.
    let mut username_map: HashMap<i64, String> = HashMap::new();
    if let Ok(rows) = sqlx::query_as::<_, (i64, String)>("SELECT id, username FROM users")
        .fetch_all(pool)
        .await
    {
        username_map = rows.into_iter().collect();
    }

    let masked_top_users: Vec<Value> = top_users
        .iter()
        .map(|(user_id, key, count)| {
            let masked: String = if key.chars().count() > 8 {
                format!("{}...", key.chars().take(8).collect::<String>())
            } else {
                key.clone()
            };
            let name = user_id
                .and_then(|id| username_map.get(&id).cloned())
                .unwrap_or_else(|| masked.clone());
            serde_json::json!({
                "user_id": user_id,
                "username": name,
                "api_key": masked,
                "count": count,
            })
        })
        .collect();

    let total_tokens: i64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM request_logs {where_clause}"
    ))
    .bind(scope.user_id)
    .bind(scope.api_key.as_deref())
    .bind(scope.channel_id)
    .bind(scope.model.as_deref())
    .bind(scope.since())
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Daily token breakdown; `day` is a real alias (the old `GROUP BY day` without the
    // alias silently failed and produced an empty chart).
    let token_daily: Vec<(String, i64, i64)> = sqlx::query_as(&format!(
        "SELECT strftime('%Y-%m-%d', created_at) AS day,
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0)
         FROM request_logs {where_clause} GROUP BY day ORDER BY day"
    ))
    .bind(scope.user_id)
    .bind(scope.api_key.as_deref())
    .bind(scope.channel_id)
    .bind(scope.model.as_deref())
    .bind(scope.since())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let model_usage: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT model, COUNT(*) AS count FROM request_logs {where_clause}
         GROUP BY model ORDER BY count DESC LIMIT 10"
    ))
    .bind(scope.user_id)
    .bind(scope.api_key.as_deref())
    .bind(scope.channel_id)
    .bind(scope.model.as_deref())
    .bind(scope.since())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    ok_response(serde_json::json!({
        "daily_trend": daily_trend.iter().map(|(day, count)| serde_json::json!({"day": day, "count": count})).collect::<Vec<_>>(),
        "channel_usage": channel_usage.iter().map(|(name, count)| serde_json::json!({"name": name, "count": count})).collect::<Vec<_>>(),
        "top_users": masked_top_users,
        "total_tokens": total_tokens,
        "token_daily": token_daily.iter().map(|(day, prompt, completion)| serde_json::json!({
            "day": day, "prompt_tokens": prompt, "completion_tokens": completion
        })).collect::<Vec<_>>(),
        "model_usage": model_usage.iter().map(|(name, count)| serde_json::json!({"name": name, "count": count})).collect::<Vec<_>>(),
    }))
}

// === System config endpoints ===

/// GET /admin/system/config — read-only view of the effective configuration.
pub async fn get_system_config(State(state): State<Arc<AppState>>) -> Response {
    // Report the *effective* global rate limit (a database override wins over yaml).
    let global_rule = state.global_rate_rule().await;

    ok_response(serde_json::json!({
        "server": {
            "host": state.config.server.host,
            "port": state.config.server.port,
            "cors_allowed_origins": state.config.server.cors_allowed_origins,
        },
        "database": {
            "path": state.config.database.path,
            "pool_size": state.config.database.pool_size,
        },
        "log": {
            "level": state.config.log.level,
            "retention_days": state.retention_days(),
            "max_records": state.config.log.max_records,
        },
        "proxy": {
            "timeout": state.config.proxy.timeout,
            "max_retries": state.config.proxy.max_retries,
            "log_request_body": state.config.proxy.log_request_body,
            "log_response_body": state.config.proxy.log_response_body,
            "trusted_proxies": state.config.proxy.trusted_proxies,
        },
        "health_check": {
            "enabled": state.config.health_check.enabled,
            "interval_secs": state.config.health_check.interval_secs,
            "failure_threshold": state.config.health_check.failure_threshold,
            "timeout_secs": state.config.health_check.timeout_secs,
        },
        "security": {
            "encryption_key_count": state.encryption.key_count(),
            "encryption_secret_source": if state.config.security.encryption_secrets.is_empty() {
                "jwt_secret"
            } else {
                "configured"
            },
        },
        "global_rate_limit": {
            "qps": global_rule.qps,
            "concurrency": global_rule.concurrency,
            "action": global_rule.action,
        },
    }))
}

/// Persist a YAML path update (best effort; failures are logged, not fatal).
fn update_config_yaml(path: &str, value: serde_yaml::Value) -> Result<(), String> {
    let config_path = std::path::Path::new("config/config.yaml");
    let content = std::fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let mut cfg: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() != 2 {
        return Err("config path must be 'section.key'".to_string());
    }
    let section = parts[0];
    let key = parts[1];

    if let Some(section_value) = cfg.get_mut(section)
        && let Some(map) = section_value.as_mapping_mut() {
            map.insert(serde_yaml::Value::String(key.to_string()), value);
        }

    let new_yaml = serde_yaml::to_string(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path, new_yaml).map_err(|e| e.to_string())?;
    Ok(())
}

/// Set the log level at runtime and optionally the retention period.
pub async fn set_log_level(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetLogLevelRequest>,
) -> Response {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    let level = req.level.to_lowercase();
    if !valid_levels.contains(&level.as_str()) {
        return bad_request(&format!(
            "Invalid log level. Valid levels: {valid_levels:?}"
        ));
    }
    if let Some(days) = req.retention_days
        && days == 0 {
            return bad_request("retention_days must be at least 1");
        }

    match state
        .log_reload_handle
        .reload(EnvFilter::new(format!("coride_api={level}")))
    {
        Ok(()) => tracing::info!(new_level = level, "Log level updated at runtime"),
        Err(e) => return internal_error("reload log level", e),
    }

    if let Some(days) = req.retention_days {
        // Apply immediately for the cleanup task and persist for restarts.
        state
            .retention_days
            .store(days, std::sync::atomic::Ordering::Relaxed);
        if let Err(e) = update_config_yaml(
            "log.retention_days",
            serde_yaml::Value::Number(days.into()),
        ) {
            tracing::warn!(error = %e, "Failed to persist retention_days to config.yaml");
        }
    }

    ok_response(serde_json::json!({
        "message": format!("Log level set to '{level}' (applied immediately)"),
        "level": level,
        "retention_days": req.retention_days,
    }))
}

/// Update the global rate limit: persisted to config.yaml *and* applied immediately
/// through the rate limit table.
pub async fn set_global_rate_limit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetRateLimitRequest>,
) -> Response {
    if req.action != "reject" && req.action != "queue" {
        return bad_request("action must be 'reject' or 'queue'");
    }
    if req.qps < 1 || req.concurrency < 1 {
        return bad_request("qps and concurrency must be at least 1");
    }

    let pool = &state.db;
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return internal_error("set global rate limit", e),
    };

    if let Err(e) =
        sqlx::query("DELETE FROM rate_limit_configs WHERE target_type = 'global'")
            .execute(&mut *tx)
            .await
    {
        return internal_error("set global rate limit", e);
    }
    if let Err(e) = sqlx::query(
        "INSERT INTO rate_limit_configs (target_type, target_id, qps, concurrency, action) VALUES ('global', NULL, ?, ?, ?)",
    )
    .bind(req.qps)
    .bind(req.concurrency)
    .bind(&req.action)
    .execute(&mut *tx)
    .await
    {
        return internal_error("set global rate limit", e);
    }
    if let Err(e) = tx.commit().await {
        return internal_error("set global rate limit", e);
    }

    if let Err(e) = state.reload_rate_limits().await {
        return internal_error("reload rate limits", e);
    }

    // Best-effort persistence for restarts (the DB row is the source of truth at runtime).
    let _ = update_config_yaml("global_rate_limit.qps", serde_yaml::Value::Number(req.qps.into()));
    let _ = update_config_yaml(
        "global_rate_limit.concurrency",
        serde_yaml::Value::Number(req.concurrency.into()),
    );
    let _ = update_config_yaml(
        "global_rate_limit.action",
        serde_yaml::Value::String(req.action.clone()),
    );

    ok_response(serde_json::json!({
        "message": "Global rate limit applied immediately",
        "qps": req.qps,
        "concurrency": req.concurrency,
        "action": req.action,
    }))
}

/// GET /admin/quotas/warnings — quotas above 80% usage.
pub async fn quota_warnings(State(state): State<Arc<AppState>>) -> Response {
    let warnings: Vec<(i64, String, String, i64, i64, f64)> = sqlx::query_as(
        "SELECT u.id, u.username, q.quota_type, q.total_limit, q.used,
                ROUND(CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT) * 100, 1) AS pct
         FROM quotas q
         JOIN users u ON u.id = q.user_id
         WHERE q.enabled = 1
           AND q.total_limit > 0
           AND CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT) >= 0.8
         ORDER BY pct DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let warning_list: Vec<Value> = warnings
        .iter()
        .map(|(user_id, username, quota_type, total, used, percent)| {
            let severity = if *percent >= 100.0 {
                "critical"
            } else if *percent >= 90.0 {
                "warning"
            } else {
                "info"
            };
            serde_json::json!({
                "user_id": user_id,
                "username": username,
                "quota_type": quota_type,
                "total_limit": total,
                "used": used,
                "percent": percent,
                "severity": severity,
                "message": format!("用户 {username} 的 {quota_type} 配额已使用 {used}/{total} ({percent:.0}%)")
            })
        })
        .collect();

    ok_response(serde_json::json!({
        "warnings": warning_list,
        "total_warnings": warning_list.len(),
    }))
}

// === Traffic plan endpoints ===

fn validate_traffic_plan_slots(slots: &[TrafficPlanSlotRequest]) -> Result<(), String> {
    if slots.is_empty() {
        return Err("at least one time slot is required".to_string());
    }
    for (index, slot) in slots.iter().enumerate() {
        if slot.start_hour < 0 || slot.start_hour > 23 {
            return Err(format!("时段 {} 起始小时必须在 0-23 之间", index + 1));
        }
        if slot.end_hour < 1 || slot.end_hour > 24 {
            return Err(format!("时段 {} 结束小时必须在 1-24 之间", index + 1));
        }
        if slot.start_hour >= slot.end_hour {
            return Err(format!("时段 {} 起始时间必须小于结束时间", index + 1));
        }
        if slot.weight <= 0 {
            return Err(format!("时段 {} 权重必须大于 0", index + 1));
        }
        for (other_index, other) in slots.iter().enumerate() {
            if index != other_index
                && slot.start_hour < other.end_hour
                && other.start_hour < slot.end_hour
            {
                return Err(format!(
                    "时段 {} 与时段 {} 存在时间重叠",
                    index + 1,
                    other_index + 1
                ));
            }
        }
    }
    Ok(())
}

/// Ensure every referenced app profile exists (gives a clear error instead of an FK error).
async fn validate_plan_profiles(
    pool: &sqlx::SqlitePool,
    slots: &[TrafficPlanSlotRequest],
) -> Result<(), String> {
    for slot in slots {
        match models::get_app_profile_by_id(pool, slot.app_profile_id).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(format!(
                    "app_profile_id {} does not exist or is disabled",
                    slot.app_profile_id
                ))
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

pub async fn get_global_traffic_plan(State(state): State<Arc<AppState>>) -> Response {
    let pool = &state.db;
    match models::get_global_traffic_plan(pool).await {
        Ok(Some(plan)) => match models::get_traffic_plan_detail(pool, &plan).await {
            Ok(detail) => ok_response(detail),
            Err(e) => internal_error("get global plan", e),
        },
        Ok(None) => ok_response(serde_json::Value::Null),
        Err(e) => internal_error("get global plan", e),
    }
}

pub async fn upsert_global_traffic_plan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpsertTrafficPlanRequest>,
) -> Response {
    let pool = &state.db;

    if let Err(message) = validate_traffic_plan_slots(&req.slots) {
        return bad_request(&message);
    }
    if let Err(message) = validate_plan_profiles(pool, &req.slots).await {
        return bad_request(&message);
    }

    let plan_id = match models::get_global_traffic_plan(pool).await {
        Ok(Some(plan)) => plan.id,
        Ok(None) => match models::create_traffic_plan(pool, None).await {
            Ok(id) => id,
            Err(e) => return internal_error("create global plan", e),
        },
        Err(e) => return internal_error("get global plan", e),
    };

    if let Err(e) = models::delete_slots_by_plan_id(pool, plan_id).await {
        return internal_error("save global plan", e);
    }
    for slot in &req.slots {
        if let Err(e) = models::insert_traffic_plan_slot(
            pool,
            plan_id,
            slot.start_hour,
            slot.end_hour,
            slot.app_profile_id,
            slot.weight,
        )
        .await
        {
            return internal_error("save global plan", e);
        }
    }

    let plan = models::TrafficPlan {
        id: plan_id,
        channel_id: None,
        created_at: chrono::Utc::now(),
    };
    match models::get_traffic_plan_detail(pool, &plan).await {
        Ok(detail) => ok_response(detail),
        Err(e) => internal_error("get global plan", e),
    }
}

pub async fn delete_global_traffic_plan(State(state): State<Arc<AppState>>) -> Response {
    let pool = &state.db;
    match models::get_global_traffic_plan(pool).await {
        Ok(Some(plan)) => match models::delete_traffic_plan(pool, plan.id).await {
            Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
            Err(e) => internal_error("delete global plan", e),
        },
        Ok(None) => not_found("No global plan found"),
        Err(e) => internal_error("get global plan", e),
    }
}

pub async fn list_channel_traffic_plans(State(state): State<Arc<AppState>>) -> Response {
    match models::list_all_traffic_plans(&state.db).await {
        Ok(plans) => ok_response(plans),
        Err(e) => internal_error("list channel plans", e),
    }
}

pub async fn get_channel_traffic_plan(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::get_traffic_plan_by_channel(pool, channel_id).await {
        Ok(Some(plan)) => match models::get_traffic_plan_detail(pool, &plan).await {
            Ok(detail) => ok_response(detail),
            Err(e) => internal_error("get channel plan", e),
        },
        Ok(None) => ok_response(serde_json::Value::Null),
        Err(e) => internal_error("get channel plan", e),
    }
}

pub async fn upsert_channel_traffic_plan(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<i64>,
    Json(req): Json<UpsertTrafficPlanRequest>,
) -> Response {
    let pool = &state.db;

    if let Err(message) = validate_traffic_plan_slots(&req.slots) {
        return bad_request(&message);
    }
    if let Err(message) = validate_plan_profiles(pool, &req.slots).await {
        return bad_request(&message);
    }

    match models::get_channel_by_id(pool, channel_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found("Channel not found"),
        Err(e) => return internal_error("get channel", e),
    }

    let plan_id = match models::get_traffic_plan_by_channel(pool, channel_id).await {
        Ok(Some(plan)) => plan.id,
        Ok(None) => match models::create_traffic_plan(pool, Some(channel_id)).await {
            Ok(id) => id,
            Err(e) => return internal_error("create channel plan", e),
        },
        Err(e) => return internal_error("get channel plan", e),
    };

    if let Err(e) = models::delete_slots_by_plan_id(pool, plan_id).await {
        return internal_error("save channel plan", e);
    }
    for slot in &req.slots {
        if let Err(e) = models::insert_traffic_plan_slot(
            pool,
            plan_id,
            slot.start_hour,
            slot.end_hour,
            slot.app_profile_id,
            slot.weight,
        )
        .await
        {
            return internal_error("save channel plan", e);
        }
    }

    let plan = models::TrafficPlan {
        id: plan_id,
        channel_id: Some(channel_id),
        created_at: chrono::Utc::now(),
    };
    match models::get_traffic_plan_detail(pool, &plan).await {
        Ok(detail) => ok_response(detail),
        Err(e) => internal_error("get channel plan", e),
    }
}

pub async fn delete_channel_traffic_plan(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<i64>,
) -> Response {
    match models::delete_traffic_plan_by_channel(&state.db, channel_id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete channel plan", e),
    }
}

// === IP access control ===

pub async fn list_blacklist(State(state): State<Arc<AppState>>) -> Response {
    match sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, ip_address, created_at FROM ip_blacklist ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(entries) => ok_response(
            entries
                .iter()
                .map(|(id, ip, created)| {
                    serde_json::json!({"id": id, "ip_address": ip, "created_at": created})
                })
                .collect::<Vec<_>>(),
        ),
        Err(e) => internal_error("list blacklist", e),
    }
}

pub async fn add_blacklist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddBlacklistRequest>,
) -> Response {
    let normalised = match ip::normalise_rule(&req.ip_address) {
        Some(value) => value,
        None => {
            return bad_request(
                "Invalid IP address. Expected IPv4, IPv6 or CIDR (e.g. 192.168.1.1, ::1, 192.168.1.0/24)",
            )
        }
    };

    match sqlx::query("INSERT INTO ip_blacklist (ip_address) VALUES (?)")
        .bind(&normalised)
        .execute(&state.db)
        .await
    {
        Ok(result) => ok_response(serde_json::json!({ "id": result.last_insert_rowid() })),
        Err(e) if e.to_string().contains("UNIQUE") => conflict("IP already in blacklist"),
        Err(e) => internal_error("add blacklist entry", e),
    }
}

pub async fn delete_blacklist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match sqlx::query("DELETE FROM ip_blacklist WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => not_found("Blacklist entry not found"),
        Ok(_) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete blacklist entry", e),
    }
}

pub async fn list_user_whitelist(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Response {
    match sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, user_id, ip_address, created_at FROM ip_whitelist WHERE user_id = ? ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(entries) => ok_response(
            entries
                .iter()
                .map(|(id, user_id, ip, created)| {
                    serde_json::json!({"id": id, "user_id": user_id, "ip_address": ip, "created_at": created})
                })
                .collect::<Vec<_>>(),
        ),
        Err(e) => internal_error("list whitelist", e),
    }
}

pub async fn add_whitelist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddWhitelistRequest>,
) -> Response {
    let normalised = match ip::normalise_rule(&req.ip_address) {
        Some(value) => value,
        None => {
            return bad_request(
                "Invalid IP address. Expected IPv4, IPv6 or CIDR (e.g. 192.168.1.1, ::1, 192.168.1.0/24)",
            )
        }
    };

    match models::get_user_by_id(&state.db, req.user_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return bad_request("user_id does not exist"),
        Err(e) => return internal_error("add whitelist entry", e),
    }

    match sqlx::query("INSERT INTO ip_whitelist (user_id, ip_address) VALUES (?, ?)")
        .bind(req.user_id)
        .bind(&normalised)
        .execute(&state.db)
        .await
    {
        Ok(result) => ok_response(serde_json::json!({ "id": result.last_insert_rowid() })),
        Err(e) if e.to_string().contains("UNIQUE") => {
            conflict("IP already in this user's whitelist")
        }
        Err(e) => internal_error("add whitelist entry", e),
    }
}

pub async fn delete_whitelist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match sqlx::query("DELETE FROM ip_whitelist WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => not_found("Whitelist entry not found"),
        Ok(_) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete whitelist entry", e),
    }
}

// === User API key endpoints ===

pub async fn list_user_keys(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    match models::list_user_keys_by_user(&state.db, ctx.user_id).await {
        Ok(keys) => ok_response(keys),
        Err(e) => internal_error("list user keys", e),
    }
}

pub async fn create_user_key(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CreateUserKeyRequest>,
) -> Response {
    if let Err(message) = validate_model_list(req.enabled_models.as_deref()) {
        return bad_request(&message);
    }

    let key_value = format!("sk-{}", Uuid::new_v4().simple());
    match models::create_user_key(
        &state.db,
        ctx.user_id,
        &key_value,
        req.name.as_deref(),
        req.enabled_models.as_deref(),
    )
    .await
    {
        Ok(id) => ok_response(serde_json::json!({
            "id": id,
            "key_value": key_value,
            "name": req.name,
            "enabled_models": req.enabled_models,
            "status": "active",
        })),
        Err(e) => internal_error("create user key", e),
    }
}

pub async fn update_user_key(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(key_id): Path<i64>,
    Json(req): Json<UpdateUserKeyRequest>,
) -> Response {
    if let Err(message) = validate_model_list(req.enabled_models.as_deref()) {
        return bad_request(&message);
    }
    if let Some(status) = req.status.as_deref()
        && status != "active" && status != "disabled" {
            return bad_request("status must be 'active' or 'disabled'");
        }

    let current = match models::get_user_key_by_id(&state.db, key_id, ctx.user_id).await {
        Ok(Some(key)) => key,
        Ok(None) => return not_found("Key not found"),
        Err(e) => return internal_error("update user key", e),
    };

    let name = req.name.or(current.name);
    let enabled_models = req.enabled_models.or(current.enabled_models);
    let status = req.status.unwrap_or(current.status);

    match models::update_user_key(
        &state.db,
        key_id,
        ctx.user_id,
        name.as_deref(),
        enabled_models.as_deref(),
        &status,
    )
    .await
    {
        Ok(true) => ok_response(serde_json::json!({ "updated": true })),
        Ok(false) => not_found("Key not found"),
        Err(e) => internal_error("update user key", e),
    }
}

pub async fn delete_user_key(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(key_id): Path<i64>,
) -> Response {
    let key_value = match models::get_user_key_value(&state.db, key_id, ctx.user_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("Key not found"),
        Err(e) => return internal_error("delete user key", e),
    };

    // Deleting the primary key row would leave `users.api_key` valid — disable it instead.
    match models::key_is_primary(&state.db, &key_value, ctx.user_id).await {
        Ok(true) => {
            return bad_request(
                "This is the account's primary API key. Disable it instead, or reset it from user management.",
            )
        }
        Ok(false) => {}
        Err(e) => return internal_error("delete user key", e),
    }

    match models::delete_user_key(&state.db, key_id, ctx.user_id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => internal_error("delete user key", e),
    }
}

pub async fn list_all_user_keys(State(state): State<Arc<AppState>>) -> Response {
    match models::list_all_user_keys(&state.db).await {
        Ok(keys) => ok_response(keys),
        Err(e) => internal_error("list keys", e),
    }
}

pub async fn list_user_keys_by_user_id(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Response {
    match models::list_user_keys_by_user(&state.db, user_id).await {
        Ok(keys) => ok_response(keys),
        Err(e) => internal_error("list keys", e),
    }
}

pub async fn get_user_info(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    match models::get_user_by_id(&state.db, ctx.user_id).await {
        Ok(Some(user)) => ok_response(serde_json::json!({
            "id": user.id,
            "username": user.username,
            "role": user.role,
            "status": user.status,
            "enabled_models": user.enabled_models,
            "note": user.note,
            "created_at": user.created_at,
        })),
        Ok(None) => not_found("User not found"),
        Err(e) => internal_error("user info", e),
    }
}

// === User-scoped stats ===

pub async fn user_dashboard_stats(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> Response {
    let pool = &state.db;

    let total_requests: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM request_logs WHERE {}",
        USER_SCOPE_SQL
    ))
    .bind(ctx.user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let today_requests: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM request_logs WHERE {} AND created_at >= datetime('now', '-1 day')",
        USER_SCOPE_SQL
    ))
    .bind(ctx.user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let total_tokens: i64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE {}",
        USER_SCOPE_SQL
    ))
    .bind(ctx.user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let p95_ms: i64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(elapsed_ms, 0) FROM request_logs WHERE {scope}
         ORDER BY elapsed_ms ASC LIMIT 1 OFFSET (
             SELECT COUNT(*) * 95 / 100 FROM request_logs WHERE {scope}
         )",
        scope = format!("{USER_SCOPE_SQL} AND created_at >= datetime('now', '-7 day')")
    ))
    .bind(ctx.user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let failure_count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM request_logs WHERE {} AND status_code >= 400",
        USER_SCOPE_SQL
    ))
    .bind(ctx.user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let error_rate = if total_requests > 0 {
        format!("{:.1}%", (failure_count as f64 / total_requests as f64) * 100.0)
    } else {
        "0.0%".to_string()
    };

    ok_response(serde_json::json!({
        "total_requests": total_requests,
        "today_requests": today_requests,
        "active_users": 1,
        "total_tokens": total_tokens,
        "p95_latency_ms": p95_ms,
        "failure_count": failure_count,
        "error_rate": error_rate,
    }))
}

pub async fn user_usage_stats(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<UserStatsQuery>,
) -> Response {
    let pool = &state.db;
    let days = query.days.unwrap_or(7).clamp(1, 365);
    let since = format!("-{days} days");

    let where_clause = format!(
        "WHERE {} AND (?2 IS NULL OR channel_id = ?2) AND (?3 IS NULL OR model = ?3)
         AND created_at >= datetime('now', ?4)",
        USER_SCOPE_SQL
    );

    let daily_trend: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT strftime('%Y-%m-%d', created_at) AS day, COUNT(*) AS count
         FROM request_logs {where_clause} GROUP BY day ORDER BY day"
    ))
    .bind(ctx.user_id)
    .bind(query.channel)
    .bind(query.model.as_deref().filter(|v| !v.trim().is_empty()))
    .bind(&since)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let channel_usage: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT COALESCE(c.name, 'Unknown'), COUNT(*)
         FROM request_logs r LEFT JOIN channels c ON c.id = r.channel_id
         {where_clause}
         GROUP BY r.channel_id ORDER BY COUNT(*) DESC LIMIT 10"
    ))
    .bind(ctx.user_id)
    .bind(query.channel)
    .bind(query.model.as_deref().filter(|v| !v.trim().is_empty()))
    .bind(&since)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // `day` must be a select alias: referencing it in GROUP BY without the alias made
    // SQLite reject the query, which silently emptied the token chart.
    let token_daily: Vec<(String, i64, i64)> = sqlx::query_as(&format!(
        "SELECT strftime('%Y-%m-%d', created_at) AS day,
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0)
         FROM request_logs {where_clause} GROUP BY day ORDER BY day"
    ))
    .bind(ctx.user_id)
    .bind(query.channel)
    .bind(query.model.as_deref().filter(|v| !v.trim().is_empty()))
    .bind(&since)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let total_tokens: i64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE {}",
        USER_SCOPE_SQL
    ))
    .bind(ctx.user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let model_usage: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT model, COUNT(*) AS count FROM request_logs {where_clause}
         GROUP BY model ORDER BY count DESC LIMIT 10"
    ))
    .bind(ctx.user_id)
    .bind(query.channel)
    .bind(query.model.as_deref().filter(|v| !v.trim().is_empty()))
    .bind(&since)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    ok_response(serde_json::json!({
        "daily_trend": daily_trend.iter().map(|(day, count)| serde_json::json!({"day": day, "count": count})).collect::<Vec<_>>(),
        "channel_usage": channel_usage.iter().map(|(name, count)| serde_json::json!({"name": name, "count": count})).collect::<Vec<_>>(),
        "total_tokens": total_tokens,
        "token_daily": token_daily.iter().map(|(day, prompt, completion)| serde_json::json!({
            "day": day, "prompt_tokens": prompt, "completion_tokens": completion
        })).collect::<Vec<_>>(),
        "model_usage": model_usage.iter().map(|(name, count)| serde_json::json!({"name": name, "count": count})).collect::<Vec<_>>(),
    }))
}

pub async fn user_logs(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<UserLogsQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let filters = LogFilters {
        user_id: Some(ctx.user_id),
        api_key_pattern: None,
        model: query.model.as_deref().filter(|v| !v.trim().is_empty()),
        // `status_code` may be a full code (404) or a class prefix (4).
        status_code_prefix: query
            .status_code
            .as_deref()
            .filter(|v| !v.trim().is_empty()),
        start_time: query.start_time.as_deref().filter(|v| !v.trim().is_empty()),
        end_time: query.end_time.as_deref().filter(|v| !v.trim().is_empty()),
    };

    let total = match models::count_logs(&state.db, &filters).await {
        Ok(total) => total,
        Err(e) => return internal_error("count user logs", e),
    };

    match models::list_logs(&state.db, page, page_size, &filters).await {
        Ok(logs) => ok_response(serde_json::json!({ "items": logs, "total": total })),
        Err(e) => internal_error("list user logs", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_keys_validation_accepts_json_array() {
        assert!(validate_api_keys(r#"["sk-a"]"#).is_ok());
        assert!(validate_api_keys(r#"["sk-a","sk-b"]"#).is_ok());
    }

    #[test]
    fn api_keys_validation_rejects_bad_payloads() {
        assert!(validate_api_keys("[]").is_err());
        assert!(validate_api_keys("not-json").is_err());
        assert!(validate_api_keys(r#"["sk-a",""]"#).is_err());
    }

    #[test]
    fn channel_quota_validation_requires_complete_configuration() {
        assert!(validate_channel_quota(None, None, None).is_ok());
        assert!(validate_channel_quota(Some("requests"), Some(10), Some("daily")).is_ok());
        assert!(validate_channel_quota(Some("requests"), None, Some("daily")).is_err());
        assert!(validate_channel_quota(Some("requests"), Some(10), None).is_err());
        assert!(validate_channel_quota(Some("bogus"), Some(10), Some("daily")).is_err());
        assert!(validate_channel_quota(Some("tokens"), Some(0), Some("daily")).is_err());
    }

    #[test]
    fn base_url_validation_requires_http_scheme() {
        assert!(validate_base_url("https://api.example.com").is_ok());
        assert!(validate_base_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_base_url("api.example.com").is_err());
        assert!(validate_base_url("").is_err());
    }

    #[test]
    fn traffic_plan_validation_rejects_overlaps() {
        let slots = vec![
            TrafficPlanSlotRequest {
                start_hour: 0,
                end_hour: 12,
                app_profile_id: 1,
                weight: 1,
            },
            TrafficPlanSlotRequest {
                start_hour: 6,
                end_hour: 18,
                app_profile_id: 2,
                weight: 1,
            },
        ];
        assert!(validate_traffic_plan_slots(&slots).is_err());
    }

    #[test]
    fn traffic_plan_validation_accepts_adjacent_slots() {
        let slots = vec![
            TrafficPlanSlotRequest {
                start_hour: 0,
                end_hour: 12,
                app_profile_id: 1,
                weight: 1,
            },
            TrafficPlanSlotRequest {
                start_hour: 12,
                end_hour: 24,
                app_profile_id: 2,
                weight: 3,
            },
        ];
        assert!(validate_traffic_plan_slots(&slots).is_ok());
    }

    #[test]
    fn rate_limit_validation_rejects_negative_values() {
        assert!(validate_rate_limit(1, 1, "reject").is_ok());
        assert!(validate_rate_limit(-1, 1, "reject").is_err());
        assert!(validate_rate_limit(1, -1, "queue").is_err());
        assert!(validate_rate_limit(1, 1, "drop").is_err());
    }

    #[test]
    fn model_list_validation_accepts_null_and_arrays() {
        assert!(validate_model_list(None).is_ok());
        assert!(validate_model_list(Some("")).is_ok());
        assert!(validate_model_list(Some(r#"["a"]"#)).is_ok());
        assert!(validate_model_list(Some("a,b")).is_err());
    }
}
