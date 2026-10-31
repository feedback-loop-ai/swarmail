//! The Linux syscall body of the boundary. Never executed in a delivery seat
//! (EPERM or ENOSPC from `clone`); proven live by `tests/trial_isolation.rs`.

use super::{
    BindPlan, Observed, Refusal, Request, SOURCE_INVALID, STEPS, Step, bind_plan, remount_flags,
};
use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const NEW_FLAGS: libc::c_int = libc::CLONE_NEWUSER
    | libc::CLONE_NEWNS
    | libc::CLONE_NEWPID
    | libc::CLONE_NEWNET
    | libc::CLONE_NEWIPC;
/// The staged root: bounded in bytes and inodes so a hostile agent fills
/// only its own trial's memory.
const ROOT_OPTIONS: &CStr = c"size=512m,nr_inodes=65536,mode=0755";
/// The descriptor the parent-liveness pipe is parked on while the rest of
/// the table is closed.
const PARENT_FD: libc::c_int = 3;
const IFF_UP: libc::c_short = 0x1;
const SIOCGIFFLAGS: u64 = 0x8913;
const SIOCSIFFLAGS: u64 = 0x8914;
const TMPFS_MAGIC: i64 = 0x0102_1994;
const CAPABILITY_VERSION_3: u32 = 0x2008_0522;

fn last_errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn io_errno(error: &std::io::Error) -> i32 {
    error.raw_os_error().unwrap_or(libc::EIO)
}

fn cstring(path: &Path) -> Result<CString, i32> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| libc::EINVAL)
}

fn mount(
    source: Option<&CStr>,
    target: &CStr,
    fstype: Option<&CStr>,
    flags: libc::c_ulong,
    data: Option<&CStr>,
) -> Result<(), i32> {
    // SAFETY: every pointer is a live NUL-terminated string or null.
    let rc = unsafe {
        libc::mount(
            source.map_or(std::ptr::null(), CStr::as_ptr),
            target.as_ptr(),
            fstype.map_or(std::ptr::null(), CStr::as_ptr),
            flags,
            data.map_or(std::ptr::null(), |d| d.as_ptr().cast()),
        )
    };
    if rc == 0 { Ok(()) } else { Err(last_errno()) }
}

/// What the init needs to build its root, fixed before the clone.
struct Setup {
    uid: u32,
    gid: u32,
    stage: PathBuf,
    plan: BindPlan,
    old_mask: libc::sigset_t,
    parent_read: libc::c_int,
    parent_write: libc::c_int,
}

