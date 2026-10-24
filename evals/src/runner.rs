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
//! output without retaining child bytes, signals the whole group on every handled
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
use std::ffi::{OsStr, OsString};
#[cfg(test)]
use std::future::Future;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::AsyncRead;
use tokio::process::Command;
use tokio::sync::oneshot;

/// The result row's schema.
pub const RESULT_SCHEMA: u32 = 2;
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
const CLEANUP_WAIT_BYPASS_HOLD_ENV: &str = "EVAL_TEST_CLEANUP_WAIT_BYPASS_HOLD_DIR";
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
// Private, per-probe IPC. No descriptor or parent environment is inherited by
// trial descendants; the endpoint is installed by the isolated test entrypoint.
#[cfg(all(test, target_os = "linux"))]
mod cleanup_probe {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::sync::{mpsc, watch};

    pub(super) const ENDPOINT: &str = "EVAL_TEST_CLEANUP_SOCKET";
    pub(super) const DELAY: &str = "EVAL_TEST_CLEANUP_DELAY";
    pub(super) static CURRENT: std::sync::OnceLock<Control> = std::sync::OnceLock::new();

    pub(super) struct Control {
        events: mpsc::UnboundedSender<String>,
        entry: watch::Receiver<bool>,
        release: watch::Receiver<bool>,
        returned: watch::Receiver<bool>,
    }

    impl Control {
        pub(super) async fn connect(path: &Path, delayed: bool) -> Self {
            let stream = tokio::net::UnixStream::connect(path).await.unwrap();
            let (read, mut write) = stream.into_split();
            let (events, mut rx) = mpsc::unbounded_channel::<String>();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    write
                        .write_all(format!("{event}\n").as_bytes())
                        .await
                        .unwrap();
                }
            });
            let (entry_tx, entry) = watch::channel(!delayed);
            let (release_tx, release) = watch::channel(false);
            let (returned_tx, returned) = watch::channel(false);
            tokio::spawn(async move {
                let mut lines = BufReader::new(read).lines();
                while let Some(command) = lines.next_line().await.unwrap() {
                    match command.as_str() {
                        "permit-entry" => {
                            entry_tx.send_replace(true);
                        }
                        "release" => {
                            release_tx.send_replace(true);
                        }
                        "ack-return" => {
                            returned_tx.send_replace(true);
                        }
                        _ => panic!("malformed cleanup probe command: {command}"),
                    }
                }
                // EOF closes the senders, so blocked owners fail visibly.
            });
            Self {
                events,
                entry,
                release,
                returned,
            }
        }

        pub(super) fn event(&self, event: String) {
            self.events
                .send(event)
                .expect("cleanup probe event receiver");
        }

        async fn gate(mut rx: watch::Receiver<bool>) -> Result<(), String> {
            loop {
                if *rx.borrow_and_update() {
                    return Ok(());
                }
                rx.changed()
                    .await
                    .map_err(|_| "cleanup probe controller disconnected".to_string())?;
            }
        }

        pub(super) async fn hold(&self, pid: i32) -> Result<(), String> {
            if !*self.entry.borrow() {
                self.event(format!("pre-entry {pid}"));
            }
            Self::gate(self.entry.clone()).await?;
            // Arm the retained latch before announcing entry. Release also
            // applies to owners arriving after the parent has sent it.
            let release = self.release.clone();
            self.event(format!("held {pid}"));
            Self::gate(release).await
        }

        pub(super) async fn acknowledge_return(&self) {
            self.event("returned".into());
            Self::gate(self.returned.clone()).await.unwrap();
        }
    }

    pub(super) fn pending(kind: &str, pids: &[i32]) {
        if let Some(control) = CURRENT.get() {
            let ids = pids
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            control.event(format!("{kind} {ids}"));
        }
    }
}
/// The built-in agent labels.
pub const BUILTINS: [&str; 2] = ["oracle", "null"];

const STARTUP_DEADLINE: Duration = Duration::from_secs(30);
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
    pub argv: Vec<OsString>,
    pub deadline_ms: Option<u64>,
    pub artifacts_dir: Option<PathBuf>,
}

pub fn new_trial_id() -> String {
    use rand::Rng;
    let n: u32 = rand::rng().random();
    format!("{}-{n:08x}", chrono::Utc::now().format("%Y%m%dT%H%M%S"))
}

/// The caller's path context is captured before the harness clears a child.
#[derive(Clone)]
struct CallerEnv {
    cwd: PathBuf,
    path: Option<OsString>,
}

impl CallerEnv {
    fn capture() -> Result<Self, String> {
        Ok(Self {
            cwd: std::env::current_dir().map_err(|_| "caller_cwd")?,
            path: std::env::var_os("PATH"),
        })
    }

    fn search_path(&self) -> Vec<PathBuf> {
        self.path
            .as_deref()
            .map(std::env::split_paths)
            .unwrap_or_else(|| std::env::split_paths(OsStr::new("/usr/bin:/bin")))
            .map(|part| {
                if part.as_os_str().is_empty() {
                    self.cwd.clone()
                } else if part.is_absolute() {
                    part
                } else {
                    self.cwd.join(part)
                }
            })
            .collect()
    }

    fn absolute_locator(&self, path: &OsStr) -> Result<PathBuf, String> {
        let path = PathBuf::from(path);
        let absolute = if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        };
        absolute
            .is_absolute()
            .then_some(absolute)
            .ok_or_else(|| "path_resolution".to_string())
    }
}

fn executable_file(path: &Path) -> bool {
    path.is_file()
        && std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

/// Resolve one executable using the original caller cwd and PATH. Returned
/// paths are absolute, and no failed candidate is rendered in an error.
fn resolve_program(program: &OsStr, caller: &CallerEnv) -> Result<PathBuf, String> {
    if program.is_empty() {
        return Err("program_missing".into());
    }
    let path = Path::new(program);
    let has_separator = program.as_bytes().contains(&b'/');
    if path.is_absolute() || has_separator {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            caller.cwd.join(path)
        };
        return executable_file(&candidate)
            .then_some(candidate)
            .ok_or_else(|| "program_missing".to_string());
    }
    caller
        .search_path()
        .into_iter()
        .map(|directory| directory.join(program))
        .find(|candidate| executable_file(candidate))
        .ok_or("program_missing".into())
}

/// One child's private runtime directories. The owning TempDir is held by
/// the trial or matrix worker until every child has been drained and reaped.
pub struct RuntimeDirs {
    root: PathBuf,
    home: PathBuf,
    tmp: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    data: PathBuf,
    state: PathBuf,
}

impl RuntimeDirs {
    pub fn create(parent: &Path, role: &str) -> Result<Self, String> {
        let root = parent.join(role);
        std::fs::create_dir(&root).map_err(|_| "private_directory")?;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "private_directory")?;
        let make = |name: &str| -> Result<PathBuf, String> {
            let path = root.join(name);
            std::fs::create_dir(&path).map_err(|_| "private_directory")?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "private_directory")?;
            Ok(path)
        };
        Ok(Self {
            root: root.clone(),
            home: make("home")?,
            tmp: make("tmp")?,
            config: make("config")?,
            cache: make("cache")?,
            data: make("data")?,
            state: make("state")?,
        })
    }

    pub fn apply(&self, cmd: &mut Command) {
        cmd.env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .env("HOME", &self.home)
            .env("TMPDIR", &self.tmp)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_CACHE_HOME", &self.cache)
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_STATE_HOME", &self.state);
    }
}

pub fn private_tempdir(prefix: &str) -> Result<tempfile::TempDir, String> {
    let directory = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in("/tmp")
        .map_err(|_| "private_directory")?;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
        .map_err(|_| "private_directory")?;
    Ok(directory)
}

const CARGO_RECORD_LIMIT: usize = 1024 * 1024;

#[derive(Default)]
struct CargoRecords {
    record: Vec<u8>,
    oversized: bool,
    invalid: bool,
    executable: Option<PathBuf>,
}

impl CargoRecords {
    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            if *byte == b'\n' {
                if self.oversized {
                    self.invalid = true;
                } else if !self.record.is_empty() {
                    match serde_json::from_slice::<Value>(&self.record) {
                        Ok(message) => {
                            if self.executable.is_none()
                                && message["reason"] == "compiler-artifact"
                                && message["target"]["name"] == "swarmail"
                                && message["target"]["kind"]
                                    .as_array()
                                    .is_some_and(|k| k.contains(&json!("bin")))
                                && let Some(path) = message["executable"].as_str()
                            {
                                self.executable = Some(PathBuf::from(path));
                            }
                        }
                        Err(_) => self.invalid = true,
                    }
                }
                self.record.clear();
                self.oversized = false;
            } else if !self.oversized {
                if self.record.len() < CARGO_RECORD_LIMIT {
                    self.record.push(*byte);
                } else {
                    self.record.clear();
                    self.oversized = true;
                }
            }
        }
    }

    fn valid(&self) -> bool {
        !self.invalid && !self.oversized && self.record.is_empty()
    }

    fn finish(self) -> Result<PathBuf, String> {
        if !self.valid() {
            return Err("cargo_output_invalid".into());
        }
        let path = self.executable.ok_or("cargo_artifact_missing")?;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("build_failed")?;
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if !executable_file(&path) {
            return Err("cargo_artifact_invalid".into());
        }
        Ok(path)
    }
}

fn read_cargo_records<R: std::io::Read>(
    mut reader: R,
    executable: Arc<Mutex<Option<PathBuf>>>,
) -> bool {
    let mut records = CargoRecords::default();
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => records.feed(&chunk[..n]),
            Err(_) => return false,
        }
    }
    let valid = records.valid();
    *executable.lock().unwrap_or_else(|e| e.into_inner()) = records.executable;
    valid
}

