//! The thin MailSlurper MCP adapter (proposed decisions 0092 §3 and 0129
//! §5): the Mailpit and MailHog adapters' shape over MailSlurper's own
//! service API — list, search, read and a labelled common-client poll over
//! search. It offers only operations MailSlurper serves: nothing is emulated
//! that it lacks (no per-test mailbox, no blocking wait, no extraction, cache
//! or hint), and no delete, prune, attachment, login or version operation is
//! reachable. Results are MailSlurper's own JSON in text content blocks; the
//! stored body passes through unchanged.
//!
//! Routes, and what binds each: the captured source files the source lock
//! binds under `docs/evals/sources/mailslurper/` (read from the release-1.15.0
//! source archive by digest; the tree is unresolved).
//! - `GET /mail?pageNumber=N` with the optional `to`, `from` and `message`
//!   filters: `ServiceController.go` `GetMailCollection` (a page is a fixed 50
//!   items, hard coded) and `setupServicesListener.go` (the route).
//! - Each filter is a SQL `LIKE '%term%'` substring match, the term wrapped in
//!   `%` and not escaped: `CommonQueries.go` `addSearchCriteria`. `message`
//!   matches the body or the subject, `to` the stored recipient list. The
//!   adapter neither narrows nor escapes a term: that is MailSlurper's real
//!   filter, and the tool descriptions say so.
//! - `GET /mail/{id}`: `ServiceController.go` `GetMail`. An unknown id is not
//!   a 404: the pinned executable answers HTTP 200 with an empty mail item,
//!   whose `id` is the empty string, and the adapter reports that as not found.
//!
//! The pinned executable is the one built from source (the lock's
//! `built_from_source` artifact); the published linux executable cannot start.
//! The ignored real-executable test calls every tool once against it, and the
//! committed wire corpus holds its responses, decoded in CI.

use crate::mcp::{Backend, Budget, begin, complete_scan};
use crate::wire::{Http, path_segment, query_value};
use serde_json::{Map, Value, json};
use std::time::Duration;
use tokio::time::Instant;

use crate::mailpit_mcp::{Bounds, MAX_QUERY_BYTES, Scan, ToolError, id_arg, poll_wait, uint};

pub const SERVER_NAME: &str = "mail-compare-mailslurper-mcp";
pub const WAIT_METHOD: &str = "client-side poll of /mail";

/// MailSlurper's fixed page size (`GetMailCollection`: `length := 50`).
pub const PAGE_SIZE: u64 = 50;
/// The adapter's own ceiling on `pageNumber`, equal to the other adapters'
/// 100,000-message offset ceiling at 50 per page. It is not MailSlurper's.
pub const MAX_PAGE: u64 = 2000;
/// Characters beyond `[A-Za-z0-9._-]` MailSlurper's ids need: none. The wire
/// corpus is the witness: its ids are lowercase hex groups joined by `-`
/// (`c2eefc7a-3bf7-4fa9-7116-28082e7e4f23`).
pub const MAILSLURPER_ID_EXTRA: &[u8] = b"";

/// The MailSlurper backend of the generic server.
pub struct Mailslurper;

impl Backend for Mailslurper {
    type Call = Call;
    const UPSTREAM: &'static str = "MailSlurper";
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

/// The filters of MailSlurper's list route; at least one is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub to: Option<String>,
    pub from: Option<String>,
    pub message: Option<String>,
}

impl Filter {
    fn query(&self) -> String {
        let mut q = String::new();
        for (key, term) in [
            ("to", &self.to),
            ("from", &self.from),
            ("message", &self.message),
        ] {
            if let Some(t) = term {
                q.push_str(&format!("&{key}={}", query_value(t)));
            }
        }
        q
    }
}

/// A validated tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    List { page: u64 },
    Search { filter: Filter, page: u64 },
    Read { id: String },
    Wait { filter: Filter, timeout_ms: u64 },
}

