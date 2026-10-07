//! Concurrent accept-all SMTP server.
//!
//! One tokio task per connection; the protocol state machine is intentionally
//! simple and the store insert is synchronous — accepting a message means it is
//! stored. There is no async gap where mail can be lost.

use crate::chaos::{Chaos, ChaosEvent};
use crate::extract::{extract_codes, extract_links};
use crate::model::{Email, EmailAddress};
use crate::store::Store;
use chrono::Utc;
use mail_parser::{Addr, Address, MessageParser};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

/// Hard cap on a single message (50 MiB).
const MAX_MESSAGE_SIZE: usize = 50 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// Hostname announced in the banner.
    pub hostname: String,
    /// Accept any AUTH credentials; the username names the target inbox.
    pub accept_any_auth: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            hostname: "swarmail.local".to_string(),
            accept_any_auth: true,
        }
    }
}

static SESSIONS: AtomicU64 = AtomicU64::new(0);

pub async fn serve(
    listener: TcpListener,
    store: Arc<Store>,
    chaos: Arc<Chaos>,
    cfg: SmtpConfig,
) -> io::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let session = SESSIONS.fetch_add(1, Ordering::Relaxed);
        debug!(%peer, session, "smtp connection accepted");
        let store = store.clone();
        let chaos = chaos.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, store, chaos, cfg).await {
                debug!(session, error = %e, "connection ended");
            }
        });
    }
}

