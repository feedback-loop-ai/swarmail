//! The runner's contract with an external agent command and its lifecycle
//! (decision 0055 §§7–9): exact argv, identical prompt and environment,
//! no admin inputs exported, truthful exit statuses, and exactly one
//! schema-valid row on every handled ending — spawn failure, startup
//! failure, server death, panic, deadline, interruption.

mod common;

use common::*;
use serde_json::Value;
use std::io::{self, Read};
use std::ops::Deref;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use swarmail_evals::runner::{self, TrialResult};
use swarmail_evals::state::FailureClass;
use swarmail_evals::tasks::Outcome;

fn assert_no_canary_in_file_tree(root: &Path, canary: &[u8]) {
    let mut pending = vec![root.to_path_buf()];
    let mut scanned = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            scanned += 1;
            assert!(
                !bytes.windows(canary.len()).any(|window| window == canary),
                "{}",
                path.display()
            );
        }
    }
    assert!(scanned > 0, "the file tree contains retained artifacts");
}

const OUTPUT_CAPTURE_LIMIT: usize = 64 * 1024;

struct DrainedStream {
    bytes: Vec<u8>,
    total: usize,
    contains_canary: bool,
}

fn drain_output<R: Read>(mut stream: R, canaries: Vec<Vec<u8>>) -> io::Result<DrainedStream> {
    let max_canary = canaries.iter().map(Vec::len).max().unwrap_or(0);
    let mut carry = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut result = DrainedStream {
        bytes: Vec::with_capacity(OUTPUT_CAPTURE_LIMIT),
        total: 0,
        contains_canary: false,
    };
    loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(result);
        }
        result.total = result.total.saturating_add(count);
        let keep = OUTPUT_CAPTURE_LIMIT
            .saturating_sub(result.bytes.len())
            .min(count);
        result.bytes.extend_from_slice(&chunk[..keep]);

        if max_canary != 0 {
            carry.extend_from_slice(&chunk[..count]);
            result.contains_canary |= canaries
                .iter()
                .any(|canary| carry.windows(canary.len()).any(|window| window == canary));
            let retain = max_canary.saturating_sub(1).min(carry.len());
            carry.drain(..carry.len() - retain);
        }
    }
}

struct BoundedOutput {
    output: Output,
    stdout_bytes: usize,
    stderr_bytes: usize,
}

