//! Every MCP tool and every protocol error path, driven through POST /mcp.

mod common;

use common::{
    SmtpConn, http_get_text, http_json, http_send_text, metric_row_value, smtp_send, start,
    start_with, wait_for_metric_row,
};

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

    // The JSON-RPC ping answers with an empty result object.
    let (_, body) = rpc(srv.http_addr, "ping", "{}").await;
    assert_eq!(body["result"], serde_json::json!({}), "{body}");

    let (_, body) = rpc(srv.http_addr, "tools/list", "{}").await;
    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 12);

    // inputSchema is a JSON *object* for all twelve tools — the JSON type,
    // not a substring (a substring check is how the string-schema bug
    // survived). Decision 0010, issue #4.
    let by_name: std::collections::HashMap<&str, &serde_json::Value> = tools
        .iter()
        .map(|t| (t["name"].as_str().unwrap(), t))
        .collect();
    for (name, tool) in &by_name {
        let schema = &tool["inputSchema"];
        assert!(
            schema.is_object(),
            "{name} publishes a non-object inputSchema: {schema}"
        );
    }

    // The five repaired tools carry the verbatim re-kind of the old string
    // constants: same fields, types, descriptions and `required` arrays.
    for name in [
        "swarmail_get_email",
        "swarmail_delete_email",
        "swarmail_extract_links",
        "swarmail_extract_codes",
    ] {
        let schema = &by_name[name]["inputSchema"];
        assert_eq!(
            schema["properties"]["id"],
            serde_json::json!({ "type": "string", "description": "Email id" }),
            "{name}"
        );
        assert_eq!(schema["required"], serde_json::json!(["id"]), "{name}");
    }
    let inbox_only = &by_name["swarmail_clear_inbox"]["inputSchema"];
    assert_eq!(
        inbox_only["properties"]["inbox"],
        serde_json::json!({ "type": "string", "description": "Inbox name (default: default)" })
    );
    assert!(
        inbox_only["required"].is_null(),
        "the inbox-only schema has no required array"
    );
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

