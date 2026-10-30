//! The bounded wire client every target shares (spec R3): one authored
//! corpus, one SMTP dialogue, one HTTP reader. Every read is capped in
//! bytes and bounded by a deadline; nothing retries.

use crate::protocol::MessageKind;
use serde::Serialize;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// An SMTP reply line longer than this is a protocol violation for the
/// client's purposes, not a reason to buffer more.
pub const MAX_REPLY_LINE: usize = 1024;
pub const MAX_REPLY_LINES: usize = 64;

/// One authored message. `token` is the identity the observers match on;
/// every token of a run has the same length, so one cannot be a substring
/// of another.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub token: String,
    pub kind: MessageKind,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub bytes: Vec<u8>,
}

pub const FROM: &str = "sender@mail-compare.invalid";
pub const TO: &str = "rcpt@mail-compare.invalid";

/// SplitMix64: the corpus derives its words from the protocol's seed, so
/// the same protocol authors byte-identical mail on every target.
fn mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

const WORDS: [&str; 16] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa",
];

fn sentence(state: &mut u64, words: usize) -> String {
    (0..words)
        .map(|_| WORDS[(mix(state) % 16) as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Message `index` of the run tagged `run_tag`. Deterministic in
/// `(seed, run_tag, index, kinds)`.
pub fn authored(seed: u64, run_tag: &str, index: u32, kinds: &[MessageKind]) -> Message {
    let kind = kinds[index as usize % kinds.len()];
    let token = format!("mc-{run_tag}-{index:06}");
    let mut state = seed ^ u64::from(index).wrapping_mul(0xA24B_AED4_963E_E407);
    let subject = format!("mail-compare {token} {}", sentence(&mut state, 3));
    let words = sentence(&mut state, 24);
    let text = format!("{token} {words}");
    let html = format!("<p>{token}</p><p>{words}</p>");
    let mut head = format!(
        "From: <{FROM}>\r\nTo: <{TO}>\r\nSubject: {subject}\r\nMessage-ID: <{token}@mail-compare.invalid>\r\nDate: Wed, 28 Oct 2026 00:00:00 +0000\r\nMIME-Version: 1.0\r\n"
    );
    let (body, text, html) = match kind {
        MessageKind::Plain => {
            head.push_str("Content-Type: text/plain; charset=utf-8\r\n");
            (format!("{text}\r\n"), Some(text), None)
        }
        MessageKind::Html => {
            head.push_str("Content-Type: text/html; charset=utf-8\r\n");
            (format!("{html}\r\n"), None, Some(html))
        }
        MessageKind::Multipart => {
            let b = format!("b-{token}");
            head.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{b}\"\r\n"
            ));
            let body = format!(
                "--{b}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text}\r\n--{b}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}\r\n--{b}--\r\n"
            );
            (body, Some(text), Some(html))
        }
    };
    let bytes = format!("{head}\r\n{body}").into_bytes();
    Message {
        token,
        kind,
        subject,
        text,
        html,
        bytes,
    }
}

/// Where a send ended. `Ambiguous` is any failure after the terminating
/// dot left the client: the server may or may not have kept the message,
/// and nothing retries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum Disposition {
    Accepted {
        code: u16,
    },
    Rejected {
        phase: &'static str,
        code: u16,
    },
    FailedBeforeTerminator {
        phase: &'static str,
        reason: &'static str,
    },
    Ambiguous {
        reason: &'static str,
    },
}

/// Monotonic nanoseconds since the run's origin, sampled by the one
/// controller process; never a cross-process wall-clock difference.
#[derive(Debug, Clone, Copy)]
pub struct Clock(pub Instant);

impl Clock {
    pub fn now_ns(&self) -> u64 {
        self.0.elapsed().as_nanos() as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    Timeout,
    Closed,
    Io,
    Oversized,
    Malformed,
    /// A scan that could not establish completeness: a repeated id, a
    /// moving count or pages that stop short.
    Incomplete,
}

impl WireError {
    pub fn reason(self) -> &'static str {
        match self {
            WireError::Timeout => "deadline",
            WireError::Closed => "closed",
            WireError::Io => "io",
            WireError::Oversized => "oversized",
            WireError::Malformed => "malformed",
            WireError::Incomplete => "incomplete",
        }
    }
}

