//! Every MCP tool and every protocol error path, driven through POST /mcp.

mod common;

use common::{
    SmtpConn, http_bytes, http_get_text, http_json, http_send_text, metric_row_value, smtp_send,
    start, start_with, wait_for_metric_row,
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
    assert_eq!(tools.len(), 17);

    // inputSchema is a JSON *object* for all seventeen tools — the JSON type,
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
    // Block structure keeps the code a standalone token for the renderer.
    let html_arg = "<p><a href='https://q.io?a=1&b=2'>go</a></p><p>code 555555</p>";
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
    let html_arg = "<p><a href='https://m.io'>go</a></p><p>code 334455</p>";
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
    assert!(
        email["text"].is_null(),
        "html supersedes text: nothing is synthesized (decision 0031 §1): {}",
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
            r#"{"to": "p@x.io", "subject": "parity", "html": "<p><a href='https://p.io'>go</a></p><p>code 778899</p>"}"#,
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
/// and a single-token AUTH PLAIN with no NULs at all names no inbox.
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

    // A single-token AUTH PLAIN (no NUL separators) names nothing: PLAIN is
    // exactly three fields (decision 0034 §5), so it is refused and the
    // delivery after it routes to `default`, never to the bare token.
    let mut c3 = SmtpConn::connect(srv.smtp_addr).await;
    c3.send("EHLO t").await;
    c3.reply().await;
    c3.send("AUTH PLAIN").await;
    assert!(c3.reply().await.starts_with("334"));
    c3.send(&base64_of("solobox")).await;
    assert_eq!(c3.reply().await, "501 5.5.2 Credentials refused");
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
    assert_eq!(body["count"], 0);
    let (_, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert!(
        list["emails"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["subject"] == "solo auth"),
        "{list}"
    );
}

/// Contract #1's MCP seed leg (decision 0012 §3, amended by 0014): with a
/// second real connection holding the writer lock, the seed tool's commit
/// fails and the tool answers an error result naming the cause — never an
/// `ok` result carrying an id that was never durable. Zero trace: the
/// refused seed is nowhere while live and the restart copy stays empty —
/// memory and data file agree, so the same call can simply retry.
#[tokio::test]
async fn a_blocked_commit_refuses_the_mcp_seed_with_an_error_result() {
    // The guard owns the data file and every sidecar: a failing assertion
    // leaves nothing in the temporary root (issue #143).
    let dir = tempfile::TempDir::with_prefix("swarmail-e2e-refuse-mcp-").unwrap();
    let db = dir.path().join("mail.db");
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
}

/// Finding 3's MCP leg (issue #31, decision 0014): with a real rejecting
/// BEFORE DELETE trigger on the data file, every destructive tool answers
/// an error result naming the cause — never an `ok` result for work that
/// did not commit — and the mail survives the restart untouched. No fault
/// shim: the refusal is the database's own.
#[tokio::test]
async fn a_blocked_delete_refuses_the_mcp_delete_tools_with_error_results() {
    // The guard owns the data file and every sidecar (issue #143).
    let dir = tempfile::TempDir::with_prefix("swarmail-e2e-refuse-mcp-del-").unwrap();
    let db = dir.path().join("mail.db");
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
}

// ---------- explicit bounds and observed totals (decision 0018) ----------

/// The wait cap is an `isError` refusal naming the maximum — the same text
/// REST refuses with — and it answers without parking (decision 0018: the
/// bound is refused before any subscribe, never clamped to it).
#[tokio::test]
async fn wait_over_the_cap_is_refused_without_parking() {
    let srv = start().await;
    let inbox = "next177-mcp-cap";
    next177_establish_registration(srv.http_addr, inbox).await;
    let before = next177_registered(srv.http_addr, inbox).await;
    let (st, body) = rpc(srv.http_addr, "tools/call", &format!(
        r#"{{"name":"swarmail_wait_for_email","arguments":{{"inbox":"{inbox}","count":1,"timeout_ms":60001}}}}"#)).await;
    assert_eq!(st, 200);
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(
        body["result"]["content"][0]["text"],
        "timeout_ms must be at most 60000"
    );
    assert_eq!(
        next177_registered(srv.http_addr, inbox).await,
        before,
        "cap refusal registered a wait"
    );
}

/// The wait results carry a `total` counted after the wait, in both arms —
/// the observed state, not a pre-wait snapshot (decision 0018).
#[tokio::test]
async fn wait_results_carry_the_post_wait_total_in_both_arms() {
    let srv = start().await;
    let addr = srv.http_addr;

    // The registration is cumulative and counts exactly once per wait
    // (issue #102): each arm reads its inbox's
    // `swarmail_awaits_registered_total` row before and after, and the
    // wait's delta is exactly one. A deleted or never-raised counter
    // leaves the delta at zero — a missing row reads as 0 — so this pins
    // the family's existence on /metrics, not just its refusal deltas.
    let registered = |text: &str| {
        common::metric_row_value(
            text,
            r#"swarmail_awaits_registered_total{inbox="observed"}"#,
        )
        .unwrap_or(0)
    };
    let (_, metrics) = common::http_get_text(addr, "/metrics").await;
    let registered_before = registered(&metrics);

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
    let (_, metrics) = common::http_get_text(addr, "/metrics").await;
    assert_eq!(
        registered(&metrics),
        registered_before + 1,
        "the wait registers exactly once: {metrics}"
    );

    // Snapshot the cumulative row before this wait, in the same process
    // and under its own label; the assertion below is a delta, not a reset
    // assumption about the process-shared registry.
    let (_, metrics) = common::http_get_text(addr, "/metrics").await;
    let registered_408_before = common::metric_row_value(
        &metrics,
        r#"swarmail_awaits_registered_total{inbox="observed408"}"#,
    )
    .unwrap_or(0);

    // Timeout arm: a non-matching mail lands mid-wait; total counts it
    // while `matched` stays honest about the filter. The wait is spawned
    // and observed parked *before* the mail is sent — mirroring the REST
    // twin — so the mail genuinely lands mid-wait and the post-wait total
    // is counted after the wait (issue #102). The bound is well past
    // poll+send+commit, so the mail cannot outrun the wait on a loaded
    // runner; the outer deadline still fails fast if the wait wedges.
    let request = tokio::spawn(tool(
        addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "observed408", "to": "nobody@x.io", "count": 9, "timeout_ms": 5000}"#,
    ));
    wait_for_metric_row(addr, r#"swarmail_awaits_active{inbox="observed408"}"#, 1).await;
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
        &tokio::time::timeout(std::time::Duration::from_secs(10), request)
            .await
            .expect("the wait must time out on its own bound")
            .unwrap(),
    );
    assert_eq!(result["timed_out"], true, "{result}");
    assert_eq!(result["matched"], 0);
    assert_eq!(
        result["total"], 1,
        "total is counted after the wait: {result}"
    );
    // The second wait registered exactly once more, under its own inbox.
    let (_, metrics) = common::http_get_text(addr, "/metrics").await;
    let registered_408_after = common::metric_row_value(
        &metrics,
        r#"swarmail_awaits_registered_total{inbox="observed408"}"#,
    )
    .expect("the 408 wait must expose its cumulative registration row");
    assert_eq!(
        registered_408_after,
        registered_408_before + 1,
        "one registration for the 408 arm: {metrics}"
    );
}

/// Every validation stage of `swarmail_set_chaos` refuses by name, and a
/// legal value still sets the rule (decision 0018: presence, type, `u8`
/// range and the 100 ceiling are distinct refusals; no cast, no clamp, no
/// silent default on any wire value).
#[tokio::test]
async fn set_chaos_refuses_every_malformed_wire_value_by_name() {
    let srv = start().await;

    // Establish two distinguishable probability-zero rules, then compare the
    // complete REST snapshot after every refused MCP write.
    let connect = tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event": "connect", "probability": 0, "error": "421 baseline connect", "delay_ms": 5}"#,
    )
    .await;
    assert_eq!(text(&connect)["connect"]["probability"], 0, "{connect}");
    let data = tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event": "data", "probability": 0, "error": "451 baseline data", "delay_ms": 15}"#,
    )
    .await;
    assert_eq!(text(&data)["data"]["probability"], 0, "{data}");
    let (snapshot_status, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(snapshot_status, 200, "{before}");

    // Every staging/type failure preserves the complete multi-event config.
    let cases: &[(&str, &str, &str)] = &[
        (
            r#"{"event":"data","probability":-1}"#,
            "negative probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":256}"#,
            "256 must refuse, not narrow to 0",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":"high"}"#,
            "a string probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":[]}"#,
            "an array probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":{}}"#,
            "an object probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":0.0}"#,
            "a float probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":150}"#,
            "150 is over the 100 ceiling",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data"}"#,
            "an absent probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":null}"#,
            "a null probability",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":101}"#,
            "101 is over the ceiling",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":"data","probability":255}"#,
            "255 is the u8 ceiling",
            "probability must be an integer between 0 and 100",
        ),
        (
            r#"{"event":[] ,"probability":0}"#,
            "an array event",
            "event must be a string",
        ),
        (
            r#"{"event":{} ,"probability":0}"#,
            "an object event",
            "event must be a string",
        ),
        (
            r#"{"event":null,"probability":0}"#,
            "a null event",
            "event must be a string",
        ),
        (
            r#"{"event":"dns","probability":0}"#,
            "an unknown event",
            "unknown event: dns (use connect|mail_from|rcpt|data)",
        ),
        (
            r#"{"event":"data","probability":0,"error":[]}"#,
            "an array error",
            "error must be a string",
        ),
        (
            r#"{"event":"data","probability":0,"error":{}}"#,
            "an object error",
            "error must be a string",
        ),
        (
            r#"{"event":"data","probability":0,"error":null}"#,
            "a null error",
            "error must be a string",
        ),
        (
            r#"{"event":"data","probability":0,"delay_ms":"soon"}"#,
            "a string delay",
            "delay_ms must be a non-negative integer",
        ),
        (
            r#"{"event":"data","probability":0,"delay_ms":[]}"#,
            "an array delay",
            "delay_ms must be a non-negative integer",
        ),
        (
            r#"{"event":"data","probability":0,"delay_ms":{}}"#,
            "an object delay",
            "delay_ms must be a non-negative integer",
        ),
        (
            r#"{"event":"data","probability":0,"delay_ms":null}"#,
            "a null delay",
            "delay_ms must be a non-negative integer",
        ),
    ];
    for (args, label, expected_message) in cases {
        let (status, installed) = http_json(
            srv.http_addr, "PUT", "/api/v1/chaos",
            Some(r#"{"connect": {"probability": 0, "error": "421 baseline connect", "delay_ms": 5}, "data": {"probability": 0, "error": "451 baseline data", "delay_ms": 15}}"#),
        ).await;
        assert_eq!(status, 200, "{installed}");
        let (get_status, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{before}");
        let (status, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_set_chaos", "arguments": {args}}}"#),
        )
        .await;
        assert_eq!(status, 200, "{label}: {body}");
        assert_eq!(body["error"]["code"], -32602, "{label}: {body}");
        assert_eq!(
            body["error"]["message"], *expected_message,
            "{label}: {body}"
        );
        let (get_status, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{label}: {after}");
        assert_eq!(after, before, "{label} changed active rules");
    }

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
    tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event":"connect","probability":0,"error":"421 kept"}"#,
    )
    .await;
    tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event":"data","probability":100,"error":"554 kept"}"#,
    )
    .await;
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
        let (get_status, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{before}");
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_set_chaos", "arguments": {args}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{args}: {body}");
        assert_eq!(body["error"]["code"], -32602, "{args}: {body}");
        assert_eq!(body["error"]["message"], sentence, "{args}: {body}");
        let (get_status, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{after}");
        assert_eq!(after, before, "{args} changed the config");
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

async fn next177_registered(addr: Http, inbox: &str) -> u64 {
    let (st, metrics) = http_get_text(addr, "/metrics").await;
    assert_eq!(st, 200);
    metric_row_value(
        &metrics,
        &format!("swarmail_awaits_registered_total{{inbox=\"{inbox}\"}}"),
    )
    .expect("the exclusive inbox must have an attributable cumulative row")
}

async fn next177_establish_registration(addr: Http, inbox: &str) {
    // Allow absence only before this positive control; afterward the row is
    // required so a renamed counter cannot turn the check into zero == zero.
    let (_, metrics) = http_get_text(addr, "/metrics").await;
    let registered_before = metric_row_value(
        &metrics,
        &format!("swarmail_awaits_registered_total{{inbox=\"{inbox}\"}}"),
    )
    .unwrap_or(0);
    let result = tool(
        addr,
        "swarmail_wait_for_email",
        &format!(r#"{{"inbox":"{inbox}","to":"never@x.io","timeout_ms":0}}"#),
    )
    .await;
    assert_eq!(text(&result)["timed_out"], true);
    assert_eq!(text(&result)["matched"], 0);
    assert_eq!(
        next177_registered(addr, inbox).await,
        registered_before + 1,
        "the positive control registers exactly once"
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

    next177_establish_registration(srv.http_addr, "malformed-wait").await;
    for (value, label) in [
        (r#""invalid""#, "a string is not an integer"),
        ("-1", "a negative is over no bound — it is unparseable"),
        ("1.5", "a float would be a narrowing cast"),
        ("null", "null is supplied: the key is present"),
    ] {
        let before = next177_registered(srv.http_addr, "malformed-wait").await;
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
        assert_eq!(
            next177_registered(srv.http_addr, "malformed-wait").await,
            before,
            "{label}: a refusal must not register a wait"
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
    next177_establish_registration(srv.http_addr, "zero-wait").await;
    let before = next177_registered(srv.http_addr, "zero-wait").await;
    let result = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox":"zero-wait","to":"nobody@x.io","timeout_ms":0}"#,
    )
    .await;
    assert_eq!(text(&result)["timed_out"], true, "{result}");
    assert_eq!(text(&result)["matched"], 0, "{result}");
    assert_eq!(
        next177_registered(srv.http_addr, "zero-wait").await,
        before + 1
    );
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(
        metric_row_value(&metrics, r#"swarmail_awaits_active{inbox="zero-wait"}"#),
        None
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

#[derive(Debug, PartialEq)]
struct DefaultMailSnapshot {
    count: u64,
    messages: serde_json::Value,
    raw: Vec<(String, Vec<u8>)>,
}

async fn default_mail_snapshot(addr: Http) -> DefaultMailSnapshot {
    let (status, count) = http_json(addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(status, 200, "{count}");
    let (status, messages) = http_json(addr, "GET", "/api/v1/inboxes/default/messages", None).await;
    assert_eq!(status, 200, "{messages}");
    let emails = messages["emails"].as_array().expect("message list");
    let count = count["count"].as_u64().expect("exact count");
    assert_eq!(count, emails.len() as u64, "{messages}");
    let mut raw = Vec::with_capacity(emails.len());
    for email in emails {
        let id = email["id"].as_str().expect("stored id").to_string();
        let (status, bytes) = http_bytes(addr, &format!("/api/v1/messages/{id}/raw")).await;
        assert_eq!(status, 200, "raw {id}");
        raw.push((id, bytes));
    }
    DefaultMailSnapshot {
        count,
        messages,
        raw,
    }
}

async fn assert_default_snapshot(addr: Http, expected: &DefaultMailSnapshot, when: &str) {
    let actual = default_mail_snapshot(addr).await;
    assert_eq!(actual, *expected, "{when}");
}

/// A *present* argument of the wrong JSON type — or explicit `null` —
/// refuses -32602 with a sentence naming the field and its domain, on
/// every tool that reads it (decision 0029 §1), and the effect never
/// happens.
#[tokio::test]
async fn a_wrong_typed_argument_refuses_by_name() {
    let srv = start().await;
    let (seed_status, seeded) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/default/seed",
        Some(r#"{"to": "retained@x.io", "subject": "clear refusal witness"}"#),
    )
    .await;
    assert_eq!(seed_status, 200, "{seeded}");
    let retained_id = seeded["id"].as_str().unwrap().to_string();
    let retained = default_mail_snapshot(srv.http_addr).await;
    assert_eq!(retained.count, 1, "the keeper starts as one mail");
    assert_eq!(retained.messages["emails"][0]["id"], retained_id);
    let legs: &[(&str, &str, &str)] = &[
        // (tool, arguments, field named in the refusal) — one leg per wrong
        // shape: string→int, int→string, array→string, explicit null,
        // a negative limit, and get_latest's refusing zero.
        (
            "swarmail_clear_inbox",
            r#"{"inbox": 42}"#,
            "inbox must be a string",
        ),
        (
            "swarmail_clear_inbox",
            r#"{"inbox": null}"#,
            "inbox must be a string",
        ),
        (
            "swarmail_clear_inbox",
            r#"{"inbox": []}"#,
            "inbox must be a string",
        ),
        (
            "swarmail_clear_inbox",
            r#"{"inbox": {}}"#,
            "inbox must be a string",
        ),
        (
            "swarmail_clear_inbox",
            r#"{"inbx": "typo"}"#,
            "unknown argument: inbx",
        ),
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
        ("swarmail_seed_email", r#"{"to": 7}"#, "to must be a string"),
        (
            "swarmail_seed_email",
            r#"{"to": null}"#,
            "to must be a string",
        ),
        (
            "swarmail_seed_email",
            r#"{"to": []}"#,
            "to must be a string",
        ),
        (
            "swarmail_seed_email",
            r#"{"to": {}}"#,
            "to must be a string",
        ),
        (
            "swarmail_seed_email",
            r#"{"typo": "ignored", "to": "new@example.test"}"#,
            "unknown argument: typo",
        ),
    ];
    for (name, args, expected_message) in legs {
        let state_sensitive = *name == "swarmail_clear_inbox" || *name == "swarmail_seed_email";
        if state_sensitive {
            assert_default_snapshot(srv.http_addr, &retained, "immediately before refusal").await;
        }
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "{name}", "arguments": {args}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{name} {args}");
        assert_eq!(body["error"]["code"], -32602, "{name} {args}: {body}");
        let message = body["error"]["message"].as_str().unwrap();
        if state_sensitive {
            assert_eq!(message, *expected_message, "{name} {args}: {message}");
            assert_default_snapshot(srv.http_addr, &retained, "immediately after refusal").await;
        } else {
            assert!(
                message.contains(expected_message),
                "{name} {args}: {message}"
            );
            assert!(message.contains("must be"), "{name} {args}: {message}");
        }
    }

    assert_default_snapshot(
        srv.http_addr,
        &retained,
        "after all malformed argument probes",
    )
    .await;
}

/// The arguments *container* is itself a supplied value (0025 §2 through
/// 0029 §1): a present container that is not an object — an array, a
/// scalar, an explicit null — refuses -32602 by name before any staging,
/// so `swarmail_clear_inbox` with `"arguments": []` can never silently
/// fall through to the schema default and clear `default`.
#[tokio::test]
async fn a_malformed_arguments_container_refuses_before_any_effect() {
    let srv = start().await;
    let (seed_status, keeper) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/default/seed",
        Some(r#"{"to":"keeper@example.com","subject":"keeper","text":"still here 778899"}"#),
    )
    .await;
    assert_eq!(seed_status, 200, "{keeper}");
    let retained = default_mail_snapshot(srv.http_addr).await;
    assert_eq!(
        retained.count, 1,
        "the clear victim is seeded before refusals"
    );

    for name in ["swarmail_clear_inbox", "swarmail_seed_email"] {
        for container in ["[]", "42", "\"clear\"", "null"] {
            assert_default_snapshot(srv.http_addr, &retained, "before malformed container").await;
            let (st, body) = rpc(
                srv.http_addr,
                "tools/call",
                &format!(r#"{{"name": "{name}", "arguments": {container}}}"#),
            )
            .await;
            assert_eq!(st, 200, "{name} {container}");
            assert_eq!(body["error"]["code"], -32602, "{name} {container}: {body}");
            assert_eq!(
                body["error"]["message"], "arguments must be an object of named arguments",
                "{name} {container}: {body}"
            );
            assert_default_snapshot(srv.http_addr, &retained, "after malformed container").await;
        }
    }
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

    next177_establish_registration(srv.http_addr, "nowaiter").await;

    // The cumulative registration is the observable (issue #102): the
    // delta across the refused call must stay zero — a wait that
    // subscribed before validating would leave a permanent +1, which the
    // released gauge cannot show.
    let registered_before = next177_registered(srv.http_addr, "nowaiter").await;
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_wait_for_email", "arguments": {"inbox": "nowaiter", "to": ["a@x.io"], "timeout_ms": 60000}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(body["error"]["message"], "to must be a string", "{body}");

    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    let registered_after = next177_registered(srv.http_addr, "nowaiter").await;
    assert_eq!(
        registered_after, registered_before,
        "a refused wait registers no waiter: {metrics}"
    );

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
/// §2): an id-less request with a string method gets silence — 202 with an
/// empty body over HTTP — whatever its method is called; a notification
/// *carrying* an id is a request and answers -32601. (An id-less object
/// with no string method is no notification: see the test below.)
#[tokio::test]
async fn an_idless_request_is_a_notification() {
    let srv = start().await;
    for line in [
        // An id-less tools/call: no reply, no effect asked for.
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"swarmail_clear_all"}}"#,
        // An id-less notification: the same silence.
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        // An unknown method is still a notification: silence.
        r#"{"jsonrpc":"2.0","method":"notifications/nope"}"#,
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

/// An id-less object is a notification only with a string method (decision
/// 0047 §3, issue #132; red before: every case below was an empty 202):
/// `{}`, a lost method and a numeric one each get 200, `-32600` and a null
/// id; in a batch the malformed member keeps its null-id slot while the
/// notification beside it stays silent, and sixteen malformed members get
/// sixteen errors. `notifications/initialized` is still an empty 202.
#[tokio::test]
async fn malformed_objects_answer_over_http() {
    let srv = start().await;
    for (body, message) in [
        ("{}", "missing method"),
        (r#"{"foo":"boo"}"#, "missing method"),
        (r#"{"method":1}"#, "method must be a string"),
        (
            r#"{"jsonrpc":"2.0","method":42}"#,
            "method must be a string",
        ),
    ] {
        let (st, reply) = http_json(srv.http_addr, "POST", "/mcp", Some(body)).await;
        assert_eq!(st, 200, "{body}: {reply}");
        assert_eq!(
            reply,
            serde_json::json!({"jsonrpc":"2.0","id":null,
                "error":{"code":-32600,"message":message}}),
            "{body}"
        );
    }

    let mixed = r#"[{"foo":"boo"},{"jsonrpc":"2.0","method":"x"}]"#;
    let (st, reply) = http_json(srv.http_addr, "POST", "/mcp", Some(mixed)).await;
    assert_eq!(st, 200, "{reply}");
    let answers = reply.as_array().expect("an array");
    assert_eq!(answers.len(), 1, "one element: {reply}");
    assert_eq!(answers[0]["error"]["code"], -32600, "{reply}");
    assert_eq!(answers[0]["id"], serde_json::Value::Null, "{reply}");

    let sixteen = format!("[{}]", vec![r#"{"foo":"boo"}"#; 16].join(","));
    let (st, reply) = http_json(srv.http_addr, "POST", "/mcp", Some(&sixteen)).await;
    assert_eq!(st, 200, "{reply}");
    let answers = reply.as_array().expect("an array");
    assert_eq!(answers.len(), 16, "{reply}");
    assert!(
        answers
            .iter()
            .all(|a| a["error"]["code"] == -32600 && a["id"].is_null()),
        "{reply}"
    );

    let initialized = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let (st, body) = http_send_text(srv.http_addr, "POST", "/mcp", Some(initialized)).await;
    assert_eq!((st, body.as_str()), (202, ""));
}

#[tokio::test]
async fn supplied_non_scalar_ids_refuse_before_tool_effects_over_http() {
    let srv = start().await;
    let invalid_ids = [
        serde_json::json!(true),
        serde_json::json!(false),
        serde_json::json!({}),
        serde_json::json!({"nested": 1}),
        serde_json::json!([]),
        serde_json::json!([1]),
    ];
    for (index, id) in invalid_ids.into_iter().enumerate() {
        let inbox = format!("invalid-id-{index}");
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": "swarmail_seed_email",
                        "arguments": { "inbox": inbox, "to": "a@x.io" } }
        });
        let (status, reply) =
            http_json(srv.http_addr, "POST", "/mcp", Some(&request.to_string())).await;
        assert_eq!(status, 200, "{request}: {reply}");
        assert_eq!(reply["id"], serde_json::Value::Null, "{request}: {reply}");
        assert_eq!(reply["error"]["code"], -32600, "{request}: {reply}");
        assert_eq!(
            reply["error"]["message"], "id must be a string, number, or null",
            "{request}: {reply}"
        );
        let (_, list) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{inbox}/messages"),
            None,
        )
        .await;
        assert_eq!(list["total"], 0, "{request} had a tool effect: {list}");
    }

    for id in [
        serde_json::json!("text"),
        serde_json::json!(7),
        serde_json::json!(-2),
        serde_json::json!(1.5),
        serde_json::Value::Null,
    ] {
        let request = serde_json::json!({"jsonrpc":"2.0", "id":id, "method":"ping"});
        let (status, reply) =
            http_json(srv.http_addr, "POST", "/mcp", Some(&request.to_string())).await;
        assert_eq!(status, 200, "{request}: {reply}");
        assert_eq!(reply["id"], id, "{request}: {reply}");
        assert_eq!(reply["result"], serde_json::json!({}), "{request}: {reply}");
    }

    let batch = serde_json::json!([
        {"id": true, "method": "ping"},
        {"id": "dup", "method": "ping"},
        {"method": "notifications/initialized"},
        {"id": false, "method": "tools/call", "params": {
            "name": "swarmail_seed_email",
            "arguments": {"inbox": "invalid-batch-id", "to": "a@x.io"}
        }},
        {"id": null, "method": "ping"},
        {"id": "dup", "method": "ping"},
        {"id": {}, "method": "ping"},
        {"id": [], "method": "ping"}
    ]);
    let (status, replies) =
        http_json(srv.http_addr, "POST", "/mcp", Some(&batch.to_string())).await;
    assert_eq!(status, 200, "{replies}");
    let replies = replies.as_array().expect("mixed batch replies");
    assert_eq!(
        replies.len(),
        7,
        "only the valid notification is omitted: {replies:?}"
    );
    assert_eq!(
        replies
            .iter()
            .map(|reply| reply["id"].clone())
            .collect::<Vec<_>>(),
        [
            serde_json::Value::Null,
            serde_json::json!("dup"),
            serde_json::Value::Null,
            serde_json::Value::Null,
            serde_json::json!("dup"),
            serde_json::Value::Null,
            serde_json::Value::Null
        ]
    );
    for index in [0, 2, 5, 6] {
        assert_eq!(replies[index]["error"]["code"], -32600, "{replies:?}");
    }
    let (_, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/invalid-batch-id/messages",
        None,
    )
    .await;
    assert_eq!(list["total"], 0, "invalid batch ID ran a tool: {list}");
}

#[cfg(target_pointer_width = "32")]
#[tokio::test]
async fn mcp_search_offset_refuses_values_beyond_32_bit_usize() {
    let srv = start().await;
    let max = u64::try_from(usize::MAX).unwrap();
    for (offset, expected_error) in [(max, false), (max + 1, true)] {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name":"swarmail_search_emails", "arguments":{"offset":offset}}
        });
        let (status, reply) =
            http_json(srv.http_addr, "POST", "/mcp", Some(&request.to_string())).await;
        assert_eq!(status, 200, "{request}: {reply}");
        if expected_error {
            assert_eq!(reply["error"]["code"], -32602, "{reply}");
            assert_eq!(
                reply["error"]["message"], "offset must be a non-negative integer that fits usize",
                "{reply}"
            );
        } else {
            assert_eq!(reply["result"]["isError"], false, "{reply}");
        }
    }
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

/// Silence is an answer, not a skip (decision 0039 §2): an id-less tool
/// call — alone or inside a notification-only batch — gets no reply, yet
/// its effect happens. The shared classification decides the reply, never
/// whether the call runs.
#[tokio::test]
async fn a_silent_notification_still_takes_effect() {
    let srv = start().await;
    let seed = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"silent","to":"a@x.io"}}}"#;
    for body in [seed.to_string(), format!("[{seed}]")] {
        let (st, reply) = http_send_text(srv.http_addr, "POST", "/mcp", Some(&body)).await;
        assert_eq!(st, 202, "{body}: {reply}");
        assert_eq!(reply, "", "{body}");
    }
    let (st, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/silent/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 2, "both silent seeds ran: {list}");
}

/// Unsupported proposals receive the newest supported revision (0079);
/// only existing capabilities are advertised.
#[tokio::test]
async fn initialize_answers_the_supported_version_and_advertises_only_tools() {
    let srv = start().await;
    for requested in ["2024-11-05", "1999-01-01"] {
        let (st, body) = rpc(
            srv.http_addr,
            "initialize",
            &format!(r#"{{"protocolVersion": "{requested}"}}"#),
        )
        .await;
        assert_eq!(st, 200, "{requested}: {body}");
        assert_eq!(
            body["result"]["protocolVersion"], "2025-11-25",
            "{requested}: {body}"
        );
        let capabilities = &body["result"]["capabilities"];
        assert!(capabilities["tools"].is_object(), "{capabilities}");
        assert!(
            capabilities.get("resources").is_none(),
            "no resources are advertised: {capabilities}"
        );
    }
    // Omitted proposals preserve legacy negotiation.
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
    tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event":"connect","probability":0,"error":"421 prior connect"}"#,
    )
    .await;
    tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event":"data","probability":100,"error":"554 prior data"}"#,
    )
    .await;
    let (snapshot_status, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(snapshot_status, 200, "{before}");
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
    let (get_status, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(get_status, 200, "{after}");
    assert_eq!(after, before, "unknown key changed active rules");
}

/// Arguments that are not an object at all refuse as a malformed
/// container (0025 §2, the review's HIGH): the shape itself is a supplied
/// value — it can never fall through to `missing event`, and nothing is
/// armed on a shape the rules cannot read.
#[tokio::test]
async fn set_chaos_with_unreadable_arguments_refuse_as_a_container() {
    let srv = start().await;
    tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event":"connect","probability":0,"error":"421 prior connect"}"#,
    )
    .await;
    tool(
        srv.http_addr,
        "swarmail_set_chaos",
        r#"{"event":"data","probability":100,"error":"554 prior data"}"#,
    )
    .await;
    let (snapshot_status, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(snapshot_status, 200, "{before}");

    for arguments in ["[]", "\"data\"", "7", "null"] {
        let (get_status, current) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{current}");
        assert_eq!(
            current, before,
            "the config is installed before each shape refusal"
        );
        let (st, body) = rpc(
            srv.http_addr,
            "tools/call",
            &format!(r#"{{"name": "swarmail_set_chaos", "arguments": {arguments}}}"#),
        )
        .await;
        assert_eq!(st, 200, "{arguments}");
        assert_eq!(body["error"]["code"], -32602, "{arguments}: {body}");
        assert_eq!(
            body["error"]["message"], "arguments must be an object of named arguments",
            "{arguments}: {body}"
        );
        let (get_status, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{after}");
        assert_eq!(after, before, "{arguments} changed active rules");
    }

    // An object container without an event still reaches the event lookup;
    // absence is a staging refusal, and it also leaves the installed rules.
    for call in [
        r#"{"name": "swarmail_set_chaos", "arguments": {}}"#,
        r#"{"name": "swarmail_set_chaos"}"#,
    ] {
        let (st, body) = rpc(srv.http_addr, "tools/call", call).await;
        assert_eq!(st, 200, "{call}");
        assert_eq!(body["error"]["code"], -32602, "{call}: {body}");
        assert_eq!(body["error"]["message"], "missing event", "{call}: {body}");
        let (get_status, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
        assert_eq!(get_status, 200, "{after}");
        assert_eq!(after, before, "{call} changed active rules");
    }
}

// ---------- the bounded staging (decision 0030 §3) ----------

/// The page cap refuses through the MCP staging shape: `-32602` error
/// objects naming field and range in one sentence, on all three staged
/// parameters — and the boundary itself stages fine, so 250 is a page on
/// every plane.
#[tokio::test]
async fn the_staging_refuses_over_the_cap_and_honors_the_boundary() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("capbox"),
        "f@x.io",
        "cap@x.io",
        "within the cap",
        "body",
    )
    .await
    .unwrap();

    // Search's limit.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_search_emails", "arguments": {"inbox": "capbox", "limit": 251}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["error"]["message"], "limit must be at most 250",
        "{body}"
    );
    // get_latest's count.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_get_latest_email", "arguments": {"inbox": "capbox", "count": 251}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["error"]["message"], "count must be at most 250",
        "{body}"
    );
    // The wait's count — the refusal answers before any park (0025 §5).
    // The cumulative registration is the observable (issue #102): the
    // delta across the refused call must stay zero — a wait that
    // subscribed before validating would leave a permanent +1, which the
    // released gauge cannot show.
    next177_establish_registration(srv.http_addr, "capbox").await;
    let registered_before = next177_registered(srv.http_addr, "capbox").await;
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_wait_for_email", "arguments": {"inbox": "capbox", "to": "cap@x.io", "count": 251, "timeout_ms": 60000}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["error"]["message"], "count must be at most 250",
        "{body}"
    );
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    let registered_after = next177_registered(srv.http_addr, "capbox").await;
    assert_eq!(
        registered_after, registered_before,
        "a refused wait registers no waiter: {metrics}"
    );
    // The prompt refusal is the 0025 §5 proof itself: a parked wait could
    // not have answered. (The awaits gauge cannot - its row is removed at
    // completion, so a scrape after the fact says nothing about whether a
    // waiter ever registered.)
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(
        metric_row_value(&metrics, r#"swarmail_awaits_active{inbox="capbox"}"#),
        None,
        "the released row is gone: {metrics}"
    );

    // The boundary stages fine on all three: 250 is a page (get_latest
    // answers with the mail), and a 250-count wait is accepted — it times
    // out as a wait, not as a refusal.
    let r = tool(
        srv.http_addr,
        "swarmail_get_latest_email",
        r#"{"inbox": "capbox", "count": 250}"#,
    )
    .await;
    assert_eq!(text(&r)["emails"].as_array().unwrap().len(), 1, "{r}");
    let r = tool(
        srv.http_addr,
        "swarmail_search_emails",
        r#"{"inbox": "capbox", "limit": 250}"#,
    )
    .await;
    assert_eq!(text(&r)["total"], 1, "{r}");
    let r = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "capbox", "to": "nobody@x.io", "count": 250, "timeout_ms": 200}"#,
    )
    .await;
    let wait = text(&r);
    assert_eq!(
        wait["timed_out"], true,
        "an accepted wait times out as a wait: {r}"
    );
    assert_eq!(wait["matched"], 0, "{r}");
}

/// The cap touches none of the count semantics it must not touch (0030
/// §3): get_latest's `count: 0` still refuses by name (its schema's
/// `minimum: 1`), the wait's `count: 0` still means at least one (0010) —
/// neither is repurposed by the cap check that shares their parameter.
#[tokio::test]
async fn the_cap_leaves_the_count_poles_alone() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("poles"),
        "f@x.io",
        "p@x.io",
        "pole mail",
        "body",
    )
    .await
    .unwrap();

    // get_latest's zero refuses by name, as before the cap.
    let (st, body) = rpc(
        srv.http_addr,
        "tools/call",
        r#"{"name": "swarmail_get_latest_email", "arguments": {"inbox": "poles", "count": 0}}"#,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
    assert_eq!(
        body["error"]["message"], "count must be an integer of at least 1",
        "{body}"
    );

    // The wait's zero still means at least one: the mail is there, so the
    // wait answers immediately with it (a literal zero-count would match
    // nothing).
    let r = tool(
        srv.http_addr,
        "swarmail_wait_for_email",
        r#"{"inbox": "poles", "to": "p@x.io", "count": 0, "timeout_ms": 5000}"#,
    )
    .await;
    assert_eq!(text(&r)["matched"], 1, "{r}");
}

// ---------- run 21: #91 the envelope's transport and value shapes ----------

/// POST raw bytes to `/mcp` with the given framing headers: (status,
/// content type, parsed JSON body). The request is written whole — a write
/// the server cut short is tolerated — so the refusal can be read whatever
/// the server consumed.
async fn post_mcp_bytes(
    addr: Http,
    framing: &str,
    body: &[u8],
) -> (u16, Option<String>, serde_json::Value) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n{framing}Connection: close\r\n\r\n"
    )
    .into_bytes();
    request.extend_from_slice(body);
    let _ = stream.write_all(&request).await;
    let mut reply = Vec::new();
    let _ = stream.read_to_end(&mut reply).await;
    let split = reply
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a response head");
    let head = String::from_utf8_lossy(&reply[..split]).to_lowercase();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let content_type = head
        .lines()
        .find_map(|l| l.strip_prefix("content-type: "))
        .map(str::to_string);
    let json = serde_json::from_slice(&reply[split + 4..]).unwrap_or(serde_json::Value::Null);
    (status, content_type, json)
}

/// Every malformed `/mcp` body is a JSON-RPC error object (decision 0044
/// §1, issue #91): bad JSON and invalid UTF-8 are HTTP 400 `-32700`; a
/// scalar, `null` or string is HTTP 200 `-32600`; each non-object batch
/// member — a nested array included — earns its own null-id `-32600` in
/// member order, while a notification member still runs silently. Red
/// before: invalid UTF-8 was a text/plain 400 and every non-object a 202.
#[tokio::test]
async fn malformed_and_non_object_bodies_get_json_rpc_errors() {
    let srv = start().await;
    let addr = srv.http_addr;
    for (body, status, code) in [
        (&b"{nope"[..], 400, -32700),
        (
            &b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"\xff\"}"[..],
            400,
            -32700,
        ),
        (&b"\xff\xfe"[..], 400, -32700),
        (&b"5"[..], 200, -32600),
        (&b"null"[..], 200, -32600),
        (&b"\"x\""[..], 200, -32600),
    ] {
        let framing = format!("Content-Length: {}\r\n", body.len());
        let (st, content_type, reply) = post_mcp_bytes(addr, &framing, body).await;
        let shown = String::from_utf8_lossy(body);
        assert_eq!(st, status, "{shown}: {reply}");
        assert_eq!(content_type.as_deref(), Some("application/json"), "{shown}");
        assert_eq!(reply["error"]["code"], code, "{shown}: {reply}");
        assert_eq!(reply["id"], serde_json::Value::Null, "{shown}");
    }

    let seed = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"mixed","to":"a@x.io"}}}"#;
    let batch = format!(
        r#"[{{"jsonrpc":"2.0","id":1,"method":"ping"}},5,{seed},[],"s",{{"jsonrpc":"2.0","id":1,"method":"ping"}}]"#
    );
    let (st, reply) = http_json(addr, "POST", "/mcp", Some(&batch)).await;
    assert_eq!(st, 200, "{reply}");
    let answers = reply.as_array().unwrap();
    let ids: Vec<&serde_json::Value> = answers.iter().map(|a| &a["id"]).collect();
    assert_eq!(
        ids,
        [
            &serde_json::json!(1),
            &serde_json::Value::Null,
            &serde_json::Value::Null,
            &serde_json::Value::Null,
            &serde_json::json!(1)
        ],
        "{reply}"
    );
    for invalid in &answers[1..4] {
        assert_eq!(invalid["error"]["code"], -32600, "{reply}");
    }
    let (_, count) = http_json(addr, "GET", "/api/v1/inboxes/mixed/count", None).await;
    assert_eq!(count["count"], 1, "the notification member ran once");

    // An empty or over-cap batch refuses whole, before any member runs.
    let seeds = format!("[{}]", vec![seed; 17].join(","));
    let (st, reply) = http_json(addr, "POST", "/mcp", Some(&seeds)).await;
    assert_eq!(
        (st, reply["error"]["code"].clone()),
        (200, serde_json::json!(-32600))
    );
    let (_, count) = http_json(addr, "GET", "/api/v1/inboxes/mixed/count", None).await;
    assert_eq!(count["count"], 1, "no member of the refused batch ran");
}

/// The body limit is axum's streaming 2 MiB, now answered as JSON-RPC
/// (decision 0044 §1): exactly 2,097,152 bytes is served, one byte more is
/// HTTP 413 `-32600` naming the limit — by `Content-Length` and chunked
/// alike, so the size is never learned by reading an unbounded body. Red
/// before: axum's plain-text 413.
#[tokio::test]
async fn the_mcp_body_limit_is_a_json_rpc_refusal() {
    let srv = start().await;
    let addr = srv.http_addr;
    let exact = |len: usize| {
        let head = r#"{"jsonrpc":"2.0","id":9,"method":"ping","pad":""#;
        let tail = r#""}"#;
        format!("{head}{}{tail}", "x".repeat(len - head.len() - tail.len())).into_bytes()
    };
    let limit = 2_097_152;
    let fits = exact(limit);
    let (st, _, reply) = post_mcp_bytes(addr, &format!("Content-Length: {limit}\r\n"), &fits).await;
    assert_eq!(
        (st, reply["id"].clone()),
        (200, serde_json::json!(9)),
        "{reply}"
    );

    let over = exact(limit + 1);
    let framing = format!("Content-Length: {}\r\n", over.len());
    let (st, content_type, reply) = post_mcp_bytes(addr, &framing, &over).await;
    assert_eq!(st, 413, "{reply}");
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(reply["error"]["code"], -32600, "{reply}");
    assert_eq!(reply["id"], serde_json::Value::Null);
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("2097152"),
        "the limit is named: {reply}"
    );

    let mut chunked = format!("{:x}\r\n", over.len()).into_bytes();
    chunked.extend_from_slice(&over);
    chunked.extend_from_slice(b"\r\n0\r\n\r\n");
    let (st, _, reply) = post_mcp_bytes(addr, "Transfer-Encoding: chunked\r\n", &chunked).await;
    assert_eq!(st, 413, "{reply}");
    assert_eq!(reply["error"]["code"], -32600, "{reply}");
}