impl Deref for BoundedOutput {
    type Target = Output;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

fn signal_process_group(pid: i32, signal: i32) {
    // SAFETY: the child was placed in a new process group immediately before spawn.
    unsafe {
        libc::kill(-pid, signal);
    }
}

fn output_with_deadline(
    mut command: Command,
    deadline: Duration,
    canaries: &[&[u8]],
) -> BoundedOutput {
    command
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("the CLI starts");
    let pid = child.id() as i32;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr_pipe = child.stderr.take().expect("stderr is piped");
    let patterns: Vec<Vec<u8>> = canaries
        .iter()
        .filter(|canary| !canary.is_empty())
        .map(|canary| canary.to_vec())
        .collect();
    let stdout_patterns = patterns.clone();
    let stdout_reader = std::thread::spawn(move || drain_output(stdout, stdout_patterns));
    let stderr_reader = std::thread::spawn(move || drain_output(stderr_pipe, patterns));

    let end = Instant::now() + deadline;
    let mut status = None;
    while status.is_none() && Instant::now() < end {
        status = child.try_wait().expect("the CLI wait succeeds");
        if status.is_none() {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    let mut expired = status.is_none();
    while status.is_some()
        && !(stdout_reader.is_finished() && stderr_reader.is_finished())
        && Instant::now() < end
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    expired |= !(stdout_reader.is_finished() && stderr_reader.is_finished());

    if expired {
        signal_process_group(pid, libc::SIGTERM);
        let cleanup_end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < cleanup_end {
            if status.is_none() {
                status = child.try_wait().expect("the CLI wait succeeds");
            }
            if status.is_some() && stdout_reader.is_finished() && stderr_reader.is_finished() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        signal_process_group(pid, libc::SIGKILL);
        let drain_end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < drain_end
            && !(stdout_reader.is_finished() && stderr_reader.is_finished())
        {
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    let process_status = child.wait().expect("the CLI process is reaped");
    assert!(
        stdout_reader.is_finished() && stderr_reader.is_finished(),
        "CLI output pipes did not reach EOF after process-group cleanup"
    );
    let stdout = stdout_reader
        .join()
        .expect("stdout collector does not panic")
        .expect("stdout collector drains successfully");
    let stderr_capture = stderr_reader
        .join()
        .expect("stderr collector does not panic")
        .expect("stderr collector drains successfully");
    assert!(
        !stdout.contains_canary && !stderr_capture.contains_canary,
        "CLI output contained a synthetic canary (stdout={} bytes, stderr={} bytes)",
        stdout.total,
        stderr_capture.total
    );
    assert!(
        !expired,
        "CLI exceeded {deadline:?} or left inherited output pipes open"
    );

    BoundedOutput {
        output: Output {
            status: process_status,
            stdout: stdout.bytes,
            stderr: stderr_capture.bytes,
        },
        stdout_bytes: stdout.total,
        stderr_bytes: stderr_capture.total,
    }
}

fn permission_mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

#[test]
fn artifacts_are_private_under_permissive_and_restrictive_umasks() {
    let _trial_scope = trial_scope();
    let canary = "synthetic-artifact-umask-canary-196";
    for mask in ["0002", "0000", "0777"] {
        let owner = tempfile::tempdir().unwrap();
        let artifacts = owner.path().join("selected-artifacts");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::set_permissions(&artifacts, std::fs::Permissions::from_mode(0o750)).unwrap();
        let parent_mode = permission_mode(&artifacts);

        let out = Command::new("/bin/sh")
            .args(["-c", "umask \"$1\"; shift; exec \"$@\"", "sh", mask])
            .arg(exe())
            .args([
                "trial",
                "--task",
                "signup",
                "--agent",
                "null",
                "--artifacts-dir",
            ])
            .arg(&artifacts)
            .env(runner::SWARMAIL_BIN_ENV, swarmail_bin())
            .env("OPENAI_API_KEY", canary)
            .output()
            .expect("the umask-scoped CLI starts");
        let row = one_row(&out);
        assert_eq!(row.outcome, Outcome::Fail, "mask={mask}: {}", stderr(&out));
        assert_eq!(row.failure_class, Some(FailureClass::GaveUp));
        assert_eq!(row.agent_exit_status, Some(0));
        assert_eq!(out.status.code(), Some(1));
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
        assert_eq!(permission_mode(&artifacts), parent_mode);

        let trial_dir = artifacts.join(&row.trial_id);
        assert_eq!(permission_mode(&trial_dir), 0o700, "mask={mask}");
        let names = [
            "state.json",
            "result.json",
            "diagnostics.log",
            "metadata.json",
        ];
        for name in names {
            let path = trial_dir.join(name);
            assert!(path.is_file(), "mask={mask}: {name} exists");
            assert_eq!(permission_mode(&path), 0o600, "mask={mask}: {name}");
        }
        let entries: Vec<_> = std::fs::read_dir(&trial_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert!(entries.len() >= names.len());
        for name in names {
            assert!(trial_dir.join(name).is_file(), "final {name} is published");
        }
        for entry in entries {
            let name = entry.file_name().unwrap().to_string_lossy();
            assert!(
                names.contains(&name.as_ref()) || name.starts_with(".tmp"),
                "only expected files or private tempfile residuals remain"
            );
            assert_eq!(permission_mode(&entry), 0o600);
        }
        let metadata: Value =
            serde_json::from_slice(&std::fs::read(trial_dir.join("metadata.json")).unwrap())
                .unwrap();
        assert_eq!(metadata["swarmail_bin"], Value::Null);
        assert_no_canary_in_file_tree(&artifacts, canary.as_bytes());
    }
}

#[test]
fn helper_agent() {
    maybe_run_helper();
}

#[test]
fn an_external_agent_gets_literal_argv_and_identical_prompt_and_env() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("echo.json");
    let (row, out) = helper_trial(
        "echo",
        &["--task", "signup", "--agent", "echo"],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("OPENAI_API_KEY", "synthetic-provider-key-196"),
            ("UNRELATED_CALLER_SETTING", "synthetic-unrelated-196"),
            ("EVAL_STALE_TOKEN", "synthetic-stale-eval-196"),
            ("EVAL_FIXTURE_ADMIN_SECRET", "synthetic-stale-admin-196"),
            ("SWARMAIL_SECRET", "synthetic-stale-swarmail-196"),
            ("RUST_LOG", "synthetic-caller-log-196"),
        ],
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

    let launch_env_path = format!("{}.launch-env", marker.display());
    let launch_env = std::fs::read(launch_env_path).unwrap();
    assert!(
        !launch_env
            .windows(b"NONUTF8_PARENT_\xff".len())
            .any(|w| w == b"NONUTF8_PARENT_\xff"),
        "caller native environment canary crossed the closed command boundary"
    );
    let pairs: Vec<(&str, &str)> = launch_env
        .split(|b| *b == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let separator = entry.iter().position(|b| *b == b'=').unwrap();
            let (key, value) = (&entry[..separator], &entry[separator + 1..]);
            (
                std::str::from_utf8(key).unwrap(),
                std::str::from_utf8(value).unwrap(),
            )
        })
        .collect();
    let names: std::collections::HashSet<_> = pairs.iter().map(|(key, _)| *key).collect();
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
        "EVAL_TASK_ID",
        "EVAL_APP_URL",
        "EVAL_MCP_URL",
        "EVAL_IDENTITY",
        "EVAL_INBOX",
        "EVAL_GOAL",
        "EVAL_PROMPT_FILE",
        "EVAL_DEADLINE_MS",
        "PWD",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected, "agent gets the entire closed map");
    let value = |key: &str| pairs.iter().find(|(name, _)| *name == key).unwrap().1;
    assert_eq!(value("PATH"), "/usr/bin:/bin");
    assert_eq!(value("LANG"), "C");
    assert_eq!(value("LC_ALL"), "C");
    assert_eq!(value("PWD"), report["cwd"].as_str().unwrap());
    let private = [
        value("HOME"),
        value("TMPDIR"),
        value("XDG_CONFIG_HOME"),
        value("XDG_CACHE_HOME"),
        value("XDG_DATA_HOME"),
        value("XDG_STATE_HOME"),
    ];
    assert!(private.iter().all(|path| path.starts_with("/tmp/")));
    assert_eq!(
        private
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        6
    );
    let launch_env_text = String::from_utf8_lossy(&launch_env);
    for canary in [
        "synthetic-provider-key-196",
        "synthetic-unrelated-196",
        "synthetic-stale-eval-196",
        "synthetic-stale-admin-196",
        "synthetic-stale-swarmail-196",
        "synthetic-caller-log-196",
    ] {
        assert!(
            !launch_env_text.contains(canary),
            "agent map leaked {canary}"
        );
    }
}

#[test]
fn relative_and_bare_agent_programs_resolve_at_the_caller_boundary() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let tool_dir = dir.path().join("tools");
    std::fs::create_dir(&tool_dir).unwrap();
    let relative_marker = dir.path().join("relative-ran");
    let bare_marker = dir.path().join("bare-ran");
    let binary = exe();
    let relative = dir.path().join("relative-agent");
    let bare = tool_dir.join("bare-agent");
    let script = |marker: &std::path::Path| {
        format!(
            "#!/bin/sh\nprintf ran > '{}'\nexec '{}' agent oracle\n",
            marker.display(),
            binary,
        )
    };
    std::fs::write(&relative, script(&relative_marker)).unwrap();
    std::fs::write(&bare, script(&bare_marker)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&relative, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::set_permissions(&bare, std::fs::Permissions::from_mode(0o700)).unwrap();

    let relative_out = cli()
        .current_dir(dir.path())
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "relative",
            "--",
            "./relative-agent",
        ])
        .output()
        .unwrap();
    assert_eq!(
        one_row(&relative_out).outcome,
        Outcome::Pass,
        "{}",
        stderr(&relative_out)
    );
    assert_eq!(std::fs::read(relative_marker).unwrap(), b"ran");

    let bare_out = cli()
        .current_dir(dir.path())
        .env("PATH", &tool_dir)
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "bare",
            "--",
            "bare-agent",
        ])
        .output()
        .unwrap();
    assert_eq!(
        one_row(&bare_out).outcome,
        Outcome::Pass,
        "{}",
        stderr(&bare_out)
    );
    assert_eq!(std::fs::read(bare_marker).unwrap(), b"ran");
}

fn private_runtime_paths(environment: &[u8]) -> std::collections::HashSet<Vec<u8>> {
    const NAMES: [&[u8]; 6] = [
        b"HOME",
        b"TMPDIR",
        b"XDG_CONFIG_HOME",
        b"XDG_CACHE_HOME",
        b"XDG_DATA_HOME",
        b"XDG_STATE_HOME",
    ];
    environment
        .split(|byte| *byte == 0)
        .filter_map(|entry| {
            let separator = entry.iter().position(|byte| *byte == b'=')?;
            NAMES
                .contains(&&entry[..separator])
                .then(|| entry[separator + 1..].to_vec())
        })
        .collect()
}

fn capture_then_exec(
    directory: &std::path::Path,
    name: &str,
    program: &std::path::Path,
) -> std::path::PathBuf {
    let capture = directory.join(format!("{name}.env"));
    let wrapper = directory.join(format!("{name}.sh"));
    let script = format!(
        "#!/bin/sh\nif [ -r /proc/$$/environ ]; then /bin/cat /proc/$$/environ > '{}'; else unset PWD; /usr/bin/env -0 > '{}'; fi\nexec '{}' \"$@\"\n",
        capture.display(),
        capture.display(),
        program.display(),
    );
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    wrapper
}

