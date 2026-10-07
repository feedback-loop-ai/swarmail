//! Minimal embedded web UI — server-rendered, zero frontend dependencies.
//! The agent-native surfaces (REST/MCP) stay the primary interface; this is
//! for humans watching a run.

use crate::api::AppState;
use axum::Router;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;

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

fn page(title: &str, body: String) -> Html<String> {
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>{title} · Swarmail</title>{STYLE}</head>
<body><div class="wrap"><h1>📮 Swarmail</h1><div class="muted">the fastest AI-native mail mock · <a href="/">inboxes</a> · <a href="/llms.txt">llms.txt</a> · <a href="/openapi.json">openapi</a></div>{body}</div></body></html>"#
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/ui/inbox/{inbox}", get(inbox_view))
        .route("/ui/message/{id}", get(message_view))
}

async fn index(State(state): State<AppState>) -> Html<String> {
    let mut rows = String::new();
    for (name, count) in state.store.inboxes() {
        rows.push_str(&format!(
            r#"<div class="card"><a href="/ui/inbox/{name}"><b>{name}</b></a> <span class="pill">{count} emails</span></div>"#
        ));
    }
    if rows.is_empty() {
        rows = r#"<div class="card muted">No inboxes yet — point SMTP at this server and send something.</div>"#.into();
    }
    page("inboxes", format!(r#"<h2>inboxes</h2>{rows}"#))
}

async fn inbox_view(State(state): State<AppState>, Path(inbox): Path<String>) -> Html<String> {
    let emails = state.store.list(&inbox, &Default::default());
    let mut rows = String::new();
    for e in emails.iter().take(200) {
        let to: Vec<String> = e.to.iter().map(|a| a.address.clone()).collect();
        rows.push_str(&format!(
            r#"<div class="card"><a href="/ui/message/{}"><b>{}</b></a> <span class="muted">→ {} · {} · {} B</span></div>"#,
            e.id,
            e.subject.as_deref().unwrap_or("(no subject)"),
            to.join(", "),
            e.received_at,
            e.size
        ));
    }
    if rows.is_empty() {
        rows = format!(r#"<div class="card muted">Inbox "{inbox}" is empty.</div>"#);
    }
    page(
        &inbox,
        format!(
            r#"<h2>inbox: {inbox} <span class="pill">{}</span></h2>{rows}"#,
            emails.len()
        ),
    )
}

async fn message_view(State(state): State<AppState>, Path(id): Path<String>) -> Html<String> {
    let Some(email) = state.store.get(&id) else {
        return page(
            "not found",
            r#"<div class="card">Message not found (pruned or cleared).</div>"#.into(),
        );
    };
    let html_body = email
        .html
        .as_deref()
        .map(|h| format!(r#"<iframe srcdoc="{}"></iframe>"#, h.replace('"', "&quot;")))
        .unwrap_or_default();
    let text_block = email
        .text
        .as_deref()
        .map(|t| format!(r#"<div class="card mono">{}</div>"#, t.replace('<', "&lt;")))
        .unwrap_or_default();
    let links = email
        .links
        .iter()
        .map(|l| format!(r#"<div class="card mono"><a href="{l}">{l}</a></div>"#))
        .collect::<String>();
    let codes = email
        .codes
        .iter()
        .map(|c| format!(r#"<span class="pill mono">{c}</span>"#))
        .collect::<String>();

    let from = email
        .from
        .as_ref()
        .map(|a| a.address.clone())
        .unwrap_or_default();
    let to: Vec<String> = email.to.iter().map(|a| a.address.clone()).collect();
    page(
        email.subject.as_deref().unwrap_or("(no subject)"),
        format!(
            r#"<h2>{}</h2>
<div class="card"><b>From:</b> {} &nbsp; <b>To:</b> {} &nbsp; <b>At:</b> {}</div>
<h2>codes</h2><div>{codes}</div><h2>links</h2>{links}<h2>html</h2>{html_body}<h2>text</h2>{text_block}"#,
            email.subject.as_deref().unwrap_or("(no subject)"),
            from,
            to.join(", "),
            email.received_at,
        ),
    )
}
