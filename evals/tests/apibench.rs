//! Protocol 4's API section on the MCP surface (accepted decision 0132 §6.2,
//! §7, story B), against the real candidate (`SWARMAIL_BIN`): benchmark
//! clients with disjoint JSON-RPC ids through one real bridge in front of a
//! real stdio MCP server, and every MCP cell, the coordinated future wait
//! included, over Swarmail's own `POST /mcp` with the exact checks and the
//! destructive postchecks. The held, enforced chunk (`hold --control
//! api-v1` under a delegated cgroup) is the host proof `api_mcp_bridge_real`
//! in the driver's suite.

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
    let fill = api::fill(s.smtp, &setup, (163, "mcp"), 1_000, clock).await;
    assert!(fill.reached, "{fill:?}");
    let mut fixtures = api::plan_fixtures(163, &cells);
    api::prepare(s.smtp, &setup, &mut fixtures).await.unwrap();
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
    let (recs, abandoned) = api::run_cells(&cells, &caps, &surface, &api::NoMeter, clock, None)
        .await
        .unwrap();
    assert_eq!(abandoned, None);
    // Swarmail's own MCP tool rate (1,024 calls a second) refuses part of
    // the 32-worker load: those are `rate_limited` failure rows, published,
    // never retried. A warmup it refuses blocks that cell. Nothing else may
    // fail, and no answer may be wrong.
    let limited =
        |r: &api::OpRow| matches!(&r.outcome, Outcome::Failed { reason } if reason == "status_429");
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
    // The limiter's window carries from one cell into the next, so only the
    // first cell is free of earlier load: it is exact throughout.
    let first = &recs[0];
    let st = api::stats(&first.measured, None);
    assert_eq!(
        (first.cell.as_str(), first.status, st.exact),
        ("list.c1", api::CellStatus::Measured, st.started)
    );
    assert!(
        recs.iter()
            .filter(|r| r.status == api::CellStatus::Measured)
            .count()
            >= 3
    );
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
