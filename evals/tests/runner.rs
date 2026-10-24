//! The runner's contract with an external agent command and its lifecycle
//! (decision 0055 §§7–9): exact argv, identical prompt and environment,
//! no admin inputs exported, truthful exit statuses, and exactly one
//! schema-valid row on every handled ending — spawn failure, startup
//! failure, server death, panic, deadline, interruption.

mod common;

use common::*;
use serde_json::Value;
use std::process::Stdio;
use swarmail_evals::runner::TrialResult;
use swarmail_evals::state::FailureClass;
use swarmail_evals::tasks::Outcome;

#[test]
fn helper_agent() {
    maybe_run_helper();
}

#[test]
fn an_external_agent_gets_literal_argv_and_identical_prompt_and_env() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("echo.json");
    let (row, out) = helper_trial(
        "echo",
        &["--task", "signup", "--agent", "echo"],
        &[(MARKER_ENV, marker.to_str().unwrap())],
    );
    // It did nothing to the app and exited 3: a fail, with its real status.
    assert_eq!(
        (row.outcome, row.failure_class),
        (Outcome::Fail, Some(FailureClass::GaveUp)),
        "{}",
        stderr(&out)
    );
    assert_eq!(row.agent_exit_status, Some(3));
    assert_eq!(row.agent_exits[0].code, Some(3));
    assert_eq!(row.agent.as_deref(), Some("echo"));

    let report: Value = serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
    let argv: Vec<&str> = report["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        &argv[1..],
        ["--exact", "helper_agent", "--nocapture", "--test-threads=1"]
    );
    assert_eq!(
        report["prompt"], report["goal"],
        "EVAL_GOAL and the prompt file agree"
    );
    let env: Vec<(String, String)> = serde_json::from_value(report["env"].clone()).unwrap();
    let get = |k: &str| env.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    for k in [
        "EVAL_TASK_ID",
        "EVAL_APP_URL",
        "EVAL_MCP_URL",
        "EVAL_IDENTITY",
        "EVAL_INBOX",
        "EVAL_GOAL",
        "EVAL_PROMPT_FILE",
        "EVAL_DEADLINE_MS",
    ] {
        assert!(get(k).is_some(), "{k} exported");
    }
    assert!(
        get("EVAL_FIXTURE_ADMIN_SECRET").is_none(),
        "the admin secret is never exported"
    );
    assert_eq!(get("EVAL_TASK_ID").as_deref(), Some("signup"));
    assert_eq!(get("EVAL_DEADLINE_MS").as_deref(), Some("60000"));
    let mcp = get("EVAL_MCP_URL").unwrap();
    assert!(
        mcp.starts_with("http://127.0.0.1:") && mcp.ends_with("/mcp"),
        "{mcp}"
    );
    let prompt = report["prompt"].as_str().unwrap();
    for k in [
        "EVAL_APP_URL",
        "EVAL_MCP_URL",
        "EVAL_IDENTITY",
        "EVAL_INBOX",
    ] {
        assert!(prompt.contains(&get(k).unwrap()), "the prompt names {k}");
    }
    assert!(
        !prompt.contains('{'),
        "every placeholder rendered: {prompt}"
    );
    assert!(
        get("EVAL_PROMPT_FILE")
            .unwrap()
            .starts_with(report["cwd"].as_str().unwrap())
    );
}

#[test]
fn two_trials_use_fresh_ports_and_identities() {
    let dir = tempfile::tempdir().unwrap();
    let urls: Vec<(String, String)> = (0..2)
        .map(|i| {
            let marker = dir.path().join(format!("{i}.json"));
            helper_trial(
                "echo",
                &["--task", "signup", "--agent", "echo"],
                &[(MARKER_ENV, marker.to_str().unwrap())],
            );
            let report: Value = serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
            let env: Vec<(String, String)> = serde_json::from_value(report["env"].clone()).unwrap();
            let get = |k: &str| env.iter().find(|(n, _)| n == k).unwrap().1.clone();
            (get("EVAL_MCP_URL"), get("EVAL_IDENTITY"))
        })
        .collect();
    assert_ne!(urls[0], urls[1]);
}

