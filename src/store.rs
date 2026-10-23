//! The in-memory, per-inbox email store.
//!
//! Design goals: lossless under bursts (no async drop points), per-inbox
//! ring pruning of *oldest* mail only, and cheap waiting (broadcast
//! watchers for `await` endpoints).
//!
//! Memory is not globally bounded (decision 0038, accepted). The per-inbox
//! message cap (`max_per_inbox`; zero disables it) is not a global byte or
//! RSS cap: inbox count is unlimited, so resident memory grows with the
//! number of inboxes times their mail. A held mail costs its raw bytes,
//! decoded bodies and extracted fields, both index entries, any `Arc` a
//! watcher ring still holds, and the transient buffers of concurrent
//! requests. A data file restores every persisted row before bind, without
//! applying the cap; its size on disk is not a measure of RAM. Each
//! distinct awaited inbox name keeps its watcher channel — roughly 780 B
//! per name observed on one build, not a fixed ceiling — until the inbox's
//! mail is cleared or emptied, or the process restarts; a repeated name
//! reuses its channel.
//!
//! `received_at`/`received_ms` are sampled at `build_email` entry, before
//! parsing and before acceptance: neither the sender's `Date` nor a commit
//! sequence, and not guaranteed to sort in acceptance order across sessions
//! or clock steps. Per-inbox order, POP3 numbering, oldest-only pruning and
//! the data file's restore sequence follow acceptance.

use crate::model::Email;
use crate::persist;
use dashmap::DashMap;
use rusqlite::ffi;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

/// The per-inbox wake ring's capacity. A watcher channel is a wake token,
/// not a mailbox — both consumers (`wait_for`, the SSE feed) discard the
/// payload and re-read the store — so the ring is sized for waking, not for
/// buffering. A full ring lags (never blocks: `send` is non-blocking at any
/// capacity, so the 250 path is untouched) and lag is repaired by the rescan
/// the wait loop already performs (decision 0021).
const WATCHER_CAP: usize = 16;

/// Test-only count of `Closed`-arm hits in `wait_for` — the deterministic
/// no-spin instrument: one hit per release, hundreds of thousands under a
/// reintroduced spin. Compiled out of the shipping binary like the seam.
#[cfg(test)]
static CLOSED_PROBE: AtomicU64 = AtomicU64::new(0);

/// Query filter for list/count/await endpoints.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Substring match against any recipient (to/cc/envelope) address,
    /// case-insensitive. The envelope's RCPT TO list is part of the answer
    /// (issue #55): a search for a *delivered* recipient must not answer
    /// zero just because no header row carries the address.
    pub to: Option<String>,
    /// Substring match against the from address, case-insensitive.
    pub from: Option<String>,
    /// Substring match against the subject, case-insensitive.
    pub subject: Option<String>,
    /// Only emails received at/after this unix-millis timestamp.
    pub since_ms: Option<i64>,
}

impl Filter {
    pub fn matches(&self, email: &Email) -> bool {
        if let Some(to) = &self.to {
            let to = to.to_lowercase();
            let hit = email
                .to
                .iter()
                .map(|a| &a.address)
                .chain(email.cc.iter().map(|a| &a.address))
                .chain(email.recipients.iter())
                .any(|a| a.to_lowercase().contains(&to));
            if !hit {
                return false;
            }
        }
        if let Some(from) = &self.from {
            let from = from.to_lowercase();
            let hit = email
                .from
                .as_ref()
                .map(|a| a.address.to_lowercase().contains(&from))
                .unwrap_or(false);
            if !hit {
                return false;
            }
        }
        if let Some(subject) = &self.subject {
            let hit = email
                .subject
                .as_ref()
                .map(|s| s.to_lowercase().contains(&subject.to_lowercase()))
                .unwrap_or(false);
            if !hit {
                return false;
            }
        }
        if let Some(since) = self.since_ms
            && email.received_ms < since
        {
            return false;
        }
        true
    }
}

#[derive(Default)]
pub struct Totals {
    pub emails_inserted: AtomicU64,
    pub emails_dropped: AtomicU64,
}

/// Drop one inbox out of the maps and return the dropped mail; shared by
/// `clear` and `clear_all` so both mirror exactly the ids they removed.
///
/// The drain is where an inbox dies, so it is where its watcher channel
/// dies: the remove is unconditional — an *empty* clear may still release a
/// channel (an awaited-but-never-mailed inbox's mail is vacuously gone) and
/// is miss-free because the waiter answers `Closed` with a re-subscribe
/// before its next scan (decision 0002, re-applied per iteration). The
/// nonexistent-inbox "no-op" means *removes no mail, returns 0*, not
/// channel immortality.
fn drain_inbox(
    inboxes: &DashMap<String, VecDeque<Arc<Email>>>,
    index: &DashMap<String, Arc<Email>>,
    watchers: &DashMap<String, broadcast::Sender<Arc<Email>>>,
    inbox: &str,
) -> Vec<Arc<Email>> {
    watchers.remove(inbox);
    match inboxes.remove(inbox) {
        Some((_, list)) => {
            for e in &list {
                index.remove(&e.id);
            }
            list.into_iter().collect()
        }
        None => Vec::new(),
    }
}

/// The compat views' one order (decision 0049 §5): newest receive stamp
/// first, the larger UUIDv7 id first on a tie — the reverse of the
/// `(received_ms, id)` maximum [`Store::latest`] picks.
pub fn newest_first(a: &Arc<Email>, b: &Arc<Email>) -> std::cmp::Ordering {
    (b.received_ms, &b.id).cmp(&(a.received_ms, &a.id))
}

pub struct Store {
    /// Oldest at the front, newest at the back (insertion order). A deque
    /// because eviction pops the front: a `Vec`'s front-drain memmoves every
    /// remaining element on each capped insert, which at the default cap
    /// costs more than the whole rest of the accept path (the steady-state
    /// bench measures it). The law is decision 0003's, unchanged — the
    /// container only changes what eviction costs.
    inboxes: DashMap<String, VecDeque<Arc<Email>>>,
    /// id -> email, a global index for O(1) lookup.
    index: DashMap<String, Arc<Email>>,
    /// Per-inbox wake channels: created by the first `subscribe` to an inbox
    /// (atomically, under the shard lock) and released when the inbox's mail
    /// is gone — by `drain_inbox` (shared by `clear`/`clear_all`), by a
    /// `delete` that empties the inbox's list, and by the `clear_all` sweep
    /// (decision 0021). `insert` never creates one: a notify with no entry
    /// is a no-op.
    watchers: DashMap<String, broadcast::Sender<Arc<Email>>>,
    /// Process-wide channel: every accepted email, for webhooks.
    global_tx: broadcast::Sender<Arc<Email>>,
    /// The one ordered consistency boundary for every persistent mutation
    /// (decision 0014): plan → commit → apply run inside this lock, so a
    /// later mutation can never prune an id before an earlier mutation's
    /// commit lands — memory and the data file are one mutation order,
    /// which the 16-thread boundary probe proves. Held across the SQLite
    /// transaction and the pre-mirror test gate: no await, no re-entry, no
    /// I/O but the one commit. Readers (`list`/`get`/`count`) stay
    /// lock-free — they only ever take DashMap shard guards, and no path
    /// takes a shard guard and then this lock (the order is one-way).
    mutation: std::sync::Mutex<()>,
    /// Maximum emails retained per inbox (oldest pruned). 0 = unlimited.
    max_per_inbox: usize,
    totals: Totals,
    /// Write-through SQLite mirror when running with a data file.
    persist: Option<persist::Persist>,
    /// The shutdown observer (decision 0024): `Some` once the server wired
    /// this store to the runtime's stop watch. The mutation boundary reads
    /// it at the commit chokepoint with `borrow()`: synchronous, but it takes
    /// a watch read lock. No await enters the boundary (0014 §1); this is not
    /// a lock-free or wait-free read. A store never wired (every unit test,
    /// every embedding) keeps today's behavior exactly: the flag is the
    /// boundary's policy input, not a constructor concern, so `Store::new`'s
    /// 46+ call sites keep their shape.
    stopping: std::sync::OnceLock<tokio::sync::watch::Receiver<bool>>,
    /// Test-only seam for the pins (decision 0020): when set, the closure
    /// runs pre-commit in `record`, *inside* the mutation boundary of every
    /// mutation and after its shutdown admission (0024) — so a test can
    /// hold an admitted commit mid-flight deterministically instead of
    /// winning a thread-timing lottery.
    /// Compiled only under test; never set in production. The closure must
    /// NOT re-enter the store: the boundary lock is held across it, so a
    /// re-entering mutation would deadlock on itself (block on a flag, or
    /// panic — the two pins' contracts).
    #[cfg(test)]
    pre_mirror_gate: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
    /// Test-only seam for the watcher release/wait interplay (decision 0021, the seam per 0020):
    /// when set, the closure runs after *every* `subscribe` inside `wait_for`
    /// — the initial one and each `Closed`-arm re-subscribe — so a test can
    /// interleave a clear or an insert into the exact subscribe→scan window
    /// deterministically instead of winning a thread-timing lottery.
    /// Compiled only under test; never set in production. It must fire only
    /// after `subscribe()` returns — never inside an `entry()` expression —
    /// because its closures re-enter the store and would deadlock on the
    /// same shard write lock.
    #[cfg(test)]
    post_subscribe_gate: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
    /// Test-only seam for POP3's PASS snapshot (issue #86, the seam per
    /// 0020): when set, the closure runs inside the snapshot job, so a test
    /// can hold real PASS work and observe that the async worker still
    /// serves other sockets. Per instance — never a global gate. Compiled
    /// only under test; never set in production.
    #[cfg(test)]
    snapshot_gate: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
    /// Test-only seam for the compat page builds (issue #96, decision 0044
    /// §5, the seam per 0020): when set, the closure runs at the start of
    /// every compat list/search build on the blocking pool, so a test can
    /// hold real page work and observe that the async worker still greets
    /// SMTP and answers health. Per instance; compiled only under test.
    #[cfg(test)]
    compat_gate: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
}

impl Totals {
    pub fn emails_inserted(&self) -> u64 {
        self.emails_inserted.load(Ordering::Relaxed)
    }

    pub fn emails_dropped(&self) -> u64 {
        self.emails_dropped.load(Ordering::Relaxed)
    }
}

impl Store {
    /// Lifetime count of accepted emails (the Prometheus counter).
    pub fn emails_inserted(&self) -> u64 {
        self.totals.emails_inserted()
    }

    /// Lifetime count of cap-pruned emails (the Prometheus counter).
    pub fn emails_dropped(&self) -> u64 {
        self.totals.emails_dropped()
    }

