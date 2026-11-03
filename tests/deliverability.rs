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
    assert_eq!(first["rule_set_id"], "swarmail-html/4");
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
        "swarmail-html/4",
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
        json!(["rule_set_id", "findings", "dns"])
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
        "swarmail-html/4",
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
        json!({"rule_set_id":"swarmail-html/4","findings":expected,"dns":{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}})
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
            let expected = json!({"rule_set_id":"swarmail-html/4","findings":findings,"dns":{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}});
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

// ---------- the DNS policy (decision 0120, #253) ----------

const DEFAULT_DNS: &str =
    r#""dns":{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}"#;
/// Mixed case, a trailing dot, unsorted values and both members.
const DECLARATION: &str = r#"{"tempfail":["_DMARC.example.org."],"txt":[{"name":"S1._DomainKey.Example.COM.","values":["v=DKIM1; p=AAAA"]},{"name":"example.com","values":["v=spf1 -all","a-first"]}]}"#;
const CANONICAL: &str = r#"{"txt":[{"name":"example.com","values":["a-first","v=spf1 -all"]},{"name":"s1._domainkey.example.com","values":["v=DKIM1; p=AAAA"]}],"tempfail":["_dmarc.example.org"]}"#;
const MAIL: &[u8] = b"From: sender@example.com\r\nDKIM-Signature: v=1; a=rsa-sha256; d=example.com; s=s1; h=from; bh=x; b=y\r\nSubject: dns\r\n\r\nbody\r\n";

