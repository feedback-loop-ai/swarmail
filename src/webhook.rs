//! Outbound webhooks: queued, retried, never blocking the SMTP path.
//!
//! Delivery speaks real HTTP through reqwest with rustls: `http://` targets
//! stay plain TCP, `https://` targets are TLS with certificate verification
//! on — the rustls webpki root store, plus any extra root a target brings in
//! `ca_pem` (the self-signed test-server case). No native-tls anywhere in
//! the tree. The wire header names are lowercased by hyper; HTTP/1.1 names
//! are case-insensitive, so the secret arrives under `X-Swarmail-Secret`
//! regardless of case. The per-root clients are cached, but under a fixed
//! bound with least-recently-used eviction — a long-lived server pointed at
//! many distinct roots must not grow the cache without limit.
//!
//! Delivery is best-effort with observable loss: at most
//! [`WEBHOOK_DELIVERY_CAP`] deliveries are in flight, every death a
//! notification can die has a row in the per-instance ledger
//! ([`Webhooks::snapshot`]), and nothing is ever durable — there is no
//! outbox. Every bound is `try`-shaped (drop-and-count, never
//! wait-and-delay), so the fan-out can never push back on the accept path.

use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

/// One delivery attempt must not hang: the whole round trip — connect
/// (TLS handshake included), send and status — is bounded by this.
const POST_TIMEOUT: Duration = Duration::from_secs(5);

/// The client for targets that trust the built-in root store, built once.
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// How many distinct root PEMs the per-PEM client cache may hold. A fixed
/// small bound: a long-lived server pointed at many distinct CAs (or, if a
/// config surface lets a caller choose PEMs, at attacker-chosen ones) must
/// not grow the cache without limit. 32 is far above any real deployment's
/// root count and small enough that the linear recency scan below is free.
/// Eviction is least-recently-used; the evicted root's next delivery simply
/// rebuilds its client. Surfaced on `/metrics` as
/// `swarmail_webhook_ca_cache_*`.
pub const CA_CLIENT_CAP: usize = 32;

/// The maximum number of webhook deliveries in flight at once. The bound is
/// `try`-shaped: a fan-out that cannot fit is dropped and counted in the
/// `dropped` ledger row, never waited for — the accept path is downstream of
/// the commit and must never be delayed by the delivery side. A stalled
/// target holds its slot for at most the retry budget (four 5 s attempt
/// timeouts plus the 100/400/1600 ms backoff — 22.1 s expected worst case)
/// before the budget is named `exhausted`. Surfaced on `/metrics`.
pub const WEBHOOK_DELIVERY_CAP: usize = 32;

/// The bounded per-PEM client cache: clients keyed by their root PEM, plus
/// those keys in recency order (front = least recently used) so eviction is
/// deterministic. One mutex guards both, so the map and the order can never
/// drift apart.
#[derive(Default)]
struct CaCache {
    clients: HashMap<String, reqwest::Client>,
    lru: VecDeque<String>,
}

/// Clients pinned to an extra root CA, keyed by that root's PEM. A `ca_pem`
/// target gets its own client so its root never leaks into other targets.
static CA_CLIENTS: OnceLock<Mutex<CaCache>> = OnceLock::new();

fn ca_clients() -> &'static Mutex<CaCache> {
    CA_CLIENTS.get_or_init(Mutex::default)
}

/// Cache observability, production-honest: cumulative build and eviction
/// counters since process start, exposed through [`ca_cache_stats`] and
/// `/metrics`, so the bound is checkable from outside the process rather
/// than only from tests. Relaxed ordering suffices: the counters are
/// independent observations, nothing is ordered behind them.
static CA_CACHE_BUILDS: AtomicU64 = AtomicU64::new(0);
static CA_CACHE_EVICTIONS: AtomicU64 = AtomicU64::new(0);

/// A snapshot of the per-PEM client cache: its current size and the
/// cumulative counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaCacheStats {
    /// Distinct root PEMs currently cached — never above `CA_CLIENT_CAP`.
    pub entries: usize,
    /// Client builds since process start: cold roots and rebuilds after an
    /// eviction. A cache hit never increments this.
    pub builds: u64,
    /// Least-recently-used evictions since process start.
    pub evictions: u64,
}

/// Introspection for the bounded per-PEM client cache. `/metrics` surfaces
/// the same numbers as `swarmail_webhook_ca_cache_*`.
pub fn ca_cache_stats() -> CaCacheStats {
    let cache = ca_clients().lock().unwrap();
    CaCacheStats {
        entries: cache.clients.len(),
        builds: CA_CACHE_BUILDS.load(Ordering::Relaxed),
        evictions: CA_CACHE_EVICTIONS.load(Ordering::Relaxed),
    }
}

/// Move `pem` to the most-recently-used end. A linear scan over at most
/// `CA_CLIENT_CAP` keys beats a linked list at this size. Only ever called
/// for a key that is in the cache, so the position lookup is an invariant,
/// not a search: a miss would mean the deque and the cache disagree.
fn touch_lru(lru: &mut VecDeque<String>, pem: &str) {
    let pos = lru
        .iter()
        .position(|k| k == pem)
        .expect("touch targets a cached key: the deque and the cache agree");
    let key = lru.remove(pos).expect("the position came from this deque");
    lru.push_back(key);
}

