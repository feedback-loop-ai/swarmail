//! Model Context Protocol (MCP) server.
//!
//! Two transports, one handler:
//! - **HTTP (Streamable)**: `POST /mcp` on the running server — JSON-RPC in,
//!   JSON-RPC out (non-streaming responses, which the spec allows).
//! - **stdio**: `swarmail mcp` bridges stdin/stdout to the HTTP endpoint so
//!   Claude Desktop & friends can launch it as a subprocess.
//!
//! Every tool maps to the same store the SMTP server writes — an agent sees
//! exactly what was accepted, with no polling loops.
//!
//! The envelope follows decision 0029 §2: only an object with a missing `id`
//! and a string `method` is a notification. The 2026-10-24 amendment to
//! accepted decision 0047 §3 also restricts supplied IDs to strings, numbers
//! and null; Boolean, object and array IDs refuse with `-32600`/null before
//! dispatch or effects. The code taxonomy is `-32700`/`-32600`/`-32601`/
//! `-32602`, and every argument is staged — an absent key keeps its schema
//! default, a present-but-wrong-typed key (or explicit `null`) refuses by
//! name before any effect.

use crate::chaos::{Chaos, ChaosEvent, ChaosRule, PROBABILITY_REFUSAL};
use crate::smtp::build_seed_email;
use crate::store::{Filter, Store, WaitOutcome};
use serde_json::{Value, json};
use std::sync::Arc;

pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Revision belongs to one HTTP request or bridge exchange, never the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Revision {
    March,
    June,
    November,
}

impl Revision {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            PROTOCOL_VERSION => Some(Self::March),
            "2025-06-18" => Some(Self::June),
            "2025-11-25" => Some(Self::November),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::March => PROTOCOL_VERSION,
            Self::June => "2025-06-18",
            Self::November => "2025-11-25",
        }
    }

    pub(crate) fn batch_refusal(self, body: &Value) -> Option<String> {
        if body.is_array() && self != Self::March {
            Some(format!("MCP {} does not support batches", self.as_str()))
        } else {
            None
        }
    }
}

/// A supplied header must contain exactly one supported value. Parse/body
/// refusals precede this check; malformed header bytes never enter diagnostics.
pub(crate) fn header_revision(headers: &axum::http::HeaderMap) -> Result<Revision, &'static str> {
    let mut values = headers.get_all("mcp-protocol-version").iter();
    match values.next() {
        None => Ok(Revision::March),
        Some(value) => {
            let revision = value.to_str().ok().and_then(|v| Revision::parse(v.trim()));
            match (revision, values.next()) {
                (Some(revision), None) => Ok(revision),
                _ => Err("MCP-Protocol-Version must be one supported revision"),
            }
        }
    }
}

fn negotiated_revision(req: &Value) -> Result<Revision, (i64, String)> {
    match req.pointer("/params/protocolVersion") {
        None => Ok(Revision::March),
        Some(Value::String(version)) => Ok(Revision::parse(version).unwrap_or(Revision::November)),
        Some(_) => Err(refusal("protocolVersion", "a string")),
    }
}

/// The largest batch the envelope answers. The advertised `2025-03-26`
/// revision requires batches; the cap keeps one request's fan-out bounded —
/// over the cap is a refusal naming the rule, never a clamp.
pub const MCP_BATCH_CAP: usize = 16;

/// A router's tool invocation rate, shared across HTTP clients and revisions.
/// Fixed windows use monotonic time; no peer-key map or background task exists.
#[derive(Debug)]
pub struct ToolRate(std::sync::Mutex<(std::time::Instant, usize)>);

impl Default for ToolRate {
    fn default() -> Self {
        Self(std::sync::Mutex::new((std::time::Instant::now(), 0)))
    }
}

impl ToolRate {
    pub(crate) const CAP: usize = 1024;
    pub(crate) const WINDOW: std::time::Duration = std::time::Duration::from_secs(1);

    fn take_at(&self, now: std::time::Instant) -> bool {
        let mut window = self.0.lock().unwrap();
        if now.saturating_duration_since(window.0) >= Self::WINDOW {
            *window = (now, 0);
        }
        if window.1 == Self::CAP {
            false
        } else {
            window.1 += 1;
            true
        }
    }
}

/// Test-owned window state for one limiter instance — the router's own,
/// handed out by `api::observe_next_router_tool_rate`. No clock framework.
#[cfg(test)]
impl ToolRate {
    pub(crate) fn set_window(&self, start: std::time::Instant, taken: usize) {
        *self.0.lock().unwrap() = (start, taken);
    }

    pub(crate) fn taken(&self) -> usize {
        self.0.lock().unwrap().1
    }
}

pub struct McpContext {
    pub tool_rate: Arc<ToolRate>,
    pub store: Arc<Store>,
    pub chaos: Arc<Chaos>,
    /// The server's stop watch: the wait tool races its wait against it so a
    /// parked client cannot hold `stop()` for its timeout's remainder
    /// (decision 0018). The same receiver the SSE feed consumes — one
    /// source of "stopping", never a second channel.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
    /// The SMTP route domain (decision 0112): `swarmail_create_inbox` mints
    /// `agent@<inbox>.<domain>` under it and refuses without it.
    pub route_domain: Option<String>,
}

/// The JSON-RPC error envelope — the one owner of its shape.
pub(crate) fn error_object(code: i64, message: &str, id: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The refusal a non-object request earns (decision 0044 §1): JSON-RPC 2.0
/// §5–§6 make it an invalid request, answered with a null id — never the
/// silence a notification gets.
const NOT_AN_OBJECT: &str = "a JSON-RPC request must be an object";

/// How a parsed envelope is answered (decision 0039 §2, updated by
/// 0044 §1) — the one owner of the rule both transports apply: `dispatch`
/// refuses by it, and the stdio bridge shapes its replies and its transport
/// errors by it, so the bridge can no longer mistake a batch for a
/// notification (issue #72).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Envelope {
    /// No reply: a single notification (an object without an `id` whose
    /// `method` is a string), or an admitted batch of notifications only.
    Silent,
    /// One request: answered with one object carrying this id, null
    /// included — a non-object value is answered too, with a null id.
    Single(Value),
    /// An admitted batch with requests: answered with one array, one entry
    /// per valid id-bearing member and one null-id entry per invalid member
    /// (including a non-scalar supplied id), in member order — duplicates
    /// and explicit nulls kept; notifications are omitted.
    Batch(Vec<Value>),
    /// An empty or over-cap array: one null-id refusal, whatever ids its
    /// members carry. The bounds are judged before any silence.
    InvalidBatch,
}

/// The id one value is answered with, if any — the shared notification and
/// supplied-ID predicate used by [`classify`] and [`handle`]. A string,
/// number or null ID is retained. Boolean, object and array IDs normalize to
/// a null reply position and are refused by `handle` before effects (accepted
/// decision 0047 §3, amended 2026-10-24). `None` — silence — only for an
/// object with no `id` and a string `method`. Other id-less objects and
/// non-object values are invalid requests owed a null ID (decision 0044 §1).
pub(crate) fn valid_id(id: &Value) -> bool {
    matches!(id, Value::String(_) | Value::Number(_) | Value::Null)
}

pub(crate) fn reply_id(value: &Value) -> Option<Value> {
    match value {
        Value::Object(fields) => match fields.get("id") {
            Some(id) if valid_id(id) => Some(id.clone()),
            Some(_) => Some(Value::Null),
            None if fields.get("method").is_some_and(Value::is_string) => None,
            None => Some(Value::Null),
        },
        _ => Some(Value::Null),
    }
}

/// Classify a parsed body. A batch member contributes only its own `id`,
/// never one nested inside another array — a nested array is itself an
/// invalid member, answered with a null id.
pub(crate) fn classify(body: &Value) -> Envelope {
    match body {
        Value::Array(reqs) if reqs.is_empty() || reqs.len() > MCP_BATCH_CAP => {
            Envelope::InvalidBatch
        }
        Value::Array(reqs) => {
            let ids: Vec<Value> = reqs.iter().filter_map(reply_id).collect();
            if ids.is_empty() {
                Envelope::Silent
            } else {
                Envelope::Batch(ids)
            }
        }
        single => match reply_id(single) {
            Some(id) => Envelope::Single(id),
            None => Envelope::Silent,
        },
    }
}

/// Modern arrays owe one error even if every member is a notification.
pub(crate) fn classify_revision(body: &Value, revision: Revision) -> Envelope {
    if revision.batch_refusal(body).is_some() {
        Envelope::InvalidBatch
    } else {
        classify(body)
    }
}

/// Answer one `POST /mcp` body: the parse lives here, from the bounded body
/// bytes (decision 0044 §1), so malformed JSON and invalid UTF-8 alike are a
/// JSON-RPC `-32700` with a null id, not a plain-text 400 (decision 0029
/// §2). A JSON array is a batch, its members polled concurrently and
/// answered in member order; a single value is one request.
pub async fn dispatch(ctx: &McpContext, body: &[u8]) -> axum::response::Response {
    dispatch_with_headers(ctx, body, &axum::http::HeaderMap::new()).await
}

