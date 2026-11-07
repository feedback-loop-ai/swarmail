//! The oracle's per-backend mail access (proposed decision 0129 §5, spec
//! R7): MIME decoding over fixture-generated messages, the normalizers, and
//! the competitor source against a stand-in adapter speaking MCP over HTTP.

use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use swarmail_evals::mail::{self, Attachment, Body, Message};
use swarmail_evals::mailsource::{
    Competitor, CompetitorSource, MailSource, decode_mime, normalize_mailhog, normalize_mailpit,
};
use swarmail_evals::mcp::McpClient;

fn rendered(body: Body) -> String {
    let m = Message {
        from: "app@fixture.eval".into(),
        to: "alice@fixture.eval".into(),
        subject: "Your verification code".into(),
        body,
    };
    String::from_utf8(mail::render(&m)).unwrap()
}

#[test]
fn every_shape_the_fixture_renders_decodes_to_its_parts() {
    let long_line = "x".repeat(950);
    let unicode = "héllo wörld — ünïcode ✓";
    for (body, text, html) in [
        (
            Body::Text("Your code is 123456.\nBye.".into()),
            "Your code is 123456.\r\nBye.",
            None,
        ),
        (Body::Text(long_line.clone()), long_line.as_str(), None),
        (Body::Text(unicode.into()), unicode, None),
        (
            Body::Alternative {
                text: "plain part".into(),
                html: "<p>html <a href=\"http://a/\">part</a></p>".into(),
            },
            "plain part",
            Some("<p>html <a href=\"http://a/\">part</a></p>"),
        ),
        (
            Body::Alternative {
                text: unicode.into(),
                html: format!("<p>{unicode}</p>"),
            },
            unicode,
            Some("<p>héllo wörld — ünïcode ✓</p>"),
        ),
        (
            Body::HtmlOnly("<b>only html</b>".into()),
            "",
            Some("<b>only html</b>"),
        ),
        (
            Body::Mixed {
                text: "with an attachment".into(),
                attachment: Attachment {
                    filename: "a.ics".into(),
                    content_type: "text/calendar".into(),
                    bytes: b"BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".to_vec(),
                },
            },
            "with an attachment",
            None,
        ),
    ] {
        let raw = rendered(body.clone());
        let got = decode_mime(&raw).unwrap_or_else(|e| panic!("{body:?}: {e}\n{raw}"));
        assert_eq!(
            got.text.as_deref().map(|t| t.trim_end()).unwrap_or(""),
            text.trim_end(),
            "{body:?}"
        );
        assert_eq!(got.html.as_deref().map(|h| h.trim_end()), html, "{body:?}");
    }
}

#[test]
fn quoted_printable_8bit_and_folded_headers_decode() {
    let qp = "Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\ncaf=C3=A9 and a soft =\r\nbreak\r\n";
    assert_eq!(
        decode_mime(qp).unwrap().text.unwrap().trim_end(),
        "café and a soft break"
    );
    let eight = "Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\ncafé\r\n";
    assert_eq!(decode_mime(eight).unwrap().text.unwrap().trim_end(), "café");
    let folded = "Content-Type: multipart/alternative;\r\n boundary=\"b1\"\r\n\r\n--b1\r\nContent-Type: text/plain\r\n\r\none\r\n--b1--\r\n";
    assert_eq!(decode_mime(folded).unwrap().text.unwrap().trim_end(), "one");
    // No Content-Type at all is RFC 5322's us-ascii text.
    assert_eq!(
        decode_mime("Subject: x\r\n\r\nhello\r\n")
            .unwrap()
            .text
            .unwrap()
            .trim_end(),
        "hello"
    );
    // LF-only line ends decode too.
    assert!(decode_mime("Subject: x\n\nhello\n").unwrap().text.is_some());
}

#[test]
fn an_unknown_mime_shape_is_an_error_never_a_pass() {
    let part = |ct: &str, cte: &str, body: &str| {
        format!("Content-Type: {ct}\r\nContent-Transfer-Encoding: {cte}\r\n\r\n{body}\r\n")
    };
    for (what, raw) in [
        ("a charset", part("text/plain; charset=iso-8859-1", "7bit", "x")),
        ("an encoding", part("text/plain; charset=utf-8", "x-uuencode", "x")),
        ("a top-level type", part("application/pdf", "base64", "eA==")),
        ("a body that is not base64", part("text/plain; charset=utf-8", "base64", "***")),
        ("bytes that are not UTF-8", part("text/plain; charset=utf-8", "base64", "/w==")),
        ("a truncated escape", part("text/plain; charset=utf-8", "quoted-printable", "=4")),
        ("a bad escape", part("text/plain; charset=utf-8", "quoted-printable", "=ZZ")),
        ("a message without headers", "just a line".to_string()),
        ("a header without a colon", "no colon here\r\n\r\nbody".to_string()),
        ("a continuation of nothing", " folded\r\n\r\nbody".to_string()),
        (
            "an unclosed multipart",
            "Content-Type: multipart/alternative; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nx\r\n".to_string(),
        ),
        (
            "a multipart without a boundary",
            "Content-Type: multipart/alternative\r\n\r\n--b--\r\n".to_string(),
        ),
        (
            "an unknown multipart subtype",
            "Content-Type: multipart/encrypted; boundary=b\r\n\r\n--b--\r\n".to_string(),
        ),
    ] {
        assert!(decode_mime(&raw).is_err(), "{what} must be refused:\n{raw}");
    }
    // Nesting is bounded.
    let mut deep = "Content-Type: text/plain\r\n\r\nx\r\n".to_string();
    for i in 0..8 {
        deep = format!(
            "Content-Type: multipart/mixed; boundary=b{i}\r\n\r\n--b{i}\r\n{deep}\r\n--b{i}--\r\n"
        );
    }
    assert!(decode_mime(&deep).unwrap_err().contains("nest"));
}

