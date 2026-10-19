//! Link and verification-code extraction.
//!
//! Extraction happens once, at ingest (decision 0005), over the two bodies
//! the sender actually sent (decision 0031 §1, 0040 §1): `extract_links`
//! and `extract_codes` never synthesize a body, and every value they
//! return is present in the decoded source — links as a lexical match over
//! it, codes as a digit run inside one decoded text run (decision 0040
//! §2–§4). Both are heuristics, not a browser: a tag boundary can expose a
//! fragment (`<b>48</b><b>2935</b>` yields `2935`), and a link is a
//! candidate, never a validated destination. One tokenizer walk owns the
//! HTML structure for both: codes read its text runs, links its quoted
//! attribute values and everything between (decision 0049 §1). A fresh
//! mail keeps at most [`MAX_CODES`] distinct codes (0049 §4); links are
//! bounded only by the body. Everything here is total and
//! linear — a hostile body is a bad body, not a panic or a scan that never
//! ends (bounded by the 0022 body cap, applied before the parse).

mod entities;

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// An http(s) URL in ordinary source, up to whitespace, a quote or an angle
/// bracket. The scheme matches ASCII case-insensitively (decision 0040 §4)
/// and is spelled out byte by byte: a `(?i)` flag would also fold `ſ`
/// (U+017F) into `s`. Commas, brackets and parentheses are URL bytes here;
/// [`trim_url`] removes the sentence punctuation and the unmatched closers
/// a candidate ends with (issue #136).
static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[hH][tT][tT][pP][sS]?://[^\s"'<>]+"#).unwrap());

/// An http(s) URL inside one quoted attribute value (decision 0049 §1): the
/// source's own closing quote already ended the value, so a quote decoded
/// from `&#39;` or `&quot;` is URL data a browser follows — the
/// `O&#39;Brien` reset link stays whole (issue #136).
static ATTR_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[hH][tT][tT][pP][sS]?://[^\s<>]+").unwrap());

/// The most distinct codes one freshly ingested mail keeps (decision 0049
/// §4, issue #139): the first 64 in extraction order. An 8 MiB body of
/// numbers would otherwise store a million strings and repeat them in
/// every page and webhook payload.
pub const MAX_CODES: usize = 64;

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
        out.push(scalar(hex, 16)?);
    } else if let Some(decimal) = token.strip_prefix('#') {
        out.push(scalar(decimal, 10)?);
    } else {
        out.push_str(entities::lookup(token)?);
    }
    Some(token_end + 1)
}

/// A numeric reference's scalar: only ASCII digits of the radix (decision
/// 0049 §2, issue #136). `from_str_radix` alone also takes a leading `+`,
/// so `&#+52;` decoded to `4` and four of them manufactured the code
/// `4829`; a sign, a non-ASCII digit, an overflow or a non-scalar stays
/// literal.
fn scalar(digits: &str, radix: u32) -> Option<char> {
    digits
        .bytes()
        .all(|byte| char::from(byte).is_digit(radix))
        .then(|| u32::from_str_radix(digits, radix).ok())
        .flatten()
        .and_then(char::from_u32)
}

/// The link scan's shape for one [`extract_links`] call, observed like
/// [`RefScanStats`]: an instance-local value the tests read, never a
/// global. Every HTML source byte lands in exactly one of the two region
/// counters, so they sum to the source length; the trim examines each
/// candidate once and then removes at most its own length, so the scan
/// stays linear whatever the punctuation (decision 0049 §1).
#[derive(Default)]
#[cfg_attr(not(test), allow(dead_code))]
struct LinkScanStats {
    /// HTML source outside every quoted attribute value: text, markup,
    /// comments, raw text and any unfinished tail, each decoded once.
    ordinary_bytes: u64,
    /// HTML source inside quoted attribute values, quotes excluded.
    quoted_bytes: u64,
    /// Candidate bytes the trim's one counting pass examined.
    trim_counted: u64,
    /// Bytes the trim removed, one step each.
    trim_removed: u64,
}