/// A wait and the seed that satisfies it, in one batch, complete together
/// (decision 0044 §2): the members are polled concurrently, so the wait
/// sees the seed instead of parking its full 60 s before the seed may run.
/// Red before: the serial loop timed the wait out first.
#[tokio::test]
async fn a_batched_wait_is_satisfied_by_its_batched_seed() {
    let srv = start().await;
    let batch = r#"[
        {"jsonrpc":"2.0","id":"wait","method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"together","timeout_ms":60000}}},
        {"jsonrpc":"2.0","id":"seed","method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"together","to":"a@x.io"}}}
    ]"#;
    let (st, reply) = http_json(srv.http_addr, "POST", "/mcp", Some(batch)).await;
    assert_eq!(st, 200, "{reply}");
    assert_eq!(reply[0]["id"], "wait", "replies keep member order");
    assert_eq!(reply[1]["id"], "seed");
    let waited = text(&reply[0]["result"]);
    assert_eq!(waited["matched"], 1, "{reply}");
    assert!(waited.get("timed_out").is_none(), "{reply}");
}

/// Poll the real dispatch with fully available bytes, alongside wire tests.
#[tokio::test]
async fn zero_timeout_finishes_on_the_first_dispatch_poll() {
    use std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll},
    };
    use swarmail::{
        chaos::Chaos,
        mcp::{self, McpContext},
        store::Store,
    };

    for (name, timeout, ready) in [
        ("dispatch-zero", Some(0), true),
        ("dispatch-default", None, false),
        ("dispatch-positive", Some(60000), false),
    ] {
        let (stop, shutdown) = tokio::sync::watch::channel(false);
        let ctx = McpContext {
            tool_rate: Arc::new(swarmail::mcp::ToolRate::default()),
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            shutdown,
        };
        let mut args = serde_json::json!({"inbox": name, "count": 1});
        if let Some(timeout) = timeout {
            args["timeout_ms"] = timeout.into();
        }
        let bytes = serde_json::to_vec(&serde_json::json!({"jsonrpc":"2.0", "id": 177,
            "method":"tools/call", "params":{"name":"swarmail_wait_for_email", "arguments":args}}))
        .unwrap();
        let mut request = std::pin::pin!(mcp::dispatch(&ctx, &bytes));
        let observed = request.as_mut().poll(&mut Context::from_waker(
            futures_util::task::noop_waker_ref(),
        ));
        assert_eq!(observed.is_ready(), ready, "first dispatch poll: {name}");
        let response = match observed {
            Poll::Ready(response) => response,
            Poll::Pending => {
                stop.send(true).unwrap();
                request.await
            }
        };
        assert_eq!(response.status().as_u16(), 200);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["id"], 177);
        if ready {
            let result = text(&body["result"]);
            assert_eq!(result["timed_out"], true);
            assert_eq!(result["matched"], 0);
        } else {
            assert_eq!(body["result"]["isError"], true);
            assert_eq!(body["result"]["content"][0]["text"], "shutting down");
        }
    }
}