/// Evict the least-recently-used build. Called only to make room past
/// [`CA_CLIENT_CAP`], so the deque holds at least one cached key.
fn evict_lru(cache: &mut CaCache) {
    let oldest = cache
        .lru
        .pop_front()
        .expect("evict runs over the cap, so a cached key exists");
    cache.clients.remove(&oldest);
    CA_CACHE_EVICTIONS.fetch_add(1, Ordering::Relaxed);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTarget {
    /// Full URL, e.g. "http://127.0.0.1:9999/hooks/mail" or
    /// "https://hooks.internal/mail" (delivered over TLS via reqwest/rustls).
    pub url: String,
    /// Only fire for this inbox; None = every inbox.
    #[serde(default)]
    pub inbox: Option<String>,
    /// Sent verbatim as the X-Swarmail-Secret header.
    #[serde(default)]
    pub secret: Option<String>,
    /// Optional PEM of an extra root CA to trust for this target, on top of
    /// the built-in root store — how a self-signed test server is trusted
    /// while certificate verification stays on.
    #[serde(default)]
    pub ca_pem: Option<String>,
}

/// A copy of the delivery ledger: every death a notification can die has a
/// row. One instance per `Webhooks` — two servers count independently, and
/// no counter is process-global. The rows are cumulative since instance
/// start; they are exact at quiescence (when `/metrics` is read between
/// bursts), not under a live reader.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WebhookLedger {
    /// Deliveries answered 2xx.
    pub delivered: u64,
    /// Fan-outs refused at [`WEBHOOK_DELIVERY_CAP`] — and nothing else:
    /// shutdown loss has its own row.
    pub dropped: u64,
    /// Mails evicted from the firehose ring before the dispatcher saw them
    /// (`Lagged`). Unit: mails, not notifications — a lagged mail's fan-out
    /// is unknowable by construction.
    pub lagged: u64,
    /// Deliveries whose four-attempt retry budget was spent.
    pub exhausted: u64,
    /// Deliveries that ended without reporting an outcome: aborted at
    /// shutdown (together with any backlog mail's matching targets), or
    /// orphaned by a firehose close.
    pub aborted: u64,
}

/// Per-instance webhook state: the configured targets and the delivery
/// ledger. Plain `AtomicU64` fields — no statics, so an embedded second
/// server in one process counts independently of the first.
#[derive(Default)]
pub struct Webhooks {
    targets: RwLock<Vec<WebhookTarget>>,
    delivered: AtomicU64,
    dropped: AtomicU64,
    lagged: AtomicU64,
    exhausted: AtomicU64,
    aborted: AtomicU64,
}

impl Webhooks {
    pub fn set(&self, targets: Vec<WebhookTarget>) {
        *self.targets.write().unwrap() = targets;
    }

    pub fn current(&self) -> Vec<WebhookTarget> {
        self.targets.read().unwrap().clone()
    }

    /// A `Copy` of the ledger, in the `CaCacheStats` shape. Relaxed loads:
    /// the rows are independent observations, nothing is ordered behind
    /// them, and the exactness that matters holds at quiescence.
    pub fn snapshot(&self) -> WebhookLedger {
        WebhookLedger {
            delivered: self.delivered.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            lagged: self.lagged.load(Ordering::Relaxed),
            exhausted: self.exhausted.load(Ordering::Relaxed),
            aborted: self.aborted.load(Ordering::Relaxed),
        }
    }
}

/// What a firehose wakeup means; a pure function so every arm is provable.
enum Wake {
    /// A new email to deliver.
    Mail(Arc<crate::model::Email>),
    /// The watcher fell behind: `n` mails were lost. The store is the truth.
    Lagged(u64),
    /// The channel closed — the store is gone; stop.
    Stop,
}

fn classify(
    res: Result<Arc<crate::model::Email>, tokio::sync::broadcast::error::RecvError>,
) -> Wake {
    match res {
        Ok(email) => Wake::Mail(email),
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
            let note = format!(
                "webhook dispatcher lagged, skipped {n}; the store remains the source of truth"
            );
            tracing::warn!("{note}");
            Wake::Lagged(n)
        }
        Err(_) => Wake::Stop, // closed
    }
}

