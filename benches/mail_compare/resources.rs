//! Enforced resources (accepted decision 0132 §3 and §4): every protocol 4
//! launch runs in a driver-owned cgroup v2 leaf with 2 CPUs, 2048 MiB, no
//! swap and 4096 tasks, pinned to two cores by affinity, sampled, and removed
//! with `cgroup.kill`. The leaf is a child of the scope systemd made for the
//! driver (`systemd-run --user --scope -p Delegate=yes`); systemd supervises
//! nothing of the target, so the driver stays the one owner, parent and
//! reaper (0092 §6, 0093 §7).
//!
//! Everything that reads or writes cgroupfs goes through [`Tree`], and every
//! `/proc` and `/sys` reader takes its root as a path, so the logic is proved
//! on fixture trees of plain files on any host. What only the kernel can do
//! (the join, `cgroup.kill`, `rmdir` of a populated leaf, the OOM killer, the
//! CPU quota) is proved by the ignored real proofs. Protocols 1 to 3 never
//! touch any of this.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// The one refusal for a host without delegated cgroup v2, word for word
/// (0132 §3.7). There is no unenforced fallback.
pub const NO_DELEGATION: &str = "resources: no delegated cgroup v2 (cpu memory pids)";
pub const CONTROLLERS: [&str; 3] = ["cpu", "memory", "pids"];
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// The five limits of 0132 §3.2, as written. A read-back that differs
/// refuses the launch.
pub const MEMORY_MAX: &str = "2147483648";
pub const MEMORY_SWAP_MAX: &str = "0";
pub const MEMORY_OOM_GROUP: &str = "1";
pub const CPU_MAX: &str = "200000 100000";
pub const PIDS_MAX: &str = "4096";

/// Files every leaf must have: the five limits, the kill switch, the peak
/// and the readings the sampler takes.
const LEAF_FILES: [&str; 13] = [
    "memory.max",
    "memory.swap.max",
    "memory.oom.group",
    "cpu.max",
    "pids.max",
    "cgroup.kill",
    "memory.peak",
    "cgroup.procs",
    "memory.current",
    "memory.stat",
    "memory.events",
    "cpu.stat",
    "pids.current",
];

/// The filesystem the cgroup code reads and writes. [`Host`] is the real
/// one; a test passes a tree that records or lies.
pub trait Tree: Send + Sync {
    fn read(&self, p: &Path) -> std::io::Result<String>;
    /// Replace the contents of an existing file with one `write`, as
    /// cgroupfs wants. A missing file is an error, never created.
    fn write(&self, p: &Path, text: &str) -> std::io::Result<()>;
    fn exists(&self, p: &Path) -> bool;
    fn mkdir(&self, p: &Path) -> std::io::Result<()>;
    fn rmdir(&self, p: &Path) -> std::io::Result<()>;
}

pub struct Host;

impl Tree for Host {
    fn read(&self, p: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(p)
    }

    fn write(&self, p: &Path, text: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(p)?;
        f.write_all(text.as_bytes())
    }

    fn exists(&self, p: &Path) -> bool {
        std::fs::symlink_metadata(p).is_ok()
    }

    fn mkdir(&self, p: &Path) -> std::io::Result<()> {
        std::fs::create_dir(p)
    }

    fn rmdir(&self, p: &Path) -> std::io::Result<()> {
        std::fs::remove_dir(p)
    }
}

pub fn host() -> Arc<dyn Tree> {
    Arc::new(Host)
}

// ---- parsers ---------------------------------------------------------------

/// A flat `key value` file (`cpu.stat`, `memory.stat`, `memory.events`):
/// one unsigned integer per key, every key once. Anything else is malformed.
pub fn parse_flat(name: &str, text: &str) -> Result<BTreeMap<String, u64>, String> {
    let mut out = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let mut it = line.split_whitespace();
        let (Some(key), Some(value), None) = (it.next(), it.next(), it.next()) else {
            return Err(format!(
                "resources: {name}: line {} is not 'key value'",
                n + 1
            ));
        };
        let value: u64 = value
            .parse()
            .map_err(|_| format!("resources: {name}: {key} is not an unsigned integer"))?;
        if out.insert(key.to_owned(), value).is_some() {
            return Err(format!("resources: {name}: {key} appears twice"));
        }
    }
    if out.is_empty() {
        return Err(format!("resources: {name} is empty"));
    }
    Ok(out)
}

/// A single-value file (`memory.current`, `memory.peak`, `pids.current`).
/// `max` is a limit, not a reading, and refuses.
pub fn parse_single(name: &str, text: &str) -> Result<u64, String> {
    text.trim().parse().map_err(|_| {
        format!(
            "resources: {name} is not an unsigned integer: {:?}",
            text.trim()
        )
    })
}

/// `cgroup.procs`: one pid per line; an empty file is an empty leaf.
pub fn parse_procs(text: &str) -> Result<Vec<i32>, String> {
    text.lines()
        .map(|l| {
            l.trim()
                .parse::<i32>()
                .ok()
                .filter(|p| *p > 0)
                .ok_or_else(|| format!("resources: cgroup.procs holds {l:?}, not a pid"))
        })
        .collect()
}

/// The cgroup v2 line of `/proc/<pid>/cgroup` (`0::/path`), as the path.
pub fn parse_proc_cgroup(text: &str) -> Result<String, String> {
    text.lines()
        .find_map(|l| l.strip_prefix("0::"))
        .map(str::to_owned)
        .ok_or_else(|| "resources: /proc cgroup has no cgroup v2 line".to_owned())
}

/// A kernel CPU list (`0-3,8,10-11`) as a set.
pub fn parse_cpu_list(text: &str) -> Result<BTreeSet<u32>, String> {
    let bad = || format!("resources: {:?} is not a CPU list", text.trim());
    let mut out = BTreeSet::new();
    for part in text.trim().split(',').filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => {
                let a: u32 = a.parse().map_err(|_| bad())?;
                let b: u32 = b.parse().map_err(|_| bad())?;
                if a > b || b > 1 << 16 {
                    return Err(bad());
                }
                out.extend(a..=b);
            }
            None => {
                out.insert(part.parse().map_err(|_| bad())?);
            }
        }
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(out)
}

