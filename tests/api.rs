//! Every REST surface, happy path and error branch, over real HTTP.

mod common;

use common::{
    SmtpConn, http_bytes, http_get_text, http_head_text, http_json, http_send_text,
    http_send_text_with_headers, send_with_host, start, start_with, wait_for_metric_row,
};

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
        // The delivery ledger: five counters, fresh-zero on a fresh server.
        // An operator watches `dropped` for bound refusals and `aborted`
        // for deliveries the shutdown counted instead of losing silently.
        "# TYPE swarmail_webhook_delivered_total counter",
        "swarmail_webhook_delivered_total 0",
        "# TYPE swarmail_webhook_dropped_total counter",
        "swarmail_webhook_dropped_total 0",
        "# TYPE swarmail_webhook_lagged_total counter",
        "swarmail_webhook_lagged_total 0",
        "# TYPE swarmail_webhook_exhausted_total counter",
        "swarmail_webhook_exhausted_total 0",
        "# TYPE swarmail_webhook_aborted_total counter",
        "swarmail_webhook_aborted_total 0",
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

/// `count=0` is an exact expectation, not a request to wait for one mail
/// (issue #6): it holds only when nothing matches — the "no mail was sent"
/// assertion — with or without a filter, and an omitted count still means 1.
#[tokio::test]
async fn assert_endpoint_accepts_zero_as_an_exact_count() {
    let srv = start().await;
    let addr = srv.http_addr;
    let check = move |query: &'static str| {
        let path = format!("/api/v1/inboxes/zero/assert?{query}");
        async move { http_json(addr, "GET", &path, None).await }
    };

    // An empty inbox: exactly zero matched.
    let (st, body) = check("count=0").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["matched"], 0);

    // One mail: zero no longer holds, and the failure names 0 as expected.
    seed(srv.http_addr, "zero", "user@example.com").await;
    let (st, body) = check("count=0").await;
    assert_eq!(st, 409, "{body}");
    assert_eq!(body["ok"], false);
    assert_eq!(body["expected"], 0);
    assert_eq!(body["matched"], 1);

    // Filtered zero: nothing for another recipient, though the inbox is not
    // empty; the filtered recipient itself does not hold at zero.
    let (st, body) = check("count=0&to=nobody@example.com").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 0);
    let (st, body) = check("count=0&to=user@example.com").await;
    assert_eq!(st, 409, "{body}");
    assert_eq!(body["expected"], 0);
    assert_eq!(body["matched"], 1);

    // An omitted count still expects exactly one.
    let (st, body) = check("").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1);

    // Multiple mails: exact counts above one, and zero still refuses.
    seed(srv.http_addr, "zero", "other@example.com").await;
    let (st, body) = check("count=2").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 2);
    let (st, body) = check("").await;
    assert_eq!(st, 409, "{body}");
    assert_eq!(body["expected"], 1);
    assert_eq!(body["matched"], 2);
    let (st, body) = check("count=0").await;
    assert_eq!(st, 409, "{body}");
    assert_eq!(body["expected"], 0);
    assert_eq!(body["matched"], 2);
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

    // The cross-inbox wipe is the explicit alias now (decision 0011): a
    // body-less shim DELETE is a 400, not a wipe.
    let (st, body) = http_json(srv.http_addr, "DELETE", "/api/v1/delete-all", None).await;
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
    // 400 before the handler runs — and the rejection body is the
    // extractor's own plain text, never a JSON envelope; `http_send_text`
    // reads it honestly where `http_json` would refuse the body.
    let (st, body) = http_send_text(srv.http_addr, "PUT", "/api/v1/chaos", Some("not json")).await;
    assert_eq!(st, 400);
    assert!(
        body.starts_with("Failed to parse the request body"),
        "the rejection is the extractor's own text: {body:?}"
    );

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
    // The Json extractor rejects malformed bodies before the handler runs,
    // with its own plain-text body — the text cousin reads it honestly;
    // `http_json` would (correctly) refuse a non-JSON body.
    let (st, body) = http_send_text(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/bad/seed",
        Some("{ nope"),
    )
    .await;
    assert_eq!(st, 400);
    assert!(
        body.starts_with("Failed to parse the request body"),
        "the rejection is the extractor's own text: {body:?}"
    );
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
    // The secret is redacted on read (decision 0029 §5): an echoed secret
    // in a response body is a log line — PUT is the only writer, and the
    // operator retains their own payload.
    assert_eq!(body[0]["secret"], serde_json::Value::Null);

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

    // A notification is accepted (202) with no body — fire-and-forget has
    // nothing to answer, and absence read as `{}` is the dishonesty issue
    // #50 names. The text cousin keeps the absence honest.
    let (st, body) = http_send_text(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
    )
    .await;
    assert_eq!(st, 202);
    assert!(body.is_empty(), "a notification has no body: {body:?}");

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
    // The all-inbox merged view rides the compat shim (no inbox scoping);
    // the page is the shared cap (decision 0030 §3) — enough for the 200
    // seeds this property needs.
    let (st, list) = http_json(srv.http_addr, "GET", "/api/v1/messages?limit=250", None).await;
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

