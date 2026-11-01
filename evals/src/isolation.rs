//! The whole-trial boundary of proposed decision 0115 — prepared, not
//! activated — built by invoking bubblewrap (revision 2026-11-01).
//!
//! `trial --isolated` (a hidden flag) validates its request, then hands it to
//! [`run`] (which drives [`launch`]): the *controller*, which stays in the host namespaces, checks
//! that `/usr/bin/bwrap` exists and is at least [`BWRAP_MIN`], proves the
//! boundary with one bounded run of the exact production argv, takes the host
//! admission (accepted 0076) and supervises one `bwrap` process whose
//! sandbox runs the unchanged trial arm. Anything that cannot be done
//! refuses by name — `boundary_unavailable:<step>` — before a server or an
//! agent exists. There is no fallback that runs the trial unisolated, no
//! host-network or host-filesystem mode, and no override of the bwrap path:
//! the one place a test reaches another binary is the `bwrap` parameter of
//! [`launch`], which production fills with the constant [`BWRAP`].
//!
//! bubblewrap is a host prerequisite, not a crate dependency: it is executed
//! as a separate process and never linked, vendored or downloaded.
//!
//! Delivery seats cannot create namespaces (bwrap refuses there with
//! `ENOSPC`), so the live path has never run in the seat that wrote it. Its
//! pure parts (the version parser, the argv builder, the inner-request
//! renderer's contract, the runner finder) and its refusal arms are
//! unit-tested here; its live behaviour is proven only by the `#[ignore]`d
//! suite in `tests/trial_isolation.rs`, run on a host whose bwrap can create
//! the namespaces.
//!
//! Nothing here widens what a trial may do: concurrency stays refused by the
//! public CLI and by admission (accepted 0076), and no row field claims
//! isolation (0115, Current integration 1–2).

use std::ffi::OsString;
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};

/// The only bubblewrap this crate will run. Absolute, never looked up on
/// `PATH`, overridable by no environment variable and no flag: an override is
/// the lever that becomes an unisolated fallback.
pub const BWRAP: &str = "/usr/bin/bwrap";

/// The oldest bubblewrap the boundary has been observed to work on. A
/// compatibility statement, not a security attestation of a distribution's
/// patch level; lowering it carries a live proof on the lower version.
pub const BWRAP_MIN: (u64, u64, u64) = (0, 11, 1);

/// The size of the trial's private `/tmp`: bounded so a hostile agent fills
/// only its own trial's memory.
pub const TMP_BYTES: u64 = 512 * 1024 * 1024;

/// How long `bwrap --version` may take.
pub const VERSION_BOUND: Duration = Duration::from_secs(5);
/// How long the preflight probe (the production argv with `/usr/bin/true`)
/// may take.
pub const PROBE_BOUND: Duration = Duration::from_secs(10);
/// How long the controller waits for its two pipe copies after bwrap exits,
/// for bwrap to be reaped after it is killed, and for its own runtime to shut
/// down.
const DRAIN_BOUND: Duration = Duration::from_secs(5);

/// The grace, in milliseconds, past a trial's deadline before its process
/// is asked to stop. One number for both callers: the matrix's per-trial
/// envelope (`run_child_assigned`) and the isolated controller's bound on
/// bwrap's own exit ([`Bounds::for_trial`]). Changing it changes both.
pub const TRIAL_GRACE_MS: u64 = 300_000;

/// The controller's own waits. Every wait it makes is bounded: the version
/// and probe by [`VERSION_BOUND`] and [`PROBE_BOUND`], bwrap's exit by
/// `envelope`, and the pipe copies, the reap after a kill and the runtime's
/// shutdown by `drain`. A parameter beside the bwrap path so a stand-in test
/// proves each bound in milliseconds; production fills it with
/// [`Bounds::for_trial`].
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    /// How long bwrap may run before it is killed.
    pub envelope: Duration,
    /// How long each of the drains, the reap and the shutdown may take.
    pub drain: Duration,
}

impl Bounds {
    /// The production bounds for a trial whose deadline is `deadline_ms`:
    /// the deadline plus [`TRIAL_GRACE_MS`], and [`DRAIN_BOUND`].
    pub fn for_trial(deadline_ms: u64) -> Self {
        Self {
            envelope: Duration::from_millis(deadline_ms.saturating_add(TRIAL_GRACE_MS)),
            drain: DRAIN_BOUND,
        }
    }
}

/// The one line the controller prints on stderr when the envelope expired
/// and bwrap was killed. Fixed text: no bwrap output and no number a row
/// could be confused with.
pub const ENVELOPE_EXPIRED: &str =
    "swarmail-evals: isolated trial envelope expired; bwrap killed\n";

/// The accepted 0076 §3 harness-error line for a reap the controller could
/// not prove. It is the line the unisolated `trial` process already writes
/// and the matrix already parses (`capture_worker_category`), not a new row
/// reason.
pub const CLEANUP_UNPROVED: &str = "swarmail-evals: harness error: trial_cleanup_unproved\n";

/// Whether bwrap was reaped, which with the runner as PID 1 is the proof the
/// namespace is empty (0076 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanup {
    /// `wait` returned: on its own, or within the drain bound of the
    /// envelope kill, or no child ever existed.
    Proved,
    /// The envelope expired and the bounded reap after the kill ran out.
    Unproved,
}

/// What the controller writes on stderr when the envelope expired: the fixed
/// envelope line, and after it, only when the reap ran out, the 0076 §3 line.
pub fn envelope_report(cleanup: Cleanup) -> &'static str {
    match cleanup {
        Cleanup::Proved => ENVELOPE_EXPIRED,
        Cleanup::Unproved => concat!(
            "swarmail-evals: isolated trial envelope expired; bwrap killed\n",
            "swarmail-evals: harness error: trial_cleanup_unproved\n"
        ),
    }
}

/// Read-only system trees every isolated trial may see. A tree that is a
/// symlink on the host (merged-usr `/bin`, `/lib`) is recreated as the same
/// symlink; a missing tree is skipped. Nothing else of the host is visible.
pub const SYSTEM_ROOTS: [&str; 5] = ["/usr", "/bin", "/lib", "/lib64", "/etc"];

/// Preconditions refused before any boundary work, by name.
pub const REQUIRES_SWARMAIL_BIN: &str = "isolation_requires_swarmail_bin";
pub const ARTIFACTS_UNSUPPORTED: &str = "isolation_artifacts_unsupported";
pub const WRAPPER_UNSUPPORTED: &str = "isolation_wrapper_unsupported";
pub const EXPOSE_INVALID: &str = "isolation_expose_invalid";
pub const EXTERNAL_AGENT_UNSUPPORTED: &str = "isolation_external_agent_unsupported";
pub const SOURCE_INVALID: &str = "isolation_source_invalid";

/// The closed set of boundary steps a refusal can name.
pub const STEP_PLATFORM: &str = "platform";
pub const STEP_ABSENT: &str = "bwrap_absent";
pub const STEP_VERSION: &str = "bwrap_version";
pub const STEP_PROBE: &str = "bwrap_probe";

/// Harness errors the controller itself reports once a launch was admitted.
pub const NO_ROW: &str = "isolated_trial_no_row";
pub const INTERRUPTED: &str = "isolated_trial_interrupted";

/// Every reason `--isolated` can refuse with, in the form the diagnostic
/// prints. A row never carries bwrap's own text: that goes to stderr beside
/// the row (0075: finite categories in the row, raw diagnostics beside it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A request precondition, or an admission verdict, named as today.
    Named(&'static str),
    /// A boundary step that failed: `boundary_unavailable:<step>`.
    Boundary { step: &'static str },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => f.write_str(name),
            Self::Boundary { step } => write!(f, "boundary_unavailable:{step}"),
        }
    }
}

/// `Ok` where the platform has bubblewrap. macOS (and anything else that is
/// not Linux) refuses by name: bwrap is Linux-only.
pub fn platform(is_linux: bool) -> Result<(), Refusal> {
    if is_linux {
        Ok(())
    } else {
        Err(Refusal::Boundary {
            step: STEP_PLATFORM,
        })
    }
}

/// The `major.minor.patch` triple of `bubblewrap <major>.<minor>.<patch>`,
/// trailing text (`-ubuntu1`, a newline, a banner) ignored. Anything else is
/// `None`.
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let rest = text.trim().strip_prefix("bubblewrap ")?;
    let token = rest.split_whitespace().next()?;
    let numeric = token
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .next()?;
    let mut parts = numeric.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Why a path cannot be bound into the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    Relative,
    Missing,
    NotRegular,
    Symlinked,
}

/// Whether `path` lies below one of the read-only system trees, which are
/// bound wholesale and need no binding of their own.
pub fn in_system_root(path: &Path) -> bool {
    SYSTEM_ROOTS.iter().any(|root| path.starts_with(root))
}

/// A path that may be bound at its own location: absolute, present, a regular
/// file, and reached through no symlink at all (the final component or any
/// directory above it), so what is bound is exactly what was named.
pub fn resolve_source(path: &Path) -> Result<PathBuf, SourceError> {
    if !path.is_absolute() {
        return Err(SourceError::Relative);
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| SourceError::Missing)?;
    if meta.file_type().is_symlink() {
        return Err(SourceError::Symlinked);
    }
    if !meta.is_file() {
        return Err(SourceError::NotRegular);
    }
    match std::fs::canonicalize(path) {
        Ok(real) if real == path => Ok(path.to_path_buf()),
        Ok(_) => Err(SourceError::Symlinked),
        Err(_) => Err(SourceError::Missing),
    }
}