/// `Cpus_allowed_list` of a `/proc/<pid>/status`.
pub fn parse_cpus_allowed(status: &str) -> Result<BTreeSet<u32>, String> {
    status
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .ok_or_else(|| "resources: status has no Cpus_allowed_list".to_owned())
        .and_then(parse_cpu_list)
}

/// The compact list form the receipts print (`0-1,4`).
pub fn format_cpu_list(cpus: &[u32]) -> String {
    let set: BTreeSet<u32> = cpus.iter().copied().collect();
    let mut parts: Vec<String> = Vec::new();
    let mut it = set.iter().copied().peekable();
    while let Some(a) = it.next() {
        let mut b = a;
        while it.peek() == Some(&(b + 1)) {
            b += 1;
            it.next();
        }
        parts.push(if a == b {
            a.to_string()
        } else {
            format!("{a}-{b}")
        });
    }
    parts.join(",")
}

fn need(map: &BTreeMap<String, u64>, name: &str, key: &str) -> Result<u64, String> {
    map.get(key)
        .copied()
        .ok_or_else(|| format!("resources: {name} lacks {key}"))
}

/// `cpu.stat`: the target's CPU time and its throttling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuStat {
    pub usage_usec: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    pub nr_throttled: u64,
    pub throttled_usec: u64,
}

impl CpuStat {
    pub fn parse(text: &str) -> Result<CpuStat, String> {
        let m = parse_flat("cpu.stat", text)?;
        Ok(CpuStat {
            usage_usec: need(&m, "cpu.stat", "usage_usec")?,
            user_usec: need(&m, "cpu.stat", "user_usec")?,
            system_usec: need(&m, "cpu.stat", "system_usec")?,
            nr_throttled: need(&m, "cpu.stat", "nr_throttled")?,
            throttled_usec: need(&m, "cpu.stat", "throttled_usec")?,
        })
    }
}

/// `memory.events`: `max` counts pressure at the limit, `oom_kill` the
/// processes the OOM killer ended. Only a rise of `oom_kill` is an OOM.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemEvents {
    pub max: u64,
    pub oom: u64,
    pub oom_kill: u64,
}

impl MemEvents {
    pub fn parse(text: &str) -> Result<MemEvents, String> {
        let m = parse_flat("memory.events", text)?;
        Ok(MemEvents {
            max: need(&m, "memory.events", "max")?,
            oom: need(&m, "memory.events", "oom")?,
            oom_kill: need(&m, "memory.events", "oom_kill")?,
        })
    }
}

/// What a `sample` row takes from `memory.stat`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemStat {
    pub anon: u64,
    pub file: u64,
    pub kernel: u64,
}

impl MemStat {
    /// `kernel` is a key of kernels from 6.5; earlier ones state its
    /// components, which are summed.
    pub fn parse(text: &str) -> Result<MemStat, String> {
        let m = parse_flat("memory.stat", text)?;
        let kernel = match m.get("kernel") {
            Some(k) => *k,
            None => [
                "kernel_stack",
                "pagetables",
                "percpu",
                "sock",
                "vmalloc",
                "slab",
            ]
            .iter()
            .map(|k| m.get(*k).copied().unwrap_or(0))
            .sum(),
        };
        Ok(MemStat {
            anon: need(&m, "memory.stat", "anon")?,
            file: need(&m, "memory.stat", "file")?,
            kernel,
        })
    }
}

// ---- detection and adoption ------------------------------------------------

/// The Linux `statfs` magic of cgroup2fs.
pub const CGROUP2_SUPER_MAGIC: u64 = 0x6367_7270;
const TMPFS_MAGIC: u64 = 0x0102_1994;
const RAMFS_MAGIC: u64 = 0x8584_58f6;

/// The `statfs` magic of the filesystem under `path`. Off Linux there is no
/// such thing, and the caller refuses.
#[cfg(target_os = "linux")]
pub fn fs_magic(path: &Path) -> std::io::Result<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::other("path holds a NUL"))?;
    // SAFETY: statfs fills a caller-owned struct from a NUL-terminated path.
    let mut s: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut s) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(s.f_type as u64)
}

#[cfg(not(target_os = "linux"))]
pub fn fs_magic(_: &Path) -> std::io::Result<u64> {
    Err(std::io::Error::other("statfs magic is read on Linux only"))
}

/// A tmpfs or ramfs scratch refuses: its pages would be charged to the
/// leaf as `shmem` and distort the memory figures (0132 §3.8). An overlay
/// whose upper layer is tmpfs is not seen; the README says so.
pub fn scratch_backing(magic: u64) -> Result<(), String> {
    if matches!(magic, TMPFS_MAGIC | RAMFS_MAGIC) {
        return Err("resources: the scratch directory is on tmpfs, whose pages would be charged to the target as shmem; use a disk-backed directory".into());
    }
    Ok(())
}

/// [`scratch_backing`] for a real directory.
pub fn check_scratch(dir: &Path) -> Result<(), String> {
    let magic = fs_magic(dir).map_err(|e| format!("resources: scratch {}: {e}", dir.display()))?;
    scratch_backing(magic)
}

/// Detect the driver's delegated scope: `cgroup.controllers` of the cgroup
/// the driver is in lists `cpu`, `memory` and `pids`. `root` is the cgroup2
/// mount, `self_cgroup` the text of `/proc/self/cgroup` and `is_cgroup2` what
/// `statfs` said of the mount. Every failure is [`NO_DELEGATION`].
pub fn detect(
    tree: &dyn Tree,
    root: &Path,
    self_cgroup: &str,
    is_cgroup2: bool,
) -> Result<PathBuf, String> {
    if !cfg!(target_os = "linux") || !is_cgroup2 {
        return Err(NO_DELEGATION.into());
    }
    let own = parse_proc_cgroup(self_cgroup).map_err(|_| NO_DELEGATION.to_owned())?;
    let dir = root.join(own.trim_start_matches('/'));
    let controllers = tree
        .read(&dir.join("cgroup.controllers"))
        .map_err(|_| NO_DELEGATION.to_owned())?;
    let have: BTreeSet<&str> = controllers.split_whitespace().collect();
    if CONTROLLERS.iter().all(|c| have.contains(c)) {
        Ok(dir)
    } else {
        Err(NO_DELEGATION.into())
    }
}

