//! Bounded wake-up transports for `wait-best` stream rows (accepted decision
//! 0132 §7.2, story C): a minimal RFC 6455 client and a server-sent-events
//! reader, on the tokio already in the tree, with the accept key checked
//! through `ring`, which is already locked.
//!
//! Nothing here carries mail. A notification only wakes the authoritative
//! scan; the scan alone answers a wait. So the transports keep no replay,
//! cache or queue of events, and they never decide that a channel is ready:
//! readiness is the observer's, from the cited registration evidence.
//!
//! Every limit is finite and checked before anything is allocated
//! ([`Limits`]). Every wait is against one absolute deadline the caller
//! owns, so trickled input cannot renew it. Reads land in a persistent
//! buffer only once they complete, so a caller may drop a pending
//! [`WebSocket::next`] or [`Sse::next`] (a `select!` against a timer) and
//! lose no byte: the next call resumes from the same parser state.

use base64::Engine as _;
use ring::rand::SecureRandom as _;
use std::fmt;
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout_at};

/// RFC 6455 §1.3: the GUID the accept key is derived with.
pub const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The named limits of the plan (`specs/wait-best/plan.md`): response head,
/// one header or SSE line, one frame or assembled message or event, and all
/// transport buffering together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub head: usize,
    pub line: usize,
    pub message: usize,
    pub buffer: usize,
}

/// A frame header is at most 14 bytes (2 + 8 length + 4 mask).
const FRAME_HEADER_MAX: usize = 14;

impl Limits {
    pub const DEFAULT: Limits = Limits {
        head: 32 * 1024,
        line: 8 * 1024,
        message: 1024 * 1024,
        buffer: 2 * 1024 * 1024,
    };

    /// A set of limits the transports can honour: every bound nonzero, a
    /// line fits in a head, and a head, or a whole frame with its header,
    /// fits in the total buffer.
    pub fn checked(self) -> Result<Limits, StreamError> {
        let fits = self.line > 0
            && self.message > 0
            && self.line <= self.head
            && self.head <= self.buffer
            && self
                .message
                .checked_add(FRAME_HEADER_MAX)
                .is_some_and(|m| m <= self.buffer);
        if fits {
            Ok(self)
        } else {
            Err(StreamError::Limits(self))
        }
    }
}

