//! SMTP protocol edges: every verb, every refusal, every parse branch —
//! driven line-by-line over a real socket.

mod common;

use common::{SmtpConn, http_json, smtp_send, start, start_smtp};
use swarmail::smtp::SmtpConfig;

#[tokio::test]
async fn ehlo_advertises_the_size_limit() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO test").await;
    let reply = c.reply_multi().await;
    assert!(reply.contains("250-SIZE 52428800"), "{reply}");
}

#[tokio::test]
async fn starttls_is_refused_while_nothing_tls_is_configured() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("STARTTLS").await;
    assert!(c.reply().await.starts_with("454"));
}

#[tokio::test]
async fn auth_can_be_disabled_by_config() {
    let srv = start_smtp(SmtpConfig {
        accept_any_auth: false,
        ..Default::default()
    })
    .await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    let reply = c.reply().await;
    assert!(!reply.contains("AUTH"), "{reply}");
    c.send("AUTH PLAIN AGkAeA==").await;
    assert!(c.reply().await.starts_with("502"));
}

#[tokio::test]
async fn auth_plain_continuation_form_and_login_form() {
    let srv = start().await;

    // PLAIN without an argument → 334 continuation, then the b64 line.
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("AUTH PLAIN").await;
    assert!(c.reply().await.starts_with("334"));
    let b64 = base64_of("\u{0}plainbox\u{0}x");
    c.send(&b64).await;
    assert!(c.reply().await.starts_with("235"));

    // LOGIN: username + password prompts.
    let mut c2 = SmtpConn::connect(srv.smtp_addr).await;
    c2.send("EHLO t").await;
    c2.reply().await;
    c2.send("AUTH LOGIN").await;
    assert!(c2.reply().await.starts_with("334 VXNlcm5hbWU6"));
    c2.send(&base64_of("loginbox")).await;
    assert!(c2.reply().await.starts_with("334 UGFzc3dvcmQ6"));
    c2.send(&base64_of("pw")).await;
    assert!(c2.reply().await.starts_with("235"));

    // The LOGIN username named the inbox: deliver on that same connection.
    c2.send("MAIL FROM:<f@x.io>").await;
    assert!(c2.reply().await.starts_with("250"));
    c2.send("RCPT TO:<r@x.io>").await;
    assert!(c2.reply().await.starts_with("250"));
    assert!(
        c2.data("Subject: to loginbox\r\n\r\nhi")
            .await
            .starts_with("250")
    );
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/loginbox/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}

#[tokio::test]
async fn auth_with_unknown_mechanism_or_bad_b64_is_handled() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;

    c.send("AUTH CRAM-MD5").await;
    assert!(c.reply().await.starts_with("504"));

    // Undecodable PLAIN: accepted (accept-any) but the inbox stays default.
    c.send("AUTH PLAIN !!!not-b64!!!").await;
    assert!(c.reply().await.starts_with("235"));
    c.send("MAIL FROM:<a@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<b@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    let reply = c.data("Subject: b64fail\r\n\r\nx").await;
    assert!(reply.starts_with("250"), "{reply}");
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}

#[tokio::test]
async fn sequencing_errors_need_mail_then_rcpt() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;

    c.send("RCPT TO:<a@x.io>").await;
    assert!(c.reply().await.starts_with("503"));

    c.send("MAIL FROM:<a@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("503"));
}

#[tokio::test]
async fn rset_noop_vrfy_quit_and_unknown_verbs() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;

    c.send("NOOP").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("VRFY someone").await;
    assert!(c.reply().await.starts_with("252"));
    c.send("BOGUS whatever").await;
    assert!(c.reply().await.starts_with("502"));

    c.send("MAIL FROM:<a@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<b@x.io>").await;
    c.reply().await;
    c.send("RSET").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("503"));

    c.send("QUIT").await;
    assert!(c.reply().await.starts_with("221"));
}

#[tokio::test]
async fn mail_from_without_angle_brackets_still_parses() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:bare@x.io").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<r@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(c.data("Subject: bare\r\n\r\nbody").await.starts_with("250"));

    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(body["emails"][0]["from"]["address"], "bare@x.io");
}