/// [`detect`] on this host.
pub fn detect_host() -> Result<PathBuf, String> {
    let root = Path::new(CGROUP_ROOT);
    let is2 = fs_magic(root).is_ok_and(|m| m == CGROUP2_SUPER_MAGIC);
    let own = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    detect(&Host, root, &own, is2)
}

/// Adopt the scope: it must hold nothing but this process and no `driver`
/// leaf. Then, in this order, make `<scope>/driver`, move the driver into it,
/// enable `+cpu +memory +pids` in the scope's `cgroup.subtree_control` and
/// read that file back (a cgroup with a member process cannot enable
/// controllers for its children, which is why the driver leaves first).
pub fn adopt(tree: &dyn Tree, scope: &Path, pid: i32) -> Result<(), String> {
    let driver = scope.join("driver");
    if tree.exists(&driver) {
        return Err(format!(
            "resources: the scope {} already holds a driver leaf",
            scope.display()
        ));
    }
    let procs = tree
        .read(&scope.join("cgroup.procs"))
        .map_err(|e| format!("resources: scope cgroup.procs: {e}"))
        .and_then(|t| parse_procs(&t))?;
    let others = procs.iter().filter(|p| **p != pid).count();
    if others > 0 {
        return Err(format!(
            "resources: the scope {} holds {others} other process(es); the driver must be its only process",
            scope.display()
        ));
    }
    tree.mkdir(&driver)
        .map_err(|e| format!("resources: make the driver leaf: {e}"))?;
    tree.write(&driver.join("cgroup.procs"), &pid.to_string())
        .map_err(|e| format!("resources: move the driver into its leaf: {e}"))?;
    tree.write(&scope.join("cgroup.subtree_control"), "+cpu +memory +pids")
        .map_err(|_| NO_DELEGATION.to_owned())?;
    let read = tree
        .read(&scope.join("cgroup.subtree_control"))
        .map_err(|_| NO_DELEGATION.to_owned())?;
    let enabled: BTreeSet<&str> = read
        .split_whitespace()
        .map(|t| t.trim_start_matches('+'))
        .collect();
    if CONTROLLERS.iter().all(|c| enabled.contains(c)) {
        Ok(())
    } else {
        Err(NO_DELEGATION.into())
    }
}

// ---- CPU layout ------------------------------------------------------------

/// Where each role runs (0132 §3.4): two distinct cores for the target, their
/// SMT siblings left idle, the rest for the clients and the driver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub target_cpus: Vec<u32>,
    pub idle_siblings: Vec<u32>,
    pub client_cpus: Vec<u32>,
    /// Siblings of a target core outside the driver's allowed set: the
    /// driver cannot keep them idle, and says so.
    pub siblings_outside_set: Vec<u32>,
}

/// A pure function of the allowed CPU set and the kernel's sibling lists:
/// the two target CPUs are the lowest allowed CPU and the lowest allowed
/// CPU that is not its sibling. `siblings(cpu)` includes `cpu` itself.
pub fn layout(
    allowed: &BTreeSet<u32>,
    siblings: &dyn Fn(u32) -> BTreeSet<u32>,
) -> Result<Layout, String> {
    let Some(&first) = allowed.iter().next() else {
        return Err("resources: the allowed CPU set is empty".into());
    };
    let of_first = siblings(first);
    let Some(&second) = allowed
        .iter()
        .find(|c| !of_first.contains(c) && **c != first)
    else {
        return Err("resources: fewer than two distinct cores in the allowed CPU set".into());
    };
    let targets = BTreeSet::from([first, second]);
    let mut sib: BTreeSet<u32> = of_first.union(&siblings(second)).copied().collect();
    sib.retain(|c| !targets.contains(c));
    let idle: BTreeSet<u32> = sib
        .iter()
        .copied()
        .filter(|c| allowed.contains(c))
        .collect();
    let outside: BTreeSet<u32> = sib.difference(&idle).copied().collect();
    let clients: Vec<u32> = allowed
        .iter()
        .copied()
        .filter(|c| !targets.contains(c) && !idle.contains(c))
        .collect();
    if clients.is_empty() {
        return Err("resources: no client CPU is left after the two target cores".into());
    }
    Ok(Layout {
        target_cpus: targets.into_iter().collect(),
        idle_siblings: idle.into_iter().collect(),
        client_cpus: clients,
        siblings_outside_set: outside.into_iter().collect(),
    })
}

/// The kernel's `thread_siblings_list` of `cpu` under `sys` (the `/sys`
/// root); a CPU whose list cannot be read is its own only sibling.
pub fn read_siblings(sys: &Path, cpu: u32) -> BTreeSet<u32> {
    let path = sys.join(format!(
        "devices/system/cpu/cpu{cpu}/topology/thread_siblings_list"
    ));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| parse_cpu_list(&t).ok())
        .unwrap_or_else(|| BTreeSet::from([cpu]))
}

/// A set of CPUs as the kernel wants it. Built before the fork, copied into
/// the child and applied there with the one async-signal-safe call.
#[derive(Clone, Copy)]
pub struct Affinity {
    #[cfg(target_os = "linux")]
    set: libc::cpu_set_t,
}

