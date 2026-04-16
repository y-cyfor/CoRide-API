use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use crate::db::models::{self, AppProfile};
use crate::service::proxy;
use crate::service::quota;
use crate::AppState;

/// OpenAI chat completions proxy.
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    handle_proxy(&state, req, "/chat/completions", "POST").await
}

/// OpenAI completions proxy.
pub async fn completions(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    handle_proxy(&state, req, "/completions", "POST").await
}

/// Anthropic messages proxy.
pub async fn messages(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    handle_proxy(&state, req, "/messages", "POST").await
}

/// List available models.
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    // Get user_id from request extensions (injected by auth middleware)
    let user_id = match req.extensions().get::<i64>() {
        Some(id) => *id,
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid API key"),
    };

    let pool = &state.db;

    // Get all enabled models
    let all_models = match models::list_models(pool, 1, 1000).await {
        Ok(m) => m,
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
    };

    // Get user to filter by enabled_models
    let user = match models::get_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        _ => return error_response(StatusCode::UNAUTHORIZED, "User not found"),
    };

    let filtered_models = if let Some(ref enabled_str) = user.enabled_models {
        if let Ok(enabled_list) = serde_json::from_str::<Vec<String>>(enabled_str) {
            all_models.into_iter().filter(|m| enabled_list.contains(&m.proxy_name)).collect()
        } else {
            all_models
        }
    } else {
        all_models
    };

    let data: Vec<Value> = filtered_models
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.proxy_name,
                "object": "model",
                "created": m.created_at.timestamp(),
                "owned_by": "coride_api",
            })
        })
        .collect();

    Json(serde_json::json!({
        "code": 0,
        "data": data,
    })).into_response()
}

/// Check if the request body asks for streaming.
fn is_stream_request(body: &Value) -> bool {
    body.get("stream").and_then(|v| v.as_bool()) == Some(true)
}

/// Core proxy request handler.
async fn handle_proxy(
    state: &AppState,
    req: Request<Body>,
    path: &str,
    method: &str,
) -> Response {
    // Get user_id and role from request extensions (injected by auth middleware)
    let user_id = match req.extensions().get::<i64>() {
        Some(id) => *id,
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid API key"),
    };

    // Get api_key for logging
    let api_key = match req.extensions().get::<String>() {
        Some(key) => key.clone(),
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid API key"),
    };

    let pool = &state.db;

    // Parse request body (limit to 10MB to prevent DoS)
    const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
    let body_bytes = match axum::body::to_bytes(req.into_body(), MAX_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid request body"),
    };

    let body_str = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid UTF-8"),
    };

    // Extract model name from request body
    let body_json: Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON body"),
    };

    let model_name = body_json
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if model_name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Missing model field");
    }

    let streaming = is_stream_request(&body_json);

    // Find channels for this model
    let channels = match models::find_channels_for_model(pool, model_name).await {
        Ok(ch) => ch,
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
    };

    if channels.is_empty() {
        return error_response(
            StatusCode::NOT_FOUND,
            &format!("Model '{}' not found or no active channels", model_name),
        );
    }

    // Try each channel until one succeeds
    let mut last_error = None;
    for channel in channels {
        let max_attempts = std::cmp::max(1, channel.retry_count);
        let mut attempt = 0;

        while attempt < max_attempts {
            attempt += 1;

            // Check channel quota FIRST (before user quota to avoid pre-deduction if channel has no quota)
            if let Err(e) = quota::check_channel_quota(pool, channel.id).await {
                last_error = Some((StatusCode::PAYMENT_REQUIRED, e.to_string()));
                break; // Don't retry if quota exceeded
            }

            // Check user quota AFTER channel quota
            let estimated_tokens = crate::utils::token_counter::estimate_tokens(&body_str) as i32;
            if let Err(e) = quota::check_user_quota(pool, user_id, estimated_tokens).await {
                return error_response(StatusCode::PAYMENT_REQUIRED, &e.to_string());
            }

            // Resolve app profile via traffic plan (per-channel → global → legacy)
            let app_profile: Option<AppProfile> = match models::resolve_app_profile_for_channel(pool, &channel).await {
                Ok(profile) => profile,
                Err(e) => {
                    tracing::warn!(error = %e, channel_id = channel.id, "Failed to resolve app profile, proceeding without profile");
                    None
                }
            };

            // For streaming requests, use stream passthrough with app profile
            if streaming {
                match proxy::proxy_request_stream(
                    state,
                    &channel,
                    method,
                    path,
                    &body_str,
                    app_profile.as_ref(),
                )
                .await
                {
                    Ok(resp) => {
                        // Deduct quota (estimate only for streaming, actual tokens unknown until stream ends)
                        let _ = quota::deduct_user_quota(pool, user_id, estimated_tokens).await;
                        let _ = quota::deduct_channel_quota(pool, channel.id, estimated_tokens).await;

                        // Log request with streaming marker
                        let req_body = if state.config.proxy.log_request_body {
                            Some(body_str.as_str())
                        } else {
                            None
                        };
                        let _ = crate::service::log::log_request(
                            pool,
                            &api_key,
                            Some(channel.id),
                            model_name,
                            path,
                            200,
                            estimated_tokens,
                            0, // completion unknown for streaming
                            estimated_tokens,
                            0, // elapsed not yet known
                            req_body,
                            None,
                            None,
                        ).await;

                        return resp;
                    }
                    Err(e) => {
                        last_error = Some((StatusCode::BAD_GATEWAY, e.to_string()));
                    }
                }
            } else {
                // Non-streaming: use full response mode
                match proxy::proxy_request(
                    state,
                    &channel,
                    method,
                    path,
                    &body_str,
                    app_profile.as_ref(),
                    state.config.proxy.log_request_body,
                    state.config.proxy.log_response_body,
                )
                .await
                {
                    Ok(result) => {
                        // If successful or non-retryable error (2xx, 3xx, 4xx except 502/503/504)
                        if result.status_code < 500 || result.status_code == 400 || result.status_code == 401 || result.status_code == 403 {
                            // Deduct quota on success
                            if (200..300).contains(&result.status_code) {
                                let _ = quota::deduct_user_quota(pool, user_id, result.total_tokens).await;
                                let _ = quota::deduct_channel_quota(pool, channel.id, result.total_tokens).await;
                            }

                            // Log request
                            let req_body = if state.config.proxy.log_request_body {
                                Some(body_str.as_str())
                            } else {
                                None
                            };
                            let resp_body = if state.config.proxy.log_response_body && result.status_code >= 400 {
                                Some(result.body.as_str())
                            } else {
                                None
                            };
                            let _ = crate::service::log::log_request(
                                pool,
                                &api_key,
                                Some(channel.id),
                                model_name,
                                path,
                                result.status_code as i32,
                                result.prompt_tokens,
                                result.completion_tokens,
                                result.total_tokens,
                                result.elapsed_ms,
                                req_body,
                                resp_body,
                                if result.status_code >= 400 { Some(result.body.as_str()) } else { None },
                            ).await;

                            // Return the response
                            if let Ok(json_resp) = serde_json::from_str::<Value>(&result.body) {
                                return Json(serde_json::json!({
                                    "code": 0,
                                    "data": json_resp,
                                })).into_response();
                            }

                            return (
                                StatusCode::from_u16(result.status_code).unwrap_or(StatusCode::OK),
                                result.body,
                            )
                                .into_response();
                        }

                        // Server error (5xx) - retry if attempts left
                        last_error = Some((StatusCode::BAD_GATEWAY, format!("Server error (attempt {}/{})", result.status_code, max_attempts)));
                    }
                    Err(e) => {
                        last_error = Some((StatusCode::BAD_GATEWAY, e.to_string()));
                    }
                }
            }
        }
    }

    // All channels failed
    let (status, msg) =
        last_error.unwrap_or((StatusCode::BAD_GATEWAY, "All channels failed".to_string()));
    error_response(status, &msg)
}

