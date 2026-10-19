//! The CLI/config contract, tested from outside the crate: the harness lives
//! in `tests/` so the production report never measures test-source branches
//! (the brokkr rule — inline `#[cfg(test)]` mods leak `matches!` arms into the
//! denominator as uncoverable branches).

mod common;

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
    // The defaults are loopback (decision 0030 §1): the mail mock serves
    // its own machine. Containers and services state their bind
    // explicitly — the binary's env knobs and the flags below.
    assert_eq!(args.smtp_listen, "127.0.0.1:1025");
    assert_eq!(args.http_listen, "127.0.0.1:8025");
    assert_eq!(args.pop3_listen, "127.0.0.1:1110");
    assert_eq!(args.max_per_inbox, 100_000);
    assert!(args.data_file.is_none());
    assert!(args.tls_cert.is_none());
    assert_eq!(args.tls_handshake_timeout_ms, 10_000);

    // Every bind is still operator-tunable, in the flag's explicit shape —
    // the container contract (`0.0.0.0`) is spelled there, not defaulted
    // into someone's laptop.
    let cli = Cli::try_parse_from([
        "swarmail",
        "serve",
        "--smtp-listen",
        "0.0.0.0:1025",
        "--http-listen",
        "0.0.0.0:8025",
        "--pop3-listen",
        "0.0.0.0:1110",
    ])
    .unwrap();
    let args = serve_args_or_panic(cli);
    assert_eq!(args.smtp_listen, "0.0.0.0:1025");
    assert_eq!(args.http_listen, "0.0.0.0:8025");
    assert_eq!(args.pop3_listen, "0.0.0.0:1110");

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

/// Independent idle defaults (decision 0036 §4), pinned by equality, never
/// by waiting them out: SMTP's five minutes ride `SmtpConfig::default`,
/// POP3's ten minutes are its own `Config` field from the CLI conversion
/// (red on the shared wiring, where POP3 had no value of its own).
#[test]
fn the_idle_defaults_are_independent_per_protocol() {
    assert_eq!(
        swarmail::smtp::SmtpConfig::default().idle_timeout,
        Duration::from_secs(300)
    );
    assert_eq!(
        swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
        Duration::from_secs(600)
    );
    let args = serve_args_or_panic(Cli::try_parse_from(["swarmail", "serve"]).unwrap());
    let cfg = Config::from(&args);
    assert_eq!(cfg.smtp.idle_timeout, Duration::from_secs(300));
    assert_eq!(cfg.pop3_idle_timeout, Duration::from_secs(600));
}

/// The CLI's own configuration, loopback `:0` binds so parallel tests
/// never collide, ready for one field's override.
fn cli_config() -> Config {
    let args = serve_args_or_panic(
        Cli::try_parse_from([
            "swarmail",
            "serve",
            "--smtp-listen",
            "127.0.0.1:0",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:0",
        ])
        .unwrap(),
    );
    Config::from(&args)
}

/// A short idle bound, far below the other protocol's compiled one.
const SHORT_IDLE: Duration = Duration::from_millis(150);

/// Independent idle overrides reach the protocol they name, through the
/// real consumer: `run_on` on the CLI's own configuration with only POP3's
/// bound shortened. The SMTP session opens (and finishes a round trip)
/// before the POP3 one, so SMTP's window starts first; the POP3 session is
/// answered `-ERR idle timeout` and closed, and only then is SMTP asked
/// again. Ignored POP3 wiring never expires (the read deadline fires);
/// crossed wiring — POP3 reading SMTP's field, or SMTP reading POP3's —
/// expires the wrong session, or the earlier-opened SMTP one too, and its
/// NOOP gets no `250`. Ordering, never a wall-clock measurement, decides.
#[tokio::test]
async fn a_pop3_idle_override_expires_pop3_and_leaves_smtp_alone() {
    let mut cfg = cli_config();
    cfg.pop3_idle_timeout = SHORT_IDLE;
    let srv = swarmail::run_on(&cfg).await.unwrap();
    let mut smtp = common::SmtpConn::connect(srv.smtp_addr).await;
    smtp.send("NOOP").await;
    assert_eq!(smtp.reply().await, "250 2.0.0 OK");
    let mut pop3 = common::Pop3::connect(srv.pop3_addr).await;
    assert_eq!(
        pop3.read_to_eof().await,
        "-ERR idle timeout\r\n",
        "POP3 expires on its own override"
    );
    smtp.send("NOOP").await;
    assert_eq!(
        smtp.reply().await,
        "250 2.0.0 OK",
        "SMTP keeps the CLI's compiled bound"
    );
    srv.stop().await;
}

