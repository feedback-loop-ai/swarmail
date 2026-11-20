//! Story B of accepted decision 0132: protocol 4's API section. Capability
//! tables and their citations against the captured bytes, the strict
//! decoders against the real wire corpus, the scan's completeness rules
//! against a bounded local responder, the scheduler's accounting under a
//! paused clock, the exact checks and destructive postchecks, and the whole
//! native cell sequence against the real candidate binary.

use super::*;
use crate::api::{self, Outcome, Phase, PlannedCell};
use crate::clients::{self, Capability, ClearScope, Family, Operation, Scope, Spend, Surface};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;

const DRAFT_V4: &[u8] = include_bytes!("../../docs/evals/draft/protocol.v4.json");
const LOCK_V5: &[u8] = include_bytes!("../../docs/evals/draft/sources.lock.v5.json");

fn corpus(path: &str) -> Value {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/evals/wire-corpus")
            .join(path),
    )
    .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

const LAUNCHED: [&str; 8] = [
    "swarmail",
    "mailpit",
    "mailhog",
    "mailslurper-source-build",
    "smtp4dev",
    "inbucket",
    "greenmail",
    "alkemio-mailslurper",
];

// ---- capability tables and citations (T1) ---------------------------------

#[test]
fn every_native_cell_is_decided_or_named_not_inspected() {
    let status = |t: &str, op| clients::native_capability(t, op).status();
    for t in LAUNCHED {
        for op in [Operation::List, Operation::Search, Operation::Get] {
            assert_eq!(status(t, op), "supported", "{t} {op:?}");
        }
        // Extraction is supported exactly where the read is.
        for op in [Operation::ExtractCode, Operation::ExtractLink] {
            assert_eq!(status(t, op), "supported", "{t} {op:?}");
        }
    }
    assert_eq!(status("mailslurper", Operation::List), "does_not_start");
    // Missing evidence is never `not_supported` (0132 §6.3).
    assert_eq!(status("mailhog", Operation::Delete), "not_inspected");
    assert_eq!(status("mailhog", Operation::Clear), "not_inspected");
    assert_eq!(
        status("mailslurper-source-build", Operation::Clear),
        "not_inspected"
    );
    assert_eq!(
        status("mailslurper-source-build", Operation::Delete),
        "not_supported"
    );
    assert_eq!(
        status("alkemio-mailslurper", Operation::Delete),
        "not_supported"
    );
    assert_eq!(status("greenmail", Operation::Delete), "not_supported");
    for t in ["swarmail", "mailpit", "smtp4dev", "inbucket"] {
        assert_eq!(status(t, Operation::Delete), "supported", "{t}");
    }
    let scope = |t: &str| clients::native_capability(t, Operation::Clear).scope();
    assert_eq!(scope("swarmail"), Some(ClearScope::Recipient));
    assert_eq!(scope("inbucket"), Some(ClearScope::Recipient));
    for t in ["mailpit", "smtp4dev", "greenmail"] {
        assert_eq!(scope(t), Some(ClearScope::All), "{t}");
    }
    // GreenMail is read over REST in v4 (acceptance answer 8), never POP3.
    match clients::native_capability("greenmail", Operation::Get) {
        Capability::Supported { routes, .. } => {
            assert!(
                routes.iter().all(|r| r.starts_with("GET /api/user/")),
                "{routes:?}"
            )
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_mcp_surface_is_the_bridge_targets_and_swarmail_only() {
    for t in ["smtp4dev", "inbucket", "greenmail", "alkemio-mailslurper"] {
        for op in Operation::ALL {
            assert_eq!(
                clients::mcp_capability(t, op).status(),
                "no_mcp_surface",
                "{t}"
            );
        }
    }
    assert_eq!(
        clients::mcp_capability("mailslurper", Operation::List).status(),
        "does_not_start"
    );
    for t in ["mailpit", "mailhog", "mailslurper-source-build"] {
        for op in [Operation::Delete, Operation::Clear] {
            match clients::mcp_capability(t, op) {
                Capability::NotSupported { reason, .. } => {
                    assert!(reason.contains("0092 §3, 0129 §5"), "{reason}")
                }
                other => panic!("{t} {op:?}: {other:?}"),
            }
        }
        assert!(
            clients::mcp_capability(t, Operation::Wait).supported(),
            "{t}"
        );
    }
    for op in Operation::ALL {
        assert!(
            clients::mcp_capability("swarmail", op).supported(),
            "{op:?}"
        );
    }
}

fn lock5() -> protocol::SourceLock {
    protocol::SourceLock::parse(LOCK_V5).unwrap()
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn every_citation_resolves_to_its_bytes_in_lock_v5() {
    let lock = lock5();
    lock.verify_files(&root().join("docs/evals"))
        .expect("lock v5's documents verify");
    for t in LAUNCHED {
        let t = TargetKind::parse(t).unwrap();
        api_chunk::check_citations(t, &lock, &root().join("docs/evals"), root())
            .unwrap_or_else(|w| panic!("{t}: {w:?}"));
    }
}

#[test]
fn a_citation_whose_bytes_changed_or_whose_document_is_not_pinned_refuses() {
    // A documents directory whose MailHog Swagger lost the search route.
    let tmp = tempfile::tempdir().unwrap();
    let src = root().join("docs/evals");
    let copy = |rel: &str, edit: &dyn Fn(String) -> String| {
        let to = tmp.path().join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        let text = std::fs::read_to_string(src.join(rel)).unwrap();
        std::fs::write(to, edit(text)).unwrap();
    };
    copy("sources/mailhog/mailhog-swagger.json", &|t| {
        t.replace("\"/api/v2/search\"", "\"/api/v2/elsewhere\"")
    });
    let w =
        api_chunk::check_citations(TargetKind::Mailhog, &lock5(), tmp.path(), root()).unwrap_err();
    assert!(
        w.iter()
            .any(|l| l.contains("does not state \"\\\"/api/v2/search\\\"\"")),
        "{w:?}"
    );
    // A pin that does not bind the cited document at all.
    let mut lock: Value = serde_json::from_slice(LOCK_V5).unwrap();
    for e in lock["entries"].as_array_mut().unwrap() {
        if e["target"] == "inbucket" {
            let docs = e["pin"]["documents"].as_array_mut().unwrap();
            docs.retain(|d| d["path"] != "sources/inbucket/inbucket-rest-apiv1.sh");
        }
    }
    let lock = protocol::SourceLock::parse(&serde_json::to_vec(&lock).unwrap()).unwrap();
    let w = api_chunk::check_citations(TargetKind::Inbucket, &lock, &src, root()).unwrap_err();
    assert!(
        w.iter()
            .any(|l| l.contains("is no document of its lock pin")),
        "{w:?}"
    );
    // A candidate route the tree no longer states.
    let repo = tempfile::tempdir().unwrap();
    let w =
        api_chunk::check_citations(TargetKind::Swarmail, &lock5(), &src, repo.path()).unwrap_err();
    assert!(
        w.iter().any(|l| l.contains("src/api.rs cannot be read")),
        "{w:?}"
    );
}

// ---- strict decoders over the real wire corpus (T2) -----------------------

#[test]
fn decoders_read_the_real_wire_corpus_exactly() {
    let (total, items) = clients::mailpit::decode_page(&corpus("mailpit/list.json"), 0).unwrap();
    assert_eq!((total, items.len()), (3, 3));
    assert!(items.iter().all(|s| !s.to.is_empty() && !s.id.is_empty()));
    // A page that does not echo the start asked for is refused.
    assert!(clients::mailpit::decode_page(&corpus("mailpit/list.json"), 250).is_none());
    let (total, items) = clients::mailhog::decode_page(&corpus("mailhog/list.json"), 0).unwrap();
    assert_eq!(total as usize, items.len());
    assert_eq!(
        items
            .iter()
            .find(|s| s.subject == "corpus-multipart")
            .unwrap()
            .to,
        ["ann@corpus.invalid"]
    );
    let (total, items) =
        clients::mailslurper::decode_page(&corpus("mailslurper/list.json"), 0).unwrap();
    assert_eq!(total as usize, items.len());
    assert_eq!(items[0].to, ["ann@corpus.invalid"]);
    // MailSlurper keeps one decoded body: the multipart read has HTML only.
    let f =
        clients::mailslurper::decode_message(&corpus("mailslurper/read-multipart.json")).unwrap();
    assert_eq!(
        (f.text.as_deref(), f.html.as_deref()),
        (None, Some("<p>code 123456</p>"))
    );
    // The pinned builds leave both named fields empty and keep the HTML
    // part in `body` (host probe, 2026-11-18): it is filed as HTML, by what
    // it is. A plain body, or a multipart body that is not markup, is text.
    let mut live = corpus("mailslurper/read-multipart.json");
    let kept = r#"<p>code 123456 <a href="https://example.test/a" rel="nofollow">go</a></p>"#;
    live["body"] = json!(kept);
    live["HTMLBody"] = json!("");
    let f = clients::mailslurper::decode_message(&live).unwrap();
    assert_eq!((f.text.as_deref(), f.html.as_deref()), (None, Some(kept)));
    for (ctype, body) in [
        ("text/plain; charset=utf-8", "<not html> code 123456"),
        ("multipart/alternative; boundary=\"b1\"", "code 123456"),
    ] {
        live["contentType"] = json!(ctype);
        live["body"] = json!(body);
        let f = clients::mailslurper::decode_message(&live).unwrap();
        assert_eq!(
            (f.text.as_deref(), f.html.as_deref()),
            (Some(body), None),
            "{ctype}"
        );
    }
    live["contentType"] = json!("text/html");
    live["body"] = json!("<p>x</p>");
    let f = clients::mailslurper::decode_message(&live).unwrap();
    assert_eq!(
        (f.text.as_deref(), f.html.as_deref()),
        (None, Some("<p>x</p>"))
    );
    // MailHog's raw multipart, quoted-printable, through the shared reader.
    let hog = corpus("mailhog/list.json");
    let item = hog["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["Content"]["Headers"]["Subject"][0] == "corpus-multipart")
        .unwrap();
    let f = clients::mailhog::decode_item(item).unwrap();
    assert_eq!(
        f.text.as_deref().map(str::trim_end),
        Some("code = 123456 café")
    );
    assert_eq!(f.to, ["ann@corpus.invalid"]);
    assert_eq!(f.from.as_deref(), Some("sender@corpus.invalid"));
}

#[test]
fn decoders_refuse_absent_required_fields() {
    let mut page = corpus("mailpit/list.json");
    page["messages"][0].as_object_mut().unwrap().remove("To");
    assert!(clients::mailpit::decode_page(&page, 0).is_none());
    let mut page = corpus("mailslurper/list.json");
    page["mailItems"][0]
        .as_object_mut()
        .unwrap()
        .remove("toAddresses");
    assert!(clients::mailslurper::decode_page(&page, 0).is_none());
    assert!(
        clients::swarmail::decode_page(&json!({"total": 1, "emails": [{"id": "a"}]}), 0).is_none()
    );
    assert!(
        clients::swarmail::decode_message(&json!({"id": "a", "to": [], "codes": "x"})).is_none()
    );
    assert!(clients::inbucket::decode_page(&json!({"not": "an array"}), 0).is_none());
    assert!(clients::greenmail::decode_page(&json!([{"uid": 1}]), 0).is_none());
    assert!(
        clients::smtp4dev::decode_page(&json!({"rowCount": 0, "results": [], "currentPage": 2}), 0)
            .is_none()
    );
}

#[test]
fn the_mime_reader_refuses_shapes_it_does_not_know() {
    let ok = b"Subject: s\r\nFrom: <a@b>\r\nTo: <c@d>\r\nContent-Type: multipart/alternative; boundary=\"x\"\r\n\r\n--x\r\nContent-Type: text/plain\r\n\r\nhi\r\n--x\r\nContent-Type: text/html\r\nContent-Transfer-Encoding: base64\r\n\r\nPHA+aGk8L3A+\r\n--x--\r\n";
    let p = clients::mime::parse(ok).unwrap();
    assert_eq!(
        (p.text.as_deref(), p.html.as_deref()),
        (Some("hi"), Some("<p>hi</p>"))
    );
    assert_eq!(
        (p.from.as_deref(), p.to.clone()),
        (Some("a@b"), vec!["c@d".to_owned()])
    );
    // No closing delimiter: a cut body.
    let cut = b"Subject: s\r\nContent-Type: multipart/alternative; boundary=\"x\"\r\n\r\n--x\r\nContent-Type: text/plain\r\n\r\nhi\r\n";
    assert!(clients::mime::parse(cut).is_none());
    // Two text parts of one kind, an unknown encoding, an unknown type.
    let two = b"Content-Type: multipart/alternative; boundary=\"x\"\r\n\r\n--x\r\nContent-Type: text/plain\r\n\r\na\r\n--x\r\nContent-Type: text/plain\r\n\r\nb\r\n--x--\r\n";
    assert!(clients::mime::parse(two).is_none());
    assert!(clients::mime::parse(b"Content-Transfer-Encoding: x-uue\r\n\r\nzz").is_none());
    assert!(clients::mime::parse(b"Content-Type: application/pdf\r\n\r\nzz").is_none());
    // A boundary inside a line is not a delimiter: refused, not cut short.
    let mid = b"Content-Type: multipart/alternative; boundary=\"x\"\r\n\r\n--x\r\nContent-Type: text/plain\r\n\r\nsee --x here\r\n--x--\r\n";
    assert!(clients::mime::parse(mid).is_none());
    assert_eq!(clients::escape("a b/é-._~"), "a%20b%2F%C3%A9-._~");
    // A path segment encodes dots too, and refuses an id of dots alone.
    assert_eq!(
        clients::segment("a b/é-._~").unwrap(),
        "a%20b%2F%C3%A9-%2E_~"
    );
    for dots in ["", ".", "..", "..."] {
        assert_eq!(clients::segment(dots), Err(clients::OpError::Malformed));
    }
}

// ---- path segments on the wire (R11) --------------------------------------

/// Read through `get` for each id and return the request targets the
/// responder saw, in order.
async fn targets_seen(ids: &[&str]) -> (Vec<Result<(), clients::OpError>>, Vec<String>) {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let addr = responder(move |p| {
        s.lock().unwrap().push(p.to_owned());
        (404, "{}".into())
    })
    .await;
    let conn = clients::Conn::new(addr);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut out = Vec::new();
    for id in ids {
        let mut spend = Spend::default();
        let r = clients::mailpit::get(&conn, id, deadline, &mut spend).await;
        out.push(match r {
            Err(clients::OpError::Status(404)) => Ok(()),
            Err(e) => Err(e),
            Ok(_) => panic!("a 404 is never a message"),
        });
    }
    let got = seen.lock().unwrap().clone();
    (out, got)
}

#[tokio::test]
async fn a_target_supplied_id_reaches_exactly_its_own_route() {
    // Dots, separators, encoded-looking ids and router-sensitive `-` arrive
    // on the wire as one segment under the intended route.
    let (out, seen) = targets_seen(&["a.b", "a/../b", "%2e%2e", "%2F", "-x-", "a?b#c"]).await;
    assert!(out.iter().all(Result::is_ok), "{out:?}");
    assert_eq!(
        seen,
        [
            "/api/v1/message/a%2Eb",
            "/api/v1/message/a%2F%2E%2E%2Fb",
            "/api/v1/message/%252e%252e",
            "/api/v1/message/%252F",
            "/api/v1/message/-x-",
            "/api/v1/message/a%3Fb%23c",
        ]
    );
    // An id of dots alone has no safe encoding: refused before dispatch,
    // so no request reaches the server at all.
    let (out, seen) = targets_seen(&[".", "..", "..."]).await;
    assert_eq!(out, vec![Err(clients::OpError::Malformed); 3]);
    assert!(seen.is_empty(), "{seen:?}");
}

#[tokio::test]
async fn a_path_the_url_parser_would_rewrite_is_refused_before_dispatch() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let s = seen.clone();
    let addr = responder(move |p| {
        s.lock().unwrap().push(p.to_owned());
        (200, "{}".into())
    })
    .await;
    let conn = clients::Conn::new(addr);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    for path in [
        "/api/v1/message/%2E%2E",
        "/api/v1/message/%2e",
        "/api/v1/message/../x",
        "/api/./x",
        "/a#frag",
    ] {
        let mut spend = Spend::default();
        let e = conn.get_json(path, deadline, &mut spend).await.unwrap_err();
        assert_eq!(
            (e, spend.requests),
            (clients::OpError::Malformed, 0),
            "{path}"
        );
    }
    assert!(seen.lock().unwrap().is_empty());
    assert!(clients::exact_url("http://h", "/a/b?q=1").is_ok());
}

/// A loopback server that declares or streams one body over the bound.
async fn huge_responder(chunked: bool) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut b = [0u8; 4096];
                let _ = s.read(&mut b).await;
                if !chunked {
                    let _ = s
                        .write_all(
                            format!(
                                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n",
                                clients::MAX_RESPONSE_BYTES + 1
                            )
                            .as_bytes(),
                        )
                        .await;
                    return;
                }
                let _ = s
                    .write_all(b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n")
                    .await;
                let pad = vec![b'x'; 64 << 10];
                for _ in 0..(2 * clients::MAX_RESPONSE_BYTES / (64 << 10)) {
                    let mut c = format!("{:x}\r\n", pad.len()).into_bytes();
                    c.extend_from_slice(&pad);
                    c.extend_from_slice(b"\r\n");
                    if s.write_all(&c).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    addr
}

#[tokio::test]
async fn a_response_over_its_bound_is_refused_before_it_is_buffered() {
    for chunked in [false, true] {
        let conn = clients::Conn::new(huge_responder(chunked).await);
        let mut spend = Spend::default();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let e = conn
            .get_bytes("/x", deadline, &mut spend)
            .await
            .unwrap_err();
        assert_eq!(e, clients::OpError::Oversized);
        assert!(
            spend.bytes <= clients::MAX_RESPONSE_BYTES,
            "{}",
            spend.bytes
        );
    }
    // The operation's budget is shared: a scan already near it is refused
    // on its next small reply, before append, with checked arithmetic.
    let conn = clients::Conn::new(responder(|_| (200, "[]".into())).await);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    for start in [clients::MAX_OPERATION_BYTES - 1, u64::MAX] {
        let mut spend = Spend {
            bytes: start,
            requests: 0,
        };
        let e = conn.get_json("/x", deadline, &mut spend).await.unwrap_err();
        assert_eq!((e, spend.bytes), (clients::OpError::Oversized, start));
    }
}

// ---- the scan's completeness rules (T2) -----------------------------------

/// A bounded loopback HTTP/1.1 responder: one canned answer per path, kept
/// alive, so the scan's rules meet real framing. Not a target: it proves the
/// client's refusals, which no real target can be made to provoke.
async fn responder(
    answer: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> std::net::SocketAddr {
    holding_responder(answer, None).await.0
}

/// [`responder`], never answering the `hold`th request (counted from 1
/// across all connections): its connection is held, without a timer, until
/// the client closes it, and the receiver then reports the request's number.
/// So a test learns from the event itself that the client discarded the
/// connection, and nothing the responder does depends on the wall clock.
async fn holding_responder(
    answer: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
    hold: Option<u64>,
) -> (std::net::SocketAddr, tokio::sync::oneshot::Receiver<u64>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let answer = Arc::new(answer);
    let (closed_tx, closed) = tokio::sync::oneshot::channel();
    let closed_tx = Arc::new(std::sync::Mutex::new(Some(closed_tx)));
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let (answer, count, closed_tx) = (answer.clone(), count.clone(), closed_tx.clone());
            tokio::spawn(async move {
                let mut buf = Vec::new();
                loop {
                    let mut chunk = [0u8; 4096];
                    let Ok(n) = s.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..end]).to_string();
                        let len = head
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        if buf.len() < end + 4 + len {
                            break;
                        }
                        let path = head.split(' ').nth(1).unwrap_or("/").to_owned();
                        buf.drain(..end + 4 + len);
                        let (status, body) = answer(&path);
                        let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        if hold == Some(n) {
                            // Unanswered until the client's end closes
                            // (EOF or reset); later bytes are not served.
                            while let Ok(m) = s.read(&mut chunk).await {
                                if m == 0 {
                                    break;
                                }
                            }
                            if let Some(tx) = closed_tx.lock().unwrap().take() {
                                let _ = tx.send(n);
                            }
                            return;
                        }
                        let resp = format!(
                            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                            body.len()
                        );
                        if s.write_all(resp.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                }
            });
        }
    });
    (addr, closed)
}

fn page(start: u64, total: u64, ids: &[&str]) -> String {
    let items: Vec<Value> = ids
        .iter()
        .map(|id| json!({"ID": id, "To": [{"Address": "r@x"}], "Subject": "s"}))
        .collect();
    json!({"start": start, "messages_count": total, "messages": items}).to_string()
}

async fn mailpit_scan(
    answer: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> Result<Vec<clients::Summary>, clients::OpError> {
    let addr = responder(answer).await;
    let conn = clients::Conn::new(addr);
    let mut spend = Spend::default();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    clients::scan(
        &conn,
        clients::Paging::Offset { size: 2 },
        |start, _, size| format!("/p?start={start}&limit={size}"),
        clients::mailpit::decode_page,
        deadline,
        &mut spend,
    )
    .await
}

fn start_of(path: &str) -> u64 {
    path.split("start=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test]
async fn a_scan_pages_to_its_count_and_refuses_every_shortcut() {
    // Complete: 2 + 2 + 1 to a count of 5.
    let ok = mailpit_scan(|p| {
        let s = start_of(p);
        let ids = [["a", "b"], ["c", "d"], ["e", "e"]][(s / 2) as usize];
        (200, page(s, 5, if s == 4 { &ids[..1] } else { &ids[..] }))
    })
    .await
    .unwrap();
    assert_eq!(
        ok.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["a", "b", "c", "d", "e"]
    );
    // A repeated id across pages.
    let w = mailpit_scan(|p| (200, page(start_of(p), 4, &["a", "b"]))).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Incomplete);
    // A count that moves between pages.
    let w = mailpit_scan(|p| {
        let s = start_of(p);
        (
            200,
            page(s, 4 + s, if s == 0 { &["a", "b"] } else { &["c", "d"] }),
        )
    })
    .await;
    assert_eq!(w.unwrap_err(), clients::OpError::Incomplete);
    // A short page before the count.
    let w = mailpit_scan(|p| (200, page(start_of(p), 5, &["a"]))).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Incomplete);
    // A page over its size, a status, a body that is not JSON, the wrong echo.
    let w = mailpit_scan(|p| (200, page(start_of(p), 3, &["a", "b", "c"]))).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Malformed);
    let w = mailpit_scan(|_| (500, "{}".into())).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Status(500));
    let w = mailpit_scan(|_| (200, "{\"start\": 0, \"messages_coun".into())).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Malformed);
    let w = mailpit_scan(|_| (200, page(7, 2, &["a", "b"]))).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Malformed);
    // A count above the largest store.
    let w = mailpit_scan(|p| (200, page(start_of(p), clients::MAX_STORE + 1, &["a", "b"]))).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Incomplete);
    // A redirect is never followed.
    let w = mailpit_scan(|_| (302, String::new())).await;
    assert_eq!(w.unwrap_err(), clients::OpError::Status(302));
}

#[tokio::test]
async fn a_scan_of_the_largest_store_is_within_its_bounds() {
    // A 150,000-message store, 250 per page, as Mailpit pages it: the scan
    // reaches the count within MAX_STORE / 250 + 2 pages and the byte bound.
    let addr = responder(|p| {
        let s = start_of(p);
        let n = (clients::MAX_STORE - s).min(250);
        let ids: Vec<String> = (s..s + n).map(|i| format!("m{i:06}")).collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        (200, page(s, clients::MAX_STORE, &refs))
    })
    .await;
    let conn = clients::Conn::new(addr);
    let mut spend = Spend::default();
    let all = clients::mailpit::all(
        &conn,
        tokio::time::Instant::now() + Duration::from_secs(120),
        &mut spend,
    )
    .await
    .unwrap();
    assert_eq!(all.len() as u64, clients::MAX_STORE);
    assert_eq!(u64::from(spend.requests), clients::MAX_STORE / 250);
    assert!(spend.bytes < clients::MAX_OPERATION_BYTES);
}

#[tokio::test]
async fn an_operation_gets_only_what_remains_of_its_budget() {
    let conn = clients::Conn::new(responder(|_| (200, "[]".into())).await);
    let mut spend = Spend::default();
    let past = tokio::time::Instant::now();
    let w = conn.get_json("/x", past, &mut spend).await.unwrap_err();
    assert_eq!((w, spend.requests), (clients::OpError::Timeout, 0));
}

// ---- the scheduler's accounting (T5) ----------------------------------------

fn sleeper(
    ms: u64,
    outcome: Outcome,
) -> Arc<impl Fn(u32, u32, tokio::time::Instant) -> api::OpFuture + Send + Sync + 'static> {
    Arc::new(move |_w, _i, _d| {
        let outcome = outcome.clone();
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            api::Done::of(outcome, Spend::default())
        }) as api::OpFuture
    })
}

fn clock() -> api::Clock {
    api::Clock(std::time::Instant::now())
}