/// The dispatcher core: consume the firehose until it closes or stop is
/// signaled, hold at most [`WEBHOOK_DELIVERY_CAP`] deliveries at once, and
/// name every death in the ledger. Owned fn so the wake arms are provable
/// without a live server.
pub(crate) async fn dispatch_loop(
    mut rx: tokio::sync::broadcast::Receiver<Arc<crate::model::Email>>,
    webhooks: Arc<Webhooks>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut deliveries: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
    loop {
        // Biased: the firehose is polled first, the shutdown watch second. A
        // mail already in the ring is handed to the `Mail` arm — whose first
        // statement re-checks stop — so a buffered mail is counted, never
        // serialized; only an empty ring reaches the shutdown arm. Stop
        // latency stays decoupled from backlog × payload size.
        let wake = tokio::select! {
            biased;
            res = rx.recv() => classify(res),
            _ = shutdown.changed() => Wake::Stop,
        };
        match wake {
            Wake::Mail(email) => {
                // First statement: the shutdown check. Once stop is signaled
                // the payload is never built — counting is O(targets), the
                // build is O(bytes). A closed watch is a stop too: the
                // error path's drop-cascade rides the same arm.
                if shutdown.has_changed().unwrap_or(true) {
                    let matching = matching_count(webhooks.current(), &email);
                    webhooks
                        .aborted
                        .fetch_add(matching as u64, Ordering::Relaxed);
                    break;
                }
                let matching: Vec<WebhookTarget> = webhooks
                    .current()
                    .into_iter()
                    .filter(|t| t.inbox.as_deref().is_none_or(|i| i == email.inbox))
                    .collect();
                if matching.is_empty() {
                    // Nothing is owed for this mail.
                } else {
                    // Harvest finished deliveries first, so the bound
                    // reflects live capacity and not last pass's dead weight.
                    while deliveries.try_join_next().is_some() {}
                    if deliveries.len() >= WEBHOOK_DELIVERY_CAP {
                        // The bound is full before any bytes are touched: the
                        // whole fan-out is refused, and the payload that would
                        // have been built is never built.
                        webhooks
                            .dropped
                            .fetch_add(matching.len() as u64, Ordering::Relaxed);
                    } else {
                        // The payload is built once per mail — an `Arc<str>`
                        // the deliveries own. A delivery that outlives the
                        // mail (a sink that delays its reply, a retry against
                        // a pruned inbox) replays these bytes; it never
                        // re-reads the store.
                        let payload: Arc<str> = serde_json::json!({
                            "event": "received",
                            "email": &*email,
                        })
                        .to_string()
                        .into();
                        for target in matching {
                            // Re-checked per spawn: a mail that straddles the
                            // cap fills it and the rest of its targets are
                            // counted, one row each.
                            if deliveries.len() < WEBHOOK_DELIVERY_CAP {
                                let webhooks = Arc::clone(&webhooks);
                                let WebhookTarget {
                                    url,
                                    secret,
                                    ca_pem,
                                    ..
                                } = target;
                                deliveries.spawn(deliver(
                                    webhooks,
                                    url,
                                    secret,
                                    ca_pem,
                                    Arc::clone(&payload),
                                ));
                            } else {
                                webhooks.dropped.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            Wake::Lagged(n) => {
                // Unit: mails. Their fan-out is unknowable — the store stays
                // the truth, and the loss is a row, not a warn line alone.
                webhooks.lagged.fetch_add(n, Ordering::Relaxed);
            }
            Wake::Stop => break,
        }
    }
    // The tail — the one place both the stop arm and a firehose close land.
    // Every death before the loop returns gets a name: the sweep counts the
    // ring residents that never got a pass, then every delivery that ends
    // without reporting its own outcome is counted (an `Ok` already
    // reported, at its last await).
    while let Ok(email) = rx.try_recv() {
        let matching = matching_count(webhooks.current(), &email);
        webhooks
            .aborted
            .fetch_add(matching as u64, Ordering::Relaxed);
    }
    deliveries.abort_all();
    while let Some(res) = deliveries.join_next().await {
        if res.is_err() {
            webhooks.aborted.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// How many of `targets` fire for `email`.
fn matching_count(targets: Vec<WebhookTarget>, email: &crate::model::Email) -> usize {
    targets
        .into_iter()
        .filter(|t| t.inbox.as_deref().is_none_or(|i| i == email.inbox))
        .count()
}

/// Spawn the dispatcher into the caller's task set: consume the global
/// firehose, deliver with retry, honor `shutdown`. The subscription happens
/// HERE, synchronously, before the spawn — the caller is required to invoke
/// this before any listener binds, so a mail accepted at startup can never
/// miss its webhook (decision 0002, one layer down). The task lives in the
/// tracker, so [`crate::RunningServer::stop`] joins it.
pub fn spawn_dispatcher(
    store: &Store,
    webhooks: &Arc<Webhooks>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    tasks: &mut tokio::task::JoinSet<()>,
) {
    let rx = store.subscribe_all();
    tasks.spawn(dispatch_loop(rx, Arc::clone(webhooks), shutdown));
}

/// Deliver one fan-out, retrying within the budget. The outcome is
/// self-reported: both increments follow the function's last await, so a
/// task that reported can never also be counted aborted (cancellation takes
/// effect only at a yield point) — the drain can trust `Ok` arms.
async fn deliver(
    webhooks: Arc<Webhooks>,
    url: String,
    secret: Option<String>,
    ca_pem: Option<String>,
    body: Arc<str>,
) {
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(100 * 4u64.pow(attempt - 1))).await;
        }
        match post_json(&url, &body, secret.as_deref(), ca_pem.as_deref()).await {
            Ok(status) if (200..300).contains(&status) => {
                webhooks.delivered.fetch_add(1, Ordering::Relaxed);
                return;
            }
            Ok(status) => tracing::debug!(%url, status, "webhook non-2xx"),
            Err(e) => tracing::debug!(%url, error = %e, "webhook delivery failed"),
        }
    }
    webhooks.exhausted.fetch_add(1, Ordering::Relaxed);
    tracing::warn!(%url, "webhook dropped after retries");
}

/// Redirects are not part of the delivery contract. A webhook target is an
/// explicit endpoint chosen by configuration; if it answers 3xx, the
/// delivery is over — a non-2xx under the ordinary retry budget, not a new
/// destination. reqwest's default policy would forward the whole request to
/// wherever the target points, handing the `X-Swarmail-Secret` header (and
/// the body on 307/308) to that destination; so redirects are never
/// followed, and a redirecting target simply costs its retry attempts.
fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::none()
}

/// The client for targets that trust the built-in root store: rustls with
/// the webpki roots, certificate verification on, built once per process.
fn shared_client() -> reqwest::Client {
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .use_rustls_tls() // rustls only — no native-tls in the tree
                .redirect(redirect_policy())
                .build()
                .expect("the rustls webhook client must build")
        })
        .clone()
}

/// Pick the client for a target: the shared rustls client, or one pinned to
/// the target's own root CA, cached by PEM so retries reuse it. The cache is
/// bounded at [`CA_CLIENT_CAP`] with least-recently-used eviction: a cold
/// PEM beyond the cap retires the oldest build, and an evicted PEM's next
/// delivery simply rebuilds its client — never a failed delivery. The
/// built-in-roots client stays outside the cache and is built once.
fn ca_pem_error(e: impl std::fmt::Display) -> String {
    format!("webhook ca_pem could not be loaded into the rustls client: {e}")
}

fn client_for(ca_pem: Option<&str>) -> Result<reqwest::Client, String> {
    let Some(pem) = ca_pem else {
        return Ok(shared_client());
    };
    let mut cache = ca_clients().lock().unwrap();
    if let Some(client) = cache.clients.get(pem).cloned() {
        touch_lru(&mut cache.lru, pem);
        return Ok(client);
    }
    // One owner for the ca_pem failure: parse and build surface the same
    // text, so whichever stage refuses the PEM, the delivery error is alike.
    let root = reqwest::Certificate::from_pem(pem.as_bytes()).map_err(ca_pem_error)?;
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(redirect_policy()) // same no-redirect contract as the shared client
        .add_root_certificate(root)
        .build()
        .map_err(ca_pem_error)?;
    if cache.clients.len() >= CA_CLIENT_CAP {
        evict_lru(&mut cache); // make room: the least-recently-used build goes
    }
    cache.clients.insert(pem.to_string(), client.clone());
    cache.lru.push_back(pem.to_string());
    CA_CACHE_BUILDS.fetch_add(1, Ordering::Relaxed);
    Ok(client)
}

async fn post_json(
    url: &str,
    body: &str,
    secret: Option<&str>,
    ca_pem: Option<&str>,
) -> Result<u16, String> {
    let url = reqwest::Url::parse(url).map_err(|e| format!("invalid webhook URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("unsupported webhook URL scheme: {}", url.scheme()));
    }
    let client = client_for(ca_pem)?;
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .timeout(POST_TIMEOUT)
        .body(body.to_string());
    if let Some(secret) = secret {
        request = request.header("X-Swarmail-Secret", secret);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    Ok(response.status().as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Email;
    use tokio::sync::broadcast;

    fn email() -> Email {
        Email {
            id: "1".into(),
            inbox: "default".into(),
            from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: None,
            received_at: String::new(),
            received_ms: 0,
            size: 0,
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

    /// Every arm of the firehose classifier, provable without a live server.
    #[test]
    fn classify_mail_lagged_stop() {
        assert!(matches!(classify(Ok(Arc::new(email()))), Wake::Mail(_)));
        assert!(matches!(
            classify(Err(broadcast::error::RecvError::Lagged(3))),
            Wake::Lagged(3)
        ));
        assert!(matches!(
            classify(Err(broadcast::error::RecvError::Closed)),
            Wake::Stop
        ));
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;
    use crate::model::Email;
    use std::sync::atomic::AtomicUsize;
    use tokio::sync::broadcast;

    fn email() -> Email {
        Email {
            id: "1".into(),
            inbox: "default".into(),
            from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: None,
            received_at: String::new(),
            received_ms: 0,
            size: 0,
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

    /// Mail is delivered, a lagging watcher retries, a closed firehose ends
    /// the loop — all three wake arms against a real broadcast channel, and
    /// the lag actually lands in the log, as promised. The lag also lands in
    /// the ledger: a lost mail is a row, not a warn line alone.
    #[tokio::test]
    async fn dispatch_loop_handles_mail_lag_and_close() {
        let log = captured_log();
        let webhooks = Arc::new(Webhooks::default()); // no targets: delivery is a no-op
        let (tx, rx) = broadcast::channel(1);
        // The stop watch's sender is held for the whole test: the loop must
        // end on the firehose close (the `classify` arm), not on a signal.
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // Both sends happen BEFORE the loop exists: the first recv is then
        // guaranteed to lag (capacity 1 holds one mail, one was lost), so
        // the Lagged arm is deterministic — no race with the loop's speed.
        tx.send(Arc::new(email())).unwrap(); // lost: overwritten in the ring
        tx.send(Arc::new(email())).unwrap(); // buffered: the next recv after the lag
        let loop_task = tokio::spawn(dispatch_loop(rx, Arc::clone(&webhooks), shutdown_rx));
        // The wait is on the ledger row the loop itself writes (never a
        // sleep the test depends on): once `lagged` moved, the ring is
        // drained and the close below can only hit the `Stop` arm.
        spin_until(2000, || webhooks.snapshot().lagged == 1).await;
        drop(tx); // close the firehose → Stop → the loop returns

        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the loop must end when the firehose closes")
            .unwrap();
        std::io::Write::flush(&mut log.clone()).unwrap();
        let logged = String::from_utf8_lossy(&log.0.lock().unwrap()).to_string();
        assert!(
            logged.contains("lagged"),
            "the lag warning must be logged: {logged}"
        );
        assert_eq!(
            webhooks.snapshot().lagged,
            1,
            "the lost mail is a ledger row (unit: mails)"
        );
    }

    /// A stop-signaled backlog is counted, never serialized (AC2): the stop
    /// and two mails land with no await between, so on this current-thread
    /// runtime the loop's first poll sees both ready and the biased select
    /// hands it the firehose — the `Mail` arm's first statement sees the
    /// signaled watch, counts the mail's matching targets and breaks; the
    /// tail sweep counts the ring resident the loop never got to. The sink
    /// records nothing.
    #[tokio::test]
    async fn a_stop_signaled_backlog_is_counted_and_never_serialized() {
        let (addr, hits, _retire, _sink) = counting_sink(None).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![target(format!("http://{addr}/hook"))]);
        let (tx, rx) = broadcast::channel(8);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(rx, Arc::clone(&webhooks), shutdown_rx));

        let _ = shutdown_tx.send(true);
        tx.send(Arc::new(email())).unwrap();
        tx.send(Arc::new(email())).unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("stop must end the loop")
            .unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(
            ledger.aborted, 2,
            "one counted at the arm check, one swept at the tail"
        );
        assert_eq!(ledger.delivered, 0, "nothing was delivered");
        assert_eq!(ledger.dropped, 0, "a shutdown loss is not a bound refusal");
        assert_eq!(
            hits.load(Ordering::Relaxed),
            0,
            "the sink records nothing: no payload was ever built"
        );
    }

    /// The tail drain: a delivery that finished before the watch fired is
    /// counted as its own outcome (an `Ok` already reported at its last
    /// await, so it can never also be counted aborted); the one cancelled
    /// mid-flight is counted aborted. AC2's counting half.
    #[tokio::test]
    async fn the_drain_counts_reported_deliveries_and_cancels_the_rest() {
        let (fast, _, _retire, _sink) = counting_sink(Some(FAST_REPLY)).await;
        let (stalled, _, stalled_retire, stalled_sink) = counting_sink(None).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![
            target(format!("http://{fast}/hook")),
            target(format!("http://{stalled}/hook")),
        ]);
        let (tx, rx) = broadcast::channel(8);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(rx, Arc::clone(&webhooks), shutdown_rx));

        tx.send(Arc::new(email())).unwrap();
        // The report is the delivery task's final statements — after its
        // last await — so `delivered == 1` proves the task is complete and
        // sits unreaped in the set: exactly the `Ok` arm the drain must see.
        spin_until(5000, || webhooks.snapshot().delivered == 1).await;
        let _ = shutdown_tx.send(true);

        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("stop must end the loop")
            .unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(
            ledger.delivered, 1,
            "the reported delivery is its own outcome"
        );
        assert_eq!(ledger.aborted, 1, "the cancelled delivery is aborted");
        assert_eq!(ledger.dropped, 0);
        assert_eq!(ledger.exhausted, 0);
        assert_eq!(ledger.lagged, 0);

        // The sink outlives nothing: retiring it ends its accept loop —
        // no test leaves a listener spinning under the coverage run.
        drop(stalled_retire);
        tokio::time::timeout(std::time::Duration::from_secs(2), stalled_sink)
            .await
            .expect("the retired sink's accept loop must end")
            .unwrap();
        drop(_retire);
    }

    /// The reap keeps the bound honest and a straddling mail fills it: 31
    /// stalled deliveries hold the cap while the one fast delivery is
    /// reaped on the next pass, so a two-target mail spawns one and counts
    /// exactly one dropped — the per-spawn re-check firing mid-mail.
    #[tokio::test]
    async fn the_reap_keeps_the_bound_honest_and_a_straddling_mail_fills_it() {
        let (fast, _, _fast_retire, _fast_sink) = counting_sink(Some(FAST_REPLY)).await;
        let (stalled, stalled_hits, _stalled_retire, _stalled_sink) = counting_sink(None).await;
        let webhooks = Arc::new(Webhooks::default());
        let mut targets = vec![target(format!("http://{fast}/hook"))];
        targets.extend(std::iter::repeat_n(
            target(format!("http://{stalled}/hook")),
            WEBHOOK_DELIVERY_CAP - 1,
        ));
        webhooks.set(targets);
        let (tx, rx) = broadcast::channel(8);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(rx, Arc::clone(&webhooks), shutdown_rx));

        tx.send(Arc::new(email())).unwrap();
        // All 32 spawned: the stalled sink holds 31 slots and the fast one
        // has reported. Only one mail exists, so no later pass can reap the
        // fast delivery — it sits in the set, complete.
        spin_until(5000, || {
            stalled_hits.load(Ordering::Relaxed) == WEBHOOK_DELIVERY_CAP - 1
        })
        .await;
        spin_until(5000, || webhooks.snapshot().delivered == 1).await;

        // Now the bound is exactly full. Two more targets in one pass: the
        // reap removes the finished delivery, the first spawn fills the
        // cap, and the second target is counted dropped.
        webhooks.set(vec![
            target(format!("http://{stalled}/hook")),
            target(format!("http://{stalled}/hook")),
        ]);
        tx.send(Arc::new(email())).unwrap();
        spin_until(5000, || webhooks.snapshot().dropped == 1).await;
        spin_until(5000, || {
            stalled_hits.load(Ordering::Relaxed) == WEBHOOK_DELIVERY_CAP
        })
        .await;

        let _ = shutdown_tx.send(true);
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("stop must end the loop")
            .unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(ledger.dropped, 1, "exactly one target refused at the bound");
        assert_eq!(ledger.delivered, 1);
        assert_eq!(
            ledger.aborted, WEBHOOK_DELIVERY_CAP as u64,
            "every stalled slot was cancelled and counted"
        );
        assert_eq!(
            stalled_hits.load(Ordering::Relaxed),
            WEBHOOK_DELIVERY_CAP,
            "32 connections reached the sink: the reap kept a slot free"
        );
        assert_eq!(ledger.exhausted, 0);
        assert_eq!(ledger.lagged, 0);
    }

    /// Two `Webhooks::default()` ledgers count independently — the refusal
    /// of the process-global statics precedent (AC5's cross-contamination
    /// half). Each loop's stop-signaled backlog counts only its own ledger.
    #[tokio::test]
    async fn two_ledgers_count_independently() {
        // The URL is never dialed: the stop path counts, it never spawns.
        let a = Arc::new(Webhooks::default());
        a.set(vec![target("http://127.0.0.1:9/hook".into())]);
        let (tx_a, rx_a) = broadcast::channel(8);
        let (stop_a, rx_stop_a) = tokio::sync::watch::channel(false);
        let loop_a = tokio::spawn(dispatch_loop(rx_a, Arc::clone(&a), rx_stop_a));
        let _ = stop_a.send(true);
        tx_a.send(Arc::new(email())).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_a)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(a.snapshot().aborted, 1, "loop a counts its own backlog");
        assert_eq!(a.snapshot().delivered, 0);

        let b = Arc::new(Webhooks::default());
        b.set(vec![target("http://127.0.0.1:9/hook".into())]);
        let (tx_b, rx_b) = broadcast::channel(8);
        let (stop_b, rx_stop_b) = tokio::sync::watch::channel(false);
        let loop_b = tokio::spawn(dispatch_loop(rx_b, Arc::clone(&b), rx_stop_b));
        let _ = stop_b.send(true);
        tx_b.send(Arc::new(email())).unwrap();
        tx_b.send(Arc::new(email())).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_b)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(b.snapshot().aborted, 2, "loop b counts its own backlog");
        assert_eq!(
            a.snapshot().aborted,
            1,
            "a's ledger must not move: no cross-contamination"
        );
    }

    /// The subscription is synchronous: the moment `spawn_dispatcher`
    /// returns, the dispatcher is a live firehose consumer. `run_on` calls
    /// it before TLS is parsed and before any listener binds, so a mail
    /// accepted in the startup window — however early — cannot miss its
    /// webhook (subscribe-before-bind, 0002 one layer down). This test
    /// binds nothing: the insert plays the accept that would otherwise race
    /// the first listener.
    #[tokio::test]
    async fn a_mail_landing_after_spawn_dispatcher_returns_is_delivered() {
        let (addr, hits, retire, sink) = counting_sink(Some(FAST_REPLY)).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![target(format!("http://{addr}/hook"))]);
        let store = Store::new(0);
        let mut tasks = tokio::task::JoinSet::new();
        let (_stop_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        spawn_dispatcher(&store, &webhooks, shutdown_rx, &mut tasks);
        // The accept, before any listener could exist: the mail lands in
        // the firehose the dispatcher above already subscribed to.
        store.insert(email()).unwrap();
        spin_until(5000, || webhooks.snapshot().delivered == 1).await;
        assert_eq!(
            hits.load(Ordering::Relaxed),
            1,
            "the startup mail reached the wire"
        );

        // The sink outlives nothing: retiring it ends its accept loop.
        drop(retire);
        tokio::time::timeout(std::time::Duration::from_secs(2), sink)
            .await
            .expect("the retired sink's accept loop must end")
            .unwrap();
        drop(tasks);
    }

    /// The reply a fast sink sends: 204, connection closed after.
    const FAST_REPLY: &str =
        "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    fn target(url: String) -> WebhookTarget {
        WebhookTarget {
            url,
            inbox: None,
            secret: None,
            ca_pem: None,
        }
    }

    /// Spin-yield until a production observable holds. Not sleep-as-sync:
    /// the wait is on a counter the production code itself writes — the
    /// deadline only turns a broken implementation into a failure message
    /// instead of a hang.
    async fn spin_until(deadline_ms: u64, fact: impl Fn() -> bool) {
        let fut = async {
            while !fact() {
                tokio::task::yield_now().await;
            }
        };
        tokio::time::timeout(std::time::Duration::from_millis(deadline_ms), fut)
            .await
            .expect("the production observable never became true");
    }

    /// A real loopback HTTP sink for the dispatcher core. `reply`: `None`
    /// accepts and never answers — a stalled target holds its slot for the
    /// budget; `Some` answers every request with those exact bytes. Returns
    /// the bound address, the connection count — a production observable the
    /// tests spin on — the retire switch, and the accept task: a test that
    /// drops the switch and awaits the task leaves no accept loop spinning
    /// under the coverage run. Every connection is served in its own task.
    async fn counting_sink(
        reply: Option<&'static str>,
    ) -> (
        std::net::SocketAddr,
        Arc<AtomicUsize>,
        tokio::sync::watch::Sender<bool>,
        tokio::task::JoinHandle<()>,
    ) {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        let (retire, mut retired) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    res = listener.accept() => res,
                    // Retired (or the test's sender dropped): end the loop.
                    _ = retired.changed() => break,
                };
                let (mut stream, _) = accepted.expect("the sink's listener must keep accepting");
                counter.fetch_add(1, Ordering::Relaxed);
                match reply {
                    // Answered: read the request, reply, close.
                    Some(status) => {
                        let _ = read_request(&mut stream).await;
                        let _ = stream.write_all(status.as_bytes()).await;
                        let _ = stream.flush().await;
                    }
                    // Stalled: read the request, then never answer. The
                    // socket is parked rather than dropped — the drop would
                    // close it and un-wedge the delivery. The fd comes back
                    // when the test process exits.
                    None => {
                        let _ = read_request(&mut stream).await;
                        std::mem::forget(stream);
                    }
                }
            }
        });
        (addr, hits, retire, task)
    }

    /// One HTTP request off a raw socket: head plus its content-length
    /// body. The dispatcher's payloads are small; this only ever walks a
    /// few hundred bytes. `None` when the peer hangs up mid-request.
    async fn read_request(stream: &mut tokio::net::TcpStream) -> Option<String> {
        use tokio::io::AsyncReadExt;
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            if let Some(pos) = find(&buf, b"\r\n\r\n") {
                let body_start = pos + 4;
                let head = String::from_utf8_lossy(&buf[..pos]).to_ascii_lowercase();
                let len = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if buf.len() >= body_start + len {
                    return Some(String::from_utf8_lossy(&buf).into_owned());
                }
            }
            let n = stream.read(&mut chunk).await.ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// The process-wide capturing log: the dispatcher's warnings must be
    /// evaluatable (a subscriber makes the warn! arguments real code), and
    /// tests can then assert on them.
    static CAPTURED_LOG: std::sync::OnceLock<Capture> = std::sync::OnceLock::new();

    #[derive(Clone, Default)]
    struct Capture(Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
        type Writer = Capture;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn captured_log() -> &'static Capture {
        CAPTURED_LOG.get_or_init(|| {
            let capture = Capture::default();
            let _ = tracing::subscriber::set_global_default(
                tracing_subscriber::fmt::Subscriber::builder()
                    .with_max_level(tracing::Level::WARN)
                    .with_writer(capture.clone())
                    .finish(),
            );
            capture
        })
    }
    /// The sink's reader is pinned at its edges, because the dispatcher
    /// tests above lean on it: a request split across segments is read
    /// whole, and a peer that hangs up before the body is complete is no
    /// request at all.
    #[tokio::test]
    async fn the_sink_reader_takes_split_requests_and_refuses_silent_ones() {
        use tokio::io::AsyncWriteExt;

        // Split across segments: the head with a partial body, a yield so
        // the sink is parked mid-request, then the rest — one request.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            read_request(&mut stream).await
        });
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"POST /hook HTTP/1.1\r\nHost: sink\r\nContent-Length: 6\r\n\r\nhe")
            .await
            .unwrap();
        tokio::task::yield_now().await;
        client.write_all(b"llo!").await.unwrap();
        let read = tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .expect("the split request must be read")
            .unwrap();
        assert_eq!(
            read.as_deref()
                .map(|r| r.rsplit("\r\n\r\n").next().unwrap_or("")),
            Some("hello!"),
            "a request split across segments is one request: {read:?}"
        );

        // A peer that promises a body it never finishes and hangs up
        // mid-request: the incomplete-body check runs against a present
        // head, the next read is the peer's EOF, and no request is
        // invented. Deterministic — no split timing to bet on.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            read_request(&mut stream).await
        });
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"POST /hook HTTP/1.1\r\nContent-Length: 6\r\n\r\nhe")
            .await
            .unwrap();
        drop(client);
        let read = tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .expect("the silent peer must not hang the reader")
            .unwrap();
        assert_eq!(read, None, "no body, no request: {read:?}");
    }
}