/// Enter the boundary. In the controller this supervises the trial and never
/// returns; in the trial's init it returns `Ok(())` once the boundary is
/// built and verified; on any failure, in either process, it returns the
/// refusal and nothing was launched.
pub fn enter(request: &Request) -> Result<(), Refusal> {
    let plan = bind_plan(&request.sources()).map_err(|_| Refusal::Named(SOURCE_INVALID))?;
    let admission = crate::admission::Admission::acquire().map_err(Refusal::Named)?;
    // SAFETY: geteuid and getegid have no preconditions.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let stage = tempfile::Builder::new()
        .prefix("isolated-")
        .tempdir_in(crate::admission::Admission::root_for(uid))
        .map_err(|e| Refusal::errno("stage_dir", io_errno(&e)))?;

    // Block the signals the controller supervises before the clone, so none
    // can arrive between the clone and the sigwait loop; the init inherits
    // the mask and restores it as its last step.
    // SAFETY: plain signal-set manipulation on local storage.
    let (set, old_mask) = unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        let mut old: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for sig in [libc::SIGTERM, libc::SIGINT, libc::SIGCHLD] {
            libc::sigaddset(&mut set, sig);
        }
        if libc::sigprocmask(libc::SIG_BLOCK, &set, &mut old) != 0 {
            return Err(Refusal::errno("sigprocmask", last_errno()));
        }
        (set, old)
    };
    let restore = |mask: &libc::sigset_t| {
        // SAFETY: restoring the mask saved above.
        unsafe { libc::sigprocmask(libc::SIG_SETMASK, mask, std::ptr::null_mut()) };
    };
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: `fds` has room for the two descriptors.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        let errno = last_errno();
        restore(&old_mask);
        return Err(Refusal::errno("pipe", errno));
    }
    let setup = Setup {
        uid,
        gid,
        stage: stage.path().to_path_buf(),
        plan,
        old_mask,
        parent_read: fds[0],
        parent_write: fds[1],
    };

    // SAFETY: the process is single-threaded here (no runtime exists yet), so
    // this raw clone is a fork with namespace flags: the child shares nothing
    // mutable with the parent.
    let pid = unsafe { clone_namespaces() };
    if pid < 0 {
        let errno = last_errno();
        // SAFETY: closing the two descriptors created above.
        unsafe {
            libc::close(setup.parent_read);
            libc::close(setup.parent_write);
        }
        restore(&setup.old_mask);
        return Err(Refusal::errno("clone", errno));
    }
    if pid == 0 {
        // The init never drops these: the controller owns the stage
        // directory and the host admission.
        std::mem::forget(stage);
        std::mem::forget(admission);
        return init(&setup);
    }
    // The controller is nondumpable only after the clone: the init must
    // still be able to write its own identity maps.
    // SAFETY: prctl with integer arguments; closing the parent's read end.
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        libc::close(setup.parent_read);
    }
    let disposition = supervise(pid as libc::pid_t, &set);
    // SAFETY: closing the write end whose closure signalled liveness.
    unsafe { libc::close(setup.parent_write) };
    drop(stage);
    drop(admission);
    finish(disposition)
}

#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "x86",
    target_arch = "arm",
    target_arch = "riscv64"
))]
unsafe fn clone_namespaces() -> libc::c_long {
    // SAFETY: flags only; the stack, tid and tls arguments are unused (zero)
    // in the argument order of every architecture listed above.
    unsafe {
        libc::syscall(
            libc::SYS_clone,
            (NEW_FLAGS | libc::SIGCHLD) as libc::c_long,
            0,
            0,
            0,
            0,
        )
    }
}

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "x86",
    target_arch = "arm",
    target_arch = "riscv64"
)))]
unsafe fn clone_namespaces() -> libc::c_long {
    // The raw clone argument order differs on this architecture and has not
    // been proven: refuse rather than guess.
    // SAFETY: setting errno has no preconditions.
    unsafe { *libc::__errno_location() = libc::ENOSYS };
    -1
}

enum Disposition {
    Exited(i32),
    Signalled(i32),
}

/// The controller's loop: forward `TERM` and `INT` to the init, reap it, and
/// report how it ended.
fn supervise(child: libc::pid_t, set: &libc::sigset_t) -> Disposition {
    loop {
        let mut sig = 0;
        // SAFETY: `set` and `sig` are live; the set is blocked in this
        // thread, which is what sigwait requires.
        if unsafe { libc::sigwait(set, &mut sig) } != 0 {
            // SAFETY: killing the child this process created.
            unsafe { libc::kill(child, libc::SIGKILL) };
            continue;
        }
        if sig == libc::SIGTERM || sig == libc::SIGINT {
            // SAFETY: forwarding to the child this process created.
            unsafe { libc::kill(child, sig) };
            continue;
        }
        let mut status = 0;
        // SAFETY: reaping the one child; `status` is live.
        let reaped = unsafe { libc::waitpid(child, &mut status, libc::WNOHANG) };
        if reaped == child {
            return if libc::WIFSIGNALED(status) {
                Disposition::Signalled(libc::WTERMSIG(status))
            } else {
                Disposition::Exited(libc::WEXITSTATUS(status))
            };
        }
        if reaped < 0 && last_errno() != libc::EINTR {
            return Disposition::Exited(2);
        }
    }
}

