//! Test-owned process sessions (Part of #408). A baseline command a test
//! starts runs as the leader of a fresh session, so every process it starts
//! (its trial worker, the trial's `serve` and `fixture`, an unsandboxed
//! agent) stays attributable to the test after reparenting: the session id
//! survives the death of any member. Under decision 0076 a trial's service
//! and fixture end with it; a test that kills a worker on purpose (0137's
//! forced death) takes over that ownership, so it must end what the worker
//! can no longer reap. The census here is the independent check that it did.
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use super::{ProcessIdentity, process_identity, survivors};

/// How long one cleanup may take: every member's exit, observed through its
/// pidfd, inside one shared deadline. A passing cleanup returns at the exits.
#[cfg(target_os = "linux")]
const CLEANUP_BOUND: Duration = Duration::from_secs(5);

/// Start `command`'s child as the leader of a new session.
pub fn isolate(command: &mut Command) -> &mut Command {
    // SAFETY: setsid(2) is async-signal-safe and touches no memory.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
}

/// The session a test's command leads. Dropping it is the survivor check:
/// a live member fails the test by name after it is ended; on unwind the
/// members are only ended, so an earlier failure keeps its own message.
pub struct OwnedSession {
    #[cfg(target_os = "linux")]
    leader: ProcessIdentity,
    what: String,
}

/// The live trial a held launch shows: the worker under the leader, its
/// `serve` and `fixture`, their private scratch root and the worker's own
/// runtime root.
pub struct LiveTrial {
    #[cfg(target_os = "linux")]
    pub worker: ProcessIdentity,
    #[cfg(target_os = "linux")]
    pub servers: [ProcessIdentity; 2],
    pub scratch: PathBuf,
    pub runtime: PathBuf,
}

impl OwnedSession {
    /// Own the session `pid` leads, with `what` naming it in diagnostics.
    /// Call it right after the spawn: the unreaped child is readable.
    pub fn of(pid: u32, what: &str) -> Self {
        #[cfg(target_os = "linux")]
        {
            let leader = process_identity(pid as i32)
                .unwrap_or_else(|| panic!("{what}: no ownership evidence for leader {pid}"));
            assert_eq!(
                leader.session, leader.pid,
                "{what}: {pid} does not lead its session"
            );
            Self {
                leader,
                what: what.into(),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = pid;
            Self { what: what.into() }
        }
    }

    /// Fail, naming every survivor, when any member is still live; the
    /// survivors are ended before the panic. A no-op off Linux, where every
    /// caller that starts a trial is ignored.
    pub fn assert_clear(&self) {
        #[cfg(target_os = "linux")]
        {
            let live = self
                .members()
                .unwrap_or_else(|e| panic!("{}: {e}", self.what));
            if !live.is_empty() {
                let report = survivors(&live);
                let rescue = self.end_all().err().unwrap_or_default();
                panic!(
                    "{}: owned processes outlived it\n{report}\n{rescue}",
                    self.what
                );
            }
        }
    }

    /// Wait, bounded by `bound`, for every member to exit on its own (a
    /// worker finishing its owned cleanup), then check the session clear.
    pub fn wait_clear(&self, bound: std::time::Duration) {
        #[cfg(target_os = "linux")]
        {
            let deadline = Instant::now() + bound;
            loop {
                let live = self
                    .members()
                    .unwrap_or_else(|e| panic!("{}: {e}", self.what));
                if live.is_empty() || Instant::now() >= deadline {
                    break;
                }
                for member in &live {
                    if let Some(fd) = open_verified(member) {
                        await_exit(&fd, deadline);
                    }
                }
            }
            self.assert_clear();
        }
        #[cfg(not(target_os = "linux"))]
        let _ = bound;
    }

    /// End every member: the cleanup a test owes once it has killed the
    /// worker that owned them. Fails when one cannot be ended in time.
    pub fn terminate(&self) {
        #[cfg(target_os = "linux")]
        if let Err(e) = self.end_all() {
            panic!("{}: {e}", self.what);
        }
    }
}

#[cfg(target_os = "linux")]
impl OwnedSession {
    /// Every live member: each process in the leader's session started no
    /// earlier than the leader, and their descendants (an agent's sandbox
    /// opens a session of its own). An unreadable process table, or a
    /// different process leading a session under the leader's id, is missing
    /// ownership evidence and refuses.
    pub fn members(&self) -> Result<Vec<ProcessIdentity>, String> {
        let all = table()?;
        let start = self.leader.start_time;
        if all
            .iter()
            .any(|p| p.pid == self.leader.pid && p.start_time != start && p.session == p.pid)
        {
            return Err(format!(
                "session {} is led by another process: ownership ambiguous",
                self.leader.pid
            ));
        }
        let mut owned: std::collections::HashSet<i32> = all
            .iter()
            .filter(|p| p.session == self.leader.pid && p.start_time >= start)
            .map(|p| p.pid)
            .collect();
        loop {
            let before = owned.len();
            for p in &all {
                if owned.contains(&p.parent) && p.start_time >= start {
                    owned.insert(p.pid);
                }
            }
            if owned.len() == before {
                break;
            }
        }
        Ok(all
            .into_iter()
            .filter(|p| owned.contains(&p.pid) && !matches!(p.state, 'Z' | 'X'))
            .collect())
    }

