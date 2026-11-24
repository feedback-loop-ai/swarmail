//! The MCP client matrix's scripted route (issue #392, proposed decision
//! 0138): `swarmail-evals client-matrix run` drives one installed MCP
//! Inspector CLI through the checklist of `docs/agents/client-matrix.md`
//! against one fresh, memory-only Swarmail, and writes a receipt draft the
//! shared checker accepts. `client-matrix validate` checks receipt files.
//!
//! The runner downloads nothing: the client is the literal argv the
//! operator gives. It starts the given binary on ephemeral loopback ports,
//! delivers the checklist's fixture over SMTP once and keys every step on
//! the ID of that final DATA reply. Each of the five CLI launches gets its
//! own observer in front of Swarmail's `/mcp`, sealed before the next
//! launch, and is judged by [`evidence`]. A draft is evidence about file
//! shape and one observed set only; it is never a compatibility verdict.

pub mod evidence;
pub mod receipt;

#[cfg(test)]
mod evidence_tests;
#[cfg(test)]
mod receipt_tests;

use clap::{Parser, Subcommand};
use evidence::{Call, Launch};
use receipt::{Client, Mode, Outcome, Receipt, RecordedBy, Status, Steps, Transport};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

/// Exit status of an all-pass draft, a written red draft and an error.
pub const EXIT_GREEN: u8 = 0;
pub const EXIT_RED: u8 = 1;
pub const EXIT_ERROR: u8 = 2;

/// The longest one client launch may run, within the run's budget.
const LAUNCH_BOUND: Duration = Duration::from_secs(60);
/// The set's grace past the budget, for the last launch's reaping.
const SET_GRACE: Duration = Duration::from_secs(15);

#[derive(Parser)]
#[command(name = "client-matrix", no_binary_name = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Drive one installed MCP Inspector CLI through the checklist and write
    /// a receipt draft. The client's argv follows `--`.
    Run(RunArgs),
    /// Check receipt files (or directories of them) with the shared checker.
    Validate {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
}

#[derive(clap::Args, Clone)]
pub struct RunArgs {
    /// The Swarmail executable built from clean main, by absolute path.
    #[arg(long)]
    pub binary: PathBuf,
    /// The full main commit that binary was built from (40 hex digits).
    #[arg(long)]
    pub commit: String,
    /// The installed Inspector package's exact version.
    #[arg(long)]
    pub client_version: String,
    /// The operating system and its version.
    #[arg(long)]
    pub os: String,
    /// `operator` or `dispatcher`.
    #[arg(long)]
    pub recorded_by: String,
    /// Where the draft is installed (created if absent; never overwritten).
    #[arg(long)]
    pub output_dir: PathBuf,
    /// The whole run's budget in seconds.
    #[arg(long, default_value_t = 300)]
    pub budget_secs: u64,
    /// The installed client's argv; the runner appends `--cli <url> ...`.
    #[arg(last = true, required = true)]
    pub client: Vec<String>,
}

/// The `client-matrix` subcommand: parse, run, report, exit status.
pub async fn command(args: &[String]) -> u8 {
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return EXIT_ERROR;
        }
    };
    match cli.command {
        Command::Validate { paths } => validate_paths(&paths),
        Command::Run(args) => match run(&args).await {
            Ok((path, receipt)) => {
                let red: Vec<String> = receipt
                    .steps
                    .in_order()
                    .iter()
                    .filter(|(_, o)| o.status == Status::Fail)
                    .map(|(step, o)| format!("{step}: {}", o.reason.as_deref().unwrap_or("")))
                    .collect();
                println!("client-matrix: draft {}", path.display());
                if red.is_empty() {
                    println!("client-matrix: all six steps passed (no compatibility verdict)");
                    EXIT_GREEN
                } else {
                    for line in &red {
                        println!("client-matrix: failed {line}");
                    }
                    EXIT_RED
                }
            }
            Err(error) => {
                eprintln!("client-matrix: error: {error}");
                EXIT_ERROR
            }
        },
    }
}

fn validate_paths(paths: &[PathBuf]) -> u8 {
    let mut code = EXIT_GREEN;
    for path in paths {
        let verdict = if path.is_dir() {
            receipt::scan(path)
        } else {
            receipt::validate_file(path).map(|_| "well formed; file shape only".into())
        };
        match verdict {
            Ok(status) => println!("{}: {status}", path.display()),
            Err(error) => {
                eprintln!("{error}");
                code = EXIT_ERROR;
            }
        }
    }
    code
}