pub fn tools(bounds: &Bounds) -> Value {
    let page = json!({"type": "integer", "minimum": 1, "maximum": MAX_PAGE, "default": 1,
        "description": "MailSlurper's pageNumber. A page holds 50 items, MailSlurper's fixed size; \
            there is no page-size argument."});
    let term = |what: &str| {
        json!({"type": "string", "minLength": 1, "maxLength": MAX_QUERY_BYTES,
            "description": what})
    };
    let to = term(
        "MailSlurper's to filter, passed through unchanged: a SQL LIKE substring match against \
         the stored recipient list.",
    );
    let from = term(
        "MailSlurper's from filter, passed through unchanged: a SQL LIKE substring match against \
         the stored from address.",
    );
    let message = term(
        "MailSlurper's message filter, passed through unchanged: a SQL LIKE substring match \
         against a mail's body or its subject.",
    );
    json!([
        {
            "name": "mailslurper_list",
            "description": "One page of MailSlurper's GET /mail, as MailSlurper returns it: \
                mailItems (each with its addresses, subject, body and content type as stored), \
                totalPages and totalRecords. MailSlurper has one shared store; it does not \
                separate mail by SMTP identity.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "properties": {"page": page}}
        },
        {
            "name": "mailslurper_search",
            "description": "One page of MailSlurper's GET /mail with at least one of the to, from \
                and message filters. Each given term is a SQL LIKE substring match: MailSlurper \
                wraps the term in % on both sides and does not escape it, so a term matches \
                anywhere inside the stored value (for to, anywhere inside the stored recipient \
                list, so a longer address that contains the term also matches), and a % or _ \
                inside a term acts as a LIKE wildcard. totalRecords is the number of matches.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "properties": {"to": to, "from": from, "message": message, "page": page}}
        },
        {
            "name": "mailslurper_read",
            "description": "One stored mail by id, from MailSlurper's GET /mail/{id}, as \
                MailSlurper returns it. MailSlurper answers a lookup that fails, an unknown id \
                included, with HTTP 500, not 404.",
            "inputSchema": {"type": "object", "additionalProperties": false, "required": ["id"],
                "properties": {"id": {"type": "string", "minLength": 1,
                    "maxLength": crate::mcp::MAX_ID_CHARS,
                    "pattern": "^[A-Za-z0-9._-]+$"}}}
        },
        {
            "name": "mailslurper_wait",
            "description": "Client-side polling, not a MailSlurper wait facility: a complete scan \
                of GET /mail with the given filters at once, then 100 ms after each completed \
                scan, until a scan matches or timeout_ms (counted from when the call was \
                accepted) passes. The filters are MailSlurper's LIKE substring matches. Answers \
                found (a scan that ended in time matched), late (the first match was seen by a \
                scan that ended after timeout_ms) or timeout (no scan saw a match: not proof \
                that none existed meanwhile), with every match and the scans and HTTP requests \
                taken.",
            "inputSchema": {"type": "object", "additionalProperties": false,
                "properties": {"to": to, "from": from, "message": message,
                    "timeout_ms": {"type": "integer", "minimum": 1,
                        "maximum": bounds.max_wait_ms()}},
                "required": ["timeout_ms"]}
        }
    ])
}

/// A filter term: present terms are non-blank (MailSlurper itself ignores a
/// blank one, which would silently stop filtering) and bounded.
fn term(args: &Map<String, Value>, key: &str) -> Result<Option<String>, ()> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= MAX_QUERY_BYTES)
            .map(|s| Some(s.to_owned()))
            .ok_or(()),
    }
}

fn filter(args: &Map<String, Value>) -> Option<Filter> {
    let f = Filter {
        to: term(args, "to").ok()?,
        from: term(args, "from").ok()?,
        message: term(args, "message").ok()?,
    };
    (f.to.is_some() || f.from.is_some() || f.message.is_some()).then_some(f)
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
        "mailslurper_list" => &["page"],
        "mailslurper_search" => &["to", "from", "message", "page"],
        "mailslurper_read" => &["id"],
        "mailslurper_wait" => &["to", "from", "message", "timeout_ms"],
        _ => return None,
    };
    if args.keys().any(|k| !allowed.contains(&k.as_str())) {
        return None;
    }
    let page = || uint(args, "page", Some(1), 1, MAX_PAGE);
    Some(match name {
        "mailslurper_list" => Call::List { page: page()? },
        "mailslurper_search" => Call::Search {
            filter: filter(args)?,
            page: page()?,
        },
        "mailslurper_read" => Call::Read {
            id: id_arg(args, "id", MAILSLURPER_ID_EXTRA)?,
        },
        _ => Call::Wait {
            filter: filter(args)?,
            timeout_ms: uint(args, "timeout_ms", None, 1, bounds.max_wait_ms())?,
        },
    })
}

