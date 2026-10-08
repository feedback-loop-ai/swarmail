//! Every MCP tool and every protocol error path, driven through POST /mcp.

mod common;

use common::{http_json, smtp_send, start};

type Http = std::net::SocketAddr;

async fn rpc(addr: Http, method: &str, params: &str) -> (u16, serde_json::Value) {
    let body = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{params}}}"#);
    http_json(addr, "POST", "/mcp", Some(&body)).await
}

async fn tool(addr: Http, name: &str, args: &str) -> serde_json::Value {
    let params = format!(r#"{{"name":"{name}","arguments":{args}}}"#);
    let (st, body) = rpc(addr, "tools/call", &params).await;
    assert_eq!(st, 200, "{name}");
    body["result"].clone()
}

fn text(result: &serde_json::Value) -> serde_json::Value {
    let raw = result["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text content in {result}"));
    serde_json::from_str(raw).expect("text is json")
}

#[tokio::test]
async fn initialize_and_tools_list() {
    let srv = start().await;
    let (_, body) = rpc(srv.http_addr, "initialize", "{}").await;
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(body["result"]["serverInfo"]["name"], "swarmail");

    let (_, body) = rpc(srv.http_addr, "tools/list", "{}").await;
    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 12);
}

#[tokio::test]
async fn search_get_latest_delete_clear() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("mcpbox"),
        "noreply@x.io",
        "a@x.io",
        "hello",
        "https://one.io 111222",
    )
    .await
    .unwrap();

    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "mcpbox"}"#,
    )
    .await;
    assert_eq!(text(&r)["emails"].as_array().unwrap().len(), 1);

    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "mcpbox", "from": "nobody@nowhere.io"}"#,
    )
    .await;
    assert_eq!(text(&r)["emails"].as_array().unwrap().len(), 0);

    let r = tool(
        srv.http_addr,
        "swarmail_get_latest_email",
        r#"{"inbox": "mcpbox"}"#,
    )
    .await;
    let id = text(&r)["emails"][0]["id"].as_str().unwrap().to_string();

    let r = tool(
        srv.http_addr,
        "swarmail_get_email",
        &format!(r#"{{"id": "{id}"}}"#),
    )
    .await;
    assert_eq!(text(&r)["subject"], "hello");

    // Missing id → an error result naming the id.
    let r = tool(srv.http_addr, "swarmail_get_email", r#"{"id": "nope"}"#).await;
    assert_eq!(r["isError"], true);
    assert!(r["content"][0]["text"].as_str().unwrap().contains("nope"));

    let r = tool(
        srv.http_addr,
        "swarmail_delete_email",
        &format!(r#"{{"id": "{id}"}}"#),
    )
    .await;
    assert_eq!(text(&r)["deleted"], true);
    let r = tool(srv.http_addr, "swarmail_delete_email", r#"{"id": "gone"}"#).await;
    assert_eq!(r["isError"], true);

    let r = tool(
        srv.http_addr,
        "swarmail_clear_inbox",
        r#"{"inbox": "mcpbox"}"#,
    )
    .await;
    assert_eq!(text(&r)["removed"], 0);
}

#[tokio::test]
async fn get_latest_on_empty_inbox_is_an_error_result() {
    let srv = start().await;
    let r = tool(
        srv.http_addr,
        "swarmail_get_latest_email",
        r#"{"inbox": "empty"}"#,
    )
    .await;
    assert_eq!(r["isError"], true);
}

#[tokio::test]
async fn wait_for_email_found_and_timed_out() {
    let srv = start().await;
    // Found: the mail already exists.
    smtp_send(
        srv.smtp_addr,
        Some("waitbox"),
        "noreply@x.io",
        "w@x.io",
        "wait",
        "body",
    )
    .await
    .unwrap();
    let r = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "waitbox", "count": 1, "timeout_ms": 1000}"#,
    )
    .await;
    assert_eq!(text(&r)["matched"], 1);

    // Timeout: the deadline elapses first.
    let r = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "waitbox", "count": 9, "timeout_ms": 150}"#,
    )
    .await;
    assert_eq!(text(&r)["timed_out"], true);
    assert_eq!(text(&r)["matched"], 1);
}

#[tokio::test]
async fn extract_links_and_codes() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("extractbox"),
        "noreply@x.io",
        "e@x.io",
        "reset",
        "Go https://x.io/reset?t=1 and enter 998877.",
    )
    .await
    .unwrap();
    let id = {
        let r = tool(
            srv.http_addr,
            "swarmail_get_latest_email",
            r#"{"inbox": "extractbox"}"#,
        )
        .await;
        text(&r)["emails"][0]["id"].as_str().unwrap().to_string()
    };

    let r = tool(
        srv.http_addr,
        "swarmail_extract_links",
        &format!(r#"{{"id": "{id}"}}"#),
    )
    .await;
    assert_eq!(text(&r)["links"][0], "https://x.io/reset?t=1");

    let r = tool(
        srv.http_addr,
        "swarmail_extract_codes",
        &format!(r#"{{"id": "{id}"}}"#),
    )
    .await;
    assert_eq!(text(&r)["codes"][0], "998877");

    let r = tool(
        srv.http_addr,
        "swarmail_extract_links",
        r#"{"id": "missing"}"#,
    )
    .await;
    assert_eq!(r["isError"], true);
}

#[tokio::test]
async fn seed_email_and_set_clear_chaos() {
    let srv = start().await;
    let r = tool(
        srv.http_addr,
        "swarmail_seed_email",
        r#"{"inbox": "seedbox", "to": "s@x.io", "subject": "seeded", "text": "https://s.io 445566"}"#,
    )
    .await;
    assert_eq!(text(&r)["codes"][0], "445566");

    for event in ["connect", "mail_from", "rcpt", "data"] {
        let r = tool(
            srv.http_addr,
            "swarmail_set_chaos",
            &format!(r#"{{"event": "{event}", "probability": 100, "error": "451 {event}"}}"#),
        )
        .await;
        assert_eq!(text(&r)[event]["error"], format!("451 {event}"));
    }

    // An unknown event is an error result.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_set_chaos", "arguments": {"event": "dns"}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown event")
    );

    let r = tool(srv.http_addr, "swarmail_clear_chaos", "{}").await;
    assert_eq!(text(&r)["chaos"], "disabled");
}

#[tokio::test]
async fn unknown_tool_and_missing_args_are_errors() {
    let srv = start().await;
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_nope", "arguments": {}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["error"]["message"], "unknown tool: swarmail_nope");

    // get_email without an id → error result, not a panic.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_get_email", "arguments": {}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("missing id")
    );
}
