//! The in-memory, per-inbox email store.
//!
//! Design goals: lossless under bursts (no async drop points), bounded memory
//! (per-inbox ring pruning of *oldest* mail only), and cheap waiting
//! (broadcast watchers for `await` endpoints).

use crate::model::Email;
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
        }
    }

    /// Subscribe to every accepted email (webhook dispatcher).
    pub fn subscribe_all(&self) -> broadcast::Receiver<Arc<Email>> {
        self.global_tx.subscribe()
    }

    /// Insert an email. Lossless: this is synchronous — once SMTP accepted the
    /// message, it is queryable. Prunes oldest beyond the per-inbox cap.
    pub fn insert(&self, email: Email) -> Arc<Email> {
        let email = Arc::new(email);
        let inbox = email.inbox.clone();
        {
            let mut list = self.inboxes.entry(inbox.clone()).or_default();
            list.push(email.clone());
            if self.max_per_inbox > 0 && list.len() > self.max_per_inbox {
                let overflow = list.len() - self.max_per_inbox;
                let pruned: Vec<Arc<Email>> = list.drain(..overflow).collect();
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
        removed.is_some()
    }

    /// Remove all emails from one inbox; returns how many were removed.
    pub fn clear(&self, inbox: &str) -> usize {
        let mut removed = 0;
        if let Some((_, mut list)) = self.inboxes.remove(inbox) {
            removed = list.len();
            for e in list.drain(..) {
                self.index.remove(&e.id);
            }
        }
        removed
    }

    /// Remove every email from every inbox.
    pub fn clear_all(&self) -> usize {
        let mut removed = 0;
        let inboxes: Vec<String> = self.inboxes.iter().map(|e| e.key().clone()).collect();
        for inbox in inboxes {
            removed += self.clear(&inbox);
        }
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
mod tests {
    use super::*;

    fn email(id: &str, inbox: &str, to: &str) -> Email {
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
            raw: vec![],
        }
    }

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
