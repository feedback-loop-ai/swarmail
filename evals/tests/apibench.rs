//! Protocol 4's API section on the MCP surface (accepted decision 0132 §6.2,
//! §7, story B), against the real candidate (`SWARMAIL_BIN`): benchmark
//! clients with disjoint JSON-RPC ids through one real bridge in front of a
//! real stdio MCP server, and every MCP cell, the coordinated future wait
//! included, over Swarmail's own `POST /mcp` with the exact checks and the
//! destructive postchecks. The held, enforced chunks (`hold --control
//! api-v1` under a delegated cgroup) are the host proofs
//! `api_mcp_real_swarmail` and `api_mcp_bridge_real_*` in the driver's suite.

mod common;

use serde_json::json;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use swarmail_evals::api::{self, Outcome};
use swarmail_evals::apibench::{self, McpSurface, Worker};
use swarmail_evals::bridge::Bridge;
use swarmail_evals::clients::{self, Surface};
use swarmail_evals::mcp::McpClient;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

struct Server {
    _child: Child,
    smtp: SocketAddr,
    http: SocketAddr,
    _dir: tempfile::TempDir,
}

async fn serve() -> Server {
    let dir = common::tempdir();
    let mut child = Command::new(common::swarmail_bin())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:0",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:0",
            "--max-per-inbox",
            "150000",
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
        let n = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            lines.read_line(&mut line),
        )
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
    Server {
        _child: child,
        smtp,
        http,
        _dir: dir,
    }
}