    pub fn new(max_per_inbox: usize) -> Self {
        let (global_tx, _) = broadcast::channel(8192);
        Self {
            inboxes: DashMap::new(),
            index: DashMap::new(),
            watchers: DashMap::new(),
            global_tx,
            mutation: std::sync::Mutex::new(()),
            max_per_inbox,
            totals: Totals::default(),
            persist: None,
            stopping: std::sync::OnceLock::new(),
            #[cfg(test)]
            pre_mirror_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            post_subscribe_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            snapshot_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            compat_gate: std::sync::Mutex::new(None),
        }
    }

    /// Open (or create) a persistent store backed by `data_file`: the full
    /// state is restored before the server accepts a single connection.
    /// The returned store owns the data file's single-writer lock until it
    /// drops (decision 0033); a held file refuses with
    /// [`persist::OpenError::InUse`].
    pub fn open(
        max_per_inbox: usize,
        data_file: &std::path::Path,
    ) -> Result<Self, persist::OpenError> {
        let persist = persist::Persist::open(data_file)?;
        let (emails, inserted, dropped) = persist.load()?;
        let mut store = Self::new(max_per_inbox);
        // Restore in insertion order without going through `insert`: the rows
        // are already committed, and restored mail must not re-fire watchers
        // or webhooks on every restart. `push_back` keeps front = oldest, so
        // the restored order matches the live one; restore still does NOT
        // prune (0003 puts eviction in `insert`) — a restore over a lowered
        // cap is trimmed by the first insert's catch-up, not silently here.
        for email in emails {
            let email = Arc::new(email);
            store
                .inboxes
                .entry(email.inbox.clone())
                .or_default()
                .push_back(email.clone());
            store.index.insert(email.id.clone(), email);
        }
        store
            .totals
            .emails_inserted
            .store(inserted, Ordering::Relaxed);
        store
            .totals
            .emails_dropped
            .store(dropped, Ordering::Relaxed);
        store.persist = Some(persist);
        Ok(store)
    }

    /// Wire this store to the runtime's stop watch (decision 0024). One
    /// shot, called once by `run_on` after the watch is born and before
    /// any listener binds — so no mutation can be admitted under an
    /// unwired flag and then refused by a wired one mid-flight. A second
    /// call is a no-op: the first watch is the store's one source of
    /// "stopping", never a second channel.
    pub(crate) fn observe_shutdown(&self, rx: tokio::sync::watch::Receiver<bool>) {
        let _ = self.stopping.set(rx);
    }

    /// Run one mirror write. The outcome is the mutation's: a failed commit
    /// is logged here — the log stays the one witness at the single owner
    /// of the mirror — and **returned**, so the caller refuses instead of
    /// acknowledging (decision 0014): fixed surface text, nothing applied,
    /// nothing acknowledged. The caller runs this inside the mutation
    /// boundary, before its memory phase applies, so a refused commit
    /// leaves memory and data file agreeing — there is nothing to roll
    /// back.
    fn record(
        &self,
        what: &'static str,
        write: impl FnOnce(&persist::Persist) -> rusqlite::Result<()>,
    ) -> rusqlite::Result<()> {
        // The boundary refuses to *start* a commit once the stop has fired
        // (decision 0024): every queued mutation acquires the boundary in
        // turn, reads the watch and refuses before SQLite, so queued
        // mutations do not start SQLite busy-handler lock retries. An admitted
        // mutation may continue; `busy_timeout` configures lock-contention
        // retries only and does not bound other storage I/O or shutdown latency.
        // The refusal is a constructed `SQLITE_ABORT` (truthful: nothing
        // was written) and applies nothing — no mail, no inbox, no counter
        // move, no eviction, no fan-out — so memory and data file agree.
        // The wire keeps 0012 §2's fixed refusal text (the mappers are
        // `Err(_)` and stay that way); the cause is this log, at the one
        // owner of the mirror. The check precedes the persist branch: the
        // policy is the boundary's, not the mirror's — a memory-only store
        // refuses too. `borrow()` is the `&self` form of the watch read:
        // the boundary takes `&self`, and nothing ever polls this clone's
        // change flag, so there is no seen-state to update.
        if let Some(stopping) = self.stopping.get()
            && *stopping.borrow()
        {
            tracing::info!(what, "refused: shutting down; nothing applied");
            return Err(rusqlite::Error::SqliteFailure(
                ffi::Error::new(ffi::SQLITE_ABORT),
                Some("shutting down".into()),
            ));
        }
        // Past admission, before the mirror: the one place a test can hold
        // an admitted mutation mid-commit (issue #143).
        #[cfg(test)]
        self.run_pre_mirror_gate();
        if let Some(persist) = &self.persist
            && let Err(e) = write(persist)
        {
            tracing::error!(error = %e, what, "storage commit failed; refused, nothing stored, committed state preserved");
            return Err(e);
        }
        Ok(())
    }

    /// Test-only: install (or remove) the pre-mirror gate. See the field doc.
    #[cfg(test)]
    pub(crate) fn set_pre_mirror_gate(&self, gate: Option<std::sync::Arc<dyn Fn() + Send + Sync>>) {
        *self
            .pre_mirror_gate
            .lock()
            .expect("pre-mirror gate poisoned") = gate;
    }

    /// Test-only: fire the pre-mirror gate, if one is installed. The Arc is
    /// cloned out before the call; the closure runs while the mutation
    /// boundary is held, so it must never re-enter the store (it would
    /// deadlock on its own lock) — it blocks or panics, per the pins.
    #[cfg(test)]
    fn run_pre_mirror_gate(&self) {
        let gate = self
            .pre_mirror_gate
            .lock()
            .expect("pre-mirror gate poisoned")
            .clone();
        if let Some(gate) = gate {
            gate();
        }
    }

    /// Test-only: how many inbox watcher channels currently live. The
    /// release tests' observable — issue #14's "count returns to baseline"
    /// criterion made literal; the production `/metrics` gauge is a
    /// deliberate follow-up recorded in decision 0021.
    #[cfg(test)]
    pub(crate) fn watcher_count(&self) -> usize {
        self.watchers.len()
    }

    /// Test-only: install (or remove) the post-subscribe gate. See the
    /// field doc.
    #[cfg(test)]
    pub(crate) fn set_post_subscribe_gate(
        &self,
        gate: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    ) {
        *self
            .post_subscribe_gate
            .lock()
            .expect("post-subscribe gate poisoned") = gate;
    }

    /// Test-only: fire the post-subscribe gate, if one is installed. The
    /// Arc is cloned out before the call so the closure may re-enter the
    /// store.
    #[cfg(test)]
    fn run_post_subscribe_gate(&self) {
        let gate = self
            .post_subscribe_gate
            .lock()
            .expect("post-subscribe gate poisoned")
            .clone();
        if let Some(gate) = gate {
            gate();
        }
    }

    /// Test-only: install (or remove) the PASS snapshot gate. See the field
    /// doc.
    #[cfg(test)]
    pub(crate) fn set_snapshot_gate(&self, gate: Option<std::sync::Arc<dyn Fn() + Send + Sync>>) {
        *self.snapshot_gate.lock().expect("snapshot gate poisoned") = gate;
    }

    /// Test-only: fire the PASS snapshot gate, if one is installed. The Arc
    /// is cloned out before the call so the closure may block freely.
    #[cfg(test)]
    pub(crate) fn run_snapshot_gate(&self) {
        let gate = self
            .snapshot_gate
            .lock()
            .expect("snapshot gate poisoned")
            .clone();
        if let Some(gate) = gate {
            gate();
        }
    }

    /// Test-only: install (or remove) the compat page gate. See the field
    /// doc.
    #[cfg(test)]
    pub(crate) fn set_compat_gate(&self, gate: Option<std::sync::Arc<dyn Fn() + Send + Sync>>) {
        *self.compat_gate.lock().expect("compat gate poisoned") = gate;
    }

    /// Test-only: fire the compat page gate, if one is installed. The Arc
    /// is cloned out before the call so the closure may block freely.
    #[cfg(test)]
    pub(crate) fn run_compat_gate(&self) {
        let gate = self
            .compat_gate
            .lock()
            .expect("compat gate poisoned")
            .clone();
        if let Some(gate) = gate {
            gate();
        }
    }

    /// Subscribe to every accepted email (webhook dispatcher).
    pub fn subscribe_all(&self) -> broadcast::Receiver<Arc<Email>> {
        self.global_tx.subscribe()
    }

