//! The reference suite's mail source (accepted decision 0132 §8, story D):
//! the oracle's [`MailSource`] over the one shared client per tool
//! (`benches/mail_compare/clients/`, compiled here by path), waiting with
//! the target's `wait-best` method (§7.2) and keeping the accounting the
//! suite's rows need.
//!
//! It answers only what the oracle asks — the mail of this test whose
//! subject contains the requested text, once the requested count exists —
//! and only from the test's own public addresses: no cache, prefetch,
//! delivery index or oracle hint. Every scan is complete (the shared pager
//! refuses a partial page), every candidate the target's filter returns is
//! split by an exact recipient match into this test's own mail and other
//! tests' mail (`foreign_seen`, counted once per distinct message), and
//! every fetched message is rechecked against the identity its listing
//! showed. A partial page, a malformed fetch or a contradictory identity is
//! an operation failure with its own typed cause, never an answer.

use crate::api::Clock;
use crate::clients::{self, Client, OpError, Scope, Spend, Summary};
use crate::mailsource::MailSource;
use crate::suite::{self, Cleanup, PlannedTrial, Policy, WaitMethod};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

/// One request's bound inside a test's budget (a scan in flight at the
/// cutoff, a reconnect attempt, Swarmail's await answering its timeout).
pub const REQUEST_BOUND: Duration = Duration::from_secs(2);

/// The pause between cleanup's readings of a store-wide listing that other
/// tests' churn moved while it was read ([`NativeSource::owned_listing`]).
pub const RESCAN_PAUSE: Duration = Duration::from_millis(10);

/// Why the source has no answer, typed apart from the oracle's string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The test's deadline came first: the mail was not all there.
    Timeout(String),
    /// A listing, await or fetch failed: a partial page, a malformed body,
    /// a status the route does not document, a transport error.
    Operation(String),
    /// A fetched message's recipients or subject contradict its listing.
    Identity(String),
}

impl Failure {
    pub fn text(&self) -> String {
        match self {
            Failure::Timeout(w) => format!("timeout: {w}"),
            Failure::Operation(w) => format!("operation: {w}"),
            Failure::Identity(w) => format!("identity: {w}"),
        }
    }
}

/// The suite's log of store-wide and neighbour mutations, read only for
/// causal evidence: a test whose own seen mail vanished while another
/// test's mutation could have reached it.
#[derive(Debug, Default)]
pub struct Mutations(Mutex<Vec<Mutation>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mutation {
    pub trial: u32,
    pub start_ns: u64,
    pub end_ns: u64,
    /// `all` for a global clear, else the address whose ids were deleted.
    pub scope: String,
}

impl Mutations {
    pub fn push(&self, m: Mutation) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).push(m);
    }

    /// Another test's mutation overlapping `[from, to]` that could have
    /// removed mail addressed to one of `owned`.
    pub fn foreign_reaching(
        &self,
        trial: u32,
        owned: &[String],
        from: u64,
        to: u64,
    ) -> Vec<Mutation> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|m| m.trial != trial && m.start_ns <= to && m.end_ns >= from)
            .filter(|m| m.scope == "all" || owned.iter().any(|a| a.eq_ignore_ascii_case(&m.scope)))
            .cloned()
            .collect()
    }
}

/// A message's key: its id, and for a per-mailbox store its mailbox too
/// (mailbox-local ids repeat across mailboxes).
type Key = (String, String);

#[derive(Debug, Default)]
struct Acct {
    foreign: BTreeSet<Key>,
    mine: BTreeMap<Key, (Scope, Summary)>,
    /// Own mail seen by a scan and missing from a later one: when, first
    /// seen and first missed.
    vanished: BTreeMap<Key, (u64, u64)>,
    first_seen: BTreeMap<Key, u64>,
    last_scan: BTreeSet<Key>,
}

/// What one wait reported, on the suite's clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    pub call_ns: u64,
    pub return_ns: u64,
    pub count: u32,
    pub cause: String,
}

/// One test's mail source.
pub struct NativeSource<'a> {
    client: &'a Client,
    http: SocketAddr,
    method: WaitMethod,
    id: u32,
    owned: Vec<String>,
    deadline: Instant,
    clock: Clock,
    acct: Mutex<Acct>,
    pub timing: Option<Timing>,
    pub failure: Option<Failure>,
    calls: u32,
    /// Whether other tests run beside this one on the same store (a suite
    /// at parallelism above 1).
    neighbours: bool,
}

