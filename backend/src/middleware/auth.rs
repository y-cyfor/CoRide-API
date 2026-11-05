use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::db::models;
use crate::AppState;

/// Everything the proxy needs to know about the caller after authentication.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: i64,
    pub role: String,
    /// Raw API key used for this request (stored on request logs).
    pub api_key: String,
    /// `user_keys.id` when the key came from the key table.
    pub key_id: Option<i64>,
    /// Model allow-list; `None` means "all models".
    pub allowed_models: Option<Vec<String>>,
}

impl AuthContext {
    /// Check the model allow-list.
    ///
    /// An empty list or a `*` entry allows everything, matching the documented
    /// "empty = unrestricted" behaviour of `enabled_models`.
    pub fn allows_model(&self, model: &str) -> bool {
        match &self.allowed_models {
            None => true,
            Some(list) => {
                list.iter().any(|allowed| {
                    allowed == "*" || allowed == model
                })
            }
        }
    }
}

/// Extract an API key from `Authorization: Bearer` or `x-api-key`.
///
/// Anthropic SDKs send `x-api-key`, OpenAI SDKs send the bearer token; both are accepted
/// so that clients can point at the proxy without changing their configuration.
fn extract_api_key(request: &Request<Body>) -> Option<String> {
    if let Some(value) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        && let Some(token) = value.strip_prefix("Bearer ") {
            let token = token.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }

    if let Some(value) = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
    {
        let value = value.trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }

    None
}

/// Auth middleware for proxy routes.
///
/// Resolves the API key to a user, rejects revoked keys and injects [`AuthContext`].
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let api_key = match extract_api_key(&request) {
        Some(key) => key,
        None => {
            return api_error(
                StatusCode::UNAUTHORIZED,
                "missing_api_key",
                "Provide an API key via 'Authorization: Bearer <key>' or 'x-api-key: <key>'",
            )
        }
    };

    let identity = match models::resolve_api_key(&state.db, &api_key).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            return api_error(
                StatusCode::UNAUTHORIZED,
                "invalid_api_key",
                "Invalid API key",
            )
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during API key authentication");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "database_unavailable",
                "Authentication is temporarily unavailable",
            );
        }
    };

    if let Some(status) = identity.key_status.as_deref()
        && status != "active" {
            return api_error(
                StatusCode::UNAUTHORIZED,
                "api_key_revoked",
                "This API key has been disabled",
            );
        }

    if identity.user_status != "active" {
        return api_error(
            StatusCode::FORBIDDEN,
            "user_disabled",
            "User account is disabled",
        );
    }

    let ctx = AuthContext {
        user_id: identity.user_id,
        role: identity.role.clone(),
        api_key: api_key.clone(),
        key_id: identity.key_id,
        allowed_models: identity.allowed_models,
    };

    request.extensions_mut().insert(ctx);
    // Kept for handlers that extract plain values.
    request.extensions_mut().insert(identity.user_id);
    request.extensions_mut().insert(identity.role);
    request.extensions_mut().insert(api_key);

    next.run(request).await
}

/// Build an OpenAI-compatible error response.
fn api_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = json!({
        "error": {
            "message": message,
            "type": "invalid_request_error",
            "code": code,
        }
    });
    (status, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(models: Option<Vec<&str>>) -> AuthContext {
        AuthContext {
            user_id: 1,
            role: "user".into(),
            api_key: "sk-test".into(),
            key_id: None,
            allowed_models: models.map(|list| list.into_iter().map(str::to_string).collect()),
        }
    }

    #[test]
    fn missing_list_allows_every_model() {
        assert!(ctx(None).allows_model("gpt-4o"));
    }

    #[test]
    fn list_restricts_models() {
        let ctx = ctx(Some(vec!["gpt-4o", "claude"]));
        assert!(ctx.allows_model("gpt-4o"));
        assert!(ctx.allows_model("claude"));
        assert!(!ctx.allows_model("gpt-4o-mini"));
    }

    #[test]
    fn wildcard_allows_everything() {
        assert!(ctx(Some(vec!["*"])).allows_model("anything"));
    }
}
