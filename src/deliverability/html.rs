//! Finite source-aware risk heuristics, not a renderer. Data is authored locally
//! from named guidance; no client measurement or runtime networking is implied.
use super::{AnalysisError, Collector, Finding, Location, Severity};
use crate::extract::decode_refs;
use mail_parser::{MessageParser, MimeHeaders, PartType};
use serde::Deserialize;
use std::{collections::HashMap, sync::LazyLock};

const RAW_BYTES: usize = 8_388_608;
const HTML_BYTES: usize = 4_194_304;
const PARTS: usize = 256;
const DEPTH: usize = 32;
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
    if raw.len() > RAW_BYTES {
        return Err(AnalysisError::Failed);
    }
    let message = MessageParser::default()
        .parse(raw)
        .ok_or(AnalysisError::Failed)?;
    let mut stack = vec![(0usize, Vec::new(), false)];
    let mut eligible = Vec::new();
    let mut count = 0;
    let mut bytes = 0;
    while let Some((id, path, excluded)) = stack.pop() {
        count += 1;
        if count > PARTS || path.len() > DEPTH {
            return Err(AnalysisError::Failed);
        }
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

/// Every cursor advance is accounted per invocation (including inert content).
/// Helpers scan disjoint slices or a constant number of passes over those slices.
#[derive(Default)]
struct Work {
    html: usize,
    css: usize,
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
        let mut braces = Vec::new();
        let mut parentheses = 0;
        while i < b.len() {
            let start = i;
            match b[i] {
                b'/' if b.get(i + 1) == Some(&b'*') => {
                    i = terminated(b, i + 2, b"*/")?;
                    segment.push(' ');
                }
                b'\'' | b'"' => {
                    let quote = b[i];
                    i += 1;
                    loop {
                        match b.get(i) {
                            Some(v) if *v == quote => {
                                i += 1;
                                break;
                            }
                            Some(b'\\') => {
                                i += 2;
                            }
                            Some(_) => {
                                i += 1;
                            }
                            None => return Err(AnalysisError::Failed),
                        }
                    }
                    // Strings cannot manufacture literal property/value tokens.
                    segment.push('\0');
                }
                b'\\' => {
                    let end = (i + 2).min(b.len());
                    segment.push_str(&String::from_utf8_lossy(&b[i..end]));
                    i = end;
                }
                b'(' => {
                    parentheses += 1;
                    if parentheses > NESTING {
                        return Err(AnalysisError::Failed);
                    }
                    segment.push('(');
                    i += 1;
                }
                b')' => {
                    if parentheses == 0 {
                        return Err(AnalysisError::Failed);
                    }
                    parentheses -= 1;
                    segment.push(')');
                    i += 1;
                }
                b'{' if parentheses == 0 => {
                    if braces.len() == NESTING {
                        return Err(AnalysisError::Failed);
                    }
                    let container = segment
                        .trim_start()
                        .to_ascii_lowercase()
                        .starts_with("@media");
                    braces.push(!container);
                    segment.clear();
                    i += 1;
                }
                b'}' if parentheses == 0 => {
                    if braces.last() == Some(&true) {
                        self.declaration(&segment);
                    }
                    if braces.pop().is_none() {
                        return Err(AnalysisError::Failed);
                    }
                    segment.clear();
                    i += 1;
                }
                b';' if parentheses == 0 => {
                    if braces.last().copied().unwrap_or(inline) {
                        self.declaration(&segment);
                    }
                    segment.clear();
                    i += 1;
                }
                _ => {
                    segment.push(b[i] as char);
                    i += 1;
                }
            }
            self.work.css += i - start;
        }
        if !braces.is_empty() || parentheses != 0 {
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
            if b[i] != b'<'
                || !b
                    .get(i + 1)
                    .is_some_and(|v| v.is_ascii_alphabetic() || matches!(v, b'/' | b'!' | b'?'))
            {
                i += 1;
                while i < b.len() && b[i] != b'<' {
                    i += 1;
                }
                if suppressed.is_empty() {
                    text |= decode_refs(&source[start..i])
                        .chars()
                        .any(|c| !c.is_whitespace());
                }
            } else if b[i..].starts_with(b"<!--") {
                i = terminated(b, i + 4, b"-->")?;
            } else if b.get(i + 1).is_some_and(|v| matches!(v, b'!' | b'?')) {
                i = inert_end(b, i + 2)?;
            } else {
                let tag = tag(source, i)?;
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
                        let (end, after) = raw_end(source, i, &tag.name)?;
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
                                        && remote_small(&tag.attrs, *min_dimension, *max_dimension);
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
fn inert_end(b: &[u8], mut i: usize) -> Result<usize, AnalysisError> {
    let mut quote = None;
    while let Some(c) = b.get(i) {
        if let Some(q) = quote {
            if *c == q {
                quote = None;
            }
        } else if matches!(c, b'\'' | b'"') {
            quote = Some(*c);
        } else if *c == b'>' {
            return Ok(i + 1);
        }
        i += 1;
    }
    Err(AnalysisError::Failed)
}
fn terminated(b: &[u8], mut i: usize, end: &[u8]) -> Result<usize, AnalysisError> {
    while i + end.len() <= b.len() {
        if b[i..].starts_with(end) {
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
fn tag(source: &str, start: usize) -> Result<Tag, AnalysisError> {
    let b = source.as_bytes();
    let closing = b[start + 1] == b'/';
    let mut i = start + 1 + usize::from(closing);
    let n = i;
    while b
        .get(i)
        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b':'))
    {
        i += 1;
    }
    let name = source[n..i].to_ascii_lowercase();
    let mut attrs = HashMap::new();
    loop {
        while b
            .get(i)
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b'/')
        {
            i += 1;
        }
        match b.get(i) {
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
        while b
            .get(i)
            .is_some_and(|c| !c.is_ascii_whitespace() && !matches!(c, b'=' | b'>' | b'/'))
        {
            i += 1;
        }
        let attr = source[n..i].to_ascii_lowercase();
        while b.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
            i += 1;
        }
        let mut value = "";
        if b.get(i) == Some(&b'=') {
            i += 1;
            while b.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
                i += 1;
            }
            let quote = b.get(i).copied().ok_or(AnalysisError::Failed)?;
            if matches!(quote, b'\'' | b'"') {
                i += 1;
                let n = i;
                while b.get(i).is_some_and(|c| *c != quote) {
                    i += 1;
                }
                value = &source[n..i];
                if i == b.len() {
                    return Err(AnalysisError::Failed);
                }
                i += 1;
            } else {
                let n = i;
                while b
                    .get(i)
                    .is_some_and(|c| !c.is_ascii_whitespace() && *c != b'>')
                {
                    i += 1;
                }
                value = &source[n..i];
            }
        }
        // Empty malformed attribute names (e.g. '=x') still consume input.
        attrs.entry(attr).or_insert_with(|| decode_refs(value));
    }
}
fn raw_end(source: &str, mut i: usize, name: &str) -> Result<(usize, usize), AnalysisError> {
    let b = source.as_bytes();
    while i < b.len() {
        if b[i..].starts_with(b"</") {
            let end = i + 2 + name.len();
            if b.get(i + 2..end)
                .is_some_and(|s| s.eq_ignore_ascii_case(name.as_bytes()))
                && b.get(end)
                    .is_some_and(|c| c.is_ascii_whitespace() || *c == b'>')
            {
                return Ok((i, tag(source, i)?.end));
            }
        }
        i += 1;
    }
    Err(AnalysisError::Failed)
}
fn remote_small(attrs: &HashMap<String, String>, min: u32, max: u32) -> bool {
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

    fn raw(body: &str) -> Vec<u8> {
        format!("Content-Type: text/html; charset=utf-8\r\n\r\n{body}").into_bytes()
    }
    fn report(body: &str) -> Report {
        evaluate(&raw(body), &Inputs::default()).unwrap()
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
            "swarmail-html/1"
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
        // Byte-accounted cursor progress, including long inert/raw and entity runs.
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
        assert_eq!(r.findings.len(), 4);
        for pair in r.findings.chunks(2) {
            assert_eq!(pair[0].location, Location::MimePart { path: vec![0] });
            assert_eq!(pair[1].location, Location::MimePart { path: vec![1, 1] });
        }
        assert!(
            evaluate(&text, &Inputs::default())
                .unwrap()
                .findings
                .is_empty()
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
                .is_empty()
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
                .is_empty()
        );
        let attached_message =
            [b"Content-Type: message/rfc822\r\n\r\n".as_slice(), &image].concat();
        assert!(
            evaluate(&attached_message, &Inputs::default())
                .unwrap()
                .findings
                .is_empty()
        );
        let inline = b"Content-Type: text/html\r\nContent-Disposition: inline\r\n\r\n<video>";
        assert_eq!(
            evaluate(inline, &Inputs::default()).unwrap().findings.len(),
            1
        );
        for (encoding, body) in [
            ("base64", STANDARD.encode("<video>")),
            ("quoted-printable", "=3Cvideo=3E".into()),
        ] {
            assert_eq!(evaluate(format!("Content-Type: text/html\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n{body}").as_bytes(), &Inputs::default()).unwrap(), report("<video>"));
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
                .is_empty()
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
                assert_eq!(
                    r.findings.len(),
                    usize::from(size > 104448),
                    "{size} {encoding}"
                );
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
        // Two contributions per HTML part; exact 256 succeeds before dedup.
        for n in [128, 129] {
            let message = multipart(&vec![raw("<img>"); n], "findings", "");
            let r = evaluate(&message, &Inputs::default());
            assert_eq!(r.is_ok(), n == 128);
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
        let mut email = crate::store::email("bad-html", "box", "a");
        email.raw = raw("<video><script>");
        let email = std::sync::Arc::new(email);
        for _ in 0..2 {
            assert_eq!(
                service.report(email.clone(), false).await,
                Err(AnalysisError::Failed)
            );
        }
        let slot = service.slots.lock().unwrap()[0].clone();
        assert!(slot.report.lock().await.is_none());
    }
}
