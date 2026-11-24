//! The trial-side owners of value capture (decision 0119 §4).
//!
//! * A transparent loopback forwarder stands in front of Swarmail's `/mcp`
//!   for the agents only (the runner's own provisioning calls go straight to
//!   Swarmail). Every request is forwarded with its method, headers and body
//!   unchanged, except that `Host` names Swarmail and an `Origin` that is a
//!   loopback spelling of the forwarder's port is respelled for Swarmail's
//!   port, so Swarmail's own guard (decision 0043) rules exactly as it would
//!   for a direct request. Replies, errors and statuses come back unchanged.
//!   Each `tools/call` in a body is observed before it is forwarded, and its
//!   in-flight span ends once Swarmail has answered. The forwarder is as
//!   reachable by the agent as Swarmail is, so it is bounded: at most
//!   [`CONNECTION_LIMIT`] connections open at once (a further one is closed
//!   unserved), each holding one HTTP/1.1 request of at most the reply
//!   bound, and at most [`REQUEST_LIMIT`] requests observed per trial (past
//!   it requests are passed through unread). Exceeding either withholds the
//!   trial's values as `trial_too_large`.
//! * A private Unix socket in the runner's runtime directory receives the
//!   `eval_http` helper's observations: `start` before a call is
//!   dispatched, `end` after its reply, each acknowledged so the order the
//!   capture sees is the order the agent's calls happened in.
//!
//! Observation never alters or retries a request; a capture that is lost or
//! ambiguous only withholds the trial's values. (The helper waits for each
//! frame's acknowledgement, bounded at two seconds, so its order is the
//! order of the calls; a stalled runner costs at most that once, after
//! which the helper drops its observer and the trial's values are absent.)
//!
//! The socket's trust boundary. Its path reaches the helper through the
//! agent's own MCP configuration, so an unconfined same-uid agent can
//! connect to it too; nothing a CLI launches can hold a secret the agent
//! cannot read. What a connection can do is therefore bounded, not
//! trusted: a frame can only report an `eval_http` call (the tool set is
//! fixed by the runner: [`Server::Fixture`] admits that one name and the
//! Swarmail catalogue is never reachable through the socket); every
//! published value must equal, byte for byte, the digest the engine's
//! journal recorded for the agent's real call, so a forged or extra frame
//! leaves the trial `binding_mismatch`, never a forged companion; every
//! value is redacted whatever its source; scoring never reads any of it.
//! A connection holds at most one open call, and a trial accepts at most
//! [`HELPER_LIMIT`] connections, so the socket cannot grow the runner's
//! memory. The forwarder is bounded, not free: at most [`CONNECTION_LIMIT`]
//! connections at once, each holding one request body of at most
//! [`REQUEST_BODY_LIMIT`] (2 MiB) and one reply of at most the MCP reply
//! bound (16 MiB), so at most 32 x 18 MiB (576 MiB) for the trial's
//! length, and [`REQUEST_LIMIT`] requests observed. Within those bounds a
//! hostile agent can withhold its own trial's evidence (which it can
//! already do by calling two tools at once), have its own excess
//! connections or oversized bodies refused, and hold that much runner
//! memory while its trial runs; it cannot reach another trial.

use crate::toolvalues::{Capture, Refusal, Server, Started};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

/// The largest reply forwarded: the MCP client's own reply bound.
const BODY_LIMIT: usize = crate::mcp::MAX_REPLY_BYTES as usize;
/// The largest request body forwarded: Swarmail's own `POST /mcp` limit
/// (`MCP_BODY_LIMIT` in the root `src/api.rs`, 2 MiB), mirrored here
/// because the harness is a separate crate. A body over it is refused
/// (413) as soon as its first byte over the limit arrives, never read to
/// its end, exactly as Swarmail itself would refuse it.
pub const REQUEST_BODY_LIMIT: usize = 2_097_152;
/// The largest observation frame the socket reads: the helper's line bound
/// plus its framing.
const FRAME_LIMIT: u64 = 3 * crate::fixture_http::INPUT_LIMIT as u64;
/// The most helper connections one trial's socket accepts: one per agent
/// process, with room for an isolation task's identities. A further
/// connection withholds the trial and closes the socket.
pub const HELPER_LIMIT: usize = 16;
/// The most forwarder connections open at once. Each serves one HTTP/1.1
/// request at a time, so this bounds what the forwarder buffers to this
/// many request bodies of at most [`REQUEST_BODY_LIMIT`] and replies of at
/// most `BODY_LIMIT` each. A further connection is closed unserved and
/// withholds the trial's values.
pub const CONNECTION_LIMIT: usize = 32;
/// The most requests the forwarder observes in one trial: room for every
/// call the value record can keep ([`crate::toolvalues::CALL_LIMIT`]) and
/// each session's own initialize, notification and list requests. Past it
/// the trial's values are withheld and requests are forwarded unread, so
/// the trial runs on as it would without capture.
pub const REQUEST_LIMIT: usize = 2 * crate::toolvalues::CALL_LIMIT;

