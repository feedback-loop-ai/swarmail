//! Process lifecycle: both servers answer while running and stop cleanly
//! when asked — the same graceful path SIGINT takes in production. Every
//! task the process owns dies with it: an open feed, a delivery held by a
//! stalled sink, a session mid-DATA.

mod common;

use common::{SmtpConn, start, start_with};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::oneshot;

#[tokio::test]
async fn servers_serve_then_stop_and_release_their_ports() {
    let srv = start().await;
    let smtp_addr = srv.smtp_addr;
    let http_addr = srv.http_addr;

    // Both listeners answer while running.
    let (st, _) = common::http_json(http_addr, "GET", "/healthz", None).await;
    assert_eq!(st, 200);
    let mut c = SmtpConn::connect(smtp_addr).await;
    c.send("NOOP").await;
    assert!(c.reply().await.starts_with("250"));

    // The graceful stop: both serve futures return, exit is logged, and the
    // ports actually close.
    srv.stop().await;

    // HTTP is gone.
    let refused_http = tokio::net::TcpStream::connect(http_addr).await.is_err();
    assert!(refused_http, "http listener should be closed after stop");
    // SMTP is gone (retry a beat: the OS may need a moment to release).
    let mut smtp_closed = false;
    for _ in 0..20 {
        if tokio::net::TcpStream::connect(smtp_addr).await.is_err() {
            smtp_closed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(smtp_closed, "smtp listener should be closed after stop");
}

/// A sink that accepts and never answers: the delivery it holds is stuck
/// until the server kills it. `ended` fires when the held connection
/// closes — the EOF an aborted delivery leaves behind.
async fn stalled_sink() -> (
    std::net::SocketAddr,
    std::sync::Arc<AtomicUsize>,
    oneshot::Receiver<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let (tx, ended) = oneshot::channel();
    let counter = Arc::clone(&hits);
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        counter.fetch_add(1, Ordering::Relaxed);
        // Drain until the peer hangs up: the EOF is the abort.
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
        let _ = tx.send(());
    });
    (addr, hits, ended)
}

/// Spin-yield until a production observable holds, under a deadline.
async fn spin_until(deadline: Duration, fact: impl Fn() -> bool) {
    tokio::time::timeout(deadline, async {
        while !fact() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the production observable never became true");
}

/// Poll-connect until the port refuses, under a deadline: the serve task
/// has returned and the OS has released the listener. A retry loop against
/// a real observable — a port that still accepts fails the test, it never
/// waits it out.
async fn refuses(addr: std::net::SocketAddr) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if tokio::net::TcpStream::connect(addr).await.is_err() {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{addr} still accepts connections after stop"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// AC1: a server with an open feed stops on demand. The feed's trailing
/// frame may race out, then the response ends and the socket closes —
/// `stop()` must return either way, and the feed must see the end.
#[tokio::test]
async fn an_open_feed_does_not_hold_stop_hostage() {
    let srv = start().await;
    let smtp_addr = srv.smtp_addr;
    let http_addr = srv.http_addr;
    let mut feed = common::SseReader::open(srv.http_addr, "/api/v1/inboxes/feedbox/feed").await;
    let (event, _) = feed.next_frame().await;
    assert_eq!(
        event, "threads",
        "the snapshot frame must be out: the feed is open"
    );

    srv.stop().await;

    // The read returns only when the response body terminates — the
    // terminal chunk of the chunked stream, or nothing at all.
    let tail = feed.read_until_eof().await;
    assert!(
        tail.ends_with("0\r\n\r\n") || tail.is_empty(),
        "the feed must end when the server stops: {tail:?}"
    );
    // And the listeners are gone with it: nothing accepts after stop.
    refuses(smtp_addr).await;
    refuses(http_addr).await;
}

/// AC2: a delivery held by a stalled sink cannot hold the stop. The sink
/// accepts the POST and never answers; `stop()` must return well inside the
/// delivery's own 22 s budget and the held connection must close (its EOF
/// is the abort, seen from the wire). That the abort is counted — the
/// ledger's `aborted` row — is proven deterministically by the inline drain
/// driver in `src/webhook.rs` (`loop_tests`), which can still read the
/// ledger after the count; here the HTTP listener is already gone.
#[tokio::test]
async fn a_stalled_sink_cannot_hold_stop_hostage() {
    let srv = start().await;
    let (addr, hits, ended) = stalled_sink().await;
    let body = serde_json::json!([{ "url": format!("http://{addr}/hook") }]).to_string();
    let (st, _) = common::http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200);

    common::smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "stalled", "x")
        .await
        .unwrap();
    spin_until(Duration::from_secs(5), || hits.load(Ordering::Relaxed) == 1).await;

    let smtp_addr = srv.smtp_addr;
    let http_addr = srv.http_addr;
    let started = std::time::Instant::now();
    tokio::time::timeout(Duration::from_secs(10), srv.stop())
        .await
        .expect("stop must return inside the delivery budget");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "stop took {:?} with a delivery held by a stalled sink",
        started.elapsed()
    );
    // The held connection closed: that is the abort, seen from the wire.
    tokio::time::timeout(Duration::from_secs(5), ended)
        .await
        .expect("the stalled delivery's socket must close on stop")
        .unwrap();
    // And the listeners are gone with it: nothing accepts after stop.
    refuses(smtp_addr).await;
    refuses(http_addr).await;
}