impl Affinity {
    #[cfg(target_os = "linux")]
    pub fn of(cpus: &[u32]) -> Result<Affinity, String> {
        // SAFETY: cpu_set_t is plain data; CPU_ZERO and CPU_SET write only
        // into it, and every index is bounds-checked first.
        let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
        unsafe { libc::CPU_ZERO(&mut set) };
        for c in cpus {
            if (*c as usize) >= libc::CPU_SETSIZE as usize {
                return Err(format!("resources: CPU {c} is beyond the affinity mask"));
            }
            unsafe { libc::CPU_SET(*c as usize, &mut set) };
        }
        Ok(Affinity { set })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn of(_: &[u32]) -> Result<Affinity, String> {
        Err(NO_DELEGATION.into())
    }

    /// Restrict the calling thread. Async-signal-safe: one syscall.
    #[cfg(target_os = "linux")]
    pub fn apply(&self) -> std::io::Result<()> {
        // SAFETY: sched_setaffinity reads the mask it is given.
        let rc = unsafe {
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &self.set)
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn apply(&self) -> std::io::Result<()> {
        Err(std::io::Error::other(NO_DELEGATION))
    }
}

/// Pin the driver to the client CPUs. Called before the runtime is built so
/// every worker thread inherits it; the sampler is thereby charged to the
/// driver and never to the target.
pub fn pin_self(cpus: &[u32]) -> Result<(), String> {
    Affinity::of(cpus)?
        .apply()
        .map_err(|e| format!("resources: pin the driver to the client CPUs: {e}"))
}

/// The child's side of the join, between fork and exec: write `0` to the
/// leaf's `cgroup.procs` through the fd the driver opened before the fork,
/// then set the affinity. Allocates nothing; every return is checked.
pub fn join_in_child(procs_fd: i32, affinity: &Affinity) -> std::io::Result<()> {
    // SAFETY: write(2) on a descriptor the driver opened for this purpose,
    // from a one-byte static buffer.
    let n = unsafe { libc::write(procs_fd, b"0".as_ptr().cast(), 1) };
    if n != 1 {
        return Err(std::io::Error::last_os_error());
    }
    affinity.apply()
}

// ---- the join's proof ------------------------------------------------------

/// What `/proc/<pid>` says after the spawn: the cgroup the process is in
/// (its last path component, since the full path carries the uid and the
/// unit name) and the CPUs it may run on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Joined {
    pub cgroup: String,
    pub cpus_allowed: Vec<u32>,
}

/// What `/proc/<pid>` says of the join: the whole cgroup path of its `0::`
/// line, which is what the comparison needs, and the receipt's view of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRead {
    pub path: String,
    pub joined: Joined,
}

/// Read the join back from `/proc`, keeping the whole cgroup path (V10).
pub fn read_join_whole(proc_root: &Path, pid: i32) -> Result<JoinRead, String> {
    let dir = proc_root.join(pid.to_string());
    let path = std::fs::read_to_string(dir.join("cgroup"))
        .map_err(|e| format!("resources: /proc/{pid}/cgroup: {e}"))
        .and_then(|t| parse_proc_cgroup(&t))?;
    let cpus = std::fs::read_to_string(dir.join("status"))
        .map_err(|e| format!("resources: /proc/{pid}/status: {e}"))
        .and_then(|t| parse_cpus_allowed(&t))?;
    Ok(JoinRead {
        joined: Joined {
            cgroup: path.rsplit('/').next().unwrap_or_default().to_owned(),
            cpus_allowed: cpus.into_iter().collect(),
        },
        path,
    })
}

/// The join is proved when the target is in the leaf the driver made, by its
/// whole cgroup path and not by its last component (another parent may hold
/// a cgroup of the same name), and may run on exactly the two target CPUs.
/// The receipt keeps the leaf's name.
pub fn check_join_whole(
    read: &JoinRead,
    leaf: (&str, &str),
    target_cpus: &[u32],
) -> Result<(), String> {
    let (name, path) = leaf;
    if read.path != path {
        return Err(format!(
            "resources: the target is in cgroup {:?}, not its leaf {path:?}",
            read.path
        ));
    }
    check_join(&read.joined, name, target_cpus)
}

/// The join is proved when the target is in its own leaf and may run on
/// exactly the two target CPUs.
pub fn check_join(j: &Joined, leaf: &str, target_cpus: &[u32]) -> Result<(), String> {
    if j.cgroup != leaf {
        return Err(format!(
            "resources: the target is in cgroup {:?}, not its leaf {leaf:?}",
            j.cgroup
        ));
    }
    if j.cpus_allowed != target_cpus {
        return Err(format!(
            "resources: the target may run on CPUs {}, not exactly {}",
            format_cpu_list(&j.cpus_allowed),
            format_cpu_list(target_cpus)
        ));
    }
    Ok(())
}

/// What the end of a window reads: `Some(true)` when every task still has
/// exactly the two target CPUs, `Some(false)` when any task has another set,
/// `None` when no task could be read at all (the leaf is empty, the target
/// is gone): no observation is not a pass. A task that vanished between the
/// listing and the read is skipped.
pub fn affinity_read(proc_root: &Path, pids: &[i32], target_cpus: &[u32]) -> Option<bool> {
    let want: BTreeSet<u32> = target_cpus.iter().copied().collect();
    let mut seen = false;
    let mut held = true;
    for pid in pids {
        let tasks = proc_root.join(pid.to_string()).join("task");
        let Ok(dir) = std::fs::read_dir(tasks) else {
            continue;
        };
        for t in dir.flatten() {
            match std::fs::read_to_string(t.path().join("status")).map(|s| parse_cpus_allowed(&s)) {
                Ok(Ok(have)) => {
                    seen = true;
                    held &= have == want;
                }
                Ok(Err(_)) => {
                    seen = true;
                    held = false;
                }
                Err(_) => {}
            }
        }
    }
    seen.then_some(held)
}

// ---- the leaf --------------------------------------------------------------

/// The five limits exactly as read back from the leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsRead {
    pub memory_max: String,
    pub memory_swap_max: String,
    pub memory_oom_group: String,
    pub cpu_max: String,
    pub pids_max: String,
}

/// `dir` as `/proc/<pid>/cgroup` writes a cgroup: its path below the cgroup
/// mount `root`, with a leading slash. A directory outside the mount (a
/// fixture tree) is its own path.
pub fn cgroup_path_of(dir: &Path, root: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) => format!("/{}", rel.display()),
        Err(_) => dir.display().to_string(),
    }
}

/// One launch's cgroup: `<scope>/target-<launch-id>`.
#[derive(Clone)]
pub struct Leaf {
    fs: Arc<dyn Tree>,
    dir: PathBuf,
    name: String,
}

/// What teardown established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Teardown {
    /// `cgroup.kill` was written.
    pub killed: bool,
    /// The leaf was empty and was removed. The only way `cgroup_removed`
    /// is true.
    pub removed: bool,
    /// The leaf's lifetime `memory.peak`, read before removal.
    pub memory_peak_bytes: Option<u64>,
    /// The leaf's lifetime `oom_kill` count from `memory.events`, read
    /// before removal; `None` when the file could not be read.
    pub oom_kills: Option<u64>,
}

