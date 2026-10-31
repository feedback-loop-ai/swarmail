//! Restart persistence: real mail over real SMTP + HTTP, a full server
//! restart against the same `--data-file`, and the same mail answered again —
//! with raw bytes, restored counters and mirrored deletes/clears.

mod common;

use std::path::PathBuf;
use swarmail::RunningServer;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// A unique data-file path inside its own scratch directory; parallel tests
/// never share one. The returned guard owns the directory: the data file,
/// its WAL sidecars and its lock go with it on every exit, so a test that
/// fails mid-way leaves nothing in the temporary root (issue #143). Bind
/// it for the whole test (`let (_dir, db) = ...`), never to `_`.
fn data_file(tag: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::TempDir::with_prefix(format!("swarmail-e2e-{tag}-")).unwrap();
    let db = dir.path().join("mail.db");
    (dir, db)
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
    let (_dir, db) = data_file("restart");

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
}

#[tokio::test]
async fn deletes_and_clears_survive_a_restart() {
    let (_dir, db) = data_file("mutations");

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
}

/// One rule on every path (decision 0032, issue #54): an inbox whose mail
/// is gone does not exist — a `delete` that empties it and a `clear` both
/// remove its entry, live and after a reopen, so the listing never carries
/// an empty shell. The next insert re-mints (0004): no tombstone.
#[tokio::test]
async fn an_emptied_inbox_is_absent_from_the_listing_live_and_after_a_reopen() {
    let (_dir, db) = data_file("emptied-listing");

    let srv = serve_data_file(&db).await;
    let gone = send(&srv, "emptied", "one").await;
    // `kept` is the inbox that must survive the emptied one's removal.
    let kept_id = send(&srv, "kept", "two").await;

    // Deleting the only "emptied" mail empties — and removes — the inbox.
    let (st, _) = common::http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{gone}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    // LIVE: the listing carries only the inbox that still holds mail, and
    // the gauge agrees — no empty entry counts.
    let names = |inboxes: &serde_json::Value| -> Vec<String> {
        inboxes
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap().to_string())
            .collect()
    };
    let (st, inboxes) = common::http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200);
    assert_eq!(names(&inboxes), vec!["kept".to_string()], "live listing");
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(metrics.contains("swarmail_inboxes 1"), "{metrics}");
    // The removal touched nothing else: the other inbox's mail still lives.
    let (st, _) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{kept_id}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "the surviving mail is untouched");
    // The per-inbox read routes keep their shape for an absent inbox (an
    // empty list, not a 404): only the *listing* stops naming it.
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/emptied/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 0);

    // `clear` follows the same rule — the emptied "kept" goes away too.
    let (st, _) = common::http_json(
        srv.http_addr,
        "DELETE",
        "/api/v1/inboxes/kept/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let (_, inboxes) = common::http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert!(
        inboxes.as_array().unwrap().is_empty(),
        "the cleared inbox is out of the live listing: {inboxes}"
    );
    srv.stop().await;

    // REOPEN: restore mints entries only for inboxes with rows, so the
    // listing matches the live one — parity, not a restart-only shape.
    let srv = serve_data_file(&db).await;
    let (st, inboxes) = common::http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200);
    assert!(
        inboxes.as_array().unwrap().is_empty(),
        "no empty inbox is restored: {inboxes}"
    );
    for name in ["emptied", "kept"] {
        let (st, list) = common::http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{name}/messages"),
            None,
        )
        .await;
        assert_eq!(st, 200);
        assert_eq!(list["total"], 0, "{name} is gone, not an empty shell");
    }
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(metrics.contains("swarmail_inboxes 0"), "{metrics}");

    // No tombstone: a fresh insert re-mints the inbox.
    let back = send(&srv, "emptied", "again").await;
    let (st, inboxes) = common::http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(st, 200);
    assert_eq!(names(&inboxes), vec!["emptied".to_string()]);
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/emptied/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 1);
    assert_eq!(list["emails"][0]["id"], back.as_str());

    srv.stop().await;
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
    let (_dir, db) = data_file("clear-race");

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
    let pre_ids = common::list_all_ids(srv.http_addr, "race").await;
    assert!(!pre_ids.is_empty(), "the race produced no mail to check");

    srv.stop().await;
    let srv = serve_data_file(&db).await;
    let post_ids = common::list_all_ids(srv.http_addr, "race").await;
    let lost: Vec<&String> = pre_ids.difference(&post_ids).collect();
    assert!(
        lost.is_empty(),
        "restart lost {} mail(s) the pre-restart store still held: {lost:?} — the racing clear erased rows memory kept",
        lost.len()
    );
    // The mirror-image hazard this note once tracked — a mail pushed just
    // before the drain whose INSERT commit lands *after* the clear's DELETE
    // commit, resurfacing on restart — is closed structurally by decision
    // 0014's one ordered boundary: plan → commit → apply share one lock, so
    // a commit can never land after the erase that memory already applied.
    // The 16-thread probe in `src/store.rs` holds that property on the wire.
    srv.stop().await;
}

