//! The oracle's mail access, per backend (proposed decision 0129 §5, spec
//! R7). The oracle is a control: it solves a task through the target's own
//! tools, as an agent would, and the scorer is the only judge. Each
//! implementation calls only that backend's adapter tools — list and read —
//! and maps what it gets to the one shape the solutions read:
//! `{to: [{address}], subject, text, html}`.
//!
//! The solutions never see a difference between targets except the mapping
//! itself. MIME decoding of MailHog's raw bodies (7bit, 8bit,
//! quoted-printable, base64, multipart) lives here, in the control, and not
//! in the adapter: an agent is handed the raw bytes. A shape this decoder
//! does not know is an oracle error, never a pass.
//!
//! The listing is filtered by subject here, from the complete listing the
//! adapter pages to completion: no backend's search syntax is relied on for
//! a control (spec R7: where upstream has no verified filter the oracle
//! filters the whole listing itself).

use crate::mcp::McpClient;
use base64::Engine;
use serde_json::{Value, json};
use std::time::Duration;

/// How long a control waits for the expected mail to be listed. A backend
/// has no accept-implies-queryable promise to lean on (decision 0129 §3).
const LIST_DEADLINE: Duration = Duration::from_secs(30);
/// The fixed poll interval between complete listings.
const LIST_POLL: Duration = Duration::from_millis(100);
/// The adapters' page ceiling.
const PAGE: u64 = 250;
/// MailSlurper's fixed page size.
const SLURPER_PAGE: u64 = 50;
/// Messages one control will read: a bound, not a claim.
const MAX_MESSAGES: u64 = 5_000;

/// Mail access for the oracle: the mails whose subject contains `subject`
/// (all when `None`), once at least `count` exist.
#[allow(async_fn_in_trait)]
pub trait MailSource {
    async fn wait_emails(
        &mut self,
        subject: Option<&str>,
        count: usize,
    ) -> Result<Vec<Value>, String>;

    /// The MCP `tools/call` requests this source has made.
    fn tool_calls(&self) -> u32;
}

/// Swarmail: today's calls, unchanged — the long-poll, in the trial's inbox.
pub struct SwarmailSource<'a> {
    pub mcp: &'a mut McpClient,
    pub inbox: &'a str,
}

impl MailSource for SwarmailSource<'_> {
    async fn wait_emails(
        &mut self,
        subject: Option<&str>,
        count: usize,
    ) -> Result<Vec<Value>, String> {
        self.mcp.wait_emails(self.inbox, subject, count).await
    }

    fn tool_calls(&self) -> u32 {
        self.mcp.tool_calls()
    }
}

/// Which competitor an adapter fronts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Competitor {
    Mailpit,
    Mailhog,
    Mailslurper,
}

impl Competitor {
    /// The competitor a `--target` name selects.
    pub fn from_target(name: &str) -> Option<Competitor> {
        match name {
            "mailpit" => Some(Competitor::Mailpit),
            "mailhog" => Some(Competitor::Mailhog),
            "mailslurper" => Some(Competitor::Mailslurper),
            _ => None,
        }
    }
}

/// A competitor reached through its adapter's tools over the bridge.
pub struct CompetitorSource<'a> {
    mcp: &'a mut McpClient,
    kind: Competitor,
    deadline: Duration,
}

impl<'a> CompetitorSource<'a> {
    pub fn new(mcp: &'a mut McpClient, kind: Competitor) -> Self {
        CompetitorSource {
            mcp,
            kind,
            deadline: LIST_DEADLINE,
        }
    }

    /// A shorter wait for the listing, for tests of the refusal path.
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }
}