    /// Insert an email. Lossless: this is synchronous — once SMTP accepted the
    /// message, it is queryable (and on disk, when persisting). Prunes oldest
    /// beyond the per-inbox cap.
    ///
    /// The mirror commit is the one failure mode (the memory phase has no
    /// fallible step): a failed commit is returned, never swallowed, so an
    /// acceptance path can refuse instead of acknowledging a mail the
    /// restart copy will never hold (decision 0012). Zero trace (decision
    /// 0014): the refusal applies nothing — no mail, no inbox minted, no
    /// counter moved, no fan-out — so memory and data file agree and the
    /// same wire path can simply retry once storage recovers.
    ///
    /// From an async handler, prefer the awaited [`Store::insert_async`]: it
    /// runs this whole mutation on tokio's blocking pool, so the mirror
    /// commit — which completes before the wrapper resolves — never occupies
    /// an async worker (decision 0023). The ordering is identical either way:
    /// the commit outcome is known before the caller replies (decision 0001).
    pub fn insert(&self, email: Email) -> Result<Arc<Email>, rusqlite::Error> {
        let email = Arc::new(email);
        let inbox = email.inbox.clone();
        // The boundary spans plan → commit → apply (decision 0014): the
        // memory phase and the mirror commit are one mutation, so no other
        // mutation can prune this mail's id between its plan and its commit
        // — and push order is commit order.
        let _guard = self.mutation.lock().expect("store mutation lock poisoned");
        // PLAN: a read-only peek. No `entry()` here — a refused commit must
        // not mint an inbox (the ghost-inbox wart dies with the boundary);
        // the apply phase mints it only on success.
        let peeked = self.inboxes.get(&inbox);
        let overflow = if self.max_per_inbox > 0 {
            (peeked.as_ref().map_or(0, |l| l.value().len()) + 1).saturating_sub(self.max_per_inbox)
        } else {
            0
        };
        // The post-counters the commit mirrors: the atomics' next values,
        // computed now because the apply phase has not bumped them yet.
        let inserted_total = self.totals.emails_inserted() + 1;
        let dropped_total = self.totals.emails_dropped() + overflow as u64;
        // COMMIT before visibility (decision 0001 + 0014): one transaction —
        // the row, the pruned ids' deletes, the post-counters — inside the
        // boundary. A failed commit propagates: the `?` refuses the mail
        // before anything is applied, so the refusal leaves memory and data
        // file agreeing and there is nothing to roll back.
        {
            let pruned_ids: Vec<&str> = peeked
                .as_ref()
                .map(|l| {
                    l.value()
                        .iter()
                        .take(overflow)
                        .map(|e| e.id.as_str())
                        .collect()
                })
                .unwrap_or_default();
            self.record("insert", |persist| {
                persist.record_insert(&email, &pruned_ids, inserted_total, dropped_total)
            })?;
        }
        // The peek's shard read guard must be gone before the write phase
        // takes the same shard: one thread, one guard at a time.
        drop(peeked);
        // APPLY, only on Ok: push, prune, index and counter are one unit —
        // the deque and the index can never disagree, and the pruned ids are
        // exactly the rows the commit deleted (no other mutation can run
        // inside the boundary). `pop_front` evicts shift-free, where a Vec's
        // `drain(..overflow)` memmoves every remaining element on every
        // capped insert (781 KiB per accept at the default cap — the bulk of
        // the steady-state bench's number). The loop is general, not a
        // single `pop_front`: a restore over a lowered cap owes more than
        // one eviction on the first insert, and the `expect` is an invariant
        // — a list above its cap always has a front to evict (0003).
        let mut list = self.inboxes.entry(inbox.clone()).or_default();
        list.push_back(email.clone());
        for _ in 0..overflow {
            let oldest = list
                .pop_front()
                .expect("a list above its cap has a front to evict");
            self.index.remove(&oldest.id);
        }
        self.index.insert(email.id.clone(), email.clone());
        self.totals
            .emails_dropped
            .fetch_add(overflow as u64, Ordering::Relaxed);
        self.totals.emails_inserted.fetch_add(1, Ordering::Relaxed);
        drop(_guard);
        // Notify watchers — outside the boundary tail; a full channel means
        // a slow await reader — the email is still safely stored, the
        // watcher re-polls the store. A missing entry means no subscribers
        // yet, or the channel was released with the inbox's mail (decision
        // 0021): the waiter re-subscribes before its next scan, so the
        // store-is-truth rescan still finds this mail. Fan-out is part of
        // acceptance and happens only after the commit it depends on.
        //
        // The whole truth about the shard write guard: `list` — the write
        // guard `entry()` took on the inbox's shard — is a Drop guard, so
        // it lives to the end of this scope, across these sends. That is
        // harmless by construction, not an oversight: a broadcast `send`
        // never blocks (a full channel's error is ignored above) and the
        // webhook dispatcher owns its deliveries on its own task — a hung
        // sink can therefore never hold the shard, and never hold this
        // mutation boundary. What a reader on another thread may see here
        // is at worst a shard locked by a send, i.e. for nanoseconds.
        if let Some(sender) = self.watchers.get(&inbox) {
            let _ = sender.send(email.clone());
        }
        let _ = self.global_tx.send(email.clone());
        Ok(email)
    }

    pub fn get(&self, id: &str) -> Option<Arc<Email>> {
        self.index.get(id).map(|e| e.clone())
    }

