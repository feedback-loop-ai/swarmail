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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_maps_the_cli_args() {
        let args = ServeArgs {
            smtp_listen: "127.0.0.1:2525".into(),
            http_listen: "127.0.0.1:8080".into(),
            max_per_inbox: 42,
        };
        let cfg = Config::from(&args);
        assert_eq!(cfg.smtp_listen, "127.0.0.1:2525");
        assert_eq!(cfg.http_listen, "127.0.0.1:8080");
        assert_eq!(cfg.max_per_inbox, 42);
        assert_eq!(
            cfg.smtp.accept_any_auth,
            SmtpConfig::default().accept_any_auth
        );
    }

    #[test]
    fn cli_parses_env_defaults_and_subcommands() {
        let cli = Cli::try_parse_from(["swarmail", "serve"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Serve(args) if args.smtp_listen == "0.0.0.0:1025"
                && args.max_per_inbox == 100_000
        ));

        let cli = Cli::try_parse_from(["swarmail", "mcp", "--url", "http://x:1"]).unwrap();
        assert!(matches!(cli.command, Command::Mcp(args) if args.url == "http://x:1"));
    }
}
