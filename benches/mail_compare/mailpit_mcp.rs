//! The thin Mailpit MCP adapter (spec C2, proposed decision 0092 §3): an
//! MCP 2025-03-26 stdio server whose four tools translate one-to-one onto
//! Mailpit's own documented HTTP API — list, search, read, and a labelled
//! common-client poll over search. It adds nothing Mailpit does not serve:
//! no admin, send, delete or release operations, no task hints, no
//! extraction, cache, index or prefetch, no inboxes Mailpit lacks and no
//! synthetic chaos. Results are Mailpit's own JSON in text content blocks.
//!
//! Every operation is bounded in bytes, pages, requests and time; at most
//! one tool call runs while a short queue waits, and the input stays read
//! while it runs, so cancellation and EOF are seen. The caller (the eval
//! owner) starts Mailpit and hands this adapter its loopback endpoint.

use crate::wire::{Http, WireError, query_value};
use serde_json::{Map, Value, json};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::Instant;

pub const PROTOCOL_REVISION: &str = "2025-03-26";
pub const SERVER_NAME: &str = "mail-compare-mailpit-mcp";

/// The adapter's engineering ceilings (spec plan, "Adapter contract").
/// They bound this client; none is a claim about Mailpit's own limits.
#[derive(Debug, Clone)]
pub struct Bounds {
    pub max_line: usize,
    /// Tool calls admitted at once, the running one included.
    pub max_pending: usize,
    pub max_reply: usize,
    pub request: Duration,
    pub max_body: u64,
    pub max_operation_bytes: u64,
    pub max_pages: u32,
    pub max_requests: u32,
    pub operation: Duration,
    pub cleanup: Duration,
    pub poll_interval: Duration,
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            max_line: 64 << 10,
            max_pending: 8,
            max_reply: 1 << 20,
            request: Duration::from_secs(10),
            max_body: 16 << 20,
            max_operation_bytes: 16 << 20,
            max_pages: 100,
            max_requests: 1000,
            operation: Duration::from_secs(30),
            cleanup: Duration::from_secs(10),
            poll_interval: Duration::from_millis(100),
        }
    }
}

pub const MAX_START: u64 = 100_000;
pub const MAX_LIMIT: u64 = 250;
pub const DEFAULT_LIMIT: u64 = 50;
pub const MAX_QUERY_BYTES: usize = 4096;
pub const MAX_ID_BYTES: usize = 512;
pub const MAX_TIMEOUT_MS: u64 = 30_000;
const SCAN_PAGE: u64 = 250;

/// `http://127.0.0.1:PORT` or `http://[::1]:PORT`, nothing else: no
/// name, credentials, path, query or non-loopback address.
pub fn parse_endpoint(s: &str) -> Result<SocketAddr, &'static str> {
    let rest = s
        .strip_prefix("http://")
        .ok_or("the endpoint must be http://<loopback ip>:<port>")?;
    let addr: SocketAddr = rest
        .parse()
        .map_err(|_| "the endpoint must be a literal socket address with no path")?;
    if !addr.ip().is_loopback() || addr.port() == 0 {
        return Err("the endpoint must be a loopback address with a port");
    }
    Ok(addr)
}

/// A finite, input-free failure: nothing upstream sent and nothing the
/// caller sent is echoed back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolError {
    NotFound,
    Upstream(u16),
    Transport(&'static str),
    Malformed,
    Incomplete,
    Budget(&'static str),
    ReplyTooLarge,
}

impl ToolError {
    pub fn text(self) -> String {
        match self {
            ToolError::NotFound => "not_found: Mailpit has no message with that id".into(),
            ToolError::Upstream(code) => format!("upstream_status: Mailpit answered HTTP {code}"),
            ToolError::Transport(r) => format!("transport: {r}"),
            ToolError::Malformed => {
                "malformed: Mailpit's answer did not have the documented shape".into()
            }
            ToolError::Incomplete => "incomplete: the paged scan could not be proved complete \
                 (a repeated id, a moving count or a short page)"
                .into(),
            ToolError::Budget(what) => format!("budget_exhausted: {what}"),
            ToolError::ReplyTooLarge => {
                "reply_too_large: the result exceeds the adapter's reply bound".into()
            }
        }
    }
}

/// A validated tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    List {
        start: u64,
        limit: u64,
    },
    Search {
        query: String,
        start: u64,
        limit: u64,
    },
    Read {
        id: String,
    },
    Wait {
        query: String,
        timeout_ms: u64,
    },
}

