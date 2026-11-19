//! The API section's SMTP sender: one bounded dialogue per session, shared
//! by the wire driver and the eval crate (which holds no SMTP client of its
//! own). It sends the fill and the authored fixtures, outside every measured
//! window. Every exchange is bounded by `step`; nothing retries, and a
//! failure after the terminating dot is ambiguous and is never resent.
//!
//! `AUTH PLAIN` is offered for Swarmail only: its username names the inbox
//! a per-test session delivers into (0004), which is the product feature
//! the API section measures as such.

use base64::Engine as _;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

const MAX_LINE: usize = 1024;
const MAX_LINES: usize = 64;

/// Where one send ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    Accepted,
    /// A refusal before the terminator: the server kept nothing.
    Refused(u16),
    /// A failure before the terminator left: nothing was kept.
    Failed,
    /// After the terminator: the server may or may not have kept it.
    Ambiguous,
}

pub struct Session {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    step: Duration,
}

impl Session {
    /// Connect, read the banner and say EHLO.
    pub async fn open(addr: SocketAddr, step: Duration) -> Result<Session, String> {
        let stream = tokio::time::timeout(step, TcpStream::connect(addr))
            .await
            .map_err(|_| "smtp connect: deadline".to_owned())?
            .map_err(|e| format!("smtp connect: {e}"))?;
        let _ = stream.set_nodelay(true);
        let (r, w) = stream.into_split();
        let mut s = Session {
            reader: BufReader::new(r),
            writer: w,
            step,
        };
        match s.reply().await? {
            220 => {}
            c => return Err(format!("smtp banner {c}")),
        }
        match s.command(b"EHLO api.mail-compare.invalid\r\n").await? {
            250 => Ok(s),
            c => Err(format!("smtp EHLO {c}")),
        }
    }

    /// `AUTH PLAIN` with `user` and a fixed password.
    pub async fn auth(&mut self, user: &str) -> Result<(), String> {
        let token = base64::engine::general_purpose::STANDARD
            .encode(format!("\0{user}\0api-bench").as_bytes());
        match self
            .command(format!("AUTH PLAIN {token}\r\n").as_bytes())
            .await?
        {
            235 => Ok(()),
            c => Err(format!("smtp AUTH {c}")),
        }
    }

    async fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        tokio::time::timeout(self.step, self.writer.write_all(bytes))
            .await
            .map_err(|_| "smtp write: deadline".to_owned())?
            .map_err(|e| format!("smtp write: {e}"))
    }

    async fn command(&mut self, line: &[u8]) -> Result<u16, String> {
        self.write(line).await?;
        self.reply().await
    }

    /// One reply, possibly multi-line: its code.
    async fn reply(&mut self) -> Result<u16, String> {
        let step = self.step;
        tokio::time::timeout(step, async {
            for _ in 0..MAX_LINES {
                let mut line = Vec::new();
                let n = (&mut self.reader)
                    .take(MAX_LINE as u64)
                    .read_until(b'\n', &mut line)
                    .await
                    .map_err(|e| format!("smtp read: {e}"))?;
                if n == 0 || line.last() != Some(&b'\n') {
                    return Err("smtp reply: closed or oversized".to_owned());
                }
                let code = std::str::from_utf8(line.get(..3).unwrap_or_default())
                    .ok()
                    .and_then(|c| c.parse::<u16>().ok())
                    .ok_or("smtp reply: malformed")?;
                match line.get(3) {
                    Some(b' ' | b'\r' | b'\n') => return Ok(code),
                    Some(b'-') => continue,
                    _ => return Err("smtp reply: malformed".to_owned()),
                }
            }
            Err("smtp reply: too many lines".to_owned())
        })
        .await
        .map_err(|_| "smtp reply: deadline".to_owned())?
    }

    /// One transaction from `from` to `rcpt` carrying `bytes` (CRLF lines).
    pub async fn send(&mut self, from: &str, rcpt: &str, bytes: &[u8]) -> Sent {
        for (line, want) in [
            (format!("MAIL FROM:<{from}>\r\n"), 250),
            (format!("RCPT TO:<{rcpt}>\r\n"), 250),
            ("DATA\r\n".to_owned(), 354),
        ] {
            match self.command(line.as_bytes()).await {
                Ok(c) if c == want => {}
                Ok(c) => return Sent::Refused(c),
                Err(_) => return Sent::Failed,
            }
        }
        let mut payload = dot_stuff(bytes);
        payload.extend_from_slice(b".\r\n");
        if self.write(&payload).await.is_err() {
            return Sent::Ambiguous;
        }
        match self.reply().await {
            Ok(c) if (200..300).contains(&c) => Sent::Accepted,
            Ok(c) => Sent::Refused(c),
            Err(_) => Sent::Ambiguous,
        }
    }

    pub async fn quit(mut self) {
        let _ = self.command(b"QUIT\r\n").await;
    }
}

/// RFC 5321 §4.5.2 transparency.
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
