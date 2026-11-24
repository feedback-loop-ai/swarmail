//! What one observed client launch proves (issue #392, proposed decision
//! 0140): the pure judge of the Inspector CLI route. Each launch is judged
//! from its own observer's exchanges, its own handshake and the result the
//! client printed; nothing here reads a socket or a clock.
//!
//! A step passes only when the launch's one handshake succeeded at the
//! first launch's revision, the client sent the expected request with the
//! checklist's arguments exactly once, Swarmail's paired reply carries the
//! expected values, and the client exited successfully having printed a
//! result that carries them too. A successful exit with missing wire
//! evidence fails, and so does wire evidence the client did not consume.

use crate::capture::Exchange;
use serde_json::{Value, json};

use super::receipt::{Outcome, TOOLS};

/// The checklist fixture's known values (`docs/agents/client-matrix.md`).
pub const INBOX: &str = "default";
pub const SUBJECT: &str = "Swarmail client matrix fixture";
pub const CODE: &str = "482913";
pub const ATTACHMENT: &str = "matrix.txt";
pub const ATTACHMENT_TYPE: &str = "text/plain";
pub const ATTACHMENT_SIZE: u64 = 13;

/// The longest failure reason a receipt carries.
const REASON_LIMIT: usize = 200;

/// One launch's request: `tools/list`, or one `tools/call`.
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    ListTools,
    Tool {
        name: &'static str,
        arguments: Value,
    },
}

/// The five launches after the fixture is stored as `id`: the step each
/// one proves and its call, in checklist order.
pub fn plan(id: &str) -> [(&'static str, Call); 5] {
    [
        ("list_tools", Call::ListTools),
        (
            "read",
            Call::Tool {
                name: TOOLS[0],
                arguments: json!({"inbox": INBOX, "count": 1}),
            },
        ),
        (
            "wait_for_code",
            Call::Tool {
                name: TOOLS[1],
                arguments: json!({"inbox": INBOX, "timeout_ms": 0}),
            },
        ),
        (
            "structured_content",
            Call::Tool {
                name: TOOLS[2],
                arguments: json!({"inbox": INBOX}),
            },
        ),
        (
            "list_attachments",
            Call::Tool {
                name: TOOLS[3],
                arguments: json!({"id": id}),
            },
        ),
    ]
}

impl Call {
    /// The Inspector CLI's arguments for this call after its URL.
    pub fn inspector_args(&self) -> Vec<String> {
        let mut args = vec!["--transport".to_string(), "http".to_string()];
        match self {
            Call::ListTools => args.extend(["--method".into(), "tools/list".into()]),
            Call::Tool { name, arguments } => {
                args.extend([
                    "--method".into(),
                    "tools/call".into(),
                    "--tool-name".into(),
                    name.to_string(),
                ]);
                for (key, value) in arguments.as_object().into_iter().flatten() {
                    let value = match value {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    };
                    args.extend(["--tool-arg".into(), format!("{key}={value}")]);
                }
            }
        }
        args
    }

    fn describe(&self) -> String {
        match self {
            Call::ListTools => "tools/list".into(),
            Call::Tool { name, .. } => format!("tools/call of {name}"),
        }
    }
}

/// How the client process ended: its success and printed stdout, or why
/// it did not finish (a timeout, an oversized output, a failed wait).
pub type ClientEnd = Result<(bool, Vec<u8>), String>;

/// One observed client launch.
pub struct Launch {
    pub exchanges: Vec<Exchange>,
    pub overflowed: bool,
    pub client: ClientEnd,
}

/// One JSON-RPC request of a launch and the reply paired with it.
struct Pair {
    method: String,
    params: Value,
    status: u16,
    reply: Option<Value>,
}

fn messages(bytes: &[u8]) -> Vec<Value> {
    match serde_json::from_slice(bytes) {
        Ok(Value::Array(items)) => items,
        Ok(object @ Value::Object(_)) => vec![object],
        _ => Vec::new(),
    }
}

/// Pair every request (a message with a method and an id) with the reply
/// of the same id in the same exchange. An id reused within one launch is
/// ambiguous; across launches it is ordinary.
fn pairs(launch: &Launch) -> Result<Vec<Pair>, String> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for exchange in launch.exchanges.iter().filter(|e| e.post) {
        let (status, replies) = match &exchange.reply {
            Some((status, body)) => (*status, messages(body)),
            None => (502, Vec::new()),
        };
        for request in messages(&exchange.request) {
            let (Some(method), Some(id)) = (request["method"].as_str(), request.get("id")) else {
                continue;
            };
            if seen.contains(id) {
                return Err(format!("request id {id} was reused within one launch"));
            }
            seen.push(id.clone());
            let reply = replies
                .iter()
                .find(|r| r.get("id") == Some(id) && r.get("method").is_none())
                .cloned();
            out.push(Pair {
                method: method.to_string(),
                params: request.get("params").cloned().unwrap_or(Value::Null),
                status,
                reply,
            });
        }
    }
    Ok(out)
}

