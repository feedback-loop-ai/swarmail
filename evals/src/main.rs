//! `swarmail-evals`: the harness's one executable. stdout carries only
//! machine output (result rows, the catalogue, reports, a server's address
//! line); every diagnostic goes to stderr.

use clap::{Parser, Subcommand};
use std::ffi::OsString;
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
use swarmail_evals::{
    artifacts, baseline, fixture, fixture_http, identity, isolation, oracle, provenance, report,
    state, wrappers,
};
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

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run fresh controls before the approved paid baseline. Paid admission
    /// currently refuses: locally evidenced CLI/auth contracts are incomplete.
    Baseline {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Launch Claude with private selected OAuth authentication.
    AgentClaude {
        #[arg(long)]
        model: String,
        #[arg(long)]
        config: PathBuf,
    },
    /// Launch Codex with private selected ChatGPT authentication.
    AgentCodex {
        #[arg(long)]
        model: String,
        #[arg(long)]
        config: PathBuf,
    },
    /// Launch dsh with the selected private route credential.
    AgentDsh {
        #[arg(long)]
        model: String,
        #[arg(long)]
        config: PathBuf,
    },
    /// Serve one app-bound HTTP tool over MCP stdio using the EVAL inputs.
    FixtureHttp,
    #[command(hide = true)]
    FixtureHttpIsolated,
    /// Print the v2 task catalogue (schema version 2) as JSON.
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
        #[arg(long)]
        seed: Option<String>,
        #[arg(long)]
        fixture_time: Option<String>,
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
        /// Supported trial concurrency; values other than 1 refuse.
        #[arg(long, default_value_t = 1)]
        concurrency: u32,
        /// Keep state.json, result.json, diagnostics and metadata here.
        #[arg(long)]
        artifacts_dir: Option<PathBuf>,
        #[arg(long)]
        seed: Option<String>,
        #[arg(long)]
        fixture_time: Option<String>,
        #[arg(long)]
        agent_model: Option<String>,
        #[arg(long)]
        agent_config_id: Option<String>,
        #[arg(long)]
        public_argv: bool,
        #[arg(long, value_enum)]
        wrapper_profile: Option<provenance::WrapperProfile>,
        #[arg(long, hide = true)]
        worker_context: Option<String>,
        /// Prepared, not activated (proposed 0115): run the whole trial
        /// inside a bubblewrap boundary or refuse by name. Never falls back
        /// to an unisolated run.
        #[arg(long, hide = true)]
        isolated: bool,
        /// One more absolute regular file bound read-only into the boundary.
        #[arg(long, hide = true, requires = "isolated")]
        isolated_expose: Vec<PathBuf>,
        /// The external agent's program and literal arguments (no shell).
        #[arg(last = true)]
        argv: Vec<OsString>,
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
        /// Supported trial concurrency; values other than 1 refuse.
        #[arg(long, default_value_t = 1)]
        concurrency: u32,
        #[arg(long)]
        artifacts_dir: Option<PathBuf>,
        #[arg(long)]
        seed: Option<String>,
        #[arg(long)]
        fixture_time: Option<String>,
    },
    /// Re-apply a task's check to a retained state.json.
    Check {
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        state: Option<PathBuf>,
        #[arg(long)]
        artifacts_dir: Option<PathBuf>,
        #[arg(long)]
        matrix: Option<PathBuf>,
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

fn fail(_msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("swarmail-evals: harness error: operation_failed");
    ExitCode::from(2)
}

/// What the `Trial` arm validated before the runtime existed: the fixture
/// inputs, the declaration rules and the worker context. Any refusal here is
/// the one `operation_failed` diagnostic, exactly as before the boundary.
fn trial_request(cmd: &Cmd) -> Result<TrialRequest, ExitCode> {
    let Cmd::Trial {
        task,
        agent,
        deadline_ms,
        concurrency,
        artifacts_dir,
        argv,
        seed,
        fixture_time,
        agent_model,
        agent_config_id,
        public_argv,
        wrapper_profile,
        worker_context,
        ..
    } = cmd
    else {
        unreachable!("only a trial is prepared");
    };
    let fixture = match provenance::FixtureInputs::resolve(seed.as_deref(), fixture_time.as_deref())
    {
        Ok(v) => v,
        Err(e) => return Err(fail(e)),
    };
    if *public_argv && argv.iter().any(|v| v.to_str().is_none()) {
        return Err(fail("public_argv_requires_utf8"));
    }
    if agent
        .as_deref()
        .is_some_and(|s| runner::BUILTINS.contains(&s))
        && (agent_model.is_some()
            || agent_config_id.is_some()
            || *public_argv
            || wrapper_profile.is_some())
    {
        return Err(fail("builtin_declaration_invalid"));
    }
    let mut req = TrialRequest {
        task: task.clone(),
        agent: agent.clone(),
        argv: argv.clone(),
        deadline_ms: *deadline_ms,
        concurrency: Some(*concurrency),
        artifacts_dir: artifacts_dir.clone(),
        fixture,
        agent_model: agent_model.clone(),
        agent_config_id: agent_config_id.clone(),
        public_argv: *public_argv,
        wrapper_profile: *wrapper_profile,
        ..Default::default()
    };
    if let Some(context) = worker_context {
        if context.len() > artifacts::SMALL_LIMIT {
            return Err(fail("worker_context_invalid"));
        }
        let context: WorkerContext = match provenance::decode(context.as_bytes()) {
            Ok(c) => c,
            Err(_) => return Err(fail("worker_context_invalid")),
        };
        if context.apply(&mut req).is_err() {
            return Err(fail("worker_context_invalid"));
        }
    }
    Ok(req)
}

/// What a validated trial will do.
enum Prepared {
    /// Run the trial arm in this process, as always.
    Run(TrialRequest),
    /// Run it inside bubblewrap: the controller stays here, and the sandbox
    /// runs `inner` (the runner's own argv) against the unchanged trial arm.
    Isolate {
        req: TrialRequest,
        request: isolation::Request,
        inner: Vec<OsString>,
    },
}

/// Validate the trial and, for `--isolated`, its boundary preconditions.
/// `Err` is the exit code of a trial that must not run: one harness-error
/// row on stdout and the reason on stderr, before any server or agent
/// exists. Nothing here falls back to an unisolated run.
fn prepare_trial(
    cmd: &Cmd,
    swarmail_bin: Result<Option<PathBuf>, String>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<Prepared, u8> {
    let req = trial_request(cmd).map_err(|_| 2u8)?;
    let Cmd::Trial {
        isolated: true,
        isolated_expose,
        ..
    } = cmd
    else {
        return Ok(Prepared::Run(req));
    };
    match isolate(&req, isolated_expose, swarmail_bin) {
        Ok(request) => {
            let inner = inner_args(cmd, &request);
            Ok(Prepared::Isolate {
                req,
                request,
                inner,
            })
        }
        Err(refusal) => Err(refuse(&req, &refusal.to_string(), out, err)),
    }
}

/// One harness-error row on stdout, the reason on stderr, and the row's exit
/// code. No writable host path enters or leaves the boundary: the refusal is
/// the row, never a retained artifact.
fn refuse(req: &TrialRequest, reason: &str, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let id = req.trial_id.clone().unwrap_or_else(runner::new_trial_id);
    let _ = writeln!(err, "swarmail-evals: harness error: {reason}");
    let bare = TrialRequest {
        artifacts_dir: None,
        ..req.clone()
    };
    let row = runner::finalize(&bare, runner::refused_row(&bare, &id), None);
    let _ = writeln!(out, "{}", serde_json::to_string(&row).expect("serializes"));
    let _ = out.flush();
    row.exit_code() as u8
}

/// The runner's own argv inside the sandbox: its executable, `trial`, and
/// every field of the parsed request rendered as its flag (`--flag=value`,
/// so a value that starts with `-` stays a value), without `--isolated` and
/// `--isolated-expose`, then `--` and the agent argv verbatim. The pattern
/// names every field and has no `..`: a field added to `Cmd::Trial` fails to
/// compile here until it is rendered. A relative or bare agent program is
/// replaced by the absolute path the controller resolved, since inside the
/// sandbox the caller's cwd and PATH are gone; an absolute one is untouched.
fn inner_args(cmd: &Cmd, request: &isolation::Request) -> Vec<OsString> {
    let Cmd::Trial {
        task,
        agent,
        deadline_ms,
        concurrency,
        artifacts_dir,
        seed,
        fixture_time,
        agent_model,
        agent_config_id,
        public_argv,
        wrapper_profile,
        worker_context,
        isolated: _,
        isolated_expose: _,
        argv,
    } = cmd
    else {
        unreachable!("only a trial is isolated");
    };
    let mut args = vec![request.runner.clone().into_os_string(), "trial".into()];
    let mut flag = |name: &str, value: Option<OsString>| {
        if let Some(value) = value {
            let mut arg = OsString::from(format!("--{name}="));
            arg.push(value);
            args.push(arg);
        }
    };
    flag("task", task.clone().map(OsString::from));
    flag("agent", agent.clone().map(OsString::from));
    flag("deadline-ms", deadline_ms.map(|v| v.to_string().into()));
    flag("concurrency", Some(concurrency.to_string().into()));
    flag(
        "artifacts-dir",
        artifacts_dir.clone().map(PathBuf::into_os_string),
    );
    flag("seed", seed.clone().map(OsString::from));
    flag("fixture-time", fixture_time.clone().map(OsString::from));
    flag("agent-model", agent_model.clone().map(OsString::from));
    flag(
        "agent-config-id",
        agent_config_id.clone().map(OsString::from),
    );
    flag(
        "wrapper-profile",
        wrapper_profile.map(|profile| {
            let value = clap::ValueEnum::to_possible_value(&profile).expect("no skipped variant");
            OsString::from(value.get_name())
        }),
    );
    flag("worker-context", worker_context.clone().map(OsString::from));
    if *public_argv {
        args.push("--public-argv".into());
    }
    if !argv.is_empty() {
        args.push("--".into());
        let mut program = argv.clone();
        if let (Some(resolved), Some(first)) = (&request.agent, program.first_mut())
            && !std::path::Path::new(first).is_absolute()
        {
            *first = resolved.clone().into_os_string();
        }
        args.extend(program);
    }
    args
}

/// The preconditions of an isolated trial. The boundary itself (bwrap, its
/// version, its probe) is the controller's, and refuses later and by name.
fn isolate(
    req: &TrialRequest,
    expose: &[PathBuf],
    swarmail_bin: Result<Option<PathBuf>, String>,
) -> Result<isolation::Request, isolation::Refusal> {
    use isolation::Refusal;
    if req.concurrency.unwrap_or(1) != 1 {
        return Err(Refusal::Named("concurrency_unsupported"));
    }
    let source_invalid = || Refusal::Named(isolation::SOURCE_INVALID);
    // The Swarmail selector and the agent program are resolved here, at the
    // caller's cwd and PATH; their directories are made canonical so the
    // bind names the real file (a symlinked final component still refuses).
    let canonical = |path: PathBuf| -> Result<PathBuf, Refusal> {
        let name = path.file_name().ok_or_else(source_invalid)?.to_owned();
        let parent = path.parent().ok_or_else(source_invalid)?;
        Ok(std::fs::canonicalize(parent)
            .map_err(|_| source_invalid())?
            .join(name))
    };
    let swarmail_bin = swarmail_bin
        .map_err(|_| source_invalid())?
        .map(canonical)
        .transpose()?;
    let builtin = req
        .agent
        .as_deref()
        .is_some_and(|s| runner::BUILTINS.contains(&s));
    let agent = match req.argv.first() {
        Some(program) if !builtin => Some(canonical(
            runner::selected_agent_program(program).map_err(|_| source_invalid())?,
        )?),
        _ => None,
    };
    let runner_exe = std::env::current_exe().map_err(|_| source_invalid())?;
    isolation::request(
        swarmail_bin,
        req.artifacts_dir.is_some(),
        req.wrapper_profile.is_some(),
        expose,
        runner_exe,
        agent,
    )
}

/// Launch the isolated trial and turn how it ended into the process's exit
/// code. A row that reached stdout was printed by the runner inside the
/// sandbox and its exit code is mirrored; every other end is one named
/// harness-error row here. `bwrap` is the parameter production fills with the
/// constant path.
async fn run_isolated(
    req: &TrialRequest,
    request: &isolation::Request,
    inner: &[OsString],
    bwrap: &std::path::Path,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let launched = isolation::launch(request, inner, bwrap).await;
    conclude(req, launched, out, err)
}

/// How a launch ended, as the process's exit code and, where no row came
/// from the runner, the one harness-error row.
fn conclude(
    req: &TrialRequest,
    launched: Result<isolation::Launched, isolation::Refusal>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    use isolation::Launched;
    match launched {
        Ok(Launched::Exited(code)) => code,
        Ok(Launched::NoRow) => refuse(req, isolation::NO_ROW, out, err),
        Ok(Launched::Interrupted) => refuse(req, isolation::INTERRUPTED, out, err),
        Err(refusal) => refuse(req, &refusal.to_string(), out, err),
    }
}

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {
        eprintln!("swarmail-evals: harness error: panicked");
    }));
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            let code = error.exit_code();
            let _ = error.print();
            return ExitCode::from(code as u8);
        }
        Err(_) => {
            eprintln!("swarmail-evals: harness error: arguments_invalid");
            return ExitCode::from(2);
        }
    };
    // A trial's request is validated before any runtime exists. An isolated
    // one is handed to the bubblewrap controller and never runs the trial arm
    // in this process.
    let mut prepared = None;
    if let Cmd::Trial { .. } = &cli.command {
        match prepare_trial(
            &cli.command,
            runner::selected_swarmail_bin(),
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ) {
            Ok(Prepared::Run(req)) => prepared = Some(req),
            Ok(Prepared::Isolate {
                req,
                request,
                inner,
            }) => {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return fail("runtime unavailable"),
                };
                // The future is driven from this, the main, thread: bwrap's
                // parent-death signal follows the thread that spawned it.
                let code = rt.block_on(run_isolated(
                    &req,
                    &request,
                    &inner,
                    std::path::Path::new(isolation::BWRAP),
                    &mut std::io::stdout(),
                    &mut std::io::stderr(),
                ));
                return ExitCode::from(code);
            }
            Err(code) => return ExitCode::from(code),
        }
    }
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return fail("runtime unavailable"),
    };
    match cli.command {
        Cmd::Baseline {
            config,
            output,
            dry_run,
        } => rt.block_on(run_baseline(config, output, dry_run)),
        Cmd::AgentClaude { model, config } => wrapper_result("claude", model, config),
        Cmd::AgentCodex { model, config } => wrapper_result("codex", model, config),
        Cmd::AgentDsh { model, config } => wrapper_result("dsh", model, config),
        Cmd::FixtureHttpIsolated => match wrappers::isolated_helper() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => fail("helper_setup_failed"),
        },
        Cmd::FixtureHttp => match wrappers::Inputs::environment() {
            Ok(inputs) => match rt.block_on(fixture_http::serve(inputs)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            },
            Err(e) => fail(e),
        },
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
            seed,
            fixture_time,
        } => match rt.block_on(fixture::run_cli_with_inputs(
            smtp,
            &task,
            &trial_id,
            match provenance::FixtureInputs::resolve(seed.as_deref(), fixture_time.as_deref()) {
                Ok(v) => v,
                Err(e) => return fail(e),
            },
        )) {
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
                    Err(_) => {
                        eprintln!("swarmail-evals: harness error: oracle_failed");
                        ExitCode::from(1)
                    }
                }
            }
            other => fail(format!("no built-in agent named {other}")),
        },
        Cmd::Trial { .. } => {
            let req = prepared.take().expect("prepared before the runtime");
            let row = rt.block_on(runner::run_trial(req, None));
            print_line(&row);
            ExitCode::from(row.exit_code() as u8)
        }
        Cmd::Matrix {
            tasks: ids,
            agents,
            trials,
            deadline_ms,
            concurrency,
            artifacts_dir,
            seed,
            fixture_time,
        } => {
            if concurrency != 1 {
                eprintln!("swarmail-evals: harness error: concurrency_unsupported");
                return ExitCode::from(2);
            }
            let fixture = match provenance::FixtureInputs::resolve(
                seed.as_deref(),
                fixture_time.as_deref(),
            ) {
                Ok(v) => v,
                Err(e) => return fail(e),
            };
            rt.block_on(matrix_with_options(
                ids,
                agents,
                trials,
                deadline_ms,
                fixture,
                artifacts_dir,
            ))
        }
        Cmd::Check {
            task,
            state,
            artifacts_dir,
            matrix,
        } => {
            if task.is_none() && state.is_none() && artifacts_dir.is_some() != matrix.is_some() {
                let replay = match (artifacts_dir, matrix) {
                    (Some(root), None) => artifacts::trial(&root).map(|v| vec![v.replay()]),
                    (None, Some(root)) => artifacts::matrix(&root),
                    _ => unreachable!(),
                };
                return match replay {
                    Ok(rows) => {
                        for row in &rows {
                            print_line(row);
                        }
                        ExitCode::from(artifacts::replay_exit(&rows) as u8)
                    }
                    Err(_) => fail("artifact_evidence_invalid"),
                };
            }
            if artifacts_dir.is_some() || matrix.is_some() {
                return fail("check_arguments_invalid");
            }
            let (Some(task), Some(state)) = (task, state) else {
                return fail("check_arguments_invalid");
            };
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
                    // Only fixed contract metadata crosses the private diagnostic boundary.
                    if e.starts_with("snapshot schema ") {
                        eprintln!(
                            "swarmail-evals: unsupported snapshot schema version; expected {}",
                            state::SNAPSHOT_SCHEMA
                        );
                    }
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
                Err(e) => {
                    if e.split_once(": ")
                        .is_some_and(|(_, reason)| reason.starts_with("schema_version "))
                    {
                        eprintln!(
                            "swarmail-evals: unsupported result schema version; expected {}",
                            runner::RESULT_SCHEMA
                        );
                    }
                    fail(e)
                }
            }
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerContext {
    cell: artifacts::PlannedCell,
    target: provenance::ExecutableIdentity,
    agent: provenance::AgentIdentity,
}
impl WorkerContext {
    fn apply(self, req: &mut TrialRequest) -> Result<(), String> {
        req.matrix = Some(self.cell.coordinates.clone());
        req.trial_id = Some(self.cell.trial_id.clone());
        let expected = runner::make_provenance(req, None, None).agent;
        if expected.kind != self.agent.kind
            || expected.model != self.agent.model
            || expected.config_id != self.agent.config_id
            || expected.command != self.agent.command
            || expected.tool_surface != self.agent.tool_surface
        {
            return Err("worker_agent_context_invalid".into());
        }
        req.assigned_agent = Some(self.agent);
        let path = runner::selected_swarmail_bin()?.ok_or("worker_target_missing")?;
        let target = identity::TargetBuild {
            path,
            identity: self.target,
        };
        target.recheck()?;
        req.target_build = Some(target);
        let row = runner::refused_row(req, &self.cell.trial_id);
        artifacts::matches_cell(&row, &self.cell)?;
        Ok(())
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
#[cfg(all(test, target_os = "linux"))]
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
        .and_then(|v| v.parse::<usize>().ok())
        != Some(check)
    {
        return;
    }
    // Separate observers acknowledge delivery without consuming the production
    // receivers. The real pending check must still observe its own queued event.
    let mut observed_term =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("probe TERM subscription");
    let mut observed_int =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("probe INT subscription");
    if std::fs::create_dir_all(&dir).is_err()
        || std::fs::write(dir.join("entered"), check.to_string()).is_err()
    {
        return;
    }
    while !dir.join("release").exists() {
        tokio::select! {
            biased;
            _ = observed_term.recv() => { let _ = std::fs::write(dir.join("signal-term"),b"queued");},
            _ = observed_int.recv() => { let _ = std::fs::write(dir.join("signal-int"),b"queued");},
            _ = tokio::time::sleep(Duration::from_millis(5)) => {},
        }
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

fn assigned_cell(
    ids: &[String],
    specs: &[report::AgentSpec],
    trials: u32,
    index: usize,
    master: &provenance::FixtureInputs,
    matrix_id: &str,
    deadline_ms: Option<u64>,
) -> artifacts::PlannedCell {
    let (task, spec) = matrix_slot(ids, specs, trials, index);
    let coordinates = provenance::MatrixCoordinates {
        matrix_id: matrix_id.into(),
        master_seed: master.seed.clone(),
        task_index: (index / (specs.len() * trials as usize)) as u64,
        agent_index: ((index / trials as usize) % specs.len()) as u64,
        repetition_index: (index % trials as usize) as u64,
    };
    let mut fixture = master.clone();
    fixture.seed = coordinates
        .cell_seed(task, &spec.label)
        .expect("validated master seed");
    artifacts::PlannedCell {
        trial_id: runner::new_trial_id(),
        task: task.into(),
        agent: spec.label.clone(),
        fixture,
        coordinates,
        deadline_ms: deadline_ms
            .unwrap_or_else(|| tasks::find(task).expect("validated task").deadline_ms),
    }
}
fn cell_request(
    cell: &artifacts::PlannedCell,
    spec: &report::AgentSpec,
    target: Option<&identity::TargetBuild>,
) -> TrialRequest {
    TrialRequest {
        task: Some(cell.task.clone()),
        agent: Some(cell.agent.clone()),
        argv: spec.argv.iter().map(OsString::from).collect(),
        deadline_ms: Some(cell.deadline_ms),
        fixture: cell.fixture.clone(),
        matrix: Some(cell.coordinates.clone()),
        trial_id: Some(cell.trial_id.clone()),
        agent_model: spec.model.clone(),
        agent_config_id: spec.config_id.clone(),
        public_argv: spec.public_argv,
        wrapper_profile: spec.wrapper_profile,
        target_build: target.cloned(),
        ..Default::default()
    }
}
struct MatrixEvidence {
    root: PathBuf,
    plan: artifacts::Binding,
    metadata: Vec<artifacts::Binding>,
    failed: bool,
}
impl MatrixEvidence {
    fn start(root: &std::path::Path, plan: &artifacts::MatrixPlan) -> Result<Self, String> {
        artifacts::validate_plan(plan)?;
        artifacts::ensure_root(root)?;
        let root = artifacts::create_private_dir(root, &plan.matrix_id)?;
        artifacts::create_private_dir(&root, "trials")?;
        let binding = artifacts::publish(
            &root,
            "matrix-plan.json",
            &artifacts::encode(plan, artifacts::PLAN_LIMIT)?,
        )?;
        Ok(Self {
            root,
            plan: binding,
            metadata: Vec::new(),
            failed: false,
        })
    }
    fn committed(&mut self, req: &TrialRequest, row: &TrialResult) {
        if self.failed {
            return;
        }
        let trials = self.root.join("trials");
        let leaf = trials.join(&row.trial_id);
        let result = (|| {
            match std::fs::symlink_metadata(&leaf) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    artifacts::retain_trial(&trials, row, None)?;
                }
                Err(_) => return Err("artifact_unavailable".into()),
                Ok(_) => {}
            }
            let retained = artifacts::trial(&leaf)?;
            if retained.row != *row || req.trial_id.as_deref() != Some(row.trial_id.as_str()) {
                return Err("artifact_commit_conflict".into());
            }
            artifacts::metadata_binding(&self.root, &row.trial_id)
        })();
        match result {
            Ok(binding) => self.metadata.push(binding),
            Err(_) => {
                self.failed = true;
                eprintln!("swarmail-evals: artifact_publication_failed");
            }
        }
    }
    fn finish(self, matrix_id: &str, total: usize) -> Result<(), String> {
        if self.failed || self.metadata.len() != total {
            return Err("matrix_incomplete".into());
        }
        let index = artifacts::MatrixIndex {
            schema_version: 1,
            matrix_id: matrix_id.into(),
            plan: self.plan,
            metadata: self.metadata,
        };
        artifacts::publish(
            &self.root,
            "matrix.json",
            &artifacts::encode(&index, artifacts::PLAN_LIMIT)?,
        )?;
        Ok(())
    }
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
    start_output_writer_to(None)
}

fn start_output_writer_to(file: Option<std::fs::File>) -> OutputWriter {
    let (tx, rx) = std::sync::mpsc::sync_channel::<OutputMessage>(1);
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    let thread_wake = wake.clone();
    std::thread::spawn(move || {
        let mut out: Box<dyn Write + Send> = match file {
            Some(file) => Box::new(file),
            None => Box::new(std::io::stdout()),
        };
        while let Ok(message) = rx.recv() {
            thread_wake.notify_one();
            let result = out
                .write_all(&message.bytes)
                .and_then(|_| out.flush())
                .map_err(|_| "stdout write failed".to_string());
            let failed = result.is_err();
            let _ = message.ack.send(result);
            if failed {
                break;
            }
        }
    });
    OutputWriter { tx, wake }
}

#[cfg(test)]
fn stdout_would_block(_payload: usize) -> bool {
    #[cfg(target_os = "linux")]
    {
        // Pipe pages can be exhausted before the nominal byte capacity. Poll
        // observes that pressure without changing descriptor flags or draining.
        let mut fd = libc::pollfd {
            fd: libc::STDOUT_FILENO,
            events: libc::POLLOUT,
            revents: 0,
        };
        (unsafe { libc::poll(&mut fd, 1, 0) }) == 0
    }
    #[cfg(not(target_os = "linux"))]
    true
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
    let queued_len = bytes.len();
    #[cfg(test)]
    let (ack_tx, mut ack_rx) = tokio::sync::oneshot::channel();
    #[cfg(not(test))]
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    #[cfg(test)]
    let mut ack_pending_recorded = false;
    #[cfg(test)]
    let ack_pending = std::future::poll_fn(|cx| {
        let result = std::pin::Pin::new(&mut ack_rx).poll(cx);
        if result.is_pending() && !ack_pending_recorded && stdout_would_block(queued_len) {
            matrix_test_event(
                MATRIX_OUTPUT_ACK_PENDING_ENV,
                "writer acknowledgement is pending",
            );
            ack_pending_recorded = true;
        } else if result.is_pending() && !ack_pending_recorded {
            let waker = cx.waker().clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                waker.wake();
            });
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
    let row_bytes = captured
        .bytes
        .strip_suffix(b"\n")
        .unwrap_or(&captured.bytes);
    if row_bytes.is_empty() || row_bytes.contains(&b'\n') || row_bytes.contains(&b'\r') {
        return Err("trial_result_line_invalid".into());
    }
    let row: TrialResult =
        serde_json::from_slice(row_bytes).map_err(|_| "trial_result_json_invalid")?;
    row.validate().map_err(|_| "trial_result_schema_invalid")?;
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

async fn matrix_envelope(at: tokio::time::Instant) {
    #[cfg(test)]
    if let Some(path) = std::env::var_os("EVAL_TEST_MATRIX_ENVELOPE_RELEASE") {
        let path = PathBuf::from(path);
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        return;
    }
    tokio::time::sleep_until(at).await;
}

// Only these complete harness-authored categories cross the worker boundary.
// All other stderr bytes are drained and discarded (#196), including long lines.
async fn capture_worker_category<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
) -> std::io::Result<Option<&'static str>> {
    let mut chunk = [0u8; 8192];
    let mut line = Vec::with_capacity(128);
    let mut oversized = false;
    let mut category = None;
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            return Ok(category);
        }
        for &byte in &chunk[..n] {
            if byte == b'\n' {
                if !oversized {
                    for name in [
                        "concurrent_trials_refused",
                        "trial_admission_unavailable",
                        "trial_cleanup_unproved",
                    ] {
                        if line == format!("swarmail-evals: harness error: {name}").as_bytes()
                            && (category.is_none() || name == "trial_cleanup_unproved")
                        {
                            category = Some(name);
                        }
                    }
                }
                line.clear();
                oversized = false;
            } else if line.len() < 128 && !oversized {
                line.push(byte);
            } else {
                oversized = true;
            }
        }
    }
}

