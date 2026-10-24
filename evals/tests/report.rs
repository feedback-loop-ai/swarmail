//! The matrix and report commands (decision 0055 §§9–10): product
//! cardinality, a clean stdout, accounting that survives harness errors,
//! aggregated exits, and the report's estimates on fixed inputs.

mod common;

use common::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::Stdio;
use swarmail_evals::runner::TrialResult;
use swarmail_evals::tasks::Outcome;

#[test]
fn helper_agent() {
    maybe_run_helper();
}

fn rows(stdout: &[u8]) -> Vec<TrialResult> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(|l| {
            let r: TrialResult = serde_json::from_str(l).unwrap_or_else(|e| panic!("{l:?}: {e}"));
            r.validate().unwrap();
            r
        })
        .collect()
}

fn agents_file(dir: &std::path::Path, agents: Value) -> String {
    let path = dir.join("agents.json");
    std::fs::write(&path, agents.to_string()).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn the_matrix_runs_the_whole_product_and_aggregates_exits() {
    let dir = tempfile::tempdir().unwrap();
    let agents = agents_file(
        dir.path(),
        json!([{"label": "oracle", "argv": []}, {"label": "null", "argv": []}]),
    );
    let out = cli()
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--agents",
            &agents,
            "--trials",
            "2",
        ])
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    assert_eq!(rows.len(), 8, "2 tasks × 2 agents × 2 trials");
    assert_eq!(
        out.status.code(),
        Some(1),
        "fails present, no harness error"
    );
    for task in ["signup", "otp"] {
        for (agent, outcome) in [("oracle", Outcome::Pass), ("null", Outcome::Fail)] {
            let n = rows
                .iter()
                .filter(|r| {
                    r.task.as_deref() == Some(task)
                        && r.agent.as_deref() == Some(agent)
                        && r.outcome == outcome
                })
                .count();
            assert_eq!(n, 2, "{task} {agent}");
        }
    }
    let ids: std::collections::HashSet<_> = rows.iter().map(|r| &r.trial_id).collect();
    assert_eq!(ids.len(), 8);

    // The rows feed the report directly.
    let mut report = cli()
        .args(["report", "--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    report.stdin.take().unwrap().write_all(&out.stdout).unwrap();
    let rep = report.wait_with_output().unwrap();
    assert!(rep.status.success());
    let v: Value = serde_json::from_slice(&rep.stdout).unwrap();
    let groups = v["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 4);
    for g in groups {
        let rate = if g["agent"] == "oracle" { 1.0 } else { 0.0 };
        assert_eq!(g["success_rate"], json!(rate), "{g}");
        assert_eq!(g["attempts"], 2);
    }
}

#[test]
fn the_matrix_keeps_counting_after_a_harness_error_and_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    let agents = agents_file(
        dir.path(),
        json!([
            {"label": "ghost", "argv": ["/nonexistent/agent"]},
            {"label": "null", "argv": []},
            {"label": "oracle", "argv": []}
        ]),
    );
    let out = cli()
        .args([
            "matrix", "--tasks", "signup", "--agents", &agents, "--trials", "1",
        ])
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].outcome, Outcome::HarnessError);
    assert_eq!(rows[1].outcome, Outcome::Fail);
    assert_eq!(rows[2].outcome, Outcome::Pass, "the matrix went on");
    assert_eq!(
        out.status.code(),
        Some(2),
        "harness errors outrank failures"
    );
}

