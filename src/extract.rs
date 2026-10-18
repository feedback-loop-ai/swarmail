//! Link and verification-code extraction.
//!
//! Extraction happens once, at ingest (decision 0005), over the two bodies
//! the sender actually sent (decision 0031 §1, 0040 §1): `extract_links`
//! and `extract_codes` never synthesize a body, and every value they
//! return is present in the decoded source — links as a lexical match over
//! it, codes as a digit run inside one decoded text run (decision 0040
//! §2–§4). Both are heuristics, not a browser: a tag boundary can expose a
//! fragment (`<b>48</b><b>2935</b>` yields `2935`), and a link is a
//! candidate, never a validated destination. Everything here is total and
//! linear — a hostile body is a bad body, not a panic or a scan that never
//! ends (bounded by the 0022 body cap, applied before the parse).

mod entities;

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// An http(s) URL up to the first excluded character. The scheme matches
/// ASCII case-insensitively (decision 0040 §4) and is spelled out byte by
/// byte: a `(?i)` flag would also fold `ſ` (U+017F) into `s`.
static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[hH][tT][tT][pP][sS]?://[^\s"'<>)\]}>,]+"#).unwrap());

/// ASCII digits between ASCII word boundaries (decision 0040 §2): `\d`
/// would admit Arabic-Indic and every other Unicode digit, and pays for
/// the Unicode tables on every body.
static CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b)[0-9]{4,10}(?-u:\b)").unwrap());

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

/// Decode HTML character references, once (decision 0040 §5): the
/// semicolon-terminated named references of [`entities`] (exact case, so
/// `&Lt;` is U+226A and `&AMP;` is `&`) plus decimal and hex numerics.
/// Total and linear — an unknown name, a missing semicolon, a malformed
/// form or a numeric outside the Unicode scalar range (or a surrogate)
/// stays literal, and the output is never decoded again, so `&amp;lt;` is
/// the text `&lt;`. `&#0;` and other control characters are honest
/// decodes of what the sender wrote — the store keeps them; the display
/// plane owns their presentation.
///
/// A named reference can grow its input — `&nGt;` is five bytes and
/// decodes to six — but never past `5 * output <= 6 * input`: numerics
/// and literal copies never grow, and the table's own test pins every
/// mapping inside that ratio.
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
        match parse_ref(rest, &mut out, stats) {
            Some(len) => rest = &rest[len..],
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

/// One character reference at the start of `s`, appended to `out`: the
/// bytes consumed, or `None` (with `out` untouched) when `s` does not begin
/// with a well-formed reference — the signal to copy the `&` through
/// literally.
fn parse_ref(s: &str, out: &mut String, stats: &mut RefScanStats) -> Option<usize> {
    // The reference is bounded: the longest standard name and a numeric's
    // digits stay inside forty characters, so a terminator that is not
    // found within the window means the '&' is literal — and the scan
    // stays total and linear. A window-less `find(';')` re-scans to the end
    // of the source for every '&': a hostile `Content-Type: text/html` body
    // of one 50 MiB '&' run pins a blocking worker for hours, and every
    // store mutation shares that pool (the review's security HOLD). A name
    // lookup sees only the bounded token.
    const REF_MAX: usize = 40;
    stats.scans += 1;
    let window = match s[1..].char_indices().nth(REF_MAX) {
        Some((byte_index, _)) => &s[1..1 + byte_index],
        None => &s[1..],
    };
    stats.bytes += window.len() as u64;
    let token_end = window.find(';')? + 1;
    let token = &s[1..token_end];
    if let Some(hex) = token
        .strip_prefix("#x")
        .or_else(|| token.strip_prefix("#X"))
    {
        out.push(u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)?);
    } else if let Some(decimal) = token.strip_prefix('#') {
        out.push(decimal.parse::<u32>().ok().and_then(char::from_u32)?);
    } else {
        out.push_str(entities::lookup(token)?);
    }
    Some(token_end + 1)
}

