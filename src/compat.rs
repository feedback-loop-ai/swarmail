//! MailHog / Mailpit HTTP-API compat shims.
//!
//! The well-known read endpoints of the two mail-sink tools, answered from
//! the SAME store — swarmail's inboxes stay the only mailbox model. A
//! request that does not name an inbox reads the whole store as one global
//! mailbox (that is what both tools expose); `?inbox=<name>` scopes a list
//! or search to a single swarmail inbox, which is the natural way a per-test
//! harness points the tool at its own mail.
//!
//! Shapes follow the upstream Go structs byte for byte where clients depend
//! on them, including quirks: Mailpit lowercases `mail.Address` JSON keys
//! (`{"name":..,"address":..}`) and drops an empty name, while MailHog uses
//! `Path{Relays,Mailbox,Domain,Params}` and a `MIME` field that is `null`
//! for non-MIME mail. Header maps are synthesized from the fields extract
//! extracted at ingest (decision 0005) — no query-time MIME re-parsing.
//!
//! Deliberate omissions live in README.md (`Compat shims`); the headline
//! ones: no attachments, no read/unread state, no send-API, and no
//! `/api/v1/messages/{id}/mime/part/...`.

use crate::api::{ApiError, AppState};
use crate::model::{Email, EmailAddress};
use crate::store::Filter;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get};
use mail_parser::decoders::html::html_to_text;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::sync::Arc;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/messages/{id}/plain", get(message_plain_plural))
        .route("/api/v1/messages/{id}/download", get(message_download))
        .route("/api/v1/delete-all", delete(delete_all))
        .route("/api/v1/message/latest", get(message_latest))
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
/// out of the query text).
#[derive(Debug, Default, Deserialize)]
pub struct SearchParams {
    pub kind: Option<String>,
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub start: Option<usize>,
    pub inbox: Option<String>,
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

// ---------- Mailpit: summary envelope ----------

/// The shared page cap through this plane's `ApiError` envelope (decision
/// 0030 §3): the sentence stays the shared one, the status stays `400`
/// JSON on all four pages — including MailHog's search, whose *kind/query*
/// postures stay bare 400s.
fn page_cap(field: &str, n: usize) -> Result<usize, ApiError> {
    crate::api::checked_page_size(field, n).map_err(crate::api::bad_request)
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

/// GET /api/v1/messages — Mailpit's paginated summary envelope.
pub async fn list_messages_mailpit(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    // The cap is staged before the scan (decision 0030 §3).
    let limit = page_limit(params.limit)?;
    let all = scoped(&state, &params.inbox);
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all.iter().skip(start).take(limit).cloned().collect();
    Ok(Json(mailpit_envelope(all.len(), all.len(), start, &page)))
}

/// GET /api/v1/search — Mailpit's search envelope; empty query is a 400.
async fn search_mailpit(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, ApiError> {
    // The cap is staged before the scan (decision 0030 §3).
    let limit = page_limit(params.limit)?;
    // Mailpit's prefix grammar (`to:…`, `from:…`, quoted phrases) is its
    // own parser — not implemented here. The one prefix the issue names is
    // refused by name (0025's discipline): a silently empty result is the
    // plausible-wrong answer, and `?kind=` is the honest route. The rest
    // of the grammar is README-listed as a known difference.
    if params
        .query
        .as_deref()
        .is_some_and(|q| q.to_lowercase().starts_with("to:"))
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "unsupported search prefix \"to:\" — Mailpit's prefix grammar is not implemented; use ?kind=to".into(),
        ));
    }
    let all = run_search(&state, &params)?;
    // Mailpit's search envelope reports `total`/`unread` for the whole
    // mailbox — the unfiltered scope (issue #55) — while the match count
    // rides `messages_count`. Nothing is ever unread, so `unread` is the
    // mailbox's own size.
    let mailbox = scoped(&state, &params.inbox).len();
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all.iter().skip(start).take(limit).cloned().collect();
    Ok(Json(mailpit_envelope(mailbox, all.len(), start, &page)))
}

/// Mailpit's summary envelope. `mailbox` is the unfiltered scope's count —
/// upstream fills `total`/`unread` from `storage.StatsGet()`, which knows
/// the mailbox, not the page. `matches` is the row count this request
/// matched: the list endpoint reports it as the mailbox's own size, the
/// search endpoint as the query's hit count (`messages_count`).
fn mailpit_envelope(mailbox: usize, matches: usize, start: usize, page: &[Arc<Email>]) -> Value {
    json!({
        "total": mailbox,
        // Nothing in swarmail is ever unread — there is no read state.
        "unread": mailbox,
        "count": page.len(),
        "messages_count": matches,
        "messages_unread_count": matches,
        "start": start,
        "tags": [],
        "messages": page.iter().map(|e| mailpit_summary(e)).collect::<Vec<_>>(),
    })
}