#[tokio::test]
async fn oversized_data_gets_552_and_is_not_stored() {
    let srv = start_smtp(SmtpConfig {
        max_message_size: 64,
        ..Default::default()
    })
    .await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<big@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<r@x.io>").await;
    c.reply().await;
    let body = format!("Subject: big\r\n\r\n{}", "x".repeat(200));
    let reply = c.data(&body).await;
    assert!(reply.starts_with("552"), "{reply}");

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn dot_stuffing_is_unstuffed_and_peer_vanish_is_silent() {
    let srv = start().await;
    // Dot-stuffing: "..leading" must store ".leading".
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<d@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<r@x.io>").await;
    c.reply().await;
    assert!(
        c.data("Subject: dots\r\n\r\n..leading dot line")
            .await
            .starts_with("250")
    );
    let (st, raw) = {
        let (_, body) = http_json(
            srv.http_addr,
            "GET",
            "/api/v1/inboxes/default/messages",
            None,
        )
        .await;
        let id = body["emails"][0]["id"].as_str().unwrap().to_string();
        let (st, raw) =
            common::http_get_text(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
        (st, raw)
    };
    assert_eq!(st, 200);
    assert!(
        raw.contains(".leading dot line"),
        "dot-unstuffed body missing; raw: {raw:?}"
    );

    // A peer that vanishes mid-DATA stores nothing and hurts no one.
    let mut c2 = SmtpConn::connect(srv.smtp_addr).await;
    c2.send("EHLO t").await;
    c2.reply().await;
    c2.send("MAIL FROM:<v@x.io>").await;
    c2.reply().await;
    c2.send("RCPT TO:<r@x.io>").await;
    c2.reply().await;
    c2.send("DATA").await;
    c2.reply().await;
    drop(c2);
    // The server must still serve: a fresh round trip works, and the
    // vanished session's partial DATA stored nothing (only the dot mail
    // and this one are present).
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "after vanish", "x")
        .await
        .unwrap();
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 2);
}

#[tokio::test]
async fn group_addresses_and_headerless_from_fall_back_to_envelope() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<envelope@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<r@x.io>").await;
    c.reply().await;
    // A group-syntax From header parses to no single address; the envelope
    // MAIL FROM becomes the sender. Group recipients are flattened.
    let body =
        "From: undisclosed-group:;\r\nTo: The Group: a@x.io, b@x.io;\r\nSubject: groups\r\n\r\nx";
    assert!(c.data(body).await.starts_with("250"));

    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let email = &body["emails"][0];
    assert_eq!(email["from"]["address"], "envelope@x.io");
    let tos: Vec<&str> = email["to"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["address"].as_str().unwrap())
        .collect();
    assert!(
        tos.contains(&"a@x.io") && tos.contains(&"b@x.io"),
        "{tos:?}"
    );
}

#[tokio::test]
async fn name_only_group_members_are_dropped_not_mangled() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<f@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<r@x.io>").await;
    c.reply().await;
    // "Bob" is a display name with no address; it must not become a bogus
    // recipient.
    assert!(
        c.data("To: g: Bob, real@x.io;\r\nSubject: nameless\r\n\r\nx")
            .await
            .starts_with("250")
    );
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let tos: Vec<&str> = body["emails"][0]["to"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["address"].as_str().unwrap())
        .collect();
    assert_eq!(tos, vec!["real@x.io"]);
}

#[tokio::test]
async fn chaos_on_connect_mailfrom_and_data_with_and_without_error() {
    let srv = start().await;

    // connect with a custom error line: the banner is replaced and the
    // connection closes.
    let (st, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"connect": {"probability": 100, "error": "451 go away", "delay_ms": 10}}"#),
    )
    .await;
    assert_eq!(st, 200);
    let mut c = SmtpConn::connect_raw(srv.smtp_addr).await;
    assert_eq!(c.reply().await, "451 go away");
    drop(c);
    http_json(srv.http_addr, "DELETE", "/api/v1/chaos", None).await;

    // mail_from chaos with a custom error.
    let (st, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"mail_from": {"probability": 100, "error": "450 busy"}}"#),
    )
    .await;
    assert_eq!(st, 200);
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<a@x.io>").await;
    assert_eq!(c.reply().await, "450 busy");
    drop(c);
    http_json(srv.http_addr, "DELETE", "/api/v1/chaos", None).await;

    // data chaos with only a delay: the mail is still stored after the wait.
    let (st, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100, "delay_ms": 20}}"#),
    )
    .await;
    assert_eq!(st, 200);
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "delayed", "x")
        .await
        .unwrap();
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
    http_json(srv.http_addr, "DELETE", "/api/v1/chaos", None).await;

    // data chaos with an error: the mail is NOT stored — that is the chaos.
    let (st, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"data": {"probability": 100, "error": "554 rejected"}}"#),
    )
    .await;
    assert_eq!(st, 200);
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<f@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<r@x.io>").await;
    c.reply().await;
    assert_eq!(c.data("Subject: nope\r\n\r\nx").await, "554 rejected");
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1); // only the delayed one
}