pub struct Smtp {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    step: Duration,
}

impl Smtp {
    /// Connect, read the banner, say EHLO. `step` bounds every single
    /// exchange.
    pub async fn open(addr: SocketAddr, step: Duration) -> Result<Smtp, (u16, WireError)> {
        let stream = tokio::time::timeout(step, TcpStream::connect(addr))
            .await
            .map_err(|_| (0, WireError::Timeout))?
            .map_err(|_| (0, WireError::Io))?;
        let (r, w) = stream.into_split();
        let mut s = Smtp {
            reader: BufReader::new(r),
            writer: w,
            step,
        };
        let banner = s.reply().await.map_err(|e| (0, e))?;
        if banner != 220 {
            return Err((banner, WireError::Malformed));
        }
        let ehlo = s
            .command(b"EHLO mail-compare.invalid\r\n")
            .await
            .map_err(|e| (0, e))?;
        if ehlo != 250 {
            return Err((ehlo, WireError::Malformed));
        }
        Ok(s)
    }

    async fn command(&mut self, line: &[u8]) -> Result<u16, WireError> {
        self.write(line).await?;
        self.reply().await
    }

    async fn write(&mut self, bytes: &[u8]) -> Result<(), WireError> {
        tokio::time::timeout(self.step, self.writer.write_all(bytes))
            .await
            .map_err(|_| WireError::Timeout)?
            .map_err(|_| WireError::Io)
    }

    /// One reply, possibly multi-line; returns its code.
    pub async fn reply(&mut self) -> Result<u16, WireError> {
        let step = self.step;
        tokio::time::timeout(step, async {
            for _ in 0..MAX_REPLY_LINES {
                let line = read_line(&mut self.reader).await?;
                if line.len() < 4 || !line[..3].iter().all(u8::is_ascii_digit) {
                    return Err(WireError::Malformed);
                }
                let code = std::str::from_utf8(&line[..3]).map_err(|_| WireError::Malformed)?;
                let code: u16 = code.parse().map_err(|_| WireError::Malformed)?;
                match line[3] {
                    b' ' | b'\r' => return Ok(code),
                    b'-' => continue,
                    _ => return Err(WireError::Malformed),
                }
            }
            Err(WireError::Oversized)
        })
        .await
        .map_err(|_| WireError::Timeout)?
    }

    /// One transaction. `sent_terminator` returns the instant the final
    /// dot was written; `acked` the instant the DATA reply arrived.
    pub async fn send(&mut self, msg: &Message, clock: &Clock) -> SendRecord {
        let start_ns = clock.now_ns();
        let mut rec = SendRecord {
            start_ns,
            terminator_ns: None,
            ack_ns: None,
            disposition: Disposition::Ambiguous {
                reason: "unreached",
            },
        };
        let steps: [(&'static str, String, u16); 3] = [
            ("mail", format!("MAIL FROM:<{FROM}>\r\n"), 250),
            ("rcpt", format!("RCPT TO:<{TO}>\r\n"), 250),
            ("data", "DATA\r\n".into(), 354),
        ];
        for (phase, line, want) in steps {
            match self.command(line.as_bytes()).await {
                Ok(code) if code == want => {}
                Ok(code) => {
                    rec.disposition = Disposition::Rejected { phase, code };
                    return rec;
                }
                Err(e) => {
                    rec.disposition = Disposition::FailedBeforeTerminator {
                        phase,
                        reason: e.reason(),
                    };
                    return rec;
                }
            }
        }
        let mut payload = dot_stuff(&msg.bytes);
        payload.extend_from_slice(b".\r\n");
        if let Err(e) = self.write(&payload).await {
            // Some of the terminator may have left: the server's view is
            // unknown from here on.
            rec.disposition = Disposition::Ambiguous { reason: e.reason() };
            return rec;
        }
        rec.terminator_ns = Some(clock.now_ns());
        match self.reply().await {
            Ok(code) => {
                rec.ack_ns = Some(clock.now_ns());
                rec.disposition = if (200..300).contains(&code) {
                    Disposition::Accepted { code }
                } else {
                    Disposition::Rejected {
                        phase: "final-data",
                        code,
                    }
                };
            }
            Err(e) => rec.disposition = Disposition::Ambiguous { reason: e.reason() },
        }
        rec
    }

    /// QUIT and drop; a failed QUIT changes no recorded disposition.
    pub async fn quit(mut self) {
        let _ = self.command(b"QUIT\r\n").await;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SendRecord {
    pub start_ns: u64,
    pub terminator_ns: Option<u64>,
    pub ack_ns: Option<u64>,
    #[serde(flatten)]
    pub disposition: Disposition,
}

async fn read_line(r: &mut BufReader<OwnedReadHalf>) -> Result<Vec<u8>, WireError> {
    let mut line = Vec::with_capacity(64);
    loop {
        let byte = match r.read_u8().await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(WireError::Closed);
            }
            Err(_) => return Err(WireError::Io),
        };
        line.push(byte);
        if byte == b'\n' {
            return Ok(line);
        }
        if line.len() >= MAX_REPLY_LINE {
            return Err(WireError::Oversized);
        }
    }
}

/// RFC 5321 §4.5.2 transparency: a line that begins with a dot gets one
/// more. The authored corpus ends with CRLF, so the terminator follows a
/// line break.
pub fn dot_stuff(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 8);
    let mut at_line_start = true;
    for &b in bytes {
        if at_line_start && b == b'.' {
            out.push(b'.');
        }
        out.push(b);
        at_line_start = b == b'\n';
    }
    out
}

