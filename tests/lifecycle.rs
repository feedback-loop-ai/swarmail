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