pub(crate) async fn dispatch_with_headers(
    ctx: &McpContext,
    body: &[u8],
    headers: &axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::Json;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let parsed: Result<Value, _> = serde_json::from_slice(body);
    let revision = match &parsed {
        Err(_) => Revision::March,
        Ok(_) => match header_revision(headers) {
            Ok(revision) => revision,
            Err(reason) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(error_object(-32600, reason, Value::Null)),
                )
                    .into_response();
            }
        },
    };
    match parsed {
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(error_object(
                -32700,
                &format!("parse error: {e}"),
                Value::Null,
            )),
        )
            .into_response(),
        Ok(body) if revision.batch_refusal(&body).is_some() => (
            StatusCode::OK,
            Json(error_object(
                -32600,
                &revision.batch_refusal(&body).unwrap(),
                Value::Null,
            )),
        )
            .into_response(),
        // The bounds are the shared classification's (decision 0039 §2):
        // judged before any member runs, whatever ids the members carry.
        Ok(batch) if classify(&batch) == Envelope::InvalidBatch => {
            let reason = if batch.as_array().is_some_and(Vec::is_empty) {
                "an empty batch is not a JSON-RPC request".to_string()
            } else {
                format!("a batch is at most {MCP_BATCH_CAP} requests")
            };
            // A refusal, never a clamp: one error object, id null.
            (
                StatusCode::OK,
                Json(error_object(-32600, &reason, Value::Null)),
            )
                .into_response()
        }
        Ok(Value::Array(reqs)) => {
            // Every admitted member runs — a notification's effect happens
            // even though its answer is silence. The members are polled
            // concurrently (decision 0044 §2): at most `MCP_BATCH_CAP`
            // futures on this task, no spawn, so sixteen waits share one
            // wait interval instead of summing sixteen. Replies keep member
            // order; the members' effects have no order — a client that
            // needs two mutations ordered sends them as two requests.
            let replies = futures_util::future::join_all(
                reqs.iter()
                    .map(|req| handle_revision(ctx, req, Revision::March, true)),
            );
            let responses: Vec<Value> = replies.await.into_iter().flatten().collect();
            // An all-notification batch is answered with silence; a mixed
            // one carries only the requests' answers, in order.
            if responses.is_empty() {
                StatusCode::ACCEPTED.into_response()
            } else {
                Json(responses).into_response()
            }
        }
        Ok(req) => match handle_revision(ctx, &req, revision, false).await {
            Some(resp) => Json(resp).into_response(),
            None => StatusCode::ACCEPTED.into_response(), // no reply for a notification
        },
    }
}

/// Handle one JSON-RPC request. Returns `None` for a notification — an
/// object with no `id` and a string `method` (decision 0029 §2), whatever
/// that method names. Supplied Boolean, object and array IDs refuse with
/// `-32600`/null before method dispatch or effects (decision 0047 §3,
/// amended 2026-10-24); other malformed id-less objects are also answered.
pub async fn handle(ctx: &McpContext, req: &Value) -> Option<Value> {
    handle_revision(ctx, req, Revision::March, false).await
}

async fn handle_revision(
    ctx: &McpContext,
    req: &Value,
    revision: Revision,
    batched: bool,
) -> Option<Value> {
    // A value that is not an object is no request at all (decision 0044
    // §1): `-32600`, null id — the classification's null-id position.
    if !req.is_object() {
        return Some(error_object(-32600, NOT_AN_OBJECT, Value::Null));
    }
    // Invalid supplied IDs are requests, not notifications: refuse before
    // method lookup or any tool effect (decision 0047 §3, amended 2026-10-24).
    if req.get("id").is_some_and(|id| !valid_id(id)) {
        return Some(error_object(
            -32600,
            "id must be a string, number, or null",
            Value::Null,
        ));
    }
    // An id-less object reaches the method checks below with a null id, so a
    // missing or non-string method is answered, never treated as silence.
    let id = reply_id(req);
    let method = match req.get("method") {
        None => {
            return id.map(|id| error_object(-32600, "missing method", id));
        }
        Some(m) => match m.as_str() {
            Some(s) => s.to_string(),
            None => {
                return id.map(|id| error_object(-32600, "method must be a string", id));
            }
        },
    };

    let outcome: Result<Value, (i64, String)> = match method.as_str() {
        // An admitted legacy batch retains historical initialize behavior.
        "initialize" => {
            let selected = if batched {
                Ok(Revision::March)
            } else {
                negotiated_revision(req)
            };
            selected.map(|selected| {
                json!({
                    "protocolVersion": selected.as_str(),
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "swarmail", "version": env!("CARGO_PKG_VERSION") },
                })
            })
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_descriptors(revision) })),
        "tools/call" => tool_call(ctx, req, revision).await,
        other => Err((-32601, format!("unknown method: {other}"))),
    };

    // A notification's effect happens even though its answer is silence:
    // an id-less member (a bare tools/call seed, a cancellation) runs to
    // completion and answers nothing. The classification's Silent covers
    // exactly this (decision 0039 §2).
    id.map(|id| match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error_object(code, &message, id),
    })
}

/// The minting tool (decision 0112 §5).
const CREATE_INBOX: &str = "swarmail_create_inbox";

/// The refusal on a server that routes nothing.
const CREATE_INBOX_UNCONFIGURED: &str = "recipient routing is not configured: start the server with --route-domain <domain> (SWARMAIL_ROUTE_DOMAIN) to mint inbox addresses";

/// A fresh inbox name and its address under the route domain (decision 0112
/// §5). A UUID v7 as 32 lowercase hex characters is inside the routing
/// alphabet and the shared inbox rule. Nothing is stored, locked, registered
/// or expired: an inbox with no mail does not exist (decision 0032), and the
/// first routed mail creates it.
fn mint_inbox(domain: &str) -> Value {
    let inbox = uuid::Uuid::now_v7().simple().to_string();
    json!({ "address": format!("agent@{inbox}.{domain}"), "inbox": inbox })
}

fn tool_descriptors(revision: Revision) -> Value {
    catalogue(revision).clone()
}

/// The catalogue a revision publishes: March's pinned literal, or the modern
/// projection both later revisions share (built once, not per discovery).
fn catalogue(revision: Revision) -> &'static Value {
    match revision {
        Revision::March => &TOOLS,
        Revision::June | Revision::November => &MODERN_TOOLS,
    }
}

/// The modern projection of [`TOOLS`] (decisions 0082 §1, 0108 §1): output
/// schemas and annotations, the compact wording of the two list reads, and
/// the `fields` argument on the four reads that project.
static MODERN_TOOLS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    let mut tools = TOOLS.clone();
    let fields = json!({
        "type": "array",
        "items": { "type": "string", "enum": crate::api::FIELD_NAMES },
        "description": MODERN_FIELDS_DESCRIPTION,
    });
    for tool in tools.as_array_mut().unwrap() {
        let name = tool["name"].as_str().unwrap().to_string();
        match name.as_str() {
            "swarmail_wait_for_email" => {
                tool["description"] = json!(tool["description"].as_str().unwrap().replace(
                    " Waits inside one batch run concurrently; their effects have no order.",
                    "",
                ));
            }
            // The shared literal is March's byte-pinned descriptor and stays
            // as captured; only the modern projection states the current rules.
            "swarmail_get_deliverability_report" => {
                tool["description"] = json!(MODERN_REPORT_DESCRIPTION);
            }
            "swarmail_search_emails" => {
                tool["description"] = json!(MODERN_SEARCH_DESCRIPTION);
            }
            "swarmail_get_latest_email" => {
                tool["description"] = json!(MODERN_LATEST_DESCRIPTION);
            }
            _ => {}
        }
        if PROJECTING_READS.contains(&name.as_str()) {
            tool["inputSchema"]["properties"]["fields"] = fields.clone();
        }
        tool["outputSchema"] = OUTPUT_SCHEMAS[name.as_str()].clone();
        // Reads never change stored mail or extraction; the report's
        // lazy cache fill is not a mail mutation (0085).
        let read_only = matches!(
            name.as_str(),
            "swarmail_search_emails"
                | "swarmail_get_email"
                | "swarmail_get_latest_email"
                | "swarmail_wait_for_email"
                | "swarmail_extract_links"
                | "swarmail_extract_codes"
                | "swarmail_get_deliverability_report"
                | focused::CODE
                | focused::LINK
                | CREATE_INBOX
                | attachments::LIST
                | attachments::GET
        );
        tool["annotations"] = if read_only {
            json!({"readOnlyHint": true})
        } else {
            json!({"readOnlyHint": false, "destructiveHint": true})
        };
    }
    tools
});

/// The four reads that accept `fields` on a modern revision (decision 0108
/// §3). The focused waits, the extractors, the seed and the report keep
/// their own shapes.
const PROJECTING_READS: [&str; 4] = [
    "swarmail_search_emails",
    "swarmail_get_latest_email",
    "swarmail_get_email",
    "swarmail_wait_for_email",
];

const MODERN_SEARCH_DESCRIPTION: &str = "Search captured emails in an inbox by recipient, sender, subject or arrival time. Returns a compact view of each email by default: id (pass it to swarmail_get_email for the full email), from, to, subject, received_ms, a snippet of at most 200 characters, codes, anchors (url and anchor_text) and attachments (filename, content_type, size); anchors and attachments show at most 250 entries with the exact count in anchors_total or attachments_total. Name fields to choose members instead. Also returns the full match count as total; offset pages past the limit.";

const MODERN_LATEST_DESCRIPTION: &str = "Fetch the most recent email(s) in an inbox as the same compact view swarmail_search_emails returns by default: id (pass it to swarmail_get_email for the full email), from, to, subject, received_ms, snippet, codes, anchors and attachments. Name fields to choose members instead.";

const MODERN_FIELDS_DESCRIPTION: &str = "Members to return for each email, plus id. Names the email lacks are omitted. Keys come in the server's fixed order, not the order requested. An empty list returns ids only. text and html appear only when named.";

fn text_result(v: Value, revision: Revision) -> Value {
    let mut result =
        json!({ "content": [ { "type": "text", "text": v.to_string() } ], "isError": false });
    match revision {
        Revision::March => {}
        Revision::June | Revision::November => result["structuredContent"] = v,
    }
    result
}

fn error_result(msg: String) -> Value {
    json!({ "content": [ { "type": "text", "text": msg } ], "isError": true })
}

fn args(req: &Value) -> &Value {
    req.pointer("/params/arguments").unwrap_or(&Value::Null)
}

/// The one owner of a staging refusal's shape (decision 0029 §1): the
/// field's name and its domain in one sentence.
fn refusal(field: &str, domain: &str) -> (i64, String) {
    (-32602, format!("{field} must be {domain}"))
}

/// The shared chaos sentences keep their own wording (they name a domain,
/// not a type); the envelope still owns the code.
fn chaos_refusal(sentence: &str) -> (i64, String) {
    (-32602, sentence.to_string())
}

/// Stage a string argument: `Ok(None)` is absence (the schema default
/// applies at the call site); a present value that is not a string —
/// explicit `null` included — refuses, by name.
fn arg_str(args: &Value, key: &str) -> Result<Option<String>, (i64, String)> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(refusal(key, "a string")),
    }
}

fn map_usize_conversion(
    key: &str,
    result: Result<usize, std::num::TryFromIntError>,
) -> Result<Option<usize>, (i64, String)> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err(refusal(key, "a non-negative integer that fits usize")),
    }
}

