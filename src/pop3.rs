//! POP3 (RFC 1939) maildrop access over the SAME store.
//!
//! A `USER` names an existing swarmail inbox — the maildrop is that inbox
//! — and `PASS` accepts any credentials, mirroring the SMTP accept-any
//! posture (decision 0004). The name is everything after `USER` and its one
//! space, verbatim — padding included, only the line's own CRLF removed —
//! under the one inbox-name rule every plane shares (decision 0046 §§4–5); a
//! command that is not UTF-8 ends the session undispatched. A failing
//! `USER` — empty argument, a blank or control-bearing name, unknown
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
//! An advertised octet count is the size the client retains: the stored
//! payload with line endings normalized to CRLF, never counting the
//! transparency dots RETR adds and the client removes, and excluding the
//! final `.\r\n` (RFC 1939 §11, decision 0037 amending 0028 §1). It is
//! derived by the same traversal that writes the wire — a counting sink
//! that skips the added dots and allocates nothing — so a count cannot
//! drift from what RETR sends.
//!
//! `QUIT` reports a failed commit (`-ERR storage commit failed`, decision
//! 0014) instead of pretending, and stays in TRANSACTION so the client may
//! retry (decision 0015 §1). Plaintext only: no APOP, no STLS — SMTP is
//! the one swarmail listener with STARTTLS.

use crate::store::{Filter, Store};
use std::collections::BTreeSet;
use std::io;
#[cfg(test)]
use std::marker::PhantomData;
#[cfg(test)]
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(test)]
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

/// Hard cap on a command line; verbs and numbers never come close. A longer
/// line is refused and drained, never buffered, so a hostile peer cannot
/// grow server memory with an unterminated line.
const MAX_COMMAND_LINE: usize = 4096;

/// POP3's idle bound (RFC 1939 §3's ten-minute autologout floor): no one
/// command read may take longer to complete, and no one socket write of a
/// reply may wait longer to accept bytes — every write that moves bytes
/// starts a fresh window, so a reply has no whole-reply deadline and a
/// reader whose socket accepts some bytes of each next write within its
/// window gets a 50 MiB RETR in full (decision 0046
/// §§7–8, issue #131). Independent of SMTP's five minutes (decision 0036 §4); it
/// is never a session lease — a peer that completes a command, or keeps
/// accepting reply bytes, inside each window keeps its slot (0036 §5), and
/// its reply buffer with it. The cap's refusal stays one whole bounded
/// write.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Total POP3 sessions served, for the debug log correlation id.
static SESSIONS: AtomicU64 = AtomicU64::new(0);

// Installed around exactly one serve future; captured once at entry so
// another listener on the same runtime can never supply observation credit.
#[cfg(test)]
tokio::task_local! { static LISTENER_PROBE: Arc<ListenerProbe>; }

#[cfg(test)]
struct ListenerProbe {
    reaped: tokio::sync::watch::Sender<usize>,
    spawned: std::sync::Mutex<Vec<tokio::task::AbortHandle>>,
    parked: AtomicBool,
    accept_error: AtomicBool,
    backoff_pending: tokio::sync::watch::Sender<bool>,
    retry_ready: tokio::sync::watch::Sender<bool>,
}

#[cfg(test)]
impl ListenerProbe {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            reaped: tokio::sync::watch::channel(0).0,
            spawned: std::sync::Mutex::new(Vec::new()),
            parked: AtomicBool::new(false),
            accept_error: AtomicBool::new(false),
            backoff_pending: tokio::sync::watch::channel(false).0,
            retry_ready: tokio::sync::watch::channel(false).0,
        })
    }

    async fn reaped_until(&self, count: usize) -> usize {
        let mut rx = self.reaped.subscribe();
        while *rx.borrow_and_update() < count {
            rx.changed().await.unwrap();
        }
        *rx.borrow()
    }
}

// Only an installed test operand can hold retry readiness. Release builds
// retain the ordinary timer at the same select and the same shutdown owner.
#[cfg(test)]
async fn observed_backoff(probe: Option<&ListenerProbe>, timer: tokio::time::Sleep) {
    use std::future::Future;
    if let Some(probe) = probe {
        let mut rx = probe.retry_ready.subscribe();
        let mut ready = std::pin::pin!(rx.wait_for(|ready| *ready));
        std::future::poll_fn(|cx| {
            let polled = ready.as_mut().poll(cx);
            if polled.is_pending() {
                probe.backoff_pending.send_replace(true);
            }
            polled.map(|result| drop(result.unwrap()))
        })
        .await;
    }
    timer.await;
}

// Test-only: counted, one-shot accept errors (decision 0020 rule 4, the
// owner-local probe precedent; the SMTP sibling's seam). Thread-local because the
// loop is polled on the injecting test's own current-thread runtime.
#[cfg(test)]
thread_local! {
    static INJECTED_ACCEPT_ERRORS: core::cell::Cell<usize> =
        const { core::cell::Cell::new(0) };
}

/// Restores the prior thread-local error count when its scope ends.
#[cfg(test)]
#[must_use = "accept-error injections must be scoped"]
pub(crate) struct AcceptErrorInjection {
    previous: usize,
    _thread_bound: PhantomData<Rc<()>>,
}

#[cfg(test)]
impl Drop for AcceptErrorInjection {
    fn drop(&mut self) {
        INJECTED_ACCEPT_ERRORS.with(|errors| errors.set(self.previous));
    }
}

