//! Trial isolation (proposed decision 0115; issue #221) — prepared, not
//! activated. Concurrency stays off; nothing here lets two trials overlap.
//! The boundary is bubblewrap (revision 2026-11-01): the controller invokes
//! `/usr/bin/bwrap`; nothing here builds a namespace itself.
//!
//! Two kinds of test live here, and the line between them is the point:
//!
//! * **Always on, any host.** The precondition refusals run through the real
//!   CLI, the hidden flags stay hidden, concurrency stays refused, and an
//!   unavailable boundary refuses by name and never falls back to an
//!   unisolated run. The leak (AC-S1) is observed against an unisolated
//!   victim.
//! * **Boundary-dependent, `#[ignore]`d with a named reason** (`NEEDS_BWRAP`).
//!   They need a host whose `/usr/bin/bwrap` (at least 0.11.1) can create user,
//!   PID, network and IPC namespaces, and run only when asked for
//!   (`-- --ignored`). Each first probes with a real null-agent
//!   `trial --isolated`; if the boundary cannot be built it prints
//!   `SKIP trial-isolation: boundary_unavailable:<step>` and FAILS. It never
//!   passes without the boundary. The gate inventory never passes
//!   `--ignored`, so a delivery seat stays green with them shown as ignored.
//!   AC-L1, AC-L2 and AC-L3 are proven only by running them (below) on such a
//!   host; AC-L4 (provider egress) is an open question for the operator and
//!   has no test.
//!
//! The cross-trial proof comes in two halves that share one probe. The
//! always-on half (AC-S1) shows that, with no boundary, a bare same-uid
//! helper given only a victim runner's PID reads the victim agent's `EVAL_*`
//! from `/proc`, completes its signup through the real MCP and fixture, and
//! the victim row is `pass`: the leak is observed, not alleged. The live half
//! (AC-L1) runs a hostile probe as trial B's agent inside the boundary
//! against a live idle victim A and requires every route to be denied.
//!
//! Proof command, from the repository root on a capable host (deterministic:
//! null and oracle built-ins, `/usr/bin/sleep`, and scripted agents from this
//! test executable; no provider, no paid call; every isolated trial carries a
//! deadline of at most 30 s):
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
    let steps = ["platform", "bwrap_absent", "bwrap_version", "bwrap_probe"];
    let step = reason
        .strip_prefix("boundary_unavailable:")
        .unwrap_or_else(|| panic!("not a boundary refusal: {reason}"));
    assert!(steps.contains(&step), "outside the closed set: {reason}");

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

// ---- live proofs: need a bwrap that can create the namespaces -------------

use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use swarmail_evals::isolation;

/// The runner executable as a controller reports it (`current_exe`).
fn runner_exe() -> PathBuf {
    std::fs::canonicalize(exe()).unwrap()
}

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
        .args(["--deadline-ms", "20000"])
        .args(["--task", "signup", "--agent", "probe", "--"])
        .arg(&wrapper)
        .args(agent_args);
    (command, dir)
}

#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
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

fn observer_check(code: Option<i32>) -> &'static str {
    match code {
        Some(0) => "every check held; a nested user namespace was refused",
        Some(100) => "every check held; a nested user namespace was granted (observed)",
        Some(40) => "the observer was not given its arguments",
        Some(41) => "the uid is not the caller's",
        Some(42) => "/proc shows the host (a host PID, or the host's init as PID 1)",
        Some(43) => "an interface other than lo exists",
        Some(44) => "lo is unusable",
        Some(45) => "a listed host path is present",
        Some(46) => "/tmp is not the bounded tmpfs",
        Some(47) => "a foreign environment key reached the agent",
        Some(48) => "a stray descriptor reached the agent",
        Some(49) => "capabilities are not empty, or NoNewPrivs is not 1",
        _ => "the observer did not report",
    }
}

