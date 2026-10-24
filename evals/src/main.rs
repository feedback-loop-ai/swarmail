//! `swarmail-evals`: the harness's one executable. stdout carries only
//! machine output (result rows, the catalogue, reports, a server's address
//! line); every diagnostic goes to stderr.

use clap::{Parser, Subcommand};
use std::io::Write;
use std::net::SocketAddr;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitCode, Stdio};
use std::time::{Duration, Instant};
use swarmail_evals::runner::{self, TrialRequest, TrialResult};
use swarmail_evals::tasks::{self, Catalogue, Outcome};
use swarmail_evals::{fixture, oracle, report, state};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(Parser)]
#[command(
    name = "swarmail-evals",
    version,
    about = "The Swarmail agent eval harness (decision 0055)"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the v1 task catalogue (schema version 1) as JSON.
    Tasks,
    /// Run the fixture app against a Swarmail SMTP address. Reads its admin
    /// secret from EVAL_FIXTURE_ADMIN_SECRET; prints
    /// `fixture public=ADDR admin=ADDR` once it listens.
    Fixture {
        #[arg(long)]
        smtp: SocketAddr,
        #[arg(long)]
        task: String,
        #[arg(long)]
        trial_id: String,
    },
    /// Run a built-in agent (`oracle` or `null`) from the EVAL_* environment.
    Agent { name: String },
    /// Run one trial and print exactly one JSON result line.
    /// Exit 0 pass, 1 fail, 2 harness error.
    Trial {
        #[arg(long)]
        task: Option<String>,
        /// `oracle`, `null`, or a label for the external command after `--`.
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        deadline_ms: Option<u64>,
        /// Keep state.json, result.json, diagnostics and metadata here.
        #[arg(long)]
        artifacts_dir: Option<PathBuf>,
        /// The external agent's program and literal arguments (no shell).
        #[arg(last = true)]
        argv: Vec<String>,
    },
    /// Run tasks × agents × trials sequentially, one JSON line per requested slot.
    /// SIGTERM or SIGINT stops admission, waits for active cleanup and exits 2.
    /// Otherwise exit 2 for harness errors, 1 for failures, and 0 for all-pass.
    Matrix {
        /// Comma-separated task ids.
        #[arg(long, value_delimiter = ',', required = true)]
        tasks: Vec<String>,
        /// A JSON array of {"label": ..., "argv": [...]}.
        #[arg(long)]
        agents: PathBuf,
        #[arg(long)]
        trials: u32,
        #[arg(long)]
        deadline_ms: Option<u64>,
    },
    /// Re-apply a task's check to a retained state.json.
    Check {
        #[arg(long)]
        task: String,
        #[arg(long)]
        state: PathBuf,
    },
    /// Aggregate JSONL result rows (`-` reads stdin).
    Report {
        #[arg(long)]
        input: String,
    },
}