/// End the controller the way the trial ended: its exit code for an exit,
/// its signal re-raised for a signal.
fn finish(disposition: Disposition) -> ! {
    match disposition {
        Disposition::Exited(code) => std::process::exit(code),
        Disposition::Signalled(sig) => {
            // SAFETY: restoring the default action, unblocking, and raising
            // the signal the trial died of.
            unsafe {
                libc::signal(sig, libc::SIG_DFL);
                let mut set: libc::sigset_t = std::mem::zeroed();
                libc::sigemptyset(&mut set);
                libc::sigaddset(&mut set, sig);
                libc::sigprocmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut());
                libc::raise(sig);
            }
            std::process::exit(128 + sig)
        }
    }
}

enum Failure {
    Errno(i32),
    Check(&'static str),
}

impl From<i32> for Failure {
    fn from(errno: i32) -> Self {
        Self::Errno(errno)
    }
}

/// The trial's PID 1: run every step in [`STEPS`] order.
fn init(setup: &Setup) -> Result<(), Refusal> {
    for step in STEPS {
        run(step, setup).map_err(|failure| match failure {
            Failure::Errno(errno) => Refusal::errno(step.op(), errno),
            Failure::Check(check) => Refusal::errno(check, libc::EINVAL),
        })?;
    }
    Ok(())
}

fn run(step: Step, setup: &Setup) -> Result<(), Failure> {
    match step {
        Step::Identity => identity(setup)?,
        Step::MountPrivate => mount(None, c"/", None, libc::MS_REC | libc::MS_PRIVATE, None)?,
        Step::StageRoot => stage_root(setup)?,
        Step::Binds => binds(setup)?,
        Step::Proc => proc_mount(setup)?,
        Step::Pivot => {
            std::env::set_current_dir(&setup.stage).map_err(|e| io_errno(&e))?;
            // SAFETY: pivot_root(".", "."): the staged root becomes the root
            // and the old root is stacked beneath it.
            let rc = unsafe { libc::syscall(libc::SYS_pivot_root, c".".as_ptr(), c".".as_ptr()) };
            if rc != 0 {
                return Err(last_errno().into());
            }
        }
        Step::Detach => {
            // SAFETY: lazily unmounting the stacked old root at ".".
            if unsafe { libc::umount2(c".".as_ptr(), libc::MNT_DETACH) } != 0 {
                return Err(last_errno().into());
            }
            std::env::set_current_dir("/").map_err(|e| io_errno(&e))?;
        }
        Step::Loopback => loopback()?,
        Step::CloseRange => close_range(setup)?,
        Step::NoNewPrivs => prctl(libc::PR_SET_NO_NEW_PRIVS, 1)?,
        Step::CapBound => cap_bound()?,
        Step::Capset => capset()?,
        Step::Dumpable => prctl(libc::PR_SET_DUMPABLE, 0)?,
        Step::ParentDeath => parent_death()?,
        Step::SelfCheck => self_check()?,
        Step::Unblock => {
            // SAFETY: restoring the mask the controller saved.
            let rc = unsafe {
                libc::sigprocmask(libc::SIG_SETMASK, &setup.old_mask, std::ptr::null_mut())
            };
            if rc != 0 {
                return Err(last_errno().into());
            }
        }
    }
    Ok(())
}

fn prctl(option: libc::c_int, arg: libc::c_ulong) -> Result<(), i32> {
    // SAFETY: prctl with integer arguments.
    if unsafe { libc::prctl(option, arg, 0, 0, 0) } != 0 {
        return Err(last_errno());
    }
    Ok(())
}

fn write_proc(path: &str, text: &str) -> Result<(), i32> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| io_errno(&e))?;
    file.write_all(text.as_bytes()).map_err(|e| io_errno(&e))
}

/// Map the initiating uid and gid to themselves and deny `setgroups`: the
/// trial is the same principal to the host, not a root inside.
fn identity(setup: &Setup) -> Result<(), i32> {
    write_proc("/proc/self/setgroups", "deny")?;
    write_proc("/proc/self/uid_map", &format!("{0} {0} 1\n", setup.uid))?;
    write_proc("/proc/self/gid_map", &format!("{0} {0} 1\n", setup.gid))
}

