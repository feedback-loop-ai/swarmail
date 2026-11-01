//! Shared proof helpers: the real CLI, the Swarmail binary resolved once
//! per test binary, and test-owned wrapper commands for the wrong-action,
//! stall and echo controls. Wrappers record their initial environment before
//! adding helper-only configuration, then use the ordinary agent spawn path.
#![allow(dead_code)]

pub mod coordination;
pub fn trial_scope() -> std::fs::File {
    coordination::trial_scope()
}

use serde_json::{Value, json};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use swarmail_evals::mcp::McpClient;
use swarmail_evals::oracle::{self, AgentEnv, App, labelled_url};
use swarmail_evals::runner::{self, TrialResult};

pub const HELPER_ENV: &str = "EVAL_TEST_HELPER";
pub const MARKER_ENV: &str = "EVAL_TEST_MARKER";
pub const BARRIER_DIR_ENV: &str = "EVAL_TEST_BARRIER_DIR";
pub const BARRIER_COUNT_ENV: &str = "EVAL_TEST_BARRIER_COUNT";
pub const LAUNCH_LOG_ENV: &str = "EVAL_TEST_LAUNCH_LOG";

/// A fresh temporary directory under the real path of the system temporary
/// directory. Tests name real paths directly so they do not depend on the
/// symlinked-ancestor rule they are not about (artifact roots admit only
/// root-owned links; private paths still refuse every link), and macOS's
/// per-user temporary directory lives under /var, a symlink to /private/var;
/// on Linux the name is unchanged.
pub fn tempdir() -> tempfile::TempDir {
    let base = std::fs::canonicalize(std::env::temp_dir()).expect("canonical temporary directory");
    tempfile::tempdir_in(base).expect("temporary directory")
}

/// A program that exits 0 at once. macOS has no /bin/true.
#[cfg(target_os = "macos")]
pub const TRUE_PROGRAM: &str = "/usr/bin/true";
/// A program that exits 0 at once.
#[cfg(not(target_os = "macos"))]
pub const TRUE_PROGRAM: &str = "/bin/true";

/// A program that exits 1 at once. macOS has no /bin/false.
#[cfg(target_os = "macos")]
pub const FALSE_PROGRAM: &str = "/usr/bin/false";
/// A program that exits 1 at once.
#[cfg(not(target_os = "macos"))]
pub const FALSE_PROGRAM: &str = "/bin/false";

pub fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_swarmail-evals")
}

/// The Swarmail under test: `SWARMAIL_BIN` if set, else built once from
/// the root checkout (offline, locked).
pub fn swarmail_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| runner::resolve_swarmail_bin().expect("the swarmail binary resolves"))
}

/// The CLI with SWARMAIL_BIN pinned (no per-trial build).
pub struct ChildGuard {
    pub child: Option<std::process::Child>,
}

impl ChildGuard {
    pub fn pid(&self) -> i32 {
        self.child.as_ref().unwrap().id() as i32
    }

    pub fn wait_with_output(mut self) -> Output {
        self.child.take().unwrap().wait_with_output().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        send_signal(child.id() as i32, libc::SIGTERM);
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if child.try_wait().ok().flatten().is_none() {
            // SAFETY: this guard owns the child process it spawned.
            unsafe {
                libc::kill(child.id() as i32, libc::SIGKILL);
            }
            let _ = child.wait();
        }
    }
}

pub fn cli() -> Command {
    let mut cmd = Command::new(exe());
    cmd.env(runner::SWARMAIL_BIN_ENV, swarmail_bin())
        .env(
            OsString::from_vec(b"NONUTF8_PARENT_\xff".to_vec()),
            OsString::from_vec(b"synthetic_\xfe_value".to_vec()),
        )
        .stdin(Stdio::null());
    cmd
}

/// Exactly one stdout line, parsed and schema-valid.
pub fn one_row(out: &Output) -> TrialResult {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one stdout line; got {stdout:?}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let row: TrialResult = serde_json::from_str(lines[0]).expect("a result row");
    row.validate().expect("the row obeys its schema");
    assert_eq!(out.status.code(), Some(row.exit_code()), "{row:?}");
    row
}

