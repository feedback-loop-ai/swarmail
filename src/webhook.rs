//! Outbound webhooks: queued, retried, never awaited by SMTP acceptance.
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
//!
//! The boundary (decision 0041): a delivery dials the configured address
//! and nothing else — no environment proxy, no redirect hop. A URL leaves
//! the process (an echo, a log line) only with its userinfo removed, and a
//! mail's payload is serialized once into shared bytes every target and
//! every retry reuses.

pub(crate) mod ca_executor;
use crate::store::Store;
use axum::body::Bytes;
use ca_executor::{Charged, Job, Output, Scope, Size};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

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
/// target holds its slot for four 5 s network-attempt timeouts plus the
/// 100/400/1600 ms backoff — 22.1 s total after client construction. Cold
/// admission/build time is outside that sum. Two dedicated CA workers (#220)
/// cannot consume SMTP/Store blocking slots; global CA admission can refuse
/// healthy targets and makes no fairness or OS scheduling promise.
/// Surfaced on `/metrics`.
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
    /// Successful delivery-side client constructions since process start,
    /// including duplicate builds whose result loses a publication race or
    /// whose awaiting delivery is cancelled. API validation/projection,
    /// failures and cache hits do not increment this.
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
fn evict_lru(cache: &mut CaCache) -> reqwest::Client {
    let oldest = cache
        .lru
        .pop_front()
        .expect("evict runs over the cap, so a cached key exists");
    let removed = cache
        .clients
        .remove(&oldest)
        .expect("the LRU key is present in the client map");
    CA_CACHE_EVICTIONS.fetch_add(1, Ordering::Relaxed);
    removed
}

/// A webhook target as PUT supplies it and GET echoes it. The request shape
/// is closed (decision 0048 §2, issue #135): an unknown key refuses — a
/// misspelled `secret` used to install a target with no secret — and the
/// optional strings default only on omission, so a present `null` refuses
/// like any other non-string. The echo still carries `null` for an absent
/// or withheld value (0048 §3): the response is a diagnostic, not a
/// template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookTarget {
    /// Full URL, e.g. "http://127.0.0.1:9999/hooks/mail" or
    /// "https://hooks.internal/mail" (delivered over TLS via reqwest/rustls).
    #[serde(deserialize_with = "text")]
    pub url: String,
    /// Only fire for this inbox; None = every inbox.
    #[serde(default, deserialize_with = "optional_text")]
    pub inbox: Option<String>,
    /// Sent verbatim as the X-Swarmail-Secret header — so it must be one
    /// (decision 0041 §1, [`secret_header`]).
    #[serde(default, deserialize_with = "optional_text")]
    pub secret: Option<String>,
    /// Optional PEM of an extra root CA to trust for this target, on top of
    /// the built-in root store — how a self-signed test server is trusted
    /// while certificate verification stays on.
    #[serde(default, deserialize_with = "optional_text")]
    pub ca_pem: Option<String>,
}

/// The PUT body: a JSON array of [`WebhookTarget`] objects. Its refusals
/// name the JSON *kind* that arrived, never its content (decision 0048 §2):
/// serde's own wording quotes a wrong-typed value — `["http://user:pw@h/"]`
/// would echo the credential — so every shape below the array is read
/// through [`Shape`], whose refusals are value-free.
#[derive(Debug)]
pub struct WebhookTargets(pub Vec<WebhookTarget>);

impl<'de> Deserialize<'de> for WebhookTargets {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(ShapeVisitor::<WebhookTargets>(std::marker::PhantomData))
    }
}

/// One element of the PUT array: an object, handed to the derived (closed)
/// [`WebhookTarget`] reader — anything else refuses by kind.
struct TargetSlot(WebhookTarget);

impl<'de> Deserialize<'de> for TargetSlot {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(ShapeVisitor::<TargetSlot>(std::marker::PhantomData))
    }
}

/// A target's string value, read value-free.
struct Text(String);

/// A required string field: refused by kind, never echoed.
fn text<'de, D: serde::Deserializer<'de>>(de: D) -> Result<String, D::Error> {
    de.deserialize_any(ShapeVisitor::<Text>(std::marker::PhantomData))
        .map(|t| t.0)
}

/// An optional string field: omission is `None` through `#[serde(default)]`,
/// a present `null` refuses like any other non-string (decision 0025 §2).
fn optional_text<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Option<String>, D::Error> {
    text(de).map(Some)
}

/// The value-free reading of one JSON shape (decision 0048 §2): the
/// accepted kind is handed on, every other kind refuses with the sentence
/// `expected <want>, found <kind>` — the kind, never the content.
trait Shape: Sized {
    /// What the shape is, in the refusal and the visitor's `expecting`.
    const WANT: &'static str;

    fn string<E: serde::de::Error>(_: &str) -> Result<Self, E> {
        Err(refuse_kind(Self::WANT, "a string"))
    }

    fn seq<'de, A: serde::de::SeqAccess<'de>>(_: A) -> Result<Self, A::Error> {
        Err(refuse_kind(Self::WANT, "an array"))
    }

    fn map<'de, A: serde::de::MapAccess<'de>>(_: A) -> Result<Self, A::Error> {
        Err(refuse_kind(Self::WANT, "an object"))
    }
}

/// The one refusal sentence of a wrong-kinded JSON value.
fn refuse_kind<E: serde::de::Error>(want: &str, found: &str) -> E {
    E::custom(format!("expected {want}, found {found}"))
}

impl Shape for WebhookTargets {
    const WANT: &'static str = "a JSON array of webhook targets";

    fn seq<'de, A: serde::de::SeqAccess<'de>>(mut seq: A) -> Result<Self, A::Error> {
        let mut targets = Vec::new();
        while let Some(TargetSlot(target)) = seq.next_element()? {
            targets.push(target);
        }
        Ok(WebhookTargets(targets))
    }
}

impl Shape for TargetSlot {
    const WANT: &'static str = "a webhook target object";

    fn map<'de, A: serde::de::MapAccess<'de>>(map: A) -> Result<Self, A::Error> {
        WebhookTarget::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            .map(TargetSlot)
    }
}

impl Shape for Text {
    const WANT: &'static str = "a string";

    fn string<E: serde::de::Error>(value: &str) -> Result<Self, E> {
        Ok(Text(value.to_string()))
    }
}

/// The visitor every [`Shape`] reads through. Every JSON kind has its own
/// arm, so serde's value-quoting `invalid_type` default is never reached
/// from JSON.
struct ShapeVisitor<T>(std::marker::PhantomData<T>);

impl<'de, T: Shape> serde::de::Visitor<'de> for ShapeVisitor<T> {
    type Value = T;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(T::WANT)
    }

    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<T, E> {
        Err(refuse_kind(T::WANT, "a boolean"))
    }

    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<T, E> {
        Err(refuse_kind(T::WANT, "a number"))
    }

    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<T, E> {
        Err(refuse_kind(T::WANT, "a number"))
    }

    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<T, E> {
        Err(refuse_kind(T::WANT, "a number"))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<T, E> {
        T::string(value)
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<T, E> {
        Err(refuse_kind(T::WANT, "null"))
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, seq: A) -> Result<T, A::Error> {
        T::seq(seq)
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::map(map)
    }
}

impl WebhookTarget {
    /// The wire rule (decision 0029 §5): every supplied value is validated
    /// where it is authored. `set_webhooks` calls this before any effect, so
    /// a target that cannot be delivered is a 400 at PUT time — never a
    /// misconfiguration that surfaces only at delivery. The URL and PEM
    /// checks are the delivery stack's own functions ([`parse_target_url`],
    /// [`build_ca_client`]), one owner, both arms warm. The PEM check
    /// builds a whole client from every root (decision 0048 §1) and drops
    /// it: no cache entry, no counter, no dial.
    pub fn validate(&self) -> Result<(), String> {
        parse_target_url(&self.url)?;
        // A supplied inbox filter names an inbox by the one rule every
        // other plane admits with (decision 0046 §6): blank or
        // control-bearing refuses, padding is matched verbatim. An absent
        // filter is still every inbox.
        if let Some(inbox) = &self.inbox {
            crate::model::validate_inbox(inbox).map_err(String::from)?;
        }
        if let Some(pem) = &self.ca_pem {
            build_ca_client(pem)?;
        }
        if let Some(secret) = &self.secret {
            secret_header(secret)?;
        }
        Ok(())
    }

    /// The outward copy of a target (decisions 0041 §3 and accepted 0067):
    /// the secret is withheld, the URL loses userinfo and CA material is
    /// reconstructed from complete, delivery-stack-loadable certificates.
    /// The installed target remains untouched for delivery. This synchronous
    /// form stays available to embedders; HTTP handlers use the same
    /// projection on a dedicated CA worker.
    pub fn redacted(&self) -> WebhookTarget {
        WebhookTarget {
            url: display_url(&self.url),
            inbox: self.inbox.clone(),
            secret: None,
            ca_pem: self.ca_pem.as_deref().map(public_ca_pem),
        }
    }
}

/// Fail-closed public CA projection. Keep only certificate DER yielded by
/// the same complete PEM parser as admission, encode it canonically and
/// validate the entire reconstructed bundle through the delivery builder.
/// Any parse/load failure discards all roots rather than returning partial
/// material or the original input. This protects direct `Webhooks::set`
/// callers as well as the HTTP boundary; it does not alter installed bytes.
fn public_ca_pem(pem: &str) -> String {
    use base64::Engine as _;

    let mut certs = Vec::new();
    for cert in rustls_pemfile::certs(&mut pem.as_bytes()) {
        match cert {
            Ok(cert) => certs.push(cert),
            Err(_) => return String::new(),
        }
    }
    if certs.is_empty() {
        return String::new();
    }

    let mut projected = String::new();
    for cert in certs {
        projected.push_str("-----BEGIN CERTIFICATE-----\n");
        let encoded = base64::engine::general_purpose::STANDARD.encode(cert.as_ref());
        for line in encoded.as_bytes().chunks(64) {
            // STANDARD emits ASCII, so these chunks are always valid UTF-8.
            projected.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
            projected.push('\n');
        }
        projected.push_str("-----END CERTIFICATE-----\n");
    }
    match build_ca_client(&projected) {
        Ok(_) => projected,
        Err(_) => String::new(),
    }
}

/// API-only preparation failure: validation is a client error; a worker
/// join failure is an internal server error with no panic/task details.
#[derive(Debug)]
pub(crate) enum TargetPreparationError {
    Invalid(String),
    Worker,
    Unavailable,
}

/// Inspect borrowed installed state and reserve before allocating its snapshot.
pub(crate) async fn redacted_targets(
    webhooks: &Webhooks,
) -> Result<Charged, TargetPreparationError> {
    let (reservation, snapshot) = {
        let targets = webhooks.targets.read().unwrap();
        let reservation =
            ca_executor::reserve(&webhooks.scope, Size::targets(&targets, targets.len()))?;
        #[cfg(test)]
        webhooks.snapshot_clones.fetch_add(1, Ordering::SeqCst);
        (reservation, targets.clone())
    };
    reservation.run(&webhooks.scope, Job::Get(snapshot)).await
}

pub(crate) async fn prepare_targets(
    webhooks: &Webhooks,
    targets: Vec<WebhookTarget>,
) -> Result<Charged, TargetPreparationError> {
    let reservation =
        ca_executor::reserve(&webhooks.scope, Size::targets(&targets, targets.capacity()))?;
    reservation.run(&webhooks.scope, Job::Put(targets)).await
}

/// The header the secret rides in.
const SECRET_HEADER: &str = "X-Swarmail-Secret";

/// The secret rule (decision 0041 §1, superseding 0029 §5's CR/LF/NUL
/// wording): at most 4096 octets, no leading or trailing space or tab (a
/// receiver may trim them, so the secret it compares would not be the one
/// configured), and a value the HTTP client's own header parser accepts.
/// One owner: the PUT validator discards the value, delivery installs it,
/// so a secret that passed PUT always builds its header. No refusal echoes
/// the secret.
pub(crate) fn secret_header(secret: &str) -> Result<reqwest::header::HeaderValue, String> {
    if secret.len() > 4096 {
        return Err("webhook secret must be at most 4096 octets".to_string());
    }
    if secret.starts_with([' ', '\t']) || secret.ends_with([' ', '\t']) {
        return Err("webhook secret must not begin or end with a space or tab".to_string());
    }
    let mut value = reqwest::header::HeaderValue::from_bytes(secret.as_bytes())
        .map_err(|_| SECRET_NOT_A_HEADER.to_string())?;
    value.set_sensitive(true);
    Ok(value)
}

/// The parser refusal, named without the value: U+0001, DEL, CR, LF and
/// NUL all land here.
const SECRET_NOT_A_HEADER: &str =
    "webhook secret must be a valid HTTP header value: no control characters but tab";

/// What a URL that does not parse is shown as: never its own text, which
/// could carry credentials no parser located.
const UNPARSEABLE_URL: &str = "<unparseable webhook URL>";

/// The one display of a webhook URL outside the process (decision 0041
/// §3): echoes and every diagnostic. Userinfo — username and password — is
/// removed; a URL without any keeps its configured spelling. Query and
/// fragment are kept: credentials do not belong there.
pub(crate) fn display_url(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(mut parsed) => {
            if parsed.username().is_empty() && parsed.password().is_none() {
                url.to_string()
            } else {
                // A URL with userinfo has a host, so both setters succeed.
                let _ = parsed.set_username("");
                let _ = parsed.set_password(None);
                parsed.to_string()
            }
        }
        Err(_) => UNPARSEABLE_URL.to_string(),
    }
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
    pub(crate) scope: Scope,
    targets: RwLock<Vec<WebhookTarget>>,
    delivered: AtomicU64,
    dropped: AtomicU64,
    lagged: AtomicU64,
    exhausted: AtomicU64,
    aborted: AtomicU64,
    #[cfg(test)]
    installations: AtomicU64,
    #[cfg(test)]
    snapshot_clones: AtomicU64,
    /// Test-only observation (decision 0020 rule 4): every payload the
    /// dispatcher built, kept alive so a test can compare a request body's
    /// backing allocation against it without address reuse faking identity.
    #[cfg(test)]
    built: Mutex<Vec<Bytes>>,
}