/// Install `count` failing accepts for the loop driven by this test.
#[cfg(test)]
pub(crate) fn inject_accept_errors(count: usize) -> AcceptErrorInjection {
    let previous = INJECTED_ACCEPT_ERRORS.with(|errors| errors.replace(count));
    AcceptErrorInjection {
        previous,
        _thread_bound: PhantomData,
    }
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

// Test-only observers of the accept loop (decision 0020 rule 4, the SMTP
// sibling's seams): whether its last accept poll parked, the flag value
// each error arm found, and every spawned session's abort handle — so a
// test can establish real task completion (`is_finished`, which EOF does
// not prove) while accept is parked. Observers only: none reaps, counts
// or admits.
#[cfg(test)]
thread_local! {
    static PARKED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static ERRORS_SEEN: core::cell::RefCell<Vec<bool>> =
        const { core::cell::RefCell::new(Vec::new()) };
    static SPAWNED: core::cell::RefCell<Vec<tokio::task::AbortHandle>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

// Test-only (decision 0020 rule 4, the SMTP sibling's seams): counted,
// one-shot `TCP_NODELAY` refusals, and the option each admitted socket
// carries once set, read back off the accepted socket (issue #126).
#[cfg(test)]
thread_local! {
    static INJECTED_NODELAY_ERRORS: core::cell::Cell<usize> =
        const { core::cell::Cell::new(0) };
    static NODELAY_SEEN: core::cell::RefCell<Vec<bool>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

/// Consume one installed `TCP_NODELAY` refusal, if any remain.
#[cfg(test)]
fn take_injected_nodelay_error() -> bool {
    INJECTED_NODELAY_ERRORS.with(|errors| {
        let left = errors.get();
        if left == 0 {
            false
        } else {
            errors.set(left - 1);
            true
        }
    })
}

/// `TCP_NODELAY` on one accepted POP3 socket (issue #126); the test build
/// can install a refusal and records the option the socket really carries.
fn set_nodelay(stream: &TcpStream) -> io::Result<()> {
    #[cfg(test)]
    if take_injected_nodelay_error() {
        return Err(io::Error::other("injected TCP_NODELAY error"));
    }
    stream.set_nodelay(true)?;
    #[cfg(test)]
    NODELAY_SEEN.with(|seen| seen.borrow_mut().push(stream.nodelay().unwrap()));
    Ok(())
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
    #[cfg(test)]
    let probe = LISTENER_PROBE.try_with(Arc::clone).ok();
    let mut shutdown = std::pin::pin!(shutdown);
    // Every session is owned by this loop: ready completions are reaped at
    // admission — after each successful accept, right before the cap check
    // (decision 0036 §1) — so the count the cap compares is never stale by
    // the sessions that finished while accept was parked; the set is
    // drained on the shutdown arm, so no session socket outlives the serve
    // loop. A session mid-transaction has no commit to land — the closed
    // socket is the only goodbye it gets.
    let mut sessions: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
    loop {
        // The accept itself, raced against the stop. The seam arm (test-only)
        // is checked before the listener on every poll, so the error arm
        // below is reachable and branch-stable; in production the poll is
        // just the accept's. A poll that parks is an observed healthy wait
        // (decision 0036 §2): it clears the flag — never the backoff clock,
        // so back-to-back failures stay degraded throughout.
        let accepted = tokio::select! {
            res = std::future::poll_fn(|cx| {
                #[cfg(test)]
                if probe.as_ref().is_some_and(|p| p.accept_error.swap(false, Ordering::Relaxed))
                    | take_injected_accept_error() {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "injected accept error",
                    )));
                }
                let polled = listener.poll_accept(cx);
                #[cfg(test)]
                {
                    PARKED.with(|parked| parked.set(polled.is_pending()));
                    if let Some(probe) = &probe {
                        probe.parked.store(polled.is_pending(), Ordering::Relaxed);
                    }
                }
                if polled.is_pending() {
                    degraded.store(false, Ordering::Relaxed);
                }
                polled
            }) => res,
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
                #[cfg_attr(not(test), allow(unused_variables))]
                let was_degraded = degraded.swap(true, Ordering::Relaxed);
                #[cfg(test)]
                ERRORS_SEEN.with(|seen| seen.borrow_mut().push(was_degraded));
                tracing::warn!(error = %e, "pop3 accept failed; retrying after the backoff");
                let backoff = tokio::time::sleep(crate::smtp::ACCEPT_BACKOFF);
                #[cfg(test)]
                let backoff = observed_backoff(probe.as_deref(), backoff);
                tokio::select! {
                    _ = &mut shutdown => {
                        sessions.abort_all();
                        while sessions.join_next().await.is_some() {}
                        return Ok(());
                    }
                    _ = backoff => {}
                }
                continue;
            }
        };
        // Every reply leaves at once (issue #126, decision 0046 §2): with
        // Nagle on, each reply behind the first in a pipelined group waits
        // for the peer's delayed ACK. Set before the cap's refusal and the
        // greeting; a socket that refuses the option is dropped alone.
        if let Err(e) = set_nodelay(&stream) {
            debug!(%peer, error = %e, "pop3 connection dropped: TCP_NODELAY refused");
            continue;
        }
        // The one reap, at admission (decision 0036 §1): `try_join_next`
        // never parks and never touches a session still running, so every
        // completion ready by now — including those that finished while
        // accept was parked — frees its slot for this very peer.
        while sessions.try_join_next().is_some() {
            #[cfg(test)]
            if let Some(probe) = &probe {
                probe.reaped.send_modify(|count| *count += 1);
            }
        }
        // Bounded sessions (decision 0027): the cap is checked against the
        // live set right after the reap. A refusal is wire-honest and
        // retryable, written under the same idle deadline before any
        // session state exists, and the socket is closed without spawning.
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
        #[cfg_attr(not(test), allow(unused_variables))]
        let handle = sessions.spawn(async move {
            if let Err(e) = handle_conn(stream, store, idle_timeout).await {
                debug!(session, error = %e, "pop3 connection ended");
            }
        });
        #[cfg(test)]
        {
            if let Some(probe) = &probe {
                probe.spawned.lock().unwrap().push(handle.clone());
            }
            SPAWNED.with(|spawned| spawned.borrow_mut().push(handle));
        }
    }
}

