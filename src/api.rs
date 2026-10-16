//! HTTP API: REST for tests, MCP endpoint, OpenAPI, health/metrics.

use crate::chaos::{Chaos, ChaosConfig};
use crate::compat;
use crate::feed;
use crate::mcp::{self, McpContext};
use crate::model::Email;
use crate::smtp;
use crate::store::{Filter, Store, WaitOutcome};
use crate::threads;
use crate::ui;
use crate::webhook::{WebhookTarget, Webhooks, ca_cache_stats};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::Stream;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub chaos: Arc<Chaos>,
    pub webhooks: Arc<Webhooks>,
    pub started: Instant,
    /// The server's stop watch. It ends open feeds (`feed::feed_stream`)
    /// so an SSE client can never hold `RunningServer::stop` hostage, and
    /// it is the only state a graceful shutdown needs that the router owns.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
    /// Per-surface accept health (decision 0027), set by each loop on a
    /// failed accept and cleared on its next success. `/healthz` names the
    /// surfaces that cannot currently take connections.
    pub smtp_degraded: Arc<AtomicBool>,
    pub pop3_degraded: Arc<AtomicBool>,
    /// The bound HTTP port when the HTTP listener is loopback-bound — the
    /// Origin guard's vocabulary (decision 0030 §2). `None` when the
    /// operator bound an explicit non-loopback address: the bind is then
    /// the trust statement and the check is inert. A loopback bind serves
    /// the operator's own machine, where a browser page from any other
    /// origin must not be able to drive the API. Two honest headers close
    /// the class: `Host` (a rebound page still spells its own hostname,
    /// so a non-loopback Host is the rebind itself) and `Origin` (a
    /// browser cannot forge it, so a cross-site drive is named).
    pub http_origin_guard: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.1 });
        (self.0, Json(body)).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// The 400 envelope with a sentence in it (decision 0025): shared with the
