//! The report contract through real SMTP, REST, MCP and server-rendered UI.
mod common;
use common::*;
use serde_json::{Value, json};

async fn report(addr: std::net::SocketAddr, id: &str) -> (u16, Value) {
    http_json(
        addr,
        "GET",
        &format!("/api/v1/messages/{id}/deliverability"),
        None,
    )
    .await
}
async fn rpc(addr: std::net::SocketAddr, method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    let (status, reply) = http_json(addr, "POST", "/mcp", Some(&body.to_string())).await;
    assert_eq!(status, 200);
    reply
}
fn result_text(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert!(reply["result"].get("structuredContent").is_none());
    assert_eq!(reply["result"]["content"].as_array().unwrap().len(), 1);
    assert_eq!(reply["result"]["content"][0]["type"], "text");
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

/// The seven bulk findings of a single-part mail without Message-ID, Date,
/// one-click list or text/plain, written independently of any response.
fn bulk_findings() -> Vec<Value> {
    let mut findings = Vec::new();
    let sources =
        "https://support.google.com/mail/answer/81126 https://senders.yahooinc.com/best-practices/";
    for (rule, header, hint, rfc) in [
        (
            "one_click",
            Some("list-unsubscribe"),
            "Provide one HTTPS List-Unsubscribe list and one exact List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check does not verify DKIM header coverage or endpoint operation.",
            "8058#section-3",
        ),
        (
            "message_id",
            Some("message-id"),
            "Provide exactly one Message-ID using the supported current RFC 5322 syntax; obsolete-only forms are unsupported by this checklist.",
            "5322#section-3.6.4",
        ),
        (
            "date",
            Some("date"),
            "Provide exactly one calendar-valid Date using current RFC 5322 syntax and a numeric zone; obsolete-only forms are unsupported by this checklist.",
            "5322#section-3.6.1",
        ),
        (
            "plain_text",
            None,
            "Include a sender-supplied text/plain body. This is checklist advice, not proof of equivalent content or an unconditional provider mandate.",
            "2046#section-5.1.4",
        ),
        (
            "dmarc_presence.not_evaluated",
            Some("from"),
            "Not evaluated: trusted applicable DMARC policy evidence is unavailable.",
            "7489#section-6.1",
        ),
        (
            "from_alignment.not_evaluated",
            Some("from"),
            "Not evaluated: trusted From identity or DKIM/SPF alignment evidence is incomplete.",
            "7489#section-3.1",
        ),
        (
            "authenticated_sender.not_evaluated",
            Some("from"),
            "Not evaluated: trusted From identity or passing-and-aligned DKIM/SPF evidence is incomplete.",
            "7489#section-3.1",
        ),
    ] {
        findings.push(json!({"rule_id":format!("bulk.{rule}.v1"), "severity": if header.is_none() { "warning" } else { "error" },
            "location": match header { Some(name) => json!({"kind":"header","name":name,"occurrence":null}), None => json!({"kind":"mime_part","path":[]}) },
            "fix_hint":format!("{hint} Sources: https://www.rfc-editor.org/rfc/rfc{rfc} {sources}")}));
    }
    findings
}

#[tokio::test]
async fn report_rest_schema_and_refusals() {
    let srv = start().await;
    let raw = b"From: sender@x.io\r\nSubject: report <&>\r\nX-Swarmail-Demo: warning\r\n\r\ncode 424242 https://x.io/path\r\n";
    let id = smtp_raw(srv.smtp_addr, "report", raw).await;
    let before = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    let (status, stored) = http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(status, 200);
    assert_eq!(stored, raw);
    let (status, first) = report(srv.http_addr, &id).await;
    assert_eq!(status, 200);
    assert_eq!(first["rule_set_id"], "swarmail-html/3");
    assert_eq!(first["findings"].as_array().unwrap().len(), 7);
    assert_eq!(
        first["findings"][6],
        json!({"rule_id":"demo.marker.v1","severity":"warning",
        "location":{"kind":"header","name":"x-swarmail-demo","occurrence":0},
        "fix_hint":"Remove the X-Swarmail-Demo header."})
    );
    assert_eq!(report(srv.http_addr, &id).await.1, first);
    assert_eq!(
        http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None
        )
        .await,
        before
    );
    for suffix in ["?oops=1", "?id=x", "?%6cimit=1"] {
        assert_eq!(
            http_json(
                srv.http_addr,
                "GET",
                &format!("/api/v1/messages/missing/deliverability{suffix}"),
                None
            )
            .await
            .0,
            400
        );
    }
    assert_eq!(
        report(srv.http_addr, "missing").await,
        (404, json!({"error":"message not found"}))
    );
    let (_, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    for text in [
        "swarmail-html/3",
        "warning",
        "demo.marker.v1",
        "x-swarmail-demo",
        "Remove the X-Swarmail-Demo header.",
        "report &lt;&amp;&gt;",
    ] {
        assert!(page.contains(text), "{text}");
    }
    http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(report(srv.http_addr, &id).await.0, 404);
    assert_eq!(
        http_get_text(srv.http_addr, &format!("/ui/message/{id}"))
            .await
            .0,
        404
    );
    srv.stop().await;
}

#[tokio::test]
async fn report_mcp_text_contract() {
    let srv = start().await;
    let init = rpc(srv.http_addr, "initialize", json!({})).await;
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    let tools = rpc(srv.http_addr, "tools/list", json!({})).await;
    let desc = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "swarmail_get_deliverability_report")
        .unwrap();
    assert!(desc.get("outputSchema").is_none());
    assert_eq!(desc["inputSchema"]["required"], json!(["id"]));
    assert_eq!(desc["inputSchema"]["additionalProperties"], false);
    assert_eq!(desc["inputSchema"]["properties"]["id"]["type"], "string");
    for raw in [
        &b"Subject: empty\r\n\r\nbody\r\n"[..],
        b"X-Swarmail-Demo: error\r\n\r\nbody\r\n",
        b"Content-Type: text/html\r\n\r\n<p style='display:flex'>Hello</p><img>\r\n",
    ] {
        let id = smtp_raw(srv.smtp_addr, "mcp-report", raw).await;
        let reply = rpc(
            srv.http_addr,
            "tools/call",
            json!({"name":"swarmail_get_deliverability_report","arguments":{"id":id}}),
        )
        .await;
        assert_eq!(result_text(&reply), report(srv.http_addr, &id).await.1);
    }
    for args in [
        json!({}),
        json!({"id":null}),
        json!({"id":4}),
        json!({"id":"missing","other":true}),
        json!([]),
    ] {
        let reply = rpc(
            srv.http_addr,
            "tools/call",
            json!({"name":"swarmail_get_deliverability_report","arguments":args}),
        )
        .await;
        assert_eq!(reply["error"]["code"], -32602, "{reply}");
    }
    let missing = rpc(
        srv.http_addr,
        "tools/call",
        json!({"name":"swarmail_get_deliverability_report","arguments":{"id":"absent"}}),
    )
    .await;
    assert_eq!(missing["result"]["isError"], true);
    assert_eq!(
        missing["result"]["content"][0]["text"],
        "message absent not found"
    );
    srv.stop().await;
}

