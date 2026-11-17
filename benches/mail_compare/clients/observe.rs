//! The stream observer of `wait-best` (accepted decision 0132 §7.2, story
//! C): subscribe, await the channel's cited readiness, then one complete
//! scan; rescan after every delivery event, after every re-established
//! readiness and on a backstop timer anchored at the first readiness. The
//! scan is authoritative and an event only wakes it, so a lossy channel is
//! usable without being claimed lossless: a dropped event shows as a
//! `backstop` observation.
//!
//! One owner drives everything from one `select!`: the cutoff, the
//! response bound of a scan in flight, the backstop, the channel and a
//! pending reconnect. No task is detached and no queue grows: causes that
//! arrive while a scan runs are kept as a set of flags, and a scan that
//! starts discharges every cause pending at its start. The biased order
//! polls the scan and the timers before the channel, so an event flood
//! cannot starve the cutoff or the backstop.

use std::collections::BTreeSet;
use std::future::Future;
use std::time::Duration;
use tokio::time::{Instant, sleep_until};

use crate::stream::{Limits, StreamError, WebSocket, WsMessage, connect, fresh_key};

/// Why a scan ran. The order is the precedence of 0132 §7.2's design when
/// several causes are pending at once: the first scan, a re-established
/// readiness, the backstop, then a delivery event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cause {
    InitialScan,
    Reconnect,
    Backstop,
    Event,
}

impl Cause {
    pub const ALL: [Cause; 4] = [
        Cause::InitialScan,
        Cause::Reconnect,
        Cause::Backstop,
        Cause::Event,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Cause::InitialScan => "initial_scan",
            Cause::Reconnect => "reconnect",
            Cause::Backstop => "backstop",
            Cause::Event => "event",
        }
    }

    pub fn parse(text: &str) -> Option<Cause> {
        Cause::ALL.into_iter().find(|c| c.name() == text)
    }
}

/// What a ready channel yields: a delivery notification wakes a scan; any
/// other traffic (a late pong, a keepalive) does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    Delivery,
    Other,
}

/// A push channel with cited readiness. `ready` opens, registers and
/// proves readiness by `deadline`, or fails; `next` yields the next item of
/// a ready connection and must be cancel-safe (a dropped call loses no
/// byte).
pub trait Channel {
    type Conn;
    fn ready(&self, deadline: Instant) -> impl Future<Output = Result<Self::Conn, StreamError>>;
    fn next(
        conn: &mut Self::Conn,
        deadline: Instant,
    ) -> impl Future<Output = Result<Wake, StreamError>>;
}

/// One complete authoritative scan: every message the target's documented
/// search or listing returns for the wait, as tokens, related or not. A
/// failed or incomplete scan is an error, never an empty answer.
pub trait Scanner {
    fn scan(&self) -> impl Future<Output = Result<Vec<String>, String>>;
}

/// The spacing of reconnect attempts after an established connection is
/// lost (the design's 100 ms): at most one attempt is in flight, each
/// bounded by one request bound and the cutoff, so attempts are bounded by
/// the wait over this gap.
pub const RECONNECT_GAP: Duration = Duration::from_millis(100);

/// The bounds of one stream observation.
#[derive(Debug, Clone, Copy)]
pub struct StreamBounds {
    /// The wait: `start + wait` is the cutoff.
    pub wait: Duration,
    /// One request bound: the response allowance of a scan still running at
    /// the cutoff, and the bound of each readiness attempt.
    pub request: Duration,
    /// The backstop interval, anchored at the first readiness.
    pub backstop: Duration,
    /// The spacing of reconnect attempts after a lost connection.
    pub reconnect_gap: Duration,
}

/// What a stream observation saw. `late` is a non-empty answer from a scan
/// that ended after the cutoff: censored, like a poll's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    Answer {
        tokens: Vec<String>,
        cause: Cause,
        late: bool,
    },
    Timeout,
}

/// Counters a test reads to prove what the observer did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trace {
    pub scans: Vec<Cause>,
    pub connects: u32,
    pub reconnect_failures: u32,
    pub events: u32,
}

