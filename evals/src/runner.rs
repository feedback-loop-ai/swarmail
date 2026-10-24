//! The trial runner (#148's agent-agnostic seam, decision 0055 §§6–9).
//!
//! One trial: resolve the Swarmail binary, start a fresh in-memory
//! Swarmail and a fresh fixture on loopback port zero, provision the
//! identities, run the scenario's barriers, run the agent command(s) with
//! the goal and URLs in their environment and a prompt file, freeze and
//! snapshot the app, stop everything, score the snapshot, and produce
//! exactly one result row.
//!
//! The runner owns every child's Unix process group: it drains their
//! output with bounded retention, signals the whole group on every handled
//! ending (deadline, interruption, panic, server death) and reaps it. That
//! is lifecycle ownership, not confinement: a child that leaves its group
//! or daemonizes escapes it, and every result says `unconfined`.

use crate::fixture::ADMIN_SECRET_ENV;
use crate::mail;
use crate::oracle::{
    ENV_APP_URL, ENV_DEADLINE_MS, ENV_GOAL, ENV_IDENTITY, ENV_INBOX, ENV_MCP_URL, ENV_PROMPT_FILE,
    ENV_TASK,
};
use crate::state::{DeliveryStatus, FailureClass, Snapshot};
use crate::tasks::{self, Outcome, PromptVars, Task, Verdict};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::VecDeque;
#[cfg(test)]
use std::future::Future;
use std::net::SocketAddr;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// The result row's schema.
pub const RESULT_SCHEMA: u32 = 1;
/// Every trial is unconfined until brokkr's MCP broker lands (0055 §7).
pub const CONFINEMENT: &str = "unconfined";
/// The env var naming the Swarmail binary under test.
pub const SWARMAIL_BIN_ENV: &str = "SWARMAIL_BIN";
#[cfg(test)]
const FAULT_ENV: &str = "EVAL_RUNNER_FAULT";
#[cfg(test)]
const CLEANUP_HOLD_ENV: &str = "EVAL_TEST_CLEANUP_HOLD_DIR";
#[cfg(test)]
const CLEANUP_WAIT_PENDING_ENV: &str = "EVAL_TEST_CLEANUP_WAIT_PENDING";
#[cfg(test)]
const CLEANUP_WAIT_BYPASS_ENV: &str = "EVAL_TEST_CLEANUP_WAIT_BYPASS";
#[cfg(test)]
const PRE_HANDLER_HOLD_ENV: &str = "EVAL_TEST_PRE_HANDLER_HOLD_DIR";
#[cfg(test)]
const PIDS_ENV_FOR_TESTS: &str = "EVAL_TEST_RUNNER_PIDS";
#[cfg(test)]
const FAULT_RELEASE_ENV: &str = "EVAL_TEST_RUNNER_FAULT_RELEASE";
#[cfg(test)]
const CLEANUP_BUDGET_HOLD_ENV: &str = "EVAL_TEST_CLEANUP_BUDGET_HOLD_DIR";
#[cfg(test)]
const CLEANUP_BUDGET_RELEASE_ENV: &str = "EVAL_TEST_CLEANUP_BUDGET_RELEASE";
#[cfg(test)]
const CLEANUP_BUDGET_LOG_ENV: &str = "EVAL_TEST_CLEANUP_BUDGET_LOG";
#[cfg(test)]
const CLEANUP_OBSERVE_ERROR_ENV: &str = "EVAL_TEST_CLEANUP_OBSERVE_ERROR";
#[cfg(test)]
const CLEANUP_FORCE_EXPIRY_ENV: &str = "EVAL_TEST_CLEANUP_FORCE_EXPIRY";
#[cfg(test)]
const RUNNER_SIGNAL_LOG_ENV: &str = "EVAL_TEST_RUNNER_SIGNAL_LOG";
#[cfg(all(test, target_os = "linux"))]
static CLEANUP_OBSERVATION_TARGET: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(0);
#[cfg(test)]
static CLEANUP_TEST_EVENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
fn cleanup_test_event(event: &str) {
    use std::io::Write as _;
    let _guard = CLEANUP_TEST_EVENT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(path) = std::env::var_os(CLEANUP_BUDGET_LOG_ENV)
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{event}");
    }
}

#[cfg(test)]
fn runner_signal_event(event: &str) {
    use std::io::Write as _;
    if let Some(path) = std::env::var_os(RUNNER_SIGNAL_LOG_ENV)
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{event}");
    }
}
/// The built-in agent labels.
pub const BUILTINS: [&str; 2] = ["oracle", "null"];

const STARTUP_DEADLINE: Duration = Duration::from_secs(30);
const LOG_RETENTION: usize = 64 * 1024;
const CLEANUP_POLL: Duration = Duration::from_millis(100);

/// One agent process's real exit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExit {
    pub identity: String,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// The one JSON line a trial writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialResult {
    pub schema_version: u32,
    pub trial_id: String,
    pub task: Option<String>,
    pub agent: Option<String>,
    pub outcome: Outcome,
    pub failure_class: Option<FailureClass>,
    pub wall_time_ms: u64,
    pub agent_exit_status: Option<i32>,
    pub agent_exits: Vec<AgentExit>,
    pub confinement: String,
}

impl TrialResult {
    /// Whether the row obeys its own schema's invariants.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != RESULT_SCHEMA {
            return Err(format!(
                "schema_version {} is not {RESULT_SCHEMA}",
                self.schema_version
            ));
        }
        if self.confinement != CONFINEMENT {
            return Err(format!(
                "confinement {:?} is not {CONFINEMENT:?}",
                self.confinement
            ));
        }
        match (self.outcome, self.failure_class) {
            (Outcome::Pass, None) => Ok(()),
            (Outcome::HarnessError, Some(FailureClass::HarnessError)) => Ok(()),
            (Outcome::Fail, Some(c)) if c != FailureClass::HarnessError => Ok(()),
            (o, c) => Err(format!("outcome {o:?} cannot carry failure_class {c:?}")),
        }
    }

    /// The trial command's exit code: 0 pass, 1 fail, 2 harness error.
    pub fn exit_code(&self) -> i32 {
        match self.outcome {
            Outcome::Pass => 0,
            Outcome::Fail => 1,
            Outcome::HarnessError => 2,
        }
    }
}

/// What one `trial` invocation asks for.
#[derive(Debug, Clone, Default)]
pub struct TrialRequest {
    pub task: Option<String>,
    pub agent: Option<String>,
    pub argv: Vec<String>,
    pub deadline_ms: Option<u64>,
    pub artifacts_dir: Option<PathBuf>,
}

pub fn new_trial_id() -> String {
    use rand::Rng;
    let n: u32 = rand::rng().random();
    format!("{}-{n:08x}", chrono::Utc::now().format("%Y%m%dT%H%M%S"))
}

/// Resolve the Swarmail binary: `SWARMAIL_BIN` if set (it must be an
/// existing file), else build the root crate's `swarmail` once, offline and
/// locked, and take the executable path from Cargo's own artifact report
/// (so a custom target directory is honoured).
pub fn resolve_swarmail_bin() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os(SWARMAIL_BIN_ENV) {
        let path = PathBuf::from(path);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "{SWARMAIL_BIN_ENV}={} is not a file",
                path.display()
            ))
        };
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("evals has no parent directory")?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = std::process::Command::new(cargo)
        .current_dir(root)
        .args([
            "build",
            "--offline",
            "--locked",
            "--bin",
            "swarmail",
            "--message-format=json-render-diagnostics",
        ])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!("building swarmail failed: {}", out.status));
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|m| {
            m["reason"] == "compiler-artifact"
                && m["target"]["name"] == "swarmail"
                && m["target"]["kind"]
                    .as_array()
                    .is_some_and(|k| k.contains(&json!("bin")))
        })
        .and_then(|m| m["executable"].as_str().map(PathBuf::from))
        .ok_or_else(|| "cargo reported no swarmail executable".to_string())
}

/// The process groups this trial owns, for cleanup on any handled path.
#[derive(Clone, Default)]
pub struct Groups(Arc<Mutex<Vec<Arc<OwnedGroup>>>>);

type ExitWatch = tokio::sync::watch::Receiver<Option<Result<ExitStatus, String>>>;
type CleanupWatch = tokio::sync::watch::Receiver<Option<Result<(), String>>>;