fn print_line(value: &impl serde::Serialize) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", serde_json::to_string(value).expect("serializes"));
    let _ = out.flush();
}

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("swarmail-evals: {msg}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime");
    match cli.command {
        Cmd::Tasks => {
            let catalogue = tasks::catalogue();
            if let Err(e) = tasks::validate_catalogue(&catalogue) {
                return fail(e);
            }
            print_line(&Catalogue {
                schema_version: tasks::CATALOGUE_SCHEMA,
                tasks: catalogue,
            });
            ExitCode::SUCCESS
        }
        Cmd::Fixture {
            smtp,
            task,
            trial_id,
        } => match rt.block_on(fixture::run_cli(smtp, &task, &trial_id)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(e),
        },
        Cmd::Agent { name } => match name.as_str() {
            "null" => ExitCode::SUCCESS,
            "oracle" => {
                let env = match oracle::AgentEnv::from_env() {
                    Ok(env) => env,
                    Err(e) => return fail(e),
                };
                match rt.block_on(oracle::run_oracle(&env)) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("oracle: {e}");
                        ExitCode::from(1)
                    }
                }
            }
            other => fail(format!("no built-in agent named {other}")),
        },
        Cmd::Trial {
            task,
            agent,
            deadline_ms,
            artifacts_dir,
            argv,
        } => {
            let req = TrialRequest {
                task,
                agent,
                argv,
                deadline_ms,
                artifacts_dir,
            };
            let row = rt.block_on(runner::run_trial(req, None));
            print_line(&row);
            ExitCode::from(row.exit_code() as u8)
        }
        Cmd::Matrix {
            tasks: ids,
            agents,
            trials,
            deadline_ms,
        } => rt.block_on(matrix(ids, agents, trials, deadline_ms)),
        Cmd::Check { task, state } => {
            let Some(task) = tasks::find(&task) else {
                return fail(format!("unknown task {task}"));
            };
            let verdict = std::fs::read(&state)
                .map_err(|e| e.to_string())
                .and_then(|b| {
                    serde_json::from_slice::<state::Snapshot>(&b).map_err(|e| e.to_string())
                })
                .and_then(|snap| tasks::score(&task, &snap));
            match verdict {
                Ok(v) => {
                    print_line(&serde_json::json!({
                        "task": task.id,
                        "outcome": v.outcome,
                        "failure_class": v.failure_class,
                    }));
                    ExitCode::from(if v.outcome == Outcome::Pass { 0 } else { 1 })
                }
                Err(e) => {
                    print_line(&serde_json::json!({
                        "task": task.id,
                        "outcome": Outcome::HarnessError,
                        "failure_class": state::FailureClass::HarnessError,
                    }));
                    fail(format!("unscorable state (harness error): {e}"))
                }
            }
        }
        Cmd::Report { input } => {
            let text = if input == "-" {
                let mut s = String::new();
                match std::io::Read::read_to_string(&mut std::io::stdin(), &mut s) {
                    Ok(_) => s,
                    Err(e) => return fail(e),
                }
            } else {
                match std::fs::read_to_string(&input) {
                    Ok(s) => s,
                    Err(e) => return fail(format!("{input}: {e}")),
                }
            };
            match report::parse_rows(&text) {
                Ok(rows) => {
                    print_line(&report::report(&rows));
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
    }
}

const MATRIX_OUTPUT_LIMIT: usize = 64 * 1024;
const MATRIX_OUTPUT_EOF_BOUND: Duration = Duration::from_secs(5);

struct CapturedOutput {
    bytes: Vec<u8>,
    overflow: bool,
}

struct ChildRun {
    row: Option<TrialResult>,
    interrupted: bool,
    failure: Option<String>,
    stop_admission: bool,
    elapsed_ms: u64,
}

fn install_matrix_signals()
-> std::io::Result<(tokio::signal::unix::Signal, tokio::signal::unix::Signal)> {
    if std::env::var_os("EVAL_TEST_MATRIX_SIGNAL_SETUP_FAIL").is_some() {
        return Err(std::io::Error::other(
            "injected matrix signal setup failure",
        ));
    }
    let term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    Ok((term, int))
}

const MATRIX_ADMISSION_HOLD_ENV: &str = "EVAL_TEST_MATRIX_ADMISSION_HOLD_DIR";
const MATRIX_FINAL_HOLD_ENV: &str = "EVAL_TEST_MATRIX_FINAL_HOLD_DIR";
const MATRIX_COMMIT_HOLD_ENV: &str = "EVAL_TEST_MATRIX_COMMIT_HOLD_DIR";

async fn matrix_test_hold(
    env: &str,
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
    interrupted: &mut bool,
) {
    let Some(dir) = std::env::var_os(env).map(PathBuf::from) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if std::fs::write(dir.join("entered"), b"waiting").is_err() {
        return;
    }
    while !dir.join("release").exists() {
        tokio::select! {
            biased;
            _ = term.recv() => {
                *interrupted = true;
                let _ = std::fs::write(dir.join("signal-term"), b"seen");
            },
            _ = int.recv() => {
                *interrupted = true;
                let _ = std::fs::write(dir.join("signal-int"), b"seen");
            },
            _ = tokio::time::sleep(Duration::from_millis(10)) => {},
        }
    }
}

async fn pending_matrix_signal(
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
) -> Option<i32> {
    tokio::select! {
        biased;
        _ = term.recv() => Some(libc::SIGTERM),
        _ = int.recv() => Some(libc::SIGINT),
        _ = tokio::task::yield_now() => None,
    }
}

fn matrix_slot<'a>(
    ids: &'a [String],
    specs: &'a [report::AgentSpec],
    trials: u32,
    index: usize,
) -> (&'a str, &'a report::AgentSpec) {
    let repetitions = trials as usize;
    let per_task = specs.len() * repetitions;
    let task_index = index / per_task;
    let within_task = index % per_task;
    let agent_index = within_task / repetitions;
    (&ids[task_index], &specs[agent_index])
}