/// The exact octets DATA accepted come back from `/raw`: binary body intact
/// (8BITMIME is real), dot-stuffed line unstuffed, CRLFs preserved, served
/// as message/rfc822 — never re-encoded. The JSON shape still omits `raw`.
#[tokio::test]
async fn raw_route_serves_the_exact_received_bytes() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<bin@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<bytes@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("354"));
    // Sent as raw octets — no string round-trip anywhere: a NUL and invalid
    // UTF-8 ride along, and one line arrives dot-stuffed.
    c.send_bytes(b"Subject: binary round trip\r\n\r\n..dot stuffed\r\n\x00\x80\xff\xfe\r\n")
        .await;
    c.send(".").await;
    let reply = c.reply().await;
    assert!(
        reply.starts_with("250"),
        "8BITMIME body accepted: {reply:?}"
    );
    let id = reply
        .strip_prefix("250 2.0.0 OK: stored as ")
        .expect("the 250 names the id")
        .to_string();

    // Byte-identical: what DATA accepted is what /raw serves.
    let path = format!("/api/v1/messages/{id}/raw");
    let (st, body) = http_bytes(srv.http_addr, &path).await;
    assert_eq!(st, 200);
    assert_eq!(
        body,
        b"Subject: binary round trip\r\n\r\n.dot stuffed\r\n\x00\x80\xff\xfe\r\n"
    );

    // The head promises the source, not a re-encoded text/plain.
    let (st, head) = http_head_text(srv.http_addr, &path).await;
    assert_eq!(st, 200);
    assert!(
        head.to_lowercase().contains("content-type: message/rfc822"),
        "{head}"
    );

    // The JSON shape stays honest: no base64, no envelope, no raw key.
    let (st, json) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert!(
        json.get("raw").is_none(),
        "raw must stay out of JSON: {json}"
    );

    // The 404 arm is untouched.
    let (st, _) = http_bytes(srv.http_addr, "/api/v1/messages/no-such-id/raw").await;
    assert_eq!(st, 404);
}

/// The served contract moves with the wire: the native raw route promises
/// message/rfc822, not text/plain.
#[tokio::test]
async fn openapi_describes_the_raw_route_as_rfc822() {
    let srv = start().await;
    let (st, doc) = http_json(srv.http_addr, "GET", "/openapi.json", None).await;
    assert_eq!(st, 200);
    let description =
        doc["paths"]["/api/v1/messages/{id}/raw"]["get"]["responses"]["200"]["description"]
            .as_str()
            .expect("the raw route documents its 200");
    assert!(
        description.contains("message/rfc822"),
        "the raw route must promise exact bytes: {description}"
    );
}

// ---------- await bounds, shutdown race and observed totals (decision 0018) ----------