/// compat plane so every staging refusal keeps one owner per sentence.
pub(crate) fn bad_request(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

/// The largest `timeout_ms` a wait may ask for, on every surface (REST and
/// MCP — decision 0018). A parked wait holds a connection and a task; an
/// unbounded client timeout would let one request park a worker for hours
/// and — before the shutdown race existed — hold `stop()` hostage for its
/// remainder. Over the cap is a refusal, never a clamp: silently waiting
/// only 60s for a caller who asked for an hour is the same narrowing disease
/// as the wire casts this decision removes. The stop budget (10s) is
/// independent of any caller's wait: waits observe the stop watch.
pub(crate) const MAX_AWAIT_TIMEOUT_MS: u64 = 60_000;

/// The documented wait default (issue #32 slice 2, decision 0025): named
/// once and used by both parsers — REST's query default and MCP's omitted
/// `timeout_ms` arm — so an omitted value keeps one documented default and
/// a *supplied* malformed value can never ride it.
pub(crate) const DEFAULT_WAIT_TIMEOUT_MS: u64 = 5_000;

/// The supplied-malformed refusal: one sentence, both transports (decision
/// 0025). A `timeout_ms` that is *present but unparseable* is refused,
/// naming the field and the domain — it is never treated like omission.
pub(crate) const WAIT_TIMEOUT_REFUSAL: &str = "timeout_ms must be an integer between 0 and 60000";

/// The one default page size for every listing plane (decision 0029 §1):
/// REST's list and MCP's `swarmail_search_emails` both omit to this — one
/// constant, so the planes cannot drift. `compat`'s Mailpit shape keeps its
/// own 50; it is a different dialect, not a second copy of this domain.
pub(crate) const DEFAULT_LIST_LIMIT: usize = 100;

/// The largest page any listing or waiting surface may return (decision
/// 0030 §3): upstream MailHog's own cap. A caller paging without `until`
/// could otherwise ask the server to serialize megabytes of bodies per
/// request — the one unbounded read left on an otherwise bounded server.
/// Above the cap is a refusal naming the field and the range, never a
/// clamp; the wait surfaces cap `count` the same way, while `assert`'s
/// exact `count` and inert `limit` stay exempt (an assertion's answer is a
/// count, not a page).
pub(crate) const MAX_PAGE_SIZE: usize = 250;

/// The one cap check for every page size (decision 0030 §3): the field's
/// name and the accepted range in one sentence — the `timeout_ms`
/// refusal's shape (0018/0025) — owned once so no plane can drift its
/// words. The sentence rides each plane's existing envelope: REST/compat
/// `400` JSON, MCP `-32602`. Staging before effect: an over-cap value is
/// refused before any scan is taken or any wait parks.
pub(crate) fn checked_page_size(field: &str, n: usize) -> Result<usize, String> {
    if n > MAX_PAGE_SIZE {
        return Err(format!("{field} must be at most {MAX_PAGE_SIZE}"));
    }
    Ok(n)
}

/// The one cap check for every wait surface (0018 §3, made shared by
/// decision 0025): over the cap is a refusal naming the bound, never a
/// clamp. Both transports call this — REST's query and the MCP wait tool —
/// so the cap and its words have exactly one owner. Refusal precedes every
/// effect: an over-cap wait never registers a watcher.
pub(crate) fn checked_wait_timeout_ms(ms: u64) -> Result<Duration, String> {
    if ms > MAX_AWAIT_TIMEOUT_MS {
        return Err(format!("timeout_ms must be at most {MAX_AWAIT_TIMEOUT_MS}"));
    }
    Ok(Duration::from_millis(ms))
}

/// Resolve once the stop has fired — at once if it already has, else on the
/// next change. Every wait surface races its wait against this signal (the
/// same receiver the SSE feed consumes: one source of "stopping", never a
/// second channel). A dropped sender resolves as stopping: the last sender
/// is gone only when the server is going away. This is 0002's per-iteration
/// re-check applied to the stop watch — the receiver is borrowed *and
/// updated* first, so a stop that fired before the call is never missed by
/// a stale cached value.
pub(crate) async fn shutdown_signal(mut rx: tokio::sync::watch::Receiver<bool>) {
    if !*rx.borrow_and_update() {
        let _ = rx.changed().await;
    }
}

// ---------- readiness gauges (issue #51) ----------
//
// A test that must sequence against the server — "my wait is parked",
// "the server has my request", "the delivery is mid-commit" — used to have
// only a sleep, which is a race wearing a timeout (decision 0020 rule 5):
// it proves nothing about where the request actually is. These gauges are
// the observables the sleeps are replaced with, read over real HTTP from
// `/metrics` — the production server's own words, never a test-side guess.
//
// Every gauge is **labeled by inbox** in addition to its total. The label
// is what makes a poll sound: several test servers share one process
// (the integration suites spawn them on one runtime), so a *global* count
// mixes parallel tests' units — a delta-over-baseline poll on a gauge that
// can fall is a coin flip. A labeled row named for the caller's own inbox
// (decision 0004: per-test inboxes on the wire) can only be raised by that
// caller's unit, so "the row reached 1" means *my* unit is in flight, not
// merely that somebody's is. An inbox with no live unit renders no row.

/// The shape every readiness gauge shares: live units per inbox. A
/// `BTreeMap` so the rendered rows are deterministic in name order.
type Registry = std::sync::Mutex<std::collections::BTreeMap<String, u64>>;

/// Parked waits per inbox: the REST `await` and the MCP
/// `swarmail_wait_for_email` both register through [`park_await`], so one
/// gauge covers both surfaces. A cap refusal or a parse failure never
/// parks, and so never counts.
static AWAITS: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// HTTP requests currently inside the router, per inbox path — raised by
/// [`count_in_flight`] for any request scoped to an inbox route. The
/// router's total lives beside it in [`HTTP_REQUESTS_IN_FLIGHT`].
static REQUESTS: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// SMTP deliveries between the DATA terminator and their insert resolving,
/// per inbox — the accept == stored seam (0001) made observable.
static DELIVERIES: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// HTTP requests currently inside the router: incremented before the
/// handler runs, released when the response is produced (or the handler's
/// task unwinds).
static HTTP_REQUESTS_IN_FLIGHT: AtomicU64 = AtomicU64::new(0);

fn raise(registry: &Registry, inbox: &str) {
    *registry
        .lock()
        .expect("a readiness registry mutex cannot poison")
        .entry(inbox.to_string())
        .or_insert(0) += 1;
}

fn release(registry: &Registry, inbox: &str) {
    let mut units = registry
        .lock()
        .expect("a readiness registry mutex cannot poison");
    let count = units
        .get_mut(inbox)
        .expect("a live unit's inbox cannot vanish before its guard drops");
    *count -= 1;
    if *count == 0 {
        // No row for a quiescent inbox: a row's existence is the unit's
        // liveness, the property the polls rely on.
        units.remove(inbox);
    }
}

/// A live unit of one readiness gauge — a parked wait, a queued request, a
/// mid-commit delivery — counted under its inbox until the guard drops, on
/// resolve *and* on unwind, so a panicking handler can never leak the row.
pub(crate) struct UnitGuard {
    registry: &'static Registry,
    inbox: String,
}

impl UnitGuard {
    fn take(registry: &'static Registry, inbox: &str) -> Self {
        raise(registry, inbox);
        Self {
            registry,
            inbox: inbox.to_string(),
        }
    }
}

impl Drop for UnitGuard {
    fn drop(&mut self) {
        release(self.registry, &self.inbox);
    }
}

/// Register a parked wait (REST `await` or MCP `wait_for_email`) for
/// `/metrics` observation, under its inbox. The returned guard releases
/// the row when the wait resolves — by answer, by stop, or by unwind.
pub(crate) fn park_await(inbox: &str) -> UnitGuard {
    UnitGuard::take(&AWAITS, inbox)
}

/// Count one delivery between its DATA terminator and its insert
/// resolving, under its inbox (the accept == stored seam, 0001).
pub(crate) fn park_delivery(inbox: &str) -> UnitGuard {
    UnitGuard::take(&DELIVERIES, inbox)
}

/// Count every request the router sees, labeled when the path scopes an
/// inbox. A test that must know the server has its request reads the
/// labeled row over real HTTP instead of sleeping; the guards release
/// their counts on response, unwind *and* cancellation (a dropped
/// connection drops the handler future — a bare fetch-sub after the await
/// would leak the total forever).
async fn count_in_flight(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let inbox = inbox_of_path(req.uri().path());
    let _label = inbox.as_deref().map(|i| UnitGuard::take(&REQUESTS, i));
    let _total = TotalGuard(&HTTP_REQUESTS_IN_FLIGHT);
    next.run(req).await
}

/// Count one request against the router's total until the guard drops.
struct TotalGuard(&'static AtomicU64);

impl Drop for TotalGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

// ---------- the loopback guard (decision 0030 §2) ----------
//
// A loopback bind serves the operator's own machine. Without a guard, any
// web page the operator browses can drive the API from JavaScript — the
// server has no auth to refuse it. Two headers close the class, each
// honestly set:
//
// `Host` is the DNS-rebinding defense. A page at `attacker.example`
// rebound to `127.0.0.1` sends *same-origin* requests — and same-origin
// GET/HEAD requests carry no `Origin` header at all (the Fetch spec adds
// one only for CORS-tainted requests). But the page still fetches its
// own hostname, so the request's `Host` remains `attacker.example:8025`:
// the rebind is visible in the Host line. While loopback-bound, a `Host`
// that is not a loopback spelling of the bound port is refused before
// the router.
//
// `Origin` is the cross-site drive defense: the one header a browser
// sets honestly (it cannot be forged by page script and is present on
// every cross-site request), so a present `Origin` that is not a
// loopback spelling of the bound port is refused before the router.
//
// Everything that is not a browser — curl, agents, the MCP stdio bridge,
// every test harness — sends a loopback `Host` to a loopback bind and no
// `Origin` at all, and is untouched.

/// The whole-plane loopback guard: while the listener is loopback-bound
/// (an off-loopback bind is the operator's own trust statement — the
/// check is inert), a request is answered `403` before the router unless
/// its `Host` authority is a loopback spelling of the bound HTTP port
/// and any present `Origin` is one too. Every method, every route:
/// `/mcp` and the UI are as drivable as the REST surface. A request
/// without `Origin` passes the Origin leg — the shape of every
/// non-browser client and of a rebound page's same-origin reads, which
/// is why the Host leg exists.
async fn origin_guard(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if let Some(bound) = state.http_origin_guard {
        let host_ok = req
            .headers()
            .get(axum::http::header::HOST)
            .and_then(|h| h.to_str().ok())
            .map(|h| authority_is_loopback_of(h, bound))
            .unwrap_or(false);
        if !host_ok {
            return ApiError(
                StatusCode::FORBIDDEN,
                "requests to the loopback plane must name a loopback Host".into(),
            )
            .into_response();
        }
        if let Some(origin) = req.headers().get(axum::http::header::ORIGIN) {
            let allowed = origin
                .to_str()
                .ok()
                .map(|o| origin_is_loopback_of(o, bound))
                .unwrap_or(false);
            if !allowed {
                return ApiError(
                    StatusCode::FORBIDDEN,
                    "cross-origin requests are refused on the loopback plane".into(),
                )
                .into_response();
            }
        }
    }
    next.run(req).await
}

/// Whether an `authority` (the `host[:port]` of a Host line or an
/// Origin's tail) names this plane: a loopback spelling — `localhost`,
/// any `127.0.0.0/8` address, or `[::1]` — carrying the bound port
/// explicitly. Hosts compare case-insensitively (DNS names are
/// case-insensitive); anything else — a named host, another port, no
/// port, garbage — is not this plane.
fn authority_is_loopback_of(authority: &str, bound: u16) -> bool {
    let authority = authority.trim();
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        // IPv6 literal: the host ends at `]`, an optional port follows.
        match rest.split_once(']') {
            Some((host, tail)) => (host, tail.strip_prefix(':')),
            None => return false,
        }
    } else {
        // The port is the *last* colon; a bare host has none.
        match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    // The bound port is the vocabulary: a loopback spelling of another
    // port is a different origin. No explicit port means the scheme's
    // default, which cannot be this bound port.
    let Some(explicit) = port.and_then(|p| p.parse::<u16>().ok()) else {
        return false;
    };
    if explicit != bound {
        return false;
    }
    let host = host.to_ascii_lowercase();
    host == "localhost"
        || host == "::1"
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// Whether `origin` names this plane: its authority is a loopback
/// spelling of the bound port (see [`authority_is_loopback_of`]).
fn origin_is_loopback_of(origin: &str, bound: u16) -> bool {
    let Some((_, authority)) = origin.split_once("://") else {
        return false;
    };
    // The authority ends at the first `/` (an Origin carries no path, but
    // a lenient caller may pass one); before it there is always a
    // remainder, even when there is no slash at all.
    let end = authority.find('/').unwrap_or(authority.len());
    authority_is_loopback_of(&authority[..end], bound)
}

/// The inbox a request path scopes — `/api/v1/inboxes/<inbox>/…`, the wire
/// segment as requested (untranslated): the labeled row a test polls is
/// the path its own request carried.
fn inbox_of_path(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        ["api", "v1", "inboxes", inbox, ..] => Some((*inbox).to_string()),
        _ => None,
    }
}

/// Waits currently parked in the wait machinery, for `/metrics`.
pub fn awaits_active() -> u64 {
    registry_total(&AWAITS)
}

/// HTTP requests currently inside the router, for `/metrics`.
pub fn http_requests_in_flight() -> u64 {
    HTTP_REQUESTS_IN_FLIGHT.load(Ordering::Relaxed)
}

/// One readiness registry's rows: the total and, sorted by inbox, one
/// labeled sample per inbox with a live unit.
fn registry_rows(registry: &'static Registry, name: &str, total: u64) -> String {
    let units = registry
        .lock()
        .expect("a readiness registry mutex cannot poison");
    let mut out = format!("# TYPE {name} gauge\n{name} {total}\n");
    for (inbox, count) in units.iter() {
        out.push_str(&format!("{name}{{inbox={inbox:?}}} {count}\n"));
    }
    out
}

fn registry_total(registry: &Registry) -> u64 {
    registry
        .lock()
        .expect("a readiness registry mutex cannot poison")
        .values()
        .sum()
}

/// The drain deadline (decision 0027): a stop waits for in-flight HTTP
/// work exactly this long, then returns anyway. A client that stalls
/// mid-request — or mid-body, where the graceful flag cannot reach it —
/// must not hold the process past its own shutdown. The value is 0018's
/// whole fence — busy bound plus margin — because the two tails overlap:
/// a commit already in flight when the stop fires runs to completion
/// (0018 §2), burning up to its busy bound *inside* the drain window,
/// and its refusal must be written to the socket before the deadline
/// cuts it. A deadline at the margin alone raced a macOS runner's
/// slower scheduling and closed a refused seed with no answer at all.
const DRAIN_DEADLINE: Duration = Duration::from_secs(10);

pub async fn serve(
    listener: TcpListener,
    state: AppState,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    // The stop watch is cloned before the router consumes the receiver:
    // the deadline race below and /healthz's stopping answer are two
    // readers of the same watch, both subscribed before the stop can fire.
    let stop_rx = state.shutdown.clone();
    let drain = async {
        shutdown_signal(stop_rx).await;
        tokio::time::sleep(DRAIN_DEADLINE).await;
    };
    // The graceful serve is raced against stop-plus-deadline: a clean
    // drain finishes on its own; a client parked mid-request (or mid-body)
    // is cut off at the deadline instead of holding the process forever.
    // The serve future is pinned so the race can poll it in place.
    let mut serve = std::pin::pin!(
        axum::serve(listener, router(state))
            .with_graceful_shutdown(shutdown)
            .into_future(),
    );
    tokio::select! {
        res = &mut serve => res,
        _ = drain => Ok(()),
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics))
        .route("/api/v1/inboxes", get(list_inboxes))
        .route(
            "/api/v1/inboxes/{inbox}/messages",
            get(list_messages).delete(clear_inbox),
        )
        .route("/api/v1/inboxes/{inbox}/count", get(count_messages))
        .route("/api/v1/inboxes/{inbox}/await", get(await_messages))
        .route("/api/v1/inboxes/{inbox}/assert", get(assert_messages))
        .route("/api/v1/inboxes/{inbox}/threads", get(list_threads))
        .route("/api/v1/inboxes/{inbox}/threads/{key}", get(get_thread))
        .route("/api/v1/inboxes/{inbox}/feed", get(inbox_feed))
        // GET on /api/v1/messages is the Mailpit/MailHog list shape; DELETE
        // stays swarmail-native with Mailpit's selective `{"ids":[..]}` body.
        .route(
            "/api/v1/messages",
            get(compat::list_messages_mailpit).delete(compat::delete_messages),
        )
        .route(
            "/api/v1/messages/{id}",
            get(get_message).delete(delete_message),
        )
        .route("/api/v1/messages/{id}/raw", get(get_raw))
        .route(
            "/api/v1/chaos",
            get(get_chaos).put(set_chaos).delete(clear_chaos),
        )
        .route("/api/v1/inboxes/{inbox}/seed", post(seed_inbox))
        .route(
            "/api/v1/webhooks",
            get(get_webhooks).put(set_webhooks).delete(clear_webhooks),
        )
        .route("/mcp", post(mcp_endpoint))
        .route("/openapi.json", get(openapi_json))
        .route("/llms.txt", get(llms_txt))
        .merge(ui::router())
        .merge(compat::router())
        // The Origin guard wraps the whole plane (decision 0030 §2). The
        // layers are applied innermost-first: this `.layer()` is
        // textually before `count_in_flight`'s, so the gauge is the
        // outer layer and a guard-refused 403 does register in the
        // in-flight count for the instant of its own answer — the ruled
        // criterion is that the guard acts before the router, which
        // holds either way.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            origin_guard,
        ))
        .layer(axum::middleware::from_fn(count_in_flight))
        .with_state(state)
}