fn matrix_refused(id: &str, spec: &report::AgentSpec) -> TrialResult {
    runner::refused_row(
        &TrialRequest {
            task: Some(id.to_string()),
            agent: Some(spec.label.clone()),
            ..Default::default()
        },
        &runner::new_trial_id(),
    )
}

struct OutputMessage {
    bytes: Vec<u8>,
    ack: tokio::sync::oneshot::Sender<Result<(), String>>,
}

struct OutputWriter {
    tx: std::sync::mpsc::SyncSender<OutputMessage>,
    wake: std::sync::Arc<tokio::sync::Notify>,
}

fn start_output_writer() -> OutputWriter {
    let (tx, rx) = std::sync::mpsc::sync_channel::<OutputMessage>(1);
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    let thread_wake = wake.clone();
    std::thread::spawn(move || {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        while let Ok(message) = rx.recv() {
            thread_wake.notify_one();
            let result = out
                .write_all(&message.bytes)
                .and_then(|_| out.flush())
                .map_err(|e| format!("stdout write failed: {e}"));
            let failed = result.is_err();
            let _ = message.ack.send(result);
            if failed {
                break;
            }
        }
    });
    OutputWriter { tx, wake }
}

/// `Some(signal)` means the row is not queued yet and the caller must first
/// apply cancellation to that uncommitted slot. A queued row is immutable.
async fn write_row(
    row: &TrialResult,
    writer: &OutputWriter,
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
    interrupted: &mut bool,
) -> Result<Option<i32>, String> {
    if let Some(signal) = pending_matrix_signal(term, int).await {
        return Ok(Some(signal));
    }
    let mut bytes = serde_json::to_vec(row).expect("trial result serializes");
    bytes.push(b'\n');
    let (ack_tx, mut ack_rx) = tokio::sync::oneshot::channel();
    let mut message = Some(OutputMessage { bytes, ack: ack_tx });
    loop {
        match writer
            .tx
            .try_send(message.take().expect("message remains until queued"))
        {
            Ok(()) => break,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                return Err("stdout writer stopped".into());
            }
            Err(std::sync::mpsc::TrySendError::Full(returned)) => {
                message = Some(returned);
                tokio::select! {
                    biased;
                    _ = term.recv() => return Ok(Some(libc::SIGTERM)),
                    _ = int.recv() => return Ok(Some(libc::SIGINT)),
                    _ = writer.wake.notified() => {},
                }
            }
        }
    }
    let deadline = tokio::time::Instant::now() + MATRIX_OUTPUT_EOF_BOUND;
    loop {
        tokio::select! {
            biased;
            _ = term.recv() => *interrupted = true,
            _ = int.recv() => *interrupted = true,
            result = &mut ack_rx => {
                result.map_err(|_| "stdout writer vanished".to_string())??;
                return Ok(None);
            }
            _ = tokio::time::sleep_until(deadline) => return Err("stdout made no progress for five seconds".into()),
        }
    }
}

fn matching_trial_row(
    captured: CapturedOutput,
    status: std::process::ExitStatus,
    id: &str,
    spec: &report::AgentSpec,
    interrupted: bool,
) -> Result<TrialResult, String> {
    if captured.overflow {
        return Err("trial result output exceeded 64 KiB".into());
    }
    let text = String::from_utf8(captured.bytes)
        .map_err(|e| format!("trial result output is not UTF-8: {e}"))?;
    let rows: Vec<_> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if rows.len() != 1 {
        return Err(format!(
            "trial printed {} nonempty result lines",
            rows.len()
        ));
    }
    let row: TrialResult = serde_json::from_str(rows[0])
        .map_err(|e| format!("trial printed invalid result JSON: {e}"))?;
    row.validate()
        .map_err(|e| format!("trial result violates schema: {e}"))?;
    if row.task.as_deref() != Some(id) || row.agent.as_deref() != Some(spec.label.as_str()) {
        return Err("trial result labels do not match the requested task and agent".into());
    }
    if !interrupted && status.code() != Some(row.exit_code()) {
        return Err(format!(
            "trial exit {:?} does not match result outcome {}",
            status.code().or_else(|| status.signal().map(|s| -s)),
            row.exit_code()
        ));
    }
    Ok(row)
}

