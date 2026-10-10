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

#[derive(Default)]
pub struct Webhooks {
    targets: RwLock<Vec<WebhookTarget>>,
}

impl Webhooks {
    pub fn set(&self, targets: Vec<WebhookTarget>) {
        *self.targets.write().unwrap() = targets;
    }

    pub fn current(&self) -> Vec<WebhookTarget> {
        self.targets.read().unwrap().clone()
    }
}

/// What a firehose wakeup means; a pure function so every arm is provable.
enum Wake {
    /// A new email to deliver.
    Mail(Arc<crate::model::Email>),
    /// A transient condition (lag) — keep looping, the store is the truth.
    Again,
    /// The channel closed — the store is gone; stop.
    Stop,
}

fn classify(
    res: Result<Arc<crate::model::Email>, tokio::sync::broadcast::error::RecvError>,
) -> Wake {
    match res {
        Ok(email) => Wake::Mail(email),
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
            tracing::warn!(
                skipped = n,
                "webhook dispatcher lagged; store remains source of truth"
            );
            Wake::Again
        }
        Err(_) => Wake::Stop, // closed
    }
}

/// The dispatcher core: consume the firehose until it closes. Owned fn so
/// the wake arms are provable without a live server.
pub(crate) async fn dispatch_loop(
    mut rx: tokio::sync::broadcast::Receiver<Arc<crate::model::Email>>,
    webhooks: Arc<Webhooks>,
) {
    loop {
        match classify(rx.recv().await) {
            Wake::Mail(email) => {
                let targets = webhooks.current();
                for t in targets {
                    if t.inbox.as_deref().is_some_and(|i| i != email.inbox) {
                        continue;
                    }
                    let payload = serde_json::json!({
                        "event": "received",
                        "email": &*email,
                    });
                    let body = payload.to_string();
                    tokio::spawn(deliver(t.url, t.secret, t.ca_pem, body));
                }
            }
            Wake::Again => {}
            Wake::Stop => return,
        }
    }
}

/// Spawn the dispatcher: consume the global firehose, deliver with retry.
pub fn spawn_dispatcher(store: Arc<Store>, webhooks: Arc<Webhooks>) {
    let rx = store.subscribe_all();
    tokio::spawn(dispatch_loop(rx, webhooks));
}

async fn deliver(url: String, secret: Option<String>, ca_pem: Option<String>, body: String) {
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(100 * 4u64.pow(attempt - 1))).await;
        }
        match post_json(&url, &body, secret.as_deref(), ca_pem.as_deref()).await {
            Ok(status) if (200..300).contains(&status) => return,
            Ok(status) => tracing::debug!(%url, status, "webhook non-2xx"),
            Err(e) => tracing::debug!(%url, error = %e, "webhook delivery failed"),
        }
    }
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
    fn classify_mail_again_stop() {
        assert!(matches!(classify(Ok(Arc::new(email()))), Wake::Mail(_)));
        assert!(matches!(
            classify(Err(broadcast::error::RecvError::Lagged(3))),
            Wake::Again
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
    /// the loop — all three wake arms against a real broadcast channel.
    #[tokio::test]
    async fn dispatch_loop_handles_mail_lag_and_close() {
        let webhooks = Arc::new(Webhooks::default()); // no targets: delivery is a no-op
        let (tx, rx) = broadcast::channel(1);
        // Both sends happen BEFORE the loop exists: the first recv is then
        // guaranteed to lag (capacity 1 holds one mail, one was lost), so
        // the Lagged arm is deterministic — no race with the loop's speed.
        tx.send(Arc::new(email())).unwrap(); // lost: overwritten in the ring
        tx.send(Arc::new(email())).unwrap(); // buffered: the next recv after the lag
        let loop_task = tokio::spawn(dispatch_loop(rx, webhooks));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        drop(tx); // close the firehose → Stop → the loop returns

        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the loop must end when the firehose closes")
            .unwrap();
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
