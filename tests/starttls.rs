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
//! - Implicit TLS (decision 0116, `SmtpConfig::implicit_tls`): the same
//!   handshake runs on connect, before the greeting. AUTH-named delivery,
//!   recipient routing, `await`, the stalled-handshake deadline, the
//!   plaintext-client refusal, chaos and the cap refusal are each proved
//!   over a real rustls client against a live server.

mod common;

use common::{
    SmtpConn, TlsSmtp, assert_starttls_handshake_fails, http_bytes, http_json, implicit_tls,
    implicit_tls_handshake_fails, stalled_starttls, start, start_with, starttls, starttls_after,
    starttls_discarding_pipelined, wait_for_metric_row,
};
use std::path::PathBuf;
use std::time::Duration;
use swarmail::RunningServer;
use swarmail::config::Config;
use swarmail::smtp::SmtpConfig;
use swarmail::tls::TlsConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::pki_types::CertificateDer;

/// A server serving STARTTLS from a real PEM pair on disk, plus the cert
/// the client must trust (the served one) and the scratch dir.
async fn start_tls(name: &str) -> (RunningServer, CertificateDer<'static>, tempfile::TempDir) {
    start_tls_cfg(name, SmtpConfig::default()).await
}

/// [`start_tls`] with an explicit SMTP posture — the handshake deadline
/// knob among the rest of the `SmtpConfig` fields.
async fn start_tls_cfg(
    name: &str,
    smtp: SmtpConfig,
) -> (RunningServer, CertificateDer<'static>, tempfile::TempDir) {
    let pair = TlsConfig::generate_self_signed("localhost").unwrap();
    // The pair's scratch dir is a drop guard (issue #101): removed on every
    // path out, panic or pass — a green suite leaves no `swarmail-*` litter
    // in `$TMPDIR`. The guard rides the tuple so it outlives the server.
    let dir = tempfile::TempDir::with_prefix(format!("swarmail-starttls-{name}-")).unwrap();
    let dir_path = dir.path().to_path_buf();
    let (cert, key) = pair.write_to_dir(&dir_path).unwrap();
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
        mailslurper_listen: None,
        max_per_inbox: 0,
        data_file: None,
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
        pop3_idle_timeout: swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
        link_reachability: swarmail::deliverability::LinkReachability::Off,
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

/// A successful STARTTLS begins a new epoch (RFC 3207 §4.2, decision 0034
/// §5, AC11): the plaintext AUTH and the open envelope are both forgotten,
/// so the TLS side needs a fresh MAIL, may — and must — authenticate again,
/// and routes to the identity it chose there. One success per epoch holds on
/// the TLS side too.
#[tokio::test]
async fn starttls_resets_identity_and_envelope_for_a_fresh_epoch() {
    let (server, root, _dir) = start_tls("epoch").await;
    let b64 = |raw: &str| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw);
    let plain_auth = format!("AUTH PLAIN {}", b64("\u{0}plainbox\u{0}pw"));
    let mut tls = starttls_after(
        server.smtp_addr,
        "localhost",
        root,
        &[
            (plain_auth.as_str(), "235"),
            ("MAIL FROM:<pre@x.io>", "250"),
            ("RCPT TO:<pre@y.io>", "250"),
        ],
    )
    .await;

    tls.send("DATA").await;
    assert_eq!(
        tls.reply_raw().await,
        "503 5.5.1 Error: need MAIL command",
        "the plaintext envelope did not survive the upgrade"
    );
    tls.send(&format!("AUTH PLAIN {}", b64("\u{0}tlsepoch\u{0}pw")))
        .await;
    assert_eq!(tls.reply_raw().await, "235 2.7.0 Accepted");
    tls.send(&format!("AUTH PLAIN {}", b64("\u{0}other\u{0}pw")))
        .await;
    assert_eq!(
        tls.reply_raw().await,
        "503 5.5.1 Error: already authenticated"
    );
    tls.send("MAIL FROM:<post@x.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    tls.send("RCPT TO:<post@y.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    let stored = tls.data("Subject: epoch\r\n\r\nbody").await;
    assert!(stored.starts_with("250"), "{stored}");

    let (code, body) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/tlsepoch/messages",
        None,
    )
    .await;
    assert_eq!(code, 200);
    let emails = body["emails"].as_array().unwrap();
    assert_eq!(emails.len(), 1, "{body}");
    assert_eq!(emails[0]["from"]["address"], "post@x.io");
    assert_eq!(emails[0]["recipients"], serde_json::json!(["post@y.io"]));
    for stale in ["plainbox", "other"] {
        let (_, count) = http_json(
            server.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{stale}/count"),
            None,
        )
        .await;
        assert_eq!(count["count"], 0, "{stale}");
    }
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
    // The scratch dir is a drop guard (issue #101): removed on every path
    // out, panic or pass.
    let dir = tempfile::TempDir::with_prefix("swarmail-starttls-broken-").unwrap();
    let dir_path = dir.path().to_path_buf();
    let (cert, key) = TlsConfig::generate_self_signed("localhost")
        .unwrap()
        .write_to_dir(&dir_path)
        .unwrap();

    // Malformed PEM content.
    let bad = dir_path.join("bad.pem");
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
    // A drop guard, not a best-effort remove (issue #101): the dir is gone
    // on every path out, panic or pass.
    let dir = tempfile::TempDir::with_prefix("swarmail-overwrite-").unwrap();
    let dir_path = dir.path().to_path_buf();
    let (cert, key) = pair.write_to_dir(&dir_path).unwrap();
    // Simulate an older, wider key: open the door before the second write.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o666)).unwrap();
    }
    let (cert2, key2) = pair.write_to_dir(&dir_path).unwrap();
    assert_eq!(cert, cert2);
    assert_eq!(key, key2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&key2).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the widened key is re-tightened: {mode:o}");
    }
}

