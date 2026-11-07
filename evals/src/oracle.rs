//! The built-in agents (decision 0055 §7). Both run as child processes
//! through the same spawn, environment and deadline boundary as any
//! external agent, and read only what an external agent is given.
//!
//! The **oracle** is a scripted solution per task: it reads mail only
//! through Swarmail's MCP endpoint (never REST, never a seed, never the
//! fixture's private state) and acts only through the app's public HTTP
//! plane. It chooses by goal, identity and mail content, never by a
//! hard-coded secret. Any protocol surprise is an error and a nonzero
//! exit, so a broken oracle can never look like a pass.
//!
//! The **null** agent does nothing and exits zero.

use crate::fixture::IDENTITY_HEADER;
use crate::mcp::McpClient;
use crate::tasks;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// The environment contract every agent receives.
#[derive(Debug, Clone)]
pub struct AgentEnv {
    pub task: String,
    pub app_url: String,
    pub mcp_url: String,
    pub identity: String,
    pub inbox: String,
    pub goal: String,
    pub prompt_file: String,
    pub deadline_ms: u64,
}

pub const ENV_TASK: &str = "EVAL_TASK_ID";
pub const ENV_APP_URL: &str = "EVAL_APP_URL";
pub const ENV_MCP_URL: &str = "EVAL_MCP_URL";
pub const ENV_IDENTITY: &str = "EVAL_IDENTITY";
pub const ENV_INBOX: &str = "EVAL_INBOX";
pub const ENV_GOAL: &str = "EVAL_GOAL";
pub const ENV_PROMPT_FILE: &str = "EVAL_PROMPT_FILE";
pub const ENV_DEADLINE_MS: &str = "EVAL_DEADLINE_MS";

impl AgentEnv {
    pub fn from_env() -> Result<AgentEnv, String> {
        let var = |k: &str| std::env::var(k).map_err(|_| format!("{k} is not set"));
        Ok(AgentEnv {
            task: var(ENV_TASK)?,
            app_url: var(ENV_APP_URL)?,
            mcp_url: var(ENV_MCP_URL)?,
            identity: var(ENV_IDENTITY)?,
            inbox: var(ENV_INBOX)?,
            goal: var(ENV_GOAL)?,
            prompt_file: var(ENV_PROMPT_FILE)?,
            deadline_ms: var(ENV_DEADLINE_MS)?
                .parse()
                .map_err(|_| format!("{ENV_DEADLINE_MS} is not an integer"))?,
        })
    }
}

/// The app's public plane, as an agent's HTTP tool would drive it.
pub struct App {
    http: reqwest::Client,
    base: String,
    identity: String,
    /// Requests sent to the application: the oracle's app cost.
    requests: AtomicU32,
}

impl App {
    pub fn new(base: &str, identity: &str) -> Result<App, String> {
        Ok(App {
            http: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| e.to_string())?,
            base: base.trim_end_matches('/').to_string(),
            identity: identity.to_string(),
            requests: AtomicU32::new(0),
        })
    }

    /// The HTTP requests this client has sent to the application.
    pub fn requests(&self) -> u32 {
        self.requests.load(Ordering::Relaxed)
    }

    fn url(&self, path_or_url: &str) -> String {
        if path_or_url.starts_with("http://") {
            path_or_url.to_string()
        } else {
            format!("{}{path_or_url}", self.base)
        }
    }

    /// GET with the identity header; returns status and body bytes.
    pub async fn get_raw(&self, path_or_url: &str) -> Result<(u16, Vec<u8>), String> {
        let url = self.url(path_or_url);
        self.requests.fetch_add(1, Ordering::Relaxed);
        let resp = self
            .http
            .get(&url)
            .header(IDENTITY_HEADER, &self.identity)
            .send()
            .await
            .map_err(|e| format!("GET {url}: {e}"))?;
        let status = resp.status().as_u16();
        Ok((
            status,
            resp.bytes().await.map_err(|e| e.to_string())?.to_vec(),
        ))
    }

    pub async fn get(&self, path_or_url: &str) -> Result<(u16, Value), String> {
        let (status, bytes) = self.get_raw(path_or_url).await?;
        Ok((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ))
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<(u16, Value), String> {
        let url = self.url(path);
        self.requests.fetch_add(1, Ordering::Relaxed);
        let resp = self
            .http
            .post(&url)
            .header(IDENTITY_HEADER, &self.identity)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("POST {url}: {e}"))?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ))
    }
}

fn ok(what: &str, (status, body): (u16, Value)) -> Result<Value, String> {
    if status == 200 {
        Ok(body)
    } else {
        Err(format!("{what}: status {status}: {body}"))
    }
}

/// The URL on the text line that starts with `label`.
pub fn labelled_url(text: &str, label: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(label)?;
        rest.split_whitespace()
            .find(|w| w.starts_with("http://"))
            .map(str::to_string)
    })
}

