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

use crate::chaos::{Chaos, ChaosEvent, ChaosRule, PROBABILITY_REFUSAL};
use crate::smtp::build_seed_email;
use crate::store::{Filter, Store, WaitOutcome};
use serde_json::{Value, json};
use std::sync::Arc;

pub const PROTOCOL_VERSION: &str = "2025-03-26";

pub struct McpContext {
    pub store: Arc<Store>,
    pub chaos: Arc<Chaos>,
    /// The server's stop watch: the wait tool races its wait against it so a
    /// parked client cannot hold `stop()` for its timeout's remainder
    /// (decision 0018). The same receiver the SSE feed consumes — one
    /// source of "stopping", never a second channel.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}

/// Handle one JSON-RPC request. Returns `None` for notifications (no reply).
pub async fn handle(ctx: &McpContext, req: &Value) -> Option<Value> {
    let method = req.get("method")?.as_str()?.to_string();
    if method.starts_with("notifications/") {
        return None;
    }
    let id = req.get("id").cloned().unwrap_or(Value::Null);

    let result: Result<Value, String> = match method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": req.pointer("/params/protocolVersion").cloned().unwrap_or(json!(PROTOCOL_VERSION)),
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": { "name": "swarmail", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": TOOLS.clone() })),
        "tools/call" => tool_call(ctx, req).await,
        other => Err(format!("unknown method: {other}")),
    };

    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": e } }),
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

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn arg_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|v| v as usize)
}

