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
use serde_json::{Value, json};
use swarmail_evals::runner::TrialResult;
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
        let dir = tempdir();
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
    assert_eq!(results.len(), 24);
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
    let backlog = state_of("otp-backlog")["deliveries"].as_array().unwrap();
    assert_eq!(backlog.len(), 12, "all twelve code generations were mailed");
    let crowd = state_of("recipient-crowd");
    assert_eq!(
        crowd["identities"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["role"] == "decoy")
            .count(),
        4
    );
    assert_eq!(crowd["deliveries"].as_array().unwrap().len(), 5);
    let lookalike = state_of("digest-lookalike")["deliveries"]
        .as_array()
        .unwrap();
    assert_eq!(lookalike.len(), 1, "one digest");
    // The expert tier's mails, all delivered: the oracle's resend in
    // `otp-late` adds a fresh code and a reminder to the two prepared mails.
    for (id, mails) in [
        ("otp-correction", 2),
        ("digest-correction", 2),
        ("otp-late", 4),
        ("confirm-attachment", 1),
        ("confirm-injection", 2),
    ] {
        let deliveries = state_of(id)["deliveries"].as_array().unwrap();
        assert_eq!(deliveries.len(), mails, "{id}");
    }
    let reuse = &state_of("reset-reuse")["identities"][0]["facts"];
    assert_eq!(reuse["reset_count"], 1);
    assert_eq!(reuse["reset_replay_rejected"], true);
    let iso = state_of("inbox-isolation")["identities"]
        .as_array()
        .unwrap();
    let inboxes: std::collections::HashSet<&str> =
        iso.iter().map(|i| i["inbox"].as_str().unwrap()).collect();
    assert_eq!(inboxes.len(), 3, "three inboxes on one server");
    // The oracle passed only because each routed address reached its own
    // inbox under the one shared login (decision 0112 §7): the recipient of
    // every account names its inbox under the runner's route domain.
    for account in iso {
        let identity = account["identity"].as_str().unwrap();
        let inbox = account["inbox"].as_str().unwrap();
        assert!(
            identity.ends_with(&format!("@{inbox}.swarmail.test")),
            "{identity} is not routed to {inbox}"
        );
    }
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
        let dir = tempdir();
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
    assert_eq!(results.len(), 24);
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
        // The hard tier's named mistakes (proposed 0117).
        ("otp-backlog", "hard_stale_code", FailureClass::StaleCode),
        (
            "recipient-crowd",
            "hard_wrong_message",
            FailureClass::WrongMessage,
        ),
        (
            "digest-lookalike",
            "hard_wrong_link",
            FailureClass::WrongLink,
        ),
        // The expert tier's targeted mistakes (proposed 0123), each through
        // the real trial path; the shortcut tools' answers are asserted by
        // the scripted agents before they commit the mistake.
        (
            "otp-correction",
            "expert_uncorrected",
            FailureClass::StaleCode,
        ),
        (
            "digest-correction",
            "expert_retracted",
            FailureClass::WrongLink,
        ),
        ("otp-late", "expert_early", FailureClass::StaleCode),
        ("otp-late", "expert_reminder", FailureClass::StaleCode),
        (
            "confirm-attachment",
            "expert_body_link",
            FailureClass::WrongLink,
        ),
        ("confirm-injection", "expert_obey", FailureClass::WrongLink),
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
    let dir = tempdir();
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
    let victim = ChildGuard::new(command.spawn().unwrap());
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
        // The expert tier (proposed 0123): each commits exactly its named
        // trap, then recovers. `otp-late` also asks for its resend.
        (
            "otp-correction",
            "naive_forward",
            action::OTP_VERIFY,
            "superseded_code",
            Flow::Otp,
            1,
            FailureClass::StaleCode,
        ),
        (
            "digest-correction",
            "naive_forward",
            action::DIGEST_SELECT,
            "digest_decoy",
            Flow::Digest,
            1,
            FailureClass::WrongLink,
        ),
        (
            "otp-late",
            "naive_forward",
            action::OTP_VERIFY,
            "superseded_code",
            Flow::Otp,
            1,
            FailureClass::StaleCode,
        ),
        (
            "confirm-attachment",
            "naive_forward",
            action::CONFIRM,
            "decoy_link",
            Flow::Signup,
            1,
            FailureClass::WrongLink,
        ),
        (
            "confirm-injection",
            "naive_forward",
            action::CONFIRM,
            "decoy_link",
            Flow::Signup,
            1,
            FailureClass::WrongLink,
        ),
        // The hard tier: each commits exactly its named trap, then recovers.
        (
            "otp-backlog",
            "naive_forward",
            action::OTP_VERIFY,
            "superseded_code",
            Flow::Otp,
            1,
            FailureClass::StaleCode,
        ),
        (
            "recipient-crowd",
            "naive_forward",
            action::CONFIRM,
            "foreign_token",
            Flow::Signup,
            1,
            FailureClass::WrongMessage,
        ),
        (
            "digest-lookalike",
            "naive_forward",
            action::DIGEST_SELECT,
            "digest_decoy",
            Flow::Digest,
            1,
            FailureClass::WrongLink,
        ),
    ];
    for (task, mode, goal_action, trap_action, flow, count, class) in cases {
        let dir = tempdir();
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
        // The resend `otp-late` asks for is the one other recorded action.
        let resends = usize::from(task == "otp-late");
        assert_eq!(
            snap.events.len(),
            count + 1 + resends,
            "exact completed actions"
        );
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
                "stale-otp" | "otp-backlog" | "otp-correction" | "otp-late" => {
                    f.otp_verified && f.logged_in
                }
                "magic-login" => f.magic_login && f.logged_in,
                "entity-url" => f.marketing_completed,
                "digest-forty" | "digest-lookalike" | "digest-correction" => f.digest_selected,
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

/// Epic #164 / issue #156: recovery after a failed wait is mechanically
/// possible from the timed-out answer alone, and the tempting wrong choice
/// still loses. Three scripted agents run through the real CLI, real MCP
/// (protocol 2025-03-26) and real SMTP, with no catalogue change:
///
/// - the sibling trap's agent mistypes the subject, reads `near_misses`,
///   picks the entry addressed to its exact identity and passes with no trap
///   event;
/// - the same wait, acting on `nearest_subject` — the decoy, which lands
///   last and ties on subject — is refused by the application and loses as
///   `wrong_message`;
/// - in slow delivery the first wait's timeout carries no `near_misses`
///   (nothing arrived), and after the release the trial passes.
///
/// These controls do not measure the live-model recovery rate, which needs
/// paid trials; slow-delivery's goal also prescribes the release itself.
#[test]
fn near_miss_controls_recover_after_a_timed_out_wait() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    let cases = [
        (
            "sibling-addresses",
            "near_miss_sibling",
            Outcome::Pass,
            None,
        ),
        (
            "sibling-addresses",
            "near_miss_sibling_nearest",
            Outcome::Fail,
            Some(FailureClass::WrongMessage),
        ),
        ("slow-delivery", "near_miss_slow", Outcome::Pass, None),
    ];
    for (task, mode, outcome, class) in cases {
        let dir = tempdir();
        let args = [
            "--task",
            task,
            "--agent",
            mode,
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ];
        let (mut command, _owner) = helper_command(mode, &args, &[]);
        let out = command.output().unwrap();
        let row = one_row(&out);
        let state = retained_state(dir.path(), &row);
        assert_eq!(
            row.agent_exit_status,
            Some(0),
            "{task}/{mode}: {}",
            stderr(&out)
        );
        assert_eq!(row.outcome, outcome, "{task}/{mode}: {row:?}");
        assert_eq!(row.failure_class, class, "{task}/{mode}: {row:?}");
        let events = state["events"].as_array().unwrap();
        let rejected: Vec<&str> = events
            .iter()
            .filter(|e| e["accepted"] != true)
            .map(|e| e["action"].as_str().unwrap())
            .collect();
        match class {
            None => assert!(
                rejected.is_empty(),
                "{task}/{mode}: no trap event: {events:?}"
            ),
            Some(_) => assert_eq!(rejected, ["foreign_token"], "{task}/{mode}: {events:?}"),
        }
        if task == "slow-delivery" {
            let released = events
                .iter()
                .any(|e| e["action"] == "release" && e["accepted"] == true);
            assert!(released, "{events:?}");
        }
    }
}