/// The total, the `(id, item)` pairs and the page body as served.
type Page = (u64, Vec<(String, Value)>, Map<String, Value>);

/// A MailSlurper list or search page, checked against the shape the source
/// declares (`MailCollectionResponse`): integer `totalRecords` and
/// `totalPages`, a `mailItems` array of at most one page, every item with a
/// non-empty string `id`, and no more items than the total says exist.
pub fn page(v: Value) -> Result<Page, ToolError> {
    let Value::Object(mut page) = v else {
        return Err(ToolError::Malformed);
    };
    let n = |k: &str| page.get(k).and_then(Value::as_u64);
    let (Some(total), Some(_pages)) = (n("totalRecords"), n("totalPages")) else {
        return Err(ToolError::Malformed);
    };
    let items = match page.get("mailItems") {
        Some(Value::Array(items)) => items.clone(),
        // A nil Go slice is encoded as `null`, and the operator observed real
        // MailSlurper leaving `mailItems` out of an empty page (decision
        // 0118 §4); the captured struct has no `omitempty`, so the shape is
        // read as it comes.
        Some(Value::Null) | None => Vec::new(),
        _ => return Err(ToolError::Malformed),
    };
    if items.len() as u64 > PAGE_SIZE || items.len() as u64 > total {
        return Err(ToolError::Malformed);
    }
    let mut ids = Vec::with_capacity(items.len());
    for m in items {
        match m.get("id").and_then(Value::as_str) {
            Some(id) if !id.is_empty() => ids.push((id.to_owned(), m)),
            _ => return Err(ToolError::Malformed),
        }
    }
    page.insert(
        "mailItems".into(),
        Value::Array(ids.iter().map(|(_, m)| m.clone()).collect()),
    );
    Ok((total, ids, page))
}

fn list_path(page: u64, filter: Option<&Filter>) -> String {
    format!(
        "/mail?pageNumber={page}{}",
        filter.map(Filter::query).unwrap_or_default()
    )
}

async fn complete_search(
    b: &mut Budget<'_>,
    filter: &Filter,
) -> Result<(u64, Vec<Value>), ToolError> {
    complete_scan(
        b,
        PAGE_SIZE,
        |start| list_path(start / PAGE_SIZE + 1, Some(filter)),
        |v, _| {
            let (total, ids, _) = page(v)?;
            Ok((total, ids))
        },
    )
    .await
}

struct SearchScan<'a> {
    budget: Budget<'a>,
    filter: &'a Filter,
}

impl Scan for SearchScan<'_> {
    async fn scan(&mut self) -> Result<(u64, Vec<Value>), ToolError> {
        complete_search(&mut self.budget, self.filter).await
    }
    fn requests(&self) -> u32 {
        self.budget.requests
    }
    fn method(&self) -> &'static str {
        WAIT_METHOD
    }
}

/// Run one validated call against MailSlurper within `bounds`, its frame
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
        Call::List { page: n } => {
            let (_, v) = b.get(&list_path(*n, None)).await?;
            Ok(Value::Object(page(v)?.2))
        }
        Call::Search { filter, page: n } => {
            let (_, v) = b.get(&list_path(*n, Some(filter))).await?;
            Ok(Value::Object(page(v)?.2))
        }
        Call::Read { id } => {
            let (_, v) = b.get(&format!("/mail/{}", path_segment(id))).await?;
            match v.get("id").and_then(Value::as_str) {
                Some(got) if got == id => Ok(v),
                // The pinned executable answers an unknown id with HTTP 200
                // and an empty mail item (`"id": ""`), not an error status
                // (`docs/evals/wire-corpus/mailslurper/read-not-found.json`).
                Some("") => Err(ToolError::NotFound),
                _ => Err(ToolError::Malformed),
            }
        }
        Call::Wait { filter, .. } => {
            b.deadline = frame.response;
            let mut scan = SearchScan { budget: b, filter };
            poll_wait(&mut scan, frame, bounds.poll_interval).await
        }
    }
}
