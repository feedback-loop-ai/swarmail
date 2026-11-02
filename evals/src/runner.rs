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
use std::io;
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
#[cfg(all(test, target_os = "linux"))]
const CLEANUP_OBSERVE_ERROR_ENV: &str = "EVAL_TEST_CLEANUP_OBSERVE_ERROR";
#[cfg(all(test, target_os = "linux"))]
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
        groups: Mutex<Vec<std::sync::Weak<OwnedGroup>>>,
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
                groups: Mutex::new(Vec::new()),
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

        pub(super) fn record_group(&self, group: &Arc<OwnedGroup>) {
            self.groups.lock().unwrap().push(Arc::downgrade(group));
        }

        // The failed-cleanup probes observe only the background failure owner:
        // no caller, reaper or ordinary output task may mask an early release.
        pub(super) async fn failed_owners_only(&self) {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                let ready = self.groups.lock().unwrap().iter().all(|weak| {
                    weak.upgrade().is_none_or(|group| {
                        Arc::strong_count(&group) == 2
                            && group.status().is_some_and(|status| status.is_ok())
                            && group
                                .drains
                                .lock()
                                .unwrap()
                                .iter()
                                .all(|drain| drain.is_finished())
                    })
                });
                if ready {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "unrelated cleanup owners finished"
                );
                tokio::task::yield_now().await;
            }
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
    #[serde(deserialize_with = "crate::provenance::nullable")]
    pub code: Option<i32>,
    #[serde(deserialize_with = "crate::provenance::nullable")]
    pub signal: Option<i32>,
}

/// The one JSON line a trial writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialResult {
    pub schema_version: u32,
    /// Supported trial execution limit, including refused/unstarted rows.
    pub concurrency: u32,
    pub trial_id: String,
    #[serde(deserialize_with = "crate::provenance::nullable")]
    pub task: Option<String>,
    #[serde(deserialize_with = "crate::provenance::nullable")]
    pub agent: Option<String>,
    pub outcome: Outcome,
    #[serde(deserialize_with = "crate::provenance::nullable")]
    pub failure_class: Option<FailureClass>,
    pub wall_time_ms: u64,
    #[serde(deserialize_with = "crate::provenance::nullable")]
    pub agent_exit_status: Option<i32>,
    pub agent_exits: Vec<AgentExit>,
    pub confinement: String,
    pub provenance: crate::provenance::Provenance,
}

impl TrialResult {
    /// Whether the row obeys its own schema's invariants.
    pub fn validate(&self) -> Result<(), String> {
        self.provenance.validate()?;
        use crate::provenance::{AgentKind, Availability, Reason, Source};
        match self.provenance.agent.kind {
            AgentKind::Invalid => {
                if self.outcome != Outcome::HarnessError
                    || self.provenance.matrix.is_some()
                    || self.provenance.task.definition_sha256
                        != Availability::unavailable(Reason::RequestInvalid)
                    || self.provenance.limits.agent_deadline_ms
                        != Availability::unavailable(Reason::RequestInvalid)
                {
                    return Err("invalid request shape".into());
                }
            }
            kind => {
                let (Some(_), Some(agent)) = (&self.task, &self.agent) else {
                    return Err("valid request lacks labels".into());
                };
                if (kind == AgentKind::Builtin) != BUILTINS.contains(&agent.as_str()) {
                    return Err("agent kind/label mismatch".into());
                }
                if kind == AgentKind::Builtin
                    && self.provenance.agent.config_id
                        != Availability::present(agent.clone(), Source::Builtin)
                {
                    return Err("builtin config mismatch".into());
                }
                if !matches!(
                    self.provenance.task.definition_sha256,
                    Availability::Present { .. }
                ) || !matches!(
                    self.provenance.limits.agent_deadline_ms,
                    Availability::Present { .. }
                ) {
                    return Err("valid request lacks task/deadline".into());
                }
            }
        }

        if !crate::artifacts::generated_id(&self.trial_id)
            || self.task.as_ref().is_some_and(|s| s.len() > 1024)
            || self
                .agent
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("invalid row identity".into());
        }
        if let Some(m) = &self.provenance.matrix {
            let (Some(task), Some(agent)) = (&self.task, &self.agent) else {
                return Err("matrix labels absent".into());
            };
            if !crate::artifacts::generated_id(&m.matrix_id)
                || m.cell_seed(task, agent)? != self.provenance.fixture.seed
            {
                return Err("matrix seed/identity mismatch".into());
            }
        }
        if self.provenance.agent.kind == crate::provenance::AgentKind::Invalid
            && self.outcome != Outcome::HarnessError
        {
            return Err("invalid request must be harness_error".into());
        }

