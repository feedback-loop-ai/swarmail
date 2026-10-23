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
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
            text.len()
        ),
        None => format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"),
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
    // Upstream's own key (issue #99): `messages_unread`, not the old
    // `messages_unread_count`.
    assert_eq!(body["messages_unread"], 4);
    assert!(body.get("messages_unread_count").is_none(), "{body}");
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

    // Upstream fills Mailpit's `total`/`unread` from the mailbox stats
    // (`storage.StatsGet()`), not from the result page (issue #55): the
    // store here is five mails (three alpha, two beta), so even a
    // one-hit search reports total/unread five. The match count rides
    // `messages_count`/`messages_unread`.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?kind=containing&query=alpha+two",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 5, "total is the mailbox, not the page");
    assert_eq!(body["unread"], 5);
    assert_eq!(
        body["messages_count"], 1,
        "the match count rides messages_count"
    );
    assert_eq!(body["messages_unread"], 1);
    assert_eq!(body["messages"].as_array().map(Vec::len), Some(1));
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
    assert_eq!(body["total"], 5);
    assert_eq!(body["messages_count"], 1);
    assert_eq!(body["messages"][0]["ID"], cc_id.as_str());

    // kind=to / kind=from / kind=subject ride the store filter: substring
    // matching against the extracted addresses and subject, scoped here to
    // the alpha inbox — whose mailbox total is three, reported even for
    // queries that match nothing (issue #55).
    async fn totals_of(addr: SocketAddr, query: &str) -> (u16, Value, Value) {
        let (status, body) = http_json(
            addr,
            "GET",
            &format!("/api/v1/search?{query}&inbox=alpha"),
            None,
        )
        .await;
        (
            status,
            body["total"].clone(),
            body["messages_count"].clone(),
        )
    }
    for (query, matches, what) in [
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
        let (status, total, messages_count) = totals_of(s.http_addr, query).await;
        assert_eq!(status, 200, "{what}");
        assert_eq!(total, 3, "total is the alpha mailbox ({query})");
        assert_eq!(messages_count, matches, "{what} ({query})");
    }

    // A kind-less Mailpit search defaults to containing.
    let (status, body) =
        http_json(s.http_addr, "GET", "/api/v1/search?query=alpha+one", None).await;
    assert_eq!(status, 200);
    assert_eq!(body["total"], 5);
    assert_eq!(body["messages_count"], 1);
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

    // Upstream's search() whitelist rejects an absent kind exactly like an
    // unknown one (issue #55): there is no default on this plane — MailHog's
    // api/v2.go answers 400 unless the kind is present and one of
    // from/to/containing. The v1 Mailpit plane keeps its default (pinned in
    // mailpit_search_supports_each_kind_and_scopes).
    let (status, headers, body) =
        http_raw(s.http_addr, "GET", "/api/v2/search?query=beta+two", None).await;
    assert_eq!(status, 400, "an absent kind is a bare 400, not a default");
    assert!(body.is_empty(), "an absent kind must be a bare 400");
    assert!(header_value(&headers, "content-length").is_some());

    // Upstream answers a bare 400 — no body, no message — for a bad kind or
    // an empty query. `subject` is the swarmail engine's own kind and is
    // not this dialect's contract: the v2 plane keeps upstream's bare 400
    // for it (the domain is staged in search_hog, not widened by the
    // shared engine).
    for query in ["kind=bogus&query=x", "kind=to", "kind=subject&query=x"] {
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

/// The shim DELETE contract (decision 0011): an explicit non-empty `ids`
/// list deletes exactly those mails through the real store; every other
/// body — absent, empty, unparseable, wrong-typed or `{"ids": []}` — is a
/// `400` that deletes nothing, and `delete-all` stays the sole wipe.
#[tokio::test]
async fn shim_deletes_go_through_the_store() {
    let s = start().await;
    let (a1, a2, b1, b2) = seed(&s).await;

    // Every destructive-looking request that names no target refuses — and
    // after each one both inboxes still hold exactly their mail (exact
    // before/after counts, not just a status: a silent wipe must never read
    // as a pass, the #6 shape).
    for body in [
        None,
        Some(("application/json", "")),
        Some(("application/json", "not json at all")),
        Some(("application/json", r#"{"ids": null}"#)),
        Some(("application/json", r#"{"ids": "alpha one"}"#)),
        Some(("application/json", r#"{"ids": [1, 2]}"#)),
        Some(("application/json", r#"{"ids": []}"#)),
    ] {
        let (status, _, raw) = http_raw(s.http_addr, "DELETE", "/api/v1/messages", body).await;
        assert_eq!(
            status,
            400,
            "{body:?} must refuse: {}",
            String::from_utf8_lossy(&raw)
        );
        let refused: Value = serde_json::from_slice(&raw).unwrap();
        assert!(
            refused["error"].as_str().unwrap().contains("ids"),
            "the 400 names the contract: {refused}"
        );
        assert_eq!(
            inbox_counts(&s).await,
            (2, 2),
            "the refusal deleted nothing: {body:?}"
        );
    }

    // The selector: exactly the named ids, across inboxes, synchronously
    // through the store.
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
    assert_eq!(
        inbox_counts(&s).await,
        (1, 1),
        "the selective delete touched nothing else"
    );

    // The other half through the same selector: the store is now empty, so
    // the next seed is the only mail there is.
    let payload = json!({"ids": [a2, b1]}).to_string();
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
    assert_eq!(
        inbox_counts(&s).await,
        (0, 0),
        "the selector empties exactly what it names"
    );

    // An id the store cannot find is counted honestly: the loop skips it,
    // the reply reports only what was actually removed, and the live mail
    // is untouched — a stale id in a client's list is not an error, but it
    // is never a removal either.
    let (_a1, a2, _b1, _b2) = seed(&s).await;
    let payload = json!({"ids": [a2, "no-such-id"]}).to_string();
    let (status, _, raw) = http_raw(
        s.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(
        removed,
        json!({"removed": 1}),
        "{}",
        String::from_utf8_lossy(&raw)
    );
    let (status, _) = http_json(s.http_addr, "GET", "/api/v1/messages/no-such-id", None).await;
    assert_eq!(status, 404);
    assert_eq!(
        inbox_counts(&s).await,
        (1, 2),
        "the unknown id removed nothing else"
    );

    // The sole cross-inbox wipe: the explicit alias, unchanged (0004) —
    // it takes exactly the store it finds: the unknown-id leg's survivor
    // plus the freshly seeded 4.
    let (_a1, _a2, _b1, _b2) = seed(&s).await;
    let (status, _, raw) = http_raw(s.http_addr, "DELETE", "/api/v1/delete-all", None).await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 7}));
    assert_eq!(
        inbox_counts(&s).await,
        (0, 0),
        "the alias is the one and only wipe"
    );
}

/// Exact per-inbox counts: destructive assertions are proven by before/after
/// mail over BOTH inboxes, never by a status code alone.
async fn inbox_counts(s: &RunningServer) -> (u64, u64) {
    let (st, alpha) = http_json(s.http_addr, "GET", "/api/v1/inboxes/alpha/count", None).await;
    assert_eq!(st, 200);
    let (st, beta) = http_json(s.http_addr, "GET", "/api/v1/inboxes/beta/count", None).await;
    assert_eq!(st, 200);
    (
        alpha["count"].as_u64().unwrap(),
        beta["count"].as_u64().unwrap(),
    )
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

/// A mail with a Cc header lands its display-name addresses in the mailpit
/// shim's Cc array (the `mailpit_address(Some(a))` map over `email.cc`).
#[tokio::test]
async fn cc_addresses_render_in_the_mailpit_list() {
    let srv = start().await;
    let mut c = SmtpConn::connect(srv.smtp_addr).await;
    c.send("EHLO t").await;
    c.reply().await;
    c.send("MAIL FROM:<sender@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    c.send("RCPT TO:<cclist@x.io>").await;
    assert!(c.reply().await.starts_with("250"));
    assert!(
        c.data("From: sender@x.io\r\nTo: cclist@x.io\r\nCc: \"Cara C\" <cara@x.io>, dot@x.io\r\nSubject: cc'd\r\n\r\nx")
            .await
            .starts_with("250")
    );

    let (st, list) = http_json(srv.http_addr, "GET", "/api/v1/messages", None).await;
    assert_eq!(st, 200);
    let mail = list["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["Subject"] == "cc'd")
        .expect("the cc'd mail is listed");
    let cc = mail["Cc"].as_array().expect("a Cc array");
    assert_eq!(cc.len(), 2, "{mail}");
    assert_eq!(cc[0]["name"], "Cara C");
    assert_eq!(cc[0]["address"], "cara@x.io");
    assert_eq!(cc[1]["address"], "dot@x.io");

    // The full single-message shape carries the same Cc array.
    let id = mail["ID"].as_str().unwrap().to_string();
    let (st, one) = http_json(srv.http_addr, "GET", &format!("/api/v1/message/{id}"), None).await;
    assert_eq!(st, 200);
    let cc = one["Cc"].as_array().expect("the single-message Cc array");
    assert_eq!(cc.len(), 2, "{one}");
    assert_eq!(cc[0]["name"], "Cara C");
}

/// Finding 3's compat-shim leg (issue #31, decision 0014): with a real
/// rejecting BEFORE DELETE trigger on the data file, the shim's deletes
/// answer `503` naming the cause — never a `200` for work that did not
/// commit — and the mail survives the restart untouched.
#[tokio::test]
async fn a_blocked_delete_refuses_the_shim_deletes_with_503() {
    // The guard owns the data file and every sidecar: a failing assertion
    // leaves nothing in the temporary root (issue #143).
    let dir = tempfile::TempDir::with_prefix("swarmail-e2e-refuse-shim-").unwrap();
    let db = dir.path().join("mail.db");
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    smtp_send(
        srv.smtp_addr,
        Some("alpha"),
        "sender@x.io",
        "rcpt@x.io",
        "doomed mail",
        "alpha one",
    )
    .await
    .unwrap();
    let doomed = latest_id(srv.http_addr, "alpha").await;

    let trigger = common::refusing_trigger(&db, "delete");

    // The shim's selector delete refuses with the fixed cause.
    let payload = json!({"ids": [doomed]}).to_string();
    let (status, _, raw) = http_raw(
        srv.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    assert_eq!(status, 503, "a failed shim delete must not claim success");
    let refused: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(refused["error"], "storage commit failed", "{refused}");

    // delete-all refuses the same way.
    let (status, _, raw) = http_raw(srv.http_addr, "DELETE", "/api/v1/delete-all", None).await;
    assert_eq!(status, 503, "a failed delete-all must not claim success");
    let refused: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(refused["error"], "storage commit failed", "{refused}");

    // Committed state is preserved, live and across a restart.
    let (st, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/alpha/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        count["count"], 1,
        "a refused shim delete must preserve the mail: {count}"
    );
    srv.stop().await;
    drop(trigger);
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    let (st, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/alpha/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        count["count"], 1,
        "a refused shim delete must not lose the mail across a restart: {count}"
    );

    // Recovery: the trigger is gone, the same delete succeeds and sticks.
    let (status, _, raw) = http_raw(
        srv.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    assert_eq!(status, 200);
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 1}));
    srv.stop().await;
}

/// R10 (issue #31 finding 2, decision 0018): the shim's bulk delete runs on
/// the blocking pool, so a multi-id DELETE burning on a held writer lock
/// cannot stall the async runtime. A 10ms timer canary keeps ticking through
/// the whole burn — its budget is the busy bound's scheduling margin, far
/// above tick noise and far below the burn itself (deadline-based, like the
/// stalled-sink legs, never a latency percentile). The delete answers `503`
/// naming the cause, and every untouched mail stays committed.
///
/// The guarantees are asserted separately (issue #32 finding 3): the canary
/// carries the offload proof — a synchronous delete would block this
/// current-thread runtime and its timers with it, blowing the widest-tick
/// bound from milliseconds to never — while a plain OS-thread watchdog
/// carries the finite-hang proof, because a stalled runtime cannot fire
/// tokio's own timeouts. Neither asserts a narrow elapsed ceiling: the busy
/// bound bounds SQLite's lock retries, not the request's total latency on a
/// loaded runner, and the refusal's truth is not made truer by arriving
/// quickly.
#[tokio::test]
async fn a_burning_bulk_delete_offloads_and_keeps_the_runtime_live() {
    // The guard owns the data file and every sidecar (issue #143).
    let dir = tempfile::TempDir::with_prefix("swarmail-compat-offload-").unwrap();
    let db = dir.path().join("mail.db");
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;

    let mut ids = Vec::new();
    for subject in ["offload one", "offload two", "offload three"] {
        smtp_send(
            srv.smtp_addr,
            Some("offload"),
            "sender@x.io",
            "rcpt@x.io",
            subject,
            "body",
        )
        .await
        .unwrap();
        ids.push(latest_id(srv.http_addr, "offload").await);
    }

    // A real second connection holds the writer lock: the bulk delete will
    // burn the busy bound on the blocking pool (decision 0012 §3, at the
    // real layer) and refuse.
    let _lock = common::hold_write_lock(&db);

    // The canary: tick every 10ms and record both the widest gap between
    // ticks and the tick count. The gap bounds per-tick scheduling delay;
    // the count proves the runtime actually POLLED the canary across the
    // burn — a stalled runtime records neither a wide gap nor any ticks at
    // all, so a bare widest-gap bound passes a stall vacuously (issue #32
    // finding 3: the count is what makes a synchronous delete fail here).
    let widest = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let ticks = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let canary_widest = widest.clone();
    let canary_ticks = ticks.clone();
    let canary = tokio::spawn(async move {
        let mut last = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let now = tokio::time::Instant::now();
            canary_widest.fetch_max(
                (now - last).as_micros() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            canary_ticks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            last = now;
        }
    });

    let started = std::time::Instant::now();
    let payload = json!({ "ids": ids }).to_string();
    // The finite-hang watchdog: a plain OS thread, because the hang it
    // watches — a handler that stalled the runtime — freezes tokio's own
    // timers, so no tokio timeout could ever fire. Its 30s ceiling is six
    // busy bounds: far above anything a loaded runner's scheduling can add
    // to a refusal that burns 5s of lock retry, far below the job timeout.
    // It proves the request ANSWERS; it asserts nothing about how soon.
    let answered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watchdog_answered = answered.clone();
    let watchdog = std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if watchdog_answered.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            if started.elapsed() > std::time::Duration::from_secs(30) {
                eprintln!(
                    "the burning bulk delete never answered within 30s — the \
                 handler stalled the runtime (a hang, not a slow burn); \
                 failing loudly instead of hanging the job"
                );
                std::process::exit(101);
            }
        }
    });
    let (status, _, raw) = http_raw(
        srv.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    answered.store(true, std::sync::atomic::Ordering::Relaxed);
    watchdog.join().expect("the watchdog thread joins");

    assert_eq!(status, 503, "a burning bulk delete must not claim success");
    let refused: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(refused["error"], "storage commit failed", "{refused}");

    canary.abort();
    let elapsed = started.elapsed();
    let widest_us = widest.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        widest_us < 1_000_000,
        "the async runtime stalled {widest_us}µs between canary ticks during a burning bulk delete — the delete did not offload"
    );
    // Offload proof: the canary must have been POLLED across the burn. Its
    // 10ms cadence covers the request whenever the runtime is free, while a
    // handler that blocked the runtime leaves a handful of ticks at most —
    // the widest-gap bound alone passes such a stall vacuously, because a
    // stalled runtime records no gaps at all. The floor is a fifth of the
    // observed request: a burn stretching on a loaded runner never softens
    // the proof, and the proof never depends on a narrow elapsed ceiling.
    let ticks_us = u128::from(ticks.load(std::sync::atomic::Ordering::Relaxed)) * 10_000;
    assert!(
        ticks_us >= elapsed.as_micros() / 5,
        "the canary covered only {ticks_us}µs of a {elapsed:?} request — the runtime was not live across the burn; the delete did not offload"
    );

    // The refusal is wholesale: every untouched mail stays committed.
    let (st, count) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/offload/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        count["count"], 3,
        "a refused bulk delete must leave every mail committed: {count}"
    );
}

// ---------- the bounded shim pages (decision 0030 §3) ----------

/// Every shim page refuses over the cap with the REST envelope — `400` and
/// the one sentence naming field and range — and honors the boundary
/// itself: `250` is a page on all four surfaces, with the envelope counts
/// still coherent (`total` full, `count` the page).
#[tokio::test]
async fn the_shim_pages_refuse_over_the_cap_and_honor_the_boundary() {
    let s = start().await;
    seed(&s).await;

    // Mailpit list.
    let (st, body) = http_json(s.http_addr, "GET", "/api/v1/messages?limit=251", None).await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(
        body,
        json!({ "error": "limit must be at most 250" }),
        "{body}"
    );
    // Mailpit search.
    let (st, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?limit=251&query=alpha",
        None,
    )
    .await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(
        body,
        json!({ "error": "limit must be at most 250" }),
        "{body}"
    );
    // MailHog v2 list.
    let (st, body) = http_json(s.http_addr, "GET", "/api/v2/messages?limit=251", None).await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(
        body,
        json!({ "error": "limit must be at most 250" }),
        "{body}"
    );
    // MailHog v2 search — the cap refusal rides the JSON envelope even
    // though a bad kind stays a bare 400 (pinned above).
    let (st, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?kind=containing&query=alpha&limit=251",
        None,
    )
    .await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(
        body,
        json!({ "error": "limit must be at most 250" }),
        "{body}"
    );

    // The boundary is a page on every surface, counts coherent.
    let (st, body) = http_json(s.http_addr, "GET", "/api/v1/messages?limit=250", None).await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["total"], 4);
    assert_eq!(body["count"], 4);
    let (st, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?limit=250&query=alpha",
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    // Mailpit's search total is the mailbox (issue #55): the whole store
    // here is four mails; the two "alpha" hits ride messages_count.
    assert_eq!(body["total"], 4);
    assert_eq!(body["messages_count"], 2);
    let (st, body) = http_json(s.http_addr, "GET", "/api/v2/messages?limit=250", None).await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["total"], 4);
    assert_eq!(body["items"].as_array().map(Vec::len), Some(4));
    let (st, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?kind=containing&query=alpha&limit=250",
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["total"], 2);
}

/// The `IDs` spelling is the same selector (issue #55): upstream Mailpit's
/// Go decoder matches field names case-insensitively, so its own clients
/// send either case. The rule — a non-empty selector — is untouched, so the
/// alias deletes exactly the named mails and a wrong-typed `IDs` still
/// refuses with 0011's sentence.
#[tokio::test]
async fn the_ids_alias_deletes_and_a_wrong_typed_one_still_refuses() {
    let s = start().await;
    let (a1, a2, _, _) = seed(&s).await;

    // The alias, capital: exactly the named mail goes.
    let payload = json!({"IDs": [a1]}).to_string();
    let (status, _, raw) = http_raw(
        s.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    assert_eq!(status, 200, "{:?}", String::from_utf8_lossy(&raw));
    let removed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(removed, json!({"removed": 1}));
    let (status, _) = http_json(s.http_addr, "GET", &format!("/api/v1/messages/{a1}"), None).await;
    assert_eq!(status, 404, "the aliased delete must have removed it");
    let (status, _) = http_json(s.http_addr, "GET", &format!("/api/v1/messages/{a2}"), None).await;
    assert_eq!(status, 200, "nothing else was touched");

    // A wrong-typed `IDs` is still a refusal: the alias changes the
    // spelling, never the rule.
    for body in [
        Some(("application/json", r#"{"IDs": "alpha one"}"#)),
        Some(("application/json", r#"{"IDs": []}"#)),
    ] {
        let (status, _, raw) = http_raw(s.http_addr, "DELETE", "/api/v1/messages", body).await;
        assert_eq!(status, 400, "{body:?} must refuse");
        let refused: Value = serde_json::from_slice(&raw).unwrap();
        assert!(
            refused["error"].as_str().unwrap().contains("ids"),
            "the 400 names the contract: {refused}"
        );
    }
    assert_eq!(
        inbox_counts(&s).await,
        (1, 2),
        "the refusals deleted nothing"
    );
}

/// The envelope's RCPT TO is part of the answer (issue #55): a mail
/// delivered to `bcc@x.io` with header `To: a@x.io` is searchable by kind=to,
/// surfaces the envelope in MailHog's `To`/`Raw.To`, becomes Mailpit's `Bcc`
/// (envelope-only), and the header rows stay in `Content.Headers`.
#[tokio::test]
async fn an_envelope_only_recipient_surfaces_in_every_compat_shape() {
    let s = start().await;
    let mut conn = SmtpConn::connect(s.smtp_addr).await;
    conn.send("EHLO swarmail-bcc-test").await;
    conn.reply_multi().await;
    let plain = "\u{0}gamma\u{0}whatever";
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    assert!(conn.reply().await.starts_with("235"));
    conn.send("MAIL FROM:<sender@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    // The envelope names a recipient no header row carries.
    conn.send("RCPT TO:<bcc@x.io>").await;
    assert!(conn.reply().await.starts_with("250"));
    let data = "From: sender@x.io\r\nTo: a@x.io\r\nSubject: bcc leg\r\n\r\nthe body\r\n";
    let reply = conn.data(data).await;
    assert!(reply.starts_with("250"), "DATA refused: {reply}");
    let id = reply
        .split_whitespace()
        .last()
        .unwrap()
        .trim_end_matches('.')
        .to_string();

    // kind=to answers for the delivered recipient.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?kind=to&query=bcc@x.io",
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    // The fixture delivers exactly one mail to a fresh server, so the
    // count is exact (0010's discipline): a dropped or duplicated mail
    // must declare itself, never hide behind "at least one".
    assert_eq!(
        body["total"].as_u64().unwrap(),
        1,
        "exactly the delivered mail is searchable: {body}"
    );
    assert_eq!(
        body["items"].as_array().map(Vec::len),
        Some(1),
        "exactly one item is returned: {body}"
    );
    assert_eq!(body["items"][0]["ID"], id.as_str());

    // MailHog's `To`/`Raw.To` hold the envelope; the header row stays in
    // Content.Headers.
    let hog = &body["items"][0];
    assert_eq!(hog["To"][0]["Mailbox"], "bcc");
    assert_eq!(hog["To"][0]["Domain"], "x.io");
    assert_eq!(hog["Raw"]["To"], json!(["bcc@x.io"]));
    assert_eq!(hog["Content"]["Headers"]["To"], json!(["a@x.io"]));

    // Mailpit's `Bcc` is exactly the envelope-only recipient, and `To`
    // keeps the header row.
    let (status, body) =
        http_json(s.http_addr, "GET", &format!("/api/v1/message/{id}"), None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["Bcc"], json!([{ "address": "bcc@x.io" }]));
    assert_eq!(body["To"], json!([{ "address": "a@x.io" }]));
    assert_eq!(body["Cc"], json!([]));
}

/// `limit=0` pages the default (never an empty one) on all four compat
/// surfaces (issue #55): upstream answers its default page for an unset
/// limit, and `0` — MailHog's `getStartLimit` treats it as unset — is the
/// same. Fifty-one mails in one inbox prove the default is exactly the
/// dialect's 50, not merely non-empty. The 0030 §3 cap above 250 is pinned
/// in the_shim_pages_refuse_over_the_cap_and_honor_the_boundary.
#[tokio::test]
async fn a_zero_limit_pages_the_default_on_every_compat_surface() {
    let s = start().await;
    for n in 0..51 {
        smtp_send(
            s.smtp_addr,
            Some("zero"),
            "sender@x.io",
            "rcpt@x.io",
            "zmail",
            &format!("zbody {n}"),
        )
        .await
        .unwrap();
    }

    // Mailpit list: a 51-mail mailbox pages 50 under the default.
    let (st, body) = http_json(s.http_addr, "GET", "/api/v1/messages?limit=0", None).await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["total"], 51);
    assert_eq!(
        body["count"], 50,
        "limit=0 pages the default, not an empty page"
    );
    // Mailpit search.
    let (st, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?kind=subject&query=zmail&limit=0",
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["messages_count"], 51);
    assert_eq!(body["count"], 50);
    // MailHog v2 list.
    let (st, body) = http_json(s.http_addr, "GET", "/api/v2/messages?limit=0", None).await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["total"], 51);
    assert_eq!(body["count"], 50);
    // MailHog v2 search.
    let (st, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?kind=containing&query=zbody&limit=0",
        None,
    )
    .await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["total"], 51);
    assert_eq!(body["count"], 50);
}

/// `GET /api/v1/message/latest` is Mailpit's newest-mail route (issue #55):
/// the full message shape over the newest stored mail, and a 404 only when
/// the store is empty — never a fabricated id.
#[tokio::test]
async fn the_latest_route_serves_the_newest_mail_and_404s_an_empty_store() {
    let s = start().await;
    // The empty store refuses first: no id exists to serve.
    let (status, body) = http_json(s.http_addr, "GET", "/api/v1/message/latest", None).await;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"], "message not found");

    let (a1, a2, b1, b2) = seed(&s).await;
    let (status, body) = http_json(s.http_addr, "GET", "/api/v1/message/latest", None).await;
    assert_eq!(status, 200, "{body}");
    // Mailpit's full message shape, and the newest mail of the whole store.
    assert_eq!(body["ID"], b2.as_str());
    for field in ["Text", "HTML", "Subject", "From", "ReturnPath", "Date"] {
        assert!(body.get(field).is_some(), "the shape carries {field}");
    }
    for stale in [&a1, &a2, &b1] {
        assert_ne!(body["ID"], stale.as_str());
    }
}

/// Mailpit's search `total`/`unread` are mailbox stats, not the result
/// page's size (issue #55) — pinned against both scopes in
/// mailpit_search_supports_each_kind_and_scopes; this leg pins the
/// `to:`-prefix refusal (issue #55/0025): a named 400, never a silently
/// empty result a client would read as "nothing arrived".
#[tokio::test]
async fn a_to_prefixed_query_is_refused_by_name() {
    let s = start().await;
    seed(&s).await;
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?query=to:rcpt@x.io",
        None,
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(
        body["error"],
        "unsupported search prefix \"to:\" — Mailpit's prefix grammar is not implemented; use ?kind=to|from|subject",
        "the refusal names the prefix and the honest route: {body}"
    );
}

// ---------- run 21: #95 scoped latest ----------

/// Issue #95's reproduction, inverted: one mail in `lat-a`, then a newer one
/// in `lat-b`. Every latest route — message, raw, headers — honours
/// `?inbox=`, and the global form still answers the store's newest. Red
/// before: `latest?inbox=lat-a` answered lat-b's mail, `/raw` and
/// `/headers` were 404s.
#[tokio::test]
async fn every_latest_route_is_scoped_by_inbox() {
    let s = start().await;
    let mine = "From: a@x.io\r\nSubject: mine\r\n\r\none";
    let a = send_raw(s.smtp_addr, "lat-a", mine).await;
    let b = send_raw(
        s.smtp_addr,
        "lat-b",
        "From: b@x.io\r\nSubject: other inbox newest\r\n\r\ntwo",
    )
    .await;

    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/message/latest?inbox=lat-a",
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["ID"], a.as_str());
    assert_eq!(body["Username"], "lat-a");
    let (_, body) = http_json(s.http_addr, "GET", "/api/v1/message/latest", None).await;
    assert_eq!(body["ID"], b.as_str(), "no inbox is the whole store");

    // The raw source is byte-exact, and the headers belong to that mail.
    let (status, headers, raw) = http_raw(
        s.http_addr,
        "GET",
        "/api/v1/message/latest/raw?inbox=lat-a",
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(raw, format!("{mine}\r\n").as_bytes());
    assert_eq!(
        header_value(&headers, "content-type"),
        Some("message/rfc822")
    );
    let (status, h) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/message/latest/headers?inbox=lat-a",
        None,
    )
    .await;
    assert_eq!(status, 200, "{h}");
    assert_eq!(h["Subject"], json!(["mine"]));
    let (_, h) = http_json(s.http_addr, "GET", "/api/v1/message/latest/headers", None).await;
    assert_eq!(h["Subject"], json!(["other inbox newest"]));

    // An empty scope is a 404 even though another inbox has mail; ordinary
    // ids still resolve through the id routes.
    for path in [
        "/api/v1/message/latest?inbox=lat-empty",
        "/api/v1/message/latest/raw?inbox=lat-empty",
        "/api/v1/message/latest/headers?inbox=lat-empty",
    ] {
        let (status, body) = http_json(s.http_addr, "GET", path, None).await;
        assert_eq!(status, 404, "{path}: {body}");
        assert_eq!(body["error"], "message not found");
    }
    let (status, body) = http_json(s.http_addr, "GET", &format!("/api/v1/message/{a}"), None).await;
    assert_eq!((status, body["ID"].clone()), (200, json!(a.as_str())));
}

/// A filtered latest is refused by name rather than ignored (decision 0044
/// §4): a nonblank `query` is a 400 naming it on every latest route, a
/// blank one is unused, an unknown key and a blank inbox refuse too.
#[tokio::test]
async fn latest_refuses_a_query_an_unknown_key_and_a_blank_inbox() {
    let s = start().await;
    let id = send_raw(s.smtp_addr, "lat-q", "Subject: q\r\n\r\nbody").await;
    for route in ["latest", "latest/raw", "latest/headers"] {
        let (status, body) = http_json(
            s.http_addr,
            "GET",
            &format!("/api/v1/message/{route}?inbox=lat-q&query=subject:q"),
            None,
        )
        .await;
        assert_eq!(status, 400, "{route}: {body}");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .starts_with("query is not supported on latest"),
            "{route}: {body}"
        );
        let (status, body) = http_json(
            s.http_addr,
            "GET",
            &format!("/api/v1/message/{route}?inbox=%20"),
            None,
        )
        .await;
        assert_eq!(status, 400, "{route}: {body}");
        assert_eq!(body["error"], "inbox must not be blank");
        let (status, body) = http_json(
            s.http_addr,
            "GET",
            &format!("/api/v1/message/{route}?inbx=lat-q"),
            None,
        )
        .await;
        assert_eq!(status, 400, "{route}: {body}");
        assert!(body["error"].as_str().unwrap().contains("inbx"), "{body}");
    }
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/message/latest?inbox=lat-q&query=%20%20",
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["ID"], id.as_str());
}

// ---------- run 21: #99 envelope sender and prefix queries ----------

/// Issue #99's probe: `MAIL FROM:<bounce+77@mailer.io>` with header `From:
/// Alice <alice@x.io>`. The compat sender is the envelope on every compat
/// shape — MailHog `From`, `Raw.From`, `kind=from`, Mailpit `ReturnPath`
/// and the synthesized `Return-Path` — while the native `from` filter and
/// Mailpit's `From` stay on the header. Red before: `kind=from&query=bounce`
/// found 0 and `ReturnPath` was alice.
#[tokio::test]
async fn the_compat_sender_follows_the_envelope() {
    let s = start().await;
    let mut conn = SmtpConn::connect(s.smtp_addr).await;
    let plain = "\u{0}env\u{0}whatever";
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
    conn.send(&format!("AUTH PLAIN {b64}")).await;
    conn.reply().await;
    conn.send("MAIL FROM:<bounce+77@mailer.io>").await;
    conn.reply().await;
    conn.send("RCPT TO:<a@x.io>").await;
    conn.reply().await;
    conn.send("RCPT TO:<bcc@x.io>").await;
    conn.reply().await;
    let reply = conn
        .data("From: Alice <alice@x.io>\r\nTo: a@x.io\r\nSubject: diverge\r\n\r\nbody")
        .await;
    assert!(reply.starts_with("250"), "{reply}");

    let addr = s.http_addr;
    let search = move |query: &'static str| async move {
        let (status, body) = http_json(
            addr,
            "GET",
            &format!("/api/v2/search?inbox=env&kind=from&query={query}"),
            None,
        )
        .await;
        assert_eq!(status, 200, "{body}");
        body["total"].clone()
    };
    assert_eq!(search("bounce").await, 1);
    assert_eq!(search("alice").await, 0);
    let (_, pit) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?inbox=env&kind=from&query=bounce",
        None,
    )
    .await;
    assert_eq!(pit["messages_count"], 1, "{pit}");

    let (_, hog) = http_json(s.http_addr, "GET", "/api/v2/messages?inbox=env", None).await;
    let item = &hog["items"][0];
    assert_eq!(
        item["From"],
        json!({"Relays": null, "Mailbox": "bounce+77", "Domain": "mailer.io", "Params": ""})
    );
    assert_eq!(item["Raw"]["From"], "bounce+77@mailer.io");
    assert_eq!(
        item["Content"]["Headers"]["Return-Path"],
        json!(["<bounce+77@mailer.io>"])
    );
    assert_eq!(
        item["Content"]["Headers"]["From"],
        json!(["Alice <alice@x.io>"])
    );
    let id = item["ID"].as_str().unwrap().to_string();
    let (_, full) = http_json(s.http_addr, "GET", &format!("/api/v1/message/{id}"), None).await;
    assert_eq!(full["ReturnPath"], "bounce+77@mailer.io");
    assert_eq!(
        full["From"],
        json!({"name": "Alice", "address": "alice@x.io"})
    );

    // The native planes: `from` is the header, `to` includes the
    // envelope-only Bcc (documented, decision 0044 §8).
    let (_, native) = http_json(s.http_addr, "GET", "/api/v1/inboxes/env/messages", None).await;
    assert_eq!(native["emails"][0]["envelope_from"], "bounce+77@mailer.io");
    for (query, count) in [("from=alice", 1), ("from=bounce", 0), ("to=bcc@x.io", 1)] {
        let (status, body) = http_json(
            s.http_addr,
            "GET",
            &format!("/api/v1/inboxes/env/count?{query}"),
            None,
        )
        .await;
        assert_eq!(
            (status, body["count"].clone()),
            (200, json!(count)),
            "{query}"
        );
    }
}

/// Every field-shaped token of a containing search refuses by name on both
/// shims (decision 0044 §7) — leading spaces, mixed case, unknown fields
/// and later tokens included — while a URL and an explicit kind's value
/// stay literal. Red before: ` to:bcc@x.io`, `from:alice` and
/// `subject:bcc` answered 200 with zero matches.
#[tokio::test]
async fn field_prefixes_refuse_by_name_and_urls_stay_literal() {
    let s = start().await;
    send_raw(
        s.smtp_addr,
        "pfx",
        "From: sender@x.io\r\nSubject: see https://x.io/a\r\n\r\nbody to:literal",
    )
    .await;
    for (query, field) in [
        ("%20to:bcc@x.io", "to"),
        ("from:alice", "from"),
        ("Subject:bcc", "Subject"),
        ("plan%20foo:x", "foo"),
    ] {
        for path in [
            format!("/api/v1/search?inbox=pfx&query={query}"),
            format!("/api/v2/search?inbox=pfx&kind=containing&query={query}"),
        ] {
            let (status, body) = http_json(s.http_addr, "GET", &path, None).await;
            assert_eq!(status, 400, "{path}: {body}");
            assert!(
                body["error"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("unsupported search prefix \"{field}:\"")),
                "{path}: {body}"
            );
        }
    }
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?inbox=pfx&query=https://x.io/a",
        None,
    )
    .await;
    assert_eq!(
        (status, body["messages_count"].clone()),
        (200, json!(1)),
        "{body}"
    );
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v1/search?inbox=pfx&kind=subject&query=see%20https",
        None,
    )
    .await;
    assert_eq!(
        (status, body["messages_count"].clone()),
        (200, json!(1)),
        "{body}"
    );
}

