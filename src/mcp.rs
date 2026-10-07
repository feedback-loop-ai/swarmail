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

use crate::chaos::{Chaos, ChaosEvent, ChaosRule};
use crate::smtp::build_email;
use crate::store::{Filter, Store, WaitOutcome};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

pub const PROTOCOL_VERSION: &str = "2025-03-26";

pub struct McpContext {
    pub store: Arc<Store>,
    pub chaos: Arc<Chaos>,
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
            store
                .get(&id)
                .map(|e| text_result(serde_json::to_value(&*e).unwrap()))
                .ok_or_else(|| format!("message {id} not found"))
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
            Ok(text_result(json!({ "emails": emails })))
        }
        "swarmail_delete_email" => {
            let id = arg_str(a, "id").ok_or("missing id")?;
            if store.delete(&id) {
                Ok(text_result(json!({ "deleted": true })))
            } else {
                Ok(error_result(format!("message {id} not found")))
            }
        }
        "swarmail_clear_inbox" => {
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            Ok(text_result(json!({ "removed": store.clear(&inbox) })))
        }
        "swarmail_clear_all" => Ok(text_result(json!({ "removed": store.clear_all() }))),
        "swarmail_wait_for_email" => {
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            let filter = Filter {
                to: arg_str(a, "to"),
                from: arg_str(a, "from"),
                subject: arg_str(a, "subject"),
                since_ms: a.get("since_ms").and_then(|v| v.as_i64()),
            };
            let want = arg_usize(a, "count").unwrap_or(1).max(1);
            let timeout =
                Duration::from_millis(a.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(5000));
            match store.wait_for(&inbox, &filter, want, timeout).await {
                WaitOutcome::Found(matches) => {
                    let emails: Vec<Value> = matches
                        .iter()
                        .map(|e| serde_json::to_value(&**e).unwrap())
                        .collect();
                    Ok(text_result(json!({ "matched": want, "emails": emails })))
                }
                WaitOutcome::Timeout { matched } => Ok(text_result(
                    json!({ "matched": matched, "timed_out": true }),
                )),
            }
        }
        "swarmail_extract_links" | "swarmail_extract_codes" => {
            let id = arg_str(a, "id").ok_or("missing id")?;
            let email = store
                .get(&id)
                .ok_or_else(|| format!("message {id} not found"))?;
            if name.ends_with("links") {
                Ok(text_result(json!({ "links": email.links })))
            } else {
                Ok(text_result(json!({ "codes": email.codes })))
            }
        }
        "swarmail_seed_email" => {
            let inbox = arg_str(a, "inbox").unwrap_or_else(|| "default".into());
            let to = arg_str(a, "to").ok_or("missing to")?;
            let from = arg_str(a, "from").unwrap_or_else(|| "fixture@swarmail.dev".into());
            let subject = arg_str(a, "subject").unwrap_or_else(|| "(no subject)".into());
            let body = arg_str(a, "text").unwrap_or_default();
            let raw = format!("From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{body}\r\n");
            let email = build_email(raw.as_bytes(), &inbox, &[to], from);
            let id = email.id.clone();
            let links = email.links.clone();
            let codes = email.codes.clone();
            store.insert(email);
            Ok(text_result(
                json!({ "id": id, "links": links, "codes": codes }),
            ))
        }
        "swarmail_set_chaos" => {
            let event = arg_str(a, "event").ok_or("missing event")?;
            let probability = a.get("probability").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let rule = ChaosRule {
                probability,
                error: arg_str(a, "error"),
                delay_ms: a.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(0),
            };
            let event_enum: ChaosEvent = serde_json::from_value(json!(event))
                .map_err(|_| format!("unknown event: {event} (use connect|mail_from|rcpt|data)"))?;
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

/// MCP-compatible JSON input schemas.
const SCHEMA_EMAIL_ID: &str = r#"{"type":"object","properties":{"id":{"type":"string","description":"Email id"}},"required":["id"]}"#;
const SCHEMA_INBOX_ONLY: &str = r#"{"type":"object","properties":{"inbox":{"type":"string","description":"Inbox name (default: default)"}}}"#;

static TOOLS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!([
        { "name": "swarmail_search_emails", "description": "Search captured emails in an inbox by recipient, sender, subject or arrival time. Returns full emails (bodies, links, codes included).",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string" }, "from": { "type": "string" },
            "subject": { "type": "string" }, "since_ms": { "type": "integer" }, "limit": { "type": "integer" } } } },
        { "name": "swarmail_get_email", "description": "Fetch one captured email in full by id.",
          "inputSchema": SCHEMA_EMAIL_ID },
        { "name": "swarmail_get_latest_email", "description": "Fetch the most recent email(s) in an inbox.",
          "inputSchema": { "type": "object", "properties": { "inbox": { "type": "string" }, "count": { "type": "integer" } } } },
        { "name": "swarmail_delete_email", "description": "Delete one captured email by id (start the next assertion clean).",
          "inputSchema": SCHEMA_EMAIL_ID },
        { "name": "swarmail_clear_inbox", "description": "Delete all emails in one inbox — the 'clear' of the clear/act/assert loop.",
          "inputSchema": SCHEMA_INBOX_ONLY },
        { "name": "swarmail_clear_all", "description": "Delete every email in every inbox.", "inputSchema": { "type": "object" } },
        { "name": "swarmail_wait_for_email", "description": "Block until count emails matching the filter arrive (push-based, no polling). Returns the matched emails.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "to": { "type": "string" }, "from": { "type": "string" },
            "subject": { "type": "string" }, "count": { "type": "integer" }, "timeout_ms": { "type": "integer" } } } },
        { "name": "swarmail_extract_links", "description": "Extract all http(s) URLs from a captured email's text/HTML bodies.",
          "inputSchema": SCHEMA_EMAIL_ID },
        { "name": "swarmail_extract_codes", "description": "Extract likely OTP/verification codes from a captured email.",
          "inputSchema": SCHEMA_EMAIL_ID },
        { "name": "swarmail_seed_email", "description": "Inject a synthetic email into an inbox without sending SMTP — fixtures for agent tests.",
          "inputSchema": { "type": "object", "properties": {
            "inbox": { "type": "string" }, "from": { "type": "string" }, "to": { "type": "string" },
            "subject": { "type": "string" }, "text": { "type": "string" }, "html": { "type": "string" } },
            "required": ["to"] } },
        { "name": "swarmail_set_chaos", "description": "Enable deliberate SMTP failure: event = connect|mail_from|rcpt|data, probability 0-100, optional SMTP error line and delay_ms.",
          "inputSchema": { "type": "object", "properties": {
            "event": { "type": "string", "enum": ["connect", "mail_from", "rcpt", "data"] },
            "probability": { "type": "integer" }, "error": { "type": "string" }, "delay_ms": { "type": "integer" } },
            "required": ["event", "probability"] } },
        { "name": "swarmail_clear_chaos", "description": "Disable all chaos rules.", "inputSchema": { "type": "object" } },
    ])
});