fn canonical() -> Value {
    serde_json::from_str(CANONICAL).unwrap()
}
async fn put_dns(srv: &swarmail::RunningServer, inbox: &str, body: &str) -> (u16, Value) {
    http_json(
        srv.http_addr,
        "PUT",
        &format!("/api/v1/inboxes/{inbox}/dns"),
        Some(body),
    )
    .await
}
async fn delete_dns(srv: &swarmail::RunningServer, inbox: &str) -> (u16, Value) {
    http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/inboxes/{inbox}/dns"),
        None,
    )
    .await
}
/// The report's exact bytes: the byte comparisons below are of the wire body.
async fn report_text(srv: &swarmail::RunningServer, id: &str) -> String {
    let (status, text) = http_get_text(
        srv.http_addr,
        &format!("/api/v1/messages/{id}/deliverability"),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    text
}
async fn dns_of(srv: &swarmail::RunningServer, id: &str) -> Value {
    report(srv.http_addr, id).await.1["dns"].clone()
}

#[tokio::test]
async fn default_dns_policy_exact_json() {
    let srv = start().await;
    let id = smtp_raw(srv.smtp_addr, "plain", MAIL).await;
    let text = report_text(&srv, &id).await;
    assert!(
        text.starts_with(r#"{"rule_set_id":"swarmail-html/4","findings":["#),
        "{text}"
    );
    assert!(text.ends_with(&format!("],{DEFAULT_DNS}}}")), "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    // The DKIM-Signature header is mail, not DNS: nothing was looked up.
    assert_eq!(body["dns"]["lookups"], json!([]));
    let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["dns", "findings", "rule_set_id"]);
    srv.stop().await;
}

#[tokio::test]
async fn inbox_dns_declarations_over_http() {
    let srv = start().await;
    let (a, b) = (
        smtp_raw(srv.smtp_addr, "a", MAIL).await,
        smtp_raw(srv.smtp_addr, "b", MAIL).await,
    );
    assert_eq!(
        put_dns(&srv, "a", DECLARATION).await,
        (
            200,
            json!({"inbox":"a","source":"inbox","policy":canonical()})
        )
    );
    assert_eq!(
        dns_of(&srv, &a).await,
        json!({"source":"inbox","policy":canonical(),"lookups":[]})
    );
    assert_eq!(dns_of(&srv, &b).await["source"], "default");
    let declared = report_text(&srv, &a).await;

    // Refusals leave the previous declaration byte-identical in the next report.
    let (status, error) =
        put_dns(&srv, "a", r#"{"txt":[{"name":"bad name","values":["x"]}]}"#).await;
    assert_eq!(status, 400);
    assert_eq!(error["error"], "txt[0].name is not a valid owner name");
    let too_big = format!(r#"{{"txt":[],"tempfail":[]}}{}"#, " ".repeat(8_169));
    assert_eq!(too_big.len(), 8_193);
    assert_eq!(put_dns(&srv, "a", &too_big).await.0, 413);
    assert_eq!(put_dns(&srv, "a", "").await.0, 400);
    assert_eq!(put_dns(&srv, "a", "[]").await.0, 400);
    assert_eq!(
        put_dns(&srv, "%20", DECLARATION).await.0,
        400,
        "a blank inbox label refuses like every other inbox route"
    );
    assert_eq!(delete_dns(&srv, "%20").await.0, 400);
    assert_eq!(
        http_json(
            srv.http_addr,
            "PUT",
            "/api/v1/inboxes/a/dns?x=1",
            Some("{}")
        )
        .await
        .0,
        400,
        "no query keys"
    );
    assert_eq!(report_text(&srv, &a).await, declared);
    // 8,192 bytes is accepted.
    let exactly = format!(r#"{{"txt":[],"tempfail":[]}}{}"#, " ".repeat(8_168));
    assert_eq!(put_dns(&srv, "edge", &exactly).await.0, 200);

    // DELETE is idempotent and answers the fallback.
    for _ in 0..2 {
        assert_eq!(
            delete_dns(&srv, "a").await,
            (
                200,
                json!({"inbox":"a","source":"default","policy":{"txt":[],"tempfail":[]}})
            )
        );
    }
    assert_eq!(dns_of(&srv, &a).await["source"], "default");

    // Clearing an inbox's mail keeps its declaration.
    put_dns(&srv, "a", DECLARATION).await;
    let (status, _) = http_json(srv.http_addr, "DELETE", "/api/v1/inboxes/a/messages", None).await;
    assert_eq!(status, 200);
    let again = smtp_raw(srv.smtp_addr, "a", MAIL).await;
    assert_eq!(dns_of(&srv, &again).await["source"], "inbox");
    srv.stop().await;
}

#[tokio::test]
async fn inbox_map_replaces_the_server_map_without_merging() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("dns.json");
    std::fs::write(
        &file,
        r#"{"txt":[{"name":"server.example","values":["s"]}]}"#,
    )
    .unwrap();
    let srv = start_with(|c| swarmail::config::Config {
        dns_policy: Some(file.clone()),
        ..c
    })
    .await;
    let (a, b) = (
        smtp_raw(srv.smtp_addr, "a", MAIL).await,
        smtp_raw(srv.smtp_addr, "b", MAIL).await,
    );
    let server = json!({"source":"server","policy":{"txt":[{"name":"server.example","values":["s"]}],"tempfail":[]},"lookups":[]});
    assert_eq!(dns_of(&srv, &a).await, server);
    put_dns(&srv, "a", r#"{"tempfail":["inbox.example"]}"#).await;
    assert_eq!(
        dns_of(&srv, &a).await,
        json!({"source":"inbox","policy":{"txt":[],"tempfail":["inbox.example"]},"lookups":[]})
    );
    assert_eq!(dns_of(&srv, &b).await, server);
    // The echo of a clear is the server map, not the empty default.
    assert_eq!(
        delete_dns(&srv, "a").await,
        (
            200,
            json!({"inbox":"a","source":"server","policy":server["policy"]})
        )
    );
    assert_eq!(dns_of(&srv, &a).await, server);
    srv.stop().await;

    // `{}` is a valid, empty server map and still reports `server`.
    std::fs::write(&file, "{}").unwrap();
    let srv = start_with(|c| swarmail::config::Config {
        dns_policy: Some(file.clone()),
        ..c
    })
    .await;
    let id = smtp_raw(srv.smtp_addr, "a", MAIL).await;
    assert_eq!(
        dns_of(&srv, &id).await,
        json!({"source":"server","policy":{"txt":[],"tempfail":[]},"lookups":[]})
    );
    srv.stop().await;
}

#[tokio::test]
async fn dns_policy_canonical_over_http() {
    let srv = start().await;
    let (a, b) = (
        smtp_raw(srv.smtp_addr, "a", MAIL).await,
        smtp_raw(srv.smtp_addr, "b", MAIL).await,
    );
    let permuted = r#"{"txt":[{"values":["v=spf1 -all","a-first"],"name":"EXAMPLE.com"},{"name":"s1._domainkey.example.com.","values":["v=DKIM1; p=AAAA"]}],"tempfail":["_dmarc.example.org"]}"#;
    let first = http_send_text(
        srv.http_addr,
        "PUT",
        "/api/v1/inboxes/a/dns",
        Some(DECLARATION),
    )
    .await;
    let second = http_send_text(
        srv.http_addr,
        "PUT",
        "/api/v1/inboxes/b/dns",
        Some(permuted),
    )
    .await;
    assert_eq!(first.0, 200);
    assert_eq!(
        first.1.replace("\"a\"", "\"x\""),
        second.1.replace("\"b\"", "\"x\""),
        "byte-identical echoes"
    );
    assert_eq!(
        report_text(&srv, &a).await,
        report_text(&srv, &b).await,
        "byte-identical reports for the same message"
    );
    // A report's `dns.policy` declares back verbatim.
    let policy = dns_of(&srv, &a).await["policy"].to_string();
    let (status, echo) = put_dns(&srv, "c", &policy).await;
    assert_eq!((status, echo["policy"].to_string()), (200, policy));
    srv.stop().await;
}

#[tokio::test]
async fn dns_change_reaches_next_report() {
    let srv = start().await;
    let id = smtp_raw(srv.smtp_addr, "a", MAIL).await;
    put_dns(&srv, "a", r#"{"tempfail":["one.example"]}"#).await;
    let first = dns_of(&srv, &id).await;
    assert_eq!(first["policy"]["tempfail"], json!(["one.example"]));
    // A name no rule reads still changes the report: policy is part of its identity.
    put_dns(&srv, "a", r#"{"tempfail":["two.example"]}"#).await;
    let second = dns_of(&srv, &id).await;
    assert_eq!(second["policy"]["tempfail"], json!(["two.example"]));
    put_dns(&srv, "a", r#"{"tempfail":["one.example"]}"#).await;
    assert_eq!(dns_of(&srv, &id).await, first);
    srv.stop().await;
}

#[tokio::test]
async fn tempfail_policy_does_not_touch_delivery() {
    let srv = start().await;
    let names = r#"{"tempfail":["example.com","s1._domainkey.example.com","_dmarc.example.com"]}"#;
    assert_eq!(put_dns(&srv, "a", names).await.0, 200);
    let id = smtp_raw(srv.smtp_addr, "a", MAIL).await;
    let (status, listed) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/a/messages", None).await;
    assert_eq!(status, 200);
    assert_eq!(listed["total"], 1, "stored and queryable at the 250");
    assert_eq!(listed["emails"][0]["id"], id);
    let dns = dns_of(&srv, &id).await;
    assert_eq!(dns["source"], "inbox");
    assert_eq!(dns["lookups"], json!([]));
    // Nothing on the accept, extract or persistence path knows about DNS.
    for (file, source) in [
        ("src/smtp.rs", include_str!("../src/smtp.rs")),
        ("src/persist.rs", include_str!("../src/persist.rs")),
    ] {
        assert!(!source.to_ascii_lowercase().contains("dns"), "{file}");
    }
    srv.stop().await;
}

#[tokio::test]
async fn declared_dmarc_does_not_satisfy_bulk_rules() {
    let srv = start().await;
    put_dns(
        &srv,
        "a",
        r#"{"txt":[{"name":"_dmarc.example.com","values":["v=DMARC1; p=reject"]},{"name":"s1._domainkey.example.com","values":["v=DKIM1; p=AAAA"]},{"name":"example.com","values":["v=spf1 -all"]}]}"#,
    )
    .await;
    let id = smtp_raw(srv.smtp_addr, "a", MAIL).await;
    let (_, body) = report(srv.http_addr, &id).await;
    let ids: Vec<&str> = body["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    for rule in [
        "bulk.dmarc_presence.not_evaluated.v1",
        "bulk.from_alignment.not_evaluated.v1",
        "bulk.authenticated_sender.not_evaluated.v1",
    ] {
        assert!(ids.contains(&rule), "{rule} in {ids:?}");
    }
    assert_eq!(
        body["dns"]["lookups"],
        json!([]),
        "no rule resolves a name yet"
    );
    srv.stop().await;
}

#[tokio::test]
async fn dns_declarations_full_is_503() {
    let srv = start().await;
    for n in 0..4096 {
        let (status, _) = put_dns(&srv, &format!("full{n}"), "{}").await;
        assert_eq!(status, 200, "inbox {n}");
    }
    let (status, error) = put_dns(&srv, "one-too-many", DECLARATION).await;
    assert_eq!(
        (status, error),
        (503, json!({"error":"dns declarations full"}))
    );
    // A replacement at capacity succeeds, and a cleared slot is reusable.
    assert_eq!(put_dns(&srv, "full0", DECLARATION).await.0, 200);
    assert_eq!(delete_dns(&srv, "full1").await.0, 200);
    assert_eq!(put_dns(&srv, "one-too-many", DECLARATION).await.0, 200);
    let id = smtp_raw(srv.smtp_addr, "full2", MAIL).await;
    assert_eq!(
        dns_of(&srv, &id).await["source"],
        "inbox",
        "nothing was evicted"
    );
    srv.stop().await;
}