    /// Newest-first listing for `inbox` matching `filter`.
    pub fn list(&self, inbox: &str, filter: &Filter) -> Vec<Arc<Email>> {
        match self.inboxes.get(inbox) {
            Some(list) => list
                .iter()
                .rev()
                .filter(|e| filter.matches(e))
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    /// The newest `cap` emails in `inbox` matching `filter`, newest first.
    ///
    /// The filter is applied **before** the bound: the result is the newest
    /// `cap` *matches*, never the newest mails filtered down — exactly the
    /// `list(inbox, filter).into_iter().take(cap)` window, walked only as
    /// far along the list as the cap needs. The thread view's cap is the
    /// caller this exists for: an inbox at the default cap is scanned no
    /// deeper than the window it renders.
    pub fn list_bounded(&self, inbox: &str, filter: &Filter, cap: usize) -> Vec<Arc<Email>> {
        match self.inboxes.get(inbox) {
            Some(list) => list
                .iter()
                .rev()
                .filter(|e| filter.matches(e))
                .take(cap)
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    /// Newest-first listing across ALL inboxes matching `filter`.
    ///
    /// The MailHog/Mailpit compat shims expose a single global mailbox, so
    /// they read the whole store through this (and sort one inbox's `list`
    /// the same way when the request names one, decision 0049 §5).
    /// Insertion order is only per-inbox, so the merged view is ordered by
    /// [`newest_first`].
    pub fn list_all(&self, filter: &Filter) -> Vec<Arc<Email>> {
        let mut all: Vec<Arc<Email>> = Vec::new();
        for entry in self.inboxes.iter() {
            all.extend(entry.value().iter().filter(|e| filter.matches(e)).cloned());
        }
        all.sort_by(newest_first);
        all
    }

    /// The newest mail in scope — one inbox, or the whole store for `None` —
    /// by `(received_ms, id)`, the order every compat listing uses (issue
    /// #95, decision 0044 §4; decision 0049 §5). Native inbox listings
    /// (`list`), POP3 numbering and pruning use acceptance order instead,
    /// which can differ. One pass over every scoped mail under
    /// short-lived shard read guards, retaining only the running winner: no
    /// sorted page and no mutation lock. Deliberately not a deque's tail —
    /// the receive stamp is sampled at parse start, before acceptance, so
    /// commit order and stamp order can differ even within one inbox. A
    /// DashMap walk is not an atomic snapshot of the whole store.
    pub fn latest(&self, inbox: Option<&str>) -> Option<Arc<Email>> {
        let mut winner: Option<Arc<Email>> = None;
        let mut consider = |list: &VecDeque<Arc<Email>>| {
            for email in list {
                let newer = winner
                    .as_ref()
                    .is_none_or(|w| (email.received_ms, &email.id) > (w.received_ms, &w.id));
                if newer {
                    winner = Some(email.clone());
                }
            }
        };
        match inbox {
            Some(inbox) => {
                if let Some(list) = self.inboxes.get(inbox) {
                    consider(list.value());
                }
            }
            None => {
                for entry in self.inboxes.iter() {
                    consider(entry.value());
                }
            }
        }
        winner
    }

    /// How many emails in `inbox` match `filter`.
    ///
    /// A count inspects matching mail; it does not need to own it — the scan
    /// is the inherent cost, the per-match `Arc` clones and the `Vec` they
    /// land in are not. Decision 0010's exactness is untouched: the same
    /// filter over the same list, counted, is the same integer `list` would
    /// return a length for.
    pub fn count(&self, inbox: &str, filter: &Filter) -> usize {
        match self.inboxes.get(inbox) {
            Some(list) => list.iter().filter(|e| filter.matches(e)).count(),
            None => 0,
        }
    }

    pub fn delete(&self, id: &str) -> Result<bool, rusqlite::Error> {
        // The boundary spans plan → commit → apply (decision 0014): the
        // mirror delete and the memory removal are one mutation, so the
        // row cannot outlive the memory it belongs to nor the reverse.
        let _guard = self.mutation.lock().expect("store mutation lock poisoned");
        // COMMIT first: the mirror deletes the row before the memory phase
        // drops the mail. A no-op DELETE — an id the store never held — is
        // still committed `Ok` (it deletes nothing), so an absent id stays
        // an honest `Ok(false)` and an orphaned row can never linger.
        //
        // The cost of that guard is one bounded commit per 404: an absent
        // id pays a real (empty) transaction against the mirror — its own
        // busy bound, per 0015 §2 — instead of a cheap `!self.index`
        // shortcut. Deliberate: the mirror is the one copy the restart
        // restores from, so the only way to *know* no row lingers under
        // this id is to ask the mirror, and the boundary is where that
        // question is ordered (0014 §2). The 404 path is not the hot path
        // (an id the store handed out is present by construction); when a
        // caller deletes a suffix it invented, it pays for the honesty.
        self.record("delete", |persist| persist.record_delete(id))?;
        // APPLY, only on Ok: index removal, deque retain, watcher release
        // and the emptied-entry removal are one unit — the index and the
        // deque can never disagree.
        let removed = self.index.remove(id);
        if let Some((_, email)) = &removed {
            // Every indexed mail was inserted with its inbox list present
            // (insert creates both together), so this is an invariant, not
            // a search: a missing list would mean the two maps disagree.
            let mut list = self
                .inboxes
                .get_mut(&email.inbox)
                .expect("indexed mail always has its inbox list");
            list.retain(|e| e.id != id);
            // One rule on every path (decision 0032): an inbox whose mail is
            // gone does not exist. `clear`/`clear_all` remove the entry via
            // `drain_inbox`, a restart drops it (`restore` rebuilds entries
            // only for inboxes with rows) — so a delete that empties the
            // list removes it here too, releasing the watcher channel with
            // it (decision 0021) exactly as the drains do. No tombstone:
            // the next insert re-mints the entry (0004), and POP3's `USER`
            // refuses the maildrop like any it never saw.
            let emptied = list.is_empty();
            drop(list);
            if emptied {
                self.watchers.remove(&email.inbox);
                // The deque guard is dropped first: `get_mut` holds this
                // shard's write lock, and a nested `remove` on the same map
                // — same shard or not — would deadlock against it (the
                // DashMap reentrancy rule `drain_inbox` is ordered by too).
                self.inboxes.remove(&email.inbox);
            }
        }
        Ok(removed.is_some())
    }

    /// Remove all emails from one inbox; returns how many were removed.
    pub fn clear(&self, inbox: &str) -> Result<usize, rusqlite::Error> {
        // The boundary spans plan → commit → apply (decision 0014): the
        // mirror deletes and the drain are one mutation.
        let _guard = self.mutation.lock().expect("store mutation lock poisoned");
        // PLAN: the ids to delete, snapshotted under the boundary. The
        // commit runs against exactly these ids and the drain applies
        // exactly these ids — no other mutation can run in between to
        // widen or shrink the set.
        let drained_ids: Vec<Arc<Email>> = self
            .inboxes
            .get(inbox)
            .map(|l| l.value().iter().cloned().collect())
            .unwrap_or_default();
        let ids: Vec<&str> = drained_ids.iter().map(|e| e.id.as_str()).collect();
        // COMMIT before visibility: per-id deletes — never `WHERE inbox =
        // ?`. The boundary holds the whole mutation, so a racing insert
        // cannot land between the plan and the commit; per-id deletes
        // remain the honest mirror of a per-id plan whatever the data file
        // holds (the clear/insert persistence race, review finding in run
        // sqlite-persistence-data-file-swa-dc3a9a2a, is closed by the
        // boundary itself).
        self.record("clear_inbox", |persist| persist.record_deletes(&ids))?;
        // APPLY, only on Ok: the drain is the memory phase.
        let drained = drain_inbox(&self.inboxes, &self.index, &self.watchers, inbox);
        debug_assert_eq!(drained.len(), ids.len(), "the drain must match the plan");
        Ok(drained.len())
    }

    /// Remove every email from every inbox.
    pub fn clear_all(&self) -> Result<usize, rusqlite::Error> {
        // The boundary spans plan → commit → apply (decision 0014).
        let _guard = self.mutation.lock().expect("store mutation lock poisoned");
        // PLAN: snapshot every inbox's ids under the boundary.
        let mut drained_ids: Vec<Arc<Email>> = Vec::new();
        for entry in self.inboxes.iter() {
            drained_ids.extend(entry.value().iter().cloned());
        }
        let ids: Vec<&str> = drained_ids.iter().map(|e| e.id.as_str()).collect();
        // COMMIT: per-id deletes, for the same reason as `clear`.
        self.record("clear_all", |persist| persist.record_deletes(&ids))?;
        // APPLY, only on Ok: every drain plus the sweep's watcher release —
        // the population issue #14's probe creates, which leaves no entries
        // for the per-inbox drain above to iterate (decision 0021). A
        // subscriber racing the sweep sees `Closed` and takes the
        // re-subscribe arm — bounded at one extra rescan per waiter.
        let inboxes: Vec<String> = self.inboxes.iter().map(|e| e.key().clone()).collect();
        for inbox in inboxes {
            drain_inbox(&self.inboxes, &self.index, &self.watchers, &inbox);
        }
        self.watchers.clear();
        Ok(drained_ids.len())
    }

    /// Runs a whole mutation on tokio's blocking pool. The returned future
    /// resolves only after the mutation has run to completion — memory
    /// mutated, prune applied, mirror commit attempted — so a `250` written
    /// after it still means queryable AND durable (decision 0001: the disk
    /// leg is ordering, not thread identity). A panicking mutation
    /// re-raises in the awaiting caller, exactly as the synchronous call
    /// would.
    ///
    /// The explicit `match` + `resume_unwind` is deliberate: the `Err` arm
    /// is the panic-symmetry contract, and a `.await.unwrap()` here would
    /// mint an uncovered branch and lose the re-raise-in-caller shape.
    /// `pub(crate)` since decision 0030 §1: the SMTP DATA arm and the seed
    /// path park their whole parse here too — same pool, same panic
    /// symmetry, no second offload helper.
    pub(crate) async fn offload<T, F>(task: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        match tokio::task::spawn_blocking(task).await {
            Ok(value) => value,
            Err(join_error) => std::panic::resume_unwind(join_error.into_panic()),
        }
    }

    /// [`Store::insert`] off the async workers: the whole mutation — memory
    /// insert, oldest-only prune (0003), mirror commit under
    /// `synchronous = FULL`, fan-out — runs on tokio's blocking pool and the
    /// future resolves only once it has completed. Awaited before any
    /// acceptance reply, an SMTP `250` still means the mail is queryable and
    /// committed (decision 0001); the commit merely stops occupying an async
    /// worker (decision 0023). Panics propagate to the awaiting caller, and
    /// so does a failed commit — the `Err` is the refusal the acceptance
    /// path answers with (decision 0012).
    pub async fn insert_async(
        self: Arc<Self>,
        email: Email,
    ) -> Result<Arc<Email>, rusqlite::Error> {
        Self::offload(move || self.insert(email)).await
    }

    /// [`Store::delete`] off the async workers. See [`Store::insert_async`].
    pub async fn delete_async(self: Arc<Self>, id: String) -> Result<bool, rusqlite::Error> {
        Self::offload(move || self.delete(&id)).await
    }

    /// [`Store::clear`] off the async workers. See [`Store::insert_async`].
    pub async fn clear_async(self: Arc<Self>, inbox: String) -> Result<usize, rusqlite::Error> {
        Self::offload(move || self.clear(&inbox)).await
    }

    /// [`Store::clear_all`] off the async workers. See [`Store::insert_async`].
    pub async fn clear_all_async(self: Arc<Self>) -> Result<usize, rusqlite::Error> {
        Self::offload(move || self.clear_all()).await
    }

    /// (inbox, count) pairs, sorted by name.
    pub fn inboxes(&self) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = self
            .inboxes
            .iter()
            .map(|e| (e.key().clone(), e.value().len()))
            .collect();
        out.sort();
        out
    }

    /// Whether `name` is a live inbox — one map lookup (issue #86). POP3's
    /// `USER` and `PASS` ask exactly this; enumerating and sorting every
    /// name through [`Store::inboxes`] to answer it scaled with the inbox
    /// count. An emptied inbox does not exist (decision 0032), so it
    /// answers `false` like one never seen.
    pub fn has_inbox(&self, name: &str) -> bool {
        self.inboxes.contains_key(name)
    }

    /// Subscribe to new-mail notifications for an inbox (lazy per-inbox
    /// channel).
    ///
    /// One atomic path: the `entry()` holds the shard write lock across
    /// channel creation, so concurrent first subscribers serialize on one
    /// boundary and every receiver attaches to the same live channel — the
    /// get-then-insert window is deleted, not fenced (decision 0021). The
    /// next `subscribe` after a release re-creates the channel.
    pub fn subscribe(&self, inbox: &str) -> broadcast::Receiver<Arc<Email>> {
        self.watchers
            .entry(inbox.to_string())
            .or_insert_with(|| broadcast::channel(WATCHER_CAP).0)
            .subscribe()
    }
}

/// The result of a bounded wait for matching mail.
pub enum WaitOutcome {
    /// At least `want` matches exist; holds up to `want`, newest first.
    Found(Vec<Arc<Email>>),
    /// The deadline elapsed; `matched` is the current match count.
    Timeout { matched: usize },
}

impl Store {
    /// Wait until `want` emails matching `filter` exist in `inbox`, or the
    /// timeout elapses.
    ///
    /// Subscribes **before** scanning so a mail delivered during the check
    /// cannot be missed (decision 0002), and the ordering is re-applied **per
    /// iteration**: a released channel (`Closed` — the inbox's mail was
    /// removed and its watcher released) is answered with exactly one
    /// re-subscribe before the next scan, never a spin and never a miss.
    /// The store remains the source of truth: every wakeup re-scans, so
    /// slow or lagging watchers are safe — a full channel (`Lagged`) and the
    /// elapsed deadline both fall through to the re-check.
    pub async fn wait_for(
        &self,
        inbox: &str,
        filter: &Filter,
        want: usize,
        timeout: Duration,
    ) -> WaitOutcome {
        let mut rx = self.subscribe(inbox);
        #[cfg(test)]
        self.run_post_subscribe_gate();
        let deadline = Instant::now() + timeout;
        loop {
            let mut matches = self.list(inbox, filter);
            if matches.len() >= want {
                matches.truncate(want);
                return WaitOutcome::Found(matches);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return WaitOutcome::Timeout {
                    matched: matches.len(),
                };
            }
            // Block until any event (new mail, lag, channel close) or the
            // deadline; the loop re-scans either way. Every `recv` parks on
            // a live channel unless the channel is closed, and a closed one
            // is answered by the single re-subscribe below — there is no
            // schedule in which an iteration does neither (decision 0021).
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(_)) | Ok(Err(RecvError::Lagged(_))) => {}
                Ok(Err(RecvError::Closed)) => {
                    // The inbox's mail was removed and its channel released:
                    // re-apply decision 0002 per iteration — subscribe, then
                    // scan.
                    rx = self.subscribe(inbox);
                    #[cfg(test)]
                    self.run_post_subscribe_gate();
                    #[cfg(test)]
                    CLOSED_PROBE.fetch_add(1, Ordering::Relaxed);
                }
                Err(_elapsed) => {} // parked past the deadline; the next scan reports it
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn email(id: &str, inbox: &str, to: &str) -> Email {
    Email {
        id: id.to_string(),
        inbox: inbox.to_string(),
        from: None,
        // The fixture has no SMTP session behind it: unknown provenance,
        // the shape a row restored from before 0044 has.
        envelope_from: None,
        to: vec![crate::model::EmailAddress {
            name: None,
            address: to.to_string(),
        }],
        cc: vec![],
        recipients: vec![to.to_string()],
        subject: Some("hi".into()),
        received_at: "2026-01-01T00:00:00Z".into(),
        received_ms: 0,
        size: 1,
        text: None,
        html: None,
        links: vec![],
        codes: vec![],
        message_id: None,
        in_reply_to: None,
        references: vec![],
        raw: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `has_inbox` answers present, absent and emptied: the first mail
    /// mints the inbox, a delete that empties it removes it (0032).
    #[test]
    fn has_inbox_answers_present_absent_and_emptied() {
        let store = Store::new(0);
        assert!(!store.has_inbox("pop"));
        let _ = store.insert(email("1", "pop", "a@x.io"));
        assert!(store.has_inbox("pop"));
        assert!(!store.has_inbox("other"));
        assert!(store.delete("1").unwrap());
        assert!(!store.has_inbox("pop"));
    }

    /// Pruning follows acceptance order, never the timestamp (decision
    /// 0038): mail stamped deliberately *later* but accepted first is the
    /// oldest and the one evicted. Synthetic stamps, no clock.
    #[test]
    fn pruning_follows_acceptance_not_received_ms() {
        let store = Store::new(1);
        let mut first = email("a", "order", "a@x.io");
        first.received_ms = 2_000;
        let mut second = email("b", "order", "a@x.io");
        second.received_ms = 1_000;
        let _ = store.insert(first);
        let _ = store.insert(second);
        let kept: Vec<String> = store
            .list("order", &Filter::default())
            .iter()
            .map(|e| e.id.clone())
            .collect();
        assert_eq!(kept, vec!["b".to_string()]);
    }

    #[test]
    fn insert_prunes_oldest_beyond_cap() {
        let store = Store::new(2);
        let _ = store.insert(email("1", "default", "a@x.io"));
        let _ = store.insert(email("2", "default", "a@x.io"));
        let _ = store.insert(email("3", "default", "a@x.io"));
        assert_eq!(store.count("default", &Filter::default()), 2);
        assert!(store.get("1").is_none());
        assert!(store.get("2").is_some());
        assert!(store.get("3").is_some());
    }

    /// The shutdown refusal at the boundary (decision 0024, inline per 0020
    /// rule 1): a wired store whose stop has fired refuses the commit
    /// *before* any write — the constructed `SQLITE_ABORT`, nothing
    /// applied: no mail, no inbox minted (the PLAN peek never runs), no
    /// counter moved, no watcher or webhook fan-out. The store here is
    /// memory-only on purpose: the policy is the boundary's, not the
    /// mirror's, so the refusal must fire with `persist` unset. The same
    /// store, wired but not stopping, inserts normally (the false arm);
    /// the unwired arm is every other test in this suite.
    #[test]
    fn a_wired_stopping_store_refuses_the_commit_and_applies_nothing() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let store = Store::new(0);
        store.observe_shutdown(rx);
        // Wired, not stopping: the normal path.
        let _ = store.insert(email("live", "default", "a@x.io"));
        assert_eq!(store.count("default", &Filter::default()), 1);
        let mut global_rx = store.subscribe_all();

        let _ = tx.send(true);
        // Asserted through the error's own accessors, not a match: a
        // never-taken panic arm in this test would sit in `src/` and count
        // against the exact coverage denominator.
        let err = store
            .insert(email("refused", "fresh-box", "b@x.io"))
            .expect_err("a stopping store must refuse the insert");
        assert_eq!(
            err.sqlite_error_code(),
            Some(ffi::ErrorCode::OperationAborted)
        );
        assert_eq!(
            err.sqlite_error().map(|e| e.extended_code),
            Some(ffi::SQLITE_ABORT)
        );
        // `Display` for a `SqliteFailure` with a message prints that message.
        assert_eq!(err.to_string(), "shutting down");
        // The refusal applied nothing — memory and data file agree because
        // memory never moved.
        assert!(store.get("refused").is_none());
        assert_eq!(store.count("fresh-box", &Filter::default()), 0);
        assert!(!store.inboxes.contains_key("fresh-box"));
        assert_eq!(store.emails_inserted(), 1);
        assert_eq!(store.emails_dropped(), 0);
        assert_eq!(store.watcher_count(), 0);
        // No fan-out: the webhook firehose carries only the live insert.
        assert!(global_rx.try_recv().is_err());
    }

    #[test]
    fn filter_by_recipient() {
        let store = Store::new(0);
        let _ = store.insert(email("1", "default", "a@x.io"));
        let _ = store.insert(email("2", "default", "b@x.io"));
        let f = Filter {
            to: Some("b@x.io".into()),
            ..Default::default()
        };
        assert_eq!(store.count("default", &f), 1);
        assert_eq!(store.list("default", &f)[0].id, "2");
    }

    #[test]
    fn list_all_merges_inboxes_newest_first() {
        let store = Store::new(0);
        assert!(store.list_all(&Filter::default()).is_empty());

        let mut old = email("old", "one", "a@x.io");
        old.received_ms = 100;
        let mut new = email("new", "two", "a@x.io");
        new.received_ms = 200;
        let _ = store.insert(old);
        let _ = store.insert(new);

        let all = store.list_all(&Filter::default());
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "new");
        assert_eq!(all[1].id, "old");

        // The id tiebreak: two mails with the same received_ms sort by id,
        // descending (newest-first carries to the tiebreak). Without this,
        // the tiebreak closure runs only when the wall clock hands two real
        // inserts the same millisecond — a coin flip the exact gate
        // refuses. Branch-stable, inline (0020 rule 1).
        let mut tie_aaa = email("tie-aaa", "three", "a@x.io");
        tie_aaa.received_ms = 100;
        let mut tie_zzz = email("tie-zzz", "three", "a@x.io");
        tie_zzz.received_ms = 100;
        let _ = store.insert(tie_aaa);
        let _ = store.insert(tie_zzz);
        let all = store.list_all(&Filter::default());
        let aaa = all
            .iter()
            .position(|e| e.id == "tie-aaa")
            .expect("tie mail present");
        let zzz = all
            .iter()
            .position(|e| e.id == "tie-zzz")
            .expect("tie mail present");
        assert!(
            zzz < aaa,
            "equal received_ms must fall back to the id tiebreak, descending: {all:?}"
        );

        // A filter that matches nothing, the way the compat shims scope.
        let none = Filter {
            from: Some("nobody@nowhere".into()),
            ..Default::default()
        };
        assert!(store.list_all(&none).is_empty());
    }

    #[test]
    fn inboxes_are_isolated() {
        let store = Store::new(0);
        let _ = store.insert(email("1", "test-a", "a@x.io"));
        let _ = store.insert(email("2", "test-b", "b@x.io"));
        assert_eq!(store.count("test-a", &Filter::default()), 1);
        assert_eq!(store.count("test-b", &Filter::default()), 1);
        let _ = store.clear("test-a");
        assert_eq!(store.count("test-a", &Filter::default()), 0);
        assert_eq!(store.count("test-b", &Filter::default()), 1);
    }

    /// Eight threads race to be the first subscriber of one fresh inbox
    /// (decision 0021, AC1): registration is atomic, so every receiver
    /// attaches to the *same live* channel and one insert is seen by all
    /// eight. On the pre-fix get-then-insert code this failed whenever two
    /// threads overlapped — the displaced receivers hung off a sender the
    /// map no longer held. Seam-free by design: post-fix the assertion holds
    /// under every schedule, so the race is deleted, not controlled.
    #[test]
    fn concurrent_first_subscribers_all_attach_and_all_see_the_insert() {
        let store = Arc::new(Store::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let mut receivers = std::thread::scope(|s| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let store = Arc::clone(&store);
                    let barrier = Arc::clone(&barrier);
                    s.spawn(move || {
                        barrier.wait();
                        store.subscribe("race")
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        let _ = store.insert(email("race-1", "race", "a@x.io"));
        for (i, rx) in receivers.iter_mut().enumerate() {
            assert_eq!(
                rx.try_recv().map(|e| e.id.clone()).unwrap(),
                "race-1",
                "receiver {i} attached to a displaced channel"
            );
        }
    }

    /// `clear` is where an inbox dies, so it is where its watcher channel
    /// dies (decision 0021, AC2-i): the receiver held across the clear
    /// observes the release. An empty clear still releases — the channel is
    /// not immortal — and the held receiver is safe: its waiter answers
    /// `Closed` with a re-subscribe before its next scan.
    #[test]
    fn a_cleared_inbox_releases_its_watcher_channel() {
        let store = Store::new(0);
        let mut rx = store.subscribe("box");
        assert_eq!(store.clear("box").unwrap(), 0);
        assert!(
            matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Closed)),
            "the channel held across the clear must observe the release"
        );
    }

    /// A `delete` that empties an inbox's list releases the channel
    /// (decision 0021, AC2-ii); a delete that leaves mail behind must keep
    /// it live — a pushed insert still reaches the held receiver.
    #[test]
    fn a_delete_that_empties_an_inbox_releases_its_watcher_channel() {
        let store = Store::new(0);
        let mut rx = store.subscribe("box");
        let _ = store.insert(email("d1", "box", "a@x.io"));
        let _ = store.insert(email("d2", "box", "a@x.io"));
        // Drain the two buffered pushes so later asserts read only pushes
        // that land after the deletes.
        assert_eq!(rx.try_recv().map(|e| e.id.clone()).unwrap(), "d1");
        assert_eq!(rx.try_recv().map(|e| e.id.clone()).unwrap(), "d2");
        let _ = store.delete("d1");
        let _ = store.insert(email("d3", "box", "a@x.io"));
        assert_eq!(
            rx.try_recv().map(|e| e.id.clone()).unwrap(),
            "d3",
            "a non-emptying delete keeps the channel live"
        );
        let _ = store.delete("d2");
        let _ = store.delete("d3"); // the list is now empty: the channel is released
        assert!(
            matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Closed)),
            "a delete that empties the inbox must release the channel"
        );
    }

    /// `clear_all` sweeps every watcher channel, including the
    /// awaited-but-never-mailed population no per-inbox drain can reach —
    /// issue #14's 5,000-fresh-inbox probe (decision 0021, AC2-iii): the
    /// test-visible count returns to baseline, and a held receiver observes
    /// the release.
    #[test]
    fn clear_all_releases_channels_of_awaited_never_mailed_inboxes() {
        let store = Store::new(0);
        const N: usize = 3;
        let mut held = None;
        for i in 0..N {
            held = Some(store.subscribe(&format!("awaited{i}")));
        }
        assert_eq!(
            store.watcher_count(),
            N,
            "each fresh subscription created a channel"
        );
        let _ = store.clear_all();
        assert_eq!(
            store.watcher_count(),
            0,
            "the sweep returns the count to baseline"
        );
        assert!(
            matches!(
                held.unwrap().try_recv(),
                Err(broadcast::error::TryRecvError::Closed)
            ),
            "the sweep released the held channel"
        );
    }

    /// One rule on every path (decision 0032, issue #54): a `delete` that
    /// empties an inbox removes its entry — the listing `inboxes()` serves
    /// (and POP3's `has_inbox` follows) — releasing the watcher channel
    /// with it, exactly as `clear` does via `drain_inbox`. The live store
    /// and a reopen of the same data file agree. This fixture inserts a new
    /// row to re-mint the inbox before reopening, so restore checks that new
    /// row rebuilds the entry; it does not claim an empty inbox stays absent
    /// across a restart. This is the hang regression: retaining a map guard
    /// while removing the emptied entry deadlocks on that same map shard.
    #[test]
    fn a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen() {
        // The data file and every sidecar (-wal, -shm, the lock) live inside
        // the guard's directory, which goes on any exit — an assertion that
        // fails mid-test leaves nothing in the temporary root (issue #143).
        let dir = tempfile::TempDir::with_prefix("swarmail-emptied-inbox-").unwrap();
        let path = dir.path().join("mail.db");
        {
            let store = Store::open(0, &path).unwrap();
            let mut rx = store.subscribe("gone");
            let _ = store.insert(email("e1", "gone", "a@x.io"));
            let _ = store.insert(email("other", "kept", "a@x.io"));
            // Drain the buffered pushes so the release below is observable.
            let _ = rx.try_recv();
            let _ = store.delete("e1");
            assert!(!store.has_inbox("gone"), "the emptied inbox is absent");
            assert!(
                matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Closed)),
                "the emptied inbox's channel is released with the entry"
            );
            assert_eq!(
                store.watcher_count(),
                0,
                "no watcher channels survive the emptied inbox"
            );
            assert_eq!(
                store
                    .inboxes()
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>(),
                vec!["kept"],
                "the emptied inbox is out of the listing"
            );
            // The next insert re-mints the inbox (0004): no tombstone.
            let _ = store.insert(email("e2", "gone", "a@x.io"));
            assert!(store.inboxes().iter().any(|(n, _)| n == "gone"));
        }
        {
            let store = Store::open(0, &path).unwrap();
            let listing = store.inboxes();
            let names = listing.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>();
            assert_eq!(
                names,
                vec!["gone", "kept"],
                "restore rebuilds entries only for inboxes with rows"
            );
            // The single lookup agrees with the listing after restore.
            assert!(store.has_inbox("gone") & store.has_inbox("kept"));
            assert!(!store.has_inbox("never"));
        }
    }

    /// The store carries the data file's single-writer refusal (decision
    /// 0033): a second `Store::open` on a held file is `InUse`, a fresh
    /// store counts nothing for its sidecar — no inbox, no mail, zero
    /// counters — and dropping the owner frees the file for a reopen.
    #[test]
    fn a_held_data_file_refuses_a_second_store_and_its_sidecar_counts_nothing() {
        let dir = tempfile::TempDir::with_prefix("swarmail-store-in-use-").unwrap();
        let path = dir.path().join("mail.db");
        let first = Store::open(0, &path).unwrap();
        assert!(dir.path().join("mail.db.lock").is_file());
        assert!(first.inboxes().is_empty(), "the sidecar is no inbox");
        assert!(first.list_all(&Filter::default()).is_empty());
        assert_eq!((first.emails_inserted(), first.emails_dropped()), (0, 0));
        let err = Store::open(0, &path)
            .err()
            .expect("a held data file refuses a second store");
        assert_eq!(format!("{err:?}"), format!("InUse({path:?})"));
        drop(first);
        let reopened = Store::open(0, &path).expect("a dropped owner frees it");
        assert!(reopened.inboxes().is_empty());
    }
}

#[cfg(test)]
mod wait_tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_finds_what_is_already_there() {
        let store = Store::new(0);
        let _ = store.insert(email("w1", "box", "a@x.io"));
        let out = store
            .wait_for("box", &Filter::default(), 1, Duration::from_millis(100))
            .await;
        match out {
            WaitOutcome::Found(matches) => assert_eq!(matches.len(), 1),
            WaitOutcome::Timeout { .. } => panic!("should have found"),
        }
    }

    #[tokio::test]
    async fn wait_for_wakes_on_arrival() {
        let store = Arc::new(Store::new(0));
        let s = store.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = s.insert(email("w2", "box", "a@x.io"));
        });
        let out = store
            .wait_for("box", &Filter::default(), 1, Duration::from_secs(2))
            .await;
        match out {
            WaitOutcome::Found(matches) => assert_eq!(matches[0].id, "w2"),
            WaitOutcome::Timeout { .. } => panic!("the watcher should have woken"),
        }
    }

    #[tokio::test]
    async fn wait_for_times_out_with_the_match_count() {
        let store = Store::new(0);
        let _ = store.insert(email("w3", "box", "a@x.io"));
        let out = store
            .wait_for("box", &Filter::default(), 5, Duration::from_millis(80))
            .await;
        match out {
            WaitOutcome::Timeout { matched } => assert_eq!(matched, 1),
            WaitOutcome::Found(_) => panic!("5 cannot be reached"),
        }
    }

    #[tokio::test]
    async fn wait_for_covers_the_lagging_watcher() {
        // A watcher whose channel fills must not hang the wait: the store
        // re-scan is the source of truth (the loop falls through on Lagged).
        let store = Arc::new(Store::new(0));
        let s = store.clone();
        let handle = {
            let store = store.clone();
            tokio::spawn(async move {
                store
                    .wait_for("box", &Filter::default(), 3, Duration::from_secs(5))
                    .await
            })
        };
        // The subscribe happens inside wait_for; give it a beat, then
        // overflow the 16-slot wake ring (WATCHER_CAP): lag is repaired by
        // the rescan, not by ring headroom (decision 0021, AC5).
        tokio::time::sleep(Duration::from_millis(100)).await;
        for i in 0..4200u32 {
            let mut e = email(&format!("lag{i}"), "box", "a@x.io");
            e.id = format!("lag{i}");
            let _ = s.insert(e);
        }
        match handle.await.unwrap() {
            WaitOutcome::Found(matches) => assert_eq!(matches.len(), 3),
            WaitOutcome::Timeout { .. } => panic!("the store had the mails"),
        }
    }

    /// A released channel cannot strand a waiter (decision 0021, AC3). The
    /// gate fires once after the initial subscribe — the clear releases the
    /// waiter's channel — and once after the `Closed` arm's re-subscribe,
    /// inserting the mail; the wait must return `Found`. Red on all three
    /// regressions: no release on clear (the insert only happens in the
    /// re-subscribe gate → parks to `Timeout`), no re-subscribe on `Closed`
    /// (the gate never fires again → `Timeout`), and sleep-through-on-
    /// `Closed` (`Timeout`). The asserted state is unreachable without the
    /// controlled interleaving, and the insert being observed proves the
    /// 0002 per-iteration ordering: the re-subscribe precedes the rescan.
    #[tokio::test]
    async fn a_waiter_on_a_released_channel_is_not_stranded() {
        let store = Arc::new(Store::new(0));
        let fires = Arc::new(AtomicU64::new(0));
        let s = store.clone();
        let gate_fires = fires.clone();
        store.set_post_subscribe_gate(Some(Arc::new(move || {
            if gate_fires.fetch_add(1, Ordering::SeqCst) == 0 {
                let _ = s.clear("box"); // fire 1: releases the just-created channel
            } else {
                let _ = s.insert(email("late", "box", "a@x.io")); // fire 2: after the re-subscribe
            }
        })));
        let out = store
            .wait_for("box", &Filter::default(), 1, Duration::from_secs(5))
            .await;
        assert_eq!(
            fires.load(Ordering::SeqCst),
            2,
            "both subscribe sites fired the gate"
        );
        match out {
            WaitOutcome::Found(matches) => assert_eq!(matches[0].id, "late"),
            WaitOutcome::Timeout { .. } => {
                panic!(
                    "the re-subscribe precedes the rescan: the mail inserted after the release must be found"
                )
            }
        }
    }

    /// A waiter on a released channel *sleeps* (decision 0021, AC4). The
    /// gate's fire 1 clears the inbox; fire 2 is a deliberate no-op, so the
    /// wait rides to its deadline and reports the honest `matched: 0`. The
    /// deterministic teeth are the `Closed`-arm delta bound below — one hit
    /// per release, where a reintroduced spin measured 202k–276k in this
    /// window — and the 5 ms co-scheduled ticker is the canary (it advanced
    /// 33 of ~40 ticks under a live spin: tokio's budget yields a spinner).
    /// No wall-clock CPU-percentage assertion anywhere.
    #[tokio::test(flavor = "current_thread")]
    async fn a_waiter_on_a_released_channel_sleeps_until_its_deadline() {
        let store = Arc::new(Store::new(0));
        let ticks = Arc::new(AtomicU64::new(0));
        let fires = Arc::new(AtomicU64::new(0));
        let s = store.clone();
        let gate_fires = fires.clone();
        store.set_post_subscribe_gate(Some(Arc::new(move || {
            if gate_fires.fetch_add(1, Ordering::SeqCst) == 0 {
                let _ = s.clear("box"); // release the channel: the waiter must re-subscribe, then sleep
            }
        })));
        let ticker = {
            let ticks = ticks.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    ticks.fetch_add(1, Ordering::Relaxed);
                }
            })
        };
        let hits_before = CLOSED_PROBE.load(Ordering::Relaxed);
        let out = store
            .wait_for("box", &Filter::default(), 1, Duration::from_millis(200))
            .await;
        ticker.abort();
        // `Timeout { matched: 0 }` in one assert: the honest deadline outcome
        // is the pass path, so an unreachable panic arm would be a line the
        // exact coverage gate can never see executed.
        assert!(
            matches!(out, WaitOutcome::Timeout { matched: 0 }),
            "no mail was ever inserted: the released-channel wait must ride to its deadline"
        );
        let advanced = ticks.load(Ordering::Relaxed);
        assert!(
            advanced >= 10,
            "a spinning waiter starves its single worker: the 5 ms ticker advanced only {advanced} of ~40 times"
        );
        // Deterministic teeth (delta, not absolute — the counter is
        // process-global and the harness runs tests in parallel): a parked
        // waiter answers its one release with one `Closed`-arm hit; a
        // reintroduced spin measured 202k–276k inside this window.
        let hits = CLOSED_PROBE.load(Ordering::Relaxed) - hits_before;
        assert!(
            hits <= 4,
            "a single release must be answered with one Closed arm hit, not {hits}"
        );
    }
}

