//! The MCP chunk's client scope (issue #385, accepted decision 0132's
//! 2026-11-19 addendum): the proof that no process an MCP chunk started
//! outlives it.
//!
//! Story B put only the holder into the chunk's delegated scope; `api-mcp`,
//! its in-process bridge and every adapter it spawns ran outside any
//! scope, so `scope_clean` proved the holder's cleanup only. The collector
//! now starts `api-mcp` in a second, sibling scope of its own (a fresh,
//! non-delegated `systemd-run --user --scope` unit per chunk): every
//! process the chunk starts there inherits it across fork, exec, `setsid`
//! and reparenting, which a process-tree walk cannot follow. The holder
//! leaves it for its own delegated scope, so it stays that scope's only
//! process.
//!
//! [`own_handshake`] runs inside the client scope before `api-mcp` exists
//! (the driver's `client-enter`): it records the kernel's own word on its
//! membership (`/proc/self/cgroup`) in an exclusive handshake file, then
//! the driver execs the workload in place. The observer, [`judge`] (the
//! driver's `client-judge`), runs outside both scopes the moment the
//! workload has ended: a recursively populated client cgroup is a
//! survivor; an empty or removed one, under a verified cgroup v2 mount and
//! the handshake's identity, is clean; anything else is unknown. One
//! deadline bounds the whole judgment, so a survivor is reported within it
//! and never waited out.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The longest a judgment may take (spec AC3: at most five seconds).
pub const JUDGE_MAX: Duration = Duration::from_secs(5);
/// The judgment's default allowance; the collector clips it further to
/// what remains of the chunk's ten minutes.
pub const JUDGE_DEFAULT: Duration = Duration::from_secs(3);
/// Diagnostic caps: members named, cgroups walked, nesting followed, and
/// bytes read from any one file.
pub const MAX_MEMBERS: usize = 64;
pub const MAX_CGROUPS: usize = 64;
pub const MAX_DEPTH: usize = 8;
pub const MAX_READ: u64 = 64 * 1024;
/// The cgroup v2 mount the kernel probe reads.
pub const CGROUP_MOUNT: &str = "/sys/fs/cgroup";
/// `statfs`'s magic for a cgroup v2 mount.
#[cfg(target_os = "linux")]
const CGROUP2_SUPER_MAGIC: i64 = 0x6367_7270;

/// The client scope is read from cgroup v2 and `/proc`: Linux only. Any
/// other platform refuses before a workload is spawned.
pub fn platform(os: &str) -> Result<(), String> {
    if os == "linux" {
        Ok(())
    } else {
        Err(format!(
            "the MCP chunk's client scope is read from cgroup v2 and /proc (Linux); {os} cannot prove that no process the chunk started survives"
        ))
    }
}

/// What `client-enter` records from inside the client scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handshake {
    /// The scope unit the collector created, `<name>.scope`.
    pub unit: String,
    /// The process's unified (`0::`) cgroup path, as the kernel named it.
    pub cgroup: String,
    /// The process that entered, and then exec'd the workload.
    pub pid: u32,
}

/// One process found in a populated client scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub pid: u32,
    /// Its `comm`, escaped to printable ASCII; `None` when it exited
    /// between being listed and being named (an identity race).
    pub name: Option<String>,
    /// The cgroup it was listed in, relative to the client scope's.
    pub cgroup: String,
}

/// The verdict on one client scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Recursively empty, or removed, with its identity proved.
    Clean,
    /// Populated at the first observation.
    Survivor,
    /// No affirmative evidence either way; the observation says why.
    Unknown,
}

/// The receipt the collector writes into the chunk's ledger entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// The unit the collector expected.
    pub unit: String,
    /// The handshake's cgroup path, once it was read and matched.
    pub cgroup: Option<String>,
    pub verdict: Verdict,
    /// `empty`, `removed` or `populated`; for `unknown`, the reason.
    pub observation: String,
    /// The survivors named, at most [`MAX_MEMBERS`].
    pub members: Vec<Member>,
    /// Every member was listed and every name tried, within the caps and
    /// the deadline; `false` says the list is partial (see `notes`).
    pub complete: bool,
    /// Truncation, identity races and a diagnostic cut by the deadline.
    pub notes: Vec<String>,
}

