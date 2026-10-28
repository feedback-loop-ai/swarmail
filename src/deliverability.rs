//! Lazy analysis of immutable stored bytes. This is separate from ingest extraction.
//! No network, clock, persistence or Store guard enters the evaluator (0085).

use crate::{model::Email, store::Store};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};

mod html;

pub const RULE_SET_ID: &str = "swarmail-html/3";
pub const MAX_FINDINGS: usize = 256;
pub const MAX_REPORT_BYTES: usize = 65_536;
const MAX_HEADER_BYTES: usize = 65_536;
const MAX_RAW_BYTES: usize = 8_388_608;
const MAX_AGGREGATE_HEADERS: usize = 262_144;
const MAX_PARTS: usize = 256;
const MAX_DEPTH: usize = 32;
const CACHE_SLOTS: usize = 128;
const JOBS: usize = 4;

/// Declaration order is the inclusive threshold order, not lexical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Coordinates describe source bytes; link locators never contain a URL.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Location {
    Header {
        name: String,
        occurrence: Option<u64>,
    },
    MimePart {
        path: Vec<u64>,
    },
    Link {
        part_path: Vec<u64>,
        byte_offset: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub rule_id: String,
    pub location: Location,
    pub severity: Severity,
    pub fix_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub rule_set_id: String,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn qualifies(&self, threshold: Severity) -> bool {
        !self.findings.iter().any(|f| f.severity >= threshold)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisConfig {
    pub demo_enabled: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self { demo_enabled: true }
    }
}

/// Complete effective inputs, including immutable trusted facts and their identity.
/// #253/#255 own observation expiry, refresh and selected-mail provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Inputs {
    config: AnalysisConfig,
    rule_set_id: &'static str,
    auth: AuthEvidence,
    #[cfg(test)]
    fixture: Vec<Finding>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            config: AnalysisConfig::default(),
            rule_set_id: RULE_SET_ID,
            auth: AuthEvidence::default(),
            #[cfg(test)]
            fixture: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisError {
    Unavailable,
    Failed,
}

impl AnalysisError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Unavailable => "deliverability analysis unavailable",
            Self::Failed => "deliverability analysis failed",
        }
    }
}

/// All collectors share these bounds and the single canonicalizer. Contributions
/// are checked individually, before an unbounded temporary vector can accumulate.
struct Collector(Vec<Finding>);

impl Collector {
    fn push(&mut self, finding: Finding) -> Result<(), AnalysisError> {
        if self.0.len() == MAX_FINDINGS {
            return Err(AnalysisError::Failed);
        }
        self.0.push(finding);
        Ok(())
    }

    fn finish(mut self, inputs: &Inputs) -> Result<Report, AnalysisError> {
        self.0.sort();
        self.0.dedup();
        let report = Report {
            rule_set_id: inputs.rule_set_id.into(),
            findings: self.0,
        };
        // Report contains only infallibly serializable strings, enums and integers.
        if serde_json::to_vec(&report).unwrap().len() > MAX_REPORT_BYTES {
            return Err(AnalysisError::Failed);
        }
        Ok(report)
    }
}

/// Immutable trusted observations; sender headers never populate these facts.
/// #255 owns verification, From validity, alignment, provenance and mail binding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AuthEvidence {
    policy_id: String,
    observation_id: String,
    policy_present: Fact,
    valid_from: Fact,
    dkim_pass: Fact,
    dkim_aligned: Fact,
    spf_pass: Fact,
    spf_aligned: Fact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Fact {
    True,
    False,
    #[default]
    Unknown,
}
impl Fact {
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
}
impl AuthEvidence {
    fn rules(&self) -> [Fact; 3] {
        [
            self.policy_present,
            self.valid_from.and(self.dkim_aligned.or(self.spf_aligned)),
            self.valid_from.and(
                self.dkim_pass
                    .and(self.dkim_aligned)
                    .or(self.spf_pass.and(self.spf_aligned)),
            ),
        ]
    }
}

type Headers = Vec<(Vec<u8>, Vec<u8>)>;

/// One bounded scanner for root and MIME headers. Continuations belong only to
/// the immediately preceding valid field. A missing separator is incomplete MIME.
fn headers(raw: &[u8]) -> Result<(Headers, usize), AnalysisError> {
    let mut fields: Headers = Vec::new();
    let mut continuing = false;
    let mut consumed = 0;
    for chunk in raw[..raw.len().min(MAX_HEADER_BYTES)].split_inclusive(|b| *b == b'\n') {
        consumed += chunk.len();
        let line = chunk
            .strip_suffix(b"\r\n")
            .or_else(|| chunk.strip_suffix(b"\n"))
            .unwrap_or(chunk);
        if line.is_empty() {
            return Ok((fields, consumed));
        }
        if matches!(line.first(), Some(b' ' | b'\t')) {
            if continuing {
                let value = &mut fields.last_mut().unwrap().1;
                value.push(b' ');
                // Trim only FWS; other controls must remain invalid bytes.
                value.extend_from_slice(
                    &line[line
                        .iter()
                        .position(|b| !matches!(b, b' ' | b'\t'))
                        .unwrap_or(line.len())..],
                );
            }
            continue;
        }
        continuing = false;
        if let Some(colon) = line.iter().position(|b| *b == b':') {
            let name = &line[..colon];
            if name != trim_value(name)
                && [
                    b"content-type".as_slice(),
                    b"content-disposition",
                    b"content-transfer-encoding",
                ]
                .iter()
                .any(|n| trim_value(name).eq_ignore_ascii_case(n))
            {
                return Err(AnalysisError::Failed);
            }
            if !name.is_empty() && name.iter().all(|b| (33..=126).contains(b)) {
                fields.push((name.to_ascii_lowercase(), line[colon + 1..].to_vec()));
                continuing = true;
            }
        } else if [
            b"content-type".as_slice(),
            b"content-disposition",
            b"content-transfer-encoding",
        ]
        .iter()
        .any(|n| {
            line.split(|b| matches!(b, b' ' | b'\t'))
                .next()
                .unwrap()
                .eq_ignore_ascii_case(n)
        }) {
            return Err(AnalysisError::Failed);
        }
    }
    Err(AnalysisError::Failed)
}

fn values<'a>(fields: &'a Headers, name: &'a [u8]) -> impl Iterator<Item = &'a [u8]> {
    fields
        .iter()
        .filter(move |(n, _)| n == name)
        .map(|(_, v)| trim_value(v))
}

fn header_location(fields: &Headers, name: &str) -> Location {
    let count = values(fields, name.as_bytes()).take(2).count();
    Location::Header {
        name: name.into(),
        occurrence: match count {
            0 => None,
            1 => Some(0),
            _ => Some(1),
        },
    }
}

fn singleton(fields: &Headers, name: &[u8], valid: fn(&[u8]) -> bool) -> bool {
    let mut found = values(fields, name);
    match (found.next(), found.next()) {
        (Some(value), None) => valid(value),
        _ => false,
    }
}