/// The checklist's fixture message with CRLF line ends: the handbook's
/// fenced block, which the root docs target checks equals
/// [`receipt::FIXTURE`].
pub fn fixture_message() -> Vec<u8> {
    receipt::FIXTURE
        .lines()
        .flat_map(|line| [line.as_bytes(), b"\r\n"].concat())
        .collect()
}

fn check_args(args: &RunArgs) -> Result<RecordedBy, String> {
    if !args.binary.is_absolute() {
        return Err("--binary must be an absolute path".into());
    }
    if !receipt::lower_hex(&args.commit, 40) {
        return Err("--commit must be the full commit, 40 lowercase hex digits".into());
    }
    for (flag, value) in [
        ("--client-version", &args.client_version),
        ("--os", &args.os),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{flag} is blank"));
        }
    }
    match args.recorded_by.as_str() {
        "operator" => Ok(RecordedBy::Operator),
        "dispatcher" => Ok(RecordedBy::Dispatcher),
        _ => Err("--recorded-by must be operator or dispatcher".into()),
    }
}

/// Run the checklist once and install its validated draft. A setup failure
/// before an attributable observation is an error and leaves no draft.
pub async fn run(args: &RunArgs) -> Result<(PathBuf, Receipt), String> {
    let recorded_by = check_args(args)?;
    let deadline = Instant::now() + Duration::from_secs(args.budget_secs);
    let date = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let name = format!("{date}-{}.json", Client::McpInspector.slug());
    let root = crate::artifacts::resolve_root(&args.output_dir)?;
    crate::artifacts::ensure_root(&root)?;
    if std::fs::symlink_metadata(root.join(&name)).is_ok() {
        return Err(format!(
            "{name} already exists in the output directory; it is kept"
        ));
    }
    let sha = crate::identity::file_hash(&args.binary)?;
    let scratch = Arc::new(crate::runner::private_tempdir("swarmail-matrix-")?);
    let groups = crate::runner::Groups::default();
    // Each launch is bounded by what is left of the budget; the set as a
    // whole gets a grace for the last launch's reaping. An exhausted budget
    // or an interruption ends the set with an error and no draft.
    let observed = tokio::select! {
        biased;
        _ = tokio::signal::ctrl_c() => Err("interrupted".to_string()),
        _ = tokio::time::sleep_until(deadline + SET_GRACE) => Err("the run budget ran out".into()),
        observed = observe_set(args, &scratch, &groups, deadline) => observed,
    };
    let cleanup = groups.cleanup_all().await;
    let (initialize, reply, revision, rest) = observed?;
    cleanup.map_err(|e| format!("cleanup: {e}"))?;
    let mut reply = reply;
    if let Some(reply) = reply.as_mut() {
        evidence::redact(reply, &secrets(args, scratch.path(), &root));
    }
    let receipt = Receipt {
        date,
        client: Client::McpInspector,
        client_version: args.client_version.clone(),
        os: args.os.clone(),
        swarmail_commit: args.commit.clone(),
        executable_sha256: sha,
        transport: Transport::Http,
        negotiated_revision: revision,
        initialize_response: reply,
        steps: Steps {
            initialize,
            list_tools: rest[0].clone(),
            read: rest[1].clone(),
            wait_for_code: rest[2].clone(),
            structured_content: rest[3].clone(),
            list_attachments: rest[4].clone(),
        },
        recorded_by,
        mode: Mode::Headless,
    };
    let mut bytes = serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    receipt::validate(&bytes, &name).map_err(|e| format!("the draft is invalid evidence: {e}"))?;
    crate::artifacts::publish(&root, &name, &bytes)
        .map_err(|e| format!("{name} was not installed: {e}"))?;
    Ok((root.join(name), receipt))
}

type Observed = (
    Outcome,
    Option<serde_json::Value>,
    Option<String>,
    Vec<Outcome>,
);

