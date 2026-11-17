//! Embedded web UI. The agent-native surfaces (REST/MCP) stay the primary
//! interface; this is for humans watching a run. `/`, the inbox view and the
//! thread view are one live **run console** (issue #355): a status bar over
//! three panes (inboxes, stream, reader). The server ships a thin shell and
//! the first paint of the inbox rail; the browser paints the rest from the
//! JSON API (`/api/v1/inboxes/{inbox}/threads[...]`, the per-message
//! projection and deliverability report) and the SSE feed. All mail-controlled
//! text is rendered with `textContent` in `static/ui.js` — never `innerHTML` —
//! so nothing the SMTP peer sent can become markup here. The one place mail
//! HTML is displayed is a separate document, `/ui/message/{id}/preview`
//! (decision 0135), framed by a bare-`sandbox` iframe and governed
//! by its own response headers.

mod preview;

use crate::api::{
    ApiError, AppState, DEFAULT_LIST_LIMIT, InboxPageQuery, bad_request, inbox_page, parse_query,
};
use crate::model::validate_inbox;
use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// The browser half of the live views, embedded at compile time. It contains
/// no mail data — only wiring — and no `</script>` sequence, so it can be
/// inlined into the shell.
const SCRIPT: &str = include_str!("static/ui.js");

/// The stylesheet of every UI page, embedded at compile time: system font
/// stacks, no `url(`, no `@import`, nothing that loads from outside.
const CSS: &str = include_str!("static/ui.css");

/// A path segment's escape set: everything but RFC 3986's unreserved
/// characters, so a message id reads as the script's `encodeURIComponent`
/// spells it.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// The tab icon: a static rendering of the project mark (assets/logo.svg,
/// without its animation), embedded at compile time and served same-origin,
/// which the pages' `img-src 'self'` admits.
const FAVICON: &str = include_str!("static/favicon.svg");

/// HTML-escape untrusted text for element content and attribute values.
/// Every interpolation of mail-controlled data goes through this.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The shell's resource policy (issue #56). Embedded inline script and style
/// are allowed; `connect-src 'self'` permits same-origin SSE. Images may load
/// from the same origin or data URLs, including same-origin image GETs.
/// `frame-src 'self'` admits exactly one kind of frame, the same-origin
/// preview document (decision 0135 §6): `default-src 'none'` would
/// otherwise refuse it. Other resource loads are refused. Every UI route
/// uses this one policy string.
const CSP: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-src 'self'";

/// The preview document's policy (decision 0135 §5), sent as a
/// response header: inline styles and `data:` images only, nothing remote,
/// no scripts, plugins, frames, base URL or form target, framed only by its
/// own origin, and a CSP `sandbox` without a single token — so the document
/// runs in an opaque origin with scripts, forms, popups and top navigation
/// off even when opened directly, outside the console's sandboxed frame.
const PREVIEW_CSP: &str = "default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox";

/// The same policy after the viewer asked to load remote content: images,
/// stylesheets and fonts may come from any http(s) host. Nothing else moves.
const PREVIEW_REMOTE_CSP: &str = "default-src 'none'; img-src data: https: http:; style-src 'unsafe-inline' https: http:; font-src https: http:; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox";

/// The one refusal of a preview query that is not empty or `remote=1`.
const PREVIEW_QUERY_REFUSAL: &str =
    "the preview query is empty or exactly remote=1; nothing else is accepted";

/// The trusted note under every preview frame (0135 §7): what can stay
/// broken and why, with the numeric bounds. Fixed text, never mail data.
const PREVIEW_NOTE: &str = "Rendered in a sandbox with scripts, forms and navigation off; remote images, stylesheets and fonts are blocked. Inline images show only for PNG, JPEG, GIF or WebP parts referenced as cid: in an img src, kept at ingest, at most 256 KiB each and 2 MiB per preview. Missing or ambiguous Content-IDs, SVG and other types, parts marked as attachments, CSS or srcset references, unstored or mismatched bytes, and mail over 8 MiB stay broken. HTML over 2 MiB, or a preview over 4 MiB, is refused.";

