//! STARTTLS: real upgrades, real verification, real failure modes — spoken
//! with a rustls client against live servers.
//!
//! - Without TLS config the listener is byte-identical, down to the exact
//!   EHLO body, and refuses STARTTLS with 454 while staying usable.
//! - With a cert configured, EHLO advertises STARTTLS, the upgrade is a real
//!   rustls handshake, the session restarts (RFC 3207 §4.2), and a `250` on
//!   a TLS session still means the mail is queryable (decision 0001).
//! - Every TLS error path ends cleanly: a cert the client refuses, a
//!   malformed PEM file, half a pair.
//! - A stalled handshake is closed by the deadline — a silent client
//!   cannot tie a session task up forever — and the server serves on.

mod common;

use common::{
    SmtpConn, assert_starttls_handshake_fails, http_json, stalled_starttls, start, start_with,
    starttls, starttls_discarding_pipelined,
};
use std::path::PathBuf;
use std::time::Duration;
use swarmail::RunningServer;
use swarmail::config::Config;
use swarmail::smtp::SmtpConfig;
use swarmail::tls::TlsConfig;
use tokio::io::AsyncReadExt;
use tokio_rustls::rustls::pki_types::CertificateDer;

/// A server serving STARTTLS from a real PEM pair on disk, plus the cert
/// the client must trust (the served one) and the scratch dir.
async fn start_tls(name: &str) -> (RunningServer, CertificateDer<'static>, PathBuf) {
    start_tls_cfg(name, SmtpConfig::default()).await
}

/// [`start_tls`] with an explicit SMTP posture — the handshake deadline
/// knob among the rest of the `SmtpConfig` fields.
async fn start_tls_cfg(
    name: &str,
    smtp: SmtpConfig,
) -> (RunningServer, CertificateDer<'static>, PathBuf) {
    let pair = TlsConfig::generate_self_signed("localhost").unwrap();
    let dir = std::env::temp_dir().join(format!(
        "swarmail-starttls-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (cert, key) = pair.write_to_dir(&dir).unwrap();
    let server = start_with(|mut c| {
        c.tls_cert = Some(cert.clone());
        c.tls_key = Some(key.clone());
        c.smtp = smtp;
        c
    })
    .await;
    let root = rustls_pemfile::certs(&mut pair.cert_pem.as_bytes())
        .next()
        .unwrap()
        .unwrap();
    (server, root, dir)
}

fn base_cfg() -> Config {
    Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: None,
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
    }
}

/// Without TLS config the listener is byte-identical to the pre-STARTTLS
/// server: no extension in EHLO, a 454 for STARTTLS, and the session keeps
/// working afterwards.
#[tokio::test]
async fn plain_listener_is_byte_identical_and_refuses_starttls() {
    let server = start().await;
    let mut c = SmtpConn::connect(server.smtp_addr).await;
    c.send("EHLO byte-check").await;
    assert_eq!(
        c.reply_multi().await,
        "250-swarmail.local\n250-PIPELINING\n250-8BITMIME\n250-SMTPUTF8\n\
         250-SIZE 52428800\n250-AUTH PLAIN LOGIN\n250 OK",
        "the plaintext EHLO body must not change"
    );
    c.send("STARTTLS").await;
    let refused = c.reply().await;
    assert!(refused.starts_with("454"), "{refused}");
    assert!(refused.contains("TLS not available"), "{refused}");
    c.send("NOOP").await;
    assert!(
        c.reply().await.starts_with("250"),
        "the session must survive the refusal"
    );
}

/// With a cert configured the extension appears — exactly one line, between
/// SIZE and AUTH.
#[tokio::test]
async fn ehlo_advertises_starttls_only_once_tls_is_configured() {
    let (server, _root, _dir) = start_tls("advertise").await;
    let mut c = SmtpConn::connect(server.smtp_addr).await;
    c.send("EHLO offer-check").await;
    assert_eq!(
        c.reply_multi().await,
        "250-swarmail.local\n250-PIPELINING\n250-8BITMIME\n250-SMTPUTF8\n\
         250-SIZE 52428800\n250-STARTTLS\n250-AUTH PLAIN LOGIN\n250 OK"
    );
}

/// The full upgrade: real rustls handshake, fresh session, delivery over
/// TLS — and a `250` still means queryable (decision 0001), same as
/// everywhere else in Swarmail.
#[tokio::test]
async fn starttls_upgrades_and_delivers_over_tls() {
    let (server, root, _dir) = start_tls("deliver").await;
    let mut tls = starttls(server.smtp_addr, "localhost", root).await;

    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "\u{0}tlsbox\u{0}whatever",
    );
    tls.send(&format!("AUTH PLAIN {b64}")).await;
    assert!(tls.reply_raw().await.starts_with("235"));

    tls.send("MAIL FROM:<sender@x.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"), "MAIL over TLS");
    tls.send("RCPT TO:<tls@y.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"), "RCPT over TLS");
    let stored = tls.data("Subject: over tls\r\n\r\nsecret body").await;
    assert!(stored.starts_with("250"), "{stored}");

    let (code, body) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/tlsbox/messages",
        None,
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(body["emails"][0]["subject"], "over tls");

    tls.send("QUIT").await;
    assert!(tls.reply_raw().await.starts_with("221"));
}