/// Stage an unsigned integer argument (0 and up).
fn arg_usize(args: &Value, key: &str) -> Result<Option<usize>, (i64, String)> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => match v.as_u64() {
            Some(n) => map_usize_conversion(key, usize::try_from(n)),
            None => Err(refusal(key, "a non-negative integer")),
        },
    }
}

/// Stage a signed integer argument — negative values are legal
/// (`since_ms` may sit before the epoch).
fn arg_i64(args: &Value, key: &str) -> Result<Option<i64>, (i64, String)> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => match v.as_i64() {
            Some(n) => Ok(Some(n)),
            None => Err(refusal(key, "an integer")),
        },
    }
}

/// The inbox argument with its schema default: an absent inbox is the
/// `default` inbox on every tool that reads one, and a supplied blank one
/// refuses by the shared rule (decision 0044 §3) — never the default.
fn arg_inbox(args: &Value) -> Result<String, (i64, String)> {
    let inbox = arg_str(args, "inbox")?.unwrap_or_else(|| "default".into());
    crate::model::validate_inbox(&inbox).map_err(|sentence| (-32602, sentence.to_string()))?;
    Ok(inbox)
}

/// The first supplied argument key the tool's own descriptor does not
/// publish (decision 0044 §3): the descriptor's `properties` are the
/// vocabulary — no second registry — and a tool without inputs publishes
/// none, so any key it is handed is unknown. An unknown tool has no
/// descriptor and keeps its `-32601`.
fn unknown_argument<'a>(tool: &str, args: &'a Value, revision: Revision) -> Option<&'a str> {
    let descriptor = catalogue(revision)
        .as_array()?
        .iter()
        .find(|t| t["name"] == tool)?;
    let known = descriptor["inputSchema"].get("properties");
    args.as_object()?
        .keys()
        .map(String::as_str)
        .find(|key| known.and_then(|p| p.get(*key)).is_none())
}

/// Stage the `fields` argument (decision 0108 §3): `None` is absence, a
/// present value must be an array of known names. Staged before any store
/// read and before a wait parks, so a refusal leaves no trace.
fn arg_fields(args: &Value) -> Result<Option<Vec<&'static str>>, (i64, String)> {
    args.get("fields")
        .map(|value| crate::api::parse_fields(value).map_err(|sentence| (-32602, sentence)))
        .transpose()
}

/// One email as a read serves it: projected when `fields` was named,
/// otherwise compact where the read is compact by default on a modern
/// revision, otherwise whole.
fn email_view(
    email: &crate::model::Email,
    revision: Revision,
    fields: Option<&[&str]>,
    compact: bool,
) -> Value {
    match fields {
        Some(fields) => crate::api::project(email, fields),
        None if compact && revision != Revision::March => {
            serde_json::to_value(crate::api::CompactEmail::from(email)).unwrap()
        }
        None => serde_json::to_value(email).unwrap(),
    }
}

/// The `id` argument, required on the by-id tools: absence keeps its
/// preserved sentence; a wrong type refuses by name.
fn required_id(args: &Value) -> Result<String, (i64, String)> {
    arg_str(args, "id")?.ok_or_else(|| (-32602, "missing id".to_string()))
}

async fn tool_call(
    ctx: &McpContext,
    req: &Value,
    revision: Revision,
) -> Result<Value, (i64, String)> {
    if !ctx.tool_rate.take_at(std::time::Instant::now()) {
        return Ok(error_result(
            "tool invocation rate exceeds 1024 per second; retry later".into(),
        ));
    }
    let params = req.pointer("/params").unwrap_or(&Value::Null);
    let name = arg_str(params, "name")?.ok_or_else(|| (-32602, "missing tool name".to_string()))?;
    // The arguments container is itself a supplied value (0025 §2): a
    // present container that is not an object — an array, a scalar, an
    // explicit null — refuses before any staging. Falling through would
    // let `swarmail_clear_inbox` with `"arguments": []` silently clear
    // the default inbox, the exact silent-default class the family
    // exists to close.
    if let Some(container) = req.pointer("/params/arguments")
        && !container.is_object()
    {
        return Err(refusal("arguments", "an object of named arguments"));
    }
    let a = args(req);
    // Every tool's argument shape is closed (decision 0044 §3, closing 0029
    // §7's named residual): a typo'd key refuses by name before any effect
    // — `{"inbx":"zz"}` must never clear the default inbox. The chaos tool
    // keeps its pinned sentence (0029 §3).
    if let Some(key) = unknown_argument(&name, a, revision) {
        let sentence = match name.as_str() {
            "swarmail_set_chaos" => {
                format!("unknown argument: {key} (use event|probability|error|delay_ms)")
            }
            _ => format!("unknown argument: {key}"),
        };
        return Err((-32602, sentence));
    }
    let store = &ctx.store;
    let text_result = |value| text_result(value, revision);

    match name.as_str() {
        "swarmail_search_emails" => {
            let inbox = arg_inbox(a)?;
            let filter = Filter {
                to: arg_str(a, "to")?,
                from: arg_str(a, "from")?,
                subject: arg_str(a, "subject")?,
                since_ms: arg_i64(a, "since_ms")?,
            };
            // The default limit is the one shared constant (REST and MCP
            // agree), and `total` is the full match count — the page is
            // the limit's business, not the total's. The page cap is the
            // shared refusal (decision 0030 §3): staged as `-32602` before
            // any scan is taken.
            let limit = match arg_usize(a, "limit")? {
                None => crate::api::DEFAULT_LIST_LIMIT,
                Some(n) => {
                    crate::api::checked_page_size("limit", n).map_err(|msg| (-32602, msg))?
                }
            };
            // `offset` pages past the cap (decision 0044 §3, issue #91):
            // uncapped like REST's, skipped before the limit is taken, and
            // `total` stays the unpaged match count.
            let offset = arg_usize(a, "offset")?.unwrap_or(0);
            let fields = arg_fields(a)?;
            let all = store.list(&inbox, &filter);
            let total = all.len();
            let emails: Vec<Value> = all
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|e| email_view(&e, revision, fields.as_deref(), true))
                .collect();
            Ok(text_result(
                json!({ "inbox": inbox, "total": total, "emails": emails }),
            ))
        }
        "swarmail_get_deliverability_report" => {
            let id = required_id(a)?;
            match store.get(&id) {
                None => Ok(error_result(format!("message {id} not found"))),
                Some(email) => match store.report(email).await {
                    Ok(report) => Ok(text_result(serde_json::to_value(report).unwrap())),
                    Err(error) => Ok(error_result(error.message().into())),
                },
            }
        }
        "swarmail_get_email" => {
            let id = required_id(a)?;
            let fields = arg_fields(a)?;
            match store.get(&id) {
                Some(e) => Ok(text_result(email_view(
                    &e,
                    revision,
                    fields.as_deref(),
                    false,
                ))),
                None => Ok(error_result(format!("message {id} not found"))),
            }
        }
        "swarmail_get_latest_email" => {
            let inbox = arg_inbox(a)?;
            // A supplied `count: 0` refuses: the schema publishes
            // `minimum: 1`, and a silent `.max(1)` would invent a page the
            // caller did not ask for. The *wait's* `count: 0` keeps 0010's
            // at-least-one semantic — a different domain, a different rule.
            // The page cap is the shared refusal (decision 0030 §3).
            let count = match arg_usize(a, "count")? {
                None => 1,
                Some(0) => return Err(refusal("count", "an integer of at least 1")),
                Some(n) => {
                    crate::api::checked_page_size("count", n).map_err(|msg| (-32602, msg))?
                }
            };
            let fields = arg_fields(a)?;
            let emails: Vec<Value> = store
                .list(&inbox, &Filter::default())
                .into_iter()
                .take(count)
                .map(|e| email_view(&e, revision, fields.as_deref(), true))
                .collect();
            if emails.is_empty() {
                return Ok(error_result(format!("inbox {inbox} is empty")));
            }
            Ok(text_result(json!({ "emails": emails })))
        }
        "swarmail_delete_email" => {
            let id = required_id(a)?;
            // A failed commit is the result (decision 0014): `isError` with
            // the fixed text — never an `ok` for work that did not commit.
            // An id the store never held is an honest not-found error.
            match store.clone().delete_async(id.clone()).await {
                Ok(true) => Ok(text_result(json!({ "deleted": true }))),
                Ok(false) => Ok(error_result(format!("message {id} not found"))),
                Err(_) => Ok(error_result("storage commit failed".into())),
            }
        }
        "swarmail_clear_inbox" => {
            let inbox = arg_inbox(a)?;
            match store.clone().clear_async(inbox).await {
                Ok(removed) => Ok(text_result(json!({ "removed": removed }))),
                Err(_) => Ok(error_result("storage commit failed".into())),
            }
        }
        "swarmail_clear_all" => match store.clone().clear_all_async().await {
            Ok(removed) => Ok(text_result(json!({ "removed": removed }))),
            Err(_) => Ok(error_result("storage commit failed".into())),
        },
        "swarmail_wait_for_email" => {
            let inbox = arg_inbox(a)?;
            let threshold = arg_str(a, "no_findings_at_or_above")?
                .map(|s| {
                    serde_json::from_value::<crate::deliverability::Severity>(json!(s))
                        .map_err(|_| refusal("no_findings_at_or_above", "info, warning or error"))
                })
                .transpose()?;
            let filter = Filter {
                to: arg_str(a, "to")?,
                from: arg_str(a, "from")?,
                subject: arg_str(a, "subject")?,
                since_ms: arg_i64(a, "since_ms")?,
            };
            // `count: 0` still means at least one (decision 0010). The page
            // cap is the shared refusal (decision 0030 §3), staged as
            // `-32602` before the wait parks — a refused wait registers no
            // waiter (0025 §5).
            let want = match arg_usize(a, "count")? {
                None => 1,
                Some(n) => crate::api::checked_page_size("count", n)
                    .map_err(|msg| (-32602, msg))?
                    .max(1),
            };
            let fields = arg_fields(a)?;
            // Omission takes the documented default (decision 0025); a
            // *supplied* value that does not parse is refused — never
            // silently defaulted. `null` is supplied (the key is present):
            // `Value::Null.as_u64()` is None, so it refuses with no special
            // case — the same answer `delay_ms` already gives an explicit
            // null.
            let timeout_ms = match a.get("timeout_ms") {
                None => crate::api::DEFAULT_WAIT_TIMEOUT_MS,
                Some(v) => match v.as_u64() {
                    Some(ms) => ms,
                    None => {
                        return Ok(error_result(crate::api::WAIT_TIMEOUT_REFUSAL.to_string()));
                    }
                },
            };
            // The cap check is the shared one (decision 0025); its refusal
            // is the same `isError` envelope the cap refusal already has
            // (decision 0018). Every staging refusal above answers before
            // `store.wait_for` — a rejected wait registers no waiter.
            let timeout = match crate::api::checked_wait_timeout_ms(timeout_ms) {
                Ok(timeout) => timeout,
                Err(msg) => return Ok(error_result(msg)),
            };
            // The wait races the stop watch (decision 0018) — the same race
            // the REST await runs, against the same receiver. The park
            // registers on the shared readiness gauge first, under the
            // wait's own inbox: a test sequencing against a parked MCP wait
            // reads its inbox's `swarmail_awaits_active` row over HTTP
            // instead of sleeping.
            let _park = crate::api::park_await(&inbox);
            let outcome = tokio::select! {
                outcome = store.wait_for_report(&inbox, &filter, want, timeout, threshold) => Some(outcome),
                _ = crate::api::shutdown_signal(ctx.shutdown.clone()) => None,
            };
            // The total is counted *after* the wait, in both arms: it
            // describes the response's observed state (decision 0018) — the
            // same residual the REST await names, bounded by one insert.
            let total = store.count(&inbox, &Filter::default());
            match outcome {
                Some(Err(error)) => Ok(error_result(error.message().into())),
                Some(Ok(WaitOutcome::Found(matches))) => {
                    let emails: Vec<Value> = matches
                        .iter()
                        .map(|e| email_view(e, revision, fields.as_deref(), false))
                        .collect();
                    Ok(text_result(
                        json!({ "matched": want, "total": total, "emails": emails }),
                    ))
                }
                Some(Ok(WaitOutcome::Timeout { matched })) => {
                    // The inbox's near misses (decision 0111), read after
                    // the wait resolved; mail the filter admits is already
                    // in `matched`, so it is skipped. Absent when the inbox
                    // holds nothing.
                    let mut out = json!({ "matched": matched, "total": total, "timed_out": true });
                    let near = near_miss::near_misses(store.clone(), inbox, filter, true).await;
                    if let Some(near) = near {
                        out["near_misses"] = near;
                    }
                    Ok(text_result(out))
                }
                None => Ok(error_result("shutting down".into())),
            }
        }
        focused::CODE | focused::LINK => focused::call(ctx, &name, a, revision).await,
        attachments::LIST | attachments::GET => attachments::call(ctx, &name, a, revision),
        "swarmail_extract_links" | "swarmail_extract_codes" => {
            let id = required_id(a)?;
            match store.get(&id) {
                None => Ok(error_result(format!("message {id} not found"))),
                Some(email) => {
                    if name.ends_with("links") {
                        Ok(text_result(json!({ "links": email.links })))
                    } else {
                        Ok(text_result(json!({ "codes": email.codes })))
                    }
                }
            }
        }
        "swarmail_seed_email" => {
            let inbox = arg_inbox(a)?;
            let to = arg_str(a, "to")?.ok_or_else(|| (-32602, "missing to".to_string()))?;
            let from = arg_str(a, "from")?;
            let subject = arg_str(a, "subject")?;
            let text = arg_str(a, "text")?;
            let html = arg_str(a, "html")?;
            // The one shared seed policy (decision 0010): the advertised
            // `html` reaches the extractor at ingest, and a refused fixture
            // is an error result that stores nothing.
            let email = match build_seed_email(
                &inbox,
                &to,
                from.as_deref(),
                subject.as_deref(),
                text.as_deref(),
                html.as_deref(),
            ) {
                Ok(email) => email,
                Err(reason) => return Ok(error_result(reason)),
            };
            let id = email.id.clone();
            let links = email.links.clone();
            let codes = email.codes.clone();
            // A failed commit refuses (decision 0012): an `isError` result
            // with the fixed text — the same vocabulary as a refused
            // fixture. The refusal leaves no trace (decision 0014 §3): the
            // mail never entered memory, so it is queryable through no
            // surface.
            match store.clone().insert_async(email).await {
                Ok(_) => Ok(text_result(
                    json!({ "id": id, "links": links, "codes": codes }),
                )),
                Err(_) => Ok(error_result("storage commit failed".to_string())),
            }
        }
        "swarmail_set_chaos" => {
            // An unknown argument key has already refused, by the shared
            // descriptor check above (decision 0029 §3, 0044 §3): a typo'd
            // key must not arm nothing while answering success.
            let event =
                arg_str(a, "event")?.ok_or_else(|| (-32602, "missing event".to_string()))?;
            let event_enum: ChaosEvent = serde_json::from_value(json!(event)).map_err(|_| {
                (
                    -32602,
                    format!("unknown event: {event} (use connect|mail_from|rcpt|data)"),
                )
            })?;
            // Wire values are never narrowed (decision 0018): the argument
            // must be present, must be an integer at all, and must fit
            // `u8` (256 refusing — not becoming 0). The 0..=100 ceiling is
            // the domain's own rule (decision 0025): `rule.validate()` is
            // its one owner — this tool carries no parallel copy. Each
            // stage refuses by name; no cast, no clamp, no silent default.
            let probability = match a.get("probability") {
                None => return Err(chaos_refusal(PROBABILITY_REFUSAL)),
                Some(v) => match v.as_u64() {
                    Some(n) => u8::try_from(n).map_err(|_| chaos_refusal(PROBABILITY_REFUSAL))?,
                    None => return Err(chaos_refusal(PROBABILITY_REFUSAL)),
                },
            };
            let delay_ms = match a.get("delay_ms") {
                None => 0,
                Some(v) => v
                    .as_u64()
                    .ok_or_else(|| (-32602, "delay_ms must be a non-negative integer".into()))?,
            };
            let rule = ChaosRule {
                probability,
                error: arg_str(a, "error")?,
                delay_ms,
            };
            // The whole-shape validator is the rule type's own (decision
            // 0025): the ceiling, the reply-line grammar and the delay cap
            // each refuse by name — before any effect on the chaos config.
            rule.validate().map_err(|sentence| (-32602, sentence))?;
            // One lock-scoped write (decision 0029 §3): two concurrent
            // arms of different events cannot erase each other.
            ctx.chaos.set_event(event_enum, rule);
            Ok(text_result(
                serde_json::to_value(ctx.chaos.current()).unwrap(),
            ))
        }
        "swarmail_clear_chaos" => {
            ctx.chaos.clear();
            Ok(text_result(json!({ "chaos": "disabled" })))
        }
        CREATE_INBOX => Ok(match ctx.route_domain.as_deref() {
            Some(domain) => text_result(mint_inbox(domain)),
            None => error_result(CREATE_INBOX_UNCONFIGURED.to_string()),
        }),
        other => Err((-32601, format!("unknown tool: {other}"))),
    }
}