/// The advertised `html` finally reaches the extractor at ingest (decision
/// 0010): the shared constructor preserves it verbatim under
/// `Content-Type: text/html` — the same raw shape the REST seed writes — and
/// the response carries the extracted codes.
#[tokio::test]
async fn seed_with_html_stores_html_and_extracts_at_ingest() {
    let srv = start().await;
    let html_arg = "<a href='https://q.io?a=1&b=2'>go</a> 555555";
    let r = tool(
        srv.http_addr,
        "swarmail_seed_email",
        &format!(
            r#"{{"inbox": "mcphtml", "to": "h@x.io", "subject": "hi", "html": "{html_arg}"}}"#
        ),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(text(&r)["codes"][0], "555555", "extraction ran: {r}");
    let id = text(&r)["id"].as_str().unwrap().to_string();

    let (st, email) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert!(
        email["html"].as_str().unwrap().contains(html_arg),
        "html preserved verbatim: {}",
        email["html"]
    );

    // The raw carries the html body verbatim — the shape the REST seed writes.
    let (st, raw) = http_get_text(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(st, 200);
    assert!(
        raw.contains(&format!(
            "MIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n{html_arg}\r\n"
        )),
        "raw: {raw}"
    );
}

/// html supersedes text on the MCP surface too (decision 0010): both bodies
/// given, the html lands verbatim and the plain body never reaches the parser
/// — the same one-constructor precedence the REST seed pins, proven through
/// the tool whose schema advertises both arguments.
#[tokio::test]
async fn seed_with_both_bodies_stores_html_and_drops_text_over_mcp() {
    let srv = start().await;
    let html_arg = "<a href='https://m.io'>go</a> 334455";
    let r = tool(
        srv.http_addr,
        "swarmail_seed_email",
        &format!(
            r#"{{"inbox": "mcpboth", "to": "w@x.io", "text": "PLAIN https://dropped.io 998877", "html": "{html_arg}"}}"#
        ),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    // Extraction ran on the html alone: the dropped text's link and code are
    // absent, and the html body's own code was found.
    assert_eq!(
        text(&r)["links"],
        serde_json::json!(["https://m.io"]),
        "the dropped text must not feed the extractor: {r}"
    );
    assert_eq!(text(&r)["codes"], serde_json::json!(["334455"]), "{r}");
    let id = text(&r)["id"].as_str().unwrap().to_string();

    let (st, email) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert!(
        email["html"].as_str().unwrap().contains(html_arg),
        "html preserved verbatim: {}",
        email["html"]
    );
    assert_ne!(
        email["text"].as_str().unwrap(),
        "PLAIN https://dropped.io 998877",
        "the plain body must not be the stored text: {}",
        email["text"]
    );

    // The raw carries the html body verbatim and never the dropped text.
    let (st, raw) = http_get_text(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(st, 200);
    assert!(
        raw.contains(&format!(
            "MIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n{html_arg}\r\n"
        )),
        "raw: {raw}"
    );
    assert!(
        !raw.contains("PLAIN") && !raw.contains("998877"),
        "the dropped text must not land anywhere: {raw}"
    );
}

/// The two adapters build the identical fixture (decision 0010): the same
/// seed through REST and MCP agrees on every content field. `id` and
/// `received_ms` are excluded by construction — clock and UUIDv7 draws, and
/// a parity test that touches them is a flake factory.
#[tokio::test]
async fn the_same_fixture_agrees_across_rest_and_mcp() {
    let srv = start().await;
    // One html fixture (the shape MCP never had) and one text-only fixture.
    let fixtures = [
        (
            "parityhtml",
            r#"{"to": "p@x.io", "subject": "parity", "html": "<a href='https://p.io'>go</a> 778899"}"#,
        ),
        (
            "paritytext",
            r#"{"to": "t@x.io", "subject": "plain", "text": "Go https://t.io 112233"}"#,
        ),
    ];
    for (inbox, rest_body) in fixtures {
        let (st, rest) = http_json(
            srv.http_addr,
            "POST",
            &format!("/api/v1/inboxes/{inbox}-rest/seed"),
            Some(rest_body),
        )
        .await;
        assert_eq!(st, 200);

        // The same fixture over MCP: the arguments body with an inbox name in
        // front of it.
        let mcp_args = format!(r#"{{"inbox": "{inbox}-mcp", {}"#, &rest_body[1..]);
        let r = tool(srv.http_addr, "swarmail_seed_email", &mcp_args).await;
        let id = text(&r)["id"].as_str().unwrap().to_string();

        // Read the MCP-seeded mail back through REST (the same store) and
        // compare the content fields only.
        let (st, mcp) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None,
        )
        .await;
        assert_eq!(st, 200);
        for field in ["text", "html", "subject", "links", "codes"] {
            assert_eq!(rest[field], mcp[field], "{field} disagrees on {inbox}");
        }
    }
}

/// A fixture whose envelope field smuggles CR/LF/NUL is refused before any
/// insert on BOTH surfaces (decision 0010): REST answers 400 naming the
/// field, MCP answers isError with the same reason, and the inbox count is
/// unchanged — the refusal composes with the exact-count assert.
#[tokio::test]
async fn hostile_envelope_fields_are_refused_on_both_surfaces() {
    let srv = start().await;
    // CR+LF smuggles a header line, NUL is the classic poison byte, a bare
    // LF is the lenient-client shape — one of the three per field. Each
    // fixture runs through both surfaces verbatim.
    for (field, rest_body, mcp_args) in [
        (
            "to",
            r#"{"to": "a@b.io\r\nBcc: smuggled@x.io"}"#,
            r#"{"inbox": "mcprefused", "to": "a@b.io\r\nBcc: smuggled@x.io"}"#,
        ),
        (
            "from",
            r#"{"to": "ok@x.io", "from": "nul@\u0000x.io"}"#,
            r#"{"inbox": "mcprefused", "to": "ok@x.io", "from": "nul@\u0000x.io"}"#,
        ),
        (
            "subject",
            r#"{"to": "ok@x.io", "subject": "line\nbreak"}"#,
            r#"{"inbox": "mcprefused", "to": "ok@x.io", "subject": "line\nbreak"}"#,
        ),
    ] {
        let (st, body) = http_json(
            srv.http_addr,
            "POST",
            "/api/v1/inboxes/restrefused/seed",
            Some(rest_body),
        )
        .await;
        assert_eq!(st, 400, "{field}: {body}");
        assert!(
            body["error"].as_str().unwrap().contains(field),
            "{field} must be named: {body}"
        );

        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_seed_email", "arguments": {mcp_args}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{field}");
        let r = body["result"].clone();
        assert_eq!(r["isError"], true, "{field}: {r}");
        assert!(
            r["content"][0]["text"].as_str().unwrap().contains(field),
            "{field} must be named: {r}"
        );
    }

    // Nothing was stored on either surface.
    for inbox in ["restrefused", "mcprefused"] {
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{inbox}/count"),
            None,
        )
        .await;
        assert_eq!(st, 200);
        assert_eq!(body["count"], 0, "nothing stored in {inbox}");
    }
}

/// The second min-one clamp (decision 0010): `count: 0` on the MCP wait
/// still means at least one — a timeout against an empty inbox, never an
/// instant success — and `matched: 1` once a matching mail has landed.
#[tokio::test]
async fn wait_for_email_keeps_minimum_one_for_zero() {
    let srv = start().await;
    let r = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "waitzero", "count": 0, "timeout_ms": 150}"#,
    )
    .await;
    assert_eq!(text(&r)["timed_out"], true, "{r}");
    assert_eq!(text(&r)["matched"], 0);

    smtp_send(
        srv.smtp_addr,
        Some("waitzero"),
        "noreply@x.io",
        "w@x.io",
        "w",
        "body",
    )
    .await
    .unwrap();
    let r = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "waitzero", "count": 0, "timeout_ms": 1000}"#,
    )
    .await;
    assert_eq!(text(&r)["matched"], 1, "{r}");
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

    // Schema/handler parity (decision 0010): every `required` argument of the
    // tools this wave owns is actually enforced by its handler — the drift
    // class that let seed_email advertise `html` while its handler dropped it.
    for name in [
        "swarmail_delete_email",
        "swarmail_extract_links",
        "swarmail_extract_codes",
    ] {
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "{name}", "arguments": {{}}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{name}");
        assert_eq!(body["error"]["message"], "missing id", "{name}");
    }

    // seed_email without its required `to`.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_seed_email", "arguments": {"inbox": "x"}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["error"]["message"], "missing to");

    // The declared requirement is enforced (decision 0018): a bare
    // set_chaos refuses by name — the argument is never silently defaulted
    // to a probability the operator never chose.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_set_chaos", "arguments": {"event": "data"}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        body["error"]["message"], "probability must be an integer between 0 and 100",
        "{body}"
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

    // set_chaos with no delay_ms: the zero-delay default. The probability is
    // a JSON integer — a float (`0.0`) is a wrong-typed wire value and is
    // refused by name (decision 0018: no narrowing casts).
    let result = text(
        &tool(
            srv.http_addr,
            "swarmail_set_chaos",
            r#"{"event": "data", "probability": 0, "error": "451 nope"}"#,
        )
        .await,
    );
    assert!(result.is_object(), "{result}");
    // …and with an explicit delay_ms: the delay closure runs.
    let delayed = text(
        &tool(
            srv.http_addr,
            "swarmail_set_chaos",
            r#"{"event": "data", "probability": 0, "error": "451 nope", "delay_ms": 5}"#,
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

/// Contract #1's MCP seed leg (decision 0012 §3, amended by 0014): with a
/// second real connection holding the writer lock, the seed tool's commit
/// fails and the tool answers an error result naming the cause — never an
/// `ok` result carrying an id that was never durable. Zero trace: the
/// refused seed is nowhere while live and the restart copy stays empty —
/// memory and data file agree, so the same call can simply retry.
#[tokio::test]
async fn a_blocked_commit_refuses_the_mcp_seed_with_an_error_result() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-e2e-refuse-mcp-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;

    let _lock = common::hold_write_lock(&db);
    let r = tool(
        srv.http_addr,
        "swarmail_seed_email",
        r#"{"inbox": "blocked", "to": "s@x.io", "subject": "refused seed", "text": "hello"}"#,
    )
    .await;
    assert_eq!(r["isError"], true, "the blocked commit must refuse: {r}");
    let msg = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        msg.contains("storage commit failed"),
        "the error result must name the cause: {msg}"
    );

    // Zero trace (decision 0014): the refused seed is nowhere while live —
    // the tool claimed a failure, so the memory must agree with the data
    // file it failed to write.
    let (st, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/blocked/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 0,
        "the refused seed must be nowhere while live: {list}"
    );

    // And it is not durable: the restart copy has no row for it.
    srv.stop().await;
    drop(_lock);
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let (st, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/blocked/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 0,
        "the refused seed must not reach the restart copy"
    );
    srv.stop().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

/// Finding 3's MCP leg (issue #31, decision 0014): with a real rejecting
/// BEFORE DELETE trigger on the data file, every destructive tool answers
/// an error result naming the cause — never an `ok` result for work that
/// did not commit — and the mail survives the restart untouched. No fault
/// shim: the refusal is the database's own.
#[tokio::test]
async fn a_blocked_delete_refuses_the_mcp_delete_tools_with_error_results() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-e2e-refuse-mcp-del-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;

    let r = tool(
        srv.http_addr,
        "swarmail_seed_email",
        r#"{"inbox": "del", "to": "s@x.io", "subject": "doomed", "text": "hello"}"#,
    )
    .await;
    let id = text(&r)["id"].as_str().unwrap().to_string();

    let trigger = common::refusing_trigger(&db, "delete");

    // Every destructive tool refuses with the fixed cause, never an `ok`.
    for (name, args, what) in [
        (
            "swarmail_delete_email",
            format!(r#"{{"id": "{id}"}}"#),
            "delete by id",
        ),
        (
            "swarmail_clear_inbox",
            r#"{"inbox": "del"}"#.to_string(),
            "clear",
        ),
        ("swarmail_clear_all", "{}".to_string(), "clear all"),
    ] {
        let r = tool(srv.http_addr, name, &args).await;
        assert_eq!(r["isError"], true, "{what} must refuse: {r}");
        let msg = r["content"][0]["text"].as_str().unwrap_or("");
        assert!(
            msg.contains("storage commit failed"),
            "{what} must name the cause: {msg}"
        );
    }

    // The commit is the truth: the mail is still there, live.
    let (st, list) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/del/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 1,
        "a refused delete must preserve the mail: {list}"
    );

    // And it survives the restart: nothing half-applied, nothing lost.
    srv.stop().await;
    drop(trigger);
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let (st, list) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/del/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 1,
        "a refused delete must not lose the mail across a restart: {list}"
    );
    assert_eq!(list["emails"][0]["subject"], "doomed");
    srv.stop().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