// ---------- shared query params ----------

#[derive(Debug, Deserialize, Default)]
pub struct ListQuery {
    pub to: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since_ms: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl ListQuery {
    fn filter(&self) -> Filter {
        Filter {
            to: self.to.clone(),
            from: self.from.clone(),
            subject: self.subject.clone(),
            since_ms: self.since_ms,
        }
    }
}

/// The await/assert domain, held flat (decision 0029 §1): the struct's
/// fields *are* the advertised query parameter set — no `#[serde(flatten)]`
/// of the list shape, so an await's domain cannot silently grow `offset`
/// (a wait has no pages: mail arrives as it lands).
#[derive(Debug, Deserialize)]
pub struct AwaitQuery {
    pub to: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since_ms: Option<i64>,
    /// Page size for the returned matches — the same shape `list` takes
    /// (#38's ruled parity): a wrong type refuses with the shared 400
    /// sentence, an omission keeps the shared default.
    pub limit: Option<usize>,
    /// Number of matching emails to wait for (default 1).
    pub count: Option<usize>,
    /// Give up after this long; default 5000ms.
    pub timeout_ms: Option<u64>,
}

impl AwaitQuery {
    fn filter(&self) -> Filter {
        Filter {
            to: self.to.clone(),
            from: self.from.clone(),
            subject: self.subject.clone(),
            since_ms: self.since_ms,
        }
    }

    /// WAIT semantics (decision 0010): a caller waiting for mail must never
    /// get an instant success against an empty inbox, so `count=0` still
    /// means at least one. Pinned by the `await?count=0` tests.
    fn min_one_count(&self) -> usize {
        self.count.unwrap_or(1).max(1)
    }

    /// ASSERT semantics (decision 0010): the exact filtered match count,
    /// zero included — `count=0` is the negative assertion. `assert` never
    /// waits and never subscribes; a snapshot is a snapshot.
    fn exact_count(&self) -> usize {
        self.count.unwrap_or(1)
    }

    fn timeout(&self) -> Result<Duration, ApiError> {
        // `unwrap_or` touches only genuine omission (decision 0025): a
        // supplied malformed `timeout_ms` never reaches here — the query
        // parse refused it with 400 before the handler ran. Over the cap
        // is the shared refusal (0018), never a clamp.
        checked_wait_timeout_ms(self.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS))
            .map_err(bad_request)
    }
}

