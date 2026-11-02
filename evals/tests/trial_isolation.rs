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
//!   victim. One always-on test needs the boundary itself,
//!   `an_isolated_null_trial_hands_over_its_row_and_exits` (AC-S10): where
//!   bwrap works it runs a real isolated null trial and passes only by the
//!   controller's actual exit with the runner's row; where it does not, it
//!   prints `SKIP trial-isolation: boundary_unavailable:<step>` and returns.
//!   The two tests whose subject is the *absent* boundary print
//!   `SKIP trial-isolation: boundary available` and return where it works.
//! * **Boundary-dependent, `#[ignore]`d with a named reason** (`NEEDS_BWRAP`).
//!   They need a host whose `/usr/bin/bwrap` (at least 0.11.1) can create user,
//!   PID, network and IPC namespaces, and run only when asked for
//!   (`-- --ignored`). Each first probes with a real null-agent
//!   `trial --isolated`; if the boundary cannot be built it prints the same
//!   `SKIP trial-isolation: boundary_unavailable:<step>` line and FAILS. It
//!   never passes without the boundary. The gate inventory never passes
//!   `--ignored`, so a delivery seat stays green with them shown as ignored.
//!   AC-L1, AC-L2 and AC-L3 are proven only by running them (below) on such a
//!   host; AC-L4 (provider egress) is an open question for the operator and
//!   has no test.
//!
//! The two rules side by side: an always-on test that needs a boundary skips
//! by name and returns; an ignored live proof skips by name and fails. Every
//! child that may run a boundary — the CLI, a victim's bwrap — is spawned
//! through `common::bounded` and held to 90 s: output on files, `SIGKILL` and
//! a named failure (`controller outlived its row`) at the bound, so the suite
//! finishes in bounded time on a host where bwrap works as well as in a seat
//! where it does not.
//!
//! The cross-trial proof comes in two halves that share one probe. The
//! always-on half (AC-S1) shows that, with no boundary, a bare same-uid
//! helper given only a victim runner's PID reads the victim agent's `EVAL_*`
//! from `/proc`, completes its signup through the real MCP and fixture, and
//! the victim row is `pass`: the leak is observed, not alleged. The live half
//! (AC-L1) runs a hostile probe as trial B's agent inside the boundary
//! against a live idle victim A and requires every route to be denied.
//!
//! What each platform proves. Everything that reads `/proc` is
//! `cfg(target_os = "linux")`: the leak proof (AC-S1) and its negative
//! control, the live proofs and the in-boundary observer. macOS compiles this
//! file and proves only that `--isolated` refuses `boundary_unavailable:platform`
//! (`macos_refuses_by_name_through_the_real_cli`); it claims nothing else,
//! and passes nothing vacuously.
//!
//! Proof command, from the repository root on a capable host (deterministic:
//! null and oracle built-ins, `/usr/bin/sleep`, and scripted agents from this
//! test executable; no provider, no paid call; every isolated trial carries a
//! deadline of at most 30 s and every child a 90 s bound; the dispatcher runs
//! it under a hard 20-minute limit, which is the acceptance gate of the pull
//! request):
//!
//!   cargo build --locked --bin swarmail
//!   cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
//!     cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1

mod common;

use common::*;
#[cfg(target_os = "linux")]
use std::process::Stdio;
#[cfg(target_os = "linux")]
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