/// The wait cap is a refusal, never a clamp: over the cap answers 400 naming
/// the maximum, before anything is subscribed (decision 0018). Within the
/// cap the wait is accepted — proven by parking 60000 and satisfying it
/// mid-wait; a refusal would have answered 400 instead.
#[tokio::test]
async fn await_timeout_over_the_cap_is_refused_and_the_cap_itself_is_accepted() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/cap/await?count=1&timeout_ms=60001",
        None,
    )
    .await;
    assert_eq!(st, 400);
    assert_eq!(
        body,
        serde_json::json!({ "error": "timeout_ms must be at most 60000" }),
        "{body}"
    );

    // 60000 is legal: the wait parks and the mid-wait delivery satisfies it.
    let addr = srv.http_addr;
    let request = tokio::spawn(http_json(
        addr,
        "GET",
        "/api/v1/inboxes/cap/await?count=1&to=capped@x.io&timeout_ms=60000",
        None,
    ));
    // The wait parks, observed under its own inbox — the mail can then
    // never miss it (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"cap\"}", 1).await;
    common::smtp_send(
        srv.smtp_addr,
        Some("cap"),
        "f@x.io",
        "capped@x.io",
        "within the cap",
        "body",
    )
    .await
    .unwrap();
    let (st, body) = tokio::time::timeout(std::time::Duration::from_secs(10), request)
        .await
        .expect("a within-cap wait must not outlive its bound")
        .unwrap();
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1);
}

/// The await's `total` describes the response's observed state: counted
/// after the wait, in both arms (decision 0018). The pre-wait snapshot this
/// replaces would report the inbox as it stood before the wait's mail
/// arrived — the stale-total disease.
#[tokio::test]
async fn await_total_is_counted_after_the_wait_in_both_arms() {
    let srv = start().await;
    let addr = srv.http_addr;

    // 200 arm: the mail arrives mid-wait; total counts it.
    let request = tokio::spawn(http_json(
        addr,
        "GET",
        "/api/v1/inboxes/observed/await?count=1&to=racer@x.io&timeout_ms=10000",
        None,
    ));
    // The wait parks, observed under its own inbox — the mail arrives
    // strictly after the wait registered, so the post-wait total counts it
    // deterministically (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"observed\"}", 1).await;
    common::smtp_send(
        srv.smtp_addr,
        Some("observed"),
        "f@x.io",
        "racer@x.io",
        "midwait",
        "body",
    )
    .await
    .unwrap();
    let (st, body) = tokio::time::timeout(std::time::Duration::from_secs(10), request)
        .await
        .expect("the wait must resolve on the delivery")
        .unwrap();
    assert_eq!(st, 200);
    assert_eq!(body["matched"], 1);
    assert_eq!(body["total"], 1, "total is counted after the wait: {body}");

    // 408 arm: a non-matching mail lands mid-wait; total counts it while
    // `matched` stays honest about the filter.
    let request = tokio::spawn(http_json(
        addr,
        "GET",
        "/api/v1/inboxes/observed408/await?count=1&to=nobody@x.io&timeout_ms=1500",
        None,
    ));
    // The wait parks, observed under its own inbox — the unrelated mail
    // can never arrive before the wait registered its subscription (issue
    // #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"observed408\"}", 1).await;
    common::smtp_send(
        srv.smtp_addr,
        Some("observed408"),
        "f@x.io",
        "someone-else@x.io",
        "unrelated",
        "body",
    )
    .await
    .unwrap();
    let (st, body) = tokio::time::timeout(std::time::Duration::from_secs(10), request)
        .await
        .expect("the wait must time out on its own bound")
        .unwrap();
    assert_eq!(st, 408);
    assert_eq!(body["matched"], 0);
    assert_eq!(body["total"], 1, "total is counted after the wait: {body}");
}

// ---------- shared domain validation (issue #32 slice 2, decision 0025) ----------

/// An out-of-domain chaos probability is refused with 400 and the shared
/// sentence — the same words the MCP tool answers — and a rejected PUT
/// leaves the active rules exactly as they were, by construction: the
/// handler validates before any `Chaos::set`. A mixed partial config with
/// one bad slot is refused too (the walk sees all four events), and the
/// poles 0 and 100 are legal on the domain layer.
#[tokio::test]
async fn chaos_probability_out_of_domain_is_refused_and_leaves_the_rules_unchanged() {
    let srv = start().await;

    // Arm a valid data rule.
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 30, "error": "451 busy", "delay_ms": 5}}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let (st, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);

    for probability in [101, 150, 255] {
        let put = format!(r#"{{"data": {{"probability": {probability}}}}}"#);
        let (st, body) = http_json(srv.http_addr, "PUT", "/api/v1/chaos", Some(&put)).await;
        assert_eq!(st, 400, "{probability}: {body}");
        assert_eq!(
            body,
            serde_json::json!({ "error": "probability must be an integer between 0 and 100" }),
            "{probability}: {body}"
        );
    }

    // A mixed partial config: one bad slot refuses the whole PUT, and the
    // armed data rule survives the rejected request untouched.
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"rcpt": {"probability": 150}}"#),
    )
    .await;
    assert_eq!(st, 400, "{body}");
    let (st, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        before, after,
        "a rejected PUT must leave the active rules exactly as they were"
    );

    // The poles are legal on the domain layer, not clamped or refused.
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 0}}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["data"]["probability"], 0, "{body}");
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100}}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["data"]["probability"], 100, "{body}");
}