#[derive(Debug, Serialize)]
pub struct InboxSummary {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct EmailList {
    pub total: usize,
    /// Held as `Arc<Email>` and serialized through serde's `rc` support:
    /// `raw` is `#[serde(skip)]`, so a page of whole-`Email` clones would
    /// deep-copy bytes (potentially many megabytes of them) that
    /// serialization provably never reads. The wire is byte-identical.
    pub emails: Vec<Arc<Email>>,
}

#[derive(Debug, Serialize)]
pub struct AwaitResponse {
    pub matched: usize,
    pub total: usize,
    /// Held as `Arc<Email>` for the same reason as [`EmailList`]: the wait's
    /// matches are already `Arc` from the store; a deep clone would copy
    /// bytes (`raw` is skipped, so serialization never reads them) the wire
    /// is byte-identical without (decision 0030 §1).
    pub emails: Vec<Arc<Email>>,
}

// ---------- handlers ----------

/// The readiness contract (decision 0027), four states:
///
/// - `200 ok` — every listener is accepting;
/// - `503 degraded` — a listener's accept loop is in a failed-accept
///   backoff; the body names the surfaces, so a probe reads *which* socket
///   is dark, not just that something is;
/// - `503 stopping` — the stop has fired and the process is draining.
///
/// Stopping wins over degraded: the process is leaving either way, and a
/// surface's acceptance state is moot once the stop has fired. Both 503
/// bodies carry a `status` field, so a probe never has to infer state from
/// the code alone.
async fn healthz(State(mut state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    if *state.shutdown.borrow_and_update() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "status": "stopping" })),
        );
    }
    let mut degraded: Vec<&'static str> = Vec::new();
    if state.smtp_degraded.load(Ordering::Relaxed) {
        degraded.push("smtp");
    }
    if state.pop3_degraded.load(Ordering::Relaxed) {
        degraded.push("pop3");
    }
    if degraded.is_empty() {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "uptime_seconds": state.started.elapsed().as_secs(),
            })),
        );
    }
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({ "status": "degraded", "degraded": degraded })),
    )
}

async fn metrics(State(state): State<AppState>) -> String {
    let inboxes = state.store.inboxes();
    let stored: usize = inboxes.iter().map(|(_, c)| c).sum();
    let ca_cache = ca_cache_stats();
    let ledger = state.webhooks.snapshot();
    let rows = [
        (
            "counter",
            "swarmail_emails_inserted_total",
            state.store.emails_inserted().to_string(),
        ),
        (
            "counter",
            "swarmail_emails_dropped_total",
            state.store.emails_dropped().to_string(),
        ),
        ("gauge", "swarmail_emails_stored", stored.to_string()),
        ("gauge", "swarmail_inboxes", inboxes.len().to_string()),
        // The per-PEM webhook client cache is bounded; these let an operator
        // watch the bound hold and see evictions happen (never above 32).
        (
            "gauge",
            "swarmail_webhook_ca_cache_entries",
            ca_cache.entries.to_string(),
        ),
        (
            "counter",
            "swarmail_webhook_ca_cache_builds_total",
            ca_cache.builds.to_string(),
        ),
        (
            "counter",
            "swarmail_webhook_ca_cache_evictions_total",
            ca_cache.evictions.to_string(),
        ),
        // The delivery ledger: every death a notification can die has a row,
        // all zero until the first delivery is attempted. `lagged` counts
        // mails (a lagged mail's fan-out is unknowable); the rest count
        // deliveries/targets. Cumulative since server start, exact at
        // quiescence — see `webhook::WebhookLedger`.
        (
            "counter",
            "swarmail_webhook_delivered_total",
            ledger.delivered.to_string(),
        ),
        (
            "counter",
            "swarmail_webhook_dropped_total",
            ledger.dropped.to_string(),
        ),
        (
            "counter",
            "swarmail_webhook_lagged_total",
            ledger.lagged.to_string(),
        ),
        (
            "counter",
            "swarmail_webhook_exhausted_total",
            ledger.exhausted.to_string(),
        ),
        (
            "counter",
            "swarmail_webhook_aborted_total",
            ledger.aborted.to_string(),
        ),
        // The readiness gauges (issue #51): what a test used to sleep and
        // hope for, the server states outright. `awaits_active` covers both
        // wait surfaces (REST + MCP); `deliveries_in_flight` is the SMTP
        // DATA-to-commit window (the accept == stored seam, 0001); the
        // router's request count is the "the server has my request" signal.
        // Each also renders one labeled sample per inbox with a live unit —
        // the per-test-unique row a poll can name (0004), immune to the
        // process-global mixing of parallel test servers.
    ];
    let mut out: String = rows
        .iter()
        .map(|(kind, name, value)| format!("# TYPE {name} {kind}\n{name} {value}\n"))
        .collect();
    // The readiness families, one TYPE line and one sample per label set.
    let awaits_total = awaits_active();
    out.push_str(&registry_rows(
        &AWAITS,
        "swarmail_awaits_active",
        awaits_total,
    ));
    out.push_str(&registry_rows(
        &REQUESTS,
        "swarmail_http_requests_in_flight",
        http_requests_in_flight(),
    ));
    out.push_str(&registry_rows(
        &DELIVERIES,
        "swarmail_deliveries_in_flight",
        deliveries_in_flight(),
    ));
    out
}

/// Deliveries between their DATA terminator and their insert resolving,
/// for `/metrics`.
pub fn deliveries_in_flight() -> u64 {
    registry_total(&DELIVERIES)
}

async fn list_inboxes(State(state): State<AppState>) -> Json<Vec<InboxSummary>> {
    Json(
        state
            .store
            .inboxes()
            .into_iter()
            .map(|(name, count)| InboxSummary { name, count })
            .collect(),
    )
}

fn parse_query<T: DeserializeOwned>(raw: &str, what: &str) -> ApiResult<T> {
    serde_urlencoded::from_str(raw).map_err(|e| bad_request(format!("invalid {what} query: {e}")))
}

