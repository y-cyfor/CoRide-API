use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::middleware::auth::AuthContext;
use crate::middleware::ip_filter::login_client_ip;
use crate::state::limiters::{ConcurrencyGuard, ConcurrencyOutcome};
use crate::state::rate_limits::RateLimitRule;
use crate::AppState;

/// Concurrency permits held for the current request.
///
/// Wrapped in `Arc<Mutex<..>>` so the value can live in request extensions (which
/// require `Clone`) while the guards themselves are moved exactly once — into the
/// streaming task when the response body outlives the handler.
#[derive(Clone, Default)]
pub struct RequestGuards(
    std::sync::Arc<std::sync::Mutex<Vec<ConcurrencyGuard>>>,
);

impl RequestGuards {
    pub fn push(&self, guard: ConcurrencyGuard) {
        if let Ok(mut guards) = self.0.lock() {
            guards.push(guard);
        }
    }

    /// Move every guard out, transferring ownership to the caller.
    pub fn take_all(&self) -> Vec<ConcurrencyGuard> {
        match self.0.lock() {
            Ok(mut guards) => std::mem::take(&mut *guards),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        }
    }
}

/// Maximum time a `queue` action may wait before the request is rejected.
const QUEUE_TIMEOUT: Duration = Duration::from_secs(30);

fn rate_limited(message: &str, retry_after_secs: u64) -> Response {
    let body = json!({
        "error": {
            "message": message,
            "type": "rate_limit_error",
            "code": "rate_limit_exceeded",
        }
    });
    let mut response = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
    if let Ok(value) = header::HeaderValue::from_str(&retry_after_secs.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

fn attach_guard(request: &mut Request<Body>, guard: ConcurrencyGuard) {
    // `Extensions` requires `Clone` for get_mut/remove, so guards travel inside an Arc.
    let guards = request
        .extensions()
        .get::<RequestGuards>()
        .cloned()
        .unwrap_or_default();
    guards.push(guard);
    request.extensions_mut().insert(guards);
}

/// Apply a rule's QPS and concurrency limits.
///
/// Returns `Ok(None)` when unlimited, `Ok(Some(guard))` when a slot was reserved, or an
/// error response when the request must be rejected.
async fn enforce(
    state: &AppState,
    key: &str,
    rule: &RateLimitRule,
) -> Result<Option<ConcurrencyGuard>, Response> {
    if rule.qps > 0 {
        let allowed = if rule.should_queue() {
            state
                .limiters
                .acquire_qps_wait(key, rule.qps, QUEUE_TIMEOUT)
                .await
        } else {
            state.limiters.try_acquire_qps(key, rule.qps)
        };

        if !allowed {
            return Err(rate_limited(
                "Rate limit exceeded, please retry later",
                1,
            ));
        }
    }

    if rule.concurrency > 0 {
        let outcome = if rule.should_queue() {
            state
                .limiters
                .acquire_concurrency_wait(key, rule.concurrency, QUEUE_TIMEOUT)
                .await
                .map(ConcurrencyOutcome::Acquired)
                .unwrap_or(ConcurrencyOutcome::Denied)
        } else {
            state
                .limiters
                .try_acquire_concurrency(key, rule.concurrency)
        };

        match outcome {
            ConcurrencyOutcome::Unlimited => return Ok(None),
            ConcurrencyOutcome::Acquired(guard) => return Ok(Some(guard)),
            ConcurrencyOutcome::Denied => {
                return Err(rate_limited(
                    "Too many concurrent requests, please retry later",
                    1,
                ))
            }
        }
    }

    Ok(None)
}

/// Global rate limiting middleware (configuration-driven fallback + DB overrides).
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let rule = state.global_rate_rule().await;

    match enforce(&state, "global", &rule).await {
        Ok(Some(guard)) => attach_guard(&mut request, guard),
        Ok(None) => {}
        Err(response) => return response,
    }

    next.run(request).await
}

/// Per-user rate limiting middleware. Must run after `auth_middleware`.
pub async fn user_rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let Some(user_id) = request
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.user_id)
    else {
        return next.run(request).await;
    };

    let rule = state
        .rate_limit_snapshot()
        .await
        .user(user_id)
        .cloned();

    if let Some(rule) = rule {
        let key = format!("user:{user_id}");
        match enforce(&state, &key, &rule).await {
            Ok(Some(guard)) => attach_guard(&mut request, guard),
            Ok(None) => {}
            Err(response) => return response,
        }
    }

    next.run(request).await
}

/// Per-IP rate limiting middleware for the login endpoint.
/// Allows 5 login attempts per minute per client IP.
pub async fn login_rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    const MAX_LOGIN_ATTEMPTS_PER_MINUTE: u32 = 5;
    const MAX_TRACKED_IPS: usize = 10_000;

    // Bound memory: a scanner hitting random IPs must not grow this map forever.
    if state.login_rate_limiters.len() > MAX_TRACKED_IPS {
        state.login_rate_limiters.clear();
        tracing::warn!("Cleared login rate limiter cache to bound memory usage");
    }

    let mut request = request;
    if request.extensions().get::<std::net::SocketAddr>().is_none() {
        // Ensure ConnectInfo is available even when the handler did not set it.
        request.extensions_mut().insert(addr);
    }

    let ip = login_client_ip(&request, &state.config.proxy.trusted_proxies);

    let limiter = state
        .login_rate_limiters
        .entry(ip.clone())
        .or_insert_with(|| {
            let limit = std::num::NonZeroU32::new(MAX_LOGIN_ATTEMPTS_PER_MINUTE)
                .expect("non-zero");
            Arc::new(governor::RateLimiter::direct(governor::Quota::per_minute(limit)))
        })
        .clone();

    if limiter.check().is_err() {
        tracing::warn!(ip = %ip, "Login rate limit exceeded");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "60")],
            axum::Json(json!({
                "code": 429,
                "message": "Too many login attempts. Please try again later.",
                "data": null,
            })),
        )
            .into_response();
    }

    next.run(request).await
}
