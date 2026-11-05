//! The MailSlurper service-API shim (decision 0118, issue #314): real SMTP
//! into a real server, real HTTP/1.1 to the opt-in listener. Every route is
//! pinned on the wire; the requests the consumer (Alkemio's tests-lib) sends
//! are replayed with their exact headers.

mod common;

use common::SmtpConn;
use serde_json::{Value, json};
use std::net::SocketAddr;
use swarmail::RunningServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

type Headers = Vec<(String, String)>;

/// A server with the MailSlurper listener on an ephemeral loopback port.
async fn start() -> RunningServer {
    common::start_with(|mut c| {
        c.mailslurper_listen = Some("127.0.0.1:0".into());
        c
    })
    .await
}

fn slurper(s: &RunningServer) -> SocketAddr {
    s.mailslurper_addr.expect("the listener is enabled")
}

/// Run one wire step under the house helpers' deadline; a wedged listener
/// fails the test with the step's name instead of hanging the CI slot
/// (issue #102). No test depends on the deadline for validity.
async fn within<T>(step: &str, fut: impl std::future::Future<Output = T>) -> T {
    match tokio::time::timeout(common::HELPER_DEADLINE, fut).await {
        Ok(out) => out,
        Err(_) => panic!("{step}: no progress within {:?}", common::HELPER_DEADLINE),
    }
}

