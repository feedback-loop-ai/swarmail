//! Process lifecycle: both servers answer while running and stop cleanly
//! when asked — the same graceful path SIGINT takes in production. Every
//! tracked serve/dispatcher task is joined. Detached connection/blocking
//! tails retain the runtime/process teardown limits of decisions 0024/0027.

mod common;

use common::{SmtpConn, start, start_with, wait_for_metric_row};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    let _ca = common::CA_CONTRACTS.lock().await;
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
    let dir = tempfile::TempDir::with_prefix("swarmail-ac3-").unwrap();
    let path = dir.path().join("middata.db");
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
    drop(store);
    drop(dir);
}

/// AC3's POP3 leg: a session mid-transaction dies with its socket, and its
/// uncommitted marks never land. RFC 1939's update state exists only on
/// QUIT; an abort is no QUIT — the closed socket is the only goodbye, and
/// the maildrop is exactly as it was (0010 exact-count discipline).
#[tokio::test]
async fn a_pop3_session_mid_transaction_dies_with_the_socket_and_deletes_nothing() {
    let dir = tempfile::TempDir::with_prefix("swarmail-ac3-pop3-").unwrap();
    let path = dir.path().join("pop3-midtrans.db");
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
    drop(store);
    drop(dir);
}

// ---------- decision 0018: the stop budget and the waits that observe it ----------

/// The server's own stop fence (decisions 0018 §2 and 0027 §7): the SQLite
/// busy bound (5000ms) plus the scheduling margin (5000ms) — the value of
/// `api::DRAIN_DEADLINE`, which ends the HTTP drain on the server's clock.
/// The 5 s busy bound itself stays pinned by persist.rs's pragma test.
const STOP_FENCE: Duration = Duration::from_millis(5000 + 5000);

/// The deadline-observation allowance, and nothing more: the time for this
/// test to see a `stop()` that the fence has already ended return — the
/// joins after the drain and the wakeups that report them. It is not runner
/// slop folded into the fence (issue #143: a 30 s "budget" once hid a fence
/// three times 0027's), and it opens no validity window.
const OBSERVATION_ALLOWANCE: Duration = Duration::from_secs(2);

/// The bound every stop assertion is written against: the fence plus the
/// separately named allowance to observe it, independent of any client's
/// wait timeout or any commit's burn.
const STOP_BUDGET: Duration = STOP_FENCE.saturating_add(OBSERVATION_ALLOWANCE);

/// AC2 (0018): a parked await cannot hold stop hostage. A 60s REST wait
/// parks on an empty inbox; the stop completes inside the budget and the
/// parked request answers `503` with the fixed text — axum's graceful drain
/// would otherwise hold `stop()` for the wait's whole remainder. The park
/// is *observed*, not timed: `swarmail_awaits_active` over real HTTP
/// states the request is parked before the stop fires — a sleep only
/// hoped so (issue #51). Nothing here waits for the wait to answer (it
/// cannot answer before the stop).
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
    // The wait parks, observed under its own inbox: the labeled row can
    // only be raised by this test's wait (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"parked\"}", 1).await;

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
    // The wait parks, observed under its own inbox — the shared gauge's
    // labeled row covers the MCP surface too (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"parkedmcp\"}", 1).await;

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
/// stop ever fired. The park is observed, not timed (issue #51); the
/// asserted outcome is the answer's shape, not a clock.
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
    // The wait parks, observed under its own inbox (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"omitted\"}", 1).await;

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
    // The wait parks, observed under its own inbox (issue #51).
    wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"omittedmcp\"}", 1).await;

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

// R11 (0018), an admitted commit at stop, lives beside the gate that holds
// it: src/store/persist_tests.rs::an_in_flight_commit_neither_holds_stop_nor_half_lands
// (issue #143 — the metric poll and the 5 s busy bound it raced made it
// flake under load).

// ---------- decision 0024: the boundary refuses to start a commit once stopping ----------

