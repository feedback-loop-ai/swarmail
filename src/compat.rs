//! MailHog / Mailpit HTTP-API compat shims.
//!
//! The well-known read endpoints of the two mail-sink tools, answered from
//! the SAME store — swarmail's inboxes stay the only mailbox model. A
//! request that does not name an inbox reads the whole store as one global
//! mailbox (that is what both tools expose); `?inbox=<name>` scopes a list,
//! a search or `latest` to a single swarmail inbox, which is the natural way
//! a per-test harness points the tool at its own mail.
//!
//! Shapes follow the upstream Go structs byte for byte where clients depend
//! on them, including quirks: Mailpit lowercases `mail.Address` JSON keys
//! (`{"name":..,"address":..}`) and drops an empty name, while MailHog uses
//! `Path{Relays,Mailbox,Domain,Params}` and a `MIME` field that is `null`
//! for non-MIME mail. Header maps are synthesized from the fields extract
//! extracted at ingest (decision 0005) — no query-time MIME re-parsing.
//!
//! Page work is bounded (decision 0044 §§5–6, issue #96): every list and
//! search build — scan, filter, page, render, serialize — runs on the
//! blocking pool through `Store::offload` and hands the async worker
//! finished bytes; a Mailpit snippet reads a bounded prefix of its body; a
//! MailHog page serializes borrowed views into one buffer capped at
//! [`MAX_COMPAT_PAGE_BYTES`].
//!
//! Deliberate omissions live in README.md (`Compat shims`); the headline
//! ones: no attachments, no read/unread state, no send-API, and no
//! `/api/v1/messages/{id}/mime/part/...`.

use crate::api::{ApiError, AppState, ViewQuery, bad_request, parse_query};
use crate::model::{Email, EmailAddress};
use crate::store::{Filter, Store};
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, RawQuery, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get};
use mail_parser::decoders::html::html_to_text;
use regex::Regex;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value, json};
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/messages/{id}/plain", get(message_plain_plural))
        .route("/api/v1/messages/{id}/download", get(message_download))
        .route("/api/v1/delete-all", delete(delete_all))
        .route("/api/v1/message/latest", get(message_latest))
        .route("/api/v1/message/latest/raw", get(message_latest_raw))
        .route(
            "/api/v1/message/latest/headers",
            get(message_latest_headers),
        )
        .route("/api/v1/message/{id}", get(message_mailpit))
        .route("/api/v1/message/{id}/plain", get(message_plain))
        .route("/api/v1/message/{id}/raw", get(message_raw))
        .route("/api/v1/message/{id}/headers", get(message_headers))
        .route("/api/v1/search", get(search_mailpit))
        .route("/api/v2/messages", get(list_messages_hog))
        .route("/api/v2/search", get(search_hog))
}

// ---------- MailHog / Mailpit request params ----------

/// Pagination shared by the Mailpit/MailHog list & search shapes, plus the
/// swarmail-only `inbox` scope.
#[derive(Debug, Default, Deserialize)]
pub struct ListParams {
    pub limit: Option<usize>,
    pub start: Option<usize>,
    pub inbox: Option<String>,
}

/// A search: `query` is required, `kind` picks the field (MailHog v2's
/// parameter style — upstream Mailpit instead parses `to:`-style prefixes
/// out of the query text, which this shim refuses by name).
#[derive(Debug, Default, Deserialize)]
pub struct SearchParams {
    /// Invocation-local observation at the actual user-needle compiler.
    #[cfg(test)]
    #[serde(skip)]
    pub(crate) compile_probe: Option<Arc<page_work_tests::CompileProbe>>,
    pub kind: Option<String>,
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub start: Option<usize>,
    pub inbox: Option<String>,
}

/// The `latest` routes' whole query shape (decision 0044 §4): the scope and
/// the refused `query`, nothing else.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LatestParams {
    inbox: Option<String>,
    query: Option<String>,
}

/// Mailpit's delete body: `{"ids": [...]}` — a non-empty list of message
/// ids is the only selector `DELETE /api/v1/messages` accepts (decision
/// 0011); anything else is a `400`. The alias also accepts the documented
/// `IDs` spelling (issue #55): upstream Mailpit's Go decoder matches field
/// names case-insensitively, so Mailpit's own clients send either case.
/// The rule — a non-empty selector — is untouched; only the spelling.
#[derive(Debug, Default, Deserialize)]
pub struct IdsBody {
    #[serde(default, alias = "IDs")]
    pub ids: Vec<String>,
}

// ---------- staging ----------

/// The shared page cap through this plane's `ApiError` envelope (decision
/// 0030 §3): the sentence stays the shared one, the status stays `400`
/// JSON on all four pages — including MailHog's search, whose *kind/query*
/// postures stay bare 400s.
fn page_cap(field: &str, n: usize) -> Result<usize, ApiError> {
    crate::api::checked_page_size(field, n).map_err(bad_request)
}

/// This dialect's own default page: upstream MailHog's `getStartLimit`
/// default (and Mailpit's) — the shared 100 is the native planes', and
/// the two dialects must not drift into each other.
const DEFAULT_PAGE_LIMIT: usize = 50;

/// The page limit, with `0` meaning "unset" (issue #55): upstream answers
/// the default page for `limit=0`, never an empty one — an empty page
/// makes a client's "assert nothing arrived" pass vacuously. The 0030 §3
/// cap still refuses anything above it; a malformed value is README-listed
/// (axum's own rejection shape).
fn page_limit(requested: Option<usize>) -> Result<usize, ApiError> {
    match requested {
        Some(0) | None => page_cap("limit", DEFAULT_PAGE_LIMIT),
        Some(n) => page_cap("limit", n),
    }
}

/// A supplied `inbox` selector through the shared admission rule (decision
/// 0044 §3): absence is the whole store, a blank name refuses — it never
/// silently widens to the global mailbox.
fn scope_inbox(inbox: Option<String>) -> Result<Option<String>, ApiError> {
    match inbox {
        Some(name) => {
            crate::model::validate_inbox(&name).map_err(bad_request)?;
            Ok(Some(name))
        }
        None => Ok(None),
    }
}