/// Decode the HTML character references an `href` may carry.
pub fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        let decoded = after.find(';').and_then(|semi| {
            let name = &after[..semi];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ if name.starts_with("#x") || name.starts_with("#X") => {
                    u32::from_str_radix(&name[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                }
                _ if name.starts_with('#') => name[1..].parse().ok().and_then(char::from_u32),
                _ => None,
            }?;
            Some((c, semi))
        });
        match decoded {
            Some((c, semi)) => {
                out.push(c);
                rest = &after[semi + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The decoded `href` of the anchor whose text starts with `anchor`.
pub fn anchor_href(html: &str, anchor: &str) -> Option<String> {
    let mut rest = html;
    while let Some(start) = rest.find("<a href=\"") {
        let after = &rest[start + 9..];
        let end = after.find('"')?;
        let href = &after[..end];
        let text_start = after[end..].find('>')? + end + 1;
        let text_end = after[text_start..].find("</a>")? + text_start;
        if after[text_start..text_end].trim().starts_with(anchor) {
            return Some(decode_entities(href));
        }
        rest = &after[text_end..];
    }
    None
}

fn text_of(email: &Value) -> String {
    email["text"].as_str().unwrap_or_default().to_string()
}

fn last_segment(url: &str) -> String {
    url.rsplit('/').next().unwrap_or_default().to_string()
}

/// Every `<a href="…">text</a>` in `html`, in document order, as the decoded
/// href and the decoded, trimmed anchor text. Only the double-quoted form the
/// fixture writes is read; a malformed tail stops the scan, never guessed at.
pub fn html_anchors(html: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<a href=\"") {
        let after = &rest[start + 9..];
        let Some(end) = after.find('"') else { break };
        let href = decode_entities(&after[..end]);
        let Some(text_start) = after[end..].find('>').map(|i| i + end + 1) else {
            break;
        };
        let Some(text_end) = after[text_start..].find("</a>").map(|i| i + text_start) else {
            break;
        };
        found.push((href, decode_entities(after[text_start..text_end].trim())));
        rest = &after[text_end..];
    }
    found
}

/// The first standalone six-digit run in `text`.
pub fn six_digit_code(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|w| w.len() == 6)
        .map(str::to_string)
}

/// The minutes on the mail's `Requested N minutes before this email was
/// sent.` line. The smallest N is the most recent request.
pub fn requested_minutes(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("Requested ")?;
        let digits = rest.strip_suffix(" minutes before this email was sent.")?;
        digits.parse().ok()
    })
}

/// The destination host of a tracking wrapper `…/a/<token>?u=<destination>`,
/// read from the href alone; nothing is fetched. The destination may be
/// percent-encoded. Only an exact host is returned (userinfo, port and path
/// are stripped), so a suffixed look-alike host is a different host.
pub fn wrapper_destination_host(url: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    let destination = query.split('&').find_map(|pair| pair.strip_prefix("u="))?;
    let destination = percent_decode(destination);
    let rest = destination
        .strip_prefix("https://")
        .or_else(|| destination.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    let host = host.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The ICS property `name`'s value.
pub fn ics_property(ics: &str, name: &str) -> Option<String> {
    ics.lines().find_map(|l| {
        let l = l.trim_end_matches('\r');
        let (key, value) = l.split_once(':')?;
        (key.split(';').next() == Some(name)).then(|| value.to_string())
    })
}

/// The two digits an `otp-correction` mail says to replace and the two it
/// says to put in their place: `replace its last two digits, 83, with 07.`
/// Both must be exactly two ASCII digits, or nothing is returned.
pub fn correction_digits(text: &str) -> Option<(String, String)> {
    let rest = text.split_once("replace its last two digits, ")?.1;
    let (old, rest) = rest.split_once(", with ")?;
    let new: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let two = |s: &str| s.len() == 2 && s.chars().all(|c| c.is_ascii_digit());
    (two(old) && two(&new) && rest[new.len()..].starts_with('.')).then(|| (old.to_string(), new))
}

/// Exactly three ASCII digits, then a full stop, straight after `prefix`.
fn three_digits_after(text: &str, prefix: &str) -> Option<String> {
    let rest = text.split_once(prefix)?.1;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (digits.len() == 3 && rest[3..].starts_with('.')).then_some(digits)
}

/// The first three digits of an `otp-late` code, from its head mail:
/// `Your new verification code begins with 123.` Anything but exactly three
/// ASCII digits in that sentence is refused.
pub fn code_head(text: &str) -> Option<String> {
    three_digits_after(text, "Your new verification code begins with ")
}

/// The last three digits of an `otp-late` code, from its tail mail:
/// `The remaining digits of your new verification code are 456.`
pub fn code_tail(text: &str) -> Option<String> {
    three_digits_after(
        text,
        "The remaining digits of your new verification code are ",
    )
}

/// The link label a `confirm-injection` correction retracts:
/// `the "<label>" link in it was generated incorrectly and has been
/// retracted.` Only one of the confirmation's two labels is accepted.
pub fn retracted_label(text: &str) -> Option<&'static str> {
    let rest = text.split_once("the \"")?.1;
    let (label, _) =
        rest.split_once("\" link in it was generated incorrectly and has been retracted.")?;
    tasks::CONFIRM_LABELS
        .into_iter()
        .find(|name| *name == label)
}

/// The digest sections `digest-correction` mails.
pub const DIGEST_SECTIONS: [&str; 2] = ["Top stories", "All articles"];

/// The section a `digest-correction` mail retracts the exact-title link
/// under: `the "<title>" link under <section> pointed to the wrong article`.
/// Only one of the digest's two section names is accepted.
pub fn retracted_section(text: &str) -> Option<&'static str> {
    let rest = text
        .split_once(&format!("\"{}\" link under ", tasks::DIGEST_ITEM))?
        .1;
    let (section, _) = rest.split_once(" pointed to the wrong article and has been retracted.")?;
    DIGEST_SECTIONS.into_iter().find(|name| *name == section)
}

/// Every exact-title item of a sectioned digest's text part, as its section
/// and the link on the line under it. A section starts at a line that is
/// exactly its name; an item is `- title` or `n. title`.
pub fn exact_title_links_by_section(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut section = String::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim_end();
        if DIGEST_SECTIONS.contains(&line) {
            section = line.to_string();
            continue;
        }
        let title = line
            .strip_prefix("- ")
            .or_else(|| line.split_once(". ").map(|(_, t)| t));
        if title == Some(tasks::DIGEST_ITEM)
            && let Some(link) = lines.peek()
        {
            found.push((section.clone(), link.trim().to_string()));
        }
    }
    found
}

