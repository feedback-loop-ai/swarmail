//! Every REST surface, happy path and error branch, over real HTTP.

mod common;

use common::{http_get_text, http_json, start};

/// Seed one fixture mail into `inbox` (the default extractor path).
async fn seed(addr: std::net::SocketAddr, inbox: &str, to: &str) -> serde_json::Value {
    let body = format!(r#"{{"to": "{to}", "text": "Click https://x.io/v?t=a. Code 424242."}}"#);
    let (st, body) = http_json(
        addr,
        "POST",
        &format!("/api/v1/inboxes/{inbox}/seed"),
        Some(&body),
    )
    .await;
    assert_eq!(st, 200, "seed failed: {body}");
    body
}

async fn first_id(addr: std::net::SocketAddr, inbox: &str) -> String {
    let (st, body) = http_json(
        addr,
        "GET",
        &format!("/api/v1/inboxes/{inbox}/messages"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    body["emails"][0]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn healthz_reports_ok() {
    let srv = start().await;
    let (st, body) = http_json(srv.http_addr, "GET", "/healthz", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["status"], "ok");
    assert!(body["uptime_seconds"].is_u64());
}

#[tokio::test]
async fn metrics_exposes_counters_and_gauges() {
    let srv = start().await;
    seed(srv.http_addr, "m", "user@example.com").await;
    let (st, text) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(st, 200);
    for needle in [
        "# TYPE swarmail_emails_inserted_total counter",
        "swarmail_emails_inserted_total 1",
        "# TYPE swarmail_emails_dropped_total counter",
        "# TYPE swarmail_emails_stored gauge",
        "swarmail_inboxes 1",
        // The bounded per-PEM webhook client cache: an operator can watch
        // the cap hold and evictions happen.
        "# TYPE swarmail_webhook_ca_cache_entries gauge",
        "# TYPE swarmail_webhook_ca_cache_builds_total counter",
        "# TYPE swarmail_webhook_ca_cache_evictions_total counter",
    ] {
        assert!(text.contains(needle), "metrics missing {needle}:\n{text}");
    }
}

#[tokio::test]
async fn list_inboxes_names_and_counts() {
    let srv = start().await;
    for inbox in ["b", "a"] {
        seed(srv.http_addr, inbox, "user@example.com").await;
    }
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200);
    assert_eq!(body[0]["name"], "a");
    assert_eq!(body[0]["count"], 1);
    assert_eq!(body[1]["name"], "b");
}

#[tokio::test]
async fn assert_endpoint_returns_ok_or_conflict() {
    let srv = start().await;
    seed(srv.http_addr, "assert", "user@example.com").await;

    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/assert/assert?count=1",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["ok"], true);
    assert_eq!(body["matched"], 1);

    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/assert/assert?count=2",
        None,
    )
    .await;
    assert_eq!(st, 409);
    assert_eq!(body["ok"], false);
    assert_eq!(body["expected"], 2);
    assert_eq!(body["matched"], 1);
}

#[tokio::test]
async fn await_endpoint_finds_and_times_out() {
    let srv = start().await;
    seed(srv.http_addr, "aw", "user@example.com").await;

    // Found: the mail is already there.
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/aw/await?count=1&to=user@example.com",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["matched"], 1);
    assert_eq!(body["emails"][0]["to"][0]["address"], "user@example.com");

    // Timeout: nothing NEW matches, the deadline elapses; matched reports
    // how many existing mails satisfied the filter.
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/aw/await?count=5&timeout_ms=150",
        None,
    )
    .await;
    assert_eq!(st, 408);
    assert_eq!(body["matched"], 1);
    assert_eq!(body["emails"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn clear_inbox_and_clear_all_count_removals() {
    let srv = start().await;
    seed(srv.http_addr, "c1", "user@example.com").await;
    seed(srv.http_addr, "c2", "user@example.com").await;

    let (st, body) = http_json(srv.http_addr, "DELETE", "/api/v1/inboxes/c1/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["removed"], 1);

    let (st, body) = http_json(srv.http_addr, "DELETE", "/api/v1/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["removed"], 1);

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/c2/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn message_get_raw_delete_and_missing_404s() {
    let srv = start().await;
    seed(srv.http_addr, "msg", "user@example.com").await;
    let id = first_id(srv.http_addr, "msg").await;

    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["subject"], "(no subject)");

    let (st, raw) = http_get_text(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(st, 200);
    assert!(raw.contains("Subject: (no subject)"));

    let (st, body) = http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["deleted"], true);

    for path in [
        format!("/api/v1/messages/{id}"),
        format!("/api/v1/messages/{id}/raw"),
    ] {
        let (st, body) = http_json(srv.http_addr, "GET", &path, None).await;
        assert_eq!(st, 404, "{path}");
        assert_eq!(body["error"], "message not found");
    }
    let (st, _) = http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 404);
}

#[tokio::test]
async fn chaos_roundtrip_and_invalid_body_is_a_bad_request() {
    let srv = start().await;
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);
    assert!(body["data"].is_null());

    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100, "error": "451 nope", "delay_ms": 5}}"#),
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["data"]["error"], "451 nope");

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["data"]["error"], "451 nope");

    // A body that fails to deserialize is rejected by the extractor with a
    // 400 before the handler runs.
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/chaos", Some("not json")).await;
    assert_eq!(st, 400);

    let (st, body) = http_json(srv.http_addr, "DELETE", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);
    assert!(body["data"].is_null(), "chaos is back to disabled: {body}");
}

