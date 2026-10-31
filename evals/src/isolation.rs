//! The whole-trial namespace boundary of proposed decision 0110 — prepared,
//! not activated.
//!
//! `trial --isolated` (a hidden flag) calls [`enter`] once, single-threaded,
//! before any Tokio runtime exists. On Linux the controller stays in the host
//! namespaces, holds the host admission and supervises; a child cloned into a
//! fresh user/mount/PID/network/IPC namespace becomes the trial's PID 1,
//! builds its root and continues into the unchanged trial arm. Anything that
//! cannot be done refuses by name — `boundary_unavailable:<operation>:<ERRNO>`
//! — before a server or an agent exists. There is no fallback that runs the
//! trial unisolated, and no host-network or host-filesystem mode.
//!
//! Delivery seats cannot create namespaces, so the syscall body below has
//! never executed in the seat that wrote it. Its pure parts (the errno-name
//! table, the bind allowlist, the step order, the remount-flag arithmetic and
//! the self-check parser) are unit-tested here; its live behaviour is proven
//! only by the `#[ignore]`d suite in `tests/trial_isolation.rs`, run on a host
//! that permits unprivileged user namespaces.
//!
//! Nothing here widens what a trial may do: concurrency stays refused by the
//! public CLI and by admission (accepted 0076), and no row field claims
//! isolation (0110, Current integration 1–2).

use std::fmt;
use std::path::{Path, PathBuf};

/// Read-only system trees every isolated trial may see. A tree that is a
/// symlink on the host (merged-usr `/bin`, `/lib`) is recreated as the same
/// symlink; a missing tree is skipped. Nothing else of the host is visible.
pub const SYSTEM_ROOTS: [&str; 5] = ["/usr", "/bin", "/lib", "/lib64", "/etc"];

/// The only device nodes the boundary binds.
pub const DEVICES: [&str; 5] = [
    "/dev/null",
    "/dev/zero",
    "/dev/full",
    "/dev/random",
    "/dev/urandom",
];

/// Preconditions refused before any namespace work, by name.
pub const REQUIRES_SWARMAIL_BIN: &str = "isolation_requires_swarmail_bin";
pub const ARTIFACTS_UNSUPPORTED: &str = "isolation_artifacts_unsupported";
pub const WRAPPER_UNSUPPORTED: &str = "isolation_wrapper_unsupported";
pub const EXPOSE_INVALID: &str = "isolation_expose_invalid";
pub const SOURCE_INVALID: &str = "isolation_source_invalid";

/// Every reason `--isolated` can refuse with, in the form the diagnostic
/// prints. Boundary failures carry the failed operation and an errno name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A request precondition, or an admission verdict, named as today.
    Named(&'static str),
    /// A boundary step that failed: `boundary_unavailable:<op>:<ERRNO>`.
    Boundary {
        op: &'static str,
        errno: &'static str,
    },
}

impl Refusal {
    pub fn errno(op: &'static str, errno: i32) -> Self {
        Self::Boundary {
            op,
            errno: errno_name(errno),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => f.write_str(name),
            Self::Boundary { op, errno } => write!(f, "boundary_unavailable:{op}:{errno}"),
        }
    }
}

/// The closed set of errnos the boundary reports by name. Anything else is
/// the one category `EUNKNOWN`; a diagnostic never interpolates a raw string
/// or number (0075: finite categories).
pub const ERRNO_NAMES: [(i32, &str); 28] = [
    (libc::EPERM, "EPERM"),
    (libc::ENOENT, "ENOENT"),
    (libc::ESRCH, "ESRCH"),
    (libc::EINTR, "EINTR"),
    (libc::EIO, "EIO"),
    (libc::ENXIO, "ENXIO"),
    (libc::EBADF, "EBADF"),
    (libc::ECHILD, "ECHILD"),
    (libc::EAGAIN, "EAGAIN"),
    (libc::ENOMEM, "ENOMEM"),
    (libc::EACCES, "EACCES"),
    (libc::EFAULT, "EFAULT"),
    (libc::EBUSY, "EBUSY"),
    (libc::EEXIST, "EEXIST"),
    (libc::EXDEV, "EXDEV"),
    (libc::ENODEV, "ENODEV"),
    (libc::ENOTDIR, "ENOTDIR"),
    (libc::EISDIR, "EISDIR"),
    (libc::EINVAL, "EINVAL"),
    (libc::ENFILE, "ENFILE"),
    (libc::EMFILE, "EMFILE"),
    (libc::ENOSPC, "ENOSPC"),
    (libc::EROFS, "EROFS"),
    (libc::ENAMETOOLONG, "ENAMETOOLONG"),
    (libc::ENOSYS, "ENOSYS"),
    (libc::ENOTEMPTY, "ENOTEMPTY"),
    (libc::ELOOP, "ELOOP"),
    (libc::EOVERFLOW, "EOVERFLOW"),
];