// ---------- epic #120 batch 4: #138 wrapped prefixes, tz, one order ----------

/// Mailpit's exclusion (`-`, `!`), quoted and parenthesized forms refuse by
/// the field's name on both shims (decision 0049 §6) — a quoted field whose
/// value spans a space included — while a URL, a plain `-word`, an
/// ordinary quoted phrase and an explicit kind's value stay literal. Red at
/// 2a325c9: every refused form answered 200 with zero matches, so a ported
/// "no mail to anyone but x" assertion passed against any mailbox.
#[tokio::test]
async fn wrapped_field_prefixes_refuse_by_name() {
    let s = start().await;
    send_raw(
        s.smtp_addr,
        "wrap",
        "From: sender@x.io\r\nSubject: -minus \"quoted phrase\" -https://x.io/a\r\n\r\nbody",
    )
    .await;
    for (query, field) in [
        ("-to%3Abcc%40x.io", "to"),
        ("!from%3Aalice", "from"),
        ("%22to%3Abcc%40x.io%22", "to"),
        ("-%22subject%3Ax%22", "subject"),
        ("(to%3Ax)", "to"),
        ("%22subject%3Ahello%20world%22", "subject"),
        ("plain%20(!%22From%3Ax%22)", "From"),
    ] {
        for path in [
            format!("/api/v1/search?inbox=wrap&query={query}"),
            format!("/api/v2/search?inbox=wrap&kind=containing&query={query}"),
        ] {
            let (status, body) = http_json(s.http_addr, "GET", &path, None).await;
            assert_eq!(status, 400, "{path}: {body}");
            assert!(
                body["error"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("unsupported search prefix \"{field}:\"")),
                "{path}: {body}"
            );
        }
    }
    for (query, hits) in [
        ("-minus", 1),
        ("%22quoted%20phrase%22", 1),
        ("-https://x.io/a", 1),
        ("-nothing-here", 0),
    ] {
        let path = format!("/api/v1/search?inbox=wrap&query={query}");
        let (status, body) = http_json(s.http_addr, "GET", &path, None).await;
        assert_eq!(
            (status, body["messages_count"].clone()),
            (200, json!(hits)),
            "{path}: {body}"
        );
        let path = format!("/api/v2/search?inbox=wrap&kind=containing&query={query}");
        let (status, body) = http_json(s.http_addr, "GET", &path, None).await;
        assert_eq!(
            (status, body["total"].clone()),
            (200, json!(hits)),
            "{path}"
        );
    }
    // An explicit kind takes its value literally, wrappers and all.
    let (status, body) = http_json(
        s.http_addr,
        "GET",
        "/api/v2/search?inbox=wrap&kind=to&query=-to%3Ax",
        None,
    )
    .await;
    assert_eq!((status, body["total"].clone()), (200, json!(0)), "{body}");
}