fn stage_root(setup: &Setup) -> Result<(), i32> {
    mount(
        Some(c"tmpfs"),
        &cstring(&setup.stage)?,
        Some(c"tmpfs"),
        libc::MS_NOSUID | libc::MS_NODEV,
        Some(ROOT_OPTIONS),
    )?;
    let tmp = setup.stage.join("tmp");
    std::fs::create_dir(&tmp).map_err(|e| io_errno(&e))?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o1777))
        .map_err(|e| io_errno(&e))
}

/// Where the host path appears inside the staged root.
fn inside(setup: &Setup, host: &Path) -> PathBuf {
    setup.stage.join(host.strip_prefix("/").unwrap_or(host))
}

fn binds(setup: &Setup) -> Result<(), i32> {
    for root in &setup.plan.system {
        let Ok(meta) = std::fs::symlink_metadata(root) else {
            continue;
        };
        let target = inside(setup, root);
        if meta.file_type().is_symlink() {
            let link = std::fs::read_link(root).map_err(|e| io_errno(&e))?;
            std::os::unix::fs::symlink(link, &target).map_err(|e| io_errno(&e))?;
        } else {
            bind_read_only(root, &target, true, false)?;
        }
    }
    std::fs::create_dir_all(inside(setup, Path::new("/dev"))).map_err(|e| io_errno(&e))?;
    for device in &setup.plan.devices {
        if device.exists() {
            bind_read_only(device, &inside(setup, device), false, true)?;
        }
    }
    for file in &setup.plan.files {
        bind_read_only(file, &inside(setup, file), false, false)?;
    }
    Ok(())
}

/// Bind `source` at `target` read-only and nosuid, carrying forward the flags
/// the kernel locked on the copied mount. The bind is not recursive: a mount
/// below an allowlisted tree is absent, never a writable window.
fn bind_read_only(source: &Path, target: &Path, dir: bool, device: bool) -> Result<(), i32> {
    if dir {
        std::fs::create_dir_all(target).map_err(|e| io_errno(&e))?;
    } else {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_errno(&e))?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .map_err(|e| io_errno(&e))?;
    }
    let (src, dst) = (cstring(source)?, cstring(target)?);
    mount(Some(&src), &dst, None, libc::MS_BIND, None)?;
    // SAFETY: `stat` is plain-old-data and `dst` is a live C string.
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: as above.
    if unsafe { libc::statvfs(dst.as_ptr(), &mut stat) } != 0 {
        return Err(last_errno());
    }
    let flags = remount_flags(stat.f_flag as u64, device);
    mount(None, &dst, None, flags as libc::c_ulong, None)
}

/// A fresh `/proc` for the new PID namespace, mounted before the pivot while
/// the host `/proc` is still fully visible (the kernel refuses otherwise).
fn proc_mount(setup: &Setup) -> Result<(), i32> {
    let target = inside(setup, Path::new("/proc"));
    std::fs::create_dir(&target).map_err(|e| io_errno(&e))?;
    mount(
        Some(c"proc"),
        &cstring(&target)?,
        Some(c"proc"),
        libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
        None,
    )
}

/// The kernel's `struct ifreq` as far as the flags ioctls read it.
#[repr(C)]
struct IfReq {
    name: [libc::c_char; libc::IFNAMSIZ],
    flags: libc::c_short,
    pad: [u8; 22],
}

fn loopback() -> Result<(), i32> {
    // SAFETY: a plain datagram socket used only for the two ioctls.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(last_errno());
    }
    let mut request = IfReq {
        name: [0; libc::IFNAMSIZ],
        flags: 0,
        pad: [0; 22],
    };
    request.name[0] = b'l' as libc::c_char;
    request.name[1] = b'o' as libc::c_char;
    let mut up = || {
        // SAFETY: `request` matches the kernel's ifreq layout for flags.
        if unsafe { libc::ioctl(fd, SIOCGIFFLAGS as _, &mut request) } != 0 {
            return Err(last_errno());
        }
        request.flags |= IFF_UP;
        // SAFETY: as above.
        if unsafe { libc::ioctl(fd, SIOCSIFFLAGS as _, &mut request) } != 0 {
            return Err(last_errno());
        }
        Ok(())
    };
    let result = up();
    // SAFETY: closing the socket this function opened.
    unsafe { libc::close(fd) };
    result
}