/// A directory sitting at the key path: metadata succeeds but `is_file` is
/// false, so the pre-existing tighten is skipped and the open itself refuses.
#[test]
fn gen_cert_refuses_when_the_key_path_is_a_directory() {
    let pair = swarmail::tls::TlsConfig::generate_self_signed("localhost").unwrap();
    // A drop guard, not a best-effort remove (issue #101).
    let dir = tempfile::TempDir::with_prefix("swarmail-dirpath-").unwrap();
    std::fs::create_dir_all(dir.path().join("key.pem")).unwrap();
    let err = pair.write_to_dir(dir.path()).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("directory")
            || err.kind() == std::io::ErrorKind::IsADirectory,
        "expected a directory error, got: {err}"
    );
}

/// A dangling symlink at the key path: metadata fails (the Err arm of the
/// pre-existing tighten), and the create-through-symlink recovery still
/// lands the key owner-only at the target.
#[cfg(unix)]
#[test]
fn gen_cert_writes_through_a_dangling_symlink_key() {
    use std::os::unix::fs::PermissionsExt;
    let pair = swarmail::tls::TlsConfig::generate_self_signed("localhost").unwrap();
    // A drop guard, not a best-effort remove (issue #101).
    let dir = tempfile::TempDir::with_prefix("swarmail-symlink-").unwrap();
    std::os::unix::fs::symlink(dir.path().join("nowhere"), dir.path().join("key.pem")).unwrap();
    let (_, key) = pair.write_to_dir(dir.path()).unwrap();
    let mode = std::fs::metadata(&key).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "the key through the dangling symlink: {mode:o}"
    );
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

// ------------------------------------------------------------------------
// Implicit TLS (decision 0116)
// ------------------------------------------------------------------------

/// A server whose SMTP listener speaks implicit TLS from the first byte.
async fn start_implicit(
    name: &str,
    smtp: SmtpConfig,
) -> (RunningServer, CertificateDer<'static>, tempfile::TempDir) {
    start_tls_cfg(
        name,
        SmtpConfig {
            implicit_tls: true,
            ..smtp
        },
    )
    .await
}

