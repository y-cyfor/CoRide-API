use serde_json::Value;

/// Extract token usage from an Anthropic-compatible response.
/// Returns (prompt_tokens, completion_tokens, total_tokens).
pub fn extract_tokens(response_body: &str) -> (i32, i32, i32) {
    let parsed: Value = match serde_json::from_str(response_body) {
        Ok(v) => v,
        Err(_) => return (0, 0, 0),
    };

    if let Some(usage) = parsed.get("usage") {
        let prompt = usage.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let completion = usage.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let total = prompt + completion;
        return (prompt, completion, total);
    }

    // For streaming responses, tokens are reported in the final delta
    (0, 0, 0)
}

/// Check if a response body indicates a streaming response.
pub fn is_streaming_response(response_body: &str) -> bool {
    let parsed: Value = match serde_json::from_str(response_body) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Check for streaming event types or stream field
    if let Some(event_type) = parsed.get("type").and_then(|v| v.as_str()) {
        return matches!(
            event_type,
            "message_start" | "content_block_start" | "content_block_delta"
            | "content_block_stop" | "message_delta" | "message_stop"
            | "ping"
        );
    }

    // Also check if stream field is true
    parsed.get("stream").and_then(|v| v.as_bool()) == Some(true)
}

/// Parse an Anthropic SSE stream chunk.
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