#[cfg(test)]
mod latest_tests {
    use super::*;

    fn stamped(id: &str, inbox: &str, ms: i64) -> Email {
        let mut e = email(id, inbox, "a@x.io");
        e.received_ms = ms;
        e
    }

    /// `latest` is the maximum `(received_ms, id)` in scope (decision 0044
    /// §4, issue #95), never the deque's tail: insertion order and receive
    /// stamps run opposite here, an equal-stamp tie breaks on the id, a
    /// named inbox never answers another inbox's mail, and an empty or
    /// unknown scope answers nothing. Red before: the route ignored the
    /// scope and answered the global newest.
    #[test]
    fn latest_is_the_scoped_maximum_not_the_tail() {
        let store = Store::new(0);
        assert!(store.latest(None).is_none());
        // Inserted newest-stamp first: the tail is the *oldest* stamp.
        store.insert(stamped("a-2", "a", 300)).unwrap();
        store.insert(stamped("a-1", "a", 100)).unwrap();
        // A tie on the stamp: the larger id wins, whatever arrived first.
        store.insert(stamped("a-3", "a", 300)).unwrap();
        store.insert(stamped("a-0", "a", 300)).unwrap();
        store.insert(stamped("b-9", "b", 900)).unwrap();
        assert_eq!(store.latest(Some("a")).unwrap().id, "a-3");
        assert_eq!(store.latest(Some("b")).unwrap().id, "b-9");
        assert_eq!(store.latest(None).unwrap().id, "b-9");
        assert!(store.latest(Some("ghost")).is_none());
        // The scope's winner is untouched by mail elsewhere.
        store.insert(stamped("c-1", "c", 5_000)).unwrap();
        assert_eq!(store.latest(Some("a")).unwrap().id, "a-3");
        assert_eq!(store.latest(None).unwrap().id, "c-1");
    }
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    fn with_subject(id: &str, subject: Option<&str>) -> Email {
        let mut e = email(id, "box", "a@x.io");
        e.subject = subject.map(|s| s.to_string());
        e
    }