impl CompetitorSource<'_> {
    fn list_tool(&self) -> &'static str {
        match self.kind {
            Competitor::Mailpit => "mailpit_list",
            Competitor::Mailhog => "mailhog_list",
            Competitor::Mailslurper => "mailslurper_list",
        }
    }

    /// Every stored message's summary (Mailpit) or full item (MailHog),
    /// paged to completion with the adapter's own `total`.
    async fn listing(&mut self) -> Result<Vec<Value>, String> {
        let (key, total_key) = match self.kind {
            Competitor::Mailpit => ("messages", "total"),
            Competitor::Mailhog => ("items", "total"),
            Competitor::Mailslurper => ("mailItems", "totalRecords"),
        };
        let tool = self.list_tool();
        let mut out: Vec<Value> = Vec::new();
        loop {
            // MailSlurper pages a fixed 50 by page number; the others take an
            // offset and a size.
            let args = match self.kind {
                Competitor::Mailslurper => {
                    json!({"page": out.len() as u64 / SLURPER_PAGE + 1})
                }
                _ => json!({"start": out.len(), "limit": PAGE}),
            };
            let page = self.mcp.call(tool, args).await?;
            let total = page[total_key]
                .as_u64()
                .ok_or_else(|| format!("{tool}: no {total_key}"))?;
            // MailSlurper may leave `mailItems` out of an empty page (decision
            // 0118 §4); every other absence is an error.
            let none = Vec::new();
            let items = match (page[key].as_array(), self.kind) {
                (Some(items), _) => items,
                (None, Competitor::Mailslurper) if page.get(key).is_none() => &none,
                (None, _) => return Err(format!("{tool}: no {key}")),
            };
            // Mailpit's `total` is the whole mailbox; MailHog's is the same
            // for a plain listing.
            out.extend(items.iter().cloned());
            if out.len() as u64 >= total {
                return Ok(out);
            }
            if items.is_empty() || out.len() as u64 > MAX_MESSAGES {
                return Err(format!("{tool}: the listing did not reach its total"));
            }
        }
    }
}

impl MailSource for CompetitorSource<'_> {
    async fn wait_emails(
        &mut self,
        subject: Option<&str>,
        count: usize,
    ) -> Result<Vec<Value>, String> {
        let until = tokio::time::Instant::now() + self.deadline;
        let matched = loop {
            let all = self.listing().await?;
            let hits: Vec<Value> = all
                .into_iter()
                .filter(|m| match (subject, subject_of(self.kind, m)) {
                    (None, _) => true,
                    (Some(want), Some(have)) => have.contains(want),
                    (Some(_), None) => false,
                })
                .collect();
            if hits.len() >= count {
                break hits;
            }
            if tokio::time::Instant::now() >= until {
                return Err(format!(
                    "waited for {count} mail(s) matching {subject:?}; listed {}",
                    hits.len()
                ));
            }
            tokio::time::sleep(LIST_POLL).await;
        };
        let mut out = Vec::with_capacity(matched.len());
        for m in &matched {
            out.push(match self.kind {
                Competitor::Mailpit => {
                    let id = m["ID"].as_str().ok_or("a Mailpit summary has no ID")?;
                    let full = self.mcp.call("mailpit_read", json!({"id": id})).await?;
                    normalize_mailpit(&full)?
                }
                // The item already carries the raw message.
                Competitor::Mailhog => normalize_mailhog(m)?,
                Competitor::Mailslurper => {
                    let id = m["id"].as_str().ok_or("a MailSlurper item has no id")?;
                    let full = self.mcp.call("mailslurper_read", json!({"id": id})).await?;
                    normalize_mailslurper(&full)?
                }
            });
        }
        Ok(out)
    }

    fn tool_calls(&self) -> u32 {
        self.mcp.tool_calls()
    }
}

fn subject_of(kind: Competitor, m: &Value) -> Option<&str> {
    match kind {
        Competitor::Mailpit => m.get("Subject")?.as_str(),
        Competitor::Mailslurper => m.get("subject")?.as_str(),
        Competitor::Mailhog => m
            .get("Content")?
            .get("Headers")?
            .get("Subject")?
            .get(0)?
            .as_str(),
    }
}

/// A Mailpit message (`GET /api/v1/message/{id}`) in the solutions' shape.
pub fn normalize_mailpit(m: &Value) -> Result<Value, String> {
    let subject = m["Subject"]
        .as_str()
        .ok_or("a Mailpit message has no Subject")?;
    let to: Vec<Value> = m["To"]
        .as_array()
        .ok_or("a Mailpit message has no To")?
        .iter()
        .map(|a| {
            a["Address"]
                .as_str()
                .map(|addr| json!({"address": addr}))
                .ok_or("a Mailpit recipient has no Address")
        })
        .collect::<Result<_, _>>()?;
    Ok(json!({
        "subject": subject,
        "text": m["Text"].as_str().unwrap_or_default(),
        "html": m["HTML"].as_str().filter(|h| !h.is_empty()),
        "to": to,
    }))
}

