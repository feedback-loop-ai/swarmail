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

async fn inbox_snapshot(
    addr: std::net::SocketAddr,
    inbox: &str,
) -> (serde_json::Value, Vec<Vec<u8>>) {
    let (status, body) = http_json(
        addr,
        "GET",
        &format!("/api/v1/inboxes/{inbox}/messages"),
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let mut raw = Vec::new();
    for email in body["emails"].as_array().unwrap() {
        let id = email["id"].as_str().unwrap();
        let (status, bytes) = http_bytes(addr, &format!("/api/v1/messages/{id}/raw")).await;
        assert_eq!(status, 200, "raw bytes for {id}");
        raw.push(bytes);
    }
    (body, raw)
}

async fn assert_named_query_refusal(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    route: &str,
    key: &str,
    supplied_value: &str,
) {
    let (status, body) = http_json(addr, method, path, None).await;
    assert_eq!(status, 400, "{method} {path}: {body}");
    let error = body["error"].as_str().unwrap_or_else(|| panic!("{body}"));
    assert!(error.contains(&format!("invalid {route} query:")), "{body}");
    assert!(error.contains(&format!("`{key}`")), "{body}");
    if !supplied_value.is_empty() {
        assert!(
            !error.contains(supplied_value),
            "query values must not be echoed: {body}"
        );
    }
}

async fn assert_named_query_refusal_with_body(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body_text: &str,
    route: &str,
    key: &str,
) {
    let (status, body) = http_json(addr, method, path, Some(body_text)).await;
    assert_eq!(status, 400, "{method} {path}: {body}");
    let error = body["error"].as_str().unwrap_or_else(|| panic!("{body}"));
    assert!(error.contains(&format!("invalid {route} query:")), "{body}");
    assert!(error.contains(&format!("`{key}`")), "{body}");
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
async fn unknown_query_deletes_preserve_exact_mail() {
    let srv = start().await;
    let addr = srv.http_addr;
    seed(addr, "scope-a", "a@example.com").await;
    seed(addr, "scope-b", "b@example.com").await;
    let before_a = inbox_snapshot(addr, "scope-a").await;
    let before_b = inbox_snapshot(addr, "scope-b").await;
    let id_a = before_a.0["emails"][0]["id"].as_str().unwrap().to_string();

    for (method, path, route, key, value) in [
        (
            "DELETE",
            "/api/v1/inboxes/scope-a/messages?subject=keep-this",
            "DELETE /api/v1/inboxes/{inbox}/messages",
            "subject",
            "keep-this",
        ),
        (
            "DELETE",
            "/api/v1/delete-all?inbox=scope-a",
            "DELETE /api/v1/delete-all",
            "inbox",
            "scope-a",
        ),
    ] {
        assert_named_query_refusal(addr, method, path, route, key, value).await;
        assert_eq!(inbox_snapshot(addr, "scope-a").await, before_a);
        assert_eq!(inbox_snapshot(addr, "scope-b").await, before_b);
    }
    assert_named_query_refusal(
        addr,
        "DELETE",
        &format!("/api/v1/messages/{id_a}?inbox=scope-a"),
        "DELETE /api/v1/messages/{id}",
        "inbox",
        "scope-a",
    )
    .await;
    assert_eq!(inbox_snapshot(addr, "scope-a").await, before_a);
    assert_eq!(inbox_snapshot(addr, "scope-b").await, before_b);

    let (status, deleted) =
        http_json(addr, "DELETE", &format!("/api/v1/messages/{id_a}"), None).await;
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(deleted["deleted"], true);
    assert_eq!(
        inbox_snapshot(addr, "scope-a").await.0["emails"],
        serde_json::json!([])
    );
    assert_eq!(inbox_snapshot(addr, "scope-b").await, before_b);
    seed(addr, "scope-a", "a@example.com").await;

    let (status, cleared) =
        http_json(addr, "DELETE", "/api/v1/inboxes/scope-a/messages", None).await;
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["removed"], 1);
    assert_eq!(
        inbox_snapshot(addr, "scope-a").await.0["emails"],
        serde_json::json!([])
    );
    assert_eq!(inbox_snapshot(addr, "scope-b").await, before_b);

    seed(addr, "scope-a", "a@example.com").await;
    let (status, removed) = http_json(addr, "DELETE", "/api/v1/delete-all?", None).await;
    assert_eq!(status, 200, "{removed}");
    assert_eq!(removed["removed"], 2);
    assert_eq!(
        inbox_snapshot(addr, "scope-a").await.0["emails"],
        serde_json::json!([])
    );
    assert_eq!(
        inbox_snapshot(addr, "scope-b").await.0["emails"],
        serde_json::json!([])
    );
}

#[tokio::test]
async fn query_free_native_operations_refuse_by_name() {
    let srv = start().await;
    let addr = srv.http_addr;
    let mail = seed(addr, "matrix", "matrix@example.com").await;
    let id = mail["id"].as_str().unwrap().to_string();
    seed(addr, "clear-target", "clear@example.com").await;

    let (st, chaos) = http_json(
        addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data":{"probability":0,"error":"451 4.3.0 test"}}"#),
    )
    .await;
    assert_eq!(st, 200, "{chaos}");
    let webhook_body =
        r#"[{"url":"http://user:pw@127.0.0.1:9/hook","inbox":"matrix","secret":"kept-secret"}]"#;
    let (st, installed) = http_json(addr, "PUT", "/api/v1/webhooks", Some(webhook_body)).await;
    assert_eq!(st, 200, "{installed}");

    let rows = [
        (
            "DELETE",
            "/api/v1/inboxes/clear-target/messages",
            "DELETE /api/v1/inboxes/{inbox}/messages",
            "subject",
            "clear-target",
        ),
        (
            "DELETE",
            "/api/v1/delete-all",
            "DELETE /api/v1/delete-all",
            "inbox",
            "matrix",
        ),
        (
            "GET",
            &format!("/api/v1/messages/{id}"),
            "GET /api/v1/messages/{id}",
            "subject",
            "matrix",
        ),
        (
            "DELETE",
            &format!("/api/v1/messages/{id}"),
            "DELETE /api/v1/messages/{id}",
            "inbox",
            "matrix",
        ),
        (
            "GET",
            &format!("/api/v1/messages/{id}/raw"),
            "GET /api/v1/messages/{id}/raw",
            "subject",
            "matrix",
        ),
        ("GET", "/api/v1/chaos", "GET /api/v1/chaos", "event", "data"),
        (
            "DELETE",
            "/api/v1/chaos",
            "DELETE /api/v1/chaos",
            "event",
            "data",
        ),
        (
            "GET",
            "/api/v1/webhooks",
            "GET /api/v1/webhooks",
            "inbox",
            "matrix",
        ),
        (
            "DELETE",
            "/api/v1/webhooks",
            "DELETE /api/v1/webhooks",
            "inbox",
            "matrix",
        ),
    ];
    for (method, route_path, label, selector, value) in rows {
        let selected = format!("{route_path}?{selector}={value}");
        assert_named_query_refusal(addr, method, &selected, label, selector, value).await;
        let typo = format!("{route_path}?typo=");
        assert_named_query_refusal(addr, method, &typo, label, "typo", "").await;
    }
    let seed_body = r#"{"to":"new@example.com","subject":"query proof","text":"Code 515151"}"#;
    for path in [
        "/api/v1/inboxes/matrix-new/seed?inbox=matrix-new",
        "/api/v1/inboxes/matrix-new/seed?typo=",
    ] {
        assert_named_query_refusal_with_body(
            addr,
            "POST",
            path,
            seed_body,
            "POST /api/v1/inboxes/{inbox}/seed",
            if path.contains("?typo") {
                "typo"
            } else {
                "inbox"
            },
        )
        .await;
    }

    // Valid message read/raw contracts work with no query and an empty query.
    for suffix in ["", "?"] {
        let (status, message) =
            http_json(addr, "GET", &format!("/api/v1/messages/{id}{suffix}"), None).await;
        assert_eq!(status, 200, "{message}");
        assert_eq!(message["id"], id);
    }
    let (status, raw_before) = http_bytes(addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(status, 200);
    let (status, raw_empty) = http_bytes(addr, &format!("/api/v1/messages/{id}/raw?")).await;
    assert_eq!(status, 200);
    assert_eq!(raw_empty, raw_before);

    for suffix in ["", "?"] {
        let (status, config) =
            http_json(addr, "GET", &format!("/api/v1/chaos{suffix}"), None).await;
        assert_eq!(status, 200, "{config}");
        assert_eq!(config, chaos);
        let (status, targets) =
            http_json(addr, "GET", &format!("/api/v1/webhooks{suffix}"), None).await;
        assert_eq!(status, 200, "{targets}");
        assert_eq!(targets[0]["secret"], serde_json::Value::Null);
        assert_eq!(targets[0]["url"], "http://127.0.0.1:9/hook");
    }

    for (suffix, query_value) in [("", ""), ("?", "")] {
        let seeded = format!("/api/v1/inboxes/matrix-control/seed{suffix}");
        let (status, made) = http_json(addr, "POST", &seeded, Some(seed_body)).await;
        assert_eq!(status, 200, "{made}");
        assert_eq!(made["codes"], serde_json::json!(["515151"]));
        assert_eq!(made["inbox"], "matrix-control");
        assert_eq!(query_value, "");
    }

    let (status, cleared) = http_json(
        addr,
        "DELETE",
        "/api/v1/inboxes/clear-target/messages",
        None,
    )
    .await;
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["removed"], 1);
    seed(addr, "clear-target", "clear@example.com").await;
    let (status, cleared) = http_json(
        addr,
        "DELETE",
        "/api/v1/inboxes/clear-target/messages?",
        None,
    )
    .await;
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["removed"], 1);

    let (status, deleted) =
        http_json(addr, "DELETE", &format!("/api/v1/messages/{id}"), None).await;
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(deleted["deleted"], true);
    let new_mail = seed(addr, "matrix", "matrix@example.com").await;
    let new_id = new_mail["id"].as_str().unwrap();
    let (status, deleted) =
        http_json(addr, "DELETE", &format!("/api/v1/messages/{new_id}?"), None).await;
    assert_eq!(status, 200, "{deleted}");
    assert_eq!(deleted["deleted"], true);

    for suffix in ["", "?"] {
        let (status, result) =
            http_json(addr, "DELETE", &format!("/api/v1/chaos{suffix}"), None).await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["data"], serde_json::Value::Null);
        if suffix.is_empty() {
            let (status, restored) = http_json(
                addr,
                "PUT",
                "/api/v1/chaos",
                Some(r#"{"data":{"probability":0}}"#),
            )
            .await;
            assert_eq!(status, 200, "{restored}");
        }
    }
    for suffix in ["", "?"] {
        let (status, result) =
            http_json(addr, "DELETE", &format!("/api/v1/webhooks{suffix}"), None).await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["cleared"], true);
        if suffix.is_empty() {
            let (status, restored) =
                http_json(addr, "PUT", "/api/v1/webhooks", Some(webhook_body)).await;
            assert_eq!(status, 200, "{restored}");
        }
    }

    let (status, wiped) = http_json(addr, "DELETE", "/api/v1/delete-all", None).await;
    assert_eq!(status, 200, "{wiped}");
    assert_eq!(wiped["removed"], 2); // only the two matrix-control seeds remain
    let (status, one) = http_json(
        addr,
        "POST",
        "/api/v1/inboxes/wipe-control/seed",
        Some(seed_body),
    )
    .await;
    assert_eq!(status, 200, "{one}");
    let (status, wiped) = http_json(addr, "DELETE", "/api/v1/delete-all?", None).await;
    assert_eq!(status, 200, "{wiped}");
    assert_eq!(wiped["removed"], 1);
}

