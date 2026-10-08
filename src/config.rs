//! CLI + runtime configuration.

use crate::smtp::SmtpConfig;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

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
    /// Mint a self-signed certificate + key (PEM) for a domain — the
    /// bootstrap for STARTTLS in dev and test.
    GenCert(GenCertArgs),
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

    /// SQLite file that persists every insert, delete and clear, and from
    /// which the full state is restored on startup. Omit for the default
    /// in-memory store.
    #[arg(long, value_name = "PATH", env = "SWARMAIL_DATA_FILE")]
    pub data_file: Option<PathBuf>,

    /// PEM certificate for STARTTLS on the SMTP port. The listener stays
    /// byte-identical plaintext without it. Requires --tls-key.
    #[arg(
        long,
        value_name = "FILE",
        env = "SWARMAIL_TLS_CERT",
        requires = "tls_key"
    )]
    pub tls_cert: Option<PathBuf>,

    /// PEM private key matching --tls-cert.
    #[arg(
        long,
        value_name = "FILE",
        env = "SWARMAIL_TLS_KEY",
        requires = "tls_cert"
    )]
    pub tls_key: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct GenCertArgs {
    /// DNS name (or literal IP) the certificate is issued for.
    #[arg(long)]
    pub domain: String,

    /// Directory the PEM files are written to: cert.pem + key.pem.
    #[arg(long, value_name = "DIR")]
    pub out: PathBuf,
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
    /// Optional SQLite data file: every mutation is written through and the
    /// full state is restored on startup.
    pub data_file: Option<PathBuf>,
    /// PEM pair for STARTTLS on the SMTP listener, as file paths. Both must
    /// be set together; the listener is plain otherwise.
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub smtp: SmtpConfig,
}

impl From<&ServeArgs> for Config {
    fn from(args: &ServeArgs) -> Self {
        Self {
            smtp_listen: args.smtp_listen.clone(),
            http_listen: args.http_listen.clone(),
            max_per_inbox: args.max_per_inbox,
            data_file: args.data_file.clone(),
            tls_cert: args.tls_cert.clone(),
            tls_key: args.tls_key.clone(),
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
            data_file: Some("/tmp/mail.db".into()),
            tls_cert: None,
            tls_key: None,
        };
        let cfg = Config::from(&args);
        assert_eq!(cfg.smtp_listen, "127.0.0.1:2525");
        assert_eq!(cfg.http_listen, "127.0.0.1:8080");
        assert_eq!(cfg.max_per_inbox, 42);
        assert_eq!(
            cfg.data_file.as_deref(),
            Some(std::path::Path::new("/tmp/mail.db"))
        );
        assert_eq!(cfg.tls_cert, None);
        assert_eq!(cfg.tls_key, None);
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
                && args.data_file.is_none()
                && args.tls_cert.is_none()
        ));

        let cli =
            Cli::try_parse_from(["swarmail", "serve", "--data-file", "/tmp/mail.db"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Serve(args) if args.data_file.as_deref() == Some(std::path::Path::new("/tmp/mail.db"))
        ));

        let cli = Cli::try_parse_from(["swarmail", "mcp", "--url", "http://x:1"]).unwrap();
        assert!(matches!(cli.command, Command::Mcp(args) if args.url == "http://x:1"));

        let cli = Cli::try_parse_from([
            "swarmail",
            "gen-cert",
            "--domain",
            "localhost",
            "--out",
            "/tmp/certs",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::GenCert(args)
                if args.domain == "localhost" && args.out == std::path::Path::new("/tmp/certs")
        ));
    }

    #[test]
    fn tls_flags_map_into_the_config() {
        let cli = Cli::try_parse_from([
            "swarmail",
            "serve",
            "--tls-cert",
            "/tmp/cert.pem",
            "--tls-key",
            "/tmp/key.pem",
        ])
        .unwrap();
        let args = serve_args_or_panic(cli);
        let cfg = Config::from(&args);
        assert_eq!(
            cfg.tls_cert.as_deref(),
            Some(std::path::Path::new("/tmp/cert.pem"))
        );
        assert_eq!(
            cfg.tls_key.as_deref(),
            Some(std::path::Path::new("/tmp/key.pem"))
        );
    }

    #[test]
    fn a_tls_cert_without_a_key_is_rejected() {
        let err =
            Cli::try_parse_from(["swarmail", "serve", "--tls-cert", "/tmp/cert.pem"]).unwrap_err();
        assert!(err.use_stderr(), "the pairing rule is a hard error");
    }

    /// The serve subcommand's args, or a hard stop — gen-cert and mcp are
    /// not server configurations. Extracted so both arms are testable.
    fn serve_args_or_panic(cli: Cli) -> ServeArgs {
        let Command::Serve(args) = cli.command else {
            panic!("expected serve");
        };
        args
    }

    #[test]
    #[should_panic(expected = "expected serve")]
    fn non_serve_subcommands_are_not_server_configs() {
        let cli = Cli::try_parse_from([
            "swarmail",
            "gen-cert",
            "--domain",
            "localhost",
            "--out",
            "/tmp/certs",
        ])
        .unwrap();
        serve_args_or_panic(cli);
    }
}
