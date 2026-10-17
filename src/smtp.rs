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
//!
//! AUTH PLAIN and AUTH LOGIN assign the session's inbox only on success: a
//! credential that names nothing — undecodable base64, a PLAIN that is not
//! exactly three fields, an empty decoded username, a `*` cancel, a
//! non-UTF-8 one — is refused `501 5.5.2` and changes nothing (decision 0028
//! §2, amending 0004's accept-any posture; `535` is reserved for the day
//! credentials are checked), and one success per TLS epoch is all a session
//! gets (decision 0034 §5). The transaction state machine is RFC 5321's: the
//! envelope is consumed at the DATA terminator, reset by RSET, HELO, EHLO
//! and STARTTLS, and left untouched by every refused MAIL or RCPT;
//! out-of-sequence commands are refused `503 5.5.1` (§4.1.4); and DATA ends
//! on CRLF `.` CRLF only (§2.3.8, decision 0034 §1) — a bare-LF `.` line,
//! or a `.` CRLF line behind a bare LF, is payload.

use crate::chaos::{Chaos, ChaosEvent};
use crate::extract::{extract_codes, extract_links};
use crate::model::{Email, EmailAddress};
use crate::store::Store;
use chrono::Utc;
use mail_parser::{Addr, Address, HeaderValue, MessageParser, PartType};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tracing::debug;

/// Hard cap on a single message (50 MiB).
const MAX_MESSAGE_SIZE: usize = 50 * 1024 * 1024;

/// Hard cap on one protocol line: verbs, arguments and AUTH continuations.
/// POP3 draws the same line for its own loop (`pop3.rs` `MAX_COMMAND_LINE`):
/// a command line is a protocol unit, not a message, and a longer one is
/// drained and refused — never buffered — so a hostile peer cannot grow
/// server memory with an unterminated line.
const MAX_COMMAND_LINE: usize = 4096;

/// How long a STARTTLS handshake may take by default. A real rustls
/// handshake over loopback is milliseconds; ten seconds already forgives a
/// very slow peer while a stalled one cannot hold a session task forever.
const DEFAULT_TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// The pause between accept retries (shared with the POP3 twin). Fixed,
/// not exponential: a transient
/// listener failure (the EMFILE storm of issue #36) is measured in
/// milliseconds of pressure, and a full second already forgives it — an
/// exponential machine is a knob without a demonstrated rate.
pub(crate) const ACCEPT_BACKOFF: Duration = Duration::from_secs(1);

/// The single-I/O idle bound (RFC 5321 §4.5.3.2's receiver timeout): no one
/// peer read and no one reply write may wait longer than this for progress.
/// Compiled default, overridable in-process (decision 0022 §5: no knob).
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Concurrent sessions per listener (decision 0027): three listeners at the
/// cap plus the listeners and stdio stay under the common 1024 soft-fd
/// floor — the repro axis issue #36 itself names.
const DEFAULT_MAX_SESSIONS: usize = 256;

/// The recipient ceiling per transaction (decision 0027): the 101st `RCPT`
/// is refused `452 4.5.3` before it is appended. A plain constant, not a
/// knob (decision 0022 §5) — no deployment ever grew a legitimate envelope
/// past it.
const MAX_RCPTS: usize = 100;

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
    /// The single-I/O idle bound (decision 0027): bounds one peer read or
    /// one reply write, never the commit. Expiry says goodbye —
    /// `421 4.4.2 …` between commands, silence mid-DATA (nothing stored).
    pub idle_timeout: Duration,
    /// Concurrent sessions this listener serves (decision 0027). Beyond it
    /// the refusal is wire-honest (`421 4.7.0 …`) and retryable, written
    /// before any session state exists.
    pub max_sessions: usize,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            hostname: "swarmail.local".to_string(),
            accept_any_auth: true,
            max_message_size: MAX_MESSAGE_SIZE,
            tls_handshake_timeout: DEFAULT_TLS_HANDSHAKE_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            max_sessions: DEFAULT_MAX_SESSIONS,
        }
    }
}

static SESSIONS: AtomicU64 = AtomicU64::new(0);

/// Test-only: sessions reaped by the accept loop's harvest. Its only reader
/// is the inline probe that installs it (decision 0020 rule 4) — the probe
/// drives the real `serve`, and the counter must track completed sessions
/// across successive connection batches or the deadline expires red.
#[cfg(test)]
static REAPED: AtomicUsize = AtomicUsize::new(0);

// Test-only: counted, one-shot accept errors (decision 0020 rule 4, the
// `REAPED` precedent). The accept loop's error arm is unreachable behind a
// well-behaved listener — a `TcpListener` cannot be made to fail on demand
// in-process — so the installing test installs the misbehavior here.
// Thread-local because the loop is polled on the injecting test's own
// current-thread runtime; a process-global would leak across the suite.
#[cfg(test)]
thread_local! {
    static INJECTED_ACCEPT_ERRORS: core::cell::Cell<usize> =
        const { core::cell::Cell::new(0) };
}

/// Install `count` failing accepts for the loop driven by this test. Called
/// before the loop is first polled, the arm is branch-stable: the first
/// poll consumes an error instead of reaching the listener.
#[cfg(test)]
pub(crate) fn inject_accept_errors(count: usize) {
    INJECTED_ACCEPT_ERRORS.with(|errors| errors.set(count));
}

/// Consume one installed accept error, if any remain.
#[cfg(test)]
fn take_injected_accept_error() -> bool {
    INJECTED_ACCEPT_ERRORS.with(|errors| {
        let left = errors.get();
        if left == 0 {
            false
        } else {
            errors.set(left - 1);
            true
        }
    })
}

pub async fn serve(
    listener: TcpListener,
    store: Arc<Store>,
    chaos: Arc<Chaos>,
    cfg: SmtpConfig,
    tls: Option<Arc<rustls::ServerConfig>>,
    degraded: Arc<AtomicBool>,
    shutdown: impl std::future::Future<Output = ()> + Send,
) -> io::Result<()> {
    let mut shutdown = std::pin::pin!(shutdown);
    // Every session is owned by this loop: ready completions are reaped at
    // the top of every iteration — never parked — so a finished session
    // cannot accumulate in the set with the cumulative connection count; the
    // set is drained on the shutdown arm, so no session socket outlives the
    // serve loop. (Dropping the set would abort its children — the drain
    // makes the wait explicit, and the abort's reach is stage-precise: a
    // session that has not crossed the DATA terminator has no commit to
    // land, so the closed socket is the only goodbye it gets; a session
    // parked at the insert's await still lands its commit — `insert_async`
    // offloads the whole mutation to `spawn_blocking` (src/store.rs), and
    // an abort cannot cancel a blocking closure — so the row is stored but
    // never acknowledged, the client's retry a duplicate, never a loss
    // (0001's honest asymmetry), and the parked `_in_flight` guard releases
    // at the abort, before the orphaned commit resolves — a shutdown-only
    // observability skew on a process that is leaving anyway.
    // tests/lifecycle.rs::an_in_flight_commit_neither_holds_stop_nor_half_lands
    // asserts the wire truth.)
    let mut sessions: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
    loop {
        // Reap ready completions without blocking: `try_join_next` never
        // parks the accept path and never touches a session still running.
        while sessions.try_join_next().is_some() {
            #[cfg(test)]
            REAPED.fetch_add(1, Ordering::Relaxed);
        }
        // The accept itself, raced against the stop. The seam arm (test-only)
        // is consumed on the first poll, so the error arm below is reachable
        // and branch-stable; in production the block is just the accept.
        let accepted = tokio::select! {
            res = async {
                #[cfg(test)]
                if take_injected_accept_error() {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "injected accept error",
                    ));
                }
                listener.accept().await
            } => res,
            _ = &mut shutdown => {
                sessions.abort_all();
                while sessions.join_next().await.is_some() {}
                return Ok(());
            }
        };
        let (stream, peer) = match accepted {
            Ok(accepted) => {
                // A successful accept is the recovery: the listener answers
                // again, so the surface stops claiming degraded before the
                // next failure can.
                degraded.store(false, Ordering::Relaxed);
                accepted
            }
            Err(e) => {
                // A transient failure is not a death sentence: the loop says
                // so once per backoff step and retries forever. An accept
                // loop that dies turns one EMFILE into a dead listener for
                // the life of the process (issue #36); the degraded flag is
                // the honest surface, the backoff the pace. The stop is
                // raced against the backoff, never stacked behind it — a
                // stop arriving mid-pause resolves immediately.
                degraded.store(true, Ordering::Relaxed);
                tracing::warn!(error = %e, "smtp accept failed; retrying after the backoff");
                tokio::select! {
                    _ = &mut shutdown => {
                        sessions.abort_all();
                        while sessions.join_next().await.is_some() {}
                        return Ok(());
                    }
                    _ = tokio::time::sleep(ACCEPT_BACKOFF) => {}
                }
                continue;
            }
        };
        // Bounded sessions (decision 0027): the cap is checked against the
        // live set after the reap. A refusal is wire-honest and retryable,
        // written under the same idle deadline before any session state
        // exists, and the socket is closed without spawning.
        if sessions.len() >= cfg.max_sessions {
            let _ = bound_io(
                async {
                    let mut stream = stream;
                    write_line(
                        &mut stream,
                        "421 4.7.0 Too many connections, closing transmission channel",
                        cfg.idle_timeout,
                    )
                    .await
                },
                cfg.idle_timeout,
            )
            .await;
            debug!(%peer, "smtp refused: session cap reached");
            continue;
        }
        let session = SESSIONS.fetch_add(1, Ordering::Relaxed);
        debug!(%peer, session, "smtp connection accepted");
        let store = store.clone();
        let chaos = chaos.clone();
        let cfg = cfg.clone();
        let tls = tls.clone();
        sessions.spawn(async move {
            if let Err(e) = handle_conn(stream, store, chaos, cfg, tls).await {
                debug!(session, error = %e, "connection ended");
            }
        });
    }
}