/// The field name a whitespace token opens with, if it opens with field
/// syntax (decision 0044 §7): a leading ASCII identifier
/// `[A-Za-z][A-Za-z0-9_-]*` followed by `:` — except a colon followed by
/// `//`, so a URL stays a literal. The spelling is the token's own.
fn field_prefix(token: &str) -> Option<&str> {
    let bytes = token.as_bytes();
    if !bytes.first()?.is_ascii_alphabetic() {
        return None;
    }
    let end = bytes
        .iter()
        .position(|b| !(b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-'))?;
    let rest = &token[end..];
    (rest.starts_with(':') && !rest.starts_with("://")).then_some(&token[..end])
}

/// The field a token names under decision 0061 §1 (operator, 2026-10-26):
/// remove every double quote, then at most one leading parenthesis and
/// one exclusion marker. This copy is only for refusal recognition;
/// accepted queries execute with their original quotes and markers.
fn wrapped_field_prefix(token: &str) -> Option<String> {
    let token = token.replace('"', "");
    let token = token.strip_prefix('(').unwrap_or(&token);
    let token = token.strip_prefix(['-', '!']).unwrap_or(token);
    field_prefix(token).map(str::to_owned)
}

/// Field syntax in a generic containing search refuses by name (decision
/// 0044 §7, issue #99): every whitespace token of the trimmed query is
/// examined, so ` to:x`, `from:alice` and `plan subject:x` refuse alike,
/// and an unknown `foo:` cannot answer a vacuous zero either — nor can a
/// wrapped `-to:x`, `!from:x`, `"to:x"` or `(to:x)` (decision 0049 §6).
/// There is no query grammar; explicit `kind=to|from|subject` values stay
/// literal.
fn refuse_field_syntax(params: &SearchParams) -> Result<(), ApiError> {
    if params.kind.as_deref().unwrap_or("containing") != "containing" {
        return Ok(());
    }
    let query = params.query.as_deref().unwrap_or("").trim();
    match query.split_whitespace().find_map(wrapped_field_prefix) {
        Some(field) => Err(bad_request(format!(
            "unsupported search prefix \"{field}:\" — Mailpit's prefix grammar is not \
             implemented; use ?kind=to|from|subject"
        ))),
        None => Ok(()),
    }
}

/// What a staged search matches.
enum Matcher {
    /// `kind=to|subject`: the native filter's own predicate — `to` is the
    /// To/Cc headers and the envelope recipients (issue #55).
    Filter(Filter),
    /// `kind=from`: the compat sender, [`compat_sender`] — the envelope
    /// when known (decision 0044 §8). The needle is lowercased once.
    Sender(String),
    /// `containing`: the escaped case-insensitive needle (0030 §4).
    Containing(Regex),
}

impl Matcher {
    fn matches(&self, email: &Email) -> bool {
        match self {
            Self::Filter(filter) => filter.matches(email),
            Self::Sender(needle) => compat_sender(email)
                .unwrap_or_default()
                .to_lowercase()
                .contains(needle.as_str()),
            Self::Containing(re) => contains(email, re),
        }
    }
}

/// Validate a search before any scan (decision 0030 §3's staging): the
/// query must be nonblank, the kind known, and a containing needle must
/// compile — a needle the engine refuses is the client's own input, so a
/// 400, not an unwind.
fn stage_search(params: &SearchParams) -> Result<Matcher, ApiError> {
    let query = params.query.as_deref().map(str::trim).unwrap_or("");
    if query.is_empty() {
        return Err(bad_request("Error: no search query"));
    }
    match params.kind.as_deref().unwrap_or("containing") {
        "to" => Ok(Matcher::Filter(Filter {
            to: Some(query.to_string()),
            ..Default::default()
        })),
        "from" => Ok(Matcher::Sender(query.to_lowercase())),
        "subject" => Ok(Matcher::Filter(Filter {
            subject: Some(query.to_string()),
            ..Default::default()
        })),
        // The needle keeps its call-site lowering — per request, once
        // (decision 0030 §4).
        "containing" => {
            #[cfg(test)]
            if let Some(probe) = &params.compile_probe {
                probe.enter();
            }
            search_needle(&query.to_lowercase())
                .map(Matcher::Containing)
                .ok_or(bad_request("Error: the search needle is too complex"))
        }
        other => Err(bad_request(format!("unknown search kind: {other}"))),
    }
}

// ---------- page work ----------

/// Run one compat page build — scan, filter, page, render and serialize —
/// on the blocking pool and hand back what it built (decision 0044 §5,
/// issue #96): no map guard crosses the await, no render or serialization
/// occupies an async worker, and no mutation lock is taken. Panics re-raise
/// in the caller (`Store::offload`'s symmetry).
async fn page_work<T, F>(store: Arc<Store>, build: F) -> T
where
    T: Send + 'static,
    F: FnOnce(&Store) -> T + Send + 'static,
{
    Store::offload(move || {
        #[cfg(test)]
        store.run_compat_gate();
        build(store.as_ref())
    })
    .await
}

/// Find one mail and build its whole response on the blocking pool
/// (decision 0049 §7, issue #139): the projection, the HTML-to-text render,
/// the body copies and the serialization all finish inside the closure, so
/// the async worker receives a finished `Response` — a `Json` returned for
/// Axum to serialize later would serialize a 30 MiB body on the worker.
/// The map guard is released by `find` before the build; no mutation lock
/// is taken. An absent mail is the read shapes' one 404. The test gate
/// holds between lookup and build and observes both invocation sites and
/// completed builds, separately from serialization. Panics re-raise in the caller.
pub(crate) async fn render_one<Find, Build>(
    store: Arc<Store>,
    find: Find,
    build: Build,
) -> Result<Response, ApiError>
where
    Find: FnOnce(&Store) -> Option<Arc<Email>> + Send + 'static,
    Build: FnOnce(&Email) -> Response + Send + 'static,
{
    #[cfg(test)]
    let caller = std::thread::current().id();
    Store::offload(move || {
        let email = {
            #[cfg(test)]
            page_work_tests::observe_work_thread("lookup", caller);
            find(&store)?
        };
        #[cfg(test)]
        page_work_tests::render_site(&store, "lookup", caller);
        let response = {
            #[cfg(test)]
            page_work_tests::observe_work_thread("build", caller);
            build(&email)
        };
        #[cfg(test)]
        page_work_tests::render_site(&store, "built", caller);
        Some(response)
    })
    .await
    .ok_or_else(not_found)
}

/// [`render_one`] for a mail named by id.
pub(crate) async fn render_by_id<Build>(
    state: &AppState,
    id: String,
    build: Build,
) -> Result<Response, ApiError>
where
    Build: FnOnce(&Email) -> Response + Send + 'static,
{
    render_one(state.store.clone(), move |store| store.get(&id), build).await
}

/// A JSON value serialized into its finished response — called inside
/// [`render_one`]'s closure, never on the async worker.
fn json_response(value: Value) -> Response {
    Json(value).into_response()
}

/// Finished JSON bytes as the response — serialized once, on the pool.
fn json_body(bytes: Vec<u8>) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], bytes).into_response()
}

/// The store as one mailbox (all inboxes), or one inbox when scoped,
/// newest first by `(received_ms, id)` — the order `latest` maximizes
/// (decision 0049 §5, issue #138). A scoped view sorts its inbox's
/// acceptance-ordered list into that order, so `messages?inbox=x&limit=1`
/// is `latest?inbox=x` at a stable store: the receive stamp is sampled at
/// parse start, and a large mail accepted after a small one can carry the
/// older stamp. Native listings, POP3 and pruning keep acceptance order.
fn scoped(store: &Store, inbox: Option<&str>) -> Vec<Arc<Email>> {
    match inbox {
        Some(inbox) => {
            let mut all = store.list(inbox, &Filter::default());
            all.sort_by(crate::store::newest_first);
            all
        }
        None => store.list_all(&Filter::default()),
    }
}

/// The scope's mail count, without listing it.
fn mailbox_size(store: &Store, inbox: Option<&str>) -> usize {
    match inbox {
        Some(inbox) => store.count(inbox, &Filter::default()),
        None => store.inboxes().iter().map(|(_, count)| count).sum(),
    }
}

/// The `start`/`limit` window of a newest-first list; empty past the end.
fn page_of(all: &[Arc<Email>], start: usize, limit: usize) -> &[Arc<Email>] {
    let rest = all.get(start..).unwrap_or_default();
    &rest[..limit.min(rest.len())]
}

/// A search's matches in scope, newest first, unpaginated (the caller
/// pages).
fn run_search(store: &Store, inbox: Option<&str>, matcher: &Matcher) -> Vec<Arc<Email>> {
    let mut all = scoped(store, inbox);
    all.retain(|e| matcher.matches(e));
    all
}

// ---------- Mailpit: summary envelope ----------

/// GET /api/v1/messages — Mailpit's paginated summary envelope.
pub async fn list_messages_mailpit(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Response, ApiError> {
    // The cap and the scope are staged before the scan (decision 0030 §3).
    let limit = page_limit(params.limit)?;
    let inbox = scope_inbox(params.inbox)?;
    let start = params.start.unwrap_or(0);
    let body = page_work(state.store.clone(), move |store| {
        let all = scoped(store, inbox.as_deref());
        mailpit_envelope(all.len(), all.len(), start, page_of(&all, start, limit))
            .to_string()
            .into_bytes()
    })
    .await;
    Ok(json_body(body))
}

/// GET /api/v1/search — Mailpit's search envelope; empty query is a 400.
async fn search_mailpit(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Response, ApiError> {
    // Everything is staged before the scan (decision 0030 §3): the cap, the
    // scope, Mailpit's prefix grammar (refused by name — a silently empty
    // result is the plausible-wrong answer, 0025's discipline) and the
    // search itself.
    let limit = page_limit(params.limit)?;
    let inbox = scope_inbox(params.inbox.clone())?;
    refuse_field_syntax(&params)?;
    let start = params.start.unwrap_or(0);
    let body = page_work(state.store.clone(), move |store| {
        let matcher = stage_search(&params)?;
        let all = run_search(store, inbox.as_deref(), &matcher);
        // Mailpit's search envelope reports `total`/`unread` for the whole
        // mailbox — the unfiltered scope (issue #55) — while the match
        // count rides `messages_count`.
        let mailbox = mailbox_size(store, inbox.as_deref());
        Ok::<_, ApiError>(
            mailpit_envelope(mailbox, all.len(), start, page_of(&all, start, limit))
                .to_string()
                .into_bytes(),
        )
    })
    .await?;
    Ok(json_body(body))
}

/// Mailpit's summary envelope. `mailbox` is the unfiltered scope's count —
/// upstream fills `total`/`unread` from `storage.StatsGet()`, which knows
/// the mailbox, not the page. `matches` is the row count this request
/// matched: the list endpoint reports it as the mailbox's own size, the
/// search endpoint as the query's hit count (`messages_count`).
fn mailpit_envelope(mailbox: usize, matches: usize, start: usize, page: &[Arc<Email>]) -> Value {
    json!({
        "total": mailbox,
        // swarmail keeps no read state, so every mail counts as unread —
        // the mailbox's own size here, the match count below (issue #99).
        "unread": mailbox,
        "count": page.len(),
        "messages_count": matches,
        "messages_unread": matches,
        "start": start,
        "tags": [],
        "messages": page.iter().map(|e| mailpit_summary(e)).collect::<Vec<_>>(),
    })
}

// ---------- Mailpit: latest ----------

/// Resolve `latest` (decision 0044 §4, issue #95): the newest mail by
/// `(received_ms, id)` in the requested scope — one inbox, or the whole
/// store when none is named — found in one pass on the blocking pool. A
/// nonblank `query` refuses by name: a filtered latest is a search and an
/// id fetch. An empty scope is a 404 even when another inbox has mail. Any
/// other key — Mailpit's `tz` included — is a named `400` (README lists
/// it). The selection and the whole render run in one blocking-pool hop
/// ([`render_one`]); the winner's `Arc` is held through the render, and
/// two calls are two snapshots.
async fn render_latest<Build>(
    state: &AppState,
    raw: RawQuery,
    build: Build,
) -> Result<Response, ApiError>
where
    Build: FnOnce(&Email) -> Response + Send + 'static,
{
    let params: LatestParams = crate::api::parse_query(raw.0.as_deref().unwrap_or(""), "latest")?;
    let inbox = scope_inbox(params.inbox)?;
    if params
        .query
        .as_deref()
        .is_some_and(|q| !q.trim().is_empty())
    {
        return Err(bad_request(
            "query is not supported on latest; search with ?query=, then fetch the message by id",
        ));
    }
    render_one(
        state.store.clone(),
        move |store| store.latest(inbox.as_deref()),
        build,
    )
    .await
}

/// The one not-found answer of the read shapes.
fn not_found() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "message not found".into())
}

