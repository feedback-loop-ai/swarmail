//! `swarmail-evals`: the harness's one executable. stdout carries only
//! machine output (result rows, the catalogue, reports, a server's address
//! line); every diagnostic goes to stderr.

use clap::{Parser, Subcommand};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use swarmail_evals::runner::{self, TrialRequest, TrialResult};
use swarmail_evals::tasks::{self, Catalogue, Outcome};
use swarmail_evals::{fixture, oracle, report, state};

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
    /// Run tasks × agents × trials sequentially, one JSON line per trial.
    /// Exit 2 if any harness error, else 1 if any fail, else 0.
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
    // Resolved once: no trial rebuilds or races the root target directory.
    // A failure still owes one harness-error row per product element
    // (0055 §9), so it is carried into the loop rather than returned.
    let launch = match runner::resolve_swarmail_bin() {
        Ok(bin) => std::env::current_exe()
            .map(|exe| (exe, bin))
            .map_err(|e| format!("current_exe: {e}")),
        Err(e) => Err(e),
    };
    if let Err(e) = &launch {
        eprintln!("swarmail-evals: harness error for every trial: {e}");
    }
    let (mut any_error, mut any_fail) = (false, false);
    for id in &ids {
        for spec in &specs {
            for _ in 0..trials {
                let row = match &launch {
                    Ok((exe, bin)) => run_child(exe, bin, id, spec, deadline_ms).await,
                    Err(_) => None,
                };
                let row = row.unwrap_or_else(|| {
                    runner::refused_row(
                        &TrialRequest {
                            task: Some(id.clone()),
                            agent: Some(spec.label.clone()),
                            ..Default::default()
                        },
                        &runner::new_trial_id(),
                    )
                });
                any_error |= row.outcome == Outcome::HarnessError;
                any_fail |= row.outcome == Outcome::Fail;
                print_line(&row);
            }
        }
    }
    ExitCode::from(if any_error {
        2
    } else if any_fail {
        1
    } else {
        0
    })
}

/// One matrix element as its own `trial` child; `None` when the child
/// could not run or printed no valid row.
async fn run_child(
    exe: &std::path::Path,
    bin: &std::path::Path,
    id: &str,
    spec: &report::AgentSpec,
    deadline_ms: Option<u64>,
) -> Option<TrialResult> {
    let mut cmd = tokio::process::Command::new(exe);
    cmd.args(["trial", "--task", id, "--agent", &spec.label])
        .env(runner::SWARMAIL_BIN_ENV, bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());
    if let Some(ms) = deadline_ms {
        cmd.args(["--deadline-ms", &ms.to_string()]);
    }
    if !spec.argv.is_empty() {
        cmd.arg("--").args(&spec.argv);
    }
    match cmd.output().await {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .and_then(|l| serde_json::from_str::<TrialResult>(l).ok())
            .filter(|r| r.validate().is_ok()),
        Err(e) => {
            eprintln!("swarmail-evals: cannot run a trial: {e}");
            None
        }
    }
}
