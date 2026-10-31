//! Trial isolation (proposed decision 0110; issue #221) — prepared, not
//! activated. Concurrency stays off; nothing here lets two trials overlap.
//!
//! Two kinds of test live here, and the line between them is the point:
//!
//! * **Always on, any host.** The precondition refusals run through the real
//!   CLI, the hidden flags stay hidden, concurrency stays refused, and an
//!   unavailable boundary refuses by name and never falls back to an
//!   unisolated run.
//! * **Namespace-dependent, `#[ignore]`d with a named reason.** They need a
//!   host that permits unprivileged user namespaces, and run only when asked
//!   for (`-- --ignored`). Each first probes with a real null-agent
//!   `trial --isolated`; if the boundary cannot be built it prints
//!   `SKIP trial-isolation: boundary_unavailable:<op>:<ERRNO>` and FAILS. It
//!   never passes without the boundary. The gate inventory never passes
//!   `--ignored`, so a delivery seat stays green with them shown as ignored.
//!
//! The hostile cross-trial proof (a same-uid agent in one trial against a
//! live idle victim in another) is specified in specs/trial-isolation/spec.md
//! (AC-S1, AC-L1) and is NOT implemented in this run; see the spec's
//! "Outstanding" section.
//!
//! Proof command, from the repository root on a capable host:
//!
//!   cargo build --locked --bin swarmail
//!   cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
//!     cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1

mod common;

use common::*;
use std::process::Stdio;
use swarmail_evals::state::FailureClass;
use swarmail_evals::tasks::Outcome;

#[test]
fn helper_agent() {
    maybe_run_helper();
}

fn sentinel_agent(dir: &std::path::Path) -> (std::path::PathBuf, [String; 2]) {
    let sentinel = dir.join("agent-ran");
    let argv = ["/usr/bin/touch".to_string(), sentinel.display().to_string()];
    (sentinel, argv)
}

#[test]
fn isolated_preconditions_refuse_through_the_real_cli() {
    let _scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let (sentinel, agent) = sentinel_agent(dir.path());
    let artifacts = dir.path().join("artifacts");
    let artifacts = artifacts.to_str().unwrap();
    let cases: Vec<(Vec<&str>, bool, &str)> = vec![
        (vec![], false, "isolation_requires_swarmail_bin"),
        (
            vec!["--artifacts-dir", artifacts],
            true,
            "isolation_artifacts_unsupported",
        ),
        (
            vec!["--wrapper-profile", "claude_2_1_310"],
            true,
            "isolation_wrapper_unsupported",
        ),
        (
            vec!["--isolated-expose", "relative/file"],
            true,
            "isolation_expose_invalid",
        ),
        (
            vec!["--isolated-expose", "/definitely/not/here"],
            true,
            "isolation_expose_invalid",
        ),
        // The public refusal keeps its meaning and wins, with or without
        // the Swarmail selector.
        (vec!["--concurrency", "2"], false, "concurrency_unsupported"),
    ];
    for (extra, has_bin, reason) in cases {
        let mut command = cli();
        if !has_bin {
            command.env_remove(swarmail_evals::runner::SWARMAIL_BIN_ENV);
        }
        let out = command
            .args([
                "trial",
                "--task",
                "signup",
                "--agent",
                "probe",
                "--isolated",
            ])
            .args(&extra)
            .arg("--")
            .args(&agent)
            .output()
            .unwrap();
        let row = one_row(&out);
        assert_eq!(row.outcome, Outcome::HarnessError, "{extra:?}");
        assert_eq!(out.status.code(), Some(2), "{extra:?}");
        assert!(
            stderr(&out).contains(&format!("harness error: {reason}\n")),
            "{extra:?}: {}",
            stderr(&out)
        );
        assert!(row.agent_exits.is_empty());
        assert!(!sentinel.exists(), "{extra:?}: an agent was launched");
    }
    assert!(!std::path::Path::new(artifacts).exists());
}

#[test]
fn the_isolation_flags_stay_hidden() {
    let out = cli().args(["trial", "--help"]).output().unwrap();
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(help.contains("--concurrency"), "{help}");
    assert!(!help.contains("isolated"), "{help}");
}

#[test]
fn concurrency_stays_off_without_the_flag_too() {
    let _scope = trial_scope();
    let (row, out) = trial(&["--task", "signup", "--agent", "null", "--concurrency", "2"]);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(row.concurrency, 1);
    assert!(stderr(&out).contains("concurrency_unsupported"));
}

/// Where the boundary cannot be built the refusal is real, named, and no
/// agent runs; where it can, this asserts nothing about it (the ignored
/// proofs do). Either way there is no unisolated fallback to observe.
#[test]
fn an_unavailable_boundary_refuses_by_name_and_never_falls_back() {
    let Some(reason) = boundary_unavailable() else {
        return;
    };
    let parts: Vec<&str> = reason.split(':').collect();
    assert_eq!(parts.len(), 3, "{reason}");
    assert_eq!(parts[0], "boundary_unavailable");
    assert!(
        parts[1]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b == b'_')
    );
    assert!(parts[2].starts_with('E') && parts[2].bytes().all(|b| b.is_ascii_uppercase()));

    let _scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let (sentinel, agent) = sentinel_agent(dir.path());
    let out = cli()
        .args([
            "trial",
            "--isolated",
            "--task",
            "signup",
            "--agent",
            "probe",
            "--",
        ])
        .args(&agent)
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains(&reason), "{}", stderr(&out));
    assert!(!sentinel.exists(), "the agent ran without the boundary");
}

