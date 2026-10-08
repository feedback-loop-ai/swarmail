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
            let server = swarmail::run_on(&cfg)
                .await
                .expect("failed to bind listeners");
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
        Command::Mcp(args) => {
            // stdio MCP bridge: JSON-RPC over stdin/stdout → POST {url}/mcp.
            let code = swarmail::stdio::run_stdio_bridge(&args.url).await;
            std::process::exit(code);
        }
    }
}