/// Start the server, store the fixture and observe the five launches.
async fn observe_set(
    args: &RunArgs,
    scratch: &Arc<tempfile::TempDir>,
    groups: &crate::runner::Groups,
    deadline: Instant,
) -> Result<Observed, String> {
    let (server, smtp, http) =
        crate::runner::start_memory_swarmail(&args.binary, scratch.path(), groups, scratch.clone())
            .await
            .map_err(|e| format!("swarmail did not start: {e}"))?;
    let message = fixture_message();
    let (sent, id) = crate::mail::send_stored(
        smtp,
        crate::mail::SmtpAuth::None,
        "",
        "sender@matrix.test",
        "operator@matrix.test",
        &message,
    )
    .await;
    let id = match (sent, id) {
        (crate::mail::SendResult::Delivered(_), Some(id)) => id,
        _ => return Err("the fixture's final DATA reply was not 250 stored as <id>".into()),
    };
    let upstream = format!("http://{http}/mcp");
    let mut steps = Vec::new();
    let mut first: Option<(Outcome, Option<serde_json::Value>, Option<String>)> = None;
    for (index, (step, call)) in evidence::plan(&id).into_iter().enumerate() {
        if let Some((init, _, _)) = &first
            && init.status == Status::Fail
        {
            steps.push(Outcome::fail(
                "blocked: the first launch's initialize failed",
            ));
            continue;
        }
        if server.exited() {
            return Err("swarmail exited during the set; the set is abandoned".into());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            steps.push(Outcome::fail("the run budget ran out before this launch"));
            continue;
        }
        let launch = launch_client(
            args, &upstream, http, &call, scratch, groups, index, remaining,
        )
        .await?;
        let revision = match &first {
            Some((_, _, revision)) => revision.clone(),
            None => {
                let handshake = evidence::first_handshake(&launch);
                let revision = handshake.2.clone();
                first = Some(handshake);
                revision
            }
        };
        let outcome = match (&first, revision) {
            (Some((init, _, _)), _) if init.status == Status::Fail => {
                Outcome::fail("blocked: the first launch's initialize failed")
            }
            (_, Some(revision)) => evidence::judge(step, &call, &launch, &revision, &id),
            (_, None) => Outcome::fail("no negotiated revision"),
        };
        steps.push(outcome);
    }
    if server.exited() {
        return Err("swarmail exited during the set; the set is abandoned".into());
    }
    let (initialize, reply, revision) = first.ok_or("no launch was observed")?;
    Ok((initialize, reply, revision, steps))
}

/// One CLI launch behind its own observer, bounded and reaped.
#[allow(clippy::too_many_arguments)]
async fn launch_client(
    args: &RunArgs,
    upstream: &str,
    http: std::net::SocketAddr,
    call: &Call,
    scratch: &Arc<tempfile::TempDir>,
    groups: &crate::runner::Groups,
    index: usize,
    remaining: Duration,
) -> Result<Launch, String> {
    let observer = crate::capture::observe(upstream, http).await?;
    let work = scratch.path().join(format!("client-{index}"));
    std::fs::create_dir(&work).map_err(|_| "private_directory")?;
    let mut cmd = tokio::process::Command::new(&args.client[0]);
    cmd.args(&args.client[1..])
        .arg("--cli")
        .arg(&observer.mcp_url)
        .args(call.inspector_args())
        .current_dir(&work);
    let client =
        crate::runner::run_captured(cmd, groups, scratch.clone(), LAUNCH_BOUND.min(remaining))
            .await
            .map(|(status, stdout)| (status.success(), stdout));
    if let Err(e) = &client
        && e == "helper_spawn"
    {
        return Err(format!(
            "the client {:?} could not be started",
            args.client[0]
        ));
    }
    let (exchanges, overflowed) = observer.seal();
    Ok(Launch {
        exchanges,
        overflowed,
        client,
    })
}

/// What the captured reply must not carry: this host's name, the user's
/// home and name, and the paths of this run.
fn secrets(args: &RunArgs, scratch: &Path, root: &Path) -> Vec<String> {
    let mut out = vec![
        args.binary.display().to_string(),
        scratch.display().to_string(),
        root.display().to_string(),
    ];
    if let Some(parent) = args.binary.parent() {
        out.push(parent.display().to_string());
    }
    for var in ["HOME", "USER", "LOGNAME"] {
        if let Some(value) = std::env::var_os(var) {
            out.push(value.to_string_lossy().into_owned());
        }
    }
    let mut host = [0u8; 256];
    // SAFETY: the buffer is valid for its length; the name is NUL-terminated
    // within it on success.
    if unsafe { libc::gethostname(host.as_mut_ptr().cast(), host.len()) } == 0 {
        let end = host.iter().position(|b| *b == 0).unwrap_or(host.len());
        out.push(String::from_utf8_lossy(&host[..end]).into_owned());
    }
    out.retain(|s| s != "/" && !s.is_empty());
    out
}