impl Webhooks {
    pub fn set(&self, targets: Vec<WebhookTarget>) {
        #[cfg(test)]
        self.installations.fetch_add(1, Ordering::SeqCst);
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
    /// The stop watch fired: keep consuming, counting — the drain ends the
    /// loop, not the stop (decision 0029 §5).
    Shutdown,
    /// The drain one-shot fired (or its sender dropped): end the loop.
    Drained,
    /// The firehose closed — the store is gone; stop.
    Stop,
}

fn classify(
    res: Result<Arc<crate::model::Email>, tokio::sync::broadcast::error::RecvError>,
) -> Wake {
    match res {
        Ok(email) => Wake::Mail(email),
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => Wake::Lagged(n),
        Err(_) => Wake::Stop, // closed
    }
}

/// The one owner of a lag's effects, live or in the tail sweep (decision
/// 0041 §5): the warn line and the ledger row. Unit: mails — their fan-out
/// is unknowable, and the store stays the truth.
fn record_lag(webhooks: &Webhooks, n: u64) {
    let note =
        format!("webhook dispatcher lagged, skipped {n}; the store remains the source of truth");
    tracing::warn!("{note}");
    webhooks.lagged.fetch_add(n, Ordering::Relaxed);
}

/// How long an over-budget target stays quiet after its warning (decision
/// 0041 §7): every refusal is still counted in `dropped`, but only the first
/// in each window is logged, so a burst against a stalled sink costs one
/// line, not one per mail.
const DROP_WARN_WINDOW: Duration = Duration::from_secs(60);

/// Whether a refusal at `now` warns. The window starts at the last emitted
/// warning; a refusal inside it is silent and does not extend it. Nothing
/// else resets it — a successful admission between refusals leaves it be.
fn should_warn(last: &mut Option<Instant>, now: Instant) -> bool {
    match *last {
        Some(at) if now.saturating_duration_since(at) < DROP_WARN_WINDOW => false,
        _ => {
            *last = Some(now);
            true
        }
    }
}

/// One sink's budget: its in-flight deliveries, plus when it last warned
/// about a refusal. Suppression lives beside the capacity it reports on,
/// so it retires with the budget (decision 0041 §7).
#[derive(Default)]
struct Budget {
    set: tokio::task::JoinSet<()>,
    warned: Option<Instant>,
}

/// The payload's shape: borrowed, so serializing builds no owned copy of
/// the mail first (decision 0041 §4).
#[derive(Serialize)]
struct Envelope<'a> {
    event: &'static str,
    email: &'a crate::model::Email,
}

/// Serialize a mail's payload once, into bytes every target and retry
/// shares: `Bytes::from(Vec)` takes the allocation, it copies nothing.
fn build_payload(email: &crate::model::Email) -> Bytes {
    let envelope = Envelope {
        event: "received",
        email,
    };
    let json = serde_json::to_vec(&envelope).expect("Email serializes");
    Bytes::from(json)
}

/// A delivery budget's key: the target's **parsed, normalized [`reqwest::Url`]** —
/// never a raw string, because a duplicate spelling of one sink must not buy
/// a second budget and re-open the starvation the budget exists to close
/// (decision 0029 §5). A URL that does not parse (a target that never went
/// through the PUT validation) falls back to its raw string: still one key,
/// still one bounded budget of its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SinkKey {
    Url(reqwest::Url),
    Raw(String),
}

fn sink_key(target: &WebhookTarget) -> SinkKey {
    match reqwest::Url::parse(&target.url) {
        Ok(url) => SinkKey::Url(url),
        Err(_) => SinkKey::Raw(target.url.clone()),
    }
}

/// Retire every budget whose target has vanished from the config and whose
/// set has drained (decision 0029 §5): or every PUT/PUT/PUT cycle leaks a
/// dead key. A vanished target with deliveries still in flight keeps its
/// key — its budget is still live, and its outcome still owes a row.
fn retire_dead(
    deliveries: &mut HashMap<SinkKey, Budget>,
    live: &std::collections::HashSet<SinkKey>,
) {
    deliveries.retain(|key, budget| live.contains(key) || !budget.set.is_empty());
}

