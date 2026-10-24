//! `swarmail-evals`: the harness's one executable. stdout carries only
//! machine output (result rows, the catalogue, reports, a server's address
//! line); every diagnostic goes to stderr.

use clap::{Parser, Subcommand};
#[cfg(test)]
use std::future::Future;
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
    #[cfg(test)]
    if std::env::var_os("EVAL_TEST_MATRIX_SIGNAL_SETUP_FAIL").is_some() {
        return Err(std::io::Error::other(
            "injected matrix signal setup failure",
        ));
    }
    let term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    Ok((term, int))
}

#[cfg(test)]
const MATRIX_ADMISSION_HOLD_ENV: &str = "EVAL_TEST_MATRIX_ADMISSION_HOLD_DIR";
#[cfg(test)]
const MATRIX_FINAL_HOLD_ENV: &str = "EVAL_TEST_MATRIX_FINAL_HOLD_DIR";
#[cfg(test)]
const MATRIX_COMMIT_HOLD_ENV: &str = "EVAL_TEST_MATRIX_COMMIT_HOLD_DIR";
#[cfg(test)]
const MATRIX_PENDING_HOLD_ENV: &str = "EVAL_TEST_MATRIX_PENDING_HOLD_DIR";
#[cfg(test)]
const MATRIX_PENDING_HOLD_AT_ENV: &str = "EVAL_TEST_MATRIX_PENDING_HOLD_AT";
#[cfg(test)]
const MATRIX_SPAWN_HOLD_ENV: &str = "EVAL_TEST_MATRIX_SPAWN_HOLD_DIR";
#[cfg(test)]
const MATRIX_SIGNAL_LOG_ENV: &str = "EVAL_TEST_MATRIX_SIGNAL_LOG";
#[cfg(test)]
const MATRIX_TRIAL_START_ENV: &str = "EVAL_TEST_MATRIX_TRIAL_START";
#[cfg(test)]
const MATRIX_SIGNAL_OBSERVED_ENV: &str = "EVAL_TEST_MATRIX_SIGNAL_OBSERVED";
#[cfg(test)]
const MATRIX_OUTPUT_ACK_PENDING_ENV: &str = "EVAL_TEST_MATRIX_OUTPUT_ACK_PENDING";
#[cfg(test)]
const MATRIX_OUTPUT_TIMEOUT_ENV: &str = "EVAL_TEST_MATRIX_OUTPUT_TIMEOUT";
#[cfg(test)]
const MATRIX_EOF_TIMEOUT_ENV: &str = "EVAL_TEST_MATRIX_EOF_TIMEOUT";
#[cfg(test)]
const MATRIX_CHILD_REAPED_ENV: &str = "EVAL_TEST_MATRIX_CHILD_REAPED";
#[cfg(test)]
const MATRIX_TRIALS_ENV: &str = "EVAL_TEST_MATRIX_TRIALS";
#[cfg(test)]
static MATRIX_PENDING_CHECKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
fn matrix_test_event(env: &str, event: &str) {
    use std::io::Write as _;
    if let Some(path) = std::env::var_os(env)
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{event}");
    }
}

#[cfg(test)]
fn matrix_test_monotonic_nanos() -> u128 {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: CLOCK_MONOTONIC writes the initialized timespec on success.
    let result = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) };
    assert_eq!(
        result, 0,
        "CLOCK_MONOTONIC is available to the Linux test harness"
    );
    (value.tv_sec as u128) * 1_000_000_000 + value.tv_nsec as u128
}

