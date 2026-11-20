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
//!   in-flight span ends once Swarmail has answered.
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
//! memory. The worst a hostile agent can do is withhold its own trial's
//! evidence, which it can already do by calling two tools at once.

use crate::toolvalues::{Capture, Refusal, Server, Started};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::task::JoinHandle;

/// The largest request body forwarded, the replies' own bound.
const BODY_LIMIT: usize = crate::mcp::MAX_REPLY_BYTES as usize;
/// The largest observation frame the socket reads: the helper's line bound
/// plus its framing.
const FRAME_LIMIT: u64 = 3 * crate::fixture_http::INPUT_LIMIT as u64;
/// The most helper connections one trial's socket accepts: one per agent
/// process, with room for an isolation task's identities. A further
/// connection withholds the trial and closes the socket.
pub const HELPER_LIMIT: usize = 16;

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
        upstream: mcp_url.to_string(),
        upstream_addr,
        own_port: own.port(),
        capture: capture.clone(),
    });
    let app = axum::Router::new()
        .route("/mcp", axum::routing::any(forward))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(state);
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let socket = runtime.join("tool-capture.sock");
    let unix = tokio::net::UnixListener::bind(&socket).map_err(|_| "capture_socket_bind")?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "capture_socket_bind")?;
    }
    let accept = tokio::spawn(accept_helpers(unix, capture.clone()));
    Ok(Running {
        capture,
        mcp_url: format!("http://{own}/mcp"),
        socket,
        tasks: vec![server, accept],
    })
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
    let observed = if method == Method::POST {
        st.capture.observe_mcp_body(&body)
    } else {
        Vec::new()
    };
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
}