/// The files bound at their own absolute paths: every named source that is
/// not already below a system tree, once each, in the order named.
pub fn bound_files(sources: &[PathBuf]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for source in sources {
        if !in_system_root(source) && !files.contains(source) {
            files.push(source.clone());
        }
    }
    files
}

/// What the boundary binds: the fixed read-only trees, plus the named
/// executables and exposed files at their own absolute paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindPlan {
    pub system: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
}

/// Resolve every named source against the allowlist. A source below a system
/// tree is already visible (it need only exist as a file); any other is bound
/// at its own path after [`resolve_source`] accepts it.
pub fn bind_plan(sources: &[PathBuf]) -> Result<BindPlan, SourceError> {
    for source in sources {
        if !source.is_absolute() {
            return Err(SourceError::Relative);
        }
        if in_system_root(source) {
            let meta = std::fs::metadata(source).map_err(|_| SourceError::Missing)?;
            if !meta.is_file() {
                return Err(SourceError::NotRegular);
            }
            continue;
        }
        resolve_source(source)?;
    }
    Ok(BindPlan {
        system: SYSTEM_ROOTS.iter().map(PathBuf::from).collect(),
        files: bound_files(sources),
    })
}

/// What the isolated trial needs from the host, resolved before any
/// boundary work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub swarmail_bin: PathBuf,
    pub runner: PathBuf,
    pub agent: Option<PathBuf>,
    pub expose: Vec<PathBuf>,
}

impl Request {
    /// Every host file the boundary must bind, in a stable order.
    pub fn sources(&self) -> Vec<PathBuf> {
        let mut all = vec![self.runner.clone(), self.swarmail_bin.clone()];
        all.extend(self.agent.iter().cloned());
        all.extend(self.expose.iter().cloned());
        all
    }
}

/// A path the caller named, after the caller tried to resolve it: `Err` is a
/// name that could not be resolved to a path at all (an unreadable
/// `SWARMAIL_BIN`, a program found nowhere). The caller resolves before this
/// runs only because resolving has no effect; which refusal is reported is
/// decided here, in one place, in the documented order.
pub type Named = Result<PathBuf, ()>;

/// Validate an isolated request. Each refusal is named and precedes every
/// namespace operation, and the first one hit wins, in this order:
/// `isolation_requires_swarmail_bin`, `isolation_artifacts_unsupported`,
/// `isolation_wrapper_unsupported`, `isolation_expose_invalid`,
/// `isolation_external_agent_unsupported`, `isolation_source_invalid`.
/// `concurrency_unsupported` and `concurrent_trials_refused` keep their
/// existing meaning and are decided elsewhere.
///
/// `agent` is `None` for a builtin agent (or no program at all), and for an
/// external program its resolved canonical path. An external program runs
/// isolated only if that path is one of the `--isolated-expose` files: until
/// the operator rules on provider egress (0115) the only programs admitted
/// are the builtins and scripted agents the caller deliberately exposed. A
/// program that did not resolve at all cannot be exposed and is reported by
/// the later, more precise `isolation_source_invalid`.
pub fn request(
    swarmail_bin: Option<Named>,
    artifacts: bool,
    wrapper: bool,
    expose: &[PathBuf],
    runner: PathBuf,
    agent: Option<Named>,
) -> Result<Request, Refusal> {
    if swarmail_bin.is_none() {
        return Err(Refusal::Named(REQUIRES_SWARMAIL_BIN));
    }
    if artifacts {
        return Err(Refusal::Named(ARTIFACTS_UNSUPPORTED));
    }
    if wrapper {
        return Err(Refusal::Named(WRAPPER_UNSUPPORTED));
    }
    // An exposed file is reached through no symlink, so its path is its
    // canonical path and membership is plain equality.
    let mut canonical_expose = Vec::with_capacity(expose.len());
    for path in expose {
        canonical_expose.push(resolve_source(path).map_err(|_| Refusal::Named(EXPOSE_INVALID))?);
    }
    if let Some(Ok(program)) = &agent
        && !canonical_expose.contains(program)
    {
        return Err(Refusal::Named(EXTERNAL_AGENT_UNSUPPORTED));
    }
    let source_invalid = || Refusal::Named(SOURCE_INVALID);
    let request = Request {
        swarmail_bin: swarmail_bin
            .and_then(Result::ok)
            .ok_or_else(source_invalid)?,
        runner,
        agent: agent.transpose().map_err(|_| source_invalid())?,
        expose: canonical_expose,
    };
    bind_plan(&request.sources()).map_err(|_| source_invalid())?;
    Ok(request)
}

/// What the host has at one of the [`SYSTEM_ROOTS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tree {
    /// Nothing usable: skipped.
    Missing,
    /// A directory: bound read-only at its own path.
    Directory,
    /// A symlink (merged-usr `/bin`): recreated as the same symlink.
    Link(PathBuf),
}

/// The host's own layout of one system tree.
pub fn host_tree(path: &Path) -> Tree {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            std::fs::read_link(path).map_or(Tree::Missing, Tree::Link)
        }
        Ok(meta) if meta.is_dir() => Tree::Directory,
        _ => Tree::Missing,
    }
}

/// The bwrap options that bring the read-only system trees in, each a
/// read-only bind or, where the host has a symlink, the same symlink.
pub fn system_root_options(layout: &dyn Fn(&Path) -> Tree) -> Vec<OsString> {
    let mut options = Vec::new();
    for root in SYSTEM_ROOTS {
        match layout(Path::new(root)) {
            Tree::Missing => {}
            Tree::Directory => options.extend(["--ro-bind", root, root].map(OsString::from)),
            Tree::Link(target) => {
                options.extend([
                    OsString::from("--symlink"),
                    target.into_os_string(),
                    OsString::from(root),
                ]);
            }
        }
    }
    options
}

/// The argv of the one bwrap process, as a pure function of the validated
/// request and the host's layout. The options come first and end at the first
/// `--`; `inner` is the runner's own argv and follows it, so no agent argument
/// can become a bwrap option: the index of the first `--` depends on the
/// request alone.
///
/// Never `--ro-bind / /` (a read-only host root still lets a process connect
/// to the forwarded agent sockets under `/run/user/<uid>`), never
/// `--unshare-all` or a `-try` unshare (an unavailable namespace must refuse,
/// not be skipped), never `--share-net` (it would put the trial's own
/// Swarmail and fixture on the host loopback).
///
/// `--as-pid-1` makes bwrap install no reaper: the process it execs, the
/// runner, is PID 1 of the trial's PID namespace. bwrap's default reaper
/// would be a same-uid, dumpable process inside the boundary that a sibling
/// trial's agent could probe through `/proc/1`; with it gone, every process
/// inside is the runner or something the runner spawned behind the
/// controller's close-on-exec sweep.
pub fn argv(
    request: &Request,
    layout: &dyn Fn(&Path) -> Tree,
    inner: &[OsString],
) -> Vec<OsString> {
    let mut a: Vec<OsString> = [
        "--unshare-user",
        "--unshare-pid",
        "--unshare-net",
        "--unshare-ipc",
        "--die-with-parent",
        "--new-session",
        "--as-pid-1",
        "--clearenv",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--setenv",
        "HOME",
        "/tmp",
        "--setenv",
        "TMPDIR",
        "/tmp",
        "--setenv",
        "SWARMAIL_BIN",
    ]
    .map(OsString::from)
    .into();
    a.push(request.swarmail_bin.clone().into_os_string());
    // The private /tmp comes before every file bind: a file that lives below
    // /tmp on the host is then bound into this tmpfs, not hidden under it.
    a.extend(["--size".into(), TMP_BYTES.to_string().into()]);
    a.extend(["--tmpfs", "/tmp"].map(OsString::from));
    a.extend(system_root_options(layout));
    for file in bound_files(&request.sources()) {
        a.push("--ro-bind".into());
        a.push(file.clone().into_os_string());
        a.push(file.into_os_string());
    }
    a.extend(["--proc", "/proc", "--dev", "/dev", "--chdir", "/tmp", "--"].map(OsString::from));
    a.extend(inner.iter().cloned());
    a
}

/// [`argv`] over this host's real layout.
pub fn command(request: &Request, inner: &[OsString]) -> Vec<OsString> {
    argv(request, &host_tree, inner)
}

/// One process of the host's table, as the runner finder needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcEntry {
    pub pid: i32,
    pub parent: i32,
    pub cmdline: Vec<u8>,
}

impl ProcEntry {
    /// Whether this process was started as `<runner> trial …`.
    fn is_runner(&self, runner: &Path) -> bool {
        let mut args = self.cmdline.split(|b| *b == 0);
        args.next() == Some(runner.as_os_str().as_bytes()) && args.next() == Some(b"trial")
    }
}