/// Anything pipelined behind STARTTLS in plaintext must be thrown away, not
/// acted on after the upgrade (RFC 3207 §4.2): the new session starts
/// empty, and a smuggled MAIL FROM buys nothing.
#[tokio::test]
async fn plaintext_pipelined_behind_starttls_is_discarded() {
    let (server, root, _dir) = start_tls("discard").await;
    let mut tls = starttls_discarding_pipelined(
        server.smtp_addr,
        "localhost",
        root,
        &["MAIL FROM:<evil@x.io>"],
    )
    .await;

    tls.send("RCPT TO:<still@y.io>").await;
    let refused = tls.reply_raw().await;
    assert!(refused.starts_with("503"), "expected 503, got {refused}");

    // The TLS session is a normal one: a fresh envelope is accepted.
    tls.send("MAIL FROM:<good@x.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    tls.send("RCPT TO:<fresh@y.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    let stored = tls.data("Subject: fresh\r\n\r\nbody").await;
    assert!(stored.starts_with("250"), "{stored}");
    let (code, body) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(code, 200);
    assert_eq!(body["emails"][0]["subject"], "fresh");
}

/// STARTTLS takes no parameters (RFC 3207 §4): 501, and the offer stays open.
#[tokio::test]
async fn starttls_with_arguments_is_a_syntax_error() {
    let (server, _root, _dir) = start_tls("args").await;
    let mut c = SmtpConn::connect(server.smtp_addr).await;
    c.send("EHLO arg-check").await;
    c.reply_multi().await;
    c.send("STARTTLS now please").await;
    let refused = c.reply().await;
    assert!(refused.starts_with("501"), "{refused}");
    c.send("EHLO arg-check").await;
    assert!(
        c.reply_multi().await.contains("250-STARTTLS"),
        "the offer must survive the syntax error"
    );
}

/// A further STARTTLS on an established TLS session is refused (RFC 3207
/// §4.2) but leaves the session alive.
#[tokio::test]
async fn a_second_starttls_over_tls_is_refused() {
    let (server, root, _dir) = start_tls("twice").await;
    let mut tls = starttls(server.smtp_addr, "localhost", root).await;
    tls.send("STARTTLS").await;
    let refused = tls.reply_raw().await;
    assert!(refused.starts_with("554"), "{refused}");
    assert!(refused.contains("already active"), "{refused}");
    tls.send("NOOP").await;
    assert!(tls.reply_raw().await.starts_with("250"));
}

/// A client that refuses the handshake — untrusted root, or the right cert
/// under the wrong name — ends the session; the server keeps serving.
#[tokio::test]
async fn a_client_that_refuses_the_cert_ends_the_session_and_the_server_serves_on() {
    let (server, root, _dir) = start_tls("refuse").await;

    // An unrelated self-signed root: the client rejects the presented cert.
    let impostor = TlsConfig::generate_self_signed("impostor.example").unwrap();
    let impostor_root = rustls_pemfile::certs(&mut impostor.cert_pem.as_bytes())
        .next()
        .unwrap()
        .unwrap();
    assert_starttls_handshake_fails(server.smtp_addr, "localhost", impostor_root).await;

    // The right cert under the wrong name: the SAN check must bite.
    assert_starttls_handshake_fails(server.smtp_addr, "wrong.example", root).await;

    // Unharmed: the next client speaks plaintext as usual.
    let mut c = SmtpConn::connect(server.smtp_addr).await;
    c.send("NOOP").await;
    assert!(c.reply().await.starts_with("250"));
}

/// A client that stalls the handshake — greeted, `220 Ready` read, then
/// silent, never a ClientHello — used to tie its session task up
/// indefinitely (review finding from run
/// starttls-self-signed-cert-genera-ef7796f0, src/smtp.rs:284). The
/// deadline must close the stalled upgrade while the server keeps serving:
/// a fresh plaintext session answers, and the acceptor still completes a
/// real upgrade for a well-behaved client.
#[tokio::test]
async fn a_stalled_starttls_handshake_is_dropped_within_the_deadline() {
    let deadline = Duration::from_millis(750);
    let (server, root, _dir) = start_tls_cfg(
        "stall",
        SmtpConfig {
            tls_handshake_timeout: deadline,
            ..Default::default()
        },
    )
    .await;

    // Connect, EHLO, STARTTLS, 220 — then stall. No ClientHello ever.
    let mut stalled = stalled_starttls(server.smtp_addr).await;

    // The deadline is a grace period, not an instant close: a probe well
    // inside it still finds the handshake in flight.
    let mut probe = [0u8; 8];
    let early = tokio::time::timeout(deadline / 5, stalled.read(&mut probe)).await;
    assert!(
        early.is_err(),
        "the stalled handshake was dropped before the deadline: {early:?}"
    );

    // Then it must close, in bounded time — against the unbounded
    // handshake this test was written for, this read hangs forever.
    let mut buf = [0u8; 8];
    let started = std::time::Instant::now();
    let n = tokio::time::timeout(deadline * 5, stalled.read(&mut buf))
        .await
        .expect("the stalled handshake was never closed — the session task is still tied up")
        .expect("the stalled handshake closed with an error, not cleanly");
    assert_eq!(n, 0, "expected a clean EOF, got {n} bytes");
    assert!(
        started.elapsed() < deadline * 2,
        "the close took {:?}, nowhere near the {deadline:?} deadline",
        started.elapsed()
    );

    // The listener is unharmed: plaintext keeps serving, and a later
    // upgrade still completes end to end.
    let mut c = SmtpConn::connect(server.smtp_addr).await;
    c.send("NOOP").await;
    assert!(
        c.reply().await.starts_with("250"),
        "the listener must keep serving after a reaped handshake"
    );

    let mut tls = starttls(server.smtp_addr, "localhost", root).await;
    tls.send("NOOP").await;
    assert!(
        tls.reply_raw().await.starts_with("250"),
        "a later STARTTLS upgrade must still complete"
    );
}

/// A malformed PEM, a missing file, or half a pair must refuse to serve at
/// all — the TLS check runs before anything binds.
#[tokio::test]
async fn a_broken_tls_pair_refuses_to_serve() {
    let dir = std::env::temp_dir().join(format!(
        "swarmail-starttls-broken-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (cert, key) = TlsConfig::generate_self_signed("localhost")
        .unwrap()
        .write_to_dir(&dir)
        .unwrap();

    // Malformed PEM content.
    let bad = dir.join("bad.pem");
    std::fs::write(&bad, "this is not pem").unwrap();
    let mut cfg = base_cfg();
    cfg.tls_cert = Some(bad.clone());
    cfg.tls_key = Some(bad.clone());
    let err = match swarmail::run_on(&cfg).await {
        Ok(_) => panic!("a malformed PEM must refuse to serve"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("tls-cert"), "{err}");

    // A missing file: named, not mysterious.
    let mut cfg = base_cfg();
    cfg.tls_cert = Some(PathBuf::from("/nonexistent/swarmail/cert.pem"));
    cfg.tls_key = Some(key.clone());
    let err = match swarmail::run_on(&cfg).await {
        Ok(_) => panic!("a missing cert file must refuse to serve"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("tls-cert"), "{err}");

    // Half a pair is no pair.
    let mut cfg = base_cfg();
    cfg.tls_cert = Some(cert.clone());
    let err = match swarmail::run_on(&cfg).await {
        Ok(_) => panic!("half a pair must refuse to serve"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("together"), "{err}");
}

/// Writing the key into a dir that already holds one exercises the
/// pre-existing-file tighten arm (`write_private_key`'s metadata hit); the
/// mode is 0600 after either arm, by the post-write pin.
#[test]
fn gen_cert_over_an_existing_key_stays_owner_only() {
    let pair = swarmail::tls::TlsConfig::generate_self_signed("localhost").unwrap();
    let dir = std::env::temp_dir().join(format!(
        "swarmail-overwrite-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (cert, key) = pair.write_to_dir(&dir).unwrap();
    // Simulate an older, wider key: open the door before the second write.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o666)).unwrap();
    }
    let (cert2, key2) = pair.write_to_dir(&dir).unwrap();
    assert_eq!(cert, cert2);
    assert_eq!(key, key2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&key2).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the widened key is re-tightened: {mode:o}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A directory sitting at the key path: metadata succeeds but `is_file` is
/// false, so the pre-existing tighten is skipped and the open itself refuses.
#[test]
fn gen_cert_refuses_when_the_key_path_is_a_directory() {
    let pair = swarmail::tls::TlsConfig::generate_self_signed("localhost").unwrap();
    let dir = std::env::temp_dir().join(format!(
        "swarmail-dirpath-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("key.pem")).unwrap();
    let err = pair.write_to_dir(&dir).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("directory")
            || err.kind() == std::io::ErrorKind::IsADirectory,
        "expected a directory error, got: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A dangling symlink at the key path: metadata fails (the Err arm of the
/// pre-existing tighten), and the create-through-symlink recovery still
/// lands the key owner-only at the target.
#[cfg(unix)]
#[test]
fn gen_cert_writes_through_a_dangling_symlink_key() {
    use std::os::unix::fs::PermissionsExt;
    let pair = swarmail::tls::TlsConfig::generate_self_signed("localhost").unwrap();
    let dir = std::env::temp_dir().join(format!(
        "swarmail-symlink-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(dir.join("nowhere"), dir.join("key.pem")).unwrap();
    let (_, key) = pair.write_to_dir(&dir).unwrap();
    let mode = std::fs::metadata(&key).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "the key through the dangling symlink: {mode:o}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A domain rcgen cannot turn into a SAN refuses the mint with the mapped
/// message naming the domain — not a bare rcgen error.
#[test]
fn gen_cert_refuses_an_unmappable_domain_with_a_named_error() {
    // rcgen only mints IA5String SANs: a non-ASCII name refuses here, and
    // the mapped error names the refused domain rather than rcgen's raw one.
    let err = swarmail::tls::TlsConfig::generate_self_signed("n\u{f6}t.io").unwrap_err();
    assert!(
        err.to_string()
            .starts_with("self-signed certificate for n\u{f6}t.io:"),
        "the error names the refused domain: {err}"
    );
}