struct OwnedGroup {
    pid: i32,
    pgid: i32,
    session: Option<i32>,
    start_time: Option<u64>,
    status: ExitWatch,
    cleanup: Mutex<Option<CleanupWatch>>,
    drains: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

#[derive(Debug, Clone, Copy)]
struct ProcIdentity {
    pid: i32,
    pgrp: i32,
    session: i32,
    start_time: u64,
    state: char,
}

fn parse_proc_stat(pid: i32, stat: &str) -> Result<ProcIdentity, String> {
    let (_, tail) = stat
        .rsplit_once(')')
        .ok_or_else(|| format!("/proc/{pid}/stat has no closing command delimiter"))?;
    let fields: Vec<_> = tail.split_whitespace().collect();
    if fields.len() <= 19 {
        return Err(format!("/proc/{pid}/stat has too few fields"));
    }
    Ok(ProcIdentity {
        pid,
        state: fields[0]
            .chars()
            .next()
            .ok_or_else(|| format!("/proc/{pid}/stat has no state"))?,
        pgrp: fields[2]
            .parse()
            .map_err(|_| format!("/proc/{pid}/stat has an invalid process group"))?,
        session: fields[3]
            .parse()
            .map_err(|_| format!("/proc/{pid}/stat has an invalid session"))?,
        start_time: fields[19]
            .parse()
            .map_err(|_| format!("/proc/{pid}/stat has an invalid start time"))?,
    })
}

fn proc_identity(pid: i32) -> Option<ProcIdentity> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        parse_proc_stat(pid, &stat).ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

fn group_members(group: &OwnedGroup) -> Result<Vec<ProcIdentity>, String> {
    #[cfg(all(test, target_os = "linux"))]
    if CLEANUP_OBSERVATION_TARGET
        .compare_exchange(
            group.pid,
            0,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_ok()
    {
        return Err(format!(
            "cannot inspect owned process group {}: injected observation error",
            group.pgid
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let entries =
            std::fs::read_dir("/proc").map_err(|e| format!("cannot inspect /proc: {e}"))?;
        let mut members = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("cannot enumerate /proc: {e}"))?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(pid) = name.parse::<i32>() else {
                continue;
            };
            let stat_path = entry.path().join("stat");
            let stat = match std::fs::read_to_string(&stat_path) {
                Ok(stat) => stat,
                // /proc enumeration races process exit. Linux may report
                // either ENOENT or ESRCH when the stat file disappears
                // between readdir and open; both prove this PID is gone.
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound
                        || e.raw_os_error() == Some(libc::ESRCH) =>
                {
                    continue;
                }
                Err(e) => return Err(format!("cannot inspect {}: {e}", stat_path.display())),
            };
            let identity = parse_proc_stat(pid, &stat)?;
            if identity.pgrp == group.pgid {
                let Some(session) = group.session else {
                    return Err(format!(
                        "cannot verify session identity for process group {}",
                        group.pgid
                    ));
                };
                if identity.session != session {
                    continue;
                }
                if identity.pid == group.pid && Some(identity.start_time) != group.start_time {
                    return Err(format!(
                        "process-group leader identity changed for {} before the group was observed empty",
                        group.pgid
                    ));
                }
                if !matches!(identity.state, 'Z' | 'X') {
                    members.push(identity);
                }
            }
        }
        Ok(members)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = group;
        // Only ESRCH proves absence on other Unix systems. Unknown and EPERM
        // are conservative presence, never cleanup success.
        let rc = unsafe { libc::kill(-group.pgid, 0) };
        if rc == 0 {
            Ok(vec![ProcIdentity {
                pid: group.pid,
                pgrp: group.pgid,
                session: group.session.unwrap_or_default(),
                start_time: group.start_time.unwrap_or_default(),
                state: 'R',
            }])
        } else {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::ESRCH) {
                Ok(Vec::new())
            } else if e.raw_os_error() == Some(libc::EPERM) {
                Ok(vec![ProcIdentity {
                    pid: group.pid,
                    pgrp: group.pgid,
                    session: group.session.unwrap_or_default(),
                    start_time: group.start_time.unwrap_or_default(),
                    state: 'R',
                }])
            } else {
                Err(format!("cannot inspect process group {}: {e}", group.pgid))
            }
        }
    }
}

fn signal_owned_group(group: &OwnedGroup, sig: i32) -> Result<(), String> {
    let members = group_members(group)?;
    if members.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    if group.session.is_none() {
        return Err(format!(
            "cannot verify session identity for owned process group {}",
            group.pgid
        ));
    }
    let rc = unsafe { libc::kill(-group.pgid, sig) };
    if rc == 0 {
        return Ok(());
    }
    let e = std::io::Error::last_os_error();
    if e.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(format!(
            "cannot signal owned process group {}: {e}",
            group.pgid
        ))
    }
}

#[cfg(test)]
fn cleanup_timeout_error(group: &OwnedGroup, running: usize) -> String {
    format!(
        "cleanup timed out for process group {} ({} running member(s), direct child reaped: {})",
        group.pgid,
        running,
        group.status().is_some()
    )
}

impl OwnedGroup {
    fn status(&self) -> Option<Result<ExitStatus, String>> {
        self.status.borrow().clone()
    }

    fn ensure_cleanup(
        self: &Arc<Self>,
    ) -> tokio::sync::watch::Receiver<Option<Result<(), String>>> {
        let mut slot = self.cleanup.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(rx) = slot.as_ref() {
            return rx.clone();
        }
        let (tx, rx) = tokio::sync::watch::channel(None);
        *slot = Some(rx.clone());
        let group = self.clone();
        tokio::spawn(async move {
            let result = group.cleanup_once().await;
            tx.send_replace(Some(result));
        });
        rx
    }

    async fn await_cleanup(self: &Arc<Self>) -> Result<(), String> {
        let mut rx = self.ensure_cleanup();
        loop {
            if let Some(result) = rx.borrow().clone() {
                return result;
            }
            rx.changed()
                .await
                .map_err(|_| format!("cleanup owner for process group {} vanished", self.pgid))?;
        }
    }

    async fn cleanup_once(self: Arc<Self>) -> Result<(), String> {
        #[cfg(test)]
        if let Some(dir) = std::env::var_os(CLEANUP_HOLD_ENV) {
            let dir = std::path::PathBuf::from(dir);
            if dir.join("arm").exists() {
                std::fs::create_dir_all(&dir)
                    .map_err(|e| format!("cleanup hold directory: {e}"))?;
                let entered = dir.join(format!("entered-{}", self.pid));
                std::fs::write(&entered, b"waiting")
                    .map_err(|e| format!("cleanup hold marker: {e}"))?;
                while !dir.join("release").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
        const TERM_GRACE: Duration = Duration::from_secs(5);
        const KILL_WINDOW: Duration = Duration::from_secs(5);
        let started = tokio::time::Instant::now();
        let term_deadline = started + TERM_GRACE;
        let final_deadline = started + TERM_GRACE + KILL_WINDOW;
        #[cfg(test)]
        {
            cleanup_test_event(&format!(
                "budget:{}:{}:{}",
                self.pid,
                TERM_GRACE.as_millis(),
                (TERM_GRACE + KILL_WINDOW).as_millis()
            ));
            if let Some(dir) = std::env::var_os(CLEANUP_BUDGET_HOLD_ENV) {
                let dir = std::path::PathBuf::from(dir);
                if dir.join("arm").exists() {
                    std::fs::create_dir_all(&dir)
                        .map_err(|e| format!("cleanup budget hold directory: {e}"))?;
                    std::fs::write(dir.join(format!("entered-{}", self.pid)), b"budget created")
                        .map_err(|e| format!("cleanup budget marker: {e}"))?;
                    let release = std::env::var_os(CLEANUP_BUDGET_RELEASE_ENV)
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| dir.join("release"));
                    while !release.exists() {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
            }
        }
        let mut signal_errors = Vec::new();
        if let Err(e) = signal_owned_group(&self, libc::SIGTERM) {
            signal_errors.push(e);
        }
        #[cfg(test)]
        cleanup_test_event(&format!("term:{}", self.pid));
        #[cfg(all(test, target_os = "linux"))]
        if std::env::var_os(CLEANUP_OBSERVE_ERROR_ENV).is_some() {
            let members = group_members(&self)?;
            if !members.is_empty() {
                CLEANUP_OBSERVATION_TARGET.store(self.pid, std::sync::atomic::Ordering::SeqCst);
            }
        }
        #[cfg(all(test, target_os = "linux"))]
        if std::env::var_os(CLEANUP_FORCE_EXPIRY_ENV).is_some() {
            let members = group_members(&self)?;
            if !members.is_empty() {
                return Err(cleanup_timeout_error(&self, members.len()));
            }
        }
        loop {
            let members = group_members(&self)?;
            if members.is_empty() && self.status().is_some() {
                break;
            }
            if tokio::time::Instant::now() >= term_deadline {
                break;
            }
            tokio::time::sleep_until(
                (tokio::time::Instant::now() + CLEANUP_POLL).min(term_deadline),
            )
            .await;
        }
        if (!group_members(&self)?.is_empty() || self.status().is_none())
            && let Err(e) = signal_owned_group(&self, libc::SIGKILL)
        {
            signal_errors.push(e);
        }
        loop {
            let members = group_members(&self)?;
            if members.is_empty() && self.status().is_some() {
                break;
            }
            if tokio::time::Instant::now() >= final_deadline {
                return Err(format!(
                    "cleanup timed out for process group {} ({} running member(s), direct child reaped: {})",
                    self.pgid,
                    members.len(),
                    self.status().is_some()
                ));
            }
            tokio::time::sleep_until(
                (tokio::time::Instant::now() + CLEANUP_POLL).min(final_deadline),
            )
            .await;
        }
        if let Some(Err(e)) = self.status() {
            return Err(format!("cannot reap process {}: {e}", self.pid));
        }
        let drains = std::mem::take(&mut *self.drains.lock().unwrap_or_else(|e| e.into_inner()));
        for mut drain in drains {
            if tokio::time::timeout_at(final_deadline, &mut drain)
                .await
                .is_err()
            {
                return Err(format!(
                    "output pipe did not close after process group {} stopped",
                    self.pgid
                ));
            }
        }
        if signal_errors.is_empty() {
            Ok(())
        } else {
            Err(signal_errors.join("; "))
        }
    }
}

impl Groups {
    fn add(&self, group: Arc<OwnedGroup>) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).push(group);
    }

    fn status_for_pid(&self, pid: i32) -> Option<Result<ExitStatus, String>> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|group| group.pid == pid)
            .and_then(|group| group.status())
    }

    async fn cleanup_pids(&self, pids: &[i32]) -> Result<(), String> {
        let wanted: std::collections::HashSet<_> = pids.iter().copied().collect();
        let groups = self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|g| wanted.contains(&g.pid))
            .cloned()
            .collect::<Vec<_>>();
        let mut tasks = tokio::task::JoinSet::new();
        for group in groups {
            tasks.spawn(async move { group.await_cleanup().await });
        }
        #[cfg(test)]
        let errors = if std::env::var_os(CLEANUP_WAIT_PENDING_ENV).is_some()
            || std::env::var_os(CLEANUP_WAIT_BYPASS_ENV).is_some()
        {
            let mut errors = Vec::new();
            let mut pending_recorded = false;
            while !tasks.is_empty() {
                let mut next = Box::pin(tasks.join_next());
                let mut was_pending = false;
                let ready = std::future::poll_fn(|cx| match next.as_mut().poll(cx) {
                    std::task::Poll::Ready(joined) => std::task::Poll::Ready(Some(joined)),
                    std::task::Poll::Pending => {
                        was_pending = true;
                        std::task::Poll::Ready(None)
                    }
                })
                .await;
                let joined = if was_pending {
                    if !pending_recorded {
                        if let Some(path) = std::env::var_os(CLEANUP_WAIT_PENDING_ENV) {
                            std::fs::write(path, b"owned cleanup future returned Pending")
                                .expect("write actual cleanup Pending acknowledgement");
                        }
                        pending_recorded = true;
                    }
                    if std::env::var_os(CLEANUP_WAIT_BYPASS_ENV).is_some() {
                        return Ok(());
                    }
                    next.await
                } else {
                    ready.expect("a completed join poll carries its result")
                };
                match joined {
                    Some(Ok(Ok(()))) => {}
                    Some(Ok(Err(e))) => errors.push(e),
                    Some(Err(e)) => errors.push(format!("cleanup waiter failed: {e}")),
                    None => break,
                }
            }
            errors
        } else {
            let mut errors = Vec::new();
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => errors.push(e),
                    Err(e) => errors.push(format!("cleanup waiter failed: {e}")),
                }
            }
            errors
        };
        #[cfg(not(test))]
        let errors = {
            let mut errors = Vec::new();
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => errors.push(e),
                    Err(e) => errors.push(format!("cleanup waiter failed: {e}")),
                }
            }
            errors
        };
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    pub async fn cleanup_all(&self) -> Result<(), String> {
        let pids = self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|g| g.pid)
            .collect::<Vec<_>>();
        self.cleanup_pids(&pids).await
    }
}

