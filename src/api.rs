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
use crate::webhook::{WebhookTarget, WebhookTargets, Webhooks, ca_cache_stats};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, RawQuery, State};
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
    /// failed accept and cleared when the loop next observes a healthy
    /// accept: a poll that parks pending (decision 0036 §2 — an idle
    /// recovered listener is not left degraded until a client connects) or
    /// an accepted connection. `/healthz` names the surfaces that cannot
    /// currently take connections.
    pub smtp_degraded: Arc<AtomicBool>,
    pub pop3_degraded: Arc<AtomicBool>,
    /// The bound HTTP port when the HTTP listener is loopback-bound — the
    /// Origin guard's vocabulary (decision 0030 §2). `None` when the
    /// operator bound an explicit non-loopback address: the bind is then
    /// the trust statement and the Host allowlist is inactive, but a
    /// present `Origin` must still name the request's own `Host`
    /// authority (decision 0043 §2). A loopback bind serves
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

/// Waits ever registered per inbox — a cumulative counter (issue #102):
/// one increment at the single [`park_await`] site, never released. A
/// refusal that subscribed before validating (the 0025 §5 violation)
/// leaves a permanent +1 here, which the refusal triples read as a
/// non-zero delta; the released gauge cannot show it, its row is gone
/// either way. Rendered through the same per-inbox row machinery. The
/// contract, in so many words: per-process values, deltas within one
/// process — never compared across binaries or restarts.
static AWAITS_REGISTERED: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());

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
/// The cumulative registration is counted first, at this single site
/// (issue #102): a wait that parked stays registered forever, so a
/// refusal's zero delta proves the wait never subscribed.
pub(crate) fn park_await(inbox: &str) -> UnitGuard {
    raise_cumulative(&AWAITS_REGISTERED, inbox);
    UnitGuard::take(&AWAITS, inbox)
}

/// The cumulative registry is capped (the landed review's LOW residual on
/// #107): a client on the unauthenticated plane can otherwise grow one
/// BTreeMap row per distinct awaited name, forever — rows are never
/// released (a parked wait stays registered by design, the refusal side of
/// 0025 §5) and /metrics renders every row. Past [`AWAIT_REGISTRY_CAP`] a
/// new name's count folds into the reserved [`AWAIT_REGISTRY_OVERFLOW`]
/// row: the total stays exact, a name tracked before the cap keeps its own
/// exact row, a name first seen past it has none, and a refusal triple for
/// such a name reads the total row. The cap limits attribution, never
/// admission. Legit deployments name far fewer inboxes than the cap.
const AWAIT_REGISTRY_CAP: usize = 4096;

/// The reserved overflow row's label: one line feed, which
/// [`crate::model::validate_inbox`] refuses (a control, and blank), so no
/// admissible inbox can share the row (decision 0047 §2, issue #130 — the
/// old `_uncounted` was a legal name). It renders as `inbox="\n"`, the
/// Prometheus escape; a literal backslash-n name renders as `inbox="\\n"`.
const AWAIT_REGISTRY_OVERFLOW: &str = "\n";

fn raise_cumulative(registry: &Registry, inbox: &str) {
    raise_cumulative_with_cap(registry, inbox, AWAIT_REGISTRY_CAP);
}

/// The capped insert, parameterized so the cap behavior is observable by a
/// unit test on a local registry without touching the process-global one.
fn raise_cumulative_with_cap(registry: &Registry, inbox: &str, cap: usize) {
    let mut units = registry
        .lock()
        .expect("a readiness registry mutex cannot poison");
    let at_cap = units.len() >= cap;
    match units.get_mut(inbox) {
        Some(count) => *count += 1,
        None if at_cap => {
            *units
                .entry(AWAIT_REGISTRY_OVERFLOW.to_string())
                .or_insert(0) += 1;
        }
        None => {
            units.insert(inbox.to_string(), 1);
        }
    }
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
    let _total = TotalGuard::take(&HTTP_REQUESTS_IN_FLIGHT);
    next.run(req).await
}

/// Count one request against the router's total until the guard drops.
/// The constructor owns the increment and `Drop` the decrement (issue
/// #88): a bare struct literal paired a decrement with no increment and
/// wrapped the total below zero on the first request. The counter is
/// borrowed, not `'static`, so the lifecycle is provable on a fresh local
/// atomic without touching the process-global one.
struct TotalGuard<'a>(&'a AtomicU64);

impl<'a> TotalGuard<'a> {
    fn take(total: &'a AtomicU64) -> Self {
        total.fetch_add(1, Ordering::Relaxed);
        Self(total)
    }
}

impl Drop for TotalGuard<'_> {
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
//
// Off loopback (decision 0043, amending 0030 §2) the Host allowlist is
// inactive — there is no loopback plane to rebind to — but the Origin leg
// stays: a page the operator visits can still POST `text/plain` to a
// published `0.0.0.0` bind without a preflight. A present `Origin` must
// name the request's own `Host` authority.

/// The off-loopback refusal sentence (decision 0043 §4).
const ORIGIN_HOST_MISMATCH: &str = "cross-origin requests must match the Host authority";

/// The whole-plane guard: while the listener is loopback-bound, a request
/// is answered `403` before the router unless its `Host` authority is a
/// loopback spelling of the bound HTTP port and any present `Origin` is
/// one too. Off loopback, a present `Origin` must name the request's own
/// `Host` authority (decision 0043). Every method, every route: `/mcp`
/// and the UI are as drivable as the REST surface. A request without
/// `Origin` passes the Origin leg — the shape of every non-browser client
/// and of a rebound page's same-origin reads, which is why the loopback
/// Host leg exists.
async fn origin_guard(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let Some(bound) = state.http_origin_guard else {
        if req.headers().contains_key(axum::http::header::ORIGIN)
            && !origin_matches_host(req.headers())
        {
            return ApiError(StatusCode::FORBIDDEN, ORIGIN_HOST_MISMATCH.into()).into_response();
        }
        return next.run(req).await;
    };
    // A Host line that omits its port names http's default, 80 (issue
    // #92); an Origin's omitted port keeps refusing (0043 §1).
    let host_ok = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|h| authority_is_loopback_of(h, bound, Some(80)))
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
    next.run(req).await
}

/// Split an authority into its host text (an IPv6 literal keeps its
/// brackets) and its port as spelled: `Some((host, None))` only when the
/// port is genuinely omitted. A supplied port that is empty, non-decimal
/// or out of `u16` range, an unclosed bracket, or anything but `:port`
/// after `]` is `None` — malformed, never mistaken for omission (the
/// absent-versus-invalid line of decision 0025, applied to ports). The
/// stdio bridge dials by the same reading (decision 0047 §4).
pub(crate) fn split_authority(authority: &str) -> Option<(&str, Option<u16>)> {
    let (host, port) = if authority.starts_with('[') {
        // IPv6 literal: the host runs through `]`; only `:port` may follow.
        let end = authority.find(']')? + 1;
        let (host, tail) = authority.split_at(end);
        if tail.is_empty() {
            return Some((host, None));
        }
        (host, tail.strip_prefix(':')?)
    } else {
        // The port is the *last* colon; a bare host has none.
        match authority.rsplit_once(':') {
            Some(split) => split,
            None => return Some((authority, None)),
        }
    };
    Some((host, Some(parse_port(port)?)))
}

