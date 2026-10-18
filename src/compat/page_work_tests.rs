//! Compat page builds run off the async worker (issue #96, decision 0044
//! §5; the placement is 0020 rule 2).
//!
//! The proof parks real Mailpit and MailHog list and search builds inside
//! the store's compat gate, on a real server driven by this test's
//! current-thread runtime, and asks that same runtime to greet an SMTP peer
//! and answer `/healthz` meanwhile. Its cold path is the gate's own
//! deadline: with a build inline on the async worker, the held gate blocks
//! the sole worker, and only that deadline — measured on the blocked thread,
//! never by the test runtime — releases it so the test ends red instead of
//! hanging. The line runs only while the bug is present, so the test cannot
//! live inline.

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// One plain HTTP/1.1 GET: (status, body bytes).
async fn get(addr: std::net::SocketAddr, path: String) -> (u16, Vec<u8>) {
    let (status, _, body) = fetch(addr, path).await;
    (status, body)
}

/// One plain HTTP/1.1 GET: (status, lowercased head, body bytes).
async fn fetch(addr: std::net::SocketAddr, path: String) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).await.unwrap();
    let split = reply.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&reply[..split]).to_lowercase();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, head, reply[split + 4..].to_vec())
}

/// A loopback server, optionally persisting to `data_file`.
fn config(data_file: Option<std::path::PathBuf>) -> crate::config::Config {
    crate::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file,
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
        pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
    }
}

/// A gate that counts entries and holds each until opened, recording
/// whether any hold had to be ended by its own deadline instead.
struct Gate {
    entered: Arc<AtomicUsize>,
    open: Arc<(Mutex<bool>, Condvar)>,
    expired: Arc<AtomicBool>,
}

impl Gate {
    fn install(store: &Store) -> Gate {
        let gate = Gate {
            entered: Arc::new(AtomicUsize::new(0)),
            open: Arc::new((Mutex::new(false), Condvar::new())),
            expired: Arc::new(AtomicBool::new(false)),
        };
        let (entered, open, expired) = (
            gate.entered.clone(),
            gate.open.clone(),
            gate.expired.clone(),
        );
        store.set_compat_gate(Some(Arc::new(move || {
            entered.fetch_add(1, Ordering::SeqCst);
            let (open, wake) = &*open;
            let (_open, wait) = wake
                .wait_timeout_while(open.lock().unwrap(), Duration::from_secs(10), |open| !*open)
                .unwrap();
            expired.fetch_or(wait.timed_out(), Ordering::SeqCst);
        })));
        gate
    }

    fn release(&self) {
        let (open, wake) = &*self.open;
        *open.lock().unwrap() = true;
        wake.notify_all();
    }
}

