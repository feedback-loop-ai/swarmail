//! Finite source-aware risk heuristics, not a renderer. Data is authored locally
//! from named guidance; no client measurement or runtime networking is implied.
use super::{AnalysisError, Collector, Finding, Location, Severity};
use crate::extract::decode_refs_counted;
use mail_parser::{MessageParser, MimeHeaders, PartType};
use serde::Deserialize;
use std::{collections::HashMap, sync::LazyLock};

const HTML_BYTES: usize = 4_194_304;
const NESTING: usize = 32;
const DATA: &str = include_str!("html-rules.json");
static CATALOGUE: LazyLock<Result<Catalogue, AnalysisError>> = LazyLock::new(|| catalogue(DATA));

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogue {
    rule_set_id: String,
    provenance: String,
    rules: Vec<Rule>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    id: String,
    severity: Severity,
    selector: Selector,
    fix_hint: String,
    clients: Vec<String>,
    source: Source,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    title: String,
    url: String,
    summary: String,
    basis_type: String,
    basis_date: String,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Selector {
    Css {
        property: String,
        values: Vec<String>,
    },
    Element {
        name: String,
    },
    MissingAttribute {
        element: String,
        attribute: String,
    },
    ImageOnly,
    DecodedBytes {
        greater_than: usize,
    },
    RemoteSmallImage {
        min_dimension: u32,
        max_dimension: u32,
    },
}
fn name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
}
fn catalogue(data: &str) -> Result<Catalogue, AnalysisError> {
    let c: Catalogue = serde_json::from_str(data).map_err(|_| AnalysisError::Failed)?;
    if c.rule_set_id != super::RULE_SET_ID
        || c.provenance.trim().is_empty()
        || c.rules.is_empty()
        || c.rules.len() > 32
    {
        return Err(AnalysisError::Failed);
    }
    let mut ids = std::collections::HashSet::new();
    for r in &c.rules {
        if r.id.trim().is_empty()
            || !ids.insert(&r.id)
            || r.fix_hint.trim().is_empty()
            || r.clients.is_empty()
            || r.clients.iter().any(|s| s.trim().is_empty())
            || r.source.title.trim().is_empty()
            || !r.source.url.starts_with("https://")
            || r.source.summary.trim().is_empty()
            || r.source.basis_type != "authored_heuristic"
            || r.source.basis_date != "2026-10-26"
        {
            return Err(AnalysisError::Failed);
        }
        let valid = match &r.selector {
            Selector::Css { property, values } => name(property) && values.iter().all(|s| name(s)),
            Selector::Element { name: n } => name(n),
            Selector::MissingAttribute { element, attribute } => name(element) && name(attribute),
            Selector::DecodedBytes { greater_than } => *greater_than > 0,
            Selector::RemoteSmallImage {
                min_dimension,
                max_dimension,
            } => min_dimension <= max_dimension,
            Selector::ImageOnly => true,
        };
        if !valid {
            return Err(AnalysisError::Failed);
        }
    }
    Ok(c)
}

pub(super) fn collect(raw: &[u8], out: &mut Collector) -> Result<(), AnalysisError> {
    collect_catalogue(raw, &CATALOGUE, out)
}
fn collect_catalogue(
    raw: &[u8],
    c: &Result<Catalogue, AnalysisError>,
    out: &mut Collector,
) -> Result<(), AnalysisError> {
    collect_with(raw, c.as_ref().map_err(|e| *e)?, out)
}
fn collect_with(raw: &[u8], c: &Catalogue, out: &mut Collector) -> Result<(), AnalysisError> {
    // Raw size, part count and nesting depth are already bounded by the
    // engine's own admission budgets, so this walk relies on them.
    if raw.is_empty() {
        return Ok(());
    }
    let message = MessageParser::default()
        .parse(raw)
        .ok_or(AnalysisError::Failed)?;
    let mut stack = vec![(0usize, Vec::new(), false)];
    let mut eligible = Vec::new();
    let mut bytes = 0;
    while let Some((id, path, excluded)) = stack.pop() {
        let part = &message.parts[id];
        let excluded = excluded
            || part.attachment_name().is_some()
            || part
                .content_disposition()
                .is_some_and(|d| d.ctype().eq_ignore_ascii_case("attachment"));
        if let PartType::Multipart(children) = &part.body {
            for (n, child) in children.iter().enumerate().rev() {
                let mut path = path.clone();
                path.push(n as u64);
                stack.push((*child as usize, path, excluded));
            }
        } else if !excluded && part.is_content_type("text", "html") {
            if part.is_encoding_problem {
                return Err(AnalysisError::Failed);
            }
            let body = part.text_contents().ok_or(AnalysisError::Failed)?;
            bytes += body.len();
            if bytes > HTML_BYTES {
                return Err(AnalysisError::Failed);
            }
            eligible.push((path, body));
        }
    }
    // Validate aggregate decoded size before any lexical analysis/contribution.
    for (path, body) in eligible {
        let mut scan = Scan::new(c);
        scan.html(body)?;
        for (rule, matched) in c.rules.iter().zip(scan.matched) {
            if matched {
                out.push(Finding {
                    rule_id: rule.id.clone(),
                    severity: rule.severity,
                    location: Location::MimePart { path: path.clone() },
                    fix_hint: rule.fix_hint.clone(),
                })?;
            }
        }
    }
    Ok(())
}

