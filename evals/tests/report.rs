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
    let _trial_scope = trial_scope();
    let dir = tempdir();
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

// Linux only: the worker wrapper captures the worker's environment from
// /proc/$PPID/environ.
#[cfg(target_os = "linux")]
#[test]
fn matrix_workers_are_closed_and_child_output_is_not_published() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let marker = dir.path().join("worker-env");
    let wrapper = dir.path().join("observe-worker.sh");
    let script = "#!/bin/sh\nprintf '%s' 'synthetic-matrix-output-canary-196'\nprintf '%s' 'synthetic-matrix-output-canary-196' >&2\n/usr/bin/cat /proc/$PPID/environ > \"$1\"\nshift\nexec \"$@\" agent oracle\n".to_string();
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let agents = agents_file(
        dir.path(),
        json!([{
            "label": "matrix-observer",
            "argv": [wrapper, marker.to_string_lossy(), exe()]
        }]),
    );
    let canary = "synthetic-matrix-parent-key-196";
    let out = cli()
        .env("OPENAI_API_KEY", canary)
        .env(
            "UNRELATED_CALLER_SETTING",
            "synthetic-matrix-parent-value-196",
        )
        .args([
            "matrix", "--tasks", "signup", "--agents", &agents, "--trials", "1",
        ])
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, Outcome::Pass, "{}", stderr(&out));
    assert!(
        !out.stdout
            .windows(canary.len())
            .any(|w| w == canary.as_bytes())
    );
    assert!(
        !out.stderr
            .windows(canary.len())
            .any(|w| w == canary.as_bytes())
    );
    assert!(!stderr(&out).contains("synthetic-matrix-output-canary-196"));

    let bytes = std::fs::read(marker).unwrap();
    assert!(
        !bytes
            .windows(b"NONUTF8_PARENT_\xff".len())
            .any(|w| w == b"NONUTF8_PARENT_\xff"),
        "caller native environment canary crossed the closed command boundary"
    );
    let pairs: Vec<(&str, &str)> = bytes
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let index = part.iter().position(|b| *b == b'=').unwrap();
            (
                std::str::from_utf8(&part[..index]).unwrap(),
                std::str::from_utf8(&part[index + 1..]).unwrap(),
            )
        })
        .collect();
    let names: std::collections::HashSet<_> = pairs.iter().map(|(name, _)| *name).collect();
    let expected: std::collections::HashSet<_> = [
        "PATH",
        "LANG",
        "LC_ALL",
        "HOME",
        "TMPDIR",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "SWARMAIL_BIN",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected, "matrix worker map");
    assert_eq!(
        pairs
            .iter()
            .find(|(name, _)| *name == "SWARMAIL_BIN")
            .unwrap()
            .1,
        swarmail_bin().to_str().unwrap()
    );
    assert!(!String::from_utf8_lossy(&bytes).contains(canary));
}

// Linux only: macOS file systems (APFS, HFS+) refuse a non-UTF-8 directory
// name with EILSEQ, so such a PATH entry cannot exist there.
#[cfg(target_os = "linux")]
#[test]
fn matrix_preserves_non_utf8_path_discovery_through_the_worker_cli() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::PermissionsExt;
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let path_dir = dir.path().join(OsString::from_vec(b"agents-\xff".to_vec()));
    std::fs::create_dir(&path_dir).unwrap();
    let marker = dir.path().join("agent-ran");
    let program = path_dir.join("native-agent");
    let literal_argument = "synthetic-matrix-literal-196";
    std::fs::write(
        &program,
        "#!/bin/sh\nprintf '%s\\0' \"$2\" > \"$1\"\nexec \"$3\" agent oracle\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let agents = agents_file(
        dir.path(),
        json!([{
            "label": "native-path-agent",
            "argv": ["native-agent", marker.to_str().unwrap(), literal_argument, exe()]
        }]),
    );

    let out = cli()
        .env("PATH", &path_dir)
        .args([
            "matrix", "--tasks", "signup", "--agents", &agents, "--trials", "1",
        ])
        .output()
        .unwrap();
    let result_rows = rows(&out.stdout);
    assert_eq!(result_rows.len(), 1, "{}", stderr(&out));
    assert_eq!(result_rows[0].agent.as_deref(), Some("native-path-agent"));
    assert_eq!(result_rows[0].outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(result_rows[0].agent_exit_status, Some(0));
    assert!(out.status.success(), "{}", stderr(&out));
    let mut expected = literal_argument.as_bytes().to_vec();
    expected.push(0);
    assert_eq!(std::fs::read(marker).unwrap(), expected);
}

#[test]
fn the_matrix_keeps_counting_after_a_harness_error_and_exits_two() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
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
    let _trial_scope = trial_scope();
    let dir = tempdir();
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
    assert!(
        !stderr(&out).contains("no-such-swarmail"),
        "lookup paths are suppressed: {}",
        stderr(&out)
    );
    assert!(!marker.exists(), "no trial child ran");
}

#[test]
fn the_matrix_refuses_bad_inputs_before_running_anything() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
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
    let task = if task == "t" { "signup" } else { task };
    let req = swarmail_evals::runner::TrialRequest {
        task: Some(task.into()),
        agent: Some("a".into()),
        argv: vec!["/bin/true".into()],
        ..Default::default()
    };
    json!({
        "schema_version": 2, "concurrency": 1, "trial_id": format!("{ms:032x}"), "task": task, "agent": "a",
        "outcome": outcome, "failure_class": class, "wall_time_ms": ms,
        "agent_exit_status": 0, "agent_exits": [], "confinement": "unconfined",
        "provenance": swarmail_evals::runner::make_provenance(&req, None, None)
    })
    .to_string()
}

