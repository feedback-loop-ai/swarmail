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
    assert_eq!(first["rule_set_id"], "swarmail-bulk/1");
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
        "swarmail-bulk/1",
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
    let raw = format!(
        "X-Swarmail-Demo: info\r\n{}\r\nbody\r\n",
        "X-Pad: abcdefghijklmnopqrstuvwxyz\r\n".repeat(2100)
    );
    let id = smtp_raw(srv.smtp_addr, "failure", raw.as_bytes()).await;
    smtp_raw(
        srv.smtp_addr,
        "failure",
        b"Subject: clean newest\r\n\r\nbody\r\n",
    )
    .await;
    assert_eq!(
        report(srv.http_addr, &id).await,
        (503, json!({"error":"deliverability analysis failed"}))
    );
    for op in ["assert?count=1", "await?count=2"] {
        let path = format!("/api/v1/inboxes/failure/{op}&no_findings_at_or_above=error");
        assert_eq!(
            http_json(srv.http_addr, "GET", &path, None).await,
            (503, json!({"error":"deliverability analysis failed"}))
        );
    }
    for (name, args) in [
        ("swarmail_get_deliverability_report", json!({"id":id})),
        (
            "swarmail_wait_for_email",
            json!({"inbox":"failure","count":2,"no_findings_at_or_above":"error"}),
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
        "/api/v1/inboxes/failure/messages",
        None,
    )
    .await;
    assert_eq!(status, 200);
    let clean = list["emails"][0]["id"].as_str().unwrap();
    let (status, page) = http_get_text(srv.http_addr, &format!("/ui/message/{clean}")).await;
    assert_eq!(status, 200);
    assert!(page.contains("bulk.from_alignment.not_evaluated.v1"));
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
    let raw = b"X-Swarmail-Demo: warning\r\nSubject: persisted\r\n\r\nhttps://x.io code 424242\r\n";
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
        "when #153 lands",
        "JSON in one text block",
    ] {
        assert!(llms.contains(expected), "{expected}");
    }
    srv.stop().await;
}