impl Leaf {
    /// Make the leaf and apply the budget.
    pub fn create(
        fs: Arc<dyn Tree>,
        scope: &Path,
        name: &str,
    ) -> Result<(Leaf, LimitsRead), String> {
        let dir = scope.join(name);
        fs.mkdir(&dir)
            .map_err(|e| format!("resources: make leaf {name}: {e}"))?;
        match Leaf::prepare(fs.clone(), dir.clone(), name) {
            Ok(done) => Ok(done),
            Err(e) => {
                let _ = fs.rmdir(&dir);
                Err(e)
            }
        }
    }

    /// Check the leaf's files, write the five limits and read each back. A
    /// missing file or a different read-back refuses, naming the file.
    pub fn prepare(
        fs: Arc<dyn Tree>,
        dir: PathBuf,
        name: &str,
    ) -> Result<(Leaf, LimitsRead), String> {
        for f in LEAF_FILES {
            if !fs.exists(&dir.join(f)) {
                return Err(format!("resources: the leaf {name} lacks {f}"));
            }
        }
        let want = [
            ("memory.swap.max", MEMORY_SWAP_MAX),
            ("memory.max", MEMORY_MAX),
            ("memory.oom.group", MEMORY_OOM_GROUP),
            ("cpu.max", CPU_MAX),
            ("pids.max", PIDS_MAX),
        ];
        let mut got = BTreeMap::new();
        for (file, value) in want {
            fs.write(&dir.join(file), value)
                .map_err(|e| format!("resources: write {file}: {e}"))?;
            let read = fs
                .read(&dir.join(file))
                .map_err(|e| format!("resources: read {file}: {e}"))?;
            let read = read.trim().to_owned();
            if read != value {
                return Err(format!(
                    "resources: {file} reads back {read:?}, not {value:?}"
                ));
            }
            got.insert(file, read);
        }
        let limits = LimitsRead {
            memory_max: got["memory.max"].clone(),
            memory_swap_max: got["memory.swap.max"].clone(),
            memory_oom_group: got["memory.oom.group"].clone(),
            cpu_max: got["cpu.max"].clone(),
            pids_max: got["pids.max"].clone(),
        };
        Ok((
            Leaf {
                fs,
                dir,
                name: name.to_owned(),
            },
            limits,
        ))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The leaf's cgroup path as `/proc/<pid>/cgroup` writes it: its
    /// directory below the cgroup mount, with a leading slash.
    pub fn cgroup_path(&self) -> String {
        cgroup_path_of(&self.dir, Path::new(CGROUP_ROOT))
    }

    /// Remove a leaf that never held a process (a launch that failed before
    /// the fork). Whatever held one goes through [`Leaf::teardown`].
    pub fn discard(&self) {
        let _ = self.fs.rmdir(&self.dir);
    }

    pub fn procs_path(&self) -> PathBuf {
        self.dir.join("cgroup.procs")
    }

    fn text(&self, file: &str) -> Result<String, String> {
        self.fs
            .read(&self.dir.join(file))
            .map_err(|e| format!("resources: read {file}: {e}"))
    }

    pub fn read_procs(&self) -> Result<Vec<i32>, String> {
        parse_procs(&self.text("cgroup.procs")?)
    }

    pub fn cpu_stat(&self) -> Result<CpuStat, String> {
        CpuStat::parse(&self.text("cpu.stat")?)
    }

    pub fn mem_events(&self) -> Result<MemEvents, String> {
        MemEvents::parse(&self.text("memory.events")?)
    }

    pub fn mem_stat(&self) -> Result<MemStat, String> {
        MemStat::parse(&self.text("memory.stat")?)
    }

    pub fn mem_current(&self) -> Result<u64, String> {
        parse_single("memory.current", &self.text("memory.current")?)
    }

    pub fn pids_current(&self) -> Result<u64, String> {
        parse_single("pids.current", &self.text("pids.current")?)
    }

    pub fn mem_peak(&self) -> Result<u64, String> {
        parse_single("memory.peak", &self.text("memory.peak")?)
    }

    /// End every process in the leaf, whatever group, session or parent it
    /// has: a process that called `setsid` is still in the leaf (0132 §3.5).
    pub fn kill(&self) -> std::io::Result<()> {
        self.fs.write(&self.dir.join("cgroup.kill"), "1")
    }

    /// The cgroup step of cleanup: write `cgroup.kill`, read `cgroup.procs`
    /// until it is empty or `until` passes (at least once after the kill, even
    /// when the earlier steps used the whole budget), read `memory.peak`, and
    /// remove the leaf. A leaf that cannot be emptied stays in place.
    pub async fn teardown(&self, until: tokio::time::Instant) -> Teardown {
        let killed = self.kill().is_ok();
        let mut peak = None;
        let mut oom_kills = None;
        let mut removed = false;
        loop {
            // Empty first; then the peak while the leaf still exists; then
            // the removal, which the kernel may refuse for an instant while a
            // process it has just ended is still being released, so it is
            // retried inside the same budget.
            if matches!(self.read_procs(), Ok(p) if p.is_empty()) {
                if peak.is_none() {
                    peak = self.mem_peak().ok();
                }
                if oom_kills.is_none() {
                    oom_kills = self.mem_events().ok().map(|e| e.oom_kill);
                }
                if self.fs.rmdir(&self.dir).is_ok() {
                    removed = true;
                    break;
                }
            }
            if tokio::time::Instant::now() >= until {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let memory_peak_bytes = peak.or_else(|| self.mem_peak().ok());
        let oom_kills = oom_kills.or_else(|| self.mem_events().ok().map(|e| e.oom_kill));
        Teardown {
            killed,
            removed,
            memory_peak_bytes,
            oom_kills,
        }
    }
}

/// The driver's adopted scope: where leaves are made, and the CPU layout the
/// run uses. One per driver process.
pub struct Scope {
    fs: Arc<dyn Tree>,
    dir: PathBuf,
    pub layout: Layout,
    next: std::sync::atomic::AtomicU32,
    /// A test seam: see `targets::Joining`. Always false outside tests.
    #[cfg(test)]
    pub trust_cgroup: bool,
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scope({})", self.dir.display())
    }
}

impl Scope {
    pub fn new(fs: Arc<dyn Tree>, dir: PathBuf, layout: Layout) -> Scope {
        Scope {
            fs,
            dir,
            layout,
            next: std::sync::atomic::AtomicU32::new(0),
            #[cfg(test)]
            trust_cgroup: false,
        }
    }

    /// A fixture scope that cannot move a real process into its leaf: the
    /// join's cgroup read-back is taken to be the leaf (test only).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn trusting_the_join(mut self) -> Scope {
        self.trust_cgroup = true;
        self
    }

    /// The next leaf: `target-<launch id>`, ids counted from 1.
    pub fn leaf(&self) -> Result<(Leaf, LimitsRead), String> {
        let n = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Leaf::create(self.fs.clone(), &self.dir, &format!("target-{n:04}"))
    }
}

/// The whole preamble of a v4 run on this host, before any runtime exists:
/// check the scratch's backing, read the allowed CPUs and the sibling
/// layout, pin the driver to the client CPUs, then detect and adopt the
/// scope. Every failure is a refusal; none falls back to an unenforced run.
pub fn prepare_host(scratch_parent: &Path) -> Result<Scope, String> {
    if !cfg!(target_os = "linux") {
        return Err(NO_DELEGATION.into());
    }
    let dir = detect_host()?;
    check_scratch(scratch_parent)?;
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("resources: /proc/self/status: {e}"))?;
    let allowed = parse_cpus_allowed(&status)?;
    let lay = layout(&allowed, &|c| read_siblings(Path::new("/sys"), c))?;
    pin_self(&lay.client_cpus)?;
    adopt(&Host, &dir, std::process::id() as i32)?;
    Ok(Scope::new(host(), dir, lay))
}