#[test]
fn the_normalizers_map_each_backends_shape_and_refuse_a_missing_field() {
    let raw = rendered(Body::Alternative {
        text: "t".into(),
        html: "<p>h</p>".into(),
    });
    let hog = json!({"ID": "i1", "Content": {"Headers": {"Subject": ["Your verification code"]}},
        "To": [{"Mailbox": "alice", "Domain": "fixture.eval", "Relays": null, "Params": ""}],
        "Raw": {"Data": raw}});
    let n = normalize_mailhog(&hog).unwrap();
    assert_eq!(n["subject"], "Your verification code");
    assert_eq!(n["to"], json!([{"address": "alice@fixture.eval"}]));
    assert_eq!(n["text"].as_str().unwrap().trim_end(), "t");
    assert_eq!(n["html"].as_str().unwrap().trim_end(), "<p>h</p>");
    for field in ["To", "Raw"] {
        let mut broken = hog.clone();
        broken.as_object_mut().unwrap().remove(field);
        assert!(normalize_mailhog(&broken).is_err(), "{field}");
    }
    let mut no_subject = hog.clone();
    no_subject["Content"] = json!({});
    assert!(normalize_mailhog(&no_subject).is_err());
    let mut bad_path = hog.clone();
    bad_path["To"] = json!([{"Mailbox": "alice"}]);
    assert!(normalize_mailhog(&bad_path).is_err());

    let pit = json!({"ID": "p1", "Subject": "S", "Text": "body", "HTML": "",
        "To": [{"Name": "", "Address": "bob@fixture.eval"}]});
    let n = normalize_mailpit(&pit).unwrap();
    assert_eq!(
        (n["subject"].clone(), n["text"].clone(), n["html"].clone()),
        ("S".into(), "body".into(), Value::Null),
        "an empty HTML is no HTML"
    );
    assert_eq!(n["to"], json!([{"address": "bob@fixture.eval"}]));
    for field in ["Subject", "To"] {
        let mut broken = pit.clone();
        broken.as_object_mut().unwrap().remove(field);
        assert!(normalize_mailpit(&broken).is_err(), "{field}");
    }
    let mut no_address = pit.clone();
    no_address["To"] = json!([{"Name": "x"}]);
    assert!(normalize_mailpit(&no_address).is_err());
}

/// A stand-in adapter: MCP over HTTP answering the list and read tools from
/// a store that grows by one message per list call.
struct Stand {
    url: String,
    calls: Arc<Mutex<Vec<String>>>,
}

