//! Cold-path harness (0020): incomplete/oversized interim-head diagnostics
//! and hung-serve watchdogs must remain cold on a successful proof.
use crate::{api, chaos::Chaos, store::Store, webhook::Webhooks};
use std::future::{Future, poll_fn};
use std::sync::{Arc, atomic::AtomicBool};
use std::task::Poll;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn http_server() -> (
    std::net::SocketAddr,
    tokio::sync::watch::Sender<bool>,
    tokio::sync::oneshot::Receiver<()>,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, rx) = tokio::sync::watch::channel(false);
    let state = api::AppState {
        store: Arc::new(Store::new(0)),
        chaos: Arc::new(Chaos::default()),
        webhooks: Arc::new(Webhooks::default()),
        started: Instant::now(),
        shutdown: rx.clone(),
        smtp_degraded: Arc::new(AtomicBool::new(false)),
        pop3_degraded: Arc::new(AtomicBool::new(false)),
        http_origin_guard: Some(addr.port()),
    };
    let receipt = api::observe_next_drain_deadline();
    let mut serving = Box::pin(api::serve(listener, state, api::shutdown_signal(rx)));
    // Explicitly poll entry before spawning, consuming this invocation's
    // receipt without relying on scheduler progress.
    poll_fn(|cx| {
        assert!(serving.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let serve = tokio::spawn(serving);
    (addr, stop, receipt, serve)
}

#[tokio::test(flavor = "current_thread")]
async fn stalled_body_selects_drain_deadline() {
    let (addr, stop, receipt, mut serve) = http_server().await;
    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
    client
        .write_all(
            format!(
                "POST /api/v1/inboxes/stall/seed HTTP/1.1\r\nHost: {addr}\r\n\
         Expect: 100-continue\r\nContent-Type: application/json\r\n\
         Content-Length: 10\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    client.write_all(b"{\"to").await.unwrap();
    // TCP may fragment the interim head: retain the incomplete body while
    // reading a complete, bounded head rather than assuming one read.
    let head = tokio::time::timeout(Duration::from_secs(30), async {
        let mut head = Vec::new();
        loop {
            head.push(
                client
                    .read_u8()
                    .await
                    .expect("EOF before complete interim head"),
            );
            if head.ends_with(b"\r\n\r\n") {
                break head;
            }
            assert!(head.len() < 4096, "oversized interim head");
        }
    })
    .await
    .expect("hyper never asked for the body");
    assert_eq!(head, b"HTTP/1.1 100 Continue\r\n\r\n");
    stop.send(true).unwrap();
    // This is a hung-proof guard, not a new product process-wall promise.
    let completed = tokio::time::timeout(Duration::from_secs(30), &mut serve).await;
    if completed.is_err() {
        serve.abort();
    }
    completed
        .expect("stalled-body drain watchdog expired")
        .unwrap()
        .unwrap();
    receipt
        .await
        .expect("the selected drain-deadline arm emitted no receipt");
    // The socket and partial body survived until serve returned. Dropping
    // serve does not promise to close axum's detached connection tasks.
    drop(client);
}

#[tokio::test(flavor = "current_thread")]
async fn graceful_http_completion_emits_no_deadline() {
    let (addr, stop, receipt, mut serve) = http_server().await;
    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
    client
        .write_all(
            format!("GET /healthz HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let mut reply = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), client.read_to_end(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert!(reply.starts_with(b"HTTP/1.1 200 OK\r\n"));
    stop.send(true).unwrap();
    let completed = tokio::time::timeout(Duration::from_secs(30), &mut serve).await;
    if completed.is_err() {
        serve.abort();
    }
    completed
        .expect("graceful HTTP watchdog expired")
        .unwrap()
        .unwrap();
    assert!(
        receipt.await.is_err(),
        "graceful serve selected the deadline arm"
    );
}

// Identity-keyed startup observation; a different server cannot consume it.
static CA_STARTUP: std::sync::Mutex<
    Option<(
        String,
        tokio::sync::oneshot::Sender<crate::webhook::ca_executor::Scope>,
    )>,
> = std::sync::Mutex::new(None);
pub(super) fn observe_ca_scope_startup(
    cfg: &crate::config::Config,
    scope: &crate::webhook::ca_executor::Scope,
) {
    let mut hook = CA_STARTUP.lock().unwrap();
    if hook
        .as_ref()
        .is_some_and(|(key, _)| key == &cfg.smtp_listen)
    {
        let (_, send) = hook.take().unwrap();
        let _ = send.send(scope.clone());
    }
}

#[tokio::test]
async fn ca_server_drop_and_startup_failure_close_only_their_scopes() {
    let mut config = crate::config::Config {
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
    let (send, scope) = tokio::sync::oneshot::channel();
    *CA_STARTUP.lock().unwrap() = Some(("ca-startup-failure-probe".into(), send));
    let first = crate::run_on(&config).await.unwrap();
    let second = crate::run_on(&config).await.unwrap();
    let first_hooks = first.webhooks.clone();
    let second_hooks = second.webhooks.clone();
    drop(first);
    assert!(first_hooks.scope.live().is_none());
    assert!(second_hooks.scope.live().is_some());
    config.smtp_listen = "ca-startup-failure-probe".into();
    config.tls_key = Some("unread-pair-half".into());
    assert!(crate::run_on(&config).await.is_err());
    let startup_scope = scope.await.unwrap();
    assert!(
        startup_scope.live().is_none(),
        "startup error drops the owner token"
    );
    assert!(second_hooks.scope.live().is_some());
    second.stop().await;
    assert!(second_hooks.scope.live().is_none());
}
