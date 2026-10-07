//! P2 end-to-end: MCP JSON-RPC, seed API, webhook delivery.

mod common;

use common::{http_json, smtp_send, start_with};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::test]
async fn mcp_initialize_and_tools() {
    let srv = start_with(|c| c).await;
    let (status, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["result"]["serverInfo"]["name"], "swarmail");

    let (status, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#),
    )
    .await;
    assert_eq!(status, 200);
    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 12);
    assert!(tools.iter().any(|t| t["name"] == "swarmail_wait_for_email"));
}

#[tokio::test]
async fn mcp_seed_search_clear_loop() {
    let srv = start_with(|c| c).await;
    // The clear → act → assert loop, entirely through MCP.
    for (id, req) in [
        (
            1,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_clear_all","arguments":{}}}"#,
        ),
        (
            2,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"to":"agent@example.com","subject":"Verify","text":"Go to https://x.io/v?t=1. Code 998877."}}}"#,
        ),
        (
            3,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"swarmail_search_emails","arguments":{"inbox":"default","to":"agent@example.com"}}}"#,
        ),
    ] {
        let (status, body) = http_json(srv.http_addr, "POST", "/mcp", Some(req)).await;
        assert_eq!(status, 200, "call {id} failed: {body}");
        assert_eq!(body["result"]["isError"], false, "call {id}: {body}");
        if id == 3 {
            let emails = &body["result"]["content"][0]["text"];
            let parsed: serde_json::Value = serde_json::from_str(emails.as_str().unwrap()).unwrap();
            assert_eq!(parsed["total"], 1);
            let email = &parsed["emails"][0];
            assert_eq!(email["links"][0], "https://x.io/v?t=1");
            assert_eq!(email["codes"][0], "998877");
        }
    }
}

#[tokio::test]
async fn seed_endpoint_extracts_and_stores() {
    let srv = start_with(|c| c).await;
    let (status, body) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/fixtures/seed",
        Some(r#"{"from":"noreply@x.io","to":"u@example.com","subject":"Reset your password","text":"Use 554433 to reset. https://x.io/reset?a=b"}"#),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["codes"][0], "554433");
    assert_eq!(body["links"][0], "https://x.io/reset?a=b");

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/fixtures/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 1);
}

#[tokio::test]
async fn webhooks_fire_with_secret_and_retry_targets() {
    let srv = start_with(|c| c).await;

    // A minimal HTTP receiver.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let receiver = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.unwrap();
        String::from_utf8_lossy(&buf[..n]).to_string()
    });

    let (status, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(&format!(
            r#"[{{"url": "http://{addr}/hooks", "inbox": "events", "secret": "topsecret"}}]"#
        )),
    )
    .await;
    assert_eq!(status, 200);

    // Deliver into the matching inbox.
    smtp_send(
        srv.smtp_addr,
        Some("events"),
        "n@x.io",
        "a@x.io",
        "hooked",
        "b",
    )
    .await
    .unwrap();
    // And an inbox that must NOT fire.
    smtp_send(srv.smtp_addr, None, "n@x.io", "a@x.io", "ignored", "b")
        .await
        .unwrap();

    let request = tokio::time::timeout(std::time::Duration::from_secs(5), receiver)
        .await
        .expect("webhook never fired")
        .unwrap();
    assert!(request.starts_with("POST /hooks"));
    assert!(request.contains("X-Swarmail-Secret: topsecret"));
    assert!(request.contains(r#""event":"received""#));
    assert!(request.contains("hooked"));
    assert!(!request.contains("ignored"), "inbox filter must hold");
}