#[tokio::test]
async fn report_threshold_exact_count_and_min_one() {
    let srv = start().await;
    let severities = ["info", "warning", "error"];
    for severity in severities {
        let inbox = format!("threshold-{severity}");
        smtp_raw(
            srv.smtp_addr,
            &inbox,
            format!("X-Swarmail-Demo: {severity}\r\n\r\nbody\r\n").as_bytes(),
        )
        .await;
        for threshold in severities {
            let qualifying = 0;
            let path = format!(
                "/api/v1/inboxes/{inbox}/assert?count={qualifying}&no_findings_at_or_above={threshold}&limit=999"
            );
            let (status, value) = http_json(srv.http_addr, "GET", &path, None).await;
            assert_eq!(status, 200, "{path}: {value}");
            assert_eq!(value["matched"], qualifying);
            let path = format!(
                "/api/v1/inboxes/{inbox}/await?timeout_ms=0&no_findings_at_or_above={threshold}"
            );
            let (status, value) = http_json(srv.http_addr, "GET", &path, None).await;
            assert_eq!(status, if qualifying == 1 { 200 } else { 408 });
            assert_eq!(value["matched"], qualifying);
            assert_eq!(value["total"], 1);
            let reply = rpc(
                srv.http_addr,
                "tools/call",
                json!({"name":"swarmail_wait_for_email","arguments":{
                "inbox":inbox,"timeout_ms":0,"no_findings_at_or_above":threshold}}),
            )
            .await;
            let value = result_text(&reply);
            assert_eq!(value["matched"], qualifying);
            assert_eq!(
                value["timed_out"],
                if qualifying == 1 {
                    Value::Null
                } else {
                    json!(true)
                }
            );
        }
    }
    for (body, expected) in [
        ("Subject: clean older\r\n\r\nbody\r\n", false),
        ("Subject: clean newer\r\n\r\nbody\r\n", false),
        (
            "Subject: dirty newest\r\nX-Swarmail-Demo: error\r\n\r\nbody\r\n",
            false,
        ),
    ] {
        let id = smtp_raw(srv.smtp_addr, "mix", body.as_bytes()).await;
        assert_eq!(
            report(srv.http_addr, &id).await.1["findings"]
                .as_array()
                .unwrap()
                .is_empty(),
            expected
        );
    }
    let path =
        "/api/v1/inboxes/mix/await?count=2&limit=1&timeout_ms=0&no_findings_at_or_above=warning";
    let (status, value) = http_json(srv.http_addr, "GET", path, None).await;
    assert_eq!(status, 408);
    assert_eq!(value["matched"], 0);
    assert_eq!(value["total"], 3);
    let (status, value) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/mix/assert?count=1&no_findings_at_or_above=warning",
        None,
    )
    .await;
    assert_eq!(status, 409);
    assert_eq!(value["matched"], 0);
    assert_eq!(
        http_json(
            srv.http_addr,
            "GET",
            "/api/v1/inboxes/mix/assert?count=0&subject=dirty&no_findings_at_or_above=warning",
            None
        )
        .await
        .0,
        200
    );
    assert_eq!(http_json(srv.http_addr, "GET", "/api/v1/inboxes/mix/await?count=0&timeout_ms=0&subject=clean&no_findings_at_or_above=info", None).await.0, 408);
    for op in ["assert", "await", "messages", "count"] {
        for invalid in ["", "Warning", "bogus", "null"] {
            let path = format!("/api/v1/inboxes/invalid/{op}?no_findings_at_or_above={invalid}");
            assert_eq!(
                http_json(srv.http_addr, "GET", &path, None).await.0,
                400,
                "{path}"
            );
        }
    }
    for op in ["messages", "count"] {
        assert_eq!(
            http_json(
                srv.http_addr,
                "GET",
                &format!("/api/v1/inboxes/mix/{op}?no_findings_at_or_above=info"),
                None
            )
            .await
            .0,
            400
        );
    }
    for invalid in [
        json!(""),
        json!("Warning"),
        json!(null),
        json!(4),
        json!(true),
    ] {
        let reply = rpc(srv.http_addr, "tools/call", json!({"name":"swarmail_wait_for_email","arguments":{"no_findings_at_or_above":invalid}})).await;
        assert_eq!(reply["error"]["code"], -32602);
    }
    let (status, value) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/empty/await?count=0&timeout_ms=0&no_findings_at_or_above=error",
        None,
    )
    .await;
    assert_eq!(status, 408);
    assert_eq!(value["matched"], 0);
    srv.stop().await;
}