    fn with_received(id: &str, ms: i64) -> Email {
        let mut e = email(id, "box", "a@x.io");
        e.received_ms = ms;
        e
    }

    #[test]
    fn filter_from_requires_a_present_matching_from() {
        let store = Store::new(0);
        let mut e = email("f1", "box", "a@x.io");
        e.from = None;
        let _ = store.insert(e);
        // A from-filter can never match a mail with no from at all.
        let f = Filter {
            from: Some("a@x.io".into()),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 0);

        let mut e = email("f2", "box", "a@x.io");
        e.from = Some(crate::model::EmailAddress {
            name: None,
            address: "sender@x.io".into(),
        });
        let _ = store.insert(e);
        let f = Filter {
            from: Some("sender@x.io".into()),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 1);
        let f = Filter {
            from: Some("other@x.io".into()),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 0);
    }

    /// The `to` leg answers for every recipient the mail was actually
    /// delivered to (issue #55): the envelope's RCPT TO list and the cc
    /// header join the header `to` rows, case-insensitively. A search for a
    /// *delivered* recipient must not answer zero just because no header
    /// row carries the address.
    #[test]
    fn the_to_filter_matches_header_cc_and_envelope_recipients() {
        let store = Store::new(0);
        // RCPT TO:<bcc@x.io> with header `To: a@x.io` — envelope-only.
        let mut bcc = email("b1", "box", "a@x.io");
        bcc.recipients = vec!["bcc@x.io".into()];
        let _ = store.insert(bcc);
        // A cc header row, absent from the `to` list.
        let mut cc = email("c1", "box", "a@x.io");
        cc.cc = vec![crate::model::EmailAddress {
            name: None,
            address: "carbon@x.io".into(),
        }];
        let _ = store.insert(cc);

        let f = |to: &str| Filter {
            to: Some(to.into()),
            ..Default::default()
        };
        // The header `to` leg (pre-existing), the cc leg and the envelope
        // leg each answer their mail; a non-recipient answers none.
        assert_eq!(store.count("box", &f("a@x.io")), 2);
        assert_eq!(store.count("box", &f("carbon@x.io")), 1);
        assert_eq!(store.count("box", &f("bcc@x.io")), 1);
        assert_eq!(store.count("box", &f("nobody@x.io")), 0);
        // Case-insensitive, as the header row always was.
        assert_eq!(store.count("box", &f("BCC@X.IO")), 1);
    }

