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
use std::thread;
use swarmail_evals::state::FailureClass;
use swarmail_evals::tasks::{self, Outcome};

#[test]
fn helper_agent() {
    maybe_run_helper();
}

/// Run `f` for every task concurrently and collect the results by id.
fn per_task<T: Send + 'static>(
    f: impl Fn(String) -> T + Send + Sync + Copy + 'static,
) -> Vec<(String, T)> {
    swarmail_bin();
    let handles: Vec<_> = tasks::catalogue()
        .into_iter()
        .map(|t| {
            let id = t.id.clone();
            thread::spawn(move || (id.clone(), f(id)))
        })
        .collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
}

/// (a) The positive control: every task passes with the oracle.
#[test]
fn every_task_passes_with_the_oracle() {
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
    let handles: Vec<_> = cases
        .into_iter()
        .map(|(task, mode, class)| {
            thread::spawn(move || {
                let (row, out) = helper_trial(mode, &["--task", task, "--agent", mode], &[]);
                (task, class, row, stderr(&out))
            })
        })
        .collect();
    for h in handles {
        let (task, class, row, err) = h.join().unwrap();
        assert_eq!(row.outcome, Outcome::Fail, "{task}: {row:?}\n{err}");
        assert_eq!(row.failure_class, Some(class), "{task}: {row:?}\n{err}");
    }
}

/// (c) Isolation: two isolation trials, both live at once (all six agents
/// meet at a barrier before any acts), both pass.
#[test]
fn two_concurrent_isolation_trials_both_pass() {
    swarmail_bin();
    let barrier = tempfile::tempdir().unwrap();
    let dir = barrier.path().to_str().unwrap().to_string();
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let dir = dir.clone();
            thread::spawn(move || {
                helper_trial(
                    "barrier_oracle",
                    &["--task", "inbox-isolation", "--agent", "oracle-at-barrier"],
                    &[(BARRIER_DIR_ENV, &dir), (BARRIER_COUNT_ENV, "6")],
                )
            })
        })
        .collect();
    let rows: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    for (row, out) in &rows {
        assert_eq!(row.outcome, Outcome::Pass, "{row:?}\n{}", stderr(out));
        assert_eq!(row.agent_exits.len(), 3);
        assert!(row.agent_exits.iter().all(|e| e.code == Some(0)));
    }
    assert_ne!(rows[0].0.trial_id, rows[1].0.trial_id);
    let idents: std::collections::HashSet<_> = rows
        .iter()
        .flat_map(|(r, _)| r.agent_exits.iter().map(|e| e.identity.clone()))
        .collect();
    assert_eq!(
        idents.len(),
        6,
        "six distinct identities across the two trials"
    );
    assert_eq!(std::fs::read_dir(barrier.path()).unwrap().count(), 6);
}
