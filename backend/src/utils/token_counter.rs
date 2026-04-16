/// Estimate the number of tokens in a text string.
/// This is a rough heuristic: ~4 chars/token for English, ~1.5 chars/token for CJK.
pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }

    let mut cjk_count = 0u64;
    let mut other_count = 0u64;

    for ch in text.chars() {
        // CJK Unified Ideographs and common extensions
        if matches!(
            ch as u32,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF
                | 0x2A700..=0x2B73F | 0x2B740..=0x2B81F | 0x2B820..=0x2CEAF
                | 0xF900..=0xFAFF | 0x2F800..=0x2FA1F | 0x3000..=0x303F
                | 0x3040..=0x309F | 0x30A0..=0x30FF | 0x31F0..=0x31FF
                | 0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F
        ) {
            cjk_count += 1;
        } else {
            other_count += 1;
        }
    }

    // CJK chars: ~1.5 chars per token
    // Other chars: ~4 chars per token
    let cjk_tokens = (cjk_count as f64 / 1.5).ceil() as u64;
    let other_tokens = (other_count as f64 / 4.0).ceil() as u64;

    cjk_tokens + other_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_english_text() {
        // "Hello world" = 11 chars, ~3 tokens at 4 chars/token
        let tokens = estimate_tokens("Hello world");
        assert!(tokens >= 2 && tokens <= 5);
    }

    #[test]
    fn test_cjk_text() {
        // "你好世界" = 4 CJK chars, ~3 tokens at 1.5 chars/token
        let tokens = estimate_tokens("你好世界");
        assert!(tokens >= 2 && tokens <= 5);
    }
}
