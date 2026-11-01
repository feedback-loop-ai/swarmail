//! Swarmail library — expose everything for integration tests and embedding.

pub mod api;
pub mod attachments;
pub mod chaos;
pub mod compat;
pub mod config;
pub mod deliverability;
pub mod extract;
pub mod feed;
pub mod mcp;
pub mod model;
pub mod persist;
pub mod pop3;
pub mod smtp;
pub mod stdio;
pub mod store;
pub mod threads;
pub mod tls;
pub mod ui;
pub mod webhook;

use std::sync::Arc;

/// The listen backlog of every listener (the kernel caps it at its own
/// `somaxconn`). The platform default of 128 overflowed under a burst of
/// fresh SMTP connections: with SYN cookies the client believes it is
/// connected, but SMTP waits for the server's greeting, so the dropped
/// connection hangs until the client gives up.
pub const LISTEN_BACKLOG: u32 = 4096;

/// Bind `addr` (a `host:port` resolved like `TcpListener::bind`, trying each
/// address in turn) and listen with [`LISTEN_BACKLOG`].
pub async fn listen(addr: &str) -> std::io::Result<tokio::net::TcpListener> {
    let mut last = std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "could not resolve to any address",
    );
    for addr in tokio::net::lookup_host(addr).await? {
        match listen_on(addr) {
            Ok(listener) => return Ok(listener),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn listen_on(addr: std::net::SocketAddr) -> std::io::Result<tokio::net::TcpListener> {
    let socket = if addr.is_ipv4() {
        tokio::net::TcpSocket::new_v4()?
    } else {
        tokio::net::TcpSocket::new_v6()?
    };
    // What `TcpListener::bind` sets on Unix: a restarted server rebinds a
    // port whose old connections still sit in TIME_WAIT.
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    socket.listen(LISTEN_BACKLOG)
}

/// Log a server task's exit; only stopping with an error is noteworthy.
fn log_stopped(what: &str, res: std::io::Result<()>) {
    if let Err(e) = res {
        tracing::error!(server = what, error = %e, "server stopped");
    }
}

/// A running server with bound listeners and owned serve/dispatcher trackers.
/// Dropping it aborts those trackers; `stop` joins them. Detached connection
/// tasks and blocking mutations can outlive their tracked callers.
pub struct RunningServer {
    pub smtp_addr: std::net::SocketAddr,
    pub http_addr: std::net::SocketAddr,
    /// The POP3 listener; a USER names an existing inbox as the maildrop.
    pub pop3_addr: std::net::SocketAddr,
    pub store: Arc<store::Store>,
    pub chaos: Arc<chaos::Chaos>,
    pub webhooks: Arc<webhook::Webhooks>,
    ca_owner: webhook::ca_executor::Owner,
    shutdown: tokio::sync::watch::Sender<bool>,
    /// The drain one-shot (decision 0029 §5): sent only after the serve
    /// tasks have resolved, releasing the dispatcher to
    /// end its loop, sweep the ring and abort what is still in flight.
    drained: tokio::sync::watch::Sender<bool>,
    tasks: tokio::task::JoinSet<()>,
    /// The dispatcher's own set (decision 0029 §5): joined by `stop`
    /// *after* `tasks` and the `drained` send, in that order.
    dispatcher: tokio::task::JoinSet<()>,
}

impl RunningServer {
    /// Signal shutdown, join the serve trackers, send drained, then join the
    /// dispatcher (decision 0029 §5). Fan-out from mutations those serve tasks
    /// still await is available before drained releases the dispatcher.
    ///
    /// This does not join detached blocking work or axum connection tasks.
    /// A late detached commit can miss the dispatcher's ledger; runtime/process
    /// teardown owns the remaining sockets (0024 §6, 0027 §7, 0041 §6).
    pub async fn stop(self) {
        self.ca_owner.close();
        let _ = self.shutdown.send(true);
        let mut tasks = self.tasks;
        while tasks.join_next().await.is_some() {}
        let _ = self.drained.send(true);
        let mut dispatcher = self.dispatcher;
        while dispatcher.join_next().await.is_some() {}
    }
}

/// Bind SMTP + POP3 + HTTP and spawn all three servers (plus the webhook
/// dispatcher).
pub async fn run_on(cfg: &config::Config) -> std::io::Result<RunningServer> {
    // The data file is opened and restored BEFORE any listener binds: a
    // server that answers must answer with its full state.
    let store = match &cfg.data_file {
        Some(path) => Arc::new(
            store::Store::open(cfg.max_per_inbox, path)
                .map_err(|e| std::io::Error::other(format!("data file {}: {e}", path.display())))?,
        ),
        None => Arc::new(store::Store::new(cfg.max_per_inbox)),
    };
    let chaos = Arc::new(chaos::Chaos::default());
    webhook::ca_executor::initialize();
    let webhooks = Arc::new(webhook::Webhooks::default());
    let ca_owner = webhook::ca_executor::Owner(webhooks.scope.clone());
    #[cfg(test)]
    lifecycle_tests::observe_ca_scope_startup(cfg, &webhooks.scope);
    // The shutdown watch is born where the store is, and the dispatcher
    // spawns immediately after it — subscribing to the firehose before TLS
    // is parsed and before any listener binds (decision 0002, one layer
    // down): a mail accepted at startup can never miss its webhook. No
    // cleanup is added to the error legs below: on a failure the watch
    // sender drops with the rest of the startup frame, and the dropped
    // `dispatcher` task set aborts its task outright — no mail was ever
    // accepted on an unbound listener, so there is nothing left to drain.
    let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
    // The store reads the same watch (decision 0024): wired here, once,
    // before the dispatcher spawns and before any listener binds — no
    // mutation can be admitted under an unwired flag and then refused by a
    // wired one mid-flight. The boundary is now the one place that knows
    // "stopping": a mutation reaching it after the stop refuses without a
    // SQLite call, which is what bounds shutdown to one busy wait.
    store.observe_shutdown(shutdown_rx.clone());
    let mut tasks = tokio::task::JoinSet::new();

    // One degraded flag per listening surface, created where the listeners
    // are bound (decision 0027): an accept loop sets its own on a failed
    // accept and clears it when it next observes a healthy accept — a poll
    // that parks pending (decision 0036 §2: an idle recovered listener
    // must not stay degraded waiting for a connection) or an accepted
    // connection; the backoff pause never clears it. /healthz reads the pair
    // to answer `degraded` with the surfaces that cannot accept. Plain
    // atomics: the loop already reorders around the failure, and the flag
    // is advisory truth about a socket, not a lock domain.
    let smtp_degraded = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pop3_degraded = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // The dispatcher's own task set and the drain one-shot it waits for
    // (decision 0029 §5): `stop` releases it with `drained` only after the
    // serve tasks — and the commits they still await — have resolved.
    // Detached late work is outside this observed dispatch ledger.
    let (drained, drained_rx) = tokio::sync::watch::channel(false);
    let mut dispatcher = tokio::task::JoinSet::new();
    webhook::spawn_dispatcher(
        &store,
        &webhooks,
        shutdown_rx.clone(),
        drained_rx,
        &mut dispatcher,
    );

    // TLS is parsed before anything binds: a server that answers must not
    // answer with a broken TLS config. Both halves of the pair are required.
    let tls = match (&cfg.tls_cert, &cfg.tls_key) {
        (None, None) => None,
        (Some(cert), Some(key)) => {
            let pair = tls::TlsConfig::from_files(cert, key)?;
            let server_cfg = pair
                .server_config()
                .map_err(|e| std::io::Error::other(format!("tls-cert {}: {e}", cert.display())))?;
            Some(Arc::new(server_cfg))
        }
        _ => {
            return Err(std::io::Error::other(
                "tls-cert and tls-key must be configured together",
            ));
        }
    };

    let smtp_listener = listen(&cfg.smtp_listen).await?;
    let http_listener = listen(&cfg.http_listen).await?;
    let smtp_addr = smtp_listener.local_addr()?;
    let http_addr = http_listener.local_addr()?;
    // Bound before any spawn: a bind failure fails the whole startup.
    let pop3_listener = listen(&cfg.pop3_listen).await?;
    let pop3_addr = pop3_listener.local_addr()?;

    {
        let store = store.clone();
        let chaos = chaos.clone();
        let smtp_cfg = cfg.smtp.clone();
        let tls = tls;
        let degraded = smtp_degraded.clone();
        let mut shutdown_rx = shutdown_rx.clone();
        tasks.spawn(async move {
            log_stopped(
                "smtp",
                smtp::serve(
                    smtp_listener,
                    store,
                    chaos,
                    smtp_cfg,
                    tls,
                    degraded,
                    async move {
                        let _ = shutdown_rx.changed().await;
                    },
                )
                .await,
            );
        });
    }

    {
        let store = store.clone();
        let chaos = chaos.clone();
        let webhooks = webhooks.clone();
        // The loopback guard's vocabulary is the bound port, and it is live
        // only while the bind itself is loopback (decision 0030 §2): a
        // loopback bind serves the operator's machine, an off-loopback one
        // is the operator's own trust statement — the Host allowlist is
        // inactive there, the Origin-to-Host check is not (decision 0043).
        let http_origin_guard = http_addr.ip().is_loopback().then(|| http_addr.port());
        let state = api::AppState {
            store,
            chaos,
            webhooks,
            started: std::time::Instant::now(),
            shutdown: shutdown_rx.clone(),
            smtp_degraded: smtp_degraded.clone(),
            pop3_degraded: pop3_degraded.clone(),
            http_origin_guard,
            route_domain: cfg.smtp.route_domain.clone(),
        };
        let mut shutdown_rx = shutdown_rx.clone();
        tasks.spawn(async move {
            log_stopped(
                "http",
                api::serve(http_listener, state, async move {
                    let _ = shutdown_rx.changed().await;
                })
                .await,
            );
        });
    }

    {
        let store = store.clone();
        let degraded = pop3_degraded.clone();
        // POP3's own idle bound (decision 0036 §4); the session cap stays
        // shared with SMTP's, as wired since 0027.
        let (idle_timeout, max_sessions) = (cfg.pop3_idle_timeout, cfg.smtp.max_sessions);
        let mut shutdown_rx = shutdown_rx.clone();
        tasks.spawn(async move {
            log_stopped(
                "pop3",
                pop3::serve(
                    pop3_listener,
                    store,
                    degraded,
                    idle_timeout,
                    max_sessions,
                    async move {
                        let _ = shutdown_rx.changed().await;
                    },
                )
                .await,
            );
        });
    }

    Ok(RunningServer {
        smtp_addr,
        http_addr,
        pop3_addr,
        store,
        chaos,
        webhooks,
        ca_owner,
        shutdown,
        drained,
        tasks,
        dispatcher,
    })
}

#[cfg(test)]
mod lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// The held tasks witness tracker ownership, not protocol/commit stages.
    #[tokio::test]
    async fn stop_joins_serve_before_drained_and_dispatcher_before_return() {
        use std::future::{Future, poll_fn};
        use std::task::Poll;
        use tokio::sync::oneshot;

        let cfg = config::Config {
            smtp_listen: "127.0.0.1:0".into(),
            http_listen: "127.0.0.1:0".into(),
            pop3_listen: "127.0.0.1:0".into(),
            max_per_inbox: 0,
            data_file: None,
            tls_cert: None,
            tls_key: None,
            smtp: Default::default(),
            pop3_idle_timeout: pop3::DEFAULT_IDLE_TIMEOUT,
        };
        let mut server = run_on(&cfg).await.unwrap();
        let (serve_entered, entered) = oneshot::channel();
        let (serve_release, release) = oneshot::channel();
        server.tasks.spawn(async move {
            serve_entered.send(()).unwrap();
            release.await.unwrap();
        });
        entered.await.unwrap();
        let (dispatcher_entered, entered) = oneshot::channel();
        let (dispatcher_release, release) = oneshot::channel();
        server.dispatcher.spawn(async move {
            dispatcher_entered.send(()).unwrap();
            release.await.unwrap();
        });
        entered.await.unwrap();
        let mut drained = server.drained.subscribe();
        let mut stop = std::pin::pin!(server.stop());
        poll_fn(|cx| {
            assert!(stop.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        assert!(!*drained.borrow(), "drained preceded the serve-task join");
        serve_release.send(()).unwrap();
        {
            let mut changed = std::pin::pin!(drained.changed());
            poll_fn(|cx| {
                assert!(stop.as_mut().poll(cx).is_pending());
                changed.as_mut().poll(cx)
            })
            .await
            .unwrap();
        }
        assert!(*drained.borrow());
        poll_fn(|cx| {
            assert!(stop.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        dispatcher_release.send(()).unwrap();
        stop.await;
    }

    /// Both branches of the server-exit logger: errors are noteworthy,
    /// clean stops are silent.
    #[test]
    fn log_stopped_only_reports_errors() {
        log_stopped("unit", Ok(())); // silent
        log_stopped("unit", Err(std::io::Error::other("boom"))); // logs
    }
}