/// The category for an errno outside [`ERRNO_NAMES`].
pub const ERRNO_UNKNOWN: &str = "EUNKNOWN";

/// A total function from any errno to a stable name.
pub fn errno_name(errno: i32) -> &'static str {
    ERRNO_NAMES
        .iter()
        .find(|(value, _)| *value == errno)
        .map_or(ERRNO_UNKNOWN, |(_, name)| name)
}

/// One setup step inside the trial's init, in the only order it may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `setgroups` denied, then the identity uid and gid maps.
    Identity,
    /// `/` and everything below it made private.
    MountPrivate,
    /// The bounded tmpfs that becomes the root.
    StageRoot,
    /// Read-only system trees, device nodes and named executables.
    Binds,
    /// A fresh `/proc`, mounted while the host `/proc` is still visible.
    Proc,
    /// `pivot_root` into the staged root.
    Pivot,
    /// The old root unmounted and detached.
    Detach,
    /// `lo` brought up in the private network namespace.
    Loopback,
    /// Every descriptor from 3 up closed.
    CloseRange,
    /// `no_new_privs`.
    NoNewPrivs,
    /// Bounding and ambient capability sets emptied.
    CapBound,
    /// Effective, permitted and inheritable sets emptied.
    Capset,
    /// Marked nondumpable.
    Dumpable,
    /// Parent-death signal, then a re-check that the parent is still alive.
    ParentDeath,
    /// The init verifies its own boundary before anything runs.
    SelfCheck,
    /// The controller's signal mask restored.
    Unblock,
}

/// The fixed order. The executor iterates this array, so the order a test
/// asserts is the order that runs.
pub const STEPS: [Step; 16] = [
    Step::Identity,
    Step::MountPrivate,
    Step::StageRoot,
    Step::Binds,
    Step::Proc,
    Step::Pivot,
    Step::Detach,
    Step::Loopback,
    Step::CloseRange,
    Step::NoNewPrivs,
    Step::CapBound,
    Step::Capset,
    Step::Dumpable,
    Step::ParentDeath,
    Step::SelfCheck,
    Step::Unblock,
];

impl Step {
    /// The operation name a refusal carries when the step fails.
    pub fn op(self) -> &'static str {
        match self {
            Self::Identity => "identity_map",
            Self::MountPrivate => "mount_private",
            Self::StageRoot => "stage_root",
            Self::Binds => "bind",
            Self::Proc => "proc",
            Self::Pivot => "pivot_root",
            Self::Detach => "detach",
            Self::Loopback => "loopback",
            Self::CloseRange => "close_range",
            Self::NoNewPrivs => "no_new_privs",
            Self::CapBound => "cap_bound",
            Self::Capset => "capset",
            Self::Dumpable => "dumpable",
            Self::ParentDeath => "parent_death",
            Self::SelfCheck => "self_check",
            Self::Unblock => "unblock",
        }
    }
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

/// What the boundary binds: the fixed read-only trees and device nodes, plus
/// the named executables and exposed files at their own absolute paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindPlan {
    pub system: Vec<PathBuf>,
    pub devices: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
}