/// GET /api/v1/message/latest — the newest mail in scope, in Mailpit's
/// message shape: the route Mailpit's UI lands on before an id is chosen.
async fn message_latest(
    State(state): State<AppState>,
    raw: RawQuery,
) -> Result<Response, ApiError> {
    render_latest(&state, raw, |e| json_response(mailpit_message(e))).await
}

/// GET /api/v1/message/latest/raw — the newest mail's exact source.
async fn message_latest_raw(
    State(state): State<AppState>,
    raw: RawQuery,
) -> Result<Response, ApiError> {
    render_latest(&state, raw, raw_response).await
}

/// GET /api/v1/message/latest/headers — the newest mail's header map.
async fn message_latest_headers(
    State(state): State<AppState>,
    raw: RawQuery,
) -> Result<Response, ApiError> {
    render_latest(&state, raw, |e| json_response(hog_headers(e))).await
}

// ---------- by id ----------

/// GET /api/v1/message/{id} — Mailpit's full message shape.
async fn message_mailpit(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    render_by_id(&state, id, |e| json_response(mailpit_message(e))).await
}

/// GET /api/v1/message/{id}/plain — the text body as text/plain.
async fn message_plain(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    render_by_id(&state, id, plain_response).await
}

/// GET /api/v1/message/{id}/raw — the full source, as MailHog serves it.
async fn message_raw(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    render_by_id(&state, id, raw_response).await
}

/// GET /api/v1/message/{id}/headers — just the synthesized header map.
async fn message_headers(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    render_by_id(&state, id, |e| json_response(hog_headers(e))).await
}

/// GET /api/v1/messages/{id}/plain — the plural path MailHog clients use.
async fn message_plain_plural(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    render_by_id(&state, id, plain_response).await
}

/// GET /api/v1/messages/{id}/download — MailHog's .eml attachment download.
async fn message_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    render_by_id(&state, id, raw_response).await
}

// ---------- MailHog v2 ----------

/// The largest MailHog v2 list/search response body (decision 0044 §6,
/// issue #96): 64 MiB of actual JSON, inclusive — escaping, the duplicated
/// MIME fields and the envelope all counted as written. A page that would
/// need one byte more refuses whole; nothing partial is ever sent. Not a
/// global memory bound, and not a substitute for the 250-item cap.
pub const MAX_COMPAT_PAGE_BYTES: usize = 67_108_864;

/// The budget refusal: the whole page, by name, with both ways out.
fn page_budget_refusal() -> ApiError {
    bad_request(format!(
        "compat page exceeds byte budget {MAX_COMPAT_PAGE_BYTES}; reduce limit or fetch raw by id"
    ))
}

/// GET /api/v2/messages — `{total, count, start, items: [Message...]}`.
async fn list_messages_hog(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Response, ApiError> {
    // The cap and the scope are staged before the scan (decision 0030 §3).
    let limit = page_limit(params.limit)?;
    let inbox = scope_inbox(params.inbox)?;
    let start = params.start.unwrap_or(0);
    let body = page_work(state.store.clone(), move |store| {
        let all = scoped(store, inbox.as_deref());
        hog_page_body(
            all.len(),
            start,
            page_of(&all, start, limit),
            MAX_COMPAT_PAGE_BYTES,
        )
    })
    .await;
    body.map(json_body)
        .map_err(|OverBudget| page_budget_refusal())
}

/// The two 400 shapes the Hog search answers: the dialect quirks (an
/// unknown kind, an empty query, an unrunnable search) stay **bare** —
/// upstream's byte-exact refusal, no body — while the swarmail refusals
/// (the shared page cap, a blank inbox, field syntax, the byte budget) ride
/// the JSON envelope like every other plane's (decisions 0030 §3, 0044). A
/// plain `Response` error leg would be an oversized `Err` — this is the
/// same wire at four bytes.
enum HogRejection {
    Bare(StatusCode),
    Json(ApiError),
}

impl IntoResponse for HogRejection {
    fn into_response(self) -> Response {
        match self {
            Self::Bare(code) => code.into_response(),
            Self::Json(err) => err.into_response(),
        }
    }
}

/// GET /api/v2/search — kind ∈ {from,to,containing}; a bare 400 otherwise,
/// exactly as upstream answers (no body, no message). The swarmail-owned
/// refusals ride the shared `400` JSON envelope (`HogRejection::Json`).
async fn search_hog(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Response, HogRejection> {
    // The KIND is the dialect's own domain, staged here and not widened by
    // the shared engine: upstream answers a bare 400 for anything but
    // from|to|containing — including an *absent* kind, whose `""` fails the
    // same whitelist (MailHog-Server `api/v2.go`, `search()`: `kind !=
    // "from" && kind != "to" && kind != "containing"` → 400) — and
    // `subject`, the swarmail engine's own kind, is not this dialect's
    // contract (0029's advertised-domain discipline).
    match params.kind.as_deref() {
        Some("from") | Some("to") | Some("containing") => {}
        _ => return Err(HogRejection::Bare(StatusCode::BAD_REQUEST)),
    }
    let limit = page_limit(params.limit).map_err(HogRejection::Json)?;
    let inbox = scope_inbox(params.inbox.clone()).map_err(HogRejection::Json)?;
    refuse_field_syntax(&params).map_err(HogRejection::Json)?;
    // The remaining quirks — an empty query, an unrunnable needle — are
    // upstream's bare `400` with no body.
    let start = params.start.unwrap_or(0);
    let body = page_work(state.store.clone(), move |store| {
        let matcher =
            stage_search(&params).map_err(|_| HogRejection::Bare(StatusCode::BAD_REQUEST))?;
        let all = run_search(store, inbox.as_deref(), &matcher);
        hog_page_body(
            all.len(),
            start,
            page_of(&all, start, limit),
            MAX_COMPAT_PAGE_BYTES,
        )
        .map_err(|OverBudget| HogRejection::Json(page_budget_refusal()))
    })
    .await;
    body.map(json_body)
}

/// The page would not fit its byte budget.
#[derive(Debug, PartialEq)]
struct OverBudget;

/// A buffer that refuses to hold more than `budget` bytes (decision 0044
/// §6): a write that would cross the budget is refused whole — no partial
/// chunk is appended — and the allocation grows by doubling but never past
/// the budget, so the logical bytes and the requested capacity both stay
/// within it (allocator overhead aside).
struct CappedBuffer {
    bytes: Vec<u8>,
    budget: usize,
}

impl std::io::Write for CappedBuffer {
    fn write(&mut self, chunk: &[u8]) -> std::io::Result<usize> {
        // `len <= budget` always holds, so the subtraction cannot wrap.
        if chunk.len() > self.budget - self.bytes.len() {
            return Err(std::io::Error::other("compat page byte budget exceeded"));
        }
        let needed = self.bytes.len() + chunk.len();
        if needed > self.bytes.capacity() {
            let target = needed
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.budget);
            self.bytes.reserve_exact(target - self.bytes.len());
        }
        self.bytes.extend_from_slice(chunk);
        Ok(chunk.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Serialize `value` into one capped buffer. The views serialized here are
/// strings, numbers and nulls — serde_json cannot fail on them — so the
/// writer's refusal is the only error this can meet, and it is the budget.
fn serialize_capped<T: Serialize>(value: &T, budget: usize) -> Result<Vec<u8>, OverBudget> {
    let mut buffer = CappedBuffer {
        bytes: Vec::new(),
        budget,
    };
    serde_json::to_writer(&mut buffer, value)
        .map(|()| buffer.bytes)
        .map_err(|_| OverBudget)
}

/// One MailHog v2 page body: `{count, items, start, total}`, the borrowed
/// items serialized straight into the capped buffer — no owned page tree,
/// no body clones.
fn hog_page_body(
    total: usize,
    start: usize,
    page: &[Arc<Email>],
    budget: usize,
) -> Result<Vec<u8>, OverBudget> {
    let view = HogPage {
        count: page.len(),
        items: page.iter().map(|e| HogItem(e)).collect(),
        start,
        total,
    };
    serialize_capped(&view, budget)
}

#[derive(Serialize)]
struct HogPage<'a> {
    count: usize,
    items: Vec<HogItem<'a>>,
    start: usize,
    total: usize,
}

/// MailHog's data.Message — the v2 list and search item — as a borrowed
/// view of one stored mail (decision 0044 §6). `To` and `Raw.To` hold the
/// *envelope* recipients (issue #55): upstream MailHog fills them from the
/// RCPT TO list, while the header rows stay in `Content.Headers`; `From`
/// and `Raw.From` hold the compat sender (decision 0044 §8). The lossy
/// `Raw.Data` string exists only while this one item serializes.
struct HogItem<'a>(&'a Email);

impl Serialize for HogItem<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let email = self.0;
        let sender = compat_sender(email).unwrap_or_default();
        let mime = hog_mime(email);
        HogMessage {
            content: HogContent {
                // The body as a client displays it: the text part, else
                // the HTML.
                body: email
                    .text
                    .as_deref()
                    .or(email.html.as_deref())
                    .unwrap_or_default(),
                headers: hog_headers(email),
                mime: &mime,
                size: email.size,
            },
            created: &email.received_at,
            from: hog_path(sender),
            id: &email.id,
            mime: &mime,
            raw: HogRaw {
                data: String::from_utf8_lossy(&email.raw),
                from: sender,
                // Swarmail does not record the EHLO name.
                helo: "",
                to: &email.recipients,
            },
            to: email.recipients.iter().map(|r| hog_path(r)).collect(),
        }
        .serialize(serializer)
    }
}