fn page(title: &str, body: String) -> Html<String> {
    let title = esc(title);
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="{CSP}"><title>{title} · Swarmail</title><link rel="icon" href="/favicon.svg" type="image/svg+xml"><style>{CSS}</style></head>
<body><div class="wrap"><h1>📮 Swarmail</h1><div class="muted">Built for a program waiting on mail. · <a href="/">inboxes</a> · <a href="/llms.txt">llms.txt</a> · <a href="/openapi.json">openapi</a></div>{body}</div></body></html>"#
    ))
}

/// The run console's logo, inline (the page loads nothing from outside).
const LOGO: &str = r##"<svg viewBox="0 0 150 150" aria-hidden="true"><rect x="6" y="6" width="138" height="138" rx="14" fill="none" stroke="#f0a232" stroke-width="8"/><rect x="35" y="47" width="80" height="56" rx="8" fill="none" stroke="#f0a232" stroke-width="9"/><path d="M35 63l40 26 40-26" fill="none" stroke="#f0a232" stroke-width="9" stroke-linecap="round" stroke-linejoin="round"/><circle cx="47" cy="30" r="7" fill="#7c5cff"/><circle cx="75" cy="30" r="7" fill="#7c5cff"/><circle cx="103" cy="30" r="8" fill="#f0a232"/></svg>"##;

/// The run console (issue #355): one page for `/`, the inbox view and the
/// thread view. No mail is rendered server-side. The `data-*` attributes are
/// the only state the script needs (`#view` names the inbox and thread,
/// `#inbox-rows` the page of the inbox list, `#panes` the pane to show before
/// any script runs); everything else comes from the JSON API and the feed.
/// `rail` is the first paint of the inbox pane, already escaped.
fn console(title: &str, pane: &str, view_attrs: &str, open: &str, rail: String) -> Html<String> {
    let title = esc(title);
    let open = esc(open);
    // `/` has no feed to follow; an inbox route connects as soon as the
    // script runs. The script, never this text, says "Live".
    let live = if pane == "inboxes" {
        "Idle"
    } else {
        "Connecting"
    };
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="{CSP}"><title>{title} · Swarmail</title><link rel="icon" href="/favicon.svg" type="image/svg+xml"><style>{CSS}</style></head>
<body class="app"><header id="status" class="bar"><div class="brand">{LOGO}Swarmail</div><div class="muted">Built for a program waiting on mail. · <a href="/">inboxes</a> · <a href="/llms.txt">llms.txt</a> · <a href="/openapi.json">openapi</a></div><div class="endpoints" aria-label="Listeners"><span class="ep" id="listener-smtp">SMTP <b>unknown</b></span><span class="ep" id="listener-pop3">POP3 <b>unknown</b></span><span class="ep" id="listener-http">HTTP <b>unknown</b></span></div><div class="stats tnum"><span title="Mail accepted over the store's lifetime"><b id="count-accepted">–</b> accepted (lifetime)</span><span class="dropped" title="Mail evicted by the per-inbox cap over the store's lifetime"><b id="count-dropped">–</b> dropped (evicted by cap, lifetime)</span></div><span class="spacer"></span><span class="chip chaos-unknown" id="chaos">Chaos unknown</span><span class="live" id="live"><span class="pulse"></span><span>{live}</span></span></header>
<main id="panes" data-pane="{pane}">{rail}<section id="stream" class="pane stream" aria-label="Messages"><div class="pane-head"><button class="btn ghost back-rail" id="back-rail" type="button" aria-label="Back to inboxes">←</button><span class="stream-title" id="stream-title">{open}</span><span class="muted tnum" id="stream-count"></span><span class="spacer"></span><span id="clear-slot"></span></div><div id="view" class="msgs" {view_attrs}></div></section><section id="reader" class="pane reader" aria-label="Message"><div class="pane-head"><button class="btn ghost back" id="back" type="button">← Messages</button></div><div class="reader-body" id="reader-body"></div></section></main>
<div class="toast" id="toast" hidden></div><noscript><div class="card muted">This console is live and needs JavaScript; the JSON API works without it.</div></noscript><script>{SCRIPT}</script></body></html>"#
    ))
}