#[derive(Clone, Default)]
pub struct Log(Arc<Mutex<VecDeque<u8>>>);

impl Log {
    fn push(&self, bytes: &[u8]) {
        let mut buf = self.0.lock().unwrap_or_else(|e| e.into_inner());
        buf.extend(bytes);
        let excess = buf.len().saturating_sub(LOG_RETENTION);
        buf.drain(..excess);
    }

    pub fn text(&self) -> String {
        let buf = self.0.lock().unwrap_or_else(|e| e.into_inner());
        String::from_utf8_lossy(&buf.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

fn drain<R: AsyncRead + Unpin + Send + 'static>(
    reader: R,
    log: Log,
    lines: Option<mpsc::UnboundedSender<String>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut line = Vec::new();
        loop {
            line.clear();
            let mut limited = tokio::io::AsyncReadExt::take(&mut reader, 8192);
            match limited.read_until(b'\n', &mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    log.push(&line);
                    if let Some(tx) = &lines {
                        let _ = tx.send(String::from_utf8_lossy(&line).into_owned());
                    }
                }
            }
        }
    })
}

/// A process whose wait task and process-group record outlive the caller.
pub struct Proc {
    pub label: String,
    pub child: ProcessWait,
    pub pgid: i32,
    pub pid: i32,
    pub log: Log,
    group: Arc<OwnedGroup>,
    pub lines: mpsc::UnboundedReceiver<String>,
}

pub struct ProcessWait {
    group: Arc<OwnedGroup>,
}

impl Proc {
    pub fn next_line(&mut self) -> impl std::future::Future<Output = Option<String>> + '_ {
        self.lines.recv()
    }

    pub async fn cleanup(&self) -> Result<(), String> {
        self.group.await_cleanup().await
    }

    async fn announce<T>(&mut self, parse: impl Fn(&str) -> Option<T>) -> Result<T, String> {
        let deadline = tokio::time::Instant::now() + STARTUP_DEADLINE;
        loop {
            tokio::select! {
                line = self.lines.recv() => match line {
                    Some(line) => if let Some(value) = parse(&line) {
                        self.lines.close();
                        return Ok(value);
                    },
                    None => return Err(format!("{} exited before announcing its addresses", self.label)),
                },
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(format!("{} did not announce within {STARTUP_DEADLINE:?}", self.label));
                }
            }
        }
    }
}

impl ProcessWait {
    pub async fn wait(&mut self) -> std::io::Result<ExitStatus> {
        let mut rx = self.group.status.clone();
        loop {
            if let Some(result) = rx.borrow().clone() {
                return result.map_err(std::io::Error::other);
            }
            rx.changed().await.map_err(|_| {
                std::io::Error::other(format!(
                    "wait owner for process {} vanished",
                    self.group.pid
                ))
            })?;
        }
    }
}

pub fn spawn_owned_command(cmd: Command, label: &str, groups: &Groups) -> std::io::Result<Proc> {
    spawn(cmd, label, groups)
}

fn spawn(mut cmd: Command, label: &str, groups: &Groups) -> std::io::Result<Proc> {
    cmd.process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| std::io::Error::other("spawned child has no pid"))? as i32;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let identity = proc_identity(pid);
    let (status_tx, status) = tokio::sync::watch::channel(None);
    let group = Arc::new(OwnedGroup {
        pid,
        pgid: pid,
        session: identity.map(|i| i.session),
        start_time: identity.map(|i| i.start_time),
        status,
        cleanup: Mutex::new(None),
        drains: Mutex::new(Vec::new()),
    });
    groups.add(group.clone());
    tokio::spawn(async move {
        let result = child.wait().await.map_err(|e| e.to_string());
        status_tx.send_replace(Some(result));
    });
    let log = Log::default();
    let (tx, rx) = mpsc::unbounded_channel();
    let mut drains = group.drains.lock().unwrap_or_else(|e| e.into_inner());
    drains.push(drain(stdout, log.clone(), Some(tx)));
    drains.push(drain(stderr, log.clone(), None));
    drop(drains);
    Ok(Proc {
        label: label.to_string(),
        child: ProcessWait {
            group: group.clone(),
        },
        pgid: pid,
        pid,
        log,
        group,
        lines: rx,
    })
}

/// Parse Swarmail's startup line (`smtp=… pop3=… http=…`), the same
/// reading `tests/binary.rs` applies.
pub fn parse_swarmail_addrs(line: &str) -> Option<(SocketAddr, SocketAddr)> {
    let field = |k: &str| {
        line.split(k)
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    let smtp = field("smtp=")?;
    let _pop3: SocketAddr = field("pop3=")?;
    let http = field("http=")?;
    Some((smtp, http))
}

pub fn parse_fixture_addrs(line: &str) -> Option<(SocketAddr, SocketAddr)> {
    let rest = line.trim().strip_prefix("fixture ")?;
    let field = |k: &str| {
        rest.split(k)
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    Some((field("public=")?, field("admin=")?))
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("a plain http client builds")
}

struct Admin {
    http: reqwest::Client,
    base: String,
    secret: String,
}

impl Admin {
    async fn call(&self, post: bool, path: &str, body: Option<Value>) -> Result<Value, String> {
        let url = format!("{}{path}", self.base);
        let req = if post {
            self.http.post(&url)
        } else {
            self.http.get(&url)
        };
        let mut req = req.bearer_auth(&self.secret);
        if let Some(body) = body {
            req = req
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        let resp = req.send().await.map_err(|e| format!("{path}: {e}"))?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|e| format!("{path}: {e}"))?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))?;
        if status != 200 {
            return Err(format!("{path}: status {status}: {value}"));
        }
        Ok(value)
    }
}

/// One MCP call from the runner (the chaos barrier). The app itself only
/// knows SMTP; arming and clearing chaos is the runner's business.
async fn runner_mcp(mcp_url: &str, tool: &str, args: Value) -> Result<(), String> {
    let mut client = crate::mcp::McpClient::connect(mcp_url).await?;
    client.call(tool, args).await.map(|_| ())
}

fn deliveries_with(value: &Value, status: DeliveryStatus) -> Result<usize, String> {
    let list = value["deliveries"]
        .as_array()
        .ok_or("no deliveries in reply")?;
    let want = serde_json::to_value(status).expect("status serializes");
    Ok(list.iter().filter(|d| d["status"] == want).count())
}

/// Everything a handled ending needs to clean up.
struct Trial {
    groups: Groups,
    servers: Vec<Proc>,
    agents: Arc<Mutex<Vec<(String, i32)>>>,
    diagnostics: Vec<String>,
}

impl Trial {
    async fn stop_servers(&mut self) -> Result<(), String> {
        let pids: Vec<_> = self.servers.iter().map(|p| p.pid).collect();
        self.groups.cleanup_pids(&pids).await?;
        while let Some(p) = self.servers.pop() {
            let log = p.log.text();
            if !log.is_empty() {
                self.diagnostics
                    .push(format!("--- {} log (tail) ---\n{log}", p.label));
            }
        }
        Ok(())
    }
}