/// A running capture: the forwarder's URL for the agents, the socket path
/// for the helper, and the tasks to stop once the agents have exited.
pub struct Running {
    pub capture: Capture,
    pub mcp_url: String,
    pub socket: PathBuf,
    tasks: Vec<JoinHandle<()>>,
}

impl Running {
    /// Stop serving. Called after every agent group has been reaped, so no
    /// request of the trial is still to come.
    pub fn stop(self) -> Capture {
        self.capture.clone()
    }
}

impl Drop for Running {
    // Also on an early return before any agent ran: nothing outlives the body.
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
    }
}

struct Forward {
    http: reqwest::Client,
    upstream: String,
    upstream_addr: SocketAddr,
    own_port: u16,
    capture: Capture,
    requests: AtomicUsize,
    /// The client matrix's invocation log (issue #392), when this forwarder
    /// observes one client launch; `None` for a trial's capture.
    log: Option<Arc<ExchangeLog>>,
}

/// Read the Swarmail catalogue the capture admits, then start both owners.
pub async fn start(
    mcp_url: &str,
    upstream_addr: SocketAddr,
    runtime: &Path,
) -> Result<Running, String> {
    let mut client = crate::mcp::McpClient::connect(mcp_url).await?;
    let listed = client.rpc("tools/list", serde_json::json!({})).await?;
    let catalogue: BTreeSet<String> = listed["tools"]
        .as_array()
        .ok_or("tool_catalogue_unavailable")?
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect();
    if catalogue.is_empty() {
        return Err("tool_catalogue_unavailable".into());
    }
    let capture = Capture::new(catalogue);
    let (own, server) = forwarder(mcp_url, upstream_addr, capture.clone(), None).await?;
    // From here an early return stops the forwarder it would leave behind.
    let stop = |e: &str| {
        server.abort();
        e.to_string()
    };
    let socket = runtime.join("tool-capture.sock");
    let unix = tokio::net::UnixListener::bind(&socket).map_err(|_| stop("capture_socket_bind"))?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| stop("capture_socket_bind"))?;
    }
    let accept = tokio::spawn(accept_helpers(unix, capture.clone()));
    Ok(Running {
        capture,
        mcp_url: format!("http://{own}/mcp"),
        socket,
        tasks: vec![server, accept],
    })
}

/// Serve the loopback forwarder in front of `upstream` (Swarmail's `/mcp`)
/// for one trial's `capture`, within [`CONNECTION_LIMIT`] and
/// [`REQUEST_LIMIT`].
async fn forwarder(
    upstream: &str,
    upstream_addr: SocketAddr,
    capture: Capture,
    log: Option<Arc<ExchangeLog>>,
) -> Result<(SocketAddr, JoinHandle<()>), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "capture_forwarder_bind")?;
    let own = listener
        .local_addr()
        .map_err(|_| "capture_forwarder_bind")?;
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "capture_forwarder_client")?;
    let state = Arc::new(Forward {
        http,
        upstream: upstream.to_string(),
        upstream_addr,
        own_port: own.port(),
        capture: capture.clone(),
        requests: AtomicUsize::new(0),
        log,
    });
    let app = axum::Router::new()
        .route("/mcp", axum::routing::any(forward))
        .layer(DefaultBodyLimit::max(REQUEST_BODY_LIMIT))
        .with_state(state);
    let bounded = Bounded {
        inner: listener,
        slots: Arc::new(Semaphore::new(CONNECTION_LIMIT)),
        capture,
    };
    let server = tokio::spawn(async move {
        let _ = axum::serve(bounded, app).await;
    });
    Ok((own, server))
}