#[tokio::test]
async fn counters_keep_counting_across_restarts() {
    let (_dir, db) = data_file("counters");

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

    // A refused insert moves no counter (decision 0014): with a real
    // rejecting trigger on the data file the send is refused, the lifetime
    // counters stand exactly where they were, and the next accepted send
    // resumes from there — no gap, no rewind.
    let trigger = common::refusing_trigger(&db, "insert");
    let reply = common::smtp_send(
        srv.smtp_addr,
        Some("box"),
        "sender@x.io",
        "rcpt@x.io",
        "refused while counting",
        "the body",
    )
    .await
    .expect_err("the refused insert must not answer 250");
    assert!(
        reply.starts_with("451 4.3.0 storage commit failed"),
        "got: {reply}"
    );
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 3\n"),
        "a refused insert must not count: {metrics}"
    );
    trigger.remove();
    send(&srv, "box", "fourth").await;
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 4\n"),
        "the counter must resume without a gap: {metrics}"
    );
    srv.stop().await;
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
        pop3_idle_timeout: swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
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
    let (_dir, db) = data_file("write-heavy");
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
    // a 250 means the mail is already there (decision 0001). The read
    // pages like any client (decision 0030 §3).
    let live_ids = common::list_all_ids(srv.http_addr, "heavy").await;
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
    let post_ids = common::list_all_ids(srv.http_addr, "heavy").await;
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
}

/// Contract #1's SMTP wire leg (decision 0012 §3): with a second real
/// connection holding the writer lock, the next DATA's commit fails and
/// the reply is the fixed `451 4.3.0 storage commit failed` line — never a
/// `250`. The refused mail is absent from the live inbox and restart
/// copy, while the previously committed mail remains in both. After the lock
/// is released, this test confirms the same wire path accepts a later mail.
#[tokio::test]
async fn a_blocked_commit_answers_451_and_never_claims_250() {
    let (_dir, db) = data_file("refuse-smtp");

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

    // Zero trace (decision 0014): the refused mail is nowhere in memory —
    // the zero assertion for its subject matches nothing — and the
    // committed-first mail is untouched. Nothing half-applied, nothing
    // queryable that no restart copy can back.
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/refuse/assert?count=0&subject=refused%20while%20locked",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        body["matched"], 0,
        "the refused mail must be nowhere while live: {body}"
    );
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
        "the refused mail must not be queryable while live, and the committed mail must be untouched: {subjects:?}"
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
}

/// Contract #1's REST seed leg: the blocked commit refuses the seed with
/// the fixed `503` + `storage commit failed` body. The live list is empty,
/// the await times out with `matched=0`, and the restart copy is empty. This
/// test does not retry after the lock is released.
#[tokio::test]
async fn a_blocked_commit_refuses_the_rest_seed_with_503() {
    let (_dir, db) = data_file("refuse-rest");

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

    // Zero trace (decision 0014): the refused seed is nowhere in memory —
    // not even minted as an inbox — so the list is empty and the await
    // matches nothing.
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
        "the refused seed must not be queryable while live: {list}"
    );
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/blocked/await?count=1&timeout_ms=300",
        None,
    )
    .await;
    assert_eq!(st, 408, "the refused seed must match nothing: {body}");
    assert_eq!(body["matched"], 0);

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
}

/// A real HTTP sink that answers 204 and captures every request head until
/// stopped: the webhook leg's receipt. The refusal must fire nothing; the
/// recovery send is the positive control that the target really is wired.
async fn capture_sink(
    stop: tokio::sync::watch::Receiver<bool>,
) -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Receiver<Vec<String>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let mut stop = stop;
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                accepted = listener.accept() => {
                    let Ok((mut stream, _)) = accepted else { break };
                    let mut buf = vec![0u8; 8192];
                    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                        .await
                        .unwrap_or(0);
                    captured.push(String::from_utf8_lossy(&buf[..n]).to_string());
                    let _ = stream
                        .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
                        .await;
                }
            }
        }
        let _ = tx.send(captured);
    });
    (addr, rx)
}