/// GET /api/v1/message/latest — the newest stored mail in Mailpit's
/// message shape (issue #55): the route Mailpit's UI lands on before an
/// id is chosen. 404 only when the store is empty — the honest refusal;
/// there is no id to fabricate. The newest-first ordering is the store's
/// own (the same one every listing uses).
async fn message_latest(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let all = state.store.list_all(&Filter::default());
    all.first()
        .map(|email| Json(mailpit_message(email)))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

/// GET /api/v1/message/{id} — Mailpit's full message shape.
async fn message_mailpit(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .store
        .get(&id)
        .map(|e| Json(mailpit_message(&e)))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

/// GET /api/v1/message/{id}/plain — the text body as text/plain.
async fn message_plain(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    state
        .store
        .get(&id)
        .map(|e| plain_response(&e))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

/// GET /api/v1/message/{id}/raw — the full source, as MailHog serves it.
async fn message_raw(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    state
        .store
        .get(&id)
        .map(|e| raw_response(&e))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

/// GET /api/v1/message/{id}/headers — just the synthesized header map.
async fn message_headers(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .store
        .get(&id)
        .map(|e| Json(hog_headers(&e)))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

/// GET /api/v1/messages/{id}/plain — the plural path MailHog clients use.
async fn message_plain_plural(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    state
        .store
        .get(&id)
        .map(|e| plain_response(&e))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

/// GET /api/v1/messages/{id}/download — MailHog's .eml attachment download.
async fn message_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    state
        .store
        .get(&id)
        .map(|e| raw_response(&e))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "message not found".into()))
}

// ---------- MailHog v2 ----------

/// GET /api/v2/messages — `{total, count, start, items: [Message...]}`.
async fn list_messages_hog(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    // The cap is staged before the scan (decision 0030 §3).
    let limit = page_limit(params.limit)?;
    let all = scoped(&state, &params.inbox);
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all.iter().skip(start).take(limit).cloned().collect();
    Ok(Json(json!({
        "total": all.len(),
        "count": page.len(),
        "start": start,
        "items": page.iter().map(|e| hog_message(e)).collect::<Vec<_>>(),
    })))
}

/// The two 400 shapes the Hog search answers: the dialect quirks (an
/// unknown kind, an empty query, an unrunnable search) stay **bare** —
/// upstream's byte-exact refusal, no body — while the shared page-cap
/// refusal rides the JSON envelope like every other plane's (decision
/// 0030 §3). A plain `Response` error leg would be an oversized `Err` —
/// this is the same wire at four bytes.
enum HogRejection {
    Bare(StatusCode),
    PageCap(ApiError),
}

impl IntoResponse for HogRejection {
    fn into_response(self) -> Response {
        match self {
            Self::Bare(code) => code.into_response(),
            Self::PageCap(err) => err.into_response(),
        }
    }
}

/// GET /api/v2/search — kind ∈ {from,to,containing}; a bare 400 otherwise,
/// exactly as upstream answers (no body, no message). The page cap is the
/// one exception (decision 0030 §3): it rides the shared `400` JSON
/// envelope, so its arm is `PageCap`.
async fn search_hog(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, HogRejection> {
    // The cap is staged before the scan (decision 0030 §3). The bare-400
    // quirks — an empty query — ride `run_search`'s own refusals: the
    // map_err below is the one place a bad staging lands, so the envelope
    // stays upstream's bare `400` with no body. The KIND is the dialect's
    // own domain, staged here and not widened by the shared engine:
    // upstream answers a bare 400 for anything but from|to|containing —
    // including an *absent* kind, whose `""` fails the same whitelist
    // (MailHog-Server `api/v2.go`, `search()`: `kind != "from" && kind !=
    // "to" && kind != "containing"` → 400) — and `subject`, the swarmail
    // engine's own kind, is not this dialect's contract (0029's
    // advertised-domain discipline).
    match params.kind.as_deref() {
        Some("from") | Some("to") | Some("containing") => {}
        _ => return Err(HogRejection::Bare(StatusCode::BAD_REQUEST)),
    }
    let limit = page_limit(params.limit).map_err(HogRejection::PageCap)?;
    let all =
        run_search(&state, &params).map_err(|_| HogRejection::Bare(StatusCode::BAD_REQUEST))?;
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all.iter().skip(start).take(limit).cloned().collect();
    Ok(Json(json!({
        "total": all.len(),
        "count": page.len(),
        "start": start,
        "items": page.iter().map(|e| hog_message(e)).collect::<Vec<_>>(),
    })))
}

// ---------- deletes ----------

/// DELETE /api/v1/delete-all — MailHog's alias for wiping the mailbox.
///
/// A failed commit is the answer (decision 0014): `503` with the fixed
/// text, never a count for work that did not commit.
async fn delete_all(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
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

/// The store as one mailbox (all inboxes), or one inbox when scoped.
fn scoped(state: &AppState, inbox: &Option<String>) -> Vec<Arc<Email>> {
    match inbox {
        Some(inbox) => state.store.list(inbox, &Filter::default()),
        None => state.store.list_all(&Filter::default()),
    }
}

/// Validate + run a search: newest-first, unpaginated (the caller pages).
fn run_search(state: &AppState, params: &SearchParams) -> Result<Vec<Arc<Email>>, ApiError> {
    let query = params.query.as_deref().map(str::trim).unwrap_or("");
    if query.is_empty() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Error: no search query".into(),
        ));
    }
    let kind = params.kind.as_deref().unwrap_or("containing");
    let filter = match kind {
        "to" => Filter {
            to: Some(query.to_string()),
            ..Default::default()
        },
        "from" => Filter {
            from: Some(query.to_string()),
            ..Default::default()
        },
        "subject" => Filter {
            subject: Some(query.to_string()),
            ..Default::default()
        },
        "containing" => Filter::default(),
        other => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                format!("unknown search kind: {other}"),
            ));
        }
    };
    let mut all = match &params.inbox {
        Some(inbox) => state.store.list(inbox, &filter),
        None => state.store.list_all(&filter),
    };
    if kind == "containing" {
        // The needle keeps its call-site lowering — per request, once; the
        // per-mail cost the issue named was the two full-size copies per
        // candidate, and the fold below replaces exactly those (decision
        // 0030 §4). A needle the engine refuses to compile is refused by
        // the plane — a 400, not an unwind: the query is the client's
        // own input, and a request that cannot be served as asked is a
        // bad request.
        let needle = query.to_lowercase();
        let re = search_needle(&needle).ok_or(ApiError(
            StatusCode::BAD_REQUEST,
            "Error: the search needle is too complex".into(),
        ))?;
        all.retain(|e| contains(e, &re));
    }
    Ok(all)
}

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