#[tokio::test]
async fn native_empty_query_shape_keeps_form_decoding() {
    let srv = start().await;
    let addr = srv.http_addr;
    seed(addr, "decode-a", "a@example.com").await;
    seed(addr, "decode-b", "b@example.com").await;
    let before_a = inbox_snapshot(addr, "decode-a").await;
    let before_b = inbox_snapshot(addr, "decode-b").await;
    for (query, key, value) in [
        ("%69nbox=not-a-filter", "inbox", "not-a-filter"),
        ("inbox", "inbox", ""),
        ("inbox=first&inbox=second", "inbox", ""),
        ("=empty-key-value", "", "empty-key-value"),
    ] {
        assert_named_query_refusal(
            addr,
            "DELETE",
            &format!("/api/v1/delete-all?{query}"),
            "DELETE /api/v1/delete-all",
            key,
            value,
        )
        .await;
        assert_eq!(inbox_snapshot(addr, "decode-a").await, before_a);
        assert_eq!(inbox_snapshot(addr, "decode-b").await, before_b);
    }

    // The existing form decoder treats separator-only input as no fields.
    let (status, empty) = http_json(addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(status, 200, "{empty}");
    let (status, separators) = http_json(addr, "GET", "/api/v1/chaos?&&", None).await;
    assert_eq!(status, 200, "{separators}");
    assert_eq!(separators, empty);
}

fn generated_ca_pem() -> String {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    params.self_signed(&key).unwrap().pem()
}

#[tokio::test]
async fn unknown_query_config_deletes_preserve_complete_configuration() {
    let srv = start().await;
    let addr = srv.http_addr;
    let ca_pem = generated_ca_pem();
    let targets = serde_json::json!([
        {
            "url": "http://user-one:password-one@127.0.0.1:9/first",
            "inbox": "only-first",
            "secret": "secret-one",
            "ca_pem": ca_pem,
        },
        {
            "url": "http://user-two:password-two@127.0.0.1:10/second",
            "inbox": "only-second",
            "secret": "secret-two",
        }
    ]);
    let target_body = targets.to_string();
    let (status, installed) = http_json(addr, "PUT", "/api/v1/webhooks", Some(&target_body)).await;
    assert_eq!(status, 200, "{installed}");
    let target_snapshot = || {
        srv.webhooks
            .current()
            .into_iter()
            .map(|target| (target.url, target.inbox, target.secret, target.ca_pem))
            .collect::<Vec<_>>()
    };
    let before_targets = target_snapshot();
    assert_eq!(before_targets.len(), 2);
    assert!(before_targets[0].0.contains("user-one:password-one@"));
    assert_eq!(before_targets[0].1.as_deref(), Some("only-first"));
    assert_eq!(before_targets[0].2.as_deref(), Some("secret-one"));
    assert!(
        before_targets[0]
            .3
            .as_deref()
            .unwrap()
            .contains("BEGIN CERTIFICATE")
    );
    assert_eq!(before_targets[1].2.as_deref(), Some("secret-two"));

    let (status, public_targets) = http_json(addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(status, 200, "{public_targets}");
    assert_eq!(public_targets[0]["secret"], serde_json::Value::Null);
    assert_eq!(public_targets[0]["url"], "http://127.0.0.1:9/first");
    assert!(
        public_targets[0]["ca_pem"]
            .as_str()
            .unwrap()
            .contains("BEGIN CERTIFICATE")
    );

    assert_named_query_refusal(
        addr,
        "DELETE",
        "/api/v1/webhooks?inbox=only-first",
        "DELETE /api/v1/webhooks",
        "inbox",
        "only-first",
    )
    .await;
    assert_eq!(target_snapshot(), before_targets);
    let (status, public_after) = http_json(addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(status, 200, "{public_after}");
    assert_eq!(public_after, public_targets);

    let chaos_body =
        r#"{"connect":{"probability":0},"data":{"probability":100,"error":"451 4.3.0 retained"}}"#;
    let (status, chaos_before) = http_json(addr, "PUT", "/api/v1/chaos", Some(chaos_body)).await;
    assert_eq!(status, 200, "{chaos_before}");
    assert_eq!(srv.store.emails_inserted(), 0);
    assert_named_query_refusal(
        addr,
        "DELETE",
        "/api/v1/chaos?event=data",
        "DELETE /api/v1/chaos",
        "event",
        "data",
    )
    .await;
    assert_eq!(
        serde_json::to_value(srv.chaos.current()).unwrap(),
        chaos_before
    );
    let (status, chaos_after) = http_json(addr, "GET", "/api/v1/chaos", None).await;
    assert_eq!(status, 200, "{chaos_after}");
    assert_eq!(chaos_after, chaos_before);

    let refused = common::smtp_send(
        srv.smtp_addr,
        Some("chaos-kept"),
        "sender@example.com",
        "recipient@example.com",
        "must refuse",
        "no insertion",
    )
    .await
    .unwrap_err();
    assert!(refused.contains("451 4.3.0 retained"), "{refused}");
    assert_eq!(srv.store.emails_inserted(), 0);
    assert!(!srv.store.has_inbox("chaos-kept"));

    let (status, cleared) = http_json(addr, "DELETE", "/api/v1/chaos", None).await;
    assert_eq!(status, 200, "{cleared}");
    assert!(srv.chaos.current().data.is_none());
    assert!(srv.chaos.current().connect.is_none());
    let accepted = common::smtp_send(
        srv.smtp_addr,
        Some("chaos-kept"),
        "sender@example.com",
        "recipient@example.com",
        "stored after clear",
        "normal delivery",
    )
    .await
    .unwrap();
    assert!(accepted.starts_with("250"), "{accepted}");
    assert_eq!(srv.store.emails_inserted(), 1);
    assert!(srv.store.has_inbox("chaos-kept"));

    let (status, cleared) = http_json(addr, "DELETE", "/api/v1/webhooks", None).await;
    assert_eq!(status, 200, "{cleared}");
    assert_eq!(cleared["cleared"], true);
    assert!(srv.webhooks.current().is_empty());
    srv.stop().await;
}

#[tokio::test]
async fn unknown_seed_query_has_no_ingest_effects() {
    let srv = start_with(|mut config| {
        config.max_per_inbox = 1;
        config
    })
    .await;
    let addr = srv.http_addr;
    seed(addr, "existing", "old@example.com").await;
    let before_existing = inbox_snapshot(addr, "existing").await;
    let before_inboxes = srv.store.inboxes();
    let inserted = srv.store.emails_inserted();
    let dropped = srv.store.emails_dropped();
    let body = r#"{"to":"new@example.com","subject":"not stored","text":"Code 616161"}"#;

    assert_named_query_refusal_with_body(
        addr,
        "POST",
        "/api/v1/inboxes/existing/seed?to=new@example.com",
        body,
        "POST /api/v1/inboxes/{inbox}/seed",
        "to",
    )
    .await;
    assert_eq!(inbox_snapshot(addr, "existing").await, before_existing);
    assert_eq!(srv.store.inboxes(), before_inboxes);
    assert_eq!(srv.store.emails_inserted(), inserted);
    assert_eq!(srv.store.emails_dropped(), dropped);

    assert_named_query_refusal_with_body(
        addr,
        "POST",
        "/api/v1/inboxes/absent/seed?typo=",
        body,
        "POST /api/v1/inboxes/{inbox}/seed",
        "typo",
    )
    .await;
    assert_eq!(srv.store.inboxes(), before_inboxes);
    assert_eq!(srv.store.emails_inserted(), inserted);
    assert_eq!(srv.store.emails_dropped(), dropped);
    assert!(!srv.store.has_inbox("absent"));

    for (path, inbox) in [
        ("/api/v1/inboxes/accepted/seed", "accepted"),
        ("/api/v1/inboxes/empty-query/seed?", "empty-query"),
    ] {
        let (status, email) = http_json(addr, "POST", path, Some(body)).await;
        assert_eq!(status, 200, "{email}");
        assert_eq!(email["inbox"], inbox);
        assert_eq!(email["subject"], "not stored");
        assert_eq!(email["codes"], serde_json::json!(["616161"]));
        let id = email["id"].as_str().unwrap();
        let (status, stored) =
            http_json(addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
        assert_eq!(status, 200, "{stored}");
        assert_eq!(stored, email, "a successful seed is queryable immediately");
    }
    assert_eq!(srv.store.emails_inserted(), inserted + 2);
    assert_eq!(srv.store.emails_dropped(), dropped);
}

#[tokio::test]
async fn native_query_refusal_preserves_error_precedence() {
    let srv = start().await;
    let addr = srv.http_addr;

    for (method, path, route) in [
        (
            "GET",
            "/api/v1/messages/missing?typo=sentinel",
            "GET /api/v1/messages/{id}",
        ),
        (
            "DELETE",
            "/api/v1/messages/missing?typo=sentinel",
            "DELETE /api/v1/messages/{id}",
        ),
        (
            "GET",
            "/api/v1/messages/missing/raw?typo=sentinel",
            "GET /api/v1/messages/{id}/raw",
        ),
    ] {
        assert_named_query_refusal(addr, method, path, route, "typo", "sentinel").await;
    }
    for (method, path) in [
        ("GET", "/api/v1/messages/missing"),
        ("GET", "/api/v1/messages/missing/raw"),
    ] {
        let (status, body) = http_json(addr, method, path, None).await;
        assert_eq!(status, 404, "{method} {path}: {body}");
        assert_eq!(body["error"], "message not found");
    }
    let (status, deleted) = http_json(addr, "DELETE", "/api/v1/messages/missing", None).await;
    assert_eq!(status, 404, "{deleted}");
    assert_eq!(deleted["error"], "message not found");

    let (status, blank) = http_json(
        addr,
        "DELETE",
        "/api/v1/inboxes/%20/messages?subject=selector",
        None,
    )
    .await;
    assert_eq!(status, 400, "{blank}");
    assert!(
        blank["error"].as_str().unwrap().contains("inbox"),
        "{blank}"
    );
    assert!(
        !blank["error"].as_str().unwrap().contains("invalid DELETE"),
        "{blank}"
    );

    let (status, origin) = http_send_text_with_headers(
        addr,
        "GET",
        "/api/v1/messages/missing?typo=sentinel",
        &[("Origin", "https://evil.example")],
        None,
    )
    .await;
    assert_eq!(status, 403, "{origin}");
    assert!(!origin.contains("invalid GET /api/v1/messages"), "{origin}");
    let (status, host) = send_with_host(
        addr,
        "GET",
        "/api/v1/messages/missing?typo=sentinel",
        "attacker.example",
        None,
    )
    .await;
    assert_eq!(status, 403, "{host}");
    assert!(!host.contains("invalid GET /api/v1/messages"), "{host}");

    let malformed = raw_http(
        addr,
        "POST /api/v1/inboxes/body/seed?typo=sentinel HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 1\r\n",
        b"{",
    )
    .await;
    assert_eq!(malformed.0, 400, "{}", malformed.1);
    assert!(
        !malformed.1.contains("invalid POST /api/v1/inboxes"),
        "{}",
        malformed.1
    );

    let wrong_media = raw_http(
        addr,
        "POST /api/v1/inboxes/body/seed?typo=sentinel HTTP/1.1\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n",
        b"{}",
    )
    .await;
    assert_eq!(wrong_media.0, 415, "{}", wrong_media.1);
    assert!(
        !wrong_media.1.contains("invalid POST /api/v1/inboxes"),
        "{}",
        wrong_media.1
    );

    let too_large = vec![b'x'; 2_097_153];
    let too_large_response = raw_http(
        addr,
        &format!("POST /api/v1/inboxes/body/seed?typo=sentinel HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", too_large.len()),
        &too_large,
    )
    .await;
    assert_eq!(too_large_response.0, 413, "{}", too_large_response.1);
    assert!(
        !too_large_response
            .1
            .contains("invalid POST /api/v1/inboxes"),
        "{}",
        too_large_response.1
    );

    let (status, data_shape) = http_json(
        addr,
        "POST",
        "/api/v1/inboxes/body/seed?typo=sentinel",
        Some(r#"{"to":"valid@example.com","subjet":"wrong"}"#),
    )
    .await;
    assert_eq!(status, 400, "{data_shape}");
    assert!(
        data_shape["error"]
            .as_str()
            .unwrap()
            .starts_with("invalid seed body:"),
        "{data_shape}"
    );

    let (status, domain) = http_json(
        addr,
        "POST",
        "/api/v1/inboxes/body/seed?typo=sentinel",
        Some(r#"{"to":"bad\r\naddress@example.com"}"#),
    )
    .await;
    assert_eq!(status, 400, "{domain}");
    assert!(
        domain["error"]
            .as_str()
            .unwrap()
            .starts_with("invalid POST /api/v1/inboxes/{inbox}/seed query:"),
        "{domain}"
    );
    assert!(
        !domain["error"].as_str().unwrap().contains("sentinel"),
        "{domain}"
    );
    assert!(!srv.store.has_inbox("body"));
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
            // The code sits in its own paragraph — a visible text node the
            // renderer keeps standalone — and the href's `&amp;` is the
            // sender's `&`: decoded per URL at ingest (issue #39).
            r#"{"to": "h@x.io", "subject": "hi", "html": "<p><a href='https://q.io?a=1&amp;b=2'>go</a></p><p>Your code is 555555.</p>"}"#,
        ),
    )
    .await;
    assert_eq!(st, 200);
    assert!(body["html"].as_str().unwrap().contains("https://q.io"));
    assert_eq!(
        body["links"][0], "https://q.io?a=1&b=2",
        "href entities decoded"
    );
    assert_eq!(body["codes"][0], "555555");
}

/// The same decode over real SMTP: an `&amp;` and a `&#38;` inside an href
/// are the sender's `&` (issue #39), decoded per URL at ingest — the seed
/// route and the SMTP parse path run the same extraction.
#[tokio::test]
async fn smtp_html_hrefs_decode_character_references() {
    let srv = start().await;
    let mut conn = SmtpConn::connect(srv.smtp_addr).await;
    conn.send("EHLO swarmail-extract-test").await;
    conn.reply_multi().await;
    let plain = "\u{0}amp\u{0}whatever";
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    assert!(conn.reply().await.starts_with("235"));
    conn.send("MAIL FROM:<sender@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    conn.send("RCPT TO:<amp@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    let data = "From: sender@x.io\r\nTo: amp@x.io\r\nSubject: refs\r\n\
                Content-Type: text/html\r\n\r\n\
                <p><a href='https://q.io?a=1&amp;b=2'>named</a></p>\
                <p><a href='https://r.io?x=1&#38;y=2'>numeric</a></p>\r\n";
    let reply = conn.data(data).await;
    assert!(reply.starts_with("250"), "DATA refused: {reply}");

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/amp/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        body["emails"][0]["links"],
        serde_json::json!(["https://q.io?a=1&b=2", "https://r.io?x=1&y=2"]),
        "both reference forms decode to the sender's &"
    );
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
    // `matched` stays honest about the filter. The bound is well past
    // poll+send+commit, so the mail cannot outrun the wait on a loaded
    // runner (issue #102); the outer deadline still fails fast.
    let request = tokio::spawn(http_json(
        addr,
        "GET",
        "/api/v1/inboxes/observed408/await?count=1&to=nobody@x.io&timeout_ms=5000",
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

async fn next177_registered(addr: std::net::SocketAddr, inbox: &str) -> u64 {
    let (st, metrics) = http_get_text(addr, "/metrics").await;
    assert_eq!(st, 200);
    common::metric_row_value(
        &metrics,
        &format!("swarmail_awaits_registered_total{{inbox=\"{inbox}\"}}"),
    )
    .expect("the exclusive inbox must have an attributable cumulative row")
}

/// Supplied malformed `timeout_ms` values keep REST's parse-time 400 — the
/// query layer refuses before the handler runs, so no waiter is ever
/// registered — while `0` is a legal, real zero deadline: an immediate 408
/// on an empty inbox, never a clamp to the default and never a refusal
/// (decision 0025).
#[tokio::test]
async fn await_supplied_malformed_timeouts_are_parse_refusals_and_zero_is_a_real_deadline() {
    let srv = start().await;
    let inbox = "next177-rest-malformed";
    let zero = format!("/api/v1/inboxes/{inbox}/await?count=1&timeout_ms=0");
    let (st, body) = http_json(srv.http_addr, "GET", &zero, None).await;
    assert_eq!(st, 408, "{body}");
    assert_eq!(body["matched"], 0);
    assert_eq!(next177_registered(srv.http_addr, inbox).await, 1);
    for value in ["-1", "invalid", "null", "1.5", "60001"] {
        let before = next177_registered(srv.http_addr, inbox).await;
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{inbox}/await?count=1&timeout_ms={value}"),
            None,
        )
        .await;
        assert_eq!(st, 400, "{value}: {body}");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains(if value == "60001" { "60000" } else { "await" }),
            "{value}: {body}"
        );
        assert_eq!(
            next177_registered(srv.http_addr, inbox).await,
            before,
            "refusal registered: {value}"
        );
    }
    let before = next177_registered(srv.http_addr, inbox).await;
    let (st, body) = http_json(srv.http_addr, "GET", &zero, None).await;
    assert_eq!(st, 408, "{body}");
    assert_eq!(body["matched"], 0);
    assert_eq!(next177_registered(srv.http_addr, inbox).await, before + 1);
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(
        common::metric_row_value(
            &metrics,
            &format!("swarmail_awaits_active{{inbox=\"{inbox}\"}}")
        ),
        None
    );
}

/// The filters the list endpoint accepts parse identically on await and
/// assert (issue #38): under the old `#[serde(flatten)]` the urlencoded
/// layer handed every flattened field over as a string and serde refused
/// the integer, so `since_ms` was refused on both while list accepted it.
/// The flat struct (decision 0029 §1) makes the parameter set explicit — a
/// fixture-derived `since_ms` includes equality and excludes the next stamp
/// on list, await and assert; a malformed
/// value keeps the parse refusal naming the field (decision 0025).
#[tokio::test]
async fn await_and_assert_parse_since_ms_exactly_as_list_does() {
    let srv = start().await;
    let fixture = seed(srv.http_addr, "flt", "user@example.com").await;
    let stamp = fixture["received_ms"]
        .as_i64()
        .expect("seed's actual parse-start stamp");
    let id = fixture["id"].as_str().unwrap();
    let after = stamp.checked_add(1).unwrap();
    for (boundary, matched) in [(stamp, 1), (after, 0)] {
        let (st, list) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/flt/messages?to=user@example.com&since_ms={boundary}"),
            None,
        )
        .await;
        assert_eq!(st, 200);
        assert_eq!(list["total"], matched);
        let emails = list["emails"].as_array().unwrap();
        assert_eq!(emails.len(), matched as usize);
        if matched == 1 {
            assert_eq!(emails[0]["id"], id);
        }
        let (st, body) = http_json(srv.http_addr, "GET",
            &format!("/api/v1/inboxes/flt/await?count=1&to=user@example.com&since_ms={boundary}&timeout_ms=0"), None).await;
        assert_eq!(st, if matched == 1 { 200 } else { 408 }, "{body}");
        assert_eq!(body["matched"], matched);
        if matched == 1 {
            assert_eq!(body["emails"][0]["id"], id);
        }
        let (st, body) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/flt/assert?count=1&to=user@example.com&since_ms={boundary}"),
            None,
        )
        .await;
        assert_eq!(st, if matched == 1 { 200 } else { 409 }, "{body}");
        assert_eq!(body["matched"], matched);
    }

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
        "/api/v1/inboxes/flt/await?count=2&limit=1&to=user@example.com",
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
    // The cumulative registration is the observable (issue #102): the
    // delta across the refused calls must stay zero — a wait that
    // subscribed before validating would leave a permanent +1, which the
    // released gauge cannot show (its row is gone either way).
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    let registered_before = common::metric_row_value(
        &metrics,
        r#"swarmail_awaits_registered_total{inbox="nocap"}"#,
    )
    .unwrap_or(0);
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
    let (_, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    let registered_after = common::metric_row_value(
        &metrics,
        r#"swarmail_awaits_registered_total{inbox="nocap"}"#,
    )
    .unwrap_or(0);
    assert_eq!(
        registered_after, registered_before,
        "a refused wait registers no waiter: {metrics}"
    );

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

/// With an explicit off-loopback bind the Host allowlist is inactive — the
/// operator's recorded posture (0030 §2): any `Host` rides through without
/// an `Origin`, because there is no loopback plane to rebind to. An
/// `Origin` naming that same Host authority passes (decision 0043).
#[tokio::test]
async fn an_off_loopback_bind_leaves_the_host_allowlist_inactive() {
    let srv = start_off_loopback().await;
    let (st, body) = send_with_host(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes",
        "attacker.example:1",
        None,
    )
    .await;
    assert_eq!(st, 200, "the Host leg is inactive off loopback: {body}");
    // The helper's Host line names the address it dials.
    let origin = format!("http://{}", srv.http_addr);
    let (st, body) = http_send_text_with_headers(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes",
        &[("Origin", origin.as_str())],
        None,
    )
    .await;
    assert_eq!(st, 200, "a same-authority Origin passes: {body}");
}

// ---- run 20: the HTTP plane's edges (issues #88, #89, #92; decision 0043) ----

/// The refusal sentence off loopback (decision 0043 §4).
const MISMATCH: &str = "cross-origin requests must match the Host authority";

/// A server whose HTTP listener is really bound to `0.0.0.0:0` through
/// `run_on`, so the posture comes from lib.rs's own derivation — not a
/// hand-disabled guard. The reported address is asserted non-loopback.
async fn start_off_loopback() -> swarmail::RunningServer {
    let srv = start_with(|mut c| {
        c.http_listen = "0.0.0.0:0".into();
        c
    })
    .await;
    assert!(
        srv.http_addr.ip().is_unspecified(),
        "the listener is off loopback: {}",
        srv.http_addr
    );
    srv
}

/// The address a client dials to reach an all-interfaces listener.
fn dial(srv: &swarmail::RunningServer) -> std::net::SocketAddr {
    std::net::SocketAddr::from(([127, 0, 0, 1], srv.http_addr.port()))
}

/// One raw HTTP/1.1 exchange, the head written byte for byte: no Host or
/// Content-Type is added behind the caller's back, so an exact Host, a
/// repeated header or a `text/plain` body rides as spelled. Returns the
/// status, the response head and the body.
async fn raw_exchange(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    headers: &[&str],
    body: &str,
) -> (u16, String, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut req = format!("{method} {path} HTTP/1.1\r\nConnection: close\r\n");
    for header in headers {
        req.push_str(header);
        req.push_str("\r\n");
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(common::HELPER_DEADLINE, stream.read_to_end(&mut buf))
        .await
        .expect("the server answers within the helper deadline")
        .expect("the response reads to EOF");
    let text = String::from_utf8_lossy(&buf).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").expect("a complete response");
    let status = head
        .split(' ')
        .nth(1)
        .expect("a status line")
        .parse()
        .expect("a numeric status");
    (status, head.to_string(), body.to_string())
}

/// The issue's drive, verbatim: armed chaos at probability 100.
const EVIL_SET_CHAOS: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_set_chaos","arguments":{"event":"connect","probability":100,"error":"554 pwned"}}}"#;

/// The chaos configuration as REST reads it — the before/after witness.
async fn chaos_now(addr: std::net::SocketAddr, host: &str) -> serde_json::Value {
    let (st, _, body) = raw_exchange(
        addr,
        "GET",
        "/api/v1/chaos",
        &[format!("Host: {host}").as_str()],
        "",
    )
    .await;
    assert_eq!(st, 200, "{body}");
    serde_json::from_str(&body).expect("chaos is JSON")
}

/// Issue #89's reproduction (AC4): on a real `0.0.0.0` bind, the
/// cross-site `text/plain` MCP POST from `https://evil.example` is refused
/// `403` before dispatch, and the chaos configuration read before it is
/// unchanged after. Red before 0043: the guard was off, the call answered
/// 200 and armed the rule. The same request without `Origin` — curl's
/// shape — is still dispatched and arms it (AC5), then is cleared.
#[tokio::test]
async fn an_off_loopback_bind_refuses_the_cross_site_mcp_drive() {
    let srv = start_off_loopback().await;
    let addr = dial(&srv);
    let host_line = format!("Host: 127.0.0.1:{}", addr.port());
    let host = host_line.as_str();
    let before = chaos_now(addr, &host[6..]).await;
    let (st, head, body) = raw_exchange(
        addr,
        "POST",
        "/mcp",
        &[
            host,
            "Origin: https://evil.example",
            "Content-Type: text/plain",
        ],
        EVIL_SET_CHAOS,
    )
    .await;
    assert_eq!(st, 403, "{head}\n{body}");
    assert!(body.contains(MISMATCH), "{body}");
    assert_eq!(
        chaos_now(addr, &host[6..]).await,
        before,
        "no effect landed"
    );

    // The control: no Origin, the same text/plain body, dispatched.
    let (st, _, body) = raw_exchange(
        addr,
        "POST",
        "/mcp",
        &[host, "Content-Type: text/plain"],
        EVIL_SET_CHAOS,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_ne!(
        chaos_now(addr, &host[6..]).await,
        before,
        "the control armed"
    );
    let (st, _, body) = raw_exchange(addr, "DELETE", "/api/v1/chaos", &[host], "").await;
    assert!(st < 300, "the control's rule is cleared: {st} {body}");
    assert_eq!(chaos_now(addr, &host[6..]).await, before);
}

/// Non-browser compatibility off loopback (AC5): with no `Origin`, a valid
/// MCP call is dispatched under every media type a client sends today —
/// JSON, text, form, none — and malformed JSON the guard admits still
/// reaches dispatch and answers its own parse error (0029 §2).
#[tokio::test]
async fn an_off_loopback_bind_keeps_every_originless_mcp_client() {
    let srv = start_off_loopback().await;
    let addr = dial(&srv);
    let host_line = format!("Host: 127.0.0.1:{}", addr.port());
    let host = host_line.as_str();
    let list = r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#;
    for content_type in [
        Some("Content-Type: application/json"),
        Some("Content-Type: text/plain"),
        Some("Content-Type: application/x-www-form-urlencoded"),
        None,
    ] {
        let mut headers = vec![host];
        headers.extend(content_type);
        let (st, _, body) = raw_exchange(addr, "POST", "/mcp", &headers, list).await;
        assert_eq!(st, 200, "{content_type:?}: {body}");
        let reply: serde_json::Value = serde_json::from_str(&body).expect("a JSON-RPC reply");
        assert_eq!(reply["id"], 7, "{content_type:?}: {body}");
        assert!(reply["result"]["tools"].is_array(), "{body}");
    }
    let (st, _, body) = raw_exchange(addr, "POST", "/mcp", &[host], "{not json").await;
    // Dispatch's own parse-error status (src/mcp.rs): 400 with the
    // -32700 object, the shape the stdio bridge forwards verbatim.
    assert_eq!(st, 400, "{body}");
    assert!(
        body.contains("-32700"),
        "dispatch owns the parse error: {body}"
    );
}

/// The off-loopback authority matrix over real HTTP (AC6–AC7): matching
/// authorities pass — same host and port, host case, bracketed IPv6,
/// explicit scheme defaults, omitted ports on both sides — and every
/// mismatch or malformed Origin refuses with the sentence. Host text is
/// free off loopback, so the client dials 127.0.0.1 and spells any Host.
#[tokio::test]
async fn an_off_loopback_origin_must_name_its_own_host_authority() {
    let srv = start_off_loopback().await;
    let addr = dial(&srv);
    for (origin, host) in [
        ("http://mail.example:8025", "mail.example:8025"),
        ("http://MAIL.example:8025", "mail.EXAMPLE:8025"),
        ("http://[::1]:8025", "[::1]:8025"),
        ("http://mail.example", "mail.example"),
        ("http://mail.example:80", "mail.example"),
        ("https://mail.example", "mail.example"),
        ("https://mail.example", "mail.example:443"),
        ("https://mail.example:443", "mail.example"),
    ] {
        let (st, _, body) = raw_exchange(
            addr,
            "GET",
            "/api/v1/inboxes",
            &[
                format!("Host: {host}").as_str(),
                format!("Origin: {origin}").as_str(),
            ],
            "",
        )
        .await;
        assert_eq!(st, 200, "{origin} + Host {host}: {body}");
    }
    for (origin, host) in [
        ("https://evil.example", "mail.example"),
        ("http://mail.example:8025", "mail.example:8026"),
        ("https://mail.example", "mail.example:8025"),
        ("https://mail.example:8025", "mail.example"),
        ("null", "mail.example"),
        ("mail.example", "mail.example"),
        ("https://user@mail.example", "mail.example"),
        ("https://mail.example/path", "mail.example"),
        ("https://mail.example:99999", "mail.example"),
        ("http://[::1", "[::1]"),
    ] {
        let (st, _, body) = raw_exchange(
            addr,
            "GET",
            "/api/v1/inboxes",
            &[
                format!("Host: {host}").as_str(),
                format!("Origin: {origin}").as_str(),
            ],
            "",
        )
        .await;
        assert_eq!(st, 403, "{origin} + Host {host}: {body}");
        assert!(body.contains(MISMATCH), "{origin} + Host {host}: {body}");
    }
    // Duplicates the HTTP parser delivers to the middleware: two Origins,
    // two Hosts. A missing Host with an Origin is HTTP/1.1-invalid.
    let origin = "Origin: https://mail.example";
    let host = "Host: mail.example";
    for headers in [[host, origin, origin], [host, host, origin]] {
        let (st, _, body) = raw_exchange(addr, "GET", "/api/v1/inboxes", &headers, "").await;
        assert!(
            (st == 403 && body.contains(MISMATCH)) || st == 400,
            "{headers:?}: {st} {body}"
        );
    }
}

/// The whole plane and the preflight off loopback (AC8): a hostile Origin
/// is refused on REST reads and writes, MCP and the UI before any effect
/// — chaos reads unchanged after an attempted PUT — and `OPTIONS /mcp`
/// answers 405 when the guard admits it and 403 when it does not, with
/// no `Access-Control-*` header on either.
#[tokio::test]
async fn an_off_loopback_hostile_origin_is_refused_on_every_route_and_preflight() {
    let srv = start_off_loopback().await;
    let addr = dial(&srv);
    let host_line = format!("Host: 127.0.0.1:{}", addr.port());
    let host = host_line.as_str();
    let evil = "Origin: https://evil.example";
    let before = chaos_now(addr, &host[6..]).await;
    let put = r#"{"rules":[{"event":"connect","probability":100}]}"#;
    for (method, path, headers, body) in [
        ("GET", "/api/v1/inboxes", vec![host, evil], ""),
        (
            "PUT",
            "/api/v1/chaos",
            vec![host, evil, "Content-Type: application/json"],
            put,
        ),
        (
            "DELETE",
            "/api/v1/inboxes/guarded/messages",
            vec![host, evil],
            "",
        ),
        (
            "POST",
            "/mcp",
            vec![host, evil, "Content-Type: text/plain"],
            EVIL_SET_CHAOS,
        ),
        ("GET", "/", vec![host, evil], ""),
    ] {
        let (st, _, text) = raw_exchange(addr, method, path, &headers, body).await;
        assert_eq!(st, 403, "{method} {path}: {text}");
        assert!(text.contains(MISMATCH), "{method} {path}: {text}");
    }
    assert_eq!(chaos_now(addr, &host[6..]).await, before, "no write landed");

    for (headers, wanted) in [(vec![host], 405), (vec![host, evil], 403)] {
        let (st, head, body) = raw_exchange(addr, "OPTIONS", "/mcp", &headers, "").await;
        assert_eq!(st, wanted, "{headers:?}: {head}\n{body}");
        assert!(
            !head.to_ascii_lowercase().contains("access-control-"),
            "no CORS grant: {head}"
        );
    }
}

/// The loopback plane is unchanged by 0043 (AC9): a `text/plain` MCP call
/// with no Origin is dispatched; a rebound Host is refused before a hostile
/// Origin is looked at (Host-first precedence, the Host sentence); and a
/// portless Origin still refuses on a loopback bind.
#[tokio::test]
async fn the_loopback_plane_keeps_its_order_and_its_text_plain_clients() {
    let srv = start().await;
    let addr = srv.http_addr;
    let host_line = format!("Host: {addr}");
    let host = host_line.as_str();
    let list = r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#;
    let (st, _, body) = raw_exchange(
        addr,
        "POST",
        "/mcp",
        &[host, "Content-Type: text/plain"],
        list,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("\"tools\""), "{body}");

    let (st, _, body) = raw_exchange(
        addr,
        "GET",
        "/api/v1/inboxes",
        &[
            format!("Host: attacker.example:{}", addr.port()).as_str(),
            "Origin: https://evil.example",
        ],
        "",
    )
    .await;
    assert_eq!(st, 403, "{body}");
    assert!(
        body.contains("must name a loopback Host"),
        "Host first: {body}"
    );

    let (st, _, body) = raw_exchange(
        addr,
        "GET",
        "/api/v1/inboxes",
        &[host, "Origin: http://localhost"],
        "",
    )
    .await;
    assert_eq!(st, 403, "{body}");
    assert!(
        body.contains("cross-origin requests are refused on the loopback plane"),
        "{body}"
    );
}

/// Serve the public router with an explicit loopback policy on an
/// ephemeral loopback socket. This proves the port-80 *policy* honestly
/// without binding privileged port 80; lib.rs's derivation of the policy
/// from the real listener is covered by `start()`'s loopback fixtures and
/// `start_off_loopback`. The stop sender lives in the serve task, so the
/// server never reads its own stop.
async fn serve_policy(bound: u16) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let state = swarmail::api::AppState {
        store: std::sync::Arc::new(swarmail::store::Store::new(0)),
        chaos: Default::default(),
        webhooks: Default::default(),
        started: std::time::Instant::now(),
        shutdown: stop_rx,
        smtp_degraded: Default::default(),
        pop3_degraded: Default::default(),
        http_origin_guard: Some(bound),
    };
    tokio::spawn(async move {
        let _stop = stop_tx;
        swarmail::api::serve(listener, state, std::future::pending()).await
    });
    addr
}

/// Issue #92 (AC10): with policy `Some(80)` a portless Host — `localhost`,
/// `127.0.0.1`, `[::1]` — is admitted, because an omitted port is http's
/// 80; the same lines refuse under `Some(8025)`. Explicit matching ports
/// pass; a wrong, empty, non-decimal or overflowing port, a malformed
/// bracket tail and a non-loopback host refuse; a portless Origin still
/// refuses at policy 80 (0043 §1). Red before the repair: every portless
/// Host refused.
#[tokio::test]
async fn a_portless_host_is_port_80_on_the_real_router() {
    let port_80 = serve_policy(80).await;
    let port_8025 = serve_policy(8025).await;
    let get = |addr, host: &str| {
        let header = format!("Host: {host}");
        async move {
            let (st, _, body) =
                raw_exchange(addr, "GET", "/api/v1/inboxes", &[header.as_str()], "").await;
            (st, body)
        }
    };
    for host in ["localhost", "127.0.0.1", "[::1]", "LOCALHOST"] {
        let (st, body) = get(port_80, host).await;
        assert_eq!(st, 200, "policy 80 admits {host}: {body}");
        let (st, body) = get(port_8025, host).await;
        assert_eq!(st, 403, "policy 8025 refuses {host}: {body}");
        assert!(body.contains("must name a loopback Host"), "{body}");
    }
    for (addr, host) in [(port_80, "localhost:80"), (port_8025, "localhost:8025")] {
        let (st, body) = get(addr, host).await;
        assert_eq!(st, 200, "{host}: {body}");
    }
    for host in [
        "localhost:8025",
        "localhost:",
        "localhost:8o",
        "localhost:65616",
        "[::1]x",
        "[::1]:",
        "mail.example",
    ] {
        let (st, body) = get(port_80, host).await;
        assert!(
            (st == 403 && body.contains("must name a loopback Host")) || st == 400,
            "policy 80 refuses {host:?}: {st} {body}"
        );
    }
    // No Host at all.
    let (st, _, body) = raw_exchange(port_80, "GET", "/api/v1/inboxes", &[], "").await;
    assert_eq!(st, 403, "{body}");
    // The Origin leg is unchanged: a portless loopback Origin refuses even
    // where a portless Host is admitted.
    let (st, _, body) = raw_exchange(
        port_80,
        "GET",
        "/api/v1/inboxes",
        &["Host: localhost", "Origin: http://localhost"],
        "",
    )
    .await;
    assert_eq!(st, 403, "{body}");
    assert!(
        body.contains("cross-origin requests are refused on the loopback plane"),
        "{body}"
    );
    let (st, _, body) = raw_exchange(
        port_80,
        "GET",
        "/api/v1/inboxes",
        &["Host: localhost", "Origin: http://localhost:80"],
        "",
    )
    .await;
    assert_eq!(st, 200, "an explicit :80 Origin passes: {body}");
}

/// One Prometheus sample: metric name, label pairs, value.
type Sample = (String, Vec<(String, String)>, f64);

fn is_metric_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == ':')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

fn is_label_name(name: &str) -> bool {
    !name.contains(':') && is_metric_name(name)
}

/// A strict reader for the Prometheus text-format subset `/metrics` emits,
/// written from the format's grammar and independent of the server's
/// encoder: `# TYPE <name> counter|gauge` lines; samples of a metric name,
/// an optional `{label="value",…}` set whose values admit only the `\\`,
/// `\"` and `\n` escapes and no raw LF, one space, a numeric value and an
/// LF. Anything else fails the whole scrape, as a strict scraper would.
fn parse_prometheus(text: &str) -> Result<Vec<Sample>, String> {
    let mut samples = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(comment) = rest.strip_prefix('#') {
            let (line, tail) = comment.split_once('\n').ok_or("a comment without LF")?;
            let words: Vec<&str> = line.split(' ').collect();
            let typed = matches!(
                words.as_slice(),
                ["", "TYPE", name, "counter" | "gauge"] if is_metric_name(name)
            );
            if !typed {
                return Err(format!("a malformed comment line: #{line:?}"));
            }
            rest = tail;
            continue;
        }
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .unwrap_or(rest.len());
        let (name, tail) = rest.split_at(end);
        if !is_metric_name(name) {
            return Err(format!("a malformed metric name at {rest:?}"));
        }
        rest = tail;
        let mut labels = Vec::new();
        if let Some(tail) = rest.strip_prefix('{') {
            rest = tail;
            loop {
                let (label, tail) = rest.split_once('=').ok_or("a label without '='")?;
                if !is_label_name(label) {
                    return Err(format!("a malformed label name {label:?}"));
                }
                let mut chars = tail
                    .strip_prefix('"')
                    .ok_or("an unquoted label value")?
                    .chars();
                let mut value = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('\\') => value.push('\\'),
                            Some('"') => value.push('"'),
                            Some('n') => value.push('\n'),
                            other => return Err(format!("an illegal escape \\{other:?}")),
                        },
                        Some('\n') => return Err("a raw LF inside a label value".into()),
                        Some(c) => value.push(c),
                        None => return Err("an unterminated label value".into()),
                    }
                }
                rest = chars.as_str();
                labels.push((label.to_string(), value));
                if let Some(tail) = rest.strip_prefix(',') {
                    rest = tail;
                    continue;
                }
                rest = rest.strip_prefix('}').ok_or("a label set without '}'")?;
                break;
            }
        }
        let tail = rest.strip_prefix(' ').ok_or("no space before the value")?;
        let (value, tail) = tail.split_once('\n').ok_or("a sample without LF")?;
        let value: f64 = value
            .parse()
            .map_err(|_| format!("a non-numeric value {value:?}"))?;
        samples.push((name.to_string(), labels, value));
        rest = tail;
    }
    Ok(samples)
}

/// The reader itself is pinned before it is trusted (AC3): a known-valid
/// independent sample parses to its exact decoded labels, and the
/// spellings Rust's `Debug` produced — `\t`, `\u{…}` — a raw LF inside a
/// quoted label, an unterminated quote and malformed sample or value
/// syntax are all rejected.
#[test]
fn the_strict_prometheus_reader_accepts_the_format_and_nothing_else() {
    let valid = "# TYPE m gauge\nm 1\nm{inbox=\"a\\\\b\\\"c\\nd\te\",x=\"\"} 2\n";
    assert_eq!(
        parse_prometheus(valid),
        Ok(vec![
            ("m".to_string(), vec![], 1.0),
            (
                "m".to_string(),
                vec![
                    ("inbox".to_string(), "a\\b\"c\nd\te".to_string()),
                    ("x".to_string(), String::new()),
                ],
                2.0,
            ),
        ])
    );
    for invalid in [
        "m{inbox=\"tab\\tname\"} 1\n", // Rust's \t
        "m{inbox=\"\\u{7f}\"} 1\n",    // Rust's \u{…}
        "m{inbox=\"raw\nlf\"} 1\n",    // a raw LF inside the quotes
        "m{inbox=\"open} 1\n",         // unterminated quote
        "m{inbox=open} 1\n",           // unquoted value
        "m{inbox=\"a\" 1\n",           // unclosed label set
        "m{9x=\"a\"} 1\n",             // malformed label name
        "m{inbox} 1\n",                // label without '='
        "m 1 2\n",                     // trailing junk after the value
        "m one\n",                     // non-numeric value
        "m\t1\n",                      // wrong separator
        "m 1",                         // no final LF
        "9m 1\n",                      // malformed metric name
        "# HELP m text\n",             // not a TYPE line
        "# TYPE m",                    // a comment without LF
    ] {
        assert!(
            parse_prometheus(invalid).is_err(),
            "{invalid:?} must reject"
        );
    }
}

/// Issue #88's label defect on the wire (AC3): a wait parked on a
/// percent-encoded inbox whose decoded name holds a backslash, a quote and
/// padding yields a scrape the strict reader accepts whole, and the awaits
/// family carries that exact decoded name. Then the wait is satisfied and
/// joined. Since issue #128 (decision 0046 §4) a name with a tab or an LF
/// is refused `400` before any wait parks — the escaper's own unit tests
/// keep the control characters; here the refusal is what the wire shows.
#[tokio::test]
async fn a_hostile_inbox_name_round_trips_through_a_strict_scrape() {
    let srv = start().await;
    let addr = srv.http_addr;
    let (st, body) = http_json(
        addr,
        "GET",
        "/api/v1/inboxes/run20%09tab%0Alf/await?timeout_ms=60000",
        None,
    )
    .await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"], "inbox must not contain control characters");
    let encoded = "%20run20back%5Cquote%22end%20";
    let decoded = " run20back\\quote\"end ";
    let wait = tokio::spawn(async move {
        http_json(
            addr,
            "GET",
            &format!("/api/v1/inboxes/{encoded}/await?timeout_ms=60000"),
            None,
        )
        .await
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let samples = loop {
        let (st, text) = http_get_text(addr, "/metrics").await;
        assert_eq!(st, 200);
        let samples = parse_prometheus(&text).unwrap_or_else(|e| panic!("{e}:\n{text}"));
        if samples.iter().any(|(name, labels, value)| {
            name == "swarmail_awaits_active"
                && labels == &[("inbox".to_string(), decoded.to_string())]
                && *value == 1.0
        }) {
            break samples;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the hostile wait never parked:\n{text}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert!(!samples.is_empty());
    let (st, body) = http_json(
        addr,
        "POST",
        &format!("/api/v1/inboxes/{encoded}/seed"),
        Some(r#"{"to": "hostile@x.io", "text": "satisfy the wait"}"#),
    )
    .await;
    assert_eq!(st, 200, "{body}");
    let (st, body) = wait.await.expect("the wait joins");
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1, "{body}");
}

// ---- run 17: extraction fidelity (issues #74, #93, #94; decision 0040) ----

/// Deliver `data` (headers, blank line, body; CRLF) over real SMTP into
/// `inbox` and return the stored mail REST serves right after the 250.
async fn deliver_mime(srv: &swarmail::RunningServer, inbox: &str, data: &str) -> serde_json::Value {
    let mut conn = SmtpConn::connect(srv.smtp_addr).await;
    conn.send("EHLO swarmail-extract-test").await;
    conn.reply_multi().await;
    let plain = format!("\u{0}{inbox}\u{0}whatever");
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    assert!(conn.reply().await.starts_with("235"));
    conn.send("MAIL FROM:<sender@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    conn.send(&format!("RCPT TO:<{inbox}@x.io>")).await;
    assert!(conn.reply().await.starts_with("250"));
    let reply = conn.data(data).await;
    assert!(reply.starts_with("250"), "DATA refused: {reply}");
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/{inbox}/messages"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    body["emails"][0].clone()
}

/// A stored array field, with the empty array the JSON shape omits.
fn array(email: &serde_json::Value, key: &str) -> serde_json::Value {
    email
        .get(key)
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]))
}

fn html_only(html: &str) -> String {
    format!(
        "From: sender@x.io\r\nSubject: t\r\nMIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n{html}"
    )
}

/// Issue #74's reproduction and its reversed order: the html body is a
/// later candidate than the parser's text fallback, and both genuine
/// bodies — and the html-only link — are stored at ingest. `/raw` serves
/// exactly the accepted bytes: body selection never touches the source.
#[tokio::test]
async fn smtp_mixed_mime_keeps_both_genuine_bodies_in_either_order() {
    let srv = start().await;
    let text = "--b\r\nContent-Type: text/plain\r\n\r\nPlain sender body\r\n";
    let html = "--b\r\nContent-Type: text/html\r\n\r\n\
                <p>HTML sender body <a href=\"https://example.test/verify\">Verify</a></p>\r\n";
    for (inbox, parts) in [
        ("mixed-th", format!("{text}{html}")),
        ("mixed-ht", format!("{html}{text}")),
    ] {
        let data = format!(
            "From: sender@x.io\r\nTo: {inbox}@x.io\r\nSubject: mixed\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=b\r\n\r\n{parts}--b--"
        );
        let email = deliver_mime(&srv, inbox, &data).await;
        assert_eq!(email["text"], "Plain sender body", "{inbox}");
        assert_eq!(
            email["html"],
            "<p>HTML sender body <a href=\"https://example.test/verify\">Verify</a></p>",
            "{inbox}"
        );
        assert_eq!(
            array(&email, "links"),
            serde_json::json!(["https://example.test/verify"]),
            "{inbox}"
        );
        let id = email["id"].as_str().unwrap();
        let (st, raw) = http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
        assert_eq!(st, 200);
        assert_eq!(raw, format!("{data}\r\n").into_bytes(), "{inbox}");
    }
}

/// Every row of issue #93's table over real SMTP, the whole codes array
/// read back through REST (decision 0040 §2–§3): adjacent cells and blocks
/// are never joined, a code in its own element is found, and typed
/// `style`/`script` and attribute values yield nothing.
#[tokio::test]
async fn smtp_html_codes_come_from_tokenized_text_runs() {
    let srv = start().await;
    let rows: [(&str, &[&str]); 10] = [
        (
            "<table><tr><td>Order 1234</td><td>5678</td></tr></table>",
            &["1234", "5678"],
        ),
        ("<div>Ref 2026</div><div>482913</div>", &["2026", "482913"]),
        ("<div>482935</div><div>is your code</div>", &["482935"]),
        ("<h1>Your code</h1><h2>482933</h2>", &["482933"]),
        ("<ul><li>Code</li><li>482936</li></ul>", &["482936"]),
        (
            "<table><tr><td>482937</td><td>expires soon</td></tr></table>",
            &["482937"],
        ),
        ("<p><strong>482938</strong>is your code</p>", &["482938"]),
        (
            "<style type=\"text/css\">p { color: #333333; }</style><p>Hello</p>",
            &[],
        ),
        (
            "<script type=\"text/javascript\">var t = 44445555;</script><p>Hello</p>",
            &[],
        ),
        (
            "<img alt=\"x > 12345678\" src=\"https://x.io/i.png\"><p>Hello</p>",
            &[],
        ),
    ];
    for (row, (html, want)) in rows.iter().enumerate() {
        let email = deliver_mime(&srv, &format!("codes{row}"), &html_only(html)).await;
        assert_eq!(array(&email, "codes"), serde_json::json!(want), "{html}");
        assert!(email.get("text").is_none(), "no text synthesized: {html}");
    }
    // Arabic-Indic digits in a text body are not codes.
    let email = deliver_mime(
        &srv,
        "codes-arabic",
        "From: sender@x.io\r\nSubject: t\r\n\r\nYour code \u{661}\u{662}\u{663}\u{664}",
    )
    .await;
    assert_eq!(array(&email, "codes"), serde_json::json!([]));
}

/// Every row of issue #94's table over real SMTP (decision 0040 §4): the
/// html source is decoded before the URL pattern cuts, so a decoded
/// delimiter ends the link; the full named set decodes; the scheme matches
/// case-insensitively and keeps its spelling.
#[tokio::test]
async fn smtp_html_links_are_cut_after_decoding() {
    let srv = start().await;
    let rows: [(&str, &[&str]); 7] = [
        (
            "<p>Unsubscribe: &lt;https://x.io/unsub?u=1&gt;</p>",
            &["https://x.io/unsub?u=1"],
        ),
        (
            "<p>Link: &quot;https://x.io/q&quot; ok</p>",
            &["https://x.io/q"],
        ),
        (
            "<p>(see https://x.io/p&#41; for more)</p>",
            &["https://x.io/p"],
        ),
        ("<a href=\"https://x.io/a&#32;b\">", &["https://x.io/a"]),
        ("<p>Visit https://x.io/n&nbsp;now</p>", &["https://x.io/n"]),
        (
            "<a href=\"https://x.io/a?b=1&AMP;c=2\">",
            &["https://x.io/a?b=1&c=2"],
        ),
        ("<a href=\"HTTPS://X.IO/upper\">", &["HTTPS://X.IO/upper"]),
    ];
    for (row, (html, want)) in rows.iter().enumerate() {
        let email = deliver_mime(&srv, &format!("links{row}"), &html_only(html)).await;
        assert_eq!(array(&email, "links"), serde_json::json!(want), "{html}");
    }
}

// ---------- run 21: #91 closed queries, null parity, paged inboxes ----------

/// Every native query shape is closed (decision 0044 §3, issue #91): the
/// issue's `assert?count=0&too=` typo — a silently dropped filter that
/// answered a false 409, or with `count=1` a false pass — refuses `400`
/// naming the key, and so does `await?offset=`, which a wait would ignore.
/// Red before: 409 / 200 / 200.
#[tokio::test]
async fn an_unknown_native_query_key_refuses_by_name() {
    let srv = start().await;
    seed(srv.http_addr, "closed", "a@x.io").await;
    for (path, key) in [
        ("/api/v1/inboxes/closed/assert?count=0&too=zzz@x.io", "too"),
        ("/api/v1/inboxes/closed/assert?count=1&too=zzz@x.io", "too"),
        (
            "/api/v1/inboxes/closed/await?offset=3&timeout_ms=60000",
            "offset",
        ),
        ("/api/v1/inboxes/closed/messages?ofset=1", "ofset"),
        ("/api/v1/inboxes/closed/count?inbox=x", "inbox"),
        ("/api/v1/inboxes?page=2", "page"),
    ] {
        let (st, body) = http_json(srv.http_addr, "GET", path, None).await;
        assert_eq!(st, 400, "{path}: {body}");
        let error = body["error"].as_str().unwrap();
        assert!(error.contains(&format!("`{key}`")), "{path}: {body}");
    }
    // The real filter still asserts exactly, the count=0 meaning intact.
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/closed/assert?count=0&to=zzz@x.io",
        None,
    )
    .await;
    assert_eq!((st, body["matched"].clone()), (200, serde_json::json!(0)));
}

/// A present `null` is supplied, not omitted (decision 0025 §2, 0044 §3):
/// every optional REST seed string and the chaos rule's `error` refuse
/// `null` and a non-string with the field named, storing nothing; omission
/// keeps the defaults; a per-event `null` still clears that event. Red
/// before: `"from": null` seeded the default sender, `"error": null` armed.
#[tokio::test]
async fn null_strings_refuse_on_rest_like_mcp() {
    let srv = start().await;
    for field in ["from", "subject", "text", "html"] {
        for bad in ["null", "7"] {
            let body = format!(r#"{{"to": "a@x.io", "{field}": {bad}}}"#);
            let (st, text) = http_send_text(
                srv.http_addr,
                "POST",
                "/api/v1/inboxes/nulls/seed",
                Some(&body),
            )
            .await;
            // 400, the native envelope (decision 0048 §2): a refused
            // supplied value on seed is no longer the extractor's 422.
            assert_eq!(st, 400, "{body}: {text}");
            assert!(text.contains(field), "{body}: the field is named: {text}");
        }
    }
    let (_, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/nulls/count", None).await;
    assert_eq!(count["count"], 0, "no refused seed stored anything");
    let (st, mail) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/nulls/seed",
        Some(r#"{"to": "a@x.io"}"#),
    )
    .await;
    assert_eq!(st, 200, "{mail}");
    assert_eq!(mail["from"]["address"], "fixture@swarmail.dev");
    assert_eq!(mail["envelope_from"], "fixture@swarmail.dev");

    let (st, text) = http_send_text(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100, "error": null}}"#),
    )
    .await;
    assert_eq!(st, 422, "{text}");
    assert!(text.contains("error"), "{text}");
    let (_, chaos) = http_json(srv.http_addr, "GET", "/api/v1/chaos", None).await;
    assert!(chaos["data"].is_null(), "nothing armed: {chaos}");

    let (st, chaos) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 0}, "rcpt": null}"#),
    )
    .await;
    assert_eq!(st, 200, "{chaos}");
    assert!(chaos["rcpt"].is_null(), "a per-event null clears: {chaos}");
    assert!(
        chaos["data"].get("error").is_none(),
        "an unset line is omitted, so the config PUTs back: {chaos}"
    );
    let echoed = chaos.to_string();
    let (st, again) = http_json(srv.http_addr, "PUT", "/api/v1/chaos", Some(&echoed)).await;
    assert_eq!((st, &again), (200, &chaos));
}