// ---- the scratch tree's footprint ------------------------------------------

/// Allocated bytes (`st_blocks × 512`) of every entry under `root`,
/// `root` included, without following symlinks: the store's disk footprint.
pub fn allocated_bytes(root: &Path) -> std::io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(root)?;
    let mut total = meta.blocks() * 512;
    if meta.is_dir() {
        for entry in std::fs::read_dir(root)? {
            match allocated_bytes(&entry?.path()) {
                Ok(n) => total += n,
                // A file the target removed while we walked is not an error.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
    }
    Ok(total)
}

// ---- snapshots and the sampler ---------------------------------------------

/// `cpu.stat` and `memory.events` at one instant of the run's clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snap {
    pub t_ns: u64,
    pub cpu: CpuStat,
    pub events: MemEvents,
}

impl Leaf {
    pub fn snap(&self, clock: &crate::wire::Clock) -> Result<Snap, String> {
        Ok(Snap {
            t_ns: clock.now_ns(),
            cpu: self.cpu_stat()?,
            events: self.mem_events()?,
        })
    }

    /// One `sample` row: memory and tasks now.
    pub fn sample(
        &self,
        run_id: &str,
        target: crate::protocol::TargetKind,
        clock: &crate::wire::Clock,
    ) -> Result<crate::records::SampleRow, String> {
        let t_ns = clock.now_ns();
        let stat = self.mem_stat()?;
        Ok(crate::records::SampleRow {
            row_version: crate::records::RESOURCE_ROW_VERSION,
            run_id: run_id.to_owned(),
            launch: self.name.clone(),
            target,
            t_ns,
            memory_current: self.mem_current()?,
            anon: stat.anon,
            file: stat.file,
            kernel: stat.kernel,
            pids: self.pids_current()?,
        })
    }
}

/// Where sample rows go. A write that fails is the run's operational
/// failure, and the sampler stops on it.
pub type Emit = Arc<dyn Fn(crate::records::SampleRow) -> Result<(), String> + Send + Sync>;

/// A task in the driver, on the client CPUs, that reads one leaf every
/// interval until it is stopped. Its own CPU is the driver's, never the
/// target's, because the driver is pinned away from the target cores.
pub struct Sampler {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<Result<u64, String>>,
}

impl Sampler {
    pub fn spawn(
        leaf: Leaf,
        run_id: String,
        target: crate::protocol::TargetKind,
        clock: crate::wire::Clock,
        interval: Duration,
        emit: Emit,
    ) -> Sampler {
        let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut rows = 0u64;
            loop {
                tokio::select! {
                    _ = &mut stopped => return Ok(rows),
                    _ = tick.tick() => {
                        let row = leaf.sample(&run_id, target, &clock)?;
                        emit(row)?;
                        rows += 1;
                    }
                }
            }
        });
        Sampler {
            stop: Some(stop),
            task,
        }
    }

    /// Stop the sampler and return how many rows it wrote, or the first
    /// thing that went wrong.
    pub async fn stop(mut self) -> Result<u64, String> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task
            .await
            .map_err(|e| format!("resources: the sampler task failed: {e}"))?
    }
}

/// The machine a chunk ran on (0132 §12, `host.json`).
pub fn host_facts(kernel: &str, systemd: &str, layout: &Layout) -> crate::records::HostFacts {
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("model name"))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_owned())
        })
        .unwrap_or_else(|| "unavailable".into());
    let first = layout.target_cpus.first().copied().unwrap_or(0);
    let governor = std::fs::read_to_string(format!(
        "/sys/devices/system/cpu/cpu{first}/cpufreq/scaling_governor"
    ))
    .map(|g| g.trim().to_owned())
    .unwrap_or_else(|_| "unavailable".into());
    // SAFETY: sysconf takes a constant and returns a number.
    let online = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
    crate::records::HostFacts {
        kernel: kernel.to_owned(),
        systemd: systemd.to_owned(),
        cpu_model: model,
        nproc: online.max(0) as u32,
        siblings: layout
            .target_cpus
            .iter()
            .map(|c| read_siblings(Path::new("/sys"), *c).into_iter().collect())
            .collect(),
        cpu_layout: layout.clone(),
        governor,
    }
}

// ---- fixed facts: the footprint (0132 §4.5) --------------------------------

/// What one walk of a directory found: the bytes and the number of its
/// regular files, and the sha256 of its listing (V18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeWalk {
    pub bytes: u64,
    pub files: u64,
    pub sha256: String,
}

