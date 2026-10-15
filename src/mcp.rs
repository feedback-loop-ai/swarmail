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
//! The envelope follows decision 0029 §2: the JSON-RPC predicate is a
//! *missing id* (a request has an id, a notification does not — whatever
//! its method name), the code taxonomy is `-32700`/`-32600`/`-32601`/
//! `-32602`, and every argument is staged — an absent key keeps its schema
//! default, a present-but-wrong-typed key (or explicit `null`) refuses by
//! name before any effect.

use crate::chaos::{Chaos, ChaosEvent, ChaosRule, PROBABILITY_REFUSAL};
use crate::smtp::build_seed_email;
use crate::store::{Filter, Store, WaitOutcome};
use serde_json::{Value, json};
use std::sync::Arc;

pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// The largest batch the envelope answers. The advertised `2025-03-26`
/// revision requires batches; the cap keeps one request's fan-out bounded —
/// over the cap is a refusal naming the rule, never a clamp.
pub const MCP_BATCH_CAP: usize = 16;

pub struct McpContext {
    pub store: Arc<Store>,
    pub chaos: Arc<Chaos>,
    /// The server's stop watch: the wait tool races its wait against it so a
    /// parked client cannot hold `stop()` for its timeout's remainder
    /// (decision 0018). The same receiver the SSE feed consumes — one
    /// source of "stopping", never a second channel.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}

/// The JSON-RPC error envelope — the one owner of its shape.
pub(crate) fn error_object(code: i64, message: &str, id: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Answer one `POST /mcp` body: the parse lives here (the `Json` extractor
/// is gone), so a malformed body is a JSON-RPC error object with a null id,
/// not a plain-text 400 (decision 0029 §2). A JSON array is a batch,
/// answered in order; a single object is one request.
pub async fn dispatch(ctx: &McpContext, body: &str) -> axum::response::Response {
    use axum::Json;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let parsed: Result<Value, _> = serde_json::from_str(body);
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
        Ok(Value::Array(reqs)) => {
            if reqs.is_empty() || reqs.len() > MCP_BATCH_CAP {
                let reason = if reqs.is_empty() {
                    "an empty batch is not a JSON-RPC request".to_string()
                } else {
                    format!("a batch is at most {MCP_BATCH_CAP} requests")
                };
                // A refusal, never a clamp: one error object, id null.
                return (
                    StatusCode::OK,
                    Json(error_object(-32600, &reason, Value::Null)),
                )
                    .into_response();
            }
            let mut responses = Vec::new();
            for req in &reqs {
                if let Some(resp) = handle(ctx, req).await {
                    responses.push(resp);
                }
            }
            // An all-notification batch is answered with silence; a mixed
            // one carries only the requests' answers, in order.
            if responses.is_empty() {
                StatusCode::ACCEPTED.into_response()
            } else {
                Json(responses).into_response()
            }
        }
        Ok(req) => match handle(ctx, &req).await {
            Some(resp) => Json(resp).into_response(),
            None => StatusCode::ACCEPTED.into_response(), // no reply for a notification
        },
    }
}

/// Handle one JSON-RPC request. Returns `None` for a notification — the
/// predicate is a *missing id* (decision 0029 §2), whatever the method name
/// says; an id-less `notifications/initialized` and an id-less `tools/call`
/// both get silence.
pub async fn handle(ctx: &McpContext, req: &Value) -> Option<Value> {
    let id = req.get("id")?.clone();
    let method = match req.get("method") {
        None => return Some(error_object(-32600, "missing method", id)),
        Some(m) => match m.as_str() {
            Some(s) => s.to_string(),
            None => return Some(error_object(-32600, "method must be a string", id)),
        },
    };

    let outcome: Result<Value, (i64, String)> = match method.as_str() {
        // One constant: the server speaks the revision it advertises, for
        // any version the client asks for, and advertises only what exists.
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "swarmail", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": TOOLS.clone() })),
        "tools/call" => tool_call(ctx, req).await,
        other => Err((-32601, format!("unknown method: {other}"))),
    };

    Some(match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error_object(code, &message, id),
    })
}

fn text_result(v: Value) -> Value {
    json!({ "content": [ { "type": "text", "text": v.to_string() } ], "isError": false })
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

/// Stage an unsigned integer argument (0 and up).
fn arg_usize(args: &Value, key: &str) -> Result<Option<usize>, (i64, String)> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => match v.as_u64() {
            Some(n) => Ok(Some(n as usize)),
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
/// `default` inbox on every tool that reads one.
fn arg_inbox(args: &Value) -> Result<String, (i64, String)> {
    Ok(arg_str(args, "inbox")?.unwrap_or_else(|| "default".into()))
}

/// The `id` argument, required on the by-id tools: absence keeps its
/// preserved sentence; a wrong type refuses by name.
fn required_id(args: &Value) -> Result<String, (i64, String)> {
    arg_str(args, "id")?.ok_or_else(|| (-32602, "missing id".to_string()))
}

async fn tool_call(ctx: &McpContext, req: &Value) -> Result<Value, (i64, String)> {
    let params = req.pointer("/params").unwrap_or(&Value::Null);
    let name = arg_str(params, "name")?.ok_or_else(|| (-32602, "missing tool name".to_string()))?;
    let a = args(req);
    let store = &ctx.store;

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
            // the limit's business, not the total's.
            let limit = arg_usize(a, "limit")?.unwrap_or(crate::api::DEFAULT_LIST_LIMIT);
            let all = store.list(&inbox, &filter);
            let total = all.len();
            let emails: Vec<Value> = all
                .into_iter()
                .take(limit)
                .map(|e| serde_json::to_value(&*e).unwrap())
                .collect();
            Ok(text_result(
                json!({ "inbox": inbox, "total": total, "emails": emails }),
            ))
        }
        "swarmail_get_email" => {
            let id = required_id(a)?;
            match store.get(&id) {
                Some(e) => Ok(text_result(serde_json::to_value(&*e).unwrap())),
                None => Ok(error_result(format!("message {id} not found"))),
            }
        }
        "swarmail_get_latest_email" => {
            let inbox = arg_inbox(a)?;
            // A supplied `count: 0` refuses: the schema publishes
            // `minimum: 1`, and a silent `.max(1)` would invent a page the
            // caller did not ask for. The *wait's* `count: 0` keeps 0010's
            // at-least-one semantic — a different domain, a different rule.
            let count = match arg_usize(a, "count")? {
                None => 1,
                Some(0) => return Err(refusal("count", "an integer of at least 1")),
                Some(n) => n,
            };
            let emails: Vec<Value> = store
                .list(&inbox, &Filter::default())
                .into_iter()
                .take(count)
                .map(|e| serde_json::to_value(&*e).unwrap())
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
            let filter = Filter {
                to: arg_str(a, "to")?,
                from: arg_str(a, "from")?,
                subject: arg_str(a, "subject")?,
                since_ms: arg_i64(a, "since_ms")?,
            };
            // `count: 0` still means at least one (decision 0010).
            let want = arg_usize(a, "count")?.unwrap_or(1).max(1);
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
                outcome = store.wait_for(&inbox, &filter, want, timeout) => Some(outcome),
                _ = crate::api::shutdown_signal(ctx.shutdown.clone()) => None,
            };
            // The total is counted *after* the wait, in both arms: it
            // describes the response's observed state (decision 0018) — the
            // same residual the REST await names, bounded by one insert.
            let total = store.count(&inbox, &Filter::default());
            match outcome {
                Some(WaitOutcome::Found(matches)) => {
                    let emails: Vec<Value> = matches
                        .iter()
                        .map(|e| serde_json::to_value(&**e).unwrap())
                        .collect();
                    Ok(text_result(
                        json!({ "matched": want, "total": total, "emails": emails }),
                    ))
                }
                Some(WaitOutcome::Timeout { matched }) => Ok(text_result(
                    json!({ "matched": matched, "total": total, "timed_out": true }),
                )),
                None => Ok(error_result("shutting down".into())),
            }
        }
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
            // with the fixed text — the same vocabulary as a refused fixture —
            // while the refused mail stays queryable in memory.
            match store.clone().insert_async(email).await {
                Ok(_) => Ok(text_result(
                    json!({ "id": id, "links": links, "codes": codes }),
                )),
                Err(_) => Ok(error_result("storage commit failed".to_string())),
            }
        }
        "swarmail_set_chaos" => {
            // An unknown argument key refuses (decision 0029 §3): a typo'd
            // key must not arm nothing while answering success.
            if let Some(keys) = a.as_object() {
                for key in keys.keys() {
                    if !matches!(key.as_str(), "event" | "probability" | "error" | "delay_ms") {
                        return Err((
                            -32602,
                            format!(
                                "unknown argument: {key} (use event|probability|error|delay_ms)"
                            ),
                        ));
                    }
                }
            }
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
        other => Err((-32601, format!("unknown tool: {other}"))),
    }
}

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
    })
});

