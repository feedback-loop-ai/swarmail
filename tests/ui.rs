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
    assert!(page.contains("<div class=\"muted\">Built for a program waiting on mail. ·"));
    assert!(!page.to_ascii_lowercase().contains("fastest"));
    assert!(page.contains("ui-inbox"), "{page}");
}

#[tokio::test]
async fn index_shows_the_empty_state_before_any_mail() {
    let srv = start().await;
    let (st, page) = http_get_text(srv.http_addr, "/").await;
    assert_eq!(st, 200);
    assert!(page.contains("No inboxes yet"), "{page}");
}

/// The index is paged like `/api/v1/inboxes` (decision 0044 §3, issue
/// #91): `limit`/`offset` pick the window and the links keep the limit; a
/// page past the end says so without claiming the store is empty;
/// `limit=0` offers the default page; an over-cap or unknown key refuses.
/// Red before: every inbox on one page, and the query ignored.
#[tokio::test]
async fn the_index_pages_and_links_its_pages() {
    let srv = start().await;
    for inbox in ["pg-a", "pg-b", "pg-c"] {
        seed(srv.http_addr, inbox, r#"{"to": "u@x.io"}"#).await;
    }
    let (st, page) = http_get_text(srv.http_addr, "/?limit=1&offset=1").await;
    assert_eq!(st, 200);
    assert!(
        page.contains("pg-b") && !page.contains("pg-a") && !page.contains("pg-c"),
        "{page}"
    );
    assert!(
        page.contains(r#"href="/?limit=1&amp;offset=0">previous"#),
        "{page}"
    );
    assert!(
        page.contains(r#"href="/?limit=1&amp;offset=2">next"#),
        "{page}"
    );

    let (_, page) = http_get_text(srv.http_addr, "/?offset=9").await;
    assert!(page.contains("No inboxes on this page."), "{page}");
    assert!(!page.contains("No inboxes yet"), "{page}");
    let (_, page) = http_get_text(srv.http_addr, "/?limit=0").await;
    assert!(page.contains(r#"href="/?offset=0">default page"#), "{page}");
    assert!(!page.contains(">next<"), "{page}");

    for query in ["limit=251", "lmit=1"] {
        let (st, body) = http_json(srv.http_addr, "GET", &format!("/?{query}"), None).await;
        assert_eq!(st, 400, "{query}: {body}");
    }
}

/// A decoded blank inbox segment refuses on both live views (decision 0044
/// §3) instead of painting a shell for an inbox that cannot exist.
#[tokio::test]
async fn the_live_views_refuse_a_blank_inbox() {
    let srv = start().await;
    for path in ["/ui/inbox/%20", "/ui/inbox/%09/thread/abc"] {
        let (st, body) = http_json(srv.http_addr, "GET", path, None).await;
        assert_eq!(st, 400, "{path}: {body}");
        assert_eq!(body["error"], "inbox must not be blank", "{path}");
    }
    // A control-bearing name refuses on both views with the shared
    // sentence (issue #128, decision 0046 §4).
    for path in ["/ui/inbox/a%00b", "/ui/inbox/a%7Fb/thread/abc"] {
        let (st, body) = http_json(srv.http_addr, "GET", path, None).await;
        assert_eq!(st, 400, "{path}: {body}");
        assert_eq!(
            body["error"], "inbox must not contain control characters",
            "{path}"
        );
    }
}

/// Issue #100 on the wire: orphan subjects carrying the localized markers
/// and whitespace variants coalesce into one subject thread each (decision
/// 0044 §9). Red before: every variant was its own thread.
#[tokio::test]
async fn localized_reply_markers_coalesce_orphan_threads() {
    let srv = start().await;
    for subject in [
        "Plan",
        "SV: Plan",
        "Antw: Plan",
        "WG: Plan",
        "Odp: Plan",
        "\u{41e}\u{442}\u{432}\u{435}\u{442}: Plan",
        "\u{56de}\u{590d}\u{ff1a}Plan",
        "Two  spaces",
        "Two\\tspaces",
    ] {
        let body = format!(r#"{{"to": "u@x.io", "subject": "{subject}"}}"#);
        seed(srv.http_addr, "thr-l10n", &body).await;
    }
    let (st, threads) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/thr-l10n/threads",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let mut counts: Vec<u64> = threads
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["count"].as_u64().unwrap())
        .collect();
    counts.sort_unstable();
    assert_eq!(counts, [2, 7], "{threads}");
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

/// The issue's exact scenario (#9): a reply whose message-id sorts before
/// the root's used to flip the lexicographic representative and re-key the
/// conversation. The advertised URL must survive ordinary growth.
#[tokio::test]
async fn the_thread_url_survives_a_reply_with_a_smaller_message_id() {
    let srv = start().await;
    send_headers(
        srv.smtp_addr,
        "keys",
        &[("Message-ID", "<z@x.io>")],
        "Stable",
    )
    .await;
    let (_, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/keys/threads", None).await;
    let advertised = threads[0]["key"].as_str().unwrap().to_string();

    send_headers(
        srv.smtp_addr,
        "keys",
        &[("Message-ID", "<a@x.io>"), ("In-Reply-To", "<z@x.io>")],
        "Re: Stable",
    )
    .await;

    let (st, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/keys/threads", None).await;
    assert_eq!(st, 200);
    assert_eq!(threads.as_array().unwrap().len(), 1, "{threads}");
    assert_eq!(
        threads[0]["key"].as_str().unwrap(),
        advertised,
        "the thread URL the first request handed out must not move"
    );
    assert_eq!(threads[0]["count"], 2);

    // The view the UI addresses by that key still resolves to the grown
    // conversation.
    let (st, thread) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/inboxes/keys/threads/{advertised}"),
        None,
    )
    .await;
    assert_eq!(st, 200);
    let emails = thread["emails"].as_array().unwrap();
    assert_eq!(emails[0]["message_id"], "z@x.io", "oldest first");
    assert_eq!(emails[1]["message_id"], "a@x.io");
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
    // The reply cites the published parent: the chain links them. The
    // malformed References stay on the parent alone — subject affinity is
    // orphan-only, so the salvaged junk ids never authorize a subject merge
    // (spec R6, decision 0018).
    send_headers(
        srv.smtp_addr,
        "bent",
        &[("In-Reply-To", "<root@x.io>")],
        "Bent",
    )
    .await;

    let (st, threads) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/bent/threads", None).await;
    assert_eq!(st, 200, "a malformed header must not 500: {threads}");
    assert_eq!(
        threads[0]["count"], 2,
        "the chain still hooks them together"
    );
    let email = &threads[0]["emails"][0];
    assert_eq!(
        email["references"],
        serde_json::json!(["bogus", "junk"]),
        "{email}"
    );
    assert_eq!(email["message_id"], "root@x.io");

    // The reply's empty Message-ID stays absent rather than becoming a
    // bogus key — In-Reply-To alone never invents one.
    let (_, list) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/bent/messages", None).await;
    let bent = list["emails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["in_reply_to"] == "root@x.io")
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

    // A missing message is a status 404 (issue #56) — never a 200 card a
    // client would treat as a rendered message.
    let (st, page) = http_get_text(srv.http_addr, "/ui/message/does-not-exist").await;
    assert_eq!(st, 404, "a missing message must 404, not render a card");
    assert!(page.contains("not found"), "{page}");
}

/// A title containing `&` is escaped exactly once — in the `<title>` tab and
/// the `<h2>` alike (issue #56). A double escape (`&amp;amp;`) would show the
/// entity itself instead of the ampersand.
#[tokio::test]
async fn an_ampersand_title_is_escaped_exactly_once() {
    let srv = start().await;
    seed(
        srv.http_addr,
        "esc",
        r#"{"to": "u@x.io", "subject": "A & B", "text": "body"}"#,
    )
    .await;
    let (_, list) =
        common::http_json(srv.http_addr, "GET", "/api/v1/inboxes/esc/messages", None).await;
    let id = list["emails"][0]["id"].as_str().unwrap();
    let (st, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    assert_eq!(st, 200);
    assert!(
        page.contains("<title>A &amp; B · Swarmail</title>"),
        "{page}"
    );
    assert!(page.contains("<h2>A &amp; B</h2>"), "{page}");
    assert!(
        !page.contains("&amp;amp;"),
        "the ampersand must not be escaped twice: {page}"
    );
}

/// A non-URL-safe inbox name is percent-encoded in the href but shown
/// unharmed as the link's visible text (issue #56).
#[tokio::test]
async fn a_hash_inbox_name_is_url_encoded_in_the_href() {
    let srv = start().await;
    // Seed into `a#b` — the request path itself is percent-encoded.
    let (st, _) = common::http_json(
        srv.http_addr,
        "POST",
        "/api/v1/inboxes/a%23b/seed",
        Some(r#"{"to": "u@x.io", "subject": "hashy", "text": "body"}"#),
    )
    .await;
    assert_eq!(st, 200);
    let (st, index) = http_get_text(srv.http_addr, "/").await;
    assert_eq!(st, 200);
    assert!(
        index.contains(r#"href="/ui/inbox/a%23b""#),
        "the href is URL-encoded: {index}"
    );
    assert!(
        index.contains(">a#b<"),
        "the visible text is unharmed: {index}"
    );
    // The encoded href serves the same inbox back (round-trip).
    let (st, shell) = http_get_text(srv.http_addr, "/ui/inbox/a%23b").await;
    assert_eq!(st, 200);
    assert!(shell.contains(r#"data-inbox="a#b""#), "{shell}");
}

/// The served shell carries a CSP (issue #56): inline script/style stay
/// working (the page inlines both), same-origin SSE keeps connecting, and
/// remote loads — including anything a mail body might name — are blocked.
#[tokio::test]
async fn the_shell_ships_a_content_security_policy() {
    let srv = start().await;
    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/anywhere").await;
    assert_eq!(st, 200);
    let needle = r#"<meta http-equiv="Content-Security-Policy" content=""#;
    let at = page
        .find(needle)
        .unwrap_or_else(|| panic!("no CSP meta in the shell: {page}"));
    let rest = &page[at + needle.len()..];
    let end = rest.find('"').expect("CSP meta unterminated");
    let csp = &rest[..end];
    for directive in [
        "default-src 'none'",
        "script-src 'unsafe-inline'",
        "style-src 'unsafe-inline'",
        "connect-src 'self'",
        "img-src 'self' data:",
    ] {
        assert!(
            csp.contains(directive),
            "CSP must carry {directive:?}: {csp}"
        );
    }
    // And the live view stays live: the shell's own script is still there
    // under the policy.
    the_executable_script(&page);
}

/// A stale `load()` response cannot paint over a newer SSE frame (issue
/// #56): the served script guards every paint with a token that the SSE
/// handler invalidates. A browser-engine interleaving test is not demanded —
/// the guard is asserted in the served asset itself.
#[tokio::test]
async fn the_live_shell_guards_paints_with_a_token() {
    let srv = start().await;
    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/anywhere").await;
    assert_eq!(st, 200);
    let script = the_executable_script(&page);
    assert!(
        script.contains("paintSeq"),
        "the paint token must guard the paints: {script}"
    );
    assert!(
        script.contains("paintSeq++"),
        "the SSE handler must invalidate in-flight loads: {script}"
    );
    // The guard clause itself, not just its identifiers: without the
    // early return the token is counted but never consulted, and a stale
    // `load()` paints over newer mail again.
    assert!(
        script.contains("seen !== paintSeq"),
        "every paint must be gated by the token comparison: {script}"
    );
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

/// A message with no normalizable subject joins its References chain without
/// a subject node: the subject-union arm is skipped, the grouping stands.
#[tokio::test]
async fn thread_grouping_survives_a_missing_subject() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<chain@x.io>").await;
    c.reply().await;
    c.send("RCPT TO:<nosubject@x.io>").await;
    c.reply().await;
    assert!(
        c.data("References: <anchor@x.io>\r\nMessage-ID: <kid@x.io>\r\n\r\nno subject here")
            .await
            .starts_with("250")
    );

    let (st, threads) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/threads",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let threads = threads.as_array().unwrap();
    assert_eq!(threads.len(), 1, "the chain groups alone: {threads:?}");
    assert_eq!(threads[0]["count"], 1);
    assert!(
        threads[0]["subject"].is_null(),
        "no subject means no subject label: {threads:?}"
    );
}

/// The message view renders extracted verification codes as pills, and the
/// mailpit shim's plain fallback serves html-only mail as text/plain.
#[tokio::test]
async fn message_view_renders_code_pills_and_compat_plain_falls_back() {
    let srv = start().await;
    seed(
        srv.http_addr,
        "pills",
        r#"{"to": "u@x.io", "subject": "coded", "text": "Your code is 555666 thanks"}"#,
    )
    .await;
    // HTML-only mail: no text part at all.
    seed(
        srv.http_addr,
        "pills",
        r#"{"to": "u@x.io", "subject": "rich only", "html": "<b>rich body</b>"}"#,
    )
    .await;

    let (_, list) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/pills/messages", None).await;
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

    let (st, page) = http_get_text(srv.http_addr, &format!("/ui/message/{}", id_of("coded"))).await;
    assert_eq!(st, 200);
    assert!(
        page.contains(r#"<span class="pill mono">555666</span>"#),
        "the extracted code renders as a pill: {page}"
    );

    // The mailpit plain shim: html-only mail falls back to the html body.
    let (st, plain) = http_get_text(
        srv.http_addr,
        &format!("/api/v1/messages/{}/plain", id_of("rich only")),
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(
        plain.trim(),
        "rich body",
        "the plain shim serves the ingested text: {plain}"
    );
}

/// References/In-Reply-To headers ride the SMTP ingest path: the thread ids
/// are extracted at ingest (`header_ids`) and the thread view groups by them.
#[tokio::test]
async fn smtp_thread_headers_extract_ids_at_ingest() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<thread@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<threads@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(
        c.data(
            "Subject: the reply\r\n\
             Message-ID: <reply@x.io>\r\n\
             In-Reply-To: <root@x.io>\r\n\
             References: <ancestor@x.io> <root@x.io>\r\n\
             \r\n\
             x"
        )
        .await
        .starts_with("250")
    );

    let (_, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/default/messages",
        None,
    )
    .await;
    let mail = &list["emails"].as_array().unwrap()[0];
    // The parser normalizes the angle brackets away.
    assert_eq!(
        mail["in_reply_to"], "root@x.io",
        "In-Reply-To extracted at ingest: {mail}"
    );
    let refs: Vec<String> = mail["references"]
        .as_array()
        .expect("the references list")
        .iter()
        .map(|r| r.as_str().unwrap().to_string())
        .collect();
    assert_eq!(refs, vec!["ancestor@x.io", "root@x.io"], "{mail}");
}

// ---------- the run console (issue #355, proposed decision 0132) ----------
//
// No JavaScript engine runs in any gate, so the script is asserted on the
// bytes the server hands out; what only a browser can show is listed in
// `specs/web-ui-run-console/spec.md` under "Not gate-provable".

const POLICY: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:";

/// The three console routes for `inbox`, as `(path, served page)`. The
/// thread route needs a real key, so the inbox must hold a mail.
async fn console_pages(
    addr: std::net::SocketAddr,
    inbox: &str,
    key: &str,
) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    for path in [
        "/".to_string(),
        format!("/ui/inbox/{inbox}"),
        format!("/ui/inbox/{inbox}/thread/{key}"),
    ] {
        let (st, page) = http_get_text(addr, &path).await;
        assert_eq!(st, 200, "{path}");
        pages.push((path, page));
    }
    pages
}

/// Seed one mail and return the key of the thread it opened.
async fn seeded_thread_key(addr: std::net::SocketAddr, inbox: &str) -> String {
    seed(addr, inbox, r#"{"to": "u@x.io", "subject": "console"}"#).await;
    let path = format!("/api/v1/inboxes/{inbox}/threads");
    let (_, threads) = http_json(addr, "GET", &path, None).await;
    threads[0]["key"].as_str().unwrap().to_string()
}

/// The served script of a console page.
async fn served_script(addr: std::net::SocketAddr) -> String {
    let (_, page) = http_get_text(addr, "/ui/inbox/script").await;
    the_executable_script(&page).to_string()
}

/// The inner text of the page's one `<style>` element.
fn style_element(page: &str) -> &str {
    let open = page.find("<style>").expect("a style element") + "<style>".len();
    let end = page[open..].find("</style>").expect("the style ends");
    &page[open..open + end]
}

/// `css` from the `{` after `head` to its matching `}`, braces balanced.
fn block_after<'a>(css: &'a str, head: &str) -> &'a str {
    let start = css
        .find(head)
        .unwrap_or_else(|| panic!("no {head} in the style"));
    let open = start + css[start..].find('{').unwrap();
    let mut depth = 0usize;
    for (i, c) in css[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &css[open + 1..open + i];
                }
            }
            _ => {}
        }
    }
    panic!("{head} never closes");
}

fn squeezed(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// AC1, AC4 (ids): `/`, an inbox and a thread all serve one page: a status
/// bar over three panes, one script element, and the pane the server picks.
#[tokio::test]
async fn every_console_route_serves_the_three_panes_under_one_status_bar() {
    let srv = start().await;
    let key = seeded_thread_key(srv.http_addr, "panes").await;
    for (path, page) in console_pages(srv.http_addr, "panes", &key).await {
        for id in [
            "id=\"status\"",
            "id=\"panes\"",
            "id=\"inboxes\"",
            "id=\"stream\"",
            "id=\"reader\"",
            "id=\"view\"",
            "id=\"listener-smtp\"",
            "id=\"listener-pop3\"",
            "id=\"listener-http\"",
            "id=\"count-accepted\"",
            "id=\"count-dropped\"",
            "id=\"chaos\"",
            "id=\"live\"",
        ] {
            assert!(page.contains(id), "{path} lacks {id}");
        }
        assert_eq!(script_elements(&page).len(), 1, "{path}");
        let pane = if path == "/" { "inboxes" } else { "stream" };
        assert!(
            page.contains(&format!("<main id=\"panes\" data-pane=\"{pane}\">")),
            "{path}: {pane}"
        );
    }
    // `#view` keeps its two data attributes; on `/` the inbox is empty.
    let (_, root) = http_get_text(srv.http_addr, "/").await;
    assert!(root.contains(r#"data-inbox="""#), "{root}");
    let (_, page) = http_get_text(srv.http_addr, &format!("/ui/inbox/panes/thread/{key}")).await;
    assert!(
        page.contains(r#"data-inbox="panes" data-thread=""#),
        "{page}"
    );
}

/// AC9 (attributes): the rail names the page the server validated, marks the
/// open inbox and only that one.
#[tokio::test]
async fn the_rail_carries_its_page_and_marks_the_open_inbox() {
    let srv = start().await;
    for inbox in ["pg-a", "pg-b", "pg-c"] {
        seed(srv.http_addr, inbox, r#"{"to": "u@x.io"}"#).await;
    }
    let (_, page) = http_get_text(srv.http_addr, "/?limit=1&offset=1").await;
    assert_eq!(attr(&page, "data-limit"), "1");
    assert_eq!(attr(&page, "data-offset"), "1");
    let (_, page) = http_get_text(srv.http_addr, "/ui/inbox/pg-b").await;
    assert_eq!(attr(&page, "data-limit"), "100");
    assert_eq!(attr(&page, "data-offset"), "0");
    assert!(
        page.contains(r#"<a class="inbox" href="/ui/inbox/pg%2Db" aria-current="true">"#),
        "the open inbox is marked: {page}"
    );
    for other in ["pg%2Da", "pg%2Dc"] {
        assert!(
            page.contains(&format!(r#"<a class="inbox" href="/ui/inbox/{other}">"#)),
            "{other} is a plain link: {page}"
        );
    }
    assert_eq!(page.matches("aria-current=\"true\">").count(), 1, "{page}");
}

/// AC10: the server chooses the empty sentence; the script only toggles it.
#[tokio::test]
async fn the_no_inboxes_sentence_is_chosen_by_the_server() {
    let srv = start().await;
    for (path, page) in console_pages(srv.http_addr, "nothing", "k").await {
        assert!(page.contains("No inboxes yet"), "{path}");
        assert!(
            page.contains(r#"id="inbox-empty">"#),
            "{path}: shown, not hidden"
        );
    }
    let key = seeded_thread_key(srv.http_addr, "something").await;
    for (path, page) in console_pages(srv.http_addr, "something", &key).await {
        assert!(!page.contains("No inboxes yet"), "{path}");
        assert!(
            page.contains(r#"id="inbox-empty" hidden>"#),
            "{path}: hidden"
        );
    }
    let script = served_script(srv.http_addr).await;
    assert!(script.contains("No inbox matches."));
    assert!(script.contains("Inboxes unavailable — retrying"));
    assert!(
        !script.contains("No inboxes"),
        "the script holds neither sentence"
    );
}

/// AC7: the demo-only controls of the prototype are not here.
#[tokio::test]
async fn the_console_ships_no_demo_controls() {
    let srv = start().await;
    let key = seeded_thread_key(srv.http_addr, "demo").await;
    for (path, page) in console_pages(srv.http_addr, "demo", &key).await {
        for demo in ["Send sample mail", "Sample data", "aria-pressed"] {
            assert!(!page.contains(demo), "{path} ships {demo}");
        }
        assert!(
            page.contains(r#"<span class="chip chaos-unknown" id="chaos">"#),
            "{path}: the chaos state is a read-only span"
        );
        assert!(!page.contains("<button class=\"chip"), "{path}");
    }
}

/// AC4 (script): the status bar reads the health body before the code, the
/// chaos state read-only, and the live indicator follows the feed.
#[tokio::test]
async fn the_status_bar_is_wired_to_healthz_chaos_and_the_feed() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    for needle in [
        "'/healthz'",
        "degraded",
        "stopping",
        "emails_inserted_total",
        "emails_dropped_total",
        "'/api/v1/chaos'",
        "Chaos unknown",
        "Reconnecting",
        "Offline",
        "Idle",
    ] {
        assert!(script.contains(needle), "the script lacks {needle}");
    }
}

/// AC9 (script): the rail polls its own page, never reads the query string,
/// and the whole page opens one feed.
#[tokio::test]
async fn the_rail_polls_its_own_page_and_opens_one_feed() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    assert!(script.contains("'/api/v1/inboxes?limit='"));
    assert!(script.contains("dataset.limit") && script.contains("dataset.offset"));
    assert!(!script.contains("location.search"));
    assert_eq!(script.matches("new EventSource(").count(), 1);
    assert!(script.contains("sessionStorage"));
}

/// AC11, AC12: each enrichment route is named once, never with the opt-in
/// link probe, and the existing wiring assertions still hold.
#[tokio::test]
async fn enrichment_names_each_route_once_and_never_probes() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    for needle in [
        "api('threads')",
        "'threads/' + encodeURIComponent(threadKey)",
        "api('feed')",
        "paintSeq++",
        "seen !== paintSeq",
    ] {
        assert!(script.contains(needle), "the script lacks {needle}");
    }
    assert_eq!(script.matches("'/projection'").count(), 1);
    assert_eq!(script.matches("'/deliverability'").count(), 1);
    assert!(!script.contains("reachability"));
}

/// AC13: every stream state is its own sentence, none of them the shell's
/// forbidden "is empty", and a failed read is never the empty state.
#[tokio::test]
async fn the_stream_states_are_distinct_strings() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    for needle in [
        "Pick an inbox to watch its mail.",
        "Empty and live.",
        "Stream unavailable — retrying",
        "newest ",
        "no findings",
    ] {
        assert!(script.contains(needle), "the script lacks {needle}");
    }
    let key = seeded_thread_key(srv.http_addr, "states").await;
    for (path, page) in console_pages(srv.http_addr, "states", &key).await {
        assert!(!page.contains("is empty"), "{path}");
    }
}

/// AC14: the console's only write is the confirmed clear, built on the
/// inbox's own messages path, and it never names another mutating route.
#[tokio::test]
async fn the_only_write_is_the_confirmed_clear() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    assert_eq!(script.matches("method: 'DELETE'").count(), 1);
    assert!(script.contains("fetch(api('messages'), { method: 'DELETE' })"));
    assert_eq!(script.matches("/api/v1/chaos").count(), 1, "named to read");
    for needle in ["Clear inbox", "Keep", "Clear failed"] {
        assert!(script.contains(needle), "the script lacks {needle}");
    }
    for forbidden in [
        "'PUT'",
        "'POST'",
        "'PATCH'",
        "/seed",
        "/webhooks",
        "/dns",
        "/raw",
        "/attachments",
    ] {
        assert!(!script.contains(forbidden), "the script names {forbidden}");
    }
}

/// AC15: ingest-time data and the query-time report are labelled apart, no
/// chip claims a positive result, and every reader state has its sentence.
#[tokio::test]
async fn the_reader_labels_ingest_and_query_time_data_apart() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    for needle in [
        "'tab-preview'",
        "'tab-text'",
        "'tab-deliverability'",
        "'tab-tests'",
        "Extracted at ingest",
        "Authentication (query-time report)",
        "no finding",
        "not evaluated",
        "Pick a mail to read it.",
        "Details unavailable — retrying",
        "Deliverability unavailable — retrying",
        "Message not found (pruned or cleared).",
        "No HTML part.",
        "No text part.",
        "No findings does not mean the mail will be delivered.",
    ] {
        assert!(script.contains(needle), "the script lacks {needle}");
    }
    for positive in ["'pass'", "\"pass\"", "=pass"] {
        assert!(!script.contains(positive), "the script claims {positive}");
    }
}

/// AC16: the preview frame is bare-sandboxed before it has content, and no
/// console page grants a token.
#[tokio::test]
async fn the_preview_frame_is_sandboxed_without_tokens() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    let sandbox = script
        .find("setAttribute('sandbox', '')")
        .expect("a bare sandbox");
    let srcdoc = script.find(".srcdoc =").expect("the srcdoc assignment");
    assert!(sandbox < srcdoc, "the sandbox is set before the content");
    let key = seeded_thread_key(srv.http_addr, "sand").await;
    for (path, page) in console_pages(srv.http_addr, "sand", &key).await {
        for token in [
            "allow-scripts",
            "allow-same-origin",
            "allow-top-navigation",
            "allow-popups",
            "allow-forms",
        ] {
            assert!(!page.contains(token), "{path} grants {token}");
        }
    }
}

/// AC17: the snippets use the page's own origin, build the MCP call with
/// `JSON.stringify`, and name only tools this server lists.
#[tokio::test]
async fn the_snippets_name_real_tools_and_quote_safely() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    assert!(script.contains("location.origin"));
    assert!(script.contains("JSON.stringify("));
    assert!(script.contains("shellSegment"));
    let (st, listed) = http_json(
        srv.http_addr,
        "POST",
        "/mcp",
        Some(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#),
    )
    .await;
    assert_eq!(st, 200);
    let tools: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    let mut named = Vec::new();
    let mut rest = script.as_str();
    while let Some(at) = rest.find("swarmail_") {
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
            .unwrap_or(tail.len());
        named.push(&tail[..end]);
        rest = &tail[end..];
    }
    assert!(!named.is_empty(), "the snippets name a tool");
    for name in named {
        assert!(
            tools.contains(&name),
            "{name} is not in tools/list: {tools:?}"
        );
    }
}

/// AC25: no markup sink, no style write, no mail-built link or navigation.
#[tokio::test]
async fn the_script_has_no_markup_sink_no_style_write_and_no_mail_href() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    for sink in [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
        "eval(",
        "Function(",
        ".style",
        "window.open",
        "location.assign",
        "location.replace",
        "location.href =",
        "setAttribute('href'",
    ] {
        assert!(!script.contains(sink), "the script contains {sink}");
    }
    let mut seen = 0;
    for (at, _) in script.match_indices(".href =") {
        seen += 1;
        assert!(
            script[at + ".href =".len()..]
                .trim_start()
                .starts_with("'/ui/"),
            "a href not built from a console path: {}",
            &script[at..at + 60]
        );
    }
    assert!(seen > 0, "the script builds its links");
}

/// AC26: markup in every mail- and client-controlled field never reaches the
/// shell as markup, while the JSON the script paints from carries each one
/// as an equal string; a hostile inbox name is escaped in `data-inbox`.
#[tokio::test]
async fn hostile_mail_never_reaches_the_shell_as_markup() {
    const IMG: &str = "<img src=x onerror=alert(1)>";
    const SCRIPT_TAG: &str = "\"><script>alert(1)</script>";
    let srv = start().await;
    let mut conn = SmtpConn::connect(srv.smtp_addr).await;
    conn.send(&format!("EHLO {SCRIPT_TAG}")).await;
    conn.reply_multi().await;
    let auth = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "\0hostile\0x");
    conn.send(&format!("AUTH PLAIN {auth}")).await;
    assert!(conn.reply().await.starts_with("235"));
    for line in ["MAIL FROM:<s@x.io>", "RCPT TO:<r@x.io>"] {
        conn.send(line).await;
        assert!(conn.reply().await.starts_with("250"));
    }
    let raw = format!(
        "From: \"{IMG}\" <s@x.io>\r\nTo: r@x.io\r\nSubject: {IMG}\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=b\r\n\r\n\
         --b\r\nContent-Type: text/plain\r\n\r\nYour code is 424242 {IMG}\r\n\
         --b\r\nContent-Type: text/html\r\n\r\n<p><a href=\"https://a.example/p\">&lt;img src=x onerror=alert(1)&gt;</a></p>\r\n\
         --b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"{IMG}\"\r\n\r\npdf\r\n\
         --b\r\nContent-Type: text/calendar\r\n\r\nBEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:{IMG}\r\nDTSTART:20261118T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n\
         --b--"
    );
    let accepted = conn.data(&raw).await;
    assert!(accepted.starts_with("250"), "{accepted}");
    let id = accepted.split_whitespace().last().unwrap().to_string();

    let (_, threads) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/hostile/threads",
        None,
    )
    .await;
    let key = threads[0]["key"].as_str().unwrap().to_string();
    let email = &threads[0]["emails"][0];
    assert_eq!(email["subject"], IMG);
    assert_eq!(email["from"]["name"], IMG);
    assert!(email["text"].as_str().unwrap().contains(IMG), "{email}");
    let (_, projection) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}/projection"),
        None,
    )
    .await;
    assert_eq!(projection["smtp_helo"], SCRIPT_TAG, "{projection}");
    assert_eq!(
        projection["links"]["items"][0]["anchor_text"], IMG,
        "{projection}"
    );
    assert_eq!(projection["attachments"]["items"][0]["filename"], IMG);
    assert_eq!(projection["attachments"]["items"][1]["ics"]["summary"], IMG);

    for (path, page) in console_pages(srv.http_addr, "hostile", &key).await {
        assert!(!page.contains("<img src=x"), "{path}");
        assert!(!page.contains("<script>alert"), "{path}");
        assert_eq!(script_elements(&page).len(), 1, "{path}");
    }
    // The standalone permalink escapes the same payloads.
    let (_, page) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    assert!(!page.contains("<img src=x"), "{page}");
    assert!(!page.contains("<script>alert"), "{page}");

    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/a%22%3E%3Cb").await;
    assert_eq!(st, 200);
    assert!(page.contains(r#"data-inbox="a&quot;&gt;&lt;b""#), "{page}");
    assert!(!page.contains(r#"data-inbox="a">"#), "{page}");
}

/// AC27: one policy string, byte for byte, on every UI route.
#[tokio::test]
async fn every_ui_route_ships_the_one_policy_byte_for_byte() {
    let srv = start().await;
    let key = seeded_thread_key(srv.http_addr, "policy").await;
    let (_, list) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/policy/messages",
        None,
    )
    .await;
    let id = list["emails"][0]["id"].as_str().unwrap().to_string();
    let mut pages = console_pages(srv.http_addr, "policy", &key).await;
    let (_, message) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
    pages.push(("message".into(), message));
    let meta = format!(r#"<meta http-equiv="Content-Security-Policy" content="{POLICY}">"#);
    for (path, page) in pages {
        assert!(page.contains(&meta), "{path}");
    }
}

/// AC28: the console loads nothing from outside.
#[tokio::test]
async fn the_console_loads_nothing_from_outside() {
    let srv = start().await;
    for (path, page) in console_pages(srv.http_addr, "benign", "k").await {
        for external in [
            "http://",
            "https://",
            "<link",
            "@import",
            "url(",
            "@font-face",
            "src=\"//",
            "href=\"//",
        ] {
            assert!(!page.contains(external), "{path} contains {external}");
        }
    }
}

/// AC29: both themes, reduced motion, and exactly one pane at phone width.
#[tokio::test]
async fn the_stylesheet_shows_one_pane_at_phone_width_and_has_both_themes() {
    let srv = start().await;
    let (_, page) = http_get_text(srv.http_addr, "/").await;
    let css = style_element(&page);
    assert!(css.contains("@media (prefers-color-scheme: dark)"));
    assert!(css.contains("@media (prefers-reduced-motion: reduce)"));
    // The reduced-motion rule must name the selector that animates the ring
    // (a weaker selector loses the cascade whatever its order).
    let calm = squeezed(block_after(css, "@media (prefers-reduced-motion: reduce)"));
    assert!(calm.contains(".live-on.pulse::after"), "{calm}");
    assert!(calm.contains("animation:none"), "{calm}");
    assert_eq!(css.matches("@media (max-width: 680px)").count(), 1);
    let phone = squeezed(block_after(css, "@media (max-width: 680px)"));
    assert!(phone.contains("#panes>.pane{display:none}"), "{phone}");
    assert!(
        phone.contains(
            r##"#panes[data-pane="inboxes"]>#inboxes,#panes[data-pane="stream"]>#stream,#panes[data-pane="reader"]>#reader{display:block}"##
        ),
        "{phone}"
    );
    let dark = block_after(css, "@media (prefers-color-scheme: dark)");
    assert!(squeezed(dark).contains("color-scheme:dark"));
    // The media query's own name ends in `color-scheme: dark` too.
    let outside = css
        .replace(dark, "")
        .replace("@media (prefers-color-scheme: dark)", "");
    assert!(
        !squeezed(&outside).contains("color-scheme:dark"),
        "only the dark block may ask for the dark scheme"
    );
}

/// R2.2: the counters say what they count, in the bar the server renders.
#[tokio::test]
async fn the_status_bar_labels_its_counters_as_lifetime_and_cap_eviction() {
    let srv = start().await;
    let (_, page) = http_get_text(srv.http_addr, "/").await;
    assert!(
        page.contains(r#"<b id="count-accepted">–</b> accepted (lifetime)"#),
        "{page}"
    );
    assert!(
        page.contains(r#"<b id="count-dropped">–</b> dropped (evicted by cap, lifetime)"#),
        "{page}"
    );
    assert!(
        page.contains(r#"title="Mail evicted by the per-inbox cap since the server started""#),
        "{page}"
    );
}

/// An inbox name is client-controlled (the SMTP AUTH user) and the rail
/// shows it to every visitor: the server-rendered rail label, `<title>` and
/// stream title must carry it as escaped text, never as markup.
#[tokio::test]
async fn a_markup_named_inbox_is_escaped_in_the_rail_title_and_stream_title() {
    const NAME: &str = "<img src=x onerror=alert(1)>";
    const ENCODED: &str = "%3Cimg%20src%3Dx%20onerror%3Dalert(1)%3E";
    let srv = start().await;
    send_headers(srv.smtp_addr, NAME, &[], "hello").await;
    let (_, names) = http_json(srv.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert!(names.to_string().contains("onerror"), "seeded: {names}");

    for path in ["/".to_string(), format!("/ui/inbox/{ENCODED}")] {
        let (st, page) = http_get_text(srv.http_addr, &path).await;
        assert_eq!(st, 200, "{path}");
        assert!(!page.contains("<img src=x"), "{path}: raw markup in {page}");
        assert!(
            page.contains("&lt;img src=x onerror=alert(1)&gt;"),
            "{path}: the name is shown as escaped text"
        );
    }
    let (_, page) = http_get_text(srv.http_addr, &format!("/ui/inbox/{ENCODED}")).await;
    assert!(
        page.contains("<title>&lt;img src=x onerror=alert(1)&gt; · Swarmail</title>")
            || page.contains("&lt;img src=x onerror=alert(1)&gt; · Swarmail</title>"),
        "{page}"
    );
    assert!(
        page.contains(r#"id="stream-title">&lt;img src=x onerror=alert(1)&gt;</span>"#),
        "{page}"
    );
}

/// The legacy link list is a fallback for a settled projection that stored no
/// occurrences: capped like every list on the page, never painted while the
/// projection is loading or failed (one mail may carry millions of URLs).
#[tokio::test]
async fn the_bare_link_fallback_is_capped_and_waits_for_a_settled_projection() {
    let srv = start().await;
    let script = served_script(srv.http_addr).await;
    assert!(script.contains("const LIST_CAP = 250;"), "{script}");
    assert!(
        script.contains("Math.min(bare.length, LIST_CAP)"),
        "the fallback is bounded"
    );
    assert!(
        script.contains("} else if (ready) {\n    // The projection is settled"),
        "the fallback runs only for a settled projection"
    );
    assert!(
        !script.contains("for (const url of email.links"),
        "unbounded loop"
    );
    assert!(script.contains("'Links loading…'"));
    assert!(script.contains("'Links unavailable — retrying'"));
    // An in-flight fetch is never queued a second time.
    assert!(script.contains("if (id === fetching) return;"));
}