#[test]
fn id_ranges_are_disjoint_and_never_zero() {
    let r: Vec<_> = (0..32).map(apibench::id_range).collect();
    for (i, a) in r.iter().enumerate() {
        assert!(a.start >= 1 && !a.is_empty());
        for b in &r[i + 1..] {
            assert!(a.end <= b.start, "{a:?} overlaps {b:?}");
        }
    }
    assert!(apibench::parse_chunk("api.mcp.mailpit.r0.f1000").is_ok());
    for bad in [
        "api.mcp.inbucket.r0.f0",
        "api.native.mailpit.r0.f0",
        "api.mcp.mailpit.r00.f0",
        "x",
    ] {
        assert!(apibench::parse_chunk(bad).is_err(), "{bad}");
    }
    assert!(
        matches!(apibench::classify("x: JSON-RPC error {\"message\":\"busy\"}", false), Outcome::Failed { reason } if reason == "busy_adapter")
    );
    assert!(matches!(
        apibench::classify("POST: operation timed out", false),
        Outcome::Timeout
    ));
    assert!(
        matches!(apibench::classify("x: this client's id range is exhausted", false), Outcome::Failed { reason } if reason == "id_range_exhausted")
    );
    let f = apibench::flags(&["--driver".into(), "d".into()]).unwrap_err();
    assert!(f.contains("--mode is required"), "{f}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn thirty_two_clients_share_one_bridge_by_disjoint_ids() {
    let s = serve().await;
    let mut stdio = Command::new(common::swarmail_bin())
        .args(["mcp", "--url", &format!("http://{}", s.http)])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("swarmail mcp starts");
    let (to, from) = (stdio.stdin.take().unwrap(), stdio.stdout.take().unwrap());
    let bridge = Bridge::start(to, BufReader::new(from)).await.unwrap();
    let url = bridge.url();
    // Initialization included, every id is the client's own.
    let mut clients = Vec::new();
    for n in 0..32 {
        clients.push(
            McpClient::connect_range(&url, apibench::id_range(n))
                .await
                .expect("initialize"),
        );
    }
    let calls = clients.into_iter().map(|mut c| {
        tokio::spawn(async move {
            let mut out = Vec::new();
            for _ in 0..4 {
                out.push(
                    c.call("swarmail_search_emails", json!({"inbox": "nobody"}))
                        .await,
                );
            }
            out
        })
    });
    let mut ok = 0;
    for h in calls {
        for r in h.await.unwrap() {
            match r {
                Ok(v) => {
                    assert_eq!(v["total"], 0);
                    ok += 1;
                }
                // The bridge's in-flight bound may refuse: a busy row by
                // source, never a misrouted reply.
                Err(e) => assert!(
                    !e.contains("is not"),
                    "a reply reached the wrong client: {e}"
                ),
            }
        }
    }
    assert!(ok >= 16, "{ok} of 128 calls answered");
    // A range of two ids: initialize and one call, then refusal.
    let mut tiny = McpClient::connect_range(&url, 7..9).await.unwrap();
    tiny.rpc("tools/list", json!({})).await.unwrap();
    let w = tiny.rpc("tools/list", json!({})).await.unwrap_err();
    assert!(w.contains("exhausted"), "{w}");
    assert!(McpClient::connect_range(&url, 0..5).await.is_err());
    bridge.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_mcp_cell_of_the_candidate_is_exact() {
    let s = serve().await;
    let clock = api::Clock(std::time::Instant::now());
    let caps = |op| clients::mcp_capability("swarmail", op);
    let (cells, rows) = api::plan_cells(Surface::Mcp, caps);
    assert!(rows.is_empty());
    let setup = clients::Client::new("swarmail", s.http).unwrap();
    let (fill, background) = api::fill_ledger(s.smtp, &setup, (163, "mcp"), 1_000, clock).await;
    assert!(fill.reached, "{fill:?}");
    let background = background.unwrap();
    let mut fixtures = api::plan_fixtures(163, &cells);
    api::prepare(s.smtp, &setup, &mut fixtures, None)
        .await
        .unwrap();
    let url = format!("http://{}/mcp", s.http);
    let mut pool = Vec::new();
    for n in 0..32 {
        let c = McpClient::connect_range(&url, apibench::id_range(n))
            .await
            .unwrap();
        pool.push(Worker {
            client: Mutex::new(c),
            target: "swarmail".into(),
        });
    }
    let verify = clients::Client::new("swarmail", s.http).unwrap();
    let surface = McpSurface {
        workers: Arc::new(pool),
        verify,
        fixtures: Arc::new(fixtures),
        smtp: s.smtp,
        seed: 163,
        clock,
    };
    let (mut recs, abandoned) = api::run_cells(&cells, &caps, &surface, &api::NoMeter, clock, None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    // The complete batch ledger (R5), read untimed through the native API —
    // the same check the native surface runs — holds the whole store to the
    // authored fixtures, the background and every acknowledged wait.
    let problems = api::final_ledger(
        &surface.verify,
        (163, &background),
        &surface.fixtures,
        &cells,
        &mut recs,
        None,
    )
    .await;
    assert!(problems.is_empty(), "{problems:?}");
    // Swarmail's own MCP tool rate (1,024 calls a second) would refuse with
    // `rate_limited` failure rows, published, never retried, and a warmup it
    // refused would block that cell. Nothing else may fail, and no answer
    // may be wrong.
    let limited = |r: &api::OpRow| matches!(&r.outcome, Outcome::Failed { reason } if reason == "rate_limited");
    for r in &recs {
        let warm = r.warmup.iter().flat_map(|w| w.rows.iter());
        assert!(
            warm.clone()
                .all(|x| x.outcome == Outcome::Exact || limited(x)),
            "{}",
            r.cell
        );
        if r.status == api::CellStatus::Blocked {
            assert_eq!(r.blocked.as_deref(), Some("warmup_failed"), "{}", r.cell);
            assert!(warm.clone().any(limited), "{}", r.cell);
            continue;
        }
        assert!(r.postcheck.is_empty(), "{}: {:?}", r.cell, r.postcheck);
        assert!(r.measured.started > 0, "{}", r.cell);
        assert!(
            r.measured
                .rows
                .iter()
                .all(|x| x.outcome == Outcome::Exact || limited(x)),
            "{}: {:?}",
            r.cell,
            r.measured
                .rows
                .iter()
                .find(|x| x.outcome != Outcome::Exact && !limited(x))
        );
    }
    // The MCP surface idles a second before each cell (operator ruling
    // 2026-11-18), so every cell starts in a fresh window, and no cell's
    // 120 calls reach the limit: every cell is measured and exact.
    for r in &recs {
        let st = api::stats(&r.measured, None);
        assert_eq!(
            (r.status, st.exact),
            (api::CellStatus::Measured, st.started),
            "{}",
            r.cell
        );
        assert!(
            r.warmup
                .iter()
                .flat_map(|w| w.rows.iter())
                .all(|x| !limited(x)),
            "{}",
            r.cell
        );
    }
    // Every wait row carries its instants on one clock, the return after the
    // call, and a signed offset from the 250.
    let waits = recs
        .iter()
        .filter(|r| r.operation == clients::Operation::Wait);
    let mut seen = BTreeSet::new();
    for r in waits {
        for row in &r.measured.rows {
            let w = row.wait.as_ref().expect("a wait row has its instants");
            assert!(w.return_ns >= w.call_ns && w.send_ns >= w.call_ns);
            let ack = w.ack_ns.expect("acknowledged");
            assert_eq!(w.ack_to_return_ns, Some(w.return_ns as i64 - ack as i64));
            assert!(seen.insert((r.cell.clone(), row.index)));
        }
    }
    // Later losses the batch did not cause — one survivor, one acknowledged
    // wait delivery: the ledger names each, and every destructive or wait
    // success becomes a wrong result. (Extra deliveries are proved in the
    // driver suite's ledger tests.)
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut spend = clients::Spend::default();
    let survivor = surface
        .fixtures
        .iter()
        .find(|f| f.role == api::Role::Survivor)
        .and_then(|f| f.id.clone())
        .unwrap();
    clients::swarmail::delete(&setup.conn, &survivor, deadline, &mut spend)
        .await
        .unwrap();
    let want = api::expected_ledger(163, &background, &surface.fixtures, &cells, &recs);
    let acked = want.waits.iter().find(|w| w.2).unwrap().0.clone();
    let gone = setup.list(&acked, deadline, &mut spend).await.unwrap();
    clients::swarmail::delete(&setup.conn, &gone[0].id, deadline, &mut spend)
        .await
        .unwrap();
    let problems = api::final_ledger(
        &surface.verify,
        (163, &background),
        &surface.fixtures,
        &cells,
        &mut recs,
        None,
    )
    .await;
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems.iter().any(|p| p.contains("missing")),
        "{problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains(&acked.address)),
        "{problems:?}"
    );
    for r in recs
        .iter()
        .filter(|r| r.operation.destructive() || r.operation == clients::Operation::Wait)
    {
        assert!(
            r.measured.rows.iter().all(|x| x.outcome != Outcome::Exact),
            "{}",
            r.cell
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_future_wait_returns_exactly_its_own_message_with_its_instants() {
    let s = serve().await;
    let clock = api::Clock(std::time::Instant::now());
    let caps = |op| clients::mcp_capability("swarmail", op);
    let (cells, _) = api::plan_cells(Surface::Mcp, caps);
    let wait: Vec<api::PlannedCell> = cells
        .into_iter()
        .filter(|c| c.operation == clients::Operation::Wait && c.concurrency == Some(8))
        .collect();
    let url = format!("http://{}/mcp", s.http);
    let mut pool = Vec::new();
    for n in 0..8 {
        let c = McpClient::connect_range(&url, apibench::id_range(n))
            .await
            .unwrap();
        pool.push(Worker {
            client: Mutex::new(c),
            target: "swarmail".into(),
        });
    }
    let verify = clients::Client::new("swarmail", s.http).unwrap();
    // No fixture exists before its own attempt.
    let surface = McpSurface {
        workers: Arc::new(pool),
        verify,
        fixtures: Arc::new(Vec::new()),
        smtp: s.smtp,
        seed: 163,
        clock,
    };
    let (recs, abandoned) = api::run_cells(&wait, &caps, &surface, &api::NoMeter, clock, None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    let r = &recs[0];
    assert_eq!(r.status, api::CellStatus::Measured, "{:?}", r.blocked);
    assert!(r.measured.started > 0);
    let mut scopes = BTreeSet::new();
    for row in r
        .measured
        .rows
        .iter()
        .chain(r.warmup.iter().flat_map(|w| w.rows.iter()))
    {
        assert_eq!(row.outcome, Outcome::Exact, "{row:?}");
        let w = row.wait.as_ref().expect("a wait row has its instants");
        assert!(w.return_ns >= w.call_ns && w.send_ns >= w.call_ns);
        let ack = w.ack_ns.expect("acknowledged");
        assert_eq!(w.ack_to_return_ns, Some(w.return_ns as i64 - ack as i64));
        assert!(scopes.insert((row.phase == api::Phase::Warmup, row.index)));
    }
}

// ---- the chunk deadline on the MCP surface (AC5, R1) -------------------------

/// A fake driver whose `hold` is `hold_body` (bash).
fn fake_holder(dir: &std::path::Path, hold_body: &str) -> std::path::PathBuf {
    let p = dir.join("driver");
    std::fs::write(
        &p,
        format!("#!/bin/bash\n[ \"$1\" = probe ] && exit 0\ncase \"$1\" in\nhold) {hold_body} ;;\n*) exit 3 ;;\nesac\n"),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    // A parallel test's fork may hold this fresh file's write descriptor
    // (ETXTBSY) until its child execs: probe until the script runs.
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

fn chunk_args(dir: &std::path::Path, driver: &std::path::Path, protocol: &str) -> Vec<String> {
    std::fs::write(dir.join("protocol.json"), protocol).unwrap();
    [
        "--driver",
        &driver.display().to_string(),
        "--mode",
        "smoke",
        "--protocol",
        &dir.join("protocol.json").display().to_string(),
        "--sources",
        "lock.json",
        "--chunk",
        "api.mcp.swarmail.r0.f0",
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
    .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_chunk_deadline_blocks_mcp_cells_and_the_record_still_reaches_the_holder() {
    // A chunk deadline that leaves nothing for cells once the record margin
    // and the ledger reserve are kept: the real candidate is filled, the
    // fixture preparation is cut at the cells' end (it can no longer serve a
    // cell), every planned cell is a `chunk_deadline` row, and the record
    // reaches the holder before its own input deadline instead of being
    // torn down unsealed.
    let s = serve().await;
    let d = common::tempdir();
    let lines = d.path().join("lines");
    let receipt = json!({
        "launch": {"target": "swarmail", "leaf": "leaf-9"},
        "cleanup": {"target": "swarmail", "leaf": "leaf-9", "cgroup_removed": true,
            "reaped": true, "group_gone": true, "listeners_closed": true,
            "scratch_removed": true, "forced_kill": false},
    });
    let hold = format!(
        "echo '{{\"target\":\"swarmail\",\"leaf\":\"leaf-9\",\"smtp\":\"{}\",\"http\":\"{}\"}}'; \
         while IFS= read -r l; do printf '%s\\n' \"$l\" >> '{}'; echo '{{\"ok\":\"x\",\"t_ns\":1}}'; done; \
         echo '{receipt}'; exit 0",
        s.smtp,
        s.http,
        lines.display()
    );
    let driver = fake_holder(d.path(), &hold);
    let ms = (apibench::RECORD_MARGIN + api::LEDGER_RESERVE).as_millis();
    let protocol = format!(r#"{{"corpus":{{"seed":5}},"resources":{{"chunk_deadline_ms":{ms}}}}}"#);
    let started = std::time::Instant::now();
    let code = apibench::api_mcp(&chunk_args(d.path(), &driver, &protocol)).await;
    assert_eq!(code, 0);
    assert!(started.elapsed() < std::time::Duration::from_millis(ms as u64));
    let cmds: Vec<serde_json::Value> = std::fs::read_to_string(&lines)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // The idle point, then the record: no window ever opened.
    let names: Vec<&str> = cmds.iter().map(|c| c["cmd"].as_str().unwrap()).collect();
    assert_eq!(names, ["idle_fill", "record"]);
    let rec: api::ApiRecord = serde_json::from_value(cmds[1]["api"].clone()).unwrap();
    assert!(rec.fill.reached);
    assert_eq!(rec.abandoned, None);
    assert!(rec.ledger.is_empty(), "{:?}", rec.ledger);
    let (cells, _) = api::plan_cells(Surface::Mcp, |op| clients::mcp_capability("swarmail", op));
    assert_eq!(rec.cells.len(), cells.len());
    for c in &rec.cells {
        assert_eq!(c.status, api::CellStatus::Blocked, "{}", c.cell);
        assert_eq!(c.blocked.as_deref(), Some("chunk_deadline"), "{}", c.cell);
        assert_eq!(
            (c.measured.started, c.measured.unstarted),
            (0, c.measured.planned)
        );
    }
    api::check_brackets(&rec).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_mcp_chunk_without_its_deadline_refuses_before_the_holder_starts() {
    let d = common::tempdir();
    let ran = d.path().join("ran");
    let driver = fake_holder(d.path(), &format!("touch '{}'; exit 0", ran.display()));
    for protocol in [
        r#"{"corpus":{"seed":5}}"#,
        r#"{"corpus":{"seed":5},"resources":{"chunk_deadline_ms":0}}"#,
        r#"{"corpus":{"seed":5},"resources":{"chunk_deadline_ms":"480000"}}"#,
    ] {
        assert_eq!(
            apibench::api_mcp(&chunk_args(d.path(), &driver, protocol)).await,
            2,
            "{protocol}"
        );
        assert!(!ran.exists(), "{protocol}: the holder was spawned");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_mcp_chunk_over_the_cap_refuses_before_the_holder_starts() {
    // The capacity preflight runs before any launch, as natively.
    let d = common::tempdir();
    let ran = d.path().join("ran");
    let driver = fake_holder(d.path(), &format!("touch '{}'; exit 0", ran.display()));
    let protocol = r#"{"corpus":{"seed":5},"resources":{"chunk_deadline_ms":480000}}"#;
    let mut args = chunk_args(d.path(), &driver, protocol);
    let at = args.iter().position(|a| a == "--chunk").unwrap() + 1;
    args[at] = format!("api.mcp.swarmail.r0.f{}", api::CAP);
    assert_eq!(apibench::api_mcp(&args).await, 2);
    assert!(!ran.exists(), "the holder was spawned");
}

#[test]
fn the_mcp_chunk_end_keeps_the_record_margin() {
    let t = std::time::Instant::now();
    let p = |ms: u64| json!({"resources": {"chunk_deadline_ms": ms}});
    let end = apibench::mcp_chunk_end(t, &p(480_000)).unwrap();
    assert_eq!(end.into_std() - t, std::time::Duration::from_secs(470));
    // A deadline shorter than the margin ends at the start, never before it.
    assert_eq!(apibench::mcp_chunk_end(t, &p(1)).unwrap().into_std(), t);
    // The cells, ledger and clear divide what is left (api::stops): on this
    // surface the head keeps the ledger's reserve before that end.
    let end = apibench::mcp_chunk_end(t, &p(25_000)).unwrap();
    assert_eq!(end.into_std() - t, std::time::Duration::from_secs(15));
    let stops = api::stops(tokio::time::Instant::from_std(t), end, false);
    assert_eq!((stops.head.cells.into_std(), stops.ledger), (t, end));
}