/// The forwarder's listener: a connection is served only while it holds
/// one of [`CONNECTION_LIMIT`] slots, released when the connection ends.
/// One accepted with no slot free is closed unserved, after the trial's
/// values are withheld.
struct Bounded {
    inner: tokio::net::TcpListener,
    slots: Arc<Semaphore>,
    capture: Capture,
}

impl axum::serve::Listener for Bounded {
    type Io = Held;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Held, SocketAddr) {
        loop {
            let (io, addr) = axum::serve::Listener::accept(&mut self.inner).await;
            match self.slots.clone().try_acquire_owned() {
                Ok(slot) => return (Held { _slot: slot, io }, addr),
                Err(_) => {
                    self.capture.refuse(Refusal::TrialTooLarge);
                    drop(io);
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// A served connection and the slot it holds.
struct Held {
    _slot: OwnedSemaphorePermit,
    io: tokio::net::TcpStream,
}

impl AsyncRead for Held {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.io).poll_read(cx, buf)
    }
}

impl AsyncWrite for Held {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.io).poll_write_vectored(cx, bufs)
    }
    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

/// Serve up to [`HELPER_LIMIT`] helper connections; one more latches
/// `binding_mismatch` and closes the socket.
async fn accept_helpers(unix: tokio::net::UnixListener, capture: Capture) {
    let mut served = 0;
    while let Ok((stream, _)) = unix.accept().await {
        if served == HELPER_LIMIT {
            capture.refuse(Refusal::BindingMismatch);
            return;
        }
        served += 1;
        tokio::spawn(helper_connection(stream, capture.clone()));
    }
}

/// One helper connection: newline-delimited JSON frames, each acknowledged
/// with one byte once the capture has recorded it. The helper dispatches
/// one call at a time, so a second `start` before an `end` is not its
/// order: the trial is withheld and the connection closed.
async fn helper_connection(stream: tokio::net::UnixStream, capture: Capture) {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut open: Option<Started> = None;
    loop {
        let mut line = Vec::new();
        match (&mut reader)
            .take(FRAME_LIMIT)
            .read_until(b'\n', &mut line)
            .await
        {
            Ok(0) => break,
            Ok(_) if line.last() == Some(&b'\n') => {}
            _ => {
                capture.refuse(Refusal::InvalidArgumentEncoding);
                break;
            }
        }
        let frame: serde_json::Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) => {
                capture.refuse(Refusal::InvalidArgumentEncoding);
                break;
            }
        };
        match (
            frame["op"].as_str(),
            frame["tool"].as_str(),
            frame["arguments"].as_str(),
        ) {
            (Some("start"), _, _) if open.is_some() => {
                capture.refuse(Refusal::BindingMismatch);
                break;
            }
            (Some("start"), Some(tool), Some(arguments)) => {
                open = Some(capture.observe(Server::Fixture, tool, arguments.as_bytes()))
            }
            (Some("end"), _, _) if open.is_some() => open = None,
            _ => {
                capture.refuse(Refusal::InvalidArgumentEncoding);
                break;
            }
        }
        if write.write_all(b"k").await.is_err() {
            break;
        }
    }
}

fn hop_by_hop(name: &header::HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
    )
}

/// The `Origin` Swarmail sees: a loopback spelling of the forwarder's own
/// origin becomes the same spelling of Swarmail's; anything else is passed
/// through for Swarmail's guard to refuse as it would.
fn respell_origin(value: &HeaderValue, own_port: u16, upstream: SocketAddr) -> HeaderValue {
    let Ok(text) = value.to_str() else {
        return value.clone();
    };
    for host in ["127.0.0.1", "localhost", "[::1]"] {
        if text == format!("http://{host}:{own_port}") {
            return HeaderValue::from_str(&format!("http://{host}:{}", upstream.port()))
                .unwrap_or_else(|_| value.clone());
        }
    }
    value.clone()
}

