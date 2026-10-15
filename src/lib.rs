//! Swarmail library — expose everything for integration tests and embedding.

pub mod api;
pub mod chaos;
pub mod compat;
pub mod config;
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

/// Log a server task's exit; only stopping with an error is noteworthy.
fn log_stopped(what: &str, res: std::io::Result<()>) {
    if let Err(e) = res {
        tracing::error!(server = what, error = %e, "server stopped");
    }
}

/// A running server: every listener bound, tasks spawned — and every task
/// the process runs owned by this struct's tracker, the dispatcher
/// included. Dropping it (without `stop`) aborts the lot; `stop` joins.
pub struct RunningServer {
    pub smtp_addr: std::net::SocketAddr,
    pub http_addr: std::net::SocketAddr,
    /// The POP3 listener; a USER names an existing inbox as the maildrop.
    pub pop3_addr: std::net::SocketAddr,
    pub store: Arc<store::Store>,
    pub chaos: Arc<chaos::Chaos>,
    pub webhooks: Arc<webhook::Webhooks>,
    shutdown: tokio::sync::watch::Sender<bool>,
    /// The drain one-shot (decision 0029 §5): sent only after the serve
    /// tasks' awaited commits have resolved, releasing the dispatcher to
    /// end its loop, sweep the ring and abort what is still in flight.
    drained: tokio::sync::watch::Sender<bool>,
    tasks: tokio::task::JoinSet<()>,
    /// The dispatcher's own set (decision 0029 §5): joined by `stop`
    /// *after* `tasks` and the `drained` send, in that order.
    dispatcher: tokio::task::JoinSet<()>,
}

impl RunningServer {
    /// Ask the servers to stop and wait until every owned task — the three
    /// serve loops and the webhook dispatcher — has returned. Nothing the
    /// process spawned is left untracked: an SSE feed ends on the same
    /// signal, a delivery in flight is aborted and counted, and a session
    /// mid-transaction dies with its socket.
    ///
    /// The sequencing is decision 0029 §5's drain: `shutdown` first; the
    /// serve tasks join (their awaited commits resolve *inside* this join,
    /// so every tracked commit's fan-out is already in the firehose); then
    /// `drained`; then the dispatcher joins — it has kept consuming the
    /// post-stop mail, counting each matching fan-out aborted, and now
    /// sweeps the ring and aborts what is left. Counted, never silently
    /// lost; `src/store.rs` untouched.
    pub async fn stop(self) {
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
    let webhooks = Arc::new(webhook::Webhooks::default());
    // The shutdown watch is born where the store is, and the dispatcher
    // spawns immediately after it — subscribing to the firehose before TLS
    // is parsed and before any listener binds (decision 0002, one layer
    // down): a mail accepted at startup can never miss its webhook. No
    // cleanup is added to the error legs below: on a failure the store
    // sender and this watch sender both drop, the firehose closes, and the
    // dispatcher self-resolves.
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
    // accept and clears it on the next success, and /healthz reads the pair
    // to answer `degraded` with the surfaces that cannot accept. Plain
    // atomics: the loop already reorders around the failure, and the flag
    // is advisory truth about a socket, not a lock domain.
    let smtp_degraded = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pop3_degraded = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // The dispatcher's own task set and the drain one-shot it waits for
    // (decision 0029 §5): `stop` releases it with `drained` only after the
    // serve tasks — and the commits they await — have resolved, so a commit
    // finishing during shutdown is counted, never silently lost.
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

    let smtp_listener = tokio::net::TcpListener::bind(&cfg.smtp_listen).await?;
    let http_listener = tokio::net::TcpListener::bind(&cfg.http_listen).await?;
    let smtp_addr = smtp_listener.local_addr()?;
    let http_addr = http_listener.local_addr()?;
    // Bound before any spawn: a bind failure fails the whole startup.
    let pop3_listener = tokio::net::TcpListener::bind(&cfg.pop3_listen).await?;
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
        // The Origin guard's vocabulary is the bound port, and it is live
        // only while the bind itself is loopback (decision 0030 §2): a
        // loopback bind serves the operator's machine, an off-loopback one
        // is the operator's own trust statement and the check is inert.
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
        let (idle_timeout, max_sessions) = (cfg.smtp.idle_timeout, cfg.smtp.max_sessions);
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
        shutdown,
        drained,
        tasks,
        dispatcher,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both branches of the server-exit logger: errors are noteworthy,
    /// clean stops are silent.
    #[test]
    fn log_stopped_only_reports_errors() {
        log_stopped("unit", Ok(())); // silent
        log_stopped("unit", Err(std::io::Error::other("boom"))); // logs
    }
}
