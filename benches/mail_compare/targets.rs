//! Owned targets (spec R4): one owner launches each server in its own
//! process group with a closed environment and private scratch, proves
//! protocol readiness within a deadline and that every declared listener
//! belongs to that group — and, for a launch that declares its listeners,
//! that the group listens on exactly that set — and on every exit path
//! signals, reaps and verifies that the group and its listeners are gone. One cleanup worker per launch owns the
//! child from spawn to disposal and outlives a cancelled owner (proposed
//! decision 0093 §7). It never touches a process or endpoint it did not
//! create.

use crate::protocol::{ConfigFile, ListenerRole, TargetKind};
use crate::wire::{Http, Pop3, WireError};
use serde::Serialize;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// The startup line Swarmail prints is read at most this far; anything
/// after it is drained unread.
const STARTUP_LINE_MAX: u64 = 4096;

#[derive(Debug, Clone)]
pub struct Limits {
    pub startup: Duration,
    pub cleanup: Duration,
    pub request: Duration,
    pub max_response_bytes: u64,
    pub open_files: u64,
}

/// How a target is launched: Swarmail's argv is fixed here (it is the
/// candidate); a competitor's comes from its verified lock entry.
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub kind: TargetKind,
    /// The program that is exec'd: the executable itself, or a runtime's
    /// launcher (`bin/java`) when `extra.artifact` names the jar.
    pub binary: PathBuf,
    /// The digest the bytes must have; `None` only for the candidate,
    /// whose digest is recorded rather than pinned. It is the digest of
    /// `extra.artifact` when that is set, else of `binary`.
    pub expected_sha256: Option<String>,
    pub template: Vec<String>,
    pub extra: LaunchExtra,
}

/// What lock version 4 adds to a launch (proposed decision 0130 §2). The
/// default is lock version 3's launch: two listeners, no environment, no
/// config file and no further checked members.
#[derive(Debug, Clone, Default)]
pub struct LaunchExtra {
    /// The declared listeners. Empty is lock 3's `[smtp, http]` with no
    /// exact-set proof; non-empty also proves the group listens on exactly
    /// these addresses.
    pub listeners: Vec<ListenerRole>,
    /// `[NAME, value]` environment entries; slots in values are substituted.
    pub env: Vec<[String; 2]>,
    /// Rendered into the owned scratch before spawn, removed with it.
    pub config: Option<ConfigFile>,
    /// The jar a runtime launch runs, substituted for `{artifact}`.
    pub artifact: Option<PathBuf>,
    /// Absolute path and sha256 of every further member that is checked at
    /// launch and again at seal.
    pub members: Vec<(PathBuf, String)>,
}

impl LaunchSpec {
    pub fn new(
        kind: TargetKind,
        binary: PathBuf,
        expected_sha256: Option<String>,
        template: Vec<String>,
    ) -> LaunchSpec {
        LaunchSpec {
            kind,
            binary,
            expected_sha256,
            template,
            extra: LaunchExtra::default(),
        }
    }

    /// A competitor's launch from its pin, of either lock version. A lock 4
    /// pin declares its listeners, environment and config file; a runtime
    /// pin launches the runtime's own program with the pinned jar as
    /// `{artifact}`, and every checked member is digested at launch and again
    /// at seal. A tree pin's checked members are relative to the executable's
    /// directory, a runtime's to the runtime's root.
    pub fn from_pin(
        kind: TargetKind,
        binary: PathBuf,
        runtime_dir: Option<PathBuf>,
        pin: &crate::protocol::Pin,
    ) -> Result<LaunchSpec, String> {
        let mut spec = LaunchSpec::new(
            kind,
            binary.clone(),
            Some(pin.artifact.executable_sha256.clone()),
            pin.launch.clone(),
        );
        if let Some(roles) = &pin.listeners {
            spec.extra.listeners = roles.clone();
            spec.extra.env = pin.launch_env.clone().unwrap_or_default();
            spec.extra.config = pin.config_file.clone();
        }
        match (&pin.runtime, runtime_dir) {
            (Some(rt), Some(dir)) => {
                spec.binary = dir.join("bin/java");
                spec.extra.artifact = Some(binary);
                spec.extra.members = rt
                    .checked_members
                    .iter()
                    .map(|m| (dir.join(&m.path), m.sha256.clone()))
                    .collect();
            }
            (Some(_), None) => {
                return Err(format!(
                    "--runtime {kind}=DIR is required: its pin needs a runtime"
                ));
            }
            (None, Some(_)) => {
                return Err(format!(
                    "--runtime {kind} is given but its pin has no runtime"
                ));
            }
            (None, None) => {
                if let Some(members) = &pin.artifact.checked_members {
                    let base = binary.parent().unwrap_or(Path::new("."));
                    spec.extra.members = members
                        .iter()
                        .map(|m| (base.join(&m.path), m.sha256.clone()))
                        .collect();
                }
            }
        }
        Ok(spec)
    }