// ---------- explicit bounds and observed totals (decision 0018) ----------

/// The wait cap is an `isError` refusal naming the maximum — the same text
/// REST refuses with — and it answers without parking (decision 0018: the
/// bound is refused before any subscribe, never clamped to it).
#[tokio::test]
async fn wait_over_the_cap_is_refused_without_parking() {
    let srv = start().await;
    let started = std::time::Instant::now();
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_wait_for_email", "arguments": {"inbox": "cap", "count": 1, "timeout_ms": 60001}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(
        body["result"]["content"][0]["text"],
        "timeout_ms must be at most 60000"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "the cap refusal must not park: {:?}",
        started.elapsed()
    );
}

/// The wait results carry a `total` counted after the wait, in both arms —
/// the observed state, not a pre-wait snapshot (decision 0018).
#[tokio::test]
async fn wait_results_carry_the_post_wait_total_in_both_arms() {
    let srv = start().await;
    let addr = srv.http_addr;

    // Found arm: the mail arrives mid-wait; total counts it.
    let request = tokio::spawn(tool(
        addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "observed", "to": "racer@x.io", "count": 1, "timeout_ms": 10000}"#,
    ));
    // The wait parks, observed under its own inbox — the mail arrives
    // strictly after the wait registered, so the post-wait total counts it
    // deterministically (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"observed\"}", 1).await;
    smtp_send(
        srv.smtp_addr,
        Some("observed"),
        "f@x.io",
        "racer@x.io",
        "midwait",
        "body",
    )
    .await
    .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), request)
        .await
        .expect("the wait must resolve on the delivery")
        .unwrap();
    assert_eq!(text(&result)["matched"], 1);
    assert_eq!(text(&result)["total"], 1, "{result}");

    // Timeout arm: a non-matching mail lands mid-wait; total counts it.
    let result = text(
        &tool(
            srv.http_addr,
            "swarmail_wait_for_email",
            r#"{"inbox": "observed408", "to": "nobody@x.io", "count": 9, "timeout_ms": 1500}"#,
        )
        .await,
    );
    assert_eq!(result["timed_out"], true, "{result}");
    assert_eq!(result["total"], 0);
    smtp_send(
        srv.smtp_addr,
        Some("observed408"),
        "f@x.io",
        "someone-else@x.io",
        "unrelated",
        "body",
    )
    .await
    .unwrap();
    let result = text(
        &tool(
            srv.http_addr,
            "swarmail_wait_for_email",
            r#"{"inbox": "observed408", "to": "nobody@x.io", "count": 9, "timeout_ms": 1500}"#,
        )
        .await,
    );
    assert_eq!(result["timed_out"], true, "{result}");
    assert_eq!(result["matched"], 0);
    assert_eq!(
        result["total"], 1,
        "total is counted after the wait: {result}"
    );
}