async fn list_messages(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Json<EmailList>> {
    let q: ListQuery = parse_query(raw.0.as_deref().unwrap_or(""), "list")?;
    // The page cap is the shared refusal (decision 0030 §3), answered
    // before the scan is taken: staging before effect.
    let limit =
        checked_page_size("limit", q.limit.unwrap_or(DEFAULT_LIST_LIMIT)).map_err(bad_request)?;
    let all = state.store.list(&inbox, &q.filter());
    let total = all.len();
    let offset = q.offset.unwrap_or(0);
    let emails: Vec<Arc<Email>> = all.into_iter().skip(offset).take(limit).collect();
    Ok(Json(EmailList { total, emails }))
}

async fn count_messages(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Json<serde_json::Value>> {
    let q: ListQuery = parse_query(raw.0.as_deref().unwrap_or(""), "count")?;
    Ok(Json(serde_json::json!({
        "inbox": inbox,
        "count": state.store.count(&inbox, &q.filter()),
    })))
}

async fn await_messages(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Response> {
    let q: AwaitQuery = parse_query(raw.0.as_deref().unwrap_or(""), "await")?;
    let want = q.min_one_count();
    // The page cap is the shared refusal (decision 0030 §3), answered
    // before the wait parks: a refused await registers no waiter (0025
    // §5). `count` is the wait's trigger and is capped like a page; the
    // page itself is `limit`'s business. `assert`'s exact `count` and
    // inert `limit` stay exempt — an assertion's answer is a count, not a
    // page.
    let want = checked_page_size("count", want).map_err(bad_request)?;
    let page =
        checked_page_size("limit", q.limit.unwrap_or(DEFAULT_LIST_LIMIT)).map_err(bad_request)?;
    let timeout = q.timeout()?;
    let filter = q.filter();
    // The wait races the stop watch (decision 0018): a client parked on a
    // long timeout cannot hold `stop()` for its remainder — the moment the
    // server begins stopping, the wait answers 503 and lets the shutdown
    // proceed. `shutdown_signal` subscribes (clones) the receiver before
    // `wait_for`'s first scan; nothing is checked before subscribing.
    // The park registers on the readiness gauge first: a test sequencing
    // against this wait reads its inbox's `swarmail_awaits_active` row over
    // HTTP instead of sleeping, and the guard releases the row on every
    // resolution path.
    let _park = park_await(&inbox);
    let outcome = tokio::select! {
        outcome = state.store.wait_for(&inbox, &filter, want, timeout) => Some(outcome),
        _ = shutdown_signal(state.shutdown.clone()) => None,
    };
    // The total is counted *after* the wait, in both arms: it describes the
    // response's observed state, not a snapshot taken before the wait could
    // change it (decision 0018). The residual — inserts landing between the
    // wait's last scan and this count — is bounded by one insert and named
    // in the decision.
    let total = state.store.count(&inbox, &Filter::default());
    match outcome {
        Some(WaitOutcome::Found(matches)) => {
            // The page is `limit`'s business, exactly as on list: the wait
            // still triggers at `count` matches, the response carries at
            // most the limit's worth of them.
            let emails = matches.iter().take(page).cloned().collect();
            Ok((
                StatusCode::OK,
                Json(AwaitResponse {
                    matched: want,
                    total,
                    emails,
                }),
            )
                .into_response())
        }
        Some(WaitOutcome::Timeout { matched }) => Ok((
            StatusCode::REQUEST_TIMEOUT,
            Json(AwaitResponse {
                matched,
                total,
                emails: vec![],
            }),
        )
            .into_response()),
        // The stop won the race: the fixed wire string, the one new answer a
        // parked wait can give.
        None => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "shutting down".into(),
        )),
    }
}

async fn assert_messages(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Response> {
    let q: AwaitQuery = parse_query(raw.0.as_deref().unwrap_or(""), "assert")?;
    // `limit` is the shared filter shape (#38): parsed and type-checked
    // exactly as on list, inert here — an assertion's answer is a count,
    // not a page.
    let want = q.exact_count();
    let matched = state.store.count(&inbox, &q.filter());
    if matched == want {
        Ok((
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "inbox": inbox, "matched": matched })),
        )
            .into_response())
    } else {
        Ok((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "ok": false, "inbox": inbox, "expected": want, "matched": matched,
            })),
        )
            .into_response())
    }
}

async fn clear_inbox(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // A failed commit is the answer (decision 0014): the clear did not
    // commit, so the caller must hear a failure — `503`, the cause usually
    // transient — instead of a count for work that never happened.
    match state.store.clone().clear_async(inbox).await {
        Ok(removed) => Ok(Json(serde_json::json!({ "removed": removed }))),
        Err(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage commit failed".into(),
        )),
    }
}

async fn get_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Arc<Email>>> {
    // The store already holds the mail behind an `Arc`; the response
    // serializes it in place — a deep clone here would copy the bytes the
    // wire is byte-identical without (`raw` is skipped, decision 0030 §1).
    state
        .store
        .get(&id)
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

async fn delete_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // A failed commit is the answer (decision 0014): the delete did not
    // commit, so the mail is still there — `503`, the cause usually
    // transient — never a `200` for work that never happened. `404` stays
    // the answer for an id the store never held.
    match state.store.clone().delete_async(id).await {
        Ok(true) => Ok(Json(serde_json::json!({ "deleted": true }))),
        Ok(false) => Err(ApiError(StatusCode::NOT_FOUND, "message not found".into())),
        Err(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage commit failed".into(),
        )),
    }
}

async fn get_raw(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    state
        .store
        .get(&id)
        // The exact received bytes — no charset re-encoding, no newline
        // translation — in the one shape that owns them: message/rfc822.
        .map(|e| compat::raw_response(&e))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

async fn get_chaos(State(state): State<AppState>) -> Json<ChaosConfig> {
    Json(state.chaos.current())
}

async fn set_chaos(
    State(state): State<AppState>,
    Json(config): Json<ChaosConfig>,
) -> ApiResult<Json<ChaosConfig>> {
    // The domain rule is owned once (issue #32 slice 2, decision 0025):
    // validate before any effect — a rejected PUT never calls `Chaos::set`,
    // so the active rules are unchanged by construction, not by rollback.
    config.validate().map_err(bad_request)?;
    state.chaos.set(config);
    Ok(Json(state.chaos.current()))
}

async fn clear_chaos(State(state): State<AppState>) -> Json<ChaosConfig> {
    state.chaos.clear();
    Json(state.chaos.current())
}

// ---------- threads & feed ----------

/// The thread view of an inbox: id chains first, normalized subject as the
/// fallback, newest thread first.
async fn list_threads(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
) -> Json<Vec<threads::Thread>> {
    Json(threads::view(&state.store, &inbox))
}

/// One thread by its stable key: the conversation, oldest first.
async fn get_thread(
    State(state): State<AppState>,
    Path((inbox, key)): Path<(String, String)>,
) -> ApiResult<Json<threads::Thread>> {
    threads::view(&state.store, &inbox)
        .into_iter()
        .find(|thread| thread.key == key)
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "thread not found".into()))
}

/// The live inbox feed: server-sent events, one full thread view per frame.
/// Subscribes before the first scan (decision 0002) — see `feed::feed_stream`.
async fn inbox_feed(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(feed::feed_stream(
        state.store.clone(),
        inbox,
        state.shutdown.clone(),
    ))
    .keep_alive(KeepAlive::default())
}

// ---------- seed ----------

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    pub from: Option<String>,
    pub to: String,
    pub subject: Option<String>,
    pub text: Option<String>,
    pub html: Option<String>,
}

/// Inject a synthetic email without SMTP — full pipeline (parse, extract)
/// runs. Construction is the one shared seed policy (`build_seed_email`,
/// decision 0010); a refused fixture (a hostile envelope field) answers `400`
/// naming the field and stores nothing. The build is pure CPU with no await
/// inside, so it is offloaded like the DATA-arm parse (decision 0030 §1) —
/// the async worker never pays the HTML pipeline.
async fn seed_inbox(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    Json(seed): Json<SeedRequest>,
) -> ApiResult<Json<Arc<Email>>> {
    let email = Store::offload(move || {
        smtp::build_seed_email(
            &inbox,
            &seed.to,
            seed.from.as_deref(),
            seed.subject.as_deref(),
            seed.text.as_deref(),
            seed.html.as_deref(),
        )
    })
    .await
    .map_err(bad_request)?;
    // A failed commit refuses (decision 0012): 503 — the cause is usually
    // transient (a full volume, a read-only remount, SQLITE_BUSY), so retry
    // after repair — with the fixed text, never a leaked SQLite detail.
    // The refusal contract is the boundary's (decision 0014 §3): nothing
    // was applied — no mail, no inbox minted, no counter moved — so there
    // is nothing to query, and the client may retry losslessly; the seed's
    // body is never half-landed. During shutdown the same fixed text covers
    // the whole refusal family — a store that has begun refusing applies
    // nothing too (decision 0024) — the wire does not fork on the cause;
    // the waits' `shutting down` (0018) answers only the waits.
    match state.store.clone().insert_async(email).await {
        Ok(stored) => Ok(Json(stored)),
        Err(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage commit failed".into(),
        )),
    }
}

