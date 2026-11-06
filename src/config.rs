//! CLI + runtime configuration.

use crate::smtp::SmtpConfig;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "swarmail",
    version,
    about = "Built for a program waiting on mail."
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
    /// SMTP listen address. The default is loopback (decision 0030 §1) —
    /// a mail mock serves its own machine; containers and services state
    /// `0.0.0.0:1025` explicitly (the image and chart do).
    #[arg(long, default_value = "127.0.0.1:1025", env = "SWARMAIL_SMTP_LISTEN")]
    pub smtp_listen: String,

    /// HTTP listen address (API, MCP, UI, MailHog/Mailpit compat shims).
    /// Loopback by default, which arms both the Host and the Origin checks
    /// (decision 0030 §2): a Host must be a loopback spelling of the bound
    /// port, and so must a present Origin. Off loopback the Host allowlist
    /// is inactive, but a present Origin must match the Host authority
    /// (decision 0043).
    #[arg(long, default_value = "127.0.0.1:8025", env = "SWARMAIL_HTTP_LISTEN")]
    pub http_listen: String,

    /// POP3 listen address (RFC 1939) serving the same store. A USER names
    /// an existing swarmail inbox; the maildrop is that inbox. Loopback by
    /// default (decision 0030 §1).
    #[arg(long, default_value = "127.0.0.1:1110", env = "SWARMAIL_POP3_LISTEN")]
    pub pop3_listen: String,

    /// MailSlurper service-API listen address (decision 0118). Off by
    /// default: unset means no socket and no route. When set, the listener
    /// serves `/mail` and `/mailcount` over the WHOLE store as one global
    /// mailbox, and its `DELETE /mail {"pruneCode":"all"}` clears every
    /// inbox: enabling it collapses per-inbox isolation for its clients.
    /// The address is the operator's own (loopback only if written so).
    #[arg(long, value_name = "ADDR", env = "SWARMAIL_MAILSLURPER_LISTEN")]
    pub mailslurper_listen: Option<String>,

    /// Maximum emails kept per inbox; oldest are pruned. 0 = unlimited.
    #[arg(long, default_value_t = 100_000, env = "SWARMAIL_MAX_PER_INBOX")]
    pub max_per_inbox: usize,

    /// Maximum concurrent sessions on each of the SMTP and POP3 listeners;
    /// one more is answered `421` and closed. At least 1.
    #[arg(
        long,
        default_value_t = crate::smtp::DEFAULT_MAX_SESSIONS,
        env = "SWARMAIL_MAX_SESSIONS",
        value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..)
    )]
    pub max_sessions: usize,

    /// SQLite file that persists every insert, delete and clear, and from
    /// which the full state is restored on startup. Omit for the default
    /// in-memory store.
    #[arg(long, value_name = "PATH", env = "SWARMAIL_DATA_FILE")]
    pub data_file: Option<PathBuf>,

    /// PEM certificate for TLS on the SMTP port: STARTTLS, or implicit TLS
    /// with --smtp-implicit-tls. The listener stays byte-identical
    /// plaintext without it. Requires --tls-key.
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

    /// Maximum time in milliseconds a TLS handshake (STARTTLS or implicit)
    /// may take before the session is dropped: a client that stalls
    /// mid-handshake must not tie a session task up forever. With
    /// --smtp-implicit-tls, 0 makes every connection fail before the
    /// greeting. Plaintext sessions are bounded per
    /// read and per write by a compiled idle timeout — 5 minutes on SMTP,
    /// 10 minutes on POP3 — not by a session lifetime (decision 0036).
    #[arg(
        long,
        value_name = "MS",
        default_value_t = 10_000,
        env = "SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS"
    )]
    pub tls_handshake_timeout_ms: u64,

    /// Speak TLS from the first byte on the SMTP listener (implicit TLS,
    /// RFC 8314). Requires --tls-cert/--tls-key. STARTTLS is not offered.
    #[arg(long, env = "SWARMAIL_SMTP_IMPLICIT_TLS")]
    pub smtp_implicit_tls: bool,

    /// Route mail by recipient (decision 0112): an SMTP envelope recipient
    /// `<local>@<label>.<DOMAIN>` is stored in inbox `<label>`, outranking
    /// the AUTH username, so parallel agents behind one sending identity
    /// stay apart. One or more DNS labels joined by `.`. Off by default; the
    /// `swarmail_create_inbox` tool mints addresses under it.
    #[arg(
        long,
        value_name = "DOMAIN",
        env = "SWARMAIL_ROUTE_DOMAIN",
        value_parser = parse_route_domain
    )]
    pub route_domain: Option<String>,

    /// JSON file declaring the TXT records and temporary-failure names that
    /// deliverability reports read (decision 0120): DKIM selectors, SPF and
    /// DMARC, offline and deterministic. A bad file stops startup before any
    /// listener binds; there is no fallback to an empty map. Per-inbox maps
    /// replace it through `PUT /api/v1/inboxes/{inbox}/dns`.
    #[arg(long, value_name = "FILE", env = "SWARMAIL_DNS_POLICY")]
    pub dns_policy: Option<PathBuf>,

    /// Where DNS answers come from: `declared` (the default, offline) or
    /// `real`, which this build refuses (DKIM verification reads declared keys
    /// only, proposed decision 0125).
    #[arg(
        long,
        value_name = "MODE",
        default_value = "declared",
        env = "SWARMAIL_DNS_RESOLVER",
        value_parser = parse_dns_resolver
    )]
    pub dns_resolver: DnsResolver,

    /// What the opt-in link-reachability check may contact (decision 0121).
    /// `off` (the default) opens no connection, ever. `public` lets a caller
    /// of `GET /api/v1/messages/{id}/deliverability?reachability=true` make
    /// the server fetch the headers of a message's http(s) links, refusing
    /// private, link-local and other non-public addresses; `loopback` also
    /// permits 127.0.0.0/8 and ::1, so mail can cause GETs to local ports.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::deliverability::LinkReachability::Off,
        env = "SWARMAIL_LINK_REACHABILITY"
    )]
    pub link_reachability: crate::deliverability::LinkReachability,
}

