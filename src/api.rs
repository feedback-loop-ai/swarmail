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

fn bad_request(msg: impl Into<String>) -> ApiError {
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

pub async fn serve(
    listener: TcpListener,
    state: AppState,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await
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

#[derive(Debug, Deserialize)]
pub struct AwaitQuery {
    #[serde(flatten)]
    pub common: ListQuery,
    /// Number of matching emails to wait for (default 1).
    pub count: Option<usize>,
    /// Give up after this long; default 5000ms.
    pub timeout_ms: Option<u64>,
}

impl AwaitQuery {
    fn filter(&self) -> Filter {
        self.common.filter()
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
        let ms = self.timeout_ms.unwrap_or(5000);
        if ms > MAX_AWAIT_TIMEOUT_MS {
            // Refused before any subscribe: an over-cap wait never registers
            // a watcher (decision 0018 — bounds are refusals, never clamps).
            return Err(bad_request(format!(
                "timeout_ms must be at most {MAX_AWAIT_TIMEOUT_MS}"
            )));
        }
        Ok(Duration::from_millis(ms))
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
    pub emails: Vec<Email>,
}

// ---------- handlers ----------

async fn healthz(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "uptime_seconds": state.started.elapsed().as_secs(),
    }))
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
    ];
    rows.iter()
        .map(|(kind, name, value)| format!("# TYPE {name} {kind}\n{name} {value}\n"))
        .collect()
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
    let all = state.store.list(&inbox, &q.filter());
    let total = all.len();
    let offset = q.offset.unwrap_or(0);
    let emails: Vec<Arc<Email>> = all
        .into_iter()
        .skip(offset)
        .take(q.limit.unwrap_or(100))
        .collect();
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
    let timeout = q.timeout()?;
    let filter = q.filter();
    // The wait races the stop watch (decision 0018): a client parked on a
    // long timeout cannot hold `stop()` for its remainder — the moment the
    // server begins stopping, the wait answers 503 and lets the shutdown
    // proceed. `shutdown_signal` subscribes (clones) the receiver before
    // `wait_for`'s first scan; nothing is checked before subscribing.
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
            let emails = matches.iter().map(|e| (**e).clone()).collect();
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
) -> ApiResult<Json<Email>> {
    state
        .store
        .get(&id)
        .map(|e| Json((*e).clone()))
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
) -> Json<ChaosConfig> {
    state.chaos.set(config);
    Json(state.chaos.current())
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
/// naming the field and stores nothing.
async fn seed_inbox(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    Json(seed): Json<SeedRequest>,
) -> ApiResult<Json<Email>> {
    let email = smtp::build_seed_email(
        &inbox,
        &seed.to,
        seed.from.as_deref(),
        seed.subject.as_deref(),
        seed.text.as_deref(),
        seed.html.as_deref(),
    )
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
        Ok(stored) => Ok(Json((*stored).clone())),
        Err(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage commit failed".into(),
        )),
    }
}

// ---------- webhooks ----------

async fn get_webhooks(State(state): State<AppState>) -> Json<Vec<WebhookTarget>> {
    Json(state.webhooks.current())
}

async fn set_webhooks(
    State(state): State<AppState>,
    Json(targets): Json<Vec<WebhookTarget>>,
) -> Json<Vec<WebhookTarget>> {
    state.webhooks.set(targets);
    Json(state.webhooks.current())
}

async fn clear_webhooks(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.webhooks.set(vec![]);
    Json(serde_json::json!({ "cleared": true }))
}

// ---------- MCP ----------

async fn mcp_endpoint(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Response {
    let ctx = McpContext {
        store: state.store.clone(),
        chaos: state.chaos.clone(),
        shutdown: state.shutdown.clone(),
    };
    match mcp::handle(&ctx, &req).await {
        Some(response) => Json(response).into_response(),
        // Notifications get no reply.
        None => StatusCode::ACCEPTED.into_response(),
    }
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
}
