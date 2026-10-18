//! POP3 over the wire: RFC 1939 sessions against a real listener, with mail
//! delivered over real SMTP — the same store the HTTP API serves.

mod common;

use common::{Pop3, SmtpConn, http_json, smtp_send, start, start_with};
use std::time::Duration;
use swarmail::RunningServer;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The raw bytes smtp_send delivers for these arguments — the RETR payload
/// and the octet counts both come from it.
fn raw_of(from: &str, to: &str, subject: &str, body: &str) -> Vec<u8> {
    format!("From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{body}\r\n").into_bytes()
}

/// The size POP3 advertises for these arguments (decision 0037, amending
/// 0028 §1): the payload with every line CRLF-terminated, the transparency
/// dots RETR adds not counted, the final `.\r\n` excluded — derived from
/// the payload itself, never from a stored length.
fn octets(from: &str, to: &str, subject: &str, body: &str) -> u64 {
    payload_octets(&raw_of(from, to, subject, body))
}

/// The advertised size of a stored mail, computed here as the length of the
/// client's view (`crlf_of`) — an independent statement of RFC 1939 §11
/// ("need not (and must not) be counted twice") — so a server drift shows
/// up as a red count, not a tautology.
fn payload_octets(raw: &[u8]) -> u64 {
    crlf_of(raw).len() as u64
}

