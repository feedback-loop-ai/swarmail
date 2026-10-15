//! Swarmail — the fastest AI-native SMTP mail mock.

use clap::Parser;
use std::io::IsTerminal;
use swarmail::config::{Cli, Command};

/// Tracing initialization, per subcommand (decision 0029 §4): the `mcp`
/// bridge's stdout is protocol — a line-parsing client must never read a
/// tracing line — so its logs go to stderr; `serve` keeps stdout, where the
/// binary tests (and an operator's terminal) read the piped `smtp=` startup
/// line. ANSI only when the target stream can render it, so captured logs
/// stay machine-parseable.
fn init_tracing(to_stderr: bool) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "swarmail=info,tower_http=off".into());
    if to_stderr {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            // `serve` keeps stdout — and the binary tests (and a
            // line-parsing supervisor) read the piped `smtp=` startup
            // line, so the same §4 rule binds here: ANSI only when the
            // target stream can render it. A piped capture must still
            // parse (`parse_addrs` splits on the literal `smtp=`, which
            // escapes would bury) however the stream is captured.
            .with_ansi(std::io::stdout().is_terminal())
            .init();
    }
}

#[tokio::main]
async fn main() {
    match Cli::parse().command {
        Command::Serve(args) => {
            init_tracing(false);
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
            init_tracing(false);
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
            // Logs to stderr (decision 0029 §4): the bridge's stdout is
            // protocol and nothing else.
            init_tracing(true);
            let code = swarmail::stdio::run_stdio_bridge(&args.url).await;
            std::process::exit(code);
        }
    }
}