impl Receipt {
    /// A receipt without affirmative evidence, for `reason`.
    pub fn unknown(unit: &str, cgroup: Option<String>, reason: impl Into<String>) -> Receipt {
        Receipt {
            unit: unit.to_owned(),
            cgroup,
            verdict: Verdict::Unknown,
            observation: reason.into(),
            members: Vec::new(),
            complete: false,
            notes: Vec::new(),
        }
    }

    fn clean(unit: &str, cgroup: Option<String>, how: &str) -> Receipt {
        Receipt {
            verdict: Verdict::Clean,
            observation: how.into(),
            complete: true,
            ..Receipt::unknown(unit, cgroup, "")
        }
    }

    /// One line for the console and the operator: the unit, the
    /// observation and every survivor named.
    pub fn summary(&self) -> String {
        let verdict = match self.verdict {
            Verdict::Clean => "clean",
            Verdict::Survivor => "SURVIVOR",
            Verdict::Unknown => "unknown",
        };
        let mut s = format!(
            "client scope {}: {verdict}, {}",
            self.unit, self.observation
        );
        for m in &self.members {
            s.push_str(&format!(
                "; pid {} ({}) in {}",
                m.pid,
                m.name.as_deref().unwrap_or("exited before it was named"),
                m.cgroup
            ));
        }
        for n in &self.notes {
            s.push_str(&format!("; {n}"));
        }
        // A receipt read back may carry anything: the line stays one line.
        s.chars()
            .map(|c| {
                if c.is_control() {
                    format!("\\x{:02x}", u32::from(c))
                } else {
                    c.to_string()
                }
            })
            .collect()
    }

    /// Whether this receipt affirmatively proves `unit` clean (admission).
    pub fn proves_clean(&self, unit: &str) -> Result<(), String> {
        if self.unit != unit {
            return Err(format!(
                "its client receipt is for {}, not {unit}",
                self.unit
            ));
        }
        let cgroup = self.cgroup.as_deref().unwrap_or("");
        if !cgroup_names_unit(cgroup, unit) {
            return Err(format!(
                "its client receipt's cgroup {cgroup:?} is not {unit}'s"
            ));
        }
        match (self.verdict, self.observation.as_str()) {
            (Verdict::Clean, "empty" | "removed") if self.members.is_empty() && self.complete => {
                Ok(())
            }
            _ => Err(format!(
                "its client scope was not proved clean: {}",
                self.summary()
            )),
        }
    }
}

/// Whether `cgroup` is an absolute, plain cgroup path whose last component
/// is `unit`.
pub fn cgroup_names_unit(cgroup: &str, unit: &str) -> bool {
    let plain = cgroup.starts_with('/')
        && !cgroup.chars().any(char::is_control)
        && cgroup
            .split('/')
            .skip(1)
            .all(|c| !c.is_empty() && c != "." && c != "..");
    plain && cgroup.rsplit('/').next() == Some(unit)
}

/// A unit name the collector may hand over: `<name>.scope`, plain.
pub fn valid_unit(unit: &str) -> bool {
    unit.len() <= 255
        && unit.len() > ".scope".len()
        && unit.ends_with(".scope")
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.:@".contains(c))
}

