//! Restart persistence: real mail over real SMTP + HTTP, a full server
//! restart against the same `--data-file`, and the same mail answered again —
//! with raw bytes, restored counters and mirrored deletes/clears.

mod common;

use std::path::PathBuf;
use swarmail::RunningServer;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

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

    // clear_all is mirrored too — reached through the explicit wipe alias
    // (decision 0011: the shim DELETE no longer falls back to it).
    let (st, _) = common::http_json(srv.http_addr, "DELETE", "/api/v1/delete-all", None).await;
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

/// A long-lived pipelined SMTP sender: one TCP connection (nodelay), each
/// mail is a single batched write — the server advertises PIPELINING — and
/// the four replies read back. This keeps ingest in the thousands of mails
/// per second, dense enough that inserts land inside the clear's drain
/// window instead of trickling past it (a fresh connection per mail with
/// un-pipelined round-trips trickles at ~25/s and the inbox is empty at
/// every clear, so no race is ever sampled).
struct BurstSender {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl BurstSender {
    async fn connect(addr: std::net::SocketAddr, inbox: &str) -> Self {
        let stream = TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (r, w) = stream.into_split();
        let mut reader = BufReader::new(r);
        let mut writer = w;
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("220"), "{line}");
        writer.write_all(b"EHLO swarmail-race\r\n").await.unwrap();
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if !line.starts_with("250-") {
                break;
            }
        }
        let plain = format!("\u{0}{inbox}\u{0}whatever");
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
        writer
            .write_all(format!("AUTH PLAIN {b64}\r\n").as_bytes())
            .await
            .unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("235"), "{line}");
        Self { reader, writer }
    }

    /// One batch: K mails' command blocks in a single write (the replies
    /// coalesce the same way), then the 4K reply lines read back — every
    /// fourth is the mail's acceptance. Returns the ids of every mail whose
    /// `250` line was fully observed (an observed acceptance is durable —
    /// the commit precedes the reply byte).
    async fn send_batch(&mut self, subjects: &[String]) -> Vec<String> {
        let mut mail = String::new();
        for subject in subjects {
            mail.push_str(&format!(
                "MAIL FROM:<sender@x.io>\r\nRCPT TO:<rcpt@x.io>\r\nDATA\r\n\
                 From: sender@x.io\r\nSubject: {subject}\r\n\r\nbody\r\n.\r\n"
            ));
        }
        self.writer.write_all(mail.as_bytes()).await.unwrap();
        let mut observed = Vec::new();
        let mut line = String::new();
        for i in 0..subjects.len() * 4 {
            line.clear();
            self.reader.read_line(&mut line).await.unwrap();
            if i % 4 == 3 {
                assert!(line.starts_with("250"), "mail rejected: {line}");
                observed.push(
                    line.trim()
                        .rsplit_once("stored as ")
                        .expect("the 250 names the stored id")
                        .1
                        .to_string(),
                );
            }
        }
        observed
    }
}