/// Poll one metrics line until it reads exactly `want`, or fail with the
/// last seen text: webhook delivery is asynchronous, so the leg waits for
/// the receipt instead of sleeping a guess.
async fn poll_metric(addr: std::net::SocketAddr, name: &str, want: &str) {
    let line = format!("{name} {want}\n");
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let (_, metrics) = common::http_get_text(addr, "/metrics").await;
        if metrics.contains(&line) {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{name} never reached {want}: {metrics}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Finding 2's wire leg (issue #31, decision 0014 §3): with a real
/// rejecting BEFORE INSERT trigger on the real data file, a capped send's
/// refusal is the truth on every surface — a `451` that claims no storage,
/// nothing queryable, no counter moved, no eviction of the committed
/// incumbent, no webhook — and the retry once the trigger is gone is an
/// ordinary accepted insert. No fault shim anywhere: the refusal is the
/// database's own.
#[tokio::test]
async fn a_refused_insert_is_never_exposed_as_accepted_mail() {
    let _ca = common::CA_CONTRACTS.lock().await;
    let (_dir, db) = data_file("refuse-insert");

    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c.max_per_inbox = 1;
        c
    })
    .await;

    // One committed mail: the cap's incumbent, and the eviction bait.
    let committed = send(&srv, "capped", "committed first").await;
    assert!(!committed.is_empty());

    // A real webhook target: the refusal must fire nothing, the recovery
    // send must fire exactly once — the sink is the receipt.
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let (dest, rx) = capture_sink(stop_rx).await;
    let targets = format!(r#"[{{"url": "http://{dest}", "secret": "s"}}]"#);
    let (st, _) = common::http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // The refusal: a real trigger on the real data file. The reply claims
    // no storage (decision 0012): the fixed refusal line, never a 250.
    let trigger = common::refusing_trigger(&db, "insert");
    let reply = common::smtp_send(
        srv.smtp_addr,
        Some("capped"),
        "sender@x.io",
        "rcpt@x.io",
        "refused at the cap",
        "the body",
    )
    .await
    .expect_err("the refused insert must not answer 250");
    assert!(
        reply.starts_with("451 4.3.0 storage commit failed"),
        "the refused insert must answer the fixed refusal line, got: {reply}"
    );

    // No eviction (AC2): the committed incumbent is untouched.
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{committed}"),
        None,
    )
    .await;
    assert_eq!(
        st, 200,
        "the refused insert must not evict committed mail: {body}"
    );

    // Absence everywhere (AC2). List: exactly the incumbent, nothing else.
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/capped/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 1,
        "the refused mail must not be listed: {list}"
    );
    assert_eq!(list["emails"][0]["id"], committed.as_str());

    // Count: the refusal moves no count.
    let (st, body) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/capped/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        body["count"], 1,
        "the refused mail must not be counted: {body}"
    );

    // Assert: the exact form still sees one mail, so the refusal cannot
    // have over-counted the inbox.
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/capped/assert?count=1",
        None,
    )
    .await;
    assert_eq!(st, 200, "the refused mail must not over-count: {body}");
    assert_eq!(body["matched"], 1);

    // Await: a filtered long-poll for the refused subject times out empty.
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/capped/await?count=1&subject=refused%20at%20the%20cap&timeout_ms=300",
        None,
    )
    .await;
    assert_eq!(st, 408, "the refused mail must match nothing: {body}");
    assert_eq!(body["matched"], 0);

    // A second refusal into a fresh inbox: it must not even mint the inbox.
    let reply = common::smtp_send(
        srv.smtp_addr,
        Some("ghostbox"),
        "sender@x.io",
        "rcpt@x.io",
        "refused fresh",
        "the body",
    )
    .await
    .expect_err("the refused fresh insert must not answer 250");
    assert!(
        reply.starts_with("451 4.3.0 storage commit failed"),
        "got: {reply}"
    );

    // POP3: no maildrop was minted for the refused mail.
    let mut pop = common::Pop3::connect(srv.pop3_addr).await;
    let reply = pop.cmd("USER ghostbox").await;
    assert!(
        reply.starts_with("-ERR"),
        "a refused insert must not mint a maildrop: {reply}"
    );

    // UI: no phantom inbox on the index, and no thread was started.
    let (_, page) = common::http_get_text(srv.http_addr, "/").await;
    assert!(
        !page.contains("ghostbox"),
        "a refused insert must not mint a UI inbox: {page}"
    );
    let (st, threads) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/ghostbox/threads",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        threads.as_array().expect("threads").len(),
        0,
        "the refused mail must start no thread: {threads}"
    );

    // Metrics: no counter moved (AC2) — not the accepted counter, and not
    // the eviction counter the cap's prune would have bumped.
    let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert!(
        metrics.contains("swarmail_emails_inserted_total 1\n"),
        "a refused insert must not count as accepted: {metrics}"
    );
    assert!(
        metrics.contains("swarmail_emails_dropped_total 0\n"),
        "a refused insert must not count an eviction: {metrics}"
    );

    // Recovery: the trigger is gone, the same wire path accepts again, and
    // the recovery send is the sink's only receipt — a refused insert may
    // notify nobody (AC2).
    trigger.remove();
    let recovered = send(&srv, "capped", "recovered").await;
    assert!(
        !recovered.is_empty(),
        "the retry must be accepted with a 250"
    );
    poll_metric(srv.http_addr, "swarmail_webhook_delivered_total", "1").await;
    stop_tx.send(true).unwrap();
    let captured = rx.await.unwrap();
    assert_eq!(
        captured.len(),
        1,
        "exactly the recovery send may notify: {captured:?}"
    );
    assert!(
        captured[0].contains("recovered"),
        "a refused insert must notify nobody: {:?}",
        &captured[..]
    );

    // Restart: the refused mails leave no row to resurrect — the copy holds
    // exactly what the cap holds now, the recovery mail (which, at cap 1,
    // lawfully evicted the incumbent on its accepted commit).
    srv.stop().await;
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c.max_per_inbox = 1;
        c
    })
    .await;
    let (st, list) = common::http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/capped/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let ids: Vec<&str> = list["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        vec![recovered.as_str()],
        "the restart copy must hold only the recovery mail — the refused mails leave no row to resurrect: {list}"
    );
    assert!(
        !list["emails"][0]["subject"]
            .as_str()
            .unwrap_or("")
            .contains("refused"),
        "a refused mail must not resurrect after restart: {list}"
    );
    srv.stop().await;
}

