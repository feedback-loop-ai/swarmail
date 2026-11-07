//! The generic half of the thin mail-backend MCP adapters (spec C2,
//! proposed decisions 0092 §3 and 0129 §5–§6): an MCP 2025-03-26 stdio
//! server over a small [`Backend`] interface. The backend names its tools,
//! validates a call and runs it against its own upstream HTTP API; this half
//! owns everything else — the bounds, admission, the capped line reader,
//! the frozen poll wait and its time frame, the per-operation budget, reply
//! shaping and JSON-RPC batches.
//!
//! Every operation is bounded in bytes, pages, requests and time; at most
//! one tool call runs while a short queue waits, and the input stays read
//! while it runs, so cancellation and EOF are seen. The caller (the eval
//! owner) starts the backend and hands this adapter its loopback endpoint,
//! or the adapter launches it through `targets::Owned`.
//!
//! Batches (MCP 2025-03-26) follow `src/mcp.rs`'s bound: an array of 1 to
//! [`MAX_BATCH`] members is answered with one array in member order; a
//! notification gets no reply and a batch of only notifications writes
//! nothing; an empty array or an over-cap one gets one null-id error and
//! executes nothing (a refusal, never a clamp). `initialize` inside a
//! batch is a correlated `-32600`. A batch is one admission unit and its
//! members run one at a time in member order, each under the ordinary
//! per-call budgets; its first member's frame starts at admission and each
//! later one's when its turn starts. The whole array is bounded by the
//! reply bound: a member whose reply would cross it gets its own correlated
//! `reply_too_large`. Duplicate ids are answered as sent. When the
//! admission queue is full every request member gets a correlated busy
//! error and nothing runs. Swarmail runs batch members concurrently; the
//! adapters run them one at a time, and the product record says so.

use crate::wire::{Http, WireError};
use serde_json::{Map, Value, json};
use std::collections::VecDeque;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::Instant;

pub const PROTOCOL_REVISION: &str = "2025-03-26";
/// The largest batch answered; the same number as `src/mcp.rs`'s
/// `MCP_BATCH_CAP` (a test holds the two together).
pub const MAX_BATCH: usize = 16;

/// What one mail backend supplies to the generic server.
pub trait Backend: Send + Sync + 'static {
    /// A validated tool call.
    type Call: Clone + std::fmt::Debug + PartialEq + Send + 'static;
    /// The upstream's name, for finite error texts.
    const UPSTREAM: &'static str;
    /// The MCP `serverInfo.name`.
    const SERVER_NAME: &'static str;

    /// The tool list, as `tools/list` returns it.
    fn tools(bounds: &Bounds) -> Value;
    /// Validate `tools/call` params; `None` refuses with invalid params.
    fn parse_call(params: &Value, bounds: &Bounds) -> Option<Self::Call>;
    /// Run one validated call against the upstream within `bounds`, its
    /// frame starting at `admitted`.
    fn run_call(
        http: &Http,
        bounds: &Bounds,
        call: &Self::Call,
        admitted: Instant,
    ) -> impl Future<Output = Result<Value, ToolError>> + Send;
}

/// The SHA-256 of a backend's tool list as `tools/list` serves it: the
/// digest a product record freezes and the runner checks.
pub fn tools_sha256<B: Backend>(bounds: &Bounds) -> String {
    let bytes = serde_json::to_vec(&B::tools(bounds)).unwrap_or_default();
    crate::targets::sha256_hex(&bytes)
}

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

impl Bounds {
    /// The longest `timeout_ms` the adapter admits, derived once from the
    /// bounds: a wait's last scan may run one full request bound past its
    /// cutoff, and that must still end inside the operation ceiling.
    pub fn max_wait(&self) -> Duration {
        self.operation.saturating_sub(self.request)
    }