/// A generated agent script that touches a sentinel file (on the host) if it
/// ever runs, plus that sentinel. It is a regular file in a fresh directory,
/// so it can be named by `--isolated-expose`.
fn sentinel_script(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let sentinel = dir.join("agent-ran");
    let script = dir.join("agent.sh");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", sentinel.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    (sentinel, script)
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
        command
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
            .args(&agent);
        let out = isolated_output(command);
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

/// The external-agent rule and the order of the refusals, through the real
/// CLI (AC-S3). Until the operator rules on provider egress (0115) an
/// external program runs isolated only if the caller exposed that very file.
#[test]
fn an_unexposed_external_agent_is_refused_and_the_order_is_the_documented_one() {
    let _scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let sentinel = dir.path().join("agent-ran");
    let artifacts = dir.path().join("artifacts");
    let artifacts = artifacts.to_str().unwrap();
    let touch = ["/usr/bin/touch".to_string(), sentinel.display().to_string()];
    let cases: Vec<(Vec<&str>, Vec<String>, &str)> = vec![
        // An external program nobody exposed.
        (
            vec![],
            touch.to_vec(),
            "isolation_external_agent_unsupported",
        ),
        (
            vec![],
            vec!["/bin/sh".into(), "-c".into(), "true".into()],
            "isolation_external_agent_unsupported",
        ),
        // Exposing something else does not admit it.
        (
            vec!["--isolated-expose", exe()],
            touch.to_vec(),
            "isolation_external_agent_unsupported",
        ),
        // The order: artifacts before an unresolvable program; a bad
        // exposure before an unexposed program; an unresolvable program is
        // its own named refusal.
        (
            vec!["--artifacts-dir", artifacts],
            vec!["/definitely/not/a/program".into()],
            "isolation_artifacts_unsupported",
        ),
        (
            vec!["--isolated-expose", "/definitely/not/here"],
            vec!["/bin/sh".into()],
            "isolation_expose_invalid",
        ),
        (
            vec![],
            vec!["/definitely/not/a/program".into()],
            "isolation_source_invalid",
        ),
    ];
    for (extra, agent, reason) in cases {
        let mut command = cli();
        command
            .args([
                "trial",
                "--isolated",
                "--task",
                "signup",
                "--agent",
                "probe",
            ])
            .args(&extra)
            .arg("--")
            .args(&agent);
        let out = isolated_output(command);
        let row = one_row(&out);
        assert_eq!(row.outcome, Outcome::HarnessError, "{extra:?} {agent:?}");
        assert_eq!(out.status.code(), Some(2), "{extra:?} {agent:?}");
        assert!(
            stderr(&out).contains(&format!("harness error: {reason}\n")),
            "{extra:?} {agent:?}: {}",
            stderr(&out)
        );
        assert!(row.agent_exits.is_empty());
        assert!(!sentinel.exists(), "{extra:?}: an agent was launched");
    }
    assert!(!std::path::Path::new(artifacts).exists());
}

#[test]
fn an_exposed_external_agent_passes_the_precondition_and_reaches_the_boundary() {
    // Where the boundary exists this would launch the agent for real, and the
    // live suite covers that; here the interest is the refusal by name.
    let Some(reason) = boundary_unavailable() else {
        eprintln!("SKIP trial-isolation: boundary available");
        return;
    };
    let _scope = trial_scope();
    let dir = tempdir();
    let (sentinel, script) = sentinel_script(dir.path());
    let mut command = cli();
    command
        .args([
            "trial",
            "--isolated",
            "--task",
            "signup",
            "--agent",
            "probe",
        ])
        .arg("--isolated-expose")
        .arg(&script)
        .arg("--")
        .arg(&script);
    let out = isolated_output(command);
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(out.status.code(), Some(2));
    // Not a precondition: the boundary itself is what refused.
    assert!(
        !stderr(&out).contains("harness error: isolation_"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains(&reason), "{}", stderr(&out));
    assert!(!sentinel.exists(), "the agent ran without the boundary");
}

#[cfg(not(target_os = "linux"))]
#[test]
fn macos_refuses_by_name_through_the_real_cli() {
    // bwrap is Linux-only: the whole of what this platform proves.
    let _scope = trial_scope();
    let out = null_probe();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(row.agent_exits.is_empty());
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("harness error: boundary_unavailable:platform\n"),
        "{}",
        stderr(&out)
    );
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
        eprintln!("SKIP trial-isolation: boundary available");
        return;
    };
    let steps = ["platform", "bwrap_absent", "bwrap_version", "bwrap_probe"];
    let step = reason
        .strip_prefix("boundary_unavailable:")
        .unwrap_or_else(|| panic!("not a boundary refusal: {reason}"));
    assert!(steps.contains(&step), "outside the closed set: {reason}");

    let _scope = trial_scope();
    let dir = tempdir();
    let (sentinel, script) = sentinel_script(dir.path());
    let mut command = cli();
    command
        .args([
            "trial",
            "--isolated",
            "--task",
            "signup",
            "--agent",
            "probe",
        ])
        .arg("--isolated-expose")
        .arg(&script)
        .arg("--")
        .arg(&script);
    let out = isolated_output(command);
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains(&reason), "{}", stderr(&out));
    assert!(!sentinel.exists(), "the agent ran without the boundary");
}