fn signal_trial(pid: u32) -> Result<(), String> {
    if unsafe { libc::kill(pid as i32, libc::SIGTERM) } == 0 {
        return Ok(());
    }
    let e = std::io::Error::last_os_error();
    if e.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err("trial_signal_failed".into())
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_child_assigned(
    exe: &std::path::Path,
    bin: &std::path::Path,
    id: &str,
    spec: &report::AgentSpec,
    argv: &[OsString],
    deadline_ms: Option<u64>,
    context: Option<&WorkerContext>,
    retention: Option<&std::path::Path>,
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
) -> ChildRun {
    let started = Instant::now();
    #[cfg(test)]
    if let Some(path) = std::env::var_os(MATRIX_TRIAL_START_ENV) {
        let _ = std::fs::write(path, matrix_test_monotonic_nanos().to_string());
    }
    let setup = runner::private_tempdir("swarmail-eval-matrix-").and_then(|owner| {
        let dirs = runner::RuntimeDirs::create(owner.path(), "worker")?;
        Ok((owner, dirs))
    });
    let (_runtime, dirs) = match setup {
        Ok(setup) => setup,
        Err(_) => {
            return ChildRun {
                row: None,
                interrupted: false,
                failure: Some("private_directory_unavailable".into()),
                stop_admission: false,
                elapsed_ms: 0,
            };
        }
    };
    let mut cmd = Command::new(exe);
    #[cfg(test)]
    let bypass_wait = std::env::var_os("EVAL_TEST_MATRIX_BYPASS_WAIT").is_some();
    #[cfg(not(test))]
    let bypass_wait = false;
    dirs.apply(&mut cmd);
    cmd.process_group(0)
        .args(["trial", "--task", id, "--agent", &spec.label])
        .env(runner::SWARMAIL_BIN_ENV, bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(!bypass_wait);
    if let Some(ms) = deadline_ms {
        cmd.args(["--deadline-ms", &ms.to_string()]);
    }
    if let Some(context) = context {
        cmd.args([
            "--seed",
            &context.cell.fixture.seed,
            "--fixture-time",
            &context.cell.fixture.fixture_time,
        ]);
        let bytes = match artifacts::encode(context, artifacts::SMALL_LIMIT) {
            Ok(b) => b,
            Err(_) => {
                return ChildRun {
                    row: None,
                    interrupted: false,
                    failure: Some("worker_context_invalid".into()),
                    stop_admission: false,
                    elapsed_ms: 0,
                };
            }
        };
        cmd.arg("--worker-context")
            .arg(String::from_utf8(bytes).expect("JSON UTF-8"));
        if let Some(root) = retention {
            cmd.arg("--artifacts-dir").arg(root);
        }
    }
    #[cfg(test)]
    if std::env::var_os("EVAL_TEST_MATRIX_SYNTHETIC_ROW").is_some()
        && let Some(context) = context
    {
        let req = TrialRequest {
            task: Some(id.into()),
            agent: Some(spec.label.clone()),
            fixture: context.cell.fixture.clone(),
            matrix: Some(context.cell.coordinates.clone()),
            deadline_ms: Some(context.cell.deadline_ms),
            trial_id: Some(context.cell.trial_id.clone()),
            assigned_agent: Some(context.agent.clone()),
            target_build: Some(identity::TargetBuild {
                path: bin.into(),
                identity: context.target.clone(),
            }),
            ..Default::default()
        };
        let mut error = runner::refused_row(&req, &context.cell.trial_id);
        error.wall_time_ms = 1;
        let mut pass = error.clone();
        pass.outcome = Outcome::Pass;
        pass.failure_class = None;
        pass.agent_exit_status = Some(0);
        pass.wall_time_ms = 1;
        cmd.args([
            "--test-error-row",
            &serde_json::to_string(&error).unwrap(),
            "--test-pass-row",
            &serde_json::to_string(&pass).unwrap(),
        ]);
    }
    if let Some(model) = &spec.model {
        cmd.args(["--agent-model", model]);
    }
    if let Some(config) = &spec.config_id {
        cmd.args(["--agent-config-id", config]);
    }
    if spec.public_argv {
        cmd.arg("--public-argv");
    }
    if let Some(profile) = spec.wrapper_profile {
        cmd.args(["--wrapper-profile", profile.name()]);
    }
    if !argv.is_empty() {
        cmd.arg("--").args(argv);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(_) => {
            return ChildRun {
                row: None,
                interrupted: false,
                failure: Some("trial_spawn_failed".into()),
                stop_admission: false,
                elapsed_ms: started.elapsed().as_millis() as u64,
            };
        }
    };
    #[cfg(test)]
    matrix_spawn_test_hold().await;
    let pid = child.id().expect("spawned trial has a PID");
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let mut reader = tokio::spawn(async move {
        let (output, category) =
            tokio::join!(capture_stdout(stdout), capture_worker_category(stderr));
        Ok::<_, std::io::Error>((output?, category?))
    });
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
    let envelope = deadline_ms
        .unwrap_or_else(|| tasks::find(id).expect("validated task").deadline_ms)
        .saturating_add(300000);
    let envelope_at = tokio::time::Instant::now() + Duration::from_millis(envelope);
    let mut envelope_requested = false;
    let expiry = matrix_envelope(envelope_at);
    tokio::pin!(expiry);
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
            _ = &mut expiry, if !envelope_requested => {
                envelope_requested = true;
                failure = Some("trial_envelope_expired".into());
                if !forwarded { forwarded = true; let _ = signal_trial(pid); }
                #[cfg(test)] {
                    matrix_test_event("EVAL_TEST_MATRIX_ENVELOPE_REQUESTED", "requested");
                    if bypass_wait { break None; }
                }
            },
            result = &mut wait => match result {
                Ok(status) => break Some(status),
                Err(_) => { failure = Some("trial_wait_failed".into()); break None; }
            },
        }
    };
    #[cfg(test)]
    if status.is_some() {
        matrix_test_event(MATRIX_CHILD_REAPED_ENV, "trial process wait completed");
    }
    let mut worker_category = None;
    let captured = if status.is_some() {
        let deadline = tokio::time::Instant::now() + MATRIX_OUTPUT_EOF_BOUND;
        loop {
            tokio::select! {
                biased;
                _ = term.recv() => interrupted = true,
                _ = int.recv() => interrupted = true,
                result = &mut reader => break match result {
                    Ok(Ok((output, category))) => { worker_category = category; Some(output) },
                    Ok(Err(_)) => { failure = Some("trial_stdout_read_failed".into()); None }
                    Err(_) => { failure = Some("trial_stdout_reader_failed".into()); None }
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
                Ok(row) => {
                    if context.is_some_and(|c| {
                        artifacts::matches_cell(&row, &c.cell).is_err()
                            || row.provenance.agent != c.agent
                            || row.provenance.target.executable_sha256 != c.target.executable_sha256
                            || row.provenance.target.build_commit != c.target.build_commit
                            || row.provenance.target.source_dirty != c.target.source_dirty
                    }) {
                        failure.get_or_insert("trial_context_mismatch".into());
                        None
                    } else {
                        Some(row)
                    }
                }
                Err(e) => {
                    failure.get_or_insert(e);
                    None
                }
            }
        }
        _ => None,
    };
    if let Some(category) = worker_category {
        failure.get_or_insert_with(|| category.to_string());
    }
    let stop_admission = status.is_none() || !output_available || failure.is_some();
    ChildRun {
        row,
        interrupted,
        failure,
        stop_admission,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn run_child(
    exe: &std::path::Path,
    bin: &std::path::Path,
    id: &str,
    spec: &report::AgentSpec,
    argv: &[OsString],
    deadline_ms: Option<u64>,
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
) -> ChildRun {
    run_child_assigned(exe, bin, id, spec, argv, deadline_ms, None, None, term, int).await
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
    Records(Result<Result<PathBuf, String>, tokio::sync::oneshot::error::RecvError>),
    Status(std::io::Result<std::process::ExitStatus>),
    OutputTimedOut,
}

async fn matrix_source(
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
    interrupted: &mut bool,
) -> Result<Option<identity::SourceContext>, String> {
    let groups = runner::Groups::default();
    let owned = groups.clone();
    let mut body =
        tokio::spawn(
            async move { runner::owned_source_context(runner::source_root(), &owned).await },
        );
    let result = tokio::select! {
        biased;
        _ = term.recv() => { *interrupted = true; None },
        _ = int.recv() => { *interrupted = true; None },
        result = &mut body => Some(result.unwrap_or_else(|_| Err("source_context_failed".into()))),
    };
    if result.is_none() {
        body.abort();
        let _ = body.await;
    }
    #[cfg(test)]
    matrix_test_hold(
        "EVAL_TEST_MATRIX_SOURCE_COMMIT_HOLD",
        term,
        int,
        interrupted,
    )
    .await;
    let cleanup = groups.cleanup_all();
    tokio::pin!(cleanup);
    loop {
        tokio::select! { biased; result = &mut cleanup => { result?; break; }, _ = term.recv() => *interrupted = true, _ = int.recv() => *interrupted = true, }
    }
    if *interrupted {
        Ok(None)
    } else {
        result.transpose()
    }
}

async fn resolve_matrix_bin(
    term: &mut tokio::signal::unix::Signal,
    int: &mut tokio::signal::unix::Signal,
    interrupted: &mut bool,
) -> Result<Option<(PathBuf, identity::TargetBuild)>, String> {
    let exe = std::env::current_exe().map_err(|_| "program_missing")?;
    if let Some(path) = runner::selected_swarmail_bin()? {
        return Ok(Some((exe, identity::TargetBuild::external(path))));
    }
    let Some(before) = matrix_source(term, int, interrupted).await? else {
        return Ok(None);
    };
    let (cmd, runtime) = runner::prepare_swarmail_build()?;
    let groups = runner::Groups::default();
    let mut cargo =
        runner::spawn_owned_cargo(cmd, &groups, runtime).map_err(|_| "cargo_spawn_failed")?;
    let mut records = cargo.cargo_records.take().ok_or("cargo_output_missing")?;
    let mut records_error = None;
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
            result = &mut records, if !output_closed => CargoEvent::Records(result),
            result = cargo.child.wait(), if status.is_none() => CargoEvent::Status(result),
            _ = output_deadline(deadline), if status_at.is_some() && !output_closed => CargoEvent::OutputTimedOut,
        };
        match event {
            CargoEvent::Signal => {
                *interrupted = true;
                if let Err(e) = cleanup_cargo(&cargo, term, int, interrupted).await {
                    eprintln!(
                        "swarmail-evals: fallback cargo cleanup failed: {}",
                        runner::safe_cleanup_category(&e)
                    );
                }
                return Ok(None);
            }
            CargoEvent::Records(result) => {
                output_closed = true;
                match result {
                    Ok(Ok(path)) => binary = Some(path),
                    Ok(Err(error)) => records_error = Some(error),
                    Err(_) => records_error = Some("cargo_output_reader_failed".into()),
                }
            }
            CargoEvent::Status(Ok(exit)) => {
                status = Some(exit);
                status_at = Some(tokio::time::Instant::now());
            }
            CargoEvent::Status(Err(_)) => {
                let cleanup = cleanup_cargo(&cargo, term, int, interrupted).await;
                return Err(format!("cargo_wait_failed; cleanup={}", cleanup.is_ok()));
            }
            CargoEvent::OutputTimedOut => {
                let cleanup = cleanup_cargo(&cargo, term, int, interrupted).await;
                return Err(format!(
                    "cargo_stdout_close_timeout; cleanup={}",
                    cleanup.is_ok()
                ));
            }
        }
    }
    if let Err(e) = cleanup_cargo(&cargo, term, int, interrupted).await {
        return Err(format!(
            "fallback cargo cleanup failed: {}",
            runner::safe_cleanup_category(&e)
        ));
    }
    if *interrupted {
        return Ok(None);
    }
    let status = status.expect("loop exits only after the cargo child was reaped");
    if !status.success() {
        return Err(format!("building swarmail failed: {status}"));
    }
    if let Some(error) = records_error {
        return Err(error);
    }
    let binary = binary.ok_or_else(|| "cargo reported no swarmail executable".to_string())?;
    let Some(after) = matrix_source(term, int, interrupted).await? else {
        return Ok(None);
    };
    if before != after {
        return Err("build_context_changed".into());
    }
    let target = before.associate(binary);
    target.recheck()?;
    Ok(Some((exe, target)))
}

