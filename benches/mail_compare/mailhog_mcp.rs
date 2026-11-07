//! The thin MailHog MCP adapter (proposed decisions 0092 §3 and 0129 §5): the
//! Mailpit adapter's shape over MailHog's own HTTP API — list, search, read
//! and a labelled common-client poll over search. It offers only operations
//! MailHog serves: nothing is emulated that it lacks (no per-test mailbox,
//! no blocking wait, no extraction, cache or hint) and no admin, send,
//! release or delete operation is reachable. Results are MailHog's own JSON
//! in text content blocks; a message's MIME stays raw.
//!
//! Routes, and what binds each:
//! - `GET /api/v2/messages?start&limit` and
//!   `GET /api/v2/search?kind=from|to|containing&query&start&limit`: the
//!   captured Swagger document `sources/mailhog/mailhog-swagger.json`, which
//!   the lock binds by size, digest and blob id. The pinned executable
//!   answers with `items` (each carrying `ID`) where the document says
//!   `messages`; the adapter reads what the executable sends, the same
//!   reading the wire driver makes (`targets.rs`), and the README lists the
//!   disagreement.
//! - `GET /api/v1/messages/{id}`: not in any captured document. The wire
//!   driver observed it on the pinned executable and uses it for its
//!   fidelity read; its attribution to MailHog's source is **open** (no
//!   reviewed source file here states it). It is the only route to one
//!   message's stored bytes, so the adapter offers it, labelled.

use crate::mcp::{Backend, Budget, begin, complete_scan};
use crate::wire::{Http, query_value};
use serde_json::{Map, Value, json};
use std::time::Duration;
use tokio::time::Instant;

use crate::mailpit_mcp::{
    Bounds, DEFAULT_LIMIT, MAX_ID_BYTES, MAX_LIMIT, MAX_QUERY_BYTES, MAX_START, SCAN_PAGE, Scan,
    ToolError, poll_wait, text, uint,
};

pub const SERVER_NAME: &str = "mail-compare-mailhog-mcp";
pub const WAIT_METHOD: &str = "client-side poll of /api/v2/search";

/// The MailHog backend of the generic server.
pub struct Mailhog;

impl Backend for Mailhog {
    type Call = Call;
    const UPSTREAM: &'static str = "MailHog";
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

/// MailHog's `kind` query parameter, which its search requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    From,
    To,
    Containing,
}

