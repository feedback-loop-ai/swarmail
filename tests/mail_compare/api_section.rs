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
}

// ---- the scan's completeness rules (T2) -----------------------------------

/// A bounded loopback HTTP/1.1 responder: one canned answer per path, kept
/// alive, so the scan's rules meet real framing. Not a target: it proves the
/// client's refusals, which no real target can be made to provoke.
async fn responder(
    answer: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let answer = Arc::new(answer);
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let answer = answer.clone();
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
    assert!(r.rows.iter().all(|x| x.outcome == Outcome::Timeout) && r.unresolved);
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
    let fill = api::fill(owned.smtp, &client, (163, "real"), 1_000, clock).await;
    assert!(fill.reached, "{fill:?}");
    assert_eq!((fill.accepted, fill.retained), (1_000, Some(1_000)));
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
    assert!(
        api_chunk::final_ledger(&native, &mut recs, 1_000)
            .await
            .is_empty()
    );
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