#[tokio::test]
async fn seed_with_html_runs_the_extraction_pipeline() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/html/seed",
        Some(
            r#"{"to": "h@x.io", "subject": "hi", "html": "<a href='https://q.io?a=1&b=2'>go</a> 555555"}"#,
        ),
    )
    .await;
    assert_eq!(st, 200);
    assert!(body["html"].as_str().unwrap().contains("https://q.io"));
    assert_eq!(body["codes"][0], "555555");
}

#[tokio::test]
async fn seed_with_invalid_body_is_a_bad_request() {
    let srv = start().await;
    // The Json extractor rejects malformed bodies before the handler runs.
    let (st, _) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/bad/seed",
        Some("{ nope"),
    )
    .await;
    assert_eq!(st, 400);
}

#[tokio::test]
async fn invalid_query_strings_are_bad_requests() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/x/await?count=notanumber",
        None,
    )
    .await;
    assert_eq!(st, 400);
    assert!(body["error"].as_str().unwrap().contains("await"));
}

#[tokio::test]
async fn webhook_targets_get_set_and_cleared() {
    let srv = start().await;
    let (st, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(r#"[{"url": "http://127.0.0.1:1/hooks", "inbox": "only-this", "secret": "s"}]"#),
    )
    .await;
    assert_eq!(st, 200);

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(st, 200);
    assert_eq!(body[0]["inbox"], "only-this");
    assert_eq!(body[0]["secret"], "s");

    let (st, body) = http_json(srv.http_addr, "DELETE", "/api/v1/webhooks", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["cleared"], true);
    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn mcp_endpoint_answers_and_notifications_are_accepted() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#),
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26");

    let (st, _) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
    )
    .await;
    assert_eq!(st, 202);

    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":2,"method":"no/such/method"}"#),
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["error"]["message"], "unknown method: no/such/method");
}

#[tokio::test]
async fn machine_docs_are_served_from_the_binary() {
    let srv = start().await;
    let (st, text) = http_get_text(srv.http_addr, "/openapi.json").await;
    assert_eq!(st, 200);
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["openapi"], "3.1.0");

    let (st, text) = http_get_text(srv.http_addr, "/llms.txt").await;
    assert_eq!(st, 200);
    assert!(text.contains("Swarmail"));
}

/// Deleting an unknown id walks `Store::delete`'s no-hit arm and answers 404.
#[tokio::test]
async fn deleting_an_unknown_id_is_a_404() {
    let srv = start().await;
    let (st, body) = http_json(srv.http_addr, "DELETE", "/api/v1/messages/no-such-id", None).await;
    assert_eq!(st, 404);
    assert_eq!(body["error"], "message not found");
}

/// `Store::list_all` orders by received_ms DESC with the id as the tie-break
/// (`then_with`); two hundred rapid seeds guarantee same-millisecond pairs,
/// so the tie-break arm runs and the ordering property must hold throughout.
#[tokio::test]
async fn list_all_orders_desc_with_a_deterministic_tie_break() {
    let srv = start().await;
    for i in 0..200 {
        let (st, _) = http_json(
            srv.http_addr,
            "POST",
            &format!("/api/v1/inboxes/tie{i}/seed"),
            Some(r#"{"to": "t@x.io", "subject": "tie", "text": "x"}"#),
        )
        .await;
        assert_eq!(st, 200);
    }
    // The all-inbox merged view rides the compat shim (no inbox scoping).
    let (st, list) = http_json(srv.http_addr, "GET", "/api/v1/messages?limit=500", None).await;
    assert_eq!(st, 200);
    let emails = list["messages"].as_array().expect("the all-inbox list");
    assert!(emails.len() >= 200, "every seed landed: {}", emails.len());
    // Merged view: received_ms DESC with the UUIDv7 id as tie-break. v7 ids
    // sort chronologically, so the whole list must be id-descending — and
    // same-millisecond pairs (guaranteed by 200 rapid seeds) are ordered by
    // the tie-break arm alone.
    let mut prev_id: Option<String> = None;
    for e in emails {
        let id = e["ID"].as_str().unwrap().to_string();
        if let Some(prev) = &prev_id {
            assert!(id < *prev, "ids descend: {id} after {prev}");
        }
        prev_id = Some(id);
    }
}
