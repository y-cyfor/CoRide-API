use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use crate::db::models::{self, AppProfile, ModelRoute, NewRequestLog};
use crate::middleware::auth::AuthContext;
use crate::middleware::rate_limit::RequestGuards;
use crate::service::{proxy, quota};
use crate::utils::rand::{weighted_order, Rng};
use crate::AppState;

/// Largest request body accepted by the proxy (10 MiB).
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Upstream statuses that are worth retrying on the *same* channel.
fn retry_same_channel(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504 | 529)
}

/// Upstream statuses that mean "this channel cannot serve us" — try the next one.
fn try_next_channel(status: u16) -> bool {
    matches!(status, 401 | 403 | 404 | 408 | 409 | 425 | 429)
        || (500..600).contains(&status)
}

fn is_anthropic_path(path: &str) -> bool {
    path.starts_with("/messages")
}

/// Build a protocol-appropriate error response with a real HTTP status code.
fn api_error(status: StatusCode, message: &str, anthropic: bool) -> Response {
    let body = if anthropic {
        serde_json::json!({
            "type": "error",
            "error": {
                "type": if status.is_server_error() { "api_error" } else { "invalid_request_error" },
                "message": message,
            }
        })
    } else {
        serde_json::json!({
            "error": {
                "type": if status.is_server_error() { "api_error" } else { "invalid_request_error" },
                "message": message,
                "code": status.as_u16(),
            }
        })
    };
    (status, Json(body)).into_response()
}

/// Pass an upstream body through unchanged, preserving its status and content type.
fn passthrough(status: StatusCode, body: String, headers: &[(String, String)]) -> Response {
    let mut builder = Response::builder().status(status);
    let mut has_content_type = false;
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            builder = builder.header(name, value);
        }
    }
    if !has_content_type {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

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

/// List models available to the caller (OpenAI-compatible shape).
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    let ctx = match req.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => return api_error(StatusCode::UNAUTHORIZED, "Invalid API key", false),
    };

    let pool = &state.db;
    let all_models = match models::list_models(pool, 1, 1000).await {
        Ok(models) => models,
        Err(e) => {
            tracing::error!(error = %e, "Failed to list models");
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list models",
                false,
            );
        }
    };

    let data: Vec<Value> = all_models
        .into_iter()
        .filter(|m| ctx.allows_model(&m.proxy_name))
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
        "object": "list",
        "data": data,
    }))
    .into_response()
}