/// The reciprocal: only SMTP's bound shortened on the CLI's configuration.
/// The POP3 session opens (and answers a CAPA) first; the SMTP session is
/// told `421 4.4.2` and closed, and the earlier POP3 session still answers.
/// Ignored SMTP wiring never expires; crossed wiring expires POP3 instead
/// or as well.
#[tokio::test]
async fn an_smtp_idle_override_expires_smtp_and_leaves_pop3_alone() {
    let mut cfg = cli_config();
    cfg.smtp.idle_timeout = SHORT_IDLE;
    let srv = swarmail::run_on(&cfg).await.unwrap();
    let mut pop3 = common::Pop3::connect(srv.pop3_addr).await;
    assert!(pop3.cmd("CAPA").await.starts_with("+OK"));
    assert!(!pop3.data().await.is_empty(), "the CAPA list");
    let mut smtp = common::SmtpConn::connect(srv.smtp_addr).await;
    assert_eq!(
        smtp.read_to_eof().await,
        "421 4.4.2 idle timeout, closing transmission channel\r\n",
        "SMTP expires on its own override"
    );
    assert!(
        pop3.cmd("CAPA").await.starts_with("+OK"),
        "POP3 keeps the CLI's compiled bound"
    );
    srv.stop().await;
}

/// The rendered `serve --help` tells the truth about idleness (decision
/// 0036 §4): both compiled defaults, taken from the constants the
/// listeners run with, and no claim that plaintext has no timeout (red on
/// the pre-0027 text). It also states the HTTP plane's Host and Origin
/// policy as the guard enforces it (decisions 0030 §2 and 0043, issue
/// #142): loopback arms both the Host allowlist and the Origin check, and
/// off loopback a present Origin must still match the Host authority —
/// not the old "arms the Origin guard", which named one check and implied
/// none off loopback. Whitespace is normalized so the assertion reads the
/// words, not clap's layout.
#[test]
fn serve_help_names_both_idle_defaults() {
    let err = Cli::try_parse_from(["swarmail", "serve", "--help"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    let help = err.render().to_string();
    let help = help.split_whitespace().collect::<Vec<_>>().join(" ");
    let minutes = |d: Duration| {
        assert_eq!(d.as_secs() % 60, 0, "{d:?} is documented in minutes");
        d.as_secs() / 60
    };
    assert!(
        help.contains(&format!(
            "{} minutes on SMTP, {} minutes on POP3",
            minutes(swarmail::smtp::SmtpConfig::default().idle_timeout),
            minutes(swarmail::pop3::DEFAULT_IDLE_TIMEOUT)
        )),
        "{help}"
    );
    assert!(!help.contains("has no timeout"), "{help}");
    assert!(!help.contains("idle-timeout"), "no idle knob is exposed");
    for phrase in [
        "Loopback by default, which arms both the Host and the Origin checks (decision 0030 §2)",
        "a Host must be a loopback spelling of the bound port, and so must a present Origin",
        "Off loopback the Host allowlist is inactive, but a present Origin must match the Host authority (decision 0043)",
    ] {
        assert!(
            help.contains(phrase),
            "serve --help must state {phrase:?}: {help}"
        );
    }
    assert!(
        !help.contains("which also arms the Origin guard"),
        "the one-check wording is gone: {help}"
    );
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