#[cfg(target_os = "linux")]
#[test]
fn fixture_gets_only_its_private_runtime_map_and_generated_admin_input() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("fixture-env");
    let (row, out) = helper_trial(
        "fixture_env_probe",
        &["--task", "signup", "--agent", "probe"],
        &[
            (
                MARKER_ENV,
                dir.path().join("agent-marker").to_str().unwrap(),
            ),
            ("EVAL_TEST_FIXTURE_ENV_MARKER", marker.to_str().unwrap()),
            ("OPENAI_API_KEY", "synthetic-provider-key-196"),
            ("UNRELATED_CALLER_SETTING", "synthetic-unrelated-196"),
            ("EVAL_STALE_TOKEN", "synthetic-stale-eval-196"),
        ],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    let bytes = std::fs::read(marker).unwrap();
    assert!(
        !bytes
            .windows(b"NONUTF8_PARENT_\xff".len())
            .any(|w| w == b"NONUTF8_PARENT_\xff"),
        "caller native environment canary crossed the closed command boundary"
    );
    let pairs: Vec<(String, String)> = bytes
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let index = part.iter().position(|b| *b == b'=').unwrap();
            (
                String::from_utf8(part[..index].to_vec()).unwrap(),
                String::from_utf8(part[index + 1..].to_vec()).unwrap(),
            )
        })
        .collect();
    let names: std::collections::HashSet<_> = pairs.iter().map(|(key, _)| key.as_str()).collect();
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
        "EVAL_FIXTURE_ADMIN_SECRET",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        names, expected,
        "fixture gets only runtime defaults and its admin input"
    );
    let agent_environment = std::fs::read(dir.path().join("agent-marker.launch-env")).unwrap();
    let agent_paths = private_runtime_paths(&agent_environment);
    let fixture_paths = private_runtime_paths(&bytes);
    assert_eq!(agent_paths.len(), 6);
    assert_eq!(fixture_paths.len(), 6);
    assert!(
        agent_paths.is_disjoint(&fixture_paths),
        "fixture and agent private directories are distinct"
    );
    let secret = pairs
        .iter()
        .find(|(key, _)| key == "EVAL_FIXTURE_ADMIN_SECRET")
        .unwrap()
        .1
        .as_str();
    assert_eq!(secret.len(), 32);
    assert!(secret.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-provider-key-196"));
    assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-unrelated-196"));
    assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-stale-eval-196"));
}

#[test]
fn swarmail_gets_only_its_fixed_runtime_map() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let wrapper = capture_then_exec(dir.path(), "swarmail", swarmail_bin());
    let out = cli()
        .env("SWARMAIL_BIN", &wrapper)
        .env("OPENAI_API_KEY", "synthetic-provider-key-196")
        .env("UNRELATED_CALLER_SETTING", "synthetic-unrelated-196")
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .output()
        .unwrap();
    assert_eq!(one_row(&out).outcome, Outcome::Pass, "{}", stderr(&out));
    let bytes = std::fs::read(dir.path().join("swarmail.env")).unwrap();
    assert!(
        !bytes
            .windows(b"NONUTF8_PARENT_\xff".len())
            .any(|w| w == b"NONUTF8_PARENT_\xff"),
        "caller native environment canary crossed the closed command boundary"
    );
    let pairs: Vec<(String, String)> = bytes
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let index = part.iter().position(|b| *b == b'=').unwrap();
            (
                String::from_utf8(part[..index].to_vec()).unwrap(),
                String::from_utf8(part[index + 1..].to_vec()).unwrap(),
            )
        })
        .collect();
    let names: std::collections::HashSet<_> = pairs.iter().map(|(key, _)| key.as_str()).collect();
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
        "RUST_LOG",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected);
    assert_eq!(
        pairs.iter().find(|(k, _)| k == "RUST_LOG").unwrap().1,
        "swarmail=info,tower_http=off"
    );
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("synthetic-provider-key-196"));
    assert!(!text.contains("synthetic-unrelated-196"));
}

#[test]
fn readiness_discards_an_oversized_preannouncement_flood() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let binary = swarmail_bin();
    let wrapper = dir.path().join("noisy-swarmail.sh");
    let script = format!(
        "#!/bin/sh\ndd if=/dev/zero bs=8192 count=128 2>/dev/null\nprintf '\\012'\nexec '{}' \"$@\"\n",
        binary.display(),
    );
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let out = cli()
        .env("SWARMAIL_BIN", &wrapper)
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .output()
        .unwrap();
    assert_eq!(one_row(&out).outcome, Outcome::Pass, "{}", stderr(&out));
    let original = swarmail_evals::results::read(&out.stdout).unwrap();
    let mut execution = serde_json::to_value(original).unwrap();
    execution.as_object_mut().unwrap().remove("provenance");
    assert!(serde_json::to_vec(&execution).unwrap().len() < 2048);
    assert!(out.stdout.len() <= swarmail_evals::provenance::RESULT_LIMIT);
    assert!(out.stderr.len() < 2048);
}

#[test]
fn readiness_continues_draining_after_the_first_valid_announcement() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("post-announcement-flood.py");
    let binary = swarmail_bin().to_str().unwrap();
    let script = format!(
        "#!/usr/bin/python3\nimport subprocess, sys\nservice = subprocess.Popen([{binary:?}, *sys.argv[1:]], stdout=subprocess.PIPE)\nannouncement = service.stdout.readline()\nsys.stdout.buffer.write(announcement)\nsys.stdout.buffer.flush()\nsys.stdout.buffer.write(bytes(1024 * 1024))\nsys.stdout.buffer.write(b'\\n')\nsys.stdout.buffer.flush()\nsys.exit(service.wait())\n",
    );
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let out = cli()
        .env("SWARMAIL_BIN", &wrapper)
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .output()
        .unwrap();
    assert_eq!(one_row(&out).outcome, Outcome::Pass, "{}", stderr(&out));
    let original = swarmail_evals::results::read(&out.stdout).unwrap();
    let mut execution = serde_json::to_value(original).unwrap();
    execution.as_object_mut().unwrap().remove("provenance");
    assert!(serde_json::to_vec(&execution).unwrap().len() < 2048);
    assert!(out.stdout.len() <= swarmail_evals::provenance::RESULT_LIMIT);
    assert!(out.stderr.len() < 2048);
}