/// Output promises are independent of input defaults and REST request schemas.
/// All representations use the captured result; these schemas never gate effects.
fn output_object(properties: Value, required: Value) -> Value {
    json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false})
}

static OUTPUT_SCHEMAS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    let address = output_object(
        json!({"address":{"type":"string"}, "name":{"type":"string"}}),
        json!(["address"]),
    );
    let strings = json!({"type":"array", "items":{"type":"string"}});
    let count = json!({"type":"integer", "minimum":0});
    // One closed object serves the four reads (decision 0108 §7): a `fields`
    // projection can omit every member but `id`, so `id` is the only one
    // required statically; the wire tests compute each request's exact key
    // set. The eighteen `Email` members keep their shapes and the three
    // derived members (with their `_total`s) join them.
    let anchor = output_object(
        json!({"anchor_text":{"type":["string","null"]}, "url":{"type":"string"}}),
        json!(["anchor_text", "url"]),
    );
    let attachment = output_object(
        json!({
            "content_type":{"type":["string","null"]},
            "filename":{"type":["string","null"]},
            "size":count
        }),
        json!(["content_type", "filename", "size"]),
    );
    let email = output_object(
        json!({
            "id":{"type":"string"}, "inbox":{"type":"string"},
            "from":address, "envelope_from":{"type":["string","null"]},
            "to":{"type":"array","items":address}, "cc":{"type":"array","items":address},
            "recipients":strings, "subject":{"type":"string"},
            "received_at":{"type":"string"}, "received_ms":{"type":"integer"}, "size":count,
            "text":{"type":"string"}, "html":{"type":"string"}, "links":strings, "codes":strings,
            "message_id":{"type":"string"}, "in_reply_to":{"type":"string"}, "references":strings,
            "snippet":{"type":"string"},
            "anchors":{"type":"array","items":anchor}, "anchors_total":count,
            "attachments":{"anyOf":[{"type":"array","items":attachment},{"type":"null"}]},
            "attachments_total":count
        }),
        json!(["id"]),
    );
    let emails = json!({"type":"array","items":email});
    let nonempty = json!({"type":"array","items":email,"minItems":1});
    let rule = output_object(
        json!({
            "probability":{"type":"integer","minimum":0,"maximum":100},
            "delay_ms":{"type":"integer","minimum":0,"maximum":300000},
            "error":{"type":"string"}
        }),
        json!(["probability", "delay_ms"]),
    );
    let event = json!({"anyOf":[rule,{"type":"null"}]});
    // Report coordinates are u64 on the wire; the producer separately owns
    // the 65536-byte compact size, canonical order and deduplication.
    let coordinate = json!({"type":"integer","minimum":0,"maximum":u64::MAX});
    let path = json!({"type":"array","items":coordinate});
    let location = json!({"oneOf":[
        output_object(json!({
            "kind":{"type":"string","const":"header"},
            "name":{"type":"string","pattern":"^[\\x21-\\x39\\x3b-\\x40\\x5b-\\x7e]+$"},
            "occurrence":{"anyOf":[coordinate,{"type":"null"}]}
        }), json!(["kind","name","occurrence"])),
        output_object(json!({"kind":{"type":"string","const":"mime_part"},"path":path}), json!(["kind","path"])),
        output_object(json!({"kind":{"type":"string","const":"link"},"part_path":path,"byte_offset":coordinate}), json!(["kind","part_path","byte_offset"]))
    ]});
    let finding = output_object(
        json!({
            "rule_id":{"type":"string","minLength":1,"pattern":"^[!-~]+$"},
            "severity":{"type":"string","enum":["info","warning","error"]},
            "location":location,
            "fix_hint":{"type":"string","minLength":1}
        }),
        json!(["rule_id", "severity", "location", "fix_hint"]),
    );
    json!({
        "swarmail_get_deliverability_report":output_object(json!({"rule_set_id":{"type":"string"},"findings":{"type":"array","maxItems":crate::deliverability::MAX_FINDINGS,"items":finding}}), json!(["rule_set_id","findings"])),
        "swarmail_search_emails":output_object(json!({"inbox":{"type":"string"},"total":count,"emails":emails}), json!(["inbox","total","emails"])),
        "swarmail_get_email":email,
        "swarmail_get_latest_email":output_object(json!({"emails":nonempty}), json!(["emails"])),
        "swarmail_delete_email":output_object(json!({"deleted":{"type":"boolean","const":true}}), json!(["deleted"])),
        "swarmail_clear_inbox":output_object(json!({"removed":count}), json!(["removed"])),
        "swarmail_clear_all":output_object(json!({"removed":count}), json!(["removed"])),
        "swarmail_wait_for_email":{"type":"object","oneOf":[
            output_object(json!({"matched":count,"total":count,"emails":nonempty}), json!(["matched","total","emails"])),
            output_object(json!({"matched":count,"total":count,"timed_out":{"type":"boolean","const":true}}), json!(["matched","total","timed_out"])),
            output_object(json!({"matched":count,"total":count,"timed_out":{"type":"boolean","const":true},"near_misses":near_miss::schema()}), json!(["matched","total","timed_out","near_misses"]))
        ]},
        "swarmail_extract_links":output_object(json!({"links":strings}), json!(["links"])),
        "swarmail_extract_codes":output_object(json!({"codes":strings}), json!(["codes"])),
        "swarmail_seed_email":output_object(json!({"id":{"type":"string"},"links":strings,"codes":strings}), json!(["id","links","codes"])),
        "swarmail_set_chaos":output_object(json!({"connect":event,"mail_from":event,"rcpt":event,"data":event}), json!(["connect","mail_from","rcpt","data"])),
        "swarmail_clear_chaos":output_object(json!({"chaos":{"type":"string","const":"disabled"}}), json!(["chaos"])),
        focused::CODE:focused::output_schema(false, &address),
        focused::LINK:focused::output_schema(true, &address),
        CREATE_INBOX:output_object(json!({"inbox":{"type":"string"},"address":{"type":"string"}}), json!(["inbox","address"])),
        attachments::LIST:attachments::output_schema(true),
        attachments::GET:attachments::output_schema(false)
    })
});

