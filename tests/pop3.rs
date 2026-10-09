//! POP3 over the wire: RFC 1939 sessions against a real listener, with mail
//! delivered over real SMTP — the same store the HTTP API serves.

mod common;

use common::{Pop3, http_json, smtp_send, start};
use swarmail::RunningServer;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The raw bytes smtp_send delivers for these arguments — the RETR payload
/// and the octet counts both come from it.
fn raw_of(from: &str, to: &str, subject: &str, body: &str) -> Vec<u8> {
    format!("From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{body}\r\n").into_bytes()
}

fn octets(from: &str, to: &str, subject: &str, body: &str) -> u64 {
    raw_of(from, to, subject, body).len() as u64
}

/// AUTH PLAIN over a raw connection: the identity names the inbox (the RCPT
/// address is only recorded in `recipients`).
async fn smtp_auth(conn: &mut common::SmtpConn, inbox: &str) {
    let plain = format!("\u{0}{inbox}\u{0}whatever");
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    let reply = conn.reply().await;
    assert!(reply.starts_with("235"), "AUTH refused: {reply}");
}

/// Deliver one mail into `inbox` and return its swarmail id.
async fn deliver(s: &RunningServer, inbox: &str, subject: &str, body: &str) -> String {
    smtp_send(
        s.smtp_addr,
        Some(inbox),
        "sender@x.io",
        inbox,
        subject,
        body,
    )
    .await
    .unwrap();
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        &format!("/api/v1/inboxes/{inbox}/messages"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    body["emails"][0]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn capa_is_answered_before_and_after_authentication() {
    let s = start().await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(c.cmd("CAPA").await, "+OK Capability list follows");
    let caps = c.data().await;
    for expected in [
        "USER",
        "TOP",
        "UIDL",
        "PIPELINING",
        "IMPLEMENTATION Swarmail",
    ] {
        assert!(caps.contains(expected), "missing {expected} in {caps}");
    }
    smtp_send(s.smtp_addr, Some("capa"), "sender@x.io", "capa", "t", "b")
        .await
        .unwrap();
    c.login("capa").await;
    assert_eq!(c.cmd("CAPA").await, "+OK Capability list follows");
    let _ = c.data().await;
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn an_unknown_maildrop_is_refused() {
    let s = start().await;
    smtp_send(s.smtp_addr, Some("real"), "sender@x.io", "real", "t", "b")
        .await
        .unwrap();
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(c.cmd("USER nobody-there").await, "-ERR no such mailbox");
    assert_eq!(c.cmd("USER").await, "-ERR missing username");
    // The one that exists is accepted, and PASS takes any password.
    assert_eq!(c.cmd("USER real").await, "+OK name is a valid mailbox");
    assert!(
        c.cmd("PASS hunter2")
            .await
            .starts_with("+OK maildrop has 1 messages"),
        "any password authenticates"
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn transaction_commands_are_gated_on_authentication() {
    let s = start().await;
    smtp_send(s.smtp_addr, Some("gated"), "sender@x.io", "gated", "t", "b")
        .await
        .unwrap();
    let mut c = Pop3::connect(s.pop3_addr).await;
    for cmd in [
        "STAT", "LIST", "UIDL", "RETR 1", "DELE 1", "TOP 1 1", "NOOP", "RSET", "HELP",
    ] {
        assert_eq!(
            c.cmd(cmd).await,
            "-ERR command invalid in AUTHORIZATION state",
            "{cmd} before authentication"
        );
    }
    assert_eq!(c.cmd("PASS secret").await, "-ERR USER required first");
    c.login("gated").await;
    assert_eq!(
        c.cmd("STAT").await,
        format!("+OK 1 {}", octets("sender@x.io", "gated", "t", "b"))
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn the_transaction_phase_rejects_auth_and_unknown_verbs() {
    let s = start().await;
    smtp_send(s.smtp_addr, Some("verbs"), "sender@x.io", "verbs", "t", "b")
        .await
        .unwrap();
    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("verbs").await;
    assert_eq!(c.cmd("USER again").await, "-ERR already authenticated");
    assert_eq!(c.cmd("PASS again").await, "-ERR already authenticated");
    assert_eq!(c.cmd("FROBNICATE").await, "-ERR unknown command");
    assert_eq!(c.cmd("").await, "-ERR unknown command");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn stat_list_uidl_retr_answer_from_the_same_store() {
    let s = start().await;
    let first = deliver(&s, "hog", "One", "first").await;
    let second = deliver(&s, "hog", "Two", "second").await;
    let (n1, n2) = (
        octets("sender@x.io", "hog", "One", "first"),
        octets("sender@x.io", "hog", "Two", "second"),
    );

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("hog").await;

    assert_eq!(c.cmd("STAT").await, format!("+OK 2 {}", n1 + n2));
    assert_eq!(
        c.cmd("LIST").await,
        format!("+OK 2 messages ({} octets)", n1 + n2)
    );
    // Oldest first: the first mail delivered is number 1.
    assert_eq!(c.data().await, format!("1 {n1}\r\n2 {n2}\r\n"));
    assert_eq!(c.cmd("LIST 1").await, format!("+OK 1 {n1}"));
    assert_eq!(c.cmd("LIST 2").await, format!("+OK 2 {n2}"));
    assert_eq!(c.cmd("LIST 0").await, "-ERR no such message");
    assert_eq!(c.cmd("LIST 3").await, "-ERR no such message");
    assert_eq!(c.cmd("LIST many").await, "-ERR invalid message number");

    assert_eq!(c.cmd("UIDL").await, "+OK");
    assert_eq!(c.data().await, format!("1 {first}\r\n2 {second}\r\n"));
    assert_eq!(c.cmd("UIDL 2").await, format!("+OK 2 {second}"));
    assert_eq!(c.cmd("UIDL 9").await, "-ERR no such message");
    assert_eq!(c.cmd("UIDL x").await, "-ERR invalid message number");

    // RETR is byte-exact: the raw DATA payload comes back as delivered.
    assert_eq!(c.cmd("RETR 1").await, format!("+OK {n1} octets"));
    assert_eq!(
        c.data_bytes().await,
        raw_of("sender@x.io", "hog", "One", "first")
    );
    assert_eq!(c.cmd("RETR 2").await, format!("+OK {n2} octets"));
    assert_eq!(
        c.data_bytes().await,
        raw_of("sender@x.io", "hog", "Two", "second")
    );
    assert_eq!(c.cmd("RETR 0").await, "-ERR no such message");
    assert_eq!(c.cmd("RETR 3").await, "-ERR no such message");
    assert_eq!(c.cmd("RETR lots").await, "-ERR invalid message number");

    // The maildrop is a snapshot: NOOP and RSET keep every number alive.
    assert_eq!(c.cmd("NOOP").await, "+OK");
    assert_eq!(
        c.cmd("RSET").await,
        format!("+OK maildrop has 2 messages ({} octets)", n1 + n2)
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    // Nothing was deleted: both mails are still queryable over HTTP.
    for id in [&first, &second] {
        let (status, _) =
            http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
        assert_eq!(status, 200, "{id} must survive a clean session");
    }
}

#[tokio::test]
async fn retr_stuffs_leading_dots_and_normalizes_line_endings() {
    let s = start().await;
    // The SMTP server dot-unstuffs per RFC 5321, so the stored raw body line
    // starts with a single dot — and a bare-LF line arrives without a CR.
    let mut conn = common::SmtpConn::connect(s.smtp_addr).await;
    smtp_auth(&mut conn, "stuffed").await;
    conn.send("MAIL FROM:<sender@x.io>").await;
    conn.reply().await;
    conn.send("RCPT TO:<stuffed>").await;
    conn.reply().await;
    conn.data("..leading-dots\nplain line").await;

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("stuffed").await;
    // The stored raw keeps the bare LF; RETR normalizes on the wire but the
    // octet count is the stored size.
    let expected = (".leading-dots\n".len() + "plain line\r\n".len()) as u64;
    assert_eq!(c.cmd("RETR 1").await, format!("+OK {expected} octets"));
    // Dot-stuffing is undone by the client; the bare LF became CRLF.
    assert_eq!(c.data_bytes().await, b".leading-dots\r\nplain line\r\n");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn dele_marks_until_quit_then_deletes_through_the_store() {
    let s = start().await;
    let first = deliver(&s, "doomed", "One", "first").await;
    let second = deliver(&s, "doomed", "Two", "second").await;
    let n2 = octets("sender@x.io", "doomed", "Two", "second");

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("doomed").await;

    assert_eq!(c.cmd("DELE 1").await, "+OK message 1 deleted");
    assert_eq!(c.cmd("DELE 1").await, "-ERR message already deleted");
    assert_eq!(c.cmd("DELE 0").await, "-ERR no such message");
    assert_eq!(c.cmd("DELE 3").await, "-ERR no such message");
    assert_eq!(c.cmd("DELE soon").await, "-ERR invalid message number");

    // The marks are visible to every listing command, never to the store.
    assert_eq!(c.cmd("STAT").await, format!("+OK 1 {n2}"));
    assert_eq!(c.cmd("LIST").await, format!("+OK 1 messages ({n2} octets)"));
    assert_eq!(c.data().await, format!("2 {n2}\r\n"));
    assert_eq!(c.cmd("UIDL").await, "+OK");
    assert_eq!(c.data().await, format!("2 {second}\r\n"));
    assert_eq!(c.cmd("RETR 1").await, "-ERR message already deleted");

    assert_eq!(c.cmd("QUIT").await, "+OK Swarmail POP3 server signing off");

    // QUIT applied the marks through Store::delete — the HTTP API agrees.
    let (gone, _) = http_json(
        s.http_addr,
        "GET",
        &format!("/api/v1/messages/{first}"),
        None,
    )
    .await;
    assert_eq!(gone, 404);
    let (kept, body) = http_json(
        s.http_addr,
        "GET",
        &format!("/api/v1/messages/{second}"),
        None,
    )
    .await;
    assert_eq!(kept, 200);
    assert_eq!(body["id"], second);
}

#[tokio::test]
async fn rset_unmarks_every_deletion() {
    let s = start().await;
    let first = deliver(&s, "rset", "One", "first").await;
    let second = deliver(&s, "rset", "Two", "second").await;
    let (n1, n2) = (
        octets("sender@x.io", "rset", "One", "first"),
        octets("sender@x.io", "rset", "Two", "second"),
    );

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("rset").await;
    assert!(c.cmd("DELE 1").await.starts_with("+OK"));
    assert!(c.cmd("DELE 2").await.starts_with("+OK"));
    assert_eq!(
        c.cmd("RSET").await,
        format!("+OK maildrop has 2 messages ({} octets)", n1 + n2)
    );
    assert_eq!(c.cmd("RETR 1").await, format!("+OK {n1} octets"));
    let _ = c.data_bytes().await;
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    for id in [&first, &second] {
        let (status, _) =
            http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
        assert_eq!(status, 200, "RSET must unmark {id}");
    }
}

#[tokio::test]
async fn ending_a_session_without_quit_applies_nothing() {
    let s = start().await;
    let first = deliver(&s, "abandoned", "One", "first").await;
    let second = deliver(&s, "abandoned", "Two", "second").await;

    // Mark, then walk away: no QUIT, no update state.
    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("abandoned").await;
    assert!(c.cmd("DELE 1").await.starts_with("+OK"));
    drop(c);

    let mut next = Pop3::connect(s.pop3_addr).await;
    next.login("abandoned").await;
    assert!(next.cmd("STAT").await.starts_with("+OK 2 "));
    assert!(next.cmd("QUIT").await.starts_with("+OK"));

    for id in [&first, &second] {
        let (status, _) =
            http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
        assert_eq!(status, 200, "{id} survives an abandoned session");
    }
}

#[tokio::test]
async fn mail_arriving_mid_session_is_invisible_until_the_next_session() {
    let s = start().await;
    deliver(&s, "snapshot", "One", "first").await;
    let n1 = octets("sender@x.io", "snapshot", "One", "first");

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("snapshot").await;
    assert!(c.cmd("STAT").await.starts_with("+OK 1 "));

    // A new mail lands in the same inbox mid-session.
    smtp_send(
        s.smtp_addr,
        Some("snapshot"),
        "sender@x.io",
        "snapshot",
        "Two",
        "second",
    )
    .await
    .unwrap();

    // The snapshot still holds one message, with stable numbering.
    assert!(c.cmd("STAT").await.starts_with("+OK 1 "));
    assert_eq!(c.cmd("LIST").await, format!("+OK 1 messages ({n1} octets)"));
    assert_eq!(c.data().await.lines().count(), 1);
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    let mut c2 = Pop3::connect(s.pop3_addr).await;
    c2.login("snapshot").await;
    assert!(c2.cmd("STAT").await.starts_with("+OK 2 "));
    assert!(c2.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn mail_cleared_elsewhere_is_no_longer_fetchable() {
    let s = start().await;
    let id = deliver(&s, "vanish", "One", "first").await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("vanish").await;

    // The store is the source of truth: a delete over HTTP (or another
    // POP3 session) is visible to RETR and TOP immediately.
    let (status, _) = http_json(
        s.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(c.cmd("RETR 1").await, "-ERR no such message");
    assert_eq!(c.cmd("TOP 1 0").await, "-ERR no such message");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn top_returns_headers_plus_the_requested_body_lines() {
    let s = start().await;
    deliver(&s, "tops", "One", "first").await;

    // A message with no header/body separator at all.
    let mut conn = common::SmtpConn::connect(s.smtp_addr).await;
    smtp_auth(&mut conn, "tops").await;
    conn.send("MAIL FROM:<sender@x.io>").await;
    conn.reply().await;
    conn.send("RCPT TO:<tops>").await;
    conn.reply().await;
    conn.data("no header block at all").await;
    let headers = "From: sender@x.io\r\nTo: tops\r\nSubject: One\r\n";
    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("tops").await;

    // Oldest first: the seeded mail is number 1, the headerless one number 2.
    assert_eq!(c.cmd("TOP 1 0").await, "+OK");
    assert_eq!(c.data().await, format!("{headers}\r\n"));
    assert_eq!(c.cmd("TOP 1 1").await, "+OK");
    assert_eq!(c.data().await, format!("{headers}\r\nfirst\r\n"));
    assert_eq!(c.cmd("TOP 1 99").await, "+OK");
    assert_eq!(c.data().await, format!("{headers}\r\nfirst\r\n"));
    // No blank line in the raw bytes: everything is treated as headers.
    assert_eq!(c.cmd("TOP 2 5").await, "+OK");
    assert_eq!(c.data().await, "no header block at all\r\n");
    assert_eq!(c.cmd("TOP 1").await, "-ERR invalid line count");
    assert_eq!(c.cmd("TOP 1 lots").await, "-ERR invalid line count");
    assert_eq!(c.cmd("TOP many 1").await, "-ERR invalid message number");
    assert_eq!(c.cmd("TOP 3 1").await, "-ERR no such message");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn oversized_command_lines_are_refused_and_drained() {
    let s = start().await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    // Way past the 4096-byte cap, with a newline: refused, not buffered.
    c.send(&format!("USER {}", "a".repeat(5000))).await;
    assert_eq!(c.reply().await, "-ERR line too long");
    // The over-long line was drained, so the stream is back in sync.
    assert_eq!(c.cmd("CAPA").await, "+OK Capability list follows");
    let _ = c.data().await;

    // Without a newline, the line is still refused — and the session dies
    // quietly rather than buffering a peer that never finishes the line.
    c.send(&"b".repeat(5000)).await;
    drop(c);

    let mut fresh = Pop3::connect(s.pop3_addr).await;
    assert_eq!(fresh.cmd("CAPA").await, "+OK Capability list follows");
    let _ = fresh.data().await;
    assert!(fresh.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn a_line_past_the_buffer_capacity_is_refused_and_the_stream_stays_in_sync() {
    let s = start().await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    // Past BufReader's 8 KiB capacity the cap fires mid-accumulation; the
    // rest of the line is drained chunk by chunk and the refusal is the
    // same. The stream must come back exactly in sync.
    c.send(&format!("USER {}", "a".repeat(20_000))).await;
    assert_eq!(c.reply().await, "-ERR line too long");
    assert_eq!(c.cmd("CAPA").await, "+OK Capability list follows");
    let _ = c.data().await;
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    // The same over-cap line with no terminator: the drain runs to EOF, the
    // refusal is still written, and the session then ends on the next read.
    let stream = TcpStream::connect(s.pop3_addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut read = tokio::io::BufReader::new(read);
    let mut greeting = Vec::new();
    read.read_until(b'\n', &mut greeting).await.unwrap();
    assert_eq!(greeting, b"+OK Swarmail POP3 server ready\r\n");
    write
        .write_all("b".repeat(20_000).as_bytes())
        .await
        .unwrap();
    write.shutdown().await.unwrap();
    let mut line = Vec::new();
    read.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, b"-ERR line too long\r\n");
    // Then the server sees the EOF and closes.
    let mut tail = Vec::new();
    read.read_to_end(&mut tail).await.unwrap();
    assert!(tail.is_empty());

    // Fed in pieces that are each below the cap, the accumulation keeps
    // looping until the cap is blown — the refusal is still one drained
    // error, and the terminator that arrives late still ends the drain.
    let stream = TcpStream::connect(s.pop3_addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut read = tokio::io::BufReader::new(read);
    let mut greeting = Vec::new();
    read.read_until(b'\n', &mut greeting).await.unwrap();
    assert_eq!(greeting, b"+OK Swarmail POP3 server ready\r\n");
    write.write_all(b"USER ").await.unwrap();
    for _ in 0..3 {
        write.write_all("d".repeat(2000).as_bytes()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    write.write_all(b"\r\n").await.unwrap();
    let mut line = Vec::new();
    read.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, b"-ERR line too long\r\n");
}

#[tokio::test]
async fn pass_rechecks_the_mailbox_before_snapshotting() {
    let s = start().await;
    smtp_send(
        s.smtp_addr,
        Some("cleared"),
        "sender@x.io",
        "cleared",
        "t",
        "b",
    )
    .await
    .unwrap();
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(c.cmd("USER cleared").await, "+OK name is a valid mailbox");
    // The maildrop disappears between USER and PASS.
    let (status, _) = http_json(s.http_addr, "DELETE", "/api/v1/delete-all", None).await;
    assert_eq!(status, 200);
    assert_eq!(c.cmd("PASS whatever").await, "-ERR no such mailbox");
    // A failed PASS leaves the session in the authorization state.
    assert_eq!(
        c.cmd("STAT").await,
        "-ERR command invalid in AUTHORIZATION state"
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn an_emptied_maildrop_is_reported_as_empty() {
    let s = start().await;
    let id = deliver(&s, "emptied", "One", "first").await;
    // Deleting the only mail leaves the inbox in place, now empty.
    let (status, _) = http_json(
        s.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(status, 200);

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("emptied").await;
    assert_eq!(c.cmd("PASS ignored").await, "-ERR already authenticated");
    assert_eq!(c.cmd("STAT").await, "+OK 0 0");
    assert_eq!(c.cmd("LIST").await, "+OK 0 messages (0 octets)");
    assert_eq!(c.data().await, "");
    assert_eq!(c.cmd("UIDL").await, "+OK");
    assert_eq!(c.data().await, "");
    assert_eq!(c.cmd("LIST 1").await, "-ERR no such message");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

#[tokio::test]
async fn an_authorization_state_quit_signs_off_without_updates() {
    let s = start().await;
    let id = deliver(&s, "quick", "One", "first").await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(c.cmd("QUIT").await, "+OK Swarmail POP3 server signing off");
    let (status, _) = http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn pop3_deletions_survive_a_restart_with_a_data_file() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-e2e-pop3-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }

    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let kept = deliver(&srv, "persisted", "One", "first").await;
    let doomed = deliver(&srv, "persisted", "Two", "second").await;

    let mut c = Pop3::connect(srv.pop3_addr).await;
    c.login("persisted").await;
    // Oldest first, so the second mail is number 2.
    assert!(c.cmd("DELE 2").await.starts_with("+OK"));
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
    srv.stop().await;

    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let mut c = Pop3::connect(srv.pop3_addr).await;
    c.login("persisted").await;
    assert!(c.cmd("STAT").await.starts_with("+OK 1 "));
    assert_eq!(
        c.cmd("RETR 1").await,
        format!(
            "+OK {} octets",
            octets("sender@x.io", "persisted", "One", "first")
        )
    );
    assert_eq!(
        c.data_bytes().await,
        raw_of("sender@x.io", "persisted", "One", "first")
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    let (gone, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{doomed}"),
        None,
    )
    .await;
    assert_eq!(gone, 404);
    let (here, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{kept}"),
        None,
    )
    .await;
    assert_eq!(here, 200);

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}
