//! Concurrent accept-all SMTP server.
//!
//! One tokio task per connection; the protocol state machine is intentionally
//! simple and the store insert is synchronous — accepting a message means it is
//! stored. There is no async gap where mail can be lost.
//!
//! STARTTLS upgrades the session transport in place: the plaintext listener
//! stays byte-identical when TLS is not configured, and with a certificate
//! configured every command after the handshake is TLS-only — the session
//! restarts per RFC 3207 §4.2, with everything the client said before the
//! handshake discarded. The handshake itself is bounded: a client that
//! stalls mid-upgrade is dropped after `SmtpConfig::tls_handshake_timeout`
//! instead of pinning its session task, while the plaintext idle posture
//! (no read timeout at all) is untouched.

use crate::chaos::{Chaos, ChaosEvent};
use crate::extract::{extract_codes, extract_links};
use crate::model::{Email, EmailAddress};
use crate::store::Store;
use chrono::Utc;
use mail_parser::{Addr, Address, HeaderValue, MessageParser};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tracing::debug;

/// Hard cap on a single message (50 MiB).
const MAX_MESSAGE_SIZE: usize = 50 * 1024 * 1024;

/// How long a STARTTLS handshake may take by default. A real rustls
/// handshake over loopback is milliseconds; ten seconds already forgives a
/// very slow peer while a stalled one cannot hold a session task forever.
const DEFAULT_TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// Hostname announced in the banner.
    pub hostname: String,
    /// Accept any AUTH credentials; the username names the target inbox.
    pub accept_any_auth: bool,
    /// Largest accepted DATA payload in bytes; larger gets 552.
    pub max_message_size: usize,
    /// How long a STARTTLS handshake may take before the session is
    /// dropped. A client that connects, greets and then goes silent
    /// mid-upgrade must not be able to tie the session task up forever;
    /// the deadline bounds exactly that wait and nothing else — the
    /// pre-TLS plaintext idle posture stays unbounded, as ever.
    pub tls_handshake_timeout: Duration,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            hostname: "swarmail.local".to_string(),
            accept_any_auth: true,
            max_message_size: MAX_MESSAGE_SIZE,
            tls_handshake_timeout: DEFAULT_TLS_HANDSHAKE_TIMEOUT,
        }
    }
}

static SESSIONS: AtomicU64 = AtomicU64::new(0);

pub async fn serve(
    listener: TcpListener,
    store: Arc<Store>,
    chaos: Arc<Chaos>,
    cfg: SmtpConfig,
    tls: Option<Arc<rustls::ServerConfig>>,
    shutdown: impl std::future::Future<Output = ()> + Send,
) -> io::Result<()> {
    let mut shutdown = std::pin::pin!(shutdown);
    loop {
        let (stream, peer) = tokio::select! {
            res = listener.accept() => res?,
            _ = &mut shutdown => return Ok(()),
        };
        let session = SESSIONS.fetch_add(1, Ordering::Relaxed);
        debug!(%peer, session, "smtp connection accepted");
        let store = store.clone();
        let chaos = chaos.clone();
        let cfg = cfg.clone();
        let tls = tls.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, store, chaos, cfg, tls).await {
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
    // Pushed through the transport now: it puts the STARTTLS 220 on the wire
    // before the handshake begins, and is a no-op for plain TCP.
    stream.flush().await
}

/// The session transport: plaintext TCP, or TLS once `STARTTLS` has been
/// negotiated. `Plain(None)` is a socket already taken for a handshake —
/// reads on it are EOF and the session is over.
enum Conn {
    Plain(Option<TcpStream>),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for Conn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Conn::Plain(Some(s)) => Pin::new(s).poll_read(cx, buf),
            Conn::Plain(None) => Poll::Ready(Ok(())),
            Conn::Tls(t) => Pin::new(&mut **t).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Conn {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Conn::Plain(Some(s)) => Pin::new(s).poll_write(cx, buf),
            Conn::Plain(None) => Poll::Ready(Err(io::Error::other("socket taken for TLS"))),
            Conn::Tls(t) => Pin::new(&mut **t).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Conn::Plain(Some(s)) => Pin::new(s).poll_flush(cx),
            Conn::Plain(None) => Poll::Ready(Ok(())),
            Conn::Tls(t) => Pin::new(&mut **t).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Conn::Plain(Some(s)) => Pin::new(s).poll_shutdown(cx),
            Conn::Plain(None) => Poll::Ready(Ok(())),
            Conn::Tls(t) => Pin::new(&mut **t).poll_shutdown(cx),
        }
    }
}