/// The trial's runner among the descendants of `root`: the process started as
/// `<runner> trial …` that has no such process above it. A built-in agent is
/// the same executable (`<runner> agent oracle`) and never matches, and a
/// nested `trial` below the runner is never preferred to it.
pub fn pick_runner(table: &[ProcEntry], root: i32, runner: &Path) -> Option<i32> {
    let by_pid: std::collections::HashMap<i32, &ProcEntry> =
        table.iter().map(|e| (e.pid, e)).collect();
    let below = |entry: &ProcEntry| -> Option<Vec<i32>> {
        // Ancestors from the entry's parent up to `root`, or None when the
        // chain does not reach it.
        let mut chain = Vec::new();
        let mut at = entry.parent;
        while at != root {
            chain.push(at);
            at = by_pid.get(&at)?.parent;
            if chain.len() > table.len() {
                return None;
            }
        }
        Some(chain)
    };
    table
        .iter()
        .filter(|e| e.is_runner(runner))
        .filter_map(|e| below(e).map(|chain| (e, chain)))
        .filter(|(_, chain)| {
            !chain
                .iter()
                .any(|pid| by_pid.get(pid).is_some_and(|e| e.is_runner(runner)))
        })
        .map(|(e, _)| e.pid)
        .min()
}

/// The host's process table: pid, parent and command line of every process
/// this uid can read.
pub fn proc_table() -> Vec<ProcEntry> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid: i32 = entry.file_name().to_str()?.parse().ok()?;
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            let (_, tail) = stat.rsplit_once(')')?;
            let parent = tail.split_whitespace().nth(1)?.parse().ok()?;
            let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
            Some(ProcEntry {
                pid,
                parent,
                cmdline,
            })
        })
        .collect()
}

/// The host PID of the runner below `root`, if one exists yet.
pub fn runner_pid(root: i32, runner: &Path) -> Option<i32> {
    pick_runner(&proc_table(), root, runner)
}

/// How a launched trial ended, as far as the controller can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launched {
    /// A row reached stdout; the controller exits with this code, which is
    /// the runner's (bwrap propagates it).
    Exited(u8),
    /// bwrap ended and no byte reached stdout: a named harness error.
    NoRow,
    /// An interrupt arrived before a runner existed: bwrap was killed.
    Interrupted,
}