fn report_of(input: &str) -> (Option<i32>, Value) {
    let dir = tempdir();
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
    let _trial_scope = trial_scope();
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
    let _trial_scope = trial_scope();
    let (code, v) = report_of("");
    assert_eq!(code, Some(0));
    assert_eq!(v["groups"], json!([]));
    assert!(
        v["tool_surface_note"]
            .as_str()
            .unwrap()
            .contains("asymmetric")
    );
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

// The matrix guard and the tests built on it prove cleanup by PID/start-time
// identities read from /proc (the matrix's process table and each owned
// process's group, session and state); macOS has no /proc, so they are Linux
// only rather than passing vacuously there.
#[cfg(target_os = "linux")]
struct MatrixGuard {
    child: Option<std::process::Child>,
    owner: ProcessIdentity,
    release: std::path::PathBuf,
    owned: Vec<ProcessIdentity>,
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn matrix_signal_case(signal: i32, noisy: bool) {
    let dir = tempdir();
    let marker = dir.path().join("active.json");
    let launches = dir.path().join("launches.txt");
    let rows_path = dir.path().join("matrix.jsonl");
    let (helper, _helper_owner) = helper_command(
        if noisy { "noisy_stall" } else { "stall" },
        &[],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            (LAUNCH_LOG_ENV, launches.to_str().unwrap()),
        ],
    );
    let wrapper = helper.get_args().last().unwrap().to_os_string();
    let agents = agents_file(
        dir.path(),
        json!([
            {"label": "oracle", "argv": []},
            {"label": "stall", "argv": [wrapper.to_str().unwrap()]}
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
    let release = if noisy {
        marker.with_extension("release")
    } else {
        dir.path().join("unused-release")
    };
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
    if noisy {
        eventually("noisy descendant acknowledged actual TERM cleanup", || {
            marker.with_extension("cleanup-entered").exists()
        });
        assert!(identity_alive(&descendant));
        assert!(
            matrix.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "matrix waits for live owned cleanup"
        );
        let paths: Vec<String> = serde_json::from_value(ready["runtime_paths"].clone()).unwrap();
        assert_eq!(paths.len(), 6);
        assert!(paths.iter().all(|p| std::path::Path::new(p).is_dir()));
        assert!(ready["stdout_bytes"].as_u64().unwrap() > 64 * 1024);
        assert!(ready["stderr_bytes"].as_u64().unwrap() > 64 * 1024);
        std::fs::write(marker.with_extension("release"), b"release").unwrap();
    }
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
    if noisy {
        let canary = b"synthetic-cancellation-output-canary";
        assert!(!output.windows(canary.len()).any(|w| w == canary));
        let paths: Vec<String> = serde_json::from_value(ready["runtime_paths"].clone()).unwrap();
        assert!(paths.iter().all(|p| !std::path::Path::new(p).exists()));
    }
    assert!(
        output.starts_with(&prefix_before),
        "the exact committed JSONL byte prefix is immutable"
    );
    let parsed = rows(&output);
    assert_eq!(parsed.len(), 4, "full schema-v2 product accounting");
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

// Linux only: proves owned cleanup through MatrixGuard's /proc identities.
#[cfg(target_os = "linux")]
#[test]
fn matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup() {
    let _trial_scope = trial_scope();
    matrix_signal_case(libc::SIGTERM, false);
    matrix_signal_case(libc::SIGINT, false);
}

// Linux only: proves owned cleanup through MatrixGuard's /proc identities.
#[cfg(target_os = "linux")]
#[test]
fn privacy_cancellation_preserves_owned_runtime_and_suffix_accounting() {
    let _trial_scope = trial_scope();
    matrix_signal_case(libc::SIGTERM, true);
    matrix_signal_case(libc::SIGINT, true);
}

// Linux only: its owner guard and its no-survivor proof are MatrixGuard's
// /proc identities.
#[cfg(target_os = "linux")]
#[test]
fn a_broken_matrix_stdout_stops_after_the_owned_trial_finishes() {
    use std::io::Read;
    let _trial_scope = trial_scope();
    let dir = tempdir();
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
    let _trial_scope = trial_scope();
    let dir = tempdir();
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

// Linux only: the reaped fallback cargo group is proved by /proc identities.
#[cfg(target_os = "linux")]
#[test]
fn matrix_signal_cancels_and_reaps_the_fallback_cargo_group() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
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

#[test]
fn v2_report_refuses_v1_and_mixed_rows_before_aggregation() {
    let v2 = row("t", "pass", None, 1);
    let mut old: Value = serde_json::from_str(&v2).unwrap();
    old["schema_version"] = json!(1);
    for input in [
        old.to_string(),
        format!("{v2}\n{old}"),
        format!("{old}\n{v2}"),
    ] {
        assert!(
            swarmail_evals::report::parse_rows(&input)
                .unwrap_err()
                .contains("schema_version")
        );
        let dir = tempdir();
        let path = dir.path().join("rows.jsonl");
        std::fs::write(&path, input).unwrap();
        let out = cli()
            .args(["report", "--input"])
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty(), "no partial aggregation");
        assert!(stderr(&out).contains("unsupported result schema version; expected 2"));
    }
}

#[test]
fn report_refuses_pre_provenance_v2_rows_without_partial_aggregation() {
    let current = row("signup", "pass", None, 1);
    let mut old: Value = serde_json::from_str(&current).unwrap();
    old.as_object_mut().unwrap().remove("provenance");
    for input in [
        old.to_string(),
        format!("{current}\n{old}"),
        format!("{old}\n{current}"),
    ] {
        assert!(swarmail_evals::report::parse_rows(&input).is_err());
        let (code, output) = report_of(&input);
        assert_eq!(code, Some(2));
        assert_eq!(output, Value::Null);
    }
}

#[test]
fn sequential_rows_reject_missing_or_invalid_concurrency() {
    let _trial_scope = trial_scope();
    let valid: Value = serde_json::from_str(&row("signup", "pass", None, 1)).unwrap();
    let mut invalid = Vec::new();
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("concurrency");
    invalid.push(missing);
    for value in [
        json!(0),
        json!(2),
        json!(-1),
        json!(1.0),
        json!("1"),
        Value::Null,
    ] {
        let mut bad = valid.clone();
        bad["concurrency"] = value;
        invalid.push(bad);
    }
    let mut old = valid.clone();
    old["schema_version"] = json!(1);
    invalid.push(old);
    for bad in invalid {
        let text = bad.to_string();
        assert!(swarmail_evals::report::parse_rows(&text).is_err(), "{bad}");
        let (status, output) = report_of(&text);
        assert_eq!(status, Some(2), "{bad}");
        assert_eq!(output, Value::Null);
    }
}

#[test]
fn sequential_matrix_refuses_overlap_and_unsupported_concurrency() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    for concurrency in ["0", "2"] {
        let out = cli()
            .args([
                "matrix",
                "--tasks",
                "signup",
                "--agents",
                "/nonexistent/agents",
                "--trials",
                "1",
                "--concurrency",
                concurrency,
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(stderr(&out).contains("concurrency_unsupported"));
    }
    let ready = dir.path().join("matrix-ready");
    let (command, _wrapper) =
        helper_command("held_idle", &[], &[(MARKER_ENV, ready.to_str().unwrap())]);
    let wrapper = command.get_args().last().unwrap().to_str().unwrap();
    let agents = agents_file(dir.path(), json!([{"label":"held-idle","argv":[wrapper]}]));
    let mut command = cli();
    command
        .args([
            "matrix",
            "--tasks",
            "signup",
            "--agents",
            &agents,
            "--trials",
            "1",
            "--concurrency",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let matrix = ChildGuard::new(command.spawn().unwrap());
    eventually("matrix agent readiness", || ready.exists());
    let (row, out) = trial(&["--task", "signup", "--agent", "oracle"]);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(stderr(&out).contains("concurrent_trials_refused"));
    let launched = dir.path().join("contender-launch");
    let agents = agents_file(
        dir.path(),
        json!([{"label":"inert","argv":["/usr/bin/touch", launched]}]),
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
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("concurrent_trials_refused"));
    let refused = rows(&out.stdout);
    assert_eq!(refused.len(), 4);
    assert!(
        refused
            .iter()
            .all(|row| row.outcome == Outcome::HarnessError
                && row.concurrency == 1
                && row.wall_time_ms == 0
                && row.agent_exits.is_empty())
    );
    assert!(!launched.exists());
    std::fs::write(ready.with_extension("release"), b"release").unwrap();
    let out = matrix.wait_with_output();
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert_eq!(rows(&out.stdout)[0].outcome, Outcome::Fail);
}

fn comparison_row(
    target: &str,
    task: &str,
    attempt: u32,
    outcome: swarmail_evals::tasks::Outcome,
) -> swarmail_evals::identity::ComparisonRow {
    use swarmail_evals::identity::*;
    ComparisonRow {
        schema_version: COMPARISON_SCHEMA,
        kind: "comparison".into(),
        protocol_sha256: "a".repeat(64),
        target: target.into(),
        executable_sha256: "b".repeat(64),
        settings: ComparisonSettings {
            connection: if target == "swarmail-mcp" {
                "native-mcp"
            } else {
                "stdio-adapter"
            }
            .into(),
            tools_sha256: "c".repeat(64),
        },
        coordinate: ComparisonCoordinate {
            task: task.into(),
            attempt,
        },
        outcome,
        hash: String::new(),
    }
    .seal()
}
#[test]
fn comparison_citations_state_protocol_coordinates_and_the_whole_denominator() {
    use swarmail_evals::{report, tasks::Outcome::*};
    let rows = [
        comparison_row("mailpit-mcp", "signup", 2, Fail),
        comparison_row("mailpit-mcp", "signup", 1, Pass),
        comparison_row("mailpit-mcp", "signup", 3, HarnessError),
        comparison_row("mailpit-mcp", "signup", 4, Pass),
        comparison_row("swarmail-mcp", "signup", 1, Pass),
    ];
    let cited = report::cite_comparison(&rows).unwrap();
    assert_eq!(cited["protocol_sha256"], "a".repeat(64));
    let cells = cited["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 2);
    // Counted by hand: 2 passes of 4 attempts, the harness error kept in the denominator.
    assert_eq!(cells[0]["target"], "mailpit-mcp");
    assert_eq!(cells[0]["denominator"], 4);
    assert_eq!(cells[0]["passes"], 2);
    assert_eq!(cells[0]["fails"], 1);
    assert_eq!(cells[0]["harness_errors"], 1);
    assert_eq!(cells[0]["pass_rate"], 0.5);
    assert_eq!(cells[0]["attempts"], serde_json::json!([1, 2, 3, 4]));
    assert_eq!(cells[1]["denominator"], 1);
    assert_eq!(cells[1]["pass_rate"], 1.0);
    // Refusals: empty, duplicate coordinate, mixed protocol.
    assert!(report::cite_comparison(&[]).is_err());
    let dup = [rows[0].clone(), rows[0].clone()];
    assert!(report::cite_comparison(&dup).unwrap_err().contains("twice"));
    let mut other = rows[4].clone();
    other.protocol_sha256 = "d".repeat(64);
    let mixed = [rows[0].clone(), other.seal()];
    assert!(
        report::cite_comparison(&mixed)
            .unwrap_err()
            .contains("different protocols")
    );
}
#[test]
fn the_dossier_refuses_without_baseline_finals_and_costs_in_that_order() {
    use swarmail_evals::{report, report::DossierInputs, tasks::Outcome::Pass};
    let rows = [comparison_row("mailpit-mcp", "signup", 1, Pass)];
    let digest = || Some("e".repeat(64));
    let refused = |i: DossierInputs| report::dossier(&rows, &i).unwrap_err();
    assert!(refused(DossierInputs::default()).contains("baseline"));
    assert!(
        refused(DossierInputs {
            baseline_sha256: digest(),
            ..Default::default()
        })
        .contains("final-contracts")
    );
    assert!(
        refused(DossierInputs {
            baseline_sha256: digest(),
            final_contracts_sha256: digest(),
            cost_sha256: Some("not-a-digest".into()),
        })
        .contains("cost-basis")
    );
    let all = DossierInputs {
        baseline_sha256: digest(),
        final_contracts_sha256: digest(),
        cost_sha256: digest(),
    };
    let out = report::dossier(&rows, &all).unwrap();
    assert_eq!(out["cells"][0]["denominator"], 1);
    assert_eq!(out["inputs"]["cost_sha256"], "e".repeat(64));
    assert!(report::dossier(&[], &all).is_err());
}

// ---- `matrix --brokkr`: the eval recipe path (decision 0116) -------------
//
// The engine is not present in delivery. `tests/data/brokkr-standin.sh` plays
// `brokkr run --recipe eval`: it runs the recipe's real trial.sh and score.sh
// in policy order. Everything the matrix does with a slot is real.

fn brokkr_agents(dir: &std::path::Path) -> String {
    agents_file(
        dir,
        json!([{"label": "oracle", "argv": []}, {"label": "null", "argv": []}]),
    )
}

/// The single matrix directory under an artifacts root.
fn only_matrix_dir(root: &std::path::Path) -> std::path::PathBuf {
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "one matrix under {root:?}");
    dirs.pop().unwrap()
}

#[test]
fn brokkr_mode_runs_the_product_through_the_recipe_scripts() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let agents = brokkr_agents(dir.path());
    let realm = brokkr_realm(dir.path());
    let root = realm.join("artifacts");
    let log = dir.path().join("brokkr.log");
    let out = brokkr_matrix("ok", &realm)
        .env("EVAL_TEST_BROKKR_LOG", &log)
        .args([
            "--tasks",
            "signup,otp",
            "--agents",
            &agents,
            "--trials",
            "2",
        ])
        .args(["--artifacts-dir"])
        .arg(&root)
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    assert_eq!(
        rows.len(),
        8,
        "2 tasks × 2 agents × 2 trials\n{}",
        stderr(&out)
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "fails present, no harness error\n{}",
        stderr(&out)
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
    assert_eq!(
        rows.iter().filter(|r| r.outcome == Outcome::Pass).count(),
        4,
        "four passes"
    );
    // Every row on the recipe path declares the trial unconfined (AC10).
    assert!(rows.iter().all(|r| r.confinement == "unconfined"));
    // Two sequential oracle trials of one task both pass, with distinct seeds.
    let signups: Vec<_> = rows
        .iter()
        .filter(|r| r.task.as_deref() == Some("signup") && r.agent.as_deref() == Some("oracle"))
        .collect();
    assert_eq!(signups.len(), 2);
    assert!(signups.iter().all(|r| r.outcome == Outcome::Pass));
    assert_ne!(
        signups[0].provenance.fixture.seed, signups[1].provenance.fixture.seed,
        "distinct seeds"
    );
    // One worker per slot, each naming a request inside the matrix's slots.
    let log_text = std::fs::read_to_string(&log).unwrap();
    assert_eq!(requests(&log_text), 8);
    assert_eq!(
        log_text
            .lines()
            .filter(|l| *l == "score=pass" || *l == "score=fail")
            .count(),
        8,
        "the score seat ruled pass or fail on every trial"
    );
    let matrix_dir = only_matrix_dir(&root);
    for row in &rows {
        let slot = matrix_dir.join("slots").join(&row.trial_id);
        assert!(log_text.contains(slot.join("request.json").to_str().unwrap()));
        assert!(slot.join("row.json").is_file());
        assert!(matrix_dir.join("trials").join(&row.trial_id).is_dir());
    }
    // The retained matrix replays offline exactly like a direct one.
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&matrix_dir)
        .env("SWARMAIL_BIN", "/absent-during-offline-replay")
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(1), "{}", stderr(&replay));
    assert_eq!(String::from_utf8_lossy(&replay.stdout).lines().count(), 8);
}

fn defect_case(mode: &str, expected_stderr: &str) {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let agents = agents_file(dir.path(), json!([{"label": "oracle", "argv": []}]));
    let log = dir.path().join("brokkr.log");
    let realm = brokkr_realm(dir.path());
    let out = brokkr_matrix(mode, &realm)
        .env("EVAL_TEST_BROKKR_LOG", &log)
        .args(["--tasks", "signup", "--agents", &agents, "--trials", "3"])
        .arg("--artifacts-dir")
        .arg(realm.join("artifacts"))
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    assert_eq!(rows.len(), 3, "{mode}: one row per slot\n{}", stderr(&out));
    assert_eq!(out.status.code(), Some(2), "{mode}");
    assert!(
        stderr(&out).contains(expected_stderr),
        "{mode}: wanted {expected_stderr:?} in\n{}",
        stderr(&out)
    );
    for row in &rows {
        assert_eq!(row.outcome, Outcome::HarnessError, "{mode}: {row:?}");
        assert_eq!(
            row.failure_class,
            Some(swarmail_evals::state::FailureClass::HarnessError),
            "{mode}: a worker defect is never an agent failure"
        );
    }
    assert!(
        rows[1..].iter().all(|r| r.wall_time_ms == 0),
        "{mode}: the unstarted suffix is zero-time"
    );
    assert_eq!(
        requests(&std::fs::read_to_string(&log).unwrap()),
        1,
        "{mode}: admission stopped after the defective slot"
    );
}

/// The number of seat-pair invocations a stand-in log records (one request
/// line each); its `score=` lines are the score seat's result words.
fn requests(log_text: &str) -> usize {
    log_text
        .lines()
        .filter(|l| l.starts_with("swarmail-eval-request: "))
        .count()
}

/// A legitimate row the agreement table commits: the stand-in rewrites the
/// slot the way the runner's precedence legs leave it, and every planned row
/// is produced with admission continuing. Returns the rows, the exit code and
/// the score seat's result words.
fn agreeing_case(mode: &str) -> (Vec<TrialResult>, Option<i32>, String, Vec<String>) {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let agents = agents_file(dir.path(), json!([{"label": "oracle", "argv": []}]));
    let log = dir.path().join("brokkr.log");
    let realm = brokkr_realm(dir.path());
    let out = brokkr_matrix(mode, &realm)
        .env("EVAL_TEST_BROKKR_LOG", &log)
        .args(["--tasks", "signup", "--agents", &agents, "--trials", "3"])
        .arg("--artifacts-dir")
        .arg(realm.join("artifacts"))
        .output()
        .unwrap();
    let rows = rows(&out.stdout);
    let log_text = std::fs::read_to_string(&log).unwrap();
    assert_eq!(rows.len(), 3, "{mode}: one row per slot\n{}", stderr(&out));
    assert_eq!(
        requests(&log_text),
        3,
        "{mode}: admission continued through every planned slot\n{}",
        stderr(&out)
    );
    let words = log_text
        .lines()
        .filter_map(|l| l.strip_prefix("score="))
        .map(str::to_owned)
        .collect();
    (rows, out.status.code(), stderr(&out), words)
}

/// AC2, AC18: a `fail`/`budget` row over a state that still replays `pass` is
/// the deadline leg of accepted 0055 section 2, not a disagreement. It is
/// committed as the row says, the score seat reports `fail`, and admission goes on.
#[test]
fn brokkr_mode_commits_a_budget_row_over_a_passing_state() {
    let (rows, code, err, words) = agreeing_case("budget-over-pass");
    assert_eq!(code, Some(1), "{err}");
    assert!(!err.contains("score_mismatch"), "{err}");
    for row in &rows {
        assert_eq!(row.outcome, Outcome::Fail, "{row:?}");
        assert_eq!(
            row.failure_class,
            Some(swarmail_evals::state::FailureClass::Budget)
        );
        assert!(row.wall_time_ms > 0, "a real row, not a refusal suffix");
    }
    assert_eq!(words, ["fail", "fail", "fail"]);
}

/// AC2, AC18: an infrastructure `harness_error` row over a state that still
/// replays `pass` is committed as written, never rewritten to `score_mismatch`,
/// and the score seat reports `unscorable` so the journal and the row agree.
#[test]
fn brokkr_mode_commits_a_harness_error_row_over_a_passing_state() {
    let (rows, code, err, words) = agreeing_case("harness-error-over-pass");
    assert_eq!(code, Some(2), "{err}");
    assert!(!err.contains("score_mismatch"), "{err}");
    for row in &rows {
        assert_eq!(row.outcome, Outcome::HarnessError, "{row:?}");
        assert!(row.wall_time_ms > 0, "committed as written");
    }
    assert_eq!(words, ["unscorable", "unscorable", "unscorable"]);
}

#[test]
fn brokkr_mode_turns_every_worker_defect_into_a_row() {
    defect_case("no-row", "trial_row_missing");
    defect_case("foreign-row", "trial_context_mismatch");
    defect_case("bad-exit", "does not match result outcome");
    defect_case("bad-score", "score_mismatch");
    // A budget row whose retained state no longer validates is a leaf that
    // changed after it froze: the one budget contradiction (the agreement table).
    defect_case("budget-over-unscorable", "score_mismatch");
    // A gate that never ruled is not a gate that disagrees.
    defect_case("no-score", "score_missing");
    // A fixed 0076 category the trial left in the slot's diagnostics surfaces
    // through the CLI as the worker's category, as the direct worker's does.
    defect_case("category-stderr", "concurrent_trials_refused");
}

/// A via-brokkr agent under the recipe path fails closed at the trial: the
/// slot is a harness error (never the agent's `gave_up`), admission stops and
/// no agent was ever launched.
#[test]
fn brokkr_mode_refuses_a_via_brokkr_agent_as_a_harness_error() {
    let _trial_scope = trial_scope();
    let exe = exe();
    let via_flag = json!([{
        "label": "claude-via",
        "argv": [exe, "agent-claude", "--model", "claude-sonnet-5-5", "--config",
                 "/nonexistent/selection.json", "--via-brokkr", "/nonexistent/brokkr"],
        "model": "claude-sonnet-5-5",
        "config_id": "cfg",
        "wrapper_profile": "claude_2_1_310_via_brokkr",
    }]);
    let profile_only = json!([{
        "label": "claude-profile",
        "argv": [exe, "agent-claude", "--model", "claude-sonnet-5-5", "--config",
                 "/nonexistent/selection.json"],
        "model": "claude-sonnet-5-5",
        "config_id": "cfg",
        "wrapper_profile": "claude_2_1_310_via_brokkr",
    }]);
    for (label, agents) in [("flag", via_flag), ("profile only", profile_only)] {
        let dir = tempdir();
        let agents = agents_file(dir.path(), agents);
        let realm = brokkr_realm(dir.path());
        let log = dir.path().join("brokkr.log");
        let out = brokkr_matrix("ok", &realm)
            .env("EVAL_TEST_BROKKR_LOG", &log)
            .args(["--tasks", "signup", "--agents", &agents, "--trials", "3"])
            .arg("--artifacts-dir")
            .arg(realm.join("artifacts"))
            .output()
            .unwrap();
        let rows = rows(&out.stdout);
        assert_eq!(rows.len(), 3, "{label}: one row per slot\n{}", stderr(&out));
        assert_eq!(out.status.code(), Some(2), "{label}");
        for row in &rows {
            assert_eq!(row.outcome, Outcome::HarnessError, "{label}: {row:?}");
            assert_eq!(
                row.failure_class,
                Some(swarmail_evals::state::FailureClass::HarnessError),
                "{label}: a refused driver is never a model failure"
            );
        }
        assert!(
            rows[1..].iter().all(|r| r.wall_time_ms == 0),
            "{label}: admission stopped; the suffix is zero-time"
        );
        assert_eq!(
            requests(&std::fs::read_to_string(&log).unwrap()),
            1,
            "{label}: one worker only"
        );
        // The trial said why, in the slot's diagnostics, before any agent ran.
        let matrix_dir = only_matrix_dir(&realm.join("artifacts"));
        let slot = matrix_dir.join("slots").join(&rows[0].trial_id);
        let diagnostics = std::fs::read_to_string(slot.join("trial.stderr")).unwrap();
        let expected = if label == "flag" {
            "driver_contract_unknown"
        } else {
            "via_brokkr_profile_mismatch"
        };
        assert!(diagnostics.contains(expected), "{label}: {diagnostics}");
        assert_eq!(
            std::fs::read_to_string(slot.join("exit")).unwrap().trim(),
            "2",
            "{label}"
        );
    }
}

#[test]
fn brokkr_mode_records_a_killed_worker_as_a_harness_error() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let agents = agents_file(dir.path(), json!([{"label": "oracle", "argv": []}]));
    let ready = dir.path().join("ready");
    let realm = brokkr_realm(dir.path());
    let mut cmd = brokkr_matrix("hang-after-start", &realm);
    let child = cmd
        .env("EVAL_TEST_BROKKR_READY", &ready)
        .args(["--tasks", "signup", "--agents", &agents, "--trials", "2"])
        .arg("--artifacts-dir")
        .arg(realm.join("artifacts"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let guard = ChildGuard::new(child);
    eventually("the stand-in worker to start", || {
        std::fs::read_to_string(&ready).is_ok_and(|t| t.trim().parse::<i32>().is_ok())
    });
    let pid: i32 = std::fs::read_to_string(&ready)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    send_signal(pid, libc::SIGKILL);
    let out = guard.wait_with_output();
    let rows = rows(&out.stdout);
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter().all(|r| r.outcome == Outcome::HarnessError
            && r.failure_class == Some(swarmail_evals::state::FailureClass::HarnessError)),
        "a killed worker is a harness error, never gave_up: {rows:?}"
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("trial_row_missing"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline() {
    let dir = tempdir();
    let agents = brokkr_agents(dir.path());
    let realm = brokkr_realm(dir.path());
    let root = realm.join("artifacts");
    let base = |extra: &[&str]| {
        let mut cmd = brokkr_matrix("ok", &realm);
        cmd.args(["--tasks", "signup", "--agents", &agents, "--trials", "1"])
            .args(extra);
        cmd
    };
    let root_arg = root.to_str().unwrap();
    let refused = |out: std::process::Output, category: &str| {
        assert_eq!(out.status.code(), Some(2), "{category}");
        assert!(out.stdout.is_empty(), "{category}: no rows");
        assert!(
            stderr(&out).contains(category),
            "{category}: {}",
            stderr(&out)
        );
    };
    // No --artifacts-dir.
    refused(base(&[]).output().unwrap(), "brokkr_requires_artifacts");
    // A deadline that cannot nest inside the seat's 1800 s.
    refused(
        base(&["--artifacts-dir", root_arg, "--deadline-ms", "1200001"])
            .output()
            .unwrap(),
        "deadline_exceeds_seat",
    );
    // A working directory without the recipe.
    let mut cmd = base(&["--artifacts-dir", root_arg]);
    cmd.current_dir(dir.path());
    refused(cmd.output().unwrap(), "brokkr_realm_invalid");
    // A relative brokkr path.
    let out = cli()
        .current_dir(&realm)
        .args([
            "matrix", "--brokkr", "brokkr", "--tasks", "signup", "--agents", &agents,
        ])
        .args(["--trials", "1", "--artifacts-dir", root_arg])
        .output()
        .unwrap();
    refused(out, "brokkr_realm_invalid");
    // An artifact root outside the realm's tree: the seat's hands cannot reach
    // it, so no slot is spent on it (a sibling of the realm, then a `..` path).
    let outside = dir.path().join("artifacts-outside");
    refused(
        base(&["--artifacts-dir", outside.to_str().unwrap()])
            .output()
            .unwrap(),
        "brokkr_artifacts_unreachable",
    );
    refused(
        base(&["--artifacts-dir", "../artifacts-outside"])
            .output()
            .unwrap(),
        "brokkr_artifacts_unreachable",
    );
    assert!(!outside.exists(), "an unreachable root is never created");
    // Concurrency stays refused in this mode too (accepted 0076).
    refused(
        base(&["--artifacts-dir", root_arg, "--concurrency", "2"])
            .output()
            .unwrap(),
        "concurrency_unsupported",
    );
    assert!(!root.exists(), "a refusal writes no evidence");
}

// ---- `report --journal`: the typed join (decision 0116) -------------------

fn golden_rows() -> Vec<TrialResult> {
    // 8 rows, 3 agents: the built-in oracle (no driver, no journal), a fully
    // joined external agent, and one with a trial the journal never saw plus,
    // under another task, a trial with tokens but no tool rows.
    let spec = [
        (1, "oracle", false, Outcome::Pass),
        (2, "oracle", false, Outcome::Pass),
        (3, "claude-a", true, Outcome::Pass),
        (4, "claude-a", true, Outcome::Pass),
        (5, "claude-a", true, Outcome::Fail),
        (6, "codex-b", true, Outcome::Pass),
        (7, "codex-b", true, Outcome::Fail),
        (8, "codex-b", true, Outcome::Pass),
    ];
    spec.into_iter()
        .map(|(n, agent, external, outcome)| {
            synthetic_row(
                // Trial 8 sits in its own (task, agent) group so one group can
                // hold tokens with no tool rows and no kill fact, apart from
                // the group that holds a trial the journal never saw.
                if n == 8 { "otp" } else { "signup" },
                agent,
                external,
                outcome,
                1000 * n as u64,
                &fixed_id(n),
            )
        })
        .collect()
}

fn write_rows(dir: &std::path::Path, rows: &[TrialResult]) -> std::path::PathBuf {
    let path = dir.join("rows.jsonl");
    let text: String = rows
        .iter()
        .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
        .collect();
    std::fs::write(&path, text).unwrap();
    path
}

fn golden_facts() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/journal-facts-golden.json")
}

fn run_report(rows: &std::path::Path, extra: &[&std::ffi::OsStr]) -> std::process::Output {
    cli()
        .args(["report", "--input"])
        .arg(rows)
        .args(extra)
        .output()
        .unwrap()
}

/// The (signup, agent) group; the golden rows keep every agent's trials there
/// except trial 8, which `group_in` finds under `otp`.
fn group<'a>(report: &'a Value, agent: &str) -> &'a Value {
    group_in(report, "signup", agent)
}

fn group_in<'a>(report: &'a Value, task: &str, agent: &str) -> &'a Value {
    report["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["task"] == task && g["agent"] == agent)
        .unwrap_or_else(|| panic!("no group {task} {agent}"))
}

#[test]
fn report_joins_journal_facts_by_trial_id() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let facts = golden_facts();
    let out = run_report(&rows_path, &["--journal".as_ref(), facts.as_os_str()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();

    // The fully joined group: hand-computed from tests/data/journal-facts-golden.json.
    // total = input + output per trial = 1200, 3600, 2400; two passes.
    let claude = &group(&report, "claude-a")["journal"];
    assert_eq!(claude["joined"], 3);
    assert_eq!(claude["missing"], json!({}));
    assert_eq!(
        claude["tokens"]["input"],
        json!({"n": 3, "median": 2000, "p95": 3000, "max": 3000, "sum": 6000})
    );
    assert_eq!(
        claude["tokens"]["output"],
        json!({"n": 3, "median": 400, "p95": 600, "max": 600, "sum": 1200})
    );
    assert_eq!(
        claude["tokens"]["cached"],
        json!({"n": 3, "median": 5000, "p95": 7000, "max": 7000, "sum": 12000})
    );
    assert_eq!(
        claude["tokens"]["total"],
        json!({"n": 3, "median": 2400, "p95": 3600, "max": 3600, "sum": 7200})
    );
    assert_eq!(claude["tokens_per_success"], json!(3600.0));
    assert_eq!(claude["tokens_per_success_reason"], Value::Null);
    assert_eq!(
        claude["turns"],
        json!({"n": 3, "median": 7, "p95": 9, "max": 9})
    );
    assert_eq!(claude["seat_wall_time_ms"]["median"], 20000);
    assert_eq!(
        claude["tool_calls"],
        json!({
            "total": 6,
            "invalid_argument": 1,
            "trials": 3,
            "by_tool": {"mcp__fixture__eval_http": 3, "mcp__swarmail__wait": 3}
        })
    );
    assert_eq!(claude["tool_calls_reason"], Value::Null);
    assert_eq!(claude["deadline_killed"], 0);
    assert_eq!(claude["deadline_killed_reason"], Value::Null);
    assert_eq!(
        claude["runs"],
        json!({fixed_id(3): "run-3", fixed_id(4): "run-4", fixed_id(5): "run-5"})
    );

    // The group with a trial the journal never saw: the figure is withheld,
    // never computed over the two that were seen.
    let codex = &group(&report, "codex-b")["journal"];
    assert_eq!(codex["joined"], 1);
    assert_eq!(codex["scored"], 2);
    assert_eq!(codex["missing"], json!({"journal_missing": 1}));
    assert_eq!(codex["tokens_per_success"], Value::Null);
    assert_eq!(codex["tokens_per_success_reason"], "incomplete_join");
    assert_eq!(codex["tokens"]["total"]["sum"], 600);
    // The tool-call total and the kill count are withheld on the same rule: a
    // figure over the one trial the journal saw would silently omit the other.
    assert_eq!(codex["tool_calls"], Value::Null);
    assert_eq!(codex["tool_calls_reason"], "incomplete_join");
    assert_eq!(codex["deadline_killed"], Value::Null);
    assert_eq!(codex["deadline_killed_reason"], "incomplete_join");

    // Tokens present, tool rows and the kill fact unavailable (trial 8, under
    // its own task): the token figures are numbers, the other two are null with
    // their reason, and nothing is zero or false.
    let partial = &group_in(&report, "otp", "codex-b")["journal"];
    assert_eq!(partial["joined"], 1);
    assert_eq!(partial["missing"], json!({}));
    assert_eq!(partial["tokens"]["total"]["sum"], 1000);
    assert_eq!(partial["tokens_per_success"], json!(1000.0));
    assert_eq!(partial["tokens_per_success_reason"], Value::Null);
    assert_eq!(partial["turns"]["median"], 4);
    assert_eq!(partial["tool_calls"], Value::Null);
    assert_eq!(partial["tool_calls_reason"], "incomplete_join");
    assert_eq!(partial["deadline_killed"], Value::Null);
    assert_eq!(partial["deadline_killed_reason"], "incomplete_join");
    assert_eq!(
        partial["runs"],
        json!({fixed_id(8): "run-8"}),
        "the run is cited: tokens were accounted"
    );

    // The built-in agent launches no driver: a typed reason for every trial
    // and no token figure at all.
    let oracle = &group(&report, "oracle")["journal"];
    assert_eq!(oracle["missing"], json!({"builtin_agent": 2}));
    assert_eq!(oracle["joined"], 0);
    assert_eq!(oracle["tokens"], Value::Null);
    assert_eq!(oracle["tokens_per_success"], Value::Null);
    assert_eq!(oracle["turns"], Value::Null);
    assert_eq!(oracle["tool_calls"], Value::Null);
    assert_eq!(oracle["deadline_killed"], Value::Null);
    assert_eq!(report["journal_source"]["brokkr_version"], "0.12.0");

    // No numeric or boolean field anywhere stands in for an unavailable fact.
    for (task, agent) in [
        ("signup", "oracle"),
        ("signup", "codex-b"),
        ("otp", "codex-b"),
    ] {
        let journal = &group_in(&report, task, agent)["journal"];
        assert!(journal["tool_calls"].is_null(), "{task} {agent}");
        assert!(journal["deadline_killed"].is_null(), "{task} {agent}");
    }
    for agent in ["oracle", "codex-b"] {
        let journal = &group(&report, agent)["journal"];
        assert!(journal["tokens_per_success"].is_null(), "{agent}");
    }
}

#[test]
fn report_without_journal_is_unchanged() {
    let dir = tempdir();
    let rows = golden_rows();
    let rows_path = write_rows(dir.path(), &rows);
    let plain = run_report(&rows_path, &[]);
    assert!(plain.status.success());
    let expected = format!(
        "{}\n",
        serde_json::to_string(&swarmail_evals::report::report(&rows)).unwrap()
    );
    assert_eq!(String::from_utf8(plain.stdout.clone()).unwrap(), expected);
    // With the journal every existing field is byte-identical: the figures
    // are added beside them.
    let facts = golden_facts();
    let joined = run_report(&rows_path, &["--journal".as_ref(), facts.as_os_str()]);
    let mut with: Value = serde_json::from_slice(&joined.stdout).unwrap();
    let without: Value = serde_json::from_slice(&plain.stdout).unwrap();
    with.as_object_mut().unwrap().remove("journal_source");
    for g in with["groups"].as_array_mut().unwrap() {
        g.as_object_mut().unwrap().remove("journal");
    }
    assert_eq!(with, without);
}

#[test]
fn a_dropped_join_is_red() {
    // The same rows with the join dropped (an empty facts file) must differ
    // from the correct join in exactly the figure the join exists to give.
    let rows = golden_rows();
    let facts_text = std::fs::read(golden_facts()).unwrap();
    let facts = swarmail_evals::journal::read(&facts_text).unwrap();
    let correct = swarmail_evals::journal::join(&rows, &facts).unwrap();
    let mut empty = facts.clone();
    empty.trials.clear();
    let dropped = swarmail_evals::journal::join(&rows, &empty).unwrap();
    let right = swarmail_evals::report::report_with_journal(&rows, Some(&correct));
    let wrong = swarmail_evals::report::report_with_journal(&rows, Some(&dropped));
    assert_eq!(
        group(&right, "claude-a")["journal"]["tokens_per_success"],
        json!(3600.0)
    );
    assert_eq!(
        group(&wrong, "claude-a")["journal"]["tokens_per_success"],
        Value::Null
    );
    assert_eq!(
        group(&wrong, "claude-a")["journal"]["tokens_per_success_reason"],
        "incomplete_join"
    );
    assert_ne!(right, wrong);
}

#[test]
fn a_journal_that_does_not_match_the_rows_refuses_the_whole_report() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let unmatched = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/journal-facts-unmatched.json");
    let out = run_report(&rows_path, &["--journal".as_ref(), unmatched.as_os_str()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "no partial report");
    assert!(
        stderr(&out).contains("journal_unmatched_trial"),
        "{}",
        stderr(&out)
    );

    // Two entries for one trial id.
    let mut facts: Value = serde_json::from_slice(&std::fs::read(golden_facts()).unwrap()).unwrap();
    let first = facts["trials"][0].clone();
    facts["trials"].as_array_mut().unwrap().push(first);
    let dup = dir.path().join("dup.json");
    std::fs::write(&dup, facts.to_string()).unwrap();
    let out = run_report(&rows_path, &["--journal".as_ref(), dup.as_os_str()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        stderr(&out).contains("journal_duplicate_trial"),
        "{}",
        stderr(&out)
    );

    // An unknown field, a missing file and a wrong schema refuse as invalid.
    let mut unknown = facts.clone();
    unknown["surprise"] = json!(1);
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, unknown.to_string()).unwrap();
    for path in [bad, dir.path().join("absent.json")] {
        let out = run_report(&rows_path, &["--journal".as_ref(), path.as_os_str()]);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(stderr(&out).contains("journal_invalid"), "{}", stderr(&out));
    }
}

// ---- retained tool-call records (decision 0116; accepted 0075 §4) ---------

fn facts_value() -> Value {
    serde_json::from_slice(&std::fs::read(golden_facts()).unwrap()).unwrap()
}

fn write_facts(dir: &std::path::Path, facts: &Value) -> std::path::PathBuf {
    let path = dir.join("facts.json");
    std::fs::write(&path, facts.to_string()).unwrap();
    path
}

fn tree_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(tree_files(&path));
        } else {
            out.push(path);
        }
    }
    out
}

fn mode(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn evidence_records_carry_no_argument_or_result_bytes() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    // Tool rows whose arguments and results are built from the fixture admin
    // secret and a planted token: the facts carry only their lengths and
    // digests, and nothing downstream may reconstruct the bytes.
    let secret = format!("admin-secret-{}", fixed_id(0xfeed));
    let token = format!("sentinel-{}", fixed_id(0xbeef));
    let argument = format!(r#"{{"path":"/admin/state?secret={secret}","token":"{token}"}}"#);
    let result = format!(r#"{{"body":"{token} {secret}"}}"#);
    let mut facts = facts_value();
    facts["trials"][0]["tool_calls"]["value"] = json!([{
        "ordinal": 1,
        "tool": "mcp__fixture__eval_http",
        "outcome": "ok",
        "argument_bytes": argument.len(),
        "argument_sha256": swarmail_evals::provenance::sha256(argument.as_bytes()),
        "result_bytes": result.len(),
        "result_sha256": swarmail_evals::provenance::sha256(result.as_bytes()),
    }]);
    let facts_path = write_facts(dir.path(), &facts);
    let evidence = dir.path().join("evidence");
    let out = run_report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts_path.as_os_str(),
            "--evidence-dir".as_ref(),
            evidence.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();

    // One record per trial whose tool rows the journal gave (four of the five
    // it saw), none for the rest: trial 8 has tokens but no tool rows.
    let files = tree_files(&evidence);
    assert_eq!(files.len(), 4, "{files:?}");
    for file in &files {
        assert_eq!(mode(file), 0o600, "{file:?}");
        assert_eq!(mode(file.parent().unwrap()), 0o700, "{file:?}");
        assert_eq!(file.file_name().unwrap(), "tool-calls.json");
        let text = std::fs::read_to_string(file).unwrap();
        for needle in [&secret, &token] {
            assert!(!text.contains(needle.as_str()), "{file:?} leaks {needle}");
        }
        let record: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(record["schema_version"], 1);
        assert_eq!(record["truncated"], false);
    }
    assert_eq!(mode(&evidence), 0o700);
    let stdout = String::from_utf8_lossy(&out.stdout);
    for needle in [&secret, &token] {
        assert!(
            !stdout.contains(needle.as_str()),
            "the report leaks {needle}"
        );
        assert!(!stderr(&out).contains(needle.as_str()));
    }

    // The record is what the facts said: name, class, lengths, digests.
    let first =
        std::fs::read_to_string(evidence.join(fixed_id(3)).join("tool-calls.json")).unwrap();
    let record: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(record["trial_id"], fixed_id(3));
    assert_eq!(record["run_id"], "run-3");
    assert_eq!(record["tool_calls"][0]["tool"], "mcp__fixture__eval_http");
    assert_eq!(record["tool_calls"][0]["argument_bytes"], argument.len());
    assert_eq!(
        record["tool_calls"][0]["argument_sha256"],
        swarmail_evals::provenance::sha256(argument.as_bytes())
    );

    // The report names each file with its length and digest; the others are
    // typed absences, never silence.
    let indexed = &group(&report, "claude-a")["journal"]["evidence"][fixed_id(3)];
    assert_eq!(indexed["path"], format!("{}/tool-calls.json", fixed_id(3)));
    assert_eq!(indexed["length"], first.len());
    assert_eq!(
        indexed["sha256"],
        swarmail_evals::provenance::sha256(first.as_bytes())
    );
    assert_eq!(
        group(&report, "oracle")["journal"]["evidence"][fixed_id(1)],
        json!({"absent": "builtin_agent"})
    );
    assert_eq!(
        group(&report, "codex-b")["journal"]["evidence"][fixed_id(7)],
        json!({"absent": "journal_missing"})
    );
    // Tokens without tool rows: no file, no directory, a typed absence.
    assert_eq!(
        group_in(&report, "otp", "codex-b")["journal"]["evidence"][fixed_id(8)],
        json!({"absent": "journal_has_no_accounting"})
    );
    assert!(!evidence.join(fixed_id(8)).exists());
}

#[test]
fn evidence_truncates_at_a_call_boundary() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let digest = swarmail_evals::provenance::sha256(b"x");
    let calls: Vec<Value> = (1..=5000)
        .map(|i| {
            json!({
                "ordinal": i, "tool": "mcp__swarmail__wait", "outcome": "ok",
                "argument_bytes": 1, "argument_sha256": digest,
                "result_bytes": 1, "result_sha256": digest,
            })
        })
        .collect();
    let mut facts = facts_value();
    facts["trials"][0]["tool_calls"]["value"] = Value::Array(calls);
    let facts_path = write_facts(dir.path(), &facts);
    let evidence = dir.path().join("evidence");
    let out = run_report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts_path.as_os_str(),
            "--evidence-dir".as_ref(),
            evidence.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let bytes = std::fs::read(evidence.join(fixed_id(3)).join("tool-calls.json")).unwrap();
    assert!(bytes.len() <= 64 * 1024, "{} bytes", bytes.len());
    let record: Value = serde_json::from_slice(&bytes).expect("valid JSON, not cut mid-call");
    assert_eq!(record["truncated"], true);
    let kept = record["tool_calls"].as_array().unwrap();
    assert!(!kept.is_empty() && kept.len() < 5000, "{}", kept.len());
    assert_eq!(
        record["retained"],
        kept.len(),
        "the count of whole calls kept"
    );
    for (i, call) in kept.iter().enumerate() {
        assert_eq!(call["ordinal"], i as u64 + 1, "whole calls, in order");
    }
    // The report still counts every call the journal had.
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let total = group(&report, "claude-a")["journal"]["tool_calls"]["total"]
        .as_u64()
        .unwrap();
    assert_eq!(total, 5000 + 3 + 1);
}

#[test]
fn evidence_never_clobbers_and_never_goes_silent() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let facts_path = golden_facts();
    let evidence = dir.path().join("evidence");
    // A pre-existing target for one trial.
    let taken = evidence.join(fixed_id(3));
    std::fs::create_dir_all(&taken).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&evidence, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::set_permissions(&taken, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    std::fs::write(taken.join("tool-calls.json"), "precious").unwrap();
    let out = run_report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts_path.as_os_str(),
            "--evidence-dir".as_ref(),
            evidence.as_os_str(),
        ],
    );
    assert!(
        out.status.success(),
        "the report still completes\n{}",
        stderr(&out)
    );
    assert_eq!(
        std::fs::read_to_string(taken.join("tool-calls.json")).unwrap(),
        "precious",
        "an existing record is never touched"
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let evidence_index = &group(&report, "claude-a")["journal"]["evidence"];
    assert_eq!(
        evidence_index[fixed_id(3)],
        json!({"absent": "evidence_write_failed"})
    );
    assert!(
        evidence_index[fixed_id(4)]["sha256"].is_string(),
        "the others were written"
    );
}

#[test]
fn evidence_requires_a_journal() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let evidence = dir.path().join("evidence");
    let out = run_report(
        &rows_path,
        &["--evidence-dir".as_ref(), evidence.as_os_str()],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        stderr(&out).contains("evidence_requires_journal"),
        "{}",
        stderr(&out)
    );
    assert!(!evidence.exists());
}

#[test]
fn a_present_empty_tool_list_is_a_record_of_zero_calls() {
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let mut facts = facts_value();
    facts["trials"][1]["tool_calls"]["value"] = json!([]);
    let facts_path = write_facts(dir.path(), &facts);
    let evidence = dir.path().join("evidence");
    let out = run_report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts_path.as_os_str(),
            "--evidence-dir".as_ref(),
            evidence.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let bytes = std::fs::read(evidence.join(fixed_id(4)).join("tool-calls.json")).unwrap();
    assert!(!bytes.is_empty(), "an empty file is never evidence");
    let record: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record["tool_calls"], json!([]));
    assert_eq!(record["retained"], 0);
    assert_eq!(record["truncated"], false);
}

#[test]
fn untyped_tool_calls_and_deadline_facts_refuse_the_whole_report() {
    // A producer that writes a bare `[]` or `false` for an absent fact would
    // turn "the journal did not say" into "zero calls" and "not killed".
    let dir = tempdir();
    let rows_path = write_rows(dir.path(), &golden_rows());
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/journal-facts-untyped.json");
    let out = run_report(&rows_path, &["--journal".as_ref(), fixture.as_os_str()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "no partial report");
    assert!(stderr(&out).contains("journal_invalid"), "{}", stderr(&out));
    for (index, field, untyped) in [
        (0, "tool_calls", json!([])),
        (1, "deadline_killed", json!(false)),
        (2, "deadline_killed", json!(true)),
        (2, "tool_calls", Value::Null),
    ] {
        let mut facts = facts_value();
        facts["trials"][index][field] = untyped;
        let path = write_facts(dir.path(), &facts);
        let out = run_report(&rows_path, &["--journal".as_ref(), path.as_os_str()]);
        assert_eq!(out.status.code(), Some(2), "{field}");
        assert!(out.stdout.is_empty(), "{field}");
        assert!(stderr(&out).contains("journal_invalid"), "{field}");
    }
}