/// An unresolvable Swarmail binary still owes one harness-error row per
/// product element, in product order, with no trial child launched.
#[test]
fn a_missing_swarmail_binary_still_emits_the_whole_product() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("launched");
    let touch = vec!["/bin/sh", "-c", "touch \"$0\"", marker.to_str().unwrap()];
    let agents = agents_file(
        dir.path(),
        json!([{"label": "first", "argv": touch}, {"label": "second", "argv": touch}]),
    );
    let missing = dir.path().join("no-such-swarmail");
    let out = cli()
        .env("SWARMAIL_BIN", &missing)
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--agents",
            &agents,
            "--trials",
            "2",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    let rows = rows(&out.stdout);
    let got: Vec<(Option<&str>, Option<&str>)> = rows
        .iter()
        .map(|r| (r.task.as_deref(), r.agent.as_deref()))
        .collect();
    let mut want = Vec::new();
    for task in ["signup", "otp"] {
        for agent in ["first", "second"] {
            want.extend([(Some(task), Some(agent)); 2]);
        }
    }
    assert_eq!(got, want);
    for r in &rows {
        assert_eq!(r.outcome, Outcome::HarnessError);
        assert_eq!((r.agent_exit_status, r.agent_exits.len()), (None, 0));
    }
    let ids: std::collections::HashSet<&str> = rows.iter().map(|r| r.trial_id.as_str()).collect();
    assert_eq!(ids.len(), 8, "distinct trial ids");
    assert_eq!(
        stderr(&out).matches("no-such-swarmail").count(),
        1,
        "{}",
        stderr(&out)
    );
    assert!(!marker.exists(), "no trial child ran");
}