/// One deadline around exactly one I/O operation — one peer read or one
/// reply write (decision 0027). A silent peer is an ended session, not a
/// pinned task: the wait is bounded, and the expiry is exactly a
/// `TimedOut` io error, so the session's existing `?` handling ends it the
/// same way as a peer that hung up. The deadline is armed per operation —
/// never across the commit: a session parked at its insert keeps waiting,
/// because accepting a message means it is stored (0001). Shared with the
/// POP3 sibling, which maps the same TimedOut to its own `-ERR idle
/// timeout`.
pub(crate) async fn bound_io<T>(
    io: impl std::future::Future<Output = io::Result<T>>,
    deadline: Duration,
) -> io::Result<T> {
    match tokio::time::timeout(deadline, io).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::from(io::ErrorKind::TimedOut)),
    }
}

async fn write_line<W: tokio::io::AsyncWrite + Unpin>(
    stream: &mut W,
    line: &str,
    idle: Duration,
) -> io::Result<()> {
    bound_io(
        async {
            stream.write_all(line.as_bytes()).await?;
            stream.write_all(b"\r\n").await?;
            // Pushed through the transport now: it puts the STARTTLS 220 on
            // the wire before the handshake begins, and is a no-op for
            // plain TCP.
            stream.flush().await
        },
        idle,
    )
    .await
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

/// The refusals of envelope syntax and parameters (decision 0034 §3–§4).
const MAIL_PATH_SYNTAX: &str = "501 5.5.4 Syntax: MAIL FROM:<address>";
const RCPT_PATH_SYNTAX: &str = "501 5.5.4 Syntax: RCPT TO:<address>";
const MAIL_PARAM_INVALID: &str = "501 5.5.4 Invalid MAIL parameter";
const MAIL_SIZE_TOO_BIG: &str = "552 5.3.4 Declared SIZE exceeds the message size limit";
const MAIL_PARAM_UNKNOWN: &str = "555 5.5.4 Unsupported MAIL parameter";
const RCPT_PARAM_UNKNOWN: &str = "555 5.5.4 RCPT parameters not supported";

/// One open transaction: the MAIL sender (empty for the null reverse-path)
/// and the recipients accepted so far. `None` in the session means no MAIL
/// — a sender-less envelope holding recipients cannot be represented.
struct Envelope {
    from: String,
    rcpts: Vec<String>,
}

/// Split a MAIL/RCPT argument into its bracketed path and the parameter
/// suffix: `keyword` (`FROM:` or `TO:`) case-insensitively, optional
/// whitespace, then `<path>` with no further angle bracket inside, then
/// nothing or whitespace before any parameters. Anything else — a missing
/// or wrong keyword, a bracketless or inverted path, a suffix glued to the
/// `>` — is `None`, the 501. The path itself stays opaque: local names and
/// UTF-8 are kept, no `@` or mailbox grammar is demanded.
fn envelope_path<'a>(arg: &'a str, keyword: &str) -> Option<(&'a str, &'a str)> {
    let head = arg.get(..keyword.len())?;
    if !head.eq_ignore_ascii_case(keyword) {
        return None;
    }
    let rest = arg[keyword.len()..].trim_start_matches([' ', '\t']);
    let rest = rest.strip_prefix('<')?;
    let close = rest.find('>')?;
    let (path, tail) = (&rest[..close], &rest[close + 1..]);
    let separated = tail.is_empty() || tail.starts_with([' ', '\t']);
    if path.contains('<') || !separated {
        return None;
    }
    Some((path, tail))
}

/// Validate MAIL's parameters left to right, the first error winning:
/// `SIZE=<digits>` within `cap` (an all-digit value too large to represent
/// is over any cap: 552), `BODY=7BIT|8BITMIME`, bare `SMTPUTF8` — names and
/// keywords case-insensitive. A malformed or repeated recognized parameter
/// is 501, an unknown one 555. An accepted SIZE is only a declaration: the
/// DATA loop still counts the real bytes.
fn mail_params(params: &str, cap: usize) -> Result<(), &'static str> {
    let (mut size, mut body, mut smtputf8) = (false, false, false);
    for param in params.split_whitespace() {
        let (name, value) = match param.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (param, None),
        };
        let name = name.to_ascii_uppercase();
        let seen = match name.as_str() {
            "SIZE" => &mut size,
            "BODY" => &mut body,
            "SMTPUTF8" => &mut smtputf8,
            _ => return Err(MAIL_PARAM_UNKNOWN),
        };
        if std::mem::replace(seen, true) {
            return Err(MAIL_PARAM_INVALID);
        }
        match (name.as_str(), value) {
            ("SIZE", Some(v)) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
                // All digits: a failed parse is overflow, over any cap.
                if !v.parse::<usize>().is_ok_and(|declared| declared <= cap) {
                    return Err(MAIL_SIZE_TOO_BIG);
                }
            }
            ("BODY", Some(v))
                if v.eq_ignore_ascii_case("7BIT") || v.eq_ignore_ascii_case("8BITMIME") => {}
            ("SMTPUTF8", None) => {}
            _ => return Err(MAIL_PARAM_INVALID),
        }
    }
    Ok(())
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
            write_line(&mut conn, err, cfg.idle_timeout).await.ok();
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

/// One protocol line, as the wire delivered it.
#[derive(Debug, PartialEq, Eq)]
enum Line {
    /// A whole line including its delimiter — bytes intact, no trim, no
    /// UTF-8 validation. DATA is octets (8BITMIME); nothing here re-encodes.
    Command(Vec<u8>),
    /// Over the caller's cap: the tail was drained, the stream is in sync.
    /// `crlf` is the drained line's real ending — its LF preceded by CR —
    /// because DATA's terminator is only a terminator after a CRLF-ended
    /// line (decision 0034 §1), discarded lines included. Command and AUTH
    /// callers ignore it.
    TooLong { crlf: bool },
    /// EOF before any byte, or a final line with no terminator — not a line
    /// the session can act on (the pop3 sibling's posture).
    Eof,
}

/// The next line, capped. Reads through the buffer so an over-long line
/// neither blocks nor accumulates: once the cap is passed with no end in
/// sight the tail is drained unbuffered and the line is refused — the
/// buffer never grows past the cap plus one `fill_buf` chunk. The
/// delimiter is kept and no byte is transformed. Mirrors pop3's
/// `read_line_capped`; each listener owns its reader, beside its loop.
async fn read_line_capped(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
    cap: usize,
) -> io::Result<Line> {
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            // EOF: a partial line has no terminator, so it is not a line.
            return Ok(Line::Eof);
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(i) => {
                // The delimiter is part of the line: bytes stay as sent.
                bytes.extend_from_slice(&available[..=i]);
                reader.consume(i + 1);
                if bytes.len() > cap {
                    // The whole line is buffered: its ending is read off it
                    // before it is discarded.
                    let crlf = bytes.ends_with(b"\r\n");
                    return Ok(Line::TooLong { crlf });
                }
                return Ok(Line::Command(bytes));
            }
            None => {
                bytes.extend_from_slice(available);
                let len = available.len();
                reader.consume(len);
                if bytes.len() > cap {
                    // Already over the cap and still no newline: stop
                    // buffering, drain the rest of the line, refuse it. The
                    // last buffered byte seeds the drain — a CR here and the
                    // LF as the drain's first byte is still a CRLF.
                    let crlf = drain_line(reader, bytes.last() == Some(&b'\r')).await?;
                    return Ok(Line::TooLong { crlf });
                }
            }
        }
    }
}

