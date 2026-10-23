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

/// Eagerly poll both sources on the startup task, then invoke startup with
/// their ownership. A spawned watcher cannot supply this ordering.
#[cfg(unix)]
async fn start_after_registration<S, R>(
    mut sigint: S,
    mut sigterm: S,
    poll: impl Fn(&mut S, &mut std::task::Context<'_>) -> std::task::Poll<Option<()>>,
    start: impl FnOnce(S, S) -> R,
) -> R {
    std::future::poll_fn(|cx| {
        let _ = poll(&mut sigint, cx);
        let _ = poll(&mut sigterm, cx);
        std::task::Poll::Ready(())
    })
    .await;
    start(sigint, sigterm)
}

#[tokio::main]
async fn main() {
    match Cli::parse().command {
        Command::Serve(args) => {
            init_tracing(false);
            let cfg = swarmail::config::Config::from(&args);
            // The interrupt streams are built and polled HERE, on the main
            // task, before any listener binds: registration is a stream's
            // first poll, not its construction, and the fast-path binds
            // complete without a yield — a watcher spawned and left to the
            // scheduler can stay unpolled until after the HTTP bind, so a
            // signal arriving once the port answers a connect (the binary
            // tests' wait_http_up, an operator's early ctrl-C) would kill
            // the process at the default disposition — no graceful stop,
            // no drain. One poll of each stream pends on the signal but
            // leaves the handler live; the spawned watcher owns the
            // registered streams from there. SIGINT and SIGTERM watch the
            // same channel: a container's `docker stop` sends SIGTERM
            // (STOPSIGNAL), a terminal's Ctrl-C sends SIGINT — both take
            // the same graceful path. A second signal during the drain
            // means the operator is not waiting: the process exits 70
            // instead of outliving its drain.
            let start =
                |#[cfg(unix)] mut sigint: tokio::signal::unix::Signal,
                 #[cfg(unix)] mut sigterm: tokio::signal::unix::Signal| async move {
                    let (interrupted_tx, interrupted_rx) = tokio::sync::watch::channel(false);
                    tokio::spawn(async move {
                        let mut graceful = false;
                        #[cfg(unix)]
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
                        #[cfg(not(unix))]
                        {
                            tokio::signal::ctrl_c().await.expect("ctrl_c handler");
                            let _ = interrupted_tx.send(true);
                        }
                    });
                    (swarmail::run_on(&cfg).await, interrupted_rx)
                };
            #[cfg(unix)]
            let (started, mut interrupted_rx) = {
                use tokio::signal::unix::{Signal, SignalKind, signal};
                start_after_registration(
                    signal(SignalKind::interrupt()).expect("SIGINT handler"),
                    signal(SignalKind::terminate()).expect("SIGTERM handler"),
                    Signal::poll_recv,
                    start,
                )
                .await
                .await
            };
            #[cfg(not(unix))]
            let (started, mut interrupted_rx) = start().await;
            // A server that fails to come up (bad bind, broken TLS pair)
            // exits non-zero with the reason — it must not sit half-alive.
            match started {
                Ok(server) => {
                    tracing::info!(
                        smtp = %server.smtp_addr,
                        pop3 = %server.pop3_addr,
                        http = %server.http_addr,
                        "Swarmail is up — SMTP on :{}, POP3 on :{}, API/UI on http://{}",
                        server.smtp_addr.port(),
                        server.pop3_addr.port(),
                        server.http_addr
                    );
                    // Serve until interrupted, then stop the servers gracefully.
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

#[cfg(all(test, unix))]
mod tests {
    /// No real signals, global handlers or background watcher can satisfy
    /// the continuation: both local sources must already have been polled.
    #[tokio::test]
    async fn signals_are_polled_before_startup() {
        use std::cell::Cell;
        use std::task::Poll;
        let interrupt_polled = Cell::new(false);
        let terminate_polled = Cell::new(false);
        super::start_after_registration(
            &interrupt_polled,
            &terminate_polled,
            |polled, _cx| {
                polled.set(true);
                Poll::Pending
            },
            |_, _| {
                assert!(
                    interrupt_polled.get(),
                    "SIGINT was not polled before startup"
                );
                assert!(
                    terminate_polled.get(),
                    "SIGTERM was not polled before startup"
                );
            },
        )
        .await;
    }
}