fn error_response(status: StatusCode, message: &str) -> Response {
    Json(serde_json::json!({
        "code": status.as_u16(),
        "message": message,
    }))
    .into_response()
}

fn ok_response<T: serde::Serialize>(data: T) -> Response {
    Json(serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": data,
    })).into_response()
}

/// Get current user info (quota, available models, status).
pub async fn user_info(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    let user_id = match req.extensions().get::<i64>() {
        Some(id) => *id,
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid API key"),
    };

    let api_key = match req.extensions().get::<String>() {
        Some(key) => key.clone(),
        None => return error_response(StatusCode::UNAUTHORIZED, "Invalid API key"),
    };

    let pool = &state.db;

    // Get user info
    let user = match models::get_user_by_id(pool, user_id).await {
        Ok(Some(u)) => u,
        _ => return error_response(StatusCode::NOT_FOUND, "User not found"),
    };

    // Get active quotas
    let quotas = match models::get_active_quotas(pool, user_id).await {
        Ok(q) => q,
        Err(_) => vec![],
    };

    let quota_info: Vec<Value> = quotas.iter().map(|q| {
        serde_json::json!({
            "id": q.id,
            "quota_type": q.quota_type,
            "total_limit": q.total_limit,
            "used": q.used,
            "cycle": q.cycle,
            "remaining": q.total_limit - q.used,
        })
    }).collect();

    // Get available models
    let all_models = match models::list_models(pool, 1, 1000).await {
        Ok(m) => m,
        Err(_) => vec![],
    };

    let available_models: Vec<String> = if let Some(ref enabled_str) = user.enabled_models {
        if let Ok(enabled_list) = serde_json::from_str::<Vec<String>>(enabled_str) {
            all_models.into_iter().filter(|m| enabled_list.contains(&m.proxy_name)).map(|m| m.proxy_name).collect()
        } else {
            all_models.into_iter().map(|m| m.proxy_name).collect()
        }
    } else {
        all_models.into_iter().map(|m| m.proxy_name).collect()
    };

    // Mask API key
    let masked_key = if api_key.len() > 8 {
        format!("{}****", &api_key[..8])
    } else {
        api_key.clone()
    };

    ok_response(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "role": user.role,
        "status": user.status,
        "api_key": masked_key,
        "quotas": quota_info,
        "available_models": available_models,
        "note": user.note,
    }))
}