/// AC3: a session mid-DATA dies with its socket — the abort is the only
/// goodbye, no half-mail is ever committed. The store is reopened after the
/// stop to prove the transaction never landed.
#[tokio::test]
async fn a_mid_transaction_session_dies_with_the_socket_and_stores_nothing() {
    let dir = std::env::temp_dir().join(format!("swarmail-ac3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("middata.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(path.clone());
        c
    })
    .await;

    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    let smtp_addr = srv.smtp_addr;
    let pop3_addr = srv.pop3_addr;
    c.send("MAIL FROM:<f@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<doomed@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("354"));
    c.send("Subject: never committed").await; // mid-DATA: no terminating dot

    tokio::time::timeout(Duration::from_secs(10), srv.stop())
        .await
        .expect("stop must return with a session mid-DATA");

    let tail = tokio::time::timeout(Duration::from_secs(10), c.read_to_eof())
        .await
        .expect("the session socket must close on stop");
    assert!(
        !tail.contains("250"),
        "an aborted session must never see a commit: {tail:?}"
    );
    // Both session listeners are gone: nothing accepts after stop.
    refuses(smtp_addr).await;
    refuses(pop3_addr).await;

    let store = swarmail::store::Store::open(100, &path).unwrap();
    assert!(
        store.inboxes().is_empty(),
        "a mid-transaction session must store nothing: {:?}",
        store.inboxes()
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// AC3's POP3 leg: a session mid-transaction dies with its socket, and its
/// uncommitted marks never land. RFC 1939's update state exists only on
/// QUIT; an abort is no QUIT — the closed socket is the only goodbye, and
/// the maildrop is exactly as it was (0010 exact-count discipline).
#[tokio::test]
async fn a_pop3_session_mid_transaction_dies_with_the_socket_and_deletes_nothing() {
    let dir = std::env::temp_dir().join(format!("swarmail-ac3-pop3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pop3-midtrans.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(path.clone());
        c
    })
    .await;

    // One mail in the maildrop, delivered over real SMTP (decision 0001).
    common::smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "kept", "body")
        .await
        .unwrap();

    let mut p = common::Pop3::connect(srv.pop3_addr).await;
    p.login("default").await;
    // LIST is multi-line: head line, then the drop, then the dot.
    p.send("LIST").await;
    let list_head = p.reply().await;
    assert!(
        list_head.starts_with("+OK"),
        "the maildrop answers LIST: {list_head}"
    );
    let list = p.data().await;
    assert!(
        list.lines().any(|l| l.trim_start().starts_with("1 ")),
        "the maildrop has the mail: {list:?}"
    );
    assert!(
        p.cmd("DELE 1").await.starts_with("+OK"),
        "the mail is marked for deletion, uncommitted"
    );
    // Mid-transaction: no QUIT is ever sent.

    let smtp_addr = srv.smtp_addr;
    let pop3_addr = srv.pop3_addr;
    tokio::time::timeout(Duration::from_secs(10), srv.stop())
        .await
        .expect("stop must return with a POP3 session mid-transaction");

    // The socket closes with no goodbye.
    let tail = tokio::time::timeout(Duration::from_secs(10), p.read_to_eof())
        .await
        .expect("the POP3 session socket must close on stop");
    assert!(
        !tail.contains("+OK"),
        "an aborted session gets no goodbye: {tail:?}"
    );
    refuses(smtp_addr).await;
    refuses(pop3_addr).await;

    // The mark never landed: the maildrop is exactly as it was.
    let store = swarmail::store::Store::open(100, &path).unwrap();
    assert_eq!(
        store.count("default", &swarmail::store::Filter::default()),
        1,
        "an uncommitted DELE dies with its session"
    );
    let _ = std::fs::remove_dir_all(dir);
}

// ---------- decision 0018: the stop budget and the waits that observe it ----------

/// The stop budget (decision 0018): the SQLite busy bound (5000ms) plus a
/// scheduling margin (5000ms) — the number every stop assertion is written
/// against, independent of any client's wait timeout or any commit's burn.
const STOP_BUDGET: Duration = Duration::from_secs(10);

/// AC2 (0018): a parked await cannot hold stop hostage. A 60s REST wait
/// parks on an empty inbox; the stop completes inside the budget and the
/// parked request answers `503` with the fixed text — axum's graceful drain
/// would otherwise hold `stop()` for the wait's whole remainder. The 300ms
/// pause is sequencing, not an outcome wait: nothing here waits for the
/// wait to answer (it cannot answer before the stop), the request only has
/// to be parked when the stop fires.
#[tokio::test]
async fn a_parked_await_does_not_hold_stop_hostage() {
    let srv = start().await;
    let addr = srv.http_addr;
    let request = tokio::spawn(common::http_json(
        addr,
        "GET",
        "/api/v1/inboxes/parked/await?count=1&timeout_ms=60000",
        None,
    ));
    tokio::time::sleep(Duration::from_millis(300)).await; // the wait parks

    let started = std::time::Instant::now();
    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("a parked await must not hold stop hostage");
    assert!(
        started.elapsed() < STOP_BUDGET,
        "stop took {:?} with a wait parked on a 60s timeout",
        started.elapsed()
    );
    let (st, body) = tokio::time::timeout(Duration::from_secs(10), request)
        .await
        .expect("the parked wait must answer once the stop is underway")
        .unwrap();
    assert_eq!(st, 503);
    assert_eq!(
        body,
        serde_json::json!({ "error": "shutting down" }),
        "{body}"
    );
}

/// AC2's MCP twin: a parked `swarmail_wait_for_email` observes the same stop
/// and answers an `isError` result with the fixed text.
#[tokio::test]
async fn a_parked_mcp_wait_does_not_hold_stop_hostage() {
    let srv = start().await;
    let addr = srv.http_addr;
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"parkedmcp","count":1,"timeout_ms":60000}}}"#;
    let request = tokio::spawn(common::http_json(addr, "POST", "/mcp", Some(body)));
    tokio::time::sleep(Duration::from_millis(300)).await; // the wait parks

    let started = std::time::Instant::now();
    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("a parked MCP wait must not hold stop hostage");
    assert!(
        started.elapsed() < STOP_BUDGET,
        "stop took {:?} with an MCP wait parked on a 60s timeout",
        started.elapsed()
    );
    let (st, body) = tokio::time::timeout(Duration::from_secs(10), request)
        .await
        .expect("the parked MCP wait must answer once the stop is underway")
        .unwrap();
    assert_eq!(st, 200);
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(body["result"]["content"][0]["text"], "shutting down");
}