/// Every validation stage of `swarmail_set_chaos` refuses by name, and a
/// legal value still sets the rule (decision 0018: presence, type, `u8`
/// range and the 100 ceiling are distinct refusals; no cast, no clamp, no
/// silent default on any wire value).
#[tokio::test]
async fn set_chaos_refuses_every_malformed_wire_value_by_name() {
    let srv = start().await;

    // 256 does not become 0: it does not fit u8.
    for (args, label) in [
        (
            r#"{"event": "data", "probability": 256}"#,
            "256 must refuse, not narrow to 0",
        ),
        (
            r#"{"event": "data", "probability": "high"}"#,
            "a string is a wrong-typed wire value",
        ),
        (
            r#"{"event": "data", "probability": 0.0}"#,
            "a float would be a narrowing cast",
        ),
        (
            r#"{"event": "data", "probability": 150}"#,
            "150 is over the 100 ceiling",
        ),
        (
            r#"{"event": "data"}"#,
            "an absent probability is refused, not defaulted",
        ),
        (
            r#"{"event": "data", "probability": null}"#,
            "a supplied null is supplied, not omission",
        ),
        (
            r#"{"event": "data", "probability": 101}"#,
            "101 is over the ceiling, refused by the shared validator",
        ),
        (
            r#"{"event": "data", "probability": 255}"#,
            "255 is the u8 ceiling, still over the domain's",
        ),
    ] {
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_set_chaos", "arguments": {args}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{label}: {body}");
        assert_eq!(
            body["error"]["message"], "probability must be an integer between 0 and 100",
            "{label}: {body}"
        );
    }

    // Wrong-typed delay_ms refuses by name; absent stays the honest zero.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_set_chaos", "arguments": {"event": "data", "probability": 10, "delay_ms": "soon"}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        body["error"]["message"], "delay_ms must be a non-negative integer",
        "{body}"
    );

    // A legal value sets the rule — the pole case is already pinned by the
    // chaos tests; this is the interior.
    let r = tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event": "data", "probability": 50, "error": "451 busy", "delay_ms": 5}"#,
    )
    .await;
    assert_eq!(text(&r)["data"]["probability"], 50, "{r}");
    assert_eq!(text(&r)["data"]["delay_ms"], 5);
    assert_eq!(text(&r)["data"]["error"], "451 busy");
}