/// R1–R3 (0024): three seeds are observed inside the router while a real
/// SQLite write lock prevents successful commits. Their labeled in-flight
/// rows witness live request work, not parsing, worker admission, queue
/// position or a passed stop check. After stop, mutations that have not
/// passed admission refuse without SQLite; an admitted mutation can spend
/// one busy bound. All three answer 503 with the fixed storage-refusal body,
/// stop meets its existing budget, and the refused set stores nothing.
#[tokio::test]
async fn queued_seeds_refuse_once_the_stop_has_fired() {
    let dir = tempfile::TempDir::with_prefix("swarmail-stop-refuse-").unwrap();
    let path = dir.path().join("refuse.db");
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
    // The seeds are inside the router, observed: each seed's own labeled
    // row witnesses live request work, not a queued commit or passed stop
    // admission. The held SQLite lock independently prevents storage.
    // A per-inbox row can only be raised by this test's seeds; a
    // global count could be satisfied by a parallel test's scrape (issue
    // #51).
    for inbox in ["refuse-one", "refuse-two", "refuse-three"] {
        wait_for_metric_row(
            http_addr,
            &format!("swarmail_http_requests_in_flight{{inbox={inbox:?}}}"),
            1,
        )
        .await;
    }

    let started = std::time::Instant::now();
    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("stop must meet its budget with live seed requests under a held SQLite write lock");
    assert!(
        started.elapsed() < STOP_BUDGET,
        "stop took {:?} with three live seed requests observed under a held SQLite write lock",
        started.elapsed()
    );

    for seed in seeds {
        let (st, body) = tokio::time::timeout(Duration::from_secs(10), seed)
            .await
            .expect("every observed seed request must answer once the stop is underway")
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
    drop(dir);
}

// 0027's real stalled-body proof now asserts the selected deadline arm:
// src/lifecycle_tests.rs::stalled_body_selects_drain_deadline, with the
// graceful_http_completion_emits_no_deadline negative control. Its private
// per-serve receipt is unavailable to public-API integration targets.
// The mandatory handler-entered 503 proof remains
// api::tests::a_healthz_inside_its_handler_at_the_stop_is_answered_503_stopping.

// ---------- issue #47: shutdown counting is a ledger row, not a silence ----

/// A fan-out whose mail commits before the stop and whose delivery is still
/// in flight when the drain fires is counted — `aborted`, never `delivered`,
/// never silently lost (issue #47, decision 0029 §5). The commit is *not*
/// pinned: the client sees the lawful `250` (guarantee 1 intact through a
/// stop, unlike 0018's burned commit), the mail is on the data file, and the
/// ledger is exact under both interleavings — the drain either aborts the
/// wedged delivery (the sweep's count) or sees the fan-out post-stop (the
/// matching count) — so no scheduling order can flake the row.
#[tokio::test]
async fn a_fanout_in_flight_at_the_drain_is_counted_and_never_delivered() {
    let _ca = common::CA_CONTRACTS.lock().await;
    let dir = tempfile::TempDir::with_prefix("swarmail-drain-count-").unwrap();
    let path = dir.path().join("counted.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(path.clone());
        c
    })
    .await;
    let (addr, hits, ended) = stalled_sink().await;
    let body = serde_json::json!([
        { "url": format!("http://{addr}/hook"), "inbox": "drain" }
    ])
    .to_string();
    let (st, resp) = common::http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200, "{resp}");

    // The commit resolves before the stop, so the client sees the lawful
    // `250`; the wedged delivery is spawned and holds its slot.
    let tail = common::smtp_send(
        srv.smtp_addr,
        Some("drain"),
        "f@x.io",
        "r@x.io",
        "counted",
        "x",
    )
    .await
    .expect("the commit resolved before the stop");
    assert!(tail.starts_with("250"), "the lawful 250: {tail:?}");
    spin_until(Duration::from_secs(5), || hits.load(Ordering::Relaxed) == 1).await;

    let smtp_addr = srv.smtp_addr;
    let http_addr = srv.http_addr;
    let webhooks = Arc::clone(&srv.webhooks);
    let started = std::time::Instant::now();
    tokio::time::timeout(STOP_BUDGET, srv.stop())
        .await
        .expect("a wedged delivery must not hold stop");

    // The abort is seen from the wire: the held connection closed, and no
    // listener answers after the stop.
    tokio::time::timeout(Duration::from_secs(5), ended)
        .await
        .expect("the wedged delivery's socket must close on stop")
        .unwrap();
    refuses(smtp_addr).await;
    refuses(http_addr).await;
    assert!(
        started.elapsed() < STOP_BUDGET,
        "stop took {:?} with a fan-out in flight",
        started.elapsed()
    );

    // The ledger: counted, never delivered, and not a budget drop.
    let ledger = webhooks.snapshot();
    assert_eq!(ledger.aborted, 1, "{ledger:?}");
    assert_eq!(ledger.delivered, 0, "{ledger:?}");
    assert_eq!(ledger.dropped, 0, "{ledger:?}");

    // And the commit landed: the 250 was honest — the row is on the real
    // data file after the stop.
    let ro =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("the data file opens read-only");
    let stored: i64 = ro
        .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
        .expect("the emails table reads");
    assert_eq!(stored, 1, "the commit the stop allowed is on the data file");
    drop(ro);
    drop(dir);
}