// ---------- webhooks ----------

/// GET returns the configured targets with the **secret redacted**
/// (decision 0029 §5): an echoed secret is the one irreversible leak in the
/// batch — a certificate is public material and `ca_pem` stays, it is the
/// diagnostic. The round-trip loss is deliberate: PUT is the only writer,
/// and the operator retains their own payload.
async fn get_webhooks(State(state): State<AppState>) -> Json<Vec<WebhookTarget>> {
    Json(
        state
            .webhooks
            .current()
            .into_iter()
            .map(|t| WebhookTarget { secret: None, ..t })
            .collect(),
    )
}

/// PUT validates every supplied value before any effect (decision 0029 §5):
/// the URL must parse with an `http`/`https` scheme and a host, the
/// `ca_pem` must load through the delivery TLS stack itself, and the
/// `secret` must be a single-line header value of at most 4096 octets. A
/// target that cannot be delivered is a 400 naming the defect — never a
/// misconfiguration discovered at delivery time — and nothing is applied.
async fn set_webhooks(
    State(state): State<AppState>,
    Json(targets): Json<Vec<WebhookTarget>>,
) -> ApiResult<Json<Vec<WebhookTarget>>> {
    for (i, target) in targets.iter().enumerate() {
        target
            .validate()
            .map_err(|e| bad_request(format!("webhook target {i}: {e}")))?;
    }
    state.webhooks.set(targets);
    Ok(Json(
        // The echoed configuration is redacted the same way as GET: the
        // caller wrote the secret, but a response body is still a log.
        state
            .webhooks
            .current()
            .into_iter()
            .map(|t| WebhookTarget { secret: None, ..t })
            .collect(),
    ))
}

async fn clear_webhooks(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.webhooks.set(vec![]);
    Json(serde_json::json!({ "cleared": true }))
}

// ---------- MCP ----------

/// The body is taken as text and parsed inside `mcp::dispatch`: the `Json`
/// extractor is gone, so a malformed body is a JSON-RPC error object with a
/// null id (HTTP 400), not a plain-text rejection (decision 0029 §2).
async fn mcp_endpoint(State(state): State<AppState>, body: String) -> Response {
    let ctx = McpContext {
        store: state.store.clone(),
        chaos: state.chaos.clone(),
        shutdown: state.shutdown.clone(),
    };
    mcp::dispatch(&ctx, &body).await
}

// ---------- machine-readable docs ----------

async fn openapi_json() -> Json<serde_json::Value> {
    Json(serde_json::from_str(include_str!("static/openapi.json")).expect("embedded openapi.json"))
}

