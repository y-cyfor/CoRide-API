use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::db::models;

// Use sqlx for raw queries in update handlers

// === Request DTOs ===

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<String>,
    pub note: Option<String>,
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
    pub model: Option<String>,
    pub status_code: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatsFilterQuery {
    pub user_api_key: Option<String>,
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
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub channel_id: Option<i64>,
    pub source_name: Option<String>,
    pub proxy_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuotaRequest {
    pub quota_type: Option<String>,
    pub total_limit: Option<i64>,
    pub cycle: Option<String>,
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
}

// === Response helper ===

fn ok_response<T: serde::Serialize>(data: T) -> Response {
    Json(serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": data,
    })).into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    Json(serde_json::json!({
        "code": status.as_u16(),
        "message": message,
    })).into_response()
}

// === Auth endpoints ===

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Response {
    let pool = &state.db;
    let user = match models::get_user_by_username(pool, &req.username).await {
        Ok(Some(u)) => u,
        Ok(None) | Err(_) => return error_response(StatusCode::UNAUTHORIZED, "Invalid username or password"),
    };

    if bcrypt::verify(&req.password, &user.password_hash).unwrap_or(false) {
        let token = match crate::utils::jwt::generate_token(
            user.id,
            &user.username,
            &user.role,
            &state.config.jwt.secret,
            state.config.jwt.expires_in as i64,
        ) {
            Ok(t) => t,
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Token generation failed: {}", e)),
        };
        return ok_response(serde_json::json!({
            "token": token,
            "user": {
                "id": user.id,
                "username": user.username,
                "role": user.role,
            }
        }));
    }

    error_response(StatusCode::UNAUTHORIZED, "Invalid username or password")
}

pub async fn get_me(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = match req.extensions().get::<i64>() {
        Some(id) => *id,
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid token"),
    };

    let pool = &state.db;
    match models::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => ok_response(serde_json::json!({
            "id": user.id,
            "username": user.username,
            "role": user.role,
            "status": user.status,
            "created_at": user.created_at,
        })),
        _ => error_response(StatusCode::NOT_FOUND, "User not found"),
    }
}

