//! The host route's launch authority (the panel's hold 2 on a73ac17a, part
//! of #404). A `--paid-launch` binding in argv is a request, never a
//! permission: anyone can compute a selection's identity. A paid wrapper on
//! the host launches only after the live baseline coordinator answers that
//! request on a private endpoint it handed down the worker chain.
//!
//! The coordinator makes one `SOCK_SEQPACKET` pair per expected agent of a
//! paid slot and places the worker's ends at descriptors
//! [`GRANT_FD`]`..`; the worker gives agent `i` its own end at
//! [`GRANT_FD`]. Each end answers exactly one request. Both sides read the
//! kernel's `SCM_CREDENTIALS` of the message they receive, so a reply
//! counts only when it was sent by a process that is an ancestor of the
//! wrapper and runs this very executable, and a request only when it was
//! sent by a descendant of the coordinator's own worker running it. A
//! forged identity string, a caller-made socket or a relaying script admits
//! nothing; a coordinator that is gone is an end of file, a refusal. Linux
//! is the one platform with that peer evidence; elsewhere a paid binding
//! refuses (`paid_grant_unsupported`).
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::fd::{OwnedFd, RawFd};

/// The descriptor a paid wrapper finds its grant endpoint on; a worker
/// receives the endpoints of its agents from here on.
pub const GRANT_FD: RawFd = 4;
/// The most endpoints a worker can hold: the largest `agent_count`.
pub const MAX_ENDPOINTS: usize = 3;
/// The coordinator's one positive answer.
pub const GRANTED: &[u8] = b"granted\n";
#[cfg(target_os = "linux")]
const REFUSED: &[u8] = b"refused\n";
/// The longest request: a digest, `:`, a Unix second.
#[cfg(target_os = "linux")]
const REQUEST_LIMIT: usize = 128;
/// How long a wrapper waits for its coordinator.
#[cfg(target_os = "linux")]
const WAIT_SECS: libc::time_t = 10;