/// The item's fields, in the order the previous `Value` tree emitted them.
#[derive(Serialize)]
struct HogMessage<'a> {
    #[serde(rename = "Content")]
    content: HogContent<'a>,
    #[serde(rename = "Created")]
    created: &'a str,
    #[serde(rename = "From")]
    from: HogPath<'a>,
    #[serde(rename = "ID")]
    id: &'a str,
    #[serde(rename = "MIME")]
    mime: &'a Option<HogMime<'a>>,
    #[serde(rename = "Raw")]
    raw: HogRaw<'a>,
    #[serde(rename = "To")]
    to: Vec<HogPath<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct HogContent<'a> {
    body: &'a str,
    headers: Value,
    #[serde(rename = "MIME")]
    mime: &'a Option<HogMime<'a>>,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct HogRaw<'a> {
    data: Cow<'a, str>,
    from: &'a str,
    helo: &'static str,
    to: &'a [String],
}

/// MailHog's `Path`: an address split at its last `@`; `Relays` is null.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct HogPath<'a> {
    domain: &'a str,
    mailbox: &'a str,
    params: &'static str,
    relays: (),
}

/// The split for an envelope string — MailHog fills `To` from the RCPT TO
/// list (issue #55), which carries addresses without display names.
fn hog_path(address: &str) -> HogPath<'_> {
    let (mailbox, domain) = address.rsplit_once('@').unwrap_or((address, ""));
    HogPath {
        domain,
        mailbox,
        params: "",
        relays: (),
    }
}

/// MailHog's MIME structure for the extracted parts; serialized as both
/// `MIME` and `Content.MIME`, upstream's own duplication.
#[derive(Serialize)]
struct HogMime<'a> {
    #[serde(rename = "Content-Type")]
    content_type: &'static str,
    #[serde(rename = "Parts")]
    parts: Vec<HogPart<'a>>,
}

#[derive(Serialize)]
struct HogPart<'a> {
    #[serde(rename = "Body")]
    body: &'a str,
    #[serde(rename = "Headers")]
    headers: HogPartHeaders,
}

#[derive(Serialize)]
struct HogPartHeaders {
    #[serde(rename = "Content-Type")]
    content_type: [&'static str; 1],
}

/// The MIME view, or `None` — serialized `null` — when the message has
/// neither a text nor an HTML body.
fn hog_mime(email: &Email) -> Option<HogMime<'_>> {
    let parts: Vec<HogPart<'_>> = [
        (email.text.as_deref(), "text/plain; charset=UTF-8"),
        (email.html.as_deref(), "text/html; charset=UTF-8"),
    ]
    .into_iter()
    .filter_map(|(body, content_type)| {
        body.map(|body| HogPart {
            body,
            headers: HogPartHeaders {
                content_type: [content_type],
            },
        })
    })
    .collect();
    (!parts.is_empty()).then_some(HogMime {
        content_type: "",
        parts,
    })
}

// ---------- deletes ----------

/// DELETE /api/v1/delete-all — swarmail's own explicit wipe of every inbox
/// (issue #99): not an upstream route. MailHog's wipe — `DELETE
/// /api/v1/messages` with no body — is refused by decision 0011, so a
/// parallel session's typo can never become everyone's data loss.
///
/// A failed commit is the answer (decision 0014): `503` with the fixed
/// text, never a count for work that did not commit.
async fn delete_all(State(state): State<AppState>, raw: RawQuery) -> Result<Json<Value>, ApiError> {
    let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "DELETE /api/v1/delete-all")?;
    match state.store.clone().clear_all_async().await {
        Ok(removed) => Ok(Json(json!({ "removed": removed }))),
        Err(_) => Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage commit failed".into(),
        )),
    }
}

/// DELETE /api/v1/messages — Mailpit's `{"ids": [...]}` delete: the named
/// ids and nothing else (decision 0011). Every other body — absent, empty,
/// unparseable, wrong-typed, or an empty `ids` array — is a `400` that
/// deletes nothing: one client's typo must never be every parallel session's
/// data loss, and the cross-inbox wipe lives only at `DELETE
/// /api/v1/delete-all`. The deletion flows through the store's awaited
/// path, so the commit is known before the reply and removals still release
/// emptied inboxes for `await` watchers to rescan (decisions 0001/0002).
pub async fn delete_messages(
    State(state): State<AppState>,
    body: Result<Json<IdsBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let ids = match body {
        Ok(Json(ids)) if !ids.ids.is_empty() => ids.ids,
        _ => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "deletion requires a non-empty \"ids\" list".into(),
            ));
        }
    };
    // The deletion itself flows through the store's awaited path, so the
    // commit is known before the reply (decision 0023) and the counted
    // result is exactly the ids the store removed. A failed commit is the
    // answer (decision 0014): `503` — the ids already committed stay
    // committed, so a retry re-runs idempotently — never a count for work
    // that did not commit. Removals still release emptied inboxes, so
    // `await` watchers rescan (decisions 0001/0002).
    let mut removed = 0usize;
    for id in &ids {
        match state.store.clone().delete_async(id.clone()).await {
            Ok(true) => removed += 1,
            Ok(false) => {}
            Err(_) => {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "storage commit failed".into(),
                ));
            }
        }
    }
    Ok(Json(json!({ "removed": removed })))
}

// ---------- shared internals ----------