impl Kind {
    fn parse(s: &str) -> Option<Kind> {
        match s {
            "from" => Some(Kind::From),
            "to" => Some(Kind::To),
            "containing" => Some(Kind::Containing),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::From => "from",
            Kind::To => "to",
            Kind::Containing => "containing",
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
        kind: Kind,
        query: String,
        start: u64,
        limit: u64,
    },
    Read {
        id: String,
    },
    Wait {
        kind: Kind,
        query: String,
        timeout_ms: u64,
    },
}

pub fn tools(bounds: &Bounds) -> Value {
    let start = json!({"type": "integer", "minimum": 0, "maximum": MAX_START, "default": 0,
        "description": "Pagination offset, as MailHog's start parameter."});
    let limit = json!({"type": "integer", "minimum": 1, "maximum": MAX_LIMIT, "default": DEFAULT_LIMIT,
        "description": "Page size. 250 is this adapter's ceiling, not MailHog's."});
    let kind = json!({"type": "string", "enum": ["from", "to", "containing"],
        "description": "MailHog's search kind, passed through unchanged."});
    let query = json!({"type": "string", "minLength": 1, "maxLength": MAX_QUERY_BYTES,
        "description": "A MailHog search query, passed through unchanged."});
    json!([
        {
            "name": "mailhog_list",
            "description": "One page of MailHog's GET /api/v2/messages, as MailHog returns it: total, \
                start, count and the stored messages (items), each with its headers, body and raw \
                MIME exactly as MailHog stores them. MailHog has one shared message store; it does \
                not separate mail by SMTP identity.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "properties": {"start": start, "limit": limit}}
        },
        {
            "name": "mailhog_search",
            "description": "One page of MailHog's GET /api/v2/search, as MailHog returns it. kind says \
                what the query matches (from, to or containing); total is the number of matches.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "required": ["kind", "query"],
                "properties": {"kind": kind, "query": query, "start": start, "limit": limit}}
        },
        {
            "name": "mailhog_read",
            "description": "One stored message by ID, from GET /api/v1/messages/{ID}, as MailHog returns \
                it (headers, body and raw MIME). This route is observed on the pinned executable and \
                is not in MailHog's captured API document.",
            "inputSchema": {"type": "object", "additionalProperties": false, "required": ["id"],
                "properties": {"id": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}}}
        },
        {
            "name": "mailhog_wait",
            "description": "Client-side polling, not a MailHog wait facility: a complete scan of \
                GET /api/v2/search at once, then 100 ms after each completed scan, until a scan \
                matches or timeout_ms (counted from when the call was accepted) passes. Answers \
                found (a scan that ended in time matched), late (the first match was seen by a \
                scan that ended after timeout_ms) or timeout (no scan saw a match: not proof that \
                none existed meanwhile), with every match and the scans and HTTP requests taken.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "required": ["kind", "query", "timeout_ms"],
                "properties": {"kind": kind, "query": query,
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
    let name = p.get("name")?.as_str()?;
    let allowed: &[&str] = match name {
        "mailhog_list" => &["start", "limit"],
        "mailhog_search" => &["kind", "query", "start", "limit"],
        "mailhog_read" => &["id"],
        "mailhog_wait" => &["kind", "query", "timeout_ms"],
        _ => return None,
    };
    if args.keys().any(|k| !allowed.contains(&k.as_str())) {
        return None;
    }
    let start = || uint(args, "start", Some(0), 0, MAX_START);
    let limit = || uint(args, "limit", Some(DEFAULT_LIMIT), 1, MAX_LIMIT);
    let kind = || Kind::parse(args.get("kind")?.as_str()?);
    Some(match name {
        "mailhog_list" => Call::List {
            start: start()?,
            limit: limit()?,
        },
        "mailhog_search" => Call::Search {
            kind: kind()?,
            query: text(args, "query", MAX_QUERY_BYTES)?,
            start: start()?,
            limit: limit()?,
        },
        "mailhog_read" => Call::Read {
            id: text(args, "id", MAX_ID_BYTES)?,
        },
        _ => Call::Wait {
            kind: kind()?,
            query: text(args, "query", MAX_QUERY_BYTES)?,
            timeout_ms: uint(args, "timeout_ms", None, 1, bounds.max_wait_ms())?,
        },
    })
}

/// The total, the `(ID, item)` pairs and the page body as served.
type Page = (u64, Vec<(String, Value)>, Map<String, Value>);

/// A MailHog list or search page, checked against the shape the pinned
/// executable sends: counts are integers, `start` echoes the offset asked
/// for, every item carries a string `ID`, and `count` (when present)
/// agrees with the items.
fn page(v: Value, start: u64) -> Result<Page, ToolError> {
    let Value::Object(mut page) = v else {
        return Err(ToolError::Malformed);
    };
    let n = |k: &str| page.get(k).and_then(Value::as_u64);
    let (Some(total), Some(echo)) = (n("total"), n("start")) else {
        return Err(ToolError::Malformed);
    };
    let items = match page.get("items") {
        Some(Value::Array(items)) => items.clone(),
        // MailHog serialises an empty result as `null`.
        Some(Value::Null) => Vec::new(),
        _ => return Err(ToolError::Malformed),
    };
    let mut ids = Vec::with_capacity(items.len());
    for m in items {
        match m.get("ID").and_then(Value::as_str) {
            Some(id) if !id.is_empty() => ids.push((id.to_owned(), m)),
            _ => return Err(ToolError::Malformed),
        }
    }
    if echo != start
        || page
            .get("count")
            .is_some_and(|c| c.as_u64() != Some(ids.len() as u64))
    {
        return Err(ToolError::Malformed);
    }
    page.insert("count".into(), (ids.len() as u64).into());
    page.insert(
        "items".into(),
        Value::Array(ids.iter().map(|(_, m)| m.clone()).collect()),
    );
    Ok((total, ids, page))
}

fn search_path(kind: Kind, query: &str, start: u64, limit: u64) -> String {
    format!(
        "/api/v2/search?kind={}&query={}&start={start}&limit={limit}",
        kind.name(),
        query_value(query)
    )
}

async fn complete_search(
    b: &mut Budget<'_>,
    kind: Kind,
    query: &str,
) -> Result<(u64, Vec<Value>), ToolError> {
    complete_scan(
        b,
        |start| search_path(kind, query, start, SCAN_PAGE),
        |v, start| {
            let (total, ids, _) = page(v, start)?;
            Ok((total, ids))
        },
    )
    .await
}

struct SearchScan<'a> {
    budget: Budget<'a>,
    kind: Kind,
    query: &'a str,
}

impl Scan for SearchScan<'_> {
    async fn scan(&mut self) -> Result<(u64, Vec<Value>), ToolError> {
        complete_search(&mut self.budget, self.kind, self.query).await
    }
    fn requests(&self) -> u32 {
        self.budget.requests
    }
    fn method(&self) -> &'static str {
        WAIT_METHOD
    }
}

/// Run one validated call against MailHog within `bounds`, its frame
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
                .get(&format!("/api/v2/messages?start={start}&limit={limit}"))
                .await?;
            Ok(Value::Object(page(v, *start)?.2))
        }
        Call::Search {
            kind,
            query,
            start,
            limit,
        } => {
            let (_, v) = b.get(&search_path(*kind, query, *start, *limit)).await?;
            Ok(Value::Object(page(v, *start)?.2))
        }
        Call::Read { id } => {
            let (_, v) = b
                .get(&format!("/api/v1/messages/{}", query_value(id)))
                .await?;
            // MailHog answers an unknown id with `200 null`.
            if v.is_null() {
                return Err(ToolError::NotFound);
            }
            if v.get("ID").and_then(Value::as_str) != Some(id.as_str()) {
                return Err(ToolError::Malformed);
            }
            Ok(v)
        }
        Call::Wait { kind, query, .. } => {
            b.deadline = frame.response;
            let mut scan = SearchScan {
                budget: b,
                kind: *kind,
                query,
            };
            poll_wait(&mut scan, frame, bounds.poll_interval).await
        }
    }
}