#[tokio::test]
async fn report_failure_is_never_clean_and_ui_keeps_message() {
    let srv = start().await;
    for (inbox, raw) in [
        (
            "failure-html",
            "Content-Type: text/html\r\n\r\n<video><script>\r\n".to_string(),
        ),
        (
            "failure-headers",
            format!("Subject: {}\r\n\r\nbody\r\n", "x".repeat(65_536)),
        ),
    ] {
        let id = smtp_raw(srv.smtp_addr, inbox, raw.as_bytes()).await;
        smtp_raw(
            srv.smtp_addr,
            inbox,
            b"Subject: clean newest\r\n\r\nbody\r\n",
        )
        .await;
        for _ in 0..2 {
            assert_eq!(
                report(srv.http_addr, &id).await,
                (503, json!({"error":"deliverability analysis failed"}))
            );
        }
        for op in ["assert?count=1", "await?count=2"] {
            let path = format!("/api/v1/inboxes/{inbox}/{op}&no_findings_at_or_above=error");
            assert_eq!(
                http_json(srv.http_addr, "GET", &path, None).await,
                (503, json!({"error":"deliverability analysis failed"}))
            );
        }
        for (name, args) in [
            ("swarmail_get_deliverability_report", json!({"id":id})),
            (
                "swarmail_wait_for_email",
                json!({"inbox":inbox,"count":2,"no_findings_at_or_above":"error"}),
            ),
        ] {
            let reply = rpc(
                srv.http_addr,
                "tools/call",
                json!({"name":name,"arguments":args}),
            )
            .await;
            assert_eq!(reply["result"]["isError"], true);
            assert_eq!(
                reply["result"]["content"][0]["text"],
                "deliverability analysis failed"
            );
        }
        let (status, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
        assert_eq!(status, 200);
        assert!(page.contains("deliverability analysis failed"));
        let (status, list) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{inbox}/messages"),
            None,
        )
        .await;
        assert_eq!(status, 200);
        let clean = list["emails"][0]["id"].as_str().unwrap();
        let (status, page) = http_get_text(srv.http_addr, &format!("/ui/message/{clean}")).await;
        assert_eq!(status, 200);
        assert!(page.contains("bulk.from_alignment.not_evaluated.v1"));
    }
    srv.stop().await;
}