// Multi-threaded on purpose: `run_on` spawns the servers onto the caller's
// runtime, and a current-thread runtime would serialize the clear's
// synchronous drain+mirror against every insert — the race would be
// structurally unsampleable and the test vacuously green.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_clear_racing_inserts_never_loses_a_250_answered_mail_on_restart() {
    let db = data_file("clear-race");
    clean_up(&db);

    let srv = serve_data_file(&db).await;
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Six burst senders of real SMTP mail, racing the clear below: an insert
    // that lands after the clear's in-memory drain but before its mirrored
    // delete is answered 250 and stays queryable — its row must survive.
    // Batched PIPELINING keeps ingest in the thousands of mails per second;
    // a fresh connection per mail trickles at ~25/s (Nagle/delayed-ACK per
    // round-trip) and the inbox is empty at every clear, so no race is ever
    // sampled.
    let mut inserters = Vec::new();
    for worker in 0..6u32 {
        let addr = srv.smtp_addr;
        let stop = stop.clone();
        inserters.push(tokio::spawn(async move {
            let mut sender = BurstSender::connect(addr, "race").await;
            let mut sent = 0u32;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let subjects: Vec<String> = (0..100)
                    .map(|n| format!("race-{worker}-{}-{n}", sent + n))
                    .collect();
                sender.send_batch(&subjects).await;
                sent += 100;
            }
            sent
        }));
    }

    // The racing party: real HTTP wipes of the same inbox, throttled so the
    // inbox is fat at every drain — a fat drain is a wide window (it walks
    // every mail's id out of the index), and a wide window is one a dense
    // ingest reliably races into. A back-to-back clear loop on an empty
    // inbox samples nothing.
    let clearer = {
        let http = srv.http_addr;
        tokio::spawn(async move {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
            let mut rounds = 0u32;
            while std::time::Instant::now() < deadline {
                let _ =
                    common::http_json(http, "DELETE", "/api/v1/inboxes/race/messages", None).await;
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                rounds += 1;
            }
            rounds
        })
    };

    let rounds = clearer.await.unwrap();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    // Quiesce every sender before snapshotting: a 250 implies the row is
    // already committed (the mirror rides the accept path), so anything
    // queryable below is on disk unless a clear erased it.
    let mut sent_total = 0u32;
    for inserter in inserters {
        sent_total += inserter.await.unwrap();
    }
    assert!(sent_total > 0, "the inserters never got a mail accepted");
    // The loop ran for the whole window (its own deadline bounds it), so any
    // count above zero means the wipes overlapped the dense ingest. A round
    // floor higher than that would measure the machine's speed, not the
    // race — an instrumented build fits fewer fat wipes into the window.
    assert!(
        rounds > 0,
        "the clear loop never ran a wipe: {rounds} rounds"
    );

    // What the store still holds is exactly what the restart must give back.
    let (st, pre) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/race/messages?limit=1000000",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let pre_ids: std::collections::HashSet<String> = pre["emails"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    assert!(!pre_ids.is_empty(), "the race produced no mail to check");

    srv.stop().await;
    let srv = serve_data_file(&db).await;
    let (st, post) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/race/messages?limit=1000000",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let post_ids: std::collections::HashSet<String> = post["emails"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    let lost: Vec<&String> = pre_ids.difference(&post_ids).collect();
    assert!(
        lost.is_empty(),
        "restart lost {} mail(s) the pre-restart store still held: {lost:?} — the racing clear erased rows memory kept",
        lost.len()
    );
    // Note on the mirror-image hazard, deliberately not asserted here: a mail
    // pushed just before the drain whose INSERT commit lands *after* the
    // clear's DELETE commit resurfaces on restart (the row is written after
    // the erase). That ordering race exists identically with the old
    // inbox-wide DELETE and with the per-id mirror — it is a separate,
    // pre-existing hazard (an epoch/generation scheme would be needed to
    // close it), tracked as new debt rather than smuggled into this fix.
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

/// The write-heavy leg: more concurrent pipelined writer sessions than the
/// runtime has workers (eight sessions on a two-worker runtime), a separate
/// connection keeping the server live with NOOPs throughout, and every
/// `250`-answered id required back — queryable while live, present in the
/// restart copy. The NOOP bound only catches a wedged server, never a slow
/// one; the structural stall proof lives in the in-crate pin, and the kill
/// proof (a server that dies mid-burst) lives in the binary suite — this leg
/// is the sustained-load one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_heavy_data_file_server_stays_live_and_loses_no_250_answered_mail() {
    let db = data_file("write-heavy");
    clean_up(&db);
    let srv = serve_data_file(&db).await;

    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Eight writer sessions on two workers: if a commit ever ran on the
    // runtime thread again, this is the load that would starve it.
    let mut writers = Vec::new();
    for w in 0..8u32 {
        let addr = srv.smtp_addr;
        let stop = stop.clone();
        writers.push(tokio::spawn(async move {
            let mut sender = BurstSender::connect(addr, "heavy").await;
            let mut observed: Vec<String> = Vec::new();
            let mut sent = 0u32;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let subjects: Vec<String> = (0..25)
                    .map(|n| format!("heavy-{w}-{}-{n}", sent + n))
                    .collect();
                observed.extend(sender.send_batch(&subjects).await);
                sent += 25;
            }
            observed
        }));
    }

    // The liveness probe: its own connection, NOOP → 250, over and over,
    // while the writers hammer the same data file. A generous 10 s bound —
    // a wedged (not slow) server would hang a round past it.
    let nooper = {
        let addr = srv.smtp_addr;
        let stop = stop.clone();
        tokio::spawn(async move {
            let mut conn = common::SmtpConn::connect(addr).await;
            let mut rounds = 0u32;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    conn.send("NOOP").await;
                    let reply = conn.reply().await;
                    assert!(reply.starts_with("250"), "NOOP rejected: {reply}");
                })
                .await
                .expect("the server wedged: a NOOP got no reply within 10 s while writes were in flight");
                rounds += 1;
            }
            rounds
        })
    };

    // A bounded hammer window, then quiesce everything.
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let mut observed: Vec<String> = Vec::new();
    for writer in writers {
        observed.extend(writer.await.unwrap());
    }
    let noop_rounds = nooper.await.unwrap();
    assert!(
        !observed.is_empty(),
        "the writers never got a mail accepted"
    );
    assert!(
        noop_rounds > 0,
        "the NOOP probe never completed a round: {noop_rounds}"
    );

    // Every observed acceptance is queryable while the server is live —
    // a 250 means the mail is already there (decision 0001).
    let (st, live) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/heavy/messages?limit=1000000",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let live_ids: std::collections::HashSet<String> = live["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["id"].as_str().expect("id").to_string())
        .collect();
    for id in &observed {
        assert!(
            live_ids.contains(id),
            "a mail answered 250 ({id}) is not queryable while the server is live"
        );
    }

    // And the restart copy gives every one of them back: a write-heavy
    // window is exactly where a soft flush would drop rows.
    srv.stop().await;
    let srv = serve_data_file(&db).await;
    let (st, post) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/heavy/messages?limit=1000000",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let post_ids: std::collections::HashSet<String> = post["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["id"].as_str().expect("id").to_string())
        .collect();
    let lost: Vec<&String> = observed
        .iter()
        .filter(|id| !post_ids.contains(*id))
        .collect();
    assert!(
        lost.is_empty(),
        "restart lost {} mail(s) answered 250 under write-heavy load: {:?}",
        lost.len(),
        &lost[..lost.len().min(5)]
    );
    srv.stop().await;
    clean_up(&db);
}

