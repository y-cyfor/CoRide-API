use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::AppState;

/// Extract client IP from request headers (X-Real-IP > X-Forwarded-For > remote_addr).
fn extract_client_ip(req: &Request<Body>) -> String {
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.trim().to_string();
        }
    }
    if let Some(xff) = req.headers().get("X-Forwarded-For") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first_ip) = xff_str.split(',').next() {
                let trimmed = first_ip.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
    }
    req.extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// IP filter middleware: check global blacklist + user whitelist.
pub async fn ip_filter(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&req);

    // Check global blacklist
    match crate::db::models::is_ip_blacklisted(&state.db, &ip).await {
        Ok(true) => {
            tracing::warn!(ip = ip, "Blocked request: IP in global blacklist");
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to check IP blacklist, allowing request");
        }
    }

    // Check user whitelist (only if user_id is in extensions, set by auth middleware)
    if let Some(user_id) = req.extensions().get::<i64>().copied() {
        match crate::db::models::check_user_ip_whitelist(&state.db, user_id, &ip).await {
            Ok(Some(false)) => {
                tracing::warn!(ip = ip, user_id = user_id, "Blocked request: IP not in user whitelist");
                return Err(StatusCode::FORBIDDEN);
            }
            Ok(Some(true)) | Ok(None) => {}
            Err(e) => {
                tracing::error!(error = %e, "Failed to check IP whitelist, allowing request");
            }
        }
    }

    Ok(next.run(req).await)
}
