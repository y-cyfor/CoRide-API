use serde_json::Value;

/// Extract token usage from an Anthropic-compatible response payload.
///
/// Cache counters are folded into the prompt count so quota accounting reflects the
/// full input the upstream billed for.
pub fn extract_tokens_value(parsed: &Value) -> (i32, i32, i32) {
    let Some(usage) = parsed.get("usage").filter(|u| !u.is_null()) else {
        return (0, 0, 0);
    };

    let input = i32_field(usage, "input_tokens");
    let cache_read = i32_field(usage, "cache_read_input_tokens");
    let cache_write = i32_field(usage, "cache_creation_input_tokens");
    let output = i32_field(usage, "output_tokens");

    let prompt = input + cache_read + cache_write;
    (prompt, output, prompt + output)
}

fn i32_field(value: &Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|v| v.clamp(0, i32::MAX as i64) as i32)
        .unwrap_or(0)
}

/// Extract token usage from an Anthropic-compatible response body.
pub fn extract_tokens(response_body: &str) -> (i32, i32, i32) {
    match serde_json::from_str::<Value>(response_body) {
        Ok(parsed) => extract_tokens_value(&parsed),
        Err(_) => (0, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_plain_usage() {
        let body = r#"{"usage":{"input_tokens":10,"output_tokens":5}}"#;
        assert_eq!(extract_tokens(body), (10, 5, 15));
    }

    #[test]
    fn folds_cache_counters_into_prompt() {
        let body = r#"{"usage":{"input_tokens":10,"cache_read_input_tokens":20,"cache_creation_input_tokens":5,"output_tokens":5}}"#;
        assert_eq!(extract_tokens(body), (35, 5, 40));
    }

    #[test]
    fn missing_usage_yields_zero() {
        assert_eq!(extract_tokens(r#"{"type":"message"}"#), (0, 0, 0));
        assert_eq!(extract_tokens("oops"), (0, 0, 0));
    }
}
