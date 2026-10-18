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
use std::net::SocketAddr;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

/// The result row's schema.
pub const RESULT_SCHEMA: u32 = 1;
/// Every trial is unconfined until brokkr's MCP broker lands (0055 §7).
pub const CONFINEMENT: &str = "unconfined";
/// The env var naming the Swarmail binary under test.
pub const SWARMAIL_BIN_ENV: &str = "SWARMAIL_BIN";
/// Test seam: inject a runner fault after the agents start (`panic`,
/// `server_death`). Used only by the lifecycle proofs.
pub const FAULT_ENV: &str = "EVAL_RUNNER_FAULT";
/// The built-in agent labels.
pub const BUILTINS: [&str; 2] = ["oracle", "null"];

const STARTUP_DEADLINE: Duration = Duration::from_secs(30);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);
const LOG_RETENTION: usize = 64 * 1024;

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
pub struct Groups(Arc<Mutex<Vec<i32>>>);

impl Groups {
    fn add(&self, pgid: i32) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).push(pgid);
    }

    /// SIGKILL every owned group (the panic path's last resort).
    pub fn kill_all(&self) {
        for &pgid in self.0.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            signal_group(pgid, libc::SIGKILL);
        }
    }
}

fn signal_group(pgid: i32, sig: i32) {
    if pgid > 0 {
        // SAFETY: kill(2) on a negative pid signals a process group; an
        // ESRCH for an already-empty group is the expected outcome.
        unsafe {
            libc::kill(-pgid, sig);
        }
    }
}

/// Bounded log retention: the newest `LOG_RETENTION` bytes survive, and
/// the pipe is drained to EOF however chatty the child is.
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
) {
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
    });
}

/// A spawned child in its own process group.
pub struct Proc {
    pub label: String,
    pub child: Child,
    pub pgid: i32,
    pub log: Log,
    lines: mpsc::UnboundedReceiver<String>,
}

fn spawn(mut cmd: Command, label: &str, groups: &Groups) -> std::io::Result<Proc> {
    cmd.process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn()?;
    let pgid = child.id().map(|p| p as i32).unwrap_or(0);
    groups.add(pgid);
    let log = Log::default();
    let (tx, rx) = mpsc::unbounded_channel();
    drain(child.stdout.take().expect("piped"), log.clone(), Some(tx));
    drain(child.stderr.take().expect("piped"), log.clone(), None);
    Ok(Proc {
        label: label.to_string(),
        child,
        pgid,
        log,
        lines: rx,
    })
}

impl Proc {
    /// Read stdout lines until `parse` accepts one, within the startup
    /// deadline; the child exiting first is an error.
    async fn announce<T>(&mut self, parse: impl Fn(&str) -> Option<T>) -> Result<T, String> {
        let deadline = tokio::time::Instant::now() + STARTUP_DEADLINE;
        loop {
            tokio::select! {
                line = self.lines.recv() => match line {
                    Some(line) => if let Some(v) = parse(&line) { return Ok(v) },
                    None => return Err(format!("{} exited before announcing its addresses", self.label)),
                },
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(format!("{} did not announce within {STARTUP_DEADLINE:?}", self.label));
                }
            }
        }
    }

    /// SIGTERM the group, wait out the grace, SIGKILL, reap; then sweep the
    /// group once more for descendants that outlived their leader.
    async fn stop(&mut self) -> Option<ExitStatus> {
        signal_group(self.pgid, libc::SIGTERM);
        let status = match tokio::time::timeout(SHUTDOWN_GRACE, self.child.wait()).await {
            Ok(s) => s.ok(),
            Err(_) => {
                signal_group(self.pgid, libc::SIGKILL);
                self.child.wait().await.ok()
            }
        };
        signal_group(self.pgid, libc::SIGKILL);
        status
    }
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
    diagnostics: Vec<String>,
}

impl Trial {
    async fn stop_servers(&mut self) {
        // Fixture first (it may hold SMTP sessions), then Swarmail.
        while let Some(mut p) = self.servers.pop() {
            p.stop().await;
            let log = p.log.text();
            if !log.is_empty() {
                self.diagnostics
                    .push(format!("--- {} log (tail) ---\n{log}", p.label));
            }
        }
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
fn agent_argv(label: &str, argv: &[String]) -> Result<Vec<String>, String> {
    if BUILTINS.contains(&label) {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
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
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
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
                log,
                ..
            }) => {
                agent_groups.push(pgid);
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
        match std::env::var(FAULT_ENV).as_deref() {
            Ok("panic") => panic!("injected runner fault"),
            Ok("server_death") => signal_group(trial.servers[0].pgid, libc::SIGKILL),
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
    // Every agent group is terminated and reaped on every ending, and swept
    // for descendants that outlived their leader.
    for &pgid in &agent_groups {
        signal_group(pgid, libc::SIGTERM);
    }
    let reaped = tokio::time::timeout(SHUTDOWN_GRACE, async {
        while let Some(joined) = waits.join_next().await {
            if let Ok((i, status)) = joined {
                statuses[i] = status;
            }
        }
    })
    .await;
    for &pgid in &agent_groups {
        signal_group(pgid, libc::SIGKILL);
    }
    if reaped.is_err() {
        while let Some(joined) = waits.join_next().await {
            if let Ok((i, status)) = joined {
                statuses[i] = status;
            }
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
    let started = Instant::now();
    let groups = Groups::default();
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
        let (task, label, argv, trial_id, bin, scratch_path, groups) = (
            task.clone(),
            label.clone(),
            req.argv.clone(),
            trial_id.clone(),
            bin.clone(),
            scratch.path().to_path_buf(),
            groups.clone(),
        );
        tokio::spawn(async move {
            let mut trial = Trial {
                groups,
                servers: Vec::new(),
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
            trial.stop_servers().await;
            (outcome, exits, snap, trial.diagnostics)
        })
    };
    let joined = tokio::select! {
        joined = &mut body => Some(joined),
        _ = term.recv() => None,
        _ = int.recv() => None,
    };
    let Some(joined) = joined else {
        // Interrupted: stop the body (its children are kill-on-drop),
        // then terminate every owned group and give them the grace.
        body.abort();
        let _ = body.await;
        groups.kill_all();
        eprintln!("swarmail-evals: harness error: the trial was interrupted");
        return TrialResult {
            wall_time_ms: started.elapsed().as_millis() as u64,
            ..refused_row(&req, &trial_id)
        };
    };
    let (verdict, exits, snap, mut diagnostics) = match joined {
        Ok((outcome, exits, snap, diagnostics)) => {
            let verdict = match (&outcome, &snap) {
                (Err(e), _) => {
                    eprintln!("swarmail-evals: harness error: {e}");
                    Verdict::harness_error()
                }
                (Ok(true), _) => Verdict::fail(FailureClass::Budget),
                (Ok(false), Some(snap)) => match tasks::score(&task, snap) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("swarmail-evals: malformed snapshot: {e}");
                        Verdict::harness_error()
                    }
                },
                (Ok(false), None) => Verdict::harness_error(),
            };
            (verdict, exits, snap, diagnostics)
        }
        Err(join) => {
            // A panic: the body's handles are gone, so its groups are
            // killed through the shared registry.
            groups.kill_all();
            eprintln!("swarmail-evals: harness error: the runner panicked: {join}");
            (
                Verdict::harness_error(),
                Vec::new(),
                None,
                vec![format!("runner panicked: {join}")],
            )
        }
    };
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