/// A port as spelled: decimal digits in `u16` range. `u16::from_str`
/// alone would admit a leading `+`.
fn parse_port(text: &str) -> Option<u16> {
    if text.bytes().all(|b| b.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

/// Whether an `authority` (the `host[:port]` of a Host line or an
/// Origin's tail) names this plane: a loopback spelling — `localhost`,
/// any `127.0.0.0/8` address, or `[::1]` — of the bound port. An omitted
/// port means `omitted`: the Host line passes `Some(80)`, http's default
/// (issue #92), so a port-80 bind admits `Host: localhost`; an Origin
/// passes `None` and still needs the port spelled (0043 §1). Hosts
/// compare case-insensitively (DNS names are case-insensitive); anything
/// else — a named host, another port, a malformed port, garbage — is not
/// this plane.
fn authority_is_loopback_of(authority: &str, bound: u16, omitted: Option<u16>) -> bool {
    let Some((host, port)) = split_authority(authority.trim()) else {
        return false;
    };
    // The bound port is the vocabulary: a loopback spelling of another
    // port is a different origin.
    if port.or(omitted) != Some(bound) {
        return false;
    }
    let host = host
        .strip_prefix('[')
        .map_or(host, |h| h.trim_end_matches(']'));
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
    authority_is_loopback_of(&authority[..end], bound, None)
}

/// Off loopback (decision 0043 §2–3): whether the request's one `Origin`
/// names the authority of its one `Host`. Both omitted ports take the
/// Origin scheme's default — http 80, https 443 — because Host carries no
/// scheme and a TLS-terminating proxy speaks plaintext to this socket;
/// then host text (ASCII case-insensitive) and port must both be equal.
/// A duplicate or non-text header, a missing Host or a malformed value on
/// either side is a mismatch. No `Forwarded`/`X-Forwarded-*` participates.
fn origin_matches_host(headers: &axum::http::HeaderMap) -> bool {
    let (Some(origin), Some(host)) = (
        sole_header(headers, axum::http::header::ORIGIN),
        sole_header(headers, axum::http::header::HOST),
    ) else {
        return false;
    };
    let Some((default, origin_host, origin_port)) = serialized_origin(origin) else {
        return false;
    };
    let Some((host, host_port)) = strict_authority(host) else {
        return false;
    };
    origin_host.eq_ignore_ascii_case(host)
        && origin_port.unwrap_or(default) == host_port.unwrap_or(default)
}

/// A header's value when it appears exactly once, as visible text.
fn sole_header(headers: &axum::http::HeaderMap, name: axum::http::HeaderName) -> Option<&str> {
    let mut values = headers.get_all(name).iter();
    match (values.next(), values.next()) {
        (Some(value), None) => value.to_str().ok(),
        _ => None,
    }
}

/// A serialized Origin (decision 0043 §2): `http://` or `https://` and an
/// authority, nothing else — no path, query, fragment or userinfo; `null`
/// and lists are not origins. Returns the scheme's default port, the host
/// text and the port as spelled.
fn serialized_origin(origin: &str) -> Option<(u16, &str, Option<u16>)> {
    let (default, authority) = match origin.strip_prefix("https://") {
        Some(authority) => (443, authority),
        None => (80, origin.strip_prefix("http://")?),
    };
    let (host, port) = strict_authority(authority)?;
    Some((default, host, port))
}

/// An authority valid for the off-loopback comparison: a bracketed IPv6
/// literal, or a non-empty registered name / IPv4 spelling of unreserved
/// characters — so userinfo (`@`), a path, a query, a fragment, a stray
/// colon or bracket, whitespace and list commas all refuse. No DNS lookup,
/// no IDNA, no alternative-spelling equivalence.
fn strict_authority(authority: &str) -> Option<(&str, Option<u16>)> {
    let (host, port) = split_authority(authority)?;
    let valid = match host.strip_prefix('[') {
        Some(literal) => literal
            .trim_end_matches(']')
            .parse::<std::net::Ipv6Addr>()
            .is_ok(),
        None => {
            !host.is_empty()
                && host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        }
    };
    valid.then_some((host, port))
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
/// labeled sample per inbox with a live unit. `kind` names the family's
/// Prometheus type — the gauges are live units, the registered waits are a
/// cumulative counter.
fn registry_rows(registry: &'static Registry, name: &str, kind: &str, total: u64) -> String {
    let units = registry
        .lock()
        .expect("a readiness registry mutex cannot poison");
    render_rows(&units, name, kind, total)
}

/// A cumulative registry's rows with the total summed from the very rows
/// rendered — one lock, one snapshot (decision 0047 §2), so a scrape's
/// unlabeled total always equals the sum of its labeled samples, the
/// overflow row's included, however many waits register meanwhile.
fn cumulative_rows(registry: &Registry, name: &str) -> String {
    let units = registry
        .lock()
        .expect("a readiness registry mutex cannot poison");
    render_rows(&units, name, "counter", units.values().sum())
}

fn render_rows(
    units: &std::collections::BTreeMap<String, u64>,
    name: &str,
    kind: &str,
    total: u64,
) -> String {
    let mut out = format!("# TYPE {name} {kind}\n{name} {total}\n");
    for (inbox, count) in units.iter() {
        out.push_str(&format!(
            "{name}{{inbox=\"{}\"}} {count}\n",
            prom_label(inbox)
        ));
    }
    out
}

/// A label value in the Prometheus text format (issue #88, decision 0043
/// §6): exactly the three escapes the format defines — backslash, double
/// quote and line feed. Every other character, a tab, a CR or any
/// Unicode, passes literally. Rust's `Debug` rendering emitted `\t` and
/// `\u{…}`, which a strict parser rejects along with the whole scrape.
fn prom_label(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
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

// Installed in an isolated current-thread runtime, consumed at serve entry
// before any await, then owned by that invocation. No last-event global.
#[cfg(test)]
thread_local! {
    static DRAIN_RECEIPT: core::cell::Cell<Option<tokio::sync::oneshot::Sender<()>>> =
        const { core::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn observe_next_drain_deadline() -> tokio::sync::oneshot::Receiver<()> {
    let (sent, received) = tokio::sync::oneshot::channel();
    DRAIN_RECEIPT.with(|slot| slot.set(Some(sent)));
    received
}

pub async fn serve(
    listener: TcpListener,
    state: AppState,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    #[cfg(test)]
    let deadline_receipt = DRAIN_RECEIPT
        .with(|slot| slot.take())
        .unwrap_or_else(|| tokio::sync::oneshot::channel().0);
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
        _ = drain => {
            #[cfg(test)]
            let _ = deadline_receipt.send(());
            Ok(())
        },
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
        .route(
            "/mcp",
            post(mcp_endpoint).layer(axum::extract::DefaultBodyLimit::max(MCP_BODY_LIMIT)),
        )
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
//
// Every native query shape is closed (decision 0044 §3, issue #91): an
// unknown key — `assert?too=` for `to=` — refuses `400` naming the key
// instead of silently dropping a filter and answering a false pass.

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
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

/// The await domain, held flat (decision 0029 §1): the struct's
/// fields *are* the advertised query parameter set — no `#[serde(flatten)]`
/// of the list shape, so an await's domain cannot silently grow `offset`
/// (a wait has no pages: mail arrives as it lands) — and, closed, an
/// `offset` the wait would ignore is refused by name (decision 0044 §3).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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

    fn timeout(&self) -> Result<Duration, ApiError> {
        // `unwrap_or` touches only genuine omission (decision 0025): a
        // supplied malformed `timeout_ms` never reaches here — the query
        // parse refused it with 400 before the handler ran. Over the cap
        // is the shared refusal (0018), never a clamp.
        checked_wait_timeout_ms(self.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS))
            .map_err(bad_request)
    }
}

/// The count domain (decision 0048 §4, issue #134): the four-name filter and
/// nothing else. A count never paginates, so `limit` and `offset` — which
/// it used to accept from the list shape and ignore — refuse by name.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountQuery {
    pub to: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since_ms: Option<i64>,
}

impl CountQuery {
    fn filter(&self) -> Filter {
        Filter {
            to: self.to.clone(),
            from: self.from.clone(),
            subject: self.subject.clone(),
            since_ms: self.since_ms,
        }
    }
}

/// The assert domain (decision 0048 §4, issue #134): the filter, the exact
/// `count`, and the inert `limit` accepted 0030 §3 keeps for parity with
/// list — parsed and type-checked, never a page. An assert never waits, so
/// `timeout_ms` — which it used to accept from the await shape and ignore —
/// refuses by name instead of promising a wait.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssertQuery {
    pub to: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since_ms: Option<i64>,
    /// Inert (0030 §3): type-checked as on list, never applied.
    pub limit: Option<usize>,
    /// The exact filtered match count expected (default 1).
    pub count: Option<usize>,
}

impl AssertQuery {
    fn filter(&self) -> Filter {
        Filter {
            to: self.to.clone(),
            from: self.from.clone(),
            subject: self.subject.clone(),
            since_ms: self.since_ms,
        }
    }

    /// ASSERT semantics (decision 0010): the exact filtered match count,
    /// zero included — `count=0` is the negative assertion. `assert` never
    /// waits and never subscribes; a snapshot is a snapshot.
    fn exact_count(&self) -> usize {
        self.count.unwrap_or(1)
    }
}

/// Empty query domain shared by native operations that take no query keys.
/// `deny_unknown_fields` makes every decoded key refuse through the same
/// route-labeled parser, before the operation reads or changes domain state.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewQuery {}

/// The inbox listing's page (decision 0044 §3, issue #91): `/api/v1/inboxes`
/// and the UI index were the only unpaged listings. `limit` defaults to the
/// shared 100 and refuses above the shared 250 cap; `offset` is uncapped.
/// The page bounds the rows returned, not the name enumeration and sort.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InboxPageQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl InboxPageQuery {
    /// The staged `(limit, offset)`: the cap refusal is the shared sentence
    /// (decision 0030 §3), answered before the names are listed.
    pub(crate) fn page(&self) -> Result<(usize, usize), String> {
        let limit = checked_page_size("limit", self.limit.unwrap_or(DEFAULT_LIST_LIMIT))?;
        Ok((limit, self.offset.unwrap_or(0)))
    }
}

/// One page of the sorted inbox names with their counts.
pub(crate) fn inbox_page(store: &Store, limit: usize, offset: usize) -> Vec<(String, usize)> {
    store
        .inboxes()
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect()
}

/// A path's inbox through the shared admission rule (decision 0044 §3): a
/// decoded blank segment refuses `400` before any read, wait or clear.
fn admit(inbox: &str) -> ApiResult<()> {
    crate::model::validate_inbox(inbox)
        .map(|_| ())
        .map_err(bad_request)
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

// Test-only (decision 0020 rule 4, the SMTP/POP3 thread-local precedent): a
// one-shot hold on the next `/healthz` handler polled on this thread. The
// handler that takes it reports its own entry, then parks until the stop has
// fired — so a test knows its real-HTTP probe is inside the handler before
// the stop, and that the handler reads the flag after it (issue #143).
// Thread-local because the router is polled on the installing test's own
// current-thread runtime; taking the hold consumes it, so nothing leaks to
// a later test on the thread.
#[cfg(test)]
thread_local! {
    static HEALTHZ_HOLD: core::cell::Cell<Option<tokio::sync::oneshot::Sender<()>>> =
        const { core::cell::Cell::new(None) };
}

/// Install the hold; the receiver resolves once a handler has entered.
#[cfg(test)]
fn hold_next_healthz() -> tokio::sync::oneshot::Receiver<()> {
    let (entered, observed) = tokio::sync::oneshot::channel();
    HEALTHZ_HOLD.with(|hold| hold.set(Some(entered)));
    observed
}

/// Take an installed hold, if any: report entry, park until the stop.
#[cfg(test)]
async fn healthz_hold(state: &AppState) {
    if let Some(entered) = HEALTHZ_HOLD.with(|hold| hold.take()) {
        entered.send(()).expect("the holding test awaits the entry");
        shutdown_signal(state.shutdown.clone()).await;
    }
}

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
    #[cfg(test)]
    healthz_hold(&state).await;
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
        "gauge",
        awaits_total,
    ));
    // The waits ever registered, per inbox (issue #102): the refusal side
    // of 0025 §5, observable — a refused call leaves the delta at zero.
    out.push_str(&cumulative_rows(
        &AWAITS_REGISTERED,
        "swarmail_awaits_registered_total",
    ));
    out.push_str(&registry_rows(
        &REQUESTS,
        "swarmail_http_requests_in_flight",
        "gauge",
        http_requests_in_flight(),
    ));
    out.push_str(&registry_rows(
        &DELIVERIES,
        "swarmail_deliveries_in_flight",
        "gauge",
        deliveries_in_flight(),
    ));
    out
}

