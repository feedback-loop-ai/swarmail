//! POP3 (RFC 1939) maildrop access over the SAME store.
//!
//! A `USER` names an existing swarmail inbox — the maildrop is that inbox
//! — and `PASS` accepts any credentials, mirroring the SMTP accept-any
//! posture (decision 0004). A failing `USER` — empty argument, unknown
//! mailbox — clears the pending name (decision 0028 §2): `PASS`
//! afterwards answers `-ERR USER required first`, never logging into a
//! mailbox a *previous* `USER` named. Every answer is read from the
//! synchronously-queryable store (decision 0001) and `DELE` only marks;
//! the marks are applied through `Store::delete_async` on `QUIT` (RFC
//! 1939's update state) — an explicit deletion that never touches the
//! oldest-only pruning path (decision 0003).
//!
//! The session snapshots its maildrop, numbered oldest-first, at
//! authentication: numbering stays stable while mail arrives, and new mail
//! is visible to the next session. That is a snapshot, not a lock (decision
//! 0028 §3): listings answer from the snapshot, fetches and updates go to
//! the store, and mail removed elsewhere (another session's committed
//! delete, an HTTP clear) is reported `-ERR no such message` — the skew is
//! documented, never hidden. `RSET` clears the session's own unapplied
//! marks; it can never resurrect a delete that has committed, because a
//! slot whose delete landed is marked `gone` on the snapshot (decision
//! 0015 §1): the store's truth carried on the slot, answered `-ERR no such
//! message` instead of "already deleted".
//!
//! An advertised octet count is the length of the bytes RETR transmits —
//! CRLF-terminated, dot-stuffed, excluding the final `.\r\n` (RFC 1939 §11,
//! decision 0028 §1). It is derived by running the same writer, never
//! stored a second time: a count cannot drift from the wire.
//!
//! `QUIT` reports a failed commit (`-ERR storage commit failed`, decision
//! 0014) instead of pretending, and stays in TRANSACTION so the client may
//! retry (decision 0015 §1). Plaintext only: no APOP, no STLS — SMTP is
//! the one swarmail listener with STARTTLS.

use crate::store::{Filter, Store};
use std::collections::BTreeSet;
use std::io;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

/// Hard cap on a command line; verbs and numbers never come close. A longer
/// line is refused and drained, never buffered, so a hostile peer cannot
/// grow server memory with an unterminated line.
const MAX_COMMAND_LINE: usize = 4096;

/// Total POP3 sessions served, for the debug log correlation id.
static SESSIONS: AtomicU64 = AtomicU64::new(0);

/// Test-only: sessions reaped by the accept loop's harvest. Its only reader
/// is the inline probe that installs it (decision 0020 rule 4) — the probe
/// drives the real `serve`, and the counter must track completed sessions
/// across successive connection batches or the deadline expires red.
#[cfg(test)]
static REAPED: AtomicUsize = AtomicUsize::new(0);

// Test-only: counted, one-shot accept errors (decision 0020 rule 4, the
// `REAPED` precedent; the SMTP sibling's seam). Thread-local because the
// loop is polled on the injecting test's own current-thread runtime.
#[cfg(test)]
thread_local! {
    static INJECTED_ACCEPT_ERRORS: core::cell::Cell<usize> =
        const { core::cell::Cell::new(0) };
}

/// Install `count` failing accepts for the loop driven by this test.
#[cfg(test)]
pub(crate) fn inject_accept_errors(count: usize) {
    INJECTED_ACCEPT_ERRORS.with(|errors| errors.set(count));
}

/// Consume one installed accept error, if any remain.
#[cfg(test)]
fn take_injected_accept_error() -> bool {
    INJECTED_ACCEPT_ERRORS.with(|errors| {
        let left = errors.get();
        if left == 0 {
            false
        } else {
            errors.set(left - 1);
            true
        }
    })
}