/// Core proxy request handler.
async fn handle_proxy(
    state: &AppState,
    req: Request<Body>,
    path: &str,
    method: &str,
) -> Response {
    let anthropic = is_anthropic_path(path);

    let ctx = match req.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => return api_error(StatusCode::UNAUTHORIZED, "Invalid API key", anthropic),
    };

    // Move concurrency permits into the response lifetime (streaming) or let them drop
    // when this handler returns (non-streaming).
    let guards = req
        .extensions()
        .get::<RequestGuards>()
        .cloned()
        .unwrap_or_default();
    let client_headers = req.headers().clone();

    let body_bytes = match axum::body::to_bytes(req.into_body(), MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return api_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body is too large or could not be read",
                anthropic,
            )
        }
    };

    let body_str = match String::from_utf8(body_bytes.to_vec()) {
        Ok(text) => text,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "Request body must be valid UTF-8",
                anthropic,
            )
        }
    };

    let body_json: Value = match serde_json::from_str(&body_str) {
        Ok(value) => value,
        Err(e) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                &format!("Invalid JSON body: {e}"),
                anthropic,
            )
        }
    };

    let model_name = body_json
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if model_name.is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "Missing 'model' field", anthropic);
    }

    // Model permission enforcement (key-level list wins over the user list).
    if !ctx.allows_model(&model_name) {
        return api_error(
            StatusCode::FORBIDDEN,
            &format!("Model '{model_name}' is not enabled for this API key"),
            anthropic,
        );
    }

    let streaming = body_json
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let routes = match models::find_routes_for_model(&state.db, &model_name).await {
        Ok(routes) => routes,
        Err(e) => {
            tracing::error!(error = %e, "Failed to resolve model routes");
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to resolve model routes",
                anthropic,
            );
        }
    };

    if routes.is_empty() {
        return api_error(
            StatusCode::NOT_FOUND,
            &format!("Model '{model_name}' not found or has no active channel"),
            anthropic,
        );
    }

    let estimated_tokens = crate::utils::token_counter::estimate_tokens(&body_str) as i64;

    // Weighted-random channel order: heavier channels are preferred but every channel
    // gets a fair share, replacing the old "always the highest weight first" behaviour.
    let weights: Vec<i64> = routes
        .iter()
        .map(|route| route.channel.weight.max(0) as i64)
        .collect();
    let mut rng = Rng::from_entropy();
    let order = weighted_order(&weights, &mut rng);

    let snapshot = state.rate_limit_snapshot().await;
    let mut last_error: Option<(StatusCode, String)> = None;

    for route_index in order {
        let route = &routes[route_index];
        let channel = match prepare_channel(state, route) {
            Ok(channel) => channel,
            Err(message) => {
                tracing::error!(
                    channel_id = route.channel.id,
                    error = %message,
                    "Skipping channel: configuration cannot be used"
                );
                last_error = Some((StatusCode::BAD_GATEWAY, message));
                continue;
            }
        };

        // 1) Channel quota (request- or token-based, reset-aware).
        let quota_cost = quota::channel_quota_cost(channel.quota_type.as_deref(), estimated_tokens);
        match quota::check_channel_quota(&state.db, channel.id, quota_cost).await {
            Ok(true) => {}
            Ok(false) => {
                tracing::warn!(channel_id = channel.id, "Channel quota exhausted, trying next");
                last_error = Some((
                    StatusCode::PAYMENT_REQUIRED,
                    format!("Channel '{}' quota exhausted", channel.name),
                ));
                continue;
            }
            Err(e) => {
                tracing::warn!(error = %e, channel_id = channel.id, "Channel quota check failed");
                last_error = Some((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Quota check unavailable".to_string(),
                ));
                continue;
            }
        }

        // 2) User quota. Channel-scoped quotas fall through to the next channel.
        match quota::check_user_quota(&state.db, ctx.user_id, channel.id, estimated_tokens).await {
            Ok(verdict) if verdict.allowed => {}
            Ok(verdict) if verdict.channel_scoped => {
                tracing::info!(
                    user_id = ctx.user_id,
                    channel_id = channel.id,
                    reason = %verdict.message,
                    "Channel-scoped quota exhausted, trying next channel"
                );
                last_error = Some((StatusCode::PAYMENT_REQUIRED, verdict.message));
                continue;
            }
            Ok(verdict) => {
                return api_error(StatusCode::PAYMENT_REQUIRED, &verdict.message, anthropic)
            }
            Err(e) => {
                tracing::error!(error = %e, "User quota check failed");
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Quota check unavailable",
                    anthropic,
                );
            }
        }

        // 3) Per-channel rate limit.
        let mut channel_guard = None;
        if let Some(rule) = snapshot.channel(channel.id) {
            let key = format!("channel:{}", channel.id);
            let qps_ok = if rule.should_queue() {
                state
                    .limiters
                    .acquire_qps_wait(&key, rule.qps, Duration::from_secs(5))
                    .await
            } else {
                state.limiters.try_acquire_qps(&key, rule.qps)
            };
            if !qps_ok {
                last_error = Some((
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Channel '{}' rate limit exceeded", channel.name),
                ));
                continue;
            }

            match state
                .limiters
                .try_acquire_concurrency(&key, rule.concurrency)
            {
                crate::state::limiters::ConcurrencyOutcome::Unlimited => {}
                crate::state::limiters::ConcurrencyOutcome::Acquired(guard) => {
                    channel_guard = Some(guard)
                }
                crate::state::limiters::ConcurrencyOutcome::Denied => {
                    // Honour the `queue` action: wait briefly instead of rejecting.
                    let waited = if rule.should_queue() {
                        state
                            .limiters
                            .acquire_concurrency_wait(&key, rule.concurrency, Duration::from_secs(5))
                            .await
                    } else {
                        None
                    };
                    match waited {
                        Some(guard) => channel_guard = Some(guard),
                        None => {
                            last_error = Some((
                                StatusCode::TOO_MANY_REQUESTS,
                                format!("Channel '{}' concurrency limit reached", channel.name),
                            ));
                            continue;
                        }
                    }
                }
            }
        }

        // 4) Traffic plan / app profile.
        let app_profile: Option<AppProfile> =
            match models::resolve_app_profile_for_channel(&state.db, &channel).await {
                Ok(profile) => profile,
                Err(e) => {
                    tracing::warn!(error = %e, channel_id = channel.id, "Traffic plan resolution failed, proceeding without profile");
                    None
                }
            };

        // 5) Channel queue (bounded wait instead of an unbounded retry loop).
        let permit = match acquire_channel_permit(state, channel.id).await {
            Ok(permit) => permit,
            Err(()) => {
                last_error = Some((
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Channel '{}' is busy, please retry", channel.name),
                ));
                continue;
            }
        };

        let max_attempts = std::cmp::max(1, channel.retry_count);
        let mut attempt = 0;

        while attempt < max_attempts {
            attempt += 1;

            let mut outbound = proxy::rewrite_model(&body_json, &route.source_name);
            if streaming {
                proxy::inject_stream_usage(&mut outbound, channel.r#type == "anthropic");
            }

            if streaming {
                match proxy::open_stream(
                    state,
                    &channel,
                    method,
                    path,
                    &outbound,
                    app_profile.as_ref(),
                    Some(&client_headers),
                )
                .await
                {
                    Ok(stream) => {
                        let accounting = proxy::StreamAccounting {
                            pool: state.db.clone(),
                            user_id: ctx.user_id,
                            key_id: ctx.key_id,
                            api_key: ctx.api_key.clone(),
                            channel_id: channel.id,
                            channel_quota_type: channel.quota_type.clone(),
                            model: model_name.clone(),
                            endpoint: path.to_string(),
                            is_anthropic: channel.r#type == "anthropic",
                            started: Instant::now(),
                            request_body: if state.config.proxy.log_request_body {
                                Some(proxy::truncate_body(&body_str, 2_000))
                            } else {
                                None
                            },
                            log_response_body: state.config.proxy.log_response_body,
                            success_status: 200,
                        };

                        let mut all_guards = guards.take_all();
                        if let Some(channel_guard) = channel_guard.take() {
                            all_guards.push(channel_guard);
                        }

                        return proxy::stream_response(
                            stream,
                            accounting,
                            all_guards,
                            permit,
                            proxy::channel_timeout(state, &channel),
                        );
                    }
                    Err(proxy::ProxyError::UpstreamStatus { status, body }) => {
                        write_log(
                            state,
                            &ctx,
                            Some(channel.id),
                            &model_name,
                            path,
                            status as i32,
                            0,
                            0,
                            0,
                            0,
                            Some(&body),
                        )
                        .await;

                        let status_code =
                            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
                        last_error = Some((status_code, body.clone()));

                        if try_next_channel(status) {
                            break; // move on to the next channel
                        }
                        if retry_same_channel(status) && attempt < max_attempts {
                            continue;
                        }
                        return api_error(status_code, &body, anthropic);
                    }
                    Err(e) => {
                        tracing::warn!(
                            channel_id = channel.id,
                            attempt,
                            error = %e,
                            "Streaming request failed"
                        );
                        last_error = Some((e.status(), e.to_string()));
                        if attempt < max_attempts {
                            continue;
                        }
                        break;
                    }
                }
            } else {
                match proxy::proxy_request(
                    state,
                    &channel,
                    method,
                    path,
                    &outbound,
                    app_profile.as_ref(),
                    Some(&client_headers),
                )
                .await
                {
                    Ok(result) => {
                        let status = StatusCode::from_u16(result.status_code)
                            .unwrap_or(StatusCode::BAD_GATEWAY);

                        if status.is_success() {
                            log_success(state, &ctx, &channel, &model_name, path, &result, &body_str)
                                .await;

                            let _ = quota::deduct_user_quota(
                                &state.db,
                                ctx.user_id,
                                channel.id,
                                result.total_tokens as i64,
                            )
                            .await;
                            let _ = quota::deduct_channel_quota(
                                &state.db,
                                channel.id,
                                channel.quota_type.as_deref(),
                                result.total_tokens as i64,
                            )
                            .await;

                            return passthrough(status, result.body, &result.headers);
                        }

                        write_log(
                            state,
                            &ctx,
                            Some(channel.id),
                            &model_name,
                            path,
                            result.status_code as i32,
                            result.prompt_tokens,
                            result.completion_tokens,
                            result.total_tokens,
                            result.elapsed_ms,
                            Some(&proxy::truncate_body(&result.body, 4_000)),
                        )
                        .await;

                        if try_next_channel(result.status_code) {
                            last_error = Some((
                                status,
                                proxy::truncate_body(&result.body, 500),
                            ));
                            if retry_same_channel(result.status_code) && attempt < max_attempts {
                                continue;
                            }
                            break;
                        }

                        if retry_same_channel(result.status_code) && attempt < max_attempts {
                            last_error = Some((status, proxy::truncate_body(&result.body, 500)));
                            continue;
                        }

                        // Client error from upstream (400/413/422...) — return as-is.
                        return passthrough(status, result.body, &result.headers);
                    }
                    Err(e) => {
                        tracing::warn!(
                            channel_id = channel.id,
                            attempt,
                            error = %e,
                            "Upstream request failed"
                        );
                        last_error = Some((e.status(), e.to_string()));
                        if attempt < max_attempts {
                            continue;
                        }
                        break;
                    }
                }
            }
        }
    }

    let (status, message) = last_error.unwrap_or((
        StatusCode::BAD_GATEWAY,
        "All channels failed to serve this request".to_string(),
    ));
    api_error(status, &message, anthropic)
}