fn local(address: &str) -> String {
    address.split('@').next().unwrap_or_default().to_owned()
}

fn op(what: &str, e: OpError) -> Failure {
    match e {
        OpError::Timeout => Failure::Timeout(format!("{what}: the operation's budget ran out")),
        e => Failure::Operation(format!("{what}: {}", e.reason())),
    }
}

impl<'a> NativeSource<'a> {
    /// The source of trial `trial`, every operation within `deadline`.
    pub fn new(
        client: &'a Client,
        http: SocketAddr,
        method: WaitMethod,
        trial: &PlannedTrial,
        (deadline, clock): (Instant, Clock),
    ) -> NativeSource<'a> {
        NativeSource {
            client,
            http,
            method,
            id: trial.id,
            owned: suite::owned(&trial.flow, trial.id),
            deadline,
            clock,
            acct: Mutex::default(),
            timing: None,
            failure: None,
            calls: 0,
            neighbours: false,
        }
    }

    /// Mark that other tests share the store with this one (`workers > 1`):
    /// only then may cleanup read a moving store-wide listing again.
    pub fn beside_neighbours(mut self, yes: bool) -> Self {
        self.neighbours = yes;
        self
    }

    fn acct(&self) -> std::sync::MutexGuard<'_, Acct> {
        self.acct.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Distinct other tests' messages the target's filter returned.
    pub fn foreign_seen(&self) -> u64 {
        self.acct().foreign.len() as u64
    }

    /// Distinct messages of this test the source saw.
    pub fn owned_seen(&self) -> u64 {
        self.acct().mine.len() as u64
    }

    /// This test's own seen mail that a later scan no longer returned, with
    /// when it was first seen and first missed.
    pub fn vanished(&self) -> Vec<(String, u64, u64)> {
        self.acct()
            .vanished
            .iter()
            .map(|((m, id), (a, b))| (format!("{m}{id}"), *a, *b))
            .collect()
    }

    pub fn owned(&self) -> &[String] {
        &self.owned
    }

    fn is_mine(&self, s: &Summary) -> bool {
        s.to.iter()
            .any(|t| self.owned.iter().any(|a| a.eq_ignore_ascii_case(t)))
    }

    /// The scopes this test's mail lives in: Swarmail's one inbox (0004),
    /// or one recipient scope per owned address, whose mailbox is its local
    /// part (a per-mailbox store's key).
    pub fn scopes(&self) -> Vec<Scope> {
        if self.client.family == clients::Family::Swarmail {
            return vec![Scope {
                address: suite::primary(self.id),
                inbox: suite::inbox(self.id),
            }];
        }
        self.owned
            .iter()
            .map(|a| Scope {
                address: a.clone(),
                inbox: local(a),
            })
            .collect()
    }

    /// Fold one complete listing into the accounting: other tests' mail is
    /// counted once, this test's kept, a subject match returned.
    fn fold(
        &self,
        scope: &Scope,
        items: Vec<Summary>,
        token: Option<&str>,
        out: &mut BTreeMap<Key, (Scope, Summary)>,
    ) {
        let mailbox = if self.client.mailbox_local_ids() {
            scope.address.to_ascii_lowercase()
        } else {
            String::new()
        };
        let mut a = self.acct();
        let now = self.clock.now_ns();
        for s in items {
            let key = (mailbox.clone(), s.id.clone());
            if self.is_mine(&s) {
                a.first_seen.entry(key.clone()).or_insert(now);
                a.mine.insert(key.clone(), (scope.clone(), s.clone()));
                if token.is_none_or(|t| s.subject.contains(t)) {
                    out.insert(key, (scope.clone(), s));
                }
            } else {
                // On a suite's fresh target every other message is another
                // test's: counted once, however often it is seen.
                a.foreign.insert(key);
            }
        }
    }

    /// One complete, authoritative scan of this test's scopes, noting own
    /// mail an earlier scan returned and this one did not (cleanup's own
    /// listings, [`owned_listing`](Self::owned_listing), never do).
    async fn scan(&self, token: Option<&str>) -> Result<Vec<(Scope, Summary)>, Failure> {
        self.scan_by(token, self.deadline)
            .await
            .map_err(|e| op("scan", e))
    }

    /// [`scan`](Self::scan) with every page inside `by`, the pager's own
    /// error kept typed. Only a complete reading reaches the accounting: a
    /// refused one folds nothing and marks nothing missing.
    async fn scan_by(
        &self,
        token: Option<&str>,
        by: Instant,
    ) -> Result<Vec<(Scope, Summary)>, OpError> {
        let out = self.collect(token, by).await?;
        // Own mail an earlier scan returned and this one did not.
        let now = self.clock.now_ns();
        let keys: BTreeSet<Key> = out.keys().cloned().collect();
        let mut a = self.acct();
        let gone: Vec<Key> = a.last_scan.difference(&keys).cloned().collect();
        for k in gone {
            let seen = a.first_seen.get(&k).copied().unwrap_or(now);
            a.vanished.entry(k).or_insert((seen, now));
        }
        a.last_scan = keys;
        Ok(out.into_values().collect())
    }

    /// One complete reading of this test's scopes, folded; the pager's own
    /// error when a page is not.
    async fn collect(
        &self,
        token: Option<&str>,
        by: Instant,
    ) -> Result<BTreeMap<Key, (Scope, Summary)>, OpError> {
        let mut out = BTreeMap::new();
        let scopes = self.scopes();
        let take = if self.client.scope_free(token) {
            1
        } else {
            scopes.len()
        };
        for scope in scopes.iter().take(take) {
            let mut spend = Spend::default();
            let items = self.client.candidates(scope, token, by, &mut spend).await?;
            self.fold(scope, items, token, &mut out);
        }
        Ok(out)
    }

    /// Whether a listing under `token` that moved while it was read may be
    /// read again: only one that spans other tests' mail (store-wide, or a
    /// subject search that takes no recipient) and only while other tests
    /// share the store and so can move it. Alone on the store, a moving
    /// listing has no innocent cause and stays a failure.
    fn rereads(&self, token: Option<&str>) -> bool {
        self.neighbours && self.client.scope_free(token)
    }

    /// Cleanup's reading of every message this test owns, decoys and
    /// leftovers included. Where the listing spans other tests' mail
    /// (Mailpit's, store-wide: no recipient filter is cited) and other
    /// workers deliver and delete beside this test, their churn can move the
    /// stated count between pages or inside one answer, and the pager then
    /// refuses the listing as incomplete. That refusal is right, and it is
    /// kept: a moving listing is not an answer. It is read again,
    /// [`RESCAN_PAUSE`] apart, until one listing is complete, and only inside
    /// this test's own cleanup deadline. Any other failure, a listing still
    /// moving at the deadline, and a moving listing with no neighbours to
    /// move it (parallelism 1, or a store this test's scopes alone can read)
    /// fail at once, as before. Only reading repeats: no mutation is ever
    /// authorized by a failed one.
    async fn owned_listing(&self) -> Result<Vec<(Scope, Summary)>, Failure> {
        let retry = self.rereads(None);
        let mut moved = 0u32;
        loop {
            match self.collect(None, self.deadline).await {
                Ok(out) => return Ok(out.into_values().collect()),
                Err(OpError::Incomplete)
                    if retry && Instant::now() + RESCAN_PAUSE < self.deadline =>
                {
                    moved += 1;
                    tokio::time::sleep(RESCAN_PAUSE).await;
                }
                // The listing never settled: say so, whatever ended the last.
                Err(e) if moved > 0 => {
                    return Err(Failure::Operation(format!(
                        "scan: incomplete ({moved} listing(s) moved while read, then {})",
                        e.reason()
                    )));
                }
                Err(e) => return Err(op("scan", e)),
            }
        }
    }

    /// The poll wait (§7.2): complete scans on the target's cadence until
    /// the required count matches, inside one cutoff, the test's work
    /// deadline less [`REQUEST_BOUND`] (the tail the fetch needs). An
    /// already-expired wait, or a wake past the cutoff, issues no request;
    /// every page of every scan is bounded by the cutoff.
    ///
    /// A subject search that spans other tests' mail (Mailpit's and
    /// smtp4dev's search take no recipient) moves when a neighbour delivers
    /// under the same subject between its pages, and the pager refuses it as
    /// incomplete (issue #388). Beside neighbours that refusal is "not
    /// answered yet": nothing is folded, fetched or mutated, and the wait
    /// polls again on its own cadence. A wait whose cutoff comes while the
    /// listing is still moving is the wait's timeout, saying so; one whose
    /// cutoff cancels a scan after a short complete answer keeps the
    /// ordinary count diagnostic. Alone on the store, an unfiltered or a
    /// recipient-scoped listing, and every other error keep their typed
    /// failure, as before.
    ///
    /// The page bound is the poll's alone: the native await's HTTP call and
    /// the stream's scans are still bounded by the full work deadline. So a
    /// poll answer still in flight at the cutoff is refused where an
    /// await's arriving then would be taken, a window one scan wide. Spec
    /// AC3 (issue #388) requires it; it is stated, not an oversight.
    async fn poll(
        &self,
        token: Option<&str>,
        count: usize,
        interval: Duration,
    ) -> Result<(Vec<(Scope, Summary)>, String), Failure> {
        // The same reserved tail as the await and the stream: the wait ends
        // REQUEST_BOUND before the test's work deadline, whatever its method.
        let ends = self
            .deadline
            .checked_sub(REQUEST_BOUND)
            .unwrap_or(self.deadline);
        let reread = token.is_some() && self.rereads(token);
        // What the last attempt said: nothing yet, a short complete answer,
        // or this many consecutive listings that moved while read.
        let mut last = Unanswered::Nothing;
        loop {
            if Instant::now() >= ends {
                return Err(last.timeout(count, token));
            }
            match self.scan_by(token, ends).await {
                Ok(found) if found.len() >= count => return Ok((found, "poll".into())),
                Ok(found) => last = Unanswered::Short(found.len()),
                Err(OpError::Incomplete) if reread => last = last.moved(),
                // Every page is bounded by `ends`, so the pager's timeout is
                // this wait's own cutoff, never an independent cause: the
                // wait's timeout, saying what the last attempt saw (still
                // moving, a short count, or nothing complete yet).
                Err(OpError::Timeout) => return Err(last.timeout(count, token)),
                Err(e) => return Err(op("scan", e)),
            }
            if Instant::now() + interval >= ends {
                return Err(last.timeout(count, token));
            }
            tokio::time::sleep(interval).await;
        }
    }
}

