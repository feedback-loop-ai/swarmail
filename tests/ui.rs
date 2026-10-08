//! The server-rendered UI: every page, populated and empty.

mod common;

use common::{http_get_text, start};

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
async fn inbox_view_shows_mail_and_empty_state() {
    let srv = start().await;
    seed(
        srv.http_addr,
        "iv",
        r#"{"to": "u@x.io", "subject": "inbox view", "text": "plain body"}"#,
    )
    .await;

    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/iv").await;
    assert_eq!(st, 200);
    assert!(page.contains("inbox view"), "{page}");

    let (st, page) = http_get_text(srv.http_addr, "/ui/inbox/empty-inbox").await;
    assert_eq!(st, 200);
    assert!(page.contains("is empty"), "{page}");
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
}