impl Conn {
    /// The plaintext socket, taken out for a STARTTLS handshake. The session
    /// asks exactly once, while the offer is still open; anything else is a
    /// state bug and refuses.
    fn take_plain(&mut self) -> io::Result<TcpStream> {
        match self {
            Conn::Plain(slot) => slot
                .take()
                .ok_or_else(|| io::Error::other("plaintext socket already taken")),
            Conn::Tls(_) => Err(io::Error::other("session is already TLS")),
        }
    }
}

/// Extract the bare address from "MAIL FROM:<a@b> SIZE=1" style arguments.
fn arg_address(arg: &str) -> String {
    match (arg.find('<'), arg.find('>')) {
        (Some(a), Some(b)) if b > a => arg[a + 1..b].to_string(),
        _ => arg.split(':').nth(1).unwrap_or(arg).trim().to_string(),
    }
}

/// What the session does when the client says `STARTTLS`.
enum TlsOffer {
    /// No TLS configured: the plaintext listener of old, byte for byte.
    None,
    /// Plaintext session with TLS configured: the next `STARTTLS` upgrades
    /// it, carrying the acceptor that performs the handshake.
    Offered(TlsAcceptor),
    /// The session is already TLS: a further `STARTTLS` is refused.
    Active,
}

async fn handle_conn(
    stream: TcpStream,
    store: Arc<Store>,
    chaos: Arc<Chaos>,
    cfg: SmtpConfig,
    tls: Option<Arc<rustls::ServerConfig>>,
) -> io::Result<()> {
    // Chaos: connect event can drop the session before the banner.
    let mut conn = Conn::Plain(Some(stream));
    if let Some(rule) = chaos.check(ChaosEvent::Connect) {
        tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
        if let Some(err) = &rule.error {
            write_line(&mut conn, err).await.ok();
        }
        return Ok(());
    }

    let mut reader = BufReader::new(conn);
    let offer = match tls {
        None => TlsOffer::None,
        Some(cfg) => TlsOffer::Offered(TlsAcceptor::from(cfg)),
    };
    let result = session(&mut reader, &store, &chaos, &cfg, offer).await;
    // Graceful close whatever happened: TLS sends close_notify, plain sends
    // FIN, and a socket already taken for a failed handshake is done.
    let _ = reader.into_inner().shutdown().await;
    result
}

/// One command loop over the session transport. `STARTTLS` swaps the reader
/// for a TLS one in place — the state below restarts there per RFC 3207.
async fn session(
    reader: &mut BufReader<Conn>,
    store: &Arc<Store>,
    chaos: &Arc<Chaos>,
    cfg: &SmtpConfig,
    mut offer: TlsOffer,
) -> io::Result<()> {
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
                resp.push_str(&cfg.max_message_size.to_string());
                // Advertised only while a plaintext session can still
                // upgrade; on an established TLS session the extension is
                // forbidden (RFC 3207 §4.2).
                if matches!(&offer, TlsOffer::Offered(_)) {
                    resp.push_str("\r\n250-STARTTLS");
                }
                if cfg.accept_any_auth {
                    resp.push_str("\r\n250-AUTH PLAIN LOGIN");
                }
                resp.push_str("\r\n250 OK");
                write_line(reader.get_mut(), &resp).await?;
            }
            "STARTTLS" => {
                offer = match offer {
                    TlsOffer::None => {
                        write_line(reader.get_mut(), "454 4.7.0 TLS not available").await?;
                        TlsOffer::None
                    }
                    TlsOffer::Active => {
                        write_line(reader.get_mut(), "554 5.5.1 TLS already active").await?;
                        TlsOffer::Active
                    }
                    TlsOffer::Offered(acc) if !arg.is_empty() => {
                        write_line(
                            reader.get_mut(),
                            "501 5.5.4 Syntax error (no parameters allowed)",
                        )
                        .await?;
                        TlsOffer::Offered(acc)
                    }
                    TlsOffer::Offered(acc) => {
                        write_line(reader.get_mut(), "220 2.0.0 Ready to start TLS").await?;
                        // The socket goes out for the handshake; anything
                        // pipelined behind STARTTLS sits in the plaintext
                        // buffer and is dropped with the old transport.
                        let raw = reader.get_mut().take_plain()?;
                        // The upgrade is bounded: a client that stalls
                        // after the 220 — greeted, then silent — must not
                        // tie this session task up forever. On elapse the
                        // `Accept` future is dropped, taking the socket
                        // with it: the peer sees the connection close and
                        // the listener keeps serving.
                        let upgraded =
                            match tokio::time::timeout(cfg.tls_handshake_timeout, acc.accept(raw))
                                .await
                            {
                                Ok(upgraded) => upgraded?,
                                Err(_) => {
                                    return Err(io::Error::new(
                                        io::ErrorKind::TimedOut,
                                        "STARTTLS handshake deadline elapsed",
                                    ));
                                }
                            };
                        *reader = BufReader::new(Conn::Tls(Box::new(upgraded)));
                        // RFC 3207 §4.2: everything the client said before
                        // the handshake is forgotten — fresh envelope, fresh
                        // auth, and the buffer went with it.
                        mail_from = None;
                        rcpts.clear();
                        inbox = "default".to_string();
                        TlsOffer::Active
                    }
                };
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
                    if raw.len() + payload.len() > cfg.max_message_size {
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

    // Thread headers are extracted once, here at ingest (decision 0005), and
    // stored as first-class fields: the thread view must not re-parse the raw
    // RFC 5322 headers on every render or push.
    let references = parsed
        .as_ref()
        .map(|m| header_ids(m.references()))
        .unwrap_or_default();
    let in_reply_to = parsed
        .as_ref()
        .map(|m| header_ids(m.in_reply_to()))
        .unwrap_or_default()
        .into_iter()
        .next();
    let message_id = parsed
        .as_ref()
        .and_then(|m| m.message_id())
        .and_then(|raw| crate::threads::parse_ids(raw).into_iter().next());

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
        message_id,
        in_reply_to,
        references,
        raw: raw.to_vec(),
    }
}