/// Mailpit's `tz` is not read on latest (decision 0049 §5): a named 400 on
/// every latest route, which README's known differences states.
#[tokio::test]
async fn latest_timezone_is_a_documented_refusal() {
    let s = start().await;
    send_raw(s.smtp_addr, "lat-tz", "Subject: tz\r\n\r\nbody").await;
    for route in ["latest", "latest/raw", "latest/headers"] {
        let path = format!("/api/v1/message/{route}?inbox=lat-tz&tz=UTC");
        let (status, body) = http_json(s.http_addr, "GET", &path, None).await;
        assert_eq!(status, 400, "{route}: {body}");
        assert!(body["error"].as_str().unwrap().contains("tz"), "{body}");
    }
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    assert!(readme.contains("`tz`"), "README names the tz refusal");
}

/// A blank compat `inbox` selector refuses on all four pages (decision 0044
/// §3) instead of widening to the whole store.
#[tokio::test]
async fn a_blank_compat_inbox_refuses_on_every_page() {
    let s = start().await;
    for path in [
        "/api/v1/messages?inbox=",
        "/api/v1/search?inbox=%20&query=x",
        "/api/v2/messages?inbox=%09",
        "/api/v2/search?inbox=&kind=containing&query=x",
    ] {
        let (status, body) = http_json(s.http_addr, "GET", path, None).await;
        assert_eq!(status, 400, "{path}: {body}");
        assert_eq!(body["error"], "inbox must not be blank", "{path}");
    }
}