/// The DNS answer source. Only the declared map exists in this build: `real`
/// is refused by `parse_dns_resolver`, so no listener binds under it
/// (decision 0120 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResolver {
    Declared,
}

/// Validate `--dns-resolver`. `real` is configured and refused with a message
/// that says how to declare records instead (decision 0125 section 3); anything
/// else is unknown.
pub fn parse_dns_resolver(value: &str) -> Result<DnsResolver, String> {
    match value {
        "declared" => Ok(DnsResolver::Declared),
        "real" => Err(
            "real DNS is not supported; declare records with --dns-policy or PUT /api/v1/inboxes/{inbox}/dns"
                .into(),
        ),
        other => Err(format!(
            "--dns-resolver must be `declared` or `real`, got {other:?}"
        )),
    }
}

/// Validate `--route-domain` and fold it to ASCII lowercase (decision 0112
/// §1): one or more labels of `[A-Za-z0-9-]{1,63}` joined by single dots.
/// An empty value, an empty label (which covers a leading, trailing or
/// doubled dot), a character outside the alphabet or an over-long label is
/// refused at startup, before any listener binds.
pub fn parse_route_domain(value: &str) -> Result<String, String> {
    for label in value.split('.') {
        let ok = (1..=63).contains(&label.len())
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if !ok {
            return Err(format!(
                "--route-domain must be DNS labels of 1-63 letters, digits or '-' joined by '.', got {value:?}"
            ));
        }
    }
    Ok(value.to_ascii_lowercase())
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
    /// Base URL of a running Swarmail server (http:// only; no port means 80).
    #[arg(long, default_value = "http://127.0.0.1:8025", env = "SWARMAIL_URL")]
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub smtp_listen: String,
    pub http_listen: String,
    /// POP3 listener serving the same store; a USER names an existing inbox.
    pub pop3_listen: String,
    /// The opt-in MailSlurper service-API listener (decision 0118): `None`
    /// binds nothing.
    pub mailslurper_listen: Option<String>,
    pub max_per_inbox: usize,
    /// Optional SQLite data file: every mutation is written through and the
    /// full state is restored on startup.
    pub data_file: Option<PathBuf>,
    /// The server DNS map file (decision 0120): loaded and checked by `run_on`
    /// before the data file opens and before any listener binds.
    pub dns_policy: Option<PathBuf>,
    /// PEM pair for STARTTLS on the SMTP listener, as file paths. Both must
    /// be set together; the listener is plain otherwise.
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub smtp: SmtpConfig,
    /// The POP3 single-I/O idle bound, independent of SMTP's (decision
    /// 0036 §4): no CLI knob, `pop3::DEFAULT_IDLE_TIMEOUT` from the CLI,
    /// overridable in-process. A Rust struct literal must name it.
    pub pop3_idle_timeout: Duration,
    /// The opt-in link-reachability mode (decision 0121): `Off` opens no
    /// connection. A Rust struct literal must name it.
    pub link_reachability: crate::deliverability::LinkReachability,
}

impl From<&ServeArgs> for Config {
    fn from(args: &ServeArgs) -> Self {
        Self {
            smtp_listen: args.smtp_listen.clone(),
            http_listen: args.http_listen.clone(),
            pop3_listen: args.pop3_listen.clone(),
            mailslurper_listen: args.mailslurper_listen.clone(),
            max_per_inbox: args.max_per_inbox,
            data_file: args.data_file.clone(),
            dns_policy: args.dns_policy.clone(),
            tls_cert: args.tls_cert.clone(),
            tls_key: args.tls_key.clone(),
            smtp: SmtpConfig {
                tls_handshake_timeout: Duration::from_millis(args.tls_handshake_timeout_ms),
                max_sessions: args.max_sessions,
                route_domain: args.route_domain.clone(),
                implicit_tls: args.smtp_implicit_tls,
                ..SmtpConfig::default()
            },
            pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
            link_reachability: args.link_reachability,
        }
    }
}