/// Finding 3's wire leg (issue #31, decision 0014 §3): with a real
/// rejecting BEFORE DELETE trigger on the real data file, the delete is
/// reported as the failure it is — never a success claim — and the
/// committed state survives intact on every surface and across a restart.
/// Once the trigger is gone the same delete succeeds and stays gone.
#[tokio::test]
async fn a_failed_delete_answers_failure_and_never_resurrects() {
    let (_dir, db) = data_file("refuse-delete");

    let srv = serve_data_file(&db).await;
    let doomed = send(&srv, "del", "doomed mail").await;
    send(&srv, "del", "kept mail").await;

    let trigger = common::refusing_trigger(&db, "delete");

    // The native delete is the failure it is (AC3): `503` with the fixed
    // body, never a `200` claiming the mail is gone.
    let (st, body) = common::http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{doomed}"),
        None,
    )
    .await;
    assert_eq!(
        st, 503,
        "a failed delete must not claim success: {st} {body}"
    );
    assert_eq!(body["error"], "storage commit failed");

    // The native clear fails the same way, for the same reason.
    let (st, body) = common::http_json(
        srv.http_addr,
        "DELETE",
        "/api/v1/inboxes/del/messages",
        None,
    )
    .await;
    assert_eq!(
        st, 503,
        "a failed clear must not claim success: {st} {body}"
    );
    assert_eq!(body["error"], "storage commit failed");

    // Committed state is preserved: nothing half-applied, both mails still
    // live and queryable.
    let (st, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/del/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 2,
        "a failed delete must preserve committed state: {list}"
    );

    // And no resurrection: the restart copy is the live copy.
    srv.stop().await;
    let srv = serve_data_file(&db).await;
    let (st, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/del/messages", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        list["total"], 2,
        "a failed delete must not lose committed state across a restart: {list}"
    );

    // Recovery: the trigger is gone, the delete succeeds and stays gone.
    trigger.remove();
    let (st, body) = common::http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{doomed}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "the recovered delete must succeed: {body}");
    assert_eq!(body["deleted"], true);
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{doomed}"),
        None,
    )
    .await;
    assert_eq!(st, 404, "the deleted mail must be gone: {st} {body}");
    srv.stop().await;
    let srv = serve_data_file(&db).await;
    let (st, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/del/messages", None).await;
    assert_eq!(st, 200);
    let subjects: Vec<&str> = list["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["subject"].as_str().expect("subject"))
        .collect();
    assert_eq!(
        subjects,
        vec!["kept mail"],
        "the recovered delete must stay deleted across a restart: {list}"
    );
    srv.stop().await;
}

/// The `emails_inbox` index names on `emails`, read straight out of the
/// data file's schema.
fn has_inbox_index(db: &std::path::Path) -> bool {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'emails_inbox'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .unwrap()
        > 0
}

/// A data file written by a release that still created `emails_inbox`
/// (issue #87) restarts with its mail intact over SMTP-delivered, POP3-read
/// and REST-read paths, its counter preserved and its seq index kept —
/// and with the retired index gone (red on the baseline schema, which
/// recreates and maintains it on every write).
#[tokio::test]
async fn a_legacy_data_file_sheds_the_unused_inbox_index_and_keeps_its_mail() {
    let (_dir, db) = data_file("legacy-index");

    let srv = serve_data_file(&db).await;
    let id = send(&srv, "legacy", "Legacy mail").await;
    srv.stop().await;
    // The legacy shape: the retired index present on the file.
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch("CREATE INDEX IF NOT EXISTS emails_inbox ON emails(inbox);")
        .unwrap();
    assert!(has_inbox_index(&db));

    let srv = serve_data_file(&db).await;
    let mut c = common::Pop3::connect(srv.pop3_addr).await;
    c.login("legacy").await;
    assert_eq!(c.cmd("UIDL 1").await, format!("+OK 1 {id}"));
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
    let (st, body) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["subject"], "Legacy mail");
    srv.stop().await;

    assert!(!has_inbox_index(&db), "the retired index survived the open");
    let conn = rusqlite::Connection::open(&db).unwrap();
    let seq_index: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'emails_seq'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(seq_index, 1, "the restore's seq index stays");
    let inserted: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'emails_inserted'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(inserted, "1");
    drop(conn);
}

async fn focused(
    addr: std::net::SocketAddr,
    tool: &str,
    args: serde_json::Value,
) -> serde_json::Value {
    let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":format!("swarmail_wait_for_{tool}"),"arguments":args}});
    let (status, reply) = common::http_json(addr, "POST", "/mcp", Some(&body.to_string())).await;
    assert_eq!(status, 200);
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

/// The schema a data file had before decision 0094: no occurrence column.
const PRE_METADATA_SCHEMA: &str = "CREATE TABLE emails (
    id TEXT PRIMARY KEY, inbox TEXT NOT NULL, seq INTEGER NOT NULL,
    from_json TEXT NOT NULL, to_json TEXT NOT NULL, cc_json TEXT NOT NULL,
    recipients_json TEXT NOT NULL, subject TEXT, received_at TEXT NOT NULL,
    received_ms INTEGER NOT NULL, size INTEGER NOT NULL, text TEXT, html TEXT,
    links_json TEXT NOT NULL, codes_json TEXT NOT NULL, message_id TEXT,
    in_reply_to TEXT, references_json TEXT NOT NULL DEFAULT '[]',
    raw BLOB NOT NULL, envelope_from TEXT);
CREATE INDEX emails_seq ON emails(seq);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO meta VALUES ('emails_inserted', '1'), ('emails_dropped', '0');";

