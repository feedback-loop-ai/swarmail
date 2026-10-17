//! Link and verification-code extraction.
//!
//! Extraction happens once, at ingest (decision 0005), over the two bodies
//! the sender actually sent (decision 0031 §1): `extract_links` and
//! `extract_codes` never synthesize a body and never invent a value the
//! source does not contain. Everything here is total and linear — a hostile
//! body is a bad body, not a panic or a scan that never ends (bounded by
//! the 0022 body cap, applied before the parse).

use mail_parser::decoders::html::html_to_text;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s"'<>)\]}>,]+"#).unwrap());

static CODE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{4,10}\b").unwrap());

/// First-occurrence document order, preserved: `sort`+`dedup` rewrote the
/// order the document itself had (issue #39), and a client showing "the
/// links in this mail" deserves the order the author wrote them in.
fn dedup(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::with_capacity(values.len());
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

/// Decode XML character references inside one matched URL (issue #39): the
/// five XML named entities (case-sensitive, as XML defines them) plus
/// decimal and hex numerics. Total and linear — an unknown name, a
/// malformed form or a numeric outside the Unicode scalar range (or a
/// surrogate) stays literal — and it never grows the input: every decoded
/// reference is at least four bytes yielding one character.
///
/// Decoding runs on the *match*, not the whole HTML source: a source-wide
/// decode would rewrite `&lt;` into `<` and corrupt the very boundaries
/// `URL_RE` matched on. `&#0;` and other control characters are honest
/// decodes of what the sender wrote — the store keeps them; the display
/// plane owns their presentation.
fn decode_refs(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        match parse_ref(rest) {
            Some((decoded, len)) => {
                out.push(decoded);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Test-only scan counters (issue #102): the ampersand test observes the
/// reference scan's shape — one call per '&' and at most the window's
/// bytes per call. `#[cfg(test)]`, so production carries no counter
/// traffic.
#[cfg(test)]
static REF_SCANS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
static REF_SCAN_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// One character reference at the start of `s`: `(decoded char, bytes
/// consumed)`, or `None` when `s` does not begin with a well-formed
/// reference — which is the signal to copy the `&` through literally.
fn parse_ref(s: &str) -> Option<(char, usize)> {
    // The reference is bounded: the five XML names and a numeric's digits
    // never approach forty bytes, so a terminator that is not found within
    // the window means the '&' is literal — and the scan stays total and
    // linear. A window-less `find(';')` re-scans to the end of the URL
    // match for every '&': a hostile `Content-Type: text/html` body of one
    // 50 MiB '&' run pins a blocking worker for hours, and every store
    // mutation shares that pool (the review's security HOLD).
    const REF_MAX: usize = 40;
    #[cfg(test)]
    REF_SCANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let window = match s[1..].char_indices().nth(REF_MAX) {
        Some((byte_index, _)) => &s[1..1 + byte_index],
        None => &s[1..],
    };
    #[cfg(test)]
    REF_SCAN_BYTES.fetch_add(window.len() as u64, std::sync::atomic::Ordering::Relaxed);
    let token_end = window.find(';')? + 1;
    let token = &s[1..token_end];
    let decoded = if let Some(hex) = token
        .strip_prefix("#x")
        .or_else(|| token.strip_prefix("#X"))
    {
        u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)?
    } else if let Some(decimal) = token.strip_prefix('#') {
        decimal.parse::<u32>().ok().and_then(char::from_u32)?
    } else {
        match token {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => return None,
        }
    };
    Some((decoded, token_end + 1))
}

/// Extract all http(s) URLs from the plain-text and HTML bodies. On the
/// HTML leg a match may still carry character references — `&amp;` in an
/// `href` is the sender's `&` — so the match is entity-decoded before the
/// trailing-punctuation trim (issue #39). The plain-text leg has no
/// entities to decode: `&amp;` in plain text is literally `&amp;`.
pub fn extract_links(text: Option<&str>, html: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for (is_html, src) in [(false, text), (true, html)] {
        let Some(src) = src else { continue };
        for m in URL_RE.find_iter(src) {
            let decoded = if is_html {
                decode_refs(m.as_str())
            } else {
                m.as_str().to_string()
            };
            // Trim trailing punctuation that sentence formatting glues on.
            let mut url = decoded.as_str();
            while url.ends_with(['.', ',', ';', ':', '!', '?']) {
                url = &url[..url.len() - 1];
            }
            out.push(url.to_string());
        }
    }
    dedup(out)
}

/// Extract likely verification/OTP codes: standalone 4–10 digit numbers.
/// Text body first (most reliable); falls back to the HTML body — rendered
/// to text first (decision 0031 §3), so codes come from rendered text
/// nodes only. The renderer is narrow in one recorded way: inline elements
/// carry no separators of their own (`<a>go</a> 555555` renders
/// "go555555"), so a code the renderer glues into a word is refused rather
/// than invented — a real verification mail keeps the code in its own text
/// or a block element. Digits inside `<style>` blocks, `<script>`s, comments or
/// attribute values yield nothing: a stylesheet color is not a code, and
/// an attribute is not displayed to the person the code is for.
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
            let rendered = html_to_text(h);
            CODE_RE
                .find_iter(&rendered)
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default();
    dedup(from_html)
}

#[cfg(test)]
mod tests {
    // The html leg is proven against realistic mail, not fixtures shaped
    // to flatter the renderer (the review's tests MEDIUM). Inline elements
    // are recorded narrowings: `<b>`/`<span>` render with their source
    // whitespace and the code stays a standalone token, but mail_parser's
    // html_to_text glues the text after an `</a>` close — `<a>go</a>
    // 555555` renders "go555555" — and a code the renderer glues into a
    // word is honestly refused: not a standalone token. The plain-text
    // leg remains the reliable leg for codes.
    #[test]
    fn inline_adjacent_codes_behave_as_the_renderer_renders() {
        assert_eq!(
            extract_codes(None, Some("See <b>555555</b> soon")),
            vec!["555555"]
        );
        assert_eq!(
            extract_codes(None, Some("<p>hello</p><span>555555</span>")),
            vec!["555555"]
        );
        assert!(extract_codes(None, Some("<a href=\"https://x.io\">go</a> 555555")).is_empty());
        assert!(extract_codes(None, Some("x<i>555555</i>y")).is_empty());
    }

    // The reference scan is bounded (the review's security HOLD): a match
    // of nothing but '&'s — no ';' anywhere — decodes in bounded work per
    // position, each '&' passing through literal, instead of rescanning
    // to the match's end for every one of them. The scan counters make the
    // bound observable: one call per '&' and at most a window's bytes per
    // call. Remove the 40-character window and the same input rescans ~4
    // orders of magnitude more bytes and this assert goes red. The slack
    // covers parallel inline tests' parse_ref calls landing inside this
    // delta (issue #102).
    #[test]
    fn an_ampersand_run_decodes_in_bounded_work() {
        // Mirrors `parse_ref`'s REF_MAX (40 chars): the per-call scan
        // ceiling. Keep the two in step.
        const REF_WINDOW_MAX: u64 = 40;
        const REF_SLACK: u64 = REF_WINDOW_MAX * 1024;
        let src = format!("https://x.io/{}", "&".repeat(500_000));
        let calls0 = REF_SCANS.load(std::sync::atomic::Ordering::Relaxed);
        let bytes0 = REF_SCAN_BYTES.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(extract_links(None, Some(&src)), vec![src.clone()]);
        let calls = REF_SCANS.load(std::sync::atomic::Ordering::Relaxed) - calls0;
        let bytes = REF_SCAN_BYTES.load(std::sync::atomic::Ordering::Relaxed) - bytes0;
        assert!(calls >= 500_000, "one call per '&': {calls}");
        assert!(
            bytes <= calls * REF_WINDOW_MAX + REF_SLACK,
            "the reference scan is unbounded: {calls} calls scanned {bytes} bytes"
        );
    }

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

    // ---- issue #39: entity decoding on the html leg ----

    #[test]
    fn named_refs_decode_per_url() {
        // `&amp;` is the href's own `&`; the text leg keeps it literal.
        let html = r#"<a href="https://x.io/a?b=1&amp;c=2">go</a>"#;
        assert_eq!(
            extract_links(None, Some(html)),
            vec!["https://x.io/a?b=1&c=2".to_string()]
        );
        let plain = "see https://x.io/a?b=1&amp;c=2 today";
        assert_eq!(
            extract_links(Some(plain), None),
            vec!["https://x.io/a?b=1&amp;c=2".to_string()],
            "plain text has no entities to decode"
        );
    }

    #[test]
    fn numeric_refs_decode_and_unknown_names_stay_literal() {
        let html = "<a href=\"https://x.io/a&#38;b&#x43;d\">go</a> \
                    <a href=\"https://x.io/e&nosuch;f&#xZZ;/\">go</a> \
                    <a href=\"https://x.io/&#55296;surrogate&#1114112;/\">go</a>";
        let links = extract_links(None, Some(html));
        assert!(
            links.contains(&"https://x.io/a&bCd".to_string()),
            "{links:?}"
        );
        // Unknown name, malformed hex and out-of-range numerics pass through.
        assert!(
            links
                .iter()
                .any(|l| l.contains("&nosuch;") && l.contains("&#xZZ;")),
            "malformed refs stay literal: {links:?}"
        );
        assert!(
            links
                .iter()
                .any(|l| l.contains("&#55296;") && l.contains("&#1114112;")),
            "surrogate and > U+10FFFF stay literal: {links:?}"
        );
    }

    #[test]
    fn decode_refs_is_total_and_never_grows() {
        for input in [
            "&",
            "&amp",
            "&amp;",
            "&#;",
            "&#x;",
            "&AMP;",
            "&Lt;",
            "https://x.io/a&amp;b",
            "&&&&amp;;",
            "&notanentity; tail",
            "trailing &",
        ] {
            let decoded = decode_refs(input);
            assert!(decoded.len() <= input.len(), "{input} grew: {decoded}");
            assert!(decode_refs(&decoded).len() <= decoded.len());
        }
        assert_eq!(decode_refs("https://x.io/a&amp;b"), "https://x.io/a&b");
        assert_eq!(decode_refs("&"), "&");
        assert_eq!(decode_refs("&amp"), "&amp");
        assert_eq!(
            decode_refs("&AMP;"),
            "&AMP;",
            "named refs are case-sensitive"
        );
        assert_eq!(decode_refs("&quot;&apos;&lt;&gt;"), "\"'<>");
        assert_eq!(decode_refs("&#65;&#x42;&#X43;"), "ABC");
    }

    // ---- issue #39: first-occurrence document order ----

    #[test]
    fn links_keep_document_order() {
        let text = "z https://z.io last, https://a.io first https://z.io again";
        assert_eq!(
            extract_links(Some(text), None),
            vec!["https://z.io".to_string(), "https://a.io".to_string(),]
        );
    }

    #[test]
    fn codes_keep_document_order() {
        let codes = extract_codes(Some("9999 then 1111 then 9999"), None);
        assert_eq!(codes, vec!["9999".to_string(), "1111".to_string()]);
    }

    // ---- issue #39: codes come from rendered text nodes only ----

    #[test]
    fn codes_ignore_style_script_and_attributes() {
        let html = "<style>p { color: #333333; }</style>\
                    <script>var token = 44445555;</script>\
                    <!-- 66667777 -->\
                    <a href=\"/reset?token=88889999\">reset</a>\
                    <p>Your code is 12345678.</p>";
        assert_eq!(
            extract_codes(None, Some(html)),
            vec!["12345678".to_string()]
        );
    }

    #[test]
    fn codes_render_from_visible_html_text() {
        let html = "<p>Use 77778888 to sign in.</p>";
        assert_eq!(
            extract_codes(None, Some(html)),
            vec!["77778888".to_string()]
        );
    }
}