#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn the_boundary_looks_as_designed_from_inside() {
    require_boundary();
    let _scope = trial_scope();
    use std::os::fd::AsRawFd;

    // A canary file under the HOST /tmp, and a canary descriptor onto it that
    // the controller is handed (not close-on-exec) and must not pass on.
    let canary = tempfile::NamedTempFile::new_in("/tmp").unwrap();
    let held = std::fs::File::open(canary.path()).unwrap();
    let raw = held.as_raw_fd();
    const CANARY_FD: i32 = 41;

    // Host paths that must be absent inside — except any that is a parent of
    // a file the boundary has to bind, which exists as an empty mount point.
    let bound = [
        runner_exe(),
        swarmail_bin().to_path_buf(),
        std::env::current_exe().unwrap(),
    ];
    let home = std::env::var("HOME").unwrap_or_default();
    let mut candidates: Vec<String> = [
        "/root", "/run", "/var", "/mnt", "/media", "/opt", "/srv", "/boot", "/sys",
    ]
    .map(String::from)
    .into();
    if !home.is_empty() {
        for dotfile in [".ssh", ".gnupg", ".claude", ".config", ".cache", ".local"] {
            candidates.push(format!("{home}/{dotfile}"));
        }
    }
    candidates.retain(|c| !bound.iter().any(|b| b.starts_with(c)));
    candidates.push(canary.path().display().to_string());

    // Host PIDs the sandbox's /proc must not list (large enough never to
    // coincide with a PID counted from 1 inside).
    let mut host_pids: Vec<String> = std::fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<i32>().ok())
        .filter(|pid| *pid >= 500)
        .map(|pid| pid.to_string())
        .collect();
    host_pids.truncate(3000);
    assert!(!host_pids.is_empty());

    // SAFETY: geteuid has no preconditions.
    let uid = unsafe { libc::geteuid() };
    let args = vec![
        format!("uid={uid}"),
        format!("host-pids={}", host_pids.join(",")),
        format!("absent={}", candidates.join(",")),
        format!("canary-fd={CANARY_FD}"),
        format!("tmp-bytes={}", isolation::TMP_BYTES),
    ];
    let (mut command, _owner) = isolated_helper_with("boundary_observe", &args);
    // SAFETY: the closure runs between fork and exec and makes one dup2.
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(raw, CANARY_FD) < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let out = command.output().unwrap();
    let row = one_row(&out);
    let code = agent_codes(&row).first().copied().flatten();
    eprintln!("observer: {} (exit {code:?})", observer_check(code));
    assert!(
        matches!(code, Some(0) | Some(100)),
        "the boundary is not as designed: {} (exit {code:?})\n{}",
        observer_check(code),
        stderr(&out)
    );
    drop(held);
}

/// Spawn an isolated trial through the real CLI whose agent sleeps, and wait
/// until it runs. The guard is the controller.
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
            "30000",
            "--",
            "/usr/bin/sleep",
            "1000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let trial = ChildGuard {
        child: Some(command.spawn().unwrap()),
    };
    let agent = wait_for_sleeping_agent(trial.pid());
    (trial, agent)
}

fn wait_for_sleeping_agent(root: i32) -> ProcessIdentity {
    let mut agent = None;
    eventually("the isolated agent", || {
        agent = process_descendants(root).into_iter().find(|p| {
            std::fs::read(format!("/proc/{}/cmdline", p.pid))
                .is_ok_and(|c| c.starts_with(b"/usr/bin/sleep"))
        });
        agent.is_some()
    });
    agent.unwrap()
}

#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn killing_the_controller_leaves_no_trial_process_alive() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, _agent) = idle_trial();
    let family = process_descendants(trial.pid());
    assert!(
        family.len() >= 3,
        "bwrap, its reaper, the runner, the agent: {family:?}"
    );
    send_signal(trial.pid(), libc::SIGKILL);
    let _ = trial.wait_with_output();
    eventually("every trial process to die", || {
        family.iter().all(|p| !identity_alive(p))
    });
    // No orphan survives to watch the next trial.
    let (next, _) = idle_trial();
    assert!(family.iter().all(|p| !identity_alive(p)));
    send_signal(next.pid(), libc::SIGKILL);
}