/// A control-bearing compat `inbox` selector refuses on every page and
/// every latest route with the shared sentence (issue #128, decision 0046
/// §4) — the one name rule, never a widened or renamed scope — while a
/// padded selector still scopes to exactly the padded inbox.
#[tokio::test]
async fn a_control_compat_inbox_refuses_on_every_scope() {
    let s = start().await;
    for selector in ["a%00b", "a%01b", "pad%0D", "del%7F", "c1%C2%85"] {
        for path in [
            format!("/api/v1/messages?inbox={selector}"),
            format!("/api/v1/search?inbox={selector}&query=x"),
            format!("/api/v2/messages?inbox={selector}"),
            format!("/api/v2/search?inbox={selector}&kind=containing&query=x"),
            format!("/api/v1/message/latest?inbox={selector}"),
            format!("/api/v1/message/latest/raw?inbox={selector}"),
            format!("/api/v1/message/latest/headers?inbox={selector}"),
        ] {
            let (status, body) = http_json(s.http_addr, "GET", &path, None).await;
            assert_eq!(status, 400, "{path}: {body}");
            assert_eq!(
                body["error"], "inbox must not contain control characters",
                "{path}"
            );
        }
    }
    send_raw(s.smtp_addr, "pad", "Subject: unpadded\r\n\r\nbody").await;
    let (status, body) =
        http_json(s.http_addr, "GET", "/api/v1/messages?inbox=%20pad%20", None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["total"], 0,
        "the padded scope is its own inbox: {body}"
    );
}