#[tokio::test]
async fn report_restart_recomputes_same_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mail.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let raw = b"X-Swarmail-Demo: warning\r\nSubject: persisted\r\nContent-Type: text/html\r\n\r\n<p style=\"display:flex\">code 424242 https://x.io</p><img>\r\n";
    let id = smtp_raw(srv.smtp_addr, "restart", raw).await;
    let original = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    let expected = report(srv.http_addr, &id).await;
    srv.stop().await;
    let srv = start_with(|mut c| {
        c.data_file = Some(db);
        c
    })
    .await;
    assert_eq!(
        http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None
        )
        .await,
        original
    );
    assert_eq!(report(srv.http_addr, &id).await, expected);
    assert_eq!(
        http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw"))
            .await
            .1,
        raw
    );
    srv.stop().await;
}

#[tokio::test]
async fn report_lookup_cannot_resurrect_pruned_or_cleared_mail() {
    let srv = start_with(|mut c| {
        c.max_per_inbox = 1;
        c
    })
    .await;
    let first = smtp_raw(
        srv.smtp_addr,
        "cap",
        b"X-Swarmail-Demo: error\r\n\r\nbody\r\n",
    )
    .await;
    assert_eq!(report(srv.http_addr, &first).await.0, 200);
    let next = smtp_raw(srv.smtp_addr, "cap", b"\r\nbody\r\n").await;
    assert_eq!(report(srv.http_addr, &first).await.0, 404);
    assert_eq!(
        report(srv.http_addr, &next).await.1["findings"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    http_json(
        srv.http_addr,
        "DELETE",
        "/api/v1/inboxes/cap/messages",
        None,
    )
    .await;
    assert_eq!(report(srv.http_addr, &next).await.0, 404);
    srv.stop().await;
}

#[tokio::test]
async fn report_served_docs_match_contract() {
    let srv = start().await;
    let (status, api) = http_json(srv.http_addr, "GET", "/openapi.json", None).await;
    assert_eq!(status, 200);
    let route = &api["paths"]["/api/v1/messages/{id}/deliverability"]["get"];
    for status in ["200", "400", "403", "404", "503"] {
        assert!(route["responses"].get(status).is_some());
    }
    assert_eq!(route["parameters"].as_array().unwrap().len(), 1);
    assert_eq!(route["parameters"][0]["name"], "id");
    let schemas = &api["components"]["schemas"];
    assert_eq!(
        schemas["DeliverabilitySeverity"]["enum"],
        json!(["info", "warning", "error"])
    );
    assert_eq!(
        schemas["DeliverabilityReport"]["properties"]["findings"]["maxItems"],
        256
    );
    assert_eq!(
        schemas["DeliverabilityReport"]["required"],
        json!(["rule_set_id", "findings"])
    );
    assert_eq!(
        schemas["DeliverabilityFinding"]["required"],
        json!(["rule_id", "severity", "location", "fix_hint"])
    );
    for (index, (kind, fields)) in [
        ("header", vec!["kind", "name", "occurrence"]),
        ("mime_part", vec!["kind", "path"]),
        ("link", vec!["kind", "part_path", "byte_offset"]),
    ]
    .into_iter()
    .enumerate()
    {
        let variant = &schemas["DeliverabilityLocation"]["oneOf"][index];
        assert_eq!(variant["properties"]["kind"]["const"], kind);
        assert_eq!(variant["required"], json!(fields));
        assert_eq!(variant["additionalProperties"], false);
    }
    for op in ["assert", "await"] {
        let op = &api["paths"][format!("/api/v1/inboxes/{{inbox}}/{op}")]["get"];
        assert!(op["responses"].get("503").is_some());
        let predicate = op["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "no_findings_at_or_above")
            .unwrap();
        assert_eq!(
            predicate["schema"]["$ref"],
            "#/components/schemas/DeliverabilitySeverity"
        );
    }
    let (status, llms) = http_get_text(srv.http_addr, "/llms.txt").await;
    assert_eq!(status, 200);
    for expected in [
        "swarmail_get_deliverability_report",
        "no_findings_at_or_above",
        "65536",
        "256",
        "128",
        "Empty findings do not imply deliverability",
        "structuredContent under a closed outputSchema",
        "historical",
        "JSON in one text block",
        "swarmail-html/3",
        "104448",
        "classic Outlook",
        "conditional comments",
        "4 MiB",
        "Can I email",
    ] {
        assert!(llms.contains(expected), "{expected}");
    }
    srv.stop().await;
}

#[tokio::test]
async fn html_findings_share_one_report_and_predicate_contract() {
    let srv = start().await;
    let body = format!(
        "<style>.x{{position:absolute;display:flex}}</style><video></video><img src='https://pixel.test/p' width=1 height=0>{}",
        " ".repeat(104449)
    );
    let raw = format!(
        "Subject: HTML <&>\r\nX-Swarmail-Demo: warning\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{body}\r\n"
    );
    let id = smtp_raw(srv.smtp_addr, "html-contract", raw.as_bytes()).await;
    // The 250 has already made this same mail and its ingest extraction queryable.
    let before = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(before.0, 200);
    let (status, first) = report(srv.http_addr, &id).await;
    assert_eq!(status, 200);
    let catalogue: Value =
        serde_json::from_str(include_str!("../src/deliverability/html-rules.json")).unwrap();
    let mut expected: Vec<Value> = catalogue["rules"].as_array().unwrap().iter().map(|r| json!({
        "rule_id":r["id"],"severity":r["severity"],"location":{"kind":"mime_part","path":[]},"fix_hint":r["fix_hint"]
    })).collect();
    expected.sort_by_key(|r| r["rule_id"].as_str().unwrap().to_string());
    expected.extend(bulk_findings());
    expected.push(json!({"rule_id":"demo.marker.v1","severity":"warning","location":{"kind":"header","name":"x-swarmail-demo","occurrence":0},"fix_hint":"Remove the X-Swarmail-Demo header."}));
    expected.sort_by_key(|r| r["rule_id"].as_str().unwrap().to_string());
    assert_eq!(expected.len(), 15);
    assert_eq!(
        first,
        json!({"rule_set_id":"swarmail-html/3","findings":expected})
    );
    assert_eq!(report(srv.http_addr, &id).await.1, first);
    let reply = rpc(
        srv.http_addr,
        "tools/call",
        json!({"name":"swarmail_get_deliverability_report","arguments":{"id":id}}),
    )
    .await;
    assert_eq!(result_text(&reply), first);
    let (_, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    assert!(page.contains("HTML &lt;&amp;&gt;"));
    for finding in first["findings"].as_array().unwrap() {
        assert!(page.contains(finding["rule_id"].as_str().unwrap()));
        assert!(page.contains(finding["fix_hint"].as_str().unwrap()));
    }
    // The bulk checklist emits error findings for this fixture (no Message-ID,
    // Date, one-click list or trusted evidence), so it qualifies at no threshold.
    for (threshold, count, assert_status, await_status) in
        [("warning", 0, 200, 408), ("error", 1, 409, 408)]
    {
        let path = format!(
            "/api/v1/inboxes/html-contract/assert?count={count}&no_findings_at_or_above={threshold}"
        );
        let (status, _r) = http_json(srv.http_addr, "GET", &path, None).await;
        assert_eq!(status, assert_status);
        let path = format!(
            "/api/v1/inboxes/html-contract/await?count=0&timeout_ms=0&no_findings_at_or_above={threshold}"
        );
        assert_eq!(
            http_json(srv.http_addr, "GET", &path, None).await.0,
            await_status
        );
    }
    assert_eq!(
        http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None
        )
        .await,
        before
    );
    assert_eq!(
        http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw"))
            .await
            .1,
        raw.as_bytes()
    );
    srv.stop().await;
}

#[tokio::test]
async fn empty_smtp_message_has_an_exact_composable_report() {
    let srv = start().await;
    // smtp_raw sends DATA followed immediately by ".\r\n" and asserts 250.
    let id = smtp_raw(srv.smtp_addr, "empty-report", b"").await;
    let before = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(before.0, 200);
    assert_eq!(
        http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await,
        (200, vec![])
    );
    // A zero-time qualified wait never submits cold work: before any
    // analysis it reports unavailable rather than a clean or failed scan.
    assert_eq!(
        http_json(
            srv.http_addr,
            "GET",
            "/api/v1/inboxes/empty-report/await?count=1&timeout_ms=0&no_findings_at_or_above=error",
            None
        )
        .await,
        (503, json!({"error":"deliverability analysis unavailable"}))
    );
    // The bulk collector refuses missing MIME metadata; HTML's empty-input
    // handling must not turn another collector's failure into a clean report.
    for _ in 0..2 {
        assert_eq!(
            report(srv.http_addr, &id).await,
            (503, json!({"error":"deliverability analysis failed"}))
        );
    }
    let reply = rpc(
        srv.http_addr,
        "tools/call",
        json!({"name":"swarmail_get_deliverability_report","arguments":{"id":id}}),
    )
    .await;
    assert_eq!(reply["result"]["isError"], true);
    assert_eq!(
        reply["result"]["content"],
        json!([{"type":"text","text":"deliverability analysis failed"}])
    );
    let (status, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    assert_eq!(status, 200);
    assert!(page.contains("deliverability analysis failed"));
    for threshold in ["info", "warning", "error"] {
        for count in [0, 1] {
            for operation in ["assert", "await"] {
                let path = format!(
                    "/api/v1/inboxes/empty-report/{operation}?count={count}&timeout_ms=1000&no_findings_at_or_above={threshold}"
                );
                let path = if operation == "assert" {
                    path.replace("&timeout_ms=1000", "")
                } else {
                    path
                };
                assert_eq!(
                    http_json(srv.http_addr, "GET", &path, None).await,
                    (503, json!({"error":"deliverability analysis failed"}))
                );
            }
            let reply = rpc(srv.http_addr, "tools/call", json!({"name":"swarmail_wait_for_email", "arguments":{"inbox":"empty-report","count":count,"timeout_ms":1000,"no_findings_at_or_above":threshold}})).await;
            assert_eq!(reply["result"]["isError"], true);
            assert_eq!(
                reply["result"]["content"],
                json!([{"type":"text","text":"deliverability analysis failed"}])
            );
            // Failures are never cached, so earlier evaluations leave no
            // ready report for a zero-time wait to read.
            let path = format!(
                "/api/v1/inboxes/empty-report/await?count={count}&timeout_ms=0&no_findings_at_or_above={threshold}"
            );
            assert_eq!(
                http_json(srv.http_addr, "GET", &path, None).await,
                (503, json!({"error":"deliverability analysis unavailable"}))
            );
            let reply = rpc(srv.http_addr, "tools/call", json!({"name":"swarmail_wait_for_email", "arguments":{"inbox":"empty-report","count":count,"timeout_ms":0,"no_findings_at_or_above":threshold}})).await;
            assert_eq!(reply["result"]["isError"], true);
            assert_eq!(
                reply["result"]["content"],
                json!([{"type":"text","text":"deliverability analysis unavailable"}])
            );
            // A genuinely empty inbox needs no analysis: exact zero holds,
            // exact one mismatches and the minimum-one wait times out.
            let path = format!(
                "/api/v1/inboxes/empty-report-none/assert?count={count}&no_findings_at_or_above={threshold}"
            );
            let (status, value) = http_json(srv.http_addr, "GET", &path, None).await;
            assert_eq!(status, if count == 0 { 200 } else { 409 }, "{value}");
            assert_eq!(
                (&value["matched"], &value["ok"]),
                (&json!(0), &json!(count == 0))
            );
            for timeout in [0, 50] {
                let path = format!(
                    "/api/v1/inboxes/empty-report-none/await?count={count}&timeout_ms={timeout}&no_findings_at_or_above={threshold}"
                );
                let (status, value) = http_json(srv.http_addr, "GET", &path, None).await;
                assert_eq!(status, 408, "{value}");
                assert_eq!(value["matched"], 0);
                let reply = rpc(srv.http_addr, "tools/call", json!({"name":"swarmail_wait_for_email", "arguments":{"inbox":"empty-report-none","count":count,"timeout_ms":timeout,"no_findings_at_or_above":threshold}})).await;
                let value = result_text(&reply);
                assert_eq!(
                    (&value["matched"], &value["timed_out"]),
                    (&json!(0), &json!(true))
                );
            }
        }
    }
    assert_eq!(
        http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None
        )
        .await,
        before
    );
    assert_eq!(
        http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await,
        (200, vec![])
    );
    srv.stop().await;
}

#[tokio::test]
async fn css_component_and_literal_url_pairs_survive_smtp_reports() {
    let srv = start().await;
    let catalogue: Value =
        serde_json::from_str(include_str!("../src/deliverability/html-rules.json")).unwrap();
    let rule = &catalogue["rules"][0];
    for (n, (css, warning)) in [
        ("--unused:{position:absolute};color:red", false),
        ("--unused:{position:absolute};position:absolute", true),
        ("background:url(https://x.test/*/a.png)", false),
        (
            "background:url(https://x.test/*/a.png);position:absolute",
            true,
        ),
    ]
    .iter()
    .enumerate()
    {
        for sheet in [false, true] {
            let inbox = format!("css-pair-{n}-{sheet}");
            let body = if sheet {
                format!("<style>.x{{{css}}}</style><p>Hello</p>")
            } else {
                format!("<p style='{css}'>Hello</p>")
            };
            let raw = format!("Content-Type: text/html\r\n\r\n{body}\r\n");
            let id = smtp_raw(srv.smtp_addr, &inbox, raw.as_bytes()).await;
            let before = http_json(
                srv.http_addr,
                "GET",
                &format!("/api/v1/messages/{id}"),
                None,
            )
            .await;
            assert_eq!(before.0, 200);
            assert_eq!(
                http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await,
                (200, raw.as_bytes().to_vec())
            );
            // The whole combined report is fixed here, not read back from a
            // response: a common wrong aggregate must fail every adapter.
            let mut findings = bulk_findings();
            if *warning {
                assert_eq!(rule["id"], "html.css.position.v1");
                assert_eq!(rule["severity"], "warning");
                findings.push(json!({"rule_id":"html.css.position.v1","severity":"warning","fix_hint":rule["fix_hint"],"location":{"kind":"mime_part","path":[]}}));
            }
            findings.sort_by_key(|f| f["rule_id"].as_str().unwrap().to_string());
            let expected = json!({"rule_set_id":"swarmail-html/3","findings":findings});
            for _ in 0..2 {
                assert_eq!(report(srv.http_addr, &id).await, (200, expected.clone()));
            }
            let reply = rpc(
                srv.http_addr,
                "tools/call",
                json!({"name":"swarmail_get_deliverability_report","arguments":{"id":id}}),
            )
            .await;
            assert_eq!(result_text(&reply), expected);
            let (status, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
            assert_eq!(status, 200);
            assert_eq!(page.contains("html.css.position.v1"), *warning);
            if *warning {
                assert!(page.contains(rule["fix_hint"].as_str().unwrap()));
            }
            // Production authentication is Unknown, so the bulk errors keep
            // every pair unqualified at every threshold; HTML warning
            // discrimination is proved on HTML-only reports in its owner tests.
            for (threshold, count) in [("info", 0), ("warning", 0), ("error", 0)] {
                let path = format!(
                    "/api/v1/inboxes/{inbox}/assert?count={count}&no_findings_at_or_above={threshold}"
                );
                let (status, value) = http_json(srv.http_addr, "GET", &path, None).await;
                assert_eq!(status, 200);
                assert_eq!(value["matched"], count);
                assert_eq!(value["ok"], true);
                let path = format!(
                    "/api/v1/inboxes/{inbox}/await?timeout_ms=0&no_findings_at_or_above={threshold}"
                );
                assert_eq!(
                    http_json(srv.http_addr, "GET", &path, None).await.0,
                    if count == 1 { 200 } else { 408 }
                );
            }
            assert_eq!(
                http_json(
                    srv.http_addr,
                    "GET",
                    &format!("/api/v1/messages/{id}"),
                    None
                )
                .await,
                before
            );
        }
    }
    srv.stop().await;
}