/// Deliveries between their DATA terminator and their insert resolving,
/// for `/metrics`.
pub fn deliveries_in_flight() -> u64 {
    registry_total(&DELIVERIES)
}

async fn list_inboxes(
    State(state): State<AppState>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Json<Vec<InboxSummary>>> {
    let q: InboxPageQuery = parse_query(raw.0.as_deref().unwrap_or(""), "inboxes")?;
    let (limit, offset) = q.page().map_err(bad_request)?;
    Ok(Json(
        inbox_page(&state.store, limit, offset)
            .into_iter()
            .map(|(name, count)| InboxSummary { name, count })
            .collect(),
    ))
}

pub(crate) fn parse_query<T: DeserializeOwned>(raw: &str, what: &str) -> ApiResult<T> {
    serde_urlencoded::from_str(raw).map_err(|e| bad_request(format!("invalid {what} query: {e}")))
}

async fn list_messages(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Json<EmailList>> {
    admit(&inbox)?;
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
    admit(&inbox)?;
    let q: CountQuery = parse_query(raw.0.as_deref().unwrap_or(""), "count")?;
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
    admit(&inbox)?;
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
    admit(&inbox)?;
    let q: AssertQuery = parse_query(raw.0.as_deref().unwrap_or(""), "assert")?;
    // `limit` is the shared filter shape (#38): parsed and type-checked
    // exactly as on list, inert here — an assertion's answer is a count,
    // not a page (0030 §3, kept by 0048 §4).
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
    raw: RawQuery,
) -> ApiResult<Json<serde_json::Value>> {
    // A failed commit is the answer (decision 0014): the clear did not
    // commit, so the caller must hear a failure — `503`, the cause usually
    // transient — instead of a count for work that never happened. A blank
    // inbox refuses first (decision 0044 §3): it clears nothing.
    admit(&inbox)?;
    let ViewQuery {} = parse_query(
        raw.0.as_deref().unwrap_or(""),
        "DELETE /api/v1/inboxes/{inbox}/messages",
    )?;
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
    raw: RawQuery,
) -> ApiResult<Response> {
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "GET /api/v1/messages/{id}")?;
    // The store already holds the mail behind an `Arc`; the response
    // serializes it in place — a deep clone here would copy the bytes the
    // wire is byte-identical without (`raw` is skipped, decision 0030 §1).
    // The serialization finishes on the blocking pool (decision 0049 §7):
    // a 30 MiB body serialized by Axum after the handler returned would
    // occupy the async worker the SMTP greeting needs.
    compat::render_by_id(&state, id, |e| Json(e).into_response()).await
}

async fn delete_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    raw: RawQuery,
) -> ApiResult<Json<serde_json::Value>> {
    let ViewQuery {} = parse_query(
        raw.0.as_deref().unwrap_or(""),
        "DELETE /api/v1/messages/{id}",
    )?;
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

async fn get_raw(
    State(state): State<AppState>,
    Path(id): Path<String>,
    raw: RawQuery,
) -> ApiResult<Response> {
    let ViewQuery {} = parse_query(
        raw.0.as_deref().unwrap_or(""),
        "GET /api/v1/messages/{id}/raw",
    )?;
    // The exact received bytes — no charset re-encoding, no newline
    // translation — in the one shape that owns them: message/rfc822, copied
    // on the blocking pool (decision 0049 §7).
    compat::render_by_id(&state, id, compat::raw_response).await
}

async fn get_chaos(State(state): State<AppState>, raw: RawQuery) -> ApiResult<Json<ChaosConfig>> {
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "GET /api/v1/chaos")?;
    Ok(Json(state.chaos.current()))
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

async fn clear_chaos(State(state): State<AppState>, raw: RawQuery) -> ApiResult<Json<ChaosConfig>> {
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "DELETE /api/v1/chaos")?;
    state.chaos.clear();
    Ok(Json(state.chaos.current()))
}

