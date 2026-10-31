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
//! The cross-trial proof comes in two halves that share one probe. The
//! always-on half (AC-S1) shows that, with no boundary, a bare same-uid
//! helper given only a victim runner's PID reads the victim agent's `EVAL_*`
//! from `/proc`, completes its signup through the real MCP and fixture, and
//! the victim row is `pass`: the leak is observed, not alleged. The live half
//! (AC-L1) runs a hostile probe as trial B's agent inside the boundary
//! against a live idle victim A and requires every route to be denied.
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

// ---- AC-S1: the leak, observed on every host -------------------------------

/// A bare same-uid helper is given ONLY the victim runner's PID. It finds the
/// victim agent among that runner's descendants (never other sessions), reads
/// its `EVAL_*` environment from `/proc`, and completes signup through the
/// real MCP and fixture. The idle victim then passes without having done
/// anything. This documents, in the repository and on purpose, that without
/// the boundary a same-uid neighbour wins: a later denial is therefore an
/// observation against a demonstrated attack, not a cooperating oracle.
#[test]
fn the_leak_is_live_a_same_uid_helper_makes_an_idle_victim_pass() {
    let _scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("victim-ready");
    let (mut command, _wrapper) = helper_command(
        "held_idle",
        &["--task", "signup", "--agent", "idle-victim"],
        &[(MARKER_ENV, ready.to_str().unwrap())],
    );
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let victim = ChildGuard {
        child: Some(command.spawn().unwrap()),
    };
    eventually("the idle victim to be ready", || ready.exists());

    // The helper is a separate process of the same uid with no EVAL_*
    // environment of its own, handed the runner's PID and nothing else.
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "helper_agent", "--nocapture", "--test-threads=1"])
        .arg(format!("victim-runner={}", victim.pid()))
        .env(HELPER_ENV, "leak_signup")
        .env_remove("EVAL_APP_URL")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "the same-uid helper did not complete the victim's signup \
         (2 usage, 3 victim agent not found, 4 environment unusable, 5 signup failed): {}",
        String::from_utf8_lossy(&out.stderr)
    );

    std::fs::write(ready.with_extension("release"), b"release").unwrap();
    let out = victim.wait_with_output();
    let row = one_row(&out);
    assert_eq!(
        row.outcome,
        Outcome::Pass,
        "the idle victim should pass through the leak: {}",
        stderr(&out)
    );
    assert_eq!(row.failure_class, None);
    assert_eq!(row.concurrency, 1);
}

/// The hostile probe is not blind. Aimed at an UNISOLATED idle victim it must
/// report the leak (route 10: the victim's agent environment is visible in
/// `/proc`); aimed at nothing it must say it is blind (30) rather than report
/// that everything was denied. Without this, the live "every route denied"
/// result could be an artefact of a probe that cannot see.
#[test]
fn the_hostile_probe_reports_a_leak_where_there_is_one_and_is_never_blind() {
    let _scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("victim-ready");
    let (mut command, _wrapper) = helper_command(
        "held_idle",
        &["--task", "signup", "--agent", "idle-victim"],
        &[(MARKER_ENV, ready.to_str().unwrap())],
    );
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let victim = ChildGuard {
        child: Some(command.spawn().unwrap()),
    };
    eventually("the idle victim to be ready", || ready.exists());
    let (agent, vars) = find_victim_agent(victim.pid()).expect("the victim agent is visible");

    // The probe's own world: two loopback listeners and a full agent
    // environment, distinct from the victim's.
    let app = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let own = |command: &mut std::process::Command| {
        command
            .args(["--exact", "helper_agent", "--nocapture", "--test-threads=1"])
            .env(HELPER_ENV, "cross_trial_probe")
            .env("EVAL_TASK_ID", "signup")
            .env(
                "EVAL_APP_URL",
                format!("http://{}", app.local_addr().unwrap()),
            )
            .env(
                "EVAL_MCP_URL",
                format!("http://{}/mcp", mcp.local_addr().unwrap()),
            )
            .env("EVAL_IDENTITY", "probe@example.test")
            .env("EVAL_INBOX", "probe")
            .env("EVAL_GOAL", "none")
            .env("EVAL_PROMPT_FILE", dir.path().join("probe-prompt"))
            .env("EVAL_DEADLINE_MS", "1000")
            .stdin(Stdio::null());
    };
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    own(&mut command);
    let out = command
        .args(victim_args(victim.pid(), agent.pid, &vars))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(10),
        "the probe must see the unisolated victim: {}",
        probe_route(out.status.code())
    );

    // Given no victim at all, the probe refuses to call that a denial.
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    own(&mut command);
    let out = command.output().unwrap();
    assert_eq!(out.status.code(), Some(91));

    // And with no agent environment of its own it is blind, not satisfied.
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "helper_agent", "--nocapture", "--test-threads=1"])
        .args(victim_args(victim.pid(), agent.pid, &vars))
        .env(HELPER_ENV, "cross_trial_probe")
        .env_remove("EVAL_APP_URL")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(30));
    send_signal(victim.pid(), libc::SIGTERM);
    let _ = victim.wait_with_output();
}

