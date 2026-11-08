//! A minimal plaintext SMTP client and fixed MIME templates (decision 0055
//! §4). No mail-builder crate is in the root lock, so the few shapes the
//! fixture needs are written by hand: text, multipart/alternative,
//! HTML-only and multipart/mixed with a base64 ICS attachment, with an RFC
//! 2047 subject when it is not ASCII.
//!
//! The client authenticates with AUTH PLAIN, the account's configured SMTP
//! identity as the username. For every task but inbox-isolation that identity
//! is the account's inbox (decision 0004); inbox-isolation sends under one
//! shared application login and a routed recipient selects the inbox
//! (decision 0112, which amends 0055 §6 for that task alone).

use crate::state::Attempt;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// The bound on any single SMTP read or write.
pub const IO_TIMEOUT: Duration = Duration::from_secs(10);
/// The longest reply line the client accepts.
const MAX_REPLY_LINE: usize = 4096;
/// The most continuation lines one reply may carry.
const MAX_REPLY_LINES: usize = 64;

/// How one delivery attempt ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendResult {
    /// The final DATA reply was 250: Swarmail stored the mail.
    Delivered(Attempt),
    /// An explicit 4xx before acceptance: safe to retry.
    Transient(Attempt),
    /// A 5xx, or a reply out of protocol: never retried.
    Permanent(Attempt),
    /// The transport failed. After the DATA terminator was sent this is
    /// ambiguous (the mail may be stored), so it is never retried either.
    Transport(Attempt),
}

impl SendResult {
    pub fn attempt(&self) -> &Attempt {
        match self {
            SendResult::Delivered(a)
            | SendResult::Transient(a)
            | SendResult::Permanent(a)
            | SendResult::Transport(a) => a,
        }
    }
}

struct Session {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

/// Read one (possibly multiline) reply and return its code.
async fn read_reply(reader: &mut BufReader<OwnedReadHalf>) -> Result<u16, String> {
    read_reply_text(reader).await.map(|(code, _)| code)
}

/// As [`read_reply`], also returning the reply's final line (the text after
/// the code and its separator).
async fn read_reply_text(reader: &mut BufReader<OwnedReadHalf>) -> Result<(u16, String), String> {
    let mut lines = 0;
    loop {
        let mut line = Vec::new();
        let read = tokio::time::timeout(IO_TIMEOUT, async {
            let mut limited = (&mut *reader).take(MAX_REPLY_LINE as u64 + 1);
            tokio::io::AsyncBufReadExt::read_until(&mut limited, b'\n', &mut line).await
        })
        .await
        .map_err(|_| "reply timed out".to_string())?
        .map_err(|e| format!("read failed: {e}"))?;
        if read == 0 {
            return Err("connection closed".to_string());
        }
        if line.len() > MAX_REPLY_LINE || !line.ends_with(b"\n") {
            return Err("reply line too long or unterminated".to_string());
        }
        let text = String::from_utf8_lossy(&line);
        let text = text.trim_end_matches(['\r', '\n']);
        if text.len() < 3 || !text.as_bytes()[..3].iter().all(u8::is_ascii_digit) {
            return Err(format!("malformed reply {text:?}"));
        }
        let code: u16 = text[..3].parse().expect("three digits");
        match text.as_bytes().get(3) {
            Some(b'-') => {
                lines += 1;
                if lines > MAX_REPLY_LINES {
                    return Err("reply has too many lines".to_string());
                }
            }
            None | Some(b' ') => return Ok((code, text.get(4..).unwrap_or("").to_string())),
            Some(_) => return Err(format!("malformed reply {text:?}")),
        }
    }
}

impl Session {
    async fn command(&mut self, line: &str) -> Result<u16, String> {
        self.write(format!("{line}\r\n").as_bytes()).await?;
        read_reply(&mut self.reader).await
    }

    async fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        tokio::time::timeout(IO_TIMEOUT, self.writer.write_all(bytes))
            .await
            .map_err(|_| "write timed out".to_string())?
            .map_err(|e| format!("write failed: {e}"))
    }
}

/// Normalize line endings to CRLF and apply DATA dot transparency (RFC
/// 5321 §4.5.2): a line that starts with `.` gains one more.
pub fn dot_stuff(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);
    let mut at_line_start = true;
    let mut prev = 0u8;
    for &b in data {
        if b == b'\n' && prev != b'\r' {
            out.push(b'\r');
        }
        if at_line_start && b == b'.' {
            out.push(b'.');
        }
        out.push(b);
        at_line_start = b == b'\n';
        prev = b;
    }
    if !out.ends_with(b"\r\n") {
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// Whether the SMTP dialogue authenticates (proposed decision 0129 §2): the
/// one per-target fixture fact. `Plain` is the dialogue every target but
/// one gets; `None` leaves out the AUTH step and nothing else, for a target
/// that does not accept the fixture's AUTH PLAIN. It is recorded in the
/// product record and the row settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmtpAuth {
    #[default]
    Plain,
    None,
}