/// Why a poll wait has no answer yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unanswered {
    Nothing,
    Short(usize),
    Moving(u32),
}

impl Unanswered {
    fn moved(self) -> Self {
        match self {
            Unanswered::Moving(n) => Unanswered::Moving(n + 1),
            _ => Unanswered::Moving(1),
        }
    }

    fn timeout(self, count: usize, token: Option<&str>) -> Failure {
        Failure::Timeout(match self {
            Unanswered::Nothing => {
                format!("no time left to poll for {count} mail(s) matching {token:?}")
            }
            Unanswered::Short(n) => {
                format!("waited for {count} mail(s) matching {token:?}; saw {n}")
            }
            Unanswered::Moving(n) => format!(
                "waited for {count} mail(s) matching {token:?}; the subject listing kept moving \
                 ({n} consecutive listing(s) moved while read)"
            ),
        })
    }
}

/// The stream observer's authoritative scan: answers only once the
/// required set is complete, and keeps that set for the fetch.
struct CountScan<'s, 'a> {
    src: &'s NativeSource<'a>,
    token: Option<&'s str>,
    count: usize,
    found: Mutex<Vec<(Scope, Summary)>>,
    failure: Mutex<Option<Failure>>,
}

impl crate::observe::Scanner for CountScan<'_, '_> {
    async fn scan(&self) -> Result<Vec<String>, String> {
        match self.src.scan(self.token).await {
            Ok(found) if found.len() >= self.count => {
                let ids = found.iter().map(|(_, s)| s.id.clone()).collect();
                *self.found.lock().unwrap_or_else(|e| e.into_inner()) = found;
                Ok(ids)
            }
            Ok(_) => Ok(Vec::new()),
            Err(f) => {
                let text = f.text();
                *self.failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
                Err(text)
            }
        }
    }
}

