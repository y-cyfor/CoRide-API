use serde_json::Value;

/// Extract token usage from an OpenAI-compatible response payload.
///
/// Supports both the classic `prompt_tokens`/`completion_tokens` fields and the newer
/// `input_tokens`/`output_tokens` naming used by some compatible providers.
pub fn extract_tokens_value(parsed: &Value) -> (i32, i32, i32) {
    let usage = match parsed.get("usage").filter(|u| !u.is_null()) {
        Some(usage) => usage,
        None => return (0, 0, 0),
    };

    let prompt = int_field(usage, &["prompt_tokens", "input_tokens"]);
    let completion = int_field(usage, &["completion_tokens", "output_tokens"]);
    let total = int_field(usage, &["total_tokens"]);

    let total = if total > 0 { total } else { prompt + completion };
    (prompt, completion, total)
}

fn int_field(value: &Value, keys: &[&str]) -> i32 {
    for key in keys {
        if let Some(v) = value.get(*key).and_then(Value::as_i64) {
            return v.clamp(0, i32::MAX as i64) as i32;
        }
    }
    0
}

/// Extract token usage from an OpenAI-compatible response body.
pub fn extract_tokens(response_body: &str) -> (i32, i32, i32) {
    match serde_json::from_str::<Value>(response_body) {
        Ok(parsed) => extract_tokens_value(&parsed),
        Err(_) => (0, 0, 0),
    }
}

/// Parse an SSE chunk into the first JSON payload it contains.
pub fn parse_stream_chunk(chunk: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(chunk).ok()?;
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                return None;
            }
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                return Some(json);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_classic_usage() {
        let body = r#"{"usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46}}"#;
        assert_eq!(extract_tokens(body), (12, 34, 46));
    }

    #[test]
    fn reads_alternative_usage_keys() {
        let body = r#"{"usage":{"input_tokens":7,"output_tokens":3}}"#;
        assert_eq!(extract_tokens(body), (7, 3, 10));
    }

    #[test]
    fn missing_usage_yields_zero() {
        assert_eq!(extract_tokens(r#"{"choices":[]}"#), (0, 0, 0));
        assert_eq!(extract_tokens("not json"), (0, 0, 0));
        assert_eq!(extract_tokens(r#"{"usage":null}"#), (0, 0, 0));
    }
}