pub fn trial(args: &[&str]) -> (TrialResult, Output) {
    let out = cli()
        .arg("trial")
        .args(args)
        .output()
        .expect("the CLI runs");
    (one_row(&out), out)
}

/// A trial whose agent is the helper in `mode`.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn helper_command(
    mode: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> (Command, tempfile::TempDir) {
    let dir = tempdir();
    let marker = env
        .iter()
        .find(|(key, _)| *key == MARKER_ENV)
        .map(|(_, value)| (*value).to_string())
        .unwrap_or_else(|| dir.path().join("helper-marker").display().to_string());
    let env_snapshot = format!("{marker}.launch-env");
    let barrier_dir = env
        .iter()
        .find(|(key, _)| *key == BARRIER_DIR_ENV)
        .map(|(_, value)| (*value).to_string());
    let barrier_count = env
        .iter()
        .find(|(key, _)| *key == BARRIER_COUNT_ENV)
        .map(|(_, value)| (*value).to_string());
    let me = std::env::current_exe().unwrap();
    // Linux reads the exact exec-time map from /proc. Elsewhere `env` runs
    // inside this shell, which exports its own SHLVL and `_`; exactly those
    // are dropped there, and the Linux run proves the harness never sets them.
    let mut script = format!(
        "#!/bin/sh\nif [ -r /proc/$$/environ ]; then /bin/cat /proc/$$/environ > {}; else unset SHLVL; /usr/bin/env -u _ -0 > {}; fi\nexport {}={}\nexport {}={}\nexport EVAL_TEST_ENV_SNAPSHOT={}\n",
        shell_quote(&env_snapshot),
        shell_quote(&env_snapshot),
        HELPER_ENV,
        shell_quote(mode),
        MARKER_ENV,
        shell_quote(&marker),
        shell_quote(&env_snapshot),
    );
    script.push_str("export EVAL_TEST_RUNNER_PID=$PPID\n");
    if let Some(value) = barrier_dir {
        script.push_str(&format!(
            "export {BARRIER_DIR_ENV}={}\n",
            shell_quote(&value)
        ));
    }
    if let Some(value) = barrier_count {
        script.push_str(&format!(
            "export {BARRIER_COUNT_ENV}={}\n",
            shell_quote(&value)
        ));
    }
    for (key, value) in env.iter().filter(|(key, _)| key.starts_with("EVAL_TEST_")) {
        script.push_str(&format!("export {key}={}\n", shell_quote(value)));
    }
    script.push_str("if [ -n \"${EVAL_TEST_ARG_MARKER-}\" ]; then printf '%s\\0' \"$@\" > \"$EVAL_TEST_ARG_MARKER\"; fi\n");
    script.push_str(&format!(
        "exec {} --exact helper_agent --nocapture --test-threads=1\n",
        shell_quote(me.to_str().unwrap())
    ));
    let wrapper = dir.path().join("agent-wrapper.sh");
    std::fs::write(&wrapper, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();

    let mut command = cli();
    command.arg("trial").args(args).arg("--").arg(&wrapper);
    for (key, value) in env {
        if ![HELPER_ENV, MARKER_ENV, BARRIER_DIR_ENV, BARRIER_COUNT_ENV].contains(key)
            && !key.starts_with("EVAL_TEST_")
        {
            command.env(key, value);
        }
    }
    (command, dir)
}

pub fn helper_trial(mode: &str, args: &[&str], env: &[(&str, &str)]) -> (TrialResult, Output) {
    helper_trial_with_agent_args(mode, args, env, &[])
}

pub fn helper_trial_with_agent_args(
    mode: &str,
    args: &[&str],
    env: &[(&str, &str)],
    agent_args: &[&str],
) -> (TrialResult, Output) {
    let (mut command, _owner) = helper_command(mode, args, env);
    command.args(agent_args);
    let out = command.output().expect("the CLI runs");
    (one_row(&out), out)
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Poll a condition with a real deadline (never a sleep the test trusts).
pub fn eventually(what: &str, mut ok: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ok() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

// The process-identity proofs read /proc/<pid>/stat, which only Linux has;
// they are compiled only there so no test can pass vacuously elsewhere.
/// Whether `pid` names a live (not zombie) process.
#[cfg(target_os = "linux")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: i32,
    pub parent: i32,
    pub pgrp: i32,
    pub session: i32,
    pub start_time: u64,
    pub state: char,
}

#[cfg(target_os = "linux")]
pub fn process_identity(pid: i32) -> Option<ProcessIdentity> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, tail) = stat.rsplit_once(')')?;
    let fields: Vec<_> = tail.split_whitespace().collect();
    Some(ProcessIdentity {
        pid,
        state: fields.first()?.chars().next()?,
        parent: fields.get(1)?.parse().ok()?,
        pgrp: fields.get(2)?.parse().ok()?,
        session: fields.get(3)?.parse().ok()?,
        start_time: fields.get(19)?.parse().ok()?,
    })
}