// Only the Linux-only matrix probe entrypoint drives the matrix this way.
#[cfg(all(test, target_os = "linux"))]
async fn matrix(
    ids: Vec<String>,
    agents: PathBuf,
    trials: u32,
    deadline_ms: Option<u64>,
) -> ExitCode {
    matrix_with_options(ids, agents, trials, deadline_ms, Default::default(), None).await
}

async fn matrix_with_options(
    ids: Vec<String>,
    agents: PathBuf,
    trials: u32,
    deadline_ms: Option<u64>,
    master: provenance::FixtureInputs,
    retention: Option<PathBuf>,
) -> ExitCode {
    if trials == 0 {
        return fail("--trials must be positive");
    }
    if let Some(id) = ids.iter().find(|id| tasks::find(id).is_none()) {
        return fail(format!("unknown task {id}"));
    }
    let agent_text = if retention.is_some() {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&agents)
            .and_then(|f| f.take(1024 * 1024 + 1).read_to_end(&mut bytes))
            .map_err(|_| "agents_file_unreadable".to_string())
            .and_then(|_| {
                if bytes.len() > 1024 * 1024 {
                    Err("agents_file_too_large".into())
                } else {
                    String::from_utf8(bytes).map_err(|_| "agents_file_invalid".into())
                }
            })
    } else {
        std::fs::read_to_string(&agents).map_err(|_| "agents_file_unreadable".to_string())
    };
    let specs = match agent_text.and_then(|t| report::parse_agents(&t)) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    matrix_run(
        MatrixInvocation {
            ids,
            specs,
            trials,
            deadline_ms,
            master,
            retention,
        },
        &mut MatrixResources::default(),
    )
    .await
}