#[test]
fn automatic_build_uses_a_cleared_named_preparation_map() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let custom_target = dir.path().join("custom-cargo-target");
    let cargo = runner::resolve_agent_command(&["cargo".into()])
        .unwrap()
        .remove(0);
    let wrapper = dir.path().join("cargo-wrapper.sh");
    let capture = dir.path().join("cargo.env");
    let script = format!(
        "#!/bin/sh\nif [ -r /proc/$$/environ ]; then /bin/cat /proc/$$/environ > '{}'; else unset PWD; /usr/bin/env -0 > '{}'; fi\nexec '{}' \"$@\"\n",
        capture.display(),
        capture.display(),
        std::path::PathBuf::from(&cargo).display(),
    );
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();

    let out = Command::new(exe())
        .env(
            std::ffi::OsString::from_vec(b"NONUTF8_PARENT_\xff".to_vec()),
            std::ffi::OsString::from_vec(b"synthetic_\xfe_value".to_vec()),
        )
        .env_remove("SWARMAIL_BIN")
        .env("CARGO", &wrapper)
        .env("CARGO_TARGET_DIR", &custom_target)
        .env("CARGO_REGISTRY_TOKEN", "synthetic-registry-token-196")
        .env("RUSTFLAGS", "synthetic-rustflags-196")
        .env("HTTP_PROXY", "http://synthetic-proxy.invalid")
        .args(["trial", "--task", "signup", "--agent", "null"])
        .output()
        .unwrap();
    assert!(
        capture.is_file(),
        "the selected Cargo wrapper ran: {}",
        stderr(&out)
    );
    assert_eq!(one_row(&out).outcome, Outcome::Fail, "{}", stderr(&out));
    let bytes = std::fs::read(capture).unwrap();
    let pairs: Vec<(String, String)> = bytes
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let index = part.iter().position(|b| *b == b'=').unwrap();
            (
                String::from_utf8(part[..index].to_vec()).unwrap(),
                String::from_utf8(part[index + 1..].to_vec()).unwrap(),
            )
        })
        .collect();
    let names: std::collections::HashSet<_> = pairs.iter().map(|(key, _)| key.as_str()).collect();
    let mut expected: std::collections::HashSet<_> = [
        "PATH",
        "LANG",
        "LC_ALL",
        "HOME",
        "TMPDIR",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "RUSTC",
        "RUSTDOC",
        "CARGO_HOME",
        "RUSTUP_HOME",
    ]
    .into_iter()
    .collect();
    if std::env::var_os("RUSTUP_TOOLCHAIN").is_some_and(|v| !v.is_empty()) {
        expected.insert("RUSTUP_TOOLCHAIN");
    }
    expected.insert("CARGO_TARGET_DIR");
    assert_eq!(names, expected, "cleared Cargo environment");
    let text = String::from_utf8_lossy(&bytes);
    for canary in [
        "synthetic-registry-token-196",
        "synthetic-rustflags-196",
        "synthetic-proxy.invalid",
    ] {
        assert!(!text.contains(canary));
    }
    let value = |key: &str| {
        pairs
            .iter()
            .find(|(name, _)| name == key)
            .unwrap()
            .1
            .clone()
    };
    for key in ["RUSTC", "RUSTDOC", "CARGO_HOME", "RUSTUP_HOME"] {
        assert!(
            std::path::Path::new(&value(key)).is_absolute(),
            "{key} is absolute"
        );
    }
    let rustc = runner::resolve_agent_command(&["rustc".into()])
        .unwrap()
        .remove(0);
    let rustdoc = runner::resolve_agent_command(&["rustdoc".into()])
        .unwrap()
        .remove(0);
    assert_eq!(
        value("RUSTC"),
        std::path::PathBuf::from(rustc).to_string_lossy()
    );
    assert_eq!(
        value("RUSTDOC"),
        std::path::PathBuf::from(rustdoc).to_string_lossy()
    );

    let mut expected_path = Vec::new();
    for program in [
        wrapper.as_path(),
        std::path::Path::new(&value("RUSTC")),
        std::path::Path::new(&value("RUSTDOC")),
        std::path::Path::new("/usr/bin"),
        std::path::Path::new("/bin"),
    ] {
        if let Some(parent) = if program.is_dir() {
            Some(program)
        } else {
            program.parent()
        } && !expected_path.iter().any(|path| path == parent)
        {
            expected_path.push(parent.to_path_buf());
        }
    }
    assert_eq!(
        std::env::split_paths(std::ffi::OsStr::new(&value("PATH"))).collect::<Vec<_>>(),
        expected_path
    );
    let cwd = std::env::current_dir().unwrap();
    let original_home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let original_home = if original_home.is_absolute() {
        original_home
    } else {
        cwd.join(original_home)
    };
    for (key, suffix) in [("CARGO_HOME", ".cargo"), ("RUSTUP_HOME", ".rustup")] {
        let expected = std::env::var_os(key)
            .map(std::path::PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                }
            })
            .unwrap_or_else(|| original_home.join(suffix));
        assert_eq!(std::path::PathBuf::from(value(key)), expected);
    }
    assert_eq!(
        std::path::PathBuf::from(value("CARGO_TARGET_DIR")),
        custom_target
    );
    assert!(
        custom_target.join("debug/swarmail").is_file(),
        "Cargo's reported executable was built under the selected custom target directory"
    );
}

#[test]
fn automatic_build_accepts_explicit_homes_without_caller_home() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let cargo_home = dir.path().join("selected-cargo-home");
    let rustup_home = dir.path().join("selected-rustup-home");
    let artifact = serde_json::json!({
        "reason": "compiler-artifact",
        "target": { "name": "swarmail", "kind": ["bin"] },
        "executable": swarmail_bin().to_string_lossy(),
    });
    std::fs::write(
        dir.path().join("compiler-artifact.json"),
        format!("{}\n", serde_json::to_string(&artifact).unwrap()),
    )
    .unwrap();
    let wrapper = dir.path().join("cargo-selector-wrapper.sh");
    let capture = dir.path().join("cargo-selector.env");
    let script = r#"#!/bin/sh
set -eu
base=$(CDPATH= cd "$(dirname "$0")" && pwd)
env -0 > "$base/cargo-selector.env"
cat "$base/compiler-artifact.json"
"#;
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();

    let out = Command::new(exe())
        .env_remove("SWARMAIL_BIN")
        .env_remove("HOME")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env("CARGO", &wrapper)
        .env("CARGO_HOME", &cargo_home)
        .env("RUSTUP_HOME", &rustup_home)
        .args(["trial", "--task", "signup", "--agent", "null"])
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Fail);
    let bytes = std::fs::read(capture).unwrap();
    let variables: std::collections::HashMap<String, String> = bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let split = entry.iter().position(|byte| *byte == b'=').unwrap();
            (
                String::from_utf8(entry[..split].to_vec()).unwrap(),
                String::from_utf8(entry[split + 1..].to_vec()).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        variables.get("CARGO_HOME").map(String::as_str),
        Some(cargo_home.to_str().unwrap())
    );
    assert_eq!(
        variables.get("RUSTUP_HOME").map(String::as_str),
        Some(rustup_home.to_str().unwrap())
    );
    assert!(variables.contains_key("HOME"), "child HOME remains private");

    for empty_cargo_home in [false, true] {
        let mut command = Command::new(exe());
        command
            .env_remove("SWARMAIL_BIN")
            .env_remove("HOME")
            .env_remove("CARGO")
            .env_remove("CARGO_HOME")
            .env_remove("RUSTUP_HOME")
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("RUSTUP_TOOLCHAIN");
        if empty_cargo_home {
            command
                .env("CARGO_HOME", "")
                .env("RUSTUP_HOME", &rustup_home);
        }
        let out = command
            .args(["trial", "--task", "signup", "--agent", "null"])
            .output()
            .unwrap();
        let row = one_row(&out);
        assert_eq!(row.outcome, Outcome::HarnessError);
        assert_eq!(out.status.code(), Some(2));
        assert!(stderr(&out).contains("swarmail_unavailable"));
    }
}