        if self.schema_version != RESULT_SCHEMA {
            return Err(format!(
                "schema_version {} is not {RESULT_SCHEMA}",
                self.schema_version
            ));
        }
        if self.concurrency != 1 {
            return Err("concurrency must be 1".into());
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
    pub concurrency: Option<u32>,
    pub fixture: crate::provenance::FixtureInputs,
    pub matrix: Option<crate::provenance::MatrixCoordinates>,
    pub trial_id: Option<String>,
    pub agent_model: Option<String>,
    pub agent_config_id: Option<String>,
    pub public_argv: bool,
    pub wrapper_profile: Option<crate::provenance::WrapperProfile>,
    pub target_build: Option<crate::identity::TargetBuild>,
    pub assigned_agent: Option<crate::provenance::AgentIdentity>,
    pub harness_identity: Option<crate::provenance::HarnessIdentity>,
    pub target_source_failure: Option<crate::provenance::Reason>,
}

pub fn new_trial_id() -> String {
    use rand::Rng;
    crate::provenance::hex(&rand::rng().random::<[u8; 16]>())
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

/// The shared sticky temporary directory every private runtime directory is
/// created in, by its symlink-free name. Private runtime paths are walked
/// without following symlinks, and macOS's /tmp is a symlink to /private/tmp.
#[cfg(target_os = "macos")]
pub const SHARED_TMP: &str = "/private/tmp";
/// The shared sticky temporary directory every private runtime directory is
/// created in, by its symlink-free name.
#[cfg(not(target_os = "macos"))]
pub const SHARED_TMP: &str = "/tmp";

pub fn private_tempdir(prefix: &str) -> Result<tempfile::TempDir, String> {
    let directory = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(SHARED_TMP)
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

/// Resolve the external agent's program at the original caller boundary,
/// without launching it (the isolated entry binds the file it names).
pub fn selected_agent_program(program: &OsStr) -> Result<PathBuf, String> {
    resolve_program(program, &CallerEnv::capture()?)
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
pub struct Groups(
    Arc<Mutex<Vec<Arc<OwnedGroup>>>>,
    Option<Arc<crate::admission::Admission>>,
    #[cfg(all(test, target_os = "linux"))] Option<Arc<DrainTestControl>>,
);

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
    _admission: Option<Arc<crate::admission::Admission>>,
    #[cfg(all(test, target_os = "linux"))]
    drain_probe: Option<Arc<DrainTestControl>>,
}

// Per-spawn holds for the actual specialized stdout consumers. These carry
// no admission, runtime or process owner; tests retain only non-owning observers.
#[cfg(all(test, target_os = "linux"))]
#[derive(Default)]
struct DrainHold {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[cfg(all(test, target_os = "linux"))]
impl DrainHold {
    async fn wait(&self) {
        self.entered.notify_one();
        self.release.notified().await;
    }
}

#[cfg(all(test, target_os = "linux"))]
struct DrainTestControl {
    stdout: DrainHold,
    progress: tokio::sync::mpsc::UnboundedSender<&'static str>,
}

// Only Linux can read a member's pid, group and state (from /proc); other
// Unix systems observe the group as a whole through kill(-pgid, 0).
#[derive(Debug, Clone, Copy)]
struct ProcIdentity {
    #[cfg(target_os = "linux")]
    pid: i32,
    #[cfg(target_os = "linux")]
    pgrp: i32,
    session: i32,
    start_time: u64,
    #[cfg(target_os = "linux")]
    state: char,
}

#[cfg(target_os = "linux")]
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
                session: group.session.unwrap_or_default(),
                start_time: group.start_time.unwrap_or_default(),
            }])
        } else {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::ESRCH) {
                Ok(Vec::new())
            } else if e.raw_os_error() == Some(libc::EPERM) {
                Ok(vec![ProcIdentity {
                    session: group.session.unwrap_or_default(),
                    start_time: group.start_time.unwrap_or_default(),
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
    } else if cfg!(target_os = "macos") && e.raw_os_error() == Some(libc::EPERM) {
        // Darwin answers EPERM for a group whose remaining members are all
        // zombies, as when a child exited just before cleanup and is not yet
        // reaped. The signal proves nothing either way: cleanup succeeds only
        // once observation (which counts EPERM as presence) finds the group
        // gone, and a member that truly cannot be signalled outlives the
        // deadline and fails cleanup as expired.
        Ok(())
    } else {
        Err(format!(
            "cannot signal owned process group {}: {e}",
            group.pgid
        ))
    }
}

#[cfg(all(test, target_os = "linux"))]
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
            let result = group.clone().cleanup_once().await;
            let failed = result.is_err();
            tx.send_replace(Some(result));
            if failed {
                // Keep this existing cleanup owner, private state and admission
                // alive after an unproved cleanup. Reporting an error is not
                // evidence that a child stopped. Do not change the cached verdict.
                loop {
                    if group_members(&group).is_ok_and(|members| members.is_empty())
                        && group.status().is_some_and(|status| status.is_ok())
                    {
                        break;
                    }
                    tokio::time::sleep(CLEANUP_POLL).await;
                }
                let drains =
                    std::mem::take(&mut *group.drains.lock().unwrap_or_else(|e| e.into_inner()));
                for mut drain in drains {
                    #[cfg(all(test, target_os = "linux"))]
                    if let Some(probe) = &group.drain_probe {
                        let mut reported = false;
                        let _ = std::future::poll_fn(|cx| {
                            let result = std::pin::Pin::new(&mut drain).poll(cx);
                            if result.is_pending() && !reported {
                                let _ = probe.progress.send("drain-pending");
                                reported = true;
                            }
                            result
                        })
                        .await;
                        continue;
                    }
                    let _ = (&mut drain).await;
                }
            }
            #[cfg(all(test, target_os = "linux"))]
            {
                let probe = group.drain_probe.clone();
                drop(group);
                if let Some(probe) = probe {
                    let _ = probe.progress.send("owner-disposed");
                }
            }
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
        let mut drains = drains.into_iter();
        while let Some(mut drain) = drains.next() {
            if tokio::time::timeout_at(final_deadline, &mut drain)
                .await
                .is_err()
            {
                // The failed-cleanup owner must join this handle and the
                // unvisited suffix before releasing private state or admission.
                let mut retained = self.drains.lock().unwrap_or_else(|e| e.into_inner());
                retained.push(drain);
                retained.extend(drains);
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
                        // Pending can mean the waiter has not been scheduled.
                        // The negative control must bypass an acknowledged live
                        // cleanup owner, not abort its waiter before startup.
                        if let Some(hold) = std::env::var_os(CLEANUP_HOLD_ENV).map(PathBuf::from)
                            && hold.join("arm").exists()
                        {
                            while !std::fs::read_dir(&hold).is_ok_and(|entries| {
                                entries.flatten().any(|e| {
                                    e.file_name().to_string_lossy().starts_with("entered-")
                                })
                            }) {
                                tokio::time::sleep(Duration::from_millis(5)).await;
                            }
                        }
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
    lifetime: (
        Option<Arc<tempfile::TempDir>>,
        Option<Arc<crate::admission::Admission>>,
    ),
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let _lifetime = lifetime;
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
    captured: Option<oneshot::Receiver<Result<Vec<u8>, String>>>,
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
    cmd: Command,
    groups: &Groups,
    role: Option<ServerRole>,
    runtime: Option<Arc<tempfile::TempDir>>,
    cargo: bool,
) -> std::io::Result<Proc> {
    spawn_extended(cmd, groups, role, runtime, cargo, false)
}

fn spawn_extended(
    mut cmd: Command,
    groups: &Groups,
    role: Option<ServerRole>,
    runtime: Option<Arc<tempfile::TempDir>>,
    cargo: bool,
    capture: bool,
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
        _admission: groups.1.clone(),
        #[cfg(all(test, target_os = "linux"))]
        drain_probe: groups.2.clone(),
    });
    #[cfg(all(test, target_os = "linux"))]
    if let Some(control) = cleanup_probe::CURRENT.get() {
        control.record_group(&group);
    }
    groups.add(group.clone());
    let admission = groups.1.clone();
    tokio::spawn(async move {
        let _lifetime = (runtime, admission);
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
    let mut captured = None;
    #[cfg(all(test, target_os = "linux"))]
    let stdout_probe = groups.2.clone();
    let cargo_records = if capture {
        let (tx, rx) = oneshot::channel();
        captured = Some(rx);
        drains.push(tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut reader = stdout;
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 8192];
            let mut overflow = false;
            let result = loop {
                match reader.read(&mut chunk).await {
                    Ok(0) => {
                        break if overflow {
                            Err("helper_output_invalid".into())
                        } else {
                            Ok(bytes)
                        };
                    }
                    Ok(n) => {
                        let keep = n.min((1024 * 1024usize).saturating_sub(bytes.len()));
                        bytes.extend_from_slice(&chunk[..keep]);
                        overflow |= keep != n;
                    }
                    Err(_) => break Err("helper_output_failed".into()),
                }
            };
            #[cfg(all(test, target_os = "linux"))]
            if let Some(probe) = stdout_probe {
                probe.stdout.wait().await;
            }
            let _ = tx.send(result);
        }));
        None
    } else if cargo {
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
            #[cfg(all(test, target_os = "linux"))]
            if let Some(probe) = stdout_probe {
                probe.stdout.wait().await;
            }
            let _ = tx.send(result);
        }));
        Some(rx)
    } else {
        drains.push(drain(
            stdout,
            role,
            sender,
            (group._runtime.clone(), groups.1.clone()),
        ));
        None
    };
    drains.push(drain(
        stderr,
        None,
        None,
        (group._runtime.clone(), groups.1.clone()),
    ));
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
        captured,
    })
}

pub fn source_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("evals parent")
}

