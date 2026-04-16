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

/// Errors that can occur during API key authentication.
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Missing or invalid Authorization header")]
    MissingHeader,
    #[error("Invalid API key")]
    InvalidKey,
    #[error("User account is disabled")]
    Disabled,
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl AuthError {
    /// Convert the error into an HTTP response with the standard API format.
    pub fn into_response(&self) -> Response {
        let (status, message): (StatusCode, &str) = match self {
            AuthError::MissingHeader => (StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header"),
            AuthError::InvalidKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            AuthError::Disabled => (StatusCode::FORBIDDEN, "User account is disabled"),
            AuthError::Db(e) => {
                let body = json!({
                    "code": 500,
                    "message": format!("Internal error: {e}"),
                    "data": null,
                });
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap();
            }
        };
        let body = json!({
            "code": status.as_u16(),
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

/// Extract the bearer token from the Authorization header.
fn extract_bearer_token(request: &Request<Body>) -> Option<String> {
    let auth_header = request.headers().get(header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;
    auth_str.strip_prefix("Bearer ").map(String::from)
}

/// Auth middleware for proxy routes.
/// Extracts API key from `Authorization: Bearer <key>`, validates the user
/// exists and is active, then injects `user_id` (i64) and `role` (String)
/// into request extensions.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Extract API key
    let api_key = match extract_bearer_token(&request) {
        Some(key) => key,
        None => return AuthError::MissingHeader.into_response(),
    };

    // Query user by API key — include status so we can distinguish
    // "key not found" (401) from "user disabled" (403).
    let result: Result<Option<(i64, String, String)>, _> = sqlx::query_as(
        "SELECT id, role, status FROM users WHERE api_key = ?",
    )
    .bind(&api_key)
    .fetch_optional(&state.db)
    .await;

    let row = match result {
        Ok(Some(row)) => row,
        Ok(None) => return AuthError::InvalidKey.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Database error during auth");
            return AuthError::Db(e).into_response();
        }
    };

    let (user_id, role, status) = row;

    if status != "active" {
        return AuthError::Disabled.into_response();
    }

    // Inject user_id, role, and api_key into request extensions
    request.extensions_mut().insert(user_id);
    request.extensions_mut().insert(role);
    request.extensions_mut().insert(api_key);

    next.run(request).await
}

// Use `axum::middleware::from_fn_with_state(state, auth_middleware)` directly in router setup.