/// Omission keeps the documented default on both transports (issue #32
/// slice 2, decision 0025): an await with no `timeout_ms` parks on the
/// 5000 the docs publish and answers the stop's fixed 503 — a refused
/// parser would have answered 400 and a zeroed default 408 before the
/// stop ever fired. The 300ms pause is the established sequencing
/// discipline; the asserted outcome is the answer's shape, not a clock.
#[tokio::test]
async fn an_omitted_timeout_parks_on_the_documented_default() {
    let srv = start().await;
    let addr = srv.http_addr;
    let request = tokio::spawn(common::http_json(
        addr,
        "GET",
        "/api/v1/inboxes/omitted/await?count=1&to=nobody@x.io",
        None,
    ));
    tokio::time::sleep(Duration::from_millis(300)).await; // the wait parks

    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("stop completes with an await parked on the default");
    let (st, body) = tokio::time::timeout(Duration::from_secs(10), request)
        .await
        .expect("the parked wait answers once the stop is underway")
        .unwrap();
    assert_eq!(st, 503, "{body}");
    assert_eq!(
        body,
        serde_json::json!({ "error": "shutting down" }),
        "{body}"
    );
}

/// The MCP twin: `swarmail_wait_for_email` with no `timeout_ms` parks on
/// the documented default and answers the stop's fixed `isError` text —
/// the same omitted value, the same default, one domain (decision 0025).
#[tokio::test]
async fn an_omitted_mcp_timeout_parks_on_the_documented_default() {
    let srv = start().await;
    let addr = srv.http_addr;
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"omittedmcp","count":1,"to":"nobody@x.io"}}}"#;
    let request = tokio::spawn(common::http_json(addr, "POST", "/mcp", Some(body)));
    tokio::time::sleep(Duration::from_millis(300)).await; // the wait parks

    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("stop completes with an MCP wait parked on the default");
    let (st, body) = tokio::time::timeout(Duration::from_secs(10), request)
        .await
        .expect("the parked MCP wait answers once the stop is underway")
        .unwrap();
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(body["result"]["content"][0]["text"], "shutting down");
}

