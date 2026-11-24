//! The host route's launch authority (the panel's holds 2 on a73ac17a and
//! the security hold on 6074b83e, part of #404). A `--paid-launch` binding
//! in argv is a request, never a permission: anyone can compute a
//! selection's identity. A paid wrapper on the host launches only after the
//! live baseline coordinator answers that request, fresh, on a private
//! endpoint it handed down the worker chain.
//!
//! The coordinator makes one `SOCK_SEQPACKET` pair per expected agent of a
//! paid slot and places the worker's ends at descriptors
//! [`GRANT_FD`]`..`; the worker gives agent `i` its own end at
//! [`GRANT_FD`]. Each end answers exactly one request.
//!
//! **The exchange** (protocol `swarmail-grant/1`, bounded packets):
//!
//! 1. *Registration.* The kernel stamps a socket pair with its creator's
//!    credentials (`SO_PEERCRED`): the wrapper reads the coordinator's pid
//!    from its own end, and that process's start time from `/proc`, and
//!    requires it to be a live ancestor running this very executable.
//! 2. *Freshness.* Input already queued on the endpoint before the wrapper
//!    asks is refused (`paid_grant_reply_stale`): a reply cannot precede
//!    its request.
//! 3. *Challenge.* The wrapper sends `swarmail-grant/1 request <nonce>
//!    <binding>`, the nonce 128 fresh bits from the OS (`getrandom`; an
//!    entropy failure refuses). The coordinator grants only a request for
//!    exactly its assigned binding, unexpired, sent by a descendant of its
//!    own worker running this executable, and echoes `swarmail-grant/1
//!    granted <nonce> <binding>`.
//! 4. *Sender.* The reply counts only when the kernel's `SCM_CREDENTIALS`
//!    name the registered coordinator pid, with the start time unchanged,
//!    and it carries the wrapper's own nonce and binding byte for byte.
//!
//! A coordinator that is gone is an end of file, a refusal. Linux is the
//! one platform with that peer evidence; elsewhere a paid binding refuses
//! (`paid_grant_unsupported`).
//!
//! **Threat model (the operator's ruling of 2026-11-24, after the third
//! hold).** This boundary guards against *accidental or unbound* paid
//! launches: a bare wrapper, a caller-made argv, a stale, pre-queued or
//! replayed grant, a missing bind, a non-release driver. It does not defend
//! against a deliberate adversary running as the same uid, who could
//! equally run the agent CLI directly with the operator's credentials;
//! reviewers route such deliberate same-uid bypasses as residuals.
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::fd::{OwnedFd, RawFd};

/// The descriptor a paid wrapper finds its grant endpoint on; a worker
/// receives the endpoints of its agents from here on.
pub const GRANT_FD: RawFd = 4;
/// The most endpoints a worker can hold: the largest `agent_count`.
pub const MAX_ENDPOINTS: usize = 3;
/// The exchange's protocol tag, first word of every packet.
pub const PROTOCOL: &str = "swarmail-grant/1";
/// The longest packet either side accepts.
#[cfg(target_os = "linux")]
const PACKET_LIMIT: usize = 256;
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

/// The paid binding a wrapper asks for: its contract identity and validity.
pub fn want(cli_sha256: &str, valid_through: u64) -> String {
    format!("{cli_sha256}:{valid_through}")
}

/// The wrapper's request packet.
pub fn request_packet(nonce: &str, want: &str) -> String {
    format!("{PROTOCOL} request {nonce} {want}")
}

/// The coordinator's positive reply to `nonce` for `want`.
pub fn granted_packet(nonce: &str, want: &str) -> String {
    format!("{PROTOCOL} granted {nonce} {want}")
}

/// The coordinator's refusal of a request carrying `nonce`.
pub fn refused_packet(nonce: &str) -> String {
    format!("{PROTOCOL} refused {nonce}")
}

/// The coordinator's answer to one request packet, sent by a process that
/// `trusted` says descends from its worker and runs this executable, for
/// its assigned binding `want`, at Unix second `now`: the reply packet and
/// whether it grants. Pure, so every arm is tested directly.
pub fn answer(packet: &[u8], trusted: bool, want: &str, now: u64) -> (String, bool) {
    let text = std::str::from_utf8(packet).unwrap_or("");
    let mut words = text.split(' ');
    let (Some(PROTOCOL), Some("request"), Some(nonce), Some(asked), None) = (
        words.next(),
        words.next(),
        words.next(),
        words.next(),
        words.next(),
    ) else {
        return (format!("{PROTOCOL} refused -"), false);
    };
    let fresh = nonce.len() == 32 && nonce.bytes().all(|b| b.is_ascii_hexdigit());
    let unexpired = want
        .rsplit_once(':')
        .and_then(|(_, t)| t.parse::<u64>().ok())
        .is_some_and(|t| t >= now);
    if fresh && trusted && asked == want && unexpired {
        (granted_packet(nonce, want), true)
    } else {
        (refused_packet(if fresh { nonce } else { "-" }), false)
    }
}

