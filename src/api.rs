//! HTTP API: REST for tests, MCP endpoint, OpenAPI, health/metrics.

use crate::chaos::{Chaos, ChaosConfig};
use crate::feed;
use crate::mcp::{self, McpContext};
use crate::model::Email;
use crate::smtp;
use crate::store::{Filter, Store, WaitOutcome};
use crate::threads;
use crate::ui;
use crate::webhook::{WebhookTarget, Webhooks};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
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
        .route("/api/v1/messages", delete(clear_all))
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

    fn count(&self) -> usize {
        self.count.unwrap_or(1).max(1)
    }

    fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.unwrap_or(5000))
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
    pub emails: Vec<Email>,
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
    let emails: Vec<Email> = all
        .into_iter()
        .skip(offset)
        .take(q.limit.unwrap_or(100))
        .map(|e| (*e).clone())
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
    let want = q.count();
    let total = state.store.count(&inbox, &Filter::default());
    match state
        .store
        .wait_for(&inbox, &q.filter(), want, q.timeout())
        .await
    {
        WaitOutcome::Found(matches) => {
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
        WaitOutcome::Timeout { matched } => Ok((
            StatusCode::REQUEST_TIMEOUT,
            Json(AwaitResponse {
                matched,
                total,
                emails: vec![],
            }),
        )
            .into_response()),
    }
}

async fn assert_messages(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    raw: axum::extract::RawQuery,
) -> ApiResult<Response> {
    let q: AwaitQuery = parse_query(raw.0.as_deref().unwrap_or(""), "assert")?;
    let want = q.count();
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
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "removed": state.store.clear(&inbox) }))
}

async fn clear_all(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "removed": state.store.clear_all() }))
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
    if state.store.delete(&id) {
        Ok(Json(serde_json::json!({ "deleted": true })))
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "message not found".into()))
    }
}

async fn get_raw(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<String> {
    state
        .store
        .get(&id)
        .map(|e| String::from_utf8_lossy(&e.raw).to_string())
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
    Sse::new(feed::feed_stream(state.store.clone(), inbox)).keep_alive(KeepAlive::default())
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

/// Inject a synthetic email without SMTP — full pipeline (parse, extract) runs.
async fn seed_inbox(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
    Json(seed): Json<SeedRequest>,
) -> Json<Email> {
    let from = seed.from.unwrap_or_else(|| "fixture@swarmail.dev".into());
    let subject = seed.subject.unwrap_or_else(|| "(no subject)".into());
    let mut raw = format!("From: {from}\r\nTo: {}\r\nSubject: {subject}\r\n", seed.to);
    if let Some(html) = &seed.html {
        raw.push_str(&format!(
            "MIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n{html}\r\n"
        ));
    } else {
        raw.push_str(&format!("\r\n{}\r\n", seed.text.unwrap_or_default()));
    }
    let email = smtp::build_email(raw.as_bytes(), &inbox, std::slice::from_ref(&seed.to), from);
    let stored = state.store.insert(email);
    Json((*stored).clone())
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