/// Resolve every named source against the allowlist. A source below a system
/// tree is already visible (it need only exist as a file); any other is bound
/// at its own path after [`resolve_source`] accepts it.
pub fn bind_plan(sources: &[PathBuf]) -> Result<BindPlan, SourceError> {
    let mut files: Vec<PathBuf> = Vec::new();
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
        let resolved = resolve_source(source)?;
        if !files.contains(&resolved) {
            files.push(resolved);
        }
    }
    Ok(BindPlan {
        system: SYSTEM_ROOTS.iter().map(PathBuf::from).collect(),
        devices: DEVICES.iter().map(PathBuf::from).collect(),
        files,
    })
}

/// What the isolated trial needs from the host, resolved before any
/// namespace work.
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

/// Validate an isolated request. Each refusal is named and precedes every
/// namespace operation; `concurrency_unsupported` and
/// `concurrent_trials_refused` keep their existing meaning and are decided
/// before this runs.
pub fn request(
    swarmail_bin: Option<PathBuf>,
    artifacts: bool,
    wrapper: bool,
    expose: &[PathBuf],
    runner: PathBuf,
    agent: Option<PathBuf>,
) -> Result<Request, Refusal> {
    let Some(swarmail_bin) = swarmail_bin else {
        return Err(Refusal::Named(REQUIRES_SWARMAIL_BIN));
    };
    if artifacts {
        return Err(Refusal::Named(ARTIFACTS_UNSUPPORTED));
    }
    if wrapper {
        return Err(Refusal::Named(WRAPPER_UNSUPPORTED));
    }
    if expose.iter().any(|path| resolve_source(path).is_err()) {
        return Err(Refusal::Named(EXPOSE_INVALID));
    }
    let request = Request {
        swarmail_bin,
        runner,
        agent,
        expose: expose.to_vec(),
    };
    bind_plan(&request.sources()).map_err(|_| Refusal::Named(SOURCE_INVALID))?;
    Ok(request)
}

/// The remount flags that keep a bind mount read-only without clearing any
/// flag the kernel locked on the copy (`statfs` reports the inherited ones).
/// `device` leaves `nodev` off so a bound device node stays usable.
pub fn remount_flags(statfs_flags: u64, device: bool) -> u64 {
    const MS_RDONLY: u64 = 1;
    const MS_NOSUID: u64 = 2;
    const MS_NODEV: u64 = 4;
    const MS_NOEXEC: u64 = 8;
    const MS_REMOUNT: u64 = 32;
    const MS_BIND: u64 = 4096;
    const MS_NOATIME: u64 = 1024;
    const MS_NODIRATIME: u64 = 2048;
    const MS_RELATIME: u64 = 0x20_0000;
    // The statfs `f_flags` bits (ST_*) for the flags the kernel can lock.
    const ST_NODEV: u64 = 4;
    const ST_NOEXEC: u64 = 8;
    const ST_NOATIME: u64 = 1024;
    const ST_NODIRATIME: u64 = 2048;
    const ST_RELATIME: u64 = 4096;
    let mut flags = MS_BIND | MS_REMOUNT | MS_RDONLY | MS_NOSUID;
    if !device || statfs_flags & ST_NODEV != 0 {
        flags |= MS_NODEV;
    }
    if statfs_flags & ST_NOEXEC != 0 {
        flags |= MS_NOEXEC;
    }
    if statfs_flags & ST_NOATIME != 0 {
        flags |= MS_NOATIME;
    }
    if statfs_flags & ST_NODIRATIME != 0 {
        flags |= MS_NODIRATIME;
    }
    if statfs_flags & ST_RELATIME != 0 {
        flags |= MS_RELATIME;
    }
    flags
}

/// What the init observes about itself after the boundary is built.
pub struct Observed<'a> {
    pub pid: u32,
    /// Numeric entries of `/proc`.
    pub proc_pids: &'a [u32],
    /// The text of `/proc/net/dev`.
    pub net_dev: &'a str,
    /// The text of `/proc/self/status`.
    pub status: &'a str,
    /// Whether `/` is the staged tmpfs.
    pub root_is_tmpfs: bool,
}

/// One `Name:\tvalue` field of `/proc/<pid>/status`.
pub fn status_field<'a>(status: &'a str, name: &str) -> Option<&'a str> {
    status
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(':'))
        .map(str::trim)
}