fn default_home_path(caller: &CallerEnv, home: &OsStr, child: &str) -> Result<PathBuf, String> {
    if home.is_empty() {
        return Err("toolchain_discovery".into());
    }
    Ok(caller.absolute_locator(home)?.join(child))
}

fn prepare_build(
    caller: &CallerEnv,
) -> Result<(std::process::Command, Arc<tempfile::TempDir>), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("build_failed")?;
    let cargo = match std::env::var_os("CARGO") {
        Some(selector) => resolve_program(&selector, caller)?,
        None => resolve_program(OsStr::new("cargo"), caller)?,
    };
    let rustc = resolve_program(OsStr::new("rustc"), caller)?;
    let rustdoc = resolve_program(OsStr::new("rustdoc"), caller)?;
    let cargo_home_selector = std::env::var_os("CARGO_HOME");
    let rustup_home_selector = std::env::var_os("RUSTUP_HOME");
    if cargo_home_selector
        .as_ref()
        .is_some_and(|path| path.is_empty())
        || rustup_home_selector
            .as_ref()
            .is_some_and(|path| path.is_empty())
    {
        return Err("toolchain_discovery".into());
    }
    let original_home = if cargo_home_selector.is_none() || rustup_home_selector.is_none() {
        Some(std::env::var_os("HOME").ok_or("toolchain_discovery")?)
    } else {
        None
    };
    let cargo_home = match cargo_home_selector {
        Some(path) => caller.absolute_locator(&path)?,
        None => default_home_path(
            caller,
            original_home.as_deref().ok_or("toolchain_discovery")?,
            ".cargo",
        )?,
    };
    let rustup_home = match rustup_home_selector {
        Some(path) => caller.absolute_locator(&path)?,
        None => default_home_path(
            caller,
            original_home.as_deref().ok_or("toolchain_discovery")?,
            ".rustup",
        )?,
    };
    let toolchain = std::env::var_os("RUSTUP_TOOLCHAIN");
    if toolchain.as_ref().is_some_and(|v| v.is_empty())
        || std::env::var_os("CARGO_TARGET_DIR")
            .as_ref()
            .is_some_and(|v| v.is_empty())
    {
        return Err("toolchain_discovery".into());
    }
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .filter(|v| !v.is_empty())
        .map(|v| caller.absolute_locator(&v))
        .transpose()?;
    let mut path_dirs = Vec::<PathBuf>::new();
    for program in [&cargo, &rustc, &rustdoc] {
        if let Some(parent) = program.parent()
            && !path_dirs.iter().any(|seen| seen == parent)
        {
            path_dirs.push(parent.to_path_buf());
        }
    }
    path_dirs.extend([PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
    let path = std::env::join_paths(path_dirs).map_err(|_| "toolchain_discovery")?;
    let owner = Arc::new(private_tempdir("swarmail-eval-build-")?);
    let dirs = RuntimeDirs::create(owner.path(), "cargo")?;
    let mut command = std::process::Command::new(cargo);
    command
        .args([
            "build",
            "--offline",
            "--locked",
            "--bin",
            "swarmail",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(root)
        .env_clear()
        .env("PATH", path)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("HOME", &dirs.home)
        .env("TMPDIR", &dirs.tmp)
        .env("XDG_CONFIG_HOME", &dirs.config)
        .env("XDG_CACHE_HOME", &dirs.cache)
        .env("XDG_DATA_HOME", &dirs.data)
        .env("XDG_STATE_HOME", &dirs.state)
        .env("RUSTC", rustc)
        .env("RUSTDOC", rustdoc)
        .env("CARGO_HOME", cargo_home)
        .env("RUSTUP_HOME", rustup_home)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(value) = toolchain {
        command.env("RUSTUP_TOOLCHAIN", value);
    }
    if let Some(value) = target_dir {
        command.env("CARGO_TARGET_DIR", value);
    }
    Ok((command, owner))
}

/// Prepare the exact named build map before any owned spawn. The runtime
/// owner must be retained through the caller's existing cleanup fence.
pub fn prepare_swarmail_build() -> Result<(Command, Arc<tempfile::TempDir>), String> {
    let (command, owner) = prepare_build(&CallerEnv::capture()?)?;
    Ok((Command::from(command), owner))
}

/// Resolve an explicit selector at the original caller boundary, without a build.
pub fn selected_swarmail_bin() -> Result<Option<PathBuf>, String> {
    let caller = CallerEnv::capture()?;
    std::env::var_os(SWARMAIL_BIN_ENV)
        .map(|p| resolve_program(&p, &caller))
        .transpose()
}

fn build_swarmail(caller: &CallerEnv) -> Result<PathBuf, String> {
    let (mut command, _owner) = prepare_build(caller)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("build_failed")?;
    let mut child = command.spawn().map_err(|_| "build_failed")?;
    let executable = Arc::new(Mutex::new(None));
    let stdout = child.stdout.take().ok_or("build_failed")?;
    let stderr = child.stderr.take().ok_or("build_failed")?;
    let found = executable.clone();
    let stdout_thread = std::thread::spawn(move || read_cargo_records(stdout, found));
    let stderr_thread = std::thread::spawn(move || {
        std::io::copy(&mut std::io::BufReader::new(stderr), &mut std::io::sink()).is_ok()
    });
    let status = child.wait().map_err(|_| "build_failed")?;
    let stdout_ok = stdout_thread.join().unwrap_or(false);
    let stderr_ok = stderr_thread.join().unwrap_or(false);
    if !status.success() || !stdout_ok || !stderr_ok {
        return Err("build_failed".into());
    }
    let mut executable = executable
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or("build_failed")?;
    if !executable.is_absolute() {
        executable = root.join(executable);
    }
    if !executable_file(&executable) {
        return Err("build_failed".into());
    }
    Ok(executable)
}

/// Resolve the Swarmail binary: the caller selector is resolved at the
/// original cwd/PATH boundary; otherwise Cargo builds it in a cleared,
/// offline, locked preparation environment and reports its artifact path.
pub fn resolve_swarmail_bin() -> Result<PathBuf, String> {
    let caller = CallerEnv::capture()?;
    if let Some(path) = std::env::var_os(SWARMAIL_BIN_ENV) {
        return resolve_program(&path, &caller);
    }
    build_swarmail(&caller)
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
    _runtime: Option<Arc<tempfile::TempDir>>,
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
                #[cfg(target_os = "linux")]
                let signalled = if let Some(control) = cleanup_probe::CURRENT.get() {
                    control.hold(self.pid).await?;
                    true
                } else {
                    false
                };
                #[cfg(not(target_os = "linux"))]
                let signalled = false;
                if !signalled {
                    while !dir.join("release").exists() {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
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
            #[cfg(all(test, target_os = "linux"))]
            if cleanup_probe::CURRENT.get().is_some() {
                // The bypass may abort its waiter before that waiter is first
                // scheduled. Keep every independently owned hold observable.
                group.ensure_cleanup();
            }
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
                    #[cfg(target_os = "linux")]
                    cleanup_probe::pending("sample-pending", pids);
                    if !pending_recorded {
                        if let Some(path) = std::env::var_os(CLEANUP_WAIT_PENDING_ENV) {
                            std::fs::write(path, b"owned cleanup future returned Pending")
                                .expect("write actual cleanup Pending acknowledgement");
                        }
                        pending_recorded = true;
                    }
                    if std::env::var_os(CLEANUP_WAIT_BYPASS_ENV).is_some() {
                        if let Some(dir) = std::env::var_os(CLEANUP_WAIT_BYPASS_HOLD_ENV) {
                            let dir = PathBuf::from(dir);
                            std::fs::create_dir_all(&dir)
                                .expect("create cleanup wait bypass hold directory");
                            std::fs::write(dir.join("entered"), b"cleanup task retained")
                                .expect("acknowledge cleanup wait bypass hold entry");
                            while !dir.join("release").exists() {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }
                        return Ok(());
                    }
                    let mut recorded = false;
                    std::future::poll_fn(|cx| {
                        let polled = next.as_mut().poll(cx);
                        if polled.is_pending() && !recorded {
                            #[cfg(target_os = "linux")]
                            cleanup_probe::pending("await-pending", pids);
                            recorded = true;
                        }
                        polled
                    })
                    .await
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

const ANNOUNCEMENT_LINE_LIMIT: usize = 8192;

#[derive(Clone, Copy)]
enum ServerRole {
    Swarmail,
    Fixture,
}

enum Announcement {
    Swarmail(SocketAddr, SocketAddr),
    Fixture(SocketAddr, SocketAddr),
}

fn parse_announcement(role: ServerRole, line: &[u8]) -> Option<Announcement> {
    let line = std::str::from_utf8(line).ok()?;
    match role {
        ServerRole::Swarmail => {
            let (smtp, http) = parse_swarmail_addrs(line)?;
            (smtp.ip().is_loopback() && http.ip().is_loopback())
                .then_some(Announcement::Swarmail(smtp, http))
        }
        ServerRole::Fixture => {
            let (public, admin) = parse_fixture_addrs(line)?;
            (public.ip().is_loopback() && admin.ip().is_loopback())
                .then_some(Announcement::Fixture(public, admin))
        }
    }
}

/// Drain with fixed reads. Only a protocol owner can retain one typed,
/// validated announcement; all other child output is discarded immediately.
fn drain<R: AsyncRead + Unpin + Send + 'static>(
    mut reader: R,
    role: Option<ServerRole>,
    announcement: Option<oneshot::Sender<Announcement>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut announcement = announcement;
        let mut chunk = [0u8; 8192];
        let mut line = Vec::with_capacity(ANNOUNCEMENT_LINE_LIMIT);
        let mut oversized = false;
        loop {
            let count = match tokio::io::AsyncReadExt::read(&mut reader, &mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            for byte in &chunk[..count] {
                if announcement.is_none() {
                    continue;
                }
                if *byte == b'\n' {
                    if !oversized
                        && let Some(role) = role
                        && let Some(value) = parse_announcement(role, &line)
                        && let Some(sender) = announcement.take()
                    {
                        let _ = sender.send(value);
                    }
                    line.clear();
                    oversized = false;
                } else if !oversized {
                    if line.len() + 1 < ANNOUNCEMENT_LINE_LIMIT {
                        line.push(*byte);
                    } else {
                        line.clear();
                        oversized = true;
                    }
                }
            }
        }
    })
}

/// A process whose wait task and process-group record outlive the caller.
pub struct Proc {
    pub child: ProcessWait,
    pub pgid: i32,
    pub pid: i32,
    group: Arc<OwnedGroup>,
    announcement: Option<oneshot::Receiver<Announcement>>,
    pub cargo_records: Option<oneshot::Receiver<Result<PathBuf, String>>>,
}

pub struct ProcessWait {
    group: Arc<OwnedGroup>,
}

impl Proc {
    pub async fn cleanup(&self) -> Result<(), String> {
        self.group.await_cleanup().await
    }

    /// Wait for the first valid typed announcement, child exit, or startup
    /// deadline. Raw lines never cross this boundary.
    async fn announce(&mut self) -> Result<Announcement, String> {
        let deadline = tokio::time::Instant::now() + STARTUP_DEADLINE;
        let receiver = self.announcement.take().ok_or("startup_protocol")?;
        tokio::select! {
            value = receiver => value.map_err(|_| "startup_protocol".into()),
            _ = tokio::time::sleep_until(deadline) => Err("startup_timeout".into()),
            _ = self.child.wait() => Err("startup_exit".into()),
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

pub fn spawn_owned_cargo(
    cmd: Command,
    groups: &Groups,
    runtime: Arc<tempfile::TempDir>,
) -> std::io::Result<Proc> {
    spawn(cmd, groups, None, Some(runtime), true)
}

fn spawn(
    mut cmd: Command,
    groups: &Groups,
    role: Option<ServerRole>,
    runtime: Option<Arc<tempfile::TempDir>>,
    cargo: bool,
) -> std::io::Result<Proc> {
    cmd.process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| std::io::Error::other("spawn_pid_missing"))? as i32;
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
        _runtime: runtime.clone(),
    });
    groups.add(group.clone());
    tokio::spawn(async move {
        let _runtime = runtime;
        let result = child.wait().await.map_err(|_| "process_wait_failed".into());
        status_tx.send_replace(Some(result));
    });
    let (sender, announcement) = if role.is_some() {
        let (tx, rx) = oneshot::channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    let mut drains = group.drains.lock().unwrap_or_else(|e| e.into_inner());
    let cargo_records = if cargo {
        let (tx, rx) = oneshot::channel();
        drains.push(tokio::spawn(async move {
            let mut reader = stdout;
            let mut records = CargoRecords::default();
            let mut chunk = [0u8; 8192];
            let result = loop {
                match tokio::io::AsyncReadExt::read(&mut reader, &mut chunk).await {
                    Ok(0) => break records.finish(),
                    Ok(n) => records.feed(&chunk[..n]),
                    Err(_) => break Err("cargo_output_read_failed".into()),
                }
            };
            let _ = tx.send(result);
        }));
        Some(rx)
    } else {
        drains.push(drain(stdout, role, sender));
        None
    };
    drains.push(drain(stderr, None, None));
    drop(drains);
    Ok(Proc {
        child: ProcessWait {
            group: group.clone(),
        },
        pgid: pid,
        pid,
        group,
        announcement,
        cargo_records,
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
    let pop3: SocketAddr = field("pop3=")?;
    let http = field("http=")?;
    (smtp.ip().is_loopback() && pop3.ip().is_loopback() && http.ip().is_loopback())
        .then_some((smtp, http))
}

pub fn parse_fixture_addrs(line: &str) -> Option<(SocketAddr, SocketAddr)> {
    let rest = line.trim().strip_prefix("fixture ")?;
    let field = |k: &str| -> Option<SocketAddr> {
        rest.split(k)
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    let public = field("public=")?;
    let admin = field("admin=")?;
    (public.ip().is_loopback() && admin.ip().is_loopback()).then_some((public, admin))
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
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
        let resp = req.send().await.map_err(|_| "admin_transport")?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|_| "admin_response_read")?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| "admin_response_invalid")?;
        if status != 200 {
            return Err("admin_response_status".into());
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
    runtime: Arc<tempfile::TempDir>,
    servers: Vec<Proc>,
    agents: Arc<Mutex<Vec<(String, i32)>>>,
    diagnostics: Vec<String>,
}

impl Trial {
    async fn stop_servers(&mut self) -> Result<(), String> {
        let pids: Vec<_> = self.servers.iter().map(|p| p.pid).collect();
        self.groups.cleanup_pids(&pids).await?;
        self.servers.clear();
        Ok(())
    }
}

/// What the agents phase ended with.
enum AgentsEnd {
    Done,
    Deadline,
    ServerDied,
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
    std::env::current_exe().map_err(|_| "program_missing".into())
}

/// Resolve the executable once at the caller boundary; keep every later
/// argument as its original literal OS string.
pub fn resolve_agent_command(argv: &[String]) -> Result<Vec<OsString>, String> {
    let caller = CallerEnv::capture()?;
    let program = argv.first().ok_or("program_missing")?;
    let resolved = resolve_program(OsStr::new(program), &caller)?;
    let mut command = Vec::with_capacity(argv.len());
    command.push(resolved.into_os_string());
    command.extend(argv[1..].iter().map(OsString::from));
    Ok(command)
}

fn agent_argv(label: &str, argv: &[OsString], caller: &CallerEnv) -> Result<Vec<OsString>, String> {
    if BUILTINS.contains(&label) {
        let exe = evals_executable()?;
        Ok(vec![
            exe.into_os_string(),
            OsString::from("agent"),
            OsString::from(label),
        ])
    } else {
        let program = resolve_program(&argv[0], caller)?;
        let mut resolved = Vec::with_capacity(argv.len());
        resolved.push(program.into_os_string());
        resolved.extend(argv[1..].iter().cloned());
        Ok(resolved)
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
    argv: &[OsString],
    trial_id: &str,
    swarmail_bin: &Path,
    scratch: &Path,
    trial: &mut Trial,
    exits: &mut Vec<AgentExit>,
    snapshot_out: &mut Option<Snapshot>,
) -> Result<bool, String> {
    // Every role gets a closed runtime map and a private owner under /tmp.
    let swarmail_dirs = RuntimeDirs::create(scratch, "swarmail")?;
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
    swarmail_dirs.apply(&mut cmd);
    cmd.env("RUST_LOG", "swarmail=info,tower_http=off")
        .current_dir(&swarmail_dirs.root);
    let mut swarmail = spawn(
        cmd,
        &trial.groups,
        Some(ServerRole::Swarmail),
        Some(trial.runtime.clone()),
        false,
    )
    .map_err(|_| "swarmail_spawn")?;
    let announced = swarmail.announce().await;
    trial.servers.push(swarmail);
    let (smtp, http) = match announced? {
        Announcement::Swarmail(smtp, http) => (smtp, http),
        Announcement::Fixture(_, _) => return Err("startup_protocol".into()),
    };
    let client = http_client();
    let health = client
        .get(format!("http://{http}/healthz"))
        .send()
        .await
        .map_err(|_| "swarmail_health")?;
    if !health.status().is_success() {
        return Err("swarmail_health".into());
    }
    mail::probe_greeting(smtp)
        .await
        .map_err(|_| "swarmail_smtp")?;
    let mcp_url = format!("http://{http}/mcp");

    // The fixture receives the generated admin secret in its environment alone.
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
    let fixture_dirs = RuntimeDirs::create(scratch, "fixture")?;
    let mut cmd = Command::new(&exe);
    cmd.args([
        "fixture",
        "--smtp",
        &smtp.to_string(),
        "--task",
        &task.id,
        "--trial-id",
        trial_id,
    ]);
    fixture_dirs.apply(&mut cmd);
    cmd.env(ADMIN_SECRET_ENV, &secret)
        .current_dir(&fixture_dirs.root);
    let mut fixture = spawn(
        cmd,
        &trial.groups,
        Some(ServerRole::Fixture),
        Some(trial.runtime.clone()),
        false,
    )
    .map_err(|_| "fixture_spawn")?;
    let announced = fixture.announce().await;
    trial.servers.push(fixture);
    let (public, admin_addr) = match announced? {
        Announcement::Fixture(public, admin) => (public, admin),
        Announcement::Swarmail(_, _) => return Err("startup_protocol".into()),
    };
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
    let caller = CallerEnv::capture()?;
    let argv = match agent_argv(label, argv, &caller) {
        Ok(argv) => argv,
        Err(_) => {
            *exits = ids.iter().map(|id| exit_of(&id.identity, None)).collect();
            return Err("program_missing".into());
        }
    };
    let mut waits = tokio::task::JoinSet::new();
    let mut agent_groups = Vec::new();
    let mut spawn_failed = false;
    for (index, id) in ids.iter().enumerate() {
        let vars = PromptVars {
            app_url: &app_url,
            mcp_url: &mcp_url,
            identity: &id.identity,
            inbox: &id.inbox,
        };
        let prompt = tasks::render_prompt(task, &vars).map_err(|_| "prompt_render")?;
        let dirs = RuntimeDirs::create(scratch, &format!("agent-{index}"))?;
        let workdir = dirs.root.join("work");
        std::fs::create_dir(&workdir).map_err(|_| "private_directory")?;
        std::fs::set_permissions(&workdir, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "private_directory")?;
        let prompt_file = workdir.join("prompt.md");
        std::fs::write(&prompt_file, &prompt).map_err(|_| "private_file")?;
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..]).current_dir(&workdir);
        dirs.apply(&mut cmd);
        cmd.env(ENV_TASK, &task.id)
            .env(ENV_APP_URL, &app_url)
            .env(ENV_MCP_URL, &mcp_url)
            .env(ENV_IDENTITY, &id.identity)
            .env(ENV_INBOX, &id.inbox)
            .env(ENV_GOAL, &prompt)
            .env(ENV_PROMPT_FILE, &prompt_file)
            .env(ENV_DEADLINE_MS, task.deadline_ms.to_string())
            .env("PWD", &workdir);
        match spawn(cmd, &trial.groups, None, Some(trial.runtime.clone()), false) {
            Ok(Proc {
                mut child,
                pgid,
                pid,
                ..
            }) => {
                agent_groups.push(pgid);
                trial
                    .agents
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push((id.identity.clone(), pid));
                waits.spawn(async move { (index, child.wait().await.ok()) });
            }
            Err(_) => {
                spawn_failed = true;
                break;
            }
        }
    }
    let mut statuses: Vec<Option<ExitStatus>> = vec![None; ids.len()];
    let end = if spawn_failed {
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
                _ = swarmail.wait() => break AgentsEnd::ServerDied,
                _ = fixture.wait() => break AgentsEnd::ServerDied,
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
    if spawn_failed {
        return Err("agent_spawn".into());
    }
    cleanup_result?;
    match end {
        AgentsEnd::ServerDied => return Err("server_died".into()),
        AgentsEnd::Done | AgentsEnd::Deadline => {}
    }

    // Freeze, then take one consistent snapshot before the servers stop.
    admin.call(true, "/admin/freeze", None).await?;
    let snap = admin.call(false, "/admin/snapshot", None).await?;
    let snap: Snapshot = serde_json::from_value(snap).map_err(|_| "snapshot_invalid")?;
    *snapshot_out = Some(snap);
    Ok(matches!(end, AgentsEnd::Deadline))
}

const ARTIFACT_DIRECTORY_MODE: u32 = 0o700;
const ARTIFACT_FILE_MODE: u32 = 0o600;

/// Make an exclusively owned trial directory under the caller-selected root.
/// Existing roots and ancestors are caller-owned and keep their permissions.
fn create_private_artifact_dir(root: &Path, trial_id: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|_| "artifact_write")?;
    let out = root.join(trial_id);
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(ARTIFACT_DIRECTORY_MODE);
    builder.create(&out).map_err(|_| "artifact_write")?;

    // mkdir applies umask; this chmod applies only to our exclusive empty leaf.
    // It precedes all file creation, and any failed chmod leaves no payload.
    if std::fs::set_permissions(
        &out,
        std::fs::Permissions::from_mode(ARTIFACT_DIRECTORY_MODE),
    )
    .is_err()
    {
        let _ = std::fs::remove_dir(&out);
        return Err("artifact_write".into());
    }
    Ok(out)
}

fn create_private_artifact_temp_with(
    out: &Path,
    builder: &tempfile::Builder<'_, '_>,
) -> io::Result<tempfile::NamedTempFile> {
    let tmp = builder.tempfile_in(out)?;
    // tempfile creates exclusively with mode 0600 on Unix. Umask may make it
    // stricter, so normalize through the open descriptor before any payload.
    tmp.as_file()
        .set_permissions(std::fs::Permissions::from_mode(ARTIFACT_FILE_MODE))?;
    Ok(tmp)
}

fn create_private_artifact_temp(out: &Path) -> io::Result<tempfile::NamedTempFile> {
    create_private_artifact_temp_with(out, &tempfile::Builder::new())
}

fn write_artifact_with(
    out: &Path,
    name: &str,
    bytes: &[u8],
    write_payload: impl FnOnce(&mut std::fs::File, &[u8]) -> io::Result<()>,
) -> Result<(), String> {
    let mut tmp = create_private_artifact_temp(out).map_err(|_| "artifact_write")?;
    write_payload(tmp.as_file_mut(), bytes).map_err(|_| "artifact_write")?;

    // tempfile 3.27.0 uses no-replace rename on Linux/macOS; its supported
    // fallback links then unlinks and can leave this private temp name behind.
    // In either case the complete payload is private before the final name
    // becomes visible, and an existing final name is never replaced.
    tmp.persist_noclobber(out.join(name)).map_err(|error| {
        drop(error);
        "artifact_write".to_string()
    })?;
    Ok(())
}

fn write_artifacts(
    dir: &Path,
    row: &TrialResult,
    snap: Option<&Snapshot>,
    diagnostics: &[String],
) -> Result<(), String> {
    let out = create_private_artifact_dir(dir, &row.trial_id)?;
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        write_artifact_with(&out, name, bytes, |file, payload| file.write_all(payload))
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
    let safe_diagnostics = diagnostics.join("\n");
    let safe_diagnostics = if safe_diagnostics.len() > 64 * 1024 {
        "diagnostics truncated\n"
    } else {
        &safe_diagnostics
    };
    write("diagnostics.log", safe_diagnostics.as_bytes())?;
    let meta = json!({
        "harness": env!("CARGO_PKG_NAME"),
        "harness_version": env!("CARGO_PKG_VERSION"),
        "swarmail_bin": Value::Null,
        "confinement": CONFINEMENT,
    });
    write(
        "metadata.json",
        &serde_json::to_vec_pretty(&meta).expect("serializes"),
    )
}

pub fn safe_cleanup_category(error: &str) -> &'static str {
    if error.starts_with("cleanup timed out") {
        "cleanup_expired"
    } else if error.starts_with("cannot inspect")
        || error.starts_with("/proc/")
        || error.starts_with("cannot verify")
        || error.starts_with("process-group leader")
    {
        "cleanup_observation_failed"
    } else if error.starts_with("cannot reap") {
        "cleanup_wait_failed"
    } else if error.starts_with("output pipe") {
        "cleanup_output_expired"
    } else if error.starts_with("cannot signal") {
        "cleanup_signal_failed"
    } else {
        "operation_failed"
    }
}

fn safe_trial_category(error: &str) -> &'static str {
    match error {
        "private_directory" => "private_directory_unavailable",
        "private_file" => "private_file_unavailable",
        "swarmail_spawn" => "swarmail_spawn_failed",
        "startup_exit" => "service_exited_before_ready",
        "startup_timeout" => "service_readiness_timeout",
        "startup_protocol" => "service_announcement_invalid",
        "swarmail_health" => "swarmail_health_failed",
        "swarmail_smtp" => "swarmail_smtp_failed",
        "fixture_spawn" => "fixture_spawn_failed",
        "fixture_index" => "fixture_health_failed",
        "program_missing" => "program_unavailable",
        "agent_spawn" => "agent_spawn_failed",
        "server_died" => "service_exited_during_trial",
        "snapshot_invalid" => "snapshot_invalid",
        "artifact_write" => "artifact_write_failed",
        "admin_transport" => "admin_transport_failed",
        "admin_response_read" => "admin_response_read_failed",
        "admin_response_invalid" => "admin_response_invalid",
        "admin_response_status" => "admin_response_rejected",
        _ => safe_cleanup_category(error),
    }
}

/// Run one trial to its one result row. Diagnostics go to stderr.
pub async fn run_trial(req: TrialRequest, swarmail_bin: Option<PathBuf>) -> TrialResult {
    let trial_id = new_trial_id();
    let (task, label) = match validate_request(&req) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("swarmail-evals: harness error: request_invalid");
            return refused_row(&req, &trial_id);
        }
    };
    let caller = match CallerEnv::capture() {
        Ok(caller) => caller,
        Err(_) => {
            eprintln!("swarmail-evals: harness error: caller_context_unavailable");
            return refused_row(&req, &trial_id);
        }
    };
    let bin = match swarmail_bin {
        Some(path) => resolve_program(path.as_os_str(), &caller),
        None => resolve_swarmail_bin(),
    };
    let bin = match bin {
        Ok(b) => b,
        Err(_) => {
            eprintln!("swarmail-evals: harness error: swarmail_unavailable");
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
            eprintln!("swarmail-evals: harness error: signal_handlers_unavailable");
            return refused_row(&req, &trial_id);
        }
    };
    let scratch = match private_tempdir("swarmail-eval-") {
        Ok(d) => Arc::new(d),
        Err(_) => {
            eprintln!("swarmail-evals: harness error: private_directory_unavailable");
            return refused_row(&req, &trial_id);
        }
    };
    let mut body = {
        let (task, label, argv, trial_id, bin, scratch_owner, groups, agents) = (
            task.clone(),
            label.clone(),
            req.argv.clone(),
            trial_id.clone(),
            bin.clone(),
            scratch.clone(),
            groups.clone(),
            agents.clone(),
        );
        tokio::spawn(async move {
            let mut trial = Trial {
                groups,
                runtime: scratch_owner.clone(),
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
                scratch_owner.path(),
                &mut trial,
                &mut exits,
                &mut snap,
            )
            .await;
            let outcome = match trial.stop_servers().await {
                Ok(()) => outcome,
                Err(e) => Err(e),
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
        #[cfg(test)]
        if let Some(dir) = std::env::var_os(CLEANUP_WAIT_BYPASS_HOLD_ENV) {
            std::fs::write(
                PathBuf::from(dir).join("returned"),
                b"cleanup bypass returned",
            )
            .expect("acknowledge cleanup wait bypass return");
        }
        if let Err(e) = cleanup_result {
            eprintln!(
                "swarmail-evals: interrupted trial cleanup failed: {}",
                safe_cleanup_category(&e)
            );
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
        Ok((outcome, exits, snap, mut diagnostics)) => {
            // Precedence (0055 §2): infrastructure — a process error, or a
            // snapshot that is absent or unscorable (a failed or parked
            // fixture delivery included) — then the deadline, then the
            // business check.
            let verdict = match (&outcome, &snap) {
                (Err(error), _) => {
                    let category = safe_trial_category(error);
                    eprintln!("swarmail-evals: harness error: {category}");
                    diagnostics.push(format!("stage=trial category={category}"));
                    Verdict::harness_error()
                }
                (Ok(_), None) => {
                    eprintln!("swarmail-evals: harness error: snapshot_missing");
                    diagnostics.push("stage=snapshot category=missing".into());
                    Verdict::harness_error()
                }
                (Ok(deadline), Some(snap)) => match tasks::validate_snapshot(&task, snap) {
                    Err(_) => {
                        eprintln!("swarmail-evals: harness error: snapshot_unscorable");
                        diagnostics.push("stage=scoring category=snapshot_invalid".into());
                        Verdict::harness_error()
                    }
                    Ok(()) if *deadline => Verdict::fail(FailureClass::Budget),
                    Ok(()) => match tasks::score(&task, snap) {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("swarmail-evals: harness error: scoring_failed");
                            diagnostics.push("stage=scoring category=failed".into());
                            Verdict::harness_error()
                        }
                    },
                },
            };
            (verdict, exits, snap, diagnostics)
        }
        Err(_) => {
            // A panic: the body's handles are gone, so its groups are
            // killed through the shared registry.
            eprintln!("swarmail-evals: harness error: runner_panicked");
            (
                Verdict::harness_error(),
                Vec::new(),
                None,
                vec!["stage=runner category=panicked".into()],
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
        diagnostics.push(format!(
            "owned process cleanup failed: {}",
            safe_cleanup_category(&e)
        ));
        eprintln!(
            "swarmail-evals: harness error: owned process cleanup failed: {}",
            safe_cleanup_category(&e)
        );
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
        && write_artifacts(dir, &row, snap.as_ref(), &diagnostics).is_err()
    {
        diagnostics.push("stage=artifact category=write_failed".into());
        eprintln!("swarmail-evals: harness error: artifact_write_failed");
    }
    if row.outcome == Outcome::HarnessError {
        for d in &diagnostics {
            eprintln!("swarmail-evals: {d}");
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
            argv: vec!["/bin/sh".into(), script.clone()],
            deadline_ms: Some(60_000),
            artifacts_dir: None,
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let row = runtime.block_on(async {
            if let Some(path) = std::env::var_os(cleanup_probe::ENDPOINT) {
                let control = cleanup_probe::Control::connect(
                    Path::new(&path),
                    std::env::var_os(cleanup_probe::DELAY).is_some(),
                )
                .await;
                assert!(cleanup_probe::CURRENT.set(control).is_ok());
            }
            let row = run_trial(req, Some(PathBuf::from(bin))).await;
            if let Some(control) = cleanup_probe::CURRENT.get() {
                // Publish actual return before retaining the runtime for the
                // parent's live-work observation. This is not a cleanup fence.
                control.acknowledge_return().await;
            }
            row
        });
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
            control: None,
            returned: false,
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

    #[derive(Debug, PartialEq)]
    enum CleanupEvent {
        PreEntry(i32),
        Held(i32),
        SamplePending(Vec<i32>),
        AwaitPending(Vec<i32>),
        Returned,
    }

    impl CleanupEvent {
        fn parse(line: &str) -> Result<Self, String> {
            let fields: Vec<_> = line.split_whitespace().collect();
            let ids = |text: &str| {
                text.split(',')
                    .map(|id| id.parse::<i32>().map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, _>>()
            };
            match fields.as_slice() {
                ["pre-entry", pid] => Ok(Self::PreEntry(
                    pid.parse::<i32>().map_err(|e| e.to_string())?,
                )),
                ["held", pid] => Ok(Self::Held(pid.parse::<i32>().map_err(|e| e.to_string())?)),
                ["sample-pending", pids] => Ok(Self::SamplePending(ids(pids)?)),
                ["await-pending", pids] => Ok(Self::AwaitPending(ids(pids)?)),
                ["returned"] => Ok(Self::Returned),
                _ => Err(format!("malformed cleanup probe event: {line}")),
            }
        }
    }

    struct CleanupControl {
        stream: std::os::unix::net::UnixStream,
        events: std::sync::mpsc::Receiver<Result<CleanupEvent, String>>,
        reader: Option<std::thread::JoinHandle<()>>,
    }

    impl CleanupControl {
        fn new(stream: std::os::unix::net::UnixStream) -> Self {
            use std::io::BufRead;
            let read = stream.try_clone().unwrap();
            let (tx, events) = std::sync::mpsc::channel();
            let reader = std::thread::spawn(move || {
                for line in std::io::BufReader::new(read).lines() {
                    let event = line
                        .map_err(|e| e.to_string())
                        .and_then(|line| CleanupEvent::parse(&line));
                    if tx.send(event).is_err() {
                        break;
                    }
                }
            });
            Self {
                stream,
                events,
                reader: Some(reader),
            }
        }

        fn command(&mut self, command: &str) {
            writeln!(self.stream, "{command}").expect("cleanup probe command");
        }

        fn recv(&self) -> CleanupEvent {
            self.events
                .recv_timeout(Duration::from_secs(120))
                .expect("cleanup probe hang watchdog or unexpected EOF (not an ordering verdict)")
                .expect("valid cleanup probe event")
        }

        fn release_all(&mut self) {
            // Teardown tolerates EOF and unblocks every gate before rescue.
            let _ = self
                .stream
                .write_all(b"permit-entry\nrelease\nack-return\n");
        }
    }

    impl Drop for CleanupControl {
        fn drop(&mut self) {
            self.release_all();
            let _ = self.stream.shutdown(std::net::Shutdown::Both);
            self.reader.take().unwrap().join().unwrap();
        }
    }

    struct Probe {
        child: Option<std::process::Child>,
        owner: ProcessIdentity,
        release: PathBuf,
        pending: PathBuf,
        control: Option<CleanupControl>,
        returned: bool,
        owned: Vec<ProcessIdentity>,
    }

    impl Probe {
        fn pid(&self) -> i32 {
            self.child.as_ref().unwrap().id() as i32
        }

        fn release(&mut self) {
            let _ = std::fs::write(&self.release, b"release");
            if let Some(control) = &mut self.control {
                control.command("release");
            }
        }

        fn event(&mut self) -> CleanupEvent {
            let event = self.control.as_ref().unwrap().recv();
            if event == CleanupEvent::Returned {
                self.returned = true;
            }
            event
        }

        fn await_return(&mut self) {
            while !self.returned {
                self.event();
            }
        }

        fn acknowledge_return(&mut self) {
            self.control.as_mut().unwrap().command("ack-return");
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
            let returned = if self.control.is_some() {
                self.returned
            } else {
                self.child
                    .as_mut()
                    .is_some_and(|child| child.try_wait().ok().flatten().is_some())
            };
            returned && self.owned.iter().all(|id| !alive(id))
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
            if let Some(control) = &mut self.control {
                control.release_all();
            }
            // Preserve marker release even if the IPC peer has failed.
            let _ = std::fs::write(&self.release, b"release");
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
        hold_bypass_return: bool,
    ) -> Probe {
        launch_probe(
            dir,
            fault,
            hold,
            bypass_cleanup_wait,
            leader_first,
            hold_bypass_return,
            None,
        )
    }

    fn launch_probe(
        dir: &Path,
        fault: Option<&str>,
        hold: bool,
        bypass_cleanup_wait: bool,
        leader_first: bool,
        hold_bypass_return: bool,
        handshake: Option<bool>,
    ) -> Probe {
        let listener = handshake
            .map(|_| std::os::unix::net::UnixListener::bind(dir.join("control.sock")).unwrap());
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
        if let Some(delayed) = handshake {
            cmd.env(cleanup_probe::ENDPOINT, dir.join("control.sock"));
            if delayed {
                cmd.env(cleanup_probe::DELAY, "1");
            }
        }
        if hold {
            cmd.env(CLEANUP_HOLD_ENV, &hold_dir);
        }
        if bypass_cleanup_wait {
            cmd.env(CLEANUP_WAIT_BYPASS_ENV, "1");
        }
        if hold_bypass_return {
            assert!(bypass_cleanup_wait, "the return hold requires bypass mode");
            cmd.env(CLEANUP_WAIT_BYPASS_HOLD_ENV, dir.join("cleanup-bypass"));
        }
        if leader_first {
            cmd.env("EVAL_TEST_RUNNER_AGENT_MODE", "leader_first");
        }
        let mut script_text = std::fs::read_to_string(&script).unwrap();
        let mut setup = format!(
            "#!/bin/sh\n/usr/bin/env -0 > '{}'\n",
            dir.join("runtime-env").display()
        );
        for (key, value) in cmd.get_envs() {
            if key.to_string_lossy().starts_with("EVAL_TEST_RUNNER_")
                && let Some(value) = value
            {
                setup.push_str(&format!(
                    "export {}='{}'\n",
                    key.to_str().unwrap(),
                    value.to_str().unwrap().replace('\'', "'\\''")
                ));
            }
        }
        script_text = script_text.replacen("#!/bin/sh\n", &setup, 1);
        std::fs::write(&script, script_text).unwrap();
        let child = cmd.spawn().unwrap();
        let owner = identity(child.id() as i32).expect("runner probe owner identity");
        let mut probe = Probe {
            child: Some(child),
            owner,
            release: release.clone(),
            pending,
            control: None,
            returned: false,
            owned: Vec::new(),
        };
        if let Some(listener) = listener {
            // Probe already owns the child, so assertion failure runs rescue.
            use std::os::fd::AsRawFd;
            let mut fd = libc::pollfd {
                fd: listener.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: fd names this live listener. This bound only diagnoses
            // a child that never connects; it is not a cleanup-order oracle.
            assert!(
                unsafe { libc::poll(&mut fd, 1, 120_000) } > 0,
                "cleanup probe connection hang watchdog"
            );
            probe.control = Some(CleanupControl::new(listener.accept().unwrap().0));
        }
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
    fn cleanup_handshake_retains_events_and_release_for_multiple_and_late_owners() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let child = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let control = cleanup_probe::Control::connect(&path, true).await;
                    let (first, second) = tokio::join!(control.hold(11), control.hold(22));
                    first.unwrap();
                    second.unwrap();
                    // The parent has already released the latch. This late
                    // owner must not need another permission or release.
                    control.hold(33).await.unwrap();
                    control.acknowledge_return().await;
                });
        });
        let mut control = CleanupControl::new(listener.accept().unwrap().0);
        let mut checkpoints = Vec::new();
        for _ in 0..2 {
            let CleanupEvent::PreEntry(pid) = control.recv() else {
                panic!("entry checkpoint");
            };
            checkpoints.push(pid);
        }
        checkpoints.sort();
        assert_eq!(checkpoints, [11, 22]);
        control.command("permit-entry");
        let mut held = Vec::new();
        for _ in 0..2 {
            let CleanupEvent::Held(pid) = control.recv() else {
                panic!("held owner");
            };
            held.push(pid);
        }
        held.sort();
        assert_eq!(held, [11, 22]);
        control.command("release");
        // Both events may be queued before these receives; the channel retains them.
        assert_eq!(control.recv(), CleanupEvent::Held(33));
        assert_eq!(control.recv(), CleanupEvent::Returned);
        control.command("ack-return");
        child.join().unwrap();
        assert!(
            control
                .events
                .recv_timeout(Duration::from_secs(120))
                .is_err()
        );
    }

    #[test]
    fn cleanup_handshake_disconnect_and_malformed_events_fail_visibly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let child = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let control = cleanup_probe::Control::connect(&path, true).await;
                    assert_eq!(
                        control.hold(11).await.unwrap_err(),
                        "cleanup probe controller disconnected"
                    );
                });
        });
        let mut control = CleanupControl::new(listener.accept().unwrap().0);
        assert_eq!(control.recv(), CleanupEvent::PreEntry(11));
        // Close without granting entry; the owner gets an error, not a hang.
        control.stream.shutdown(std::net::Shutdown::Both).unwrap();
        child.join().unwrap();
        control.release_all();

        let (read, mut write) = std::os::unix::net::UnixStream::pair().unwrap();
        let control = CleanupControl::new(read);
        for line in ["nonsense", "held invalid", "sample-pending 1,no", ""] {
            writeln!(write, "{line}").unwrap();
            assert!(
                control
                    .events
                    .recv_timeout(Duration::from_secs(120))
                    .unwrap()
                    .is_err()
            );
        }
        drop(write);
        assert!(
            control
                .events
                .recv_timeout(Duration::from_secs(120))
                .is_err()
        );
    }

    #[test]
    fn cleanup_handshake_failure_teardown_releases_all_gates_and_reaps() {
        let dir = tempfile::tempdir().unwrap();
        let mut identities = Vec::new();
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut probe = launch_probe(dir.path(), None, true, false, false, false, Some(true));
            identities.push(probe.owner.clone());
            identities.extend(probe.owned.clone());
            unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
            loop {
                match probe.event() {
                    CleanupEvent::PreEntry(_) => break,
                    CleanupEvent::Returned => panic!("unexpected early runner return"),
                    _ => {}
                }
            }
            panic!("intentional failure with entry, cleanup and return gates closed");
        }));
        let failure = failed.expect_err("the injected failure must execute");
        assert_eq!(
            failure.downcast_ref::<&str>(),
            Some(&"intentional failure with entry, cleanup and return gates closed")
        );
        assert!(
            identities.iter().all(|id| !alive(id)),
            "failure teardown stops owned identities"
        );
        assert!(
            identity(identities[0].pid).is_none(),
            "failure teardown reaps the probe"
        );
        let stderr = std::fs::read_to_string(dir.path().join("probe.stderr")).unwrap();
        assert!(!stderr.contains("controller disconnected"), "{stderr}");
        let stdout = std::fs::read_to_string(dir.path().join("probe.stdout")).unwrap();
        assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
    }

    fn causal_cleanup_pending(probe: &mut Probe, delayed: bool, bypass: bool) {
        let leader = probe.owned[0].pid;
        let (mut held, mut sampled, mut awaited, mut permitted) = (false, false, false, !delayed);
        while !(held && sampled && (bypass || awaited)) {
            match probe.event() {
                CleanupEvent::PreEntry(pid) => {
                    assert!(delayed);
                    eprintln!("pre-entry({pid}); permit-entry");
                    probe.control.as_mut().unwrap().command("permit-entry");
                    permitted = true;
                }
                CleanupEvent::Held(pid) if pid == leader => {
                    assert!(permitted, "hold entry requires explicit permission");
                    held = true;
                }
                CleanupEvent::SamplePending(pids) if pids.contains(&leader) => sampled = true,
                CleanupEvent::AwaitPending(pids) if pids.contains(&leader) => awaited = true,
                CleanupEvent::Returned => assert!(
                    bypass,
                    "runner returned before the awaited cleanup fence: live owned work"
                ),
                _ => {}
            }
        }
        assert!(
            probe.owned.iter().all(alive),
            "owned work is live at the held cleanup fence"
        );
        eprintln!("held({leader}); sample-pending; await-pending={awaited}");
    }

    #[test]
    fn runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults() {
        for delayed in [false, true] {
            let cases = [
                (None, Some(libc::SIGTERM)),
                (None, Some(libc::SIGINT)),
                (Some("panic"), None),
                (Some("server_death"), None),
            ];
            for (fault, signal) in cases {
                eprintln!("positive delayed={delayed} fault={fault:?} signal={signal:?}");
                let dir = tempfile::tempdir().unwrap();
                let mut probe =
                    launch_probe(dir.path(), fault, true, false, false, false, Some(delayed));
                if let Some(signal) = signal {
                    unsafe { libc::kill(probe.pid(), signal) };
                }
                causal_cleanup_pending(&mut probe, delayed, false);
                probe.release();
                probe.await_return();
                assert!(
                    probe.returned_after_owned_cleanup(),
                    "runner return requires owned cleanup"
                );
                probe.acknowledge_return();
                assert!(probe.wait().success());
                assert_runner_returned_after_cleanup(&mut probe);
            }

            for signal in [libc::SIGTERM, libc::SIGINT] {
                eprintln!("bypass delayed={delayed} signal={signal}");
                let dir = tempfile::tempdir().unwrap();
                let mut probe =
                    launch_probe(dir.path(), None, true, true, false, false, Some(delayed));
                unsafe { libc::kill(probe.pid(), signal) };
                causal_cleanup_pending(&mut probe, delayed, true);
                probe.await_return();
                assert!(
                    !probe.returned_after_owned_cleanup(),
                    "the same completion predicate fails while owned work remains live"
                );
                assert!(probe.owned.iter().any(alive));
                assert!(
                    probe.finish().is_err(),
                    "completion rejects live owned work"
                );
                probe.release();
                probe.acknowledge_return();
                assert!(probe.wait().success());
                assert!(
                    probe.cleanup_owned(),
                    "negative-control owned work is torn down"
                );
                assert!(probe.owned.iter().all(|id| !alive(id)));
                assert!(probe.finish().is_ok(), "explicit expected rescue completed");
            }
        }
    }

    #[test]
    fn mixed_signals_after_cleanup_budget_creation_keep_one_owner_and_fixed_budgets() {
        let dir = tempfile::tempdir().unwrap();
        let budget_hold = dir.path().join("cleanup-budget");
        std::fs::create_dir_all(&budget_hold).unwrap();
        std::fs::write(budget_hold.join("arm"), b"arm").unwrap();
        let mut probe =
            launch_probe_with_controls(dir.path(), Some("panic"), false, false, true, false);
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
            ("observation_error", "cleanup_observation_failed"),
            ("forced_expiry", "cleanup_expired"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe =
                launch_probe_with_controls(dir.path(), Some(fault), false, false, true, false);
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

    fn fault_privacy_case(fault: &str, expected: &str) {
        let dir = tempfile::tempdir().unwrap();
        let mut probe =
            launch_probe_with_controls(dir.path(), Some(fault), true, false, false, false);
        wait_for_cleanup_pending(&mut probe, dir.path());
        assert!(probe.owned.iter().any(alive));
        probe.release();
        assert!(probe.wait().success());
        assert_runner_returned_after_cleanup(&mut probe);
        let stderr = std::fs::read_to_string(dir.path().join("probe.stderr")).unwrap();
        assert!(stderr.contains(expected), "{stderr}");
    }

    #[test]
    fn server_death_during_the_trial_is_a_harness_error() {
        fault_privacy_case("server_death", "service_exited_during_trial");
    }

    #[test]
    fn a_runner_panic_cleans_up_and_still_emits_one_row() {
        fault_privacy_case("panic", "runner_panicked");
    }

    #[test]
    fn trial_runtime_owner_survives_aborted_body() {
        let dir = tempfile::tempdir().unwrap();
        let mut probe = launch_probe_with_controls(dir.path(), None, true, false, false, false);
        let bytes = std::fs::read(dir.path().join("runtime-env")).unwrap();
        let paths: Vec<PathBuf> = bytes
            .split(|b| *b == 0)
            .filter_map(|entry| {
                let (key, value) = entry.split_at(entry.iter().position(|b| *b == b'=')?);
                [
                    b"HOME".as_slice(),
                    b"TMPDIR",
                    b"XDG_CONFIG_HOME",
                    b"XDG_CACHE_HOME",
                    b"XDG_DATA_HOME",
                    b"XDG_STATE_HOME",
                ]
                .contains(&key)
                .then(|| PathBuf::from(OsStr::from_bytes(&value[1..])))
            })
            .collect();
        assert_eq!(paths.len(), 6);
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        wait_for_cleanup_pending(&mut probe, dir.path());
        assert!(probe.owned.iter().any(alive));
        assert!(
            paths.iter().all(|path| path.is_dir()),
            "runtime survives joined body abort and actual cleanup Pending"
        );
        probe.release();
        assert!(probe.wait().success());
        assert_runner_returned_after_cleanup(&mut probe);
        assert!(
            paths.iter().all(|path| !path.exists()),
            "runtime released after cleanup"
        );
    }

    #[test]
    fn runner_probe_failure_teardown_handles_owner_and_leader_exit() {
        let dir = tempfile::tempdir().unwrap();
        let bypass = dir.path().join("cleanup-bypass");
        let mut probe = launch_probe_with_controls(dir.path(), None, true, true, true, true);
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        wait_for_cleanup_pending(&mut probe, dir.path());
        let leader = probe.owned.first().unwrap().clone();
        let descendant = probe.owned.get(1).unwrap().clone();
        eventually("the agent cleanup task to enter its hold", || {
            dir.path()
                .join("cleanup")
                .join(format!("entered-{}", leader.pid))
                .is_file()
        });
        eventually("the bypass control to acknowledge actual Pending", || {
            bypass.join("entered").is_file()
        });
        assert!(
            probe.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "the owner remains live until the acknowledged bypass is released"
        );
        std::fs::write(bypass.join("release"), b"release").unwrap();
        eventually("the cleanup bypass to return", || {
            bypass.join("returned").is_file()
        });
        assert!(probe.wait_for(Duration::from_secs(2)).is_some());
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
            control: None,
            returned: false,
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

#[cfg(test)]
mod privacy_tests {
    use super::{
        ARTIFACT_DIRECTORY_MODE, ARTIFACT_FILE_MODE, CARGO_RECORD_LIMIT, CallerEnv,
        create_private_artifact_dir, create_private_artifact_temp,
        create_private_artifact_temp_with, default_home_path, read_cargo_records, resolve_program,
        write_artifact_with,
    };
    use serde_json::json;
    use std::ffi::{OsStr, OsString};
    use std::io::Cursor;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn executable(path: &std::path::Path) {
        std::fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[test]
    fn artifact_leaf_and_temporary_are_private_before_payload() {
        use std::io::Write;

        if std::env::var_os("EVAL_TEST_ARTIFACT_MODE_CHILD").is_none() {
            for mask in ["0000", "0777"] {
                let output = std::process::Command::new("/bin/sh")
                    .args(["-c", "umask \"$1\"; shift; exec \"$@\"", "sh", mask])
                    .arg(std::env::current_exe().unwrap())
                    .args(["--exact", "runner::privacy_tests::artifact_leaf_and_temporary_are_private_before_payload", "--nocapture"])
                    .env("EVAL_TEST_ARTIFACT_MODE_CHILD", "1").output().unwrap();
                assert!(
                    output.status.success(),
                    "pre-payload mask={mask}: {}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        }

        let owner = super::private_tempdir("swarmail-artifact-mode-proof-").unwrap();
        let root = owner.path().join("selected-root");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o750)).unwrap();
        let root_mode = mode(&root);
        let out = create_private_artifact_dir(&root, "fresh-trial").unwrap();
        assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
        assert_eq!(mode(&root), root_mode, "the selected parent is untouched");

        let mut temp = create_private_artifact_temp(&out).unwrap();
        assert_eq!(mode(temp.path()), ARTIFACT_FILE_MODE);
        assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
        assert_eq!(std::fs::metadata(temp.path()).unwrap().len(), 0);
        temp.write_all(b"payload after private creation").unwrap();
        assert_eq!(
            std::fs::read(temp.path()).unwrap(),
            b"payload after private creation"
        );
    }

    #[test]
    fn artifact_leaf_refuses_existing_directories_files_and_symlinks() {
        use std::os::unix::fs::symlink;

        let owner = tempfile::tempdir().unwrap();
        let root = owner.path().join("root");
        std::fs::create_dir(&root).unwrap();
        let target = owner.path().join("target");
        std::fs::write(&target, b"sentinel").unwrap();
        for (name, kind) in ["directory", "file", "live-link", "dangling-link"]
            .into_iter()
            .zip([0, 1, 2, 3])
        {
            let leaf = root.join(name);
            match kind {
                0 => std::fs::create_dir(&leaf).unwrap(),
                1 => std::fs::write(&leaf, b"sentinel").unwrap(),
                2 => symlink(&target, &leaf).unwrap(),
                _ => symlink(owner.path().join("missing-target"), &leaf).unwrap(),
            }
            let existing_mode = std::fs::symlink_metadata(&leaf)
                .unwrap()
                .permissions()
                .mode()
                & 0o7777;
            assert!(create_private_artifact_dir(&root, name).is_err());
            assert_eq!(
                std::fs::symlink_metadata(&leaf)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                existing_mode,
                "an existing trial leaf is not chmodded"
            );
            if kind == 1 {
                assert_eq!(std::fs::read(&leaf).unwrap(), b"sentinel");
            }
            if kind == 2 {
                assert_eq!(std::fs::read(&target).unwrap(), b"sentinel");
            }
            if kind == 3 {
                assert!(std::fs::read_link(&leaf).is_ok());
            }
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"sentinel");
    }

    #[test]
    fn artifact_temp_name_collision_preserves_the_unowned_file() {
        let owner = tempfile::tempdir().unwrap();
        let out = create_private_artifact_dir(owner.path(), "trial").unwrap();
        let collision = out.join(".tmp");
        std::fs::write(&collision, b"unowned sentinel").unwrap();
        std::fs::set_permissions(&collision, std::fs::Permissions::from_mode(0o640)).unwrap();
        let permissions = mode(&collision);
        let mut builder = tempfile::Builder::new();
        builder.prefix(".tmp").rand_bytes(0);
        assert!(create_private_artifact_temp_with(&out, &builder).is_err());
        assert_eq!(std::fs::read(&collision).unwrap(), b"unowned sentinel");
        assert_eq!(mode(&collision), permissions);
        assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
    }

    #[test]
    fn artifact_final_name_appears_only_after_complete_private_payload() {
        use std::io::Write;

        let owner = tempfile::tempdir().unwrap();
        let out = create_private_artifact_dir(owner.path(), "trial").unwrap();
        let final_path = out.join("result.json");
        let payload = b"complete artifact payload";
        write_artifact_with(&out, "result.json", payload, |file, bytes| {
            assert_eq!(
                file.metadata()?.permissions().mode() & 0o7777,
                ARTIFACT_FILE_MODE
            );
            assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
            assert!(!final_path.exists());
            file.write_all(&bytes[..8])?;
            assert!(!final_path.exists());
            file.write_all(&bytes[8..])
        })
        .unwrap();
        assert_eq!(std::fs::read(&final_path).unwrap(), payload);
        assert_eq!(mode(&final_path), ARTIFACT_FILE_MODE);
        assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
    }

    #[test]
    fn artifact_write_failure_cleans_only_its_private_partial_file() {
        use std::io::Write;

        let owner = tempfile::tempdir().unwrap();
        let out = create_private_artifact_dir(owner.path(), "trial").unwrap();
        let final_path = out.join("result.json");
        let error = write_artifact_with(
            &out,
            "result.json",
            b"synthetic private payload",
            |file, payload| {
                assert_eq!(
                    file.metadata()?.permissions().mode() & 0o7777,
                    ARTIFACT_FILE_MODE
                );
                assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
                assert!(!final_path.exists());
                file.write_all(&payload[..8])?;
                Err(std::io::Error::other("synthetic write failure"))
            },
        );
        assert_eq!(error.unwrap_err(), "artifact_write");
        assert!(!final_path.exists());
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
        assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
    }

    #[test]
    fn artifact_publication_refuses_existing_targets_without_clobbering() {
        use std::io::Write;
        use std::os::unix::fs::{MetadataExt, symlink};

        let owner = tempfile::tempdir().unwrap();
        let out = create_private_artifact_dir(owner.path(), "trial").unwrap();
        let payload = b"complete replacement must not happen";

        let file = out.join("file-target");
        std::fs::write(&file, b"file sentinel").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
        let file_mode = mode(&file);
        assert_eq!(
            write_artifact_with(&out, "file-target", payload, |tmp, bytes| tmp
                .write_all(bytes)),
            Err("artifact_write".into())
        );
        assert_eq!(std::fs::read(&file).unwrap(), b"file sentinel");
        assert_eq!(mode(&file), file_mode);

        let directory = out.join("directory-target");
        std::fs::create_dir(&directory).unwrap();
        let directory_mode = mode(&directory);
        assert_eq!(
            write_artifact_with(&out, "directory-target", payload, |tmp, bytes| tmp
                .write_all(bytes)),
            Err("artifact_write".into())
        );
        assert_eq!(mode(&directory), directory_mode);
        assert!(directory.is_dir());

        let live_target = out.join("live-target-data");
        std::fs::write(&live_target, b"live sentinel").unwrap();
        let live_link = out.join("live-link-target");
        symlink(&live_target, &live_link).unwrap();
        let link_target = std::fs::read_link(&live_link).unwrap();
        assert_eq!(
            write_artifact_with(&out, "live-link-target", payload, |tmp, bytes| tmp
                .write_all(bytes)),
            Err("artifact_write".into())
        );
        assert_eq!(std::fs::read(&live_target).unwrap(), b"live sentinel");
        assert_eq!(std::fs::read_link(&live_link).unwrap(), link_target);

        let dangling = out.join("dangling-link-target");
        let absent = owner.path().join("still-absent");
        symlink(&absent, &dangling).unwrap();
        let dangling_target = std::fs::read_link(&dangling).unwrap();
        assert_eq!(
            write_artifact_with(&out, "dangling-link-target", payload, |tmp, bytes| tmp
                .write_all(bytes)),
            Err("artifact_write".into())
        );
        assert!(!absent.exists());
        assert_eq!(std::fs::read_link(&dangling).unwrap(), dangling_target);

        let original = out.join("hard-link-original");
        let hard_link = out.join("hard-link-target");
        std::fs::write(&original, b"hard-link sentinel").unwrap();
        std::fs::set_permissions(&original, std::fs::Permissions::from_mode(0o640)).unwrap();
        std::fs::hard_link(&original, &hard_link).unwrap();
        let original_meta = std::fs::metadata(&original).unwrap();
        assert_eq!(
            write_artifact_with(&out, "hard-link-target", payload, |tmp, bytes| tmp
                .write_all(bytes)),
            Err("artifact_write".into())
        );
        let after_meta = std::fs::metadata(&original).unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), b"hard-link sentinel");
        assert_eq!(std::fs::read(&hard_link).unwrap(), b"hard-link sentinel");
        assert_eq!(after_meta.ino(), original_meta.ino());
        assert_eq!(after_meta.mode() & 0o7777, original_meta.mode() & 0o7777);
        assert_eq!(mode(&out), ARTIFACT_DIRECTORY_MODE);
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 7);
    }

    #[tokio::test]
    async fn admin_transport_read_and_parse_failures_are_value_free() {
        use super::{Admin, http_client};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        let admin = Admin {
            http: http_client(),
            base: format!("http://{address}"),
            secret: "synthetic-secret".into(),
        };
        assert_eq!(
            admin
                .call(false, "/synthetic-url-canary", None)
                .await
                .unwrap_err(),
            "admin_transport"
        );
        for (response, expected) in [
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 5000\r\nConnection: close\r\n\r\nsynthetic-read-canary",
                "admin_response_read",
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 22\r\nConnection: close\r\n\r\nsynthetic-parse-canary!",
                "admin_response_invalid",
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut chunk = [0u8; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert_ne!(n, 0);
                    request.extend_from_slice(&chunk[..n]);
                }
                stream.write_all(response.as_bytes()).await.unwrap();
            });
            let admin = Admin {
                http: http_client(),
                base: format!("http://{address}"),
                secret: "synthetic-secret".into(),
            };
            assert_eq!(
                admin
                    .call(false, "/synthetic-url-canary", None)
                    .await
                    .unwrap_err(),
                expected
            );
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn cargo_runtime_owner_survives_cancelled_preparation() {
        use super::{Groups, RuntimeDirs, private_tempdir, spawn_owned_cargo};
        use std::time::Duration;
        use tokio::process::Command;
        let control = tempfile::tempdir().unwrap();
        let entered = control.path().join("entered");
        let ready = control.path().join("ready");
        let release = control.path().join("release");
        let runtime = Arc::new(private_tempdir("swarmail-eval-build-owner-").unwrap());
        let root = runtime.path().to_path_buf();
        let dirs = RuntimeDirs::create(runtime.path(), "cargo").unwrap();
        let script = format!(
            "trap 'touch \"{}\"; while [ ! -e \"{}\" ]; do :; done; exit 0' TERM; touch \"{}\"; while :; do :; done",
            entered.display(),
            release.display(),
            ready.display()
        );
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", &script]);
        dirs.apply(&mut cmd);
        let groups = Groups::default();
        let cargo = spawn_owned_cargo(cmd, &groups, runtime).unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            while !ready.exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        {
            let cleanup = cargo.cleanup();
            tokio::pin!(cleanup);
            tokio::select! { result = &mut cleanup => panic!("cleanup returned before barrier: {result:?}"),
            _ = async { while !entered.exists() { tokio::task::yield_now().await; } } => {} }
            assert!(
                root.is_dir() && dirs.home.is_dir(),
                "private owner retained through actual cleanup Pending"
            );
            std::fs::write(release, b"release").unwrap();
            cleanup.await.unwrap();
        }
        drop(cargo);
        drop(groups);
        assert!(
            !root.exists(),
            "owner released after group cleanup/reap/drains"
        );
    }

    #[tokio::test]
    async fn admin_error_categories_discard_injected_response_text() {
        use super::{Admin, http_client, safe_trial_category};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let canary = b"synthetic-http-response-canary-196";
        let body = br#"{"message":"synthetic-http-response-canary-196"}"#;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let count = stream.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0, "client sent a complete request");
                request.extend_from_slice(&chunk[..count]);
            }
            let headers = format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(body).await.unwrap();
        });
        let admin = Admin {
            http: http_client(),
            base: format!("http://{address}"),
            secret: "synthetic-admin-secret".into(),
        };
        let error = admin.call(false, "/injected", None).await.unwrap_err();
        assert_eq!(error, "admin_response_status");
        assert_eq!(safe_trial_category(&error), "admin_response_rejected");
        assert!(!error.as_bytes().windows(canary.len()).any(|w| w == canary));
        server.await.unwrap();
    }

    #[test]
    fn executable_resolution_uses_only_the_supplied_original_context() {
        let owner = tempfile::tempdir().unwrap();
        let cwd = owner.path().to_path_buf();
        let path_dir = cwd.join("tools");
        std::fs::create_dir(&path_dir).unwrap();
        executable(&cwd.join("relative-agent"));
        executable(&path_dir.join("bare-agent"));
        let odd_name = OsString::from_vec(b"nonutf8-\xff-agent".to_vec());
        executable(&path_dir.join(&odd_name));
        let path =
            std::env::join_paths([OsString::new(), path_dir.clone().into_os_string()]).unwrap();
        let caller = CallerEnv {
            cwd: cwd.clone(),
            path: Some(path),
        };
        assert_eq!(
            resolve_program(OsStr::new("relative-agent"), &caller).unwrap(),
            cwd.join("relative-agent")
        );
        assert_eq!(
            resolve_program(OsStr::new("bare-agent"), &caller).unwrap(),
            path_dir.join("bare-agent")
        );
        assert_eq!(
            resolve_program(&odd_name, &caller).unwrap(),
            path_dir.join(&odd_name)
        );
        let absolute = path_dir.join("bare-agent");
        assert_eq!(
            resolve_program(absolute.as_os_str(), &caller).unwrap(),
            absolute
        );
        assert_eq!(
            resolve_program(OsStr::new("missing-agent"), &caller).unwrap_err(),
            "program_missing"
        );
    }

    #[test]
    fn default_toolchain_homes_resolve_relative_home_at_the_caller_boundary() {
        let owner = tempfile::tempdir().unwrap();
        let caller = CallerEnv {
            cwd: owner.path().to_path_buf(),
            path: None,
        };
        assert_eq!(
            default_home_path(&caller, OsStr::new("caller-home"), ".cargo").unwrap(),
            owner.path().join("caller-home/.cargo")
        );
        assert_eq!(
            default_home_path(&caller, owner.path().as_os_str(), ".rustup").unwrap(),
            owner.path().join(".rustup")
        );
        assert_eq!(
            default_home_path(&caller, OsStr::new(""), ".cargo").unwrap_err(),
            "toolchain_discovery"
        );
    }

    #[test]
    fn cargo_record_reader_bounds_lines_and_keeps_draining_after_overflow() {
        let artifact = json!({
            "reason": "compiler-artifact",
            "target": {"name": "swarmail", "kind": ["bin"]},
            "executable": "/tmp/swarmail"
        })
        .to_string();
        let found = Arc::new(Mutex::new(None));
        assert!(read_cargo_records(
            Cursor::new(format!("{artifact}\n")),
            found.clone()
        ));
        assert_eq!(*found.lock().unwrap(), Some(PathBuf::from("/tmp/swarmail")));

        let mut stream = vec![b'x'; CARGO_RECORD_LIMIT + 1];
        stream.push(b'\n');
        stream.extend_from_slice(artifact.as_bytes());
        stream.push(b'\n');
        let after_overflow = Arc::new(Mutex::new(None));
        assert!(!read_cargo_records(
            Cursor::new(stream),
            after_overflow.clone()
        ));
        assert_eq!(
            *after_overflow.lock().unwrap(),
            Some(PathBuf::from("/tmp/swarmail")),
            "overflow is discarded through newline and later protocol data is drained"
        );
    }
}
