use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::AppState;

/// Resolve the client IP for a request.
///
/// `X-Real-IP` is only trusted when the direct connection comes from a configured
/// trusted proxy (e.g. the bundled nginx on 127.0.0.1); otherwise the socket address is
/// used, so a client cannot spoof its IP.
pub fn client_ip(req: &Request<Body>, trusted_proxies: &[String]) -> String {
    // `into_make_service_with_connect_info` inserts `ConnectInfo<SocketAddr>`;
    // plain `SocketAddr` is accepted too for tests and custom servers.
    let socket_addr = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0)
        .or_else(|| req.extensions().get::<std::net::SocketAddr>().copied());

    if let Some(addr) = socket_addr {
        let peer = addr.ip().to_string();
        if trusted_proxies.iter().any(|proxy| proxy == &peer)
            && let Some(ip) = req
                .headers()
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|ip| !ip.is_empty())
            {
                // Only accept a syntactically valid address from the proxy.
                if ip.parse::<std::net::IpAddr>().is_ok() {
                    return ip.to_string();
                }
            }
        return peer;
    }

    "unknown".to_string()
}

/// Global IP blacklist check. Runs before authentication so blocked clients are
/// rejected as early as possible.
pub async fn ip_blocklist(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = client_ip(&req, &state.config.proxy.trusted_proxies);

    match crate::db::models::is_ip_blacklisted(&state.db, &ip).await {
        Ok(true) => {
            tracing::warn!(ip = %ip, "Blocked request: IP in global blacklist");
            return forbidden(&ip);
        }
        Ok(false) => {}
        Err(e) => {
            // Fail open: a database hiccup must not take the whole service down.
            tracing::error!(error = %e, "Failed to check IP blacklist, allowing request");
        }
    }

    next.run(req).await
}

/// Per-user IP whitelist check. Runs after authentication so `AuthContext` is available.
pub async fn ip_whitelist(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let user_id = req
        .extensions()
        .get::<crate::middleware::auth::AuthContext>()
        .map(|ctx| ctx.user_id);

    let Some(user_id) = user_id else {
        // Unauthenticated request: nothing to check here.
        return next.run(req).await;
    };

    let ip = client_ip(&req, &state.config.proxy.trusted_proxies);

    match crate::db::models::check_user_ip_whitelist(&state.db, user_id, &ip).await {
        Ok(Some(false)) => {
            tracing::warn!(ip = %ip, user_id, "Blocked request: IP not in user whitelist");
            return forbidden(&ip);
        }
        Ok(Some(true)) | Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to check IP whitelist, allowing request");
        }
    }

    next.run(req).await
}

fn forbidden(ip: &str) -> Response {
    let body = json!({
        "error": {
            "message": "Access denied for this IP address",
            "type": "invalid_request_error",
            "code": "ip_forbidden",
            "ip": ip,
        }
    });
    (StatusCode::FORBIDDEN, axum::Json(body)).into_response()
}

/// Extract the client IP for the login rate limiter.
pub fn login_client_ip(
    req: &Request<Body>,
    trusted_proxies: &[String],
) -> String {
    let ip = client_ip(req, trusted_proxies);
    if ip.is_empty() {
        "unknown".to_string()
    } else {
        ip
    }
}

/// Helper used by tests: build a request with a socket address extension.
#[cfg(test)]
pub fn request_with_ip(ip: std::net::SocketAddr, real_ip: Option<&str>) -> Request<Body> {
    use axum::http::header;

    let mut builder = Request::builder().uri("/v1/chat/completions");
    if let Some(real_ip) = real_ip {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        builder = builder.header("X-Real-IP", real_ip);
    }
    let mut req = builder.body(Body::empty()).unwrap();
    req.extensions_mut().insert(ip);
    req
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    #[test]
    fn uses_socket_address_by_default() {
        let req = request_with_ip(addr(50000), None);
        assert_eq!(client_ip(&req, &[]), "127.0.0.1");
    }

    #[test]
    fn trusts_real_ip_from_trusted_proxy() {
        let req = request_with_ip(addr(50000), Some("203.0.113.9"));
        assert_eq!(
            client_ip(&req, &["127.0.0.1".to_string()]),
            "203.0.113.9"
        );
    }

    #[test]
    fn ignores_real_ip_from_untrusted_peer() {
        let req = request_with_ip(addr(50000), Some("203.0.113.9"));
        assert_eq!(client_ip(&req, &["10.0.0.1".to_string()]), "127.0.0.1");
    }

    #[test]
    fn rejects_malformed_real_ip() {
        let req = request_with_ip(addr(50000), Some("not-an-ip"));
        assert_eq!(
            client_ip(&req, &["127.0.0.1".to_string()]),
            "127.0.0.1"
        );
    }

    #[test]
    fn missing_socket_address_falls_back_to_unknown() {
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req, &[]), "unknown");
    }
}