impl SmtpAuth {
    pub fn parse(value: &str) -> Option<SmtpAuth> {
        match value {
            "plain" => Some(SmtpAuth::Plain),
            "none" => Some(SmtpAuth::None),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SmtpAuth::Plain => "plain",
            SmtpAuth::None => "none",
        }
    }
}

/// The id Swarmail stored a mail under, read from its final DATA reply
/// (`250 2.0.0 OK: stored as {id}`; decision 0001). `None` for any other
/// text, so a caller never keys on a guess.
pub fn stored_id(reply_text: &str) -> Option<String> {
    let id = reply_text.strip_prefix("2.0.0 OK: stored as ")?;
    (!id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    .then(|| id.to_string())
}

/// Deliver one message: AUTH PLAIN as `inbox`, one recipient, one DATA.
pub async fn send(
    smtp: SocketAddr,
    smtp_identity: &str,
    from: &str,
    to: &str,
    data: &[u8],
) -> SendResult {
    send_stored(smtp, SmtpAuth::Plain, smtp_identity, from, to, data)
        .await
        .0
}

/// As [`send`], also returning the stored id when the final DATA reply is
/// `250 2.0.0 OK: stored as {id}`. The id is never part of an [`Attempt`].
/// The AUTH step is left out when `auth` is `None`.
pub async fn send_stored(
    smtp: SocketAddr,
    auth: SmtpAuth,
    smtp_identity: &str,
    from: &str,
    to: &str,
    data: &[u8],
) -> (SendResult, Option<String>) {
    let attempt = |stage: &str, code: u16| Attempt {
        stage: stage.to_string(),
        code,
    };
    let stream = match tokio::time::timeout(IO_TIMEOUT, TcpStream::connect(smtp)).await {
        Ok(Ok(s)) => s,
        _ => return (SendResult::Transport(attempt("connect", 0)), None),
    };
    let (r, w) = stream.into_split();
    let mut s = Session {
        reader: BufReader::new(r),
        writer: w,
    };
    let token = B64.encode(format!("\0{smtp_identity}\0eval"));
    let steps: Vec<(&str, Option<String>, u16)> = [
        Some(("greeting", None, 220)),
        Some(("ehlo", Some("EHLO fixture.eval".to_string()), 250)),
        (auth == SmtpAuth::Plain).then(|| ("auth", Some(format!("AUTH PLAIN {token}")), 235)),
        Some(("mail_from", Some(format!("MAIL FROM:<{from}>")), 250)),
        Some(("rcpt", Some(format!("RCPT TO:<{to}>")), 250)),
    ]
    .into_iter()
    .flatten()
    .collect();
    for (stage, line, want) in steps {
        let reply = match line {
            None => read_reply(&mut s.reader).await,
            Some(line) => s.command(&line).await,
        };
        match reply {
            Ok(code) if code == want => {}
            Ok(code) => return (classify(stage, code), None),
            Err(_) => return (SendResult::Transport(attempt(stage, 0)), None),
        }
    }
    match s.command("DATA").await {
        Ok(354) => {}
        Ok(code) => return (classify("data", code), None),
        Err(_) => return (SendResult::Transport(attempt("data", 0)), None),
    }
    let mut body = dot_stuff(data);
    body.extend_from_slice(b".\r\n");
    if s.write(&body).await.is_err() {
        return (SendResult::Transport(attempt("dot", 0)), None);
    }
    let (result, id) = match read_reply_text(&mut s.reader).await {
        Ok((250, text)) => (SendResult::Delivered(attempt("dot", 250)), stored_id(&text)),
        Ok((code, _)) => (classify("dot", code), None),
        Err(_) => (SendResult::Transport(attempt("dot", 0)), None),
    };
    let _ = s.command("QUIT").await;
    (result, id)
}

fn classify(stage: &str, code: u16) -> SendResult {
    let a = Attempt {
        stage: stage.to_string(),
        code,
    };
    if (400..500).contains(&code) {
        SendResult::Transient(a)
    } else {
        SendResult::Permanent(a)
    }
}

/// Read the server's greeting and quit: the runner's SMTP readiness probe.
pub async fn probe_greeting(smtp: SocketAddr) -> Result<(), String> {
    let stream = tokio::time::timeout(IO_TIMEOUT, TcpStream::connect(smtp))
        .await
        .map_err(|_| "connect timed out".to_string())?
        .map_err(|e| format!("connect failed: {e}"))?;
    let (r, w) = stream.into_split();
    let mut s = Session {
        reader: BufReader::new(r),
        writer: w,
    };
    match read_reply(&mut s.reader).await? {
        220 => {
            let _ = s.command("QUIT").await;
            Ok(())
        }
        code => Err(format!("greeting was {code}, not 220")),
    }
}

/// An attachment carried base64-encoded in a multipart/mixed message.
#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Text(String),
    Alternative {
        text: String,
        html: String,
    },
    HtmlOnly(String),
    Mixed {
        text: String,
        attachment: Attachment,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: Body,
}

/// An RFC 2047 encoded-word subject when it is not plain ASCII, split on
/// character boundaries so every encoded word stays under 75 octets.
pub fn encode_subject(subject: &str) -> String {
    if subject.is_ascii() && !subject.contains("=?") {
        return subject.to_string();
    }
    let mut words = Vec::new();
    let mut chunk = String::new();
    for c in subject.chars() {
        if chunk.len() + c.len_utf8() > 45 {
            words.push(format!("=?UTF-8?B?{}?=", B64.encode(&chunk)));
            chunk.clear();
        }
        chunk.push(c);
    }
    if !chunk.is_empty() {
        words.push(format!("=?UTF-8?B?{}?=", B64.encode(&chunk)));
    }
    words.join("\r\n ")
}

fn base64_lines(bytes: &[u8]) -> String {
    let encoded = B64.encode(bytes);
    let mut out = String::with_capacity(encoded.len() + encoded.len() / 76 * 2 + 2);
    for chunk in encoded.as_bytes().chunks(76) {
        out.push_str(std::str::from_utf8(chunk).expect("base64 is ASCII"));
        out.push_str("\r\n");
    }
    out
}

/// A text part: 7bit when every line is short ASCII, base64 otherwise, so
/// no line exceeds RFC 5321's 1000 octets whatever the template holds.
fn text_part(content_type: &str, body: &str) -> String {
    let short_ascii = body.is_ascii() && body.lines().all(|l| l.len() <= 900);
    if short_ascii {
        format!(
            "Content-Type: {content_type}; charset=utf-8\r\nContent-Transfer-Encoding: 7bit\r\n\r\n{}\r\n",
            body.replace("\r\n", "\n").replace('\n', "\r\n")
        )
    } else {
        format!(
            "Content-Type: {content_type}; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}",
            base64_lines(body.as_bytes())
        )
    }
}

/// Explicit non-secret render inputs, independent of private RNG consumption.
pub struct RenderContext {
    pub date: chrono::DateTime<chrono::Utc>,
    pub message_id: String,
    pub boundary: String,
}
impl RenderContext {
    pub fn fixture(
        inputs: &crate::provenance::FixtureInputs,
        task: &str,
        slot: u64,
        flow: &str,
        generation: u64,
    ) -> Self {
        Self {
            date: inputs.time(),
            message_id: crate::provenance::hex(
                &inputs.choice(task, slot, flow, generation, "message-id")[..16],
            ),
            boundary: format!(
                "=_eval_{}",
                crate::provenance::hex(
                    &inputs.choice(task, slot, flow, generation, "mime-boundary")[..16]
                )
            ),
        }
    }
}
/// Render the message as RFC 5322 bytes (CRLF line endings, no dot
/// stuffing — `send` applies that on the wire).
pub fn render(msg: &Message) -> Vec<u8> {
    render_with(
        msg,
        &RenderContext::fixture(&Default::default(), "standalone-mail", 0, "mail", 0),
    )
}
pub fn render_with(msg: &Message, context: &RenderContext) -> Vec<u8> {
    let id = &context.message_id;
    let date = context.date.to_rfc2822();
    let mut out = format!(
        "From: Fixture App <{}>\r\nTo: <{}>\r\nSubject: {}\r\nDate: {date}\r\nMessage-ID: <{id}@fixture.eval>\r\nMIME-Version: 1.0\r\n",
        msg.from,
        msg.to,
        encode_subject(&msg.subject)
    );
    match &msg.body {
        Body::Text(text) => out.push_str(&text_part("text/plain", text)),
        Body::HtmlOnly(html) => out.push_str(&text_part("text/html", html)),
        Body::Alternative { text, html } => {
            let b = &context.boundary;
            out.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{b}\"\r\n\r\n--{b}\r\n{}--{b}\r\n{}--{b}--\r\n",
                text_part("text/plain", text),
                text_part("text/html", html)
            ));
        }
        Body::Mixed { text, attachment } => {
            let b = &context.boundary;
            out.push_str(&format!(
                "Content-Type: multipart/mixed; boundary=\"{b}\"\r\n\r\n--{b}\r\n{}--{b}\r\nContent-Type: {}; name=\"{name}\"\r\nContent-Disposition: attachment; filename=\"{name}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}--{b}--\r\n",
                text_part("text/plain", text),
                attachment.content_type,
                base64_lines(&attachment.bytes),
                name = attachment.filename,
            ));
        }
    }
    out.into_bytes()
}