pub async fn owned_source_context(
    root: &Path,
    groups: &Groups,
) -> Result<crate::identity::SourceContext, String> {
    let caller = CallerEnv::capture()?;
    let git = resolve_program(OsStr::new("git"), &caller).ok();
    let mut facts = Vec::new();
    for args in [
        vec!["rev-parse", "--verify", "HEAD"],
        vec!["status", "--porcelain", "--untracked-files=all"],
    ] {
        let value = if let Some(git) = &git {
            let runtime = Arc::new(private_tempdir("swarmail-eval-git-")?);
            let dirs = RuntimeDirs::create(runtime.path(), "git")?;
            let mut cmd = Command::new(git);
            dirs.apply(&mut cmd);
            cmd.current_dir(root)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args([
                    "-c",
                    "core.fsmonitor=false",
                    "-c",
                    "core.untrackedCache=false",
                ])
                .args(args);
            let mut proc = spawn_extended(cmd, groups, None, Some(runtime), false, true)
                .map_err(|_| "helper_spawn")?;
            let rx = proc.captured.take().ok_or("helper_output")?;
            let result = tokio::time::timeout(Duration::from_secs(10), async {
                let status = proc.child.wait().await.map_err(|_| "helper_wait")?;
                let bytes = rx.await.map_err(|_| "helper_output")??;
                Ok::<_, String>(status.success().then_some(bytes))
            })
            .await;
            proc.cleanup().await?;
            result.map_err(|_| "helper_timeout")??
        } else {
            None
        };
        facts.push(value);
    }
    crate::identity::SourceContext::with_git(root, facts[0].as_deref(), facts[1].as_deref())
}

pub async fn build_owned_target(
    root: &Path,
    groups: &Groups,
) -> Result<crate::identity::TargetBuild, String> {
    let before = owned_source_context(root, groups).await?;
    let (mut cmd, runtime) = prepare_swarmail_build().map_err(|_| "build_failed")?;
    cmd.current_dir(root);
    let mut cargo = spawn_owned_cargo(cmd, groups, runtime).map_err(|_| "cargo_spawn")?;
    let records = cargo.cargo_records.take().ok_or("cargo_output_missing")?;
    let result = async {
        let status = cargo.child.wait().await.map_err(|_| "cargo_wait")?;
        let path = tokio::time::timeout(Duration::from_secs(5), records)
            .await
            .map_err(|_| "cargo_output_timeout")?
            .map_err(|_| "cargo_output_failed")??;
        if !status.success() {
            return Err("build_failed".into());
        }
        if before != owned_source_context(root, groups).await? {
            return Err("build_context_changed".into());
        }
        let target = before.associate(path);
        target.recheck()?;
        Ok(target)
    }
    .await;
    cargo.cleanup().await?;
    result
}

// Synchronous inspection/build conveniences also subscribe before any helper.
pub fn inspect_source(root: &Path) -> Result<crate::identity::SourceContext, String> {
    let root = root.to_path_buf();
    tokio::runtime::Runtime::new().map_err(|_| "runtime")?.block_on(async {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).map_err(|_| "signal_setup")?;
        let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).map_err(|_| "signal_setup")?;
        let groups = Groups::default(); let owned = groups.clone();
        let mut body = tokio::spawn(async move { owned_source_context(&root, &owned).await });
        let result = tokio::select! { biased; _ = term.recv() => None, _ = int.recv() => None, result = &mut body => Some(result.map_err(|_| "source_context_failed")), };
        if result.is_none() { body.abort(); let _ = body.await; }
        let cleanup = groups.cleanup_all(); tokio::pin!(cleanup);
        loop { tokio::select! { biased; result = &mut cleanup => { result?; break; }, _ = term.recv() => {}, _ = int.recv() => {}, } }
        result.ok_or("preparation_interrupted")??
    })
}
pub fn resolve_owned_target(root: &Path) -> Result<crate::identity::TargetBuild, String> {
    let root = root.to_path_buf();
    tokio::runtime::Runtime::new().map_err(|_| "runtime")?.block_on(async {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).map_err(|_| "signal_setup")?;
        let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).map_err(|_| "signal_setup")?;
        let groups = Groups::default(); let owned = groups.clone();
        let mut body = tokio::spawn(async move { build_owned_target(&root, &owned).await });
        let result = tokio::select! { biased; _ = term.recv() => None, _ = int.recv() => None, result = &mut body => Some(result.map_err(|_| "build_failed")), };
        if result.is_none() { body.abort(); let _ = body.await; }
        let cleanup = groups.cleanup_all(); tokio::pin!(cleanup);
        loop { tokio::select! { biased; result = &mut cleanup => { result?; break; }, _ = term.recv() => {}, _ = int.recv() => {}, } }
        result.ok_or("preparation_interrupted")??
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
    /// The SMTP AUTH username the fixture sends this account's mail under.
    smtp_identity: String,
}

/// The one application login every inbox-isolation account sends under
/// (decision 0112 §7).
const ISOLATION_SMTP_IDENTITY: &str = "fixture-app";
/// The route domain the runner starts Swarmail with (decision 0112 §7). It
/// is inert for the `@example.test` identities of every other task.
const ROUTE_DOMAIN: &str = "swarmail.test";

fn identities(trial_id: &str, count: u8) -> Vec<Identity> {
    use rand::Rng;
    let names = ["ada", "grace", "barbara"];
    let short = trial_id
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>()
        // A routed label is folded to lowercase by the server (decision 0112
        // §1), so the inbox the agent reads must already be lowercase.
        .to_ascii_lowercase();
    (0..count as usize)
        .map(|i| {
            let n: u32 = rand::rng().random_range(0x10000..0xfffff);
            let inbox = format!("eval-{short}-{}", names[i]);
            if count > 1 {
                // Inbox-isolation: three accounts, one application login; the
                // routed recipient names the inbox (decision 0112 §7).
                Identity {
                    identity: format!("{}.{n:x}@{inbox}.{ROUTE_DOMAIN}", names[i]),
                    smtp_identity: ISOLATION_SMTP_IDENTITY.to_string(),
                    inbox,
                }
            } else {
                Identity {
                    identity: format!("{}.{n:x}@example.test", names[i]),
                    smtp_identity: inbox.clone(),
                    inbox,
                }
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
        let program = resolve_program(argv.first().ok_or("program_missing")?, caller)?;
        let mut resolved = Vec::with_capacity(argv.len());
        resolved.push(program.into_os_string());
        resolved.extend(argv[1..].iter().cloned());
        Ok(resolved)
    }
}

/// Validate the request before anything starts.
pub fn validate_request(req: &TrialRequest) -> Result<(Task, String), String> {
    if req.concurrency.unwrap_or(1) != 1 {
        return Err("concurrency_unsupported".into());
    }
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
    req.fixture.validate()?;
    if label.len() > 1024
        || task_id.len() > 1024
        || req
            .agent_model
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1024)
        || req
            .agent_config_id
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1024)
    {
        return Err("public declaration exceeds bounds".into());
    }
    if BUILTINS.contains(&label)
        && (req.agent_model.is_some()
            || req.agent_config_id.is_some()
            || req.public_argv
            || req.wrapper_profile.is_some())
    {
        return Err("builtin declarations cannot be overridden".into());
    }
    if req.public_argv && req.argv.iter().any(|v| v.to_str().is_none()) {
        return Err("public_argv_requires_utf8".into());
    }
    if let Some(profile) = req.wrapper_profile {
        profile.validate_model(req.agent_model.as_deref())?;
    }
    tasks::validate_task(&task)?;
    Ok((task, label.to_string()))
}

