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
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::sync::Arc;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/messages/{id}/plain", get(message_plain_plural))
        .route("/api/v1/messages/{id}/download", get(message_download))
        .route("/api/v1/delete-all", delete(delete_all))
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

/// Mailpit's delete body (`{"ids": [...]}`); absent/empty means delete all.
#[derive(Debug, Default, Deserialize)]
pub struct IdsBody {
    #[serde(default)]
    pub ids: Vec<String>,
}

// ---------- Mailpit: summary envelope ----------

/// GET /api/v1/messages — Mailpit's paginated summary envelope.
pub async fn list_messages_mailpit(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<Value> {
    let all = scoped(&state, &params.inbox);
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all
        .iter()
        .skip(start)
        .take(params.limit.unwrap_or(50))
        .cloned()
        .collect();
    Json(mailpit_envelope(&all, start, &page))
}

/// GET /api/v1/search — Mailpit's search envelope; empty query is a 400.
async fn search_mailpit(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, ApiError> {
    let all = run_search(&state, &params)?;
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all
        .iter()
        .skip(start)
        .take(params.limit.unwrap_or(50))
        .cloned()
        .collect();
    Ok(Json(mailpit_envelope(&all, start, &page)))
}

fn mailpit_envelope(all: &[Arc<Email>], start: usize, page: &[Arc<Email>]) -> Value {
    json!({
        "total": all.len(),
        // Nothing in swarmail is ever unread — there is no read state.
        "unread": all.len(),
        "count": page.len(),
        "messages_count": all.len(),
        "messages_unread_count": all.len(),
        "start": start,
        "tags": [],
        "messages": page.iter().map(|e| mailpit_summary(e)).collect::<Vec<_>>(),
    })
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
) -> Json<Value> {
    let all = scoped(&state, &params.inbox);
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all
        .iter()
        .skip(start)
        .take(params.limit.unwrap_or(50))
        .cloned()
        .collect();
    Json(json!({
        "total": all.len(),
        "count": page.len(),
        "start": start,
        "items": page.iter().map(|e| hog_message(e)).collect::<Vec<_>>(),
    }))
}

/// GET /api/v2/search — kind ∈ {from,to,containing}; a bare 400 otherwise,
/// exactly as upstream answers (no body, no message).
async fn search_hog(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, StatusCode> {
    match params.kind.as_deref() {
        Some("from") | Some("to") | Some("containing") | None => {}
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    if params.query.as_deref().unwrap_or("").trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let all = run_search(&state, &params).map_err(|_| StatusCode::BAD_REQUEST)?;
    let start = params.start.unwrap_or(0);
    let page: Vec<Arc<Email>> = all
        .iter()
        .skip(start)
        .take(params.limit.unwrap_or(50))
        .cloned()
        .collect();
    Ok(Json(json!({
        "total": all.len(),
        "count": page.len(),
        "start": start,
        "items": page.iter().map(|e| hog_message(e)).collect::<Vec<_>>(),
    })))
}

// ---------- deletes ----------

/// DELETE /api/v1/delete-all — MailHog's alias for wiping the mailbox.
async fn delete_all(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "removed": state.store.clear_all() }))
}

/// DELETE /api/v1/messages — MailHog deletes all; Mailpit sends
/// `{"ids": [...]}` and deletes exactly those. Both work: an absent, empty
/// or unparseable body wipes everything, like upstream's decoders do.
pub async fn delete_messages(
    State(state): State<AppState>,
    body: Result<Json<IdsBody>, JsonRejection>,
) -> Json<Value> {
    let ids = body
        .ok()
        .and_then(|Json(b)| if b.ids.is_empty() { None } else { Some(b.ids) });
    let removed = match ids {
        Some(ids) => ids.iter().filter(|id| state.store.delete(id)).count(),
        None => state.store.clear_all(),
    };
    Json(json!({ "removed": removed }))
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
        let needle = query.to_lowercase();
        all.retain(|e| contains(e, &needle));
    }
    Ok(all)
}

/// Substring hit across everything extract found plus the raw source. Every
/// candidate is collected (no short-circuit) so a miss looks like a hit.
fn contains(email: &Email, needle: &str) -> bool {
    let mut haystacks: Vec<String> = Vec::new();
    if let Some(text) = &email.text {
        haystacks.push(text.to_lowercase());
    }
    if let Some(html) = &email.html {
        haystacks.push(html.to_lowercase());
    }
    if let Some(subject) = &email.subject {
        haystacks.push(subject.to_lowercase());
    }
    for address in email.to.iter().chain(email.cc.iter()) {
        haystacks.push(address.address.to_lowercase());
    }
    if let Some(from) = &email.from {
        haystacks.push(from.address.to_lowercase());
    }
    haystacks.push(String::from_utf8_lossy(&email.raw).to_lowercase());
    haystacks.iter().any(|h| h.contains(needle))
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

/// The `Created`/`Date`-style timestamp: swarmail stamps receive time.
fn date_of(email: &Email) -> String {
    email.received_at.clone()
}

/// Up to 200 characters of the body, whitespace-collapsed, `...`-suffixed
/// when truncated — Mailpit's snippet, without the HTML stripping (swarmail
/// extracts the text part at ingest, so prefer it).
fn snippet(email: &Email) -> String {
    let mut text = email
        .text
        .clone()
        .or_else(|| email.html.clone())
        .unwrap_or_default();
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
    m.insert("Bcc".into(), json!([]));
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
    m.insert("Bcc".into(), json!([]));
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
    let (mailbox, domain) = match address {
        None => (String::new(), String::new()),
        Some(a) => match a.address.rsplit_once('@') {
            Some((mailbox, domain)) => (mailbox.to_string(), domain.to_string()),
            None => (a.address.clone(), String::new()),
        },
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

/// MailHog's data.Message — the v1 list item and v2 search item.
fn hog_message(email: &Email) -> Value {
    let mime = hog_mime(email);
    json!({
        "ID": email.id,
        "From": hog_path(email.from.as_ref()),
        "To": email.to.iter().map(|a| hog_path(Some(a))).collect::<Vec<_>>(),
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
            "To": email.to.iter().map(|a| a.address.clone()).collect::<Vec<_>>(),
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
    let body = email
        .text
        .clone()
        .or_else(|| email.html.clone())
        .unwrap_or_default();
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response()
}

/// The full source as an .eml download — MailHog's download shape.
fn raw_response(email: &Email) -> Response {
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

        // The HTML part answers when there is no text part; nothing when
        // there is no body at all.
        let mut m = mail();
        m.html = Some("<p>hi there</p>".into());
        assert_eq!(snippet(&m), "<p>hi there</p>");
        assert_eq!(snippet(&mail()), "");
    }

    #[test]
    fn contains_searches_every_haystack() {
        let mut m = mail();
        m.to = vec![address(Some("To"), "to@x.io")];
        assert!(!contains(&m, "anything"));

        m.text = Some("plain text body".into());
        // Haystacks are lowercased by the caller; the needle arrives lowercase.
        assert!(contains(&m, "text body"));
        m.html = Some("<i>markup</i>".into());
        assert!(contains(&m, "<i>markup"));
        m.subject = Some("A Subject".into());
        assert!(contains(&m, "subject"));
        m.cc = vec![address(None, "carbon-copy@x.io")];
        assert!(contains(&m, "carbon-copy"));
        m.from = Some(address(Some("F"), "from@x.io"));
        assert!(contains(&m, "from@"));
        // Only the raw source carries this needle (the DATA footer).
        assert!(contains(&m, "raw source"));
        assert!(!contains(&m, "no such needle anywhere"));
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