impl NativeSource<'_> {
    async fn stream(
        &self,
        token: Option<&str>,
        count: usize,
        backstop: Duration,
    ) -> Result<(Vec<(Scope, Summary)>, String), Failure> {
        let path = match self.client.family {
            clients::Family::Mailhog => "/api/v2/websocket",
            f => {
                return Err(Failure::Operation(format!(
                    "no stream is implemented for {}",
                    f.name()
                )));
            }
        };
        let start = Instant::now();
        // The whole observation, a scan still in flight at its cutoff
        // included, ends by the test's deadline.
        let wait = self
            .deadline
            .checked_duration_since(start)
            .and_then(|d| d.checked_sub(REQUEST_BOUND))
            .filter(|d| !d.is_zero())
            .ok_or_else(|| Failure::Timeout("no time left for a stream wait".into()))?;
        let channel = crate::observe::MailhogChannel {
            addr: self.http,
            path,
            limits: crate::stream::Limits::DEFAULT,
        };
        let scanner = CountScan {
            src: self,
            token,
            count,
            found: Mutex::default(),
            failure: Mutex::default(),
        };
        let bounds = crate::observe::StreamBounds {
            wait,
            request: REQUEST_BOUND,
            backstop,
            reconnect_gap: crate::observe::RECONNECT_GAP,
        };
        let mut trace = crate::observe::Trace::default();
        let seen = crate::observe::stream_wait(&channel, &scanner, start, bounds, &mut trace).await;
        if let Some(f) = scanner
            .failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            return Err(f);
        }
        match seen {
            Ok(crate::observe::Seen::Answer {
                cause, late: false, ..
            }) => {
                let found =
                    std::mem::take(&mut *scanner.found.lock().unwrap_or_else(|e| e.into_inner()));
                Ok((found, cause.name().to_owned()))
            }
            Ok(_) => Err(Failure::Timeout(format!(
                "the stream wait for {count} mail(s) matching {token:?} timed out"
            ))),
            Err(e) => Err(Failure::Operation(format!("stream: {e}"))),
        }
    }

    /// Swarmail's blocking await in the test's own inbox (0002: it
    /// subscribes before it checks), bounded inside the test's deadline.
    async fn native(
        &self,
        token: Option<&str>,
        count: usize,
    ) -> Result<(Vec<(Scope, Summary)>, String), Failure> {
        if self.client.family != clients::Family::Swarmail {
            return Err(Failure::Operation("only Swarmail has a native wait".into()));
        }
        let left = self
            .deadline
            .checked_duration_since(Instant::now())
            .and_then(|d| d.checked_sub(REQUEST_BOUND))
            .filter(|d| !d.is_zero())
            .ok_or_else(|| Failure::Timeout("no time left for the await".into()))?;
        let scope = &self.scopes()[0];
        let subject = token
            .map(|t| format!("&subject={}", clients::escape(t)))
            .unwrap_or_default();
        let path = format!(
            "/api/v1/inboxes/{}/await?count={count}&limit={}&timeout_ms={}{subject}",
            clients::escape(&scope.inbox),
            clients::swarmail::PAGE,
            left.as_millis()
        );
        let mut spend = Spend::default();
        let (status, body) = self
            .client
            .conn
            .call(reqwest::Method::GET, &path, None, self.deadline, &mut spend)
            .await
            .map_err(|e| op("await", e))?;
        match status {
            200 => {}
            408 => {
                return Err(Failure::Timeout(format!(
                    "the await for {count} mail(s) matching {token:?} timed out"
                )));
            }
            s => return Err(Failure::Operation(format!("await: status_{s}"))),
        }
        let v: Value = serde_json::from_slice(&body)
            .map_err(|_| Failure::Operation("await: malformed".into()))?;
        let emails = v
            .get("emails")
            .and_then(Value::as_array)
            .ok_or_else(|| Failure::Operation("await: malformed".into()))?;
        let (_, items) =
            clients::swarmail::decode_page(&json!({"total": emails.len(), "emails": emails}), 0)
                .ok_or_else(|| Failure::Operation("await: malformed".into()))?;
        let mut out = BTreeMap::new();
        self.fold(scope, items, token, &mut out);
        if out.len() < count {
            return Err(Failure::Operation(format!(
                "await answered {} of {count}",
                out.len()
            )));
        }
        Ok((out.into_values().collect(), "native".into()))
    }

    /// Read each matched message from its own mailbox and recheck it.
    async fn fetch(
        &self,
        found: &[(Scope, Summary)],
        token: Option<&str>,
    ) -> Result<Vec<Value>, Failure> {
        let mut out = Vec::with_capacity(found.len());
        for (scope, s) in found {
            let mut spend = Spend::default();
            let f = self
                .client
                .get(scope, &s.id, self.deadline, &mut spend)
                .await
                .map_err(|e| op("fetch", e))?;
            let listed: BTreeSet<String> = s.to.iter().map(|a| a.to_ascii_lowercase()).collect();
            let fetched: BTreeSet<String> = f.to.iter().map(|a| a.to_ascii_lowercase()).collect();
            if !self.is_mine(&Summary {
                to: f.to.clone(),
                ..s.clone()
            }) || listed.is_disjoint(&fetched)
            {
                return Err(Failure::Identity(format!(
                    "message {} was listed to {listed:?} but reads as to {fetched:?}",
                    s.id
                )));
            }
            let subject = f.subject.clone().unwrap_or_default();
            if token.is_some_and(|t| !subject.contains(t)) {
                return Err(Failure::Identity(format!(
                    "message {} was listed with a matching subject but reads as {subject:?}",
                    s.id
                )));
            }
            out.push(json!({
                "to": f.to.iter().map(|a| json!({"address": a})).collect::<Vec<_>>(),
                "subject": subject,
                "text": f.text.clone().unwrap_or_default(),
                "html": f.html.clone().unwrap_or_default(),
            }));
        }
        Ok(out)
    }
}