/// Decrypt the channel API keys, returning an error when they cannot be opened.
fn prepare_channel(state: &AppState, route: &ModelRoute) -> Result<models::Channel, String> {
    let mut channel = route.channel.clone();
    match models::decrypt_api_keys_checked(&channel.api_keys, &state.encryption) {
        Ok(plain) => {
            channel.api_keys = plain;
            Ok(channel)
        }
        Err(_) => Err(format!(
            "Channel '{}' API keys cannot be decrypted; check the encryption secret",
            channel.name
        )),
    }
}

/// Acquire a channel concurrency permit, waiting a bounded amount of time.
///
/// `Ok(None)` means the queue is unlimited (`channel_queue.max_concurrent = 0`);
/// `Err(())` means the channel stayed busy for the whole wait.
async fn acquire_channel_permit(
    state: &AppState,
    channel_id: i64,
) -> Result<Option<tokio::sync::OwnedSemaphorePermit>, ()> {
    let queue = state.config.channel_queue.clone();
    if queue.max_concurrent == 0 {
        return Ok(None); // unlimited
    }

    let capacity = queue.max_concurrent as usize;
    let semaphore = state
        .channel_semaphores
        .entry(channel_id)
        .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(capacity)))
        .clone();

    if let Ok(permit) = semaphore.clone().try_acquire_owned() {
        return Ok(Some(permit));
    }

    let wait_ms = (queue.max_retries.max(1) as u64) * queue.delay_max_ms.max(100);
    let wait = Duration::from_millis(wait_ms.min(30_000));
    match tokio::time::timeout(wait, semaphore.acquire_owned()).await {
        Ok(Ok(permit)) => Ok(Some(permit)),
        _ => Err(()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn write_log(
    state: &AppState,
    ctx: &AuthContext,
    channel_id: Option<i64>,
    model: &str,
    endpoint: &str,
    status_code: i32,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    elapsed_ms: i32,
    error_message: Option<&str>,
) {
    let log = NewRequestLog {
        user_api_key: &ctx.api_key,
        user_id: Some(ctx.user_id),
        key_id: ctx.key_id,
        channel_id,
        model,
        endpoint,
        status_code,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        elapsed_ms,
        request_body: None,
        response_body: None,
        error_message,
    };
    if let Err(e) = models::create_request_log(&state.db, &log).await {
        tracing::warn!(error = %e, "Failed to write request log");
    }
}

async fn log_success(
    state: &AppState,
    ctx: &AuthContext,
    channel: &models::Channel,
    model: &str,
    endpoint: &str,
    result: &proxy::ProxyResult,
    request_body: &str,
) {
    let request_body = if state.config.proxy.log_request_body {
        Some(proxy::truncate_body(request_body, 2_000))
    } else {
        None
    };
    let response_body = if state.config.proxy.log_response_body {
        Some(proxy::truncate_body(&result.body, 2_000))
    } else {
        None
    };

    let log = NewRequestLog {
        user_api_key: &ctx.api_key,
        user_id: Some(ctx.user_id),
        key_id: ctx.key_id,
        channel_id: Some(channel.id),
        model,
        endpoint,
        status_code: result.status_code as i32,
        prompt_tokens: result.prompt_tokens,
        completion_tokens: result.completion_tokens,
        total_tokens: result.total_tokens,
        elapsed_ms: result.elapsed_ms,
        request_body: request_body.as_deref(),
        response_body: response_body.as_deref(),
        error_message: None,
    };
    if let Err(e) = models::create_request_log(&state.db, &log).await {
        tracing::warn!(error = %e, "Failed to write request log");
    }
}

/// Get current user info (quota, available models, status).
pub async fn user_info(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    let ctx = match req.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => return api_error(StatusCode::UNAUTHORIZED, "Invalid API key", false),
    };

    let pool = &state.db;

    let user = match models::get_user_by_id(pool, ctx.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return api_error(StatusCode::NOT_FOUND, "User not found", false),
        Err(e) => {
            tracing::error!(error = %e, "Failed to load user");
            return api_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal error", false);
        }
    };

    let quotas = models::get_active_quotas(pool, ctx.user_id)
        .await
        .unwrap_or_default();

    let quota_info: Vec<Value> = quotas
        .iter()
        .map(|q| {
            serde_json::json!({
                "id": q.id,
                "quota_type": q.quota_type,
                "total_limit": q.total_limit,
                "used": q.used,
                "cycle": q.cycle,
                "channel_id": q.channel_id,
                "period_end": q.period_end,
                "remaining": (q.total_limit - q.used).max(0),
            })
        })
        .collect();

    let all_models = models::list_models(pool, 1, 1000).await.unwrap_or_default();
    let available_models: Vec<String> = all_models
        .into_iter()
        .filter(|m| ctx.allows_model(&m.proxy_name))
        .map(|m| m.proxy_name)
        .collect();

    let masked_key = if ctx.api_key.chars().count() > 8 {
        let prefix: String = ctx.api_key.chars().take(8).collect();
        format!("{prefix}****")
    } else {
        ctx.api_key.clone()
    };

    Json(serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": {
            "id": user.id,
            "username": user.username,
            "role": user.role,
            "status": user.status,
            "api_key": masked_key,
            "quotas": quota_info,
            "available_models": available_models,
            "note": user.note,
        }
    }))
    .into_response()
}

/// Helper: collect header values for tests.
#[allow(dead_code)]
fn header_map_to_pairs(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect()
}