/// Accept POP3 connections until `shutdown`, one task per session. The
/// accept loop retries forever with a fixed backoff (decision 0027): a
/// transient failure sets `degraded` and pauses; a stop is raced against
/// the pause, never stacked behind it.
pub async fn serve(
    listener: TcpListener,
    store: Arc<Store>,
    degraded: Arc<AtomicBool>,
    idle_timeout: Duration,
    max_sessions: usize,
    shutdown: impl std::future::Future<Output = ()> + Send,
) -> io::Result<()> {
    let mut shutdown = std::pin::pin!(shutdown);
    // Every session is owned by this loop: ready completions are reaped at
    // the top of every iteration — never parked — so a finished session
    // cannot accumulate in the set with the cumulative connection count; the
    // set is drained on the shutdown arm, so no session socket outlives the
    // serve loop. A session mid-transaction has no commit to land — the
    // closed socket is the only goodbye it gets.
    let mut sessions: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
    loop {
        // Reap ready completions without blocking: `try_join_next` never
        // parks the accept path and never touches a session still running.
        while sessions.try_join_next().is_some() {
            #[cfg(test)]
            REAPED.fetch_add(1, Ordering::Relaxed);
        }
        // The accept itself, raced against the stop. The seam arm (test-only)
        // is consumed on the first poll, so the error arm below is reachable
        // and branch-stable; in production the block is just the accept.
        let accepted = tokio::select! {
            res = async {
                #[cfg(test)]
                if take_injected_accept_error() {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "injected accept error",
                    ));
                }
                listener.accept().await
            } => res,
            _ = &mut shutdown => {
                sessions.abort_all();
                while sessions.join_next().await.is_some() {}
                return Ok(());
            }
        };
        let (stream, peer) = match accepted {
            Ok(accepted) => {
                // A successful accept is the recovery: the listener answers
                // again, so the surface stops claiming degraded before the
                // next failure can.
                degraded.store(false, Ordering::Relaxed);
                accepted
            }
            Err(e) => {
                // A transient failure is not a death sentence (decision
                // 0027): say so once per backoff step and retry forever.
                degraded.store(true, Ordering::Relaxed);
                tracing::warn!(error = %e, "pop3 accept failed; retrying after the backoff");
                tokio::select! {
                    _ = &mut shutdown => {
                        sessions.abort_all();
                        while sessions.join_next().await.is_some() {}
                        return Ok(());
                    }
                    _ = tokio::time::sleep(crate::smtp::ACCEPT_BACKOFF) => {}
                }
                continue;
            }
        };
        // Bounded sessions (decision 0027): the cap is checked against the
        // live set after the reap. A refusal is wire-honest and retryable,
        // written under the same idle deadline before any session state
        // exists, and the socket is closed without spawning.
        if sessions.len() >= max_sessions {
            let _ = crate::smtp::bound_io(
                async {
                    let mut stream = stream;
                    stream.write_all(b"-ERR too many connections\r\n").await?;
                    stream.flush().await
                },
                idle_timeout,
            )
            .await;
            debug!(%peer, "pop3 refused: session cap reached");
            continue;
        }
        let session = SESSIONS.fetch_add(1, Ordering::Relaxed);
        debug!(%peer, session, "pop3 connection accepted");
        let store = store.clone();
        sessions.spawn(async move {
            if let Err(e) = handle_conn(stream, store, idle_timeout).await {
                debug!(session, error = %e, "pop3 connection ended");
            }
        });
    }
}