fn b64(raw: &str) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw)
}

/// AUTH PLAIN as `inbox` on an established session; asserts the `235`.
async fn auth_as(tls: &mut TlsSmtp, inbox: &str) {
    tls.send(&format!(
        "AUTH PLAIN {}",
        b64(&format!("\0{inbox}\0whatever"))
    ))
    .await;
    let reply = tls.reply_raw().await;
    assert!(reply.starts_with("235"), "{reply}");
}

/// MAIL, RCPT, DATA with `body` (already CRLF-lined, no terminator), the
/// final reply returned. Octets, not text: the payload may be 8-bit.
async fn deliver_bytes(tls: &mut TlsSmtp, rcpt: &str, body: &[u8]) -> String {
    tls.send("MAIL FROM:<sender@x.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"), "MAIL");
    tls.send(&format!("RCPT TO:<{rcpt}>")).await;
    assert!(tls.reply_raw().await.starts_with("250"), "RCPT");
    tls.send("DATA").await;
    assert!(tls.reply_raw().await.starts_with("354"), "DATA");
    tls.send_bytes(body).await;
    tls.send_bytes(b".\r\n").await;
    tls.reply_raw().await
}

/// AC1 — a client configured as `smtps://` (Kratos' `legacy_ssl=true`)
/// starts TLS on connect, reads the greeting inside it, authenticates as
/// the inbox it wants and delivers. The `250` still means queryable
/// (0001): the very next read of the API holds exactly that one message,
/// its raw octets intact through the dot-unstuffing (0034), its fields
/// extracted at ingest (0005), in the AUTH-named inbox (0004).
#[tokio::test]
async fn implicit_tls_delivers_to_the_auth_named_inbox_and_is_stored_at_250() {
    let (server, root, _dir) = start_implicit("implicit-deliver", SmtpConfig::default()).await;
    let mut tls = implicit_tls(server.smtp_addr, "localhost", root).await;
    auth_as(&mut tls, "kratos").await;

    let mut stuffed =
        b"From: Kratos <kratos@x.io>\r\nSubject: verify your account\r\n\r\n".to_vec();
    stuffed.extend_from_slice(b"Open https://app.test/verify?t=1 now\r\n");
    stuffed.extend_from_slice(b"bytes \xFF and \x00 survive\r\n");
    stuffed.extend_from_slice(b"..a line that began with a dot\r\n");
    let mut expected = stuffed.clone();
    let at = expected
        .windows(2)
        .position(|w| w == b"..")
        .expect("the stuffed line");
    expected.remove(at);

    let stored = deliver_bytes(&mut tls, "to@y.io", &stuffed).await;
    assert!(stored.starts_with("250"), "{stored}");

    // No wait, no retry: the 250 was the commit.
    let (code, body) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/kratos/messages",
        None,
    )
    .await;
    assert_eq!(code, 200);
    let emails = body["emails"].as_array().unwrap();
    assert_eq!(emails.len(), 1, "{body}");
    assert_eq!(emails[0]["subject"], "verify your account");
    assert_eq!(emails[0]["from"]["address"], "kratos@x.io");
    assert!(
        emails[0]["links"]
            .to_string()
            .contains("https://app.test/verify?t=1"),
        "{}",
        emails[0]
    );
    let id = emails[0]["id"].as_str().unwrap();
    assert!(stored.contains(id), "the 250 names the stored id: {stored}");
    let (st, raw) = http_bytes(server.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(st, 200);
    assert_eq!(
        raw, expected,
        "the sent payload, dot-unstuffed, octet for octet"
    );
    let (_, other) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(other["emails"].as_array().unwrap().len(), 0);

    tls.send("QUIT").await;
    assert!(tls.reply_raw().await.starts_with("221"));
}