/// A MailHog item (`GET /api/v2/messages` `items[]`) in the solutions'
/// shape, its body decoded from the raw message it stores.
pub fn normalize_mailhog(m: &Value) -> Result<Value, String> {
    let subject = subject_of(Competitor::Mailhog, m).ok_or("a MailHog item has no Subject")?;
    // The envelope recipients, as MailHog's `Path` splits them.
    let to: Vec<Value> = m["To"]
        .as_array()
        .ok_or("a MailHog item has no To")?
        .iter()
        .map(|p| match (p["Mailbox"].as_str(), p["Domain"].as_str()) {
            (Some(mailbox), Some(domain)) => Ok(json!({"address": format!("{mailbox}@{domain}")})),
            _ => Err("a MailHog recipient has no Mailbox and Domain"),
        })
        .collect::<Result<_, _>>()?;
    let raw = m["Raw"]["Data"]
        .as_str()
        .ok_or("a MailHog item has no Raw.Data")?;
    let body = decode_mime(raw)?;
    Ok(json!({
        "subject": subject,
        "text": body.text.unwrap_or_default(),
        "html": body.html,
        "to": to,
    }))
}

/// A MailSlurper mail (`GET /mail/{id}`) in the solutions' shape, read from
/// the shapes the pinned executable was observed to serve
/// (`docs/evals/wire-corpus/mailslurper/`), not from what its source suggests.
/// MailSlurper stores one `body` string per mail, and `boundary` and
/// `transferEncoding` come back empty whatever the mail said, so the mail's
/// MIME structure is not recoverable and none is read:
///
/// - `text/plain`: MailSlurper turned every line break into `\n<br/>` and its
///   sanitiser HTML-escaped the text (`&` is `&amp;`). The mapping undoes
///   exactly those two things, and a `<br` left over is an error, because the
///   sanitiser would have escaped a literal one.
/// - `text/html`: the body, as stored.
/// - `multipart/*`: the body is the one part MailSlurper kept, the HTML part
///   (its `GetMail` base64-decodes it when it can); a multipart's plain-text
///   alternative is not stored. A body that is not markup is an error, never
///   a guess at the part's transfer encoding.
///
/// `toAddresses` must be a list of address strings; any other element shape
/// is an error until a real response shows it, and the addresses are kept
/// exactly, because MailSlurper's own `to` filter is a substring match and the
/// solutions match the recipient exactly.
pub fn normalize_mailslurper(m: &Value) -> Result<Value, String> {
    let subject = m["subject"]
        .as_str()
        .ok_or("a MailSlurper mail has no subject")?;
    let to: Vec<Value> = m["toAddresses"]
        .as_array()
        .ok_or("a MailSlurper mail has no toAddresses")?
        .iter()
        .map(|a| {
            a.as_str()
                .map(|addr| json!({"address": addr}))
                .ok_or("a MailSlurper recipient is not an address string")
        })
        .collect::<Result<_, _>>()?;
    let ctype = m["contentType"]
        .as_str()
        .filter(|c| !c.trim().is_empty())
        .ok_or("a MailSlurper mail has no contentType")?;
    let body = m["body"].as_str().ok_or("a MailSlurper mail has no body")?;
    let (ty, _) = media_type(ctype);
    let (text, html) = match ty.as_str() {
        "text/plain" => {
            let lines = body.replace("\n<br/>", "\n");
            if lines.contains("<br") {
                return Err(
                    "a MailSlurper text body holds markup its sanitiser would have escaped".into(),
                );
            }
            (crate::oracle::decode_entities(&lines), Value::Null)
        }
        "text/html" => (String::new(), json!(body)),
        t if t.starts_with("multipart/") && body.trim_start().starts_with('<') => {
            (String::new(), json!(body))
        }
        t if t.starts_with("multipart/") => {
            return Err("a MailSlurper multipart body is not its HTML part".into());
        }
        other => return Err(format!("unknown MailSlurper content type {other:?}")),
    };
    Ok(json!({
        "subject": subject,
        "text": text,
        "html": html,
        "to": to,
    }))
}

/// The readable parts of a message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Body {
    pub text: Option<String>,
    pub html: Option<String>,
}

const MAX_DEPTH: usize = 4;

/// Decode a whole RFC 5322 message into its first inline `text/plain` and
/// `text/html` parts. Known shapes only: 7bit, 8bit, quoted-printable and
/// base64 transfer encodings; UTF-8 or US-ASCII text; `multipart/*` of
/// those. Anything else is an error.
pub fn decode_mime(raw: &str) -> Result<Body, String> {
    let mut body = Body::default();
    walk(raw, 0, &mut body)?;
    Ok(body)
}

struct Headers(Vec<(String, String)>);