/// Consume the rest of an over-long line — up to and including its newline —
/// without buffering it, so the byte stream stays in sync and the next line
/// is read whole. A peer that never sends the newline just idles; its
/// connection ends whenever it goes away. Answers whether the newline was
/// preceded by CR: `prev_cr` is the byte before the drain began, carried
/// across chunks so a CR/LF split over a read boundary is still a CRLF. EOF
/// before the newline is no CRLF — none is invented.
async fn drain_line(
    reader: &mut (impl tokio::io::AsyncBufRead + Unpin),
    mut prev_cr: bool,
) -> io::Result<bool> {
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok(false);
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(i) => {
                let crlf = if i == 0 {
                    prev_cr
                } else {
                    available[i - 1] == b'\r'
                };
                reader.consume(i + 1);
                return Ok(crlf);
            }
            None => {
                prev_cr = available.last() == Some(&b'\r');
                let len = available.len();
                reader.consume(len);
            }
        }
    }
}

/// The outcome of one command-path read.
enum CmdLine {
    /// A whole line, UTF-8-validated: a command is a protocol word, not
    /// payload — DATA is where bytes live.
    Line(String),
    /// Over `MAX_COMMAND_LINE`: drained, answered `500 5.5.2 Line too long`
    /// (the SMTP sibling of POP3's `-ERR line too long`), failing only the
    /// attempt it arrived on.
    Refused,
    /// The peer is gone: no line, no reply, session over.
    Gone,
}

