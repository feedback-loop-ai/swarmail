//! The in-memory, per-inbox email store.
//!
//! Design goals: lossless under bursts (no async drop points), bounded memory
//! (per-inbox ring pruning of *oldest* mail only), and cheap waiting
//! (broadcast watchers for `await` endpoints).

use crate::model::Email;
use crate::persist;
use dashmap::DashMap;
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
/// the wait loop already performs (decision 0010).
const WATCHER_CAP: usize = 16;

/// Test-only count of `Closed`-arm hits in `wait_for` — the deterministic
/// no-spin instrument: one hit per release, hundreds of thousands under a
/// reintroduced spin. Compiled out of the shipping binary like the seam.
#[cfg(test)]
static CLOSED_PROBE: AtomicU64 = AtomicU64::new(0);

/// Query filter for list/count/await endpoints.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Substring match against any recipient (to/cc) address, case-insensitive.
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
                .chain(email.cc.iter())
                .any(|a| a.address.to_lowercase().contains(&to));
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
    inboxes: &DashMap<String, Vec<Arc<Email>>>,
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
            list
        }
        None => Vec::new(),
    }
}

pub struct Store {
    inboxes: DashMap<String, Vec<Arc<Email>>>,
    /// id -> email, a global index for O(1) lookup.
    index: DashMap<String, Arc<Email>>,
    /// Per-inbox wake channels: created by the first `subscribe` to an inbox
    /// (atomically, under the shard lock) and released when the inbox's mail
    /// is gone — by `drain_inbox` (shared by `clear`/`clear_all`), by a
    /// `delete` that empties the inbox's list, and by the `clear_all` sweep
    /// (decision 0010). `insert` never creates one: a notify with no entry
    /// is a no-op.
    watchers: DashMap<String, broadcast::Sender<Arc<Email>>>,
    /// Process-wide channel: every accepted email, for webhooks.
    global_tx: broadcast::Sender<Arc<Email>>,
    /// Maximum emails retained per inbox (oldest pruned). 0 = unlimited.
    max_per_inbox: usize,
    totals: Totals,
    /// Write-through SQLite mirror when running with a data file.
    persist: Option<persist::Persist>,
    /// Test-only seam for the clear/insert persistence race (review finding
    /// in run sqlite-persistence-data-file-swa-dc3a9a2a): when set, the
    /// closure runs *inside* the drain→mirror window of `clear`/`clear_all`,
    /// so a test can interleave a live insert into that exact window
    /// deterministically instead of winning a thread-timing lottery.
    /// Compiled only under test; never set in production.
    #[cfg(test)]
    pre_mirror_gate: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
    /// Test-only seam for the watcher release/wait interplay (decision 0010):
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
            max_per_inbox,
            totals: Totals::default(),
            persist: None,
            #[cfg(test)]
            pre_mirror_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            post_subscribe_gate: std::sync::Mutex::new(None),
        }
    }

    /// Open (or create) a persistent store backed by `data_file`: the full
    /// state is restored before the server accepts a single connection.
    pub fn open(max_per_inbox: usize, data_file: &std::path::Path) -> rusqlite::Result<Self> {
        let persist = persist::Persist::open(data_file)?;
        let (emails, inserted, dropped) = persist.load()?;
        let mut store = Self::new(max_per_inbox);
        // Restore in insertion order without going through `insert`: the rows
        // are already committed, and restored mail must not re-fire watchers
        // or webhooks on every restart.
        for email in emails {
            let email = Arc::new(email);
            store
                .inboxes
                .entry(email.inbox.clone())
                .or_default()
                .push(email.clone());
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

    /// Mirror a mutation to the data file, if any. Persistence failures are
    /// logged and swallowed: the memory store stays authoritative (the mail
    /// is queryable — decision 0001 still holds), the restart copy degrades.
    fn record(
        &self,
        what: &'static str,
        write: impl FnOnce(&persist::Persist) -> rusqlite::Result<()>,
    ) {
        if let Some(persist) = &self.persist
            && let Err(e) = write(persist)
        {
            tracing::error!(error = %e, what, "persistence write failed; memory store remains authoritative");
        }
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
    /// cloned out before the call so the closure may re-enter the store.
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
    /// deliberate follow-up recorded in decision 0010.
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

    /// Subscribe to every accepted email (webhook dispatcher).
    pub fn subscribe_all(&self) -> broadcast::Receiver<Arc<Email>> {
        self.global_tx.subscribe()
    }

    /// Insert an email. Lossless: this is synchronous — once SMTP accepted the
    /// message, it is queryable (and on disk, when persisting). Prunes oldest
    /// beyond the per-inbox cap.
    pub fn insert(&self, email: Email) -> Arc<Email> {
        let email = Arc::new(email);
        let inbox = email.inbox.clone();
        let mut pruned: Vec<Arc<Email>> = Vec::new();
        {
            let mut list = self.inboxes.entry(inbox.clone()).or_default();
            list.push(email.clone());
            if self.max_per_inbox > 0 && list.len() > self.max_per_inbox {
                let overflow = list.len() - self.max_per_inbox;
                pruned = list.drain(..overflow).collect();
                for p in &pruned {
                    self.index.remove(&p.id);
                }
                self.totals
                    .emails_dropped
                    .fetch_add(overflow as u64, Ordering::Relaxed);
            }
        }
        self.index.insert(email.id.clone(), email.clone());
        self.totals.emails_inserted.fetch_add(1, Ordering::Relaxed);
        // Persist before any fan-out and before the caller's 250: when SMTP
        // accepts, the mail is queryable AND committed (decision 0001).
        self.record("insert", |persist| {
            let pruned_ids: Vec<&str> = pruned.iter().map(|e| e.id.as_str()).collect();
            persist.record_insert(
                &email,
                &pruned_ids,
                self.totals.emails_inserted(),
                self.totals.emails_dropped(),
            )
        });
        // Notify watchers; a full channel means a slow await reader — the email
        // is still safely stored, the watcher re-polls the store. A missing
        // entry means no subscribers yet, or the channel was released with
        // the inbox's mail (decision 0010): the waiter re-subscribes before
        // its next scan, so the store-is-truth rescan still finds this mail.
        if let Some(sender) = self.watchers.get(&inbox) {
            let _ = sender.send(email.clone());
        }
        let _ = self.global_tx.send(email.clone());
        email
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

    /// Newest-first listing across ALL inboxes matching `filter`.
    ///
    /// The MailHog/Mailpit compat shims expose a single global mailbox, so
    /// they read the whole store through this (and scope to one inbox by
    /// calling `list` when the request names one). Insertion order is only
    /// per-inbox, so the merged view is ordered by receive time, with the
    /// UUIDv7 id as the tiebreak (it sorts chronologically).
    pub fn list_all(&self, filter: &Filter) -> Vec<Arc<Email>> {
        let mut all: Vec<Arc<Email>> = Vec::new();
        for entry in self.inboxes.iter() {
            all.extend(entry.value().iter().filter(|e| filter.matches(e)).cloned());
        }
        all.sort_by(|a, b| {
            b.received_ms
                .cmp(&a.received_ms)
                .then_with(|| b.id.cmp(&a.id))
        });
        all
    }

    pub fn count(&self, inbox: &str, filter: &Filter) -> usize {
        self.list(inbox, filter).len()
    }

    pub fn delete(&self, id: &str) -> bool {
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
            // An emptied inbox's mail is gone, so its watcher channel is
            // released with it (decision 0010). Disjoint maps: the inboxes
            // guard is held, the watchers map is a separate shard set.
            if list.is_empty() {
                self.watchers.remove(&email.inbox);
            }
        }
        self.record("delete", |persist| persist.record_delete(id));
        removed.is_some()
    }

    /// Remove all emails from one inbox; returns how many were removed.
    pub fn clear(&self, inbox: &str) -> usize {
        let drained = drain_inbox(&self.inboxes, &self.index, &self.watchers, inbox);
        #[cfg(test)]
        self.run_pre_mirror_gate();
        // The mirror deletes exactly the drained ids, never `WHERE inbox =
        // ?`. An insert landing between the drain and this mirror is answered
        // 250 and queryable in memory (decision 0001); an inbox-wide DELETE
        // would erase its row and the mail would vanish on restart — the
        // clear/insert persistence race (review finding in run
        // sqlite-persistence-data-file-swa-dc3a9a2a). A post-drain insert's
        // id is by construction not among the drained ids, so mirroring
        // per-id deletes can never erase it, whichever way the two
        // operations interleave; the disk outcome for everything the clear
        // actually drained is unchanged.
        let ids: Vec<&str> = drained.iter().map(|e| e.id.as_str()).collect();
        self.record("clear_inbox", |persist| persist.record_deletes(&ids));
        drained.len()
    }

    /// Remove every email from every inbox.
    pub fn clear_all(&self) -> usize {
        let mut drained: Vec<Arc<Email>> = Vec::new();
        let inboxes: Vec<String> = self.inboxes.iter().map(|e| e.key().clone()).collect();
        for inbox in inboxes {
            drained.extend(drain_inbox(
                &self.inboxes,
                &self.index,
                &self.watchers,
                &inbox,
            ));
        }
        // The sweep also releases channels of awaited-but-never-mailed
        // inboxes — the population issue #14's probe creates, which leaves no
        // entries for the per-inbox drain above to iterate (decision 0010).
        // A subscriber racing the sweep sees `Closed` and takes the
        // re-subscribe arm — bounded at one extra rescan per waiter.
        self.watchers.clear();
        #[cfg(test)]
        self.run_pre_mirror_gate();
        // Per-id mirror for the same reason as `clear` (the race finding):
        // the racing insert is not among the drained ids, so it keeps its
        // row; the one-transaction wipe would erase it.
        let ids: Vec<&str> = drained.iter().map(|e| e.id.as_str()).collect();
        self.record("clear_all", |persist| persist.record_deletes(&ids));
        drained.len()
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

    /// Subscribe to new-mail notifications for an inbox (lazy per-inbox
    /// channel).
    ///
    /// One atomic path: the `entry()` holds the shard write lock across
    /// channel creation, so concurrent first subscribers serialize on one
    /// boundary and every receiver attaches to the same live channel — the
    /// get-then-insert window is deleted, not fenced (decision 0010). The
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
            // schedule in which an iteration does neither (decision 0010).
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

    #[test]
    fn insert_prunes_oldest_beyond_cap() {
        let store = Store::new(2);
        store.insert(email("1", "default", "a@x.io"));
        store.insert(email("2", "default", "a@x.io"));
        store.insert(email("3", "default", "a@x.io"));
        assert_eq!(store.count("default", &Filter::default()), 2);
        assert!(store.get("1").is_none());
        assert!(store.get("2").is_some());
        assert!(store.get("3").is_some());
    }

    #[test]
    fn filter_by_recipient() {
        let store = Store::new(0);
        store.insert(email("1", "default", "a@x.io"));
        store.insert(email("2", "default", "b@x.io"));
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
        store.insert(old);
        store.insert(new);

        let all = store.list_all(&Filter::default());
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "new");
        assert_eq!(all[1].id, "old");

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
        store.insert(email("1", "test-a", "a@x.io"));
        store.insert(email("2", "test-b", "b@x.io"));
        assert_eq!(store.count("test-a", &Filter::default()), 1);
        assert_eq!(store.count("test-b", &Filter::default()), 1);
        store.clear("test-a");
        assert_eq!(store.count("test-a", &Filter::default()), 0);
        assert_eq!(store.count("test-b", &Filter::default()), 1);
    }

    /// Eight threads race to be the first subscriber of one fresh inbox
    /// (decision 0010, AC1): registration is atomic, so every receiver
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
        store.insert(email("race-1", "race", "a@x.io"));
        for (i, rx) in receivers.iter_mut().enumerate() {
            assert_eq!(
                rx.try_recv().map(|e| e.id.clone()).unwrap(),
                "race-1",
                "receiver {i} attached to a displaced channel"
            );
        }
    }

    /// `clear` is where an inbox dies, so it is where its watcher channel
    /// dies (decision 0010, AC2-i): the receiver held across the clear
    /// observes the release. An empty clear still releases — the channel is
    /// not immortal — and the held receiver is safe: its waiter answers
    /// `Closed` with a re-subscribe before its next scan.
    #[test]
    fn a_cleared_inbox_releases_its_watcher_channel() {
        let store = Store::new(0);
        let mut rx = store.subscribe("box");
        assert_eq!(store.clear("box"), 0);
        assert!(
            matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Closed)),
            "the channel held across the clear must observe the release"
        );
    }

    /// A `delete` that empties an inbox's list releases the channel
    /// (decision 0010, AC2-ii); a delete that leaves mail behind must keep
    /// it live — a pushed insert still reaches the held receiver.
    #[test]
    fn a_delete_that_empties_an_inbox_releases_its_watcher_channel() {
        let store = Store::new(0);
        let mut rx = store.subscribe("box");
        store.insert(email("d1", "box", "a@x.io"));
        store.insert(email("d2", "box", "a@x.io"));
        // Drain the two buffered pushes so later asserts read only pushes
        // that land after the deletes.
        assert_eq!(rx.try_recv().map(|e| e.id.clone()).unwrap(), "d1");
        assert_eq!(rx.try_recv().map(|e| e.id.clone()).unwrap(), "d2");
        store.delete("d1");
        store.insert(email("d3", "box", "a@x.io"));
        assert_eq!(
            rx.try_recv().map(|e| e.id.clone()).unwrap(),
            "d3",
            "a non-emptying delete keeps the channel live"
        );
        store.delete("d2");
        store.delete("d3"); // the list is now empty: the channel is released
        assert!(
            matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Closed)),
            "a delete that empties the inbox must release the channel"
        );
    }

    /// `clear_all` sweeps every watcher channel, including the
    /// awaited-but-never-mailed population no per-inbox drain can reach —
    /// issue #14's 5,000-fresh-inbox probe (decision 0010, AC2-iii): the
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
        store.clear_all();
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
}