/// The wire-text rules refuse on the MCP plane with the shared sentences
/// (issue #41, decision 0029 §3): a 2xx `error` is not a failure, CR/LF
/// would write two replies for one command, and `delay_ms` has the
/// stage-timer scale. The sentences are the REST plane's own words — one
/// domain validator, one refusal text. A legal 510-octet line and the
/// delay ceiling itself are the interior the refusals protect.
#[tokio::test]
async fn set_chaos_refuses_the_wire_text_rules_by_name() {
    let srv = start().await;
    for (args, sentence) in [
        (
            r#"{"event": "data", "probability": 100, "error": "250 2.0.0 OK"}"#,
            "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets",
        ),
        (
            r#"{"event": "data", "probability": 100, "error": "551 go\r\naway"}"#,
            "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets",
        ),
        (
            r#"{"event": "data", "probability": 100, "error": "go away"}"#,
            "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets",
        ),
        (
            r#"{"event": "data", "probability": 100, "delay_ms": 300001}"#,
            "delay_ms must be an integer between 0 and 300000",
        ),
    ] {
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_set_chaos", "arguments": {args}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{args}: {body}");
        assert_eq!(body["error"]["message"], sentence, "{args}: {body}");
    }

    // The interior is legal: a 510-octet single-line 5xx reply (the cap is
    // inclusive) and the delay ceiling itself both store.
    let line = format!("551 {}", "x".repeat(506));
    let r = tool(
        srv.http_addr,
        "swarmail_set_chaos",
        &format!(
            r#"{{"event": "data", "probability": 100, "error": {line:?}, "delay_ms": 300000}}"#
        ),
    )
    .await;
    assert_eq!(text(&r)["data"]["delay_ms"], 300000, "{r}");
    assert_eq!(text(&r)["data"]["error"], line, "{r}");
}

/// A refused `set_chaos` never stores its rule (decision 0025): the set
/// tail is the read-back — after the refused data rule, a legal rcpt set
/// returns the current config with rcpt armed and data still null.
#[tokio::test]
async fn a_refused_set_chaos_never_stores_its_rule() {
    let srv = start().await;
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_set_chaos", "arguments": {"event": "data", "probability": 150}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        body["error"]["message"], "probability must be an integer between 0 and 100",
        "{body}"
    );

    let r = tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event": "rcpt", "probability": 50}"#,
    )
    .await;
    let current = text(&r);
    assert_eq!(current["rcpt"]["probability"], 50, "{current}");
    assert!(
        current["data"].is_null(),
        "the refused rule was never stored: {current}"
    );
}

/// A supplied malformed `timeout_ms` refuses by name and registers no
/// waiter (decision 0025): against an inbox that already holds a matching
/// message, the refusal answers immediately — never the Found shape a
/// silent 5000ms default would give — and the seeded mail is untouched.
#[tokio::test]
async fn a_supplied_malformed_timeout_refuses_and_registers_no_waiter() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("malformed-wait"),
        "f@x.io",
        "seeded@x.io",
        "seeded",
        "body",
    )
    .await
    .unwrap();

    for (value, label) in [
        (r#""invalid""#, "a string is not an integer"),
        ("-1", "a negative is over no bound — it is unparseable"),
        ("1.5", "a float would be a narrowing cast"),
        ("null", "null is supplied: the key is present"),
    ] {
        let started = std::time::Instant::now();
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(
                r#"{{"name": "swarmail_wait_for_email", "arguments": {{"inbox": "malformed-wait", "to": "seeded@x.io", "timeout_ms": {value}}}}}"#
            ),
        )
        .await;
        assert_eq!(st, 200, "{label}: {body}");
        assert_eq!(body["result"]["isError"], true, "{label}: {body}");
        assert_eq!(
            body["result"]["content"][0]["text"],
            "timeout_ms must be an integer between 0 and 60000",
            "{label}: {body}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "{label}: the refusal must answer immediately, not after a wait: {:?}",
            started.elapsed()
        );
    }

    // The seeded mail is untouched: still listable, never consumed.
    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "malformed-wait"}"#,
    )
    .await;
    assert_eq!(text(&r)["total"], 1, "{r}");
}

/// The `timeout_ms: 0` pole is legal on the domain layer: the wait answers
/// its own real zero deadline at once — `timed_out`, never a refusal and
/// never the 5000ms default (decision 0025).
#[tokio::test]
async fn wait_zero_timeout_is_a_real_zero_deadline() {
    let srv = start().await;
    let started = std::time::Instant::now();
    let result = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "zero-wait", "to": "nobody@x.io", "timeout_ms": 0}"#,
    )
    .await;
    assert_eq!(text(&result)["timed_out"], true, "{result}");
    assert_eq!(text(&result)["matched"], 0, "{result}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "timeout_ms=0 must answer at once: {:?}",
        started.elapsed()
    );
}