    pub fn max_wait_ms(&self) -> u64 {
        self.max_wait().as_millis() as u64
    }
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
pub const SCAN_PAGE: u64 = 250;

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
    /// A short fixed reason, for records that keep no prose.
    pub fn reason(self) -> &'static str {
        match self {
            ToolError::NotFound => "not_found",
            ToolError::Upstream(_) => "upstream_status",
            ToolError::Transport(r) | ToolError::Budget(r) => r,
            ToolError::Malformed => "malformed",
            ToolError::Incomplete => "incomplete",
            ToolError::ReplyTooLarge => "reply_too_large",
        }
    }

    pub fn text(self, upstream: &str) -> String {
        match self {
            ToolError::NotFound => format!("not_found: {upstream} has no message with that id"),
            ToolError::Upstream(code) => {
                format!("upstream_status: {upstream} answered HTTP {code}")
            }
            ToolError::Transport(r) => format!("transport: {r}"),
            ToolError::Malformed => {
                format!("malformed: {upstream}'s answer did not have the documented shape")
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

pub fn uint(
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

pub fn text(args: &Map<String, Value>, key: &str, max: usize) -> Option<String> {
    args.get(key)?
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max)
        .map(str::to_owned)
}

/// One operation's aggregate allowance: requests, bytes and pages across
/// all its HTTP exchanges, under one absolute deadline.
pub struct Budget<'a> {
    pub http: &'a Http,
    pub bounds: &'a Bounds,
    pub deadline: Instant,
    pub requests: u32,
    pub bytes: u64,
}

impl<'a> Budget<'a> {
    pub fn new(http: &'a Http, bounds: &'a Bounds, deadline: Instant) -> Budget<'a> {
        Budget {
            http,
            bounds,
            deadline,
            requests: 0,
            bytes: 0,
        }
    }

    pub async fn get(&mut self, path: &str) -> Result<(u16, Value), ToolError> {
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

/// Every match of a search, paged to completion: `path(start)` names one
/// page and `parse` reads it into `(matching count, [(id, message)])`,
/// refusing a page of the wrong shape. The scan ends exactly when the pages
/// reach the matching count, with every ID distinct, every page before the
/// last full and the count unchanged; anything else is incomplete, never a
/// shorter answer. A complete walk is not a snapshot: equal-count churn
/// between pages can pass these checks.
pub async fn complete_scan<P, R>(
    b: &mut Budget<'_>,
    path: P,
    parse: R,
) -> Result<(u64, Vec<Value>), ToolError>
where
    P: Fn(u64) -> String,
    R: Fn(Value, u64) -> Result<(u64, Vec<(String, Value)>), ToolError>,
{
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut expected = None;
    for _ in 0..b.bounds.max_pages {
        // The next offset is what was received, never a fixed stride, so a
        // short page cannot silently skip the matches it left out.
        let start = out.len() as u64;
        let (_, v) = b.get(&path(start)).await?;
        let (matching, items) = parse(v, start)?;
        if *expected.get_or_insert(matching) != matching {
            return Err(ToolError::Incomplete);
        }
        let got = items.len();
        for (id, m) in items {
            if !seen.insert(id) {
                return Err(ToolError::Incomplete);
            }
            out.push(m);
        }
        match (out.len() as u64).cmp(&matching) {
            std::cmp::Ordering::Equal => return Ok((matching, out)),
            std::cmp::Ordering::Greater => return Err(ToolError::Incomplete),
            // A short page before the last is not progress either.
            std::cmp::Ordering::Less if (got as u64) < SCAN_PAGE => {
                return Err(ToolError::Incomplete);
            }
            std::cmp::Ordering::Less => {}
        }
    }
    Err(ToolError::Budget("pages per scan"))
}

/// One tool call's time frame, fixed at its admission (queue time
/// included, spec C2): `ceiling` ends the whole operation and is never
/// replaced; a wait's observations count only up to `cutoff`, and its
/// last scan may finish by `response`, which never passes the ceiling.
/// This is the only constructor of a frame, for the adapter and for the
/// wire driver alike: it holds `cutoff + request <= ceiling`, so the
/// response allowance is always the full request bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub ceiling: Instant,
    pub cutoff: Instant,
    pub response: Instant,
}

impl Frame {
    /// A `wait` beyond `bounds.max_wait()` is a caller error: argument
    /// validation refuses it before a frame exists, so it is never clamped.
    pub fn new(admitted: Instant, bounds: &Bounds, wait: Duration) -> Frame {
        assert!(
            wait <= bounds.max_wait(),
            "a wait of {wait:?} cannot keep a {:?} response allowance inside {:?}",
            bounds.request,
            bounds.operation
        );
        let ceiling = admitted + bounds.operation;
        let cutoff = admitted + wait;
        Frame {
            ceiling,
            cutoff,
            response: cutoff + bounds.request,
        }
    }
}

/// One complete scan for a wait, and the HTTP requests spent so far.
pub trait Scan {
    async fn scan(&mut self) -> Result<(u64, Vec<Value>), ToolError>;
    fn requests(&self) -> u32;
    /// The label the wait answers with (a poll is always labelled).
    fn method(&self) -> &'static str {
        "client-side poll"
    }
}

/// The labelled common-client poll (proposed decision 0093 §4–5): a scan
/// at once, then one `interval` after each completed scan while before
/// the cutoff. `found` needs a complete scan that matched by the cutoff;
/// a match first seen by a scan that ended later is `late`, censored for
/// the requested interval. `timeout` is answered only at or after the
/// cutoff and means no complete scan saw a match, not continuous absence.
/// No scan starts at or after the cutoff; a scan still running at
/// `response` is incomplete, an error like every other failed scan.
pub async fn poll_wait<S: Scan>(
    s: &mut S,
    frame: Frame,
    interval: Duration,
) -> Result<Value, ToolError> {
    let mut scans = 0u32;
    loop {
        let (matching, messages) = tokio::time::timeout_at(frame.response, s.scan())
            .await
            .map_err(|_| ToolError::Budget("operation deadline"))??;
        scans += 1;
        let done = Instant::now();
        let status = match (matching > 0, done <= frame.cutoff) {
            (true, true) => "found",
            (true, false) => "late",
            (false, false) => "timeout",
            (false, true) => {
                // The frozen poll interval, not synchronization: the
                // sleep never reaches past the cutoff it observes.
                tokio::time::sleep_until((done + interval).min(frame.cutoff)).await;
                if Instant::now() < frame.cutoff {
                    continue;
                }
                "timeout"
            }
        };
        return Ok(json!({"status": status, "messages_count": matching,
            "messages": messages, "scans": scans, "requests": s.requests(),
            "method": s.method()}));
    }
}

/// The frame of one call and the ceiling check every backend starts with:
/// a call whose ceiling passed while it queued refuses without sending a
/// request.
pub fn begin(bounds: &Bounds, admitted: Instant, wait: Duration) -> Result<Frame, ToolError> {
    let frame = Frame::new(admitted, bounds, wait);
    if Instant::now() >= frame.ceiling {
        return Err(ToolError::Budget("operation deadline"));
    }
    Ok(frame)
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

/// A tool result: the upstream's JSON as one text block, or a finite
/// error. `max` is what is left of the reply bound.
fn tool_reply(id: Value, outcome: Result<Value, ToolError>, max: usize, upstream: &str) -> Value {
    let (body, is_error) = match outcome {
        Ok(v) => (v.to_string(), false),
        Err(e) => (json!({"error": e.text(upstream)}).to_string(), true),
    };
    // The envelope adds well under 1 KiB to the text it carries.
    let (body, is_error) = if body.len() + 1024 > max {
        (
            json!({"error": ToolError::ReplyTooLarge.text(upstream)}).to_string(),
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

/// One member of a job, in member order: an answer already known, or a
/// call still to run.
enum Slot<C> {
    Reply(Value),
    Call { id: Value, call: C },
}

/// One admission unit: a single tool call, or a batch holding at least one.
struct Job<C> {
    batch: bool,
    admitted: Instant,
    started: bool,
    slots: VecDeque<Slot<C>>,
    done: Vec<Value>,
    used: usize,
}

impl<C> Job<C> {
    fn new(batch: bool, slots: VecDeque<Slot<C>>) -> Job<C> {
        Job {
            batch,
            admitted: Instant::now(),
            started: false,
            slots,
            done: Vec::new(),
            used: 0,
        }
    }

    fn push(&mut self, v: Value) {
        self.used += v.to_string().len();
        self.done.push(v);
    }

    /// What is left of the reply bound for the next member.
    fn left(&self, bounds: &Bounds) -> usize {
        if self.batch {
            bounds.max_reply.saturating_sub(self.used)
        } else {
            bounds.max_reply
        }
    }

    /// The message to write: the array, the lone reply, or nothing.
    fn finish(self) -> Option<Value> {
        if self.batch {
            (!self.done.is_empty()).then_some(Value::Array(self.done))
        } else {
            self.done.into_iter().next()
        }
    }

    fn drop_call(&mut self, target: &Value) {
        self.slots
            .retain(|s| !matches!(s, Slot::Call { id, .. } if id == target));
    }

    fn has_call(&self, target: &Value) -> bool {
        self.slots
            .iter()
            .any(|s| matches!(s, Slot::Call { id, .. } if id == target))
    }
}

struct Running<C> {
    job: Job<C>,
    seq: u64,
    id: Value,
    task: tokio::task::JoinHandle<()>,
}

struct State<C> {
    initialized: bool,
    queue: VecDeque<Job<C>>,
    running: Option<Running<C>>,
    /// A job whose running member just ended and which carries on.
    advance: Option<Job<C>>,
}

impl<C> State<C> {
    fn busy(&self) -> usize {
        self.queue.len() + usize::from(self.running.is_some())
    }

    fn in_flight(&self, id: &Value) -> bool {
        self.running
            .as_ref()
            .is_some_and(|r| &r.id == id || r.job.has_call(id))
            || self.queue.iter().any(|j| j.has_call(id))
    }
}

/// Serve MCP on `input`/`output` until EOF or a stuck or closed output.
pub async fn serve<B, R, W>(input: R, mut output: W, http: Http, bounds: Bounds) -> End
where
    B: Backend,
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    let http = Arc::new(http);
    let (in_tx, mut in_rx) = mpsc::channel(1);
    let reader = tokio::spawn(read_lines(input, bounds.max_line, in_tx));
    let (done_tx, mut done_rx) = mpsc::channel::<(u64, Result<Value, ToolError>)>(1);
    let mut st: State<B::Call> = State {
        initialized: false,
        queue: VecDeque::new(),
        running: None,
        advance: None,
    };
    let mut seq = 0u64;
    let end = loop {
        let mut replies = Vec::new();
        tokio::select! {
            msg = in_rx.recv() => match msg {
                None | Some(Inbound::Eof) => break End::Eof,
                Some(Inbound::TooLong) => replies.push(error(Value::Null, -32600, "request line exceeds the adapter's line bound")),
                Some(Inbound::Line(line)) => {
                    if let Some(r) = handle::<B>(&line, &mut st, &bounds) {
                        replies.push(r);
                    }
                }
            },
            Some((done, outcome)) = done_rx.recv() => {
                if st.running.as_ref().is_some_and(|r| r.seq == done) {
                    let r = st.running.take().expect("checked above");
                    let mut job = r.job;
                    let reply = tool_reply(r.id, outcome, job.left(&bounds), B::UPSTREAM);
                    job.push(reply);
                    st.advance = Some(job);
                }
            }
        }
        drive::<B>(&mut st, &mut replies, &mut seq, &http, &bounds, &done_tx);
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
    st.queue.clear();
    if let Some(r) = st.running.take() {
        r.task.abort();
        let _ = tokio::time::timeout(bounds.cleanup, r.task).await;
    }
    reader.abort();
    let _ = tokio::time::timeout(bounds.cleanup, reader).await;
    end
}

/// Move jobs along: a job whose member just ended carries on, and when
/// nothing runs the next queued job starts. A finished job's message is
/// pushed to `replies`.
fn drive<B: Backend>(
    st: &mut State<B::Call>,
    replies: &mut Vec<Value>,
    seq: &mut u64,
    http: &Arc<Http>,
    bounds: &Bounds,
    done_tx: &mpsc::Sender<(u64, Result<Value, ToolError>)>,
) {
    loop {
        let mut job = match st.advance.take() {
            Some(j) => j,
            None if st.running.is_some() => return,
            None => match st.queue.pop_front() {
                Some(j) => j,
                None => return,
            },
        };
        loop {
            match job.slots.pop_front() {
                None => {
                    replies.extend(job.finish());
                    break;
                }
                Some(Slot::Reply(v)) => job.push(v),
                Some(Slot::Call { id, call }) => {
                    *seq += 1;
                    let this = *seq;
                    let admitted = if job.started {
                        Instant::now()
                    } else {
                        job.admitted
                    };
                    job.started = true;
                    let (http, bounds2, tx) = (http.clone(), bounds.clone(), done_tx.clone());
                    let task = tokio::spawn(async move {
                        let outcome = B::run_call(&http, &bounds2, &call, admitted).await;
                        let _ = tx.send((this, outcome)).await;
                    });
                    st.running = Some(Running {
                        job,
                        seq: this,
                        id,
                        task,
                    });
                    return;
                }
            }
        }
    }
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

/// What one JSON-RPC member amounts to.
enum Member<C> {
    /// A notification, or a client response: no reply.
    Nothing,
    Reply(Value),
    Call {
        id: Value,
        call: C,
    },
}

fn busy(id: Value) -> Value {
    error(id, -32000, "too many tool calls in flight; retry later")
}

/// One inbound line; the message to write now, if any. Tool calls are
/// queued here and answered when they finish.
fn handle<B: Backend>(line: &[u8], st: &mut State<B::Call>, bounds: &Bounds) -> Option<Value> {
    let Ok(msg) = serde_json::from_slice::<Value>(line) else {
        return Some(error(Value::Null, -32700, "parse error"));
    };
    match msg {
        Value::Array(members) => {
            if members.is_empty() || members.len() > MAX_BATCH {
                // A refusal, never a clamp: one error object, id null.
                let why = if members.is_empty() {
                    "an empty batch is not a JSON-RPC request".to_string()
                } else {
                    format!("a batch is at most {MAX_BATCH} requests")
                };
                return Some(error(Value::Null, -32600, &why));
            }
            let slots: VecDeque<Slot<B::Call>> = members
                .into_iter()
                .filter_map(|m| match member::<B>(m, st, bounds, true) {
                    Member::Nothing => None,
                    Member::Reply(v) => Some(Slot::Reply(v)),
                    Member::Call { id, call } => Some(Slot::Call { id, call }),
                })
                .collect();
            if slots.is_empty() {
                return None;
            }
            let calls = slots.iter().any(|s| matches!(s, Slot::Call { .. }));
            let full = st.busy() >= bounds.max_pending;
            if !calls || full {
                // Nothing to admit, or no room: answer now. A full queue
                // refuses every request member and runs nothing.
                let out = slots
                    .into_iter()
                    .map(|s| match s {
                        Slot::Reply(v) if !full || v["id"].is_null() => v,
                        Slot::Reply(v) => busy(v["id"].clone()),
                        Slot::Call { id, .. } => busy(id),
                    })
                    .collect();
                return Some(Value::Array(out));
            }
            st.queue.push_back(Job::new(true, slots));
            None
        }
        single => match member::<B>(single, st, bounds, false) {
            Member::Nothing => None,
            Member::Reply(v) => Some(v),
            Member::Call { id, call } => {
                if st.busy() >= bounds.max_pending {
                    return Some(busy(id));
                }
                st.queue
                    .push_back(Job::new(false, VecDeque::from([Slot::Call { id, call }])));
                None
            }
        },
    }
}

fn member<B: Backend>(
    msg: Value,
    st: &mut State<B::Call>,
    bounds: &Bounds,
    in_batch: bool,
) -> Member<B::Call> {
    let Value::Object(m) = msg else {
        return Member::Reply(error(Value::Null, -32600, "invalid request"));
    };
    let id = m.get("id").cloned();
    let id_ok = matches!(&id, Some(Value::String(_))) || id.as_ref().is_some_and(Value::is_i64);
    if m.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || (id.is_some() && !id_ok) {
        return Member::Reply(error(
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
            .map_or(Member::Nothing, |id| {
                Member::Reply(error(id, -32600, "invalid request"))
            });
    };
    let params = m.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        // Notifications never get a reply.
        if method == "notifications/cancelled"
            && let Some(t) = params.get("requestId")
        {
            // Only the named member is cancelled: the running one ends
            // and its job carries on; a waiting one is dropped.
            if st.running.as_ref().is_some_and(|r| &r.id == t) {
                let r = st.running.take().expect("checked above");
                r.task.abort();
                st.advance = Some(r.job);
            }
            if let Some(r) = st.running.as_mut() {
                r.job.drop_call(t);
            }
            for j in st.queue.iter_mut() {
                j.drop_call(t);
            }
        }
        return Member::Nothing;
    };
    let reply = Member::Reply;
    match method {
        "initialize" => {
            if in_batch {
                return reply(error(id, -32600, "initialize cannot be part of a batch"));
            }
            if st.initialized {
                return reply(error(id, -32600, "already initialized"));
            }
            if params.get("protocolVersion").and_then(Value::as_str) != Some(PROTOCOL_REVISION) {
                return reply(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602,
                    "message": "unsupported protocol version",
                    "data": {"supported": [PROTOCOL_REVISION]}}}));
            }
            st.initialized = true;
            reply(result(
                id,
                json!({"protocolVersion": PROTOCOL_REVISION,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": B::SERVER_NAME, "version": env!("CARGO_PKG_VERSION")}}),
            ))
        }
        "ping" => reply(result(id, json!({}))),
        _ if !st.initialized => reply(error(id, -32600, "not initialized")),
        "tools/list" => reply(result(id, json!({"tools": B::tools(bounds)}))),
        "tools/call" => {
            if st.in_flight(&id) {
                return reply(error(id, -32600, "a request with this id is in flight"));
            }
            match B::parse_call(&params, bounds) {
                None => reply(error(id, -32602, "invalid tool name or arguments")),
                Some(call) => Member::Call { id, call },
            }
        }
        _ => reply(error(id, -32601, "method not found")),
    }
}

/// What launch mode needs from the launcher's limits: the frozen protocol
/// v2 resource deadlines, which a test holds to `docs/evals/protocol.json`.
pub fn launch_limits() -> crate::targets::Limits {
    crate::targets::Limits {
        startup: Duration::from_millis(30_000),
        cleanup: Duration::from_millis(10_000),
        request: Duration::from_millis(10_000),
        max_response_bytes: 16 << 20,
        open_files: 4096,
    }
}

/// Launch mode's core (proposed decision 0129 §4): announce the owned
/// backend's SMTP address and this adapter's tool-list digest as the first
/// line of output, serve MCP until the input ends, then run the owner's full
/// cleanup. The backend is never left running: whatever ended the serving,
/// its cleanup is run and its facts returned. Only an orderly end with every
/// cleanup fact true is a receipt (`receipt`).
pub async fn serve_owned<B, R, W>(
    owned: &mut crate::targets::Owned,
    input: R,
    mut output: W,
    bounds: Bounds,
) -> (End, crate::targets::CleanupFacts)
where
    B: Backend,
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    let announce = format!(
        "smtp={} tools_sha256={}\n",
        owned.smtp,
        tools_sha256::<B>(&bounds)
    );
    let written = async {
        output.write_all(announce.as_bytes()).await?;
        output.flush().await
    };
    let end = match tokio::time::timeout(bounds.cleanup, written).await {
        Ok(Ok(())) => {
            let http = Http::new(owned.http_addr, bounds.request, bounds.max_body);
            serve::<B, _, _>(input, output, http, bounds).await
        }
        Ok(Err(_)) => End::OutputClosed,
        Err(_) => End::OutputBlocked,
    };
    (end, owned.shutdown().await)
}

/// Whether an adapter's end is a cleanup receipt: it served to an orderly
/// end of input and every cleanup fact is true.
pub fn receipt(end: End, facts: &crate::targets::CleanupFacts) -> bool {
    end == End::Eof && facts.proved()
}