#[test]
fn oversized_cargo_record_is_drained_and_fails_without_echo() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let cargo = runner::resolve_agent_command(&["cargo".into()])
        .unwrap()
        .remove(0);
    let wrapper = dir.path().join("oversized-cargo.sh");
    let canary = "synthetic-cargo-output-canary-196";
    let record = serde_json::json!({
        "reason": "compiler-artifact",
        "target": { "name": "swarmail", "kind": ["bin"] },
        "executable": swarmail_bin().to_string_lossy(),
        "filler": format!("{canary}{}", "x".repeat(1024 * 1024)),
    });
    let record_path = dir.path().join("oversized-record.json");
    let record_bytes = serde_json::to_vec(&record).unwrap();
    assert!(record_bytes.len() > 1024 * 1024);
    std::fs::write(&record_path, &record_bytes).unwrap();
    let cargo = std::path::PathBuf::from(cargo)
        .to_string_lossy()
        .into_owned();
    let cargo = format!("'{}'", cargo.replace('\'', "'\\''"));
    let script = format!(
        r#"#!/bin/sh
set -eu
base=$(CDPATH= cd "$(dirname "$0")" && pwd)
: > "$base/producer-started"
wc -c < "$base/oversized-record.json" | tr -d '[:space:]' > "$base/producer-bytes"
bytes=$(cat "$base/producer-bytes")
[ "$bytes" -gt 1048576 ]
cat "$base/oversized-record.json"
printf '\n'
: > "$base/producer-complete"
if {cargo} "$@"; then status=0; else status=$?; fi
printf '%s\n' "$status" > "$base/real-cargo-status"
exit "$status"
"#
    );
    std::fs::write(&wrapper, &script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let artifacts = dir.path().join("cargo-artifacts");
    let mut command = Command::new(exe());
    command
        .env(
            std::ffi::OsString::from_vec(b"NONUTF8_PARENT_\xff".to_vec()),
            std::ffi::OsString::from_vec(b"synthetic_\xfe_value".to_vec()),
        )
        .env_remove("SWARMAIL_BIN")
        .env("CARGO", &wrapper)
        .env("OPENAI_API_KEY", canary)
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "null",
            "--artifacts-dir",
            artifacts.to_str().unwrap(),
        ]);
    let out = output_with_deadline(command, Duration::from_secs(180), &[canary.as_bytes()]);
    assert!(out.stdout_bytes <= OUTPUT_CAPTURE_LIMIT);
    assert!(out.stderr_bytes <= OUTPUT_CAPTURE_LIMIT);
    let producer_dir = wrapper.parent().unwrap();
    assert!(producer_dir.join("producer-started").is_file());
    assert!(producer_dir.join("producer-complete").is_file());
    assert_eq!(
        std::fs::read_to_string(producer_dir.join("producer-bytes"))
            .unwrap()
            .trim()
            .parse::<usize>()
            .unwrap(),
        record_bytes.len()
    );
    assert_eq!(
        std::fs::read_to_string(producer_dir.join("real-cargo-status"))
            .unwrap()
            .trim(),
        "0"
    );
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(stderr(&out).contains("swarmail_unavailable"));
    let leaf = artifacts.join(&row.trial_id);
    let retained = swarmail_evals::artifacts::trial(&leaf).unwrap();
    assert_eq!(retained.row, row);
    assert!(retained.state.is_none());
    for name in ["metadata.json", "result.json", "diagnostics.log"] {
        let payload = std::fs::read(leaf.join(name)).unwrap();
        assert!(
            !payload
                .windows(canary.len())
                .any(|w| w == canary.as_bytes()),
            "private preparation output excluded"
        );
    }
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

    let short_record = serde_json::json!({
        "reason": "compiler-artifact",
        "target": { "name": "swarmail", "kind": ["bin"] },
        "executable": swarmail_bin().to_string_lossy(),
        "filler": canary,
    });
    let short_record_bytes = serde_json::to_vec(&short_record).unwrap();
    assert!(short_record_bytes.len() < 1024 * 1024);
    std::fs::write(dir.path().join("short-record.json"), &short_record_bytes).unwrap();
    let short_wrapper = dir.path().join("short-cargo.sh");
    for marker in [
        "producer-started",
        "producer-bytes",
        "producer-complete",
        "real-cargo-status",
    ] {
        std::fs::remove_file(producer_dir.join(marker)).unwrap();
    }
    std::fs::write(
        &short_wrapper,
        script
            .replace("oversized-record.json", "short-record.json")
            .replace("-gt 1048576", "-lt 1048576"),
    )
    .unwrap();
    std::fs::set_permissions(&short_wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut short_command = Command::new(exe());
    short_command
        .env_remove("SWARMAIL_BIN")
        .env("CARGO", &short_wrapper)
        .env("OPENAI_API_KEY", canary)
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "null",
            "--artifacts-dir",
            artifacts.to_str().unwrap(),
        ]);
    let short_out = output_with_deadline(
        short_command,
        Duration::from_secs(180),
        &[canary.as_bytes()],
    );
    assert!(short_out.stdout_bytes <= OUTPUT_CAPTURE_LIMIT);
    assert!(short_out.stderr_bytes <= OUTPUT_CAPTURE_LIMIT);
    let short_row = one_row(&short_out);
    assert_eq!(short_row.outcome, Outcome::Fail);
    assert_no_canary_in_file_tree(&artifacts.join(&short_row.trial_id), canary.as_bytes());
    assert!(producer_dir.join("producer-started").is_file());
    assert!(producer_dir.join("producer-complete").is_file());
    assert_eq!(
        std::fs::read_to_string(producer_dir.join("producer-bytes"))
            .unwrap()
            .trim()
            .parse::<usize>()
            .unwrap(),
        short_record_bytes.len()
    );
    assert_eq!(
        std::fs::read_to_string(producer_dir.join("real-cargo-status"))
            .unwrap()
            .trim(),
        "0"
    );
    assert!(
        !short_out
            .stdout
            .windows(canary.len())
            .any(|w| w == canary.as_bytes())
    );
    assert!(
        !short_out
            .stderr
            .windows(canary.len())
            .any(|w| w == canary.as_bytes())
    );
}

