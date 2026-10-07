//! Link and verification-code extraction.

use regex::Regex;
use std::sync::LazyLock;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s"'<>)\]}>,]+"#).unwrap());

static CODE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{4,10}\b").unwrap());

fn dedup(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

/// Extract all http(s) URLs from the plain-text and HTML bodies.
pub fn extract_links(text: Option<&str>, html: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for src in [text, html].into_iter().flatten() {
        for m in URL_RE.find_iter(src) {
            // Trim trailing punctuation that sentence formatting glues on.
            let mut url = m.as_str();
            while url.ends_with(['.', ',', ';', ':', '!', '?']) {
                url = &url[..url.len() - 1];
            }
            out.push(url.to_string());
        }
    }
    dedup(out)
}

/// Extract likely verification/OTP codes: standalone 4–10 digit numbers.
/// Text body first (most reliable); falls back to the HTML body.
pub fn extract_codes(text: Option<&str>, html: Option<&str>) -> Vec<String> {
    let from_text: Vec<String> = text
        .map(|t| {
            CODE_RE
                .find_iter(t)
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default();
    if !from_text.is_empty() {
        return dedup(from_text);
    }
    let from_html: Vec<String> = html
        .map(|h| {
            CODE_RE
                .find_iter(h)
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default();
    dedup(from_html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_extracted_and_trimmed() {
        let html = r#"<a href="https://x.io/verify?t=abc">click.</a> See https://x.io/docs."#;
        let links = extract_links(None, Some(html));
        assert!(links.contains(&"https://x.io/verify?t=abc".to_string()));
        assert!(links.contains(&"https://x.io/docs".to_string()));
        assert!(!links.iter().any(|l| l.ends_with('.')));
    }

    #[test]
    fn codes_prefer_text_body() {
        let codes = extract_codes(Some("Your code is 424242."), Some("<b>11112222</b>"));
        assert_eq!(codes, vec!["424242".to_string()]);
    }
}