/// The octets RETR puts on the wire for a stored mail, computed here: every
/// line CRLF-terminated *and* dot-stuffed, no terminating "." counted. Not
/// the advertised size — it exceeds it by one per dot-leading line.
fn wire_octets(raw: &[u8]) -> u64 {
    let mut n = 0u64;
    let mut start = 0;
    while start < raw.len() {
        let end = raw[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(raw.len(), |i| start + i + 1);
        let mut line = &raw[start..end];
        line = line.strip_suffix(b"\n").unwrap_or(line);
        line = line.strip_suffix(b"\r").unwrap_or(line);
        n += (line.len() + if line.first() == Some(&b'.') { 1 } else { 0 } + 2) as u64;
        start = end;
    }
    n
}

/// The client-side view of a payload: every line CRLF-terminated — what
/// the client holds after undoing RETR's stuffing.
fn crlf_of(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < raw.len() {
        let end = raw[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(raw.len(), |i| start + i + 1);
        let mut line = &raw[start..end];
        line = line.strip_suffix(b"\n").unwrap_or(line);
        line = line.strip_suffix(b"\r").unwrap_or(line);
        out.extend_from_slice(line);
        out.extend_from_slice(b"\r\n");
        start = end;
    }
    out
}

/// Deliver a raw payload to `inbox` over real SMTP: the exact bytes go
/// through DATA untouched (the store keeps them verbatim).
async fn deliver_raw(s: &RunningServer, inbox: &str, raw: &[u8]) {
    let mut c = SmtpConn::connect(s.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    smtp_auth(&mut c, inbox).await;
    c.send("MAIL FROM:<sender@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<r@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("DATA").await;
    assert!(c.reply().await.starts_with("354"));
    c.send_bytes(raw).await;
    c.send(".").await;
    let reply = c.reply().await;
    assert!(reply.starts_with("250"), "raw delivery refused: {reply}");
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

/// The advertised octet count is the payload the client keeps, for every
/// line-ending shape (issue #84, decision 0037 amending 0028 §1): PASS,
/// STAT, LIST, LIST n, RSET and RETR n all name the CRLF-normalized payload
/// without the transparency dots — the client's unstuffed view, checked
/// here against the bytes it received — while the wire itself, measured
/// straight off the socket, still carries one extra octet per dot-leading
/// line. Bare-LF payloads make the stored length differ, and dot-leading
/// ones make the wire length differ, so a count copied from either is red.
#[tokio::test]
async fn sizes_match_what_retr_transmits() {
    let s = start().await;
    let mails: Vec<Vec<u8>> = vec![
        // Plain CRLF: the wire is exactly the stored payload.
        raw_of("sender@x.io", "sizes", "crlf", "body"),
        // Bare-LF lines: RETR terminates them CRLF — two octets more. The
        // wire still owes the terminator its preceding CRLF (decision 0034):
        // the delivery ends with an empty CRLF line, stored as part of the
        // payload, and the bare-LF lines themselves are never normalized.
        b"From: sender@x.io\r\nTo: r@x.io\r\nSubject: barelf\r\n\r\nline1\nline2\n\r\n".to_vec(),
        // Mixed: one bare-LF line, one CRLF line.
        b"From: sender@x.io\r\nTo: r@x.io\r\nSubject: mixed\r\n\r\na\nb\r\n".to_vec(),
        // Dot-stuffed: each leading-dot line gains one octet on the wire,
        // none in the advertised size.
        b"From: sender@x.io\r\nTo: r@x.io\r\nSubject: dotted\r\n\r\n.x\n.y\r\n".to_vec(),
    ];
    for raw in &mails {
        deliver_raw(&s, "sizes", raw).await;
    }
    let sizes: Vec<u64> = mails.iter().map(|raw| payload_octets(raw)).collect();
    let total: u64 = sizes.iter().sum();
    // The dotted mail's wire is two octets longer than its size.
    assert_eq!(wire_octets(&mails[3]), sizes[3] + 2);

    let mut c = Pop3::connect(s.pop3_addr).await;
    // PASS names the normalized total, not the stored or the wire total.
    assert_eq!(c.cmd("USER sizes").await, "+OK name is a valid mailbox");
    assert_eq!(
        c.cmd("PASS whatever").await,
        format!("+OK maildrop has 4 messages ({total} octets)")
    );
    assert_eq!(c.cmd("STAT").await, format!("+OK 4 {total}"));
    assert_eq!(
        c.cmd("LIST").await,
        format!("+OK 4 messages ({total} octets)")
    );
    assert_eq!(
        c.data().await,
        format!(
            "1 {}\r\n2 {}\r\n3 {}\r\n4 {}\r\n",
            sizes[0], sizes[1], sizes[2], sizes[3]
        )
    );
    for (i, n) in sizes.iter().enumerate() {
        let num = i + 1;
        assert_eq!(
            c.cmd(&format!("LIST {num}")).await,
            format!("+OK {num} {n}")
        );
        // The wire RETR sends is the stuffed form — measured verbatim off
        // the socket, its added dots and all.
        let reply = c.cmd(&format!("RETR {num}")).await;
        assert_eq!(reply, format!("+OK {n} octets"), "mail {num}: {reply}");
        let transmitted = c.raw_bytes().await;
        assert_eq!(
            transmitted.len() as u64,
            wire_octets(&mails[i]),
            "mail {num}: transmitted {}",
            transmitted.len()
        );
        // And the body the client keeps — stuffing undone — is exactly the
        // advertised count: the payload with line endings normalized.
        assert_eq!(
            c.cmd(&format!("RETR {num}")).await,
            format!("+OK {n} octets")
        );
        let kept = c.data_bytes().await;
        assert_eq!(kept.len() as u64, *n, "mail {num}: advertised vs kept");
        assert_eq!(
            kept,
            crlf_of(&mails[i]),
            "mail {num}: body is the payload, CRLF-terminated"
        );
    }
    // RSET restates the same total.
    assert_eq!(
        c.cmd("RSET").await,
        format!("+OK maildrop has 4 messages ({total} octets)")
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

/// A failed USER clears the pending name (issue #45, decision 0028 §2):
/// PASS after a failing USER answers `-ERR USER required first` — never
/// logging into a mailbox a *previous* USER named.
#[tokio::test]
async fn a_failed_user_clears_the_pending_name() {
    let s = start().await;
    smtp_send(s.smtp_addr, Some("real"), "sender@x.io", "real", "t", "b")
        .await
        .unwrap();
    let mut c = Pop3::connect(s.pop3_addr).await;

    // A USER that names nothing clears the name USER real left.
    assert_eq!(c.cmd("USER real").await, "+OK name is a valid mailbox");
    assert_eq!(c.cmd("USER nobody-there").await, "-ERR no such mailbox");
    assert_eq!(c.cmd("PASS whatever").await, "-ERR USER required first");

    // So does an empty USER.
    assert_eq!(c.cmd("USER real").await, "+OK name is a valid mailbox");
    assert_eq!(c.cmd("USER").await, "-ERR missing username");
    assert_eq!(c.cmd("PASS whatever").await, "-ERR USER required first");

    // The session is unharmed: a good USER + PASS still logs in.
    c.login("real").await;
    assert!(c.cmd("STAT").await.starts_with("+OK 1 "));
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

/// The maildrop is a snapshot, not a lock (issue #45, decision 0028 §3):
/// mail removed elsewhere after authentication is still named by the
/// session's listings — the documented skew — while every fetch and update
/// goes to the store and answers honestly: `-ERR no such message` for
/// mail that is no longer there, on RETR and on DELE alike.
#[tokio::test]
async fn the_session_reports_mail_removed_elsewhere() {
    let s = start().await;
    let first = deliver(&s, "skew", "One", "first").await;
    let _second = deliver(&s, "skew", "Two", "second").await;

    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("skew").await;

    // Removed over HTTP, after the snapshot was taken.
    let (st, _) = http_json(
        s.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{first}"),
        None,
    )
    .await;
    assert_eq!(st, 200);

    // The snapshot still names both: STAT keeps counting what it
    // authenticated with — the skew is documented, not hidden.
    let n1 = octets("sender@x.io", "skew", "One", "first");
    let n2 = octets("sender@x.io", "skew", "Two", "second");
    assert_eq!(c.cmd("STAT").await, format!("+OK 2 {}", n1 + n2));
    assert_eq!(
        c.cmd("LIST").await,
        format!("+OK 2 messages ({} octets)", n1 + n2)
    );
    assert_eq!(c.data().await, format!("1 {n1}\r\n2 {n2}\r\n"));

    // But the store is the truth for fetches and updates alike (decision
    // 0028 §3, issue #86): the removed mail answers "no such message" on
    // RETR and on DELE, and DELE adds no mark — the snapshot's numbering
    // and listings are unchanged.
    assert_eq!(c.cmd("RETR 1").await, "-ERR no such message");
    assert_eq!(c.cmd("DELE 1").await, "-ERR no such message");
    assert_eq!(c.cmd("STAT").await, format!("+OK 2 {}", n1 + n2));
    // The survivor still marks and unmarks normally.
    assert_eq!(c.cmd("DELE 2").await, "+OK message 2 deleted");
    assert!(c.cmd("RSET").await.starts_with("+OK maildrop has 2 messages"));
    // And the survivor still serves, byte-exact.
    assert_eq!(c.cmd("RETR 2").await, format!("+OK {n2} octets"));
    assert_eq!(
        c.data_bytes().await,
        raw_of("sender@x.io", "skew", "Two", "second")
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

/// Issue #84's literal fixture: a 30-octet mail of three dot-leading lines
/// reports 30 in every reply — PASS, STAT, LIST, LIST 1, RETR, RSET — while
/// RETR's payload occupies 33 octets on the wire (red on the dot-inclusive
/// count, which said 33 everywhere). Literal bytes, no derived oracle; the
/// sender stuffs its own dots per RFC 5321, so the store keeps 30.
#[tokio::test]
async fn three_dot_leading_lines_report_thirty_octets_everywhere() {
    let s = start().await;
    deliver_raw(&s, "thirty", b"..123456\r\n..123456\r\n..123456789\r\n").await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(c.cmd("USER thirty").await, "+OK name is a valid mailbox");
    assert_eq!(
        c.cmd("PASS x").await,
        "+OK maildrop has 1 messages (30 octets)"
    );
    assert_eq!(c.cmd("STAT").await, "+OK 1 30");
    assert_eq!(c.cmd("LIST").await, "+OK 1 messages (30 octets)");
    assert_eq!(c.data().await, "1 30\r\n");
    assert_eq!(c.cmd("LIST 1").await, "+OK 1 30");
    assert_eq!(c.cmd("RETR 1").await, "+OK 30 octets");
    assert_eq!(
        c.raw_bytes().await,
        b"..123456\r\n..123456\r\n..123456789\r\n".to_vec()
    );
    assert_eq!(c.cmd("RETR 1").await, "+OK 30 octets");
    assert_eq!(
        c.data_bytes().await,
        b".123456\r\n.123456\r\n.123456789\r\n".to_vec()
    );
    assert_eq!(
        c.cmd("RSET").await,
        "+OK maildrop has 1 messages (30 octets)"
    );
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
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
    // The stored raw keeps the bare LF (".leading-dots\nplain line\r\n");
    // RETR re-stuffs ".leading-dots\r\n" to "..leading-dots\r\n" on the
    // wire, but its octet count is what the client keeps: CRLF-terminated,
    // the added dot not counted (decision 0037).
    let expected = payload_octets(b".leading-dots\nplain line\r\n");
    assert_eq!(expected, 27, "15 + 12: the normalized payload, not the wire");
    assert_eq!(wire_octets(b".leading-dots\nplain line\r\n"), 28);
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

    // The same over-cap line over the real wire, written in ONE coalesced
    // pass — sizes and byte order, no pacing (issue #102; the ruled shape:
    // a server-side seam cannot exist in a wire test, a pacing helper is
    // the sleep renamed). The fragment is larger than
    // max(MAX_COMMAND_LINE, BufReader's 8 KiB) with the terminator as the
    // last bytes of the stream: no server fill before the terminator can
    // contain '\n' (a fill is at most 8 KiB and the newline sits beyond
    // the run), so the accumulation walks to the mid-line cap and the
    // drain runs to the terminator under any TCP coalescing — on every
    // run, deterministically. Exactly one refusal, no partial reply, no
    // echo of the line; the next command is read whole afterwards.
    let stream = TcpStream::connect(s.pop3_addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut read = tokio::io::BufReader::new(read);
    let mut greeting = Vec::new();
    read.read_until(b'\n', &mut greeting).await.unwrap();
    assert_eq!(greeting, b"+OK Swarmail POP3 server ready\r\n");
    let mut line = Vec::new();
    line.extend_from_slice(b"USER ");
    line.extend_from_slice(&"d".repeat(20_000).into_bytes());
    line.extend_from_slice(b"\r\n");
    write.write_all(&line).await.unwrap();
    let mut reply = Vec::new();
    read.read_until(b'\n', &mut reply).await.unwrap();
    assert_eq!(reply, b"-ERR line too long\r\n");
    // The drained line left the stream in sync: the next command is
    // answered as itself, not as the over-long line's tail.
    write.write_all(b"QUIT\r\n").await.unwrap();
    let mut reply = Vec::new();
    read.read_until(b'\n', &mut reply).await.unwrap();
    assert_eq!(reply, b"+OK Swarmail POP3 server signing off\r\n");
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
async fn an_emptied_maildrop_is_refused_like_a_never_seen_one() {
    let s = start().await;
    let id = deliver(&s, "emptied", "One", "first").await;
    // Deleting the only mail removes the inbox itself (decision 0032,
    // issue #54): no empty shell survives the delete.
    let (status, _) = http_json(
        s.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(status, 200);

    // The maildrop is gone, so USER refuses it — and the refusal is byte
    // for byte the one a never-minted name gets, in the same session.
    let mut c = Pop3::connect(s.pop3_addr).await;
    let vanished = c.cmd("USER emptied").await;
    assert_eq!(
        vanished,
        c.cmd("USER never-minted").await,
        "a vanished maildrop answers exactly like a never-seen one"
    );
    assert_eq!(vanished, "-ERR no such mailbox");
    // Without a valid USER the transaction phase stays unreachable.
    assert_eq!(c.cmd("PASS ignored").await, "-ERR USER required first");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    // No tombstone: the next delivery re-mints the maildrop (0004).
    deliver(&s, "emptied", "Back", "again").await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    c.login("emptied").await;
    assert!(c.cmd("STAT").await.starts_with("+OK 1 "));
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

/// A maildrop can vanish between USER and PASS: another client's delete
/// removed the inbox — with it, the only mail — before the handshake
/// finished. PASS re-checks and refuses, with the same words USER would
/// answer now that the name names nothing (issue #54's one rule).
#[tokio::test]
async fn a_maildrop_vanished_between_user_and_pass_is_refused_at_pass() {
    let s = start().await;
    let id = deliver(&s, "vanish", "One", "first").await;
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(c.cmd("USER vanish").await, "+OK name is a valid mailbox");
    // The inbox goes away mid-handshake (decision 0032: the emptied entry
    // is removed, not emptied in place).
    let (status, _) = http_json(
        s.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(c.cmd("PASS ignored").await, "-ERR no such mailbox");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
}

/// The refusal holds on a reopened store too (issue #54): an emptied
/// maildrop leaves no row, so restore mints no entry and POP3 refuses the
/// name across the restart — while a name that still holds mail is served.
#[tokio::test]
async fn a_vanished_maildrop_is_refused_after_a_reopen_too() {
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
    let gone = deliver(&srv, "gone", "One", "first").await;
    let kept = deliver(&srv, "kept", "Two", "second").await;
    // Deleting the only "gone" mail empties — and removes — that inbox.
    let (status, _) = http_json(
        srv.http_addr,
        "DELETE",
        &format!("/api/v1/messages/{gone}"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let (status, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{kept}"),
        None,
    )
    .await;
    assert_eq!(status, 200, "the other inbox is untouched");
    srv.stop().await;

    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let mut c = Pop3::connect(srv.pop3_addr).await;
    let vanished = c.cmd("USER gone").await;
    assert_eq!(
        vanished,
        c.cmd("USER never-minted").await,
        "a reopened store refuses the emptied maildrop like a never-seen one"
    );
    assert_eq!(vanished, "-ERR no such mailbox");
    assert!(c.cmd("QUIT").await.starts_with("+OK"));
    // The surviving inbox is still served after the restart.
    let mut c = Pop3::connect(srv.pop3_addr).await;
    c.login("kept").await;
    assert!(c.cmd("STAT").await.starts_with("+OK 1 "));
    assert!(c.cmd("QUIT").await.starts_with("+OK"));

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
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

/// Finding 3's POP3 leg (issue #31, decision 0014): a failed commit at QUIT
/// is answered `-ERR`, the session stays in TRANSACTION with the marks
/// intact but unapplied to the store — the mail is still served, so no
/// sign-off, no silent loss. Once the trigger is gone the retry signs off
/// and the mail is gone for good. No fault shim: the refusal is the
/// database's own.
#[tokio::test]
async fn a_failed_quit_answers_err_and_keeps_the_marks() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-e2e-refuse-pop3-{}-{}.db",
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
    let mail = deliver(&srv, "quitbox", "doomed", "body").await;

    let mut c = Pop3::connect(srv.pop3_addr).await;
    c.login("quitbox").await;
    assert_eq!(c.cmd("DELE 1").await, "+OK message 1 deleted");

    let trigger = common::refusing_trigger(&db, "delete");

    // The failed sign-off: `-ERR`, never a `+OK` for work that did not
    // commit (AC3).
    let reply = c.cmd("QUIT").await;
    assert!(
        reply.starts_with("-ERR"),
        "a failed QUIT must not sign off: {reply}"
    );
    assert!(
        reply.contains("storage commit failed"),
        "the -ERR must name the cause: {reply}"
    );

    // The update is abandoned, not half-done: the session is still in
    // TRANSACTION with the marks intact (its own view excludes the marked
    // mail), while the store still serves the mail — unapplied means the
    // maildrop and the data file were never touched.
    assert_eq!(
        c.cmd("LIST").await,
        "+OK 0 messages (0 octets)",
        "the session's own view still excludes the marked mail: {reply}"
    );
    assert_eq!(
        c.data().await,
        "",
        "no scan listing survives the failed update"
    );
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{mail}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "a failed QUIT must not delete the mail: {body}");

    // Recovery: the trigger is gone, the retry signs off and applies.
    trigger.remove();
    assert_eq!(c.cmd("QUIT").await, "+OK Swarmail POP3 server signing off");
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{mail}"),
        None,
    )
    .await;
    assert_eq!(st, 404, "the retry must apply the marks: {st}");

    // And the deletion is durable: the restart copy has no row.
    srv.stop().await;
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let (st, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/quitbox/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        count["count"], 0,
        "the retried QUIT must stay deleted across a restart: {count}"
    );
    srv.stop().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

/// 0015 §1's partial-prefix instrument (issue #31 finding 3, AC3): a
/// *selective* trigger refuses exactly one id, so `QUIT` fails *mid-loop* —
/// after an earlier mark's commit has already landed. An all-refusing
/// trigger can never reach that state (its first delete fails and the
/// prefix stays empty), which is why [`a_failed_quit_answers_err_and_keeps_the_marks`]
/// proves refusal but not the prefix. The contract, driven from the wire:
/// `-ERR storage commit failed`, never a sign-off, the session stays in
/// TRANSACTION, the allowed mark stays committed — gone from the live tree
/// and from a fresh open of the same file — the refused mail and the
/// unmarked bystander stay served on both trees, and the retry `QUIT` in
/// the same session signs off (idempotent over the committed prefix),
/// leaving the restart copy exactly the committed set — no resurrection.
/// One sequential session: no sleeps, no wall-clock assertions (0020 rule
/// 5). Red on `5b09843`, which swallowed the failure and answered `+OK`.
#[tokio::test]
async fn a_failed_quit_prefix_commits_and_the_retry_signs_off() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-e2e-pop3-prefix-{}-{}.db",
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
    // Maildrop order is oldest first: 1 = the allowed mark, 2 = the refused
    // one, 3 = an unmarked bystander. The QUIT loop applies marks in
    // maildrop-number order, so number 1's delete commits before number 2's
    // fails — the mid-loop shape. The bystander is the exactness witness:
    // "exactly the committed set" is only checkable against a non-empty
    // committed set.
    let prefix = deliver(&srv, "quitprefix", "prefix", "first").await;
    let refused = deliver(&srv, "quitprefix", "refused", "second").await;
    let kept = deliver(&srv, "quitprefix", "kept", "third").await;

    let mut c = Pop3::connect(srv.pop3_addr).await;
    c.login("quitprefix").await;
    assert!(c.cmd("DELE 1").await.starts_with("+OK"));
    assert!(c.cmd("DELE 2").await.starts_with("+OK"));

    let trigger = common::refusing_trigger_except(&db, &refused);

    // The mid-loop failure: `-ERR storage commit failed`, never a sign-off.
    let reply = c.cmd("QUIT").await;
    assert!(
        reply.starts_with("-ERR"),
        "a mid-loop QUIT failure must not sign off: {reply}"
    );
    assert!(
        reply.contains("storage commit failed"),
        "the -ERR must name the cause: {reply}"
    );

    // The session stays in TRANSACTION: transaction-state commands still
    // answer (a sign-off would have closed the stream), and the session's
    // own view still excludes its marks — only the unmarked bystander is
    // left of its snapshot.
    let n3 = octets("sender@x.io", "quitprefix", "kept", "third");
    assert_eq!(
        c.cmd("LIST").await,
        format!("+OK 1 messages ({n3} octets)"),
        "the session must still be in TRANSACTION: {reply}"
    );
    assert_eq!(c.data().await, format!("3 {n3}\r\n"));
    assert_eq!(c.cmd("NOOP").await, "+OK");

    // The snapshot carries the store's truth for the committed prefix
    // (decision 0015 §1): its number answers "no such message" — the mail
    // is gone, not merely marked — while the failing suffix stays marked
    // ("already deleted") until a retry or RSET resolves it.
    assert_eq!(c.cmd("RETR 1").await, "-ERR no such message");
    assert_eq!(c.cmd("DELE 1").await, "-ERR no such message");
    assert_eq!(c.cmd("RETR 2").await, "-ERR message already deleted");

    // The prefix committed: the allowed mail is gone from the live tree and
    // from a fresh open of the same file, while the refused mail and the
    // bystander are present on both trees — the failure did not roll the
    // landed commit back, and nothing half-applied.
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{prefix}"),
        None,
    )
    .await;
    assert_eq!(st, 404, "the allowed mark's commit must have landed: {st}");
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{refused}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "the refused mail must still be served: {st}");
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{kept}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "an unmarked mail is untouched: {st}");

    let reopened = swarmail::store::Store::open(0, &db).expect("the same data file reopens");
    assert!(
        reopened.get(&prefix).is_none(),
        "the prefix must be committed on disk, not just absent from memory"
    );
    assert!(
        reopened.get(&refused).is_some(),
        "the refused mail must still have its row"
    );
    assert!(
        reopened.get(&kept).is_some(),
        "the bystander must still have its row"
    );

    // Recovery: the trigger is gone; the retry in the same session applies
    // the remaining mark — idempotent over the committed prefix — and signs
    // off.
    trigger.remove();
    assert_eq!(c.cmd("QUIT").await, "+OK Swarmail POP3 server signing off");
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{refused}"),
        None,
    )
    .await;
    assert_eq!(st, 404, "the retry must apply the refused mark: {st}");

    // The restart copy holds exactly the committed set: the bystander and
    // nothing else — neither deleted mail resurrects.
    srv.stop().await;
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let (st, count) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/quitprefix/count",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        count["count"], 1,
        "the restart copy holds exactly the committed set: {count}"
    );
    for (id, want, why) in [
        (&prefix, 404, "the committed prefix must stay deleted"),
        (&refused, 404, "the retried mark must stay deleted"),
        (&kept, 200, "the bystander must survive"),
    ] {
        let (st, _) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None,
        )
        .await;
        assert_eq!(st, want, "{why}: {st}");
    }
    srv.stop().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

/// RSET clears only the session's own unapplied marks (issue #45, decision
/// 0028 §3): a slot whose delete has committed is marked `gone` on the
/// snapshot (decision 0015 §1), so RSET cannot resurrect it — after the
/// RSET its number answers "-ERR no such message" — while the suffix whose
/// delete failed is unmarked and serves again. The HTTP tree witnesses
/// both: the prefix stays deleted, the refused mail was not deleted by a
/// mere RSET.
#[tokio::test]
async fn rset_never_resurrects_a_committed_delete() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-e2e-pop3-rset-{}-{}.db",
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
    let prefix = deliver(&srv, "rsetgone", "prefix", "first").await;
    let refused = deliver(&srv, "rsetgone", "refused", "second").await;
    let _kept = deliver(&srv, "rsetgone", "kept", "third").await;
    let n2 = octets("sender@x.io", "rsetgone", "refused", "second");
    let n3 = octets("sender@x.io", "rsetgone", "kept", "third");

    let mut c = Pop3::connect(srv.pop3_addr).await;
    c.login("rsetgone").await;
    assert!(c.cmd("DELE 1").await.starts_with("+OK"));
    assert!(c.cmd("DELE 2").await.starts_with("+OK"));

    let trigger = common::refusing_trigger_except(&db, &refused);
    assert!(c.cmd("QUIT").await.starts_with("-ERR"), "mid-loop failure");

    // RSET clears the session's own marks — including the one whose delete
    // failed — so the summary counts the refused mail and the bystander.
    // The committed prefix is not among them: it is gone, not unmarked.
    assert_eq!(
        c.cmd("RSET").await,
        format!("+OK maildrop has 2 messages ({} octets)", n2 + n3)
    );
    // The gone slot stays gone: the number is not resurrectable.
    assert_eq!(c.cmd("RETR 1").await, "-ERR no such message");
    assert_eq!(c.cmd("DELE 1").await, "-ERR no such message");
    // The suffix is truly unmarked: it serves again.
    assert_eq!(c.cmd("RETR 2").await, format!("+OK {n2} octets"));
    assert_eq!(
        c.data_bytes().await,
        raw_of("sender@x.io", "rsetgone", "refused", "second")
    );
    // And the tree saw no delete beyond the committed prefix.
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{prefix}"),
        None,
    )
    .await;
    assert_eq!(st, 404, "the committed prefix must stay deleted: {st}");
    let (st, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{refused}"),
        None,
    )
    .await;
    assert_eq!(st, 200, "RSET must not delete the refused mail: {st}");
    trigger.remove();
    srv.stop().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

// --- 0027: bounded idleness and a compiled session cap --------------------

/// The compiled idle deadline (no knob, 0022 §5): a peer that goes silent
/// after the banner is answered `-ERR idle timeout` and the channel closes
/// (red before 0027: the read blocked for the socket's lifetime). Only
/// POP3's own field is shortened, through `run_on` (decision 0036 §4):
/// an SMTP session opened first still answers after the POP3 expiry (red
/// on the shared-field wiring, where POP3 waited out SMTP's five minutes
/// and the helper deadline fired instead).
#[tokio::test]
async fn a_silent_pop3_peer_is_answered_err_then_the_channel_closes() {
    let s = start_with(|mut c| {
        c.pop3_idle_timeout = Duration::from_millis(150);
        c
    })
    .await;
    let mut smtp = SmtpConn::connect_raw(s.smtp_addr).await;
    assert!(smtp.reply().await.starts_with("220"));
    let mut c = Pop3::connect(s.pop3_addr).await;
    assert_eq!(
        c.read_to_eof().await,
        "-ERR idle timeout\r\n",
        "the expiry answer, then EOF"
    );
    smtp.send("NOOP").await;
    assert_eq!(
        smtp.reply().await,
        "250 2.0.0 OK",
        "SMTP keeps its own bound"
    );
}

/// The reciprocal wiring (decision 0036 §4): shortening SMTP's idle bound
/// leaves POP3 on its own — a POP3 session opened first still answers
/// after a silent SMTP peer has been told `421 4.4.2` and closed (red on
/// the shared-field wiring, which expired the POP3 session too).
#[tokio::test]
async fn a_short_smtp_idle_bound_leaves_pop3_sessions_alone() {
    let s = start_with(|mut c| {
        c.smtp.idle_timeout = Duration::from_millis(150);
        c
    })
    .await;
    let mut pop = Pop3::connect(s.pop3_addr).await;
    let mut smtp = SmtpConn::connect_raw(s.smtp_addr).await;
    assert!(smtp.reply().await.starts_with("220"));
    assert_eq!(
        smtp.reply().await,
        "421 4.4.2 idle timeout, closing transmission channel"
    );
    assert_eq!(smtp.read_to_eof().await, "", "the SMTP channel closed");
    assert!(
        pop.cmd("CAPA").await.starts_with("+OK"),
        "POP3 keeps its own bound"
    );
}

/// The compiled cap refuses honestly: the next session over the cap gets
/// exactly `-ERR too many connections` and no greeting (red before 0027:
/// sessions were unbounded).
#[tokio::test]
async fn a_full_pop3_listener_refuses_the_next_session_before_a_greeting() {
    let s = start_with(|mut c| {
        c.smtp.max_sessions = 1;
        c
    })
    .await;
    // Session one: live (greeting read, nothing said).
    let mut a = Pop3::connect(s.pop3_addr).await;
    // Session two: refused before any greeting.
    let mut b = TcpStream::connect(s.pop3_addr).await.unwrap();
    let mut refused = Vec::new();
    b.read_to_end(&mut refused).await.unwrap();
    assert_eq!(
        String::from_utf8_lossy(&refused),
        "-ERR too many connections\r\n"
    );
    // The cap is a gate, not a scar: once A quits, a fresh session speaks.
    // The freed slot is reaped at admission (decision 0036 §1), so the very
    // next peer, offered once, is greeted — no retry through a refusal. A
    // smoke check: the deterministic proof that the completion is observed
    // as a task, not merely as EOF, is inline in src/pop3.rs.
    a.send("QUIT").await;
    assert!(a.reply().await.starts_with("+OK"));
    assert_eq!(a.read_to_eof().await, "", "the session ended");
    let mut fresh = Pop3::connect(s.pop3_addr).await;
    fresh.send("CAPA").await;
    assert!(
        fresh.reply().await.starts_with("+OK"),
        "the fresh session speaks"
    );
}
