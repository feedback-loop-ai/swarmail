//! The run console's HTML preview document (issue #374, proposed decision
//! 0135): the stored `Email.html`, read at request time, with `cid:`
//! references in `img src` resolved to `data:` URIs of retained inline
//! raster parts, and a limited display transformation that keeps a click or
//! an automatic feature from taking the frame anywhere. The document's own
//! response headers — CSP with a bare `sandbox`, `nosniff`, `no-referrer` —
//! are the security boundary for scripts, forms, plugins and frames; this
//! module is not a general sanitizer.
//!
//! Nothing here writes: the identity lookup re-parses `raw` under a bound to
//! learn which retained attachment entry carries which `Content-ID`, checks
//! the entry's type, size and SHA-256 against both the part and the retained
//! bytes, and emits only the retained bytes (0135 §2). The SMTP path, the
//! stored record and every extraction answer stay as ingest left them
//! (decisions 0001, 0005, 0110).

use crate::attachments::{candidates, sha256_hex};
use crate::extract::decode_refs;
use crate::model::{Attachment, Email};
use mail_parser::{MessageParser, MessagePart, MimeHeaders};
use std::collections::HashMap;
use tokio::sync::OwnedSemaphorePermit;

/// The raw source the identity lookup parses at most; above it the preview
/// still renders, with every `cid:` reference left unresolved.
pub(crate) const MAX_RAW_LOOKUP_BYTES: usize = 8 << 20;
/// The stored HTML a preview renders at most; above it the route answers 413.
pub(crate) const MAX_SOURCE_HTML_BYTES: usize = 2 << 20;
/// The decoded bytes of one image that may be inlined.
pub(crate) const MAX_IMAGE_BYTES: usize = 256 << 10;
/// The `data:` URI bytes one document may insert, charged per occurrence.
pub(crate) const MAX_INSERTED_BYTES: usize = 2 << 20;
/// The rendered document at most; above it the route answers 413.
pub(crate) const MAX_OUTPUT_BYTES: usize = 4 << 20;
/// Preview renders admitted at once per service; a third request is busy.
pub(crate) const PREVIEW_JOBS: usize = 2;

/// The inlinable types (0135 §3). SVG is refused: it is a document, with
/// its own links, styles and foreign-content parsing, not a raster.
const RASTER_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// The document for a mail without an HTML part.
pub(crate) const NO_HTML: &str = "<!doctype html><title>No HTML part</title><p style=\"font:14px system-ui,sans-serif;color:#666\">This message has no HTML part.</p>";

/// The document for HTML the transformation could not read reliably — a
/// tag still open at the end of the source. Never the unprocessed HTML.
pub(crate) const UNAVAILABLE: &str = "<!doctype html><title>Preview unavailable</title><p style=\"font:14px system-ui,sans-serif;color:#666\">Preview unavailable: this HTML could not be prepared for display safely. The Text tab and the raw source still show the mail.</p>";

/// What a render produced.
#[derive(Debug, PartialEq)]
pub(crate) enum Rendered {
    Document(String),
    /// The stored HTML or the rendered document exceeds its byte budget.
    TooLarge,
}

/// The blocking job a request submits. It owns the admission permit, so
/// the permit is released when the render actually ends — a caller that
/// disconnects meanwhile cannot free the slot early (0135 §4).
pub(crate) fn job(
    permit: OwnedSemaphorePermit,
    email: std::sync::Arc<Email>,
) -> impl FnOnce() -> Rendered + Send + 'static {
    move || {
        let rendered = render(&email);
        drop(permit);
        rendered
    }
}

/// The preview of one stored mail.
pub(crate) fn render(email: &Email) -> Rendered {
    let Some(html) = email.html.as_deref() else {
        return Rendered::Document(NO_HTML.into());
    };
    if html.len() > MAX_SOURCE_HTML_BYTES {
        return Rendered::TooLarge;
    }
    let images = if email.raw.len() <= MAX_RAW_LOOKUP_BYTES {
        inline_images(email)
    } else {
        HashMap::new()
    };
    match transform(html, &images) {
        None => Rendered::Document(UNAVAILABLE.into()),
        Some(out) if out.len() > MAX_OUTPUT_BYTES => Rendered::TooLarge,
        Some(out) => Rendered::Document(out),
    }
}

/// One inlinable image: its allowlisted type and the retained bytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Image<'e> {
    pub mime: &'static str,
    pub bytes: &'e [u8],
}

/// Every resolvable `Content-ID` of the outer message (0135 §§2–3). The
/// candidate ordinals are ingest's own walk, so ordinal `k` is the `k`-th
/// retained attachment entry. An ID carried by two outer parts — whatever
/// their types — is ambiguous and resolves to nothing; an unknown or
/// labels-only list, an unparseable source, or a part that fails a check
/// resolves to nothing too. Nothing is recovered from `raw`.
pub(crate) fn inline_images(email: &Email) -> HashMap<String, Image<'_>> {
    let Some(list) = email.attachments.as_ref().filter(|list| !list.labels_only) else {
        return HashMap::new();
    };
    let Some(message) = MessageParser::default().parse(&email.raw) else {
        return HashMap::new();
    };
    let mut ordinal = vec![None; message.parts.len()];
    for (k, (id, _, _)) in candidates(&message).enumerate() {
        ordinal[id] = Some(k);
    }
    let mut found: HashMap<String, Option<Image<'_>>> = HashMap::new();
    for (id, part) in message.parts.iter().enumerate() {
        let Some(cid) = part.content_id().and_then(normalize_cid) else {
            continue;
        };
        let image = ordinal[id]
            .and_then(|k| list.parts.get(k))
            .and_then(|entry| eligible(part, entry));
        found
            .entry(cid)
            .and_modify(|slot| *slot = None)
            .or_insert(image);
    }
    found
        .into_iter()
        .filter_map(|(cid, image)| Some((cid, image?)))
        .collect()
}