impl MailSource for NativeSource<'_> {
    async fn wait_emails(
        &mut self,
        subject: Option<&str>,
        count: usize,
    ) -> Result<Vec<Value>, String> {
        self.calls += 1;
        let call_ns = self.clock.now_ns();
        let waited = match self.method {
            WaitMethod::Native => self.native(subject, count).await,
            WaitMethod::Poll { interval_ms } => {
                self.poll(subject, count, Duration::from_millis(interval_ms))
                    .await
            }
            WaitMethod::Stream { backstop_ms } => {
                self.stream(subject, count, Duration::from_millis(backstop_ms))
                    .await
            }
        };
        let read = match waited {
            Ok((found, cause)) => self.fetch(&found, subject).await.map(|v| (v, cause)),
            Err(f) => Err(f),
        };
        match read {
            Ok((mails, cause)) => {
                self.timing = Some(Timing {
                    call_ns,
                    return_ns: self.clock.now_ns(),
                    count: count as u32,
                    cause,
                });
                Ok(mails)
            }
            Err(f) => {
                let text = f.text();
                self.failure = Some(f);
                Err(text)
            }
        }
    }

    fn tool_calls(&self) -> u32 {
        self.calls
    }
}

impl NativeSource<'_> {
    /// Move the source's deadline to the test's cleanup deadline, the
    /// reserved tail after its work deadline; never earlier.
    pub fn extend_deadline(&mut self, to: Instant) {
        self.deadline = self.deadline.max(to);
    }

    /// This test's own cleanup under `policy` (0132 §8.4), inside the
    /// test's deadline. Delete-by-id recovers every message this test owns
    /// from a fresh complete listing (decoys and leftovers of a failed flow
    /// included) and deletes exactly those ids from their own mailboxes; a
    /// failed narrow operation stops there and never authorizes a broader
    /// one. Each mutation is logged for the suite's causal evidence, and
    /// the scope is verified empty afterwards. Both readings are
    /// [`owned_listing`](Self::owned_listing)'s: beside other workers, a
    /// store-wide listing their churn moved is read again until one is
    /// complete, inside this deadline.
    pub async fn cleanup(&self, policy: Policy, log: &Mutations) -> Cleanup {
        let failed = |reason: String| Cleanup::Failed { reason };
        let start_ns = self.clock.now_ns();
        let removed = match policy {
            Policy::GlobalClearAfterSuite => return Cleanup::Deferred,
            Policy::Unavailable => return Cleanup::NotAvailable,
            Policy::InboxClear | Policy::GlobalClearPerTest => {
                let before = match self.owned_listing().await {
                    Ok(v) => v.len() as u64,
                    Err(f) => return failed(format!("recover: {}", f.text())),
                };
                let scope = &self.scopes()[0];
                let mut spend = Spend::default();
                let r = self.client.clear(scope, self.deadline, &mut spend).await;
                log.push(Mutation {
                    trial: self.id,
                    start_ns,
                    end_ns: self.clock.now_ns(),
                    scope: if policy == Policy::InboxClear {
                        scope.address.clone()
                    } else {
                        "all".into()
                    },
                });
                if let Err(e) = r {
                    return failed(format!("clear: {}", e.reason()));
                }
                before
            }
            Policy::DeleteById => {
                let owned = match self.owned_listing().await {
                    Ok(v) => v,
                    Err(f) => return failed(format!("recover: {}", f.text())),
                };
                let mut n = 0;
                for (scope, s) in &owned {
                    let mut spend = Spend::default();
                    let r = self
                        .client
                        .delete(scope, &s.id, self.deadline, &mut spend)
                        .await;
                    log.push(Mutation {
                        trial: self.id,
                        start_ns,
                        end_ns: self.clock.now_ns(),
                        scope: scope.address.clone(),
                    });
                    if let Err(e) = r {
                        return failed(format!("delete {}: {}", s.id, e.reason()));
                    }
                    n += 1;
                }
                n
            }
        };
        match self.owned_listing().await {
            Ok(left) if left.is_empty() => Cleanup::Done { removed },
            Ok(left) => failed(format!(
                "{} owned message(s) remain after cleanup",
                left.len()
            )),
            Err(f) => failed(format!("verify: {}", f.text())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observe::Scanner;

    /// A Mailpit search answering `n` of this test's confirmations, or 500.
    async fn searching(n: usize, fail: bool) -> SocketAddr {
        let to = suite::primary(101);
        let router = axum::Router::new().route(
            "/api/v1/search",
            axum::routing::get(move || {
                let to = to.clone();
                async move {
                    if fail {
                        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(Value::Null));
                    }
                    let items: Vec<Value> = (0..n)
                        .map(|i| json!({"ID": format!("m{i}"), "To": [{"Address": to}], "Subject": "Confirm your account"}))
                        .collect();
                    (
                        axum::http::StatusCode::OK,
                        axum::Json(json!({"start": 0, "messages_count": n, "messages": items})),
                    )
                }
            }),
        );
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(l, router).await;
        });
        addr
    }

    #[tokio::test]
    async fn the_stream_scan_answers_only_a_complete_required_set() {
        let trial = suite::plan_ids(163, [101]).remove(0);
        let addr = searching(3, false).await;
        let client = Client::new("mailpit", addr).unwrap();
        let src = NativeSource::new(
            &client,
            addr,
            WaitMethod::Stream { backstop_ms: 1000 },
            &trial,
            (
                Instant::now() + Duration::from_secs(10),
                Clock(std::time::Instant::now()),
            ),
        );
        let scan = |count| CountScan {
            src: &src,
            token: Some("Confirm your account"),
            count,
            found: Mutex::default(),
            failure: Mutex::default(),
        };
        // Fewer than the count: an empty answer, so the observer keeps
        // waiting for an event, a reconnect or the backstop.
        let short = scan(4);
        assert_eq!(short.scan().await, Ok(Vec::new()));
        assert!(short.found.lock().unwrap().is_empty());
        // The whole set answers, and is kept for the fetch.
        let whole = scan(3);
        assert_eq!(whole.scan().await.unwrap().len(), 3);
        assert_eq!(whole.found.lock().unwrap().len(), 3);
        // A failed scan is an error with its typed cause, never empty.
        let addr = searching(0, true).await;
        let client = Client::new("mailpit", addr).unwrap();
        let src = NativeSource::new(
            &client,
            addr,
            WaitMethod::Stream { backstop_ms: 1000 },
            &trial,
            (
                Instant::now() + Duration::from_secs(10),
                Clock(std::time::Instant::now()),
            ),
        );
        let failing = CountScan {
            src: &src,
            token: Some("Confirm your account"),
            count: 1,
            found: Mutex::default(),
            failure: Mutex::default(),
        };
        assert_eq!(
            failing.scan().await.unwrap_err(),
            "operation: scan: status_500"
        );
        assert!(matches!(
            *failing.failure.lock().unwrap(),
            Some(Failure::Operation(_))
        ));
    }
}