#[test]
fn agent_argv_stays_literal_and_wrapper_config_is_explicit() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let arg_marker = dir.path().join("literal-args");
    let literal = [
        "two words",
        "semi;colon",
        "$(touch never-run)",
        "quote'check",
    ];
    let (row, out) = helper_trial_with_agent_args(
        "succeed_then_crash",
        &["--task", "signup", "--agent", "literal"],
        &[("EVAL_TEST_ARG_MARKER", arg_marker.to_str().unwrap())],
        &literal,
    );
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.agent_exit_status, Some(7));
    let mut expected = literal.join("\0").into_bytes();
    expected.push(0);
    assert_eq!(std::fs::read(arg_marker).unwrap(), expected);

    let marker = dir.path().join("config-marker");
    let config = dir.path().join("private-agent-config");
    std::fs::write(&config, "credential=synthetic-wrapper-secret-196\n").unwrap();
    let (row, out) = helper_trial(
        "file_config",
        &["--task", "signup", "--agent", "configured"],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("EVAL_TEST_AUTH_CONFIG", config.to_str().unwrap()),
        ],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(std::fs::read(marker).unwrap(), b"config-read");
    let secret = b"synthetic-wrapper-secret-196";
    assert!(!out.stdout.windows(secret.len()).any(|b| b == secret));
    assert!(!out.stderr.windows(secret.len()).any(|b| b == secret));
}

#[test]
fn two_trials_use_fresh_ports_and_identities() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let trials: Vec<(String, String, std::collections::HashSet<Vec<u8>>)> = (0..2)
        .map(|i| {
            let marker = dir.path().join(format!("{i}.json"));
            helper_trial(
                "echo",
                &["--task", "signup", "--agent", "echo"],
                &[(MARKER_ENV, marker.to_str().unwrap())],
            );
            let report: Value = serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
            assert_ne!(
                report["home"].as_str().unwrap(),
                std::env::var("HOME").unwrap()
            );
            assert_eq!(report["home_sentinel_was_present"], false);
            assert_eq!(report["home_sentinel_written"], true);
            let env: Vec<(String, String)> = serde_json::from_value(report["env"].clone()).unwrap();
            let get = |k: &str| env.iter().find(|(n, _)| n == k).unwrap().1.clone();
            let launch_env = std::fs::read(format!("{}.launch-env", marker.display())).unwrap();
            (
                get("EVAL_MCP_URL"),
                get("EVAL_IDENTITY"),
                private_runtime_paths(&launch_env),
            )
        })
        .collect();
    assert_ne!(
        (trials[0].0.as_str(), trials[0].1.as_str()),
        (trials[1].0.as_str(), trials[1].1.as_str())
    );
    assert!(
        trials[0].2.is_disjoint(&trials[1].2),
        "trial private directories are distinct"
    );
}

#[test]
fn a_spawn_failure_is_a_harness_error_with_null_statuses() {
    let _trial_scope = trial_scope();
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
fn direct_trial_preserves_non_utf8_later_arguments() {
    let _trial_scope = trial_scope();
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("argument-bytes");
    let program = dir.path().join("native-argument-agent.sh");
    std::fs::write(
        &program,
        "#!/bin/sh\nprintf '%s\\0' \"$2\" > \"$1\"\nexec \"$3\" agent oracle\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let argument = OsString::from_vec(b"literal-argument-\xff-196".to_vec());
    let out = cli()
        .args(["trial", "--task", "signup", "--agent", "native-agent", "--"])
        .arg(&program)
        .arg(&marker)
        .arg(&argument)
        .arg(exe())
        .output()
        .expect("the direct trial CLI starts");
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.agent_exit_status, Some(0));
    assert!(out.status.success(), "{}", stderr(&out));
    let mut expected = argument.into_vec();
    expected.push(0);
    assert_eq!(std::fs::read(marker).unwrap(), expected);
}

#[test]
fn missing_program_and_bad_clap_values_do_not_echo_input_text() {
    let _trial_scope = trial_scope();
    let path = "/tmp/synthetic-missing-program-path-196";
    let (row, out) = trial(&["--task", "signup", "--agent", "missing", "--", path]);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(!stderr(&out).contains(path));
    assert!(stderr(&out).contains("program_unavailable"));

    let canary = "synthetic-invalid-cli-value-196";
    let out = Command::new(exe())
        .env(
            std::ffi::OsString::from_vec(b"NONUTF8_PARENT_\xff".to_vec()),
            std::ffi::OsString::from_vec(b"synthetic_\xfe_value".to_vec()),
        )
        .env("OPENAI_API_KEY", canary)
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "oracle",
            "--deadline-ms",
            canary,
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        !out.stderr
            .windows(canary.len())
            .any(|w| w == canary.as_bytes())
    );
}

#[test]
fn artifact_write_failures_use_safe_categories() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let not_directory = dir.path().join("caller-selected-artifact-path-196");
    std::fs::write(&not_directory, b"file").unwrap();
    let (row, out) = trial(&[
        "--task",
        "signup",
        "--agent",
        "oracle",
        "--artifacts-dir",
        not_directory.to_str().unwrap(),
    ]);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(stderr(&out).contains("artifact_write_failed"));
    assert!(!stderr(&out).contains(not_directory.to_str().unwrap()));
}

#[test]
fn a_swarmail_that_never_comes_up_is_a_harness_error() {
    let _trial_scope = trial_scope();
    let out = cli()
        .env("SWARMAIL_BIN", "/bin/false")
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::HarnessError);
    assert!(
        stderr(&out).contains("service_announcement_invalid")
            || stderr(&out).contains("service_exited_before_ready"),
        "safe startup category: {}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("false"),
        "binary path is not reported"
    );
}

#[test]
fn invalid_inputs_are_one_harness_error_row_with_the_supplied_labels() {
    let _trial_scope = trial_scope();
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
    let _trial_scope = trial_scope();
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
    let _trial_scope = trial_scope();
    let (row, out) = helper_trial(
        "succeed_then_crash",
        &["--task", "signup", "--agent", "crashy"],
        &[],
    );
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.agent_exit_status, Some(7));
}

