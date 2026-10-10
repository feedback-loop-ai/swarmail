//! POP3 (RFC 1939) maildrop access over the SAME store.
//!
//! A `USER` names an existing swarmail inbox — the maildrop is that inbox
//! — and `PASS` accepts any credentials, mirroring the SMTP accept-any
//! posture (decision 0004). The session snapshots its maildrop, numbered
//! oldest-first, at authentication: numbering stays stable while mail
//! arrives, and new mail is visible to the next session. Every answer is
//! read from the synchronously-queryable store (decision 0001) and `DELE`
//! only marks; the marks are applied through `Store::delete` on `QUIT`
//! (RFC 1939's update state) — an explicit deletion that never touches the
//! oldest-only pruning path (decision 0003). Plaintext only: no APOP, no
//! TLS, like the rest of swarmail's listeners.

use crate::store::{Filter, Store};
use std::collections::BTreeSet;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

/// Hard cap on a command line; verbs and numbers never come close. A longer
/// line is refused and drained, never buffered, so a hostile peer cannot
/// grow server memory with an unterminated line.
const MAX_COMMAND_LINE: usize = 4096;

/// Total POP3 sessions served, for the debug log correlation id.
static SESSIONS: AtomicU64 = AtomicU64::new(0);

/// Accept POP3 connections until `shutdown`, one task per session.
pub async fn serve(
    listener: TcpListener,
    store: Arc<Store>,
    shutdown: impl std::future::Future<Output = ()> + Send,
) -> io::Result<()> {
    let mut shutdown = std::pin::pin!(shutdown);
    loop {
        let (stream, peer) = tokio::select! {
            res = listener.accept() => res?,
            _ = &mut shutdown => return Ok(()),
        };
        let session = SESSIONS.fetch_add(1, Ordering::Relaxed);
        debug!(%peer, session, "pop3 connection accepted");
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, store).await {
                debug!(session, error = %e, "pop3 connection ended");
            }
        });
    }
}

/// One maildrop entry, snapshotted oldest-first at authentication.
#[derive(Debug)]
struct MaildropMsg {
    id: String,
    size: u64,
}

/// The two RFC 1939 phases. The maildrop and deletion marks live outside so
/// command arms can mutate them without fighting the state enum's borrow.
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Authorization,
    Transaction,
}

/// A command line, or the reason there is none to dispatch.
enum Line {
    Command(String),
    TooLong,
    Eof,
}

/// The next command line, capped. Reads through the buffer so an over-long
/// line neither blocks nor accumulates: the tail is drained and reported.
async fn read_line_capped(reader: &mut (impl AsyncBufRead + Unpin)) -> io::Result<Line> {
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            // EOF: a partial line has no terminator, so it is not a command.
            return Ok(Line::Eof);
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(i) => {
                bytes.extend_from_slice(&available[..i]);
                reader.consume(i + 1);
                if bytes.len() > MAX_COMMAND_LINE {
                    return Ok(Line::TooLong);
                }
                let line = String::from_utf8_lossy(&bytes)
                    .trim_end_matches('\r')
                    .to_string();
                return Ok(Line::Command(line));
            }
            None => {
                bytes.extend_from_slice(available);
                let len = available.len();
                reader.consume(len);
                if bytes.len() > MAX_COMMAND_LINE {
                    // Already over the cap with no end in sight: stop
                    // buffering, drain the rest of the line, and leave the
                    // loop to refuse it below.
                    drain_line(reader).await?;
                    break;
                }
            }
        }
    }
    // The only way out of the loop is a cap blown mid-line, already drained.
    Ok(Line::TooLong)
}

/// Consume the rest of an over-long line (up to and including its newline)
/// without buffering it, so the session's byte stream stays in sync and the
/// next command is read whole. A peer that never sends the newline just
/// idles; its connection ends whenever it goes away.
async fn drain_line(reader: &mut (impl AsyncBufRead + Unpin)) -> io::Result<()> {
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok(());
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(i) => {
                reader.consume(i + 1);
                return Ok(());
            }
            None => {
                let len = available.len();
                reader.consume(len);
            }
        }
    }
}