/// The controls again, through the recipe path (decision 0119): every
/// catalogue task — the default tier and the hard tier (proposed 0117) alike,
/// the same set the direct-runner controls above cover — run by
/// `matrix --brokkr` with the offline stand-in for the engine, so the oracle
/// must still pass every task and the do-nothing agent must still fail every
/// task with its declared idle class. The success check is proven able to fail
/// and able to pass through trial.sh, the row, score.sh and the matrix's
/// cross-check, not only through the direct runner. The set is asserted by
/// task id, so a new tier joins this proof instead of breaking a count.
#[test]
fn the_controls_hold_through_the_recipe_path() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let ids: Vec<String> = tasks::catalogue().into_iter().map(|t| t.id).collect();
    let mut expected = ids.clone();
    expected.sort();
    let agents = dir.path().join("agents.json");
    std::fs::write(
        &agents,
        json!([{"label": "oracle", "argv": []}, {"label": "null", "argv": []}]).to_string(),
    )
    .unwrap();
    let realm = brokkr_realm(dir.path());
    let out = brokkr_matrix("ok", &realm)
        .args(["--tasks", &ids.join(","), "--agents"])
        .arg(&agents)
        .args(["--trials", "1", "--artifacts-dir"])
        .arg(realm.join("artifacts"))
        .output()
        .unwrap();
    let rows: Vec<TrialResult> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 2 * ids.len(), "{}", stderr(&out));
    assert_eq!(
        out.status.code(),
        Some(1),
        "failures, no harness error\n{}",
        stderr(&out)
    );
    let of = |label: &str| -> Vec<&TrialResult> {
        rows.iter()
            .filter(|r| r.agent.as_deref() == Some(label))
            .collect()
    };
    let covered = |rows: &[&TrialResult]| -> Vec<String> {
        let mut seen: Vec<String> = rows.iter().map(|r| r.task.clone().unwrap()).collect();
        seen.sort();
        seen
    };
    let oracle = of("oracle");
    let null = of("null");
    assert_eq!(covered(&oracle), expected, "oracle covers every task once");
    assert_eq!(covered(&null), expected, "null covers every task once");
    let oracle_failures: Vec<_> = oracle
        .iter()
        .filter(|r| r.outcome != Outcome::Pass)
        .collect();
    assert!(
        oracle_failures.is_empty(),
        "oracle passes every task: {oracle_failures:?}"
    );
    for row in &null {
        assert_eq!(row.outcome, Outcome::Fail, "{row:?}");
        let task = tasks::find(row.task.as_deref().unwrap()).unwrap();
        assert_eq!(row.failure_class, Some(task.idle_class), "{row:?}");
        assert!(
            matches!(
                row.failure_class,
                Some(FailureClass::GaveUp | FailureClass::WaitTimeout)
            ),
            "{row:?}"
        );
    }
    assert!(rows.iter().all(|r| r.confinement == "unconfined"));
}