fn file_sha256(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let rustls::SupportedCipherSuite::Tls13(suite) =
        rustls::crypto::ring::cipher_suite::TLS13_AES_128_GCM_SHA256
    else {
        unreachable!("a TLS 1.3 suite")
    };
    let mut ctx = suite.common.hash_provider.start();
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Ok(ctx
        .finish()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// A name `sha256sum` prints as it is: UTF-8, no newline, no backslash.
fn plain_name(name: &std::ffi::OsStr, under: &Path) -> Result<String, String> {
    let text = name.to_str().ok_or_else(|| {
        format!(
            "{}: a file name that is not UTF-8 (sha256sum would escape it)",
            under.display()
        )
    })?;
    if text.contains('\n') || text.contains('\\') {
        return Err(format!(
            "{}: the name {text:?} holds a newline or a backslash (sha256sum would escape it)",
            under.display()
        ));
    }
    Ok(text.to_owned())
}

/// One walk of `root`: the bytes (by length) and count of its regular
/// files, and the sha256 of its listing, one line
/// `<sha256 of the file>  ./<relative path>` and a newline per file, sorted
/// by the path's bytes. That is the output of
/// `find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum` and so
/// can be checked by hand. A symbolic link and a directory count for
/// nothing and are not listed; the root itself may be reached through a link.
pub fn walk_tree(root: &Path) -> Result<TreeWalk, String> {
    let meta = std::fs::metadata(root).map_err(|e| format!("{}: {e}", root.display()))?;
    if !meta.is_dir() {
        return Err(format!("{}: not a directory", root.display()));
    }
    let mut found: Vec<(String, PathBuf, u64)> = Vec::new();
    let mut pending: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = pending.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
            let kind = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            if !kind.is_file() && !kind.is_dir() {
                continue;
            }
            let name = plain_name(&entry.file_name(), &dir)?;
            let here = format!("{rel}/{name}");
            if kind.is_dir() {
                pending.push((entry.path(), here));
            } else {
                let len = entry
                    .metadata()
                    .map_err(|e| format!("{}: {e}", entry.path().display()))?
                    .len();
                found.push((here, entry.path(), len));
            }
        }
    }
    found.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut listing = String::new();
    let mut bytes = 0;
    for (rel, path, len) in &found {
        let digest = file_sha256(path).map_err(|e| format!("{}: {e}", path.display()))?;
        listing.push_str(&format!("{digest}  .{rel}\n"));
        bytes += len;
    }
    Ok(TreeWalk {
        bytes,
        files: found.len() as u64,
        sha256: crate::targets::sha256_hex(listing.as_bytes()),
    })
}

/// The directory an archive was extracted into: `bin` with the pinned
/// `member` path removed from its end (V18).
pub fn extraction_root(bin: &Path, member: &str) -> Result<PathBuf, String> {
    if !bin.ends_with(member) {
        return Err(format!(
            "--bin {} does not end with the pinned member {member}",
            bin.display()
        ));
    }
    let mut root = bin;
    for _ in Path::new(member).components() {
        root = root.parent().unwrap_or(Path::new("."));
    }
    Ok(if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root.to_path_buf()
    })
}

/// One target's executable and the trees `footprint` walked for it, as
/// resolved paths: what the containment guard compares.
struct Occupied {
    target: crate::protocol::TargetKind,
    executable: PathBuf,
    trees: Vec<PathBuf>,
}

