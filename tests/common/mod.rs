//! Shared end-to-end helpers: real servers, real protocol, no mocks.
#![allow(dead_code)] // compiled per test binary; not every binary uses every helper

use swarmail::{RunningServer, config::Config};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[allow(dead_code)]
pub async fn start() -> RunningServer {
    start_with(|c| c).await
}
pub async fn start_with(override_cfg: impl FnOnce(Config) -> Config) -> RunningServer {
    let cfg = override_cfg(Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: None,
        smtp: Default::default(),
    });
    swarmail::run_on(&cfg).await.unwrap()
}

/// Minimal raw-SMTP client: returns the final reply line of the session,
/// failing on any 4xx/5xx (which is exactly what chaos tests need).
pub async fn smtp_send(
    addr: std::net::SocketAddr,
    inbox: Option<&str>,
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    let stream = TcpStream::connect(addr).await.unwrap();
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    let mut line = String::new();

    // read greeting
    line.clear();
    reader.read_line(&mut line).await.unwrap();
    if !line.starts_with("220") {
        return Err(line.trim_end().to_string());
    }

    // EHLO (read multi-line reply)
    w.write_all(b"EHLO swarmail-test\r\n").await.unwrap();
    loop {
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        if !line.starts_with("250-") {
            break;
        }
    }

    // Optional AUTH PLAIN — username names the inbox.
    if let Some(inbox) = inbox {
        let plain = format!("\u{0}{inbox}\u{0}whatever");
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
        w.write_all(format!("AUTH PLAIN {b64}\r\n").as_bytes())
            .await
            .unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        if !line.starts_with("235") {
            return Err(line.trim_end().to_string());
        }
    }

    for cmd in [format!("MAIL FROM:<{from}>"), format!("RCPT TO:<{to}>")] {
        w.write_all(format!("{cmd}\r\n").as_bytes()).await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        if !line.starts_with("250") {
            return Err(line.trim_end().to_string());
        }
    }

    w.write_all(b"DATA\r\n").await.unwrap();
    line.clear();
    reader.read_line(&mut line).await.unwrap();
    if !line.starts_with("354") {
        return Err(line.trim_end().to_string());
    }

    let message = format!("From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{body}\r\n.\r\n");
    w.write_all(message.as_bytes()).await.unwrap();
    line.clear();
    reader.read_line(&mut line).await.unwrap();
    if !line.starts_with("250") {
        return Err(line.trim_end().to_string());
    }

    w.write_all(b"QUIT\r\n").await.unwrap();
    Ok(line.trim_end().to_string())
}

/// Start with a custom SMTP config (auth flags, message-size limit).
pub async fn start_smtp(smtp_cfg: swarmail::smtp::SmtpConfig) -> RunningServer {
    start_with(|mut c| {
        c.smtp = smtp_cfg;
        c
    })
    .await
}

/// Step-by-step SMTP client for edge-case protocol tests: full control over
/// every line, no strictness — that is the point.
pub struct SmtpConn {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    write: tokio::net::tcp::OwnedWriteHalf,
}

impl SmtpConn {
    /// Connect without asserting the greeting (chaos may replace it).
    pub async fn connect_raw(addr: std::net::SocketAddr) -> SmtpConn {
        let stream = TcpStream::connect(addr).await.unwrap();
        let (r, w) = stream.into_split();
        SmtpConn {
            reader: BufReader::new(r),
            write: w,
        }
    }

    pub async fn connect(addr: std::net::SocketAddr) -> SmtpConn {
        let mut conn = SmtpConn::connect_raw(addr).await;
        let greeting = conn.reply().await;
        assert!(greeting.starts_with("220"), "bad greeting: {greeting}");
        conn
    }

    pub async fn send(&mut self, line: &str) {
        self.write
            .write_all(format!("{line}\r\n").as_bytes())
            .await
            .unwrap();
    }

    /// Read one reply line (final line of multi-line replies).
    pub async fn reply(&mut self) -> String {
        let mut line = String::new();
        loop {
            line.clear();
            self.reader.read_line(&mut line).await.unwrap();
            let cont = line.starts_with("250-");
            if !cont {
                return line.trim_end().to_string();
            }
        }
    }

    /// Read a full multi-line reply (EHLO etc.), lines joined with \n.
    pub async fn reply_multi(&mut self) -> String {
        let mut all = Vec::new();
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line).await.unwrap();
            let cont = line.starts_with("250-");
            all.push(line.trim_end().to_string());
            if !cont {
                return all.join("\n");
            }
        }
    }

    /// Send a DATA payload and return the final reply.
    pub async fn data(&mut self, body: &str) -> String {
        self.send("DATA").await;
        let resp = self.reply().await;
        assert!(resp.starts_with("354"), "DATA refused: {resp}");
        self.send(body).await;
        self.send(".").await;
        self.reply().await
    }
}

/// GET a path and return (status, body-as-text) — for metrics, docs and UI.
pub async fn http_get_text(addr: std::net::SocketAddr, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buf)
        .await
        .unwrap();
    let s = String::from_utf8_lossy(&buf);
    let status: u16 = s
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    // split_once, not split: bodies (like /raw) may themselves contain \r\n\r\n.
    (
        status,
        s.split_once("\r\n\r\n")
            .map(|x| x.1)
            .unwrap_or("")
            .to_string(),
    )
}

/// Tiny raw-HTTP client — enough for Swarmail's own JSON API, no client dep.
pub async fn http_json(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> (u16, serde_json::Value) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: t\r\nConnection: close\r\n");
    if let Some(b) = body {
        req.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            b.len()
        ));
    }
    req.push_str("\r\n");
    if let Some(b) = body {
        req.push_str(b);
    }
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buf)
        .await
        .unwrap();
    let s = String::from_utf8_lossy(&buf);
    let status: u16 = s
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let body = s.split("\r\n\r\n").nth(1).unwrap_or("{}");
    (
        status,
        serde_json::from_str(body).unwrap_or(serde_json::json!({})),
    )
}
