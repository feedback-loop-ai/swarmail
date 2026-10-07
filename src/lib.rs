//! Swarmail library — expose everything for integration tests and embedding.

pub mod api;
pub mod chaos;
pub mod config;
pub mod extract;
pub mod model;
pub mod smtp;
pub mod store;

use std::sync::Arc;

/// A running server: both listeners bound, tasks spawned.
pub struct RunningServer {
    pub smtp_addr: std::net::SocketAddr,
    pub http_addr: std::net::SocketAddr,
    pub store: Arc<store::Store>,
    pub chaos: Arc<chaos::Chaos>,
}

/// Bind SMTP + HTTP and spawn both servers.
pub async fn run_on(cfg: &config::Config) -> std::io::Result<RunningServer> {
    let store = Arc::new(store::Store::new(cfg.max_per_inbox));
    let chaos = Arc::new(chaos::Chaos::default());

    let smtp_listener = tokio::net::TcpListener::bind(&cfg.smtp_listen).await?;
    let http_listener = tokio::net::TcpListener::bind(&cfg.http_listen).await?;
    let smtp_addr = smtp_listener.local_addr()?;
    let http_addr = http_listener.local_addr()?;

    {
        let store = store.clone();
        let chaos = chaos.clone();
        let smtp_cfg = cfg.smtp.clone();
        tokio::spawn(async move {
            if let Err(e) = smtp::serve(smtp_listener, store, chaos, smtp_cfg).await {
                tracing::error!(error = %e, "smtp server stopped");
            }
        });
    }

    {
        let store = store.clone();
        let chaos = chaos.clone();
        let state = api::AppState {
            store,
            chaos,
            started: std::time::Instant::now(),
        };
        tokio::spawn(async move {
            if let Err(e) = api::serve(http_listener, state).await {
                tracing::error!(error = %e, "http server stopped");
            }
        });
    }

    Ok(RunningServer {
        smtp_addr,
        http_addr,
        store,
        chaos,
    })
}