/// What the agents phase ended with.
enum AgentsEnd {
    Done,
    Deadline,
    ServerDied(String),
}

fn exit_of(identity: &str, status: Option<ExitStatus>) -> AgentExit {
    AgentExit {
        identity: identity.to_string(),
        code: status.and_then(|s| s.code()),
        signal: status.and_then(|s| s.signal()),
    }
}

struct Identity {
    identity: String,
    inbox: String,
}

fn identities(trial_id: &str, count: u8) -> Vec<Identity> {
    use rand::Rng;
    let names = ["ada", "grace", "barbara"];
    let short: String = trial_id
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    (0..count as usize)
        .map(|i| {
            let n: u32 = rand::rng().random_range(0x10000..0xfffff);
            Identity {
                identity: format!("{}.{n:x}@example.test", names[i]),
                inbox: format!("eval-{short}-{}", names[i]),
            }
        })
        .collect()
}

/// The trial's resolved agent argv (built-ins run as our own child).
fn evals_executable() -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Some(path) = std::env::var_os("EVAL_TEST_RUNNER_EXECUTABLE") {
        return Ok(PathBuf::from(path));
    }
    std::env::current_exe().map_err(|e| format!("current_exe: {e}"))
}

fn agent_argv(label: &str, argv: &[String]) -> Result<Vec<String>, String> {
    if BUILTINS.contains(&label) {
        let exe = evals_executable()?;
        Ok(vec![
            exe.display().to_string(),
            "agent".to_string(),
            label.to_string(),
        ])
    } else {
        Ok(argv.to_vec())
    }
}

/// Validate the request before anything starts.
pub fn validate_request(req: &TrialRequest) -> Result<(Task, String), String> {
    let task_id = req.task.as_deref().ok_or("--task is required")?;
    let mut task = tasks::find(task_id).ok_or_else(|| format!("unknown task {task_id}"))?;
    let label = req.agent.as_deref().ok_or("--agent is required")?;
    if label.is_empty() {
        return Err("--agent must not be empty".into());
    }
    if BUILTINS.contains(&label) && !req.argv.is_empty() {
        return Err(format!("the built-in agent {label} takes no command"));
    }
    if !BUILTINS.contains(&label) && req.argv.is_empty() {
        return Err(format!(
            "the external agent {label} needs a command after --"
        ));
    }
    if let Some(ms) = req.deadline_ms {
        if ms == 0 || ms > tasks::MAX_DEADLINE_MS {
            return Err(format!(
                "--deadline-ms must be between 1 and {}",
                tasks::MAX_DEADLINE_MS
            ));
        }
        task.deadline_ms = ms;
    }
    tasks::validate_task(&task)?;
    Ok((task, label.to_string()))
}

/// The harness-error row for a request that never started.
pub fn refused_row(req: &TrialRequest, trial_id: &str) -> TrialResult {
    TrialResult {
        schema_version: RESULT_SCHEMA,
        trial_id: trial_id.to_string(),
        task: req.task.clone(),
        agent: req.agent.clone(),
        outcome: Outcome::HarnessError,
        failure_class: Some(FailureClass::HarnessError),
        wall_time_ms: 0,
        agent_exit_status: None,
        agent_exits: Vec::new(),
        confinement: CONFINEMENT.to_string(),
    }
}

