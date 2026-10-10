//! The CLI/config contract, tested from outside the crate: the harness lives
//! in `tests/` so the production report never measures test-source branches
//! (the brokkr rule — inline `#[cfg(test)]` mods leak `matches!` arms into the
//! denominator as uncoverable branches).

use clap::Parser;
use std::time::Duration;
use swarmail::config::{Cli, Command, Config, ServeArgs};

/// The serve subcommand's args, or a hard stop — gen-cert and mcp are
/// not server configurations. Extracted so both arms are testable.
fn serve_args_or_panic(cli: Cli) -> ServeArgs {
    let Command::Serve(args) = cli.command else {
        panic!("expected serve");
    };
    args
}

#[test]
fn config_maps_the_cli_args() {
    let args = ServeArgs {
        smtp_listen: "127.0.0.1:2525".into(),
        http_listen: "127.0.0.1:8080".into(),
        pop3_listen: "127.0.0.1:1110".into(),
        max_per_inbox: 42,
        data_file: Some("/tmp/mail.db".into()),
        tls_cert: None,
        tls_key: None,
        tls_handshake_timeout_ms: 2500,
    };
    let cfg = Config::from(&args);
    assert_eq!(cfg.smtp_listen, "127.0.0.1:2525");
    assert_eq!(cfg.http_listen, "127.0.0.1:8080");
    assert_eq!(cfg.pop3_listen, "127.0.0.1:1110");
    assert_eq!(cfg.max_per_inbox, 42);
    assert_eq!(
        cfg.data_file.as_deref(),
        Some(std::path::Path::new("/tmp/mail.db"))
    );
    assert_eq!(cfg.tls_cert, None);
    assert_eq!(cfg.tls_key, None);
    assert_eq!(
        cfg.smtp.accept_any_auth,
        swarmail::smtp::SmtpConfig::default().accept_any_auth
    );
    // The handshake deadline rides the same posture config as the rest
    // of the SMTP knobs, in its CLI unit (milliseconds).
    assert_eq!(cfg.smtp.tls_handshake_timeout, Duration::from_millis(2500));
}

#[test]
fn cli_parses_env_defaults_and_subcommands() {
    let cli = Cli::try_parse_from(["swarmail", "serve"]).unwrap();
    let args = serve_args_or_panic(cli);
    assert_eq!(args.smtp_listen, "0.0.0.0:1025");
    assert_eq!(args.http_listen, "0.0.0.0:8025");
    assert_eq!(args.pop3_listen, "0.0.0.0:1110");
    assert_eq!(args.max_per_inbox, 100_000);
    assert!(args.data_file.is_none());
    assert!(args.tls_cert.is_none());
    assert_eq!(args.tls_handshake_timeout_ms, 10_000);

    // The handshake deadline is operator-tunable, in milliseconds.
    let cli =
        Cli::try_parse_from(["swarmail", "serve", "--tls-handshake-timeout-ms", "1234"]).unwrap();
    let args = serve_args_or_panic(cli);
    assert_eq!(args.tls_handshake_timeout_ms, 1234);

    let cli = Cli::try_parse_from(["swarmail", "serve", "--data-file", "/tmp/mail.db"]).unwrap();
    let args = serve_args_or_panic(cli);
    assert_eq!(
        args.data_file.as_deref(),
        Some(std::path::Path::new("/tmp/mail.db"))
    );

    let cli = Cli::try_parse_from(["swarmail", "mcp", "--url", "http://x:1"]).unwrap();
    let Command::Mcp(args) = cli.command else {
        panic!("expected mcp");
    };
    assert_eq!(args.url, "http://x:1");

    let cli = Cli::try_parse_from([
        "swarmail",
        "gen-cert",
        "--domain",
        "localhost",
        "--out",
        "/tmp/certs",
    ])
    .unwrap();
    let Command::GenCert(args) = cli.command else {
        panic!("expected gen-cert");
    };
    assert_eq!(args.domain, "localhost");
    assert_eq!(args.out, std::path::Path::new("/tmp/certs"));
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