/// A `Content-ID` as compared (0135 §3): outer whitespace trimmed, one
/// angle-bracket pair removed; empty or control-bearing IDs are refused.
pub(crate) fn normalize_cid(raw: &str) -> Option<String> {
    let id = raw.trim();
    let id = id
        .strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(id);
    let usable = !id.is_empty() & !id.contains(char::is_control);
    usable.then(|| id.to_string())
}

/// The retained bytes of a candidate part, when it may be inlined: inline
/// or absent disposition, an allowlisted raster type, at most
/// [`MAX_IMAGE_BYTES`], and an entry whose type, size and SHA-256 agree
/// with both the part and the retained content.
fn eligible<'e>(part: &MessagePart<'_>, entry: &'e Attachment) -> Option<Image<'e>> {
    let attachment = part
        .content_disposition()
        .is_some_and(|disposition| disposition.ctype().eq_ignore_ascii_case("attachment"));
    if attachment {
        return None;
    }
    let header = part.content_type()?;
    let label = format!(
        "{}/{}",
        header.ctype().to_ascii_lowercase(),
        header.subtype()?.to_ascii_lowercase()
    );
    let mime = *RASTER_TYPES.iter().find(|mime| **mime == label)?;
    let content = entry.content.as_deref()?;
    let bytes = part.contents();
    let checks = [
        entry.content_type.as_deref() == Some(mime),
        bytes.len() <= MAX_IMAGE_BYTES,
        content == bytes,
        entry.size == content.len() as u64,
        entry.sha256 == sha256_hex(content),
    ];
    checks.iter().all(|ok| *ok).then_some(Image {
        mime,
        bytes: content,
    })
}

/// The `cid:` identifier an `img src` value names (0135 §3): character
/// references decoded once, outer ASCII whitespace trimmed, the scheme
/// matched ASCII case-insensitively, then the identifier percent-decoded
/// once, strictly, as UTF-8. `+` stays `+`.
pub(crate) fn cid_reference(value: &str) -> Option<String> {
    let decoded = decode_refs(value);
    let url = decoded.trim_matches(|c: char| c.is_ascii_whitespace());
    let scheme = url.get(..4)?;
    if !scheme.eq_ignore_ascii_case("cid:") {
        return None;
    }
    let bytes = &url.as_bytes()[4..];
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'%' {
            let high = hex(bytes.get(i + 1))?;
            let low = hex(bytes.get(i + 2))?;
            out.push(high << 4 | low);
            i += 3;
        } else {
            out.push(byte);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex(byte: Option<&u8>) -> Option<u8> {
    char::from(*byte?).to_digit(16).map(|digit| digit as u8)
}

// ---------- the display transformation ----------

/// HTML whitespace, with CR (the parser's preprocessing makes it LF).
fn space(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Whether a tag name ends before `byte`: whitespace, `/`, `>` or the end.
fn ends_tag_name(byte: Option<&u8>) -> bool {
    match byte {
        None | Some(b'/' | b'>') => true,
        Some(byte) => space(*byte),
    }
}

/// Elements whose content the tokenizer reads as text in the HTML namespace.
/// `noscript` is not among them: a sandboxed document without scripts parses
/// its content as markup.
const RAW_TEXT: [&str; 8] = [
    "script", "style", "xmp", "iframe", "noembed", "noframes", "title", "textarea",
];

/// Start tags that end foreign (SVG/MathML) content.
const BREAKOUT: [&str; 44] = [
    "b",
    "big",
    "blockquote",
    "body",
    "br",
    "center",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "hr",
    "i",
    "img",
    "li",
    "listing",
    "menu",
    "meta",
    "nobr",
    "ol",
    "p",
    "pre",
    "ruby",
    "s",
    "small",
    "span",
    "strong",
    "strike",
    "sub",
    "sup",
    "table",
    "tt",
    "u",
    "ul",
    "var",
];

/// Tag names that can navigate, refresh, prefetch or retarget a link. Any
/// `<name` of these that the scan does not read as a start tag — inside a
/// comment, raw text or an attribute value — is written `&lt;name`, which
/// means the same there and can never become a tag. A browser that splits
/// the source differently from this scan therefore still finds no such tag
/// that the transformation did not see.
const DANGEROUS: [&[u8]; 7] = [b"a", b"area", b"base", b"link", b"meta", b"set", b"animate"];

/// Attributes removed from every start tag: the link-click beacon, request
/// referrer overrides and SMIL's retargeting of `href`.
const STRIPPED: [&str; 3] = ["ping", "referrerpolicy", "attributename"];

/// Link relations that may stay: an ordinary stylesheet, subject to CSP.
const STYLESHEET_RELS: [&str; 2] = ["stylesheet", "alternate"];

/// What a removed tag leaves: an empty comment, so the text on either side
/// can never join into a new tag.
const REMOVED: &str = "<!---->";

/// The value of an attribute: the span with its quotes, and the text inside.
type Value = ((usize, usize), (usize, usize));

struct Attr {
    name: String,
    start: usize,
    end: usize,
    value: Option<Value>,
}

impl Attr {
    fn text<'h>(&self, html: &'h str) -> &'h str {
        self.value.map_or("", |(_, (from, to))| &html[from..to])
    }
}

struct Tag {
    name: String,
    end: usize,
    self_closing: bool,
    attrs: Vec<Attr>,
}

fn find(bytes: &[u8], from: usize, target: u8) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|byte| *byte == target)
        .map(|at| from + at)
}

/// Past the first `>` from `from`, or the end: a bogus comment, a DOCTYPE.
fn past_gt(bytes: &[u8], from: usize) -> usize {
    find(bytes, from, b'>').map_or(bytes.len(), |at| at + 1)
}