/// One maildrop entry, snapshotted oldest-first at authentication. `size`
/// is the wire: the octets RETR transmits for this mail (decision 0028 §1),
/// derived by running the writer — never the stored length. `gone` marks a
/// slot whose delete has *committed* (decision 0015 §1): the mail is no
/// longer in the store, so the slot may serve nothing — the store's truth
/// carried on the slot, keeping `RSET` from resurrecting a landed delete.
#[derive(Debug)]
struct MaildropMsg {
    id: String,
    size: u64,
    gone: bool,
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
/// The cap is the loop's own guard — entered while the accumulation is
/// within the cap, left the moment it is past it — so both sides of the
/// check are taken by any over-cap line: an `if`-and-`break` inside the
/// read loop instead leaves the check's false arm (and its block's closing
/// line) at a zero count under llvm-cov's merged async mapping, which the
/// exact gate refuses (issue #101).
async fn read_line_capped(reader: &mut (impl AsyncBufRead + Unpin)) -> io::Result<Line> {
    let mut bytes: Vec<u8> = Vec::new();
    while bytes.len() <= MAX_COMMAND_LINE {
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
            }
        }
    }
    // The guard is the only exit: a cap blown mid-line. Drain the rest of
    // the line, then refuse it below. The drain lives outside the loop —
    // an `await` in a conditional block inside it misattributes its region
    // under the merged mapping (the statements report hits while the
    // block's closing line reads zero), which the exact gate refuses
    // (issue #101).
    drain_line(reader).await?;
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