#[cfg(test)]
async fn matrix_pending_test_hold() {
    let Some(dir) = std::env::var_os(MATRIX_PENDING_HOLD_ENV).map(PathBuf::from) else {
        return;
    };
    let check = MATRIX_PENDING_CHECKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    if std::env::var(MATRIX_PENDING_HOLD_AT_ENV)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        != Some(check)
    {
        return;
    }
    if std::fs::create_dir_all(&dir).is_err()
        || std::fs::write(dir.join("entered"), check.to_string()).is_err()
    {
        return;
    }
    while !dir.join("release").exists() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[cfg(test)]
async fn matrix_spawn_test_hold() {
    let Some(dir) = std::env::var_os(MATRIX_SPAWN_HOLD_ENV).map(PathBuf::from) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err()
        || std::fs::write(dir.join("entered"), b"child retained after spawn").is_err()
    {
        return;
    }
    while !dir.join("release").exists() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[cfg(test)]
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
    #[cfg(test)]
    matrix_pending_test_hold().await;
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
    #[cfg(test)]
    let (ack_tx, mut ack_rx) = tokio::sync::oneshot::channel();
    #[cfg(not(test))]
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    #[cfg(test)]
    let mut ack_pending_recorded = false;
    #[cfg(test)]
    let ack_pending = std::future::poll_fn(|cx| {
        let result = std::pin::Pin::new(&mut ack_rx).poll(cx);
        if result.is_pending() && !ack_pending_recorded {
            matrix_test_event(
                MATRIX_OUTPUT_ACK_PENDING_ENV,
                "writer acknowledgement is pending",
            );
            ack_pending_recorded = true;
        }
        result
    });
    #[cfg(test)]
    tokio::pin!(ack_pending);
    #[cfg(not(test))]
    let mut ack_pending = ack_rx;
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
            _ = term.recv() => {
                *interrupted = true;
                #[cfg(test)]
                matrix_test_event(MATRIX_SIGNAL_LOG_ENV, "write-term");
            },
            _ = int.recv() => {
                *interrupted = true;
                #[cfg(test)]
                matrix_test_event(MATRIX_SIGNAL_LOG_ENV, "write-int");
            },
            result = &mut ack_pending => {
                result.map_err(|_| "stdout writer vanished".to_string())??;
                return Ok(None);
            }
            _ = tokio::time::sleep_until(deadline) => {
                #[cfg(test)]
                matrix_test_event(MATRIX_OUTPUT_TIMEOUT_ENV, "stdout made no progress for five seconds");
                return Err("stdout made no progress for five seconds".into());
            },
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
    #[cfg(test)]
    if let Some(path) = std::env::var_os(MATRIX_TRIAL_START_ENV) {
        let _ = std::fs::write(path, matrix_test_monotonic_nanos().to_string());
    }
    let mut cmd = Command::new(exe);
    #[cfg(test)]
    let bypass_wait = std::env::var_os("EVAL_TEST_MATRIX_BYPASS_WAIT").is_some();
    #[cfg(not(test))]
    let bypass_wait = false;
    cmd.process_group(0)
        .args(["trial", "--task", id, "--agent", &spec.label])
        .env(runner::SWARMAIL_BIN_ENV, bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(!bypass_wait);
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
    #[cfg(test)]
    matrix_spawn_test_hold().await;
    let pid = child.id().expect("spawned trial has a PID");
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut reader = tokio::spawn(capture_stdout(stdout));
    #[cfg(test)]
    let mut wait = {
        let mut child_wait = Box::pin(child.wait());
        let pending_path = std::env::var_os("EVAL_TEST_MATRIX_WAIT_PENDING").map(PathBuf::from);
        let cleanup_path = std::env::var_os("EVAL_TEST_MATRIX_CLEANUP_ENTERED").map(PathBuf::from);
        let mut acknowledged = false;
        let observe_pending = pending_path.is_some() && cleanup_path.is_some();
        Box::pin(std::future::poll_fn(move |cx| {
            let result = child_wait.as_mut().poll(cx);
            if result.is_pending() && !acknowledged && observe_pending {
                if cleanup_path.as_ref().is_some_and(|path| path.exists()) {
                    if let Some(path) = pending_path.as_ref() {
                        let _ = std::fs::write(path, b"pending");
                    }
                    acknowledged = true;
                } else {
                    let waker = cx.waker().clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        waker.wake();
                    });
                }
            }
            result
        }))
    };
    #[cfg(not(test))]
    let mut wait = Box::pin(child.wait());
    let mut interrupted = false;
    let mut forwarded = false;
    let mut failure = None;
    let status = loop {
        tokio::select! {
            biased;
            _ = term.recv() => {
                interrupted = true;
                #[cfg(test)]
                {
                    matrix_test_event(MATRIX_SIGNAL_LOG_ENV, if forwarded { "term-repeat" } else { "term-forward" });
                    if let Some(path) = std::env::var_os(MATRIX_SIGNAL_OBSERVED_ENV) {
                        let _ = std::fs::write(path, matrix_test_monotonic_nanos().to_string());
                    }
                }
                if !forwarded { forwarded = true; if let Err(e) = signal_trial(pid) { failure = Some(e); } }
                #[cfg(test)]
                if bypass_wait {
                    break None;
                }
            }
            _ = int.recv() => {
                interrupted = true;
                #[cfg(test)]
                {
                    matrix_test_event(MATRIX_SIGNAL_LOG_ENV, if forwarded { "int-repeat" } else { "int-forward" });
                    if let Some(path) = std::env::var_os(MATRIX_SIGNAL_OBSERVED_ENV) {
                        let _ = std::fs::write(path, matrix_test_monotonic_nanos().to_string());
                    }
                }
                if !forwarded { forwarded = true; if let Err(e) = signal_trial(pid) { failure = Some(e); } }
                #[cfg(test)]
                if bypass_wait {
                    break None;
                }
            }
            result = &mut wait => match result {
                Ok(status) => break Some(status),
                Err(e) => { failure = Some(format!("cannot reap trial {pid}: {e}")); break None; }
            },
        }
    };
    #[cfg(test)]
    if status.is_some() {
        matrix_test_event(MATRIX_CHILD_REAPED_ENV, "trial process wait completed");
    }
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
                    #[cfg(test)]
                    matrix_test_event(MATRIX_EOF_TIMEOUT_ENV, "trial stdout remained open more than five seconds after child exit");
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
    #[cfg(test)]
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
            #[cfg(test)]
            Ok(Some((_, bin)))
                if let Some(exe) = std::env::var_os("EVAL_TEST_MATRIX_TRIAL_EXE") =>
            {
                Some((PathBuf::from(exe), bin))
            }
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
        #[cfg(test)]
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
        #[cfg(test)]
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

    const MATRIX_PROBE_ENV: &str = "EVAL_TEST_MATRIX_PROBE";
    const MATRIX_TRIAL_EXE_ENV: &str = "EVAL_TEST_MATRIX_TRIAL_EXE";
    const MATRIX_WAIT_PENDING_ENV: &str = "EVAL_TEST_MATRIX_WAIT_PENDING";
    const MATRIX_CLEANUP_ENTERED_ENV: &str = "EVAL_TEST_MATRIX_CLEANUP_ENTERED";
    const MATRIX_RELEASE_ENV: &str = "EVAL_TEST_MATRIX_RELEASE";
    const MATRIX_TRIAL_PID_ENV: &str = "EVAL_TEST_MATRIX_TRIAL_PID";
    const MATRIX_PRE_TRAP_HOLD_ENV: &str = "EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR";

    #[test]
    fn matrix_probe_entrypoint() {
        if std::env::var_os(MATRIX_PROBE_ENV).is_none() {
            return;
        }
        if std::env::var_os(runner::SWARMAIL_BIN_ENV).is_none() {
            let bin = runner::resolve_swarmail_bin().expect("the Swarmail binary resolves");
            // SAFETY: the re-entered probe is a dedicated child test process.
            unsafe { std::env::set_var(runner::SWARMAIL_BIN_ENV, bin) };
        }
        let agents = PathBuf::from(std::env::var_os("EVAL_TEST_MATRIX_AGENTS").unwrap());
        let ids = std::env::var("EVAL_TEST_MATRIX_TASKS")
            .unwrap_or_else(|_| "signup,otp".into())
            .split(',')
            .map(str::to_owned)
            .collect();
        let trials = std::env::var(MATRIX_TRIALS_ENV)
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let exit = runtime.block_on(matrix(ids, agents, trials, None));
        assert_eq!(exit, ExitCode::from(2));
    }

    #[derive(Clone, Debug)]
    struct MatrixIdentity {
        pid: i32,
        pgrp: i32,
        session: i32,
        start_time: u64,
        state: char,
    }

    fn matrix_identity(pid: i32) -> Option<MatrixIdentity> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let (_, tail) = stat.rsplit_once(')')?;
        let fields: Vec<_> = tail.split_whitespace().collect();
        Some(MatrixIdentity {
            pid,
            state: fields.first()?.chars().next()?,
            pgrp: fields.get(2)?.parse().ok()?,
            session: fields.get(3)?.parse().ok()?,
            start_time: fields.get(19)?.parse().ok()?,
        })
    }

    fn matrix_identity_matches(want: &MatrixIdentity, now: &MatrixIdentity) -> bool {
        want.pid == now.pid
            && want.pgrp == now.pgrp
            && want.session == now.session
            && want.start_time == now.start_time
    }

    fn matrix_identity_alive(want: &MatrixIdentity) -> bool {
        matrix_identity(want.pid).is_some_and(|now| {
            matrix_identity_matches(want, &now) && !matches!(now.state, 'Z' | 'X')
        })
    }

    struct MatrixProbe {
        child: Option<std::process::Child>,
        owner: MatrixIdentity,
        release: PathBuf,
        pre_trap_release: PathBuf,
        owned: Vec<MatrixIdentity>,
    }

    impl MatrixProbe {
        fn pid(&self) -> i32 {
            self.child.as_ref().unwrap().id() as i32
        }

        fn release(&self) {
            let _ = std::fs::write(&self.release, b"release");
        }

        fn release_pre_trap(&self) {
            let _ = std::fs::write(&self.pre_trap_release, b"release");
        }

        fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
            self.child.as_mut().unwrap().try_wait().unwrap()
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
                .unwrap_or_else(|| panic!("matrix probe exceeded its 30-second wait deadline"))
        }

        fn finish(&mut self) -> Result<(), String> {
            let child = self
                .child
                .as_mut()
                .ok_or_else(|| "matrix probe child handle was already removed".to_string())?;
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => return Err("matrix probe still requires owner rescue".into()),
                Err(error) => return Err(format!("cannot reap matrix probe child: {error}")),
            }
            let survivors: Vec<_> = self
                .owned
                .iter()
                .filter(|identity| matrix_identity_alive(identity))
                .collect();
            if !survivors.is_empty() {
                return Err(format!(
                    "matrix probe left owned identities running: {survivors:?}"
                ));
            }
            Ok(())
        }

        fn record_identity(&mut self, identity: MatrixIdentity) {
            if !self
                .owned
                .iter()
                .any(|known| matrix_identity_matches(known, &identity))
            {
                self.owned.push(identity);
            }
        }

        fn record_ready_identity(&mut self, path: &std::path::Path) {
            let value: serde_json::Value = serde_json::from_slice(
                &std::fs::read(path).expect("complete trap-ready marker exists"),
            )
            .expect("complete trap-ready marker is JSON");
            let identity = MatrixIdentity {
                pid: value["pid"].as_i64().expect("ready PID") as i32,
                pgrp: value["pgrp"].as_i64().expect("ready process group") as i32,
                session: value["session"].as_i64().expect("ready session") as i32,
                start_time: value["start_time"].as_u64().expect("ready start time"),
                state: 'S',
            };
            self.record_identity(identity);
        }

        fn record_descendants(&mut self) {
            let root = self.pid();
            if !matrix_identity(root).is_some_and(|now| matrix_identity_matches(&self.owner, &now))
            {
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
                let Some(process) = matrix_identity(pid) else {
                    continue;
                };
                let parent = std::fs::read_to_string(format!("/proc/{pid}/stat"))
                    .ok()
                    .and_then(|stat| {
                        stat.rsplit_once(')')?
                            .1
                            .split_whitespace()
                            .nth(1)?
                            .parse::<i32>()
                            .ok()
                    });
                if let Some(parent) = parent {
                    processes.push((process, parent));
                }
            }
            let mut parents = std::collections::HashSet::from([root]);
            loop {
                let children: Vec<_> = processes
                    .iter()
                    .filter(|(_, parent)| parents.contains(parent))
                    .map(|(child, _)| child.clone())
                    .collect();
                let added = children
                    .iter()
                    .filter(|child| parents.insert(child.pid))
                    .count();
                for child in children {
                    self.record_identity(child);
                }
                if added == 0 {
                    break;
                }
            }
        }

        fn signal_owned_groups(&self, signal: i32) {
            let groups: std::collections::HashSet<_> =
                self.owned.iter().map(|id| id.pgrp).collect();
            for pgrp in groups {
                let known_member_is_live = self.owned.iter().any(|want| {
                    want.pgrp == pgrp
                        && matrix_identity(want.pid).is_some_and(|now| {
                            matrix_identity_matches(want, &now) && !matches!(now.state, 'Z' | 'X')
                        })
                });
                if known_member_is_live {
                    // SAFETY: each group is signalled only while a recorded PID/start/session
                    // identity still proves membership in this test-owned group.
                    unsafe { libc::kill(-pgrp, signal) };
                }
            }
        }

        fn wait_owned_stopped(&self, limit: Duration) -> bool {
            let deadline = std::time::Instant::now() + limit;
            while self.owned.iter().any(matrix_identity_alive) {
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
    }

    impl Drop for MatrixProbe {
        fn drop(&mut self) {
            self.release();
            self.release_pre_trap();
            self.record_descendants();
            if self.child.is_some() {
                let ordinary_wait = if std::thread::panicking() {
                    Duration::ZERO
                } else {
                    Duration::from_secs(30)
                };
                if self.wait_for(ordinary_wait).is_none() {
                    let pid = self.pid();
                    unsafe { libc::kill(pid, libc::SIGTERM) };
                    self.record_descendants();
                    if self.wait_for(Duration::from_secs(15)).is_none() {
                        unsafe { libc::kill(pid, libc::SIGKILL) };
                        if self.wait_for(Duration::from_secs(5)).is_none() {
                            eprintln!(
                                "matrix probe {pid} did not reap within bounded failure teardown"
                            );
                        }
                    }
                }
            }
            if !self.cleanup_owned() {
                eprintln!(
                    "matrix probe failure teardown left an owned identity running: {:?}",
                    self.owned
                );
            }
        }
    }

    fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while !condition() {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {what}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn matrix_probe(
        dir: &std::path::Path,
        bypass_wait: bool,
        signal_setup_fail: bool,
        hold_stage: Option<&str>,
        trial_mode: &str,
        tasks: &str,
    ) -> MatrixProbe {
        matrix_probe_with_pre_trap(
            dir,
            bypass_wait,
            signal_setup_fail,
            hold_stage,
            trial_mode,
            tasks,
            false,
        )
    }

    #[derive(Default)]
    struct MatrixProbeControls {
        hold_before_trap: bool,
        pending_hold_at: Option<usize>,
        spawn_hold: bool,
        trials: Option<u32>,
        missing_trial_executable: bool,
        pipe_stdout: bool,
        pipe_stderr: bool,
    }

    fn matrix_probe_with_pre_trap(
        dir: &std::path::Path,
        bypass_wait: bool,
        signal_setup_fail: bool,
        hold_stage: Option<&str>,
        trial_mode: &str,
        tasks: &str,
        hold_before_trap: bool,
    ) -> MatrixProbe {
        matrix_probe_config(
            dir,
            bypass_wait,
            signal_setup_fail,
            hold_stage,
            trial_mode,
            tasks,
            MatrixProbeControls {
                hold_before_trap,
                ..MatrixProbeControls::default()
            },
        )
    }

    fn matrix_probe_config(
        dir: &std::path::Path,
        bypass_wait: bool,
        signal_setup_fail: bool,
        hold_stage: Option<&str>,
        trial_mode: &str,
        tasks: &str,
        controls: MatrixProbeControls,
    ) -> MatrixProbe {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let script = dir.join("trial-helper.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
task= agent=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --task) task=$2; shift 2 ;;
    --agent) agent=$2; shift 2 ;;
    *) shift ;;
  esac
done
stat_tail=$(sed 's/^.*) //' "/proc/$$/stat")
set -- $stat_tail
ready="$EVAL_TEST_MATRIX_TRIAL_PID"
tmp="$ready.tmp.$$"
printf '{"pid":%s,"pgrp":%s,"session":%s,"start_time":%s}\n' "$$" "$3" "$4" "${20}" > "$tmp"
if [ -n "${EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR:-}" ]; then
  mkdir -p "$EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR"
  touch "$EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR/entered"
  while [ ! -e "$EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR/release" ]; do :; done
fi
interrupt() {
  trap '' TERM INT
  touch "$EVAL_TEST_MATRIX_CLEANUP_ENTERED"
  while [ ! -e "$EVAL_TEST_MATRIX_RELEASE" ]; do :; done
  printf '{"schema_version":1,"trial_id":"matrix-probe","task":"%s","agent":"%s","outcome":"harness_error","failure_class":"harness_error","wall_time_ms":1,"agent_exit_status":null,"agent_exits":[],"confinement":"unconfined"}\n' "$task" "$agent"
  exit 0
}
trap interrupt TERM INT
if [ -n "${EVAL_TEST_MATRIX_LAUNCH_LOG:-}" ]; then printf '%s\n' "$task" >> "$EVAL_TEST_MATRIX_LAUNCH_LOG"; fi
mv "$tmp" "$ready"
if [ "$EVAL_TEST_MATRIX_TRIAL_MODE" = holder ]; then
  sleep 1000 2>/dev/null &
  printf '%s\n' "$!" > "$EVAL_TEST_MATRIX_HOLDER_PID"
  exit 0
fi
if [ "$EVAL_TEST_MATRIX_TRIAL_MODE" = complete ]; then
  printf '{"schema_version":1,"trial_id":"matrix-probe","task":"%s","agent":"%s","outcome":"pass","failure_class":null,"wall_time_ms":1,"agent_exit_status":0,"agent_exits":[],"confinement":"unconfined"}\n' "$task" "$agent"
  exit 0
fi
while :; do :; done
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).unwrap();
        let agents = dir.join("agents.json");
        std::fs::write(&agents, r#"[{"label":"oracle","argv":[]}]"#).unwrap();
        let hold = hold_stage.map(|stage| dir.join(stage));
        let release = hold
            .as_ref()
            .map(|path| path.join("release"))
            .unwrap_or_else(|| dir.join("release"));
        let pre_trap_dir = dir.join("pre-trap");
        let pre_trap_release = pre_trap_dir.join("release");
        let pending = dir.join("wait-pending");
        let cleanup = dir.join("cleanup-entered");
        let stdout = std::fs::File::create(dir.join("stdout.log")).unwrap();
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "tests::matrix_probe_entrypoint",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(MATRIX_PROBE_ENV, "1")
        .env("EVAL_TEST_MATRIX_AGENTS", &agents)
        .env("EVAL_TEST_MATRIX_TASKS", tasks)
        .env("EVAL_TEST_MATRIX_TRIAL_MODE", trial_mode)
        .env(MATRIX_TRIAL_EXE_ENV, &script)
        .env(MATRIX_WAIT_PENDING_ENV, &pending)
        .env(MATRIX_CLEANUP_ENTERED_ENV, &cleanup)
        .env(MATRIX_RELEASE_ENV, &release)
        .env(MATRIX_TRIAL_PID_ENV, dir.join("trial.pid"))
        .env(MATRIX_SIGNAL_LOG_ENV, dir.join("signals.log"))
        .env(MATRIX_TRIAL_START_ENV, dir.join("trial-start.ns"))
        .env(MATRIX_SIGNAL_OBSERVED_ENV, dir.join("signal-observed.ns"))
        .env(MATRIX_EOF_TIMEOUT_ENV, dir.join("eof-timeout"))
        .env(MATRIX_CHILD_REAPED_ENV, dir.join("trial-reaped"))
        .env(MATRIX_OUTPUT_TIMEOUT_ENV, dir.join("output-timeout"))
        .env(
            MATRIX_OUTPUT_ACK_PENDING_ENV,
            dir.join("output-ack-pending"),
        )
        .env("EVAL_TEST_MATRIX_LAUNCH_LOG", dir.join("launches.log"))
        .env("EVAL_TEST_MATRIX_HOLDER_PID", dir.join("holder.pid"))
        .env(runner::SWARMAIL_BIN_ENV, "/bin/true")
        .env("EVAL_TEST_MATRIX_RELEASE", &release)
        .stdout(if controls.pipe_stdout {
            Stdio::piped()
        } else {
            Stdio::from(stdout)
        })
        .stderr(if controls.pipe_stderr {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        if let Some(trials) = controls.trials {
            cmd.env(MATRIX_TRIALS_ENV, trials.to_string());
        }
        if let Some(at) = controls.pending_hold_at {
            cmd.env(MATRIX_PENDING_HOLD_ENV, dir.join("pending"));
            cmd.env(MATRIX_PENDING_HOLD_AT_ENV, at.to_string());
        }
        if controls.spawn_hold {
            cmd.env(MATRIX_SPAWN_HOLD_ENV, dir.join("spawned"));
        }
        if controls.missing_trial_executable {
            cmd.env(MATRIX_TRIAL_EXE_ENV, dir.join("missing-trial"));
        }
        if bypass_wait {
            cmd.env("EVAL_TEST_MATRIX_BYPASS_WAIT", "1");
        }
        if signal_setup_fail {
            cmd.env("EVAL_TEST_MATRIX_SIGNAL_SETUP_FAIL", "1");
        }
        if controls.hold_before_trap {
            cmd.env(MATRIX_PRE_TRAP_HOLD_ENV, &pre_trap_dir);
        }
        if let Some(hold) = hold.as_ref() {
            let name = match hold_stage.unwrap() {
                "admission" => MATRIX_ADMISSION_HOLD_ENV,
                "commit" => MATRIX_COMMIT_HOLD_ENV,
                "final" => MATRIX_FINAL_HOLD_ENV,
                other => panic!("unknown matrix hold stage {other}"),
            };
            cmd.env(name, hold);
        }
        let child = cmd.spawn().unwrap();
        let owner = matrix_identity(child.id() as i32).expect("matrix probe owner identity");
        MatrixProbe {
            child: Some(child),
            owner,
            release,
            pre_trap_release,
            owned: Vec::new(),
        }
    }

    fn matrix_wait_fence_case(bypass_wait: bool, signal: i32) {
        let dir = tempfile::tempdir().unwrap();
        let mut matrix = matrix_probe_with_pre_trap(
            dir.path(),
            bypass_wait,
            false,
            None,
            "held",
            "signup,otp",
            true,
        );
        let cleanup = dir.path().join("cleanup-entered");
        let pending = dir.path().join("wait-pending");
        let trial_pid_file = dir.path().join("trial.pid");
        let pre_trap = dir.path().join("pre-trap");
        eventually("helper before trap installation", || {
            pre_trap.join("entered").is_file()
        });
        matrix.record_descendants();
        assert!(
            !trial_pid_file.exists(),
            "no PID-only readiness is published before traps"
        );
        assert!(
            matrix.try_wait().is_none(),
            "matrix remains live before readiness"
        );
        assert!(
            matrix.owned.iter().any(matrix_identity_alive),
            "the test-owned helper is recorded while it is held before trap installation"
        );
        matrix.release_pre_trap();
        eventually("atomic complete trap-ready marker", || {
            trial_pid_file.is_file()
        });
        matrix.record_ready_identity(&trial_pid_file);
        let active = matrix.owned.last().unwrap().clone();
        assert!(
            matrix_identity_alive(&active),
            "the acknowledged helper identity is live"
        );

        // Signal only the matrix PID, and only after its active helper has
        // atomically acknowledged that its own TERM/INT trap is installed.
        unsafe { libc::kill(matrix.pid(), signal) };
        eventually("the active trial cleanup hold", || cleanup.is_file());

        if bypass_wait {
            eventually(
                "the negative-control matrix exit while cleanup is held",
                || matrix.try_wait().is_some(),
            );
            assert!(
                !pending.exists(),
                "bypassing the wait fence cannot acknowledge a pending wait"
            );
            assert!(
                matrix_identity_alive(&active),
                "the negative control exits while owned work is still held"
            );
            assert!(
                matrix.finish().is_err(),
                "the checked guard rejects the live owned trial before rescue"
            );
            matrix.release();
            assert!(matrix.wait().success());
            eventually("the released negative-control trial to stop", || {
                !matrix_identity_alive(&active)
            });
            let start_ns: u128 = std::fs::read_to_string(dir.path().join("trial-start.ns"))
                .unwrap()
                .parse()
                .unwrap();
            let signal_ns: u128 = std::fs::read_to_string(dir.path().join("signal-observed.ns"))
                .unwrap()
                .parse()
                .unwrap();
            assert!(
                signal_ns >= start_ns,
                "the observed signal follows trial start"
            );
            let lower_ms = ((signal_ns - start_ns) / 1_000_000) as u64;
            let stdout = std::fs::read_to_string(dir.path().join("stdout.log")).unwrap();
            let rows = stdout
                .lines()
                .filter_map(|line| {
                    let json = line.get(line.find('{')?..)?;
                    serde_json::from_str::<TrialResult>(json).ok()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                rows.len(),
                2,
                "active and unstarted slots are emitted: {stdout}"
            );
            assert_eq!(rows[0].outcome, Outcome::HarnessError);
            assert_eq!(rows[0].agent_exit_status, None);
            assert!(rows[0].agent_exits.is_empty());
            assert!(rows[0].wall_time_ms > 0);
            assert!(
                rows[0].wall_time_ms + 1 >= lower_ms && rows[0].wall_time_ms <= lower_ms + 1_000,
                "active row time is bracketed by the trial-start and signal events: lower={lower_ms}ms, row={}ms",
                rows[0].wall_time_ms
            );
            assert_eq!(rows[1].outcome, Outcome::HarnessError);
            assert_eq!(rows[1].wall_time_ms, 0);
            assert_eq!(rows[1].agent_exit_status, None);
            assert!(rows[1].agent_exits.is_empty());
            assert_ne!(rows[0].trial_id, rows[1].trial_id);
            assert!(matrix.finish().is_ok(), "negative-control owner was reaped");
        } else {
            eventually("the child's actual wait future returning Pending", || {
                pending.is_file()
            });
            assert!(
                matrix.wait_for(Duration::from_millis(50)).is_none(),
                "the matrix wait remains bounded and retains its child handle"
            );
            assert!(matrix.try_wait().is_none());
            assert!(
                matrix_identity_alive(&active),
                "the trial remains live at the held cleanup boundary"
            );
            matrix.release();
            assert!(matrix.wait().success());
            assert!(
                !matrix_identity_alive(&active),
                "trial exited before matrix completion"
            );
            let stdout = std::fs::read_to_string(dir.path().join("stdout.log")).unwrap();
            let rows = stdout
                .lines()
                .filter_map(|line| {
                    let json = line.get(line.find('{')?..)?;
                    serde_json::from_str::<TrialResult>(json).ok()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                rows.len(),
                2,
                "active and unstarted rows are accounted: {stdout}"
            );
            assert_eq!(rows[0].outcome, Outcome::HarnessError);
            assert!(rows[0].wall_time_ms > 0);
            assert!(rows[0].agent_exits.is_empty());
            assert_eq!(rows[1].outcome, Outcome::HarnessError);
            assert_eq!(rows[1].wall_time_ms, 0);
            assert_eq!(rows[1].agent_exit_status, None);
            assert!(rows[1].agent_exits.is_empty());
            assert_ne!(rows[0].trial_id, rows[1].trial_id);
            assert!(matrix.finish().is_ok(), "wait-fenced owner was reaped");
        }
    }

    #[test]
    fn matrix_completion_fence_is_observed_and_its_negative_control_exits_early() {
        for signal in [libc::SIGTERM, libc::SIGINT] {
            matrix_wait_fence_case(false, signal);
            matrix_wait_fence_case(true, signal);
        }
    }

    #[test]
    fn matrix_probe_readiness_unwind_releases_and_reaps_owned_processes() {
        let dir = tempfile::tempdir().unwrap();
        let ready = dir.path().join("trial.pid");
        let mut owned = Vec::new();
        let mut matrix_pid = 0;
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut matrix = matrix_probe_with_pre_trap(
                dir.path(),
                false,
                false,
                None,
                "held",
                "signup,otp",
                true,
            );
            matrix_pid = matrix.pid();
            eventually("pre-trap readiness hold", || {
                dir.path().join("pre-trap/entered").is_file()
            });
            matrix.record_descendants();
            owned = matrix.owned.clone();
            assert!(
                !ready.exists(),
                "failure occurs before trap-ready publication"
            );
            assert!(matrix.try_wait().is_none());
            std::panic::resume_unwind(Box::new("injected assertion unwind"));
        }));
        assert!(unwind.is_err());
        assert!(
            matrix_identity(matrix_pid).is_none(),
            "matrix probe was reaped"
        );
        assert!(
            !owned.is_empty(),
            "the guard retained identities before unwind"
        );
        assert!(
            owned
                .iter()
                .all(|identity| !matrix_identity_alive(identity)),
            "readiness-failure teardown stopped every retained identity: {owned:?}"
        );
    }

    fn matrix_probe_rows(dir: &std::path::Path) -> Vec<TrialResult> {
        let stdout = std::fs::read_to_string(dir.join("stdout.log")).unwrap();
        stdout
            .lines()
            .filter_map(|line| {
                let json = line.get(line.find('{')?..)?;
                serde_json::from_str::<TrialResult>(json).ok()
            })
            .collect()
    }

    #[test]
    fn matrix_owner_test_controls_cover_setup_admission_commit_and_final_row() {
        let setup = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe(setup.path(), false, true, None, "complete", "signup,otp");
        assert!(probe.wait().success());
        assert!(!setup.path().join("trial.pid").exists());
        let rows = matrix_probe_rows(setup.path());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| {
            row.outcome == Outcome::HarnessError
                && row.wall_time_ms == 0
                && row.agent_exits.is_empty()
        }));

        let admission = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe(
            admission.path(),
            false,
            false,
            Some("admission"),
            "complete",
            "signup,otp",
        );
        let hold = admission.path().join("admission");
        eventually("matrix admission control", || {
            hold.join("entered").is_file()
        });
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        eventually("matrix admission signal", || {
            hold.join("signal-term").is_file()
        });
        probe.release();
        assert!(probe.wait().success());
        assert!(!admission.path().join("trial.pid").exists());
        let rows = matrix_probe_rows(admission.path());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.wall_time_ms == 0));

        let commit = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe(
            commit.path(),
            false,
            false,
            Some("commit"),
            "complete",
            "signup,otp",
        );
        let hold = commit.path().join("commit");
        eventually("matrix commit control", || hold.join("entered").is_file());
        assert!(matrix_probe_rows(commit.path()).is_empty());
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        eventually("matrix commit signal", || {
            hold.join("signal-term").is_file()
        });
        probe.release();
        assert!(probe.wait().success());
        let rows = matrix_probe_rows(commit.path());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, Outcome::HarnessError);
        assert_eq!(rows[0].agent_exit_status, Some(0));
        assert_eq!(rows[0].agent_exits.len(), 0);
        assert_eq!(rows[1].outcome, Outcome::HarnessError);
        assert_eq!(rows[1].wall_time_ms, 0);

        let final_row = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe(
            final_row.path(),
            false,
            false,
            Some("final"),
            "complete",
            "signup",
        );
        let hold = final_row.path().join("final");
        eventually("matrix final-row control", || {
            hold.join("entered").is_file()
        });
        let committed = std::fs::read(final_row.path().join("stdout.log")).unwrap();
        let rows = matrix_probe_rows(final_row.path());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].outcome, Outcome::Pass);
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        eventually("matrix final-row signal", || {
            hold.join("signal-term").is_file()
        });
        probe.release();
        assert!(probe.wait().success());
        let finished = std::fs::read(final_row.path().join("stdout.log")).unwrap();
        assert!(
            finished.starts_with(&committed),
            "the committed final row remains an immutable byte prefix"
        );
    }

    #[test]
    fn pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children() {
        let before_spawn = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            before_spawn.path(),
            false,
            false,
            None,
            "complete",
            "signup,otp",
            MatrixProbeControls {
                pending_hold_at: Some(2),
                ..MatrixProbeControls::default()
            },
        );
        let pending = before_spawn.path().join("pending");
        eventually("the pending check before spawn", || {
            pending.join("entered").is_file()
        });
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        std::fs::write(pending.join("release"), b"release").unwrap();
        assert!(probe.wait().success());
        assert!(!before_spawn.path().join("launches.log").exists());
        let rows = matrix_probe_rows(before_spawn.path());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| {
            row.outcome == Outcome::HarnessError
                && row.wall_time_ms == 0
                && row.agent_exit_status.is_none()
                && row.agent_exits.is_empty()
        }));
        assert!(rows.iter().all(|row| !row.trial_id.is_empty()));
        assert_eq!(
            rows.iter()
                .map(|row| row.trial_id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            2
        );

        let between_rows = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            between_rows.path(),
            false,
            false,
            None,
            "complete",
            "signup,otp",
            MatrixProbeControls {
                pending_hold_at: Some(5),
                ..MatrixProbeControls::default()
            },
        );
        let pending = between_rows.path().join("pending");
        eventually("the production between-row pending check", || {
            pending.join("entered").is_file()
        });
        let launches = std::fs::read_to_string(between_rows.path().join("launches.log")).unwrap();
        assert_eq!(launches.lines().collect::<Vec<_>>(), ["signup"]);
        unsafe { libc::kill(probe.pid(), libc::SIGINT) };
        std::fs::write(pending.join("release"), b"release").unwrap();
        assert!(probe.wait().success());
        let rows = matrix_probe_rows(between_rows.path());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, Outcome::Pass);
        assert_eq!(rows[1].outcome, Outcome::HarnessError);
        assert_eq!(rows[1].wall_time_ms, 0);
        assert_eq!(
            std::fs::read_to_string(between_rows.path().join("launches.log"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["signup"]
        );

        let before_enqueue = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            before_enqueue.path(),
            false,
            false,
            None,
            "complete",
            "signup,otp",
            MatrixProbeControls {
                pending_hold_at: Some(4),
                ..MatrixProbeControls::default()
            },
        );
        let pending = before_enqueue.path().join("pending");
        eventually("the pending check before enqueue", || {
            pending.join("entered").is_file()
        });
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        std::fs::write(pending.join("release"), b"release").unwrap();
        assert!(probe.wait().success());
        let rows = matrix_probe_rows(before_enqueue.path());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, Outcome::HarnessError);
        assert_eq!(rows[0].agent_exit_status, Some(0));
        assert_eq!(rows[1].outcome, Outcome::HarnessError);
        assert_eq!(rows[1].wall_time_ms, 0);
        assert_eq!(
            std::fs::read_to_string(before_enqueue.path().join("launches.log"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["signup"]
        );

        let issued = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            issued.path(),
            false,
            false,
            None,
            "held",
            "signup,otp",
            MatrixProbeControls {
                spawn_hold: true,
                ..MatrixProbeControls::default()
            },
        );
        let spawned = issued.path().join("spawned");
        eventually("the already-issued child hold", || {
            spawned.join("entered").is_file()
        });
        probe.record_ready_identity(&issued.path().join("trial.pid"));
        let active = probe.owned.last().unwrap().clone();
        assert!(matrix_identity_alive(&active));
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        std::fs::write(spawned.join("release"), b"release").unwrap();
        let cleanup = issued.path().join("cleanup-entered");
        eventually(
            "the retained child to receive forwarded termination",
            || cleanup.is_file(),
        );
        let pending_wait = issued.path().join("wait-pending");
        eventually("the retained child wait to remain pending", || {
            pending_wait.is_file()
        });
        assert!(probe.try_wait().is_none());
        assert!(matrix_identity_alive(&active));
        probe.release();
        assert!(probe.wait().success());
        assert!(!matrix_identity_alive(&active));
        let rows = matrix_probe_rows(issued.path());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, Outcome::HarnessError);
        assert_eq!(rows[1].wall_time_ms, 0);
        assert_eq!(
            std::fs::read_to_string(issued.path().join("launches.log"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["signup"]
        );
    }

    #[test]
    fn mixed_signals_during_matrix_cleanup_do_not_bypass_the_retained_wait() {
        let dir = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe(dir.path(), false, false, None, "held", "signup,otp");
        let active_file = dir.path().join("trial.pid");
        eventually("active trial readiness", || active_file.is_file());
        probe.record_ready_identity(&active_file);
        let active = probe.owned.last().unwrap().clone();
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        let cleanup = dir.path().join("cleanup-entered");
        let pending = dir.path().join("wait-pending");
        let log = dir.path().join("signals.log");
        eventually("first forwarded signal and held cleanup", || {
            cleanup.is_file()
                && pending.is_file()
                && std::fs::read_to_string(&log).is_ok_and(|contents| contents.lines().count() == 1)
        });
        assert_eq!(
            std::fs::read_to_string(&log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["term-forward"]
        );
        for (index, (signal, _)) in [(libc::SIGINT, "int-repeat"), (libc::SIGTERM, "term-repeat")]
            .into_iter()
            .enumerate()
        {
            unsafe { libc::kill(probe.pid(), signal) };
            eventually("mixed signal acknowledgement", || {
                std::fs::read_to_string(&log)
                    .is_ok_and(|contents| contents.lines().count() == index + 2)
            });
        }
        assert_eq!(
            std::fs::read_to_string(&log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["term-forward", "int-repeat", "term-repeat"]
        );
        assert!(probe.try_wait().is_none());
        assert!(matrix_identity_alive(&active));
        probe.release();
        assert!(probe.wait().success());
        assert!(!matrix_identity_alive(&active));
        let rows = matrix_probe_rows(dir.path());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, Outcome::HarnessError);
        assert_eq!(rows[1].wall_time_ms, 0);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("launches.log"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["signup"]
        );
    }

    #[test]
    fn matrix_spawn_failure_keeps_all_rows_and_unknown_agent_statuses() {
        let dir = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            dir.path(),
            false,
            false,
            None,
            "complete",
            "signup,otp",
            MatrixProbeControls {
                missing_trial_executable: true,
                ..MatrixProbeControls::default()
            },
        );
        assert!(probe.wait().success());
        assert!(!dir.path().join("launches.log").exists());
        let rows = matrix_probe_rows(dir.path());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| {
            row.outcome == Outcome::HarnessError
                && row.agent_exit_status.is_none()
                && row.agent_exits.is_empty()
                && row.trial_id != "matrix-probe"
        }));
        assert_eq!(
            rows.iter()
                .map(|row| row.trial_id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            2
        );
    }

    #[test]
    fn matrix_stdout_backpressure_is_bounded_and_still_observes_signals() {
        use std::io::BufRead;
        let dir = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            dir.path(),
            false,
            false,
            Some("admission"),
            "complete",
            "signup",
            MatrixProbeControls {
                trials: Some(5000),
                pipe_stdout: true,
                pipe_stderr: true,
                ..MatrixProbeControls::default()
            },
        );
        let hold = dir.path().join("admission");
        eventually("refusal-only matrix admission hold", || {
            hold.join("entered").is_file()
        });
        unsafe { libc::kill(probe.pid(), libc::SIGTERM) };
        eventually("admission signal acknowledgement", || {
            hold.join("signal-term").is_file()
        });
        probe.release();
        let ack_pending = dir.path().join("output-ack-pending");
        eventually("matrix output acknowledgement wait", || {
            ack_pending.is_file()
        });
        unsafe { libc::kill(probe.pid(), libc::SIGINT) };
        let stderr = probe.child.as_mut().unwrap().stderr.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if line.contains("stdout made no progress for five seconds") {
                    let _ = tx.send(line.clone());
                }
                line.clear();
            }
        });
        let diagnostic = rx
            .recv_timeout(Duration::from_secs(12))
            .expect("the real unread pipe reached the fixed output acknowledgement deadline");
        assert!(diagnostic.contains("stdout made no progress for five seconds"));
        assert!(dir.path().join("output-timeout").is_file());
        assert!(
            std::fs::read_to_string(dir.path().join("signals.log"))
                .unwrap()
                .lines()
                .any(|event| event == "write-int"),
            "the writer wait continues observing interruption while stdout is blocked"
        );
        let mut stdout = probe.child.as_mut().unwrap().stdout.take().unwrap();
        let mut drained = Vec::new();
        std::io::Read::read_to_end(&mut stdout, &mut drained).unwrap();
        assert!(probe.wait().success());
        assert!(drained.windows(1).any(|byte| byte[0] == b'\n'));
    }

    #[test]
    fn matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test() {
        use std::io::Read;
        let dir = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            dir.path(),
            false,
            false,
            None,
            "holder",
            "signup,otp",
            MatrixProbeControls {
                pipe_stderr: true,
                ..MatrixProbeControls::default()
            },
        );
        let holder_file = dir.path().join("holder.pid");
        eventually("inherited stdout descriptor holder", || {
            holder_file.is_file()
        });
        let holder_pid = std::fs::read_to_string(&holder_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let holder = matrix_identity(holder_pid).expect("holder identity");
        probe.record_identity(holder.clone());
        probe.record_ready_identity(&dir.path().join("trial.pid"));
        let trial = probe
            .owned
            .iter()
            .find(|id| id.pid != holder.pid)
            .unwrap()
            .clone();
        assert!(matrix_identity_alive(&holder));
        let eof_deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !(dir.path().join("trial-reaped").is_file()
            && dir.path().join("eof-timeout").is_file())
            && std::time::Instant::now() < eof_deadline
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            dir.path().join("trial-reaped").is_file() && dir.path().join("eof-timeout").is_file(),
            "missing EOF completion markers; trial={trial:?} current={:?}; holder={holder:?} current={:?}; stdout={}",
            matrix_identity(trial.pid),
            matrix_identity(holder.pid),
            std::fs::read_to_string(dir.path().join("stdout.log")).unwrap_or_default()
        );
        assert!(probe.wait().success());
        assert!(
            !matrix_identity_alive(&trial),
            "the exited trial child was reaped"
        );
        assert!(
            matrix_identity_alive(&holder),
            "the inherited descriptor remained open at return"
        );
        let mut stderr = String::new();
        probe
            .child
            .as_mut()
            .unwrap()
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(
            stderr.contains("trial stdout remained open more than five seconds after child exit"),
            "{stderr}"
        );
        assert!(
            probe.cleanup_owned(),
            "the test-owned holder is explicitly reaped"
        );
        assert!(!matrix_identity_alive(&holder));
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