async fn stand_in(kind: Competitor, messages: Vec<Value>, appear_after: usize) -> Stand {
    use axum::{Json, Router, routing::post};
    let calls = Arc::new(Mutex::new(Vec::new()));
    let lists = Arc::new(AtomicUsize::new(0));
    let (log, messages) = (calls.clone(), Arc::new(messages));
    let app = Router::new().route(
        "/mcp",
        post(move |Json(req): Json<Value>| {
            let (log, lists, messages) = (log.clone(), lists.clone(), messages.clone());
            async move {
                let Some(id) = req.get("id").cloned() else {
                    return (axum::http::StatusCode::ACCEPTED, String::new());
                };
                let method = req["method"].as_str().unwrap_or_default();
                let result = if method == "initialize" {
                    json!({"protocolVersion": "2025-03-26", "capabilities": {}})
                } else {
                    let name = req["params"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    log.lock().unwrap().push(name.clone());
                    let text = match name.as_str() {
                        "mailpit_list" | "mailhog_list" => {
                            let seen = lists.fetch_add(1, Ordering::SeqCst);
                            let shown: Vec<Value> = if seen < appear_after {
                                Vec::new()
                            } else {
                                messages.iter().cloned().collect()
                            };
                            let key = if name == "mailpit_list" {
                                "messages"
                            } else {
                                "items"
                            };
                            let start =
                                req["params"]["arguments"]["start"].as_u64().unwrap_or(0) as usize;
                            let page: Vec<Value> =
                                shown.iter().skip(start).take(2).cloned().collect();
                            json!({"total": shown.len(), "start": start, key: page}).to_string()
                        }
                        "mailpit_read" => {
                            let want = req["params"]["arguments"]["id"]
                                .as_str()
                                .unwrap_or_default();
                            messages
                                .iter()
                                .find(|m| m["ID"] == want)
                                .cloned()
                                .unwrap_or(Value::Null)
                                .to_string()
                        }
                        _ => "{}".into(),
                    };
                    json!({"content": [{"type": "text", "text": text}], "isError": false})
                };
                (
                    axum::http::StatusCode::OK,
                    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string(),
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let _ = kind;
    Stand {
        url: format!("http://{addr}/mcp"),
        calls,
    }
}

fn pit_message(id: &str, subject: &str, to: &str) -> Value {
    json!({"ID": id, "Subject": subject, "Text": format!("text of {id}"), "HTML": "",
        "To": [{"Name": "", "Address": to}]})
}

fn hog_message(id: &str, subject: &str, to: &str) -> Value {
    let (mailbox, domain) = to.split_once('@').unwrap();
    let raw = format!("Subject: {subject}\r\nContent-Type: text/plain\r\n\r\ntext of {id}\r\n");
    json!({"ID": id, "Content": {"Headers": {"Subject": [subject]}},
        "To": [{"Mailbox": mailbox, "Domain": domain}], "Raw": {"Data": raw}})
}

#[tokio::test]
async fn the_mailpit_source_pages_the_listing_filters_by_subject_and_reads_each_mail() {
    let msgs = vec![
        pit_message("a", "Your verification code", "u@fixture.eval"),
        pit_message("b", "Welcome", "u@fixture.eval"),
        pit_message("c", "Your verification code", "u@fixture.eval"),
        pit_message("d", "Your verification code", "u@fixture.eval"),
    ];
    // The store shows nothing for the first two listings: accept is not
    // presumed to be storage, so the source polls.
    let stand = stand_in(Competitor::Mailpit, msgs, 2).await;
    let mut mcp = McpClient::connect(&stand.url).await.unwrap();
    let mut source = CompetitorSource::new(&mut mcp, Competitor::Mailpit);
    let got = source
        .wait_emails(Some("Your verification code"), 3)
        .await
        .unwrap();
    assert_eq!(got.len(), 3, "the Welcome mail is filtered out");
    assert_eq!(got[0]["text"], "text of a");
    assert_eq!(got[0]["to"], json!([{"address": "u@fixture.eval"}]));
    let calls = stand.calls.lock().unwrap().clone();
    let reads = calls.iter().filter(|c| *c == "mailpit_read").count();
    let lists = calls.iter().filter(|c| *c == "mailpit_list").count();
    assert_eq!(reads, 3, "{calls:?}");
    assert!(lists >= 4, "two empty polls, then two pages: {calls:?}");
    assert_eq!(source.tool_calls() as usize, calls.len());
    assert!(
        calls.iter().all(|c| c.starts_with("mailpit_")),
        "only the backend's own tools"
    );
}

#[tokio::test]
async fn the_mailhog_source_decodes_the_items_it_lists_and_never_reads_by_id() {
    let msgs = vec![
        hog_message("x", "Your weekly digest", "u@fixture.eval"),
        hog_message("y", "Other", "u@fixture.eval"),
    ];
    let stand = stand_in(Competitor::Mailhog, msgs, 0).await;
    let mut mcp = McpClient::connect(&stand.url).await.unwrap();
    let mut source = CompetitorSource::new(&mut mcp, Competitor::Mailhog);
    let got = source.wait_emails(Some("weekly digest"), 1).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["text"].as_str().unwrap().trim_end(), "text of x");
    assert!(
        stand
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|c| c == "mailhog_list")
    );
}

#[tokio::test]
async fn a_count_that_is_never_met_is_an_error_after_the_deadline() {
    let stand = stand_in(
        Competitor::Mailpit,
        vec![pit_message("a", "Welcome", "u@fixture.eval")],
        0,
    )
    .await;
    let mut mcp = McpClient::connect(&stand.url).await.unwrap();
    let mut source = CompetitorSource::new(&mut mcp, Competitor::Mailpit)
        .with_deadline(std::time::Duration::from_millis(300));
    let err = source
        .wait_emails(Some("Your verification code"), 1)
        .await
        .unwrap_err();
    assert!(err.contains("listed 0"), "{err}");
    // An unknown MIME shape is an oracle error even when the mail is there.
    let mut odd = hog_message("z", "S", "u@fixture.eval");
    odd["Raw"]["Data"] = json!("Content-Type: application/pdf\r\n\r\n%PDF");
    let stand = stand_in(Competitor::Mailhog, vec![odd], 0).await;
    let mut mcp = McpClient::connect(&stand.url).await.unwrap();
    let mut source = CompetitorSource::new(&mut mcp, Competitor::Mailhog);
    assert!(source.wait_emails(Some("S"), 1).await.is_err());
}