async fn handle_conn(stream: TcpStream, store: Arc<Store>, idle: Duration) -> io::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut phase = Phase::Authorization;
    // The USER argument, valid until PASS or a failed authentication.
    let mut user: Option<String> = None;
    // The maildrop, snapshot oldest-first (store.list is newest-first).
    let mut maildrop: Vec<MaildropMsg> = Vec::new();
    // DELE marks, as 1-based maildrop numbers, applied on QUIT.
    let mut deleted: BTreeSet<usize> = BTreeSet::new();

    write_line(&mut write, "+OK Swarmail POP3 server ready", idle).await?;
    loop {
        // The command read is the session's idle chokepoint: a peer that
        // says nothing is answered honestly and the session ends — it
        // cannot pin a task forever (0027; the shared wrapper maps the
        // elapsed deadline to `TimedOut`).
        let line = match crate::smtp::bound_io(read_line_capped(&mut reader), idle).await {
            Ok(line) => match line {
                Line::Command(c) => c,
                Line::TooLong => {
                    write_line(&mut write, "-ERR line too long", idle).await?;
                    continue;
                }
                Line::Eof => return Ok(()),
            },
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                write_line(&mut write, "-ERR idle timeout", idle).await?;
                return Ok(());
            }
            Err(e) => return Err(e),
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
                    // A failing USER leaves no pending name (decision 0028
                    // §2): PASS afterwards answers "-ERR USER required
                    // first", never logging into a mailbox a previous USER
                    // named.
                    user = None;
                    write_line(&mut write, "-ERR missing username", idle).await?;
                } else if !inbox_exists(&store, arg) {
                    user = None;
                    write_line(&mut write, "-ERR no such mailbox", idle).await?;
                } else {
                    user = Some(arg.to_string());
                    write_line(&mut write, "+OK name is a valid mailbox", idle).await?;
                }
            }
            (Phase::Authorization, "PASS") => {
                let Some(name) = user.as_ref() else {
                    write_line(&mut write, "-ERR USER required first", idle).await?;
                    continue;
                };
                // Re-checked: the inbox may have been cleared since USER.
                if !inbox_exists(&store, name) {
                    write_line(&mut write, "-ERR no such mailbox", idle).await?;
                    continue;
                }
                maildrop = store
                    .list(name, &Filter::default())
                    .into_iter()
                    .rev()
                    .map(|e| MaildropMsg {
                        id: e.id.clone(),
                        // The advertised size is the wire: the octets RETR
                        // would transmit for this mail (decision 0028 §1),
                        // derived by running the writer — never `e.size`,
                        // which is a second copy of a fact the wire already
                        // states and could drift from it.
                        size: wire_len(&e.raw),
                        gone: false,
                    })
                    .collect();
                deleted.clear();
                // The re-check above narrows the race but is not atomic
                // with this read (0014: mutation-only boundary — 0028's
                // snapshot posture, 0032's entry-lifetime rule notwithstanding):
                // a concurrent delete/clear can empty the inbox between
                // the two reads, and a zero count is then the honest answer.
                let (n, m) = counts(&maildrop, &deleted);
                let reply = format!("+OK maildrop has {n} messages ({m} octets)");
                write_line(&mut write, &reply, idle).await?;
                phase = Phase::Transaction;
                user = None;
            }
            (Phase::Authorization, "CAPA") => write_capabilities(&mut write, idle).await?,
            (Phase::Authorization, "QUIT") => {
                // AUTHORIZATION-state QUIT: no maildrop, nothing to update.
                write_line(&mut write, "+OK Swarmail POP3 server signing off", idle).await?;
                return Ok(());
            }
            (Phase::Authorization, _) => {
                write_line(
                    &mut write,
                    "-ERR command invalid in AUTHORIZATION state",
                    idle,
                )
                .await?;
            }

            (Phase::Transaction, "USER" | "PASS") => {
                write_line(&mut write, "-ERR already authenticated", idle).await?;
            }
            (Phase::Transaction, "STAT") => {
                let (n, m) = counts(&maildrop, &deleted);
                write_line(&mut write, &format!("+OK {n} {m}"), idle).await?;
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
                        Ok((n, size)) => {
                            write_line(&mut write, &format!("+OK {n} {size}"), idle).await?
                        }
                        Err(e) => write_line(&mut write, &format!("-ERR {e}"), idle).await?,
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
                        Ok((n, id)) => {
                            write_line(&mut write, &format!("+OK {n} {id}"), idle).await?
                        }
                        Err(e) => write_line(&mut write, &format!("-ERR {e}"), idle).await?,
                    }
                }
            }
            (Phase::Transaction, "RETR") => {
                let num = match arg.parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => {
                        write_line(&mut write, "-ERR invalid message number", idle).await?;
                        continue;
                    }
                };
                match resolve(&maildrop, &deleted, num) {
                    Err(e) => write_line(&mut write, &format!("-ERR {e}"), idle).await?,
                    Ok(msg) => match store.get(&msg.id) {
                        // Gone from the store mid-session (cleared over HTTP
                        // or POP3 in another session): not fetchable.
                        None => write_line(&mut write, "-ERR no such message", idle).await?,
                        Some(email) => {
                            // The count is the body's own length: the octets
                            // this session is about to transmit (decision
                            // 0028 §1) — the same writer that fills it, so
                            // the advertisement cannot drift from the wire.
                            let mut body = Vec::new();
                            write_stuffed(&mut body, &email.raw);
                            let mut out = format!("+OK {} octets\r\n", body.len()).into_bytes();
                            out.extend_from_slice(&body);
                            out.extend_from_slice(b".\r\n");
                            crate::smtp::bound_io(write.write_all(&out), idle).await?;
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
                        write_line(&mut write, "-ERR invalid message number", idle).await?;
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
                        write_line(&mut write, "-ERR invalid line count", idle).await?;
                        continue;
                    }
                };
                match resolve(&maildrop, &deleted, num) {
                    Err(e) => write_line(&mut write, &format!("-ERR {e}"), idle).await?,
                    Ok(msg) => match store.get(&msg.id) {
                        None => write_line(&mut write, "-ERR no such message", idle).await?,
                        Some(email) => {
                            let mut out = b"+OK\r\n".to_vec();
                            let body_at = find_body_start(&email.raw);
                            write_stuffed(&mut out, &email.raw[..body_at]);
                            write_stuffed_limited(&mut out, &email.raw[body_at..], lines);
                            out.extend_from_slice(b".\r\n");
                            crate::smtp::bound_io(write.write_all(&out), idle).await?;
                        }
                    },
                }
            }
            (Phase::Transaction, "DELE") => {
                let num = match arg.parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => {
                        write_line(&mut write, "-ERR invalid message number", idle).await?;
                        continue;
                    }
                };
                match resolve(&maildrop, &deleted, num) {
                    Err(e) => write_line(&mut write, &format!("-ERR {e}"), idle).await?,
                    Ok(_) => {
                        deleted.insert(num);
                        write_line(&mut write, &format!("+OK message {num} deleted"), idle).await?;
                    }
                }
            }
            (Phase::Transaction, "NOOP") => write_line(&mut write, "+OK", idle).await?,
            (Phase::Transaction, "RSET") => {
                deleted.clear();
                let (n, m) = counts(&maildrop, &deleted);
                write_line(
                    &mut write,
                    &format!("+OK maildrop has {n} messages ({m} octets)"),
                    idle,
                )
                .await?;
            }
            (Phase::Transaction, "QUIT") => {
                // RFC 1939 update state — only now do the marks go, through
                // the store's existing delete path (awaited off the async
                // workers, decision 0023) — before the signing-off reply. A
                // failed commit is the answer (decision 0014): `-ERR` with
                // the fixed text, no sign-off — the session stays in
                // TRANSACTION. The update is at-least-once, not atomic
                // (decision 0015 §1): the marks are applied one store delete
                // at a time, so a mid-loop failure leaves the *committed
                // prefix* committed and the failing suffix marked, unapplied.
                // Nothing reports deleted and resurrects: a number whose
                // delete landed leaves the mark set and its slot is marked
                // `gone` — the store's truth on the slot, so RETR answers
                // `-ERR no such message` for mail that no longer exists and
                // `RSET` cannot bring it back — while the suffix keeps its
                // mark and answers `-ERR message already deleted`, its mail
                // still on both trees. Nothing lies and every client path
                // converges: a retry `QUIT` is idempotent over the unapplied
                // suffix, and `RSET` unmarks only what has not committed.
                let mut commit_failed = false;
                let numbers: Vec<usize> = deleted.iter().copied().collect();
                for num in numbers {
                    match store
                        .clone()
                        .delete_async(maildrop[num - 1].id.clone())
                        .await
                    {
                        Ok(_) => {
                            // Committed: the number leaves the mark set and
                            // the slot is marked gone — it can never be
                            // served or resurrected again.
                            maildrop[num - 1].gone = true;
                            deleted.remove(&num);
                        }
                        Err(_) => {
                            commit_failed = true;
                            break;
                        }
                    }
                }
                if commit_failed {
                    // A deliberate deviation from RFC 1939 §6, which has
                    // QUIT end the session whatever the removal did —
                    // "Whether the removal was successful or not, the
                    // server then releases any exclusive-access lock on
                    // the maildrop and closes the TCP connection." Here
                    // the `-ERR` is not a sign-off: the socket stays open
                    // and the session stays in TRANSACTION (Mailpit's
                    // behavior, which the compat shims track), so the
                    // client keeps its maildrop and its marks — it may
                    // re-LIST, RSET to unmark, or retry QUIT, which the
                    // loop above makes idempotent over the unapplied
                    // suffix. Closing the connection would strand a client
                    // that could otherwise converge.
                    write_line(&mut write, "-ERR storage commit failed", idle).await?;
                    continue;
                }
                write_line(&mut write, "+OK Swarmail POP3 server signing off", idle).await?;
                return Ok(());
            }
            (Phase::Transaction, "CAPA") => write_capabilities(&mut write, idle).await?,
            (Phase::Transaction, _) => {
                write_line(&mut write, "-ERR unknown command", idle).await?;
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
/// A number marked `deleted` answers "message already deleted" — its mail
/// is still in the store until QUIT. A *gone* slot (its delete committed,
/// decision 0015 §1) answers "no such message" — the mail no longer exists
/// to serve, whatever the marks say.
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
    if maildrop[num - 1].gone {
        return Err("no such message");
    }
    Ok(&maildrop[num - 1])
}