async fn write_line<W: tokio::io::AsyncWrite + Unpin>(
    stream: &mut W,
    line: &str,
) -> io::Result<()> {
    stream.write_all(line.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    Ok(())
}

/// Extract the bare address from "MAIL FROM:<a@b> SIZE=1" style arguments.
fn arg_address(arg: &str) -> String {
    match (arg.find('<'), arg.find('>')) {
        (Some(a), Some(b)) if b > a => arg[a + 1..b].to_string(),
        _ => arg.split(':').nth(1).unwrap_or(arg).trim().to_string(),
    }
}

async fn handle_conn(
    mut stream: TcpStream,
    store: Arc<Store>,
    chaos: Arc<Chaos>,
    cfg: SmtpConfig,
) -> io::Result<()> {
    // Chaos: connect event can drop the session before the banner.
    if let Some(rule) = chaos.check(ChaosEvent::Connect) {
        tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
        if let Some(err) = &rule.error {
            write_line(&mut stream, err).await.ok();
        }
        return Ok(());
    }

    let mut reader = BufReader::new(stream);
    write_line(
        reader.get_mut(),
        &format!("220 {} Swarmail ready", cfg.hostname),
    )
    .await?;

    let mut mail_from: Option<String> = None;
    let mut rcpts: Vec<String> = Vec::new();
    let mut inbox = "default".to_string();

    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(()); // peer closed
        }
        let cmd_line = line.trim_end_matches(['\r', '\n']);
        let (verb, arg) = match cmd_line.find(' ') {
            Some(i) => (
                cmd_line[..i].to_ascii_uppercase(),
                cmd_line[i + 1..].to_string(),
            ),
            None => (cmd_line.to_ascii_uppercase(), String::new()),
        };

        match verb.as_str() {
            "HELO" | "EHLO" => {
                let mut resp = format!("250-{}\r\n", cfg.hostname);
                resp.push_str("250-PIPELINING\r\n250-8BITMIME\r\n250-SMTPUTF8\r\n250-SIZE ");
                resp.push_str(&MAX_MESSAGE_SIZE.to_string());
                if cfg.accept_any_auth {
                    resp.push_str("\r\n250-AUTH PLAIN LOGIN");
                }
                resp.push_str("\r\n250 OK");
                write_line(reader.get_mut(), &resp).await?;
            }
            "STARTTLS" => {
                // Phase B: self-signed TLS. Kratos dev flows use disable_starttls anyway.
                write_line(reader.get_mut(), "454 4.7.0 TLS not available").await?;
            }
            "AUTH" => {
                if !cfg.accept_any_auth {
                    write_line(reader.get_mut(), "502 5.5.2 AUTH disabled").await?;
                    continue;
                }
                let mut parts = arg.split_whitespace();
                let mechanism = parts.next().unwrap_or("").to_ascii_uppercase();
                let b64 = parts.next().map(|s| s.to_string());
                match mechanism.as_str() {
                    "PLAIN" => {
                        let b64 = match b64 {
                            Some(b) => Some(b),
                            None => {
                                write_line(reader.get_mut(), "334 ").await?;
                                line.clear();
                                reader.read_line(&mut line).await?;
                                Some(line.trim_end_matches(['\r', '\n']).to_string())
                            }
                        };
                        if let Some(inbox_name) = auth_plain_inbox(b64.as_deref()) {
                            inbox = inbox_name;
                        }
                        write_line(reader.get_mut(), "235 2.7.0 Accepted").await?;
                    }
                    "LOGIN" => {
                        write_line(reader.get_mut(), "334 VXNlcm5hbWU6").await?;
                        line.clear();
                        reader.read_line(&mut line).await?;
                        if let Some(user) = b64_decode(line.trim_end_matches(['\r', '\n'])) {
                            inbox = String::from_utf8_lossy(&user).trim().to_string();
                        }
                        write_line(reader.get_mut(), "334 UGFzc3dvcmQ6").await?;
                        line.clear();
                        reader.read_line(&mut line).await?;
                        write_line(reader.get_mut(), "235 2.7.0 Accepted").await?;
                    }
                    _ => {
                        write_line(
                            reader.get_mut(),
                            "504 5.5.4 Unrecognized authentication type",
                        )
                        .await?
                    }
                }
            }
            "MAIL" => {
                if let Some(rule) = chaos.check(ChaosEvent::MailFrom) {
                    tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
                    write_line(
                        reader.get_mut(),
                        rule.error.as_deref().unwrap_or("451 4.3.0 chaos"),
                    )
                    .await?;
                    continue;
                }
                mail_from = Some(arg_address(&arg));
                rcpts.clear();
                write_line(reader.get_mut(), "250 2.1.0 OK").await?;
            }
            "RCPT" => {
                if let Some(rule) = chaos.check(ChaosEvent::Rcpt) {
                    tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
                    write_line(
                        reader.get_mut(),
                        rule.error.as_deref().unwrap_or("451 4.3.0 chaos"),
                    )
                    .await?;
                    continue;
                }
                if mail_from.is_none() {
                    write_line(reader.get_mut(), "503 5.5.1 Error: need MAIL command").await?;
                    continue;
                }
                rcpts.push(arg_address(&arg));
                write_line(reader.get_mut(), "250 2.1.5 OK").await?;
            }
            "DATA" => {
                if rcpts.is_empty() {
                    write_line(reader.get_mut(), "503 5.5.1 Error: need RCPT command").await?;
                    continue;
                }
                write_line(reader.get_mut(), "354 End data with <CR><LF>.<CR><LF>").await?;

                let mut raw: Vec<u8> = Vec::with_capacity(4096);
                let mut over_limit = false;
                loop {
                    line.clear();
                    let n = reader.read_line(&mut line).await?;
                    if n == 0 {
                        return Ok(()); // peer vanished mid-DATA; nothing to store
                    }
                    if line == ".\r\n" || line == ".\n" {
                        break;
                    }
                    // Dot-unstuffing: RFC 5321 §4.5.2
                    let payload: &[u8] = if line.as_bytes().starts_with(b"..") {
                        &line.as_bytes()[1..]
                    } else {
                        line.as_bytes()
                    };
                    if raw.len() + payload.len() > MAX_MESSAGE_SIZE {
                        over_limit = true;
                    } else {
                        raw.extend_from_slice(payload);
                    }
                }
                if over_limit {
                    write_line(reader.get_mut(), "552 5.3.4 Message too big").await?;
                    continue;
                }
                if let Some(rule) = chaos.check(ChaosEvent::Data) {
                    tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
                    if rule.error.is_some() {
                        // Mail is intentionally NOT stored — that is the chaos.
                        write_line(
                            reader.get_mut(),
                            rule.error.as_deref().unwrap_or("451 4.3.0 chaos"),
                        )
                        .await?;
                        continue;
                    }
                }

                let email =
                    build_email(&raw, &inbox, &rcpts, mail_from.clone().unwrap_or_default());
                let id = email.id.clone();
                store.insert(email);
                write_line(reader.get_mut(), &format!("250 2.0.0 OK: stored as {id}")).await?;
            }
            "RSET" => {
                mail_from = None;
                rcpts.clear();
                write_line(reader.get_mut(), "250 2.0.0 OK").await?;
            }
            "NOOP" => write_line(reader.get_mut(), "250 2.0.0 OK").await?,
            "VRFY" => write_line(reader.get_mut(), "252 2.1.5 Cannot VRFY user").await?,
            "QUIT" => {
                write_line(reader.get_mut(), "221 2.0.0 Bye").await?;
                return Ok(());
            }
            _ => write_line(reader.get_mut(), "502 5.5.2 Command not recognized").await?,
        }
    }
}