#[tokio::test]
async fn connect_chaos_with_delay_only_still_drops_the_session() {
    let srv = start().await;
    let (st, _) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/chaos",
        Some(r#"{"connect": {"probability": 100, "delay_ms": 5}}"#),
    )
    .await;
    assert_eq!(st, 200);
    // No error line configured: the connection just closes silently.
    let mut c = SmtpConn::connect_raw(srv.smtp_addr).await;
    let greeted = tokio::time::timeout(std::time::Duration::from_millis(500), c.reply()).await;
    assert!(greeted.is_err() || !greeted.unwrap().starts_with("220"));
}

fn base64_of(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s)
}

/// The angle-bracket-less MAIL FROM falls back to the `key: value` split
/// (`arg_address`'s `_` arm), and a `<` that appears after `>` does too.
#[tokio::test]
async fn mail_from_without_brackets_uses_the_key_value_fallback() {
    let srv = start().await;

    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:bob@x.io").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<r@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(
        c.data("Subject: no brackets\r\n\r\nx")
            .await
            .starts_with("250")
    );

    // A `<` after the `>` also misses the bracket arm (b > a is false).
    let mut c2 = SmtpConn::connect(srv.smtp_addr).await;
    c2.send("EHLO t").await;
    c2.reply().await;
    c2.send("MAIL FROM:>weird<").await;
    assert!(c2.reply().await.starts_with("250"));
    c2.send("RCPT TO:<r@x.io>").await;
    assert!(c2.reply().await.starts_with("250"));
    assert!(
        c2.data("Subject: inverted brackets\r\n\r\nx")
            .await
            .starts_with("250")
    );

    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let froms: Vec<String> = body["emails"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["from"]["address"].as_str().unwrap().to_string())
        .collect();
    assert!(froms.contains(&"bob@x.io".to_string()), "got {froms:?}");
    assert!(froms.contains(&">weird<".to_string()), "got {froms:?}");
}

/// AUTH LOGIN with an undecodable username: the inbox stays whatever it was
/// (the `if let Some(user)` arm is skipped) and the handshake still succeeds.
#[tokio::test]
async fn auth_login_with_undecodable_username_stays_default() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("AUTH LOGIN").await;
    assert!(c.reply().await.starts_with("334 VXNlcm5hbWU6"));
    c.send("!!!not-b64!!!").await;
    assert!(c.reply().await.starts_with("334 UGFzc3dvcmQ6"));
    c.send(&base64_of("pw")).await;
    assert!(c.reply().await.starts_with("235"));

    c.send("MAIL FROM:<f@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<defaultlogin@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(
        c.data("Subject: login b64 fail\r\n\r\nx")
            .await
            .starts_with("250")
    );
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}

/// AUTH PLAIN whose authcid is empty: `auth_plain_inbox` returns None and the
/// mail flows to the default inbox (the `user.is_empty()` arm).
#[tokio::test]
async fn auth_plain_with_empty_authcid_stays_default() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("AUTH PLAIN").await;
    assert!(c.reply().await.starts_with("334"));
    c.send(&base64_of("\u{0}\u{0}pw")).await;
    assert!(c.reply().await.starts_with("235"));

    c.send("MAIL FROM:<f@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<emptyauth@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(
        c.data("Subject: empty authcid\r\n\r\nx")
            .await
            .starts_with("250")
    );
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}

/// A peer that sends DATA and then vanishes (clean EOF, zero body bytes) is
/// dropped gracefully: nothing is stored, and the server keeps serving.
#[tokio::test]
async fn data_with_immediate_eof_stores_nothing() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<ghost@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<vanished@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("354"));
    c.shutdown_write().await;

    // The server closes its side; the read half sees the EOF.
    let _eof = c.read_to_end().await;

    // Nothing was stored for the vanished session's inbox.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/vanished/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 0);

    // The server kept serving: a fresh session works end to end.
    let mut fresh = SmtpConn::connect(srv.smtp_addr).await;
    fresh.send("EHLO t").await;
    fresh.reply().await;
    fresh.send("MAIL FROM:<after@x.io>").await;
    assert!(fresh.reply().await.starts_with("250"));
}

/// A DATA body terminated by `.` + bare LF (no CR) is a legal terminator:
/// RFC 5321 §2.3.8 tolerance — the `line == ".\n"` arm of the check.
#[tokio::test]
async fn data_terminated_by_bare_lf_dot_is_accepted() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<lf@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<barelf@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("354"));
    c.send_bytes(b"Subject: bare lf terminator\r\n\r\nbody\r\n.\n")
        .await;
    assert!(c.reply().await.starts_with("250"));

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1, "the bare-LF-terminated mail was stored");
}
