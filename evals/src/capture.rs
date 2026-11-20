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
//! Observation never blocks, retries or alters a request; a capture that is
//! lost or ambiguous only withholds the trial's values.

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
    let observer = capture.clone();
    let accept = tokio::spawn(async move {
        while let Ok((stream, _)) = unix.accept().await {
            tokio::spawn(helper_connection(stream, observer.clone()));
        }
    });
    Ok(Running {
        capture,
        mcp_url: format!("http://{own}/mcp"),
        socket,
        tasks: vec![server, accept],
    })
}

/// One helper connection: newline-delimited JSON frames, each acknowledged
/// with one byte once the capture has recorded it.
async fn helper_connection(stream: tokio::net::UnixStream, capture: Capture) {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut open: Vec<Started> = Vec::new();
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
            (Some("start"), Some(tool), Some(arguments)) => {
                open.push(capture.observe(Server::Fixture, tool, arguments.as_bytes()))
            }
            (Some("end"), _, _) if !open.is_empty() => drop(open.remove(0)),
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