// ---- live proofs: need unprivileged user namespaces -----------------------

/// A trial whose agent is the helper in `mode`, run inside the boundary.
fn isolated_helper(mode: &str) -> (std::process::Command, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let me = std::env::current_exe().unwrap();
    let wrapper = dir.path().join("agent-wrapper.sh");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport {HELPER_ENV}='{mode}'\nexec '{}' --exact helper_agent --nocapture --test-threads=1\n",
            me.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = cli();
    command
        .args(["trial", "--isolated", "--isolated-expose"])
        .arg(&me)
        .args(["--task", "signup", "--agent", "probe", "--"])
        .arg(&wrapper);
    (command, dir)
}

#[test]
#[ignore = "trial-isolation: needs unprivileged user namespaces"]
fn a_fresh_home_and_the_trials_own_services_work_through_the_boundary() {
    require_boundary();
    let _scope = trial_scope();
    // Two isolated trials in a row: HOME is fresh and writable each time,
    // so the first one's sentinel is absent from the second.
    for _ in 0..2 {
        let (mut command, _owner) = isolated_helper("home_probe");
        let out = command.output().unwrap();
        let row = one_row(&out);
        assert_eq!(agent_codes(&row), [Some(0)], "{}", stderr(&out));
    }
    // The oracle completes signup inside the boundary through its own
    // Swarmail MCP and its own fixture.
    let (row, out) = trial(&["--isolated", "--task", "signup", "--agent", "oracle"]);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.concurrency, 1);
}

/// Spawn an isolated trial whose agent sleeps, and wait until it runs.
fn idle_trial() -> (ChildGuard, ProcessIdentity) {
    let mut command = cli();
    command
        .args([
            "trial",
            "--isolated",
            "--task",
            "signup",
            "--agent",
            "idle",
            "--deadline-ms",
            "120000",
            "--",
            "/usr/bin/sleep",
            "1000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let trial = ChildGuard {
        child: Some(command.spawn().unwrap()),
    };
    let root = trial.pid();
    let mut agent = None;
    eventually("the isolated agent", || {
        agent = process_descendants(root).into_iter().find(|p| {
            std::fs::read(format!("/proc/{}/cmdline", p.pid))
                .is_ok_and(|c| c.starts_with(b"/usr/bin/sleep"))
        });
        agent.is_some()
    });
    (trial, agent.unwrap())
}

#[test]
#[ignore = "trial-isolation: needs unprivileged user namespaces"]
fn killing_the_controller_leaves_no_trial_process_alive() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, _agent) = idle_trial();
    let family = process_descendants(trial.pid());
    assert!(family.len() >= 2, "init and agent: {family:?}");
    send_signal(trial.pid(), libc::SIGKILL);
    let _ = trial.wait_with_output();
    eventually("every trial process to die", || {
        family.iter().all(|p| !identity_alive(p))
    });
    // No orphan survives to watch the next trial.
    let (next, _) = idle_trial();
    send_signal(next.pid(), libc::SIGKILL);
}

#[test]
#[ignore = "trial-isolation: needs unprivileged user namespaces"]
fn killing_the_init_leaves_no_agent_alive_and_the_controller_follows() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent) = idle_trial();
    let controller = trial.pid();
    let init = process_descendants(controller)
        .into_iter()
        .find(|p| p.parent == controller)
        .expect("the init is the controller's child");
    send_signal(init.pid, libc::SIGKILL);
    eventually("the agent to die with its namespace", || {
        !identity_alive(&agent)
    });
    let out = trial.wait_with_output();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(
        out.status.signal(),
        Some(libc::SIGKILL),
        "controller mirrors"
    );
}

#[test]
#[ignore = "trial-isolation: needs unprivileged user namespaces"]
fn an_interrupt_emits_the_interrupted_row_and_leaves_nothing_alive() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent) = idle_trial();
    let family = process_descendants(trial.pid());
    send_signal(trial.pid(), libc::SIGINT);
    let out = trial.wait_with_output();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("the trial was interrupted"),
        "{}",
        stderr(&out)
    );
    eventually("every trial process to die", || {
        !identity_alive(&agent) && family.iter().all(|p| !identity_alive(p))
    });
}

#[test]
#[ignore = "trial-isolation: needs unprivileged user namespaces"]
fn an_agent_killed_from_outside_leaves_the_idle_class_not_a_pass() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent) = idle_trial();
    send_signal(agent.pid, libc::SIGKILL);
    let out = trial.wait_with_output();
    let row = one_row(&out);
    let idle = swarmail_evals::tasks::find("signup").unwrap().idle_class;
    assert_eq!(row.outcome, Outcome::Fail, "{}", stderr(&out));
    assert_eq!(row.failure_class, Some(idle));
    assert!(matches!(
        idle,
        FailureClass::GaveUp | FailureClass::WaitTimeout
    ));
}