/// AC2 — inside an already-TLS session STARTTLS is neither advertised nor
/// accepted (RFC 3207 §4.2): EHLO lists exactly what a plaintext listener
/// lists, `STARTTLS` is `554`, and the refusal changes nothing — the AUTH
/// identity survives (a second AUTH is `503`, 0034 §5) and the next
/// delivery still lands in the AUTH-named inbox.
#[tokio::test]
async fn starttls_is_neither_advertised_nor_accepted_inside_implicit_tls() {
    let (server, root, _dir) = start_implicit("implicit-nostarttls", SmtpConfig::default()).await;
    let mut tls = implicit_tls(server.smtp_addr, "localhost", root).await;
    tls.send("EHLO inside-tls").await;
    assert_eq!(
        tls.reply_multi().await,
        "250-swarmail.local\n250-PIPELINING\n250-8BITMIME\n250-SMTPUTF8\n\
         250-SIZE 52428800\n250-AUTH PLAIN LOGIN\n250 OK",
        "the EHLO body is the plaintext listener's, STARTTLS line absent"
    );
    auth_as(&mut tls, "stay").await;
    tls.send("STARTTLS").await;
    assert_eq!(
        tls.reply_raw().await,
        "554 5.5.1 TLS already active",
        "STARTTLS inside TLS is refused"
    );
    tls.send(&format!("AUTH PLAIN {}", b64("\0again\0x"))).await;
    let again = tls.reply_raw().await;
    assert!(again.starts_with("503"), "one AUTH per epoch: {again}");

    let stored = deliver_bytes(
        &mut tls,
        "to@y.io",
        b"Subject: kept identity\r\n\r\nbody\r\n",
    )
    .await;
    assert!(stored.starts_with("250"), "{stored}");
    let (_, body) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/stay/messages",
        None,
    )
    .await;
    assert_eq!(body["emails"][0]["subject"], "kept identity");
}

/// AC3 — recipient routing (0112) is transport-blind: with a route domain,
/// a recipient `<label>.<domain>` outranks the AUTH name over implicit TLS.
#[tokio::test]
async fn implicit_tls_routes_by_recipient_ahead_of_auth() {
    let (server, root, _dir) = start_implicit(
        "implicit-route",
        SmtpConfig {
            route_domain: Some("swarmail.test".into()),
            ..Default::default()
        },
    )
    .await;
    let mut tls = implicit_tls(server.smtp_addr, "localhost", root).await;
    auth_as(&mut tls, "a").await;
    let stored = deliver_bytes(
        &mut tls,
        "x@routed.swarmail.test",
        b"Subject: routed\r\n\r\nbody\r\n",
    )
    .await;
    assert!(stored.starts_with("250"), "{stored}");
    let (_, routed) = http_json(
        server.http_addr,
        "GET",
        "/api/v1/inboxes/routed/messages",
        None,
    )
    .await;
    assert_eq!(routed["emails"][0]["subject"], "routed");
    let (_, auth) = http_json(server.http_addr, "GET", "/api/v1/inboxes/a/messages", None).await;
    assert_eq!(auth["emails"].as_array().unwrap().len(), 0, "{auth}");
}

/// AC4 — `await` subscribes before it checks (0002): a long-poll issued
/// first, and observed parked on the metric row (no sleep), returns the
/// mail an implicit-TLS delivery then makes. The await's own deadline
/// bounds the test.
#[tokio::test]
async fn an_await_issued_first_sees_an_implicit_tls_delivery() {
    let (server, root, _dir) = start_implicit("implicit-await", SmtpConfig::default()).await;
    let http = server.http_addr;
    let waiting = tokio::spawn(async move {
        http_json(
            http,
            "GET",
            "/api/v1/inboxes/parked/await?count=1&timeout_ms=30000",
            None,
        )
        .await
    });
    wait_for_metric_row(http, "swarmail_awaits_active{inbox=\"parked\"}", 1).await;

    let mut tls = implicit_tls(server.smtp_addr, "localhost", root).await;
    auth_as(&mut tls, "parked").await;
    let stored = deliver_bytes(
        &mut tls,
        "to@y.io",
        b"Subject: woke the await\r\n\r\nbody\r\n",
    )
    .await;
    assert!(stored.starts_with("250"), "{stored}");

    let (st, body) = waiting.await.unwrap();
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1);
    assert_eq!(body["emails"][0]["subject"], "woke the await");
}