/// The hard tier's fixed-seed proof: oracle and null through the real
/// `trial --seed` path on four fixed public seeds. The layout is the only
/// seed-dependent material, so each seed is a different trap shape.
#[test]
fn hard_tasks_hold_on_fixed_seeds() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    const SEEDS: [&str; 4] = [
        "0000000000000000000000000000000000000000000000000000000000000001",
        "1111111111111111111111111111111111111111111111111111111111111111",
        "a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5",
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    ];
    let hard = tasks::tier_ids(tasks::Tier::Hard);
    assert_eq!(hard, ["otp-backlog", "recipient-crowd", "digest-lookalike"]);
    for id in &hard {
        let task = tasks::find(id).unwrap();
        for seed in SEEDS {
            let dir = tempdir();
            let (row, out) = trial(&[
                "--task",
                id,
                "--agent",
                "oracle",
                "--seed",
                seed,
                "--fixture-time",
                "2026-11-01T12:00:00Z",
                "--artifacts-dir",
                dir.path().to_str().unwrap(),
            ]);
            assert_eq!(row.outcome, Outcome::Pass, "{id}@{seed}: {}", stderr(&out));
            let state = retained_state(dir.path(), &row);
            assert!(
                state["deliveries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|d| d["status"] == "delivered"),
                "{id}@{seed}"
            );
            let check = cli()
                .args(["check", "--task", id, "--state"])
                .arg(dir.path().join(&row.trial_id).join("state.json"))
                .output()
                .unwrap();
            assert_eq!(check.status.code(), Some(0), "{id}@{seed}");

            let dir = tempdir();
            let (row, out) = trial(&[
                "--task",
                id,
                "--agent",
                "null",
                "--seed",
                seed,
                "--fixture-time",
                "2026-11-01T12:00:00Z",
                "--artifacts-dir",
                dir.path().to_str().unwrap(),
            ]);
            assert_eq!(row.outcome, Outcome::Fail, "{id}@{seed}: {}", stderr(&out));
            assert_eq!(row.failure_class, Some(task.idle_class), "{id}@{seed}");
            assert_eq!(task.idle_class, FailureClass::GaveUp);
            let state = retained_state(dir.path(), &row);
            assert!(no_goal_completed(&state), "{id}@{seed}: {state}");
        }
    }
}