/// The harness-error row for a request that never started.
pub fn refused_row(req: &TrialRequest, trial_id: &str) -> TrialResult {
    TrialResult {
        schema_version: RESULT_SCHEMA,
        concurrency: 1,
        trial_id: trial_id.to_string(),
        task: req
            .task
            .clone()
            .filter(|s| !s.is_empty() && s.len() <= 1024),
        agent: req
            .agent
            .clone()
            .filter(|s| !s.is_empty() && s.len() <= 1024),
        outcome: Outcome::HarnessError,
        failure_class: Some(FailureClass::HarnessError),
        wall_time_ms: 0,
        agent_exit_status: None,
        agent_exits: Vec::new(),
        confinement: CONFINEMENT.to_string(),
        provenance: make_provenance(req, req.target_build.as_ref(), None),
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
    target: &crate::identity::TargetBuild,
    inputs: &crate::provenance::FixtureInputs,
    provenance: &crate::provenance::Provenance,
    scratch: &Path,
    trial: &mut Trial,
    exits: &mut Vec<AgentExit>,
    snapshot_out: &mut Option<Snapshot>,
) -> Result<bool, String> {
    // Every role gets a closed runtime map and a private owner under /tmp.
    let swarmail_dirs = RuntimeDirs::create(scratch, "swarmail")?;
    target.recheck()?;
    let mut cmd = Command::new(&target.path);
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
        "--route-domain",
        ROUTE_DOMAIN,
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
    recheck_digest(&exe, &provenance.harness.executable_sha256)?;
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
        "--seed",
        &inputs.seed,
        "--fixture-time",
        &inputs.fixture_time,
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
        .map(|i| {
            json!({ "identity": i.identity, "inbox": i.inbox, "smtp_identity": i.smtp_identity })
        })
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
        recheck_digest(Path::new(&argv[0]), &provenance.agent.executable_sha256)?;
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
    services_survived(&mut trial.servers, &client, http, &app_url).await?;

    // Freeze, then take one consistent snapshot before the servers stop.
    admin.call(true, "/admin/freeze", None).await?;
    let snap = admin.call(false, "/admin/snapshot", None).await?;
    let snap: Snapshot = serde_json::from_value(snap).map_err(|_| "snapshot_invalid")?;
    *snapshot_out = Some(snap);
    Ok(matches!(end, AgentsEnd::Deadline))
}

/// How long a service whose liveness probe failed gets to show it exited,
/// which only names the harness error; the trial is a harness error anyway.
const SERVICE_EXIT_GRACE: Duration = Duration::from_secs(5);

/// Prove both services outlived the agents before the trial is graded. The
/// agent loop sees a service die only once its exit is reaped, and an agent
/// that kills a service and then exits can be reaped first, so the trial
/// would be graded against a dead service. A service killed before this
/// probe can never answer it — its pending SIGKILL ends every thread at the
/// next return to user space — so an answer proves it alive after the agents
/// ended, and an exit at any point before that answer is a harness error.
async fn services_survived(
    servers: &mut [Proc],
    client: &reqwest::Client,
    swarmail_http: SocketAddr,
    app_url: &str,
) -> Result<(), String> {
    let (swarmail_slot, fixture_slot) = servers.split_at_mut(1);
    let swarmail = &mut swarmail_slot[0].child;
    let fixture = &mut fixture_slot[0].child;
    let probe = async {
        let health = client
            .get(format!("http://{swarmail_http}/healthz"))
            .send()
            .await
            .map_err(|_| "swarmail_health")?;
        if !health.status().is_success() {
            return Err("swarmail_health");
        }
        let index = client
            .get(format!("{app_url}/"))
            .send()
            .await
            .map_err(|_| "fixture_index")?;
        if !index.status().is_success() {
            return Err("fixture_index");
        }
        Ok(())
    };
    let failed = tokio::select! {
        biased;
        _ = swarmail.wait() => return Err("server_died".into()),
        _ = fixture.wait() => return Err("server_died".into()),
        probed = probe => match probed {
            Ok(()) => return Ok(()),
            Err(category) => category,
        },
    };
    // A refused or reset probe is how a dying service usually shows first;
    // its reaped exit names the error when it follows within the grace.
    tokio::select! {
        biased;
        _ = swarmail.wait() => Err("server_died".into()),
        _ = fixture.wait() => Err("server_died".into()),
        _ = tokio::time::sleep(SERVICE_EXIT_GRACE) => Err(failed.into()),
    }
}

const ARTIFACT_DIRECTORY_MODE: u32 = 0o700;
const ARTIFACT_FILE_MODE: u32 = 0o600;

/// Make an exclusively owned trial directory under the caller-selected root.
/// Existing roots and ancestors are caller-owned and keep their permissions.
pub(crate) fn create_private_artifact_dir(root: &Path, trial_id: &str) -> Result<PathBuf, String> {
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

pub(crate) fn write_artifact_with(
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
        "cargo_output_invalid"
        | "cargo_output_failed"
        | "cargo_output_timeout"
        | "cargo_output_missing"
        | "cargo_spawn"
        | "cargo_wait"
        | "build_failed"
        | "cargo_artifact_missing"
        | "cargo_artifact_invalid"
        | "build_context_changed" => "swarmail_unavailable",
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
    let id = req.trial_id.clone().unwrap_or_else(new_trial_id);
    if req.concurrency.unwrap_or(1) != 1 {
        eprintln!("swarmail-evals: harness error: concurrency_unsupported");
        return finalize(&req, refused_row(&req, &id), None);
    }
    let admission = match crate::admission::Admission::acquire() {
        Ok(guard) => Arc::new(guard),
        Err(category) => {
            eprintln!("swarmail-evals: harness error: {category}");
            return finalize(&req, refused_row(&req, &id), None);
        }
    };
    let fallback = refused_row(&req, &id);
    // A dropped library waiter detaches this owner on a surviving runtime.
    // Admission encloses target recheck and retention as well as owned cleanup.
    tokio::spawn(run_admitted_trial(req, swarmail_bin, id, admission))
        .await
        .unwrap_or(fallback)
}

async fn run_admitted_trial(
    mut req: TrialRequest,
    swarmail_bin: Option<PathBuf>,
    id: String,
    admission: Arc<crate::admission::Admission>,
) -> TrialResult {
    eprintln!(
        "swarmail-evals: seed={} fixture_time={}",
        req.fixture.seed, req.fixture.fixture_time
    );
    let (mut row, snap) = execute_trial(&mut req, swarmail_bin, id, admission.clone()).await;
    if req
        .target_build
        .as_ref()
        .is_some_and(|t| t.recheck().is_err())
    {
        row.outcome = Outcome::HarnessError;
        row.failure_class = Some(FailureClass::HarnessError);
    }
    let row = finalize(&req, row, snap.as_ref());
    #[cfg(test)]
    if let Some(dir) = std::env::var_os("EVAL_TEST_FINALIZE_HOLD_DIR").map(PathBuf::from) {
        std::fs::write(dir.join("finalized"), &row.trial_id).unwrap();
        while !dir.join("finalize-release").exists() {
            tokio::task::yield_now().await;
        }
    }
    row
}

pub fn finalize(req: &TrialRequest, mut row: TrialResult, snap: Option<&Snapshot>) -> TrialResult {
    if row.validate().is_err()
        || crate::artifacts::encode(&row, crate::provenance::RESULT_LIMIT - 1).is_err()
    {
        row.outcome = Outcome::HarnessError;
        row.failure_class = Some(FailureClass::HarnessError);
    }
    if let Some(root) = &req.artifacts_dir
        && crate::artifacts::resolve_root(root)
            .and_then(|real| crate::artifacts::retain_trial(&real, &row, snap))
            .is_err()
    {
        row.outcome = Outcome::HarnessError;
        row.failure_class = Some(FailureClass::HarnessError);
        eprintln!("swarmail-evals: harness error: artifact_write_failed");
    }
    row
}

async fn execute_trial(
    req: &mut TrialRequest,
    swarmail_bin: Option<PathBuf>,
    trial_id: String,
    admission: Arc<crate::admission::Admission>,
) -> (TrialResult, Option<Snapshot>) {
    let (task, label) = match validate_request(req) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("swarmail-evals: harness error: request_invalid");
            return (refused_row(req, &trial_id), None);
        }
    };
    let caller = match CallerEnv::capture() {
        Ok(caller) => caller,
        Err(_) => {
            eprintln!("swarmail-evals: harness error: caller_context_unavailable");
            return (refused_row(req, &trial_id), None);
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
    let started = Arc::new(Mutex::new(None::<Instant>));
    let groups = Groups(
        Default::default(),
        Some(admission),
        #[cfg(all(test, target_os = "linux"))]
        None,
    );
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
            return (refused_row(req, &trial_id), None);
        }
    };
    let scratch = match private_tempdir("swarmail-eval-") {
        Ok(d) => Arc::new(d),
        Err(_) => {
            eprintln!("swarmail-evals: harness error: private_directory_unavailable");
            return (refused_row(req, &trial_id), None);
        }
    };
    let captured = make_provenance(req, req.target_build.as_ref(), None);
    req.assigned_agent = Some(captured.agent.clone());
    req.harness_identity = Some(captured.harness.clone());
    let target_capture = Arc::new(Mutex::new(req.target_build.clone()));
    let mut body = {
        let req_body = req.clone();
        let started = started.clone();
        let target_capture = target_capture.clone();
        let provenance = captured.clone();
        let (task, label, argv, trial_id, scratch_owner, groups, agents) = (
            task.clone(),
            label.clone(),
            req.argv.clone(),
            trial_id.clone(),
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
            let target = match req_body.target_build.clone() {
                Some(target) => Ok(target),
                None => match swarmail_bin
                    .map(|p| resolve_program(p.as_os_str(), &caller).map(Some))
                    .unwrap_or_else(selected_swarmail_bin)
                {
                    Ok(Some(path)) => Ok(crate::identity::TargetBuild::external(path)),
                    Ok(None) => build_owned_target(source_root(), &trial.groups).await,
                    Err(e) => Err(e),
                },
            };
            if let Ok(target) = &target {
                *target_capture.lock().unwrap_or_else(|e| e.into_inner()) = Some(target.clone());
            }
            let outcome = match target {
                Ok(target) => {
                    *started.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
                    run_body(
                        &task,
                        &label,
                        &argv,
                        &trial_id,
                        &target,
                        &req_body.fixture,
                        &provenance,
                        scratch_owner.path(),
                        &mut trial,
                        &mut exits,
                        &mut snap,
                    )
                    .await
                }
                Err(e) => Err(e),
            };
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
            eprintln!("swarmail-evals: harness error: trial_cleanup_unproved");
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
        req.target_build = target_capture
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        return (
            TrialResult {
                schema_version: RESULT_SCHEMA,
                concurrency: 1,
                trial_id,
                task: req.task.clone(),
                agent: req.agent.clone(),
                outcome: Outcome::HarnessError,
                failure_class: Some(FailureClass::HarnessError),
                wall_time_ms: started
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .map(|at| at.elapsed().as_millis() as u64)
                    .unwrap_or(0),
                agent_exit_status: if task.agent_count == 1 {
                    agent_exits.first().and_then(|e| e.code)
                } else {
                    None
                },
                agent_exits,
                confinement: CONFINEMENT.to_string(),
                provenance: make_provenance(req, req.target_build.as_ref(), None),
            },
            None,
        );
    };
    let (mut verdict, exits, snap, mut diagnostics) = match joined {
        Ok((outcome, exits, snap, mut diagnostics)) => {
            if outcome
                .as_ref()
                .is_err_and(|e| e == "build_context_changed")
            {
                req.target_source_failure = Some(crate::provenance::Reason::BuildContextChanged);
            }
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
        eprintln!("swarmail-evals: harness error: trial_cleanup_unproved");
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
    req.target_build = target_capture
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let row = TrialResult {
        schema_version: RESULT_SCHEMA,
        concurrency: 1,
        trial_id: trial_id.clone(),
        task: Some(task.id.clone()),
        agent: Some(label),
        outcome: verdict.outcome,
        failure_class: verdict.failure_class,
        wall_time_ms: started
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map(|at| at.elapsed().as_millis() as u64)
            .unwrap_or(0),
        agent_exit_status,
        agent_exits: exits,
        confinement: CONFINEMENT.to_string(),
        provenance: make_provenance(req, req.target_build.as_ref(), snap.as_ref()),
    };
    if row.outcome == Outcome::HarnessError {
        for d in &diagnostics {
            eprintln!("swarmail-evals: {d}");
        }
    }
    drop(scratch);
    (row, snap)
}

/// Resolve facts from actual inputs; absence is never a guessed identity.
pub fn make_provenance(
    req: &TrialRequest,
    target: Option<&crate::identity::TargetBuild>,
    snap: Option<&Snapshot>,
) -> crate::provenance::Provenance {
    use crate::provenance::*;
    let valid = validate_request(req).ok();
    let builtin = req.agent.as_deref().is_some_and(|s| BUILTINS.contains(&s));
    let argv = req.agent.as_deref().and_then(|label| {
        CallerEnv::capture()
            .ok()
            .and_then(|c| agent_argv(label, &req.argv, &c).ok())
    });
    let executable = argv
        .as_ref()
        .filter(|_| req.assigned_agent.is_none())
        .and_then(|a| a.first())
        .map(PathBuf::from);
    fn unavailable<T>() -> Availability<T> {
        Availability::unavailable(Reason::NotResolved)
    }
    let target_identity = target
        .map(|t| t.identity.clone())
        .unwrap_or(ExecutableIdentity {
            executable_sha256: Availability::unavailable(
                req.target_source_failure.unwrap_or(Reason::NotResolved),
            ),
            build_commit: Availability::unavailable(
                req.target_source_failure.unwrap_or(Reason::NotResolved),
            ),
            source_dirty: Availability::unavailable(
                req.target_source_failure.unwrap_or(Reason::NotResolved),
            ),
        });
    let command = if builtin {
        argv.as_ref()
            .map(|_| {
                Availability::present(
                    CommandIdentity {
                        algorithm: "public-argv-sha256-v1".into(),
                        sha256: public_argv(&[req.agent.clone().unwrap_or_default()]),
                    },
                    Source::Builtin,
                )
            })
            .unwrap_or_else(unavailable)
    } else if req.public_argv && valid.is_some() {
        Availability::present(
            CommandIdentity {
                algorithm: "public-argv-sha256-v1".into(),
                sha256: public_argv(
                    &req.argv
                        .iter()
                        .map(|v| v.to_str().expect("validated public UTF-8").to_string())
                        .collect::<Vec<_>>(),
                ),
            },
            Source::DeclaredPublicArgv,
        )
    } else {
        Availability::unavailable(Reason::PrivateArgv)
    };
    let chaos = snap
        .map(|s| {
            let probability = if valid.as_ref().is_some_and(|(t, _)| t.scenario.uses_chaos()) {
                100
            } else {
                0
            };
            let refusal_observed = s
                .deliveries
                .iter()
                .any(|d| d.attempts.iter().any(|a| a.code == 451));
            let retry_accepted = s
                .deliveries
                .iter()
                .any(|d| d.attempts.len() > 1 && d.status == DeliveryStatus::Delivered);
            Availability::present(
                ChaosObservation {
                    probability,
                    refusal_observed,
                    cleared: probability == 100 && retry_accepted,
                    retry_accepted,
                },
                Source::ObservedFixture,
            )
        })
        .unwrap_or_else(unavailable);
    let mut provenance = Provenance {
        fixture: req.fixture.clone(),
        matrix: req.matrix.clone(),
        task: TaskIdentity {
            catalogue_schema: 2,
            catalogue_sha256: catalogue_hash(),
            definition_sha256: valid
                .as_ref()
                .and_then(|(t, _)| task_hash(&t.id))
                .map(|v| Availability::present(v, Source::CatalogueDefinition))
                .unwrap_or_else(|| Availability::unavailable(Reason::RequestInvalid)),
            scorer_id: "business-score-v2".into(),
        },
        harness: req.harness_identity.clone().unwrap_or_else(|| {
            let mut h = crate::identity::harness();
            h.executable_sha256 = evals_executable()
                .map(|p| crate::identity::measured(&p))
                .unwrap_or_else(|_| Availability::unavailable(Reason::NotReadable));
            h
        }),
        target: TargetIdentity {
            executable_sha256: target_identity.executable_sha256,
            build_commit: target_identity.build_commit,
            source_dirty: target_identity.source_dirty,
            settings: if snap.is_some() {
                Availability::present(
                    ServerSettings {
                        max_per_inbox: 100000,
                        in_memory: true,
                        loopback: true,
                        port_zero: true,
                        inherited_settings_removed: true,
                    },
                    Source::ResolvedInput,
                )
            } else {
                Availability::unavailable(Reason::NotResolved)
            },
            chaos,
        },
        agent: AgentIdentity {
            tool_surface: ToolSurface::for_request(valid.is_some(), builtin, req.wrapper_profile),
            kind: if valid.is_none() {
                AgentKind::Invalid
            } else if builtin {
                AgentKind::Builtin
            } else {
                AgentKind::External
            },
            executable_sha256: executable
                .map(|p| crate::identity::measured(&p))
                .unwrap_or_else(|| Availability::unavailable(Reason::NotReadable)),
            command,
            model: if valid.is_none() {
                Availability::unavailable(Reason::RequestInvalid)
            } else if builtin {
                Availability::NotApplicable {
                    reason: Reason::Builtin,
                }
            } else {
                req.agent_model
                    .clone()
                    .map(|v| Availability::present(v, Source::OperatorDeclared))
                    .unwrap_or_else(|| Availability::unavailable(Reason::NotDeclared))
            },
            config_id: if builtin {
                req.agent
                    .clone()
                    .map(|v| Availability::present(v, Source::Builtin))
                    .unwrap_or_else(unavailable)
            } else {
                req.agent_config_id
                    .clone()
                    .map(|v| Availability::present(v, Source::OperatorDeclared))
                    .unwrap_or_else(|| Availability::unavailable(Reason::NotDeclared))
            },
        },
        limits: Limits {
            agent_deadline_ms: valid
                .as_ref()
                .map(|(t, _)| Availability::present(t.deadline_ms, Source::ResolvedInput))
                .unwrap_or_else(|| Availability::unavailable(Reason::RequestInvalid)),
            startup_ms: 30000,
            io_ms: 10000,
            shutdown_ms: 5000,
            matrix_child_envelope_ms: req
                .matrix
                .as_ref()
                .and_then(|_| valid.as_ref().map(|(t, _)| t.deadline_ms + 300000)),
        },
    };
    if valid.is_none() {
        provenance.agent.executable_sha256 = Availability::unavailable(Reason::RequestInvalid);
        provenance.agent.command = Availability::unavailable(Reason::RequestInvalid);
        provenance.agent.config_id = Availability::unavailable(Reason::RequestInvalid);
    }
    if let Some(agent) = &req.assigned_agent {
        provenance.agent = agent.clone();
    }
    provenance
}

fn recheck_digest(
    path: &Path,
    expected: &crate::provenance::Availability<String>,
) -> Result<(), String> {
    if let crate::provenance::Availability::Present { value, .. } = expected
        && crate::identity::file_hash(path)? == *value
    {
        return Ok(());
    }
    Err("executable_changed_or_unreadable".into())
}

// Linux only: these probes prove cleanup by PID/start-time identities read
// from /proc/<pid>/stat, which macOS does not have.
#[cfg(all(test, target_os = "linux"))]
mod test_probes {
    use super::*;
    use std::io::Write as _;
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
            ..Default::default()
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
                if std::env::var_os(CLEANUP_OBSERVE_ERROR_ENV).is_some()
                    || std::env::var_os(CLEANUP_FORCE_EXPIRY_ENV).is_some()
                {
                    control.failed_owners_only().await;
                }
                // Publish actual return before retaining the runtime for the
                // parent's live-work observation. This is not a cleanup fence.
                control.acknowledge_return().await;
            }
            row
        });
        assert_eq!(row.outcome, Outcome::HarnessError);
        println!("{}", serde_json::to_string(&row).unwrap());
    }

    async fn within<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::time::timeout(Duration::from_secs(25), future)
            .await
            .expect("cleanup regression hang watchdog")
    }

    async fn finished(handle: &tokio::task::AbortHandle) {
        within(async {
            while !handle.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    fn drain_timeout_retention(cargo: bool, suffix: bool) {
        let _trial_scope = crate::test_support::trial_scope();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let (progress, mut events) = tokio::sync::mpsc::unbounded_channel();
                let probe = Arc::new(DrainTestControl {
                    stdout: DrainHold::default(),
                    progress,
                });
                let admission = Arc::new(crate::admission::Admission::acquire().unwrap());
                let groups = Groups(Default::default(), Some(admission), Some(probe.clone()));
                let private = Arc::new(private_tempdir("swarmail-eval-drain-").unwrap());
                let private_path = private.path().to_path_buf();
                let mut cmd = tokio::process::Command::new("/bin/sh");
                cmd.args([
                    "-c",
                    if cargo {
                        r#"printf '%s\n' '{"reason":"build-finished","success":true}'"#
                    } else {
                        "printf helper-output"
                    },
                ]);
                let mut proc = if cargo {
                    spawn_owned_cargo(cmd, &groups, private.clone()).unwrap()
                } else {
                    spawn_extended(cmd, &groups, None, Some(private.clone()), false, true).unwrap()
                };
                within(probe.stdout.entered.notified()).await;
                assert!(within(proc.child.wait()).await.unwrap().success());
                assert!(group_members(&proc.group).unwrap().is_empty());
                let (stdout, stderr) = {
                    let drains = proc.group.drains.lock().unwrap();
                    (drains[0].abort_handle(), drains[1].abort_handle())
                };
                finished(&stderr).await;
                assert!(!stdout.is_finished());
                let later = Arc::new(DrainHold::default());
                let later_task = if suffix {
                    let hold = later.clone();
                    let task = tokio::spawn(async move { hold.wait().await });
                    let observer = task.abort_handle();
                    proc.group.drains.lock().unwrap().push(task);
                    within(later.entered.notified()).await;
                    Some(observer)
                } else {
                    None
                };
                let weak = Arc::downgrade(&proc.group);
                let mut verdict = proc.group.ensure_cleanup();
                let error = within(async {
                    loop {
                        if let Some(result) = verdict.borrow_and_update().clone() {
                            break result.unwrap_err();
                        }
                        verdict.changed().await.unwrap();
                    }
                })
                .await;
                assert!(error.starts_with("output pipe did not close after process group"));
                // Neither observers nor the cached verdict own the process group.
                // Wait for the failure owner to poll a retained drain or dispose
                // itself: an unscheduled owner cannot make the baseline pass.
                drop(proc);
                drop(groups);
                drop(private);
                let transition = within(events.recv()).await.unwrap();
                assert!(matches!(transition, "drain-pending" | "owner-disposed"));
                assert!(
                    matches!(crate::admission::Admission::acquire(), Err("concurrent_trials_refused")),
                    "pending stdout must retain admission after the failed cleanup owner progresses ({transition})"
                );
                assert!(private_path.is_dir(), "pending stdout retains private state");
                assert_eq!(weak.strong_count(), 1, "only the cleanup owner remains");

                probe.stdout.release.notify_one();
                finished(&stdout).await;
                if let Some(later_task) = later_task {
                    let transition = within(events.recv()).await.unwrap();
                    assert!(matches!(transition, "drain-pending" | "owner-disposed"));
                    assert!(!later_task.is_finished());
                    assert!(
                        matches!(crate::admission::Admission::acquire(), Err("concurrent_trials_refused")),
                        "the unvisited suffix must retain admission after stdout finishes"
                    );
                    assert!(private_path.is_dir(), "the suffix retains private state");
                    later.release.notify_one();
                    finished(&later_task).await;
                }
                assert_eq!(within(events.recv()).await, Some("owner-disposed"));
                assert_eq!(weak.strong_count(), 0);
                assert!(!private_path.exists(), "private state disposed before admission release");
                drop(crate::admission::Admission::acquire().unwrap());
                assert_eq!(verdict.borrow().as_ref().unwrap(), &Err(error));
                assert!(!verdict.has_changed().unwrap_or(false), "the failed verdict stays cached");
            });
    }

    #[test]
    fn cleanup_drain_timeout_retains_helper_stdout() {
        drain_timeout_retention(false, false);
    }

    #[test]
    fn cleanup_drain_timeout_retains_cargo_stdout() {
        drain_timeout_retention(true, false);
    }

    #[test]
    fn cleanup_drain_timeout_retains_unjoined_suffix() {
        drain_timeout_retention(false, true);
    }

    #[test]
    fn sequential_library_probe_entrypoint() {
        let Some(dir) = std::env::var_os("EVAL_TEST_LIBRARY_DROP_DIR").map(PathBuf::from) else {
            return;
        };
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let req = TrialRequest {
                task: Some("signup".into()),
                agent: Some("held-library-agent".into()),
                artifacts_dir: Some(dir.join("artifacts")),
                argv: vec!["/bin/sh".into(), dir.join("agent.sh").into_os_string()],
                ..Default::default()
            };
            let waiter = tokio::spawn(run_trial(
                req,
                Some(PathBuf::from(std::env::var_os(BIN_ENV).unwrap())),
            ));
            let deadline = Instant::now() + Duration::from_secs(20);
            while !dir.join("ready").exists() {
                assert!(Instant::now() < deadline);
                tokio::task::yield_now().await;
            }
            waiter.abort();
            assert!(waiter.await.unwrap_err().is_cancelled());
            let refused = run_trial(
                TrialRequest {
                    task: Some("signup".into()),
                    agent: Some("null".into()),
                    ..Default::default()
                },
                None,
            )
            .await;
            assert_eq!(refused.outcome, Outcome::HarnessError);
            assert_eq!(refused.wall_time_ms, 0);
            assert!(refused.agent_exits.is_empty());
            assert_eq!(refused.concurrency, 1);
            assert!(matches!(
                crate::admission::Admission::acquire(),
                Err("concurrent_trials_refused")
            ));
            std::fs::write(dir.join("waiter-dropped"), b"dropped").unwrap();
            loop {
                match crate::admission::Admission::acquire() {
                    Ok(guard) => {
                        drop(guard);
                        break;
                    }
                    Err("concurrent_trials_refused") => {
                        assert!(Instant::now() < deadline);
                        tokio::task::yield_now().await;
                    }
                    Err(other) => panic!("unexpected admission: {other}"),
                }
            }
            let home = std::fs::read_to_string(dir.join("ready")).unwrap();
            assert!(
                !Path::new(&home).exists(),
                "private state disposed before admission release"
            );
        });
    }

    #[test]
    fn sequential_admission_library_and_cleanup_lifetime() {
        let _trial_scope = crate::test_support::trial_scope();
        let dir = tempfile::tempdir().unwrap();
        let script = format!(
            "#!/bin/sh\nprintf '%s' \"$HOME\" > '{}'\nwhile [ ! -e '{}' ]; do :; done\n",
            dir.path().join("ready").display(),
            dir.path().join("release").display()
        );
        std::fs::write(dir.path().join("agent.sh"), script).unwrap();
        let output = std::fs::File::create(dir.path().join("output")).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "runner::test_probes::sequential_library_probe_entrypoint",
                "--nocapture",
            ])
            .env("EVAL_TEST_LIBRARY_DROP_DIR", dir.path())
            .env("EVAL_TEST_FINALIZE_HOLD_DIR", dir.path())
            .env(BIN_ENV, resolve_swarmail_bin().unwrap())
            .env(
                "EVAL_TEST_RUNNER_EXECUTABLE",
                std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("swarmail-evals"),
            )
            .stdout(output.try_clone().unwrap())
            .stderr(output);
        struct LibraryProbe(std::process::Child, PathBuf);
        impl Drop for LibraryProbe {
            fn drop(&mut self) {
                let _ = std::fs::write(self.1.join("release"), b"release");
                let _ = std::fs::write(self.1.join("finalize-release"), b"release");
                let _ = self.0.wait();
            }
        }
        let mut probe = LibraryProbe(command.spawn().unwrap(), dir.path().to_path_buf());
        eventually("dropped library waiter", || {
            dir.path().join("waiter-dropped").exists()
        });
        assert!(matches!(
            crate::admission::Admission::acquire(),
            Err("concurrent_trials_refused")
        ));
        std::fs::write(dir.path().join("release"), b"release").unwrap();
        eventually("detached trial finalized", || {
            dir.path().join("finalized").exists()
        });
        assert!(
            matches!(
                crate::admission::Admission::acquire(),
                Err("concurrent_trials_refused")
            ),
            "admission encloses finalization after execute_trial returns"
        );
        let trial_id = std::fs::read_to_string(dir.path().join("finalized")).unwrap();
        let retained =
            crate::artifacts::trial(&dir.path().join("artifacts").join(trial_id)).unwrap();
        assert_eq!(retained.row.outcome, Outcome::Fail);
        assert_eq!(retained.row.concurrency, 1);
        std::fs::write(dir.path().join("finalize-release"), b"release").unwrap();
        assert!(
            probe.0.wait().unwrap().success(),
            "{}",
            std::fs::read_to_string(dir.path().join("output")).unwrap()
        );
        drop(crate::admission::Admission::acquire().unwrap());
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
        let _trial_scope = crate::test_support::trial_scope();
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
  /bin/sh -c 'trap "" TERM; : > "$1"; exec sleep 1000' sh "$EVAL_TEST_RUNNER_PIDS.child-ready" &
  child=$!
  while [ ! -f "$EVAL_TEST_RUNNER_PIDS.child-ready" ]; do :; done
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
        if matches!(fault, Some("observation_error" | "forced_expiry")) {
            // The surviving descendant must not keep ordinary pipe drains (and
            // their independent lifetime captures) alive in the retention proof.
            script_text = script_text.replace("exec sleep 1000", "exec sleep 1000 >/dev/null 2>&1");
        }
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
        let _trial_scope = crate::test_support::trial_scope();
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
        let _trial_scope = crate::test_support::trial_scope();
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
                assert!(
                    matches!(
                        crate::admission::Admission::acquire(),
                        Err("concurrent_trials_refused")
                    ),
                    "admission remains held through acknowledged pending cleanup"
                );
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
        let _trial_scope = crate::test_support::trial_scope();
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
        let _trial_scope = crate::test_support::trial_scope();
        for (fault, expected) in [
            ("observation_error", "cleanup_observation_failed"),
            ("forced_expiry", "cleanup_expired"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = launch_probe(
                dir.path(),
                Some(fault),
                false,
                false,
                true,
                false,
                Some(false),
            );
            probe.record_owned_from_tree();
            let direct_agent = probe.owned.first().unwrap().clone();
            std::fs::write(dir.path().join("fault-release"), b"release").unwrap();
            eventually("the actual cleanup join to become Pending", || {
                probe.pending.is_file()
            });
            probe.await_return();
            assert!(
                probe.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "the failure verdict is observed on a surviving runtime"
            );
            assert!(
                identity(direct_agent.pid).is_none(),
                "direct agent child was reaped: {direct_agent:?}"
            );
            assert!(
                probe.owned.iter().any(alive),
                "the test observed a real surviving owned process"
            );
            let bytes = std::fs::read(dir.path().join("runtime-env")).unwrap();
            let home = bytes
                .split(|byte| *byte == 0)
                .find_map(|entry| entry.strip_prefix(b"HOME="))
                .map(|value| PathBuf::from(OsStr::from_bytes(value)))
                .unwrap();
            assert!(
                matches!(
                    crate::admission::Admission::acquire(),
                    Err("concurrent_trials_refused")
                ),
                "failed cleanup must retain admission with live owned work: {fault}"
            );
            assert!(
                home.is_dir(),
                "failed cleanup retains private state: {fault}"
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
            eventually(
                "failed owner to release admission after actual cleanup",
                || crate::admission::Admission::acquire().is_ok(),
            );
            assert!(
                !home.exists(),
                "private state disposed before reuse: {fault}"
            );
            probe.acknowledge_return();
            assert!(probe.wait().success());
            assert!(probe.finish().is_ok());
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
        let _trial_scope = crate::test_support::trial_scope();
        fault_privacy_case("server_death", "service_exited_during_trial");
    }

    #[test]
    fn a_runner_panic_cleans_up_and_still_emits_one_row() {
        let _trial_scope = crate::test_support::trial_scope();
        fault_privacy_case("panic", "runner_panicked");
    }

    #[test]
    fn trial_runtime_owner_survives_aborted_body() {
        let _trial_scope = crate::test_support::trial_scope();
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
        let _trial_scope = crate::test_support::trial_scope();
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
        let _trial_scope = crate::test_support::trial_scope();
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
        // Linux only: macOS file systems (APFS, HFS+) refuse a non-UTF-8
        // file name with EILSEQ, so such a program cannot exist there.
        #[cfg(target_os = "linux")]
        let odd_name =
            <OsString as std::os::unix::ffi::OsStringExt>::from_vec(b"nonutf8-\xff-agent".to_vec());
        #[cfg(target_os = "linux")]
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
        #[cfg(target_os = "linux")]
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

#[cfg(test)]
mod identity_tests {
    use super::{ISOLATION_SMTP_IDENTITY, ROUTE_DOMAIN, identities};

    /// The routed label rule of decision 0112 §1: one label of
    /// `[a-z0-9-]{1,63}`.
    fn is_routed_label(label: &str) -> bool {
        (1..=63).contains(&label.len())
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    }

    /// Inbox-isolation: three accounts share the one application login and
    /// are separated by routed recipients whose label is the account inbox.
    #[test]
    fn isolation_accounts_share_one_login_and_route_by_recipient() {
        let ids = identities("trial-20261031-ABCDEF12", 3);
        assert_eq!(ids.len(), 3);
        let mut inboxes = std::collections::HashSet::new();
        for id in &ids {
            assert_eq!(id.smtp_identity, ISOLATION_SMTP_IDENTITY);
            assert!(is_routed_label(&id.inbox), "{}", id.inbox);
            let (_, host) = id.identity.rsplit_once('@').unwrap();
            assert_eq!(host, format!("{}.{ROUTE_DOMAIN}", id.inbox));
            assert!(inboxes.insert(id.inbox.clone()));
        }
        assert_eq!(inboxes.len(), 3);
    }

    /// Every other task keeps its pre-0112 wire shape: an `@example.test`
    /// identity, which the route domain never touches, authenticating as its
    /// inbox.
    #[test]
    fn single_agent_tasks_keep_their_identity_and_authenticate_as_the_inbox() {
        let ids = identities("trial-20261031-abcdef12", 1);
        assert_eq!(ids.len(), 1);
        assert!(ids[0].identity.ends_with("@example.test"));
        assert!(!ids[0].identity.ends_with(&format!(".{ROUTE_DOMAIN}")));
        assert_eq!(ids[0].smtp_identity, ids[0].inbox);
        assert_eq!(ids[0].inbox, "eval-abcdef12-ada");
    }
}