// ---------- issue #143: a silent peer fails every helper read, by name ----

/// A real local peer that accepts every connection, writes `script` once
/// (possibly nothing) and then never says another byte, holding each socket
/// open — the wedged server the helper deadlines exist for.
async fn silent_peer(script: &'static [u8]) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((mut socket, _)) = listener.accept().await {
            socket.write_all(script).await.unwrap();
            held.push(socket);
        }
    });
    addr
}

/// One helper driven against its own silent peer.
type Probe = fn(std::net::SocketAddr) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

/// Run `probe` on its own thread and runtime against a fresh silent peer,
/// capturing its panic message (or the fact that it returned). The result
/// arrives on the channel; a probe whose deadline is gone never sends.
fn probe_silent_peer(script: &'static [u8], probe: Probe) -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(async { probe(silent_peer(script).await).await })
        }));
        let message = match outcome {
            Ok(()) => "the helper returned against a silent peer".to_string(),
            Err(payload) => payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "a non-text panic".to_string()),
        };
        let _ = tx.send(message);
    });
    rx
}

/// The watchdog's allowance over the longest helper deadline: time for a
/// deadline that has fired to be reported. A probe still silent past it has
/// lost its deadline — the hang this test exists to turn red.
const WATCHDOG_ALLOWANCE: Duration = Duration::from_secs(15);