#[test]
fn the_matrix_refuses_bad_inputs_before_running_anything() {
    let dir = tempfile::tempdir().unwrap();
    let good = agents_file(dir.path(), json!([{"label": "null", "argv": []}]));
    for args in [
        vec![
            "--tasks",
            "signup",
            "--agents",
            good.as_str(),
            "--trials",
            "0",
        ],
        vec![
            "--tasks",
            "nope",
            "--agents",
            good.as_str(),
            "--trials",
            "1",
        ],
        vec![
            "--tasks",
            "signup",
            "--agents",
            "/nonexistent.json",
            "--trials",
            "1",
        ],
    ] {
        let out = cli().arg("matrix").args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

fn row(task: &str, outcome: &str, class: Option<&str>, ms: u64) -> String {
    json!({
        "schema_version": 1, "trial_id": format!("t{ms}"), "task": task, "agent": "a",
        "outcome": outcome, "failure_class": class, "wall_time_ms": ms,
        "agent_exit_status": 0, "agent_exits": [], "confinement": "unconfined"
    })
    .to_string()
}

fn report_of(input: &str) -> (Option<i32>, Value) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rows.jsonl");
    std::fs::write(&path, input).unwrap();
    let out = cli()
        .args(["report", "--input", path.to_str().unwrap()])
        .output()
        .unwrap();
    (
        out.status.code(),
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
    )
}

#[test]
fn the_report_separates_harness_errors_from_the_success_rate() {
    let mut lines = Vec::new();
    for ms in 1..=8 {
        lines.push(row("t", "pass", None, ms * 100));
    }
    lines.push(row("t", "fail", Some("wrong_link"), 900));
    lines.push(row("t", "fail", Some("gave_up"), 1000));
    lines.push(row("t", "harness_error", Some("harness_error"), 5));
    let (code, v) = report_of(&lines.join("\n"));
    assert_eq!(code, Some(0));
    let g = &v["groups"][0];
    assert_eq!(
        (
            g["attempts"].as_u64(),
            g["passes"].as_u64(),
            g["fails"].as_u64(),
            g["harness_errors"].as_u64()
        ),
        (Some(11), Some(8), Some(2), Some(1))
    );
    assert_eq!(g["success_rate"], json!(0.8));
    let ci = g["success_ci95"].as_array().unwrap();
    assert!((ci[0].as_f64().unwrap() - 0.4901625).abs() < 1e-6, "{ci:?}");
    assert!((ci[1].as_f64().unwrap() - 0.9433178).abs() < 1e-6, "{ci:?}");
    assert_eq!(g["passes_over_attempts"], json!(8.0 / 11.0));
    assert_eq!(
        g["wall_time_ms"],
        json!({"n": 10, "median": 500, "p95": 1000})
    );
    assert_eq!(g["failure_classes"]["wrong_link"], 1);
    assert_eq!(g["failure_classes"]["harness_error"], 1);
    assert_eq!(g["failure_classes"]["budget"], 0);
}

#[test]
fn empty_and_all_error_inputs_give_null_estimates_and_bad_rows_refuse() {
    let (code, v) = report_of("");
    assert_eq!((code, v), (Some(0), json!({"groups": []})));
    let (code, v) = report_of(&row("t", "harness_error", Some("harness_error"), 3));
    assert_eq!(code, Some(0));
    let g = &v["groups"][0];
    assert_eq!(g["success_rate"], Value::Null);
    assert_eq!(g["success_ci95"], Value::Null);
    assert_eq!(g["wall_time_ms"]["median"], Value::Null);
    assert_eq!(g["harness_error_rate"], json!(1.0));
    for bad in [
        "{not json".to_string(),
        row("t", "pass", Some("gave_up"), 1),
        row("t", "fail", None, 1),
        row("t", "fail", Some("asleep"), 1),
        row("t", "pass", None, 1).replace("unconfined", "sandboxed"),
    ] {
        let (code, _) = report_of(&bad);
        assert_eq!(code, Some(2), "{bad}");
    }
}

struct MatrixGuard {
    child: Option<std::process::Child>,
    owner: ProcessIdentity,
    release: std::path::PathBuf,
    owned: Vec<ProcessIdentity>,
}

impl MatrixGuard {
    fn pid(&self) -> i32 {
        self.child.as_ref().unwrap().id() as i32
    }

    fn record_identity(&mut self, identity: ProcessIdentity) {
        if !self.owned.iter().any(|known| {
            known.pid == identity.pid
                && known.start_time == identity.start_time
                && known.pgrp == identity.pgrp
                && known.session == identity.session
        }) {
            self.owned.push(identity);
        }
    }

    fn record_descendants(&mut self) {
        let pid = self.pid();
        if !process_identity(pid).is_some_and(|now| {
            now.start_time == self.owner.start_time
                && now.pgrp == self.owner.pgrp
                && now.session == self.owner.session
        }) {
            return;
        }
        for process in process_descendants(pid) {
            self.record_identity(process);
        }
    }

    fn wait_for(&mut self, limit: std::time::Duration) -> Option<std::process::ExitStatus> {
        let deadline = std::time::Instant::now() + limit;
        loop {
            match self.child.as_mut()?.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => {}
                Err(_) => return None,
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        self.wait_for(std::time::Duration::from_secs(30))
            .unwrap_or_else(|| panic!("matrix child exceeded its 30-second wait deadline"))
    }

    fn signal_owned_groups(&self, signal: i32) {
        let groups: std::collections::HashSet<_> = self.owned.iter().map(|id| id.pgrp).collect();
        for pgrp in groups {
            let known_member_is_live = self.owned.iter().any(|want| {
                want.pgrp == pgrp
                    && process_identity(want.pid).is_some_and(|now| {
                        now.start_time == want.start_time
                            && now.pgrp == want.pgrp
                            && now.session == want.session
                            && !matches!(now.state, 'Z' | 'X')
                    })
            });
            if known_member_is_live {
                // SAFETY: a recorded PID/start/session identity still proves membership in this
                // test-owned process group; empty or recycled groups are never signalled.
                unsafe { libc::kill(-pgrp, signal) };
            }
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| "matrix guard child handle was already removed".to_string())?;
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => return Err("matrix still requires owner rescue".into()),
            Err(error) => return Err(format!("cannot reap matrix child: {error}")),
        }
        let survivors: Vec<_> = self.owned.iter().filter(|id| identity_alive(id)).collect();
        if !survivors.is_empty() {
            return Err(format!(
                "matrix left owned identities running: {survivors:?}"
            ));
        }
        Ok(())
    }

    fn cleanup_owned(&self) -> bool {
        let wait_stopped = |limit: std::time::Duration| {
            let deadline = std::time::Instant::now() + limit;
            while self.owned.iter().any(identity_alive) {
                if std::time::Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            true
        };
        self.signal_owned_groups(libc::SIGTERM);
        if wait_stopped(std::time::Duration::from_secs(5)) {
            return true;
        }
        self.signal_owned_groups(libc::SIGKILL);
        wait_stopped(std::time::Duration::from_secs(5))
    }
}

impl Drop for MatrixGuard {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.release, b"release");
        self.record_descendants();
        if self.child.is_some() {
            let ordinary_wait = if std::thread::panicking() {
                std::time::Duration::ZERO
            } else {
                std::time::Duration::from_secs(30)
            };
            if self.wait_for(ordinary_wait).is_none() {
                let pid = self.pid();
                send_signal(pid, libc::SIGTERM);
                self.record_descendants();
                if self.wait_for(std::time::Duration::from_secs(15)).is_none() {
                    // SAFETY: this guard still owns and has not reaped the direct child.
                    unsafe { libc::kill(pid, libc::SIGKILL) };
                    if self.wait_for(std::time::Duration::from_secs(5)).is_none() {
                        eprintln!(
                            "matrix child {pid} did not reap within bounded failure teardown"
                        );
                    }
                }
            }
        }
        if !self.cleanup_owned() {
            eprintln!(
                "matrix test guard left owned processes running: {:?}",
                self.owned
            );
        }
    }
}