/// A well-formed body with a wrong-typed `probability` keeps the Json
/// extractor's own rejection — 422, serde's message — and malformed JSON
/// keeps its 400 (decision 0025: the slice shares the domain *sentence*,
/// not the extractor's envelope). The rules survive each rejection.
#[tokio::test]
async fn chaos_wrong_typed_probability_keeps_the_extractor_envelope_and_the_rules() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 30}}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let (st, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);

    for (put, status) in [
        (r#"{"data": {"probability": "high"}}"#, 422),
        (r#"{"data": {"probability": 1.5}}"#, 422),
        (r#"{"data": {"probability": null}}"#, 422),
        (r#"{"data": {"probability": 256}}"#, 422),
        ("not json", 400),
    ] {
        // The extractor's rejection is plain text, never a JSON envelope —
        // no `{"error": …}` is synthesized for a malformed body — so the
        // text cousin reads it honestly; `http_json` would refuse the body
        // (and the old `{}` coercion made the envelope assertion vacuous).
        let (st, body) = http_send_text(srv.http_addr, "PUT", "/api/v1/chaos", Some(put)).await;
        assert_eq!(st, status, "{put}: {body}");
        assert!(
            body.starts_with("Failed to deserialize the JSON body")
                || body.starts_with("Failed to parse the request body"),
            "{put}: the rejection is the extractor's own text: {body:?}"
        );
    }

    let (st, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);
    assert_eq!(before, after, "an extractor rejection stores nothing");
}

/// Supplied malformed `timeout_ms` values keep REST's parse-time 400 — the
/// query layer refuses before the handler runs, so no waiter is ever
/// registered — while `0` is a legal, real zero deadline: an immediate 408
/// on an empty inbox, never a clamp to the default and never a refusal
/// (decision 0025).
#[tokio::test]
async fn await_supplied_malformed_timeouts_are_parse_refusals_and_zero_is_a_real_deadline() {
    let srv = start().await;
    for value in ["-1", "invalid", "null", "1.5"] {
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/malformed/await?count=1&timeout_ms={value}"),
            None,
        )
        .await;
        assert_eq!(st, 400, "{value}: {body}");
        assert!(
            body["error"].as_str().unwrap().contains("await"),
            "{value}: {body}"
        );
    }

    let started = std::time::Instant::now();
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/zero-deadline/await?count=1&to=nobody@x.io&timeout_ms=0",
        None,
    )
    .await;
    assert_eq!(st, 408, "{body}");
    assert_eq!(body["matched"], 0, "{body}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "timeout_ms=0 must answer its own deadline at once: {:?}",
        started.elapsed()
    );
}

/// The filters the list endpoint accepts parse identically on await and
/// assert (issue #38): under the old `#[serde(flatten)]` the urlencoded
/// layer handed every flattened field over as a string and serde refused
/// the integer, so `since_ms` was refused on both while list accepted it.
/// The flat struct (decision 0029 §1) makes the parameter set explicit — a
/// past `since_ms` is an inclusive filter, a future one is a real filter
/// the wait can time out and the assert can conflict on, and a malformed
/// value keeps the parse refusal naming the field (decision 0025).
#[tokio::test]
async fn await_and_assert_parse_since_ms_exactly_as_list_does() {
    let srv = start().await;
    seed(srv.http_addr, "flt", "user@example.com").await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is set")
        .as_millis() as i64;

    // A `since_ms` in the past is inclusive: the mail matches, on both.
    let past = now - 10_000;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/flt/await?count=1&to=user@example.com&since_ms={past}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1, "{body}");
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/flt/assert?count=1&to=user@example.com&since_ms={past}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1, "{body}");

    // A `since_ms` in the future excludes the mail — the filter is live,
    // not decoration: the wait can time out on it, the assert conflicts.
    let future = now + 600_000;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!(
            "/api/v1/inboxes/flt/await?count=1&to=user@example.com&since_ms={future}&timeout_ms=200"
        ),
        None,
    )
    .await;
    assert_eq!(st, 408, "{body}");
    assert_eq!(body["matched"], 0, "{body}");
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/flt/assert?count=1&to=user@example.com&since_ms={future}"),
        None,
    )
    .await;
    assert_eq!(st, 409, "{body}");
    assert_eq!(body["matched"], 0, "{body}");

    // A malformed `since_ms` is a supplied value: refused at the parse —
    // the plane's own refusal class, the same words a malformed
    // `timeout_ms` gets (the extractor's cause rides along).
    for (path, what) in [
        ("await", "invalid await query"),
        ("assert", "invalid assert query"),
    ] {
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/flt/{path}?since_ms=soon"),
            None,
        )
        .await;
        assert_eq!(st, 400, "{path}: {body}");
        assert!(
            body["error"].as_str().unwrap().starts_with(what),
            "{path}: the refusal names the plane: {body}"
        );
    }

    // `limit` parses on await and assert exactly as on list (#38): two
    // mails match, the await still triggers at `count` but returns at
    // most the limit's worth, assert accepts it (its answer is a count,
    // not a page), and a supplied wrong type refuses with the plane's
    // shared 400 sentence — the silent-ignore class 0029 §1 closes.
    seed(srv.http_addr, "flt", "user@example.com").await;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/flt/await?count=2&limit=1&to=user@example.com&since_ms={past}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 2, "{body}");
    let page = body["emails"].as_array().expect("the email page");
    assert_eq!(page.len(), 1, "the limit bounds the page: {body}");
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/flt/assert?count=2&limit=1&to=user@example.com",
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 2, "{body}");
    for (path, what) in [
        ("await", "invalid await query"),
        ("assert", "invalid assert query"),
    ] {
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/flt/{path}?limit=abc"),
            None,
        )
        .await;
        assert_eq!(st, 400, "{path}: {body}");
        assert!(
            body["error"].as_str().unwrap().starts_with(what),
            "{path}: the refusal names the plane: {body}"
        );
    }
}