/// The published input schemas carry the bounds, so compliant clients refuse
/// at the tool layer (decision 0018).
#[tokio::test]
async fn the_schemas_publish_the_wire_bounds() {
    let srv = start().await;
    let (_, body) = rpc(srv.http_addr, "tools/list", "{}").await;
    let tools = body["result"]["tools"].as_array().unwrap();
    let by_name: std::collections::HashMap<&str, &serde_json::Value> = tools
        .iter()
        .map(|t| (t["name"].as_str().unwrap(), t))
        .collect();

    let wait = &by_name["swarmail_wait_for_email"]["inputSchema"]["properties"];
    assert_eq!(wait["timeout_ms"]["maximum"], 60000, "{wait}");
    assert_eq!(wait["timeout_ms"]["minimum"], 0, "{wait}");
    assert_eq!(
        wait["timeout_ms"]["default"], 5000,
        "the schema publishes the one documented default (decision 0025): {wait}"
    );
    assert!(
        wait["since_ms"].is_object(),
        "the since_ms filter is advertised: {wait}"
    );

    let chaos = &by_name["swarmail_set_chaos"]["inputSchema"]["properties"];
    assert_eq!(chaos["probability"]["minimum"], 0, "{chaos}");
    assert_eq!(chaos["probability"]["maximum"], 100, "{chaos}");
    assert_eq!(chaos["delay_ms"]["minimum"], 0, "{chaos}");

    // The staged domains are published where they are enforced (decision
    // 0029 §1): the search's limit takes the shared default and its
    // non-negative domain; get_latest's count refuses zero by name; the
    // wait's count documents 0010's at-least-one semantic; the chaos rule's
    // delay ceiling and reply-line grammar are on the wire.
    let search = &by_name["swarmail_search_emails"]["inputSchema"]["properties"];
    assert_eq!(search["limit"]["minimum"], 0, "{search}");
    assert_eq!(search["limit"]["default"], 100, "{search}");
    let latest = &by_name["swarmail_get_latest_email"]["inputSchema"]["properties"];
    assert_eq!(latest["count"]["minimum"], 1, "{latest}");
    let wait_count = &wait["count"];
    assert_eq!(wait_count["minimum"], 0, "{wait_count}");
    assert!(
        wait_count["description"]
            .as_str()
            .unwrap()
            .contains("at least one"),
        "{wait_count}"
    );
    assert_eq!(chaos["delay_ms"]["maximum"], 300000, "{chaos}");
    assert!(
        chaos["error"]["description"]
            .as_str()
            .unwrap()
            .contains("510"),
        "{chaos}"
    );
}

// ---------- the validation family: #46 (arguments + envelope) ----------

/// A *present* argument of the wrong JSON type — or explicit `null` —
/// refuses -32602 with a sentence naming the field and its domain, on
/// every tool that reads it (decision 0029 §1), and the effect never
/// happens.
#[tokio::test]
async fn a_wrong_typed_argument_refuses_by_name() {
    let srv = start().await;
    let legs: &[(&str, &str, &str)] = &[
        // (tool, arguments, field named in the refusal) — one leg per wrong
        // shape: string→int, int→string, array→string, explicit null,
        // a negative limit, and get_latest's refusing zero.
        ("swarmail_clear_inbox", r#"{"inbox": 42}"#, "inbox"),
        ("swarmail_clear_inbox", r#"{"inbox": null}"#, "inbox"),
        ("swarmail_wait_for_email", r#"{"to": ["a@x.io"]}"#, "to"),
        (
            "swarmail_wait_for_email",
            r#"{"since_ms": "yesterday"}"#,
            "since_ms",
        ),
        ("swarmail_search_emails", r#"{"limit": -1}"#, "limit"),
        ("swarmail_search_emails", r#"{"limit": "3"}"#, "limit"),
        ("swarmail_get_latest_email", r#"{"count": "3"}"#, "count"),
        ("swarmail_get_latest_email", r#"{"count": 0}"#, "count"),
        ("swarmail_get_email", r#"{"id": 7}"#, "id"),
        ("swarmail_seed_email", r#"{"to": 7}"#, "to"),
    ];
    for (name, args, field) in legs {
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "{name}", "arguments": {args}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{name} {args}");
        assert_eq!(body["error"]["code"], -32602, "{name} {args}: {body}");
        let message = body["error"]["message"].as_str().unwrap();
        assert!(message.contains(field), "{name} {args}: {message}");
        assert!(message.contains("must be"), "{name} {args}: {message}");
    }

    // The refusal is not just words: `clear_inbox {"inbox": 42}` did not
    // clear `default` — and stored nothing anywhere else either.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 0, "a refused clear_inbox clears nothing");
}

/// A refused wait registers no waiter (0025 §5 through the MCP plane): the
/// staging refusal answers before `store.wait_for` — the inbox's readiness
/// row is released — and the seeded mail is untouched for the next caller.
#[tokio::test]
async fn the_wait_registers_no_waiter_when_staging_refuses() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("nowaiter"),
        "f@x.io",
        "seeded@x.io",
        "seeded",
        "body",
    )
    .await
    .unwrap();

    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_wait_for_email", "arguments": {"inbox": "nowaiter", "to": ["a@x.io"], "timeout_ms": 60000}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(body["error"]["message"], "to must be a string", "{body}");

    // No waiter: the inbox's awaits row is gone the moment the refusal
    // answered — a registered park would still hold the row at 1.
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(
        metric_row_value(&metrics, r#"swarmail_awaits_active{inbox="nowaiter"}"#),
        None,
        "a refused wait registers no waiter: {metrics}"
    );

    // The seeded mail is untouched — the refused call consumed nothing.
    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "nowaiter"}"#,
    )
    .await;
    assert_eq!(text(&r)["total"], 1, "{r}");
}

/// `total` is the full match count, byte-for-byte the REST shape (AC1.5):
/// the page is the limit's business, so a caller paging at 2 of 3 matches
/// still learns there are three.
#[tokio::test]
async fn mcp_search_total_is_the_full_match_count() {
    let srv = start().await;
    for i in 0..3 {
        smtp_send(
            srv.smtp_addr,
            Some("totalbox"),
            "f@x.io",
            "match@x.io",
            &format!("m{i}"),
            "x",
        )
        .await
        .unwrap();
    }
    for i in 0..3 {
        smtp_send(
            srv.smtp_addr,
            Some("totalbox"),
            "f@x.io",
            "other@x.io",
            &format!("o{i}"),
            "x",
        )
        .await
        .unwrap();
    }

    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "totalbox", "to": "match@x.io", "limit": 2}"#,
    )
    .await;
    let t = text(&r);
    assert_eq!(t["total"], 3, "{t}");
    assert_eq!(t["emails"].as_array().unwrap().len(), 2, "{t}");

    // `limit: 0` is inside the staged domain: an empty page, the full count.
    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "totalbox", "to": "match@x.io", "limit": 0}"#,
    )
    .await;
    let t = text(&r);
    assert_eq!(t["total"], 3, "{t}");
    assert_eq!(t["emails"].as_array().unwrap().len(), 0, "{t}");

    // The default page (the shared 100) holds every match, and the total
    // counts only matches.
    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "totalbox", "to": "match@x.io"}"#,
    )
    .await;
    let t = text(&r);
    assert_eq!(t["total"], 3, "{t}");
    assert_eq!(t["emails"].as_array().unwrap().len(), 3, "{t}");
}