/// Extract all http(s) URLs from the plain-text and HTML bodies, text
/// first, in document order, exact-string deduplicated. The plain-text leg
/// has no entities to decode: `&amp;` in plain text is literally `&amp;`.
///
/// The HTML leg is cut by its source structure first and decoded after
/// (decision 0049 §1, issue #136): the one tokenizer walk the code scan
/// uses reports every quoted attribute value, which is decoded once and
/// matched with quotes allowed — the source's closing quote ended it, so a
/// decoded `'` is the link's own. Everything else — text, markup outside
/// values, comments, raw text, an unfinished tail — is decoded once and
/// matched as before, so `&lt;https://x.io/u&gt;` still ends at the decoded
/// `>` (issue #94). Decoded source is regex input only, never tokenized.
pub fn extract_links(text: Option<&str>, html: Option<&str>) -> Vec<String> {
    let mut stats = LinkScanStats::default();
    links_scanned(text, html, &mut stats)
}

fn links_scanned(text: Option<&str>, html: Option<&str>, stats: &mut LinkScanStats) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(text) = text {
        push_urls(&URL_RE, text, &mut out, stats);
    }
    if let Some(html) = html {
        let mut regions = LinkRegions {
            html,
            cursor: 0,
            out: &mut out,
            stats,
        };
        walk(html, &mut TagScanStats::default(), &mut regions);
        regions.ordinary(html.len());
    }
    dedup(out)
}

#[cfg(test)]
fn links_with_stats(text: Option<&str>, html: Option<&str>) -> (Vec<String>, LinkScanStats) {
    let mut stats = LinkScanStats::default();
    let links = links_scanned(text, html, &mut stats);
    (links, stats)
}

fn push_urls(re: &Regex, src: &str, out: &mut Vec<String>, stats: &mut LinkScanStats) {
    for m in re.find_iter(src) {
        out.push(trim_url(m.as_str(), stats).to_string());
    }
}

/// The bracket pairs a URL may carry when balanced: Wikipedia's
/// `Foo_(bar)`, an IPv6 literal's `[::1]`, a template's `{id}`.
const OPENERS: &[u8; 3] = b"([{";
const CLOSERS: &[u8; 3] = b")]}";

/// A candidate without the sentence punctuation and unmatched closers it
/// ends with (decision 0049 §1): `(see https://x.io/p).` keeps
/// `https://x.io/p`, while `https://x.io/wiki/Foo_(bar)` and
/// `http://[::1]:8080/p` stay whole. One counting pass, then each trimmed
/// byte decrements its pair's surplus — linear, never a rescan. The scheme
/// is never trimmed, so the candidate never empties, and only ASCII bytes
/// are removed, so the cut is a character boundary.
fn trim_url<'a>(url: &'a str, stats: &mut LinkScanStats) -> &'a str {
    let bytes = url.as_bytes();
    let mut surplus = [0i64; 3];
    for &byte in bytes {
        if let Some(pair) = OPENERS.iter().position(|&open| open == byte) {
            surplus[pair] -= 1;
        } else if let Some(pair) = CLOSERS.iter().position(|&close| close == byte) {
            surplus[pair] += 1;
        }
    }
    stats.trim_counted += bytes.len() as u64;
    let mut end = bytes.len();
    loop {
        let last = bytes[end - 1];
        if b".,;:!?".contains(&last) {
            // Sentence punctuation glued on by formatting.
        } else if let Some(pair) = CLOSERS
            .iter()
            .position(|&close| close == last)
            .filter(|&pair| surplus[pair] > 0)
        {
            surplus[pair] -= 1;
        } else {
            return &url[..end];
        }
        end -= 1;
        stats.trim_removed += 1;
    }
}

/// The link consumer of the HTML walk: ordinary source from the cursor up
/// to each quoted value, then the value itself.
struct LinkRegions<'a> {
    html: &'a str,
    cursor: usize,
    out: &'a mut Vec<String>,
    stats: &'a mut LinkScanStats,
}

impl LinkRegions<'_> {
    /// The ordinary source from the cursor to `end`, decoded once.
    fn ordinary(&mut self, end: usize) {
        let src = &self.html[self.cursor..end];
        self.stats.ordinary_bytes += src.len() as u64;
        push_urls(&URL_RE, &decode_refs(src), self.out, self.stats);
        self.cursor = end;
    }
}

