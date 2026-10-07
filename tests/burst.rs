//! End-to-end guarantees: the whole point of Swarmail.
//!
//! These tests bind the real SMTP + HTTP servers on ephemeral ports and speak
//! real protocol — no mocks, no stubs.

mod common;

use common::{http_json, smtp_send, start};

#[tokio::test]
async fn single_mail_roundtrip_with_extraction() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        None,
        "noreply@alkem.io",
        "user@example.com",
        "Verify your account",
        "Click https://x.io/verify?t=abc. Your code is 424242.",
    )
    .await
    .unwrap();

    let (status, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    let email = &body["emails"][0];
    assert_eq!(email["subject"], "Verify your account");
    assert_eq!(email["to"][0]["address"], "user@example.com");
    assert_eq!(email["links"][0], "https://x.io/verify?t=abc");
    assert_eq!(email["codes"][0], "424242");
}

#[tokio::test]
async fn the_burst_guarantee() {
    // 50 concurrent sessions x 20 mails = 1000. Every accepted DATA must be
    // queryable, immediately, exactly — the promise MailCrab cannot make.
    let srv = start().await;
    const CONNS: usize = 50;
    const PER_CONN: usize = 20;

    let mut tasks = Vec::new();
    for c in 0..CONNS {
        let smtp_addr = srv.smtp_addr;
        tasks.push(tokio::spawn(async move {
            for i in 0..PER_CONN {
                smtp_send(
                    smtp_addr,
                    None,
                    "noreply@test.io",
                    &format!("user{c}-{i}@example.com"),
                    &format!("burst {c}/{i}"),
                    "load",
                )
                .await
                .unwrap_or_else(|e| panic!("conn {c} mail {i} rejected: {e}"));
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(
        body["count"], 1000,
        "every accepted mail must be stored — no silent loss"
    );
}

#[tokio::test]
async fn await_waits_for_arrival() {
    let srv = start().await;

    // Deliver 300ms in the future; await must block until it arrives.
    let smtp_addr = srv.smtp_addr;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        smtp_send(
            smtp_addr,
            None,
            "noreply@x.io",
            "late@example.com",
            "late mail",
            "hello",
        )
        .await
        .unwrap();
    });

    let (status, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/await?to=late@example.com&timeout_ms=5000&count=1",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["matched"], 1);
    assert_eq!(body["emails"][0]["subject"], "late mail");
}

#[tokio::test]
async fn await_times_out_cleanly() {
    let srv = start().await;
    let (status, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/await?to=nobody@example.com&timeout_ms=200",
        None,
    )
    .await;
    assert_eq!(status, 408);
    assert_eq!(body["matched"], 0);
}

#[tokio::test]
async fn per_inbox_isolation_via_auth() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("proj-42"),
        "noreply@x.io",
        "a@example.com",
        "s",
        "b",
    )
    .await
    .unwrap();
    smtp_send(
        srv.smtp_addr,
        None,
        "noreply@x.io",
        "b@example.com",
        "s",
        "b",
    )
    .await
    .unwrap();

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/proj-42/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 1);

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 1);
}

#[tokio::test]
async fn chaos_can_reject_rcpt_and_recovery_works() {
    let srv = start().await;

    // 100% RCPT rejection.
    let (status, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"rcpt": {"probability": 100, "error": "451 4.3.0 chaos"}}"#),
    )
    .await;
    assert_eq!(status, 200);

    let err = smtp_send(srv.smtp_addr, None, "n@x.io", "a@example.com", "s", "b")
        .await
        .unwrap_err();
    assert!(err.contains("451"), "chaos error must surface: {err}");

    // Chaos off — delivery works again.
    let (status, _) = http_json(srv.http_addr, "DELETE", "/api/v1/chaos", None).await;
    assert_eq!(status, 200);
    smtp_send(srv.smtp_addr, None, "n@x.io", "a@example.com", "s", "b")
        .await
        .unwrap();

    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(body["count"], 1);
}