    #[test]
    fn filter_subject_is_case_insensitive_and_null_safe() {
        let store = Store::new(0);
        let _ = store.insert(with_subject("s1", Some("Quarterly REPORT")));
        let _ = store.insert(with_subject("s2", None));

        let f = Filter {
            subject: Some("quarterly".into()),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 1);
        let f = Filter {
            subject: Some("nope".into()),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 0);
    }

    #[test]
    fn filter_since_ms_is_inclusive() {
        let store = Store::new(0);
        let _ = store.insert(with_received("t1", 100));
        let _ = store.insert(with_received("t2", 200));

        let f = Filter {
            since_ms: Some(150),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 1);
        let f = Filter {
            since_ms: Some(100),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 2);
        let f = Filter {
            since_ms: Some(201),
            ..Default::default()
        };
        assert_eq!(store.count("box", &f), 0);
    }

    #[test]
    fn get_delete_and_clear_edge_cases() {
        let store = Store::new(0);
        assert!(store.get("missing").is_none());
        assert!(!store.delete("missing").unwrap());
        assert_eq!(store.clear("no-such-inbox").unwrap(), 0);

        let _ = store.insert(email("d1", "box", "a@x.io"));
        assert!(store.delete("d1").unwrap());
        assert!(store.get("d1").is_none());
        assert_eq!(store.count("box", &Filter::default()), 0);
    }

    #[test]
    fn inboxes_are_sorted_with_counts() {
        let store = Store::new(0);
        assert!(store.inboxes().is_empty());
        let _ = store.insert(email("1", "zeta", "a@x.io"));
        let _ = store.insert(email("2", "alpha", "a@x.io"));
        let _ = store.insert(email("3", "alpha", "a@x.io"));
        assert_eq!(
            store.inboxes(),
            vec![("alpha".to_string(), 2), ("zeta".to_string(), 1)]
        );
    }

    #[test]
    fn totals_counters_are_readable() {
        let store = Store::new(2);
        assert_eq!(store.emails_inserted(), 0);
        assert_eq!(store.emails_dropped(), 0);
        let _ = store.insert(email("1", "box", "a@x.io"));
        let _ = store.insert(email("2", "box", "a@x.io"));
        let _ = store.insert(email("3", "box", "a@x.io"));
        assert_eq!(store.emails_inserted(), 3);
        assert_eq!(store.emails_dropped(), 1);
    }

    #[tokio::test]
    async fn second_subscriber_on_the_same_inbox_gets_the_live_channel() {
        let store = Store::new(0);
        let mut rx1 = store.subscribe("hot");
        let mut rx2 = store.subscribe("hot");
        let _ = store.insert(email("s1", "hot", "a@x.io"));
        assert_eq!(rx1.try_recv().unwrap().id, "s1");
        assert_eq!(rx2.try_recv().unwrap().id, "s1");
    }
}

// The persistence suite lives in src/store/persist_tests.rs — the harness
// side of decision 0020: the exact gate counts production, not test prose.
#[cfg(test)]
mod persist_tests;

#[cfg(test)]
mod bounded_tests {
    use super::*;

    /// The cap is respected and the newest mails are the ones kept — the
    /// same window `list(..).take(cap)` produces.
    #[test]
    fn list_bounded_respects_the_cap_keeping_the_newest() {
        let store = Store::new(0);
        for i in 0..10 {
            let mut e = email(&format!("b{i}"), "box", "a@x.io");
            e.received_ms = i;
            let _ = store.insert(e);
        }
        let bounded = store.list_bounded("box", &Filter::default(), 3);
        assert_eq!(
            bounded.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["b9", "b8", "b7"]
        );
        let window: Vec<String> = store
            .list("box", &Filter::default())
            .into_iter()
            .take(3)
            .map(|e| e.id.clone())
            .collect();
        assert_eq!(
            bounded.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            window
        );
    }

    /// The filter is applied **before** the bound: the newest `cap`
    /// *matches*, never the newest mails filtered down. The distinguishing
    /// case: a filter whose newest match is not among the newest mails.
    #[test]
    fn list_bounded_filters_before_it_bounds() {
        let store = Store::new(0);
        for i in 0..10 {
            let subject = if i % 2 == 0 { "keep" } else { "skip" };
            let mut e = email(&format!("b{i}"), "box", "a@x.io");
            e.subject = Some(subject.into());
            e.received_ms = i;
            let _ = store.insert(e);
        }
        let filter = Filter {
            subject: Some("keep".into()),
            ..Default::default()
        };
        let bounded = store.list_bounded("box", &filter, 2);
        assert_eq!(
            bounded.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["b8", "b6"],
            "the newest two matches, not the newest mails filtered down"
        );
        let window: Vec<String> = store
            .list("box", &filter)
            .into_iter()
            .take(2)
            .map(|e| e.id.clone())
            .collect();
        assert_eq!(
            bounded.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            window
        );
    }

    #[test]
    fn list_bounded_of_a_missing_inbox_is_empty() {
        let store = Store::new(0);
        assert!(
            store
                .list_bounded("ghost", &Filter::default(), 5)
                .is_empty()
        );
    }

    /// Decision 0010's exactness is allocation-free now: a count equals the
    /// length of the list the same filter produces, for a needle that
    /// matches a subset — and for the default filter over the whole inbox.
    #[test]
    fn count_matches_list_len_for_a_non_trivial_filter() {
        let store = Store::new(0);
        for i in 0..6 {
            let subject = if i % 2 == 0 { "hit" } else { "miss" };
            let mut e = email(&format!("c{i}"), "box", "a@x.io");
            e.subject = Some(subject.into());
            e.received_ms = i;
            let _ = store.insert(e);
        }
        let filter = Filter {
            subject: Some("hit".into()),
            ..Default::default()
        };
        assert_eq!(store.count("box", &filter), 3);
        assert_eq!(
            store.count("box", &filter),
            store.list("box", &filter).len()
        );
        assert_eq!(
            store.count("box", &Filter::default()),
            store.list("box", &Filter::default()).len()
        );
    }

    /// One role's output: the ids an insert role accepted, or the ids a
    /// delete role removed. The clear roles return nothing to assert — their
    /// work shows up in the rest state the probe inspects.
    #[derive(Default)]
    struct ProbeRole {
        accepted: Vec<String>,
        deleted: Vec<String>,
    }

