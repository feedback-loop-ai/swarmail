//! MailHog / Mailpit compat shims over the wire: real HTTP against the same
//! server the SMTP tests use, asserting the exact upstream JSON shapes.

mod common;

use common::{SmtpConn, http_get_text, http_json, smtp_send, start};
use serde_json::{Value, json};
use std::net::SocketAddr;
use swarmail::RunningServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The shim mail factory: an SMTP session that names its inbox via AUTH and
/// sends an arbitrary DATA payload. Returns the swarmail id.
async fn send_raw(s: std::net::SocketAddr, inbox: &str, data: &str) -> String {
    let mut conn = SmtpConn::connect(s).await;
    let plain = format!("\u{0}{inbox}\u{0}whatever");
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    conn.reply().await;
    conn.send("MAIL FROM:<sender@x.io>").await;
    conn.reply().await;
    conn.send("RCPT TO:<ignored>").await;
    conn.reply().await;
    let reply = conn.data(data).await;
    assert!(reply.starts_with("250"), "DATA refused: {reply}");
    reply
        .split_whitespace()
        .last()
        .unwrap()
        .trim_end_matches('.')
        .to_string()
}

/// HTTP with header visibility: (status, headers, body bytes) — the JSON
/// helper can't assert content types or dispositions.
async fn http_raw(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<(&str, &str)>,
) -> (u16, Vec<(String, String)>, Vec<u8>) {
    let payload = match body {
        Some((ctype, text)) => format!(
            "{method} {path} HTTP/1.1\r\nHost: t\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
            text.len()
        ),
        None => format!("{method} {path} HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n"),
    };
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(payload.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no header/body split");
    let head = String::from_utf8_lossy(&buf[..split]).to_string();
    let mut lines = head.lines();
    let status: u16 = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_lowercase(), v.to_string()))
        .collect();
    (status, headers, buf[split + 4..].to_vec())
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// Two inboxes, two mails each, in a known order. Returns
/// (oldest_alpha, newest_alpha, oldest_beta, newest_beta).
async fn seed(s: &RunningServer) -> (String, String, String, String) {
    smtp_send(
        s.smtp_addr,
        Some("alpha"),
        "sender@x.io",
        "rcpt@x.io",
        "Alpha old",
        "alpha one",
    )
    .await
    .unwrap();
    let a1 = latest_id(s.http_addr, "alpha").await;
    smtp_send(
        s.smtp_addr,
        Some("alpha"),
        "sender@x.io",
        "rcpt@x.io",
        "Alpha new",
        "alpha two",
    )
    .await
    .unwrap();
    let a2 = latest_id(s.http_addr, "alpha").await;
    smtp_send(
        s.smtp_addr,
        Some("beta"),
        "sender@x.io",
        "rcpt@x.io",
        "Beta old",
        "beta one",
    )
    .await
    .unwrap();
    let b1 = latest_id(s.http_addr, "beta").await;
    smtp_send(
        s.smtp_addr,
        Some("beta"),
        "sender@x.io",
        "rcpt@x.io",
        "Beta new",
        "beta two",
    )
    .await
    .unwrap();
    let b2 = latest_id(s.http_addr, "beta").await;
    (a1, a2, b1, b2)
}

