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
    delaying_responder(answer, |_| None).await
}

/// [`responder`], holding the `n`th request's answer (counted from 1 across
/// all connections) for `delay(n)` when it names one.
async fn delaying_responder(
    answer: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
    delay: fn(u64) -> Option<Duration>,
) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let answer = Arc::new(answer);
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let (answer, count) = (answer.clone(), count.clone());
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
                        if let Some(d) = delay(n) {
                            tokio::time::sleep(d).await;
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
    addr
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
struct Scripted {
    script: fn(Operation, Phase, u32) -> Option<Outcome>,
    phase_of: fn(u32) -> Phase,
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>
    {
        Box::pin(async { Ok(Vec::new()) })
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
    let s = Scripted {
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
    api::run_cells(cells, &caps, &s, &api::NoMeter, clock(), deadline)
        .await
        .unwrap()
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_worker_that_does_not_return_leaves_its_start_and_an_unproved_cancellation() {
    // The operation blocks its thread past the deadline and the bound: it
    // cannot be dropped in time. Its start is still a terminal timeout row,
    // and the phase says local cancellation was not proved.
    let op = Arc::new(|_w: u32, i: u32, _d: tokio::time::Instant| {
        Box::pin(async move {
            if i == 0 {
                std::thread::sleep(Duration::from_millis(600));
            }
            api::Done::of(Outcome::Exact, Spend::default())
        }) as api::OpFuture
    });
    let r = api::run_cell_bounded(
        clock(),
        Phase::Measured,
        1,
        5,
        Duration::from_millis(50),
        Duration::from_millis(100),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_native_surface_keeps_measuring_after_a_read_times_out() {
    // The producer path (R1): the real native surface over Mailpit's client,
    // against a loopback server holding one measured list request past the
    // cell's five-second deadline. That list is a timeout row; the next
    // cell measures all of its operations; nothing is abandoned.
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
    // Request 1..20 are list.c1's warmup; 31 is its 11th measured call.
    let addr = delaying_responder(
        move |_| (200, body.clone()),
        |n| (n == 31).then_some(Duration::from_secs(8)),
    )
    .await;
    let surface = api::NativeSurface {
        clients: Arc::new(
            (0..8)
                .map(|_| clients::Client::new("mailpit", addr).unwrap())
                .collect(),
        ),
        fixtures: Arc::new(reads),
    };
    let caps = |op| clients::native_capability("mailpit", op);
    let cells = [cell(Operation::List, 1), cell(Operation::List, 8)];
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
    let second = &recs[1];
    assert_eq!(second.status, api::CellStatus::Measured);
    assert_eq!(second.measured.started, 100);
    assert!(
        second
            .measured
            .rows
            .iter()
            .all(|r| r.outcome == Outcome::Exact)
    );
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
    }
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
    api::prepare(owned.smtp, &client, &mut fixtures)
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
    let w = api::prepare(dead, &client, &mut fixtures)
        .await
        .unwrap_err();
    assert!(w.contains("smtp connect"), "{w}");
}

/// The whole native API chunk under the enforced leaf: the driver's own `run`
/// with the chunk coordinate, then `admit`-level checks of its directory.
/// Needs a delegated cgroup v2 scope (run it as `systemd-run --user --scope -p
/// Delegate=yes`), and fails, never skips, without one.
#[test]
#[ignore = "host proof: a delegated cgroup v2 scope; run alone under timeout 600 taskset -c 0-31"]
fn api_native_real() {
    let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let out = std::process::Command::new(env!("CARGO"))
        .args([
            "bench",
            "--locked",
            "--bench",
            "mail_compare",
            "--",
            "run",
            "--mode",
            "smoke",
        ])
        .args(["--protocol", "docs/evals/draft/protocol.v4.json"])
        .args([
            "--sources",
            "docs/evals/draft/sources.lock.v5.json",
            "--documents",
            "docs/evals",
        ])
        .arg("--output")
        .arg(root.path().join("out"))
        .arg("--scratch")
        .arg(root.path().join("scratch"))
        .args([
            "--run-id",
            "api-native-real",
            "--chunk",
            "api.native.swarmail.r0.f1000",
        ])
        .args([
            "--bin",
            &format!("swarmail={}", env!("CARGO_BIN_EXE_swarmail")),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.code() == Some(0),
        "the chunk must seal clean: {text}"
    );
    let admitted = records::admit(&root.path().join("out")).expect("the chunk admits");
    let rec: api::ApiRecord =
        serde_json::from_slice(&std::fs::read(root.path().join("out/api.json")).unwrap()).unwrap();
    assert!(rec.fill.reached);
    assert!(
        rec.cells
            .iter()
            .all(|c| c.status == api::CellStatus::Measured)
    );
    assert!(admitted.resources.iter().any(|r| matches!(r, records::ResourceRow::Window(w) if w.window == records::WindowKind::ApiCell && w.counters().is_some())));
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
    let fill = api::fill(owned.smtp, &client, (163, "f100k"), 100_000, clock).await;
    assert!(fill.reached, "{fill:?}");
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
    let ids: BTreeSet<&String> = all.iter().map(|s| &s.id).collect();
    assert_eq!((all.len(), ids.len()), (100_000, 100_000));
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

/// The held MCP chunk end to end: `swarmail-evals api-mcp` drives `hold
/// --control api-v1` for the candidate and a bridged adapter target, and
/// both chunks seal and admit. Needs a delegated cgroup v2 scope and the
/// eval binary (`SWARMAIL_EVALS_BIN`); fails, never skips, without them.
#[test]
#[ignore = "host proof: a delegated cgroup v2 scope and the eval binary; run alone under timeout 600 taskset -c 0-31"]
fn api_mcp_bridge_real() {
    let evals = std::env::var("SWARMAIL_EVALS_BIN")
        .expect("SWARMAIL_EVALS_BIN names the built eval binary");
    let driver =
        std::env::var("MAIL_COMPARE_BIN").expect("MAIL_COMPARE_BIN names the built driver");
    let root = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let out = std::process::Command::new(evals)
        .args(["api-mcp", "--driver", &driver, "--mode", "smoke"])
        .args(["--protocol", "docs/evals/draft/protocol.v4.json"])
        .args([
            "--sources",
            "docs/evals/draft/sources.lock.v5.json",
            "--documents",
            "docs/evals",
        ])
        .args([
            "--chunk",
            "api.mcp.swarmail.r0.f1000",
            "--run-id",
            "api-mcp-real",
        ])
        .args([
            "--bin",
            &format!("swarmail={}", env!("CARGO_BIN_EXE_swarmail")),
        ])
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
    assert!(
        matches!(out.status.code(), Some(0 | 1)),
        "the chunk must seal: {text}"
    );
    let admitted = records::admit(&root.path().join("out")).expect("the chunk admits");
    assert!(admitted.resources.iter().any(|r| matches!(r, records::ResourceRow::Window(w) if w.window == records::WindowKind::ApiCell && w.counters().is_some())));
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
}
