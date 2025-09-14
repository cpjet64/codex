use sha2::Digest;
use sha2::Sha256;

/// Normalize tool names and clamp length with hash suffix.
/// - lowercase
/// - allow [a-z0-9_-.]
/// - collapse runs of invalids into '-'
/// - clamp to 64 chars, keep 8-char hash tail when truncated
pub fn sanitize_tool_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.chars().map(|c| c.to_ascii_lowercase()) {
        let ok = ch.is_ascii_alphanumeric() || ch == '_' || ch == '.';
        if ok {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= 80 {
            break;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.len() <= 64 {
        return out;
    }
    let mut hasher = Sha256::new();
    hasher.update(out.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let suffix = &hash[0..8];
    let keep = 64_usize.saturating_sub(9);
    let head = &out[0..keep];
    format!("{head}-{suffix}")
}

#[cfg(test)]
mod tests {
    use super::sanitize_tool_name;
    use pretty_assertions::assert_eq;

    #[test]
    fn basic_normalization() {
        assert_eq!(sanitize_tool_name("Hello World!"), "hello-world");
        assert_eq!(sanitize_tool_name("A__B"), "a__b");
        assert_eq!(sanitize_tool_name("A.B"), "a.b");
    }

    #[test]
    fn clamped_with_hash() {
        let s = "x".repeat(200);
        let got = sanitize_tool_name(&s);
        assert!(got.len() <= 64);
        assert!(got.contains('-'));
    }
}