/// A decoded blank inbox segment refuses on every native inbox route
/// (decision 0044 §3) — no read, wait, clear, seed or feed — and nothing
/// blank is ever stored; a padded nonblank name is kept verbatim. Red
/// before: the blank seed stored an inbox the UI linked to a 404.
#[tokio::test]
async fn a_blank_path_inbox_refuses_on_every_native_route() {
    let srv = start().await;
    for blank in ["%20", "%20%09", "%E3%80%80"] {
        for (method, route) in [
            ("GET", "messages"),
            ("GET", "count"),
            ("GET", "await?timeout_ms=60000"),
            ("GET", "assert?count=0"),
            ("DELETE", "messages"),
            ("GET", "threads"),
            ("GET", "threads/abc"),
            ("GET", "feed"),
        ] {
            let path = format!("/api/v1/inboxes/{blank}/{route}");
            let (st, body) = http_json(srv.http_addr, method, &path, None).await;
            assert_eq!(st, 400, "{method} {path}: {body}");
            assert_eq!(body["error"], "inbox must not be blank", "{path}");
        }
        let (st, body) = http_json(
            srv.http_addr,
            "POST",
            &format!("/api/v1/inboxes/{blank}/seed"),
            Some(r#"{"to": "a@x.io"}"#),
        )
        .await;
        assert_eq!(st, 400, "{body}");
        assert_eq!(body["error"], "inbox must not be blank");
    }
    let (_, inboxes) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(inboxes, serde_json::json!([]), "nothing blank was stored");
    seed(srv.http_addr, "%20pad%20", "a@x.io").await;
    let (_, inboxes) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(inboxes, serde_json::json!([{"name": " pad ", "count": 1}]));
}

/// A decoded inbox segment carrying a control character refuses on every
/// native inbox route with the shared sentence (issue #128, decision 0046
/// §4) — before any read, wait, clear, seed or feed — so no waiter parks
/// and nothing is stored. Red before: `a%00b` was an inbox like any other.
#[tokio::test]
async fn a_control_path_inbox_refuses_on_every_native_route() {
    let srv = start().await;
    for control in [
        "a%00b",
        "%00user%00s3cretpw",
        "pad%0D",
        "del%7F",
        "c1%C2%85",
    ] {
        for (method, route) in [
            ("GET", "messages"),
            ("GET", "count"),
            ("GET", "await?timeout_ms=60000"),
            ("GET", "assert?count=0"),
            ("DELETE", "messages"),
            ("GET", "threads"),
            ("GET", "threads/abc"),
            ("GET", "feed"),
        ] {
            let path = format!("/api/v1/inboxes/{control}/{route}");
            let (st, body) = http_json(srv.http_addr, method, &path, None).await;
            assert_eq!(st, 400, "{method} {path}: {body}");
            assert_eq!(
                body["error"], "inbox must not contain control characters",
                "{path}"
            );
        }
        let (st, body) = http_json(
            srv.http_addr,
            "POST",
            &format!("/api/v1/inboxes/{control}/seed"),
            Some(r#"{"to": "a@x.io"}"#),
        )
        .await;
        assert_eq!(st, 400, "{body}");
        assert_eq!(body["error"], "inbox must not contain control characters");
    }
    let (_, inboxes) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(inboxes, serde_json::json!([]), "nothing was stored");
}

/// `/api/v1/inboxes` is paged (decision 0044 §3, issue #91): with 251
/// inboxes the default page is 100, 250 is the largest page, 251 refuses by
/// name, `limit=0` is empty, a huge offset is past the end, and walking the
/// pages yields every name once, sorted. Red before: one unbounded array.
#[tokio::test]
async fn the_inbox_listing_pages_past_the_cap() {
    let srv = start().await;
    for i in 0..251 {
        srv.store
            .insert(swarmail::smtp::build_email(
                b"Subject: x\r\n\r\nx",
                &format!("page-{i:03}"),
                &["a@x.io".to_string()],
                "s@x.io".into(),
            ))
            .unwrap();
    }
    let names = |body: &serde_json::Value| -> Vec<String> {
        body.as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap().to_string())
            .collect()
    };
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200);
    assert_eq!(names(&body).len(), 100);
    assert_eq!(body[0], serde_json::json!({"name": "page-000", "count": 1}));
    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes?limit=250", None).await;
    assert_eq!(names(&body).len(), 250);
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes?limit=251", None).await;
    assert_eq!(st, 400);
    assert_eq!(body["error"], "limit must be at most 250");
    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes?limit=0", None).await;
    assert_eq!(body, serde_json::json!([]));
    let huge = format!("/api/v1/inboxes?offset={}", u64::MAX);
    let (_, body) = http_json(srv.http_addr, "GET", &huge, None).await;
    assert_eq!(body, serde_json::json!([]));
    let (st, _) = http_json(srv.http_addr, "GET", "/api/v1/inboxes?offset=-1", None).await;
    assert_eq!(st, 400);
    let mut walked = Vec::new();
    for offset in (0..300).step_by(100) {
        let path = format!("/api/v1/inboxes?limit=100&offset={offset}");
        let (_, body) = http_json(srv.http_addr, "GET", &path, None).await;
        walked.extend(names(&body));
    }
    let expected: Vec<String> = (0..251).map(|i| format!("page-{i:03}")).collect();
    assert_eq!(walked, expected, "disjoint, complete, sorted");
}