/// Cursor displacement is a progress invariant, never a work estimate.
/// Inspections charge reads/probes at their execution site and conservative
/// length-bounded library operations per invocation (including failed helpers).
/// The census and fixed bound are in specs/html-review-repair/implementation.md.
#[derive(Default)]
struct Work {
    html: usize,
    css: usize,
    inspections: u64,
}
impl Work {
    fn byte(&mut self, b: &[u8], i: usize) -> Option<u8> {
        self.inspections += 1;
        b.get(i).copied()
    }
    fn prefix(&mut self, b: &[u8], i: usize, prefix: &[u8]) -> bool {
        self.inspections += prefix.len() as u64 + 1;
        b[i..].starts_with(prefix)
    }
    fn charge(&mut self, n: usize) {
        self.inspections += n as u64;
    }
    fn decode(&mut self, s: &str) -> String {
        decode_refs_counted(s, &mut self.inspections)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Rules,
    Declarations,
    Component,
}
struct Frame {
    close: u8,
    context: Context,
}
struct Scan<'a> {
    catalogue: &'a Catalogue,
    matched: Vec<bool>,
    work: Work,
}
impl<'a> Scan<'a> {
    fn new(catalogue: &'a Catalogue) -> Self {
        Self {
            catalogue,
            matched: vec![false; catalogue.rules.len()],
            work: Work::default(),
        }
    }
    fn declaration(&mut self, source: &str) {
        // Split, trim, !important, seven selectors and their finite value lists.
        // Segments are disjoint; opaque components contribute one sentinel.
        self.work.charge(64 * (source.len() + 1));
        let Some((property, value)) = source.split_once(':') else {
            return;
        };
        let property = property.trim();
        let mut value = value.trim();
        if let Some((body, suffix)) = value.rsplit_once('!')
            && suffix.trim().eq_ignore_ascii_case("important")
        {
            value = body.trim();
        }
        for (r, hit) in self.catalogue.rules.iter().zip(&mut self.matched) {
            if let Selector::Css {
                property: p,
                values,
            } = &r.selector
            {
                *hit |= property.eq_ignore_ascii_case(p)
                    && !value.is_empty()
                    && (values.is_empty() || values.iter().any(|v| value.eq_ignore_ascii_case(v)));
            }
        }
    }
    fn css(&mut self, source: &str, inline: bool) -> Result<(), AnalysisError> {
        let b = source.as_bytes();
        let mut i = 0;
        let mut segment = String::new();
        let mut frames: Vec<Frame> = Vec::new();
        let root = if inline {
            Context::Declarations
        } else {
            Context::Rules
        };
        let mut in_value = false;
        while i < b.len() {
            let start = i;
            self.work.charge(8); // repeated delimiter reads, sentinel/copy and frame operations
            let context = frames.last().map_or(root, |f| f.context);
            let component = context == Context::Component;
            match self.work.byte(b, i).unwrap() {
                b'/' if self.work.byte(b, i + 1) == Some(b'*') => {
                    i = terminated(b, i + 2, b"*/", &mut self.work)?;
                    if !component {
                        segment.push(' ');
                    }
                }
                b'\'' | b'"' => {
                    i = string_end(b, i, &mut self.work)?;
                    if !component {
                        segment.push('\0');
                    }
                }
                b'\\' => {
                    let end = escaped_end(source, i, &mut self.work);
                    if !component {
                        self.work.charge(end - i);
                        segment.push_str(&source[i..end]);
                    }
                    i = end;
                }
                b'u' | b'U' if url_start(b, i, &mut self.work) => {
                    if frames.len() == NESTING {
                        return Err(AnalysisError::Failed);
                    }
                    if let Some(end) = url_end(source, i + 4, &mut self.work)? {
                        if !component {
                            segment.push('\0');
                        }
                        i = end;
                    } else {
                        // Quoted url uses the ordinary function/string grammar.
                        if !component {
                            segment.push(b[i] as char);
                        }
                        i += 1;
                    }
                }
                b'(' | b'[' | b'{' if component || in_value || b[i] != b'{' => {
                    if frames.len() == NESTING {
                        return Err(AnalysisError::Failed);
                    }
                    let close = match b[i] {
                        b'(' => b')',
                        b'[' => b']',
                        _ => b'}',
                    };
                    frames.push(Frame {
                        close,
                        context: Context::Component,
                    });
                    if !component {
                        segment.push('\0');
                    }
                    i += 1;
                }
                b'{' => {
                    if frames.len() == NESTING {
                        return Err(AnalysisError::Failed);
                    }
                    self.work.charge(3 * segment.len() + 7);
                    let container = segment
                        .trim_start()
                        .to_ascii_lowercase()
                        .starts_with("@media");
                    frames.push(Frame {
                        close: b'}',
                        context: if container {
                            Context::Rules
                        } else {
                            Context::Declarations
                        },
                    });
                    segment.clear();
                    in_value = false;
                    i += 1;
                }
                b')' | b']' | b'}' => {
                    let frame = frames.pop().ok_or(AnalysisError::Failed)?;
                    if frame.close != b[i] {
                        return Err(AnalysisError::Failed);
                    }
                    if frame.context != Context::Component {
                        if frame.context == Context::Declarations {
                            self.declaration(&segment);
                        }
                        segment.clear();
                        in_value = false;
                    }
                    i += 1;
                }
                b';' if !component => {
                    if context == Context::Declarations {
                        self.declaration(&segment);
                    }
                    segment.clear();
                    in_value = false;
                    i += 1;
                }
                _ => {
                    if !component {
                        if b[i] == b':' && context == Context::Declarations {
                            in_value = true;
                        }
                        let c = next_char(source, i, &mut self.work);
                        segment.push(c);
                        i += c.len_utf8();
                    } else {
                        i += 1;
                    }
                }
            }
            self.work.css += i - start;
        }
        if !frames.is_empty() {
            return Err(AnalysisError::Failed);
        }
        if inline {
            self.declaration(&segment);
        }
        Ok(())
    }
    fn html(&mut self, source: &str) -> Result<(), AnalysisError> {
        let b = source.as_bytes();
        let mut i = 0;
        let mut suppressed: Vec<String> = Vec::new();
        let mut image = false;
        let mut text = false;
        while i < b.len() {
            let start = i;
            if self.work.byte(b, i) != Some(b'<')
                || !self
                    .work
                    .byte(b, i + 1)
                    .is_some_and(|v| v.is_ascii_alphabetic() || matches!(v, b'/' | b'!' | b'?'))
            {
                i += 1;
                while i < b.len() && self.work.byte(b, i) != Some(b'<') {
                    i += 1;
                }
                if suppressed.is_empty() {
                    let decoded = self.work.decode(&source[start..i]);
                    self.work.charge(decoded.len() + 1);
                    text |= decoded.chars().any(|c| !c.is_whitespace());
                }
            } else if self.work.prefix(b, i, b"<!--") {
                i = terminated(b, i + 4, b"-->", &mut self.work)?;
            } else if self
                .work
                .byte(b, i + 1)
                .is_some_and(|v| matches!(v, b'!' | b'?'))
            {
                i = inert_end(b, i + 2, &mut self.work)?;
            } else {
                let tag = tag(source, i, &mut self.work)?;
                // Fixed rule comparisons, <=32 suppression entries, stack/name
                // copies and fixed-key map probes. Hash collision internals excluded.
                self.work.charge(512 + 64 * tag.name.len());
                i = tag.end;
                if tag.closing {
                    if suppressed.last() == Some(&tag.name) {
                        suppressed.pop();
                    }
                } else {
                    if tag.name == "head" || tag.name == "template" {
                        if suppressed.len() == NESTING {
                            return Err(AnalysisError::Failed);
                        }
                        suppressed.push(tag.name.clone());
                    }
                    let in_template = suppressed.iter().any(|s| s == "template");
                    if tag.name == "script" || tag.name == "style" || tag.name == "title" {
                        let (end, after) = raw_end(source, i, &tag.name, &mut self.work)?;
                        if tag.name == "style" && !in_template {
                            self.css(&source[i..end], false)?;
                        }
                        i = after;
                    } else if suppressed.is_empty() {
                        if let Some(style) = tag.attrs.get("style") {
                            self.css(style, true)?;
                        }
                        image |= tag.name == "img";
                        for (r, hit) in self.catalogue.rules.iter().zip(&mut self.matched) {
                            match &r.selector {
                                Selector::Element { name } => *hit |= tag.name == *name,
                                Selector::MissingAttribute { element, attribute } => {
                                    *hit |=
                                        tag.name == *element && !tag.attrs.contains_key(attribute);
                                }
                                Selector::RemoteSmallImage {
                                    min_dimension,
                                    max_dimension,
                                } => {
                                    *hit |= tag.name == "img"
                                        && remote_small(
                                            &tag.attrs,
                                            *min_dimension,
                                            *max_dimension,
                                            &mut self.work,
                                        );
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            self.work.html += i - start;
        }
        if !suppressed.is_empty() {
            return Err(AnalysisError::Failed);
        }
        self.work.charge(32);
        for (r, hit) in self.catalogue.rules.iter().zip(&mut self.matched) {
            match r.selector {
                Selector::ImageOnly => *hit |= image && !text,
                Selector::DecodedBytes { greater_than } => *hit |= source.len() > greater_than,
                _ => {}
            }
        }
        Ok(())
    }
}
fn next_char(source: &str, i: usize, work: &mut Work) -> char {
    let c = source[i..].chars().next().unwrap();
    work.charge(c.len_utf8());
    c
}
fn escaped_end(source: &str, i: usize, work: &mut Work) -> usize {
    if i + 1 == source.len() {
        source.len()
    } else {
        i + 1 + next_char(source, i + 1, work).len_utf8()
    }
}
fn string_end(b: &[u8], mut i: usize, work: &mut Work) -> Result<usize, AnalysisError> {
    let quote = work.byte(b, i).unwrap();
    i += 1;
    loop {
        match work.byte(b, i) {
            Some(c) if c == quote => return Ok(i + 1),
            Some(b'\\') => i += 2,
            Some(_) => i += 1,
            None => return Err(AnalysisError::Failed),
        }
    }
}
fn css_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}
fn url_start(b: &[u8], i: usize, work: &mut Work) -> bool {
    work.charge(6); // bounded boundary and case-insensitive literal probe
    (i == 0
        || !matches!(b[i-1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'\\' | 128..=255))
        && b.get(i..i + 4)
            .is_some_and(|s| s.eq_ignore_ascii_case(b"url("))
}
/// None selects ordinary quoted function handling; Some consumes an unquoted
/// token. Comments and component delimiters in that token are URL data.
fn url_end(source: &str, mut i: usize, work: &mut Work) -> Result<Option<usize>, AnalysisError> {
    let b = source.as_bytes();
    while work.byte(b, i).is_some_and(css_space) {
        i += 1;
    }
    if matches!(work.byte(b, i), Some(b'\'' | b'"')) {
        return Ok(None);
    }
    loop {
        match work.byte(b, i).ok_or(AnalysisError::Failed)? {
            b')' => return Ok(Some(i + 1)),
            c if css_space(c) => {
                while work.byte(b, i).is_some_and(css_space) {
                    i += 1;
                }
                return if work.byte(b, i) == Some(b')') {
                    Ok(Some(i + 1))
                } else {
                    Err(AnalysisError::Failed)
                };
            }
            b'\\' => {
                i += 1;
                let c = work.byte(b, i).ok_or(AnalysisError::Failed)?;
                if matches!(c, b'\r' | b'\n' | b'\x0c') {
                    return Err(AnalysisError::Failed);
                }
                if c.is_ascii_hexdigit() {
                    let mut digits = 0;
                    while digits < 6 && work.byte(b, i).is_some_and(|c| c.is_ascii_hexdigit()) {
                        i += 1;
                        digits += 1;
                    }
                    if work.byte(b, i).is_some_and(css_space) {
                        let cr = work.byte(b, i) == Some(b'\r');
                        i += 1;
                        if cr && work.byte(b, i) == Some(b'\n') {
                            i += 1;
                        }
                    }
                } else {
                    i += next_char(source, i, work).len_utf8();
                }
            }
            b'\'' | b'"' | b'(' | 0..=8 | 11 | 14..=31 | 127 => return Err(AnalysisError::Failed),
            _ => i += next_char(source, i, work).len_utf8(),
        }
    }
}
fn inert_end(b: &[u8], mut i: usize, work: &mut Work) -> Result<usize, AnalysisError> {
    let mut quote = None;
    while let Some(c) = work.byte(b, i) {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if matches!(c, b'\'' | b'"') {
            quote = Some(c);
        } else if c == b'>' {
            return Ok(i + 1);
        }
        i += 1;
    }
    Err(AnalysisError::Failed)
}
fn terminated(b: &[u8], mut i: usize, end: &[u8], work: &mut Work) -> Result<usize, AnalysisError> {
    while i + end.len() <= b.len() {
        if work.prefix(b, i, end) {
            return Ok(i + end.len());
        }
        i += 1;
    }
    Err(AnalysisError::Failed)
}
struct Tag {
    name: String,
    closing: bool,
    attrs: HashMap<String, String>,
    end: usize,
}
fn tag(source: &str, start: usize, work: &mut Work) -> Result<Tag, AnalysisError> {
    let b = source.as_bytes();
    let closing = work.byte(b, start + 1) == Some(b'/');
    let mut i = start + 1 + usize::from(closing);
    let n = i;
    while work
        .byte(b, i)
        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b':'))
    {
        i += 1;
    }
    work.charge(i - n);
    let name = source[n..i].to_ascii_lowercase();
    let mut attrs = HashMap::new();
    loop {
        while work
            .byte(b, i)
            .is_some_and(|c| c.is_ascii_whitespace() || c == b'/')
        {
            i += 1;
        }
        match work.byte(b, i) {
            None => return Err(AnalysisError::Failed),
            Some(b'>') => {
                return Ok(Tag {
                    name,
                    closing,
                    attrs,
                    end: i + 1,
                });
            }
            _ => {}
        }
        let n = i;
        while work
            .byte(b, i)
            .is_some_and(|c| !c.is_ascii_whitespace() && !matches!(c, b'=' | b'>' | b'/'))
        {
            i += 1;
        }
        work.charge(3 * (i - n) + 1); // lowercase, hash and key comparison
        let attr = source[n..i].to_ascii_lowercase();
        while work.byte(b, i).is_some_and(|c| c.is_ascii_whitespace()) {
            i += 1;
        }
        let mut value = "";
        if work.byte(b, i) == Some(b'=') {
            i += 1;
            while work.byte(b, i).is_some_and(|c| c.is_ascii_whitespace()) {
                i += 1;
            }
            let quote = work.byte(b, i).ok_or(AnalysisError::Failed)?;
            if matches!(quote, b'\'' | b'"') {
                i += 1;
                let n = i;
                while work.byte(b, i).is_some_and(|c| c != quote) {
                    i += 1;
                }
                value = &source[n..i];
                if i == b.len() {
                    return Err(AnalysisError::Failed);
                }
                i += 1;
            } else {
                let n = i;
                while work
                    .byte(b, i)
                    .is_some_and(|c| !c.is_ascii_whitespace() && c != b'>')
                {
                    i += 1;
                }
                value = &source[n..i];
            }
        }
        // Empty malformed attribute names (e.g. '=x') still consume input.
        attrs.entry(attr).or_insert_with(|| work.decode(value));
    }
}
fn raw_end(
    source: &str,
    mut i: usize,
    name: &str,
    work: &mut Work,
) -> Result<(usize, usize), AnalysisError> {
    let b = source.as_bytes();
    while i < b.len() {
        if work.prefix(b, i, b"</") {
            work.charge(name.len() + 2);
            let end = i + 2 + name.len();
            if b.get(i + 2..end)
                .is_some_and(|s| s.eq_ignore_ascii_case(name.as_bytes()))
                && b.get(end)
                    .is_some_and(|c| c.is_ascii_whitespace() || *c == b'>')
            {
                return Ok((i, tag(source, i, work)?.end));
            }
        }
        i += 1;
    }
    Err(AnalysisError::Failed)
}
fn remote_small(attrs: &HashMap<String, String>, min: u32, max: u32, work: &mut Work) -> bool {
    // Three fixed-key lookups; trims, digit validation and bounded integer parse.
    work.charge(
        64 + 8 * ["src", "width", "height"]
            .iter()
            .filter_map(|k| attrs.get(*k))
            .map(String::len)
            .sum::<usize>(),
    );
    let remote = attrs.get("src").is_some_and(|s| {
        let s = s.trim();
        s.get(..7)
            .is_some_and(|p| p.eq_ignore_ascii_case("http://"))
            || s.get(..8)
                .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
    });
    remote
        && ["width", "height"].iter().all(|key| {
            attrs.get(*key).is_some_and(|s| {
                let s = s.trim();
                !s.is_empty()
                    && s.bytes().all(|b| b.is_ascii_digit())
                    && s.parse::<u32>().is_ok_and(|v| (min..=max).contains(&v))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::super::{Inputs, Report, evaluate};
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde_json::{Value, json};

    // The engine owns these admission budgets; the resource-limit tests pin them.
    const RAW_BYTES: usize = super::super::MAX_RAW_BYTES;
    const PARTS: usize = super::super::MAX_PARTS;
    const DEPTH: usize = super::super::MAX_DEPTH;

    fn raw(body: &str) -> Vec<u8> {
        format!("Content-Type: text/html; charset=utf-8\r\n\r\n{body}").into_bytes()
    }
    fn report(body: &str) -> Report {
        let mut r = evaluate(&raw(body), &Inputs::default()).unwrap();
        r.findings.retain(|f| f.rule_id.starts_with("html."));
        r
    }
    fn ids(body: &str) -> Vec<String> {
        report(body)
            .findings
            .into_iter()
            .map(|f| f.rule_id)
            .collect()
    }
    fn has(body: &str, id: &str) -> bool {
        ids(body).iter().any(|s| s == id)
    }
    fn multipart(children: &[Vec<u8>], boundary: &str, extra: &str) -> Vec<u8> {
        let mut raw =
            format!("Content-Type: multipart/alternative; boundary={boundary}\r\n{extra}\r\n")
                .into_bytes();
        for child in children {
            raw.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            raw.extend(child);
            raw.extend(b"\r\n");
        }
        raw.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        raw
    }
    #[test]
    fn reviewed_custom_properties_preserve_declaration_context() {
        for css in [
            "--unused:{position:absolute};color:red",
            "--unused:{a:[position:absolute;display:flex]};color:red",
        ] {
            for body in [
                format!("<p style='{css}'>Hello</p>"),
                format!("<style>.x{{{css}}}</style><p>Hello</p>"),
            ] {
                assert_eq!(
                    report(&body),
                    Report {
                        rule_set_id: super::super::RULE_SET_ID.into(),
                        findings: vec![]
                    }
                );
            }
        }
    }
    #[test]
    fn reviewed_unquoted_url_keeps_comment_characters_literal() {
        let css = "background:url(https://x.test/*/a.png)";
        for body in [
            format!("<p style='{css}'>Hello</p>"),
            format!("<style>.x{{{css}}}</style><p>Hello</p>"),
        ] {
            assert_eq!(
                Ok::<_, AnalysisError>(report(&body)),
                Ok(Report {
                    rule_set_id: super::super::RULE_SET_ID.into(),
                    findings: vec![]
                })
            );
        }
    }
    #[test]
    fn reviewed_empty_raw_has_no_html_parts() {
        let mut collector = Collector(Vec::new());
        collect(b"", &mut collector).unwrap();
        assert!(collector.0.is_empty());
        assert_eq!(
            evaluate(b"", &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }
    fn expected(ids: &[&str], path: &[u64]) -> Report {
        let c = catalogue(DATA).unwrap();
        let mut findings: Vec<_> = c
            .rules
            .iter()
            .filter(|r| ids.contains(&r.id.as_str()))
            .map(|r| Finding {
                rule_id: r.id.clone(),
                severity: r.severity,
                fix_hint: r.fix_hint.clone(),
                location: Location::MimePart {
                    path: path.to_vec(),
                },
            })
            .collect();
        findings.sort();
        Report {
            rule_set_id: super::super::RULE_SET_ID.into(),
            findings,
        }
    }
    #[test]
    fn component_values_keep_outer_and_following_declarations_visible() {
        for (css, ids) in [
            (
                "--unused:{position:absolute};position:absolute",
                vec!["html.css.position.v1"],
            ),
            (
                "--unused:{a:[(position:absolute;display:flex)]};position:relative",
                vec!["html.css.position.v1"],
            ),
            (
                "--unused:{a:'}';b:/*}*/[\\]()]};display:flex",
                vec!["html.css.display-flex.v1"],
            ),
            (
                "position:{display:flex; a:[b(c)]}",
                vec!["html.css.position.v1"],
            ),
            (
                "position:fn({a:b});display:inline-flex ! IMPORTANT",
                vec!["html.css.position.v1", "html.css.display-flex.v1"],
            ),
            ("display:fl{position:absolute}ex", vec![]),
            (
                "display:[flex];--unused:(position:absolute);color:red",
                vec![],
            ),
            ("--unused:{url(x/*/a);position:absolute};color:red", vec![]),
            (
                "--unused:{\"position:absolute\"};position&#58;absolute",
                vec!["html.css.position.v1"],
            ),
            ("--unused:{a:\\é};color:red", vec![]),
        ] {
            let attr = css.replace('"', "&quot;");
            for body in [
                format!("<p style=\"{attr}\">Hello</p>"),
                format!(
                    "<style>@media all {{ .x:hover[data-a=x] {{{}}} }}</style><p>Hello</p>",
                    css.replace("&#58;", ":")
                ),
            ] {
                let result = report(&body);
                assert_eq!(result, expected(&ids, &[]), "{body}");
                assert_eq!(result.qualifies(Severity::Warning), ids.is_empty());
            }
        }
        let nested = multipart(
            &[
                raw("Hello"),
                multipart(
                    &[raw(
                        "<p style='--unused:{position:relative};position:absolute'>Hello</p>",
                    )],
                    "inner",
                    "",
                ),
            ],
            "outer",
            "",
        );
        let mut nested_report = evaluate(&nested, &Inputs::default()).unwrap();
        nested_report
            .findings
            .retain(|f| f.rule_id.starts_with("html."));
        assert_eq!(nested_report, expected(&["html.css.position.v1"], &[1, 0]));
        for n in [32, 33] {
            let css = format!(
                "--unused:{}x{};position:absolute",
                "[({".repeat(n / 3) + &"(".repeat(n % 3),
                ")".repeat(n % 3) + &"})]".repeat(n / 3)
            );
            assert_eq!(
                evaluate(
                    &raw(&format!("<p style='{css}'>Hello</p>")),
                    &Inputs::default()
                )
                .is_ok(),
                n == 32
            );
        }
        for css in ["--x:[}", "--x:{]", "--x:([)]", "--x:{a:b", "x:]", "}"] {
            assert_eq!(
                evaluate(
                    &raw(&format!("<p style='{css}'>Hello</p>")),
                    &Inputs::default()
                ),
                Err(AnalysisError::Failed)
            );
        }
    }
    #[test]
    fn literal_urls_protect_data_without_disabling_ordinary_comments() {
        for css in [
            "background:URL( https://x.test/*/a.png );position:absolute",
            "background:url(https://x.test/a{b[c;d:e]);position:absolute",
            r"background:url(a\)b);position:absolute",
            r"background:url(a\28 b\000029c);position:absolute",
            "background:url(a\\29\r\nb);position:absolute",
            "background:url(a\\\u{e9}b);position:absolute",
            "background:url(a\\000029 b);position:absolute",
            "background:url( );position:absolute",
            "background:url(\"a/*\");position:absolute",
            "background:url('a)/*');position:absolute",
            "background:myurl(a/* comment */b);position:absolute",
            "background:-url(a/* comment */b);position:absolute",
            "background:éurl(a/* comment */b);position:absolute",
            "background:\\url(a/* comment */b);position:absolute",
            "background: url (a/* comment */b);position:absolute",
            "--unused:(url(a/*/b));position:absolute",
            "--unused:(url('a/*'));position:absolute",
            "background:url( 'a/*');position:absolute",
            "background:url(a\\29\rb);position:absolute",
        ] {
            // HTML double quotes protect single-quoted CSS strings.
            let attr = css.replace('"', "&quot;");
            for body in [
                format!("<p style=\"{attr}\">Hello</p>"),
                format!("<style>.x{{{css}}}</style><p>Hello</p>"),
            ] {
                assert_eq!(
                    report(&body),
                    expected(&["html.css.position.v1"], &[]),
                    "{body}"
                );
            }
        }
        for css in [
            "background:url(a b)",
            "background:url(a'b)",
            "background:url(a\"b)",
            "background:url(a(b)",
            "background:url(a",
            "background:url(a\\",
            "background:url(a\\\nb)",
            "background:url(a\\\rb)",
            "background:url(a\\\u{c}b)",
            "background:url(a\0b)",
            "background:url(a\u{1}b)",
            "background:url(a\u{b}b)",
            "background:url(a\u{e}b)",
            "background:url(a\u{1f}b)",
            "background:url(a\u{7f}b)",
            "background:myurl(a/*/b)",
            "background:fn(a/*/b)",
            "background: url (a/*/b)",
            "background:url(a);/*",
            "background:url(a   ",
        ] {
            let body = format!("<style>.x{{position:absolute;{css}}}</style>");
            assert_eq!(
                evaluate(&raw(&body), &Inputs::default()),
                Err(AnalysisError::Failed),
                "{css}"
            );
        }
        for n in [31, 32] {
            let body = format!(
                "<p style='x:{}url(a){}'>Hello</p>",
                "(".repeat(n),
                ")".repeat(n)
            );
            assert_eq!(evaluate(&raw(&body), &Inputs::default()).is_ok(), n == 31);
        }
    }
    #[test]
    fn inspected_work_has_a_fixed_linear_bound_separate_from_progress() {
        const C: u64 = 8192;
        const K: u64 = 4096;
        let c = catalogue(DATA).unwrap();
        // Chosen before measuring; each exercises the real production helper.
        for n in [64, 512, 4096, 32768] {
            for body in [
                format!("<!--{}-->", "-".repeat(n)),
                format!(
                    "<p data-x='{}' title={}>Hello</p>",
                    ">".repeat(n),
                    "x".repeat(n)
                ),
                format!(
                    "<script>{}</SCRIPT ><title>{}</title>",
                    "</scripX>".repeat(n),
                    "</titlX>".repeat(n)
                ),
                format!("<!x '{}'><?x \"{}\">Hello", ">".repeat(n), "'".repeat(n)),
                format!(
                    "<p>{}{}{}{}{}{}</p>",
                    "&".repeat(n),
                    "&unknown;".repeat(n),
                    "&éé;".repeat(n),
                    "&nGt;".repeat(n),
                    "&#x41;".repeat(n),
                    "&amp;lt;".repeat(n)
                ),
                format!(
                    "<style>.x{{--unused:[{{({})}}];content:'{}';background:url({});display:flex}}</style>",
                    "x;position:absolute;".repeat(n),
                    "\\\"".repeat(n),
                    "a/*/".repeat(n)
                ),
                format!(
                    "<p style='{}'>Hello</p>",
                    "color:red;--x:{position:absolute};".repeat(n)
                ),
            ] {
                let mut scan = Scan::new(&c);
                scan.html(&body).unwrap();
                assert_eq!(scan.work.html, body.len(), "progress");
                // Mutation experiment must reach this assertion with progress green.
                assert!(
                    scan.work.inspections <= C * body.len() as u64 + K,
                    "inspections={} n={} limit={}",
                    scan.work.inspections,
                    body.len(),
                    C * body.len() as u64 + K
                );
            }
            for body in [
                format!("<!--{}--", "-".repeat(n)),
                format!("<p x='{}", "x".repeat(n)),
                format!("<script>{}", "</scripX>".repeat(n)),
                format!("<!x '{}'", ">".repeat(n)),
                format!("<style>.x{{x:{}/*", "color:red;".repeat(n)),
                format!("<style>.x{{background:url({}\\", "a".repeat(n)),
                format!("<style>.x{{content:'{}\\", "a".repeat(n)),
            ] {
                let mut scan = Scan::new(&c);
                assert_eq!(scan.html(&body), Err(AnalysisError::Failed));
                assert!(scan.work.html <= body.len());
                assert!(scan.work.inspections <= C * body.len() as u64 + K);
            }
        }
        // Helper-only domains, including the 40-scalar construction lookahead.
        for source in [
            "",
            "u",
            "url(x)",
            "&",
            "&nGt;",
            "&unknown;",
            "&éé;",
            "&#x41;",
            "&amp;lt;",
            "é",
        ] {
            let mut work = Work::default();
            let decoded = work.decode(source);
            assert!(5 * decoded.len() <= 6 * source.len());
            assert!(work.inspections <= C * source.len() as u64 + K);
            let mut scan = Scan::new(&c);
            scan.css(source, true).unwrap();
            assert_eq!(scan.work.css, source.len());
            assert!(scan.work.inspections <= C * source.len() as u64 + K);
        }
        // Empty inline declarations and malformed short constructs count as well.
        let mut scan = Scan::new(&c);
        scan.css("", true).unwrap();
        assert!(scan.work.inspections <= K);
    }
    #[test]
    fn reviewed_version_snapshot_and_every_rule_pair() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/html-compatibility.json"))
                .unwrap();
        assert_eq!(
            fixture["catalogue_snapshot"],
            serde_json::from_str::<Value>(DATA).unwrap()
        );
        assert_eq!(
            fixture["catalogue_snapshot"]["rule_set_id"],
            "swarmail-html/2"
        );
        let c = catalogue(DATA).unwrap();
        assert_eq!(c.rules.len(), 7);
        assert_eq!(fixture["pairs"].as_array().unwrap().len(), c.rules.len());
        for (rule, pair) in c.rules.iter().zip(fixture["pairs"].as_array().unwrap()) {
            assert_eq!(pair["rule_id"], rule.id);
            for (key, matched) in [("pass", false), ("fail", true)] {
                let recipe = &pair[key];
                let body = match recipe.as_str() {
                    Some(s) => s.to_string(),
                    None => recipe["repeat"]
                        .as_str()
                        .unwrap()
                        .repeat(recipe["bytes"].as_u64().unwrap() as usize),
                };
                let r = report(&body);
                let expected = if matched {
                    vec![Finding {
                        rule_id: rule.id.clone(),
                        severity: rule.severity,
                        location: Location::MimePart { path: vec![] },
                        fix_hint: rule.fix_hint.clone(),
                    }]
                } else {
                    vec![]
                };
                assert_eq!(r.findings, expected, "{} {key}", rule.id);
            }
        }
    }
    #[test]
    fn catalogue_is_closed_and_purely_validated() {
        let base: Value = serde_json::from_str(DATA).unwrap();
        for (pointer, value) in [
            ("/rule_set_id", json!("stale/0")),
            ("/provenance", json!(" ")),
            ("/rules", json!([])),
            ("/rules", json!(vec![base["rules"][0].clone(); 33])),
            ("/rules/0/id", json!("")),
            ("/rules/1/id", base["rules"][0]["id"].clone()),
            ("/rules/0/severity", json!("fatal")),
            ("/rules/0/fix_hint", json!(" ")),
            ("/rules/0/clients", json!([])),
            ("/rules/0/clients", json!([" "])),
            ("/rules/0/source/title", json!("")),
            ("/rules/0/source/url", json!("http://x")),
            ("/rules/0/source/summary", json!("")),
            ("/rules/0/source/basis_type", json!("measured")),
            ("/rules/0/source/basis_date", json!("yesterday")),
            ("/rules/0/selector/property", json!("")),
            ("/rules/0/selector/property", json!("POSITION")),
            ("/rules/0/selector/values", json!(["not a token"])),
            ("/rules/2/selector/name", json!("")),
            ("/rules/3/selector/element", json!("")),
            ("/rules/3/selector/attribute", json!("")),
            ("/rules/5/selector/greater_than", json!(0)),
            ("/rules/6/selector/min_dimension", json!(2)),
            ("/rules/0/selector/kind", json!("code")),
        ] {
            let mut v = base.clone();
            *v.pointer_mut(pointer).unwrap() = value;
            assert!(catalogue(&v.to_string()).is_err(), "{pointer}");
        }
        for pointer in ["", "/rules/0", "/rules/0/source", "/rules/0/selector"] {
            let mut v = base.clone();
            v.pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unknown".into(), json!(true));
            assert!(catalogue(&v.to_string()).is_err());
        }
        assert!(catalogue("not JSON").is_err());
        assert_eq!(
            collect_catalogue(
                &raw("hello"),
                &Err(AnalysisError::Failed),
                &mut Collector(vec![])
            ),
            Err(AnalysisError::Failed)
        );
        assert_eq!(
            collect_catalogue(b"", &Err(AnalysisError::Failed), &mut Collector(vec![])),
            Err(AnalysisError::Failed)
        );
        // Whitespace remains ordinary MIME input, not the exact-empty shortcut.
        assert!(MessageParser::default().parse(b" ").is_none());
        assert_eq!(
            evaluate(b" ", &Inputs::default()),
            Err(AnalysisError::Failed)
        );
        // Valid selector and threshold edits change behavior without new Rust.
        let mut changed = base;
        changed["rules"][0]["selector"]["property"] = json!("color");
        changed["rules"][5]["selector"]["greater_than"] = json!(3);
        let c = catalogue(&changed.to_string()).unwrap();
        let mut out = Collector(vec![]);
        collect_with(&raw("<p style='color:red'>ok</p>"), &c, &mut out).unwrap();
        assert_eq!(out.0.len(), 2);
        assert!(out.0.iter().any(|f| f.rule_id == "html.css.position.v1"));
    }
    #[test]
    fn lexical_html_and_css_are_source_aware() {
        let positive = [
            "<DIV STYLE='PoSiTiOn : absolute; DISPLAY: INLINE-FLEX ! IMPORTANT'>text</DIV>",
            "<head><style>@media screen {@media all {.x {display: flex!important; position:relative}}}</style></head>",
            "<p style='display&#58;flex'>text</p>",
            "<div style='background:url(\"a;display:block}\");display:flex'>ok</div>",
            "<style>.x { /* hi */ display: /*yes*/ flex; }</style>",
            "<video data-x='>'></video>",
            "<!--[if mso]><!--><video></video><!--<![endif]-->",
        ];
        for body in positive {
            assert!(!ids(body).is_empty(), "{body}");
        }
        assert_eq!(
            ids("<ViDeO data-x='>'></ViDeO><video></video>"),
            vec!["html.element.video.v1"]
        );
        assert_eq!(
            ids("<p style='display:flex' STYLE='display:block'>ok</p>"),
            vec!["html.css.display-flex.v1"]
        );
        for body in [
            "position:absolute; display:flex <p title='display:flex'>ok</p>",
            "<custom-tag:label style \t= \n 'font:12px/14px;display:block' title \t= \t test>ok</custom-tag:label>",
            "<!--<video><img><style>.x{display:flex}</style>-->",
            "<!--[if mso]><video></video><![endif]-->",
            "<script>var s='<video>';display:flex;</script>",
            "<head><video><img>words<title>words</title></head>",
            "<template><style>.x{display:flex}</style><video><img>words</template>",
            "<style>.x[title='display:flex'] {content:'display:flex;position:absolute';}</style>",
            "<style>display:flex; .x{display:'flex';content:\"a\\\";position:absolute\"}</style>",
            "<p style='display:flex grid; position: ;display:flex!no'>ok</p>",
            "<p style='background:url(display:flex); content: \"a\\\";display:flex\"'>ok</p>",
            r"<p style='displ\61 y:flex; display:fl\65 x'>ok</p>",
            "&lt;video&gt; &amp;lt;img&amp;gt; <3 > literal <",
            "<?xml version='1'?><!DOCTYPE html><!DOCTYPE html \"<video>\"><!x '<img>'>",
            "<p =x foo=bar / weird:attr='&unknown;'>&bad;</p>",
        ] {
            assert!(ids(body).is_empty(), "{body}");
        }
    }
    #[test]
    fn images_use_attributes_and_lexical_body_text() {
        for alt in ["alt", "alt=''", "alt=' '", "ALT=decorative", "alt='' alt=x"] {
            assert!(!has(
                &format!("<p>text</p><img {alt}>"),
                "html.img.alt-missing.v1"
            ));
        }
        for body in [
            "",
            "<p> </p>",
            "<img alt=''>hi",
            "<img alt=''>&amp;nbsp;",
            "<img alt=''><span style='display:none'>hidden</span>",
        ] {
            assert!(!has(body, "html.image-only.v1"));
        }
        for body in [
            "<img alt=''>",
            "<img alt='nonempty' title='hello'>&nbsp;&#160;\u{a0}\n",
            "<head>words<img></head><title>title</title><template><img>words</template><img alt=''>",
        ] {
            assert!(has(body, "html.image-only.v1"), "{body}");
        }
        for dimension in ["0", "1", "0001", " 1 ", "&#49;"] {
            assert!(has(
                &format!("<img alt='' src=' HTtPs&#58;//x ' width='{dimension}' height='0'>"),
                "html.tracking-pixel.v1"
            ));
        }
        for attrs in [
            "src='http://x' width=1 height=1",
            "src='https://x' width=0 height=1",
        ] {
            assert!(has(
                &format!("<img alt='' {attrs}>"),
                "html.tracking-pixel.v1"
            ));
        }
        for attrs in [
            "",
            "src='https://x'",
            "src='https://x' width=1",
            "width=1 height=1",
            "src='https://x' width=2 height=1",
            "src='https://x' width=1 height=2",
            "src='https://x' width='' height=1",
            "src='https://x' width=+1 height=1",
            "src='https://x' width=1px height=1",
            "src='https://x' width=1.0 height=1",
            "src='https://x' width=1% height=1",
            "src='https://x' width=999999999999999 height=1",
            "src='cid:x' width=1 height=1",
            "src='data:x' width=1 height=1",
            "src='/x' width=1 height=1",
            "src='éééé' width=1 height=1",
            "src='https://x' style='width:1px;height:1px'",
            "src='https://x' width=2 width=1 height=1",
        ] {
            assert!(
                !has(&format!("<img alt='' {attrs}>"), "html.tracking-pixel.v1"),
                "{attrs}"
            );
        }
        let pixel = report("<img alt='' width=0 height=0 src='https://x'>");
        assert_eq!(pixel.findings.len(), 2);
        assert!(!pixel.qualifies(Severity::Warning));
    }
    #[test]
    fn malformed_supported_syntax_fails_and_nesting_is_bounded() {
        for body in [
            "<!doctype",
            "<?x '",
            "<p",
            "<p x='",
            "<p unfinished",
            "<p x=",
            "<p x='ok'",
            "<!--",
            "<!--end--",
            "<script>",
            "<style>",
            "<title>",
            "<head>",
            "<template>",
            "<style></sty",
            "<style></stylesheet>",
            "<style>.x {display:flex</style>",
            "<p style='content:&quot;unterminated'>",
            "<style>.x{content:'a\\</style>",
            "<style>/* comment</style>",
            "<style>}</style>",
            "<p style=')'>",
            "<p style='url('>",
            "<style>.x{content:({;})</style>",
        ] {
            assert_eq!(
                evaluate(&raw(body), &Inputs::default()),
                Err(AnalysisError::Failed),
                "{body}"
            );
        }
        for n in [32, 33] {
            for body in [
                format!("{}{}", "<template>".repeat(n), "</template>".repeat(n)),
                format!("<style>{} {}</style>", ".x{".repeat(n), "}".repeat(n)),
                format!("<p style='x:{}x{}'>", "(".repeat(n), ")".repeat(n)),
            ] {
                assert_eq!(evaluate(&raw(&body), &Inputs::default()).is_ok(), n == 32);
            }
        }
        // A declaration block and its value functions share one lexical budget.
        for n in [31, 32] {
            let body = format!("<style>.x{{x:{}x{}}}</style>", "(".repeat(n), ")".repeat(n));
            assert_eq!(evaluate(&raw(&body), &Inputs::default()).is_ok(), n == 31);
        }
        // Cursor progress remains a distinct invariant, including inert/raw/entity runs.
        let c = catalogue(DATA).unwrap();
        let body = format!(
            "<script>{}</SCRIPT ><p data-x='{}'>{}{}</p><style>.x{{content:'{}';x:url({});display:flex}}</style>",
            "</scripX>".repeat(4096),
            ">".repeat(4096),
            "&".repeat(4096),
            "<3".repeat(4096),
            "\\\"".repeat(4096),
            "a;{}".repeat(4096)
        );
        let mut scan = Scan::new(&c);
        scan.html(&body).unwrap();
        assert_eq!(scan.work.html, body.len());
        assert!(scan.work.css < body.len());
        assert!(scan.matched[1]);
        // Trailing escaped literal is outside identifier normalization.
        let mut scan = Scan::new(&c);
        scan.css("x:é;\\", true).unwrap();
        assert_eq!(scan.work.css, "x:é;\\".len());
    }
    #[test]
    fn mime_paths_exclusions_and_decoding() {
        let image = raw("<img>");
        let text = b"Content-Type: text/plain\r\n\r\nPlain fallback <video>".to_vec();
        let nested = multipart(&[text.clone(), image.clone()], "inner", "");
        let raw_mime = multipart(&[image.clone(), nested], "outer", "");
        let r = evaluate(&raw_mime, &Inputs::default()).unwrap();
        let html: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("html."))
            .collect();
        assert_eq!(html.len(), 4);
        for pair in html.chunks(2) {
            assert_eq!(pair[0].location, Location::MimePart { path: vec![0] });
            assert_eq!(pair[1].location, Location::MimePart { path: vec![1, 1] });
        }
        assert!(
            evaluate(&text, &Inputs::default())
                .unwrap()
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("html."))
        );
        for headers in [
            "Content-Type: text/html\r\nContent-Disposition: attachment",
            "Content-Type: text/html\r\nContent-Disposition: inline; filename=x.html",
            "Content-Type: text/html; name=x.html",
        ] {
            assert!(
                evaluate(
                    format!("{headers}\r\n\r\n<video>").as_bytes(),
                    &Inputs::default()
                )
                .unwrap()
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("html."))
            );
        }
        let attached = multipart(
            std::slice::from_ref(&image),
            "attach",
            "Content-Disposition: attachment\r\n",
        );
        assert!(
            evaluate(&attached, &Inputs::default())
                .unwrap()
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("html."))
        );
        let attached_message =
            [b"Content-Type: message/rfc822\r\n\r\n".as_slice(), &image].concat();
        assert!(
            evaluate(&attached_message, &Inputs::default())
                .unwrap()
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("html."))
        );
        let inline = b"Content-Type: text/html\r\nContent-Disposition: inline\r\n\r\n<video>";
        let n = evaluate(inline, &Inputs::default())
            .unwrap()
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("html."))
            .count();
        assert_eq!(n, 1);
        for (encoding, body) in [
            ("base64", STANDARD.encode("<video>")),
            ("quoted-printable", "=3Cvideo=3E".into()),
        ] {
            let r = evaluate(
                format!(
                    "Content-Type: text/html\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{body}"
                )
                .as_bytes(),
                &Inputs::default(),
            )
            .unwrap();
            let html: Vec<_> = r
                .findings
                .iter()
                .filter(|f| f.rule_id.starts_with("html."))
                .cloned()
                .collect();
            let expected_report = report("<video>");
            let expected: Vec<_> = expected_report
                .findings
                .iter()
                .filter(|f| f.rule_id.starts_with("html."))
                .cloned()
                .collect();
            assert_eq!(html, expected);
        }
        for encoding in ["base64", "quoted-printable"] {
            let bad = format!(
                "Content-Type: text/html\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n====%z"
            );
            assert_eq!(
                evaluate(bad.as_bytes(), &Inputs::default()),
                Err(AnalysisError::Failed)
            );
        }
        // Pinned decoder replaces invalid UTF-8, it does not promise losslessness.
        let replaced = b"Content-Type: text/html; charset=utf-8\r\n\r\n<img alt=''>\xff";
        assert!(
            evaluate(replaced, &Inputs::default())
                .unwrap()
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("html."))
        );
        assert_eq!(
            evaluate(b"", &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }
    #[test]
    fn decoded_clipping_boundaries_follow_tracked_recipes() {
        let recipes: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/html-compatibility.json"))
                .unwrap();
        for size in recipes["size_recipe"]["byte_lengths"].as_array().unwrap() {
            let size = size.as_u64().unwrap() as usize;
            for encoding in recipes["size_recipe"]["encodings"].as_array().unwrap() {
                let encoding = encoding.as_str().unwrap();
                // A single-byte Windows-1252 é expands to two UTF-8 bytes.
                let mut bytes = vec![b'x'; size - 2];
                bytes.push(0xe9);
                let encoded = if encoding == "base64" {
                    STANDARD.encode(&bytes)
                } else {
                    format!("{}=E9", "x".repeat(size - 2))
                };
                let message = format!(
                    "Content-Type: text/html; charset=windows-1252\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{encoded}"
                );
                let r = evaluate(message.as_bytes(), &Inputs::default()).unwrap();
                let html_n = r
                    .findings
                    .iter()
                    .filter(|f| f.rule_id.starts_with("html."))
                    .count();
                assert_eq!(html_n, usize::from(size > 104448), "{size} {encoding}");
            }
        }
        // HTML source bytes, not entity-decoded characters, count.
        assert!(has(&"&nbsp;".repeat(18000), "html.gmail-clipping.v1"));
    }
    #[test]
    fn raw_decoded_structural_and_contribution_limits_are_exact() {
        let mut bytes = b"Content-Type: text/plain\r\n\r\n".to_vec();
        bytes.resize(RAW_BYTES, b'x');
        assert!(evaluate(&bytes, &Inputs::default()).is_ok());
        bytes.push(b'x');
        assert_eq!(
            evaluate(&bytes, &Inputs::default()),
            Err(AnalysisError::Failed)
        );
        for n in [HTML_BYTES, HTML_BYTES + 1] {
            let body = "x".repeat(n);
            assert_eq!(
                evaluate(&raw(&body), &Inputs::default()).is_ok(),
                n == HTML_BYTES
            );
        }
        let halves = multipart(
            &[
                raw(&"x".repeat(HTML_BYTES / 2)),
                raw(&"x".repeat(HTML_BYTES / 2 + 1)),
            ],
            "aggregate",
            "",
        );
        assert_eq!(
            evaluate(&halves, &Inputs::default()),
            Err(AnalysisError::Failed)
        );
        for n in [PARTS - 1, PARTS] {
            let parts = vec![b"Content-Type: text/plain\r\n\r\nx".to_vec(); n];
            assert_eq!(
                evaluate(
                    &multipart(&parts, "parts", "Content-Disposition: attachment\r\n"),
                    &Inputs::default()
                )
                .is_ok(),
                n == PARTS - 1
            );
        }
        for n in [DEPTH, DEPTH + 1] {
            let mut message = raw("<video>");
            for level in 0..n {
                message = multipart(&[message], &format!("depth{level}"), "");
            }
            assert_eq!(evaluate(&message, &Inputs::default()).is_ok(), n == DEPTH);
        }
        // Two contributions per HTML part; the 256 cap counts the seven bulk
        // findings this minimal fixture also emits, so 124 parts stay exact.
        for n in [124, 125] {
            let message = multipart(&vec![raw("<img>"); n], "findings", "");
            let r = evaluate(&message, &Inputs::default());
            assert_eq!(r.is_ok(), n == 124);
        }
        // No partial cache entry survives a later malformed part.
        let bad = multipart(&[raw("<video>"), raw("<script>")], "bad", "");
        assert_eq!(
            evaluate(&bad, &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }
    #[tokio::test]
    async fn malformed_html_is_retried_and_never_cached() {
        let service = super::super::Service::default();
        for body in [
            "<video><script>",
            "<style>.x{position:absolute;background:url(a b)}</style>",
        ] {
            let mut email = crate::store::email("bad-html", "box", "a");
            email.raw = raw(body);
            let email = std::sync::Arc::new(email);
            for _ in 0..2 {
                assert_eq!(
                    service.report(email.clone(), false).await,
                    Err(AnalysisError::Failed)
                );
            }
            let slot = service.slots.lock().unwrap()[0].clone();
            assert!(slot.report.lock().await.is_none());
            // Distinct immutable owners receive independent failed slots.
            service.slots.lock().unwrap().clear();
        }
    }
}