pub fn tools() -> Value {
    let start = json!({"type": "integer", "minimum": 0, "maximum": MAX_START, "default": 0,
        "description": "Pagination offset, newest first."});
    let limit = json!({"type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT,
        "description": "Page size. 250 is this adapter's ceiling, not Mailpit's."});
    let query = json!({"type": "string", "minLength": 1, "maxLength": MAX_QUERY_BYTES,
        "description": "A Mailpit search query, passed through unchanged (Mailpit's search filter syntax)."});
    json!([
        {
            "name": "mailpit_list",
            "description": "One page of Mailpit's GET /api/v1/messages, newest first, as Mailpit returns it: \
                total (mailbox size), messages_count, start, count (this page) and the message summaries. \
                Mailpit has one shared mailbox; it does not separate mail by SMTP identity.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "properties": {"start": start, "limit": limit}}
        },
        {
            "name": "mailpit_search",
            "description": "One page of Mailpit's GET /api/v1/search, newest first. messages_count is the \
                number of matches; total is the whole mailbox, not the matches.",
            "inputSchema": {"type": "object", "additionalProperties": false, "required": ["query"],
                "properties": {"query": query, "start": start, "limit": limit}}
        },
        {
            "name": "mailpit_read",
            "description": "Mailpit's GET /api/v1/message/{ID}: the stored message's summary with its \
                ordinary Text and HTML bodies, as Mailpit returns it. Side effect: Mailpit marks the \
                message read. The ID \"latest\" is Mailpit's own alias for the newest message.",
            "inputSchema": {"type": "object", "additionalProperties": false, "required": ["id"],
                "properties": {"id": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}}}
        },
        {
            "name": "mailpit_wait",
            "description": "Client-side polling, not a Mailpit wait facility: a complete scan of \
                GET /api/v1/search at once and then every 100 ms until a message matches or \
                timeout_ms passes. Answers found (with every match) or timeout, each only after a \
                complete scan, plus how many scans and HTTP requests it took.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "required": ["query", "timeout_ms"],
                "properties": {"query": query,
                    "timeout_ms": {"type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_MS}}}
        }
    ])
}

fn uint(
    args: &Map<String, Value>,
    key: &str,
    default: Option<u64>,
    min: u64,
    max: u64,
) -> Option<u64> {
    match args.get(key) {
        None => default,
        Some(v) => v.as_u64().filter(|n| (min..=max).contains(n)),
    }
}

fn text(args: &Map<String, Value>, key: &str, max: usize) -> Option<String> {
    args.get(key)?
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max)
        .map(str::to_owned)
}