/// The dispatcher core: consume the firehose until it closes or the drain
/// completes, hold at most [`WEBHOOK_DELIVERY_CAP`] deliveries **per
/// target**, and name every death in the ledger. Owned fn so the wake arms
/// are provable without a live server.
///
/// The shutdown sequencing is decision 0029 §5's: the loop does not end at
/// the stop watch — it keeps consuming post-stop mail, counting each
/// matching fan-out aborted, until the caller's `drained` one-shot (sent
/// after the serve tasks' awaited commits have resolved) or the firehose
/// closes. Only then come the tail sweep and the abort, as today.
pub(crate) async fn dispatch_loop(
    mut rx: tokio::sync::broadcast::Receiver<Arc<crate::model::Email>>,
    webhooks: Arc<Webhooks>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    mut drained: tokio::sync::watch::Receiver<bool>,
) {
    // One budget per sink, keyed by the normalized URL (decision 0029 §5).
    let mut deliveries: HashMap<SinkKey, Budget> = HashMap::new();
    // The stop watch has fired: every later mail is counted, never
    // delivered, until the drain.
    let mut stopping = false;
    loop {
        // Biased: the drain one-shot is polled first, the firehose second,
        // the stop watch last. When the drain fires, the loop ends at once
        // and the tail sweep counts the ring residents it ends on — the
        // counting is the same O(targets) pass either way, and no resident
        // is ever serialized. A mail already in the ring is handed to the
        // `Mail` arm whenever the drain is not the ready arm, so a buffered
        // post-stop mail is counted at the arm check, never delivered.
        let wake = tokio::select! {
            biased;
            _ = drained.changed() => Wake::Drained,
            res = rx.recv() => classify(res),
            _ = shutdown.changed() => Wake::Shutdown,
        };
        match wake {
            Wake::Mail(email) => {
                // First statement: the stop check. Once stop is signaled the
                // payload is never built — counting is O(targets), the build
                // is O(bytes) — and the fan-out is named `aborted`, not
                // delivered: commits that finish during shutdown are counted,
                // never silently lost (decision 0029 §5). A closed watch is a
                // stop too: the error path's drop-cascade rides the same arm.
                if stopping || shutdown.has_changed().unwrap_or(true) {
                    let matching = matching_count(webhooks.current(), &email);
                    webhooks
                        .aborted
                        .fetch_add(matching as u64, Ordering::Relaxed);
                    continue;
                }
                let matching: Vec<WebhookTarget> = webhooks
                    .current()
                    .into_iter()
                    .filter(|t| t.inbox.as_deref().is_none_or(|i| i == email.inbox))
                    .collect();
                if matching.is_empty() {
                    // Nothing is owed for this mail.
                } else {
                    // Harvest finished deliveries first, so the bounds
                    // reflect live capacity and not last pass's dead weight.
                    for budget in deliveries.values_mut() {
                        while budget.set.try_join_next().is_some() {}
                    }
                    // Retire dead-and-drained keys, so config churn cannot
                    // leak a budget per PUT (decision 0029 §5).
                    let live: std::collections::HashSet<SinkKey> =
                        webhooks.current().iter().map(sink_key).collect();
                    retire_dead(&mut deliveries, &live);
                    // The per-target bound is checked before the payload is
                    // built, so a fan-out refused everywhere never allocates
                    // bytes: each sink's slots are its own. `claimed` counts
                    // spawns made *within this pass*, so a mail carrying two
                    // targets spelled alike cannot buy past the budget on a
                    // pre-spawn snapshot.
                    let mut room = Vec::with_capacity(matching.len());
                    let mut claimed: HashMap<SinkKey, usize> = HashMap::new();
                    let now = Instant::now();
                    for target in matching {
                        let key = sink_key(&target);
                        let used = deliveries.get(&key).map(|b| b.set.len()).unwrap_or(0)
                            + claimed.get(&key).copied().unwrap_or(0);
                        if used >= WEBHOOK_DELIVERY_CAP {
                            // Every refusal is counted; the first in each
                            // window is named (#47 AC6.2, decision 0041 §7):
                            // the count alone cannot tell an operator *which*
                            // target is over budget — the warn does, once.
                            webhooks.dropped.fetch_add(1, Ordering::Relaxed);
                            let budget = deliveries.entry(key).or_default();
                            if should_warn(&mut budget.warned, now) {
                                let shown = display_url(&target.url);
                                tracing::warn!(url = %shown, budget = WEBHOOK_DELIVERY_CAP, "webhook target over its delivery budget, refusing until slots free; further refusals within 60 s are counted, not logged");
                            }
                        } else {
                            *claimed.entry(key).or_insert(0) += 1;
                            room.push(target);
                        }
                    }
                    if !room.is_empty() {
                        // The payload is built once per mail, into shared
                        // bytes the deliveries own (decision 0041 §4): each
                        // target and each retry clones a reference, never
                        // the body. A delivery that outlives the mail (a
                        // sink that delays its reply, a retry against a
                        // pruned inbox) replays these bytes; it never
                        // re-reads the store.
                        let payload = build_payload(&email);
                        #[cfg(test)]
                        webhooks.built.lock().unwrap().push(payload.clone());
                        for target in room {
                            let webhooks = Arc::clone(&webhooks);
                            let key = sink_key(&target);
                            let WebhookTarget {
                                url,
                                secret,
                                ca_pem,
                                ..
                            } = target;
                            deliveries.entry(key).or_default().set.spawn(deliver(
                                webhooks,
                                url,
                                secret,
                                ca_pem,
                                payload.clone(),
                            ));
                        }
                    }
                }
            }
            // Unit: mails. Their fan-out is unknowable — the store stays the
            // truth, and the loss is a row, not a warn line alone.
            Wake::Lagged(n) => record_lag(&webhooks, n),
            // The stop watch fired: keep consuming until the drain — post-stop
            // mails are counted, never silently lost (decision 0029 §5).
            Wake::Shutdown => stopping = true,
            Wake::Stop => break, // the firehose closed
            Wake::Drained => break,
        }
    }
    // The tail — the one place the drain arm and a firehose close land.
    // Every death before the loop returns gets a name: the sweep counts the
    // ring residents that never got a pass, then every delivery that ends
    // without reporting its own outcome is counted (an `Ok` already
    // reported, at its last await).
    //
    // The sweep ends only at Empty or Closed (decision 0041 §5): a lag
    // report is not the end of the ring — retained mail follows it — so
    // each lag is recorded (mails, fan-out unknown) and the sweep goes on.
    loop {
        match rx.try_recv() {
            Ok(email) => {
                let matching = matching_count(webhooks.current(), &email);
                webhooks
                    .aborted
                    .fetch_add(matching as u64, Ordering::Relaxed);
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                record_lag(&webhooks, n);
            }
            Err(_) => break, // Empty or Closed: nothing is retained
        }
    }
    deliveries
        .values_mut()
        .for_each(|budget| budget.set.abort_all());
    for budget in deliveries.values_mut() {
        while let Some(res) = budget.set.join_next().await {
            if res.is_err() {
                webhooks.aborted.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    // The final ledger, once, after every join (decision 0041 §6): the
    // shutdown's `aborted` count is produced after HTTP has stopped, so
    // `/metrics` can never show it — this default-level line does.
    let WebhookLedger {
        delivered,
        dropped,
        lagged,
        exhausted,
        aborted,
    } = webhooks.snapshot();
    // The message rides a const: a multiline field list leaves the literal's
    // line mapped to the callsite's static registration, which llvm-cov
    // counts as never executed even when the event fires. It is bound to a
    // local named `message` (decision 0048 §6, issue #135) — the event's
    // message field, where a bare const identifier became a field of its
    // own (`DISPATCHER_STOPPED="…"`) and `message = CONST` left its line
    // reading as never executed: a const value generates no code.
    const DISPATCHER_STOPPED: &str = "webhook dispatcher stopped";
    let message = DISPATCHER_STOPPED;
    tracing::info!(message, delivered, dropped, lagged, exhausted, aborted);
}

/// How many of `targets` fire for `email`.
fn matching_count(targets: Vec<WebhookTarget>, email: &crate::model::Email) -> usize {
    targets
        .into_iter()
        .filter(|t| t.inbox.as_deref().is_none_or(|i| i == email.inbox))
        .count()
}

/// Spawn the dispatcher into the caller's task set: consume the global
/// firehose, deliver with retry, honor `shutdown` and the `drained` one-shot
/// (decision 0029 §5: the dispatcher lives in its own set, joined by
/// [`crate::RunningServer::stop`] *after* the serve tasks — whose awaited
/// commits resolve inside that join — have released it with `drained`).
/// The subscription happens HERE, synchronously, before the spawn — the
/// caller is required to invoke this before any listener binds, so a mail
/// accepted at startup can never miss its webhook (decision 0002, one layer
/// down).
pub fn spawn_dispatcher(
    store: &Store,
    webhooks: &Arc<Webhooks>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    drained: tokio::sync::watch::Receiver<bool>,
    tasks: &mut tokio::task::JoinSet<()>,
) {
    let rx = store.subscribe_all();
    tasks.spawn(dispatch_loop(rx, Arc::clone(webhooks), shutdown, drained));
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
    body: Bytes,
) {
    // The URL as every line below shows it: userinfo removed (decision 0041
    // §3). The delivery itself still dials `url`, credentials and all.
    let shown = display_url(&url);
    // The last delivery error, for the final warn (decision 0029 §5): the
    // cause rides the one default-level line — no new metrics family, no
    // per-attempt spam.
    let mut last = String::from("no attempt completed");
    for attempt in 0..4u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(100 * 4u64.pow(attempt - 1))).await;
        }
        match post_json_scoped(
            &webhooks.scope,
            &url,
            &body,
            secret.as_deref(),
            ca_pem.as_deref(),
        )
        .await
        {
            Ok(status) if (200..300).contains(&status) => {
                webhooks.delivered.fetch_add(1, Ordering::Relaxed);
                return;
            }
            Ok(status) => {
                last = format!("status {status}");
                tracing::debug!(url = %shown, status, "webhook non-2xx");
            }
            Err(e) => {
                last = e;
                tracing::debug!(url = %shown, error = %last, "webhook delivery failed");
            }
        }
    }
    webhooks.exhausted.fetch_add(1, Ordering::Relaxed);
    tracing::warn!(url = %shown, error = %last, "webhook dropped after retries");
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

/// The one delivery policy every client derives from (decision 0041 §2):
/// rustls only — no native-tls in the tree — redirects never followed, and
/// no proxy. reqwest would otherwise read `HTTP_PROXY`/`HTTPS_PROXY`/
/// `ALL_PROXY` from the environment and hand every payload and secret to
/// a proxy nobody configured for webhooks; a delivery dials the configured
/// address and nothing else.
fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(redirect_policy())
        .no_proxy()
}

/// The client for targets that trust the built-in root store: rustls with
/// the webpki roots, certificate verification on, built once per process.
fn shared_client() -> reqwest::Client {
    CLIENT
        .get_or_init(|| {
            client_builder()
                .build()
                .expect("the rustls webhook client must build")
        })
        .clone()
}

/// Pick the client for a target: the shared rustls client, or one pinned to
/// the target's own root CA, cached by PEM so retries reuse it. The cache is
/// bounded at [`CA_CLIENT_CAP`] with least-recently-used eviction: a cold
/// PEM beyond the cap retires the oldest build, and an evicted PEM's next
/// delivery rebuilds under the ordinary CA admission/retry policy. The
/// built-in-roots client stays outside the cache and is built once.
/// One owner for the ca_pem failure: parse and build surface the same
/// text, so whichever stage refuses the PEM, the delivery error is alike.
fn ca_pem_error(e: impl std::fmt::Display) -> String {
    format!("webhook ca_pem could not be loaded into the rustls client: {e}")
}

/// How many source causes an error's display carries past its own text
/// (decision 0048 §6): enough for reqwest → hyper → connector → io, and a
/// fixed bound so a cyclic or pathological chain cannot grow a log line.
const CAUSE_DEPTH: usize = 8;

