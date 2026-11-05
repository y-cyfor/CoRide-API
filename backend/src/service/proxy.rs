use std::collections::HashMap;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::db::models::{AppProfile, Channel, NewRequestLog};
use crate::service::quota;

/// Result of a non-streaming upstream call.
#[derive(Debug, Clone, Serialize)]
pub struct ProxyResult {
    pub status_code: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
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
    #[error("Upstream returned HTTP {status}: {body}")]
    UpstreamStatus { status: u16, body: String },
    #[error("Invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl ProxyError {
    /// HTTP status to surface when no upstream response could be used.
    pub fn status(&self) -> StatusCode {
        match self {
            ProxyError::UpstreamStatus { status, .. } => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            ProxyError::NoApiKey => StatusCode::BAD_GATEWAY,
            _ => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Pick the next API key for a channel using round-robin.
///
/// Only the *count* of calls matters, so a monotonically increasing cursor gives fair
/// distribution even when the channel has no quota configured.
pub fn select_api_key(channel: &Channel, cursor: u64) -> Option<String> {
    let keys: Vec<String> = serde_json::from_str(channel.api_keys.as_str()).ok()?;
    if keys.is_empty() {
        return None;
    }
    let idx = (cursor as usize) % keys.len();
    Some(keys[idx].clone())
}

/// Get or create an HTTP client for the given channel proxy URL.
fn get_channel_http_client(state: &crate::AppState, channel: &Channel) -> reqwest::Client {
    let proxy_key = channel.proxy_url.clone();

    if let Some(client) = state.http_clients.get(&proxy_key) {
        return client.clone();
    }

    let client = build_client_with_proxy(proxy_key.as_deref());
    state.http_clients.entry(proxy_key).or_insert(client.clone());
    client
}

/// Build a reqwest::Client with optional proxy configuration.
pub fn build_client_with_proxy(proxy_url: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90));
    if let Some(url) = proxy_url
        && let Ok(proxy) = reqwest::Proxy::all(url) {
            builder = builder.proxy(proxy);
        }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

/// Effective upstream timeout for a channel, clamped to a sane range.
pub fn channel_timeout(state: &crate::AppState, channel: &Channel) -> Duration {
    let configured = if channel.timeout > 0 {
        channel.timeout as u64
    } else {
        state.config.proxy.timeout
    };
    Duration::from_millis(configured.clamp(1_000, state.config.max_proxy_timeout_ms()))
}

/// Headers copied from the client request to the upstream call.
const FORWARDED_REQUEST_HEADERS: &[&str] = &[
    "anthropic-beta",
    "anthropic-version",
    "openai-beta",
    "openai-organization",
];

/// Response headers copied back to the client.
fn should_forward_response_header(name: &str) -> bool {
    matches!(
        name,
        "content-type"
            | "cache-control"
            | "x-request-id"
            | "request-id"
            | "openai-processing-ms"
            | "retry-after"
            | "anthropic-version"
    ) || name.starts_with("anthropic-ratelimit-")
        || name.starts_with("x-ratelimit-")
}

/// Build request headers from app profile, channel config and client headers.
pub fn build_headers(
    app_profile: Option<&AppProfile>,
    channel: &Channel,
    api_key: &str,
    client_headers: Option<&HeaderMap>,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let is_anthropic = channel.r#type == "anthropic";

    if is_anthropic {
        headers.insert("x-api-key".to_string(), api_key.to_string());
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers.insert(
            "anthropic-beta".to_string(),
            "prompt-caching-2024-07-31".to_string(),
        );
    } else {
        headers.insert("Authorization".to_string(), format!("Bearer {api_key}"));
    }

    headers.insert("Content-Type".to_string(), "application/json".to_string());

    // Client headers first so operator configuration can override them.
    if let Some(client_headers) = client_headers {
        for name in FORWARDED_REQUEST_HEADERS {
            if let Some(value) = client_headers.get(*name).and_then(|v| v.to_str().ok()) {
                if is_anthropic && *name == "anthropic-beta" {
                    merge_anthropic_beta(&mut headers, value);
                } else {
                    headers.insert((*name).to_string(), value.to_string());
                }
            }
        }
    }

    if let Some(profile) = app_profile {
        headers.insert("User-Agent".to_string(), profile.user_agent.clone());
        if let Some(ref extra) = profile.extra_headers
            && let Ok(map) = serde_json::from_str::<HashMap<String, String>>(extra) {
                if is_anthropic {
                    merge_extra_headers_anthropic(&mut headers, &map);
                } else {
                    headers.extend(map);
                }
            }
    }

    if let Some(ref custom) = channel.custom_headers
        && let Ok(map) = serde_json::from_str::<HashMap<String, String>>(custom) {
            if is_anthropic {
                merge_extra_headers_anthropic(&mut headers, &map);
            } else {
                headers.extend(map);
            }
        }

    headers
}

fn merge_extra_headers_anthropic(
    headers: &mut HashMap<String, String>,
    extra: &HashMap<String, String>,
) {
    for (key, value) in extra {
        if key.eq_ignore_ascii_case("anthropic-beta") {
            merge_anthropic_beta(headers, value);
        } else {
            headers.insert(key.clone(), value.clone());
        }
    }
}

/// Merge `anthropic-beta` values without dropping previously configured betas.
fn merge_anthropic_beta(headers: &mut HashMap<String, String>, value: &str) {
    let existing = headers.entry("anthropic-beta".to_string()).or_default();
    let mut parts: Vec<String> = existing
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    for part in value.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() && !parts.iter().any(|p| p == trimmed) {
            parts.push(trimmed.to_string());
        }
    }
    *existing = parts.join(", ");
}

/// Rewrite the `model` field to the upstream model name for a channel.
pub fn rewrite_model(body: &Value, source_name: &str) -> Value {
    let mut adapted = body.clone();
    if let Some(obj) = adapted.as_object_mut() {
        obj.insert("model".to_string(), Value::String(source_name.to_string()));
    }
    adapted
}

/// Ask the upstream to report token usage on streaming responses.
pub fn inject_stream_usage(body: &mut Value, is_anthropic: bool) {
    if is_anthropic {
        return; // Anthropic always reports usage in the stream.
    }
    if let Some(obj) = body.as_object_mut() {
        let already = obj
            .get("stream_options")
            .and_then(|v| v.get("include_usage"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !already {
            let options = obj
                .entry("stream_options")
                .or_insert_with(|| Value::Object(Default::default()));
            if let Some(options) = options.as_object_mut() {
                options.insert("include_usage".to_string(), Value::Bool(true));
            }
        }
    }
}

/// Extract token usage from a non-streaming response body.
pub fn extract_usage(body: &str, is_anthropic: bool) -> (i32, i32, i32) {
    let Ok(parsed) = serde_json::from_str::<Value>(body) else {
        return (0, 0, 0);
    };
    if is_anthropic {
        crate::service::anthropic::extract_tokens_value(&parsed)
    } else {
        crate::service::openai::extract_tokens_value(&parsed)
    }
}

fn apply_headers(
    mut builder: reqwest::RequestBuilder,
    headers: &HashMap<String, String>,
) -> reqwest::RequestBuilder {
    for (key, value) in headers {
        builder = builder.header(key.as_str(), value.as_str());
    }
    builder
}

/// Perform a non-streaming upstream request.
pub async fn proxy_request(
    state: &crate::AppState,
    channel: &Channel,
    method: &str,
    path: &str,
    body: &Value,
    app_profile: Option<&AppProfile>,
    client_headers: Option<&HeaderMap>,
) -> Result<ProxyResult, ProxyError> {
    let start = Instant::now();
    let cursor = state.next_key_cursor(channel.id);
    let api_key = select_api_key(channel, cursor).ok_or(ProxyError::NoApiKey)?;
    let url = format!("{}{}", channel.base_url.trim_end_matches('/'), path);
    let headers = build_headers(app_profile, channel, &api_key, client_headers);

    let http_client = get_channel_http_client(state, channel);
    let mut req_builder = http_client.request(
        reqwest::Method::try_from(method).unwrap_or(reqwest::Method::POST),
        &url,
    );
    req_builder = apply_headers(req_builder, &headers);
    req_builder = req_builder
        .timeout(channel_timeout(state, channel))
        .json(body);

    let resp = req_builder.send().await?;
    let status = resp.status();
    let response_headers = collect_response_headers(resp.headers());
    let resp_body = resp.text().await?;
    let elapsed = start.elapsed().as_millis() as i32;

    let (prompt_tokens, completion_tokens, total_tokens) =
        if status.is_success() {
            extract_usage(&resp_body, channel.r#type == "anthropic")
        } else {
            (0, 0, 0)
        };

    Ok(ProxyResult {
        status_code: status.as_u16(),
        body: resp_body,
        headers: response_headers,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        elapsed_ms: elapsed,
        is_stream: false,
    })
}

/// A streaming upstream call that already passed the status check.
pub struct StreamResponse {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub response: reqwest::Response,
}

/// Open a streaming upstream request.
///
/// Non-2xx responses are converted into [`ProxyError::UpstreamStatus`] so the caller can
/// try the next channel instead of streaming an error body to the client.
pub async fn open_stream(
    state: &crate::AppState,
    channel: &Channel,
    method: &str,
    path: &str,
    body: &Value,
    app_profile: Option<&AppProfile>,
    client_headers: Option<&HeaderMap>,
) -> Result<StreamResponse, ProxyError> {
    let cursor = state.next_key_cursor(channel.id);
    let api_key = select_api_key(channel, cursor).ok_or(ProxyError::NoApiKey)?;
    let url = format!("{}{}", channel.base_url.trim_end_matches('/'), path);
    let headers = build_headers(app_profile, channel, &api_key, client_headers);

    let http_client = get_channel_http_client(state, channel);
    let mut req_builder = http_client.request(
        reqwest::Method::try_from(method).unwrap_or(reqwest::Method::POST),
        &url,
    );
    req_builder = apply_headers(req_builder, &headers);
    // No total timeout for streams: long generations are legitimate. Liveness is
    // enforced by the per-chunk idle timeout in `stream_response`.
    req_builder = req_builder.json(body);

    let resp = req_builder.send().await?;
    let status = resp.status();
    let response_headers = collect_response_headers(resp.headers());

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ProxyError::UpstreamStatus {
            status: status.as_u16(),
            body: truncate_body(&body, 4_000),
        });
    }

    Ok(StreamResponse {
        status,
        headers: response_headers,
        response: resp,
    })
}

fn collect_response_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let name_str = name.as_str();
            if !should_forward_response_header(name_str) {
                return None;
            }
            value
                .to_str()
                .ok()
                .map(|value| (name_str.to_string(), value.to_string()))
        })
        .collect()
}