/// AC9 (issue #143): a server that accepts and never answers fails each
/// helper read within its bound, the panic naming the helper and the step —
/// never a hung suite. Every read form is driven against a real silent
/// peer: a line, a multi-line reply, a byte-wise body line, a read-to-EOF,
/// the line-wise EOF reads, the HTTP response, the SSE head and frame, the
/// STARTTLS plaintext phase and the TLS handshake. The probes run in
/// parallel on their own threads; an outer watchdog, measured from their
/// common start, tells an expected deadline panic from a removed deadline
/// (red: the probe never reports, and the watchdog names it).
#[test]
fn a_silent_peer_fails_every_helper_within_its_bound_naming_the_step() {
    const SMTP_TO_STARTTLS_READY: &[u8] =
        b"220 silent\r\n250-silent\r\n250-STARTTLS\r\n250 OK\r\n220 Ready\r\n";
    let cases: Vec<(&str, &'static [u8], Probe)> = vec![
        ("smtp_send: the greeting: no reply within", b"", |addr| {
            Box::pin(async move {
                let _ = common::smtp_send(addr, None, "f@x.io", "r@x.io", "s", "b").await;
            })
        }),
        (
            "SmtpConn::connect: the greeting: no reply within",
            b"",
            |addr| {
                Box::pin(async move {
                    SmtpConn::connect(addr).await;
                })
            },
        ),
        (
            "SmtpConn::reply: a reply line: no reply within",
            b"220 hi\r\n",
            |addr| {
                Box::pin(async move {
                    let mut c = SmtpConn::connect(addr).await;
                    c.send("NOOP").await;
                    c.reply().await;
                })
            },
        ),
        (
            "SmtpConn::reply_multi: a reply line: no reply within",
            b"",
            |addr| {
                Box::pin(async move {
                    SmtpConn::connect_raw(addr).await.reply_multi().await;
                })
            },
        ),
        (
            "SmtpConn::read_to_end: the session's EOF: no EOF within",
            b"",
            |addr| {
                Box::pin(async move {
                    SmtpConn::connect_raw(addr).await.read_to_end().await;
                })
            },
        ),
        (
            "SmtpConn::read_to_eof: the session's EOF: no line or EOF within",
            b"",
            |addr| {
                Box::pin(async move {
                    SmtpConn::connect_raw(addr).await.read_to_eof().await;
                })
            },
        ),
        (
            "Pop3::connect: the greeting: no reply within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::Pop3::connect(addr).await;
                })
            },
        ),
        (
            "Pop3::reply: the reply line: no reply within",
            b"+OK hi\r\n",
            |addr| {
                Box::pin(async move {
                    common::Pop3::connect(addr).await.cmd("STAT").await;
                })
            },
        ),
        (
            "Pop3::data_bytes: a multi-line body line: no reply within",
            b"+OK hi\r\n",
            |addr| {
                Box::pin(async move {
                    common::Pop3::connect(addr).await.data_bytes().await;
                })
            },
        ),
        (
            "Pop3::raw_bytes: a multi-line body line: no reply within",
            b"+OK hi\r\n",
            |addr| {
                Box::pin(async move {
                    common::Pop3::connect(addr).await.raw_bytes().await;
                })
            },
        ),
        (
            "Pop3::read_to_eof: the session's EOF: no line or EOF within",
            b"+OK hi\r\n",
            |addr| {
                Box::pin(async move {
                    common::Pop3::connect(addr).await.read_to_eof().await;
                })
            },
        ),
        (
            "http_get_text: the HTTP response: no EOF within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::http_get_text(addr, "/healthz").await;
                })
            },
        ),
        (
            "http_get_parts: the HTTP response: no EOF within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::http_bytes(addr, "/healthz").await;
                })
            },
        ),
        (
            "http_send_text_with_headers: the HTTP response: no EOF within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::http_json(addr, "GET", "/healthz", None).await;
                })
            },
        ),
        (
            "send_with_host: the HTTP response: no EOF within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::send_with_host(addr, "GET", "/healthz", "localhost", None).await;
                })
            },
        ),
        (
            "http_send_text_allow_eof: the HTTP response: ",
            b"",
            |addr| {
                Box::pin(async move {
                    common::http_send_text_allow_eof(addr, "GET", "/healthz", None).await;
                })
            },
        ),
        (
            "SseReader::open: a response head line: no reply within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::SseReader::open(addr, "/feed").await;
                })
            },
        ),
        (
            "SseReader::next_frame: no feed frame line within 5s",
            b"HTTP/1.1 200 OK\r\n\r\n",
            |addr| {
                Box::pin(async move {
                    common::SseReader::open(addr, "/feed")
                        .await
                        .next_frame()
                        .await;
                })
            },
        ),
        (
            "SseReader::read_until_eof: the feed's EOF: no line or EOF within",
            b"HTTP/1.1 200 OK\r\n\r\n",
            |addr| {
                Box::pin(async move {
                    common::SseReader::open(addr, "/feed")
                        .await
                        .read_until_eof()
                        .await;
                })
            },
        ),
        (
            "plaintext_starttls_phase: the greeting: no reply within",
            b"",
            |addr| {
                Box::pin(async move {
                    common::stalled_starttls(addr).await;
                })
            },
        ),
        (
            "starttls_upgrading: no TLS handshake reply within the helper deadline",
            SMTP_TO_STARTTLS_READY,
            |addr| {
                Box::pin(async move {
                    let root = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
                        .unwrap()
                        .cert
                        .der()
                        .clone();
                    common::starttls(addr, "localhost", root).await;
                })
            },
        ),
    ];

    let started = std::time::Instant::now();
    let running: Vec<_> = cases
        .into_iter()
        .map(|(want, script, probe)| (want, probe_silent_peer(script, probe)))
        .collect();
    let watchdog = started + common::HELPER_DEADLINE + WATCHDOG_ALLOWANCE;
    for (want, outcome) in running {
        let left = watchdog.saturating_duration_since(std::time::Instant::now());
        let message = outcome.recv_timeout(left).unwrap_or_else(|_| {
            panic!("watchdog: the probe for {want:?} never failed — its read deadline is gone")
        });
        assert!(
            message.contains(want),
            "the silent peer must fail the helper naming {want:?}; it said: {message:?}"
        );
    }
}