/// AC5 — a client that connects and never sends a ClientHello is dropped
/// at `tls_handshake_timeout`: not before (a probe well inside the deadline
/// is still pending), then a clean EOF having received no byte at all, and
/// the listener serves on.
#[tokio::test]
async fn a_stalled_implicit_handshake_is_dropped_at_the_timeout() {
    let deadline = Duration::from_millis(750);
    let (server, root, _dir) = start_implicit(
        "implicit-stall",
        SmtpConfig {
            tls_handshake_timeout: deadline,
            ..Default::default()
        },
    )
    .await;
    let mut stalled = tokio::net::TcpStream::connect(server.smtp_addr)
        .await
        .unwrap();

    let mut probe = [0u8; 8];
    let early = tokio::time::timeout(deadline / 5, stalled.read(&mut probe)).await;
    assert!(
        early.is_err(),
        "the stalled handshake was dropped before the deadline: {early:?}"
    );

    let mut buf = [0u8; 8];
    let started = std::time::Instant::now();
    let n = tokio::time::timeout(deadline * 5, stalled.read(&mut buf))
        .await
        .expect("the stalled handshake was never closed — the session task is still tied up")
        .expect("the stalled handshake closed with an error, not cleanly");
    assert_eq!(n, 0, "no SMTP byte, not even a greeting: got {n} bytes");
    assert!(
        started.elapsed() < deadline * 2,
        "the close took {:?}, nowhere near the {deadline:?} deadline",
        started.elapsed()
    );

    let mut tls = implicit_tls(server.smtp_addr, "localhost", root).await;
    tls.send("NOOP").await;
    assert!(tls.reply_raw().await.starts_with("250"));
}

/// AC6 — a plaintext client on the TLS port is refused: the bytes it sent
/// are not a ClientHello, rustls ends the handshake long before the
/// (deliberately long) deadline, and nothing readable as SMTP comes back —
/// no greeting, at most a TLS alert record (first byte 0x15). The listener
/// serves on.
#[tokio::test]
async fn a_plaintext_client_on_the_implicit_tls_port_gets_no_smtp_reply() {
    let (server, root, _dir) = start_implicit(
        "implicit-plaintext",
        SmtpConfig {
            tls_handshake_timeout: Duration::from_secs(60),
            ..Default::default()
        },
    )
    .await;
    let mut raw = tokio::net::TcpStream::connect(server.smtp_addr)
        .await
        .unwrap();
    raw.write_all(b"EHLO x\r\n").await.unwrap();
    let mut got = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), raw.read_to_end(&mut got))
        .await
        .expect("the plaintext client was not closed — rustls never refused it")
        .ok(); // a reset after the alert is as closed as an EOF
    assert!(
        got.is_empty() || got[0] == 0x15,
        "neither empty nor a TLS alert: {got:?}"
    );
    assert!(
        !got.windows(3).any(|w| w == b"220"),
        "an SMTP greeting leaked in cleartext: {got:?}"
    );

    let mut tls = implicit_tls(server.smtp_addr, "localhost", root).await;
    auth_as(&mut tls, "after-plaintext").await;
    let stored = deliver_bytes(&mut tls, "to@y.io", b"Subject: after\r\n\r\nbody\r\n").await;
    assert!(stored.starts_with("250"), "{stored}");
}