#[cfg(target_os = "linux")]
pub fn identity_alive(identity: &ProcessIdentity) -> bool {
    process_identity(identity.pid)
        .is_some_and(|now| now.start_time == identity.start_time && !matches!(now.state, 'Z' | 'X'))
}

#[cfg(target_os = "linux")]
pub fn pid_alive(pid: i32) -> bool {
    process_identity(pid).is_some_and(|process| !matches!(process.state, 'Z' | 'X'))
}

/// Read the Linux process table and return every process below `root`, using
/// PID/start-time identities so the exit proof is safe against PID reuse.
#[cfg(target_os = "linux")]
pub fn process_descendants(root: i32) -> Vec<ProcessIdentity> {
    let mut all = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return all;
    };
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid) = name.parse::<i32>() else {
            continue;
        };
        if let Some(identity) = process_identity(pid) {
            all.push(identity);
        }
    }
    let mut found = Vec::new();
    let mut parents = std::collections::HashSet::from([root]);
    loop {
        let children: Vec<_> = all
            .iter()
            .filter(|p| parents.contains(&p.parent))
            .cloned()
            .collect();
        let added = children.iter().filter(|p| parents.insert(p.pid)).count();
        if added == 0 {
            break;
        }
        found.extend(children);
    }
    found
}

pub fn send_signal(pid: i32, sig: i32) {
    // SAFETY: plain kill(2).
    unsafe {
        libc::kill(pid, sig);
    }
}

/// Run the helper agent when this test binary was launched as one.
/// Returns without effect in a normal test run.
pub fn maybe_run_helper() {
    let Ok(mode) = std::env::var(HELPER_ENV) else {
        return;
    };
    let code = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(helper(&mode));
    std::process::exit(code);
}

fn marker() -> Option<PathBuf> {
    std::env::var_os(MARKER_ENV).map(PathBuf::from)
}

async fn first_link(mcp: &mut McpClient, inbox: &str, subject: &str, label: &str) -> String {
    let mail = mcp.wait_emails(inbox, Some(subject), 1).await.unwrap();
    labelled_url(mail[0]["text"].as_str().unwrap(), label).unwrap()
}