/// A tag whose name starts at `from`, read as the HTML tokenizer reads it:
/// names lowercased, quoted values running to their quote, an unquoted
/// value to whitespace or `>`. `None` when the source ends inside the tag.
fn parse_tag(html: &str, from: usize) -> Option<Tag> {
    let bytes = html.as_bytes();
    let mut i = from;
    while !ends_tag_name(bytes.get(i)) {
        i += 1;
    }
    let name = html[from..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        match *bytes.get(i)? {
            b'>' => {
                return Some(Tag {
                    name,
                    end: i + 1,
                    self_closing: false,
                    attrs,
                });
            }
            b'/' => {
                i += 1;
                if bytes.get(i) == Some(&b'>') {
                    return Some(Tag {
                        name,
                        end: i + 1,
                        self_closing: true,
                        attrs,
                    });
                }
            }
            byte if space(byte) => i += 1,
            _ => i = parse_attr(html, i, &mut attrs)?,
        }
    }
}

/// One attribute starting at `start` (its first character always belongs to
/// the name, a leading `=` included); the position after it.
fn parse_attr(html: &str, start: usize, attrs: &mut Vec<Attr>) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut i = start + 1;
    while !(ends_tag_name(bytes.get(i)) | (bytes.get(i) == Some(&b'='))) {
        i += 1;
    }
    let name_end = i;
    while bytes.get(i).is_some_and(|byte| space(*byte)) {
        i += 1;
    }
    let mut value = None;
    if bytes.get(i) == Some(&b'=') {
        i += 1;
        while bytes.get(i).is_some_and(|byte| space(*byte)) {
            i += 1;
        }
        let span = match *bytes.get(i)? {
            quote @ (b'"' | b'\'') => {
                let close = find(bytes, i + 1, quote)?;
                ((i, close + 1), (i + 1, close))
            }
            // `>` right after `=` is an empty value; it still ends the tag.
            _ => {
                let from = i;
                while !(bytes.get(i).is_none_or(|byte| space(*byte))
                    | (bytes.get(i) == Some(&b'>')))
                {
                    i += 1;
                }
                ((from, i), (from, i))
            }
        };
        i = span.0.1;
        value = Some(span);
    }
    attrs.push(Attr {
        name: html[start..name_end].to_ascii_lowercase(),
        start,
        end: value.map_or(name_end, |(whole, _)| whole.1),
        value,
    });
    Some(i)
}

/// Whether `<` followed by `after` opens one of the [`DANGEROUS`] tags.
/// Only the first eight bytes are read, so the check is constant-time.
fn dangerous(after: &[u8]) -> bool {
    let window = &after[..after.len().min(8)];
    let Some(end) = (0..window.len()).find(|at| ends_tag_name(window.get(*at))) else {
        return false;
    };
    DANGEROUS
        .iter()
        .any(|name| window[..end].eq_ignore_ascii_case(name))
}

/// The output, copied from the source with edits.
struct Writer<'h> {
    html: &'h str,
    out: String,
    at: usize,
}

impl Writer<'_> {
    /// Copy the source up to `end`, defanging every dangerous `<name`.
    fn copy_to(&mut self, end: usize) {
        let segment = &self.html[self.at..end];
        let mut last = 0;
        for (at, _) in segment.match_indices('<') {
            if dangerous(&self.html.as_bytes()[self.at + at + 1..]) {
                self.out.push_str(&segment[last..at]);
                self.out.push_str("&lt;");
                last = at + 1;
            }
        }
        self.out.push_str(&segment[last..]);
        self.at = end;
    }

    /// A start tag the scan read: its `<` is written as is.
    fn open_tag(&mut self, lt: usize) {
        self.copy_to(lt);
        self.out.push('<');
        self.at = lt + 1;
    }

    /// The source from `from` to `to` replaced by `with`.
    fn replace(&mut self, from: usize, to: usize, with: &str) {
        self.copy_to(from);
        self.out.push_str(with);
        self.at = to;
    }
}

/// The scan's state between tags.
struct Scan<'i, 'e> {
    images: &'i HashMap<String, Image<'e>>,
    /// Open SVG/MathML roots; above zero, the scan is in foreign content.
    foreign: usize,
    /// `data:` bytes inserted so far.
    inserted: usize,
}

/// The display transformation (0135 §5): `img src` CIDs resolved; `base`,
/// `meta` and every non-stylesheet `link` removed; anchors' and areas'
/// `href` removed; the [`STRIPPED`] attributes removed; and every dangerous
/// tag opener outside a tag defanged. `None` when a tag runs to the end of
/// the source: the caller then serves [`UNAVAILABLE`], never the source.
pub(crate) fn transform(html: &str, images: &HashMap<String, Image<'_>>) -> Option<String> {
    let bytes = html.as_bytes();
    let mut out = Writer {
        html,
        out: String::with_capacity(html.len()),
        at: 0,
    };
    let mut scan = Scan {
        images,
        foreign: 0,
        inserted: 0,
    };
    let mut i = 0;
    while let Some(lt) = find(bytes, i, b'<') {
        i = match bytes.get(lt + 1) {
            Some(b'!') => declaration(bytes, lt, scan.foreign > 0),
            Some(b'?') => past_gt(bytes, lt),
            Some(b'/') => scan.end_tag(html, lt)?,
            Some(byte) if byte.is_ascii_alphabetic() => scan.start_tag(html, lt, &mut out)?,
            _ => lt + 1,
        };
    }
    out.copy_to(html.len());
    Some(out.out)
}

/// The position after the first `pattern` at or past `from`, or the end.
fn past(bytes: &[u8], from: usize, pattern: &[u8]) -> Option<usize> {
    bytes[from..]
        .windows(pattern.len())
        .position(|window| window == pattern)
        .map(|at| from + at + pattern.len())
}

/// `<!` at `lt`: a comment (ending at `-->` or `--!>`, or at once for
/// `<!-->` and `<!--->`), a CDATA section in foreign content, or a DOCTYPE
/// or bogus comment ending at `>`.
fn declaration(bytes: &[u8], lt: usize, foreign: bool) -> usize {
    let rest = &bytes[lt + 2..];
    if rest.starts_with(b"--") {
        let body = lt + 4;
        if bytes[body..].starts_with(b">") {
            return body + 1;
        }
        if bytes[body..].starts_with(b"->") {
            return body + 2;
        }
        let close = past(bytes, body, b"-->");
        let bang = past(bytes, body, b"--!>");
        return close.into_iter().chain(bang).min().unwrap_or(bytes.len());
    }
    if foreign & rest.starts_with(b"[CDATA[") {
        return past(bytes, lt + 9, b"]]>").unwrap_or(bytes.len());
    }
    past_gt(bytes, lt)
}