/// Validate `tools/call` params; `None` refuses with invalid params.
pub fn parse_call(params: &Value) -> Option<Call> {
    let p = params.as_object()?;
    if p.keys()
        .any(|k| !matches!(k.as_str(), "name" | "arguments" | "_meta"))
    {
        return None;
    }
    let empty = Map::new();
    let args = match p.get("arguments") {
        None => &empty,
        Some(v) => v.as_object()?,
    };
    let allowed: &[&str] = match p.get("name")?.as_str()? {
        "mailpit_list" => &["start", "limit"],
        "mailpit_search" => &["query", "start", "limit"],
        "mailpit_read" => &["id"],
        "mailpit_wait" => &["query", "timeout_ms"],
        _ => return None,
    };
    if args.keys().any(|k| !allowed.contains(&k.as_str())) {
        return None;
    }
    let start = || uint(args, "start", Some(0), 0, MAX_START);
    let limit = || uint(args, "limit", Some(DEFAULT_LIMIT), 1, MAX_LIMIT);
    Some(match p.get("name")?.as_str()? {
        "mailpit_list" => Call::List {
            start: start()?,
            limit: limit()?,
        },
        "mailpit_search" => Call::Search {
            query: text(args, "query", MAX_QUERY_BYTES)?,
            start: start()?,
            limit: limit()?,
        },
        "mailpit_read" => Call::Read {
            id: text(args, "id", MAX_ID_BYTES)?,
        },
        _ => Call::Wait {
            query: text(args, "query", MAX_QUERY_BYTES)?,
            timeout_ms: uint(args, "timeout_ms", None, 1, MAX_TIMEOUT_MS)?,
        },
    })
}

/// One operation's aggregate allowance: requests, bytes and pages across
/// all its HTTP exchanges, under one absolute deadline.
struct Budget<'a> {
    http: &'a Http,
    bounds: &'a Bounds,
    deadline: Instant,
    requests: u32,
    bytes: u64,
}

impl Budget<'_> {
    async fn get(&mut self, path: &str) -> Result<(u16, Value), ToolError> {
        if self.requests >= self.bounds.max_requests {
            return Err(ToolError::Budget("request count"));
        }
        self.requests += 1;
        let (status, body) = tokio::time::timeout_at(self.deadline, self.http.get(path))
            .await
            .map_err(|_| ToolError::Budget("operation deadline"))?
            .map_err(|e| match e {
                WireError::Oversized => ToolError::Budget("response bytes"),
                e => ToolError::Transport(e.reason()),
            })?;
        self.bytes += body.len() as u64;
        if self.bytes > self.bounds.max_operation_bytes {
            return Err(ToolError::Budget("operation bytes"));
        }
        match status {
            200 => serde_json::from_slice(&body)
                .map(|v| (status, v))
                .map_err(|_| ToolError::Malformed),
            404 => Err(ToolError::NotFound),
            s => Err(ToolError::Upstream(s)),
        }
    }
}

/// A Mailpit list or search page, checked against the documented shape:
/// counts are integers, `start` echoes the offset asked for, every
/// summary carries a string ID, and the undocumented page `count` the
/// pinned binary also sends agrees with the summaries when present.
fn summary_page(v: Value, start: u64) -> Result<Map<String, Value>, ToolError> {
    let Value::Object(mut page) = v else {
        return Err(ToolError::Malformed);
    };
    let n = |k: &str| page.get(k).and_then(Value::as_u64);
    let (Some(_total), Some(_matching), Some(echo)) = (n("total"), n("messages_count"), n("start"))
    else {
        return Err(ToolError::Malformed);
    };
    let messages = page
        .get("messages")
        .and_then(Value::as_array)
        .ok_or(ToolError::Malformed)?;
    if echo != start
        || messages.iter().any(|m| {
            m.get("ID")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        })
        || page
            .get("count")
            .is_some_and(|c| c.as_u64() != Some(messages.len() as u64))
    {
        return Err(ToolError::Malformed);
    }
    let count = messages.len() as u64;
    page.insert("count".into(), count.into());
    Ok(page)
}

fn search_path(query: &str, start: u64, limit: u64) -> String {
    format!(
        "/api/v1/search?query={}&start={start}&limit={limit}",
        query_value(query)
    )
}