// ---------- batch 3 (#134, #135): closed bodies and route-named query sets ----------

/// A raw request with whatever head the test spells — the media-type and
/// size legs need a body without the helpers' `application/json`. Write
/// errors are tolerated: a server may answer an oversized body before it
/// has read it all.
async fn raw_http(addr: std::net::SocketAddr, head: &str, body: &[u8]) -> (u16, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut request = format!("{head}Host: {addr}\r\nConnection: close\r\n\r\n").into_bytes();
    request.extend_from_slice(body);
    let _ = stream.write_all(&request).await;
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf).await;
    let s = String::from_utf8_lossy(&buf).to_string();
    let status = s
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = s.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    (status, body.to_string())
}

/// Issue #134: the REST seed body is closed (decision 0048 §2). `subjet`
/// used to answer 200 and store "(no subject)"; it is now a 400 naming the
/// key, nothing is stored and no inbox is minted. The legal spelling still
/// seeds through the ingest pipeline and is queryable at once.
#[tokio::test]
async fn seed_unknown_key_refuses_without_storing() {
    let srv = start().await;
    let (st, body) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/n1/seed",
        Some(r#"{"to": "a@x.io", "subjet": "typo"}"#),
    )
    .await;
    assert_eq!(st, 400, "{body}");
    let error = body["error"].as_str().unwrap();
    assert!(error.starts_with("invalid seed body: "), "{body}");
    assert!(error.contains("unknown field `subjet`"), "{body}");
    let (_, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/n1/count", None).await;
    assert_eq!(count["count"], 0, "nothing stored");
    let (_, inboxes) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(inboxes, serde_json::json!([]), "no inbox minted");

    let (st, mail) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/n1/seed",
        Some(r#"{"to": "a@x.io", "subject": "kept", "text": "Code 424242"}"#),
    )
    .await;
    assert_eq!(st, 200, "{mail}");
    assert_eq!(mail["subject"], "kept");
    assert_eq!(
        mail["codes"],
        serde_json::json!(["424242"]),
        "extracted at ingest"
    );
    let (_, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/n1/count", None).await;
    assert_eq!(count["count"], 1, "the accepted seed is queryable");
}

/// Decision 0048 §2's scoped amendment: on seed POST and webhook PUT every
/// JSON *data* error — unknown key, `null`, wrong type, missing field, wrong
/// top-level shape — is a native 400 `{"error": …}` naming the field, where
/// the extractor answered 422; nothing is stored or replaced. Syntax (400),
/// media type (415) and size (413) keep the extractor's own responses, and
/// chaos keeps its 422.
#[tokio::test]
async fn seed_and_webhook_data_errors_are_local_400s() {
    let srv = start().await;
    let prior = r#"[{"url": "http://127.0.0.1:9/prior", "secret": "s"}]"#;
    let (st, installed) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(prior)).await;
    assert_eq!(st, 200, "{installed}");

    for (body, named) in [
        (r#"{"to": "a@x.io", "subject": null}"#, "subject"),
        (r#"{"to": 7}"#, "to"),
        (r#"{"text": "x"}"#, "missing field `to`"),
        (r#"["a@x.io"]"#, "invalid seed body"),
        (r#""a@x.io""#, "invalid seed body"),
    ] {
        let (st, resp) = http_json(
            srv.http_addr,
            "POST",
            "/api/v1/inboxes/dataerr/seed",
            Some(body),
        )
        .await;
        assert_eq!(st, 400, "{body}: {resp}");
        assert!(
            resp["error"].as_str().unwrap().contains(named),
            "{body}: {resp}"
        );
    }
    let (_, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/dataerr/count", None).await;
    assert_eq!(count["count"], 0, "no refused seed stored anything");

    for (body, named) in [
        (
            r#"{"url": "http://127.0.0.1:9/x"}"#,
            "expected a JSON array of webhook targets, found an object",
        ),
        (r#""http://user:hunter2@127.0.0.1:9/x""#, "found a string"),
        (
            r#"["http://user:hunter2@127.0.0.1:9/x"]"#,
            "[0]: expected a webhook target object, found a string",
        ),
        (
            r#"[{"url": 7}]"#,
            "[0].url: expected a string, found a number",
        ),
        (
            r#"[{"url": "http://127.0.0.1:9/x", "secret": 4242}]"#,
            "[0].secret: expected a string, found a number",
        ),
        (r#"[{"inbox": "a"}]"#, "missing field `url`"),
    ] {
        let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(body)).await;
        assert_eq!(st, 400, "{body}: {resp}");
        let error = resp["error"].as_str().unwrap();
        assert!(
            error.starts_with("invalid webhooks body: "),
            "{body}: {resp}"
        );
        assert!(error.contains(named), "{body}: {resp}");
        assert!(
            !error.contains("hunter2") && !error.contains("4242"),
            "no value echoed: {resp}"
        );
    }
    let (_, after) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(after, installed, "no refused PUT replaced the list");

    // The non-data rejections are the extractor's own, untouched. Each
    // malformed body opens the shape its route reads, so the syntax error
    // is what the parse meets first.
    for (method, path, malformed) in [
        ("POST", "/api/v1/inboxes/dataerr/seed", "{ nope"),
        ("PUT", "/api/v1/webhooks", "[ nope"),
    ] {
        let (st, text) = http_send_text(srv.http_addr, method, path, Some(malformed)).await;
        assert_eq!(st, 400, "{path}: {text}");
        assert!(
            text.starts_with("Failed to parse the request body"),
            "{path}: {text}"
        );
        let head = format!(
            "{method} {path} HTTP/1.1\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n"
        );
        let (st, text) = raw_http(srv.http_addr, &head, b"[]").await;
        assert_eq!(st, 415, "{path}: {text}");
        let big = vec![b' '; 2_097_153];
        let head = format!(
            "{method} {path} HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            big.len()
        );
        let (st, text) = raw_http(srv.http_addr, &head, &big).await;
        assert_eq!(st, 413, "{path}: {text}");
    }
    let (st, text) = http_send_text(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100, "eror": "x"}}"#),
    )
    .await;
    assert_eq!(st, 422, "chaos keeps the extractor's 422: {text}");
    let (_, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/dataerr/count", None).await;
    assert_eq!(count["count"], 0);
    let (_, after) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(after, installed, "nothing replaced the list");
}

/// Issue #134: count, assert and the three view routes take exactly their
/// own keys (decision 0048 §4). Each key below used to answer 200 and be
/// ignored — `assert?timeout_ms=` promised a wait that never happened — and
/// is now a 400 naming the route and the key; a filter name supported on
/// list is still refused where the route takes none.
#[tokio::test]
async fn native_count_assert_and_views_refuse_ignored_keys() {
    let srv = start().await;
    let seeded = seed(srv.http_addr, "q134", "a@x.io").await;
    let (_, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/q134/threads", None).await;
    let key = threads[0]["key"].as_str().unwrap().to_string();
    let thread_query = format!("threads/{key}?limit=1");
    for (path, route, bad) in [
        ("count?limit=1", "count", "limit"),
        ("count?offset=3&limit=1", "count", "offset"),
        ("count?limit=9999", "count", "limit"),
        ("count?bogus=1", "count", "bogus"),
        ("assert?count=0&timeout_ms=5000", "assert", "timeout_ms"),
        ("assert?timeout_ms=1", "assert", "timeout_ms"),
        ("assert?offset=1", "assert", "offset"),
        ("assert?bogus=1", "assert", "bogus"),
        ("threads?bogus=1", "threads", "bogus"),
        ("threads?to=nobody", "threads", "to"),
        (thread_query.as_str(), "thread", "limit"),
        ("feed?bogus=1", "feed", "bogus"),
        ("feed?since_ms=1", "feed", "since_ms"),
    ] {
        let full = format!("/api/v1/inboxes/q134/{path}");
        let (st, body) = http_json(srv.http_addr, "GET", &full, None).await;
        assert_eq!(st, 400, "{full}: {body}");
        let error = body["error"].as_str().unwrap();
        assert!(
            error.starts_with(&format!("invalid {route} query: ")),
            "{full}: {body}"
        );
        assert!(error.contains(&format!("`{bad}`")), "{full}: {body}");
    }

    // The retained keys keep their meaning: every count filter applies,
    // and the assert compares exactly — default 1, 0 legal, above the page
    // cap legal — with `limit` type-checked and inert.
    let since = seeded["received_ms"].as_i64().unwrap();
    let later = format!("?since_ms={}", since + 1);
    let exact = format!("?since_ms={since}");
    for (query, count) in [
        ("", 1),
        ("?to=a@x.io", 1),
        ("?to=b@x.io", 0),
        ("?from=fixture@swarmail.dev", 1),
        ("?subject=nomatch", 0),
        (later.as_str(), 0),
        (exact.as_str(), 1),
    ] {
        let path = format!("/api/v1/inboxes/q134/count{query}");
        let (st, body) = http_json(srv.http_addr, "GET", &path, None).await;
        assert_eq!(
            (st, body["count"].clone()),
            (200, serde_json::json!(count)),
            "{path}"
        );
    }
    for (query, status) in [
        ("", 200),
        ("?count=1", 200),
        ("?count=0", 409),
        ("?count=0&to=b@x.io", 200),
        ("?count=1&limit=1", 200),
        ("?count=1&limit=999", 200),
        ("?count=300", 409),
    ] {
        let path = format!("/api/v1/inboxes/q134/assert{query}");
        let (st, body) = http_json(srv.http_addr, "GET", &path, None).await;
        assert_eq!(st, status, "{path}: {body}");
    }
    for path in ["count?since_ms=soon", "assert?limit=-1", "assert?count=x"] {
        let full = format!("/api/v1/inboxes/q134/{path}");
        let (st, body) = http_json(srv.http_addr, "GET", &full, None).await;
        assert_eq!(st, 400, "{full}: {body}");
    }

    // The views still answer without a query.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/q134/threads", None).await;
    assert_eq!((st, body.as_array().unwrap().len()), (200, 1));
    let thread = format!("/api/v1/inboxes/q134/threads/{key}");
    let (st, body) = http_json(srv.http_addr, "GET", &thread, None).await;
    assert_eq!((st, body["count"].clone()), (200, serde_json::json!(1)));
}

/// Issue #134's documentation half (decision 0048 §5): the served llms.txt
/// no longer claims "Strict input everywhere" — compat list/search keep
/// their open shapes, which the wire still shows (`inbx=` is tolerated and
/// scopes nothing), while the native routes and the latest family refuse.
#[tokio::test]
async fn the_strictness_claims_match_the_wire() {
    let srv = start().await;
    let (_, llms) = http_get_text(srv.http_addr, "/llms.txt").await;
    assert!(!llms.contains("Strict input everywhere"), "{llms}");
    assert!(
        llms.contains("inbx"),
        "the compat tolerance is named: {llms}"
    );
    seed(srv.http_addr, "s1", "a@x.io").await;
    seed(srv.http_addr, "s2", "a@x.io").await;
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/messages?inbx=s1", None).await;
    assert_eq!(st, 200, "compat list tolerates an unknown key: {body}");
    assert_eq!(body["total"], 2, "and the typo scopes nothing: {body}");
    let (st, _) = http_json(srv.http_addr, "GET", "/api/v1/message/latest?inbx=s1", None).await;
    assert_eq!(st, 400, "latest stays strict");
}

// ---- epic #120 batch 4: extraction fidelity (issues #136, #137, #139; decision 0049) ----

/// Issue #136 over real SMTP and REST: a quoted attribute value ends at
/// its own source quote and then decodes, so the encoded apostrophes and
/// quotes real templating engines write stay inside the link; balanced
/// parentheses, IPv6 literals and path commas survive; a signed numeric
/// reference stays literal and manufactures no code; named references of
/// the full WHATWG table decode in an attribute and in text. Red at
/// 2a325c9: `https://app.test/reset?name=O`, `https://x.io/?q=`,
/// `…/Foo_(bar`, `http://[::1`, `…/w_100` and the code `4829`; red on the
/// 71-name interim table: `…/caf&eacute;` and `…/go&rarr;`.
#[tokio::test]
async fn smtp_issue_136_link_fidelity() {
    let srv = start().await;
    let html = concat!(
        "<a href=\"https://app.test/reset?name=O&#39;Brien&amp;token=abc123\">a</a>",
        "<a href=\"https://app.test/reset?name=O&#x27;Brien&amp;token=abc124\">b</a>",
        "<a href=\"https://app.test/reset?name=O'Brien&amp;token=abc125\">c</a>",
        "<a href=\"https://x.io/?q=&quot;x&quot;\">q</a>",
        "<p>See https://en.wikipedia.org/wiki/Foo_(bar), <a href=\"http://[::1]:8080/p\">",
        "local</a> and (https://cdn.x.io/w_100,h_100/x.jpg).</p>",
        "<p>&#+52;&#+56;&#+50;&#+57; <a href=\"https://x.io/&#+52;/\">n</a></p>",
        "<a href=\"https://x.io/caf&eacute;\">e</a><p>https://x.io/go&rarr;</p>",
    );
    let email = deliver_mime(&srv, "links136", &html_only(html)).await;
    assert_eq!(
        array(&email, "links"),
        serde_json::json!([
            "https://app.test/reset?name=O'Brien&token=abc123",
            "https://app.test/reset?name=O'Brien&token=abc124",
            "https://app.test/reset?name=O'Brien&token=abc125",
            "https://x.io/?q=\"x\"",
            "https://en.wikipedia.org/wiki/Foo_(bar)",
            "http://[::1]:8080/p",
            "https://cdn.x.io/w_100,h_100/x.jpg",
            "https://x.io/&#+52;/",
            "https://x.io/café",
            "https://x.io/go→",
        ])
    );
    assert_eq!(array(&email, "codes"), serde_json::json!([]));
    // The stored body is the sender's, byte for byte (decision 0031).
    assert_eq!(email["html"], format!("{html}\r\n"));
}

/// One multipart/mixed mail: `parts` between the outer boundary lines.
fn mixed_mime(parts: &str) -> String {
    format!(
        "From: sender@x.io\r\nSubject: m\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=b\r\n\r\n{parts}--b--"
    )
}

/// Issue #137 over real SMTP: only a text part and an HTML part that are
/// direct siblings in one multipart/alternative earn text preference;
/// independent parts contribute in source order, so a footer or an inline
/// text file after an HTML-only body no longer hides its code. Red at
/// 2a325c9: `["2026"]` and `["444445"]`. The stored bodies stay the
/// sender's own, and the alternative preference is unchanged.
#[tokio::test]
async fn smtp_mixed_footers_do_not_hide_html_codes() {
    let srv = start().await;
    let footer = "--b\r\nContent-Type: text/plain\r\n\r\n-- list footer 2026\r\n";
    let html = "--b\r\nContent-Type: text/html\r\n\r\n<p>Your code is 121212</p>\r\n";
    let cases: [(&str, String, &[&str]); 6] = [
        (
            "foot137",
            mixed_mime(&format!(
                "--b\r\nContent-Type: multipart/alternative; boundary=a\r\n\r\n\
                 --a\r\nContent-Type: text/html\r\n\r\n<p>Your code is 121212</p>\r\n--a--\r\n\
                 {footer}"
            )),
            &["121212", "2026"],
        ),
        (
            "file137",
            mixed_mime(
                "--b\r\nContent-Type: text/html\r\n\r\n<p>Your code is 333334</p>\r\n\
                 --b\r\nContent-Type: text/plain\r\n\
                 Content-Disposition: inline; filename=notes.txt\r\n\r\nnotes 444445\r\n",
            ),
            &["333334", "444445"],
        ),
        // Reversed: the text part comes first in the source.
        (
            "rev137",
            mixed_mime(&format!("{footer}{html}")),
            &["2026", "121212"],
        ),
        // Direct alternatives: the text's code, the HTML's ignored.
        (
            "alt137",
            mixed_mime(
                "--b\r\nContent-Type: multipart/alternative; boundary=a\r\n\r\n\
                 --a\r\nContent-Type: text/plain\r\n\r\nCode 555556\r\n\
                 --a\r\nContent-Type: text/html\r\n\r\n<p>Code 666667</p>\r\n--a--\r\n",
            ),
            &["555556"],
        ),
        // Direct alternatives whose text has no code: the HTML's.
        (
            "empty137",
            mixed_mime(
                "--b\r\nContent-Type: multipart/alternative; boundary=a\r\n\r\n\
                 --a\r\nContent-Type: text/plain\r\n\r\nSee the HTML\r\n\
                 --a\r\nContent-Type: text/html\r\n\r\n<p>Code 777778</p>\r\n--a--\r\n",
            ),
            &["777778"],
        ),
        // A shared ancestor is not enough: related under alternative merges.
        (
            "rel137",
            "From: sender@x.io\r\nSubject: r\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/alternative; boundary=a\r\n\r\n\
             --a\r\nContent-Type: text/plain\r\n\r\nCode 888889\r\n\
             --a\r\nContent-Type: multipart/related; boundary=r\r\n\r\n\
             --r\r\nContent-Type: text/html\r\n\r\n<p>Code 999990</p>\r\n--r--\r\n--a--"
                .to_string(),
            &["888889", "999990"],
        ),
    ];
    for (inbox, data, want) in cases {
        let email = deliver_mime(&srv, inbox, &data).await;
        assert_eq!(array(&email, "codes"), serde_json::json!(want), "{inbox}");
        assert!(
            email["text"].is_string() && email["html"].is_string(),
            "{inbox}: {email}"
        );
    }
    let email = deliver_mime(&srv, "body137", &mixed_mime(&format!("{html}{footer}"))).await;
    assert_eq!(email["text"], "-- list footer 2026");
    assert_eq!(email["html"], "<p>Your code is 121212</p>");
}

/// Issue #139's 8 MiB numeric fixture over real SMTP: 1,048,576 distinct
/// seven-digit numbers, each followed by one space. Exactly the first 64
/// codes are stored — a 641-byte array — and every page that carries the
/// mail is bounded by its body, not by a million strings: the native list
/// (full bodies) under 9 MiB, Mailpit's summary page under 16 KiB, the
/// complete MailHog page under 40 MiB. Red at 2a325c9: 1,048,576 codes, an
/// 18.9 MB native list and a 33.6 MB MailHog page.
#[tokio::test]
async fn smtp_numeric_mail_has_bounded_codes_and_response_bytes() {
    let srv = start().await;
    let body: String = (0..1_048_576u32)
        .map(|n| format!("{} ", 1_000_000 + n))
        .collect();
    assert_eq!(body.len(), 8 * 1024 * 1024);
    let data = format!("From: sender@x.io\r\nSubject: n\r\n\r\n{body}");
    let email = deliver_mime(&srv, "num139", &data).await;
    let want: Vec<String> = (0..64).map(|n| (1_000_000 + n).to_string()).collect();
    assert_eq!(array(&email, "codes"), serde_json::json!(want));
    assert_eq!(serde_json::to_string(&email["codes"]).unwrap().len(), 641);

    let (st, native) = http_bytes(srv.http_addr, "/api/v1/inboxes/num139/messages").await;
    assert_eq!(st, 200);
    assert!(native.len() < 9 * 1024 * 1024, "{}", native.len());
    let native: serde_json::Value = serde_json::from_slice(&native).unwrap();
    assert!(native["emails"][0]["text"].as_str().unwrap().len() >= body.len() - 2);

    let (st, summary) = http_bytes(srv.http_addr, "/api/v1/messages?inbox=num139").await;
    assert_eq!(st, 200);
    assert!(summary.len() < 16 * 1024, "{}", summary.len());
    let summary: serde_json::Value = serde_json::from_slice(&summary).unwrap();
    assert_eq!(summary["messages"].as_array().unwrap().len(), 1);

    let (st, hog) = http_bytes(srv.http_addr, "/api/v2/messages?inbox=num139&limit=1").await;
    assert_eq!(st, 200);
    assert!(hog.len() < 40 * 1024 * 1024, "{}", hog.len());
    let hog: serde_json::Value = serde_json::from_slice(&hog).unwrap();
    assert_eq!(hog["items"].as_array().unwrap().len(), 1);
}

/// The cap holds on the seed path too (decision 0049 §4): 65 distinct
/// codes across the HTML's runs, the 65th dropped, a duplicate taking no
/// slot.
#[tokio::test]
async fn seeded_mail_keeps_the_first_64_distinct_codes() {
    let srv = start().await;
    let mut cells: Vec<String> = (0..65).map(|n| (500_000 + n).to_string()).collect();
    cells.insert(10, "500000".into());
    let html: String = cells.iter().map(|c| format!("<td>{c}</td>")).collect();
    let body = serde_json::json!({"to": "a@x.io", "html": html}).to_string();
    let (st, seeded) = http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/seed139/seed",
        Some(&body),
    )
    .await;
    assert_eq!(st, 200, "{seeded}");
    let want: Vec<String> = (0..64).map(|n| (500_000 + n).to_string()).collect();
    let id = first_id(srv.http_addr, "seed139").await;
    let (st, stored) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(stored["codes"], serde_json::json!(want));
}

// ---- epic #120 batch 6 (#142): the published contract against the wire ----

/// The contract as the server serves it, read over the wire.
async fn served_openapi(addr: std::net::SocketAddr) -> serde_json::Value {
    let (st, doc) = http_json(addr, "GET", "/openapi.json", None).await;
    assert_eq!(st, 200, "{doc}");
    doc
}

#[tokio::test]
async fn query_free_native_contracts_match_served_docs() {
    let srv = start().await;
    let doc = served_openapi(srv.http_addr).await;
    let rows = [
        (
            "DELETE",
            "/api/v1/inboxes/{inbox}/messages",
            ["inbox"].as_slice(),
        ),
        ("DELETE", "/api/v1/delete-all", [].as_slice()),
        ("GET", "/api/v1/messages/{id}", ["id"].as_slice()),
        ("DELETE", "/api/v1/messages/{id}", ["id"].as_slice()),
        ("GET", "/api/v1/messages/{id}/raw", ["id"].as_slice()),
        ("GET", "/api/v1/chaos", [].as_slice()),
        ("DELETE", "/api/v1/chaos", [].as_slice()),
        ("GET", "/api/v1/webhooks", [].as_slice()),
        ("DELETE", "/api/v1/webhooks", [].as_slice()),
        ("POST", "/api/v1/inboxes/{inbox}/seed", ["inbox"].as_slice()),
    ];
    for (method, path, expected_path_parameters) in rows {
        let method = method.to_ascii_lowercase();
        let operation = &doc["paths"][path][&method];
        let parameters = operation["parameters"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let query_parameters: Vec<_> = parameters
            .iter()
            .filter(|parameter| parameter["in"] == "query")
            .collect();
        assert!(
            query_parameters.is_empty(),
            "{method} {path}: {query_parameters:?}"
        );
        let path_parameters: Vec<_> = parameters
            .iter()
            .filter(|parameter| parameter["in"] == "path")
            .map(|parameter| parameter["name"].as_str().unwrap())
            .collect();
        assert_eq!(path_parameters, expected_path_parameters, "{method} {path}");
        let label = format!("{} {path}", method.to_ascii_uppercase());
        let description = operation["description"].as_str().unwrap();
        assert!(description.contains(&label), "{label}: {description}");
        assert!(
            description.contains("no query keys"),
            "{label}: {description}"
        );
        let refusal = operation["responses"]["400"]["description"]
            .as_str()
            .unwrap_or_else(|| panic!("missing 400 description for {label}"));
        assert!(refusal.contains("query key"), "{label}: {refusal}");
        assert!(refusal.contains("native JSON"), "{label}: {refusal}");
    }

    let list_path = &doc["paths"]["/api/v1/inboxes/{inbox}/messages"];
    let list_get = &list_path["get"];
    let effective_parameters = list_path["parameters"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .chain(
            list_get["parameters"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
        )
        .collect::<Vec<_>>();
    let mut list_query_names = effective_parameters
        .iter()
        .filter(|parameter| parameter["in"] == "query")
        .map(|parameter| parameter["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    list_query_names.sort_unstable();
    assert_eq!(
        list_query_names,
        ["from", "limit", "offset", "since_ms", "subject", "to"]
    );
    let list_path_names = effective_parameters
        .iter()
        .filter(|parameter| parameter["in"] == "path")
        .map(|parameter| parameter["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(list_path_names, ["inbox"]);
    let inbox_path_parameter = effective_parameters
        .iter()
        .find(|parameter| parameter["in"] == "path" && parameter["name"] == "inbox")
        .expect("list GET retains its inbox path parameter");
    assert_eq!(inbox_path_parameter["required"], true);
    let list_refusal = list_get["responses"]["400"]["description"]
        .as_str()
        .expect("list GET publishes its 400 contract");
    assert!(
        !list_refusal.contains("every native query shape is closed"),
        "list GET must not claim universal native query closure: {list_refusal}"
    );
    assert!(
        list_refusal.contains(
            "GET /api/v1/inboxes/{inbox}/messages accepts only the query keys to, from, subject, since_ms, limit and offset"
        ),
        "list GET must name its own allowed query keys: {list_refusal}"
    );

    let readme = include_str!("../README.md");
    let (_, llms) = http_get_text(srv.http_addr, "/llms.txt").await;
    for route in [
        "DELETE /api/v1/inboxes/{inbox}/messages",
        "DELETE /api/v1/delete-all",
        "GET /api/v1/messages/{id}",
        "DELETE /api/v1/messages/{id}",
        "GET /api/v1/messages/{id}/raw",
        "GET /api/v1/chaos",
        "DELETE /api/v1/chaos",
        "GET /api/v1/webhooks",
        "DELETE /api/v1/webhooks",
        "POST /api/v1/inboxes/{inbox}/seed",
    ] {
        assert!(readme.contains(route), "README omits {route}");
        assert!(llms.contains(route), "served llms.txt omits {route}");
    }
    for (text, name) in [(readme, "README"), (llms.as_str(), "llms.txt")] {
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized.contains("empty-valued"),
            "{name} omits empty-valued refusal"
        );
        assert!(
            normalized.contains("literal empty query"),
            "{name} omits empty-query behavior"
        );
        assert!(
            normalized.contains("tolerance of unknown keys")
                || normalized.contains("list/search tolerance"),
            "{name} omits compat tolerance"
        );
        assert!(
            (normalized.contains("body-based") || normalized.contains("ids body"))
                && normalized.contains("ids"),
            "{name} omits selective delete guidance"
        );
    }
}

/// One documented query parameter's schema.
fn param_schema(doc: &serde_json::Value, path: &str, name: &str) -> serde_json::Value {
    doc["paths"][path]["get"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("{path} documents {name}"))["schema"]
        .clone()
}

/// Whether the published operation lists `status` in its response map.
fn documents(doc: &serde_json::Value, path: &str, method: &str, status: u16) -> bool {
    doc["paths"][path][method.to_ascii_lowercase()]["responses"]
        .get(status.to_string())
        .is_some()
}

/// A schema value as a page size.
fn as_size(value: &serde_json::Value) -> usize {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("an integer: {value}")) as usize
}

/// A raw exchange that tolerates the server answering before it has read
/// the whole body (the oversized legs), returning the status, the response
/// head and the body.
async fn raw_head_exchange(
    addr: std::net::SocketAddr,
    head: &str,
    body: &[u8],
) -> (u16, String, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut request = format!("{head}Host: {addr}\r\nConnection: close\r\n\r\n").into_bytes();
    request.extend_from_slice(body);
    let _ = stream.write_all(&request).await;
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf).await;
    let text = String::from_utf8_lossy(&buf).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, head.to_ascii_lowercase(), body.to_string())
}

/// Issue #142 (D22): the await and assert page domains the document
/// publishes are the ones real requests meet. Await's omitted `limit` is
/// the published default (100: `count=120` answers 100 emails), its
/// published minimum 0 answers `[]`, its maximum is a page and one more is
/// refused; its `count` defaults to one, its published minimum 0 is a
/// minimum-one wait — one email on a full inbox, a 408 on an empty one,
/// never an instant success — and above the maximum refuses. Assert's
/// `limit` publishes no default and no cap and is inert at every value,
/// while a negative one is the parser's 400. Red on the old 50/minimum-1
/// schema values.
#[tokio::test]
async fn the_published_await_and_assert_domains_match_the_wire() {
    let srv = start().await;
    let doc = served_openapi(srv.http_addr).await;
    let await_path = "/api/v1/inboxes/{inbox}/await";
    let assert_path = "/api/v1/inboxes/{inbox}/assert";
    for i in 0..120 {
        srv.store
            .insert(swarmail::smtp::build_email(
                format!("Subject: m{i}\r\n\r\nx").as_bytes(),
                "pin142-await",
                &["a@x.io".to_string()],
                "s@x.io".into(),
            ))
            .unwrap();
    }
    let addr = srv.http_addr;
    let get = |query: String| {
        let path = format!("/api/v1/inboxes/pin142-await/{query}");
        async move { http_json(addr, "GET", &path, None).await }
    };
    let emails = |body: &serde_json::Value| body["emails"].as_array().unwrap().len();

    let limit = param_schema(&doc, await_path, "limit");
    let (default, min, max) = (
        as_size(&limit["default"]),
        as_size(&limit["minimum"]),
        as_size(&limit["maximum"]),
    );
    let (st, body) = get("await?count=120&timeout_ms=5000".into()).await;
    assert_eq!((st, body["matched"].clone()), (200, 120.into()), "{body}");
    assert_eq!(emails(&body), default, "the omitted limit is the default");
    assert_eq!(default, 100, "count=120 answers a 100-email page");
    let (st, body) = get(format!("await?count=120&limit={min}&timeout_ms=5000")).await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(
        body["emails"],
        serde_json::json!([]),
        "limit=0 is an empty page"
    );
    let (st, body) = get(format!("await?count=120&limit={max}&timeout_ms=5000")).await;
    assert_eq!((st, emails(&body)), (200, 120), "{body}");
    let (st, body) = get(format!("await?count=1&limit={}", max + 1)).await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"], format!("limit must be at most {max}"));

    let count = param_schema(&doc, await_path, "count");
    let (default, min, max) = (
        as_size(&count["default"]),
        as_size(&count["minimum"]),
        as_size(&count["maximum"]),
    );
    assert_eq!(min, 0, "count=0 is accepted: {count}");
    let (st, body) = get("await?timeout_ms=5000".into()).await;
    assert_eq!((st, emails(&body)), (200, default), "{body}");
    assert_eq!(body["matched"], default);
    let (st, body) = get(format!("await?count={min}&timeout_ms=5000")).await;
    assert_eq!(
        (st, emails(&body)),
        (200, 1),
        "count=0 waits for one: {body}"
    );
    assert_eq!(body["matched"], 1, "{body}");
    let empty = format!("/api/v1/inboxes/pin142-empty/await?count={min}&timeout_ms=0");
    let (st, body) = http_json(srv.http_addr, "GET", &empty, None).await;
    assert_eq!(st, 408, "count=0 on an empty inbox still waits: {body}");
    assert_eq!(body["matched"], 0, "{body}");
    let (st, body) = get(format!("await?count={}", max + 1)).await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"], format!("count must be at most {max}"));

    let inert = param_schema(&doc, assert_path, "limit");
    assert_eq!(
        inert.get("default"),
        None,
        "no invented page default: {inert}"
    );
    assert_eq!(inert.get("maximum"), None, "no page cap: {inert}");
    assert_eq!(as_size(&inert["minimum"]), 0, "{inert}");
    for limit in [
        String::new(),
        "&limit=0".into(),
        "&limit=1".into(),
        "&limit=251".into(),
        format!("&limit={}", u64::MAX),
    ] {
        let (st, body) = get(format!("assert?count=120{limit}")).await;
        assert_eq!(st, 200, "the limit is inert ({limit:?}): {body}");
        assert_eq!(body["matched"], 120, "{body}");
    }
    let (st, body) = get("assert?limit=-1".into()).await;
    assert_eq!(st, 400, "below the parser's domain: {body}");
    let exact = param_schema(&doc, assert_path, "count");
    let (st, body) = get("assert".into()).await;
    assert_eq!(st, 409, "{body}");
    assert_eq!(body["expected"], exact["default"], "{body}");
    let negative = format!(
        "/api/v1/inboxes/pin142-empty/assert?count={}",
        as_size(&exact["minimum"])
    );
    let (st, body) = http_json(srv.http_addr, "GET", &negative, None).await;
    assert_eq!(st, 200, "count=0 asserts an empty inbox: {body}");
}

/// Issue #142 (D23): the Host/Origin guard answers before the router on
/// every route, so every published operation lists the shared 403 — and
/// each one really answers it: on a loopback bind to a hostile Origin and
/// to a rebound Host, and off loopback to an Origin that is not the Host's
/// authority. The operation list is the served document itself, so a new
/// operation without its 403 entry, or one the guard does not cover, is
/// red.
#[tokio::test]
async fn every_published_operation_lists_and_answers_the_guard_403() {
    let srv = start().await;
    let off = start_off_loopback().await;
    let doc = served_openapi(srv.http_addr).await;
    let refusal = &doc["components"]["responses"]["GuardRefused"];
    assert!(
        refusal["content"]["application/json"].is_object(),
        "{refusal}"
    );
    let port = srv.http_addr.port();
    let hostile_origin = format!("http://attacker.example:{port}");
    let rebound_host = format!("attacker.example:{port}");
    let off_addr = dial(&off);
    let off_host = format!("Host: 127.0.0.1:{}", off_addr.port());
    let mut operations = 0;
    for (template, ops) in doc["paths"].as_object().unwrap() {
        let path = template
            .replace("{inbox}", "guard142")
            .replace("{id}", "nope")
            .replace("{key}", "k");
        for (method, op) in ops.as_object().unwrap() {
            operations += 1;
            let method = method.to_ascii_uppercase();
            assert_eq!(
                op["responses"]["403"]["$ref"], "#/components/responses/GuardRefused",
                "{method} {template} lists the guard's 403"
            );
            let (st, body) = http_send_text_with_headers(
                srv.http_addr,
                &method,
                &path,
                &[("Origin", hostile_origin.as_str())],
                None,
            )
            .await;
            assert_eq!(st, 403, "{method} {path} with a hostile Origin: {body}");
            assert!(
                body.contains("cross-origin requests are refused on the loopback plane"),
                "{method} {path}: {body}"
            );
            let (st, body) =
                send_with_host(srv.http_addr, &method, &path, &rebound_host, None).await;
            assert_eq!(st, 403, "{method} {path} with a rebound Host: {body}");
            assert!(
                body.contains("requests to the loopback plane must name a loopback Host"),
                "{method} {path}: {body}"
            );
            let (st, _, body) = raw_exchange(
                off_addr,
                &method,
                &path,
                &[off_host.as_str(), "Origin: https://evil.example"],
                "",
            )
            .await;
            assert_eq!(st, 403, "{method} {path} off loopback: {body}");
            assert!(body.contains(MISMATCH), "{method} {path}: {body}");
        }
    }
    assert!(operations >= 39, "every operation was walked: {operations}");
}

/// Issue #142 (D13, D23): each status the document newly publishes is a
/// status a real request on that very route and method answers, and the
/// answer is in that operation's response map — a blank-inbox 400 on the
/// compat list, the clear and the views; a malformed or over-cap compat
/// list 400; the await's cap 400s; the clear's 503 under a real refusing
/// trigger; and the REST bodies' 2 MiB bound — exactly 2097152 bytes
/// accepted, one more answered with the extractor's text/plain 413 — and
/// their 415, plus chaos's 422. Removing any entry is red.
#[tokio::test]
async fn each_published_status_is_answered_on_its_operation() {
    let dir = tempfile::TempDir::with_prefix("swarmail-e2e-pin142-").unwrap();
    let db = dir.path().join("mail.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let doc = served_openapi(srv.http_addr).await;
    let clear = "/api/v1/inboxes/{inbox}/messages";
    for (method, template, path, status) in [
        ("GET", "/api/v1/messages", "/api/v1/messages?inbox=%20", 400),
        ("GET", "/api/v1/messages", "/api/v1/messages?limit=251", 400),
        ("GET", "/api/v1/messages", "/api/v1/messages?limit=x", 400),
        ("DELETE", clear, "/api/v1/inboxes/%20/messages", 400),
        (
            "GET",
            "/api/v1/inboxes/{inbox}/threads",
            "/api/v1/inboxes/%20/threads",
            400,
        ),
        (
            "GET",
            "/api/v1/inboxes/{inbox}/threads/{key}",
            "/api/v1/inboxes/%20/threads/k",
            400,
        ),
        (
            "GET",
            "/api/v1/inboxes/{inbox}/feed",
            "/api/v1/inboxes/%20/feed",
            400,
        ),
        (
            "GET",
            "/api/v1/inboxes/{inbox}/await",
            "/api/v1/inboxes/pin142/await?count=251",
            400,
        ),
        (
            "GET",
            "/api/v1/inboxes/{inbox}/await",
            "/api/v1/inboxes/pin142/await?limit=251",
            400,
        ),
    ] {
        let (st, body) = http_send_text(srv.http_addr, method, path, None).await;
        assert_eq!(st, status, "{method} {path}: {body}");
        assert!(
            documents(&doc, template, method, st),
            "{method} {template} answered {st}, which it must publish"
        );
    }

    // The clear's failed commit: a real rejecting trigger on the data file.
    seed(srv.http_addr, "pin142-clear", "a@x.io").await;
    let trigger = common::refusing_trigger(&db, "delete");
    let (st, body) = http_json(
        srv.http_addr,
        "DELETE",
        "/api/v1/inboxes/pin142-clear/messages",
        None,
    )
    .await;
    assert_eq!(st, 503, "{body}");
    assert_eq!(body["error"], "storage commit failed");
    assert!(documents(&doc, clear, "DELETE", st), "the clear's 503");
    trigger.remove();
    let (st, body) = http_json(
        srv.http_addr,
        "DELETE",
        "/api/v1/inboxes/pin142-clear/messages",
        None,
    )
    .await;
    assert_eq!((st, body), (200, serde_json::json!({"removed": 1})));

    // The REST body bound, on every JSON-body operation.
    let limit = 2_097_152;
    for (method, template, path) in [
        (
            "POST",
            "/api/v1/inboxes/{inbox}/seed",
            "/api/v1/inboxes/pin142-body/seed",
        ),
        ("PUT", "/api/v1/webhooks", "/api/v1/webhooks"),
        ("PUT", "/api/v1/chaos", "/api/v1/chaos"),
    ] {
        let head = format!(
            "{method} {path} HTTP/1.1\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n"
        );
        let (st, head, body) = raw_head_exchange(srv.http_addr, &head, b"{}").await;
        assert_eq!(st, 415, "{path}: {body}");
        assert!(head.contains("content-type: text/plain"), "{path}: {head}");
        let big = vec![b' '; limit + 1];
        let head = format!(
            "{method} {path} HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            big.len()
        );
        let (st, head, body) = raw_head_exchange(srv.http_addr, &head, &big).await;
        assert_eq!(st, 413, "{path}: {body}");
        assert!(head.contains("content-type: text/plain"), "{path}: {head}");
        assert_eq!(
            body,
            "Failed to buffer the request body: length limit exceeded"
        );
        for status in [413, 415] {
            assert!(
                documents(&doc, template, method, status),
                "{method} {template} publishes {status}"
            );
        }
    }
    let too_large = &doc["components"]["responses"]["BodyTooLarge"];
    assert!(
        too_large["content"]["text/plain"].is_object(),
        "{too_large}"
    );
    assert!(
        too_large["description"]
            .as_str()
            .unwrap()
            .contains(&format!("{limit}-byte")),
        "{too_large}"
    );
    // Exactly the bound is a body, not a refusal.
    let prefix = r#"{"to": "a@x.io", "text": ""#;
    let mut exact = prefix.to_string();
    exact.push_str(&"x".repeat(limit - prefix.len() - 2));
    exact.push_str(r#""}"#);
    assert_eq!(exact.len(), limit);
    let (st, body) = http_send_text(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/pin142-body/seed",
        Some(&exact),
    )
    .await;
    assert_eq!(
        st,
        200,
        "a {limit}-byte seed is accepted: {}",
        &body[..80.min(body.len())]
    );

    let (st, body) = http_send_text(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100, "eror": "x"}}"#),
    )
    .await;
    assert_eq!(st, 422, "{body}");
    assert!(documents(&doc, "/api/v1/chaos", "PUT", st), "chaos's 422");
    srv.stop().await;
}

/// Issue #142 (D24): a thread's `subject` is the oldest member's that has
/// one, as published — never the newest mail's — and the key is absent,
/// not null, when no member has one. The fixtures are generated per run
/// with distinguishing subjects and explicit receive times: a two-mail
/// chain whose reply retitles it, a chain whose subject-less root is
/// titled only by its reply, and a lone subject-less mail.
#[tokio::test]
async fn the_thread_subject_is_the_published_oldest_subject() {
    let srv = start().await;
    let doc = served_openapi(srv.http_addr).await;
    let subject = &doc["components"]["schemas"]["Thread"]["properties"]["subject"];
    assert_eq!(subject["type"], "string", "optional, never null: {subject}");
    let words = subject["description"].as_str().unwrap();
    assert!(
        words.contains("oldest mail in the thread that has one")
            && !words.contains("newest mail's"),
        "{subject}"
    );
    assert!(
        words.contains("absent when no mail in the thread has a subject"),
        "{subject}"
    );
    let inbox = "pin142-threads";
    // Inserted out of receive order: the thread orders by receive time.
    for (raw, ms) in [
        (
            "Message-ID: <b@pin142>\r\nIn-Reply-To: <a@pin142>\r\nSubject: Re: changed course\r\n\r\nreply",
            2_000,
        ),
        (
            "Message-ID: <a@pin142>\r\nSubject: Original plan\r\n\r\nroot",
            1_000,
        ),
        (
            "Message-ID: <d@pin142>\r\nIn-Reply-To: <c@pin142>\r\nSubject: Named later\r\n\r\nreply",
            4_000,
        ),
        ("Message-ID: <c@pin142>\r\n\r\nuntitled root", 3_000),
        ("Message-ID: <e@pin142>\r\n\r\nalone", 5_000),
    ] {
        let mut email = swarmail::smtp::build_email(
            raw.as_bytes(),
            inbox,
            &["a@x.io".to_string()],
            "s@x.io".into(),
        );
        email.received_ms = ms;
        srv.store.insert(email).unwrap();
    }
    let path = format!("/api/v1/inboxes/{inbox}/threads");
    let (st, threads) = http_json(srv.http_addr, "GET", &path, None).await;
    assert_eq!(st, 200, "{threads}");
    let counts: Vec<u64> = threads
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["count"].as_u64().unwrap())
        .collect();
    assert_eq!(counts, [1, 2, 2], "newest thread first: {threads}");

    let retitled = &threads[2];
    assert_eq!(retitled["emails"][1]["subject"], "Re: changed course");
    assert_eq!(
        retitled["subject"], "Original plan",
        "the oldest, not the newest"
    );
    let titled_late = &threads[1];
    assert!(
        titled_late["emails"][0].get("subject").is_none(),
        "the root has no subject: {titled_late}"
    );
    assert_eq!(
        titled_late["subject"], "Named later",
        "the oldest member that has a subject"
    );
    let untitled = threads[0].as_object().unwrap();
    assert!(
        !untitled.contains_key("subject"),
        "absent, not null: {untitled:?}"
    );

    let key = retitled["key"].as_str().unwrap();
    let (st, one) = http_json(srv.http_addr, "GET", &format!("{path}/{key}"), None).await;
    assert_eq!((st, one["subject"].clone()), (200, "Original plan".into()));
}

/// Supplemental public-router proof: complete request data lets a single
/// synchronous poll distinguish explicit zero from a defaulted/positive wait.
/// The network test above separately retains the real HTTP contract.
#[tokio::test]
async fn zero_timeout_finishes_on_the_first_router_poll() {
    use axum::{body::Body, http::Request};
    use std::{
        future::Future,
        sync::{Arc, atomic::AtomicBool},
        task::{Context, Poll},
    };
    use swarmail::{
        api::{self, AppState},
        chaos::Chaos,
        store::Store,
        webhook::Webhooks,
    };

    async fn call<S>(mut service: S, request: Request<Body>) -> Result<S::Response, S::Error>
    where
        S: axum::ServiceExt<Request<Body>>,
    {
        std::future::poll_fn(|cx| service.poll_ready(cx)).await?;
        service.call(request).await
    }

    for (name, query, ready) in [
        ("router-zero", "&timeout_ms=0", true),
        ("router-default", "", false),
        ("router-positive", "&timeout_ms=60000", false),
    ] {
        let (stop, rx) = tokio::sync::watch::channel(false);
        let state = AppState {
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            webhooks: Arc::new(Webhooks::default()),
            started: std::time::Instant::now(),
            shutdown: rx,
            smtp_degraded: Arc::new(AtomicBool::new(false)),
            pop3_degraded: Arc::new(AtomicBool::new(false)),
            http_origin_guard: None,
        };
        let req = Request::builder()
            .uri(format!("/api/v1/inboxes/{name}/await?count=1{query}"))
            .body(Body::empty())
            .unwrap();
        let mut request = std::pin::pin!(call(api::router(state), req));
        let observed = request.as_mut().poll(&mut Context::from_waker(
            futures_util::task::noop_waker_ref(),
        ));
        assert_eq!(observed.is_ready(), ready, "first router poll: {name}");
        let response = match observed {
            Poll::Ready(response) => response.unwrap(),
            Poll::Pending => {
                stop.send(true).unwrap();
                request.await.unwrap()
            }
        };
        assert_eq!(response.status().as_u16(), if ready { 408 } else { 503 });
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if ready {
            assert_eq!(body["matched"], 0);
        } else {
            assert_eq!(body["error"], "shutting down");
        }
    }
}