struct MatrixInvocation {
    ids: Vec<String>,
    specs: Vec<report::AgentSpec>,
    trials: u32,
    deadline_ms: Option<u64>,
    master: provenance::FixtureInputs,
    retention: Option<PathBuf>,
}
#[derive(Default)]
struct MatrixResources {
    signals: Option<(tokio::signal::unix::Signal, tokio::signal::unix::Signal)>,
    writer: Option<OutputWriter>,
    resolved: Option<(PathBuf, identity::TargetBuild)>,
    plan: Option<artifacts::MatrixPlan>,
    interrupted: bool,
}
async fn matrix_run(invocation: MatrixInvocation, resources: &mut MatrixResources) -> ExitCode {
    let MatrixInvocation {
        ids,
        specs,
        trials,
        deadline_ms,
        master,
        retention,
    } = invocation;
    if trials == 0
        || ids.is_empty()
        || specs.is_empty()
        || ids.iter().any(|id| tasks::find(id).is_none())
    {
        return fail("matrix_input_invalid");
    }
    let total_slots = match ids
        .len()
        .checked_mul(specs.len())
        .and_then(|count| count.checked_mul(trials as usize))
    {
        Some(count) => count,
        None => return fail("matrix product cardinality exceeds the addressable range"),
    };
    if deadline_ms.is_some_and(|ms| ms == 0 || ms > tasks::MAX_DEADLINE_MS) {
        return fail("deadline_invalid");
    }
    if retention.is_some() && total_slots > 4096 {
        return fail("retained_matrix_too_large");
    }
    eprintln!(
        "swarmail-evals: seed={} fixture_time={}",
        master.seed, master.fixture_time
    );
    let matrix_id = runner::new_trial_id();
    let planned = retention.as_ref().map(|_| artifacts::MatrixPlan {
        schema_version: 1,
        matrix_id: matrix_id.clone(),
        master: master.clone(),
        tasks: ids.clone(),
        agents: specs.iter().map(|s| s.label.clone()).collect(),
        repetitions: trials,
        cells: (0..total_slots)
            .map(|i| assigned_cell(&ids, &specs, trials, i, &master, &matrix_id, deadline_ms))
            .collect(),
    });
    resources.plan = planned.clone();
    let mut evidence_failed = false;
    let mut evidence = match retention.as_ref().zip(planned.as_ref()) {
        Some((root, plan)) => match MatrixEvidence::start(root, plan) {
            Ok(e) => Some(e),
            Err(_) => {
                evidence_failed = true;
                eprintln!("swarmail-evals: artifact_publication_failed");
                None
            }
        },
        None => None,
    };
    let harness_identity = identity::harness();
    let agent_identities: Vec<_> = specs
        .iter()
        .map(|spec| {
            let cell = assigned_cell(&ids, &specs, trials, 0, &master, &matrix_id, deadline_ms);
            let mut req = cell_request(&cell, spec, None);
            req.agent = Some(spec.label.clone());
            runner::make_provenance(&req, None, None).agent
        })
        .collect();
    let cell_at = |i: usize| {
        planned
            .as_ref()
            .map(|p| p.cells[i].clone())
            .unwrap_or_else(|| {
                assigned_cell(&ids, &specs, trials, i, &master, &matrix_id, deadline_ms)
            })
    };
    let (mut term, mut int) = match resources
        .signals
        .take()
        .map(Ok)
        .unwrap_or_else(install_matrix_signals)
    {
        Ok(signals) => signals,
        Err(_) => {
            eprintln!("swarmail-evals: matrix_signal_setup_failed");
            for index in 0..total_slots {
                let (_, spec) = matrix_slot(&ids, &specs, trials, index);
                let cell = cell_at(index);
                let mut req = cell_request(&cell, spec, None);
                req.harness_identity = Some(harness_identity.clone());
                req.assigned_agent =
                    Some(agent_identities[(index / trials as usize) % specs.len()].clone());
                let row = runner::refused_row(&req, &cell.trial_id);
                print_line(&row);
                if let Some(e) = &mut evidence {
                    e.committed(&req, &row);
                }
            }
            if let Some(e) = evidence {
                let _ = e.finish(&matrix_id, total_slots);
            }
            return ExitCode::from(2);
        }
    };
    let writer = resources.writer.take().unwrap_or_else(start_output_writer);
    let mut interrupted = resources.interrupted;
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
    let resolved = if interrupted || evidence_failed {
        None
    } else if let Some(resolved) = &resources.resolved {
        Some(resolved.clone())
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
    resources.resolved = resolved.clone();
    if let Some(error) = resolution_error.as_deref() {
        let _ = error;
        eprintln!("swarmail-evals: harness error for every trial: target_unavailable");
    }
    let resolved_agents: Vec<Option<Vec<OsString>>> = specs
        .iter()
        .map(|spec| {
            if runner::BUILTINS.contains(&spec.label.as_str()) && spec.argv.is_empty() {
                Some(Vec::new())
            } else {
                runner::resolve_agent_command(&spec.argv).ok()
            }
        })
        .collect();
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
        let cell = cell_at(next_slot);
        let mut req = cell_request(&cell, spec, resolved.as_ref().map(|(_, t)| t));
        if resolution_error.as_deref() == Some("build_context_changed") {
            req.target_source_failure = Some(provenance::Reason::BuildContextChanged);
        }
        req.harness_identity = Some(harness_identity.clone());
        let expected_agent = agent_identities[(next_slot / trials as usize) % specs.len()].clone();
        req.assigned_agent = Some(expected_agent.clone());
        let mut row = if let Some((exe, bin)) = &resolved {
            let agent_index = (next_slot / trials as usize) % specs.len();
            let child = if let Some(argv) = &resolved_agents[agent_index] {
                {
                    let context = WorkerContext {
                        cell: cell.clone(),
                        target: bin.identity.clone(),
                        agent: expected_agent,
                    };
                    let trials_root = evidence.as_ref().map(|e| e.root.join("trials"));
                    run_child_assigned(
                        exe,
                        &bin.path,
                        id,
                        spec,
                        argv,
                        deadline_ms,
                        Some(&context),
                        trials_root.as_deref(),
                        &mut term,
                        &mut int,
                    )
                    .await
                }
            } else {
                ChildRun {
                    row: None,
                    interrupted: false,
                    failure: Some("agent_program_unavailable".into()),
                    stop_admission: false,
                    elapsed_ms: 0,
                }
            };
            let child_interrupted = child.interrupted;
            let child_failed = child.failure.is_some();
            let child_stop = child.stop_admission;
            let mut row = child.row.unwrap_or_else(|| {
                let mut refused = runner::refused_row(&req, &cell.trial_id);
                refused.wall_time_ms = child.elapsed_ms;
                refused
            });
            if let Some(error) = child.failure.as_deref() {
                eprintln!("swarmail-evals: matrix trial error: {error}");
            }
            if child_failed {
                row.outcome = Outcome::HarnessError;
                row.failure_class = Some(state::FailureClass::HarnessError);
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
            runner::refused_row(&req, &cell.trial_id)
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
        if let Some(e) = &mut evidence {
            e.committed(&req, &row);
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
            let (_, spec) = matrix_slot(&ids, &specs, trials, next_slot);
            let cell = cell_at(next_slot);
            let mut req = cell_request(&cell, spec, resolved.as_ref().map(|(_, t)| t));
            req.harness_identity = Some(harness_identity.clone());
            req.assigned_agent =
                Some(agent_identities[(next_slot / trials as usize) % specs.len()].clone());
            let row = runner::refused_row(&req, &cell.trial_id);
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
            if let Some(e) = &mut evidence {
                e.committed(&req, &row);
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
    if let Some(e) = evidence
        && e.finish(&matrix_id, total_slots).is_err()
    {
        evidence_failed = true;
        eprintln!("swarmail-evals: artifact_publication_failed");
    }
    resources.signals = Some((term, int));
    resources.interrupted = interrupted;
    if output_failed || interrupted || any_error || evidence_failed {
        ExitCode::from(2)
    } else if any_fail {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn wrapper_result(harness: &str, model: String, config: PathBuf) -> ExitCode {
    match wrappers::run(harness, &model, &config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            // run returns only finite harness-authored categories.
            eprintln!("swarmail-evals: wrapper refused: {code}; see evals/wrappers.md");
            ExitCode::from(2)
        }
    }
}

fn numeric_status(code: ExitCode) -> u8 {
    if code == ExitCode::SUCCESS {
        0
    } else if code == ExitCode::from(1) {
        1
    } else {
        2
    }
}

fn baseline_leaf(output: &std::path::Path) -> Result<PathBuf, String> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    artifacts::safe_root(parent)?;
    let name = output.file_name().ok_or("baseline_output_invalid")?;
    // This primitive creates exclusively with 0700 before exposing payload.
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let path = parent.join(name);
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .map_err(|_| "baseline_output_exists_or_unavailable")?;
    let dir = std::fs::File::open(&path).map_err(|_| "baseline_output_unavailable")?;
    dir.set_permissions(std::fs::Permissions::from_mode(0o700))
        .map_err(|_| "baseline_output_permissions")?;
    Ok(path)
}
fn baseline_stream(root: &std::path::Path, name: &str) -> Result<std::fs::File, String> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(root.join(name))
        .map_err(|_| "baseline_stream_unavailable")?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(|_| "baseline_stream_permissions")?;
    Ok(f)
}

async fn run_baseline(config: PathBuf, output: PathBuf, dry_run: bool) -> ExitCode {
    let plan = baseline::Plan::new(&output);
    if !config.is_absolute() {
        return fail("baseline_config_locator_invalid");
    }
    if dry_run {
        // Before credential reads, output creation, random fixture resolution,
        // signal setup, preparation, trial or provider process launch.
        print_line(&plan);
        return ExitCode::SUCCESS;
    }
    match baseline_execute(&config, &output, &plan).await {
        Ok(code) => ExitCode::from(code),
        Err(_) => {
            eprintln!("swarmail-evals: baseline refused; evidence kept; see evals/wrappers.md");
            ExitCode::from(2)
        }
    }
}

async fn baseline_execute(
    config: &std::path::Path,
    output: &std::path::Path,
    plan: &baseline::Plan,
) -> Result<u8, String> {
    let signals = install_matrix_signals().map_err(|_| "baseline_signals_unavailable")?;
    let root = baseline_leaf(output)?;
    let mut resources = MatrixResources {
        signals: Some(signals),
        ..Default::default()
    };
    let master = provenance::FixtureInputs::default();
    resources.writer = Some(start_output_writer_to(Some(baseline_stream(
        &root,
        "controls.jsonl",
    )?)));
    let status = numeric_status(
        matrix_run(
            MatrixInvocation {
                ids: plan.tasks.clone(),
                specs: plan.control_agents.clone(),
                trials: 1,
                deadline_ms: Some(plan.deadline_ms),
                master: master.clone(),
                retention: Some(root.join("controls")),
            },
            &mut resources,
        )
        .await,
    );
    let controls = baseline::rows(&artifacts::read_file(
        &root,
        "controls.jsonl",
        4096 * provenance::RESULT_LIMIT,
    )?)?;
    let control_plan = resources
        .plan
        .as_ref()
        .ok_or("baseline_control_plan_missing")?;
    let (exe, target) = resources
        .resolved
        .clone()
        .ok_or("baseline_target_missing")?;
    let harness = identity::harness();
    baseline::validate_controls(
        &controls,
        control_plan,
        plan,
        status,
        &harness,
        &target.identity,
    )?;
    // Validate immutable retained bytes as well as stdout and the in-memory
    // prelaunch plan. A failed retention marker never authorizes paid work.
    let retained = artifacts::matrix(&root.join("controls").join(&control_plan.matrix_id))?;
    if retained.len() != controls.len()
        || retained
            .iter()
            .zip(&controls)
            .any(|(a, b)| a.original_result != *b)
    {
        return Err("baseline_control_retention_invalid".into());
    }
    baseline_transition(&mut resources).await?;
    wrappers::validate_config(config)?;
    let agents = plan.effective_agents(&exe, config)?;
    resources.writer = Some(start_output_writer_to(Some(baseline_stream(
        &root,
        "paid.jsonl",
    )?)));
    let paid_status = numeric_status(
        matrix_run(
            MatrixInvocation {
                ids: plan.tasks.clone(),
                specs: agents.clone(),
                trials: plan.paid_repetitions,
                deadline_ms: Some(plan.deadline_ms),
                master,
                retention: Some(root.join("paid")),
            },
            &mut resources,
        )
        .await,
    );
    let paid = baseline::rows(&artifacts::read_file(
        &root,
        "paid.jsonl",
        4096 * provenance::RESULT_LIMIT,
    )?)?;
    // Strict parsing is needed to report; complete accounting is needed for a
    // successful run. Diagnostic reports may describe an incomplete row set.
    let accounting = resources
        .plan
        .as_ref()
        .ok_or_else(|| "baseline_paid_plan_missing".to_string())
        .and_then(|p| {
            baseline::validate_accounting(
                &paid,
                p,
                &agents,
                &plan.tasks,
                plan.paid_repetitions,
                &harness,
                &target.identity,
            )
        })
        .is_ok();
    let report_ok = artifacts::publish(
        &root,
        "report.json",
        &artifacts::encode(&report::report(&paid), artifacts::PLAN_LIMIT)?,
    )
    .is_ok();
    Ok(baseline::status(
        paid_status,
        accounting && !resources.interrupted,
        report_ok,
        &paid,
    ))
}

async fn baseline_transition(resources: &mut MatrixResources) -> Result<(), String> {
    let (term, int) = resources
        .signals
        .as_mut()
        .ok_or("baseline_signals_missing")?;
    #[cfg(test)]
    matrix_test_hold(
        "EVAL_TEST_BASELINE_GAP_HOLD",
        term,
        int,
        &mut resources.interrupted,
    )
    .await;
    if resources.interrupted || pending_matrix_signal(term, int).await.is_some() {
        resources.interrupted = true;
        return Err("baseline_interrupted".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn matching_spec() -> report::AgentSpec {
        report::AgentSpec {
            label: "oracle".into(),
            argv: Vec::new(),
            ..Default::default()
        }
    }

    fn matching_row() -> TrialResult {
        runner::refused_row(
            &TrialRequest {
                task: Some("signup".into()),
                agent: Some("oracle".into()),
                ..Default::default()
            },
            &"0".repeat(32),
        )
    }

    fn exit_status(code: i32) -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(code << 8)
    }

    #[cfg(target_os = "linux")]
    const MATRIX_PROBE_ENV: &str = "EVAL_TEST_MATRIX_PROBE";
    #[cfg(target_os = "linux")]
    const MATRIX_TRIAL_EXE_ENV: &str = "EVAL_TEST_MATRIX_TRIAL_EXE";
    #[cfg(target_os = "linux")]
    const MATRIX_WAIT_PENDING_ENV: &str = "EVAL_TEST_MATRIX_WAIT_PENDING";
    #[cfg(target_os = "linux")]
    const MATRIX_CLEANUP_ENTERED_ENV: &str = "EVAL_TEST_MATRIX_CLEANUP_ENTERED";
    #[cfg(target_os = "linux")]
    const MATRIX_RELEASE_ENV: &str = "EVAL_TEST_MATRIX_RELEASE";
    #[cfg(target_os = "linux")]
    const MATRIX_TRIAL_PID_ENV: &str = "EVAL_TEST_MATRIX_TRIAL_PID";
    #[cfg(target_os = "linux")]
    const MATRIX_PRE_TRAP_HOLD_ENV: &str = "EVAL_TEST_MATRIX_PRE_TRAP_HOLD_DIR";

    // The probes below identify every process they own by /proc/<pid>/stat
    // (their trial helper script reports it, and their owner guard reads it),
    // which macOS does not have, so the probe harness and its tests are Linux only.
    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_probe_entrypoint() {
        if std::env::var_os(MATRIX_PROBE_ENV).is_none() {
            return;
        }
        if std::env::var_os(runner::SWARMAIL_BIN_ENV).is_none()
            && std::env::var_os("EVAL_TEST_MATRIX_SOURCE_COMMIT_HOLD").is_none()
        {
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
        let exit = if let Some(root) = std::env::var_os("EVAL_TEST_MATRIX_ARTIFACTS") {
            runtime.block_on(matrix_with_options(
                ids,
                agents,
                trials,
                None,
                Default::default(),
                Some(root.into()),
            ))
        } else {
            runtime.block_on(matrix(ids, agents, trials, None))
        };
        assert_eq!(exit, ExitCode::from(2));
    }

    #[cfg(target_os = "linux")]
    #[derive(Clone, Debug)]
    struct MatrixIdentity {
        pid: i32,
        pgrp: i32,
        session: i32,
        start_time: u64,
        state: char,
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    fn matrix_identity_matches(want: &MatrixIdentity, now: &MatrixIdentity) -> bool {
        want.pid == now.pid
            && want.pgrp == now.pgrp
            && want.session == now.session
            && want.start_time == now.start_time
    }

    #[cfg(target_os = "linux")]
    fn matrix_identity_alive(want: &MatrixIdentity) -> bool {
        matrix_identity(want.pid).is_some_and(|now| {
            matrix_identity_matches(want, &now) && !matches!(now.state, 'Z' | 'X')
        })
    }

    #[cfg(target_os = "linux")]
    struct MatrixProbe {
        child: Option<std::process::Child>,
        owner: MatrixIdentity,
        release: PathBuf,
        pre_trap_release: PathBuf,
        owned: Vec<MatrixIdentity>,
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    #[derive(Default)]
    struct MatrixProbeControls {
        hold_before_trap: bool,
        pending_hold_at: Option<usize>,
        spawn_hold: bool,
        trials: Option<u32>,
        missing_trial_executable: bool,
        pipe_stdout: bool,
        pipe_stderr: bool,
        real_worker: bool,
        envelope: bool,
        source_hold: bool,
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
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
    --test-error-row) error_row=$2; shift 2 ;;
    --test-pass-row) pass_row=$2; shift 2 ;;
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
  printf '%s\n' "$error_row"
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
if [ "$EVAL_TEST_MATRIX_TRIAL_MODE" = cleanup-unproved ]; then
  printf '%s\n' 'swarmail-evals: harness error: trial_cleanup_unproved' >&2
  printf '%s\n' "$error_row"
  exit 2
fi
if [ "$EVAL_TEST_MATRIX_TRIAL_MODE" = complete ]; then
  printf '%s\n' "$pass_row"
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
        .env("EVAL_TEST_MATRIX_SYNTHETIC_ROW", "1")
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
        if controls.source_hold {
            let cargo = dir.join("forbidden-cargo.sh");
            std::fs::write(
                &cargo,
                format!(
                    "#!/bin/sh\ntouch '{}'/cargo-started\nexit 1\n",
                    dir.display()
                ),
            )
            .unwrap();
            std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o700)).unwrap();
            cmd.env_remove(runner::SWARMAIL_BIN_ENV)
                .env("CARGO", cargo)
                .env("EVAL_TEST_MATRIX_SOURCE_COMMIT_HOLD", dir.join("source"));
        }
        if controls.real_worker {
            let worker = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("swarmail-evals");
            cmd.env(MATRIX_TRIAL_EXE_ENV, &worker)
                .env(
                    runner::SWARMAIL_BIN_ENV,
                    runner::resolve_swarmail_bin().unwrap(),
                )
                .env_remove("EVAL_TEST_MATRIX_SYNTHETIC_ROW")
                .env("EVAL_TEST_MATRIX_ARTIFACTS", dir.join("evidence"));
            std::fs::write(&agents, serde_json::to_vec(&serde_json::json!([{ "label": "real-oracle", "argv": [worker, "agent", "oracle"] }])).unwrap()).unwrap();
        }
        if controls.envelope {
            cmd.env("EVAL_TEST_MATRIX_ENVELOPE_RELEASE", dir.join("expire"))
                .env(
                    "EVAL_TEST_MATRIX_ENVELOPE_REQUESTED",
                    dir.join("envelope-requested"),
                );
        }
        let mut script_text = std::fs::read_to_string(&script).unwrap();
        let mut setup = String::from("#!/bin/sh\n");
        for (key, value) in cmd.get_envs() {
            if key.to_string_lossy().starts_with("EVAL_TEST_MATRIX_")
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
        let owner = matrix_identity(child.id() as i32).expect("matrix probe owner identity");
        MatrixProbe {
            child: Some(child),
            owner,
            release,
            pre_trap_release,
            owned: Vec::new(),
        }
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_completion_fence_is_observed_and_its_negative_control_exits_early() {
        let _trial_scope = crate::test_support::trial_scope();
        for signal in [libc::SIGTERM, libc::SIGINT] {
            matrix_wait_fence_case(false, signal);
            matrix_wait_fence_case(true, signal);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_probe_readiness_unwind_releases_and_reaps_owned_processes() {
        let _trial_scope = crate::test_support::trial_scope();
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

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_owner_test_controls_cover_setup_admission_commit_and_final_row() {
        let _trial_scope = crate::test_support::trial_scope();
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

    #[cfg(target_os = "linux")]
    #[test]
    fn pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children() {
        let _trial_scope = crate::test_support::trial_scope();
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
        eventually("queued TERM acknowledgement", || {
            pending.join("signal-term").is_file()
        });
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
        eventually("queued INT acknowledgement", || {
            pending.join("signal-int").is_file()
        });
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
        eventually("queued TERM acknowledgement", || {
            pending.join("signal-term").is_file()
        });
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
        let trial_ready = issued.path().join("trial.pid");
        eventually("the issued trial's trap-ready identity", || {
            trial_ready.is_file()
        });
        probe.record_ready_identity(&trial_ready);
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

    #[cfg(target_os = "linux")]
    #[test]
    fn mixed_signals_during_matrix_cleanup_do_not_bypass_the_retained_wait() {
        let _trial_scope = crate::test_support::trial_scope();
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

    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_spawn_failure_keeps_all_rows_and_unknown_agent_statuses() {
        let _trial_scope = crate::test_support::trial_scope();
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

    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_stdout_backpressure_is_bounded_and_still_observes_signals() {
        let _trial_scope = crate::test_support::trial_scope();
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
        // An acknowledgement can be pending on the first, writable row.
        // Keep interrupting until the writer wait itself acknowledges one;
        // a marker from an earlier row is not synchronization with backpressure.
        eventually("the blocked writer acknowledges interruption", || {
            if std::fs::read_to_string(dir.path().join("signals.log"))
                .is_ok_and(|log| log.lines().any(|event| event == "write-int"))
            {
                true
            } else {
                unsafe { libc::kill(probe.pid(), libc::SIGINT) };
                false
            }
        });
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

    #[cfg(target_os = "linux")]
    #[test]
    fn matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test() {
        let _trial_scope = crate::test_support::trial_scope();
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

    #[cfg(target_os = "linux")]
    #[test]
    fn source_observation_cancellation_stays_latched_before_cargo_admission() {
        let _trial_scope = crate::test_support::trial_scope();
        for signal in [libc::SIGTERM, libc::SIGINT] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = matrix_probe_config(
                dir.path(),
                false,
                false,
                None,
                "complete",
                "signup,otp",
                MatrixProbeControls {
                    source_hold: true,
                    ..Default::default()
                },
            );
            let hold = dir.path().join("source");
            eventually("source observation before cleanup/admission", || {
                hold.join("entered").exists()
            });
            unsafe {
                libc::kill(probe.pid(), signal);
            }
            eventually("source cancellation acknowledged", || {
                hold.join(if signal == libc::SIGTERM {
                    "signal-term"
                } else {
                    "signal-int"
                })
                .exists()
            });
            std::fs::write(hold.join("release"), b"release observed source cleanup").unwrap();
            assert!(probe.wait().success());
            assert!(!dir.path().join("cargo-started").exists());
            assert!(!dir.path().join("trial.pid").exists());
            let rows = matrix_probe_rows(dir.path());
            assert_eq!(rows.len(), 2);
            assert!(
                rows.iter()
                    .all(|r| r.outcome == Outcome::HarnessError && r.wall_time_ms == 0)
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn retained_child_conflicts_with_cancellation_winning_commit_without_mutation() {
        let _trial_scope = crate::test_support::trial_scope();
        for signal in [libc::SIGTERM, libc::SIGINT] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = matrix_probe_config(
                dir.path(),
                false,
                false,
                Some("commit"),
                "complete",
                "signup,otp",
                MatrixProbeControls {
                    real_worker: true,
                    ..Default::default()
                },
            );
            let hold = dir.path().join("commit");
            eventually("retained child before commitment", || {
                hold.join("entered").is_file()
            });
            let root = std::fs::read_dir(dir.path().join("evidence"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let plan: artifacts::MatrixPlan =
                serde_json::from_slice(&std::fs::read(root.join("matrix-plan.json")).unwrap())
                    .unwrap();
            let leaf = root.join("trials").join(&plan.cells[0].trial_id);
            let original = artifacts::trial(&leaf).unwrap();
            assert_eq!(original.row.outcome, Outcome::Pass);
            let bytes: Vec<_> = [
                "result.json",
                "state.json",
                "metadata.json",
                "diagnostics.log",
            ]
            .into_iter()
            .map(|name| (name, std::fs::read(leaf.join(name)).unwrap()))
            .collect();
            unsafe {
                libc::kill(probe.pid(), signal);
            }
            eventually("signal wins before row commitment", || {
                hold.join(if signal == libc::SIGTERM {
                    "signal-term"
                } else {
                    "signal-int"
                })
                .exists()
            });
            probe.release();
            assert!(probe.wait().success(), "probe asserts the matrix exit is 2");
            let rows = matrix_probe_rows(dir.path());
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].trial_id, original.row.trial_id);
            assert_eq!(rows[0].outcome, Outcome::HarnessError);
            assert_eq!(rows[0].agent_exit_status, Some(0));
            assert_eq!(rows[1].wall_time_ms, 0);
            assert_eq!(rows[1].outcome, Outcome::HarnessError);
            for (name, before) in bytes {
                assert_eq!(std::fs::read(leaf.join(name)).unwrap(), before);
            }
            assert!(!root.join("matrix.json").exists());
            assert!(artifacts::matrix(&root).is_err());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn envelope_stop_observes_the_same_pending_cleanup_and_bypass_negative_control() {
        let _trial_scope = crate::test_support::trial_scope();
        for bypass in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut probe = matrix_probe_config(
                dir.path(),
                bypass,
                false,
                None,
                "held",
                "signup,otp",
                MatrixProbeControls {
                    envelope: true,
                    ..Default::default()
                },
            );
            let ready = dir.path().join("trial.pid");
            eventually("envelope trial ready", || ready.is_file());
            probe.record_ready_identity(&ready);
            let active = probe.owned.last().unwrap().clone();
            std::fs::write(dir.path().join("expire"), b"expire after readiness").unwrap();
            eventually("envelope requested cancellation", || {
                dir.path().join("envelope-requested").exists()
            });
            eventually("cleanup entered", || {
                dir.path().join("cleanup-entered").exists()
            });
            if bypass {
                assert!(probe.wait().success());
                assert!(
                    matrix_identity_alive(&active),
                    "negative control returns with owned work alive"
                );
            } else {
                eventually("envelope child wait acknowledged Pending", || {
                    dir.path().join("wait-pending").exists()
                });
                assert!(probe.try_wait().is_none());
                assert!(matrix_identity_alive(&active));
                unsafe {
                    libc::kill(probe.pid(), libc::SIGINT);
                }
                eventually("signal after envelope is a repeat", || {
                    std::fs::read_to_string(dir.path().join("signals.log"))
                        .is_ok_and(|s| s.contains("int-repeat"))
                });
                assert!(probe.try_wait().is_none());
            }
            probe.release();
            if !bypass {
                assert!(probe.wait().success());
            }
            eventually("owned envelope child stopped", || {
                !matrix_identity_alive(&active)
            });
            let rows = matrix_probe_rows(dir.path());
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].outcome, Outcome::HarnessError);
            assert_eq!(rows[1].wall_time_ms, 0);
        }
    }

    #[test]
    fn matching_trial_row_requires_one_bounded_valid_matching_row_and_exit_pair() {
        let _trial_scope = crate::test_support::trial_scope();
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
        bad_schema["schema_version"] = serde_json::json!(1);
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

    #[tokio::test]
    async fn worker_categories_are_bounded_and_finite() {
        use std::io::Cursor;
        let prefix = "swarmail-evals: harness error: ";
        for bytes in [
            b"synthetic-private-canary\n".to_vec(),
            format!("{prefix}unknown\n").into_bytes(),
            format!("{prefix}concurrent_trials_refused").into_bytes(),
            [
                vec![b'x'; 256],
                format!("{prefix}concurrent_trials_refused\n").into_bytes(),
            ]
            .concat(),
        ] {
            assert_eq!(
                capture_worker_category(Cursor::new(bytes)).await.unwrap(),
                None
            );
        }
        for category in [
            "concurrent_trials_refused",
            "trial_admission_unavailable",
            "trial_cleanup_unproved",
        ] {
            let input = format!("{prefix}{category}\n");
            assert_eq!(
                capture_worker_category(Cursor::new(input)).await.unwrap(),
                Some(category)
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix() {
        let _trial_scope = crate::test_support::trial_scope();
        let dir = tempfile::tempdir().unwrap();
        let mut probe = matrix_probe_config(
            dir.path(),
            false,
            false,
            None,
            "cleanup-unproved",
            "signup,otp",
            MatrixProbeControls {
                pipe_stderr: true,
                ..MatrixProbeControls::default()
            },
        );
        assert!(probe.wait().success());
        let rows = matrix_probe_rows(dir.path());
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|row| row.outcome == Outcome::HarnessError && row.concurrency == 1)
        );
        assert_eq!(rows[1].wall_time_ms, 0);
        let launches = std::fs::read_to_string(dir.path().join("launches.log")).unwrap();
        assert_eq!(launches.lines().count(), 1);
        let mut stderr = String::new();
        std::io::Read::read_to_string(
            &mut probe.child.as_mut().unwrap().stderr.take().unwrap(),
            &mut stderr,
        )
        .unwrap();
        assert!(stderr.contains("trial_cleanup_unproved"));
    }

    #[test]
    fn capture_stdout_drains_beyond_its_retention_limit() {
        let _trial_scope = crate::test_support::trial_scope();
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

#[cfg(test)]
mod privacy_tests {
    async fn bounded_stdout<R: tokio::io::AsyncRead + Unpin>(reader: R) -> Option<Vec<u8>> {
        let captured = super::capture_stdout(reader).await.ok()?;
        (!captured.overflow).then_some(captured.bytes)
    }

    async fn run_child(
        exe: &std::path::Path,
        bin: &std::path::Path,
        id: &str,
        spec: &swarmail_evals::report::AgentSpec,
        argv: &[std::ffi::OsString],
        deadline: Option<u64>,
    ) -> Option<swarmail_evals::runner::TrialResult> {
        let (mut term, mut int) = super::install_matrix_signals().unwrap();
        super::run_child(exe, bin, id, spec, argv, deadline, &mut term, &mut int)
            .await
            .row
    }
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use swarmail_evals::report::AgentSpec;
    use swarmail_evals::runner::{AgentExit, TrialResult};
    use swarmail_evals::state::FailureClass;
    use swarmail_evals::tasks::Outcome;

    fn worker_program(directory: &Path, name: &str, output: &str) -> std::path::PathBuf {
        let program = directory.join(name);
        let quoted = output.replace('\'', "'\\''");
        std::fs::write(
            &program,
            format!("#!/bin/sh\nprintf '%s' '{quoted}'\nexit 1\n"),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        program
    }

    #[tokio::test]
    async fn matrix_worker_requires_one_matching_schema_row_and_exit() {
        let _trial_scope = crate::test_support::trial_scope();
        let owner = tempfile::tempdir().unwrap();
        let expected = TrialResult {
            schema_version: 2,
            concurrency: 1,
            trial_id: "0".repeat(32),
            task: Some("signup".into()),
            agent: Some("oracle".into()),
            outcome: Outcome::Fail,
            failure_class: Some(FailureClass::GaveUp),
            wall_time_ms: 3,
            agent_exit_status: Some(3),
            agent_exits: vec![AgentExit {
                identity: "synthetic-identity".into(),
                code: Some(3),
                signal: None,
            }],
            confinement: "unconfined".into(),
            provenance: swarmail_evals::runner::make_provenance(
                &swarmail_evals::runner::TrialRequest {
                    task: Some("signup".into()),
                    agent: Some("oracle".into()),
                    ..Default::default()
                },
                None,
                None,
            ),
        };
        let spec = AgentSpec {
            label: "oracle".into(),
            argv: Vec::new(),
            ..Default::default()
        };
        let valid = worker_program(
            owner.path(),
            "valid-worker-row",
            &serde_json::to_string(&expected).unwrap(),
        );
        assert_eq!(
            run_child(&valid, Path::new("/bin/true"), "signup", &spec, &[], None,).await,
            Some(expected.clone())
        );

        let valid_json = serde_json::to_value(&expected).unwrap();
        let mut invalid = Vec::new();
        let mut missing = valid_json.clone();
        missing.as_object_mut().unwrap().remove("concurrency");
        invalid.push(missing);
        for value in [
            serde_json::json!(0),
            serde_json::json!(2),
            serde_json::json!("1"),
            serde_json::Value::Null,
        ] {
            let mut bad = valid_json.clone();
            bad["concurrency"] = value;
            invalid.push(bad);
        }
        let mut old = valid_json;
        old["schema_version"] = serde_json::json!(1);
        invalid.push(old);
        for bad in invalid {
            let program = worker_program(owner.path(), "invalid-concurrency", &bad.to_string());
            assert!(
                run_child(&program, Path::new("/bin/true"), "signup", &spec, &[], None)
                    .await
                    .is_none()
            );
        }

        let extra = worker_program(
            owner.path(),
            "extra-worker-output",
            &format!(
                "{}\nsynthetic-worker-canary",
                serde_json::to_string(&expected).unwrap()
            ),
        );
        assert!(
            run_child(&extra, Path::new("/bin/true"), "signup", &spec, &[], None,)
                .await
                .is_none()
        );

        let wrong_task = worker_program(
            owner.path(),
            "wrong-worker-task",
            &serde_json::to_string(&TrialResult {
                task: Some("otp".into()),
                ..expected.clone()
            })
            .unwrap(),
        );
        assert!(
            run_child(
                &wrong_task,
                Path::new("/bin/true"),
                "signup",
                &spec,
                &[],
                None,
            )
            .await
            .is_none()
        );

        let wrong_agent = worker_program(
            owner.path(),
            "wrong-worker-agent",
            &serde_json::to_string(&TrialResult {
                agent: Some("different-agent".into()),
                ..expected.clone()
            })
            .unwrap(),
        );
        assert!(
            run_child(
                &wrong_agent,
                Path::new("/bin/true"),
                "signup",
                &spec,
                &[],
                None,
            )
            .await
            .is_none()
        );

        let exit_mismatch = worker_program(
            owner.path(),
            "worker-exit-mismatch",
            &serde_json::to_string(&TrialResult {
                outcome: Outcome::Pass,
                failure_class: None,
                agent_exit_status: None,
                agent_exits: Vec::new(),
                ..expected.clone()
            })
            .unwrap(),
        );
        assert!(
            run_child(
                &exit_mismatch,
                Path::new("/bin/true"),
                "signup",
                &spec,
                &[],
                None,
            )
            .await
            .is_none()
        );

        let carriage_return = worker_program(
            owner.path(),
            "worker-carriage-return",
            &format!("{}\r", serde_json::to_string(&expected).unwrap()),
        );
        assert!(
            run_child(
                &carriage_return,
                Path::new("/bin/true"),
                "signup",
                &spec,
                &[],
                None,
            )
            .await
            .is_none()
        );

        let malformed = worker_program(owner.path(), "malformed-worker-row", "{not-json");
        assert!(
            run_child(
                &malformed,
                Path::new("/bin/true"),
                "signup",
                &spec,
                &[],
                None,
            )
            .await
            .is_none()
        );
    }

    #[tokio::test]
    async fn matrix_row_reader_caps_storage_and_drains_overflow() {
        let _trial_scope = crate::test_support::trial_scope();
        let accepted = vec![b'x'; super::MATRIX_OUTPUT_LIMIT];
        assert_eq!(
            bounded_stdout(Cursor::new(accepted.clone())).await,
            Some(accepted)
        );
        let oversized = vec![b'y'; super::MATRIX_OUTPUT_LIMIT + 1];
        assert_eq!(bounded_stdout(Cursor::new(oversized)).await, None);
    }
}

#[cfg(test)]
#[path = "../tests/common/coordination.rs"]
mod test_support;

#[cfg(test)]
mod baseline_tests {
    use super::*;
    struct Probe(std::process::Child);
    impl Drop for Probe {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    #[tokio::test]
    async fn transition_retains_signals_between_matrices_and_never_admits_paid_work() {
        if std::env::var_os("EVAL_TEST_BASELINE_TRANSITION_CHILD").is_some() {
            let mut resources = MatrixResources {
                signals: Some(install_matrix_signals().unwrap()),
                ..Default::default()
            };
            assert!(baseline_transition(&mut resources).await.is_err());
            assert!(resources.interrupted);
            assert!(baseline_transition(&mut resources).await.is_err());
            return;
        }
        // The parent holds the scope; the re-executed child above must not
        // take it again, or it would wait on its own parent.
        let _trial_scope = crate::test_support::trial_scope();
        for signal in [libc::SIGTERM, libc::SIGINT] {
            let dir = tempfile::tempdir().unwrap();
            let hold = dir.path().join("gap");
            let mut child = Probe(std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "baseline_tests::transition_retains_signals_between_matrices_and_never_admits_paid_work", "--nocapture"])
                .env("EVAL_TEST_BASELINE_TRANSITION_CHILD", "1").env("EVAL_TEST_BASELINE_GAP_HOLD", &hold)
                .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
            let deadline = Instant::now() + Duration::from_secs(10);
            while !hold.join("entered").exists() {
                assert!(Instant::now() < deadline, "child did not acknowledge gap");
                assert!(child.0.try_wait().unwrap().is_none());
                std::thread::yield_now();
            }
            // The child has acknowledged installed signal ownership and the
            // actual between-matrices boundary. No timing guess is involved.
            assert_eq!(unsafe { libc::kill(child.0.id() as i32, signal) }, 0);
            // Release only once the child has observed the signal: delivery
            // is asynchronous, and a release seen first under load would let
            // the boundary's non-blocking check miss it.
            let seen = hold.join(if signal == libc::SIGTERM {
                "signal-term"
            } else {
                "signal-int"
            });
            while !seen.is_file() {
                assert!(
                    Instant::now() < deadline,
                    "child did not observe the signal"
                );
                assert!(child.0.try_wait().unwrap().is_none());
                std::thread::yield_now();
            }
            std::fs::write(hold.join("release"), b"release").unwrap();
            assert!(child.0.wait().unwrap().success());
            assert!(!dir.path().join("paid.jsonl").exists());
        }
    }
}

#[cfg(test)]
mod isolation_entry_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use swarmail_evals::runner::TrialResult;

    fn parse(args: &[&str]) -> Cmd {
        let mut all = vec!["swarmail-evals", "trial"];
        all.extend_from_slice(args);
        Cli::try_parse_from(all).expect("arguments parse").command
    }

    struct Ran {
        result: Result<Prepared, u8>,
        stdout: String,
        stderr: String,
    }

    fn run(args: &[&str], bin: Result<Option<PathBuf>, String>) -> Ran {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let result = prepare_trial(&parse(args), bin, &mut out, &mut err);
        Ran {
            result,
            stdout: String::from_utf8(out).unwrap(),
            stderr: String::from_utf8(err).unwrap(),
        }
    }

    fn row(stdout: &str) -> TrialResult {
        let lines: Vec<_> = stdout.lines().collect();
        assert_eq!(lines.len(), 1, "exactly one row: {stdout:?}");
        let row: TrialResult = serde_json::from_str(lines[0]).unwrap();
        row.validate().unwrap();
        row
    }

    fn a_binary() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = std::fs::canonicalize(dir.path()).unwrap().join("swarmail");
        std::fs::write(&path, b"not run").unwrap();
        (dir, path)
    }

    #[test]
    fn each_precondition_refuses_by_name_before_any_boundary_work() {
        let (dir, bin) = a_binary();
        let some = || Ok(Some(bin.clone()));
        let artifacts = dir.path().join("artifacts");
        type Case<'a> = (Vec<&'a str>, Result<Option<PathBuf>, String>, &'a str);
        let cases: Vec<Case> = vec![
            (
                vec!["--isolated"],
                Ok(None),
                "isolation_requires_swarmail_bin",
            ),
            (
                vec!["--isolated"],
                Err("program_missing".into()),
                "isolation_source_invalid",
            ),
            (
                vec!["--isolated", "--artifacts-dir", artifacts.to_str().unwrap()],
                some(),
                "isolation_artifacts_unsupported",
            ),
            (
                vec![
                    "--isolated",
                    "--agent",
                    "claude",
                    "--wrapper-profile",
                    "claude_2_1_310",
                ],
                some(),
                "isolation_wrapper_unsupported",
            ),
            (
                vec!["--isolated", "--isolated-expose", "relative/file"],
                some(),
                "isolation_expose_invalid",
            ),
            (
                vec!["--isolated", "--isolated-expose", "/definitely/not/here"],
                some(),
                "isolation_expose_invalid",
            ),
            (
                vec!["--isolated", "--", "/definitely/not/an/agent"],
                some(),
                "isolation_source_invalid",
            ),
            // The public refusal keeps its meaning and comes first, even
            // where a later precondition also fails.
            (
                vec!["--isolated", "--concurrency", "2"],
                Ok(None),
                "concurrency_unsupported",
            ),
        ];
        for (args, bin, reason) in cases {
            let ran = run(&args, bin);
            assert_eq!(ran.result.as_ref().err(), Some(&2), "{args:?}");
            assert!(
                ran.stderr.contains(&format!("harness error: {reason}\n")),
                "{args:?}: {}",
                ran.stderr
            );
            assert_eq!(row(&ran.stdout).outcome, Outcome::HarnessError, "{args:?}");
        }
        assert!(!artifacts.exists());
    }

    #[test]
    fn a_trial_without_the_flag_runs_in_process_and_never_touches_the_boundary() {
        let ran = run(&["--task", "signup", "--agent", "null"], Ok(None));
        assert!(matches!(ran.result, Ok(Prepared::Run(_))));
        assert!(ran.stdout.is_empty() && ran.stderr.is_empty());
        // The expose flag means nothing without the boundary.
        assert!(
            Cli::try_parse_from(["swarmail-evals", "trial", "--isolated-expose", "/x"]).is_err()
        );
    }

    #[test]
    fn a_valid_isolated_request_is_handed_to_the_controller_with_its_inner_argv() {
        let (_dir, bin) = a_binary();
        let ran = run(
            &["--task", "signup", "--agent", "oracle", "--isolated"],
            Ok(Some(bin.clone())),
        );
        assert!(ran.stdout.is_empty() && ran.stderr.is_empty());
        let Ok(Prepared::Isolate {
            req,
            request,
            inner,
        }) = ran.result
        else {
            panic!("expected an isolated launch");
        };
        assert_eq!(req.concurrency, Some(1));
        assert_eq!(request.swarmail_bin, bin);
        assert_eq!(request.runner, std::env::current_exe().unwrap());
        assert_eq!(inner[0], request.runner.clone().into_os_string());
        assert_eq!(inner[1], "trial");
        assert!(
            !inner
                .iter()
                .any(|a| a.to_string_lossy().contains("isolated"))
        );
    }

    fn all_fields() -> Vec<&'static str> {
        vec![
            "--task",
            "signup",
            "--agent",
            "probe",
            "--deadline-ms",
            "1234",
            "--concurrency",
            "1",
            "--artifacts-dir",
            "/a/b",
            "--seed",
            "seed-1",
            "--fixture-time",
            "2026-01-02T03:04:05Z",
            "--agent-model",
            "model-x",
            "--agent-config-id",
            "config-y",
            "--public-argv",
            "--wrapper-profile",
            "claude_2_1_310",
            "--worker-context",
            "context-z",
            "--isolated",
            "--isolated-expose",
            "/x/one",
            "--isolated-expose",
            "/x/two",
            "--",
            "agent",
            "--isolated",
            "--ro-bind",
            "/",
            "/",
            "--",
            "-x",
        ]
    }

    fn request_for(agent: Option<&str>) -> isolation::Request {
        isolation::Request {
            swarmail_bin: PathBuf::from("/b/swarmail"),
            runner: PathBuf::from("/b/swarmail-evals"),
            agent: agent.map(PathBuf::from),
            expose: Vec::new(),
        }
    }

    #[test]
    fn the_inner_request_round_trips_through_clap_without_the_boundary_flags() {
        let original = parse(&all_fields());
        let inner = inner_args(&original, &request_for(Some("/real/agent")));
        assert_eq!(inner[0], "/b/swarmail-evals");
        let mut again = vec![OsString::from("swarmail-evals")];
        again.extend(inner[1..].iter().cloned());
        let rendered = Cli::try_parse_from(again)
            .expect("the inner argv parses")
            .command;

        // The same request minus `isolated`/`isolated_expose`, with the bare
        // agent program replaced by the absolute one the controller found.
        let mut want = parse(&all_fields());
        let Cmd::Trial {
            isolated,
            isolated_expose,
            argv,
            ..
        } = &mut want
        else {
            unreachable!()
        };
        *isolated = false;
        isolated_expose.clear();
        argv[0] = OsString::from("/real/agent");
        assert_eq!(format!("{rendered:?}"), format!("{want:?}"));

        // The agent's own `--isolated`, `--ro-bind` and `--` survive literally.
        let Cmd::Trial { argv, .. } = &rendered else {
            unreachable!()
        };
        assert_eq!(
            argv.iter().map(|a| a.to_str().unwrap()).collect::<Vec<_>>(),
            [
                "/real/agent",
                "--isolated",
                "--ro-bind",
                "/",
                "/",
                "--",
                "-x"
            ]
        );
    }

    #[test]
    fn the_resolved_agent_replaces_only_a_relative_program() {
        let render = |args: &[&str], agent: Option<&str>| {
            let mut all = vec!["--isolated", "--"];
            all.extend_from_slice(args);
            let inner = inner_args(&parse(&all), &request_for(agent));
            inner
                .iter()
                .skip_while(|a| *a != "--")
                .skip(1)
                .map(|a| a.to_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            render(&["agent", "--flag"], Some("/real/agent")),
            ["/real/agent", "--flag"]
        );
        assert_eq!(
            render(&["/given/agent"], Some("/real/agent")),
            ["/given/agent"]
        );
        assert_eq!(render(&["agent"], None), ["agent"]);
        // No agent argv, no separator.
        let inner = inner_args(
            &parse(&["--isolated", "--agent", "oracle"]),
            &request_for(None),
        );
        assert!(!inner.iter().any(|a| a == "--"));
        // A value that starts with a dash stays a value.
        let inner = inner_args(&parse(&["--isolated", "--task=-odd"]), &request_for(None));
        assert!(inner.iter().any(|a| a == "--task=-odd"));
    }

    // ---- the controller's refusals, through the parameter seam (AC-S2) --------

    fn script(dir: &std::path::Path, version: &str, probe: &str, real: &str) -> PathBuf {
        let path = dir.join("bwrap");
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\ncase \"$1\" in --version) echo '{version}'; exit 0;; esac\nfor a in \"$@\"; do last=\"$a\"; done\nif [ \"$last\" = /usr/bin/true ]; then\n{probe}\nfi\n{real}\n"
            ),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        for _ in 0..500 {
            match std::process::Command::new(&path).arg("--version").output() {
                Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => break,
            }
        }
        path
    }

    /// Drive a whole isolated request against a stand-in bwrap.
    fn through_the_controller(bwrap: &std::path::Path, sentinel: &std::path::Path) -> Ran {
        let (_dir, bin) = a_binary();
        let program = sentinel.parent().unwrap().join("agent");
        std::fs::write(
            &program,
            format!("#!/bin/sh\ntouch '{}'\n", sentinel.display()),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let args = [
            "--task",
            "signup",
            "--agent",
            "probe",
            "--isolated",
            "--",
            program.to_str().unwrap(),
        ];
        let prepared = prepare_trial(&parse(&args), Ok(Some(bin)), &mut out, &mut err)
            .unwrap_or_else(|code| {
                panic!("refused with {code}: {}", String::from_utf8_lossy(&err))
            });
        let Prepared::Isolate {
            req,
            request,
            inner,
        } = prepared
        else {
            panic!("not isolated")
        };
        let code = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_isolated(
                &req, &request, &inner, bwrap, &mut out, &mut err,
            ));
        Ran {
            result: Err(code),
            stdout: String::from_utf8(out).unwrap(),
            stderr: String::from_utf8(err).unwrap(),
        }
    }

    #[test]
    fn every_refused_boundary_is_a_named_harness_error_and_no_agent_runs() {
        let dir = tempfile::tempdir().unwrap();
        let agent_ran = dir.path().join("agent-ran");
        let touch = format!("touch '{}'; exit 0", agent_ran.display());
        let bad_probe = "echo 'bwrap: Creating new namespace failed: ENOSPC' >&2; exit 1";
        let stand_in = |name: &str, version: &str, probe: &str| {
            let home = dir.path().join(name);
            std::fs::create_dir(&home).unwrap();
            script(&home, version, probe, &touch)
        };
        let cases: Vec<(PathBuf, &str)> = vec![
            (
                PathBuf::from("/definitely/not/bwrap"),
                "boundary_unavailable:bwrap_absent",
            ),
            (
                stand_in("old", "bubblewrap 0.9.0", &touch),
                "boundary_unavailable:bwrap_version",
            ),
            (
                stand_in("refusing", "bubblewrap 0.11.1", bad_probe),
                "boundary_unavailable:bwrap_probe",
            ),
        ];
        for (bwrap, reason) in cases {
            let ran = through_the_controller(&bwrap, &agent_ran);
            assert_eq!(ran.result.as_ref().err(), Some(&2), "{reason}");
            assert!(
                ran.stderr.contains(&format!("harness error: {reason}\n")),
                "{reason}: {}",
                ran.stderr
            );
            let row = row(&ran.stdout);
            assert_eq!(row.outcome, Outcome::HarnessError);
            assert_eq!(row.task.as_deref(), Some("signup"));
            assert_eq!(row.agent.as_deref(), Some("probe"));
            assert_eq!(row.concurrency, 1);
            assert!(row.agent_exits.is_empty());
            assert!(!agent_ran.exists(), "{reason}: an agent was launched");
        }
    }

    #[test]
    fn how_a_launch_ended_is_one_exit_code_and_at_most_one_named_row() {
        let req = TrialRequest {
            task: Some("signup".into()),
            agent: Some("probe".into()),
            ..Default::default()
        };
        let conclude_with = |launched| {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let code = conclude(&req, launched, &mut out, &mut err);
            (
                code,
                String::from_utf8(out).unwrap(),
                String::from_utf8(err).unwrap(),
            )
        };
        // A row the runner printed is mirrored; the controller adds nothing.
        for code in [0u8, 1, 2] {
            let (got, out, err) = conclude_with(Ok(isolation::Launched::Exited(code)));
            assert_eq!((got, out.as_str(), err.as_str()), (code, "", ""));
        }
        for (launched, name) in [
            (isolation::Launched::NoRow, "isolated_trial_no_row"),
            (
                isolation::Launched::Interrupted,
                "isolated_trial_interrupted",
            ),
        ] {
            let (code, out, err) = conclude_with(Ok(launched));
            assert_eq!(code, 2);
            assert_eq!(err, format!("swarmail-evals: harness error: {name}\n"));
            assert_eq!(row(&out).outcome, Outcome::HarnessError);
        }
        let (code, out, err) =
            conclude_with(Err(isolation::Refusal::Named("concurrent_trials_refused")));
        assert_eq!(code, 2);
        assert!(err.contains("concurrent_trials_refused"));
        assert_eq!(row(&out).outcome, Outcome::HarnessError);
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn macos_refuses_by_name_through_the_whole_entry() {
        let dir = tempfile::tempdir().unwrap();
        let sentinel = dir.path().join("agent-ran");
        let ran = through_the_controller(std::path::Path::new("/usr/bin/bwrap"), &sentinel);
        assert_eq!(ran.result.as_ref().err(), Some(&2));
        assert!(
            ran.stderr
                .contains("harness error: boundary_unavailable:platform\n"),
            "{}",
            ran.stderr
        );
        assert_eq!(row(&ran.stdout).outcome, Outcome::HarnessError);
        assert!(!sentinel.exists());
    }
}