async fn forward(
    State(st): State<Arc<Forward>>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Past the request cap the trial's values are withheld and the body is
    // forwarded unread: the trial runs on exactly as without capture.
    let observed = if st.requests.fetch_add(1, Ordering::Relaxed) >= REQUEST_LIMIT {
        st.capture.refuse(Refusal::TrialTooLarge);
        Vec::new()
    } else if method == Method::POST {
        st.capture.observe_mcp_body(&body)
    } else {
        Vec::new()
    };
    let logged = st
        .log
        .as_ref()
        .map(|_| (method == Method::POST, body.clone()));
    let mut req = st.http.request(method, &st.upstream);
    for (name, value) in &headers {
        if hop_by_hop(name) {
            continue;
        }
        let value = if name == header::ORIGIN {
            respell_origin(value, st.own_port, st.upstream_addr)
        } else {
            value.clone()
        };
        req = req.header(name, value);
    }
    let reply = async {
        let mut resp = req.body(body).send().await.ok()?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let mut bytes = Vec::new();
        while let Some(chunk) = resp.chunk().await.ok()? {
            if chunk.len() > BODY_LIMIT - bytes.len() {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        Some((status, headers, bytes))
    }
    .await;
    drop(observed);
    if let (Some(log), Some((post, request))) = (&st.log, logged) {
        log.record(
            post,
            &request,
            reply.as_ref().map(|(s, _, b)| (s.as_u16(), &b[..])),
        );
    }
    let Some((status, upstream_headers, bytes)) = reply else {
        return StatusCode::BAD_GATEWAY.into_response();
    };
    let mut out = Response::new(Body::from(bytes));
    *out.status_mut() = status;
    for (name, value) in &upstream_headers {
        if !hop_by_hop(name) {
            out.headers_mut().append(name, value.clone());
        }
    }
    out
}

/// The most exchanges one client launch may make through its observer: the
/// matrix needs an initialize, a notification and one method, with room
/// for a client's own lifecycle traffic. Past it the launch is unattributable.
pub const EXCHANGE_LIMIT: usize = 64;
/// The most request and reply bytes one launch's observer keeps in memory.
pub const EXCHANGE_BYTES: usize = 8 << 20;

/// One request the observed client sent and the reply it received, paired
/// where the forwarder already holds both. `reply` is `None` when Swarmail
/// gave none (the client then saw `502`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    /// Whether the request was a `POST` (the only kind carrying JSON-RPC).
    pub post: bool,
    pub request: Vec<u8>,
    pub reply: Option<(u16, Vec<u8>)>,
}

/// One launch's exchanges, bounded by [`EXCHANGE_LIMIT`] and
/// [`EXCHANGE_BYTES`]; a record past either only marks the log overflowed.
#[derive(Default)]
pub struct ExchangeLog {
    state: std::sync::Mutex<LogState>,
}

#[derive(Default)]
struct LogState {
    exchanges: Vec<Exchange>,
    bytes: usize,
    overflowed: bool,
    sealed: bool,
}

impl ExchangeLog {
    fn record(&self, post: bool, request: &[u8], reply: Option<(u16, &[u8])>) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.sealed {
            return;
        }
        let size = request.len() + reply.map_or(0, |(_, b)| b.len());
        if state.exchanges.len() == EXCHANGE_LIMIT || size > EXCHANGE_BYTES - state.bytes {
            state.overflowed = true;
            return;
        }
        state.bytes += size;
        state.exchanges.push(Exchange {
            post,
            request: request.to_vec(),
            reply: reply.map(|(s, b)| (s, b.to_vec())),
        });
    }

    /// Close the log: what was recorded, and whether a record was refused.
    fn seal(&self) -> (Vec<Exchange>, bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.sealed = true;
        (std::mem::take(&mut state.exchanges), state.overflowed)
    }
}

/// A forwarder observing one client launch: its own loopback endpoint in
/// front of Swarmail's `/mcp`, so what it records belongs to that launch.
pub struct Observer {
    pub mcp_url: String,
    log: Arc<ExchangeLog>,
    server: JoinHandle<()>,
}

impl Observer {
    /// Stop serving and return the launch's exchanges in arrival order and
    /// whether the log overflowed. Called after the launch was reaped.
    pub fn seal(self) -> (Vec<Exchange>, bool) {
        self.server.abort();
        self.log.seal()
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        self.server.abort();
    }
}