impl Regions for LinkRegions<'_> {
    fn text(&mut self, _run: &str, _refs: &mut RefScanStats) {}

    fn value(&mut self, start: usize, end: usize) {
        self.ordinary(start);
        let value = &self.html[start..end];
        self.stats.quoted_bytes += value.len() as u64;
        push_urls(&ATTR_URL_RE, &decode_refs(value), self.out, self.stats);
        self.cursor = end;
    }
}

/// How the two selected bodies sit in the MIME tree (decision 0049 §3,
/// issue #137), decided at ingest from the parsed message and never stored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyOrder {
    /// Direct children of one `multipart/alternative` — two renderings of
    /// one message: the text's codes, and the HTML's only when the text
    /// has none. Also the order for a mail with fewer than two bodies.
    Alternatives,
    /// Independent parts, the text part first in the source.
    TextFirst,
    /// Independent parts, the HTML part first in the source.
    HtmlFirst,
}

/// Extract likely verification/OTP codes: standalone runs of 4–10 ASCII
/// digits, the first [`MAX_CODES`] distinct ones in extraction order.
///
/// The bodies contribute as `order` says (decision 0049 §3): alternatives
/// prefer the text — the most reliable rendering — and fall back to the
/// HTML; independent parts both contribute in source order, so a
/// mailing-list footer after an HTML-only mail no longer hides its code.
/// The HTML contributes its text runs (decision 0040 §2) — each run between
/// two pieces of markup decoded and scanned on its own, so digits in
/// adjacent cells or blocks are never joined and a code beside its
/// sentence is still found. Digits inside `<style>`, `<script>`,
/// `<title>`, `<head>`, `<template>`, comments or attribute values yield
/// nothing: a stylesheet color is not a code, and an attribute is not
/// displayed to the person the code is for.
pub fn extract_codes(text: Option<&str>, html: Option<&str>, order: BodyOrder) -> Vec<String> {
    let mut codes = CodeSet::default();
    let scan_html = |codes: &mut CodeSet| {
        if let Some(html) = html {
            walk(html, &mut TagScanStats::default(), codes);
        }
    };
    match order {
        BodyOrder::Alternatives => {
            codes.scan(text.unwrap_or_default());
            if codes.codes.is_empty() {
                scan_html(&mut codes);
            }
        }
        BodyOrder::TextFirst => {
            codes.scan(text.unwrap_or_default());
            scan_html(&mut codes);
        }
        BodyOrder::HtmlFirst => {
            scan_html(&mut codes);
            codes.scan(text.unwrap_or_default());
        }
    }
    codes.codes
}

/// The bounded distinct collector (decision 0049 §4): a code is checked
/// against at most [`MAX_CODES`] retained strings *before* it is copied,
/// a duplicate takes no slot, and once full nothing more is matched or
/// allocated — the storage never holds a 65th string, so the bound is on
/// allocation, not a truncation after an unbounded collection.
#[derive(Default)]
struct CodeSet {
    codes: Vec<String>,
}

impl CodeSet {
    fn scan(&mut self, src: &str) {
        let mut matches = CODE_RE.find_iter(src);
        while self.codes.len() < MAX_CODES {
            let Some(found) = matches.next() else {
                return;
            };
            if !self.codes.iter().any(|code| code == found.as_str()) {
                self.codes.push(found.as_str().to_string());
            }
        }
    }
}

impl Regions for CodeSet {
    /// A run is decoded whether or not the set is full: the reference
    /// scan's work stays exactly what decision 0040 §6 pins.
    fn text(&mut self, run: &str, refs: &mut RefScanStats) {
        let decoded = decode_refs_scanned(run, refs);
        self.scan(&decoded);
    }

    fn value(&mut self, _start: usize, _end: usize) {}
}

/// The tokenizer's work for one [`walk`] (decision 0040 §6),
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
    let mut codes = CodeSet::default();
    walk(html, &mut stats, &mut codes);
    (codes.codes, stats)
}