/// The case-insensitive literal fold for one search needle (decision 0030
/// §4): the needle escaped into a `(?i)` regex — one small allocation per
/// request replaces the per-mail lowercase and lossy-string copies the old
/// shape paid. Escaped literals are always syntactically valid, but the
/// engine still refuses a needle whose compiled automaton exceeds its size
/// budget — under `(?i)` every cased character folds to a class (`k` gives
/// `K/k/U+212A`), so a query near the URI ceiling can exhaust the budget —
/// and the refusal is the caller's: untrusted input meets the plane's own
/// refusal, never a panic. Public because the read-stall bench measures
/// this exact fold, not a copy of it.
pub fn search_needle(needle: &str) -> Option<Regex> {
    Regex::new(&format!("(?i){}", regex::escape(needle))).ok()
}

/// Every haystack a containing search examines, in order — text, html,
/// subject, the To/Cc addresses, the From address, then the raw source —
/// produced lazily: the raw source's lossy string is built only when the
/// search actually reaches it.
fn haystacks(email: &Email) -> impl Iterator<Item = Cow<'_, str>> {
    let fields = email
        .text
        .iter()
        .chain(&email.html)
        .chain(&email.subject)
        .map(|field| Cow::Borrowed(field.as_str()));
    let addresses = email
        .to
        .iter()
        .chain(&email.cc)
        .chain(&email.from)
        .map(|address| Cow::Borrowed(address.address.as_str()));
    fields
        .chain(addresses)
        .chain(std::iter::once_with(|| String::from_utf8_lossy(&email.raw)))
}

/// Substring hit across the source fields and raw source. The
/// search stops at its first hit (decision 0044 §7, issue #96): a later
/// field can only repeat a yes already given, so the full raw source — the
/// largest haystack — is reached only when every field missed. No
/// haystack is copied: the fields match in place, the raw source through
/// the lossy borrow `String::from_utf8_lossy` already is.
pub fn contains(email: &Email, re: &Regex) -> bool {
    haystacks(email).any(|haystack| re.is_match(&haystack))
}

/// The compat sender (decision 0044 §8, issue #99): the SMTP envelope
/// sender when it is known — the empty null reverse path of a bounce
/// included, never replaced by the header — and, only for a row stored
/// before the envelope was kept (`None`), the header From address. `None`
/// when neither exists. MailHog's From/Raw.From/kind=from, Mailpit's
/// ReturnPath and the synthesized Return-Path read this; the native `from`
/// filter stays header-based.
fn compat_sender(email: &Email) -> Option<&str> {
    match &email.envelope_from {
        Some(envelope) => Some(envelope.as_str()),
        None => email.from.as_ref().map(|a| a.address.as_str()),
    }
}

/// Mailpit's `mail.Address`: lowercase keys, empty name dropped.
fn mailpit_address(address: Option<&EmailAddress>) -> Value {
    match address {
        None => Value::Null,
        Some(a) => match &a.name {
            Some(name) if !name.is_empty() => json!({ "name": name, "address": a.address }),
            _ => json!({ "address": a.address }),
        },
    }
}

/// Mailpit's `Bcc`: the envelope-only recipients (issue #55) — RCPT TO
/// addresses that appear in neither the To nor the Cc header. Upstream
/// stores the envelope alongside the headers and fills `Bcc` from it; a
/// swarmail envelope recipient with no header row is exactly upstream's
/// Bcc. The comparison is case-insensitive on the address; the order is
/// the envelope's own; an RCPT TO carries no display name, so the entries
/// hold only the address.
fn bcc_only(email: &Email) -> Vec<Value> {
    let visible: HashSet<String> = email
        .to
        .iter()
        .chain(email.cc.iter())
        .map(|a| a.address.to_lowercase())
        .collect();
    email
        .recipients
        .iter()
        .filter(|r| !visible.contains(&r.to_lowercase()))
        .map(|r| {
            mailpit_address(Some(&EmailAddress {
                name: None,
                address: r.clone(),
            }))
        })
        .collect()
}

/// The `Created`/`Date`-style timestamp: swarmail stamps receive time.
fn date_of(email: &Email) -> String {
    email.received_at.clone()
}

/// The most source bytes one snippet inspects, for text and HTML alike
/// (decision 0044 §5, issue #96): a snippet costs a bounded prefix of its
/// body, not the body.
pub const SNIPPET_SOURCE_PREFIX_BYTES: usize = 8192;

/// The snippet's own length, in collapsed characters.
const SNIPPET_CHARS: usize = 200;

