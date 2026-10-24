//! Shared proof helpers: the real CLI, the Swarmail binary resolved once
//! per test binary, and a test-only external agent (the test executable
//! re-run with `EVAL_TEST_HELPER` set) for the wrong-action, stall and
//! echo controls. The helper is an external command like any other: it
//! reaches the trial through the runner's spawn/env/deadline boundary.
#![allow(dead_code)]

use serde_json::{Value, json};
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
        .env_remove(HELPER_ENV)
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
pub fn helper_trial(mode: &str, args: &[&str], env: &[(&str, &str)]) -> (TrialResult, Output) {
    let me = std::env::current_exe().unwrap();
    let mut cmd = cli();
    cmd.env(HELPER_ENV, mode).arg("trial").args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.args([
        "--",
        me.to_str().unwrap(),
        "--exact",
        "helper_agent",
        "--nocapture",
        "--test-threads=1",
    ]);
    let out = cmd.output().expect("the CLI runs");
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

/// Whether `pid` names a live (not zombie) process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: i32,
    pub parent: i32,
    pub pgrp: i32,
    pub session: i32,
    pub start_time: u64,
    pub state: char,
}

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

pub fn identity_alive(identity: &ProcessIdentity) -> bool {
    process_identity(identity.pid)
        .is_some_and(|now| now.start_time == identity.start_time && !matches!(now.state, 'Z' | 'X'))
}

pub fn pid_alive(pid: i32) -> bool {
    process_identity(pid).is_some_and(|process| !matches!(process.state, 'Z' | 'X'))
}

/// Read the Linux process table and return every process below `root`, using
/// PID/start-time identities so the exit proof is safe against PID reuse.
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
        "echo" => {
            let prompt = std::fs::read_to_string(&env.prompt_file).unwrap();
            let vars: Vec<(String, String)> = std::env::vars()
                .filter(|(k, _)| k.starts_with("EVAL_"))
                .collect();
            let report = json!({
                "argv": std::env::args().collect::<Vec<_>>(),
                "cwd": std::env::current_dir().unwrap(),
                "prompt": prompt,
                "env": vars,
                "goal": env.goal,
            });
            std::fs::write(marker().unwrap(), report.to_string()).unwrap();
            3
        }
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
        "stall" => {
            if let Ok(path) = std::env::var(LAUNCH_LOG_ENV) {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .unwrap();
                writeln!(file, "{}", std::process::id()).unwrap();
            }
            // Deliberately never waited on: a descendant that outlives its
            // leader is what the runner's group sweep must reap.
            #[allow(clippy::zombie_processes)]
            let child = std::process::Command::new("sleep")
                .arg("1000")
                .spawn()
                .unwrap();
            let leader = process_identity(std::process::id() as i32).unwrap();
            let descendant = process_identity(child.id() as i32).unwrap();
            let pids = json!({
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