// ---------- run 21: #96 bounded page work ----------

/// A MailHog page whose JSON would pass 64 MiB refuses whole, by name
/// (decision 0044 §6): one 18 MiB text mail serializes its body four times
/// over (Body, both MIME fields, Raw.Data). Raw-by-id still serves it, and
/// a page of small mail is untouched. Red before: the page was built and
/// sent, hundreds of megabytes of server memory later.
#[tokio::test]
async fn an_over_budget_mailhog_page_refuses_by_name() {
    let s = start().await;
    let line = format!("{}\r\n", "a".repeat(76));
    let body = line.repeat(18 * 1024 * 1024 / line.len());
    let data = format!("From: big@x.io\r\nSubject: huge\r\n\r\n{body}");
    let id = send_raw(s.smtp_addr, "huge", &data).await;
    send_raw(s.smtp_addr, "small", "Subject: tiny\r\n\r\nsmall").await;

    for path in [
        "/api/v2/messages?inbox=huge&limit=1",
        "/api/v2/search?inbox=huge&kind=containing&query=aaaa",
    ] {
        let (status, body) = http_json(s.http_addr, "GET", path, None).await;
        assert_eq!(status, 400, "{path}");
        assert_eq!(
            body["error"],
            "compat page exceeds byte budget 67108864; reduce limit or fetch raw by id",
            "{path}"
        );
    }
    let (status, _, raw) = http_raw(
        s.http_addr,
        "GET",
        &format!("/api/v1/message/{id}/raw"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        raw.len(),
        data.len() + 2,
        "the exact source, CRLF-terminated"
    );
    let (status, body) = http_json(s.http_addr, "GET", "/api/v2/messages?inbox=small", None).await;
    assert_eq!((status, body["count"].clone()), (200, json!(1)), "{body}");
    // The Mailpit summary of the huge mail is a bounded snippet.
    let (status, body) = http_json(s.http_addr, "GET", "/api/v1/messages?inbox=huge", None).await;
    assert_eq!(status, 200);
    let snippet = body["messages"][0]["Snippet"].as_str().unwrap();
    assert_eq!(snippet.chars().count(), 203, "{snippet}");
}

/// #170: the admitted HTTP request must reach regex's named refusal,
/// rather than panic or fail HTTP parsing. The ASCII URI is 60,021 bytes (Mailpit).
#[tokio::test]
async fn too_complex_needle_refuses_on_the_wire() {
    let srv = start().await;
    let needle = "k".repeat(60_000);
    for (path, named) in [
        (format!("/api/v1/search?query={needle}"), true),
        (
            format!("/api/v2/search?kind=containing&query={needle}"),
            false,
        ),
    ] {
        let (status, _, body) = http_raw(srv.http_addr, "GET", &path, None).await;
        assert_eq!(
            status,
            400,
            "URI bytes {}: {}",
            path.len(),
            String::from_utf8_lossy(&body)
        );
        if named {
            let value: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(value["error"], "Error: the search needle is too complex");
        } else {
            assert!(body.is_empty(), "{}", String::from_utf8_lossy(&body));
        }
    }
    srv.stop().await;
}

/// Extracted arrays are stored data, not containing-search candidates.
/// Insert a valid historical-style record whose arrays are absent in raw.
#[tokio::test]
async fn extracted_only_fields_miss_both_wire_searches() {
    let srv = start().await;
    let id = send_raw(
        srv.smtp_addr,
        "only-arrays",
        "Subject: ordinarycontrol\r\n\r\nbodycontrol",
    )
    .await;
    let mut email = (*srv.store.get(&id).unwrap()).clone();
    email.links = vec!["https://array-only.invalid/linksentinel".into()];
    email.codes = vec!["9876543210".into()];
    srv.store.delete(&id).unwrap();
    srv.store.insert(email).unwrap();
    let (_, saved) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(
        saved["links"],
        json!(["https://array-only.invalid/linksentinel"])
    );
    assert_eq!(saved["codes"], json!(["9876543210"]));
    for (query, count) in [
        ("linksentinel", 0),
        ("9876543210", 0),
        ("ordinarycontrol", 1),
        ("bodycontrol", 1),
    ] {
        for path in [
            format!("/api/v1/search?query={query}&inbox=only-arrays"),
            format!("/api/v2/search?kind=containing&query={query}&inbox=only-arrays"),
        ] {
            let (status, body) = http_json(srv.http_addr, "GET", &path, None).await;
            assert_eq!(status, 200, "{path}: {body}");
            assert_eq!(body["count"], count, "{path}: {body}");
        }
    }
    srv.stop().await;
}

/// #190: a selective SQLite fault proves a nonempty committed prefix,
/// with the failing and unprocessed suffix intact live and after reopen.
#[tokio::test]
async fn shim_delete_preserves_committed_prefix_across_reopen_and_retry() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("prefix.db");
    let srv = common::start_with(|mut cfg| {
        cfg.data_file = Some(db.clone());
        cfg
    })
    .await;
    let mut ids = Vec::new();
    for subject in ["A", "B", "C"] {
        ids.push(
            send_raw(
                srv.smtp_addr,
                "prefix",
                &format!("Subject: {subject}\r\n\r\nbody"),
            )
            .await,
        );
    }
    let unrelated = send_raw(srv.smtp_addr, "unrelated", "Subject: control\r\n\r\nbody").await;
    let sql = rusqlite::Connection::open(&db).unwrap();
    sql.execute_batch(&format!("CREATE TRIGGER refuse_middle BEFORE DELETE ON emails WHEN OLD.id = '{}' BEGIN SELECT RAISE(ABORT, 'middle only'); END;", ids[1])).unwrap();
    let payload = json!({"ids": ids}).to_string();
    let (status, _, raw) = http_raw(
        srv.http_addr,
        "DELETE",
        "/api/v1/messages",
        Some(("application/json", &payload)),
    )
    .await;
    assert_eq!(status, 503, "a failed middle delete must refuse");
    assert_eq!(
        serde_json::from_slice::<Value>(&raw).unwrap()["error"],
        "storage commit failed"
    );
    for (id, want) in [
        (&ids[0], 404),
        (&ids[1], 200),
        (&ids[2], 200),
        (&unrelated, 200),
    ] {
        assert_eq!(
            http_raw(srv.http_addr, "GET", &format!("/api/v1/message/{id}"), None)
                .await
                .0,
            want,
            "live {id}"
        );
    }
    srv.stop().await;
    let srv = common::start_with(|mut cfg| {
        cfg.data_file = Some(db.clone());
        cfg
    })
    .await;
    for (id, want) in [
        (&ids[0], 404),
        (&ids[1], 200),
        (&ids[2], 200),
        (&unrelated, 200),
    ] {
        assert_eq!(
            http_raw(srv.http_addr, "GET", &format!("/api/v1/message/{id}"), None)
                .await
                .0,
            want,
            "reopen {id}"
        );
    }
    sql.execute_batch("DROP TRIGGER refuse_middle").unwrap();
    for removed in [2, 0] {
        let (status, _, raw) = http_raw(
            srv.http_addr,
            "DELETE",
            "/api/v1/messages",
            Some(("application/json", &payload)),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(
            serde_json::from_slice::<Value>(&raw).unwrap(),
            json!({"removed": removed})
        );
    }
    assert_eq!(
        http_raw(
            srv.http_addr,
            "GET",
            &format!("/api/v1/message/{unrelated}"),
            None
        )
        .await
        .0,
        200
    );
    srv.stop().await;
}

/// Every #190 R4.1 nested-quote row and R4.5 punctuation reaches fresh
/// extraction through SMTP; 250 is followed immediately by native readback.
#[tokio::test]
async fn nested_attribute_urls_and_curly_suffixes_survive_fresh_ingest() {
    let srv = start().await;
    for (html, expected) in [
        (
            r#"<td style="background-image:url('https://x.io/a.png'),url('https://x.io/b.png')">x</td><a href="https://x.io/verify?t=1">Verify</a>"#,
            vec![
                "https://x.io/a.png",
                "https://x.io/b.png",
                "https://x.io/verify?t=1",
            ],
        ),
        (
            r#"<td style="background-image:url('https://cdn.x.io/hero.jpg')">"#,
            vec!["https://cdn.x.io/hero.jpg"],
        ),
        (
            r#"<td style="background-image:url(&quot;https://cdn.x.io/hero.jpg&quot;)">"#,
            vec!["https://cdn.x.io/hero.jpg"],
        ),
        (
            r#"<a onclick="location.href='https://x.io/go'">"#,
            vec!["https://x.io/go"],
        ),
        (
            r#"<meta http-equiv="refresh" content="0;url='https://x.io/r'">"#,
            vec!["https://x.io/r"],
        ),
        (
            r#"<span title="Visit 'https://x.io/t' now">"#,
            vec!["https://x.io/t"],
        ),
        (
            r#"<td background="https://x.io/hero.jpg" style="background:url('https://x.io/hero.jpg')">"#,
            vec!["https://x.io/hero.jpg"],
        ),
        (
            r#"<a href="  https://x.io/?q=&quot;x&quot;">"#,
            vec!["https://x.io/?q=\"x\""],
        ),
        (
            "<p>“https://x.io/q”. ‘https://x.io/r’.</p>",
            vec!["https://x.io/q", "https://x.io/r"],
        ),
    ] {
        let id = send_raw(
            srv.smtp_addr,
            "nested-links",
            &format!("Content-Type: text/html; charset=utf-8\r\n\r\n{html}"),
        )
        .await;
        let (status, email) = http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}"),
            None,
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(email["links"], json!(expected), "{html}: {email}");
    }
    let id = send_raw(
        srv.smtp_addr,
        "nested-links",
        "Content-Type: text/plain; charset=utf-8\r\n\r\n“https://x.io/q”. ‘https://x.io/r’.",
    )
    .await;
    let (_, email) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(email["links"], json!(["https://x.io/q", "https://x.io/r"]));
    srv.stop().await;
}