/// Mark every descriptor above stderr close-on-exec in the child between fork
/// and exec, so nothing the controller was handed survives into bwrap.
#[cfg(target_os = "linux")]
fn sweep(process: &mut tokio::process::Command) {
    const CLOSE_RANGE_CLOEXEC: libc::c_uint = 1 << 2;
    // SAFETY: the closure runs between fork and exec and makes one raw
    // `close_range` system call; it allocates nothing and takes no lock.
    unsafe {
        process.pre_exec(|| {
            let rc = libc::syscall(
                libc::SYS_close_range,
                3 as libc::c_uint,
                libc::c_uint::MAX,
                CLOSE_RANGE_CLOEXEC,
            );
            if rc == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn sweep(_: &mut tokio::process::Command) {}

/// One bwrap process over `inner`, with the controller's stdio discipline:
/// no stdin, both output pipes owned by the controller, its own process
/// group (a terminal interrupt reaches the controller only), no inherited
/// environment, and the descriptor sweep. Dropping the child kills bwrap,
/// whose `--die-with-parent` takes the sandbox with it.
fn process(bwrap: &Path, request: &Request, inner: &[OsString]) -> tokio::process::Command {
    let mut process = tokio::process::Command::new(bwrap);
    process
        .args(command(request, inner))
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .process_group(0);
    sweep(&mut process);
    process
}

/// `bwrap_absent`: missing, not a regular file, or not executable.
fn present(bwrap: &Path) -> Result<(), Refusal> {
    match std::fs::metadata(bwrap) {
        Ok(meta) if meta.is_file() && meta.permissions().mode() & 0o111 != 0 => Ok(()),
        _ => Err(Refusal::Boundary { step: STEP_ABSENT }),
    }
}

/// `bwrap_version`: `--version` finished in time, printed a parseable
/// version, and it is at least [`BWRAP_MIN`]. Checked on every launch.
async fn version_ok(bwrap: &Path) -> Result<(), Refusal> {
    let refusal = || Refusal::Boundary { step: STEP_VERSION };
    let mut ask = tokio::process::Command::new(bwrap);
    ask.arg("--version")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    sweep(&mut ask);
    let child = ask.spawn().map_err(|_| refusal())?;
    let output = tokio::time::timeout(VERSION_BOUND, child.wait_with_output())
        .await
        .map_err(|_| refusal())?
        .map_err(|_| refusal())?;
    let text = String::from_utf8_lossy(&output.stdout);
    match parse_version(&text) {
        Some(found) if output.status.success() && found >= BWRAP_MIN => Ok(()),
        _ => Err(refusal()),
    }
}

/// `bwrap_probe`: the exact production argv, with `/usr/bin/true` as the
/// command, exits 0 within the bound. bwrap's own words go to this process's
/// stderr unchanged and never into a row.
async fn probe(bwrap: &Path, request: &Request) -> Result<(), Refusal> {
    let refusal = || Refusal::Boundary { step: STEP_PROBE };
    let mut run = process(bwrap, request, &[OsString::from("/usr/bin/true")]);
    run.stdout(Stdio::null());
    let child = run.spawn().map_err(|_| refusal())?;
    let output = tokio::time::timeout(PROBE_BOUND, child.wait_with_output())
        .await
        .map_err(|_| refusal())?
        .map_err(|_| refusal())?;
    write_through(Sink::Stderr, &output.stderr);
    if output.status.success() {
        Ok(())
    } else {
        Err(refusal())
    }
}

/// The controller's own output streams, written with blocking calls on the
/// thread that drives the runtime, the only thread that may take the
/// process's standard-stream locks, never from a blocking task (tokio's
/// `io-std` feature is not enabled in this crate).
#[derive(Clone, Copy)]
enum Sink {
    Stdout,
    Stderr,
}

fn write_through(sink: Sink, bytes: &[u8]) {
    use std::io::Write;
    match sink {
        Sink::Stdout => {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(bytes);
            let _ = out.flush();
        }
        Sink::Stderr => {
            let mut out = std::io::stderr().lock();
            let _ = out.write_all(bytes);
            let _ = out.flush();
        }
    }
}

/// Copy `from` to the controller's `sink` byte for byte, adding every read to
/// `seen` the moment it arrives. A failing sink never stops the drain: the
/// sandbox must not block on a pipe.
///
/// The write happens on the thread that drives this future, which is the
/// controller's main thread (see [`run`]): no other thread ever has to take
/// the process's standard-stream lock on its behalf, so a lock the caller
/// holds can never park the copy. The count is the read side's, so a drain
/// that expires never turns a delivered row into a missing one.
async fn pump(mut from: impl AsyncRead + Unpin, sink: Sink, seen: Arc<AtomicU64>) {
    let mut buffer = [0u8; 8192];
    loop {
        match from.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                seen.fetch_add(n as u64, Ordering::Relaxed);
                write_through(sink, &buffer[..n]);
            }
        }
    }
}

/// Run the trial inside bwrap and judge how it ended. Called after the probe
/// and the host admission; the admission is the caller's to hold until this
/// returns, that is until bwrap's `wait` has returned. The runner is PID 1 of
/// the trial's namespace (`--as-pid-1`), so that happens only after the
/// kernel has emptied the namespace.
///
/// bwrap forwards no signals, and its `--die-with-parent` kill is `SIGKILL`,
/// which would lose the interrupted row. So a `SIGTERM`/`SIGINT` here is
/// delivered, once, to the runner's host PID (the runner then emits its own
/// interrupted row, exactly as an unisolated trial does, 0073; as namespace
/// init it receives the signal only once it has installed that handler, and
/// an earlier one is dropped, so that trial ends at its deadline); with no
/// runner yet, bwrap is killed and the interrupt is reported by name.
///
/// Every wait is bounded by `bounds`. If bwrap outlives `bounds.envelope` it
/// is killed (`--die-with-parent` empties the namespace), reaped within
/// `bounds.drain`, and [`envelope_report`] goes to stderr; the verdict is
/// then the usual one: `Exited(2)` beside whatever row reached stdout, or
/// `NoRow`. The copies are judged by the bytes the read side counted, not by
/// whether a drain finished in time.
///
/// The reap is judged beside the verdict: [`Cleanup::Unproved`] exactly when
/// the envelope expired and the bounded reap after the kill ran out.
async fn supervise(
    bwrap: &Path,
    request: &Request,
    inner: &[OsString],
    bounds: Bounds,
) -> (Launched, Cleanup) {
    use tokio::signal::unix::{SignalKind, signal};
    let (Ok(mut term), Ok(mut int)) = (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
    ) else {
        return (Launched::NoRow, Cleanup::Proved);
    };
    // The spawn happens on the caller's thread. A parent-death signal follows
    // the thread that forked, so `main` drives this future from its main
    // thread and nothing here moves the spawn into a spawned task.
    let Ok(mut child) = process(bwrap, request, inner).spawn() else {
        return (Launched::NoRow, Cleanup::Proved);
    };
    let root = child.id().map_or(-1, |pid| pid as i32);
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return (Launched::NoRow, Cleanup::Proved);
    };
    let seen = Arc::new(AtomicU64::new(0));
    let rows = tokio::spawn(pump(stdout, Sink::Stdout, Arc::clone(&seen)));
    let diagnostics = tokio::spawn(pump(stderr, Sink::Stderr, Arc::new(AtomicU64::new(0))));

    let envelope = tokio::time::sleep(bounds.envelope);
    tokio::pin!(envelope);
    let mut signalled = false;
    let mut abandoned = false;
    let mut cleanup = Cleanup::Proved;
    let status = loop {
        tokio::select! {
            status = child.wait() => break status,
            () = &mut envelope => {
                let _ = child.start_kill();
                let reaped = tokio::time::timeout(bounds.drain, child.wait()).await;
                if reaped.is_err() {
                    cleanup = Cleanup::Unproved;
                }
                write_through(Sink::Stderr, envelope_report(cleanup).as_bytes());
                break reaped.unwrap_or_else(|_| Err(std::io::Error::other("bwrap was not reaped")));
            }
            _ = term.recv(), if !signalled => {}
            _ = int.recv(), if !signalled => {}
        }
        signalled = true;
        match runner_pid(root, &request.runner) {
            Some(pid) => {
                // SAFETY: plain kill(2) on a PID read from the host table a
                // moment ago; at worst the runner has just exited.
                unsafe { libc::kill(pid, libc::SIGTERM) };
            }
            None => {
                abandoned = true;
                let _ = child.start_kill();
            }
        }
    };
    let _ = tokio::time::timeout(bounds.drain, rows).await;
    let _ = tokio::time::timeout(bounds.drain, diagnostics).await;
    (
        verdict(status, seen.load(Ordering::Relaxed), abandoned),
        cleanup,
    )
}

/// How a trial ended from what the controller observed: bwrap's wait result
/// (an error stands for "never reaped"), the bytes the read side counted on
/// stdout, and whether an interrupt abandoned the trial before a runner
/// existed. A reap that ran out is an `Err` here, so it exits 2 whatever
/// reached stdout.
fn verdict(
    status: std::io::Result<std::process::ExitStatus>,
    seen: u64,
    abandoned: bool,
) -> Launched {
    if abandoned {
        return Launched::Interrupted;
    }
    match (status.ok().and_then(|s| s.code()), seen) {
        (_, 0) => Launched::NoRow,
        (Some(code), _) => Launched::Exited(code as u8),
        (None, _) => Launched::Exited(2),
    }
}

/// The boundary's controller. Refuses, in this order and before any server or
/// agent exists, with `boundary_unavailable:{platform,bwrap_absent,
/// bwrap_version,bwrap_probe}`; then takes the host admission (0076 §1) and
/// holds it until bwrap has been reaped, which with the runner as PID 1 is
/// after the namespace is empty; then runs the trial. Nothing is cached
/// between launches, and nothing falls back to an unisolated run.
///
/// `bwrap` is a parameter so a test can aim the controller at a stand-in;
/// production passes [`BWRAP`]. The future must be driven from the main
/// thread (see [`supervise`]); production drives it through [`run`].
pub async fn launch(
    request: &Request,
    inner: &[OsString],
    bwrap: &Path,
    bounds: Bounds,
) -> Result<Launched, Refusal> {
    platform(cfg!(target_os = "linux"))?;
    present(bwrap)?;
    version_ok(bwrap).await?;
    probe(bwrap, request).await?;
    let admission = crate::admission::Admission::acquire().map_err(Refusal::Named)?;
    let (launched, cleanup) = supervise(bwrap, request, inner, bounds).await;
    match cleanup {
        Cleanup::Proved => drop(admission),
        // 0076 §3: never released as if cleanup were proved. Held to the
        // controller's exit, which `run` and `main` reach after the row.
        Cleanup::Unproved => admission.retain(),
    }
    Ok(launched)
}

/// The one way production drives [`launch`], and the one the unit tests
/// drive: a current-thread runtime on the calling thread, so the spawn, every
/// pipe copy and every write to the controller's standard streams happen on
/// that one thread, and a runtime that ends in at most `bounds.drain`.
/// Nothing here spawns a blocking-pool thread, so no destructor can wait on
/// one. The caller must hold no standard-stream guard across this call.
///
/// A runtime that cannot be built is a trial with no row.
pub fn run(
    request: &Request,
    inner: &[OsString],
    bwrap: &Path,
    bounds: Bounds,
) -> Result<Launched, Refusal> {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return Ok(Launched::NoRow);
    };
    let launched = runtime.block_on(launch(request, inner, bwrap, bounds));
    runtime.shutdown_timeout(bounds.drain);
    launched
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn os(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    #[test]
    fn refusal_reads_as_the_documented_diagnostic() {
        for step in [STEP_PLATFORM, STEP_ABSENT, STEP_VERSION, STEP_PROBE] {
            assert_eq!(
                Refusal::Boundary { step }.to_string(),
                format!("boundary_unavailable:{step}")
            );
            assert!(step.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'));
        }
        assert_eq!(
            Refusal::Named(REQUIRES_SWARMAIL_BIN).to_string(),
            "isolation_requires_swarmail_bin"
        );
    }

    #[test]
    fn the_platform_refusal_is_by_name_and_only_for_not_linux() {
        assert_eq!(platform(true), Ok(()));
        assert_eq!(
            platform(false).unwrap_err().to_string(),
            "boundary_unavailable:platform"
        );
    }

    #[test]
    fn the_version_parser_accepts_and_refuses_exactly() {
        for (text, want) in [
            ("bubblewrap 0.11.1", (0, 11, 1)),
            ("bubblewrap 0.11.1\n", (0, 11, 1)),
            ("bubblewrap 0.11.1-ubuntu1", (0, 11, 1)),
            ("bubblewrap 0.11.1 (extra words)", (0, 11, 1)),
            ("bubblewrap 1.0.0", (1, 0, 0)),
            ("bubblewrap 0.9.0", (0, 9, 0)),
            ("bubblewrap 0.11.0", (0, 11, 0)),
        ] {
            assert_eq!(parse_version(text), Some(want), "{text:?}");
        }
        for text in [
            "",
            "garbage",
            "bubblewrap",
            "bubblewrap ",
            "bubblewrap 0.11",
            "bubblewrap 0.11.1.2",
            "bubblewrap v0.11.1",
            "bubblewrap a.b.c",
            "bubblewrap ..",
            "bwrap 0.11.1",
            "0.11.1",
        ] {
            assert_eq!(parse_version(text), None, "{text:?}");
        }
        // The floor is a tuple comparison, not a string one.
        assert!(parse_version("bubblewrap 0.11.1").unwrap() >= BWRAP_MIN);
        assert!(parse_version("bubblewrap 0.11.2").unwrap() >= BWRAP_MIN);
        assert!(parse_version("bubblewrap 0.12.0").unwrap() >= BWRAP_MIN);
        assert!(parse_version("bubblewrap 1.0.0").unwrap() >= BWRAP_MIN);
        assert!(parse_version("bubblewrap 0.11.0").unwrap() < BWRAP_MIN);
        assert!(parse_version("bubblewrap 0.9.0").unwrap() < BWRAP_MIN);
        assert!(parse_version("bubblewrap 0.100.0").unwrap() >= BWRAP_MIN);
    }

    #[test]
    fn resolver_accepts_only_a_plain_absolute_regular_file() {
        let dir = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
        let file = dir.join("agent");
        std::fs::write(&file, b"#!/bin/sh\n").unwrap();
        assert_eq!(resolve_source(&file), Ok(file.clone()));

        assert_eq!(
            resolve_source(Path::new("relative/agent")),
            Err(SourceError::Relative)
        );
        assert_eq!(resolve_source(&dir.join("nope")), Err(SourceError::Missing));
        assert_eq!(resolve_source(&dir), Err(SourceError::NotRegular));

        let link = dir.join("link");
        symlink(&file, &link).unwrap();
        assert_eq!(resolve_source(&link), Err(SourceError::Symlinked));

        // A symlinked directory above the file is a symlink too.
        let real = dir.join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("tool"), b"x").unwrap();
        let alias = dir.join("alias");
        symlink(&real, &alias).unwrap();
        assert_eq!(
            resolve_source(&alias.join("tool")),
            Err(SourceError::Symlinked)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn bind_plan_names_exactly_the_allowlist() {
        let dir = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        // /etc/passwd is below a system tree: visible already, never bound.
        let system_file = PathBuf::from("/etc/passwd");
        assert!(system_file.is_file());
        let plan = bind_plan(&[a.clone(), system_file, b.clone(), a.clone()]).unwrap();
        assert_eq!(plan.files, vec![a, b]);
        assert_eq!(
            plan.system,
            ["/usr", "/bin", "/lib", "/lib64", "/etc"].map(PathBuf::from)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn bind_plan_refuses_every_unsafe_source() {
        let dir = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
        let file = dir.join("tool");
        std::fs::write(&file, b"x").unwrap();
        let link = dir.join("link");
        symlink(&file, &link).unwrap();
        let refused = |path: PathBuf| bind_plan(&[path]).unwrap_err();
        assert_eq!(refused(PathBuf::from("tool")), SourceError::Relative);
        assert_eq!(refused(dir.join("missing")), SourceError::Missing);
        assert_eq!(refused(dir.clone()), SourceError::NotRegular);
        assert_eq!(refused(link), SourceError::Symlinked);
        // A system tree does not excuse a missing or non-regular file.
        assert_eq!(
            refused(PathBuf::from("/etc/definitely-not-a-file")),
            SourceError::Missing
        );
        assert_eq!(refused(PathBuf::from("/etc")), SourceError::NotRegular);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn request_refuses_each_precondition_by_name_in_order() {
        let dir = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
        let bin = dir.join("swarmail");
        let runner = dir.join("runner");
        let program = dir.join("program");
        for file in [&bin, &runner, &program] {
            std::fs::write(file, b"x").unwrap();
        }
        let ask =
            |bin: Option<Named>, artifacts, wrapper, expose: &[PathBuf], agent: Option<Named>| {
                request(bin, artifacts, wrapper, expose, runner.clone(), agent)
            };
        let named = |name| Err(Refusal::Named(name));
        let missing = dir.join("no");
        let unexposed = Some(Ok(program.clone()));
        // Every refusal wins over the ones after it.
        assert_eq!(
            ask(
                None,
                true,
                true,
                std::slice::from_ref(&missing),
                unexposed.clone()
            ),
            named(REQUIRES_SWARMAIL_BIN)
        );
        assert_eq!(
            ask(
                Some(Ok(bin.clone())),
                true,
                true,
                std::slice::from_ref(&missing),
                unexposed.clone()
            ),
            named(ARTIFACTS_UNSUPPORTED)
        );
        assert_eq!(
            ask(
                Some(Ok(bin.clone())),
                false,
                true,
                std::slice::from_ref(&missing),
                unexposed.clone()
            ),
            named(WRAPPER_UNSUPPORTED)
        );
        for bad in [PathBuf::from("relative"), missing.clone(), dir.clone()] {
            assert_eq!(
                ask(
                    Some(Ok(bin.clone())),
                    false,
                    false,
                    &[bad],
                    unexposed.clone()
                ),
                named(EXPOSE_INVALID)
            );
        }
        // An external program that is not an exposed file is refused before
        // anything about the sources is looked at, even with an unusable
        // Swarmail selector.
        assert_eq!(
            ask(
                Some(Err(())),
                false,
                false,
                std::slice::from_ref(&runner),
                unexposed.clone()
            ),
            named(EXTERNAL_AGENT_UNSUPPORTED)
        );
        // A named executable that cannot be bound, or resolved, is its own
        // refusal; so is a program that resolved nowhere.
        assert_eq!(
            ask(Some(Ok(dir.join("missing"))), false, false, &[], None),
            named(SOURCE_INVALID)
        );
        assert_eq!(
            ask(Some(Err(())), false, false, &[], None),
            named(SOURCE_INVALID)
        );
        assert_eq!(
            ask(Some(Ok(bin.clone())), false, false, &[], Some(Err(()))),
            named(SOURCE_INVALID)
        );
        // An exposed external program is admitted, bound once, and a builtin
        // (no program) needs no exposure.
        let exposed = ask(
            Some(Ok(bin.clone())),
            false,
            false,
            std::slice::from_ref(&program),
            Some(Ok(program.clone())),
        )
        .unwrap();
        assert_eq!(exposed.agent, Some(program.clone()));
        assert_eq!(
            exposed.sources(),
            vec![
                runner.clone(),
                bin.clone(),
                program.clone(),
                program.clone()
            ]
        );
        let ok = ask(Some(Ok(bin.clone())), false, false, &[], None).unwrap();
        assert_eq!(ok.sources(), vec![runner.clone(), bin.clone()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // ---- the argv builder (AC-S4) -------------------------------------------

    fn golden_request() -> Request {
        Request {
            swarmail_bin: PathBuf::from("/w/bin/swarmail"),
            runner: PathBuf::from("/w/bin/swarmail-evals"),
            agent: Some(PathBuf::from("/w/agents/agent")),
            expose: vec![PathBuf::from("/w/probe/helper")],
        }
    }

    fn merged_usr(path: &Path) -> Tree {
        match path.to_str().unwrap() {
            "/usr" | "/etc" => Tree::Directory,
            "/bin" => Tree::Link("usr/bin".into()),
            "/lib" => Tree::Link("usr/lib".into()),
            "/lib64" => Tree::Link("usr/lib64".into()),
            other => panic!("not a system tree: {other}"),
        }
    }

    fn first_separator(argv: &[OsString]) -> usize {
        argv.iter().position(|a| a == "--").expect("a separator")
    }

    #[test]
    fn the_golden_argv_is_exactly_the_documented_list() {
        let inner = os(&["/w/bin/swarmail-evals", "trial", "--task", "signup"]);
        let got = argv(&golden_request(), &merged_usr, &inner);
        let want = os(&[
            "--unshare-user",
            "--unshare-pid",
            "--unshare-net",
            "--unshare-ipc",
            "--die-with-parent",
            "--new-session",
            "--as-pid-1",
            "--clearenv",
            "--setenv",
            "PATH",
            "/usr/bin:/bin",
            "--setenv",
            "HOME",
            "/tmp",
            "--setenv",
            "TMPDIR",
            "/tmp",
            "--setenv",
            "SWARMAIL_BIN",
            "/w/bin/swarmail",
            "--size",
            "536870912",
            "--tmpfs",
            "/tmp",
            "--ro-bind",
            "/usr",
            "/usr",
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            "--ro-bind",
            "/etc",
            "/etc",
            "--ro-bind",
            "/w/bin/swarmail-evals",
            "/w/bin/swarmail-evals",
            "--ro-bind",
            "/w/bin/swarmail",
            "/w/bin/swarmail",
            "--ro-bind",
            "/w/agents/agent",
            "/w/agents/agent",
            "--ro-bind",
            "/w/probe/helper",
            "/w/probe/helper",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--chdir",
            "/tmp",
            "--",
            "/w/bin/swarmail-evals",
            "trial",
            "--task",
            "signup",
        ]);
        assert_eq!(got, want);
        assert_eq!(TMP_BYTES, 536_870_912);
    }

    #[test]
    fn the_argv_never_widens_the_boundary() {
        let inner = os(&["/w/bin/swarmail-evals", "trial"]);
        for layout in [
            &merged_usr as &dyn Fn(&Path) -> Tree,
            &|_: &Path| Tree::Missing,
            &|_: &Path| Tree::Directory,
        ] {
            let got = argv(&golden_request(), layout, &inner);
            let options = &got[..first_separator(&got)];
            // No host root, however read-only: it would expose the
            // forwarded agent sockets under /run/user/<uid>.
            for window in options.windows(3) {
                if window[0] == "--ro-bind" || window[0] == "--bind" {
                    assert_ne!(window[1], "/", "{window:?}");
                    assert_ne!(window[2], "/", "{window:?}");
                }
            }
            for refused in ["--bind", "--dev-bind", "--share-net", "--unshare-all"] {
                assert!(!options.iter().any(|a| a == refused), "{refused}");
            }
            assert!(
                !options
                    .iter()
                    .any(|a| a.to_string_lossy().ends_with("-try")),
                "an unavailable namespace must refuse, not be skipped"
            );
            for must in [
                "--unshare-user",
                "--unshare-pid",
                "--unshare-net",
                "--unshare-ipc",
                "--die-with-parent",
                "--new-session",
                "--clearenv",
                "--proc",
                "--dev",
            ] {
                assert!(options.iter().any(|a| a == must), "{must}");
            }
            // The environment is cleared before any variable is set.
            let clear = options.iter().position(|a| a == "--clearenv").unwrap();
            for (at, a) in options.iter().enumerate() {
                if a == "--setenv" {
                    assert!(clear < at);
                }
            }
            // The size applies to the very next tmpfs, which is /tmp.
            let size = options.iter().position(|a| a == "--size").unwrap();
            assert_eq!(options[size + 1], "536870912");
            assert_eq!(options[size + 2], "--tmpfs");
            assert_eq!(options[size + 3], "/tmp");
            // The runner follows the first separator immediately.
            assert_eq!(got[first_separator(&got) + 1], "/w/bin/swarmail-evals");
        }
    }

    #[test]
    fn a_file_below_tmp_is_bound_into_the_private_tmpfs_not_under_it() {
        let request = Request {
            swarmail_bin: PathBuf::from("/tmp/x/swarmail"),
            runner: PathBuf::from("/tmp/x/runner"),
            agent: Some(PathBuf::from("/tmp/x/agent")),
            expose: Vec::new(),
        };
        let got = argv(&request, &merged_usr, &os(&["/tmp/x/runner"]));
        let tmpfs = got.iter().position(|a| a == "--tmpfs").unwrap();
        for (at, a) in got.iter().enumerate() {
            if a == "--ro-bind" && got[at + 1].to_string_lossy().starts_with("/tmp/") {
                assert!(tmpfs < at, "a /tmp bind would be hidden by the tmpfs");
            }
        }
    }

    #[test]
    fn system_trees_and_duplicates_are_not_bound_twice() {
        let request = Request {
            swarmail_bin: PathBuf::from("/usr/local/bin/swarmail"),
            runner: PathBuf::from("/w/runner"),
            agent: Some(PathBuf::from("/w/probe")),
            expose: vec![PathBuf::from("/w/probe"), PathBuf::from("/etc/hosts")],
        };
        let got = argv(&request, &|_| Tree::Missing, &os(&["/w/runner"]));
        let binds: Vec<_> = got
            .windows(3)
            .filter(|w| w[0] == "--ro-bind")
            .map(|w| w[1].clone())
            .collect();
        assert_eq!(binds, os(&["/w/runner", "/w/probe"]));
        // No tree at all on the host: only the fixed options remain.
        assert!(!got.iter().any(|a| a == "--symlink"));
    }

    #[test]
    fn no_agent_argument_can_become_a_bwrap_option() {
        let request = golden_request();
        let benign_agent = ["/w/agents/agent", "--flag", "value", "tail"];
        let inner_for = |agent: &[&str]| {
            let mut inner = os(&["/w/bin/swarmail-evals", "trial", "--"]);
            inner.extend(os(agent));
            inner
        };
        let benign = argv(&request, &merged_usr, &inner_for(&benign_agent));
        let split = first_separator(&benign);
        // Each hostile token, planted at every index of the agent argv (and
        // appended after it), leaves the options before the first `--`
        // untouched and the first `--` where the benign argv has it.
        let tokens = [
            "--ro-bind",
            "--share-net",
            "--as-pid-1",
            "--",
            "--isolated",
            "--chdir",
        ];
        for token in tokens {
            for at in 0..=benign_agent.len() {
                let mut agent = benign_agent.to_vec();
                agent.insert(at, token);
                let hostile = argv(&request, &merged_usr, &inner_for(&agent));
                assert_eq!(
                    first_separator(&hostile),
                    split,
                    "{token} at {at}: the first -- depends on the request only"
                );
                assert_eq!(benign[..=split], hostile[..=split], "{token} at {at}");
                let planted = hostile[..split].iter().filter(|o| *o == token).count();
                let own = benign[..split].iter().filter(|o| *o == token).count();
                assert_eq!(planted, own, "{token} at {at}: a planted option");
                // Everything after the separator is the runner's, verbatim.
                assert_eq!(
                    hostile[split + 1..],
                    inner_for(&agent)[..],
                    "{token} at {at}"
                );
            }
        }
        // `--as-pid-1` is the boundary's own, exactly once, before the
        // environment is cleared.
        let options = &benign[..split];
        assert_eq!(options.iter().filter(|o| *o == "--as-pid-1").count(), 1);
        let pid1 = options.iter().position(|o| o == "--as-pid-1").unwrap();
        let session = options.iter().position(|o| o == "--new-session").unwrap();
        let clear = options.iter().position(|o| o == "--clearenv").unwrap();
        assert!(session < pid1 && pid1 < clear);
    }

    #[test]
    fn the_host_layout_is_read_from_the_host() {
        // Each system tree is what the host's own metadata says it is: a
        // directory on most Linux hosts, a symlink where merged-usr or macOS
        // makes it one (`/etc` is a symlink on macOS), nothing where absent.
        for root in SYSTEM_ROOTS {
            let want = match std::fs::symlink_metadata(root) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    Tree::Link(std::fs::read_link(root).unwrap())
                }
                Ok(meta) if meta.is_dir() => Tree::Directory,
                _ => Tree::Missing,
            };
            assert_eq!(host_tree(Path::new(root)), want, "{root}");
        }
        assert_eq!(host_tree(Path::new("/definitely/not/here")), Tree::Missing);
        let dir = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
        std::fs::write(dir.join("file"), b"x").unwrap();
        symlink("usr/bin", dir.join("link")).unwrap();
        assert_eq!(host_tree(&dir.join("file")), Tree::Missing);
        assert_eq!(host_tree(&dir.join("link")), Tree::Link("usr/bin".into()));
        assert_eq!(host_tree(&dir), Tree::Directory);
        std::fs::remove_dir_all(&dir).unwrap();
        // The real argv is the pure one over the real layout.
        let inner = os(&["/w/bin/swarmail-evals", "trial"]);
        assert_eq!(
            command(&golden_request(), &inner),
            argv(&golden_request(), &host_tree, &inner)
        );
    }

    // ---- finding the runner (interrupts) --------------------------------------

    fn entry(pid: i32, parent: i32, cmdline: &str) -> ProcEntry {
        ProcEntry {
            pid,
            parent,
            cmdline: cmdline.replace(' ', "\0").into_bytes(),
        }
    }

    #[test]
    fn the_runner_is_the_shallowest_trial_below_the_controllers_bwrap() {
        let runner = Path::new("/r/swarmail-evals");
        let table = vec![
            entry(1, 0, "/sbin/init"),
            entry(100, 99, "/usr/bin/bwrap --unshare-user"),
            entry(101, 100, "bwrap --unshare-user"),
            entry(
                102,
                101,
                "/r/swarmail-evals trial --task signup --isolated-not",
            ),
            // A built-in agent is the same executable; it is not the runner.
            entry(105, 102, "/r/swarmail-evals agent oracle"),
            // Even a nested trial below the runner is never preferred to it.
            entry(106, 105, "/r/swarmail-evals trial --task nested"),
            // A look-alike outside the controller's tree is ignored.
            entry(200, 1, "/r/swarmail-evals trial --task other"),
            // A chain that never reaches the root is ignored.
            entry(300, 299, "/r/swarmail-evals trial --task orphan"),
        ];
        assert_eq!(pick_runner(&table, 100, runner), Some(102));
        // Asked about a different root, the answer follows the root.
        assert_eq!(pick_runner(&table, 1, runner), Some(200));
        // Not started yet.
        assert_eq!(pick_runner(&table[..3], 100, runner), None);
        // A different executable never matches, nor a bare name.
        assert_eq!(pick_runner(&table, 100, Path::new("/r/other")), None);
        assert_eq!(pick_runner(&table, 100, Path::new("swarmail-evals")), None);
        // Several independent runners: the lowest PID, deterministically.
        let two = vec![
            entry(100, 1, "bwrap"),
            entry(11, 100, "/r/swarmail-evals trial"),
            entry(10, 100, "/r/swarmail-evals trial"),
        ];
        assert_eq!(pick_runner(&two, 100, runner), Some(10));
        // A cycle in the table cannot hang the walk.
        let cycle = vec![entry(7, 8, "/r/swarmail-evals trial"), entry(8, 7, "x")];
        assert_eq!(pick_runner(&cycle, 100, runner), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_real_process_table_lists_this_process_with_its_parent() {
        let me = std::process::id() as i32;
        let table = proc_table();
        let mine = table.iter().find(|e| e.pid == me).expect("this process");
        // SAFETY: getppid has no preconditions.
        assert_eq!(mine.parent, unsafe { libc::getppid() });
        let exe = std::env::current_exe().unwrap();
        assert!(!mine.cmdline.is_empty());
        // This process is not a runner (it was not started as `<exe> trial`),
        // so nothing is found below its parent.
        assert_eq!(runner_pid(mine.parent, &exe), None);
    }

    #[test]
    fn the_reap_judgement_speaks_the_same_line_the_matrix_parses() {
        // The Proved pole is the envelope line alone, with no cleanup line.
        assert_eq!(envelope_report(Cleanup::Proved), ENVELOPE_EXPIRED);
        assert!(!envelope_report(Cleanup::Proved).contains("trial_cleanup_unproved"));
        // The Unproved pole is the envelope line, then the 0076 §3 line, once.
        let unproved = envelope_report(Cleanup::Unproved);
        assert_eq!(unproved, format!("{ENVELOPE_EXPIRED}{CLEANUP_UNPROVED}"));
        assert_eq!(unproved.matches("trial_cleanup_unproved").count(), 1);
        assert_eq!(
            CLEANUP_UNPROVED,
            "swarmail-evals: harness error: trial_cleanup_unproved\n"
        );
    }

    #[test]
    fn the_verdict_table_exits_two_whatever_reached_stdout_when_not_reaped() {
        use std::os::unix::process::ExitStatusExt;
        use std::process::ExitStatus;
        let code = |c: i32| Ok(ExitStatus::from_raw(c << 8));
        let signal = || Ok(ExitStatus::from_raw(libc::SIGKILL));
        let never = || Err(std::io::Error::other("bwrap was not reaped"));
        assert_eq!(verdict(never(), 0, false), Launched::NoRow);
        assert_eq!(verdict(never(), 9, false), Launched::Exited(2));
        assert_eq!(verdict(code(1), 9, false), Launched::Exited(1));
        assert_eq!(verdict(code(0), 9, false), Launched::Exited(0));
        assert_eq!(verdict(code(1), 0, false), Launched::NoRow);
        assert_eq!(verdict(signal(), 9, false), Launched::Exited(2));
        assert_eq!(verdict(code(0), 9, true), Launched::Interrupted);
        assert_eq!(verdict(never(), 0, true), Launched::Interrupted);
    }

    // ---- the controller's refusal arms and judgement (AC-S2) -------------------
    //
    // A stand-in `bwrap` is a generated shell script handed to `launch` through
    // its parameter: the version query, the probe (its last argument is
    // `/usr/bin/true`) and the real run each do what the test says. Nothing
    // here creates a namespace; the live boundary is the ignored suite's.

    #[cfg(target_os = "linux")]
    mod controller {
        use super::*;
        use crate::admission::Admission;

        fn request() -> Request {
            Request {
                swarmail_bin: PathBuf::from("/x/swarmail"),
                runner: PathBuf::from("/x/runner"),
                agent: None,
                expose: Vec::new(),
            }
        }

        fn inner() -> Vec<OsString> {
            os(&["/x/runner", "trial", "--task", "signup"])
        }

        /// Write an executable stand-in, and wait out `ETXTBSY` (a concurrent
        /// fork elsewhere in the test process may briefly hold the script's
        /// write descriptor).
        fn fake(dir: &Path, version: &str, probe: &str, real: &str) -> PathBuf {
            let path = dir.join("bwrap");
            let script = format!(
                "#!/bin/sh\ncase \"$1\" in --version) echo '{version}'; exit 0;; esac\nfor a in \"$@\"; do last=\"$a\"; done\nif [ \"$last\" = /usr/bin/true ]; then\n{probe}\nfi\n{real}\n"
            );
            write_executable(&path, &script);
            path
        }

        fn write_executable(path: &Path, content: &str) {
            std::fs::write(path, content).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
            for _ in 0..500 {
                match std::process::Command::new(path).arg("--version").output() {
                    Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    _ => return,
                }
            }
            panic!("the stand-in stayed busy");
        }

        /// The host admission, once any other thread's forked-but-not-yet-exec'd
        /// child has let go of its copy of the lock descriptor.
        fn admitted() -> Admission {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                match Admission::acquire() {
                    Ok(admission) => return admission,
                    Err(_) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(reason) => panic!("the admission stayed taken: {reason}"),
                }
            }
        }

        /// [`run`], waiting out the same fork window where the launch is
        /// expected to be admitted.
        fn run_when_free(bwrap: &Path) -> Result<Launched, Refusal> {
            run_when_free_bounded(bwrap, test_bounds())
        }

        fn run_when_free_bounded(bwrap: &Path, bounds: Bounds) -> Result<Launched, Refusal> {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                let outcome = run_bounded(bwrap, bounds);
                if outcome != Err(Refusal::Named("concurrent_trials_refused"))
                    || std::time::Instant::now() >= deadline
                {
                    return outcome;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        /// Bounds long enough for a shell stand-in to finish and short enough
        /// that a hang is a failure, not a wait.
        fn test_bounds() -> Bounds {
            Bounds {
                envelope: Duration::from_secs(60),
                drain: Duration::from_secs(5),
            }
        }

        /// The production entry, with test bounds.
        fn run(bwrap: &Path) -> Result<Launched, Refusal> {
            run_bounded(bwrap, test_bounds())
        }

        fn run_bounded(bwrap: &Path, bounds: Bounds) -> Result<Launched, Refusal> {
            super::super::run(&request(), &inner(), bwrap, bounds)
        }

        /// Millisecond bounds: the envelope and each drain expire in 200 ms.
        fn short() -> Bounds {
            Bounds {
                envelope: Duration::from_millis(200),
                drain: Duration::from_millis(200),
            }
        }

        fn boundary(step: &'static str) -> Result<Launched, Refusal> {
            Err(Refusal::Boundary { step })
        }

        #[test]
        fn a_missing_or_unrunnable_bwrap_refuses_bwrap_absent() {
            let dir = tempfile::tempdir().unwrap();
            assert_eq!(
                run(Path::new("/definitely/not/bwrap")),
                boundary(STEP_ABSENT)
            );
            // A directory is not a program.
            assert_eq!(run(dir.path()), boundary(STEP_ABSENT));
            // Nor is a file nobody may execute.
            let plain = dir.path().join("bwrap");
            std::fs::write(&plain, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(run(&plain), boundary(STEP_ABSENT));
            // The production path is a constant, absolute and outside PATH.
            assert!(Path::new(BWRAP).is_absolute());
        }

        #[test]
        fn an_old_or_unreadable_version_refuses_bwrap_version() {
            let dir = tempfile::tempdir().unwrap();
            let sentinel = dir.path().join("probe-ran");
            let touch = format!("touch '{}'; exit 0", sentinel.display());
            for version in [
                "bubblewrap 0.9.0",
                "bubblewrap 0.11.0",
                "garbage",
                "",
                "bubblewrap 0.11",
            ] {
                let bwrap = fake(dir.path(), version, &touch, &touch);
                assert_eq!(run(&bwrap), boundary(STEP_VERSION), "{version:?}");
            }
            // A program that is not a program at all (no interpreter line).
            let bwrap = dir.path().join("bwrap");
            write_executable_raw(&bwrap, "not a program\n");
            assert_eq!(run(&bwrap), boundary(STEP_VERSION));
            assert!(!sentinel.exists(), "no probe ran after a refused version");
        }

        fn write_executable_raw(path: &Path, content: &str) {
            std::fs::write(path, content).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        #[test]
        fn a_failing_probe_refuses_bwrap_probe_before_any_admission() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let ran = dir.path().join("trial-ran");
            // Someone else holds the host admission: were the probe to come
            // after it, this launch would be `concurrent_trials_refused`.
            let held = admitted();
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "echo 'bwrap: Creating new namespace failed: nesting depth (ENOSPC)' >&2; exit 1",
                &format!("touch '{}'; exit 0", ran.display()),
            );
            assert_eq!(run(&bwrap), boundary(STEP_PROBE));
            assert!(!ran.exists(), "no trial ran after a refused probe");
            // A probe that hangs is the same refusal only after its bound; a
            // probe that cannot be started is one at once.
            drop(held);
            let missing_interpreter = dir.path().join("other");
            write_executable_raw(&missing_interpreter, "#!/definitely/not/an/interpreter\n");
            assert_eq!(run(&missing_interpreter), boundary(STEP_VERSION));
        }

        #[test]
        fn a_good_probe_then_a_held_admission_refuses_concurrency_and_runs_nothing() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let ran = dir.path().join("trial-ran");
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                &format!("touch '{}'; echo row; exit 0", ran.display()),
            );
            let held = admitted();
            assert_eq!(
                run(&bwrap),
                Err(Refusal::Named("concurrent_trials_refused"))
            );
            assert!(!ran.exists());
            drop(held);
            assert_eq!(run_when_free(&bwrap), Ok(Launched::Exited(0)));
            assert!(ran.exists());
            // The admission was released when the launch ended.
            drop(admitted());
        }

        #[test]
        fn a_silent_exit_after_a_good_probe_is_a_named_no_row() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let bwrap = fake(dir.path(), "bubblewrap 0.11.1", "exit 0", "exit 1");
            assert_eq!(run_when_free(&bwrap), Ok(Launched::NoRow));
            // Killed by a signal with nothing on stdout: the same verdict.
            let bwrap = fake(dir.path(), "bubblewrap 0.11.1", "exit 0", "kill -9 $$");
            assert_eq!(run_when_free(&bwrap), Ok(Launched::NoRow));
            // bwrap's own words on stderr are not a row.
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                "echo 'bwrap: something' >&2; exit 1",
            );
            assert_eq!(run_when_free(&bwrap), Ok(Launched::NoRow));
            drop(admitted());
        }

        #[test]
        fn a_row_on_stdout_is_mirrored_with_the_exit_code_it_came_with() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            for code in [0u8, 1, 2] {
                let bwrap = fake(
                    dir.path(),
                    "bubblewrap 0.11.1",
                    "exit 0",
                    &format!("echo '{{\"schema_version\":2}}'; exit {code}"),
                );
                assert_eq!(run_when_free(&bwrap), Ok(Launched::Exited(code)));
            }
            // A death by signal after a row is exit 2, never a mirrored 0.
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                "echo row; kill -9 $$",
            );
            assert_eq!(run_when_free(&bwrap), Ok(Launched::Exited(2)));
        }

        /// Whether a process is gone, or only a zombie nobody has reaped.
        fn gone(pid: i32) -> bool {
            // SAFETY: signal 0 only probes for existence.
            let alive = unsafe { libc::kill(pid, 0) } == 0;
            !alive
                || std::fs::read_to_string(format!("/proc/{pid}/stat"))
                    .map_or(true, |s| s.contains(") Z "))
        }

        #[test]
        fn a_row_then_a_hung_bwrap_is_killed_at_the_envelope_and_still_delivers_the_row() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let pid_file = dir.path().join("bwrap-pid");
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                &format!(
                    "echo $$ > '{}'; echo '{{\"schema_version\":2}}'; exec /bin/sleep 60",
                    pid_file.display()
                ),
            );
            let started = std::time::Instant::now();
            assert_eq!(
                run_when_free_bounded(&bwrap, short()),
                Ok(Launched::Exited(2))
            );
            // Envelope plus reap, nowhere near the stand-in's 60 s.
            assert!(started.elapsed() < Duration::from_secs(30));
            let pid: i32 = std::fs::read_to_string(&pid_file)
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            assert!(gone(pid), "the stand-in bwrap survived the envelope");
            drop(admitted());
        }

        #[test]
        fn nothing_then_a_hung_bwrap_is_a_named_no_row_after_the_envelope() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                "exec /bin/sleep 60",
            );
            let started = std::time::Instant::now();
            assert_eq!(run_when_free_bounded(&bwrap, short()), Ok(Launched::NoRow));
            assert!(started.elapsed() < Duration::from_secs(30));
            drop(admitted());
        }

        /// The child half of the envelope-line test.
        #[test]
        fn envelope_line_child() {
            let (Some(bwrap), Some(result)) = (
                std::env::var_os("ISOLATION_TEST_BWRAP"),
                std::env::var_os("ISOLATION_TEST_RESULT"),
            ) else {
                return;
            };
            let outcome = run_bounded(Path::new(&bwrap), short());
            std::fs::write(result, format!("{outcome:?}")).unwrap();
        }

        #[test]
        fn an_expired_envelope_says_so_on_stderr_in_one_fixed_line() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let result = dir.path().join("result");
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                "echo '{\"schema_version\":2}'; echo 'bwrap: secret-words' >&2; exec /bin/sleep 60",
            );
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "isolation::tests::controller::envelope_line_child",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("ISOLATION_TEST_BWRAP", &bwrap)
                .env("ISOLATION_TEST_RESULT", &result)
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            assert_eq!(std::fs::read_to_string(&result).unwrap(), "Ok(Exited(2))");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(stderr.matches(ENVELOPE_EXPIRED).count(), 1, "{stderr:?}");
            // The stand-in was reaped within the bound: cleanup is proved.
            assert_eq!(
                stderr.matches("trial_cleanup_unproved").count(),
                0,
                "{stderr:?}"
            );
            assert_eq!(
                ENVELOPE_EXPIRED,
                "swarmail-evals: isolated trial envelope expired; bwrap killed\n"
            );
            // bwrap's own words still reach stderr, beside the fixed line.
            assert!(stderr.contains("bwrap: secret-words\n"), "{stderr:?}");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains("{\"schema_version\":2}\n"), "{stdout:?}");
        }

        #[test]
        fn a_row_then_a_grandchild_holding_stdout_is_still_a_delivered_row() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            // The grandchild inherits stdout and stderr and outlives bwrap
            // by far more than the drain; the row was read and counted.
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                "echo '{\"schema_version\":2}'; (/bin/sleep 5 &) ; exit 0",
            );
            let started = std::time::Instant::now();
            assert_eq!(
                run_when_free_bounded(&bwrap, short()),
                Ok(Launched::Exited(0))
            );
            assert!(started.elapsed() < Duration::from_secs(4));
            drop(admitted());
        }

        #[test]
        fn the_production_bounds_are_the_deadline_plus_the_matrix_grace() {
            assert_eq!(TRIAL_GRACE_MS, 300_000);
            let bounds = Bounds::for_trial(60_000);
            assert_eq!(bounds.envelope, Duration::from_millis(360_000));
            assert_eq!(bounds.drain, DRAIN_BOUND);
            // A deadline at the type's edge saturates instead of wrapping.
            assert_eq!(
                Bounds::for_trial(u64::MAX).envelope,
                Duration::from_millis(u64::MAX)
            );
        }

        #[test]
        fn the_probe_and_the_run_use_exactly_the_production_argv_and_close_descriptors() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let probe_argv = dir.path().join("probe-argv");
            let real_argv = dir.path().join("real-argv");
            let real_fds = dir.path().join("real-fds");
            // A descriptor the controller was handed and did not mark
            // close-on-exec: the sweep must keep it out of bwrap.
            let opened = std::fs::File::open("/dev/null").unwrap();
            use std::os::fd::AsRawFd;
            // SAFETY: F_DUPFD duplicates a descriptor this test owns into a
            // number well above the ones `ls` uses itself, with FD_CLOEXEC
            // clear (F_DUPFD, unlike F_DUPFD_CLOEXEC, leaves it so).
            let canary_fd = unsafe { libc::fcntl(opened.as_raw_fd(), libc::F_DUPFD, 40) };
            assert!(canary_fd >= 40);
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                &format!("printf '%s\\n' \"$@\" > '{}'; exit 0", probe_argv.display()),
                &format!(
                    "printf '%s\\n' \"$@\" > '{}'; /bin/ls /proc/self/fd > '{}'; echo row",
                    real_argv.display(),
                    real_fds.display()
                ),
            );
            assert_eq!(run_when_free(&bwrap), Ok(Launched::Exited(0)));
            let lines = |path: &Path| -> Vec<OsString> {
                std::fs::read_to_string(path)
                    .unwrap()
                    .lines()
                    .map(OsString::from)
                    .collect()
            };
            assert_eq!(
                lines(&probe_argv),
                command(&request(), &os(&["/usr/bin/true"]))
            );
            assert_eq!(lines(&real_argv), command(&request(), &inner()));
            let open: Vec<i32> = std::fs::read_to_string(&real_fds)
                .unwrap()
                .lines()
                .filter_map(|l| l.parse().ok())
                .collect();
            assert!(
                !open.contains(&canary_fd),
                "fd {canary_fd} leaked: {open:?}"
            );
            assert!(
                open.iter().all(|fd| *fd <= 3),
                "stray descriptors: {open:?}"
            );
            // SAFETY: closing the duplicate made above.
            unsafe { libc::close(canary_fd) };
        }

        /// The child half of the interrupt test: only does anything when the
        /// parent test re-executes this binary for it.
        #[test]
        fn interrupt_child() {
            let (Some(bwrap), Some(result)) = (
                std::env::var_os("ISOLATION_TEST_BWRAP"),
                std::env::var_os("ISOLATION_TEST_RESULT"),
            ) else {
                return;
            };
            let outcome = run(Path::new(&bwrap));
            std::fs::write(result, format!("{outcome:?}")).unwrap();
        }

        /// The child half of the lock test: holds the process's standard
        /// output lock on its main thread, as `main` did across the whole
        /// trial, and drives the controller. Only acts when the parent test
        /// re-executes this binary for it.
        #[test]
        fn lock_holding_child() {
            let (Some(bwrap), Some(result)) = (
                std::env::var_os("ISOLATION_TEST_BWRAP"),
                std::env::var_os("ISOLATION_TEST_RESULT"),
            ) else {
                return;
            };
            let held = std::io::stdout().lock();
            let outcome = run(Path::new(&bwrap));
            drop(held);
            std::fs::write(result, format!("{outcome:?}")).unwrap();
        }

        #[test]
        fn a_controller_holding_its_own_stdout_lock_still_delivers_the_row_and_exits() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let result = dir.path().join("result");
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                "echo '{\"schema_version\":2}'; exit 1",
            );
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "isolation::tests::controller::lock_holding_child",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("ISOLATION_TEST_BWRAP", &bwrap)
                .env("ISOLATION_TEST_RESULT", &result)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            // A real deadline, not a sleep used as synchronization: the child
            // either exits with its verdict or is killed and the test fails.
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("the controller outlived its row while holding the stdout lock");
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            assert!(status.success(), "the child failed: {status:?}");
            assert_eq!(std::fs::read_to_string(&result).unwrap(), "Ok(Exited(1))");
        }

        #[test]
        fn an_interrupt_before_any_runner_kills_bwrap_and_is_reported() {
            let _scope = crate::test_support::trial_scope();
            let dir = tempfile::tempdir().unwrap();
            let started = dir.path().join("started");
            let pid_file = dir.path().join("bwrap-pid");
            let result = dir.path().join("result");
            let bwrap = fake(
                dir.path(),
                "bubblewrap 0.11.1",
                "exit 0",
                &format!(
                    "echo $$ > '{}'; touch '{}'; exec /bin/sleep 60",
                    pid_file.display(),
                    started.display()
                ),
            );
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "isolation::tests::controller::interrupt_child",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("ISOLATION_TEST_BWRAP", &bwrap)
                .env("ISOLATION_TEST_RESULT", &result)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            // The stand-in writes `started` only after the controller has
            // registered its signal handlers and spawned it.
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while !started.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the stand-in never started"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            let bwrap_pid: i32 = std::fs::read_to_string(&pid_file)
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            // SAFETY: plain kill(2) on the child this test spawned.
            unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
            assert!(child.wait().unwrap().success());
            assert_eq!(std::fs::read_to_string(&result).unwrap(), "Ok(Interrupted)");
            // The stand-in bwrap did not outlive its controller's verdict.
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            // SAFETY: signal 0 only probes for existence.
            while unsafe { libc::kill(bwrap_pid, 0) } == 0
                && !std::fs::read_to_string(format!("/proc/{bwrap_pid}/stat"))
                    .map_or(true, |s| s.contains(") Z "))
            {
                assert!(std::time::Instant::now() < deadline, "bwrap survived");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn this_platform_refuses_by_name_before_looking_for_bwrap() {
        let request = Request {
            swarmail_bin: PathBuf::from("/x/swarmail"),
            runner: PathBuf::from("/x/runner"),
            agent: None,
            expose: Vec::new(),
        };
        let outcome = run(
            &request,
            &[],
            Path::new("/definitely/not/bwrap"),
            Bounds::for_trial(1),
        );
        assert_eq!(
            outcome.unwrap_err().to_string(),
            "boundary_unavailable:platform"
        );
    }
}