/// The ids a `References`/`In-Reply-To` header carries, in header order (the
/// parser may hand back a text list for folded headers). An absent header is
/// an empty list, never an error.
fn header_ids(value: &HeaderValue<'_>) -> Vec<String> {
    value
        .as_text_list()
        .map(|list| {
            list.iter()
                .flat_map(|text| crate::threads::parse_ids(text))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transport that has already handed out its socket reads as EOF and
    /// refuses writes: there is nothing left to talk to once the handshake
    /// owns the connection.
    #[tokio::test]
    async fn a_taken_socket_reads_eof_and_refuses_writes() {
        let mut conn = Conn::Plain(None);
        let waker = std::task::Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut out = [0u8; 8];
        let mut buf = ReadBuf::new(&mut out);
        let eof = Pin::new(&mut conn).poll_read(&mut cx, &mut buf);
        assert!(
            matches!(&eof, Poll::Ready(Ok(()))),
            "expected EOF, got {eof:?}"
        );
        let refused = Pin::new(&mut conn).poll_write(&mut cx, b"NOOP");
        assert!(
            matches!(&refused, Poll::Ready(Err(e)) if e.to_string().contains("taken")),
            "expected a write refusal, got {refused:?}"
        );

        conn.flush().await.unwrap();
        conn.shutdown().await.unwrap();
    }

    /// The plaintext socket is handed out exactly once, and a TLS transport
    /// has nothing plaintext to hand out at all.
    #[tokio::test]
    async fn take_plain_is_refused_once_taken_or_over_tls() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut conn = Conn::Plain(Some(TcpStream::connect(addr).await.unwrap()));
        assert!(conn.take_plain().is_ok());
        assert!(conn.take_plain().is_err());

        let mut tls = Conn::Tls(Box::new(negotiated_tls_stream().await));
        assert!(tls.take_plain().is_err());
    }

    /// A real negotiated TLS transport over loopback — the server side of
    /// the pair, exactly what `Conn` wraps after a STARTTLS upgrade.
    async fn negotiated_tls_stream() -> TlsStream<TcpStream> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let pair = crate::tls::TlsConfig::generate_self_signed("localhost").unwrap();
        let cfg = Arc::new(pair.server_config().unwrap());
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            TlsAcceptor::from(cfg).accept(tcp).await.unwrap()
        });

        // The client trusts only the minted cert — a real verification.
        let mut roots = tokio_rustls::rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut pair.cert_pem.as_bytes()) {
            roots.add(cert.unwrap()).unwrap();
        }
        let client_cfg = Arc::new(
            tokio_rustls::rustls::ClientConfig::builder_with_provider(Arc::new(
                tokio_rustls::rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth(),
        );
        let connector = tokio_rustls::TlsConnector::from(client_cfg);
        tokio::spawn(async move {
            connector
                .connect(
                    tokio_rustls::rustls::pki_types::ServerName::try_from("localhost".to_string())
                        .unwrap(),
                    TcpStream::connect(addr).await.unwrap(),
                )
                .await
                .unwrap();
        });

        server.await.unwrap()
    }
}
