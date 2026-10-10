//! CLI + runtime configuration.

use crate::smtp::SmtpConfig;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

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
    /// Run the Swarmail server (SMTP + POP3 + HTTP: API, MCP, UI).
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

    /// HTTP listen address (API, MCP, UI, MailHog/Mailpit compat shims).
    #[arg(long, default_value = "0.0.0.0:8025", env = "SWARMAIL_HTTP_LISTEN")]
    pub http_listen: String,

    /// POP3 listen address (RFC 1939) serving the same store. A USER names
    /// an existing swarmail inbox; the maildrop is that inbox.
    #[arg(long, default_value = "0.0.0.0:1110", env = "SWARMAIL_POP3_LISTEN")]
    pub pop3_listen: String,

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

    /// Maximum time in milliseconds a STARTTLS handshake may take before
    /// the session is dropped: a client that stalls mid-upgrade must not
    /// tie a session task up forever. The pre-TLS plaintext idle posture
    /// has no timeout and stays that way.
    #[arg(
        long,
        value_name = "MS",
        default_value_t = 10_000,
        env = "SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS"
    )]
    pub tls_handshake_timeout_ms: u64,
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
    /// POP3 listener serving the same store; a USER names an existing inbox.
    pub pop3_listen: String,
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
            pop3_listen: args.pop3_listen.clone(),
            max_per_inbox: args.max_per_inbox,
            data_file: args.data_file.clone(),
            tls_cert: args.tls_cert.clone(),
            tls_key: args.tls_key.clone(),
            smtp: SmtpConfig {
                tls_handshake_timeout: Duration::from_millis(args.tls_handshake_timeout_ms),
                ..SmtpConfig::default()
            },
        }
    }
}
