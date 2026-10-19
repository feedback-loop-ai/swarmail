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
}

impl App {
    pub fn new(base: &str, identity: &str) -> Result<App, String> {
        Ok(App {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| e.to_string())?,
            base: base.trim_end_matches('/').to_string(),
            identity: identity.to_string(),
        })
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

/// The ICS property `name`'s value.
pub fn ics_property(ics: &str, name: &str) -> Option<String> {
    ics.lines().find_map(|l| {
        let l = l.trim_end_matches('\r');
        let (key, value) = l.split_once(':')?;
        (key.split(';').next() == Some(name)).then(|| value.to_string())
    })
}

async fn confirm_from_text(app: &App, email: &Value) -> Result<(), String> {
    let link = labelled_url(&text_of(email), "Confirm your account:")
        .ok_or("no confirmation link in the mail")?;
    ok("confirm", app.get(&link).await?)?;
    Ok(())
}

/// Run the scripted solution for `env.task`.
pub async fn run_oracle(env: &AgentEnv) -> Result<(), String> {
    let app = App::new(&env.app_url, &env.identity)?;
    let mut mcp = McpClient::connect(&env.mcp_url).await?;
    let inbox = env.inbox.as_str();
    match env.task.as_str() {
        "signup" | "confirm-decoys" | "smtp-retry" | "inbox-isolation" => {
            let mail = mcp
                .wait_emails(inbox, Some("Confirm your account"), 1)
                .await?;
            confirm_from_text(&app, &mail[0]).await
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
            confirm_from_text(&app, mine).await
        }
        "i18n-subject" => {
            let mails = mcp.wait_emails(inbox, None, 2).await?;
            let mine = mails
                .iter()
                .find(|m| m["subject"] == json!(tasks::I18N_SUBJECT))
                .ok_or("no mail with the requested subject")?;
            confirm_from_text(&app, mine).await
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
            confirm_from_text(&app, &mail[0]).await
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
        other => Err(format!("the oracle has no solution for task {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