/// Every match of `query`, paged to completion: the scan ends exactly
/// when the pages reach messages_count, with every ID distinct and the
/// count unchanged; anything else is incomplete, never a shorter answer.
async fn complete_search(b: &mut Budget<'_>, query: &str) -> Result<(u64, Vec<Value>), ToolError> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut expected = None;
    for page_no in 0..u64::from(b.bounds.max_pages) {
        let start = page_no * SCAN_PAGE;
        let (_, v) = b.get(&search_path(query, start, SCAN_PAGE)).await?;
        let mut page = summary_page(v, start)?;
        let matching = page["messages_count"]
            .as_u64()
            .ok_or(ToolError::Malformed)?;
        if *expected.get_or_insert(matching) != matching {
            return Err(ToolError::Incomplete);
        }
        let Some(Value::Array(messages)) = page.remove("messages") else {
            return Err(ToolError::Malformed);
        };
        let got = messages.len();
        for m in messages {
            let id = m["ID"].as_str().unwrap_or_default().to_owned();
            if !seen.insert(id) {
                return Err(ToolError::Incomplete);
            }
            out.push(m);
        }
        match (out.len() as u64).cmp(&matching) {
            std::cmp::Ordering::Equal => return Ok((matching, out)),
            std::cmp::Ordering::Greater => return Err(ToolError::Incomplete),
            std::cmp::Ordering::Less if got == 0 => return Err(ToolError::Incomplete),
            std::cmp::Ordering::Less => {}
        }
    }
    Err(ToolError::Budget("pages per scan"))
}

/// Run one validated call against Mailpit within `bounds`.
pub async fn run_call(http: &Http, bounds: &Bounds, call: &Call) -> Result<Value, ToolError> {
    let mut b = Budget {
        http,
        bounds,
        deadline: Instant::now() + bounds.operation,
        requests: 0,
        bytes: 0,
    };
    match call {
        Call::List { start, limit } => {
            let (_, v) = b
                .get(&format!("/api/v1/messages?start={start}&limit={limit}"))
                .await?;
            Ok(Value::Object(summary_page(v, *start)?))
        }
        Call::Search {
            query,
            start,
            limit,
        } => {
            let (_, v) = b.get(&search_path(query, *start, *limit)).await?;
            Ok(Value::Object(summary_page(v, *start)?))
        }
        Call::Read { id } => {
            let (_, v) = b
                .get(&format!("/api/v1/message/{}", query_value(id)))
                .await?;
            let got = v.get("ID").and_then(Value::as_str);
            if got.is_none_or(|g| g != id && id != "latest") {
                return Err(ToolError::Malformed);
            }
            Ok(v)
        }
        Call::Wait { query, timeout_ms } => {
            let until = Instant::now() + Duration::from_millis(*timeout_ms);
            // The last scan starts before `until` and may take one request
            // bound to finish; the operation deadline allows exactly that.
            b.deadline = until + bounds.request;
            let mut scans = 0u32;
            loop {
                let (matching, messages) = complete_search(&mut b, query).await?;
                scans += 1;
                if matching > 0 {
                    return Ok(json!({"status": "found", "messages_count": matching,
                        "messages": messages, "scans": scans, "requests": b.requests,
                        "method": "client-side poll of /api/v1/search"}));
                }
                if Instant::now() + bounds.poll_interval >= until {
                    return Ok(json!({"status": "timeout", "messages_count": 0,
                        "messages": [], "scans": scans, "requests": b.requests,
                        "method": "client-side poll of /api/v1/search"}));
                }
                // The frozen poll interval, not synchronization: the
                // absolute deadline above ends the wait.
                tokio::time::sleep(bounds.poll_interval).await;
            }
        }
    }
}

enum Inbound {
    Line(Vec<u8>),
    TooLong,
    Eof,
}