/// (message count, total octets) over the still-live maildrop entries: not
/// marked `deleted` and not `gone` — a gone slot's mail no longer exists,
/// so neither its count nor its octets can be reported (decision 0015 §1).
fn counts(maildrop: &[MaildropMsg], deleted: &BTreeSet<usize>) -> (usize, u64) {
    let mut n = 0;
    let mut m = 0;
    for (i, msg) in maildrop.iter().enumerate() {
        if deleted.contains(&(i + 1)) || msg.gone {
            continue;
        }
        n += 1;
        m += msg.size;
    }
    (n, m)
}

/// The still-live 1-based maildrop numbers, oldest-first: not marked
/// `deleted` and not `gone` (a gone slot serves nothing — its mail is no
/// longer in the store to fetch).
fn live_numbers(maildrop: &[MaildropMsg], deleted: &BTreeSet<usize>) -> Vec<usize> {
    (1..=maildrop.len())
        .filter(|n| !deleted.contains(n) && !maildrop[n - 1].gone)
        .collect()
}

/// One reply line, CRLF-terminated, under one idle bound (decision 0027):
/// a peer that stops accepting replies ends the session silently through
/// the existing `?` — a TimedOut is an ended session, not a pinned task.
async fn write_line<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    line: &str,
    idle: Duration,
) -> io::Result<()> {
    crate::smtp::bound_io(
        async {
            w.write_all(line.as_bytes()).await?;
            w.write_all(b"\r\n").await
        },
        idle,
    )
    .await
}