/// An error's display with its source causes appended, `: `-separated, at
/// most [`CAUSE_DEPTH`] of them (issue #135): reqwest's own text stops at
/// `error sending request` or `builder error`, which leaves an operator
/// unable to tell why. A reqwest error reaches here only after
/// `without_url`, so no cause can carry the target's userinfo.
pub(crate) fn with_causes(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    for _ in 0..CAUSE_DEPTH {
        let Some(cause) = source else {
            break;
        };
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[cfg(test)]
static CACHE_TESTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// One-shot injection belongs to one generated raw PEM, so unrelated API
// requests cannot consume it. The guard clears only its own still-armed key.
#[cfg(test)]
static API_PREPARATION_PANIC: Mutex<Option<String>> = Mutex::new(None);

#[cfg(test)]
struct ApiPreparationPanic {
    pem: String,
}

#[cfg(test)]
impl Drop for ApiPreparationPanic {
    fn drop(&mut self) {
        let mut armed = API_PREPARATION_PANIC.lock().unwrap();
        if armed.as_deref() == Some(self.pem.as_str()) {
            *armed = None;
        }
    }
}

#[cfg(test)]
fn panic_next_api_preparation_for_test(pem: &str) -> ApiPreparationPanic {
    let mut armed = API_PREPARATION_PANIC.lock().unwrap();
    assert!(armed.is_none(), "one API preparation panic at a time");
    *armed = Some(pem.to_owned());
    ApiPreparationPanic {
        pem: pem.to_owned(),
    }
}

#[cfg(test)]
fn panic_api_preparation_for_test(targets: &[WebhookTarget]) {
    let should_panic = {
        let mut armed = API_PREPARATION_PANIC.lock().unwrap();
        if armed.as_ref().is_some_and(|pem| {
            targets
                .iter()
                .any(|target| target.ca_pem.as_ref() == Some(pem))
        }) {
            *armed = None;
            true
        } else {
            false
        }
    };
    if should_panic {
        panic!("injected webhook API preparation panic");
    }
}

#[cfg(test)]
struct CaBuildPause {
    pem: String,
    entered: tokio::sync::oneshot::Sender<std::thread::ThreadId>,
    release: std::sync::mpsc::Receiver<()>,
    finished: tokio::sync::oneshot::Sender<()>,
}

#[cfg(test)]
static CA_BUILD_PAUSE: Mutex<VecDeque<CaBuildPause>> = Mutex::new(VecDeque::new());

#[cfg(test)]
static CA_BUILD_PANIC: Mutex<Option<String>> = Mutex::new(None);

#[cfg(test)]
fn panic_ca_build_for_test(pem: &str) {
    let should_panic = {
        let mut panic = CA_BUILD_PANIC.lock().unwrap();
        if panic.as_deref() == Some(pem) {
            *panic = None;
            true
        } else {
            false
        }
    };
    if should_panic {
        panic!("injected webhook CA construction panic");
    }
}

#[cfg(test)]
fn install_ca_build_panic(pem: &str) {
    let mut panic = CA_BUILD_PANIC.lock().unwrap();
    assert!(panic.is_none(), "only one CA build panic may be installed");
    *panic = Some(pem.to_string());
}

#[cfg(test)]
fn pause_ca_build_for_test(pem: &str) -> Option<tokio::sync::oneshot::Sender<()>> {
    let pause = {
        let mut hook = CA_BUILD_PAUSE.lock().unwrap();
        hook.iter()
            .position(|pause| pause.pem == pem)
            .and_then(|position| hook.remove(position))
    };
    if let Some(pause) = pause {
        let _ = pause.entered.send(std::thread::current().id());
        pause
            .release
            .recv()
            .expect("the owner test releases the held CA construction");
        Some(pause.finished)
    } else {
        None
    }
}

#[cfg(test)]
fn install_ca_build_pause(
    pem: &str,
) -> (
    tokio::sync::oneshot::Receiver<std::thread::ThreadId>,
    std::sync::mpsc::Sender<()>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
    let mut hook = CA_BUILD_PAUSE.lock().unwrap();
    hook.push_back(CaBuildPause {
        pem: pem.to_string(),
        entered: entered_tx,
        release: release_rx,
        finished: finished_tx,
    });
    (entered_rx, release_tx, finished_rx)
}

/// One owner for the PEM rule (decisions 0029 §5, 0048 §1): the PUT
/// validator and the delivery cache miss both build through here, so a PEM
/// the validator accepted is exactly a PEM delivery can build — the two
/// arms cannot drift. Every certificate in the PEM is read (a bundle's
/// trailing root is trusted, a trailing bad block refuses), at least one is
/// demanded — `reqwest::Certificate::from_pem` is lazy and accepts bytes
/// with no PEM in them at all — and the client build is completed, because
/// under rustls the DER is only loaded into the root store there (issue
/// #122: a well-framed PEM of junk DER passed every earlier check). Unrelated
/// PEM sections are skipped by the parser, as before. Uncached and offline:
/// the caller owns the cache.
pub(crate) fn build_ca_client(pem: &str) -> Result<reqwest::Client, String> {
    #[cfg(test)]
    let finished = pause_ca_build_for_test(pem);
    let result = build_ca_client_inner(pem);
    #[cfg(test)]
    if let Some(finished) = finished {
        let _ = finished.send(());
    }
    result
}

fn build_ca_client_inner(pem: &str) -> Result<reqwest::Client, String> {
    let mut builder = client_builder();
    let mut roots = 0usize;
    for der in rustls_pemfile::certs(&mut pem.as_bytes()) {
        let der = der.map_err(ca_pem_error)?;
        let root = reqwest::Certificate::from_der(der.as_ref()).map_err(ca_pem_error)?;
        builder = builder.add_root_certificate(root);
        roots += 1;
    }
    if roots == 0 {
        return Err(ca_pem_error("no PEM certificate section was found"));
    }
    // The same policy as the shared client — no redirect, no proxy — plus
    // these roots.
    builder.build().map_err(|e| ca_pem_error(with_causes(&e)))
}

/// The scheme refusal, fixed text: the scheme is the caller's input.
const UNSUPPORTED_SCHEME: &str =
    "unsupported webhook URL scheme: only http and https are delivered";

/// One owner for the URL rule (decision 0029 §5): the PUT validator and the
/// delivery path both call it, so a target is refused where it is authored
/// and the delivery arm stays warm. The URL must parse with scheme
/// `http`/`https` and name a host.
pub(crate) fn parse_target_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("invalid webhook URL: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        // The host clause is what catches the scheme-less spelling:
        // `Url::parse` reads `localhost:9000` as *scheme* `localhost` with
        // no host at all — the misconfig that used to surface only at
        // delivery. Named for the defect it is.
        // Neither refusal reflects the input (decision 0041 §3): a URL
        // that does not parse right could carry credentials anywhere.
        _ if parsed.host_str().is_none() => {
            return Err("webhook URL must name a host".to_string());
        }
        _ => return Err(UNSUPPORTED_SCHEME.to_string()),
    }
    // For `http`/`https` the parser itself refuses an empty host, so a URL
    // that reaches here always names one; the rule has one owner.
    Ok(parsed)
}

#[cfg(test)]
async fn client_for(ca_pem: Option<&str>) -> Result<reqwest::Client, String> {
    client_for_scope(&Scope::default(), ca_pem).await
}

async fn client_for_scope(scope: &Scope, ca_pem: Option<&str>) -> Result<reqwest::Client, String> {
    let Some(pem) = ca_pem else {
        return Ok(shared_client());
    };
    {
        let mut cache = ca_clients().lock().unwrap();
        if let Some(client) = cache.clients.get(pem).cloned() {
            touch_lru(&mut cache.lru, pem);
            return Ok(client);
        }
    }
    let reservation = ca_executor::reserve(scope, Some(Size::delivery(pem))).map_err(|error| {
        #[cfg(test)]
        ca_construction_tests::observe_ca_refusal(pem);
        delivery_ca_error(error)
    })?;
    let charged = reservation
        .run(scope, Job::Delivery(pem.to_string()))
        .await
        .map_err(delivery_ca_error)?;
    let Charged { value, reservation } = charged;
    let (key, client) = match value {
        Ok(Output::Delivery(key, client)) => (key, client),
        value => {
            return Err(delivery_ca_error(
                value.err().unwrap_or(TargetPreparationError::Worker),
            ));
        }
    };
    let mut displaced = None;
    let published = {
        let _live = scope
            .live()
            .ok_or_else(|| ca_executor::UNAVAILABLE.to_string())?;
        let mut cache = ca_clients().lock().unwrap();
        if let Some(winner) = cache.clients.get(&key).cloned() {
            touch_lru(&mut cache.lru, &key);
            winner
        } else {
            if cache.clients.len() >= CA_CLIENT_CAP {
                displaced = Some(evict_lru(&mut cache));
            }
            cache.clients.insert(key.clone(), client.clone());
            cache.lru.push_back(key.clone());
            client.clone()
        }
    };
    drop(displaced);
    drop(client);
    drop(key);
    drop(reservation);
    Ok(published)
}

fn delivery_ca_error(error: TargetPreparationError) -> String {
    match error {
        TargetPreparationError::Invalid(error) => error,
        TargetPreparationError::Unavailable => ca_executor::UNAVAILABLE.to_string(),
        TargetPreparationError::Worker => "webhook CA client construction task failed".to_string(),
    }
}

/// One attempt's request, built and returned rather than sent, so the body
/// a test inspects is the very request `post_json` executes. The body is a
/// clone of the shared payload — a reference count, not a copy (decision
/// 0041 §4) — and the secret header comes from [`secret_header`], the PUT
/// validator's own rule.
fn build_request(
    client: &reqwest::Client,
    url: reqwest::Url,
    body: &Bytes,
    secret: Option<&str>,
) -> Result<reqwest::Request, String> {
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .timeout(POST_TIMEOUT)
        .body(body.clone());
    if let Some(secret) = secret {
        request = request.header(SECRET_HEADER, secret_header(secret)?);
    }
    // Every part above is already valid — a parsed URL, a static content
    // type, a parsed header value — so the build cannot refuse.
    Ok(request
        .build()
        .expect("a request of already-valid parts always builds"))
}

// Test-only observer (decision 0020 rule 4, issue #143): every request
// `post_json` constructed on this thread, captured after construction and
// before it is sent. It records and never shapes — the request sent is the
// one built — so a sharing proof sees the body `post_json` really executes,
// and a copy inserted anywhere on its path cannot hide behind a direct
// `build_request` call. `try_clone` of a bytes body clones the reference,
// never the bytes.
#[cfg(test)]
thread_local! {
    static POSTED: core::cell::RefCell<Vec<reqwest::Request>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
async fn post_json(
    url: &str,
    body: &Bytes,
    secret: Option<&str>,
    ca_pem: Option<&str>,
) -> Result<u16, String> {
    post_json_scoped(&Scope::default(), url, body, secret, ca_pem).await
}

async fn post_json_scoped(
    scope: &Scope,
    url: &str,
    body: &Bytes,
    secret: Option<&str>,
    ca_pem: Option<&str>,
) -> Result<u16, String> {
    let url = parse_target_url(url)?;
    let client = client_for_scope(scope, ca_pem).await?;
    let request = build_request(&client, url, body, secret)?;
    #[cfg(test)]
    POSTED.with(|posted| {
        posted
            .borrow_mut()
            .push(request.try_clone().expect("a bytes body always clones"))
    });
    // Authorize and poll the next send under the live publication guard.
    let mut send = std::pin::pin!(execute_request(&client, request));
    let first = std::future::poll_fn(|cx| {
        use std::future::Future;
        let Some(_live) = scope.live() else {
            return std::task::Poll::Ready(std::task::Poll::Ready(Err(
                ca_executor::UNAVAILABLE.to_string()
            )));
        };
        std::task::Poll::Ready(send.as_mut().poll(cx))
    })
    .await;
    match first {
        std::task::Poll::Ready(result) => result,
        std::task::Poll::Pending => send.await,
    }
}

/// Execute and format through one credential-redacting boundary, including
/// DNS and TLS failures from the real connector.
async fn execute_request(
    client: &reqwest::Client,
    request: reqwest::Request,
) -> Result<u16, String> {
    // reqwest's error text ends "for url (…)"; the caller logs the redacted
    // URL beside it, so the error's own copy goes (decision 0041 §3) —
    // before the causes are read, which name the failure, not the target
    // (decision 0048 §6).
    let response = client
        .execute(request)
        .await
        .map_err(|e| with_causes(&e.without_url()))?;
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
            envelope_from: None,
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

    pub(super) fn email() -> Email {
        Email {
            id: "1".into(),
            inbox: "default".into(),
            from: None,
            envelope_from: None,
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
        // The drain one-shot is never fired: this loop ends on the close.
        let (_drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        // Both sends happen BEFORE the loop exists: the first recv is then
        // guaranteed to lag (capacity 1 holds one mail, one was lost), so
        // the Lagged arm is deterministic — no race with the loop's speed.
        tx.send(Arc::new(email())).unwrap(); // lost: overwritten in the ring
        tx.send(Arc::new(email())).unwrap(); // buffered: the next recv after the lag
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));
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
    /// runtime the loop's first poll sees all arms ready and the biased
    /// select hands it the drain: the loop ends at once and the tail sweep
    /// counts both ring residents it ends on. The sink records nothing —
    /// no payload was ever built (decision 0029 §5).
    #[tokio::test]
    async fn a_stop_signaled_backlog_is_counted_and_never_serialized() {
        let (addr, hits, _retire, _sink) = counting_sink(None).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![target(format!("http://{addr}/hook"))]);
        let (tx, rx) = broadcast::channel(8);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

        let _ = shutdown_tx.send(true);
        tx.send(Arc::new(email())).unwrap();
        tx.send(Arc::new(email())).unwrap();
        // The drain releases the loop only after the stop-signaled mail is
        // counted — the shape of `RunningServer::stop`, sequenced.
        let _ = drained_tx.send(true);

        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("stop must end the loop")
            .unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(
            ledger.aborted, 2,
            "both ring residents are swept by the drain the loop ends on"
        );
        assert_eq!(ledger.delivered, 0, "nothing was delivered");
        assert_eq!(ledger.dropped, 0, "a shutdown loss is not a bound refusal");
        assert_eq!(
            hits.load(Ordering::Relaxed),
            0,
            "the sink records nothing: no payload was ever built"
        );
        assert!(
            webhooks.built.lock().unwrap().is_empty(),
            "a stopping mail builds no payload"
        );
    }

    /// Every death the shutdown counts, in one ledger (decision 0029 §5):
    /// a mail consumed while the stop watch is armed is counted at the arm
    /// check — both when the watch's change is first seen there and when
    /// the loop is already `stopping` — and the ring resident the drain
    /// itself ends on is counted by the tail sweep. Each wait below is on
    /// the ledger row the loop itself writes, never a sleep. The sink
    /// records nothing at any point.
    #[tokio::test]
    async fn every_shutdown_counting_arm_names_its_mail_in_one_ledger() {
        let (addr, hits, _retire, _sink) = counting_sink(None).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![target(format!("http://{addr}/hook"))]);
        let (tx, rx) = broadcast::channel(8);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

        // Stop and a mail land before the loop's first poll: the firehose
        // wins the biased poll, and the arm check sees the unsignaled-then-
        // changed watch (`has_changed` is true even though `stopping` is
        // not yet set).
        let _ = shutdown_tx.send(true);
        tx.send(Arc::new(email())).unwrap();
        spin_until(2000, || webhooks.snapshot().aborted == 1).await;

        // With the ring empty the loop takes the `Shutdown` wake — now a
        // later mail is counted through `stopping` alone.
        tokio::task::yield_now().await;
        tx.send(Arc::new(email())).unwrap();
        spin_until(2000, || webhooks.snapshot().aborted == 2).await;

        // The drain ends the loop over a resident: the tail sweep counts it.
        tx.send(Arc::new(email())).unwrap();
        let _ = drained_tx.send(true);
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the drain must end the loop")
            .unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(
            ledger.aborted, 3,
            "one per arm check, one swept at the drain"
        );
        assert_eq!(ledger.delivered, 0, "nothing was delivered");
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
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

        tx.send(Arc::new(email())).unwrap();
        // The report is the delivery task's final statements — after its
        // last await — so `delivered == 1` proves the task is complete and
        // sits unreaped in the set: exactly the `Ok` arm the drain must see.
        spin_until(5000, || webhooks.snapshot().delivered == 1).await;
        let _ = shutdown_tx.send(true);
        // The drain: the dispatcher has kept consuming (there is nothing
        // more), and now ends — sweeping and aborting what is left.
        let _ = drained_tx.send(true);

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
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

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
        let _ = drained_tx.send(true); // the drain ends the loop (0029 §5)
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
        let (drain_a, rx_drain_a) = tokio::sync::watch::channel(false);
        let loop_a = tokio::spawn(dispatch_loop(rx_a, Arc::clone(&a), rx_stop_a, rx_drain_a));
        let _ = stop_a.send(true);
        tx_a.send(Arc::new(email())).unwrap();
        // The drain ends the loop after the stop-signaled mail is counted.
        let _ = drain_a.send(true);
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
        let (drain_b, rx_drain_b) = tokio::sync::watch::channel(false);
        let loop_b = tokio::spawn(dispatch_loop(rx_b, Arc::clone(&b), rx_stop_b, rx_drain_b));
        let _ = stop_b.send(true);
        tx_b.send(Arc::new(email())).unwrap();
        tx_b.send(Arc::new(email())).unwrap();
        let _ = drain_b.send(true);
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
        let (_drain_tx, drained_rx) = tokio::sync::watch::channel(false);
        spawn_dispatcher(&store, &webhooks, shutdown_rx, drained_rx, &mut tasks);
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
    pub(super) const FAST_REPLY: &str =
        "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    pub(super) fn target(url: String) -> WebhookTarget {
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
    pub(super) async fn spin_until(deadline_ms: u64, fact: impl Fn() -> bool) {
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
    pub(super) async fn counting_sink(
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
    pub(super) struct Capture(pub(super) Arc<std::sync::Mutex<Vec<u8>>>);

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

    pub(super) fn captured_log() -> &'static Capture {
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

    /// The shared client for `ca_pem: None` stays outside the cache; a cold
    /// PEM builds once; the same PEM again is a hit that rebuilds nothing.
    #[tokio::test]
    async fn clients_are_cached_per_ca() {
        let _g = CACHE_TESTS.lock().await;
        let pem = ca_pem();
        let before = ca_cache_stats();

        client_for(Some(&pem)).await.unwrap();
        let built = ca_cache_stats();
        assert_eq!(built.builds, before.builds + 1, "a cold PEM builds once");
        assert_eq!(
            built.entries,
            (before.entries + 1).min(CA_CLIENT_CAP),
            "the PEM is cached, within the bound (an insert at the cap evicts, not grows)"
        );

        client_for(Some(&pem)).await.unwrap();
        let hit = ca_cache_stats();
        assert_eq!(
            hit.builds, built.builds,
            "the same PEM must reuse the client, not rebuild"
        );
        assert_eq!(
            hit.entries, built.entries,
            "a hit does not change the cache size"
        );

        client_for(None).await.unwrap();
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
    #[tokio::test]
    async fn cache_stays_within_the_cap_and_counts_evictions() {
        let _g = CACHE_TESTS.lock().await;
        let before = ca_cache_stats();

        // More distinct cold roots than the cache can hold, all built once.
        for _ in 0..CA_CLIENT_CAP + 8 {
            client_for(Some(&ca_pem())).await.unwrap();
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
    #[tokio::test]
    async fn eviction_is_least_recently_used_and_the_evicted_pem_rebuilds() {
        let _g = CACHE_TESTS.lock().await;

        // Fill the cache exactly, on top of whatever earlier tests left:
        // cap fresh roots always end at the cap, whoever came before.
        for _ in 0..CA_CLIENT_CAP {
            client_for(Some(&ca_pem())).await.unwrap();
        }

        let touched = ca_pem();
        let older = ca_pem();
        client_for(Some(&touched)).await.unwrap();
        client_for(Some(&older)).await.unwrap();
        client_for(Some(&touched)).await.unwrap(); // touched is now MRU; older is not

        // cap-1 further cold builds: each retires the current LRU, working
        // forward to `older`. `touched`, being more recent, is still cached.
        for _ in 0..CA_CLIENT_CAP - 1 {
            client_for(Some(&ca_pem())).await.unwrap();
        }

        let builds = ca_cache_stats().builds;
        client_for(Some(&touched)).await.unwrap();
        assert_eq!(
            ca_cache_stats().builds,
            builds,
            "the recently-used root must survive the wave: a hit, no rebuild"
        );
        client_for(Some(&older)).await.unwrap();
        assert_eq!(
            ca_cache_stats().builds,
            builds + 1,
            "the untouched older root must have been evicted, and its next use rebuilds"
        );
    }

    /// A URL that does not parse at all is a delivery error, not a panic.
    #[tokio::test]
    async fn unparseable_url_is_an_error() {
        let body = Bytes::from_static(b"{}");
        let err = post_json("not a url", &body, None, None).await.unwrap_err();
        assert!(err.contains("invalid webhook URL"), "{err}");
    }

    /// Schemes other than http/https are rejected before any connection,
    /// by a fixed sentence that does not reflect the input.
    #[tokio::test]
    async fn unsupported_scheme_is_an_error() {
        let body = Bytes::from_static(b"{}");
        let err = post_json("ftp://user:hunter2@hooks.example/x", &body, None, None)
            .await
            .unwrap_err();
        assert_eq!(err, UNSUPPORTED_SCHEME);
    }

    /// A secret installed without the PUT validator (a direct `set`) is
    /// refused by the delivery's own header rule before anything is
    /// dialed — the same rule, the same named error, no secret echoed.
    #[tokio::test]
    async fn a_directly_installed_bad_secret_is_refused_before_dialing() {
        let body = Bytes::from_static(b"{}");
        let err = post_json(
            "http://127.0.0.1:9/hook",
            &body,
            Some("bad\u{1}secret"),
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(err, SECRET_NOT_A_HEADER);
        assert!(!err.contains("bad"), "the refusal never echoes the secret");
    }

    /// The request a delivery executes carries the shared payload itself:
    /// its body is the very allocation the dispatcher built, not a copy
    /// (decision 0041 §4). Observed where it matters (issue #143): each
    /// request is the one `post_json` itself constructed, captured by the
    /// `POSTED` observer after construction and before the send, while two
    /// real attempts are delivered to a loopback sink — so a copy inserted
    /// anywhere in `post_json`'s path fails here, which a direct
    /// `build_request` call could never see. The source is kept alive, so a
    /// matching address cannot be allocator reuse. The accepted secret
    /// rides as a sensitive header with its exact bytes.
    #[tokio::test]
    async fn a_built_request_shares_the_payload_allocation() {
        let payload = build_payload(&loop_tests::email());
        let (addr, hits, retire, sink) =
            loop_tests::counting_sink(Some(loop_tests::FAST_REPLY)).await;
        let url = format!("http://user:pw@{addr}/hook");
        POSTED.with(|posted| posted.borrow_mut().clear());
        assert_eq!(
            post_json(&url, &payload, Some("s3\tcret é"), None).await,
            Ok(204)
        );
        assert_eq!(post_json(&url, &payload, None, None).await, Ok(204));
        assert_eq!(hits.load(Ordering::Relaxed), 2, "both attempts were sent");
        drop(retire);
        sink.await.unwrap();
        let posted = POSTED.with(|posted| std::mem::take(&mut *posted.borrow_mut()));
        assert_eq!(posted.len(), 2, "one observed request per attempt");
        let (first, second) = (&posted[0], &posted[1]);
        assert_eq!(
            first.url().as_str(),
            format!("http://{addr}/hook"),
            "the request dials the target (reqwest moves the userinfo to a header)"
        );
        for request in [first, second] {
            let body = request.body().unwrap().as_bytes().unwrap();
            assert_eq!(
                body.as_ptr(),
                payload.as_ptr(),
                "the body is the shared allocation"
            );
            assert_eq!(body.len(), payload.len());
        }
        let header = first.headers().get(SECRET_HEADER).unwrap();
        assert_eq!(header.as_bytes(), "s3\tcret é".as_bytes());
        assert!(
            header.is_sensitive(),
            "the secret header is marked sensitive"
        );
        assert!(second.headers().get(SECRET_HEADER).is_none());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&payload).unwrap(),
            serde_json::json!({ "event": "received", "email": loop_tests::email() }),
            "the shared bytes are the documented envelope"
        );
    }

    /// A reqwest transport error loses its own URL copy (decision 0041 §3):
    /// the text that reaches a log line names the failure, never the URL —
    /// the caller logs the redacted URL beside it.
    #[tokio::test]
    async fn a_transport_error_carries_no_url() {
        // Bind and drop: a port nothing listens on, so the dial is refused.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let body = Bytes::from_static(b"{}");
        let url = format!("http://user:hunter2@127.0.0.1:{port}/hook?k=v");
        let err = post_json(&url, &body, None, None).await.unwrap_err();
        // The display prefix is kept and the transport cause follows it
        // (decision 0048 §6, issue #135): bare `error sending request` told
        // an operator nothing.
        assert!(err.starts_with("error sending request: "), "{err}");
        assert!(!err.contains("127.0.0.1"), "no URL in the error: {err}");
        assert!(
            !err.contains("hunter2"),
            "no credential in the error: {err}"
        );
    }

    /// A resolver owned by one real reqwest client: no system DNS or
    /// environment mutation, and a recognizable useful connector cause.
    #[derive(Debug)]
    struct RefuseDns(Arc<std::sync::atomic::AtomicUsize>);

    impl reqwest::dns::Resolve for RefuseDns {
        fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(std::io::Error::other("fixture DNS resolution refused").into()) })
        }
    }

    fn credential_request(client: &reqwest::Client, url: &str) -> reqwest::Request {
        let original = reqwest::Url::parse(url).unwrap();
        let mut request =
            build_request(client, original.clone(), &Bytes::from_static(b"{}"), None).unwrap();
        // The builder transfers Basic auth to a header. Also retain the
        // original metadata here so omitting without_url is a live leak,
        // rather than a vacuous mutant of an already credential-free URL.
        *request.url_mut() = original;
        request
    }

    fn assert_no_fixture_userinfo(error: &str) {
        for sensitive in [
            "UserSentinel",
            "PassSentinel",
            "User%53entinel",
            "Pass%53entinel",
            "@",
        ] {
            assert!(!error.contains(sensitive), "userinfo leaked: {error}");
        }
        assert!(error.starts_with("error sending request: "), "{error}");
    }

    #[tokio::test]
    async fn dns_failure_causes_keep_no_userinfo() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let client = client_builder()
            .dns_resolver(Arc::new(RefuseDns(calls.clone())))
            .build()
            .unwrap();
        let request = credential_request(
            &client,
            "http://User%53entinel:Pass%53entinel@fixture.invalid/hook",
        );
        let error = execute_request(&client, request).await.unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(error.contains("dns error"), "{error}");
        assert!(error.contains("fixture DNS resolution refused"), "{error}");
        assert_no_fixture_userinfo(&error);
    }

    #[tokio::test]
    async fn tls_verification_failure_causes_keep_no_userinfo() {
        use chrono::Datelike;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
        let today = chrono::Utc::now().date_naive();
        let before = today.pred_opt().unwrap();
        let after = today.succ_opt().unwrap();
        let key = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::new(vec!["localhost".into()]).unwrap();
        params.not_before =
            rcgen::date_time_ymd(before.year(), before.month() as u8, before.day() as u8);
        params.not_after =
            rcgen::date_time_ymd(after.year(), after.month() as u8, after.day() as u8);
        let cert = params.self_signed(&key).unwrap();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(cert.der().to_vec())],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
            )
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let sink = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            assert!(
                acceptor.accept(stream).await.is_err(),
                "untrusted handshake must fail"
            );
        });
        let client = client_builder().build().unwrap();
        let request = credential_request(
            &client,
            &format!("https://User%53entinel:Pass%53entinel@{addr}/hook"),
        );
        let error = execute_request(&client, request).await.unwrap_err();
        sink.await.unwrap();
        assert!(error.contains("invalid peer certificate"), "{error}");
        assert!(error.contains("UnknownIssuer"), "{error}");
        assert_no_fixture_userinfo(&error);
    }

    /// A ca_pem that is not a PEM certificate fails before connecting.
    #[tokio::test]
    async fn bad_ca_pem_is_an_error() {
        let _serial = CACHE_TESTS.lock().await;
        let err = client_for(Some("-----BEGIN CERTIFICATE-----\nnope"))
            .await
            .unwrap_err();
        assert!(err.contains("ca_pem"), "{err}");
    }

    /// A PEM block whose base64 carries `bytes`: well-framed, whatever the
    /// DER inside.
    fn framed(bytes: &[u8]) -> String {
        use base64::Engine as _;
        let body = base64::engine::general_purpose::STANDARD.encode(bytes);
        format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
    }

    /// Issue #122, decision 0048 §1: every certificate in the PEM is loaded
    /// into the delivery client's own root store and the build completed,
    /// so a well-framed block of junk DER refuses — before, only the first
    /// block was decoded and `from_der` merely wrapped the bytes, so PUT
    /// answered 200 and every delivery failed building its client. A
    /// trailing bad or malformed block refuses after a good one; a bundle
    /// of good roots, and an unrelated PEM section beside one, build. Each
    /// refusal is the one named `ca_pem` sentence.
    #[test]
    fn every_root_is_loaded_through_the_delivery_build() {
        let good = ca_pem();
        let refused = [
            ("no section", "not a pem at all".to_string()),
            ("empty block", framed(b"")),
            (
                "malformed base64",
                "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n".to_string(),
            ),
            ("junk DER", framed(b"not a DER certificate")),
            ("good then junk DER", format!("{good}{}", framed(b"junk"))),
            (
                "good then malformed block",
                format!("{good}-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n"),
            ),
        ];
        for (case, pem) in refused {
            let err = build_ca_client(&pem).expect_err(case);
            assert!(
                err.starts_with("webhook ca_pem could not be loaded into the rustls client: "),
                "{case}: {err}"
            );
        }
        let junk = build_ca_client(&framed(b"not a DER certificate")).unwrap_err();
        assert!(
            junk.len() > ca_pem_error("builder error").len(),
            "the build refusal names its cause past `builder error`: {junk}"
        );
        for (case, pem) in [
            ("one root", good.clone()),
            ("two roots", format!("{good}{}", ca_pem())),
            (
                "an unrelated section first",
                format!("-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n{good}"),
            ),
        ] {
            assert!(build_ca_client(&pem).is_ok(), "{case}");
        }
    }

    /// PUT validation builds and drops (decision 0048 §1): it never takes a
    /// cache entry, moves a counter or touches recency. With the cache full,
    /// validating its least-recently-used PEM leaves that PEM the LRU — the
    /// next cold root evicts it, and its next delivery rebuilds — and a
    /// flood of validations beyond the cap changes no stat at all.
    #[tokio::test]
    async fn validation_never_touches_the_client_cache() {
        let _g = CACHE_TESTS.lock().await;
        let oldest = ca_pem();
        client_for(Some(&oldest)).await.unwrap();
        for _ in 0..CA_CLIENT_CAP - 1 {
            client_for(Some(&ca_pem())).await.unwrap();
        }
        let before = ca_cache_stats();
        let target = |pem: &str| WebhookTarget {
            url: "http://127.0.0.1:9/hook".into(),
            inbox: None,
            secret: None,
            ca_pem: Some(pem.into()),
        };
        assert_eq!(target(&oldest).validate(), Ok(()));
        for _ in 0..CA_CLIENT_CAP + 1 {
            assert_eq!(target(&ca_pem()).validate(), Ok(()));
        }
        assert_eq!(ca_cache_stats(), before, "validation moved no stat");

        client_for(Some(&ca_pem())).await.unwrap(); // one cold root: evicts the LRU
        let builds = ca_cache_stats().builds;
        client_for(Some(&oldest)).await.unwrap();
        assert_eq!(
            ca_cache_stats().builds,
            builds + 1,
            "the validated PEM stayed least recently used, so it was the one evicted"
        );
    }

    /// One link of a test error chain: `link n`, sourced by `link n-1`.
    #[derive(Debug)]
    struct Link(usize, Option<Box<Link>>);

    impl Link {
        fn chain(depth: usize) -> Link {
            (1..=depth).fold(Link(0, None), |inner, n| Link(n, Some(Box::new(inner))))
        }
    }

    impl std::fmt::Display for Link {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "link {}", self.0)
        }
    }

    impl std::error::Error for Link {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.1.as_deref().map(|l| l as _)
        }
    }

    /// The cause formatter (decision 0048 §6): a sourceless error is its own
    /// text, a chain is `: `-joined in order, and a chain deeper than the
    /// bound stops after [`CAUSE_DEPTH`] causes.
    #[test]
    fn causes_are_appended_in_order_and_bounded() {
        assert_eq!(with_causes(&Link::chain(0)), "link 0");
        assert_eq!(
            with_causes(&Link::chain(3)),
            "link 3: link 2: link 1: link 0"
        );
        let deep = with_causes(&Link::chain(20));
        assert_eq!(deep.split(": ").count(), 1 + CAUSE_DEPTH, "{deep}");
        assert!(deep.ends_with("link 12"), "{deep}");
    }
}