#[tokio::test(start_paused = true)]
async fn a_cell_stops_assigning_at_its_deadline_and_every_start_ends_once() {
    // Full: 100 quick operations on 8 workers, all started, none unstarted.
    let r = api::run_cell(
        clock(),
        Phase::Measured,
        8,
        100,
        api::CELL_DEADLINE,
        sleeper(1, Outcome::Exact),
    )
    .await;
    assert_eq!((r.started, r.unstarted, r.rows.len()), (100, 0, 100));
    let idx: BTreeSet<u32> = r.rows.iter().map(|x| x.index).collect();
    assert_eq!(idx.len(), 100);
    // Partial: one worker, 100 ms each, a 5 s deadline: 50 started, 50 not.
    let r = api::run_cell(
        clock(),
        Phase::Measured,
        1,
        100,
        api::CELL_DEADLINE,
        sleeper(100, Outcome::Exact),
    )
    .await;
    assert_eq!((r.started, r.unstarted), (50, 50));
    assert!(!r.unresolved);
    // An operation that outlives the deadline is a timeout row, unresolved.
    let r = api::run_cell(
        clock(),
        Phase::Measured,
        2,
        100,
        api::CELL_DEADLINE,
        sleeper(60_000, Outcome::Exact),
    )
    .await;
    assert_eq!((r.started, r.unstarted), (2, 98));
    // The scheduler records the timeouts; whether an effect is unknown is
    // the operation's to say (R1): a read never, a mutation always.
    assert!(r.rows.iter().all(|x| x.outcome == Outcome::Timeout) && !r.unresolved);
    assert!(!r.cancel_unproved);
    // Zero: a zero budget starts nothing.
    let r = api::run_cell(
        clock(),
        Phase::Measured,
        4,
        100,
        Duration::ZERO,
        sleeper(1, Outcome::Exact),
    )
    .await;
    assert_eq!((r.started, r.unstarted, r.rows.len()), (0, 100, 0));
    // Counts are never multiplied by concurrency.
    let r = api::run_cell(
        clock(),
        Phase::Warmup,
        32,
        20,
        api::CELL_DEADLINE,
        sleeper(1, Outcome::Exact),
    )
    .await;
    assert_eq!((r.started, r.rows.len()), (20, 20));
}

// ---- effect-specific cutoffs (R1) -------------------------------------------

#[test]
fn only_a_mutation_left_unresolved_is_uncertain() {
    let failed = |r: &str| Outcome::Failed { reason: r.into() };
    for op in [Operation::Delete, Operation::Clear, Operation::Wait] {
        assert!(api::uncertain(op, &Outcome::Timeout), "{op:?}");
        assert!(api::uncertain(op, &failed("transport")), "{op:?}");
        assert!(api::uncertain(op, &failed("fixture_ambiguous")), "{op:?}");
        // A completed refusal is a known non-effect.
        for known in ["status_404", "status_429", "busy_bridge", "fixture_refused"] {
            assert!(!api::uncertain(op, &failed(known)), "{op:?} {known}");
        }
        assert!(!api::uncertain(op, &Outcome::Exact));
    }
    for op in [
        Operation::List,
        Operation::Search,
        Operation::Get,
        Operation::ExtractCode,
        Operation::ExtractLink,
    ] {
        assert!(!api::uncertain(op, &Outcome::Timeout), "{op:?}");
        assert!(!api::uncertain(op, &failed("transport")), "{op:?}");
    }
}

/// A surface whose operation closures are scripted per operation: `slow`
/// indices sleep past any deadline, `fail` indices end in the given outcome.
#[test]
fn an_acknowledged_wait_is_resolved_however_its_wait_side_ended() {
    // Review finding 2: a wait row whose delivery's 250 was observed is a
    // known effect — the ledger expects the message — whether its wait side
    // timed out or lost its transport; one with no acknowledgement (or none
    // recorded, a row the scheduler dropped) stays uncertain. Other
    // operations are ruled by their outcome alone.
    let row = |op_out: Outcome, ack: Option<u64>, times: bool| api::OpRow {
        phase: Phase::Measured,
        index: 0,
        worker: 0,
        start_ns: 0,
        end_ns: 1,
        outcome: op_out,
        requests: 1,
        bytes: 0,
        wait: times.then_some(api::WaitTimes {
            call_ns: 0,
            send_ns: 0,
            ack_ns: ack,
            return_ns: 1,
            ack_to_return_ns: ack.map(|a| 1 - a as i64),
        }),
    };
    let transport = Outcome::Failed {
        reason: "transport".into(),
    };
    for o in [Outcome::Timeout, transport.clone()] {
        assert!(!api::unresolved(
            Operation::Wait,
            &row(o.clone(), Some(0), true)
        ));
        assert!(api::unresolved(
            Operation::Wait,
            &row(o.clone(), None, true)
        ));
        assert!(api::unresolved(
            Operation::Wait,
            &row(o.clone(), None, false)
        ));
        // A delete or clear carries no wait times: its outcome rules.
        for op in [Operation::Delete, Operation::Clear] {
            assert!(api::unresolved(op, &row(o.clone(), Some(0), true)));
        }
        assert!(!api::unresolved(Operation::List, &row(o, None, false)));
    }
    let ambiguous = Outcome::Failed {
        reason: "fixture_ambiguous".into(),
    };
    assert!(api::unresolved(
        Operation::Wait,
        &row(ambiguous, None, true)
    ));
    let no_time = Outcome::Failed {
        reason: "wait_no_time".into(),
    };
    assert!(!api::unresolved(Operation::Wait, &row(no_time, None, true)));
}

struct Scripted {
    script: fn(Operation, Phase, u32) -> Option<Outcome>,
    phase_of: fn(u32) -> Phase,
    /// The postcheck's listing never answers: it ends only at its `end`.
    postcheck_hangs: bool,
}

impl api::Surfacer for Scripted {
    fn op(
        &self,
        cell: &PlannedCell,
        offset: u32,
    ) -> Arc<dyn Fn(u32, u32, tokio::time::Instant) -> api::OpFuture + Send + Sync> {
        let (op, script) = (cell.operation, self.script);
        let phase = (self.phase_of)(offset);
        Arc::new(move |_w, i, _d| {
            let got = script(op, phase, i);
            Box::pin(async move {
                match got {
                    // In flight past the deadline: the scheduler drops it.
                    None => {
                        tokio::time::sleep(Duration::from_secs(3_600)).await;
                        api::Done::of(Outcome::Exact, Spend::default())
                    }
                    Some(o) => {
                        tokio::time::sleep(Duration::from_millis(7)).await;
                        api::Done::of(o, Spend::default())
                    }
                }
            }) as api::OpFuture
        })
    }

    fn postcheck<'a>(
        &'a self,
        _cell: &'a PlannedCell,
        _rows: &'a mut Vec<api::OpRow>,
        end: Option<tokio::time::Instant>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>
    {
        let hangs = self.postcheck_hangs;
        Box::pin(async move {
            if hangs {
                // A listing that outlives its verification deadline.
                tokio::time::sleep_until(api::verify_deadline(end)).await;
                return Err("postcheck listing: timeout".into());
            }
            Ok(Vec::new())
        })
    }
}

fn cell(op: Operation, c: u32) -> PlannedCell {
    PlannedCell {
        operation: op,
        concurrency: Some(c),
        warmup: api::WARMUP,
        planned: api::MEASURED,
        scope: None,
        ordinal: 0,
    }
}

async fn run_script(
    cells: &[PlannedCell],
    script: fn(Operation, Phase, u32) -> Option<Outcome>,
    deadline: Option<tokio::time::Instant>,
) -> (Vec<api::CellRecord>, Option<String>) {
    let ends = deadline.map(|d| api::Ends {
        cells: d,
        verify: d,
    });
    run_scripted(cells, script, ends, false).await
}

async fn run_scripted(
    cells: &[PlannedCell],
    script: fn(Operation, Phase, u32) -> Option<Outcome>,
    ends: Option<api::Ends>,
    postcheck_hangs: bool,
) -> (Vec<api::CellRecord>, Option<String>) {
    let s = Scripted {
        postcheck_hangs,
        script,
        phase_of: |offset| {
            if offset == 0 {
                Phase::Warmup
            } else {
                Phase::Measured
            }
        },
    };
    let caps = |op| clients::native_capability("swarmail", op);
    api::run_cells(cells, &caps, &s, &api::NoMeter, clock(), ends)
        .await
        .unwrap()
}

/// A surface with the MCP surface's idle gap that logs, on the paused
/// clock, each operation's start and each window bound.
#[derive(Default)]
struct Gapped {
    log: Arc<std::sync::Mutex<Vec<(String, tokio::time::Instant)>>>,
}

impl api::Surfacer for Gapped {
    fn op(
        &self,
        cell: &PlannedCell,
        offset: u32,
    ) -> Arc<dyn Fn(u32, u32, tokio::time::Instant) -> api::OpFuture + Send + Sync> {
        let tag = format!(
            "{} {}",
            cell.name(),
            if offset == 0 { "warmup" } else { "measured" }
        );
        let log = self.log.clone();
        Arc::new(move |_w, _i, _d| {
            log.lock()
                .unwrap()
                .push((tag.clone(), tokio::time::Instant::now()));
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(7)).await;
                api::Done::of(Outcome::Exact, Spend::default())
            }) as api::OpFuture
        })
    }

    fn postcheck<'a>(
        &'a self,
        _cell: &'a PlannedCell,
        _rows: &'a mut Vec<api::OpRow>,
        _end: Option<tokio::time::Instant>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn cell_gap(&self) -> Duration {
        api::MCP_CELL_GAP
    }
}