/// Why a transport stopped. [`StreamError::is_disconnect`] separates a lost
/// connection, which the observer may reconnect, from a malformed or
/// refused one, which is an error of the observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// The caller's absolute deadline passed.
    Deadline,
    /// The connection ended; `at` names where.
    Eof { at: &'static str },
    /// An I/O error other than end of stream.
    Io { kind: ErrorKind, message: String },
    /// The peer closed the WebSocket (and the close was answered).
    Closed,
    /// A limits value the transports cannot honour.
    Limits(Limits),
    /// A bound would be exceeded: `what` names the bound.
    TooLarge { what: &'static str, limit: usize },
    /// The HTTP response head is malformed.
    Head(String),
    /// The response status is not the one the protocol requires.
    Status(u16),
    /// The WebSocket handshake answer does not prove an upgrade.
    Handshake(String),
    /// An extension or subprotocol was negotiated that this client does
    /// not speak.
    Negotiation(String),
    /// The SSE response has the wrong media type or encoding.
    ContentType(String),
    /// The HTTP body framing is malformed or unsupported.
    Framing(String),
    /// A WebSocket frame breaks RFC 6455.
    Frame(&'static str),
    /// Text that must be UTF-8 is not.
    Utf8(&'static str),
}

impl StreamError {
    /// A connection that ended or reset, as opposed to one that spoke
    /// malformed bytes or refused the protocol.
    pub fn is_disconnect(&self) -> bool {
        match self {
            StreamError::Eof { .. } | StreamError::Closed => true,
            StreamError::Io { kind, .. } => matches!(
                kind,
                ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
                    | ErrorKind::UnexpectedEof
            ),
            _ => false,
        }
    }
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Deadline => write!(f, "stream: deadline passed"),
            StreamError::Eof { at } => write!(f, "stream: connection ended {at}"),
            StreamError::Io { kind, message } => write!(f, "stream: {kind:?}: {message}"),
            StreamError::Closed => write!(f, "stream: peer closed the websocket"),
            StreamError::Limits(l) => write!(f, "stream: limits cannot be honoured: {l:?}"),
            StreamError::TooLarge { what, limit } => {
                write!(f, "stream: {what} exceeds its {limit}-byte limit")
            }
            StreamError::Head(why) => write!(f, "stream: malformed response head: {why}"),
            StreamError::Status(code) => write!(f, "stream: unexpected status {code}"),
            StreamError::Handshake(why) => write!(f, "stream: websocket handshake: {why}"),
            StreamError::Negotiation(what) => write!(f, "stream: unsupported negotiation: {what}"),
            StreamError::ContentType(why) => write!(f, "stream: not an event stream: {why}"),
            StreamError::Framing(why) => write!(f, "stream: body framing: {why}"),
            StreamError::Frame(why) => write!(f, "stream: websocket frame: {why}"),
            StreamError::Utf8(what) => write!(f, "stream: {what} is not UTF-8"),
        }
    }
}

impl std::error::Error for StreamError {}

fn io_error(e: std::io::Error) -> StreamError {
    StreamError::Io {
        kind: e.kind(),
        message: e.to_string(),
    }
}

/// One TCP connection to a declared endpoint, bounded by `deadline`.
pub async fn connect(addr: SocketAddr, deadline: Instant) -> Result<TcpStream, StreamError> {
    // `timeout_at` polls the connect before it reads the clock, and a
    // loopback connect can complete on that first poll (macOS does): a
    // passed deadline is refused before any connect starts.
    if Instant::now() >= deadline {
        return Err(StreamError::Deadline);
    }
    match timeout_at(deadline, TcpStream::connect(addr)).await {
        Err(_) => Err(StreamError::Deadline),
        Ok(Err(e)) => Err(io_error(e)),
        Ok(Ok(s)) => {
            s.set_nodelay(true).map_err(io_error)?;
            Ok(s)
        }
    }
}

/// The byte source shared by both transports: a persistent buffer that a
/// read extends only when it completes.
struct Wire<S> {
    io: S,
    buf: Vec<u8>,
    limits: Limits,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Wire<S> {
    /// Read once into the buffer, at most `room` bytes. `Ok(0)` is end of
    /// stream. Cancel-safe: the bytes join `buf` only after the read
    /// completes, and a dropped read consumed nothing.
    async fn fill(&mut self, room: usize, deadline: Instant) -> Result<usize, StreamError> {
        if room == 0 {
            return Err(StreamError::TooLarge {
                what: "transport buffering",
                limit: self.limits.buffer,
            });
        }
        let mut chunk = [0u8; 8192];
        let want = room.min(chunk.len());
        match timeout_at(deadline, self.io.read(&mut chunk[..want])).await {
            Err(_) => Err(StreamError::Deadline),
            Ok(Err(e)) => Err(io_error(e)),
            Ok(Ok(n)) => {
                self.buf.extend_from_slice(&chunk[..n]);
                Ok(n)
            }
        }
    }

    /// Write `out` from its front, removing what was written. Cancel-safe in
    /// the same way: a write that did not complete wrote nothing.
    async fn flush(&mut self, out: &mut Vec<u8>, deadline: Instant) -> Result<(), StreamError> {
        while !out.is_empty() {
            match timeout_at(deadline, self.io.write(out)).await {
                Err(_) => return Err(StreamError::Deadline),
                Ok(Err(e)) => return Err(io_error(e)),
                Ok(Ok(0)) => {
                    return Err(StreamError::Eof {
                        at: "while writing",
                    });
                }
                Ok(Ok(n)) => {
                    out.drain(..n);
                }
            }
        }
        match timeout_at(deadline, self.io.flush()).await {
            Err(_) => Err(StreamError::Deadline),
            Ok(Err(e)) => Err(io_error(e)),
            Ok(Ok(())) => Ok(()),
        }
    }

    /// Read the HTTP/1.1 response head, leaving any bytes after it in the
    /// buffer (a coalesced first frame or event).
    async fn read_head(&mut self, deadline: Instant) -> Result<Head, StreamError> {
        loop {
            if let Some(end) = find(&self.buf, b"\r\n\r\n") {
                if end + 4 > self.limits.head {
                    return Err(StreamError::TooLarge {
                        what: "response head",
                        limit: self.limits.head,
                    });
                }
                let head = parse_head(&self.buf[..end + 2], self.limits)?;
                self.buf.drain(..end + 4);
                return Ok(head);
            }
            check_head_lines(&self.buf, self.limits)?;
            if self.buf.len() >= self.limits.head {
                return Err(StreamError::TooLarge {
                    what: "response head",
                    limit: self.limits.head,
                });
            }
            let room = self.limits.head - self.buf.len();
            if self.fill(room, deadline).await? == 0 {
                return Err(StreamError::Eof {
                    at: "before the response head ended",
                });
            }
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// A partial head may not already hold an over-long line.
fn check_head_lines(partial: &[u8], limits: Limits) -> Result<(), StreamError> {
    if partial
        .split(|b| *b == b'\n')
        .any(|l| l.len() > limits.line)
    {
        return Err(StreamError::TooLarge {
            what: "header line",
            limit: limits.line,
        });
    }
    Ok(())
}

/// A parsed response head: the status code and the header fields, names
/// lowercased, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

impl Head {
    /// Every value of `name`, in order.
    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.headers
            .iter()
            .filter(move |(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The comma-separated tokens of every `name` field, lowercased.
    pub fn tokens(&self, name: &str) -> Vec<String> {
        self.values(name)
            .flat_map(|v| v.split(','))
            .map(|t| t.trim().to_ascii_lowercase())
            .filter(|t| !t.is_empty())
            .collect()
    }

    /// The one value of `name`; a repeated field is malformed.
    pub fn single<'a>(&'a self, name: &'a str) -> Result<Option<&'a str>, StreamError> {
        let mut values = self.values(name);
        let first = values.next();
        if values.next().is_some() {
            return Err(StreamError::Head(format!("{name} is repeated")));
        }
        Ok(first)
    }
}

/// Parse `bytes`, which end with the CRLF of the last header line (or of
/// the status line when there are no fields).
pub fn parse_head(bytes: &[u8], limits: Limits) -> Result<Head, StreamError> {
    let text = std::str::from_utf8(bytes).map_err(|_| StreamError::Utf8("response head"))?;
    let body = text
        .strip_suffix("\r\n")
        .ok_or_else(|| StreamError::Head("missing CRLF".into()))?;
    let mut lines = body.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut parts = status_line.splitn(3, ' ');
    let version = parts.next().unwrap_or_default();
    if version != "HTTP/1.1" {
        return Err(StreamError::Head(format!("not HTTP/1.1: {version:?}")));
    }
    let code = parts.next().unwrap_or_default();
    let status = match (code.len(), code.parse::<u16>()) {
        (3, Ok(s)) if (100..=599).contains(&s) => s,
        _ => return Err(StreamError::Head(format!("bad status code {code:?}"))),
    };
    let mut headers = Vec::new();
    for line in lines {
        if line.len() > limits.line {
            return Err(StreamError::TooLarge {
                what: "header line",
                limit: limits.line,
            });
        }
        if line.starts_with([' ', '\t']) || line.contains(['\r', '\n']) {
            return Err(StreamError::Head("folded or broken header line".into()));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| StreamError::Head(format!("header without a colon: {line:?}")))?;
        let token = |c: char| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c);
        if name.is_empty() || !name.chars().all(token) {
            return Err(StreamError::Head(format!("bad header name {name:?}")));
        }
        headers.push((
            name.to_ascii_lowercase(),
            value.trim_matches([' ', '\t']).to_string(),
        ));
    }
    Ok(Head { status, headers })
}

// ---- RFC 6455 ---------------------------------------------------------------

/// The `Sec-WebSocket-Accept` a server must answer `key` with (RFC 6455
/// §4.2.2): base64 of the SHA-1 of the key and the GUID, through `ring`.
pub fn accept_key(key: &str) -> String {
    let mut input = String::with_capacity(key.len() + WEBSOCKET_GUID.len());
    input.push_str(key);
    input.push_str(WEBSOCKET_GUID);
    let digest = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, input.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(digest.as_ref())
}

fn random<const N: usize>() -> Result<[u8; N], StreamError> {
    let mut out = [0u8; N];
    ring::rand::SystemRandom::new()
        .fill(&mut out)
        .map_err(|_| StreamError::Io {
            kind: ErrorKind::Other,
            message: "no system randomness".into(),
        })?;
    Ok(out)
}

/// A fresh handshake key: 16 random bytes, base64 (RFC 6455 §4.1).
pub fn fresh_key() -> Result<String, StreamError> {
    Ok(base64::engine::general_purpose::STANDARD.encode(random::<16>()?))
}

/// What one [`WebSocket::next`] call yields. Pings are answered inside
/// the client and never surface; a pong surfaces because a cited
/// ping/pong ordering may be a readiness signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
}

const OP_CONTINUATION: u8 = 0x0;
const OP_TEXT: u8 = 0x1;
const OP_BINARY: u8 = 0x2;
const OP_CLOSE: u8 = 0x8;
const OP_PING: u8 = 0x9;
const OP_PONG: u8 = 0xA;

/// A client WebSocket over any byte stream (a real `TcpStream` in the
/// driver; the same type over any socket in the tests).
pub struct WebSocket<S> {
    wire: Wire<S>,
    /// The opcode and payload of a fragmented message being assembled.
    assembling: Option<(u8, Vec<u8>)>,
    /// Client frames not yet written (pongs, the close reply, sends).
    out: Vec<u8>,
    /// The close code the peer sent, once it closed.
    closed: Option<Option<u16>>,
}

/// What one parsing pass produced.
enum Parsed {
    Message(WsMessage),
    /// A reply was queued; write it before parsing on.
    Replied,
    NeedMore,
}

/// One parsed frame header.
struct FrameHead {
    fin: bool,
    opcode: u8,
    len: usize,
    header: usize,
}

/// Parse a server frame header from the front of `buf`; `None` until the
/// whole header is present. Every length is checked before any payload is
/// awaited or allocated.
fn frame_head(buf: &[u8], limits: Limits) -> Result<Option<FrameHead>, StreamError> {
    if buf.len() < 2 {
        return Ok(None);
    }
    let (b0, b1) = (buf[0], buf[1]);
    if b0 & 0x70 != 0 {
        return Err(StreamError::Frame("reserved bits set without an extension"));
    }
    let fin = b0 & 0x80 != 0;
    let opcode = b0 & 0x0f;
    if !matches!(
        opcode,
        OP_CONTINUATION | OP_TEXT | OP_BINARY | OP_CLOSE | OP_PING | OP_PONG
    ) {
        return Err(StreamError::Frame("unknown opcode"));
    }
    if b1 & 0x80 != 0 {
        return Err(StreamError::Frame("a server frame is masked"));
    }
    let control = opcode >= OP_CLOSE;
    if control && !fin {
        return Err(StreamError::Frame("a fragmented control frame"));
    }
    let short = (b1 & 0x7f) as usize;
    if control && short > 125 {
        return Err(StreamError::Frame("a control frame over 125 bytes"));
    }
    let (len, header) = match short {
        126 => {
            if buf.len() < 4 {
                return Ok(None);
            }
            let len = u16::from_be_bytes([buf[2], buf[3]]) as u64;
            if len < 126 {
                return Err(StreamError::Frame("a non-minimal 16-bit length"));
            }
            (len, 4)
        }
        127 => {
            if buf.len() < 10 {
                return Ok(None);
            }
            let mut be = [0u8; 8];
            be.copy_from_slice(&buf[2..10]);
            let len = u64::from_be_bytes(be);
            if len >> 63 != 0 {
                return Err(StreamError::Frame("a 64-bit length with its top bit set"));
            }
            if len <= u16::MAX as u64 {
                return Err(StreamError::Frame("a non-minimal 64-bit length"));
            }
            (len, 10)
        }
        n => (n as u64, 2),
    };
    if len > limits.message as u64 {
        return Err(StreamError::TooLarge {
            what: "websocket frame",
            limit: limits.message,
        });
    }
    Ok(Some(FrameHead {
        fin,
        opcode,
        len: len as usize,
        header,
    }))
}

/// A masked client frame (RFC 6455 §5.3: every client frame is masked
/// with a fresh key).
fn client_frame(opcode: u8, payload: &[u8]) -> Result<Vec<u8>, StreamError> {
    let mask = random::<4>()?;
    let mut f = Vec::with_capacity(payload.len() + FRAME_HEADER_MAX);
    f.push(0x80 | opcode);
    match payload.len() {
        n if n < 126 => f.push(0x80 | n as u8),
        n if n <= u16::MAX as usize => {
            f.push(0x80 | 126);
            f.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            f.push(0x80 | 127);
            f.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    f.extend_from_slice(&mask);
    f.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    Ok(f)
}

/// Close codes a peer may send (RFC 6455 §7.4).
fn valid_close_code(code: u16) -> bool {
    matches!(code, 1000..=1003 | 1007..=1011 | 3000..=4999)
}

impl<S: AsyncRead + AsyncWrite + Unpin> WebSocket<S> {
    /// Open the WebSocket on `io` with a fresh random key: write the
    /// upgrade request for `path` on `host`, then require status 101, the
    /// `Upgrade: websocket` and `Connection: upgrade` tokens and the accept
    /// key derived from this key. Any extension or subprotocol answer is
    /// refused: the request offers none.
    pub async fn handshake(
        io: S,
        host: &str,
        path: &str,
        deadline: Instant,
        limits: Limits,
    ) -> Result<WebSocket<S>, StreamError> {
        let limits = limits.checked()?;
        let key = fresh_key()?;
        let mut wire = Wire {
            io,
            buf: Vec::new(),
            limits,
        };
        let mut request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )
        .into_bytes();
        wire.flush(&mut request, deadline).await?;
        let head = wire.read_head(deadline).await?;
        if head.status != 101 {
            return Err(StreamError::Status(head.status));
        }
        if !head.tokens("upgrade").iter().any(|t| t == "websocket") {
            return Err(StreamError::Handshake("no Upgrade: websocket".into()));
        }
        if !head.tokens("connection").iter().any(|t| t == "upgrade") {
            return Err(StreamError::Handshake("no Connection: upgrade".into()));
        }
        match head.single("sec-websocket-accept")? {
            Some(got) if got == accept_key(&key) => {}
            Some(got) => {
                return Err(StreamError::Handshake(format!(
                    "accept key {got:?} is not derived from the request's key"
                )));
            }
            None => return Err(StreamError::Handshake("no Sec-WebSocket-Accept".into())),
        }
        for name in ["sec-websocket-extensions", "sec-websocket-protocol"] {
            if let Some(v) = head.values(name).next() {
                return Err(StreamError::Negotiation(format!("{name}: {v}")));
            }
        }
        Ok(WebSocket {
            wire,
            assembling: None,
            out: Vec::new(),
            closed: None,
        })
    }

    /// The bytes this client holds: unparsed input, the message being
    /// assembled and unwritten output.
    pub fn buffered(&self) -> usize {
        self.wire.buf.len() + self.assembling.as_ref().map_or(0, |(_, p)| p.len()) + self.out.len()
    }

    /// Send a ping; its pong surfaces from [`WebSocket::next`].
    pub async fn ping(&mut self, payload: &[u8], deadline: Instant) -> Result<(), StreamError> {
        if payload.len() > 125 {
            return Err(StreamError::Frame("a control frame over 125 bytes"));
        }
        self.queue(OP_PING, payload)?;
        self.flush(deadline).await
    }

    /// Send one text message (a cited registration envelope, where a
    /// channel needs one).
    pub async fn send_text(&mut self, text: &str, deadline: Instant) -> Result<(), StreamError> {
        if text.len() > self.wire.limits.message {
            return Err(StreamError::TooLarge {
                what: "websocket message",
                limit: self.wire.limits.message,
            });
        }
        self.queue(OP_TEXT, text.as_bytes())?;
        self.flush(deadline).await
    }

    fn queue(&mut self, opcode: u8, payload: &[u8]) -> Result<(), StreamError> {
        if self.closed.is_some() {
            return Err(StreamError::Closed);
        }
        let frame = client_frame(opcode, payload)?;
        if self.buffered() + frame.len() > self.wire.limits.buffer {
            return Err(StreamError::TooLarge {
                what: "transport buffering",
                limit: self.wire.limits.buffer,
            });
        }
        self.out.extend_from_slice(&frame);
        Ok(())
    }

    async fn flush(&mut self, deadline: Instant) -> Result<(), StreamError> {
        let mut out = std::mem::take(&mut self.out);
        let result = self.wire.flush(&mut out, deadline).await;
        // What was not written stays queued, ahead of anything newer.
        out.append(&mut self.out);
        self.out = out;
        result
    }

    /// The next complete data message or pong, before `deadline`. Pings
    /// are answered; a close is answered and ends the socket with
    /// [`StreamError::Closed`].
    pub async fn next(&mut self, deadline: Instant) -> Result<WsMessage, StreamError> {
        loop {
            if !self.out.is_empty() {
                self.flush(deadline).await?;
            }
            if self.closed.is_some() {
                return Err(StreamError::Closed);
            }
            match self.parse_one()? {
                Parsed::Message(message) => return Ok(message),
                Parsed::Replied => continue,
                Parsed::NeedMore => {}
            }
            let room = self.wire.limits.buffer.saturating_sub(self.buffered());
            if self.wire.fill(room, deadline).await? == 0 {
                let at = if self.wire.buf.is_empty() && self.assembling.is_none() {
                    "between messages"
                } else {
                    "inside a frame or message"
                };
                return Err(StreamError::Eof { at });
            }
        }
    }

    /// Consume complete frames from the front of the buffer until one
    /// yields a message or queues a reply (a pong or the close answer, which
    /// [`WebSocket::next`] writes before it parses on), or more bytes are
    /// needed.
    fn parse_one(&mut self) -> Result<Parsed, StreamError> {
        let limits = self.wire.limits;
        while let Some(h) = frame_head(&self.wire.buf, limits)? {
            if self.wire.buf.len() < h.header + h.len {
                return Ok(Parsed::NeedMore);
            }
            let payload: Vec<u8> = self.wire.buf[h.header..h.header + h.len].to_vec();
            self.wire.buf.drain(..h.header + h.len);
            match h.opcode {
                OP_PING => {
                    self.queue(OP_PONG, &payload)?;
                    return Ok(Parsed::Replied);
                }
                OP_PONG => return Ok(Parsed::Message(WsMessage::Pong(payload))),
                OP_CLOSE => {
                    let code = match payload.len() {
                        0 => None,
                        1 => return Err(StreamError::Frame("a one-byte close payload")),
                        _ => {
                            let code = u16::from_be_bytes([payload[0], payload[1]]);
                            if !valid_close_code(code) {
                                return Err(StreamError::Frame("an invalid close code"));
                            }
                            std::str::from_utf8(&payload[2..])
                                .map_err(|_| StreamError::Utf8("close reason"))?;
                            Some(code)
                        }
                    };
                    let reply = code.map(u16::to_be_bytes);
                    self.queue(OP_CLOSE, reply.as_ref().map_or(&[][..], |c| &c[..]))?;
                    self.closed = Some(code);
                    return Ok(Parsed::Replied);
                }
                OP_CONTINUATION => {
                    let Some((_, assembled)) = self.assembling.as_mut() else {
                        return Err(StreamError::Frame("a continuation with no message"));
                    };
                    if assembled.len() + payload.len() > limits.message {
                        return Err(StreamError::TooLarge {
                            what: "websocket message",
                            limit: limits.message,
                        });
                    }
                    assembled.extend_from_slice(&payload);
                    if h.fin {
                        let (opcode, whole) = self.assembling.take().unwrap_or_default();
                        return Self::complete(opcode, whole).map(Parsed::Message);
                    }
                }
                opcode => {
                    if self.assembling.is_some() {
                        return Err(StreamError::Frame("a new message inside a fragmented one"));
                    }
                    if h.fin {
                        return Self::complete(opcode, payload).map(Parsed::Message);
                    }
                    self.assembling = Some((opcode, payload));
                }
            }
        }
        Ok(Parsed::NeedMore)
    }

    fn complete(opcode: u8, payload: Vec<u8>) -> Result<WsMessage, StreamError> {
        if opcode == OP_TEXT {
            String::from_utf8(payload)
                .map(WsMessage::Text)
                .map_err(|_| StreamError::Utf8("text message"))
        } else {
            Ok(WsMessage::Binary(payload))
        }
    }

    /// The close code the peer sent, once it closed.
    pub fn close_code(&self) -> Option<Option<u16>> {
        self.closed
    }
}

// ---- server-sent events -----------------------------------------------------

/// One dispatched event. `event` is the type (`message` when unnamed).
/// The `id` and `retry` fields imply neither replay nor readiness, and the
/// reader does not act on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
    pub id: Option<String>,
}

/// What one [`Sse::next`] call yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseItem {
    Event(SseEvent),
    /// The body ended cleanly; an incomplete event at the end was
    /// discarded, as the event-stream specification requires.
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Body {
    /// `Content-Length`: this many body bytes remain.
    Length(u64),
    /// Read until the connection closes.
    Close,
    /// Chunked transfer coding, in this state.
    Chunked(Chunk),
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chunk {
    Size,
    Data(u64),
    DataEnd,
    Trailers,
}

/// A server-sent-events reader over any byte stream.
pub struct Sse<S> {
    wire: Wire<S>,
    body: Body,
    /// Decoded body bytes not yet split into lines.
    text: Vec<u8>,
    /// A CR ended the last line; an LF right after it is the same break.
    after_cr: bool,
    /// Still at the start of the stream (a leading BOM is dropped).
    at_start: bool,
    data: String,
    has_data: bool,
    event: String,
    id: Option<String>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Sse<S> {
    /// Request `path` on `host` as an event stream and require status 200,
    /// the `text/event-stream` media type, no content coding, and transfer
    /// framing this reader honours: chunked, a content length, or close.
    pub async fn open(
        io: S,
        host: &str,
        path: &str,
        deadline: Instant,
        limits: Limits,
    ) -> Result<Sse<S>, StreamError> {
        let limits = limits.checked()?;
        let mut wire = Wire {
            io,
            buf: Vec::new(),
            limits,
        };
        let mut request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nAccept: text/event-stream\r\n\
             Cache-Control: no-cache\r\n\r\n"
        )
        .into_bytes();
        wire.flush(&mut request, deadline).await?;
        let head = wire.read_head(deadline).await?;
        let body = sse_body(&head)?;
        Ok(Sse {
            wire,
            body,
            text: Vec::new(),
            after_cr: false,
            at_start: true,
            data: String::new(),
            has_data: false,
            event: String::new(),
            id: None,
        })
    }

    /// The bytes this reader holds: undecoded input, the partial line and
    /// the event being built.
    pub fn buffered(&self) -> usize {
        self.wire.buf.len() + self.text.len() + self.data.len() + self.event.len()
    }

    /// The next dispatched event, or [`SseItem::Ended`] once the body
    /// ended, before `deadline`. Comments, empty dispatches and an
    /// unterminated event at the end are not events.
    pub async fn next(&mut self, deadline: Instant) -> Result<SseItem, StreamError> {
        loop {
            while let Some(line) = self.take_line()? {
                if let Some(event) = self.field(&line)? {
                    return Ok(SseItem::Event(event));
                }
            }
            if self.decode()? {
                continue;
            }
            if self.body == Body::Done {
                return Ok(SseItem::Ended);
            }
            let room = self.wire.limits.buffer.saturating_sub(self.buffered());
            if self.wire.fill(room, deadline).await? == 0 {
                if self.body == Body::Close {
                    self.body = Body::Done;
                    continue;
                }
                return Err(StreamError::Eof {
                    at: "inside the event-stream body",
                });
            }
        }
    }

    /// Advance the body framing over the wire buffer, moving decoded bytes
    /// into `text`, at most a line's worth beyond what it holds. `true`
    /// when anything moved or the framing advanced.
    fn decode(&mut self) -> Result<bool, StreamError> {
        let room = (self.wire.limits.line + 1).saturating_sub(self.text.len()) as u64;
        match self.body {
            Body::Done => Ok(false),
            Body::Close => Ok(self.take_body(room) > 0),
            Body::Length(0) => {
                self.body = Body::Done;
                Ok(true)
            }
            Body::Length(left) => {
                let n = self.take_body(room.min(left));
                self.body = Body::Length(left - n);
                Ok(n > 0)
            }
            Body::Chunked(Chunk::Size) => {
                let Some(line) = self.wire_line()? else {
                    return Ok(false);
                };
                let size = line.split(';').next().unwrap_or_default().trim_end();
                if size.is_empty() || !size.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return Err(StreamError::Framing(format!("bad chunk size {size:?}")));
                }
                let size = u64::from_str_radix(size, 16)
                    .map_err(|_| StreamError::Framing("chunk size overflows".into()))?;
                self.body = Body::Chunked(if size == 0 {
                    Chunk::Trailers
                } else {
                    Chunk::Data(size)
                });
                Ok(true)
            }
            Body::Chunked(Chunk::Data(left)) => {
                let n = self.take_body(room.min(left));
                self.body = Body::Chunked(if n == left {
                    Chunk::DataEnd
                } else {
                    Chunk::Data(left - n)
                });
                Ok(n > 0)
            }
            Body::Chunked(Chunk::DataEnd) => {
                if self.wire.buf.len() < 2 {
                    return Ok(false);
                }
                if &self.wire.buf[..2] != b"\r\n" {
                    return Err(StreamError::Framing("chunk data not ended by CRLF".into()));
                }
                self.wire.buf.drain(..2);
                self.body = Body::Chunked(Chunk::Size);
                Ok(true)
            }
            Body::Chunked(Chunk::Trailers) => {
                let Some(line) = self.wire_line()? else {
                    return Ok(false);
                };
                if line.is_empty() {
                    self.body = Body::Done;
                }
                Ok(true)
            }
        }
    }

    /// Up to `max` body bytes from the wire buffer into `text`.
    fn take_body(&mut self, max: u64) -> u64 {
        let n = (self.wire.buf.len() as u64).min(max) as usize;
        self.text.extend(self.wire.buf.drain(..n));
        n as u64
    }

    /// One CRLF-terminated framing line (a chunk size or a trailer) from
    /// the wire buffer.
    fn wire_line(&mut self) -> Result<Option<String>, StreamError> {
        let limit = self.wire.limits.line;
        let too_long = StreamError::TooLarge {
            what: "chunk framing line",
            limit,
        };
        let Some(end) = find(&self.wire.buf, b"\r\n") else {
            if self.wire.buf.len() > limit {
                return Err(too_long);
            }
            return Ok(None);
        };
        if end > limit {
            return Err(too_long);
        }
        let line = String::from_utf8(self.wire.buf[..end].to_vec())
            .map_err(|_| StreamError::Utf8("chunk framing line"))?;
        self.wire.buf.drain(..end + 2);
        Ok(Some(line))
    }

    /// One complete event-stream line from `text`. CRLF, LF or CR ends a
    /// line; a CR at the end of the bytes held ends it too, and an LF
    /// that arrives after it later is skipped as the same break.
    fn take_line(&mut self) -> Result<Option<String>, StreamError> {
        if self.after_cr && !self.text.is_empty() {
            if self.text[0] == b'\n' {
                self.text.remove(0);
            }
            self.after_cr = false;
        }
        if self.at_start {
            const BOM: &[u8] = b"\xEF\xBB\xBF";
            let held = self.text.len().min(3);
            if self.text[..held] != BOM[..held] {
                self.at_start = false;
            } else if held == 3 {
                self.text.drain(..3);
                self.at_start = false;
            } else {
                // Too few bytes to tell (an incomplete line either way).
                return Ok(None);
            }
        }
        let limit = self.wire.limits.line;
        let too_long = StreamError::TooLarge {
            what: "event-stream line",
            limit,
        };
        let Some(end) = self.text.iter().position(|b| *b == b'\n' || *b == b'\r') else {
            if self.text.len() > limit {
                return Err(too_long);
            }
            return Ok(None);
        };
        if end > limit {
            return Err(too_long);
        }
        let line = String::from_utf8(self.text[..end].to_vec())
            .map_err(|_| StreamError::Utf8("event-stream line"))?;
        self.after_cr = self.text[end] == b'\r';
        self.text.drain(..end + 1);
        Ok(Some(line))
    }

    /// Apply one line (the WHATWG event-stream interpretation); an empty
    /// line dispatches the event being built, if it has data.
    fn field(&mut self, line: &str) -> Result<Option<SseEvent>, StreamError> {
        if line.is_empty() {
            let event = std::mem::take(&mut self.event);
            let mut data = std::mem::take(&mut self.data);
            if !std::mem::take(&mut self.has_data) {
                return Ok(None);
            }
            data.pop(); // the last data line's LF
            return Ok(Some(SseEvent {
                event: if event.is_empty() {
                    "message".into()
                } else {
                    event
                },
                data,
                id: self.id.clone(),
            }));
        }
        if line.starts_with(':') {
            return Ok(None);
        }
        let (name, value) = match line.split_once(':') {
            Some((n, v)) => (n, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match name {
            "data" => {
                let limit = self.wire.limits.message;
                if self.data.len() + value.len() + 1 > limit {
                    return Err(StreamError::TooLarge {
                        what: "event data",
                        limit,
                    });
                }
                self.data.push_str(value);
                self.data.push('\n');
                self.has_data = true;
            }
            "event" => self.event = value.to_string(),
            "id" if !value.contains('\0') => self.id = Some(value.to_string()),
            _ => {}
        }
        Ok(None)
    }
}

/// The body framing of an event-stream response, or why it is refused.
fn sse_body(head: &Head) -> Result<Body, StreamError> {
    if head.status != 200 {
        return Err(StreamError::Status(head.status));
    }
    let media = head
        .single("content-type")?
        .ok_or_else(|| StreamError::ContentType("no Content-Type".into()))?;
    let essence = media.split(';').next().unwrap_or_default().trim();
    if !essence.eq_ignore_ascii_case("text/event-stream") {
        return Err(StreamError::ContentType(media.to_string()));
    }
    if let Some(coding) = head
        .tokens("content-encoding")
        .into_iter()
        .find(|c| c != "identity")
    {
        return Err(StreamError::ContentType(format!("content coding {coding}")));
    }
    let transfer = head.tokens("transfer-encoding");
    let lengths: Vec<&str> = head.values("content-length").collect();
    match (transfer.as_slice(), lengths.as_slice()) {
        ([], []) => Ok(Body::Close),
        ([], [len]) => {
            if len.is_empty() || !len.bytes().all(|b| b.is_ascii_digit()) {
                return Err(StreamError::Framing(format!("bad Content-Length {len:?}")));
            }
            len.parse::<u64>()
                .map(Body::Length)
                .map_err(|_| StreamError::Framing("Content-Length overflows".into()))
        }
        ([], _) => Err(StreamError::Framing("repeated Content-Length".into())),
        ([only], []) if only == "chunked" => Ok(Body::Chunked(Chunk::Size)),
        (_, []) => Err(StreamError::Framing(format!(
            "unsupported transfer coding {}",
            transfer.join(", ")
        ))),
        (_, _) => Err(StreamError::Framing(
            "both Transfer-Encoding and Content-Length".into(),
        )),
    }
}
