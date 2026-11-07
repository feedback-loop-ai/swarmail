//! The loopback bridge from the agent's transport to an adapter's stdio
//! (proposed decision 0129 §4, spec R3). Every target is offered to agents
//! as Streamable HTTP `POST /mcp`; for a competitor that endpoint is this
//! bridge, served inside the runner process. Each request body is written
//! as one line to the owned adapter's stdin and the replies are routed back
//! by JSON-RPC id; a batch is forwarded as one line and its array reply is
//! returned as one body.
//!
//! - A body holding only notifications is forwarded and answered `202`.
//! - `GET` is `405`. The `Host`/`Origin` guard is Swarmail's (decision 0043):
//!   a foreign `Host`, or a present `Origin` that is not a loopback spelling
//!   of the bound port, is `403`.
//! - A request id already in flight is answered with a correlated `-32600`
//!   and nothing is forwarded.
//! - A body that could draw only an uncorrelatable (null-id) reply from the
//!   adapter — not JSON, not an object or array, an empty or over-cap array,
//!   a batch with no request in it — is refused here, locally.
//! - An adapter that has exited yields `503` for every request in flight and
//!   after: the trial then ends as a visible harness failure.
//! - Each session's `clientInfo` and `protocolVersion` are kept for the
//!   trial's private artifacts. No body is kept or logged (decision 0075).
//!
//! The bridge does not own the adapter process: the runner does, and only
//! the adapter's own exit status after full cleanup is a receipt.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// The adapters' line bound (`mcp::Bounds::default().max_line`): a larger
/// body is never forwarded.
pub const MAX_BODY: usize = 64 << 10;
/// The batch cap every adapter and Swarmail's own endpoint hold.
pub const MAX_BATCH: usize = 16;
/// The longest the bridge holds a request for the adapter's reply: above an
/// adapter's 30 s operation ceiling plus its queue.
const REPLY_DEADLINE: Duration = Duration::from_secs(120);
/// The longest a recorded `clientInfo` or version string is kept.
const NAME_MAX: usize = 128;

/// What a session announced about its client: negotiation metadata only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Session {
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub protocol_version: Option<String>,
}

#[derive(Default)]
struct Inner {
    by_id: HashMap<String, u64>,
    slots: HashMap<u64, (Vec<String>, oneshot::Sender<Value>)>,
    sessions: Vec<Session>,
}

struct Shared {
    port: u16,
    to_adapter: mpsc::Sender<String>,
    inner: Mutex<Inner>,
    next: Mutex<u64>,
    exited: AtomicBool,
}