async fn handle_conn(stream: TcpStream, store: Arc<Store>) -> io::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut phase = Phase::Authorization;
    // The USER argument, valid until PASS or a failed authentication.
    let mut user: Option<String> = None;
    // The maildrop, snapshot oldest-first (store.list is newest-first).
    let mut maildrop: Vec<MaildropMsg> = Vec::new();
    // DELE marks, as 1-based maildrop numbers, applied on QUIT.
    let mut deleted: BTreeSet<usize> = BTreeSet::new();

    write_line(&mut write, "+OK Swarmail POP3 server ready").await?;
    loop {
        let line = match read_line_capped(&mut reader).await? {
            Line::Command(c) => c,
            Line::TooLong => {
                write_line(&mut write, "-ERR line too long").await?;
                continue;
            }
            Line::Eof => return Ok(()),
        };
        let (verb, arg) = match line.split_once(' ') {
            Some((v, a)) => (v.to_ascii_uppercase(), a.trim()),
            None => (line.trim().to_ascii_uppercase(), ""),
        };

        // The state machine: a two-phase gate. The maildrop exists only in
        // the transaction phase; there, every command is answered from the
        // snapshot so numbering is stable mid-session.
        match (phase, verb.as_str()) {
            (Phase::Authorization, "USER") => {
                if arg.is_empty() {
                    write_line(&mut write, "-ERR missing username").await?;
                } else if !inbox_exists(&store, arg) {
                    write_line(&mut write, "-ERR no such mailbox").await?;
                } else {
                    user = Some(arg.to_string());
                    write_line(&mut write, "+OK name is a valid mailbox").await?;
                }
            }
            (Phase::Authorization, "PASS") => {
                let Some(name) = user.as_ref() else {
                    write_line(&mut write, "-ERR USER required first").await?;
                    continue;
                };
                // Re-checked: the inbox may have been cleared since USER.
                if !inbox_exists(&store, name) {
                    write_line(&mut write, "-ERR no such mailbox").await?;
                    continue;
                }
                maildrop = store
                    .list(name, &Filter::default())
                    .into_iter()
                    .rev()
                    .map(|e| MaildropMsg {
                        id: e.id.clone(),
                        size: e.size,
                    })
                    .collect();
                deleted.clear();
                let (n, m) = counts(&maildrop, &deleted);
                let reply = if n == 0 {
                    "+OK maildrop empty".to_string()
                } else {
                    format!("+OK maildrop has {n} messages ({m} octets)")
                };
                write_line(&mut write, &reply).await?;
                phase = Phase::Transaction;
                user = None;
            }
            (Phase::Authorization, "CAPA") => write_capabilities(&mut write).await?,
            (Phase::Authorization, "QUIT") => {
                // AUTHORIZATION-state QUIT: no maildrop, nothing to update.
                write_line(&mut write, "+OK Swarmail POP3 server signing off").await?;
                return Ok(());
            }
            (Phase::Authorization, _) => {
                write_line(&mut write, "-ERR command invalid in AUTHORIZATION state").await?;
            }

            (Phase::Transaction, "USER" | "PASS") => {
                write_line(&mut write, "-ERR already authenticated").await?;
            }
            (Phase::Transaction, "STAT") => {
                let (n, m) = counts(&maildrop, &deleted);
                write_line(&mut write, &format!("+OK {n} {m}")).await?;
            }
            (Phase::Transaction, "LIST") => {
                if arg.is_empty() {
                    let (n, m) = counts(&maildrop, &deleted);
                    let mut out = format!("+OK {n} messages ({m} octets)\r\n");
                    for num in live_numbers(&maildrop, &deleted) {
                        out.push_str(&format!("{} {}\r\n", num, maildrop[num - 1].size));
                    }
                    out.push_str(".\r\n");
                    write.write_all(out.as_bytes()).await?;
                } else {
                    match arg
                        .parse::<usize>()
                        .map_err(|_| "invalid message number")
                        .and_then(|n| resolve(&maildrop, &deleted, n).map(|msg| (n, msg.size)))
                    {
                        Ok((n, size)) => write_line(&mut write, &format!("+OK {n} {size}")).await?,
                        Err(e) => write_line(&mut write, &format!("-ERR {e}")).await?,
                    }
                }
            }
            (Phase::Transaction, "UIDL") => {
                if arg.is_empty() {
                    let mut out = String::from("+OK\r\n");
                    for num in live_numbers(&maildrop, &deleted) {
                        out.push_str(&format!("{num} {}\r\n", maildrop[num - 1].id));
                    }
                    out.push_str(".\r\n");
                    write.write_all(out.as_bytes()).await?;
                } else {
                    match arg
                        .parse::<usize>()
                        .map_err(|_| "invalid message number")
                        .and_then(|n| {
                            resolve(&maildrop, &deleted, n).map(|msg| (n, msg.id.clone()))
                        }) {
                        Ok((n, id)) => write_line(&mut write, &format!("+OK {n} {id}")).await?,
                        Err(e) => write_line(&mut write, &format!("-ERR {e}")).await?,
                    }
                }
            }
            (Phase::Transaction, "RETR") => {
                let num = match arg.parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => {
                        write_line(&mut write, "-ERR invalid message number").await?;
                        continue;
                    }
                };
                match resolve(&maildrop, &deleted, num) {
                    Err(e) => write_line(&mut write, &format!("-ERR {e}")).await?,
                    Ok(msg) => match store.get(&msg.id) {
                        // Gone from the store mid-session (cleared over HTTP
                        // or POP3 in another session): not fetchable.
                        None => write_line(&mut write, "-ERR no such message").await?,
                        Some(email) => {
                            let mut out = format!("+OK {} octets\r\n", msg.size).into_bytes();
                            write_stuffed(&mut out, &email.raw);
                            out.extend_from_slice(b".\r\n");
                            write.write_all(&out).await?;
                        }
                    },
                }
            }
            (Phase::Transaction, "TOP") => {
                let num = match arg
                    .split_whitespace()
                    .next()
                    .and_then(|p| p.parse::<usize>().ok())
                {
                    Some(n) => n,
                    None => {
                        write_line(&mut write, "-ERR invalid message number").await?;
                        continue;
                    }
                };
                let lines = match arg
                    .split_whitespace()
                    .nth(1)
                    .and_then(|p| p.parse::<usize>().ok())
                {
                    Some(l) => l,
                    None => {
                        write_line(&mut write, "-ERR invalid line count").await?;
                        continue;
                    }
                };
                match resolve(&maildrop, &deleted, num) {
                    Err(e) => write_line(&mut write, &format!("-ERR {e}")).await?,
                    Ok(msg) => match store.get(&msg.id) {
                        None => write_line(&mut write, "-ERR no such message").await?,
                        Some(email) => {
                            let mut out = b"+OK\r\n".to_vec();
                            let body_at = find_body_start(&email.raw);
                            write_stuffed(&mut out, &email.raw[..body_at]);
                            write_stuffed_limited(&mut out, &email.raw[body_at..], lines);
                            out.extend_from_slice(b".\r\n");
                            write.write_all(&out).await?;
                        }
                    },
                }
            }
            (Phase::Transaction, "DELE") => {
                let num = match arg.parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => {
                        write_line(&mut write, "-ERR invalid message number").await?;
                        continue;
                    }
                };
                match resolve(&maildrop, &deleted, num) {
                    Err(e) => write_line(&mut write, &format!("-ERR {e}")).await?,
                    Ok(_) => {
                        deleted.insert(num);
                        write_line(&mut write, &format!("+OK message {num} deleted")).await?;
                    }
                }
            }
            (Phase::Transaction, "NOOP") => write_line(&mut write, "+OK").await?,
            (Phase::Transaction, "RSET") => {
                deleted.clear();
                let (n, m) = counts(&maildrop, &deleted);
                write_line(
                    &mut write,
                    &format!("+OK maildrop has {n} messages ({m} octets)"),
                )
                .await?;
            }
            (Phase::Transaction, "QUIT") => {
                // RFC 1939 update state — only now do the marks go, through
                // the store's existing delete path (synchronous, mirrored).
                for num in &deleted {
                    store.delete(&maildrop[*num - 1].id);
                }
                write_line(&mut write, "+OK Swarmail POP3 server signing off").await?;
                return Ok(());
            }
            (Phase::Transaction, "CAPA") => write_capabilities(&mut write).await?,
            (Phase::Transaction, _) => {
                write_line(&mut write, "-ERR unknown command").await?;
            }
        }
    }
}

