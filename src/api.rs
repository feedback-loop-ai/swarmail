//! HTTP API: REST for tests, plus health/metrics. (MCP, UI, OpenAPI land in P2/P3.)

use crate::chaos::{Chaos, ChaosConfig};
use crate::model::Email;
use crate::store::{Filter, Store};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub chaos: Arc<Chaos>,
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

pub async fn serve(listener: TcpListener, state: AppState) -> std::io::Result<()> {
    axum::serve(listener, router(state)).await
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
    pub to: Option<String>,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub since_ms: Option<i64>,
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
    let t = &state.store.totals;
    let inboxes = state.store.inboxes();
    let stored: usize = inboxes.iter().map(|(_, c)| c).sum();
    let mut out = String::new();
    out.push_str(&format!(
        "# TYPE swarmail_emails_inserted_total counter\nswarmail_emails_inserted_total {}\n",
        t.emails_inserted.load(std::sync::atomic::Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "# TYPE swarmail_emails_dropped_total counter\nswarmail_emails_dropped_total {}\n",
        t.emails_dropped.load(std::sync::atomic::Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "# TYPE swarmail_emails_stored gauge\nswarmail_emails_stored {stored}\n"
    ));
    out.push_str(&format!(
        "# TYPE swarmail_inboxes gauge\nswarmail_inboxes {}\n",
        inboxes.len()
    ));
    out
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
    let filter = q.filter();
    let deadline = Instant::now() + q.timeout();

    // Subscribe first so a mail arriving during the check cannot be missed.
    let mut rx = state.store.subscribe(&inbox);
    loop {
        let matches = state.store.list(&inbox, &filter);
        if matches.len() >= want {
            let total = state.store.count(&inbox, &Filter::default());
            let emails: Vec<Email> = matches
                .into_iter()
                .take(want)
                .map(|e| (*e).clone())
                .collect();
            return Ok((
                StatusCode::OK,
                Json(AwaitResponse {
                    matched: want,
                    total,
                    emails,
                }),
            )
                .into_response());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let total = state.store.count(&inbox, &Filter::default());
            return Ok((
                StatusCode::REQUEST_TIMEOUT,
                Json(AwaitResponse {
                    matched: matches.len(),
                    total,
                    emails: vec![],
                }),
            )
                .into_response());
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(email)) => {
                if filter.matches(&email) {
                    // Re-check the store: a burst may have delivered several at once.
                    continue;
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                // Slow watcher; the store remains the source of truth.
                continue;
            }
            Ok(Err(_)) | Err(_) => {
                let matches = state.store.list(&inbox, &filter);
                if matches.len() >= want {
                    let total = state.store.count(&inbox, &Filter::default());
                    let emails: Vec<Email> = matches
                        .into_iter()
                        .take(want)
                        .map(|e| (*e).clone())
                        .collect();
                    return Ok((
                        StatusCode::OK,
                        Json(AwaitResponse {
                            matched: want,
                            total,
                            emails,
                        }),
                    )
                        .into_response());
                }
                if Instant::now() >= deadline {
                    let total = state.store.count(&inbox, &Filter::default());
                    return Ok((
                        StatusCode::REQUEST_TIMEOUT,
                        Json(AwaitResponse {
                            matched: matches.len(),
                            total,
                            emails: vec![],
                        }),
                    )
                        .into_response());
                }
            }
        }
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