/// AC-S10: the controller hands over the runner's row and exits. A real
/// isolated null-agent trial through the real CLI, held to the 90 s bound by
/// [`bounded`], so it passes only by the controller's actual exit. The null
/// agent does nothing, so the trial fails honestly: the runner's own row, a
/// `fail` outcome and exit code 1 (never the controller's harness-error row,
/// which is exit 2) are what crossed the boundary. Where the boundary cannot
/// be built it prints the named SKIP line and returns; the ignored proofs
/// print it and fail instead. On the controller that lost the row (the live
/// proof of 2026-11-01) this is red: `isolated_trial_no_row`, then a process
/// that never exits.
#[cfg(target_os = "linux")]
#[test]
fn an_isolated_null_trial_hands_over_its_row_and_exits() {
    let _scope = trial_scope();
    let out = null_probe();
    if let Some(step) = refused_step(&out) {
        eprintln!("SKIP trial-isolation: {step}");
        return;
    }
    let row = one_row(&out);
    assert_ne!(
        row.outcome,
        Outcome::HarnessError,
        "the controller did not hand over the runner's row: {}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("isolated_trial_no_row"),
        "{}",
        stderr(&out)
    );
    assert_eq!(row.outcome, Outcome::Fail, "{}", stderr(&out));
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert_eq!(row.concurrency, 1);
}

/// AC-L3's diagnostic: `survivors` names a live process with its pid, state and
/// command line, and is empty once the process is gone and reaped.
#[cfg(target_os = "linux")]
#[test]
fn survivors_names_a_live_process_and_is_empty_once_it_is_gone() {
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new("/usr/bin/sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let pid = child.id() as i32;
    let family = vec![process_identity(pid).expect("the child is in the process table")];
    let named = survivors(&family);
    assert!(named.contains(&format!("pid {pid} state ")), "{named}");
    assert!(named.contains("cmdline /usr/bin/sleep 30"), "{named}");
    assert!(
        named.contains(&format!("PPid {}", std::process::id())),
        "{named}"
    );
    send_signal(pid, libc::SIGKILL);
    child.wait().unwrap();
    assert_eq!(survivors(&family), "");
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

/// AC-S12: the bounded runner fails by name instead of waiting. A child that
/// would run a minute is held to 200 ms: the wait panics with the bound in the
/// message, and the child is gone, not left running behind the failure.
#[cfg(target_os = "linux")]
#[test]
fn a_child_that_outlives_its_bound_fails_by_name_and_is_killed() {
    let dir = tempdir();
    let pid_file = dir.path().join("pid");
    let mut command = std::process::Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(format!(
            "echo $$ > '{}'; echo partial-row; exec /bin/sleep 60",
            pid_file.display()
        ))
        .stdin(Stdio::null());
    let guard = bounded(command, std::time::Duration::from_millis(200));
    let started = std::time::Instant::now();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        guard.wait_with_output();
    }));
    let message = match caught {
        Ok(()) => panic!("a child that outlived its bound was waited for"),
        Err(payload) => payload
            .downcast_ref::<String>()
            .cloned()
            .expect("the failure carries a message"),
    };
    assert!(
        message.contains("controller outlived its row after 200ms"),
        "{message}"
    );
    assert!(message.contains("partial-row"), "{message}");
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    let pid: i32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // SAFETY: signal 0 only probes for existence; the child was reaped.
    assert_ne!(unsafe { libc::kill(pid, 0) }, 0, "the child survived");
    // A child that exits in time is not disturbed.
    let mut command = std::process::Command::new("/bin/sh");
    command.args(["-c", "echo done; echo oops >&2; exit 3"]);
    let out = bounded(command, std::time::Duration::from_secs(30)).wait_with_output();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "done\n");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "oops\n");
}

// ---- AC-S1: the leak, observed on every host -------------------------------