/// Contract #1's SMTP wire leg (decision 0012 §3): with a second real
/// connection holding the writer lock, the next DATA's commit fails and
/// the reply is the fixed `451 4.3.0 storage commit failed` line — never a
/// `250`. The memory truth stands (the refused mail is queryable over
/// REST), the restart copy taken without the lock does not contain it, and
/// once the lock is released the same wire path accepts again.
#[tokio::test]
async fn a_blocked_commit_answers_451_and_never_claims_250() {
    let db = data_file("refuse-smtp");
    clean_up(&db);

    let srv = serve_data_file(&db).await;
    // One committed mail first: the mirror works before the lock lands.
    let committed = send(&srv, "refuse", "committed before the lock").await;
    assert!(!committed.is_empty());

    let _lock = common::hold_write_lock(&db);
    let reply = common::smtp_send(
        srv.smtp_addr,
        Some("refuse"),
        "sender@x.io",
        "rcpt@x.io",
        "refused while locked",
        "the body",
    )
    .await
    .expect_err("the blocked commit must refuse the DATA");
    assert!(
        reply.starts_with("451 4.3.0 storage commit failed"),
        "the blocked commit must answer the fixed refusal line, got: {reply}"
    );
    assert!(
        !reply.contains("250"),
        "no acceptance may ride the refusal: {reply}"
    );

    // Memory truth: the refused mail is queryable while live — only the
    // durability claim is withdrawn, not the storage (decision 0012 §6).
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/refuse/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let subjects: Vec<&str> = list["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["subject"].as_str().expect("subject"))
        .collect();
    assert!(
        subjects.contains(&"refused while locked"),
        "the refused mail is not queryable while live: {subjects:?}"
    );

    // The restart copy, taken with the lock released, holds the committed
    // mail and not the refused one: nothing claimed a 250 for it, and no
    // row exists to restore.
    srv.stop().await;
    drop(_lock);
    let srv = serve_data_file(&db).await;
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/refuse/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let subjects: Vec<&str> = list["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["subject"].as_str().expect("subject"))
        .collect();
    assert_eq!(
        subjects,
        vec!["committed before the lock"],
        "the refused mail must not reach the restart copy"
    );

    // The lock is gone: the same wire path accepts again.
    let retried = send(&srv, "refuse", "accepted after release").await;
    assert!(!retried.is_empty(), "the retry must be accepted with a 250");
    srv.stop().await;
    clean_up(&db);
}

/// Contract #1's REST seed leg: the blocked commit refuses the seed with
/// the fixed `503` + `storage commit failed` body, the memory truth still
/// stands, and the restart copy stays empty — the seed's caller is told
/// the truth instead of a `200` for an uncommitted row.
#[tokio::test]
async fn a_blocked_commit_refuses_the_rest_seed_with_503() {
    let db = data_file("refuse-rest");
    clean_up(&db);

    let srv = serve_data_file(&db).await;
    let _lock = common::hold_write_lock(&db);
    let (st, body) = common::http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/blocked/seed",
        Some(r#"{"to":"fixture@x.io","subject":"refused seed","text":"hello"}"#),
    )
    .await;
    assert_eq!(st, 503, "the blocked commit must refuse the seed: {body}");
    assert_eq!(body["error"], "storage commit failed");

    // Memory truth: the refused seed is queryable while live.
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/blocked/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 1, "the refused seed must stay queryable");
    assert_eq!(list["emails"][0]["subject"], "refused seed");

    // And it is not durable: the restart copy has no row for it.
    srv.stop().await;
    drop(_lock);
    let srv = serve_data_file(&db).await;
    let (st, list) = common::http_json(
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
    clean_up(&db);
}