/// Printable ASCII; every other byte, and the backslash, as `\xNN`.
pub fn escape(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        if (0x20..0x7f).contains(&b) && b != b'\\' {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    s
}

/// The unified cgroup path of a `/proc/<pid>/cgroup` text: its one `0::`
/// line.
pub fn unified(text: &str) -> Option<&str> {
    let mut lines = text.lines().filter_map(|l| l.strip_prefix("0::"));
    let path = lines.next()?;
    lines.next().is_none().then_some(path)
}

/// The handshake of the running process for `unit`, from its own
/// `/proc/self/cgroup`: it must already be in the unit's cgroup.
pub fn own_handshake(unit: &str, proc_cgroup: &str) -> Result<Handshake, String> {
    let cgroup = unified(proc_cgroup)
        .ok_or("/proc/self/cgroup has no single unified (0::) line: not a cgroup v2 host")?;
    if !cgroup_names_unit(cgroup, unit) {
        return Err(format!(
            "this process is in {}, not in its client scope {unit}",
            escape(cgroup.as_bytes())
        ));
    }
    Ok(Handshake {
        unit: unit.to_owned(),
        cgroup: cgroup.to_owned(),
        pid: std::process::id(),
    })
}

/// Confirm, through the cgroup v2 mount at `mount`, that `pid` is listed
/// in `cgroup`'s `cgroup.procs`: the handshake's path is one the mount the
/// observer reads actually holds, not only the kernel's name for it.
pub fn confirm_member(mount: &Path, cgroup: &str, pid: u32) -> Result<(), String> {
    use std::io::Read;
    let path = mount
        .join(cgroup.trim_start_matches('/'))
        .join("cgroup.procs");
    let mut text = String::new();
    std::fs::File::open(&path)
        .and_then(|f| f.take(MAX_READ).read_to_string(&mut text))
        .map_err(|e| {
            format!(
                "{}: {e}: the client scope is not visible under the mount",
                path.display()
            )
        })?;
    if text.lines().any(|l| l.trim() == pid.to_string()) {
        Ok(())
    } else {
        Err(format!(
            "pid {pid} is not listed in {}: the mount does not show this process in its client scope",
            path.display()
        ))
    }
}

/// Write `bytes` to `path`, which must not exist yet (owner-only, synced).
pub fn write_exclusive(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!("{}: {e} (it must be new)", path.display()))?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Where the judgment reads from. Every method is one bounded read; the
/// judgment's deadline cuts any of them.
pub trait Probe {
    /// The handshake file's bytes.
    fn handshake(&self) -> impl Future<Output = io::Result<Vec<u8>>>;
    /// `Ok` when the mount is a readable cgroup v2 mount.
    fn mount(&self) -> impl Future<Output = Result<(), String>>;
    /// `cgroup.events` of `cgroup`; `NotFound` when it is removed.
    fn events(&self, cgroup: &str) -> impl Future<Output = io::Result<String>>;
    /// `cgroup.procs` of `cgroup`.
    fn procs(&self, cgroup: &str) -> impl Future<Output = io::Result<String>>;
    /// The child cgroups' names of `cgroup`.
    fn children(&self, cgroup: &str) -> impl Future<Output = io::Result<Vec<String>>>;
    /// The `comm` of `pid`; an error once it has exited.
    fn comm(&self, pid: u32) -> impl Future<Output = io::Result<Vec<u8>>>;
}

fn cut(what: &str) -> String {
    format!("{what} was cut at the judgment's deadline")
}

/// Judge the client scope `unit` with one deadline, `budget` from now
/// (clipped to [`JUDGE_MAX`]). The first observation decides: a populated
/// scope is a survivor however soon it empties, and diagnostics never
/// extend the deadline.
pub async fn judge<P: Probe>(p: &P, unit: &str, budget: Duration) -> Receipt {
    let deadline = tokio::time::Instant::now() + budget.min(JUDGE_MAX);
    let raw = match tokio::time::timeout_at(deadline, p.handshake()).await {
        Err(_) => return Receipt::unknown(unit, None, cut("the handshake read")),
        Ok(Err(e)) if e.kind() == io::ErrorKind::NotFound => {
            return Receipt::unknown(
                unit,
                None,
                "no handshake: the workload never entered its client scope",
            );
        }
        Ok(Err(e)) => {
            return Receipt::unknown(unit, None, format!("the handshake is unreadable: {e}"));
        }
        Ok(Ok(b)) => b,
    };
    let hs: Handshake = match serde_json::from_slice(&raw) {
        Ok(h) => h,
        Err(e) => {
            return Receipt::unknown(unit, None, format!("the handshake is malformed: {e}"));
        }
    };
    if hs.unit != unit {
        return Receipt::unknown(
            unit,
            None,
            format!(
                "the handshake names {}, not this chunk's unit",
                escape(hs.unit.as_bytes())
            ),
        );
    }
    if !cgroup_names_unit(&hs.cgroup, unit) {
        return Receipt::unknown(
            unit,
            None,
            format!(
                "the handshake's cgroup {} is not {unit}'s: the workload was not placed in its client scope",
                escape(hs.cgroup.as_bytes())
            ),
        );
    }
    let cg = Some(hs.cgroup.clone());
    match tokio::time::timeout_at(deadline, p.mount()).await {
        Err(_) => return Receipt::unknown(unit, cg, cut("the mount check")),
        Ok(Err(e)) => return Receipt::unknown(unit, cg, e),
        Ok(Ok(())) => {}
    }
    let events = match tokio::time::timeout_at(deadline, p.events(&hs.cgroup)).await {
        Err(_) => return Receipt::unknown(unit, cg, cut("the first observation")),
        // The handshake proved the placement and `client-enter` saw itself
        // listed there through this mount, so a removed cgroup is the scope
        // ended, not a unit that never existed: provided the observer's
        // mount still holds the parent, which a cgroup namespace or a
        // bind-mounted subtree would not.
        Ok(Err(e)) if e.kind() == io::ErrorKind::NotFound => {
            return removed(p, unit, cg, &hs.cgroup, deadline).await;
        }
        Ok(Err(e)) => {
            return Receipt::unknown(unit, cg, format!("cgroup.events is unreadable: {e}"));
        }
        Ok(Ok(t)) => t,
    };
    let populated: Vec<&str> = events
        .lines()
        .filter_map(|l| l.strip_prefix("populated "))
        .collect();
    match populated[..] {
        ["0"] => Receipt::clean(unit, cg, "empty"),
        ["1"] => {
            let mut r = Receipt {
                verdict: Verdict::Survivor,
                observation: "populated".into(),
                complete: true,
                ..Receipt::unknown(unit, cg, "")
            };
            let walked = tokio::time::timeout_at(deadline, diagnose(p, &hs.cgroup)).await;
            match walked {
                Ok(d) => (r.members, r.complete, r.notes) = d,
                Err(_) => {
                    r.complete = false;
                    r.notes.push(cut("the diagnostics"));
                }
            }
            r
        }
        _ => Receipt::unknown(
            unit,
            cg,
            format!("cgroup.events is malformed: {}", escape(events.as_bytes())),
        ),
    }
}

/// The verdict on a client cgroup this mount no longer holds: clean only
/// when its parent is still there, so the path is one the observer sees.
async fn removed<P: Probe>(
    p: &P,
    unit: &str,
    cg: Option<String>,
    cgroup: &str,
    deadline: tokio::time::Instant,
) -> Receipt {
    let parent = match cgroup.rsplit_once('/') {
        Some(("", _)) | None => "/",
        Some((parent, _)) => parent,
    };
    match tokio::time::timeout_at(deadline, p.children(parent)).await {
        Err(_) => Receipt::unknown(unit, cg, cut("the removed scope's parent check")),
        Ok(Ok(_)) => Receipt::clean(unit, cg, "removed"),
        Ok(Err(e)) if e.kind() == io::ErrorKind::NotFound => Receipt::unknown(
            unit,
            cg,
            format!(
                "neither the client cgroup nor its parent {} is under the mount: the observer cannot see the client scope",
                escape(parent.as_bytes())
            ),
        ),
        Ok(Err(e)) => Receipt::unknown(
            unit,
            cg,
            format!("the removed scope's parent is unreadable: {e}"),
        ),
    }
}

/// Name a populated scope's members, breadth first, within the caps. It
/// only reads: nothing is signalled here, and never a pid by its name.
async fn diagnose<P: Probe>(p: &P, root: &str) -> (Vec<Member>, bool, Vec<String>) {
    let (mut members, mut complete, mut notes) = (Vec::new(), true, Vec::new());
    let mut queue = std::collections::VecDeque::from([(String::new(), 0usize)]);
    let mut walked = 0;
    'walk: while let Some((rel, depth)) = queue.pop_front() {
        if walked == MAX_CGROUPS {
            complete = false;
            notes.push(format!("the walk stopped at {MAX_CGROUPS} cgroups"));
            break;
        }
        walked += 1;
        // Child cgroup names are anyone's: they are escaped before they
        // reach a receipt, a note or the console.
        let here = if rel.is_empty() {
            "/".to_owned()
        } else {
            escape(rel.as_bytes())
        };
        let path = format!("{root}{rel}");
        match p.procs(&path).await {
            Ok(text) => {
                for line in text.lines() {
                    let Ok(pid) = line.trim().parse::<u32>() else {
                        complete = false;
                        notes.push(format!(
                            "{here}: cgroup.procs is malformed: {}",
                            escape(line.as_bytes())
                        ));
                        continue;
                    };
                    if members.len() == MAX_MEMBERS {
                        complete = false;
                        notes.push(format!("the member list stopped at {MAX_MEMBERS}"));
                        break 'walk;
                    }
                    let name = match p.comm(pid).await {
                        Ok(b) => Some(escape(b.strip_suffix(b"\n").unwrap_or(&b))),
                        Err(_) => {
                            notes.push(format!(
                                "pid {pid} exited before it was named (an identity race)"
                            ));
                            None
                        }
                    };
                    members.push(Member {
                        pid,
                        name,
                        cgroup: here.clone(),
                    });
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                notes.push(format!(
                    "{here}: removed before its members were listed (an identity race)"
                ));
                continue;
            }
            Err(e) => {
                complete = false;
                notes.push(format!("{here}: cgroup.procs: {e}"));
                continue;
            }
        }
        match p.children(&path).await {
            Ok(names) if !names.is_empty() && depth == MAX_DEPTH => {
                complete = false;
                notes.push(format!("the walk stopped at depth {MAX_DEPTH}"));
            }
            Ok(names) => {
                queue.extend(names.into_iter().map(|n| (format!("{rel}/{n}"), depth + 1)));
            }
            // A cgroup the kernel removed held no process and no child at
            // its removal, so nothing is missing from the list; the race is
            // still recorded.
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                notes.push(format!(
                    "{here}: removed before its children were listed (an identity race)"
                ));
            }
            Err(e) => {
                complete = false;
                notes.push(format!("{here}: children: {e}"));
            }
        }
    }
    (members, complete, notes)
}