/// The chaos wire-text rules refuse on the REST plane with the shared
/// sentences (issue #41, decision 0029 §3): an `error` that would answer
/// `250` — the rule that made every DATA acceptance a lie — is not a
/// failure, CR/LF would desynchronize the session stream, and an over-cap
/// `delay_ms` exceeds the stage-timer scale; an unknown event or rule key
/// keeps the extractor's own 422. A rejected PUT leaves the armed rules
/// exactly as they were, and a legal rule still stores after the refusals.
#[tokio::test]
async fn the_chaos_error_line_and_delay_refuse_on_the_rest_plane() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 30, "error": "451 busy"}}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let (st, before) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);

    for (put, sentence) in [
        // A 2xx error line is not a failure: the rule would answer success
        // while the chaos arm stored nothing — guarantee 1 broken through
        // the control plane.
        (
            r#"{"data": {"probability": 100, "error": "250 2.0.0 OK"}}"#,
            "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets",
        ),
        // CR/LF would write two replies for one command.
        (
            r#"{"data": {"probability": 100, "error": "551 go\r\naway"}}"#,
            "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets",
        ),
        // Not a code at all.
        (
            r#"{"data": {"probability": 100, "error": "go away"}}"#,
            "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets",
        ),
        // The delay is the stage-timer scale, never unbounded.
        (
            r#"{"data": {"probability": 100, "delay_ms": 300001}}"#,
            "delay_ms must be an integer between 0 and 300000",
        ),
    ] {
        let (st, body) = http_json(srv.http_addr, "PUT", "/api/v1/chaos", Some(put)).await;
        assert_eq!(st, 400, "{put}: {body}");
        assert_eq!(
            body,
            serde_json::json!({ "error": sentence }),
            "{put}: {body}"
        );
    }

    // The cap is inclusive on the legal side and octets, not chars: 511
    // refuses with the same sentence (510 is pinned legal below, after the
    // unchanged check).
    let over = format!("551 {}", "x".repeat(507));
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(&format!(
            r#"{{"data": {{"probability": 100, "error": {over:?}}}}}"#
        )),
    )
    .await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(
        body,
        serde_json::json!({ "error": "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets" }),
        "{body}"
    );

    // Unknown events and unknown rule keys are not silently dropped — the
    // MCP plane refuses them, so REST does too (the extractor's own 422).
    for put in [
        r#"{"solo": {"probability": 100}}"#,
        r#"{"data": {"probability": 100, "wat": 1}}"#,
    ] {
        let (st, body) = http_send_text(srv.http_addr, "PUT", "/api/v1/chaos", Some(put)).await;
        assert_eq!(st, 422, "{put}: {body}");
    }

    // The armed rule survived every refusal unchanged, and the boundary is
    // inclusive: a legal rule — 510 octets, the delay ceiling itself —
    // still stores after them.
    let (st, after) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        before, after,
        "a rejected PUT must leave the active rules exactly as they were"
    );
    let boundary = format!("551 {}", "x".repeat(506));
    let (st, body) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(&format!(
            r#"{{"data": {{"probability": 100, "error": {boundary:?}, "delay_ms": 300000}}}}"#
        )),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["data"]["error"], boundary, "{body}");
    assert_eq!(body["data"]["delay_ms"], 300000, "{body}");
}