    /// One round: a fresh memory-only store, eight roles released by one
    /// barrier — 3× insert on "A", 1× delete on "A", 1× clear on "A",
    /// 2× insert on "B", 1× clear_all — run to completion and joined. The
    /// delete role sweeps every A id the round can name, so its successes
    /// spread across the whole round and race every drain.
    fn run_probe_round(store: &Arc<Store>, round: usize, per_role: usize) -> ProbeRole {
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let mut accepted = Vec::new();
        let mut deleted = Vec::new();
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            // 3× insert on "A".
            for role in 0..3 {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    let mut ids = Vec::with_capacity(per_role);
                    for j in 0..per_role {
                        let id = format!("a{round}-{role}-{j}");
                        let mail = store
                            .insert(email(&id, "A", "a@x.io"))
                            .expect("memory-only probe store has no commit to fail");
                        ids.push(mail.id.clone());
                    }
                    ProbeRole {
                        accepted: ids,
                        deleted: Vec::new(),
                    }
                }));
            }
            // 2× insert on "B".
            for role in 0..2 {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    let mut ids = Vec::with_capacity(per_role);
                    for j in 0..per_role {
                        let id = format!("b{round}-{role}-{j}");
                        let mail = store
                            .insert(email(&id, "B", "b@x.io"))
                            .expect("memory-only probe store has no commit to fail");
                        ids.push(mail.id.clone());
                    }
                    ProbeRole {
                        accepted: ids,
                        deleted: Vec::new(),
                    }
                }));
            }
            // 1× delete on "A": repeated sweeps over the whole A id space.
            {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    let mut removed = Vec::new();
                    for _ in 0..per_role {
                        for role in 0..3 {
                            for j in 0..per_role {
                                let id = format!("a{round}-{role}-{j}");
                                if store.delete(&id).unwrap_or(false) {
                                    removed.push(id);
                                }
                            }
                        }
                    }
                    ProbeRole {
                        accepted: Vec::new(),
                        deleted: removed,
                    }
                }));
            }
            // 1× clear on "A".
            {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    for _ in 0..per_role {
                        let _ = store.clear("A");
                    }
                    ProbeRole::default()
                }));
            }
            // 1× clear_all.
            {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    for _ in 0..per_role {
                        let _ = store.clear_all();
                    }
                    ProbeRole::default()
                }));
            }
            for handle in handles {
                let role: ProbeRole = handle.join().unwrap_or_default();
                accepted.extend(role.accepted);
                deleted.extend(role.deleted);
            }
        });
        ProbeRole { accepted, deleted }
    }

    /// The rest-state check: after the round's join, the two maps must be
    /// one unit. The count form is the contract's phrasing — `count(inbox)`
    /// equals the number of accepted ids retrievable in that inbox; the
    /// per-id form names both race directions — an index entry with no
    /// deque entry (the ghost) and a deque entry with no index entry (a
    /// lost id). A successfully deleted id is asserted absent: nothing
    /// re-inserts a round's id, so a delete that returned true must stick.
    fn assert_probe_round(
        store: &Arc<Store>,
        round: usize,
        accepted: &[String],
        deleted: &[String],
    ) {
        let mut retrievable: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for id in accepted {
            if let Some(e) = store.get(id) {
                *retrievable.entry(e.inbox.clone()).or_default() += 1;
            }
        }
        for inbox in ["A", "B"] {
            assert_eq!(
                store.count(inbox, &Filter::default()),
                retrievable.get(inbox).copied().unwrap_or(0),
                "round {round}: count({inbox}) disagrees with the retrievable id set"
            );
        }
        let listed: Vec<String> = ["A", "B"]
            .iter()
            .flat_map(|i| {
                store
                    .list(i, &Filter::default())
                    .iter()
                    .map(|e| e.id.clone())
                    .collect::<Vec<_>>()
            })
            .collect();
        for id in accepted {
            let in_index = store.get(id).is_some();
            let in_deque = listed.contains(id);
            assert_eq!(
                in_index, in_deque,
                "round {round}: {id} is split across the maps (index: {in_index}, list: {in_deque})"
            );
        }
        for id in deleted {
            assert!(
                store.get(id).is_none(),
                "round {round}: {id} was deleted but is still retrievable"
            );
        }
    }

    /// Runs `task` on a worker thread under a real deadline: a panic inside
    /// the task is re-raised here with its original payload, and a task
    /// that never finishes fails this call instead of hanging the suite.
    /// Both failure arms are warm — exercised below by
    /// [`a_panicking_round_role_re_raises_here`] and
    /// [`a_wedged_round_fails_instead_of_hanging`] — because a cold arm is
    /// an uncovered line under the exact coverage gate (decision 0020 rule 2).
    fn join_with_deadline<T, F>(task: F, deadline: Duration, stuck: &str) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done_tx.send(std::panic::catch_unwind(std::panic::AssertUnwindSafe(task)));
        });
        match done_rx.recv_timeout(deadline) {
            Ok(Ok(value)) => value,
            Ok(Err(payload)) => std::panic::resume_unwind(payload),
            Err(_) => panic!("{stuck}"),
        }
    }

    /// The deadline machinery's panic arm: a round role that panics
    /// re-raises here with its original payload — never a swallowed
    /// `Err`, which is the shape an `.unwrap()` would mint and the
    /// reason `offload` shares it (the panic-symmetry contract).
    #[test]
    #[should_panic(expected = "the round role panics")]
    fn a_panicking_round_role_re_raises_here() {
        join_with_deadline(
            || panic!("the round role panics"),
            Duration::from_secs(60),
            "a role never finished",
        );
    }

    /// The deadline machinery's stuck arm: a round that never finishes
    /// fails the call in bounded time instead of hanging the suite — the
    /// probe's real-deadline guarantee, pinned at 50 ms.
    #[test]
    #[should_panic(expected = "a wedged role never finished")]
    fn a_wedged_round_fails_instead_of_hanging() {
        join_with_deadline(
            || {
                let (tx, rx) = std::sync::mpsc::channel::<()>();
                std::mem::forget(tx);
                let _ = rx.recv();
            },
            Duration::from_millis(50),
            "a wedged role never finished",
        );
    }

    /// The 8-thread barrier probe (epic #13 contract #2): concurrent
    /// insert/delete/clear/clear_all over one store must leave
    /// `count(inbox)` exactly equal to the set of ids `get(id)` retrieves —
    /// the inbox map and the global ID index mutate as one unit (decision
    /// 0012), never as separate DashMap steps. Fifty rounds, eight roles on
    /// a barrier each, joined per round. The deadline is a real one: a role
    /// that never finishes fails the round instead of hanging the suite,
    /// and a role panic re-raises here with its original payload.
    #[test]
    fn eight_threads_cannot_split_an_inbox_from_its_index() {
        const ROUNDS: usize = 50;
        const PER_ROLE: usize = 25;
        for round in 0..ROUNDS {
            let store = Arc::new(Store::new(0));
            let outcome = join_with_deadline(
                {
                    let store = store.clone();
                    move || run_probe_round(&store, round, PER_ROLE)
                },
                Duration::from_secs(60),
                &format!("probe round {round} stuck: a role never finished"),
            );
            assert_probe_round(&store, round, &outcome.accepted, &outcome.deleted);
        }
    }

    /// One temp data-file path per boundary round, inside its own scratch
    /// directory: parallel rounds (and the harness's own files) never share
    /// one, and the returned guard removes the file with every sidecar
    /// (-wal, -shm, the lock) on any exit — a failing round leaves nothing
    /// in the temporary root (issue #143).
    fn boundary_db_path(round: usize) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::TempDir::with_prefix(format!("swarmail-boundary-{round}-")).unwrap();
        let path = dir.path().join("mail.db");
        (dir, path)
    }

    /// One boundary round: a fresh persisted store at cap 1 on a real data
    /// file, sixteen barrier-released roles — 6× insert on "A", 6× insert on
    /// "B", 1× delete on "A", 1× delete on "B", 1× clear on "A", 1×
    /// clear_all — run to completion and joined. Cap 1 makes *every* insert
    /// prune, so the insert→commit seam the two orders disagree about is
    /// crossed twelve times per instant; no storage failure is injected
    /// (finding 1's repro conditions, verbatim).
    fn run_boundary_round(store: &Arc<Store>, round: usize, per_role: usize) {
        let barrier = Arc::new(std::sync::Barrier::new(16));
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            // 12× insert: roles 0-5 on "A", roles 6-11 on "B".
            for role in 0..12 {
                let (inbox, prefix) = if role < 6 { ("A", "a") } else { ("B", "b") };
                let role = role % 6;
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    for j in 0..per_role {
                        let id = format!("{prefix}{round}-{role}-{j}");
                        store
                            .insert(email(&id, inbox, "a@x.io"))
                            .expect("the boundary probe injects no storage failures");
                    }
                }));
            }
            // 1× delete on "A", 1× delete on "B": repeated sweeps over the
            // whole id space the round can name.
            for (prefix, roles) in [("a", 6), ("b", 6)] {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    for j in 0..per_role {
                        for role in 0..roles {
                            store
                                .delete(&format!("{prefix}{round}-{role}-{j}"))
                                .expect("the boundary probe injects no storage failures");
                        }
                    }
                }));
            }
            // 1× clear on "A".
            {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    for _ in 0..per_role {
                        store
                            .clear("A")
                            .expect("the boundary probe injects no storage failures");
                    }
                }));
            }
            // 1× clear_all.
            {
                let barrier = barrier.clone();
                let store = store.clone();
                handles.push(s.spawn(move || {
                    barrier.wait();
                    for _ in 0..per_role {
                        store
                            .clear_all()
                            .expect("the boundary probe injects no storage failures");
                    }
                }));
            }
            for handle in handles {
                handle
                    .join()
                    .expect("boundary round role panicked inside the scope");
            }
        });
    }

    /// The boundary round's rest-state check: after the join, the live store
    /// and a fresh reopen of the same file must be one state — the same
    /// per-inbox id lists (sets *and* order: the commit order under the
    /// boundary is the push order), the same counts, the same counters —
    /// and neither inbox may exceed its cap after the restart. On the
    /// two-orders tree a later insert prunes an earlier id before that
    /// insert's commit lands, so the earlier transaction persists an
    /// already-evicted message and the restart copy names a mail live never
    /// held (finding 1). The live store is the data file's one owner
    /// (decision 0033), so its state is read first and the store dropped
    /// before the restart opens the file — the order a real restart has.
    fn assert_boundary_round(store: Arc<Store>, round: usize, path: &std::path::Path) {
        let live: Vec<(Vec<String>, usize)> = ["A", "B"]
            .into_iter()
            .map(|inbox| {
                let ids: Vec<String> = store
                    .list(inbox, &Filter::default())
                    .iter()
                    .map(|e| e.id.clone())
                    .collect();
                (ids, store.count(inbox, &Filter::default()))
            })
            .collect();
        let (inserted, dropped) = (store.emails_inserted(), store.emails_dropped());
        drop(store);
        let restarted = Store::open(1, path).expect("the same data file reopens");
        for (inbox, (live, count)) in ["A", "B"].into_iter().zip(live) {
            let disk: Vec<String> = restarted
                .list(inbox, &Filter::default())
                .iter()
                .map(|e| e.id.clone())
                .collect();
            assert_eq!(
                live, disk,
                "round {round}: live {inbox} and the restart copy are two states"
            );
            assert_eq!(
                count,
                restarted.count(inbox, &Filter::default()),
                "round {round}: count({inbox}) disagrees with the restart copy"
            );
            for id in &live {
                assert!(
                    restarted.get(id).is_some(),
                    "round {round}: live mail {id} has no restart row"
                );
            }
            assert!(
                disk.len() <= 1,
                "round {round}: {inbox} exceeds its cap after the restart"
            );
        }
        assert_eq!(
            inserted,
            restarted.emails_inserted(),
            "round {round}: the inserted counter disagrees with the restart copy"
        );
        assert_eq!(
            dropped,
            restarted.emails_dropped(),
            "round {round}: the dropped counter disagrees with the restart copy"
        );
    }

    /// The 16-thread boundary probe (issue #31 finding 1, decision 0014):
    /// memory and disk are one mutation order, so after sixteen
    /// barrier-released roles hammer a persisted store at cap 1, live
    /// `list`/`get`/`count` and a fresh reopen of the same file agree —
    /// and no inbox exceeds the cap after the restart. Red on the
    /// two-orders tree (the finding's repro, three runs), green under the
    /// boundary on every schedule: the mutation lock now spans plan →
    /// commit → apply, so push order *is* commit order and a commit can
    /// never outlive its own eviction.
    #[test]
    fn sixteen_threads_cannot_leave_memory_and_disk_in_two_orders() {
        const ROUNDS: usize = 6;
        const PER_ROLE: usize = 15;
        for round in 0..ROUNDS {
            let (_dir, path) = boundary_db_path(round);
            {
                let store = Arc::new(Store::open(1, &path).unwrap());
                join_with_deadline(
                    {
                        let store = store.clone();
                        move || run_boundary_round(&store, round, PER_ROLE)
                    },
                    Duration::from_secs(120),
                    &format!("boundary round {round} stuck: a role never finished"),
                );
                // A final synchronous insert per inbox pins a non-empty rest
                // state: the clear_all role may lawfully land last and empty
                // the store, and a vacuous check (0 == 0 on both sides)
                // proves nothing about the order the boundary serialized —
                // the coverage gate refuses exactly that schedule.
                store
                    .insert(email(&format!("a{round}-final"), "A", "a@x.io"))
                    .expect("the boundary probe injects no storage failures");
                store
                    .insert(email(&format!("b{round}-final"), "B", "b@x.io"))
                    .expect("the boundary probe injects no storage failures");
                assert_boundary_round(store, round, &path);
            }
        }
    }
}
