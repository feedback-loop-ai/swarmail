//! Swarmail — the fastest AI-native SMTP mail mock.

use clap::Parser;
use std::io::IsTerminal;
use swarmail::config::{Cli, Command};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swarmail=info,tower_http=off".into()),
        )
        // ANSI only when stdout can render it: a piped or captured log
        // (CI, systemd, `docker logs`, the binary tests' own readback of
        // the startup line) gets plain text, so the tracing fields stay
        // machine-parseable — `smtp=` must literally appear. A terminal
        // keeps its color.
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    match Cli::parse().command {
        Command::Serve(args) => {
            let cfg = swarmail::config::Config::from(&args);
            // The interrupt watcher registers both handlers before the
            // servers bind: the signal streams register on their first
            // poll, and a signal arriving between the startup line and
            // that first poll would kill the process at the default
            // disposition — no graceful stop, no drain. A spawned watcher
            // is polled at run_on's first yield, long before the line the
            // binary tests (and an operator's terminal) read. SIGINT and
            // SIGTERM watch the same channel: a container's `docker stop`
            // sends SIGTERM (STOPSIGNAL), a terminal's Ctrl-C sends SIGINT
            // — both take the same graceful path. A second signal during
            // the drain means the operator is not waiting: the process
            // exits 70 instead of outliving its drain.
            let (interrupted_tx, mut interrupted_rx) = tokio::sync::watch::channel(false);
            tokio::spawn(async move {
                let mut graceful = false;
                #[cfg(unix)]
                {
                    use tokio::signal::unix::{SignalKind, signal};
                    let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");
                    let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
                    loop {
                        tokio::select! {
                            _ = sigint.recv() => {}
                            _ = sigterm.recv() => {}
                        }
                        if graceful {
                            tracing::warn!(
                                "second signal during the drain; exiting without waiting"
                            );
                            std::process::exit(70);
                        }
                        graceful = true;
                        let _ = interrupted_tx.send(true);
                    }
                }
                #[cfg(not(unix))]
                {
                    tokio::signal::ctrl_c().await.expect("ctrl_c handler");
                    let _ = interrupted_tx.send(true);
                }
            });
            // A server that fails to come up (bad bind, broken TLS pair)
            // exits non-zero with the reason — it must not sit half-alive.
            match swarmail::run_on(&cfg).await {
                Ok(server) => {
                    tracing::info!(
                        smtp = %server.smtp_addr,
                        http = %server.http_addr,
                        "Swarmail is up — SMTP on :{}, API/UI on http://{}",
                        server.smtp_addr.port(),
                        server.http_addr
                    );
                    // Serve until interrupted, then stop both servers gracefully.
                    let _ = interrupted_rx.changed().await;
                    tracing::info!("shutting down");
                    server.stop().await;
                }
                Err(e) => {
                    eprintln!("swarmail: {e}");
                    std::process::exit(1);
                }
            }
        }
        Command::GenCert(args) => {
            match swarmail::tls::TlsConfig::generate_self_signed(&args.domain)
                .and_then(|pair| pair.write_to_dir(&args.out))
            {
                Ok((cert, key)) => println!("wrote {} and {}", cert.display(), key.display()),
                Err(e) => {
                    eprintln!("swarmail: {e}");
                    std::process::exit(1);
                }
            }
        }
        Command::Mcp(args) => {
            // stdio MCP bridge: JSON-RPC over stdin/stdout → POST {url}/mcp.
            let code = swarmail::stdio::run_stdio_bridge(&args.url).await;
            std::process::exit(code);
        }
    }
}