// ---------- the bounded pages + Arc responses (decision 0030) ----------

/// The list's page cap is a refusal, never a clamp: over it the answer is
/// `400` naming field and range in one sentence (decision 0025's envelope,
/// 0030 §3's rule), and exactly the cap is a legal page — the boundary
/// itself answers 200 with a full page.
#[tokio::test]
async fn the_list_refuses_over_the_cap_and_accepts_exactly_it() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/capped/messages?limit=251",
        None,
    )
    .await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(
        body,
        serde_json::json!({ "error": "limit must be at most 250" })
    );

    // Exactly the cap is a page, not a refusal: 250 seeds, one boundary page.
    for i in 0..250 {
        let body = format!(r#"{{"to": "b{i}@x.io", "text": "x"}}"#);
        let (st, body) = http_json(
            srv.http_addr,
            "POST",
            "/api/v1/inboxes/boundary/seed",
            Some(&body),
        )
        .await;
        assert_eq!(st, 200, "{body}");
    }
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/boundary/messages?limit=250",
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["emails"].as_array().map(Vec::len), Some(250));
    // The default page is unchanged: no limit, no refusal, the shared 100.
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/boundary/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["emails"].as_array().map(Vec::len), Some(100));
}

/// The wait's staging refuses before it parks (0025 §5 through the REST
/// plane): `count` and `limit` over the cap answer 400 with the sentence,
/// register no waiter, and the exact cap is a legal wait — accepted, timed
/// out cleanly at 408 with `matched: 0`.
#[tokio::test]
async fn the_wait_refuses_over_the_cap_registers_no_waiter_and_accepts_the_boundary() {
    let srv = start().await;
    for (field, query) in [
        ("count", "count=251"),
        ("limit", "limit=251"),
        ("count", "count=251&limit=251"),
    ] {
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/nocap/await?{query}&timeout_ms=60000"),
            None,
        )
        .await;
        assert_eq!(st, 400, "{field}: {body}");
        assert_eq!(
            body,
            serde_json::json!({ "error": format!("{field} must be at most 250") }),
            "{field}: {body}"
        );
    }

    // The prompt refusal is the 0025 §5 proof itself: a parked wait could
    // not have answered. (The awaits gauge cannot — its row is removed at
    // completion, so a scrape after the fact says nothing about whether a
    // waiter ever registered.)
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(
        common::metric_row_value(&metrics, r#"swarmail_awaits_active{inbox="nocap"}"#),
        None,
        "the released row is gone: {metrics}"
    );

    // The boundary is a legal wait: 250 accepted (a refusal would answer
    // 400), timed out cleanly at 408 with `matched: 0` — 0010's count
    // semantics untouched by the cap.
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/boundarywait/await?count=250&limit=250&timeout_ms=200",
        None,
    )
    .await;
    assert_eq!(st, 408, "{body}");
    assert_eq!(body["matched"], 0);
}

