//! CLI + runtime configuration.

use crate::smtp::SmtpConfig;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "swarmail",
    version,
    about = "The fastest AI-native SMTP mail mock"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the Swarmail server (SMTP + HTTP: API, MCP, UI).
    Serve(ServeArgs),
    /// Run the MCP stdio bridge against a running Swarmail.
    Mcp(McpArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    /// SMTP listen address.
    #[arg(long, default_value = "0.0.0.0:1025", env = "SWARMAIL_SMTP_LISTEN")]
    pub smtp_listen: String,

    /// HTTP listen address (API, MCP, UI).
    #[arg(long, default_value = "0.0.0.0:8025", env = "SWARMAIL_HTTP_LISTEN")]
    pub http_listen: String,

    /// Maximum emails kept per inbox; oldest are pruned. 0 = unlimited.
    #[arg(long, default_value_t = 100_000, env = "SWARMAIL_MAX_PER_INBOX")]
    pub max_per_inbox: usize,
}

#[derive(Args, Debug, Clone)]
pub struct McpArgs {
    /// Base URL of a running Swarmail server.
    #[arg(long, default_value = "http://127.0.0.1:8025", env = "SWARMAIL_URL")]
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub smtp_listen: String,
    pub http_listen: String,
    pub max_per_inbox: usize,
    pub smtp: SmtpConfig,
}

impl From<&ServeArgs> for Config {
    fn from(args: &ServeArgs) -> Self {
        Self {
            smtp_listen: args.smtp_listen.clone(),
            http_listen: args.http_listen.clone(),
            max_per_inbox: args.max_per_inbox,
            smtp: SmtpConfig::default(),
        }
    }
}