// ---- Attachments (decision 0110): real SMTP in, MCP out. ----

const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn wrapped_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .encode(bytes)
        .as_bytes()
        .chunks(76)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// A base64 attachment part: (headers, body).
fn att_binary(name: &str, bytes: &[u8]) -> (String, String) {
    (
        format!(
            "Content-Type: application/octet-stream\r\nContent-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"{name}\""
        ),
        wrapped_base64(bytes),
    )
}

fn att_mail(parts: &[(String, String)]) -> Vec<u8> {
    let mut raw = String::from(
        "From: app@x.io\r\nTo: rcpt@x.io\r\nSubject: attachments\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n--bd\r\n\
         Content-Type: text/plain\r\n\r\nsee attached\r\n",
    );
    for (headers, body) in parts {
        raw.push_str(&format!("--bd\r\n{headers}\r\n\r\n{body}\r\n"));
    }
    raw.push_str("--bd--\r\n");
    raw.into_bytes()
}

async fn att_list(addr: Http, id: &str) -> serde_json::Value {
    let result = tool(
        addr,
        "swarmail_list_attachments",
        &format!(r#"{{"id":"{id}"}}"#),
    )
    .await;
    assert_eq!(result["isError"], false, "{result}");
    text(&result)
}

async fn att_get(addr: Http, id: &str, index: u64) -> serde_json::Value {
    tool(
        addr,
        "swarmail_get_attachment",
        &format!(r#"{{"id":"{id}","index":{index}}}"#),
    )
    .await
}

fn att_bytes(fetched: &serde_json::Value) -> Vec<u8> {
    use base64::Engine;
    let body = text(fetched);
    let content = body["content"].as_str().unwrap();
    match body["encoding"].as_str().unwrap() {
        "utf-8" => content.as_bytes().to_vec(),
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(content)
            .unwrap(),
        other => panic!("encoding {other}"),
    }
}

/// AC1/AC2/AC7: list and fetch a real SMTP-delivered mail the moment its
/// `250` arrives — types lowercased, sizes decoded, hashes proven by the
/// known vectors, a bare calendar body and an attached message listed, the
/// NUL-carrying text part answered as base64.
#[tokio::test]
async fn attachments_are_listed_and_fetched_right_after_the_250() {
    let srv = start().await;
    let invite = "BEGIN:VCALENDAR\r\nBEGIN:VTIMEZONE\r\nDTSTART:19700101T000000\r\nEND:VTIMEZONE\r\n\
                  BEGIN:VEVENT\r\nUID:evt-9@x.io\r\nDTSTART;TZID=Europe/Berlin:20261102T150000\r\n\
                  DTEND;TZID=Europe/Berlin:20261102T160000\r\nSUMMARY:Plan\\, Q4\r\n\
                  END:VEVENT\r\nEND:VCALENDAR";
    let inner = "From: i@x.io\r\nSubject: inner\r\nMIME-Version: 1.0\r\n\
                 Content-Type: multipart/mixed; boundary=\"in\"\r\n\r\n--in\r\n\
                 Content-Type: text/plain\r\n\r\nx\r\n--in\r\n\
                 Content-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"deep.pdf\"\r\n\r\npdf\r\n--in--";
    let parts = [
        att_binary("abc.bin", b"abc"),
        att_binary("empty.bin", b""),
        (
            "Content-Type: Text/CSV; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\
             Content-Disposition: attachment; filename=\"a.csv\""
                .into(),
            "caf=C3=A9,1=\r\n2".into(),
        ),
        (
            "Content-Type: text/plain\r\nContent-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"nul.txt\""
                .into(),
            wrapped_base64(b"a\0b"),
        ),
        ("Content-Type: text/calendar; method=REQUEST".into(), invite.into()),
        ("Content-Type: message/rfc822".into(), inner.into()),
    ];
    let id = common::smtp_raw(srv.smtp_addr, "att", &att_mail(&parts)).await;
    // Queryable at once: no wait, no retry (decision 0001).
    let list = att_list(srv.http_addr, &id).await;
    assert_eq!(
        (&list["total"], &list["truncated"]),
        (&6.into(), &false.into())
    );
    assert_eq!(list["id"], id.as_str());
    let entries = list["attachments"].as_array().unwrap();
    let view: Vec<_> = entries
        .iter()
        .map(|e| {
            (
                e["filename"].as_str(),
                e["content_type"].as_str().unwrap(),
                e["size"].as_u64().unwrap(),
                e["stored"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        view,
        [
            (Some("abc.bin"), "application/octet-stream", 3, true),
            (Some("empty.bin"), "application/octet-stream", 0, true),
            (Some("a.csv"), "text/csv", "café,12".len() as u64, true),
            (Some("nul.txt"), "text/plain", 3, true),
            (None, "text/calendar", invite.len() as u64, true),
            (
                None,
                "message/rfc822",
                entries[5]["size"].as_u64().unwrap(),
                true
            ),
        ]
    );
    assert_eq!(entries[0]["sha256"], SHA256_ABC);
    assert_eq!(entries[1]["sha256"], SHA256_EMPTY);
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry["index"], i);
        let hash = entry["sha256"].as_str().unwrap();
        assert!(
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
    }
    assert!(entries[..4].iter().all(|e| e["ics"].is_null()));
    let ics = &entries[4]["ics"];
    assert_eq!(ics["uid"], "evt-9@x.io");
    assert_eq!(ics["dtstart"], "20261102T150000");
    assert_eq!(ics["dtstart_tzid"], "Europe/Berlin");
    assert_eq!(ics["dtend"], "20261102T160000");
    assert_eq!(ics["summary"], "Plan, Q4");
    assert!(
        entries[5]["ics"].is_null(),
        "an attached message is one entry"
    );

    // Fetch: every stored part answers the listed size and hash.
    let abc = att_get(srv.http_addr, &id, 0).await;
    assert_eq!(text(&abc)["encoding"], "utf-8");
    assert_eq!(att_bytes(&abc), b"abc");
    assert_eq!(text(&abc)["sha256"], SHA256_ABC);
    assert_eq!(text(&att_get(srv.http_addr, &id, 1).await)["content"], "");
    let csv = att_get(srv.http_addr, &id, 2).await;
    assert_eq!(att_bytes(&csv), "café,12".as_bytes());
    assert_eq!(text(&csv)["encoding"], "utf-8");
    let nul = att_get(srv.http_addr, &id, 3).await;
    assert_eq!(text(&nul)["encoding"], "base64");
    assert_eq!(att_bytes(&nul), b"a\0b");
    let calendar = att_get(srv.http_addr, &id, 4).await;
    assert_eq!(att_bytes(&calendar), invite.as_bytes());
    for (i, entry) in entries.iter().enumerate() {
        let fetched = text(&att_get(srv.http_addr, &id, i as u64).await);
        assert_eq!(fetched["size"], entry["size"]);
        assert_eq!(fetched["sha256"], entry["sha256"]);
        assert_eq!(fetched["filename"], entry["filename"]);
        assert_eq!(fetched["content_type"], entry["content_type"]);
    }
    srv.stop().await;
}

#[tokio::test]
async fn attachment_refusals_are_named() {
    let srv = start().await;
    let id = common::smtp_raw(
        srv.smtp_addr,
        "att-ref",
        &att_mail(&[att_binary("a.bin", b"abc")]),
    )
    .await;
    let unknown = tool(
        srv.http_addr,
        "swarmail_list_attachments",
        r#"{"id":"nope"}"#,
    )
    .await;
    assert_eq!(unknown["isError"], true);
    assert_eq!(unknown["content"][0]["text"], "message nope not found");
    let unknown = att_get(srv.http_addr, "nope", 0).await;
    assert_eq!(unknown["content"][0]["text"], "message nope not found");
    let past = att_get(srv.http_addr, &id, 1).await;
    assert_eq!(past["isError"], true);
    assert!(
        past["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("past the list of 1")
    );
    for (arguments, message) in [
        (
            format!(r#"{{"id":"{id}","index":-1}}"#),
            "index must be a non-negative integer",
        ),
        (
            format!(r#"{{"id":"{id}","index":1.5}}"#),
            "index must be a non-negative integer",
        ),
        (
            format!(r#"{{"id":"{id}"}}"#),
            "index must be a non-negative integer",
        ),
        (r#"{"index":0}"#.to_string(), "missing id"),
    ] {
        let params = format!(r#"{{"name":"swarmail_get_attachment","arguments":{arguments}}}"#);
        let (status, body) = rpc(srv.http_addr, "tools/call", &params).await;
        assert_eq!(
            (status, &body["error"]["code"]),
            (200, &(-32602).into()),
            "{arguments}"
        );
        assert_eq!(body["error"]["message"], message);
    }
    srv.stop().await;
}

/// AC3 on the wire: the budget edges, first-fit and the count cap, each
/// through a real SMTP delivery.
#[tokio::test]
async fn attachment_caps_hold_on_the_wire() {
    let srv = start().await;
    let addr = srv.http_addr;
    let stored_flags = |list: &serde_json::Value| -> Vec<bool> {
        list["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["stored"].as_bool().unwrap())
            .collect()
    };
    let refusal =
        |fetched: &serde_json::Value| fetched["content"][0]["text"].as_str().unwrap().to_string();

    // Exactly 1 MiB is stored and fetched whole; one octet more is listed
    // with its size and hash but refused, naming size, budget and route.
    let whole = vec![0xA5u8; 1_048_576];
    let id = common::smtp_raw(
        srv.smtp_addr,
        "cap-1",
        &att_mail(&[att_binary("whole.bin", &whole)]),
    )
    .await;
    assert_eq!(stored_flags(&att_list(addr, &id).await), [true]);
    let fetched = att_get(addr, &id, 0).await;
    assert_eq!(text(&fetched)["encoding"], "base64");
    assert_eq!(att_bytes(&fetched), whole);
    let over = vec![0xA5u8; 1_048_577];
    let id = common::smtp_raw(
        srv.smtp_addr,
        "cap-2",
        &att_mail(&[att_binary("over.bin", &over)]),
    )
    .await;
    let list = att_list(addr, &id).await;
    assert_eq!(stored_flags(&list), [false]);
    assert_eq!(list["attachments"][0]["size"], 1_048_577);
    assert_eq!(list["attachments"][0]["sha256"].as_str().unwrap().len(), 64);
    let fetched = att_get(addr, &id, 0).await;
    assert_eq!(fetched["isError"], true);
    let sentence = refusal(&fetched);
    for needle in [
        "1048577 bytes",
        "1048576 bytes (1 MiB)",
        &format!("GET /api/v1/messages/{id}/raw"),
    ] {
        assert!(sentence.contains(needle), "{sentence}");
    }
    // The raw route it names still serves the whole mail.
    let (status, raw) = http_bytes(addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(status, 200);
    assert!(raw.len() > 1_048_577);

    // Two half-MiB parts fit; a following single octet does not.
    let half = vec![1u8; 524_288];
    let id = common::smtp_raw(
        srv.smtp_addr,
        "cap-3",
        &att_mail(&[
            att_binary("a", &half),
            att_binary("b", &half),
            att_binary("c", b"x"),
        ]),
    )
    .await;
    assert_eq!(
        stored_flags(&att_list(addr, &id).await),
        [true, true, false]
    );

    // A 1 MiB part before a 300-octet calendar: the calendar is unstored
    // but its fields are present.
    let mut ics =
        String::from("BEGIN:VEVENT\r\nUID:tail@x.io\r\nSUMMARY:After the blob\r\nEND:VEVENT\r\n");
    ics.push_str(&"X".repeat(300 - ics.len()));
    let id = common::smtp_raw(
        srv.smtp_addr,
        "cap-4",
        &att_mail(&[
            att_binary("blob", &whole),
            ("Content-Type: text/calendar".into(), ics.clone()),
        ]),
    )
    .await;
    let list = att_list(addr, &id).await;
    assert_eq!(stored_flags(&list), [true, false]);
    assert_eq!(list["attachments"][1]["size"], 300);
    assert_eq!(list["attachments"][1]["ics"]["uid"], "tail@x.io");
    assert_eq!(list["attachments"][1]["ics"]["summary"], "After the blob");
    assert_eq!(att_get(addr, &id, 1).await["isError"], true);

    // 250 parts list in full; 251 list 250 with the exact total.
    let one = |i: usize| att_binary(&format!("f{i}.bin"), b"x");
    let exactly: Vec<_> = (0..250).map(one).collect();
    let id = common::smtp_raw(srv.smtp_addr, "cap-5", &att_mail(&exactly)).await;
    let list = att_list(addr, &id).await;
    assert_eq!(
        (&list["total"], &list["truncated"]),
        (&250.into(), &false.into())
    );
    assert_eq!(list["attachments"].as_array().unwrap().len(), 250);
    let over: Vec<_> = (0..251).map(one).collect();
    let id = common::smtp_raw(srv.smtp_addr, "cap-6", &att_mail(&over)).await;
    let list = att_list(addr, &id).await;
    assert_eq!(
        (&list["total"], &list["truncated"]),
        (&251.into(), &true.into())
    );
    assert_eq!(list["attachments"].as_array().unwrap().len(), 250);
    assert_eq!(list["attachments"][249]["filename"], "f249.bin");

    // A 1025-byte filename is cut at a character boundary, at most 1024.
    let mut name = "a".to_string();
    name.push_str(&"é".repeat(512));
    assert_eq!(name.len(), 1025);
    let id = common::smtp_raw(
        srv.smtp_addr,
        "cap-7",
        &att_mail(&[att_binary(&name, b"x")]),
    )
    .await;
    let cut = att_list(addr, &id).await["attachments"][0]["filename"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(cut.len() <= 1024 && name.starts_with(&cut), "{}", cut.len());
    assert_eq!(cut.len(), 1023);
    srv.stop().await;
}

/// ICS at ingest (AC5), through the wire: folded lines, nested components,
/// a TZID, a Z suffix, an escaped summary and two events — and the other
/// calendar shapes, a calendar that does not parse and a hostile one.
#[tokio::test]
async fn ics_fields_are_read_at_ingest() {
    let srv = start().await;
    let addr = srv.http_addr;
    let calendar = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\n\
        BEGIN:VTIMEZONE\r\nTZID:Europe/Berlin\r\nBEGIN:STANDARD\r\nDTSTART:19701025T030000\r\nEND:STANDARD\r\nEND:VTIMEZONE\r\n\
        BEGIN:VEVENT\r\nUID:first@x.io\r\nDTSTART;TZID=Europe/Berlin:20261102T150000\r\n\
        DTEND:20261102T160000Z\r\nSUMMARY:Quarterly\\, board\\n\r\n  review\r\n\
        BEGIN:VALARM\r\nSUMMARY:alarm\r\nEND:VALARM\r\nEND:VEVENT\r\n\
        BEGIN:VEVENT\r\nUID:second@x.io\r\nSUMMARY:no\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let id = common::smtp_raw(
        srv.smtp_addr,
        "ics",
        &att_mail(&[
            ("Content-Type: text/calendar\r\nContent-Disposition: attachment; filename=\"invite.ics\"".into(), calendar.into()),
            ("Content-Type: application/ics".into(), "BEGIN:VEVENT\r\nUID:ics-type\r\nEND:VEVENT\r\n".into()),
            ("Content-Type: application/octet-stream; name=\"x.ICS\"".into(), "BEGIN:VEVENT\r\nUID:by-name\r\nEND:VEVENT\r\n".into()),
            ("Content-Type: text/calendar".into(), "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".into()),
            ("Content-Type: text/calendar".into(), "not a calendar at all".into()),
            att_binary("bad.ics", b"BEGIN:VEVENT\r\n\xff\xfe\r\nUID:bad-utf8\r\nEND:VEVENT\r\n"),
        ]),
    )
    .await;
    let list = att_list(addr, &id).await;
    let entries = list["attachments"].as_array().unwrap();
    let first = &entries[0]["ics"];
    assert_eq!(first["uid"], "first@x.io");
    assert_eq!(first["dtstart"], "20261102T150000");
    assert_eq!(first["dtstart_tzid"], "Europe/Berlin");
    assert_eq!(first["dtend"], "20261102T160000Z");
    assert_eq!(first["dtend_tzid"], serde_json::Value::Null);
    assert_eq!(first["summary"], "Quarterly, board\n review");
    assert_eq!(entries[0]["filename"], "invite.ics");
    assert_eq!(entries[1]["ics"]["uid"], "ics-type");
    assert_eq!(entries[2]["ics"]["uid"], "by-name");
    assert!(
        entries[3]["ics"].is_null() && entries[4]["ics"].is_null() && entries[5]["ics"].is_null()
    );

    // A calendar over 1 MiB yields no fields, and a malformed one is still
    // accepted and queryable.
    let mut huge = String::from("BEGIN:VEVENT\r\nUID:huge\r\nEND:VEVENT\r\n");
    huge.push_str(&"X-FILL:".repeat(150_000));
    assert!(huge.len() > 1_048_576);
    let id = common::smtp_raw(
        srv.smtp_addr,
        "ics-2",
        &att_mail(&[
            ("Content-Type: text/calendar".into(), huge),
            (
                "Content-Type: text/calendar".into(),
                "BEGIN:VEVENT\r\nBEGIN:VEVENT\r\n::\r\n".into(),
            ),
        ]),
    )
    .await;
    let list = att_list(addr, &id).await;
    assert!(list["attachments"][0]["ics"].is_null());
    assert_eq!(list["attachments"][0]["stored"], false, "over the budget");
    assert_eq!(
        list["attachments"][1]["ics"]["uid"],
        serde_json::Value::Null
    );
    srv.stop().await;
}

/// AC4: hostile mail under the 50 MiB DATA cap. Each is accepted with `250`,
/// is listable at once under the caps, and retains at most 1 MiB of
/// attachment bytes — read from the data file's `attachment_bytes` column,
/// not inferred. The parser's transient allocation while it reads the mail
/// is the existing, 50 MiB-bounded behaviour (decision 0022); what this
/// pins is what the feature keeps.
#[tokio::test]
async fn hostile_attachments_cannot_grow_what_is_retained() {
    let dir = tempfile::TempDir::with_prefix("swarmail-att-hostile-").unwrap();
    let db = dir.path().join("mail.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let addr = srv.http_addr;
    let retained = |id: &str| -> usize {
        let conn =
            rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        conn.query_row(
            "SELECT length(attachment_bytes) FROM emails WHERE id = ?1",
            [id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap() as usize
    };

    // Fifty thousand minimal parts: the list stays at 250, the total is
    // exact, and 250 one-octet parts are all that is kept.
    let mut raw = String::from(
        "From: app@x.io\r\nTo: rcpt@x.io\r\nSubject: many\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n",
    );
    for _ in 0..50_000 {
        raw.push_str("--bd\r\nContent-Disposition: attachment\r\n\r\nx\r\n");
    }
    raw.push_str("--bd--\r\n");
    let id = common::smtp_raw(srv.smtp_addr, "hostile-many", raw.as_bytes()).await;
    drop(raw);
    let list = att_list(addr, &id).await;
    assert_eq!(
        (&list["total"], &list["truncated"]),
        (&50_000.into(), &true.into())
    );
    assert_eq!(list["attachments"].as_array().unwrap().len(), 250);
    assert_eq!(retained(&id), 250);

    // One part of 40 MiB of base64 (30 MiB decoded): listed with its true
    // size and hash, nothing kept.
    let decoded = vec![0u8; 30 * 1024 * 1024];
    let (headers, body) = att_binary("huge.bin", &decoded);
    assert!(body.len() > 40 * 1024 * 1024 && body.len() < 50 * 1024 * 1024);
    let id = common::smtp_raw(srv.smtp_addr, "hostile-big", &att_mail(&[(headers, body)])).await;
    let list = att_list(addr, &id).await;
    assert_eq!(list["attachments"][0]["size"], 30 * 1024 * 1024);
    assert_eq!(list["attachments"][0]["stored"], false);
    assert_eq!(retained(&id), 0);
    assert_eq!(att_get(addr, &id, 0).await["isError"], true);

    // A mix that tries to exceed the budget in aggregate: three 400 KiB
    // parts keep two, never more than 1 MiB in sum.
    let chunk = vec![3u8; 400 * 1024];
    let id = common::smtp_raw(
        srv.smtp_addr,
        "hostile-sum",
        &att_mail(&[
            att_binary("a", &chunk),
            att_binary("b", &chunk),
            att_binary("c", &chunk),
        ]),
    )
    .await;
    assert_eq!(retained(&id), 2 * 400 * 1024);
    assert!(retained(&id) <= 1_048_576);
    srv.stop().await;
}

/// AC7: a mail that trips the per-inbox cap still evicts the oldest, and
/// its attachment columns go with the row.
#[tokio::test]
async fn pruning_takes_the_attachment_columns_with_the_row() {
    let dir = tempfile::TempDir::with_prefix("swarmail-att-prune-").unwrap();
    let db = dir.path().join("mail.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c.max_per_inbox = 1;
        c
    })
    .await;
    let first = common::smtp_raw(
        srv.smtp_addr,
        "prune",
        &att_mail(&[att_binary("old.bin", b"old")]),
    )
    .await;
    let second = common::smtp_raw(
        srv.smtp_addr,
        "prune",
        &att_mail(&[att_binary("new.bin", b"new")]),
    )
    .await;
    let gone = tool(
        srv.http_addr,
        "swarmail_list_attachments",
        &format!(r#"{{"id":"{first}"}}"#),
    )
    .await;
    assert_eq!(gone["isError"], true);
    assert_eq!(
        att_list(srv.http_addr, &second).await["attachments"][0]["filename"],
        "new.bin"
    );
    let conn =
        rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let rows: Vec<(String, Vec<u8>)> = conn
        .prepare("SELECT id, attachment_bytes FROM emails")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows, [(second, b"new".to_vec())]);
    srv.stop().await;
}