async fn llms_txt() -> &'static str {
    include_str!("static/llms.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stop watch's two arms, probed in isolation (decision 0020): an
    /// already-fired stop resolves at once — the receiver is borrowed and
    /// updated before the check, so a stop that fired before the call is
    /// never missed behind a stale cached value.
    #[tokio::test]
    async fn an_already_fired_stop_resolves_the_wait_at_once() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        tx.send(true).expect("the stop fires");
        tokio::time::timeout(std::time::Duration::from_millis(100), shutdown_signal(rx))
            .await
            .expect("the already-fired stop resolves without parking");
    }

    /// The live arm: a stop that fires while the caller is parked resolves
    /// the wait through the watch's real wake — no poll loop, no sleep.
    #[tokio::test]
    async fn a_stop_firing_while_parked_resolves_the_wait() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let wait = tokio::spawn(shutdown_signal(rx));
        // One yield on the single-threaded runtime runs the spawned waiter
        // to its first suspension: it has observed `false` and parked on
        // `changed()` before the stop fires.
        tokio::task::yield_now().await;
        tx.send(true).expect("the stop fires");
        tokio::time::timeout(std::time::Duration::from_millis(100), wait)
            .await
            .expect("the stop resolves the parked waiter")
            .expect("the waiter completes");
    }

    /// The listing page's wire is byte-identical to the whole-clone shape it
    /// replaced — serde's `rc` support serializes `Arc<Email>` exactly as
    /// `Email` — and no `raw` key ever appears: the page holds `Arc`s
    /// precisely so the dead bytes serialization skips are never copied.
    #[test]
    fn the_listing_page_is_byte_identical_and_carries_no_raw() {
        let mut email = crate::store::email("p1", "box", "to@x.io");
        email.raw = b"raw source bytes".to_vec();
        let page = EmailList {
            total: 1,
            emails: vec![Arc::new(email.clone())],
        };
        let serialized = serde_json::to_string(&page).unwrap();
        // The emails array is indistinguishable from the Vec<Email> clone it
        // replaced, byte for byte.
        assert_eq!(
            serde_json::to_string(&vec![email.clone()]).unwrap(),
            serde_json::to_string(&page.emails).unwrap(),
            "the page's emails serialize as the Vec<Email> shape"
        );
        let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(value["total"], 1);
        assert!(value["emails"][0].get("raw").is_none(), "raw is skipped");
        assert_eq!(value["emails"][0]["id"], "p1");
    }

    /// The shared page cap check (decision 0030 §3): over the cap is the
    /// exact refusal sentence — field named, range named — never a clamp,
    /// and the boundary itself is a legal page on every plane.
    #[test]
    fn checked_page_size_refuses_over_the_cap_and_honors_the_boundary() {
        assert_eq!(
            checked_page_size("limit", MAX_PAGE_SIZE + 1),
            Err(format!("limit must be at most {MAX_PAGE_SIZE}"))
        );
        assert_eq!(
            checked_page_size("count", usize::MAX),
            Err(format!("count must be at most {MAX_PAGE_SIZE}"))
        );
        assert_eq!(checked_page_size("limit", 0), Ok(0));
        assert_eq!(checked_page_size("count", MAX_PAGE_SIZE), Ok(MAX_PAGE_SIZE));
    }

    /// The Origin guard's vocabulary (decision 0030 §2), pinned spelling by
    /// spelling: the loopback forms of the bound port pass, and every other
    /// authority — another port, a named host, a non-loopback address, an
    /// origin with no port, `null`, a schemeless string, garbage brackets —
    /// is not this plane.
    #[test]
    fn origin_spellings_of_the_bound_port() {
        let bound = 8025;
        for good in [
            "http://localhost:8025",
            "https://localhost:8025",
            "http://LOCALHOST:8025",
            "http://127.0.0.1:8025",
            // The whole /8 is loopback, not just .1.
            "http://127.255.0.9:8025",
            "http://[::1]:8025",
        ] {
            assert!(origin_is_loopback_of(good, bound), "{good} must pass");
        }
        for bad in [
            "http://localhost:8026",             // a loopback spelling of another port
            "http://localhost:99999",            // a port that cannot exist
            "http://localhost",                  // no explicit port: the scheme default
            "http://[::1]",                      // bracketed literal, no port
            "http://[::1:8025",                  // unclosed bracket
            "http://attacker.example:8025",      // a named host that is not loopback
            "http://10.0.0.9:8025",              // a private-but-not-loopback host
            "http://[::2]:8025",                 // IPv6 non-loopback
            "null",                              // a sandboxed iframe's opaque origin
            "attacker.example:8025",             // no scheme: not an origin at all
            "http://attacker.example:8025/evil", // the path is not the authority
        ] {
            assert!(!origin_is_loopback_of(bad, bound), "{bad} must refuse");
        }
    }

    /// The wait's page is byte-identical to the whole-clone shape it
    /// replaced and carries no `raw` key, exactly like the listing page:
    /// the matches come out of the store as `Arc`s and serialize in place.
    #[test]
    fn the_await_page_is_byte_identical_and_carries_no_raw() {
        let mut email = crate::store::email("w1", "box", "to@x.io");
        email.raw = b"raw source bytes".to_vec();
        let response = AwaitResponse {
            matched: 1,
            total: 1,
            emails: vec![Arc::new(email.clone())],
        };
        assert_eq!(
            serde_json::to_string(&vec![email]).unwrap(),
            serde_json::to_string(&response.emails).unwrap(),
            "the wait's emails serialize as the Vec<Email> shape"
        );
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["matched"], 1);
        assert!(value["emails"][0].get("raw").is_none(), "raw is skipped");
    }

    /// The shared cap check (decision 0025): the cap arm keeps the exact
    /// 0018 text the wire tests pin, and the boundary values are real
    /// durations — the poles are honored, never clamped.
    #[test]
    fn checked_wait_timeout_refuses_over_the_cap_and_honors_the_poles() {
        assert_eq!(
            checked_wait_timeout_ms(MAX_AWAIT_TIMEOUT_MS + 1),
            Err(format!("timeout_ms must be at most {MAX_AWAIT_TIMEOUT_MS}"))
        );
        assert_eq!(
            checked_wait_timeout_ms(u64::MAX),
            Err(format!("timeout_ms must be at most {MAX_AWAIT_TIMEOUT_MS}"))
        );
        assert_eq!(checked_wait_timeout_ms(0), Ok(Duration::from_millis(0)));
        assert_eq!(
            checked_wait_timeout_ms(MAX_AWAIT_TIMEOUT_MS),
            Ok(Duration::from_millis(MAX_AWAIT_TIMEOUT_MS))
        );
    }

    /// A minimal state for the direct healthz probes: the gauges read only
    /// the flags, the stop watch and the clock, so the cheapest instances
    /// serve and nothing here touches a store.
    fn gauge_state(stop: tokio::sync::watch::Receiver<bool>, smtp: bool, pop3: bool) -> AppState {
        AppState {
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            webhooks: Arc::new(Webhooks::default()),
            started: Instant::now(),
            shutdown: stop,
            smtp_degraded: Arc::new(AtomicBool::new(smtp)),
            pop3_degraded: Arc::new(AtomicBool::new(pop3)),
            // The healthz gauges are probe-only state; the probe runs
            // headerless, so the guard is inert here.
            http_origin_guard: None,
        }
    }

    /// Every listener accepting is `200 ok` with the uptime the old probe
    /// pinned (tests/api.rs asserts the same over the wire).
    #[tokio::test]
    async fn healthz_is_ok_when_every_listener_accepts() {
        let (_, rx) = tokio::sync::watch::channel(false);
        let (code, Json(value)) = healthz(State(gauge_state(rx, false, false))).await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(value["status"], "ok");
        assert!(value["uptime_seconds"].is_u64());
    }

    /// A dark SMTP listener is named: 503, `degraded`, the surface listed.
    #[tokio::test]
    async fn healthz_names_a_degraded_smtp_listener() {
        let (_, rx) = tokio::sync::watch::channel(false);
        let (code, Json(value)) = healthz(State(gauge_state(rx, true, false))).await;
        assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value["status"], "degraded");
        assert_eq!(value["degraded"], serde_json::json!(["smtp"]));
    }

    /// The POP3 surface names itself the same way, and both dark listeners
    /// are listed together in the router's fixed order.
    #[tokio::test]
    async fn healthz_names_the_degraded_pop3_listener_and_both_together() {
        let (_, rx) = tokio::sync::watch::channel(false);
        let (code, Json(value)) = healthz(State(gauge_state(rx, false, true))).await;
        assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value["degraded"], serde_json::json!(["pop3"]));

        let (_, rx) = tokio::sync::watch::channel(false);
        let (code, Json(value)) = healthz(State(gauge_state(rx, true, true))).await;
        assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value["degraded"], serde_json::json!(["smtp", "pop3"]));
    }

    /// Stopping wins over degraded: once the stop has fired the process is
    /// leaving either way, and the answer is `stopping` even with every
    /// listener dark.
    #[tokio::test]
    async fn healthz_reports_stopping_over_everything_else() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        tx.send(true).expect("the stop fires");
        let (code, Json(value)) = healthz(State(gauge_state(rx, true, true))).await;
        assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value["status"], "stopping");
        assert!(
            value.get("degraded").is_none(),
            "stopping names no surfaces"
        );
    }

    /// A plain-HTTP probe against a real router: status line and body.
    /// The Host names the probe's own addr — a loopback spelling of the
    /// bound port, the shape the guard admits.
    async fn http_probe(addr: std::net::SocketAddr, path: &str) -> (u16, String) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut c = tokio::net::TcpStream::connect(addr).await.unwrap();
        c.write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
        let mut buf = Vec::new();
        c.read_to_end(&mut buf).await.unwrap();
        let text = String::from_utf8_lossy(&buf).into_owned();
        // `expect`, not `unwrap_or_else(panic!)`: a closure here is a
        // function record the exact gate demands warm on every run, and
        // its only arm is a failed probe — `expect` carries the same
        // diagnostic with no cold body of our own.
        let status: u16 = text
            .split_whitespace()
            .nth(1)
            .expect("no status line")
            .parse()
            .expect("status is a number");
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    /// The wire red-proof (C36.2): the SMTP accept loop, driven by its own
    /// seam into a persistent failure, degrades /healthz on the real wire —
    /// one flag pair shared between a real `smtp::serve` and a real
    /// `api::serve`, the probe plain HTTP against the real router. Red
    /// before 0027: the hardcoded `ok` never named a dark listener.
    #[tokio::test]
    async fn a_degraded_smtp_listener_answers_503_naming_it_on_the_wire() {
        let smtp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_addr = http_listener.local_addr().unwrap();
        let smtp_degraded = Arc::new(AtomicBool::new(false));
        let pop3_degraded = Arc::new(AtomicBool::new(false));
        let state = AppState {
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            webhooks: Arc::new(Webhooks::default()),
            started: Instant::now(),
            shutdown: tokio::sync::watch::channel(false).1,
            smtp_degraded: smtp_degraded.clone(),
            pop3_degraded: pop3_degraded.clone(),
            // The red-proof's probes are headerless, and the listener is
            // loopback: the guard is live but Origin-absent requests pass.
            http_origin_guard: http_addr.ip().is_loopback().then(|| http_addr.port()),
        };
        let api_task = tokio::spawn(serve(http_listener, state, std::future::pending()));
        // The healthy baseline first, before the misbehavior exists: the
        // probe reads the hardcoded-contract `ok` off the real wire — the
        // state the red-proof is measured against.
        let (code, body) = http_probe(http_addr, "/healthz").await;
        assert_eq!(code, 200, "the healthy listener answers ok: {body}");
        let baseline: serde_json::Value =
            serde_json::from_str(&body).expect("the healthy body is json");
        assert_eq!(baseline["status"], "ok");
        assert!(
            baseline.get("degraded").is_none(),
            "the healthy baseline names no surfaces: {baseline}"
        );
        let smtp_task = tokio::spawn(crate::smtp::serve(
            smtp_listener,
            Arc::new(Store::new(0)),
            Arc::new(Chaos::default()),
            crate::smtp::SmtpConfig::default(),
            None,
            smtp_degraded.clone(),
            std::future::pending(),
        ));
        // Persistent misbehavior: far more failures than the probe can
        // outlive — the loop stays degraded for the whole test.
        crate::smtp::inject_accept_errors(1000);
        // The flag is the sequencer, not a probe retry loop (0020 rule 5):
        // once the loop has been polled into its backoff the degradation is
        // a settled fact, so ONE probe reads it — no race to schedule
        // around, no branch that only a lucky interleave takes.
        let deadline = Instant::now() + Duration::from_secs(10);
        while !smtp_degraded.load(Ordering::Relaxed) {
            assert!(
                Instant::now() < deadline,
                "the accept loop never flagged itself degraded"
            );
            tokio::task::yield_now().await;
        }
        let (code, body) = http_probe(http_addr, "/healthz").await;
        assert_eq!(code, 503, "{body}");
        assert_eq!(
            body,
            serde_json::json!({"status": "degraded", "degraded": ["smtp"]}).to_string()
        );
        // The seam is thread-local and this thread runs other tests after
        // this one: leave it as it was found, so no later serve on this
        // thread inherits a phantom failure.
        crate::smtp::inject_accept_errors(0);
        smtp_task.abort();
        api_task.abort();
    }

    /// The served contract is pinned to the constants (decision 0025):
    /// openapi.json's `ChaosRule.probability` bounds, the PUT's documented
    /// 400 and the await `timeout_ms` default/minimum/maximum all equal the
    /// domain's numbers — the fourth copy of each can drift only by
    /// failing CI.
    #[test]
    fn the_served_openapi_publishes_the_shared_domain() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("static/openapi.json")).expect("openapi.json");
        let probability = &doc["components"]["schemas"]["ChaosRule"]["properties"]["probability"];
        assert_eq!(probability["type"], "integer", "{probability}");
        assert_eq!(probability["minimum"], 0, "{probability}");
        assert_eq!(probability["maximum"], crate::chaos::PROBABILITY_MAX);

        let put_400 = &doc["paths"]["/api/v1/chaos"]["put"]["responses"]["400"]["description"];
        assert!(
            put_400
                .as_str()
                .unwrap()
                .contains(crate::chaos::PROBABILITY_REFUSAL),
            "{put_400}"
        );

        let parameters = &doc["paths"]["/api/v1/inboxes/{inbox}/await"]["get"]["parameters"];
        let timeout_ms = parameters
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "timeout_ms")
            .expect("the await documents timeout_ms")["schema"]
            .clone();
        assert_eq!(
            timeout_ms["default"], DEFAULT_WAIT_TIMEOUT_MS,
            "{timeout_ms}"
        );
        assert_eq!(timeout_ms["minimum"], 0, "{timeout_ms}");
        assert_eq!(timeout_ms["maximum"], MAX_AWAIT_TIMEOUT_MS, "{timeout_ms}");
    }

    /// The doc equals the parser (decision 0029 §1, 0025 §6 pattern): the
    /// await/assert parameter sets the document publishes are exactly the
    /// fields `AwaitQuery` accepts — the four-name filter, `limit`, `count`,
    /// and (await only) `timeout_ms` — with no `offset`, which a wait cannot
    /// have: mail arrives as it lands, there are no pages to skip.
    #[test]
    fn the_served_openapi_publishes_the_await_and_assert_parameters() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("static/openapi.json")).expect("openapi.json");
        let names = |path: &str| -> Vec<String> {
            let mut v: Vec<String> = doc["paths"][path]["get"]["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["name"].as_str().unwrap().to_string())
                .collect();
            v.retain(|n| n != "inbox");
            v.sort();
            v
        };
        assert_eq!(
            names("/api/v1/inboxes/{inbox}/await"),
            [
                "count",
                "from",
                "limit",
                "since_ms",
                "subject",
                "timeout_ms",
                "to"
            ],
            "the await's advertised set is the handler's fields"
        );
        assert_eq!(
            names("/api/v1/inboxes/{inbox}/assert"),
            ["count", "from", "limit", "since_ms", "subject", "to"],
            "the assert's advertised set is the handler's fields — no timeout, no offset"
        );

        // The parser accepts each documented name, so the document is never
        // ahead of the handler.
        for (params, has_timeout) in [
            (
                "to=a&from=b&subject=c&since_ms=1&limit=7&count=2&timeout_ms=3",
                true,
            ),
            ("to=a&from=b&subject=c&since_ms=1&limit=7&count=2", false),
        ] {
            let q: AwaitQuery = serde_urlencoded::from_str(params)
                .expect("the parser accepts every documented parameter");
            assert_eq!(q.to.as_deref(), Some("a"), "{params}");
            assert_eq!(q.from.as_deref(), Some("b"), "{params}");
            assert_eq!(q.subject.as_deref(), Some("c"), "{params}");
            assert_eq!(q.since_ms, Some(1), "{params}");
            assert_eq!(q.limit, Some(7), "{params}");
            assert_eq!(q.count, Some(2), "{params}");
            assert_eq!(q.timeout_ms.is_some(), has_timeout, "{params}");
        }

        // A supplied value of the wrong type refuses with 400 — the same
        // shared sentence `list`'s limit yields for a non-digit (#38).
        let err = serde_urlencoded::from_str::<AwaitQuery>("limit=abc")
            .expect_err("a string limit refuses");
        assert!(
            err.to_string().contains("invalid digit"),
            "the shared parse sentence names the defect: {err}"
        );
    }

    /// The labeled registry's release paths, probed in isolation (decision
    /// 0020 rule 1, branch-stable): a label survives an inner release while
    /// an outer guard still holds it, and vanishes exactly when its last
    /// unit resolves — the row's existence is the unit's liveness, the
    /// property every poll relies on. The keys are test-owned (`cov-…`),
    /// so no wire test's inbox can collide with this global.
    #[test]
    fn a_label_survives_inner_releases_and_vanishes_at_the_last_one() {
        let outer = UnitGuard::take(&AWAITS, "cov-inner");
        let inner = UnitGuard::take(&AWAITS, "cov-inner");
        drop(inner);
        assert_eq!(
            awaits_active(),
            1,
            "one live unit remains after the inner release"
        );
        let rendered = registry_rows(&AWAITS, "swarmail_awaits_active", 1);
        assert!(
            rendered.contains("swarmail_awaits_active{inbox=\"cov-inner\"} 1"),
            "the label survives an inner release: {rendered}"
        );
        drop(outer);
        assert_eq!(awaits_active(), 0, "the last release empties the gauge");
        let rendered = registry_rows(&AWAITS, "swarmail_awaits_active", 0);
        assert!(
            !rendered.contains("cov-inner"),
            "no row for a quiescent inbox: {rendered}"
        );
    }

    /// The request path's inbox scope: an inbox route names its label,
    /// anything else names none — a labeled poll must be satisfiable only
    /// by a request to the caller's own route.
    #[test]
    fn an_inbox_scoped_path_names_its_label_and_any_other_names_none() {
        assert_eq!(
            inbox_of_path("/api/v1/inboxes/cov-path/seed"),
            Some("cov-path".to_string())
        );
        assert_eq!(inbox_of_path("/metrics"), None);
        assert_eq!(inbox_of_path("/api/v1/inboxes"), None);
        assert_eq!(inbox_of_path("/"), None);
    }
}