/// The kernel: a cgroup v2 mount, `/proc` and the handshake file.
pub struct Kernel {
    pub mount: PathBuf,
    pub handshake: PathBuf,
    verify_mount: bool,
}

impl Kernel {
    pub fn new(handshake: &Path) -> Kernel {
        Kernel {
            mount: PathBuf::from(CGROUP_MOUNT),
            handshake: handshake.to_owned(),
            verify_mount: true,
        }
    }

    /// A tree of files standing for the mount, for the classifier's own
    /// tests: its `statfs` is not checked.
    #[cfg(test)]
    pub fn unverified(mount: &Path, handshake: &Path) -> Kernel {
        Kernel {
            mount: mount.to_owned(),
            handshake: handshake.to_owned(),
            verify_mount: false,
        }
    }

    fn at(&self, cgroup: &str, file: &str) -> PathBuf {
        let mut p = self.mount.join(cgroup.trim_start_matches('/'));
        if !file.is_empty() {
            p.push(file);
        }
        p
    }
}

/// Read at most [`MAX_READ`] bytes of `path` on a blocking thread.
async fn read_bounded(path: PathBuf) -> io::Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut out = Vec::new();
        std::fs::File::open(&path)?
            .take(MAX_READ)
            .read_to_end(&mut out)?;
        Ok(out)
    })
    .await
    .map_err(io::Error::other)?
}