// ---- live proofs: need unprivileged user namespaces -----------------------

/// A trial whose agent is the helper in `mode`, run inside the boundary.
fn isolated_helper(mode: &str) -> (std::process::Command, tempfile::TempDir) {
    isolated_helper_with(mode, &[])
}

/// The same, with `agent_args` appended to the wrapper's argv. The wrapper
/// forwards them to the helper (libtest treats them as inert name filters).
fn isolated_helper_with(
    mode: &str,
    agent_args: &[String],
) -> (std::process::Command, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let me = std::env::current_exe().unwrap();
    let wrapper = dir.path().join("agent-wrapper.sh");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport {HELPER_ENV}='{mode}'\nexec '{}' --exact helper_agent --nocapture --test-threads=1 \"$@\"\n",
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
        .arg(&wrapper)
        .args(agent_args);
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

// ---- AC-L1: the hostile same-uid agent against a live idle victim ----------

/// Wrap `command` so it runs in a private mount namespace of the same uid in
/// which `/tmp/swarmail-evals-<uid>`, the per-uid admission rendezvous, is a
/// fresh tmpfs. Victim A's controller keeps the real admission (0076 is
/// untouched: no production bypass exists); trial B's controller sees an
/// empty rendezvous and is admitted in its own view. This is a test-side
/// arrangement only. It maps the caller's own uid to itself, so B is the same
/// uid as A, and drops the namespace's capabilities before exec so the
/// controller starts as any unprivileged process would. Needs util-linux
/// `unshare` (--map-current-user, --keep-caps) and `setpriv`; if either is
/// missing B does not start and the test fails loudly rather than passing.
fn with_private_admission(command: std::process::Command) -> std::process::Command {
    // SAFETY: geteuid has no preconditions.
    let uid = unsafe { libc::geteuid() };
    let rendezvous = format!("/tmp/swarmail-evals-{uid}");
    std::fs::create_dir_all(&rendezvous).unwrap();
    let mut wrapped = std::process::Command::new("unshare");
    wrapped
        .args(["--user", "--map-current-user", "--keep-caps", "--mount", "--"])
        .args([
            "sh",
            "-c",
            "mount -t tmpfs -o mode=0700 tmpfs \"$0\" && exec setpriv --ambient-caps=-all --inh-caps=-all \"$@\"",
        ])
        .arg(&rendezvous)
        .arg(command.get_program());
    wrapped.args(command.get_args());
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => wrapped.env(key, value),
            None => wrapped.env_remove(key),
        };
    }
    wrapped.stdin(Stdio::null());
    wrapped
}

#[test]
#[ignore = "trial-isolation: needs unprivileged user namespaces"]
fn a_hostile_same_uid_agent_in_trial_b_cannot_reach_trial_a() {
    require_boundary();
    let _scope = trial_scope();

    // Victim A: an isolated trial whose agent just sleeps, held under the
    // deadline. The trusted test reads A's coordinates from the HOST /proc.
    let (victim, agent) = idle_trial();
    let vars = proc_environ(agent.pid).unwrap_or_else(|| {
        panic!(
            "the trusted test cannot read the victim agent's environment from the host /proc \
             (pid {}); the live proof needs those coordinates",
            agent.pid
        )
    });
    let victim_args = victim_args(victim.pid(), agent.pid, &vars);
    assert!(identity_alive(&agent));

    // Trial B: isolated too, its agent the hostile probe, A's coordinates in
    // argv, admitted in its own view of the rendezvous.
    let (command, _owner) = isolated_helper_with("cross_trial_probe", &victim_args);
    let mut command = with_private_admission(command);
    let out = command
        .output()
        .expect("the private-admission launcher runs");
    let row = one_row(&out);
    assert_eq!(
        row.concurrency,
        1,
        "trial B did not run as a trial: {}",
        stderr(&out)
    );
    let code = agent_codes(&row).first().copied().flatten();
    assert_eq!(
        code,
        Some(0),
        "trial B reached trial A: {} (exit {code:?})\n{}",
        probe_route(code),
        stderr(&out)
    );

    // Trial A was alive throughout; now its agent is killed from outside.
    // Trial A's row must be the idle-class failure: never a pass, never
    // budget, never a harness error.
    assert!(identity_alive(&agent), "trial A's agent died during B");
    send_signal(agent.pid, libc::SIGKILL);
    let out = victim.wait_with_output();
    let row = one_row(&out);
    let idle = swarmail_evals::tasks::find("signup").unwrap().idle_class;
    assert_eq!(row.outcome, Outcome::Fail, "{}", stderr(&out));
    assert_eq!(row.failure_class, Some(idle), "{}", stderr(&out));
}
