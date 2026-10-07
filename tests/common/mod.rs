//! Shared end-to-end helpers: real servers, real protocol, no mocks.

use swarmail::{RunningServer, config::Config};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[allow(dead_code)]
pub async fn start() -> RunningServer {
    start_with(|c| c).await
}
pub async fn start_with(
    #[allow(unused_variables)] override_cfg: impl FnOnce(Config) -> Config,
) -> RunningServer {
    let cfg = Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        smtp: Default::default(),
    };
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