/// A maildrop exists iff the inbox does: a POP3 `USER` names an existing
/// swarmail inbox, and the first mail creates one (decision 0004).
fn inbox_exists(store: &Store, name: &str) -> bool {
    store.inboxes().iter().any(|(n, _)| n == name)
}

/// The maildrop entry for a 1-based message number, or the RFC 1939 error.
fn resolve<'a>(
    maildrop: &'a [MaildropMsg],
    deleted: &BTreeSet<usize>,
    num: usize,
) -> Result<&'a MaildropMsg, &'static str> {
    if num == 0 || num > maildrop.len() {
        return Err("no such message");
    }
    if deleted.contains(&num) {
        return Err("message already deleted");
    }
    Ok(&maildrop[num - 1])
}

/// (message count, total octets) over the still-live maildrop entries.
fn counts(maildrop: &[MaildropMsg], deleted: &BTreeSet<usize>) -> (usize, u64) {
    let mut n = 0;
    let mut m = 0;
    for (i, msg) in maildrop.iter().enumerate() {
        if deleted.contains(&(i + 1)) {
            continue;
        }
        n += 1;
        m += msg.size;
    }
    (n, m)
}

/// The still-live 1-based maildrop numbers, oldest-first.
fn live_numbers(maildrop: &[MaildropMsg], deleted: &BTreeSet<usize>) -> Vec<usize> {
    (1..=maildrop.len())
        .filter(|n| !deleted.contains(n))
        .collect()
}