#[cfg(test)]
mod validation_tests {
    use super::loop_tests::{FAST_REPLY, captured_log, counting_sink, spin_until, target};
    use super::*;
    use crate::model::Email;
    use tokio::sync::broadcast;

    fn email() -> Email {
        Email {
            id: "1".into(),
            inbox: "default".into(),
            from: None,
            envelope_from: None,
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

    // ---------- decision 0048 §2: the closed, value-free request shape ----------

    /// The PUT body's reader (issue #135): an array of closed objects whose
    /// strings are present or omitted, never `null`. Every JSON kind meets
    /// every shape — the array, its element, a string field — and each
    /// mismatch is refused by its kind alone; a value never rides into the
    /// error (a URL's credential would). Unknown, duplicate and missing keys
    /// keep serde's own wording, which names keys only.
    #[test]
    fn the_target_shape_is_closed_and_refuses_by_kind_never_by_value() {
        let parse = |body: &str| serde_json::from_str::<WebhookTargets>(body).map(|t| t.0);
        let kinds = [
            ("true", "a boolean"),
            ("-7", "a number"),
            ("12345", "a number"),
            ("1.5", "a number"),
            (r#""hunter2""#, "a string"),
            ("null", "null"),
            (r#"["hunter2"]"#, "an array"),
            (r#"{"k": "hunter2"}"#, "an object"),
        ];
        for (value, kind) in kinds {
            let shapes = [
                (
                    value.to_string(),
                    "a JSON array of webhook targets",
                    "an array",
                ),
                (format!("[{value}]"), "a webhook target object", "an object"),
                (format!(r#"[{{"url": {value}}}]"#), "a string", "a string"),
                (
                    format!(r#"[{{"url": "http://h/", "secret": {value}}}]"#),
                    "a string",
                    "a string",
                ),
            ];
            for (body, want, accepted) in shapes {
                if kind == accepted {
                    continue;
                }
                let err = parse(&body).expect_err(&body).to_string();
                assert!(
                    err.starts_with(&format!("expected {want}, found {kind}")),
                    "{body}: {err}"
                );
                assert!(!err.contains("hunter2"), "{body}: no value echoed: {err}");
                assert!(!err.contains("12345"), "{body}: no value echoed: {err}");
            }
        }
        for field in ["inbox", "secret", "ca_pem"] {
            let body = format!(r#"[{{"url": "http://h/", "{field}": null}}]"#);
            let err = parse(&body).expect_err(&body).to_string();
            assert!(err.starts_with("expected a string, found null"), "{err}");
        }
        for (body, words) in [
            (
                r#"[{"url": "http://h/", "secrt": "s3cret"}]"#,
                "unknown field `secrt`, expected one of `url`, `inbox`, `secret`, `ca_pem`",
            ),
            (r#"[{"secret": "s3cret"}]"#, "missing field `url`"),
            (
                r#"[{"url": "http://h/", "url": "http://i/"}]"#,
                "duplicate field `url`",
            ),
        ] {
            let err = parse(body).expect_err(body).to_string();
            assert!(err.starts_with(words), "{body}: {err}");
            assert!(!err.contains("s3cret"), "{body}: {err}");
        }

        let legal = parse(r#"[{"url": "http://h/"}, {"url": "http://i/", "inbox": "a", "secret": "s", "ca_pem": "p"}]"#)
            .expect("a legal list parses");
        assert_eq!(legal.len(), 2);
        assert_eq!(
            (&legal[0].inbox, &legal[0].secret, &legal[0].ca_pem),
            (&None, &None, &None),
            "omission is None"
        );
        assert_eq!(legal[1].url, "http://i/");
        assert_eq!(legal[1].inbox.as_deref(), Some("a"));
        assert_eq!(legal[1].secret.as_deref(), Some("s"));
        assert_eq!(legal[1].ca_pem.as_deref(), Some("p"));
        assert!(parse("[]").expect("an empty list clears").is_empty());

        // A non-JSON caller reaches the visitors' `expecting` through a kind
        // JSON never produces (bytes): each shape names itself.
        let bytes = || serde::de::value::BytesDeserializer::<serde::de::value::Error>::new(b"x");
        let expecting = [
            WebhookTargets::deserialize(bytes()).map(drop),
            TargetSlot::deserialize(bytes()).map(drop),
            text(bytes()).map(drop),
        ];
        let wants = [
            "a JSON array of webhook targets",
            "a webhook target object",
            "a string",
        ];
        for (result, want) in expecting.into_iter().zip(wants) {
            let err = result.unwrap_err().to_string();
            assert!(err.ends_with(&format!("expected {want}")), "{err}");
        }
    }

    // ---------- decision 0029 §5: validated where they reach a wire ----------

    /// Every arm of the PUT validator, provable without a server: the legal
    /// target passes, and each refusal names the defect.
    #[test]
    fn validate_refuses_by_name_and_accepts_the_legal() {
        let legal = WebhookTarget {
            url: "http://127.0.0.1:9/hook".into(),
            inbox: None,
            secret: Some("s3cret".into()),
            ca_pem: None,
        };
        assert_eq!(legal.validate(), Ok(()));

        let unparseable = WebhookTarget {
            url: "not a url".into(),
            ..legal.clone()
        };
        assert!(
            unparseable
                .validate()
                .unwrap_err()
                .starts_with("invalid webhook URL"),
            "{unparseable:?}"
        );

        // The scheme-less spelling the rule exists to close: `Url::parse`
        // reads `localhost:9000` as *scheme* `localhost` with no host at
        // all — refused by name, never discovered at delivery time.
        let schemeless = WebhookTarget {
            url: "localhost:9000/hook".into(),
            ..legal.clone()
        };
        assert_eq!(
            schemeless.validate().unwrap_err(),
            "webhook URL must name a host"
        );

        let ftp = WebhookTarget {
            url: "ftp://hooks.example/x".into(),
            ..legal.clone()
        };
        assert_eq!(ftp.validate().unwrap_err(), UNSUPPORTED_SCHEME);

        // A supplied inbox filter answers to the one name rule (issue
        // #128): padding is kept, blank and control-bearing names refuse.
        let filter = |inbox: &str| WebhookTarget {
            inbox: Some(inbox.into()),
            ..legal.clone()
        };
        assert_eq!(filter(" pad ").validate(), Ok(()));
        assert_eq!(
            filter("  ").validate().unwrap_err(),
            crate::model::BLANK_INBOX_REFUSAL
        );
        assert_eq!(
            filter("a\0b").validate().unwrap_err(),
            crate::model::CONTROL_INBOX_REFUSAL
        );

        // A string with no PEM section at all: the parse is the refusal, not
        // the client build (which has its own arm in delivery_tests).
        let bad_pem = WebhookTarget {
            ca_pem: Some("this is not a pem at all".into()),
            ..legal.clone()
        };
        assert!(
            bad_pem
                .validate()
                .unwrap_err()
                .contains("webhook ca_pem could not be loaded into the rustls client"),
            "{bad_pem:?}"
        );

        // The 4096-octet boundary: exactly the cap is a header value, one
        // octet more is a refusal.
        let at_cap = WebhookTarget {
            secret: Some("x".repeat(4096)),
            ..legal.clone()
        };
        assert_eq!(at_cap.validate(), Ok(()));
        let over = WebhookTarget {
            secret: Some("x".repeat(4097)),
            ..legal.clone()
        };
        assert_eq!(
            over.validate().unwrap_err(),
            "webhook secret must be at most 4096 octets"
        );

        let crlf = WebhookTarget {
            secret: Some("a\r\nb".into()),
            ..legal
        };
        assert_eq!(crlf.validate().unwrap_err(), SECRET_NOT_A_HEADER);
    }

    /// The secret domain is the HTTP client's own (decision 0041 §1, issue
    /// #75): every value the PUT validator accepts builds the delivery
    /// header, and every control the header parser refuses — U+0001 and
    /// DEL included, which the old CR/LF/NUL rule let through — is refused
    /// by name, without echoing the secret.
    #[test]
    fn the_secret_domain_is_the_header_parsers() {
        for refused in [
            "bad\u{1}secret",
            "bad\u{7f}secret",
            "a\rb",
            "a\nb",
            "a\0b",
            "\u{1b}[0m",
        ] {
            let err = secret_header(refused).unwrap_err();
            assert_eq!(err, SECRET_NOT_A_HEADER, "{refused:?}");
            assert!(!err.contains("bad"), "no secret in the refusal: {err}");
        }
        // Edge whitespace a receiver may trim: each side, each character.
        for padded in [" s", "\ts", "s ", "s\t"] {
            assert_eq!(
                secret_header(padded).unwrap_err(),
                "webhook secret must not begin or end with a space or tab",
                "{padded:?}"
            );
        }
        // Accepted: empty, internal tab and space, visible ASCII, UTF-8.
        for accepted in ["", "a\tb c", "s3cret!~", "pässwörd", "密码"] {
            let value = secret_header(accepted).unwrap();
            assert_eq!(value.as_bytes(), accepted.as_bytes(), "{accepted:?}");
            assert!(value.is_sensitive());
        }
        // The octet bound counts bytes, not characters: 2048 two-octet
        // characters are exactly the cap, one octet more is a refusal.
        assert!(secret_header(&"é".repeat(2048)).is_ok());
        assert_eq!(
            secret_header(&format!("{}x", "é".repeat(2048))).unwrap_err(),
            "webhook secret must be at most 4096 octets"
        );
        assert!(secret_header(&"x".repeat(4096)).is_ok());
    }

    /// One display policy for URLs that leave the process (decision 0041
    /// §3): userinfo goes whole — username, password, either alone, plain
    /// or percent-encoded — query and fragment stay, a URL without
    /// userinfo keeps its spelling, and an unparseable one is a constant.
    #[test]
    fn display_url_removes_userinfo_and_nothing_else() {
        assert_eq!(
            display_url("http://127.0.0.1:9"),
            "http://127.0.0.1:9",
            "no userinfo: the configured spelling"
        );
        assert_eq!(
            display_url("http://user:hunter2@127.0.0.1:9/x?token=t#f"),
            "http://127.0.0.1:9/x?token=t#f"
        );
        assert_eq!(
            display_url("https://user@hooks.example/x"),
            "https://hooks.example/x"
        );
        assert_eq!(
            display_url("http://:hunter2@127.0.0.1:9/x"),
            "http://127.0.0.1:9/x"
        );
        assert_eq!(
            display_url("http://us%65r:hun%74er2@127.0.0.1:9/x"),
            "http://127.0.0.1:9/x"
        );
        assert_eq!(display_url("not a url hunter2"), UNPARSEABLE_URL);
    }

    /// The outward copy withholds credentials and reconstructs only
    /// complete, loadable public certificate blocks. Installed bytes stay
    /// intact for delivery and operator round trips.
    #[test]
    fn a_redacted_target_keeps_only_validated_public_ca_certificates() {
        let cert = rcgen::generate_simple_self_signed(vec!["ca.test".into()])
            .unwrap()
            .cert
            .pem();
        let key = rcgen::KeyPair::generate().unwrap().serialize_pem();
        let key_body = key
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<String>();
        let ca_input = format!("operator note\n{key}{cert}{cert}");
        let installed = WebhookTarget {
            url: "http://user:hunter2@127.0.0.1:9/hook".into(),
            inbox: Some("only".into()),
            secret: Some("s3cret".into()),
            ca_pem: Some(ca_input.clone()),
        };
        let shown = installed.redacted();
        assert!(
            shown.url == "http://127.0.0.1:9/hook",
            "public URL strips credentials"
        );
        assert_eq!(shown.inbox.as_deref(), Some("only"));
        assert!(shown.secret.is_none(), "secret withheld");
        assert!(
            shown.ca_pem.as_deref() == Some(format!("{cert}{cert}").as_str()),
            "canonical public projection"
        );
        let public = shown.ca_pem.unwrap();
        assert!(!public.contains("PRIVATE KEY"));
        assert!(
            !public.contains(
                key_body
                    .get(64..96)
                    .expect("generated private-body sentinel")
            )
        );
        assert!(!public.contains("operator note"));
        assert!(
            installed.ca_pem.as_deref() == Some(ca_input.as_str()),
            "raw configuration preserved"
        );
        assert!(
            installed.url == "http://user:hunter2@127.0.0.1:9/hook",
            "raw URL preserved"
        );
        assert!(
            installed.secret.as_deref() == Some("s3cret"),
            "raw secret preserved"
        );

        for invalid in [
            "no certificate here",
            "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
            "-----BEGIN CERTIFICATE-----\nYWJj\n-----END CERTIFICATE-----\n",
        ] {
            assert_eq!(public_ca_pem(invalid), "");
        }
        assert!(
            public_ca_pem(&format!(
                "{cert}-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n"
            ))
            .is_empty()
        );
        assert!(public_ca_pem("-----BEGIN CERTIFICATE-----\n!!!").is_empty());

        let private_der = rcgen::KeyPair::generate().unwrap().serialize_der();
        use base64::Engine as _;
        let private_body = base64::engine::general_purpose::STANDARD.encode(private_der);
        let mislabeled_key =
            format!("-----BEGIN CERTIFICATE-----\n{private_body}\n-----END CERTIFICATE-----\n");
        assert!(
            public_ca_pem(&mislabeled_key).is_empty(),
            "relabeled key withheld"
        );
        let directly_installed_key = WebhookTarget {
            url: "http://127.0.0.1:9/key-as-certificate".into(),
            inbox: None,
            secret: None,
            ca_pem: Some(mislabeled_key),
        };
        assert!(
            directly_installed_key.redacted().ca_pem.as_deref() == Some(""),
            "direct relabeled key withheld"
        );
    }

    /// The drop-warning window on supplied instants (decision 0041 §7): the
    /// first refusal warns, one nanosecond short of 60 s stays quiet (and
    /// does not extend the window), exactly 60 s after the last warning
    /// warns again and restarts it. No test waits a minute.
    #[test]
    fn the_drop_warning_window_is_sixty_seconds_from_the_last_warning() {
        let t = Instant::now();
        let mut last = None;
        assert!(should_warn(&mut last, t), "the first refusal warns");
        assert_eq!(last, Some(t));
        let almost = t + DROP_WARN_WINDOW - Duration::from_nanos(1);
        assert!(!should_warn(&mut last, almost), "inside the window: quiet");
        assert_eq!(last, Some(t), "a quiet refusal does not move the window");
        let edge = t + DROP_WARN_WINDOW;
        assert!(should_warn(&mut last, edge), "the window has passed");
        assert_eq!(last, Some(edge));
        assert!(!should_warn(&mut last, edge + Duration::from_secs(1)));
    }

    /// The budget key: a parsed URL normalizes, so two spellings of one
    /// sink share a budget; an unparseable URL falls back to the raw string.
    #[test]
    fn sink_keys_normalize_and_fall_back() {
        let upper_host_with_default_port = sink_key(&target("http://EXAMPLE.com:80/hook".into()));
        let lowercase_host = sink_key(&target("http://example.com/hook".into()));
        assert!(matches!(&upper_host_with_default_port, SinkKey::Url(_)));
        assert_eq!(
            upper_host_with_default_port, lowercase_host,
            "different spellings of the same parsed sink share one key"
        );
        assert_ne!(
            upper_host_with_default_port,
            sink_key(&target("http://example.com/other".into())),
            "a distinct path remains a distinct sink"
        );
        assert!(matches!(
            sink_key(&target("not a url".into())),
            SinkKey::Raw(_)
        ));
        // The Raw fallback is stable: the same unparseable spelling is the
        // same key, so its budget is still exactly one.
        assert_eq!(
            sink_key(&target("not a url".into())),
            sink_key(&target("not a url".into()))
        );
    }

    /// Retirement: a key is dropped only when its target has vanished from
    /// the config AND its set has drained — or every PUT cycle leaks a dead
    /// key (decision 0029 §5).
    #[tokio::test]
    async fn retirement_takes_the_dead_and_drained_keys_only() {
        let mut deliveries = HashMap::new();
        let mut live = std::collections::HashSet::new();

        // A live target keeps its budget whether drained or not.
        deliveries.insert(SinkKey::Raw("live".into()), Budget::default());
        live.insert(SinkKey::Raw("live".into()));
        // A vanished target with a delivery still in flight keeps its key:
        // its outcome still owes a row.
        let mut busy = Budget::default();
        busy.set.spawn(std::future::pending::<()>());
        deliveries.insert(SinkKey::Raw("busy".into()), busy);
        // A vanished, drained target goes — and its warning window with it
        // (decision 0041 §7): a re-added target warns afresh.
        let dead = Budget {
            warned: Some(Instant::now()),
            ..Budget::default()
        };
        deliveries.insert(SinkKey::Raw("dead".into()), dead);

        retire_dead(&mut deliveries, &live);
        assert_eq!(deliveries.len(), 2);
        assert!(deliveries.contains_key(&SinkKey::Raw("live".into())));
        assert!(deliveries.contains_key(&SinkKey::Raw("busy".into())));
        assert!(!deliveries.contains_key(&SinkKey::Raw("dead".into())));
    }

    /// The two-sink acceptance (decision 0029 §5): a dead target cannot
    /// cause drops for a healthy one. The stalled sink fills its own 32
    /// slots; the next mail's fan-out to *it* is counted dropped, while the
    /// healthy sink's slots are its own and every one of its fan-outs
    /// delivers. Under the old shared pool the healthy sink's fan-outs
    /// would have been dropped from the 33rd mail on.
    #[tokio::test]
    async fn a_dead_target_cannot_cause_drops_for_a_healthy_one() {
        let (stalled, _stalled_hits, stalled_retire, stalled_sink) = counting_sink(None).await;
        let (fast, _fast_hits, fast_retire, fast_sink) = counting_sink(Some(FAST_REPLY)).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![
            target(format!("http://{stalled}/hook")),
            target(format!("http://{fast}/hook")),
        ]);
        let (tx, rx) = broadcast::channel(WEBHOOK_DELIVERY_CAP + 4);
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

        // Phase one: fill the stalled sink's own budget. Every mail also
        // fans out to the healthy sink, and each of those fan-outs delivers —
        // waited on the ledger itself (a production observable, decision 0020),
        // so the healthy budget is empty again before phase two.
        for i in 0..WEBHOOK_DELIVERY_CAP {
            let mut e = email();
            e.id = format!("{i}");
            let _ = tx.send(Arc::new(e));
        }
        spin_until(5000, || {
            webhooks.snapshot().delivered == WEBHOOK_DELIVERY_CAP as u64
        })
        .await;

        // Phase two: one more mail. The stalled sink's budget is exactly full —
        // its fan-out is counted dropped — while the healthy sink's slots are
        // its own and its fan-out delivers. Under the old shared pool this
        // healthy fan-out would have been dropped too.
        let mut e = email();
        e.id = "last".into();
        let _ = tx.send(Arc::new(e));
        spin_until(5000, || {
            webhooks.snapshot().delivered == (WEBHOOK_DELIVERY_CAP + 1) as u64
        })
        .await;
        spin_until(5000, || webhooks.snapshot().dropped == 1).await;

        let _ = drained_tx.send(true);
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the drain must end the loop")
            .unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(
            ledger.delivered,
            (WEBHOOK_DELIVERY_CAP + 1) as u64,
            "every healthy fan-out delivered: {ledger:?}"
        );
        assert_eq!(
            ledger.dropped, 1,
            "the stalled sink's next fan-out is dropped; the healthy sink's is not: {ledger:?}"
        );
        assert_eq!(ledger.exhausted, 0);
        assert_eq!(
            ledger.aborted, WEBHOOK_DELIVERY_CAP as u64,
            "the stalled deliveries still in flight at the drain: {ledger:?}"
        );

        // The sinks outlive nothing: retiring them ends their accept loops —
        // no test leaves a listener spinning under the coverage run.
        drop(stalled_retire);
        drop(fast_retire);
        tokio::time::timeout(std::time::Duration::from_secs(2), stalled_sink)
            .await
            .expect("the retired sink's accept loop must end")
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), fast_sink)
            .await
            .expect("the retired sink's accept loop must end")
            .unwrap();
    }

    /// A closed firehose ends the dispatcher: the store is gone, and the
    /// tail sweep still names the ring resident (decision 0029 §5).
    #[tokio::test]
    async fn a_closed_firehose_names_its_ring_resident() {
        let (tx, rx) = broadcast::channel(4);
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![target("http://127.0.0.1:9/hook".into())]);
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (_drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let handle = tokio::spawn(dispatch_loop(rx, webhooks.clone(), shutdown_rx, drained_rx));
        let _ = tx.send(Arc::new(email()));
        drop(tx); // the firehose closes before the drain
        handle.await.unwrap();
        let ledger = webhooks.snapshot();
        assert_eq!(ledger.aborted, 1, "the ring resident is named: {ledger:?}");
        assert_eq!(ledger.delivered, 0, "the close is not a delivery");
    }

    /// A lagged firehose is counted in mails (decision 0021's unit): the
    /// fan-out of a lagged mail is unknowable by construction.
    #[tokio::test]
    async fn a_lagged_firehose_is_counted_in_mails() {
        let (tx, rx) = broadcast::channel(1);
        let webhooks = Arc::new(Webhooks::default());
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (_drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let handle = tokio::spawn(dispatch_loop(rx, webhooks.clone(), shutdown_rx, drained_rx));
        for i in 0..3 {
            let mut e = email();
            e.id = format!("{i}");
            let _ = tx.send(Arc::new(e)); // cap 1: the receiver lags by two
        }
        drop(tx);
        handle.await.unwrap();
        assert_eq!(webhooks.snapshot().lagged, 2);
    }

    // ---------- decision 0041: the webhook boundary ----------

    /// Issue #76's controlled case: the shutdown sweep continues past a lag
    /// report. On this current-thread runtime the dispatcher is spawned but
    /// never polled while 8,200 uniquely identified mails are inserted
    /// synchronously — the global ring holds 8,192 — and both shutdown
    /// signals are sent before its first poll, so the biased select hands
    /// it the drain at once. The tail's first read is `Lagged(8)`, and the
    /// 8,192 retained notifications follow it: 8 lagged mails, 8,192 aborted
    /// notifications, no delivery attempt, no payload built. The old
    /// `while let Ok` sweep stopped at the lag and counted nothing.
    #[tokio::test]
    async fn the_shutdown_sweep_continues_past_a_lagged_report() {
        let store = Store::new(16); // pruning keeps the store small; the ring is fixed
        let webhooks = Arc::new(Webhooks::default());
        // Never dialed: the stop path counts, it never spawns.
        webhooks.set(vec![target("http://127.0.0.1:9/hook".into())]);
        let mut tasks = tokio::task::JoinSet::new();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        spawn_dispatcher(&store, &webhooks, shutdown_rx, drained_rx, &mut tasks);
        for i in 0..8200 {
            let mut e = email();
            e.id = format!("lag-{i}");
            store.insert(e).unwrap();
        }
        let _ = shutdown_tx.send(true);
        let _ = drained_tx.send(true);
        tasks
            .join_next()
            .await
            .expect("the dispatcher was spawned")
            .expect("the dispatcher returns cleanly");
        assert_eq!(
            webhooks.snapshot(),
            WebhookLedger {
                delivered: 0,
                dropped: 0,
                lagged: 8,
                exhausted: 0,
                aborted: 8192,
            }
        );
        assert!(webhooks.built.lock().unwrap().is_empty());
    }

    /// One mail to four sinks builds one payload (decision 0041 §4): every
    /// target's delivery — and every retry of the one that fails all four
    /// attempts — replays that single build, and each sink receives it.
    #[tokio::test]
    async fn one_mail_builds_one_payload_for_every_target_and_retry() {
        let mut sinks = Vec::new();
        for _ in 0..4 {
            sinks.push(counting_sink(Some(FAST_REPLY)).await);
        }
        // Bind and drop: a port nothing listens on, every attempt refused.
        let refused = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let webhooks = Arc::new(Webhooks::default());
        let mut targets: Vec<WebhookTarget> = sinks
            .iter()
            .map(|(addr, ..)| target(format!("http://{addr}/hook")))
            .collect();
        targets.push(target(format!("http://{refused}/hook")));
        webhooks.set(targets);
        let (tx, rx) = broadcast::channel(8);
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

        tx.send(Arc::new(email())).unwrap();
        spin_until(5000, || webhooks.snapshot().delivered == 4).await;
        spin_until(10_000, || webhooks.snapshot().exhausted == 1).await;
        assert_eq!(webhooks.built.lock().unwrap().len(), 1, "built once");
        for (_, hits, ..) in &sinks {
            assert_eq!(hits.load(Ordering::Relaxed), 1);
        }

        let _ = drained_tx.send(true);
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the drain must end the loop")
            .unwrap();
        for (_, _, retire, sink) in sinks {
            drop(retire);
            tokio::time::timeout(std::time::Duration::from_secs(2), sink)
                .await
                .expect("the retired sink's accept loop must end")
                .unwrap();
        }
    }

    /// A burst of refusals against one stalled target (issue #98, decision
    /// 0041 §7 and §3): every refusal is counted in `dropped`, one warning
    /// names the target, its URL carries no credentials, and a refused
    /// fan-out builds no payload. The target's userinfo is real: the
    /// stalled sink is dialed with it.
    #[tokio::test]
    async fn a_refusal_burst_counts_every_drop_and_warns_once_without_credentials() {
        let log = captured_log();
        let (stalled, stalled_hits, stalled_retire, stalled_sink) = counting_sink(None).await;
        let webhooks = Arc::new(Webhooks::default());
        webhooks.set(vec![target(format!("http://user:hunter2@{stalled}/hook"))]);
        let (tx, rx) = broadcast::channel(2 * WEBHOOK_DELIVERY_CAP);
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
        let loop_task = tokio::spawn(dispatch_loop(
            rx,
            Arc::clone(&webhooks),
            shutdown_rx,
            drained_rx,
        ));

        for i in 0..WEBHOOK_DELIVERY_CAP {
            let mut e = email();
            e.id = format!("fill-{i}");
            tx.send(Arc::new(e)).unwrap();
        }
        spin_until(5000, || {
            stalled_hits.load(Ordering::Relaxed) == WEBHOOK_DELIVERY_CAP
        })
        .await;
        const BURST: u64 = 5;
        for i in 0..BURST {
            let mut e = email();
            e.id = format!("burst-{i}");
            tx.send(Arc::new(e)).unwrap();
        }
        spin_until(5000, || webhooks.snapshot().dropped == BURST).await;
        assert_eq!(
            webhooks.built.lock().unwrap().len(),
            WEBHOOK_DELIVERY_CAP,
            "one build per admitted mail; a refused fan-out builds nothing"
        );

        let _ = drained_tx.send(true);
        tokio::time::timeout(std::time::Duration::from_secs(2), loop_task)
            .await
            .expect("the drain must end the loop")
            .unwrap();
        std::io::Write::flush(&mut log.clone()).unwrap();
        let logged = String::from_utf8_lossy(&log.0.lock().unwrap()).to_string();
        // Only the budget warning names this sink: the lines carrying its
        // address are exactly the drop warnings.
        let sink = format!("{stalled}/hook");
        let warnings: Vec<&str> = logged.lines().filter(|l| l.contains(&sink)).collect();
        assert_eq!(warnings.len(), 1, "{BURST} drops, one warning: {logged}");
        assert!(warnings[0].contains("over its delivery budget"), "{logged}");
        assert!(
            !logged.contains("hunter2"),
            "no credential is logged: {logged}"
        );
        assert_eq!(webhooks.snapshot().aborted, WEBHOOK_DELIVERY_CAP as u64);

        drop(stalled_retire);
        tokio::time::timeout(std::time::Duration::from_secs(2), stalled_sink)
            .await
            .expect("the retired sink's accept loop must end")
            .unwrap();
    }
}

#[cfg(test)]
mod ca_construction_tests;
