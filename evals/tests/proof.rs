//! The harness is proven able to fail before it measures anything (epic
//! #164's rule; #148's acceptance criteria; decision 0055).
//!
//! Every trial here runs through the real CLI: a fresh Swarmail built from
//! this checkout, a fresh fixture sending real SMTP, the agent as a child
//! process speaking MCP and HTTP, and the app-state check.
//!
//!   cargo test --locked --test proof        (from evals/)

mod common;

use common::*;
use serde_json::Value;
use swarmail_evals::state::FailureClass;
use swarmail_evals::tasks::{self, Outcome};

#[test]
fn helper_agent() {
    maybe_run_helper();
}

/// Run every task sequentially through the real command path.
fn per_task<T>(f: impl Fn(String) -> T) -> Vec<(String, T)> {
    swarmail_bin();
    tasks::catalogue()
        .into_iter()
        .map(|task| {
            let id = task.id;
            let result = f(id.clone());
            (id, result)
        })
        .collect()
}

/// (a) The positive control: every task passes with the oracle.
#[test]
fn every_task_passes_with_the_oracle() {
    let _trial_scope = trial_scope();
    let results = per_task(|id| {
        let dir = tempfile::tempdir().unwrap();
        let (row, out) = trial(&[
            "--task",
            &id,
            "--agent",
            "oracle",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ]);
        let state = retained_state(dir.path(), &row);
        (row, stderr(&out), state)
    });
    assert_eq!(results.len(), 16);
    let mut failures = Vec::new();
    for (id, (row, err, state)) in &results {
        if row.outcome != Outcome::Pass {
            failures.push(format!("{id}: {row:?}\n{err}"));
            continue;
        }
        assert_eq!(row.task.as_deref(), Some(id.as_str()));
        assert_eq!(row.agent.as_deref(), Some("oracle"));
        assert_eq!(row.confinement, "unconfined");
        let task = tasks::find(id).unwrap();
        assert_eq!(row.agent_exits.len(), task.agent_count as usize);
        assert!(row.agent_exits.iter().all(|e| e.code == Some(0)), "{row:?}");
        if task.agent_count == 1 {
            assert_eq!(row.agent_exit_status, Some(0));
        } else {
            assert_eq!(
                row.agent_exit_status, None,
                "isolation invents no aggregate status"
            );
        }
        // Every delivery went through Swarmail's SMTP and was accepted.
        let deliveries = state["deliveries"].as_array().unwrap();
        assert!(!deliveries.is_empty(), "{id}");
        assert!(
            deliveries.iter().all(|d| d["status"] == "delivered"),
            "{id}: {deliveries:?}"
        );
    }
    assert!(
        failures.is_empty(),
        "oracle failures:\n{}",
        failures.join("\n\n")
    );

    // Trap fidelity carried by the oracle's own runs.
    let state_of = |id: &str| &results.iter().find(|(i, _)| i == id).unwrap().1.2;
    let retry = &state_of("smtp-retry")["deliveries"][0]["attempts"];
    let codes: Vec<u64> = retry
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["code"].as_u64().unwrap())
        .collect();
    assert_eq!(
        codes,
        vec![451, 250],
        "a real 451 under chaos, then the fixture's retry"
    );
    let stale = state_of("stale-otp")["deliveries"].as_array().unwrap();
    assert_eq!(stale.len(), 2, "both OTP generations were mailed");
    let sibling = state_of("sibling-addresses")["identities"]
        .as_array()
        .unwrap();
    assert_eq!(sibling.iter().filter(|i| i["role"] == "decoy").count(), 1);
    let reuse = &state_of("reset-reuse")["identities"][0]["facts"];
    assert_eq!(reuse["reset_count"], 1);
    assert_eq!(reuse["reset_replay_rejected"], true);
    let iso = state_of("inbox-isolation")["identities"]
        .as_array()
        .unwrap();
    let inboxes: std::collections::HashSet<&str> =
        iso.iter().map(|i| i["inbox"].as_str().unwrap()).collect();
    assert_eq!(inboxes.len(), 3, "three AUTH inboxes on one server");
}

fn no_goal_completed(state: &Value) -> bool {
    state["identities"].as_array().unwrap().iter().all(|i| {
        let f = &i["facts"];
        f.as_object()
            .unwrap()
            .iter()
            .all(|(_, v)| v == false || v == 0)
    })
}

/// (b) The negative control: every task fails with null, each with the
/// idle class the task declares, and nothing was completed.
#[test]
fn every_task_fails_with_null_and_its_declared_idle_class() {
    let _trial_scope = trial_scope();
    let results = per_task(|id| {
        let dir = tempfile::tempdir().unwrap();
        let (row, out) = trial(&[
            "--task",
            &id,
            "--agent",
            "null",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ]);
        let state = retained_state(dir.path(), &row);
        (row, stderr(&out), state)
    });
    assert_eq!(results.len(), 16);
    for (id, (row, err, state)) in &results {
        let task = tasks::find(id).unwrap();
        assert_eq!(row.outcome, Outcome::Fail, "{id}: {row:?}\n{err}");
        assert_eq!(row.failure_class, Some(task.idle_class), "{id}: {row:?}");
        assert!(
            row.agent_exits.iter().all(|e| e.code == Some(0)),
            "null exits zero and still fails"
        );
        assert!(no_goal_completed(state), "{id}: {state}");
    }
    // Both declared idle classes are exercised.
    let classes: std::collections::HashSet<_> = results
        .iter()
        .map(|(_, (r, _, _))| r.failure_class.unwrap())
        .collect();
    assert!(
        classes.contains(&FailureClass::GaveUp) && classes.contains(&FailureClass::WaitTimeout)
    );
}

