//! End-to-end guarantees: the whole point of Swarmail.
//!
//! These tests bind the real SMTP + HTTP servers on ephemeral ports and speak
//! real protocol — no mocks, no stubs.

use swarmail::{RunningServer, config::Config};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

async fn start() -> RunningServer {
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
async fn smtp_send(
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
async fn http_json(
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

#[tokio::test]
async fn single_mail_roundtrip_with_extraction() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        None,
        "noreply@alkem.io",
        "user@example.com",
        "Verify your account",
        "Click https://x.io/verify?t=abc. Your code is 424242.",
    )
    .await
    .unwrap();

    let (status, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    let email = &body["emails"][0];
    assert_eq!(email["subject"], "Verify your account");
    assert_eq!(email["to"][0]["address"], "user@example.com");
    assert_eq!(email["links"][0], "https://x.io/verify?t=abc");
    assert_eq!(email["codes"][0], "424242");
}

#[tokio::test]
async fn the_burst_guarantee() {
    // 50 concurrent sessions x 20 mails = 1000. Every accepted DATA must be
    // queryable, immediately, exactly — the promise MailCrab cannot make.
    let srv = start().await;
    const CONNS: usize = 50;
    const PER_CONN: usize = 20;

    let mut tasks = Vec::new();
    for c in 0..CONNS {
        let smtp_addr = srv.smtp_addr;
        tasks.push(tokio::spawn(async move {
            for i in 0..PER_CONN {
                smtp_send(
                    smtp_addr,
                    None,
                    "noreply@test.io",
                    &format!("user{c}-{i}@example.com"),
                    &format!("burst {c}/{i}"),
                    "load",
                )
                .await
                .unwrap_or_else(|e| panic!("conn {c} mail {i} rejected: {e}"));
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(
        body["count"], 1000,
        "every accepted mail must be stored — no silent loss"
    );
}

#[tokio::test]
async fn await_waits_for_arrival() {
    let srv = start().await;

    // Deliver 300ms in the future; await must block until it arrives.
    let smtp_addr = srv.smtp_addr;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        smtp_send(
            smtp_addr,
            None,
            "noreply@x.io",
            "late@example.com",
            "late mail",
            "hello",
        )
        .await
        .unwrap();
    });

    let (status, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/await?to=late@example.com&timeout_ms=5000&count=1",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["matched"], 1);
    assert_eq!(body["emails"][0]["subject"], "late mail");
}

#[tokio::test]
async fn await_times_out_cleanly() {
    let srv = start().await;
    let (status, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/await?to=nobody@example.com&timeout_ms=200",
        None,
    )
    .await;
    assert_eq!(status, 408);
    assert_eq!(body["matched"], 0);
}

#[tokio::test]
async fn per_inbox_isolation_via_auth() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("proj-42"),
        "noreply@x.io",
        "a@example.com",
        "s",
        "b",
    )
    .await
    .unwrap();
    smtp_send(
        srv.smtp_addr,
        None,
        "noreply@x.io",
        "b@example.com",
        "s",
        "b",
    )
    .await
    .unwrap();

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/proj-42/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 1);

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 1);
}

#[tokio::test]
async fn chaos_can_reject_rcpt_and_recovery_works() {
    let srv = start().await;

    // 100% RCPT rejection.
    let (status, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"rcpt": {"probability": 100, "error": "451 4.3.0 chaos"}}"#),
    )
    .await;
    assert_eq!(status, 200);

    let err = smtp_send(srv.smtp_addr, None, "n@x.io", "a@example.com", "s", "b")
        .await
        .unwrap_err();
    assert!(err.contains("451"), "chaos error must surface: {err}");

    // Chaos off — delivery works again.
    let (status, _) = http_json(srv.http_addr, "DELETE", "/api/v1/chaos", None).await;
    assert_eq!(status, 200);
    smtp_send(srv.smtp_addr, None, "n@x.io", "a@example.com", "s", "b")
        .await
        .unwrap();

    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(body["count"], 1);
}
