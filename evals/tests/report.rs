//! The matrix and report commands (decision 0055 §§9–10): product
//! cardinality, a clean stdout, accounting that survives harness errors,
//! aggregated exits, and the report's estimates on fixed inputs.

mod common;

use common::*;
use serde_json::{Value, json};
use std::io::Write;
use std::process::Stdio;
use swarmail_evals::runner::TrialResult;
use swarmail_evals::tasks::Outcome;

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
        json!([{"label": "ghost", "argv": ["/nonexistent/agent"]}, {"label": "oracle", "argv": []}]),
    );
    let out = cli()
        .args([
            "matrix", "--tasks", "signup", "--agents", &agents, "--trials", "1",
        ])
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].outcome, Outcome::HarnessError);
    assert_eq!(rows[1].outcome, Outcome::Pass, "the matrix went on");
    assert_eq!(out.status.code(), Some(2));
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