fn auth_plain_inbox(b64: Option<&str>) -> Option<String> {
    let decoded = b64_decode(b64?)?;
    // PLAIN: authzid NUL authcid NUL passwd — the authcid names the inbox.
    let parts: Vec<&[u8]> = decoded.split(|b| *b == 0).collect();
    let user = parts.get(1).or_else(|| parts.first())?;
    let user = String::from_utf8_lossy(user).trim().to_string();
    if user.is_empty() { None } else { Some(user) }
}

fn b64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input.trim())
        .ok()
}

fn collect_addresses(address: Option<&Address>) -> Vec<EmailAddress> {
    let mut out = Vec::new();
    let mut push = |spec: &Addr| {
        let address = spec
            .address
            .as_ref()
            .map(|c| c.to_string())
            .unwrap_or_default();
        if !address.is_empty() {
            out.push(EmailAddress {
                name: spec.name.as_ref().map(|c| c.to_string()),
                address,
            });
        }
    };
    match address {
        Some(Address::List(list)) => {
            for spec in list {
                push(spec);
            }
        }
        Some(Address::Group(groups)) => {
            for group in groups {
                for spec in &group.addresses {
                    push(spec);
                }
            }
        }
        None => {}
    }
    out
}

pub fn build_email(raw: &[u8], inbox: &str, recipients: &[String], from_envelope: String) -> Email {
    let now = Utc::now();
    let parsed = MessageParser::default().parse(raw);

    let mut from = parsed
        .as_ref()
        .and_then(|m| m.from())
        .and_then(|a| match a {
            Address::List(list) => list.first().map(|spec| EmailAddress {
                name: spec.name.as_ref().map(|c| c.to_string()),
                address: spec
                    .address
                    .as_ref()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| from_envelope.clone()),
            }),
            _ => None,
        });
    if from.is_none() && !from_envelope.is_empty() {
        from = Some(EmailAddress {
            name: None,
            address: from_envelope,
        });
    }

    let to = parsed
        .as_ref()
        .map(|m| collect_addresses(m.to()))
        .unwrap_or_default();
    let cc = parsed
        .as_ref()
        .map(|m| collect_addresses(m.cc()))
        .unwrap_or_default();
    let subject = parsed
        .as_ref()
        .and_then(|m| m.subject())
        .map(|s| s.to_string());
    let text = parsed
        .as_ref()
        .and_then(|m| m.body_text(0))
        .map(|s| s.to_string());
    let html = parsed
        .as_ref()
        .and_then(|m| m.body_html(0))
        .map(|s| s.to_string());

    let links = extract_links(text.as_deref(), html.as_deref());
    let codes = extract_codes(text.as_deref(), html.as_deref());

    Email {
        id: uuid::Uuid::now_v7().to_string(),
        inbox: inbox.to_string(),
        from,
        to,
        cc,
        recipients: recipients.to_vec(),
        subject,
        received_at: now.to_rfc3339(),
        received_ms: now.timestamp_millis(),
        size: raw.len() as u64,
        text,
        html,
        links,
        codes,
        raw: raw.to_vec(),
    }
}