/// The code taxonomy (decision 0029 §2): `-32700` for a body that is not
/// JSON, `-32600` for a body that is not a request, `-32601` for a method
/// or tool that does not exist, `-32602` for a supplied argument outside
/// its domain — with the preserved messages.
#[tokio::test]
async fn the_envelope_carries_the_code_taxonomy() {
    let srv = start().await;

    // -32700: the parse failed before any request existed to carry an id;
    // the answer is a JSON-RPC error object, not a plain-text 400.
    let (st, body) = http_json(srv.http_addr, "POST", "/mcp", Some("{nope")).await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["jsonrpc"], "2.0", "{body}");
    assert_eq!(body["id"], serde_json::Value::Null, "{body}");
    assert_eq!(body["error"]["code"], -32700, "{body}");

    // -32600: a request with no method, carrying its id; a non-string
    // method is the same refusal.
    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":5}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], 5, "{body}");
    assert_eq!(body["error"]["code"], -32600, "{body}");
    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":6,"method":42}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], 6, "{body}");
    assert_eq!(body["error"]["code"], -32600, "{body}");

    // -32601, messages preserved: an unknown method and an unknown tool.
    let (st, body) = rpc(srv.http_addr, "swarmail_nope", "{}").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], 1, "{body}");
    assert_eq!(body["error"]["code"], -32601, "{body}");
    assert_eq!(
        body["error"]["message"], "unknown method: swarmail_nope",
        "{body}"
    );
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_nope", "arguments": {}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32601, "{body}");
    assert_eq!(
        body["error"]["message"], "unknown tool: swarmail_nope",
        "{body}"
    );

    // -32602, messages preserved: a missing id argument and a missing tool
    // name.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_get_email", "arguments": {}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(body["error"]["message"], "missing id", "{body}");
    let (st, body) = rpc(srv.http_addr, "tools/call", r#"{"arguments": {}}"#).await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(body["error"]["message"], "missing tool name", "{body}");
}

/// The id predicate is JSON-RPC's, not a method-name prefix (decision 0029
/// §2): an id-less request gets silence — 202 with an empty body over HTTP
/// — whatever its method is called; a notification *carrying* an id is a
/// request and answers -32601.
#[tokio::test]
async fn an_idless_request_is_a_notification() {
    let srv = start().await;
    for line in [
        // An id-less tools/call: no reply, no effect asked for.
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"swarmail_clear_all"}}"#,
        // An id-less notification: the same silence.
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        // A non-string method with no id: silence — the id is the predicate.
        r#"{"jsonrpc":"2.0","method":42}"#,
    ] {
        let (st, body) = http_send_text(srv.http_addr, "POST", "/mcp", Some(line)).await;
        assert_eq!(st, 202, "{line}: {body}");
        assert_eq!(
            body, "",
            "{line}: an id-less request is answered with silence"
        );
    }

    // A notification that carries an id is a request.
    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":7,"method":"notifications/initialized"}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], 7, "{body}");
    assert_eq!(
        body["error"]["message"], "unknown method: notifications/initialized",
        "{body}"
    );
}