#[test]
fn child_output_canaries_are_drained_without_publication() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("emitter.marker");
    let canary_file = dir.path().join("independent-canary.txt");
    let artifacts = dir.path().join("artifacts");
    let canary = "synthetic-output-canary-196";
    let mut canary_bytes = Vec::new();
    let mut pattern = canary.as_bytes().to_vec();
    pattern.extend_from_slice(b"ab\xf0\x9f\x92\xa9\xfftail");
    while canary_bytes.len() <= 64 * 1024 {
        canary_bytes.extend_from_slice(&pattern);
    }
    assert!(canary_bytes.len() > 64 * 1024);
    std::fs::write(&canary_file, &canary_bytes).unwrap();
    let (row, out) = helper_trial(
        "unsafe_output",
        &[
            "--task",
            "signup",
            "--agent",
            "emitter",
            "--artifacts-dir",
            artifacts.to_str().unwrap(),
        ],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("EVAL_TEST_CANARY_FILE", canary_file.to_str().unwrap()),
        ],
    );
    assert_eq!(row.outcome, Outcome::Fail, "{}", stderr(&out));
    assert_eq!(row.agent_exit_status, Some(0));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap(),
        format!("emitted_bytes_per_stream={}", canary_bytes.len())
    );
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

    let mut pending = vec![artifacts.clone()];
    let mut metadata = None;
    let mut scanned_files = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                assert_eq!(permission_mode(&path), 0o700);
                pending.push(path);
                continue;
            }
            assert_eq!(permission_mode(&path), 0o600);
            let bytes = std::fs::read(&path).unwrap();
            scanned_files += 1;
            assert!(
                !bytes.windows(canary.len()).any(|w| w == canary.as_bytes()),
                "{}",
                path.display()
            );
            assert!(
                !bytes.contains(&0xff),
                "invalid child output is not published in {}",
                path.display()
            );
            if path.file_name().unwrap() == "metadata.json" {
                metadata = Some(serde_json::from_slice::<Value>(&bytes).unwrap());
            }
        }
    }
    assert!(
        scanned_files >= 3,
        "all retained and temporary files were scanned"
    );
    assert_eq!(metadata.unwrap()["swarmail_bin"], Value::Null);
}

#[cfg(target_os = "linux")]
#[test]
fn child_output_server_death_remains_a_harness_error_on_linux() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("emitter.marker");
    let canary_file = dir.path().join("independent-canary.txt");
    let canary = b"synthetic-output-canary-196\xffsplit";
    std::fs::write(&canary_file, canary).unwrap();
    let (row, out) = helper_trial(
        "unsafe_output_then_server_death",
        &["--task", "signup", "--agent", "emitter"],
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("EVAL_TEST_CANARY_FILE", canary_file.to_str().unwrap()),
        ],
    );
    assert_eq!(row.outcome, Outcome::HarnessError, "{}", stderr(&out));
    assert_eq!(
        std::fs::read_to_string(marker).unwrap(),
        format!("emitted_bytes_per_stream={}", canary.len())
    );
}

#[test]
fn a_noisy_agent_is_drained_and_does_not_hang_the_trial() {
    let _trial_scope = trial_scope();
    let (row, out) = helper_trial("noisy", &["--task", "signup", "--agent", "noisy"], &[]);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert!(
        !stderr(&out).contains(&"x".repeat(256)),
        "child output is discarded"
    );
}

#[test]
fn normal_trial_binary_ignores_the_former_runner_fault_control() {
    let _trial_scope = trial_scope();
    for fault in ["panic", "server_death"] {
        let (row, out) = helper_trial(
            "echo",
            &["--task", "signup", "--agent", "echo"],
            &[("EVAL_RUNNER_FAULT", fault)],
        );
        assert_ne!(
            row.outcome,
            Outcome::HarnessError,
            "{fault}: {}",
            stderr(&out)
        );
        assert!(!stderr(&out).contains("injected runner fault"));
    }
    let dir = tempfile::tempdir().unwrap();
    let controls = [
        ("EVAL_TEST_CLEANUP_WAIT_BYPASS", "1".to_string()),
        (
            "EVAL_TEST_CLEANUP_HOLD_DIR",
            dir.path().join("cleanup-hold").display().to_string(),
        ),
        (
            "EVAL_TEST_PRE_HANDLER_HOLD_DIR",
            dir.path().join("pre-handler-hold").display().to_string(),
        ),
        (
            "EVAL_TEST_RUNNER_EXECUTABLE",
            dir.path().join("runner-exe").display().to_string(),
        ),
        (
            "EVAL_TEST_CLEANUP_WAIT_PENDING",
            dir.path().join("must-not-be-created").display().to_string(),
        ),
        (
            "EVAL_TEST_RUNNER_PIDS",
            dir.path().join("agent-pids").display().to_string(),
        ),
        (
            "EVAL_TEST_RUNNER_FAULT_RELEASE",
            dir.path().join("fault-release").display().to_string(),
        ),
        (
            "EVAL_TEST_CLEANUP_BUDGET_HOLD_DIR",
            dir.path().join("budget-hold").display().to_string(),
        ),
        (
            "EVAL_TEST_CLEANUP_BUDGET_RELEASE",
            dir.path().join("budget-release").display().to_string(),
        ),
        (
            "EVAL_TEST_CLEANUP_BUDGET_LOG",
            dir.path().join("budget-log").display().to_string(),
        ),
        ("EVAL_TEST_CLEANUP_OBSERVE_ERROR", "1".to_string()),
        ("EVAL_TEST_CLEANUP_FORCE_EXPIRY", "1".to_string()),
        (
            "EVAL_TEST_RUNNER_SIGNAL_LOG",
            dir.path().join("signal-log").display().to_string(),
        ),
    ];
    let env = controls
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    let (row, out) = helper_trial("echo", &["--task", "signup", "--agent", "echo"], &env);
    assert_ne!(row.outcome, Outcome::HarnessError, "{}", stderr(&out));
    for (name, value) in &controls {
        if value != "1" {
            assert!(
                !std::path::Path::new(value).exists(),
                "normal binary ignored test-only control {name}"
            );
        }
    }
}

#[test]
fn sigterm_interrupts_cleans_up_and_emits_one_row() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let (mut command, _owner) = helper_command(
        "stall",
        &["--task", "signup", "--agent", "stall"],
        &[(MARKER_ENV, marker.to_str().unwrap())],
    );
    let child = command
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
    let _trial_scope = trial_scope();
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
    let _trial_scope = common::trial_scope();
    let row = parked_after_provisioning("chaos_release", "20000");
    assert_eq!(row.agent_exit_status, Some(0), "the agent's real exit");
}

#[test]
fn a_delivery_parked_after_provisioning_outranks_the_deadline() {
    let _trial_scope = common::trial_scope();
    let row = parked_after_provisioning("chaos_release_stall", "8000");
    assert_eq!(row.agent_exits[0].signal, Some(libc::SIGTERM));
}

#[test]
fn the_catalogue_is_served_as_schema_version_two() {
    let out = cli().arg("tasks").output().unwrap();
    assert!(out.status.success());
    let cat: swarmail_evals::tasks::Catalogue = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(cat.schema_version, 2);
    assert_eq!(cat.tasks, swarmail_evals::tasks::catalogue());
}

#[test]
fn cleanup_escalates_after_a_leader_exits_and_a_descendant_ignores_term() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("pids.json");
    let (mut command, _owner) = helper_command(
        "resistant",
        &["--task", "signup", "--agent", "resistant"],
        &[(MARKER_ENV, marker.to_str().unwrap())],
    );
    let child = command
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

#[test]
fn loopback_clients_ignore_caller_proxy_settings() {
    let _trial_scope = trial_scope();
    let out = cli()
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .env("http_proxy", "http://127.0.0.1:9")
        .env("https_proxy", "http://127.0.0.1:9")
        .env("all_proxy", "http://127.0.0.1:9")
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert_eq!(row.agent_exit_status, Some(0));
}