impl api::Meter for Gapped {
    fn open<'a>(&'a self, cell: &'a str) -> api::MeterFuture<'a> {
        let now = tokio::time::Instant::now();
        self.log.lock().unwrap().push((format!("{cell} open"), now));
        Box::pin(async { Ok(()) })
    }
    fn close<'a>(&'a self, cell: &'a str) -> api::MeterFuture<'a> {
        let now = tokio::time::Instant::now();
        self.log
            .lock()
            .unwrap()
            .push((format!("{cell} close"), now));
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test(start_paused = true)]
async fn the_mcp_surface_idles_a_second_before_each_cell_outside_its_windows() {
    // Operator ruling 2026-11-18: before each MCP cell, a second with no
    // call, before the warmup and outside every window, so each cell starts
    // in a fresh rate window; the chunk's deadline bounds it.
    let cells = [cell(Operation::List, 1), cell(Operation::Get, 8)];
    let caps = |op| clients::mcp_capability("swarmail", op);
    let g = Gapped::default();
    let start = tokio::time::Instant::now();
    let (recs, abandoned) = api::run_cells(&cells, &caps, &g, &g, clock(), None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    assert!(recs.iter().all(|r| r.status == api::CellStatus::Measured));
    let log = g.log.lock().unwrap().clone();
    let first = |tag: &str| log.iter().find(|(t, _)| t == tag).unwrap().1;
    let last = |prefix: &str| {
        log.iter()
            .rev()
            .find(|(t, _)| t.starts_with(prefix))
            .unwrap()
            .1
    };
    // The first cell's first call comes a full gap after the start.
    assert_eq!(first("list.c1 warmup") - start, api::MCP_CELL_GAP);
    // The second's comes a full gap after the first cell's window closed:
    // nothing ran in between, and the gap is in no window.
    let closed = first("list.c1 close");
    assert!(last("list.c1 measured") <= closed);
    assert_eq!(first("get.c8 warmup") - closed, api::MCP_CELL_GAP);
    assert!(first("get.c8 open") > first("get.c8 warmup"));
    // Each window opens after its warmup and holds only measured calls.
    for c in ["list.c1", "get.c8"] {
        let (open, close) = (first(&format!("{c} open")), first(&format!("{c} close")));
        assert!(last(&format!("{c} warmup")) < open);
        assert!(
            log.iter()
                .filter(|(_, at)| *at > open && *at < close)
                .all(|(t, _)| t == &format!("{c} measured"))
        );
    }
    // A gap the chunk's deadline cuts: the cell is cut, never started.
    let g = Gapped::default();
    let end = tokio::time::Instant::now() + Duration::from_millis(400);
    let ends = api::Ends {
        cells: end,
        verify: end,
    };
    let (recs, _) = api::run_cells(&cells, &caps, &g, &g, clock(), Some(ends))
        .await
        .unwrap();
    assert!(
        recs.iter()
            .all(|r| r.blocked.as_deref() == Some(api::CHUNK_DEADLINE) && r.warmup.is_none())
    );
    assert!(g.log.lock().unwrap().is_empty(), "no call, no window");
    assert_eq!(tokio::time::Instant::now(), end);
    // A surface without a gap starts its first cell at once.
    let s = Scripted {
        script: |_, _, _| Some(Outcome::Exact),
        phase_of: |_| Phase::Warmup,
        postcheck_hangs: false,
    };
    assert_eq!(api::Surfacer::cell_gap(&s), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn a_read_in_flight_at_the_deadline_times_out_and_later_cells_still_measure() {
    // list.c1: measured index 37 never answers; at the five-second deadline
    // it is a timeout row, the rest of the queue untaken. search.c1 after it
    // measures normally and the launch is not abandoned.
    let cells = [cell(Operation::List, 1), cell(Operation::Search, 1)];
    let (recs, abandoned) = run_script(
        &cells,
        |op, phase, i| {
            if op == Operation::List && phase == Phase::Measured && i == 37 {
                None
            } else {
                Some(Outcome::Exact)
            }
        },
        None,
    )
    .await;
    assert_eq!(abandoned, None);
    let list = &recs[0].measured;
    assert_eq!(recs[0].status, api::CellStatus::Measured);
    assert_eq!((list.started, list.unstarted), (38, 62));
    assert_eq!(list.rows.last().unwrap().outcome, Outcome::Timeout);
    assert!(!list.unresolved && !list.cancel_unproved);
    let search = &recs[1];
    assert_eq!(search.status, api::CellStatus::Measured);
    assert_eq!(
        (search.measured.started, search.measured.unstarted),
        (100, 0)
    );
}

#[tokio::test(start_paused = true)]
async fn a_failed_read_warmup_blocks_only_its_own_cell() {
    let cells = [cell(Operation::Get, 8), cell(Operation::Get, 32)];
    let (recs, abandoned) = run_script(
        &cells,
        |_, phase, i| match (phase, i) {
            // get.c8's warmup has a slow read; get.c32's warmup is clean.
            (Phase::Warmup, 3) => None,
            _ => Some(Outcome::Exact),
        },
        None,
    )
    .await;
    // Both warmups saw index 3 hang, so both are blocked — and neither
    // abandoned the launch: a read never does.
    assert_eq!(abandoned, None);
    assert!(
        recs.iter()
            .all(|r| r.blocked.as_deref() == Some("warmup_failed"))
    );
    // A read warmup that merely fails blocks only itself.
    let cells = [cell(Operation::List, 1), cell(Operation::Search, 1)];
    let (recs, abandoned) = run_script(
        &cells,
        |op, phase, _| match (op, phase) {
            (Operation::List, Phase::Warmup) => Some(Outcome::Failed {
                reason: "transport".into(),
            }),
            _ => Some(Outcome::Exact),
        },
        None,
    )
    .await;
    assert_eq!(abandoned, None);
    assert_eq!(recs[0].blocked.as_deref(), Some("warmup_failed"));
    assert_eq!(recs[1].status, api::CellStatus::Measured);
}

#[tokio::test(start_paused = true)]
async fn an_unresolved_mutation_blocks_every_later_cell_and_abandons_the_launch() {
    let later = [
        cell(Operation::Clear, 1),
        PlannedCell {
            operation: Operation::Clear,
            concurrency: None,
            warmup: 0,
            planned: 1,
            scope: Some(ClearScope::All),
            ordinal: 0,
        },
    ];
    // A delete timing out in its measured phase, a delete losing its
    // transport after dispatch, a clear doing so in warmup, a wait timing out.
    type Script = fn(Operation, Phase, u32) -> Option<Outcome>;
    let cases: [(Operation, Script, &str); 4] = [
        (
            Operation::Delete,
            |op, ph, i| {
                (op != Operation::Delete || ph != Phase::Measured || i != 50)
                    .then_some(Outcome::Exact)
            },
            "measured",
        ),
        (
            Operation::Delete,
            |op, ph, i| {
                Some(
                    if op == Operation::Delete && ph == Phase::Measured && i == 2 {
                        Outcome::Failed {
                            reason: "transport".into(),
                        }
                    } else {
                        Outcome::Exact
                    },
                )
            },
            "measured",
        ),
        (
            Operation::Clear,
            |op, ph, i| {
                Some(if op == Operation::Clear && ph == Phase::Warmup && i == 0 {
                    Outcome::Failed {
                        reason: "transport".into(),
                    }
                } else {
                    Outcome::Exact
                })
            },
            "warmup",
        ),
        (
            Operation::Wait,
            |op, ph, i| {
                (op != Operation::Wait || ph != Phase::Measured || i != 0).then_some(Outcome::Exact)
            },
            "measured",
        ),
    ];
    for (op, script, phase) in cases {
        let mut cells = vec![cell(op, 8)];
        cells.extend(later.iter().cloned());
        let (recs, abandoned) = run_script(&cells, script, None).await;
        let why = abandoned.unwrap_or_else(|| panic!("{op:?} must abandon"));
        assert!(
            why.contains(phase) && why.contains("effect unknown"),
            "{why}"
        );
        // No later cell runs — the global clear included — and each names why.
        for r in &recs[1..] {
            assert_eq!(r.status, api::CellStatus::Blocked);
            assert_eq!(r.blocked.as_deref(), Some(why.as_str()));
            assert_eq!(r.measured.started, 0);
        }
        // The uncertain cell keeps its started rows, the failure among them.
        let first = &recs[0];
        let ran = if phase == "warmup" {
            first.warmup.as_ref().unwrap()
        } else {
            &first.measured
        };
        assert!(ran.unresolved && ran.started >= 1);
    }
    // A completed refusal on a delete is a failure row, not uncertainty.
    let mut cells = vec![cell(Operation::Delete, 1)];
    cells.extend(later.iter().cloned());
    let (recs, abandoned) = run_script(
        &cells,
        |op, ph, i| {
            Some(
                if op == Operation::Delete && ph == Phase::Measured && i == 9 {
                    Outcome::Failed {
                        reason: "status_404".into(),
                    }
                } else {
                    Outcome::Exact
                },
            )
        },
        None,
    )
    .await;
    assert_eq!(abandoned, None);
    assert!(recs.iter().all(|r| r.status == api::CellStatus::Measured));
}

#[tokio::test(start_paused = true)]
async fn the_chunk_deadline_bounds_a_cell_and_blocks_the_rest() {
    // Half a second of chunk left: list.c1's phases get only what remains, and
    // the cells after the deadline are blocked by it, not by a failure.
    let cells = [
        cell(Operation::List, 1),
        cell(Operation::Search, 1),
        cell(Operation::Get, 1),
    ];
    let end = tokio::time::Instant::now() + Duration::from_millis(500);
    let (recs, abandoned) = run_script(&cells, |_, _, _| Some(Outcome::Exact), Some(end)).await;
    assert_eq!(abandoned, None);
    // 20 warmups at 7 ms leave 360 ms: the measured phase stops there.
    assert_eq!(recs[0].status, api::CellStatus::Measured);
    let m = &recs[0].measured;
    assert_eq!((m.started, m.unstarted), (52, 48));
    for r in &recs[1..] {
        assert_eq!(r.blocked.as_deref(), Some("chunk_deadline"));
    }
}

#[tokio::test(start_paused = true)]
async fn a_warmup_cut_or_ended_by_the_chunk_deadline_is_a_cut_cell() {
    // Cut mid-warmup (100 ms of 140): not "warmup_failed". Ended exactly at
    // it (20 warmups at 7 ms): not a measured cell that started nothing.
    for ms in [100, 140] {
        let cells = [cell(Operation::List, 1), cell(Operation::Search, 1)];
        let end = tokio::time::Instant::now() + Duration::from_millis(ms);
        let (recs, abandoned) = run_script(&cells, |_, _, _| Some(Outcome::Exact), Some(end)).await;
        assert_eq!(abandoned, None, "{ms}");
        assert_eq!(recs[0].status, api::CellStatus::Blocked, "{ms}");
        assert_eq!(recs[0].blocked.as_deref(), Some("chunk_deadline"), "{ms}");
        assert!(recs[0].warmup.is_some(), "{ms}");
        assert_eq!(recs[0].measured.started, 0, "{ms}");
        assert_eq!(recs[1].blocked.as_deref(), Some("chunk_deadline"), "{ms}");
        // The cut is the harness's stopping reason on both surfaces.
        assert!(
            recs.iter()
                .all(|r| !api_chunk::blocked_is_failure(r.blocked.as_deref()))
        );
    }
    // An unknown effect in a warmup still abandons, deadline or not.
    let cells = [cell(Operation::Delete, 1), cell(Operation::List, 1)];
    let end = tokio::time::Instant::now() + Duration::from_millis(100);
    let (recs, abandoned) = run_script(
        &cells,
        |op, _, i| (op == Operation::Delete && i == 0).then_some(Outcome::Timeout),
        Some(end),
    )
    .await;
    assert!(abandoned.is_some());
    assert_eq!(recs[0].blocked.as_deref(), Some("warmup_failed"));
}

#[tokio::test(start_paused = true)]
async fn a_postcheck_cut_by_the_chunk_deadline_is_bounded_and_unverified() {
    // The delete cell measures fully, then its postcheck's listing never
    // answers: it stops at the verification end, the cell's successes are
    // unverified failures (not uncertainty), and run_cells returns — no
    // infrastructure error, no teardown — with later cells cut.
    let cells = [cell(Operation::Delete, 1), cell(Operation::List, 1)];
    let start = tokio::time::Instant::now();
    let ends = api::Ends {
        cells: start + Duration::from_secs(30),
        verify: start + Duration::from_secs(40),
    };
    let (recs, abandoned) =
        run_scripted(&cells, |_, _, _| Some(Outcome::Exact), Some(ends), true).await;
    assert_eq!(tokio::time::Instant::now(), ends.verify);
    assert_eq!(abandoned, None);
    let d = &recs[0];
    assert_eq!(d.status, api::CellStatus::Measured);
    assert_eq!(d.measured.started, api::MEASURED);
    let cut = Outcome::Failed {
        reason: api::POSTCHECK_CUT.into(),
    };
    assert!(d.measured.rows.iter().all(|r| r.outcome == cut));
    assert!(
        d.warmup
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .all(|r| r.outcome == cut)
    );
    assert!(!api::uncertain(Operation::Delete, &cut));
    assert_eq!(d.postcheck.len(), 1);
    assert!(
        d.postcheck[0].contains(api::POSTCHECK_CUT),
        "{:?}",
        d.postcheck
    );
    assert_eq!(recs[1].blocked.as_deref(), Some("chunk_deadline"));
    // A postcheck that fails before the end is still infrastructure.
    let s = Scripted {
        postcheck_hangs: true,
        script: |_, _, _| Some(Outcome::Exact),
        phase_of: |o| {
            if o == 0 {
                Phase::Warmup
            } else {
                Phase::Measured
            }
        },
    };
    let caps = |op| clients::native_capability("swarmail", op);
    // Without an end the hanging listing runs to its own bound: an error.
    let err = api::run_cells(&cells[..1], &caps, &s, &api::NoMeter, clock(), None)
        .await
        .unwrap_err();
    assert!(err.contains("postcheck listing"), "{err}");
    // The bound itself: never past `end`, never past its own 120 s.
    let now = tokio::time::Instant::now();
    assert_eq!(api::verify_deadline(None), now + api::VERIFY_DEADLINE);
    assert_eq!(
        api::verify_deadline(Some(now + Duration::from_secs(1))),
        now + Duration::from_secs(1)
    );
    assert_eq!(
        api::verify_deadline(Some(now + Duration::from_secs(999))),
        now + api::VERIFY_DEADLINE
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_worker_that_does_not_return_leaves_its_start_and_an_unproved_cancellation() {
    // The operation blocks its thread past the deadline and the bound: it
    // cannot be dropped in time. Its start is still a terminal timeout row,
    // and the phase says local cancellation was not proved.
    let op = Arc::new(|_w: u32, i: u32, _d: tokio::time::Instant| {
        Box::pin(async move {
            if i == 0 {
                // Far past the deadline and bound below, so a loaded host
                // still sees the bound expire first.
                std::thread::sleep(Duration::from_secs(5));
            }
            api::Done::of(Outcome::Exact, Spend::default())
        }) as api::OpFuture
    });
    let r = api::run_cell_bounded(
        clock(),
        Phase::Measured,
        1,
        5,
        // A budget a loaded host still starts index 0 within, and a bound
        // that ends long before the blocked operation would.
        Duration::from_secs(1),
        Duration::from_millis(500),
        op,
    )
    .await;
    assert!(r.cancel_unproved);
    assert_eq!((r.started, r.unstarted), (1, 4));
    assert_eq!(r.rows[0].outcome, Outcome::Timeout);
    // A phase whose cancellation was not proved abandons the launch even
    // for a read; a read's timeout alone never does.
    let mut ran = r.clone();
    let why = api::phase_verdict(&mut ran, Operation::List, "measured", "list.c1").unwrap();
    assert!(why.contains("cancellation") && !ran.unresolved, "{why}");
    ran.cancel_unproved = false;
    assert_eq!(
        api::phase_verdict(&mut ran, Operation::List, "measured", "list.c1"),
        None
    );
    let why = api::phase_verdict(&mut ran, Operation::Delete, "measured", "delete.c1").unwrap();
    assert!(why.contains("effect unknown") && ran.unresolved, "{why}");
}

#[test]
fn a_chunk_keeps_the_ledger_and_the_global_clear_their_own_reserves() {
    let start = tokio::time::Instant::now();
    let end = start + Duration::from_secs(470);
    // With a global clear: head, then the ledger, then the clear's reserve.
    let s = api::stops(start, end, true);
    assert_eq!(
        s.tail,
        api::Ends {
            cells: end,
            verify: end
        }
    );
    assert_eq!(s.ledger, end - api::CLEAR_RESERVE);
    assert_eq!(
        s.head,
        api::Ends {
            cells: end - api::CLEAR_RESERVE - api::LEDGER_RESERVE,
            verify: end - api::CLEAR_RESERVE,
        }
    );
    // The clear's reserve holds its measured call and more.
    assert!(api::CLEAR_RESERVE > api::CELL_DEADLINE);
    // Without one, the ledger owns the time up to the end.
    let s = api::stops(start, end, false);
    assert_eq!(s.ledger, end);
    assert_eq!(s.head.cells, end - api::LEDGER_RESERVE);
    // A chunk shorter than its reserves stops everything at its start,
    // never before it.
    let short = start + Duration::from_secs(10);
    let s = api::stops(start, short, true);
    assert_eq!(
        (s.head.cells, s.head.verify, s.ledger),
        (start, start, start)
    );
    assert_eq!(s.tail.cells, short);
    assert_eq!(api::stops(short, start, true).tail.cells, short);
}

#[tokio::test(start_paused = true)]
async fn the_global_clear_still_measures_after_a_ledger_that_used_its_whole_reserve() {
    // The producer path both drivers take (AC6): the head runs until its
    // stop, the batch ledger takes its whole deadline — as a 100,000-message
    // ledger may — and the one global clear after it still measures, because
    // the tail owns the clear's reserve. With the head's stops reused for
    // the tail, as before, the same clear is cut by the chunk deadline.
    let start = tokio::time::Instant::now();
    let end = start + Duration::from_secs(470);
    let stops = api::stops(start, end, true);
    let clear = PlannedCell {
        operation: Operation::Clear,
        concurrency: None,
        warmup: 0,
        planned: 1,
        scope: Some(ClearScope::All),
        ordinal: 0,
    };
    let exact: fn(Operation, Phase, u32) -> Option<Outcome> = |_, _, _| Some(Outcome::Exact);
    tokio::time::sleep_until(stops.head.cells).await;
    let (head, abandoned) =
        run_scripted(&[cell(Operation::List, 1)], exact, Some(stops.head), false).await;
    assert_eq!(abandoned, None);
    assert_eq!(head[0].blocked.as_deref(), Some(api::CHUNK_DEADLINE));
    // The ledger runs to its own verification deadline.
    tokio::time::sleep_until(api::verify_deadline(Some(stops.ledger))).await;
    assert_eq!(tokio::time::Instant::now(), stops.ledger);
    let (tail, abandoned) =
        run_scripted(std::slice::from_ref(&clear), exact, Some(stops.tail), false).await;
    assert_eq!(abandoned, None);
    assert_eq!(tail[0].status, api::CellStatus::Measured, "{:?}", tail[0]);
    assert_eq!(
        (tail[0].measured.started, tail[0].measured.unstarted),
        (1, 0)
    );
    assert_eq!(tail[0].measured.rows[0].outcome, Outcome::Exact);
    assert!(tail[0].postcheck.is_empty(), "{:?}", tail[0].postcheck);
    // The head's stops reused for the tail cut the same clear.
    let (cut, _) = run_scripted(std::slice::from_ref(&clear), exact, Some(stops.head), false).await;
    assert_eq!(cut[0].blocked.as_deref(), Some(api::CHUNK_DEADLINE));
}

/// A surface whose first measured operation blocks its worker's thread, so
/// the scheduler cannot drop it in time.
struct Unyielding;

impl api::Surfacer for Unyielding {
    fn op(
        &self,
        _cell: &PlannedCell,
        offset: u32,
    ) -> Arc<dyn Fn(u32, u32, tokio::time::Instant) -> api::OpFuture + Send + Sync> {
        Arc::new(move |_w, i, _d| {
            Box::pin(async move {
                if offset > 0 && i == 0 {
                    // Past the cell's three seconds and the five-second
                    // cancellation bound, on any host.
                    std::thread::sleep(Duration::from_secs(10));
                }
                api::Done::of(Outcome::Exact, Spend::default())
            }) as api::OpFuture
        })
    }

    fn postcheck<'a>(
        &'a self,
        _cell: &'a PlannedCell,
        _rows: &'a mut Vec<api::OpRow>,
        _end: Option<tokio::time::Instant>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unproved_cancellation_leaves_the_chunk_incomplete_on_both_surfaces() {
    // The whole producer path short of the sealing I/O (AC5): the real
    // scheduler marks the phase, every later cell is blocked by the
    // abandonment, the record accounts — as the record line the MCP holder
    // reads, too — and the seal's refusal makes it incomplete (exit 2,
    // unsealed), not a measured workload failure (exit 1).
    for surface in [Surface::Native, Surface::Mcp] {
        let chunk = runner::Chunk::Api {
            surface,
            target: TargetKind::Mailpit,
            repetition: 0,
            fill: 0,
        };
        let cells = api_chunk::cells(TargetKind::Mailpit, surface);
        let caps = |op| clients::capability("mailpit", surface, op);
        let now = tokio::time::Instant::now();
        let ends = api::Ends {
            cells: now + Duration::from_secs(3),
            verify: now + Duration::from_secs(3),
        };
        let (recs, abandoned) = api::run_cells(
            &cells,
            &caps,
            &Unyielding,
            &api::NoMeter,
            clock(),
            Some(ends),
        )
        .await
        .unwrap();
        let why = abandoned.clone().expect("the launch is abandoned");
        assert!(why.contains("cancellation"), "{why}");
        assert!(recs[0].measured.cancel_unproved, "{surface:?}");
        assert!(
            recs[1..]
                .iter()
                .all(|r| r.blocked.as_deref() == Some(why.as_str())),
            "{surface:?}"
        );
        let mut rec = record_for(&chunk);
        rec.fill.reached = true;
        rec.cells = recs;
        rec.abandoned = abandoned;
        if surface == Surface::Mcp {
            // The brackets the eval runner recorded around the one window.
            rec.control = [("idle_fill", None), ("window_start", Some("list.c1"))]
                .into_iter()
                .chain([("window_end", Some("list.c1"))])
                .enumerate()
                .map(|(n, (cmd, id))| api::Bracket {
                    cmd: cmd.into(),
                    id: id.map(Into::into),
                    sent_ns: 2 * n as u64,
                    received_ns: 2 * n as u64 + 1,
                    owner_t_ns: n as u64,
                })
                .collect();
        }
        let line: api::ApiRecord =
            serde_json::from_slice(&serde_json::to_vec(&rec).unwrap()).unwrap();
        for r in [&rec, &line] {
            api_chunk::check_record(r, &chunk).unwrap();
            let e = api_chunk::incomplete(r).expect("an incomplete chunk");
            assert!(e.contains("list.c1") && e.contains("incomplete"), "{e}");
        }
        // The same record with its cancellation proved is a measured one.
        let mut proved = rec.clone();
        proved.cells[0].measured.cancel_unproved = false;
        assert_eq!(api_chunk::incomplete(&proved), None);
        let mut warm = proved.clone();
        warm.cells[0].warmup.as_mut().unwrap().cancel_unproved = true;
        assert!(api_chunk::incomplete(&warm).is_some());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_native_surface_keeps_measuring_after_a_read_times_out() {
    // The producer path (R1): the real native surface over Mailpit's client,
    // against a loopback server that never answers one measured list
    // request. That list is a timeout row at the cell's five-second
    // deadline, its connection is discarded with it, the next cell measures
    // (the dropped read's own client included), and nothing is abandoned.
    let reads: Vec<api::Authored> = (0..api::K)
        .map(|n| {
            let mut a = api::author(7, api::Role::Read, n, &api::read_scope());
            a.id = Some(format!("r{n}"));
            a
        })
        .collect();
    let body = {
        let items: Vec<Value> = reads
            .iter()
            .map(
                |a| json!({"ID": a.id, "To": [{"Address": a.scope.address}], "Subject": a.subject}),
            )
            .collect();
        json!({"start": 0, "messages_count": items.len(), "messages": items}).to_string()
    };
    // Request 1..20 are list.c1's warmup; 31 is its 11th measured call. It
    // is held until the client closes its connection, by no timer, so no
    // answer from the first cell can arrive during the second.
    let (addr, discarded) = holding_responder(move |_| (200, body.clone()), Some(31)).await;
    let surface = api::NativeSurface {
        clients: Arc::new(
            (0..8)
                .map(|_| clients::Client::new("mailpit", addr).unwrap())
                .collect(),
        ),
        fixtures: Arc::new(reads),
    };
    let caps = |op| clients::native_capability("mailpit", op);
    // The second cell is the production list.c8 without its warmup: whether
    // twenty loopback calls all fit in a production five-second warmup on a
    // loaded host is not this property, and a warmup's own timeout blocks a
    // cell by design (`a_failed_read_warmup_blocks_only_its_own_cell`).
    let next = PlannedCell {
        warmup: 0,
        ..cell(Operation::List, 8)
    };
    let cells = [cell(Operation::List, 1), next];
    let (recs, abandoned) = api::run_cells(&cells, &caps, &surface, &api::NoMeter, clock(), None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    let first = &recs[0].measured;
    assert_eq!(recs[0].status, api::CellStatus::Measured);
    assert_eq!(
        first.started,
        11,
        "{:?}",
        first
            .rows
            .iter()
            .map(|r| r.outcome.class())
            .collect::<Vec<_>>()
    );
    assert_eq!(first.rows[10].outcome, Outcome::Timeout);
    assert!(first.rows[..10].iter().all(|r| r.outcome == Outcome::Exact));
    assert!(!first.unresolved && !first.cancel_unproved);
    // The dropped read took its connection with it: the server saw the
    // client close the held request's connection (an event, awaited under a
    // bound only so a regression fails instead of hanging).
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(30), discarded)
            .await
            .expect("the timed-out read's connection was never discarded")
            .unwrap(),
        31
    );
    // The later cell measures. How many of its 100 fit in its five seconds
    // is the host's business, not this property's; that the dropped read's
    // client (worker 0) answers exactly again is.
    let second = &recs[1];
    assert_eq!(
        second.status,
        api::CellStatus::Measured,
        "{:?}",
        second.blocked
    );
    assert!(second.warmup.is_none());
    let rows = &second.measured.rows;
    assert!(second.measured.started > 0);
    assert!(
        rows.iter()
            .all(|r| matches!(r.outcome, Outcome::Exact | Outcome::Timeout)),
        "{:?}",
        rows.iter().map(|r| r.outcome.class()).collect::<Vec<_>>()
    );
    assert!(
        rows.iter()
            .any(|r| r.worker == 0 && r.outcome == Outcome::Exact),
        "{:?}",
        rows.iter()
            .map(|r| (r.worker, r.outcome.class()))
            .collect::<Vec<_>>()
    );
    assert!(!second.measured.unresolved && !second.measured.cancel_unproved);
}

#[test]
fn statistics_are_nearest_rank_over_exact_successes_only() {
    let row = |i: u32, d: u64, outcome: Outcome| api::OpRow {
        phase: Phase::Measured,
        index: i,
        worker: 0,
        start_ns: 0,
        end_ns: d,
        outcome,
        requests: 1,
        bytes: 0,
        wait: None,
    };
    let mut rows: Vec<api::OpRow> = (0..100)
        .map(|i| row(i, u64::from(i + 1) * 10, Outcome::Exact))
        .collect();
    rows[99].outcome = Outcome::InexactFidelity {
        detail: "text".into(),
    };
    rows[98].outcome = Outcome::Failed {
        reason: "transport".into(),
    };
    let ran = api::Ran {
        rows,
        planned: 100,
        started: 100,
        unstarted: 0,
        start_ns: 0,
        end_ns: 2_000_000_000,
        unresolved: false,
        cancel_unproved: false,
    };
    let s = api::stats(&ran, Some(5_000));
    assert_eq!((s.exact, s.inexact_fidelity, s.failed), (98, 1, 1));
    assert_eq!(
        (s.p50_ns, s.p95_ns, s.p99_ns),
        (Some(490), Some(940), Some(980))
    );
    assert_eq!(s.inexact_ns, vec![1000]);
    assert_eq!(s.exact_per_second, Some(49.0));
    assert_eq!(s.cpu_us_per_started, Some(50.0));
    // Nothing exact: null quantiles. Nothing started: null CPU per op.
    let none = api::Ran {
        planned: 100,
        unstarted: 100,
        ..Default::default()
    };
    let s = api::stats(&none, Some(7));
    assert_eq!((s.p50_ns, s.cpu_us_per_started, s.exact), (None, None, 0));
    assert_eq!(api::nearest_rank(&[5], 99), Some(5));
}

#[test]
fn a_wait_rows_latency_ends_at_its_return_not_its_quit() {
    // Review low (AC7/AC9): a future wait's row ends after its bounded
    // SMTP QUIT; its latency is the waiting client's return.
    let wait = |ret: u64| api::WaitTimes {
        call_ns: 0,
        send_ns: 1,
        ack_ns: Some(5),
        return_ns: ret,
        ack_to_return_ns: Some(ret as i64 - 5),
    };
    let rows: Vec<api::OpRow> = (0..4u32)
        .map(|i| api::OpRow {
            phase: Phase::Measured,
            index: i,
            worker: 0,
            start_ns: 0,
            end_ns: 2_000_000_000,
            outcome: Outcome::Exact,
            requests: 1,
            bytes: 0,
            wait: Some(wait(u64::from(i + 1) * 100)),
        })
        .collect();
    let ran = api::Ran {
        rows,
        planned: 4,
        started: 4,
        unstarted: 0,
        start_ns: 0,
        end_ns: 2_000_000_000,
        unresolved: false,
        cancel_unproved: false,
    };
    let s = api::stats(&ran, None);
    assert_eq!(
        (s.p50_ns, s.p95_ns, s.p99_ns),
        (Some(200), Some(400), Some(400))
    );
}

// ---- exactness (T3) ---------------------------------------------------------

fn summary(id: &str, to: &str, subject: &str) -> clients::Summary {
    clients::Summary {
        id: id.into(),
        to: vec![to.into()],
        subject: subject.into(),
    }
}

#[test]
fn list_and_search_are_exact_or_wrong() {
    let want: BTreeSet<String> = ["a", "b"].iter().map(|s| (*s).to_owned()).collect();
    let ok = vec![summary("a", "r@x", "s"), summary("b", "r@x", "s")];
    assert_eq!(api::check_list(Ok(ok.clone()), &want), Outcome::Exact);
    let dup = vec![summary("a", "r@x", "s"), summary("a", "r@x", "s")];
    assert!(
        matches!(api::check_list(Ok(dup), &want), Outcome::WrongResult { detail } if detail.contains("duplicate"))
    );
    let missing = vec![summary("a", "r@x", "s")];
    assert!(matches!(
        api::check_list(Ok(missing), &want),
        Outcome::WrongResult { .. }
    ));
    let extra = vec![
        summary("a", "r@x", "s"),
        summary("b", "r@x", "s"),
        summary("c", "r@x", "s"),
    ];
    assert!(matches!(
        api::check_list(Ok(extra), &want),
        Outcome::WrongResult { .. }
    ));
    assert_eq!(
        api::check_list(Err(clients::OpError::Timeout), &want),
        Outcome::Timeout
    );
    assert_eq!(
        api::check_search(Ok(vec![summary("a", "r@x", "s")]), "a"),
        Outcome::Exact
    );
    assert!(matches!(
        api::check_search(Ok(ok), "a"),
        Outcome::WrongResult { .. }
    ));
    // The exact recipient filter drops a look-alike a substring filter keeps.
    let hits = vec![
        summary("a", "apiread00x@api.mail-compare.invalid", "s"),
        summary("b", "xapiread00x@api.mail-compare.invalid", "s"),
    ];
    let kept = clients::exactly_to(hits, "apiread00x@api.mail-compare.invalid");
    assert_eq!(kept.len(), 1);
}

fn fetched_from(a: &api::Authored) -> clients::Fetched {
    clients::Fetched {
        id: "x".into(),
        subject: Some(a.subject.clone()),
        from: Some(a.from.clone()),
        to: vec![a.scope.address.clone()],
        text: Some(a.text.replace('\n', "\r\n")),
        html: Some(a.html.clone()),
        codes: Some(vec![a.code.clone()]),
        links: Some(vec![a.link.clone(), a.link.clone()]),
    }
}

#[test]
fn mailslurpers_rewritten_single_body_is_inexact_and_nothing_else_is() {
    // Operator ruling 2026-11-18: a MailSlurper answer that kept only the
    // multipart's HTML part, with `rel="nofollow"` added to its anchor (the
    // host's bytes), is inexact_fidelity; the extractors read the kept HTML
    // and their right answers are inexact too. Nothing else moves.
    let a = api::author(5, api::Role::Read, 0, &api::read_scope());
    let kept_html = a.html.replace("\">", "\" rel=\"nofollow\">");
    assert_ne!(kept_html, a.html, "the fixture has an anchor");
    assert_eq!(api::mailslurper_unrewritten(&kept_html), a.html);
    let kept = clients::Fetched {
        text: None,
        html: Some(kept_html.clone()),
        ..fetched_from(&a)
    };
    let inexact = |d: &str| Outcome::InexactFidelity { detail: d.into() };
    let get = |family, f: &clients::Fetched| api::check_get(family, Ok(f.clone()), &a);
    assert_eq!(
        get(Family::Mailslurper, &kept),
        inexact("html rewritten, text absent")
    );
    // The single body unrewritten keeps the plain detail.
    let plain = clients::Fetched {
        html: Some(a.html.clone()),
        ..kept.clone()
    };
    assert_eq!(get(Family::Mailslurper, &plain), inexact("text absent"));
    for op in [Operation::ExtractCode, Operation::ExtractLink] {
        let want = if op == Operation::ExtractCode {
            &a.code
        } else {
            &a.link
        };
        let got = api::check_extract(Family::Mailslurper, Ok(kept.clone()), op, want);
        assert_eq!(got, inexact("text absent"), "{op:?}");
        // A wrong value read from the kept HTML is still wrong.
        let got = api::check_extract(Family::Mailslurper, Ok(kept.clone()), op, "999999");
        assert!(
            matches!(got, Outcome::WrongResult { .. }),
            "{op:?}: {got:?}"
        );
        // Another family's HTML-only answer gets no such reading.
        let got = api::check_extract(Family::Mailpit, Ok(kept.clone()), op, want);
        assert_eq!(
            got,
            Outcome::WrongResult {
                detail: "no value".into()
            },
            "{op:?}"
        );
        // A text part is read as ever, and exactly.
        let full = fetched_from(&a);
        assert_eq!(
            api::check_extract(Family::Mailslurper, Ok(full), op, want),
            Outcome::Exact
        );
    }
    // The rewrite is MailSlurper's alone; a family that rewrote would be wrong.
    assert!(matches!(
        get(Family::Mailpit, &kept),
        Outcome::WrongResult { .. }
    ));
    // Any other change to the kept HTML, another attribute or another text,
    // and any header mismatch beside the rewrite, stays wrong_result.
    for html in [
        a.html.replace("\">", "\" rel=\"noopener\">"),
        kept_html.replace("Hello", "Hi"),
    ] {
        let f = clients::Fetched {
            html: Some(html.clone()),
            ..kept.clone()
        };
        assert!(
            matches!(get(Family::Mailslurper, &f), Outcome::WrongResult { .. }),
            "{html}"
        );
    }
    let wrong_subject = clients::Fetched {
        subject: Some("other".into()),
        ..kept.clone()
    };
    assert!(matches!(
        get(Family::Mailslurper, &wrong_subject),
        Outcome::WrongResult { .. }
    ));
    // Both parts, the HTML rewritten: the ruling covers the single body
    // only, so a rewrite beside a kept text part is a wrong HTML part.
    let both = clients::Fetched {
        html: Some(kept_html),
        ..fetched_from(&a)
    };
    assert_eq!(
        get(Family::Mailslurper, &both),
        Outcome::WrongResult {
            detail: "html".into()
        }
    );
    // The kept HTML as text: lines at block tags, tags dropped, entities read.
    assert_eq!(
        clients::html_text(
            "<p>a &amp; b</p><p>Confirm: <a href=\"x\">http://x</a></p>tail<br/>end"
        ),
        "a & b\nConfirm: http://x\ntail\nend"
    );
    assert_eq!(clients::html_text("x < y"), "x < y");
}

#[test]
fn get_is_exact_inexact_only_where_v3_documents_it_and_otherwise_wrong() {
    let a = api::author(163, api::Role::Read, 3, &api::read_scope());
    let f = fetched_from(&a);
    assert_eq!(
        api::check_get(Family::Mailpit, Ok(f.clone()), &a),
        Outcome::Exact
    );
    let mut no_text = f.clone();
    no_text.text = None;
    // MailSlurper's one decoded body is documented; nobody else's.
    assert!(matches!(
        api::check_get(Family::Mailslurper, Ok(no_text.clone()), &a),
        Outcome::InexactFidelity { .. }
    ));
    assert!(matches!(
        api::check_get(Family::Mailpit, Ok(no_text), &a),
        Outcome::WrongResult { .. }
    ));
    let mut subject = f.clone();
    subject.subject = Some("other".into());
    assert!(matches!(
        api::check_get(Family::Mailslurper, Ok(subject), &a),
        Outcome::WrongResult { .. }
    ));
    // R7: only the documented absent part. A part MailSlurper returns but
    // gets wrong, both parts gone, or an absent part beside any other
    // mismatch is a wrong result, never inexact.
    let ms = |g: clients::Fetched| api::check_get(Family::Mailslurper, Ok(g), &a);
    let mut empty_html = f.clone();
    empty_html.html = Some("  \r\n".into());
    assert_eq!(
        ms(empty_html),
        Outcome::InexactFidelity {
            detail: "html absent".into()
        }
    );
    let mut wrong_text = f.clone();
    wrong_text.text = Some(format!("{}\n..dot-stuffed", a.text));
    assert_eq!(
        ms(wrong_text),
        Outcome::WrongResult {
            detail: "text".into()
        }
    );
    let mut wrong_html = f.clone();
    wrong_html.html = Some("<p>other</p>".into());
    assert!(matches!(ms(wrong_html), Outcome::WrongResult { .. }));
    let mut both = f.clone();
    (both.text, both.html) = (None, None);
    assert_eq!(
        ms(both),
        Outcome::WrongResult {
            detail: "text absent,html absent".into()
        }
    );
    let mut mixed = f.clone();
    mixed.text = None;
    mixed.to = vec!["someone@else.invalid".into()];
    assert_eq!(
        ms(mixed),
        Outcome::WrongResult {
            detail: "to,text absent".into()
        }
    );
    let mut mixed = f.clone();
    mixed.text = None;
    mixed.html = Some("<p>other</p>".into());
    assert!(matches!(ms(mixed), Outcome::WrongResult { .. }));
    // Inexact latencies stay out of the exact quantiles.
    let ran = api::Ran {
        rows: {
            let mut r = rows(2, Phase::Measured, Outcome::Exact);
            r[1].outcome = Outcome::InexactFidelity {
                detail: "html absent".into(),
            };
            r
        },
        planned: 2,
        started: 2,
        ..Default::default()
    };
    let st = api::stats(&ran, None);
    assert_eq!((st.exact, st.inexact_fidelity), (1, 1));
    let mut absent_from = f.clone();
    absent_from.from = None;
    assert!(
        matches!(api::check_get(Family::Swarmail, Ok(absent_from), &a), Outcome::WrongResult { detail } if detail == "from")
    );
    // Extraction: Swarmail's ingest fields, everyone else the frozen
    // extractor over the text they read.
    assert_eq!(
        clients::answer_code(Family::Swarmail, &f).as_deref(),
        Some(a.code.as_str())
    );
    assert_eq!(
        clients::answer_link(Family::Swarmail, &f, api::LINK_LABEL).as_deref(),
        Some(a.link.as_str())
    );
    assert_eq!(
        clients::answer_code(Family::Inbucket, &f).as_deref(),
        Some(a.code.as_str())
    );
    assert_eq!(
        clients::answer_link(Family::Inbucket, &f, api::LINK_LABEL).as_deref(),
        Some(a.link.as_str())
    );
    let mut two = f.clone();
    two.links = Some(vec![a.link.clone(), "http://other.invalid/".into()]);
    assert_eq!(
        clients::answer_link(Family::Swarmail, &two, api::LINK_LABEL),
        None
    );
    assert!(matches!(
        api::check_value(Ok(None), &a.code),
        Outcome::WrongResult { .. }
    ));
    assert!(matches!(
        api::check_value(Ok(Some("000000".into())), &a.code),
        Outcome::WrongResult { .. }
    ));
    assert_eq!(
        api::check_value(Ok(Some(a.code.clone())), &a.code),
        Outcome::Exact
    );
}

#[test]
fn authored_fixtures_carry_one_code_and_one_labelled_link() {
    for role in [
        api::Role::Read,
        api::Role::Victim,
        api::Role::ClearScope,
        api::Role::Future,
    ] {
        for n in [0u32, 7, 99_999] {
            let a = api::author(163, role, n, &api::read_scope());
            assert_eq!(
                clients::extract::six_digit_code(&a.text).as_deref(),
                Some(a.code.as_str())
            );
            assert_eq!(
                clients::extract::labelled_url(&a.text, api::LINK_LABEL).as_deref(),
                Some(a.link.as_str())
            );
            let p = clients::mime::parse(&api::fixture_bytes(&a)).unwrap();
            assert_eq!(p.subject.as_deref(), Some(a.subject.as_str()));
            assert!(api::same_body(Some(&a.text), p.text.as_deref()));
            assert!(api::same_body(Some(&a.html), p.html.as_deref()));
            assert_eq!(p.to, std::slice::from_ref(&a.scope.address));
        }
    }
    // Deterministic, and distinct per role and index.
    let a = api::author(163, api::Role::Read, 1, &api::read_scope());
    assert_eq!(a, api::author(163, api::Role::Read, 1, &api::read_scope()));
    assert_ne!(
        a.token,
        api::author(163, api::Role::Victim, 1, &api::read_scope()).token
    );
}

#[test]
fn the_fill_is_the_v3_corpus_byte_for_byte() {
    let kinds = [
        protocol::MessageKind::Plain,
        protocol::MessageKind::Html,
        protocol::MessageKind::Multipart,
    ];
    for i in 0..9u32 {
        let v3 = wire::authored(163, "tag", i, &kinds);
        let to = api::background_scope(i % api::BACKGROUND).address;
        let want = String::from_utf8(v3.bytes)
            .unwrap()
            .replace(&format!("To: <{}>", wire::TO), &format!("To: <{to}>"));
        assert_eq!(
            String::from_utf8(api::fill_bytes(163, "tag", i, &to)).unwrap(),
            want,
            "{i}"
        );
    }
}

// ---- destructive postchecks (T6) --------------------------------------------

fn rows(n: u32, phase: Phase, outcome: Outcome) -> Vec<api::OpRow> {
    (0..n)
        .map(|i| api::OpRow {
            phase,
            index: i,
            worker: 0,
            start_ns: 0,
            end_ns: 1,
            outcome: outcome.clone(),
            requests: 1,
            bytes: 0,
            wait: None,
        })
        .collect()
}

#[test]
fn a_delete_postcheck_proves_victims_gone_and_everything_else_kept() {
    let victims: Vec<String> = (0..6).map(|i| format!("v{i}")).collect();
    let survivors: BTreeSet<String> = ["s0".into(), "s1".into()].into();
    // Warmup 2, measured 3 of 4: victims v0..v4 deleted, v5 unstarted.
    let mut r = rows(2, Phase::Warmup, Outcome::Exact);
    r.extend(rows(3, Phase::Measured, Outcome::Exact));
    let retained: BTreeSet<String> = ["s0", "s1", "v5"].iter().map(|s| (*s).to_owned()).collect();
    assert!(api::postcheck_delete(&retained, &survivors, &victims, &mut r, 2).is_empty());
    assert!(r.iter().all(|x| x.outcome == Outcome::Exact));
    // A victim still there: that row only is wrong.
    let mut r2 = r.clone();
    let mut kept = retained.clone();
    kept.insert("v3".into());
    let p = api::postcheck_delete(&kept, &survivors, &victims, &mut r2, 2);
    assert_eq!(p.len(), 1);
    assert_eq!(r2.iter().filter(|x| x.outcome != Outcome::Exact).count(), 1);
    // A survivor gone, an unstarted victim gone, or a foreign id: every
    // success is invalidated.
    for broken in [
        vec!["s0", "v5"],
        vec!["s0", "s1"],
        vec!["s0", "s1", "v5", "zz"],
    ] {
        let mut r3 = r.clone();
        let set: BTreeSet<String> = broken.iter().map(|s| (*s).to_owned()).collect();
        assert!(!api::postcheck_delete(&set, &survivors, &victims, &mut r3, 2).is_empty());
        assert!(
            r3.iter()
                .all(|x| matches!(x.outcome, Outcome::WrongResult { .. })),
            "{broken:?}"
        );
    }
    // A failed delete's victim may be either way.
    let mut r4 = r.clone();
    r4[2].outcome = Outcome::Failed {
        reason: "status_500".into(),
    };
    let mut with_failed = retained.clone();
    with_failed.insert("v2".into());
    assert!(api::postcheck_delete(&with_failed, &survivors, &victims, &mut r4, 2).is_empty());
}

#[test]
fn a_clear_postcheck_proves_scopes_emptied_and_nothing_outside_touched() {
    let mut r = rows(1, Phase::Warmup, Outcome::Exact);
    r.extend(rows(2, Phase::Measured, Outcome::Exact));
    let counts = [0, 0, 0, u64::from(api::SURVIVORS)];
    assert!(api::postcheck_clear(&counts, &mut r, 1, true).is_empty());
    let mut r2 = r.clone();
    assert!(!api::postcheck_clear(&[0, 3, 0, 10], &mut r2, 1, true).is_empty());
    assert_eq!(r2.iter().filter(|x| x.outcome != Outcome::Exact).count(), 1);
    let mut r3 = r.clone();
    assert!(!api::postcheck_clear(&[0, 0, 0, 9], &mut r3, 1, true).is_empty());
    assert!(r3.iter().all(|x| x.outcome != Outcome::Exact));
    let mut r4 = r.clone();
    assert!(!api::postcheck_clear(&counts, &mut r4, 1, false).is_empty());
    assert!(r4.iter().all(|x| x.outcome != Outcome::Exact));
}

// ---- the complete batch ledger (R5, R6) -------------------------------------

/// A batch of one delete cell and one recipient-clear cell, every fixture
/// resolved to a distinct id, with the measured rows the test chooses.
fn ledger_batch() -> (
    Vec<PlannedCell>,
    Vec<api::Authored>,
    Vec<api::CellRecord>,
    BTreeSet<String>,
) {
    let mut del = cell(Operation::Delete, 1);
    del.ordinal = 0;
    let mut clr = cell(Operation::Clear, 1);
    clr.scope = Some(ClearScope::Recipient);
    clr.ordinal = 0;
    let cells = vec![del, clr];
    let mut fixtures = api::plan_fixtures(5, &cells);
    for (n, f) in fixtures.iter_mut().enumerate() {
        f.id = Some(format!("id{n:05}"));
    }
    // Warmups all exact; delete measured 0..60 exact, the rest unstarted;
    // clear measured 0..50 exact, 50 refused, the rest unstarted.
    let mut recs = Vec::new();
    for (c, (exact, refused)) in cells.iter().zip([(60, 0), (50, 1)]) {
        let caps = clients::native_capability("swarmail", c.operation);
        let mut r = api::CellRecord::new(c, &caps);
        r.warmup = Some(api::Ran {
            rows: rows(c.warmup, Phase::Warmup, Outcome::Exact),
            planned: c.warmup,
            started: c.warmup,
            ..Default::default()
        });
        let mut m = rows(exact, Phase::Measured, Outcome::Exact);
        m.extend(
            rows(
                exact + refused,
                Phase::Measured,
                Outcome::Failed {
                    reason: "status_404".into(),
                },
            )
            .into_iter()
            .skip(exact as usize),
        );
        r.measured = api::Ran {
            started: m.len() as u32,
            unstarted: c.planned - m.len() as u32,
            planned: c.planned,
            rows: m,
            ..Default::default()
        };
        recs.push(r);
    }
    let background: BTreeSet<String> = (0..30).map(|n| format!("bg{n}")).collect();
    (cells, fixtures, recs, background)
}

/// What an intact store holds after `ledger_batch`.
fn intact(want: &api::Expected) -> BTreeSet<String> {
    want.kept.clone()
}

#[test]
fn the_expected_ledger_follows_every_role_and_terminal_row() {
    let (cells, fixtures, recs, background) = ledger_batch();
    let want = api::expected_ledger(5, &background, &fixtures, &cells, &recs);
    let id = |role: api::Role, nth: usize| {
        fixtures
            .iter()
            .filter(|f| f.role == role)
            .nth(nth)
            .unwrap()
            .id
            .clone()
            .unwrap()
    };
    // Exact deletes are gone, unstarted victims kept; survivors, the read
    // set, the sentinel and the background kept.
    assert!(want.gone.contains(&id(api::Role::Victim, 0)));
    assert!(want.gone.contains(&id(api::Role::Victim, 79)));
    assert!(want.kept.contains(&id(api::Role::Victim, 80)));
    for role in [api::Role::Survivor, api::Role::Read, api::Role::Sentinel] {
        assert!(want.kept.contains(&id(role, 0)), "{role:?}");
    }
    assert!(want.kept.is_superset(&background));
    // Clear slot 69 (measured 49) exact: emptied; slot 70 refused and 71
    // never started: intact.
    let scope_ids = |slot| -> Vec<String> {
        fixtures
            .iter()
            .filter(|f| f.scope == api::clear_scope(0, slot))
            .map(|f| f.id.clone().unwrap())
            .collect()
    };
    assert!(scope_ids(69).iter().all(|i| want.gone.contains(i)));
    assert!(scope_ids(70).iter().all(|i| want.kept.contains(i)));
    assert!(scope_ids(71).iter().all(|i| want.kept.contains(i)));
    assert!(want.unsure.is_empty() && want.waits.is_empty());
    // The intact store passes.
    assert!(api::ledger_problems(&want, &intact(&want), &[]).is_empty());
}

#[test]
fn the_ledger_catches_later_loss_partial_clears_and_substituted_ids() {
    let (cells, fixtures, recs, background) = ledger_batch();
    let want = api::expected_ledger(5, &background, &fixtures, &cells, &recs);
    let find = |role: api::Role, nth: usize| {
        fixtures
            .iter()
            .filter(|f| f.role == role)
            .nth(nth)
            .unwrap()
            .id
            .clone()
            .unwrap()
    };
    let check = |store: BTreeSet<String>| api::ledger_problems(&want, &store, &[]);
    // A survivor deleted by a later cell.
    let mut s = intact(&want);
    s.remove(&find(api::Role::Survivor, 3));
    assert!(check(s)[0].contains("missing"));
    // An unstarted victim deleted anyway.
    let mut s = intact(&want);
    s.remove(&find(api::Role::Victim, 100));
    assert!(check(s)[0].contains("missing"));
    // A scope the clear reported exact, only partly emptied.
    let mut s = intact(&want);
    let left = fixtures
        .iter()
        .find(|f| f.scope == api::clear_scope(0, 25))
        .unwrap()
        .id
        .clone()
        .unwrap();
    s.insert(left);
    assert!(check(s)[0].contains("still present"));
    // A background message lost.
    let mut s = intact(&want);
    s.remove("bg7");
    assert!(check(s)[0].contains("missing"));
    // The sentinel replaced by the same number of other ids: equal counts,
    // substituted ids — missing and unaccounted both named.
    let mut s = intact(&want);
    s.remove(&find(api::Role::Sentinel, 0));
    s.insert("impostor".into());
    let p = check(s);
    assert_eq!(p.len(), 2, "{p:?}");
    assert!(
        p[0].contains("missing") && p[1].contains("unaccounted"),
        "{p:?}"
    );
    // Any mismatch invalidates every destructive (and wait) success.
    let mut recs2 = recs.clone();
    api::invalidate_batch(&mut recs2, &p);
    for r in &recs2 {
        assert!(r.measured.rows.iter().all(|x| x.outcome != Outcome::Exact));
        assert!(
            r.warmup
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .all(|x| x.outcome != Outcome::Exact)
        );
        assert_eq!(r.postcheck, p);
    }
    let mut recs3 = recs.clone();
    api::invalidate_batch(&mut recs3, &[]);
    assert_eq!(recs3, recs);
}

#[test]
fn the_ledger_holds_each_wait_scope_to_its_acknowledged_delivery() {
    let mut w = cell(Operation::Wait, 1);
    w.warmup = 1;
    w.planned = 2;
    let caps = clients::mcp_capability("swarmail", Operation::Wait);
    let mut r = api::CellRecord::new(&w, &caps);
    let times = |ack: Option<u64>| api::WaitTimes {
        call_ns: 1,
        send_ns: 2,
        ack_ns: ack,
        return_ns: 3,
        ack_to_return_ns: ack.map(|a| 3 - a as i64),
    };
    let mut warm = rows(1, Phase::Warmup, Outcome::Exact);
    warm[0].wait = Some(times(Some(2)));
    r.warmup = Some(api::Ran {
        started: 1,
        planned: 1,
        rows: warm,
        ..Default::default()
    });
    // Measured 0 acknowledged; measured 1 refused before its terminator.
    let mut m = rows(1, Phase::Measured, Outcome::Exact);
    m[0].wait = Some(times(Some(2)));
    let mut refused = rows(
        2,
        Phase::Measured,
        Outcome::Failed {
            reason: "fixture_refused".into(),
        },
    );
    refused[1].wait = Some(times(None));
    m.push(refused.remove(1));
    r.measured = api::Ran {
        started: 2,
        planned: 2,
        rows: m,
        ..Default::default()
    };
    let want = api::expected_ledger(9, &BTreeSet::new(), &[], &[w.clone()], &[r]);
    assert_eq!(
        want.waits.iter().map(|x| x.2).collect::<Vec<_>>(),
        [true, true, false]
    );
    let msg = |id: &str, slot: usize| {
        let (scope, token, _) = &want.waits[slot];
        summary(id, &scope.address, &format!("x {token} y"))
    };
    let ok = [vec![msg("a", 0)], vec![msg("b", 1)], vec![]];
    let views: Vec<_> = ok.iter().cloned().zip(want.waits.iter()).collect();
    let store: BTreeSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
    assert!(api::ledger_problems(&want, &store, &views).is_empty());
    // A missing acknowledged delivery, and an extra one where none was sent.
    let bad = [vec![], vec![msg("b", 1)], vec![msg("c", 2)]];
    let views: Vec<_> = bad.iter().cloned().zip(want.waits.iter()).collect();
    let p = api::ledger_problems(&want, &store, &views);
    assert_eq!(p.len(), 3, "{p:?}");
}

// ---- the fill's failure branches (R16) ---------------------------------------

/// A loopback SMTP server that accepts every transaction (or, with `hold`,
/// never acknowledges a terminator), recording each session and recipient.
async fn fill_smtp(
    hold: bool,
) -> (
    std::net::SocketAddr,
    Arc<std::sync::Mutex<Vec<(u32, String)>>>,
) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen: Arc<std::sync::Mutex<Vec<(u32, String)>>> = Arc::default();
    let s2 = seen.clone();
    tokio::spawn(async move {
        let mut session = 0u32;
        while let Ok((s, _)) = listener.accept().await {
            session += 1;
            let (seen, me) = (s2.clone(), session);
            tokio::spawn(async move {
                let (r, mut w) = s.into_split();
                let mut r = tokio::io::BufReader::new(r);
                let _ = w.write_all(b"220 x\r\n").await;
                let mut data = false;
                loop {
                    let mut l = String::new();
                    if r.read_line(&mut l).await.unwrap_or(0) == 0 {
                        return;
                    }
                    if data {
                        if l == ".\r\n" {
                            data = false;
                            if hold {
                                std::future::pending::<()>().await;
                            }
                            let _ = w.write_all(b"250 ok\r\n").await;
                        }
                        continue;
                    }
                    let reply: &[u8] = if l.starts_with("EHLO") {
                        b"250-x\r\n250 OK\r\n"
                    } else if let Some(to) = l.strip_prefix("RCPT TO:<") {
                        seen.lock()
                            .unwrap()
                            .push((me, to.trim_end().trim_end_matches('>').to_owned()));
                        b"250 ok\r\n"
                    } else if l.starts_with("DATA") {
                        data = true;
                        b"354 go\r\n"
                    } else if l.starts_with("QUIT") {
                        let _ = w.write_all(b"221 bye\r\n").await;
                        return;
                    } else {
                        b"250 ok\r\n"
                    };
                    let _ = w.write_all(reply).await;
                }
            });
        }
    });
    (addr, seen)
}

/// A Mailpit-shaped listing of `n` distinct ids.
async fn listing_of(n: u64) -> std::net::SocketAddr {
    responder(move |p| {
        let s = start_of(p);
        let ids: Vec<String> = (s..n.min(s + 250)).map(|i| format!("m{i}")).collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        (200, page(s, n, &refs))
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fill_goes_over_eight_sessions_round_robin_and_reaches_its_level() {
    let (smtp, seen) = fill_smtp(false).await;
    let client = clients::Client::new("mailpit", listing_of(400).await).unwrap();
    let (rec, ids) = api::fill_ledger(smtp, &client, (163, "t"), 400, clock()).await;
    assert!(rec.reached, "{rec:?}");
    assert_eq!((rec.accepted, rec.retained), (400, Some(400)));
    assert_eq!(ids.unwrap().len(), 400);
    let seen = seen.lock().unwrap().clone();
    let sessions: BTreeSet<u32> = seen.iter().map(|x| x.0).collect();
    assert_eq!(sessions.len(), api::FILL_SESSIONS);
    let mut per = std::collections::BTreeMap::<&str, u32>::new();
    for (_, to) in &seen {
        *per.entry(to.as_str()).or_default() += 1;
    }
    assert_eq!(per.len(), api::BACKGROUND as usize);
    assert!(per.values().all(|&n| n == 4), "{per:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fill_whose_store_keeps_less_than_it_accepted_is_not_reached() {
    // Every send acknowledged, one message short in the store: never a
    // shorter success, never retried.
    let (smtp, seen) = fill_smtp(false).await;
    let client = clients::Client::new("mailpit", listing_of(99).await).unwrap();
    // The store is read again until the bound (a store still storing
    // would catch up), so a bound shorter than FILL_DEADLINE keeps the test
    // brief; it also holds the 100 sends and the first read, so it leaves
    // room for a loaded parallel suite.
    let bound = Duration::from_secs(10);
    let (rec, ids) = api::fill_within(smtp, &client, (163, "t"), 100, clock(), bound).await;
    assert!(!rec.reached);
    assert_eq!((rec.accepted, rec.retained), (100, Some(99)));
    assert_eq!(ids.unwrap().len(), 99);
    assert!(
        rec.detail
            .unwrap()
            .contains("accepted 100 retained Some(99) of 100")
    );
    assert_eq!(seen.lock().unwrap().len(), 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fill_that_runs_out_of_time_is_not_reached_and_says_how_far_it_got() {
    // No terminator is ever acknowledged: each session's first send is in
    // flight at the bound, counted ambiguous, never retried, and nothing
    // after it is sent. The same path FILL_DEADLINE bounds.
    let (smtp, seen) = fill_smtp(true).await;
    let client = clients::Client::new("mailpit", listing_of(0).await).unwrap();
    let (rec, _) = api::fill_within(
        smtp,
        &client,
        (163, "t"),
        1_000,
        clock(),
        Duration::from_millis(500),
    )
    .await;
    assert!(!rec.reached);
    assert_eq!(rec.accepted, 0);
    assert_eq!(rec.ambiguous, api::FILL_SESSIONS as u64);
    assert_eq!(seen.lock().unwrap().len(), api::FILL_SESSIONS);
    assert!(rec.elapsed_ns >= 500_000_000);
}

/// An Inbucket-shaped store: mailbox `<local>` lists `held(local)` messages,
/// numbered `1`, `2`, … in each mailbox, as the pinned Inbucket numbers
/// them.
async fn per_mailbox_store(held: impl Fn(&str) -> u32 + Send + Sync + 'static) -> clients::Client {
    let addr = responder(move |p| {
        let local = p.strip_prefix("/api/v1/mailbox/").unwrap_or_default();
        let items: Vec<Value> = (1..=held(local))
            .map(|n| {
                json!({
                    "id": n.to_string(),
                    "to": [format!("<{local}@{}>", clients::DOMAIN)],
                    "subject": format!("{local} {n}"),
                })
            })
            .collect();
        (200, Value::Array(items).to_string())
    })
    .await;
    clients::Client::new("inbucket", addr).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_per_mailbox_store_names_each_message_by_its_mailbox() {
    // Inbucket's ids (and GreenMail's UIDs) restart at 1 in every mailbox:
    // on the host, the fill's 1,000 messages in 100 mailboxes read back as
    // 10 ids. The ledger names each by its mailbox, so the fill counts
    // every one and a fixture's loss is not hidden by another mailbox's
    // message of the same number.
    assert!(api::ids_per_mailbox(Family::Inbucket) && api::ids_per_mailbox(Family::Greenmail));
    for f in [
        Family::Swarmail,
        Family::Mailpit,
        Family::Mailhog,
        Family::Mailslurper,
        Family::Smtp4dev,
    ] {
        assert!(!api::ids_per_mailbox(f), "{f:?}");
        assert_eq!(api::ledger_key(f, &api::read_scope(), "7"), "7");
    }
    let (smtp, _) = fill_smtp(false).await;
    let client = per_mailbox_store(|l| u32::from(l.starts_with("apibg")) * 2).await;
    let (rec, ids) = api::fill_ledger(smtp, &client, (163, "t"), 200, clock()).await;
    assert!(rec.reached, "{rec:?}");
    assert_eq!((rec.accepted, rec.retained), (200, Some(200)));
    let background = ids.unwrap();
    let first = format!("{}/1", api::background_scope(0).address);
    assert!(background.contains(&first), "{background:?}");
    // The final ledger: a read fixture and a sentinel, each its mailbox's
    // `1`, beside the background's own `1`s.
    let mut fixtures = vec![
        api::author(5, api::Role::Read, 0, &api::read_scope()),
        api::author(5, api::Role::Sentinel, 0, &api::sentinel_scope()),
    ];
    for f in &mut fixtures {
        f.id = Some("1".into());
    }
    let read = api::read_scope().inbox;
    let intact = per_mailbox_store(move |l| match l {
        _ if l.starts_with("apibg") => 2,
        _ => 1,
    })
    .await;
    let problems =
        api::final_ledger(&intact, (5, &background), &fixtures, &[], &mut [], None).await;
    assert!(problems.is_empty(), "{problems:?}");
    // The sentinel's mailbox lost its message: `1` is still in every other
    // mailbox, and the ledger still names the loss.
    let lost = per_mailbox_store(move |l| match l {
        _ if l.starts_with("apibg") => 2,
        _ if l == read => 1,
        _ => 0,
    })
    .await;
    let problems = api::final_ledger(&lost, (5, &background), &fixtures, &[], &mut [], None).await;
    let sentinel = format!("{}/1", api::sentinel_scope().address);
    assert_eq!(
        problems,
        [format!("1 expected id(s) missing, e.g. {sentinel}")]
    );
}

/// A Mailpit-shaped store that acknowledges before it stores: the `k`th
/// listing request sees `shown(k)` of the `n` messages, and a request in
/// the middle of a page sequence sees the count move.
async fn catching_up(
    n: u64,
    shown: impl Fn(u64) -> u64 + Send + Sync + 'static,
) -> clients::Client {
    let reads = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let addr = responder(move |p| {
        let k = reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let total = shown(k).min(n);
        let s = start_of(p);
        let ids: Vec<String> = (s..total.min(s + 250)).map(|i| format!("m{i}")).collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        (200, page(s, total, &refs))
    })
    .await;
    clients::Client::new("mailpit", addr).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fill_a_store_is_still_storing_is_confirmed_once_it_shows_every_message() {
    // MailSlurper answers 250 and stores later: on the host its listing
    // moved between pages right after the fill ("retained count
    // unreadable: incomplete"), then held every message seconds later. The
    // fill reads it again within its bound and never sends again.
    let (smtp, seen) = fill_smtp(false).await;
    // 400 messages in two 250-item pages; each request sees 60 more, so
    // the first scans move under the pager and the later ones are short.
    let client = catching_up(400, |k| 60 * (k + 1)).await;
    let (rec, ids) = api::fill_ledger(smtp, &client, (163, "t"), 400, clock()).await;
    assert!(rec.reached, "{rec:?}");
    assert_eq!((rec.accepted, rec.retained), (400, Some(400)));
    assert_eq!(ids.unwrap().len(), 400);
    assert_eq!(seen.lock().unwrap().len(), 400, "nothing was sent again");
    // A moving listing is read again; an answer that is enough, too many or
    // a refusal is final.
    assert!(api::unsettled(&Ok(3), 4));
    assert!(api::unsettled(&Err(clients::OpError::Incomplete), 4));
    for done in [
        Ok(4),
        Ok(5),
        Err(clients::OpError::Malformed),
        Err(clients::OpError::Status(500)),
        Err(clients::OpError::Timeout),
    ] {
        assert!(!api::unsettled(&done, 4), "{done:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixtures_a_store_is_still_storing_resolve_once_it_shows_them() {
    // The same store behaviour at preparation: a fixture scope that shows
    // its messages only on a later read still resolves every token, and
    // one that never shows them all is refused at the bound.
    let (smtp, seen) = fill_smtp(false).await;
    let mut fixtures = vec![
        api::author(5, api::Role::Read, 0, &api::read_scope()),
        api::author(5, api::Role::Read, 1, &api::read_scope()),
    ];
    let subjects: Vec<String> = fixtures.iter().map(|f| f.subject.clone()).collect();
    let reads = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let addr = responder(move |_| {
        // The first two reads show one message, then both.
        let k = reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let shown = if k < 2 { 1 } else { 2 };
        let items: Vec<Value> = subjects[..shown]
            .iter()
            .enumerate()
            .map(|(n, s)| json!({"id": (n + 1).to_string(), "to": [format!("<apiread00x@{}>", clients::DOMAIN)], "subject": s}))
            .collect();
        (200, Value::Array(items).to_string())
    })
    .await;
    let client = clients::Client::new("inbucket", addr).unwrap();
    api::prepare(smtp, &client, &mut fixtures, None)
        .await
        .unwrap();
    assert_eq!(
        fixtures.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        [Some("1".into()), Some("2".into())]
    );
    assert_eq!(seen.lock().unwrap().len(), 2, "each fixture sent once");
}

#[tokio::test(start_paused = true)]
async fn a_store_that_never_shows_everything_is_answered_at_the_bound() {
    // The settling read on the paused clock: a listing that stays short is
    // read every SETTLE_POLL until the next read could not begin before
    // the bound, and its last answer stands; a final answer stops at once.
    let start = tokio::time::Instant::now();
    let deadline = start + Duration::from_millis(1_000);
    let reads = std::cell::Cell::new(0u32);
    let got = api::settled(deadline, 2, Vec::len, || {
        reads.set(reads.get() + 1);
        async { Ok(vec![1u8]) }
    })
    .await;
    assert_eq!(got, Ok(vec![1]));
    assert_eq!(reads.get(), 10);
    assert_eq!(
        tokio::time::Instant::now() - start,
        Duration::from_millis(900)
    );
    for (answer, want) in [(Ok(vec![1u8, 2]), 1), (Err(clients::OpError::Malformed), 1)] {
        let reads = std::cell::Cell::new(0u32);
        let got = api::settled(deadline, 2, Vec::len, || {
            reads.set(reads.get() + 1);
            let a = answer.clone();
            async move { a }
        })
        .await;
        assert_eq!((got, reads.get()), (answer, want));
    }
    // A read again that the bound cuts is no answer: the one before stands.
    // The first read's own timeout is the answer.
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1_000);
    let reads = std::cell::Cell::new(0u32);
    let got = api::settled(deadline, 2, Vec::len, || {
        reads.set(reads.get() + 1);
        let n = reads.get();
        async move {
            match n {
                1 => Ok(vec![1u8]),
                _ => Err(clients::OpError::Timeout),
            }
        }
    })
    .await;
    assert_eq!((got, reads.get()), (Ok(vec![1]), 2));
    let got = api::settled(deadline, 2, Vec::<u8>::len, || async {
        Err(clients::OpError::Timeout)
    })
    .await;
    assert_eq!(got, Err(clients::OpError::Timeout));
}

#[test]
fn a_fill_not_reached_blocks_every_cell_and_admits() {
    // What both producers publish for it: every planned cell blocked by
    // fill_not_reached, nothing started, and the record admits as such.
    for surface in [Surface::Native, Surface::Mcp] {
        let chunk = runner::Chunk::Api {
            surface,
            target: TargetKind::Swarmail,
            repetition: 0,
            fill: 10_000,
        };
        let rec = record_for(&chunk);
        assert!(
            rec.cells
                .iter()
                .all(|c| c.status == api::CellStatus::Blocked
                    && c.blocked.as_deref() == Some("fill_not_reached")
                    && c.measured.unstarted == c.measured.planned)
        );
        api_chunk::check_record(&rec, &chunk).unwrap();
        assert!(api_chunk::row_failures(&rec).is_empty());
    }
}

// ---- plan, growth and admission (T4, T10) -----------------------------------

#[test]
fn the_plan_orders_reads_then_waits_then_deletes_then_clears() {
    let (cells, rows) = api::plan_cells(Surface::Native, |op| {
        clients::native_capability("swarmail", op)
    });
    let names: Vec<String> = cells.iter().map(PlannedCell::name).collect();
    assert_eq!(names.len(), 21);
    assert_eq!(&names[..3], ["list.c1", "list.c8", "list.c32"]);
    assert_eq!(
        &names[15..],
        [
            "delete.c1",
            "delete.c8",
            "delete.c32",
            "clear.c1",
            "clear.c8",
            "clear.c32"
        ]
    );
    assert!(rows.is_empty());
    let growth = api::fixture_growth(163, &cells);
    assert_eq!(growth, 10 + 3 * (120 + 10) + 3 * 120 * 10 + 10);
    assert!(api::within_cap(100_000, growth).is_ok());
    assert!(api::within_cap(146_000, growth).is_err());
    // Mailpit: one global clear, last, no warmup, one invocation.
    let (cells, _) = api::plan_cells(Surface::Native, |op| {
        clients::native_capability("mailpit", op)
    });
    let last = cells.last().unwrap();
    assert_eq!(
        (last.name().as_str(), last.warmup, last.planned),
        ("clear.all", 0, 1)
    );
    // MailHog: delete and clear are published rows, not cells.
    let (cells, rows) = api::plan_cells(Surface::Native, |op| {
        clients::native_capability("mailhog", op)
    });
    assert_eq!(cells.len(), 15);
    assert_eq!(
        rows.iter().map(|r| r.status.as_str()).collect::<Vec<_>>(),
        ["not_inspected", "not_inspected"]
    );
    // MCP adds the future wait between reads and destructive cells.
    let (cells, _) = api::plan_cells(Surface::Mcp, |op| clients::mcp_capability("swarmail", op));
    assert_eq!(cells[15].name(), "wait.c1");
    assert_eq!(api::fixture_growth(163, &cells), growth + 3 * 120);
}

#[test]
fn the_v4_draft_carries_0132s_api_section_and_plans_its_chunks() {
    let p = Protocol::parse(DRAFT_V4).unwrap();
    p.validate().unwrap();
    assert_eq!(p.api, Some(protocol::ApiSection::specified()));
    let plan = runner::plan_v4(&p);
    let native = plan
        .iter()
        .filter(|c| {
            matches!(
                c,
                runner::Chunk::Api {
                    surface: Surface::Native,
                    ..
                }
            )
        })
        .count();
    let mcp = plan
        .iter()
        .filter(|c| {
            matches!(
                c,
                runner::Chunk::Api {
                    surface: Surface::Mcp,
                    ..
                }
            )
        })
        .count();
    assert_eq!((native, mcp), (96, 48));
    // An api section that is not 0132's refuses; a frozen v4 without one too.
    let mut v: Value = serde_json::from_slice(DRAFT_V4).unwrap();
    v["api"]["concurrency"] = json!([1, 10, 100]);
    let w = Protocol::parse(&serde_json::to_vec(&v).unwrap())
        .unwrap()
        .validate()
        .unwrap_err()
        .to_string();
    assert!(w.contains("api: protocol 4's api section is 0132's"), "{w}");
    let mut v: Value = serde_json::from_slice(DRAFT_V4).unwrap();
    v.as_object_mut().unwrap().remove("api");
    v["status"] = json!("frozen");
    let w = Protocol::parse(&serde_json::to_vec(&v).unwrap())
        .unwrap()
        .validate()
        .unwrap_err()
        .to_string();
    assert!(
        w.contains("a frozen protocol 4 needs its api section"),
        "{w}"
    );
    // Protocol 3 refuses the field by name.
    let mut v3: Value = serde_json::from_slice(PROTOCOL_V3).unwrap();
    v3["api"] = serde_json::to_value(protocol::ApiSection::specified()).unwrap();
    let w = Protocol::parse(&serde_json::to_vec(&v3).unwrap())
        .unwrap()
        .validate()
        .unwrap_err()
        .to_string();
    assert!(w.contains("refuses api"), "{w}");
    // The driver refuses to run an mcp chunk: that is api-mcp's.
    let w =
        runner::v4_preflight(&p, "smoke", Some("api.mcp.mailpit.r0.f0"), None, None).unwrap_err();
    assert!(w.contains("api-mcp"), "{w}");
}

fn record_for(chunk: &runner::Chunk) -> api::ApiRecord {
    let runner::Chunk::Api {
        surface,
        target,
        repetition,
        fill,
    } = chunk
    else {
        panic!()
    };
    let cells = api_chunk::cells(*target, *surface);
    api::ApiRecord {
        surface: *surface,
        target: target.name().into(),
        repetition: *repetition,
        fill: api::FillRecord {
            fill: *fill,
            ..Default::default()
        },
        fixtures: 0,
        growth: 0,
        capabilities: Vec::new(),
        cells: cells
            .iter()
            .map(|c| {
                api::CellRecord::blocked(
                    c,
                    &clients::capability(target.name(), *surface, c.operation),
                    "fill_not_reached",
                    0,
                )
            })
            .collect(),
        abandoned: None,
        store: "test".into(),
        ledger: Vec::new(),
        control: Vec::new(),
    }
}

fn warm(r: &mut api::ApiRecord) -> &mut api::Ran {
    r.cells[0].warmup.as_mut().unwrap()
}

#[test]
fn admission_holds_warmups_and_cell_identities_to_the_plan() {
    let chunk = runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 1000,
    };
    // A measured record: every cell's warmup and measured rows complete.
    let mut rec = record_for(&chunk);
    for (c, p) in rec
        .cells
        .iter_mut()
        .zip(api_chunk::cells(TargetKind::Mailpit, Surface::Native))
    {
        c.status = api::CellStatus::Measured;
        c.blocked = None;
        c.warmup = (p.warmup > 0).then(|| api::Ran {
            rows: rows(p.warmup, Phase::Warmup, Outcome::Exact),
            planned: p.warmup,
            started: p.warmup,
            ..Default::default()
        });
        c.measured = api::Ran {
            rows: rows(p.planned, Phase::Measured, Outcome::Exact),
            planned: p.planned,
            started: p.planned,
            ..Default::default()
        };
    }
    api_chunk::check_record(&rec, &chunk).unwrap();
    // It survives its own bytes, and its report recomputes from them.
    let back: api::ApiRecord = serde_json::from_slice(&serde_json::to_vec(&rec).unwrap()).unwrap();
    assert_eq!(back, rec);
    api_chunk::check_record(&back, &chunk).unwrap();
    assert_eq!(
        api_chunk::api_report(&back, &[]),
        api_chunk::api_report(&rec, &[])
    );
    let refuse = |f: &dyn Fn(&mut api::ApiRecord), want: &str| {
        let mut r = rec.clone();
        f(&mut r);
        let e = api_chunk::check_record(&r, &chunk).unwrap_err();
        assert!(e.contains(want), "{e}");
    };
    // Warmup: a duplicate index, a count, an outcome, a phase, a missing or
    // extra warmup.
    refuse(&|r| warm(r).rows[1].index = 0, "warmup");
    refuse(&|r| warm(r).started = 19, "warmup");
    refuse(&|r| warm(r).planned = 21, "warmup");
    refuse(&|r| warm(r).rows[4].outcome = Outcome::Timeout, "warmup");
    refuse(
        &|r| {
            warm(r).rows[4].outcome = Outcome::Failed {
                reason: "status_500".into(),
            }
        },
        "warmup",
    );
    refuse(&|r| warm(r).rows[2].phase = Phase::Measured, "warmup");
    refuse(&|r| r.cells[0].warmup = None, "warmup");
    let last = rec.cells.len() - 1;
    refuse(
        &move |r| {
            r.cells[last].warmup = Some(api::Ran::default());
        },
        "warmup",
    );
    // Identity: operation, concurrency, method, scope, routes, citations.
    refuse(
        &|r| r.cells[0].operation = Operation::Search,
        "planned cell",
    );
    refuse(&|r| r.cells[0].concurrency = Some(8), "planned cell");
    refuse(
        &|r| r.cells[0].method = Some(clients::Method::Inbox),
        "planned cell",
    );
    refuse(
        &move |r| r.cells[last].scope = Some(ClearScope::Recipient),
        "planned cell",
    );
    refuse(&|r| r.cells[0].routes.push("GET /x".into()), "planned cell");
    refuse(&|r| r.cells[0].cites[0].push('x'), "planned cell");
    // Status and cause agree.
    refuse(
        &|r| r.cells[0].blocked = Some("x".into()),
        "does not add up",
    );
}

/// A record of `chunk` whose every planned cell measured fully and exactly.
fn measured_record(chunk: &runner::Chunk) -> api::ApiRecord {
    let runner::Chunk::Api {
        surface, target, ..
    } = chunk
    else {
        panic!()
    };
    let mut rec = record_for(chunk);
    for (c, p) in rec
        .cells
        .iter_mut()
        .zip(api_chunk::cells(*target, *surface))
    {
        c.status = api::CellStatus::Measured;
        c.blocked = None;
        c.warmup = (p.warmup > 0).then(|| api::Ran {
            rows: rows(p.warmup, Phase::Warmup, Outcome::Exact),
            planned: p.warmup,
            started: p.warmup,
            ..Default::default()
        });
        c.measured = api::Ran {
            rows: rows(p.planned, Phase::Measured, Outcome::Exact),
            planned: p.planned,
            started: p.planned,
            ..Default::default()
        };
    }
    if *surface == Surface::Mcp {
        // The holder's brackets of every measured cell, in order.
        let b = |cmd: &str, id: &str, at: u64| api::Bracket {
            cmd: cmd.into(),
            id: Some(id.to_owned()),
            sent_ns: at,
            received_ns: at + 5,
            owner_t_ns: 1_000 * at,
        };
        let names: Vec<String> = rec.cells.iter().map(|c| c.cell.clone()).collect();
        for (n, name) in names.iter().enumerate() {
            let at = 100 * (n as u64 + 1);
            rec.control.push(b("window_start", name, at));
            rec.control.push(b("window_end", name, at + 50));
        }
    }
    rec
}

/// The cell's warmup and measured rows in one slot space, then back.
fn with_rows(c: &mut api::CellRecord, f: impl FnOnce(&mut Vec<api::OpRow>) -> Vec<String>) {
    let mut all: Vec<api::OpRow> = c
        .warmup
        .iter()
        .flat_map(|w| w.rows.clone())
        .chain(c.measured.rows.clone())
        .collect();
    c.postcheck = f(&mut all);
    let (w, m): (Vec<_>, Vec<_>) = all.into_iter().partition(|r| r.phase == Phase::Warmup);
    if let Some(warm) = c.warmup.as_mut() {
        warm.rows = w;
    }
    c.measured.rows = m;
}

#[test]
fn a_record_its_verification_invalidated_still_admits_and_fails() {
    // Review finding 1: every verification that rewrites a measured cell's
    // exact warmup rows (a delete or clear postcheck, a cut postcheck, the
    // batch ledger) leaves a record the seal, the holder's record arm and
    // report admission — all three call check_record — admit, and whose
    // rows the one verdict counts as the target's failure (exit 1), on both
    // surfaces. The same rewritten rows with no named problem refuse.
    for surface in [Surface::Native, Surface::Mcp] {
        let chunk = runner::Chunk::Api {
            surface,
            target: TargetKind::Swarmail,
            repetition: 0,
            fill: 0,
        };
        let base = measured_record(&chunk);
        api_chunk::check_record(&base, &chunk).unwrap();
        assert!(api_chunk::row_failures(&base).is_empty());
        let at = |r: &api::ApiRecord, op: Operation| {
            r.cells
                .iter()
                .position(|c| c.operation == op && c.warmup.is_some())
                .unwrap_or_else(|| panic!("{surface:?} plans a warmed {op:?} cell"))
        };
        let admits_and_fails = |r: &api::ApiRecord, cell: usize, what: &str| {
            api_chunk::check_record(r, &chunk).unwrap_or_else(|e| panic!("{what}: {e}"));
            let f = api_chunk::row_failures(r);
            assert!(
                f.iter().any(|f| f.contains(&r.cells[cell].cell)),
                "{what}: {f:?}"
            );
            // Its bytes admit again: what report admission reads.
            let back: api::ApiRecord =
                serde_json::from_slice(&serde_json::to_vec(r).unwrap()).unwrap();
            api_chunk::check_record(&back, &chunk).unwrap();
        };
        // A delete whose warmup victim is still retained: only warmup slot
        // 0 is wrong, every measured row stays exact, and it still fails.
        let mut r = base.clone();
        let d = at(&r, Operation::Delete);
        let warmup = r.cells[d].warmup.as_ref().unwrap().planned;
        let total = warmup + r.cells[d].measured.planned;
        let victims: Vec<String> = (0..total).map(|i| format!("v{i}")).collect();
        let survivors: BTreeSet<String> = (0..10).map(|i| format!("s{i}")).collect();
        let retained: BTreeSet<String> = survivors
            .iter()
            .cloned()
            .chain(std::iter::once("v0".to_owned()))
            .collect();
        with_rows(&mut r.cells[d], |rows| {
            api::postcheck_delete(&retained, &survivors, &victims, rows, warmup)
        });
        let w = &r.cells[d].warmup.as_ref().unwrap().rows;
        assert!(matches!(w[0].outcome, Outcome::WrongResult { .. }));
        assert!(
            r.cells[d]
                .measured
                .rows
                .iter()
                .all(|x| x.outcome == Outcome::Exact)
        );
        admits_and_fails(&r, d, "delete survivor kept");
        // A survivor lost: every success of the cell, warmup included.
        let mut r = base.clone();
        let mut lost = survivors.clone();
        lost.remove("s0");
        let gone: BTreeSet<String> = lost.into_iter().collect();
        with_rows(&mut r.cells[d], |rows| {
            api::postcheck_delete(&gone, &survivors, &victims, rows, warmup)
        });
        admits_and_fails(&r, d, "delete scope changed");
        // A recipient clear whose cleared scope still holds mail.
        let mut r = base.clone();
        let c = at(&r, Operation::Clear);
        let warmup = r.cells[c].warmup.as_ref().unwrap().planned;
        let total = warmup + r.cells[c].measured.planned;
        let mut counts = vec![0u64; total as usize];
        counts[1] = 3;
        with_rows(&mut r.cells[c], |rows| {
            api::postcheck_clear(&counts, rows, warmup, true)
        });
        admits_and_fails(&r, c, "partial clear");
        // A postcheck the chunk deadline cut.
        let mut r = base.clone();
        with_rows(&mut r.cells[d], |rows| {
            api::unverified(rows);
            vec![format!("delete: {}: timeout", api::POSTCHECK_CUT)]
        });
        admits_and_fails(&r, d, "cut postcheck");
        // The batch ledger: a missing or extra wait delivery rewrites every
        // destructive and wait cell's successes, warmup included.
        let mut r = base.clone();
        r.ledger = vec!["1 unaccounted id(s), e.g. x".into()];
        api::invalidate_batch(&mut r.cells, &r.ledger.clone());
        // Wait is planned on MCP only; natively the delete cell carries it.
        let wt = at(
            &r,
            if surface == Surface::Mcp {
                Operation::Wait
            } else {
                Operation::Delete
            },
        );
        assert!(
            r.cells[wt]
                .warmup
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .all(|x| matches!(x.outcome, Outcome::WrongResult { .. }))
        );
        admits_and_fails(&r, wt, "ledger");
        // The same rewrites with no problem named are not what any
        // producer writes: refused.
        let mut forged = r.clone();
        forged.cells[wt].postcheck.clear();
        let e = api_chunk::check_record(&forged, &chunk).unwrap_err();
        assert!(e.contains("warmup"), "{e}");
        let mut forged = base.clone();
        forged.cells[d].warmup.as_mut().unwrap().rows[0].outcome = Outcome::Failed {
            reason: api::POSTCHECK_CUT.into(),
        };
        assert!(api_chunk::check_record(&forged, &chunk).is_err());
        // A named problem never admits a warmup row that failed as it ran.
        let mut forged = base.clone();
        forged.cells[d].postcheck = vec!["x".into()];
        forged.cells[d].warmup.as_mut().unwrap().rows[0].outcome = Outcome::Timeout;
        assert!(api_chunk::check_record(&forged, &chunk).is_err());
        forged.cells[d].warmup.as_mut().unwrap().rows[0].outcome = Outcome::Failed {
            reason: "status_500".into(),
        };
        assert!(api_chunk::check_record(&forged, &chunk).is_err());
    }
}

#[test]
fn the_global_clear_is_one_raw_sample_with_no_quantiles() {
    let chunk = runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 0,
    };
    let mut rec = record_for(&chunk);
    let last = rec.cells.len() - 1;
    let clear = &mut rec.cells[last];
    assert_eq!(
        (
            clear.cell.as_str(),
            clear.concurrency,
            clear.measured.planned
        ),
        ("clear.all", None, 1)
    );
    clear.status = api::CellStatus::Measured;
    clear.blocked = None;
    let mut one = rows(1, Phase::Measured, Outcome::Exact);
    (one[0].start_ns, one[0].end_ns) = (1_000, 4_000);
    clear.measured = api::Ran {
        rows: one,
        planned: 1,
        started: 1,
        start_ns: 900,
        end_ns: 4_100,
        ..Default::default()
    };
    api_chunk::check_record(&rec, &chunk).unwrap();
    let report = api_chunk::api_report(&rec, &[]);
    let c = &report["cells"][last];
    assert_eq!(c["cell"], "clear.all");
    assert_eq!(c["sample_ns"], json!(3_000));
    for q in ["p50_ns", "p95_ns", "p99_ns"] {
        assert_eq!(c["stats"][q], Value::Null, "{q}");
    }
    assert_eq!(
        (c["stats"]["started"].clone(), c["stats"]["exact"].clone()),
        (json!(1), json!(1))
    );
    // Every other cell keeps its quantiles and has no sample.
    assert_eq!(report["cells"][0]["sample_ns"], Value::Null);
    // A forged quantile is not what admission recomputes, so it refuses.
    let mut forged = report.clone();
    forged["cells"][last]["stats"]["p50_ns"] = json!(3_000);
    assert_ne!(forged, api_chunk::api_report(&rec, &[]));
    // No warmup and no second invocation: either is a record that does not
    // add up.
    let mut two = rec.clone();
    let again = two.cells[last].measured.rows[0].clone();
    two.cells[last].measured.rows.push(again);
    two.cells[last].measured.rows[1].index = 1;
    two.cells[last].measured.started = 2;
    assert!(api_chunk::check_record(&two, &chunk).is_err());
    let mut warmed = rec.clone();
    warmed.cells[last].warmup = Some(api::Ran::default());
    assert!(api_chunk::check_record(&warmed, &chunk).is_err());
}

/// An MCP record whose first two cells measured, with the brackets the
/// runner would have kept and the holder windows they name.
fn bracketed() -> (runner::Chunk, api::ApiRecord, Vec<records::ResourceRow>) {
    let chunk = runner::Chunk::Api {
        surface: Surface::Mcp,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 0,
    };
    let mut rec = record_for(&chunk);
    rec.fill.reached = true;
    let plan = api_chunk::cells(TargetKind::Mailpit, Surface::Mcp);
    let b = |cmd: &str, id: Option<&str>, at: u64, owner: u64| api::Bracket {
        cmd: cmd.into(),
        id: id.map(str::to_owned),
        sent_ns: at,
        received_ns: at + 5,
        owner_t_ns: owner,
    };
    rec.control.push(b("idle_fill", None, 10, 500));
    let mut resources = Vec::new();
    let mut idle = cell_window("api.f0", records::WindowOutcome::Measured);
    idle.window = records::WindowKind::IdleFill;
    idle.end_ns = 500;
    resources.push(records::ResourceRow::Window(idle));
    for (n, (c, p)) in rec.cells.iter_mut().zip(&plan).take(2).enumerate() {
        c.status = api::CellStatus::Measured;
        c.blocked = None;
        c.warmup = Some(api::Ran {
            rows: rows(p.warmup, Phase::Warmup, Outcome::Exact),
            planned: p.warmup,
            started: p.warmup,
            ..Default::default()
        });
        c.measured = api::Ran {
            rows: rows(p.planned, Phase::Measured, Outcome::Exact),
            planned: p.planned,
            started: p.planned,
            ..Default::default()
        };
        let at = 100 * (n as u64 + 1);
        rec.control
            .push(b("window_start", Some(&c.cell), at, 1_000 * at));
        rec.control
            .push(b("window_end", Some(&c.cell), at + 50, 1_000 * at + 7));
        let mut w = cell_window(&format!("api.{}", c.cell), records::WindowOutcome::Measured);
        (w.start_ns, w.end_ns) = (1_000 * at, 1_000 * at + 7);
        resources.push(records::ResourceRow::Window(w));
    }
    (chunk, rec, resources)
}

#[test]
fn control_brackets_roundtrip_and_bind_to_the_holders_windows() {
    let (chunk, rec, resources) = bracketed();
    api_chunk::check_record(&rec, &chunk).unwrap();
    api_chunk::check_bracket_windows(&rec, &resources).unwrap();
    let back: api::ApiRecord = serde_json::from_slice(&serde_json::to_vec(&rec).unwrap()).unwrap();
    assert_eq!(back.control, rec.control);
    api_chunk::check_record(&back, &chunk).unwrap();
    let refuse = |f: &dyn Fn(&mut api::ApiRecord)| {
        let mut r = rec.clone();
        f(&mut r);
        api_chunk::check_record(&r, &chunk).unwrap_err()
    };
    // Missing, duplicate, reordered, foreign, out of local order.
    assert!(
        refuse(&|r| {
            r.control.remove(2);
        })
        .contains("brackets")
    );
    assert!(
        refuse(&|r| {
            let d = r.control[1].clone();
            r.control.insert(1, d);
        })
        .contains("brackets")
    );
    assert!(refuse(&|r| r.control.swap(1, 2)).contains("brackets"));
    assert!(refuse(&|r| r.control[1].id = Some("get.c32".into())).contains("brackets"));
    assert!(refuse(&|r| r.control[3].sent_ns = 0).contains("out of order"));
    assert!(
        refuse(&|r| r.control[3].received_ns = r.control[3].sent_ns - 1).contains("out of order")
    );
    // A window that ends before it starts on the holder's clock.
    assert!(refuse(&|r| r.control[2].owner_t_ns = 0).contains("holder's clock"));
    // No idle-fill bracket although the fill was reached.
    assert!(refuse(&|r| r.control.remove(0).cmd.clear()).contains("brackets"));
    // A native record carries none.
    let native = runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 0,
    };
    let mut n = record_for(&native);
    n.control = rec.control.clone();
    assert!(
        api_chunk::check_record(&n, &native)
            .unwrap_err()
            .contains("native")
    );
    // Bound to the holder's windows by its own clock only.
    let mut r = rec.clone();
    r.control[1].owner_t_ns += 1;
    assert!(api_chunk::check_bracket_windows(&r, &resources).is_err());
    let mut r = rec.clone();
    r.control[0].owner_t_ns = 499;
    assert!(api_chunk::check_bracket_windows(&r, &resources).is_err());
    assert!(api_chunk::check_bracket_windows(&rec, &resources[..1]).is_err());
}

#[test]
fn api_json_admits_only_when_it_adds_up_against_the_plan() {
    let chunk = runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 1000,
    };
    let rec = record_for(&chunk);
    api_chunk::check_record(&rec, &chunk).unwrap();
    // A cell missing, counts that do not add up, a duplicate index, a
    // started row in a blocked cell, another chunk's record.
    let mut r = rec.clone();
    r.cells.pop();
    assert!(
        api_chunk::check_record(&r, &chunk)
            .unwrap_err()
            .contains("not the plan's")
    );
    let mut r = rec.clone();
    r.cells[0].measured.unstarted = 99;
    assert!(
        api_chunk::check_record(&r, &chunk)
            .unwrap_err()
            .contains("does not add up")
    );
    let mut r = rec.clone();
    r.cells[0].status = api::CellStatus::Measured;
    r.cells[0].measured.rows = rows(2, Phase::Measured, Outcome::Exact);
    r.cells[0].measured.rows[1].index = 0;
    r.cells[0].measured.started = 2;
    r.cells[0].measured.unstarted = 98;
    assert!(
        api_chunk::check_record(&r, &chunk)
            .unwrap_err()
            .contains("does not add up")
    );
    let mut r = rec.clone();
    r.cells[0].measured.rows = rows(1, Phase::Measured, Outcome::Exact);
    r.cells[0].measured.started = 1;
    r.cells[0].measured.unstarted = 99;
    assert!(
        api_chunk::check_record(&r, &chunk)
            .unwrap_err()
            .contains("does not add up")
    );
    let other = runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::Mailpit,
        repetition: 1,
        fill: 1000,
    };
    assert!(
        api_chunk::check_record(&rec, &other)
            .unwrap_err()
            .contains("another chunk")
    );
    // The report is a function of the record: an edited count is visible.
    let a = api_chunk::api_report(&rec, &[]);
    let mut r = rec.clone();
    r.cells[3].measured.unstarted = 0;
    assert_ne!(a, api_chunk::api_report(&r, &[]));
    assert_eq!(a["cells"][0]["stats"]["p50_ns"], Value::Null);
    // Every API chunk plans its windows: three for the launch and fill, one per cell.
    let w = api_chunk::planned_windows(TargetKind::Mailpit, 0, Surface::Native, 1000);
    assert_eq!(w.len(), 3 + rec.cells.len());
}

// ---- the real candidate (T2, T6, T12) ---------------------------------------

async fn launch_candidate(root: &Path) -> targets::Owned {
    targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(api::CAP)),
        &root.join("leaf"),
        &limits(),
        &wire::Clock(Instant::now()),
        &targets::Keepers::default(),
    )
    .await
    .expect("the real binary becomes ready")
}

/// Every native cell of the candidate, in order, at fill 1,000: real SMTP
/// (per-test inboxes by AUTH), real HTTP, keep-alive clients per worker, the
/// exact checks and the destructive postchecks. Resource windows are the
/// leaf's and are proved by the ignored chunk proof; here no meter is held.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn the_candidates_native_cells_are_exact_end_to_end() {
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_candidate(root.path()).await;
    let clock = api::Clock(std::time::Instant::now());
    let client = clients::Client::new("swarmail", owned.http_addr).unwrap();
    let (fill, background) =
        api::fill_ledger(owned.smtp, &client, (163, "real"), 1_000, clock).await;
    assert!(fill.reached, "{fill:?}");
    assert_eq!((fill.accepted, fill.retained), (1_000, Some(1_000)));
    let background = background.unwrap();
    assert_eq!(background.len(), 1_000);
    let (cells, rows) = api::plan_cells(Surface::Native, |op| {
        clients::native_capability("swarmail", op)
    });
    assert!(rows.is_empty());
    let mut fixtures = api::plan_fixtures(163, &cells);
    api::prepare(owned.smtp, &client, &mut fixtures, None)
        .await
        .expect("fixtures resolve");
    assert!(fixtures.iter().all(|f| f.id.is_some()));
    let pool = (0..32)
        .map(|_| clients::Client::new("swarmail", owned.http_addr).unwrap())
        .collect();
    let native = api::NativeSurface {
        clients: Arc::new(pool),
        fixtures: Arc::new(fixtures),
    };
    let caps = |op| clients::native_capability("swarmail", op);
    let (mut recs, abandoned) = api::run_cells(&cells, &caps, &native, &api::NoMeter, clock, None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    let problems = api::final_ledger(
        &native.clients[0],
        (163, &background),
        &native.fixtures,
        &cells,
        &mut recs,
        None,
    )
    .await;
    assert!(problems.is_empty(), "{problems:?}");
    for r in &recs {
        let s = api::stats(&r.measured, None);
        assert_eq!(
            r.status,
            api::CellStatus::Measured,
            "{}: {:?}",
            r.cell,
            r.blocked
        );
        assert!(r.postcheck.is_empty(), "{}: {:?}", r.cell, r.postcheck);
        assert!(s.started > 0, "{}", r.cell);
        assert_eq!(
            s.exact,
            s.started,
            "{}: {:?}",
            r.cell,
            r.measured.rows.iter().find(|x| x.outcome != Outcome::Exact)
        );
        assert!(s.p50_ns.is_some());
    }
    // The destructive cells really removed what they say they removed.
    let mut spend = Spend::default();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let left = client
        .list(&api::delete_scope(0), deadline, &mut spend)
        .await
        .unwrap();
    let del = recs.iter().find(|r| r.cell == "delete.c1").unwrap();
    assert_eq!(
        left.len() as u32,
        120 - del.measured.started - 20 + api::SURVIVORS
    );
    let cleared = client
        .list(&api::clear_scope(2, 25), deadline, &mut spend)
        .await
        .unwrap();
    let clr = recs.iter().find(|r| r.cell == "clear.c32").unwrap();
    assert_eq!(cleared.is_empty(), 25 < 20 + clr.measured.started);
    // R6 on the real store: delete one survivor and one background message
    // behind the batch's back. The same ledger now finds both and turns
    // every destructive success into a wrong result.
    let survivor = native
        .fixtures
        .iter()
        .find(|f| f.role == api::Role::Survivor)
        .and_then(|f| f.id.clone())
        .unwrap();
    let bg = background.iter().next().unwrap().clone();
    for id in [&survivor, &bg] {
        clients::swarmail::delete(&client.conn, id, deadline, &mut spend)
            .await
            .unwrap();
    }
    let problems = api::final_ledger(
        &native.clients[0],
        (163, &background),
        &native.fixtures,
        &cells,
        &mut recs,
        None,
    )
    .await;
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("2 expected id(s) missing"),
        "{problems:?}"
    );
    for r in recs.iter().filter(|r| r.operation.destructive()) {
        assert!(
            r.measured.rows.iter().all(|x| x.outcome != Outcome::Exact),
            "{}",
            r.cell
        );
    }
    assert!(owned.shutdown().await.proved());
}

/// A missing-evidence or unready target never becomes a capability: a
/// fixture send to a closed port is a refusal of preparation, by name.
#[tokio::test]
async fn preparation_refuses_when_the_target_does_not_answer() {
    let dead = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let client = clients::Client::new("mailpit", dead).unwrap();
    let mut fixtures = vec![api::author(1, api::Role::Read, 0, &api::read_scope())];
    let w = api::prepare(dead, &client, &mut fixtures, None)
        .await
        .unwrap_err();
    assert!(w.contains("smtp connect"), "{w}");
    // A server that accepts and never greets holds preparation only to the
    // chunk's end, well inside its own ten-second step: a cut, by name.
    let mute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = mute.local_addr().unwrap();
    let held = tokio::spawn(async move {
        let mut keep = Vec::new();
        while let Ok((s, _)) = mute.accept().await {
            keep.push(s);
        }
    });
    let end = tokio::time::Instant::now() + Duration::from_millis(300);
    let w = api::prepare(addr, &client, &mut fixtures, Some(end))
        .await
        .unwrap_err();
    assert_eq!(w, "chunk_deadline");
    assert!(tokio::time::Instant::now() < end + api::SMTP_STEP);
    held.abort();
}

// ---- host proofs (AC10, R9) -------------------------------------------------
//
// Each runs one chunk of the real driver (`MAIL_COMPARE_BIN`, the built
// `mail_compare` bench executable) against the pinned executable named by
// `MAIL_COMPARE_<TARGET>` (and `MAIL_COMPARE_JRE` / `MAIL_COMPARE_MUSL` for
// GreenMail's and the Alkemio image's runtime roots), one test at a time.
// The driver must be the only process of a delegated cgroup v2 scope, and
// on the MCP surface it is not the test's child but `swarmail-evals
// api-mcp`'s: so the test does not run inside a scope itself. It places
// the driver — the native chunk's `run`, the MCP chunk's `hold` — alone in
// a scope of its own (`scoped_driver`, `systemd-run --user --scope -p
// Delegate=yes`, which execs it in place), and the adapters stay ordinary
// children of `api-mcp`. Run the test binary outside any scope:
//
//   bin=$(cargo test --locked --test mail_compare --no-run --message-format=json \
//     | grep -o '"executable":"[^"]*mail_compare-[^"]*"' | tail -n1 | cut -d'"' -f4)
//   cat /proc/loadavg
//   timeout 600 taskset -c 0-31 env MAIL_COMPARE_BIN=… MAIL_COMPARE_MAILPIT=… \
//     "$bin" --ignored --exact api_section::api_native_real_mailpit --test-threads=1
//   cat /proc/loadavg
//
// Wrapping the test binary in the scope instead leaves it, and the eval
// runner, beside the driver, which refuses: "the scope … holds 1 other
// process(es); the driver must be its only process".
//
// Without a variable, or without delegation, each fails — never skips. A
// pass is exact: every cell measured, every row exact or one of the
// outcomes the capability table and v3's fidelity records document, the
// ledger intact, every window clean, the chunk sealed, admitted and its
// cleanup proved. Exit 0 or 1 alone, or a few measured cells, is no pass.

fn env_path(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must name its pinned bytes"))
}

/// The `--bin` and `--runtime` arguments of `target`'s pinned bytes.
fn pinned_args(target: TargetKind) -> Vec<String> {
    let var = format!(
        "MAIL_COMPARE_{}",
        target.name().to_uppercase().replace('-', "_")
    );
    let bin = if target == TargetKind::Swarmail {
        env!("CARGO_BIN_EXE_swarmail").to_owned()
    } else {
        env_path(&var)
    };
    let mut out = vec!["--bin".into(), format!("{target}={bin}")];
    let runtime = match target {
        TargetKind::Greenmail => Some("MAIL_COMPARE_JRE"),
        TargetKind::AlkemioMailslurper => Some("MAIL_COMPARE_MUSL"),
        _ => None,
    };
    if let Some(var) = runtime {
        out.extend(["--runtime".into(), format!("{target}={}", env_path(var))]);
    }
    out
}

/// What a real chunk left: its exit, its record, its report and admission.
struct RealChunk {
    code: Option<i32>,
    text: String,
    rec: api::ApiRecord,
    report: Value,
    admitted: records::Admitted,
    /// An MCP chunk's client scope receipt (issue #385), judged the moment
    /// its command ended, and the unit it was for.
    client: Option<(String, client_scope::Receipt)>,
}

/// A driver path that runs `driver`'s resource-owning modes (`run`, `hold`)
/// as the only process of a delegated scope of their own, and every other
/// mode (an adapter) in place. `systemd-run --scope` execs the command, so
/// the process the caller waits on and signals is the driver itself.
fn scoped_driver(dir: &std::path::Path, driver: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let quoted = format!("'{}'", driver.replace('\'', r"'\''"));
    let path = dir.join("scoped-driver");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n  run|hold) exec systemd-run --user --scope --quiet \
             -p Delegate=yes -- {quoted} \"$@\" ;;\nesac\nexec {quoted} \"$@\"\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The client scope of an MCP host proof, as the collector makes it (issue
/// #385): a fresh, non-delegated scope unit whose first process is the
/// driver's `client-enter`, which records its membership and execs
/// `api-mcp` in place. Returns the command and the unit.
fn client_scoped(dir: &Path, chunk: &str) -> (std::process::Command, String) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!(
        "mail-compare-hostproof-{}-{nanos}-{chunk}-client",
        std::process::id()
    );
    let unit = format!("{name}.scope");
    let mut c = std::process::Command::new("systemd-run");
    c.args(["--user", "--scope", "--quiet"])
        .arg(format!("--unit={name}"))
        .arg("--")
        .arg(env_path("MAIL_COMPARE_BIN"))
        .args(["client-enter", "--unit", &unit, "--handshake"])
        .arg(dir.join("client-handshake.json"))
        .arg("--")
        .arg(env_path("SWARMAIL_EVALS_BIN"));
    (c, unit)
}

/// Judge an MCP host proof's client scope with the driver's own
/// `client-judge`, as the collector does: 3.5 s inside a `timeout` of 4 s,
/// its output in a file (a survivor holding an inherited pipe cannot delay
/// it), and clean only with exit 0 as well as a receipt that says so.
fn judge_client(dir: &Path, unit: &str) -> (Option<i32>, client_scope::Receipt) {
    let out = dir.join("client-receipt.json");
    let log = dir.join("client-judge.log");
    let judged = std::process::Command::new("timeout")
        .arg("4")
        .arg(env_path("MAIL_COMPARE_BIN"))
        .args(["client-judge", "--unit", unit, "--handshake"])
        .arg(dir.join("client-handshake.json"))
        .arg("--out")
        .arg(&out)
        .args(["--budget-ms", "3500"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&log).unwrap())
        .status()
        .unwrap();
    let bytes = std::fs::read(&out).unwrap_or_else(|e| {
        panic!(
            "client-judge ended with {judged} and left no receipt ({e}): {}",
            std::fs::read_to_string(&log).unwrap_or_default()
        )
    });
    (judged.code(), serde_json::from_slice(&bytes).unwrap())
}

fn real_chunk(surface: Surface, target: TargetKind, fill: u32) -> RealChunk {
    let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let driver = scoped_driver(root.path(), &env_path("MAIL_COMPARE_BIN"));
    let chunk = format!("api.{}.{target}.r0.f{fill}", surface.name());
    let mut client_unit = None;
    let mut cmd = match surface {
        Surface::Native => {
            let mut c = std::process::Command::new(&driver);
            c.args(["run", "--mode", "smoke"]);
            c
        }
        Surface::Mcp => {
            let (mut c, unit) = client_scoped(root.path(), &chunk);
            client_unit = Some(unit);
            c.args(["api-mcp", "--driver"])
                .arg(&driver)
                .args(["--mode", "smoke"]);
            c
        }
    };
    // The chunk's output goes to files, never pipes: a survivor that
    // inherited them would hold a pipe open, and the proof would wait for
    // its EOF instead of judging at the command's exit.
    let (stdout, stderr) = (root.path().join("stdout"), root.path().join("stderr"));
    let status = cmd
        .args(["--protocol", "docs/evals/draft/protocol.v4.json"])
        .args([
            "--sources",
            "docs/evals/draft/sources.lock.v5.json",
            "--documents",
            "docs/evals",
        ])
        .args(["--chunk", &chunk, "--run-id", "api-real"])
        .args(pinned_args(target))
        .arg("--output")
        .arg(root.path().join("out"))
        .arg("--scratch")
        .arg(root.path().join("scratch"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(std::process::Stdio::null())
        .stdout(std::fs::File::create(&stdout).unwrap())
        .stderr(std::fs::File::create(&stderr).unwrap())
        .status()
        .unwrap();
    // The client scope first, at once, as the collector judges it.
    let client = client_unit.map(|u| {
        let (code, r) = judge_client(root.path(), &u);
        (u, code, r)
    });
    let text = format!(
        "{}{}",
        std::fs::read_to_string(&stdout).unwrap_or_default(),
        std::fs::read_to_string(&stderr).unwrap_or_default()
    );
    // Clean before anything else is read: the admission, the records and
    // the documented fill shortfall come after it.
    let client = client.map(|(unit, code, r)| {
        eprintln!(
            "host proof {chunk}: client-judge exit {code:?}: {}",
            r.summary()
        );
        assert_eq!(code, Some(0), "{chunk}: client-judge must exit 0\n{text}");
        r.proves_clean(&unit)
            .unwrap_or_else(|e| panic!("{chunk}: {e}\n{text}"));
        (unit, r)
    });
    let dir = root.path().join("out");
    let admitted =
        records::admit(&dir).unwrap_or_else(|e| panic!("{chunk} must admit: {e}\n{text}"));
    let rec = serde_json::from_slice(&std::fs::read(dir.join("api.json")).unwrap()).unwrap();
    let report = serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    RealChunk {
        code: status.code(),
        text,
        rec,
        report,
        admitted,
        client,
    }
}

/// The failure reasons a real chunk may show without being wrong (0132
/// §6.2, §7): a bridged call above the bridge's 16 slots or an adapter's
/// queue is a published busy refusal, Swarmail's tool rate a published
/// `rate_limited` one. Nothing else may fail.
fn tolerated(surface: Surface, target: TargetKind) -> &'static [&'static str] {
    match (surface, target) {
        (Surface::Native, _) => &[],
        (Surface::Mcp, TargetKind::Swarmail) => &["rate_limited"],
        (Surface::Mcp, _) => &["busy_bridge", "busy_adapter"],
    }
}

/// The one inexact outcome v3 documents: MailSlurper's absent body part,
/// on reads of the message.
fn documented_inexact(target: TargetKind, op: Operation, o: &Outcome) -> bool {
    matches!(o, Outcome::InexactFidelity { detail } if detail.ends_with(" absent"))
        && matches!(
            target,
            TargetKind::MailslurperSourceBuild | TargetKind::AlkemioMailslurper
        )
        && matches!(
            op,
            Operation::Get | Operation::ExtractCode | Operation::ExtractLink
        )
}

/// Whether a target's own documents record that it acknowledges mail it
/// then does not keep: README, "it acknowledges a mail before it stores it
/// and lost some under SQLite lock contention" — the source-built
/// MailSlurper only. Accepted 0130 records that Alkemio's fork left
/// `loses_under_burst` (seven of seven 600/600 burst runs), so its family
/// sentence documents no shortfall of its own. Only a documented target may
/// answer a host proof with `fill_not_reached`; for any other, a missed
/// fill fails the proof.
fn fill_shortfall_documented(target: TargetKind) -> bool {
    target == TargetKind::MailslurperSourceBuild
}

/// A `fill_not_reached` record held to 0132 §6.4 exactly: a target whose
/// shortfall is documented, every send terminal and counted, the retained
/// count read and short, the store read until the fill's 300 s bound, the
/// shortfall named in the record's own words, and every planned cell
/// blocked `fill_not_reached` with nothing started, no fixture, no ledger
/// and nothing abandoned.
fn fill_shortfall_exact(target: TargetKind, rec: &api::ApiRecord) -> Result<(), String> {
    let f = &rec.fill;
    let n = u64::from(f.fill);
    if !fill_shortfall_documented(target) {
        return Err(format!("{target} has no documented shortfall: {f:?}"));
    }
    if f.reached || f.accepted + f.refused + f.failed + f.ambiguous != n {
        return Err(format!("not every send is counted: {f:?}"));
    }
    let Some(kept) = f.retained.filter(|r| *r < n) else {
        return Err(format!("no retained count short of the fill: {f:?}"));
    };
    let named = format!(
        "accepted {} retained Some({kept}) of {n} within {} s",
        f.accepted,
        api::FILL_DEADLINE.as_secs()
    );
    if f.detail.as_deref() != Some(named.as_str()) {
        return Err(format!("the shortfall is not named as {named:?}: {f:?}"));
    }
    if f.accepted == n
        && (f.elapsed_ns as u128) < (api::FILL_DEADLINE - api::SETTLE_POLL).as_nanos()
    {
        return Err(format!(
            "the store was not read until the fill's bound: {f:?}"
        ));
    }
    if rec.fixtures != 0 || !rec.ledger.is_empty() || rec.abandoned.is_some() {
        return Err("a fixture, ledger or abandonment after a missed fill".into());
    }
    for c in &rec.cells {
        let m = &c.measured;
        let blocked = c.status == api::CellStatus::Blocked
            && c.blocked.as_deref() == Some("fill_not_reached")
            && c.warmup.is_none()
            && m.rows.is_empty()
            && m.started == 0
            && m.unstarted == m.planned;
        if !blocked {
            return Err(format!("{} is not blocked fill_not_reached", c.cell));
        }
    }
    Ok(())
}

#[test]
fn a_missed_fill_passes_a_host_proof_only_exactly_and_only_where_documented() {
    // 0132 §6.4's documented outcome, as the host proofs accept it.
    let chunk = runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::MailslurperSourceBuild,
        repetition: 0,
        fill: 1_000,
    };
    let mut rec = record_for(&chunk);
    rec.fill = api::FillRecord {
        fill: 1_000,
        accepted: 1_000,
        retained: Some(764),
        reached: false,
        elapsed_ns: 299_936_025_956,
        detail: Some("accepted 1000 retained Some(764) of 1000 within 300 s".into()),
        ..Default::default()
    };
    let ms = TargetKind::MailslurperSourceBuild;
    assert_eq!(fill_shortfall_exact(ms, &rec), Ok(()));
    // Never for a target without a documented shortfall, Alkemio's fork
    // included (0130: it left loses_under_burst).
    for t in [
        TargetKind::AlkemioMailslurper,
        TargetKind::Swarmail,
        TargetKind::Mailpit,
        TargetKind::Inbucket,
    ] {
        assert!(fill_shortfall_exact(t, &rec).is_err(), "{t}");
    }
    let refuses = |edit: &dyn Fn(&mut api::ApiRecord)| {
        let mut r = rec.clone();
        edit(&mut r);
        fill_shortfall_exact(ms, &r).unwrap_err()
    };
    // The shortfall must be counted, short, named and read to the bound.
    assert!(refuses(&|r| r.fill.retained = None).contains("retained"));
    assert!(refuses(&|r| r.fill.retained = Some(1_000)).contains("retained"));
    assert!(refuses(&|r| r.fill.accepted = 990).contains("counted"));
    assert!(refuses(&|r| r.fill.reached = true).contains("counted"));
    assert!(refuses(&|r| r.fill.detail = Some("short".into())).contains("named"));
    assert!(refuses(&|r| r.fill.elapsed_ns = 2_000_000_000).contains("bound"));
    // And nothing may have run after it.
    assert!(refuses(&|r| r.fixtures = 4).contains("fixture"));
    assert!(refuses(&|r| r.ledger = vec!["x".into()]).contains("ledger"));
    assert!(refuses(&|r| r.cells[0].blocked = Some("warmup_failed".into())).contains("list.c1"));
    assert!(refuses(&|r| r.cells[1].measured.started = 1).contains("list.c8"));
    // A send refused, failed or cut ambiguous is a counted shortfall too.
    let mut r = rec.clone();
    (r.fill.accepted, r.fill.ambiguous, r.fill.retained) = (992, 8, Some(992));
    r.fill.elapsed_ns = 1_000_000;
    r.fill.detail = Some("accepted 992 retained Some(992) of 1000 within 300 s".into());
    assert_eq!(fill_shortfall_exact(ms, &r), Ok(()));
}

/// The measured indices of `ran` that its cell's five-second bound cut (R1):
/// a `timeout` row whose end is at the bound — not before it, and within
/// the local cancellation bound after it — and that is the last row its
/// worker started. Each worker has at most one. Any other `timeout` is not
/// in the set, and two cuts on one worker, a cut its worker started after,
/// or cuts in a cell with no exact row, is an error.
fn cell_bound_cuts(ran: &api::Ran) -> Result<BTreeSet<u32>, String> {
    let bound = ran.start_ns + api::CELL_DEADLINE.as_nanos() as u64;
    let late = bound + api::CANCEL_BOUND.as_nanos() as u64;
    let mut cuts = BTreeSet::new();
    let mut workers = BTreeSet::new();
    for r in &ran.rows {
        if r.outcome != Outcome::Timeout || r.end_ns < bound || r.end_ns > late {
            continue;
        }
        if !workers.insert(r.worker) {
            return Err(format!("worker {} has two rows cut at the bound", r.worker));
        }
        let after = ran
            .rows
            .iter()
            .any(|x| x.worker == r.worker && x.start_ns > r.start_ns);
        if after {
            return Err(format!(
                "worker {} started again after its row {} was cut",
                r.worker, r.index
            ));
        }
        cuts.insert(r.index);
    }
    // A cut is the bound reaching a call in flight, never the cell's whole
    // answer: a cell the bound cut must also have measured something
    // exactly, so a route that hangs on every worker cannot pass (R1, R9).
    if !cuts.is_empty() && !ran.rows.iter().any(|r| r.outcome == Outcome::Exact) {
        return Err(format!(
            "{} of {} started rows cut at the bound and none exact",
            cuts.len(),
            ran.started
        ));
    }
    Ok(cuts)
}

#[test]
fn only_a_row_cut_at_its_cell_bound_is_a_host_proofs_timeout() {
    // The host proof's acceptance of R1's read timeout: one row per worker,
    // ending at the cell's bound, the last its worker started. A timeout
    // before the bound, long after it, a second on one worker, or one its
    // worker started after, is not.
    let s = 1_000_000_000u64;
    let bound = s + api::CELL_DEADLINE.as_nanos() as u64;
    let row = |index, worker, start_ns, end_ns, outcome| api::OpRow {
        phase: Phase::Measured,
        index,
        worker,
        start_ns,
        end_ns,
        outcome,
        requests: 1,
        bytes: 0,
        wait: None,
    };
    let ran = |rows: Vec<api::OpRow>| api::Ran {
        planned: 100,
        started: rows.len() as u32,
        unstarted: 100 - rows.len() as u32,
        rows,
        start_ns: s,
        end_ns: bound + 1_000,
        ..api::Ran::default()
    };
    let cut = |index, worker| {
        row(
            index,
            worker,
            bound - 50_000_000,
            bound + 1_000,
            Outcome::Timeout,
        )
    };
    let done = |index, worker, at| row(index, worker, at, at + 1_000, Outcome::Exact);
    // Two workers, each cut once at the bound: both indices.
    let ok = ran(vec![done(0, 0, s), done(1, 1, s), cut(2, 0), cut(3, 1)]);
    assert_eq!(cell_bound_cuts(&ok), Ok([2, 3].into()));
    // A timeout well before the bound, or past the cancellation bound, is
    // not a cut: it stays a rejected row.
    let early = row(0, 0, s, bound - 1, Outcome::Timeout);
    let late = row(
        0,
        0,
        s,
        bound + api::CANCEL_BOUND.as_nanos() as u64 + 1,
        Outcome::Timeout,
    );
    for r in [early, late] {
        assert_eq!(cell_bound_cuts(&ran(vec![r])), Ok(BTreeSet::new()));
    }
    // Two on one worker, or a row its worker started after the cut.
    assert!(cell_bound_cuts(&ran(vec![cut(0, 0), cut(1, 0)])).is_err());
    let mut again = cut(0, 0);
    again.start_ns = s;
    let e = cell_bound_cuts(&ran(vec![again, done(1, 0, s + 10)])).unwrap_err();
    assert!(e.contains("started again"), "{e}");
    // A cell whose every started row the bound cut — one worker, or every
    // worker of c32 — measured nothing exact and is refused.
    let e = cell_bound_cuts(&ran(vec![cut(0, 0)])).unwrap_err();
    assert_eq!(e, "1 of 1 started rows cut at the bound and none exact");
    let all = ran((0..32).map(|w| cut(w, w)).collect());
    let e = cell_bound_cuts(&all).unwrap_err();
    assert_eq!(e, "32 of 32 started rows cut at the bound and none exact");
    // Cuts beside only non-exact rows are refused too; one exact row admits.
    let refused = row(
        0,
        0,
        s,
        s + 1_000,
        Outcome::Failed {
            reason: "busy".into(),
        },
    );
    assert!(cell_bound_cuts(&ran(vec![refused.clone(), cut(1, 1)])).is_err());
    assert_eq!(
        cell_bound_cuts(&ran(vec![refused, done(1, 1, s), cut(2, 1)])),
        Ok([2].into())
    );
}

/// Assert a real chunk is exact as the README defines it; `true` when its
/// fill was reached and its cells measured, `false` for a documented
/// `fill_not_reached` held to [`fill_shortfall_exact`].
fn assert_real_chunk(surface: Surface, target: TargetKind, c: &RealChunk) -> bool {
    let ctx = format!("{} {target}", surface.name());
    let rec = &c.rec;
    // The plan's matrix: every planned cell in order, every unsupported
    // operation a published capability row, the fill reached, nothing
    // abandoned, the complete ledger intact.
    let (cells, caps) = api::plan_cells(surface, |op| {
        clients::capability(target.name(), surface, op)
    });
    assert_eq!(
        rec.cells.iter().map(|x| x.cell.clone()).collect::<Vec<_>>(),
        cells.iter().map(PlannedCell::name).collect::<Vec<_>>(),
        "{ctx}"
    );
    assert_eq!(rec.capabilities, caps, "{ctx}");
    if !rec.fill.reached {
        assert_fill_not_reached(&ctx, target, c);
        return false;
    }
    assert_eq!(rec.abandoned, None, "{ctx}");
    assert!(rec.ledger.is_empty(), "{ctx}: {:?}", rec.ledger);
    let ok = tolerated(surface, target);
    // A wait cell the five-second cutoff reached (an adapter's 0129 poll
    // answers at its 100 ms tick, so wait.c1's hundred attempts do not
    // fit): the attempt its own end took is a timeout that kept its 250,
    // one with too little time left sent nothing (`wait_no_time`). Both
    // are failure rows of a cell that still measured, never uncertainty.
    let cut_wait = |r: &api::OpRow, op| {
        op == Operation::Wait
            && !api::unresolved(op, r)
            && match &r.outcome {
                Outcome::Timeout => r.wait.as_ref().is_some_and(|w| w.ack_ns.is_some()),
                Outcome::Failed { reason } => reason == "wait_no_time",
                _ => false,
            }
    };
    let fine = |r: &api::OpRow, op| match &r.outcome {
        Outcome::Exact => true,
        _ if cut_wait(r, op) => true,
        Outcome::Failed { reason } => ok.contains(&reason.as_str()),
        o => documented_inexact(target, op, o),
    };
    let mut failures = 0;
    for cell in &rec.cells {
        let warm = cell.warmup.iter().flat_map(|w| w.rows.iter());
        if cell.status == api::CellStatus::Blocked {
            // Only a warmup a tolerated refusal hit may block a cell.
            assert_eq!(
                cell.blocked.as_deref(),
                Some("warmup_failed"),
                "{ctx} {}",
                cell.cell
            );
            assert!(!ok.is_empty(), "{ctx} {}: blocked", cell.cell);
            assert!(
                warm.clone().all(|r| fine(r, cell.operation)),
                "{ctx} {}",
                cell.cell
            );
            failures += 1;
            continue;
        }
        assert!(
            cell.postcheck.is_empty(),
            "{ctx} {}: {:?}",
            cell.cell,
            cell.postcheck
        );
        assert!(cell.measured.started > 0, "{ctx} {}", cell.cell);
        let cuts =
            cell_bound_cuts(&cell.measured).unwrap_or_else(|e| panic!("{ctx} {}: {e}", cell.cell));
        for r in &cell.measured.rows {
            assert!(
                fine(r, cell.operation) || cuts.contains(&r.index),
                "{ctx} {}: {r:?}",
                cell.cell
            );
            failures += usize::from(matches!(
                r.outcome,
                Outcome::Failed { .. } | Outcome::Timeout
            ));
            if let (Some(w), Outcome::Exact) = (&r.wait, &r.outcome) {
                let ack = w.ack_ns.expect("an exact wait was acknowledged");
                assert!(w.call_ns <= w.send_ns, "{ctx} {r:?}");
                assert_eq!(w.ack_to_return_ns, Some(w.return_ns as i64 - ack as i64));
            }
        }
    }
    // The single-worker reads carry no load of their own: always measured.
    for name in ["list.c1", "search.c1", "get.c1"] {
        let cell = rec.cells.iter().find(|x| x.cell == name).unwrap();
        assert_eq!(cell.status, api::CellStatus::Measured, "{ctx} {name}");
    }
    // Exit 0 exactly when nothing failed; a tolerated refusal is still a
    // failure row, so the chunk seals with 1 and says so.
    let want = if failures == 0 { 0 } else { 1 };
    assert_eq!(c.code, Some(want), "{ctx}: {}", c.text);
    // Resources: every measured cell's window clean with counters, the
    // idle-fill point taken, samples present.
    for cell in rec
        .cells
        .iter()
        .filter(|x| x.status == api::CellStatus::Measured)
    {
        let profile = format!("api.{}", cell.cell);
        let w = c.admitted.resources.iter().find_map(|r| match r {
            records::ResourceRow::Window(w)
                if w.window == records::WindowKind::ApiCell && w.profile == profile =>
            {
                Some(w)
            }
            _ => None,
        });
        let w = w.unwrap_or_else(|| panic!("{ctx} {}: no window", cell.cell));
        assert_eq!(
            w.outcome,
            records::WindowOutcome::Measured,
            "{ctx} {}",
            cell.cell
        );
        assert!(w.counters().is_some(), "{ctx} {}", cell.cell);
    }
    let idle_fill = c.admitted.resources.iter().any(|r| {
        matches!(r, records::ResourceRow::Window(w)
            if w.window == records::WindowKind::IdleFill
                && w.outcome == records::WindowOutcome::Measured)
    });
    assert!(idle_fill, "{ctx}");
    let sampled = c
        .admitted
        .resources
        .iter()
        .any(|r| matches!(r, records::ResourceRow::Sample(_)));
    assert!(sampled, "{ctx}");
    // Cleanup: every launch's receipt proves its leaf removed.
    let cleanups = c.report["cleanups"].as_array().expect("cleanups");
    assert!(!cleanups.is_empty(), "{ctx}");
    assert!(
        cleanups.iter().all(|x| x["cgroup_removed"] == json!(true)),
        "{ctx}"
    );
    // The report's API figures are api.json's, recomputed.
    assert_eq!(
        c.report["api"],
        api_chunk::api_report(rec, &c.admitted.resources),
        "{ctx}"
    );
    true
}

/// A real chunk whose fill was not reached (0132 §6.4), exactly: the record
/// by [`fill_shortfall_exact`]; exit 1 naming the shortfall; the readiness
/// windows measured with counters, the idle-fill point and every planned
/// cell's window published `fill_not_reached`, samples present, the cleanup
/// proved and the report recomputed.
fn assert_fill_not_reached(ctx: &str, target: TargetKind, c: &RealChunk) {
    let rec = &c.rec;
    fill_shortfall_exact(target, rec).unwrap_or_else(|e| panic!("{ctx}: {e}"));
    assert_eq!(c.code, Some(1), "{ctx}: {}", c.text);
    assert!(c.text.contains("fill_not_reached"), "{ctx}: {}", c.text);
    let windows: Vec<&records::WindowRow> = c
        .admitted
        .resources
        .iter()
        .filter_map(|r| match r {
            records::ResourceRow::Window(w) => Some(w),
            records::ResourceRow::Sample(_) => None,
        })
        .collect();
    use records::{WindowKind as K, WindowOutcome as O};
    for kind in [K::Ready, K::IdleReady] {
        let w: Vec<_> = windows.iter().filter(|w| w.window == kind).collect();
        assert_eq!(w.len(), 1, "{ctx} {kind:?}");
        assert_eq!(w[0].outcome, O::Measured, "{ctx} {kind:?}");
        assert!(w[0].counters().is_some(), "{ctx} {kind:?}");
    }
    let idle_fill: Vec<_> = windows.iter().filter(|w| w.window == K::IdleFill).collect();
    assert_eq!(idle_fill.len(), 1, "{ctx}");
    assert_eq!(idle_fill[0].outcome, O::FillNotReached, "{ctx}");
    for cell in &rec.cells {
        let profile = format!("api.{}", cell.cell);
        let w: Vec<_> = windows
            .iter()
            .filter(|w| w.window == K::ApiCell && w.profile == profile)
            .collect();
        assert_eq!(w.len(), 1, "{ctx} {}", cell.cell);
        assert_eq!(w[0].outcome, O::FillNotReached, "{ctx} {}", cell.cell);
    }
    let sampled = c
        .admitted
        .resources
        .iter()
        .any(|r| matches!(r, records::ResourceRow::Sample(_)));
    assert!(sampled, "{ctx}");
    let cleanups = c.report["cleanups"].as_array().expect("cleanups");
    assert!(!cleanups.is_empty(), "{ctx}");
    assert!(
        cleanups.iter().all(|x| x["cgroup_removed"] == json!(true)),
        "{ctx}"
    );
    assert_eq!(
        c.report["api"],
        api_chunk::api_report(rec, &c.admitted.resources),
        "{ctx}"
    );
}

/// The line a host proof prints when it passed on 0132 §6.4's documented
/// `fill_not_reached` rather than on measured cells: such a pass proves the
/// shortfall exactly and nothing of the target's routes, waits or matrix,
/// so its receipt must say which it was.
const FILL_NOT_REACHED_PASS: &str =
    "host proof passed on fill_not_reached (0132 §6.4): no cell measured, routes not exercised:";

fn native_real(target: TargetKind) {
    let c = real_chunk(Surface::Native, target, 1_000);
    if !assert_real_chunk(Surface::Native, target, &c) {
        eprintln!("{FILL_NOT_REACHED_PASS} native {target}");
        return;
    }
    // Meaningful destruction where supported: some delete or clear landed.
    for cell in c.rec.cells.iter().filter(|x| x.operation.destructive()) {
        let landed = cell
            .measured
            .rows
            .iter()
            .any(|r| r.outcome == Outcome::Exact);
        assert!(landed, "{target} {}", cell.cell);
    }
}

/// The prerequisites of an MCP host proof.
fn mcp_vars(target: TargetKind) -> Vec<String> {
    let mut v = vec![
        "MAIL_COMPARE_BIN".to_owned(),
        "SWARMAIL_EVALS_BIN".to_owned(),
    ];
    if target != TargetKind::Swarmail {
        v.push(format!(
            "MAIL_COMPARE_{}",
            target.name().to_uppercase().replace('-', "_")
        ));
    }
    v
}

/// The client scope of a real MCP chunk was judged, and clean: no process
/// `api-mcp` started (the bridge, an adapter, a descendant) outlived it,
/// and the kernel placed the workload in that scope, not in the holder's.
/// The holder is still its own delegated scope's only process: `hold`
/// refuses otherwise, and the chunk was admitted.
fn assert_client_clean(target: TargetKind, c: &RealChunk) {
    let (unit, r) = c.client.as_ref().expect("the client scope was judged");
    r.proves_clean(unit)
        .unwrap_or_else(|e| panic!("mcp {target}: {e}\n{}", c.text));
    let cgroup = r.cgroup.as_deref().unwrap();
    assert!(
        cgroup.ends_with(&format!("/{unit}")),
        "mcp {target}: {cgroup}"
    );
    eprintln!(
        "host proof client scope clean: mcp {target}: {}",
        r.summary()
    );
}

fn mcp_real(target: TargetKind) {
    need(
        &mcp_vars(target)
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let c = real_chunk(Surface::Mcp, target, 1_000);
    // Before any other assertion, the documented fill shortfall included.
    assert_client_clean(target, &c);
    if !assert_real_chunk(Surface::Mcp, target, &c) {
        eprintln!("{FILL_NOT_REACHED_PASS} mcp {target}");
        return;
    }
    // The coordinated future wait ran and returned exactly its own message.
    let wait = c.rec.cells.iter().find(|x| x.cell == "wait.c1").unwrap();
    assert_eq!(wait.status, api::CellStatus::Measured, "{target}");
    let exact = wait
        .measured
        .rows
        .iter()
        .any(|r| r.outcome == Outcome::Exact);
    assert!(exact, "{target}");
    if target != TargetKind::Swarmail {
        // An adapter's delete and clear are cited not_supported rows.
        for op in [Operation::Delete, Operation::Clear] {
            let row = c.rec.capabilities.iter().find(|r| r.operation == op);
            let row = row.unwrap_or_else(|| panic!("{target}: no {op:?} row"));
            assert_eq!(row.status, "not_supported", "{target} {op:?}");
            assert!(!row.cites.is_empty(), "{target} {op:?}");
        }
    }
}

macro_rules! host_proofs {
    ($($name:ident => $f:ident($t:expr), $why:literal;)*) => {$(
        #[test]
        #[ignore = $why]
        fn $name() {
            $f($t)
        }
    )*};
}

host_proofs! {
    api_native_real_swarmail => native_real(TargetKind::Swarmail),
        "host proof: MAIL_COMPARE_BIN, a delegated cgroup v2 scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_mailpit => native_real(TargetKind::Mailpit),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_MAILPIT, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_mailhog => native_real(TargetKind::Mailhog),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_MAILHOG, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_mailslurper_source_build => native_real(TargetKind::MailslurperSourceBuild),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_smtp4dev => native_real(TargetKind::Smtp4dev),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_SMTP4DEV, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_inbucket => native_real(TargetKind::Inbucket),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_INBUCKET, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_greenmail => native_real(TargetKind::Greenmail),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_GREENMAIL, MAIL_COMPARE_JRE, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_native_real_alkemio_mailslurper => native_real(TargetKind::AlkemioMailslurper),
        "host proof: MAIL_COMPARE_BIN, MAIL_COMPARE_ALKEMIO_MAILSLURPER, MAIL_COMPARE_MUSL, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_mcp_real_swarmail => mcp_real(TargetKind::Swarmail),
        "host proof: MAIL_COMPARE_BIN, SWARMAIL_EVALS_BIN, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_mcp_bridge_real_mailpit => mcp_real(TargetKind::Mailpit),
        "host proof: MAIL_COMPARE_BIN, SWARMAIL_EVALS_BIN, MAIL_COMPARE_MAILPIT, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_mcp_bridge_real_mailhog => mcp_real(TargetKind::Mailhog),
        "host proof: MAIL_COMPARE_BIN, SWARMAIL_EVALS_BIN, MAIL_COMPARE_MAILHOG, a delegated scope; run alone under timeout 600 taskset -c 0-31";
    api_mcp_bridge_real_mailslurper_source_build => mcp_real(TargetKind::MailslurperSourceBuild),
        "host proof: MAIL_COMPARE_BIN, SWARMAIL_EVALS_BIN, MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD, a delegated scope; run alone under timeout 600 taskset -c 0-31";
}

#[test]
fn every_launchable_route_has_its_host_proof() {
    // The proofs above cover every target that launches on each surface,
    // nothing fewer (R9). The official MailSlurper launches on neither.
    let native = [
        "swarmail",
        "mailpit",
        "mailhog",
        "mailslurper-source-build",
        "smtp4dev",
        "inbucket",
        "greenmail",
        "alkemio-mailslurper",
    ];
    for t in &native {
        assert!(
            clients::native_capability(t, Operation::List).supported(),
            "{t}"
        );
    }
    assert_eq!(
        clients::native_capability("mailslurper", Operation::List).status(),
        "does_not_start"
    );
    let mcp: Vec<&str> = native
        .iter()
        .copied()
        .filter(|t| clients::has_mcp_surface(t))
        .collect();
    assert_eq!(
        mcp,
        ["swarmail", "mailpit", "mailhog", "mailslurper-source-build"]
    );
}

/// Fill 100,000 into the real candidate within the 300 s bound and confirm
/// every retained id by a complete scan of the store it went to.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "host proof: 100,000 messages; run alone under timeout 600 taskset -c 0-31"]
async fn api_fill_100k_real() {
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_candidate(root.path()).await;
    let clock = api::Clock(std::time::Instant::now());
    let client = clients::Client::new("swarmail", owned.http_addr).unwrap();
    let (fill, background) =
        api::fill_ledger(owned.smtp, &client, (163, "f100k"), 100_000, clock).await;
    assert!(fill.reached, "{fill:?}");
    let background = background.expect("the retained ids were read");
    assert!(fill.elapsed_ns < api::FILL_DEADLINE.as_nanos() as u64);
    // Every id, distinct, by a complete scan of the default inbox.
    let default = Scope {
        address: String::new(),
        inbox: "default".into(),
    };
    let mut spend = Spend::default();
    let all = clients::swarmail::list(
        &client.conn,
        &default,
        None,
        tokio::time::Instant::now() + Duration::from_secs(300),
        &mut spend,
    )
    .await
    .unwrap();
    let ids: BTreeSet<String> = all.iter().map(|s| s.id.clone()).collect();
    assert_eq!((all.len(), ids.len()), (100_000, 100_000));
    // The retained ids the ledger holds the store to are exactly the scan's.
    assert_eq!(ids, background);
    assert!(owned.shutdown().await.proved());
}

// ---- hold --control api-v1 (T7) ---------------------------------------------

#[test]
fn control_lines_are_closed_bounded_and_ordered() {
    use api_chunk::{Control, ControlState, admit_control, parse_control, windows_agree};
    let cells = api_chunk::cells(TargetKind::Mailpit, Surface::Mcp);
    assert_eq!(
        parse_control(br#"{"cmd":"window_start","id":"list.c1"}"#).unwrap(),
        Control::WindowStart {
            id: "list.c1".into()
        }
    );
    assert_eq!(
        parse_control(br#"{"cmd":"idle_fill"}"#).unwrap(),
        Control::IdleFill {}
    );
    for bad in [
        &br#"{"cmd":"reboot"}"#[..],
        br#"{"cmd":"idle_fill","x":1}"#,
        br#"{"cmd":"window_end"}"#,
        b"not json",
    ] {
        assert!(
            parse_control(bad).is_err(),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
    let long = format!(
        "{{\"cmd\":\"window_start\",\"id\":\"{}\"}}",
        "x".repeat(2000)
    );
    assert!(
        parse_control(long.as_bytes())
            .unwrap_err()
            .contains("exceeds")
    );
    let mut st = ControlState::default();
    let start = |id: &str| Control::WindowStart { id: id.into() };
    let end = |id: &str| Control::WindowEnd { id: id.into() };
    assert!(
        admit_control(&start("delete.c1"), &st, &cells)
            .unwrap_err()
            .contains("not a planned cell")
    );
    assert!(
        admit_control(&end("list.c1"), &st, &cells)
            .unwrap_err()
            .contains("was not opened")
    );
    admit_control(&Control::IdleFill {}, &st, &cells).unwrap();
    st.idle_fill = true;
    assert!(
        admit_control(&Control::IdleFill {}, &st, &cells)
            .unwrap_err()
            .contains("already taken")
    );
    admit_control(&start("list.c1"), &st, &cells).unwrap();
    let snap = resources::Snap {
        t_ns: 1,
        cpu: Default::default(),
        events: Default::default(),
    };
    st.opened.insert("list.c1".into(), snap);
    assert!(
        admit_control(&start("list.c8"), &st, &cells)
            .unwrap_err()
            .contains("already open")
    );
    st.opened.clear();
    st.closed.insert("list.c1".into());
    assert!(
        admit_control(&start("list.c1"), &st, &cells)
            .unwrap_err()
            .contains("already opened")
    );
    // The record must agree with the windows: a measured cell whose window
    // never opened, or a blocked one whose window did, refuses.
    let chunk = runner::Chunk::Api {
        surface: Surface::Mcp,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 0,
    };
    let rec = record_for(&chunk);
    assert!(windows_agree(&st, &rec).unwrap_err().contains("list.c1"));
    let mut fresh = ControlState::default();
    windows_agree(&fresh, &rec).unwrap();
    fresh.record = Some(rec.clone());
    assert!(
        admit_control(&Control::IdleFill {}, &fresh, &cells)
            .unwrap_err()
            .contains("after the record")
    );
    // A silent hold keeps its meaning: the control mode is opt-in, and a
    // driver run refuses mcp chunks while the holder refuses native ones.
    let p = Protocol::parse(DRAFT_V4).unwrap();
    let w = runner::v4_preflight_for(
        Surface::Mcp,
        &p,
        "smoke",
        Some("api.native.mailpit.r0.f0"),
        None,
        None,
    )
    .unwrap_err();
    assert!(w.contains("api.mcp chunks only"), "{w}");
    assert!(
        runner::v4_preflight_for(
            Surface::Mcp,
            &p,
            "smoke",
            Some("api.mcp.mailpit.r0.f0"),
            None,
            None
        )
        .is_ok()
    );
}

// ---- adapter documents (R2) -------------------------------------------------

const ADAPTED: [TargetKind; 3] = [
    TargetKind::Mailpit,
    TargetKind::Mailhog,
    TargetKind::MailslurperSourceBuild,
];

/// Copy the documents the draft lock pins under a fresh root.
fn documents_copy() -> tempfile::TempDir {
    let d = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let lock: Value = serde_json::from_slice(
        &std::fs::read(root().join("docs/evals/draft/sources.lock.v5.json")).unwrap(),
    )
    .unwrap();
    for e in lock["entries"].as_array().unwrap() {
        for doc in e["pin"]["documents"].as_array().into_iter().flatten() {
            let rel = doc["path"].as_str().unwrap();
            let to = d.path().join(rel);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(root().join("docs/evals").join(rel), to).unwrap();
        }
    }
    d
}

fn mailpit_doc(lock: &Path) -> String {
    let v: Value = serde_json::from_slice(&std::fs::read(lock).unwrap()).unwrap();
    v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["target"] == "mailpit")
        .unwrap()["pin"]["documents"][0]["path"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn every_adapter_serves_the_draft_lock_from_its_documents_root() {
    let draft = root().join("docs/evals/draft/sources.lock.v5.json");
    let docs = root().join("docs/evals");
    // The documented draft command: the lock under draft/, its documents
    // under docs/evals. Every adapter accepts it.
    for kind in ADAPTED {
        mcp::adapter_sources(kind, &draft, Some(&docs)).unwrap_or_else(|e| panic!("{kind}: {e}"));
    }
    // Without --documents the draft lock's own directory holds none of them:
    // refused, never served from wherever the bytes happen to be.
    let e = mcp::adapter_sources(TargetKind::Mailpit, &draft, None).unwrap_err();
    assert!(e.starts_with("source documents:"), "{e}");
    // The default — documents beside the lock — still holds.
    for kind in ADAPTED {
        mcp::adapter_sources(kind, &root().join("docs/evals/sources.lock.json"), None)
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
    }
    // A copied root serves too: the documents need not be in the tree.
    let copy = documents_copy();
    for kind in ADAPTED {
        mcp::adapter_sources(kind, &draft, Some(copy.path()))
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
    }
    // No adapter for a target without one, whatever the lock says.
    let e = mcp::adapter_sources(TargetKind::Smtp4dev, &draft, Some(&docs)).unwrap_err();
    assert!(e.contains("has no MCP adapter"), "{e}");
}

#[test]
fn an_adapter_mode_reads_its_documents_flag_from_its_command_line() {
    // R2, the adapter side of the forwarding: the command line apibench
    // builds (`--sources LOCK --endpoint URL --documents ROOT`) is the one
    // the adapter mode reads, and its --documents is what serves the draft.
    let draft = root().join("docs/evals/draft/sources.lock.v5.json");
    let docs = root().join("docs/evals");
    let argv = |extra: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = [
            "--sources",
            draft.to_str().unwrap(),
            "--endpoint",
            "http://127.0.0.1:4321",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        v.extend(extra.iter().map(|s| s.to_string()));
        v
    };
    for kind in ADAPTED {
        let (f, _) = mcp::adapter_flags(kind, &argv(&["--documents", docs.to_str().unwrap()]))
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert_eq!(f["documents"], vec![docs.display().to_string()]);
    }
    // Dropped, the draft lock's own directory holds no documents: refused.
    let e = mcp::adapter_flags(TargetKind::Mailpit, &argv(&[])).unwrap_err();
    assert!(e.starts_with("source documents:"), "{e}");
    let e = mcp::adapter_flags(TargetKind::Mailpit, &argv(&["--bogus", "x"])).unwrap_err();
    assert_eq!(e, "unknown flag --bogus");
    let e =
        mcp::adapter_flags(TargetKind::Mailpit, &["--endpoint".into(), "x".into()]).unwrap_err();
    assert_eq!(e, "--sources is required");
    let e = mcp::adapter_flags(TargetKind::Mailpit, &["--sources".into()]).unwrap_err();
    assert!(!e.is_empty());
}

#[test]
fn an_adapter_refuses_missing_resized_altered_or_escaping_documents() {
    let draft = root().join("docs/evals/draft/sources.lock.v5.json");
    let rel = mailpit_doc(&draft);
    let refuses = |root: &Path, lock: &Path| {
        let e = mcp::adapter_sources(TargetKind::Mailpit, lock, Some(root)).unwrap_err();
        assert!(
            e.starts_with("source documents:") || e.contains("path"),
            "{e}"
        );
    };
    // Missing.
    let copy = documents_copy();
    std::fs::remove_file(copy.path().join(&rel)).unwrap();
    refuses(copy.path(), &draft);
    // Resized.
    let copy = documents_copy();
    let p = copy.path().join(&rel);
    let mut bytes = std::fs::read(&p).unwrap();
    bytes.push(b'\n');
    std::fs::write(&p, &bytes).unwrap();
    refuses(copy.path(), &draft);
    // Altered at the same size.
    let copy = documents_copy();
    let p = copy.path().join(&rel);
    let mut bytes = std::fs::read(&p).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 0x01;
    std::fs::write(&p, &bytes).unwrap();
    refuses(copy.path(), &draft);
    // Escaping: a lock naming a document outside its root.
    let copy = documents_copy();
    let mut v: Value = serde_json::from_slice(&std::fs::read(&draft).unwrap()).unwrap();
    for e in v["entries"].as_array_mut().unwrap() {
        if e["target"] == "mailpit" {
            e["pin"]["documents"][0]["path"] = json!(format!("../{rel}"));
        }
    }
    let lock = copy.path().join("escaping.lock.json");
    std::fs::write(&lock, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(mcp::adapter_sources(TargetKind::Mailpit, &lock, Some(copy.path())).is_err());
}

// ---- one lifecycle verdict for both surfaces (R4, R12) ------------------------

fn cell_window(profile: &str, outcome: records::WindowOutcome) -> records::WindowRow {
    let snap = |t_ns| resources::Snap {
        t_ns,
        cpu: Default::default(),
        events: Default::default(),
    };
    runner::window_row(
        "run",
        "leaf",
        (TargetKind::Mailpit, profile, 0),
        records::WindowKind::ApiCell,
        (&snap(1), &snap(2)),
        Some(true),
        None,
        outcome,
    )
}

fn exact_cell(name: &str) -> api::CellRecord {
    let p = cell(Operation::List, 1);
    let mut c = api::CellRecord::new(&p, &clients::native_capability("mailpit", Operation::List));
    c.cell = name.into();
    c.measured = api::Ran {
        rows: rows(100, Phase::Measured, Outcome::Exact),
        planned: 100,
        started: 100,
        ..Default::default()
    };
    c
}

#[test]
fn equal_results_need_exact_rows_and_a_clean_resource_window() {
    let c = exact_cell("list.c1");
    let s = api::stats(&c.measured, Some(10));
    let clean = cell_window("api.list.c1", records::WindowOutcome::Measured);
    assert!(api_chunk::at_equal_results(&c, &s, Some(&clean)));
    // A window that closed on an OOM kill or an exit, or none at all, never
    // ranks, however exact the answers were.
    for bad in [
        records::WindowOutcome::OomKilled,
        records::WindowOutcome::TargetExited,
        records::WindowOutcome::ChunkDeadline,
    ] {
        let w = cell_window("api.list.c1", bad);
        assert!(!api_chunk::at_equal_results(&c, &s, Some(&w)), "{bad:?}");
    }
    assert!(!api_chunk::at_equal_results(&c, &s, None));
    let mut bare = clean.clone();
    bare.cpu_end = None;
    assert!(!api_chunk::at_equal_results(&c, &s, Some(&bare)));
    // The report applies it: a failed window turns at_equal_results off.
    let mut rec = record_for(&runner::Chunk::Api {
        surface: Surface::Native,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 0,
    });
    rec.cells = vec![c];
    let row = |o| vec![records::ResourceRow::Window(cell_window("api.list.c1", o))];
    let good = api_chunk::api_report(&rec, &row(records::WindowOutcome::Measured));
    let oom = api_chunk::api_report(&rec, &row(records::WindowOutcome::OomKilled));
    assert_eq!(good["cells"][0]["at_equal_results"], json!(true));
    assert_eq!(oom["cells"][0]["at_equal_results"], json!(false));
}

#[test]
fn wrong_failed_or_timed_out_rows_and_dead_windows_cannot_exit_clean() {
    let mut rec = record_for(&runner::Chunk::Api {
        surface: Surface::Mcp,
        target: TargetKind::Mailpit,
        repetition: 0,
        fill: 0,
    });
    let mut cells = vec![exact_cell("list.c1"), exact_cell("list.c8")];
    assert!(api_chunk::row_failures(&ApiRecordView(&mut rec, cells.clone()).get()).is_empty());
    // Inexact fidelity is unranked evidence, not a failure.
    cells[0].measured.rows[3].outcome = Outcome::InexactFidelity {
        detail: "text".into(),
    };
    assert!(api_chunk::row_failures(&ApiRecordView(&mut rec, cells.clone()).get()).is_empty());
    for bad in [
        Outcome::WrongResult { detail: "x".into() },
        Outcome::Failed {
            reason: "status_500".into(),
        },
        Outcome::Timeout,
    ] {
        let mut c = cells.clone();
        c[1].measured.rows[7].outcome = bad.clone();
        let f = api_chunk::row_failures(&ApiRecordView(&mut rec, c).get());
        assert_eq!(f.len(), 1, "{bad:?}");
        assert!(f[0].contains("list.c8"), "{f:?}");
    }
    // A blocked cell's cause is reported where it is blocked, not here.
    let mut c = cells.clone();
    c[0].status = api::CellStatus::Blocked;
    c[0].measured.rows[0].outcome = Outcome::Timeout;
    assert!(api_chunk::row_failures(&ApiRecordView(&mut rec, c).get()).is_empty());
    // Every readiness, idle and cell window that closed on an OOM or an
    // exit is the target's own failure, on either surface.
    let mut ws: Vec<records::ResourceRow> = Vec::new();
    for (kind, outcome) in [
        (records::WindowKind::Ready, records::WindowOutcome::Measured),
        (
            records::WindowKind::IdleReady,
            records::WindowOutcome::OomKilled,
        ),
        (
            records::WindowKind::IdleFill,
            records::WindowOutcome::TargetExited,
        ),
        (
            records::WindowKind::ApiCell,
            records::WindowOutcome::Blocked,
        ),
        (
            records::WindowKind::ApiCell,
            records::WindowOutcome::ChunkDeadline,
        ),
    ] {
        let mut w = cell_window("api.f0", outcome);
        w.window = kind;
        ws.push(records::ResourceRow::Window(w));
    }
    let f = api_chunk::window_failures(&ws);
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(
        f[0].contains("IdleReady") && f[1].contains("IdleFill"),
        "{f:?}"
    );
    // Death after the last reply: the final look.
    assert!(api_chunk::final_state_failure(false, Some(0)).is_none());
    assert!(api_chunk::final_state_failure(false, None).is_none());
    assert!(api_chunk::final_state_failure(true, Some(0)).is_some());
    assert!(api_chunk::final_state_failure(false, Some(1)).is_some());
}

/// A record with `cells` swapped in.
struct ApiRecordView<'a>(&'a mut api::ApiRecord, Vec<api::CellRecord>);

impl ApiRecordView<'_> {
    fn get(self) -> api::ApiRecord {
        let mut r = self.0.clone();
        r.cells = self.1;
        r
    }
}

#[tokio::test]
async fn the_record_line_is_bounded_as_a_record_whatever_its_member_order() {
    use api_chunk::{Control, MAX_CONTROL_LINE, MAX_RECORD_LINE, parse_control};
    // The record exactly as `swarmail-evals api-mcp` sends it: `json!` with
    // serde_json's sorted map puts `api` before `cmd`, so the line does not
    // start with `{"cmd":"record"` — the host's first MCP chunk refused it
    // as "a control line of 210903 bytes exceeds 1024".
    let chunk = runner::Chunk::Api {
        surface: Surface::Mcp,
        target: TargetKind::Swarmail,
        repetition: 0,
        fill: 1_000,
    };
    let rec = record_for(&chunk);
    let sent = json!({"cmd": "record", "api": rec}).to_string();
    assert!(sent.starts_with("{\"api\":"), "{}", &sent[..40]);
    // Padded with insignificant whitespace to the host's real size.
    const REAL: usize = 210_903;
    let body = sent.strip_suffix('}').unwrap();
    let pad = " ".repeat(REAL - sent.len());
    let line = format!("{body}{pad}}}");
    assert_eq!(line.len(), REAL);
    match parse_control(line.as_bytes()).unwrap() {
        Control::Record { api } => assert_eq!(*api, rec),
        other => panic!("{other:?}"),
    }
    // Through the holder's reader too: the real-sized line reads whole.
    let framed = format!("{line}\n").into_bytes();
    let mut r = tokio::io::BufReader::new(framed.as_slice());
    let end = std::time::Instant::now() + Duration::from_secs(30);
    let read = api_chunk::next_control_line(&mut r, end).await.unwrap();
    assert_eq!(read.as_deref(), Some(line.as_bytes()));
    // A record over its own bound refuses, in either member order.
    for first in [true, false] {
        let pad = " ".repeat(MAX_RECORD_LINE + 1 - sent.len());
        let api = serde_json::to_string(&rec).unwrap();
        let huge = if first {
            format!("{{\"cmd\":\"record\",{pad}\"api\":{api}}}")
        } else {
            format!("{{\"api\":{api},{pad}\"cmd\":\"record\"}}")
        };
        assert!(huge.len() > MAX_RECORD_LINE);
        let e = parse_control(huge.as_bytes()).unwrap_err();
        assert!(e.contains(&format!("exceeds {MAX_RECORD_LINE}")), "{e}");
    }
    // Any other command keeps the short bound, in either member order: the
    // record's allowance is the record's alone.
    for line in [
        format!(
            "{{\"id\":\"list.c1\",{}\"cmd\":\"window_start\"}}",
            " ".repeat(MAX_CONTROL_LINE)
        ),
        format!(
            "{{\"cmd\":\"window_end\",{}\"id\":\"list.c1\"}}",
            " ".repeat(MAX_CONTROL_LINE)
        ),
        format!("{{\"cmd\":\"idle_fill\"{}}}", " ".repeat(MAX_CONTROL_LINE)),
    ] {
        let e = parse_control(line.as_bytes()).unwrap_err();
        assert!(e.contains(&format!("exceeds {MAX_CONTROL_LINE}")), "{e}");
    }
    // At the short bound exactly, a command still parses.
    let at = r#"{"id":"list.c1","cmd":"window_start"}"#;
    let at = format!(
        "{}{}}}",
        at.strip_suffix('}').unwrap(),
        " ".repeat(MAX_CONTROL_LINE - at.len())
    );
    assert_eq!(at.len(), MAX_CONTROL_LINE);
    assert_eq!(
        parse_control(at.as_bytes()).unwrap(),
        Control::WindowStart {
            id: "list.c1".into()
        }
    );
}

#[tokio::test(start_paused = true)]
async fn the_holder_ends_its_control_input_at_the_chunk_deadline() {
    // A caller that holds the control input open and silent cannot keep the
    // target past the chunk's lifetime (R12).
    let (_keep, read) = tokio::io::duplex(64);
    let mut r = tokio::io::BufReader::new(read);
    let end = std::time::Instant::now() + Duration::from_secs(30);
    let e = api_chunk::next_control_line(&mut r, end).await.unwrap_err();
    assert!(e.contains("chunk deadline"), "{e}");
    // Lines before the deadline, and EOF, read as ever.
    let (mut w, read) = tokio::io::duplex(64);
    tokio::io::AsyncWriteExt::write_all(&mut w, b"{\"cmd\":\"idle_fill\"}\n")
        .await
        .unwrap();
    drop(w);
    let mut r = tokio::io::BufReader::new(read);
    let end = std::time::Instant::now() + Duration::from_secs(30);
    assert_eq!(
        api_chunk::next_control_line(&mut r, end).await.unwrap(),
        Some(b"{\"cmd\":\"idle_fill\"}".to_vec())
    );
    assert_eq!(
        api_chunk::next_control_line(&mut r, end).await.unwrap(),
        None
    );
}

#[test]
fn the_readme_lists_every_client_modules_digest() {
    let readme = std::fs::read_to_string(root().join("docs/evals/README.md")).unwrap();
    for f in Family::ALL {
        let bytes =
            std::fs::read(root().join("benches/mail_compare/clients").join(f.module())).unwrap();
        let line = format!("| `{}` | `{}` |", f.module(), protocol::sha256_hex(&bytes));
        assert!(
            readme.contains(&line),
            "docs/evals/README.md must list {line}"
        );
    }
}

#[test]
fn the_run_report_publishes_every_coordinate_that_launches_nothing() {
    let p = Protocol::parse(DRAFT_V4).unwrap();
    let plan = runner::plan_v4(&p);
    let v = report::api_run_report(&p, &plan, &[], &json!({"report_version": 3}));
    let rows = v["not_launched"].as_array().unwrap();
    // Native: the official MailSlurper row; MCP: it and the four targets
    // with no MCP surface. Three repetitions, four fills, no receipts.
    assert_eq!(rows.len(), (1 + 5) * 3 * 4);
    let status = |surface: &str, target: &str| {
        rows.iter()
            .filter(|r| r["surface"] == surface && r["target"] == target)
            .map(|r| r["status"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(status("native", "mailslurper"), ["does_not_start"].into());
    assert_eq!(status("mcp", "inbucket"), ["no_mcp_surface"].into());
    assert!(status("native", "inbucket").is_empty());
    assert!(
        rows.iter()
            .all(|r| r.get("launch").is_none() && r.get("cleanup").is_none())
    );
    // R15: every official MailSlurper coordinate, on both surfaces, cites
    // the captured failure its pin records; every row has null figures; no
    // other row claims a failure document.
    let cite = "sources/mailslurper/mailslurper-official-failure.txt";
    let official: Vec<&Value> = rows
        .iter()
        .filter(|r| r["target"] == "mailslurper")
        .collect();
    assert_eq!(official.len(), 2 * 3 * 4);
    assert!(
        official.iter().all(|r| r["cite"] == json!(cite)),
        "{official:?}"
    );
    assert!(rows.iter().all(|r| r["stats"].is_null()));
    assert!(
        rows.iter()
            .filter(|r| r["target"] != "mailslurper")
            .all(|r| r.get("cite").is_none())
    );
    // The cited document is a document of that pin.
    let lock = lock5();
    let pin = lock.pin(TargetKind::Mailslurper).unwrap();
    assert!(pin.documents.iter().any(|d| d.path == cite));
}

// ---- test execution host proofs (story D, 0132 §8) --------------------------
//
// One real `testexec` chunk each, the reference suite of 100 scored fixture
// trials, run by the eval crate (`SWARMAIL_EVALS_BIN`) against a target held
// by the driver (`MAIL_COMPARE_BIN`) under its enforced leaf. As for the MCP
// proofs, the test binary and the eval client run outside any scope and the
// driver's `hold` alone enters a delegated scope of its own (`scoped_driver`).
// Run each alone, the load read before and after:
//
//   timeout 600 taskset -c 0-31 "$bin" --ignored --exact \
//     api_section::testexec_real_swarmail --test-threads=1 --nocapture
//
// A pass is exact: exit 0, all 100 planned trials started and scored
// `pass` with their cleanup proved, the chunk sealed and admitted (its
// executable rechecked by digest, its report recomputed from testexec.json
// and the suite window), the suite window measured with its counters, and
// every launch's cleanup receipt proving its leaf removed.

/// The prerequisites a suite host proof names, all at once, when absent.
fn need(vars: &[&str]) {
    let missing: Vec<&str> = vars
        .iter()
        .copied()
        .filter(|v| std::env::var_os(v).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "host proof prerequisites missing: {} must name their built or pinned bytes",
        missing.join(", ")
    );
}

fn suite_vars(target: TargetKind) -> Vec<String> {
    let mut v = vec![
        "MAIL_COMPARE_BIN".to_owned(),
        "SWARMAIL_EVALS_BIN".to_owned(),
    ];
    if target != TargetKind::Swarmail {
        v.push(format!(
            "MAIL_COMPARE_{}",
            target.name().to_uppercase().replace('-', "_")
        ));
    }
    v
}

fn real_suite(target: TargetKind, parallelism: u32) {
    let vars = suite_vars(target);
    need(&vars.iter().map(String::as_str).collect::<Vec<_>>());
    let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let driver = scoped_driver(root.path(), &env_path("MAIL_COMPARE_BIN"));
    let chunk = format!("testexec.{target}.p{parallelism}.r0");
    let pinned = pinned_args(target);
    let out = std::process::Command::new(env_path("SWARMAIL_EVALS_BIN"))
        .arg("testexec")
        .arg("--driver")
        .arg(&driver)
        .args(["--mode", "smoke"])
        .args(["--protocol", "docs/evals/draft/protocol.v4.json"])
        .args([
            "--sources",
            "docs/evals/draft/sources.lock.v5.json",
            "--documents",
            "docs/evals",
        ])
        .args(["--chunk", &chunk, "--run-id", "testexec-real"])
        .args(&pinned)
        .arg("--output")
        .arg(root.path().join("out"))
        .arg("--scratch")
        .arg(root.path().join("scratch"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let dir = root.path().join("out");
    let p = Protocol::parse(DRAFT_V4).unwrap();
    let lock = SourceLock::parse(LOCK_V5).unwrap();
    let c = runner::Chunk::parse(&chunk, &p).unwrap();
    // The launched executable is rechecked by digest from its file: the
    // candidate Cargo built, or the lock-pinned competitor.
    let bin = pinned[1].split_once('=').unwrap().1.to_owned();
    let bins = std::collections::BTreeMap::from([(target, PathBuf::from(bin))]);
    let a = report::Admission {
        protocol: &p,
        protocol_bytes: DRAFT_V4,
        lock: &lock,
        lock_bytes: LOCK_V5,
        bins: &bins,
    };
    let admitted = report::admit_chunk(&dir, &c, &a)
        .unwrap_or_else(|e| panic!("{chunk} must admit: {e}\n{text}"));
    let rec: suite::SuiteRecord =
        serde_json::from_slice(&std::fs::read(dir.join("testexec.json")).unwrap()).unwrap();
    suite_chunk::check_against(&rec, &c, &p).unwrap();
    assert_eq!(out.status.code(), Some(0), "{chunk}: {text}");
    assert_eq!(rec.rows.len(), 100, "{chunk}: {:?}", rec.unstarted);
    for r in &rec.rows {
        assert_eq!(r.outcome, suite::Outcome::Pass, "{chunk}: {r:?}");
    }
    assert!(
        suite::failures(&rec).is_empty(),
        "{:?}",
        suite::failures(&rec)
    );
    suite::cleanup_proved(&rec).unwrap();
    let window = admitted.resources.iter().find_map(|r| match r {
        records::ResourceRow::Window(w) if w.window == records::WindowKind::Suite => Some(w),
        _ => None,
    });
    let window = window.unwrap_or_else(|| panic!("{chunk}: no suite window"));
    assert_eq!(window.outcome, records::WindowOutcome::Measured, "{chunk}");
    assert!(window.counters().is_some(), "{chunk}");
    let cleanups = admitted.report["cleanups"].as_array().expect("cleanups");
    assert!(!cleanups.is_empty());
    assert!(
        cleanups.iter().all(|x| x["cgroup_removed"] == json!(true)),
        "{chunk}"
    );
    let r = &admitted.report[suite::SECTION];
    eprintln!(
        "host proof passed: {chunk}: {} of {} pass, foreign_seen {}, suite wall {} ns, target cpu {} usec, peak memory {} bytes, client cpu {} usec",
        r["by_class"]["pass"],
        r["planned"],
        r["foreign_seen"],
        r["suite_wall_ns"],
        r["resources"]["target"]["usage_usec"],
        r["resources"]["peak_memory_current"],
        r["client_cpu_usec"]
    );
}

// One reason per platform: a second `ignore` would be an unused attribute.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[cfg_attr(
    target_os = "linux",
    ignore = "host proof: MAIL_COMPARE_BIN, SWARMAIL_EVALS_BIN, a delegated scope; run alone under timeout 600 taskset -c 0-31"
)]
fn testexec_real_swarmail() {
    real_suite(TargetKind::Swarmail, 1)
}

// One reason per platform: a second `ignore` would be an unused attribute.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[cfg_attr(
    target_os = "linux",
    ignore = "host proof: MAIL_COMPARE_BIN, SWARMAIL_EVALS_BIN, MAIL_COMPARE_MAILPIT, a delegated scope; run alone under timeout 600 taskset -c 0-31"
)]
fn testexec_real_mailpit() {
    real_suite(TargetKind::Mailpit, 8)
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn the_testexec_host_proofs_are_discovered_and_refuse_without_their_bytes() {
    let me = std::env::current_exe().unwrap();
    let listed = std::process::Command::new(&me)
        .args(["--list", "--ignored"])
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);
    let proofs = [
        ("api_section::testexec_real_swarmail", TargetKind::Swarmail),
        ("api_section::testexec_real_mailpit", TargetKind::Mailpit),
    ];
    // The dispatcher's documented commands name exactly these proofs, run
    // alone through the bounded wrapper.
    let readme =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/evals/README.md"))
            .unwrap();
    assert!(readme.contains(
        "timeout 600 taskset -c 0-31 \"$bin\" --ignored --exact \"$1\" --test-threads=1 --nocapture"
    ));
    for (name, target) in proofs {
        assert!(
            readme.lines().any(|l| l == format!("run_one {name}")),
            "docs/evals/README.md must run {name} alone"
        );
        assert!(
            listed.lines().any(|l| l == format!("{name}: test")),
            "{name} is not discovered:\n{listed}"
        );
        // Run alone with every prerequisite removed: a named failure, never
        // a skip and never a launch.
        let vars = suite_vars(target);
        let mut cmd = std::process::Command::new(&me);
        cmd.args(["--ignored", "--exact", name, "--test-threads=1"]);
        for v in &vars {
            cmd.env_remove(v);
        }
        let out = cmd.output().unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_ne!(out.status.code(), Some(0), "{name}: {text}");
        assert!(text.contains("1 failed"), "{name}: {text}");
        let named = format!("host proof prerequisites missing: {}", vars.join(", "));
        assert!(text.contains(&named), "{name}: {text}");
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn the_mcp_host_proofs_are_discovered_and_refuse_without_their_bytes() {
    // Issue #385: each MCP host proof judges its client scope, and runs on
    // the dispatcher alone. Locally: discovered, and with any one of its
    // prerequisites unset, one named failure, never a skip or a launch.
    let me = std::env::current_exe().unwrap();
    let listed = std::process::Command::new(&me)
        .args(["--list", "--ignored"])
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);
    let readme =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/evals/README.md"))
            .unwrap();
    let proofs = [
        ("api_section::api_mcp_real_swarmail", TargetKind::Swarmail),
        (
            "api_section::api_mcp_bridge_real_mailpit",
            TargetKind::Mailpit,
        ),
        (
            "api_section::api_mcp_bridge_real_mailhog",
            TargetKind::Mailhog,
        ),
        (
            "api_section::api_mcp_bridge_real_mailslurper_source_build",
            TargetKind::MailslurperSourceBuild,
        ),
    ];
    for (name, target) in proofs {
        assert!(
            listed.lines().any(|l| l == format!("{name}: test")),
            "{name} is not discovered:\n{listed}"
        );
        assert!(
            readme.lines().any(|l| l == format!("run_one_mcp {name}")),
            "docs/evals/README.md must run {name} alone"
        );
        let vars = mcp_vars(target);
        for unset in &vars {
            let mut cmd = std::process::Command::new(&me);
            cmd.args(["--ignored", "--exact", name, "--test-threads=1"]);
            for v in &vars {
                cmd.env(v, "/nonexistent/pinned-bytes");
            }
            cmd.env_remove(unset);
            let out = cmd.output().unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert_ne!(out.status.code(), Some(0), "{name} without {unset}: {text}");
            assert!(text.contains("1 failed"), "{name} without {unset}: {text}");
            let named = format!("host proof prerequisites missing: {unset} must name");
            assert!(text.contains(&named), "{name} without {unset}: {text}");
        }
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn only_the_holder_enters_the_delegated_scope() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let fake = root.path().join("bin");
    std::fs::create_dir(&fake).unwrap();
    let log = root.path().join("log");
    std::fs::create_dir(&log).unwrap();
    // A systemd-run that records its argv and execs the command after `--`,
    // as the real one does with --scope; a driver that records its own.
    let script = |path: &Path, body: &str| {
        std::fs::write(path, body).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    };
    script(
        &fake.join("systemd-run"),
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$LOG/scope\"\nwhile [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\"\n",
    );
    let driver = root.path().join("driver");
    script(
        &driver,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$LOG/driver-$1\"\n",
    );
    let scoped = scoped_driver(root.path(), driver.to_str().unwrap());
    let path = format!(
        "{}:{}",
        fake.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |args: &[&str]| {
        let st = std::process::Command::new(&scoped)
            .args(args)
            .env("PATH", &path)
            .env("LOG", &log)
            .status()
            .unwrap();
        assert!(st.success());
    };
    let lines = |name: &str| -> Vec<String> {
        std::fs::read_to_string(log.join(name))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    };
    let hold = [
        "hold",
        "--control",
        "testexec-v1",
        "--chunk",
        "testexec.mailpit.p8.r0",
    ];
    run(&hold);
    let mut want: Vec<String> = ["--user", "--scope", "--quiet", "-p", "Delegate=yes", "--"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    want.push(driver.display().to_string());
    want.extend(hold.iter().map(|s| (*s).to_owned()));
    assert_eq!(lines("scope"), want);
    assert_eq!(lines("driver-hold"), hold);
    // An adapter (any other mode) runs in place: no scope.
    std::fs::remove_file(log.join("scope")).unwrap();
    run(&["mailpit-mcp", "--sources", "x"]);
    assert!(!log.join("scope").exists());
    assert_eq!(
        lines("driver-mailpit-mcp"),
        ["mailpit-mcp", "--sources", "x"]
    );
}