/// Start an observer in front of `mcp_url` (Swarmail's `/mcp` at
/// `upstream_addr`), forwarding exactly as a trial's capture does.
pub async fn observe(mcp_url: &str, upstream_addr: SocketAddr) -> Result<Observer, String> {
    let log = Arc::new(ExchangeLog::default());
    // The trial's value capture rides along unread: the matrix reads the log.
    let capture = Capture::new(BTreeSet::new());
    let (own, server) = forwarder(mcp_url, upstream_addr, capture, Some(log.clone())).await?;
    Ok(Observer {
        mcp_url: format!("http://{own}/mcp"),
        log,
        server,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolvalues::{CaptureOutcome, Registry};

    fn reason(capture: &Capture) -> Option<Refusal> {
        let mut registry = Registry::new();
        registry.register("canary-literal");
        match capture
            .seal(&format!("{:032x}", 1), &registry, b"{}")
            .outcome
        {
            CaptureOutcome::Refused { reason } => Some(reason),
            CaptureOutcome::Complete { .. } => None,
        }
    }

    #[test]
    fn a_loopback_origin_of_the_forwarder_names_swarmail() {
        let upstream: SocketAddr = "127.0.0.1:4100".parse().unwrap();
        for host in ["127.0.0.1", "localhost", "[::1]"] {
            let own = HeaderValue::from_str(&format!("http://{host}:5200")).unwrap();
            assert_eq!(
                respell_origin(&own, 5200, upstream),
                HeaderValue::from_str(&format!("http://{host}:4100")).unwrap()
            );
        }
        // Anything else passes through for Swarmail's own guard to rule on.
        for other in [
            &b"http://127.0.0.1:5201"[..],
            b"http://evil.example:5200",
            b"https://127.0.0.1:5200",
            b"http://127.0.0.1:5200/",
            b"null",
            b"http://caf\xc3\xa9:5200",
        ] {
            let value = HeaderValue::from_bytes(other).unwrap();
            assert_eq!(respell_origin(&value, 5200, upstream), value);
        }
    }

    async fn frames(capture: &Capture, lines: &[&str]) -> usize {
        let (ours, theirs) = tokio::net::UnixStream::pair().unwrap();
        let served = tokio::spawn(helper_connection(theirs, capture.clone()));
        let (mut read, mut write) = ours.into_split();
        let mut acks = 0;
        for line in lines {
            if write.write_all(line.as_bytes()).await.is_err() {
                break;
            }
            if !line.ends_with('\n') {
                write.shutdown().await.unwrap();
            }
            let mut ack = [0u8; 1];
            match tokio::time::timeout(std::time::Duration::from_secs(10), read.read(&mut ack))
                .await
                .expect("the connection answers or closes")
            {
                Ok(1) => acks += 1,
                _ => break,
            }
        }
        drop(write);
        tokio::time::timeout(std::time::Duration::from_secs(10), served)
            .await
            .expect("the connection ends")
            .unwrap();
        acks
    }

    #[tokio::test]
    async fn a_helper_connection_holds_one_call_at_a_time() {
        let start = "{\"op\":\"start\",\"tool\":\"eval_http\",\"arguments\":\"{}\"}\n";
        let end = "{\"op\":\"end\"}\n";
        // The helper's own order: start, end, start, end.
        let c = Capture::new(BTreeSet::from(["t".to_string()]));
        assert_eq!(frames(&c, &[start, end, start, end]).await, 4);
        assert_eq!(reason(&c), None);
        // A second start while one is open withholds the trial and closes.
        let c = Capture::new(BTreeSet::from(["t".to_string()]));
        assert_eq!(frames(&c, &[start, start, end]).await, 1);
        assert_eq!(reason(&c), Some(Refusal::BindingMismatch));
        // The socket cannot declare another tool: only eval_http is owned.
        let c = Capture::new(BTreeSet::from(["t".to_string()]));
        let other = "{\"op\":\"start\",\"tool\":\"t\",\"arguments\":\"{}\"}\n";
        assert_eq!(frames(&c, &[other, end]).await, 2);
        assert_eq!(reason(&c), Some(Refusal::UnknownTool));
        // An end without a start, an unknown op, a malformed frame and an
        // unterminated one are refused as unreadable.
        for bad in [end, "{\"op\":\"unreadable\"}\n", "nope\n", "{\"op\":"] {
            let c = Capture::new(BTreeSet::from(["t".to_string()]));
            assert_eq!(frames(&c, &[bad]).await, 0, "{bad}");
            assert_eq!(reason(&c), Some(Refusal::InvalidArgumentEncoding), "{bad}");
        }
    }

    /// A stand-in for Swarmail's `/mcp` that answers `ok` and counts.
    async fn upstream() -> (SocketAddr, Arc<AtomicUsize>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let counted = hits.clone();
        let app = axum::Router::new().route(
            "/mcp",
            axum::routing::any(move || {
                counted.fetch_add(1, Ordering::SeqCst);
                async { "ok" }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (addr, hits)
    }

    /// One keep-alive request on a raw connection: `true` once the whole
    /// reply has been read, `false` when the forwarder closes it instead.
    async fn exchange(stream: &mut tokio::net::TcpStream) -> bool {
        // A write into a connection already closed fails; the read says so.
        let _ = stream
            .write_all(b"GET /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await;
        let mut got = Vec::new();
        let mut buf = [0u8; 4096];
        while !got.ends_with(b"\r\n\r\nok") {
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.read(&mut buf))
                .await
                .expect("the forwarder answers or closes")
            {
                Ok(0) | Err(_) => return false,
                Ok(n) => got.extend_from_slice(&buf[..n]),
            }
        }
        true
    }

    async fn bounded_forwarder() -> (Capture, SocketAddr, Arc<AtomicUsize>) {
        let (up, hits) = upstream().await;
        let c = Capture::new(BTreeSet::from(["t".to_string()]));
        let (own, _server) = forwarder(&format!("http://{up}/mcp"), up, c.clone(), None)
            .await
            .unwrap();
        (c, own, hits)
    }

    #[tokio::test]
    async fn the_forwarder_serves_a_bounded_number_of_connections() {
        let (c, own, hits) = bounded_forwarder().await;
        // Exactly at the cap: every connection is served and kept open.
        let mut held = Vec::new();
        for _ in 0..CONNECTION_LIMIT {
            let mut s = tokio::net::TcpStream::connect(own).await.unwrap();
            assert!(exchange(&mut s).await);
            held.push(s);
        }
        assert_eq!(reason(&c), None);
        // One over: closed unserved, never forwarded, the trial withheld.
        let mut over = tokio::net::TcpStream::connect(own).await.unwrap();
        assert!(!exchange(&mut over).await, "closed unserved");
        assert_eq!(reason(&c), Some(Refusal::TrialTooLarge));
        assert_eq!(hits.load(Ordering::SeqCst), CONNECTION_LIMIT);
        // The connections already held are served on as before.
        assert!(exchange(&mut held[0]).await);
        assert_eq!(hits.load(Ordering::SeqCst), CONNECTION_LIMIT + 1);
    }

    #[tokio::test]
    async fn the_forwarder_observes_a_bounded_number_of_requests() {
        let (c, own, hits) = bounded_forwarder().await;
        let mut s = tokio::net::TcpStream::connect(own).await.unwrap();
        // Exactly at the cap: every request observed, the record kept.
        for _ in 0..REQUEST_LIMIT {
            assert!(exchange(&mut s).await);
        }
        assert_eq!(reason(&c), None);
        // One over: still forwarded and answered, but the trial is withheld.
        assert!(exchange(&mut s).await);
        assert_eq!(hits.load(Ordering::SeqCst), REQUEST_LIMIT + 1);
        assert_eq!(reason(&c), Some(Refusal::TrialTooLarge));
    }

    /// Read one reply's status line and headers, or `None` if the
    /// connection closes first.
    async fn status(stream: &mut tokio::net::TcpStream) -> Option<String> {
        let mut got = Vec::new();
        let mut buf = [0u8; 4096];
        while !got.windows(4).any(|w| w == b"\r\n\r\n") {
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.read(&mut buf))
                .await
                .expect("the forwarder answers or closes")
            {
                Ok(0) | Err(_) => return None,
                Ok(n) => got.extend_from_slice(&buf[..n]),
            }
        }
        let text = String::from_utf8_lossy(&got);
        text.lines().next().map(str::to_string)
    }

    #[tokio::test]
    async fn the_forwarder_bounds_a_request_body_at_swarmails_own_limit() {
        let (c, own, hits) = bounded_forwarder().await;
        let post = |length: usize| {
            format!(
                "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\ncontent-type: application/json\r\ncontent-length: {length}\r\n\r\n"
            )
        };
        // Exactly at the limit: read whole and forwarded.
        let mut s = tokio::net::TcpStream::connect(own).await.unwrap();
        s.write_all(post(REQUEST_BODY_LIMIT).as_bytes())
            .await
            .unwrap();
        s.write_all(&vec![b' '; REQUEST_BODY_LIMIT]).await.unwrap();
        assert_eq!(status(&mut s).await.as_deref(), Some("HTTP/1.1 200 OK"));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        // One byte over, of a body that declares four times the limit: the
        // forwarder answers 413 once that byte arrives and never waits for
        // the rest of the body, which is never sent.
        let mut s = tokio::net::TcpStream::connect(own).await.unwrap();
        s.write_all(post(4 * REQUEST_BODY_LIMIT).as_bytes())
            .await
            .unwrap();
        s.write_all(&vec![b' '; REQUEST_BODY_LIMIT + 1])
            .await
            .unwrap();
        assert_eq!(
            status(&mut s).await.as_deref(),
            Some("HTTP/1.1 413 Payload Too Large")
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1, "never forwarded");
        assert_eq!(reason(&c), None, "a refused body is not a call");
        const { assert!(REQUEST_BODY_LIMIT < BODY_LIMIT) };
    }

    #[tokio::test]
    async fn the_socket_accepts_a_bounded_number_of_helpers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.sock");
        let unix = tokio::net::UnixListener::bind(&path).unwrap();
        let c = Capture::new(BTreeSet::from(["t".to_string()]));
        let accepting = tokio::spawn(accept_helpers(unix, c.clone()));
        let mut held = Vec::new();
        for _ in 0..HELPER_LIMIT {
            held.push(tokio::net::UnixStream::connect(&path).await.unwrap());
        }
        assert_eq!(reason(&c), None);
        // One more: the trial is withheld and the socket stops accepting.
        held.push(tokio::net::UnixStream::connect(&path).await.unwrap());
        tokio::time::timeout(std::time::Duration::from_secs(10), accepting)
            .await
            .expect("the accept loop ends")
            .unwrap();
        assert_eq!(reason(&c), Some(Refusal::BindingMismatch));
    }

    #[tokio::test]
    async fn an_observer_pairs_each_request_with_its_reply_within_its_bounds() {
        let (up, hits) = upstream().await;
        let observer = observe(&format!("http://{up}/mcp"), up).await.unwrap();
        let own: SocketAddr = observer
            .mcp_url
            .strip_prefix("http://")
            .and_then(|r| r.strip_suffix("/mcp"))
            .unwrap()
            .parse()
            .unwrap();
        let mut s = tokio::net::TcpStream::connect(own).await.unwrap();
        assert!(exchange(&mut s).await);
        let (log, overflowed) = observer.seal();
        assert!(!overflowed);
        assert_eq!(
            log,
            vec![Exchange {
                post: false,
                request: Vec::new(),
                reply: Some((200, b"ok".to_vec())),
            }]
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        // The bounds: a record past the count or the bytes only overflows,
        // and nothing is kept once the log is sealed.
        let log = ExchangeLog::default();
        for _ in 0..EXCHANGE_LIMIT {
            log.record(true, b"{}", None);
        }
        log.record(true, b"{}", None);
        let (kept, overflowed) = log.seal();
        assert_eq!((kept.len(), overflowed), (EXCHANGE_LIMIT, true));
        log.record(true, b"{}", None);
        assert_eq!(log.seal().0.len(), 0);
        let log = ExchangeLog::default();
        log.record(true, &vec![b' '; EXCHANGE_BYTES - 1], Some((200, b"x")));
        log.record(true, b"y", None);
        let (kept, overflowed) = log.seal();
        assert_eq!((kept.len(), overflowed), (1, true));
    }

    #[tokio::test]
    async fn an_observer_records_an_absent_reply() {
        // Nothing listens upstream: the client sees 502 and the log says so.
        let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up = dead.local_addr().unwrap();
        drop(dead);
        let observer = observe(&format!("http://{up}/mcp"), up).await.unwrap();
        let reply = reqwest::Client::new()
            .post(&observer.mcp_url)
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(reply.status(), StatusCode::BAD_GATEWAY);
        let (log, _) = observer.seal();
        assert_eq!(log.len(), 1);
        assert!(log[0].post && log[0].reply.is_none());
        assert_eq!(log[0].request, b"{}");
    }
}
