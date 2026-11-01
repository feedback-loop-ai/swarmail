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
    let matrix = ChildGuard {
        child: Some(command.spawn().unwrap()),
    };
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