/// Batches are answered in order and bounded (decision 0029 §2): requests
/// get their answers, notifications their silence, and `[]` or an array
/// over the cap is one -32600 naming the rule.
#[tokio::test]
async fn a_batch_is_answered_in_order_and_bounded() {
    let srv = start().await;
    let batch = format!(
        "[{},{},{}]",
        r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"swarmail_nope"}}"#,
    );
    let (st, body) = http_json(srv.http_addr, "POST", "/mcp", Some(&batch)).await;
    assert_eq!(st, 200, "{body}");
    let answers = body.as_array().expect("a batch is answered with an array");
    assert_eq!(answers.len(), 2, "the notification gets no answer: {body}");
    assert_eq!(answers[0]["id"], 1, "{body}");
    assert_eq!(answers[0]["result"], serde_json::json!({}), "{body}");
    assert_eq!(answers[1]["id"], 2, "{body}");
    assert_eq!(
        answers[1]["error"]["message"], "unknown tool: swarmail_nope",
        "{body}"
    );

    // An all-notification batch: an empty 202.
    let (st, body) = http_send_text(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"[{"jsonrpc":"2.0","method":"notifications/initialized"}]"#),
    )
    .await;
    assert_eq!(st, 202, "{body}");
    assert_eq!(body, "");

    // Empty and over-cap: one refusal, id null, naming the rule.
    for (batch, label, expects_cap) in [
        ("[]", "an empty batch", false),
        (
            &*format!("[{}]", vec!["0"; 17].join(",")),
            "an over-cap batch",
            true,
        ),
    ] {
        let (st, body) = http_json(srv.http_addr, "POST", "/mcp", Some(batch)).await;
        assert_eq!(st, 200, "{label}: {body}");
        assert_eq!(body["error"]["code"], -32600, "{label}: {body}");
        assert_eq!(body["id"], serde_json::Value::Null, "{label}: {body}");
        if expects_cap {
            assert!(
                body["error"]["message"].as_str().unwrap().contains("16"),
                "the refusal names the cap: {body}"
            );
        }
    }
}

/// `initialize` answers the one supported revision and advertises only
/// what exists (decision 0029 §2): the client's requested version is not
/// echoed, and `resources` — whose every method is unknown — is gone.
#[tokio::test]
async fn initialize_answers_the_supported_version_and_advertises_only_tools() {
    let srv = start().await;
    for requested in ["2025-06-18", "2024-11-05", "1999-01-01"] {
        let (st, body) = rpc(
            srv.http_addr,
            "initialize",
            &format!(r#"{{"protocolVersion": "{requested}"}}"#),
        )
        .await;
        assert_eq!(st, 200, "{requested}: {body}");
        assert_eq!(
            body["result"]["protocolVersion"], "2025-03-26",
            "{requested}: {body}"
        );
        let capabilities = &body["result"]["capabilities"];
        assert!(capabilities["tools"].is_object(), "{capabilities}");
        assert!(
            capabilities.get("resources").is_none(),
            "no resources are advertised: {capabilities}"
        );
    }
    // Even with no params at all: the constant is the answer.
    let (_, body) = rpc(srv.http_addr, "initialize", "{}").await;
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26", "{body}");
}

// ---------- the latest page's own min-one (decision 0029 §1) ----------

/// `count: 0` on the latest-email tool refuses rather than clamping: the
/// schema publishes `minimum: 1`, and a silent `.max(1)` would invent a
/// page the caller did not ask for. A legal count is honored — the page is
/// the newest `count` mails, newest first — and the *wait's* `count: 0`
/// keeps 0010's at-least-one semantic: a different domain, a different rule.
/// The honored side of the same parameter: two mails, a page of two —
/// newest first, as the store lists them.
#[tokio::test]
async fn get_latest_count_zero_is_refused_and_a_legal_count_is_honored() {
    let srv = start().await;
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_get_latest_email", "arguments": {"inbox": "pagezero", "count": 0}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["error"]["message"], "count must be an integer of at least 1",
        "{body}"
    );

    // The honored side of the same parameter: two mails, a page of two.
    for subject in ["first", "second"] {
        smtp_send(
            srv.smtp_addr,
            Some("pagecount"),
            "noreply@x.io",
            "w@x.io",
            subject,
            "body",
        )
        .await
        .unwrap();
    }
    let r = tool(
        srv.http_addr,
        "swarmail_get_latest_email",
        r#"{"inbox": "pagecount", "count": 2}"#,
    )
    .await;
    let page = text(&r);
    let page: Vec<&str> = page["emails"]
        .as_array()
        .expect("a page of emails")
        .iter()
        .map(|e| e["subject"].as_str().expect("subject"))
        .collect();
    assert_eq!(page, ["second", "first"], "{r}");
}

// ---------- the chaos shape is closed on the tool plane too ----------

/// An unknown argument key refuses with the key named (decision 0029 §3):
/// a typo'd key must not arm nothing while answering success.
#[tokio::test]
async fn set_chaos_refuses_an_unknown_argument_key_by_name() {
    let srv = start().await;
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_set_chaos", "arguments": {"event": "data", "probability": 100, "solus": 1}}"#,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["error"]["message"], "unknown argument: solus (use event|probability|error|delay_ms)",
        "{body}"
    );
}

/// Arguments that are not an object at all reach the `event` lookup and
/// refuse as missing — nothing is armed on a shape the rules cannot read.
#[tokio::test]
async fn set_chaos_with_unreadable_arguments_refuses_missing_event() {
    let srv = start().await;
    for arguments in ["\"data\"", "7", "null"] {
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_set_chaos", "arguments": {arguments}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{arguments}");
        assert_eq!(body["error"]["code"], -32602, "{arguments}: {body}");
        assert_eq!(
            body["error"]["message"], "missing event",
            "{arguments}: {body}"
        );
    }
}