/// A one-event iCalendar invitation (RFC 5545), CRLF-terminated. Times are
/// UTC basic format (`20261026T150000Z`).
pub fn ics(uid: &str, summary: &str, start: &str, end: &str, attendee: &str) -> Vec<u8> {
    ics_with_time(uid, summary, start, end, attendee, chrono::Utc::now())
}
pub fn ics_with_time(
    uid: &str,
    summary: &str,
    start: &str,
    end: &str,
    attendee: &str,
    time: chrono::DateTime<chrono::Utc>,
) -> Vec<u8> {
    let stamp = time.format("%Y%m%dT%H%M%SZ");
    [
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//swarmail-evals//fixture//EN".to_string(),
        "METHOD:REQUEST".to_string(),
        "BEGIN:VEVENT".to_string(),
        format!("UID:{uid}"),
        format!("DTSTAMP:{stamp}"),
        format!("DTSTART:{start}"),
        format!("DTEND:{end}"),
        format!("SUMMARY:{summary}"),
        "ORGANIZER:mailto:events@fixture.eval".to_string(),
        format!("ATTENDEE;RSVP=TRUE:mailto:{attendee}"),
        "END:VEVENT".to_string(),
        "END:VCALENDAR".to_string(),
        String::new(),
    ]
    .join("\r\n")
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_stuffing_doubles_leading_dots_and_normalizes_line_ends() {
        assert_eq!(dot_stuff(b"a\n.b\r\n..c"), b"a\r\n..b\r\n...c\r\n".to_vec());
        assert_eq!(dot_stuff(b".\r\n"), b"..\r\n".to_vec());
        assert_eq!(dot_stuff(b"x.y\r\n"), b"x.y\r\n".to_vec());
    }

    #[test]
    fn the_stored_id_is_read_only_from_the_exact_final_reply() {
        assert_eq!(
            stored_id("2.0.0 OK: stored as abc-123_X"),
            Some("abc-123_X".to_string())
        );
        for bad in [
            "",
            "2.0.0 OK: stored as ",
            "2.0.0 OK: stored as a b",
            "2.0.0 OK: stored as a/b",
            "2.0.0 OK: queued as abc",
            "OK: stored as abc",
        ] {
            assert_eq!(stored_id(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_unicode_subject_becomes_bounded_encoded_words() {
        assert_eq!(encode_subject("Plain"), "Plain");
        let encoded = encode_subject(crate::tasks::I18N_SUBJECT);
        for word in encoded.split("\r\n ") {
            assert!(
                word.starts_with("=?UTF-8?B?") && word.ends_with("?="),
                "{word}"
            );
            assert!(word.len() <= 75, "{word}");
        }
        let decoded: String = encoded
            .split("\r\n ")
            .map(|w| {
                let inner = &w["=?UTF-8?B?".len()..w.len() - 2];
                String::from_utf8(B64.decode(inner).unwrap()).unwrap()
            })
            .collect();
        assert_eq!(decoded, crate::tasks::I18N_SUBJECT);
    }

    #[test]
    fn rendered_lines_stay_under_the_smtp_limit_and_carry_the_attachment() {
        let ics = ics(
            "u1@x",
            "Review",
            "20261026T150000Z",
            "20261026T160000Z",
            "a@b",
        );
        let long = "é".repeat(2000);
        let msg = Message {
            from: "f@x".into(),
            to: "t@x".into(),
            subject: "S".into(),
            body: Body::Mixed {
                text: long,
                attachment: Attachment {
                    filename: "invite.ics".into(),
                    content_type: "text/calendar; method=REQUEST; charset=utf-8".into(),
                    bytes: ics.clone(),
                },
            },
        };
        let raw = render(&msg);
        let text = String::from_utf8(raw).unwrap();
        assert!(text.split("\r\n").all(|l| l.len() <= 998));
        assert!(text.contains(&base64_lines(&ics)));
    }
}
