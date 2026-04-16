use std::collections::HashMap;
use serde_json::Value;

/// Adapt a request body by mapping model names according to `model_map`.
///
/// For non-streaming requests the `model` field is rewritten in-place.
/// For streaming requests the `model` field inside `stream_options` is also updated
/// so the upstream provider returns usage info.
pub fn adapt_request(body: &Value, model_map: &HashMap<String, String>) -> Value {
    let mut adapted = body.clone();

    if let Some(model) = adapted.get("model").and_then(|v| v.as_str()) {
        if let Some(target) = model_map.get(model) {
            adapted["model"] = Value::String(target.clone());
        }
    }

    adapted
}

/// Extract token usage from an OpenAI-compatible response.
/// Returns (prompt_tokens, completion_tokens, total_tokens).
pub fn extract_tokens(response_body: &str) -> (i32, i32, i32) {
    let parsed: Value = match serde_json::from_str(response_body) {
        Ok(v) => v,
        Err(_) => return (0, 0, 0),
    };

    if let Some(usage) = parsed.get("usage") {
        let prompt = usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let completion = usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let total = usage
            .get("total_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| (prompt as i64) + (completion as i64)) as i32;
        return (prompt, completion, total);
    }

    // Streaming: check last chunk for usage
    (0, 0, 0)
}

/// Parse an SSE stream chunk, returning the JSON payload if valid.
pub fn parse_stream_chunk(chunk: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(chunk).ok()?;
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                return None;
            }
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                return Some(json);
            }
        }
    }
    None
}

/// Map a proxy model name to the real model name.
pub fn map_model_name(proxy_name: &str, model_map: &std::collections::HashMap<String, String>) -> String {
    model_map.get(proxy_name).cloned().unwrap_or_else(|| proxy_name.to_string())
}