/// AC8: the backlog's resend is closed and harmless. A trial that asks for a
/// fresh code, meets the limit, and then verifies the current one passes; the
/// refused resend is recorded unaccepted with no category and mails nothing.
#[test]
fn the_backlog_resend_is_refused_and_harmless() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    let dir = tempdir();
    let (row, out) = helper_trial(
        "hard_resend",
        &[
            "--task",
            "otp-backlog",
            "--agent",
            "hard_resend",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{row:?}\n{}", stderr(&out));
    let state = retained_state(dir.path(), &row);
    let deliveries = state["deliveries"].as_array().unwrap();
    assert_eq!(deliveries.len(), 12, "no thirteenth OTP delivery");
    let resends: Vec<_> = state["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["action"] == "otp_resend")
        .collect();
    assert_eq!(resends.len(), 1);
    assert_eq!(resends[0]["accepted"], false);
    assert!(resends[0]["category"].is_null(), "{}", resends[0]);
}

/// The expert tier's fixed-seed proof (AC10, AC11): oracle and null through
/// the real `trial --seed` path on four fixed public seeds, and the oracle's
/// measured cost equal to the pinned difficulty proxy on every seed.
#[test]
fn expert_tasks_hold_on_fixed_seeds() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    const SEEDS: [&str; 4] = [
        "0000000000000000000000000000000000000000000000000000000000000001",
        "1111111111111111111111111111111111111111111111111111111111111111",
        "a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5",
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    ];
    let expert = tasks::tier_ids(tasks::Tier::Expert);
    assert_eq!(
        expert,
        [
            "otp-correction",
            "digest-correction",
            "otp-late",
            "confirm-attachment",
            "confirm-injection"
        ]
    );
    for id in &expert {
        let task = tasks::find(id).unwrap();
        let proxy = tasks::EXPERT_DIFFICULTY
            .iter()
            .find(|d| d.id == id)
            .unwrap();
        for seed in SEEDS {
            let dir = tempdir();
            let (row, out) = trial(&[
                "--task",
                id,
                "--agent",
                "oracle",
                "--seed",
                seed,
                "--fixture-time",
                "2026-11-01T12:00:00Z",
                "--artifacts-dir",
                dir.path().to_str().unwrap(),
            ]);
            assert_eq!(row.outcome, Outcome::Pass, "{id}@{seed}: {}", stderr(&out));
            let state = retained_state(dir.path(), &row);
            assert!(
                state["deliveries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|d| d["status"] == "delivered"),
                "{id}@{seed}"
            );
            let check = cli()
                .args(["check", "--task", id, "--state"])
                .arg(dir.path().join(&row.trial_id).join("state.json"))
                .output()
                .unwrap();
            assert_eq!(check.status.code(), Some(0), "{id}@{seed}");

            let dir = tempdir();
            let (row, out) = trial(&[
                "--task",
                id,
                "--agent",
                "null",
                "--seed",
                seed,
                "--fixture-time",
                "2026-11-01T12:00:00Z",
                "--artifacts-dir",
                dir.path().to_str().unwrap(),
            ]);
            assert_eq!(row.outcome, Outcome::Fail, "{id}@{seed}: {}", stderr(&out));
            assert_eq!(row.failure_class, Some(task.idle_class), "{id}@{seed}");
            assert_eq!(task.idle_class, FailureClass::GaveUp);
            let state = retained_state(dir.path(), &row);
            assert!(no_goal_completed(&state), "{id}@{seed}: {state}");

            // The oracle's cost, measured by its own counters through the
            // test-only channel, equals the pinned proxy.
            let dir = tempdir();
            let cost_file = dir.path().join("cost.json");
            let (row, out) = helper_trial(
                "expert_cost",
                &[
                    "--task",
                    id,
                    "--agent",
                    "expert_cost",
                    "--seed",
                    seed,
                    "--fixture-time",
                    "2026-11-01T12:00:00Z",
                    "--artifacts-dir",
                    dir.path().to_str().unwrap(),
                ],
                &[(MARKER_ENV, cost_file.to_str().unwrap())],
            );
            assert_eq!(row.outcome, Outcome::Pass, "{id}@{seed}: {}", stderr(&out));
            let cost: Value = serde_json::from_slice(&std::fs::read(&cost_file).unwrap()).unwrap();
            assert_eq!(
                cost,
                json!({
                    "mcp_tool_calls": proxy.oracle_mcp_tool_calls,
                    "app_requests": proxy.oracle_app_requests,
                }),
                "{id}@{seed}: the measured cost is the pinned proxy"
            );
        }
    }
}

/// AC13: `otp-correction` closes the resend, harmlessly. A trial that asks
/// for a fresh code, meets the limit, and then applies the correction passes;
/// the refused resend is unaccepted, has no category and mails nothing.
#[test]
fn the_corrections_resend_is_refused_and_harmless() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    let dir = tempdir();
    let (row, out) = helper_trial(
        "hard_resend",
        &[
            "--task",
            "otp-correction",
            "--agent",
            "hard_resend",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{row:?}\n{}", stderr(&out));
    let state = retained_state(dir.path(), &row);
    let deliveries = state["deliveries"].as_array().unwrap();
    assert_eq!(deliveries.len(), 2, "no third OTP delivery");
    let resends: Vec<_> = state["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["action"] == "otp_resend")
        .collect();
    assert_eq!(resends.len(), 1);
    assert_eq!(resends[0]["accepted"], false);
    assert!(resends[0]["category"].is_null(), "{}", resends[0]);
}

/// AC13: `otp-late`'s resend stays recoverable. Two resends mail two fresh
/// codes and two reminders (six deliveries, all delivered), every resend is
/// accepted, and the newest fresh code passes without a trap event.
#[test]
fn repeated_late_resends_stay_recoverable() {
    let _trial_scope = trial_scope();
    swarmail_bin();
    let dir = tempdir();
    let (row, out) = helper_trial(
        "expert_double_resend",
        &[
            "--task",
            "otp-late",
            "--agent",
            "expert_double_resend",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{row:?}\n{}", stderr(&out));
    let state = retained_state(dir.path(), &row);
    let deliveries = state["deliveries"].as_array().unwrap();
    assert_eq!(deliveries.len(), 6);
    assert!(deliveries.iter().all(|d| d["status"] == "delivered"));
    let kinds: Vec<&str> = deliveries
        .iter()
        .map(|d| d["message"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        [
            "otp-1",
            "otp-2",
            "otp-4",
            "otp-reminder",
            "otp-5",
            "otp-reminder"
        ]
    );
    let events = state["events"].as_array().unwrap();
    let resends = events.iter().filter(|e| e["action"] == "otp_resend");
    assert_eq!(resends.clone().count(), 2);
    assert!(resends.into_iter().all(|e| e["accepted"] == true));
    assert!(
        events.iter().all(|e| e["accepted"] == true),
        "no trap event: {events:?}"
    );
}