/// Decision 0094 §2 on a genuine pre-column file: the row keeps its stored
/// links and codes exactly — deliberately ones re-extraction would not
/// produce, and more codes than fresh ingest keeps — through two restarts,
/// its occurrences stay unknown (SQL NULL, never backfilled), and fresh
/// mail beside it records its occurrences.
#[tokio::test]
async fn a_pre_metadata_data_file_keeps_unknown_provenance_across_restarts() {
    let (_dir, db) = data_file("pre-metadata");
    let codes: Vec<String> = (0..70).map(|i| format!("{:06}", 100000 + i)).collect();
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(PRE_METADATA_SCHEMA).unwrap();
        conn.execute(
            "INSERT INTO emails (id, inbox, seq, from_json, to_json, cc_json, recipients_json, subject,
                received_at, received_ms, size, text, html, links_json, codes_json, raw)
             VALUES ('legacy-row', 'legacy', 0, '{\"address\":\"old@x.io\"}', '[]', '[]', '[]', 'Old',
                '1970-01-01T00:00:01Z', 1000, 3, 'see https://old.test/a', NULL,
                '[\"https://old.test/a\",\"https://old.test/b\"]', ?1, x'00ff')",
            [serde_json::to_string(&codes).unwrap()],
        )
        .unwrap();
    }
    let mut fresh = String::new();
    for restart in 0..3 {
        let srv = serve_data_file(&db).await;
        if restart == 0 {
            let raw = b"From: app@x.io\r\nTo: r@x.io\r\nSubject: New\r\nContent-Type: text/html\r\n\r\n<a href='https://new.test/go' rel=next>Go</a>\r\n";
            fresh = common::smtp_raw(srv.smtp_addr, "legacy", raw).await;
        }
        let links = focused(
            srv.http_addr,
            "link",
            serde_json::json!({"inbox":"legacy","host":"old","timeout_ms":0}),
        )
        .await;
        assert_eq!(links["status"], "ambiguous");
        let rows: Vec<serde_json::Value> = links["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                serde_json::json!([
                    c["url"],
                    c["link_index"],
                    c["anchor_text"],
                    c["rel"],
                    c["source"],
                    c["sender"]
                ])
            })
            .collect();
        assert_eq!(
            rows,
            [
                serde_json::json!(["https://old.test/a", 0, null, null, null, {"address":"old@x.io"}]),
                serde_json::json!(["https://old.test/b", 1, null, null, null, {"address":"old@x.io"}]),
            ]
        );
        let new = focused(
            srv.http_addr,
            "link",
            serde_json::json!({"inbox":"legacy","anchor_text":"^Go$","timeout_ms":0}),
        )
        .await;
        assert_eq!(
            (
                new["selected_url"].clone(),
                new["candidates"][0]["rel"].clone()
            ),
            (
                serde_json::json!("https://new.test/go"),
                serde_json::json!("next")
            )
        );
        let unknown = focused(
            srv.http_addr,
            "link",
            serde_json::json!({"inbox":"legacy","anchor_text":"","host":"old","timeout_ms":0}),
        )
        .await;
        assert_eq!(
            unknown["status"], "timed_out",
            "an unknown label never matches"
        );
        // The newest mail has no code; the historical array is complete.
        let code = focused(
            srv.http_addr,
            "code",
            serde_json::json!({"inbox":"legacy","timeout_ms":0}),
        )
        .await;
        assert_eq!(
            (code["status"].clone(), code["candidates_total"].clone()),
            (serde_json::json!("ambiguous"), serde_json::json!(70))
        );
        assert_eq!(code["candidates"][0]["email_id"], "legacy-row");
        let (_, row) =
            common::http_json(srv.http_addr, "GET", "/api/v1/messages/legacy-row", None).await;
        assert_eq!(
            row["links"],
            serde_json::json!(["https://old.test/a", "https://old.test/b"])
        );
        assert_eq!(row["codes"], serde_json::json!(codes));
        assert!(row.get("link_metadata").is_none());
        srv.stop().await;
        let conn = rusqlite::Connection::open(&db).unwrap();
        let stored: Vec<(String, Option<String>)> = conn
            .prepare("SELECT id, link_metadata_json FROM emails ORDER BY seq")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(stored, [
            ("legacy-row".into(), None),
            (fresh.clone(), Some(r#"[{"url":"https://new.test/go","anchor_text":"Go","rel":"next","source":"html"}]"#.into())),
        ]);
    }
}

