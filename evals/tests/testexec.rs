//! Protocol 4's test execution (accepted decision 0132 §8, story D): the
//! reference suite against the real candidate (`SWARMAIL_BIN`), and against
//! scripted loopback stores that speak real SMTP and the documented HTTP
//! shapes of Mailpit (one shared store, a noisy subject search, delete by
//! id), Inbucket (mailbox-local ids) and GreenMail (only a global purge).
//! Every flow is the existing fixture flow solved by the existing oracle and
//! judged by the existing scorer; nothing here is mocked between them. The
//! held, enforced chunks are the host proofs
//! `api_section::testexec_real_swarmail` and
//! `api_section::testexec_real_mailpit` in the driver's suite.

mod common;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use swarmail_evals::api::Clock;
use swarmail_evals::clients;
use swarmail_evals::mailsource::MailSource;
use swarmail_evals::native_source::{Failure, Mutations, NativeSource};
use swarmail_evals::suite::{self, Cleanup, Coord, Outcome, Policy, WaitMethod};
use swarmail_evals::testexec::{self, LocalMeter, SuiteConfig, WorkerKit};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::Instant;

// ---- a scripted store: real SMTP in, a documented HTTP shape out ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Mailpit,
    Inbucket,
    Greenmail,
}

#[derive(Clone, Debug)]
struct Msg {
    id: String,
    mailbox: String,
    to: String,
    subject: String,
    raw: Vec<u8>,
}

#[derive(Default)]
struct Store {
    msgs: Vec<Msg>,
    next: BTreeMap<String, u64>,
    /// Delete-by-id requests answered 500.
    fail_delete: BTreeSet<String>,
    /// Every mutation request, in order: an id, or `all`.
    mutations: Vec<String>,
    /// Listing and search requests served.
    scans: u32,
    /// Answer listings with a count one above the items (a partial page).
    inflate: bool,
    /// Answer every read with this recipient instead of the stored one.
    lie_to: Option<String>,
    /// Answer every subject search with no hits (the mail is stored and
    /// listed, but the filtered wait never finds it).
    blind_search: bool,
    /// Messages to store once `scans` reaches the key (a delayed delivery,
    /// ordered after the scans that preceded it).
    later: BTreeMap<u32, Vec<Msg>>,
    /// Store-wide listings' first pages (`GET /api/v1/messages`, start 0)
    /// served so far.
    first_pages: u32,
    /// The first pages (by ordinal, from 1) right after which another
    /// worker's delivery lands: the listing's stated total moves before its
    /// next page, as it does when neighbours deliver beside a cleanup.
    move_on: BTreeSet<u32>,
}

type Shared = Arc<Mutex<Store>>;

struct Scripted {
    store: Shared,
    smtp: SocketAddr,
    http: SocketAddr,
    /// While closed, every DATA's final reply waits for a permit.
    gate: Arc<tokio::sync::Semaphore>,
    gated: Arc<std::sync::atomic::AtomicBool>,
}

fn local(a: &str) -> String {
    a.split('@').next().unwrap_or_default().to_ascii_lowercase()
}

impl Store {
    fn add(&mut self, kind: Kind, to: &str, raw: Vec<u8>) -> String {
        let subject = clients::mime::parse(&raw)
            .and_then(|p| p.subject)
            .unwrap_or_default();
        let mailbox = match kind {
            Kind::Mailpit => String::new(),
            Kind::Inbucket => local(to),
            Kind::Greenmail => to.to_ascii_lowercase(),
        };
        let n = self.next.entry(mailbox.clone()).or_insert(0);
        *n += 1;
        let id = match kind {
            Kind::Mailpit => format!("mp-{n}"),
            _ => n.to_string(),
        };
        self.msgs.push(Msg {
            id: id.clone(),
            mailbox,
            to: to.to_owned(),
            subject,
            raw,
        });
        id
    }

    fn scanned(&mut self) {
        self.scans += 1;
        if let Some(more) = self.later.remove(&self.scans) {
            self.msgs.extend(more);
        }
    }
}

/// A raw message to `to` with `subject` and a text body.
fn raw(to: &str, subject: &str, text: &str) -> Vec<u8> {
    format!(
        "From: <app@fixture.eval>\r\nTo: <{to}>\r\nSubject: {subject}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text}\r\n"
    )
    .into_bytes()
}

async fn smtp_session(
    sock: tokio::net::TcpStream,
    kind: Kind,
    store: Shared,
    gate: Arc<tokio::sync::Semaphore>,
    gated: Arc<std::sync::atomic::AtomicBool>,
) {
    let (r, mut w) = sock.into_split();
    let mut r = BufReader::new(r);
    let _ = w.write_all(b"220 scripted\r\n").await;
    let mut rcpt = String::new();
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).await.unwrap_or(0) == 0 {
            return;
        }
        let up = line.to_ascii_uppercase();
        let reply: &[u8] = if up.starts_with("EHLO") {
            b"250 scripted\r\n"
        } else if up.starts_with("AUTH") {
            b"235 ok\r\n"
        } else if up.starts_with("MAIL") {
            b"250 ok\r\n"
        } else if up.starts_with("RCPT") {
            rcpt = line
                .split_once('<')
                .and_then(|(_, a)| a.split_once('>'))
                .map(|(a, _)| a.to_owned())
                .unwrap_or_default();
            b"250 ok\r\n"
        } else if up.starts_with("DATA") {
            let _ = w.write_all(b"354 go\r\n").await;
            let mut data = Vec::new();
            loop {
                let mut l = Vec::new();
                if r.read_until(b'\n', &mut l).await.unwrap_or(0) == 0 {
                    return;
                }
                if l == b".\r\n" {
                    break;
                }
                let l = if l.starts_with(b"..") {
                    l[1..].to_vec()
                } else {
                    l
                };
                data.extend_from_slice(&l);
            }
            store.lock().unwrap().add(kind, &rcpt, data);
            if gated.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = gate.acquire().await.map(|p| p.forget());
            }
            b"250 stored\r\n"
        } else if up.starts_with("QUIT") {
            let _ = w.write_all(b"221 bye\r\n").await;
            return;
        } else {
            b"500 what\r\n"
        };
        if w.write_all(reply).await.is_err() {
            return;
        }
    }
}

#[derive(serde::Deserialize)]
struct PageQ {
    start: Option<u64>,
    limit: Option<u64>,
    query: Option<String>,
}

fn mailpit_page(s: &mut Store, q: &PageQ, filter: Option<&str>) -> Value {
    s.scanned();
    let hits: Vec<&Msg> = s
        .msgs
        .iter()
        .filter(|m| filter.is_none_or(|f| m.subject.contains(f)))
        .collect();
    let start = q.start.unwrap_or(0) as usize;
    let limit = q.limit.unwrap_or(50) as usize;
    let page: Vec<Value> = hits
        .iter()
        .skip(start)
        .take(limit)
        .map(|m| json!({"ID": m.id, "To": [{"Address": m.to}], "Subject": m.subject}))
        .collect();
    json!({
        "start": start,
        "messages_count": hits.len() + usize::from(s.inflate),
        "messages": page,
    })
}