/// One line for the command loop and the AUTH continuations: capped like
/// POP3's, UTF-8-validated like `read_line`'s — invalid command bytes still
/// end the session, as they always did. The wait is one idle bound wide.
async fn read_command_line(reader: &mut BufReader<Conn>, idle: Duration) -> io::Result<CmdLine> {
    match bound_io(read_line_capped(&mut *reader, MAX_COMMAND_LINE), idle).await? {
        Line::Command(bytes) => Ok(CmdLine::Line(
            String::from_utf8(bytes).map_err(io::Error::other)?,
        )),
        Line::TooLong { .. } => {
            write_line(reader.get_mut(), "500 5.5.2 Line too long", idle).await?;
            Ok(CmdLine::Refused)
        }
        Line::Eof => Ok(CmdLine::Gone),
    }
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
        cfg.idle_timeout,
    )
    .await?;

    // The open transaction, if any (decision 0034 §3), and the identity a
    // successful AUTH chose — `None` routes to `default`, `Some` records the
    // success itself, `Some("default")` included (§5).
    let mut envelope: Option<Envelope> = None;
    let mut identity: Option<String> = None;

    loop {
        let cmd_line = match read_command_line(reader, cfg.idle_timeout).await {
            // Between commands a silent peer is told goodbye before the
            // channel goes down; the goodbye itself may fail (the peer is
            // already gone) and that is fine — the session ends either way.
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                write_line(
                    reader.get_mut(),
                    "421 4.4.2 idle timeout, closing transmission channel",
                    cfg.idle_timeout,
                )
                .await
                .ok();
                return Ok(());
            }
            Err(e) => return Err(e),
            Ok(cmd_line) => match cmd_line {
                CmdLine::Line(line) => line,
                CmdLine::Refused => continue, // over-long: drained, answered, session alive
                CmdLine::Gone => return Ok(()), // peer closed
            },
        };
        let cmd_line = cmd_line.trim_end_matches(['\r', '\n']);
        let (verb, arg) = match cmd_line.find(' ') {
            Some(i) => (
                cmd_line[..i].to_ascii_uppercase(),
                cmd_line[i + 1..].to_string(),
            ),
            None => (cmd_line.to_ascii_uppercase(), String::new()),
        };

        match verb.as_str() {
            // A greeting resets the transaction exactly like RSET and keeps
            // the AUTH identity (decision 0034 §3). Bare greetings are
            // accepted, and so is MAIL before any greeting — the sink's
            // named leniencies (§6).
            "HELO" => {
                envelope = None;
                write_line(
                    reader.get_mut(),
                    &format!("250 {}", cfg.hostname),
                    cfg.idle_timeout,
                )
                .await?;
            }
            "EHLO" => {
                envelope = None;
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
                write_line(reader.get_mut(), &resp, cfg.idle_timeout).await?;
            }
            "STARTTLS" => {
                offer = match offer {
                    TlsOffer::None => {
                        write_line(
                            reader.get_mut(),
                            "454 4.7.0 TLS not available",
                            cfg.idle_timeout,
                        )
                        .await?;
                        TlsOffer::None
                    }
                    TlsOffer::Active => {
                        write_line(
                            reader.get_mut(),
                            "554 5.5.1 TLS already active",
                            cfg.idle_timeout,
                        )
                        .await?;
                        TlsOffer::Active
                    }
                    TlsOffer::Offered(acc) if !arg.is_empty() => {
                        write_line(
                            reader.get_mut(),
                            "501 5.5.4 Syntax error (no parameters allowed)",
                            cfg.idle_timeout,
                        )
                        .await?;
                        TlsOffer::Offered(acc)
                    }
                    TlsOffer::Offered(acc) => {
                        write_line(
                            reader.get_mut(),
                            "220 2.0.0 Ready to start TLS",
                            cfg.idle_timeout,
                        )
                        .await?;
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
                        envelope = None;
                        identity = None;
                        TlsOffer::Active
                    }
                };
            }
            "AUTH" => {
                if !cfg.accept_any_auth {
                    write_line(
                        reader.get_mut(),
                        "502 5.5.2 AUTH disabled",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                // RFC 4954 §4: AUTH is not permitted during a mail
                // transaction. The transaction survives the refusal — the
                // client may finish it or RSET first.
                if envelope.is_some() {
                    write_line(
                        reader.get_mut(),
                        "503 5.5.1 Error: MAIL transaction in progress",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                // One successful AUTH per TLS epoch (decision 0034 §5): a
                // second is refused before any decoding or prompt, so the
                // identity cannot be switched mid-session — `default`
                // counts. Only STARTTLS starts a new epoch.
                if identity.is_some() {
                    write_line(
                        reader.get_mut(),
                        "503 5.5.1 Error: already authenticated",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                // A mechanism and at most one initial response: nothing
                // missing, nothing ignored.
                let mut parts = arg.split_whitespace();
                let (Some(mechanism), b64, None) = (parts.next(), parts.next(), parts.next())
                else {
                    write_line(
                        reader.get_mut(),
                        "501 5.5.4 Syntax: AUTH mechanism [initial-response]",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                };
                let b64 = b64.map(str::to_string);
                match mechanism.to_ascii_uppercase().as_str() {
                    "PLAIN" => {
                        let b64 = match b64 {
                            Some(b) => Some(b),
                            None => {
                                write_line(reader.get_mut(), "334 ", cfg.idle_timeout).await?;
                                match read_command_line(reader, cfg.idle_timeout).await? {
                                    CmdLine::Line(line) => {
                                        Some(line.trim_end_matches(['\r', '\n']).to_string())
                                    }
                                    CmdLine::Refused => {
                                        // Fails only this AUTH attempt.
                                        continue;
                                    }
                                    CmdLine::Gone => return Ok(()),
                                }
                            }
                        };
                        match auth_plain_inbox(b64.as_deref()) {
                            // The credential named an inbox: the session's
                            // identity is assigned.
                            Some(inbox_name) => identity = Some(inbox_name),
                            // It named nothing — undecodable base64, not
                            // exactly three fields, an empty authcid, the
                            // `*` cancel, a non-UTF-8 one — so the refusal
                            // (decisions 0028 §2, 0034 §5) changes nothing:
                            // the session stays unauthenticated, a later
                            // delivery flows to `default`, and a retry may
                            // still succeed.
                            None => {
                                write_line(
                                    reader.get_mut(),
                                    "501 5.5.2 Credentials refused",
                                    cfg.idle_timeout,
                                )
                                .await?;
                                continue;
                            }
                        }
                        write_line(reader.get_mut(), "235 2.7.0 Accepted", cfg.idle_timeout)
                            .await?;
                    }
                    "LOGIN" => {
                        // RFC 4954 §4: the optional initial response is the
                        // username; only without one is the Username prompt
                        // sent. smtplib sends it inline and would otherwise
                        // answer the Username prompt with the password.
                        let username = match b64 {
                            Some(initial) => Some(initial),
                            None => {
                                write_line(reader.get_mut(), "334 VXNlcm5hbWU6", cfg.idle_timeout)
                                    .await?;
                                match read_command_line(reader, cfg.idle_timeout).await? {
                                    CmdLine::Line(line) => {
                                        Some(line.trim_end_matches(['\r', '\n']).to_string())
                                    }
                                    CmdLine::Refused => continue, // fails only this AUTH attempt
                                    CmdLine::Gone => return Ok(()),
                                }
                            }
                        };
                        // The username must name an inbox; anything else —
                        // undecodable base64, non-UTF-8, empty, the `*`
                        // cancel — is refused and changes nothing (decision
                        // 0028 §2). The inbox is assigned only once the
                        // whole credential is consumed: a cancelled password
                        // must not leave a half-applied AUTH behind.
                        let Some(user) = username.and_then(|line| auth_login_username(&line))
                        else {
                            write_line(
                                reader.get_mut(),
                                "501 5.5.2 Credentials refused",
                                cfg.idle_timeout,
                            )
                            .await?;
                            continue;
                        };
                        write_line(reader.get_mut(), "334 UGFzc3dvcmQ6", cfg.idle_timeout).await?;
                        let password = match read_command_line(reader, cfg.idle_timeout).await? {
                            CmdLine::Line(line) => line.trim_end_matches(['\r', '\n']).to_string(),
                            CmdLine::Refused => continue, // fails only this AUTH attempt
                            CmdLine::Gone => return Ok(()),
                        };
                        if password == "*" {
                            // The cancel token names nothing, like any
                            // refused credential (decision 0028 §2).
                            write_line(
                                reader.get_mut(),
                                "501 5.5.2 Credentials refused",
                                cfg.idle_timeout,
                            )
                            .await?;
                            continue;
                        }
                        // Accept-any: the password is consumed, never checked.
                        identity = Some(user);
                        write_line(reader.get_mut(), "235 2.7.0 Accepted", cfg.idle_timeout)
                            .await?;
                    }
                    _ => {
                        write_line(
                            reader.get_mut(),
                            "504 5.5.4 Unrecognized authentication type",
                            cfg.idle_timeout,
                        )
                        .await?
                    }
                }
            }
            // MAIL and RCPT check state, then syntax, then parameters (and
            // RCPT's ceiling), and only then roll chaos (decision 0034 §3):
            // every refusal leaves the open transaction exactly as it was,
            // and an injected failure only ever hits a command that would
            // otherwise have been accepted.
            "MAIL" => {
                // A transaction is open: a nested MAIL is refused and the
                // open transaction survives (RFC 5321 §4.1.4) — state
                // unchanged, so the client can keep adding recipients.
                if envelope.is_some() {
                    write_line(
                        reader.get_mut(),
                        "503 5.5.1 Error: nested MAIL command",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                let Some((from, params)) = envelope_path(&arg, "FROM:") else {
                    write_line(reader.get_mut(), MAIL_PATH_SYNTAX, cfg.idle_timeout).await?;
                    continue;
                };
                if let Err(refusal) = mail_params(params, cfg.max_message_size) {
                    write_line(reader.get_mut(), refusal, cfg.idle_timeout).await?;
                    continue;
                }
                if let Some(rule) = chaos.check(ChaosEvent::MailFrom) {
                    tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
                    write_line(
                        reader.get_mut(),
                        rule.error.as_deref().unwrap_or("451 4.3.0 chaos"),
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                envelope = Some(Envelope {
                    from: from.to_string(),
                    rcpts: Vec::new(),
                });
                write_line(reader.get_mut(), "250 2.1.0 OK", cfg.idle_timeout).await?;
            }
            "RCPT" => {
                let Some(open) = envelope.as_mut() else {
                    write_line(
                        reader.get_mut(),
                        "503 5.5.1 Error: need MAIL command",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                };
                // Only MAIL may name the null path: a recipient is never
                // `<>`.
                let to = match envelope_path(&arg, "TO:") {
                    Some((to, params)) if !to.is_empty() => {
                        if !params.trim().is_empty() {
                            write_line(reader.get_mut(), RCPT_PARAM_UNKNOWN, cfg.idle_timeout)
                                .await?;
                            continue;
                        }
                        to
                    }
                    _ => {
                        write_line(reader.get_mut(), RCPT_PATH_SYNTAX, cfg.idle_timeout).await?;
                        continue;
                    }
                };
                // The recipient ceiling (decision 0027): the 101st RCPT is
                // refused before it is appended, so the transaction never
                // grows past it — an over-long list is answered `452 4.5.3`,
                // the transient shape a real sender can retry smaller.
                if open.rcpts.len() >= MAX_RCPTS {
                    write_line(
                        reader.get_mut(),
                        "452 4.5.3 Too many recipients",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                if let Some(rule) = chaos.check(ChaosEvent::Rcpt) {
                    tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
                    write_line(
                        reader.get_mut(),
                        rule.error.as_deref().unwrap_or("451 4.3.0 chaos"),
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                open.rcpts.push(to.to_string());
                write_line(reader.get_mut(), "250 2.1.5 OK", cfg.idle_timeout).await?;
            }
            "DATA" => {
                // The two 503s stay distinct and name the state the client is
                // actually in (RFC 5321 §4.1.4 ordering): a DATA that cannot
                // name a current MAIL — none at all, or one already consumed
                // by its own completion — answers `need MAIL`; `need RCPT`
                // is for an open transaction with no recipients yet, which
                // the client can still satisfy, so it leaves the transaction
                // open.
                //
                // Past both checks the envelope is taken here, once: every
                // exit from the read loop below other than the real
                // terminator ends the session, so this is the consumption
                // at the terminator — every outcome arm (552, chaos, 250,
                // 451) runs against a taken transaction and none leaves
                // recipients bleeding into the next mail.
                let Envelope { from, rcpts } = match envelope.take() {
                    None => {
                        write_line(
                            reader.get_mut(),
                            "503 5.5.1 Error: need MAIL command",
                            cfg.idle_timeout,
                        )
                        .await?;
                        continue;
                    }
                    Some(open) if open.rcpts.is_empty() => {
                        envelope = Some(open);
                        write_line(
                            reader.get_mut(),
                            "503 5.5.1 Error: need RCPT command",
                            cfg.idle_timeout,
                        )
                        .await?;
                        continue;
                    }
                    Some(open) => open,
                };
                write_line(
                    reader.get_mut(),
                    "354 End data with <CR><LF>.<CR><LF>",
                    cfg.idle_timeout,
                )
                .await?;

                let mut raw: Vec<u8> = Vec::with_capacity(4096);
                let mut over_limit = false;
                // Whether the previous wire line ended in CRLF — true right
                // after the 354, so an immediate `.` CRLF is the empty
                // message. A `.` CRLF line ends DATA only behind a CRLF
                // (decision 0034 §1); behind a bare LF it is payload. Read
                // off the wire bytes before unstuffing, discarded over-cap
                // lines included — never off what `raw` could hold.
                let mut prev_crlf = true;
                loop {
                    // The per-line allowance: one byte more than the message
                    // may still accept, because a leading `..` unstuffs one
                    // byte shorter — an exactly-cap message whose final line
                    // is dot-stuffed fits — and floored at 3 so the
                    // terminator, `.` CRLF, is always recognizable at the cap
                    // and while draining. No other terminator is recognized
                    // (RFC 5321 §2.3.8): a `.` + bare-LF line, and a `.`
                    // CRLF line behind a bare LF, are payload, passed
                    // verbatim by the `..`-only unstuffing — the
                    // SMTP-smuggling class never opens here.
                    let allowance = cfg
                        .max_message_size
                        .saturating_sub(raw.len())
                        .saturating_add(1)
                        .max(3);
                    // The wait for the next data line is one idle bound wide;
                    // an expiry is exactly the vanished-peer statement — the
                    // session ends with nothing stored, no goodbye mid-DATA
                    // (the peer's message was never complete enough to
                    // answer for).
                    let bytes =
                        match bound_io(read_line_capped(&mut *reader, allowance), cfg.idle_timeout)
                            .await
                        {
                            Ok(Line::Command(bytes)) => bytes,
                            // A line the allowance refuses belongs to an over-cap
                            // message: it was drained unbuffered, the flag sticks
                            // and the stream is in sync for the terminator — its
                            // real ending decides whether the next `.` CRLF is one.
                            Ok(Line::TooLong { crlf }) => {
                                over_limit = true;
                                prev_crlf = crlf;
                                continue;
                            }
                            Ok(Line::Eof) => return Ok(()), // peer vanished mid-DATA; nothing to store
                            Err(e) if e.kind() == io::ErrorKind::TimedOut => return Ok(()),
                            Err(e) => return Err(e),
                        };
                    if prev_crlf && bytes == b".\r\n" {
                        break;
                    }
                    prev_crlf = bytes.ends_with(b"\r\n");
                    // Dot-unstuffing: RFC 5321 §4.5.2
                    let payload: &[u8] = if bytes.starts_with(b"..") {
                        &bytes[1..]
                    } else {
                        &bytes
                    };
                    if raw.len() + payload.len() > cfg.max_message_size {
                        over_limit = true;
                    } else {
                        raw.extend_from_slice(payload);
                    }
                }
                if over_limit {
                    write_line(
                        reader.get_mut(),
                        "552 5.3.4 Message too big",
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }
                if let Some(rule) = chaos.check(ChaosEvent::Data) {
                    // One policy at every stage (issue #17): a fired rule
                    // waits, then fails — an explicit `error` line, else this
                    // stage's default `451 4.3.0 chaos`. A delay-only DATA
                    // rule is a failure, not a slow success: the mail is
                    // never stored.
                    tokio::time::sleep(std::time::Duration::from_millis(rule.delay_ms)).await;
                    write_line(
                        reader.get_mut(),
                        rule.error.as_deref().unwrap_or("451 4.3.0 chaos"),
                        cfg.idle_timeout,
                    )
                    .await?;
                    continue;
                }

                // The delivery is observable from the DATA terminator until
                // the insert resolves — and since 0030 §1 the parse sits in
                // that window too: the inbox's
                // `swarmail_deliveries_in_flight` row is live while the mail
                // is being built as well as while its commit is in motion,
                // so a test sequencing stop/chaos against a delivery sees
                // the whole seam the wire actually has. The guard releases
                // the row on resolve or unwind. Parked with the session's
                // inbox name: the Email that carries it does not exist yet.
                // An unauthenticated session routes to `default`.
                let inbox = identity.as_deref().unwrap_or("default");
                let _in_flight = crate::api::park_delivery(inbox);
                // Whole parse and whole mutation on the blocking pool,
                // awaited before the reply: the 250 below stays *after* the
                // commit outcome is known (decision 0001), it just no longer
                // occupies an async worker (decision 0023) — the parse
                // joined the mutation there (decision 0030 §1).
                // `build_email` is pure CPU, so the block is a move, not an
                // await: raw, the taken recipients and the taken sender
                // move in; the session's inbox is copied because the
                // session lives on. A panic in the parse re-raises here,
                // exactly as the synchronous call would (decision 0023). A
                // failed commit refuses (decision 0012) with the fixed line
                // — a transient 451 (RFC 5321 §4.2.2, requested action
                // aborted), distinct from chaos's fixed `451 4.3.0 chaos`,
                // internals to the log and never to the peer — where the old
                // tree acknowledged the lie.
                let email = Store::offload({
                    let inbox = inbox.to_string();
                    move || build_email(&raw, &inbox, &rcpts, from)
                })
                .await;
                let id = email.id.clone();
                match store.clone().insert_async(email).await {
                    Ok(_) => {
                        write_line(
                            reader.get_mut(),
                            &format!("250 2.0.0 OK: stored as {id}"),
                            cfg.idle_timeout,
                        )
                        .await?
                    }
                    Err(_) => {
                        write_line(
                            reader.get_mut(),
                            "451 4.3.0 storage commit failed",
                            cfg.idle_timeout,
                        )
                        .await?
                    }
                }
            }
            "RSET" => {
                envelope = None;
                write_line(reader.get_mut(), "250 2.0.0 OK", cfg.idle_timeout).await?;
            }
            "NOOP" => write_line(reader.get_mut(), "250 2.0.0 OK", cfg.idle_timeout).await?,
            "VRFY" => {
                write_line(
                    reader.get_mut(),
                    "252 2.1.5 Cannot VRFY user",
                    cfg.idle_timeout,
                )
                .await?
            }
            "QUIT" => {
                write_line(reader.get_mut(), "221 2.0.0 Bye", cfg.idle_timeout).await?;
                return Ok(());
            }
            _ => {
                write_line(
                    reader.get_mut(),
                    "502 5.5.2 Command not recognized",
                    cfg.idle_timeout,
                )
                .await?
            }
        }
    }
}

/// The inbox a PLAIN credential names, or `None` when it names nothing:
/// undecodable base64, anything but exactly three NUL-separated fields, an
/// empty-after-trim authcid, or a non-UTF-8 one. `None` is the `501 5.5.2`
/// refusal (decisions 0028 §2, 0034 §5) — the session's inbox is assigned
/// only on success, never minted from a replacement character, and never
/// from a password that a short credential shifted into the authcid's seat.
fn auth_plain_inbox(b64: Option<&str>) -> Option<String> {
    let decoded = b64_decode(b64?)?;
    // PLAIN: authzid NUL authcid NUL passwd — the authcid names the inbox;
    // the authzid is ignored and the password accepted unchecked.
    let mut fields = decoded.split(|b| *b == 0);
    let (Some(_authzid), Some(user), Some(_password), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return None;
    };
    let user = String::from_utf8(user.to_vec()).ok()?;
    let user = user.trim();
    if user.is_empty() {
        None
    } else {
        Some(user.to_string())
    }
}

/// The inbox a LOGIN username line names, or `None` when it names nothing —
/// undecodable base64, a non-UTF-8 username, or an empty-after-trim one
/// (decision 0028 §2). The literal `*` cancel is not valid base64 and
/// refuses the same way.
fn auth_login_username(line: &str) -> Option<String> {
    let decoded = b64_decode(line)?;
    let user = String::from_utf8(decoded).ok()?;
    let user = user.trim();
    if user.is_empty() {
        None
    } else {
        Some(user.to_string())
    }
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

/// The seed defaults both adapters share (decision 0010) — one owner, so the
/// two fixtures cannot drift apart again.
const SEED_FROM: &str = "fixture@swarmail.dev";
const SEED_SUBJECT: &str = "(no subject)";

/// Build the fixture both adapters seed with (decision 0010, issue #5): the
/// shared defaults, HTML preserved verbatim (superseding `text` when both are
/// given), and the raw shape the REST seed has always written — ending in the
/// ordinary [`build_email`], so extraction still happens at ingest (0005).
///
/// Unlike [`build_email`], which runs on hostile wire bytes all day, a
/// fixture *promises* the caller that the mail that lands is the fixture
/// that was asked for. So `CR`, `LF` or `NUL` in `to`, `from` or `subject`
/// is refused with an `Err` naming the field **before** any insert: nothing
/// is stored and the inbox count is unchanged, which is what lets the refusal
/// compose with the exact-count assert (issue #6).
pub fn build_seed_email(
    inbox: &str,
    to: &str,
    from: Option<&str>,
    subject: Option<&str>,
    text: Option<&str>,
    html: Option<&str>,
) -> Result<Email, String> {
    let envelope = [
        ("to", to),
        ("from", from.unwrap_or(SEED_FROM)),
        ("subject", subject.unwrap_or(SEED_SUBJECT)),
    ];
    for (field, value) in envelope {
        if value.contains(['\r', '\n', '\0']) {
            return Err(format!(
                "invalid {field}: envelope fields refuse CR, LF and NUL"
            ));
        }
    }
    let mut raw = format!(
        "From: {}\r\nTo: {to}\r\nSubject: {}\r\n",
        from.unwrap_or(SEED_FROM),
        subject.unwrap_or(SEED_SUBJECT)
    );
    if let Some(html) = html {
        raw.push_str(&format!(
            "MIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n{html}\r\n"
        ));
    } else {
        raw.push_str(&format!("\r\n{}\r\n", text.unwrap_or_default()));
    }
    Ok(build_email(
        raw.as_bytes(),
        inbox,
        &[to.to_string()],
        from.unwrap_or(SEED_FROM).to_string(),
    ))
}

/// The first *text/plain* body part, verbatim — the sender's own text, with
/// no synthesis (decision 0031 §1). A body part of any other type yields
/// `None` even though the parser lists it: in a text-only
/// multipart/alternative the text part is pushed into `html_body` as a
/// fallback, so the part-type decides, not the list.
fn sender_text_part(message: &mail_parser::Message<'_>) -> Option<String> {
    match &message.text_part(0)?.body {
        PartType::Text(text) => Some(text.to_string()),
        _ => None,
    }
}

/// The first *text/html* body part, verbatim — the sender's own html, with
/// no synthesis (decision 0031 §1).
fn sender_html_part(message: &mail_parser::Message<'_>) -> Option<String> {
    match &message.html_part(0)?.body {
        PartType::Html(html) => Some(html.to_string()),
        _ => None,
    }
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
    // The stored record holds only what the sender sent (decision 0031 §1):
    // a text/plain body part stores `text`, a text/html body part stores
    // `html`, and a mail with neither — or with some other body shape —
    // stores neither. mail-parser's `body_text`/`body_html` *synthesize*:
    // with one variant present they also hand back the other, converted in
    // the wrong direction (`body_text` on an html part converts html→text,
    // `body_html` on a text part converts text→html), which would store a
    // body the sender never sent. The part-type match below is the honest
    // accessor — in a multipart/alternative missing one variant, the parser
    // pushes the present part into both body lists, so list membership is
    // not part-type membership.
    let text = parsed.as_ref().and_then(sender_text_part);
    let html = parsed.as_ref().and_then(sender_html_part);

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
    use crate::store::Filter;

    /// The stored bodies are exactly what the sender sent (decision 0031
    /// §1): a text part stores `text` and no `html`, an html part stores
    /// `html` and no `text`, an alternative with both stores both, and a
    /// bodyless mail stores neither. The parser's own synthesizing
    /// accessors would hand back a converted body in each single-variant
    /// case — the bug issue #39 names.
    #[test]
    fn bodies_hold_only_what_the_sender_sent() {
        let text_mail = build_email(
            b"From: s@x.io\r\nSubject: t\r\n\r\nplain body\r\n",
            "in",
            &["r@x.io".to_string()],
            "s@x.io".into(),
        );
        assert_eq!(text_mail.text.as_deref(), Some("plain body\r\n"));
        assert!(text_mail.html.is_none(), "no html synthesized from text");

        let html_mail = build_email(
            b"From: s@x.io\r\nSubject: h\r\nMIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n<p>rich body</p>\r\n",
            "in",
            &["r@x.io".to_string()],
            "s@x.io".into(),
        );
        assert!(html_mail.html.is_some());
        assert!(html_mail.text.is_none(), "no text synthesized from html");

        let both = build_email(
            b"From: s@x.io\r\nSubject: b\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"BB\"\r\n\r\n--BB\r\nContent-Type: text/plain\r\n\r\nplain\r\n--BB\r\nContent-Type: text/html\r\n\r\n<p>rich</p>\r\n--BB--\r\n",
            "in",
            &["r@x.io".to_string()],
            "s@x.io".into(),
        );
        assert_eq!(both.text.as_deref(), Some("plain"));
        assert!(both.html.as_deref().unwrap().contains("rich"));

        let bodyless = build_email(
            b"From: s@x.io\r\nSubject: n\r\n",
            "in",
            &["r@x.io".to_string()],
            "s@x.io".into(),
        );
        assert!(bodyless.text.is_none());
        assert!(bodyless.html.is_none());
    }

    /// An AsyncRead that yields its bytes one per poll — the harshest
    /// chunking a peer can produce, so every buffer boundary is a chunk
    /// boundary and every line straddles reads.
    struct Bytewise {
        data: Vec<u8>,
        pos: usize,
    }

    impl Bytewise {
        fn new(data: &[u8]) -> Self {
            Self {
                data: data.to_vec(),
                pos: 0,
            }
        }
    }

    impl AsyncRead for Bytewise {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            if this.pos >= this.data.len() {
                return Poll::Ready(Ok(())); // EOF
            }
            buf.put_slice(&this.data[this.pos..this.pos + 1]);
            this.pos += 1;
            Poll::Ready(Ok(()))
        }
    }

    /// A complete line survives straddled chunks byte-intact: CRLF kept, no
    /// trim, no re-encoding, and the following line is still next.
    #[tokio::test]
    async fn a_complete_line_survives_straddled_chunks_byte_intact() {
        let mut reader = BufReader::new(Bytewise::new(b"EHLO test\r\nMAIL FROM:<a@b>\r\n"));
        let line = read_line_capped(&mut reader, MAX_COMMAND_LINE)
            .await
            .unwrap();
        assert_eq!(line, Line::Command(b"EHLO test\r\n".to_vec()));
        let line = read_line_capped(&mut reader, MAX_COMMAND_LINE)
            .await
            .unwrap();
        assert_eq!(line, Line::Command(b"MAIL FROM:<a@b>\r\n".to_vec()));
    }

    /// An over-long line is refused once its cap is passed and — the part a
    /// wire test cannot see — the tail was drained, so the next line is read
    /// whole afterwards.
    #[tokio::test]
    async fn an_over_long_line_is_refused_and_the_next_line_read_whole() {
        let mut reader = BufReader::new(Bytewise::new(b"0123456789ABCDEF\r\nnext\r\n"));
        let line = read_line_capped(&mut reader, 8).await.unwrap();
        // Bytewise: the CR is the last byte of one drain chunk and the LF
        // the whole next one — still a CRLF.
        assert_eq!(line, Line::TooLong { crlf: true });
        let line = read_line_capped(&mut reader, 8).await.unwrap();
        assert_eq!(line, Line::Command(b"next\r\n".to_vec()));

        let mut reader = BufReader::new(Bytewise::new(b"0123456789ABCDEF\nnext\r\n"));
        let line = read_line_capped(&mut reader, 8).await.unwrap();
        assert_eq!(line, Line::TooLong { crlf: false });
        let line = read_line_capped(&mut reader, 8).await.unwrap();
        assert_eq!(line, Line::Command(b"next\r\n".to_vec()));
    }

    /// An AsyncRead that yields exactly the chunks it was given, one per
    /// poll — the controlled read boundaries `Bytewise` cannot express: a
    /// whole over-cap line in one buffer, or a CR/LF split at a chosen byte.
    struct Chunks(std::collections::VecDeque<Vec<u8>>);

    impl Chunks {
        fn new(chunks: &[&[u8]]) -> Self {
            Self(chunks.iter().map(|c| c.to_vec()).collect())
        }
    }

    impl AsyncRead for Chunks {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if let Some(chunk) = self.get_mut().0.pop_front() {
                buf.put_slice(&chunk);
            }
            Poll::Ready(Ok(())) // nothing left: EOF
        }
    }

    /// The over-cap line's real ending, chunked as `chunks` under cap 8,
    /// and the line read after it — so every case also proves the stream
    /// stayed in sync.
    async fn too_long_then(chunks: &[&[u8]]) -> (Line, Line) {
        let mut reader = BufReader::new(Chunks::new(chunks));
        let first = read_line_capped(&mut reader, 8).await.unwrap();
        let next = read_line_capped(&mut reader, 8).await.unwrap();
        (first, next)
    }

    /// Every over-cap exit reports the discarded line's true ending
    /// (decision 0034 §1): the whole-buffer exit reads it off the buffered
    /// line, the drain exit off the bytes it consumes — with the CR before
    /// the LF found in the same chunk, at the drain's entry, or at the end
    /// of an earlier drain chunk. Never guessed either way.
    #[tokio::test]
    async fn an_over_cap_line_reports_its_real_ending_on_every_exit() {
        let next = || Line::Command(b"next\r\n".to_vec());
        let crlf = || Line::TooLong { crlf: true };
        let lf = || Line::TooLong { crlf: false };

        // Whole line in one buffer, over the cap on arrival.
        assert_eq!(
            too_long_then(&[b"0123456789\r\nnext\r\n"]).await,
            (crlf(), next())
        );
        assert_eq!(
            too_long_then(&[b"0123456789\nnext\r\n"]).await,
            (lf(), next())
        );

        // Drained: CR and LF in the same later chunk, or LF alone there.
        assert_eq!(
            too_long_then(&[b"0123456789", b"AB\r\nnext\r\n"]).await,
            (crlf(), next())
        );
        assert_eq!(
            too_long_then(&[b"0123456789", b"AB\nnext\r\n"]).await,
            (lf(), next())
        );

        // The CR is the last buffered byte when the drain begins and the LF
        // the drain's first byte: the seed carries it.
        assert_eq!(
            too_long_then(&[b"012345678\r", b"\nnext\r\n"]).await,
            (crlf(), next())
        );
        // The CR ends one drain chunk, the LF opens the next.
        assert_eq!(
            too_long_then(&[b"0123456789", b"AB\r", b"\nnext\r\n"]).await,
            (crlf(), next())
        );
        // The drain's first byte is an LF behind no CR.
        assert_eq!(
            too_long_then(&[b"0123456789", b"\nnext\r\n"]).await,
            (lf(), next())
        );
        // A CR earlier in a drain chunk is not the byte before the LF.
        assert_eq!(
            too_long_then(&[b"0123456789", b"\rAB", b"\nnext\r\n"]).await,
            (lf(), next())
        );

        // EOF inside the drain, right after a CR: no LF, no CRLF invented.
        assert_eq!(
            too_long_then(&[b"0123456789", b"AB\r"]).await,
            (lf(), Line::Eof)
        );
    }

    /// EOF before any byte, or a final line with no terminator: neither is a
    /// line the session can act on.
    #[tokio::test]
    async fn eof_before_any_byte_or_without_a_terminator_is_not_a_line() {
        let mut reader = BufReader::new(Bytewise::new(b""));
        assert_eq!(read_line_capped(&mut reader, 8).await.unwrap(), Line::Eof);

        let mut reader = BufReader::new(Bytewise::new(b"NOOP"));
        assert_eq!(read_line_capped(&mut reader, 8).await.unwrap(), Line::Eof);
    }

    /// An over-long line that never ends is drained to EOF unbuffered and
    /// still refused; the session then just sees EOF.
    #[tokio::test]
    async fn an_over_long_line_without_a_terminator_drains_to_eof() {
        let mut reader = BufReader::new(Bytewise::new(b"0123456789ABCDEF"));
        assert_eq!(
            read_line_capped(&mut reader, 8).await.unwrap(),
            Line::TooLong { crlf: false }
        );
        assert_eq!(read_line_capped(&mut reader, 8).await.unwrap(), Line::Eof);
    }

    /// The MAIL/RCPT path framing (decision 0034 §3): keyword
    /// case-insensitive, optional whitespace after the colon, a bracketed
    /// path kept opaque, and nothing or whitespace before any parameters.
    #[test]
    fn envelope_paths_are_framed_strictly_and_kept_opaque() {
        fn from(arg: &str) -> Option<(&str, &str)> {
            envelope_path(arg, "FROM:")
        }
        assert_eq!(from("FROM:<a@b>"), Some(("a@b", "")));
        assert_eq!(from("from: <a@b>"), Some(("a@b", "")));
        assert_eq!(from("From:\t<>"), Some(("", "")));
        assert_eq!(from("FROM:<alice> SIZE=1"), Some(("alice", " SIZE=1")));
        assert_eq!(from("FROM:<a@b>\tBODY=7BIT"), Some(("a@b", "\tBODY=7BIT")));
        assert_eq!(
            envelope_path("TO:<j\u{f6}rg@x.io>", "TO:"),
            Some(("j\u{f6}rg@x.io", ""))
        );
        assert_eq!(envelope_path("to:<r@x.io> ", "TO:"), Some(("r@x.io", " ")));

        // Nothing; shorter than the keyword; the wrong keyword; no colon;
        // bracketless; inverted; unclosed; a second `<` inside; a stray `>`
        // glued on; a suffix glued to the bracket; text before the bracket;
        // a keyword length that is not even a char boundary.
        for refused in [
            "",
            "FR",
            "TO:<a@b>",
            "FROM <a@b>",
            "FROM:a@b",
            "FROM:>a@b<",
            "FROM:<a@b",
            "FROM:<<a@b>>",
            "FROM:<a@b>>",
            "FROM:<a@b>SIZE=1",
            "FROM:x<a@b>",
            "\u{e9}\u{e9}\u{e9}<a@b>",
        ] {
            assert_eq!(from(refused), None, "{refused:?}");
        }
    }

    /// MAIL parameters, left to right, first error wins (decision 0034 §4).
    #[test]
    fn mail_parameters_follow_the_table() {
        let cap = 64;
        for accepted in [
            "",
            " ",
            " SIZE=0",
            " SIZE=64",
            " size=000064",
            " SIZE=1 BODY=7BIT SMTPUTF8",
            " BODY=8bitmime",
            " smtputf8",
        ] {
            assert_eq!(mail_params(accepted, cap), Ok(()), "{accepted:?}");
        }
        for (params, refusal) in [
            (" SIZE=65", MAIL_SIZE_TOO_BIG),
            (" SIZE=99999999999999999999999999", MAIL_SIZE_TOO_BIG),
            (" SIZE", MAIL_PARAM_INVALID),
            (" SIZE=", MAIL_PARAM_INVALID),
            (" SIZE=12k", MAIL_PARAM_INVALID),
            (" SIZE=+12", MAIL_PARAM_INVALID),
            (" SIZE=-1", MAIL_PARAM_INVALID),
            (" BODY", MAIL_PARAM_INVALID),
            (" BODY=BINARYMIME", MAIL_PARAM_INVALID),
            (" SMTPUTF8=yes", MAIL_PARAM_INVALID),
            (" SIZE=1 SIZE=1", MAIL_PARAM_INVALID),
            (" BODY=7BIT body=7bit", MAIL_PARAM_INVALID),
            (" SMTPUTF8 SMTPUTF8", MAIL_PARAM_INVALID),
            (" RET=FULL", MAIL_PARAM_UNKNOWN),
            (" =1", MAIL_PARAM_UNKNOWN),
            // Left to right: the first error wins, whichever it is.
            (" RET=FULL SIZE=x", MAIL_PARAM_UNKNOWN),
            (" SIZE=x RET=FULL", MAIL_PARAM_INVALID),
            (" SIZE=65 RET=FULL", MAIL_SIZE_TOO_BIG),
            // A duplicate is refused before its value is weighed.
            (" SIZE=1 SIZE=65", MAIL_PARAM_INVALID),
        ] {
            assert_eq!(mail_params(params, cap), Err(refusal), "{params:?}");
        }
    }

    /// PLAIN is exactly `authzid NUL authcid NUL password`: the authcid and
    /// only the authcid names the inbox — never a password shifted into its
    /// seat by a short credential (decision 0034 §5).
    #[test]
    fn plain_requires_exactly_three_fields() {
        use base64::Engine;
        let plain = |raw: &[u8]| {
            auth_plain_inbox(Some(&base64::engine::general_purpose::STANDARD.encode(raw)))
        };
        assert_eq!(plain(b"\0box\0pw"), Some("box".to_string()));
        assert_eq!(plain(b"admin\0box\0"), Some("box".to_string()));
        assert_eq!(plain(b"\0 box \0pw"), Some("box".to_string()));
        assert_eq!(plain(b""), None);
        assert_eq!(plain(b"secret"), None);
        assert_eq!(plain(b"box\0secret"), None);
        assert_eq!(plain(b"\0secret"), None);
        assert_eq!(plain(b"\0box\0pw\0more"), None);
        assert_eq!(plain(b"\0\0pw"), None);
        assert_eq!(plain(b"\0\xff\0pw"), None);
        assert_eq!(auth_plain_inbox(Some("!!!")), None);
        assert_eq!(auth_plain_inbox(None), None);
    }

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
        // The refusal message rides the Debug rendering of the poll result —
        // an assertion on the formatted text, with no arm of its own to cover.
        let reported = format!("{refused:?}");
        assert!(
            reported.contains("taken"),
            "the refusal names the posture: {reported}"
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

    /// The accept loop reaps completed sessions: each offered connection is
    /// one loop iteration, and each iteration's harvest joins every session
    /// completed so far — so a completed entry is reaped by a later accept
    /// instead of accumulating in the set. Polled against a real deadline
    /// (never a sleep): with the harvest gone the counter never advances and
    /// the deadline expires red. Returns the counter so a second batch can
    /// prove the reap keeps tracking completions.
    async fn reap_completed_sessions_until(count: usize) -> usize {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            Arc::new(Chaos::default()),
            SmtpConfig::default(),
            None,
            Arc::new(AtomicBool::new(false)),
            std::future::pending(),
        ));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let reaped = loop {
            assert!(
                std::time::Instant::now() < deadline,
                "the accept loop never reaped {count} completed session(s)"
            );
            // EOF on delivery: the session ends, the next accept reaps it.
            drop(TcpStream::connect(addr).await.unwrap());
            let reaped = REAPED.load(Ordering::Relaxed);
            if reaped >= count {
                break reaped;
            }
            tokio::task::yield_now().await;
        };
        server.abort();
        reaped
    }

    /// Completed connection entries do not accumulate across successive
    /// batches: the first batch is reaped, and the second advances the
    /// counter beyond it — the harvest is continuous, not a shutdown-only
    /// drain (red on the join-in-shutdown-arm-only shape, which never reaps).
    #[tokio::test]
    async fn the_accept_loop_reaps_completed_sessions_across_batches() {
        let after_first_batch = reap_completed_sessions_until(1).await;
        assert!(after_first_batch >= 1, "{after_first_batch}");
        let after_second_batch = reap_completed_sessions_until(after_first_batch + 1).await;
        assert!(
            after_second_batch > after_first_batch,
            "the reap must keep tracking completions: {after_first_batch} -> {after_second_batch}"
        );
    }

    /// The flag a test observes flipping, spun against a real deadline —
    /// never a sleep (decision 0020 rule 5): the loop sets and clears it as
    /// a side effect of its own arms, on the polling thread's clock.
    async fn flag_becomes(flag: &AtomicBool, want: bool, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while flag.load(Ordering::Relaxed) != want {
            assert!(
                std::time::Instant::now() < deadline,
                "the accept loop never {what}"
            );
            tokio::task::yield_now().await;
        }
    }

    /// One reply line from a loopback SMTP peer, byte-at-a-time so replies
    /// the server pipelines behind the banner are still consumed one line
    /// per call.
    async fn reply(stream: &mut TcpStream) -> String {
        use tokio::io::AsyncReadExt;
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte).await.unwrap();
            line.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        String::from_utf8_lossy(&line).into_owned()
    }

    /// One real delivery over a driven listener: banner, envelope, DATA,
    /// terminator, QUIT — a session speaking to a real peer, nothing
    /// mocked. The mail is synchronously queryable when the 250 lands.
    /// (No AUTH: the session's inbox is the compiled default, and the store
    /// is per-test anyway.)
    async fn deliver_one(store: &Arc<Store>, addr: std::net::SocketAddr) {
        let mut c = TcpStream::connect(addr).await.unwrap();
        assert!(reply(&mut c).await.starts_with("220 "));
        c.write_all(b"MAIL FROM:<sender@example.com>\r\n")
            .await
            .unwrap();
        assert!(reply(&mut c).await.starts_with("250 "));
        c.write_all(b"RCPT TO:<probe>\r\n").await.unwrap();
        assert!(reply(&mut c).await.starts_with("250 "));
        c.write_all(b"DATA\r\n").await.unwrap();
        assert!(reply(&mut c).await.starts_with("354 "));
        c.write_all(b"Subject: probe\r\n\r\nbody\r\n.\r\n")
            .await
            .unwrap();
        let stored = reply(&mut c).await;
        assert!(stored.starts_with("250 "), "{stored}");
        c.write_all(b"QUIT\r\n").await.unwrap();
        assert!(reply(&mut c).await.starts_with("221 "));
        assert_eq!(
            store.count("default", &Filter::default()),
            1,
            "the 250 means the mail is queryable (0001)"
        );
    }

    /// One injected accept failure: the loop flags itself degraded, backs
    /// off, and then delivers a real mail on the same listener — a
    /// transient accept error is survived, not fatal (red: the old `?`
    /// arm returned from `serve` on the first failure). The flag is also
    /// observed cleared: the next successful accept is the recovery.
    #[tokio::test]
    async fn one_failed_accept_backs_off_then_delivers() {
        let store = Arc::new(Store::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        let server = tokio::spawn(serve(
            listener,
            store.clone(),
            Arc::new(Chaos::default()),
            SmtpConfig::default(),
            None,
            degraded.clone(),
            std::future::pending(),
        ));
        // Flipped before the loop is first polled, so the first accept is
        // the injected failure — the arm is branch-stable (0020 rule 1).
        inject_accept_errors(1);

        flag_becomes(&degraded, true, "flagged degraded on the failed accept").await;
        // The same listener still delivers once the backoff has passed.
        deliver_one(&store, addr).await;
        flag_becomes(
            &degraded,
            false,
            "cleared the flag after the successful accept",
        )
        .await;
        server.abort();
    }

    /// A stop arriving inside a backoff pause resolves the loop well inside
    /// that pause — the shutdown is raced against the backoff, never
    /// stacked behind it (red: a sleep-then-check shape waits the whole
    /// second before even looking). A live session holds a slot when the
    /// stop lands, so the drain inside the shutdown arm joins a genuinely
    /// aborted task — the shape a stop interrupts a *busy* degraded
    /// listener, not an idle one.
    #[tokio::test]
    async fn a_stop_inside_a_backoff_resolves_well_inside_it() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            Arc::new(Chaos::default()),
            SmtpConfig::default(),
            None,
            degraded.clone(),
            async move {
                let mut stop_rx = stop_rx;
                let _ = stop_rx.changed().await;
            },
        ));
        // The banner is written by the session task, so reading it proves
        // the session is spawned — and it stays spawned, parked on a read
        // this test never answers.
        let mut client = TcpStream::connect(addr).await.unwrap();
        assert!(reply(&mut client).await.starts_with("220 "));
        inject_accept_errors(1);
        // The loop is parked inside the accept (nothing wakes it) — the
        // seam is consumed on the accept's next *poll*, so a second
        // connection is the poke: its handshake fires the accept's waker,
        // the poll re-checks the seam first, and the error wins before the
        // accept is ever awaited. Branch-stable, no sleep.
        let _poke = TcpStream::connect(addr).await.unwrap();
        // The flag is set in the same poll that enters the backoff select,
        // so observing it means the loop is parked there — the stop lands
        // mid-pause, not before it.
        flag_becomes(&degraded, true, "entered the backoff").await;
        let _ = stop_tx.send(true);
        let outcome = tokio::time::timeout(Duration::from_millis(500), server).await;
        let res = outcome.expect("the stop did not resolve inside the backoff");
        assert!(res.is_ok(), "the raced stop must exit cleanly: {res:?}");
    }

    /// A real I/O error mid-DATA — neither the idle deadline's TimedOut nor
    /// a tidy hang-up's EOF — ends the session with the error, stores
    /// nothing, and the listener survives to deliver the next mail. A
    /// linger-zero close sends the RST (the POP3 reset test's shape): the
    /// body line is written first, so the read that observes the reset is
    /// the server's *next* one — the error arm, not the tidy EOF above it.
    #[tokio::test]
    async fn an_io_error_mid_data_ends_the_session_and_the_listener_survives() {
        let store = Arc::new(Store::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(serve(
            listener,
            store.clone(),
            Arc::new(Chaos::default()),
            SmtpConfig::default(),
            None,
            Arc::new(AtomicBool::new(false)),
            std::future::pending(),
        ));
        let mut c = TcpStream::connect(addr).await.unwrap();
        assert!(reply(&mut c).await.starts_with("220 "));
        c.write_all(b"MAIL FROM:<reset@x.io>\r\n").await.unwrap();
        assert!(reply(&mut c).await.starts_with("250 "));
        c.write_all(b"RCPT TO:<probe>\r\n").await.unwrap();
        assert!(reply(&mut c).await.starts_with("250 "));
        c.write_all(b"DATA\r\n").await.unwrap();
        assert!(reply(&mut c).await.starts_with("354 "));
        c.write_all(b"Subject: reset mid-data\r\n").await.unwrap();
        #[allow(deprecated)] // the RST is the point of the test
        c.set_linger(Some(Duration::ZERO)).unwrap();
        drop(c);

        // The listener survived the error: a fresh session delivers as
        // usual — and the count is exactly one, so nothing from the reset
        // one was stored.
        deliver_one(&store, addr).await;
        server.abort();
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