/// Newline-delimited input, each line capped at `max` bytes: an overlong
/// line is skipped to its end and reported, never buffered.
async fn read_lines<R: AsyncRead + Unpin>(input: R, max: usize, tx: mpsc::Sender<Inbound>) {
    let mut r = BufReader::new(input);
    loop {
        let mut line = Vec::new();
        let mut too_long = false;
        let eof = loop {
            let buf = match r.fill_buf().await {
                Ok(b) => b,
                Err(_) => break true,
            };
            if buf.is_empty() {
                break true;
            }
            let (chunk, used, done) = match buf.iter().position(|b| *b == b'\n') {
                Some(i) => (&buf[..i], i + 1, true),
                None => (buf, buf.len(), false),
            };
            if !too_long && line.len() + chunk.len() > max {
                too_long = true;
                line = Vec::new();
            } else if !too_long {
                line.extend_from_slice(chunk);
            }
            r.consume(used);
            if done {
                break false;
            }
        };
        let msg = if too_long {
            Some(Inbound::TooLong)
        } else {
            (!line.iter().all(u8::is_ascii_whitespace)).then_some(Inbound::Line(line))
        };
        if let Some(m) = msg
            && tx.send(m).await.is_err()
        {
            return;
        }
        if eof {
            let _ = tx.send(Inbound::Eof).await;
            return;
        }
    }
}

/// How serving ended. Only `Eof` is an orderly end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Eof,
    OutputBlocked,
    OutputClosed,
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// A tool result: Mailpit's JSON as one text block, or a finite error.
fn tool_reply(id: Value, outcome: Result<Value, ToolError>, max: usize) -> Value {
    let (body, is_error) = match outcome {
        Ok(v) => (v.to_string(), false),
        Err(e) => (json!({"error": e.text()}).to_string(), true),
    };
    // The envelope adds well under 1 KiB to the text it carries.
    let (body, is_error) = if body.len() + 1024 > max {
        (
            json!({"error": ToolError::ReplyTooLarge.text()}).to_string(),
            true,
        )
    } else {
        (body, is_error)
    };
    result(
        id,
        json!({"content": [{"type": "text", "text": body}], "isError": is_error}),
    )
}

struct Running {
    seq: u64,
    id: Value,
    task: tokio::task::JoinHandle<()>,
}

/// Serve MCP on `input`/`output` until EOF or a stuck or closed output.
pub async fn serve<R, W>(input: R, mut output: W, http: Http, bounds: Bounds) -> End
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    let http = Arc::new(http);
    let (in_tx, mut in_rx) = mpsc::channel(1);
    let reader = tokio::spawn(read_lines(input, bounds.max_line, in_tx));
    let (done_tx, mut done_rx) = mpsc::channel::<(u64, Result<Value, ToolError>)>(1);
    let mut initialized = false;
    let mut queue: VecDeque<(Value, Call)> = VecDeque::new();
    let mut running: Option<Running> = None;
    let mut seq = 0u64;
    let end = loop {
        let mut replies = Vec::new();
        tokio::select! {
            msg = in_rx.recv() => match msg {
                None | Some(Inbound::Eof) => break End::Eof,
                Some(Inbound::TooLong) => replies.push(error(Value::Null, -32600, "request line exceeds the adapter's line bound")),
                Some(Inbound::Line(line)) => {
                    let busy = queue.len() + usize::from(running.is_some());
                    if let Some(r) = handle(&line, &mut initialized, busy, &bounds, &mut queue, &mut running) {
                        replies.push(r);
                    }
                }
            },
            Some((done, outcome)) = done_rx.recv() => {
                if running.as_ref().is_some_and(|r| r.seq == done) {
                    let r = running.take().expect("checked above");
                    replies.push(tool_reply(r.id, outcome, bounds.max_reply));
                }
            }
        }
        if running.is_none()
            && let Some((id, call)) = queue.pop_front()
        {
            seq += 1;
            let (http, bounds2, tx) = (http.clone(), bounds.clone(), done_tx.clone());
            let task = tokio::spawn(async move {
                let outcome = run_call(&http, &bounds2, &call).await;
                let _ = tx.send((seq, outcome)).await;
            });
            running = Some(Running { seq, id, task });
        }
        let mut stop = None;
        for r in replies {
            if let Err(e) = write(&mut output, &r, bounds.cleanup).await {
                stop = Some(e);
                break;
            }
        }
        if let Some(e) = stop {
            break e;
        }
    };
    // Admission is closed: cancel the running call, drop the queue
    // unanswered and wait (bounded) for the cancelled task to end.
    queue.clear();
    if let Some(r) = running.take() {
        r.task.abort();
        let _ = tokio::time::timeout(bounds.cleanup, r.task).await;
    }
    reader.abort();
    let _ = tokio::time::timeout(bounds.cleanup, reader).await;
    end
}