/// A loopback HTTP reader with a byte cap and a per-request deadline. It
/// never consults proxy variables (the controller's environment is not the
/// experiment's) and counts every request it makes, so polling overhead is
/// part of the record.
pub struct Http {
    client: reqwest::Client,
    base: String,
    max_bytes: usize,
    requests: std::sync::atomic::AtomicU64,
}

impl Http {
    pub fn new(addr: SocketAddr, deadline: Duration, max_bytes: u64) -> Http {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(deadline)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("a plain loopback client builds");
        Http {
            client,
            base: format!("http://{addr}"),
            max_bytes: usize::try_from(max_bytes).unwrap_or(usize::MAX),
            requests: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn requests(&self) -> u64 {
        self.requests.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// GET `path`; the status and the body, capped.
    pub async fn get(&self, path: &str) -> Result<(u16, Vec<u8>), WireError> {
        self.get_within(path, None).await
    }

    /// GET with a deadline other than the client's (a long-poll that must
    /// outlive the ordinary request deadline by its own wait).
    pub async fn get_within(
        &self,
        path: &str,
        deadline: Option<Duration>,
    ) -> Result<(u16, Vec<u8>), WireError> {
        self.requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut req = self.client.get(format!("{}{path}", self.base));
        if let Some(d) = deadline {
            req = req.timeout(d);
        }
        let mut resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                WireError::Timeout
            } else {
                WireError::Io
            }
        })?;
        let status = resp.status().as_u16();
        let mut body = Vec::new();
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    if body.len() + chunk.len() > self.max_bytes {
                        return Err(WireError::Oversized);
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(None) => return Ok((status, body)),
                Err(e) if e.is_timeout() => return Err(WireError::Timeout),
                Err(_) => return Err(WireError::Io),
            }
        }
    }

    pub async fn get_json(&self, path: &str) -> Result<(u16, serde_json::Value), WireError> {
        let (status, body) = self.get(path).await?;
        let value = serde_json::from_slice(&body).map_err(|_| WireError::Malformed)?;
        Ok((status, value))
    }
}

/// Percent-encode a query value (RFC 3986 unreserved kept).
pub fn query_value(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

/// Body comparison that ignores only the transport's line-ending and
/// trailing-whitespace choices; any other byte difference is a mismatch.
pub fn same_body(authored: Option<&str>, observed: Option<&str>) -> bool {
    let norm = |s: &str| s.replace("\r\n", "\n").trim_end().to_owned();
    match (authored, observed) {
        (None, None) => true,
        (Some(a), Some(o)) => norm(a) == norm(o),
        (None, Some(o)) => o.trim().is_empty(),
        (Some(_), None) => false,
    }
}