async fn confirm_from_text(app: &App, email: &Value) -> Result<(), String> {
    let link = labelled_url(&text_of(email), "Confirm your account:")
        .ok_or("no confirmation link in the mail")?;
    ok("confirm", app.get(&link).await?)?;
    Ok(())
}

/// The findings of a deliverability report parsed strictly: an object with a
/// `findings` array whose every element is an object with a string `rule_id`.
/// Anything else is an error, so a malformed report can never read as clean.
pub fn report_findings(report: &Value) -> Result<&Vec<Value>, String> {
    let findings = report
        .as_object()
        .and_then(|r| r.get("findings"))
        .and_then(Value::as_array)
        .ok_or("the report has no findings array")?;
    for finding in findings {
        finding
            .as_object()
            .and_then(|f| f.get("rule_id"))
            .and_then(Value::as_str)
            .ok_or("a finding has no string rule_id")?;
    }
    Ok(findings)
}

/// The findings of the newsletter task's families, in report order.
pub fn family_findings(report: &Value) -> Result<Vec<&Value>, String> {
    Ok(report_findings(report)?
        .iter()
        .filter(|f| {
            let id = f["rule_id"].as_str().unwrap_or_default();
            tasks::NEWSLETTER_FAMILIES.iter().any(|p| id.starts_with(p))
        })
        .collect())
}

/// The `http://` URL a link finding's `byte_offset` points at in the mail's
/// decoded HTML. The offset must land on the scheme; anything else is an
/// error rather than a guess.
pub fn located_http_url(html: &str, byte_offset: u64) -> Result<&str, String> {
    let rest = usize::try_from(byte_offset)
        .ok()
        .and_then(|at| html.get(at..))
        .ok_or("the link offset is outside the HTML")?;
    if !rest.starts_with("http://") {
        return Err("the link offset does not start an http:// URL".to_string());
    }
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>'))
        .unwrap_or(rest.len());
    Ok(&rest[..end])
}

/// `template` with the one occurrence of `url` served over https.
pub fn upgrade_url(template: &str, url: &str) -> Result<String, String> {
    let https = url
        .strip_prefix("http://")
        .map(|rest| format!("https://{rest}"))
        .ok_or("not an http:// URL")?;
    if template.matches(url).count() != 1 {
        return Err(format!("the template does not hold {url} exactly once"));
    }
    Ok(template.replacen(url, &https, 1))
}

/// `html` with `alt` added to the first `<img>` tag that has none.
pub fn add_img_alt(html: &str, alt: &str) -> Result<String, String> {
    let mut from = 0;
    while let Some(i) = html[from..].find("<img") {
        let start = from + i;
        let end = html[start..]
            .find('>')
            .map(|e| start + e)
            .ok_or("an <img> tag is never closed")?;
        if !html[start..end].contains("alt=") {
            let mut out = String::with_capacity(html.len() + alt.len() + 8);
            out.push_str(&html[..end]);
            out.push_str(&format!(" alt=\"{alt}\""));
            out.push_str(&html[end..]);
            return Ok(out);
        }
        from = end;
    }
    Err("no <img> tag lacks alt".to_string())
}

/// Sentence case: the first character upper, the rest lower.
pub fn sentence_case(subject: &str) -> String {
    let mut chars = subject.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

/// Apply the fix each family finding of `report` names to the template
/// (`subject`, `html`), given the mail's own decoded HTML for locating a
/// link. An unknown family finding is an error: the oracle fixes only what
/// the task's three named findings ask.
pub fn repair_template(
    report: &Value,
    mail_html: &str,
    subject: &str,
    html: &str,
) -> Result<(String, String), String> {
    let (mut subject, mut html) = (subject.to_string(), html.to_string());
    for finding in family_findings(report)? {
        match finding["rule_id"].as_str().unwrap_or_default() {
            "link.scheme.http.v1" => {
                let offset = finding["location"]["byte_offset"]
                    .as_u64()
                    .ok_or("the link finding has no byte_offset")?;
                html = upgrade_url(&html, located_http_url(mail_html, offset)?)?;
            }
            "html.img.alt-missing.v1" => html = add_img_alt(&html, tasks::MARKETING_OFFER)?,
            "spam.subject.all_caps.v1" => subject = sentence_case(&subject),
            other => return Err(format!("the oracle has no fix for {other}")),
        }
    }
    Ok((subject, html))
}

/// What the oracle spent solving one task: the unpaid difficulty proxy
/// (proposed 0123 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OracleCost {
    pub mcp_tool_calls: u32,
    pub app_requests: u32,
}

/// Run the scripted solution for `env.task`.
pub async fn run_oracle(env: &AgentEnv) -> Result<(), String> {
    run_oracle_costed(env).await.map(|_| ())
}

/// As [`run_oracle`], returning the MCP `tools/call` requests and the
/// application requests the solution made.
pub async fn run_oracle_costed(env: &AgentEnv) -> Result<OracleCost, String> {
    let app = App::new(&env.app_url, &env.identity)?;
    let mut mcp = McpClient::connect(&env.mcp_url).await?;
    solve(env, &app, &mut mcp).await?;
    Ok(OracleCost {
        mcp_tool_calls: mcp.tool_calls(),
        app_requests: app.requests(),
    })
}