/// One maildrop entry, snapshotted oldest-first at authentication. `size`
/// is the unstuffed, CRLF-normalized payload the client retains after RETR
/// (decision 0037), counted by the writer's own traversal — never the
/// stored length. `gone` marks a
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
                // Exactly the delimiter goes — the LF above and one CR
                // before it — and the rest is strict UTF-8 (decision 0046
                // §5): an extra CR is data a USER name refuses, and invalid
                // bytes end the session undispatched instead of selecting
                // an inbox through a U+FFFD replacement (issue #128).
                if bytes.last() == Some(&b'\r') {
                    bytes.pop();
                }
                let line = String::from_utf8(bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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
        // `raw` is everything after the first space, untouched: USER names
        // an inbox by the one verbatim rule (decision 0046 §5), so a padded
        // name is the padded inbox. Every other verb reads its trimmed
        // `arg`, as it always has.
        let (verb, raw) = match line.split_once(' ') {
            Some((v, a)) => (v.to_ascii_uppercase(), a),
            None => (line.trim().to_ascii_uppercase(), ""),
        };
        let arg = raw.trim();

        // The state machine: a two-phase gate. The maildrop exists only in
        // the transaction phase; there, every command is answered from the
        // snapshot so numbering is stable mid-session.
        match (phase, verb.as_str()) {
            (Phase::Authorization, "USER") => {
                // A failing USER leaves no pending name (decision 0028 §2):
                // PASS afterwards answers "-ERR USER required first", never
                // logging into a mailbox a previous USER named.
                user = None;
                if raw.is_empty() {
                    write_line(&mut write, "-ERR missing username", idle).await?;
                } else if let Err(refusal) = crate::model::validate_inbox(raw) {
                    write_line(&mut write, &format!("-ERR {refusal}"), idle).await?;
                } else if !store.has_inbox(raw) {
                    write_line(&mut write, "-ERR no such mailbox", idle).await?;
                } else {
                    user = Some(raw.to_string());
                    write_line(&mut write, "+OK name is a valid mailbox", idle).await?;
                }
            }
            (Phase::Authorization, "PASS") => {
                let Some(name) = user.as_ref() else {
                    write_line(&mut write, "-ERR USER required first", idle).await?;
                    continue;
                };
                // Re-checked: the inbox may have been cleared since USER.
                if !store.has_inbox(name) {
                    write_line(&mut write, "-ERR no such mailbox", idle).await?;
                    continue;
                }
                // The whole snapshot — list, reverse, size every mail —
                // runs in one awaited job on the blocking pool (issue #86,
                // the 0030 precedent): it scans every byte of the maildrop,
                // and on the async worker it stalled every other socket.
                // Only metadata comes back.
                let job_store = store.clone();
                let inbox = name.clone();
                maildrop = Store::offload(move || snapshot(&job_store, &inbox)).await;
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
                    // Through the progress writer (issue #71, decision 0046
                    // §7): a peer that stops reading ends the session
                    // silently through `?` and frees its slot.
                    write_reply(&mut write, out.as_bytes(), idle).await?;
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
                    // Bounded like LIST (issue #71).
                    write_reply(&mut write, out.as_bytes(), idle).await?;
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
                            let out = retr_reply(msg.size, &email.raw);
                            write_reply(&mut write, &out, idle).await?;
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
                            write_reply(&mut write, &out, idle).await?;
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
                    // Removed elsewhere since the snapshot: DELE answers
                    // exactly as RETR does (decision 0028 §3, issue #86),
                    // and adds no mark. A check, not a lock — a removal
                    // after it is absorbed by QUIT's idempotent delete.
                    Ok(msg) if store.get(&msg.id).is_none() => {
                        write_line(&mut write, "-ERR no such message", idle).await?
                    }
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

/// The maildrop snapshot, oldest-first (`store.list` is newest-first),
/// each mail sized by its unstuffed payload (decision 0037) — never
/// `e.size`, a second copy of a fact the traversal already states. PASS
/// runs this off the async worker (issue #86); a maildrop exists iff its
/// inbox does (`Store::has_inbox`, decision 0004), checked by the caller.
fn snapshot(store: &Store, inbox: &str) -> Vec<MaildropMsg> {
    #[cfg(test)]
    store.run_snapshot_gate();
    store
        .list(inbox, &Filter::default())
        .into_iter()
        .rev()
        .map(|e| MaildropMsg {
            id: e.id.clone(),
            size: payload_len(&e.raw),
            gone: false,
        })
        .collect()
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

/// Test-only: one socket write the progress writer made — when it was
/// entered, when it returned and how many bytes the socket took.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct WriteSeen {
    pub(crate) entered: std::time::Instant,
    pub(crate) done: std::time::Instant,
    pub(crate) moved: usize,
}

// Test-only observer (decision 0020 rule 4, issue #131): every reply the
// progress writer sent on this thread, as the real socket writes it took.
// It records and never shapes — each byte still goes through the transport
// — so a test can see a reply span windows at the server, not infer it
// from how long its own reader took.
#[cfg(test)]
thread_local! {
    pub(crate) static REPLY_WRITES: core::cell::RefCell<Vec<Vec<WriteSeen>>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

/// One whole reply, bounded by progress (issue #131, decision 0046 §7):
/// each write of what remains gets its own idle window, and every write
/// that moves bytes earns the next one — so a reader whose socket accepts
/// some bytes of each next write within its window receives a RETR of any
/// size in full, while one that takes nothing
/// for a whole window is cut off silently through the caller's `?` (a
/// TimedOut is an ended session, not a pinned task). Progress is bytes the
/// local socket accepted, not bytes the peer's application consumed, and
/// there is no total-duration or minimum-rate bound. A write of zero bytes
/// is `WriteZero`. A reply that fits the socket is still one application
/// write (issue #83).
async fn write_reply<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    mut out: &[u8],
    idle: Duration,
) -> io::Result<()> {
    #[cfg(test)]
    REPLY_WRITES.with(|replies| replies.borrow_mut().push(Vec::new()));
    while !out.is_empty() {
        #[cfg(test)]
        let entered = std::time::Instant::now();
        let n = crate::smtp::bound_io(w.write(out), idle).await?;
        if n == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        #[cfg(test)]
        REPLY_WRITES.with(|replies| {
            let seen = WriteSeen {
                entered,
                done: std::time::Instant::now(),
                moved: n,
            };
            replies.borrow_mut().last_mut().unwrap().push(seen);
        });
        out = &out[n..];
    }
    Ok(())
}

/// One reply line, CRLF-terminated, through the progress writer: a peer
/// that stops accepting replies ends the session silently through the
/// existing `?`. Text and CRLF are offered together (issue #83): a separate
/// 2-byte CRLF write waits on Nagle and the peer's delayed ACK, about 40 ms
/// a reply. One application write — not a promise of one syscall or
/// segment.
async fn write_line<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    line: &str,
    idle: Duration,
) -> io::Result<()> {
    let mut out = Vec::with_capacity(line.len() + 2);
    out.extend_from_slice(line.as_bytes());
    out.extend_from_slice(b"\r\n");
    write_reply(w, &out, idle).await
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
    write_reply(w, out.as_bytes(), idle).await
}

/// Append `raw` to `out` as RFC 1939 multi-line data: every line becomes
/// CRLF-terminated (a bare LF and a missing final newline are normalized;
/// a bare CR mid-line passes through — it is not a line ending) and any
/// line starting with "." gets a "." stuffed in front.
fn write_stuffed(out: &mut Vec<u8>, raw: &[u8]) {
    write_stuffed_limited(out, raw, usize::MAX);
}

/// One whole RETR reply: the status line, the stuffed mail and the final
/// `.\r\n`. The count is the snapshot's: the unstuffed, normalized payload
/// LIST and STAT advertise (decision 0037) — the mail under an id never
/// changes. The reply is the one extra copy of the mail a session holds
/// (issue #86): the stuffed body goes straight into it from the live
/// bytes, never through a second buffer — the allocation harness
/// (`pop3/alloc_tests.rs`, issue #143) pins exactly that.
fn retr_reply(size: u64, raw: &[u8]) -> Vec<u8> {
    let mut out = format!("+OK {size} octets\r\n").into_bytes();
    write_stuffed(&mut out, raw);
    out.extend_from_slice(b".\r\n");
    out
}

/// The advertised size of a stored mail: its payload as the client keeps
/// it — CRLF-normalized, without the transparency dots RETR adds and the
/// client strips, excluding the final `.\r\n` (RFC 1939 §11, decision 0037
/// amending 0028 §1). The writer's own traversal with a counting sink: no
/// mirrored walk of its rules, and no buffer — sizing a maildrop copies
/// no mail (issue #86).
fn payload_len(raw: &[u8]) -> u64 {
    let mut n: u64 = 0;
    stuff_lines(raw, usize::MAX, |bytes, dot| {
        if !dot {
            n += bytes.len() as u64;
        }
    });
    n
}

/// The first `max_lines` lines of `raw`, dot-stuffed; the terminating "."
/// line is the caller's.
fn write_stuffed_limited(out: &mut Vec<u8>, raw: &[u8], max_lines: usize) {
    stuff_lines(raw, max_lines, |bytes, _dot| out.extend_from_slice(bytes));
}

/// The one RFC 1939 multi-line traversal, shared by the writer and the
/// counter: each of the first `max_lines` lines is emitted as its content
/// and a CRLF (a bare LF and a missing final newline are normalized; a
/// bare CR mid-line passes through), preceded by a transparency `.` —
/// flagged `true` — when the line starts with one.
fn stuff_lines(raw: &[u8], mut max_lines: usize, mut emit: impl FnMut(&[u8], bool)) {
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
            emit(&b"."[..], true);
        }
        emit(line, false);
        emit(&b"\r\n"[..], false);
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

    /// The real POP3 owner, its held backoff and the real HTTP router share
    /// one health flag. All assertions are stable positive observations;
    /// no failure-only diagnostic/cleanup arm needs a 0020 harness.
    #[tokio::test]
    async fn pop3_degradation_reaches_http_healthz() {
        use tokio::io::AsyncReadExt;

        // Always release and abort on drop, including assertion unwind or
        // the whole-scenario deadline. Success explicitly stops and joins.
        struct Cleanup {
            probe: Arc<ListenerProbe>,
            pop3: tokio::task::AbortHandle,
            http: tokio::task::AbortHandle,
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                self.probe.retry_ready.send_replace(true);
                self.pop3.abort();
                self.http.abort();
            }
        }
        async fn health(addr: std::net::SocketAddr) -> (String, serde_json::Value) {
            let mut client = TcpStream::connect(addr).await.unwrap();
            client
                .write_all(
                    format!("GET /healthz HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .await
                .unwrap();
            let mut bytes = Vec::new();
            client.read_to_end(&mut bytes).await.unwrap();
            let reply = String::from_utf8(bytes).unwrap();
            let (head, body) = reply.split_once("\r\n\r\n").unwrap();
            (
                head.lines().next().unwrap().to_string(),
                serde_json::from_str(body).unwrap(),
            )
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let http_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_addr = http_listener.local_addr().unwrap();
        let store = Arc::new(Store::new(0));
        store
            .insert(crate::store::email("health-message", "health", "a@x.io"))
            .unwrap();
        let probe = ListenerProbe::new();
        let degraded = Arc::new(AtomicBool::new(false));
        let (stop, mut pop3_stop) = tokio::sync::watch::channel(false);
        let mut http_stop = pop3_stop.clone();
        let state = crate::api::AppState {
            store: store.clone(),
            chaos: Arc::new(crate::chaos::Chaos::default()),
            webhooks: Arc::new(crate::webhook::Webhooks::default()),
            started: std::time::Instant::now(),
            shutdown: pop3_stop.clone(),
            smtp_degraded: Arc::new(AtomicBool::new(false)),
            pop3_degraded: degraded.clone(),
            http_origin_guard: Some(http_addr.port()),
        };
        let mut pop3 = tokio::spawn(LISTENER_PROBE.scope(
            probe.clone(),
            serve(
                listener,
                store,
                degraded,
                Duration::from_secs(60),
                2,
                async move {
                    pop3_stop.changed().await.unwrap();
                },
            ),
        ));
        let mut http = tokio::spawn(crate::api::serve(http_listener, state, async move {
            http_stop.changed().await.unwrap();
        }));
        let cleanup = Cleanup {
            probe: probe.clone(),
            pop3: pop3.abort_handle(),
            http: http.abort_handle(),
        };
        tokio::time::timeout(Duration::from_secs(30), async {
            let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
            let mut line = String::new();
            client.read_line(&mut line).await.unwrap();
            assert_eq!(line, "+OK Swarmail POP3 server ready\r\n");
            client.write_all(b"USER health\r\n").await.unwrap();
            line.clear();
            client.read_line(&mut line).await.unwrap();
            assert_eq!(line, "+OK name is a valid mailbox\r\n");
            client.write_all(b"PASS x\r\n").await.unwrap();
            line.clear();
            client.read_line(&mut line).await.unwrap();
            assert_eq!(line, "+OK maildrop has 1 messages (0 octets)\r\n");
            client.write_all(b"NOOP\r\n").await.unwrap();
            line.clear();
            client.read_line(&mut line).await.unwrap();
            assert_eq!(line, "+OK\r\n");
            let (status, baseline) = health(http_addr).await;
            assert_eq!(status, "HTTP/1.1 200 OK");
            assert_eq!(baseline["status"], "ok");
            assert!(baseline.get("degraded").is_none());

            let mut entered = probe.backoff_pending.subscribe();
            assert!(!*probe.retry_ready.borrow());
            probe.accept_error.store(true, Ordering::Relaxed);
            // A real connection wakes this owner's pending accept poll.
            let poke = TcpStream::connect(addr).await.unwrap();
            entered.wait_for(|pending| *pending).await.unwrap();
            let (status, body) = health(http_addr).await;
            assert_eq!(status, "HTTP/1.1 503 Service Unavailable");
            assert_eq!(
                body,
                serde_json::json!({"status":"degraded","degraded":["pop3"]})
            );
            drop(poke);
            drop(client);
            probe.retry_ready.send_replace(true);
            stop.send(true).unwrap();
            (&mut pop3).await.unwrap().unwrap();
            (&mut http).await.unwrap().unwrap();
        })
        .await
        .expect("POP3 health scenario must finish, including owned task joins");
        drop(cleanup);
    }

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

    /// One injected accept failure with nothing queued: the loop flags
    /// itself degraded, backs off, and clears the flag when the next accept
    /// poll parks — before any peer connects (decision 0036 §2; red on the
    /// success-only clear, which leaves an idle recovered listener degraded
    /// forever). Only then does a real session prove the listener speaks.
    #[tokio::test]
    async fn one_failed_accept_clears_on_the_parked_retry_then_serves() {
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
        let errors = inject_accept_errors(1);

        flag_becomes(&degraded, true, "flagged degraded on the failed accept").await;
        // No peer yet: the parked retry alone is the recovery.
        flag_becomes(&degraded, false, "cleared the flag on the parked retry").await;
        let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert!(
            line.starts_with(b"+OK"),
            "the listener never recovered: {line:?}"
        );
        server.abort();
        let _ = server.await;
        drop(errors);
    }

    /// Back-to-back failures publish no healthy moment between them: the
    /// second error arm finds the flag still set — the backoff clock alone
    /// never clears it (decision 0036 §2; red on a clear before the poll or
    /// at the backoff's end). The parked third poll then clears it, and a
    /// real session follows.
    #[tokio::test]
    async fn successive_failed_accepts_stay_degraded_across_the_retry() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        ERRORS_SEEN.with(|seen| seen.borrow_mut().clear());
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            degraded.clone(),
            Duration::from_secs(60),
            256,
            std::future::pending(),
        ));
        let errors = inject_accept_errors(2);

        // The loop has not been polled yet, so the wait is always entered.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while ERRORS_SEEN.with(|seen| seen.borrow().len()) < 2 {
            assert!(
                std::time::Instant::now() < deadline,
                "the accept loop never took both injected errors"
            );
            tokio::task::yield_now().await;
        }
        assert_eq!(
            ERRORS_SEEN.with(|seen| seen.borrow().clone()),
            vec![false, true],
            "the retry boundary published a healthy flag between failures"
        );
        flag_becomes(&degraded, false, "cleared the flag on the parked retry").await;
        let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert!(line.starts_with(b"+OK"), "{line:?}");
        server.abort();
        let _ = server.await;
        drop(errors);
    }

    /// A greeted POP3 session on a driven listener — the greeting is written
    /// by the session task, so reading it proves the spawn.
    async fn greeted(addr: std::net::SocketAddr) -> BufReader<TcpStream> {
        let mut client = BufReader::new(TcpStream::connect(addr).await.unwrap());
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert_eq!(
            line, b"+OK Swarmail POP3 server ready\r\n",
            "first offered peer must be admitted"
        );
        client
    }

    /// Tokio's own completion flag for the session spawned `index`-th on
    /// this thread (a peer's EOF does not prove it), and-ed — never
    /// short-circuited — with the accept loop being parked.
    fn finished_while_parked(index: usize) -> bool {
        SPAWNED.with(|spawned| spawned.borrow()[index].is_finished())
            & PARKED.with(|parked| parked.get())
    }

    /// `QUIT` the session spawned `index`-th, then wait until its task has
    /// really finished while the accept loop is parked — the completion
    /// lands with nothing to wake the loop, so no reap can run before the
    /// next peer arrives. A real deadline, never a sleep; the first check
    /// precedes any yield, so the session cannot have run yet and the wait
    /// is entered on every call.
    async fn quit_finished_while_parked(client: &mut BufReader<TcpStream>, index: usize) {
        client.write_all(b"QUIT\r\n").await.unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !finished_while_parked(index) {
            assert!(
                std::time::Instant::now() < deadline,
                "session {index} never finished while accept was parked"
            );
            tokio::task::yield_now().await;
        }
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert_eq!(line, b"+OK Swarmail POP3 server signing off\r\n");
    }

    /// Admission reaps (decision 0036 §1): a genuinely full cap refuses
    /// exactly, and a slot freed while accept is parked — one session, then
    /// a whole full burst — admits the very next peer, offered once (red on
    /// the loop-head reap, whose stale count refuses it).
    #[tokio::test]
    async fn a_slot_freed_while_accept_is_parked_admits_the_next_peer() {
        use tokio::io::AsyncReadExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        SPAWNED.with(|spawned| spawned.borrow_mut().clear());
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(60),
            2,
            std::future::pending(),
        ));
        let mut first = greeted(addr).await; // spawned 0
        let mut second = greeted(addr).await; // spawned 1

        // Both sessions live: the cap is genuinely full.
        let mut refused = TcpStream::connect(addr).await.unwrap();
        let mut wire = Vec::new();
        refused.read_to_end(&mut wire).await.unwrap();
        assert_eq!(wire, b"-ERR too many connections\r\n");

        // One slot frees while accept is parked.
        quit_finished_while_parked(&mut first, 0).await;
        let mut third = greeted(addr).await; // spawned 2

        // The whole full burst completes while accept is parked.
        quit_finished_while_parked(&mut second, 1).await;
        quit_finished_while_parked(&mut third, 2).await;
        let _fourth = greeted(addr).await;
        let _fifth = greeted(addr).await;
        server.abort();
    }

    /// Stop must drain the actual owner while retry readiness is held.
    /// A second case releases readiness and exercises retry and its timer.
    #[tokio::test]
    async fn a_stop_inside_a_backoff_resolves_well_inside_it() {
        use tokio::io::AsyncReadExt;
        for release_before_stop in [false, true] {
            let (addr, probe, stop, server) = observed_listener().await;
            let mut client = greeted(addr).await;
            probe.accept_error.store(true, Ordering::Relaxed);
            let mut poke = BufReader::new(TcpStream::connect(addr).await.unwrap());
            let mut entered = probe.backoff_pending.subscribe();
            tokio::time::timeout(
                Duration::from_secs(30),
                entered.wait_for(|entered| *entered),
            )
            .await
            .expect("owner never polled held backoff Pending")
            .unwrap();
            assert!(!*probe.retry_ready.borrow());
            assert!(!probe.spawned.lock().unwrap()[0].is_finished());
            if release_before_stop {
                probe.retry_ready.send_replace(true);
                let mut line = String::new();
                poke.read_line(&mut line).await.unwrap();
                assert!(line.starts_with("+OK "));
            }
            stop.send(true).unwrap();
            let stopped = tokio::time::timeout(Duration::from_secs(30), async {
                while !server.is_finished() {
                    tokio::task::yield_now().await;
                }
            })
            .await;
            // Failure cleanup releases the latch and aborts/joins this
            // owner before asserting the stop-before-release observation.
            let readiness_at_stop = *probe.retry_ready.borrow();
            probe.retry_ready.send_replace(true);
            server.abort();
            let outcome = server.await;
            stopped.expect("stop did not finish before retry release");
            outcome.unwrap().unwrap();
            assert_eq!(readiness_at_stop, release_before_stop);
            assert!(
                probe
                    .spawned
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|handle| handle.is_finished())
            );
            let mut tail = Vec::new();
            assert_eq!(client.read_to_end(&mut tail).await.unwrap(), 0);
        }
    }

    #[tokio::test]
    async fn accept_error_injection_restores_every_scope_exit() {
        fn current() -> usize {
            INJECTED_ACCEPT_ERRORS.with(core::cell::Cell::get)
        }
        fn return_early(early: bool) {
            let _errors = inject_accept_errors(4);
            if early {
                return;
            }
            assert_eq!(current(), 4);
        }

        assert_eq!(current(), 0);
        {
            let _errors = inject_accept_errors(3);
            assert_eq!(current(), 3);
        }
        assert_eq!(current(), 0);
        return_early(true);
        assert_eq!(current(), 0);
        return_early(false);
        assert_eq!(current(), 0);

        let outer = inject_accept_errors(5);
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = inject_accept_errors(11);
            panic!("controlled accept-error injection unwind");
        }));
        assert!(unwound.is_err());
        assert_eq!(current(), 5);
        drop(outer);
        assert_eq!(current(), 0);

        let previous = inject_accept_errors(7);
        let mut pending = Box::pin({
            let mut injection = None;
            std::future::poll_fn(move |_| {
                if injection.is_none() {
                    injection = Some(inject_accept_errors(13));
                }
                Poll::<()>::Pending
            })
        });
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        assert!(std::future::Future::poll(pending.as_mut(), &mut cx).is_pending());
        assert_eq!(current(), 13);
        assert!(std::future::Future::poll(pending.as_mut(), &mut cx).is_pending());
        assert_eq!(current(), 13);
        drop(pending);
        assert_eq!(current(), 7);
        drop(previous);
        assert_eq!(current(), 0);

        ERRORS_SEEN.with(|seen| seen.borrow_mut().clear());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        let (stop, mut rx) = tokio::sync::watch::channel(false);
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            degraded.clone(),
            Duration::from_secs(60),
            2,
            async move {
                let _ = rx.changed().await;
            },
        ));
        let mut client = tokio::time::timeout(Duration::from_secs(5), greeted(addr))
            .await
            .expect("the same-thread listener did not greet");
        assert!(ERRORS_SEEN.with(|seen| seen.borrow().is_empty()));
        assert!(!degraded.load(Ordering::Relaxed));
        client.write_all(b"QUIT\r\n").await.unwrap();
        let goodbye = tokio::time::timeout(Duration::from_secs(5), async {
            let mut line = String::new();
            client.read_line(&mut line).await.unwrap();
            line
        })
        .await
        .expect("POP3 goodbye exceeded its bound");
        assert_eq!(goodbye, "+OK Swarmail POP3 server signing off\r\n");
        stop.send(true).unwrap();
        server.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn a_pop3_session_keeps_its_snapshot_during_accept_error_backoff() {
        async fn line(client: &mut BufReader<TcpStream>) -> String {
            tokio::time::timeout(Duration::from_secs(5), async {
                let mut line = String::new();
                client.read_line(&mut line).await.unwrap();
                line
            })
            .await
            .expect("POP3 reply exceeded its bound")
        }
        async fn command(client: &mut BufReader<TcpStream>, value: &str) -> String {
            tokio::time::timeout(Duration::from_secs(5), async {
                client.write_all(value.as_bytes()).await.unwrap();
                line(client).await
            })
            .await
            .expect("POP3 command/reply exceeded its bound")
        }

        let store = Arc::new(Store::new(0));
        let mut seed = crate::store::email("held-pop3", "held-pop3", "recipient@example.com");
        seed.raw = b"Subject: held\r\n\r\nheld body\r\n".to_vec();
        let expected_size = seed.raw.len();
        store.insert(seed).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let degraded = Arc::new(AtomicBool::new(false));
        let probe = ListenerProbe::new();
        let (stop, mut rx) = tokio::sync::watch::channel(false);
        let server = tokio::spawn(LISTENER_PROBE.scope(
            probe.clone(),
            serve(
                listener,
                store,
                degraded.clone(),
                Duration::from_secs(60),
                2,
                async move {
                    let _ = rx.changed().await;
                },
            ),
        ));

        let mut client = tokio::time::timeout(Duration::from_secs(5), greeted(addr))
            .await
            .expect("POP3 session did not greet");
        assert_eq!(
            command(&mut client, "USER held-pop3\r\n").await,
            "+OK name is a valid mailbox\r\n"
        );
        assert_eq!(
            command(&mut client, "PASS pw\r\n").await,
            format!("+OK maildrop has 1 messages ({expected_size} octets)\r\n")
        );

        let mut entered = probe.backoff_pending.subscribe();
        probe.accept_error.store(true, Ordering::Relaxed);
        let mut retry = BufReader::new(
            tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr))
                .await
                .expect("retry connection did not complete")
                .unwrap(),
        );
        tokio::time::timeout(Duration::from_secs(5), entered.wait_for(|pending| *pending))
            .await
            .expect("POP3 accept loop did not enter the observed backoff")
            .unwrap();
        assert!(degraded.load(Ordering::Relaxed));
        assert!(!*probe.retry_ready.borrow());

        assert_eq!(
            command(&mut client, "STAT\r\n").await,
            format!("+OK 1 {expected_size}\r\n")
        );
        let listing = tokio::time::timeout(Duration::from_secs(5), async {
            client.write_all(b"LIST\r\n").await.unwrap();
            (
                line(&mut client).await,
                line(&mut client).await,
                line(&mut client).await,
            )
        })
        .await
        .expect("POP3 LIST exceeded its bound");
        assert!(listing.0.starts_with("+OK 1 messages"), "{:?}", listing.0);
        assert_eq!(listing.1, format!("1 {expected_size}\r\n"));
        assert_eq!(listing.2, ".\r\n");

        let retrieved = tokio::time::timeout(Duration::from_secs(5), async {
            client.write_all(b"RETR 1\r\n").await.unwrap();
            let status = line(&mut client).await;
            let mut body = String::new();
            loop {
                let next = line(&mut client).await;
                if next == ".\r\n" {
                    break;
                }
                body.push_str(&next);
            }
            (status, body)
        })
        .await
        .expect("POP3 RETR exceeded its bound");
        assert!(retrieved.0.starts_with("+OK"), "{:?}", retrieved.0);
        assert_eq!(retrieved.1, "Subject: held\r\n\r\nheld body\r\n");

        probe.retry_ready.send_replace(true);
        assert_eq!(line(&mut retry).await, "+OK Swarmail POP3 server ready\r\n");
        assert!(!degraded.load(Ordering::Relaxed));
        assert_eq!(
            command(&mut client, "QUIT\r\n").await,
            "+OK Swarmail POP3 server signing off\r\n"
        );
        assert_eq!(
            command(&mut retry, "QUIT\r\n").await,
            "+OK Swarmail POP3 server signing off\r\n"
        );
        stop.send(true).unwrap();
        server.await.unwrap().unwrap();
    }

    /// One probe, one actual serve owner, one JoinSet for every batch.
    async fn observed_listener() -> (
        std::net::SocketAddr,
        Arc<ListenerProbe>,
        tokio::sync::watch::Sender<bool>,
        tokio::task::JoinHandle<io::Result<()>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let probe = ListenerProbe::new();
        let (stop, mut rx) = tokio::sync::watch::channel(false);
        let server = tokio::spawn(LISTENER_PROBE.scope(
            probe.clone(),
            serve(
                listener,
                Arc::new(Store::new(0)),
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(60),
                2,
                async move {
                    let _ = rx.changed().await;
                },
            ),
        ));
        (addr, probe, stop, server)
    }

    async fn owner_quit(client: &mut BufReader<TcpStream>, index: usize, probe: &ListenerProbe) {
        client.write_all(b"QUIT\r\n").await.unwrap();
        tokio::time::timeout(Duration::from_secs(30), async {
            while !(probe.spawned.lock().unwrap()[index].is_finished()
                & probe.parked.load(Ordering::Relaxed))
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("owner session never finished while its accept was parked");
        let mut line = String::new();
        client.read_line(&mut line).await.unwrap();
        assert_eq!(line, "+OK Swarmail POP3 server signing off\r\n");
    }

    /// Both waits are deliberately polled Pending before any completion,
    /// then Ready after harvest by this owner's single offered admission.
    #[tokio::test]
    async fn the_accept_loop_reaps_completed_sessions_across_batches() {
        use std::future::Future;
        use tokio::io::AsyncReadExt;
        let (addr, probe, stop, server) = observed_listener().await;
        let mut first = greeted(addr).await;
        let mut second = greeted(addr).await;
        let mut refused = TcpStream::connect(addr).await.unwrap();
        let mut wire = Vec::new();
        refused.read_to_end(&mut wire).await.unwrap();
        assert_eq!(wire, b"-ERR too many connections\r\n");
        assert_eq!(
            *probe.reaped.borrow(),
            0,
            "live sessions must not be harvested"
        );
        let mut first_wait = std::pin::pin!(probe.reaped_until(1));
        assert!(
            first_wait
                .as_mut()
                .poll(&mut std::task::Context::from_waker(
                    futures_util::task::noop_waker_ref()
                ))
                .is_pending()
        );

        // A second real owner harvests while the first is held full. This
        // must not satisfy even part of the first owner's pending wait.
        let (other_addr, other, other_stop, other_server) = observed_listener().await;
        let mut unrelated = greeted(other_addr).await;
        owner_quit(&mut unrelated, 0, &other).await;
        let _other_live = greeted(other_addr).await;
        assert_eq!(
            *other.reaped.borrow(),
            1,
            "isolation owner must harvest its completed session at admission"
        );
        assert_eq!(
            *probe.reaped.borrow(),
            0,
            "unrelated owner supplied harvest credit"
        );
        assert!(
            first_wait
                .as_mut()
                .poll(&mut std::task::Context::from_waker(
                    futures_util::task::noop_waker_ref()
                ))
                .is_pending()
        );
        other_stop.send(true).unwrap();
        other_server.await.unwrap().unwrap();

        owner_quit(&mut first, 0, &probe).await;
        let mut third = greeted(addr).await;
        assert_eq!(
            *probe.reaped.borrow(),
            1,
            "first admission must harvest exactly one"
        );
        assert_eq!(first_wait.await, 1);

        let mut second_wait = std::pin::pin!(probe.reaped_until(3));
        assert!(
            second_wait
                .as_mut()
                .poll(&mut std::task::Context::from_waker(
                    futures_util::task::noop_waker_ref()
                ))
                .is_pending()
        );
        owner_quit(&mut second, 1, &probe).await;
        owner_quit(&mut third, 2, &probe).await;
        let _fourth = greeted(addr).await;
        assert_eq!(
            *probe.reaped.borrow(),
            3,
            "second admission must harvest both completions"
        );
        assert_eq!(second_wait.await, 3);
        stop.send(true).unwrap();
        server.await.unwrap().unwrap();
        assert!(
            probe
                .spawned
                .lock()
                .unwrap()
                .iter()
                .all(|handle| handle.is_finished())
        );
    }

    /// The advertised size is the normalized payload without the added
    /// dots (decision 0037; red on the dot-inclusive count, which says 33
    /// for the first fixture). Literal expected sizes, not a second walk.
    #[test]
    fn payload_len_counts_normalized_bytes_without_transparency_dots() {
        let dotted: &[u8] = b".123456\r\n.123456\r\n.123456789\r\n";
        assert_eq!(payload_len(dotted), 30);
        let mut wire = Vec::new();
        write_stuffed(&mut wire, dotted);
        assert_eq!(wire.len(), 33, "the wire still carries the three dots");
        // Bare LF gains a CR; a bare CR mid-line is content.
        assert_eq!(payload_len(b"a\nb\n"), 6);
        assert_eq!(payload_len(b"a\rb\r\n"), 5);
        // Mixed endings, then a missing final newline.
        assert_eq!(payload_len(b"a\r\nb\nc"), 9);
        assert_eq!(payload_len(b""), 0);
        // 8-bit bytes count as themselves.
        assert_eq!(payload_len(b"\xff\xfe\n"), 4);
    }

    /// A single-line reply is one write of text plus CRLF (issue #83; red
    /// on the text-then-CRLF shape). POP3 adds no flush: plain TCP only.
    #[tokio::test]
    async fn a_reply_line_is_one_write() {
        let mut rec = crate::smtp::Recorder::default();
        write_line(&mut rec, "+OK", Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(
            rec.ops,
            vec![crate::smtp::WriteOp::Write(b"+OK\r\n".to_vec())]
        );
    }

    /// A peer that stops reading a multi-line reply is cut off at the idle
    /// deadline and its slot is admitted again (issue #71; red on the
    /// unbounded write, whose session never finishes). One slot, a 1 KiB
    /// receive buffer and a thousand pipelined commands whose replies far
    /// outrun every socket buffer: the server blocks in the write, never in
    /// a read. The session's completion is Tokio's own flag, spun against a
    /// real deadline; its first check precedes any yield, so the wait is
    /// always entered. The peer then finds no idle goodbye in what it got —
    /// the write, not a read, expired.
    async fn a_stalled_reply_frees_the_slot(command: &str) {
        use tokio::io::AsyncReadExt;

        let store = Arc::new(Store::new(0));
        for i in 0..2000 {
            let _ = store.insert(crate::store::email(&format!("m{i:05}"), "pop", "a@x.io"));
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        SPAWNED.with(|spawned| spawned.borrow_mut().clear());
        let server = tokio::spawn(serve(
            listener,
            store,
            Arc::new(AtomicBool::new(false)),
            Duration::from_millis(200),
            1,
            std::future::pending(),
        ));
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.set_recv_buffer_size(1024).unwrap();
        let mut client = BufReader::new(socket.connect(addr).await.unwrap());
        let mut line = Vec::new();
        client.read_until(b'\n', &mut line).await.unwrap();
        assert!(line.starts_with(b"+OK"), "{line:?}");
        client.write_all(b"USER pop\r\nPASS x\r\n").await.unwrap();
        for _ in 0..2 {
            let mut line = Vec::new();
            client.read_until(b'\n', &mut line).await.unwrap();
            assert!(line.starts_with(b"+OK"), "{line:?}");
        }
        client
            .write_all(format!("{command}\r\n").repeat(1000).as_bytes())
            .await
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !finished_while_parked(0) {
            assert!(
                std::time::Instant::now() < deadline,
                "the stalled {command} writer never released its slot"
            );
            tokio::task::yield_now().await;
        }
        let _next = greeted(addr).await;
        // A reset is as good an ending as EOF here; only the tail matters.
        let mut got = Vec::new();
        let _ = client.read_to_end(&mut got).await;
        assert!(!got.ends_with(b"-ERR idle timeout\r\n"));
        server.abort();
    }

    #[tokio::test]
    async fn a_stalled_list_reply_frees_the_slot() {
        a_stalled_reply_frees_the_slot("LIST").await;
    }

    #[tokio::test]
    async fn a_stalled_uidl_reply_frees_the_slot() {
        a_stalled_reply_frees_the_slot("UIDL").await;
    }

    /// The progress writer refuses a writer that takes nothing (issue
    /// #131, decision 0046 §7): a 4-byte sink takes the reply's first four
    /// bytes on the first write, the offset advances, and the next write's
    /// zero is `WriteZero` — never a silent spin.
    #[tokio::test]
    async fn the_progress_writer_advances_partial_writes_and_refuses_zero() {
        let mut room = [0u8; 4];
        let mut sink = std::io::Cursor::new(&mut room[..]);
        let err = write_reply(&mut sink, b"+OK hello\r\n", Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WriteZero);
        assert_eq!(&room, b"+OK ");
    }

    /// The progress writer's other endings, on a real in-process pipe
    /// (issue #131, decision 0046 §7): an empty reply makes no write; a
    /// pipe with room for four bytes takes exactly four — the partial write
    /// the observer records — and the next write, with nobody reading, is
    /// cut off at the idle deadline as `TimedOut`; once the far end is gone
    /// the write's own error comes back unchanged.
    #[tokio::test]
    async fn the_progress_writer_ends_on_expiry_and_error() {
        use tokio::io::AsyncReadExt;

        let idle = Duration::from_millis(50);
        let (mut near, mut far) = tokio::io::duplex(4);
        REPLY_WRITES.with(|replies| replies.borrow_mut().clear());
        write_reply(&mut near, b"", idle).await.unwrap();
        let expired = write_reply(&mut near, b"+OK hello\r\n", idle)
            .await
            .unwrap_err();
        assert_eq!(expired.kind(), io::ErrorKind::TimedOut);
        let moved: Vec<Vec<usize>> = REPLY_WRITES.with(|replies| {
            replies
                .borrow()
                .iter()
                .map(|reply| reply.iter().map(|write| write.moved).collect())
                .collect()
        });
        assert_eq!(moved, vec![vec![], vec![4]]);
        let mut taken = [0u8; 4];
        far.read_exact(&mut taken).await.unwrap();
        assert_eq!(&taken, b"+OK ");
        drop(far);
        let broken = write_reply(&mut near, b"x", idle).await.unwrap_err();
        assert_eq!(broken.kind(), io::ErrorKind::BrokenPipe);
    }

    /// Every accepted POP3 socket carries `TCP_NODELAY` before its greeting
    /// (issue #126, AC3), read back off the accepted socket; an installed
    /// refusal drops exactly that socket and the next peer is served.
    #[tokio::test]
    async fn pop3_accepted_socket_nodelay() {
        use tokio::io::AsyncReadExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        NODELAY_SEEN.with(|seen| seen.borrow_mut().clear());
        let server = tokio::spawn(serve(
            listener,
            Arc::new(Store::new(0)),
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(60),
            256,
            std::future::pending(),
        ));
        INJECTED_NODELAY_ERRORS.with(|errors| errors.set(1));

        let mut dropped = TcpStream::connect(addr).await.unwrap();
        let mut wire = Vec::new();
        dropped.read_to_end(&mut wire).await.unwrap();
        assert!(wire.is_empty(), "no byte before the drop: {wire:?}");
        let _served = greeted(addr).await;
        assert_eq!(NODELAY_SEEN.with(|seen| seen.borrow().clone()), vec![true]);
        server.abort();
    }
}

#[cfg(test)]
// The PASS offload proof lives in src/pop3/maildrop_tests.rs (decision 0020
// rule 2): its cold path — the independent release thread that frees a
// snapshot gate an inline snapshot would hold forever on the sole worker —
// runs only while the bug is present, so it is unwritable inline.
mod maildrop_tests;

#[cfg(test)]
// The RETR progress proof lives in src/pop3/progress_tests.rs (decision
// 0020 rule 2): its diagnostics — a reply that fit one window, a write that
// never waited on backpressure, a reader cut off mid-message — run only
// while the schedule betrays or the whole-reply bound is back.
mod progress_tests;

// The RETR one-copy and allocation-free sizing proofs live in
// src/pop3/alloc_tests.rs (decision 0020 rule 2, issue #143): its counting
// allocator's diagnostics — a second body-sized buffer, a per-mail sizing
// allocation, an allocation the counter never saw — run only while the bug
// is present.
#[cfg(test)]
mod alloc_tests;