/// Extract all http(s) URLs from the plain-text and HTML bodies, text
/// first, in document order, exact-string deduplicated. The HTML source is
/// reference-decoded *once, whole, before* the URL pattern cuts it
/// (decision 0040 §4, issue #94): `&lt;https://x.io/u&gt;` is the sender's
/// `<https://x.io/u>`, so the decoded `>` must end the link — decoding
/// after the cut kept it inside. The decoded source is regex input only,
/// never tokenized as HTML. The plain-text leg has no entities to decode:
/// `&amp;` in plain text is literally `&amp;`.
pub fn extract_links(text: Option<&str>, html: Option<&str>) -> Vec<String> {
    let decoded = html.map(decode_refs);
    let mut out = Vec::new();
    for src in [text, decoded.as_deref()].into_iter().flatten() {
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

/// Extract likely verification/OTP codes: standalone runs of 4–10 ASCII
/// digits. Text body first (most reliable); if it holds none, the HTML
/// body's text runs (decision 0040 §2) — each run between two pieces of
/// markup decoded and scanned on its own, so digits in adjacent cells or
/// blocks are never joined and a code beside its sentence is still found.
/// Digits inside `<style>`, `<script>`, `<title>`, `<head>`, `<template>`,
/// comments or attribute values yield nothing: a stylesheet color is not a
/// code, and an attribute is not displayed to the person the code is for.
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
    let mut stats = TagScanStats::default();
    dedup(html.map(|h| html_codes(h, &mut stats)).unwrap_or_default())
}

/// The tokenizer's work for one [`html_codes`] call (decision 0040 §6),
/// observed like [`RefScanStats`]: an instance-local value, never a
/// global. Every source byte lands in exactly one of the three consumption
/// counters, so they sum to the source length; `close_probes` counts the
/// raw-text close attempts, each a fixed-width comparison of the element's
/// name and one delimiter byte. The cursor only moves forward, so the
/// scan is linear in the source.
#[derive(Default)]
#[cfg_attr(not(test), allow(dead_code))]
struct TagScanStats {
    /// Bytes outside markup — text runs, emitted or suppressed, including
    /// every `<` that does not open markup.
    text_bytes: u64,
    /// Bytes of tags, comments, declarations and unterminated markup.
    markup_bytes: u64,
    /// Bytes of `script`/`style`/`title` content.
    raw_bytes: u64,
    /// One per `</` examined inside raw text.
    close_probes: u64,
    /// The reference decoder's work over the emitted runs.
    refs: RefScanStats,
}

#[cfg(test)]
fn html_codes_with_stats(html: &str) -> (Vec<String>, TagScanStats) {
    let mut stats = TagScanStats::default();
    let codes = html_codes(html, &mut stats);
    (codes, stats)
}

/// The containers whose whole content is suppressed and whose nesting is
/// tracked: only a close matching the innermost one ends it.
#[derive(Clone, Copy, PartialEq)]
enum Container {
    Head,
    Template,
}

fn container(name: &[u8]) -> Option<Container> {
    if name.eq_ignore_ascii_case(b"head") {
        Some(Container::Head)
    } else if name.eq_ignore_ascii_case(b"template") {
        Some(Container::Template)
    } else {
        None
    }
}

/// The raw-text elements: their content is never markup, and only their
/// own closing name ends it — `<script>"</p>"</script>` is all script.
const RAW_TEXT: [&[u8]; 3] = [b"script", b"style", b"title"];

fn raw_text(name: &[u8]) -> Option<&'static [u8]> {
    RAW_TEXT
        .into_iter()
        .find(|raw| name.eq_ignore_ascii_case(raw))
}

/// A tag name ends at whitespace, `/` or `>`.
fn ends_name(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'/' || byte == b'>'
}

/// `<` opens markup only before an ASCII letter, `/`, `!` or `?`; any
/// other less-than sign — `a < b`, `<3`, `<é` — is text.
fn opens_markup(next: Option<&u8>) -> bool {
    next.is_some_and(|&byte| byte.is_ascii_alphabetic() || matches!(byte, b'/' | b'!' | b'?'))
}

