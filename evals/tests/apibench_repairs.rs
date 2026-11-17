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

#[test]
fn api_mcp_forwards_the_verified_documents_root_to_the_adapter() {
    // R2: the adapter's command line carries the chunk's --documents, so the
    // draft lock serves; without one it falls back to the lock's directory.
    let http: std::net::SocketAddr = "127.0.0.1:4321".parse().unwrap();
    let args = |c: &tokio::process::Command| {
        c.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };
    let with = apibench::adapter_command(
        std::path::Path::new("/d/mail_compare"),
        "mailhog-mcp",
        std::path::Path::new("docs/evals/draft/sources.lock.v5.json"),
        Some(std::path::Path::new("docs/evals")),
        http,
    );
    assert_eq!(
        args(&with),
        [
            "mailhog-mcp",
            "--sources",
            "docs/evals/draft/sources.lock.v5.json",
            "--endpoint",
            "http://127.0.0.1:4321",
            "--documents",
            "docs/evals",
        ]
    );
    let without = apibench::adapter_command(
        std::path::Path::new("/d/mail_compare"),
        "mailpit-mcp",
        std::path::Path::new("docs/evals/sources.lock.json"),
        None,
        http,
    );
    assert!(!args(&without).iter().any(|a| a == "--documents"));
    // Every bridged target has an adapter mode; Swarmail is served directly.
    for t in ["mailpit", "mailhog", "mailslurper-source-build"] {
        assert!(apibench::adapter_mode(t).is_some(), "{t}");
    }
    assert_eq!(apibench::adapter_mode("swarmail"), None);
}

// ---- the holder's receipt and every teardown path (R4, R12) -----------------

fn receipt(target: &str, leaf: &str) -> Value {
    json!({
        "launch": {"target": target, "leaf": leaf},
        "cleanup": {"target": target, "leaf": leaf, "cgroup_removed": true,
            "reaped": true, "group_gone": true, "listeners_closed": true,
            "scratch_removed": true, "forced_kill": false},
    })
}

#[test]
fn only_a_matching_proved_receipt_is_accepted() {
    let good = receipt("mailpit", "leaf-1");
    apibench::validate_receipt(&good, "mailpit", Some("leaf-1")).unwrap();
    apibench::validate_receipt(&good, "mailpit", None).unwrap();
    let refuse =
        |v: &Value, leaf: Option<&str>| apibench::validate_receipt(v, "mailpit", leaf).unwrap_err();
    // Foreign: another target, another leaf, a cleanup of another launch.
    assert!(refuse(&receipt("mailhog", "leaf-1"), None).contains("not of"));
    assert!(refuse(&good, Some("leaf-2")).contains("another leaf"));
    let mut v = good.clone();
    v["cleanup"]["leaf"] = json!("leaf-9");
    assert!(refuse(&v, None).contains("another leaf"));
    // Missing parts, and a launch without a leaf.
    assert!(refuse(&json!({}), None).contains("no launch"));
    let mut v = good.clone();
    v["launch"]["leaf"] = Value::Null;
    assert!(refuse(&v, None).contains("no leaf"));
    // Each proof, false or absent.
    for proof in [
        "cgroup_removed",
        "reaped",
        "group_gone",
        "listeners_closed",
        "scratch_removed",
    ] {
        let mut v = good.clone();
        v["cleanup"][proof] = json!(false);
        assert!(refuse(&v, None).contains(proof));
        v["cleanup"].as_object_mut().unwrap().remove(proof);
        assert!(refuse(&v, None).contains(proof));
    }
}

#[test]
fn the_exit_is_the_holders_only_with_its_receipt_and_our_part_done() {
    use apibench::chunk_exit;
    assert_eq!(chunk_exit(Some(0), Ok(()), Ok(())).0, 0);
    assert_eq!(chunk_exit(Some(1), Ok(()), Ok(())).0, 1);
    // Infrastructure on either side, or no proof of cleanup: incomplete.
    assert_eq!(chunk_exit(Some(2), Ok(()), Ok(())).0, 2);
    assert_eq!(chunk_exit(None, Ok(()), Ok(())).0, 2);
    assert_eq!(chunk_exit(Some(0), Err("x".into()), Ok(())).0, 2);
    assert_eq!(chunk_exit(Some(0), Ok(()), Err("bridge".into())).0, 2);
    assert_eq!(chunk_exit(Some(1), Err("x".into()), Ok(())).0, 2);
}