/// Mark every descriptor an endpoint may arrive on close-on-exec, first
/// thing in every process of this executable: a worker's servers and a
/// wrapper's CLI never inherit one. The process itself keeps them.
pub fn seal_inherited() {
    for fd in GRANT_FD..GRANT_FD + MAX_ENDPOINTS as RawFd {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
}

/// The paid request a wrapper sends: its binding, verbatim.
pub fn want(cli_sha256: &str, valid_through: u64) -> String {
    format!("{cli_sha256}:{valid_through}")
}

/// Place `ends` at [`GRANT_FD`]`..` in a child, after fork and before exec:
/// first above every target, then down, so no end overwrites another.
///
/// # Safety
/// Only async-signal-safe calls; for a `pre_exec` closure.
pub unsafe fn place(ends: &[RawFd]) -> std::io::Result<()> {
    let mut high = [0 as RawFd; MAX_ENDPOINTS];
    for (i, fd) in ends.iter().enumerate() {
        let h = unsafe { libc::fcntl(*fd, libc::F_DUPFD_CLOEXEC, 64) };
        if h < 0 {
            return Err(std::io::Error::last_os_error());
        }
        high[i] = h;
    }
    for (i, h) in high.iter().take(ends.len()).enumerate() {
        if unsafe { libc::dup2(*h, GRANT_FD + i as RawFd) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        unsafe { libc::close(*h) };
    }
    Ok(())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::os::unix::fs::MetadataExt as _;

    pub fn pair() -> std::io::Result<(OwnedFd, OwnedFd)> {
        let mut fds = [0 as RawFd; 2];
        let r = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        };
        if r != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let ends = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        passcred(&ends.0)?;
        passcred(&ends.1)?;
        Ok(ends)
    }
    pub fn passcred(fd: &impl AsRawFd) -> std::io::Result<()> {
        let on: libc::c_int = 1;
        let r = unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&on as *const libc::c_int).cast(),
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if r == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    fn sockopt(fd: RawFd, name: libc::c_int) -> Option<libc::c_int> {
        let mut v: libc::c_int = 0;
        let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        let r = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                name,
                (&mut v as *mut libc::c_int).cast(),
                &mut len,
            )
        };
        (r == 0).then_some(v)
    }
    /// Whether `fd` is a local sequenced-packet socket: what an endpoint is.
    pub fn is_endpoint(fd: RawFd) -> bool {
        sockopt(fd, libc::SO_DOMAIN) == Some(libc::AF_UNIX)
            && sockopt(fd, libc::SO_TYPE) == Some(libc::SOCK_SEQPACKET)
    }
    /// One message and the kernel's credentials of its sender.
    pub fn receive(fd: RawFd, buf: &mut [u8]) -> Result<(usize, libc::ucred), &'static str> {
        let mut iov = libc::iovec {
            iov_base: buf.as_mut_ptr().cast(),
            iov_len: buf.len(),
        };
        let mut control = [0u64; 16];
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = std::mem::size_of_val(&control) as _;
        let n = loop {
            let n = unsafe { libc::recvmsg(fd, &mut msg, libc::MSG_CMSG_CLOEXEC) };
            if n >= 0 || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                break n;
            }
        };
        if n <= 0 {
            return Err("paid_grant_channel_closed");
        }
        if msg.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0 {
            return Err("paid_grant_message_invalid");
        }
        let mut creds = None;
        unsafe {
            let mut c = libc::CMSG_FIRSTHDR(&msg);
            while !c.is_null() {
                if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_CREDENTIALS {
                    creds = Some(std::ptr::read_unaligned(
                        libc::CMSG_DATA(c) as *const libc::ucred
                    ));
                }
                c = libc::CMSG_NXTHDR(&msg, c);
            }
        }
        creds
            .map(|c| (n as usize, c))
            .ok_or("paid_grant_sender_unknown")
    }
    pub fn send(fd: RawFd, bytes: &[u8]) -> Result<(), &'static str> {
        let n = unsafe { libc::send(fd, bytes.as_ptr().cast(), bytes.len(), libc::MSG_NOSIGNAL) };
        if n == bytes.len() as isize {
            Ok(())
        } else {
            Err("paid_grant_channel_closed")
        }
    }
    fn parent_of(pid: libc::pid_t) -> Option<libc::pid_t> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // `pid (comm) state ppid ...`: comm may hold spaces and parens.
        stat.rsplit_once(')')?
            .1
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    }
    /// Whether `ancestor` is a proper ancestor of `pid` (bounded walk).
    pub fn descends(pid: libc::pid_t, ancestor: libc::pid_t) -> bool {
        let mut at = pid;
        for _ in 0..64 {
            match parent_of(at) {
                Some(p) if p == ancestor => return true,
                Some(p) if p > 1 => at = p,
                _ => return false,
            }
        }
        false
    }
    /// Whether process `pid` runs this executable (same file, not a copy).
    pub fn runs_this(pid: libc::pid_t) -> bool {
        match (
            std::fs::metadata(format!("/proc/{pid}/exe")),
            std::fs::metadata("/proc/self/exe"),
        ) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    pub fn bound_wait(fd: RawFd) {
        let tv = libc::timeval {
            tv_sec: WAIT_SECS,
            tv_usec: 0,
        };
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                (&tv as *const libc::timeval).cast(),
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            )
        };
    }
}

/// The coordinator's side: `n` endpoint pairs, the coordinator's end first.
#[cfg(target_os = "linux")]
pub fn endpoints(n: usize) -> Result<Vec<(OwnedFd, OwnedFd)>, String> {
    if n == 0 || n > MAX_ENDPOINTS {
        return Err("paid_grant_endpoints_invalid".into());
    }
    (0..n)
        .map(|_| linux::pair().map_err(|_| "paid_grant_unavailable".to_string()))
        .collect()
}
#[cfg(not(target_os = "linux"))]
pub fn endpoints(_n: usize) -> Result<Vec<(OwnedFd, OwnedFd)>, String> {
    Err("paid_grant_unsupported".into())
}

/// Answer one endpoint once: grant only a request for exactly `want`, sent
/// by a process that descends from `worker` and runs this executable.
/// Returns whether it granted. Blocks until the request or the end of
/// file that follows the worker's and its agents' exits.
#[cfg(target_os = "linux")]
pub fn serve(end: OwnedFd, worker: u32, want: &str) -> bool {
    let mut buf = [0u8; REQUEST_LIMIT];
    let Ok((n, creds)) = linux::receive(end.as_raw_fd(), &mut buf) else {
        return false;
    };
    let ok = &buf[..n] == want.as_bytes()
        && linux::descends(creds.pid, worker as libc::pid_t)
        && linux::runs_this(creds.pid);
    let _ = linux::send(end.as_raw_fd(), if ok { GRANTED } else { REFUSED });
    ok
}
#[cfg(not(target_os = "linux"))]
pub fn serve(_end: OwnedFd, _worker: u32, _want: &str) -> bool {
    false
}

