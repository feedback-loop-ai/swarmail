//! The thin Mailpit MCP adapter (spec C2, proposed decision 0092 §3): an
//! MCP 2025-03-26 stdio server whose four tools translate one-to-one onto
//! Mailpit's own documented HTTP API — list, search, read, and a labelled
//! common-client poll over search. It adds nothing Mailpit does not serve:
//! no admin, send, delete or release operations, no task hints, no
//! extraction, cache, index or prefetch, no inboxes Mailpit lacks and no
//! synthetic chaos. Results are Mailpit's own JSON in text content blocks.
//!
//! This is the Mailpit backend of the generic server in `mcp.rs` (bounds,
//! admission, framing, the poll wait and batches live there). The caller
//! (the eval owner) starts Mailpit and hands the adapter its loopback
//! endpoint, or the adapter launches it.

use crate::mcp::{Backend, Budget, begin};
use crate::wire::{Http, query_value};
use serde_json::{Map, Value, json};
use std::time::Duration;
use tokio::time::Instant;

pub use crate::mcp::{
    Bounds, DEFAULT_LIMIT, End, Frame, MAX_ID_BYTES, MAX_LIMIT, MAX_QUERY_BYTES, MAX_START,
    SCAN_PAGE, Scan, ToolError, parse_endpoint, poll_wait, text, uint,
};
pub const SERVER_NAME: &str = "mail-compare-mailpit-mcp";

/// The Mailpit backend of the generic server.
pub struct Mailpit;

impl Backend for Mailpit {
    type Call = Call;
    const UPSTREAM: &'static str = "Mailpit";
    const SERVER_NAME: &'static str = SERVER_NAME;

    fn tools(bounds: &Bounds) -> Value {
        tools(bounds)
    }

    fn parse_call(params: &Value, bounds: &Bounds) -> Option<Call> {
        parse_call(params, bounds)
    }

    async fn run_call(
        http: &Http,
        bounds: &Bounds,
        call: &Call,
        admitted: Instant,
    ) -> Result<Value, ToolError> {
        run_call(http, bounds, call, admitted).await
    }
}

/// Serve the Mailpit adapter on `input`/`output`.
pub async fn serve<R, W>(input: R, output: W, http: Http, bounds: Bounds) -> End
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin,
{
    crate::mcp::serve::<Mailpit, R, W>(input, output, http, bounds).await
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

pub fn tools(bounds: &Bounds) -> Value {
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
                GET /api/v1/search at once, then 100 ms after each completed scan, until a scan \
                matches or timeout_ms (counted from when the call was accepted) passes. Answers \
                found (a scan that ended in time matched), late (the first match was seen by a \
                scan that ended after timeout_ms) or timeout (no scan saw a match: not proof that \
                none existed meanwhile), with every match and the scans and HTTP requests taken.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "required": ["query", "timeout_ms"],
                "properties": {"query": query,
                    "timeout_ms": {"type": "integer", "minimum": 1,
                        "maximum": bounds.max_wait_ms()}}}
        }
    ])
}

/// Validate `tools/call` params; `None` refuses with invalid params.
pub fn parse_call(params: &Value, bounds: &Bounds) -> Option<Call> {
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
            timeout_ms: uint(args, "timeout_ms", None, 1, bounds.max_wait_ms())?,
        },
    })
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
/// when the pages reach messages_count, with every ID distinct, every
/// page before the last full and the count unchanged; anything else is
/// incomplete, never a shorter answer. A complete walk is not a snapshot:
/// equal-count churn between pages can pass these checks.
async fn complete_search(b: &mut Budget<'_>, query: &str) -> Result<(u64, Vec<Value>), ToolError> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut expected = None;
    for _ in 0..b.bounds.max_pages {
        // The next offset is what was received, never a fixed stride, so a
        // short page cannot silently skip the matches it left out.
        let start = out.len() as u64;
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
            // A short page before the last is not progress either.
            std::cmp::Ordering::Less if (got as u64) < SCAN_PAGE => {
                return Err(ToolError::Incomplete);
            }
            std::cmp::Ordering::Less => {}
        }
    }
    Err(ToolError::Budget("pages per scan"))
}

struct SearchScan<'a> {
    budget: Budget<'a>,
    query: &'a str,
}

impl Scan for SearchScan<'_> {
    async fn scan(&mut self) -> Result<(u64, Vec<Value>), ToolError> {
        complete_search(&mut self.budget, self.query).await
    }
    fn requests(&self) -> u32 {
        self.budget.requests
    }
    fn method(&self) -> &'static str {
        WAIT_METHOD
    }
}

pub const WAIT_METHOD: &str = "client-side poll of /api/v1/search";

/// Run one validated call against Mailpit within `bounds`, its frame
/// starting at `admitted`. A call whose ceiling passed while it queued
/// refuses without sending a request.
pub async fn run_call(
    http: &Http,
    bounds: &Bounds,
    call: &Call,
    admitted: Instant,
) -> Result<Value, ToolError> {
    let wait = match call {
        Call::Wait { timeout_ms, .. } => Duration::from_millis(*timeout_ms),
        _ => Duration::ZERO,
    };
    let frame = begin(bounds, admitted, wait)?;
    let mut b = Budget::new(http, bounds, frame.ceiling);
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
        Call::Wait { query, .. } => {
            b.deadline = frame.response;
            let mut scan = SearchScan { budget: b, query };
            poll_wait(&mut scan, frame, bounds.poll_interval).await
        }
    }
}