/// A bare same-uid helper is given ONLY the victim runner's PID. It finds the
/// victim agent among that runner's descendants (never other sessions), reads
/// its `EVAL_*` environment from `/proc`, and completes signup through the
/// real MCP and fixture. The idle victim then passes without having done
/// anything. This documents, in the repository and on purpose, that without
/// the boundary a same-uid neighbour wins: a later denial is therefore an
/// observation against a demonstrated attack, not a cooperating oracle.
#[cfg(target_os = "linux")]
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
    let victim = ChildGuard::new(command.spawn().unwrap());
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
#[cfg(target_os = "linux")]
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
    let victim = ChildGuard::new(command.spawn().unwrap());
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

    // A port coincidence is void (90), never a verdict: if the "victim's"
    // MCP or fixture port is one this probe's own network namespace is
    // listening on, a connect would reach its own service. Here the probe's
    // two services are the listeners above, and the coordinates name them.
    let free_port = || {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let (app_port, mcp_port) = (
        app.local_addr().unwrap().port(),
        mcp.local_addr().unwrap().port(),
    );
    let coordinates = |app_port: u16, mcp_port: u16| {
        let vars = std::collections::BTreeMap::from([
            (
                "EVAL_APP_URL".to_string(),
                format!("http://127.0.0.1:{app_port}"),
            ),
            (
                "EVAL_MCP_URL".to_string(),
                format!("http://127.0.0.1:{mcp_port}/mcp"),
            ),
        ]);
        // No such process: only the ports can coincide.
        victim_args(i32::MAX, i32::MAX, &vars)
    };
    let exit_for = |args: Vec<String>| {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        own(&mut command);
        command.args(args).output().unwrap().status.code()
    };
    assert_eq!(
        exit_for(coordinates(free_port(), mcp_port)),
        Some(90),
        "the victim's MCP port is one of the probe's own listeners"
    );
    assert_eq!(
        exit_for(coordinates(app_port, free_port())),
        Some(90),
        "the victim's fixture port is one of the probe's own listeners"
    );
    // The void does not over-fire: with nothing listening on either port and
    // no victim to find, every route is denied.
    assert_eq!(exit_for(coordinates(free_port(), free_port())), Some(0));
    send_signal(victim.pid(), libc::SIGTERM);
    let _ = victim.wait_with_output();
}

/// The observer is not vacuous: run unisolated, it sees the host's `/proc`
/// and reports it (42) rather than passing, and the two walks it relies on
/// find what they look for (AC-L2). The live proof's green is then not an
/// artefact of an observer that cannot see.
#[cfg(target_os = "linux")]
#[test]
fn the_in_boundary_observer_reports_the_host_where_there_is_no_boundary() {
    let _scope = trial_scope();
    // SAFETY: geteuid has no preconditions.
    let uid = unsafe { libc::geteuid() };
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "helper_agent", "--nocapture", "--test-threads=1"])
        .args([
            format!("uid={uid}"),
            format!("host-pids={}", std::process::id()),
            "absent=/definitely/not/here".to_string(),
            "canary-fd=99".to_string(),
            "tmp-bytes=1".to_string(),
            "runner=/definitely/not/the/runner".to_string(),
            "bound=".to_string(),
        ])
        .env(HELPER_ENV, "boundary_observe")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    // The helper's own PID is visible in the host /proc it lists.
    assert_eq!(
        out.status.code(),
        Some(42),
        "{}",
        observer_check(out.status.code())
    );
    // With no arguments it says so instead of guessing.
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "helper_agent", "--nocapture", "--test-threads=1"])
        .env(HELPER_ENV, "boundary_observe")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(40));
}

#[cfg(target_os = "linux")]
#[test]
fn the_observers_directory_walks_find_what_they_look_for() {
    let dir = tempdir();
    let bound = dir.path().join("a/b/tool");
    std::fs::create_dir_all(bound.parent().unwrap()).unwrap();
    std::fs::write(&bound, b"x").unwrap();
    let files = [bound.clone()];
    // Only the bound file and the directories on the way to it: clean.
    assert_eq!(stray_under(dir.path(), &files), None);
    // A missing root is clean (the boundary may have no /home at all).
    assert_eq!(stray_under(&dir.path().join("nope"), &files), None);
    // Anything else is a stray: a sibling directory, a sibling file, a link.
    for make in [
        |d: &std::path::Path| std::fs::create_dir(d.join("a/c")).unwrap(),
        |d: &std::path::Path| std::fs::write(d.join("a/b/other"), b"y").unwrap(),
        |d: &std::path::Path| std::fs::write(d.join("loose"), b"y").unwrap(),
        |d: &std::path::Path| std::os::unix::fs::symlink("b", d.join("a/link")).unwrap(),
    ] {
        make(dir.path());
        assert!(stray_under(dir.path(), &files).is_some());
        // Reset to clean for the next case.
        std::fs::remove_dir_all(dir.path().join("a")).ok();
        std::fs::remove_file(dir.path().join("loose")).ok();
        std::fs::create_dir_all(bound.parent().unwrap()).unwrap();
        std::fs::write(&bound, b"x").unwrap();
    }

    // A directory descriptor onto /proc is exactly what the walk looks for:
    // it lists this process's PID. Held, it is found; never listed, not.
    let held = std::fs::File::open("/proc").unwrap();
    let me = std::collections::HashSet::from([std::process::id().to_string()]);
    assert!(descriptor_reaches_host_proc(&me));
    drop(held);
    let nobody = std::collections::HashSet::from(["no-such-entry".to_string()]);
    assert!(!descriptor_reaches_host_proc(&nobody));
}