#[cfg(test)]
mod delivery_tests {
    use super::*;

    /// A minimal but real self-signed CA, PEM-encoded, for client tests.
    /// Every call mints a fresh key, so every call yields a PEM that is
    /// globally distinct — a brand-new cache key each time.
    fn ca_pem() -> String {
        let key = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
        params.self_signed(&key).unwrap().pem()
    }

    /// The per-PEM cache is process-global, so cache-touching tests
    /// serialize on this: one test's eviction flood must not pull another
    /// test's PEM out from under a precise counter assertion.
    static CACHE_TESTS: Mutex<()> = Mutex::new(());

    /// The shared client for `ca_pem: None` stays outside the cache; a cold
    /// PEM builds once; the same PEM again is a hit that rebuilds nothing.
    #[test]
    fn clients_are_cached_per_ca() {
        let _g = CACHE_TESTS.lock().unwrap();
        let pem = ca_pem();
        let before = ca_cache_stats();

        client_for(Some(&pem)).unwrap();
        let built = ca_cache_stats();
        assert_eq!(built.builds, before.builds + 1, "a cold PEM builds once");
        assert_eq!(
            built.entries,
            (before.entries + 1).min(CA_CLIENT_CAP),
            "the PEM is cached, within the bound (an insert at the cap evicts, not grows)"
        );

        client_for(Some(&pem)).unwrap();
        let hit = ca_cache_stats();
        assert_eq!(
            hit.builds, built.builds,
            "the same PEM must reuse the client, not rebuild"
        );
        assert_eq!(
            hit.entries, built.entries,
            "a hit does not change the cache size"
        );

        client_for(None).unwrap();
        let shared = ca_cache_stats();
        assert_eq!(
            shared.builds, hit.builds,
            "the built-in-roots client is outside the cache: no build"
        );
        assert_eq!(
            shared.entries, hit.entries,
            "the built-in-roots client is not cached"
        );
    }