async fn capture_stdout<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
) -> std::io::Result<CapturedOutput> {
    let mut bytes = Vec::with_capacity(MATRIX_OUTPUT_LIMIT);
    let mut overflow = false;
    let mut chunk = [0_u8; 8192];
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        let remaining = MATRIX_OUTPUT_LIMIT.saturating_sub(bytes.len());
        let kept = remaining.min(count);
        bytes.extend_from_slice(&chunk[..kept]);
        overflow |= kept != count;
    }
    Ok(CapturedOutput { bytes, overflow })
}

fn signal_trial(pid: u32) -> Result<(), String> {
    if unsafe { libc::kill(pid as i32, libc::SIGTERM) } == 0 {
        return Ok(());
    }
    let e = std::io::Error::last_os_error();
    if e.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(format!("cannot forward SIGTERM to active trial {pid}: {e}"))
    }
}

async fn run_child(
    exe: &std::path::Path,
    bin: &std::path::Path,
    id: &str,
    spec: &report::AgentSpec,
    deadline_ms: Option<u64>,
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
) -> ChildRun {
    let started = Instant::now();
    let mut cmd = Command::new(exe);
    cmd.process_group(0)
        .args(["trial", "--task", id, "--agent", &spec.label])
        .env(runner::SWARMAIL_BIN_ENV, bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    if let Some(ms) = deadline_ms {
        cmd.args(["--deadline-ms", &ms.to_string()]);
    }
    if !spec.argv.is_empty() {
        cmd.arg("--").args(&spec.argv);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            return ChildRun {
                row: None,
                interrupted: false,
                failure: Some(format!("cannot run trial: {e}")),
                stop_admission: false,
                elapsed_ms: started.elapsed().as_millis() as u64,
            };
        }
    };
    let pid = child.id().expect("spawned trial has a PID");
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut reader = tokio::spawn(capture_stdout(stdout));
    let mut wait = Box::pin(child.wait());
    let mut interrupted = false;
    let mut forwarded = false;
    let mut failure = None;
    let status = loop {
        tokio::select! {
            biased;
            _ = term.recv() => {
                interrupted = true;
                if !forwarded { forwarded = true; if let Err(e) = signal_trial(pid) { failure = Some(e); } }
            }
            _ = int.recv() => {
                interrupted = true;
                if !forwarded { forwarded = true; if let Err(e) = signal_trial(pid) { failure = Some(e); } }
            }
            result = &mut wait => match result {
                Ok(status) => break Some(status),
                Err(e) => { failure = Some(format!("cannot reap trial {pid}: {e}")); break None; }
            }
        }
    };
    let captured = if status.is_some() {
        let deadline = tokio::time::Instant::now() + MATRIX_OUTPUT_EOF_BOUND;
        loop {
            tokio::select! {
                biased;
                _ = term.recv() => interrupted = true,
                _ = int.recv() => interrupted = true,
                result = &mut reader => break match result {
                    Ok(Ok(output)) => Some(output),
                    Ok(Err(e)) => { failure = Some(format!("cannot read trial stdout: {e}")); None }
                    Err(e) => { failure = Some(format!("trial stdout reader failed: {e}")); None }
                },
                _ = tokio::time::sleep_until(deadline) => {
                    failure = Some("trial stdout remained open more than five seconds after child exit".into());
                    reader.abort();
                    let _ = reader.await;
                    break None;
                }
            }
        }
    } else {
        None
    };
    let output_available = captured.is_some();
    let row = match (captured, status) {
        (Some(captured), Some(status)) => {
            match matching_trial_row(captured, status, id, spec, interrupted) {
                Ok(row) => Some(row),
                Err(e) => {
                    failure.get_or_insert(e);
                    None
                }
            }
        }
        _ => None,
    };
    let stop_admission = status.is_none() || !output_available || failure.is_some();
    ChildRun {
        row,
        interrupted,
        failure,
        stop_admission,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

fn parse_cargo_artifact(line: &str) -> Option<PathBuf> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value["reason"] == "compiler-artifact"
        && value["target"]["name"] == "swarmail"
        && value["target"]["kind"]
            .as_array()?
            .iter()
            .any(|kind| kind == "bin")
    {
        value["executable"].as_str().map(PathBuf::from)
    } else {
        None
    }
}

async fn cleanup_cargo(
    cargo: &runner::Proc,
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
    interrupted: &mut bool,
) -> Result<(), String> {
    let cleanup = cargo.cleanup();
    tokio::pin!(cleanup);
    loop {
        tokio::select! {
            biased;
            result = &mut cleanup => return result,
            _ = term.recv() => *interrupted = true,
            _ = int.recv() => *interrupted = true,
        }
    }
}

async fn output_deadline(at: Option<tokio::time::Instant>) {
    if let Some(at) = at {
        tokio::time::sleep_until(at).await;
    } else {
        std::future::pending::<()>().await;
    }
}

enum CargoEvent {
    Signal,
    Line(Option<String>),
    Status(std::io::Result<std::process::ExitStatus>),
    OutputTimedOut,
}

async fn resolve_matrix_bin(
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
    interrupted: &mut bool,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    if let Some(path) = std::env::var_os(runner::SWARMAIL_BIN_ENV) {
        let path = PathBuf::from(path);
        return if path.is_file() {
            Ok(Some((exe, path)))
        } else {
            Err(format!(
                "{}={} is not a file",
                runner::SWARMAIL_BIN_ENV,
                path.display()
            ))
        };
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("evals has no parent directory")?
        .to_path_buf();
    let cargo_bin = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = Command::new(cargo_bin);
    cmd.current_dir(root).args([
        "build",
        "--offline",
        "--locked",
        "--bin",
        "swarmail",
        "--message-format=json-render-diagnostics",
    ]);
    let groups = runner::Groups::default();
    let mut cargo = runner::spawn_owned_command(cmd, "cargo build", &groups)
        .map_err(|e| format!("cannot run cargo build: {e}"))?;
    let mut binary = None;
    let mut status = None;
    let mut status_at = None;
    let mut output_closed = false;
    loop {
        if status.is_some() && output_closed {
            break;
        }
        let deadline = status_at.map(|at| at + MATRIX_OUTPUT_EOF_BOUND);
        let event = tokio::select! {
            biased;
            _ = term.recv() => CargoEvent::Signal,
            _ = int.recv() => CargoEvent::Signal,
            line = cargo.lines.recv(), if !output_closed => CargoEvent::Line(line),
            result = cargo.child.wait(), if status.is_none() => CargoEvent::Status(result),
            _ = output_deadline(deadline), if status_at.is_some() && !output_closed => CargoEvent::OutputTimedOut,
        };
        match event {
            CargoEvent::Signal => {
                *interrupted = true;
                if let Err(e) = cleanup_cargo(&cargo, term, int, interrupted).await {
                    eprintln!("swarmail-evals: fallback cargo cleanup failed: {e}");
                }
                return Ok(None);
            }
            CargoEvent::Line(Some(line)) => {
                if binary.is_none() {
                    binary = parse_cargo_artifact(&line);
                }
            }
            CargoEvent::Line(None) => output_closed = true,
            CargoEvent::Status(Ok(exit)) => {
                status = Some(exit);
                status_at = Some(tokio::time::Instant::now());
            }
            CargoEvent::Status(Err(e)) => {
                let cleanup = cleanup_cargo(&cargo, term, int, interrupted).await;
                return Err(format!(
                    "cannot wait for fallback cargo: {e}; cleanup: {cleanup:?}"
                ));
            }
            CargoEvent::OutputTimedOut => {
                let cleanup = cleanup_cargo(&cargo, term, int, interrupted).await;
                return Err(format!(
                    "fallback cargo stdout did not close after exit; cleanup: {cleanup:?}"
                ));
            }
        }
    }
    if let Err(e) = cleanup_cargo(&cargo, term, int, interrupted).await {
        return Err(format!("fallback cargo cleanup failed: {e}"));
    }
    if *interrupted {
        return Ok(None);
    }
    let status = status.expect("loop exits only after the cargo child was reaped");
    if !status.success() {
        return Err(format!("building swarmail failed: {status}"));
    }
    let binary = binary.ok_or_else(|| "cargo reported no swarmail executable".to_string())?;
    Ok(Some((exe, binary)))
}

async fn matrix(
    ids: Vec<String>,
    agents: PathBuf,
    trials: u32,
    deadline_ms: Option<u64>,
) -> ExitCode {
    if trials == 0 {
        return fail("--trials must be positive");
    }
    if let Some(id) = ids.iter().find(|id| tasks::find(id).is_none()) {
        return fail(format!("unknown task {id}"));
    }
    let specs = match std::fs::read_to_string(&agents)
        .map_err(|e| format!("{}: {e}", agents.display()))
        .and_then(|t| report::parse_agents(&t))
    {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let total_slots = match ids
        .len()
        .checked_mul(specs.len())
        .and_then(|count| count.checked_mul(trials as usize))
    {
        Some(count) => count,
        None => return fail("matrix product cardinality exceeds the addressable range"),
    };
    let (mut term, mut int) = match install_matrix_signals() {
        Ok(signals) => signals,
        Err(e) => {
            eprintln!("swarmail-evals: cannot install matrix signal handlers: {e}");
            for index in 0..total_slots {
                let (id, spec) = matrix_slot(&ids, &specs, trials, index);
                print_line(&matrix_refused(id, spec));
            }
            return ExitCode::from(2);
        }
    };
    let writer = start_output_writer();
    let mut interrupted = false;
    matrix_test_hold(
        MATRIX_ADMISSION_HOLD_ENV,
        &mut term,
        &mut int,
        &mut interrupted,
    )
    .await;
    interrupted |= pending_matrix_signal(&mut term, &mut int).await.is_some();
    let mut resolution_error = None;
    let resolved = if interrupted {
        None
    } else {
        match resolve_matrix_bin(&mut term, &mut int, &mut interrupted).await {
            Ok(value) => value,
            Err(e) => {
                resolution_error = Some(e);
                None
            }
        }
    };
    if let Some(error) = resolution_error.as_deref() {
        eprintln!("swarmail-evals: harness error for every trial: {error}");
    }
    let mut next_slot = 0;
    let (mut any_error, mut any_fail) = (resolved.is_none(), false);
    let mut stop_admission = false;
    let mut output_failed = false;
    while next_slot < total_slots {
        if interrupted || pending_matrix_signal(&mut term, &mut int).await.is_some() {
            interrupted = true;
            break;
        }
        let (id, spec) = matrix_slot(&ids, &specs, trials, next_slot);
        let mut row = if let Some((exe, bin)) = &resolved {
            let child = run_child(exe, bin, id, spec, deadline_ms, &mut term, &mut int).await;
            let child_interrupted = child.interrupted;
            let child_failed = child.failure.is_some();
            let child_stop = child.stop_admission;
            let mut row = child.row.unwrap_or_else(|| {
                let mut refused = matrix_refused(id, spec);
                refused.wall_time_ms = child.elapsed_ms;
                refused
            });
            if let Some(error) = child.failure.as_deref() {
                eprintln!("swarmail-evals: matrix trial error: {error}");
            }
            if child_interrupted {
                interrupted = true;
                row.outcome = Outcome::HarnessError;
                row.failure_class = Some(state::FailureClass::HarnessError);
            }
            any_error |= child_failed;
            stop_admission |= child_stop;
            row
        } else {
            matrix_refused(id, spec)
        };
        matrix_test_hold(
            MATRIX_COMMIT_HOLD_ENV,
            &mut term,
            &mut int,
            &mut interrupted,
        )
        .await;
        if interrupted || pending_matrix_signal(&mut term, &mut int).await.is_some() {
            interrupted = true;
            row.outcome = Outcome::HarnessError;
            row.failure_class = Some(state::FailureClass::HarnessError);
        }
        any_error |= row.outcome == Outcome::HarnessError;
        any_fail |= row.outcome == Outcome::Fail;
        loop {
            match write_row(&row, &writer, &mut term, &mut int, &mut interrupted).await {
                Ok(Some(_)) => {
                    interrupted = true;
                    row.outcome = Outcome::HarnessError;
                    row.failure_class = Some(state::FailureClass::HarnessError);
                    any_error = true;
                }
                Ok(None) => break,
                Err(e) => {
                    eprintln!("swarmail-evals: cannot write matrix result: {e}");
                    output_failed = true;
                    any_error = true;
                    break;
                }
            }
        }
        if output_failed {
            break;
        }
        next_slot += 1;
        if interrupted || stop_admission {
            break;
        }
    }
    if interrupted || stop_admission {
        let unstarted = total_slots.saturating_sub(next_slot);
        if interrupted {
            eprintln!("swarmail-evals: matrix interrupted; refused {unstarted} unstarted slot(s)");
        }
        while next_slot < total_slots {
            let (id, spec) = matrix_slot(&ids, &specs, trials, next_slot);
            let row = matrix_refused(id, spec);
            loop {
                match write_row(&row, &writer, &mut term, &mut int, &mut interrupted).await {
                    Ok(Some(_)) => interrupted = true,
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("swarmail-evals: cannot write matrix refusal row: {e}");
                        output_failed = true;
                        any_error = true;
                        break;
                    }
                }
            }
            if output_failed {
                break;
            }
            next_slot += 1;
        }
    } else if next_slot == total_slots {
        matrix_test_hold(MATRIX_FINAL_HOLD_ENV, &mut term, &mut int, &mut interrupted).await;
        interrupted |= pending_matrix_signal(&mut term, &mut int).await.is_some();
        if interrupted {
            eprintln!("swarmail-evals: matrix interrupted; refused 0 unstarted slot(s)");
        }
    }
    if output_failed || interrupted || any_error {
        ExitCode::from(2)
    } else if any_fail {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn matching_spec() -> report::AgentSpec {
        report::AgentSpec {
            label: "oracle".into(),
            argv: Vec::new(),
        }
    }

    fn matching_row() -> TrialResult {
        runner::refused_row(
            &TrialRequest {
                task: Some("signup".into()),
                agent: Some("oracle".into()),
                ..Default::default()
            },
            "trial-id",
        )
    }

    fn exit_status(code: i32) -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(code << 8)
    }

    #[test]
    fn matching_trial_row_requires_one_bounded_valid_matching_row_and_exit_pair() {
        let spec = matching_spec();
        let row = matching_row();
        let bytes = serde_json::to_vec(&row).unwrap();
        let valid = CapturedOutput {
            bytes: bytes.clone(),
            overflow: false,
        };
        let parsed = matching_trial_row(valid, exit_status(2), "signup", &spec, false).unwrap();
        assert_eq!(parsed, row);

        let mut extra_line = bytes.clone();
        extra_line.extend_from_slice(b"\n{}\n");
        let mut wrong_task = row.clone();
        wrong_task.task = Some("otp".into());
        let mut wrong_agent = row.clone();
        wrong_agent.agent = Some("other".into());
        let mut bad_schema = serde_json::to_value(&row).unwrap();
        bad_schema["schema_version"] = serde_json::json!(2);
        let bad_schema = serde_json::to_vec(&bad_schema).unwrap();

        for output in [
            CapturedOutput {
                bytes: b"{not json".to_vec(),
                overflow: false,
            },
            CapturedOutput {
                bytes: extra_line,
                overflow: false,
            },
            CapturedOutput {
                bytes: serde_json::to_vec(&wrong_task).unwrap(),
                overflow: false,
            },
            CapturedOutput {
                bytes: serde_json::to_vec(&wrong_agent).unwrap(),
                overflow: false,
            },
            CapturedOutput {
                bytes: bad_schema,
                overflow: false,
            },
            CapturedOutput {
                bytes,
                overflow: true,
            },
        ] {
            assert!(matching_trial_row(output, exit_status(2), "signup", &spec, false).is_err());
        }

        assert!(
            matching_trial_row(
                CapturedOutput {
                    bytes: serde_json::to_vec(&row).unwrap(),
                    overflow: false,
                },
                exit_status(0),
                "signup",
                &spec,
                false,
            )
            .is_err()
        );
        assert!(
            matching_trial_row(
                CapturedOutput {
                    bytes: serde_json::to_vec(&row).unwrap(),
                    overflow: false,
                },
                exit_status(0),
                "signup",
                &spec,
                true,
            )
            .is_ok()
        );
    }

    #[test]
    fn capture_stdout_drains_beyond_its_retention_limit() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (mut writer, reader) = tokio::io::duplex(1024);
            let write_task = tokio::spawn(async move {
                let bytes = vec![b'x'; MATRIX_OUTPUT_LIMIT + 17];
                tokio::io::AsyncWriteExt::write_all(&mut writer, &bytes)
                    .await
                    .unwrap();
            });
            let output = capture_stdout(reader).await.unwrap();
            write_task.await.unwrap();
            assert!(output.overflow);
            assert_eq!(output.bytes.len(), MATRIX_OUTPUT_LIMIT);
        });
    }
}