// === User endpoints ===

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    match models::list_users(pool, page, page_size).await {
        Ok(users) => {
            // Enrich each user with their quota usage info
            let mut result = Vec::new();
            for user in users {
                let quota_info: Option<(i64, i64)> = sqlx::query_as(
                    "SELECT COALESCE(SUM(total_limit), 0) as total_limit, COALESCE(SUM(used), 0) as used FROM quotas WHERE user_id = ? AND enabled = 1",
                )
                .bind(user.id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();

                let (quota_total, quota_used) = quota_info.unwrap_or((0, 0));
                let mut obj = serde_json::to_value(&user).unwrap_or_default();
                if let Some(obj_obj) = obj.as_object_mut() {
                    obj_obj.insert("quota_total".to_string(), serde_json::json!(quota_total));
                    obj_obj.insert("quota_used".to_string(), serde_json::json!(quota_used));
                }
                result.push(obj);
            }
            ok_response(result)
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Response {
    let pool = &state.db;
    let password_hash = match bcrypt::hash(&req.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let api_key = format!("sk-{}", Uuid::new_v4().simple());
    let role = req.role.unwrap_or_else(|| "user".to_string());

    match models::create_user(pool, &req.username, &password_hash, &role, &api_key, "active", None, req.note.as_deref()).await {
        Ok(id) => ok_response(serde_json::json!({
            "id": id,
            "api_key": api_key,
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn reset_user_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    let new_key = format!("lp-{}", Uuid::new_v4().simple());
    match models::update_user_api_key(pool, id, &new_key).await {
        Ok(()) => ok_response(serde_json::json!({ "api_key": new_key })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn delete_user_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_user(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === Channel endpoints ===

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    // Get total count
    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM channels").fetch_one(pool).await {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    match models::list_channels(pool, page, page_size).await {
        Ok(channels) => ok_response(serde_json::json!({
            "items": channels,
            "total": total,
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateChannelRequest>,
) -> Response {
    let pool = &state.db;
    match models::create_channel(
        pool, &req.name, &req.r#type, &req.base_url, &req.api_keys,
        req.custom_headers.as_deref(),
        req.weight.unwrap_or(1), req.timeout.unwrap_or(300000),
        req.retry_count.unwrap_or(1),
        req.quota_type.as_deref(), req.quota_limit, req.quota_cycle.as_deref(),
        req.app_profile_id,
    ).await {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn delete_channel_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_channel(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    let channel = match models::get_channel_by_id(pool, id).await {
        Ok(Some(c)) => c,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Channel not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    // Test connectivity by sending a simple request to the base URL
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let test_url = format!("{}/models", channel.base_url.trim_end_matches('/'));

    // Get first API key
    let keys: Vec<String> = match serde_json::from_str(&channel.api_keys) {
        Ok(k) => k,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid API keys format"),
    };
    if keys.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "No API key configured");
    }
    let api_key = &keys[0];

    // Build request
    let mut req_builder = client.get(&test_url);
    if channel.r#type == "anthropic" {
        req_builder = req_builder.header("x-api-key", api_key);
    } else {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
    }

    match req_builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            let status_code = status.as_u16();

            // Try to read response body for more context
            let body_summary = match resp.text().await {
                Ok(body) => {
                    let truncated = if body.len() > 200 {
                        format!("{}...", &body[..200])
                    } else {
                        body
                    };
                    truncated
                }
                Err(_) => String::from("Unable to read response body"),
            };

            if status.is_success() {
                ok_response(serde_json::json!({
                    "status": "ok",
                    "http_status": status_code,
                    "message": "渠道连通性测试成功",
                    "response_body": body_summary
                }))
            } else {
                // Provide friendly hints for common error codes
                let hint = match status_code {
                    401 => "API Key 可能无效或未配置，请检查 API Key 是否正确",
                    403 => "该 API Key 无权访问 /models 端点，请确认 Key 权限",
                    404 => "BaseURL 可能不正确，或该供应商不支持 /models 端点",
                    429 => "请求频率超限，请稍后再试",
                    500..=599 => "上游服务异常，请稍后再试",
                    _ => "请检查 BaseURL 和 API Key 配置是否正确",
                };

                ok_response(serde_json::json!({
                    "status": "warning",
                    "http_status": status_code,
                    "message": format!("渠道返回状态码 {}，{}", status_code, hint),
                    "response_body": body_summary
                }))
            }
        }
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &format!("连接上游服务失败: {}", e)),
    }
}

// === Model endpoints ===

pub async fn list_models_endpoint(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    match models::list_models(pool, page, page_size).await {
        Ok(models_list) => ok_response(models_list),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn create_model_endpoint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateModelRequest>,
) -> Response {
    let pool = &state.db;
    match models::create_model(pool, req.channel_id, &req.source_name, &req.proxy_name).await {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn delete_model_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_model(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === Quota endpoints ===

pub async fn list_quotas(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    match models::list_quotas(pool, pagination.user_id, page, page_size).await {
        Ok(quotas) => ok_response(quotas),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn create_quota(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateQuotaRequest>,
) -> Response {
    let pool = &state.db;
    let (period_start, period_end) = crate::service::quota::init_quota_period(&req.cycle);

    let result = sqlx::query(
        "INSERT INTO quotas (user_id, quota_type, total_limit, cycle, period_start, period_end, enabled) VALUES (?, ?, ?, ?, ?, ?, true)",
    )
    .bind(req.user_id)
    .bind(&req.quota_type)
    .bind(req.total_limit)
    .bind(&req.cycle)
    .bind(period_start)
    .bind(period_end)
    .execute(pool);

    match result.await {
        Ok(r) => ok_response(serde_json::json!({ "id": r.last_insert_rowid() })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn delete_quota(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_quota(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === RateLimit endpoints ===

pub async fn list_ratelimits(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    match models::list_ratelimits(pool).await {
        Ok(configs) => ok_response(configs),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn create_ratelimit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRateLimitRequest>,
) -> Response {
    // Validate target_type: only 'user' and 'channel' allowed (global is in system settings)
    if req.target_type != "user" && req.target_type != "channel" {
        return error_response(StatusCode::BAD_REQUEST, "Invalid target_type. Must be 'user' or 'channel'");
    }
    // Validate target_id is present for user/channel targets
    if req.target_id.is_none() {
        return error_response(StatusCode::BAD_REQUEST, "target_id is required for user/channel rate limits");
    }

    let pool = &state.db;
    match models::create_ratelimit(pool, &req.target_type, req.target_id, req.qps, req.concurrency, &req.action).await {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn delete_ratelimit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_ratelimit(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === AppProfile endpoints ===

pub async fn list_app_profiles(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Response {
    let pool = &state.db;
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);

    match models::list_app_profiles(pool, page, page_size).await {
        Ok(profiles) => ok_response(profiles),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn create_app_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAppProfileRequest>,
) -> Response {
    let pool = &state.db;
    match models::create_app_profile(pool, &req.name, &req.identifier, &req.user_agent, req.extra_headers.as_deref(), req.description.as_deref()).await {
        Ok(id) => ok_response(serde_json::json!({ "id": id })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn delete_app_profile_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_app_profile(pool, id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === Stats endpoints ===

pub async fn dashboard_stats(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    let total_requests: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM request_logs")
        .fetch_one(pool)
        .await
        .ok();

    let today_requests: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM request_logs WHERE created_at >= datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await
    .ok();

    let active_users: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_api_key) FROM request_logs WHERE created_at >= datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await
    .ok();

    // P95 latency (last 1000 requests)
    let p95_ms: Option<i64> = sqlx::query_scalar(
        "SELECT elapsed_ms FROM request_logs ORDER BY elapsed_ms DESC LIMIT 1 OFFSET (SELECT COUNT(*) * 95 / 100 FROM request_logs)"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Error rate (4xx and 5xx)
    let error_count: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM request_logs WHERE status_code >= 400",
    )
    .fetch_one(pool)
    .await
    .ok();

    let total = total_requests.unwrap_or(0);
    let errors = error_count.unwrap_or(0);
    let error_rate = if total > 0 {
        format!("{:.1}%", (errors as f64 / total as f64) * 100.0)
    } else {
        "0.0%".to_string()
    };

    ok_response(serde_json::json!({
        "total_requests": total,
        "today_requests": today_requests.unwrap_or(0),
        "active_users": active_users.unwrap_or(0),
        "p95_latency_ms": p95_ms.unwrap_or(0),
        "error_rate": error_rate,
    }))
}

// === Log endpoints ===

pub async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogListQuery>,
) -> Response {
    let pool = &state.db;
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    match models::list_logs(pool, page, page_size, None, query.model.as_deref(), query.status_code.as_deref()).await {
        Ok(logs) => ok_response(logs),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === Update endpoints ===

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(String, String, String, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT username, role, status, enabled_models, note FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let (username, role, status, enabled_models, note) = match current {
        Some((u, r, s, em, n)) => (
            req.username.unwrap_or(u),
            req.role.unwrap_or(r),
            req.status.unwrap_or(s),
            req.enabled_models.or(em),
            req.note.or(n),
        ),
        None => return error_response(StatusCode::NOT_FOUND, "User not found"),
    };

    match models::update_user(pool, id, &username, &role, &status, enabled_models.as_deref(), note.as_deref()).await {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateChannelRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(String, String, String, String, Option<String>, i32, i32, i32, Option<String>, Option<i64>, Option<String>, Option<i64>, String)> =
        sqlx::query_as("SELECT name, type, base_url, api_keys, custom_headers, weight, timeout, retry_count, quota_type, quota_limit, quota_cycle, app_profile_id, status FROM channels WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let (name, channel_type, base_url, api_keys, custom_headers, weight, timeout, retry_count, quota_type, quota_limit, quota_cycle, app_profile_id, status) = match current {
        Some((n, t, b, a, ch, w, to, r, qt, ql, qc, ap, s)) => (
            req.name.unwrap_or(n),
            req.r#type.unwrap_or(t),
            req.base_url.unwrap_or(b),
            req.api_keys.unwrap_or(a),
            req.custom_headers.or(ch),
            req.weight.unwrap_or(w),
            req.timeout.unwrap_or(to),
            req.retry_count.unwrap_or(r),
            req.quota_type.or(qt),
            req.quota_limit.or(ql),
            req.quota_cycle.or(qc),
            req.app_profile_id.or(ap),
            req.status.unwrap_or(s),
        ),
        None => return error_response(StatusCode::NOT_FOUND, "Channel not found"),
    };

    match models::update_channel(
        pool, id, &name, &channel_type, &base_url, &api_keys,
        custom_headers.as_deref(), weight, timeout, retry_count,
        quota_type.as_deref(), quota_limit, quota_cycle.as_deref(),
        app_profile_id, &status,
    ).await {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn update_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateModelRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(i64, String, String, bool, bool)> =
        sqlx::query_as("SELECT channel_id, source_name, proxy_name, enabled, is_default FROM models WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let (channel_id, source_name, proxy_name, enabled, is_default) = match current {
        Some((c, s, p, e, d)) => (
            req.channel_id.unwrap_or(c),
            req.source_name.unwrap_or(s),
            req.proxy_name.unwrap_or(p),
            e, d,
        ),
        None => return error_response(StatusCode::NOT_FOUND, "Model not found"),
    };

    match sqlx::query(
        "UPDATE models SET channel_id=?, source_name=?, proxy_name=?, enabled=?, is_default=? WHERE id=?",
    )
    .bind(channel_id)
    .bind(&source_name)
    .bind(&proxy_name)
    .bind(enabled)
    .bind(is_default)
    .bind(id)
    .execute(pool)
    .await {
        Ok(_) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn update_quota(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateQuotaRequest>,
) -> Response {
    let pool = &state.db;
    // update_quota only changes total_limit and enabled flag
    let current: Option<(i64, bool)> =
        sqlx::query_as("SELECT total_limit, enabled FROM quotas WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let total_limit = match current {
        Some((t, _)) => req.total_limit.unwrap_or(t),
        None => return error_response(StatusCode::NOT_FOUND, "Quota not found"),
    };
    let enabled = true; // keep enabled on update

    match models::update_quota(pool, id, total_limit, enabled).await {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn update_ratelimit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRateLimitRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(i32, i32, String)> =
        sqlx::query_as("SELECT qps, concurrency, action FROM rate_limit_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let (qps, concurrency, action) = match current {
        Some((q, c, a)) => (
            req.qps.unwrap_or(q),
            req.concurrency.unwrap_or(c),
            req.action.unwrap_or(a),
        ),
        None => return error_response(StatusCode::NOT_FOUND, "RateLimit not found"),
    };

    match models::update_ratelimit(pool, id, qps, concurrency, &action).await {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

pub async fn update_app_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAppProfileRequest>,
) -> Response {
    let pool = &state.db;
    let current: Option<(String, String, Option<String>, Option<String>, bool)> =
        sqlx::query_as("SELECT name, user_agent, extra_headers, description, enabled FROM app_profiles WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let (name, user_agent, extra_headers, description, enabled) = match current {
        Some((n, u, e, d, en)) => (
            req.name.unwrap_or(n),
            req.user_agent.unwrap_or(u),
            req.extra_headers.or(e),
            req.description.or(d),
            en,
        ),
        None => return error_response(StatusCode::NOT_FOUND, "AppProfile not found"),
    };

    match models::update_app_profile(
        pool, id, &name, &user_agent,
        extra_headers.as_deref(), description.as_deref(), enabled,
    ).await {
        Ok(()) => ok_response(serde_json::json!({ "updated": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// === Advanced Stats endpoints ===

pub async fn usage_stats(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<StatsFilterQuery>,
) -> Response {
    let pool = &state.db;
    let days = filter.days.unwrap_or(7);

    // Daily request trend (last N days)
    let daily_trend: Vec<(String, i64)> = match filter.user_api_key.as_ref() {
        Some(user_key) => {
            sqlx::query_as(
                "SELECT strftime('%Y-%m-%d', created_at) as day, COUNT(*) as count
                 FROM request_logs WHERE user_api_key = ? AND created_at >= datetime('now', ? || ' days')
                 GROUP BY day ORDER BY day"
            )
            .bind(user_key)
            .bind(format!("-{}", days))
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        }
        None => {
            sqlx::query_as(
                "SELECT strftime('%Y-%m-%d', created_at) as day, COUNT(*) as count
                 FROM request_logs
                 WHERE created_at >= datetime('now', ? || ' days')
                 GROUP BY day ORDER BY day"
            )
            .bind(format!("-{}", days))
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        }
    };

    // Channel usage distribution
    let channel_usage: Vec<(String, i64)> = if filter.model.is_some() || filter.user_api_key.is_some() {
        let user_clause = filter.user_api_key.as_ref().map(|_| "r.user_api_key = ?").unwrap_or("1=1");
        let model_clause = filter.model.as_ref().map(|_| "r.model = ?").unwrap_or("1=1");
        let sql = format!("SELECT COALESCE(c.name, 'Unknown') as name, COUNT(*) as count
             FROM request_logs r
             LEFT JOIN channels c ON c.id = r.channel_id
             WHERE {} AND {} AND created_at >= datetime('now', ? || ' days')
             GROUP BY r.channel_id ORDER BY count DESC LIMIT 10", user_clause, model_clause);
        let mut q = sqlx::query_as(&sql);
        if let Some(ref u) = filter.user_api_key { q = q.bind(u); }
        if let Some(ref m) = filter.model { q = q.bind(m); }
        q.bind(format!("-{}", days))
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT COALESCE(c.name, 'Unknown') as name, COUNT(*) as count
             FROM request_logs r
             LEFT JOIN channels c ON c.id = r.channel_id
             WHERE r.channel_id = COALESCE(?, r.channel_id) AND created_at >= datetime('now', ? || ' days')
             GROUP BY r.channel_id ORDER BY count DESC LIMIT 10"
        )
        .bind(filter.channel_id)
        .bind(format!("-{}", days))
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    // Top users by request count (with masked API keys)
    let top_users: Vec<(String, i64)> = if filter.channel_id.is_some() || filter.model.is_some() {
        let channel_clause = filter.channel_id.as_ref().map(|_| "r.channel_id = ?").unwrap_or("1=1");
        let model_clause = filter.model.as_ref().map(|_| "r.model = ?").unwrap_or("1=1");
        let sql = format!("SELECT r.user_api_key, COUNT(*) as count
             FROM request_logs r
             WHERE {} AND {} AND created_at >= datetime('now', ? || ' days')
             GROUP BY r.user_api_key ORDER BY count DESC LIMIT 10", channel_clause, model_clause);
        let mut q = sqlx::query_as(&sql);
        if let Some(cid) = filter.channel_id { q = q.bind(cid); }
        if let Some(ref m) = filter.model { q = q.bind(m); }
        q.bind(format!("-{}", days))
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT user_api_key, COUNT(*) as count
             FROM request_logs
             GROUP BY user_api_key ORDER BY count DESC LIMIT 10"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    // Mask API keys: show first 8 chars + "..."
    let masked_top_users: Vec<_> = top_users.iter().map(|(key, count)| {
        let masked = if key.len() > 8 {
            format!("{}...", &key[..8])
        } else {
            key.clone()
        };
        serde_json::json!({"api_key": masked, "count": count})
    }).collect();

    // Total token consumption
    let total_tokens: Option<i64> = if filter.user_api_key.is_some() || filter.channel_id.is_some() || filter.model.is_some() {
        let user_clause = filter.user_api_key.as_ref().map(|_| "user_api_key = ?").unwrap_or("1=1");
        let channel_clause = filter.channel_id.as_ref().map(|_| "channel_id = ?").unwrap_or("1=1");
        let model_clause = filter.model.as_ref().map(|_| "model = ?").unwrap_or("1=1");
        let sql = format!("SELECT COALESCE(SUM(total_tokens), 0) FROM request_logs WHERE {} AND {} AND {}", user_clause, channel_clause, model_clause);
        let mut q = sqlx::query_scalar(&sql);
        if let Some(ref u) = filter.user_api_key { q = q.bind(u); }
        if let Some(cid) = filter.channel_id { q = q.bind(cid); }
        if let Some(ref m) = filter.model { q = q.bind(m); }
        q.fetch_one(pool).await.ok()
    } else {
        sqlx::query_scalar("SELECT COALESCE(SUM(total_tokens), 0) FROM request_logs")
            .fetch_one(pool)
            .await
            .ok()
    };

    // Daily token breakdown (prompt vs completion)
    let token_daily: Vec<(String, i64, i64)> = if filter.user_api_key.is_some() || filter.channel_id.is_some() || filter.model.is_some() {
        let user_clause = filter.user_api_key.as_ref().map(|_| "user_api_key = ?").unwrap_or("1=1");
        let channel_clause = filter.channel_id.as_ref().map(|_| "channel_id = ?").unwrap_or("1=1");
        let model_clause = filter.model.as_ref().map(|_| "model = ?").unwrap_or("1=1");
        let sql = format!("SELECT strftime('%Y-%m-%d', created_at) as day,
                COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
                COALESCE(SUM(completion_tokens), 0) as completion_tokens
             FROM request_logs WHERE {} AND {} AND {} AND created_at >= datetime('now', ? || ' days')
             GROUP BY day ORDER BY day", user_clause, channel_clause, model_clause);
        let mut q = sqlx::query_as(&sql);
        if let Some(ref u) = filter.user_api_key { q = q.bind(u); }
        if let Some(cid) = filter.channel_id { q = q.bind(cid); }
        if let Some(ref m) = filter.model { q = q.bind(m); }
        q.bind(format!("-{}", days))
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT strftime('%Y-%m-%d', created_at) as day,
                    COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
                    COALESCE(SUM(completion_tokens), 0) as completion_tokens
             FROM request_logs
             WHERE created_at >= datetime('now', ? || ' days')
             GROUP BY day ORDER BY day"
        )
        .bind(format!("-{}", days))
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    ok_response(serde_json::json!({
        "daily_trend": daily_trend.iter().map(|(day, count)| serde_json::json!({"day": day, "count": count})).collect::<Vec<_>>(),
        "channel_usage": channel_usage.iter().map(|(name, count)| serde_json::json!({"name": name, "count": count})).collect::<Vec<_>>(),
        "top_users": masked_top_users,
        "total_tokens": total_tokens.unwrap_or(0),
        "token_daily": token_daily.iter().map(|(day, pt, ct)| serde_json::json!({"day": day, "prompt_tokens": pt, "completion_tokens": ct})).collect::<Vec<_>>(),
    }))
}

// === Export CSV endpoint ===

pub async fn export_logs_csv(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    let logs: Vec<(i64, String, Option<i64>, String, String, i32, i32, i32, i32, i32, String)> =
        sqlx::query_as(
            "SELECT id, user_api_key, channel_id, model, endpoint, status_code,
                    prompt_tokens, completion_tokens, total_tokens, elapsed_ms, created_at
             FROM request_logs ORDER BY created_at DESC LIMIT 10000"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let mut csv = String::from("id,user_api_key,channel_id,model,endpoint,status_code,prompt_tokens,completion_tokens,total_tokens,elapsed_ms,created_at\n");
    for (id, key, ch, model, endpoint, status, pt, ct, tt, ms, created) in logs {
        csv.push_str(&format!(
            "{},{},{},\"{}\",\"{}\",{},{},{},{},{},{}\n",
            id, key, ch.unwrap_or(0), model.replace('"', "\"\""),
            endpoint.replace('"', "\"\""), status, pt, ct, tt, ms, created
        ));
    }

    (
        axum::http::StatusCode::OK,
        [
            ("Content-Type", "text/csv"),
            ("Content-Disposition", "attachment; filename=request_logs.csv"),
        ],
        csv,
    ).into_response()
}

// === System config endpoints ===

#[derive(Debug, Deserialize)]
pub struct SetLogLevelRequest {
    pub level: String,
}

/// Get current system config (read-only view of config.yaml values)
pub async fn get_system_config(
    State(state): State<Arc<AppState>>,
) -> Response {
    ok_response(serde_json::json!({
        "server": {
            "host": state.config.server.host,
            "port": state.config.server.port,
        },
        "database": {
            "path": state.config.database.path,
            "pool_size": state.config.database.pool_size,
        },
        "log": {
            "level": state.config.log.level,
            "retention_days": state.config.log.retention_days,
            "max_records": state.config.log.max_records,
        },
        "proxy": {
            "timeout": state.config.proxy.timeout,
            "max_retries": state.config.proxy.max_retries,
            "log_request_body": state.config.proxy.log_request_body,
            "log_response_body": state.config.proxy.log_response_body,
        },
        "global_rate_limit": {
            "qps": state.config.global_rate_limit.qps,
            "concurrency": state.config.global_rate_limit.concurrency,
            "action": state.config.global_rate_limit.action,
        },
    }))
}

/// Set log level at runtime (uses tracing-subscriber reload)
pub async fn set_log_level(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SetLogLevelRequest>,
) -> Response {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    let level = req.level.to_lowercase();
    if !valid_levels.contains(&level.as_str()) {
        return error_response(StatusCode::BAD_REQUEST, &format!("Invalid log level. Valid levels: {:?}", valid_levels));
    }

    tracing::info!(new_level = level, "Log level change requested (requires restart for full effect)");

    ok_response(serde_json::json!({
        "message": format!("Log level set to '{}'. Note: Some components may require restart.", level),
        "level": level,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SetRateLimitRequest {
    pub qps: i32,
    pub concurrency: i32,
    pub action: String,
}

/// Update global rate limit config in config.yaml (requires restart)
pub async fn set_global_rate_limit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetRateLimitRequest>,
) -> Response {
    let valid_actions = ["reject", "queue"];
    if !valid_actions.contains(&req.action.as_str()) {
        return error_response(StatusCode::BAD_REQUEST, "Invalid action. Must be 'reject' or 'queue'");
    }
    if req.qps < 1 || req.concurrency < 1 {
        return error_response(StatusCode::BAD_REQUEST, "QPS and concurrency must be at least 1");
    }

    // Update config.yaml
    let config_path = std::path::Path::new("config/config.yaml");
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            let mut new_content = content;
            // Replace rate limit values using simple string replacement
            let qps_pattern = format!("qps: {}", state.config.global_rate_limit.qps);
            let concurrency_pattern = format!("concurrency: {}", state.config.global_rate_limit.concurrency);
            let action_pattern = format!("action: \"{}\"", state.config.global_rate_limit.action);

            new_content = new_content.replace(&qps_pattern, &format!("qps: {}", req.qps));
            new_content = new_content.replace(&concurrency_pattern, &format!("concurrency: {}", req.concurrency));
            new_content = new_content.replace(&action_pattern, &format!("action: \"{}\"", req.action));

            if let Err(e) = std::fs::write(config_path, &new_content) {
                tracing::error!(error = %e, "Failed to write config.yaml");
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write config: {}", e));
            }

            tracing::info!(qps = req.qps, concurrency = req.concurrency, action = req.action, "Global rate limit updated (requires restart)");
            ok_response(serde_json::json!({
                "message": "Rate limit updated. Restart required for changes to take effect.",
                "qps": req.qps,
                "concurrency": req.concurrency,
                "action": req.action,
            }))
        }
        Err(e) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to read config: {}", e))
        }
    }
}

/// Get quota warnings for all users (usage > 80%)
pub async fn quota_warnings(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;

    // Get all active quotas with usage > 80%
    let warnings: Vec<(i64, String, String, i64, i64, f64)> = sqlx::query_as(
        "SELECT u.id, u.username, q.quota_type, q.total_limit, q.used,
                ROUND(CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT) * 100, 1) as pct
         FROM quotas q
         JOIN users u ON u.id = q.user_id
         WHERE q.enabled = 1
         AND CAST(q.used AS FLOAT) / CAST(q.total_limit AS FLOAT) >= 0.8
         ORDER BY pct DESC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let warning_list: Vec<_> = warnings.iter().map(|(uid, username, qtype, total, used, pct)| {
        let severity = if *pct >= 100.0 { "critical" } else if *pct >= 90.0 { "warning" } else { "info" };
        serde_json::json!({
            "user_id": uid,
            "username": username,
            "quota_type": qtype,
            "total_limit": total,
            "used": used,
            "percent": pct,
            "severity": severity,
            "message": format!("用户 {} 的 {} 配额已使用 {}/{} ({:.0}%)", username, qtype, used, total, pct)
        })
    }).collect();

    ok_response(serde_json::json!({
        "warnings": warning_list,
        "total_warnings": warning_list.len(),
    }))
}

// === Traffic Plan endpoints ===

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

/// GET /admin/traffic-plan/global
pub async fn get_global_traffic_plan(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    match models::get_global_traffic_plan(pool).await {
        Ok(Some(plan)) => {
            match models::get_traffic_plan_detail(pool, &plan).await {
                Ok(detail) => ok_response(detail),
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            }
        }
        Ok(None) => ok_response(serde_json::json!(null)),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// PUT /admin/traffic-plan/global
pub async fn upsert_global_traffic_plan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpsertTrafficPlanRequest>,
) -> Response {
    let pool = &state.db;

    let plan_id = match models::get_global_traffic_plan(pool).await {
        Ok(Some(p)) => p.id,
        Ok(None) => {
            match models::create_traffic_plan(pool, None).await {
                Ok(id) => id,
                Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            }
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    if let Err(e) = models::delete_slots_by_plan_id(pool, plan_id).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    for slot in &req.slots {
        if let Err(e) = models::insert_traffic_plan_slot(
            pool, plan_id, slot.start_hour, slot.end_hour, slot.app_profile_id, slot.weight,
        ).await {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    }

    let plan = models::TrafficPlan { id: plan_id, channel_id: None, created_at: chrono::Utc::now() };
    match models::get_traffic_plan_detail(pool, &plan).await {
        Ok(detail) => ok_response(detail),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// DELETE /admin/traffic-plan/global
pub async fn delete_global_traffic_plan(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    match models::get_global_traffic_plan(pool).await {
        Ok(Some(plan)) => {
            if let Err(e) = models::delete_traffic_plan(pool, plan.id).await {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
            }
            ok_response(serde_json::json!({ "deleted": true }))
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "No global plan found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// GET /admin/traffic-plan/channels
pub async fn list_channel_traffic_plans(
    State(state): State<Arc<AppState>>,
) -> Response {
    let pool = &state.db;
    match models::list_all_traffic_plans(pool).await {
        Ok(plans) => ok_response(plans),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// GET /admin/traffic-plan/channel/{id}
pub async fn get_channel_traffic_plan(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::get_traffic_plan_by_channel(pool, channel_id).await {
        Ok(Some(plan)) => {
            match models::get_traffic_plan_detail(pool, &plan).await {
                Ok(detail) => ok_response(detail),
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            }
        }
        Ok(None) => ok_response(serde_json::json!(null)),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// PUT /admin/traffic-plan/channel/{id}
pub async fn upsert_channel_traffic_plan(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<i64>,
    Json(req): Json<UpsertTrafficPlanRequest>,
) -> Response {
    let pool = &state.db;

    // Verify channel exists
    match models::get_channel_by_id(pool, channel_id).await {
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Channel not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        _ => {}
    }

    let plan_id = match models::get_traffic_plan_by_channel(pool, channel_id).await {
        Ok(Some(p)) => p.id,
        Ok(None) => {
            match models::create_traffic_plan(pool, Some(channel_id)).await {
                Ok(id) => id,
                Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            }
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    if let Err(e) = models::delete_slots_by_plan_id(pool, plan_id).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    for slot in &req.slots {
        if let Err(e) = models::insert_traffic_plan_slot(
            pool, plan_id, slot.start_hour, slot.end_hour, slot.app_profile_id, slot.weight,
        ).await {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    }

    let plan = models::TrafficPlan { id: plan_id, channel_id: Some(channel_id), created_at: chrono::Utc::now() };
    match models::get_traffic_plan_detail(pool, &plan).await {
        Ok(detail) => ok_response(detail),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// DELETE /admin/traffic-plan/channel/{id}
pub async fn delete_channel_traffic_plan(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<i64>,
) -> Response {
    let pool = &state.db;
    match models::delete_traffic_plan_by_channel(pool, channel_id).await {
        Ok(()) => ok_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