/// The exact-count `assert` is exempt from the page cap — its `count` is an
/// exact expectation, not a page, and its `limit` is inert (0030 §3): both
/// ride past the cap and answer by assertion outcome, never 400.
#[tokio::test]
async fn the_assert_never_refuses_on_the_page_cap() {
    let srv = start().await;
    seed(srv.http_addr, "assertinert", "user@example.com").await;
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/assertinert/assert?count=251&limit=251",
        None,
    )
    .await;
    assert_eq!(st, 409, "the assert answers by outcome, not 400: {body}");
    assert_eq!(body["expected"], 251);
    assert_eq!(body["matched"], 1);
}

/// Every Arc response is byte-identical to the whole-clone shape it
/// replaced (the inline pins hold it for the types; this holds it for the
/// wire): the await, get and seed answers carry full fields and no `raw`
/// key — the raw bytes live only behind `/raw` (0022).
#[tokio::test]
async fn the_arc_responses_carry_no_raw_and_full_fields() {
    let srv = start().await;
    let seeded = seed(srv.http_addr, "arcwire", "arc@x.io").await;
    assert!(
        seeded.get("raw").is_none(),
        "the seed response carries no raw key: {seeded}"
    );

    let id = first_id(srv.http_addr, "arcwire").await;
    let (st, mail) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "{mail}");
    assert!(
        mail.get("raw").is_none(),
        "the get response carries no raw: {mail}"
    );
    assert_eq!(mail["subject"], "(no subject)");
    assert_eq!(mail["to"][0]["address"], "arc@x.io");

    let (st, awaited) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/arcwire/await?count=1&to=arc@x.io&timeout_ms=5000",
        None,
    )
    .await;
    assert_eq!(st, 200, "{awaited}");
    assert_eq!(awaited["matched"], 1);
    assert!(
        awaited["emails"][0].get("raw").is_none(),
        "the await response carries no raw: {awaited}"
    );
    assert_eq!(awaited["emails"][0]["id"], id);
}

// ---------- the loopback posture (decision 0030 §2) ----------

/// Every loopback spelling of the bound HTTP port passes the guard — the
/// scheme is irrelevant, the host's case is normalized, 127/8 in full —
/// and an absent `Origin` rides through untouched (bridge, agents, curl,
/// the test suites themselves).
#[tokio::test]
async fn same_origin_loopback_spellings_pass_the_guard() {
    let srv = start().await;
    let port = srv.http_addr.port();
    for origin in [
        format!("http://localhost:{port}"),
        format!("https://LOCALHOST:{port}"),
        format!("http://127.0.0.1:{port}"),
        format!("http://127.255.0.9:{port}"),
        format!("http://[::1]:{port}"),
    ] {
        let (st, body) = http_send_text_with_headers(
            srv.http_addr,
            "GET",
            "/api/v1/inboxes",
            &[("Origin", origin.as_str())],
            None,
        )
        .await;
        assert_eq!(st, 200, "{origin}: {body}");
    }
    // Absent Origin: the plane's default caller.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200, "{body}");
}