async fn write<W: AsyncWrite + Unpin>(out: &mut W, v: &Value, bound: Duration) -> Result<(), End> {
    let mut bytes = v.to_string().into_bytes();
    bytes.push(b'\n');
    match tokio::time::timeout(bound, async {
        out.write_all(&bytes).await?;
        out.flush().await
    })
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(End::OutputClosed),
        Err(_) => Err(End::OutputBlocked),
    }
}

/// One inbound message; the reply to write now, if any. Tool calls are
/// queued here and answered when they finish.
fn handle(
    line: &[u8],
    initialized: &mut bool,
    busy: usize,
    bounds: &Bounds,
    queue: &mut VecDeque<(Value, Call)>,
    running: &mut Option<Running>,
) -> Option<Value> {
    let Ok(msg) = serde_json::from_slice::<Value>(line) else {
        return Some(error(Value::Null, -32700, "parse error"));
    };
    let Value::Object(m) = msg else {
        return Some(error(
            Value::Null,
            -32600,
            "only single JSON-RPC objects are accepted; batches are not supported",
        ));
    };
    let id = m.get("id").cloned();
    let id_ok = matches!(&id, Some(Value::String(_))) || id.as_ref().is_some_and(Value::is_i64);
    if m.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || (id.is_some() && !id_ok) {
        return Some(error(
            if id_ok {
                id.unwrap_or(Value::Null)
            } else {
                Value::Null
            },
            -32600,
            "invalid request",
        ));
    }
    let Some(method) = m.get("method").and_then(Value::as_str) else {
        // A response from the client, or nothing at all: no reply either way.
        return id
            .filter(|_| !m.contains_key("result") && !m.contains_key("error"))
            .map(|id| error(id, -32600, "invalid request"));
    };
    let params = m.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        // Notifications never get a reply.
        match method {
            "notifications/initialized" => {}
            "notifications/cancelled" => {
                let target = params.get("requestId");
                if let Some(t) = target {
                    if running.as_ref().is_some_and(|r| &r.id == t) {
                        let r = running.take().expect("checked above");
                        r.task.abort();
                    }
                    queue.retain(|(qid, _)| qid != t);
                }
            }
            _ => {}
        }
        return None;
    };
    let in_flight = |id: &Value| {
        running.as_ref().is_some_and(|r| &r.id == id) || queue.iter().any(|(q, _)| q == id)
    };
    match method {
        "initialize" => {
            if *initialized {
                return Some(error(id, -32600, "already initialized"));
            }
            if params.get("protocolVersion").and_then(Value::as_str) != Some(PROTOCOL_REVISION) {
                return Some(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602,
                    "message": "unsupported protocol version",
                    "data": {"supported": [PROTOCOL_REVISION]}}}));
            }
            *initialized = true;
            Some(result(
                id,
                json!({"protocolVersion": PROTOCOL_REVISION,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")}}),
            ))
        }
        "ping" => Some(result(id, json!({}))),
        _ if !*initialized => Some(error(id, -32600, "not initialized")),
        "tools/list" => Some(result(id, json!({"tools": tools()}))),
        "tools/call" => {
            if in_flight(&id) {
                return Some(error(id, -32600, "a request with this id is in flight"));
            }
            let Some(call) = parse_call(&params) else {
                return Some(error(id, -32602, "invalid tool name or arguments"));
            };
            if busy >= bounds.max_pending {
                return Some(error(
                    id,
                    -32000,
                    "too many tool calls in flight; retry later",
                ));
            }
            queue.push_back((id, call));
            None
        }
        _ => Some(error(id, -32601, "method not found")),
    }
}
