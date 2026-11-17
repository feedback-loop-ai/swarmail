//! A scripted loopback MCP server and SMTP server for the API section's
//! producer tests (story B repairs): real HTTP/1.1 and SMTP framing, with
//! each answer — and its timing — chosen by the test. They are not targets:
//! they provoke orderings and refusals no real target can be made to show
//! on demand (a read held past a deadline, a wait answered before the
//! delivery is acknowledged, a busy refusal by source).

#![allow(dead_code)]

use serde_json::{Value, json};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

/// One `tools/call` answer.
pub enum Answer {
    /// A tool result whose text block is this JSON.
    Result(Value),
    /// A JSON-RPC error with this code and message.
    Error(i64, String),
}

pub type Handler =
    Arc<dyn Fn(String, Value, u64) -> Pin<Box<dyn Future<Output = Answer> + Send>> + Send + Sync>;

/// Serve MCP over HTTP: `initialize` and notifications answered as the
/// protocol requires, every `tools/call` by `handler(tool, arguments, n)`
/// where `n` counts calls from 1. Returns the `/mcp` URL.
pub async fn mcp(handler: Handler) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let calls = Arc::new(AtomicU64::new(0));
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let (handler, calls) = (handler.clone(), calls.clone());
            tokio::spawn(async move {
                let mut buf = Vec::new();
                loop {
                    let mut chunk = [0u8; 8192];
                    let Ok(n) = s.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..end]).to_ascii_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .map_or(0, |v| v.trim().parse::<usize>().unwrap());
                        if buf.len() < end + 4 + len {
                            break;
                        }
                        let req: Value =
                            serde_json::from_slice(&buf[end + 4..end + 4 + len]).unwrap();
                        buf.drain(..end + 4 + len);
                        let Some(id) = req.get("id").cloned() else {
                            if s.write_all(b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n")
                                .await
                                .is_err()
                            {
                                return;
                            }
                            continue;
                        };
                        let reply = match req["method"].as_str().unwrap_or_default() {
                            "initialize" => json!({"jsonrpc": "2.0", "id": id, "result": {
                                "protocolVersion": "2025-03-26"}}),
                            _ => {
                                let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                                let tool = req["params"]["name"].as_str().unwrap_or_default();
                                let args = req["params"]["arguments"].clone();
                                match handler(tool.to_owned(), args, n).await {
                                    Answer::Result(v) => json!({"jsonrpc": "2.0", "id": id,
                                        "result": {"isError": false, "content": [
                                            {"type": "text", "text": v.to_string()}]}}),
                                    Answer::Error(code, message) => json!({"jsonrpc": "2.0",
                                        "id": id, "error": {"code": code, "message": message}}),
                                }
                            }
                        }
                        .to_string();
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{reply}",
                            reply.len()
                        );
                        if s.write_all(resp.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                }
            });
        }
    });
    format!("http://{addr}/mcp")
}

/// The ten read fixtures' listing in Swarmail's `{total, emails}` shape.
pub fn swarmail_page(items: &[(String, String, String)]) -> Value {
    let emails: Vec<Value> = items
        .iter()
        .map(|(id, to, subject)| json!({"id": id, "to": [{"address": to}], "subject": subject}))
        .collect();
    json!({"total": emails.len(), "emails": emails})
}

/// When a scripted SMTP server answers.
#[derive(Clone)]
pub struct SmtpScript {
    /// Hold the `250` after the DATA terminator until this fires.
    pub hold_data_ack: Option<Arc<tokio::sync::Notify>>,
    /// Given two permits when the terminator has arrived (the message is
    /// "sent"), one for each party that waits on it.
    pub data_received: Option<Arc<tokio::sync::Semaphore>>,
    /// Delay the `221` to `QUIT`.
    pub quit_delay: Duration,
}

/// A minimal ESMTP server: greeting, EHLO (with AUTH PLAIN), AUTH, MAIL,
/// RCPT, DATA and QUIT, each answered as `script` says.
pub async fn smtp(script: SmtpScript) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((s, _)) = listener.accept().await {
            let script = script.clone();
            tokio::spawn(async move {
                let (r, mut w) = s.into_split();
                let mut r = BufReader::new(r);
                if w.write_all(b"220 scripted ESMTP\r\n").await.is_err() {
                    return;
                }
                let mut in_data = false;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).await.unwrap_or(0) == 0 {
                        return;
                    }
                    if in_data {
                        if line == ".\r\n" {
                            in_data = false;
                            if let Some(n) = &script.data_received {
                                n.add_permits(2);
                            }
                            if let Some(n) = &script.hold_data_ack {
                                n.notified().await;
                            }
                            if w.write_all(b"250 2.0.0 queued\r\n").await.is_err() {
                                return;
                            }
                        }
                        continue;
                    }
                    let verb = line.get(..4).unwrap_or_default().to_ascii_uppercase();
                    let reply: &[u8] = match verb.as_str() {
                        "EHLO" | "HELO" => b"250-scripted\r\n250 AUTH PLAIN\r\n",
                        "AUTH" => b"235 2.7.0 ok\r\n",
                        "MAIL" | "RCPT" | "RSET" | "NOOP" => b"250 ok\r\n",
                        "DATA" => {
                            in_data = true;
                            b"354 go\r\n"
                        }
                        "QUIT" => {
                            tokio::time::sleep(script.quit_delay).await;
                            let _ = w.write_all(b"221 bye\r\n").await;
                            return;
                        }
                        _ => b"502 no\r\n",
                    };
                    if w.write_all(reply).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    addr
}
