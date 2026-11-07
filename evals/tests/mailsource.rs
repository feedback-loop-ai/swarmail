//! The oracle's per-backend mail access (proposed decision 0129 §5, spec
//! R7): MIME decoding over fixture-generated messages, the normalizers, and
//! the competitor source against a stand-in adapter speaking MCP over HTTP.

use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use swarmail_evals::mail::{self, Attachment, Body, Message};
use swarmail_evals::mailsource::{
    Competitor, CompetitorSource, MailSource, decode_mime, normalize_mailhog, normalize_mailpit,
    normalize_mailslurper,
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
                        "mailslurper_list" => {
                            let seen = lists.fetch_add(1, Ordering::SeqCst);
                            let shown: Vec<Value> = if seen < appear_after {
                                Vec::new()
                            } else {
                                messages.iter().cloned().collect()
                            };
                            let page =
                                req["params"]["arguments"]["page"].as_u64().unwrap_or(1) as usize;
                            let items: Vec<Value> = shown
                                .iter()
                                .skip((page - 1) * 50)
                                .take(50)
                                .cloned()
                                .collect();
                            // An empty page leaves `mailItems` out, as the
                            // operator observed real MailSlurper do.
                            if items.is_empty() {
                                json!({"totalPages": 0, "totalRecords": shown.len()}).to_string()
                            } else {
                                json!({"mailItems": items, "totalPages": shown.len().div_ceil(50),
                                    "totalRecords": shown.len()})
                                .to_string()
                            }
                        }
                        "mailslurper_read" => {
                            let want = req["params"]["arguments"]["id"]
                                .as_str()
                                .unwrap_or_default();
                            messages
                                .iter()
                                .find(|m| m["id"] == want)
                                .cloned()
                                .unwrap_or(Value::Null)
                                .to_string()
                        }
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

fn slurper_message(id: &str, subject: &str, to: &str) -> Value {
    json!({"id": id, "dateSent": "2026-11-07 10:00:00", "fromAddress": "app@fixture.eval",
        "toAddresses": [to], "subject": subject, "body": format!("text of {id}"),
        "contentType": "text/plain; charset=utf-8", "boundary": "", "transferEncoding": "",
        "attachments": []})
}

#[test]
fn the_mailslurper_normalizer_reads_the_shapes_the_pinned_executable_serves() {
    let plain = slurper_message("m1", "Your verification code", "bob@fixture.eval");
    let n = normalize_mailslurper(&plain).unwrap();
    assert_eq!(
        (n["subject"].clone(), n["text"].clone(), n["html"].clone()),
        (
            "Your verification code".into(),
            "text of m1".into(),
            Value::Null
        )
    );
    assert_eq!(n["to"], json!([{"address": "bob@fixture.eval"}]));
    // MailSlurper's line break is `\n<br/>` and its sanitiser escapes `&`.
    let mut lines = plain.clone();
    lines["body"] = "a\n<br/>b &amp; c\n<br/>".into();
    assert_eq!(normalize_mailslurper(&lines).unwrap()["text"], "a\nb & c\n");
    let mut html = plain.clone();
    html["contentType"] = "text/html; charset=utf-8".into();
    html["body"] = "<p>only html</p>".into();
    let n = normalize_mailslurper(&html).unwrap();
    assert_eq!(
        (n["text"].clone(), n["html"].clone()),
        ("".into(), "<p>only html</p>".into())
    );
    // A multipart keeps one part, the HTML part, whatever its boundary says.
    let mut multi = plain.clone();
    multi["contentType"] = "multipart/alternative; boundary=\"b1\"".into();
    multi["body"] = "<p>code 123456</p>".into();
    let n = normalize_mailslurper(&multi).unwrap();
    assert_eq!(
        (n["text"].clone(), n["html"].clone()),
        ("".into(), "<p>code 123456</p>".into())
    );
    // Anything it does not know is an error, never a pass.
    for (field, value) in [
        ("subject", Value::Null),
        ("toAddresses", Value::Null),
        ("toAddresses", json!([{"address": "x@y"}])),
        ("contentType", Value::Null),
        ("contentType", "".into()),
        ("contentType", "application/pdf".into()),
        ("body", Value::Null),
    ] {
        let mut broken = plain.clone();
        broken[field] = value.clone();
        assert!(normalize_mailslurper(&broken).is_err(), "{field}={value}");
    }
    // A multipart body that is not markup (a text part, or base64 GetMail did
    // not decode), and markup in a text body the sanitiser would have escaped.
    for body in ["PHA+Y29kZQ==", "code =3D 1"] {
        let mut bad = multi.clone();
        bad["body"] = body.into();
        let why = normalize_mailslurper(&bad).unwrap_err();
        assert!(why.contains("not its HTML part"), "{why}");
    }
    let mut bad = plain.clone();
    bad["body"] = "x <br/> y".into();
    let why = normalize_mailslurper(&bad).unwrap_err();
    assert!(
        why.contains("markup its sanitiser would have escaped"),
        "{why}"
    );
}

fn corpus_text(target: &str, file: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/evals/wire-corpus")
        .join(target)
        .join(file);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

#[test]
fn the_mailslurper_normalizer_maps_every_real_response_in_its_wire_corpus() {
    use swarmail_evals::oracle::labelled_url;
    let read = |f: &str| normalize_mailslurper(&corpus_text("mailslurper", f)).unwrap();
    // A body GetMail does not base64-decode: plain text, line breaks undone.
    let n = read("read.json");
    assert_eq!(n["text"], "code 111111\n");
    assert_eq!(n["html"], Value::Null);
    assert_eq!(n["to"], json!([{"address": "ann@corpus.invalid"}]));
    // A body GetMail does decode: the multipart's base64 HTML part, which the
    // listing still carries encoded.
    let n = read("read-multipart.json");
    assert_eq!(n["html"], "<p>code 123456</p>");
    assert_eq!(n["text"], "");
    let listed = corpus_text("mailslurper", "list.json");
    let item = listed["mailItems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["subject"] == "corpus-multipart")
        .unwrap();
    assert_eq!(item["body"], "PHA+Y29kZSAxMjM0NTY8L3A+");
    assert!(
        normalize_mailslurper(item).is_err(),
        "the list is not decoded"
    );
    // A top-level base64 text body, decoded by MailSlurper at ingest.
    let n = read("read-base64.json");
    assert_eq!(
        n["text"],
        "code 444444\nsecond line https://x.invalid/a?b=1&c=2\n"
    );
    // The fixture's text mails are read by their labelled URL: the entity and
    // the line breaks are undone, so the URL is the one that was sent.
    let n = read("read-lines.json");
    assert_eq!(
        labelled_url(n["text"].as_str().unwrap(), "Confirm your account:").as_deref(),
        Some("http://x.invalid/a?b=1&c=2")
    );
    // Every plain mail the listing carries normalizes; each recipient is exact,
    // so the substring filter's lookalike is a different address.
    let found = corpus_text("mailslurper", "search-filtered.json");
    assert_eq!(found["totalRecords"], 5, "the filter also returns joann@");
    let mut to: Vec<String> = found["mailItems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["subject"] != "corpus-multipart")
        .map(|m| {
            normalize_mailslurper(m).unwrap()["to"][0]["address"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    to.sort();
    assert_eq!(
        to,
        [
            "ann@corpus.invalid",
            "ann@corpus.invalid",
            "ann@corpus.invalid",
            "joann@corpus.invalid"
        ]
    );
    let empty = corpus_text("mailslurper", "search-empty.json");
    assert_eq!(
        (empty["totalRecords"].clone(), empty["totalPages"].clone()),
        (0.into(), 0.into())
    );
    assert_eq!(empty["mailItems"], json!([]));
}

#[tokio::test]
async fn a_lookalike_recipient_the_substring_filter_returns_is_excluded_by_the_exact_match() {
    // MailSlurper's `to` filter is a SQL LIKE substring match: `ann@x` also
    // returns `joann@x`. The source lists whatever the adapter returns and
    // normalizes each mail's recipients exactly; the solutions then keep only
    // the mail whose address equals their identity.
    let msgs = vec![
        slurper_message("a", "Confirm your account", "ann@fixture.eval"),
        slurper_message("b", "Confirm your account", "joann@fixture.eval"),
    ];
    let stand = stand_in(Competitor::Mailslurper, msgs, 0).await;
    let mut mcp = McpClient::connect(&stand.url).await.unwrap();
    let mut source = CompetitorSource::new(&mut mcp, Competitor::Mailslurper);
    let got = source
        .wait_emails(Some("Confirm your account"), 2)
        .await
        .unwrap();
    let mine: Vec<&Value> = got
        .iter()
        .filter(|m| {
            m["to"]
                .as_array()
                .is_some_and(|to| to.iter().any(|a| a["address"] == json!("ann@fixture.eval")))
        })
        .collect();
    assert_eq!(got.len(), 2, "the substring match returned the lookalike");
    assert_eq!(mine.len(), 1, "the exact match excludes it");
    assert_eq!(mine[0]["to"], json!([{"address": "ann@fixture.eval"}]));
}

#[tokio::test]
async fn the_mailslurper_source_pages_by_fifty_polls_an_empty_page_and_uses_only_its_tools() {
    let msgs: Vec<Value> = (0..51)
        .map(|i| {
            slurper_message(
                &format!("m{i:02}"),
                "Your verification code",
                "u@fixture.eval",
            )
        })
        .collect();
    // The first listing is empty (and leaves `mailItems` out): accept is not
    // presumed to be storage, so the source polls.
    let stand = stand_in(Competitor::Mailslurper, msgs, 1).await;
    let mut mcp = McpClient::connect(&stand.url).await.unwrap();
    let mut source = CompetitorSource::new(&mut mcp, Competitor::Mailslurper);
    let got = source
        .wait_emails(Some("Your verification code"), 51)
        .await
        .unwrap();
    assert_eq!(got.len(), 51);
    assert_eq!(got[50]["text"], "text of m50");
    let calls = stand.calls.lock().unwrap().clone();
    assert_eq!(
        calls.iter().filter(|c| *c == "mailslurper_read").count(),
        51
    );
    assert_eq!(
        calls.iter().filter(|c| *c == "mailslurper_list").count(),
        3,
        "one empty poll, then two pages: {calls:?}"
    );
    assert!(
        calls.iter().all(|c| c.starts_with("mailslurper_")),
        "{calls:?}"
    );
    assert_eq!(source.tool_calls() as usize, calls.len());
}

fn corpus(target: &str, file: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/evals/wire-corpus")
        .join(target)
        .join(file);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}")))
        .unwrap()
}

#[test]
fn the_normalizers_map_every_real_upstream_response_in_the_wire_corpus() {
    // Real Mailpit and MailHog responses to synthetic fixture-shaped mail,
    // captured from the pinned executables (docs/evals/wire-corpus).
    for (file, text, html) in [
        ("read.json", "code 111111", None),
        (
            "read-multipart.json",
            "code = 123456 café",
            Some("<p>code 123456</p>"),
        ),
    ] {
        let pit = normalize_mailpit(&corpus("mailpit", file)).unwrap();
        let hog = normalize_mailhog(&corpus("mailhog", file)).unwrap();
        for (name, n) in [("mailpit", &pit), ("mailhog", &hog)] {
            assert_eq!(
                n["text"].as_str().unwrap().trim_end(),
                text,
                "{name} {file}"
            );
            assert_eq!(n["html"].as_str().map(str::trim_end), html, "{name} {file}");
            assert_eq!(
                n["to"],
                json!([{"address": "ann@corpus.invalid"}]),
                "{name} {file}"
            );
        }
        assert_eq!(pit["subject"], hog["subject"], "{file}");
    }
    // Every item MailHog lists normalizes, and each recipient is exact: the
    // lookalike is a different address, whatever the upstream filter returns.
    let list = corpus("mailhog", "list.json");
    let mut to: Vec<String> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            let n = normalize_mailhog(m).unwrap();
            n["to"][0]["address"].as_str().unwrap().to_owned()
        })
        .collect();
    to.sort();
    assert_eq!(
        to,
        [
            "ann@corpus.invalid",
            "ann@corpus.invalid",
            "joann@corpus.invalid"
        ]
    );
    // The filtered search returned all three: the upstream filter is a
    // substring match on both, so the exact match is the oracle's.
    assert_eq!(
        corpus("mailpit", "search-filtered.json")["messages_count"],
        3
    );
    assert_eq!(corpus("mailhog", "search-filtered.json")["total"], 3);
}