    /// SIGKILL every member through a verified pidfd and observe each exit
    /// inside one shared deadline; rescan for members started meanwhile.
    fn end_all(&self) -> Result<(), String> {
        let deadline = Instant::now() + CLEANUP_BOUND;
        loop {
            let live = self.members()?;
            if live.is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "members still live after {CLEANUP_BOUND:?}:\n{}",
                    survivors(&live)
                ));
            }
            let fds: Vec<_> = live.iter().filter_map(kill_verified).collect();
            for fd in &fds {
                await_exit(fd, deadline);
            }
        }
    }

    /// The trial a held launch shows, verified before anything is killed:
    /// one `trial` worker under the leader, its `serve` (the candidate) and
    /// `fixture` (this harness) children, live in sibling role directories
    /// of one private scratch root under the shared temporary directory.
    pub fn live_trial(&self) -> LiveTrial {
        let live = self
            .members()
            .unwrap_or_else(|e| panic!("{}: {e}", self.what));
        let fail = |why: &str| -> ! { panic!("{}: {why}\n{}", self.what, survivors(&live)) };
        let canonical = |p: &std::path::Path| std::fs::canonicalize(p).unwrap();
        let harness = canonical(std::path::Path::new(super::exe()));
        let candidate = canonical(super::swarmail_bin());
        let one = |exe: &PathBuf, sub: &str, parent: i32| -> ProcessIdentity {
            let found: Vec<_> = live
                .iter()
                .filter(|p| p.parent == parent && role(p.pid) == Some((exe.clone(), sub.into())))
                .collect();
            match found.as_slice() {
                [p] => (*p).clone(),
                _ => fail(&format!("expected one live `{sub}` under {parent}")),
            }
        };
        let worker = one(&harness, "trial", self.leader.pid);
        let serve = one(&candidate, "serve", worker.pid);
        let fixture = one(&harness, "fixture", worker.pid);
        let cwd = |p: &ProcessIdentity| std::fs::read_link(format!("/proc/{}/cwd", p.pid)).ok();
        let (Some(a), Some(b)) = (cwd(&serve), cwd(&fixture)) else {
            fail("a server's working directory is unreadable")
        };
        let scratch = match (a.parent(), b.parent()) {
            (Some(x), Some(y)) if x == y && a.ends_with("swarmail") && b.ends_with("fixture") => {
                x.to_path_buf()
            }
            _ => fail("the servers do not share one trial scratch root"),
        };
        private_root(&scratch, "swarmail-eval-").unwrap_or_else(|e| fail(&e));
        // The coordinator's runtime root for the worker: its HOME is
        // `<root>/worker/home`.
        let runtime = std::fs::read(format!("/proc/{}/environ", worker.pid))
            .ok()
            .and_then(|raw| {
                raw.split(|b| *b == 0)
                    .find_map(|e| e.strip_prefix(b"HOME=").map(<[u8]>::to_vec))
            })
            .map(|home| PathBuf::from(std::ffi::OsString::from_vec(home)))
            .and_then(|home| Some(home.parent()?.parent()?.to_path_buf()))
            .unwrap_or_else(|| fail("the worker's runtime root is unreadable"));
        private_root(&runtime, "swarmail-eval-matrix-").unwrap_or_else(|e| fail(&e));
        LiveTrial {
            worker,
            servers: [serve, fixture],
            scratch,
            runtime,
        }
    }

    /// SIGKILL `trial`'s worker, pinned by identity, as a forced death.
    pub fn kill_worker(&self, trial: &LiveTrial) {
        if kill_verified(&trial.worker).is_none() {
            panic!(
                "{}: the worker could not be signalled\n{}",
                self.what,
                survivors(std::slice::from_ref(&trial.worker))
            );
        }
    }

    /// Remove `trial`'s abandoned scratch root once nothing owned is live.
    pub fn dispose_scratch(&self, trial: &LiveTrial) {
        self.assert_clear();
        std::fs::remove_dir_all(&trial.scratch).unwrap();
    }

    /// Remove the worker's runtime root its killed coordinator abandoned,
    /// once nothing owned is live.
    pub fn dispose_runtime(&self, trial: &LiveTrial) {
        self.assert_clear();
        std::fs::remove_dir_all(&trial.runtime).unwrap();
    }
}