/// Where raw text opened by `name` ends: the `<` of the first `</name`
/// followed by whitespace, `/` or `>`, ASCII case-insensitively.
fn raw_text_end(bytes: &[u8], from: usize, name: &str) -> usize {
    let mut at = from;
    while let Some(lt) = find(bytes, at, b'<') {
        if closes(&bytes[lt + 1..], name) {
            return lt;
        }
        at = lt + 1;
    }
    bytes.len()
}

/// Whether the bytes after a `<` are the end tag of `name`.
fn closes(tail: &[u8], name: &str) -> bool {
    let Some(rest) = tail.strip_prefix(b"/") else {
        return false;
    };
    let Some(word) = rest.get(..name.len()) else {
        return false;
    };
    word.eq_ignore_ascii_case(name.as_bytes()) & ends_tag_name(rest.get(name.len()))
}

impl Scan<'_, '_> {
    /// `</` at `lt`: an end tag (attributes and all), `</>` (nothing), or a
    /// bogus comment. An end tag of a foreign root leaves one level; `</p>`
    /// and `</br>` leave foreign content.
    fn end_tag(&mut self, html: &str, lt: usize) -> Option<usize> {
        let bytes = html.as_bytes();
        match bytes.get(lt + 2) {
            Some(byte) if byte.is_ascii_alphabetic() => {
                let tag = parse_tag(html, lt + 2)?;
                match tag.name.as_str() {
                    "svg" | "math" => self.foreign = self.foreign.saturating_sub(1),
                    "p" | "br" => self.foreign = 0,
                    _ => {}
                }
                Some(tag.end)
            }
            Some(b'>') => Some(lt + 3),
            _ => Some(past_gt(bytes, lt)),
        }
    }

    /// `<` and a letter at `lt`: a start tag, written with its edits.
    fn start_tag(&mut self, html: &str, lt: usize, out: &mut Writer<'_>) -> Option<usize> {
        let tag = parse_tag(html, lt + 1)?;
        if (self.foreign > 0) & breaks_out(&tag) {
            self.foreign = 0;
        }
        let html_namespace = self.foreign == 0;
        let removed = match tag.name.as_str() {
            "base" | "meta" => true,
            "link" => !stylesheet(&tag, html),
            _ => false,
        };
        if removed {
            out.replace(lt, tag.end, REMOVED);
        } else {
            out.open_tag(lt);
            self.edit(html, &tag, html_namespace, out);
        }
        if matches!(tag.name.as_str(), "svg" | "math") & !tag.self_closing {
            self.foreign += 1;
            return Some(tag.end);
        }
        if !html_namespace {
            return Some(tag.end);
        }
        if tag.name == "plaintext" {
            return Some(html.len());
        }
        if RAW_TEXT.contains(&tag.name.as_str()) {
            return Some(raw_text_end(html.as_bytes(), tag.end, &tag.name));
        }
        Some(tag.end)
    }

    /// The attribute edits of a kept start tag, in source order.
    fn edit(&mut self, html: &str, tag: &Tag, html_namespace: bool, out: &mut Writer<'_>) {
        let anchor = matches!(tag.name.as_str(), "a" | "area");
        let sources = tag.attrs.iter().filter(|attr| attr.name == "src").count();
        let rewrite = html_namespace & (tag.name == "img") & (sources == 1);
        for attr in &tag.attrs {
            let navigates = anchor & matches!(attr.name.as_str(), "href" | "xlink:href");
            if navigates | STRIPPED.contains(&attr.name.as_str()) {
                out.replace(attr.start, attr.end, "");
                continue;
            }
            let Some(((from, to), _)) = attr.value.filter(|_| rewrite & (attr.name == "src"))
            else {
                continue;
            };
            if let Some(uri) = self.data_uri(attr.text(html)) {
                out.replace(from, to, &format!("\"{uri}\""));
            }
        }
    }

    /// The `data:` URI for an `img src` value, when it names a resolvable
    /// CID and its bytes fit the remaining budget: first fit, charged per
    /// occurrence before anything is allocated.
    fn data_uri(&mut self, value: &str) -> Option<String> {
        let image = self.images.get(&cid_reference(value)?)?;
        let prefix = format!("data:{};base64,", image.mime);
        let cost = prefix.len() + image.bytes.len().div_ceil(3) * 4;
        self.inserted = self
            .inserted
            .checked_add(cost)
            .filter(|total| *total <= MAX_INSERTED_BYTES)?;
        use base64::Engine as _;
        Some(prefix + &base64::engine::general_purpose::STANDARD.encode(image.bytes))
    }
}

/// Whether a start tag in foreign content returns the parser to HTML.
fn breaks_out(tag: &Tag) -> bool {
    let font = (tag.name == "font")
        & tag
            .attrs
            .iter()
            .any(|attr| matches!(attr.name.as_str(), "color" | "face" | "size"));
    BREAKOUT.contains(&tag.name.as_str()) | font
}

