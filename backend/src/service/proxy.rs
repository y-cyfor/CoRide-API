use std::collections::HashMap;
use std::time::Instant;

use axum::body::Body;
use axum::response::Response;
use serde::Serialize;
use serde_json::Value;

use crate::db::models::{AppProfile, Channel};

#[derive(Debug, Clone, Serialize)]
pub struct ProxyResult {
    pub status_code: u16,
    pub body: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub elapsed_ms: i32,
    pub is_stream: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("No API key available for channel")]
    NoApiKey,
    #[error("Upstream timeout after {0}ms")]
    Timeout(u32),
    #[error("Upstream error: {0}")]
    Upstream(String),
    #[error("Invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Select an API key from channel using round-robin based on quota_used.
fn select_api_key(channel: &Channel) -> Option<String> {
    let keys: Vec<String> = serde_json::from_str(channel.api_keys.as_str()).ok()?;
    if keys.is_empty() {
        return None;
    }
    let idx = (channel.quota_used as usize) % keys.len();
    Some(keys[idx].clone())
}

/// Build request headers from app profile and custom headers.
fn build_headers(
    app_profile: Option<&AppProfile>,
    channel: &Channel,
    api_key: &str,
    channel_type: &str,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    // Authorization
    if channel_type == "anthropic" {
        headers.insert("x-api-key".to_string(), api_key.to_string());
        headers.insert("anthropic-beta".to_string(), "prompt-caching-2024-07-31".to_string());
    } else {
        headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
    }

    // Content type
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    // App profile headers
    if let Some(profile) = app_profile {
        headers.insert("User-Agent".to_string(), profile.user_agent.clone());
        if let Some(ref extra) = profile.extra_headers {
            if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(extra) {
                // Merge anthropic-beta values instead of overwriting
                if channel_type == "anthropic" {
                    merge_anthropic_beta(&mut headers, &map);
                } else {
                    headers.extend(map);
                }
            }
        }
    }

    // Channel custom headers
    if let Some(ref custom) = channel.custom_headers {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(custom) {
            // Merge anthropic-beta values instead of overwriting
            if channel_type == "anthropic" {
                merge_anthropic_beta(&mut headers, &map);
            } else {
                headers.extend(map);
            }
        }
    }

    headers
}

/// Merge anthropic-beta header values, avoiding duplicates.
fn merge_anthropic_beta(headers: &mut HashMap<String, String>, extra: &HashMap<String, String>) {
    for (key, value) in extra {
        if key == "anthropic-beta" {
            let existing = headers.entry(key.clone()).or_default();
            // Comma-separated merge, avoid duplicates
            let mut parts: Vec<&str> = existing.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            for part in value.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() && !parts.contains(&trimmed) {
                    parts.push(trimmed);
                }
            }
            *existing = parts.join(", ");
        } else {
            headers.insert(key.clone(), value.clone());
        }
    }
}

/// Proxy a request to the upstream channel (non-streaming mode).
/// Reads the full response body and returns it as a string.
pub async fn proxy_request(
    state: &crate::AppState,
    channel: &Channel,
    method: &str,
    path: &str,
    body: &str,
    app_profile: Option<&AppProfile>,
    log_request_body: bool,
    log_response_body: bool,
) -> Result<ProxyResult, ProxyError> {
    let start = Instant::now();

    let api_key = select_api_key(channel).ok_or(ProxyError::NoApiKey)?;
    let url = format!("{}{}", channel.base_url.trim_end_matches('/'), path);
    let headers = build_headers(app_profile, channel, &api_key, &channel.r#type);

    let body_value: Value = serde_json::from_str(body)?;
    let is_stream = body_value.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut req_builder = state.http_client.request(
        reqwest::Method::try_from(method).unwrap_or(reqwest::Method::POST),
        &url,
    );

    for (key, value) in &headers {
        req_builder = req_builder.header(key, value);
    }

    req_builder = req_builder.json(&body_value);

    let resp = req_builder.send().await?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().await?;
    let elapsed = start.elapsed().as_millis() as i32;

    // Parse tokens from response
    let (prompt_tokens, completion_tokens, total_tokens) = if channel.r#type == "anthropic" {
        crate::service::anthropic::extract_tokens(&resp_body)
    } else {
        crate::service::openai::extract_tokens(&resp_body)
    };

    // Truncate body for logging
    let request_body: Option<String> = if log_request_body {
        Some(body.chars().take(2000).collect())
    } else {
        None
    };
    let response_body: Option<String> = if log_response_body {
        Some(resp_body.chars().take(2000).collect())
    } else {
        None
    };

    let _ = (request_body, response_body); // Used by caller for logging

    Ok(ProxyResult {
        status_code: status,
        body: resp_body,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        elapsed_ms: elapsed,
        is_stream,
    })
}

/// Proxy a request with streaming response passthrough.
/// Returns an axum Response that streams the upstream SSE data directly to the client.
pub async fn proxy_request_stream(
    state: &crate::AppState,
    channel: &Channel,
    method: &str,
    path: &str,
    body: &str,
    app_profile: Option<&AppProfile>,
) -> Result<Response, ProxyError> {
    let api_key = select_api_key(channel).ok_or(ProxyError::NoApiKey)?;
    let url = format!("{}{}", channel.base_url.trim_end_matches('/'), path);
    let headers = build_headers(app_profile, channel, &api_key, &channel.r#type);

    let body_value: Value = serde_json::from_str(body)?;

    let mut req_builder = state.http_client.request(
        reqwest::Method::try_from(method).unwrap_or(reqwest::Method::POST),
        &url,
    );

    for (key, value) in &headers {
        req_builder = req_builder.header(key, value);
    }

    req_builder = req_builder.json(&body_value);

    let resp = req_builder.send().await?;
    let status = resp.status();

    // Forward relevant response headers
    let mut response_builder = Response::builder().status(status);
    for (name, value) in resp.headers() {
        if name == "content-type"
            || name == "cache-control"
            || name == "connection"
            || name == "x-request-id"
        {
            if let Ok(v) = value.to_str() {
                response_builder = response_builder.header(name, v);
            }
        }
    }

    // Convert reqwest byte stream to axum Body
    let stream = resp.bytes_stream();
    let body = Body::from_stream(stream);

    Ok(response_builder.body(body).unwrap())
}