    /// The cache never exceeds its cap, however many distinct PEMs arrive:
    /// each cold root beyond the cap evicts the least-recently-used build.
    /// Observable through the production stats hook (`/metrics` surfaces
    /// the same counters) — no test-only introspection.
    #[test]
    fn cache_stays_within_the_cap_and_counts_evictions() {
        let _g = CACHE_TESTS.lock().unwrap();
        let before = ca_cache_stats();

        // More distinct cold roots than the cache can hold, all built once.
        for _ in 0..CA_CLIENT_CAP + 8 {
            client_for(Some(&ca_pem())).unwrap();
        }

        let after = ca_cache_stats();
        assert!(
            after.entries <= CA_CLIENT_CAP,
            "the cache must never exceed its cap, saw {}",
            after.entries
        );
        assert_eq!(
            after.entries, CA_CLIENT_CAP,
            "a flood of distinct roots fills the cache exactly to the cap"
        );
        // At least 8 of the cap+8 cold roots had to retire something: the
        // cache held `before.entries <= cap` keys and ends at the cap.
        let evicted = after.evictions - before.evictions;
        assert!(
            evicted >= 8,
            "distinct roots beyond the cap must have evicted, saw only {evicted}"
        );
        assert_eq!(
            after.builds,
            before.builds + CA_CLIENT_CAP as u64 + 8,
            "each distinct cold PEM built exactly once"
        );
    }

