//! Every MCP tool and every protocol error path, driven through POST /mcp.

mod common;

use common::{SmtpConn, http_json, smtp_send, start};

fn base64_of(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s)
}

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

/// Every tool's fallback closures (`unwrap_or_else` defaults) only run when
/// the optional argument is ABSENT — drive each tool bare, plus the one arm
/// no other test reaches (`swarmail_clear_all`).
#[tokio::test]
async fn bare_tool_calls_hit_every_default_and_clear_all() {
    let srv = start().await;

    // Seed two mails into the default inbox so the readers have something
    // to find, then one into a scratch inbox that clear_all retires.
    smtp_send(
        srv.smtp_addr,
        None,
        "bare@x.io",
        "to@x.io",
        "bare subject one",
        "https://one.io 111222",
    )
    .await
    .unwrap();
    smtp_send(
        srv.smtp_addr,
        None,
        "bare@x.io",
        "to@x.io",
        "bare subject two",
        "https://two.io 333444",
    )
    .await
    .unwrap();

    // search_emails with NO arguments: default inbox, default limit.
    let result = text(&tool(srv.http_addr, "swarmail_search_emails", "{}").await);
    assert_eq!(result["total"], 2, "{result}");
    assert_eq!(result["emails"].as_array().unwrap().len(), 2, "{result}");
    // …and with an explicit since_ms: the since filter closure runs.
    let since = text(
        &tool(
            srv.http_addr,
            "swarmail_search_emails",
            r#"{"since_ms": 0}"#,
        )
        .await,
    );
    assert_eq!(since["total"], 2, "{since}");

    // get_latest_email with NO arguments: default inbox, count 1.
    let result = text(&tool(srv.http_addr, "swarmail_get_latest_email", "{}").await);
    assert_eq!(
        result["emails"][0]["subject"], "bare subject two",
        "{result}"
    );

    // seed_email with only the required `to`: the from/subject/text defaults.
    let result = text(
        &tool(
            srv.http_addr,
            "swarmail_seed_email",
            r#"{"to": "bare@x.io"}"#,
        )
        .await,
    );
    assert!(result.is_object(), "{result}");

    // set_chaos with no delay_ms: the zero-delay default.
    let result = text(
        &tool(
            srv.http_addr,
            "swarmail_set_chaos",
            r#"{"event": "data", "probability": 0.0, "error": "nope"}"#,
        )
        .await,
    );
    assert!(result.is_object(), "{result}");
    // …and with an explicit delay_ms: the delay closure runs.
    let delayed = text(
        &tool(
            srv.http_addr,
            "swarmail_set_chaos",
            r#"{"event": "data", "probability": 0.0, "error": "nope", "delay_ms": 5}"#,
        )
        .await,
    );
    assert!(delayed.is_object(), "{delayed}");

    // wait_for_email with NO arguments: default inbox, count 1, default
    // timeout — the second bare mail is already there.
    let result = text(&tool(srv.http_addr, "swarmail_wait_for_email", "{}").await);
    assert!(result.is_array() || result.is_object(), "{result}");
    // …and with an explicit since_ms: the await filter closure runs.
    let result = text(
        &tool(
            srv.http_addr,
            "swarmail_wait_for_email",
            r#"{"since_ms": 0}"#,
        )
        .await,
    );
    assert!(result.is_array() || result.is_object(), "{result}");

    // clear_inbox with NO argument retires the default inbox.
    let result = text(&tool(srv.http_addr, "swarmail_clear_inbox", "{}").await);
    assert!(result["removed"].as_u64().unwrap() >= 1, "{result}");

    // clear_all: the whole-store arm.
    smtp_send(
        srv.smtp_addr,
        Some("scratchbox"),
        "s@x.io",
        "t@x.io",
        "scratch",
        "x",
    )
    .await
    .unwrap();
    let result = text(&tool(srv.http_addr, "swarmail_clear_all", "{}").await);
    assert!(result["removed"].as_u64().unwrap() >= 1, "{result}");
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/scratchbox/count",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 0, "clear_all retires every inbox");
}

/// Rich header shapes: display names ride both To and From, a From header
/// with a display name but no address falls back to the envelope sender,
/// and a single-token AUTH PLAIN names the inbox with no NULs at all.
#[tokio::test]
async fn header_names_and_single_token_plain_auth() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<env@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<named@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(
        c.data(
            "From: \"Fiona F\" <fiona@x.io>\r\nTo: \"Bob B\" <bob@x.io>\r\nSubject: named\r\n\r\nx"
        )
        .await
        .starts_with("250")
    );

    // A From header whose display name carries no address: the envelope
    // sender is the fallback (`unwrap_or_else(|| from_envelope.clone())`).
    let mut c2 = SmtpConn::connect(srv.smtp_addr).await;
    c2.send("EHLO t").await;
    c2.reply().await;
    c2.send("MAIL FROM:<env2@x.io>").await;
    assert!(c2.reply().await.starts_with("250"));
    c2.send("RCPT TO:<named@x.io>").await;
    assert!(c2.reply().await.starts_with("250"));
    assert!(
        c2.data("From: Fiona\r\nTo: bob@x.io\r\nSubject: noaddr\r\n\r\nx")
            .await
            .starts_with("250")
    );

    let (_, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    let emails = list["emails"].as_array().unwrap();
    let named = emails
        .iter()
        .find(|e| e["subject"] == "named")
        .expect("the named mail landed");
    assert_eq!(named["from"]["name"], "Fiona F", "{named}");
    assert_eq!(named["from"]["address"], "fiona@x.io", "{named}");
    assert_eq!(named["to"][0]["name"], "Bob B", "{named}");

    let noaddr = emails
        .iter()
        .find(|e| e["subject"] == "noaddr")
        .expect("the noaddr mail landed");
    assert_eq!(noaddr["from"]["name"], "Fiona", "{noaddr}");
    assert_eq!(noaddr["from"]["address"], "env2@x.io", "{noaddr}");

    // A single-token AUTH PLAIN (no NUL separators): the token itself names
    // the inbox (`parts.get(1).or_else(|| parts.first())`).
    let mut c3 = SmtpConn::connect(srv.smtp_addr).await;
    c3.send("EHLO t").await;
    c3.reply().await;
    c3.send("AUTH PLAIN").await;
    assert!(c3.reply().await.starts_with("334"));
    c3.send(&base64_of("solobox")).await;
    assert!(c3.reply().await.starts_with("235"));
    c3.send("MAIL FROM:<f@x.io>").await;
    assert!(c3.reply().await.starts_with("250"));
    c3.send("RCPT TO:<solo@x.io>").await;
    assert!(c3.reply().await.starts_with("250"));
    assert!(
        c3.data("Subject: solo auth\r\n\r\nx")
            .await
            .starts_with("250")
    );
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/solobox/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}