/// Truncate a string on a character boundary.
pub fn truncate_body(body: &str, max_chars: usize) -> String {
    if body.chars().count() <= max_chars {
        body.to_string()
    } else {
        let truncated: String = body.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}

/// Everything needed to account a streamed request once it finishes.
#[derive(Clone)]
pub struct StreamAccounting {
    pub pool: sqlx::SqlitePool,
    pub user_id: i64,
    pub key_id: Option<i64>,
    pub api_key: String,
    pub channel_id: i64,
    pub channel_quota_type: Option<String>,
    pub model: String,
    pub endpoint: String,
    pub is_anthropic: bool,
    pub started: Instant,
    pub request_body: Option<String>,
    pub log_response_body: bool,
    pub success_status: u16,
}

/// Incremental SSE usage parser.
///
/// Handles both OpenAI (`usage` object, usually in the final chunk when
/// `stream_options.include_usage` is set) and Anthropic (`message_start` carries input
/// tokens, `message_delta` carries output tokens, including cache counters).
#[derive(Debug, Default)]
pub struct StreamMeter {
    buffer: Vec<u8>,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    saw_usage: bool,
    capture: bool,
    response_preview: String,
}

impl StreamMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a raw chunk of the SSE stream.
    pub fn feed(&mut self, chunk: &[u8], is_anthropic: bool) {
        if self.capture && self.response_preview.chars().count() < 2_000
            && let Ok(text) = std::str::from_utf8(chunk) {
                let remaining = 2_000usize.saturating_sub(self.response_preview.chars().count());
                self.response_preview
                    .extend(text.chars().take(remaining));
            }

        self.buffer.extend_from_slice(chunk);

        // Process complete SSE events (terminated by a blank line).
        while let Some((event_len, terminator_len)) = find_event_boundary(&self.buffer) {
            let event: Vec<u8> = self.buffer.drain(..event_len).collect();
            self.buffer.drain(..terminator_len);
            self.parse_event(&event, is_anthropic);
        }
    }

    /// Start capturing a preview of the response body (for request logging).
    pub fn capture_preview(&mut self) {
        self.capture = true;
        self.response_preview.clear();
    }

    pub fn preview(&self) -> Option<&str> {
        if self.response_preview.is_empty() {
            None
        } else {
            Some(&self.response_preview)
        }
    }

    fn parse_event(&mut self, event: &[u8], is_anthropic: bool) {
        let Ok(text) = std::str::from_utf8(event) else {
            return;
        };
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            if is_anthropic {
                self.absorb_anthropic(&value);
            } else {
                self.absorb_openai(&value);
            }
        }
    }

    fn absorb_openai(&mut self, value: &Value) {
        if let Some(usage) = value.get("usage").filter(|u| !u.is_null()) {
            let prompt = json_i32(usage, "prompt_tokens");
            let completion = json_i32(usage, "completion_tokens");
            let total = usage
                .get("total_tokens")
                .and_then(Value::as_i64)
                .map(|v| v as i32)
                .unwrap_or(prompt + completion);
            self.record(prompt, completion, total);
        }
    }

    fn absorb_anthropic(&mut self, value: &Value) {
        match value.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(usage) = value.get("message").and_then(|m| m.get("usage")) {
                    let input = json_i32(usage, "input_tokens")
                        + json_i32(usage, "cache_creation_input_tokens")
                        + json_i32(usage, "cache_read_input_tokens");
                    if input > 0 {
                        self.prompt_tokens = input;
                        self.saw_usage = true;
                    }
                }
            }
            Some("message_delta") => {
                if let Some(usage) = value.get("usage") {
                    let output = json_i32(usage, "output_tokens");
                    if output > 0 {
                        self.completion_tokens = output;
                        self.saw_usage = true;
                    }
                }
            }
            _ => {}
        }
        if self.saw_usage {
            self.total_tokens = self.prompt_tokens + self.completion_tokens;
        }
    }

    fn record(&mut self, prompt: i32, completion: i32, total: i32) {
        if prompt > 0 || completion > 0 || total > 0 {
            self.prompt_tokens = prompt;
            self.completion_tokens = completion;
            self.total_tokens = total.max(prompt + completion);
            self.saw_usage = true;
        }
    }

    /// `(prompt, completion, total)` as observed so far.
    pub fn usage(&self) -> (i32, i32, i32) {
        (self.prompt_tokens, self.completion_tokens, self.total_tokens)
    }

    pub fn saw_usage(&self) -> bool {
        self.saw_usage
    }
}

