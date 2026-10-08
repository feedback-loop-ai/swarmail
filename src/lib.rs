//! Swarmail library — expose everything for integration tests and embedding.

pub mod api;
pub mod chaos;
pub mod config;
pub mod extract;
pub mod mcp;
pub mod model;
pub mod persist;
pub mod smtp;
pub mod stdio;
pub mod store;
pub mod ui;
pub mod webhook;

use std::sync::Arc;

/// Log a server task's exit; only stopping with an error is noteworthy.
fn log_stopped(what: &str, res: std::io::Result<()>) {
    if let Err(e) = res {
        tracing::error!(server = what, error = %e, "server stopped");
    }
}

/// A running server: both listeners bound, tasks spawned.
pub struct RunningServer {
    pub smtp_addr: std::net::SocketAddr,
    pub http_addr: std::net::SocketAddr,
    pub store: Arc<store::Store>,
    pub chaos: Arc<chaos::Chaos>,
    pub webhooks: Arc<webhook::Webhooks>,
    shutdown: tokio::sync::watch::Sender<bool>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl RunningServer {
    /// Ask both servers to stop and wait until they have logged their exit.
    pub async fn stop(self) {
        let _ = self.shutdown.send(true);
        for task in self.tasks {
            let _ = task.await;
        }
    }
}

/// Bind SMTP + HTTP and spawn both servers (plus the webhook dispatcher).
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
    webhook::spawn_dispatcher(store.clone(), webhooks.clone());

    let smtp_listener = tokio::net::TcpListener::bind(&cfg.smtp_listen).await?;
    let http_listener = tokio::net::TcpListener::bind(&cfg.http_listen).await?;
    let smtp_addr = smtp_listener.local_addr()?;
    let http_addr = http_listener.local_addr()?;

    let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
    let mut tasks = Vec::new();

    {
        let store = store.clone();
        let chaos = chaos.clone();
        let smtp_cfg = cfg.smtp.clone();
        let mut shutdown_rx = shutdown_rx.clone();
        tasks.push(tokio::spawn(async move {
            log_stopped(
                "smtp",
                smtp::serve(smtp_listener, store, chaos, smtp_cfg, async move {
                    let _ = shutdown_rx.changed().await;
                })
                .await,
            );
        }));
    }

    {
        let store = store.clone();
        let chaos = chaos.clone();
        let webhooks = webhooks.clone();
        let state = api::AppState {
            store,
            chaos,
            webhooks,
            started: std::time::Instant::now(),
        };
        let mut shutdown_rx = shutdown_rx.clone();
        tasks.push(tokio::spawn(async move {
            log_stopped(
                "http",
                api::serve(http_listener, state, async move {
                    let _ = shutdown_rx.changed().await;
                })
                .await,
            );
        }));
    }

    Ok(RunningServer {
        smtp_addr,
        http_addr,
        store,
        chaos,
        webhooks,
        shutdown,
        tasks,
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