/// The longest prefix of `s` within `max` bytes that ends on a character
/// boundary.
fn bounded_prefix(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// The bounded text a snippet is built from, and whether the body was cut
/// to get it. The stored record is sender-honest (decision 0031 §1): when
/// the sender sent only an html part there is no `text`, so the snippet
/// renders a bounded prefix of that html at display time — a view for a
/// person reading the list (0031 §3, priced per item by 0044 §5), not a
/// body to store or extract from: no links or codes are derived here, and
/// nothing is written back.
fn snippet_source(email: &Email) -> (Cow<'_, str>, bool) {
    match (&email.text, &email.html) {
        (Some(text), _) => {
            let prefix = bounded_prefix(text, SNIPPET_SOURCE_PREFIX_BYTES);
            (Cow::Borrowed(prefix), prefix.len() < text.len())
        }
        (None, Some(html)) => {
            let prefix = bounded_prefix(html, SNIPPET_SOURCE_PREFIX_BYTES);
            (Cow::Owned(html_to_text(prefix)), prefix.len() < html.len())
        }
        (None, None) => (Cow::Borrowed(""), false),
    }
}

/// Up to 200 characters of `source` with every whitespace run collapsed
/// to one space, `...`-suffixed when either the output or the source was
/// cut. The collapse is lazy: it stops after the 201st collapsed character.
fn collapse(source: &str, source_cut: bool) -> String {
    let mut collapsed = source
        .split_whitespace()
        .flat_map(|word| std::iter::once(' ').chain(word.chars()))
        .skip(1);
    let out: String = collapsed.by_ref().take(SNIPPET_CHARS).collect();
    if source_cut || collapsed.next().is_some() {
        format!("{out}...")
    } else {
        out
    }
}

/// Mailpit's snippet, without the HTML stripping of a full render (swarmail
/// prefers the text part the sender actually sent). Large leading markup or
/// whitespace can leave it short; the stored bodies and the full plain and
/// raw views are untouched.
pub(crate) fn snippet(email: &Email) -> String {
    let (source, cut) = snippet_source(email);
    collapse(&source, cut)
}

/// Mailpit's MessageSummary — the list item shape.
fn mailpit_summary(email: &Email) -> Value {
    let mut m = Map::new();
    m.insert("ID".into(), json!(email.id));
    m.insert(
        "MessageID".into(),
        json!(email.message_id.clone().unwrap_or_else(|| email.id.clone())),
    );
    m.insert("Read".into(), json!(false));
    m.insert("From".into(), mailpit_address(email.from.as_ref()));
    m.insert(
        "To".into(),
        json!(
            email
                .to
                .iter()
                .map(|a| mailpit_address(Some(a)))
                .collect::<Vec<_>>()
        ),
    );
    m.insert(
        "Cc".into(),
        json!(
            email
                .cc
                .iter()
                .map(|a| mailpit_address(Some(a)))
                .collect::<Vec<_>>()
        ),
    );
    m.insert("Bcc".into(), json!(bcc_only(email)));
    m.insert("ReplyTo".into(), json!([]));
    m.insert(
        "Subject".into(),
        json!(email.subject.clone().unwrap_or_default()),
    );
    m.insert("Created".into(), json!(date_of(email)));
    // Mailpit records the authenticated SMTP user; swarmail routes by inbox.
    m.insert("Username".into(), json!(email.inbox));
    m.insert("Tags".into(), json!([]));
    m.insert("Size".into(), json!(email.size));
    m.insert("Attachments".into(), json!(0));
    m.insert("Snippet".into(), json!(snippet(email)));
    Value::Object(m)
}

/// Mailpit's full Message — the single-message shape.
fn mailpit_message(email: &Email) -> Value {
    let mut m = Map::new();
    m.insert("ID".into(), json!(email.id));
    m.insert(
        "MessageID".into(),
        json!(email.message_id.clone().unwrap_or_else(|| email.id.clone())),
    );
    m.insert("Read".into(), json!(false));
    m.insert("From".into(), mailpit_address(email.from.as_ref()));
    m.insert(
        "To".into(),
        json!(
            email
                .to
                .iter()
                .map(|a| mailpit_address(Some(a)))
                .collect::<Vec<_>>()
        ),
    );
    m.insert(
        "Cc".into(),
        json!(
            email
                .cc
                .iter()
                .map(|a| mailpit_address(Some(a)))
                .collect::<Vec<_>>()
        ),
    );
    m.insert("Bcc".into(), json!(bcc_only(email)));
    m.insert("ReplyTo".into(), json!([]));
    // Mailpit's Return-Path is the envelope sender: the compat sender
    // (decision 0044 §8), `""` for a bounce's null reverse path.
    m.insert(
        "ReturnPath".into(),
        json!(compat_sender(email).unwrap_or_default()),
    );
    m.insert(
        "Subject".into(),
        json!(email.subject.clone().unwrap_or_default()),
    );
    m.insert("Date".into(), json!(date_of(email)));
    m.insert("Tags".into(), json!([]));
    m.insert("Username".into(), json!(email.inbox));
    m.insert("Text".into(), json!(email.text.clone().unwrap_or_default()));
    m.insert("HTML".into(), json!(email.html.clone().unwrap_or_default()));
    m.insert("Size".into(), json!(email.size));
    m.insert("Inline".into(), json!([]));
    m.insert("Attachments".into(), json!([]));
    Value::Object(m)
}

/// MailHog's Content.Headers, synthesized from extracted fields (decision
/// 0005) plus the honest Received/Return-Path stamps MailHog also adds. The
/// Return-Path is the compat sender — `<>` for a known null reverse path —
/// and absent only when no sender is known at all.
fn hog_headers(email: &Email) -> Value {
    let mut m = Map::new();
    let mut push = |key: &str, value: Option<String>| {
        if let Some(value) = value {
            m.insert(key.to_string(), json!([value]));
        }
    };
    push("From", email.from.as_ref().map(format_address));
    push("To", (!email.to.is_empty()).then(|| joined(&email.to)));
    push("Cc", (!email.cc.is_empty()).then(|| joined(&email.cc)));
    push("Subject", email.subject.clone());
    push("Date", Some(date_of(email)));
    push("Message-ID", email.message_id.clone());
    push(
        "Return-Path",
        compat_sender(email).map(|sender| format!("<{sender}>")),
    );
    push(
        "Received",
        Some(format!(
            "from swarmail-smtp by Swarmail; {}",
            email.received_at
        )),
    );
    Value::Object(m)
}

/// `Name <addr>` or bare `addr` — the wire form both tools display.
fn format_address(a: &EmailAddress) -> String {
    match &a.name {
        Some(name) => format!("{name} <{}>", a.address),
        None => a.address.clone(),
    }
}

/// Comma-joined wire-form addresses.
fn joined(list: &[EmailAddress]) -> String {
    list.iter()
        .map(format_address)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The text body as text/plain, falling back to the HTML for html-only mail.
fn plain_response(email: &Email) -> Response {
    // The stored record is sender-honest (decision 0031 §1): the shim serves
    // the text part when the sender sent one, and for html-only mail renders
    // the html at display time — the second sanctioned rendering (decision
    // 0031 §3), derived at request time from the stored body (bounded by
    // 0022), never written back and never fed to extraction.
    let body = match &email.text {
        Some(text) => text.clone(),
        None => email
            .html
            .as_ref()
            .map(|h| html_to_text(h))
            .unwrap_or_default(),
    };
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response()
}

/// The full source as an .eml download — MailHog's download shape. The one
/// owner of that shape: the compat route and the native `/raw` route both
/// serve it, so the bytes stay exact everywhere.
pub(crate) fn raw_response(email: &Email) -> Response {
    let disposition = format!("attachment; filename=\"{}.eml\"", email.id);
    (
        [
            (header::CONTENT_TYPE, "message/rfc822".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        email.raw.clone(),
    )
        .into_response()
}

#[cfg(test)]
mod page_work_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal stored email; tests mutate the fields they exercise.
    fn mail() -> Email {
        Email {
            id: "id-1".into(),
            inbox: "inbox".into(),
            from: None,
            envelope_from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: None,
            received_at: "2026-02-03T04:05:06Z".into(),
            received_ms: 1_770_000_306_000,
            size: 42,
            text: None,
            html: None,
            link_metadata: None,
            attachments: None,
            links: vec![],
            codes: vec![],
            message_id: None,
            in_reply_to: None,
            references: vec![],
            raw: b"raw source".to_vec(),
        }
    }

    fn address(name: Option<&str>, addr: &str) -> EmailAddress {
        EmailAddress {
            name: name.map(Into::into),
            address: addr.into(),
        }
    }

    /// One MailHog item as the wire sees it.
    fn hog_item(email: &Email) -> Value {
        serde_json::to_value(HogItem(email)).unwrap()
    }

    #[test]
    fn mailpit_address_drops_empty_names() {
        assert_eq!(mailpit_address(None), Value::Null);
        assert_eq!(
            mailpit_address(Some(&address(None, "a@b.c"))),
            json!({"address": "a@b.c"})
        );
        assert_eq!(
            mailpit_address(Some(&address(Some(""), "a@b.c"))),
            json!({"address": "a@b.c"})
        );
        assert_eq!(
            mailpit_address(Some(&address(Some("Zoë"), "a@b.c"))),
            json!({"name": "Zoë", "address": "a@b.c"})
        );
    }

    #[test]
    fn hog_path_splits_and_copes_without_a_domain() {
        let path = |address: &str| serde_json::to_value(hog_path(address)).unwrap();
        assert_eq!(
            path(""),
            json!({"Relays": null, "Mailbox": "", "Domain": "", "Params": ""})
        );
        assert_eq!(
            path("mailbox@domain"),
            json!({"Relays": null, "Mailbox": "mailbox", "Domain": "domain", "Params": ""})
        );
        assert_eq!(
            path("just-a-mailbox"),
            json!({"Relays": null, "Mailbox": "just-a-mailbox", "Domain": "", "Params": ""})
        );
    }

    #[test]
    fn addresses_format_as_on_the_wire() {
        assert_eq!(
            format_address(&address(Some("Zoe"), "z@x.io")),
            "Zoe <z@x.io>"
        );
        assert_eq!(format_address(&address(None, "z@x.io")), "z@x.io");
        assert_eq!(joined(&[]), "");
        assert_eq!(
            joined(&[address(None, "a@x.io"), address(Some("Bee"), "b@x.io")]),
            "a@x.io, Bee <b@x.io>"
        );
    }

    #[test]
    fn snippet_collapses_truncates_and_falls_back() {
        // Over 200 characters: exactly 200 plus the suffix.
        let mut m = mail();
        m.text = Some("word ".repeat(60));
        let cut = snippet(&m);
        assert_eq!(cut.chars().count(), 203);
        assert!(cut.ends_with("..."));
        assert!(!cut.ends_with("  ..."));

        // Whitespace collapses to single spaces.
        let mut m = mail();
        m.text = Some("a\n b\t  c".into());
        assert_eq!(snippet(&m), "a b c");

        // Sender-honest storage (decision 0031 §1): an html-only mail has no
        // `text`, so the snippet renders it (decision 0031 §3) — and a mail
        // with neither body still has an empty snippet.
        let mut m = mail();
        m.html = Some("<p>the rendered body</p>".into());
        assert_eq!(snippet(&m), "the rendered body");
        assert_eq!(snippet(&mail()), "");
    }

    /// The snippet's work is bounded by its prefix (decision 0044 §5, issue
    /// #96): whatever the body's size, the text the collapse and the HTML
    /// render ever see is at most `SNIPPET_SOURCE_PREFIX_BYTES` of source —
    /// long text, long HTML and an all-whitespace body alike. Red before:
    /// the whole body was copied or rendered.
    #[test]
    fn the_snippet_inspects_a_bounded_prefix() {
        let huge = 4 * 1024 * 1024;
        let mut text = mail();
        text.text = Some("x".repeat(huge));
        let (source, cut) = snippet_source(&text);
        assert_eq!((source.len(), cut), (SNIPPET_SOURCE_PREFIX_BYTES, true));
        assert_eq!(snippet(&text), format!("{}...", "x".repeat(200)));

        let mut html = mail();
        html.html = Some(format!("<p>{}</p>", "y".repeat(huge)));
        let (source, cut) = snippet_source(&html);
        assert!(
            source.len() <= SNIPPET_SOURCE_PREFIX_BYTES,
            "{}",
            source.len()
        );
        assert!(cut);

        // All whitespace: nothing to show, but the cut is still said.
        let mut blank = mail();
        blank.text = Some(" \n\t".repeat(huge));
        let (source, cut) = snippet_source(&blank);
        assert_eq!((source.len(), cut), (SNIPPET_SOURCE_PREFIX_BYTES, true));
        assert_eq!(snippet(&blank), "...");
        // The stored body is untouched.
        assert_eq!(blank.text.as_ref().unwrap().len(), 3 * huge);
    }

    /// The truncation contract's edges: exactly 200 collapsed characters
    /// stay whole, 201 cut; a multibyte character straddling the byte bound
    /// is dropped whole, never split; entities and an unfinished tag render
    /// without panicking; a short body is never suffixed.
    #[test]
    fn the_snippet_edges_hold() {
        let mut m = mail();
        m.text = Some("a".repeat(200));
        assert_eq!(snippet(&m), "a".repeat(200));
        m.text = Some("a".repeat(201));
        assert_eq!(snippet(&m), format!("{}...", "a".repeat(200)));

        // 8191 ASCII bytes then a 2-byte `é`: the prefix ends before it.
        let straddle = format!("{}é tail", "b".repeat(SNIPPET_SOURCE_PREFIX_BYTES - 1));
        assert_eq!(
            bounded_prefix(&straddle, SNIPPET_SOURCE_PREFIX_BYTES).len(),
            SNIPPET_SOURCE_PREFIX_BYTES - 1
        );
        assert_eq!(
            bounded_prefix("short", SNIPPET_SOURCE_PREFIX_BYTES),
            "short"
        );
        assert_eq!(bounded_prefix("éé", 3), "é");

        let mut m = mail();
        m.html = Some("<p>fish &amp; chips</p>".into());
        assert_eq!(snippet(&m), "fish & chips");
        m.html = Some("<p>open <di".into());
        assert_eq!(snippet(&m), "open");
    }

    /// The search short-circuits (decision 0044 §7): an early-field hit
    /// pulls exactly one haystack, so the raw source's lossy string is
    /// never built; a raw-only hit pulls them all, raw last, and an invalid
    /// byte still reads as U+FFFD.
    #[test]
    fn contains_stops_at_the_first_hit_and_reaches_raw_last() {
        let mut m = mail();
        m.text = Some("early needle".into());
        m.subject = Some("needle again".into());
        m.raw = vec![b'n', 0xFF, b'x'];
        let re = search_needle("needle").unwrap();
        let mut pulled = 0;
        assert!(
            haystacks(&m)
                .inspect(|_| pulled += 1)
                .any(|h| re.is_match(&h))
        );
        assert_eq!(pulled, 1, "the raw source was never reached");
        assert_eq!(haystacks(&m).count(), 3, "text, subject, raw");
        assert_eq!(haystacks(&m).last().unwrap(), "n\u{FFFD}x");
        assert!(contains(&m, &search_needle("\u{FFFD}").unwrap()));
    }

    #[test]
    fn contains_searches_every_haystack() {
        let mut m = mail();
        m.to = vec![address(Some("To"), "to@x.io")];
        assert!(!contains(&m, &search_needle("anything").unwrap()));

        m.text = Some("plain text body".into());
        // The fold is case-insensitive at the leaf: a needle hits in any
        // case without either side being copied lowercase first.
        assert!(contains(&m, &search_needle("text body").unwrap()));
        assert!(contains(&m, &search_needle("TEXT BODY").unwrap()));
        m.html = Some("<i>markup</i>".into());
        assert!(contains(&m, &search_needle("<i>markup").unwrap()));
        m.subject = Some("A Subject".into());
        assert!(contains(&m, &search_needle("subject").unwrap()));
        m.cc = vec![address(None, "carbon-copy@x.io")];
        assert!(contains(&m, &search_needle("carbon-copy").unwrap()));
        m.from = Some(address(Some("F"), "from@x.io"));
        assert!(contains(&m, &search_needle("from@").unwrap()));
        // Only the raw source carries this needle (the DATA footer).
        assert!(contains(&m, &search_needle("raw source").unwrap()));
        assert!(!contains(
            &m,
            &search_needle("no such needle anywhere").unwrap()
        ));

        // The needle is a literal, never a pattern: the dot in `a.b` is
        // escaped, so it does not match across `a b` the way an
        // unescaped `.` would, and regex garbage (`([`) is escaped into
        // a valid literal rather than a pattern error.
        m.text = Some("a b".into());
        assert!(!contains(&m, &search_needle("a.b").unwrap()));
        m.text = Some("a.b".into());
        assert!(contains(&m, &search_needle("a.b").unwrap()));
        assert!(!contains(&m, &search_needle("([").unwrap()));

        // The engine's compiled-size budget is a real refusal for a
        // case-folded literal: a quarter-million `k`s each become a
        // three-way class under `(?i)` and the build yields None — the
        // caller refuses the request instead of unwinding.
        assert!(search_needle(&"k".repeat(250_000)).is_none());
    }

    /// Field syntax is recognized lexically (decision 0044 §7): a leading
    /// ASCII identifier and its colon, any case, any field name, spelled as
    /// supplied; a URL's `://` and a token that does not open with an
    /// identifier stay literal.
    #[test]
    fn field_prefixes_are_recognized_lexically() {
        for (token, field) in [
            ("to:bcc@x.io", Some("to")),
            ("From:alice", Some("From")),
            ("SUBJECT:x", Some("SUBJECT")),
            ("foo_bar-2:x", Some("foo_bar-2")),
            ("is:", Some("is")),
            ("https://host/path", None),
            ("mailto:x@y", Some("mailto")),
            ("plain", None),
            ("2fa:code", None),
            (":lead", None),
            ("ab.c:d", None),
            ("é:x", None),
            ("", None),
        ] {
            assert_eq!(field_prefix(token), field, "{token}");
        }
        let refused = |kind: Option<&str>, query: &str| {
            refuse_field_syntax(&SearchParams {
                kind: kind.map(str::to_string),
                query: Some(query.to_string()),
                ..Default::default()
            })
            .err()
            .map(|e| e.1)
        };
        assert_eq!(
            refused(None, "  plan subject:bcc").as_deref(),
            Some(
                "unsupported search prefix \"subject:\" — Mailpit's prefix grammar is not \
                 implemented; use ?kind=to|from|subject"
            )
        );
        assert_eq!(refused(Some("containing"), "see https://x.io/a:b"), None);
        // Quotes normalize before the single wrappers, for refusal only (0061 §1).
        for (token, field) in [
            ("-to:x", Some("to")),
            ("!from:x", Some("from")),
            ("\"to:x\"", Some("to")),
            ("-\"subject:x\"", Some("subject")),
            ("(to:x)", Some("to")),
            ("(!\"is:x", Some("is")),
            ("-https://x.io", None),
            ("-word", None),
            ("--to:x", None),
            ("\"(to:x)", Some("to")),
        ] {
            assert_eq!(wrapped_field_prefix(token).as_deref(), field, "{token}");
        }
        // An explicit kind takes its value literally.
        assert_eq!(refused(Some("to"), "to:x"), None);
    }

    /// The compat sender is the envelope when known — a bounce's empty
    /// reverse path included — and the header From only for a legacy row
    /// with no envelope (decision 0044 §8); `kind=from` matches it.
    #[test]
    fn the_compat_sender_prefers_the_known_envelope() {
        let mut m = mail();
        assert_eq!(compat_sender(&m), None);
        m.from = Some(address(Some("Alice"), "alice@x.io"));
        assert_eq!(compat_sender(&m), Some("alice@x.io"));
        m.envelope_from = Some("bounce+77@mailer.io".into());
        assert_eq!(compat_sender(&m), Some("bounce+77@mailer.io"));
        let from = |query: &str| {
            stage_search(&SearchParams {
                kind: Some("from".into()),
                query: Some(query.into()),
                ..Default::default()
            })
            .ok()
            .unwrap()
            .matches(&m)
        };
        assert!(from("BOUNCE"));
        assert!(!from("alice"));
        m.envelope_from = Some(String::new());
        assert_eq!(compat_sender(&m), Some(""));
        assert_eq!(mailpit_message(&m)["ReturnPath"], "");
        assert_eq!(hog_headers(&m)["Return-Path"], json!(["<>"]));
        assert_eq!(hog_item(&m)["Raw"]["From"], "");
    }

    #[test]
    fn hog_mime_is_null_without_bodies() {
        let mime = |m: &Email| serde_json::to_value(hog_mime(m)).unwrap();
        assert_eq!(mime(&mail()), Value::Null);
        let mut m = mail();
        m.text = Some("the text".into());
        assert_eq!(
            mime(&m),
            json!({"Content-Type": "", "Parts": [
                {"Headers": {"Content-Type": ["text/plain; charset=UTF-8"]}, "Body": "the text"}
            ]})
        );
        let mut m = mail();
        m.html = Some("<i>the html</i>".into());
        assert_eq!(
            mime(&m),
            json!({"Content-Type": "", "Parts": [
                {"Headers": {"Content-Type": ["text/html; charset=UTF-8"]}, "Body": "<i>the html</i>"}
            ]})
        );
        let mut m = mail();
        m.text = Some("t".into());
        m.html = Some("<i>h</i>".into());
        assert_eq!(mime(&m)["Parts"].as_array().unwrap().len(), 2);
    }

    /// The borrowed item keeps the complete wire shape (decision 0044 §6):
    /// one golden value per body case — text-only, html-only, multipart,
    /// no body — and invalid raw bytes read lossily.
    #[test]
    fn the_borrowed_hog_item_keeps_the_whole_shape() {
        let mut m = mail();
        m.from = Some(address(Some("Hdr"), "hdr@x.io"));
        m.envelope_from = Some("env@x.io".into());
        m.recipients = vec!["r@x.io".into()];
        m.text = Some("t".into());
        m.raw = vec![b'r', 0xFF];
        let text_part =
            json!({"Headers": {"Content-Type": ["text/plain; charset=UTF-8"]}, "Body": "t"});
        let html_part =
            json!({"Headers": {"Content-Type": ["text/html; charset=UTF-8"]}, "Body": "<b>h</b>"});
        let expected = |body: &str, mime: Value| {
            json!({
                "ID": "id-1",
                "From": {"Relays": null, "Mailbox": "env", "Domain": "x.io", "Params": ""},
                "To": [{"Relays": null, "Mailbox": "r", "Domain": "x.io", "Params": ""}],
                "Content": {"Headers": hog_headers(&m), "Body": body, "Size": 42, "MIME": mime.clone()},
                "Created": "2026-02-03T04:05:06Z",
                "MIME": mime,
                "Raw": {"From": "env@x.io", "To": ["r@x.io"], "Data": "r\u{FFFD}", "Helo": ""},
            })
        };
        assert_eq!(
            hog_item(&m),
            expected(
                "t",
                json!({"Content-Type": "", "Parts": [text_part.clone()]})
            )
        );
        // The synthesized headers do not depend on the bodies, so one
        // expectation builder serves every body case.
        let mut both = m.clone();
        both.html = Some("<b>h</b>".into());
        assert_eq!(
            hog_item(&both),
            expected(
                "t",
                json!({"Content-Type": "", "Parts": [text_part, html_part.clone()]})
            )
        );
        let mut html_only = both.clone();
        html_only.text = None;
        assert_eq!(
            hog_item(&html_only),
            expected(
                "<b>h</b>",
                json!({"Content-Type": "", "Parts": [html_part]})
            )
        );
        let mut bare = html_only.clone();
        bare.html = None;
        assert_eq!(hog_item(&bare), expected("", Value::Null));
    }

    /// The capped buffer counts the bytes actually written (decision 0044
    /// §6): a page fits at exactly its serialized size and refuses at one
    /// byte less — JSON escapes and the duplicated MIME included — and the
    /// refused buffer never exceeded its budget.
    #[test]
    fn the_page_budget_fits_exactly_and_refuses_the_next_byte() {
        let mut m = mail();
        m.text = Some("quote \" backslash \\ newline \n tab \t".into());
        let page = vec![Arc::new(m)];
        let exact = hog_page_body(1, 0, &page, usize::MAX).unwrap();
        let parsed: Value = serde_json::from_slice(&exact).unwrap();
        assert_eq!(
            parsed["items"][0]["MIME"],
            parsed["items"][0]["Content"]["MIME"]
        );
        assert_eq!(parsed["count"], 1);
        assert_eq!(hog_page_body(1, 0, &page, exact.len()).unwrap(), exact);
        assert_eq!(hog_page_body(1, 0, &page, exact.len() - 1), Err(OverBudget));
        // The text appears three times (Body and both MIME parts), escaped.
        let escaped = r#"quote \" backslash \\ newline \n tab \t"#;
        assert_eq!(
            String::from_utf8(exact).unwrap().matches(escaped).count(),
            3
        );

        use std::io::Write as _;
        let mut buffer = CappedBuffer {
            bytes: Vec::new(),
            budget: 5,
        };
        buffer.write_all(b"abc").unwrap();
        assert!(
            buffer.write_all(b"def").is_err(),
            "no partial chunk is taken"
        );
        assert_eq!(buffer.bytes, b"abc");
        buffer.write_all(b"de").unwrap();
        buffer.flush().unwrap();
        assert_eq!(buffer.bytes, b"abcde");
        assert!(
            buffer.bytes.capacity() <= 5,
            "growth is capped at the budget"
        );
        assert!(buffer.write_all(b"f").is_err());
    }

    #[test]
    fn hog_headers_omits_absent_fields() {
        // Bare mail: only the honest stamps are present.
        let h = hog_headers(&mail());
        for absent in ["From", "To", "Cc", "Subject", "Message-ID", "Return-Path"] {
            assert!(h.get(absent).is_none(), "{absent} must be absent");
        }
        assert_eq!(h["Date"], json!(["2026-02-03T04:05:06Z"]));
        assert!(
            h["Received"][0]
                .as_str()
                .unwrap()
                .starts_with("from swarmail-smtp by Swarmail; 2026-02-03")
        );

        let mut m = mail();
        m.from = Some(address(Some("From Name"), "from@x.io"));
        m.to = vec![address(None, "to@x.io")];
        m.cc = vec![address(None, "cc@x.io")];
        m.subject = Some("Subject".into());
        m.message_id = Some("<m-id@x.io>".into());
        let h = hog_headers(&m);
        assert_eq!(h["From"], json!(["From Name <from@x.io>"]));
        assert_eq!(h["To"], json!(["to@x.io"]));
        assert_eq!(h["Cc"], json!(["cc@x.io"]));
        assert_eq!(h["Subject"], json!(["Subject"]));
        assert_eq!(h["Message-ID"], json!(["<m-id@x.io>"]));
        assert_eq!(h["Return-Path"], json!(["<from@x.io>"]));
    }

    #[test]
    fn mailpit_message_defaults_without_a_from() {
        let m = mailpit_message(&mail());
        assert_eq!(m["From"], Value::Null);
        assert_eq!(m["ReturnPath"], "");
        assert_eq!(m["Text"], "");
        assert_eq!(m["HTML"], "");
        assert_eq!(m["Subject"], "");
        // No Message-ID header: the swarmail id anchors the MessageID.
        assert_eq!(m["MessageID"], "id-1");
        assert_eq!(m["Date"], "2026-02-03T04:05:06Z");
    }

    #[test]
    fn hog_message_body_falls_back_to_the_html_part() {
        let mut m = mail();
        m.html = Some("<i>the html</i>".into());
        let hog = hog_item(&m);
        assert_eq!(hog["Content"]["Body"], "<i>the html</i>");
        assert_eq!(hog["Raw"]["From"], "");
        assert_eq!(hog["Raw"]["Data"], "raw source");
    }

    /// The window helpers: a page past the end is empty, a short tail is
    /// what remains, and the global mailbox size sums every inbox.
    #[test]
    fn pages_and_scopes_are_exact() {
        let all: Vec<Arc<Email>> = (0..3).map(|_| Arc::new(mail())).collect();
        assert_eq!(page_of(&all, 0, 2).len(), 2);
        assert_eq!(page_of(&all, 2, 250).len(), 1);
        assert_eq!(page_of(&all, 3, 250).len(), 0);
        assert_eq!(page_of(&all, usize::MAX, 250).len(), 0);
        let store = Store::new(0);
        store
            .insert(crate::store::email("a", "one", "x@x.io"))
            .unwrap();
        store
            .insert(crate::store::email("b", "two", "x@x.io"))
            .unwrap();
        assert_eq!(mailbox_size(&store, None), 2);
        assert_eq!(mailbox_size(&store, Some("one")), 1);
        assert_eq!(mailbox_size(&store, Some("ghost")), 0);
        assert_eq!(scope_inbox(None).unwrap(), None);
        assert_eq!(
            scope_inbox(Some(" x ".into())).unwrap().as_deref(),
            Some(" x ")
        );
        assert_eq!(
            scope_inbox(Some(" \u{3000}".into())).unwrap_err().1,
            crate::model::BLANK_INBOX_REFUSAL
        );
    }
}