#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn killing_the_runner_leaves_no_agent_alive_and_the_controller_names_the_missing_row() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent) = idle_trial();
    let family = process_descendants(trial.pid());
    let runner = isolation::runner_pid(trial.pid(), &runner_exe())
        .expect("the runner is below the controller");
    send_signal(runner, libc::SIGKILL);
    eventually("the agent to die with its namespace", || {
        !identity_alive(&agent)
    });
    let out = trial.wait_with_output();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError, "{}", stderr(&out));
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("harness error: isolated_trial_no_row"),
        "{}",
        stderr(&out)
    );
    eventually("every trial process to die", || {
        family.iter().all(|p| !identity_alive(p))
    });
}

#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn an_interrupt_emits_the_runners_interrupted_row_and_leaves_nothing_alive() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent) = idle_trial();
    let family = process_descendants(trial.pid());
    send_signal(trial.pid(), libc::SIGTERM);
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
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
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

/// Victim A: an isolated signup trial whose agent just sleeps, launched by the
/// trusted test through the library's own argv builder — the test is A's
/// controller and holds no host admission for it. Returns A's bwrap (the
/// guard), A's agent and A's runner, as host PIDs read from the host `/proc`.
fn idle_victim() -> (ChildGuard, ProcessIdentity, i32) {
    let runner = runner_exe();
    let request = isolation::Request {
        swarmail_bin: std::fs::canonicalize(swarmail_bin()).unwrap(),
        runner: runner.clone(),
        agent: Some(PathBuf::from("/usr/bin/sleep")),
        expose: Vec::new(),
    };
    let mut inner: Vec<OsString> = vec![runner.clone().into_os_string()];
    inner.extend(
        [
            "trial",
            "--task=signup",
            "--agent=idle",
            "--deadline-ms=30000",
            "--",
            "/usr/bin/sleep",
            "1000",
        ]
        .map(OsString::from),
    );
    let victim = launch_victim(&request, &inner);
    let agent = wait_for_sleeping_agent(victim.pid());
    let runner_pid =
        isolation::runner_pid(victim.pid(), &runner).expect("victim A's runner is below its bwrap");
    (victim, agent, runner_pid)
}

#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn a_hostile_same_uid_agent_in_trial_b_cannot_reach_trial_a() {
    require_boundary();
    let _scope = trial_scope();

    // Victim A is idle and alive; the trusted test reads its coordinates from
    // the HOST /proc (same uid, host namespaces).
    let (victim, agent, runner) = idle_victim();
    let vars = proc_environ(agent.pid).unwrap_or_else(|| {
        panic!(
            "the trusted test cannot read the victim agent's environment from the host /proc \
             (pid {}); the live proof needs those coordinates",
            agent.pid
        )
    });
    let victim_args = victim_args(runner, agent.pid, &vars);
    assert!(identity_alive(&agent));

    // Trial B: the real `trial --isolated` CLI, its agent the hostile probe,
    // A's coordinates in argv. Under separate network namespaces both trials
    // bind ephemeral ports on their own 127.0.0.1, so a numeric coincidence
    // can make the probe test B's own service; the probe reports that as a
    // void (90) and the run is repeated — twice at most, then it fails by name.
    let mut voids = 0;
    let (code, out) = loop {
        let (mut command, _owner) = isolated_helper_with("cross_trial_probe", &victim_args);
        let out = command.output().expect("trial B runs");
        let row = one_row(&out);
        assert_eq!(
            row.concurrency,
            1,
            "trial B did not run as a trial: {}",
            stderr(&out)
        );
        let code = agent_codes(&row).first().copied().flatten();
        if code == Some(90) {
            voids += 1;
            assert!(
                voids <= 2,
                "the probe's own ports coincided with trial A's three times running: the proof is void"
            );
            continue;
        }
        break (code, out);
    };
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