/// The wrapper's verdict on a reply `packet` to its request for `want`
/// with `nonce`, once the sender is authenticated.
pub fn judge(packet: &[u8], nonce: &str, want: &str) -> Result<(), String> {
    if packet == granted_packet(nonce, want).as_bytes() {
        Ok(())
    } else if packet == refused_packet(nonce).as_bytes() {
        Err("paid_grant_refused".into())
    } else {
        Err("paid_grant_reply_unbound".into())
    }
}

/// Unix seconds now.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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
    /// The credentials the kernel stamped on `fd`'s socket pair when it was
    /// made: its creator's, for an endpoint the coordinator's.
    pub fn peer(fd: RawFd) -> Option<libc::ucred> {
        let mut c: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let r = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut c as *mut libc::ucred).cast(),
                &mut len,
            )
        };
        (r == 0 && c.pid > 0).then_some(c)
    }
    /// A live process's start time (clock ticks since boot): with its pid,
    /// the process instance, which a reused pid does not share.
    pub fn started(pid: libc::pid_t) -> Option<u64> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        stat.rsplit_once(')')?
            .1
            .split_whitespace()
            .nth(19)?
            .parse()
            .ok()
    }
    /// Whether a packet is already queued on `fd`, without waiting.
    pub fn queued(fd: RawFd) -> Result<bool, &'static str> {
        let mut b = [0u8; 1];
        let n = unsafe {
            libc::recv(
                fd,
                b.as_mut_ptr().cast(),
                1,
                libc::MSG_PEEK | libc::MSG_DONTWAIT,
            )
        };
        match n {
            0 => Err("paid_grant_channel_closed"),
            n if n > 0 => Ok(true),
            _ if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock => {
                Ok(false)
            }
            _ => Err("paid_grant_channel_closed"),
        }
    }
    /// 128 fresh bits from the OS, hex: the wrapper's challenge.
    pub fn nonce() -> Result<String, &'static str> {
        let mut b = [0u8; 16];
        let n = unsafe { libc::getrandom(b.as_mut_ptr().cast(), b.len(), 0) };
        if n == b.len() as isize {
            Ok(crate::provenance::hex(&b))
        } else {
            Err("paid_grant_entropy_unavailable")
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
/// This process makes them, so the kernel registers it as their creator.
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

/// Answer one endpoint once: grant only a fresh request for exactly
/// `want`, unexpired, sent by a process that descends from `worker` and
/// runs this executable, echoing its nonce. Returns whether it granted.
/// Blocks until the request or the end of file that follows the worker's
/// and its agents' exits.
#[cfg(target_os = "linux")]
pub fn serve(end: OwnedFd, worker: u32, want: &str) -> bool {
    let mut buf = [0u8; PACKET_LIMIT];
    let Ok((n, creds)) = linux::receive(end.as_raw_fd(), &mut buf) else {
        return false;
    };
    let trusted = linux::descends(creds.pid, worker as libc::pid_t) && linux::runs_this(creds.pid);
    let (reply, granted) = answer(&buf[..n], trusted, want, now());
    let _ = linux::send(end.as_raw_fd(), reply.as_bytes());
    granted
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

/// The wrapper's side: the exchange in this module's header, on
/// [`GRANT_FD`]. The endpoint is closed either way, so nothing launched
/// afterwards holds it.
#[cfg(target_os = "linux")]
pub fn request(want: &str) -> Result<(), String> {
    if want.len() > PACKET_LIMIT / 2 || !linux::is_endpoint(GRANT_FD) {
        return Err("paid_grant_channel_missing".into());
    }
    let end = unsafe { OwnedFd::from_raw_fd(GRANT_FD) };
    let fd = end.as_raw_fd();
    linux::passcred(&end).map_err(|_| "paid_grant_channel_missing")?;
    linux::bound_wait(fd);
    // Registration: the pair's creator, a live ancestor running this
    // executable, pinned to its process instance.
    let me = std::process::id() as libc::pid_t;
    let coordinator = linux::peer(fd)
        .map(|c| c.pid)
        .filter(|&pid| linux::descends(me, pid) && linux::runs_this(pid))
        .ok_or("paid_grant_unauthenticated")?;
    let started = linux::started(coordinator).ok_or("paid_grant_unauthenticated")?;
    // Freshness: nothing may answer before the request exists.
    if linux::queued(fd)? {
        return Err("paid_grant_reply_stale".into());
    }
    let nonce = linux::nonce()?;
    linux::send(fd, request_packet(&nonce, want).as_bytes())?;
    let mut buf = [0u8; PACKET_LIMIT];
    let (n, creds) = linux::receive(fd, &mut buf)?;
    if creds.pid != coordinator || linux::started(coordinator) != Some(started) {
        return Err("paid_grant_unauthenticated".into());
    }
    judge(&buf[..n], &nonce, want)
}
#[cfg(not(target_os = "linux"))]
pub fn request(_want: &str) -> Result<(), String> {
    Err("paid_grant_unsupported".into())
}

/// The debug-only stand-in coordinator's faults (`grant-standin
/// --fault`), each a reply other than the genuine coordinator's fresh
/// answer: `prequeue` (a well-formed grant queued before the request),
/// `nonce` (another nonce), `contract` (another binding) and `sender` (the
/// right reply, sent by another process). Absent from release builds.
#[cfg(all(debug_assertions, target_os = "linux"))]
pub fn serve_faulty(end: OwnedFd, want: &str, fault: &str) {
    let fd = end.as_raw_fd();
    let forged = granted_packet(&"0".repeat(32), want);
    let mut buf = [0u8; PACKET_LIMIT];
    if fault == "prequeue" {
        // Queued by [`prequeue`] before the wrapper started: wait it out.
        let _ = linux::receive(fd, &mut buf);
        return;
    }
    let Ok((n, _)) = linux::receive(fd, &mut buf) else {
        return;
    };
    let nonce = std::str::from_utf8(&buf[..n])
        .ok()
        .and_then(|t| t.split(' ').nth(2))
        .unwrap_or("-")
        .to_string();
    let reply = match fault {
        "nonce" => forged,
        "contract" => granted_packet(&nonce, &self::want(&"0".repeat(64), u64::MAX)),
        _ => granted_packet(&nonce, want),
    };
    if fault != "sender" {
        let _ = linux::send(fd, reply.as_bytes());
        return;
    }
    match unsafe { libc::fork() } {
        0 => unsafe {
            libc::send(fd, reply.as_ptr().cast(), reply.len(), libc::MSG_NOSIGNAL);
            libc::_exit(0)
        },
        pid if pid > 0 => unsafe {
            libc::waitpid(pid, std::ptr::null_mut(), 0);
        },
        _ => {}
    }
}

/// The `prequeue` fault's packet, sent on the coordinator's end before
/// the wrapper is even spawned.
#[cfg(all(debug_assertions, target_os = "linux"))]
pub fn prequeue(end: &OwnedFd, want: &str) {
    let _ = linux::send(
        end.as_raw_fd(),
        granted_packet(&"0".repeat(32), want).as_bytes(),
    );
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// The coordinator grants only a fresh, well-formed, trusted request for
    /// its exact unexpired binding, and echoes the request's nonce.
    #[test]
    fn the_coordinator_answers_only_its_own_fresh_binding() {
        let n = "a".repeat(32);
        let w = want("ab", 7);
        assert_eq!(w, "ab:7");
        let ok = request_packet(&n, &w);
        assert_eq!(
            answer(ok.as_bytes(), true, &w, 7),
            (granted_packet(&n, &w), true)
        );
        // Untrusted sender, other binding, expired, unparsable validity.
        let refused = (refused_packet(&n), false);
        assert_eq!(answer(ok.as_bytes(), false, &w, 7), refused);
        assert_eq!(answer(ok.as_bytes(), true, "ab:8", 7), refused);
        assert_eq!(answer(ok.as_bytes(), true, &w, 8), refused);
        assert_eq!(answer(ok.as_bytes(), true, "ab", 0), refused);
        // Malformed nonce, wrong protocol, extra word, not UTF-8.
        let none = (format!("{PROTOCOL} refused -"), false);
        assert_eq!(
            answer(request_packet("zz", &w).as_bytes(), true, &w, 0),
            (refused_packet("-"), false)
        );
        assert_eq!(answer(b"swarmail-grant/0 request x y", true, &w, 0), none);
        assert_eq!(answer(format!("{ok} z").as_bytes(), true, &w, 0), none);
        assert_eq!(answer(&[0xff], true, &w, 0), none);
        // The wrapper's verdict: its own nonce and binding, or nothing.
        assert_eq!(judge(granted_packet(&n, &w).as_bytes(), &n, &w), Ok(()));
        assert_eq!(
            judge(refused_packet(&n).as_bytes(), &n, &w).unwrap_err(),
            "paid_grant_refused"
        );
        for other in [
            granted_packet(&"b".repeat(32), &w),
            granted_packet(&n, "ab:8"),
            refused_packet("-"),
            "granted\n".to_string(),
        ] {
            assert_eq!(
                judge(other.as_bytes(), &n, &w).unwrap_err(),
                "paid_grant_reply_unbound"
            );
        }
        assert!(now() > 1_700_000_000);
    }

    /// The kernel's registration, freshness and sender evidence the
    /// wrapper relies on, read from a pair this process makes.
    #[test]
    fn an_endpoint_names_its_creator_and_any_queued_packet() {
        assert!(endpoints(0).is_err() && endpoints(MAX_ENDPOINTS + 1).is_err());
        assert!(claim(0).is_err() && claim(MAX_ENDPOINTS + 1).is_err());
        let me = std::process::id() as libc::pid_t;
        let mut ends = endpoints(1).unwrap();
        let (coordinator, worker) = ends.pop().unwrap();
        // Off the grant descriptors: tests in this process that run a
        // wrapper's request would otherwise take a pair landing on fd 4.
        let high = |fd: OwnedFd| {
            let h = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
            assert!(h >= 64);
            unsafe { OwnedFd::from_raw_fd(h) }
        };
        let (coordinator, worker) = (high(coordinator), high(worker));
        // The creator is this process, live, running this executable; it
        // is not its own ancestor.
        assert_eq!(linux::peer(worker.as_raw_fd()).unwrap().pid, me);
        assert!(linux::peer(-1).is_none());
        assert!(linux::started(me).is_some());
        assert!(linux::started(-1).is_none());
        assert!(linux::runs_this(me));
        assert!(!linux::descends(me, me));
        assert_eq!(linux::nonce().unwrap().len(), 32);
        assert_ne!(linux::nonce().unwrap(), linux::nonce().unwrap());
        // Nothing queued, then a queued packet, seen without consuming it.
        assert_eq!(linux::queued(worker.as_raw_fd()), Ok(false));
        linux::send(coordinator.as_raw_fd(), b"x").unwrap();
        assert_eq!(linux::queued(worker.as_raw_fd()), Ok(true));
        let mut buf = [0u8; 4];
        let (n, creds) = linux::receive(worker.as_raw_fd(), &mut buf).unwrap();
        assert_eq!((&buf[..n], creds.pid), (&b"x"[..], me));
        // The coordinator refuses a request from a process that does not
        // descend from its worker, echoing the request's nonce.
        let server = std::thread::spawn(move || serve(coordinator, me as u32, "x:9999999999"));
        let ask = request_packet(&"c".repeat(32), "x:9999999999");
        linux::send(worker.as_raw_fd(), ask.as_bytes()).unwrap();
        let mut buf = [0u8; PACKET_LIMIT];
        let (n, _) = linux::receive(worker.as_raw_fd(), &mut buf).unwrap();
        assert_eq!(&buf[..n], refused_packet(&"c".repeat(32)).as_bytes());
        assert!(!server.join().unwrap());
        // A closed coordinator is an end of file, never a grant. A fork in
        // a parallel test may hold a copy of its end until its exec, so the
        // end of file is awaited (bounded) before it is peeked.
        linux::bound_wait(worker.as_raw_fd());
        let mut buf = [0u8; 16];
        assert_eq!(
            linux::receive(worker.as_raw_fd(), &mut buf).err(),
            Some("paid_grant_channel_closed")
        );
        assert_eq!(
            linux::queued(worker.as_raw_fd()),
            Err("paid_grant_channel_closed")
        );
        assert_eq!(linux::queued(-1), Err("paid_grant_channel_closed"));
        assert!(!linux::is_endpoint(-1));
        assert!(linux::is_endpoint(worker.as_raw_fd()));
    }
}