/// No target's walked tree may contain another target's executable or
/// walked tree, the candidate included (V18): a careless extraction of every
/// archive into one directory would inflate a row without changing a digest
/// the lock holds.
fn refuse_shared_trees(all: &[Occupied]) -> Result<(), String> {
    for a in all {
        for tree in &a.trees {
            for b in all.iter().filter(|b| b.target != a.target) {
                if b.executable.starts_with(tree) || b.trees.iter().any(|t| t.starts_with(tree)) {
                    return Err(format!(
                        "{} and {} share a tree: one's installed tree contains the other's executable or tree",
                        a.target, b.target
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Fixed facts per target, computed once from the pinned bytes with no
/// measurement window: what a user downloads, what it takes installed, and
/// what it needs that nothing pins. Launches nothing; rechecks every digest
/// it relies on, so a replaced executable refuses.
///
/// `docs` is the directory the lock's captured documents live under.
pub fn footprint(
    p: &crate::protocol::Protocol,
    lock: &crate::protocol::SourceLock,
    docs: &Path,
    bins: &BTreeMap<crate::protocol::TargetKind, PathBuf>,
    runtimes: &BTreeMap<crate::protocol::TargetKind, PathBuf>,
) -> Result<serde_json::Value, String> {
    use crate::protocol::{DocKind, TargetKind};
    let mut targets = Vec::new();
    let mut occupied: Vec<Occupied> = Vec::new();
    for t in p.launch_order(0) {
        let bin = bins
            .get(&t)
            .ok_or_else(|| format!("--bin {t}=PATH is required"))?;
        let size = |path: &Path| {
            std::fs::metadata(path)
                .map(|m| m.len())
                .map_err(|e| format!("{t}: {}: {e}", path.display()))
        };
        let mut entry = serde_json::Map::new();
        entry.insert("target".into(), serde_json::json!(t));
        if t == TargetKind::Swarmail {
            // The candidate is the release binary itself; its digest is
            // recorded per run, not pinned.
            let n = size(bin)?;
            entry.insert("download_bytes".into(), n.into());
            entry.insert(
                "download_basis".into(),
                "the release binary (cargo build --release --locked --bin swarmail)".into(),
            );
            entry.insert("installed_bytes".into(), n.into());
            entry.insert("installed_files".into(), 1.into());
            entry.insert(
                "installed_basis".into(),
                "the release binary alone: it needs no runtime".into(),
            );
            entry.insert("installed_trees".into(), serde_json::json!([]));
            occupied.push(Occupied {
                target: t,
                executable: std::fs::canonicalize(bin)
                    .map_err(|e| format!("{t}: {}: {e}", bin.display()))?,
                trees: Vec::new(),
            });
            targets.push(serde_json::Value::Object(entry));
            continue;
        }
        let pin = lock
            .pin(t)
            .ok_or_else(|| format!("{t}: the source lock pins no executable"))?;
        let runtime = runtimes.get(&t).cloned();
        let spec = crate::targets::LaunchSpec::from_pin(
            t,
            bin.clone(),
            runtime.clone(),
            pin,
            pin.storage.max_messages,
        )?;
        // Every digest the footprint relies on is rechecked from the files.
        let measured = spec.extra.artifact.as_ref().unwrap_or(&spec.binary);
        let want = spec.expected_sha256.clone().unwrap_or_default();
        if !crate::targets::unchanged(measured, &want) {
            return Err(format!(
                "{t}: the executable {} is not the pinned bytes (changed or unreadable)",
                measured.display()
            ));
        }
        for (path, sha) in &spec.extra.members {
            if !crate::targets::unchanged(path, sha) {
                return Err(format!(
                    "{t}: the checked member {} is not the pinned bytes (changed or unreadable)",
                    path.display()
                ));
            }
        }
        // Download: the archive, jar or executable a user fetches; for the
        // image row, the compressed layers of the captured manifest.
        let (download, basis) = match &pin.image {
            Some(img) => {
                let doc = pin
                    .documents
                    .iter()
                    .find(|d| d.kind == DocKind::ImageManifest)
                    .ok_or_else(|| format!("{t}: no captured image manifest"))?;
                let bytes = std::fs::read(docs.join(&doc.path))
                    .map_err(|e| format!("{t}: {}: {e}", doc.path))?;
                let v: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{t}: {}: {e}", doc.path))?;
                let layers = v["layers"]
                    .as_array()
                    .ok_or_else(|| format!("{t}: the captured manifest has no layers"))?;
                let sum = layers
                    .iter()
                    .map(|l| l["size"].as_u64())
                    .sum::<Option<u64>>()
                    .ok_or_else(|| format!("{t}: a layer states no size"))?;
                (
                    sum,
                    format!(
                        "the compressed layers of {} in the captured manifest ({} layers)",
                        img.reference,
                        layers.len()
                    ),
                )
            }
            None => (
                pin.artifact.size,
                format!("the pinned {}", pin.artifact.file_name),
            ),
        };
        entry.insert("download_bytes".into(), download.into());
        entry.insert("download_basis".into(), basis.into());
        // Installed (V18): every row has a size. An archive row is every
        // regular file of its extraction root; any other row is the
        // executable, its checked members and the whole pinned runtime tree.
        let mut trees: Vec<(&str, PathBuf, TreeWalk)> = Vec::new();
        let mut walk = |kind: &'static str, dir: &Path| -> Result<(), String> {
            let found = walk_tree(dir).map_err(|e| format!("{t}: {e}"))?;
            trees.push((kind, dir.to_path_buf(), found));
            Ok(())
        };
        let (installed, files, basis) = if pin.artifact.kind
            == crate::protocol::ArtifactKind::Archive
        {
            let member = pin
                .artifact
                .member
                .as_deref()
                .ok_or_else(|| format!("{t}: an archive pin names its member"))?;
            let root = extraction_root(measured, member).map_err(|e| format!("{t}: {e}"))?;
            // The walk skips links, so the executable is counted only when
            // it is the member itself under the resolved root: no link on
            // the way to it, and no link in its place (V18).
            let resolved = |p: &Path| {
                std::fs::canonicalize(p).map_err(|e| format!("{t}: {}: {e}", p.display()))
            };
            if resolved(measured)? != resolved(&root)?.join(member) {
                return Err(format!(
                    "{t}: --bin {} is not the member {member} of its extraction root {}: a link on the way would leave the executable uncounted",
                    measured.display(),
                    root.display()
                ));
            }
            walk("archive", &root)?;
            if let Some(dir) = &runtime {
                walk("runtime", dir)?;
            }
            (
                0,
                0,
                "every regular file of the extraction root (the extracted archive), the executable counted once",
            )
        } else if let Some(dir) = &runtime {
            walk("runtime", dir)?;
            (
                size(measured)?,
                1,
                "the executable, its checked members and the whole pinned runtime tree",
            )
        } else {
            let mut sum = size(measured)?;
            let mut count = 1u64;
            for (path, _) in &spec.extra.members {
                if path != measured {
                    sum += size(path)?;
                    count += 1;
                }
            }
            (sum, count, "the executable and its checked members")
        };
        let installed = installed + trees.iter().map(|(_, _, w)| w.bytes).sum::<u64>();
        let files = files + trees.iter().map(|(_, _, w)| w.files).sum::<u64>();
        if installed == 0 {
            return Err(format!("{t}: the installed size is 0"));
        }
        entry.insert("installed_bytes".into(), installed.into());
        entry.insert("installed_files".into(), files.into());
        entry.insert("installed_basis".into(), basis.into());
        entry.insert(
            "installed_trees".into(),
            trees
                .iter()
                .map(|(kind, _, w)| {
                    serde_json::json!({
                        "kind": kind,
                        "files": w.files,
                        "bytes": w.bytes,
                        "sha256": w.sha256,
                    })
                })
                .collect::<Vec<_>>()
                .into(),
        );
        occupied.push(Occupied {
            target: t,
            executable: std::fs::canonicalize(measured)
                .map_err(|e| format!("{t}: {}: {e}", measured.display()))?,
            trees: trees
                .iter()
                .map(|(_, dir, _)| {
                    std::fs::canonicalize(dir).map_err(|e| format!("{t}: {}: {e}", dir.display()))
                })
                .collect::<Result<_, _>>()?,
        });
        // What the row needs that nothing pins is labelled, never zero.
        let unpinned = match t {
            TargetKind::MailslurperSourceBuild => Some("host glibc"),
            TargetKind::Smtp4dev => Some("host libraries of the self-contained .NET bundle"),
            _ => None,
        };
        if let Some(what) = unpinned {
            entry.insert(
                "unpinned_runtime".into(),
                format!("unpinned, not counted: {what}").into(),
            );
        }
        targets.push(serde_json::Value::Object(entry));
    }
    refuse_shared_trees(&occupied)?;
    let blocked: Vec<serde_json::Value> = p
        .does_not_start
        .iter()
        .flatten()
        .map(|d| {
            serde_json::json!({
                "target": d.target,
                "result": format!("does not start (published result: {})", d.cite),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "footprint_version": 1,
        "protocol_id": p.id,
        "targets": targets,
        "blocked": blocked,
    }))
}