async fn latest_id(addr: SocketAddr, inbox: &str) -> String {
    let (status, body) = http_json(
        addr,
        "GET",
        &format!("/api/v1/inboxes/{inbox}/messages"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    body["emails"][0]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn mailpit_envelope_lists_the_store_newest_first() {
    let s = start().await;
    let (a1, _a2, _b1, b2) = seed(&s).await;

    let (status, body) = http_json(s.http_addr, "GET", "/api/v1/messages", None).await;
    assert_eq!(status, 200);
    // The whole-store envelope: every count mirrors `total` (no read state).
    assert_eq!(body["total"], 4);
    assert_eq!(body["unread"], 4);
    assert_eq!(body["messages_count"], 4);
    assert_eq!(body["messages_unread_count"], 4);
    assert_eq!(body["count"], 4);
    assert_eq!(body["start"], 0);
    assert_eq!(body["tags"], json!([]));
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    // Newest first — the last mail delivered leads the list.
    assert_eq!(messages[0]["ID"], b2.as_str());
    assert_eq!(messages[3]["ID"], a1.as_str());
    // Summary shape, per upstream's MessageSummary.
    let m = &messages[0];
    assert_eq!(m["MessageID"], m["ID"]);
    assert_eq!(m["Read"], false);
    assert_eq!(m["From"], json!({"address": "sender@x.io"}));
    assert_eq!(m["To"], json!([{"address": "rcpt@x.io"}]));
    assert_eq!(m["Cc"], json!([]));
    assert_eq!(m["Bcc"], json!([]));
    assert_eq!(m["ReplyTo"], json!([]));
    assert_eq!(m["Subject"], "Beta new");
    assert_eq!(m["Username"], "beta");
    assert_eq!(m["Tags"], json!([]));
    assert_eq!(m["Attachments"], 0);
    assert_eq!(m["Snippet"], "beta two");
    assert!(m["Created"].as_str().is_some());
    assert!(m["Size"].as_u64().is_some());

    // Scoped to one inbox, with Mailpit's paging.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/messages?inbox=alpha&limit=1&start=1",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 2);
    assert_eq!(body["count"], 1);
    assert_eq!(body["start"], 1);
    assert_eq!(body["messages"][0]["ID"], a1.as_str());
    assert_eq!(body["messages"][0]["Username"], "alpha");
}

#[tokio::test]
async fn mailpit_search_supports_each_kind_and_scopes() {
    let s = start().await;
    seed(&s).await;
    // A mail whose token only exists in the raw source: a Cc header.
    let cc_id = send_raw(
        s.smtp_addr,
        "alpha",
        "From: sender@x.io\r\nTo: rcpt@x.io\r\nCc: carbon@x.io\r\nSubject: Alpha cc\r\n\r\nalpha three\r\n",
    )
    .await;

    // Containing: found in the extracted text part.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?kind=containing&query=alpha+two",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    assert_eq!(body["messages"][0]["Subject"], "Alpha new");

    // Containing: found in a recipient address.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?kind=containing&query=carbon",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    assert_eq!(body["messages"][0]["ID"], cc_id.as_str());

    // kind=to / kind=from / kind=subject ride the store filter: substring
    // matching against the extracted addresses and subject, scoped here to
    // the alpha inbox.
    async fn total_of(addr: SocketAddr, query: &str) -> (u16, Value) {
        let (status, body) = http_json(
            addr,
            "GET",
            &format!("/api/v1/search?{query}&inbox=alpha"),
            None,
        )
        .await;
        (status, body["total"].clone())
    }
    for (query, expected, what) in [
        ("kind=to&query=rcpt", 3, "every alpha recipient matches"),
        ("kind=to&query=nobody", 0, "no such recipient"),
        ("kind=from&query=sender", 3, "every alpha sender matches"),
        ("kind=from&query=other", 0, "no such sender"),
        ("kind=subject&query=old", 1, "one subject contains old"),
        (
            "kind=subject&query=ALPHA",
            3,
            "subject match is case-insensitive",
        ),
    ] {
        let (status, total) = total_of(s.http_addr, query).await;
        assert_eq!(status, 200, "{what}");
        assert_eq!(total, expected, "{what} ({query})");
    }

    // A kind-less Mailpit search defaults to containing.
    let (status, body) =
        http_json(s.http_addr, "GET", "/api/v1/search?query=alpha+one", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    assert_eq!(body["messages"][0]["Subject"], "Alpha old");

    // kind=subject is not a MailHog kind — Mailpit accepts it, the v2 shape
    // answers a bare 400 (asserted in the MailHog test below).
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?kind=bogus&query=x",
        None,
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "unknown search kind: bogus");

    let (status, body) = http_json(s.http_addr, "GET", "/api/v1/search?kind=to", None).await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "Error: no search query");
}

#[tokio::test]
async fn mailpit_message_plain_and_headers_endpoints() {
    let s = start().await;
    smtp_send(
        s.smtp_addr,
        Some("solo"),
        "sender@x.io",
        "rcpt@x.io",
        "Solo",
        "the body",
    )
    .await
    .unwrap();
    let id = latest_id(s.http_addr, "solo").await;
    // An HTML-only mail for the /plain fallback.
    let html_id = send_raw(
        s.smtp_addr,
        "solo",
        "From: sender@x.io\r\nTo: rcpt@x.io\r\nSubject: Html only\r\n\r\n<p>html body</p>\r\n",
    )
    .await;

    // Full message shape (upstream's Message).
    let (status, m) = http_json(s.http_addr, "GET", &format!("/api/v1/message/{id}"), None).await;
    assert_eq!(status, 200);
    for key in [
        "ID",
        "MessageID",
        "Read",
        "From",
        "To",
        "Cc",
        "Bcc",
        "ReplyTo",
        "ReturnPath",
        "Subject",
        "Date",
        "Tags",
        "Username",
        "Text",
        "HTML",
        "Size",
        "Inline",
        "Attachments",
    ] {
        assert!(m.get(key).is_some(), "missing {key} in {m}");
    }
    assert_eq!(m["ID"], id.as_str());
    assert_eq!(m["Username"], "solo");
    assert_eq!(m["ReturnPath"], "sender@x.io");
    assert_eq!(m["Text"], "the body\r\n");
    assert_eq!(m["Attachments"], json!([]));
    assert_eq!(m["Inline"], json!([]));

    // text/plain body.
    let (status, headers, body) = http_raw(
        s.http_addr,
        "GET",
        &format!("/api/v1/message/{id}/plain"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        header_value(&headers, "content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(body, b"the body\r\n");

    // The plural path MailHog clients use answers the same way.
    let (status, _, plural) = http_raw(
        s.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}/plain"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(plural, b"the body\r\n");

    // HTML-only mail: /plain serves the readable form extract found.
    let (status, _, body) = http_raw(
        s.http_addr,
        "GET",
        &format!("/api/v1/message/{html_id}/plain"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&body).contains("html body"));

    // Synthesized header map.
    let (status, h) = http_json(
        s.http_addr,
        "GET",
        &format!("/api/v1/message/{id}/headers"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(h["From"], json!(["sender@x.io"]));
    assert_eq!(h["To"], json!(["rcpt@x.io"]));
    assert_eq!(h["Subject"], json!(["Solo"]));
    assert_eq!(h["Return-Path"], json!(["<sender@x.io>"]));
    assert!(h["Date"].as_array().is_some());
    assert!(h["Received"][0].as_str().unwrap().contains("swarmail-smtp"));

    // Missing ids are 404s on every read shape.
    for path in [
        "/api/v1/message/nope",
        "/api/v1/message/nope/plain",
        "/api/v1/message/nope/headers",
        "/api/v1/messages/nope/plain",
        "/api/v1/messages/nope/download",
        "/api/v1/message/nope/raw",
    ] {
        let (status, _, _) = http_raw(s.http_addr, "GET", path, None).await;
        assert_eq!(status, 404, "{path}");
    }
}

#[tokio::test]
async fn mailhog_raw_and_download_serve_the_source() {
    let s = start().await;
    smtp_send(
        s.smtp_addr,
        Some("hogged"),
        "sender@x.io",
        "rcpt@x.io",
        "Hogged",
        "raw body",
    )
    .await
    .unwrap();
    let id = latest_id(s.http_addr, "hogged").await;
    let expected = "From: sender@x.io\r\nTo: rcpt@x.io\r\nSubject: Hogged\r\n\r\nraw body\r\n";

    for path in [
        format!("/api/v1/messages/{id}/download"),
        format!("/api/v1/message/{id}/raw"),
    ] {
        let (status, headers, body) = http_raw(s.http_addr, "GET", &path, None).await;
        assert_eq!(status, 200, "{path}");
        assert_eq!(
            header_value(&headers, "content-type"),
            Some("message/rfc822"),
            "{path}"
        );
        assert_eq!(body, expected.as_bytes(), "{path}");
    }
    // The download names itself after the message.
    let (status, headers, _) = http_raw(
        s.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}/download"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        header_value(&headers, "content-disposition"),
        Some(format!("attachment; filename=\"{id}.eml\"").as_str())
    );
}

#[tokio::test]
async fn mailhog_v2_lists_and_searches_with_upstream_shapes() {
    let s = start().await;
    let (a1, _a2, _b1, b2) = seed(&s).await;

    let (status, body) = http_json(s.http_addr, "GET", "/api/v2/messages", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 4);
    assert_eq!(body["count"], 4);
    assert_eq!(body["start"], 0);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0]["ID"], b2.as_str());
    assert_eq!(items[3]["ID"], a1.as_str());

    // data.Message shape, quirks included.
    let m = &items[0];
    assert_eq!(
        m["From"],
        json!({"Relays": null, "Mailbox": "sender", "Domain": "x.io", "Params": ""})
    );
    assert_eq!(
        m["To"],
        json!([{"Relays": null, "Mailbox": "rcpt", "Domain": "x.io", "Params": ""}])
    );
    assert_eq!(m["Raw"]["From"], "sender@x.io");
    assert_eq!(m["Raw"]["To"], json!(["rcpt@x.io"]));
    assert_eq!(m["Raw"]["Helo"], "");
    assert!(
        m["Raw"]["Data"]
            .as_str()
            .unwrap()
            .contains("Subject: Beta new")
    );
    assert_eq!(m["Content"]["Body"], "beta two\r\n");
    assert_eq!(m["Content"]["Size"], 65);
    // MIME appears at both levels, with the extracted text part.
    let mime = &m["MIME"];
    assert_eq!(
        mime["Parts"][0]["Headers"]["Content-Type"],
        json!(["text/plain; charset=UTF-8"])
    );
    assert_eq!(mime["Parts"][0]["Body"], "beta two\r\n");
    assert_eq!(m["Content"]["MIME"], *mime);
    // Synthesized headers include the honest stamps.
    assert_eq!(m["Content"]["Headers"]["Subject"], json!(["Beta new"]));
    assert_eq!(
        m["Content"]["Headers"]["Return-Path"],
        json!(["<sender@x.io>"])
    );

    // Search with each valid kind, plus the bare-400 quirks.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?kind=containing&query=alpha+one",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["ID"], a1.as_str());

    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?kind=from&query=sender&inbox=beta",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 2);

    // A kind-less v2 search defaults to containing too.
    let (status, body) = http_json(s.http_addr, "GET", "/api/v2/search?query=beta+two", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["ID"], b2.as_str());

    // Upstream answers a bare 400 — no body, no message — for a bad kind or
    // an empty query.
    for query in ["kind=bogus&query=x", "kind=to"] {
        let (status, headers, body) =
            http_raw(s.http_addr, "GET", &format!("/api/v2/search?{query}"), None).await;
        assert_eq!(status, 400, "{query}");
        assert!(body.is_empty(), "{query} must be a bare 400");
        assert!(header_value(&headers, "content-length").is_some());
    }
}

#[tokio::test]
async fn bodyless_mail_has_a_null_mime_in_mailhog_shapes() {
    let s = start().await;
    send_raw(s.smtp_addr, "bare", "no header block at all").await;
    let id = latest_id(s.http_addr, "bare").await;
    let (_, body) = http_json(s.http_addr, "GET", "/api/v2/messages?inbox=bare", None).await;
    assert_eq!(body["items"][0]["ID"], id.as_str());
    assert_eq!(body["items"][0]["MIME"], json!(null));
    assert_eq!(body["items"][0]["Content"]["MIME"], json!(null));
    assert_eq!(body["items"][0]["Content"]["Body"], "");
}

#[tokio::test]
async fn a_mail_without_a_from_header_reads_as_an_empty_path() {
    let s = start().await;
    // No From header and an empty envelope sender: the reverse-path is
    // genuinely unknown, so both shapes must say so.
    let mut conn = SmtpConn::connect(s.smtp_addr).await;
    conn.send("MAIL FROM:<>").await;
    conn.reply().await;
    conn.send("RCPT TO:<nobody>").await;
    conn.reply().await;
    conn.data("Subject: anonymous\r\n\r\nno from at all\r\n")
        .await;
    let id = latest_id(s.http_addr, "default").await;

    let (_, hog) = http_json(s.http_addr, "GET", "/api/v2/messages?inbox=default", None).await;
    assert_eq!(hog["items"][0]["ID"], id.as_str());
    assert_eq!(
        hog["items"][0]["From"],
        json!({"Relays": null, "Mailbox": "", "Domain": "", "Params": ""})
    );
    assert_eq!(hog["items"][0]["Raw"]["From"], "");

    let (_, pit) = http_json(s.http_addr, "GET", "/api/v1/messages?inbox=default", None).await;
    assert_eq!(pit["messages"][0]["ID"], id.as_str());
    assert_eq!(pit["messages"][0]["From"], json!(null));

    let (_, full) = http_json(s.http_addr, "GET", &format!("/api/v1/message/{id}"), None).await;
    assert_eq!(full["From"], json!(null));
    assert_eq!(full["ReturnPath"], "");
}

#[tokio::test]
async fn long_bodies_get_a_truncated_snippet() {
    let s = start().await;
    let long = "word ".repeat(60);
    smtp_send(
        s.smtp_addr,
        Some("snips"),
        "sender@x.io",
        "rcpt@x.io",
        "Snips",
        &long,
    )
    .await
    .unwrap();
    let (_, body) = http_json(s.http_addr, "GET", "/api/v1/messages?inbox=snips", None).await;
    let snippet = body["messages"][0]["Snippet"].as_str().unwrap();
    assert_eq!(snippet.len(), 203);
    assert!(snippet.ends_with("..."));
}

#[tokio::test]
async fn shim_deletes_go_through_the_store() {
    let s = start().await;
    let (a1, a2, b1, b2) = seed(&s).await;

    // Mailpit's selective delete: exactly the named ids.
    let payload = json!({"ids": [a1, b2]}).to_string();
    let (status, _, raw) = http_raw(
        s.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 2}));

    for id in [&a1, &b2] {
        let (status, _) =
            http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
        assert_eq!(status, 404, "{id} must be gone");
    }
    for id in [&a2, &b1] {
        let (status, _) =
            http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
        assert_eq!(status, 200, "{id} must survive");
    }

    // MailHog's wipe: no body at all.
    let (status, _, raw) = http_raw(s.http_addr, "DELETE", "/api/v1/messages", None).await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 2}));
    let (status, body) = http_json(s.http_addr, "GET", "/api/v1/messages", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 0);

    // Re-seed and wipe through the alias + the unparseable-body path.
    let (_a1, _a2, _b1, _b2) = seed(&s).await;
    let (status, _, raw) = http_raw(s.http_addr, "DELETE", "/api/v1/delete-all", None).await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 4}));

    let (_a1, _a2, _b1, _b2) = seed(&s).await;
    let (status, _, raw) = http_raw(
        s.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", "not json at all")),
    )
    .await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 4}));

    // An empty ids array is "delete everything", like upstream.
    let (_a1, _a2, _b1, _b2) = seed(&s).await;
    let (status, _, raw) = http_raw(
        s.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", r#"{"ids": []}"#)),
    )
    .await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 4}));
}

#[tokio::test]
async fn the_shims_coexist_with_the_native_routes() {
    let s = start().await;
    let (_a1, _a2, _b1, _b2) = seed(&s).await;
    // The native single-message route is untouched by the merge.
    let (status, _) = http_get_text(s.http_addr, "/api/v1/inboxes").await;
    assert_eq!(status, 200);
    let (status, body) =
        http_json(s.http_addr, "GET", "/api/v1/inboxes/alpha/messages", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 2);
    // The documented shapes keep their native envelope (`emails`, not `items`).
    assert!(body["emails"].is_array());
}