/// Deliberate wrong actions fail in their observable classes.
#[test]
fn wrong_actions_fail_in_their_observable_classes() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    let cases = [
        (
            "sibling-addresses",
            "wrong_message",
            FailureClass::WrongMessage,
        ),
        ("stale-otp", "stale_code", FailureClass::StaleCode),
        ("confirm-decoys", "wrong_link", FailureClass::WrongLink),
        ("reset-login", "tool_misuse", FailureClass::ToolMisuse),
        ("inbox-isolation", "foreign", FailureClass::WrongMessage),
    ];
    for (task, mode, class) in cases {
        let (row, out) = helper_trial(mode, &["--task", task, "--agent", mode], &[]);
        assert_eq!(
            row.outcome,
            Outcome::Fail,
            "{task}: {row:?}\n{}",
            stderr(&out)
        );
        assert_eq!(row.failure_class, Some(class), "{task}: {row:?}");
    }
}

/// (c) Sequential admission prevents a competing agent from being launched
/// while an idle victim is active. No cross-trial attack client is needed.
#[test]
fn sequential_poacher_control_refuses_competing_trial() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("victim-ready");
    let (mut command, _wrapper) = helper_command(
        "held_idle",
        &[
            "--task",
            "signup",
            "--agent",
            "idle-victim",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ],
        &[(MARKER_ENV, ready.to_str().unwrap())],
    );
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let victim = ChildGuard {
        child: Some(command.spawn().unwrap()),
    };
    eventually("idle victim readiness", || ready.exists());
    let launch = dir.path().join("competitor-launched");
    let (row, out) = trial(&[
        "--task",
        "signup",
        "--agent",
        "inert-competitor",
        "--",
        "/usr/bin/touch",
        launch.to_str().unwrap(),
    ]);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(row.wall_time_ms, 0);
    assert!(stderr(&out).contains("concurrent_trials_refused"));
    assert!(!launch.exists(), "competing agent must never launch");
    std::fs::write(ready.with_extension("release"), b"release").unwrap();
    let out = victim.wait_with_output();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Fail, "{}", stderr(&out));
    assert_eq!(row.failure_class, Some(FailureClass::GaveUp));
    let state = retained_state(dir.path(), &row);
    assert!(no_goal_completed(&state));
    assert!(
        state["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["accepted"] != true)
    );
    let (row, out) = trial(&["--task", "signup", "--agent", "oracle"]);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
}