/// The inbox pane's first paint: one link per inbox of the page, the open one
/// marked, the server's own empty sentence (the script only shows or hides
/// it), and the pager. The href is percent-encoded (a `#` or `?` in an inbox
/// name would otherwise be read as a fragment or query, and `a#b` would link
/// to `a`); the visible text stays escaped HTML.
fn rail(state: &AppState, limit: usize, offset: usize, open: &str) -> String {
    let total = state.store.inboxes().len();
    let mut rows = String::new();
    for summary in inbox_page(&state.store, limit, offset) {
        let href = utf8_percent_encode(&summary.name, NON_ALPHANUMERIC);
        let label = esc(&summary.name);
        let count = summary.count;
        let current = if summary.name == open {
            r#" aria-current="true""#
        } else {
            ""
        };
        rows.push_str(&format!(
            r#"<a class="inbox" href="/ui/inbox/{href}"{current}><span class="name">{label}</span><span class="count">{count}</span><span class="when"></span></a>"#
        ));
    }
    // The server chooses the sentence: the existing contract is that a
    // non-empty store's page never says "No inboxes yet".
    let (sentence, hidden) = if total == 0 {
        (
            "No inboxes yet — point SMTP at this server and send something.",
            "",
        )
    } else if rows.is_empty() {
        ("No inboxes on this page.", "")
    } else {
        ("No inboxes on this page.", " hidden")
    };
    let pager = pager(limit, offset, total);
    format!(
        r#"<nav id="inboxes" class="pane rail" aria-label="Inboxes"><div class="pane-head"><span class="label">Inboxes</span><span class="spacer"></span><span class="muted tnum" id="inbox-total"></span></div><div class="filter"><input id="inbox-filter" type="search" placeholder="Filter inboxes" aria-label="Filter inboxes" autocomplete="off"></div><div class="inboxes" id="inbox-rows" data-limit="{limit}" data-offset="{offset}">{rows}</div><div class="empty" id="inbox-empty"{hidden}>{sentence}</div><div class="empty problem" id="inbox-note" hidden></div>{pager}<div class="rail-foot">One inbox per test: the SMTP <code>AUTH</code> username picks it, or the recipient's domain routes it.</div></nav>"#
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/ui/inbox/{inbox}", get(inbox_view))
        .route("/ui/inbox/{inbox}/thread/{key}", get(thread_view))
        .route("/ui/message/{id}", get(message_view))
        // `/favicon.ico` answers too: clients that never read the page's
        // `<link rel="icon">` still ask for it, and an SVG body is fine.
        .route("/favicon.svg", get(favicon))
        .route("/favicon.ico", get(favicon))
        .layer(axum::middleware::map_response(refuse_framing))
        // Merged after the DENY layer, so only the preview document may be
        // framed — by its own origin (0135 §5).
        .merge(preview_router(Arc::new(Semaphore::new(
            preview::PREVIEW_JOBS,
        ))))
}

/// The preview route with its admission: one semaphore per router, so per
/// service, shared by every clone of it (0135 §4).
fn preview_router(permits: Arc<Semaphore>) -> Router<AppState> {
    Router::new()
        .route(
            "/ui/message/{id}/preview",
            get(
                move |State(state): State<AppState>, Path(id): Path<String>, raw: RawQuery| {
                    message_preview(permits.clone(), state, id, raw.0)
                },
            )
            .fallback(preview_method_not_allowed),
        )
        .layer(axum::middleware::map_response(preview_headers))
}

/// The tab icon, cacheable for a day: it changes only with a release.
async fn favicon() -> Response {
    (
        [
            (axum::http::header::CONTENT_TYPE, "image/svg+xml"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        FAVICON,
    )
        .into_response()
}

/// No other origin may frame the console (issue #355 review). The page's
/// CSP is a `<meta>` tag, which cannot carry `frame-ancestors`, and the
/// console holds the first destructive browser control (Clear inbox): a
/// framing page could clickjack it. The header is the only way to refuse.
async fn refuse_framing(mut res: Response) -> Response {
    res.headers_mut().insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    res
}

/// Every preview response — the document and each refusal — carries the
/// document headers (0135 §5): no caching, no sniffing, no referrer, framing
/// by the same origin only, and the default policy unless the handler chose
/// the remote one. Guard refusals answer before the router and keep their
/// own envelope.
async fn preview_headers(mut res: Response) -> Response {
    let headers = res.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("SAMEORIGIN"),
    );
    headers
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert(HeaderValue::from_static(PREVIEW_CSP));
    res
}

/// The preview answers GET and HEAD; any other method is a JSON 405.
async fn preview_method_not_allowed() -> Response {
    let mut res = ApiError(
        StatusCode::METHOD_NOT_ALLOWED,
        "the preview answers GET and HEAD only".into(),
    )
    .into_response();
    res.headers_mut()
        .insert(header::ALLOW, HeaderValue::from_static("GET,HEAD"));
    res
}

/// `GET /ui/message/{id}/preview[?remote=1]` (decision 0135): the
/// stored HTML as its own sandboxed document. The query is closed; a render
/// slot is taken without waiting, before the snapshot, and moves into the
/// blocking job; the mail is one `Arc` snapshot, so a prune after it cannot
/// cut the render short and nothing holds the store meanwhile.
async fn message_preview(
    permits: Arc<Semaphore>,
    state: AppState,
    id: String,
    query: Option<String>,
) -> Response {
    let remote = match query.as_deref() {
        None | Some("") => false,
        Some("remote=1") => true,
        Some(_) => {
            return ApiError(StatusCode::BAD_REQUEST, PREVIEW_QUERY_REFUSAL.into()).into_response();
        }
    };
    let Ok(permit) = permits.try_acquire_owned() else {
        return ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "preview busy; retry".into(),
        )
        .into_response();
    };
    let Some(email) = state.store.get(&id) else {
        return ApiError(StatusCode::NOT_FOUND, "message not found".into()).into_response();
    };
    preview_response(
        tokio::task::spawn_blocking(preview::job(permit, email)).await,
        remote,
    )
}

/// The answer for a finished (or failed) render. A worker that panicked is
/// a fixed 500 that carries no mail text.
fn preview_response(
    joined: Result<preview::Rendered, tokio::task::JoinError>,
    remote: bool,
) -> Response {
    match joined {
        Ok(preview::Rendered::Document(html)) => {
            let policy = if remote {
                PREVIEW_REMOTE_CSP
            } else {
                PREVIEW_CSP
            };
            (
                [
                    (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                    (header::CONTENT_SECURITY_POLICY, policy),
                ],
                html,
            )
                .into_response()
        }
        Ok(preview::Rendered::TooLarge) => ApiError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the preview exceeds its byte budget: 2 MiB of HTML, 4 MiB rendered".into(),
        )
        .into_response(),
        Err(_) => ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "preview rendering failed".into(),
        )
        .into_response(),
    }
}