#[test]
fn a_spawn_failure_is_a_harness_error_with_null_statuses() {
    let (row, _) = trial(&[
        "--task",
        "signup",
        "--agent",
        "ghost",
        "--",
        "/nonexistent/agent",
    ]);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(row.agent_exit_status, None);
    assert_eq!(row.agent_exits.len(), 1);
    assert_eq!(
        (row.agent_exits[0].code, row.agent_exits[0].signal),
        (None, None)
    );
}

#[test]
fn a_swarmail_that_never_comes_up_is_a_harness_error() {
    let out = cli()
        .env("SWARMAIL_BIN", "/bin/false")
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(
        stderr(&out).contains("exited before announcing"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn invalid_inputs_are_one_harness_error_row_with_the_supplied_labels() {
    for (args, task, agent) in [
        (vec!["--task", "signup"], Some("signup"), None),
        (vec!["--agent", "oracle"], None, Some("oracle")),
        (
            vec!["--task", "nope", "--agent", "oracle"],
            Some("nope"),
            Some("oracle"),
        ),
        (
            vec!["--task", "signup", "--agent", "oracle", "--", "/bin/true"],
            Some("signup"),
            Some("oracle"),
        ),
        (
            vec!["--task", "signup", "--agent", "custom"],
            Some("signup"),
            Some("custom"),
        ),
        (
            vec![
                "--task",
                "signup",
                "--agent",
                "oracle",
                "--deadline-ms",
                "0",
            ],
            Some("signup"),
            Some("oracle"),
        ),
    ] {
        let (row, _) = trial(&args);
        assert_eq!(row.outcome, Outcome::HarnessError, "{args:?}");
        assert_eq!(row.task.as_deref(), task, "{args:?}");
        assert_eq!(row.agent.as_deref(), agent, "{args:?}");
        assert!(row.agent_exits.is_empty());
    }
}

fn read_pids(marker: &std::path::Path) -> (i32, i32) {
    let pids: Value = serde_json::from_slice(&std::fs::read(marker).unwrap()).unwrap();
    (
        pids["leader"].as_i64().unwrap() as i32,
        pids["descendant"].as_i64().unwrap() as i32,
    )
}

#[test]
fn a_deadline_is_budget_and_reaps_the_agents_descendants() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let (row, out) = helper_trial(
        "stall",
        &[
            "--task",
            "signup",
            "--agent",
            "stall",
            "--deadline-ms",
            "1500",
        ],
        &[(MARKER_ENV, marker.to_str().unwrap())],
    );
    assert_eq!(
        (row.outcome, row.failure_class),
        (Outcome::Fail, Some(FailureClass::Budget)),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        row.agent_exit_status, None,
        "killed by a signal: no numeric code"
    );
    assert_eq!(row.agent_exits[0].signal, Some(libc::SIGTERM));
    let (leader, descendant) = read_pids(&marker);
    eventually("the stalled leader and its descendant are gone", || {
        !pid_alive(leader) && !pid_alive(descendant)
    });
}

#[test]
fn an_agent_that_succeeds_then_crashes_still_passes_with_its_real_status() {
    let (row, out) = helper_trial(
        "succeed_then_crash",
        &["--task", "signup", "--agent", "crashy"],
        &[],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.agent_exit_status, Some(7));
}

#[test]
fn a_noisy_agent_is_drained_and_does_not_hang_the_trial() {
    let (row, out) = helper_trial("noisy", &["--task", "signup", "--agent", "noisy"], &[]);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
}

#[test]
fn server_death_during_the_trial_is_a_harness_error() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let (row, out) = helper_trial(
        "stall",
        &["--task", "signup", "--agent", "stall"],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("EVAL_RUNNER_FAULT", "server_death"),
        ],
    );
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(
        stderr(&out).contains("swarmail exited during the trial"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_runner_panic_cleans_up_and_still_emits_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let (row, out) = helper_trial(
        "stall",
        &["--task", "signup", "--agent", "stall"],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("EVAL_RUNNER_FAULT", "panic"),
        ],
    );
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(stderr(&out).contains("panicked"), "{}", stderr(&out));
    if marker.exists() {
        let (leader, descendant) = read_pids(&marker);
        eventually("the agent group is killed after the panic", || {
            !pid_alive(leader) && !pid_alive(descendant)
        });
    }
}

#[test]
fn sigterm_interrupts_cleans_up_and_emits_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let me = std::env::current_exe().unwrap();
    let child = cli()
        .env(HELPER_ENV, "stall")
        .env(MARKER_ENV, &marker)
        .args(["trial", "--task", "signup", "--agent", "stall", "--"])
        .args([
            me.to_str().unwrap(),
            "--exact",
            "helper_agent",
            "--nocapture",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    eventually("the agent is running", || marker.exists());
    send_signal(child.id() as i32, libc::SIGTERM);
    let out = child.wait_with_output().unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError, "{}", stderr(&out));
    assert_eq!(row.task.as_deref(), Some("signup"));
    let (leader, descendant) = read_pids(&marker);
    eventually("the interrupted agent group is gone", || {
        !pid_alive(leader) && !pid_alive(descendant)
    });
}

#[test]
fn a_retained_snapshot_reproduces_the_verdict_and_bad_state_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let (row, _) = trial(&[
        "--task",
        "signup",
        "--agent",
        "oracle",
        "--artifacts-dir",
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(row.outcome, Outcome::Pass);
    let trial_dir = dir.path().join(&row.trial_id);
    for f in [
        "state.json",
        "result.json",
        "diagnostics.log",
        "metadata.json",
    ] {
        assert!(trial_dir.join(f).is_file(), "{f} retained");
    }
    let retained: swarmail_evals::runner::TrialResult =
        serde_json::from_slice(&std::fs::read(trial_dir.join("result.json")).unwrap()).unwrap();
    assert_eq!(retained, row);
    let state = trial_dir.join("state.json");
    let check = |task: &str, path: &std::path::Path| {
        let out = cli()
            .args(["check", "--task", task, "--state", path.to_str().unwrap()])
            .output()
            .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (
            out.status.code(),
            v["outcome"].as_str().unwrap().to_string(),
        )
    };
    assert_eq!(check("signup", &state), (Some(0), "pass".into()));
    // The same snapshot scored as another task is mismatched, not a verdict.
    assert_eq!(check("otp", &state), (Some(2), "harness_error".into()));
    // A fabricated success flag (no backing event) refuses.
    let mut forged: Value = serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    forged["events"] = Value::Array(vec![]);
    let forged_path = dir.path().join("forged.json");
    std::fs::write(&forged_path, forged.to_string()).unwrap();
    assert_eq!(
        check("signup", &forged_path),
        (Some(2), "harness_error".into())
    );
    std::fs::write(&forged_path, "{not json").unwrap();
    assert_eq!(
        check("signup", &forged_path),
        (Some(2), "harness_error".into())
    );
}

/// Precedence step 1 beats both the deadline and the business check: an
/// agent leaves a parked 451 receipt after provisioning (chaos at 100 over
/// a public release), and the trial is a harness error whether the agent
/// exits or runs into its deadline. `check` replays the retained state to
/// the same verdict.
fn parked_after_provisioning(mode: &str, deadline: &str) -> TrialResult {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("released");
    let artifacts = dir.path().join("artifacts");
    let (row, out) = helper_trial(
        mode,
        &[
            "--task",
            "slow-delivery",
            "--agent",
            mode,
            "--deadline-ms",
            deadline,
            "--artifacts-dir",
            artifacts.to_str().unwrap(),
        ],
        &[(MARKER_ENV, marker.to_str().unwrap())],
    );
    assert!(marker.exists(), "{mode}: the release completed");
    assert_eq!(
        (row.outcome, row.failure_class),
        (Outcome::HarnessError, Some(FailureClass::HarnessError)),
        "{mode}: {}",
        stderr(&out)
    );
    let state = retained_state(&artifacts, &row);
    let parked: Vec<&Value> = state["deliveries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["status"] == "parked")
        .collect();
    assert_eq!(parked.len(), 1, "{mode}: {state}");
    assert_eq!(
        parked[0]["attempts"],
        // DATA chaos answers the terminating dot with 451.
        serde_json::json!([{"stage": "dot", "code": 451}]),
        "{mode}"
    );
    let path = artifacts.join(&row.trial_id).join("state.json");
    let replay = cli()
        .args(["check", "--task", "slow-delivery", "--state"])
        .arg(&path)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(
        (replay.status.code(), &v["outcome"], &v["failure_class"]),
        (
            Some(2),
            &serde_json::json!("harness_error"),
            &serde_json::json!("harness_error")
        ),
        "{mode}"
    );
    row
}

#[test]
fn a_delivery_parked_after_provisioning_outranks_the_business_check() {
    let row = parked_after_provisioning("chaos_release", "20000");
    assert_eq!(row.agent_exit_status, Some(0), "the agent's real exit");
}

#[test]
fn a_delivery_parked_after_provisioning_outranks_the_deadline() {
    let row = parked_after_provisioning("chaos_release_stall", "8000");
    assert_eq!(row.agent_exits[0].signal, Some(libc::SIGTERM));
}

#[test]
fn the_catalogue_is_served_as_schema_version_one() {
    let out = cli().arg("tasks").output().unwrap();
    assert!(out.status.success());
    let cat: swarmail_evals::tasks::Catalogue = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(cat.schema_version, 1);
    assert_eq!(cat.tasks, swarmail_evals::tasks::catalogue());
}

#[test]
fn cleanup_escalates_after_a_leader_exits_and_a_descendant_ignores_term() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let me = std::env::current_exe().unwrap();
    let child = cli()
        .env(HELPER_ENV, "resistant")
        .env(MARKER_ENV, &marker)
        .args(["trial", "--task", "signup", "--agent", "resistant", "--"])
        .args([
            me.to_str().unwrap(),
            "--exact",
            "helper_agent",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let child = ChildGuard { child: Some(child) };
    eventually("the TERM-resistant descendant readiness marker", || {
        marker.is_file()
    });
    let value: Value = serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
    let leader = ProcessIdentity {
        pid: value["leader"].as_i64().unwrap() as i32,
        parent: 0,
        pgrp: value["leader_pgrp"].as_i64().unwrap() as i32,
        session: value["leader_session"].as_i64().unwrap() as i32,
        start_time: value["leader_start"].as_u64().unwrap(),
        state: 'S',
    };
    let descendant = ProcessIdentity {
        pid: value["descendant"].as_i64().unwrap() as i32,
        parent: 0,
        pgrp: value["descendant_pgrp"].as_i64().unwrap() as i32,
        session: value["descendant_session"].as_i64().unwrap() as i32,
        start_time: value["descendant_start"].as_u64().unwrap(),
        state: 'S',
    };
    assert_eq!(leader.pgrp, descendant.pgrp);
    assert!(identity_alive(&leader) && identity_alive(&descendant));
    send_signal(child.pid(), libc::SIGTERM);
    let out = child.wait_with_output();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(
        !identity_alive(&leader),
        "the direct agent leader stopped before matrix cleanup completed"
    );
    assert!(
        !identity_alive(&descendant),
        "cleanup escalated and observed the descendant gone"
    );
}
