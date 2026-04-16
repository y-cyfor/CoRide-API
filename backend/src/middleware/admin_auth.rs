use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use serde_json::json;

use crate::AppState;
use crate::utils::jwt;

/// Admin auth middleware.
/// Expects `Authorization: Bearer <jwt_token>`, validates the JWT,
/// and injects user claims into request extensions.
pub async fn admin_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Extract bearer token
    let auth_header = match request.headers().get(header::AUTHORIZATION) {
        Some(h) => match h.to_str() {
            Ok(s) => s,
            Err(_) => return auth_error_response("Invalid Authorization header"),
        },
        None => return auth_error_response("Missing Authorization header"),
    };

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return auth_error_response("Invalid Authorization header format"),
    };

    // Verify JWT
    let claims = match jwt::verify_token(token, &state.config.jwt.secret) {
        Ok(c) => c,
        Err(e) => {
            return auth_error_response(&format!("Invalid token: {}", e));
        }
    };

    // Check user is still active
    let status: Option<String> = match sqlx::query_scalar("SELECT status FROM users WHERE id = ?")
        .bind(claims.user_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return auth_error_response(&format!("Database error: {}", e));
        }
    };

    match status {
        Some(s) if s == "active" => {}
        Some(_) => return auth_error_response("User account is disabled"),
        None => return auth_error_response("User not found"),
    }

    // Inject claims into request extensions
    request.extensions_mut().insert(claims.user_id);
    request.extensions_mut().insert(claims.username);
    request.extensions_mut().insert(claims.role);

    next.run(request).await
}

fn auth_error_response(message: &str) -> Response {
    let body = json!({
        "code": 401,
        "message": message,
        "data": null,
    });
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}