async fn solve(env: &AgentEnv, app: &App, mcp: &mut McpClient) -> Result<(), String> {
    let inbox = env.inbox.as_str();
    match env.task.as_str() {
        "signup" | "confirm-decoys" | "smtp-retry" | "inbox-isolation" => {
            let mail = mcp
                .wait_emails(inbox, Some("Confirm your account"), 1)
                .await?;
            confirm_from_text(app, &mail[0]).await
        }
        "sibling-addresses" => {
            let mails = mcp
                .wait_emails(inbox, Some("Confirm your account"), 2)
                .await?;
            let mine = mails
                .iter()
                .find(|m| {
                    m["to"]
                        .as_array()
                        .is_some_and(|to| to.iter().any(|a| a["address"] == json!(env.identity)))
                })
                .ok_or("no confirmation addressed to my exact identity")?;
            confirm_from_text(app, mine).await
        }
        "recipient-crowd" => {
            let mails = mcp
                .wait_emails(inbox, Some("Confirm your account"), 5)
                .await?;
            if mails.len() != 5 {
                return Err(format!("expected five confirmations, saw {}", mails.len()));
            }
            let mine = mails
                .iter()
                .find(|m| {
                    m["to"]
                        .as_array()
                        .is_some_and(|to| to.iter().any(|a| a["address"] == json!(env.identity)))
                })
                .ok_or("no confirmation addressed to my exact identity")?;
            confirm_from_text(app, mine).await
        }
        "i18n-subject" => {
            let mails = mcp.wait_emails(inbox, None, 2).await?;
            let mine = mails
                .iter()
                .find(|m| m["subject"] == json!(tasks::I18N_SUBJECT))
                .ok_or("no mail with the requested subject")?;
            confirm_from_text(app, mine).await
        }
        "slow-delivery" => {
            let first = mcp
                .wait(inbox, Some("Confirm your account"), 1, 1000)
                .await?;
            if first["timed_out"] != json!(true) {
                return Err(format!("the held mail arrived before release: {first}"));
            }
            ok("release", app.post("/deliveries/release", json!({})).await?)?;
            let mail = mcp
                .wait_emails(inbox, Some("Confirm your account"), 1)
                .await?;
            confirm_from_text(app, &mail[0]).await
        }
        "otp-backlog" => {
            let want = tasks::OTP_BACKLOG;
            let mails = mcp
                .wait_emails(inbox, Some("Your verification code"), want)
                .await?;
            if mails.len() != want {
                return Err(format!("expected {want} code mails, saw {}", mails.len()));
            }
            let mut newest: Option<(u32, String)> = None;
            for mail in &mails {
                let text = text_of(mail);
                let minutes = requested_minutes(&text).ok_or("a code mail has no request line")?;
                let code = six_digit_code(&text).ok_or("no six-digit code in the mail")?;
                if newest.as_ref().is_none_or(|(best, _)| minutes < *best) {
                    newest = Some((minutes, code));
                }
            }
            let (_, code) = newest.ok_or("no code mails")?;
            ok(
                "verify",
                app.post("/otp/verify", json!({ "code": code })).await?,
            )?;
            Ok(())
        }
        "digest-lookalike" => {
            let mail = mcp
                .wait_emails(inbox, Some("Your weekly digest"), 1)
                .await?;
            let html = mail[0]["html"].as_str().ok_or("no HTML body")?;
            let mut right = html_anchors(html).into_iter().filter(|(href, text)| {
                text == tasks::DIGEST_ITEM
                    && wrapper_destination_host(href).as_deref() == Some(tasks::DIGEST_PUBLISHER)
            });
            let (link, _) = right
                .next()
                .ok_or("no exact-title anchor on the publisher")?;
            if right.next().is_some() {
                return Err("more than one exact-title anchor on the publisher".to_string());
            }
            ok("open article", app.get(&link).await?)?;
            Ok(())
        }
        "otp" | "stale-otp" => {
            let want = if env.task == "otp" { 1 } else { 2 };
            let mails = mcp
                .wait_emails(inbox, Some("Your verification code"), want)
                .await?;
            let current = if want == 1 {
                &mails[0]
            } else {
                mails
                    .iter()
                    .find(|m| text_of(m).contains("Any earlier code no longer works"))
                    .ok_or("no resent code")?
            };
            let text = text_of(current);
            let code = text
                .split(|c: char| !c.is_ascii_digit())
                .find(|w| w.len() == 6)
                .ok_or("no six-digit code in the mail")?;
            ok(
                "verify",
                app.post("/otp/verify", json!({ "code": code })).await?,
            )?;
            Ok(())
        }
        "magic-login" => {
            let mail = mcp.wait_emails(inbox, Some("Your sign-in link"), 1).await?;
            let link = labelled_url(&text_of(&mail[0]), "Sign in to your account:")
                .ok_or("no sign-in link")?;
            ok("magic login", app.get(&link).await?)?;
            Ok(())
        }
        "entity-url" => {
            let mail = mcp.wait_emails(inbox, Some("Autumn sale"), 1).await?;
            let html = mail[0]["html"].as_str().ok_or("no HTML body")?;
            let href = anchor_href(html, &format!("Claim the {}", tasks::MARKETING_OFFER))
                .ok_or("no offer anchor")?;
            ok("offer", app.get(&href).await?)?;
            Ok(())
        }
        "digest-forty" => {
            let mail = mcp
                .wait_emails(inbox, Some("Your weekly digest"), 1)
                .await?;
            let text = text_of(&mail[0]);
            let mut lines = text.lines();
            let suffix = format!(". {}", tasks::DIGEST_ITEM);
            lines
                .by_ref()
                .find(|l| l.trim_end().ends_with(&suffix))
                .ok_or("the requested article is not listed")?;
            let link = lines
                .next()
                .ok_or("no link under the article")?
                .trim()
                .to_string();
            ok("open article", app.get(&link).await?)?;
            Ok(())
        }
        "reset-login" | "reset-reuse" => {
            let mail = mcp
                .wait_emails(inbox, Some("Reset your password"), 1)
                .await?;
            let link =
                labelled_url(&text_of(&mail[0]), "Reset your password:").ok_or("no reset link")?;
            ok("reset page", app.get(&link).await?)?;
            let token = last_segment(&link);
            let password = "correct horse battery staple";
            ok(
                "reset",
                app.post(
                    "/reset/complete",
                    json!({ "token": token, "password": password }),
                )
                .await?,
            )?;
            if env.task == "reset-reuse" {
                let (status, body) = app
                    .post(
                        "/reset/complete",
                        json!({ "token": token, "password": "a second password" }),
                    )
                    .await?;
                if status != 410 {
                    return Err(format!(
                        "the replayed reset was not refused: {status} {body}"
                    ));
                }
            }
            ok(
                "login",
                app.post("/login", json!({ "password": password })).await?,
            )?;
            Ok(())
        }
        "invite-time" => {
            let mail = mcp.wait_emails(inbox, Some("Invitation:"), 1).await?;
            let link = labelled_url(&text_of(&mail[0]), "Open the invitation:")
                .ok_or("no invitation link")?;
            let page = ok("invite page", app.get(&link).await?)?;
            let ics_url = page["ics_url"].as_str().ok_or("no ics_url")?;
            let (status, bytes) = app.get_raw(ics_url).await?;
            if status != 200 {
                return Err(format!("ICS download: status {status}"));
            }
            let ics = String::from_utf8(bytes).map_err(|_| "ICS is not UTF-8")?;
            let uid = ics_property(&ics, "UID").ok_or("no UID")?;
            let start = ics_property(&ics, "DTSTART").ok_or("no DTSTART")?;
            ok(
                "accept",
                app.post(
                    "/invite/accept",
                    json!({ "token": last_segment(&link), "uid": uid, "start": start }),
                )
                .await?,
            )?;
            Ok(())
        }
        "support-reply" => {
            let mail = mcp.wait_emails(inbox, Some("[Ticket #"), 1).await?;
            let subject = mail[0]["subject"].as_str().unwrap_or_default();
            let ticket: String = subject
                .split('#')
                .nth(1)
                .ok_or("no ticket number")?
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let text = text_of(&mail[0]);
            let challenge = text
                .lines()
                .find_map(|l| l.trim().strip_prefix("Reply challenge:"))
                .ok_or("no reply challenge")?
                .trim()
                .to_string();
            ok(
                "reply",
                app.post(
                    "/support/reply",
                    json!({ "ticket": ticket, "challenge": challenge, "reply": "The export button does nothing when I click it." }),
                )
                .await?,
            )?;
            Ok(())
        }
        "otp-correction" => {
            // Neither mail holds the current code: the first mail's first
            // four digits plus the two the correction says to put in place of
            // its last two.
            let mails = mcp.wait_emails(inbox, None, 2).await?;
            let mut correction = None;
            let mut first = None;
            for mail in &mails {
                let text = text_of(mail);
                if let Some(digits) = correction_digits(&text) {
                    correction = Some(digits);
                } else if let Some(code) = six_digit_code(&text) {
                    first = Some(code);
                }
            }
            let (old, new) = correction.ok_or("no correction mail")?;
            let first = first.ok_or("no first code mail")?;
            if !first.ends_with(&old) {
                return Err("the correction does not match the first code".to_string());
            }
            let code = format!("{}{new}", &first[..first.len() - old.len()]);
            ok(
                "verify",
                app.post("/otp/verify", json!({ "code": code })).await?,
            )?;
            Ok(())
        }
        "digest-correction" => {
            let mails = mcp.wait_emails(inbox, None, 2).await?;
            let retracted = mails
                .iter()
                .find_map(|m| retracted_section(&text_of(m)))
                .ok_or("no correction retracting a section")?;
            let digest = mails
                .iter()
                .find(|m| retracted_section(&text_of(m)).is_none())
                .ok_or("no digest mail")?;
            let mut kept = exact_title_links_by_section(&text_of(digest))
                .into_iter()
                .filter(|(section, _)| section != retracted);
            let (_, link) = kept
                .next()
                .ok_or("no exact-title link outside the retracted section")?;
            if kept.next().is_some() {
                return Err(
                    "more than one exact-title link outside the retracted section".to_string(),
                );
            }
            ok("open article", app.get(&link).await?)?;
            Ok(())
        }
        "otp-late" => {
            // The restart invalidated every code mailed so far, and the
            // resend mails the new code in two parts: ask for it, then join
            // the head and the tail among the code mails that were not there
            // before. The reminder after them repeats a superseded code.
            let before = mcp
                .wait_emails(inbox, Some("Your verification code"), 2)
                .await?;
            let seen: Vec<&Value> = before.iter().map(|m| &m["id"]).collect();
            ok("resend", app.post("/otp/resend", json!({})).await?)?;
            let after = mcp
                .wait_emails(inbox, Some("Your verification code"), 4)
                .await?;
            let fresh: Vec<&Value> = after.iter().filter(|m| !seen.contains(&&m["id"])).collect();
            if fresh.len() != 2 {
                return Err(format!(
                    "expected two code mails after the resend, found {}",
                    fresh.len()
                ));
            }
            let head = fresh
                .iter()
                .find_map(|m| code_head(&text_of(m)))
                .ok_or("no head part after the resend")?;
            let tail = fresh
                .iter()
                .find_map(|m| code_tail(&text_of(m)))
                .ok_or("no tail part after the resend")?;
            ok(
                "verify",
                app.post("/otp/verify", json!({ "code": format!("{head}{tail}") }))
                    .await?,
            )?;
            Ok(())
        }
        "confirm-attachment" => {
            let mails = mcp
                .wait_emails(inbox, Some("Confirm your account"), 1)
                .await?;
            let mail = &mails[0];
            let id = mail["id"].as_str().ok_or("the mail has no id")?;
            let list = mcp.list_attachments(id).await?;
            if list["attachments"][0]["filename"] != json!(tasks::CONFIRMATION_FILE) {
                return Err(format!(
                    "the first attachment is not {}",
                    tasks::CONFIRMATION_FILE
                ));
            }
            let part = mcp.get_attachment(id, 0).await?;
            if part["encoding"] != json!("utf-8") {
                return Err("the attachment is not UTF-8 text".to_string());
            }
            let link = labelled_url(
                part["content"].as_str().unwrap_or_default(),
                "Confirm your account:",
            )
            .ok_or("no confirmation link in the attachment")?;
            ok("confirm", app.get(&link).await?)?;
            Ok(())
        }
        "confirm-injection" => {
            // Apply the application's retraction, follow the confirmation's
            // other link, and never read the newest mail: it only claims the
            // task has changed.
            let mails = mcp
                .wait_emails(inbox, Some("Confirm your account"), 3)
                .await?;
            let retracted = mails
                .iter()
                .filter(|m| m["subject"] == json!("Correction: Confirm your account"))
                .find_map(|m| retracted_label(&text_of(m)))
                .ok_or("no correction retracting a link")?;
            let confirmation = mails
                .iter()
                .find(|m| m["subject"] == json!("Confirm your account"))
                .ok_or("no confirmation email with the exact subject")?;
            let text = text_of(confirmation);
            let mut kept = tasks::CONFIRM_LABELS
                .into_iter()
                .filter(|label| *label != retracted)
                .filter_map(|label| labelled_url(&text, &format!("{label}:")));
            let link = kept.next().ok_or("no link outside the retracted label")?;
            if kept.next().is_some() {
                return Err("more than one link outside the retracted label".to_string());
            }
            ok("confirm", app.get(&link).await?)?;
            Ok(())
        }
        "newsletter-findings" => {
            // Read the report over MCP, fix each family finding it names,
            // re-send, find the new mail as the one not seen before (the POST
            // returned after the final 250, decision 0001, so no wait is
            // needed) and demand a clean report for it.
            let first = mcp.wait_emails(inbox, None, 1).await?;
            let first_id = first[0]["id"].as_str().ok_or("the newsletter has no id")?;
            let report = mcp
                .call(
                    "swarmail_get_deliverability_report",
                    json!({ "id": first_id }),
                )
                .await?;
            let template = ok("read the template", app.get("/newsletter").await?)?;
            let (subject, html) = repair_template(
                &report,
                first[0]["html"]
                    .as_str()
                    .ok_or("the newsletter has no HTML")?,
                template["subject"]
                    .as_str()
                    .ok_or("the template has no subject")?,
                template["html"]
                    .as_str()
                    .ok_or("the template has no html")?,
            )?;
            ok(
                "re-send",
                app.post("/newsletter", json!({ "subject": subject, "html": html }))
                    .await?,
            )?;
            let all = mcp.wait_emails(inbox, None, 2).await?;
            let fresh: Vec<&Value> = all.iter().filter(|m| m["id"] != json!(first_id)).collect();
            let [fresh] = fresh[..] else {
                return Err(format!("expected one new mail, saw {}", fresh.len()));
            };
            let again = mcp
                .call(
                    "swarmail_get_deliverability_report",
                    json!({ "id": fresh["id"] }),
                )
                .await?;
            let left = family_findings(&again)?;
            if !left.is_empty() {
                return Err(format!("{} family findings remain", left.len()));
            }
            Ok(())
        }
        other => Err(format!("the oracle has no solution for task {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(id: &str, location: Value) -> Value {
        json!({"rule_id": id, "severity": "warning", "location": location, "fix_hint": "x"})
    }

    #[test]
    fn a_newsletter_report_is_repaired_by_rule_id() {
        let mail = tasks::NEWSLETTER_HTML.replace('\n', "\r\n");
        let at = mail.find("http://").unwrap();
        let report = json!({"rule_set_id": "x", "dns": {}, "findings": [
            finding("bulk.one_click.v1", json!({"kind":"header","name":"list-unsubscribe","occurrence":null})),
            finding("html.img.alt-missing.v1", json!({"kind":"mime_part","path":[]})),
            finding("link.scheme.http.v1", json!({"kind":"link","part_path":[],"byte_offset":at})),
            finding("spam.subject.all_caps.v1", json!({"kind":"header","name":"subject","occurrence":0})),
        ]});
        let (subject, html) = repair_template(
            &report,
            &mail,
            tasks::NEWSLETTER_SUBJECT,
            tasks::NEWSLETTER_HTML,
        )
        .unwrap();
        assert_eq!(subject, "Autumn lamp offer");
        assert!(html.contains("<img src=\"https://shop.example/lamp.png\" alt=\"Autumn Lamp\">"));
        assert!(html.contains("href=\"https://shop.example/autumn\""));
        assert!(!html.contains("http://"));
        // An unknown family finding is an error, never a silent skip.
        let mut unknown = report.clone();
        unknown["findings"].as_array_mut().unwrap().push(finding(
            "spam.subject.punctuation.v1",
            json!({"kind":"header","name":"subject","occurrence":0}),
        ));
        assert!(repair_template(&unknown, &mail, "S", tasks::NEWSLETTER_HTML).is_err());
    }

    #[test]
    fn repair_helpers_refuse_what_they_cannot_locate() {
        assert!(located_http_url("<a href=\"http://x/y\">", 9).is_ok());
        assert_eq!(
            located_http_url("<a href=\"http://x/y\">", 9).unwrap(),
            "http://x/y"
        );
        assert!(located_http_url("<a href=\"http://x/y\">", 8).is_err());
        assert!(located_http_url("abc", 99).is_err());
        assert!(upgrade_url("a http://x b", "https://x").is_err());
        assert!(upgrade_url("a b", "http://x").is_err());
        assert!(upgrade_url("http://x http://x", "http://x").is_err());
        assert_eq!(
            upgrade_url("a http://x b", "http://x").unwrap(),
            "a https://x b"
        );
        assert!(add_img_alt("<p>", "a").is_err());
        assert!(add_img_alt("<img alt=\"z\">", "a").is_err());
        assert!(add_img_alt("<img src=x", "a").is_err());
        assert_eq!(
            add_img_alt("<img alt=\"z\"><img src=\"y\">", "a").unwrap(),
            "<img alt=\"z\"><img src=\"y\" alt=\"a\">"
        );
        assert_eq!(sentence_case("AUTUMN LAMP"), "Autumn lamp");
        assert_eq!(sentence_case(""), "");
    }

    #[test]
    fn a_malformed_report_never_reads_as_clean() {
        for bad in [
            json!([]),
            json!({}),
            json!({"findings": {}}),
            json!({"findings": [1]}),
            json!({"findings": [{"rule_id": 3}]}),
            json!({"findings": [{"severity": "x"}]}),
        ] {
            assert!(family_findings(&bad).is_err(), "{bad}");
        }
        assert_eq!(
            family_findings(&json!({"findings": [{"rule_id": "dkim.x.v1"}]}))
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn entity_decoding_and_anchor_selection() {
        assert_eq!(
            decode_entities("a&amp;b&#61;c&#x3D;d&bogus;"),
            "a&b=c=d&bogus;"
        );
        let html = "<a href=\"http://x/1\">Unsubscribe</a><a href=\"http://x/o?a&#61;1&amp;b&#61;2\">Claim the lamp</a>";
        assert_eq!(
            anchor_href(html, "Claim").as_deref(),
            Some("http://x/o?a=1&b=2")
        );
        assert_eq!(anchor_href(html, "Nope"), None);
    }

    #[test]
    fn html_anchors_are_read_in_order_and_malformed_tails_stop() {
        let html = "<li><a href=\"http://x/a?u=1&amp;v=2\"> One &amp; two </a></li><a href=\"http://x/b\">Two</a>";
        assert_eq!(
            html_anchors(html),
            vec![
                ("http://x/a?u=1&v=2".to_string(), "One & two".to_string()),
                ("http://x/b".to_string(), "Two".to_string()),
            ]
        );
        assert!(html_anchors("<a href=\"http://x").is_empty());
        assert!(html_anchors("<a href=\"http://x\" no close").is_empty());
        assert!(html_anchors("<a href=\"http://x\">no end").is_empty());
        assert!(html_anchors("plain text").is_empty());
    }

    #[test]
    fn requested_minutes_reads_the_generation_line() {
        let text = "Your verification code is 123456.\n\nRequested 44 minutes before this email was sent.\nIt expires in 10 minutes.\n";
        assert_eq!(requested_minutes(text), Some(44));
        assert_eq!(
            requested_minutes("Requested 0 minutes before this email was sent.\n"),
            Some(0)
        );
        assert_eq!(requested_minutes("Your code is 123456.\n"), None);
        assert_eq!(
            requested_minutes("Requested at 2026-10-24 12:04:00 UTC\n"),
            None
        );
        assert_eq!(
            requested_minutes("Requested soon minutes before this email was sent.\n"),
            None
        );
        assert_eq!(
            requested_minutes("Requested  minutes before this email was sent.\n"),
            None
        );
        assert_eq!(six_digit_code(text).as_deref(), Some("123456"));
        assert_eq!(six_digit_code("only 12345 and 1234567"), None);
    }

    #[test]
    fn wrapper_hosts_are_exact_not_substrings() {
        let wrap = |dest: &str| format!("http://127.0.0.1:1/a/tok?u={dest}");
        assert_eq!(
            wrapper_destination_host(&wrap("https://news.fixture.eval/async")).as_deref(),
            Some("news.fixture.eval")
        );
        // The tracker suffix makes it a different host.
        assert_eq!(
            wrapper_destination_host(&wrap("https://news.fixture.eval.click-tracker.net/async"))
                .as_deref(),
            Some("news.fixture.eval.click-tracker.net")
        );
        // Percent-encoded, with a port, userinfo, mixed case and a further query.
        assert_eq!(
            wrapper_destination_host(&wrap("https%3A%2F%2FNews.Fixture.Eval%3A443%2Fx%3Fa%3D1"))
                .as_deref(),
            Some("news.fixture.eval")
        );
        assert_eq!(
            wrapper_destination_host(&wrap("https://news.fixture.eval@evil.test/x")).as_deref(),
            Some("evil.test")
        );
        // Another parameter first, then no destination at all, then junk.
        assert_eq!(
            wrapper_destination_host("http://h/a/t?x=1&u=https://a.test/p").as_deref(),
            Some("a.test")
        );
        assert_eq!(wrapper_destination_host("http://h/a/t"), None);
        assert_eq!(wrapper_destination_host("http://h/a/t?x=1"), None);
        assert_eq!(wrapper_destination_host(&wrap("ftp://a.test/")), None);
        assert_eq!(wrapper_destination_host(&wrap("https:///x")), None);
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("a%zzb"), "a%zzb");
    }

    #[test]
    fn correction_digits_read_exactly_two_and_two() {
        let text = "Keep the first four digits from that email and replace its last two digits, 83, with 07.\n\nThe code in that email no longer works.\n";
        assert_eq!(
            correction_digits(text),
            Some(("83".to_string(), "07".to_string()))
        );
        // A missing line, a missing tail, extra digits and a missing stop.
        assert_eq!(correction_digits("Your code is 123456.\n"), None);
        assert_eq!(
            correction_digits("replace its last two digits, 83 with 07."),
            None
        );
        assert_eq!(
            correction_digits("replace its last two digits, 8, with 07."),
            None
        );
        assert_eq!(
            correction_digits("replace its last two digits, 83, with 071."),
            None
        );
        assert_eq!(
            correction_digits("replace its last two digits, 83, with 07"),
            None
        );
        assert_eq!(
            correction_digits("replace its last two digits, 8x, with 07."),
            None
        );
    }

    #[test]
    fn the_retracted_section_must_be_one_of_the_digests_sections() {
        let said = |section: &str| {
            format!(
                "A correction to this week's digest: the \"{}\" link under {section} pointed to the wrong article and has been retracted.\n",
                tasks::DIGEST_ITEM
            )
        };
        assert_eq!(retracted_section(&said("Top stories")), Some("Top stories"));
        assert_eq!(
            retracted_section(&said("All articles")),
            Some("All articles")
        );
        assert_eq!(retracted_section(&said("Sports")), None);
        assert_eq!(retracted_section("No correction here."), None);
        assert_eq!(
            retracted_section(&format!(
                "the \"{}\" link under Top stories was fine",
                tasks::DIGEST_ITEM
            )),
            None
        );
    }

    #[test]
    fn exact_titles_are_listed_with_their_section_and_link() {
        let text = format!(
            "This week's digest.\n\nTop stories\n- Other (part 1)\n  http://x/1\n- {t}\n  http://x/2\n\nAll articles\n1. Other (part 1)\n   http://x/3\n2. {t}\n   http://x/4\n3. {t} (part 2)\n   http://x/5\n",
            t = tasks::DIGEST_ITEM
        );
        assert_eq!(
            exact_title_links_by_section(&text),
            vec![
                ("Top stories".to_string(), "http://x/2".to_string()),
                ("All articles".to_string(), "http://x/4".to_string()),
            ]
        );
        assert!(exact_title_links_by_section("no items").is_empty());
        // An item with nothing under it names no link.
        assert!(
            exact_title_links_by_section(&format!("Top stories\n- {}", tasks::DIGEST_ITEM))
                .is_empty()
        );
    }

    #[test]
    fn code_parts_are_exactly_three_digits_in_their_own_sentence() {
        let head = "Your new verification code begins with 123.\n\nFor your security, its remaining digits follow in a separate email.\n";
        let tail = "The remaining digits of your new verification code are 456.\n\nEnter all six digits. The code expires in 10 minutes.\n";
        assert_eq!(code_head(head).as_deref(), Some("123"));
        assert_eq!(code_tail(tail).as_deref(), Some("456"));
        // Each helper reads only its own sentence.
        assert_eq!(code_head(tail), None);
        assert_eq!(code_tail(head), None);
        // Two digits, four digits, a missing stop, a non-digit and no sentence.
        for bad in [
            "Your new verification code begins with 12.",
            "Your new verification code begins with 1234.",
            "Your new verification code begins with 123",
            "Your new verification code begins with 12x.",
            "Your verification code is 123456.",
        ] {
            assert_eq!(code_head(bad), None, "{bad}");
        }
        for bad in [
            "The remaining digits of your new verification code are 45.",
            "The remaining digits of your new verification code are 4567.",
            "The remaining digits of your new verification code are 456",
            "A digit run of 456 elsewhere. The remaining digits follow.",
        ] {
            assert_eq!(code_tail(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_retracted_label_must_be_one_of_the_confirmations_labels() {
        let said = |label: &str| {
            format!(
                "A correction to our previous email: the \"{label}\" link in it was generated incorrectly and has been retracted. The other link in that email is correct.\n"
            )
        };
        for label in tasks::CONFIRM_LABELS {
            assert_eq!(retracted_label(&said(label)), Some(label));
        }
        assert_eq!(retracted_label(&said("Click here")), None);
        assert_eq!(retracted_label("No correction here."), None);
        assert_eq!(
            retracted_label("the \"Confirm your account\" link in it was fine"),
            None
        );
    }

    #[test]
    fn labelled_urls_and_ics_properties() {
        let text = "Unsubscribe: http://a/1\nConfirm your account: http://a/2\n";
        assert_eq!(
            labelled_url(text, "Confirm your account:").as_deref(),
            Some("http://a/2")
        );
        let ics = "BEGIN:VEVENT\r\nUID:u@x\r\nDTSTART:20261026T150000Z\r\nEND:VEVENT\r\n";
        assert_eq!(
            ics_property(ics, "DTSTART").as_deref(),
            Some("20261026T150000Z")
        );
        assert_eq!(ics_property(ics, "UID").as_deref(), Some("u@x"));
    }
}