/// AC7 — a `Connect` chaos rule (probability 100, 0020 rule 5) is checked
/// after the handshake, so its injected error line reaches an implicit
/// client as decrypted TLS application data and never as cleartext on the
/// socket.
#[tokio::test]
async fn connect_chaos_reaches_an_implicit_client_inside_tls() {
    let (server, root, _dir) = start_implicit("implicit-chaos", SmtpConfig::default()).await;
    let (st, installed) = http_json(
        server.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"connect":{"probability":100,"error":"451 4.3.0 chaos"}}"#),
    )
    .await;
    assert_eq!(st, 200, "{installed}");

    // `implicit_tls` asserts a 220, which chaos replaces: read the line by
    // hand through the helper-built handshake instead.
    let mut chaotic = common::implicit_tls_raw(server.smtp_addr, "localhost", root).await;
    assert_eq!(
        chaotic.reply_raw().await,
        "451 4.3.0 chaos",
        "the injected line travels inside TLS"
    );

    // A client that never speaks TLS never sees the text in cleartext.
    let mut raw = tokio::net::TcpStream::connect(server.smtp_addr)
        .await
        .unwrap();
    raw.write_all(b"EHLO x\r\n").await.unwrap();
    let mut got = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), raw.read_to_end(&mut got))
        .await
        .expect("the raw client was not closed")
        .ok();
    assert!(
        !got.windows(b"451 4.3.0 chaos".len())
            .any(|w| w == b"451 4.3.0 chaos"),
        "the chaos line leaked in cleartext: {got:?}"
    );
}

/// AC8 — the cap refusal is 0027 §3's, byte for byte, in implicit mode: a
/// raw second connection reads the plaintext `421 4.7.0 …` and EOF; a TLS
/// client in the same position fails its handshake rather than hang; once
/// the held session quits the slot is free and an implicit client delivers.
#[tokio::test]
async fn the_session_cap_refusal_is_unchanged_in_implicit_mode() {
    let (server, root, _dir) = start_implicit(
        "implicit-cap",
        SmtpConfig {
            max_sessions: 1,
            ..Default::default()
        },
    )
    .await;
    let mut held = implicit_tls(server.smtp_addr, "localhost", root.clone()).await;

    let mut refused = SmtpConn::connect_raw(server.smtp_addr).await;
    assert_eq!(
        refused.reply().await,
        "421 4.7.0 Too many connections, closing transmission channel"
    );
    assert_eq!(refused.read_to_eof().await, "", "the refusal is all of it");

    implicit_tls_handshake_fails(server.smtp_addr, "localhost", root.clone()).await;

    held.send("QUIT").await;
    assert!(held.reply_raw().await.starts_with("221"));
    held.read_to_eof().await;

    // The freed slot is reaped at admission (0036 §1). A dial that lands
    // before the reap is refused in the clear and closed, so retry until a
    // handshake completes — bounded by a real deadline, never a sleep.
    let started = std::time::Instant::now();
    let mut tls = loop {
        match common::try_implicit_tls(server.smtp_addr, "localhost", root.clone()).await {
            Some(session) => break session,
            None => assert!(
                started.elapsed() < Duration::from_secs(20),
                "the freed slot was never reaped"
            ),
        }
    };
    auth_as(&mut tls, "after-cap").await;
    let stored = deliver_bytes(&mut tls, "to@y.io", b"Subject: after cap\r\n\r\nbody\r\n").await;
    assert!(stored.starts_with("250"), "{stored}");
}

/// AC9 — the mode without a pair is refused before anything binds, with
/// the one message; never served as plaintext.
#[tokio::test]
async fn implicit_tls_without_a_pair_refuses_to_serve() {
    let mut cfg = base_cfg();
    cfg.smtp.implicit_tls = true;
    let err = match swarmail::run_on(&cfg).await {
        Ok(_) => panic!("implicit TLS without a pair must refuse to serve"),
        Err(e) => e,
    };
    assert_eq!(
        err.to_string(),
        "--smtp-implicit-tls requires --tls-cert and --tls-key"
    );
}
