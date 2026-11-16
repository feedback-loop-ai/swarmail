//! Embedded web UI. The agent-native surfaces (REST/MCP) stay the primary
//! interface; this is for humans watching a run. `/`, the inbox view and the
//! thread view are one live **run console** (issue #355): a status bar over
//! three panes (inboxes, stream, reader). The server ships a thin shell and
//! the first paint of the inbox rail; the browser paints the rest from the
//! JSON API (`/api/v1/inboxes/{inbox}/threads[...]`, the per-message
//! projection and deliverability report) and the SSE feed. All mail-controlled
//! text is rendered with `textContent` in `static/ui.js` — never `innerHTML` —
//! so nothing the SMTP peer sent can become markup here.

use crate::api::{
    ApiError, AppState, DEFAULT_LIST_LIMIT, InboxPageQuery, bad_request, inbox_page, parse_query,
};
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

/// The stylesheet of every UI page, embedded at compile time: system font
/// stacks, no `url(`, no `@import`, nothing that loads from outside.
const CSS: &str = include_str!("static/ui.css");

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
/// Other resource loads are refused by `default-src 'none'`. The sandboxed
/// srcdoc mail preview inherits the policy, but CSP does not govern clicked
/// link navigation: this is not a guarantee against beaconing mail contents.
/// Every UI route uses this one policy string.
const CSP: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:";

fn page(title: &str, body: String) -> Html<String> {
    let title = esc(title);
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="{CSP}"><title>{title} · Swarmail</title><style>{CSS}</style></head>
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
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="{CSP}"><title>{title} · Swarmail</title><style>{CSS}</style></head>
<body class="app"><header id="status" class="bar"><div class="brand">{LOGO}Swarmail</div><div class="muted">Built for a program waiting on mail. · <a href="/">inboxes</a> · <a href="/llms.txt">llms.txt</a> · <a href="/openapi.json">openapi</a></div><div class="endpoints" aria-label="Listeners"><span class="ep" id="listener-smtp">SMTP <b>unknown</b></span><span class="ep" id="listener-pop3">POP3 <b>unknown</b></span><span class="ep" id="listener-http">HTTP <b>unknown</b></span></div><div class="stats tnum"><span><b id="count-accepted">–</b> accepted</span><span class="dropped"><b id="count-dropped">–</b> dropped</span></div><span class="spacer"></span><span class="chip chaos-unknown" id="chaos">Chaos unknown</span><span class="live" id="live"><span class="pulse"></span><span>{live}</span></span></header>
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

fn report_section(
    report: Result<&crate::deliverability::Report, &crate::deliverability::AnalysisError>,
) -> String {
    let body = match report {
        Err(error) => esc(error.message()),
        Ok(report) => {
            let mut body = format!("<div class=\"mono\">{}</div>", esc(&report.rule_set_id));
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
        "<h2>deliverability</h2><section>{body}<div class=\"muted\">Demo and bulk-sender checks only. Empty findings do not imply deliverability.</div></section>"
    )
}

fn render_message(
    email: &crate::model::Email,
    report: Result<&crate::deliverability::Report, &crate::deliverability::AnalysisError>,
) -> Response {
    let report_section = report_section(report);
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

    #[test]
    fn report_ui_escapes_every_provider_field_and_keeps_locators_inactive() {
        use crate::deliverability::{Finding, Location, Report, Severity};
        let attack = "<img src=\"https://attacker/\">&'";
        let report = Report {
            dns: Default::default(),
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