/// Observe until a scan answers or the cutoff passes. A readiness failure
/// before the first scan is an error, never a silent fallback to a poll; a
/// malformed channel is an error; a lost connection after readiness is
/// reconnected every `reconnect_gap` while the backstop keeps scanning. No
/// scan starts at or after the cutoff, and a scan still running one
/// request bound after it is an error.
pub async fn stream_wait<C: Channel, S: Scanner>(
    channel: &C,
    scanner: &S,
    start: Instant,
    b: StreamBounds,
    trace: &mut Trace,
) -> Result<Seen, String> {
    let cutoff = start + b.wait;
    let response = cutoff + b.request;
    let first = channel
        .ready((Instant::now() + b.request).min(cutoff))
        .await
        .map_err(|e| format!("readiness: {e}"))?;
    trace.connects += 1;
    let mut conn = Some(first);
    let mut pending: BTreeSet<Cause> = BTreeSet::from([Cause::InitialScan]);
    let mut backstop_at = Instant::now() + b.backstop;
    let mut reconnect_at: Option<Instant> = None;
    let mut reconnecting = None;
    let mut scanning = None;
    let mut scan_cause = Cause::InitialScan;
    loop {
        let now = Instant::now();
        if scanning.is_none()
            && now < cutoff
            && let Some(&cause) = pending.first()
        {
            // A scan discharges every cause pending at its start; a cause
            // arriving while it runs waits for the next one.
            pending.clear();
            scan_cause = cause;
            trace.scans.push(cause);
            scanning = Some(Box::pin(scanner.scan()));
        }
        if scanning.is_none() && now >= cutoff {
            return Ok(Seen::Timeout);
        }
        if conn.is_none() && reconnecting.is_none() && reconnect_at.is_some_and(|at| now >= at) {
            reconnect_at = None;
            reconnecting = Some(Box::pin(channel.ready((now + b.request).min(cutoff))));
        }
        let far = cutoff + b.request;
        tokio::select! {
            biased;
            done = async { scanning.as_mut().expect("guarded").await }, if scanning.is_some() => {
                scanning = None;
                let tokens = done?;
                let late = Instant::now() > cutoff;
                if !tokens.is_empty() {
                    return Ok(Seen::Answer { tokens, cause: scan_cause, late });
                }
            }
            _ = sleep_until(response), if scanning.is_some() => {
                return Err("operation deadline: a scan was still running one request bound after the cutoff".into());
            }
            _ = sleep_until(cutoff), if scanning.is_none() => return Ok(Seen::Timeout),
            _ = sleep_until(backstop_at) => {
                pending.insert(Cause::Backstop);
                // Anchored at readiness: traffic and scans never move it.
                while backstop_at <= Instant::now() {
                    backstop_at += b.backstop;
                }
            }
            got = async { reconnecting.as_mut().expect("guarded").await }, if reconnecting.is_some() => {
                reconnecting = None;
                match got {
                    Ok(c) => {
                        trace.connects += 1;
                        conn = Some(c);
                        pending.insert(Cause::Reconnect);
                    }
                    Err(e) if e.is_disconnect() || matches!(e, StreamError::Deadline | StreamError::Io { .. }) => {
                        trace.reconnect_failures += 1;
                        reconnect_at = Some(Instant::now() + b.reconnect_gap);
                    }
                    Err(e) => return Err(format!("reconnect: {e}")),
                }
            }
            _ = sleep_until(reconnect_at.unwrap_or(far)), if conn.is_none() && reconnecting.is_none() && reconnect_at.is_some() => {}
            item = async { C::next(conn.as_mut().expect("guarded"), response).await }, if conn.is_some() => {
                match item {
                    Ok(Wake::Delivery) => {
                        trace.events += 1;
                        pending.insert(Cause::Event);
                    }
                    Ok(Wake::Other) => {}
                    Err(e) if e.is_disconnect() => {
                        conn = None;
                        reconnect_at = Some(Instant::now() + b.reconnect_gap);
                    }
                    // The channel reads until the response bound, which
                    // the biased timers above always reach first.
                    Err(StreamError::Deadline) => {}
                    Err(e) => return Err(format!("channel: {e}")),
                }
            }
        }
    }
}

/// MailHog's `api/v2/websocket` (lock v5, `mailhog-server-api-v2.go`,
/// `mailhog-websockets-hub.go`, `mailhog-websockets-connection.go`):
/// `Hub.Serve` writes the upgrade response, hands the connection to the
/// hub loop on an unbuffered channel, and only then starts the read loop,
/// whose vendored gorilla `conn.go` answers a ping with a pong. A pong
/// echoing this connection's fresh ping payload therefore follows the
/// registration, and every broadcast the hub loop takes later reaches it.
/// Every text message after that is a delivered message's JSON: a delivery
/// notification, whose content is never read.
pub struct MailhogChannel {
    pub addr: std::net::SocketAddr,
    pub path: &'static str,
    pub limits: Limits,
}

impl Channel for MailhogChannel {
    type Conn = WebSocket<tokio::net::TcpStream>;

    async fn ready(&self, deadline: Instant) -> Result<Self::Conn, StreamError> {
        let io = connect(self.addr, deadline).await?;
        let host = self.addr.to_string();
        let mut ws = WebSocket::handshake(io, &host, self.path, deadline, self.limits).await?;
        let probe = fresh_key()?;
        ws.ping(probe.as_bytes(), deadline).await?;
        loop {
            match ws.next(deadline).await? {
                WsMessage::Pong(p) if p == probe.as_bytes() => return Ok(ws),
                WsMessage::Pong(_) => {
                    return Err(StreamError::Handshake(
                        "a pong that does not echo this connection's ping".into(),
                    ));
                }
                // A broadcast that reaches the connection before the pong
                // was taken after the registration; the first scan, which
                // follows readiness, sees its mail.
                WsMessage::Text(_) => {}
                WsMessage::Binary(_) => {
                    return Err(StreamError::Frame("a binary message on a JSON channel"));
                }
            }
        }
    }

    async fn next(conn: &mut Self::Conn, deadline: Instant) -> Result<Wake, StreamError> {
        match conn.next(deadline).await? {
            WsMessage::Text(_) => Ok(Wake::Delivery),
            WsMessage::Pong(_) => Ok(Wake::Other),
            WsMessage::Binary(_) => Err(StreamError::Frame("a binary message on a JSON channel")),
        }
    }
}