/// Completed try-all commands must lose even though the real app goal is true.
#[test]
fn completed_naive_controls_recover_the_goal_but_lose_the_trial() {
    let _trial_scope = common::trial_scope();
    use swarmail_evals::state::{DeliveryStatus, Flow, Role, Snapshot, action};
    let cases = [
        (
            "confirm-decoys",
            "naive_forward",
            action::CONFIRM,
            "decoy_link",
            Flow::Signup,
            2,
            FailureClass::WrongLink,
        ),
        (
            "confirm-decoys",
            "naive_reverse",
            action::CONFIRM,
            "decoy_link",
            Flow::Signup,
            2,
            FailureClass::WrongLink,
        ),
        (
            "sibling-addresses",
            "naive_sibling_agent",
            action::CONFIRM,
            "foreign_token",
            Flow::Signup,
            1,
            FailureClass::WrongMessage,
        ),
        (
            "sibling-addresses",
            "naive_sibling_decoy",
            action::CONFIRM,
            "foreign_token",
            Flow::Signup,
            1,
            FailureClass::WrongMessage,
        ),
        (
            "i18n-subject",
            "naive_forward",
            action::CONFIRM,
            "decoy_message",
            Flow::Signup,
            1,
            FailureClass::WrongMessage,
        ),
        (
            "stale-otp",
            "naive_forward",
            action::OTP_VERIFY,
            "superseded_code",
            Flow::Otp,
            1,
            FailureClass::StaleCode,
        ),
        (
            "magic-login",
            "naive_forward",
            action::MAGIC_LOGIN,
            "decoy_link",
            Flow::MagicLogin,
            1,
            FailureClass::WrongLink,
        ),
        (
            "entity-url",
            "naive_forward",
            action::MARKETING,
            "decoy_link",
            Flow::Marketing,
            2,
            FailureClass::WrongLink,
        ),
        (
            "entity-url",
            "naive_entity_raw",
            action::MARKETING,
            "marketing_mangled",
            Flow::Marketing,
            1,
            FailureClass::WrongLink,
        ),
        (
            "digest-forty",
            "naive_forward",
            action::DIGEST_SELECT,
            "digest_decoy",
            Flow::Digest,
            39,
            FailureClass::WrongLink,
        ),
    ];
    for (task, mode, goal_action, trap_action, flow, count, class) in cases {
        let dir = tempfile::tempdir().unwrap();
        let args = [
            "--task",
            task,
            "--agent",
            mode,
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ];
        let (mut command, wrapper) = helper_command(mode, &args, &[]);
        let argv = format!("{command:?}");
        let script = std::fs::read_to_string(wrapper.path().join("agent-wrapper.sh")).unwrap();
        let out = command.output().unwrap();
        let row = one_row(&out);
        let state = retained_state(dir.path(), &row);
        let snap: Snapshot = serde_json::from_value(state.clone()).unwrap();
        assert_eq!(
            row.agent_exit_status,
            Some(0),
            "{task}/{mode}: {row:?} {}",
            stderr(&out)
        );
        assert_eq!(row.agent_exits.len(), 1);
        assert_eq!(row.agent_exits[0].code, Some(0));
        assert!(!snap.deliveries.is_empty());
        assert!(
            snap.deliveries
                .iter()
                .all(|d| d.status == DeliveryStatus::Delivered
                    && d.attempts
                        .last()
                        .is_some_and(|a| a.stage == "dot" && a.code == 250)),
            "{snap:?}"
        );
        let traps: Vec<_> = snap
            .events
            .iter()
            .filter(|e| e.action == trap_action)
            .collect();
        assert_eq!(traps.len(), count, "{snap:?}");
        assert_eq!(snap.events.len(), count + 1, "exact completed actions");
        let who = snap
            .identities
            .iter()
            .find(|i| i.role == Role::Agent)
            .unwrap();
        for trap in &traps {
            assert!(!trap.accepted);
            assert_eq!(trap.category, Some(class));
            if mode == "naive_sibling_decoy" {
                assert_eq!(trap.flow, None);
                assert_eq!(
                    trap.claimed,
                    snap.identities
                        .iter()
                        .find(|i| i.role == Role::Decoy)
                        .map(|i| i.identity.clone())
                );
            } else {
                assert_eq!(trap.flow, Some(flow));
                assert_eq!(trap.claimed.as_deref(), Some(who.identity.as_str()));
            }
        }
        assert_eq!(
            snap.events
                .iter()
                .filter(|e| e.accepted && e.action == goal_action)
                .count(),
            1
        );
        let f = &who.facts;
        assert!(
            match task {
                "stale-otp" => f.otp_verified && f.logged_in,
                "magic-login" => f.magic_login && f.logged_in,
                "entity-url" => f.marketing_completed,
                "digest-forty" => f.digest_selected,
                _ => f.confirmed,
            },
            "the real goal recovered: {snap:?}"
        );
        let check = cli()
            .args(["check", "--task", task, "--state"])
            .arg(dir.path().join(&row.trial_id).join("state.json"))
            .output()
            .unwrap();
        let verdict: tasks::Verdict = serde_json::from_slice(&check.stdout).unwrap();
        assert_eq!(
            (verdict.outcome, verdict.failure_class),
            (row.outcome, row.failure_class)
        );
        assert_eq!(check.status.code(), Some(row.exit_code()));
        let replay = cli()
            .args(["check", "--artifacts-dir"])
            .arg(dir.path().join(&row.trial_id))
            .env("SWARMAIL_BIN", "/absent-during-offline-replay")
            .output()
            .unwrap();
        assert_eq!(
            replay.status.code(),
            Some(row.exit_code()),
            "{}",
            stderr(&replay)
        );
        let replay: Value = serde_json::from_slice(&replay.stdout).unwrap();
        assert_eq!(
            replay["original_result"],
            serde_json::to_value(&row).unwrap()
        );
        assert_eq!(replay["business_replay"]["status"], "scored");
        assert_eq!(
            replay["business_replay"]["verdict"],
            serde_json::to_value(verdict).unwrap()
        );

        // Optional durable receipts for the exact same executable red/green probes.
        if let Some(base) = std::env::var_os("EVAL_TRAP_RECEIPTS") {
            let base = std::path::PathBuf::from(base).join(format!("{task}-{mode}"));
            std::fs::create_dir_all(&base).unwrap();
            std::fs::write(
                base.join("state.json"),
                serde_json::to_vec_pretty(&state).unwrap(),
            )
            .unwrap();
            std::fs::write(base.join("result.json"), &out.stdout).unwrap();
            std::fs::write(base.join("check.json"), &check.stdout).unwrap();
            std::fs::write(
                base.join("command.txt"),
                format!("{argv}\ncheck exit: {}\n", check.status),
            )
            .unwrap();
            std::fs::write(base.join("agent-wrapper.sh"), script).unwrap();
        }
        assert_eq!(row.outcome, Outcome::Fail, "{task}/{mode}: {row:?}");
        assert_eq!(row.failure_class, Some(class));
    }
}