/// R11 (0018): a commit already in flight at stop is not cancelled — the
/// blocking pool runs it to completion — but it does not hold stop. Under a
/// held write lock the delivery's commit burns its whole busy bound; the
/// stop lands inside the budget, the client never saw `250`, and a fresh
/// read of the data file shows the refused commit stored nothing.
#[tokio::test]
async fn an_in_flight_commit_neither_holds_stop_nor_half_lands() {
    let dir = std::env::temp_dir().join(format!("swarmail-stop-burn-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("burn.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(path.clone());
        c
    })
    .await;
    // A real second connection holds the writer lock: the delivery's commit
    // will burn its full busy bound and refuse (decision 0012 §3, at the
    // real layer).
    let _lock = common::hold_write_lock(&path);

    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("MAIL FROM:<f@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<burn@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    // The reply to the dot is where the commit burns: drive it on a task and
    // stop under it. The bound is 5s, so a 100ms sequencing pause puts the
    // stop inside the burn with certainty — no outcome is awaited here.
    let reply = tokio::spawn(async move { c.data("a mail that cannot commit").await });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let smtp_addr = srv.smtp_addr;
    let http_addr = srv.http_addr;
    let started = std::time::Instant::now();
    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("an in-flight commit must not hold stop");
    assert!(
        started.elapsed() < STOP_BUDGET,
        "stop took {:?} while a commit burned its busy bound",
        started.elapsed()
    );

    // The socket died with the session: whatever the client reads, it is
    // never a `250` — a commit that outlives its socket cannot half-land.
    let tail = tokio::time::timeout(Duration::from_secs(10), reply)
        .await
        .expect("the delivery task must see the socket close")
        .unwrap();
    assert!(
        !tail.starts_with("250"),
        "a refused commit must never answer 250: {tail:?}"
    );
    refuses(smtp_addr).await;
    refuses(http_addr).await;

    // A read-only look at the real data file while the lock is still held:
    // the burned commit stored nothing (the write lock makes this exact,
    // not racy — the insert cannot land while it is held).
    let ro =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("the data file opens read-only");
    let stored: i64 = ro
        .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
        .expect("the emails table reads");
    assert_eq!(stored, 0, "a commit in flight at stop stored nothing");
    drop(ro);

    // Release the writer lock: the commit the stop did not wait for runs to
    // completion on the blocking pool and lands — losslessly, one copy,
    // though the client never saw `250` (R11: "retry is lossless"). If the
    // commit had been cancelled instead, nothing would ever land.
    drop(_lock);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let ro = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("the data file opens read-only");
        let stored: i64 = ro
            .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
            .expect("the emails table reads");
        drop(ro);
        if stored == 1 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the in-flight commit never completed after the stop: {stored} rows"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let _ = std::fs::remove_dir_all(dir);
}

// ---------- decision 0024: the boundary refuses to start a commit once stopping ----------

/// R1–R3 (0024): under a held write lock, three concurrent seeds queue on
/// the mutation boundary; the stop flips the store's flag and every queued
/// mutation acquires the boundary, reads the watch and refuses without one
/// SQLite call — so each burns a scheduling instant, not its own 5s busy
/// bound. All three answer `503` with 0012 §2's fixed body (the wire does
/// not fork: a stopping refusal and a busy refusal share the text, the
/// cause is the log), `stop()` lands inside `STOP_BUDGET` — upper bound
/// only, sequence-independent: the seed that wins the boundary first is
/// past the check and burns its own bound, and the budget arithmetic
/// (at most one commit in flight + ε per queued release) covers both. A
/// fresh read of the data file shows the refused set stored nothing.
/// The 300ms pause is sequencing, not an outcome wait: it only puts the
/// three requests on the boundary before the stop fires.
#[tokio::test]
async fn queued_seeds_refuse_once_the_stop_has_fired() {
    let dir = std::env::temp_dir().join(format!("swarmail-stop-refuse-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("refuse.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(path.clone());
        c
    })
    .await;
    // A real second connection holds the writer lock: every seed's commit
    // is forced to queue or burn (decision 0012 §3, at the real layer).
    let _lock = common::hold_write_lock(&path);

    let http_addr = srv.http_addr;
    let mut seeds = Vec::new();
    for inbox in ["refuse-one", "refuse-two", "refuse-three"] {
        let body = r#"{"to": "s@x.io", "text": "a mail that must not land"}"#;
        let route = format!("/api/v1/inboxes/{inbox}/seed");
        seeds.push(tokio::spawn(async move {
            common::http_json(http_addr, "POST", &route, Some(body)).await
        }));
    }
    tokio::time::sleep(Duration::from_millis(300)).await; // the seeds queue on the boundary

    let started = std::time::Instant::now();
    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("queued mutations must not hold stop past one busy bound");
    assert!(
        started.elapsed() < STOP_BUDGET,
        "stop took {:?} with three seeds queued on the boundary",
        started.elapsed()
    );

    for seed in seeds {
        let (st, body) = tokio::time::timeout(Duration::from_secs(10), seed)
            .await
            .expect("every queued seed must answer once the stop is underway")
            .unwrap();
        assert_eq!(st, 503, "{body}");
        assert_eq!(
            body,
            serde_json::json!({ "error": "storage commit failed" }),
            "{body}"
        );
    }

    // A read-only look at the real data file while the lock is still held:
    // the refused set stored nothing — not one row, not one inbox (the
    // held write lock makes this exact, not racy: nothing can land while
    // it is held).
    let ro =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("the data file opens read-only");
    let stored: i64 = ro
        .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
        .expect("the emails table reads");
    assert_eq!(stored, 0, "the refused set stored nothing");
    drop(ro);
    let _ = std::fs::remove_dir_all(dir);
}