#[test]
fn v2_check_refuses_v1_state_with_a_safe_version_diagnostic() {
    let _trial_scope = common::trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let (row, _) = trial(&[
        "--task",
        "signup",
        "--agent",
        "oracle",
        "--artifacts-dir",
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(row.schema_version, 2);
    assert_eq!(row.outcome, Outcome::Pass);
    let mut state = retained_state(dir.path(), &row);
    assert_eq!(state["schema_version"], 2);
    state["schema_version"] = serde_json::json!(1);
    let path = dir.path().join("old.json");
    std::fs::write(&path, state.to_string()).unwrap();
    let out = cli()
        .args(["check", "--task", "signup", "--state"])
        .arg(path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("unsupported snapshot schema version; expected 2"));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["outcome"], "harness_error");
}

#[test]
fn trap_bearing_trials_preserve_deadline_and_delivery_precedence() {
    let _trial_scope = common::trial_scope();
    for (mode, class, receipt) in [
        ("naive_deadline", FailureClass::Budget, "delivered"),
        ("naive_parked", FailureClass::HarnessError, "parked"),
        ("naive_failed", FailureClass::HarnessError, "failed"),
        (
            "naive_parked_deadline",
            FailureClass::HarnessError,
            "parked",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let artifacts = dir.path().join("artifacts");
        let (row, out) = helper_trial(
            mode,
            &[
                "--task",
                "stale-otp",
                "--agent",
                mode,
                "--deadline-ms",
                "8000",
                "--artifacts-dir",
                artifacts.to_str().unwrap(),
            ],
            &[],
        );
        assert_eq!(row.schema_version, 2);
        assert_eq!(
            row.failure_class,
            Some(class),
            "{mode}: {row:?} {}",
            stderr(&out)
        );
        assert_eq!(
            row.outcome,
            if class == FailureClass::Budget {
                Outcome::Fail
            } else {
                Outcome::HarnessError
            }
        );
        if mode.contains("deadline") {
            assert_eq!(row.agent_exits[0].signal, Some(libc::SIGTERM));
        } else {
            assert_eq!(row.agent_exit_status, Some(0));
        }
        let state = retained_state(&artifacts, &row);
        assert_eq!(state["identities"][0]["facts"]["otp_verified"], true);
        assert_eq!(state["identities"][0]["facts"]["logged_in"], true);
        assert!(
            state["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["action"] == "superseded_code"
                    && e["accepted"] == false
                    && e["category"] == "stale_code")
        );
        assert_eq!(
            state["deliveries"].as_array().unwrap().last().unwrap()["status"],
            receipt
        );
        let check = cli()
            .args(["check", "--task", "stale-otp", "--state"])
            .arg(artifacts.join(&row.trial_id).join("state.json"))
            .output()
            .unwrap();
        let verdict: Value = serde_json::from_slice(&check.stdout).unwrap();
        // Snapshot replay cannot reconstruct a live deadline.
        assert_eq!(
            verdict["failure_class"],
            if class == FailureClass::Budget {
                "stale_code"
            } else {
                "harness_error"
            }
        );
        assert_eq!(
            check.status.code(),
            Some(if class == FailureClass::Budget { 1 } else { 2 })
        );
    }
}

#[test]
fn sequential_trial_concurrency_requests() {
    let _trial_scope = trial_scope();
    for concurrency in ["0", "2"] {
        let (row, out) = trial(&[
            "--task",
            "signup",
            "--agent",
            "null",
            "--concurrency",
            concurrency,
        ]);
        assert_eq!(row.outcome, Outcome::HarnessError);
        assert_eq!(row.wall_time_ms, 0);
        assert!(row.agent_exits.is_empty());
        assert!(stderr(&out).contains("concurrency_unsupported"));
        let req = runner::TrialRequest {
            concurrency: Some(concurrency.parse().unwrap()),
            ..Default::default()
        };
        assert_eq!(
            runner::validate_request(&req).unwrap_err(),
            "concurrency_unsupported"
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let row = runtime.block_on(runner::run_trial(req, None));
        assert_eq!(row.concurrency, 1);
        assert_eq!(row.wall_time_ms, 0);
    }
    for invalid in ["no", "-1", "4294967296"] {
        let out = cli()
            .args(["trial", "--concurrency", invalid])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(stderr(&out).contains("arguments_invalid"));
    }
    let (row, out) = trial(&[
        "--task",
        "signup",
        "--agent",
        "oracle",
        "--concurrency",
        "1",
    ]);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
}

#[test]
fn sequential_rows_record_concurrency() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    for agent in ["oracle", "null", "missing-command"] {
        let args = if agent == "missing-command" {
            vec![
                "--task",
                "signup",
                "--agent",
                agent,
                "--artifacts-dir",
                dir.path().to_str().unwrap(),
                "--",
                "/nonexistent/agent",
            ]
        } else {
            vec![
                "--task",
                "signup",
                "--agent",
                agent,
                "--artifacts-dir",
                dir.path().to_str().unwrap(),
            ]
        };
        let (row, _) = trial(&args);
        assert_eq!(row.schema_version, 2);
        assert_eq!(row.concurrency, 1);
        let retained: TrialResult = serde_json::from_slice(
            &std::fs::read(dir.path().join(&row.trial_id).join("result.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(retained, row);
    }
}

#[test]
fn sequential_admission_refuses_independent_cli_trials() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    for signal in [libc::SIGTERM, libc::SIGINT] {
        let ready = dir.path().join(format!("ready-{signal}"));
        let (mut command, _wrapper) = helper_command(
            "held_idle",
            &["--task", "signup", "--agent", "idle"],
            &[(MARKER_ENV, ready.to_str().unwrap())],
        );
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let victim = ChildGuard {
            child: Some(command.spawn().unwrap()),
        };
        eventually("live trial", || ready.exists());
        let artifacts = dir.path().join(format!("unused-artifacts-{signal}"));
        let launch = dir.path().join(format!("unused-agent-{signal}"));
        let out = cli()
            .current_dir(dir.path())
            .env("HOME", dir.path().join("other-home"))
            .env("TMPDIR", dir.path().join("other-tmp"))
            .args([
                "trial",
                "--task",
                "signup",
                "--agent",
                "contender",
                "--artifacts-dir",
                artifacts.to_str().unwrap(),
                "--",
                "/usr/bin/touch",
                launch.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        let row = one_row(&out);
        assert_eq!(row.wall_time_ms, 0);
        assert!(row.agent_exits.is_empty());
        assert!(stderr(&out).contains("concurrent_trials_refused"));
        let retained = swarmail_evals::artifacts::trial(&artifacts.join(&row.trial_id)).unwrap();
        assert_eq!(retained.row, row);
        assert!(!launch.exists());
        send_signal(victim.pid(), signal);
        let out = victim.wait_with_output();
        assert_eq!(one_row(&out).outcome, Outcome::HarnessError);
        let (row, out) = trial(&["--task", "signup", "--agent", "oracle"]);
        assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    }
}