/// `path` is, by its resolved name, a `prefix`-named private directory
/// directly under the shared temporary directory.
#[cfg(target_os = "linux")]
fn private_root(path: &std::path::Path, prefix: &str) -> Result<(), String> {
    use swarmail_evals::{artifacts, runner};
    let tmp = artifacts::resolve_root(std::path::Path::new(runner::SHARED_TMP))?;
    let resolved = artifacts::resolve_root(path)?;
    artifacts::safe_root(&resolved)?;
    let named = resolved
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with(prefix));
    if resolved != path || resolved.parent() != Some(tmp.as_path()) || !named {
        return Err(format!("unexpected private root {}", path.display()));
    }
    Ok(())
}

/// Off Linux the trial-killing test is ignored; these keep it compiling.
#[cfg(not(target_os = "linux"))]
impl OwnedSession {
    pub fn live_trial(&self) -> LiveTrial {
        unreachable!("{}: the process census reads /proc (Linux)", self.what)
    }
    pub fn kill_worker(&self, _trial: &LiveTrial) {
        unreachable!("{}: the process census reads /proc (Linux)", self.what)
    }
    pub fn dispose_scratch(&self, _trial: &LiveTrial) {
        unreachable!("{}: the process census reads /proc (Linux)", self.what)
    }
    pub fn dispose_runtime(&self, _trial: &LiveTrial) {
        unreachable!("{}: the process census reads /proc (Linux)", self.what)
    }
}

#[cfg(target_os = "linux")]
impl Drop for OwnedSession {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _ = self.end_all();
            return;
        }
        self.assert_clear();
    }
}

/// Every process in the table; an unreadable table refuses.
#[cfg(target_os = "linux")]
fn table() -> Result<Vec<ProcessIdentity>, String> {
    let entries = std::fs::read_dir("/proc").map_err(|e| format!("process table: {e}"))?;
    Ok(entries
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<i32>().ok())
        .filter_map(process_identity)
        .collect())
}

/// The executable and subcommand of `pid`.
#[cfg(target_os = "linux")]
fn role(pid: i32) -> Option<(PathBuf, String)> {
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let sub = cmdline.split(|b| *b == 0).nth(1)?;
    Some((exe, String::from_utf8_lossy(sub).into_owned()))
}

/// A pidfd for `member`, kept only if the pid still names that live
/// process (same start time and session) after the open: no reused pid is
/// ever waited on or signalled.
#[cfg(target_os = "linux")]
fn open_verified(member: &ProcessIdentity) -> Option<std::os::fd::OwnedFd> {
    use std::os::fd::FromRawFd;
    // SAFETY: pidfd_open takes a pid and flags and returns a new descriptor.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, member.pid, 0) };
    if raw < 0 {
        return None;
    }
    // SAFETY: the descriptor was just returned to us and is owned here.
    let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(raw as i32) };
    process_identity(member.pid)
        .filter(|now| {
            now.start_time == member.start_time
                && now.session == member.session
                && !matches!(now.state, 'Z' | 'X')
        })
        .map(|_| fd)
}

/// SIGKILL `member` through a verified pidfd; the descriptor to await.
#[cfg(target_os = "linux")]
fn kill_verified(member: &ProcessIdentity) -> Option<std::os::fd::OwnedFd> {
    use std::os::fd::AsRawFd;
    let fd = open_verified(member)?;
    // SAFETY: pidfd_send_signal on a descriptor this function owns.
    let rc = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            libc::SIGKILL,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    (rc == 0).then_some(fd)
}

/// Block until the pidfd reports its process's exit or `deadline` passes.
#[cfg(target_os = "linux")]
fn await_exit(fd: &std::os::fd::OwnedFd, deadline: Instant) {
    use std::os::fd::AsRawFd;
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let left = deadline.saturating_duration_since(Instant::now());
    let ms = left.as_millis().min(i32::MAX as u128) as i32;
    // SAFETY: one valid pollfd for the call's duration.
    unsafe { libc::poll(&mut poll, 1, ms) };
}