/// A fake driver: `hold` runs `hold_body`, an adapter mode runs
/// `adapter_body`; both are bash. Every invocation appends its first
/// argument to `calls`.
fn fake_driver(dir: &std::path::Path, hold_body: &str, adapter_body: &str) -> std::path::PathBuf {
    let p = dir.join("driver");
    let calls = dir.join("calls");
    std::fs::write(
        &p,
        format!(
            "#!/bin/bash\necho \"$1\" >> '{}'\ncase \"$1\" in\nhold) {hold_body} ;;\n*) {adapter_body} ;;\nesac\n",
            calls.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

async fn run_api_mcp(dir: &std::path::Path, driver: &std::path::Path, target: &str) -> i32 {
    std::fs::write(dir.join("protocol.json"), r#"{"corpus":{"seed":1}}"#).unwrap();
    let args: Vec<String> = [
        "--driver",
        &driver.display().to_string(),
        "--mode",
        "smoke",
        "--protocol",
        &dir.join("protocol.json").display().to_string(),
        "--sources",
        "lock.json",
        "--documents",
        "docs",
        "--chunk",
        &format!("api.mcp.{target}.r0.f0"),
        "--bin",
        "swarmail=/x",
        "--output",
        &dir.join("out").display().to_string(),
        "--scratch",
        &dir.join("scratch").display().to_string(),
        "--run-id",
        "r",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    apibench::api_mcp(&args).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_target_that_never_came_up_is_its_own_failure_with_a_proved_receipt() {
    // The holder's first line is its receipt and it exits 1 (R4): the chunk
    // exits 1, as a native NotReady crash does.
    let d = tempfile::tempdir().unwrap();
    let r = receipt("swarmail", "leaf-1").to_string();
    let driver = fake_driver(d.path(), &format!("echo '{r}'; exit 1"), "exit 2");
    assert_eq!(run_api_mcp(d.path(), &driver, "swarmail").await, 1);
    // The same with a receipt that does not prove cgroup removal: exit 2.
    let mut bad = receipt("swarmail", "leaf-1");
    bad["cleanup"]["cgroup_removed"] = json!(false);
    let driver = fake_driver(d.path(), &format!("echo '{bad}'; exit 1"), "exit 2");
    assert_eq!(run_api_mcp(d.path(), &driver, "swarmail").await, 2);
    // A holder that refused before it announced anything: exit 2.
    let driver = fake_driver(d.path(), "exit 2", "exit 2");
    assert_eq!(run_api_mcp(d.path(), &driver, "swarmail").await, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_failure_after_the_holder_announced_closes_its_input_and_awaits_its_receipt() {
    // The holder announces an endpoint nothing listens on, then waits for
    // its input to close, records that it did, prints a proved receipt and
    // exits 0. The MCP session cannot start (Swarmail direct), or the
    // adapter dies at once (Mailpit through the bridge): either way the
    // chunk is incomplete (2), and only after the holder saw EOF and proved
    // its cleanup.
    for target in ["swarmail", "mailpit"] {
        let d = tempfile::tempdir().unwrap();
        let eof = d.path().join("eof");
        let r = receipt(target, "leaf-7").to_string();
        let hold = format!(
            "echo '{{\"target\":\"{target}\",\"leaf\":\"leaf-7\",\"smtp\":\"127.0.0.1:9\",\"http\":\"127.0.0.1:9\"}}'; cat >/dev/null; touch '{}'; echo '{r}'; exit 0",
            eof.display()
        );
        let driver = fake_driver(d.path(), &hold, "exit 3");
        assert_eq!(run_api_mcp(d.path(), &driver, target).await, 2, "{target}");
        assert!(
            eof.exists(),
            "{target}: the holder never saw its input close"
        );
        let calls = std::fs::read_to_string(d.path().join("calls")).unwrap();
        if target == "mailpit" {
            assert_eq!(calls, "hold\nmailpit-mcp\n");
        }
    }
}