/// Damage to the occurrence column refuses the restore instead of reading
/// as "no links" — including a missing nullable member, a complete
/// positional occurrence and a tagged source of either variant (decision
/// 0097 §1), each refused for its own decode and not for a held file —
/// while SQL NULL, a known empty array and valid populated bytes all serve.
#[tokio::test]
async fn a_corrupt_occurrence_column_refuses_to_serve() {
    let (_dir, db) = data_file("corrupt-metadata");
    let srv = serve_data_file(&db).await;
    let id = send(&srv, "corrupt", "plain").await;
    srv.stop().await;
    let set = |value: Option<&str>| {
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute(
                "UPDATE emails SET link_metadata_json = ?1 WHERE id = ?2",
                rusqlite::params![value, id],
            )
            .unwrap();
    };
    for (corrupt, cause) in [
        ("{", "invalid type: map, expected a sequence"),
        ("null", "invalid type: null, expected a sequence"),
        (
            r#"[{"url":"https://x.io/","rel":null,"source":"html"}]"#,
            "missing field `anchor_text`",
        ),
        (
            r#"[{"url":"https://x.io/","anchor_text":null,"rel":null,"source":"pdf"}]"#,
            "unknown variant `pdf`",
        ),
        (
            r#"[["https://x.io/","Confirm",null,"html"]]"#,
            "invalid type: sequence, expected a link occurrence object",
        ),
        (
            r#"[{"url":"https://x.io/","anchor_text":"Confirm","rel":null,"source":{"html":null}}]"#,
            "invalid type: map, expected a string",
        ),
        (
            r#"[{"url":"https://x.io/","anchor_text":"Confirm","rel":null,"source":{"text":null}}]"#,
            "invalid type: map, expected a string",
        ),
        (
            r#"[{"url":"https://x.io/","url":"https://y.io/","anchor_text":null,"rel":null,"source":"html"}]"#,
            "duplicate field `url`",
        ),
    ] {
        set(Some(corrupt));
        let cfg = swarmail::config::Config {
            smtp_listen: "127.0.0.1:0".into(),
            http_listen: "127.0.0.1:0".into(),
            pop3_listen: "127.0.0.1:0".into(),
            max_per_inbox: 0,
            data_file: Some(db.clone()),
            tls_cert: None,
            tls_key: None,
            smtp: Default::default(),
            pop3_idle_timeout: swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
        };
        let err = match swarmail::run_on(&cfg).await {
            Ok(_) => panic!("a corrupt occurrence column must refuse to serve: {corrupt}"),
            Err(e) => e,
        };
        let err = err.to_string();
        assert!(
            err.contains("Text at index: 19") && err.contains(cause),
            "{corrupt}: {err}"
        );
    }
    set(Some(
        r#"[{"url":"https://x.io/","anchor_text":"Confirm","rel":"","source":"html"},{"url":"https://x.io/","anchor_text":null,"rel":null,"source":"text"}]"#,
    ));
    let srv = serve_data_file(&db).await;
    let link = focused(
        srv.http_addr,
        "link",
        serde_json::json!({"inbox":"corrupt","anchor_text":"^Confirm$","timeout_ms":0}),
    )
    .await;
    assert_eq!(link["status"], "selected");
    assert_eq!(link["selected_url"], "https://x.io/");
    assert_eq!(link["matched"], 1);
    let only = &link["candidates"];
    assert_eq!(only.as_array().unwrap().len(), 1);
    assert_eq!(only[0]["email_id"], id.as_str());
    assert_eq!(only[0]["link_index"], 0);
    assert_eq!(only[0]["rel"], "");
    assert_eq!(only[0]["source"], "html");
    assert_eq!(link["candidates_total"], 1);
    let both = focused(
        srv.http_addr,
        "link",
        serde_json::json!({"inbox":"corrupt","timeout_ms":0}),
    )
    .await;
    assert_eq!(both["candidates_total"], 2);
    assert_eq!(
        both["candidates"][1]["anchor_text"],
        serde_json::Value::Null
    );
    assert_eq!(both["candidates"][1]["source"], "text");
    srv.stop().await;
    for valid in [None, Some("[]")] {
        set(valid);
        let srv = serve_data_file(&db).await;
        let none = focused(
            srv.http_addr,
            "link",
            serde_json::json!({"inbox":"corrupt","timeout_ms":0}),
        )
        .await;
        assert_eq!(none["status"], "timed_out");
        srv.stop().await;
    }
}

/// Newest means accepted last (decision 0094 §5), and a restart restores
/// that order from the persisted sequence: with the stamps and the ids of
/// two rows reversed on disk, the code wait still selects the mail
/// accepted second and lists the first as older.
#[tokio::test]
async fn restored_acceptance_order_decides_the_newest_code() {
    let (_dir, db) = data_file("restored-order");
    let srv = serve_data_file(&db).await;
    let first =
        common::smtp_raw(srv.smtp_addr, "order", b"Subject: a\r\n\r\ncode 111111\r\n").await;
    let second =
        common::smtp_raw(srv.smtp_addr, "order", b"Subject: b\r\n\r\ncode 222222\r\n").await;
    srv.stop().await;
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(&format!(
        "UPDATE emails SET id = 'zzzz-first', received_ms = 9000000000000 WHERE id = '{first}';
         UPDATE emails SET id = '0000-second', received_ms = -9000000000000 WHERE id = '{second}';"
    ))
    .unwrap();
    drop(conn);
    for _ in 0..2 {
        let srv = serve_data_file(&db).await;
        let code = focused(
            srv.http_addr,
            "code",
            serde_json::json!({"inbox":"order","timeout_ms":0}),
        )
        .await;
        assert_eq!(code["selected"]["email_id"], "0000-second");
        assert_eq!(code["selected"]["code"], "222222");
        assert_eq!(code["older_candidates"][0]["email_id"], "zzzz-first");
        assert_eq!(
            code["older_candidates"][0]["age_ms"], 0,
            "a future stamp has no negative age"
        );
        srv.stop().await;
    }
}