/// The console on `/`, paged like `/api/v1/inboxes` (decision 0044 §3, issue
/// #91): the same closed `limit`/`offset` query, the same default and cap.
/// An empty store says so; an empty page beyond the end says only that. No
/// inbox is open, so the stream pane says to pick one.
async fn index(State(state): State<AppState>, raw: RawQuery) -> Result<Html<String>, ApiError> {
    let q: InboxPageQuery = parse_query(raw.0.as_deref().unwrap_or(""), "inboxes")?;
    let (limit, offset) = q.page().map_err(bad_request)?;
    Ok(console(
        "inboxes",
        "inboxes",
        r#"data-inbox="""#,
        "",
        rail(&state, limit, offset, ""),
    ))
}

/// The index's navigation. A positive page links back (saturating at the
/// first page) when it is not the first, and forward only when the checked
/// next offset still names an inbox — never a wrapped or zero-step link. A
/// zero-size page has no step at all: it links the default-size page at the
/// same offset instead. Every link keeps the page's own positive `limit`.
fn pager(limit: usize, offset: usize, total: usize) -> String {
    if limit == 0 {
        return format!(r#"<div class="muted"><a href="/?offset={offset}">default page</a></div>"#);
    }
    let mut links = Vec::new();
    if offset > 0 {
        let previous = offset.saturating_sub(limit);
        links.push(format!(
            r#"<a href="/?limit={limit}&amp;offset={previous}">previous</a>"#
        ));
    }
    if let Some(next) = offset.checked_add(limit).filter(|next| *next < total) {
        links.push(format!(
            r#"<a href="/?limit={limit}&amp;offset={next}">next</a>"#
        ));
    }
    if links.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="muted">{}</div>"#, links.join(" · "))
    }
}