async fn read_text(path: PathBuf) -> io::Result<String> {
    String::from_utf8(read_bounded(path).await?)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

impl Probe for Kernel {
    async fn handshake(&self) -> io::Result<Vec<u8>> {
        read_bounded(self.handshake.clone()).await
    }

    async fn mount(&self) -> Result<(), String> {
        let (root, verify) = (self.mount.clone(), self.verify_mount);
        tokio::task::spawn_blocking(move || {
            if !root.join("cgroup.controllers").is_file() {
                return Err(format!(
                    "{} is not a readable cgroup v2 mount",
                    root.display()
                ));
            }
            if verify {
                statfs_is_cgroup2(&root)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn events(&self, cgroup: &str) -> io::Result<String> {
        read_text(self.at(cgroup, "cgroup.events")).await
    }

    async fn procs(&self, cgroup: &str) -> io::Result<String> {
        read_text(self.at(cgroup, "cgroup.procs")).await
    }

    async fn children(&self, cgroup: &str) -> io::Result<Vec<String>> {
        let dir = self.at(cgroup, "");
        tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            for e in std::fs::read_dir(dir)?.take(4 * MAX_CGROUPS) {
                let e = e?;
                if e.file_type()?.is_dir() {
                    out.push(e.file_name().to_string_lossy().into_owned());
                }
            }
            out.sort();
            Ok(out)
        })
        .await
        .map_err(io::Error::other)?
    }

    async fn comm(&self, pid: u32) -> io::Result<Vec<u8>> {
        read_bounded(PathBuf::from(format!("/proc/{pid}/comm"))).await
    }
}

#[cfg(target_os = "linux")]
fn statfs_is_cgroup2(root: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(root.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    // SAFETY: `c` is a valid NUL-terminated path and `buf` a zeroed statfs
    // the call only fills.
    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut buf) } != 0 {
        return Err(format!(
            "{}: statfs: {}",
            root.display(),
            io::Error::last_os_error()
        ));
    }
    #[allow(clippy::unnecessary_cast)]
    if buf.f_type as i64 != CGROUP2_SUPER_MAGIC {
        return Err(format!("{} is not a cgroup v2 mount", root.display()));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn statfs_is_cgroup2(_: &Path) -> Result<(), String> {
    platform(std::env::consts::OS)
}

/// The arguments of `client-enter` and `client-judge`.
#[derive(Debug, PartialEq)]
pub struct Args {
    pub unit: String,
    pub handshake: PathBuf,
    pub out: Option<PathBuf>,
    pub budget: Duration,
    pub command: Vec<String>,
}

/// Parse `--unit U --handshake F -- CMD...` (`client-enter`, `command`
/// true) or `--unit U --handshake F --out F [--budget-ms N]`
/// (`client-judge`).
pub fn parse_args(args: &[String], command: bool) -> Result<Args, String> {
    let (flags, rest) = match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => (args, &[][..]),
    };
    let (mut unit, mut handshake, mut out, mut budget) = (None, None, None, JUDGE_DEFAULT);
    let mut it = flags.iter();
    while let Some(f) = it.next() {
        let v = it.next().ok_or_else(|| format!("{f} needs a value"))?;
        match f.as_str() {
            "--unit" => unit = Some(v.clone()),
            "--handshake" => handshake = Some(PathBuf::from(v)),
            "--out" if !command => out = Some(PathBuf::from(v)),
            "--budget-ms" if !command => {
                let ms: u64 = v.parse().map_err(|_| format!("--budget-ms {v}"))?;
                budget = Duration::from_millis(ms);
                if budget > JUDGE_MAX {
                    return Err(format!(
                        "--budget-ms {ms} is above the judgment's {} ms",
                        JUDGE_MAX.as_millis()
                    ));
                }
            }
            _ => return Err(format!("unknown flag {f}")),
        }
    }
    let unit = unit.ok_or("--unit is required")?;
    if !valid_unit(&unit) {
        return Err(format!(
            "--unit {} is not a plain <name>.scope",
            escape(unit.as_bytes())
        ));
    }
    let handshake = handshake.ok_or("--handshake is required")?;
    if command && rest.is_empty() {
        return Err("a command is required after --".into());
    }
    if !command && args.iter().any(|a| a == "--") {
        return Err("client-judge takes no command".into());
    }
    if !command && out.is_none() {
        return Err("--out is required".into());
    }
    Ok(Args {
        unit,
        handshake,
        out,
        budget,
        command: rest.to_vec(),
    })
}
