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
    let mut stats = RefScanStats::default();
    decode_refs_scanned(input, &mut stats)
}

/// The reference scan's shape for one `decode_refs` call (issue #102, the
/// review's MEDIUM): one scan per '&' and at most the window's byte length
/// per scan, observed through the value `decode_refs_scanned` fills — not
/// through process-global counters, which decision 0020 §1 forbids without
/// a seam. Production constructs the value and drops it: two integer adds
/// on the ingest path's existing parse, no shared mutable state. The
/// fields are read only by the bounded-work tests.
#[derive(Default)]
#[cfg_attr(not(test), allow(dead_code))]
struct RefScanStats {
    /// Well-formed or not, each '&' costs exactly one bounded scan.
    scans: u64,
    /// Bytes that scan actually examined: the window is up to `REF_MAX`
    /// *characters*, so up to 4 * `REF_MAX` bytes on a multibyte body.
    bytes: u64,
}

fn decode_refs_scanned(input: &str, stats: &mut RefScanStats) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        match parse_ref(rest, stats) {
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

#[cfg(test)]
fn decode_refs_with_stats(input: &str) -> (String, RefScanStats) {
    let mut stats = RefScanStats::default();
    let out = decode_refs_scanned(input, &mut stats);
    (out, stats)
}

/// One character reference at the start of `s`: `(decoded char, bytes
/// consumed)`, or `None` when `s` does not begin with a well-formed
/// reference — which is the signal to copy the `&` through literally.
fn parse_ref(s: &str, stats: &mut RefScanStats) -> Option<(char, usize)> {
    // The reference is bounded: the five XML names and a numeric's digits
    // never approach forty bytes, so a terminator that is not found within
    // the window means the '&' is literal — and the scan stays total and
    // linear. A window-less `find(';')` re-scans to the end of the URL
    // match for every '&': a hostile `Content-Type: text/html` body of one
    // 50 MiB '&' run pins a blocking worker for hours, and every store
    // mutation shares that pool (the review's security HOLD).
    const REF_MAX: usize = 40;
    stats.scans += 1;
    let window = match s[1..].char_indices().nth(REF_MAX) {
        Some((byte_index, _)) => &s[1..1 + byte_index],
        None => &s[1..],
    };
    stats.bytes += window.len() as u64;
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
    // to the match's end for every one of them. The bound is observed
    // through `decode_refs_with_stats`'s returned value — exact counts,
    // no slack, no process-global state (decision 0020 §1; issue #102):
    // 500_000 '&'s cost 500_000 scans; scan i (1-based) sees the
    // min(40, 500_000 - i) one-byte characters after it, so the total is
    // 499_960 * 40 + (39 + 38 + ... + 0) = 19_999_180 bytes. Remove the
    // 40-character window and the same input rescans ~4 orders of
    // magnitude more bytes and this assert goes red.
    #[test]
    fn an_ampersand_run_decodes_in_bounded_work() {
        let src = "&".repeat(500_000);
        let (out, stats) = decode_refs_with_stats(&src);
        assert_eq!(out, src);
        assert_eq!(stats.scans, 500_000, "one scan per '&'");
        assert_eq!(stats.bytes, 19_999_180, "each scan sees at most the window");
    }

    // The window bounds *characters*, not bytes: interleaving each '&'
    // with one two-byte character, scan i (1-based) has 1 + 2*(500_000-i)
    // characters after it (the remainder ends on the two-byte char), so a
    // full window is 40 characters = 20 two-byte + 20 one-byte = 60 bytes,
    // and the tail — j two-byte and j one-byte characters plus one
    // trailing two-byte — contributes 3j + 2 for j = 19..=0:
    // 499_980 * 60 + (3 * 190 + 40) = 29_999_410 — the exact observation
    // the slack-based counter probe could not make (issue #102).
    #[test]
    fn the_reference_window_counts_characters_not_bytes() {
        let src = "&\u{e9}".repeat(500_000);
        let (out, stats) = decode_refs_with_stats(&src);
        assert_eq!(out, src);
        assert_eq!(stats.scans, 500_000);
        assert_eq!(stats.bytes, 29_999_410);
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