/// A `link` whose first `rel` is an ordinary stylesheet: every token is
/// `stylesheet` or `alternate`, and one is `stylesheet`.
fn stylesheet(tag: &Tag, html: &str) -> bool {
    let rel = tag
        .attrs
        .iter()
        .find(|attr| attr.name == "rel")
        .map_or(String::new(), |attr| {
            decode_refs(attr.text(html)).to_ascii_lowercase()
        });
    let tokens: Vec<&str> = rel.split_ascii_whitespace().collect();
    tokens.contains(&"stylesheet") & tokens.iter().all(|token| STYLESHEET_RELS.contains(token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR fixture";

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn mail(raw: &str) -> Email {
        crate::smtp::build_email(raw.as_bytes(), "p", &["r@x.io".into()], "s@x.io".into())
    }

    /// A multipart/related mail: the HTML first, then each part as given.
    fn related(html: &str, parts: &[String]) -> String {
        let mut raw = String::from(
            "From: s@x.io\r\nTo: r@x.io\r\nSubject: p\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/related; boundary=b\r\n\r\n\
             --b\r\nContent-Type: text/html\r\n\r\n",
        );
        raw.push_str(html);
        raw.push_str("\r\n");
        for part in parts {
            raw.push_str("--b\r\n");
            raw.push_str(part);
            raw.push_str("\r\n");
        }
        raw.push_str("--b--\r\n");
        raw
    }

    /// A base64 part with the given headers.
    fn part(headers: &str, bytes: &[u8]) -> String {
        format!(
            "{headers}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}",
            b64(bytes)
        )
    }

    fn uri(mime: &str, bytes: &[u8]) -> String {
        format!("data:{mime};base64,{}", b64(bytes))
    }

    fn none() -> HashMap<String, Image<'static>> {
        HashMap::new()
    }

    fn plain(html: &str) -> String {
        transform(html, &none()).expect("a complete document")
    }

    #[test]
    fn every_allowed_raster_type_resolves_inline_or_without_disposition() {
        let jpeg: &[u8] = b"\xff\xd8\xff\xe0 jpeg";
        let gif: &[u8] = b"GIF89a gif";
        let webp: &[u8] = b"RIFF\0\0\0\0WEBPVP8 ";
        let email = mail(&related(
            "<img src=cid:png>",
            &[
                part(
                    "Content-Type: image/PNG; name=a.png\r\nContent-ID: <png>\r\nContent-Disposition: inline",
                    PNG,
                ),
                part("Content-Type: image/jpeg\r\nContent-ID: <jpeg>", jpeg),
                part("Content-Type: image/gif\r\nContent-ID: <gif>", gif),
                part("Content-Type: image/webp\r\nContent-ID: <webp>", webp),
            ],
        ));
        let images = inline_images(&email);
        assert_eq!(images.len(), 4, "{:?}", images.keys());
        for (cid, mime, bytes) in [
            ("png", "image/png", PNG),
            ("jpeg", "image/jpeg", jpeg),
            ("gif", "image/gif", gif),
            ("webp", "image/webp", webp),
        ] {
            assert_eq!(images[cid], Image { mime, bytes }, "{cid}");
        }
        // The bytes emitted are the retained entry's own, not a fresh decode.
        let retained = email.attachments.as_ref().unwrap().parts[0]
            .content
            .as_deref()
            .unwrap();
        assert!(std::ptr::eq(images["png"].bytes, retained));
    }

    #[test]
    fn svg_attachments_other_types_and_duplicates_never_resolve() {
        let email = mail(&related(
            "<p>",
            &[
                part("Content-Type: image/svg+xml\r\nContent-ID: <svg>", b"<svg/>"),
                part(
                    "Content-Type: image/png\r\nContent-ID: <att>\r\nContent-Disposition: ATTACHMENT; filename=a.png",
                    PNG,
                ),
                part("Content-Type: application/octet-stream\r\nContent-ID: <bin>", PNG),
                part("Content-Type: image\r\nContent-ID: <bare>", PNG),
                part("Content-Type: image/png\r\nContent-ID: <dup>", PNG),
                part("Content-Type: image/png\r\nContent-ID: <dup>", PNG),
                // The same ID on a part that is no candidate at all still
                // makes the ID ambiguous.
                part("Content-Type: image/png\r\nContent-ID: <twice>", PNG),
                "Content-Type: text/plain\r\nContent-ID: <twice>\r\n\r\nhello".into(),
                // A calendar-shaped candidate without a content type.
                "Content-Disposition: inline; filename=a.ics\r\nContent-ID: <ics>\r\n\r\nBEGIN:VCALENDAR".into(),
                part("Content-Type: image/png\r\nContent-ID: <ok>", PNG),
            ],
        ));
        let images = inline_images(&email);
        assert_eq!(images.keys().collect::<Vec<_>>(), ["ok"]);
    }

    #[test]
    fn calendar_candidates_keep_the_ordinals_aligned() {
        // A bare text/calendar body is a candidate before the image, so the
        // image is entry 1 — reading it as entry 0 would fail its checks.
        let email = mail(&related(
            "<p>",
            &[
                "Content-Type: text/calendar\r\n\r\nBEGIN:VCALENDAR\r\nEND:VCALENDAR".into(),
                part("Content-Type: image/png\r\nContent-ID: <after>", PNG),
            ],
        ));
        assert_eq!(email.attachments.as_ref().unwrap().total, 2);
        assert_eq!(inline_images(&email)["after"].bytes, PNG);
    }

    #[test]
    fn every_entry_check_fails_closed() {
        let raw = related(
            "<p>",
            &[part("Content-Type: image/png\r\nContent-ID: <x>", PNG)],
        );
        let resolves = |edit: &dyn Fn(&mut Email)| {
            let mut email = mail(&raw);
            edit(&mut email);
            inline_images(&email).contains_key("x")
        };
        assert!(resolves(&|_| {}));
        fn entry(email: &mut Email) -> &mut Attachment {
            &mut email.attachments.as_mut().unwrap().parts[0]
        }
        assert!(!resolves(
            &|e| entry(e).content_type = Some("image/gif".into())
        ));
        assert!(!resolves(&|e| entry(e).size += 1));
        assert!(!resolves(&|e| entry(e).sha256 = sha256_hex(b"other")));
        assert!(!resolves(&|e| entry(e).content = Some(b"other".to_vec())));
        assert!(!resolves(&|e| {
            entry(e).content = None;
            entry(e).stored = false;
        }));
        assert!(!resolves(&|e| e
            .attachments
            .as_mut()
            .unwrap()
            .parts
            .clear()));
        assert!(!resolves(&|e| e
            .attachments
            .as_mut()
            .unwrap()
            .labels_only = true));
        assert!(!resolves(&|e| e.attachments = None));
        assert!(!resolves(&|e| e.raw.clear()));
    }

    #[test]
    fn the_image_cap_is_inclusive() {
        for (size, resolves) in [(MAX_IMAGE_BYTES, true), (MAX_IMAGE_BYTES + 1, false)] {
            let bytes = vec![7u8; size];
            let email = mail(&related(
                "<p>",
                &[part("Content-Type: image/gif\r\nContent-ID: <big>", &bytes)],
            ));
            assert_eq!(
                inline_images(&email).contains_key("big"),
                resolves,
                "{size}"
            );
        }
    }

    #[test]
    fn content_ids_normalize_once_and_refuse_empty_or_control() {
        assert_eq!(normalize_cid(" <a@b> ").as_deref(), Some("a@b"));
        assert_eq!(normalize_cid("a@b").as_deref(), Some("a@b"));
        assert_eq!(normalize_cid("<<a>>").as_deref(), Some("<a>"));
        assert_eq!(normalize_cid("<a").as_deref(), Some("<a"));
        assert_eq!(normalize_cid("<>"), None);
        assert_eq!(normalize_cid("  "), None);
        assert_eq!(normalize_cid("a\u{1}b"), None);
    }

    #[test]
    fn cid_references_decode_entities_then_percent_once() {
        for (value, id) in [
            ("cid:a", Some("a")),
            ("CiD:a", Some("a")),
            (" \tcid:a \n", Some("a")),
            ("cid:a%20b", Some("a b")),
            ("cid:a%2fb%2F", Some("a/b/")),
            ("cid:a+b", Some("a+b")),
            ("cid:a%2541", Some("a%41")),
            ("&#99;id:&amp;x", Some("&x")),
            ("cid:%C3%A9", Some("é")),
            ("cid:", Some("")),
            ("cid:%zz", None),
            ("cid:%4", None),
            ("cid:%", None),
            ("cid:%ff", None),
            ("cid:%+1", None),
            ("https://x/a.png", None),
            ("ci", None),
            ("abcé", None),
            ("éabc", None),
        ] {
            assert_eq!(cid_reference(value).as_deref(), id, "{value:?}");
        }
    }

    fn images(entries: &[(&str, &'static str, &'static [u8])]) -> HashMap<String, Image<'static>> {
        entries
            .iter()
            .map(|(cid, mime, bytes)| (cid.to_string(), Image { mime, bytes }))
            .collect()
    }

    #[test]
    fn img_src_cids_become_data_uris_in_every_value_form() {
        let map = images(&[("a", "image/png", PNG), ("a b", "image/gif", b"GIF")]);
        let png = uri("image/png", PNG);
        let gif = uri("image/gif", b"GIF");
        for (html, expected) in [
            ("<img src=\"cid:a\">", format!("<img src=\"{png}\">")),
            (
                "<IMG SRC='cid:a' alt=x>",
                format!("<IMG SRC=\"{png}\" alt=x>"),
            ),
            ("<img src=cid:a>", format!("<img src=\"{png}\">")),
            ("<img src = cid:a />", format!("<img src = \"{png}\" />")),
            ("<img src=cid:a/>", "<img src=cid:a/>".into()),
            (
                "<img alt=\"\" src=\"cid:a%20b\"/>",
                format!("<img alt=\"\" src=\"{gif}\"/>"),
            ),
            ("<img src=\"&#99;id:a\">", format!("<img src=\"{png}\">")),
            // Unresolvable, duplicated, valueless and non-img sources stay.
            ("<img src=\"cid:z\">", "<img src=\"cid:z\">".into()),
            (
                "<img src=cid:a src=cid:a>",
                "<img src=cid:a src=cid:a>".into(),
            ),
            ("<img src>", "<img src>".into()),
            (
                "<img srcset=\"cid:a 1x\">",
                "<img srcset=\"cid:a 1x\">".into(),
            ),
            ("<input src=cid:a>", "<input src=cid:a>".into()),
            (
                "<p style=\"background:url(cid:a)\">",
                "<p style=\"background:url(cid:a)\">".into(),
            ),
            (
                "<svg><image href=cid:a></svg>",
                "<svg><image href=cid:a></svg>".into(),
            ),
            (
                "<!-- <img src=cid:a> -->",
                "<!-- <img src=cid:a> -->".into(),
            ),
            (
                "<style><img src=cid:a></style>",
                "<style><img src=cid:a></style>".into(),
            ),
            ("cid:a", "cid:a".into()),
        ] {
            assert_eq!(
                transform(html, &map).as_deref(),
                Some(expected.as_str()),
                "{html}"
            );
        }
        // `<img>` ends foreign content, so it is HTML again and resolves.
        assert_eq!(
            transform("<svg><img src=cid:a>", &map).unwrap(),
            format!("<svg><img src=\"{png}\">")
        );
    }

    #[test]
    fn navigation_refresh_and_hints_are_removed() {
        for (html, expected) in [
            ("<a href=\"https://t/x\" title=t>x</a>", "<a  title=t>x</a>"),
            ("<a title=t href=https://t/x>x</a>", "<a title=t >x</a>"),
            ("<A HREF='x'title=t>", "<A title=t>"),
            ("<a/href=x>", "<a/>"),
            ("<a href>", "<a >"),
            ("<a href = x ping=y>", "<a  >"),
            ("<area shape=rect href=x>", "<area shape=rect >"),
            (
                "<svg><a xlink:href=x><text>t</text></a></svg>",
                "<svg><a ><text>t</text></a></svg>",
            ),
            (
                "<svg><a href=x><set attributeName=href to=y /></a></svg>",
                "<svg><a ><set  to=y /></a></svg>",
            ),
            ("<img referrerpolicy=unsafe-url src=x>", "<img  src=x>"),
            ("<p data-href=x href=y>", "<p data-href=x href=y>"),
            ("<base href=https://t/>x", "<!---->x"),
            (
                "<meta http-equiv=refresh content=\"0;url=https://t/\">",
                "<!---->",
            ),
            ("<meta charset=utf-8>", "<!---->"),
            ("<link rel=prefetch href=https://t/>", "<!---->"),
            ("<link rel=dns-prefetch href=//t>", "<!---->"),
            ("<link href=https://t/>", "<!---->"),
            ("<link rel=\"stylesheet preload\" href=s>", "<!---->"),
            ("<link rel=alternate href=s>", "<!---->"),
            (
                "<link rel=stylesheet href=https://t/s.css>",
                "<link rel=stylesheet href=https://t/s.css>",
            ),
            (
                "<link rel=\"Alternate StyleSheet\" href=s>",
                "<link rel=\"Alternate StyleSheet\" href=s>",
            ),
            (
                "<link rel=\"&#115;tylesheet\" href=s>",
                "<link rel=\"&#115;tylesheet\" href=s>",
            ),
            // A removed tag can never join its neighbours into a new one.
            ("<<base>a href=x>", "<<!---->a href=x>"),
        ] {
            assert_eq!(plain(html), expected, "{html}");
        }
    }

    #[test]
    fn dangerous_openers_outside_tags_are_defanged() {
        for (html, expected) in [
            ("<!-- <a href=x> -->", "<!-- &lt;a href=x> -->"),
            (
                "<!-- <META http-equiv=refresh> -->",
                "<!-- &lt;META http-equiv=refresh> -->",
            ),
            (
                "<!-- <animate/> <set> <area\t> <base> <link\n> -->",
                "<!-- &lt;animate/> &lt;set> &lt;area\t> &lt;base> &lt;link\n> -->",
            ),
            ("<!-- <ab> <abbr> <a -->", "<!-- <ab> <abbr> &lt;a -->"),
            ("<!-- <ab> <abbr> <a", "<!-- <ab> <abbr> <a"),
            ("<p title=\"<a href=x>\">", "<p title=\"&lt;a href=x>\">"),
            ("<b <a x>", "<b &lt;a x>"),
            ("<b <a=1>", "<b <a=1>"),
            ("<style>a{}</style><a href=y>", "<style>a{}</style><a >"),
            (
                "<style><a href=x></style><a href=y>",
                "<style>&lt;a href=x></style><a >",
            ),
            (
                "<script>if(a<b)x='<a href=\"'</script>z",
                "<script>if(a<b)x='&lt;a href=\"'</script>z",
            ),
            (
                "<title>t <meta x></TITLE >",
                "<title>t &lt;meta x></TITLE >",
            ),
            (
                "<textarea></textareax><a></textarea/>",
                "<textarea></textareax>&lt;a></textarea/>",
            ),
            ("<xmp><a>", "<xmp>&lt;a>"),
            ("<style>a</sty", "<style>a</sty"),
            (
                "<plaintext></plaintext><a href=x>",
                "<plaintext></plaintext>&lt;a href=x>",
            ),
            (
                "<noscript><a href=x>n</a></noscript>",
                "<noscript><a >n</a></noscript>",
            ),
            ("<<a href=x>", "<<a >"),
            ("x < a href=x", "x < a href=x"),
            ("1<2 <", "1<2 <"),
        ] {
            assert_eq!(plain(html), expected, "{html}");
        }
    }

    #[test]
    fn comments_declarations_and_end_tags_end_where_the_tokenizer_ends_them() {
        for (html, expected) in [
            ("<!--><a href=x>", "<!--><a >"),
            ("<!---><a href=x>", "<!---><a >"),
            ("<!----><a href=x>", "<!----><a >"),
            ("<!-- x --!><a href=x>", "<!-- x --!><a >"),
            ("<!-- x ---><a href=x>", "<!-- x ---><a >"),
            ("<!-- open <a href=x>", "<!-- open &lt;a href=x>"),
            ("<!DOCTYPE html><a href=x>", "<!DOCTYPE html><a >"),
            (
                "<![CDATA[ <a href=x> ]]><a href=y>",
                "<![CDATA[ &lt;a href=x> ]]><a >",
            ),
            (
                "<svg><![CDATA[ > <a href=x> ]]></svg><a href=y>",
                "<svg><![CDATA[ > &lt;a href=x> ]]></svg><a >",
            ),
            ("<svg><![CDATA[ <a href=x>", "<svg><![CDATA[ &lt;a href=x>"),
            ("<?xml <a href=x>?><a href=y>", "<?xml &lt;a href=x>?><a >"),
            ("</a title=\">\"><a href=x>", "</a title=\">\"><a >"),
            ("</><a href=x>", "</><a >"),
            ("</ <a href=x>x><a href=y>", "</ &lt;a href=x>x><a >"),
            ("x</", "x</"),
        ] {
            assert_eq!(plain(html), expected, "{html}");
        }
    }

    #[test]
    fn foreign_content_is_tracked_for_raw_text() {
        for (html, expected) in [
            // In SVG, <style> holds markup, so the anchor is a real tag.
            (
                "<svg><style><a href=x></style></svg>",
                "<svg><style><a ></style></svg>",
            ),
            (
                "<svg><svg></svg><style><a href=x></style>",
                "<svg><svg></svg><style><a ></style>",
            ),
            (
                "<math><style><a href=x></style></math>",
                "<math><style><a ></style></math>",
            ),
            // Back in HTML after the root closes, after a breakout start tag
            // (font only with color, face or size), or after </p> and </br>.
            (
                "<svg></svg><style><a href=x></style>",
                "<svg></svg><style>&lt;a href=x></style>",
            ),
            ("<svg><b><style><a href=x>", "<svg><b><style>&lt;a href=x>"),
            (
                "<svg><font color=red><style><a href=x>",
                "<svg><font color=red><style>&lt;a href=x>",
            ),
            (
                "<svg><font><style><a href=x></style>",
                "<svg><font><style><a ></style>",
            ),
            (
                "<svg></p><style><a href=x>",
                "<svg></p><style>&lt;a href=x>",
            ),
            (
                "<svg></br><style><a href=x>",
                "<svg></br><style>&lt;a href=x>",
            ),
            ("</svg><style><a href=x>", "</svg><style>&lt;a href=x>"),
            // A self-closing root never opens foreign content.
            ("<svg/><style><a href=x>", "<svg/><style>&lt;a href=x>"),
        ] {
            assert_eq!(plain(html), expected, "{html}");
        }
    }

    #[test]
    fn a_tag_open_at_the_end_is_unavailable_not_passed_through() {
        for html in [
            "<a href=x",
            "<a href='x",
            "<a href=\"x>",
            "<a ",
            "<a/",
            "<a href=",
            "<a href ",
            "<img src=cid:a",
            "</a",
            "</a title='>",
            "x<b",
        ] {
            assert_eq!(transform(html, &none()), None, "{html}");
        }
    }

    /// 7 references of a and one of b insert exactly the budget: every one
    /// is inlined. One byte more and b is left as sent, while a later small
    /// image still fits (first fit, charged per occurrence).
    #[test]
    fn the_inserted_budget_is_inclusive_and_first_fit() {
        let a = vec![1u8; 3 * 65531];
        let tiny = [3u8];
        let html = format!(
            "{}<img src=cid:b><img src=cid:c>",
            "<img src=cid:a>".repeat(7)
        );
        for (b_size, b_inlined, c_inlined) in [(3 * 65527, true, false), (3 * 65528, false, true)] {
            let b = vec![2u8; b_size];
            let map: HashMap<String, Image<'_>> = [("a", &a[..]), ("b", &b[..]), ("c", &tiny[..])]
                .into_iter()
                .map(|(cid, bytes)| {
                    (
                        cid.to_string(),
                        Image {
                            mime: "image/png",
                            bytes,
                        },
                    )
                })
                .collect();
            let cost = |bytes: &[u8]| uri("image/png", bytes).len();
            assert_eq!(7 * cost(&a) + cost(&b) > MAX_INSERTED_BYTES, !b_inlined);
            let out = transform(&html, &map).unwrap();
            assert_eq!(out.matches(&uri("image/png", &a)).count(), 7);
            assert_eq!(out.contains(&uri("image/png", &b)), b_inlined, "{b_size}");
            assert_eq!(out.contains("<img src=cid:b>"), !b_inlined, "{b_size}");
            assert_eq!(
                out.contains(&uri("image/png", &tiny)),
                c_inlined,
                "{b_size}"
            );
        }
    }

    fn png_mail(size: usize) -> Email {
        let bytes = vec![9u8; size];
        mail(&related(
            "<img src=cid:a>",
            &[part("Content-Type: image/png\r\nContent-ID: <a>", &bytes)],
        ))
    }

    #[test]
    fn render_answers_every_outcome() {
        let mut email = png_mail(4);
        let inlined = format!("<img src=\"{}\">", uri("image/png", &[9u8; 4]));
        assert_eq!(render(&email), Rendered::Document(inlined.clone()));

        // The raw lookup cap is inclusive; above it CIDs stay unresolved.
        let len = email.raw.len();
        email.raw.resize(MAX_RAW_LOOKUP_BYTES, b' ');
        assert_eq!(render(&email), Rendered::Document(inlined));
        email.raw.push(b' ');
        assert_eq!(render(&email), Rendered::Document("<img src=cid:a>".into()));
        email.raw.truncate(len);

        // The source cap is inclusive and is checked before any parse.
        email.html = Some("x".repeat(MAX_SOURCE_HTML_BYTES));
        assert_eq!(
            render(&email),
            Rendered::Document("x".repeat(MAX_SOURCE_HTML_BYTES))
        );
        email.html = Some(format!("<{}", "x".repeat(MAX_SOURCE_HTML_BYTES)));
        assert_eq!(render(&email), Rendered::TooLarge);

        email.html = Some("<p>ok<a href=".into());
        assert_eq!(render(&email), Rendered::Document(UNAVAILABLE.into()));
        email.html = None;
        assert_eq!(render(&email), Rendered::Document(NO_HTML.into()));
    }

    /// A document that grows past the output cap: a source just under 2 MiB
    /// of `<base>` tags (each written as a seven-byte marker) and seven
    /// inlined 192 KiB images.
    #[test]
    fn the_output_cap_refuses_a_grown_document() {
        let mut email = png_mail(3 * 65531);
        let images = "<img src=cid:a>".repeat(7);
        let count = (MAX_SOURCE_HTML_BYTES - images.len()) / 6;
        let bases = "<base>".repeat(count);
        email.html = Some(bases.clone() + &images);
        assert_eq!(render(&email), Rendered::TooLarge);
        email.html = Some(bases);
        assert_eq!(render(&email), Rendered::Document(REMOVED.repeat(count)));
        assert!(REMOVED.len() * count <= MAX_OUTPUT_BYTES);
    }

    #[tokio::test]
    async fn the_job_owns_its_permit_until_the_render_ends() {
        use std::sync::Arc;
        use tokio::sync::Semaphore;
        let permits = Arc::new(Semaphore::new(PREVIEW_JOBS));
        let email = Arc::new(png_mail(4));
        let run = job(permits.clone().try_acquire_owned().unwrap(), email.clone());
        assert_eq!(
            permits.available_permits(),
            PREVIEW_JOBS - 1,
            "held before it runs"
        );
        assert!(matches!(run(), Rendered::Document(_)));
        assert_eq!(
            permits.available_permits(),
            PREVIEW_JOBS,
            "released when it ends"
        );

        // A caller that goes away drops only the handle: the job runs to its
        // end on the blocking pool and only then gives its permit back.
        let run = job(permits.clone().try_acquire_owned().unwrap(), email);
        drop(tokio::task::spawn_blocking(run));
        let all = permits.acquire_many(PREVIEW_JOBS as u32).await.unwrap();
        drop(all);
    }
}
