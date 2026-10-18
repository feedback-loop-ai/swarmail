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
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).await.unwrap();
    let split = reply.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&reply[..split]).to_string();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, reply[split + 4..].to_vec())
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