/// The body of one trial. Every error return is a harness error; the
/// caller still owns cleanup through `trial`. `Ok(true)` means the agent
/// deadline expired.
#[allow(clippy::too_many_arguments)]
async fn run_body(
    task: &Task,
    label: &str,
    argv: &[String],
    trial_id: &str,
    swarmail_bin: &Path,
    scratch: &Path,
    trial: &mut Trial,
    exits: &mut Vec<AgentExit>,
    snapshot_out: &mut Option<Snapshot>,
) -> Result<bool, String> {
    // Swarmail: fresh, in memory, every listener on loopback port zero,
    // inherited SWARMAIL_* settings removed, startup logging pinned.
    let mut cmd = Command::new(swarmail_bin);
    cmd.args([
        "serve",
        "--smtp-listen",
        "127.0.0.1:0",
        "--pop3-listen",
        "127.0.0.1:0",
        "--http-listen",
        "127.0.0.1:0",
        "--max-per-inbox",
        "100000",
    ]);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("SWARMAIL_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("RUST_LOG", "swarmail=info,tower_http=off")
        .current_dir(scratch);
    let mut swarmail = spawn(cmd, "swarmail", &trial.groups)
        .map_err(|e| format!("cannot start {}: {e}", swarmail_bin.display()))?;
    let announced = swarmail.announce(parse_swarmail_addrs).await;
    trial.servers.push(swarmail);
    let (smtp, http) = announced?;
    let client = http_client();
    let health = client
        .get(format!("http://{http}/healthz"))
        .send()
        .await
        .map_err(|e| format!("swarmail health: {e}"))?;
    if !health.status().is_success() {
        return Err(format!("swarmail health: {}", health.status()));
    }
    mail::probe_greeting(smtp)
        .await
        .map_err(|e| format!("swarmail SMTP: {e}"))?;
    let mcp_url = format!("http://{http}/mcp");

    // The fixture, with the admin secret in its environment alone.
    let secret: String = {
        use rand::Rng;
        use rand::distr::Alphanumeric;
        rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    };
    let exe = evals_executable()?;
    let mut cmd = Command::new(&exe);
    cmd.args([
        "fixture",
        "--smtp",
        &smtp.to_string(),
        "--task",
        &task.id,
        "--trial-id",
        trial_id,
    ])
    .env(ADMIN_SECRET_ENV, &secret)
    .current_dir(scratch);
    let mut fixture = spawn(cmd, "fixture", &trial.groups)
        .map_err(|e| format!("cannot start the fixture: {e}"))?;
    let announced = fixture.announce(parse_fixture_addrs).await;
    trial.servers.push(fixture);
    let (public, admin_addr) = announced?;
    let app_url = format!("http://{public}");
    let index = client
        .get(format!("{app_url}/"))
        .send()
        .await
        .map_err(|e| format!("fixture: {e}"))?;
    if !index.status().is_success() {
        return Err(format!("fixture index: {}", index.status()));
    }
    let admin = Admin {
        http: client.clone(),
        base: format!("http://{admin_addr}"),
        secret,
    };

    // Provision, then the scenario's barriers — all before any agent.
    let ids = identities(trial_id, task.agent_count);
    let provision: Vec<Value> = ids
        .iter()
        .map(|i| json!({ "identity": i.identity, "inbox": i.inbox }))
        .collect();
    admin
        .call(
            true,
            "/admin/provision",
            Some(json!({ "identities": provision })),
        )
        .await?;
    if task.scenario.uses_chaos() {
        runner_mcp(
            &mcp_url,
            "swarmail_set_chaos",
            json!({ "event": "data", "probability": 100 }),
        )
        .await?;
        let first = admin.call(true, "/admin/prepare", None).await;
        // Clear chaos whatever happened: it must never outlive its barrier.
        runner_mcp(&mcp_url, "swarmail_clear_chaos", json!({})).await?;
        let first = first?;
        if deliveries_with(&first, DeliveryStatus::Parked)? != task.agent_count as usize {
            return Err(format!(
                "chaos barrier: expected a parked 451 per identity, got {first}"
            ));
        }
        let second = admin.call(true, "/admin/retry", None).await?;
        if deliveries_with(&second, DeliveryStatus::Delivered)? != task.agent_count as usize {
            return Err(format!(
                "chaos barrier: the retry was not delivered: {second}"
            ));
        }
    } else {
        let prepared = admin.call(true, "/admin/prepare", None).await?;
        if deliveries_with(&prepared, DeliveryStatus::Failed)? > 0
            || deliveries_with(&prepared, DeliveryStatus::Parked)? > 0
        {
            return Err(format!("fixture delivery failed: {prepared}"));
        }
    }

    // The agents: one process group each, launched together.
    let argv = agent_argv(label, argv)?;
    let mut waits = tokio::task::JoinSet::new();
    let mut agent_groups = Vec::new();
    let mut logs = Vec::new();
    let mut spawn_failed = None;
    for (index, id) in ids.iter().enumerate() {
        let vars = PromptVars {
            app_url: &app_url,
            mcp_url: &mcp_url,
            identity: &id.identity,
            inbox: &id.inbox,
        };
        let prompt = tasks::render_prompt(task, &vars)?;
        let workdir = scratch.join(format!("agent-{index}"));
        std::fs::create_dir_all(&workdir).map_err(|e| format!("workdir: {e}"))?;
        let prompt_file = workdir.join("prompt.md");
        std::fs::write(&prompt_file, &prompt).map_err(|e| format!("prompt file: {e}"))?;
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..])
            .current_dir(&workdir)
            .env_remove(ADMIN_SECRET_ENV)
            .env(ENV_TASK, &task.id)
            .env(ENV_APP_URL, &app_url)
            .env(ENV_MCP_URL, &mcp_url)
            .env(ENV_IDENTITY, &id.identity)
            .env(ENV_INBOX, &id.inbox)
            .env(ENV_GOAL, &prompt)
            .env(ENV_PROMPT_FILE, &prompt_file)
            .env(ENV_DEADLINE_MS, task.deadline_ms.to_string());
        match spawn(cmd, &format!("agent {}", id.identity), &trial.groups) {
            Ok(Proc {
                label,
                mut child,
                pgid,
                pid,
                log,
                ..
            }) => {
                agent_groups.push(pgid);
                trial
                    .agents
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push((id.identity.clone(), pid));
                logs.push((label, log));
                waits.spawn(async move { (index, child.wait().await.ok()) });
            }
            Err(e) => {
                spawn_failed = Some(format!("cannot spawn agent {:?}: {e}", argv[0]));
                break;
            }
        }
    }
    let mut statuses: Vec<Option<ExitStatus>> = vec![None; ids.len()];
    let end = if spawn_failed.is_some() {
        AgentsEnd::Done
    } else {
        #[cfg(test)]
        match std::env::var(FAULT_ENV).as_deref() {
            Ok("panic") => panic!("injected runner fault"),
            Ok("panic_after_agent_readiness") => {
                if let Some(path) = std::env::var_os(PIDS_ENV_FOR_TESTS).map(PathBuf::from) {
                    while !path.is_file() {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
                if let Some(release) = std::env::var_os(FAULT_RELEASE_ENV).map(PathBuf::from) {
                    while !release.exists() {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
                panic!("injected runner fault after agent readiness");
            }
            Ok("server_death") => {
                let _ = signal_owned_group(&trial.servers[0].group, libc::SIGKILL);
            }
            _ => {}
        }
        let deadline = tokio::time::Instant::now() + Duration::from_millis(task.deadline_ms);
        let (swarmail_slot, fixture_slot) = trial.servers.split_at_mut(1);
        let swarmail = &mut swarmail_slot[0].child;
        let fixture = &mut fixture_slot[0].child;
        loop {
            tokio::select! {
                joined = waits.join_next() => match joined {
                    Some(Ok((i, status))) => statuses[i] = status,
                    Some(Err(_)) => {}
                    None => break AgentsEnd::Done,
                },
                _ = tokio::time::sleep_until(deadline) => break AgentsEnd::Deadline,
                s = swarmail.wait() => break AgentsEnd::ServerDied(format!("swarmail exited during the trial: {s:?}")),
                s = fixture.wait() => break AgentsEnd::ServerDied(format!("the fixture exited during the trial: {s:?}")),
            }
        }
    };
    // Stop every agent group concurrently and retain their direct waiters.
    let cleanup_result = trial.groups.cleanup_pids(&agent_groups).await;
    while let Some(joined) = waits.join_next().await {
        if let Ok((i, status)) = joined {
            statuses[i] = status;
        }
    }
    *exits = ids
        .iter()
        .zip(&statuses)
        .map(|(id, s)| exit_of(&id.identity, *s))
        .collect();
    for (label, log) in logs {
        let text = log.text();
        if !text.is_empty() {
            trial
                .diagnostics
                .push(format!("--- {label} log (tail) ---\n{text}"));
        }
    }
    if let Some(e) = spawn_failed {
        return Err(e);
    }
    if let Err(e) = cleanup_result {
        return Err(format!("agent cleanup failed: {e}"));
    }
    match end {
        AgentsEnd::ServerDied(why) => return Err(why),
        AgentsEnd::Done | AgentsEnd::Deadline => {}
    }

    // Freeze, then take one consistent snapshot before the servers stop.
    admin.call(true, "/admin/freeze", None).await?;
    let snap = admin.call(false, "/admin/snapshot", None).await?;
    let snap: Snapshot = serde_json::from_value(snap).map_err(|e| format!("snapshot: {e}"))?;
    *snapshot_out = Some(snap);
    Ok(matches!(end, AgentsEnd::Deadline))
}

/// The artifacts a requested directory retains for one trial.
fn write_artifacts(
    dir: &Path,
    row: &TrialResult,
    snap: Option<&Snapshot>,
    diagnostics: &[String],
    bin: Option<&Path>,
) -> Result<(), String> {
    let out = dir.join(&row.trial_id);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let tmp = out.join(format!(".{name}.tmp"));
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, out.join(name)).map_err(|e| e.to_string())
    };
    if let Some(snap) = snap {
        write(
            "state.json",
            &serde_json::to_vec_pretty(snap).expect("serializes"),
        )?;
    }
    write(
        "result.json",
        &serde_json::to_vec_pretty(row).expect("serializes"),
    )?;
    write("diagnostics.log", diagnostics.join("\n").as_bytes())?;
    let meta = json!({
        "harness": env!("CARGO_PKG_NAME"),
        "harness_version": env!("CARGO_PKG_VERSION"),
        "swarmail_bin": bin.map(|b| b.display().to_string()),
        "confinement": CONFINEMENT,
    });
    write(
        "metadata.json",
        &serde_json::to_vec_pretty(&meta).expect("serializes"),
    )
}

/// Run one trial to its one result row. Diagnostics go to stderr.
pub async fn run_trial(req: TrialRequest, swarmail_bin: Option<PathBuf>) -> TrialResult {
    let trial_id = new_trial_id();
    let (task, label) = match validate_request(&req) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("swarmail-evals: {e}");
            return refused_row(&req, &trial_id);
        }
    };
    let bin = match swarmail_bin.map(Ok).unwrap_or_else(resolve_swarmail_bin) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("swarmail-evals: {e}");
            return refused_row(&req, &trial_id);
        }
    };
    #[cfg(test)]
    if let Some(dir) = std::env::var_os(PRE_HANDLER_HOLD_ENV) {
        let dir = PathBuf::from(dir);
        if std::fs::create_dir_all(&dir).is_ok()
            && std::fs::write(dir.join("entered"), b"waiting").is_ok()
        {
            while !dir.join("release").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    let started = Instant::now();
    let groups = Groups::default();
    let agents = Arc::new(Mutex::new(Vec::<(String, i32)>::new()));
    // Registered before any child starts, so an interrupt at any phase
    // takes the cleanup path and still yields the row.
    let signals = (
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()),
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()),
    );
    let (mut term, mut int) = match signals {
        (Ok(t), Ok(i)) => (t, i),
        _ => {
            eprintln!("swarmail-evals: cannot install signal handlers");
            return refused_row(&req, &trial_id);
        }
    };
    let scratch = match tempfile::Builder::new().prefix("swarmail-eval-").tempdir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("swarmail-evals: scratch dir: {e}");
            return refused_row(&req, &trial_id);
        }
    };
    let mut body = {
        let (task, label, argv, trial_id, bin, scratch_path, groups, agents) = (
            task.clone(),
            label.clone(),
            req.argv.clone(),
            trial_id.clone(),
            bin.clone(),
            scratch.path().to_path_buf(),
            groups.clone(),
            agents.clone(),
        );
        tokio::spawn(async move {
            let mut trial = Trial {
                groups,
                servers: Vec::new(),
                agents,
                diagnostics: Vec::new(),
            };
            let mut exits = Vec::new();
            let mut snap = None;
            let outcome = run_body(
                &task,
                &label,
                &argv,
                &trial_id,
                &bin,
                &scratch_path,
                &mut trial,
                &mut exits,
                &mut snap,
            )
            .await;
            let outcome = match trial.stop_servers().await {
                Ok(()) => outcome,
                Err(e) => Err(format!("server cleanup failed: {e}")),
            };
            (outcome, exits, snap, trial.diagnostics)
        })
    };
    let joined = tokio::select! {
        biased;
        _ = term.recv() => {
            #[cfg(test)]
            runner_signal_event("term");
            None
        },
        _ = int.recv() => {
            #[cfg(test)]
            runner_signal_event("int");
            None
        },
        joined = &mut body => Some(joined),
    };
    let Some(joined) = joined else {
        // The body may be cancelled at any await point. Child waiters and
        // cleanup owners live in the shared group registry, outside the body.
        body.abort();
        let _ = body.await;
        let cleanup = groups.cleanup_all();
        tokio::pin!(cleanup);
        let cleanup_result = loop {
            tokio::select! {
                biased;
                result = &mut cleanup => break result,
                _ = term.recv() => {},
                _ = int.recv() => {},
            }
        };
        if let Err(e) = cleanup_result {
            eprintln!("swarmail-evals: interrupted trial cleanup failed: {e}");
        }
        let agent_exits: Vec<_> = agents
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(identity, pid)| {
                exit_of(identity, groups.status_for_pid(*pid).and_then(Result::ok))
            })
            .collect();
        eprintln!("swarmail-evals: harness error: the trial was interrupted");
        return TrialResult {
            schema_version: RESULT_SCHEMA,
            trial_id,
            task: req.task.clone(),
            agent: req.agent.clone(),
            outcome: Outcome::HarnessError,
            failure_class: Some(FailureClass::HarnessError),
            wall_time_ms: started.elapsed().as_millis() as u64,
            agent_exit_status: if task.agent_count == 1 {
                agent_exits.first().and_then(|e| e.code)
            } else {
                None
            },
            agent_exits,
            confinement: CONFINEMENT.to_string(),
        };
    };
    let (mut verdict, exits, snap, mut diagnostics) = match joined {
        Ok((outcome, exits, snap, diagnostics)) => {
            // Precedence (0055 §2): infrastructure — a process error, or a
            // snapshot that is absent or unscorable (a failed or parked
            // fixture delivery included) — then the deadline, then the
            // business check.
            let verdict = match (&outcome, &snap) {
                (Err(e), _) => {
                    eprintln!("swarmail-evals: harness error: {e}");
                    Verdict::harness_error()
                }
                (Ok(_), None) => Verdict::harness_error(),
                (Ok(deadline), Some(snap)) => match tasks::validate_snapshot(&task, snap) {
                    Err(e) => {
                        eprintln!("swarmail-evals: harness error: unscorable snapshot: {e}");
                        Verdict::harness_error()
                    }
                    Ok(()) if *deadline => Verdict::fail(FailureClass::Budget),
                    Ok(()) => match tasks::score(&task, snap) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("swarmail-evals: harness error: unscorable snapshot: {e}");
                            Verdict::harness_error()
                        }
                    },
                },
            };
            (verdict, exits, snap, diagnostics)
        }
        Err(join) => {
            eprintln!("swarmail-evals: harness error: the runner panicked: {join}");
            (
                Verdict::harness_error(),
                Vec::new(),
                None,
                vec![format!("runner panicked: {join}")],
            )
        }
    };
    let cleanup = groups.cleanup_all();
    tokio::pin!(cleanup);
    let mut interrupted_during_cleanup = false;
    let cleanup_result = loop {
        tokio::select! {
            biased;
            result = &mut cleanup => break result,
            _ = term.recv() => {
                interrupted_during_cleanup = true;
                #[cfg(test)]
                runner_signal_event("term-cleanup");
            },
            _ = int.recv() => {
                interrupted_during_cleanup = true;
                #[cfg(test)]
                runner_signal_event("int-cleanup");
            },
        }
    };
    if let Err(e) = cleanup_result {
        diagnostics.push(format!("owned process cleanup failed: {e}"));
        eprintln!("swarmail-evals: harness error: owned process cleanup failed: {e}");
        verdict = Verdict::harness_error();
    }
    if interrupted_during_cleanup {
        eprintln!("swarmail-evals: harness error: the trial was interrupted");
        verdict = Verdict::harness_error();
    }
    let agent_exit_status = if task.agent_count == 1 {
        exits.first().and_then(|e| e.code)
    } else {
        None
    };
    let row = TrialResult {
        schema_version: RESULT_SCHEMA,
        trial_id: trial_id.clone(),
        task: Some(task.id.clone()),
        agent: Some(label),
        outcome: verdict.outcome,
        failure_class: verdict.failure_class,
        wall_time_ms: started.elapsed().as_millis() as u64,
        agent_exit_status,
        agent_exits: exits,
        confinement: CONFINEMENT.to_string(),
    };
    if let Some(dir) = &req.artifacts_dir
        && let Err(e) = write_artifacts(dir, &row, snap.as_ref(), &diagnostics, Some(&bin))
    {
        diagnostics.push(format!("artifacts: {e}"));
        eprintln!("swarmail-evals: cannot write artifacts: {e}");
    }
    if row.outcome == Outcome::HarnessError {
        for d in &diagnostics {
            eprintln!("{d}");
        }
    }
    drop(scratch);
    row
}