fn mailpit_routes(store: Shared) -> Router {
    Router::new()
        .route(
            "/api/v1/messages",
            get(
                |State(s): State<Shared>, Query(q): Query<PageQ>| async move {
                    let mut s = s.lock().unwrap();
                    let page = mailpit_page(&mut s, &q, None);
                    if q.start.unwrap_or(0) == 0 {
                        s.first_pages += 1;
                        let n = s.first_pages;
                        if s.move_on.remove(&n) {
                            let to = suite::primary(190);
                            s.add(Kind::Mailpit, &to, raw(&to, "Neighbour", "delivered"));
                        }
                    }
                    axum::Json(page)
                },
            )
            .delete(|State(s): State<Shared>, body: String| async move {
                let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                let mut s = s.lock().unwrap();
                match v["IDs"].as_array() {
                    Some(ids) => {
                        for id in ids.iter().filter_map(Value::as_str) {
                            s.mutations.push(id.to_owned());
                            if s.fail_delete.contains(id) {
                                return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                            }
                            s.msgs.retain(|m| m.id != id);
                        }
                    }
                    None => {
                        s.mutations.push("all".into());
                        s.msgs.clear();
                    }
                }
                axum::http::StatusCode::OK
            }),
        )
        .route(
            "/api/v1/search",
            get(
                |State(s): State<Shared>, Query(q): Query<PageQ>| async move {
                    let mut s = s.lock().unwrap();
                    let f = match s.blind_search {
                        true => "\u{0}never".to_owned(),
                        false => q.query.clone().unwrap_or_default(),
                    };
                    axum::Json(mailpit_page(&mut s, &q, Some(&f)))
                },
            ),
        )
        .route(
            "/api/v1/message/{id}",
            get(
                |State(s): State<Shared>, Path(id): Path<String>| async move {
                    let s = s.lock().unwrap();
                    let Some(m) = s.msgs.iter().find(|m| m.id == id) else {
                        return (axum::http::StatusCode::NOT_FOUND, axum::Json(Value::Null));
                    };
                    let p = clients::mime::parse(&m.raw).unwrap();
                    let to = s.lie_to.clone().unwrap_or_else(|| m.to.clone());
                    (
                        axum::http::StatusCode::OK,
                        axum::Json(json!({
                            "ID": m.id,
                            "Subject": p.subject.unwrap_or_default(),
                            "From": {"Address": p.from.unwrap_or_default()},
                            "To": [{"Address": to}],
                            "Text": p.text.unwrap_or_default(),
                            "HTML": p.html.unwrap_or_default(),
                        })),
                    )
                },
            ),
        )
        .with_state(store)
}

fn inbucket_routes(store: Shared) -> Router {
    Router::new()
        .route(
            "/api/v1/mailbox/{name}",
            get(
                |State(s): State<Shared>, Path(name): Path<String>| async move {
                    let mut s = s.lock().unwrap();
                    s.scanned();
                    let items: Vec<Value> = s
                        .msgs
                        .iter()
                        .filter(|m| m.mailbox == name)
                        .map(|m| json!({"id": m.id, "to": [m.to], "subject": m.subject}))
                        .collect();
                    axum::Json(Value::Array(items))
                },
            ),
        )
        .route(
            "/api/v1/mailbox/{name}/{id}/source",
            get(
                |State(s): State<Shared>, Path((name, id)): Path<(String, String)>| async move {
                    let s = s.lock().unwrap();
                    s.msgs
                        .iter()
                        .find(|m| m.mailbox == name && m.id == id)
                        .map(|m| m.raw.clone())
                        .ok_or(axum::http::StatusCode::NOT_FOUND)
                },
            ),
        )
        .route(
            "/api/v1/mailbox/{name}/{id}",
            axum::routing::delete(
                |State(s): State<Shared>, Path((name, id)): Path<(String, String)>| async move {
                    let mut s = s.lock().unwrap();
                    s.mutations.push(format!("{name}/{id}"));
                    s.msgs.retain(|m| !(m.mailbox == name && m.id == id));
                    axum::http::StatusCode::OK
                },
            ),
        )
        .with_state(store)
}

fn greenmail_routes(store: Shared) -> Router {
    Router::new()
        .route(
            "/api/user/{email}/messages/INBOX",
            get(
                |State(s): State<Shared>, Path(email): Path<String>| async move {
                    let mut s = s.lock().unwrap();
                    s.scanned();
                    let email = email.to_ascii_lowercase();
                    let items: Vec<Value> = s
                    .msgs
                    .iter()
                    .filter(|m| m.mailbox == email)
                    .map(|m| json!({"uid": m.id, "mimeMessage": String::from_utf8_lossy(&m.raw)}))
                    .collect();
                    axum::Json(Value::Array(items))
                },
            ),
        )
        .route(
            "/api/mail/purge",
            post(|State(s): State<Shared>| async move {
                let mut s = s.lock().unwrap();
                s.mutations.push("all".into());
                s.msgs.clear();
                axum::http::StatusCode::OK
            }),
        )
        .with_state(store)
}

async fn scripted(kind: Kind) -> Scripted {
    let store: Shared = Arc::default();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let gated = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let smtp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let smtp_addr = smtp.local_addr().unwrap();
    let (s2, g2, f2) = (store.clone(), gate.clone(), gated.clone());
    tokio::spawn(async move {
        while let Ok((sock, _)) = smtp.accept().await {
            tokio::spawn(smtp_session(sock, kind, s2.clone(), g2.clone(), f2.clone()));
        }
    });
    let router = match kind {
        Kind::Mailpit => mailpit_routes(store.clone()),
        Kind::Inbucket => inbucket_routes(store.clone()),
        Kind::Greenmail => greenmail_routes(store.clone()),
    };
    let http = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_addr = http.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(http, router).await;
    });
    Scripted {
        store,
        smtp: smtp_addr,
        http: http_addr,
        gate,
        gated,
    }
}

// ---- the real candidate ----------------------------------------------------

struct Candidate {
    _child: tokio::process::Child,
    smtp: SocketAddr,
    http: SocketAddr,
    _dir: tempfile::TempDir,
}

async fn candidate() -> Candidate {
    let dir = common::tempdir();
    let mut child = tokio::process::Command::new(common::swarmail_bin())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:0",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:0",
        ])
        .current_dir(dir.path())
        .env_clear()
        .env("HOME", dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("swarmail serve starts");
    let mut lines = BufReader::new(child.stdout.take().unwrap());
    let (smtp, http) = loop {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(30), lines.read_line(&mut line))
            .await
            .expect("the startup line in time")
            .unwrap();
        assert!(n > 0, "swarmail exited before announcing");
        if let Some(addrs) = swarmail_evals::runner::parse_swarmail_addrs(&line) {
            break addrs;
        }
    };
    tokio::spawn(async move {
        let mut sink = String::new();
        while lines.read_line(&mut sink).await.is_ok_and(|n| n > 0) {
            sink.clear();
        }
    });
    Candidate {
        _child: child,
        smtp,
        http,
        _dir: dir,
    }
}

// ---- the suite, as a chunk runs it ----------------------------------------

/// Two trials of each flow, numbered from the suite's first id.
fn ten() -> Vec<suite::PlannedTrial> {
    suite::plan_ids(163, suite::FIRST_ID..suite::FIRST_ID + 10)
}

struct Bounds {
    test: Duration,
    suite: Duration,
    work_end: Instant,
}

fn generous() -> Bounds {
    Bounds {
        test: Duration::from_secs(30),
        suite: Duration::from_secs(120),
        work_end: Instant::now() + Duration::from_secs(150),
    }
}

async fn run(
    target: &str,
    (smtp, http): (SocketAddr, SocketAddr),
    parallelism: u32,
    plan: Vec<suite::PlannedTrial>,
    (method, b): (WaitMethod, Bounds),
) -> suite::SuiteRecord {
    let clock = Clock(std::time::Instant::now());
    let cfg = SuiteConfig {
        coord: Coord {
            target: target.into(),
            parallelism,
            repetition: 0,
        },
        seed: 163,
        method,
        smtp,
        http,
        smtp_auth: testexec::smtp_auth(target),
        plan,
        clock,
        test_deadline: b.test,
        suite_deadline: b.suite,
        work_end: b.work_end,
    };
    let mut kits = Vec::new();
    for _ in 0..parallelism {
        kits.push(WorkerKit::bind(&cfg).await.unwrap());
    }
    let rec = tokio::time::timeout(
        Duration::from_secs(170),
        testexec::run_suite(cfg, kits, &LocalMeter(clock)),
    )
    .await
    .expect("the suite returns within its bounds")
    .unwrap();
    // Every planned id is accounted for exactly once, started or not.
    let mut ids: Vec<u32> = rec
        .rows
        .iter()
        .map(|r| r.id)
        .chain(rec.unstarted.iter().map(|u| u.id))
        .collect();
    ids.sort();
    let mut want: Vec<u32> = rec.plan.iter().map(|t| t.id).collect();
    want.sort();
    assert_eq!(ids, want);
    rec
}