    /// Eviction is least-recently-used, deterministically: a root that keeps
    /// being used survives a wave of cold roots that retires its untouched
    /// older neighbor — and the retired neighbor's next delivery still
    /// works, by rebuilding.
    #[test]
    fn eviction_is_least_recently_used_and_the_evicted_pem_rebuilds() {
        let _g = CACHE_TESTS.lock().unwrap();

        // Fill the cache exactly, on top of whatever earlier tests left:
        // cap fresh roots always end at the cap, whoever came before.
        for _ in 0..CA_CLIENT_CAP {
            client_for(Some(&ca_pem())).unwrap();
        }

        let touched = ca_pem();
        let older = ca_pem();
        client_for(Some(&touched)).unwrap();
        client_for(Some(&older)).unwrap();
        client_for(Some(&touched)).unwrap(); // touched is now MRU; older is not

        // cap-1 further cold builds: each retires the current LRU, working
        // forward to `older`. `touched`, being more recent, is still cached.
        for _ in 0..CA_CLIENT_CAP - 1 {
            client_for(Some(&ca_pem())).unwrap();
        }

        let builds = ca_cache_stats().builds;
        client_for(Some(&touched)).unwrap();
        assert_eq!(
            ca_cache_stats().builds,
            builds,
            "the recently-used root must survive the wave: a hit, no rebuild"
        );
        client_for(Some(&older)).unwrap();
        assert_eq!(
            ca_cache_stats().builds,
            builds + 1,
            "the untouched older root must have been evicted, and its next use rebuilds"
        );
    }

    /// A URL that does not parse at all is a delivery error, not a panic.
    #[tokio::test]
    async fn unparseable_url_is_an_error() {
        let err = post_json("not a url", "{}", None, None).await.unwrap_err();
        assert!(err.contains("invalid webhook URL"), "{err}");
    }

    /// Schemes other than http/https are rejected before any connection.
    #[tokio::test]
    async fn unsupported_scheme_is_an_error() {
        let err = post_json("ftp://hooks.example/x", "{}", None, None)
            .await
            .unwrap_err();
        assert!(err.contains("unsupported webhook URL scheme: ftp"), "{err}");
    }

    /// A ca_pem that is not a PEM certificate fails before connecting.
    #[tokio::test]
    async fn bad_ca_pem_is_an_error() {
        let err = client_for(Some("-----BEGIN CERTIFICATE-----\nnope")).unwrap_err();
        assert!(err.contains("ca_pem"), "{err}");
    }
}