#[test]
fn listening_ports_are_read_from_the_proc_net_tables() {
    let tcp = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 11111 1 0 100 0 0 10 0\n\
   1: 0100007F:0050 0100007F:C350 01 00000000:00000000 00:00000000 00000000  1000        0 22222 1 0 100 0 0 10 0\n\
   2: 00000000:01BB 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 33333 1 0 100 0 0 10 0\n";
    assert_eq!(parse_listen_ports(tcp), [0x1F90, 0x01BB]);
    let tcp6 = "  sl  local_address                         remote_address                        st tx_queue\n\
   0: 00000000000000000000000001000000:2328 00000000000000000000000000000000:0000 0A 00000000:00000000\n";
    assert_eq!(parse_listen_ports(tcp6), [0x2328]);
    assert_eq!(parse_listen_ports(""), Vec::<u16>::new());
    assert_eq!(parse_listen_ports("header only\n"), Vec::<u16>::new());
    assert_eq!(parse_listen_ports("h\n garbage line\n"), Vec::<u16>::new());
    // On Linux the real tables see a listener this very test opens.
    #[cfg(target_os = "linux")]
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(listening_ports().contains(&port));
        drop(listener);
    }
}

// ---- live proofs: need a bwrap that can create the namespaces -------------

#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use swarmail_evals::isolation;

/// The runner executable as a controller reports it (`current_exe`).
#[cfg(target_os = "linux")]
fn runner_exe() -> PathBuf {
    std::fs::canonicalize(exe()).unwrap()
}

/// A trial whose agent is the helper in `mode`, run inside the boundary.
#[cfg(target_os = "linux")]
fn isolated_helper(mode: &str) -> (std::process::Command, tempfile::TempDir) {
    isolated_helper_with(mode, &[])
}

/// The same, with `agent_args` appended to the wrapper's argv. The wrapper
/// forwards them to the helper (libtest treats them as inert name filters).
#[cfg(target_os = "linux")]
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
        .arg("--isolated-expose")
        .arg(&wrapper)
        .args(["--deadline-ms", "20000"])
        .args(["--task", "signup", "--agent", "probe", "--"])
        .arg(&wrapper)
        .args(agent_args);
    (command, dir)
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn a_fresh_home_and_the_trials_own_services_work_through_the_boundary() {
    require_boundary();
    let _scope = trial_scope();
    // Two isolated trials in a row: HOME is fresh and writable each time,
    // so the first one's sentinel is absent from the second.
    for _ in 0..2 {
        let (command, _owner) = isolated_helper("home_probe");
        let out = isolated_output(command);
        let row = one_row(&out);
        assert_eq!(agent_codes(&row), [Some(0)], "{}", stderr(&out));
    }
    // The oracle completes signup inside the boundary through its own
    // Swarmail MCP and its own fixture.
    let (row, out) = trial(&[
        "--isolated",
        "--deadline-ms",
        "30000",
        "--task",
        "signup",
        "--agent",
        "oracle",
    ]);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.concurrency, 1);
}

#[cfg(target_os = "linux")]
fn observer_check(code: Option<i32>) -> &'static str {
    match code {
        Some(0) => {
            "every check held; a nested user namespace was refused; /proc/1/mem is read-only"
        }
        Some(100) => "every check held; a nested user namespace was granted (observed)",
        Some(101) => "every check held; /proc/1/mem opens read-write (observed, within-trial)",
        Some(102) => {
            "every check held; a nested user namespace was granted and /proc/1/mem opens \
             read-write (observed)"
        }
        Some(40) => "the observer was not given its arguments",
        Some(41) => "the uid is not the caller's",
        Some(42) => {
            "/proc shows the host (a host PID), or PID 1 is not the runner (a reaper stands inside)"
        }
        Some(43) => "an interface other than lo exists",
        Some(44) => "lo is unusable",
        Some(45) => "a listed host path is present, or a stray entry is under /home",
        Some(46) => "/tmp is not the bounded tmpfs",
        Some(47) => "a foreign environment key reached the agent",
        Some(48) => "a stray descriptor reached the agent",
        Some(49) => "capabilities are not empty, or NoNewPrivs is not 1",
        Some(50) => "a descriptor of some process inside reaches the host /proc",
        _ => "the observer did not report",
    }
}