impl Headers {
    fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn split_head(part: &str) -> Result<(Headers, &str), String> {
    let (head, body) = ["\r\n\r\n", "\n\n"]
        .iter()
        .filter_map(|sep| part.find(sep).map(|at| (at, sep.len())))
        .min()
        .map(|(at, len)| (&part[..at], &part[at + len..]))
        .ok_or("a MIME part has no header block")?;
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in head.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            let last = headers.last_mut().ok_or("a header continues nothing")?;
            last.1.push(' ');
            last.1.push_str(line.trim());
        } else if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        } else if !line.is_empty() {
            return Err("a header line has no colon".into());
        }
    }
    Ok((Headers(headers), body))
}

/// `type/subtype` lowercase, and the value of a named parameter.
fn media_type(value: &str) -> (String, impl Fn(&str) -> Option<String> + '_) {
    let mut parts = value.split(';');
    let ty = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let params: Vec<(String, String)> = parts
        .filter_map(|p| {
            let (k, v) = p.split_once('=')?;
            Some((
                k.trim().to_ascii_lowercase(),
                v.trim().trim_matches('"').to_string(),
            ))
        })
        .collect();
    (ty, move |name: &str| {
        params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    })
}

fn walk(part: &str, depth: usize, out: &mut Body) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("MIME parts nest too deeply".into());
    }
    let (headers, body) = split_head(part)?;
    let ctype = headers
        .get("content-type")
        .unwrap_or("text/plain; charset=us-ascii");
    let (ty, param) = media_type(ctype);
    if headers.get("content-disposition").is_some_and(|d| {
        d.trim_start()
            .to_ascii_lowercase()
            .starts_with("attachment")
    }) {
        return Ok(());
    }
    if let Some(sub) = ty.strip_prefix("multipart/") {
        if !matches!(sub, "alternative" | "mixed" | "related") {
            return Err(format!("unknown multipart subtype {sub:?}"));
        }
        let boundary = param("boundary").ok_or("a multipart has no boundary")?;
        return multipart(body, &boundary, depth, out);
    }
    if ty != "text/plain" && ty != "text/html" {
        return Err(format!("unknown content type {ty:?}"));
    }
    let charset = param("charset").unwrap_or_else(|| "us-ascii".into());
    if !matches!(charset.to_ascii_lowercase().as_str(), "utf-8" | "us-ascii") {
        return Err(format!("unknown charset {charset:?}"));
    }
    let encoding = headers
        .get("content-transfer-encoding")
        .unwrap_or("7bit")
        .to_ascii_lowercase();
    let bytes = match encoding.as_str() {
        "7bit" | "8bit" => body.as_bytes().to_vec(),
        "quoted-printable" => quoted_printable(body)?,
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(body.split_whitespace().collect::<String>())
            .map_err(|_| "invalid base64 body")?,
        other => return Err(format!("unknown transfer encoding {other:?}")),
    };
    let text = String::from_utf8(bytes).map_err(|_| "a body is not UTF-8")?;
    let slot = if ty == "text/html" {
        &mut out.html
    } else {
        &mut out.text
    };
    if slot.is_none() {
        *slot = Some(text);
    }
    Ok(())
}

fn multipart(body: &str, boundary: &str, depth: usize, out: &mut Body) -> Result<(), String> {
    let open = format!("--{boundary}");
    let mut parts = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    let mut closed = false;
    for line in body.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        if bare == format!("{open}--") {
            if let Some(p) = current.take() {
                parts.push(p.concat());
            }
            closed = true;
            break;
        } else if bare == open {
            if let Some(p) = current.take() {
                parts.push(p.concat());
            }
            current = Some(Vec::new());
        } else if let Some(p) = current.as_mut() {
            p.push(line);
        }
    }
    if !closed {
        return Err("a multipart is not closed".into());
    }
    for part in &parts {
        // The line break before a boundary belongs to the boundary.
        let part = part
            .strip_suffix("\r\n")
            .or_else(|| part.strip_suffix('\n'))
            .unwrap_or(part);
        walk(part, depth + 1, out)?;
    }
    Ok(())
}

fn quoted_printable(body: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'=' => {
                if bytes[i + 1..].starts_with(b"\r\n") {
                    i += 3;
                } else if bytes[i + 1..].starts_with(b"\n") {
                    i += 2;
                } else {
                    let hex = body.get(i + 1..i + 3).ok_or("a truncated =XX escape")?;
                    out.push(u8::from_str_radix(hex, 16).map_err(|_| "a bad =XX escape")?);
                    i += 3;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    Ok(out)
}