/// A non-loopback host, a wrong port, or a spelling that cannot name the
/// bound port at all is answered `403` with the refusal sentence — on
/// every method, `/mcp` included. The metric claim is not re-asserted
/// here: the in-flight gauge is removed at response completion, so a
/// scrape after the fact cannot distinguish a never-registered request
/// from a released one (the layering claim has no observable).
#[tokio::test]
async fn cross_origin_and_malformed_spellings_are_refused_before_the_router() {
    let srv = start().await;
    let port = srv.http_addr.port();
    let wrong_port = port.wrapping_add(1);
    let refusals = [
        // Non-loopback hosts, bound-port and not.
        format!("http://attacker.example:{port}"),
        "http://10.0.0.9:8025".to_string(),
        format!("http://[::2]:{port}"),
        // Wrong port on a loopback host.
        format!("http://localhost:{wrong_port}"),
        format!("http://[::1]:{wrong_port}"),
        // Malformed or port-less spellings: nothing to match the bound
        // port against, so nothing passes. The unclosed bracket is the
        // intended spelling, not a typo'd literal.
        "http://localhost".to_string(),
        "http://localhost:99999".to_string(),
        format!("http://[::1:{port}"), // unclosed bracket
        "localhost:8025".to_string(),
        "null".to_string(),
        // The path is not the authority: the host behind the path refuses.
        format!("http://attacker.example:{port}/evil"),
    ];
    for origin in refusals {
        for (method, path) in [
            ("GET", "/api/v1/inboxes"),
            ("DELETE", "/api/v1/inboxes/guarded"),
            ("POST", "/mcp"),
        ] {
            let (st, body) = http_send_text_with_headers(
                srv.http_addr,
                method,
                path,
                &[("Origin", origin.as_str())],
                None,
            )
            .await;
            assert_eq!(st, 403, "{method} {path} with Origin {origin:?}: {body}");
            assert!(
                body.contains("cross-origin requests are refused on the loopback plane"),
                "{origin}: {body}"
            );
        }
    }
}

/// The DNS-rebinding leg (0030 §2): a rebound page sends *same-origin*
/// requests — and same-origin GET/HEADs carry no `Origin` at all — but its
/// `Host` line still spells the attacker's hostname. While loopback-bound,
/// a `Host` that is not a loopback spelling of the bound port is refused
/// (`403`, its own sentence), on reads and writes alike; a loopback
/// spelling rides through; a request with no `Host` line at all has
/// nothing to admit and is refused.
#[tokio::test]
async fn a_rebound_host_refuses_even_without_an_origin() {
    let srv = start().await;
    let port = srv.http_addr.port();

    // The rebind: no Origin header, the Host still the attacker's.
    for (method, path, body) in [
        ("GET", "/api/v1/inboxes", None),
        ("GET", "/api/v1/messages", None),
        (
            "POST",
            "/mcp",
            Some(r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#),
        ),
    ] {
        let (st, text) = send_with_host(
            srv.http_addr,
            method,
            path,
            &format!("attacker.example:{port}"),
            body,
        )
        .await;
        assert_eq!(st, 403, "{method} {path}: {text}");
        assert!(
            text.contains("must name a loopback Host"),
            "{method} {path}: {text}"
        );
    }

    // Every loopback spelling of the bound port admits the request — the
    // exact set of Host lines a legitimate caller to this plane sends.
    for host in [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
        format!("127.255.0.9:{port}"),
    ] {
        let (st, text) = send_with_host(srv.http_addr, "GET", "/api/v1/inboxes", &host, None).await;
        assert_eq!(st, 200, "{host}: {text}");
    }

    // A loopback host on the wrong port is another plane.
    let (st, text) = send_with_host(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes",
        &format!("127.0.0.1:{}", port.wrapping_add(1)),
        None,
    )
    .await;
    assert_eq!(st, 403, "{text}");

    // No Host line at all: HTTP/1.1 clients always send one; a request
    // without it names no plane and is refused.
    let mut stream = tokio::net::TcpStream::connect(srv.http_addr).await.unwrap();
    use tokio::io::AsyncWriteExt;
    stream
        .write_all(b"GET /api/v1/inboxes HTTP/1.1\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buf)
        .await
        .unwrap();
    let s = String::from_utf8_lossy(&buf);
    let status: u16 = s
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(status, 403, "{s}");
    assert!(s.contains("must name a loopback Host"), "{s}");
}

/// With an explicit off-loopback bind the check is inert — the operator's
/// recorded posture (0030 §2): any `Origin` rides through and so does any
/// `Host`, because there is no loopback plane to protect.
#[tokio::test]
async fn an_off_loopback_bind_leaves_the_guard_inert() {
    let srv = start_with(|mut c| {
        c.http_listen = "0.0.0.0:0".into();
        c
    })
    .await;
    let (st, body) = http_send_text_with_headers(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes",
        &[("Origin", "http://attacker.example:1")],
        None,
    )
    .await;
    assert_eq!(st, 200, "the guard is inert off loopback: {body}");
    let (st, body) = send_with_host(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes",
        "attacker.example:1",
        None,
    )
    .await;
    assert_eq!(st, 200, "the Host leg is inert off loopback: {body}");
}
