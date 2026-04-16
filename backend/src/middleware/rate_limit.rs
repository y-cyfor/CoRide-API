use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use serde_json::json;
use thiserror::Error;

use crate::AppState;

/// Errors that can occur during rate limiting.
#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    TooManyRequests,
    #[error("Concurrency limit exceeded")]
    ConcurrencyExceeded,
}

impl RateLimitError {
    /// Convert the error into an HTTP response with the standard API format.
    pub fn into_response(&self) -> Response {
        let (status, message) = match self {
            RateLimitError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            RateLimitError::ConcurrencyExceeded => (StatusCode::TOO_MANY_REQUESTS, "Too many concurrent requests"),
        };
        let body = json!({
            "code": 429,
            "message": message,
            "data": null,
        });
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
}

/// Global rate limit middleware using governor DirectRateLimiter.
/// Checks QPS token bucket and concurrency limit before forwarding the request.
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let max_concurrent = state.config.global_rate_limit.concurrency;

    // Check QPS token bucket via governor
    if state.global_qps_limiter.check().is_err() {
        return RateLimitError::TooManyRequests.into_response();
    }

    // Check concurrency limit using CAS loop to avoid race condition
    if max_concurrent > 0 {
        loop {
            let current = ACTIVE_REQUESTS.load(std::sync::atomic::Ordering::Relaxed);
            if current >= max_concurrent {
                return RateLimitError::ConcurrencyExceeded.into_response();
            }
            // Try to increment atomically
            if ACTIVE_REQUESTS.compare_exchange_weak(
                current,
                current + 1,
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
            ).is_ok() {
                break;
            }
            // CAS failed, retry
        }
    }

    let response = next.run(request).await;

    // Decrement active request counter
    if max_concurrent > 0 {
        ACTIVE_REQUESTS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    response
}

/// Global counter for active concurrent requests.
static ACTIVE_REQUESTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// Use `axum::middleware::from_fn_with_state(state, rate_limit_middleware)` directly in router setup.