/// The console on an inbox: the thread list, repainted from the feed as mail
/// arrives (no manual refresh). A blank inbox refuses (decision 0044 §3).
async fn inbox_view(
    State(state): State<AppState>,
    Path(inbox): Path<String>,
) -> Result<Html<String>, ApiError> {
    validate_inbox(&inbox).map_err(bad_request)?;
    Ok(console(
        &inbox,
        "stream",
        &format!(r#"data-inbox="{}""#, esc(&inbox)),
        &inbox,
        rail(&state, DEFAULT_LIST_LIMIT, 0, &inbox),
    ))
}

/// The console on one conversation, kept fresh too.
async fn thread_view(
    State(state): State<AppState>,
    Path((inbox, key)): Path<(String, String)>,
) -> Result<Html<String>, ApiError> {
    validate_inbox(&inbox).map_err(bad_request)?;
    Ok(console(
        "thread",
        "stream",
        &format!(
            r#"data-inbox="{}" data-thread="{}""#,
            esc(&inbox),
            esc(&key)
        ),
        &inbox,
        rail(&state, DEFAULT_LIST_LIMIT, 0, &inbox),
    ))
}

async fn message_view(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let Some(email) = state.store.get(&id) else {
        // The card body stays for humans; the status stops lying (issue
        // #56): a missing message answers 404, not a 200 card.
        return (
            StatusCode::NOT_FOUND,
            page(
                "not found",
                r#"<div class="card">Message not found (pruned or cleared).</div>"#.into(),
            ),
        )
            .into_response();
    };
    let report = state.store.report(email.clone()).await;
    crate::store::Store::offload(move || render_message(&email, report.as_deref())).await
}

/// The ARC chain line (decision 0133 section 2): the evaluator's own member,
/// escaped like every other provider field, never derived from the findings.
fn arc_line(arc: &crate::deliverability::ArcChain) -> String {
    let state = serde_json::to_value(arc.state).unwrap();
    let instance = arc
        .failing_instance
        .map(|instance| format!(" · instance {instance}"))
        .unwrap_or_default();
    let reason = arc
        .reason
        .as_deref()
        .map(|reason| format!(" · {}", esc(reason)))
        .unwrap_or_default();
    format!(
        "<div class=\"mono\">arc · {} · {} sets{instance}{reason}</div>",
        esc(state.as_str().unwrap()),
        arc.sets
    )
}

fn report_section(
    report: Result<&crate::deliverability::Report, &crate::deliverability::AnalysisError>,
) -> String {
    let body = match report {
        Err(error) => esc(error.message()),
        Ok(report) => {
            let mut body = format!("<div class=\"mono\">{}</div>", esc(&report.rule_set_id));
            body.push_str(&arc_line(&report.arc));
            if report.findings.is_empty() {
                body.push_str("<div>No findings from this rule set.</div>");
            }
            for finding in &report.findings {
                let severity = serde_json::to_value(finding.severity).unwrap();
                let locator = serde_json::to_string(&finding.location).unwrap();
                body.push_str(&format!(
                    "<div class=\"card mono\">{} · {} · {} · {}</div>",
                    esc(severity.as_str().unwrap()),
                    esc(&finding.rule_id),
                    esc(&locator),
                    esc(&finding.fix_hint)
                ));
            }
            body
        }
    };
    format!(
        "<h2>deliverability</h2><section>{body}<div class=\"muted\">Built-in rules only. Empty findings do not imply deliverability.</div></section>"
    )
}

fn render_message(
    email: &crate::model::Email,
    report: Result<&crate::deliverability::Report, &crate::deliverability::AnalysisError>,
) -> Response {
    let report_section = report_section(report);
    // The html preview is its own document (decision 0135), framed
    // by a bare sandbox. This page offers no remote toggle: the default
    // route only, with the same trusted note as the console.
    let html_body = email
        .html
        .as_ref()
        .map(|_| {
            let id = utf8_percent_encode(&email.id, SEGMENT);
            format!(
                r#"<iframe sandbox title="HTML preview, scripts disabled" src="/ui/message/{id}/preview"></iframe><div class="muted">{PREVIEW_NOTE}</div>"#
            )
        })
        .unwrap_or_default();
    let text_block = email
        .text
        .as_deref()
        .map(|t| format!(r#"<div class="card mono">{}</div>"#, esc(t)))
        .unwrap_or_default();
    let links = email
        .links
        .iter()
        .map(|l| {
            let l = esc(l);
            format!(r#"<div class="card mono"><a href="{l}" rel="noreferrer">{l}</a></div>"#)
        })
        .collect::<String>();
    let codes = email
        .codes
        .iter()
        .map(|c| format!(r#"<span class="pill mono">{}</span>"#, esc(c)))
        .collect::<String>();

    let from = email
        .from
        .as_ref()
        .map(|a| esc(&a.address))
        .unwrap_or_default();
    let to: Vec<String> = email.to.iter().map(|a| esc(&a.address)).collect();
    // The subject is escaped exactly once (issue #56): `page` escapes the
    // title; the `<h2>` below takes the escaped form. Passing the escaped
    // string through both would render `A &amp;amp; B` for `A & B`.
    let subject = email.subject.as_deref().unwrap_or("(no subject)");
    page(
        subject,
        format!(
            r#"<h2>{}</h2>
<div class="card"><b>From:</b> {from} &nbsp; <b>To:</b> {} &nbsp; <b>At:</b> {}</div>
{report_section}<h2>codes</h2><div>{codes}</div><h2>links</h2>{links}<h2>html</h2>{html_body}<h2>text</h2>{text_block}"#,
            esc(subject),
            to.join(", "),
            esc(&email.received_at),
        ),
    )
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview_state() -> AppState {
        AppState {
            store: Arc::new(crate::store::Store::new(10)),
            chaos: Arc::new(crate::chaos::Chaos::default()),
            webhooks: Arc::new(crate::webhook::Webhooks::default()),
            started: std::time::Instant::now(),
            shutdown: tokio::sync::watch::channel(false).1,
            smtp_degraded: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pop3_degraded: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            http_origin_guard: None,
            route_domain: None,
        }
    }

    fn stored(state: &AppState, html: &str) -> String {
        let raw =
            format!("From: s@x.io\r\nTo: r@x.io\r\nContent-Type: text/html\r\n\r\n{html}\r\n");
        let email =
            crate::smtp::build_email(raw.as_bytes(), "pv", &["r@x.io".into()], "s@x.io".into());
        state.store.insert(email).unwrap().id.clone()
    }

    async fn status_and_body(res: Response) -> (StatusCode, String) {
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    /// Two renders hold both slots; a third is refused without waiting, and
    /// a freed slot admits again. The slots are the router's own: another
    /// router's are untouched (0135 §4).
    #[tokio::test]
    async fn the_preview_admits_two_renders_and_refuses_a_third() {
        let state = preview_state();
        let id = stored(&state, "<p>hi</p>");
        let permits = Arc::new(Semaphore::new(preview::PREVIEW_JOBS));
        let first = permits.clone().try_acquire_owned().unwrap();
        let _second = permits.clone().try_acquire_owned().unwrap();
        let (status, body) = status_and_body(
            message_preview(permits.clone(), state.clone(), id.clone(), None).await,
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("preview busy; retry"), "{body}");
        let other = Arc::new(Semaphore::new(preview::PREVIEW_JOBS));
        let (status, _) =
            status_and_body(message_preview(other, state.clone(), id.clone(), None).await).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "another service's slots are its own"
        );
        drop(first);
        let (status, body) =
            status_and_body(message_preview(permits.clone(), state, id, None).await).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "<p>hi</p>\r\n"));
        assert_eq!(
            permits.available_permits(),
            1,
            "the render gave its slot back"
        );
    }

    /// The query is closed: absent, empty or exactly `remote=1`.
    #[tokio::test]
    async fn the_preview_query_is_closed() {
        let state = preview_state();
        let id = stored(&state, "<p>q</p>");
        let permits = Arc::new(Semaphore::new(preview::PREVIEW_JOBS));
        for query in [None, Some(""), Some("remote=1")] {
            let res = message_preview(
                permits.clone(),
                state.clone(),
                id.clone(),
                query.map(String::from),
            )
            .await;
            assert_eq!(res.status(), StatusCode::OK, "{query:?}");
        }
        for query in [
            "remote=0",
            "remote",
            "remote=",
            "remote=1&remote=1",
            "x=1",
            "remote=%31",
            "remote=1&x",
        ] {
            let (status, body) = status_and_body(
                message_preview(
                    permits.clone(),
                    state.clone(),
                    id.clone(),
                    Some(query.into()),
                )
                .await,
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
            assert!(body.contains(PREVIEW_QUERY_REFUSAL), "{query}: {body}");
        }
    }

    /// A snapshot taken before a prune renders to the end; a lookup after
    /// it is a 404. Nothing reserves the mail against eviction.
    #[tokio::test]
    async fn a_snapshot_outlives_a_prune_and_a_later_lookup_is_gone() {
        let state = preview_state();
        let id = stored(&state, "<p>kept</p>");
        let permits = Arc::new(Semaphore::new(preview::PREVIEW_JOBS));
        let snapshot = state.store.get(&id).unwrap();
        state.store.clear("pv").unwrap();
        let permit = permits.clone().try_acquire_owned().unwrap();
        let joined = tokio::task::spawn_blocking(preview::job(permit, snapshot)).await;
        let (status, body) = status_and_body(preview_response(joined, false)).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "<p>kept</p>\r\n"));
        let (status, body) = status_and_body(message_preview(permits, state, id, None).await).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("message not found"), "{body}");
    }

    /// Each render outcome's status, type and policy; a worker that failed
    /// is a fixed 500 that names nothing from the mail.
    #[tokio::test]
    async fn every_render_outcome_has_its_answer() {
        let document = || Ok(preview::Rendered::Document("<p>".into()));
        for (remote, policy) in [(false, PREVIEW_CSP), (true, PREVIEW_REMOTE_CSP)] {
            let res = preview_response(document(), remote);
            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(res.headers()[header::CONTENT_SECURITY_POLICY], policy);
            assert_eq!(
                res.headers()[header::CONTENT_TYPE],
                "text/html; charset=utf-8"
            );
        }
        let (status, body) =
            status_and_body(preview_response(Ok(preview::Rendered::TooLarge), false)).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(body.contains("byte budget"), "{body}");
        let failed = tokio::spawn(async { panic!("mail text that must not leak") })
            .await
            .unwrap_err();
        let (status, body) = status_and_body(preview_response(Err(failed), true)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, r#"{"error":"preview rendering failed"}"#);
    }

    /// Both policies refuse scripts, plugins, frames, form targets and base
    /// URLs, sandbox without a token, and differ only in what remote mode
    /// widens: images, stylesheets and fonts over http(s).
    #[test]
    fn the_preview_policies_differ_only_in_remote_loads() {
        for policy in [PREVIEW_CSP, PREVIEW_REMOTE_CSP] {
            let directives: Vec<&str> = policy.split("; ").collect();
            for directive in [
                "default-src 'none'",
                "script-src 'none'",
                "object-src 'none'",
                "frame-src 'none'",
                "base-uri 'none'",
                "form-action 'none'",
                "frame-ancestors 'self'",
                "sandbox",
            ] {
                assert!(
                    directives.contains(&directive),
                    "{policy} lacks {directive}"
                );
            }
            assert!(!policy.contains("allow-"), "{policy}");
        }
        let widened = PREVIEW_CSP
            .replace("img-src data:", "img-src data: https: http:")
            .replace(
                "style-src 'unsafe-inline';",
                "style-src 'unsafe-inline' https: http:; font-src https: http:;",
            );
        assert_eq!(widened, PREVIEW_REMOTE_CSP);
        assert!(CSP.ends_with("; frame-src 'self'"));
    }

    #[test]
    fn report_ui_escapes_every_provider_field_and_keeps_locators_inactive() {
        use crate::deliverability::{Finding, Location, Report, Severity};
        let attack = "<img src=\"https://attacker/\">&'";
        let report = Report {
            dns: Default::default(),
            arc: Default::default(),
            rule_set_id: attack.into(),
            findings: vec![
                Finding {
                    rule_id: attack.into(),
                    severity: Severity::Info,
                    location: Location::Header {
                        name: attack.into(),
                        occurrence: None,
                    },
                    fix_hint: attack.into(),
                    weight: 0,
                },
                Finding {
                    rule_id: "fixture".into(),
                    severity: Severity::Error,
                    location: Location::Link {
                        part_path: vec![0, 1],
                        byte_offset: 19,
                    },
                    fix_hint: "https://attacker/".into(),
                    weight: 0,
                },
            ],
            total: 0,
        };
        let html = report_section(Ok(&report));
        assert!(!html.contains("<img"));
        assert!(!html.contains("<a "));
        assert!(html.contains("&lt;img src=&quot;https://attacker/&quot;&gt;&amp;&#39;"));
        assert!(html.contains("byte_offset"));
        assert!(html.contains("https://attacker/"));
        assert!(
            report_section(Err(&crate::deliverability::AnalysisError::Unavailable))
                .contains("deliverability analysis unavailable")
        );
    }

    #[test]
    fn the_message_page_shows_the_arc_member_for_every_state() {
        use crate::deliverability::{ArcChain, ArcState, Report};
        let line = |arc: ArcChain| {
            let report = Report {
                rule_set_id: "r".into(),
                findings: vec![],
                dns: Default::default(),
                arc,
                total: 0,
            };
            report_section(Ok(&report))
        };
        let none = line(ArcChain::default());
        assert!(none.contains(r#"<div class="mono">arc · none · 0 sets</div>"#));
        let pass = line(ArcChain {
            state: ArcState::Pass,
            sets: 2,
            ..ArcChain::default()
        });
        assert!(pass.contains(r#"<div class="mono">arc · pass · 2 sets</div>"#));
        let fail = line(ArcChain {
            state: ArcState::Fail,
            sets: 2,
            failing_instance: Some(1),
            reason: Some("arc.signature_mismatch.v1".into()),
        });
        assert!(fail.contains(
            r#"<div class="mono">arc · fail · 2 sets · instance 1 · arc.signature_mismatch.v1</div>"#
        ));
        let declined = line(ArcChain {
            state: ArcState::NotEvaluated,
            sets: 1,
            failing_instance: None,
            reason: Some("<b>".into()),
        });
        assert!(declined.contains(r#"arc · not_evaluated · 1 sets · &lt;b&gt;</div>"#));
        assert!(!declined.contains("<b>"));
    }

    /// Every navigation arm (decision 0044 §3): no links on a lone page,
    /// next-only at the start, previous-only at the end (saturating), both
    /// in the middle, a default-page link for `limit=0`, and no wrapped next
    /// at the top of `usize`.
    #[test]
    fn the_pager_never_wraps_or_steps_by_zero() {
        assert_eq!(pager(100, 0, 3), "");
        assert_eq!(
            pager(2, 0, 3),
            r#"<div class="muted"><a href="/?limit=2&amp;offset=2">next</a></div>"#
        );
        assert_eq!(
            pager(2, 1, 3),
            r#"<div class="muted"><a href="/?limit=2&amp;offset=0">previous</a></div>"#
        );
        assert_eq!(
            pager(1, 1, 3),
            r#"<div class="muted"><a href="/?limit=1&amp;offset=0">previous</a> · <a href="/?limit=1&amp;offset=2">next</a></div>"#
        );
        assert_eq!(
            pager(0, 7, 3),
            r#"<div class="muted"><a href="/?offset=7">default page</a></div>"#
        );
        assert_eq!(
            pager(250, usize::MAX, usize::MAX),
            format!(
                r#"<div class="muted"><a href="/?limit=250&amp;offset={}">previous</a></div>"#,
                usize::MAX - 250
            )
        );
    }
}