/// One reply line, CRLF-terminated.
async fn write_line<W: tokio::io::AsyncWrite + Unpin>(w: &mut W, line: &str) -> io::Result<()> {
    w.write_all(line.as_bytes()).await?;
    w.write_all(b"\r\n").await
}

/// RFC 2449 CAPA. Plaintext listener: no STLS; no APOP (any password works).
async fn write_capabilities<W: tokio::io::AsyncWrite + Unpin>(w: &mut W) -> io::Result<()> {
    let mut out = String::from("+OK Capability list follows\r\n");
    for cap in [
        "USER",
        "TOP",
        "UIDL",
        "PIPELINING",
        "IMPLEMENTATION Swarmail",
    ] {
        out.push_str(cap);
        out.push('\r');
        out.push('\n');
    }
    out.push_str(".\r\n");
    w.write_all(out.as_bytes()).await
}

/// Append `raw` to `out` as RFC 1939 multi-line data: every line becomes
/// CRLF-terminated (bare LF, bare CR and a missing final newline are
/// normalized) and any line starting with "." gets a "." stuffed in front.
fn write_stuffed(out: &mut Vec<u8>, raw: &[u8]) {
    write_stuffed_limited(out, raw, usize::MAX);
}

/// The first `max_lines` lines of `raw`, dot-stuffed; the terminating "."
/// line is the caller's.
fn write_stuffed_limited(out: &mut Vec<u8>, raw: &[u8], mut max_lines: usize) {
    let mut start = 0;
    while start < raw.len() && max_lines > 0 {
        let end = raw[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(raw.len(), |i| start + i + 1);
        let line = &raw[start..end];
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.first() == Some(&b'.') {
            out.push(b'.');
        }
        out.extend_from_slice(line);
        out.extend_from_slice(b"\r\n");
        start = end;
        max_lines -= 1;
    }
}

/// Byte offset of the body: after the first empty line. A message with no
/// empty line is all headers (and no body), like RFC 822 wants it read.
fn find_body_start(raw: &[u8]) -> usize {
    let mut start = 0;
    while start < raw.len() {
        let end = raw[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(raw.len(), |i| start + i + 1);
        let line = &raw[start..end];
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            return end;
        }
        start = end;
    }
    raw.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A peer that resets the connection mid-session ends it with a logged
    /// error, not a panic: one clean round-trip, then an abrupt close with
    /// response bytes still unread — the kernel answers that with a RST.
    #[tokio::test]
    async fn a_reset_connection_ends_the_session_with_a_log() {
        use tokio::io::AsyncBufReadExt;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
        let (server, _) = listener.accept().await.unwrap();
        let task = tokio::spawn(handle_conn(server, Arc::new(Store::new(0))));

        // Greeting, then one clean round-trip (CAPA works in both states).
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert!(line.starts_with(b"+OK"));
        client.write_all(b"CAPA\r\n").await.unwrap();
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert_eq!(line, b"+OK Capability list follows\r\n");
        // One more command, then an abortive close: linger-zero makes the
        // drop send a RST on every platform (an unread-reply close alone is
        // a Linux refinement — macOS answers a tidy FIN), so the server's
        // next read fails instead of seeing a tidy EOF everywhere.
        client.write_all(b"NOOP\r\n").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await; // the reply lands, stays unread
        let stream = client.into_inner();
        #[allow(deprecated)] // the RST is the point of the test
        stream.set_linger(Some(std::time::Duration::ZERO)).unwrap();
        drop(stream);

        let outcome = task.await.unwrap();
        assert!(outcome.is_err(), "expected an io error, got {outcome:?}");
    }

    #[test]
    fn stuffing_normalizes_line_endings_and_escapes_dots() {
        let mut out = Vec::new();
        write_stuffed(&mut out, b".lead\r\nmid\n.\r\r\nno-newline");
        assert_eq!(out, b"..lead\r\nmid\r\n..\r\r\nno-newline\r\n");

        let mut out = Vec::new();
        write_stuffed(&mut out, b"");
        assert!(out.is_empty());

        let mut out = Vec::new();
        write_stuffed(&mut out, b"a\r\n.");
        assert_eq!(out, b"a\r\n..\r\n");
    }

    #[test]
    fn stuffing_limits_body_lines() {
        let mut out = Vec::new();
        write_stuffed_limited(&mut out, b"1\n2\n3", 2);
        assert_eq!(out, b"1\r\n2\r\n");
    }

    #[test]
    fn body_starts_after_the_first_blank_line() {
        assert_eq!(find_body_start(b"Subject: x\r\n\r\nbody"), 14);
        // No blank line: everything is headers.
        assert_eq!(find_body_start(b"Subject: x\r\n"), 12);
        // Blank line already at the end.
        assert_eq!(find_body_start(b"A: 1\r\n\r\n"), 8);
    }

    #[test]
    fn counts_skip_marked_messages() {
        let maildrop = vec![
            MaildropMsg {
                id: "a".into(),
                size: 10,
            },
            MaildropMsg {
                id: "b".into(),
                size: 20,
            },
            MaildropMsg {
                id: "c".into(),
                size: 30,
            },
        ];
        let deleted: BTreeSet<usize> = [2].into_iter().collect();
        assert_eq!(counts(&maildrop, &deleted), (2, 40));
        assert_eq!(live_numbers(&maildrop, &deleted), vec![1, 3]);
        assert_eq!(
            resolve(&maildrop, &deleted, 2).unwrap_err(),
            "message already deleted"
        );
        assert_eq!(
            resolve(&maildrop, &deleted, 4).unwrap_err(),
            "no such message"
        );
        assert_eq!(
            resolve(&maildrop, &deleted, 0).unwrap_err(),
            "no such message"
        );
        assert_eq!(resolve(&maildrop, &deleted, 1).unwrap().id, "a");
    }
}