#[cfg(target_os = "linux")]
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
        std::fs::canonicalize(swarmail_bin()).unwrap(),
        std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap(),
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
        format!("runner={}", runner_exe().display()),
        format!(
            "bound={}",
            bound
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
    ];
    // The host's Yama setting is a recorded observation, not a prerequisite
    // (0115, After the first review).
    let ptrace_scope = std::fs::read_to_string("/proc/sys/kernel/yama/ptrace_scope")
        .map_or_else(|_| "unavailable".to_string(), |v| v.trim().to_string());
    eprintln!("observation: host kernel.yama.ptrace_scope = {ptrace_scope}");
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
    let out = isolated_output(command);
    let row = one_row(&out);
    let code = agent_codes(&row).first().copied().flatten();
    eprintln!("observer: {} (exit {code:?})", observer_check(code));
    eprintln!(
        "observation: nested user namespace granted inside: {}; /proc/1/mem opens read-write: {}",
        matches!(code, Some(100) | Some(102)),
        matches!(code, Some(101) | Some(102)),
    );
    assert!(
        matches!(code, Some(0) | Some(100) | Some(101) | Some(102)),
        "the boundary is not as designed: {} (exit {code:?})\n{}",
        observer_check(code),
        stderr(&out)
    );
    drop(held);
}

/// Spawn an isolated trial through the real CLI whose agent sleeps, and wait
/// until it runs. The guard is the controller.
#[cfg(target_os = "linux")]
fn idle_trial() -> (ChildGuard, ProcessIdentity, tempfile::TempDir) {
    use std::os::unix::fs::PermissionsExt;
    // An external agent runs isolated only if it is exposed (0115): a
    // generated script that becomes `/usr/bin/sleep` by `exec`.
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("idle.sh");
    std::fs::write(&script, "#!/bin/sh\nexec /usr/bin/sleep 1000\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = cli();
    command
        .args(["trial", "--isolated", "--isolated-expose"])
        .arg(&script)
        .args([
            "--task",
            "signup",
            "--agent",
            "idle",
            "--deadline-ms",
            "30000",
            "--",
        ])
        .arg(&script);
    let trial = bounded(command, ISOLATED_CLI_BOUND);
    let agent = wait_for_sleeping_agent(trial.pid());
    (trial, agent, dir)
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn killing_the_controller_leaves_no_trial_process_alive() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, _agent, _owner) = idle_trial();
    let family = process_descendants(trial.pid());
    assert!(
        family.len() >= 3,
        "bwrap, the runner (PID 1), the agent: {family:?}"
    );
    send_signal(trial.pid(), libc::SIGKILL);
    let _ = trial.wait_with_output();
    eventually_else(
        "every trial process to die",
        || family.iter().all(|p| !identity_alive(p)),
        || survivors(&family),
    );
    // No orphan survives to watch the next trial.
    let (next, _, _next_owner) = idle_trial();
    assert!(family.iter().all(|p| !identity_alive(p)));
    send_signal(next.pid(), libc::SIGKILL);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn killing_the_runner_leaves_no_agent_alive_and_the_controller_names_the_missing_row() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent, _owner) = idle_trial();
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

#[cfg(target_os = "linux")]
#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn an_interrupt_emits_the_runners_interrupted_row_and_leaves_nothing_alive() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent, _owner) = idle_trial();
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

#[cfg(target_os = "linux")]
#[test]
#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]
fn an_agent_killed_from_outside_leaves_the_idle_class_not_a_pass() {
    require_boundary();
    let _scope = trial_scope();
    let (trial, agent, _owner) = idle_trial();
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
#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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
        let (command, _owner) = isolated_helper_with("cross_trial_probe", &victim_args);
        let out = isolated_output(command);
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
