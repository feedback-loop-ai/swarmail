//! Swarmail — the fastest AI-native SMTP mail mock.

use clap::Parser;
use swarmail::config::{Cli, Command};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swarmail=info,tower_http=off".into()),
        )
        .init();

    match Cli::parse().command {
        Command::Serve(args) => {
            let cfg = swarmail::config::Config::from(&args);
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
                    tokio::signal::ctrl_c().await.expect("ctrl_c handler");
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