static SCHEMA_INBOX_ONLY: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!({
        "type": "object",
        "properties": { "inbox": { "type": "string", "description": "Inbox name (default: default)" } },
    })
});

static TOOLS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!([
        { "name": "swarmail_search_emails", "description": "Search captured emails in an inbox by recipient, sender, subject or arrival time. Returns full emails (bodies, links, codes included) and the full match count as total.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string" }, "from": { "type": "string" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer", "description": "Arrival time lower bound in ms since the epoch; negative values are legal" },
            "limit": { "type": "integer", "minimum": 0, "default": 100 } } } },
        { "name": "swarmail_get_email", "description": "Fetch one captured email in full by id.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_get_latest_email", "description": "Fetch the most recent email(s) in an inbox.",
          "inputSchema": { "type": "object", "properties": { "inbox": { "type": "string" }, "count": { "type": "integer", "minimum": 1, "default": 1 } } } },
        { "name": "swarmail_delete_email", "description": "Delete one captured email by id (start the next assertion clean).",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_clear_inbox", "description": "Delete all emails in one inbox — the 'clear' of the clear/act/assert loop.",
          "inputSchema": SCHEMA_INBOX_ONLY.clone() },
        { "name": "swarmail_clear_all", "description": "Delete every email in every inbox.", "inputSchema": { "type": "object" } },
        { "name": "swarmail_wait_for_email", "description": "Block until count emails matching the filter arrive (push-based, no polling). Returns the matched emails. Waits at most timeout_ms (60000 at most) and races the server's shutdown, answering an error once it begins stopping. A count of 0 still means at least one (decision 0010).",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string" }, "from": { "type": "string" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer", "description": "Arrival time lower bound in ms since the epoch; negative values are legal" },
            "count": { "type": "integer", "minimum": 0, "default": 1, "description": "Matched emails to wait for; 0 still means at least one" },
            "timeout_ms": { "type": "integer", "default": 5000, "minimum": 0, "maximum": 60000 } } } },
        { "name": "swarmail_extract_links", "description": "Extract all http(s) URLs from a captured email's text/HTML bodies.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_extract_codes", "description": "Extract likely OTP/verification codes from a captured email.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_seed_email", "description": "Inject a synthetic email into an inbox without sending SMTP — fixtures for agent tests. HTML is preserved verbatim and supersedes text; envelope fields (to, from, subject) containing CR, LF or NUL are refused before anything is stored.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "from": { "type": "string" }, "to": { "type": "string" },
            "subject": { "type": "string" }, "text": { "type": "string" }, "html": { "type": "string" } },
            "required": ["to"] } },
        { "name": "swarmail_set_chaos", "description": "Enable deliberate SMTP failure: event = connect|mail_from|rcpt|data, probability 0-100, optional SMTP error line (a single line starting with a 4xx or 5xx code, at most 510 octets) and delay_ms (at most 300000).",
          "inputSchema": { "type": "object", "properties": {
            "event": { "type": "string", "enum": ["connect", "mail_from", "rcpt", "data"] },
            "probability": { "type": "integer", "minimum": 0, "maximum": 100 },
            "error": { "type": "string", "description": "A single-line SMTP reply starting with a 4xx or 5xx code, at most 510 octets" },
            "delay_ms": { "type": "integer", "minimum": 0, "maximum": 300000 } },
            "required": ["event", "probability"] } },
        { "name": "swarmail_clear_chaos", "description": "Disable all chaos rules.", "inputSchema": { "type": "object" } },
    ])
});

#[cfg(test)]
mod tests {
    use super::*;

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
        let latest = find("swarmail_get_latest_email");
        assert_eq!(latest["count"]["minimum"], 1, "{latest}");
    }
}
