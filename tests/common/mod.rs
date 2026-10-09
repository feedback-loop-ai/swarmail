//! Shared end-to-end helpers: real servers, real protocol, no mocks.
#![allow(dead_code)] // compiled per test binary; not every binary uses every helper

use swarmail::RunningServer;
use swarmail::config::Config;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName};
use tokio_rustls::{TlsConnector, rustls};

#[allow(dead_code)]
pub async fn start() -> RunningServer {
    start_with(|c| c).await
}
pub async fn start_with(override_cfg: impl FnOnce(Config) -> Config) -> RunningServer {
    let cfg = override_cfg(Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: None,
        tls_cert: None,
        tls_key: None,
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

/// A raw POP3 client — enough of RFC 1939 to exercise the server over the
/// wire, like the SMTP client above. No client crate: plain TCP.
pub struct Pop3 {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

#[allow(dead_code)]
impl Pop3 {
    /// Connect and check the greeting.
    pub async fn connect(addr: std::net::SocketAddr) -> Pop3 {
        let stream = TcpStream::connect(addr).await.unwrap();
        let (r, w) = stream.into_split();
        let mut c = Pop3 {
            reader: BufReader::new(r),
            writer: w,
        };
        let greeting = c.reply().await;
        assert!(
            greeting.starts_with("+OK"),
            "unexpected greeting: {greeting}"
        );
        c
    }

    pub async fn send(&mut self, line: &str) {
        self.writer
            .write_all(format!("{line}\r\n").as_bytes())
            .await
            .unwrap();
    }

    /// Read one reply line, trimmed of its CRLF.
    pub async fn reply(&mut self) -> String {
        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();
        line.trim_end().to_string()
    }

    /// Send a command and read its single-line reply.
    pub async fn cmd(&mut self, line: &str) -> String {
        self.send(line).await;
        self.reply().await
    }

    /// Read a multi-line reply body up to the terminating "." line, with
    /// dot-stuffing undone and CRLF line endings restored.
    pub async fn data_bytes(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut line = Vec::new();
            self.reader.read_until(b'\n', &mut line).await.unwrap();
            assert!(!line.is_empty(), "connection closed mid-data");
            let terminated = line.ends_with(b"\n");
            while line.last() == Some(&b'\n') || line.last() == Some(&b'\r') {
                line.pop();
            }
            if line == b"." {
                return out;
            }
            if line.first() == Some(&b'.') {
                line.remove(0);
            }
            out.extend_from_slice(&line);
            if terminated {
                out.extend_from_slice(b"\r\n");
            }
        }
    }

    /// The multi-line body as text (LIST/UIDL/CAPA/TOP assertions).
    pub async fn data(&mut self) -> String {
        String::from_utf8(self.data_bytes().await).unwrap()
    }

    /// USER + PASS round-trip; any password is accepted (decision 0004).
    pub async fn login(&mut self, user: &str) {
        assert!(self.cmd(&format!("USER {user}")).await.starts_with("+OK"));
        assert!(self.cmd("PASS whatever").await.starts_with("+OK"));
    }
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

/// A rustls client that trusts exactly one root — the cert the server is
/// expected to present. Real verification; no `danger` shortcuts anywhere.
fn tls_connector(root: CertificateDer<'static>) -> TlsConnector {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root).unwrap();
    TlsConnector::from(std::sync::Arc::new(
        rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth(),
    ))
}

/// An SMTP session speaking over a negotiated TLS transport.
pub struct TlsSmtp {
    reader: BufReader<tokio_rustls::client::TlsStream<TcpStream>>,
}

impl TlsSmtp {
    pub async fn send(&mut self, line: &str) {
        self.reader
            .get_mut()
            .write_all(format!("{line}\r\n").as_bytes())
            .await
            .unwrap();
    }

    /// Read a single reply line.
    pub async fn reply_raw(&mut self) -> String {
        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();
        line.trim_end().to_string()
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
        let resp = self.reply_raw().await;
        assert!(resp.starts_with("354"), "DATA refused: {resp}");
        self.send(body).await;
        self.send(".").await;
        self.reply_raw().await
    }
}

/// A raw server-sent-events reader over a persistent connection: the feed's
/// frames as `(event, data)` pairs, read line by line off real HTTP —
/// tolerant of the keep-alive comment lines axum interleaves.
pub struct SseReader {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    // Held for the reader's lifetime: dropping the write half would
    // half-close the connection and cancel the streaming response.
    _write: tokio::net::tcp::OwnedWriteHalf,
}

impl SseReader {
    /// GET the path and skip the response head, leaving the event stream.
    pub async fn open(addr: std::net::SocketAddr, path: &str) -> SseReader {
        let stream = TcpStream::connect(addr).await.unwrap();
        let (r, mut w) = stream.into_split();
        w.write_all(
            format!("GET {path} HTTP/1.1\r\nHost: t\r\nAccept: text/event-stream\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
        let mut reader = BufReader::new(r);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
        }
        SseReader { reader, _write: w }
    }

    /// The next event frame: (event name, data). Wrapped in a timeout so a
    /// missing push fails the test with a message instead of hanging it.
    pub async fn next_frame(&mut self) -> (String, String) {
        let deadline = std::time::Duration::from_secs(5);
        let mut event = "message".to_string();
        let mut data = String::new();
        loop {
            let mut line = String::new();
            let read = tokio::time::timeout(deadline, self.reader.read_line(&mut line))
                .await
                .expect("timed out waiting for a feed frame")
                .expect("feed connection closed unexpectedly");
            assert!(read > 0, "feed connection closed before a frame arrived");
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                if data.is_empty() {
                    continue; // a keep-alive comment frame: keep reading
                }
                return (event, data);
            }
            if let Some(rest) = line.strip_prefix("event:") {
                event = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(rest.trim_start());
            }
            // anything else (comments, field names we do not use) is ignored
        }
    }
}

/// The plaintext half of a STARTTLS session: greeting read, EHLO sent (with
/// the STARTTLS extension advertised), STARTTLS written in the same segment
/// as `pipelined` (which the server must then discard, RFC 3207 §4.2), the
/// "220 Ready" reply read — and the raw socket returned for the handshake.
async fn plaintext_starttls_phase(
    addr: std::net::SocketAddr,
    pipelined: &[&str],
) -> (TcpStream, String, String) {
    let stream = TcpStream::connect(addr).await.unwrap();
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("220"), "bad greeting: {line}");

    w.write_all(b"EHLO swarmail-tls-test\r\n").await.unwrap();
    let mut ehlo = Vec::new();
    loop {
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        let cont = line.starts_with("250-");
        ehlo.push(line.trim_end().to_string());
        if !cont {
            break;
        }
    }
    let ehlo = ehlo.join("\n");
    assert!(ehlo.contains("250-STARTTLS"), "no STARTTLS offer: {ehlo}");

    let mut burst = String::from("STARTTLS\r\n");
    for extra in pipelined {
        burst.push_str(extra);
        burst.push_str("\r\n");
    }
    w.write_all(burst.as_bytes()).await.unwrap();

    line.clear();
    reader.read_line(&mut line).await.unwrap();
    let ready = line.trim_end().to_string();
    assert!(ready.starts_with("220"), "STARTTLS refused: {ready}");

    (reader.into_inner().reunite(w).unwrap(), ehlo, ready)
}

/// Complete a STARTTLS upgrade with a real rustls client: the handshake runs
/// against `root` as the only trusted cert, under `server_name`.
pub async fn starttls(
    addr: std::net::SocketAddr,
    server_name: &str,
    root: CertificateDer<'static>,
) -> TlsSmtp {
    starttls_discarding_pipelined(addr, server_name, root, &[]).await
}

/// [`starttls`], but with extra commands riding in the same TCP segment as
/// the STARTTLS command — a client that pipelines plaintext behind it. The
/// server must throw those away, not act on them after the upgrade.
pub async fn starttls_discarding_pipelined(
    addr: std::net::SocketAddr,
    server_name: &str,
    root: CertificateDer<'static>,
    pipelined: &[&str],
) -> TlsSmtp {
    let (tcp, _, _) = plaintext_starttls_phase(addr, pipelined).await;
    let tls = tls_connector(root)
        .connect(ServerName::try_from(server_name.to_string()).unwrap(), tcp)
        .await
        .expect("STARTTLS handshake");
    TlsSmtp {
        reader: BufReader::new(tls),
    }
}

/// The client must refuse the handshake — wrong name or untrusted root —
/// and the server then has no choice but to drop the session.
pub async fn assert_starttls_handshake_fails(
    addr: std::net::SocketAddr,
    server_name: &str,
    root: CertificateDer<'static>,
) {
    let (tcp, _, _) = plaintext_starttls_phase(addr, &[]).await;
    let res = tls_connector(root)
        .connect(ServerName::try_from(server_name.to_string()).unwrap(), tcp)
        .await;
    assert!(res.is_err(), "handshake unexpectedly succeeded");
}
