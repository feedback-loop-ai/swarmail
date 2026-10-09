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

/// Drop one inbox out of the maps; shared by `clear` and `clear_all` so both
/// persist exactly one mirrored delete.
fn drain_inbox(
    inboxes: &DashMap<String, Vec<Arc<Email>>>,
    index: &DashMap<String, Arc<Email>>,
    inbox: &str,
) -> usize {
    let mut removed = 0;
    if let Some((_, mut list)) = inboxes.remove(inbox) {
        removed = list.len();
        for e in list.drain(..) {
            index.remove(&e.id);
        }
    }
    removed
}

pub struct Store {
    inboxes: DashMap<String, Vec<Arc<Email>>>,
    /// id -> email, a global index for O(1) lookup.
    index: DashMap<String, Arc<Email>>,
    watchers: DashMap<String, broadcast::Sender<Arc<Email>>>,
    /// Process-wide channel: every accepted email, for webhooks.
    global_tx: broadcast::Sender<Arc<Email>>,
    /// Maximum emails retained per inbox (oldest pruned). 0 = unlimited.
    max_per_inbox: usize,
    totals: Totals,
    /// Write-through SQLite mirror when running with a data file.
    persist: Option<persist::Persist>,
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
        // is still safely stored, the watcher re-polls the store.
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

    pub fn count(&self, inbox: &str, filter: &Filter) -> usize {
        self.list(inbox, filter).len()
    }

    pub fn delete(&self, id: &str) -> bool {
        let removed = self.index.remove(id);
        if let Some((_, email)) = &removed
            && let Some(mut list) = self.inboxes.get_mut(&email.inbox)
        {
            list.retain(|e| e.id != id);
        }
        self.record("delete", |persist| persist.record_delete(id));
        removed.is_some()
    }

    /// Remove all emails from one inbox; returns how many were removed.
    pub fn clear(&self, inbox: &str) -> usize {
        let removed = drain_inbox(&self.inboxes, &self.index, inbox);
        self.record("clear_inbox", |persist| persist.record_clear_inbox(inbox));
        removed
    }

    /// Remove every email from every inbox.
    pub fn clear_all(&self) -> usize {
        let mut removed = 0;
        let inboxes: Vec<String> = self.inboxes.iter().map(|e| e.key().clone()).collect();
        for inbox in inboxes {
            removed += drain_inbox(&self.inboxes, &self.index, &inbox);
        }
        self.record("clear_all", |persist| persist.record_clear_all());
        removed
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

    /// Subscribe to new-mail notifications for an inbox (lazy per-inbox channel).
    pub fn subscribe(&self, inbox: &str) -> broadcast::Receiver<Arc<Email>> {
        match self.watchers.get(inbox) {
            Some(sender) => sender.subscribe(),
            None => {
                let (sender, receiver) = broadcast::channel(4096);
                self.watchers.insert(inbox.to_string(), sender.clone());
                receiver
            }
        }
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
    /// cannot be missed (decision 0002). The store remains the source of
    /// truth: every wakeup re-scans, so slow or lagging watchers are safe —
    /// a full channel or a closed one both fall through to the deadline
    /// re-check.
    pub async fn wait_for(
        &self,
        inbox: &str,
        filter: &Filter,
        want: usize,
        timeout: Duration,
    ) -> WaitOutcome {
        let mut rx = self.subscribe(inbox);
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
            // deadline; the loop re-scans either way.
            let _ = tokio::time::timeout(remaining, rx.recv()).await;
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
        // overflow the 4096-slot channel.
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
        let mut rx2 = store.subscribe("hot"); // the Some(sender) branch
        store.insert(email("s1", "hot", "a@x.io"));
        assert_eq!(rx1.try_recv().unwrap().id, "s1");
        assert_eq!(rx2.try_recv().unwrap().id, "s1");
    }
}

#[cfg(test)]
mod persist_tests {
    use super::*;

    fn db_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "swarmail-store-{tag}-{}-{nanos}.db",
            std::process::id()
        ))
    }

    #[test]
    fn a_reopened_store_restores_mail_order_and_counters() {
        let path = db_path("restore");
        {
            let store = Store::open(3, &path).unwrap();
            for id in ["1", "2", "3"] {
                let mut e = email(id, "box", "a@x.io");
                e.raw = vec![b'r', id.as_bytes()[0]];
                store.insert(e);
            }
            assert!(store.delete("2")); // a delete must be mirrored too
        }
        let store = Store::open(3, &path).unwrap();
        // Newest-first list of what is left, in restored insertion order.
        let listed = store.list("box", &Filter::default());
        assert_eq!(
            listed.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["3", "1"]
        );
        // Raw bytes survive the round trip.
        assert_eq!(store.get("1").unwrap().raw, b"r1".to_vec());
        // Lifetime counters are restored, not reset.
        assert_eq!(store.emails_inserted(), 3);
        assert_eq!(store.emails_dropped(), 0);
        assert_eq!(store.inboxes(), vec![("box".to_string(), 2)]);
    }

    #[test]
    fn cap_pruning_is_mirrored_to_the_data_file() {
        let path = db_path("cap");
        {
            let store = Store::open(2, &path).unwrap();
            for id in ["1", "2", "3"] {
                store.insert(email(id, "box", "a@x.io"));
            }
        }
        let store = Store::open(2, &path).unwrap();
        // The pruned-oldest row is gone from the restart copy as well.
        assert_eq!(
            store
                .list("box", &Filter::default())
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["3", "2"]
        );
        assert!(store.get("1").is_none());
        assert_eq!(store.emails_inserted(), 3);
        assert_eq!(store.emails_dropped(), 1);
    }

    #[test]
    fn clear_and_clear_all_are_mirrored_to_the_data_file() {
        let path = db_path("clear");
        {
            let store = Store::open(0, &path).unwrap();
            store.insert(email("c1", "box-a", "a@x.io"));
            store.insert(email("c2", "box-b", "a@x.io"));
            assert_eq!(store.clear("box-a"), 1);
        }
        {
            let store = Store::open(0, &path).unwrap();
            assert_eq!(store.count("box-a", &Filter::default()), 0);
            assert_eq!(store.count("box-b", &Filter::default()), 1);
            assert_eq!(store.clear_all(), 1);
        }
        let store = Store::open(0, &path).unwrap();
        assert_eq!(store.count("box-b", &Filter::default()), 0);
        assert_eq!(store.emails_inserted(), 2); // counters outlive clears
    }

    #[test]
    fn a_failing_data_file_never_blocks_the_store() {
        let store = Store::open(0, &db_path("failing")).unwrap();
        store.persist.as_ref().unwrap().fail_writes();
        // Insert still returns the mail and the memory store stays truthful.
        let stored = store.insert(email("f1", "box", "a@x.io"));
        assert_eq!(stored.id, "f1");
        assert_eq!(store.count("box", &Filter::default()), 1);
        assert!(store.delete("f1"));
        assert_eq!(store.clear("box"), 0);
        store.insert(email("f2", "box", "a@x.io"));
        assert_eq!(store.clear_all(), 1);
    }

    #[test]
    fn open_refuses_an_unusable_data_file() {
        assert!(Store::open(0, &db_path("nope").join("missing-dir").join("x.db")).is_err());
    }
}