/// Every single-message response — Mailpit's full, plain, raw and header
/// shapes, their latest aliases, MailHog's plural plain and download, and
/// the native JSON and raw — is built on the blocking pool (decision 0049
/// §7, issue #139). While all eleven builds are held *between the lookup
/// and the build*, a new SMTP peer is greeted, `/healthz` answers and a
/// missing id still 404s, with no gate expired; release finishes every
/// response with its full body and content type. Red at 2a325c9: the
/// renders ran inline on the sole worker, so nothing was ever held and
/// the eleven never entered the gate together.
#[tokio::test]
async fn held_single_message_builds_leave_smtp_and_health_serving() {
    let srv = crate::run_on(&config(None)).await.unwrap();
    let raw = b"Subject: held\r\nContent-Type: text/html\r\n\r\n<p>Hello <b>held</b> world</p>\r\n";
    let mut held = crate::store::email("h1", "held", "a@x.io");
    held.html = Some("<p>Hello <b>held</b> world</p>".into());
    held.raw = raw.to_vec();
    srv.store.insert(held).unwrap();
    let gate = Gate::install(&srv.store);

    let addr = srv.http_addr;
    let paths = [
        "/api/v1/message/h1",
        "/api/v1/message/h1/plain",
        "/api/v1/messages/h1/plain",
        "/api/v1/message/h1/raw",
        "/api/v1/message/h1/headers",
        "/api/v1/messages/h1/download",
        "/api/v1/message/latest?inbox=held",
        "/api/v1/message/latest/raw?inbox=held",
        "/api/v1/message/latest/headers?inbox=held",
        "/api/v1/messages/h1",
        "/api/v1/messages/h1/raw",
    ];
    let requests: Vec<_> = paths
        .iter()
        .map(|path| tokio::spawn(fetch(addr, path.to_string())))
        .collect();
    let deadline = Instant::now() + Duration::from_secs(30);
    while gate.entered.load(Ordering::SeqCst) < paths.len() {
        assert!(
            Instant::now() < deadline,
            "only {} of {} single-message builds were ever held together",
            gate.entered.load(Ordering::SeqCst),
            paths.len()
        );
        tokio::task::yield_now().await;
    }

    let mut smtp = BufReader::new(TcpStream::connect(srv.smtp_addr).await.unwrap());
    let mut greeting = String::new();
    smtp.read_line(&mut greeting).await.unwrap();
    assert!(greeting.starts_with("220"), "{greeting}");
    let (status, health) = get(addr, "/healthz".into()).await;
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&health));
    // The lookup precedes the gate: an absent mail never waits on it.
    for missing in [
        "/api/v1/message/nope",
        "/api/v1/messages/nope",
        "/api/v1/message/latest?inbox=empty",
    ] {
        assert_eq!(get(addr, missing.into()).await.0, 404, "{missing}");
    }
    assert!(
        !gate.expired.load(Ordering::SeqCst),
        "SMTP and health were served only after a held build's gate expired"
    );

    gate.release();
    for (path, request) in paths.iter().zip(requests) {
        let (status, head, body) = request.await.unwrap();
        assert_eq!(status, 200, "{path}: {}", String::from_utf8_lossy(&body));
        if path.ends_with("/raw") || path.contains("raw?") || path.ends_with("download") {
            assert!(
                head.contains("content-type: message/rfc822"),
                "{path}: {head}"
            );
            assert_eq!(body, raw, "{path}");
        } else if path.ends_with("plain") {
            assert!(head.contains("content-type: text/plain"), "{path}: {head}");
            let text = String::from_utf8(body).unwrap();
            assert!(
                text.contains("held") && !text.contains("<b>"),
                "{path}: {text}"
            );
        } else {
            assert!(
                head.contains("content-type: application/json"),
                "{path}: {head}"
            );
            let body: Value = serde_json::from_slice(&body).unwrap();
            let id = body.get("ID").or(body.get("id"));
            assert!(
                id == Some(&json!("h1")) || body.get("Date").is_some(),
                "{path}: {body}"
            );
        }
    }
    assert!(!gate.expired.load(Ordering::SeqCst));
    srv.store.set_compat_gate(None);
    srv.stop().await;
}