/// A refused insert at the cap is invisible to a parked focused wait: the
/// refused mail would have been the only match, yet the wait completes
/// only on the later accepted target, the incumbent was never evicted by
/// the refusal, and the restart copy holds exactly what was accepted.
#[tokio::test]
async fn a_refused_insert_under_the_cap_never_completes_a_focused_wait() {
    let (_dir, db) = data_file("refuse-focused");
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c.max_per_inbox = 1;
        c
    })
    .await;
    let incumbent = common::smtp_raw(
        srv.smtp_addr,
        "capped",
        b"Subject: i\r\n\r\nhttps://app.test/incumbent\r\n",
    )
    .await;
    let http = srv.http_addr;
    let waiting = tokio::spawn(async move {
        focused(
            http,
            "link",
            serde_json::json!({"inbox":"capped","path":"^/(refused|target)$","timeout_ms":60000}),
        )
        .await
    });
    common::wait_for_metric_row(srv.http_addr, "swarmail_awaits_active{inbox=\"capped\"}", 1).await;
    let trigger = common::refusing_trigger(&db, "insert");
    let reply = common::smtp_send(
        srv.smtp_addr,
        Some("capped"),
        "s@x.io",
        "r@x.io",
        "refused",
        "https://app.test/refused",
    )
    .await
    .expect_err("the refused insert must not answer 250");
    assert!(
        reply.starts_with("451 4.3.0 storage commit failed"),
        "{reply}"
    );
    let (status, _) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{incumbent}"),
        None,
    )
    .await;
    assert_eq!(status, 200, "the refusal evicted nothing");
    drop(trigger);
    let target = common::smtp_raw(
        srv.smtp_addr,
        "capped",
        b"Subject: t\r\n\r\nhttps://app.test/target\r\n",
    )
    .await;
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), waiting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["selected_url"], "https://app.test/target");
    assert_eq!(
        (
            result["matched"].clone(),
            result["candidates_total"].clone()
        ),
        (serde_json::json!(1), serde_json::json!(1))
    );
    assert_eq!(result["candidates"][0]["email_id"], target);
    srv.stop().await;
    let srv = serve_data_file(&db).await;
    assert_eq!(
        common::list_all_ids(srv.http_addr, "capped").await,
        [target].into()
    );
    srv.stop().await;
}

// ---- Attachments (decision 0110 §6): the restart copy. ----

fn att_part(headers: &str, body: &str) -> String {
    format!("--bd\r\n{headers}\r\n\r\n{body}\r\n")
}

fn att_binary_part(name: &str, bytes: &[u8]) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let wrapped: Vec<&str> = encoded
        .as_bytes()
        .chunks(76)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();
    att_part(
        &format!(
            "Content-Type: application/octet-stream\r\nContent-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"{name}\""
        ),
        &wrapped.join("\r\n"),
    )
}

fn att_mail(parts: &[String]) -> String {
    format!(
        "From: app@x.io\r\nTo: rcpt@x.io\r\nSubject: att\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n{}--bd--\r\n",
        parts.concat()
    )
}

async fn att_call(
    srv: &RunningServer,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":name,"arguments":arguments}});
    let (status, reply) =
        common::http_json(srv.http_addr, "POST", "/mcp", Some(&body.to_string())).await;
    assert_eq!(status, 200);
    reply["result"].clone()
}

