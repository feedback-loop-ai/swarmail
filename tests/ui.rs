//! The UI: live inbox/thread views and every server-rendered page, plus the
//! JSON endpoints and SSE frames they are wired to. Everything below speaks
//! real protocol — raw SMTP for the mail, raw HTTP for the pages and feeds.

mod common;

use common::{SmtpConn, SseReader, http_get_text, http_json, smtp_send, start};

/// Seed one fixture mail into `inbox` (the default extractor path).
async fn seed(addr: std::net::SocketAddr, inbox: &str, body: &str) {
    let (st, _) = common::http_json(
        addr,
        "POST",
        &format!("/api/v1/inboxes/{inbox}/seed"),
        Some(body),
    )
    .await;
    assert_eq!(st, 200);
}

/// Send a mail over real SMTP with full control over its headers — the seed
/// endpoint cannot set Message-ID/References — AUTH PLAIN naming the inbox.
async fn send_headers(
    addr: std::net::SocketAddr,
    inbox: &str,
    headers: &[(&str, &str)],
    subject: &str,
) {
    let mut conn = SmtpConn::connect(addr).await;
    conn.send("EHLO swarmail-thread-test").await;
    conn.reply_multi().await;
    let plain = format!("\u{0}{inbox}\u{0}whatever");
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    let reply = conn.reply().await;
    assert!(reply.starts_with("235"), "AUTH refused: {reply}");
    conn.send("MAIL FROM:<sender@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    conn.send("RCPT TO:<rcpt@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    let mut data = format!("From: sender@x.io\r\nTo: rcpt@x.io\r\nSubject: {subject}\r\n");
    for (name, value) in headers {
        data.push_str(&format!("{name}: {value}\r\n"));
    }
    data.push_str("\r\nbody\r\n");
    let reply = conn.data(&data).await;
    assert!(reply.starts_with("250"), "DATA refused: {reply}");
}

/// The inner source of every `<script>…</script>` element in a served page,
/// in document order. The markup is machine-generated and every piece of
/// mail-controlled text is escaped before it is interpolated, so the scan can
/// be strict: an open tag ends at its `>`, the element at the first
/// `</script>` — a sequence the shipped script cannot contain, which is what
/// makes inlining it safe at all.
fn script_elements(page: &str) -> Vec<&str> {
    let mut scripts = Vec::new();
    let mut rest = page;
    while let Some(open) = rest.find("<script") {
        let tail = &rest[open..];
        let Some(gt) = tail.find('>') else { break };
        let body = &tail[gt + 1..];
        let Some(end) = body.find("</script>") else {
            break;
        };
        scripts.push(&body[..end]);
        rest = &body[end + "</script>".len()..];
    }
    scripts
}

/// The one `<script>` element of a live view, asserted to be executable
/// wiring and not inert text: it binds the view element, reads the
/// server-rendered state out of it, builds real API paths, goes live through
/// an `EventSource`, and paints mail through `textContent` only. This is the
/// assertion the reviewed defect defeats: JS that is merely *contained* in
/// the page never executes, so the script must be a parsed element.
fn the_executable_script(page: &str) -> &str {
    let scripts = script_elements(page);
    assert_eq!(scripts.len(), 1, "exactly one script element: {page}");
    let script = scripts[0];
    assert!(
        script.contains("document.getElementById('view')"),
        "the script binds the view element: {script}"
    );
    assert!(
        script.contains("dataset.inbox"),
        "the script reads the server-rendered state: {script}"
    );
    assert!(
        script.contains("'/api/v1/inboxes/'"),
        "the script builds API paths: {script}"
    );
    assert!(
        script.contains("new EventSource("),
        "the page goes live through an EventSource: {script}"
    );
    assert!(
        script.contains("textContent"),
        "mail text is painted textContent-only: {script}"
    );
    assert!(
        !script.contains(".innerHTML"),
        "no markup from mail: {script}"
    );
    script
}

/// The value of `name="…"` in the served markup — the state the script
/// receives besides the API paths it builds. Attribute values are escaped,
/// so the first `"` after the needle always closes it.
fn attr(page: &str, name: &str) -> String {
    let needle = format!(r#"{name}=""#);
    let at = page
        .find(&needle)
        .unwrap_or_else(|| panic!("{name} missing from: {page}"));
    let rest = &page[at + needle.len()..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("{name} unterminated in: {page}"));
    rest[..end].to_string()
}

#[tokio::test]
async fn index_lists_inboxes() {
    let srv = start().await;
    seed(
        srv.http_addr,
        "ui-inbox",
        r#"{"to": "u@x.io", "subject": "ui test"}"#,
    )
    .await;

    let (st, page) = http_get_text(srv.http_addr, "/").await;
    assert_eq!(st, 200);
    assert!(page.contains("Swarmail"));
    assert!(page.contains("ui-inbox"), "{page}");
}

#[tokio::test]
async fn index_shows_the_empty_state_before_any_mail() {
    let srv = start().await;
    let (st, page) = http_get_text(srv.http_addr, "/").await;
    assert_eq!(st, 200);
    assert!(page.contains("No inboxes yet"), "{page}");
}

#[tokio::test]
async fn inbox_view_is_a_live_shell_wired_to_the_api_and_feed() {
    let srv = start().await;
    // The first mail lands over real SMTP before the page is even fetched:
    // both the script's initial fetch and the feed snapshot must carry it.
    smtp_send(
        srv.smtp_addr,
        Some("iv"),
        "s@x.io",
        "r@x.io",
        "inbox view",
        "body",
    )
    .await
    .unwrap();

    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/iv").await;
    assert_eq!(st, 200);
    // The shell ships wiring, not mail: the subject is the API's job now.
    assert!(page.contains(r#"data-inbox="iv""#), "{page}");
    assert!(
        !page.contains("inbox view"),
        "no server-rendered mail: {page}"
    );
    let script = the_executable_script(&page);

    // The feed the script opens — `api('feed')` resolved against the view's
    // data-inbox — exists and streams: the snapshot carries the mail
    // accepted above, and a further insert is pushed without any poll.
    assert!(
        script.contains("api('feed')"),
        "the script opens the feed: {script}"
    );
    let feed_path = format!("/api/v1/inboxes/{}/feed", attr(&page, "data-inbox"));
    let mut feed = SseReader::open(srv.http_addr, &feed_path).await;
    let (event, data) = feed.next_frame().await;
    assert_eq!(
        event, "threads",
        "the wire event the script listens for: {data}"
    );
    let snapshot: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(
        snapshot[0]["subject"], "inbox view",
        "the snapshot is the thread view the page paints: {data}"
    );

    smtp_send(
        srv.smtp_addr,
        Some("iv"),
        "s@x.io",
        "r@x.io",
        "pushed live",
        "body",
    )
    .await
    .unwrap();
    let (event, data) = feed.next_frame().await;
    assert_eq!(event, "threads");
    let pushed: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(
        pushed[0]["subject"], "pushed live",
        "the push carries the mail accepted over SMTP: {data}"
    );

    // The grouping the page renders is exactly the threads API's answer:
    // one conversation per subject, newest thread first.
    let (st, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/iv/threads", None).await;
    assert_eq!(st, 200);
    let threads = threads.as_array().unwrap();
    assert_eq!(
        threads.len(),
        2,
        "two mails, two conversations: {threads:?}"
    );
    assert_eq!(threads[0]["subject"], "pushed live");
    assert_eq!(threads[0]["count"], 1);
    assert_eq!(threads[1]["subject"], "inbox view");

    // The empty inbox ships the same wiring; its empty state is painted live.
    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/empty-inbox").await;
    assert_eq!(st, 200);
    assert!(page.contains(r#"data-inbox="empty-inbox""#), "{page}");
    the_executable_script(&page);
    assert!(
        !page.contains("is empty"),
        "the empty state is a live paint: {page}"
    );
}

#[tokio::test]
async fn thread_view_is_a_live_shell_for_one_conversation() {
    let srv = start().await;
    // A real conversation over SMTP, so the page has a thread to render.
    send_headers(
        srv.smtp_addr,
        "tv",
        &[("Message-ID", "<root@x.io>")],
        "Kickoff",
    )
    .await;
    send_headers(
        srv.smtp_addr,
        "tv",
        &[("Message-ID", "<re@x.io>"), ("In-Reply-To", "<root@x.io>")],
        "Re: Kickoff",
    )
    .await;
    let (_, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/tv/threads", None).await;
    let key = threads[0]["key"].as_str().unwrap().to_string();

    let (st, page) = http_get_text(srv.http_addr, &format!("/ui/inbox/tv/thread/{key}")).await;
    assert_eq!(st, 200);
    assert!(page.contains(r#"data-inbox="tv""#), "{page}");
    assert!(page.contains(&format!(r#"data-thread="{key}""#)), "{page}");
    // The same executable-wiring contract as the inbox view — and the fetch
    // it issues resolves from the data-thread attribute above.
    let script = the_executable_script(&page);
    assert!(
        script.contains("'threads/' + encodeURIComponent(threadKey)"),
        "the script fetches the conversation endpoint: {script}"
    );

    // The conversation the page paints is exactly what that endpoint
    // returns: one thread, oldest first.
    let thread_path = format!(
        "/api/v1/inboxes/{}/threads/{}",
        attr(&page, "data-inbox"),
        attr(&page, "data-thread")
    );
    let (st, thread) = http_json(srv.http_addr, "GET", &thread_path, None).await;
    assert_eq!(st, 200);
    let emails = thread["emails"].as_array().unwrap();
    assert_eq!(emails.len(), 2, "the page renders the API's grouping");
    assert_eq!(emails[0]["subject"], "Kickoff");
    assert_eq!(emails[1]["subject"], "Re: Kickoff");

    // And the conversation stays live: the feed pushes the wire event the
    // script listens for, carrying the thread the page filters for — its own
    // data-thread key.
    let feed_path = format!("/api/v1/inboxes/{}/feed", attr(&page, "data-inbox"));
    let mut feed = SseReader::open(srv.http_addr, &feed_path).await;
    assert_eq!(feed.next_frame().await.0, "threads", "snapshot first");
    send_headers(
        srv.smtp_addr,
        "tv",
        &[
            ("Message-ID", "<third@x.io>"),
            ("In-Reply-To", "<root@x.io>"),
        ],
        "Re: Kickoff again",
    )
    .await;
    let (event, data) = feed.next_frame().await;
    assert_eq!(event, "threads");
    let pushed: serde_json::Value = serde_json::from_str(&data).unwrap();
    let live = pushed
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["key"] == key.as_str())
        .unwrap_or_else(|| panic!("the pushed view lost the page's thread: {data}"));
    assert_eq!(live["count"], 3, "the page's thread grew: {data}");
}

#[tokio::test]
async fn thread_endpoints_group_a_references_chain_over_real_smtp() {
    let srv = start().await;
    send_headers(
        srv.smtp_addr,
        "chain",
        &[("Message-ID", "<root@x.io>")],
        "Kickoff",
    )
    .await;
    send_headers(
        srv.smtp_addr,
        "chain",
        &[
            ("Message-ID", "<re@x.io>"),
            ("In-Reply-To", "<root@x.io>"),
            ("References", "<root@x.io> <missing@x.io>"),
        ],
        "Re: Kickoff",
    )
    .await;

    let (st, threads) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/chain/threads", None).await;
    assert_eq!(st, 200);
    assert_eq!(threads.as_array().unwrap().len(), 1, "one chain: {threads}");
    assert_eq!(threads[0]["count"], 2);
    let key = threads[0]["key"].as_str().unwrap();
    assert_eq!(key.len(), 16, "the key is a short URL-safe token");

    let (st, thread) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/chain/threads/{key}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    let emails = thread["emails"].as_array().unwrap();
    assert_eq!(emails.len(), 2);
    assert_eq!(
        emails[0]["message_id"], "root@x.io",
        "conversation is oldest first"
    );
    assert_eq!(emails[1]["message_id"], "re@x.io");
    assert_eq!(
        emails[1]["references"][1], "missing@x.io",
        "ids keep header order"
    );
    assert_eq!(emails[1]["in_reply_to"], "root@x.io");

    // The UI link target of a thread card is exactly this route.
    let (st, body) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/chain/threads/none",
        None,
    )
    .await;
    assert_eq!(st, 404);
    assert_eq!(body["error"], "thread not found");
}

#[tokio::test]
async fn thread_endpoints_fall_back_to_the_normalized_subject() {
    let srv = start().await;
    // No ids at all on either mail — the subject is the only thread hook.
    send_headers(srv.smtp_addr, "fallback", &[], "Re: Fwd: STATUS").await;
    send_headers(srv.smtp_addr, "fallback", &[], "status").await;
    send_headers(srv.smtp_addr, "fallback", &[], "unrelated").await;

    let (st, threads) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/fallback/threads",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let status = threads
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["count"] == 2)
        .unwrap_or_else(|| panic!("subject fallback failed: {threads}"));
    assert_eq!(
        status["subject"], "Re: Fwd: STATUS",
        "the display subject keeps its case"
    );
    assert_eq!(threads.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn malformed_thread_headers_degrade_without_losing_mail() {
    let srv = start().await;
    // An empty Message-ID and a reference with a stray space inside: mail-parser
    // salvages what it can ("<bogus junk>" is one id, "bogus junk") and drops
    // the rest — none of it may fail the ingest or the view.
    send_headers(
        srv.smtp_addr,
        "bent",
        &[
            ("Message-ID", "<root@x.io>"),
            ("References", "<bogus junk>"),
        ],
        "Bent",
    )
    .await;
    send_headers(srv.smtp_addr, "bent", &[], "bent").await;

    let (st, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/bent/threads", None).await;
    assert_eq!(st, 200, "a malformed header must not 500: {threads}");
    assert_eq!(
        threads[0]["count"], 2,
        "the subject still hooks them together"
    );
    let email = &threads[0]["emails"][0];
    assert_eq!(
        email["references"],
        serde_json::json!(["bogus", "junk"]),
        "{email}"
    );
    assert_eq!(email["message_id"], "root@x.io");

    // The empty Message-ID stays absent rather than becoming a bogus key.
    let (_, list) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/bent/messages", None).await;
    let bent = list["emails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["subject"] == "bent")
        .unwrap();
    assert_eq!(bent["message_id"], serde_json::Value::Null);
}

#[tokio::test]
async fn the_feed_pushes_the_new_thread_without_a_manual_refresh() {
    let srv = start().await;
    let mut feed = SseReader::open(srv.http_addr, "/api/v1/inboxes/live/feed").await;
    // First frame: the snapshot of the (empty) view.
    let (event, data) = feed.next_frame().await;
    assert_eq!(
        event, "threads",
        "the wire event name the browser listens for: {data}"
    );
    assert_eq!(data, "[]");

    // Mail accepted over real SMTP must be pushed, not polled for.
    smtp_send(
        srv.smtp_addr,
        Some("live"),
        "s@x.io",
        "r@x.io",
        "Hello live",
        "body",
    )
    .await
    .unwrap();
    let (event, data) = feed.next_frame().await;
    assert_eq!(event, "threads");
    let view: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(
        view[0]["subject"], "Hello live",
        "the push carries the thread view: {data}"
    );
    assert_eq!(view[0]["count"], 1);
}

#[tokio::test]
async fn message_view_renders_text_html_and_missing() {
    let srv = start().await;
    // Text-only mail.
    seed(
        srv.http_addr,
        "mv",
        r#"{"to": "u@x.io", "subject": "text mail", "text": "see https://x.io/a"}"#,
    )
    .await;
    // HTML-only mail (no text).
    seed(
        srv.http_addr,
        "mv",
        r#"{"to": "u@x.io", "subject": "html mail", "html": "<b>rich</b>"}"#,
    )
    .await;

    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/mv/messages", None).await;
    let id_of = |subject: &str| {
        list["emails"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["subject"] == subject)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    };

    let (st, page) = http_get_text(
        srv.http_addr,
        &format!("/ui/message/{}", id_of("text mail")),
    )
    .await;
    assert_eq!(st, 200);
    assert!(page.contains("text mail"), "{page}");
    assert!(page.contains("https://x.io/a"));

    let (st, page) = http_get_text(
        srv.http_addr,
        &format!("/ui/message/{}", id_of("html mail")),
    )
    .await;
    assert_eq!(st, 200);
    assert!(page.contains("html mail"), "{page}");
    assert!(
        page.contains("&lt;b&gt;rich&lt;/b&gt;"),
        "html preview is escaped: {page}"
    );
    assert!(page.contains("sandbox"), "preview iframe must be sandboxed");

    let (st, page) = http_get_text(srv.http_addr, "/ui/message/does-not-exist").await;
    assert_eq!(st, 200);
    assert!(page.contains("not found"), "{page}");
}

#[tokio::test]
async fn ui_is_escaped_not_injected() {
    let srv = start().await;
    seed(
        srv.http_addr,
        "xss",
        r#"{"to": "u@x.io", "subject": "<script>alert(1)</script>", "text": "<img onerror>"}"#,
    )
    .await;
    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/xss/messages", None).await;
    let id = list["emails"][0]["id"].as_str().unwrap();
    let (st, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    assert_eq!(st, 200);
    assert!(
        !page.contains("<script>alert"),
        "subject was injected: {page}"
    );
    // Stronger than the string check: the message view ships no script
    // element at all, so nothing on it is executable even in principle.
    assert!(
        script_elements(&page).is_empty(),
        "the message view must not ship a script: {page}"
    );
}