fn json_i32(value: &Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|v| v.clamp(0, i32::MAX as i64) as i32)
        .unwrap_or(0)
}

/// Locate the end of the next complete SSE event.
///
/// Returns `(event_len, terminator_len)` where the terminator is `\n\n` or `\r\n\r\n`.
fn find_event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i + 1 < buffer.len() {
        if buffer[i] == b'\n' && buffer[i + 1] == b'\n' {
            return Some((i, 2));
        }
        if i + 3 < buffer.len()
            && buffer[i] == b'\r'
            && buffer[i + 1] == b'\n'
            && buffer[i + 2] == b'\r'
            && buffer[i + 3] == b'\n'
        {
            return Some((i, 4));
        }
        i += 1;
    }
    None
}

/// Build the client-facing response for a stream, accounting the request when it ends.
///
/// The returned body forwards bytes as they arrive; a background task parses usage,
/// deducts quota, writes the request log and releases the concurrency slot.
#[allow(clippy::too_many_arguments)]
pub fn stream_response(
    stream: StreamResponse,
    accounting: StreamAccounting,
    guards: Vec<crate::state::limiters::ConcurrencyGuard>,
    channel_permit: Option<tokio::sync::OwnedSemaphorePermit>,
    idle_timeout: Duration,
) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(16);
    let is_anthropic = accounting.is_anthropic;

    tokio::spawn(async move {
        // Keep the guards/permit alive until the stream finishes.
        let _guards = guards;
        let _permit = channel_permit;

        let mut meter = StreamMeter::new();
        if accounting.log_response_body {
            meter.capture_preview();
        }
        let mut upstream = stream.response;
        let mut stream_error: Option<String> = None;

        loop {
            match tokio::time::timeout(idle_timeout, upstream.chunk()).await {
                Ok(Ok(Some(bytes))) => {
                    meter.feed(&bytes, is_anthropic);
                    if tx.send(Ok(bytes)).await.is_err() {
                        // Client disconnected: stop forwarding, keep what we measured.
                        stream_error = Some("client disconnected".to_string());
                        break;
                    }
                }
                Ok(Ok(None)) => break,
                Ok(Err(e)) => {
                    stream_error = Some(format!("upstream stream error: {e}"));
                    let _ = tx
                        .send(Err(std::io::Error::other("upstream stream error")))
                        .await;
                    break;
                }
                Err(_) => {
                    stream_error = Some(format!(
                        "upstream stream stalled for more than {}s",
                        idle_timeout.as_secs()
                    ));
                    let _ = tx
                        .send(Err(std::io::Error::other("upstream stream timeout")))
                        .await;
                    break;
                }
            }
        }

        let (prompt, completion, total) = meter.usage();
        let elapsed_ms = accounting.started.elapsed().as_millis().min(i32::MAX as u128) as i32;
        let status_code = match &stream_error {
            Some(_) => 502,
            None => accounting.success_status as i32,
        };

        // Charge whatever was actually generated, even when the client disconnects
        // mid-stream: the upstream tokens were consumed either way.
        if total > 0 {
            let _ = quota::deduct_user_quota(
                &accounting.pool,
                accounting.user_id,
                accounting.channel_id,
                total as i64,
            )
            .await;
            let _ = quota::deduct_channel_quota(
                &accounting.pool,
                accounting.channel_id,
                accounting.channel_quota_type.as_deref(),
                total as i64,
            )
            .await;
        }

        let response_body = if accounting.log_response_body {
            meter.preview().map(|p| p.to_string())
        } else {
            None
        };

        let log = NewRequestLog {
            user_api_key: &accounting.api_key,
            user_id: Some(accounting.user_id),
            key_id: accounting.key_id,
            channel_id: Some(accounting.channel_id),
            model: &accounting.model,
            endpoint: &accounting.endpoint,
            status_code,
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
            elapsed_ms,
            request_body: accounting.request_body.as_deref(),
            response_body: response_body.as_deref(),
            error_message: stream_error.as_deref(),
        };
        if let Err(e) = crate::db::models::create_request_log(&accounting.pool, &log).await {
            tracing::warn!(error = %e, "Failed to write streamed request log");
        }
    });

    let body = Body::from_stream(ReceiverStream::new(rx));
    let mut builder = Response::builder().status(stream.status);
    for (name, value) in &stream.headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            builder = builder.header(name, value);
        }
    }
    builder
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Channel;

    fn channel(api_keys: &str, kind: &str) -> Channel {
        Channel {
            id: 1,
            name: "test".into(),
            r#type: kind.into(),
            base_url: "http://localhost".into(),
            api_keys: api_keys.into(),
            custom_headers: None,
            status: "active".into(),
            weight: 1,
            timeout: 300_000,
            retry_count: 1,
            quota_type: None,
            quota_limit: None,
            quota_used: 0,
            quota_cycle: None,
            quota_period_start: None,
            quota_period_end: None,
            app_profile_id: None,
            proxy_url: None,
            auto_disabled: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn api_key_selection_rotates() {
        let ch = channel(r#"["a","b","c"]"#, "openai");
        let picked: Vec<String> = (0..6)
            .map(|cursor| select_api_key(&ch, cursor).unwrap())
            .collect();
        assert_eq!(picked, vec!["a", "b", "c", "a", "b", "c"]);
    }

    #[test]
    fn api_key_selection_handles_empty_and_invalid() {
        assert!(select_api_key(&channel("[]", "openai"), 0).is_none());
        assert!(select_api_key(&channel("not-json", "openai"), 0).is_none());
    }

    #[test]
    fn anthropic_headers_include_version_and_beta() {
        let ch = channel(r#"["k"]"#, "anthropic");
        let headers = build_headers(None, &ch, "k", None);
        assert_eq!(headers.get("x-api-key").map(String::as_str), Some("k"));
        assert_eq!(
            headers.get("anthropic-version").map(String::as_str),
            Some("2023-06-01")
        );
        assert!(headers.get("anthropic-beta").unwrap().contains("prompt-caching"));
        assert!(!headers.contains_key("Authorization"));
    }

    #[test]
    fn openai_headers_use_bearer() {
        let ch = channel(r#"["k"]"#, "openai");
        let headers = build_headers(None, &ch, "k", None);
        assert_eq!(
            headers.get("Authorization").map(String::as_str),
            Some("Bearer k")
        );
        assert!(!headers.contains_key("x-api-key"));
    }

    #[test]
    fn anthropic_beta_values_merge() {
        let mut ch = channel(r#"["k"]"#, "anthropic");
        ch.custom_headers = Some(r#"{"anthropic-beta":"message-batches-2024-09-24"}"#.into());
        let headers = build_headers(None, &ch, "k", None);
        let beta = headers.get("anthropic-beta").unwrap();
        assert!(beta.contains("prompt-caching-2024-07-31"));
        assert!(beta.contains("message-batches-2024-09-24"));
    }

    #[test]
    fn model_rewrite_replaces_only_model() {
        let body = serde_json::json!({"model":"alias","messages":[],"stream":true});
        let rewritten = rewrite_model(&body, "real-model");
        assert_eq!(rewritten["model"], "real-model");
        assert_eq!(rewritten["stream"], true);
    }

    #[test]
    fn stream_usage_injection_is_idempotent() {
        let mut body = serde_json::json!({"stream": true});
        inject_stream_usage(&mut body, false);
        assert_eq!(body["stream_options"]["include_usage"], true);
        inject_stream_usage(&mut body, false);
        assert_eq!(body["stream_options"]["include_usage"], true);

        let mut anthropic = serde_json::json!({"stream": true});
        inject_stream_usage(&mut anthropic, true);
        assert!(anthropic.get("stream_options").is_none());
    }

    #[test]
    fn meter_reads_openai_usage() {
        let mut meter = StreamMeter::new();
        meter.feed(
            b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
            false,
        );
        assert_eq!(meter.usage(), (0, 0, 0));
        meter.feed(
            b"data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":4,\"total_tokens\":14}}\n\n",
            false,
        );
        assert_eq!(meter.usage(), (10, 4, 14));
        assert!(meter.saw_usage());
    }

    #[test]
    fn meter_reads_anthropic_usage_including_cache() {
        let mut meter = StreamMeter::new();
        meter.feed(
            b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":100,\"cache_read_input_tokens\":50,\"cache_creation_input_tokens\":25}}}\n\n",
            true,
        );
        meter.feed(
            b"event: message_delta\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":7}}\n\n",
            true,
        );
        assert_eq!(meter.usage(), (175, 7, 182));
    }

    #[test]
    fn meter_handles_crlf_events() {
        let mut meter = StreamMeter::new();
        meter.feed(
            b"data: {\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\r\n\r\n",
            false,
        );
        assert_eq!(meter.usage(), (1, 2, 3));
    }

    #[test]
    fn meter_tolerates_split_events() {
        let mut meter = StreamMeter::new();
        meter.feed(b"data: {\"usage\":{\"prompt_tok", false);
        assert_eq!(meter.usage(), (0, 0, 0));
        meter.feed(b"ens\":5,\"completion_tokens\":6}}\n\n", false);
        assert_eq!(meter.usage(), (5, 6, 11));
    }

    #[test]
    fn truncate_body_is_char_safe() {
        let text = "中文内容".repeat(100);
        let truncated = truncate_body(&text, 10);
        assert_eq!(truncated.chars().count(), 11); // 10 chars + ellipsis
    }
}
