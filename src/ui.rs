//! Minimal embedded web UI. The agent-native surfaces (REST/MCP) stay the
//! primary interface; this is for humans watching a run. The inbox and thread
//! views are live: the server ships a thin shell and the browser paints it
//! from the JSON API (`/api/v1/inboxes/{inbox}/threads[...]`), refreshed by
//! the SSE feed on every accepted mail. All mail-controlled text is rendered
//! with `textContent` in `static/ui.js` — never `innerHTML` — so nothing the
//! SMTP peer sent can become markup here.

use crate::api::{ApiError, AppState, InboxPageQuery, bad_request, inbox_page, parse_query};
use crate::model::validate_inbox;
use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// The browser half of the live views, embedded at compile time. It contains
/// no mail data — only wiring — and no `</script>` sequence, so it can be
/// inlined into the shell.
const SCRIPT: &str = include_str!("static/ui.js");

const STYLE: &str = r#"<style>
body{font-family:ui-sans-serif,system-ui,sans-serif;margin:0;background:#0d1117;color:#e6edf3}
a{color:#58a6ff;text-decoration:none}a:hover{text-decoration:underline}
.wrap{max-width:960px;margin:0 auto;padding:24px}
h1{font-size:20px;margin:0 0 4px}h2{font-size:14px;color:#8b949e;margin:24px 0 8px;font-weight:600;text-transform:uppercase;letter-spacing:.05em}
.card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:12px 16px;margin:8px 0}
.mono{font-family:ui-monospace,monospace;font-size:12px}
.pill{display:inline-block;background:#1f6feb22;color:#58a6ff;border:1px solid #1f6feb55;border-radius:99px;padding:1px 10px;font-size:12px;margin-right:6px}
.muted{color:#8b949e;font-size:12px}
iframe{width:100%;min-height:320px;border:1px solid #30363d;border-radius:8px;background:#fff}
</style>"#;

/// HTML-escape untrusted text for element content and attribute values.
/// Every interpolation of mail-controlled data goes through this.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The one security policy for the shell (issue #56). Inline script and
/// style stay (the script is embedded, the style is a constant above);
/// `connect-src 'self'` keeps the SSE `EventSource` working; `default-src
/// 'none'` refuses every other load — and the mail-body preview is a
/// sandboxed `srcdoc` iframe that inherits this policy container, so a
/// hostile mail rendered in it cannot beacon the codes it holds to an
/// attacker URL. One string, one choke point: this template is the one
/// every UI route flows through.
const CSP: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:";

fn page(title: &str, body: String) -> Html<String> {
    let title = esc(title);
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="{CSP}"><title>{title} · Swarmail</title>{STYLE}</head>
<body><div class="wrap"><h1>📮 Swarmail</h1><div class="muted">the fastest AI-native mail mock · <a href="/">inboxes</a> · <a href="/llms.txt">llms.txt</a> · <a href="/openapi.json">openapi</a></div>{body}</div></body></html>"#
    ))
}

/// A live shell: no mail is rendered server-side. The `data-*` attributes are
/// the only state the script needs; everything else comes from the JSON API.
fn shell(title: &str, view: String) -> Html<String> {
    page(
        title,
        format!(
            r#"{view}<noscript><div class="card muted">This view is live and needs JavaScript; the JSON API works without it.</div></noscript><script>{SCRIPT}</script>"#
        ),
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/ui/inbox/{inbox}", get(inbox_view))
        .route("/ui/inbox/{inbox}/thread/{key}", get(thread_view))
        .route("/ui/message/{id}", get(message_view))
}

/// The inbox index, paged like `/api/v1/inboxes` (decision 0044 §3, issue
/// #91): the same closed `limit`/`offset` query, the same default and cap.
/// An empty store says so; an empty page beyond the end says only that.
async fn index(State(state): State<AppState>, raw: RawQuery) -> Result<Html<String>, ApiError> {
    let q: InboxPageQuery = parse_query(raw.0.as_deref().unwrap_or(""), "inboxes")?;
    let (limit, offset) = q.page().map_err(bad_request)?;
    let total = state.store.inboxes().len();
    let mut rows = String::new();
    for (name, count) in inbox_page(&state.store, limit, offset) {
        // The href is percent-encoded (a `#` or `?` in an inbox name would
        // otherwise be read as a fragment or query, and `a#b` would link to
        // `a`); the visible text stays escaped HTML.
        let href = utf8_percent_encode(&name, NON_ALPHANUMERIC);
        let label = esc(&name);
        rows.push_str(&format!(
            r#"<div class="card"><a href="/ui/inbox/{href}"><b>{label}</b></a> <span class="pill">{count} emails</span></div>"#
        ));
    }
    if total == 0 {
        rows = r#"<div class="card muted">No inboxes yet — point SMTP at this server and send something.</div>"#.into();
    } else if rows.is_empty() {
        rows = r#"<div class="card muted">No inboxes on this page.</div>"#.into();
    }
    let pager = pager(limit, offset, total);
    Ok(page("inboxes", format!(r#"<h2>inboxes</h2>{rows}{pager}"#)))
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

/// The live inbox view: the thread list, repainted from the feed as mail
/// arrives (no manual refresh). A blank inbox refuses (decision 0044 §3).
async fn inbox_view(Path(inbox): Path<String>) -> Result<Html<String>, ApiError> {
    validate_inbox(&inbox).map_err(bad_request)?;
    Ok(shell(
        &inbox,
        format!(r#"<div id="view" data-inbox="{}"></div>"#, esc(&inbox)),
    ))
}

/// The live thread view: one conversation, oldest first, kept fresh too.
async fn thread_view(Path((inbox, key)): Path<(String, String)>) -> Result<Html<String>, ApiError> {
    validate_inbox(&inbox).map_err(bad_request)?;
    Ok(shell(
        "thread",
        format!(
            r#"<div id="view" data-inbox="{}" data-thread="{}"></div>"#,
            esc(&inbox),
            esc(&key)
        ),
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
    // The html preview renders in a sandboxed iframe (no scripts), its
    // markup escaped into the srcdoc attribute.
    let html_body = email
        .html
        .as_deref()
        .map(|h| format!(r#"<iframe sandbox srcdoc="{}"></iframe>"#, esc(h)))
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
<h2>codes</h2><div>{codes}</div><h2>links</h2>{links}<h2>html</h2>{html_body}<h2>text</h2>{text_block}"#,
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
