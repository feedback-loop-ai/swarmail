//! A minimal client for Swarmail's MCP HTTP endpoint (`POST /mcp`) as it
//! ships today: protocol `2025-03-26`, tool results as JSON inside one
//! text block, no `structuredContent` (epic #164). Ids are correlated, a
//! JSON-RPC error or an `isError` result is an error — never data.
//!
//! Requests carry the URL's own loopback `Host` and no `Origin`, which is
//! exactly what decision 0043's guard admits.

use serde_json::{Value, json};
use std::time::Duration;

/// One reply's byte bound (0132 §7, spec AC7): the native clients' bound,
/// so no reply — initialization and JSON-RPC errors included — is buffered
/// past it.
pub const MAX_REPLY_BYTES: u64 = 16 << 20;
/// One logical operation's byte bound over every reply it reads, shared
/// across nested calls (a paged scan is one operation).
pub const MAX_OPERATION_BYTES: u64 = 1 << 30;

/// The marker of a byte-bound refusal in an error string, so a caller can
/// classify it without parsing prose.
pub const OVERSIZED: &str = "oversized reply";
/// The marker of a refused redirect.
pub const REDIRECT: &str = "refused redirect";

pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Above the server's 60 s wait ceiling, so a maximal wait is answered.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(75);

pub struct McpClient {
    http: reqwest::Client,
    url: String,
    next_id: u64,
    /// `tools/call` requests sent: the oracle's MCP cost (proposed 0123 §6).
    /// `initialize` and the initialized notification are session setup, not
    /// calls, and are not counted.
    tool_calls: u32,
    /// The last id this client may use: `u64::MAX` for an ordinary client, the
    /// end of its range for a benchmark client (0132 §7, spec AC7).
    last_id: u64,
}

impl McpClient {
    /// Initialize a session against `url` (which ends in `/mcp`).
    pub async fn connect(url: &str) -> Result<McpClient, String> {
        McpClient::connect_ids(url, 1..u64::MAX).await
    }

    /// A benchmark client: every JSON-RPC id it sends, initialization
    /// included, is drawn from `ids`, so clients given disjoint ranges share
    /// one bridge without colliding. Past the range it refuses to send.
    pub async fn connect_range(url: &str, ids: std::ops::Range<u64>) -> Result<McpClient, String> {
        if ids.start == 0 || ids.is_empty() {
            return Err(format!("id range {ids:?} is empty or starts at 0"));
        }
        McpClient::connect_ids(url, ids).await
    }