enum Kind<'a> {
    Start(&'a [u8]),
    End(&'a [u8]),
    Other,
}

/// The markup that begins at `lt`: where it ends (one past its `>`) and
/// what it is, or `None` when the source ends inside it.
fn markup_at(bytes: &[u8], lt: usize) -> Option<(usize, Kind<'_>)> {
    let rest = &bytes[lt..];
    if let Some(body) = rest.strip_prefix(b"<!--") {
        // `<!-->` and `<!--->` are the abrupt closes; otherwise `-->`.
        let len = if body.starts_with(b">") {
            1
        } else if body.starts_with(b"->") {
            2
        } else {
            body.windows(3).position(|w| w == b"-->")? + 3
        };
        return Some((lt + 4 + len, Kind::Other));
    }
    match rest[1] {
        b'!' | b'?' => {
            let gt = rest[2..].iter().position(|&byte| byte == b'>')?;
            Some((lt + 2 + gt + 1, Kind::Other))
        }
        b'/' => tag(bytes, lt + 2).map(|(name, end)| (end, Kind::End(name))),
        _ => tag(bytes, lt + 1).map(|(name, end)| (end, Kind::Start(name))),
    }
}

/// A tag's name from `start`, and the end of the tag: one past the first
/// `>` that is not inside a quoted attribute value. A quote opens a value
/// only after `=` and optional whitespace, so the apostrophe in
/// `alt=don't` is a value byte; a quoted `>` never ends the tag.
fn tag(bytes: &[u8], start: usize) -> Option<(&[u8], usize)> {
    let name_len = bytes[start..].iter().position(|&byte| ends_name(byte))?;
    let mut index = start + name_len;
    let mut after_equals = false;
    loop {
        match *bytes.get(index)? {
            b'>' => return Some((&bytes[start..start + name_len], index + 1)),
            b'=' => after_equals = true,
            quote @ (b'"' | b'\'') if after_equals => {
                index += 1 + bytes[index + 1..].iter().position(|&byte| byte == quote)?;
                after_equals = false;
            }
            byte if byte.is_ascii_whitespace() => {}
            _ => after_equals = false,
        }
        index += 1;
    }
}

/// The `<` of the first `</name` followed by a name delimiter at or after
/// `from` (ASCII case-insensitive), or `None` if the raw text runs to the
/// end of the source. Each `</` costs one fixed-width probe and the search
/// resumes after it, so near misses never rescan.
fn raw_close(html: &str, from: usize, name: &[u8], stats: &mut TagScanStats) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut search = from;
    while let Some(offset) = html[search..].find("</") {
        let lt = search + offset;
        stats.close_probes += 1;
        let name_end = lt + 2 + name.len();
        if bytes
            .get(lt + 2..name_end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
            && bytes.get(name_end).is_some_and(|&byte| ends_name(byte))
        {
            return Some(lt);
        }
        search = lt + 2;
    }
    None
}

/// Consume the markup at `lt` — for a raw-text element, also its content
/// and its close — updating the container stack: the offset after it, or
/// `None` when the source ends inside it (the remainder is suppressed).
fn skip_markup(
    html: &str,
    lt: usize,
    containers: &mut Vec<Container>,
    stats: &mut TagScanStats,
) -> Option<usize> {
    let len = html.len();
    let Some((end, kind)) = markup_at(html.as_bytes(), lt) else {
        stats.markup_bytes += (len - lt) as u64;
        return None;
    };
    stats.markup_bytes += (end - lt) as u64;
    match kind {
        Kind::Start(name) => {
            // A self-closing spelling still opens the element: the tag's
            // trailing `/` changes nothing here.
            if let Some(raw) = raw_text(name) {
                let Some(close) = raw_close(html, end, raw, stats) else {
                    stats.raw_bytes += (len - end) as u64;
                    return None;
                };
                stats.raw_bytes += (close - end) as u64;
                let Some((_, close_end)) = tag(html.as_bytes(), close + 2) else {
                    stats.markup_bytes += (len - close) as u64;
                    return None;
                };
                stats.markup_bytes += (close_end - close) as u64;
                return Some(close_end);
            }
            if let Some(opened) = container(name) {
                containers.push(opened);
            }
        }
        Kind::End(name) => {
            if container(name).is_some_and(|closed| containers.last() == Some(&closed)) {
                containers.pop();
            }
        }
        Kind::Other => {}
    }
    Some(end)
}

/// The codes in an HTML body's text runs. Structure is recognized first,
/// then each run outside every suppressed region is decoded once and
/// scanned on its own — every markup boundary ends a run. Slicing happens
/// only at ASCII `<`/`>` positions or the ends, so it never splits a UTF-8
/// character.
fn html_codes(html: &str, stats: &mut TagScanStats) -> Vec<String> {
    let mut codes = Vec::new();
    let mut containers = Vec::new();
    let mut run = 0;
    let mut search = 0;
    while let Some(offset) = html[search..].find('<') {
        let lt = search + offset;
        search = lt + 1;
        if !opens_markup(html.as_bytes().get(lt + 1)) {
            continue;
        }
        stats.text_bytes += (lt - run) as u64;
        if containers.is_empty() {
            scan_run(&html[run..lt], &mut codes, &mut stats.refs);
        }
        let Some(end) = skip_markup(html, lt, &mut containers, stats) else {
            return codes;
        };
        run = end;
        search = end;
    }
    stats.text_bytes += (html.len() - run) as u64;
    if containers.is_empty() {
        scan_run(&html[run..], &mut codes, &mut stats.refs);
    }
    codes
}

fn scan_run(run: &str, codes: &mut Vec<String>, stats: &mut RefScanStats) {
    let decoded = decode_refs_scanned(run, stats);
    codes.extend(CODE_RE.find_iter(&decoded).map(|m| m.as_str().to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inline elements are run boundaries, so a code beside its sentence is
    // found (issue #93): the renderer glued `go</a> 555555` and
    // `x<i>555555</i>y` into words and lost them.
    #[test]
    fn inline_adjacent_codes_are_their_own_runs() {
        assert_eq!(
            extract_codes(None, Some("See <b>555555</b> soon")),
            vec!["555555"]
        );
        assert_eq!(
            extract_codes(None, Some("<p>hello</p><span>555555</span>")),
            vec!["555555"]
        );
        assert_eq!(
            extract_codes(None, Some("<a href=\"https://x.io\">go</a> 555555")),
            vec!["555555"]
        );
        assert_eq!(extract_codes(None, Some("x<i>555555</i>y")), vec!["555555"]);
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

    #[test]
    fn links_are_extracted_and_trimmed() {
        let html = r#"<a href="https://x.io/verify?t=abc">click.</a> See https://x.io/docs."#;
        assert_eq!(
            extract_links(None, Some(html)),
            vec!["https://x.io/verify?t=abc", "https://x.io/docs"]
        );
    }

    #[test]
    fn codes_prefer_text_body() {
        let codes = extract_codes(Some("Your code is 424242."), Some("<b>11112222</b>"));
        assert_eq!(codes, vec!["424242".to_string()]);
        // No code in the text: the html runs are the fallback.
        assert_eq!(
            extract_codes(Some("no code here"), Some("<b>11112222</b>")),
            vec!["11112222"]
        );
        assert!(extract_codes(None, None).is_empty());
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
        assert_eq!(
            extract_links(None, Some(html)),
            vec![
                "https://x.io/a&bCd",
                "https://x.io/e&nosuch;f&#xZZ;/",
                "https://x.io/&#55296;surrogate&#1114112;/",
            ],
            "unknown names, malformed hex and out-of-range numerics pass through"
        );
    }

    // Decision 0040 §5/§6: every reference form decodes once, malformed
    // ones stay literal, and the output never exceeds 6/5 of the input.
    #[test]
    fn decode_refs_is_total_and_bounded() {
        for input in [
            "&",
            "&amp",
            "&amp;",
            "&#;",
            "&#x;",
            "&AMP;",
            "&Lt;",
            "&nGt;",
            "&nLt;&nGt;",
            "&ThickSpace;",
            "https://x.io/a&amp;b",
            "&&&&amp;;",
            "&notanentity; tail",
            "trailing &",
        ] {
            let decoded = decode_refs(input);
            assert!(
                5 * decoded.len() <= 6 * input.len(),
                "{input} grew: {decoded}"
            );
        }
        assert_eq!(decode_refs("https://x.io/a&amp;b"), "https://x.io/a&b");
        assert_eq!(decode_refs("&"), "&");
        assert_eq!(decode_refs("&amp"), "&amp", "a semicolon is required");
        assert_eq!(decode_refs("&nbsp"), "&nbsp");
        assert_eq!(decode_refs("&AMP;&LT;&GT;&QUOT;"), "&<>\"");
        assert_eq!(decode_refs("&Amp;"), "&Amp;", "names are exact-case");
        assert_eq!(decode_refs("&Lt;&Gt;"), "\u{226a}\u{226b}");
        assert_eq!(decode_refs("&nbsp;"), "\u{a0}");
        assert_eq!(decode_refs("&fjlig;"), "fj");
        assert_eq!(decode_refs("&NotEqualTilde;"), "\u{2242}\u{338}");
        assert_eq!(decode_refs("&ThickSpace;"), "\u{205f}\u{200a}");
        assert_eq!(decode_refs("&nGt;"), "\u{226b}\u{20d2}");
        assert_eq!(decode_refs("&nLt;"), "\u{226a}\u{20d2}");
        assert_eq!(decode_refs("&quot;&apos;&lt;&gt;"), "\"'<>");
        assert_eq!(decode_refs("&#65;&#x42;&#X43;"), "ABC");
        assert_eq!(decode_refs("&#1;&#x0;"), "\u{1}\u{0}", "controls are kept");
        assert_eq!(decode_refs("&#xD800;&#-1;"), "&#xD800;&#-1;");
        assert_eq!(decode_refs("&amp;lt;"), "&lt;", "decoded once, never again");
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
        assert_eq!(
            extract_codes(None, Some("<p>9999</p><p>1111</p><p>9999</p>")),
            vec!["9999", "1111"]
        );
    }

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

    // ---- issue #94: decode before the URL pattern cuts ----

    #[test]
    fn decoded_delimiters_end_the_link() {
        for (html, want) in [
            (
                "<p>Unsubscribe: &lt;https://x.io/unsub?u=1&gt;</p>",
                vec!["https://x.io/unsub?u=1"],
            ),
            (
                "<p>Link: &quot;https://x.io/q&quot; ok</p>",
                vec!["https://x.io/q"],
            ),
            (
                "<p>(see https://x.io/p&#41; for more)</p>",
                vec!["https://x.io/p"],
            ),
            ("<a href=\"https://x.io/a&#32;b\">", vec!["https://x.io/a"]),
            (
                "<p>Visit https://x.io/n&nbsp;now</p>",
                vec!["https://x.io/n"],
            ),
            (
                "<a href=\"https://x.io/a?b=1&AMP;c=2\">",
                vec!["https://x.io/a?b=1&c=2"],
            ),
            (
                "<a href=\"HTTPS://X.IO/upper\">",
                vec!["HTTPS://X.IO/upper"],
            ),
            // Encoded scheme punctuation is found only because the whole
            // source is decoded before matching.
            ("<p>https&#58;//x.io/enc</p>", vec!["https://x.io/enc"]),
            (
                "&lt;https://a.io/1&gt;&lt;https://b.io/2&gt;&quot;https://a.io/1&quot;",
                vec!["https://a.io/1", "https://b.io/2"],
            ),
            (
                "<p>https://x.io/Case and hTTp://x.io/Case.</p>",
                vec!["https://x.io/Case", "hTTp://x.io/Case"],
            ),
            // `ſ` is not an ASCII `s`: no Unicode case folding.
            ("<p>http\u{17f}://x.io/long-s</p>", vec![]),
        ] {
            assert_eq!(extract_links(None, Some(html)), want, "{html}");
        }
    }

    #[test]
    fn links_dedupe_across_both_bodies_in_text_first_order() {
        assert_eq!(
            extract_links(
                Some("https://t.io/a &lt;https://h.io/b&gt;"),
                Some("<a href='https://h.io/b'>b</a> https://t.io/a")
            ),
            vec!["https://t.io/a", "https://h.io/b&gt", "https://h.io/b"],
            "plain text is literal: its `&gt;` is never decoded"
        );
    }

    // ---- issue #93: codes come from tokenized text runs ----

    #[test]
    fn the_issue_93_rows_yield_exactly_the_mail_s_codes() {
        for (html, want) in [
            (
                "<table><tr><td>Order 1234</td><td>5678</td></tr></table>",
                vec!["1234", "5678"],
            ),
            (
                "<div>Ref 2026</div><div>482913</div>",
                vec!["2026", "482913"],
            ),
            ("<div>482935</div><div>is your code</div>", vec!["482935"]),
            ("<h1>Your code</h1><h2>482933</h2>", vec!["482933"]),
            ("<ul><li>Code</li><li>482936</li></ul>", vec!["482936"]),
            (
                "<table><tr><td>482937</td><td>expires soon</td></tr></table>",
                vec!["482937"],
            ),
            ("<p><strong>482938</strong>is your code</p>", vec!["482938"]),
            (
                "<style type=\"text/css\">p { color: #333333; }</style><p>Hello</p>",
                vec![],
            ),
            (
                "<script type=\"text/javascript\">var t = 44445555;</script><p>Hello</p>",
                vec![],
            ),
            (
                "<img alt=\"x > 12345678\" src=\"https://x.io/i.png\"><p>Hello</p>",
                vec![],
            ),
        ] {
            assert_eq!(extract_codes(None, Some(html)), want, "{html}");
        }
    }

    #[test]
    fn suppressed_regions_hide_their_codes_whatever_their_attributes() {
        for (html, want) in [
            (
                "<head lang=\"en\"><meta content=\"1111\">2222</head>3333",
                vec!["3333"],
            ),
            ("<HEAD>1111</Head >2222", vec!["2222"]),
            ("<header>1111</header>", vec!["1111"]),
            ("<title>Code 1111</title><p>2222</p>", vec!["2222"]),
            ("<TITLE>1111</title>2222", vec!["2222"]),
            ("<scripts>1111</scripts>", vec!["1111"]),
            // Nested containers: only the matching innermost close pops.
            (
                "<template><template>1111</template>2222</template>3333",
                vec!["3333"],
            ),
            ("<head><p></template>1111</head>2222", vec!["2222"]),
            (
                "<head><template></head>1111</template>2222</head>3333",
                vec!["3333"],
            ),
            ("</head>1111", vec!["1111"]),
            // Raw text: fake markup and other closes stay inside.
            (
                "<script>if (a</b) '</head>' 1111</script>2222",
                vec!["2222"],
            ),
            ("<style><!-- 1111 --></style>2222", vec!["2222"]),
            ("<script>1111</scriptx></SCRIPT\t>2222", vec!["2222"]),
            ("<script>1111</script/>2222", vec!["2222"]),
            // A self-closing spelling still opens the element.
            ("<script/>1111</script>2222", vec!["2222"]),
            ("<head/>1111</head>2222", vec!["2222"]),
        ] {
            assert_eq!(extract_codes(None, Some(html)), want, "{html}");
        }
    }

    #[test]
    fn comments_declarations_and_attributes_are_never_text() {
        for (html, want) in [
            ("<!-- 1111 -->2222", vec!["2222"]),
            ("<!-->1111", vec!["1111"]),
            ("<!--->1111", vec!["1111"]),
            ("<!---->1111", vec!["1111"]),
            ("<!-- a -- b 1111 -->2222", vec!["2222"]),
            ("<!DOCTYPE html 1111>2222", vec!["2222"]),
            ("<![CDATA[1111]]>2222", vec!["2222"]),
            ("<?xml version=\"1111\"?>2222", vec!["2222"]),
            ("<a title='x > 1111'>2222</a>", vec!["2222"]),
            ("<a title = \"x > 1111\">2222</a>", vec!["2222"]),
            ("<a alt=don't>1111</a>", vec!["1111"]),
            ("<a \"x>1111", vec!["1111"]),
            ("<a b=\"1\"c='2' d=3>4444</a>", vec!["4444"]),
            ("<br/>1111<br />2222", vec!["1111", "2222"]),
            ("</ 1111>2222", vec!["2222"]),
            ("</>1111", vec!["1111"]),
        ] {
            assert_eq!(extract_codes(None, Some(html)), want, "{html}");
        }
    }

    // A less-than sign opens markup only before an ASCII letter, `/`, `!`
    // or `?`; everything else stays text, multibyte text included.
    #[test]
    fn only_markup_openers_leave_text() {
        for (html, want) in [
            ("1111 < 2222", vec!["1111", "2222"]),
            ("a <3333 b", vec!["3333"]),
            ("<\u{e9}1111", vec!["1111"]),
            (
                "caf\u{e9} 1111 <p>\u{65e5}\u{672c} 2222</p>",
                vec!["1111", "2222"],
            ),
            ("<", vec![]),
            ("1111<", vec!["1111"]),
        ] {
            assert_eq!(extract_codes(None, Some(html)), want, "{html}");
        }
    }

    // Unterminated recognized markup suppresses the remainder; codes
    // emitted before it remain.
    #[test]
    fn every_unfinished_construct_consumes_the_rest() {
        for html in [
            "1111<!-- 2222",
            "1111<!DOCTYPE 2222",
            "1111<?pi 2222",
            "1111<p 2222",
            "1111<p a=\"2222>",
            "1111<p a='2222>",
            "1111<p",
            "1111</p",
            "1111<script>2222",
            "1111<script>2222</script",
            "1111<script>2222</script a=\">",
            "1111<head>2222",
            "1111<template><p>2222</p>",
        ] {
            assert_eq!(extract_codes(None, Some(html)), vec!["1111"], "{html}");
        }
    }

    #[test]
    fn runs_decode_once_and_codes_are_ascii() {
        for (html, want) in [
            ("&lt;style&gt;333333", vec!["333333"]),
            ("&lt;!-- 1111 --&gt;", vec!["1111"]),
            ("&#52;&#56;&#50;&#57;&#51;&#53;", vec!["482935"]),
            ("<b>48</b><b>2935</b>", vec!["2935"]),
            (
                "123 1234 1234567890 12345678901",
                vec!["1234", "1234567890"],
            ),
            ("\u{661}\u{662}\u{663}\u{664}", vec![]),
            ("\u{661}\u{662}\u{663}\u{664}5678", vec!["5678"]),
            ("1111&nbsp;2222", vec!["1111", "2222"]),
        ] {
            assert_eq!(extract_codes(None, Some(html)), want, "{html}");
        }
        assert!(
            extract_codes(Some("\u{661}\u{662}\u{663}\u{664}"), None).is_empty(),
            "non-ASCII digits are never codes"
        );
    }

    // ---- decision 0040 §6: the tokenizer's work, exactly ----

    #[test]
    fn repeated_tags_and_bare_less_thans_cost_one_pass() {
        let (codes, stats) = html_codes_with_stats(&"<p>".repeat(500_000));
        assert!(codes.is_empty());
        assert_eq!(stats.markup_bytes, 1_500_000);
        assert_eq!(stats.text_bytes, 0);
        assert_eq!(stats.raw_bytes, 0);
        assert_eq!(stats.close_probes, 0);

        let (codes, stats) = html_codes_with_stats(&"<".repeat(500_000));
        assert!(codes.is_empty());
        assert_eq!(stats.text_bytes, 500_000);
        assert_eq!(stats.markup_bytes, 0);
        assert_eq!(stats.refs.scans, 0);
    }

    // Each `</` inside raw text is one probe, and the search resumes after
    // it: 1,000 near misses cost 1,000 probes, not a rescan per miss.
    #[test]
    fn raw_text_near_misses_cost_one_probe_each() {
        let src = format!("<script>{}", "</scrip>".repeat(1_000));
        let (codes, stats) = html_codes_with_stats(&src);
        assert!(codes.is_empty());
        assert_eq!(stats.close_probes, 1_000);
        assert_eq!(stats.markup_bytes, 8);
        assert_eq!(stats.raw_bytes, 8_000);
        assert_eq!(stats.text_bytes, 0);

        let src = format!("<style>{}</style >1234", "</stylex".repeat(3));
        let (codes, stats) = html_codes_with_stats(&src);
        assert_eq!(codes, vec!["1234"]);
        assert_eq!(stats.close_probes, 4);
        assert_eq!(stats.markup_bytes, 7 + 9);
        assert_eq!(stats.raw_bytes, 24);
        assert_eq!(stats.text_bytes, 4);
    }

    #[test]
    fn nested_containers_are_counted_markup_and_suppressed_text() {
        let src = format!(
            "{}1111{}2222",
            "<template>".repeat(3),
            "</template>".repeat(3)
        );
        let (codes, stats) = html_codes_with_stats(&src);
        assert_eq!(codes, vec!["2222"]);
        assert_eq!(stats.markup_bytes, 30 + 33);
        assert_eq!(stats.text_bytes, 8);
        assert_eq!(stats.refs.scans, 0);
    }

    // Every string of length 0..=5 over the tokenizer's structural
    // alphabet, plus a two-byte character: no panic, no invalid slice,
    // every byte accounted to exactly one consumption counter, and every
    // code a digit run present in the input. This complements the
    // monotonic-cursor argument; it is not a proof for all strings.
    #[test]
    fn small_alphabet_enumeration_is_total() {
        const ALPHABET: [&str; 14] = [
            "<", ">", "/", "!", "-", "=", "\"", "'", "&", ";", "#", "a", "1", "\u{e9}",
        ];
        let mut input = String::new();
        for len in 0..=5u32 {
            for mut index in 0..ALPHABET.len().pow(len) {
                input.clear();
                for _ in 0..len {
                    input.push_str(ALPHABET[index % ALPHABET.len()]);
                    index /= ALPHABET.len();
                }
                let (codes, stats) = html_codes_with_stats(&input);
                assert_eq!(
                    stats.text_bytes + stats.markup_bytes + stats.raw_bytes,
                    input.len() as u64,
                    "{input:?}"
                );
                assert!(
                    codes
                        .iter()
                        .all(|code| code.bytes().all(|byte| byte == b'1'))
                );
                let _ = extract_links(Some(&input), Some(&input));
            }
        }
    }

    #[test]
    fn long_multibyte_and_malformed_reference_fixtures_terminate() {
        for src in [
            "\u{65e5}<\u{65e5}</\u{65e5}<!\u{65e5}<a \u{65e5}='\u{65e5}'>1111".repeat(100),
            "<script>\u{e9}</scr\u{e9}ipt>".repeat(100),
            "&#x1F600;&#99999999999;&#x;&\u{e9};&;".repeat(100),
        ] {
            let (_, stats) = html_codes_with_stats(&src);
            assert_eq!(
                stats.text_bytes + stats.markup_bytes + stats.raw_bytes,
                src.len() as u64
            );
        }
        assert_eq!(
            extract_codes(None, Some("<a \u{e9}='x'>1111</a>")),
            vec!["1111"]
        );
    }
}