/// Park the parent-liveness pipe on [`PARENT_FD`] and close everything else
/// from there up, including the init's copy of the pipe's write end (which
/// would keep the pipe alive after the controller dies).
fn close_range(setup: &Setup) -> Result<(), i32> {
    // SAFETY: closing and duplicating descriptors this process owns.
    unsafe {
        libc::close(setup.parent_write);
        if libc::dup3(setup.parent_read, PARENT_FD, libc::O_CLOEXEC) < 0 {
            return Err(last_errno());
        }
        let rc = libc::syscall(
            libc::SYS_close_range,
            (PARENT_FD + 1) as libc::c_uint,
            libc::c_uint::MAX,
            0,
        );
        if rc != 0 {
            return Err(last_errno());
        }
    }
    Ok(())
}

fn cap_bound() -> Result<(), i32> {
    for cap in 0..64 {
        // SAFETY: prctl with integer arguments.
        if unsafe { libc::prctl(libc::PR_CAPBSET_READ, cap, 0, 0, 0) } < 0 {
            break;
        }
        prctl(libc::PR_CAPBSET_DROP, cap as libc::c_ulong)?;
    }
    // SAFETY: as above.
    let rc = unsafe {
        libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        )
    };
    if rc != 0 {
        return Err(last_errno());
    }
    Ok(())
}

#[repr(C)]
struct CapHeader {
    version: u32,
    pid: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

fn capset() -> Result<(), i32> {
    let header = CapHeader {
        version: CAPABILITY_VERSION_3,
        pid: 0,
    };
    let data = [CapData {
        effective: 0,
        permitted: 0,
        inheritable: 0,
    }; 2];
    // SAFETY: the two-element data array is the version-3 layout.
    if unsafe { libc::syscall(libc::SYS_capset, &header, data.as_ptr()) } != 0 {
        return Err(last_errno());
    }
    Ok(())
}

/// Ask for `SIGKILL` when the controller dies, then prove it has not already
/// died: the read end of the liveness pipe reports hang-up if it has.
fn parent_death() -> Result<(), Failure> {
    prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong)?;
    let mut poll = libc::pollfd {
        fd: PARENT_FD,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one live pollfd, zero timeout.
    let ready = unsafe { libc::poll(&mut poll, 1, 0) };
    let errno = last_errno();
    // SAFETY: the parking descriptor is closed exactly once.
    unsafe { libc::close(PARENT_FD) };
    if ready < 0 {
        return Err(errno.into());
    }
    if poll.revents & (libc::POLLHUP | libc::POLLERR) != 0 {
        return Err(Failure::Check("parent_death"));
    }
    Ok(())
}

fn self_check() -> Result<(), Failure> {
    let proc_pids: Vec<u32> = std::fs::read_dir("/proc")
        .map_err(|e| io_errno(&e))?
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
        .collect();
    let net_dev = std::fs::read_to_string("/proc/net/dev").map_err(|e| io_errno(&e))?;
    let status = std::fs::read_to_string("/proc/self/status").map_err(|e| io_errno(&e))?;
    // SAFETY: `stat` is plain-old-data and the path is a literal.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: as above.
    if unsafe { libc::statfs(c"/".as_ptr(), &mut stat) } != 0 {
        return Err(last_errno().into());
    }
    let observed = Observed {
        // SAFETY: getpid has no preconditions.
        pid: unsafe { libc::getpid() } as u32,
        proc_pids: &proc_pids,
        net_dev: &net_dev,
        status: &status,
        root_is_tmpfs: stat.f_type as i64 == TMPFS_MAGIC,
    };
    super::verify(&observed).map_err(Failure::Check)
}
