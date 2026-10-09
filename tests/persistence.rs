//! Restart persistence: real mail over real SMTP + HTTP, a full server
//! restart against the same `--data-file`, and the same mail answered again —
//! with raw bytes, restored counters and mirrored deletes/clears.

mod common;

use std::path::PathBuf;
use swarmail::RunningServer;

/// A unique data-file path; parallel tests must never share one.
fn data_file(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "swarmail-e2e-{tag}-{}-{nanos}.db",
        std::process::id()
    ))
}

/// Best-effort cleanup of the data file and its WAL sidecars.
fn clean_up(db: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

async fn serve_data_file(db: &std::path::Path) -> RunningServer {
    common::start_with(|mut c| {
        c.data_file = Some(db.to_path_buf());
        c
    })
    .await
}

async fn send(srv: &RunningServer, inbox: &str, subject: &str) -> String {
    let reply = common::smtp_send(
        srv.smtp_addr,
        Some(inbox),
        "sender@x.io",
        "rcpt@x.io",
        subject,
        "the body",
    )
    .await
    .expect("smtp send");
    reply.split_whitespace().last().unwrap().to_string()
}

#[tokio::test]
async fn mail_sent_over_smtp_survives_a_full_restart() {
    let db = data_file("restart");
    clean_up(&db);

    let srv = serve_data_file(&db).await;
    let smtp_id = send(&srv, "persist", "Hello persist").await;
    let (st, _) = common::http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/seeded/seed",
        Some(
            r#"{"to":"fixture@x.io","subject":"Seeded mail","text":"Your code is 424242 thanks"}"#,
        ),
    )
    .await;
    assert_eq!(st, 200, "seed failed");
    srv.stop().await;

    let srv = serve_data_file(&db).await;
    // Both inboxes are back, with their mail.
    let (st, inboxes) = common::http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200);
    let names: Vec<&str> = inboxes
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["persist", "seeded"]);

    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/persist/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 1);
    assert_eq!(list["emails"][0]["id"], smtp_id.as_str());
    assert_eq!(list["emails"][0]["subject"], "Hello persist");
    assert_eq!(list["emails"][0]["from"]["address"], "sender@x.io");

    // The raw bytes are the ones that were accepted.
    let (st, raw) =
        common::http_get_text(srv.http_addr, &format!("/api/v1/messages/{smtp_id}/raw")).await;
    assert_eq!(st, 200);
    assert!(raw.contains("Subject: Hello persist"), "raw lost: {raw:?}");
    assert!(raw.contains("the body"), "raw lost: {raw:?}");

    // The seeded path (full parse/extract pipeline) is persisted too.
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/seeded/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["emails"][0]["subject"], "Seeded mail");
    assert_eq!(list["emails"][0]["codes"], serde_json::json!(["424242"]));

    // The wait path answers immediately from restored state: subscribe →
    // scan finds the restored mail (decision 0002 ordering intact).
    let (st, awaited) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/persist/await?count=1&timeout_ms=1000",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(awaited["matched"], 1);
    assert_eq!(awaited["emails"][0]["id"], smtp_id.as_str());

    // Lifetime counters are restored, not reset.
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 2"),
        "inserted counter not restored: {metrics}"
    );

    srv.stop().await;
    clean_up(&db);
}

#[tokio::test]
async fn deletes_and_clears_survive_a_restart() {
    let db = data_file("mutations");
    clean_up(&db);

    let srv = serve_data_file(&db).await;
    for subject in ["one", "two"] {
        send(&srv, "box-a", subject).await;
    }
    send(&srv, "box-b", "three").await;
    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/box-a/messages", None).await;
    // Newest first: deleting the newest must leave the oldest, and the
    // deleted row must not come back with the restart.
    assert_eq!(list["emails"][0]["subject"], "two");
    let first = list["emails"][0]["id"].as_str().unwrap().to_string();
    let (st, _) = common::http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{first}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "delete failed");
    let (st, cleared) = common::http_json(
        srv.http_addr,
        "DELETE",
        "/api/v1/inboxes/box-b/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(cleared["removed"], 1);
    srv.stop().await;

    let srv = serve_data_file(&db).await;
    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/box-a/messages", None).await;
    assert_eq!(list["total"], 1, "the surviving mail must survive");
    assert_eq!(list["emails"][0]["subject"], "one");
    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/box-b/messages", None).await;
    assert_eq!(list["total"], 0, "the cleared inbox must stay cleared");
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 3"),
        "{metrics}"
    );

    // clear_all is mirrored too.
    let (st, _) = common::http_json(srv.http_addr, "DELETE", "/api/v1/messages", None).await;
    assert_eq!(st, 200);
    srv.stop().await;

    let srv = serve_data_file(&db).await;
    let (_, inboxes) = common::http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert!(
        inboxes.as_array().unwrap().is_empty(),
        "clear_all not mirrored"
    );
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 3"),
        "{metrics}"
    );
    srv.stop().await;
    clean_up(&db);
}

#[tokio::test]
async fn counters_keep_counting_across_restarts() {
    let db = data_file("counters");
    clean_up(&db);

    let srv = serve_data_file(&db).await;
    send(&srv, "box", "first").await;
    send(&srv, "box", "second").await;
    srv.stop().await;

    // The counter continues from its restored value: this mail is #3, not #1.
    let srv = serve_data_file(&db).await;
    send(&srv, "box", "third").await;
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 3"),
        "counter restarted from zero: {metrics}"
    );
    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/box/messages", None).await;
    assert_eq!(list["total"], 3);
    srv.stop().await;
    clean_up(&db);
}

#[tokio::test]
async fn a_broken_data_file_refuses_to_serve() {
    let cfg = swarmail::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        // A directory that does not exist: no half-open server may answer.
        data_file: Some(std::env::temp_dir().join("swarmail-no-such-dir/x.db")),
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
    };
    let err = match swarmail::run_on(&cfg).await {
        Ok(_) => panic!("a broken data file must refuse to serve"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("data file"), "{err}");
}