#[cfg(test)]
mod wait_tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_finds_what_is_already_there() {
        let store = Store::new(0);
        store.insert(email("w1", "box", "a@x.io"));
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
            s.insert(email("w2", "box", "a@x.io"));
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
        store.insert(email("w3", "box", "a@x.io"));
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
        // the rescan, not by ring headroom (decision 0010, AC5).
        tokio::time::sleep(Duration::from_millis(100)).await;
        for i in 0..4200u32 {
            let mut e = email(&format!("lag{i}"), "box", "a@x.io");
            e.id = format!("lag{i}");
            s.insert(e);
        }
        match handle.await.unwrap() {
            WaitOutcome::Found(matches) => assert_eq!(matches.len(), 3),
            WaitOutcome::Timeout { .. } => panic!("the store had the mails"),
        }
    }

    /// A released channel cannot strand a waiter (decision 0010, AC3). The
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
                s.clear("box"); // fire 1: releases the just-created channel
            } else {
                s.insert(email("late", "box", "a@x.io")); // fire 2: after the re-subscribe
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

    /// A waiter on a released channel *sleeps* (decision 0010, AC4). The
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
                s.clear("box"); // release the channel: the waiter must re-subscribe, then sleep
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
        store.insert(e);
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
        store.insert(e);
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

    #[test]
    fn filter_subject_is_case_insensitive_and_null_safe() {
        let store = Store::new(0);
        store.insert(with_subject("s1", Some("Quarterly REPORT")));
        store.insert(with_subject("s2", None));

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
        store.insert(with_received("t1", 100));
        store.insert(with_received("t2", 200));

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
        assert!(!store.delete("missing"));
        assert_eq!(store.clear("no-such-inbox"), 0);

        store.insert(email("d1", "box", "a@x.io"));
        assert!(store.delete("d1"));
        assert!(store.get("d1").is_none());
        assert_eq!(store.count("box", &Filter::default()), 0);
    }

    #[test]
    fn inboxes_are_sorted_with_counts() {
        let store = Store::new(0);
        assert!(store.inboxes().is_empty());
        store.insert(email("1", "zeta", "a@x.io"));
        store.insert(email("2", "alpha", "a@x.io"));
        store.insert(email("3", "alpha", "a@x.io"));
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
        store.insert(email("1", "box", "a@x.io"));
        store.insert(email("2", "box", "a@x.io"));
        store.insert(email("3", "box", "a@x.io"));
        assert_eq!(store.emails_inserted(), 3);
        assert_eq!(store.emails_dropped(), 1);
    }

    #[tokio::test]
    async fn second_subscriber_on_the_same_inbox_gets_the_live_channel() {
        let store = Store::new(0);
        let mut rx1 = store.subscribe("hot");
        let mut rx2 = store.subscribe("hot");
        store.insert(email("s1", "hot", "a@x.io"));
        assert_eq!(rx1.try_recv().unwrap().id, "s1");
        assert_eq!(rx2.try_recv().unwrap().id, "s1");
    }
}

#[cfg(test)]
mod persist_tests;