fn cap_empty(status: &str, name: &str) -> bool {
    status_field(status, name)
        .is_some_and(|value| !value.is_empty() && value.bytes().all(|b| b == b'0'))
}

/// Interface names listed by `/proc/net/dev` (two header lines, then one
/// `name: counters…` line each).
pub fn interfaces(net_dev: &str) -> Vec<&str> {
    net_dev
        .lines()
        .skip(2)
        .filter_map(|line| line.split_once(':').map(|(name, _)| name.trim()))
        .collect()
}

/// The init's self-check. `Err` names the first failed check as an
/// operation, so the refusal reads `boundary_unavailable:<check>:EINVAL`.
pub fn verify(observed: &Observed<'_>) -> Result<(), &'static str> {
    if observed.pid != 1 {
        return Err("self_check_pid");
    }
    if observed.proc_pids != [1] {
        return Err("self_check_proc");
    }
    if interfaces(observed.net_dev) != ["lo"] {
        return Err("self_check_net");
    }
    if !observed.root_is_tmpfs {
        return Err("self_check_root");
    }
    let caps = ["CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"];
    if !caps.iter().all(|name| cap_empty(observed.status, name)) {
        return Err("self_check_caps");
    }
    if status_field(observed.status, "NoNewPrivs") != Some("1") {
        return Err("self_check_privs");
    }
    Ok(())
}

/// The boundary entry. A test passes its own; production passes [`enter`].
pub type Enter = fn(&Request) -> Result<(), Refusal>;

#[cfg(target_os = "linux")]
pub use linux::enter;

/// Non-Linux targets have no boundary and always refuse by name.
#[cfg(not(target_os = "linux"))]
pub fn enter(_request: &Request) -> Result<(), Refusal> {
    Err(Refusal::errno("platform", libc::ENOSYS))
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn rank(step: Step) -> usize {
        // Exhaustive on purpose: adding a step without ranking it here fails
        // to compile, so STEPS cannot silently omit one.
        match step {
            Step::Identity => 0,
            Step::MountPrivate => 1,
            Step::StageRoot => 2,
            Step::Binds => 3,
            Step::Proc => 4,
            Step::Pivot => 5,
            Step::Detach => 6,
            Step::Loopback => 7,
            Step::CloseRange => 8,
            Step::NoNewPrivs => 9,
            Step::CapBound => 10,
            Step::Capset => 11,
            Step::Dumpable => 12,
            Step::ParentDeath => 13,
            Step::SelfCheck => 14,
            Step::Unblock => 15,
        }
    }

    fn at(step: Step) -> usize {
        STEPS.iter().position(|s| *s == step).unwrap()
    }

    #[test]
    fn errno_table_is_total_closed_and_unambiguous() {
        let mut values = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for (value, name) in ERRNO_NAMES {
            assert!(values.insert(value), "errno {value} named twice");
            assert!(names.insert(name), "{name} used twice");
            assert!(name.starts_with('E') && name.bytes().all(|b| b.is_ascii_uppercase()));
            assert_eq!(errno_name(value), name);
        }
        // The two failures the delivery boxes have actually shown.
        assert_eq!(errno_name(libc::EPERM), "EPERM");
        assert_eq!(errno_name(libc::ENOSPC), "ENOSPC");
        // Everything else is one finite category, never a raw number.
        for stray in [0, -1, 9999, i32::MAX, libc::EDOM, libc::ENOTSOCK] {
            assert_eq!(errno_name(stray), ERRNO_UNKNOWN);
        }
    }

    #[test]
    fn refusal_reads_as_the_documented_diagnostic() {
        assert_eq!(
            Refusal::errno("clone", libc::ENOSPC).to_string(),
            "boundary_unavailable:clone:ENOSPC"
        );
        assert_eq!(
            Refusal::errno("proc", 12345).to_string(),
            "boundary_unavailable:proc:EUNKNOWN"
        );
        assert_eq!(
            Refusal::Named(REQUIRES_SWARMAIL_BIN).to_string(),
            "isolation_requires_swarmail_bin"
        );
    }

    #[test]
    fn steps_run_in_the_only_safe_order() {
        for (index, step) in STEPS.iter().enumerate() {
            assert_eq!(rank(*step), index, "{step:?} is out of place");
        }
        let ops: std::collections::HashSet<_> = STEPS.iter().map(|s| s.op()).collect();
        assert_eq!(ops.len(), STEPS.len(), "every step names a distinct op");
        // Identity maps first: nothing else is meaningful before them.
        assert_eq!(at(Step::Identity), 0);
        // A fresh /proc needs the host /proc visible, so before the pivot;
        // the binds' sources are host paths, so also before it.
        assert!(at(Step::Proc) < at(Step::Pivot));
        assert!(at(Step::Binds) < at(Step::Pivot));
        assert!(at(Step::StageRoot) < at(Step::Binds));
        assert!(at(Step::Pivot) < at(Step::Detach));
        // Loopback needs the privileges that are dropped later.
        assert!(at(Step::Loopback) < at(Step::CapBound));
        // The parent-liveness descriptor survives the close until its check.
        assert!(at(Step::CloseRange) < at(Step::ParentDeath));
        // The bounding set needs CAP_SETPCAP, which capset removes.
        assert!(at(Step::CapBound) < at(Step::Capset));
        // Descriptor closure and every privilege drop precede the self-check.
        for before in [
            Step::CloseRange,
            Step::NoNewPrivs,
            Step::CapBound,
            Step::Capset,
            Step::Dumpable,
        ] {
            assert!(at(before) < at(Step::SelfCheck), "{before:?}");
        }
        // The parent-death signal is set after the last credential change
        // (credential changes can clear it) and the check is last but one.
        let last_credential = [
            Step::NoNewPrivs,
            Step::CapBound,
            Step::Capset,
            Step::Dumpable,
        ]
        .into_iter()
        .map(at)
        .max()
        .unwrap();
        assert!(last_credential < at(Step::ParentDeath));
        assert!(at(Step::ParentDeath) < at(Step::SelfCheck));
        // No agent can run before the check, and the mask is restored last.
        assert_eq!(at(Step::Unblock), STEPS.len() - 1);
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
        assert_eq!(
            plan.devices,
            [
                "/dev/null",
                "/dev/zero",
                "/dev/full",
                "/dev/random",
                "/dev/urandom"
            ]
            .map(PathBuf::from)
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
        std::fs::write(&bin, b"x").unwrap();
        std::fs::write(&runner, b"x").unwrap();
        let ask = |bin: Option<PathBuf>, artifacts, wrapper, expose: &[PathBuf]| {
            request(bin, artifacts, wrapper, expose, runner.clone(), None)
        };
        let named = |name| Err(Refusal::Named(name));
        // Every refusal wins over the ones after it.
        assert_eq!(
            ask(None, true, true, &[dir.join("no")]),
            named(REQUIRES_SWARMAIL_BIN)
        );
        assert_eq!(
            ask(Some(bin.clone()), true, true, &[dir.join("no")]),
            named(ARTIFACTS_UNSUPPORTED)
        );
        assert_eq!(
            ask(Some(bin.clone()), false, true, &[dir.join("no")]),
            named(WRAPPER_UNSUPPORTED)
        );
        for bad in [PathBuf::from("relative"), dir.join("no"), dir.clone()] {
            assert_eq!(
                ask(Some(bin.clone()), false, false, &[bad]),
                named(EXPOSE_INVALID)
            );
        }
        // A named executable that cannot be bound is its own refusal.
        assert_eq!(
            ask(Some(dir.join("missing")), false, false, &[]),
            named(SOURCE_INVALID)
        );
        let ok = ask(Some(bin.clone()), false, false, &[]).unwrap();
        assert_eq!(ok.sources(), vec![runner.clone(), bin.clone()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn remount_keeps_the_kernel_locked_flags_and_adds_read_only() {
        let base = remount_flags(0, false);
        // MS_BIND | MS_REMOUNT | MS_RDONLY | MS_NOSUID | MS_NODEV
        assert_eq!(base, 4096 | 32 | 1 | 2 | 4);
        // A device node stays usable: nodev is not added on its behalf.
        assert_eq!(remount_flags(0, true) & 4, 0);
        // But a nodev the host already set is never dropped.
        assert_eq!(remount_flags(4, true) & 4, 4);
        let locked = 8 | 1024 | 2048 | 4096;
        let flags = remount_flags(locked, false);
        for bit in [8, 1024, 2048, 0x20_0000] {
            assert_eq!(flags & bit, bit);
        }
    }

    const NET_DEV_ONLY_LO: &str = "Inter-|   Receive\n face |bytes\n    lo: 0 0 0\n";
    const CLEAN_STATUS: &str = "Name:\tswarmail\nNoNewPrivs:\t1\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000000\nCapEff:\t0000000000000000\nCapBnd:\t0000000000000000\nCapAmb:\t0000000000000000\n";

    fn observed<'a>(net: &'a str, status: &'a str, pids: &'a [u32]) -> Observed<'a> {
        Observed {
            pid: 1,
            proc_pids: pids,
            net_dev: net,
            status,
            root_is_tmpfs: true,
        }
    }

    #[test]
    fn self_check_names_the_first_failed_property() {
        assert_eq!(
            verify(&observed(NET_DEV_ONLY_LO, CLEAN_STATUS, &[1])),
            Ok(())
        );
        let mut seen = observed(NET_DEV_ONLY_LO, CLEAN_STATUS, &[1]);
        seen.pid = 7;
        assert_eq!(verify(&seen), Err("self_check_pid"));
        assert_eq!(
            verify(&observed(NET_DEV_ONLY_LO, CLEAN_STATUS, &[1, 9])),
            Err("self_check_proc")
        );
        assert_eq!(
            verify(&observed(NET_DEV_ONLY_LO, CLEAN_STATUS, &[])),
            Err("self_check_proc")
        );
        let two = format!("{NET_DEV_ONLY_LO}  eth0: 0 0 0\n");
        assert_eq!(
            verify(&observed(&two, CLEAN_STATUS, &[1])),
            Err("self_check_net")
        );
        assert_eq!(
            verify(&observed("", CLEAN_STATUS, &[1])),
            Err("self_check_net")
        );
        let mut seen = observed(NET_DEV_ONLY_LO, CLEAN_STATUS, &[1]);
        seen.root_is_tmpfs = false;
        assert_eq!(verify(&seen), Err("self_check_root"));
        for field in ["CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"] {
            let dirty = CLEAN_STATUS.replace(
                &format!("{field}:\t0000000000000000"),
                &format!("{field}:\t0000000000000400"),
            );
            assert_eq!(
                verify(&observed(NET_DEV_ONLY_LO, &dirty, &[1])),
                Err("self_check_caps"),
                "{field}"
            );
            // A missing field is a failure, never a pass.
            let missing = CLEAN_STATUS.replace(&format!("{field}:"), "Other:");
            assert_eq!(
                verify(&observed(NET_DEV_ONLY_LO, &missing, &[1])),
                Err("self_check_caps"),
                "{field}"
            );
        }
        let privs = CLEAN_STATUS.replace("NoNewPrivs:\t1", "NoNewPrivs:\t0");
        assert_eq!(
            verify(&observed(NET_DEV_ONLY_LO, &privs, &[1])),
            Err("self_check_privs")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_status_parser_reads_the_real_kernel_format() {
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        for field in [
            "CapInh",
            "CapPrm",
            "CapEff",
            "CapBnd",
            "CapAmb",
            "NoNewPrivs",
        ] {
            assert!(
                status_field(&status, field).is_some(),
                "{field} missing from the real status file"
            );
        }
        let net = std::fs::read_to_string("/proc/net/dev").unwrap();
        // The real file has exactly the two-line header the parser skips.
        assert!(net.lines().take(2).all(|l| l.contains('|')));
        // An ordinary test process is not a boundary: the check must fail.
        let pids: Vec<u32> = vec![std::process::id()];
        let seen = Observed {
            pid: std::process::id(),
            proc_pids: &pids,
            net_dev: &net,
            status: &status,
            root_is_tmpfs: false,
        };
        assert!(verify(&seen).is_err());
    }
}