/// Modern-only report description; March keeps the historical TOOLS literal.
const MODERN_REPORT_DESCRIPTION: &str = "Lazy versioned deliverability report (rule set swarmail-html/3) from the stored bytes, off the SMTP path: the bulk-sender checklist, the HTML client-compatibility heuristics and the synthetic demo marker. Missing trusted authentication evidence yields error-level not_evaluated findings, never a clean report; empty findings do not imply deliverability. At most 256 findings and 65536 compact JSON bytes; excess or analysis failure is an error, never truncation. Returns the report as structuredContent and the same JSON in a text block.";

/// MCP-compatible JSON input schemas. These are objects — the MCP spec
/// requires an object, and a `&str` schema embedded in `json!` serialized as
/// a JSON *string*, publishing an invalid schema for five tools (decision
/// 0010, issue #4). The fields are the verbatim re-kind of the old string
/// constants: same names, types, descriptions and `required` arrays. Each
/// schema publishes the types and bounds its staging enforces (decision
/// 0029 §1).
static SCHEMA_EMAIL_ID: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!({
        "type": "object",
        "properties": { "id": { "type": "string", "description": "Email id" } },
        "required": ["id"],
        "additionalProperties": false,
    })
});

static SCHEMA_INBOX_ONLY: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!({
        "type": "object",
        "properties": { "inbox": { "type": "string", "description": "Inbox name (default: default)" } },
        "additionalProperties": false,
    })
});

/// The recipient predicate every `to` filter publishes (decision 0044 §8,
/// issue #99): header To/Cc *and* the envelope's RCPT TO list.
const TO_FILTER_DESCRIPTION: &str = "Case-insensitive substring of any recipient: the To and Cc headers and the SMTP envelope recipients (RCPT TO, so a Bcc'd address matches) — not a header-presence assertion";

static TOOLS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    let mut tools = json!([
        { "name": "swarmail_search_emails", "description": "Search captured emails in an inbox by recipient, sender, subject or arrival time. Returns full emails (bodies, links, codes included) and the full match count as total; offset pages past the limit.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string", "description": TO_FILTER_DESCRIPTION },
            "from": { "type": "string", "description": "Case-insensitive substring of the header From address" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer", "description": "Arrival time lower bound in ms since the epoch; negative values are legal" },
            "limit": { "type": "integer", "minimum": 0, "maximum": 250, "default": 100 },
            "offset": { "type": "integer", "minimum": 0, "default": 0, "description": "Matches to skip, newest first; total is counted before paging" } },
            "additionalProperties": false } },
        { "name": "swarmail_get_deliverability_report", "description": "Lazy versioned report from stored bytes with demo and bulk-sender checks. Empty findings do not imply deliverability. Returns report JSON in a text block (2025-03-26).",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_get_email", "description": "Fetch one captured email in full by id.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_get_latest_email", "description": "Fetch the most recent email(s) in an inbox.",
          "inputSchema": { "type": "object", "properties": { "inbox": { "type": "string" }, "count": { "type": "integer", "minimum": 1, "maximum": 250, "default": 1 } },
            "additionalProperties": false } },
        { "name": "swarmail_delete_email", "description": "Delete one captured email by id (start the next assertion clean).",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_clear_inbox", "description": "Delete all emails in one inbox — the 'clear' of the clear/act/assert loop.",
          "inputSchema": SCHEMA_INBOX_ONLY.clone() },
        { "name": "swarmail_clear_all", "description": "Delete every email in every inbox.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "swarmail_wait_for_email", "description": "Block until count emails matching the filter arrive (push-based, no polling). Returns the matched emails. Waits at most timeout_ms (60000 at most) and races the server's shutdown, answering an error once it begins stopping. A count of 0 still means at least one (decision 0010). Waits inside one batch run concurrently; their effects have no order.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string", "description": TO_FILTER_DESCRIPTION },
            "from": { "type": "string", "description": "Case-insensitive substring of the header From address" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer", "description": "Arrival time lower bound in ms since the epoch; negative values are legal" },
            "count": { "type": "integer", "minimum": 0, "maximum": 250, "default": 1, "description": "Matched emails to wait for; 0 still means at least one" },
            "no_findings_at_or_above": { "type": "string", "enum": ["info", "warning", "error"], "description": "Qualify messages with no findings at or above this inclusive threshold; analysis failures are operational errors" },
            "timeout_ms": { "type": "integer", "default": 5000, "minimum": 0, "maximum": 60000 } },
            "additionalProperties": false } },
        { "name": "swarmail_extract_links", "description": "Extract all http(s) URLs from a captured email's text/HTML bodies.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_extract_codes", "description": "Extract likely OTP/verification codes from a captured email.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_seed_email", "description": "Inject a synthetic email into an inbox without sending SMTP — fixtures for agent tests. HTML is preserved verbatim and supersedes text; envelope fields (to, from, subject) containing CR, LF or NUL are refused before anything is stored.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "from": { "type": "string" }, "to": { "type": "string" },
            "subject": { "type": "string" }, "text": { "type": "string" }, "html": { "type": "string" } },
            "required": ["to"], "additionalProperties": false } },
        { "name": "swarmail_set_chaos", "description": "Enable deliberate SMTP failure: event = connect|mail_from|rcpt|data, probability 0-100, optional SMTP error line (a single line starting with a 4xx or 5xx code, at most 510 octets) and delay_ms (at most 300000).",
          "inputSchema": { "type": "object", "properties": {
            "event": { "type": "string", "enum": ["connect", "mail_from", "rcpt", "data"] },
            "probability": { "type": "integer", "minimum": 0, "maximum": 100 },
            "error": { "type": "string", "description": "A single-line SMTP reply starting with a 4xx or 5xx code, at most 510 octets" },
            "delay_ms": { "type": "integer", "minimum": 0, "maximum": 300000 } },
            "required": ["event", "probability"], "additionalProperties": false } },
        { "name": "swarmail_clear_chaos", "description": "Disable all chaos rules.", "inputSchema": { "type": "object", "additionalProperties": false } },
    ]);
    // Appended (decision 0094 §7): every earlier entry keeps its place.
    tools.as_array_mut().unwrap().extend(focused::descriptors());
    // Appended after the focused pair (decision 0112 §5): the earlier
    // indices keep their places.
    tools.as_array_mut().unwrap().push(json!({
        "name": CREATE_INBOX,
        "description": "Mint a fresh private inbox name and the address that routes to it (agent@<inbox>.<route-domain>). Send the application's mail to that address, then read the inbox by name: mail for other addresses never appears there. Allocates nothing - the inbox exists once its first mail arrives, and a wait on it before then works. Needs the server to run with --route-domain.",
        "inputSchema": { "type": "object", "additionalProperties": false }
    }));
    // Appended after swarmail_create_inbox (decision 0110 §7), the same way:
    // every earlier index keeps its place.
    tools
        .as_array_mut()
        .unwrap()
        .extend(attachments::descriptors());
    tools
});

mod attachments;
mod focused;
mod near_miss;

#[cfg(test)]
mod batch_tests;

#[cfg(test)]
mod tests {
    use super::*;

    // Indexing a missing name yields Null, which once published a null
    // outputSchema for the report; the key sets must be exactly equal.
    #[test]
    fn every_tool_has_exactly_one_object_output_schema() {
        let names: std::collections::BTreeSet<&str> = TOOLS
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        let schemas = OUTPUT_SCHEMAS.as_object().unwrap();
        assert_eq!(names.len(), 18);
        assert_eq!(
            names,
            schemas
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>()
        );
        for schema in schemas.values() {
            assert_eq!(schema["type"], "object");
            assert!(schema.get("$ref").is_none());
        }
        for revision in [Revision::June, Revision::November] {
            for tool in tool_descriptors(revision).as_array().unwrap() {
                assert!(tool["outputSchema"].is_object());
            }
        }
        for tool in tool_descriptors(Revision::March).as_array().unwrap() {
            assert!(tool.get("outputSchema").is_none());
        }
    }