async fn tool_call(ctx: &McpContext, req: &Value) -> Result<Value, String> {
    let name = req
        .pointer("/params/name")
        .and_then(|v| v.as_str())
        .ok_or("missing tool name")?
        .to_string();
    let a = args(req);
    let store = &ctx.store;

    match name.as_str() {
        "swarmail_search_emails" => {
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            let filter = Filter {
                to: arg_str(a, "to"),
                from: arg_str(a, "from"),
                subject: arg_str(a, "subject"),
                since_ms: a.get("since_ms").and_then(|v| v.as_i64()),
            };
            let limit = arg_usize(a, "limit").unwrap_or(50);
            let emails: Vec<Value> = store
                .list(&inbox, &filter)
                .into_iter()
                .take(limit)
                .map(|e| serde_json::to_value(&*e).unwrap())
                .collect();
            Ok(text_result(
                json!({ "inbox": inbox, "total": emails.len(), "emails": emails }),
            ))
        }
        "swarmail_get_email" => {
            let id = arg_str(a, "id").ok_or("missing id")?;
            match store.get(&id) {
                Some(e) => Ok(text_result(serde_json::to_value(&*e).unwrap())),
                None => Ok(error_result(format!("message {id} not found"))),
            }
        }
        "swarmail_get_latest_email" => {
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            let count = arg_usize(a, "count").unwrap_or(1).max(1);
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
            let id = arg_str(a, "id").ok_or("missing id")?;
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
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
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
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            let filter = Filter {
                to: arg_str(a, "to"),
                from: arg_str(a, "from"),
                subject: arg_str(a, "subject"),
                since_ms: a.get("since_ms").and_then(|v| v.as_i64()),
            };
            let want = arg_usize(a, "count").unwrap_or(1).max(1);
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
            // (decision 0018). Both refusals answer before `store.wait_for`
            // — a rejected wait registers no waiter.
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
            let id = arg_str(a, "id").ok_or("missing id")?;
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
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            let to = arg_str(a, "to").ok_or("missing to")?;
            let from = arg_str(a, "from");
            let subject = arg_str(a, "subject");
            let text = arg_str(a, "text");
            let html = arg_str(a, "html");
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
            let event = arg_str(a, "event").ok_or("missing event")?;
            let event_enum: ChaosEvent = serde_json::from_value(json!(event))
                .map_err(|_| format!("unknown event: {event} (use connect|mail_from|rcpt|data)"))?;
            // Wire values are never narrowed (decision 0018): the argument
            // must be present, must be an integer at all, and must fit
            // `u8` (256 refusing — not becoming 0). The 0..=100 ceiling is
            // the domain's own rule (decision 0025): `rule.validate()` is
            // its one owner — this tool carries no parallel copy. Each
            // stage refuses by name; no cast, no clamp, no silent default.
            let probability_value = a.get("probability").ok_or(PROBABILITY_REFUSAL)?;
            let probability_u64 = probability_value.as_u64().ok_or(PROBABILITY_REFUSAL)?;
            let probability = u8::try_from(probability_u64).map_err(|_| PROBABILITY_REFUSAL)?;
            let delay_ms = match a.get("delay_ms") {
                None => 0,
                Some(v) => v
                    .as_u64()
                    .ok_or("delay_ms must be a non-negative integer")?,
            };
            let rule = ChaosRule {
                probability,
                error: arg_str(a, "error"),
                delay_ms,
            };
            // The ceiling refusal lands in the JSON-RPC `error` envelope —
            // exactly what `?` already produces for the staging refusals
            // above, and what `set_chaos_refuses_every_malformed_wire_value_
            // by_name` pins byte-for-byte.
            rule.validate()?;
            let mut cfg = ctx.chaos.current();
            match event_enum {
                ChaosEvent::Connect => cfg.connect = Some(rule),
                ChaosEvent::MailFrom => cfg.mail_from = Some(rule),
                ChaosEvent::Rcpt => cfg.rcpt = Some(rule),
                ChaosEvent::Data => cfg.data = Some(rule),
            }
            ctx.chaos.set(cfg);
            Ok(text_result(
                serde_json::to_value(ctx.chaos.current()).unwrap(),
            ))
        }
        "swarmail_clear_chaos" => {
            ctx.chaos.clear();
            Ok(text_result(json!({ "chaos": "disabled" })))
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

/// MCP-compatible JSON input schemas. These are objects — the MCP spec
/// requires an object, and a `&str` schema embedded in `json!` serialized as
/// a JSON *string*, publishing an invalid schema for five tools (decision
/// 0010, issue #4). The fields are the verbatim re-kind of the old string
/// constants: same names, types, descriptions and `required` arrays.
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
        { "name": "swarmail_search_emails", "description": "Search captured emails in an inbox by recipient, sender, subject or arrival time. Returns full emails (bodies, links, codes included).",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string" }, "from": { "type": "string" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer" }, "limit": { "type": "integer" } } } },
        { "name": "swarmail_get_email", "description": "Fetch one captured email in full by id.",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_get_latest_email", "description": "Fetch the most recent email(s) in an inbox.",
          "inputSchema": { "type": "object", "properties": { "inbox": { "type": "string" }, "count": { "type": "integer" } } } },
        { "name": "swarmail_delete_email", "description": "Delete one captured email by id (start the next assertion clean).",
          "inputSchema": SCHEMA_EMAIL_ID.clone() },
        { "name": "swarmail_clear_inbox", "description": "Delete all emails in one inbox — the 'clear' of the clear/act/assert loop.",
          "inputSchema": SCHEMA_INBOX_ONLY.clone() },
        { "name": "swarmail_clear_all", "description": "Delete every email in every inbox.", "inputSchema": { "type": "object" } },
        { "name": "swarmail_wait_for_email", "description": "Block until count emails matching the filter arrive (push-based, no polling). Returns the matched emails. Waits at most timeout_ms (60000 at most) and races the server's shutdown, answering an error once it begins stopping.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string" }, "from": { "type": "string" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer" },
            "count": { "type": "integer", "minimum": 1 },
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
        { "name": "swarmail_set_chaos", "description": "Enable deliberate SMTP failure: event = connect|mail_from|rcpt|data, probability 0-100, optional SMTP error line and delay_ms.",
          "inputSchema": { "type": "object", "properties": {
            "event": { "type": "string", "enum": ["connect", "mail_from", "rcpt", "data"] },
            "probability": { "type": "integer", "minimum": 0, "maximum": 100 },
            "error": { "type": "string" }, "delay_ms": { "type": "integer", "minimum": 0 } },
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

        let chaos = find("swarmail_set_chaos");
        assert_eq!(chaos["probability"]["minimum"], 0, "{chaos}");
        assert_eq!(
            chaos["probability"]["maximum"],
            crate::chaos::PROBABILITY_MAX,
            "{chaos}"
        );
    }
}