/// RFC 2449 CAPA. Plaintext listener: no STLS; no APOP (any password works).
async fn write_capabilities<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    idle: Duration,
) -> io::Result<()> {
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
    crate::smtp::bound_io(w.write_all(out.as_bytes()), idle).await
}

/// Append `raw` to `out` as RFC 1939 multi-line data: every line becomes
/// CRLF-terminated (a bare LF and a missing final newline are normalized;
/// a bare CR mid-line passes through — it is not a line ending) and any
/// line starting with "." gets a "." stuffed in front.
fn write_stuffed(out: &mut Vec<u8>, raw: &[u8]) {
    write_stuffed_limited(out, raw, usize::MAX);
}

/// The wire length of a stored mail: the octets RETR transmits for it —
/// CRLF-terminated, dot-stuffed, excluding the final `.\r\n` (RFC 1939 §11,
/// decision 0028 §1). Derived by running the writer itself, never a
/// mirrored walk of its rules: the count cannot drift from the wire, and
/// no stored field ever states the length twice.
fn wire_len(raw: &[u8]) -> u64 {
    let mut buf = Vec::new();
    write_stuffed(&mut buf, raw);
    buf.len() as u64
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
        let task = tokio::spawn(handle_conn(
            server,
            Arc::new(Store::new(0)),
            Duration::from_secs(60),
        ));

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
                gone: false,
            },
            MaildropMsg {
                id: "b".into(),
                size: 20,
                gone: false,
            },
            MaildropMsg {
                id: "c".into(),
                size: 30,
                gone: false,
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

    /// The flag a test observes flipping, spun against a real deadline —
    /// never a sleep (decision 0020 rule 5).
    async fn flag_becomes(flag: &AtomicBool, want: bool, what: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while flag.load(Ordering::Relaxed) != want {
            assert!(
                std::time::Instant::now() < deadline,
                "the accept loop never {what}"
            );
            tokio::task::yield_now().await;
        }
    }

    /// One injected accept failure: the loop flags itself degraded, backs
    /// off, and then serves a real session on the same listener — a
    /// transient accept error is survived, not fatal (the SMTP sibling's
    /// proof). The flag is also observed cleared: the next successful
    /// accept is the recovery.
    #[tokio::test]
    async fn one_failed_accept_backs_off_then_recovers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            degraded.clone(),
            Duration::from_secs(60),
            256,
            std::future::pending(),
        ));
        // Flipped before the loop is first polled, so the first accept is
        // the injected failure — the arm is branch-stable (0020 rule 1).
        inject_accept_errors(1);

        flag_becomes(&degraded, true, "flagged degraded on the failed accept").await;
        // The same listener still speaks once the backoff has passed.
        let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert!(
            line.starts_with(b"+OK"),
            "the listener never recovered: {line:?}"
        );
        flag_becomes(
            &degraded,
            false,
            "cleared the flag after the successful accept",
        )
        .await;
        server.abort();
    }

    /// A stop arriving inside a backoff pause resolves the loop well inside
    /// that pause — the shutdown is raced against the backoff, never
    /// stacked behind it (the SMTP sibling's proof, on the POP3 arm). A
    /// live session holds a slot when the stop lands, so the drain inside
    /// the shutdown arm joins a genuinely aborted task — the shape a stop
    /// interrupts a *busy* degraded listener, not an idle one.
    #[tokio::test]
    async fn a_stop_inside_a_backoff_resolves_well_inside_it() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            degraded.clone(),
            Duration::from_secs(60),
            256,
            async move {
                let mut stop_rx = stop_rx;
                let _ = stop_rx.changed().await;
            },
        ));
        // The greeting is written by the session task, so reading it proves
        // the session is spawned — and it stays spawned, parked on a read
        // this test never answers.
        let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert!(
            line.starts_with(b"+OK "),
            "greeting proves the spawn: {line:?}"
        );
        inject_accept_errors(1);
        // The loop is parked inside the accept (nothing wakes it) — the
        // seam is consumed on the accept's next *poll*, so a second
        // connection is the poke: its handshake fires the accept's waker,
        // the poll re-checks the seam first, and the error wins before the
        // accept is ever awaited. Branch-stable, no sleep.
        let _poke = TcpStream::connect(addr).await.unwrap();
        // The flag is set in the same poll that enters the backoff select,
        // so observing it means the loop is parked there — the stop lands
        // mid-pause, not before it.
        flag_becomes(&degraded, true, "entered the backoff").await;
        let _ = stop_tx.send(true);
        let outcome = tokio::time::timeout(Duration::from_millis(500), server).await;
        let res = outcome.expect("the stop did not resolve inside the backoff");
        assert!(res.is_ok(), "the raced stop must exit cleanly: {res:?}");
    }

    /// The accept loop reaps completed sessions: each offered connection is
    /// one loop iteration, and each iteration's harvest joins every session
    /// completed so far — so a completed entry is reaped by a later accept
    /// instead of accumulating in the set. Polled against a real deadline
    /// (never a sleep): with the harvest gone the counter never advances and
    /// the deadline expires red. Returns the counter so a second batch can
    /// prove the reap keeps tracking completions.
    async fn reap_completed_sessions_until(count: usize) -> usize {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(60),
            256,
            std::future::pending(),
        ));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let reaped = loop {
            assert!(
                std::time::Instant::now() < deadline,
                "the accept loop never reaped {count} completed session(s)"
            );
            // EOF on arrival: the session ends, the next accept reaps it.
            drop(TcpStream::connect(addr).await.unwrap());
            let reaped = REAPED.load(Ordering::Relaxed);
            if reaped >= count {
                break reaped;
            }
            tokio::task::yield_now().await;
        };
        server.abort();
        reaped
    }

    /// Completed connection entries do not accumulate across successive
    /// batches: the first batch is reaped, and the second advances the
    /// counter beyond it — the harvest is continuous, not a shutdown-only
    /// drain (red on the join-in-shutdown-arm-only shape, which never reaps).
    #[tokio::test]
    async fn the_accept_loop_reaps_completed_sessions_across_batches() {
        let after_first_batch = reap_completed_sessions_until(1).await;
        assert!(after_first_batch >= 1, "{after_first_batch}");
        let after_second_batch = reap_completed_sessions_until(after_first_batch + 1).await;
        assert!(
            after_second_batch > after_first_batch,
            "the reap must keep tracking completions: {after_first_batch} -> {after_second_batch}"
        );
    }
}