    #[test]
    fn tool_rate_uses_exact_monotonic_windows() {
        // Literal contract, deliberately not derived from the production
        // constants: 1024 attempts per fixed one-second window.
        let rate = ToolRate::default();
        let t = std::time::Instant::now();
        rate.set_window(t, 0);
        for _ in 0..1024 {
            assert!(rate.take_at(t));
        }
        assert!(!rate.take_at(t), "attempt 1025 is the first refusal");
        assert_eq!(rate.taken(), 1024);
        assert!(!rate.take_at(t + std::time::Duration::from_nanos(999_999_999)));
        assert_eq!(rate.taken(), 1024, "a refusal neither consumes nor refills");
        let t1 = t + std::time::Duration::from_secs(1);
        assert!(rate.take_at(t1), "one second opens a fresh window");
        for _ in 0..1023 {
            assert!(rate.take_at(t1));
        }
        assert!(
            !rate.take_at(t1),
            "the fresh window also holds exactly 1024"
        );
        assert_eq!(rate.taken(), 1024);
    }

    #[tokio::test]
    async fn exhausted_tool_rate_refuses_before_effects_and_keeps_control_available() {
        let (_stop, ctx) = context();
        // A generated future start keeps this dispatch proof independent of
        // scheduling; the preceding test proves the literal 1024-per-second
        // contract with explicit instants.
        ctx.tool_rate.set_window(
            std::time::Instant::now() + std::time::Duration::from_secs(3600),
            1024,
        );
        ctx.store
            .insert(crate::store::email("kept", "kept", "a@x.io"))
            .unwrap();
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mutations = Arc::new(AtomicUsize::new(0));
        let observed = mutations.clone();
        ctx.store.set_pre_mirror_gate(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        })));
        let subscriptions = Arc::new(AtomicUsize::new(0));
        let observed = subscriptions.clone();
        ctx.store.set_post_subscribe_gate(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        })));
        let chaos_before = serde_json::to_value(ctx.chaos.current()).unwrap();
        for revision in [Revision::March, Revision::June, Revision::November] {
            for (name, arguments) in [
                ("swarmail_seed_email", json!({"to":"a@x.io"})),
                ("swarmail_clear_all", json!({})),
                ("swarmail_wait_for_email", json!({"timeout_ms":0})),
                (
                    "swarmail_set_chaos",
                    json!({"event":"data","probability":100}),
                ),
            ] {
                let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}});
                let reply = handle_revision(&ctx, &req, revision, false).await.unwrap();
                assert_eq!(
                    reply["result"]["isError"], true,
                    "rate did not refuse {name}"
                );
                assert_eq!(
                    reply["result"]["content"][0]["text"],
                    "tool invocation rate exceeds 1024 per second; retry later"
                );
                assert!(reply["result"].get("structuredContent").is_none());
            }
            for method in ["ping", "initialize", "tools/list"] {
                assert!(
                    handle_revision(&ctx, &json!({"id":2,"method":method}), revision, false)
                        .await
                        .unwrap()
                        .get("result")
                        .is_some()
                );
            }
            assert!(
                handle_revision(
                    &ctx,
                    &json!({"method":"tools/call","params":{"name":"swarmail_clear_all"}}),
                    revision,
                    false
                )
                .await
                .is_none()
            );
        }
        assert_eq!(ctx.store.list_all(&Filter::default()).len(), 1);
        assert_eq!(ctx.store.watcher_count(), 0);
        assert_eq!(mutations.load(Ordering::SeqCst), 0);
        assert_eq!(subscriptions.load(Ordering::SeqCst), 0);
        assert_eq!(
            serde_json::to_value(ctx.chaos.current()).unwrap(),
            chaos_before
        );
        ctx.tool_rate.set_window(std::time::Instant::now(), 0);
        assert_eq!(
            call(&ctx, "swarmail_seed_email", json!({"to":"a@x.io"})).await["result"]["isError"],
            false
        );
        assert_eq!(
            call(&ctx, "swarmail_wait_for_email", json!({"timeout_ms":0})).await["result"]["isError"],
            false
        );
        assert_eq!(mutations.load(Ordering::SeqCst), 1);
        assert_eq!(subscriptions.load(Ordering::SeqCst), 1);
        ctx.store.set_pre_mirror_gate(None);
        ctx.store.set_post_subscribe_gate(None);
    }

    #[tokio::test]
    async fn revision_admission_has_no_transient_mutations_or_waiters() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (_stop, ctx) = context();
        ctx.store
            .insert(crate::store::email("kept", "kept", "a@x.io"))
            .unwrap();
        ctx.chaos.set_event(
            ChaosEvent::Data,
            ChaosRule {
                probability: 100,
                error: None,
                delay_ms: 0,
            },
        );
        let mutations = Arc::new(AtomicUsize::new(0));
        let observed = mutations.clone();
        ctx.store.set_pre_mirror_gate(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        })));
        let subscriptions = Arc::new(AtomicUsize::new(0));
        let observed = subscriptions.clone();
        let store = Arc::downgrade(&ctx.store);
        ctx.store.set_post_subscribe_gate(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            // Delivery exactly after subscribe and before scan proves 0002
            // through the MCP wait tool, without sleeps or a lost-wake window.
            store
                .upgrade()
                .unwrap()
                .insert(crate::store::email("window", "window", "a@x.io"))
                .unwrap();
        })));
        let before = serde_json::to_value(ctx.store.list_all(&Filter::default())).unwrap();
        let chaos = serde_json::to_value(ctx.chaos.current()).unwrap();
        let member = |name: &str, arguments: Value| json!({"id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}});
        let members = json!([
            member("swarmail_seed_email", json!({"to":"a@x.io"})),
            member("swarmail_delete_email", json!({"id":"kept"})),
            member("swarmail_clear_inbox", json!({"inbox":"kept"})),
            member("swarmail_clear_all", json!({})),
            member("swarmail_clear_chaos", json!({})),
            member(
                "swarmail_set_chaos",
                json!({"event":"connect","probability":100})
            ),
            member(
                "swarmail_wait_for_email",
                json!({"inbox":"window","timeout_ms":0})
            )
        ]);
        for revision in [Revision::June, Revision::November] {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("mcp-protocol-version", revision.as_str().parse().unwrap());
            let response =
                dispatch_with_headers(&ctx, members.to_string().as_bytes(), &headers).await;
            assert_eq!(response.status(), 200);
            assert_eq!(mutations.load(Ordering::SeqCst), 0);
            assert_eq!(subscriptions.load(Ordering::SeqCst), 0);
            assert_eq!(ctx.store.watcher_count(), 0);
            assert_eq!(
                serde_json::to_value(ctx.store.list_all(&Filter::default())).unwrap(),
                before
            );
            assert_eq!(serde_json::to_value(ctx.chaos.current()).unwrap(), chaos);
            // The identical modern single wait really subscribes and sees
            // mail inserted in its subscribe-before-scan window.
            let request = member(
                "swarmail_wait_for_email",
                json!({"inbox":"window","timeout_ms":0}),
            );
            let reply = handle_revision(&ctx, &request, revision, false)
                .await
                .unwrap();
            let data: Value =
                serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap())
                    .unwrap();
            assert_eq!(data["matched"], 1);
            ctx.store.clear("window").unwrap();
            mutations.store(0, Ordering::SeqCst);
            subscriptions.store(0, Ordering::SeqCst);
        }
        // Legacy positive controls prove both observers are live.
        let seed = member("swarmail_seed_email", json!({"to":"a@x.io"}));
        dispatch(&ctx, json!([seed]).to_string().as_bytes()).await;
        assert_eq!(mutations.load(Ordering::SeqCst), 1);
        dispatch(
            &ctx,
            json!([member(
                "swarmail_wait_for_email",
                json!({"inbox":"window","timeout_ms":0})
            )])
            .to_string()
            .as_bytes(),
        )
        .await;
        assert_eq!(subscriptions.load(Ordering::SeqCst), 1);
        ctx.store.set_post_subscribe_gate(None);
        ctx.store.set_pre_mirror_gate(None);
    }

    #[test]
    fn version_headers_refuse_non_text_bytes_without_echoing_them() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "mcp-protocol-version",
            axum::http::HeaderValue::from_bytes(b"\xff").unwrap(),
        );
        assert_eq!(
            header_revision(&headers),
            Err("MCP-Protocol-Version must be one supported revision")
        );
    }

    /// The shared envelope classification, row by row (decision 0039 §2):
    /// bounds before silence, ids kept as JSON values, nothing discovered
    /// inside a nested array.
    #[test]
    fn the_envelope_classification_answers_every_row() {
        let over_cap = Value::Array(vec![json!({"jsonrpc":"2.0","id":1,"method":"ping"}); 17]);
        let over_cap_silent = Value::Array(vec![json!({"jsonrpc":"2.0","method":"x"}); 17]);
        let sixteen_malformed = Value::Array(vec![json!({"foo":"boo"}); 16]);
        for (body, want) in [
            (json!({"id":7,"method":"ping"}), Envelope::Single(json!(7))),
            (
                json!({"id":null,"method":"ping"}),
                Envelope::Single(Value::Null),
            ),
            (
                json!({"method":"notifications/initialized"}),
                Envelope::Silent,
            ),
            // An id-less object is silent only with a string method
            // (decision 0047 §3; red before: all four Silent).
            (json!({"jsonrpc":"2.0"}), Envelope::Single(Value::Null)),
            (json!({}), Envelope::Single(Value::Null)),
            (json!({"foo":"boo"}), Envelope::Single(Value::Null)),
            (json!({"method":1}), Envelope::Single(Value::Null)),
            (json!({"jsonrpc":"2.0","method":"nope"}), Envelope::Silent),
            (
                json!([{"foo":"boo"}, {"jsonrpc":"2.0","method":"x"}]),
                Envelope::Batch(vec![Value::Null]),
            ),
            (sixteen_malformed, Envelope::Batch(vec![Value::Null; 16])),
            // A non-object is an invalid request, answered with a null id —
            // never silence (decision 0044 §1; red before: all Silent).
            (json!(42), Envelope::Single(Value::Null)),
            (json!("text"), Envelope::Single(Value::Null)),
            (Value::Null, Envelope::Single(Value::Null)),
            (json!([]), Envelope::InvalidBatch),
            (over_cap, Envelope::InvalidBatch),
            (over_cap_silent, Envelope::InvalidBatch),
            (json!([{"method":"a"}, {"method":"b"}]), Envelope::Silent),
            (
                json!([[{"id":1}], 3]),
                Envelope::Batch(vec![Value::Null, Value::Null]),
            ),
            (
                json!([{"id":1}, {"method":"n"}, {"id":null}, {"id":1}]),
                Envelope::Batch(vec![json!(1), Value::Null, json!(1)]),
            ),
            (
                json!([{"id":"a"}, 5, {"method":"n"}, null]),
                Envelope::Batch(vec![json!("a"), Value::Null, Value::Null]),
            ),
        ] {
            assert_eq!(classify(&body), want, "{body}");
        }
    }

    #[test]
    fn invalid_supplied_ids_normalize_to_null_reply_positions() {
        for invalid in [
            json!(true),
            json!(false),
            json!({}),
            json!({"x": 1}),
            json!([]),
            json!([1]),
        ] {
            assert_eq!(
                classify(&json!({"id": invalid, "method": "ping"})),
                Envelope::Single(Value::Null),
                "invalid id: {invalid}"
            );
        }
    }

    #[tokio::test]
    async fn invalid_ids_refuse_before_dispatch_or_effects() {
        let (_stop, ctx) = context();
        for (index, id) in [
            json!(true),
            json!(false),
            json!({}),
            json!({"nested": []}),
            json!([]),
            json!([1]),
        ]
        .into_iter()
        .enumerate()
        {
            let inbox = format!("invalid-id-{index}");
            let request = json!({
                "id": id,
                "method": "tools/call",
                "params": { "name": "swarmail_seed_email",
                            "arguments": { "inbox": inbox, "to": "a@x.io" } }
            });
            let reply = handle(&ctx, &request)
                .await
                .expect("invalid ID owes a reply");
            assert_eq!(reply["error"]["code"], -32600, "{request}: {reply}");
            assert_eq!(reply["id"], Value::Null, "{request}: {reply}");
            assert_eq!(
                reply["error"]["message"], "id must be a string, number, or null",
                "{request}: {reply}"
            );
            assert!(ctx.store.inboxes().is_empty(), "{request} ran a tool");
            assert_eq!(
                ctx.store.watcher_count(),
                0,
                "{request} registered a waiter"
            );
        }

        let malformed_method = json!({"id": true, "method": 42});
        let reply = handle(&ctx, &malformed_method)
            .await
            .expect("invalid ID is rejected before method validation");
        assert_eq!(reply["error"]["code"], -32600, "{reply}");
        assert_eq!(reply["id"], Value::Null, "{reply}");
        assert_eq!(
            reply["error"]["message"], "id must be a string, number, or null",
            "{reply}"
        );

        for id in [json!("text"), json!(7), json!(-2), json!(1.5), Value::Null] {
            let request = json!({"id": id, "method": "ping"});
            let reply = handle(&ctx, &request).await.expect("valid ID owes a reply");
            assert_eq!(reply["id"], id, "{request}: {reply}");
            assert_eq!(reply["result"], json!({}), "{request}: {reply}");
        }
    }

    #[test]
    fn map_usize_conversion_translates_success_and_named_refusals() {
        assert_eq!(
            map_usize_conversion("offset", Ok(usize::MAX)),
            Ok(Some(usize::MAX))
        );
        assert_eq!(map_usize_conversion("count", Ok(0)), Ok(Some(0)));
        assert_eq!(
            map_usize_conversion("offset", Err(u8::try_from(256_u16).unwrap_err())),
            Err((
                -32602,
                "offset must be a non-negative integer that fits usize".to_string()
            ))
        );
        assert_eq!(
            map_usize_conversion("count", Err(u8::try_from(256_u16).unwrap_err())),
            Err((
                -32602,
                "count must be a non-negative integer that fits usize".to_string()
            ))
        );
    }

    #[test]
    fn arg_usize_preserves_optional_values_and_rejects_wrong_shapes() {
        assert_eq!(arg_usize(&json!({}), "offset"), Ok(None));
        assert_eq!(arg_usize(&json!({"offset": 0}), "offset"), Ok(Some(0)));
        assert_eq!(
            arg_usize(&json!({"offset": usize::MAX}), "offset"),
            Ok(Some(usize::MAX))
        );
        for invalid in [
            json!(null),
            json!(-1),
            json!(1.5),
            json!("1"),
            json!(true),
            json!({}),
        ] {
            assert_eq!(
                arg_usize(&json!({"offset": invalid}), "offset"),
                Err((-32602, "offset must be a non-negative integer".to_string())),
                "{invalid}"
            );
        }
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn usize_staging_accepts_max_and_refuses_overflow_on_32_bit() {
        let max = u64::try_from(usize::MAX).unwrap();
        assert_eq!(
            arg_usize(&json!({"offset": max}), "offset"),
            Ok(Some(usize::MAX))
        );
        assert_eq!(
            arg_usize(&json!({"offset": max + 1}), "offset"),
            Err((
                -32602,
                "offset must be a non-negative integer that fits usize".to_string()
            ))
        );
    }

    /// The classification is the server's own answer (decision 0039 §2,
    /// AC8): for every row, real `dispatch` replies in exactly the shape
    /// the classification promises — and a silent notification still
    /// executes its effect.
    #[tokio::test]
    async fn dispatch_answers_as_the_classification_says() {
        use axum::http::StatusCode;
        let (_stop, shutdown) = tokio::sync::watch::channel(false);
        let ctx = McpContext {
            tool_rate: Arc::new(ToolRate::default()),
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            shutdown,
            route_domain: None,
        };
        let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let note = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let seed = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"parity","to":"a@x.io"}}}"#;
        let bodies = [
            ping.to_string(),
            r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#.to_string(),
            seed.to_string(),
            "42".to_string(),
            "null".to_string(),
            "[]".to_string(),
            format!("[{}]", vec![ping; 17].join(",")),
            format!("[{}]", vec![note; 17].join(",")),
            format!("[{note},{seed}]"),
            format!(r#"[{ping},{note},{{"jsonrpc":"2.0","id":null,"method":"nope"}},{ping}]"#),
            format!("[{ping},[1,2],{note},\"x\"]"),
            "{}".to_string(),
            r#"{"foo":"boo"}"#.to_string(),
            r#"{"method":1}"#.to_string(),
            r#"{"jsonrpc":"2.0","method":"nope"}"#.to_string(),
            format!(r#"[{{"foo":"boo"}},{note}]"#),
            format!("[{}]", vec![r#"{"foo":"boo"}"#; 16].join(",")),
        ];
        for body in bodies {
            let parsed: Value = serde_json::from_str(&body).unwrap();
            let response = dispatch(&ctx, body.as_bytes()).await;
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            match classify(&parsed) {
                Envelope::Silent => {
                    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
                    assert!(bytes.is_empty(), "{body}");
                }
                Envelope::Single(id) => {
                    let reply: Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(reply["id"], id, "{body}");
                }
                Envelope::Batch(ids) => {
                    let reply: Value = serde_json::from_slice(&bytes).unwrap();
                    let answered: Vec<Value> = reply
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r["id"].clone())
                        .collect();
                    assert_eq!(answered, ids, "{body}");
                }
                Envelope::InvalidBatch => {
                    let reply: Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(reply["error"]["code"], -32600, "{body}");
                    assert_eq!(reply["id"], Value::Null, "{body}");
                }
            }
        }
        // Both silent seeds (the single one and the batch member) ran.
        assert_eq!(ctx.store.count("parity", &Filter::default()), 2);
    }

    /// A fresh context and its stop sender — held by the test, since a
    /// dropped sender reads as stopping.
    fn context() -> (tokio::sync::watch::Sender<bool>, McpContext) {
        let (stop, shutdown) = tokio::sync::watch::channel(false);
        let ctx = McpContext {
            tool_rate: Arc::new(ToolRate::default()),
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            shutdown,
            route_domain: None,
        };
        (stop, ctx)
    }

    async fn call(ctx: &McpContext, tool: &str, arguments: Value) -> Value {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                         "params":{"name": tool, "arguments": arguments}});
        handle(ctx, &req).await.expect("a request is answered")
    }

    /// Decision 0112 §5: with a route domain the tool mints a fresh lowercase
    /// hex inbox and its address and stores nothing; without one it refuses
    /// in an `isError` result naming the flag. Both revisions' bytes.
    #[tokio::test]
    async fn create_inbox_mints_under_the_domain_and_refuses_without_one() {
        let (_stop, mut ctx) = context();
        let refused = call(&ctx, CREATE_INBOX, json!({})).await;
        assert_eq!(refused["result"]["isError"], true);
        let message = refused["result"]["content"][0]["text"].as_str().unwrap();
        assert!(message.contains("--route-domain"), "{message}");

        ctx.route_domain = Some("swarmail.test".into());
        let first = call(&ctx, CREATE_INBOX, json!({})).await;
        let second = call(&ctx, CREATE_INBOX, json!({})).await;
        let value = |reply: &Value| {
            serde_json::from_str::<Value>(reply["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap()
        };
        let (first, second) = (value(&first), value(&second));
        assert_ne!(first["inbox"], second["inbox"]);
        let inbox = first["inbox"].as_str().unwrap();
        assert_eq!(inbox.len(), 32);
        assert_eq!(first["address"], format!("agent@{inbox}.swarmail.test"));
        assert!(ctx.store.inboxes().is_empty(), "minting allocates nothing");
        let sentinel = call(&ctx, CREATE_INBOX, json!({"zz": 1})).await;
        assert_eq!(sentinel["error"]["code"], -32602);
    }

    /// Every tool's argument shape is closed (decision 0044 §3, issue #91):
    /// a sentinel key refuses `-32602` naming it — zero-argument tools
    /// included — before any effect. A typo'd clear leaves `default`
    /// standing, a typo'd wait registers no watcher, and the chaos tool keeps
    /// its pinned sentence. Red before: the typo'd clear emptied `default`.
    #[tokio::test]
    async fn every_tool_refuses_an_unknown_argument_before_any_effect() {
        let (_stop, ctx) = context();
        ctx.store
            .insert(crate::store::email("keep", "default", "a@x.io"))
            .unwrap();
        let tools = TOOLS.as_array().unwrap();
        assert_eq!(tools.len(), 18);
        for tool in tools {
            let name = tool["name"].as_str().unwrap();
            assert_eq!(
                tool["inputSchema"]["additionalProperties"], false,
                "{name} publishes its closed shape"
            );
            let reply = call(&ctx, name, json!({"zz_typo": 1})).await;
            assert_eq!(reply["error"]["code"], -32602, "{name}: {reply}");
            let message = reply["error"]["message"].as_str().unwrap();
            assert!(
                message.starts_with("unknown argument: zz_typo"),
                "{name}: {reply}"
            );
        }
        let reply = call(
            &ctx,
            "swarmail_set_chaos",
            json!({"event": "data", "probability": 100, "solus": 1}),
        )
        .await;
        assert_eq!(
            reply["error"]["message"],
            "unknown argument: solus (use event|probability|error|delay_ms)"
        );
        assert!(ctx.chaos.current().data.is_none(), "no rule armed");
        let reply = call(&ctx, "swarmail_clear_inbox", json!({"inbx": "zz"})).await;
        assert_eq!(reply["error"]["message"], "unknown argument: inbx");
        assert_eq!(ctx.store.count("default", &Filter::default()), 1);
        let reply = call(
            &ctx,
            "swarmail_wait_for_email",
            json!({"inbox": "typo-wait", "timeout_ms": 60000, "tto": "a@x.io"}),
        )
        .await;
        assert_eq!(reply["error"]["message"], "unknown argument: tto");
        assert_eq!(ctx.store.watcher_count(), 0, "no waiter registered");
        // An unknown tool keeps its own code: there is no descriptor to
        // judge the keys by.
        let reply = call(&ctx, "swarmail_nope", json!({"zz_typo": 1})).await;
        assert_eq!(reply["error"]["code"], -32601, "{reply}");
        // Advertised keys still stage.
        let reply = call(&ctx, "swarmail_search_emails", json!({"inbox": "default"})).await;
        assert_eq!(reply["result"]["isError"], false, "{reply}");
    }

    /// A supplied blank inbox refuses on every inbox-reading tool (decision
    /// 0044 §3) — never the default, never a stored blank inbox — while an
    /// omitted one still selects `default` and a padded nonblank name is
    /// kept verbatim. Red before: the blank seed was stored.
    #[tokio::test]
    async fn a_blank_inbox_refuses_on_every_inbox_tool() {
        let (_stop, ctx) = context();
        for blank in ["", "  ", "\t\u{3000}\u{a0}"] {
            for (tool, extra) in [
                ("swarmail_search_emails", json!({})),
                ("swarmail_get_latest_email", json!({})),
                ("swarmail_clear_inbox", json!({})),
                ("swarmail_wait_for_email", json!({"timeout_ms": 60000})),
                ("swarmail_wait_for_code", json!({"timeout_ms": 60000})),
                ("swarmail_wait_for_link", json!({"timeout_ms": 60000})),
                ("swarmail_seed_email", json!({"to": "a@x.io"})),
            ] {
                let mut arguments = extra.clone();
                arguments["inbox"] = json!(blank);
                let reply = call(&ctx, tool, arguments).await;
                assert_eq!(reply["error"]["code"], -32602, "{tool} {blank:?}: {reply}");
                assert_eq!(
                    reply["error"]["message"],
                    crate::model::BLANK_INBOX_REFUSAL,
                    "{tool} {blank:?}"
                );
            }
        }
        // A control-bearing name refuses on every inbox tool too, with the
        // shared sentence and before any effect (issue #128).
        for control in ["\0user\0pw", "a\u{1}b", "del\u{7f}", "c1\u{85}"] {
            for (tool, extra) in [
                ("swarmail_search_emails", json!({})),
                ("swarmail_get_latest_email", json!({})),
                ("swarmail_clear_inbox", json!({})),
                ("swarmail_wait_for_email", json!({"timeout_ms": 60000})),
                ("swarmail_wait_for_code", json!({"timeout_ms": 60000})),
                ("swarmail_wait_for_link", json!({"timeout_ms": 60000})),
                ("swarmail_seed_email", json!({"to": "a@x.io"})),
            ] {
                let mut arguments = extra.clone();
                arguments["inbox"] = json!(control);
                let reply = call(&ctx, tool, arguments).await;
                assert_eq!(reply["error"]["code"], -32602, "{tool} {control:?}");
                assert_eq!(
                    reply["error"]["message"],
                    crate::model::CONTROL_INBOX_REFUSAL,
                    "{tool} {control:?}"
                );
            }
        }
        assert!(ctx.store.inboxes().is_empty(), "nothing stored");
        assert_eq!(ctx.store.watcher_count(), 0, "no waiter registered");
        let padded = json!({"inbox": " pad ", "to": "a@x.io"});
        let reply = call(&ctx, "swarmail_seed_email", padded).await;
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        assert_eq!(ctx.store.inboxes(), vec![(" pad ".to_string(), 1)]);
    }

    /// MCP search pages with `offset` (issue #91): skipped before the limit,
    /// `total` unpaged, beyond the end empty, a malformed offset refused by
    /// name. Red before: the offset was ignored and older mail unreachable.
    #[tokio::test]
    async fn search_pages_with_offset_and_keeps_the_total() {
        let (_stop, ctx) = context();
        for (i, id) in ["m0", "m1", "m2"].into_iter().enumerate() {
            let mut email = crate::store::email(id, "paged", "a@x.io");
            email.received_ms = i as i64;
            ctx.store.insert(email).unwrap();
        }
        let page = |reply: Value| -> (Value, Vec<String>) {
            let body: Value =
                serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap())
                    .unwrap();
            let ids = body["emails"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["id"].as_str().unwrap().to_string())
                .collect();
            (body["total"].clone(), ids)
        };
        let reply = call(
            &ctx,
            "swarmail_search_emails",
            json!({"inbox": "paged", "limit": 1, "offset": 1}),
        )
        .await;
        assert_eq!(page(reply), (json!(3), vec!["m1".to_string()]));
        let reply = call(
            &ctx,
            "swarmail_search_emails",
            json!({"inbox": "paged", "offset": 2}),
        )
        .await;
        assert_eq!(page(reply), (json!(3), vec!["m0".to_string()]));
        let reply = call(
            &ctx,
            "swarmail_search_emails",
            json!({"inbox": "paged", "offset": usize::MAX}),
        )
        .await;
        assert_eq!(page(reply), (json!(3), Vec::<String>::new()));
        for bad in [json!(-1), json!(null), json!("1"), json!(1.5)] {
            let reply = call(
                &ctx,
                "swarmail_search_emails",
                json!({"inbox": "paged", "offset": bad}),
            )
            .await;
            assert_eq!(
                reply["error"]["message"], "offset must be a non-negative integer",
                "{bad}"
            );
        }
    }

    /// The published schema is pinned to the domain's constants (issue #32
    /// slice 2, decision 0025): the wait's `timeout_ms` default is the one
    /// documented default and the bounds are the domain's, not this file's
    /// literals — a schema literal and a constant can drift only by
    /// failing CI.
    #[test]
    fn the_schemas_publish_the_shared_domain() {
        let tools = TOOLS.clone();
        let find = |name: &str| {
            tools
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == name)
                .expect("every published tool is listed")["inputSchema"]["properties"]
                .clone()
        };

        let wait = find("swarmail_wait_for_email");
        let timeout_ms = &wait["timeout_ms"];
        assert_eq!(timeout_ms["type"], "integer", "{timeout_ms}");
        assert_eq!(timeout_ms["default"], crate::api::DEFAULT_WAIT_TIMEOUT_MS);
        assert_eq!(timeout_ms["minimum"], 0, "{timeout_ms}");
        assert_eq!(timeout_ms["maximum"], crate::api::MAX_AWAIT_TIMEOUT_MS);
        // The wait's count publishes 0010's at-least-one semantic rather
        // than a refusal at zero: the domain, not the envelope, owns it.
        assert_eq!(wait["count"]["minimum"], 0, "{wait}");
        assert_eq!(wait["count"]["default"], 1, "{wait}");
        assert!(
            wait["count"]["description"]
                .as_str()
                .unwrap()
                .contains("at least one")
        );

        let chaos = find("swarmail_set_chaos");
        assert_eq!(chaos["probability"]["minimum"], 0, "{chaos}");
        assert_eq!(
            chaos["probability"]["maximum"],
            crate::chaos::PROBABILITY_MAX,
            "{chaos}"
        );
        assert_eq!(
            chaos["delay_ms"]["maximum"],
            crate::chaos::MAX_DELAY_MS,
            "{chaos}"
        );
        assert_eq!(
            chaos["error"]["description"],
            "A single-line SMTP reply starting with a 4xx or 5xx code, at most 510 octets",
            "{chaos}"
        );

        // The search's limit publishes the shared default and its staging
        // domain; get_latest's count refuses zero by name (AC1.3).
        let search = find("swarmail_search_emails");
        assert_eq!(
            search["limit"]["default"],
            crate::api::DEFAULT_LIST_LIMIT,
            "{search}"
        );
        assert_eq!(search["limit"]["minimum"], 0, "{search}");
        // Every page-size domain publishes the shared cap (decision 0030
        // §3) — the constant, not this file's literal.
        assert_eq!(
            search["limit"]["maximum"],
            crate::api::MAX_PAGE_SIZE,
            "{search}"
        );
        // The recipient predicate and the search's offset are published
        // where they are enforced (decision 0044 §§3, 8).
        for tool in [&search, &wait] {
            assert_eq!(tool["to"]["description"], TO_FILTER_DESCRIPTION, "{tool}");
        }
        assert!(TO_FILTER_DESCRIPTION.contains("envelope recipients"));
        assert_eq!(search["offset"]["minimum"], 0, "{search}");
        assert_eq!(search["offset"]["default"], 0, "{search}");
        let latest = find("swarmail_get_latest_email");
        assert_eq!(latest["count"]["minimum"], 1, "{latest}");
        assert_eq!(
            latest["count"]["maximum"],
            crate::api::MAX_PAGE_SIZE,
            "{latest}"
        );
        assert_eq!(
            wait["count"]["maximum"],
            crate::api::MAX_PAGE_SIZE,
            "{wait}"
        );
    }
}
