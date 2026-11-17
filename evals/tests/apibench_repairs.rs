//! Producer-path proofs of story B's review repairs on the MCP surface
//! (specs/api-latency, R1, R3, R8): the real `McpSurface`, `Worker` and
//! `wait_once` against scripted loopback MCP and SMTP servers that choose
//! each answer's timing. Real framing on both protocols; no mocks in the
//! code under test.

#[path = "scripted/mod.rs"]
mod scripted;

use scripted::{Answer, Handler, SmtpScript};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use swarmail_evals::api::{self, Outcome, PlannedCell, Role};
use swarmail_evals::apibench::{self, McpSurface, Worker};
use swarmail_evals::clients::{self, Operation};
use swarmail_evals::mcp::McpClient;
use tokio::sync::Mutex;

fn clock() -> api::Clock {
    api::Clock(std::time::Instant::now())
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

async fn workers(url: &str, n: u64) -> Vec<Worker> {
    let mut out = Vec::new();
    for k in 0..n {
        out.push(Worker {
            client: Mutex::new(
                McpClient::connect_range(url, apibench::id_range(k))
                    .await
                    .unwrap(),
            ),
            target: "swarmail".into(),
        });
    }
    out
}

fn handler(
    f: impl Fn(
        String,
        Value,
        u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Answer> + Send>>
    + Send
    + Sync
    + 'static,
) -> Handler {
    Arc::new(f)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_mcp_surface_keeps_measuring_after_a_read_times_out() {
    // R1 on the MCP surface: the 31st tool call (list.c1's 11th measured
    // operation) is held past the five-second deadline. It is a timeout
    // row, list.c8 after it measures everything, nothing is abandoned.
    let reads: Vec<api::Authored> = (0..api::K)
        .map(|n| {
            let mut a = api::author(7, Role::Read, n, &api::read_scope());
            a.id = Some(format!("r{n}"));
            a
        })
        .collect();
    let page = scripted::swarmail_page(
        &reads
            .iter()
            .map(|a| {
                (
                    a.id.clone().unwrap(),
                    a.scope.address.clone(),
                    a.subject.clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let url = scripted::mcp(handler(move |_, _, n| {
        let page = page.clone();
        Box::pin(async move {
            if n == 31 {
                tokio::time::sleep(Duration::from_secs(8)).await;
            }
            Answer::Result(page)
        })
    }))
    .await;
    let surface = McpSurface {
        workers: Arc::new(workers(&url, 8).await),
        verify: clients::Client::new("swarmail", "127.0.0.1:9".parse().unwrap()).unwrap(),
        fixtures: Arc::new(reads),
        smtp: "127.0.0.1:9".parse().unwrap(),
        seed: 7,
        clock: clock(),
    };
    let caps = |op| clients::mcp_capability("swarmail", op);
    let cells = [cell(Operation::List, 1), cell(Operation::List, 8)];
    let (recs, abandoned) = api::run_cells(&cells, &caps, &surface, &api::NoMeter, clock(), None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    let first = &recs[0].measured;
    assert_eq!(recs[0].status, api::CellStatus::Measured);
    assert_eq!(first.started, 11);
    assert_eq!(first.rows[10].outcome, Outcome::Timeout);
    assert!(first.rows[..10].iter().all(|r| r.outcome == Outcome::Exact));
    assert!(!first.unresolved && !first.cancel_unproved);
    assert_eq!(recs[1].status, api::CellStatus::Measured);
    assert_eq!(recs[1].measured.started, 100);
    assert!(
        recs[1]
            .measured
            .rows
            .iter()
            .all(|r| r.outcome == Outcome::Exact)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_wait_answered_before_its_ack_keeps_a_negative_offset_and_quit_cannot_move_it() {
    // R3: the server answers the wait as soon as the message's terminator
    // arrives; the SMTP `250` is held until the wait call has returned (the
    // worker's session is free again), and the `221` to QUIT never comes.
    // On one thread the return is stamped in the same poll that frees the
    // session, so the ordering is the wire's, not a sleep's.
    let sent = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Notify::new());
    let smtp = scripted::smtp(SmtpScript {
        hold_data_ack: Some(release.clone()),
        data_received: Some(sent.clone()),
        quit_delay: Duration::from_secs(3_600),
    })
    .await;
    let s2 = sent.clone();
    let url = scripted::mcp(handler(move |tool, args, _| {
        let sent = s2.clone();
        Box::pin(async move {
            assert_eq!(tool, "swarmail_wait_for_email");
            sent.acquire().await.unwrap().forget();
            let inbox = args["inbox"].as_str().unwrap().to_owned();
            let subject = args["subject"].as_str().unwrap().to_owned();
            Answer::Result(json!({"emails": [{"id": "w1", "subject": subject,
                "to": [{"address": format!("{inbox}@{}", clients::DOMAIN)}]}]}))
        })
    }))
    .await;
    let worker = Arc::new(workers(&url, 1).await.pop().unwrap());
    let a = api::author(7, Role::Future, 1, &apibench::wait_scope(0, 1));
    let w2 = worker.clone();
    let gate = tokio::spawn(async move {
        sent.acquire().await.unwrap().forget();
        // Free only once the wait call has returned.
        drop(w2.client.lock().await);
        release.notify_one();
    });
    let c = clock();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let done = apibench::wait_once(&worker, (smtp, c), a, deadline).await;
    gate.await.unwrap();
    let after = c.now_ns();
    assert_eq!(done.outcome, Outcome::Exact);
    let t = done.wait.unwrap();
    let ack = t.ack_ns.expect("the delivery was acknowledged");
    assert!(t.call_ns <= t.send_ns, "{t:?}");
    assert!(t.return_ns < ack, "{t:?}");
    assert_eq!(t.ack_to_return_ns, Some(t.return_ns as i64 - ack as i64));
    assert!(t.ack_to_return_ns.unwrap() < 0);
    // QUIT ran to its own bound after the return and moved nothing.
    assert!(
        after - ack >= apibench::QUIT_BOUND.as_nanos() as u64,
        "{t:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_refused_or_ambiguous_delivery_is_classified_by_what_the_store_may_hold() {
    // A fixture the server refused is a known non-effect; one whose `250`
    // never came may have been kept, which is mutation uncertainty (R1).
    let refuse = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let raddr = refuse.local_addr().unwrap();
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        while let Ok((s, _)) = refuse.accept().await {
            tokio::spawn(async move {
                let (r, mut w) = s.into_split();
                let mut r = tokio::io::BufReader::new(r);
                let _ = w.write_all(b"220 x\r\n").await;
                loop {
                    let mut l = String::new();
                    if r.read_line(&mut l).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let reply: &[u8] = if l.starts_with("RCPT") {
                        b"550 no such user\r\n"
                    } else if l.starts_with("EHLO") {
                        b"250-x\r\n250 AUTH PLAIN\r\n"
                    } else if l.starts_with("AUTH") {
                        b"235 ok\r\n"
                    } else {
                        b"250 ok\r\n"
                    };
                    let _ = w.write_all(reply).await;
                }
            });
        }
    });
    let url = scripted::mcp(handler(|_, _, _| {
        Box::pin(async { Answer::Result(json!({"emails": []})) })
    }))
    .await;
    let worker = workers(&url, 1).await.pop().unwrap();
    let a = api::author(7, Role::Future, 2, &apibench::wait_scope(0, 2));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let done = apibench::wait_once(&worker, (raddr, clock()), a, deadline).await;
    assert_eq!(
        done.outcome,
        Outcome::Failed {
            reason: "fixture_refused".into()
        }
    );
    assert!(!api::uncertain(Operation::Wait, &done.outcome));
    assert!(api::uncertain(
        Operation::Wait,
        &Outcome::Failed {
            reason: "fixture_ambiguous".into()
        }
    ));
}