/// Substring hit across everything extract found plus the raw source. Every
/// candidate is examined (the fold never short-circuits — a field later in
/// the mail can hit when an earlier one misses) and no haystack is copied:
/// the fields match in place, the raw source only through the lossy borrow
/// `String::from_utf8_lossy` already is.
pub fn contains(email: &Email, re: &Regex) -> bool {
    let mut hit = false;
    if let Some(text) = &email.text {
        hit |= re.is_match(text);
    }
    if let Some(html) = &email.html {
        hit |= re.is_match(html);
    }
    if let Some(subject) = &email.subject {
        hit |= re.is_match(subject);
    }
    for address in email.to.iter().chain(email.cc.iter()) {
        hit |= re.is_match(&address.address);
    }
    if let Some(from) = &email.from {
        hit |= re.is_match(&from.address);
    }
    hit |= re.is_match(&String::from_utf8_lossy(&email.raw));
    hit
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

/// Up to 200 characters of the body, whitespace-collapsed, `...`-suffixed
/// when truncated — Mailpit's snippet, without the HTML stripping (swarmail
/// prefers the text part the sender actually sent).
fn snippet(email: &Email) -> String {
    // The stored record is sender-honest (decision 0031 §1): when the sender
    // sent only an html part there is no `text`, so the snippet renders that
    // html at display time. That is one of exactly two sanctioned renderings
    // (decision 0031 §3) — a view for a person reading the list, not a body
    // to store or extract from: no links or codes are derived here, and
    // nothing is written back. The render is bounded by the stored body's
    // own cap (0022).
    let mut text = match &email.text {
        Some(text) => text.clone(),
        None => email
            .html
            .as_ref()
            .map(|h| html_to_text(h))
            .unwrap_or_default(),
    };
    text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() > 200 {
        let cut: String = text.chars().take(200).collect();
        format!("{cut}...")
    } else {
        text
    }
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
    // Swarmail records only the header From; Mailpit's Return-Path is the
    // envelope sender, so the closest truthful value is that same address.
    m.insert(
        "ReturnPath".into(),
        json!(
            email
                .from
                .as_ref()
                .map(|a| a.address.clone())
                .unwrap_or_default()
        ),
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

/// MailHog's `Path`: an address split at its domain.
fn hog_path(address: Option<&EmailAddress>) -> Value {
    hog_path_str(address.map(|a| a.address.as_str()).unwrap_or_default())
}

/// The same split for a bare envelope string — MailHog fills `To` from the
/// RCPT TO list (issue #55), which carries addresses without display names.
fn hog_path_str(address: &str) -> Value {
    let (mailbox, domain) = match address.rsplit_once('@') {
        Some((mailbox, domain)) => (mailbox.to_string(), domain.to_string()),
        None => (address.to_string(), String::new()),
    };
    json!({ "Relays": Value::Null, "Mailbox": mailbox, "Domain": domain, "Params": "" })
}

/// MailHog's Content.Headers, synthesized from extracted fields (decision
/// 0005) plus the honest Received/Return-Path stamps MailHog also adds.
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
        email.from.as_ref().map(|a| format!("<{}>", a.address)),
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

/// MailHog's MIME structure for the extracted parts, or `null` when the
/// message has neither a text nor an HTML body.
fn hog_mime(email: &Email) -> Value {
    let mut parts = Vec::new();
    if let Some(text) = &email.text {
        parts.push(json!({
            "Headers": { "Content-Type": ["text/plain; charset=UTF-8"] },
            "Body": text,
        }));
    }
    if let Some(html) = &email.html {
        parts.push(json!({
            "Headers": { "Content-Type": ["text/html; charset=UTF-8"] },
            "Body": html,
        }));
    }
    if parts.is_empty() {
        Value::Null
    } else {
        json!({ "Content-Type": "", "Parts": parts })
    }
}

/// MailHog's data.Message — the v1 list item and v2 search item. `To` and
/// `Raw.To` hold the *envelope* recipients (issue #55): upstream MailHog
/// fills them from the RCPT TO list, while the header rows stay in
/// `Content.Headers`.
fn hog_message(email: &Email) -> Value {
    let mime = hog_mime(email);
    json!({
        "ID": email.id,
        "From": hog_path(email.from.as_ref()),
        "To": email.recipients.iter().map(|r| hog_path_str(r)).collect::<Vec<_>>(),
        "Content": {
            "Headers": hog_headers(email),
            // The body as a client displays it: the text part, else the HTML.
            "Body": email.text.clone().or_else(|| email.html.clone()).unwrap_or_default(),
            "Size": email.size,
            "MIME": mime.clone(),
        },
        "Created": date_of(email),
        "MIME": mime,
        "Raw": {
            "From": email.from.as_ref().map(|a| a.address.clone()).unwrap_or_default(),
            "To": email.recipients.clone(),
            "Data": String::from_utf8_lossy(&email.raw),
            // Swarmail does not record the EHLO name.
            "Helo": "",
        },
    })
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
mod tests {
    use super::*;

    /// A minimal stored email; tests mutate the fields they exercise.
    fn mail() -> Email {
        Email {
            id: "id-1".into(),
            inbox: "inbox".into(),
            from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: None,
            received_at: "2026-02-03T04:05:06Z".into(),
            received_ms: 1_770_000_306_000,
            size: 42,
            text: None,
            html: None,
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
        assert_eq!(
            hog_path(None),
            json!({"Relays": null, "Mailbox": "", "Domain": "", "Params": ""})
        );
        assert_eq!(
            hog_path(Some(&address(None, "mailbox@domain"))),
            json!({"Relays": null, "Mailbox": "mailbox", "Domain": "domain", "Params": ""})
        );
        assert_eq!(
            hog_path(Some(&address(None, "just-a-mailbox"))),
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

    #[test]
    fn hog_mime_is_null_without_bodies() {
        assert_eq!(hog_mime(&mail()), Value::Null);
        let mut m = mail();
        m.text = Some("the text".into());
        assert_eq!(
            hog_mime(&m),
            json!({"Content-Type": "", "Parts": [
                {"Headers": {"Content-Type": ["text/plain; charset=UTF-8"]}, "Body": "the text"}
            ]})
        );
        let mut m = mail();
        m.html = Some("<i>the html</i>".into());
        assert_eq!(
            hog_mime(&m),
            json!({"Content-Type": "", "Parts": [
                {"Headers": {"Content-Type": ["text/html; charset=UTF-8"]}, "Body": "<i>the html</i>"}
            ]})
        );
        let mut m = mail();
        m.text = Some("t".into());
        m.html = Some("<i>h</i>".into());
        assert_eq!(hog_mime(&m)["Parts"].as_array().unwrap().len(), 2);
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
        let hog = hog_message(&m);
        assert_eq!(hog["Content"]["Body"], "<i>the html</i>");
        assert_eq!(hog["Raw"]["From"], "");
        assert_eq!(hog["Raw"]["Data"], "raw source");
    }
}