fn passed(rec: &suite::SuiteRecord) {
    for r in &rec.rows {
        assert_eq!(r.outcome, Outcome::Pass, "{r:?}");
        assert!(r.cleanup.proved(rec.policy), "{r:?}");
        let w = r.wait.as_ref().expect("a pass waited");
        // The 250 that completed the required set came before the wait
        // returned, on the one clock: a signed, positive offset.
        let ack = w.ack_to_return_ns.expect("the completing 250 was observed");
        assert!(ack > 0, "{r:?}");
        assert!(w.call_ns >= r.claim_ns && w.return_ns <= r.end_ns, "{r:?}");
    }
    assert!(rec.unstarted.is_empty(), "{:?}", rec.unstarted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_reference_flow_passes_on_the_real_candidate_in_its_own_inbox() {
    let c = candidate().await;
    let rec = run(
        "swarmail",
        (c.smtp, c.http),
        8,
        ten(),
        (WaitMethod::Native, generous()),
    )
    .await;
    assert_eq!(rec.policy, Policy::InboxClear);
    assert_eq!(rec.rows.len(), 10);
    passed(&rec);
    let flows: BTreeSet<&str> = rec.rows.iter().map(|r| r.flow.as_str()).collect();
    assert_eq!(flows.len(), 5);
    for r in &rec.rows {
        // Per-test inboxes: no other test's mail is ever in view.
        assert_eq!(r.foreign_seen, 0, "{r:?}");
        assert_eq!(r.wait.as_ref().unwrap().cause, "native");
        // The count waits complete on their last required acceptance.
        let want = match r.flow.as_str() {
            "otp-backlog" => 12,
            "recipient-crowd" => 5,
            _ => 1,
        };
        assert_eq!(r.wait.as_ref().unwrap().count, want, "{r:?}");
        assert!(
            matches!(r.cleanup, Cleanup::Done { removed } if removed >= want as u64),
            "{r:?}"
        );
    }
    let report = suite::report(&rec);
    assert_eq!(report["by_class"]["pass"], json!(10));
    assert_eq!(report["started"], json!(10));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shared_store_suite_sees_foreign_mail_and_deletes_only_its_own() {
    let s = scripted(Kind::Mailpit).await;
    // Another suite's leftovers on the same store: a neighbour's mail that
    // the subject search returns and the exact match drops.
    {
        let mut st = s.store.lock().unwrap();
        for n in 150..153 {
            let to = suite::primary(n);
            st.add(Kind::Mailpit, &to, raw(&to, "Your sign-in link", "noise"));
        }
    }
    let rec = run(
        "mailpit",
        (s.smtp, s.http),
        8,
        ten(),
        (WaitMethod::Poll { interval_ms: 100 }, generous()),
    )
    .await;
    assert_eq!(rec.policy, Policy::DeleteById);
    passed(&rec);
    // The magic-login trials saw the neighbour's three sign-in mails
    // (counted once each however often they were scanned), and passed.
    let magic: Vec<_> = rec
        .rows
        .iter()
        .filter(|r| r.flow == "magic-login")
        .collect();
    assert_eq!(magic.len(), 2);
    for r in &magic {
        assert!(r.foreign_seen >= 3, "{r:?}");
    }
    // Only the neighbour's mail is left: every trial deleted exactly its
    // own, decoys included, and no store-wide clear ever ran.
    let st = s.store.lock().unwrap();
    let left: BTreeSet<&str> = st.msgs.iter().map(|m| m.to.as_str()).collect();
    assert_eq!(left.len(), 3, "{left:?}");
    assert!(!st.mutations.iter().any(|m| m == "all"));
}

/// `n` other tests' mail under a subject no flow waits for: enough for a
/// store-wide listing of two pages.
fn neighbours(st: &mut Store, n: u32) {
    for i in 0..n {
        let to = suite::primary(150 + i % 40);
        st.add(Kind::Mailpit, &to, raw(&to, "Neighbour", "noise"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_parallel_suites_cleanup_reads_a_listing_its_neighbours_moved_again() {
    // Mailpit's cleanup reads the whole store (no recipient filter is
    // cited). Beside seven other workers its stated total moves between
    // pages; the first ten listings here do, then the store is still.
    let s = scripted(Kind::Mailpit).await;
    {
        let mut st = s.store.lock().unwrap();
        neighbours(&mut st, 260);
        st.move_on = (1..=10).collect();
    }
    let rec = run(
        "mailpit",
        (s.smtp, s.http),
        8,
        ten(),
        (WaitMethod::Poll { interval_ms: 100 }, generous()),
    )
    .await;
    assert_eq!(rec.policy, Policy::DeleteById);
    // Every row is a pass with its cleanup proved, none `harness`.
    passed(&rec);
    let st = s.store.lock().unwrap();
    assert!(st.move_on.is_empty(), "every moving listing was served");
    // Every trial deleted exactly its own: the 260 and the ten deliveries
    // that moved the listings are all still there, and nothing cleared.
    assert_eq!(st.msgs.len(), 270);
    assert!(st.msgs.iter().all(|m| m.subject == "Neighbour"));
    assert!(!st.mutations.iter().any(|m| m == "all"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_global_clear_only_store_clears_per_test_alone_and_once_after_a_parallel_suite() {
    // One worker: the global purge runs after each test, inside its budget.
    let s = scripted(Kind::Greenmail).await;
    let one = suite::plan_ids(163, [100, 101]);
    let rec = run(
        "greenmail",
        (s.smtp, s.http),
        1,
        one,
        (WaitMethod::Poll { interval_ms: 100 }, generous()),
    )
    .await;
    assert_eq!(rec.policy, Policy::GlobalClearPerTest);
    passed(&rec);
    assert!(rec.final_cleanup.is_none());
    assert_eq!(s.store.lock().unwrap().mutations, ["all", "all"]);
    // Eight workers: no per-test cleanup, one purge after every worker
    // settled and after the window closed.
    let s = scripted(Kind::Greenmail).await;
    let rec = run(
        "greenmail",
        (s.smtp, s.http),
        8,
        ten(),
        (WaitMethod::Poll { interval_ms: 100 }, generous()),
    )
    .await;
    assert_eq!(rec.policy, Policy::GlobalClearAfterSuite);
    passed(&rec);
    assert!(rec.rows.iter().all(|r| r.cleanup == Cleanup::Deferred));
    let f = rec.final_cleanup.as_ref().expect("the suite's one clear");
    assert!(
        matches!(f.cleanup, Cleanup::Done { removed } if removed > 0),
        "{f:?}"
    );
    let last = rec.rows.iter().map(|r| r.end_ns).max().unwrap();
    assert!(f.start_ns >= last);
    assert!(f.start_ns >= rec.window_end.as_ref().unwrap().received_ns);
    let st = s.store.lock().unwrap();
    assert_eq!(st.mutations, ["all"]);
    assert!(st.msgs.is_empty());
}

// ---- the source on its own: filters, pages, identities, cleanup -----------

fn crowd() -> suite::PlannedTrial {
    let t = suite::plan_ids(163, [101]).remove(0);
    assert_eq!(t.flow, "recipient-crowd");
    t
}

fn msg(kind: Kind, id: &str, to: &str, subject: &str) -> Msg {
    Msg {
        id: id.into(),
        mailbox: match kind {
            Kind::Mailpit => String::new(),
            Kind::Inbucket => local(to),
            Kind::Greenmail => to.to_ascii_lowercase(),
        },
        to: to.into(),
        subject: subject.into(),
        raw: raw(to, subject, &format!("Hi {to}")),
    }
}

/// How far past its deadline a bounded operation may be seen to return on
/// a loaded host: scheduling, not a budget.
const REQUEST_SLACK: Duration = Duration::from_secs(1);

fn soon() -> (Instant, Clock) {
    (
        Instant::now() + Duration::from_secs(20),
        Clock(std::time::Instant::now()),
    )
}

#[tokio::test]
async fn a_noisy_paged_filter_is_scanned_completely_and_foreign_mail_counted_once() {
    let s = scripted(Kind::Mailpit).await;
    let me = crowd();
    {
        let mut st = s.store.lock().unwrap();
        // 300 other tests' confirmations, more than one 250-message page,
        // addressed to their primaries and their own crowd aliases.
        for i in 0..300u32 {
            let other = 110 + i % 90;
            let to = suite::owned("recipient-crowd", other)[(i % 5) as usize].clone();
            st.add(
                Kind::Mailpit,
                &to,
                raw(&to, "Confirm your account", "noise"),
            );
        }
        // This test's five arrive after the first complete scan (its two
        // pages), so the source must rescan; one alias is the `…2` form.
        let mine: Vec<Msg> = suite::owned(&me.flow, me.id)
            .iter()
            .enumerate()
            .map(|(i, a)| {
                msg(
                    Kind::Mailpit,
                    &format!("own-{i}"),
                    a,
                    "Confirm your account",
                )
            })
            .collect();
        st.later.insert(3, mine);
    }
    let client = clients::Client::new("mailpit", s.http).unwrap();
    let mut src = NativeSource::new(
        &client,
        s.http,
        WaitMethod::Poll { interval_ms: 20 },
        &me,
        soon(),
    );
    let got = src
        .wait_emails(Some("Confirm your account"), 5)
        .await
        .unwrap();
    let to: BTreeSet<String> = got
        .iter()
        .map(|m| m["to"][0]["address"].as_str().unwrap().to_owned())
        .collect();
    let owned: BTreeSet<String> = suite::owned(&me.flow, me.id).into_iter().collect();
    assert_eq!(to, owned);
    // Scanned at least twice, every foreign message counted once.
    assert!(s.store.lock().unwrap().scans >= 4);
    assert_eq!(src.foreign_seen(), 300);
    assert_eq!(src.owned_seen(), 5);
    assert_eq!(src.timing.as_ref().unwrap().cause, "poll");
}

#[tokio::test]
async fn a_partial_page_and_a_contradictory_identity_are_operation_failures() {
    let me = crowd();
    let owned = suite::owned(&me.flow, me.id);
    let s = scripted(Kind::Mailpit).await;
    {
        let mut st = s.store.lock().unwrap();
        for a in &owned {
            st.add(Kind::Mailpit, a, raw(a, "Confirm your account", "mine"));
        }
        st.inflate = true;
    }
    let client = clients::Client::new("mailpit", s.http).unwrap();
    let mut src = NativeSource::new(
        &client,
        s.http,
        WaitMethod::Poll { interval_ms: 20 },
        &me,
        soon(),
    );
    let e = src
        .wait_emails(Some("Confirm your account"), 5)
        .await
        .unwrap_err();
    assert_eq!(e, "operation: scan: incomplete");
    assert!(matches!(src.failure, Some(Failure::Operation(_))));
    // A read that names another recipient than its listing did.
    {
        let mut st = s.store.lock().unwrap();
        st.inflate = false;
        st.lie_to = Some(suite::primary(150));
    }
    let mut src = NativeSource::new(
        &client,
        s.http,
        WaitMethod::Poll { interval_ms: 20 },
        &me,
        soon(),
    );
    let e = src
        .wait_emails(Some("Confirm your account"), 5)
        .await
        .unwrap_err();
    assert!(e.starts_with("identity: message "), "{e}");
    assert!(matches!(src.failure, Some(Failure::Identity(_))));
}

#[tokio::test]
async fn mailbox_local_ids_stay_apart_and_missing_mail_times_out() {
    let me = crowd();
    let s = scripted(Kind::Inbucket).await;
    {
        let mut st = s.store.lock().unwrap();
        // Each alias's mailbox numbers its mail from 1: five messages, one id.
        for a in suite::owned(&me.flow, me.id) {
            st.add(Kind::Inbucket, &a, raw(&a, "Confirm your account", "mine"));
        }
        assert!(st.msgs.iter().all(|m| m.id == "1"));
    }
    let client = clients::Client::new("inbucket", s.http).unwrap();
    let mut src = NativeSource::new(
        &client,
        s.http,
        WaitMethod::Poll { interval_ms: 20 },
        &me,
        soon(),
    );
    let got = src
        .wait_emails(Some("Confirm your account"), 5)
        .await
        .unwrap();
    assert_eq!(got.len(), 5);
    // Asking for one more than exists ends on the test's deadline, a
    // timeout and never interference: nothing vanished.
    let deadline = (
        Instant::now() + Duration::from_millis(300),
        Clock(std::time::Instant::now()),
    );
    let mut src = NativeSource::new(
        &client,
        s.http,
        WaitMethod::Poll { interval_ms: 20 },
        &me,
        deadline,
    );
    let e = src
        .wait_emails(Some("Confirm your account"), 6)
        .await
        .unwrap_err();
    assert!(e.starts_with("timeout: "), "{e}");
    assert!(src.vanished().is_empty());
}

#[tokio::test]
async fn delete_by_id_removes_only_owned_mail_and_a_failed_delete_never_escalates() {
    let me = crowd();
    let s = scripted(Kind::Mailpit).await;
    {
        let mut st = s.store.lock().unwrap();
        for a in suite::owned(&me.flow, me.id) {
            st.add(Kind::Mailpit, &a, raw(&a, "Confirm your account", "mine"));
        }
        // A leftover of this test under another subject is still its own.
        let p = suite::primary(me.id);
        st.add(Kind::Mailpit, &p, raw(&p, "Reminder", "left over"));
        for n in [150, 151] {
            let to = suite::primary(n);
            st.add(
                Kind::Mailpit,
                &to,
                raw(&to, "Confirm your account", "neighbour"),
            );
        }
    }
    let client = clients::Client::new("mailpit", s.http).unwrap();
    let log = Mutations::default();
    let src = NativeSource::new(
        &client,
        s.http,
        WaitMethod::Poll { interval_ms: 20 },
        &me,
        soon(),
    );
    assert_eq!(
        src.cleanup(Policy::DeleteById, &log).await,
        Cleanup::Done { removed: 6 }
    );
    {
        let st = s.store.lock().unwrap();
        let left: Vec<&str> = st.msgs.iter().map(|m| m.to.as_str()).collect();
        assert_eq!(left, [suite::primary(150), suite::primary(151)]);
        assert!(!st.mutations.iter().any(|m| m == "all"));
    }
    // A refused delete of one owned id stops there: no broader mutation,
    // and the neighbours' mail is untouched.
    {
        let mut st = s.store.lock().unwrap();
        for a in suite::owned(&me.flow, me.id) {
            st.add(Kind::Mailpit, &a, raw(&a, "Confirm your account", "again"));
        }
        let first = st
            .msgs
            .iter()
            .find(|m| m.to == suite::primary(me.id))
            .unwrap()
            .id
            .clone();
        st.fail_delete.insert(first);
        st.mutations.clear();
    }
    let c = src.cleanup(Policy::DeleteById, &log).await;
    assert!(
        matches!(&c, Cleanup::Failed { reason } if reason.contains("status_500")),
        "{c:?}"
    );
    let st = s.store.lock().unwrap();
    assert!(
        !st.mutations.iter().any(|m| m == "all"),
        "{:?}",
        st.mutations
    );
    assert_eq!(
        st.msgs
            .iter()
            .filter(|m| m.to.starts_with("test-15"))
            .count(),
        2
    );
}

#[tokio::test]
async fn a_moving_store_wide_listing_is_reread_only_beside_neighbours_and_only_in_the_deadline() {
    let me = crowd();
    let s = scripted(Kind::Mailpit).await;
    let own = |st: &mut Store| {
        for a in suite::owned(&me.flow, me.id) {
            st.add(Kind::Mailpit, &a, raw(&a, "Confirm your account", "mine"));
        }
        let p = suite::primary(me.id);
        st.add(Kind::Mailpit, &p, raw(&p, "Reminder", "left over"));
    };
    {
        let mut st = s.store.lock().unwrap();
        neighbours(&mut st, 260);
        own(&mut st);
        // The recovery's first listing and the verification's first both
        // move between their two pages.
        st.move_on = [1, 3].into();
    }
    let client = clients::Client::new("mailpit", s.http).unwrap();
    let log = Mutations::default();
    let source = |deadline: (Instant, Clock), beside: bool| {
        NativeSource::new(
            &client,
            s.http,
            WaitMethod::Poll { interval_ms: 20 },
            &me,
            deadline,
        )
        .beside_neighbours(beside)
    };
    // Beside other workers: each moved listing is read again, and the
    // complete ones recover, delete and verify exactly this test's six.
    let c = source(soon(), true).cleanup(Policy::DeleteById, &log).await;
    assert_eq!(c, Cleanup::Done { removed: 6 });
    {
        let st = s.store.lock().unwrap();
        assert_eq!(st.first_pages, 4);
        assert_eq!(st.msgs.len(), 262);
        assert!(st.msgs.iter().all(|m| m.subject == "Neighbour"));
        assert_eq!(st.mutations.len(), 6);
    }
    // Alone on the store (parallelism 1) nothing else can move it: the
    // moving listing is a failure at once, and nothing is deleted.
    {
        let mut st = s.store.lock().unwrap();
        own(&mut st);
        st.mutations.clear();
        st.move_on = [st.first_pages + 1].into();
    }
    let c = source(soon(), false)
        .cleanup(Policy::DeleteById, &log)
        .await;
    assert_eq!(
        c,
        Cleanup::Failed {
            reason: "recover: operation: scan: incomplete".into()
        }
    );
    assert!(s.store.lock().unwrap().mutations.is_empty());
    // A listing that never settles ends inside the cleanup's own deadline,
    // failed, and authorizes no mutation at all.
    {
        let mut st = s.store.lock().unwrap();
        let from = st.first_pages + 1;
        st.move_on = (from..from + 10_000).collect();
    }
    let started = Instant::now();
    let by = started + Duration::from_millis(300);
    let c = source((by, Clock(std::time::Instant::now())), true)
        .cleanup(Policy::DeleteById, &log)
        .await;
    assert!(Instant::now() <= by + REQUEST_SLACK, "{c:?}");
    assert!(
        matches!(&c, Cleanup::Failed { reason }
            if reason.starts_with("recover: operation: scan: incomplete (")),
        "{c:?}"
    );
    let st = s.store.lock().unwrap();
    assert!(st.mutations.is_empty(), "{:?}", st.mutations);
    assert_eq!(
        st.msgs.iter().filter(|m| m.subject != "Neighbour").count(),
        6
    );
}

// ---- the station: one listener pair, one generation at a time -------------

async fn admin(
    station: SocketAddr,
    secret: &str,
    path: &str,
    body: Option<Value>,
) -> reqwest::Result<reqwest::Response> {
    let c = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut r = c
        .post(format!("http://{station}{path}"))
        .header("authorization", format!("Bearer {secret}"));
    if let Some(b) = body {
        r = r
            .header("content-type", "application/json")
            .body(b.to_string());
    }
    r.send().await
}

fn trial_cfg(
    flow: &str,
    id: u32,
    secret: &str,
    acks: Option<swarmail_evals::fixture::AckHook>,
) -> swarmail_evals::fixture::TrialConfig {
    let t = suite::plan_ids(163, [id]).remove(0);
    swarmail_evals::fixture::TrialConfig {
        task: swarmail_evals::tasks::find(flow).unwrap(),
        trial_id: format!("testexec-{id}"),
        admin_secret: secret.into(),
        inputs: swarmail_evals::provenance::FixtureInputs {
            seed: t.fixture_seed,
            fixture_time: suite::FIXTURE_TIME.into(),
            seed_algorithm: swarmail_evals::provenance::ALGORITHM.into(),
            projection_version: 1,
        },
        ack: acks,
    }
}

fn provision(id: u32) -> Value {
    json!({"identities": [{"identity": suite::primary(id), "inbox": suite::inbox(id), "smtp_identity": suite::inbox(id)}]})
}

#[tokio::test]
async fn a_held_handler_finishes_against_its_own_trial_and_never_the_next() {
    let s = scripted(Kind::Mailpit).await;
    s.gated.store(true, std::sync::atomic::Ordering::SeqCst);
    let mut station =
        swarmail_evals::fixture::Station::bind(s.smtp, swarmail_evals::mail::SmtpAuth::None)
            .await
            .unwrap();
    let addr = station.admin;
    let acks: Arc<Mutex<Vec<swarmail_evals::fixture::Ack>>> = Arc::default();
    let a2 = acks.clone();
    let hook: swarmail_evals::fixture::AckHook = Arc::new(move |a| a2.lock().unwrap().push(a));
    station
        .install(trial_cfg("magic-login", 104, "a", Some(hook)))
        .unwrap();
    assert!(
        admin(addr, "a", "/admin/provision", Some(provision(104)))
            .await
            .unwrap()
            .status()
            .is_success()
    );
    // Trial A's prepare is held inside its handler at the final DATA reply.
    let held = tokio::spawn(async move {
        admin(addr, "a", "/admin/prepare", None)
            .await
            .map(|r| r.status().as_u16())
    });
    let until = std::time::Instant::now() + Duration::from_secs(20);
    while s.store.lock().unwrap().msgs.is_empty() {
        assert!(
            std::time::Instant::now() < until,
            "the held delivery never reached the store"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // Retiring A waits for that handler: it has not drained while held.
    let mut retire = Box::pin(station.retire(Duration::from_secs(20)));
    assert!(
        tokio::time::timeout(Duration::from_millis(200), &mut retire)
            .await
            .is_err()
    );
    s.gate.add_permits(1);
    retire.await.unwrap();
    assert_eq!(held.await.unwrap().unwrap(), 200);
    // The 250 it read was reported to A's hook, before its QUIT.
    assert_eq!(acks.lock().unwrap().len(), 1);
    assert!(acks.lock().unwrap()[0].delivered);
    // Between generations the planes answer no one.
    let c = reqwest::Client::builder().no_proxy().build().unwrap();
    assert!(
        c.get(format!("http://{}/", station.public))
            .send()
            .await
            .is_err()
    );
    // B starts clean: no delivery of A's reached it.
    station
        .install(trial_cfg("magic-login", 109, "b", None))
        .unwrap();
    let snap = c
        .get(format!("http://{}/admin/snapshot", station.admin))
        .header("authorization", "Bearer b")
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let snap: Value = serde_json::from_slice(&snap).unwrap();
    assert_eq!(snap["deliveries"], json!([]), "{snap}");
    station.retire(Duration::from_secs(5)).await.unwrap();
    station.stop().await;
}

#[tokio::test]
async fn a_generation_that_does_not_drain_in_its_bound_cuts_the_station() {
    let s = scripted(Kind::Mailpit).await;
    s.gated.store(true, std::sync::atomic::Ordering::SeqCst);
    let mut station =
        swarmail_evals::fixture::Station::bind(s.smtp, swarmail_evals::mail::SmtpAuth::None)
            .await
            .unwrap();
    let addr = station.admin;
    station
        .install(trial_cfg("magic-login", 104, "a", None))
        .unwrap();
    admin(addr, "a", "/admin/provision", Some(provision(104)))
        .await
        .unwrap();
    let held = tokio::spawn(async move {
        let _ = admin(addr, "a", "/admin/prepare", None).await;
    });
    let until = std::time::Instant::now() + Duration::from_secs(20);
    while s.store.lock().unwrap().msgs.is_empty() {
        assert!(std::time::Instant::now() < until);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let e = station
        .retire(Duration::from_millis(100))
        .await
        .unwrap_err();
    assert!(e.contains("did not drain"), "{e}");
    // No next trial is installed beside the live handler.
    let e = station
        .install(trial_cfg("magic-login", 109, "b", None))
        .unwrap_err();
    assert!(e.contains("still installed"), "{e}");
    s.gate.add_permits(1);
    let _ = held.await;
    station.stop().await;
}

// ---- budgets: absolute, every planned id accounted for --------------------

#[tokio::test]
async fn claims_stop_at_the_chunks_end_and_the_suites_cutoff_with_every_id_unstarted() {
    let s = scripted(Kind::Mailpit).await;
    let past = Bounds {
        test: Duration::from_secs(30),
        suite: Duration::from_secs(120),
        work_end: Instant::now(),
    };
    let rec = run(
        "mailpit",
        (s.smtp, s.http),
        8,
        ten(),
        (WaitMethod::Poll { interval_ms: 100 }, past),
    )
    .await;
    assert!(rec.rows.is_empty());
    assert!(
        rec.unstarted.iter().all(|u| u.reason == "chunk_deadline"),
        "{:?}",
        rec.unstarted
    );
    // Nothing could be claimed, so the window was never opened: the
    // holder's rule for a suite that started nothing, sealed as its loss.
    assert!(rec.window_start.is_none() && rec.window_end.is_none());
    let cut = Bounds {
        test: Duration::from_secs(30),
        suite: Duration::ZERO,
        work_end: Instant::now() + Duration::from_secs(60),
    };
    let rec = run(
        "mailpit",
        (s.smtp, s.http),
        8,
        ten(),
        (WaitMethod::Poll { interval_ms: 100 }, cut),
    )
    .await;
    assert!(rec.rows.is_empty());
    assert_eq!(rec.unstarted.len(), 10);
    assert!(rec.unstarted.iter().all(|u| u.reason == "suite_deadline"));
    assert!(rec.window_start.is_none() && rec.window_end.is_none());
    // The figures still say what was planned, started and why not.
    let r = suite::report(&rec);
    assert_eq!(
        (
            r["planned"].clone(),
            r["started"].clone(),
            r["unstarted"].clone()
        ),
        (json!(10), json!(0), json!(10))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_poll_wait_that_times_out_is_a_timeout_row_that_still_removes_its_leftovers() {
    // A delete-by-id poll store whose subject search never finds the mail
    // the fixture delivered: the wait exhausts its deadline. The row is the
    // target's `timeout` (never `harness` from a scoring step that ran out
    // of time), and the reserved tail still recovers the delivered mail
    // from a complete listing and deletes it, within the test's bound.
    let s = scripted(Kind::Mailpit).await;
    s.store.lock().unwrap().blind_search = true;
    let test = testexec::CLAIM_FLOOR + Duration::from_secs(1);
    let b = Bounds {
        test,
        suite: Duration::from_secs(60),
        work_end: Instant::now() + Duration::from_secs(60),
    };
    let plan = suite::plan_ids(163, [100, 101]);
    let rec = run(
        "mailpit",
        (s.smtp, s.http),
        1,
        plan,
        (WaitMethod::Poll { interval_ms: 100 }, b),
    )
    .await;
    assert_eq!(rec.rows.len(), 2, "{:?}", rec.unstarted);
    for r in &rec.rows {
        assert_eq!(r.outcome, Outcome::Timeout, "{r:?}");
        assert!(r.cause.as_deref().unwrap().starts_with("timeout:"), "{r:?}");
        assert!(r.wait.is_none() && r.wait_absent.is_some(), "{r:?}");
        assert!(
            matches!(r.cleanup, Cleanup::Done { removed } if removed >= 1),
            "{r:?}"
        );
        assert!(r.end_ns - r.claim_ns <= test.as_nanos() as u64, "{r:?}");
    }
    // Every delivered message went, by id; nothing store-wide ran.
    let st = s.store.lock().unwrap();
    assert!(st.msgs.is_empty(), "{:?}", st.msgs);
    assert!(!st.mutations.is_empty() && !st.mutations.iter().any(|m| m == "all"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fixture_that_cannot_drain_cuts_the_suite_and_no_global_clear_runs_beside_it() {
    // Eight workers on a global-only store. Each first trial's prepare is
    // held at its first final DATA reply; while it is, a connection is
    // opened into every installed generation and left mid-request. The
    // trials then finish, but no generation can prove quiescence within its
    // drain bound: the suite is cut, the rest stay unstarted, and the
    // store-wide clear (which would reach the live handlers' work) never
    // runs.
    let s = scripted(Kind::Greenmail).await;
    s.gated.store(true, std::sync::atomic::Ordering::SeqCst);
    let clock = Clock(std::time::Instant::now());
    let cfg = SuiteConfig {
        coord: Coord {
            target: "greenmail".into(),
            parallelism: 8,
            repetition: 0,
        },
        seed: 163,
        method: WaitMethod::Poll { interval_ms: 100 },
        smtp: s.smtp,
        http: s.http,
        smtp_auth: testexec::smtp_auth("greenmail"),
        plan: ten(),
        clock,
        test_deadline: Duration::from_secs(30),
        suite_deadline: Duration::from_secs(60),
        work_end: Instant::now() + Duration::from_secs(60),
    };
    let mut kits = Vec::new();
    for _ in 0..8 {
        kits.push(WorkerKit::bind(&cfg).await.unwrap());
    }
    let publics: Vec<SocketAddr> = kits.iter().map(|k| k.station.public).collect();
    let started = std::time::Instant::now();
    let suite =
        tokio::spawn(async move { testexec::run_suite(cfg, kits, &LocalMeter(clock)).await });
    let until = std::time::Instant::now() + Duration::from_secs(20);
    while s.store.lock().unwrap().msgs.len() < 8 {
        assert!(
            std::time::Instant::now() < until,
            "eight prepares never reached the store"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let mut held = Vec::new();
    for p in publics {
        let mut c = tokio::net::TcpStream::connect(p).await.unwrap();
        c.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n").await.unwrap();
        held.push(c);
    }
    s.gated.store(false, std::sync::atomic::Ordering::SeqCst);
    s.gate.add_permits(64);
    let rec = tokio::time::timeout(Duration::from_secs(60), suite)
        .await
        .expect("the cut suite returns within its bounds")
        .unwrap()
        .unwrap();
    // Bounded: the trials, one drain bound, and nothing more.
    assert!(
        started.elapsed() < testexec::DRAIN_BOUND + Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(rec.rows.len(), 8, "{:?}", rec.unstarted);
    for r in &rec.rows {
        assert_eq!(r.outcome, Outcome::Harness, "{r:?}");
        assert!(
            r.cause
                .as_deref()
                .is_some_and(|c| c.contains("did not drain")),
            "{r:?}"
        );
    }
    assert_eq!(rec.unstarted.len(), 2);
    assert!(
        rec.unstarted
            .iter()
            .all(|u| u.reason.starts_with("suite_cut: ")),
        "{:?}",
        rec.unstarted
    );
    let f = rec.final_cleanup.as_ref().unwrap();
    assert!(
        matches!(&f.cleanup, Cleanup::Failed { reason } if reason.starts_with("not run")),
        "{f:?}"
    );
    assert!(s.store.lock().unwrap().mutations.is_empty());
    drop(held);
}

// ---- the fixture's own facts the suite relies on --------------------------

#[tokio::test]
async fn each_flow_sends_its_waited_subject_to_exactly_its_owned_addresses() {
    let waited = [
        ("otp-backlog", "Your verification code"),
        ("recipient-crowd", "Confirm your account"),
        ("digest-lookalike", "Your weekly digest"),
        ("reset-login", "Reset your password"),
        ("magic-login", "Your sign-in link"),
    ];
    for (i, (flow, subject)) in waited.iter().enumerate() {
        let id = suite::FIRST_ID + i as u32;
        assert_eq!(suite::plan_ids(163, [id])[0].flow, *flow);
        let s = scripted(Kind::Mailpit).await;
        let mut station =
            swarmail_evals::fixture::Station::bind(s.smtp, swarmail_evals::mail::SmtpAuth::None)
                .await
                .unwrap();
        station.install(trial_cfg(flow, id, "x", None)).unwrap();
        admin(station.admin, "x", "/admin/provision", Some(provision(id)))
            .await
            .unwrap();
        let r = admin(station.admin, "x", "/admin/prepare", None)
            .await
            .unwrap();
        assert!(r.status().is_success(), "{flow}");
        {
            let st = s.store.lock().unwrap();
            assert!(!st.msgs.is_empty());
            assert!(
                st.msgs.iter().all(|m| m.subject.contains(subject)),
                "{flow}"
            );
            let to: BTreeSet<String> = st.msgs.iter().map(|m| m.to.clone()).collect();
            let owned: BTreeSet<String> = suite::owned(flow, id).into_iter().collect();
            assert_eq!(to, owned, "{flow}");
        }
        station.retire(Duration::from_secs(5)).await.unwrap();
        station.stop().await;
    }
}

#[tokio::test]
async fn the_suites_app_client_stays_on_its_fixtures_origin_and_follows_no_redirect() {
    let router = Router::new().route(
        "/a/{t}",
        get(|| async {
            (
                axum::http::StatusCode::FOUND,
                [("location", "http://127.0.0.2:9/steal")],
                "",
            )
        }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move {
        let _ = axum::serve(l, router).await;
    });
    let app = swarmail_evals::oracle::App::confined(
        &base,
        "test-100@suite.invalid",
        Duration::from_secs(5),
    )
    .unwrap();
    let (status, _) = app.get(&format!("{base}/a/x")).await.unwrap();
    assert_eq!(status, 302);
    for elsewhere in ["http://127.0.0.2:9/a/x", &format!("{base}.evil/a/x")] {
        let e = app.get(elsewhere).await.unwrap_err();
        assert!(e.contains("outside the fixture's origin"), "{e}");
    }
    let e = app
        .post("http://127.0.0.2:9/login", json!({}))
        .await
        .unwrap_err();
    assert!(e.contains("outside the fixture's origin"), "{e}");
    assert_eq!(app.requests(), 1);
}

// ---- the subcommand refuses before it spawns anything ---------------------

#[test]
fn testexec_refuses_bad_coordinates_pins_protocols_and_outputs_before_any_launch() {
    let dir = common::tempdir();
    let marker = dir.path().join("spawned");
    let driver = dir.path().join("driver");
    std::fs::write(
        &driver,
        format!("#!/bin/sh\ntouch '{}'\nexit 2\n", marker.display()),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&driver, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let root = common::repo_root();
    let draft = root.join("docs/evals/draft/protocol.v4.json");
    let mut p: Value = serde_json::from_slice(&std::fs::read(&draft).unwrap()).unwrap();
    p.as_object_mut().unwrap().remove("testexec");
    let none = dir.path().join("none.json");
    std::fs::write(&none, p.to_string()).unwrap();
    let mut p: Value = serde_json::from_slice(&std::fs::read(&draft).unwrap()).unwrap();
    p["testexec"]["test_deadline_ms"] = json!(60_000);
    let tuned = dir.path().join("tuned.json");
    std::fs::write(&tuned, p.to_string()).unwrap();
    let bin = dir.path().join("mailpit-bin");
    std::fs::write(&bin, b"").unwrap();
    let taken = dir.path().join("taken");
    std::fs::create_dir(&taken).unwrap();
    let ok_bin = format!("mailpit={}", bin.display());
    let cases: Vec<(&str, String, std::path::PathBuf, std::path::PathBuf, &str)> = vec![
        (
            "testexec.mailslurper.p1.r0",
            ok_bin.clone(),
            draft.clone(),
            dir.path().join("o1"),
            "does not start",
        ),
        (
            "testexec.mailpit.p4.r0",
            ok_bin.clone(),
            draft.clone(),
            dir.path().join("o2"),
            "not one of 1, 8, 32",
        ),
        (
            "testexec.mailpit.p8.r3",
            ok_bin.clone(),
            draft.clone(),
            dir.path().join("o3"),
            "outside 0..3",
        ),
        (
            "testexec.nope.p8.r0",
            ok_bin.clone(),
            draft.clone(),
            dir.path().join("o4"),
            "unknown target",
        ),
        (
            "api.mcp.mailpit.r0.f0",
            ok_bin.clone(),
            draft.clone(),
            dir.path().join("o5"),
            "is not a testexec",
        ),
        (
            "testexec.mailpit.p8.r0",
            format!("swarmail={}", bin.display()),
            draft.clone(),
            dir.path().join("o6"),
            "--bin names swarmail",
        ),
        (
            "testexec.mailpit.p8.r0",
            "mailpit=/nonexistent/mailpit".into(),
            draft.clone(),
            dir.path().join("o7"),
            "is not a file",
        ),
        (
            "testexec.mailpit.p8.r0",
            ok_bin.clone(),
            draft.clone(),
            taken.clone(),
            "already exists",
        ),
        (
            "testexec.mailpit.p8.r0",
            ok_bin.clone(),
            none.clone(),
            dir.path().join("o8"),
            "no testexec section",
        ),
        (
            "testexec.mailpit.p8.r0",
            ok_bin.clone(),
            tuned.clone(),
            dir.path().join("o9"),
            "not 0132's",
        ),
    ];
    for (chunk, bin, protocol, output, want) in cases {
        let out = std::process::Command::new(common::exe())
            .arg("testexec")
            .arg("--driver")
            .arg(&driver)
            .args(["--mode", "smoke", "--protocol"])
            .arg(&protocol)
            .args(["--sources", "docs/evals/draft/sources.lock.v5.json"])
            .args(["--chunk", chunk, "--bin", &bin, "--run-id", "refusal"])
            .arg("--output")
            .arg(&output)
            .arg("--scratch")
            .arg(dir.path().join("scratch"))
            .current_dir(&root)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{chunk}: {err}");
        assert!(
            err.contains("refused before launch") && err.contains(want),
            "{chunk}: {want}: {err}"
        );
        assert!(!marker.exists(), "{chunk}: the holder was spawned");
    }
}

// ---- the subcommand end to end, against a scripted holder ----------------

/// A holder that announces the real candidate's endpoints and its client
/// CPUs, answers the suite window on its own (fixed) clock, keeps the record
/// it is sent, and on EOF prints a proved receipt and exits `code`.
fn scripted_holder(dir: &std::path::Path, endpoints: &Value, code: i32) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = dir.join("driver");
    let rec = dir.join("record.json");
    let eof = dir.join("eof");
    let receipt = json!({
        "launch": {"target": "swarmail", "leaf": "target-0001"},
        "cleanup": {"target": "swarmail", "leaf": "target-0001", "cgroup_removed": true,
            "reaped": true, "group_gone": true, "listeners_closed": true,
            "scratch_removed": true, "forced_kill": false},
    });
    let body = format!(
        "#!/bin/bash\n[ \"$1\" = probe ] && exit 0\n[ \"$1\" = hold ] || exit 9\necho '{endpoints}'\nwhile IFS= read -r line; do\n  case \"$line\" in\n    *'\"cmd\":\"record\"'*) printf '%s\\n' \"$line\" > '{}'; echo '{{\"ok\":\"record\"}}' ;;\n    *'\"cmd\":\"window_start\"'*) echo '{{\"ok\":\"window_start\",\"id\":\"suite\",\"t_ns\":11}}' ;;\n    *'\"cmd\":\"window_end\"'*) echo '{{\"ok\":\"window_end\",\"id\":\"suite\",\"t_ns\":22}}' ;;\n    *) echo '{{\"error\":\"unknown\"}}' ;;\n  esac\ndone\ntouch '{}'\necho '{receipt}'\nexit {code}\n",
        rec.display(),
        eof.display()
    );
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match std::process::Command::new(&p).arg("probe").status() {
            Ok(s) if s.success() => return p,
            Err(e) if e.raw_os_error() == Some(26) => std::thread::sleep(Duration::from_millis(5)),
            other => panic!("the scripted holder does not run: {other:?}"),
        }
    }
    panic!("the scripted holder stayed busy")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn the_subcommand_runs_the_whole_suite_and_hands_the_holder_a_checked_record() {
    let c = candidate().await;
    let d = common::tempdir();
    let cpus: Vec<u32> =
        (0..std::thread::available_parallelism().map_or(1, |n| n.get() as u32)).collect();
    let endpoints = json!({
        "target": "swarmail", "leaf": "target-0001",
        "smtp": c.smtp.to_string(), "http": c.http.to_string(), "pop3": null,
        "client_cpus": cpus,
    });
    let driver = scripted_holder(d.path(), &endpoints, 0);
    let root = common::repo_root();
    let args: Vec<String> = [
        "--driver",
        &driver.display().to_string(),
        "--mode",
        "smoke",
        "--protocol",
        &root
            .join("docs/evals/draft/protocol.v4.json")
            .display()
            .to_string(),
        "--sources",
        "docs/evals/draft/sources.lock.v5.json",
        "--chunk",
        "testexec.swarmail.p8.r0",
        "--bin",
        &format!("swarmail={}", common::swarmail_bin().display()),
        "--output",
        &d.path().join("out").display().to_string(),
        "--scratch",
        &d.path().join("scratch").display().to_string(),
        "--run-id",
        "e2e",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let code = tokio::time::timeout(Duration::from_secs(300), testexec::testexec(&args))
        .await
        .expect("the suite ends inside its bound");
    assert_eq!(code, 0);
    assert!(
        d.path().join("eof").exists(),
        "the holder never saw its input close"
    );
    let line: Value =
        serde_json::from_slice(&std::fs::read(d.path().join("record.json")).unwrap()).unwrap();
    assert_eq!(line["cmd"], json!("record"));
    let rec: suite::SuiteRecord = serde_json::from_value(line["testexec"].clone()).unwrap();
    let coord = Coord::parse("testexec.swarmail.p8.r0").unwrap();
    suite::check_record(&rec, &coord).unwrap();
    assert_eq!(rec.rows.len(), 100);
    assert!(
        suite::failures(&rec).is_empty(),
        "{:?}",
        suite::failures(&rec)
    );
    // Every worker took part, and each trial ran on its claimed worker.
    let workers: BTreeSet<u32> = rec.rows.iter().map(|r| r.worker).collect();
    assert_eq!(workers.len(), 8);
    // The window's holder instants are the holder's own.
    assert_eq!(rec.window_start.as_ref().unwrap().owner_t_ns, 11);
    assert_eq!(rec.window_end.as_ref().unwrap().owner_t_ns, 22);
    assert!(rec.client_cpu.is_some());
    // The holder exiting 1 with a proved receipt is the chunk's 1.
    let d = common::tempdir();
    let driver = scripted_holder(d.path(), &endpoints, 1);
    let mut args1 = args.clone();
    let out = args1.iter().position(|a| a == "--output").unwrap();
    args1[out + 1] = d.path().join("out").display().to_string();
    let drv = args1.iter().position(|a| a == "--driver").unwrap();
    args1[drv + 1] = driver.display().to_string();
    let p = args1.iter().position(|a| a == "--chunk").unwrap();
    args1[p + 1] = "testexec.swarmail.p1.r1".into();
    // Only the coordinate changed; one worker this time.
    let code = tokio::time::timeout(Duration::from_secs(300), testexec::testexec(&args1))
        .await
        .unwrap();
    assert_eq!(code, 1);
}

// ---- the final DATA reply, observed before QUIT ---------------------------

/// One SMTP session: the dialogue up to the message, then `dot` as the
/// final reply (or a close with none), then a `QUIT` answered only once
/// `quit` is notified.
async fn one_smtp(dot: Option<&'static [u8]>, quit: Arc<tokio::sync::Notify>) -> SocketAddr {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        let (sock, _) = l.accept().await.unwrap();
        let (r, mut w) = sock.into_split();
        let mut r = BufReader::new(r);
        w.write_all(b"220 one\r\n").await.unwrap();
        let mut line = String::new();
        loop {
            line.clear();
            if r.read_line(&mut line).await.unwrap_or(0) == 0 {
                return;
            }
            if line.starts_with("DATA") {
                w.write_all(b"354 go\r\n").await.unwrap();
                loop {
                    line.clear();
                    r.read_line(&mut line).await.unwrap();
                    if line == ".\r\n" {
                        break;
                    }
                }
                let Some(reply) = dot else { return };
                w.write_all(reply).await.unwrap();
            } else if line.starts_with("QUIT") {
                quit.notified().await;
                let _ = w.write_all(b"221 bye\r\n").await;
                return;
            } else {
                w.write_all(b"250 ok\r\n").await.unwrap();
            }
        }
    });
    addr
}

#[tokio::test]
async fn the_final_data_reply_is_observed_before_quit_and_only_when_read() {
    use swarmail_evals::mail::{SendResult, SmtpAuth, send_observed};
    let send = |addr, seen: Arc<Mutex<Vec<(u16, bool)>>>| async move {
        let observe = move |r: &SendResult| {
            seen.lock()
                .unwrap()
                .push((r.attempt().code, matches!(r, SendResult::Delivered(_))))
        };
        send_observed(
            addr,
            SmtpAuth::None,
            "u",
            "a@x",
            "b@x",
            b"Subject: s\r\n\r\nbody\r\n",
            Some(&observe),
        )
        .await
        .0
    };
    // A 250 is observed while the QUIT is still unanswered.
    let quit = Arc::new(tokio::sync::Notify::new());
    let addr = one_smtp(Some(b"250 stored\r\n"), quit.clone()).await;
    let seen: Arc<Mutex<Vec<(u16, bool)>>> = Arc::default();
    let task = tokio::spawn(send(addr, seen.clone()));
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while seen.lock().unwrap().is_empty() {
        assert!(
            std::time::Instant::now() < until,
            "the 250 was never observed"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(*seen.lock().unwrap(), [(250, true)]);
    assert!(
        !task.is_finished(),
        "the send returned before its QUIT was answered"
    );
    quit.notify_one();
    assert!(matches!(task.await.unwrap(), SendResult::Delivered(_)));
    // A refused message is observed as a refusal.
    let seen: Arc<Mutex<Vec<(u16, bool)>>> = Arc::default();
    let quit = Arc::new(tokio::sync::Notify::new());
    quit.notify_one();
    let addr = one_smtp(Some(b"550 no\r\n"), quit).await;
    assert!(matches!(
        send(addr, seen.clone()).await,
        SendResult::Permanent(_)
    ));
    assert_eq!(*seen.lock().unwrap(), [(550, false)]);
    // A lost reply is never observed: its acceptance stays unknown, and
    // the send is a transport failure that is never retried.
    let seen: Arc<Mutex<Vec<(u16, bool)>>> = Arc::default();
    let addr = one_smtp(None, Arc::new(tokio::sync::Notify::new())).await;
    assert!(matches!(
        send(addr, seen.clone()).await,
        SendResult::Transport(_)
    ));
    assert!(seen.lock().unwrap().is_empty());
}