fn marker_identity(value: &Value, name: &str) -> ProcessIdentity {
    ProcessIdentity {
        pid: value[name].as_i64().unwrap() as i32,
        parent: 0,
        pgrp: value[format!("{name}_pgrp")].as_i64().unwrap() as i32,
        session: value[format!("{name}_session")].as_i64().unwrap() as i32,
        start_time: value[format!("{name}_start")].as_u64().unwrap(),
        state: 'S',
    }
}

fn matrix_signal_case(signal: i32) {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("active.json");
    let launches = dir.path().join("launches.txt");
    let rows_path = dir.path().join("matrix.jsonl");
    let helper_exe = std::env::current_exe().unwrap();
    let agents = agents_file(
        dir.path(),
        json!([
            {"label": "oracle", "argv": []},
            {"label": "stall", "argv": [helper_exe, "--exact", "helper_agent", "--nocapture", "--test-threads=1"]}
        ]),
    );
    let stdout = std::fs::File::create(&rows_path).unwrap();
    let child = cli()
        .env(HELPER_ENV, "stall")
        .env(MARKER_ENV, &marker)
        .env(LAUNCH_LOG_ENV, &launches)
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--agents",
            &agents,
            "--trials",
            "1",
        ])
        .stdout(stdout)
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let release = dir.path().join("unused-release");
    let owner = process_identity(child.id() as i32).expect("matrix process identity");
    let mut matrix = MatrixGuard {
        child: Some(child),
        owner,
        release,
        owned: Vec::new(),
    };
    let ready_deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !marker.is_file() && std::time::Instant::now() < ready_deadline {
        matrix.record_descendants();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    matrix.record_descendants();
    assert!(
        marker.is_file(),
        "readiness missing; status={:?}, launches={:?}, rows={:?}",
        matrix.child.as_mut().unwrap().try_wait(),
        std::fs::read_to_string(&launches),
        std::fs::read_to_string(&rows_path)
    );

    let ready: Value = serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
    let agent = marker_identity(&ready, "leader");
    let descendant = marker_identity(&ready, "descendant");
    assert!(
        identity_alive(&agent),
        "the ready agent identity is live: {agent:?}"
    );
    assert!(
        identity_alive(&descendant),
        "the ready descendant identity is live: {descendant:?}"
    );
    let before = process_descendants(matrix.pid());
    for process in &before {
        matrix.record_identity(process.clone());
    }
    matrix.record_identity(agent.clone());
    matrix.record_identity(descendant.clone());
    let unique_by_pid: std::collections::HashMap<_, _> = before
        .iter()
        .map(|process| (process.pid, process))
        .collect();
    let unique_pids: std::collections::HashSet<_> = unique_by_pid.keys().copied().collect();
    assert!(
        unique_pids.len() == 5,
        "readiness contains the exact five distinct owned process IDs: {before:?}"
    );
    assert!(
        before
            .iter()
            .any(|p| p.pid == agent.pid && p.start_time == agent.start_time)
    );
    assert!(
        before
            .iter()
            .any(|p| p.pid == descendant.pid && p.start_time == descendant.start_time)
    );

    let prefix_before = std::fs::read(&rows_path).unwrap();
    let prefix_lines: Vec<_> = prefix_before
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(
        prefix_lines.len(),
        1,
        "only the completed prefix is committed before readiness"
    );
    let prefix: Value = serde_json::from_slice(prefix_lines[0]).unwrap();
    assert_eq!(
        (
            prefix["task"].as_str(),
            prefix["agent"].as_str(),
            prefix["outcome"].as_str()
        ),
        (Some("signup"), Some("oracle"), Some("pass"))
    );

    // The only signal is sent to the matrix PID; its ordinary trial runner
    // must finish owned cleanup before the matrix exits.
    send_signal(matrix.pid(), signal);
    let status = matrix.wait();
    assert_eq!(
        status.code(),
        Some(2),
        "SIGTERM and SIGINT both use matrix exit 2"
    );
    for process in &before {
        assert!(
            !identity_alive(process),
            "owned process was gone when matrix exited: {process:?}"
        );
    }
    assert!(!identity_alive(&agent));
    assert!(!identity_alive(&descendant));

    let output = std::fs::read(rows_path).unwrap();
    assert!(
        output.starts_with(&prefix_before),
        "the exact committed JSONL byte prefix is immutable"
    );
    let parsed = rows(&output);
    assert_eq!(parsed.len(), 4, "full schema-v1 product accounting");
    assert_eq!(
        serde_json::to_value(&parsed[0]).unwrap(),
        prefix,
        "the completed prefix is immutable"
    );
    assert_eq!(
        (
            parsed[1].task.as_deref(),
            parsed[1].agent.as_deref(),
            parsed[1].outcome
        ),
        (Some("signup"), Some("stall"), Outcome::HarnessError)
    );
    assert_eq!(
        parsed[1].agent_exits.len(),
        1,
        "active agent status is retained"
    );
    assert_eq!(parsed[1].agent_exits[0].signal, Some(libc::SIGTERM));
    for (row, agent_label) in parsed[2..].iter().zip(["oracle", "stall"]) {
        assert_eq!(row.task.as_deref(), Some("otp"));
        assert_eq!(row.agent.as_deref(), Some(agent_label));
        assert_eq!(row.outcome, Outcome::HarnessError);
        assert_eq!(row.wall_time_ms, 0);
        assert_eq!(row.agent_exit_status, None);
        assert!(row.agent_exits.is_empty());
    }
    assert_eq!(
        parsed
            .iter()
            .map(|row| row.trial_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    assert_eq!(
        std::fs::read_to_string(launches).unwrap().lines().count(),
        1,
        "the refusal suffix never starts a trial"
    );

    let report_path = dir.path().join("matrix.jsonl");
    let report = cli()
        .args(["report", "--input", report_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(report.status.success(), "{}", stderr(&report));
    let report: Value = serde_json::from_slice(&report.stdout).unwrap();
    let groups = report["groups"].as_array().unwrap();
    let oracle: Vec<_> = groups.iter().filter(|g| g["agent"] == "oracle").collect();
    assert_eq!(oracle.len(), 2);
    assert_eq!(
        oracle
            .iter()
            .map(|g| g["attempts"].as_u64().unwrap())
            .sum::<u64>(),
        2
    );
    let signup = oracle.iter().find(|g| g["task"] == "signup").unwrap();
    let otp = oracle.iter().find(|g| g["task"] == "otp").unwrap();
    assert_eq!(
        (signup["passes"].as_u64(), signup["harness_errors"].as_u64()),
        (Some(1), Some(0))
    );
    assert_eq!(
        (otp["passes"].as_u64(), otp["harness_errors"].as_u64()),
        (Some(0), Some(1))
    );
    assert_eq!(signup["passes_over_attempts"], 1.0);
    assert_eq!(otp["passes_over_attempts"], 0.0);
    let stall: Vec<_> = groups.iter().filter(|g| g["agent"] == "stall").collect();
    assert_eq!(stall.len(), 2);
    assert_eq!(
        stall
            .iter()
            .map(|g| g["attempts"].as_u64().unwrap())
            .sum::<u64>(),
        2
    );
    assert!(
        stall
            .iter()
            .all(|g| g["harness_errors"] == 1 && g["success_rate"].is_null())
    );
    assert!(
        matrix.finish().is_ok(),
        "matrix and all owned identities were reaped"
    );
}

#[test]
fn matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup() {
    matrix_signal_case(libc::SIGTERM);
    matrix_signal_case(libc::SIGINT);
}

#[test]
fn a_broken_matrix_stdout_stops_after_the_owned_trial_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let launches = dir.path().join("launches.txt");
    let argv = vec![
        "/bin/sh",
        "-c",
        "printf 'started\n' >> \"$0\"",
        launches.to_str().unwrap(),
    ];
    let agents = agents_file(dir.path(), json!([{"label": "touch", "argv": argv}]));
    let mut child = cli()
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--agents",
            &agents,
            "--trials",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take().unwrap());
    let owner = process_identity(child.id() as i32).expect("matrix process identity");
    let mut matrix = MatrixGuard {
        child: Some(child),
        owner,
        release: dir.path().join("unused-release"),
        owned: Vec::new(),
    };
    let status = matrix.wait();
    assert_eq!(status.code(), Some(2));
    assert_eq!(
        std::fs::read_to_string(&launches).unwrap().lines().count(),
        1,
        "stdout failure stops admission after the active trial is reaped"
    );
    let mut diagnostic = String::new();
    matrix
        .child
        .as_mut()
        .unwrap()
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut diagnostic)
        .unwrap();
    assert!(diagnostic.contains("stdout write failed"), "{diagnostic}");
    assert!(
        matrix.finish().is_ok(),
        "broken-pipe matrix cleanup completed"
    );
}

#[test]
fn normal_matrix_binary_ignores_test_control_environment_variables() {
    let dir = tempfile::tempdir().unwrap();
    let agents = agents_file(dir.path(), json!([{"label": "oracle", "argv": []}]));
    let mut cmd = cli();
    for (name, value) in [
        ("EVAL_TEST_MATRIX_SIGNAL_SETUP_FAIL", "1"),
        ("EVAL_TEST_MATRIX_ADMISSION_HOLD_DIR", "admission"),
        ("EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR", "pre-trap"),
        ("EVAL_TEST_MATRIX_COMMIT_HOLD_DIR", "commit"),
        ("EVAL_TEST_MATRIX_FINAL_HOLD_DIR", "final"),
        ("EVAL_TEST_MATRIX_PENDING_HOLD_DIR", "pending"),
        ("EVAL_TEST_MATRIX_PENDING_HOLD_AT", "1"),
        ("EVAL_TEST_MATRIX_SPAWN_HOLD_DIR", "spawned"),
        ("EVAL_TEST_MATRIX_SIGNAL_LOG", "signal-log"),
        ("EVAL_TEST_MATRIX_TRIAL_START", "trial-start"),
        ("EVAL_TEST_MATRIX_SIGNAL_OBSERVED", "signal-observed"),
        ("EVAL_TEST_MATRIX_OUTPUT_ACK_PENDING", "ack-pending"),
        ("EVAL_TEST_MATRIX_OUTPUT_TIMEOUT", "output-timeout"),
        ("EVAL_TEST_MATRIX_EOF_TIMEOUT", "eof-timeout"),
        ("EVAL_TEST_MATRIX_CHILD_REAPED", "child-reaped"),
        ("EVAL_TEST_MATRIX_TRIALS", "2"),
        ("EVAL_TEST_CLEANUP_HOLD_DIR", "cleanup"),
        ("EVAL_TEST_PRE_HANDLER_HOLD_DIR", "pre-handler"),
        ("EVAL_TEST_MATRIX_WAIT_PENDING", "wait-pending"),
        ("EVAL_TEST_MATRIX_CLEANUP_ENTERED", "cleanup-entered"),
        ("EVAL_TEST_MATRIX_RELEASE", "release"),
        ("EVAL_TEST_MATRIX_TRIAL_PID", "trial-pid"),
        ("EVAL_TEST_MATRIX_TRIAL_EXE", "trial-exe"),
        ("EVAL_TEST_MATRIX_PROBE", "1"),
        ("EVAL_RUNNER_FAULT", "panic"),
    ] {
        cmd.env(name, dir.path().join(value));
    }
    let out = cmd
        .args([
            "matrix", "--tasks", "signup", "--agents", &agents, "--trials", "1",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let result_rows = rows(&out.stdout);
    assert_eq!(result_rows.len(), 1);
    assert_eq!(result_rows[0].outcome, Outcome::Pass);
    for name in [
        "admission",
        "pre-trap",
        "commit",
        "final",
        "pending",
        "spawned",
        "signal-log",
        "trial-start",
        "signal-observed",
        "ack-pending",
        "output-timeout",
        "eof-timeout",
        "child-reaped",
        "cleanup",
        "pre-handler",
        "wait-pending",
        "cleanup-entered",
        "release",
        "trial-pid",
        "trial-exe",
    ] {
        assert!(
            !dir.path().join(name).exists(),
            "ordinary production binary did not create {name} test-control state"
        );
    }
}

#[test]
fn matrix_signal_cancels_and_reaps_the_fallback_cargo_group() {
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("cargo-ready");
    let invocation = dir.path().join("cargo-invocations.log");
    let script = dir.path().join("cargo-wrapper.sh");
    std::fs::write(&script, format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"{}\"\nsleep 1000 &\nchild=$!\nprintf '%s %s\\n' \"$$\" \"$child\" > \"{}.tmp\"\nmv \"{}.tmp\" \"{}\"\nwait \"$child\"\n",
        invocation.display(), ready.display(), ready.display(), ready.display()
    )).unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script, permissions).unwrap();
    let agents = agents_file(dir.path(), json!([{"label": "oracle", "argv": []}]));
    let rows_path = dir.path().join("rows.jsonl");
    let child = cli()
        .env_remove(swarmail_evals::runner::SWARMAIL_BIN_ENV)
        .env("CARGO", &script)
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--agents",
            &agents,
            "--trials",
            "1",
        ])
        .stdout(std::fs::File::create(&rows_path).unwrap())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let owner = process_identity(child.id() as i32).expect("matrix process identity");
    let mut matrix = MatrixGuard {
        child: Some(child),
        owner,
        release: dir.path().join("unused-release"),
        owned: Vec::new(),
    };
    eventually("fallback build readiness", || {
        matrix.record_descendants();
        ready.is_file()
    });
    let pids: Vec<i32> = std::fs::read_to_string(&ready)
        .unwrap()
        .split_whitespace()
        .map(|p| p.parse().unwrap())
        .collect();
    assert_eq!(pids.len(), 2);
    let invocations = std::fs::read_to_string(&invocation).unwrap();
    let cargo_args = invocations.lines().collect::<Vec<_>>();
    assert_eq!(
        cargo_args.len(),
        1,
        "fallback cargo is invoked once: {invocations}"
    );
    assert_eq!(
        cargo_args[0],
        "build --offline --locked --bin swarmail --message-format=json-render-diagnostics"
    );
    let owned: Vec<_> = pids
        .iter()
        .map(|pid| process_identity(*pid).unwrap())
        .collect();
    assert!(owned.iter().all(identity_alive));
    send_signal(matrix.pid(), libc::SIGTERM);
    let status = matrix.wait();
    assert_eq!(status.code(), Some(2));
    assert!(
        owned.iter().all(|identity| !identity_alive(identity)),
        "fallback children stopped before matrix exit"
    );
    let result_rows = rows(&std::fs::read(rows_path).unwrap());
    assert_eq!(result_rows.len(), 2);
    assert!(
        result_rows
            .iter()
            .all(|row| row.outcome == Outcome::HarnessError && row.wall_time_ms == 0)
    );
    assert_eq!(
        result_rows
            .iter()
            .map(|row| row.trial_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        2,
        "both unstarted fallback slots receive fresh refusal identities"
    );
    assert!(
        result_rows
            .iter()
            .all(|row| row.agent_exit_status.is_none() && row.agent_exits.is_empty())
    );
    assert!(
        matrix.finish().is_ok(),
        "fallback owner and children were reaped"
    );
}