/// The launch's one handshake: its reply and revision, or why it is not
/// usable, with whatever reply was captured.
fn handshake(pairs: &[Pair]) -> Result<(Value, String), (String, Option<Value>)> {
    let inits: Vec<&Pair> = pairs.iter().filter(|p| p.method == "initialize").collect();
    let init = match inits.as_slice() {
        [] => return Err(("no initialize request was observed".into(), None)),
        [one] => *one,
        many => {
            return Err((
                format!("{} initialize requests in one launch", many.len()),
                None,
            ));
        }
    };
    let Some(reply) = init.reply.clone() else {
        return Err(("initialize got no JSON-RPC reply".into(), None));
    };
    if let Some(error) = reply.get("error") {
        let code = error["code"].as_i64().map_or("?".into(), |c| c.to_string());
        return Err((
            format!("initialize returned JSON-RPC error {code}"),
            Some(reply),
        ));
    }
    if !(200..300).contains(&init.status) {
        let status = init.status;
        return Err((format!("initialize answered HTTP {status}"), Some(reply)));
    }
    match reply["result"]["protocolVersion"].as_str() {
        Some(revision) if !revision.trim().is_empty() => {
            let revision = revision.to_string();
            Ok((reply, revision))
        }
        _ => Err((
            "initialize reply has no protocolVersion".into(),
            Some(reply),
        )),
    }
}

/// The first launch's handshake as the receipt records it: the
/// `initialize` outcome, the captured reply and the negotiated revision.
pub fn first_handshake(launch: &Launch) -> (Outcome, Option<Value>, Option<String>) {
    let attempt = pairs(launch).map_err(|reason| (reason, None));
    match attempt.and_then(|p| handshake(&p)) {
        Ok((reply, revision)) => (Outcome::pass(), Some(reply), Some(revision)),
        Err((reason, reply)) => {
            let revision = reply
                .as_ref()
                .and_then(|r| r["result"]["protocolVersion"].as_str().map(str::to_string));
            (fail(&reason), reply, revision)
        }
    }
}

fn fail(reason: &str) -> Outcome {
    Outcome::fail(sanitize(reason))
}

/// One printable line of at most [`REASON_LIMIT`] characters.
pub fn sanitize(reason: &str) -> String {
    let line: String = reason
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(REASON_LIMIT)
        .collect();
    if line.trim().is_empty() {
        "unspecified failure".into()
    } else {
        line
    }
}

/// Judge one launch of `step`'s `call` against the first launch's
/// `revision` and the stored `id`.
pub fn judge(step: &str, call: &Call, launch: &Launch, revision: &str, id: &str) -> Outcome {
    match judged(step, call, launch, revision, id) {
        Ok(()) => Outcome::pass(),
        Err(reason) => fail(&reason),
    }
}

fn judged(
    step: &str,
    call: &Call,
    launch: &Launch,
    revision: &str,
    id: &str,
) -> Result<(), String> {
    if launch.overflowed {
        return Err("the launch's traffic exceeded the observer's bound".into());
    }
    let pairs = pairs(launch)?;
    let (_, own) = handshake(&pairs).map_err(|(reason, _)| format!("handshake: {reason}"))?;
    if own != revision {
        return Err(format!(
            "this launch negotiated {own}, the first launch {revision}"
        ));
    }
    let (method, tool) = match call {
        Call::ListTools => ("tools/list", None),
        Call::Tool { name, .. } => ("tools/call", Some(*name)),
    };
    let mut matching = Vec::new();
    for pair in pairs.iter().filter(|p| p.method == method) {
        let name = pair.params["name"].as_str();
        if tool.is_some() && name != tool {
            return Err(format!(
                "unexpected tools/call of {}",
                name.unwrap_or("an unnamed tool")
            ));
        }
        matching.push(pair);
    }
    let pair = match matching.as_slice() {
        [] => return Err(format!("the client sent no {}", call.describe())),
        [one] => *one,
        many => {
            return Err(format!(
                "the client sent {} {}",
                many.len(),
                call.describe()
            ));
        }
    };
    if let Call::Tool { arguments, .. } = call {
        let sent = pair.params.get("arguments").cloned().unwrap_or(Value::Null);
        if &sent != arguments {
            return Err(format!("the client sent arguments {sent}, not {arguments}"));
        }
    }
    let reply = pair
        .reply
        .as_ref()
        .ok_or("the call got no JSON-RPC reply")?;
    if !(200..300).contains(&pair.status) {
        return Err(format!("the call answered HTTP {}", pair.status));
    }
    if reply.get("error").is_some() {
        return Err("the call returned a JSON-RPC error".into());
    }
    let result = reply.get("result").ok_or("the reply has no result")?;
    if result["isError"] == json!(true) {
        return Err("the tool returned isError".into());
    }
    expect(step, result, id).map_err(|reason| format!("Swarmail's reply: {reason}"))?;
    let (success, stdout) = launch
        .client
        .as_ref()
        .map_err(|e| format!("the client did not finish: {e}"))?;
    if !success {
        return Err("the client exited with a failure status".into());
    }
    let printed: Value =
        serde_json::from_slice(stdout).map_err(|_| "the client printed no JSON result")?;
    expect(step, &printed, id).map_err(|reason| format!("the client's result: {reason}"))
}