// ---------- threads & feed ----------

/// The thread view of an inbox: id chains first, normalized subject as the
/// fallback, newest thread first.
async fn list_threads(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Json<Vec<threads::Thread>>> {
    admit(&inbox)?;
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "threads")?;
    Ok(Json(threads::view(&state.store, &inbox)))
}

/// One thread by its stable key: the conversation, oldest first.
async fn get_thread(
    State(state): State<AppState>,
    Path((inbox, key)): Path<(String, String)>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Json<threads::Thread>> {
    admit(&inbox)?;
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "thread")?;
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
    raw: axum::extract::RawQuery,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    // A blank inbox refuses before the feed subscribes (decision 0044 §3),
    // and so does any query key (0048 §4): the feed takes none.
    admit(&inbox)?;
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "feed")?;
    Ok(Sse::new(feed::feed_stream(
        state.store.clone(),
        inbox,
        state.shutdown.clone(),
    ))
    .keep_alive(KeepAlive::default()))
}

// ---------- seed ----------

/// The REST seed body. The optional strings take their defaults only when
/// omitted: a present `null` is supplied and refuses with the field named —
/// the same answer MCP's seed gives (decision 0025 §2, 0044 §3; issue #91).
/// The key set is closed (decision 0048 §2, issue #134): a typo such as
/// `subjet` refuses by name instead of storing the default subject.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedRequest {
    #[serde(default, deserialize_with = "crate::model::present_string")]
    pub from: Option<String>,
    pub to: String,
    #[serde(default, deserialize_with = "crate::model::present_string")]
    pub subject: Option<String>,
    #[serde(default, deserialize_with = "crate::model::present_string")]
    pub text: Option<String>,
    #[serde(default, deserialize_with = "crate::model::present_string")]
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
    raw: RawQuery,
    body: Result<Json<SeedRequest>, JsonRejection>,
) -> Result<Json<Arc<Email>>, BodyError> {
    let seed = json_body(body, "seed")?;
    let ViewQuery {} = parse_query(
        raw.0.as_deref().unwrap_or(""),
        "POST /api/v1/inboxes/{inbox}/seed",
    )?;
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
        )
        .into()),
    }
}

/// A seed or webhook refusal: the native `{"error": …}` envelope, or an
/// extractor rejection forwarded exactly as axum shaped it.
pub enum BodyError {
    Native(ApiError),
    Extractor(JsonRejection),
}

impl From<ApiError> for BodyError {
    fn from(e: ApiError) -> Self {
        BodyError::Native(e)
    }
}

impl IntoResponse for BodyError {
    fn into_response(self) -> Response {
        match self {
            BodyError::Native(e) => e.into_response(),
            BodyError::Extractor(rejection) => rejection.into_response(),
        }
    }
}

/// The seed and webhook bodies' extraction (decision 0048 §2, amending
/// 0025/0029's extractor posture for these two routes only): a JSON *data*
/// error — an unknown key, a present `null`, a wrong type, a missing field,
/// a wrong top-level shape — is a refusal of a supplied value, so it answers
/// the native `400 {"error": …}` naming the field, where axum's extractor
/// answered 422. Every other rejection — malformed JSON 400, a missing or
/// wrong media type 415, an oversized body 413 — is forwarded exactly as
/// the extractor shaped it. Chaos and every other route keep the
/// extractor's own mapping.
fn json_body<T>(body: Result<Json<T>, JsonRejection>, what: &str) -> Result<T, BodyError> {
    match body {
        Ok(Json(value)) => Ok(value),
        Err(JsonRejection::JsonDataError(e)) => {
            Err(bad_request(format!("invalid {what} body: {}", e.body_text())).into())
        }
        Err(other) => Err(BodyError::Extractor(other)),
    }
}

// ---------- webhooks ----------

/// GET returns the configured targets **redacted** (decision 0029 §5, as
/// amended by 0041 §3): the secret is withheld and the URL loses its
/// userinfo — an echoed credential is the one irreversible leak in the
/// batch. A certificate is public material and `ca_pem` stays, it is the
/// diagnostic. The round-trip loss is deliberate: PUT is the only writer,
/// and the operator retains their own payload.
async fn get_webhooks(
    State(state): State<AppState>,
    raw: RawQuery,
) -> ApiResult<Json<Vec<WebhookTarget>>> {
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "GET /api/v1/webhooks")?;
    Ok(Json(
        state
            .webhooks
            .current()
            .iter()
            .map(WebhookTarget::redacted)
            .collect(),
    ))
}

/// PUT validates every supplied value before any effect (decision 0029 §5):
/// the URL must parse with an `http`/`https` scheme and a host, the
/// `ca_pem` must load through the delivery TLS stack itself, and the
/// `secret` must be an HTTP header value of at most 4096 octets with no
/// leading or trailing space or tab (decision 0041 §1). A target that
/// cannot be delivered is a 400 naming the defect — never a
/// misconfiguration discovered at delivery time — and nothing is applied.
async fn set_webhooks(
    State(state): State<AppState>,
    body: Result<Json<WebhookTargets>, JsonRejection>,
) -> Result<Json<Vec<WebhookTarget>>, BodyError> {
    // The closed shape first (decision 0048 §2): an unknown key, a `null`
    // or a wrong kind refuses 400 by path and kind — never by value, so no
    // credential is echoed — before any target is looked at.
    let WebhookTargets(targets) = json_body(body, "webhooks")?;
    // Every target validates — its CA roots built into a throwaway client
    // (0048 §1) — before the one swap below: a refused list replaces
    // nothing, however many valid targets precede the bad one.
    for (i, target) in targets.iter().enumerate() {
        target
            .validate()
            .map_err(|e| bad_request(format!("webhook target {i}: {e}")))?;
    }
    state.webhooks.set(targets);
    // The echoed configuration is redacted the same way as GET: the caller
    // wrote the credentials, but a response body is still a log.
    Ok(Json(
        state
            .webhooks
            .current()
            .iter()
            .map(WebhookTarget::redacted)
            .collect(),
    ))
}

async fn clear_webhooks(
    State(state): State<AppState>,
    raw: RawQuery,
) -> ApiResult<Json<serde_json::Value>> {
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "DELETE /api/v1/webhooks")?;
    state.webhooks.set(vec![]);
    Ok(Json(serde_json::json!({ "cleared": true })))
}

// ---------- MCP ----------

/// The largest `POST /mcp` body: axum's default body limit, named so the
/// refusal can say it (decision 0044 §1). The limit itself stays axum's
/// streaming one — an oversized body is never read to its end to learn its
/// size, chunked or not.
pub(crate) const MCP_BODY_LIMIT: usize = 2_097_152;