/// The IDs a compat page lists, in order.
async fn page_ids(addr: std::net::SocketAddr, path: &str) -> Vec<String> {
    let (status, body) = get(addr, path.to_string()).await;
    assert_eq!(status, 200, "{path}: {}", String::from_utf8_lossy(&body));
    let body: Value = serde_json::from_slice(&body).unwrap();
    body.get("messages")
        .or(body.get("items"))
        .or(body.get("emails"))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            item.get("ID")
                .or(item.get("id"))
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

/// Every compat list and search — scoped or global, either dialect, any
/// page — and `latest` share one order: descending `(received_ms, id)`
/// (decision 0049 §5, issue #138). The fixture's acceptance order runs
/// against its stamps and two mails tie on the stamp, and the order
/// survives a restart; the native inbox list keeps acceptance order. Red
/// at 2a325c9: `messages?inbox=ord` listed acceptance order while
/// `latest?inbox=ord` answered the newest stamp.
#[tokio::test]
async fn compat_views_share_timestamp_order() {
    let dir = tempfile::tempdir().unwrap();
    let data_file = dir.path().join("order.db");
    let srv = crate::run_on(&config(Some(data_file.clone())))
        .await
        .unwrap();
    // Accepted in this order; stamped out of it.
    for (id, inbox, ms) in [
        ("m-b", "ord", 300),
        ("m-a", "ord", 100),
        ("m-c", "ord", 300),
        ("m-d", "ord", 200),
        ("o-1", "other", 250),
    ] {
        let mut mail = crate::store::email(id, inbox, "a@x.io");
        mail.received_ms = ms;
        srv.store.insert(mail).unwrap();
    }
    let srv = {
        let ids = |list: &[&str]| list.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let check = |srv: crate::RunningServer| async move {
            let addr = srv.http_addr;
            let scoped = ids(&["m-c", "m-b", "m-d", "m-a"]);
            for path in [
                "/api/v1/messages?inbox=ord",
                "/api/v1/search?inbox=ord&query=hi",
                "/api/v2/messages?inbox=ord",
                "/api/v2/search?inbox=ord&kind=containing&query=hi",
            ] {
                assert_eq!(page_ids(addr, path).await, scoped, "{path}");
            }
            assert_eq!(
                page_ids(addr, "/api/v1/messages?inbox=ord&start=1&limit=2").await,
                ids(&["m-b", "m-d"])
            );
            assert_eq!(
                page_ids(
                    addr,
                    "/api/v2/search?inbox=ord&kind=to&query=a@x.io&start=2&limit=1"
                )
                .await,
                ids(&["m-d"])
            );
            assert_eq!(
                page_ids(addr, "/api/v1/messages").await,
                ids(&["m-c", "m-b", "o-1", "m-d", "m-a"])
            );
            for (path, want) in [
                ("/api/v1/message/latest?inbox=ord", "m-c"),
                ("/api/v1/message/latest", "m-c"),
                ("/api/v1/message/latest?inbox=other", "o-1"),
            ] {
                let (status, body) = get(addr, path.into()).await;
                assert_eq!(status, 200);
                let body: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(body["ID"], want, "{path}");
            }
            // Native inbox order stays acceptance order, newest first.
            assert_eq!(
                page_ids(addr, "/api/v1/inboxes/ord/messages").await,
                ids(&["m-d", "m-c", "m-a", "m-b"])
            );
            srv
        };
        let srv = check(srv).await;
        srv.stop().await;
        let srv = crate::run_on(&config(Some(data_file))).await.unwrap();
        check(srv).await
    };
    srv.stop().await;
}

/// While all four compat page builds are held on the blocking pool, a new
/// SMTP peer is greeted and `/healthz` answers, and no gate has expired
/// (red on an inline build: the worker blocks inside the gate until its
/// deadline releases it, and only then can anything else be served).
/// Release lets every build finish with a 200. No sleep orders anything:
/// the gate reports entry, the test spins against a real deadline.
#[tokio::test]
async fn held_compat_page_builds_leave_smtp_and_health_serving() {
    let cfg = crate::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: None,
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
        pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
    };
    let srv = crate::run_on(&cfg).await.unwrap();
    let mut held = crate::store::email("p1", "parked", "a@x.io");
    held.text = Some("hi there".into());
    srv.store.insert(held).unwrap();

    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let expired = Arc::new(AtomicBool::new(false));
    {
        let (entered, gate, expired) = (entered.clone(), gate.clone(), expired.clone());
        srv.store.set_compat_gate(Some(Arc::new(move || {
            entered.fetch_add(1, Ordering::SeqCst);
            let (open, wake) = &*gate;
            let (_open, wait) = wake
                .wait_timeout_while(open.lock().unwrap(), Duration::from_secs(10), |open| !*open)
                .unwrap();
            expired.fetch_or(wait.timed_out(), Ordering::SeqCst);
        })));
    }

    let addr = srv.http_addr;
    let paths = [
        "/api/v1/messages?inbox=parked",
        "/api/v1/search?inbox=parked&query=hi",
        "/api/v2/messages?inbox=parked",
        "/api/v2/search?inbox=parked&kind=containing&query=hi",
    ];
    let requests: Vec<_> = paths
        .iter()
        .map(|path| tokio::spawn(get(addr, path.to_string())))
        .collect();
    let deadline = Instant::now() + Duration::from_secs(30);
    while entered.load(Ordering::SeqCst) < paths.len() {
        assert!(
            Instant::now() < deadline,
            "only {} of {} compat builds were ever held together",
            entered.load(Ordering::SeqCst),
            paths.len()
        );
        tokio::task::yield_now().await;
    }

    // Every build is parked: the worker must still greet and answer.
    let mut smtp = BufReader::new(TcpStream::connect(srv.smtp_addr).await.unwrap());
    let mut greeting = String::new();
    smtp.read_line(&mut greeting).await.unwrap();
    assert!(greeting.starts_with("220"), "{greeting}");
    let (status, health) = get(addr, "/healthz".into()).await;
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&health));
    assert!(
        !expired.load(Ordering::SeqCst),
        "SMTP and health were served only after a held build's gate expired"
    );

    {
        let (open, wake) = &*gate;
        *open.lock().unwrap() = true;
        wake.notify_all();
    }
    for request in requests {
        let (status, body) = request.await.unwrap();
        assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert!(
            body.get("messages").or(body.get("items")).is_some(),
            "{body}"
        );
    }
    assert!(!expired.load(Ordering::SeqCst));
    // With the gate removed the same build runs straight through.
    srv.store.set_compat_gate(None);
    let (status, _) = get(addr, "/api/v1/messages?inbox=parked".into()).await;
    assert_eq!(status, 200);
    srv.stop().await;
}