/// The list and the stored bytes come back from the restored row exactly as
/// before the restart — twice over — and a part the budget refused stays
/// refused with the same sentence.
#[tokio::test]
async fn attachments_survive_a_restart_byte_for_byte() {
    let (_dir, db) = data_file("attachments");
    let pdf: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
    let over = vec![7u8; 1_048_577];
    let invite = "BEGIN:VEVENT\r\nUID:persist@x.io\r\nDTSTART;TZID=Europe/Berlin:20261102T150000\r\nSUMMARY:Kept\r\nEND:VEVENT\r\n";
    let raw = att_mail(&[
        att_binary_part("report.pdf", &pdf),
        att_part("Content-Type: text/calendar", invite),
        att_binary_part("over.bin", &over),
    ]);
    let (id, listed, fetched, refused) = {
        let srv = serve_data_file(&db).await;
        let id = common::smtp_raw(srv.smtp_addr, "att", raw.as_bytes()).await;
        let listed = att_call(
            &srv,
            "swarmail_list_attachments",
            serde_json::json!({"id": id}),
        )
        .await;
        let mut fetched = Vec::new();
        for index in 0..2 {
            fetched.push(
                att_call(
                    &srv,
                    "swarmail_get_attachment",
                    serde_json::json!({"id": id, "index": index}),
                )
                .await,
            );
        }
        let refused = att_call(
            &srv,
            "swarmail_get_attachment",
            serde_json::json!({"id": id, "index": 2}),
        )
        .await;
        srv.stop().await;
        (id, listed, fetched, refused)
    };
    assert_eq!(listed["isError"], false);
    assert_eq!(refused["isError"], true);
    for _ in 0..2 {
        let srv = serve_data_file(&db).await;
        let again = att_call(
            &srv,
            "swarmail_list_attachments",
            serde_json::json!({"id": id}),
        )
        .await;
        assert_eq!(again, listed, "the list is the one ingest recorded");
        let parsed: serde_json::Value =
            serde_json::from_str(again["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(parsed["attachments"][1]["ics"]["uid"], "persist@x.io");
        assert_eq!(parsed["attachments"][2]["stored"], false);
        for (index, before) in fetched.iter().enumerate() {
            let after = att_call(
                &srv,
                "swarmail_get_attachment",
                serde_json::json!({"id": id, "index": index}),
            )
            .await;
            assert_eq!(&after, before, "part {index} comes back byte for byte");
        }
        let body: serde_json::Value =
            serde_json::from_str(fetched[0]["content"][0]["text"].as_str().unwrap()).unwrap();
        use base64::Engine;
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(body["content"].as_str().unwrap())
                .unwrap(),
            pdf
        );
        let again = att_call(
            &srv,
            "swarmail_get_attachment",
            serde_json::json!({"id": id, "index": 2}),
        )
        .await;
        assert_eq!(again, refused);
        srv.stop().await;
    }
}

/// A row from before the columns restores with an unknown list — the named
/// refusal, never "no attachments" — and fresh mail beside it lists.
#[tokio::test]
async fn a_pre_attachment_row_answers_unknown_and_is_never_backfilled() {
    let (_dir, db) = data_file("pre-attachments");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(PRE_METADATA_SCHEMA).unwrap();
        conn.execute(
            "INSERT INTO emails (id, inbox, seq, from_json, to_json, cc_json, recipients_json, subject,
                received_at, received_ms, size, text, html, links_json, codes_json, raw)
             VALUES ('legacy-row', 'legacy', 0, 'null', '[]', '[]', '[]', 'Old',
                't', 1000, 3, NULL, NULL, '[]', '[]',
                CAST('From: a@x.io\r\nContent-Type: application/pdf; name=a.pdf\r\nContent-Disposition: attachment\r\n\r\npdf' AS BLOB))",
            [],
        )
        .unwrap();
    }
    for restart in 0..2 {
        let srv = serve_data_file(&db).await;
        for (tool, arguments) in [
            (
                "swarmail_list_attachments",
                serde_json::json!({"id": "legacy-row"}),
            ),
            (
                "swarmail_get_attachment",
                serde_json::json!({"id": "legacy-row", "index": 0}),
            ),
        ] {
            let reply = att_call(&srv, tool, arguments).await;
            assert_eq!(reply["isError"], true);
            assert!(
                reply["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .starts_with("the attachments of message legacy-row are unknown"),
                "{reply}"
            );
        }
        if restart == 0 {
            let id = common::smtp_raw(
                srv.smtp_addr,
                "legacy",
                att_mail(&[att_binary_part("new.bin", b"new")]).as_bytes(),
            )
            .await;
            let fresh = att_call(
                &srv,
                "swarmail_list_attachments",
                serde_json::json!({"id": id}),
            )
            .await;
            assert_eq!(fresh["isError"], false);
        }
        srv.stop().await;
    }
    let conn = rusqlite::Connection::open(&db).unwrap();
    let nulls: i64 = conn
        .query_row(
            "SELECT count(*) FROM emails WHERE id = 'legacy-row' AND attachments_json IS NULL AND attachment_bytes IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(nulls, 1, "the old row is never rewritten");
}

/// Damage to the attachment columns refuses to serve, naming the column;
/// repairing it serves again.
#[tokio::test]
async fn damaged_attachment_columns_refuse_to_serve_by_name() {
    let (_dir, db) = data_file("attachments-damaged");
    let id = {
        let srv = serve_data_file(&db).await;
        let id = common::smtp_raw(
            srv.smtp_addr,
            "dmg",
            att_mail(&[att_binary_part("a.bin", b"abc")]).as_bytes(),
        )
        .await;
        srv.stop().await;
        id
    };
    let good: (String, Vec<u8>) = {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.query_row(
            "SELECT attachments_json, attachment_bytes FROM emails",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    };
    assert_eq!(good.1, b"abc");
    let set = |json: Option<&str>, bytes: Option<&[u8]>| {
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute(
                "UPDATE emails SET attachments_json=?1, attachment_bytes=?2",
                rusqlite::params![json, bytes],
            )
            .unwrap();
    };
    for (json, bytes, named) in [
        (Some("not json"), Some(&b"abc"[..]), "attachments_json"),
        (
            Some(good.0.as_str()),
            Some(&b"abcd"[..]),
            "attachment_bytes",
        ),
        (Some(good.0.as_str()), None, "attachment_bytes"),
        (None, Some(&b"abc"[..]), "attachments_json"),
    ] {
        set(json, bytes);
        let cfg = swarmail::config::Config {
            smtp_listen: "127.0.0.1:0".into(),
            http_listen: "127.0.0.1:0".into(),
            pop3_listen: "127.0.0.1:0".into(),
            max_per_inbox: 0,
            data_file: Some(db.clone()),
            tls_cert: None,
            tls_key: None,
            smtp: Default::default(),
            pop3_idle_timeout: swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
        };
        let error = match swarmail::run_on(&cfg).await {
            Ok(_) => panic!("a damaged {named} must refuse to serve"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("data file") && error.contains(named),
            "{error}"
        );
    }
    set(Some(good.0.as_str()), Some(&good.1));
    let srv = serve_data_file(&db).await;
    let listed = att_call(
        &srv,
        "swarmail_list_attachments",
        serde_json::json!({"id": id}),
    )
    .await;
    assert_eq!(listed["isError"], false);
    srv.stop().await;
}