    async fn connect_ids(url: &str, ids: std::ops::Range<u64>) -> Result<McpClient, String> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let mut client = McpClient {
            http,
            url: url.to_string(),
            next_id: ids.start - 1,
            tool_calls: 0,
            last_id: ids.end - 1,
        };
        let init = client
            .rpc(
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "swarmail-evals", "version": env!("CARGO_PKG_VERSION") },
                }),
            )
            .await?;
        if init["protocolVersion"] != PROTOCOL_VERSION {
            return Err(format!("server negotiated {}", init["protocolVersion"]));
        }
        client
            .notify("notifications/initialized")
            .await
            .map_err(|e| format!("initialized notification: {e}"))?;
        Ok(client)
    }

    async fn post(&self, body: &Value) -> Result<(u16, Vec<u8>), String> {
        let mut spent = 0;
        self.post_within(body, None, &mut spent).await
    }

    /// One POST whose reply is streamed under [`MAX_REPLY_BYTES`] and the
    /// operation's shared `spent` budget under [`MAX_OPERATION_BYTES`]: each
    /// chunk is accounted, checked, before it is appended, and a redirect is
    /// refused rather than followed.
    async fn post_within(
        &self,
        body: &Value,
        timeout: Option<Duration>,
        spent: &mut u64,
    ) -> Result<(u16, Vec<u8>), String> {
        let mut req = self.http.post(&self.url);
        if let Some(t) = timeout {
            req = req.timeout(t);
        }
        let mut resp = req
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("POST {}: {e}", self.url))?;
        let status = resp.status().as_u16();
        if resp.status().is_redirection() {
            return Err(format!("POST {}: {REDIRECT} (status {status})", self.url));
        }
        bounded_body(&mut resp, spent).await.map(|b| (status, b))
    }

    async fn notify(&self, method: &str) -> Result<(), String> {
        let (status, _) = self
            .post(&json!({ "jsonrpc": "2.0", "method": method }))
            .await?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(format!("status {status}"))
        }
    }

    /// One request; returns its `result` or an error naming the failure.
    pub async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.rpc_within(method, params, None).await
    }

    /// One request bounded by `timeout` (the remaining budget of a
    /// measured operation) instead of the client's ordinary deadline.
    pub async fn rpc_within(
        &mut self,
        method: &str,
        params: Value,
        timeout: Option<Duration>,
    ) -> Result<Value, String> {
        let mut spent = 0;
        self.rpc_spending(method, params, timeout, &mut spent).await
    }

    /// [`Self::rpc_within`] charging the reply to an operation's shared
    /// byte budget `spent`.
    pub async fn rpc_spending(
        &mut self,
        method: &str,
        params: Value,
        timeout: Option<Duration>,
        spent: &mut u64,
    ) -> Result<Value, String> {
        if self.next_id >= self.last_id {
            return Err(format!("{method}: this client's id range is exhausted"));
        }
        self.next_id += 1;
        let id = self.next_id;
        let (status, bytes) = self
            .post_within(
                &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
                timeout,
                spent,
            )
            .await?;
        let reply: Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("{method}: status {status}, unparseable reply: {e}"))?;
        if reply["id"] != json!(id) {
            return Err(format!("{method}: reply id {} is not {id}", reply["id"]));
        }
        if let Some(err) = reply.get("error") {
            return Err(format!("{method}: JSON-RPC error {err}"));
        }
        reply
            .get("result")
            .cloned()
            .ok_or_else(|| format!("{method}: reply has no result"))
    }

    /// Call a tool and decode the JSON inside its text block.
    pub async fn call(&mut self, tool: &str, arguments: Value) -> Result<Value, String> {
        self.call_within(tool, arguments, None).await
    }

    /// [`McpClient::call`] bounded by `timeout`.
    pub async fn call_within(
        &mut self,
        tool: &str,
        arguments: Value,
        timeout: Option<Duration>,
    ) -> Result<Value, String> {
        let mut spent = 0;
        self.call_spending(tool, arguments, timeout, &mut spent)
            .await
    }

    /// [`Self::call_within`] charging the reply to an operation's shared
    /// byte budget `spent` (every page of one scan charges the same one).
    pub async fn call_spending(
        &mut self,
        tool: &str,
        arguments: Value,
        timeout: Option<Duration>,
        spent: &mut u64,
    ) -> Result<Value, String> {
        self.tool_calls += 1;
        let result = self
            .rpc_spending(
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
                timeout,
                spent,
            )
            .await?;
        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("{tool}: result has no text content"))?;
        if result["isError"] != json!(false) {
            return Err(format!("{tool}: tool error: {text}"));
        }
        serde_json::from_str(text).map_err(|e| format!("{tool}: text is not JSON: {e}"))
    }

    /// The `tools/call` requests this session has sent.
    pub fn tool_calls(&self) -> u32 {
        self.tool_calls
    }

    /// `swarmail_list_attachments`: the parts of one captured email, in
    /// order. No wait or search view of a mail in the protocol this client
    /// speaks carries them, so this is how a caller learns a part exists.
    pub async fn list_attachments(&mut self, id: &str) -> Result<Value, String> {
        self.call("swarmail_list_attachments", json!({ "id": id }))
            .await
    }

    /// `swarmail_get_attachment`: one stored part of a captured email by its
    /// index in the attachment list. A UTF-8 part is returned as text.
    pub async fn get_attachment(&mut self, id: &str, index: usize) -> Result<Value, String> {
        self.call(
            "swarmail_get_attachment",
            json!({ "id": id, "index": index }),
        )
        .await
    }

    /// `swarmail_wait_for_email`: the long-poll (subscribe before check,
    /// decision 0002). Returns the decoded result, timed out or not.
    pub async fn wait(
        &mut self,
        inbox: &str,
        subject: Option<&str>,
        count: usize,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        self.wait_to(inbox, subject, None, count, timeout_ms).await
    }

    /// As [`Self::wait`], with the tool's `to` recipient filter on the wire.
    pub async fn wait_to(
        &mut self,
        inbox: &str,
        subject: Option<&str>,
        to: Option<&str>,
        count: usize,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let mut args = json!({ "inbox": inbox, "count": count, "timeout_ms": timeout_ms });
        if let Some(s) = subject {
            args["subject"] = json!(s);
        }
        if let Some(t) = to {
            args["to"] = json!(t);
        }
        self.call("swarmail_wait_for_email", args).await
    }

    /// Wait until `count` matching mails exist; a timeout is an error.
    pub async fn wait_emails(
        &mut self,
        inbox: &str,
        subject: Option<&str>,
        count: usize,
    ) -> Result<Vec<Value>, String> {
        self.wait_emails_to(inbox, subject, None, count).await
    }

    /// As [`Self::wait_emails`], with the tool's `to` recipient filter sent
    /// on the wire (a substring match on the stored recipients).
    pub async fn wait_emails_to(
        &mut self,
        inbox: &str,
        subject: Option<&str>,
        to: Option<&str>,
        count: usize,
    ) -> Result<Vec<Value>, String> {
        let found = self.wait_to(inbox, subject, to, count, 30_000).await?;
        if found["timed_out"] == json!(true) {
            return Err(format!(
                "waited for {count} mail(s) matching {subject:?} to {to:?} in {inbox}; matched {}",
                found["matched"]
            ));
        }
        found["emails"]
            .as_array()
            .cloned()
            .ok_or_else(|| "wait result has no emails".to_string())
    }
}

/// Read `resp`'s body under the reply and operation bounds. A declared
/// length over either bound refuses before any byte is read; a streamed body
/// is accounted chunk by chunk, with checked arithmetic, before the chunk is
/// appended, so nothing past a bound is ever buffered.
async fn bounded_body(resp: &mut reqwest::Response, spent: &mut u64) -> Result<Vec<u8>, String> {
    let over = |reply: u64, spent: u64, more: u64| {
        reply > MAX_REPLY_BYTES
            || spent
                .checked_add(more)
                .is_none_or(|t| t > MAX_OPERATION_BYTES)
    };
    if let Some(n) = resp.content_length()
        && over(n, *spent, n)
    {
        return Err(format!("{OVERSIZED}: declared {n} bytes"));
    }
    let mut out: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("read body: {e}"))? {
        let len = chunk.len() as u64;
        let reply = (out.len() as u64).checked_add(len).unwrap_or(u64::MAX);
        if over(reply, *spent, len) {
            return Err(format!("{OVERSIZED}: past {reply} bytes"));
        }
        *spent += len;
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}