    fn roles(&self) -> Vec<ListenerRole> {
        if self.extra.listeners.is_empty() {
            vec![ListenerRole::Smtp, ListenerRole::Http]
        } else {
            self.extra.listeners.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaunchFacts {
    pub target: TargetKind,
    pub executable_sha256: String,
    /// The template, not the expanded argv: expanded values carry private
    /// scratch paths and ports, which stay out of public records.
    pub argv_template: Vec<String>,
    pub ready_ns: u64,
    /// Every checked member's relative name and sha256 as launched.
    pub members: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleanupFacts {
    pub target: TargetKind,
    pub forced_kill: bool,
    pub reaped: bool,
    pub group_gone: bool,
    /// The conjunction of `listeners`; its serialized name is unchanged.
    pub listeners_closed: bool,
    /// Each declared listener's own proof.
    pub listeners: Vec<ListenerFact>,
    pub scratch_removed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ListenerFact {
    pub role: ListenerRole,
    pub closed: bool,
}

impl CleanupFacts {
    pub fn proved(&self) -> bool {
        self.reaped && self.group_gone && self.listeners_closed && self.scratch_removed
    }

    /// What a worker that failed, panicked or never finished establishes:
    /// nothing.
    pub fn unproved(target: TargetKind) -> CleanupFacts {
        CleanupFacts {
            target,
            forced_kill: false,
            reaped: false,
            group_gone: false,
            listeners_closed: false,
            listeners: Vec::new(),
            scratch_removed: false,
        }
    }
}

/// Every cleanup worker a run started. A worker outlives its owner: a
/// dropped `Owned` — an interrupted run, a cancelled launch — leaves its
/// worker to finish the whole cleanup, and whoever holds the registry
/// awaits it before reporting. Cancellation never proves cleanup by
/// itself; only a settled worker's facts do.
#[derive(Clone, Default)]
pub struct Keepers(Arc<Mutex<Vec<Worked>>>);

/// One registered worker and the target it cleans.
type Worked = (TargetKind, JoinHandle<CleanupFacts>);

impl Keepers {
    /// Await every worker registered so far, all within one `bound`. A
    /// worker that panicked, or is still running at the bound, is aborted
    /// (its group guard kills the group) and counted unproved.
    pub async fn settle(&self, bound: Duration) -> Vec<CleanupFacts> {
        let workers = std::mem::take(&mut *self.0.lock().expect("keepers lock"));
        let until = tokio::time::Instant::now() + bound;
        let mut out = Vec::with_capacity(workers.len());
        for (kind, mut worker) in workers {
            match tokio::time::timeout_at(until, &mut worker).await {
                Ok(Ok(facts)) => out.push(facts),
                Ok(Err(_)) => out.push(CleanupFacts::unproved(kind)),
                Err(_) => {
                    worker.abort();
                    let _ = worker.await;
                    out.push(CleanupFacts::unproved(kind));
                }
            }
        }
        out
    }
}

/// Whether a listener is held by the owned group (spec R4: corroborate
/// both binds). The reserved port is released before the child binds it,
/// so a foreign process can win the gap; readiness alone cannot tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bind {
    /// Every listening socket that accepts at the address is held by a
    /// member of the group, and there is at least one.
    Owned,
    /// Something listens there, and no group member holds any of it.
    Foreign,
    /// Group members hold some of the listening sockets, others not.
    Ambiguous,
    /// Nothing listens there.
    Absent,
    /// Ownership cannot be read on this platform or right now.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    Binary(&'static str),
    DigestMismatch,
    /// A checked member (a runtime's `java` or `libjvm.so`, an unpacked
    /// sibling) is not the bytes the lock pinned.
    MemberDigestMismatch,
    /// The launch template names a slot the declared listeners do not
    /// provide, or omits a declared listener.
    Template,
    Scratch,
    Spawn,
    /// The ready group listens on an address no declaration names (a
    /// tool's fixed default port); it was cleaned up, and the facts say
    /// whether that cleanup is proved.
    UndeclaredListener(CleanupFacts),
    /// Readiness failed; the partial start was still cleaned up, and the
    /// facts say whether that cleanup is proved.
    NotReady(&'static str, CleanupFacts),
}

/// What the owner and its worker share. The drain handle and the
/// listener addresses arrive during startup; the worker takes them when it
/// cleans up, so a cancelled launch still disposes of both.
#[derive(Default)]
struct Shared {
    addrs: Vec<(ListenerRole, SocketAddr)>,
    drain: Option<JoinHandle<()>>,
    /// A test seam: awaited by the worker after it is told to stop and
    /// before it cleans, so a test can hold cleanup or fail it.
    #[cfg(test)]
    hook: Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>>,
}

pub struct Owned {
    pub kind: TargetKind,
    pub smtp: SocketAddr,
    /// The HTTP listener; port 0 for a target that is read over POP3.
    pub http_addr: SocketAddr,
    pub http: Http,
    /// GreenMail's read path (proposed decision 0130 §4).
    pub pop3: Option<Pop3>,
    pub facts: LaunchFacts,
    pgid: i32,
    limits: Limits,
    shared: Arc<Mutex<Shared>>,
    /// Set by the worker when the leader exits on its own.
    exited: Arc<AtomicBool>,
    /// Dropping or sending this tells the worker to clean up.
    stop: Option<oneshot::Sender<()>>,
    done: Option<oneshot::Receiver<CleanupFacts>>,
    settled: Option<CleanupFacts>,
}

pub use crate::protocol::sha256_hex;

/// Loopback ports the OS just handed out, all held until every one is
/// reserved so no two launch roles can share one. The listeners are dropped
/// before the child binds, so a foreign bind can win the gap; that loss
/// shows up as a readiness failure, never as measuring someone else's
/// server, because readiness also requires the owned child to be alive.
fn reserve_ports(n: usize) -> Option<Vec<SocketAddr>> {
    let held: Vec<TcpListener> = (0..n)
        .map(|_| TcpListener::bind("127.0.0.1:0").ok())
        .collect::<Option<_>>()?;
    held.iter().map(|l| l.local_addr().ok()).collect()
}

fn private_dir(path: &Path) -> bool {
    std::fs::DirBuilder::new().mode(0o700).create(path).is_ok()
}

/// Substitute every slot of a lock 4 text: one address and one port per
/// declared role, the owned data directory and scratch, the launched
/// artifact and the rendered config file.
pub fn substitute(
    text: &str,
    roles: &[(ListenerRole, String)],
    dirs: (&Path, &Path),
    artifact: Option<&Path>,
    config_path: Option<&Path>,
) -> String {
    let mut out = text.to_owned();
    for (escape, mark) in crate::protocol::BRACE_ESCAPES {
        out = out.replace(escape, mark);
    }
    for (role, addr) in roles {
        let port = addr.rsplit(':').next().unwrap_or_default();
        out = out
            .replace(&role.addr_slot(), addr)
            .replace(&role.port_slot(), port);
    }
    out = out
        .replace("{data_dir}", &dirs.0.to_string_lossy())
        .replace("{scratch}", &dirs.1.to_string_lossy());
    if let Some(a) = artifact {
        out = out.replace("{artifact}", &a.to_string_lossy());
    }
    if let Some(c) = config_path {
        out = out.replace("{config_path}", &c.to_string_lossy());
    }
    for (escape, mark) in crate::protocol::BRACE_ESCAPES {
        out = out.replace(mark, &escape[..1]);
    }
    out
}

/// A launch member read and digested; unreadable is a refusal, not a pass.
fn digest_of(path: &Path) -> Result<String, LaunchError> {
    let bytes = std::fs::read(path).map_err(|_| LaunchError::Binary("unreadable"))?;
    Ok(sha256_hex(&bytes))
}

macro_rules! bind_whys {
    ($($role:literal),*) => {
        fn bind_why(role: ListenerRole, bind: Bind) -> &'static str {
            match (role.name(), bind) {
                $(
                    ($role, Bind::Foreign) => concat!($role, " bind is foreign"),
                    ($role, Bind::Ambiguous) => concat!($role, " bind is ambiguous"),
                    ($role, Bind::Absent) => concat!($role, " bind is absent"),
                    ($role, Bind::Unknown) => concat!($role, " bind ownership unknown"),
                )*
                _ => "bind is not owned",
            }
        }
    };
}
bind_whys!("smtp", "http", "pop3", "imap", "ui");

impl Owned {
    /// Launch `spec` with scratch at `scratch` (which must not exist).
    pub async fn launch(
        spec: &LaunchSpec,
        scratch: &Path,
        limits: &Limits,
        clock: &crate::wire::Clock,
        keepers: &Keepers,
    ) -> Result<Owned, LaunchError> {
        let roles = spec.roles();
        if !spec.extra.listeners.is_empty() {
            // The declared listeners and the template must agree before a
            // process starts: an undeclared slot or an unused listener
            // would leave a port the driver does not own.
            let mut why = Vec::new();
            crate::protocol::launch_checks(
                &mut why,
                spec.kind,
                &roles,
                &spec.template,
                &spec.extra.env,
                spec.extra.config.as_ref(),
            );
            if !why.is_empty() {
                return Err(LaunchError::Template);
            }
        }
        // Absolute paths only past this point: a relative program or scratch
        // path would resolve against the child's working directory.
        let binary =
            std::fs::canonicalize(&spec.binary).map_err(|_| LaunchError::Binary("unreadable"))?;
        let mode = std::fs::metadata(&binary).map_err(|_| LaunchError::Binary("unreadable"))?;
        if mode.permissions().mode() & 0o111 == 0 {
            return Err(LaunchError::Binary("not executable"));
        }
        let artifact = match &spec.extra.artifact {
            Some(a) => {
                Some(std::fs::canonicalize(a).map_err(|_| LaunchError::Binary("unreadable"))?)
            }
            None => None,
        };
        let digest = digest_of(artifact.as_ref().unwrap_or(&binary))?;
        if spec
            .expected_sha256
            .as_deref()
            .is_some_and(|want| want != digest)
        {
            return Err(LaunchError::DigestMismatch);
        }
        let mut members = Vec::new();
        for (path, want) in &spec.extra.members {
            let have = digest_of(path)?;
            if have != *want {
                return Err(LaunchError::MemberDigestMismatch);
            }
            members.push((path.to_string_lossy().into_owned(), have));
        }
        if !private_dir(scratch) {
            return Err(LaunchError::Scratch);
        }
        let Ok(scratch) = std::fs::canonicalize(scratch) else {
            let _ = std::fs::remove_dir(scratch);
            return Err(LaunchError::Scratch);
        };
        let scratch = scratch.as_path();
        for leaf in ["home", "tmp", "config", "cache", "data", "state"] {
            if !private_dir(&scratch.join(leaf)) {
                let _ = std::fs::remove_dir_all(scratch);
                return Err(LaunchError::Scratch);
            }
        }
        // One reserved loopback port per declared role; Swarmail binds its
        // own ports and reports them on stdout.
        let slots: Vec<(ListenerRole, String)> = if spec.kind == TargetKind::Swarmail {
            roles
                .iter()
                .map(|r| (*r, "127.0.0.1:0".to_owned()))
                .collect()
        } else {
            match reserve_ports(roles.len()) {
                Some(addrs) => roles
                    .iter()
                    .zip(addrs)
                    .map(|(r, a)| (*r, a.to_string()))
                    .collect(),
                None => {
                    let _ = std::fs::remove_dir_all(scratch);
                    return Err(LaunchError::Spawn);
                }
            }
        };
        let data_dir = scratch.join("data");
        let config_path = match &spec.extra.config {
            Some(c) => {
                let path = scratch.join("config").join(&c.name);
                let text = c
                    .lines
                    .iter()
                    .map(|l| {
                        substitute(
                            l,
                            &slots,
                            (&data_dir, scratch),
                            artifact.as_deref(),
                            Some(&path),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let written = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        f.write_all(text.as_bytes())?;
                        f.write_all(b"\n")
                    });
                if written.is_err() {
                    let _ = std::fs::remove_dir_all(scratch);
                    return Err(LaunchError::Scratch);
                }
                Some(path)
            }
            None => None,
        };
        let sub = |text: &str| {
            substitute(
                text,
                &slots,
                (&data_dir, scratch),
                artifact.as_deref(),
                config_path.as_deref(),
            )
        };
        let argv: Vec<String> = spec.template.iter().map(|a| sub(a)).collect();
        let mut cmd = Command::new(&binary);
        cmd.args(&argv)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C")
            .env("HOME", scratch.join("home"))
            .env("TMPDIR", scratch.join("tmp"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .env("XDG_DATA_HOME", scratch.join("data"))
            .env("XDG_STATE_HOME", scratch.join("state"));
        for [name, value] in &spec.extra.env {
            cmd.env(name, sub(value));
        }
        cmd.current_dir(scratch)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(if spec.kind == TargetKind::Swarmail {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .process_group(0)
            .kill_on_drop(true);
        let nofile = limits.open_files;
        // SAFETY: setrlimit is async-signal-safe and touches only the
        // child's own limits between fork and exec.
        unsafe {
            cmd.pre_exec(move || {
                let lim = libc::rlimit {
                    rlim_cur: nofile as libc::rlim_t,
                    rlim_max: nofile as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &lim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                let _ = std::fs::remove_dir_all(scratch);
                return Err(LaunchError::Spawn);
            }
        };
        let pgid = child.id().map(|p| p as i32).unwrap_or(0);
        let stdout = child.stdout.take();
        // From here the worker owns the child: every later exit of this
        // function, its cancellation included, ends in the worker's cleanup.
        let shared = Arc::new(Mutex::new(Shared::default()));
        let exited = Arc::new(AtomicBool::new(false));
        let (stop_tx, stop_rx) = oneshot::channel();
        let (done_tx, done_rx) = oneshot::channel();
        let worker = tokio::spawn(keep(
            Worker {
                kind: spec.kind,
                pgid,
                scratch: scratch.to_owned(),
                limits: limits.clone(),
                shared: shared.clone(),
                exited: exited.clone(),
            },
            child,
            stop_rx,
            done_tx,
        ));
        keepers
            .0
            .lock()
            .expect("keepers lock")
            .push((spec.kind, worker));
        let mut owned = Owned {
            kind: spec.kind,
            smtp: "127.0.0.1:0".parse().expect("literal"),
            http_addr: "127.0.0.1:0".parse().expect("literal"),
            http: Http::new(
                "127.0.0.1:0".parse().expect("literal"),
                limits.request,
                limits.max_response_bytes,
            ),
            pop3: None,
            facts: LaunchFacts {
                target: spec.kind,
                executable_sha256: digest,
                argv_template: spec.template.clone(),
                ready_ns: 0,
                members,
            },
            pgid,
            limits: limits.clone(),
            shared,
            exited,
            stop: Some(stop_tx),
            done: Some(done_rx),
            settled: None,
        };
        let deadline = Instant::now() + limits.startup;
        let declared: Option<Vec<(ListenerRole, SocketAddr)>> = match (spec.kind, stdout) {
            (TargetKind::Swarmail, Some(out)) => {
                let (addrs, pop3, drain) = read_startup(out, deadline).await;
                owned.shared.lock().expect("shared lock").drain = drain;
                addrs.map(|(s, h)| {
                    let mut v = vec![(ListenerRole::Smtp, s), (ListenerRole::Http, h)];
                    // The candidate also reports its POP3 address; it is a
                    // declared listener only when the launch declares it.
                    if let (true, Some(p)) = (roles.contains(&ListenerRole::Pop3), pop3) {
                        v.push((ListenerRole::Pop3, p));
                    }
                    v
                })
            }
            _ => slots
                .iter()
                .map(|(r, a)| a.parse().ok().map(|a| (*r, a)))
                .collect(),
        };
        let Some(declared) = declared else {
            let facts = owned.shutdown().await;
            return Err(LaunchError::NotReady("no listener addresses", facts));
        };
        owned.shared.lock().expect("shared lock").addrs = declared.clone();
        let addr_of =
            |role: ListenerRole| declared.iter().find(|(r, _)| *r == role).map(|(_, a)| *a);
        let (Some(smtp), http) = (addr_of(ListenerRole::Smtp), addr_of(ListenerRole::Http)) else {
            let facts = owned.shutdown().await;
            return Err(LaunchError::NotReady("no listener addresses", facts));
        };
        owned.smtp = smtp;
        if let Some(http) = http {
            owned.http_addr = http;
            owned.http = Http::new(http, limits.request, limits.max_response_bytes);
        }
        if spec.kind == TargetKind::Greenmail {
            let Some(pop3) = addr_of(ListenerRole::Pop3) else {
                let facts = owned.shutdown().await;
                return Err(LaunchError::NotReady("no listener addresses", facts));
            };
            owned.pop3 = Some(Pop3::new(pop3, limits.request, limits.max_response_bytes));
        }
        if let Err(why) = owned.ready(deadline).await {
            let facts = owned.shutdown().await;
            return Err(LaunchError::NotReady(why, facts));
        }
        // Readiness answered, but a foreign listener that won the gap would
        // answer too: every declared bind must be held by the owned group.
        for (role, addr) in &declared {
            let bind = bind_within(pgid, *addr, deadline).await;
            if bind != Bind::Owned {
                let facts = owned.shutdown().await;
                return Err(LaunchError::NotReady(bind_why(*role, bind), facts));
            }
        }
        // A launch that declares its listeners owns exactly those: a tool's
        // fixed default port would otherwise stay bound, collide with a
        // parallel run and still read as a closed cleanup.
        if !spec.extra.listeners.is_empty() {
            let want: std::collections::BTreeSet<SocketAddr> =
                declared.iter().map(|(_, a)| *a).collect();
            match group_listeners(pgid) {
                Some(have) if have == want => {}
                Some(_) => {
                    let facts = owned.shutdown().await;
                    return Err(LaunchError::UndeclaredListener(facts));
                }
                None => {
                    let facts = owned.shutdown().await;
                    return Err(LaunchError::NotReady("listener set unknown", facts));
                }
            }
        }
        owned.facts.ready_ns = clock.now_ns();
        Ok(owned)
    }

    /// A detached target for decoder tests: no child, no scratch, only the
    /// read path pointed at a scripted responder. `shutdown` proves nothing.
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn fixture(
        kind: TargetKind,
        http: SocketAddr,
        pop3: Option<SocketAddr>,
        limits: &Limits,
    ) -> Owned {
        Owned {
            kind,
            smtp: "127.0.0.1:0".parse().expect("literal"),
            http_addr: http,
            http: Http::new(http, limits.request, limits.max_response_bytes),
            pop3: pop3.map(|a| Pop3::new(a, limits.request, limits.max_response_bytes)),
            facts: LaunchFacts {
                target: kind,
                executable_sha256: String::new(),
                argv_template: Vec::new(),
                ready_ns: 0,
                members: Vec::new(),
            },
            pgid: 0,
            limits: limits.clone(),
            shared: Arc::default(),
            exited: Arc::default(),
            stop: None,
            done: None,
            settled: None,
        }
    }

    /// The leader's pid, which is also its process group id. Read by the
    /// lifecycle tests; the driver itself never needs it.
    #[allow(dead_code)]
    pub fn pid(&self) -> i32 {
        self.pgid
    }

    /// Install the worker's test hook: it runs after the worker is told to
    /// stop and before it cleans up.
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn on_cleanup(&self, hook: impl std::future::Future<Output = ()> + Send + 'static) {
        self.shared.lock().expect("shared lock").hook = Some(Box::pin(hook));
    }

    /// Swap the output drain for a task that panics, so the worker's join
    /// of it fails during cleanup. The real drain is aborted and awaited
    /// first: only the nested failure is injected, never a leaked task.
    #[cfg(test)]
    #[allow(dead_code)]
    pub async fn fail_drain(&self) {
        let real = {
            let mut shared = self.shared.lock().expect("shared lock");
            let failing = tokio::spawn(async { panic!("injected drain failure (expected)") });
            shared.drain.replace(failing)
        };
        if let Some(real) = real {
            real.abort();
            let _ = real.await;
        }
    }

    /// Protocol readiness: the owned child still runs, the SMTP banner is
    /// `220` and the read path answers — the HTTP listing with `200`, or for
    /// GreenMail the POP3 `+OK` greeting — all before `deadline`.
    async fn ready(&mut self, deadline: Instant) -> Result<(), &'static str> {
        loop {
            if self.exited.load(Ordering::SeqCst) {
                return Err("exited before ready");
            }
            let banner = crate::wire::Smtp::open(self.smtp, self.limits.request).await;
            let read_ok = match &self.pop3 {
                Some(pop3) => pop3.greeting().await.is_ok(),
                None => matches!(self.http.get(&self.list_path(0, 1)).await, Ok((200, _))),
            };
            if let (Ok(s), true) = (banner, read_ok) {
                s.quit().await;
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("readiness deadline");
            }
            // A bounded readiness probe, not synchronization on a guess:
            // the deadline above is what fails the launch.
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Requests made on the read path so far: HTTP requests, or POP3
    /// commands for GreenMail.
    pub fn requests(&self) -> u64 {
        self.http.requests() + self.pop3.as_ref().map_or(0, Pop3::requests)
    }

    // The competitor mappings below follow the captured API documents in
    // docs/evals/sources/ where those agree with the pinned executables,
    // and the executables where they do not; every disagreement is listed
    // in docs/evals/README.md and protocol.json's unresolved list. The
    // MailSlurper, smtp4dev, Inbucket and Alkemio shapes below were authored
    // from the upstream documentation as recalled. Two are now checked
    // against real bytes in unmeasured tests: the Inbucket shape, read back
    // from the pinned executable by round-trip, hostile-body and cap tests,
    // and the GreenMail POP3 reader, by a labelled driver proof against the
    // real jar under a JRE. The MailSlurper, smtp4dev and Alkemio shapes are
    // still UNVERIFIED, because no executable of theirs has stored and
    // served a message (MailSlurper's supplied binary cannot start its SQLite
    // storage, smtp4dev's archive has no license document and Alkemio's
    // binary has no musl loader); those rows stay pending until their real
    // tests pass. A wrong shape fails loudly (malformed or incomplete),
    // never as a shorter success.

    fn list_path(&self, start: u32, limit: u32) -> String {
        let page = start / limit.max(1) + 1;
        match self.kind {
            TargetKind::Swarmail => format!(
                "/api/v1/inboxes/default/messages?subject=mail-compare&limit={limit}&offset={start}"
            ),
            TargetKind::Mailpit => format!("/api/v1/messages?start={start}&limit={limit}"),
            TargetKind::Mailhog => format!("/api/v2/messages?start={start}&limit={limit}"),
            TargetKind::Mailslurper | TargetKind::AlkemioMailslurper => {
                format!("/mail?pageNumber={page}")
            }
            TargetKind::Smtp4dev => {
                format!("/api/messages?pageNumber={page}&pageSize={limit}")
            }
            TargetKind::Inbucket => format!("/api/v1/mailbox/{}", mailbox()),
            // Read over POP3, never over HTTP; this path is the readiness
            // probe's, which a POP3 target does not use.
            TargetKind::Greenmail => String::new(),
        }
    }

    fn find_path(&self, token: &str, start: u32, limit: u32) -> String {
        let q = crate::wire::query_value(token);
        match self.kind {
            TargetKind::Swarmail => {
                format!("/api/v1/inboxes/default/messages?subject={q}&limit={limit}&offset={start}")
            }
            TargetKind::Mailpit => format!("/api/v1/search?query={q}&start={start}&limit={limit}"),
            TargetKind::Mailhog => {
                format!("/api/v2/search?kind=containing&query={q}&start={start}&limit={limit}")
            }
            // No verified server search: these are scanned completely and
            // filtered on the client (see `find`).
            _ => self.list_path(start, limit),
        }
    }

    /// One page: the count of everything the request matches (not the
    /// mailbox size) and `[(server id, subject)]`. See [`decode_page`].
    fn page(&self, v: &serde_json::Value, start: u32) -> Option<(u64, Vec<(String, String)>)> {
        decode_page(self.kind, v, start)
    }

    /// Page `path(start, PAGE)` to completion: `[(server id, subject)]`.
    /// Complete means the pages reached the matching count with every id
    /// distinct, every page before the last full and the count unchanged.
    /// A page that does not parse, a repeated id, a moving count, a short
    /// page before the count or more pages than `max_pages` is an error,
    /// never a shorter success. A complete walk is not a snapshot: churn
    /// that keeps the count can pass, so exact-count checks run quiescent.
    async fn scan(
        &self,
        path: impl Fn(u32, u32) -> String,
        max_pages: u32,
    ) -> Result<Vec<(String, String)>, WireError> {
        let size = page_size(self.kind);
        let mut out: Vec<(String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut expected = None;
        for _ in 0..max_pages {
            // The next offset is what arrived, never a fixed stride.
            let start = u32::try_from(out.len()).map_err(|_| WireError::Oversized)?;
            let (status, v) = self.http.get_json(&path(start, size)).await?;
            let (total, items) = (status == 200)
                .then(|| self.page(&v, start))
                .flatten()
                .ok_or(WireError::Malformed)?;
            if *expected.get_or_insert(total) != total {
                return Err(WireError::Incomplete);
            }
            let got = items.len();
            for (id, subject) in items {
                if !seen.insert(id.clone()) {
                    return Err(WireError::Incomplete);
                }
                out.push((id, subject));
            }
            match (out.len() as u64).cmp(&total) {
                std::cmp::Ordering::Equal => return Ok(out),
                std::cmp::Ordering::Greater => return Err(WireError::Incomplete),
                std::cmp::Ordering::Less if got < size as usize => {
                    return Err(WireError::Incomplete);
                }
                std::cmp::Ordering::Less => {}
            }
        }
        Err(WireError::Oversized)
    }

    /// Every message the server shows, paged to completion, as
    /// `(token or "", server id)`. GreenMail is read over POP3 in a fresh
    /// session; `max_pages` bounds HTTP paging only.
    pub async fn list_all(&self, max_pages: u32) -> Result<Vec<(String, String)>, WireError> {
        if let Some(pop3) = &self.pop3 {
            let mut out = Vec::new();
            for (uid, head) in pop3.list().await? {
                out.push((token_of(&subject_of_headers(&head)?), uid));
            }
            return Ok(out);
        }
        let all = self.scan(|s, l| self.list_path(s, l), max_pages).await?;
        Ok(all
            .into_iter()
            .map(|(id, subject)| (token_of(&subject), id))
            .collect())
    }

    /// One complete search for `token`: every hit, related or not, so a
    /// target that answers with unrelated mail is recorded doing so. A
    /// target without a verified server search is scanned completely and
    /// filtered on the client by token (labelled per row), so only matching
    /// mail is returned; completeness is the scan's.
    pub async fn find(&self, token: &str) -> Result<Vec<String>, WireError> {
        const FIND_MAX_PAGES: u32 = 4;
        if !has_server_search(self.kind) {
            let all = self
                .list_all(crate::protocol::MAX_MESSAGES / page_size(self.kind) + 1)
                .await?;
            return Ok(all
                .into_iter()
                .filter(|(t, _)| t == token)
                .map(|(t, _)| t)
                .collect());
        }
        let hits = self
            .scan(|s, l| self.find_path(token, s, l), FIND_MAX_PAGES)
            .await?;
        Ok(hits
            .into_iter()
            .map(|(_, subject)| token_of(&subject))
            .collect())
    }

    /// Swarmail's own blocking wait. `Ok(None)` is its timeout answer.
    pub async fn native_wait(
        &self,
        token: &str,
        deadline: Duration,
    ) -> Result<Option<Vec<String>>, WireError> {
        if self.kind != TargetKind::Swarmail {
            return Err(WireError::Malformed);
        }
        let path = format!(
            "/api/v1/inboxes/default/await?subject={}&count=1&timeout_ms={}",
            crate::wire::query_value(token),
            deadline.as_millis()
        );
        let (status, body) = self
            .http
            .get_within(&path, Some(deadline + self.limits.request))
            .await?;
        let v: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| WireError::Malformed)?;
        match status {
            200 => self
                .page(&v, 0)
                .map(|(_, items)| Some(items.into_iter().map(|(_, s)| token_of(&s)).collect()))
                .ok_or(WireError::Malformed),
            408 => Ok(None),
            _ => Err(WireError::Malformed),
        }
    }

    /// Subject, text and HTML of one stored message, for the untimed
    /// fidelity check. Swarmail's parts come from its ingest extraction;
    /// a competitor's from the raw message it stored, parsed by the
    /// crate's own MIME parser — one parser for one corpus. Mailpit's
    /// served `Text` of an HTML-only message is its own derivation, not a
    /// stored part, so its documented raw endpoint is read instead.
    pub async fn fetch(&self, id: &str) -> Result<Fetched, WireError> {
        let id_q = crate::wire::query_value(id);
        match self.kind {
            TargetKind::Swarmail => {
                let (status, v) = self
                    .http
                    .get_json(&format!("/api/v1/messages/{id_q}"))
                    .await?;
                if status != 200 {
                    return Err(WireError::Malformed);
                }
                let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_owned);
                Ok(Fetched {
                    subject: s("subject"),
                    text: s("text"),
                    html: s("html"),
                })
            }
            TargetKind::Mailpit => {
                let (status, raw) = self
                    .http
                    .get(&format!("/api/v1/message/{id_q}/raw"))
                    .await?;
                if status != 200 {
                    return Err(WireError::Malformed);
                }
                parse_raw(&raw)
            }
            TargetKind::Mailhog => {
                let (status, v) = self
                    .http
                    .get_json(&format!("/api/v1/messages/{id_q}"))
                    .await?;
                // MailHog answers an unknown id with `200 null`.
                if status != 200 || v.is_null() {
                    return Err(WireError::Malformed);
                }
                let raw = v
                    .get("Raw")
                    .and_then(|r| r.get("Data"))
                    .and_then(|d| d.as_str())
                    .ok_or(WireError::Malformed)?;
                parse_raw(raw.as_bytes())
            }
            TargetKind::Smtp4dev => {
                let (status, raw) = self.http.get(&format!("/api/messages/{id_q}/raw")).await?;
                if status != 200 {
                    return Err(WireError::Malformed);
                }
                parse_raw(&raw)
            }
            TargetKind::Inbucket => {
                let (status, raw) = self
                    .http
                    .get(&format!("/api/v1/mailbox/{}/{id_q}/source", mailbox()))
                    .await?;
                if status != 200 {
                    return Err(WireError::Malformed);
                }
                parse_raw(&raw)
            }
            TargetKind::Mailslurper | TargetKind::AlkemioMailslurper => {
                let (status, v) = self.http.get_json(&format!("/mail/{id_q}")).await?;
                if status != 200 {
                    return Err(WireError::Malformed);
                }
                // MailSlurper keeps the message's decoded fields, not its
                // raw bytes: the stored subject, content type, boundary and
                // body are re-wrapped and read by the same MIME parser.
                let s = |k: &str| v.get(k).and_then(|x| x.as_str());
                let subject = s("subject").ok_or(WireError::Malformed)?;
                let body = s("body").ok_or(WireError::Malformed)?;
                let content_type = s("contentType").unwrap_or("text/plain");
                let boundary = s("boundary")
                    .filter(|b| !b.is_empty())
                    .map(|b| format!("; boundary=\"{b}\""))
                    .unwrap_or_default();
                parse_raw(
                    format!(
                        "Subject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: {content_type}{boundary}\r\n\r\n{body}"
                    )
                    .as_bytes(),
                )
            }
            TargetKind::Greenmail => {
                let pop3 = self.pop3.as_ref().ok_or(WireError::Malformed)?;
                parse_raw(&pop3.retrieve(id).await?)
            }
        }
    }

    /// Tell the worker to clean up and await its facts: signal the group,
    /// reap the leader, prove the group and listeners are gone and remove
    /// the owned scratch. Idempotent, and safe to cancel: the worker
    /// carries on, and a later call (or the registry) still awaits it.
    pub async fn shutdown(&mut self) -> CleanupFacts {
        if let Some(facts) = &self.settled {
            return facts.clone();
        }
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let facts = match self.done.as_mut() {
            Some(done) => done
                .await
                .unwrap_or_else(|_| CleanupFacts::unproved(self.kind)),
            None => CleanupFacts::unproved(self.kind),
        };
        self.done = None;
        self.settled = Some(facts.clone());
        facts
    }
}

/// What a cleanup worker needs besides the child itself.
struct Worker {
    kind: TargetKind,
    pgid: i32,
    scratch: PathBuf,
    limits: Limits,
    shared: Arc<Mutex<Shared>>,
    exited: Arc<AtomicBool>,
}

/// Kills the group if the worker itself is dropped before it finishes —
/// the runtime shutting down under it. That is forced death: nothing is
/// reaped or proved, and the scratch stays for the operator.
struct GroupGuard(i32);

impl Drop for GroupGuard {
    fn drop(&mut self) {
        signal_group(self.0, libc::SIGKILL);
    }
}

/// The one worker that owns a launched child. It waits for the owner's
/// stop — sent, or implied by the owner being dropped — noting an early
/// exit of the leader meanwhile, then performs the whole cleanup and
/// reports it to the owner (if still listening) and to the registry.
async fn keep(
    w: Worker,
    mut child: Child,
    mut stop: oneshot::Receiver<()>,
    done: oneshot::Sender<CleanupFacts>,
) -> CleanupFacts {
    let mut guard = GroupGuard(w.pgid);
    let mut reaped = None;
    tokio::select! {
        _ = &mut stop => {}
        status = child.wait() => {
            w.exited.store(true, Ordering::SeqCst);
            reaped = Some(status.is_ok());
            let _ = stop.await;
        }
    }
    #[cfg(test)]
    {
        let hook = w.shared.lock().expect("shared lock").hook.take();
        if let Some(hook) = hook {
            hook.await;
        }
    }
    let facts = cleanup(&w, child, reaped).await;
    // The group was killed and awaited above; a later kill could only
    // reach a reused id.
    guard.0 = 0;
    let _ = done.send(facts.clone());
    facts
}

/// One composed budget, `limits.cleanup`, for the whole disposal: half of
/// it is the leader's grace after `SIGTERM`; the forced kill, the group's
/// disappearance, the output drain and the listener probes share what
/// remains. A step the budget cuts off is unproved, never skipped.
async fn cleanup(w: &Worker, mut child: Child, reaped_early: Option<bool>) -> CleanupFacts {
    let start = tokio::time::Instant::now();
    let grace = start + w.limits.cleanup / 2;
    let until = start + w.limits.cleanup;
    let (addrs, drain) = {
        let mut shared = w.shared.lock().expect("shared lock");
        (shared.addrs.clone(), shared.drain.take())
    };
    let mut facts = CleanupFacts {
        target: w.kind,
        forced_kill: false,
        reaped: reaped_early.unwrap_or(false),
        group_gone: false,
        listeners_closed: false,
        listeners: Vec::new(),
        scratch_removed: false,
    };
    if reaped_early.is_none() {
        signal_group(w.pgid, libc::SIGTERM);
        if let Ok(Ok(_)) = tokio::time::timeout_at(grace, child.wait()).await {
            facts.reaped = true;
        } else {
            facts.forced_kill = true;
            signal_group(w.pgid, libc::SIGKILL);
            facts.reaped = matches!(
                tokio::time::timeout_at(until, child.wait()).await,
                Ok(Ok(_))
            );
        }
    }
    // A member that outlived the leader still holds the group id, so
    // the id cannot have been reused: it is still ours to kill. With no
    // member left the kill finds nothing (ESRCH); reuse in between would
    // need the pid space to wrap inside that instant.
    signal_group(w.pgid, libc::SIGKILL);
    loop {
        facts.group_gone = !group_exists(w.pgid);
        if facts.group_gone || tokio::time::Instant::now() >= until {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // The drain ends at the pipe's EOF. One the budget cuts off is aborted
    // and awaited, never left behind; one that failed (a panic, a nested
    // join failure) proves nothing.
    if let Some(mut drain) = drain {
        let drained = match tokio::time::timeout_at(until, &mut drain).await {
            Ok(joined) => joined.is_ok(),
            Err(_) => {
                drain.abort();
                let _ = drain.await;
                false
            }
        };
        facts.reaped &= drained;
    }
    // Every declared listener is probed on its own, so a single one left
    // open is named and fails the conjunction.
    for (role, addr) in &addrs {
        facts.listeners.push(ListenerFact {
            role: *role,
            closed: refused(*addr, until).await,
        });
    }
    facts.listeners_closed = if addrs.is_empty() {
        // No address was ever learned, so none can be probed; the group
        // being gone is what shows no listener of ours survives.
        facts.group_gone
    } else {
        facts.listeners.iter().all(|l| l.closed)
    };
    facts.scratch_removed = std::fs::remove_dir_all(&w.scratch).is_ok() || !w.scratch.exists();
    facts
}

/// Whether the executable at `binary` still has the digest it had at
/// launch: rechecked before a run is sealed, so bytes replaced mid-run
/// never stand behind a sealed record.
pub fn unchanged(binary: &Path, launched_sha256: &str) -> bool {
    std::fs::canonicalize(binary)
        .and_then(std::fs::read)
        .is_ok_and(|bytes| sha256_hex(&bytes) == launched_sha256)
}

/// Whether a launch's executable and every checked member still have the
/// digests they had at launch: rechecked before a run is sealed.
pub fn spec_unchanged(spec: &LaunchSpec, launched: &LaunchFacts) -> bool {
    let measured = spec.extra.artifact.as_ref().unwrap_or(&spec.binary);
    unchanged(measured, &launched.executable_sha256)
        && spec.extra.members.iter().all(|(path, _)| {
            let name = path.to_string_lossy();
            launched
                .members
                .iter()
                .find(|(n, _)| *n == name)
                .is_some_and(|(_, sha)| unchanged(path, sha))
        })
        && launched.members.len() == spec.extra.members.len()
}

/// Every TCP address the group listens on (Linux: read from `/proc`): the
/// listening sockets whose inode a group member holds. Wildcard and
/// v4-mapped addresses are reported as they are, v4-mapped ones normalized
/// to the v4 address, so a listener on all interfaces never equals a
/// loopback declaration. `None` when the tables cannot be read.
pub fn group_listeners(pgid: i32) -> Option<std::collections::BTreeSet<SocketAddr>> {
    #[cfg(target_os = "linux")]
    {
        linux_group_listeners(pgid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pgid;
        None
    }
}

#[cfg(target_os = "linux")]
fn linux_group_listeners(pgid: i32) -> Option<std::collections::BTreeSet<SocketAddr>> {
    use std::collections::{BTreeMap, BTreeSet};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    let mut sockets: BTreeMap<String, SocketAddr> = BTreeMap::new();
    for (table, v6) in [("/proc/net/tcp", false), ("/proc/net/tcp6", true)] {
        let Ok(text) = std::fs::read_to_string(table) else {
            if v6 {
                continue; // no IPv6 stack, so no v6 listener
            }
            return None;
        };
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.split_whitespace().collect();
            let (Some(local), Some(state), Some(inode)) = (f.get(1), f.get(3), f.get(9)) else {
                return None;
            };
            if *state != "0A" {
                continue;
            }
            let (ip_hex, port_hex) = local.split_once(':')?;
            let port = u16::from_str_radix(port_hex, 16).ok()?;
            let mut bytes = Vec::with_capacity(16);
            for at in (0..ip_hex.len()).step_by(8) {
                let word = u32::from_str_radix(ip_hex.get(at..at + 8)?, 16).ok()?;
                bytes.extend_from_slice(&word.to_ne_bytes());
            }
            let ip = match (v6, bytes.len()) {
                (false, 4) => IpAddr::V4(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])),
                (true, 16) => {
                    let b: [u8; 16] = bytes.try_into().ok()?;
                    let v6 = Ipv6Addr::from(b);
                    v6.to_ipv4_mapped()
                        .map(IpAddr::V4)
                        .unwrap_or(IpAddr::V6(v6))
                }
                _ => return None,
            };
            sockets.insert(format!("socket:[{inode}]"), SocketAddr::new(ip, port));
        }
    }
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
        else {
            continue;
        };
        // A process that exits mid-walk has stopped being a member.
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let pgrp = stat
            .rsplit(')')
            .next()
            .and_then(|rest| rest.split_whitespace().nth(2))
            .and_then(|g| g.parse::<i32>().ok());
        if pgrp != Some(pgid) {
            continue;
        }
        // A member whose descriptors cannot be read leaves the set unknown.
        for fd in std::fs::read_dir(format!("/proc/{pid}/fd")).ok()?.flatten() {
            if let Ok(link) = std::fs::read_link(fd.path())
                && let Some(addr) = link.to_str().and_then(|l| sockets.get(l))
            {
                out.insert(*addr);
            }
        }
    }
    Some(out)
}

/// [`binds`], waiting for a listener that is not there yet. A server may
/// answer readiness on one listener before it binds another (GreenMail's REST
/// API starts after its mail servers). Only [`Bind::Absent`] is waited out,
/// until `deadline`; any holder, foreign or not, is reported at once.
pub async fn bind_within(pgid: i32, addr: SocketAddr, deadline: Instant) -> Bind {
    loop {
        let bind = binds(pgid, addr);
        if bind != Bind::Absent || Instant::now() >= deadline {
            return bind;
        }
        // A bounded probe: the deadline above is what ends the wait.
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Who holds the listening sockets that accept at `addr` (Linux: read from
/// `/proc`). Wildcard and v4-mapped listeners on the same port accept
/// there too, so they count.
pub fn binds(pgid: i32, addr: SocketAddr) -> Bind {
    #[cfg(target_os = "linux")]
    {
        linux_binds(pgid, addr)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (pgid, addr);
        Bind::Unknown
    }
}

#[cfg(target_os = "linux")]
fn linux_binds(pgid: i32, addr: SocketAddr) -> Bind {
    use std::collections::BTreeSet;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    const LISTEN: &str = "0A";
    let accepts = |ip: IpAddr| -> bool {
        let mapped = match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
            v4 => v4,
        };
        ip.is_unspecified() || mapped == addr.ip()
    };
    let mut wanted = BTreeSet::new();
    for (table, v6) in [("/proc/net/tcp", false), ("/proc/net/tcp6", true)] {
        let Ok(text) = std::fs::read_to_string(table) else {
            if v6 {
                continue; // no IPv6 stack, so no v6 listener
            }
            return Bind::Unknown;
        };
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.split_whitespace().collect();
            let (Some(local), Some(state), Some(inode)) = (f.get(1), f.get(3), f.get(9)) else {
                return Bind::Unknown;
            };
            let Some((ip_hex, port_hex)) = local.split_once(':') else {
                return Bind::Unknown;
            };
            if *state != LISTEN || u16::from_str_radix(port_hex, 16) != Ok(addr.port()) {
                continue;
            }
            // The kernel prints the address as host-order 32-bit words.
            let mut bytes = Vec::with_capacity(16);
            for at in (0..ip_hex.len()).step_by(8) {
                let Some(Ok(word)) = ip_hex.get(at..at + 8).map(|h| u32::from_str_radix(h, 16))
                else {
                    return Bind::Unknown;
                };
                bytes.extend_from_slice(&word.to_ne_bytes());
            }
            let ip = match (v6, bytes.len()) {
                (false, 4) => IpAddr::V4(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])),
                (true, 16) => {
                    let b: [u8; 16] = bytes.try_into().expect("sixteen bytes");
                    IpAddr::V6(Ipv6Addr::from(b))
                }
                _ => return Bind::Unknown,
            };
            if accepts(ip) {
                wanted.insert(format!("socket:[{inode}]"));
            }
        }
    }
    if wanted.is_empty() {
        return Bind::Absent;
    }
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return Bind::Unknown;
    };
    let mut held = BTreeSet::new();
    for entry in procs.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
        else {
            continue;
        };
        // A process that exits mid-walk has stopped being a member.
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let pgrp = stat
            .rsplit(')')
            .next()
            .and_then(|rest| rest.split_whitespace().nth(2))
            .and_then(|g| g.parse::<i32>().ok());
        if pgrp != Some(pgid) {
            continue;
        }
        let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
            return Bind::Unknown; // a member whose descriptors cannot be read
        };
        for fd in fds.flatten() {
            if let Ok(link) = std::fs::read_link(fd.path())
                && let Some(link) = link.to_str()
                && wanted.contains(link)
            {
                held.insert(link.to_owned());
            }
        }
    }
    match held.len() {
        0 => Bind::Foreign,
        n if n == wanted.len() => Bind::Owned,
        _ => Bind::Ambiguous,
    }
}

fn signal_group(pgid: i32, sig: i32) {
    if pgid > 1 {
        // SAFETY: kill(2) on the negative of our own child's group id.
        unsafe {
            libc::kill(-pgid, sig);
        }
    }
}

fn group_exists(pgid: i32) -> bool {
    // SAFETY: signal 0 only probes for existence.
    pgid > 1 && unsafe { libc::kill(-pgid, 0) } == 0
}

/// A listener counts as closed when a connect is refused by `until` — a
/// bound port someone else grabbed in the meantime reads as still open,
/// which fails cleanup rather than passing it.
async fn refused(addr: SocketAddr, until: tokio::time::Instant) -> bool {
    addr.port() == 0
        || matches!(
            tokio::time::timeout_at(until, tokio::net::TcpStream::connect(addr)).await,
            Ok(Err(_))
        )
}

/// The Swarmail startup line names the bound addresses; it is read with a
/// byte cap and a deadline, and the rest of stdout is drained unread.
async fn read_startup(
    out: tokio::process::ChildStdout,
    deadline: Instant,
) -> (
    Option<(SocketAddr, SocketAddr)>,
    Option<SocketAddr>,
    Option<tokio::task::JoinHandle<()>>,
) {
    let mut reader = BufReader::new(out.take(STARTUP_LINE_MAX));
    let mut line = String::new();
    let read = tokio::time::timeout_at(deadline.into(), reader.read_line(&mut line)).await;
    let mut rest = reader.into_inner().into_inner();
    let drain = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut rest, &mut tokio::io::sink()).await;
    });
    if !matches!(read, Ok(Ok(n)) if n > 0) {
        return (None, None, Some(drain));
    }
    let field = |name: &str| -> Option<SocketAddr> {
        line.split(name)
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    (
        field("smtp=").zip(field("http=")),
        field("pop3="),
        Some(drain),
    )
}

/// `"mail-compare <token> …"` → `<token>`; anything else → `""`.
pub fn token_of(subject: &str) -> String {
    let mut words = subject.split_whitespace();
    match (words.next(), words.next()) {
        (Some("mail-compare"), Some(t)) if t.starts_with("mc-") => t.to_owned(),
        _ => String::new(),
    }
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The mailbox the corpus's recipient reads from: the address's local part.
fn mailbox() -> &'static str {
    crate::wire::TO.split('@').next().unwrap_or_default()
}

/// Whether a target has a source-verified server search the driver uses.
/// Every other target is scanned completely and filtered on the client.
pub fn has_server_search(kind: TargetKind) -> bool {
    matches!(
        kind,
        TargetKind::Swarmail | TargetKind::Mailpit | TargetKind::Mailhog
    )
}

/// How many messages one listing page holds. A page that comes up short
/// before the count is an incomplete scan, so a wrong figure fails loudly.
pub fn page_size(kind: TargetKind) -> u32 {
    match kind {
        TargetKind::Mailslurper | TargetKind::AlkemioMailslurper => 50,
        _ => 250,
    }
}

/// A message id as the listing states it: a string or an integer.
fn id_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// One listing page, decoded: the count of everything the request matches
/// and `[(server id, subject)]`. Mailpit's search keeps the mailbox size in
/// `total` and the matches in `messages_count`; MailHog's `total` is the
/// match count whatever its Swagger says. Where the target echoes `start`
/// (or its page number) it must be the position asked for. A body that is
/// not exactly the documented shape, or whose items disagree with its own
/// count, is `None`: never a shorter success.
pub fn decode_page(
    kind: TargetKind,
    v: &serde_json::Value,
    start: u32,
) -> Option<(u64, Vec<(String, String)>)> {
    let size = page_size(kind);
    let page_number = u64::from(start / size) + 1;
    // (items, total, subject) per object-shaped target.
    let (list, total): (&Vec<serde_json::Value>, u64) = match kind {
        TargetKind::Swarmail => (v.get("emails")?.as_array()?, v.get("total")?.as_u64()?),
        TargetKind::Mailpit => {
            if v.get("start")?.as_u64()? != u64::from(start) {
                return None;
            }
            (
                v.get("messages")?.as_array()?,
                v.get("messages_count")?.as_u64()?,
            )
        }
        TargetKind::Mailhog => {
            if v.get("start")?.as_u64()? != u64::from(start) {
                return None;
            }
            (v.get("items")?.as_array()?, v.get("total")?.as_u64()?)
        }
        TargetKind::Mailslurper | TargetKind::AlkemioMailslurper => {
            if !start.is_multiple_of(size) {
                return None;
            }
            (
                v.get("mailItems")?.as_array()?,
                v.get("totalMailItems")?.as_u64()?,
            )
        }
        TargetKind::Smtp4dev => {
            if !start.is_multiple_of(size) || v.get("currentPage")?.as_u64()? != page_number {
                return None;
            }
            (v.get("results")?.as_array()?, v.get("rowCount")?.as_u64()?)
        }
        // A per-mailbox listing is one unpaged array: its length is its
        // own count, and only offset 0 exists.
        TargetKind::Inbucket => {
            let list = v.as_array()?;
            if start != 0 {
                return None;
            }
            (list, list.len() as u64)
        }
        // POP3 has no JSON listing.
        TargetKind::Greenmail => return None,
    };
    let mut out = Vec::with_capacity(list.len());
    for m in list {
        let (id, subject) = match kind {
            TargetKind::Swarmail => (m.get("id")?, m.get("subject")),
            TargetKind::Mailpit => (m.get("ID")?, m.get("Subject")),
            TargetKind::Mailhog => (
                m.get("ID")?,
                m.get("Content")
                    .and_then(|c| c.get("Headers"))
                    .and_then(|h| h.get("Subject"))
                    .and_then(|s| s.get(0)),
            ),
            _ => (m.get("id")?, m.get("subject")),
        };
        out.push((
            id_text(id)?,
            subject
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_owned(),
        ));
    }
    Some((total, out))
}

/// The subject of a header block (a POP3 `TOP n 0` answer), as the crate's
/// own MIME parser reads it.
pub fn subject_of_headers(head: &[u8]) -> Result<String, WireError> {
    let parsed = mail_parser::MessageParser::default()
        .parse(head)
        .ok_or(WireError::Malformed)?;
    Ok(parsed.subject().unwrap_or_default().to_owned())
}

/// Only parts that are text/plain or text/html as sent: the parser's
/// converted alternatives are not what was stored.
fn parse_raw(raw: &[u8]) -> Result<Fetched, WireError> {
    use mail_parser::MimeHeaders;
    let parsed = mail_parser::MessageParser::default()
        .parse(raw)
        .ok_or(WireError::Malformed)?;
    // A part's body may legally contain its own boundary string mid-line;
    // the MIME parser reads that as a delimiter and cuts the part short.
    // A shorter body is never a success, so such a message is refused.
    for part in &parsed.parts {
        let Some(boundary) = part
            .content_type()
            .and_then(|c| c.attribute("boundary"))
            .filter(|b| !b.is_empty())
        else {
            continue;
        };
        let needle = format!("--{boundary}");
        let mut from = 0;
        while let Some(at) = find_bytes(&raw[from..], needle.as_bytes()).map(|i| i + from) {
            if at > 0 && raw[at - 1] != b'\n' {
                return Err(WireError::Malformed);
            }
            from = at + needle.len();
        }
    }
    let part = |idx: &[u32], html: bool| {
        idx.iter()
            .filter_map(|i| parsed.parts.get(*i as usize))
            .find(|p| p.is_text_html() == html && p.is_text())
            .and_then(|p| p.text_contents().map(str::to_owned))
    };
    Ok(Fetched {
        subject: parsed.subject().map(str::to_owned),
        text: part(&parsed.text_body, false),
        html: part(&parsed.html_body, true),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub subject: Option<String>,
    pub text: Option<String>,
    pub html: Option<String>,
}

/// The fixed candidate launch: memory store, OS-assigned loopback ports,
/// the protocol's cap (`0` = unlimited), and a session cap as high as the
/// protocol's largest concurrency, so the stress profile measures the store
/// rather than the default 256-session refusal. The cap is asymmetric by
/// necessity: Mailpit and MailHog document no session setting, and Mailpit
/// answered `421` to a few of the 1000 fresh sessions on its own, so those
/// executables run as shipped and the refusals are recorded as measured data.
pub fn swarmail_template(cap: u64) -> Vec<String> {
    [
        "serve",
        "--smtp-listen",
        "{smtp_addr}",
        "--pop3-listen",
        "127.0.0.1:0",
        "--http-listen",
        "{http_addr}",
        "--max-sessions",
        &crate::protocol::MAX_SESSIONS.to_string(),
        "--max-per-inbox",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .chain([cap.to_string()])
    .collect()
}