async fn helper(mode: &str) -> i32 {
    let env = AgentEnv::from_env().expect("the agent environment");
    let app = App::new(&env.app_url, &env.identity).unwrap();
    match mode {
        mode if mode.starts_with("naive_") => naive_recovery(mode, &env, &app).await,
        "held_idle" => {
            let marker = marker().unwrap();
            std::fs::write(&marker, b"ready").unwrap();
            let deadline = Instant::now() + Duration::from_secs(30);
            while !marker.with_extension("release").exists() {
                assert!(Instant::now() < deadline, "idle release deadline");
                tokio::task::yield_now().await;
            }
            0
        }
        "echo" => {
            let home = std::env::var("HOME").unwrap();
            let sentinel = Path::new(&home).join("sequential-home-sentinel");
            let sentinel_was_present = sentinel.exists();
            std::fs::write(&sentinel, b"written by real agent").unwrap();
            let prompt = std::fs::read_to_string(&env.prompt_file).unwrap();
            let vars: Vec<(String, String)> = std::env::vars()
                .filter(|(k, _)| {
                    [
                        "EVAL_TASK_ID",
                        "EVAL_APP_URL",
                        "EVAL_MCP_URL",
                        "EVAL_IDENTITY",
                        "EVAL_INBOX",
                        "EVAL_GOAL",
                        "EVAL_PROMPT_FILE",
                        "EVAL_DEADLINE_MS",
                    ]
                    .contains(&k.as_str())
                })
                .collect();
            let report = json!({
                "home": home,
                "home_sentinel_was_present": sentinel_was_present,
                "home_sentinel_written": sentinel.is_file(),
                "argv": std::env::args().collect::<Vec<_>>(),
                "cwd": std::env::current_dir().unwrap(),
                "prompt": prompt,
                "env": vars,
                "goal": env.goal,
            });
            std::fs::write(marker().unwrap(), report.to_string()).unwrap();
            3
        }
        // The process-identity helper modes report /proc identities.
        #[cfg(target_os = "linux")]
        "resistant" => {
            // The agent leader exits on TERM; this same-group descendant
            // ignores TERM so the owning cleanup must escalate to KILL.
            #[allow(clippy::zombie_processes)]
            let child = std::process::Command::new("/bin/sh")
                .args(["-c", "trap '' TERM; exec sleep 1000"])
                .spawn()
                .unwrap();
            let leader = process_identity(std::process::id() as i32).unwrap();
            let descendant = process_identity(child.id() as i32).unwrap();
            let path = marker().unwrap();
            let pids = json!({
                "leader": leader.pid, "leader_start": leader.start_time,
                "leader_pgrp": leader.pgrp, "leader_session": leader.session,
                "descendant": descendant.pid, "descendant_start": descendant.start_time,
                "descendant_pgrp": descendant.pgrp, "descendant_session": descendant.session,
            });
            std::fs::write(path.with_extension("tmp"), pids.to_string()).unwrap();
            std::fs::rename(path.with_extension("tmp"), &path).unwrap();
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }
        #[cfg(target_os = "linux")]
        "stall" | "noisy_stall" => {
            if let Ok(path) = std::env::var(LAUNCH_LOG_ENV) {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .unwrap();
                writeln!(file, "{}", std::process::id()).unwrap();
            }
            let path = marker().unwrap();
            let mut child_cmd = std::process::Command::new("/bin/sh");
            if mode == "noisy_stall" {
                use std::io::Write;
                let chunk = vec![b'x'; 96 * 1024];
                std::io::stdout().write_all(&chunk).unwrap();
                std::io::stderr().write_all(&chunk).unwrap();
                std::io::stdout()
                    .write_all(b"synthetic-cancellation-output-canary")
                    .unwrap();
                std::io::stderr()
                    .write_all(b"synthetic-cancellation-output-canary")
                    .unwrap();
                let script = format!(
                    "trap 'touch \"{}\"; while [ ! -e \"{}\" ]; do :; done; exit 0' TERM; touch \"{}\"; while :; do :; done",
                    path.with_extension("cleanup-entered").display(),
                    path.with_extension("release").display(),
                    path.with_extension("descendant-ready").display()
                );
                child_cmd.args(["-c", &script]);
            } else {
                child_cmd.args(["-c", "exec sleep 1000"]);
            }
            // An intentionally unwaited same-group descendant exercises the final fence.
            #[allow(clippy::zombie_processes)]
            let child = child_cmd.spawn().unwrap();
            if mode == "noisy_stall" {
                let deadline = Instant::now() + Duration::from_secs(10);
                while !path.with_extension("descendant-ready").exists() {
                    assert!(Instant::now() < deadline);
                    tokio::task::yield_now().await;
                }
            }
            let leader = process_identity(std::process::id() as i32).unwrap();
            let descendant = process_identity(child.id() as i32).unwrap();
            let pids = json!({
                "runtime_paths": (["HOME", "TMPDIR", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME"].map(|key| std::env::var(key).unwrap())),
                "stdout_bytes": if mode == "noisy_stall" {96 * 1024 + b"synthetic-cancellation-output-canary".len()} else {0},
                "stderr_bytes": if mode == "noisy_stall" {96 * 1024 + b"synthetic-cancellation-output-canary".len()} else {0},
                "leader": leader.pid, "leader_start": leader.start_time,
                "leader_pgrp": leader.pgrp, "leader_session": leader.session,
                "descendant": descendant.pid, "descendant_start": descendant.start_time,
                "descendant_pgrp": descendant.pgrp, "descendant_session": descendant.session,
            });
            let path = marker().unwrap();
            std::fs::write(path.with_extension("tmp"), pids.to_string()).unwrap();
            std::fs::rename(path.with_extension("tmp"), &path).unwrap();
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }
        "fixture_env_probe" => {
            #[cfg(target_os = "linux")]
            {
                let runner_pid = std::env::var("EVAL_TEST_RUNNER_PID").unwrap();
                let marker = std::env::var_os("EVAL_TEST_FIXTURE_ENV_MARKER").unwrap();
                let mut captured = false;
                for entry in std::fs::read_dir("/proc").unwrap() {
                    let entry = entry.unwrap();
                    let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                        continue;
                    };
                    if pid == std::process::id() {
                        continue;
                    }
                    let process = entry.path();
                    let Ok(status) = std::fs::read_to_string(process.join("status")) else {
                        continue;
                    };
                    if !status.lines().any(|line| {
                        line.strip_prefix("PPid:")
                            .and_then(|value| value.trim().parse::<u32>().ok())
                            .is_some_and(|parent| parent.to_string() == runner_pid)
                    }) {
                        continue;
                    }
                    let Ok(command) = std::fs::read(process.join("cmdline")) else {
                        continue;
                    };
                    if !command
                        .split(|byte| *byte == 0)
                        .any(|arg| arg == b"fixture")
                    {
                        continue;
                    }
                    let environment = std::fs::read(process.join("environ")).unwrap();
                    std::fs::write(marker, environment).unwrap();
                    captured = true;
                    break;
                }
                if !captured {
                    return 64;
                }
                if oracle::run_oracle(&env).await.is_ok() {
                    0
                } else {
                    1
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                64
            }
        }
        "file_config" => {
            let config =
                std::fs::read_to_string(std::env::var_os("EVAL_TEST_AUTH_CONFIG").unwrap())
                    .unwrap();
            if config != "credential=synthetic-wrapper-secret-196\n" {
                return 64;
            }
            std::fs::write(marker().unwrap(), b"config-read").unwrap();
            if oracle::run_oracle(&env).await.is_ok() {
                0
            } else {
                1
            }
        }
        "unsafe_output" | "unsafe_output_then_server_death" => {
            let canary = std::fs::read(std::env::var_os("EVAL_TEST_CANARY_FILE").unwrap()).unwrap();
            let mut emitted = 0;
            for chunk in canary.chunks(3) {
                std::io::Write::write_all(&mut std::io::stdout(), chunk).unwrap();
                std::io::Write::write_all(&mut std::io::stderr(), chunk).unwrap();
                emitted += chunk.len();
            }
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            std::io::Write::flush(&mut std::io::stderr()).unwrap();
            std::fs::write(
                marker().unwrap(),
                format!("emitted_bytes_per_stream={emitted}"),
            )
            .unwrap();
            if mode == "unsafe_output_then_server_death" {
                #[cfg(target_os = "linux")]
                {
                    let runner_pid = std::env::var("EVAL_TEST_RUNNER_PID").unwrap();
                    let mut killed = false;
                    for entry in std::fs::read_dir("/proc").unwrap() {
                        let entry = entry.unwrap();
                        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                            continue;
                        };
                        if pid == std::process::id() {
                            continue;
                        }
                        let process = entry.path();
                        let Ok(status) = std::fs::read_to_string(process.join("status")) else {
                            continue;
                        };
                        if !status.lines().any(|line| {
                            line.strip_prefix("PPid:")
                                .and_then(|value| value.trim().parse::<u32>().ok())
                                .is_some_and(|parent| parent.to_string() == runner_pid)
                        }) {
                            continue;
                        }
                        let Ok(command) = std::fs::read(process.join("cmdline")) else {
                            continue;
                        };
                        if command.split(|byte| *byte == 0).any(|arg| arg == b"serve") {
                            // SAFETY: this test helper targets the runner's own Swarmail child.
                            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
                            killed = true;
                            break;
                        }
                    }
                    if !killed {
                        return 64;
                    }
                }
                #[cfg(not(target_os = "linux"))]
                return 64;
            }
            0
        }
        "noisy" => {
            let chunk = "x".repeat(64 * 1024);
            for _ in 0..80 {
                println!("{chunk}");
                eprintln!("{chunk}");
            }
            if oracle::run_oracle(&env).await.is_ok() {
                0
            } else {
                1
            }
        }
        "succeed_then_crash" => {
            oracle::run_oracle(&env).await.unwrap();
            7
        }
        "barrier_oracle" => {
            let dir = PathBuf::from(std::env::var(BARRIER_DIR_ENV).unwrap());
            let count: usize = std::env::var(BARRIER_COUNT_ENV).unwrap().parse().unwrap();
            std::fs::write(dir.join(&env.identity), b"started").unwrap();
            // Every agent of both trials must be live before any proceeds.
            let deadline = Instant::now() + Duration::from_secs(30);
            while std::fs::read_dir(&dir).unwrap().count() < count {
                if Instant::now() > deadline {
                    eprintln!("barrier never filled");
                    return 9;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            match oracle::run_oracle(&env).await {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        "wrong_message" => {
            // Follow the look-alike sibling's confirmation instead of ours.
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            let mails = mcp
                .wait_emails(&env.inbox, Some("Confirm your account"), 2)
                .await
                .unwrap();
            let theirs = mails
                .iter()
                .find(|m| m["to"][0]["address"] != json!(env.identity))
                .unwrap();
            let link =
                labelled_url(theirs["text"].as_str().unwrap(), "Confirm your account:").unwrap();
            app.get(&link).await.unwrap();
            0
        }
        "near_miss_sibling" | "near_miss_sibling_nearest" => {
            // A mistyped subject times the wait out; the timed-out answer
            // alone (decision 0111) names the mail it missed. The recovery
            // control picks the entry addressed to the exact identity; the
            // negative control presents `nearest_subject` as the match.
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            let missed = mcp
                .wait(&env.inbox, Some("Confirm yur account"), 1, 1000)
                .await
                .unwrap();
            assert_eq!(missed["timed_out"], json!(true), "{missed}");
            assert_eq!(missed["matched"], json!(0), "{missed}");
            let near = &missed["near_misses"];
            assert_eq!(near["arrivals_total"], json!(2), "{missed}");
            let chosen = if mode == "near_miss_sibling" {
                near["arrivals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|entry| entry["to"] == json!([env.identity]))
                    .unwrap()
            } else {
                &near["nearest_subject"]
            };
            let mail = mcp
                .call("swarmail_get_email", json!({ "id": chosen["email_id"] }))
                .await
                .unwrap();
            let link =
                labelled_url(mail["text"].as_str().unwrap(), "Confirm your account:").unwrap();
            app.get(&link).await.unwrap();
            0
        }
        "near_miss_slow" => {
            // Nothing has arrived: the timed-out answer says so by carrying
            // no near misses at all. Release the held mail, wait, confirm.
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            let first = mcp
                .wait(&env.inbox, Some("Confirm your account"), 1, 1000)
                .await
                .unwrap();
            assert_eq!(first["timed_out"], json!(true), "{first}");
            assert_eq!(first["total"], json!(0), "{first}");
            assert!(first.get("near_misses").is_none(), "{first}");
            let (status, body) = app.post("/deliveries/release", json!({})).await.unwrap();
            assert_eq!(status, 200, "{body}");
            let mail = mcp
                .wait_emails(&env.inbox, Some("Confirm your account"), 1)
                .await
                .unwrap();
            let link =
                labelled_url(mail[0]["text"].as_str().unwrap(), "Confirm your account:").unwrap();
            assert_eq!(app.get(&link).await.unwrap().0, 200);
            0
        }
        "stale_code" => {
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            let mails = mcp
                .wait_emails(&env.inbox, Some("Your verification code"), 2)
                .await
                .unwrap();
            let old = mails
                .iter()
                .find(|m| {
                    !m["text"]
                        .as_str()
                        .unwrap()
                        .contains("new verification code")
                })
                .unwrap();
            let code = old["codes"][0].as_str().unwrap();
            app.post("/otp/verify", json!({ "code": code }))
                .await
                .unwrap();
            0
        }
        "wrong_link" => {
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            let link = first_link(
                &mut mcp,
                &env.inbox,
                "Confirm your account",
                "Unsubscribe from these emails:",
            )
            .await;
            app.get(&link).await.unwrap();
            0
        }
        "private_claims"
        | "private_claims_then_oracle"
        | "private_claims_budget"
        | "private_claims_retention_failure" => {
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            let mails = mcp
                .wait_emails(&env.inbox, Some("Your verification code"), 1)
                .await
                .unwrap();
            let otp = mails[0]["codes"][0].as_str().unwrap();
            let private = "private-external-claim-canary";
            for claim in [otp, private] {
                let response = reqwest::Client::new()
                    .post(format!("{}/otp/verify", env.app_url))
                    .header("x-eval-identity", claim)
                    .header("content-type", "application/json")
                    .body(json!({"code":otp}).to_string())
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), 400);
                let body = response.text().await.unwrap();
                assert!(
                    !body.contains(otp) && !body.contains(private),
                    "response excludes private claims"
                );
            }
            // A private test control outside the published evidence tree; never
            // echo its values in assertions, stdout or the implementation receipt.
            let dir = marker().unwrap();
            std::fs::write(dir.join(&env.identity), json!([otp, private]).to_string()).unwrap();
            if mode == "private_claims_then_oracle" {
                oracle::run_oracle(&env).await.unwrap();
            }
            if mode == "private_claims_budget" {
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            }
            if mode == "private_claims_retention_failure" {
                std::fs::write(
                    std::env::var_os("EVAL_TEST_RETENTION_PATH").unwrap(),
                    b"occupied",
                )
                .unwrap();
            }
            0
        }
        "tool_misuse" => {
            app.post(
                "/reset/complete",
                json!({ "token": "not-a-token", "password": "pw" }),
            )
            .await
            .unwrap();
            0
        }
        "foreign" => {
            // Ada reads Grace's inbox and presents Grace's link as herself;
            // everyone then confirms their own account.
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            if env.identity.starts_with("ada.") {
                let grace_inbox = env.inbox.replace("-ada", "-grace");
                let link = first_link(
                    &mut mcp,
                    &grace_inbox,
                    "Confirm your account",
                    "Confirm your account:",
                )
                .await;
                let (status, _) = app.get(&link).await.unwrap();
                assert_eq!(status, 403);
            }
            if oracle::run_oracle(&env).await.is_ok() {
                0
            } else {
                1
            }
        }
        "chaos_release" | "chaos_release_stall" => {
            // After provisioning, arm DATA chaos at 100, release the held
            // mail (the reply comes only once its attempt is recorded), then
            // clear chaos: the release leaves a parked 451 receipt.
            let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
            mcp.call(
                "swarmail_set_chaos",
                json!({"event": "data", "probability": 100}),
            )
            .await
            .unwrap();
            let (status, body) = app.post("/deliveries/release", json!({})).await.unwrap();
            mcp.call("swarmail_clear_chaos", json!({})).await.unwrap();
            assert_eq!((status, &body["released"]), (200, &json!(1)), "{body}");
            std::fs::write(marker().unwrap(), b"released").unwrap();
            if mode == "chaos_release" {
                return 0;
            }
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }
        other => {
            eprintln!("unknown helper mode {other}");
            64
        }
    }
}

/// Read a retained snapshot from an artifacts directory.
pub fn retained_state(dir: &Path, row: &TrialResult) -> Value {
    let path = dir.join(&row.trial_id).join("state.json");
    serde_json::from_slice(&std::fs::read(&path).expect("state.json retained")).unwrap()
}

/// Independent public strategies: deliberately try alternatives, then recover.
/// No oracle, admin snapshot, private tokens or scorer input is consulted.
async fn naive_recovery(mode: &str, env: &AgentEnv, app: &App) -> i32 {
    let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
    let want = if matches!(
        env.task.as_str(),
        "sibling-addresses" | "i18n-subject" | "stale-otp"
    ) {
        2
    } else {
        1
    };
    let mails = mcp.wait_emails(&env.inbox, None, want).await.unwrap();
    let follow = |mail: &Value| {
        labelled_url(mail["text"].as_str().unwrap(), "Confirm your account:").unwrap()
    };
    match env.task.as_str() {
        "sibling-addresses" => {
            let theirs = mails
                .iter()
                .find(|m| m["to"][0]["address"] != env.identity)
                .unwrap();
            let mine = mails
                .iter()
                .find(|m| m["to"][0]["address"] == env.identity)
                .unwrap();
            let claimant = if mode == "naive_sibling_decoy" {
                App::new(&env.app_url, theirs["to"][0]["address"].as_str().unwrap()).unwrap()
            } else {
                App::new(&env.app_url, &env.identity).unwrap()
            };
            assert_eq!(claimant.get(&follow(theirs)).await.unwrap().0, 403);
            assert_eq!(app.get(&follow(mine)).await.unwrap().0, 200);
        }
        "i18n-subject" => {
            // Subscription first, then the requested account message.
            for subject in [
                swarmail_evals::tasks::I18N_DECOY_SUBJECT,
                swarmail_evals::tasks::I18N_SUBJECT,
            ] {
                let mail = mails.iter().find(|m| m["subject"] == subject).unwrap();
                assert_eq!(app.get(&follow(mail)).await.unwrap().0, 200);
            }
        }
        "stale-otp" => {
            for current in [false, true] {
                let mail = mails
                    .iter()
                    .find(|m| {
                        m["text"]
                            .as_str()
                            .unwrap()
                            .contains("new verification code")
                            == current
                    })
                    .unwrap();
                let code = mail["codes"][0].as_str().unwrap();
                assert_eq!(
                    app.post("/otp/verify", json!({"code": code}))
                        .await
                        .unwrap()
                        .0,
                    if current { 200 } else { 410 }
                );
            }
        }
        "entity-url" if mode == "naive_entity_raw" => {
            let html = mails[0]["html"].as_str().unwrap();
            let raw = html
                .split("href=\"")
                .skip(1)
                .map(|h| h.split('"').next().unwrap())
                .find(|h| h.contains("/m/offer"))
                .unwrap();
            assert_eq!(app.get(raw).await.unwrap().0, 400);
            // Decode the same emailed URL, without consulting a private target.
            let decoded = raw.replace("&#61;", "=").replace("&amp;", "&");
            assert_eq!(app.get(&decoded).await.unwrap().0, 200);
        }
        "confirm-decoys" | "magic-login" | "entity-url" | "digest-forty" => {
            let mut links: Vec<&str> = mails[0]["links"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let distinct: std::collections::HashSet<_> = links.iter().copied().collect();
            assert_eq!(distinct.len(), links.len());
            assert_eq!(
                links.len(),
                match env.task.as_str() {
                    "digest-forty" => 40,
                    "magic-login" => 2,
                    _ => 3,
                }
            );
            if mode == "naive_reverse" {
                links.reverse();
            }
            for link in links {
                assert_eq!(app.get(link).await.unwrap().0, 200, "{link}");
            }
        }
        other => panic!("no naive recovery for {other}"),
    }
    if mode.contains("parked") || mode.contains("failed") {
        let code = if mode.contains("failed") {
            "550 5.3.0 rejected"
        } else {
            "451 4.3.0 retry"
        };
        mcp.call(
            "swarmail_set_chaos",
            json!({"event": "data", "probability": 100, "error": code}),
        )
        .await
        .unwrap();
        assert_eq!(app.post("/otp/resend", json!({})).await.unwrap().0, 200);
        mcp.call("swarmail_clear_chaos", json!({})).await.unwrap();
    }
    if mode.contains("deadline") {
        std::future::pending::<()>().await;
    }
    0
}