pub struct Bridge {
    pub addr: SocketAddr,
    shared: Arc<Shared>,
    shutdown: Option<oneshot::Sender<()>>,
    server: JoinHandle<()>,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

fn key_of(id: &Value) -> String {
    id.to_string()
}

/// A request member: a string or integer id and a string method.
fn request_id(m: &Value) -> Option<&Value> {
    let id = m.get("id")?;
    (m.get("method")?.is_string() && (id.is_string() || id.is_i64())).then_some(id)
}

/// A notification member: a method and no id at all.
fn is_notification(m: &Value) -> bool {
    m.is_object() && m.get("method").is_some_and(Value::is_string) && m.get("id").is_none()
}

fn error_body(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn json_response(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn printable(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    Some(
        s.chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .take(NAME_MAX)
            .collect(),
    )
}

/// The same Host/Origin guard as Swarmail's endpoint (decision 0043): the
/// `Host` must be a loopback spelling of the bound port, and a present
/// `Origin` must be too.
fn guard(headers: &HeaderMap, port: u16) -> Option<Response> {
    let spelled = |s: &str| {
        [
            format!("127.0.0.1:{port}"),
            format!("localhost:{port}"),
            format!("[::1]:{port}"),
        ]
        .iter()
        .any(|ok| ok.eq_ignore_ascii_case(s))
    };
    let host_ok = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(spelled);
    let origin_ok = match headers.get(header::ORIGIN) {
        None => true,
        Some(o) => o
            .to_str()
            .ok()
            .and_then(|o| o.strip_prefix("http://"))
            .is_some_and(spelled),
    };
    (!(host_ok && origin_ok)).then(|| {
        json_response(
            StatusCode::FORBIDDEN,
            &error_body(Value::Null, -32600, "forbidden host or origin"),
        )
    })
}

fn unavailable() -> Response {
    json_response(
        StatusCode::SERVICE_UNAVAILABLE,
        &error_body(Value::Null, -32000, "the adapter is not running"),
    )
}

async fn get_mcp() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

async fn post_mcp(State(sh): State<Arc<Shared>>, headers: HeaderMap, body: Bytes) -> Response {
    if let Some(refused) = guard(&headers, sh.port) {
        return refused;
    }
    if sh.exited.load(Ordering::SeqCst) {
        return unavailable();
    }
    if body.len() > MAX_BODY {
        return json_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            &error_body(Value::Null, -32600, "request body exceeds the line bound"),
        );
    }
    let bad = |status, code, why: &str| json_response(status, &error_body(Value::Null, code, why));
    let Ok(msg) = serde_json::from_slice::<Value>(&body) else {
        return bad(StatusCode::BAD_REQUEST, -32700, "parse error");
    };
    let (members, batch): (Vec<&Value>, bool) = match &msg {
        Value::Array(v) if (1..=MAX_BATCH).contains(&v.len()) => (v.iter().collect(), true),
        Value::Array(_) => {
            return bad(
                StatusCode::BAD_REQUEST,
                -32600,
                "a batch is 1 to 16 requests",
            );
        }
        Value::Object(_) => (vec![&msg], false),
        _ => return bad(StatusCode::BAD_REQUEST, -32600, "invalid request"),
    };
    let ids: Vec<&Value> = members.iter().filter_map(|m| request_id(m)).collect();
    if ids.is_empty() {
        if !members.iter().all(|m| is_notification(m)) {
            return bad(StatusCode::BAD_REQUEST, -32600, "invalid request");
        }
        // Notifications only: forwarded for their effect, answered 202.
        return if sh.to_adapter.send(msg.to_string()).await.is_ok() {
            StatusCode::ACCEPTED.into_response()
        } else {
            unavailable()
        };
    }
    // Register every request id of this body to one reply slot.
    let (tx, rx) = oneshot::channel();
    let slot = {
        let mut next = sh.next.lock().unwrap_or_else(|e| e.into_inner());
        *next += 1;
        *next
    };
    {
        let mut inner = sh.inner.lock().unwrap_or_else(|e| e.into_inner());
        let keys: Vec<String> = {
            let mut k: Vec<String> = ids.iter().map(|i| key_of(i)).collect();
            k.dedup();
            k
        };
        if let Some(busy) = ids.iter().find(|i| inner.by_id.contains_key(&key_of(i))) {
            // An id already in flight: refuse, correlated, forward nothing.
            let refuse =
                |id: &Value| error_body(id.clone(), -32600, "a request with this id is in flight");
            return if batch {
                let all: Vec<Value> = ids.iter().map(|i| refuse(i)).collect();
                json_response(StatusCode::OK, &Value::Array(all))
            } else {
                json_response(StatusCode::OK, &refuse(busy))
            };
        }
        for k in &keys {
            inner.by_id.insert(k.clone(), slot);
        }
        inner.slots.insert(slot, (keys, tx));
        for m in &members {
            if m.get("method").and_then(Value::as_str) == Some("initialize") {
                let p = m.get("params");
                let info = p.and_then(|p| p.get("clientInfo"));
                inner.sessions.push(Session {
                    client_name: printable(info.and_then(|i| i.get("name"))),
                    client_version: printable(info.and_then(|i| i.get("version"))),
                    protocol_version: printable(p.and_then(|p| p.get("protocolVersion"))),
                });
            }
        }
    }
    let forget = |sh: &Shared| {
        let mut inner = sh.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((keys, _)) = inner.slots.remove(&slot) {
            for k in keys {
                inner.by_id.remove(&k);
            }
        }
    };
    if sh.to_adapter.send(msg.to_string()).await.is_err() {
        forget(&sh);
        return unavailable();
    }
    match tokio::time::timeout(REPLY_DEADLINE, rx).await {
        Ok(Ok(reply)) => json_response(StatusCode::OK, &reply),
        // The reader dropped the slot: the adapter exited.
        Ok(Err(_)) => unavailable(),
        Err(_) => {
            forget(&sh);
            json_response(
                StatusCode::GATEWAY_TIMEOUT,
                &error_body(Value::Null, -32000, "the adapter did not answer in time"),
            )
        }
    }
}

/// Route each adapter line to the request that owns its id.
async fn route<R: AsyncBufRead + Unpin>(mut from: R, sh: Arc<Shared>) {
    let mut line = String::new();
    loop {
        line.clear();
        match from.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Ok(reply) = serde_json::from_str::<Value>(line.trim_end()) else {
            continue;
        };
        let first_id = match &reply {
            Value::Array(v) => v.iter().find_map(|m| m.get("id").filter(|i| !i.is_null())),
            other => other.get("id").filter(|i| !i.is_null()),
        };
        let Some(id) = first_id else { continue };
        let mut inner = sh.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(slot) = inner.by_id.get(&key_of(id)).copied() else {
            continue;
        };
        if let Some((keys, tx)) = inner.slots.remove(&slot) {
            for k in keys {
                inner.by_id.remove(&k);
            }
            let _ = tx.send(reply);
        }
    }
    // The adapter's output ended: it exited. Every waiting request is
    // released (its slot dropped), and later ones are refused.
    sh.exited.store(true, Ordering::SeqCst);
    let mut inner = sh.inner.lock().unwrap_or_else(|e| e.into_inner());
    inner.slots.clear();
    inner.by_id.clear();
}

impl Bridge {
    /// Serve `POST /mcp` on a loopback port, forwarding to `to_adapter` and
    /// routing from `from_adapter` (positioned after the adapter's
    /// announcement line).
    pub async fn start<W, R>(to_adapter: W, from_adapter: R) -> std::io::Result<Bridge>
    where
        W: AsyncWrite + Unpin + Send + 'static,
        R: AsyncBufRead + Unpin + Send + 'static,
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let (line_tx, mut line_rx) = mpsc::channel::<String>(16);
        let shared = Arc::new(Shared {
            port: addr.port(),
            to_adapter: line_tx,
            inner: Mutex::new(Inner::default()),
            next: Mutex::new(0),
            exited: AtomicBool::new(false),
        });
        let mut out = to_adapter;
        let writer = tokio::spawn(async move {
            while let Some(line) = line_rx.recv().await {
                let mut bytes = line.into_bytes();
                bytes.push(b'\n');
                if out.write_all(&bytes).await.is_err() || out.flush().await.is_err() {
                    break;
                }
            }
            // Dropping `out` closes the adapter's stdin: the orderly end.
        });
        let reader = tokio::spawn(route(from_adapter, shared.clone()));
        let app = axum::Router::new()
            .route("/mcp", post(post_mcp).get(get_mcp))
            .with_state(shared.clone());
        let (shutdown, stop) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stop.await;
                })
                .await;
        });
        Ok(Bridge {
            addr,
            shared,
            shutdown: Some(shutdown),
            server,
            reader,
            writer,
        })
    }

    /// The URL an agent is given.
    pub fn url(&self) -> String {
        format!("http://{}/mcp", self.addr)
    }

    /// The negotiation metadata of every session seen so far.
    pub fn sessions(&self) -> Vec<Session> {
        self.shared
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sessions
            .clone()
    }

    /// Whether the adapter's output has ended.
    pub fn adapter_exited(&self) -> bool {
        self.shared.exited.load(Ordering::SeqCst)
    }

    /// Stop serving and close the adapter's stdin, which is the adapter's
    /// cue to clean up and exit. The caller then waits for that exit.
    pub async fn stop(mut self) {
        if let Some(s) = self.shutdown.take() {
            let _ = s.send(());
        }
        let _ = tokio::time::timeout(Duration::from_secs(5), &mut self.server).await;
        self.server.abort();
        // The writer ends when the last sender drops. The reader and the
        // router's state each hold one, so they go first.
        self.reader.abort();
        let _ = (&mut self.reader).await;
        drop(self.shared);
        let _ = tokio::time::timeout(Duration::from_secs(5), &mut self.writer).await;
        self.writer.abort();
    }
}