/// The cause axum's extractor gives a body over [`MCP_BODY_LIMIT`]: the
/// stdio bridge refuses an oversized body locally in these same words
/// (decision 0047 §4), pinned against a real 413 by the bridge's suite.
pub(crate) const MCP_BODY_TOO_LARGE: &str =
    "Failed to buffer the request body: length limit exceeded";

/// The one wording of a refused `POST /mcp` body: the cause, then the
/// named limit — the server's 413 and the bridge's local refusal alike.
pub(crate) fn mcp_body_refusal(cause: &str) -> String {
    format!("request body refused: {cause} (the limit is {MCP_BODY_LIMIT} bytes)")
}

/// The body is taken as bounded bytes and parsed inside `mcp::dispatch`:
/// malformed JSON and invalid UTF-8 alike are a JSON-RPC `-32700` with a
/// null id (HTTP 400), not a plain-text rejection (decisions 0029 §2, 0044
/// §1). A body the extractor refuses — over the limit (413), or one that
/// could not be read — keeps the extractor's status and becomes a null-id
/// `-32600` naming the limit and the cause, never axum's plain text.
async fn mcp_endpoint(
    State(state): State<AppState>,
    body: Result<axum::body::Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let body = match body {
        Ok(body) => body,
        Err(rejection) => {
            let message = mcp_body_refusal(&rejection.body_text());
            return (
                rejection.status(),
                Json(mcp::error_object(-32600, &message, serde_json::Value::Null)),
            )
                .into_response();
        }
    };
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
    use super::{
        AWAIT_REGISTRY_CAP, AWAIT_REGISTRY_OVERFLOW, Registry, cumulative_rows, prom_label,
        raise_cumulative_with_cap,
    };

    /// A Prometheus label value decoded by the text format's own three
    /// escapes — written here independently of `prom_label`, so the
    /// distinction below is the format's, not the encoder's say-so.
    fn decode_label(escaped: &str) -> String {
        let mut out = String::new();
        let mut chars = escaped.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next().unwrap_or('\\') {
                'n' => out.push('\n'),
                other => out.push(other),
            }
        }
        out
    }

    /// The overflow row can be no inbox's (decision 0047 §2, issue #130;
    /// red before: `_uncounted` was both a legal inbox and the overflow
    /// label): the sentinel is one line feed, which the shared inbox rule
    /// refuses, while `_uncounted` and the literal two-character `\n` stay
    /// legal — and after encoding all three samples are distinct, each
    /// decoding back to its own name.
    #[test]
    fn overflow_is_inadmissible_and_distinct_after_prometheus_encoding() {
        assert_eq!(AWAIT_REGISTRY_OVERFLOW, "\n");
        assert!(crate::model::validate_inbox(AWAIT_REGISTRY_OVERFLOW).is_err());
        let legal = ["_uncounted", r"\n"];
        for name in legal {
            assert_eq!(crate::model::validate_inbox(name), Ok(name));
        }
        let encoded: Vec<String> = [AWAIT_REGISTRY_OVERFLOW, legal[0], legal[1]]
            .into_iter()
            .map(prom_label)
            .collect();
        assert_eq!(encoded, [r"\n", "_uncounted", r"\\n"]);
        for (escaped, name) in encoded
            .iter()
            .zip([AWAIT_REGISTRY_OVERFLOW, legal[0], legal[1]])
        {
            assert_eq!(decode_label(escaped), name);
        }
    }

    /// Past the production cap a real `_uncounted` keeps an exact row of
    /// its own (issue #130's reproduction, on a local registry so the
    /// process-global one stays untouched): tracked before the cap, it
    /// counts only its own registrations; names first seen past the cap —
    /// `_uncounted` would be one too — fold, exactly, into the one
    /// overflow row. 4096 real rows plus the overflow, never more.
    #[test]
    fn tracked_uncounted_stays_exact_after_registry_capacity() {
        let registry: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());
        raise_cumulative_with_cap(&registry, "_uncounted", AWAIT_REGISTRY_CAP);
        for n in 1..AWAIT_REGISTRY_CAP {
            raise_cumulative_with_cap(&registry, &format!("box-{n}"), AWAIT_REGISTRY_CAP);
        }
        for late in ["late-1", "late-2", "late-3"] {
            raise_cumulative_with_cap(&registry, late, AWAIT_REGISTRY_CAP);
        }
        raise_cumulative_with_cap(&registry, "_uncounted", AWAIT_REGISTRY_CAP);
        raise_cumulative_with_cap(&registry, "box-1", AWAIT_REGISTRY_CAP);
        let units = registry.lock().expect("test registry cannot poison");
        assert_eq!(units.get("_uncounted"), Some(&2), "only its own two waits");
        assert_eq!(
            units.get("box-1"),
            Some(&2),
            "a tracked name keeps counting"
        );
        assert_eq!(
            units.get("late-1"),
            None,
            "a name first seen past the cap folds"
        );
        assert_eq!(
            units.get(AWAIT_REGISTRY_OVERFLOW),
            Some(&3),
            "folded exactly"
        );
        assert_eq!(units.len(), AWAIT_REGISTRY_CAP + 1);
        assert_eq!(
            units.values().sum::<u64>(),
            AWAIT_REGISTRY_CAP as u64 + 5,
            "every registration counted once"
        );
    }

    /// The cumulative scrape is one snapshot (decision 0047 §2): the
    /// unlabeled total is summed from the rows rendered beside it, so it
    /// equals the sum of the labeled samples — the overflow's `inbox="\n"`
    /// included.
    #[test]
    fn cumulative_scrape_uses_one_registry_snapshot() {
        let registry: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());
        for name in ["a", "b", "a", "c", "d", "_uncounted"] {
            raise_cumulative_with_cap(&registry, name, 3);
        }
        let rendered = cumulative_rows(&registry, "t");
        assert_eq!(
            rendered,
            "# TYPE t counter\nt 6\nt{inbox=\"\\n\"} 2\nt{inbox=\"a\"} 2\nt{inbox=\"b\"} 1\n\
             t{inbox=\"c\"} 1\n"
        );
    }

    /// The production cap is the documented constant.
    #[test]
    fn the_production_cap_is_4096() {
        assert_eq!(AWAIT_REGISTRY_CAP, 4096);
    }

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

    /// The authority split keeps absence and malformation apart (issue
    /// #92, decision 0043 §1): only a genuinely omitted port is `None`; an
    /// empty, signed, non-decimal or overflowing port and every malformed
    /// bracket tail are refusals, never a default in disguise.
    #[test]
    fn the_authority_split_tells_an_omitted_port_from_a_malformed_one() {
        assert_eq!(split_authority("localhost"), Some(("localhost", None)));
        assert_eq!(
            split_authority("localhost:8025"),
            Some(("localhost", Some(8025)))
        );
        assert_eq!(split_authority("[::1]"), Some(("[::1]", None)));
        assert_eq!(split_authority("[::1]:80"), Some(("[::1]", Some(80))));
        for malformed in [
            "localhost:",      // empty port
            "localhost:+80",   // u16::from_str would admit the sign
            "localhost:8o",    // non-decimal
            "localhost:65536", // out of range
            "[::1",            // unclosed bracket
            "[::1]x",          // garbage after the bracket
            "[::1]:",          // empty port after the bracket
            "[::1]:99999",     // out of range after the bracket
        ] {
            assert_eq!(split_authority(malformed), None, "{malformed}");
        }
        assert_eq!(parse_port("0"), Some(0));
        assert_eq!(parse_port("65535"), Some(65535));
        assert_eq!(parse_port(""), None);
    }

    /// The loopback Host leg's port default (issue #92): a portless Host
    /// is http's 80, so it names a port-80 plane and no other, while a
    /// malformed supplied port never rides the default; the Origin leg
    /// passes no default and keeps refusing a portless origin (0043 §1).
    #[test]
    fn a_portless_host_means_80_and_a_portless_origin_still_refuses() {
        for host in ["localhost", "LOCALHOST", "127.0.0.1", "[::1]"] {
            assert!(authority_is_loopback_of(host, 80, Some(80)), "{host}");
            assert!(!authority_is_loopback_of(host, 8025, Some(80)), "{host}");
        }
        assert!(authority_is_loopback_of("localhost:80", 80, Some(80)));
        assert!(authority_is_loopback_of(" localhost:8025 ", 8025, Some(80)));
        for refused in [
            "localhost:",
            "localhost:abc",
            "localhost:65616", // overflow, not 80 wrapped
            "[::1]x",
            "[::1",
            "attacker.example",
            "10.0.0.9",
            "",
        ] {
            assert!(
                !authority_is_loopback_of(refused, 80, Some(80)),
                "{refused:?}"
            );
        }
        assert!(!origin_is_loopback_of("http://localhost", 80));
        assert!(origin_is_loopback_of("http://localhost:80", 80));
    }

    /// A header map built from `(name, value)` octets — `from_bytes` so a
    /// value that is not visible text can be named.
    fn headers(pairs: &[(&str, &[u8])]) -> axum::http::HeaderMap {
        let mut map = axum::http::HeaderMap::new();
        for (name, value) in pairs {
            map.append(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                axum::http::HeaderValue::from_bytes(value).unwrap(),
            );
        }
        map
    }

    /// The off-loopback authority comparison (decision 0043 §2–3), case
    /// by case: matching authorities pass with omitted ports normalized by
    /// the Origin scheme; a mismatch, a duplicate, non-text, a missing
    /// Host and every malformed authority class refuse — including the
    /// spellings the HTTP parser would reject before the router.
    #[test]
    fn the_off_loopback_origin_must_name_the_host_authority() {
        let matches = |origin: &[u8], host: &[u8]| {
            origin_matches_host(&headers(&[("origin", origin), ("host", host)]))
        };
        for (origin, host) in [
            ("http://mail.example:8025", "mail.example:8025"),
            ("http://MAIL.example:8025", "mail.EXAMPLE:8025"),
            ("http://[::1]:8025", "[::1]:8025"),
            ("http://mail.example", "mail.example"),
            ("http://mail.example", "mail.example:80"),
            ("http://mail.example:80", "mail.example"),
            ("https://mail.example", "mail.example"),
            ("https://mail.example", "mail.example:443"),
            ("https://mail.example:443", "mail.example"),
            ("http://10.0.0.9:8025", "10.0.0.9:8025"),
        ] {
            assert!(
                matches(origin.as_bytes(), host.as_bytes()),
                "{origin} + Host {host} must pass"
            );
        }
        for (origin, host) in [
            ("https://evil.example", "mail.example"),
            ("http://mail.example:8025", "mail.example:8026"),
            ("https://mail.example", "mail.example:8025"),
            ("https://mail.example:8025", "mail.example"),
            ("http://mail.example", "mail.example:443"),
            ("null", "mail.example"),
            ("mail.example", "mail.example"),
            ("ftp://mail.example", "mail.example"),
            ("HTTP://mail.example", "mail.example"),
            ("https://", "mail.example"),
            ("https://user@mail.example", "mail.example"),
            ("https://mail.example/", "mail.example"),
            ("https://mail.example/path", "mail.example"),
            ("https://mail.example?q", "mail.example"),
            ("https://mail.example#f", "mail.example"),
            ("https://mail.example:443/x", "mail.example"),
            ("https://a.example, https://mail.example", "mail.example"),
            ("https://mail.example:", "mail.example"),
            ("https://mail.example:+443", "mail.example"),
            ("https://mail.example:99999", "mail.example"),
            ("http://[::1", "[::1]"),
            ("http://[::1]x", "[::1]"),
            ("http://[zz]", "[zz]"),
            ("http://[]", "[]"),
            ("http://a:b:8025", "a:b:8025"),
            ("https://mail.example", ""),
            ("https://mail.example", "mail.example:"),
            ("https://mail.example", "user@mail.example"),
        ] {
            assert!(
                !matches(origin.as_bytes(), host.as_bytes()),
                "{origin} + Host {host:?} must refuse"
            );
        }
        // Non-text header octets on either side.
        assert!(!matches(
            &b"https://mail.\xffexample"[..],
            &b"mail.example"[..]
        ));
        assert!(!matches(
            &b"https://mail.example"[..],
            &b"mail.\xffexample"[..]
        ));
        // A missing Host, a duplicate Origin and a duplicate Host.
        let origin = "https://mail.example".as_bytes();
        let host = "mail.example".as_bytes();
        assert!(!origin_matches_host(&headers(&[("origin", origin)])));
        assert!(!origin_matches_host(&headers(&[
            ("origin", origin),
            ("origin", origin),
            ("host", host),
        ])));
        assert!(!origin_matches_host(&headers(&[
            ("origin", origin),
            ("host", host),
            ("host", host),
        ])));
        assert!(origin_matches_host(&headers(&[
            ("origin", origin),
            ("host", host)
        ])));
    }

    /// The label renderer emits exactly the Prometheus text format's three
    /// escapes and passes every other character literally (issue #88): the
    /// byte fixtures pin the backslash, quote and LF escapes and the
    /// literal tab, CR and Unicode that `Debug` used to mangle.
    #[test]
    fn label_values_take_exactly_the_three_prometheus_escapes() {
        assert_eq!(prom_label("plain-box"), "plain-box");
        assert_eq!(prom_label("a\\b"), "a\\\\b");
        assert_eq!(prom_label("a\"b"), "a\\\"b");
        assert_eq!(prom_label("a\nb"), "a\\nb");
        assert_eq!(prom_label("a\tb"), "a\tb");
        assert_eq!(prom_label("a\rb"), "a\rb");
        assert_eq!(prom_label("posté✉"), "posté✉");
        assert_eq!(
            prom_label("tab\tname\\\"q\n"),
            "tab\tname\\\\\\\"q\\n",
            "the issue's hostile name renders with legal escapes only"
        );
    }

    /// Every registry family renders through the one encoder: a local
    /// registry holding a hostile name renders the same escaped sample
    /// under each family's name and kind.
    #[test]
    fn every_registry_family_renders_through_the_label_encoder() {
        static LOCAL: Registry = std::sync::Mutex::new(std::collections::BTreeMap::new());
        let guard = UnitGuard::take(&LOCAL, "q\"\\\n\t");
        for (name, kind) in [
            ("swarmail_awaits_active", "gauge"),
            ("swarmail_awaits_registered_total", "counter"),
            ("swarmail_http_requests_in_flight", "gauge"),
            ("swarmail_deliveries_in_flight", "gauge"),
        ] {
            let escaped = "q\\\"\\\\\\n\t";
            assert_eq!(
                registry_rows(&LOCAL, name, kind, 1),
                format!("# TYPE {name} {kind}\n{name} 1\n{name}{{inbox=\"{escaped}\"}} 1\n")
            );
        }
        drop(guard);
    }

    /// The router's total is owned by its guard (issue #88): the
    /// constructor increments and `Drop` decrements, on a fresh local
    /// atomic — no reset of the process-global total — across a normal
    /// drop, a cancelled future that holds the guard, and an unwind.
    #[test]
    fn the_total_guard_counts_up_on_take_and_down_on_every_release() {
        let total = AtomicU64::new(0);
        let guard = TotalGuard::take(&total);
        assert_eq!(total.load(Ordering::Relaxed), 1, "take raises the total");
        drop(guard);
        assert_eq!(total.load(Ordering::Relaxed), 0, "drop releases it");

        // Cancellation: a pending future owning the guard is destroyed.
        let held = TotalGuard::take(&total);
        let mut pending = Box::pin(std::future::poll_fn(move |_| {
            let _owned = &held;
            std::task::Poll::<()>::Pending
        }));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(pending.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            total.load(Ordering::Relaxed),
            1,
            "the parked future holds it"
        );
        drop(pending);
        assert_eq!(total.load(Ordering::Relaxed), 0, "cancellation releases it");

        // Unwind: a panic while the guard is live still releases it.
        let unwound = std::panic::catch_unwind::<_, ()>(std::panic::AssertUnwindSafe(|| {
            let _guard = TotalGuard::take(&total);
            assert_eq!(total.load(Ordering::Relaxed), 1);
            std::panic::resume_unwind(Box::new("the handler unwinds"))
        }));
        assert!(unwound.is_err(), "the closure unwound");
        assert_eq!(total.load(Ordering::Relaxed), 0, "unwind releases it");
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
            // headerless, so the guard has nothing to refuse here.
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

    /// The stopping answer on the wire, unconditionally (issue #143,
    /// decision 0027 §2 as clarified): a real-HTTP `/healthz` that is known
    /// to be *inside its handler* when the stop fires is answered `503
    /// {"status":"stopping"}` before the graceful drain closes its socket.
    /// The order is established, not hoped for: the installed hold reports
    /// the handler's entry before the stop is sent and parks the handler
    /// until the stop has fired, so it reads the flag after the stop. A
    /// stop that drops the in-flight request — an EOF or a reset where the
    /// answer belongs — fails the read or the status assertion; nothing
    /// here is skipped. The read is bounded by the drain deadline itself,
    /// the server's own fence: the answer must be written inside it.
    #[tokio::test]
    async fn a_healthz_inside_its_handler_at_the_stop_is_answered_503_stopping() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (stop, stop_rx) = tokio::sync::watch::channel(false);
        let mut graceful = stop_rx.clone();
        let server = tokio::spawn(serve(
            listener,
            gauge_state(stop_rx, false, false),
            async move {
                let _ = graceful.changed().await;
            },
        ));
        let entered = hold_next_healthz();
        let mut probe = tokio::net::TcpStream::connect(addr).await.unwrap();
        probe
            .write_all(
                format!("GET /healthz HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .unwrap();
        tokio::time::timeout(DRAIN_DEADLINE, entered)
            .await
            .expect("the probe never reached the healthz handler")
            .expect("the hold reports its entry");

        stop.send(true).expect("the stop fires");
        let mut reply = Vec::new();
        tokio::time::timeout(DRAIN_DEADLINE, probe.read_to_end(&mut reply))
            .await
            .expect("no answer inside the drain deadline")
            .expect("the in-flight probe was reset instead of answered");
        let reply = String::from_utf8(reply).expect("an ASCII answer");
        assert!(
            reply.starts_with("HTTP/1.1 503 "),
            "the in-flight probe must be answered 503: {reply:?}"
        );
        assert_eq!(
            reply.split_once("\r\n\r\n").map(|(_, body)| body),
            Some(r#"{"status":"stopping"}"#),
            "{reply:?}"
        );
        assert!(
            openapi_documents("/healthz", "get", 503),
            "the stopping 503 is in the published response map"
        );
        tokio::time::timeout(DRAIN_DEADLINE, server)
            .await
            .expect("the drain ends once the probe is answered")
            .expect("the serve task completes")
            .expect("the serve returns cleanly");
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
        // The published contract carries both observed answers (issue #142,
        // D23): the healthy 200 and this degraded 503 are each in the
        // operation's response map — red when the 503 entry is removed.
        for observed in [200, code] {
            assert!(
                openapi_documents("/healthz", "get", observed),
                "GET /healthz answered {observed}, which openapi.json must list"
            );
        }
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
        let q: AwaitQuery = serde_urlencoded::from_str(
            "to=a&from=b&subject=c&since_ms=1&limit=7&count=2&timeout_ms=3",
        )
        .expect("the await parser accepts every documented parameter");
        assert_eq!(
            (q.to.as_deref(), q.from.as_deref(), q.subject.as_deref()),
            (Some("a"), Some("b"), Some("c"))
        );
        assert_eq!((q.since_ms, q.limit, q.count), (Some(1), Some(7), Some(2)));
        assert_eq!(q.timeout_ms, Some(3));
        let q: AssertQuery =
            serde_urlencoded::from_str("to=a&from=b&subject=c&since_ms=1&limit=7&count=2")
                .expect("the assert parser accepts every documented parameter");
        assert_eq!(
            (q.to.as_deref(), q.from.as_deref(), q.subject.as_deref()),
            (Some("a"), Some("b"), Some("c"))
        );
        assert_eq!((q.since_ms, q.limit, q.count), (Some(1), Some(7), Some(2)));
        // …and refuses what the document leaves out (decision 0048 §4): an
        // assert never waits.
        let err = serde_urlencoded::from_str::<AssertQuery>("timeout_ms=1")
            .expect_err("an assert timeout refuses");
        assert!(err.to_string().contains("`timeout_ms`"), "{err}");

        // A supplied value of the wrong type refuses with 400 — the same
        // shared sentence `list`'s limit yields for a non-digit (#38).
        let err = serde_urlencoded::from_str::<AwaitQuery>("limit=abc")
            .expect_err("a string limit refuses");
        assert!(
            err.to_string().contains("invalid digit"),
            "the shared parse sentence names the defect: {err}"
        );
    }

    /// Whether the embedded openapi.json lists `status` in the response map
    /// of `method` on `path` — the wire tests' link from an observed answer
    /// to the published operation (issue #142, D23).
    fn openapi_documents(path: &str, method: &str, status: u16) -> bool {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("static/openapi.json")).expect("openapi.json");
        doc["paths"][path][method]["responses"]
            .get(status.to_string().as_str())
            .is_some()
    }

    /// The await and assert page domains as published equal the parsers
    /// and constants that apply them (issue #142, D22): await's `limit` is
    /// the shared list page — default 100, 0 legal, capped at 250 — and its
    /// `count` defaults to one, accepts 0 as a minimum-one wait and caps at
    /// 250; assert's `limit` is type-checked but inert — no default, no cap,
    /// any non-negative integer — and its exact `count` defaults to one and
    /// keeps 0. Red on the old 50/minimum-1 values. tests/api.rs proves the
    /// same numbers on the wire.
    #[test]
    fn the_served_openapi_publishes_the_await_and_assert_domains() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("static/openapi.json")).expect("openapi.json");
        let schema = |path: &str, name: &str| -> serde_json::Value {
            doc["paths"][path]["get"]["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["name"] == name)
                .expect("the parameter is documented")["schema"]
                .clone()
        };
        let await_path = "/api/v1/inboxes/{inbox}/await";
        let assert_path = "/api/v1/inboxes/{inbox}/assert";

        let limit = schema(await_path, "limit");
        assert_eq!(limit["default"], DEFAULT_LIST_LIMIT, "{limit}");
        assert_eq!(limit["minimum"], 0, "{limit}");
        assert_eq!(limit["maximum"], MAX_PAGE_SIZE, "{limit}");
        assert_eq!(checked_page_size("limit", 0), Ok(0), "0 is a legal page");
        let omitted: AwaitQuery = serde_urlencoded::from_str("").expect("an empty await query");
        assert_eq!(omitted.limit, None, "an omitted limit rides the default");

        let count = schema(await_path, "count");
        assert_eq!(count["default"], omitted.min_one_count(), "{count}");
        assert_eq!(count["minimum"], 0, "{count}");
        assert_eq!(count["maximum"], MAX_PAGE_SIZE, "{count}");
        let zero: AwaitQuery = serde_urlencoded::from_str("count=0").expect("count=0 parses");
        assert_eq!(zero.min_one_count(), 1, "count=0 waits for one");
        assert_eq!(
            checked_page_size("count", MAX_PAGE_SIZE + 1),
            Err(format!("count must be at most {MAX_PAGE_SIZE}"))
        );

        let inert = schema(assert_path, "limit");
        assert_eq!(inert.get("default"), None, "no page default: {inert}");
        assert_eq!(inert.get("maximum"), None, "no page cap: {inert}");
        assert_eq!(inert["minimum"], 0, "the parser's unsigned domain: {inert}");
        for accepted in ["limit=0".to_string(), format!("limit={}", usize::MAX)] {
            serde_urlencoded::from_str::<AssertQuery>(&accepted).expect("an inert limit parses");
        }
        serde_urlencoded::from_str::<AssertQuery>("limit=-1").expect_err("a negative limit");

        let exact = schema(assert_path, "count");
        let omitted: AssertQuery = serde_urlencoded::from_str("").expect("an empty assert query");
        assert_eq!(exact["default"], omitted.exact_count(), "{exact}");
        assert_eq!(exact["minimum"], 0, "{exact}");
        assert_eq!(
            exact.get("maximum"),
            None,
            "assert's count is exempt: {exact}"
        );
        let zero: AssertQuery = serde_urlencoded::from_str("count=0").expect("count=0 parses");
        assert_eq!(zero.exact_count(), 0, "0 is the negative assertion");
    }

    /// The count and view routes publish exactly what their parsers take
    /// (decision 0048 §4, issue #134): count the four-name filter, the three
    /// view routes nothing beyond their path. Each documented count key
    /// parses; `limit` and `offset` — the list keys count used to swallow —
    /// refuse, and so does any key on a view.
    #[test]
    fn the_served_openapi_publishes_the_count_and_view_parameters() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("static/openapi.json")).expect("openapi.json");
        let query_names = |path: &str| -> Vec<String> {
            let mut v: Vec<String> = doc["paths"][path]["get"]["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["in"] == "query")
                .map(|p| p["name"].as_str().unwrap().to_string())
                .collect();
            v.sort();
            v
        };
        assert_eq!(
            query_names("/api/v1/inboxes/{inbox}/count"),
            ["from", "since_ms", "subject", "to"]
        );
        for view in [
            "/api/v1/inboxes/{inbox}/threads",
            "/api/v1/inboxes/{inbox}/threads/{key}",
            "/api/v1/inboxes/{inbox}/feed",
        ] {
            assert!(query_names(view).is_empty(), "{view} takes no query");
        }
        let q: CountQuery = serde_urlencoded::from_str("to=a&from=b&subject=c&since_ms=-1")
            .expect("the count parser accepts every documented parameter");
        assert_eq!(
            (q.to.as_deref(), q.from.as_deref(), q.subject.as_deref()),
            (Some("a"), Some("b"), Some("c"))
        );
        assert_eq!(q.since_ms, Some(-1));
        for key in ["limit", "offset"] {
            let err = serde_urlencoded::from_str::<CountQuery>(&format!("{key}=1"))
                .expect_err("a page key refuses on count");
            assert!(err.to_string().contains(&format!("`{key}`")), "{err}");
        }
        serde_urlencoded::from_str::<ViewQuery>("").expect("an empty query is a view query");
        let err = serde_urlencoded::from_str::<ViewQuery>("to=a").expect_err("a view key refuses");
        assert!(err.to_string().contains("`to`"), "{err}");
    }

    /// The served body schemas agree with the parsers (decision 0048 §§2–3):
    /// the seed and webhook request objects are closed, their documented
    /// keys are exactly what the parsers take and their optional strings
    /// are non-null on input, while the webhook response schema keeps the
    /// echo's nullable fields.
    #[test]
    fn the_served_openapi_publishes_the_closed_bodies() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("static/openapi.json")).expect("openapi.json");
        let schemas = &doc["components"]["schemas"];
        let keys = |schema: &serde_json::Value| -> Vec<String> {
            let mut v: Vec<String> = schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            v.sort();
            v
        };
        let seed = &schemas["SeedRequest"];
        assert_eq!(seed["additionalProperties"], false);
        assert_eq!(keys(seed), ["from", "html", "subject", "text", "to"]);
        let request = &schemas["WebhookTargetRequest"];
        assert_eq!(request["additionalProperties"], false);
        assert_eq!(request["required"], serde_json::json!(["url"]));
        assert_eq!(keys(request), ["ca_pem", "inbox", "secret", "url"]);
        let response = &schemas["WebhookTarget"];
        assert_eq!(keys(response), keys(request));
        for field in ["inbox", "secret", "ca_pem"] {
            assert_eq!(request["properties"][field]["type"], "string", "{field}");
            assert_eq!(
                response["properties"][field]["type"],
                serde_json::json!(["string", "null"]),
                "{field}"
            );
        }
        let put = &doc["paths"]["/api/v1/webhooks"]["put"];
        assert_eq!(
            put["requestBody"]["content"]["application/json"]["schema"]["items"]["$ref"],
            "#/components/schemas/WebhookTargetRequest"
        );

        // Every documented key parses; one more key refuses.
        let full =
            r#"{"to": "a@x.io", "from": "b@x.io", "subject": "s", "text": "t", "html": "h"}"#;
        serde_json::from_str::<SeedRequest>(full).expect("every documented seed key parses");
        let err = serde_json::from_str::<SeedRequest>(r#"{"to": "a@x.io", "cc": "c@x.io"}"#)
            .expect_err("an undocumented seed key refuses");
        assert!(err.to_string().contains("unknown field `cc`"), "{err}");
        let full = r#"[{"url": "http://h/", "inbox": "i", "secret": "s", "ca_pem": "p"}]"#;
        serde_json::from_str::<WebhookTargets>(full).expect("every documented target key parses");
    }

    /// A refused feed query subscribes nothing (decision 0048 §4, issue
    /// #134): the query parse sits between the inbox admission and the
    /// feed's construction, so a `?bogus=` feed answers 400 with the
    /// store's watcher table untouched — and the positive control, a
    /// query-less feed on the same state, does subscribe at construction
    /// (0002's subscribe-before-scan), so the observation can fail.
    #[tokio::test]
    async fn a_refused_feed_query_never_subscribes() {
        let (_stop, rx) = tokio::sync::watch::channel(false);
        let state = gauge_state(rx, false, false);
        let refused = inbox_feed(
            State(state.clone()),
            Path("feedq".into()),
            axum::extract::RawQuery(Some("bogus=1".into())),
        )
        .await;
        let ApiError(status, message) = refused.map(drop).expect_err("a feed query key refuses");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            message.starts_with("invalid feed query: unknown field `bogus`"),
            "{message}"
        );
        assert_eq!(
            state.store.watcher_count(),
            0,
            "the refusal subscribed nothing"
        );

        let accepted = inbox_feed(
            State(state.clone()),
            Path("feedq".into()),
            axum::extract::RawQuery(None),
        )
        .await;
        assert!(accepted.is_ok(), "a query-less feed is served");
        assert_eq!(
            state.store.watcher_count(),
            1,
            "the served feed subscribed at construction"
        );
        drop(accepted);
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
        let rendered = registry_rows(&AWAITS, "swarmail_awaits_active", "gauge", 1);
        assert!(
            rendered.contains("swarmail_awaits_active{inbox=\"cov-inner\"} 1"),
            "the label survives an inner release: {rendered}"
        );
        drop(outer);
        assert_eq!(awaits_active(), 0, "the last release empties the gauge");
        let rendered = registry_rows(&AWAITS, "swarmail_awaits_active", "gauge", 0);
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