/// What one HTML walk reports (decision 0049 §1): the code consumer's
/// text runs and the link consumer's quoted attribute values, both from
/// the same lexical recognition — one owner of where a quote opens and
/// where a tag ends, two consumers with their own suppression.
trait Regions {
    /// A text run outside every suppressed region, still undecoded.
    fn text(&mut self, run: &str, refs: &mut RefScanStats);
    /// The source byte range of one quoted attribute value, its quotes
    /// excluded. Reported in source order as the tag is scanned.
    fn value(&mut self, start: usize, end: usize);
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
fn markup_at<'a>(
    bytes: &'a [u8],
    lt: usize,
    regions: &mut dyn Regions,
) -> Option<(usize, Kind<'a>)> {
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
        b'/' => tag(bytes, lt + 2, regions).map(|(name, end)| (end, Kind::End(name))),
        _ => tag(bytes, lt + 1, regions).map(|(name, end)| (end, Kind::Start(name))),
    }
}

/// A tag's name from `start`, and the end of the tag: one past the first
/// `>` that is not inside a quoted attribute value. A quote opens a value
/// only after `=` and optional whitespace, so the apostrophe in
/// `alt=don't` is a value byte; a quoted `>` never ends the tag. Each
/// closed value is reported to `regions` as soon as its quote closes.
fn tag<'a>(bytes: &'a [u8], start: usize, regions: &mut dyn Regions) -> Option<(&'a [u8], usize)> {
    let name_len = bytes[start..].iter().position(|&byte| ends_name(byte))?;
    let mut index = start + name_len;
    let mut after_equals = false;
    loop {
        match *bytes.get(index)? {
            b'>' => return Some((&bytes[start..start + name_len], index + 1)),
            b'=' => after_equals = true,
            quote @ (b'"' | b'\'') if after_equals => {
                let close =
                    index + 1 + bytes[index + 1..].iter().position(|&byte| byte == quote)?;
                regions.value(index + 1, close);
                index = close;
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
    regions: &mut dyn Regions,
) -> Option<usize> {
    let len = html.len();
    let Some((end, kind)) = markup_at(html.as_bytes(), lt, regions) else {
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
                let Some((_, close_end)) = tag(html.as_bytes(), close + 2, regions) else {
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

/// One walk over an HTML body's source structure. Structure is recognized
/// first: each text run outside every suppressed region goes to
/// `regions.text` — every markup boundary ends a run — and each quoted
/// attribute value to `regions.value`. Unterminated recognized markup ends
/// the walk; what follows is the caller's. Slicing happens only at ASCII
/// `<`/`>`/quote positions or the ends, so it never splits a UTF-8
/// character.
fn walk(html: &str, stats: &mut TagScanStats, regions: &mut dyn Regions) {
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
            regions.text(&html[run..lt], &mut stats.refs);
        }
        let Some(end) = skip_markup(html, lt, &mut containers, stats, regions) else {
            return;
        };
        run = end;
        search = end;
    }
    stats.text_bytes += (html.len() - run) as u64;
    if containers.is_empty() {
        regions.text(&html[run..], &mut stats.refs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codes for two bodies that are alternatives — the text-preferring
    /// order every pre-0049 fixture was written against.
    fn alt_codes(text: Option<&str>, html: Option<&str>) -> Vec<String> {
        extract_codes(text, html, BodyOrder::Alternatives)
    }

    // Inline elements are run boundaries, so a code beside its sentence is
    // found (issue #93): the renderer glued `go</a> 555555` and
    // `x<i>555555</i>y` into words and lost them.
    #[test]
    fn inline_adjacent_codes_are_their_own_runs() {
        assert_eq!(
            alt_codes(None, Some("See <b>555555</b> soon")),
            vec!["555555"]
        );
        assert_eq!(
            alt_codes(None, Some("<p>hello</p><span>555555</span>")),
            vec!["555555"]
        );
        assert_eq!(
            alt_codes(None, Some("<a href=\"https://x.io\">go</a> 555555")),
            vec!["555555"]
        );
        assert_eq!(alt_codes(None, Some("x<i>555555</i>y")), vec!["555555"]);
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

    // The window bounds the *search*, not only its accounting (issue #143):
    // a valid decimal reference for `A` padded with leading zeros, its `;`
    // the 41st character after the '&', lies past the 40-character window
    // and stays literal; one zero fewer puts the `;` 40th — the boundary —
    // and it decodes. A decoder that still counts the window but searches
    // past it decodes the long one too, and the output says so. Both scans
    // examine exactly the 40-byte window.
    #[test]
    fn a_reference_whose_semicolon_lies_past_the_window_stays_literal() {
        let inside = format!("&#{}65;", "0".repeat(36));
        let past = format!("&#{}65;", "0".repeat(37));
        assert_eq!((inside.len(), past.len()), (41, 42));
        let (out, stats) = decode_refs_with_stats(&inside);
        assert_eq!(out, "A", "the boundary-valid reference decodes");
        assert_eq!((stats.scans, stats.bytes), (1, 40));
        let (out, stats) = decode_refs_with_stats(&past);
        assert_eq!(out, past, "past the window, the reference is literal");
        assert_eq!((stats.scans, stats.bytes), (1, 40));
        // The same verdicts through the html leg's real output.
        let html = format!(r#"<a href="https://x.io/{inside}/{past}/z">go</a>"#);
        assert_eq!(
            extract_links(None, Some(&html)),
            vec![format!("https://x.io/A/{past}/z")]
        );
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
        let codes = alt_codes(Some("Your code is 424242."), Some("<b>11112222</b>"));
        assert_eq!(codes, vec!["424242".to_string()]);
        // No code in the text: the html runs are the fallback.
        assert_eq!(
            alt_codes(Some("no code here"), Some("<b>11112222</b>")),
            vec!["11112222"]
        );
        assert!(alt_codes(None, None).is_empty());
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

    // Decision 0040 §5 / 0049 §2: the snapshot's 106 legacy names without
    // a semicolon are not syntax here, in either context — `&n` and
    // `&n=1` stay literal, while `&n;x` decodes the full mapping and keeps
    // the `x`. This is deliberately narrower than the HTML tokenizer's
    // context-dependent bare matches; unknown `&notit;` gets no
    // longest-prefix recovery. Derived from the independent JSON.
    #[test]
    fn legacy_names_need_their_semicolon() {
        let source = entities::tests::snapshot();
        let legacy: Vec<_> = source
            .iter()
            .filter(|(key, _)| !key.ends_with(';'))
            .collect();
        assert_eq!(legacy.len(), 106);
        for (key, entry) in legacy {
            let value = entities::tests::mapping(key, entry);
            assert_eq!(decode_refs(key), *key);
            assert_eq!(decode_refs(&format!("{key}=1")), format!("{key}=1"));
            assert_eq!(decode_refs(&format!("{key};x")), format!("{value}x"));
            let html = format!("<a href=\"https://x.io/?a{key}=1\">go</a> https://y.io/?b{key}=2");
            assert_eq!(
                extract_links(None, Some(&html)),
                vec![
                    format!("https://x.io/?a{key}=1"),
                    format!("https://y.io/?b{key}=2")
                ],
                "{key}: literal in an attribute and in text"
            );
        }
        assert_eq!(decode_refs("&amp"), "&amp");
        assert_eq!(decode_refs("&amp=1"), "&amp=1");
        assert_eq!(decode_refs("&notit;"), "&notit;");
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
        let codes = alt_codes(Some("9999 then 1111 then 9999"), None);
        assert_eq!(codes, vec!["9999".to_string(), "1111".to_string()]);
        assert_eq!(
            alt_codes(None, Some("<p>9999</p><p>1111</p><p>9999</p>")),
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
        assert_eq!(alt_codes(None, Some(html)), vec!["12345678".to_string()]);
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

    // ---- issue #136: source boundaries before decoding ----

    // A quoted attribute value ends at its own source quote, then decodes:
    // the encoded apostrophes MarkupSafe (`&#39;`) and Django (`&#x27;`)
    // write, and an encoded `&quot;`, are the URL's own bytes. Red before:
    // the whole-source decode turned them into delimiters, and every
    // `O'Brien` link stopped at `name=O`.
    #[test]
    fn attribute_links_keep_encoded_quotes() {
        let reset = "https://app.test/reset?name=O'Brien&token=abc123";
        for (html, want) in [
            (
                "<a href=\"https://app.test/reset?name=O&#39;Brien&amp;token=abc123\">r</a>",
                vec![reset],
            ),
            (
                "<a href=\"https://app.test/reset?name=O&#x27;Brien&amp;token=abc123\">r</a>",
                vec![reset],
            ),
            (
                "<a href=\"https://app.test/reset?name=O'Brien&amp;token=abc123\">r</a>",
                vec![reset],
            ),
            (
                "<a href=\"https://x.io/?q=&quot;x&quot;\">q</a>",
                vec!["https://x.io/?q=\"x\""],
            ),
            // The other quote style: a raw `"` inside `'...'` is data too.
            (
                "<a href='https://x.io/a\"b&#39;c'>q</a>",
                vec!["https://x.io/a\"b'c"],
            ),
            // An encoded scheme inside a value still decodes before matching.
            (
                "<a href=\"https&#58;//x.io/enc\">e</a>",
                vec!["https://x.io/enc"],
            ),
            // Outside a quoted value a decoded quote still ends the link:
            // in text, in an unquoted value, in a comment.
            ("<p>https://x.io/O&#39;Brien</p>", vec!["https://x.io/O"]),
            ("<a href=https://x.io/O&#39;B>u</a>", vec!["https://x.io/O"]),
            (
                "<!-- https://x.io/c&#39;d --><p>t</p>",
                vec!["https://x.io/c"],
            ),
            // Source-only links survive where codes are suppressed.
            (
                "<head><script>u=\"https://x.io/s\"</script></head>",
                vec!["https://x.io/s"],
            ),
            (
                "<style>a{background:url(https://x.io/bg.png)}</style>",
                vec!["https://x.io/bg.png"],
            ),
            // An unfinished tail is ordinary source, matched once.
            ("<p>a</p><a href=\"https://x.io/t", vec!["https://x.io/t"]),
            (
                "<a href=\"https://x.io/v&#39;w\" title",
                vec!["https://x.io/v'w"],
            ),
            // A value never reaches into the next one or the text after.
            (
                "<a href=\"https://a.io/1\" data-x='https://b.io/2'>https://c.io/3</a>",
                vec!["https://a.io/1", "https://b.io/2", "https://c.io/3"],
            ),
        ] {
            assert_eq!(extract_links(None, Some(html)), want, "{html}");
        }
    }

    // Commas, IPv6 literals and balanced parentheses are URL bytes; trailing
    // sentence punctuation and unmatched closers are not. Red before: the
    // pattern stopped at the first `)`, `]` or `,`.
    #[test]
    fn links_keep_balanced_url_punctuation() {
        for (src, want) in [
            (
                "see https://en.wikipedia.org/wiki/Foo_(bar) now",
                vec!["https://en.wikipedia.org/wiki/Foo_(bar)"],
            ),
            ("http://[::1]:8080/p", vec!["http://[::1]:8080/p"]),
            (
                "https://cdn.x.io/w_100,h_100/x.jpg",
                vec!["https://cdn.x.io/w_100,h_100/x.jpg"],
            ),
            (
                "(see https://en.wikipedia.org/wiki/Foo_(bar)).",
                vec!["https://en.wikipedia.org/wiki/Foo_(bar)"],
            ),
            ("(see https://x.io/p).", vec!["https://x.io/p"]),
            ("[https://x.io/b]", vec!["https://x.io/b"]),
            ("first https://x.io/a, then", vec!["https://x.io/a"]),
            ("https://x.io/{id}?x=1;", vec!["https://x.io/{id}?x=1"]),
            ("https://x.io/a)b", vec!["https://x.io/a)b"]),
            ("https://x.io/a(", vec!["https://x.io/a("]),
            ("https://x.io/caf\u{e9}).", vec!["https://x.io/caf\u{e9}"]),
        ] {
            assert_eq!(extract_links(Some(src), None), want, "{src}");
            let html = format!("<p>{src}</p><a href=\"{src}\">x</a>");
            assert_eq!(extract_links(None, Some(&html)), want, "{html}");
        }
    }

    // A numeric reference is ASCII digits of its radix, nothing else. Red
    // before: `from_str_radix` took a leading `+`, so `&#+52;` was `4` and
    // four of them manufactured the code `4829`.
    #[test]
    fn numeric_references_refuse_signs() {
        for literal in [
            "&#+52;",
            "&#x+34;",
            "&#X+34;",
            "&#-52;",
            "&#\u{664};",
            "&#x\u{664};",
            "&# 52;",
        ] {
            assert_eq!(decode_refs(literal), literal);
        }
        assert_eq!(decode_refs("&#52;&#x34;&#X34;"), "444");
        assert!(alt_codes(None, Some("&#+52;&#+56;&#+50;&#+57;")).is_empty());
        assert_eq!(alt_codes(None, Some("&#52;&#56;&#50;&#57;")), vec!["4829"]);
        assert_eq!(
            extract_links(None, Some("<a href=\"https://x.io/&#+52;/\">x</a>")),
            vec!["https://x.io/&#+52;/"]
        );
    }

    // Every HTML byte is matched in exactly one region, and the trim
    // examines a candidate once and removes each byte once: exact counts
    // over malformed, multibyte and long-punctuation inputs (decision 0049
    // §1).
    #[test]
    fn link_regions_and_trim_cost_one_pass() {
        let anchor = "<a href=\"https://x.io/a\">b</a>";
        let (links, stats) = links_with_stats(None, Some(anchor));
        assert_eq!(links, vec!["https://x.io/a"]);
        assert_eq!(stats.quoted_bytes, 14);
        assert_eq!(stats.ordinary_bytes, anchor.len() as u64 - 14);
        assert_eq!((stats.trim_counted, stats.trim_removed), (14, 0));

        for html in [
            "<a href=\"x\" title='\u{e9}'>\u{65e5}</a><a b=\"",
            "<p a=\"1\"<q b='2'>",
            "<script>\"</script x=\"y\">",
            "<!-- \" -->'<a c=\"&#39;\">",
            "&lt;a href=&quot;https://x.io&quot;&gt;",
        ] {
            let (_, stats) = links_with_stats(None, Some(html));
            assert_eq!(
                stats.ordinary_bytes + stats.quoted_bytes,
                html.len() as u64,
                "{html}"
            );
        }

        let dots = format!("https://x.io/{}", ".".repeat(100_000));
        let (links, stats) = links_with_stats(Some(&dots), None);
        assert_eq!(links, vec!["https://x.io/"]);
        assert_eq!(stats.trim_counted, dots.len() as u64);
        assert_eq!(stats.trim_removed, 100_000);

        let closers = format!("https://x.io/{}", ")]}".repeat(50_000));
        let (links, stats) = links_with_stats(None, Some(&closers));
        assert_eq!(links, vec!["https://x.io/"]);
        assert_eq!(stats.trim_removed, 150_000);

        let balanced = format!("https://x.io/{}{}", "(".repeat(50_000), ")".repeat(50_000));
        let (links, stats) = links_with_stats(Some(&balanced), None);
        assert_eq!(links, vec![balanced.clone()]);
        assert_eq!((stats.trim_counted, stats.trim_removed), (100_013, 0));
    }

    // ---- issue #139: the bounded code collector ----

    // At most 64 distinct codes, first-occurrence order, a duplicate taking
    // no slot — and the bound holds on the collector's own storage while
    // it collects, not on a truncated result. Red before: every match was
    // kept, a million for an 8 MiB numeric body.
    #[test]
    fn codes_cap_bounds_collection_before_allocation() {
        let numbers = |range: std::ops::Range<u32>| -> Vec<String> {
            range.map(|n| (1_000_000 + n).to_string()).collect()
        };
        assert!(alt_codes(Some("none here"), None).is_empty());
        let sixty_four = numbers(0..64);
        assert_eq!(alt_codes(Some(&sixty_four.join(" ")), None), sixty_four);
        let sixty_five = numbers(0..65);
        assert_eq!(
            alt_codes(Some(&sixty_five.join(" ")), None),
            sixty_five[..64]
        );

        // Duplicates around the boundary: they consume no slot, so the
        // 64th distinct value still lands and the 65th does not.
        let mut around = numbers(0..63);
        around.extend([numbers(0..1)[0].clone(), numbers(0..1)[0].clone()]);
        around.extend(numbers(63..65));
        let kept = alt_codes(Some(&around.join(" ")), None);
        assert_eq!(kept, numbers(0..64));

        // The ten-digit maximum bounds the serialized array: 64 * 12 + 63
        // separators + 2 brackets.
        let ten: Vec<String> = (0..80)
            .map(|n| (1_000_000_000u64 + n).to_string())
            .collect();
        let kept = alt_codes(Some(&ten.join(" ")), None);
        assert_eq!(serde_json::to_string(&kept).unwrap().len(), 833);

        // HTML runs share the one collector.
        let html: String = numbers(0..70)
            .iter()
            .map(|n| format!("<p>{n}</p>"))
            .collect();
        assert_eq!(alt_codes(None, Some(&html)), numbers(0..64));

        // Independent bodies share it too, in source order.
        let text = numbers(0..40).join(" ");
        let html: String = numbers(40..80)
            .iter()
            .map(|n| format!("<td>{n}</td>"))
            .collect();
        assert_eq!(
            extract_codes(Some(&text), Some(&html), BodyOrder::TextFirst),
            numbers(0..64)
        );
        let mut html_first = numbers(40..80);
        html_first.extend(numbers(0..24));
        assert_eq!(
            extract_codes(Some(&text), Some(&html), BodyOrder::HtmlFirst),
            html_first
        );

        // The storage itself never outgrows the cap across a million
        // distinct candidates.
        let million: String = (0..1_048_576u32)
            .map(|n| format!("{} ", 1_000_000 + n))
            .collect();
        let mut set = CodeSet::default();
        set.scan(&million);
        set.scan(&million);
        assert_eq!(set.codes, numbers(0..64));
        assert!(
            set.codes.capacity() <= MAX_CODES,
            "{}",
            set.codes.capacity()
        );
    }

    // ---- issue #137: the MIME relation decides the preference ----

    #[test]
    fn only_alternatives_prefer_the_text_codes() {
        let html = "<p>Your code is 121212</p>";
        let footer = "-- list footer 2026";
        assert_eq!(
            extract_codes(Some(footer), Some(html), BodyOrder::HtmlFirst),
            vec!["121212", "2026"]
        );
        assert_eq!(
            extract_codes(Some(footer), Some(html), BodyOrder::TextFirst),
            vec!["2026", "121212"]
        );
        assert_eq!(
            extract_codes(Some(footer), Some(html), BodyOrder::Alternatives),
            vec!["2026"]
        );
        // Alternatives fall back to the HTML only when the text has none.
        assert_eq!(
            extract_codes(Some("no code"), Some(html), BodyOrder::Alternatives),
            vec!["121212"]
        );
        // The same code in both bodies is one code.
        assert_eq!(
            extract_codes(Some("121212"), Some(html), BodyOrder::TextFirst),
            vec!["121212"]
        );
        assert!(extract_codes(None, None, BodyOrder::HtmlFirst).is_empty());
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
            assert_eq!(alt_codes(None, Some(html)), want, "{html}");
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
            assert_eq!(alt_codes(None, Some(html)), want, "{html}");
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
            assert_eq!(alt_codes(None, Some(html)), want, "{html}");
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
            assert_eq!(alt_codes(None, Some(html)), want, "{html}");
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
            assert_eq!(alt_codes(None, Some(html)), vec!["1111"], "{html}");
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
            assert_eq!(alt_codes(None, Some(html)), want, "{html}");
        }
        assert!(
            alt_codes(Some("\u{661}\u{662}\u{663}\u{664}"), None).is_empty(),
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
                let (_, links) = links_with_stats(Some(&input), Some(&input));
                assert_eq!(
                    links.ordinary_bytes + links.quoted_bytes,
                    input.len() as u64,
                    "{input:?}"
                );
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
            alt_codes(None, Some("<a \u{e9}='x'>1111</a>")),
            vec!["1111"]
        );
    }
}
