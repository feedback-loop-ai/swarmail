//! A minimal client for Swarmail's MCP HTTP endpoint (`POST /mcp`) as it
//! ships today: protocol `2025-03-26`, tool results as JSON inside one
//! text block, no `structuredContent` (epic #164). Ids are correlated, a
//! JSON-RPC error or an `isError` result is an error — never data.
//!
//! Requests carry the URL's own loopback `Host` and no `Origin`, which is
//! exactly what decision 0043's guard admits.

use serde_json::{Value, json};
use std::time::Duration;

pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Above the server's 60 s wait ceiling, so a maximal wait is answered.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(75);

pub struct McpClient {
    http: reqwest::Client,
    url: String,
    next_id: u64,
}

impl McpClient {
    /// Initialize a session against `url` (which ends in `/mcp`).
    pub async fn connect(url: &str) -> Result<McpClient, String> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let mut client = McpClient {
            http,
            url: url.to_string(),
            next_id: 0,
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
        let resp = self
            .http
            .post(&self.url)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("POST {}: {e}", self.url))?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|e| format!("read body: {e}"))?;
        Ok((status, bytes.to_vec()))
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
        self.next_id += 1;
        let id = self.next_id;
        let (status, bytes) = self
            .post(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
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
        let result = self
            .rpc(
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
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

    /// `swarmail_wait_for_email`: the long-poll (subscribe before check,
    /// decision 0002). Returns the decoded result, timed out or not.
    pub async fn wait(
        &mut self,
        inbox: &str,
        subject: Option<&str>,
        count: usize,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let mut args = json!({ "inbox": inbox, "count": count, "timeout_ms": timeout_ms });
        if let Some(s) = subject {
            args["subject"] = json!(s);
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
        let found = self.wait(inbox, subject, count, 30_000).await?;
        if found["timed_out"] == json!(true) {
            return Err(format!(
                "waited for {count} mail(s) matching {subject:?} in {inbox}; matched {}",
                found["matched"]
            ));
        }
        found["emails"]
            .as_array()
            .cloned()
            .ok_or_else(|| "wait result has no emails".to_string())
    }
}
