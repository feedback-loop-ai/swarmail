//! Minimal embedded web UI. The agent-native surfaces (REST/MCP) stay the
//! primary interface; this is for humans watching a run. The inbox and thread
//! views are live: the server ships a thin shell and the browser paints it
//! from the JSON API (`/api/v1/inboxes/{inbox}/threads[...]`), refreshed by
//! the SSE feed on every accepted mail. All mail-controlled text is rendered
//! with `textContent` in `static/ui.js` — never `innerHTML` — so nothing the
//! SMTP peer sent can become markup here.

use crate::api::AppState;
use axum::Router;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;

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

fn page(title: &str, body: String) -> Html<String> {
    let title = esc(title);
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>{title} · Swarmail</title>{STYLE}</head>
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

async fn index(State(state): State<AppState>) -> Html<String> {
    let mut rows = String::new();
    for (name, count) in state.store.inboxes() {
        let name = esc(&name);
        rows.push_str(&format!(
            r#"<div class="card"><a href="/ui/inbox/{name}"><b>{name}</b></a> <span class="pill">{count} emails</span></div>"#
        ));
    }
    if rows.is_empty() {
        rows = r#"<div class="card muted">No inboxes yet — point SMTP at this server and send something.</div>"#.into();
    }
    page("inboxes", format!(r#"<h2>inboxes</h2>{rows}"#))
}

/// The live inbox view: the thread list, repainted from the feed as mail
/// arrives (no manual refresh).
async fn inbox_view(Path(inbox): Path<String>) -> Html<String> {
    shell(
        &inbox,
        format!(r#"<div id="view" data-inbox="{}"></div>"#, esc(&inbox)),
    )
}

/// The live thread view: one conversation, oldest first, kept fresh too.
async fn thread_view(Path((inbox, key)): Path<(String, String)>) -> Html<String> {
    shell(
        "thread",
        format!(
            r#"<div id="view" data-inbox="{}" data-thread="{}"></div>"#,
            esc(&inbox),
            esc(&key)
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
    let subject = esc(email.subject.as_deref().unwrap_or("(no subject)"));
    page(
        &subject,
        format!(
            r#"<h2>{subject}</h2>
<div class="card"><b>From:</b> {from} &nbsp; <b>To:</b> {} &nbsp; <b>At:</b> {}</div>
<h2>codes</h2><div>{codes}</div><h2>links</h2>{links}<h2>html</h2>{html_body}<h2>text</h2>{text_block}"#,
            to.join(", "),
            esc(&email.received_at),
        ),
    )
}