/// Strip only legal CFWS comments, retaining separators so comments cannot join
/// two tokens. Current Message-ID has no quoted local part; Date has no quotes.
fn without_comments(value: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut escaped = false;
    let mut literal = false;
    for &b in value {
        if !(b == b'\t' || (32..=126).contains(&b)) {
            return None;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if depth == 0 {
            match b {
                b'[' => literal = true,
                b']' => literal = false,
                _ => {}
            }
        }
        match b {
            b'\\' if depth > 0 => escaped = true,
            b'(' if !literal => {
                depth += 1;
                if depth == 1 {
                    out.push(b' ');
                }
            }
            b')' if !literal => {
                depth = depth.checked_sub(1)?;
            }
            _ if depth == 0 => out.push(b),
            _ => {}
        }
    }
    (depth == 0).then_some(out)
}

fn dot_atom(value: &[u8]) -> bool {
    !value.is_empty()
        && value.split(|b| *b == b'.').all(|atom| {
            !atom.is_empty()
                && atom
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-/=?^_`{|}~".contains(b))
        })
}

fn message_id(value: &[u8]) -> bool {
    let Some(value) = without_comments(value) else {
        return false;
    };
    let value = trim_value(&value);
    let Some(inner) = value.strip_prefix(b"<").and_then(|v| v.strip_suffix(b">")) else {
        return false;
    };
    let Some(at) = inner.iter().position(|b| *b == b'@') else {
        return false;
    };
    let right = &inner[at + 1..];
    let literal = right.strip_prefix(b"[").and_then(|r| r.strip_suffix(b"]"));
    dot_atom(&inner[..at])
        && match literal {
            Some(v) => v
                .iter()
                .all(|b| (33..=90).contains(b) || (94..=126).contains(b)),
            None => dot_atom(right),
        }
}

fn number(value: &str, min: usize, max: usize) -> Option<u32> {
    if !(min..=max).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn date(value: &[u8]) -> bool {
    use chrono::Datelike;
    let Some(value) = without_comments(value) else {
        return false;
    };
    let value = std::str::from_utf8(&value).unwrap(); // ASCII was checked above.
    let mut words: Vec<_> = value.split_ascii_whitespace().collect();
    let weekday = if words.first().is_some_and(|w| w.ends_with(',')) {
        Some(words.remove(0))
    } else {
        None
    };
    let [day, month, year, time, zone] = words.as_slice() else {
        return false;
    };
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|m| m.eq_ignore_ascii_case(month));
    let (Some(day), Some(month)) = (number(day, 1, 2), month) else {
        return false;
    };
    if year.len() < 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let significant = year.trim_start_matches('0');
    if significant.len() < 4 || (significant.len() == 4 && significant < "1900") {
        return false;
    }
    // The Gregorian calendar repeats every 400 years. Reduce bounded source
    // digits before using chrono, avoiding integer overflow and its year ceiling.
    let cycle = year
        .bytes()
        .fold(0u32, |n, b| (n * 10 + u32::from(b - b'0')) % 400);
    let Some(calendar) =
        chrono::NaiveDate::from_ymd_opt(2000 + cycle as i32, month as u32 + 1, day)
    else {
        return false;
    };
    if let Some(w) = weekday {
        let expected = ["Mon,", "Tue,", "Wed,", "Thu,", "Fri,", "Sat,", "Sun,"]
            [calendar.weekday().num_days_from_monday() as usize];
        if !expected.eq_ignore_ascii_case(w) {
            return false;
        }
    }
    let clock: Vec<_> = time.split(':').collect();
    if !(2..=3).contains(&clock.len()) {
        return false;
    }
    let limits = [23, 59, 60];
    if !clock
        .iter()
        .zip(limits)
        .all(|(v, limit)| number(v, 2, 2).is_some_and(|n| n <= limit))
    {
        return false;
    }
    let bytes = zone.as_bytes();
    bytes.len() == 5
        && matches!(bytes[0], b'+' | b'-')
        && number(&zone[1..3], 2, 2).is_some_and(|n| n <= 23)
        && number(&zone[3..], 2, 2).is_some_and(|n| n <= 59)
}

fn uri(value: &[u8]) -> Option<bool> {
    // RFC 3986 ASCII characters only, and every percent escape is complete.
    let mut i = 0;
    while i < value.len() {
        let b = value[i];
        if b == b'%' {
            if !value.get(i + 1..i + 3)?.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            i += 3;
        } else {
            if !(b.is_ascii_alphanumeric() || b"-._~:/?#[]@!$&'()*+,;=".contains(&b)) {
                return None;
            }
            i += 1;
        }
    }
    let text = std::str::from_utf8(value).ok()?;
    let (scheme, rest) = text.split_once(':')?;
    if !scheme.as_bytes().first()?.is_ascii_alphabetic()
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
        || rest.is_empty()
    {
        return None;
    }
    let path = if let Some(after) = rest.strip_prefix("//") {
        let end = after.find(['/', '?', '#']).unwrap_or(after.len());
        &after[end..]
    } else {
        rest
    };
    if path.contains(['[', ']']) || rest.matches('#').count() > 1 {
        return None;
    }
    if !scheme.eq_ignore_ascii_case("https") {
        reqwest::Url::parse(text).ok()?;
        return Some(false);
    }
    let authority = rest.strip_prefix("//")?.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') || rest.contains('#') {
        return None;
    }
    // Local parsing only. The original grammar refuses whitespace and backslashes
    // that URL normalization might repair. No target is resolved or contacted.
    reqwest::Url::parse(text).ok()?;
    if authority.ends_with(':') {
        return None;
    }
    Some(true)
}

fn unsubscribe(value: &[u8]) -> bool {
    let mut rest = trim_value(value);
    let mut https = false;
    loop {
        let Some(after) = rest.strip_prefix(b"<") else {
            return false;
        };
        let Some(close) = after.iter().position(|b| *b == b'>') else {
            return false;
        };
        let Some(secure) = uri(&after[..close]) else {
            return false;
        };
        https |= secure;
        rest = trim_value(&after[close + 1..]);
        if rest.is_empty() {
            return https;
        }
        let Some(after_comma) = rest.strip_prefix(b",") else {
            return false;
        };
        rest = trim_value(after_comma);
    }
}

fn post_value(value: &[u8]) -> bool {
    value == b"List-Unsubscribe=One-Click"
}

/// Complete MIME metadata, without body decoding. Parameter names retain RFC2231
/// suffixes so filename metadata cannot evade attachment ancestry.
type MimeValue = (String, Vec<(String, String)>);
fn mime_value(value: &[u8]) -> Result<MimeValue, AnalysisError> {
    let fail = AnalysisError::Failed;
    let text = std::str::from_utf8(value).map_err(|_| fail)?;
    if !value.iter().all(|b| *b == b'\t' || (32..=126).contains(b)) {
        return Err(fail);
    }
    let (kind, tail) = text.split_once(';').unwrap_or((text, ""));
    let kind = kind.trim().to_ascii_lowercase();
    let token = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
    };
    if !kind.split('/').all(token) {
        return Err(fail);
    }
    if text.contains(';') && tail.trim().is_empty() {
        return Err(fail);
    }
    let mut params = Vec::new();
    let mut rest = tail.trim();
    while !rest.is_empty() {
        let (name, after) = rest.split_once('=').ok_or(fail)?;
        let name = name.trim().to_ascii_lowercase();
        if !token(&name) || params.iter().any(|(n, _)| n == &name) {
            return Err(fail);
        }
        rest = after.trim_start();
        let mut value = String::new();
        if let Some(quoted) = rest.strip_prefix('"') {
            let mut chars = quoted.char_indices();
            let end = loop {
                let (index, ch) = chars.next().ok_or(fail)?;
                match ch {
                    '"' => break index + 1,
                    '\\' => value.push(chars.next().ok_or(fail)?.1),
                    _ => value.push(ch),
                }
            };
            rest = &quoted[end..];
        } else {
            let (v, tail) = rest.split_once(';').unwrap_or((rest, ""));
            let v = v.trim();
            if !token(v) {
                return Err(fail);
            }
            value.push_str(v);
            if rest.contains(';') && tail.trim().is_empty() {
                return Err(fail);
            }
            rest = tail.trim_start();
            params.push((name, value));
            continue;
        }
        params.push((name, value));
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        rest = rest.strip_prefix(';').ok_or(fail)?.trim_start();
        if rest.is_empty() {
            return Err(fail);
        }
    }
    Ok((kind, params))
}

fn mime_field(fields: &Headers, name: &[u8]) -> Result<Option<MimeValue>, AnalysisError> {
    let mut v = values(fields, name);
    let first = v.next();
    if v.next().is_some() {
        return Err(AnalysisError::Failed);
    }
    first.map(mime_value).transpose()
}

fn multipart_children<'a>(body: &'a [u8], boundary: &[u8]) -> Result<Vec<&'a [u8]>, AnalysisError> {
    let fail = AnalysisError::Failed;
    if boundary.is_empty()
        || boundary.len() > 70
        || boundary.last() == Some(&b' ')
        || !boundary
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(b))
    {
        return Err(fail);
    }
    let mut delimiter = b"--".to_vec();
    delimiter.extend_from_slice(boundary);
    let mut children = Vec::new();
    let mut start = None;
    let mut offset = 0;
    for line in body.split_inclusive(|b| *b == b'\n') {
        let line_start = offset;
        offset += line.len();
        let line = line
            .strip_suffix(b"\r\n")
            .or_else(|| line.strip_suffix(b"\n"))
            .unwrap_or(line);
        let Some(tail) = line.strip_prefix(delimiter.as_slice()) else {
            continue;
        };
        let tail = &tail[..tail
            .iter()
            .rposition(|b| !matches!(b, b' ' | b'\t'))
            .map_or(0, |n| n + 1)];
        if !tail.is_empty() && tail != b"--" {
            continue;
        }
        if let Some(begin) = start {
            if children.len() == MAX_PARTS {
                return Err(fail);
            }
            children.push(&body[begin..line_start]);
        }
        if tail == b"--" {
            return if children.is_empty() {
                Err(fail)
            } else {
                Ok(children)
            };
        }
        start = Some(offset);
    }
    Err(fail)
}

fn plain_body(raw: &[u8], root: Headers, root_size: usize) -> Result<bool, AnalysisError> {
    let mut stack = vec![(raw, root, root_size, 0, false, false)];
    let mut aggregate = 0;
    let mut count = 0;
    let mut plain = false;
    while let Some((raw, fields, size, depth, ancestor, digest)) = stack.pop() {
        count += 1;
        aggregate += size;
        let (kind, params) = mime_field(&fields, b"content-type")?.unwrap_or_else(|| {
            (
                if digest {
                    "message/rfc822"
                } else {
                    "text/plain"
                }
                .into(),
                Vec::new(),
            )
        });
        if kind.split('/').count() != 2 {
            return Err(AnalysisError::Failed);
        }
        let disposition = mime_field(&fields, b"content-disposition")?;
        if disposition.as_ref().is_some_and(|(k, _)| k.contains('/')) {
            return Err(AnalysisError::Failed);
        }
        let attached = ancestor
            || disposition.as_ref().is_some_and(|(k, p)| {
                k == "attachment"
                    || p.iter()
                        .any(|(n, _)| n == "filename" || n.starts_with("filename*"))
            })
            || params
                .iter()
                .any(|(n, _)| n == "name" || n.starts_with("name*"));
        if kind.starts_with("multipart/") {
            let encodings: Vec<_> = values(&fields, b"content-transfer-encoding").collect();
            if encodings.len() > 1
                || encodings.first().is_some_and(|v| {
                    ![&b"7bit"[..], b"8bit", b"binary"]
                        .iter()
                        .any(|e| v.eq_ignore_ascii_case(e))
                })
            {
                return Err(AnalysisError::Failed);
            }
            if depth == MAX_DEPTH {
                return Err(AnalysisError::Failed);
            }
            let boundary = params
                .iter()
                .find(|(n, _)| n == "boundary")
                .ok_or(AnalysisError::Failed)?;
            let children = multipart_children(&raw[size..], boundary.1.as_bytes())?;
            if count + stack.len() + children.len() > MAX_PARTS {
                return Err(AnalysisError::Failed);
            }
            for child in children.into_iter().rev() {
                let (h, size) = headers(child)?;
                let pending: usize = stack.iter().map(|(_, _, n, _, _, _)| n).sum();
                if aggregate + pending + size > MAX_AGGREGATE_HEADERS {
                    return Err(AnalysisError::Failed);
                }
                stack.push((
                    child,
                    h,
                    size,
                    depth + 1,
                    attached,
                    kind == "multipart/digest",
                ));
            }
        } else if kind == "text/plain" && !attached {
            plain = true;
        }
    }
    Ok(plain)
}

const GMAIL: &str = "https://support.google.com/mail/answer/81126";
const YAHOO: &str = "https://senders.yahooinc.com/best-practices/";
const RULES: [(&str, &str, &str, &str); 7] = [
    (
        "one_click",
        "list-unsubscribe",
        "Provide one HTTPS List-Unsubscribe list and one exact List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check does not verify DKIM header coverage or endpoint operation.",
        "https://www.rfc-editor.org/rfc/rfc8058#section-3",
    ),
    (
        "message_id",
        "message-id",
        "Provide exactly one Message-ID using the supported current RFC 5322 syntax; obsolete-only forms are unsupported by this checklist.",
        "https://www.rfc-editor.org/rfc/rfc5322#section-3.6.4",
    ),
    (
        "date",
        "date",
        "Provide exactly one calendar-valid Date using current RFC 5322 syntax and a numeric zone; obsolete-only forms are unsupported by this checklist.",
        "https://www.rfc-editor.org/rfc/rfc5322#section-3.6.1",
    ),
    (
        "plain_text",
        "",
        "Include a sender-supplied text/plain body. This is checklist advice, not proof of equivalent content or an unconditional provider mandate.",
        "https://www.rfc-editor.org/rfc/rfc2046#section-5.1.4",
    ),
    (
        "dmarc_presence",
        "from",
        "Publish a valid applicable DMARC policy; p=none satisfies this presence check.",
        "https://www.rfc-editor.org/rfc/rfc7489#section-6.1",
    ),
    (
        "from_alignment",
        "from",
        "Align a DKIM d= or SPF MAIL FROM identity with an unambiguous RFC5322.From domain under the applicable DMARC policy.",
        "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
    ),
    (
        "authenticated_sender",
        "from",
        "Supply a passing DKIM or SPF result aligned with RFC5322.From. This scoped check does not certify every provider authentication requirement.",
        "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
    ),
];
const UNKNOWN_HINTS: [&str; 3] = [
    "Not evaluated: trusted applicable DMARC policy evidence is unavailable.",
    "Not evaluated: trusted From identity or DKIM/SPF alignment evidence is incomplete.",
    "Not evaluated: trusted From identity or passing-and-aligned DKIM/SPF evidence is incomplete.",
];

fn bulk(
    raw: &[u8],
    fields: &Headers,
    size: usize,
    inputs: &Inputs,
    out: &mut Collector,
) -> Result<(), AnalysisError> {
    let list_valid = singleton(fields, b"list-unsubscribe", unsubscribe);
    let known = [
        list_valid && singleton(fields, b"list-unsubscribe-post", post_value),
        singleton(fields, b"message-id", message_id),
        singleton(fields, b"date", date),
        plain_body(raw, fields.clone(), size)?,
    ];
    let auth = inputs.auth.rules();
    for (i, (name, header, hint, source)) in RULES.iter().enumerate() {
        let fact = if i < known.len() {
            if known[i] { Fact::True } else { Fact::False }
        } else {
            auth[i - known.len()]
        };
        if fact == Fact::True {
            continue;
        }
        let unknown = fact == Fact::Unknown;
        let hint = if unknown {
            UNKNOWN_HINTS[i - known.len()]
        } else {
            hint
        };
        let header = if i == 0 && list_valid {
            "list-unsubscribe-post"
        } else {
            header
        };
        out.push(Finding {
            rule_id: format!(
                "bulk.{name}.{}v1",
                if unknown { "not_evaluated." } else { "" }
            ),
            location: if i == 3 {
                Location::MimePart { path: Vec::new() }
            } else {
                header_location(fields, header)
            },
            severity: if i == 3 {
                Severity::Warning
            } else {
                Severity::Error
            },
            fix_hint: format!("{hint} Sources: {source} {GMAIL} {YAHOO}"),
        })?;
    }
    Ok(())
}

fn trim_value(mut value: &[u8]) -> &[u8] {
    while matches!(value.first(), Some(b' ' | b'\t')) {
        value = &value[1..];
    }
    while matches!(value.last(), Some(b' ' | b'\t')) {
        value = &value[..value.len() - 1];
    }
    value
}

fn collect(raw: &[u8], inputs: &Inputs, out: &mut Collector) -> Result<(), AnalysisError> {
    if raw.len() > MAX_RAW_BYTES {
        return Err(AnalysisError::Failed);
    }
    let (fields, size) = headers(raw)?;
    let value = values(&fields, b"x-swarmail-demo").next();
    if inputs.config.demo_enabled {
        let severity = match value {
            Some(b"info") => Some(Severity::Info),
            Some(b"warning") => Some(Severity::Warning),
            Some(b"error") => Some(Severity::Error),
            _ => None,
        };
        if let Some(severity) = severity {
            out.push(Finding {
                rule_id: "demo.marker.v1".into(),
                location: Location::Header {
                    name: "x-swarmail-demo".into(),
                    occurrence: Some(0),
                },
                severity,
                fix_hint: "Remove the X-Swarmail-Demo header.".into(),
            })?;
        }
    }
    bulk(raw, &fields, size, inputs, out)?;
    html::collect(raw, out)?;
    #[cfg(test)]
    for finding in &inputs.fixture {
        out.push(finding.clone())?;
    }
    Ok(())
}

fn evaluate(raw: &[u8], inputs: &Inputs) -> Result<Report, AnalysisError> {
    let mut out = Collector(Vec::new());
    collect(raw, inputs, &mut out)?;
    out.finish(inputs)
}

struct Slot {
    email: Weak<Email>,
    inputs: Inputs,
    report: Arc<AsyncMutex<Option<Arc<Report>>>>,
}

/// Short bookkeeping lock, async per-key lock and worker-owned reservations.
/// The deque's Arc is the only unpinned owner; Weak keys retain no mail bytes.
pub(crate) struct Service {
    slots: Mutex<VecDeque<Arc<Slot>>>,
    jobs: Arc<Semaphore>,
    #[cfg(test)]
    pub(crate) gate: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    #[cfg(test)]
    auth: std::sync::OnceLock<AuthEvidence>,
}

impl Default for Service {
    fn default() -> Self {
        Self {
            slots: Mutex::new(VecDeque::new()),
            jobs: Arc::new(Semaphore::new(JOBS)),
            #[cfg(test)]
            gate: Mutex::new(None),
            #[cfg(test)]
            auth: std::sync::OnceLock::new(),
        }
    }
}

impl Service {
    fn pin(&self, email: &Arc<Email>, inputs: &Inputs) -> Result<Arc<Slot>, AnalysisError> {
        let mut slots = self.slots.lock().unwrap();
        if let Some(slot) = slots.iter().find(|slot| {
            slot.inputs == *inputs
                && slot
                    .email
                    .upgrade()
                    .is_some_and(|old| Arc::ptr_eq(&old, email))
        }) {
            return Ok(slot.clone());
        }
        if slots.len() == CACHE_SLOTS {
            let index = slots
                .iter()
                .position(|s| Arc::strong_count(s) == 1)
                .ok_or(AnalysisError::Unavailable)?;
            slots.remove(index);
        }
        let slot = Arc::new(Slot {
            email: Arc::downgrade(email),
            inputs: inputs.clone(),
            report: Arc::new(AsyncMutex::new(None)),
        });
        slots.push_back(slot.clone());
        Ok(slot)
    }

    async fn with_inputs(
        &self,
        email: Arc<Email>,
        inputs: Inputs,
        ready_only: bool,
    ) -> Result<Arc<Report>, AnalysisError> {
        let slot = self.pin(&email, &inputs)?;
        let mut guard = if ready_only {
            slot.report
                .clone()
                .try_lock_owned()
                .map_err(|_| AnalysisError::Unavailable)?
        } else {
            slot.report.clone().lock_owned().await
        };
        if let Some(report) = &*guard {
            return Ok(report.clone());
        }
        if ready_only {
            return Err(AnalysisError::Unavailable);
        }
        let permit = self
            .jobs
            .clone()
            .try_acquire_owned()
            .map_err(|_| AnalysisError::Unavailable)?;
        #[cfg(test)]
        let gate = self.gate.lock().unwrap().clone();
        Store::offload(move || {
            // Keep every reservation in the actual worker through publication,
            // cancellation and unwind. Dropping the caller cannot refund a job.
            let _permit = permit;
            let _pin = slot;
            #[cfg(test)]
            if let Some(gate) = gate {
                gate();
            }
            let report = Arc::new(evaluate(&email.raw, &inputs)?);
            *guard = Some(report.clone());
            Ok(report)
        })
        .await
    }

    pub(crate) async fn report(
        &self,
        email: Arc<Email>,
        ready_only: bool,
    ) -> Result<Arc<Report>, AnalysisError> {
        let inputs = Inputs::default();
        #[cfg(test)]
        let inputs = Inputs {
            auth: self.auth.get().cloned().unwrap_or_default(),
            ..inputs
        };
        self.with_inputs(email, inputs, ready_only).await
    }
}

#[cfg(test)]
mod concurrency_tests;

#[cfg(test)]
mod tests {
    use super::concurrency_tests::{GOOD, config, get, mail, post, smtp, smtp_bytes};
    use super::*;
    use crate::store::Filter;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn finding(location: Location, severity: Severity) -> Finding {
        Finding {
            rule_id: "fixture.v1".into(),
            location,
            severity,
            fix_hint: "Fix fixture.".into(),
        }
    }

    #[test]
    fn report_contract_variants_and_order() {
        let locations = vec![
            Location::Header {
                name: "a".into(),
                occurrence: None,
            },
            Location::Header {
                name: "a".into(),
                occurrence: Some(0),
            },
            Location::Header {
                name: "a".into(),
                occurrence: Some(2),
            },
            Location::Header {
                name: "b".into(),
                occurrence: Some(0),
            },
            Location::MimePart { path: vec![] },
            Location::MimePart { path: vec![0, 2] },
            Location::MimePart { path: vec![0, 10] },
            Location::Link {
                part_path: vec![0],
                byte_offset: 2,
            },
            Location::Link {
                part_path: vec![0],
                byte_offset: 10,
            },
            Location::Link {
                part_path: vec![1],
                byte_offset: 0,
            },
        ];
        let mut expected = Vec::new();
        for location in locations {
            for severity in [Severity::Info, Severity::Warning, Severity::Error] {
                expected.push(finding(location.clone(), severity));
            }
        }
        let mut later_hint = expected.last().unwrap().clone();
        later_hint.fix_hint = "Z".into();
        expected.push(later_hint);
        let mut later_rule = expected[0].clone();
        later_rule.rule_id = "z.v1".into();
        expected.push(later_rule);
        let mut reversed = expected.clone();
        reversed.reverse();
        reversed.extend(expected.clone());
        let report = Collector(reversed).finish(&Inputs::default()).unwrap();
        assert_eq!(report.findings, expected);
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(
            value["findings"][0],
            json!({"rule_id":"fixture.v1","severity":"info",
            "location":{"kind":"header","name":"a","occurrence":null},"fix_hint":"Fix fixture."})
        );
        assert_eq!(
            value["findings"][12]["location"],
            json!({"kind":"mime_part","path":[]})
        );
        assert_eq!(
            value["findings"][21]["location"],
            json!({"kind":"link","part_path":[0],"byte_offset":2})
        );
        assert_eq!(serde_json::from_value::<Report>(value).unwrap(), report);
        assert!(!report.qualifies(Severity::Error));
        let empty = Collector(Vec::new()).finish(&Inputs::default()).unwrap();
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            json!({"rule_set_id":"swarmail-html/3","findings":[]})
        );
        assert!(empty.qualifies(Severity::Info));
    }

    #[test]
    fn empty_html_keeps_later_collectors_and_finish_bounds() {
        let mut out = Collector(Vec::new());
        html::collect(b"", &mut out).unwrap();
        let f = finding(
            Location::Header {
                name: "fixture".into(),
                occurrence: None,
            },
            Severity::Warning,
        );
        out.push(f.clone()).unwrap();
        assert_eq!(
            out.finish(&Inputs::default()).unwrap().findings,
            vec![f.clone()]
        );
        let mut bounded = Collector(Vec::new());
        html::collect(b"", &mut bounded).unwrap();
        for _ in 0..MAX_FINDINGS {
            bounded.push(f.clone()).unwrap();
        }
        assert_eq!(bounded.push(f.clone()), Err(AnalysisError::Failed));
        let mut too_large = f;
        too_large.fix_hint = "x".repeat(MAX_REPORT_BYTES);
        assert_eq!(
            Collector(vec![too_large]).finish(&Inputs::default()),
            Err(AnalysisError::Failed)
        );
        assert_eq!(
            evaluate(b"", &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }

    #[test]
    fn report_limits_refuse_without_truncation() {
        let input = Inputs::default();
        let f = finding(Location::MimePart { path: vec![] }, Severity::Info);
        let mut out = Collector(Vec::new());
        for _ in 0..MAX_FINDINGS {
            out.push(f.clone()).unwrap();
        }
        assert_eq!(out.push(f.clone()), Err(AnalysisError::Failed));
        assert_eq!(out.finish(&input).unwrap().findings.len(), 1);
        let mut f = f;
        let overhead = serde_json::to_vec(&Collector(vec![f.clone()]).finish(&input).unwrap())
            .unwrap()
            .len()
            - f.fix_hint.len();
        f.fix_hint = "x".repeat(MAX_REPORT_BYTES - overhead);
        let report = Collector(vec![f.clone()]).finish(&input).unwrap();
        assert_eq!(serde_json::to_vec(&report).unwrap().len(), MAX_REPORT_BYTES);
        f.fix_hint.push('x');
        assert_eq!(
            Collector(vec![f]).finish(&input),
            Err(AnalysisError::Failed)
        );
        let inputs = Inputs {
            fixture: vec![
                finding(Location::MimePart { path: vec![] }, Severity::Info);
                MAX_FINDINGS + 1
            ],
            ..input
        };
        assert_eq!(evaluate(b"\n", &inputs), Err(AnalysisError::Failed));
    }

    #[test]
    fn demo_marker_uses_first_raw_header() {
        for (raw, expected) in [
            (&b""[..], None),
            (b"\r\nX-Swarmail-Demo: error\r\n", None),
            (b"bad\n orphan\nX-Swarmail-Demo : error\n: error\n\n", None),
            (b"X-Swarmail-Demo: INFO\n", None),
            (b"X-Swarmail-Demo: info\r", None),
            (b"X-Swarmail-Demo:\n", None),
            (b"X-Swarmail-Demo: \xff\n", None),
            (b"X-\xff: error\n", None),
            (b"X-Swarmail-Demo: info\x0b\n", None),
            (b"X-Swarmail-Demo: info\x0c\n", None),
            (
                b"X-Swarmail-Demo: info\nX-Swarmail-Demo: error\n",
                Some(Severity::Info),
            ),
            (
                b"x-SWARMaiL-demo:\r\n \t warning \t\r\n\r\nbody",
                Some(Severity::Warning),
            ),
            (b"X-Swarmail-Demo: error", Some(Severity::Error)),
            (b"X-Swarmail-Demo: info\n malformed\n", None),
            (
                b"X-Swarmail-Demo: info\nbroken\n ignored\n",
                Some(Severity::Info),
            ),
            (
                b"X-Swarmail-Demo: info\nOther: x\n ignored\n",
                Some(Severity::Info),
            ),
            (b"X-Swarmail-Demo: info\n \t\n", Some(Severity::Info)),
            (
                b"\torphan\nX-Swarmail-Demo: \tinfo\t \n",
                Some(Severity::Info),
            ),
        ] {
            let mut complete = raw.to_vec();
            if !complete.ends_with(b"\n") {
                complete.extend_from_slice(b"\r\n");
            }
            complete.extend_from_slice(b"\n");
            let mut report = evaluate(&complete, &Inputs::default()).unwrap();
            report.findings.retain(|f| f.rule_id == "demo.marker.v1");
            assert_eq!(
                report.findings.first().map(|f| f.severity),
                expected,
                "{raw:?}"
            );
            if let Some(f) = report.findings.first() {
                assert_eq!(f.rule_id, "demo.marker.v1");
                assert_eq!(f.fix_hint, "Remove the X-Swarmail-Demo header.");
                assert_eq!(
                    f.location,
                    Location::Header {
                        name: "x-swarmail-demo".into(),
                        occurrence: Some(0)
                    }
                );
            }
        }
        assert!(
            evaluate(
                b"X-Swarmail-Demo: error\n\n",
                &Inputs {
                    config: AnalysisConfig {
                        demo_enabled: false
                    },
                    ..Inputs::default()
                }
            )
            .unwrap()
            .findings
            .iter()
            .all(|f| f.rule_id != "demo.marker.v1")
        );
        let mut full = Collector(vec![
            finding(
                Location::MimePart { path: vec![] },
                Severity::Info
            );
            MAX_FINDINGS
        ]);
        assert_eq!(
            collect(b"X-Swarmail-Demo: error\n\n", &Inputs::default(), &mut full),
            Err(AnalysisError::Failed)
        );
        assert_eq!(
            collect(b"\n", &Inputs::default(), &mut full),
            Err(AnalysisError::Failed)
        );
    }

    #[test]
    fn demo_header_budget_is_exact() {
        let mut raw = b"X-Swarmail-Demo: warning\r\nOther: ".to_vec();
        raw.resize(MAX_HEADER_BYTES - 4, b'x');
        raw.extend_from_slice(b"\r\n\r\n");
        assert_eq!(raw.len(), MAX_HEADER_BYTES);
        assert_eq!(
            evaluate(&raw, &Inputs::default()).unwrap().findings.len(),
            7
        );
        raw.extend_from_slice(&vec![0xff; MAX_HEADER_BYTES + 1]);
        assert_eq!(
            evaluate(&raw, &Inputs::default()).unwrap().findings.len(),
            7
        );
        raw.insert(31, b'x');
        for demo_enabled in [true, false] {
            assert_eq!(
                evaluate(
                    &raw,
                    &Inputs {
                        config: AnalysisConfig { demo_enabled },
                        ..Inputs::default()
                    }
                ),
                Err(AnalysisError::Failed)
            );
        }
        assert!(evaluate(&vec![b'x'; MAX_HEADER_BYTES], &Inputs::default()).is_err());
        assert_eq!(
            evaluate(&vec![b'x'; MAX_HEADER_BYTES + 1], &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }

    #[tokio::test]
    async fn cache_identity_covers_bytes_config_rules() {
        let service = Service::default();
        let mut email = super::concurrency_tests::html_email("same", "box", "a@x.io");
        email.raw = b"X-Swarmail-Demo: warning\n\n".to_vec();
        let email = Arc::new(email);
        let first = service.report(email.clone(), false).await.unwrap();
        assert!(Arc::ptr_eq(
            &first,
            &service.report(email.clone(), true).await.unwrap()
        ));
        let disabled = Inputs {
            config: AnalysisConfig {
                demo_enabled: false,
            },
            ..Inputs::default()
        };
        assert!(
            service
                .with_inputs(email.clone(), disabled, false)
                .await
                .unwrap()
                .findings
                .iter()
                .all(|f| f.rule_id != "demo.marker.v1")
        );
        let version = Inputs {
            rule_set_id: "fixture/2",
            ..Inputs::default()
        };
        assert_eq!(
            service
                .with_inputs(email.clone(), version, false)
                .await
                .unwrap()
                .rule_set_id,
            "fixture/2"
        );
        let mut independent = (*email).clone();
        independent.id = "different-arrival".into();
        independent.received_ms += 1000;
        independent.inbox = "elsewhere".into();
        let other = Service::default()
            .report(Arc::new(independent), false)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&other).unwrap()
        );
        assert_eq!(service.slots.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn cache_rejects_clone_reinsert_and_recovered_owner() {
        let store = Store::new(0);
        let mut e = super::concurrency_tests::html_email("same", "box", "a@x.io");
        e.raw = b"X-Swarmail-Demo: error\n\n".to_vec();
        let selected = store.insert(e).unwrap();
        let dirty = store.report(selected.clone()).await.unwrap();
        store.clear("box").unwrap();
        let mut owned = Arc::try_unwrap(selected).unwrap();
        owned.raw = b"\n".to_vec();
        let clean = store.insert(owned).unwrap();
        assert!(
            store
                .report(clean.clone())
                .await
                .unwrap()
                .findings
                .iter()
                .all(|f| f.rule_id != "demo.marker.v1")
        );
        let mut cloned = (*clean).clone();
        cloned.raw = b"X-Swarmail-Demo: info\n\n".to_vec();
        store.clear_all().unwrap();
        let replacement = store.insert(cloned).unwrap();
        assert_eq!(
            store
                .report(replacement)
                .await
                .unwrap()
                .findings
                .last()
                .unwrap()
                .severity,
            Severity::Info
        );
        let mut recovered = clean;
        Arc::make_mut(&mut recovered).raw = b"X-Swarmail-Demo: warning\n\n".to_vec();
        assert_eq!(
            store
                .report(recovered)
                .await
                .unwrap()
                .findings
                .last()
                .unwrap()
                .severity,
            Severity::Warning
        );
        assert_eq!(dirty.findings.last().unwrap().severity, Severity::Error);
    }

    #[tokio::test]
    async fn cache_eviction_never_retrieves_removed_mail() {
        let service = Service::default();
        let mut pinned = Vec::new();
        let input = Inputs::default();
        for n in 0..CACHE_SLOTS {
            pinned.push(
                service
                    .pin(
                        &Arc::new(super::concurrency_tests::html_email(
                            &n.to_string(),
                            "box",
                            "a",
                        )),
                        &input,
                    )
                    .unwrap(),
            );
        }
        let mail = Arc::new(super::concurrency_tests::html_email("next", "box", "a"));
        assert!(matches!(
            service.pin(&mail, &input),
            Err(AnalysisError::Unavailable)
        ));
        let oldest = Arc::downgrade(&pinned.remove(0));
        service.pin(&mail, &input).unwrap();
        assert!(oldest.upgrade().is_none());
        drop(pinned);
        assert_eq!(service.slots.lock().unwrap().len(), CACHE_SLOTS);
        let store = Store::new(1);
        let old = store
            .insert(super::concurrency_tests::html_email("old", "box", "a"))
            .unwrap();
        store.report(old.clone()).await.unwrap();
        let weak = Arc::downgrade(&old);
        store
            .insert(super::concurrency_tests::html_email("new", "box", "a"))
            .unwrap();
        assert!(store.get("old").is_none());
        store.report(old.clone()).await.unwrap();
        drop(old);
        assert!(weak.upgrade().is_none());
        store.delete("new").unwrap();
        assert!(store.get("new").is_none());
    }

    #[tokio::test]
    async fn report_provider_snapshot_is_deterministic() {
        let service = Service::default();
        let email = Arc::new(super::concurrency_tests::html_email("seam", "box", "a"));
        let part = finding(Location::MimePart { path: vec![1, 0] }, Severity::Warning);
        let link = finding(
            Location::Link {
                part_path: vec![1, 0],
                byte_offset: 17,
            },
            Severity::Info,
        );
        let inputs = Inputs {
            rule_set_id: "fixture/1",
            auth: super::concurrency_tests::passing(),
            fixture: vec![link.clone(), part.clone(), link],
            ..Inputs::default()
        };
        let report = service
            .with_inputs(email.clone(), inputs.clone(), false)
            .await
            .unwrap();
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0], part);
        assert!(!report.qualifies(Severity::Warning));
        assert!(report.qualifies(Severity::Error));
        assert!(Arc::ptr_eq(
            &report,
            &service
                .with_inputs(email.clone(), inputs, true)
                .await
                .unwrap()
        ));
        let changed = service
            .with_inputs(
                email,
                Inputs {
                    rule_set_id: "fixture/1",
                    auth: super::concurrency_tests::passing(),
                    ..Inputs::default()
                },
                false,
            )
            .await
            .unwrap();
        assert!(changed.findings.is_empty());
        assert_ne!(
            serde_json::to_vec(&report).unwrap(),
            serde_json::to_vec(&changed).unwrap()
        );
    }
    #[tokio::test]
    async fn report_panic_releases_owned_capacity() {
        let service = Arc::new(Service::default());
        *service.gate.lock().unwrap() = Some(Arc::new(|| panic!("fixture evaluation panic")));
        let s = service.clone();
        let email = mail("panic");
        let e = email.clone();
        assert!(
            tokio::spawn(async move { s.report(e, false).await })
                .await
                .unwrap_err()
                .is_panic()
        );
        assert_eq!(service.jobs.available_permits(), JOBS);
        *service.gate.lock().unwrap() = None;
        assert!(service.report(email, false).await.is_ok());
    }

    #[tokio::test]
    async fn report_failure_retries_without_clean_success() {
        let service = Service::default();
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        *service.gate.lock().unwrap() = Some(Arc::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        }));
        let mut e = (*mail("bad")).clone();
        e.raw = vec![b'x'; MAX_HEADER_BYTES + 1];
        let e = Arc::new(e);
        for _ in 0..2 {
            assert_eq!(
                service.report(e.clone(), false).await,
                Err(AnalysisError::Failed)
            );
        }
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let slot = service.slots.lock().unwrap()[0].clone();
        assert!(slot.report.lock().await.is_none());
        assert_eq!(
            AnalysisError::Failed.message(),
            "deliverability analysis failed"
        );
        assert_eq!(
            AnalysisError::Unavailable.message(),
            "deliverability analysis unavailable"
        );
    }

    #[tokio::test]
    async fn report_failed_scan_is_not_exact_or_clean() {
        let store = Store::new(0);
        let mut bad = super::concurrency_tests::html_email("bad", "box", "a");
        bad.raw = vec![b'x'; MAX_HEADER_BYTES + 1];
        store.insert(bad).unwrap();
        store
            .insert(super::concurrency_tests::html_email("clean", "box", "a"))
            .unwrap();
        assert_eq!(
            store
                .qualify(
                    store.list("box", &Filter::default()),
                    Severity::Error,
                    usize::MAX,
                    false
                )
                .await,
            Err(AnalysisError::Failed)
        );
        assert_eq!(store.watcher_count(), 0, "exact assert never subscribes");
        assert!(matches!(
            store
                .wait_for_report(
                    "box",
                    &Filter::default(),
                    2,
                    Duration::from_secs(5),
                    Some(Severity::Error)
                )
                .await,
            Err(AnalysisError::Failed)
        ));
    }

    #[tokio::test]
    async fn report_is_lazy_and_ingest_fields_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mail.db");
        let srv = crate::run_on(&config(Some(db.clone()))).await.unwrap();
        let id = smtp(&srv).await;
        post(
            &srv,
            "/api/v1/inboxes/seed/seed",
            serde_json::json!({"to":"a@x.io","text":"code 424242 https://x.io/"}),
        )
        .await;
        assert!(
            srv.store.reports.slots.lock().unwrap().is_empty(),
            "SMTP and seed must not evaluate reports"
        );
        get(&srv, "/").await;
        get(&srv, "/ui/inbox/default").await;
        assert!(
            srv.store.reports.slots.lock().unwrap().is_empty(),
            "list views must not warm analysis"
        );
        let original = srv.store.get(&id).unwrap();
        assert_eq!(
            get(&srv, &format!("/api/v1/messages/{id}/raw"))
                .await
                .1
                .as_bytes(),
            original.raw
        );
        let count = Arc::new(AtomicUsize::new(0));
        let n = count.clone();
        *srv.store.reports.gate.lock().unwrap() = Some(Arc::new(move || {
            n.fetch_add(1, Ordering::SeqCst);
        }));
        for path in [
            format!("/api/v1/messages/{id}/deliverability?unknown=x"),
            "/api/v1/inboxes/default/assert?no_findings_at_or_above=Warning".into(),
            "/api/v1/inboxes/default/await?no_findings_at_or_above=".into(),
        ] {
            assert_eq!(get(&srv, &path).await.0, 400);
        }
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let first = get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await;
        assert_eq!(first.0, 200);
        assert_eq!(
            get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await,
            first
        );
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(original, srv.store.get(&id).unwrap());
        srv.stop().await;
        let srv = crate::run_on(&config(Some(db))).await.unwrap();
        assert!(
            srv.store.reports.slots.lock().unwrap().is_empty(),
            "restore must not evaluate"
        );
        assert_eq!(
            get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await,
            first
        );
        assert_eq!(original, srv.store.get(&id).unwrap());
        srv.stop().await;
    }
    #[test]
    fn bulk_auth_truth_tables() {
        use Fact::{False as F, True as T, Unknown as U};
        for (a, b, and, or) in [
            (T, T, T, T),
            (T, F, F, T),
            (T, U, U, T),
            (F, T, F, T),
            (F, F, F, F),
            (F, U, F, U),
            (U, T, U, T),
            (U, F, F, U),
            (U, U, U, U),
        ] {
            assert_eq!(a.and(b), and);
            assert_eq!(a.or(b), or);
        }
        // Exhaust all six independent facts; the reference uses possibility
        // sets, not the evaluator's AND/OR functions.
        for policy in [T, F, U] {
            for from in [T, F, U] {
                for dp in [T, F, U] {
                    for da in [T, F, U] {
                        for sp in [T, F, U] {
                            for sa in [T, F, U] {
                                let evidence = AuthEvidence {
                                    policy_present: policy,
                                    valid_from: from,
                                    dkim_pass: dp,
                                    dkim_aligned: da,
                                    spf_pass: sp,
                                    spf_aligned: sa,
                                    ..AuthEvidence::default()
                                };
                                let possibilities = |v| match v {
                                    T => vec![true],
                                    F => vec![false],
                                    U => vec![false, true],
                                };
                                let mut alignment = Vec::new();
                                let mut authentication = Vec::new();
                                for f in possibilities(from) {
                                    for d in possibilities(da) {
                                        for s in possibilities(sa) {
                                            for p in possibilities(dp) {
                                                for q in possibilities(sp) {
                                                    alignment.push(f && (d || s));
                                                    authentication
                                                        .push(f && ((p && d) || (q && s)));
                                                }
                                            }
                                        }
                                    }
                                }
                                let fact =
                                    |v: Vec<bool>| match (v.contains(&true), v.contains(&false)) {
                                        (true, false) => T,
                                        (false, true) => F,
                                        _ => U,
                                    };
                                assert_eq!(
                                    evidence.rules(),
                                    [policy, fact(alignment), fact(authentication)]
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn bulk_header_grammar() {
        for v in [
            "<a@b>",
            "(pre (nested) \\)) <a.b+z@example.test> (post)",
            "<a@[127.0.0.1]>",
            "<a@[IPv6:::1]>",
            "<a@[x(y)]>",
            "<a@[]>",
            "\t<a@b>\t",
        ] {
            assert!(message_id(v.as_bytes()), "{v}");
        }
        for v in [
            "",
            "a@b",
            "<a>",
            "<@b>",
            "<a@>",
            "<a..b@x>",
            "<.a@x>",
            "<a@b.>",
            "<a@b..c>",
            "<a@b@c>",
            "<a@b> <c@d>",
            "<a@b>junk",
            "(x)<a@b>)",
            "(x<a@b>",
            "<a@[x y]>",
            "<a@[x\\y]>",
            "<a@[x]z>",
            "<\"a\"@x>",
            "<a (x)@b>",
            "<a@b>\r",
            "<a@b>\0",
            "<a@b>\u{7f}",
            "<a@b>é",
            "(bad\\)",
            "(x\\",
        ] {
            assert!(!message_id(v.as_bytes()), "{v:?}");
        }
        for v in [
            "Mon, 26 Oct 2026 12:00:00 +0000",
            "Mon, 26 Oct 000000002026 12:00 +0000",
            "1 Jan 999999999 12:00 +0000",
            "29 Feb 1000000000000000 12:00 +0000",
            "26 oct 2026 12:00 -0000",
            "(pre) 29 Feb 2000 23:59:60 -2359 (post)",
            "Sun, 1 Mar 2026 00:00 +0000",
            "26\tOct\t2026\t12:00\t+0000",
        ] {
            assert!(date(v.as_bytes()), "{v}");
        }
        for v in [
            "",
            "26 Oct 2026 12:00",
            "26 Oct 2026 12:00 +0000 junk",
            "Tue, 26 Oct 2026 12:00 +0000",
            "26 Xxx 2026 12:00 +0000",
            "x Oct 2026 12:00 +0000",
            "26 Oct 26 12:00 +0000",
            "26 Oct 1899 12:00 +0000",
            "26 Oct 0000 12:00 +0000",
            "26 Oct 00001899 12:00 +0000",
            "26 Oct 20x6 12:00 +0000",
            "31 Apr 2026 12:00 +0000",
            "29 Feb 1900 12:00 +0000",
            "0 Jan 2026 12:00 +0000",
            "26 Oct 2026 12 +0000",
            "26 Oct 2026 12:00:00:00 +0000",
            "26 Oct 2026 24:00 +0000",
            "26 Oct 2026 12:60 +0000",
            "26 Oct 2026 12:00:61 +0000",
            "26 Oct 2026 1:00 +0000",
            "26 Oct 2026 12:00 +2400",
            "26 Oct 2026 12:00 +0060",
            "26 Oct 2026 12:00 +0a00",
            "26 Oct 2026 12:00 +00a0",
            "26 Oct 2026 12:00 00000",
            "26 Oct 2026 12:00 GMT",
            "26 Oct 2026 12:00 +0000\x0b",
            "26 Oct 2026 12:00 +0000é",
        ] {
            assert!(!date(v.as_bytes()), "{v:?}");
        }
        for v in [
            "<https://example.test/u>",
            "<HTTPS://example.test:443/u?q=%20>",
            "<mailto:leave@example.test>, <https://[::1]:443/u>",
            "<a+b.c-d:opaque>, <https://example.test>",
        ] {
            assert!(unsubscribe(v.as_bytes()), "{v}");
        }
        for v in [
            "",
            "https://x/u",
            "<https://x/u",
            "<mailto:a@x>",
            "<https://x>,",
            "<https://x>junk",
            "<https://x> <https://x>",
            "<https://x>, broken",
            "<https://x/%>",
            "<https://x/%0>",
            "<https://x/%zz>",
            "<https://x/a b>",
            "<https://x/\\>",
            "<https://x/é>",
            "<https://x/#fragment>",
            "<https://u:p@x/u>",
            "<https://>",
            "<https:x>",
            "<https:///x>",
            "<https://x:>",
            "<https://x:99999>",
            "<https://[bad]>",
            "<:foo>",
            "<1x:foo>",
            "<a_:foo>",
            "<x:>",
            "<x>",
            "<x:%00>",
            "<https://x>, <mailto:>",
        ] {
            // %00 is legal in an opaque non-HTTPS URI, but cannot alone pass.
            assert!(!unsubscribe(v.as_bytes()), "{v}");
        }
        assert!(post_value(b"List-Unsubscribe=One-Click"));
        assert!(!post_value(b"list-unsubscribe=One-Click"));
        assert!(!post_value(b"List-Unsubscribe=One-Click; extra=1"));
    }

    #[test]
    fn bulk_mime_metadata_grammar() {
        for v in [
            "text/plain",
            "TEXT/PLAIN; charset=utf-8",
            "text/plain;\tcharset=utf-8",
            "text/plain; name=\"a;b\"; charset=\"utf-8\"",
            "inline; filename=\"a\\\"b\"",
            "text/plain; charset=utf-8; format=flowed",
        ] {
            assert!(mime_value(v.as_bytes()).is_ok(), "{v}");
        }
        for v in [
            "",
            "text/",
            "text plain",
            "text/plain;",
            "text/plain; name",
            "text/plain; =x",
            "text/plain; name=",
            "text/plain; name=a b",
            "text/plain; name=x;",
            "text/plain; name=x; name=y",
            "text/plain; name=\"x",
            "text/plain; name=\"x\\",
            "text/plain; name=\"x\"junk",
            "text/plain; name=\"x\";",
            "text/plain; name=\"x\"; name=y",
            "text/plain\0",
            "text/pláin",
        ] {
            assert!(mime_value(v.as_bytes()).is_err(), "{v}");
        }
        assert!(mime_value(b"text/\xff").is_err());
        for raw in [
            b"Content-Type : text/html\n\n".as_slice(),
            b"Content-Type broken\n\n",
        ] {
            assert!(headers(raw).is_err());
        }
        assert!(!unsubscribe(b"<https://x>, <mailto:bad[>"));
        assert!(!unsubscribe(b"<https://x>, <x:bad#a#b>"));
        assert!(!unsubscribe(b"<https://x>, <http://[bad]>"));
        assert!(number("99999999999", 1, 11).is_none());
        let (h, n) = headers(b"Content-Type: text/plain\n\n").unwrap();
        assert!(plain_body(b"Content-Type: text/plain\n\n", h, n).unwrap());
        for raw in [
            "Content-Type: text\n\n",
            "Content-Disposition: inline/bad\n\n",
            "Content-Type: text/plain\nContent-Type: text/html\n\n",
            "Content-Disposition: inline\nContent-Disposition: attachment\n\n",
            "Content-Type: multipart/mixed\n\n",
            "Content-Type: multipart/mixed; boundary=x\nContent-Transfer-Encoding: base64\n\n--x\n\nbody\n--x--\n",
            "Content-Type: multipart/mixed; boundary=x\nContent-Transfer-Encoding: 7bit\nContent-Transfer-Encoding: 8bit\n\n--x\n\nbody\n--x--\n",
        ] {
            let (h, n) = headers(raw.as_bytes()).unwrap();
            assert_eq!(
                plain_body(raw.as_bytes(), h, n),
                Err(AnalysisError::Failed),
                "{raw}"
            );
        }
        for b in ["", "a ", "a@", &"x".repeat(71)] {
            assert!(multipart_children(b"", b.as_bytes()).is_err());
        }
        for body in [
            "--x--\n",
            "preamble\n",
            "--x\n\nbody\n",
            "--xjunk\n\nbody\n--x--\n",
        ] {
            assert!(multipart_children(body.as_bytes(), b"x").is_err());
        }
        assert_eq!(
            multipart_children(b"preamble\n--x \t\n\nbody\n--x--\t", b"x")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            multipart_children(b"--x\n\nbody\n--xjunk\n--x--", b"x")
                .unwrap()
                .len(),
            1
        );
        let many = format!("{}--x--\n", "--x\n\nb\n".repeat(MAX_PARTS + 1));
        assert!(multipart_children(many.as_bytes(), b"x").is_err());
    }

    #[tokio::test]
    async fn bulk_auth_cache_identity() {
        let service = Service::default();
        let e = mail("same");
        let original = Inputs {
            auth: super::concurrency_tests::passing(),
            ..Inputs::default()
        };
        let first = service
            .with_inputs(e.clone(), original.clone(), false)
            .await
            .unwrap();
        assert!(first.findings.is_empty());
        let mut variants = Vec::new();
        for field in 0..8 {
            let mut input = original.clone();
            match field {
                0 => input.auth.policy_present = Fact::False,
                1 => input.auth.valid_from = Fact::False,
                2 => input.auth.dkim_pass = Fact::Unknown,
                3 => input.auth.dkim_aligned = Fact::False,
                4 => input.auth.spf_pass = Fact::Unknown,
                5 => input.auth.spf_aligned = Fact::Unknown,
                6 => input.auth.policy_id.push('x'),
                _ => input.auth.observation_id.push('x'),
            }
            variants.push(input);
        }
        for input in variants {
            let next = service
                .with_inputs(e.clone(), input.clone(), false)
                .await
                .unwrap();
            assert!(!Arc::ptr_eq(&first, &next));
            assert!(Arc::ptr_eq(
                &next,
                &service.with_inputs(e.clone(), input, true).await.unwrap()
            ));
        }
        let duplicate_public_id = Arc::new((*e).clone());
        assert!(!Arc::ptr_eq(
            &first,
            &service
                .with_inputs(duplicate_public_id, original, false)
                .await
                .unwrap()
        ));
        assert_eq!(service.slots.lock().unwrap().len(), 10);
    }
    #[tokio::test]
    async fn bulk_auth_wire_pairs_and_positive_predicates() {
        use Fact::{False as F, True as T, Unknown as U};
        // Each tuple is independently supplied trusted evidence, not report rows.
        for (facts, expected) in [
            ([T, T, T, T, F, F], [T, T, T]), // valid p=none policy, aligned DKIM wins
            ([F, T, F, F, F, F], [F, F, F]),
            ([U, U, U, U, U, U], [U, U, U]),
            ([T, F, T, T, T, T], [T, F, F]), // invalid From forbids success
            ([F, T, T, T, U, U], [F, T, T]), // policy presence is independent
            ([T, T, F, T, F, F], [T, T, F]), // alignment alone is not authentication
            ([T, T, T, F, U, U], [T, U, U]), // unaligned pass cannot beat unknown SPF
            ([T, T, U, U, T, T], [T, T, T]), // aligned SPF wins over unknown DKIM
        ] {
            let srv = crate::run_on(&config(None)).await.unwrap();
            let [
                policy_present,
                valid_from,
                dkim_pass,
                dkim_aligned,
                spf_pass,
                spf_aligned,
            ] = facts;
            srv.store
                .reports
                .auth
                .set(AuthEvidence {
                    policy_id: "p=none".into(),
                    observation_id: "fixture-verifier".into(),
                    policy_present,
                    valid_from,
                    dkim_pass,
                    dkim_aligned,
                    spf_pass,
                    spf_aligned,
                })
                .unwrap();
            let id = smtp_bytes(&srv, GOOD).await;
            let path = format!("/api/v1/messages/{id}/deliverability");
            let (status, body) = get(&srv, &path).await;
            assert_eq!(status, 200);
            let value: serde_json::Value = serde_json::from_str(&body).unwrap();
            for (i, rule) in ["dmarc_presence", "from_alignment", "authenticated_sender"]
                .iter()
                .enumerate()
            {
                let found = value["findings"].as_array().unwrap().iter().find(|v| {
                    v["rule_id"]
                        .as_str()
                        .unwrap()
                        .starts_with(&format!("bulk.{rule}."))
                });
                if expected[i] == T {
                    assert!(found.is_none(), "{value}");
                    continue;
                }
                let found = found.unwrap();
                let (hint, rfc) = match (i, expected[i]) {
                    (0, F) => (
                        "Publish a valid applicable DMARC policy; p=none satisfies this presence check.",
                        "https://www.rfc-editor.org/rfc/rfc7489#section-6.1",
                    ),
                    (0, _) => (
                        "Not evaluated: trusted applicable DMARC policy evidence is unavailable.",
                        "https://www.rfc-editor.org/rfc/rfc7489#section-6.1",
                    ),
                    (1, F) => (
                        "Align a DKIM d= or SPF MAIL FROM identity with an unambiguous RFC5322.From domain under the applicable DMARC policy.",
                        "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
                    ),
                    (1, _) => (
                        "Not evaluated: trusted From identity or DKIM/SPF alignment evidence is incomplete.",
                        "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
                    ),
                    (_, F) => (
                        "Supply a passing DKIM or SPF result aligned with RFC5322.From. This scoped check does not certify every provider authentication requirement.",
                        "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
                    ),
                    _ => (
                        "Not evaluated: trusted From identity or passing-and-aligned DKIM/SPF evidence is incomplete.",
                        "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
                    ),
                };
                assert_eq!(
                    *found,
                    serde_json::json!({"rule_id":format!("bulk.{rule}.{}v1",if expected[i] == U { "not_evaluated." } else { "" }),"severity":"error","location":{"kind":"header","name":"from","occurrence":0},"fix_hint":format!("{hint} Sources: {rfc} https://support.google.com/mail/answer/81126 https://senders.yahooinc.com/best-practices/")})
                );
            }
            assert_eq!(get(&srv, &path).await, (status, body));
            let clean = expected == [T, T, T];
            let (status, page) = get(&srv, &format!("/ui/message/{id}")).await;
            assert_eq!(status, 200);
            assert_eq!(page.contains("No findings from this rule set."), clean);
            assert_eq!(page.contains("Sources:"), !clean);

            for threshold in ["info", "warning", "error"] {
                let (status, body) = get(
                    &srv,
                    &format!(
                        "/api/v1/inboxes/default/assert?count={}&no_findings_at_or_above={threshold}",
                        usize::from(clean)
                    ),
                )
                .await;
                assert_eq!(status, 200, "{body}");
                assert_eq!(get(&srv,&format!("/api/v1/inboxes/default/await?timeout_ms=0&no_findings_at_or_above={threshold}")).await.0,if clean {200} else {408});
            }
            if clean {
                let newest = smtp_bytes(&srv, GOOD).await;
                let (status, body) = get(
                    &srv,
                    "/api/v1/inboxes/default/await?count=2&limit=1&no_findings_at_or_above=warning",
                )
                .await;
                assert_eq!(status, 200);
                let value: serde_json::Value = serde_json::from_str(&body).unwrap();
                assert_eq!(value["matched"], 2);
                assert_eq!(value["emails"].as_array().unwrap().len(), 1);
                assert_eq!(value["emails"][0]["id"], newest);
            }
            srv.stop().await;
        }
    }
}