#[cfg(all(test, target_os = "linux"))]
mod test_probes {
    use super::*;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::Command;

    const PROBE_ENV: &str = "EVAL_TEST_RUNNER_PROBE";
    const SCRIPT_ENV: &str = "EVAL_TEST_RUNNER_SCRIPT";
    const PIDS_ENV: &str = "EVAL_TEST_RUNNER_PIDS";
    const BIN_ENV: &str = "EVAL_TEST_RUNNER_BIN";
    const GUARD_FAILURE_ENV: &str = "EVAL_TEST_RUNNER_GUARD_FINISH_DIR";
    const GUARD_FAILURE_READY: &str = "guard-failure-ready";
    const GUARD_FAILURE_RELEASE: &str = "guard-failure-release";

    #[derive(Clone, Debug)]
    struct ProcessIdentity {
        pid: i32,
        pgrp: i32,
        session: i32,
        start_time: u64,
        state: char,
    }

    fn identity(pid: i32) -> Option<ProcessIdentity> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let (_, tail) = stat.rsplit_once(')')?;
        let fields: Vec<_> = tail.split_whitespace().collect();
        Some(ProcessIdentity {
            pid,
            state: fields.first()?.chars().next()?,
            pgrp: fields.get(2)?.parse().ok()?,
            session: fields.get(3)?.parse().ok()?,
            start_time: fields.get(19)?.parse().ok()?,
        })
    }

    fn same_identity(want: &ProcessIdentity, now: &ProcessIdentity) -> bool {
        want.pid == now.pid
            && want.pgrp == now.pgrp
            && want.session == now.session
            && want.start_time == now.start_time
    }

    fn alive(want: &ProcessIdentity) -> bool {
        identity(want.pid)
            .is_some_and(|now| same_identity(want, &now) && !matches!(now.state, 'Z' | 'X'))
    }

    fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !condition() {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {what}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn runner_probe_entrypoint() {
        if std::env::var_os(PROBE_ENV).is_none() {
            return;
        }
        let script = std::env::var_os(SCRIPT_ENV).expect("probe script path");
        let bin = std::env::var_os(BIN_ENV).expect("Swarmail binary path");
        let req = TrialRequest {
            task: Some("signup".into()),
            agent: Some("runner-probe".into()),
            argv: vec!["/bin/sh".into(), script.to_string_lossy().into_owned()],
            deadline_ms: Some(60_000),
            artifacts_dir: None,
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let row = runtime.block_on(run_trial(req, Some(PathBuf::from(bin))));
        assert_eq!(row.outcome, Outcome::HarnessError);
        println!("{}", serde_json::to_string(&row).unwrap());
    }

    #[test]
    fn runner_guard_finish_failure_entrypoint() {
        let Ok(dir) = std::env::var(GUARD_FAILURE_ENV) else {
            return;
        };
        let dir = PathBuf::from(dir);
        let ready = dir.join(GUARD_FAILURE_READY);
        let release = dir.join(GUARD_FAILURE_RELEASE);
        let script = "sleep 1000 & child=$!; printf '%s %s\n' \"$$\" \"$child\" > \"$EVAL_TEST_GUARD_PIDS\"; exit 0";
        let pids = dir.join("guard-owned-pids");
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(script)
            .env("EVAL_TEST_GUARD_PIDS", &pids)
            .process_group(0)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let owner = identity(child.id() as i32).expect("guard fixture leader identity");
        let pid_deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !pids.is_file() {
            assert!(
                std::time::Instant::now() < pid_deadline,
                "guard fixture readiness"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let pids: Vec<i32> = std::fs::read_to_string(&pids)
            .unwrap()
            .split_whitespace()
            .map(|pid| pid.parse().unwrap())
            .collect();
        assert_eq!(pids.len(), 2);
        let direct = identity(pids[0]).expect("direct child identity before wait");
        let survivor = identity(pids[1]).expect("unreaped group member identity");
        let status = child.wait().unwrap();
        assert!(status.success());
        std::fs::write(&ready, b"direct child waited; survivor retained").unwrap();
        let release_deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !release.exists() {
            assert!(
                std::time::Instant::now() < release_deadline,
                "parent acknowledges identities"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut probe = Probe {
            child: Some(child),
            owner,
            release,
            pending: dir.join("unused-pending"),
            owned: vec![direct, survivor],
        };
        let error = probe
            .finish()
            .expect_err("guard must reject an unresolved owned process");
        assert!(error.contains("unresolved owned identities"), "{error}");
        panic!("intentional guard finish rejection: {error}");
    }

    #[test]
    fn runner_guard_finish_rejection_fails_in_isolated_process_and_rescues() {
        let dir = tempfile::tempdir().unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "runner::test_probes::runner_guard_finish_failure_entrypoint",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(GUARD_FAILURE_ENV, dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        eventually("isolated guard failure identities", || {
            dir.path().join(GUARD_FAILURE_READY).is_file()
        });
        let pids: Vec<i32> = std::fs::read_to_string(dir.path().join("guard-owned-pids"))
            .unwrap()
            .split_whitespace()
            .map(|pid| pid.parse().unwrap())
            .collect();
        let survivor = identity(pids[1]).expect("guard fixture descendant remains live");
        assert!(
            alive(&survivor),
            "the parent sees the unresolved descendant"
        );
        std::fs::write(dir.path().join(GUARD_FAILURE_RELEASE), b"release").unwrap();
        let output = child.wait_with_output().unwrap();
        let transcript = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "intentional failure returned success: {transcript}"
        );
        assert!(
            transcript.contains("intentional guard finish rejection"),
            "{transcript}"
        );
        eventually(
            "isolated failure teardown reaps its retained identities",
            || !alive(&survivor),
        );
    }

    struct Probe {
        child: Option<std::process::Child>,
        owner: ProcessIdentity,
        release: PathBuf,
        pending: PathBuf,
        owned: Vec<ProcessIdentity>,
    }

    impl Probe {
        fn pid(&self) -> i32 {
            self.child.as_ref().unwrap().id() as i32
        }

        fn release(&self) {
            let _ = std::fs::write(&self.release, b"release");
        }

        fn wait_for(&mut self, limit: Duration) -> Option<std::process::ExitStatus> {
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
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        fn wait(&mut self) -> std::process::ExitStatus {
            self.wait_for(Duration::from_secs(30))
                .unwrap_or_else(|| panic!("runner probe exceeded its 30-second wait deadline"))
        }

        fn finish(&mut self) -> Result<(), String> {
            let child = self
                .child
                .as_mut()
                .ok_or_else(|| "runner probe child handle was already removed".to_string())?;
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => return Err("runner probe still requires owner rescue".into()),
                Err(e) => return Err(format!("cannot reap runner probe child: {e}")),
            }
            let survivors: Vec<_> = self
                .owned
                .iter()
                .filter(|identity| alive(identity))
                .collect();
            if !survivors.is_empty() {
                return Err(format!(
                    "runner probe has unresolved owned identities: {survivors:?}"
                ));
            }
            Ok(())
        }

        fn returned_after_owned_cleanup(&mut self) -> bool {
            self.child
                .as_mut()
                .is_some_and(|child| child.try_wait().ok().flatten().is_some())
                && self.owned.iter().all(|id| !alive(id))
        }

        fn signal_owned_groups(&self, signal: i32) {
            let groups: std::collections::HashSet<_> =
                self.owned.iter().map(|id| id.pgrp).collect();
            for pgrp in groups {
                let has_known_live_member = self.owned.iter().any(|want| {
                    want.pgrp == pgrp
                        && identity(want.pid).is_some_and(|now| {
                            same_identity(want, &now) && !matches!(now.state, 'Z' | 'X')
                        })
                });
                if has_known_live_member {
                    // SAFETY: the group is signalled only while a recorded PID/start-time/session
                    // identity still proves membership in this test-owned process group.
                    unsafe { libc::kill(-pgrp, signal) };
                }
            }
        }

        fn wait_owned_stopped(&self, limit: Duration) -> bool {
            let deadline = std::time::Instant::now() + limit;
            while self.owned.iter().any(alive) {
                if std::time::Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            true
        }

        fn cleanup_owned(&self) -> bool {
            self.signal_owned_groups(libc::SIGTERM);
            if self.wait_owned_stopped(Duration::from_secs(5)) {
                return true;
            }
            self.signal_owned_groups(libc::SIGKILL);
            self.wait_owned_stopped(Duration::from_secs(5))
        }

        fn record_owned_from_tree(&mut self) {
            let root = self.pid();
            if !identity(root).is_some_and(|now| same_identity(&self.owner, &now)) {
                return;
            }
            let mut processes = Vec::new();
            let Ok(entries) = std::fs::read_dir("/proc") else {
                return;
            };
            for entry in entries.flatten() {
                let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else {
                    continue;
                };
                if let Some(process) = identity(pid) {
                    let ppid = std::fs::read_to_string(format!("/proc/{pid}/stat"))
                        .ok()
                        .and_then(|stat| {
                            stat.rsplit_once(')')?
                                .1
                                .split_whitespace()
                                .nth(1)?
                                .parse::<i32>()
                                .ok()
                        });
                    if let Some(ppid) = ppid {
                        processes.push((process, ppid));
                    }
                }
            }
            let mut parents = std::collections::HashSet::from([root]);
            loop {
                let children: Vec<_> = processes
                    .iter()
                    .filter(|(_, ppid)| parents.contains(ppid))
                    .map(|(process, _)| process.clone())
                    .collect();
                let added = children
                    .iter()
                    .filter(|child| parents.insert(child.pid))
                    .count();
                for child in children {
                    if !self.owned.iter().any(|known| same_identity(known, &child)) {
                        self.owned.push(child);
                    }
                }
                if added == 0 {
                    break;
                }
            }
        }
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            self.release();
            self.record_owned_from_tree();
            if self.child.is_some() && self.wait_for(Duration::from_secs(30)).is_none() {
                let pid = self.pid();
                unsafe { libc::kill(pid, libc::SIGTERM) };
                if self.wait_for(Duration::from_secs(15)).is_none() {
                    unsafe { libc::kill(pid, libc::SIGKILL) };
                    if self.wait_for(Duration::from_secs(5)).is_none() {
                        eprintln!(
                            "runner probe {pid} did not reap within its bounded failure teardown"
                        );
                    }
                }
            }
            if !self.cleanup_owned() {
                eprintln!(
                    "runner probe failure teardown left an owned process identity running: {:?}",
                    self.owned
                );
            }
        }
    }

    fn launch_probe_with_controls(
        dir: &std::path::Path,
        fault: Option<&str>,
        hold: bool,
        bypass_cleanup_wait: bool,
        leader_first: bool,
    ) -> Probe {
        let script = dir.join("stall-agent.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
if [ "$EVAL_TEST_RUNNER_AGENT_MODE" = leader_first ]; then
  /bin/sh -c 'trap "" TERM; exec sleep 1000' &
  child=$!
  trap 'exit 0' TERM
else
  sleep 1000 &
  child=$!
fi
tmp="$EVAL_TEST_RUNNER_PIDS.tmp.$$"
printf '%s %s\n' "$$" "$child" > "$tmp"
mv "$tmp" "$EVAL_TEST_RUNNER_PIDS"
wait "$child"
"#,
        )
        .unwrap();
        let pids = dir.join("agent-pids");
        let hold_dir = dir.join("cleanup");
        let release = if hold {
            hold_dir.join("release")
        } else {
            dir.join("release")
        };
        if hold {
            std::fs::create_dir_all(&hold_dir).unwrap();
            std::fs::write(hold_dir.join("arm"), b"arm").unwrap();
        }
        let pending = dir.join("cleanup-wait-pending");
        let bin = resolve_swarmail_bin().expect("root Swarmail binary resolves");
        let output = std::fs::File::create(dir.join("probe.stdout")).unwrap();
        let stderr = std::fs::File::create(dir.join("probe.stderr")).unwrap();
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "runner::test_probes::runner_probe_entrypoint",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PROBE_ENV, "1")
        .env(SCRIPT_ENV, &script)
        .env(PIDS_ENV, &pids)
        .env(BIN_ENV, &bin)
        .env(SWARMAIL_BIN_ENV, &bin)
        .env(CLEANUP_WAIT_PENDING_ENV, &pending)
        .env(CLEANUP_BUDGET_HOLD_ENV, dir.join("cleanup-budget"))
        .env(CLEANUP_BUDGET_RELEASE_ENV, dir.join("release"))
        .env(CLEANUP_BUDGET_LOG_ENV, dir.join("cleanup-budgets.log"))
        .env(RUNNER_SIGNAL_LOG_ENV, dir.join("signals.log"))
        .env(
            "EVAL_TEST_RUNNER_EXECUTABLE",
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(env!("CARGO_PKG_NAME")),
        )
        .stdout(output)
        .stderr(stderr);
        match fault {
            Some("observation_error") => {
                cmd.env(FAULT_ENV, "panic_after_agent_readiness")
                    .env(FAULT_RELEASE_ENV, dir.join("fault-release"))
                    .env(CLEANUP_OBSERVE_ERROR_ENV, "1");
            }
            Some("forced_expiry") => {
                cmd.env(FAULT_ENV, "panic_after_agent_readiness")
                    .env(FAULT_RELEASE_ENV, dir.join("fault-release"))
                    .env(CLEANUP_FORCE_EXPIRY_ENV, "1");
            }
            Some(fault) => {
                cmd.env(FAULT_ENV, fault);
            }
            None => {}
        }
        if hold {
            cmd.env(CLEANUP_HOLD_ENV, &hold_dir);
        }
        if bypass_cleanup_wait {
            cmd.env(CLEANUP_WAIT_BYPASS_ENV, "1");
        }
        if leader_first {
            cmd.env("EVAL_TEST_RUNNER_AGENT_MODE", "leader_first");
        }
        let child = cmd.spawn().unwrap();
        let owner = identity(child.id() as i32).expect("runner probe owner identity");
        let mut probe = Probe {
            child: Some(child),
            owner,
            release: release.clone(),
            pending,
            owned: Vec::new(),
        };
        eventually("the runner's agent readiness", || {
            probe.record_owned_from_tree();
            if pids.is_file() {
                return true;
            }
            if let Some(status) = probe.child.as_mut().unwrap().try_wait().unwrap() {
                panic!(
                    "runner probe exited before agent readiness ({status}); stdout={}; stderr={}",
                    std::fs::read_to_string(dir.join("probe.stdout")).unwrap_or_default(),
                    std::fs::read_to_string(dir.join("probe.stderr")).unwrap_or_default()
                );
            }
            false
        });
        let ids: Vec<i32> = std::fs::read_to_string(&pids)
            .unwrap()
            .split_whitespace()
            .map(|p| p.parse().unwrap())
            .collect();
        assert_eq!(ids.len(), 2);
        probe.owned = ids.iter().filter_map(|pid| identity(*pid)).collect();
        assert_eq!(probe.owned.len(), 2);
        probe
    }

    fn wait_for_cleanup_pending(probe: &mut Probe, dir: &std::path::Path) {
        let hold = dir.join("cleanup");
        eventually("runner cleanup hold entry", || {
            std::fs::read_dir(&hold).ok().is_some_and(|entries| {
                entries
                    .flatten()
                    .any(|entry| entry.file_name().to_string_lossy().starts_with("entered-"))
            })
        });
        eventually("the actual cleanup future to return Pending", || {
            probe.pending.is_file()
        });
        assert!(
            probe.owned.iter().any(alive),
            "some owned work remains live at Pending: expected={:?}, actual={:?}",
            probe.owned,
            probe
                .owned
                .iter()
                .map(|id| identity(id.pid))
                .collect::<Vec<_>>()
        );
    }

    fn assert_runner_returned_after_cleanup(probe: &mut Probe) {
        assert!(probe.returned_after_owned_cleanup());
        let leader = probe.owned.first().expect("agent leader identity recorded");
        assert!(
            identity(leader.pid).is_none(),
            "the runner reaped its direct agent child before returning: {leader:?}"
        );
        assert!(
            probe.finish().is_ok(),
            "positive cleanup needs no guard rescue"
        );
    }

    #[test]
    fn runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults() {
        let cases = [
            (None, Some(libc::SIGTERM)),
            (None, Some(libc::SIGINT)),
            (Some("panic"), None),
            (Some("server_death"), None),
        ];
        for (fault, signal) in cases {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = launch_probe_with_controls(dir.path(), fault, true, false, false);
            if let Some(signal) = signal {
                unsafe { libc::kill(probe.pid(), signal) };
            }
            wait_for_cleanup_pending(&mut probe, dir.path());
            assert!(probe.child.as_mut().unwrap().try_wait().unwrap().is_none());
            assert!(probe.owned.iter().all(alive));
            assert!(
                probe.wait_for(Duration::from_millis(50)).is_none(),
                "the runner remains pending while owned cleanup is held"
            );
            probe.release();
            assert!(probe.wait().success());
            assert_runner_returned_after_cleanup(&mut probe);
        }

        for signal in [libc::SIGTERM, libc::SIGINT] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = launch_probe_with_controls(dir.path(), None, true, true, false);
            unsafe { libc::kill(probe.pid(), signal) };
            wait_for_cleanup_pending(&mut probe, dir.path());
            assert!(
                probe.wait_for(Duration::from_secs(2)).is_some(),
                "the negative control bypasses the actual pending cleanup fence"
            );
            assert!(
                !probe.returned_after_owned_cleanup(),
                "the same completion predicate fails while owned work remains live"
            );
            assert!(probe.owned.iter().any(alive));
            assert!(
                probe.finish().is_err(),
                "the completion predicate must reject live owned work"
            );
            probe.release();
            assert!(
                probe.cleanup_owned(),
                "negative-control owned work is torn down"
            );
            assert!(probe.owned.iter().all(|id| !alive(id)));
            assert!(probe.finish().is_ok(), "explicit expected rescue completed");
        }
    }

    #[test]
    fn mixed_signals_after_cleanup_budget_creation_keep_one_owner_and_fixed_budgets() {
        let dir = tempfile::tempdir().unwrap();
        let budget_hold = dir.path().join("cleanup-budget");
        std::fs::create_dir_all(&budget_hold).unwrap();
        std::fs::write(budget_hold.join("arm"), b"arm").unwrap();
        let mut probe = launch_probe_with_controls(dir.path(), Some("panic"), false, false, true);
        probe.record_owned_from_tree();

        let budget_log = dir.path().join("cleanup-budgets.log");
        eventually("one post-creation budget per owned group", || {
            std::fs::read_to_string(&budget_log).is_ok_and(|contents| {
                contents
                    .lines()
                    .filter(|line| line.starts_with("budget:"))
                    .count()
                    >= 3
            })
        });
        let entered = std::fs::read_dir(&budget_hold)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("entered-"))
            .count();
        let budget_ids: Vec<_> = std::fs::read_to_string(&budget_log)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with("budget:"))
            .map(str::to_owned)
            .collect();
        assert!(
            entered >= 3,
            "all cleanup owners reached the post-budget hold: {entered}"
        );
        assert!(budget_ids.len() >= 3);
        let owners: std::collections::HashSet<_> = budget_ids
            .iter()
            .map(|line| line.split(':').nth(1).unwrap())
            .collect();
        assert_eq!(
            owners.len(),
            budget_ids.len(),
            "each group has one cleanup instance"
        );
        assert!(
            probe.pending.is_file(),
            "the awaited cleanup join is actually Pending"
        );
        assert!(
            probe.owned.iter().any(alive),
            "real owned work remains live"
        );
        let fixed_budgets = budget_ids.clone();
        let signal_log = dir.path().join("signals.log");
        for (index, signal) in [libc::SIGTERM, libc::SIGINT, libc::SIGTERM]
            .into_iter()
            .enumerate()
        {
            assert_eq!(unsafe { libc::kill(probe.pid(), signal) }, 0);
            eventually("cleanup signal acknowledgement", || {
                std::fs::read_to_string(&signal_log)
                    .is_ok_and(|contents| contents.lines().count() == index + 1)
            });
            let current = std::fs::read_to_string(&budget_log).unwrap();
            assert_eq!(
                current
                    .lines()
                    .filter(|line| line.starts_with("budget:"))
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                fixed_budgets,
                "signals cannot renew any cleanup deadline or owner"
            );
        }
        assert_eq!(
            std::fs::read_to_string(&signal_log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["term-cleanup", "int-cleanup", "term-cleanup"]
        );
        assert!(probe.child.as_mut().unwrap().try_wait().unwrap().is_none());
        probe.release();
        assert!(probe.wait().success());
        assert_runner_returned_after_cleanup(&mut probe);
        let final_log = std::fs::read_to_string(&budget_log).unwrap();
        assert_eq!(
            final_log
                .lines()
                .filter(|line| line.starts_with("budget:"))
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            fixed_budgets
        );
        assert_eq!(
            final_log
                .lines()
                .filter(|line| line.starts_with("term:"))
                .count(),
            fixed_budgets.len(),
            "each owned group receives one TERM action; log={final_log}"
        );
    }

    #[test]
    fn cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes() {
        for (fault, expected) in [
            ("observation_error", "injected observation error"),
            ("forced_expiry", "cleanup timed out for process group"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = launch_probe_with_controls(dir.path(), Some(fault), false, false, true);
            probe.record_owned_from_tree();
            let direct_agent = probe.owned.first().unwrap().clone();
            std::fs::write(dir.path().join("fault-release"), b"release").unwrap();
            eventually("the actual cleanup join to become Pending", || {
                probe.pending.is_file()
            });
            let status = probe.wait_for(Duration::from_secs(12));
            assert!(
                status.is_some_and(|status| status.success()),
                "cleanup probe did not finish; stdout={}; stderr={}; owned={:?}",
                std::fs::read_to_string(dir.path().join("probe.stdout")).unwrap_or_default(),
                std::fs::read_to_string(dir.path().join("probe.stderr")).unwrap_or_default(),
                probe
                    .owned
                    .iter()
                    .map(|id| (id.pid, identity(id.pid)))
                    .collect::<Vec<_>>()
            );
            assert!(
                identity(direct_agent.pid).is_none(),
                "direct agent child was reaped: {direct_agent:?}"
            );
            assert!(
                probe.owned.iter().any(alive),
                "the test observed a real surviving owned process"
            );
            assert!(
                probe.finish().is_err(),
                "checked teardown refuses the surviving owned process"
            );
            let stderr = std::fs::read_to_string(dir.path().join("probe.stderr")).unwrap();
            assert!(stderr.contains(expected), "{stderr}");
            assert!(
                probe.cleanup_owned(),
                "the test guard safely terminates the retained group"
            );
            assert!(probe.owned.iter().all(|identity| !alive(identity)));
        }
    }

    #[test]
    fn runner_probe_failure_teardown_handles_owner_and_leader_exit() {
        let dir = tempfile::tempdir().unwrap();
        let mut probe = launch_probe_with_controls(dir.path(), None, true, true, true);
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        wait_for_cleanup_pending(&mut probe, dir.path());
        assert!(probe.wait_for(Duration::from_secs(2)).is_some());
        let leader = probe.owned.first().unwrap().clone();
        let descendant = probe.owned.get(1).unwrap().clone();
        eventually(
            "the owner and group leader to exit before the resistant descendant",
            || !alive(&leader) && alive(&descendant),
        );
        assert!(
            probe.finish().is_err(),
            "the guard rejects its live descendant"
        );
        assert!(
            probe.cleanup_owned(),
            "the known group member permits safe cleanup"
        );
        assert!(!alive(&descendant));
        assert!(probe.finish().is_ok(), "explicit owner rescue completed");
    }

    #[test]
    fn trial_pre_handler_control_runs_before_owned_children_exist() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("must-not-start.sh");
        std::fs::write(&script, "#!/bin/sh\ntouch \"$EVAL_TEST_RUNNER_PIDS\"\n").unwrap();
        let hold = dir.path().join("pre-handler");
        std::fs::create_dir_all(&hold).unwrap();
        let bin = resolve_swarmail_bin().expect("root Swarmail binary resolves");
        let stdout = std::fs::File::create(dir.path().join("pre.stdout")).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "runner::test_probes::runner_probe_entrypoint",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(PROBE_ENV, "1")
            .env(SCRIPT_ENV, &script)
            .env(PIDS_ENV, dir.path().join("agent-pids"))
            .env(BIN_ENV, &bin)
            .env(SWARMAIL_BIN_ENV, &bin)
            .env(
                "EVAL_TEST_RUNNER_EXECUTABLE",
                std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join(env!("CARGO_PKG_NAME")),
            )
            .env(PRE_HANDLER_HOLD_ENV, &hold)
            .stdout(stdout)
            .stderr(std::process::Stdio::null());
        let child = child.spawn().unwrap();
        let owner = identity(child.id() as i32).expect("pre-handler probe owner identity");
        let mut probe = Probe {
            child: Some(child),
            owner,
            release: hold.join("release"),
            pending: dir.path().join("pre-handler-pending"),
            owned: Vec::new(),
        };
        eventually("trial pre-handler acknowledgement", || {
            hold.join("entered").is_file()
        });
        assert!(!dir.path().join("agent-pids").exists());
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        let status = probe.wait();
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        assert!(!dir.path().join("agent-pids").exists());
        assert!(probe.finish().is_ok());
    }
}
