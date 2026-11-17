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
    // The later cell measures; how many fit in its five seconds is the
    // host's business.
    assert_eq!(recs[1].status, api::CellStatus::Measured);
    assert!(recs[1].measured.started > 0);
    let rows = &recs[1].measured.rows;
    assert!(
        rows.iter()
            .all(|r| matches!(r.outcome, Outcome::Exact | Outcome::Timeout))
    );
    assert!(rows.iter().any(|r| r.outcome == Outcome::Exact));
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
            "#!/bin/bash\n[ \"$1\" = probe ] && exit 0\necho \"$1\" >> '{}'\ncase \"$1\" in\nhold) {hold_body} ;;\n*) {adapter_body} ;;\nesac\n",
            calls.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    // A parallel test's fork may still hold the write descriptor of this
    // fresh file (ETXTBSY) until its child execs: run a probe until the
    // script executes, so the code under test never meets that race.
    for _ in 0..200 {
        match std::process::Command::new(&p).arg("probe").status() {
            Ok(s) if s.success() => return p,
            Err(e) if e.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(5))
            }
            other => panic!("the fake driver does not run: {other:?}"),
        }
    }
    panic!("the fake driver stayed busy")
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

// ---- refusals keep their source to the row (R8) ------------------------------

/// A stdio MCP server behind the real bridge: it holds every tool call until
/// the bridge's whole in-flight bound is waiting on it, refuses the first
/// one with the adapters' own busy error, and answers the rest with the read
/// set's page.
async fn holding_server(
    page: Value,
) -> (
    tokio::io::WriteHalf<tokio::io::DuplexStream>,
    tokio::io::BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let (bridge_side, server_side) = tokio::io::duplex(1 << 20);
    let (bridge_r, bridge_w) = tokio::io::split(bridge_side);
    let (server_r, mut server_w) = tokio::io::split(server_side);
    tokio::spawn(async move {
        let mut lines = tokio::io::BufReader::new(server_r).lines();
        let mut held: Vec<Value> = Vec::new();
        let mut released = false;
        let mut first = true;
        while let Ok(Some(line)) = lines.next_line().await {
            let req: Value = serde_json::from_str(&line).unwrap();
            let Some(id) = req.get("id").cloned() else {
                continue;
            };
            let reply = |id: Value, method: &str| -> Value {
                if method == "initialize" {
                    json!({"jsonrpc": "2.0", "id": id, "result": {"protocolVersion": "2025-03-26"}})
                } else {
                    json!({"jsonrpc": "2.0", "id": id, "result": {"isError": false,
                        "content": [{"type": "text", "text": page.to_string()}]}})
                }
            };
            let method = req["method"].as_str().unwrap_or_default().to_owned();
            let mut out = Vec::new();
            if method != "tools/call" || released {
                out.push(reply(id, &method));
            } else if first {
                first = false;
                out.push(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000,
                    "message": "too many tool calls in flight; retry later"}}));
            } else {
                held.push(id);
                if held.len() + 1 >= swarmail_evals::bridge::MAX_IN_FLIGHT {
                    released = true;
                    out.extend(held.drain(..).map(|id| reply(id, "tools/call")));
                }
            }
            for v in out {
                let mut b = v.to_string().into_bytes();
                b.push(b'\n');
                if server_w.write_all(&b).await.is_err() {
                    return;
                }
            }
        }
    });
    (bridge_w, tokio::io::BufReader::new(bridge_r))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn busy_refusals_reach_the_published_rows_by_source() {
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
    let (to, from) = holding_server(page).await;
    let bridge = swarmail_evals::bridge::Bridge::start(to, from)
        .await
        .unwrap();
    // Thirty-two workers, initialization included in their disjoint ranges.
    let surface = McpSurface {
        workers: Arc::new(workers(&bridge.url(), 32).await),
        verify: clients::Client::new("swarmail", "127.0.0.1:9".parse().unwrap()).unwrap(),
        fixtures: Arc::new(reads),
        smtp: "127.0.0.1:9".parse().unwrap(),
        seed: 7,
        clock: clock(),
    };
    let caps = |op| clients::mcp_capability("swarmail", op);
    let mut c32 = cell(Operation::List, 32);
    c32.warmup = 0;
    let (recs, abandoned) = api::run_cells(&[c32], &caps, &surface, &api::NoMeter, clock(), None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    let rows = &recs[0].measured.rows;
    let reason = |r: &api::OpRow| match &r.outcome {
        Outcome::Failed { reason } => reason.clone(),
        other => other.class().to_owned(),
    };
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    for r in rows {
        *seen.entry(reason(r)).or_default() += 1;
    }
    // The adapter's queue and the bridge's bound both refused, each named;
    // nothing else failed, nothing was retried (one row per start).
    assert_eq!(seen.get("busy_adapter"), Some(&1), "{seen:?}");
    assert!(
        seen.get("busy_bridge").copied().unwrap_or(0) >= 1,
        "{seen:?}"
    );
    assert!(seen.get("exact").copied().unwrap_or(0) >= 1, "{seen:?}");
    assert_eq!(
        seen.keys().cloned().collect::<Vec<_>>(),
        ["busy_adapter", "busy_bridge", "exact"],
        "{seen:?}"
    );
    assert_eq!(rows.len() as u32, recs[0].measured.started);
    // The serialized row carries the source, as the report publishes it.
    let busy = rows.iter().find(|r| reason(r) == "busy_bridge").unwrap();
    let v = serde_json::to_value(busy).unwrap();
    assert_eq!(
        (v["outcome"].as_str(), v["reason"].as_str()),
        (Some("failed"), Some("busy_bridge"))
    );
    // Completed refusals are known non-effects, never mutation uncertainty.
    for r in ["busy_bridge", "busy_adapter", "rate_limited"] {
        assert!(!api::uncertain(
            Operation::Delete,
            &Outcome::Failed { reason: r.into() }
        ));
    }
    bridge.stop().await;
}

// ---- a fill not reached, through the MCP producer (R16) ----------------------

/// Swarmail's HTTP shape, just enough: `POST /mcp` initializes, and the
/// default inbox lists `kept` distinct ids, paged.
async fn short_store(kept: u64) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = Vec::new();
                loop {
                    let mut chunk = [0u8; 8192];
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
                                    .map(|v| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if buf.len() < end + 4 + len {
                            break;
                        }
                        let body: Vec<u8> = buf[end + 4..end + 4 + len].to_vec();
                        buf.drain(..end + 4 + len);
                        let target = head.split(' ').nth(1).unwrap_or("/").to_owned();
                        let (status, reply) = if target == "/mcp" {
                            let req: Value = serde_json::from_slice(&body).unwrap();
                            match req.get("id") {
                                None => (202, String::new()),
                                Some(id) => (
                                    200,
                                    json!({"jsonrpc": "2.0", "id": id,
                                    "result": {"protocolVersion": "2025-03-26"}})
                                    .to_string(),
                                ),
                            }
                        } else {
                            let q = |k: &str| -> u64 {
                                target
                                    .split(&format!("{k}="))
                                    .nth(1)
                                    .and_then(|v| v.split('&').next())
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(0)
                            };
                            let (offset, limit) = (q("offset"), q("limit"));
                            let emails: Vec<Value> = (offset..kept.min(offset + limit))
                                .map(|i| json!({"id": format!("m{i}"), "to": [{"address": "x@y"}], "subject": "s"}))
                                .collect();
                            (200, json!({"total": kept, "emails": emails}).to_string())
                        };
                        let resp = format!(
                            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{reply}",
                            reply.len()
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fill_the_store_did_not_keep_blocks_every_mcp_cell_and_exits_one() {
    // The holder announces a store that keeps 99 of 100 accepted messages.
    // api-mcp fills, finds the disagreement, blocks every planned cell as
    // fill_not_reached, sends that record, closes its input and takes the
    // holder's exit 1 with a proved receipt. Nothing is retried.
    let d = tempfile::tempdir().unwrap();
    let smtp = scripted::smtp(SmtpScript {
        hold_data_ack: None,
        data_received: None,
        quit_delay: Duration::ZERO,
    })
    .await;
    let http = short_store(99).await;
    let lines = d.path().join("lines");
    let r = receipt("swarmail", "leaf-3").to_string();
    let hold = format!(
        "echo '{{\"target\":\"swarmail\",\"leaf\":\"leaf-3\",\"smtp\":\"{smtp}\",\"http\":\"{http}\"}}'; \
         while IFS= read -r l; do printf '%s\\n' \"$l\" >> '{}'; echo '{{\"ok\":\"x\",\"t_ns\":1}}'; done; \
         echo '{r}'; exit 1",
        lines.display()
    );
    let driver = fake_driver(d.path(), &hold, "exit 3");
    std::fs::write(d.path().join("protocol.json"), r#"{"corpus":{"seed":1}}"#).unwrap();
    let args: Vec<String> = [
        "--driver",
        &driver.display().to_string(),
        "--mode",
        "smoke",
        "--protocol",
        &d.path().join("protocol.json").display().to_string(),
        "--sources",
        "lock.json",
        "--chunk",
        "api.mcp.swarmail.r0.f100",
        "--bin",
        "swarmail=/x",
        "--output",
        &d.path().join("out").display().to_string(),
        "--scratch",
        &d.path().join("scratch").display().to_string(),
        "--run-id",
        "r",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(apibench::api_mcp(&args).await, 1);
    let sent = std::fs::read_to_string(&lines).unwrap();
    let cmds: Vec<Value> = sent
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // No idle point and no window: the record was the only command.
    assert_eq!(cmds.len(), 1, "{cmds:?}");
    assert_eq!(cmds[0]["cmd"], "record");
    let rec: api::ApiRecord = serde_json::from_value(cmds[0]["api"].clone()).unwrap();
    assert!(!rec.fill.reached);
    assert_eq!(
        (rec.fill.fill, rec.fill.accepted, rec.fill.retained),
        (100, 100, Some(99))
    );
    let (cells, _) = api::plan_cells(clients::Surface::Mcp, |op| {
        clients::mcp_capability("swarmail", op)
    });
    assert_eq!(rec.cells.len(), cells.len());
    for c in &rec.cells {
        assert_eq!(c.status, api::CellStatus::Blocked, "{}", c.cell);
        assert_eq!(c.blocked.as_deref(), Some("fill_not_reached"));
        assert_eq!(
            (c.measured.started, c.measured.unstarted),
            (0, c.measured.planned)
        );
    }
    assert!(rec.control.is_empty());
    api::check_brackets(&rec).unwrap();
}