/// A worker's endpoints, verified before any agent starts: `n` local
/// sequenced-packet sockets at [`GRANT_FD`]`..`.
#[cfg(target_os = "linux")]
pub fn claim(n: usize) -> Result<Vec<RawFd>, String> {
    if n == 0 || n > MAX_ENDPOINTS {
        return Err("paid_grant_endpoints_invalid".into());
    }
    (0..n as RawFd)
        .map(|i| GRANT_FD + i)
        .map(|fd| {
            if linux::is_endpoint(fd) {
                Ok(fd)
            } else {
                Err("paid_grant_channel_missing".to_string())
            }
        })
        .collect()
}
#[cfg(not(target_os = "linux"))]
pub fn claim(_n: usize) -> Result<Vec<RawFd>, String> {
    Err("paid_grant_unsupported".into())
}

/// The wrapper's side: ask the coordinator on [`GRANT_FD`] for `want` and
/// require its authenticated `granted`. The endpoint is closed either way,
/// so nothing launched afterwards holds it.
#[cfg(target_os = "linux")]
pub fn request(want: &str) -> Result<(), String> {
    if want.len() > REQUEST_LIMIT || !linux::is_endpoint(GRANT_FD) {
        return Err("paid_grant_channel_missing".into());
    }
    let end = unsafe { OwnedFd::from_raw_fd(GRANT_FD) };
    let fd = end.as_raw_fd();
    linux::passcred(&end).map_err(|_| "paid_grant_channel_missing")?;
    linux::bound_wait(fd);
    linux::send(fd, want.as_bytes())?;
    let mut buf = [0u8; 16];
    let (n, creds) = linux::receive(fd, &mut buf)?;
    let me = std::process::id() as libc::pid_t;
    if !(linux::descends(me, creds.pid) && linux::runs_this(creds.pid)) {
        return Err("paid_grant_unauthenticated".into());
    }
    if &buf[..n] != GRANTED {
        return Err("paid_grant_refused".into());
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
pub fn request(_want: &str) -> Result<(), String> {
    Err("paid_grant_unsupported".into())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// A grant is answered once, to the exact request, and a reply from a
    /// process that is not an ancestor running this executable is refused.
    #[test]
    fn an_endpoint_answers_one_exact_request_from_a_descendant() {
        assert_eq!(want("ab", 7), "ab:7");
        assert!(endpoints(0).is_err() && endpoints(MAX_ENDPOINTS + 1).is_err());
        assert!(claim(0).is_err() && claim(MAX_ENDPOINTS + 1).is_err());
        // This test process is not its own ancestor: a self-made pair whose
        // reply it sends itself never authenticates.
        let mut ends = endpoints(1).unwrap();
        let (coordinator, worker) = ends.pop().unwrap();
        let me = std::process::id();
        let server = std::thread::spawn(move || serve(coordinator, me, "x:1"));
        linux::send(worker.as_raw_fd(), b"x:1").unwrap();
        let mut buf = [0u8; 16];
        let (n, creds) = linux::receive(worker.as_raw_fd(), &mut buf).unwrap();
        // The requester is this process, not a descendant of itself.
        assert_eq!(String::from_utf8_lossy(&buf[..n]), "refused\n");
        assert_eq!(creds.pid as u32, me);
        assert!(!server.join().unwrap());
        assert!(linux::runs_this(me as libc::pid_t));
        assert!(!linux::descends(me as libc::pid_t, me as libc::pid_t));
        // A closed coordinator is an end of file, never a grant.
        let (coordinator, worker) = linux::pair().unwrap();
        drop(coordinator);
        let mut buf = [0u8; 16];
        assert_eq!(
            linux::receive(worker.as_raw_fd(), &mut buf).err(),
            Some("paid_grant_channel_closed")
        );
        assert!(!linux::is_endpoint(-1));
        assert!(linux::is_endpoint(worker.as_raw_fd()));
    }
}