/// The JSON payload of a tool result: its text block, else its
/// `structuredContent` object.
fn payload(result: &Value) -> Result<Value, String> {
    let text = result["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str())
        .and_then(|t| serde_json::from_str::<Value>(t).ok())
        .filter(Value::is_object);
    text.or_else(|| Some(result["structuredContent"].clone()).filter(Value::is_object))
        .ok_or_else(|| "no JSON payload in the tool result".to_string())
}

/// The checklist's pass condition for `step` on one tool result.
pub fn expect(step: &str, result: &Value, id: &str) -> Result<(), String> {
    match step {
        "list_tools" => {
            let names: Vec<&str> = result["tools"]
                .as_array()
                .ok_or("no tools array")?
                .iter()
                .filter_map(|t| t["name"].as_str())
                .collect();
            match TOOLS.iter().find(|tool| !names.contains(tool)) {
                Some(missing) => Err(format!("{missing} is not listed")),
                None => Ok(()),
            }
        }
        "read" => {
            let body = payload(result)?;
            match body["emails"].as_array().map(Vec::as_slice) {
                Some([email]) if email["id"] == id && email["subject"] == SUBJECT => Ok(()),
                Some([_]) => Err("the email is not the stored fixture".into()),
                _ => Err("not exactly one email".into()),
            }
        }
        "wait_for_code" => {
            let body = payload(result)?;
            let selected = &body["selected"];
            if body["status"] != "selected" {
                Err(format!("status is {}, not selected", body["status"]))
            } else if selected["code"] != CODE || selected["email_id"] != id {
                Err("the selected code is not the fixture's".into())
            } else {
                Ok(())
            }
        }
        "structured_content" => {
            let structured = result
                .get("structuredContent")
                .filter(|s| s.is_object())
                .ok_or("text block only, no structuredContent object")?;
            let found = structured["emails"]
                .as_array()
                .is_some_and(|emails| emails.iter().any(|e| e["id"] == id));
            if found {
                Ok(())
            } else {
                Err("structuredContent.emails lacks the stored ID".into())
            }
        }
        "list_attachments" => {
            let body = payload(result)?;
            let listed = body["attachments"].as_array().is_some_and(|all| {
                all.iter().any(|a| {
                    a["filename"] == ATTACHMENT
                        && a["content_type"] == ATTACHMENT_TYPE
                        && a["size"] == ATTACHMENT_SIZE
                })
            });
            if body["id"] != id {
                Err("the listing is not of the stored ID".into())
            } else if !listed {
                Err(format!(
                    "no {ATTACHMENT} entry of {ATTACHMENT_TYPE}, size {ATTACHMENT_SIZE}"
                ))
            } else {
                Ok(())
            }
        }
        other => Err(format!("{other} is not a judged step")),
    }
}

/// Replace, in place, every string of `value` that contains one of
/// `secrets` (a host name, a path, an account) with `"<redacted>"`. The
/// JSON-RPC envelope's `jsonrpc`, `id` and the `protocolVersion` stay.
pub fn redact(value: &mut Value, secrets: &[String]) {
    fn walk(value: &mut Value, secrets: &[String], keep: bool) {
        match value {
            Value::String(text) if !keep && secrets.iter().any(|s| text.contains(s.as_str())) => {
                *text = "<redacted>".into();
            }
            Value::Array(items) => items.iter_mut().for_each(|v| walk(v, secrets, false)),
            Value::Object(map) => {
                for (key, v) in map.iter_mut() {
                    let keep = matches!(key.as_str(), "jsonrpc" | "id" | "protocolVersion");
                    walk(v, secrets, keep);
                }
            }
            _ => {}
        }
    }
    let secrets: Vec<String> = secrets
        .iter()
        .filter(|s| s.trim().len() > 1)
        .cloned()
        .collect();
    walk(value, &secrets, false);
}