/// One HTTP/1.1 exchange, headers sent exactly as given (a `Host` is added
/// only when the caller names none). The body, when present, rides
/// `Content-Length`.
async fn req(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> (u16, Headers, Vec<u8>) {
    let mut head = format!("{method} {path} HTTP/1.1\r\n");
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        head.push_str(&format!("Host: {addr}\r\n"));
    }
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(body) = body {
        head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    head.push_str("Connection: close\r\n\r\n");
    head.push_str(body.unwrap_or(""));
    let step = |what: &str| format!("{method} {path}: {what} at {addr}");
    let mut stream = within(&step("connect"), TcpStream::connect(addr))
        .await
        .unwrap();
    within(&step("write"), stream.write_all(head.as_bytes()))
        .await
        .unwrap();
    let mut buf = Vec::new();
    within(&step("read to EOF"), stream.read_to_end(&mut buf))
        .await
        .unwrap();
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no header/body split");
    let head = String::from_utf8_lossy(&buf[..split]).to_string();
    let mut lines = head.lines();
    let status = lines
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

fn header<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// `GET path` with no special headers.
async fn get(addr: SocketAddr, path: &str) -> (u16, Headers, Vec<u8>) {
    req(addr, "GET", path, &[], None).await
}

async fn get_json(addr: SocketAddr, path: &str) -> Value {
    let (status, _, body) = get(addr, path).await;
    assert_eq!(
        status,
        200,
        "GET {path}: {}",
        String::from_utf8_lossy(&body)
    );
    serde_json::from_slice(&body).unwrap()
}

/// The recorded `DELETE` of the consumer.
async fn prune(addr: SocketAddr, body: &str) -> (u16, Headers, Vec<u8>) {
    req(
        addr,
        "DELETE",
        "/mail",
        &[("Content-Type", "application/json")],
        Some(body),
    )
    .await
}

/// One mail through the real SMTP listener; returns the id the `250` names.
/// `inbox` AUTHs into an inbox, `None` is unauthenticated.
async fn send_mail(
    smtp: SocketAddr,
    inbox: Option<&str>,
    from: &str,
    rcpts: &[&str],
    data: &str,
) -> String {
    let mut conn = SmtpConn::connect(smtp).await;
    if let Some(inbox) = inbox {
        let plain = format!("\u{0}{inbox}\u{0}whatever");
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
        conn.send(&format!("AUTH PLAIN {b64}")).await;
        let reply = conn.reply().await;
        assert!(reply.starts_with("235"), "AUTH refused: {reply}");
    }
    conn.send(&format!("MAIL FROM:<{from}>")).await;
    assert!(conn.reply().await.starts_with("250"));
    for rcpt in rcpts {
        conn.send(&format!("RCPT TO:<{rcpt}>")).await;
        assert!(conn.reply().await.starts_with("250"));
    }
    let reply = conn.data(data).await;
    assert!(reply.starts_with("250"), "DATA refused: {reply}");
    reply
        .split_whitespace()
        .last()
        .unwrap()
        .trim_end_matches('.')
        .to_string()
}

/// A plain-text mail with a subject, through `send_mail`.
async fn send_text(smtp: SocketAddr, inbox: Option<&str>, subject: &str, body: &str) -> String {
    let data = format!("From: s@x.io\r\nTo: r@x.io\r\nSubject: {subject}\r\n\r\n{body}\r\n");
    send_mail(smtp, inbox, "s@x.io", &["r@x.io"], &data).await
}

/// The exact count over every inbox, through the native API.
async fn native_total(s: &RunningServer) -> u64 {
    let (status, body) = common::http_json(s.http_addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(status, 200, "{body}");
    body.as_array()
        .unwrap()
        .iter()
        .map(|inbox| inbox["count"].as_u64().unwrap())
        .sum()
}

/// A native read of one stored mail.
async fn native(s: &RunningServer, id: &str) -> Value {
    let (status, body) =
        common::http_json(s.http_addr, "GET", &format!("/api/v1/messages/{id}"), None).await;
    assert_eq!(status, 200, "{body}");
    body
}

fn err(body: &[u8]) -> String {
    let v: Value = serde_json::from_slice(body).unwrap();
    v["error"].as_str().unwrap().to_string()
}

const EMPTY: &str = r#"{"totalPages":0,"totalRecords":0}"#;

/// AC1: the consumer's two recorded calls, unchanged, find the mail and
/// delete it. The verification link sits in an `href`-only anchor of an
/// HTML-only mail; the regex is the kind the client scans `body` with.
#[tokio::test]
async fn the_alkemio_client_recording_finds_and_deletes_mail() {
    let s = start().await;
    let html = "From: noreply@app.test\r\nTo: new.user@x.io\r\nSubject: Verify your email\r\n\
                MIME-Version: 1.0\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                <p>Welcome!</p><a href=\"https://app.test/verify?token=abc123\"><img src=\"https://app.test/b.png\"></a>\r\n";
    send_mail(
        s.smtp_addr,
        Some("alpha"),
        "noreply@app.test",
        &["new.user@x.io"],
        html,
    )
    .await;
    send_text(s.smtp_addr, None, "Second", "no links here").await;

    let (status, headers, body) = req(
        slurper(&s),
        "GET",
        "/mail",
        &[
            ("Accept", "application/json"),
            ("Accept-Encoding", "gzip, deflate, br"),
        ],
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(header(&headers, "content-type"), Some("application/json"));
    assert_eq!(header(&headers, "content-encoding"), None, "identity only");
    let list: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list["totalRecords"], 2);
    let items = list["mailItems"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["subject"], "Second", "newest first");
    let verify = &items[1];
    assert_eq!(verify["subject"], "Verify your email");
    let link = regex::Regex::new(r#"https?://[^\s"'<>]*verify[^\s"'<>]*"#).unwrap();
    assert_eq!(
        link.find(verify["body"].as_str().unwrap())
            .unwrap()
            .as_str(),
        "https://app.test/verify?token=abc123"
    );
    let to: Vec<&str> = verify["toAddresses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert!(to.contains(&"new.user@x.io"), "{to:?}");

    let (status, headers, body) = prune(slurper(&s), r#"{"pruneCode":"all"}"#).await;
    assert_eq!(status, 200);
    assert_eq!(
        header(&headers, "content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(body, b"2");
    let (status, _, body) = get(slurper(&s), "/mail").await;
    assert_eq!((status, body.as_slice()), (200, EMPTY.as_bytes()));
    assert_eq!(native_total(&s).await, 0, "both inboxes are gone");
}

/// AC2: with the listener unset there is no address, and the HTTP plane
/// serves none of the routes; a wipe attempt there deletes nothing.
#[tokio::test]
async fn off_by_default_and_absent_from_the_http_plane() {
    let s = common::start().await;
    assert_eq!(s.mailslurper_addr, None);
    send_text(s.smtp_addr, Some("alpha"), "kept", "x").await;
    send_text(s.smtp_addr, None, "kept too", "x").await;
    let before = native_total(&s).await;
    assert_eq!(before, 2);
    for (method, path, body) in [
        ("GET", "/mail", None),
        ("GET", "/mailcount", None),
        ("GET", "/mail/anything", None),
        ("GET", "/mail/anything/message", None),
        ("DELETE", "/mail", Some(r#"{"pruneCode":"all"}"#)),
    ] {
        let (status, _, _) = req(
            s.http_addr,
            method,
            path,
            &[("Content-Type", "application/json")],
            body,
        )
        .await;
        assert_eq!(status, 404, "{method} {path} on the HTTP plane");
    }
    assert_eq!(native_total(&s).await, before, "nothing was deleted");
}

/// The listener serves the shim's routes and nothing else: the HTTP
/// plane's own routes are absent, the unserved MailSlurper routes are plain
/// 404s, and a wrong method is a 405.
#[tokio::test]
async fn the_listener_serves_only_the_shim_routes() {
    let s = start().await;
    let id = send_text(s.smtp_addr, None, "one", "x").await;
    for path in [
        "/",
        "/healthz",
        "/api/v1/inboxes",
        "/metrics",
        "/pruneoptions",
        "/version",
        "/mailcount/extra",
        &format!("/mail/{id}/messageraw"),
        &format!("/mail/{id}/attachment/1"),
    ] {
        let (status, _, _) = get(slurper(&s), path).await;
        assert_eq!(status, 404, "GET {path}");
    }
    let (status, _, _) = req(slurper(&s), "POST", "/mail", &[], None).await;
    assert_eq!(status, 405);
    let (status, _, _) = req(slurper(&s), "DELETE", "/mailcount", &[], None).await;
    assert_eq!(status, 405);
}

/// AC4: the loopback plane's guard, keyed to the MailSlurper listener's own
/// port. A refused wipe deletes nothing.
#[tokio::test]
async fn the_loopback_guard_refuses_foreign_hosts_and_origins() {
    let s = start().await;
    send_text(s.smtp_addr, None, "kept", "x").await;
    let addr = slurper(&s);
    let port = addr.port();
    let other_port = format!("127.0.0.1:{}", port.wrapping_add(1));
    let own = format!("127.0.0.1:{port}");
    let wipe = r#"{"pruneCode":"all"}"#;
    let foreign_port = format!("evil.example:{port}");
    let cases: [(&str, &str); 4] = [
        ("Host", &other_port),
        ("Host", "evil.example"),
        ("Host", &foreign_port),
        ("Origin", "https://evil.example"),
    ];
    for (name, value) in cases {
        let headers = [("Content-Type", "application/json"), (name, value)];
        let (status, _, _) = req(addr, "DELETE", "/mail", &headers, Some(wipe)).await;
        assert_eq!(status, 403, "{name}: {value}");
        let (status, _, _) = req(addr, "GET", "/mail", &headers, None).await;
        assert_eq!(status, 403, "GET with {name}: {value}");
    }
    // The HTTP plane's port is not this listener's vocabulary.
    let http_host = s.http_addr.to_string();
    let (status, _, _) = req(addr, "GET", "/mail", &[("Host", &http_host)], None).await;
    assert_eq!(status, 403, "the HTTP port is a foreign Host here");
    assert_eq!(
        native_total(&s).await,
        1,
        "no refused wipe deleted anything"
    );

    // Loopback spellings of its own port, and an own-origin page, pass.
    let localhost = format!("localhost:{port}");
    for host in [own.as_str(), localhost.as_str()] {
        let (status, _, _) = req(addr, "GET", "/mail", &[("Host", host)], None).await;
        assert_eq!(status, 200, "Host {host}");
    }
    let origin = format!("http://127.0.0.1:{port}");
    let (status, _, _) = req(addr, "GET", "/mail", &[("Origin", &origin)], None).await;
    assert_eq!(status, 200);
}

/// Off loopback the Host allowlist is inactive but a present `Origin` must
/// match the request's `Host` (decision 0043), as on the HTTP plane.
#[tokio::test]
async fn an_off_loopback_listener_checks_origin_against_host() {
    let s = common::start_with(|mut c| {
        c.mailslurper_listen = Some("0.0.0.0:0".into());
        c
    })
    .await;
    let port = slurper(&s).port();
    let dial: SocketAddr = ([127, 0, 0, 1], port).into();
    let (status, _, _) = req(dial, "GET", "/mail", &[("Host", "mail.example")], None).await;
    assert_eq!(status, 200, "any Host is the operator's trust statement");
    let (status, _, _) = req(
        dial,
        "GET",
        "/mail",
        &[("Host", "mail.example"), ("Origin", "https://evil.example")],
        None,
    )
    .await;
    assert_eq!(status, 403);
    let (status, _, _) = req(
        dial,
        "GET",
        "/mail",
        &[("Host", "mail.example"), ("Origin", "http://mail.example")],
        None,
    )
    .await;
    assert_eq!(status, 200);
}

/// AC5: `GET /mail/{id}` answers exactly the eight keys, in order, each
/// pinned against what the native API stored — compared as exact bytes.
#[tokio::test]
async fn the_item_shape_is_pinned_field_by_field() {
    let s = start().await;
    let multipart = "From: Alice <alice@header.test>\r\nTo: Bob <bob@x.io>\r\n\
        Cc: Carol <carol@x.io>\r\nSubject: Welcome <b>aboard</b>\r\nMIME-Version: 1.0\r\n\
        Content-Type: multipart/alternative; boundary=\"b1\"\r\n\r\n\
        --b1\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nplain version\r\n\
        --b1\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
        <p>html &amp; \"quoted\" <a href=\"https://x.io/go\">go</a></p>\r\n--b1--\r\n";
    // dave@x.io is a Bcc: an envelope recipient absent from every header.
    let both = send_mail(
        s.smtp_addr,
        Some("alpha"),
        "bounce@envelope.test",
        &["bob@x.io", "carol@x.io", "dave@x.io"],
        multipart,
    )
    .await;
    let text_only = send_text(s.smtp_addr, None, "Plain one", "just text").await;
    let bodyless = send_mail(
        s.smtp_addr,
        Some("beta"),
        "s@x.io",
        &["r@x.io"],
        // The blank line after the headers is the one `data` adds.
        "From: s@x.io\r\nTo: r@x.io\r\nSubject: Empty\r\n",
    )
    .await;

    let expected = |id: &str,
                    stored: &Value,
                    from: &str,
                    to: &[&str],
                    cc: &[&str],
                    subject: &str,
                    body: &str| {
        let date = chrono::DateTime::from_timestamp_millis(stored["received_ms"].as_i64().unwrap())
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let j = |v: Value| serde_json::to_string(&v).unwrap();
        format!(
            r#"{{"id":{},"dateSent":{},"fromAddress":{},"toAddresses":{},"ccAddresses":{},"subject":{},"body":{},"attachments":[]}}"#,
            j(json!(id)),
            j(json!(date)),
            j(json!(from)),
            j(json!(to)),
            j(json!(cc)),
            j(json!(subject)),
            j(json!(body)),
        )
    };

    let stored = native(&s, &both).await;
    let html = stored["html"].as_str().unwrap();
    assert!(
        html.contains("<a href=\"https://x.io/go\">go</a>"),
        "{html}"
    );
    let (status, headers, body) = get(slurper(&s), &format!("/mail/{both}")).await;
    assert_eq!(status, 200);
    assert_eq!(header(&headers, "content-type"), Some("application/json"));
    assert_eq!(
        String::from_utf8(body).unwrap(),
        expected(
            &both,
            &stored,
            "bounce@envelope.test",
            &["bob@x.io", "carol@x.io", "dave@x.io"],
            &["carol@x.io"],
            "Welcome &lt;b&gt;aboard&lt;/b&gt;",
            html
        ),
        "HTML wins over text; the envelope sender and recipients; header Cc"
    );

    let stored = native(&s, &text_only).await;
    let (_, _, body) = get(slurper(&s), &format!("/mail/{text_only}")).await;
    assert_eq!(
        String::from_utf8(body).unwrap(),
        expected(
            &text_only,
            &stored,
            "s@x.io",
            &["r@x.io"],
            &[],
            "Plain one",
            stored["text"].as_str().unwrap()
        ),
        "a text-only mail's body is its text"
    );

    let stored = native(&s, &bodyless).await;
    let (_, _, body) = get(slurper(&s), &format!("/mail/{bodyless}")).await;
    assert_eq!(
        String::from_utf8(body).unwrap(),
        expected(&bodyless, &stored, "s@x.io", &["r@x.io"], &[], "Empty", ""),
        "a bodyless mail's body is empty"
    );

    let (status, _, body) = get(slurper(&s), "/mail/no-such-id").await;
    assert_eq!(status, 404);
    assert_eq!(err(&body), "message not found");
}

/// Issue #326 (0118 §4, dated addendum 2026-11-04, accepted 2026-11-05): the MailSlurper
/// listener serves `subject` HTML-escaped with Go's `html.EscapeString` set,
/// on both item routes, and nothing else changes. One mail over real SMTP;
/// the listener is read at once after the `250`. The body is the stored
/// bytes on every route; the native API, Mailpit and MailHog serve the raw
/// subject, and Mailpit's `kind=subject` search matches the raw text and
/// not the escaped spelling. Only `"` is observed; the rest are inferred.
#[tokio::test]
async fn the_subject_is_escaped_on_both_item_routes_and_raw_everywhere_else() {
    let s = start().await;
    let raw = "New Form response to \"f\" & <b>'x'</b>";
    let escaped = "New Form response to &#34;f&#34; &amp; &lt;b&gt;&#39;x&#39;&lt;/b&gt;";
    let body = "\"q\" & <i>'y'</i>";
    let id = send_text(s.smtp_addr, None, raw, body).await;
    let stored = native(&s, &id).await;
    let stored_body = stored["text"].as_str().unwrap();
    assert!(stored_body.starts_with(body), "{stored_body:?}");

    let list = get_json(slurper(&s), "/mail").await;
    assert_eq!(list["mailItems"][0]["id"], id.as_str());
    assert_eq!(list["mailItems"][0]["subject"], escaped, "GET /mail");
    assert_eq!(list["mailItems"][0]["body"], stored_body);
    let item = get_json(slurper(&s), &format!("/mail/{id}")).await;
    assert_eq!(item["subject"], escaped, "GET /mail/{{id}}");
    assert_eq!(item["body"], stored_body, "the body is never escaped");
    let (status, _, message) = get(slurper(&s), &format!("/mail/{id}/message")).await;
    assert_eq!(status, 200);
    assert_eq!(String::from_utf8(message).unwrap(), stored_body);

    assert_eq!(stored["subject"], raw, "native API");
    let (status, pm) =
        common::http_json(s.http_addr, "GET", &format!("/api/v1/message/{id}"), None).await;
    assert_eq!(status, 200);
    assert_eq!(pm["Subject"], raw, "Mailpit");
    let (status, hog) = common::http_json(s.http_addr, "GET", "/api/v2/messages", None).await;
    assert_eq!(status, 200);
    assert_eq!(
        hog["items"][0]["Content"]["Headers"]["Subject"][0], raw,
        "MailHog"
    );

    let search = |fragment: &str| {
        let path = format!("/api/v1/search?kind=subject&query={fragment}");
        async move {
            let (status, found) = common::http_json(s.http_addr, "GET", &path, None).await;
            assert_eq!(status, 200, "{found}");
            found["messages_count"].as_u64().unwrap()
        }
    };
    assert_eq!(search("to%20%22f%22").await, 1, "the raw fragment matches");
    assert_eq!(search("%26%2334%3Bf").await, 0, "the escaped one does not");
    s.stop().await;
}

/// AC6: the message route is the item's `body` as `text/html`, hardened for
/// a browser (the HTML is the sender's), and `404` for an unknown id.
#[tokio::test]
async fn one_mail_and_its_message_by_id() {
    let s = start().await;
    let html = "From: s@x.io\r\nTo: r@x.io\r\nSubject: H\r\nMIME-Version: 1.0\r\n\
                Content-Type: text/html\r\n\r\n<h1>Hi</h1><script>alert(1)</script>\r\n";
    let id = send_mail(s.smtp_addr, None, "s@x.io", &["r@x.io"], html).await;
    let item = get_json(slurper(&s), &format!("/mail/{id}")).await;
    let (status, headers, body) = get(slurper(&s), &format!("/mail/{id}/message")).await;
    assert_eq!(status, 200);
    assert_eq!(
        header(&headers, "content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(
        header(&headers, "content-security-policy"),
        Some("sandbox"),
        "sender HTML runs no script and holds no origin"
    );
    assert_eq!(header(&headers, "x-content-type-options"), Some("nosniff"));
    assert_eq!(
        String::from_utf8(body).unwrap(),
        item["body"].as_str().unwrap()
    );
    assert!(item["body"].as_str().unwrap().contains("<h1>Hi</h1>"));

    let (status, _, body) = get(slurper(&s), "/mail/no-such-id/message").await;
    assert_eq!(status, 404);
    assert_eq!(err(&body), "message not found");
}

/// AC7: `{"mailCount":n}` is the exact sum over every inbox.
#[tokio::test]
async fn mailcount_sums_every_inbox() {
    let s = start().await;
    let (status, headers, body) = get(slurper(&s), "/mailcount").await;
    assert_eq!(status, 200);
    assert_eq!(header(&headers, "content-type"), Some("application/json"));
    assert_eq!(body, br#"{"mailCount":0}"#);
    for (inbox, n) in [(Some("a"), 2), (Some("b"), 3), (None, 1)] {
        for i in 0..n {
            send_text(s.smtp_addr, inbox, &format!("m{i}"), "x").await;
        }
    }
    let (_, _, body) = get(slurper(&s), "/mailcount").await;
    assert_eq!(body, br#"{"mailCount":6}"#);
}

/// AC8: upstream pages at 50. 51 mails over two inboxes: the consumer's
/// `pageNumber`-less request holds the 50 newest, page 2 the oldest one,
/// both carry the snapshot's totals, and the pages together are the 51 sent
/// ids with none repeated.
#[tokio::test]
async fn pages_cut_at_fifty_newest_first_and_counted() {
    let s = start().await;
    let mut sent = Vec::new();
    for i in 0..51 {
        let inbox = if i % 2 == 0 { "even" } else { "odd" };
        sent.push(send_text(s.smtp_addr, Some(inbox), &format!("m{i:02}"), "x").await);
    }
    let ids = |page: &Value| -> Vec<String> {
        page["mailItems"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_string())
            .collect()
    };
    // The literal 50 is upstream's page size, asserted here and nowhere
    // else, so the test pins upstream and not the shim's own constant.
    let one = get_json(slurper(&s), "/mail").await;
    assert_eq!(one["mailItems"].as_array().unwrap().len(), 50);
    let newest_first: Vec<String> = sent.iter().rev().cloned().collect();
    assert_eq!(ids(&one), newest_first[..50], "the 50 newest, both inboxes");
    assert_eq!(get_json(slurper(&s), "/mail?pageNumber=1").await, one);

    let two = get_json(slurper(&s), "/mail?pageNumber=2").await;
    assert_eq!(ids(&two), newest_first[50..], "the oldest one");
    for page in [&one, &two] {
        assert_eq!(
            (page["totalPages"].clone(), page["totalRecords"].clone()),
            (json!(2), json!(51))
        );
    }
    let mut union: Vec<String> = ids(&one).into_iter().chain(ids(&two)).collect();
    union.sort();
    let mut expected = sent.clone();
    expected.sort();
    assert_eq!(union, expected, "every sent id once, none repeated");

    // Past the end: the totals, no items key.
    for path in [
        "/mail?pageNumber=3",
        "/mail?pageNumber=18446744073709551615",
    ] {
        let (status, _, body) = get(slurper(&s), path).await;
        assert_eq!(status, 200, "{path}");
        assert_eq!(body, br#"{"totalPages":2,"totalRecords":51}"#, "{path}");
    }
    s.stop().await;
}

#[tokio::test]
async fn invalid_page_numbers_refuse_by_name() {
    let s = start().await;
    send_text(s.smtp_addr, None, "kept", "x").await;
    for value in ["0", "-1", "x", "", "1.5", "+", "99999999999999999999999999"] {
        let (status, _, body) = get(slurper(&s), &format!("/mail?pageNumber={value}")).await;
        assert_eq!(status, 400, "pageNumber={value:?}");
        assert!(
            err(&body).contains("pageNumber"),
            "{value:?}: {}",
            err(&body)
        );
    }
}

#[tokio::test]
async fn an_empty_store_omits_mail_items() {
    let s = start().await;
    let (status, headers, body) = get(slurper(&s), "/mail").await;
    assert_eq!(status, 200);
    assert_eq!(header(&headers, "content-type"), Some("application/json"));
    assert_eq!(body, EMPTY.as_bytes());
}

/// AC9: the query is closed everywhere; no filter is silently ignored.
#[tokio::test]
async fn every_unsupported_query_key_refuses_by_name() {
    let s = start().await;
    let id = send_text(s.smtp_addr, None, "kept", "x").await;
    for key in [
        "message", "from", "to", "start", "end", "orderby", "dir", "zzz",
    ] {
        let (status, _, body) = get(slurper(&s), &format!("/mail?{key}=x")).await;
        assert_eq!(status, 400, "?{key}");
        assert!(err(&body).contains(key), "?{key}: {}", err(&body));
        // A valid pageNumber beside it does not save it.
        let (status, _, _) = get(slurper(&s), &format!("/mail?pageNumber=1&{key}=x")).await;
        assert_eq!(status, 400, "?pageNumber=1&{key}");
    }
    for path in [
        "/mailcount?x=1".to_string(),
        format!("/mail/{id}?x=1"),
        format!("/mail/{id}/message?x=1"),
    ] {
        let (status, _, body) = get(slurper(&s), &path).await;
        assert_eq!(status, 400, "{path}");
        assert!(err(&body).contains('x'), "{path}: {}", err(&body));
    }
    let (status, _, body) = req(
        slurper(&s),
        "DELETE",
        "/mail?x=1",
        &[("Content-Type", "application/json")],
        Some(r#"{"pruneCode":"all"}"#),
    )
    .await;
    assert_eq!(status, 400);
    assert!(err(&body).contains('x'));
    assert_eq!(
        native_total(&s).await,
        1,
        "the query refusal deleted nothing"
    );
}

/// AC10: every refusal is a 400 that deletes nothing, counted before and
/// after over both inboxes; the age codes are named in their reason.
#[tokio::test]
async fn every_destructive_refusal_deletes_nothing() {
    let s = start().await;
    send_text(s.smtp_addr, Some("alpha"), "one", "x").await;
    send_text(s.smtp_addr, None, "two", "x").await;
    let before = native_total(&s).await;
    assert_eq!(before, 2);
    let json_ct = [("Content-Type", "application/json")];
    let bodies: [Option<&str>; 13] = [
        None,
        Some(""),
        Some("{}"),
        Some(r#"{"pruneCode":null}"#),
        Some(r#"{"pruneCode":7}"#),
        Some(r#"{"pruneCode":"ALL"}"#),
        Some(r#"{"pruneCode":"forever"}"#),
        Some(r#"{"pruneCode":"all","x":1}"#),
        Some("not json"),
        Some(r#"["all"]"#),
        Some(r#""all""#),
        Some(r#"[{"pruneCode":"all"}]"#),
        Some(r#"{"pruneCode":["all"]}"#),
    ];
    for body in bodies {
        let (status, _, answer) = req(slurper(&s), "DELETE", "/mail", &json_ct, body).await;
        assert_eq!(status, 400, "body {body:?}");
        assert!(!err(&answer).is_empty());
        assert_eq!(native_total(&s).await, before, "body {body:?} deleted");
    }
    // Wrong or absent content type: the same literal body does not wipe.
    for headers in [&[("Content-Type", "text/plain")][..], &[][..]] {
        let (status, _, _) = req(
            slurper(&s),
            "DELETE",
            "/mail",
            headers,
            Some(r#"{"pruneCode":"all"}"#),
        )
        .await;
        assert_eq!(status, 400, "{headers:?}");
        assert_eq!(native_total(&s).await, before);
    }
    for code in ["60plus", "30plus", "2wksplus"] {
        let (status, _, answer) = prune(slurper(&s), &format!(r#"{{"pruneCode":"{code}"}}"#)).await;
        assert_eq!(status, 400, "{code}");
        let reason = err(&answer);
        assert!(reason.contains(code), "{reason}");
        assert!(reason.contains("0003"), "{reason}");
        assert_eq!(native_total(&s).await, before, "{code} deleted");
    }
    let (_, _, answer) = prune(slurper(&s), r#"{"pruneCode":"forever"}"#).await;
    assert!(err(&answer).contains("forever"));
    let (_, _, answer) = prune(slurper(&s), r#"{"pruneCode":7}"#).await;
    assert!(err(&answer).contains("pruneCode"));

    // The literal still wipes, with the count removed.
    let (status, _, answer) = prune(slurper(&s), r#"{"pruneCode":"all"}"#).await;
    assert_eq!((status, answer.as_slice()), (200, &b"2"[..]));
    assert_eq!(native_total(&s).await, 0);
    // A wipe of nothing is still a 200 with 0.
    let (status, _, answer) = prune(slurper(&s), r#"{"pruneCode":"all"}"#).await;
    assert_eq!((status, answer.as_slice()), (200, &b"0"[..]));
}

/// AC11: a failed commit during the wipe is a 503, never a count, and the
/// mail survives.
#[tokio::test]
async fn a_failed_wipe_commit_answers_503() {
    let dir = tempfile::TempDir::with_prefix("swarmail-e2e-slurper-503-").unwrap();
    let db = dir.path().join("mail.db");
    let s = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c.mailslurper_listen = Some("127.0.0.1:0".into());
        c
    })
    .await;
    send_text(s.smtp_addr, Some("alpha"), "doomed", "x").await;
    let trigger = common::refusing_trigger(&db, "delete");
    let (status, _, body) = prune(slurper(&s), r#"{"pruneCode":"all"}"#).await;
    assert_eq!(status, 503);
    assert_eq!(err(&body), "storage commit failed");
    assert_eq!(native_total(&s).await, 1, "the refused wipe kept the mail");
    trigger.remove();
    let (status, _, body) = prune(slurper(&s), r#"{"pruneCode":"all"}"#).await;
    assert_eq!(
        (status, body.as_slice()),
        (200, &b"1"[..]),
        "the retry commits"
    );
    s.stop().await;
}

/// AC13: a mail is listed the moment its `250` is read (decision 0001).
#[tokio::test]
async fn every_250_is_listed_at_once() {
    let s = start().await;
    for i in 0..5 {
        let subject = format!("fresh {i}");
        send_text(s.smtp_addr, None, &subject, "x").await;
        let list = get_json(slurper(&s), "/mail").await;
        assert_eq!(list["totalRecords"], i + 1);
        assert_eq!(list["mailItems"][0]["subject"], subject.as_str());
        let count = get_json(slurper(&s), "/mailcount").await;
        assert_eq!(count["mailCount"], i + 1);
    }
}

/// AC14: an id survives a restart on the same data file, byte for byte,
/// and a wipe survives the next restart.
#[tokio::test]
async fn an_id_survives_a_data_file_restart() {
    let dir = tempfile::TempDir::with_prefix("swarmail-e2e-slurper-restart-").unwrap();
    let db = dir.path().join("mail.db");
    let with_file = |db: std::path::PathBuf| {
        move |mut c: swarmail::config::Config| {
            c.data_file = Some(db);
            c.mailslurper_listen = Some("127.0.0.1:0".into());
            c
        }
    };
    let s = common::start_with(with_file(db.clone())).await;
    let id = send_text(s.smtp_addr, Some("alpha"), "persisted", "body text").await;
    let (_, _, first) = get(slurper(&s), &format!("/mail/{id}")).await;
    s.stop().await;

    let s = common::start_with(with_file(db.clone())).await;
    let (status, _, second) = get(slurper(&s), &format!("/mail/{id}")).await;
    assert_eq!(status, 200);
    assert_eq!(second, first, "the same item after the restart");

    // A wipe persists too: a nightly run never scans a stale link.
    let (status, _, body) = prune(slurper(&s), r#"{"pruneCode":"all"}"#).await;
    assert_eq!((status, body.as_slice()), (200, &b"1"[..]));
    s.stop().await;

    let s = common::start_with(with_file(db)).await;
    let (status, _, body) = get(slurper(&s), "/mail").await;
    assert_eq!((status, body.as_slice()), (200, EMPTY.as_bytes()));
    assert_eq!(native_total(&s).await, 0, "both stores empty natively");
    s.stop().await;
}

/// AC15: the guide states the global view, the flag, the cluster recipe,
/// the refusals by name and each known difference of decision 0118 §6;
/// the quickstart and the network posture name the knob.
#[test]
fn the_guide_states_the_global_view_and_every_known_difference() {
    let read = |path: &str| {
        let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        std::fs::read_to_string(&full)
            .unwrap_or_else(|e| panic!("{} is unreadable: {e}", full.display()))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let shims = read("docs/guide/compat-shims.md");
    assert!(
        shims.starts_with("# MailHog / Mailpit / MailSlurper shims"),
        "the page title names all three shims"
    );
    for needle in [
        "## MailSlurper service API",
        "50 to a page",
        "the page size is 50, upstream's own",
        "*`dateSent` is the receive time, in UTC*",
        "*Order is the receive time*",
        "*A page past the end is a `200`*",
        "*An unknown id is a `404`*",
        "*`ccAddresses` is an extra key*",
        "`swarmail_http_requests_in_flight`",
        "*A list page over 64 MiB is a `400`*",
        "*Still recalled, not read from source*",
        "*`subject` is HTML-escaped, `body` is not*",
        "**It is a global view.**",
        "`GET /mail` and `GET /mailcount` read **every inbox**",
        "`DELETE /mail` with `{\"pruneCode\":\"all\"}` **clears every inbox**",
        "collapses per-inbox isolation",
        "one Swarmail (one process and one data file) per suite",
        "`--mailslurper-listen <host:port>`",
        "`SWARMAIL_MAILSLURPER_LISTEN`",
        "**The HTTP port serves none of these routes, under any prefix**",
        "--set config.SWARMAIL_MAILSLURPER_LISTEN=0.0.0.0:4437",
        "publish port 4437 with a Service of your own",
        "`60plus`, `30plus` and `2wksplus` are each a `400` that names the code",
        "`mailItems` is omitted whenever the page is empty",
        "*Not served*",
        "*Omitted item fields*",
        "*Closed query*",
        "*Fixed paging*",
        "*Identity encoding*",
        "*Errors*",
        "*Not in `/metrics`*",
        "decision 0118",
    ] {
        assert!(shims.contains(needle), "compat-shims.md lacks {needle:?}");
    }
    // The agent brief scopes the "only cross-inbox wipe" claim to the HTTP
    // plane and names the MailSlurper route; the source is the file the
    // server serves (tests/api.rs pins that byte equality).
    let llms = read("src/static/llms.txt");
    assert!(
        llms.contains("is the HTTP plane's only cross-inbox wipe"),
        "llms.txt scopes the wipe claim"
    );
    assert!(
        llms.contains("`DELETE /mail` with `{\"pruneCode\":\"all\"}` clears every inbox"),
        "llms.txt names the MailSlurper wipe"
    );
    for path in ["README.md", "docs/guide/repo-layout.md"] {
        assert!(
            read(path).contains("MailSlurper"),
            "{path} names MailSlurper"
        );
    }
    for (path, needle) in [
        (
            "docs/guide/quickstart.md",
            "`SWARMAIL_MAILSLURPER_LISTEN` (no default, off",
        ),
        (
            "docs/guide/network-posture.md",
            "`--mailslurper-listen`, off by default",
        ),
    ] {
        assert!(read(path).contains(needle), "{path} lacks {needle:?}");
    }
}
