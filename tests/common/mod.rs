//! Shared end-to-end helpers: real servers, real protocol, no mocks.
#![allow(dead_code)] // compiled per test binary; not every binary uses every helper

use std::time::Duration;

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

    /// Read the connection to EOF and return whatever came since the last
    /// reply. How a server shutdown shows up on a session: the socket
    /// closes with no goodbye — an abort is no QUIT, so no update state.
    /// The per-read deadline turns a session that never ends into a failed
    /// test instead of a hung one.
    pub async fn read_to_eof(&mut self) -> String {
        let mut all = String::new();
        loop {
            let mut line = String::new();
            let read =
                tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
                    .await
                    .expect("the session never reached EOF");
            match read {
                Ok(0) | Err(_) => return all,
                Ok(_) => all.push_str(&line),
            }
        }
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

    /// Read the connection to EOF and return whatever came since the last
    /// reply. How a server shutdown shows up on a session: the socket
    /// closes with no goodbye. The per-read deadline turns a session that
    /// never ends into a failed test instead of a hung one.
    pub async fn read_to_eof(&mut self) -> String {
        let mut all = String::new();
        loop {
            let mut line = String::new();
            let read =
                tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
                    .await
                    .expect("the session never reached EOF");
            match read {
                Ok(0) | Err(_) => return all,
                Ok(_) => all.push_str(&line),
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

    /// Close the write half: the server sees a clean EOF (the peer vanished
    /// mid-conversation, e.g. mid-DATA) while this side can still read.
    pub async fn shutdown_write(&mut self) {
        self.write.shutdown().await.unwrap();
    }

    /// Write raw bytes with no line-ending massage — for payloads whose
    /// terminator deliberately differs from CRLF (RFC 5321 §4.5.2 edges).
    pub async fn send_bytes(&mut self, bytes: &[u8]) {
        self.write.write_all(bytes).await.unwrap();
    }

    /// Read the server's side to EOF (after `shutdown_write`), if it closes.
    pub async fn read_to_end(&mut self) -> Vec<u8> {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        let _ = self.reader.read_to_end(&mut buf).await;
        buf
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

/// Read one metric's value out of a `/metrics` body (Prometheus text
/// format). `None` when the row is absent — the caller decides whether that
/// is a red test or a poll that must keep waiting.
pub fn metric_value(text: &str, name: &str) -> Option<u64> {
    metric_row_value(text, name)
}

/// Read one metric row's value, matched on the row's whole first token:
/// `swarmail_awaits_active` matches only the unlabeled total and
/// `swarmail_awaits_active{inbox="x"}` only that label's sample — a label
/// can never be confused with its family's aggregate row.
pub fn metric_row_value(text: &str, row: &str) -> Option<u64> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next() == Some(row) {
                parts.next().and_then(|value| value.parse().ok())
            } else {
                None
            }
        })
}

/// One scrape of `name` off `/metrics`, over real HTTP — the reading leg of
/// the gauge pair (issue #51).
pub async fn metric_now(addr: std::net::SocketAddr, name: &str) -> u64 {
    let (status, text) = http_get_text(addr, "/metrics").await;
    assert_eq!(status, 200, "metrics must answer while reading {name}");
    metric_value(&text, name).unwrap_or_else(|| panic!("metrics missing {name}:\n{text}"))
}

/// Poll `/metrics` (deadline-bounded) until `name` has risen by `delta`
/// over `baseline`, then return the value. Capture the baseline with
/// [`metric_now`] BEFORE the trigger exists (before the request whose park
/// is awaited is spawned): a baseline taken afterwards can already observe
/// the awaited state, and a delta over an already-satisfied state can
/// never fire — the sleep's race in a nicer dress.
///
/// This shape is sound **only for monotonic counters** (`*_total`): a
/// counter never falls, so only new activity can satisfy the delta. A
/// *gauge* can fall (parallel tests' units resolve while the poll runs),
/// so a delta over a baseline is a coin flip on one — gauge polls go
/// through [`wait_for_metric_row`] instead, whose labeled row is
/// per-test-unique (decision 0004) and cannot be raised by another test.
pub async fn wait_for_metric_from(
    addr: std::net::SocketAddr,
    name: &str,
    baseline: u64,
    delta: u64,
) -> u64 {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let value = metric_now(addr, name).await;
        if value >= baseline + delta {
            return value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{name} never rose by {delta} over {baseline} (last value {value})"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// [`wait_for_metric_from`] with the baseline taken at the first
/// observation — the right shape for monotonic counters (`*_total`) whose
/// trigger is still ahead of the poll: the baseline cannot already include
/// the awaited event.
pub async fn wait_for_metric(addr: std::net::SocketAddr, name: &str, delta: u64) -> u64 {
    wait_for_metric_from(addr, name, metric_now(addr, name).await, delta).await
}

/// Poll `/metrics` (deadline-bounded) until the exact row `row` —
/// `family{inbox="…"}` — reaches `wanted`, then return it. The labeled row
/// is the sound gauge poll: the label names the caller's own unit (the
/// per-test inbox, decision 0004), so a parallel test's parked wait or
/// mid-commit delivery can never satisfy it, and the row's very existence
/// is the unit's liveness — a resolved unit renders no row at all.
pub async fn wait_for_metric_row(addr: std::net::SocketAddr, row: &str, wanted: u64) -> u64 {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let (status, text) = http_get_text(addr, "/metrics").await;
        assert_eq!(status, 200, "metrics must answer while polling {row}");
        if let Some(value) = metric_row_value(&text, row)
            && value >= wanted
        {
            return value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{row} never reached {wanted}; last /metrics body:\n{text}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The body-honest GET: `(status, body as raw octets)`. `http_get_text`'s
/// lossy read is exactly what masked the raw-bytes defect — never use it on
/// a body that must round-trip. Stringification stays confined to the ASCII
/// status-line/head; see `http_head_text`.
pub async fn http_bytes(addr: std::net::SocketAddr, path: &str) -> (u16, Vec<u8>) {
    let (status, _, body) = http_get_parts(addr, path).await;
    (status, body)
}

/// The ASCII head of a GET — status line plus headers, never the body.
pub async fn http_head_text(addr: std::net::SocketAddr, path: &str) -> (u16, String) {
    let (status, head, _) = http_get_parts(addr, path).await;
    (status, head)
}

/// One raw GET, split byte-exactly on the first `\r\n\r\n` (bodies like
/// `/raw` carry `\r\n\r\n` of their own): (status, head-as-text, body octets).
async fn http_get_parts(addr: std::net::SocketAddr, path: &str) -> (u16, String, Vec<u8>) {
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
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no header/body boundary");
    // The head is ASCII by construction; the body is octets and stays octets.
    let head = String::from_utf8_lossy(&buf[..split]).to_string();
    let status: u16 = head
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, head, buf[split + 4..].to_vec())
}

/// The raw-HTTP text client: `(status, body as String)` for any method.
/// `http_json` parses on top of this; a caller that *expects* a non-JSON
/// body (axum's text rejections, a no-content 202) comes here instead —
/// reading a text response as JSON is exactly the dishonesty issue #50
/// named, and so is silencing it back into `{}`.
pub async fn http_send_text(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> (u16, String) {
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
    // split_once, not split: a body may itself contain \r\n\r\n.
    let body = s
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("")
        .to_string();
    (status, body)
}

/// Tiny raw-HTTP client — enough for Swarmail's own JSON API, no client dep.
///
/// A non-JSON body is a loud failure, never a silent `{}`: the old
/// `unwrap_or(json!({}))` could turn a 404's HTML error page — or a UI
/// route answering an accidental wrong path — into an empty object, and an
/// assertion against the empty object passes vacuously (issue #50). A
/// caller that expects JSON gets JSON or a red test that names the method,
/// the path, the status and the body; a caller that expects anything else
/// has `http_send_text`. A genuinely empty body on a no-content status
/// (204/304) stays absence — garbage and absence are different failures
/// and only the first is dishonest (decision in the framing, R7).
pub async fn http_json(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> (u16, serde_json::Value) {
    let (status, body) = http_send_text(addr, method, path, body).await;
    match serde_json::from_str(&body) {
        Ok(parsed) => (status, parsed),
        Err(_) if body.is_empty() && (status == 204 || status == 304) => {
            (status, serde_json::Value::Null)
        }
        Err(err) => {
            panic!("http_json {method} {path}: status {status} body is not JSON ({err}): {body:?}")
        }
    }
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

    /// Read the connection to EOF and return whatever came since the last
    /// reply. How a server shutdown shows up on a session: the socket
    /// closes with no goodbye. The per-read deadline turns a session that
    /// never ends into a failed test instead of a hung one.
    pub async fn read_to_eof(&mut self) -> String {
        let mut all = String::new();
        loop {
            let mut line = String::new();
            let read =
                tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
                    .await
                    .expect("the session never reached EOF");
            match read {
                Ok(0) | Err(_) => return all,
                Ok(_) => all.push_str(&line),
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

    /// Read the connection to EOF, returning everything after the last
    /// frame. How a server shutdown shows up on an open feed: the response
    /// body terminates and the socket closes. The per-read deadline turns a
    /// feed that never ends into a failed test instead of a hung one.
    pub async fn read_until_eof(&mut self) -> String {
        let mut all = String::new();
        loop {
            let mut line = String::new();
            let read =
                tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
                    .await
                    .expect("the feed never reached EOF");
            match read {
                Ok(0) | Err(_) => return all,
                Ok(_) => all.push_str(&line),
            }
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

/// The raw socket of a session that has been offered TLS and never takes
/// it: greeting read, EHLO answered, `STARTTLS` sent, `220 Ready` read —
/// and then nothing. The handshake is stalled by hand; the caller holds
/// the socket open to wait out the server's handshake deadline.
pub async fn stalled_starttls(addr: std::net::SocketAddr) -> TcpStream {
    let (tcp, _, _) = plaintext_starttls_phase(addr, &[]).await;
    tcp
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

/// A live second connection holding SQLite's writer lock (`BEGIN
/// IMMEDIATE`) against the server's WAL data file. Any commit the server
/// attempts while this is held blocks on the WAL write lock, burns its
/// busy timeout and fails with `SQLITE_BUSY` — the deterministic refusal
/// of decision 0012 §3, no fault-injection needed: the conflict is real,
/// at the real layer, through a real second connection. Dropping the value
/// rolls the empty transaction back and frees the lock, so later legs see
/// a clean database.
pub struct WriteLock {
    _conn: rusqlite::Connection,
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        let _ = self._conn.execute_batch("ROLLBACK;");
    }
}

pub fn hold_write_lock(db: &std::path::Path) -> WriteLock {
    let conn = rusqlite::Connection::open(db).expect("second connection opens the data file");
    conn.busy_timeout(std::time::Duration::from_secs(10))
        .expect("busy timeout set");
    conn.execute_batch("BEGIN IMMEDIATE;")
        .expect("second connection takes the writer lock");
    WriteLock { _conn: conn }
}

/// A real rejecting trigger on the real data file — the failure injection of
/// issue #31's findings 2 and 3, at the real layer and through a real
/// SQLite constraint: every `INSERT` (kind `"insert"`) or `DELETE` (kind
/// `"delete"`) the server's own connection runs is `RAISE(ABORT, 'refused')`d
/// while the guard lives. [`refusing_trigger_except`] is the selective form:
/// one refused id, every other delete allowed (decision 0015's mid-loop
/// instrument). No fault-injection framework, no mock: the refusal
/// is the database's own, and it hits every writer — the store's commits
/// included — whatever connection issues it. [`RefusingTrigger::remove`]
/// drops the trigger early so a later leg can commit normally; the guard's
/// `Drop` drops it too, so an assert-panic never poisons the file.
pub struct RefusingTrigger {
    conn: Option<rusqlite::Connection>,
    name: &'static str,
}

impl RefusingTrigger {
    fn take(&mut self) {
        if let Some(conn) = self.conn.take() {
            conn.execute_batch(&format!("DROP TRIGGER {}", self.name))
                .expect("trigger dropped");
        }
    }

    /// Drop the trigger now: later legs must commit again (the retry leg).
    pub fn remove(mut self) {
        self.take();
    }
}

impl Drop for RefusingTrigger {
    fn drop(&mut self) {
        self.take();
    }
}

pub fn refusing_trigger(db: &std::path::Path, kind: &str) -> RefusingTrigger {
    let (name, sql): (&'static str, &'static str) = match kind {
        "insert" => (
            "refuse_insert",
            "CREATE TRIGGER refuse_insert BEFORE INSERT ON emails \
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        ),
        "delete" => (
            "refuse_delete",
            "CREATE TRIGGER refuse_delete BEFORE DELETE ON emails \
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        ),
        other => panic!("unknown refusing-trigger kind: {other}"),
    };
    install_refusing_trigger(db, name, sql)
}

/// A *selective* refusing delete trigger (decision 0015's instrument): it
/// `RAISE(ABORT, 'refused')`s exactly the one named id and lets every other
/// delete through. This is the only way a wire leg can reach a `QUIT`
/// failure *mid-loop* — an all-refusing trigger fails on the first delete,
/// so the committed prefix it would prove stays empty. Same guard, same
/// `Drop` cleanup as [`refusing_trigger`].
pub fn refusing_trigger_except(db: &std::path::Path, id: &str) -> RefusingTrigger {
    // The id is ours (a UUIDv7 the server minted), but the quote-doubling
    // keeps the injection honest for any payload.
    let sql = format!(
        "CREATE TRIGGER refuse_delete_except BEFORE DELETE ON emails \
         WHEN OLD.id = '{}' BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        id.replace('\'', "''")
    );
    install_refusing_trigger(db, "refuse_delete_except", &sql)
}

fn install_refusing_trigger(
    db: &std::path::Path,
    name: &'static str,
    sql: &str,
) -> RefusingTrigger {
    let conn = rusqlite::Connection::open(db).expect("second connection opens the data file");
    conn.busy_timeout(std::time::Duration::from_secs(10))
        .expect("busy timeout set");
    conn.execute_batch(sql)
        .expect("trigger created on the data file");
    RefusingTrigger {
        conn: Some(conn),
        name,
    }
}
