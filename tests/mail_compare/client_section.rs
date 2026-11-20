//! The MCP chunk's client scope judgment (issue #385): the production
//! classifier, `client_scope::judge`, against a real fake adapter whose
//! child outlives its stop signal and its clean control, against file
//! fixtures standing for a cgroup v2 mount, and on a paused clock.

use super::*;
use client_scope::{Probe, Receipt, Verdict};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

const UNIT: &str = "mail-compare-20261120T000000Z-42-api.mcp.mailpit.r0.f0-client.scope";
const CGROUP: &str = "/user.slice/user-1000.slice/user@1000.service/app.slice/mail-compare-20261120T000000Z-42-api.mcp.mailpit.r0.f0-client.scope";

fn handshake_bytes(unit: &str, cgroup: &str) -> Vec<u8> {
    serde_json::to_vec(&client_scope::Handshake {
        unit: unit.into(),
        cgroup: cgroup.into(),
        pid: 1,
    })
    .unwrap()
}

// ---------------------------------------------------------------------------
// A real fake adapter. Its membership source is the processes the fixture
// started, as the kernel would count them in the scope they inherited: the
// scope is populated while any of them lives.
// ---------------------------------------------------------------------------

struct Liveness {
    pids: Vec<i32>,
}

fn alive(pid: i32) -> bool {
    // A zombie has exited; only its parent's reap is left.
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map(|s| {
            s.rsplit(')')
                .next()
                .is_some_and(|r| !r.trim_start().starts_with('Z'))
        })
        .unwrap_or(false)
}

impl Probe for Liveness {
    async fn handshake(&self) -> io::Result<Vec<u8>> {
        Ok(handshake_bytes(UNIT, CGROUP))
    }
    async fn mount(&self) -> Result<(), String> {
        Ok(())
    }
    async fn events(&self, _: &str) -> io::Result<String> {
        let populated = self.pids.iter().any(|&p| alive(p));
        Ok(format!("populated {}\nfrozen 0\n", u8::from(populated)))
    }
    async fn procs(&self, _: &str) -> io::Result<String> {
        Ok(self
            .pids
            .iter()
            .filter(|&&p| alive(p))
            .map(|p| format!("{p}\n"))
            .collect())
    }
    async fn children(&self, _: &str) -> io::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn comm(&self, pid: u32) -> io::Result<Vec<u8>> {
        std::fs::read(format!("/proc/{pid}/comm"))
    }
}

/// A fake adapter: it forks a child that acknowledges its readiness and
/// blocks on the fixture's FIFO, then waits to be stopped. With
/// `child_ignores_term` the child ignores SIGTERM and outlives the stop;
/// without it the adapter's TERM handler reaps it first.
fn fake_adapter(dir: &Path, child_ignores_term: bool) -> (std::process::Child, i32, PathBuf) {
    use std::os::unix::process::CommandExt;
    let fifo = dir.join("hold.fifo");
    let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    // SAFETY: a valid NUL-terminated path; mkfifo only creates it.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let ack = dir.join("ack");
    // The surviving child ignores TERM and the adapter exits without it;
    // the control's adapter stops its child and reaps it before it exits.
    let (trap, on_term) = if child_ignores_term {
        ("trap '' TERM;", "exit 0")
    } else {
        ("", "kill -TERM $child; wait $child; exit 0")
    };
    let script = format!(
        "( {trap} echo $BASHPID > '{ack}.tmp'; mv '{ack}.tmp' '{ack}'; read -r _ < '{fifo}' ) </dev/null >/dev/null 2>&1 &\n\
         child=$!\n\
         trap '{on_term}' TERM\n\
         while :; do sleep 0.05 & wait $!; done\n",
        ack = ack.display(),
        fifo = fifo.display()
    );
    let child = std::process::Command::new("bash")
        .arg("-c")
        .arg(&script)
        .process_group(0)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    // Readiness is the child's own acknowledgement, bounded.
    let end = std::time::Instant::now() + Duration::from_secs(10);
    let pid = loop {
        if let Ok(t) = std::fs::read_to_string(&ack) {
            break t.trim().parse::<i32>().unwrap();
        }
        assert!(
            std::time::Instant::now() < end,
            "the adapter's child never acknowledged"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    (child, pid, fifo)
}

/// Stop the adapter as `timeout` does (SIGTERM to its process group) and
/// reap it, bounded.
fn stop(adapter: &mut std::process::Child) {
    let pgid = adapter.id() as i32;
    // SAFETY: a signal to the fixture's own process group.
    unsafe { libc::kill(-pgid, libc::SIGTERM) };
    let end = std::time::Instant::now() + Duration::from_secs(10);
    while adapter.try_wait().unwrap().is_none() {
        assert!(std::time::Instant::now() < end, "the adapter did not stop");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The fixture's unconditional, bounded teardown of a surviving child.
fn release(fifo: &Path, pid: i32) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if let Ok(mut w) = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(fifo)
    {
        let _ = w.write_all(b"go\n");
    }
    let end = std::time::Instant::now() + Duration::from_secs(10);
    while alive(pid) && std::time::Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    // SAFETY: the fixture's own child, if it is still there.
    unsafe { libc::kill(pid, libc::SIGKILL) };
}

fn judge_now<P: Probe>(p: &P) -> Receipt {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(client_scope::judge(p, UNIT, client_scope::JUDGE_DEFAULT))
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn an_adapter_child_that_outlives_the_stop_signal_is_a_named_survivor() {
    let dir = tempfile::tempdir().unwrap();
    let (mut adapter, child, fifo) = fake_adapter(dir.path(), true);
    let adapter_pid = adapter.id() as i32;
    stop(&mut adapter);
    let r = judge_now(&Liveness {
        pids: vec![adapter_pid, child],
    });
    let still_alive = alive(child);
    release(&fifo, child);
    assert!(
        still_alive,
        "the child was alive when its survival was reported"
    );
    assert_eq!(r.verdict, Verdict::Survivor, "{r:?}");
    assert_eq!(r.observation, "populated");
    assert_eq!(r.members.len(), 1, "{r:?}");
    assert_eq!(r.members[0].pid, child as u32);
    assert_eq!(r.members[0].name.as_deref(), Some("bash"));
    assert!(r.complete, "{r:?}");
    assert!(
        r.summary().contains(&format!("pid {child} (bash)")),
        "{}",
        r.summary()
    );
    assert!(r.proves_clean(UNIT).is_err());
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn an_adapter_that_reaps_its_child_before_it_exits_is_clean() {
    let dir = tempfile::tempdir().unwrap();
    let (mut adapter, child, fifo) = fake_adapter(dir.path(), false);
    let adapter_pid = adapter.id() as i32;
    stop(&mut adapter);
    let r = judge_now(&Liveness {
        pids: vec![adapter_pid, child],
    });
    release(&fifo, child);
    assert_eq!(r.verdict, Verdict::Clean, "{r:?}");
    assert_eq!(r.observation, "empty");
    r.proves_clean(UNIT).unwrap();
}

// ---------------------------------------------------------------------------
// The kernel probe over a tree of files standing for a cgroup v2 mount.
// ---------------------------------------------------------------------------

struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    /// A mount holding the client scope's cgroup, and a matching handshake.
    fn new() -> Tree {
        let t = Tree {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::write(t.mount().join("cgroup.controllers"), "cpu memory pids\n").unwrap();
        std::fs::create_dir_all(t.scope()).unwrap();
        std::fs::write(t.handshake(), handshake_bytes(UNIT, CGROUP)).unwrap();
        t
    }
    fn mount(&self) -> PathBuf {
        let m = self.dir.path().join("mount");
        std::fs::create_dir_all(&m).unwrap();
        m
    }
    fn scope(&self) -> PathBuf {
        self.mount().join(CGROUP.trim_start_matches('/'))
    }
    fn handshake(&self) -> PathBuf {
        self.dir.path().join("handshake.json")
    }
    fn events(&self, rel: &str, populated: &str) {
        let d = self.scope().join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("cgroup.events"),
            format!("populated {populated}\nfrozen 0\n"),
        )
        .unwrap();
    }
    fn procs(&self, rel: &str, pids: &[u32]) {
        let d = self.scope().join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(&d).unwrap();
        let text: String = pids.iter().map(|p| format!("{p}\n")).collect();
        std::fs::write(d.join("cgroup.procs"), text).unwrap();
    }
    fn judge(&self) -> Receipt {
        judge_now(&client_scope::Kernel::unverified(
            &self.mount(),
            &self.handshake(),
        ))
    }
}

#[test]
fn an_empty_or_removed_client_cgroup_with_its_handshake_is_clean() {
    let t = Tree::new();
    t.events("", "0");
    let r = t.judge();
    assert_eq!(
        (r.verdict, r.observation.as_str()),
        (Verdict::Clean, "empty")
    );
    r.proves_clean(UNIT).unwrap();
    assert_eq!(r.cgroup.as_deref(), Some(CGROUP));
    // Removed: systemd collected the empty scope before it was read.
    std::fs::remove_dir_all(t.scope()).unwrap();
    let r = t.judge();
    assert_eq!(
        (r.verdict, r.observation.as_str()),
        (Verdict::Clean, "removed")
    );
    r.proves_clean(UNIT).unwrap();
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn a_nested_member_makes_the_scope_a_survivor_and_a_vanished_pid_is_a_noted_race() {
    let t = Tree::new();
    // The root lists nobody; a nested cgroup holds this test's own process
    // and a pid that no longer exists.
    t.events("", "1");
    t.procs("", &[]);
    t.procs("/inner", &[]);
    t.procs("/inner/deeper", &[std::process::id(), 4_000_000_000]);
    let r = t.judge();
    assert_eq!(
        (r.verdict, r.observation.as_str()),
        (Verdict::Survivor, "populated")
    );
    assert_eq!(r.members.len(), 2, "{r:?}");
    assert_eq!(r.members[0].pid, std::process::id());
    assert_eq!(r.members[0].cgroup, "/inner/deeper");
    assert!(r.members[0].name.is_some());
    assert_eq!(r.members[1].name, None);
    assert!(
        r.notes
            .iter()
            .any(|n| n.contains("4000000000 exited before it was named")),
        "{r:?}"
    );
    assert!(r.complete, "a race is noted, not a truncation: {r:?}");
    // A survivor that empties later stays the first observation's survivor.
    t.events("", "0");
    assert_eq!(
        t.judge().verdict,
        Verdict::Clean,
        "a fresh judgment sees it empty"
    );
    assert_eq!(r.verdict, Verdict::Survivor);
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn the_member_list_is_capped_and_says_so() {
    let t = Tree::new();
    t.events("", "1");
    let pids: Vec<u32> = (4_000_000_000..4_000_000_100).collect();
    t.procs("", &pids);
    let r = t.judge();
    assert_eq!(r.verdict, Verdict::Survivor);
    assert_eq!(r.members.len(), client_scope::MAX_MEMBERS);
    assert!(!r.complete);
    assert!(r.notes.iter().any(|n| n.contains("stopped at 64")), "{r:?}");
}

#[test]
fn a_removed_client_cgroup_whose_parent_the_mount_lacks_is_unknown() {
    // The handshake names a path this mount never held (a cgroup
    // namespace, or a bind-mounted subtree): not the scope ended.
    let t = Tree::new();
    std::fs::remove_dir_all(t.mount().join("user.slice")).unwrap();
    let r = t.judge();
    assert_eq!(r.verdict, Verdict::Unknown, "{r:?}");
    assert!(
        r.observation.contains(
            "nor its parent /user.slice/user-1000.slice/user@1000.service/app.slice is under the mount"
        ),
        "{}",
        r.observation
    );
    assert!(r.proves_clean(UNIT).is_err());
    // A scope directly under the mount's root has the root for a parent.
    let top = format!("/{UNIT}");
    std::fs::write(t.handshake(), handshake_bytes(UNIT, &top)).unwrap();
    let r = t.judge();
    assert_eq!(
        (r.verdict, r.observation.as_str()),
        (Verdict::Clean, "removed"),
        "{r:?}"
    );
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn a_child_cgroup_name_is_escaped_in_the_receipt_and_its_summary() {
    let t = Tree::new();
    t.events("", "1");
    t.procs("", &[]);
    t.procs("/in\nner\x1b[2J", &[std::process::id()]);
    let r = t.judge();
    assert_eq!(r.verdict, Verdict::Survivor, "{r:?}");
    assert_eq!(r.members.len(), 1, "{r:?}");
    assert_eq!(r.members[0].cgroup, "/in\\x0aner\\x1b[2J");
    let line = r.summary();
    assert!(!line.chars().any(char::is_control), "{line:?}");
    assert!(line.contains("in /in\\x0aner\\x1b[2J"), "{line}");
    // A receipt read back with raw control characters still prints as one
    // line.
    let mut forged = r.clone();
    forged.notes.push("a\nforged line\r".into());
    forged.observation = "pop\x07ulated".into();
    let line = forged.summary();
    assert!(!line.chars().any(char::is_control), "{line:?}");
    assert!(line.contains("a\\x0aforged line\\x0d"), "{line}");
    assert!(line.contains("pop\\x07ulated"), "{line}");
}

#[test]
fn client_enter_confirms_its_pid_through_the_mount() {
    let t = Tree::new();
    let pid = std::process::id();
    // Missing: the mount does not show the client scope at all.
    let e = client_scope::confirm_member(&t.mount(), CGROUP, pid).unwrap_err();
    assert!(e.contains("is not visible under the mount"), "{e}");
    // Present, but this pid is not listed.
    t.procs("", &[1, pid + 1]);
    let e = client_scope::confirm_member(&t.mount(), CGROUP, pid).unwrap_err();
    assert!(e.contains(&format!("pid {pid} is not listed")), "{e}");
    // Listed.
    t.procs("", &[1, pid]);
    client_scope::confirm_member(&t.mount(), CGROUP, pid).unwrap();
}

#[test]
fn missing_mismatched_malformed_or_unreadable_evidence_is_unknown() {
    type Case = (&'static str, fn(&Tree), &'static str);
    let cases: [Case; 8] = [
        (
            "no handshake",
            |t| std::fs::remove_file(t.handshake()).unwrap(),
            "no handshake",
        ),
        (
            "a malformed handshake",
            |t| std::fs::write(t.handshake(), b"{").unwrap(),
            "malformed",
        ),
        (
            "another unit's handshake",
            |t| std::fs::write(t.handshake(), handshake_bytes("other.scope", CGROUP)).unwrap(),
            "not this chunk's unit",
        ),
        (
            "a cgroup outside the unit",
            |t| {
                std::fs::write(
                    t.handshake(),
                    handshake_bytes(UNIT, "/user.slice/session-1.scope"),
                )
                .unwrap()
            },
            "was not placed in its client scope",
        ),
        (
            "a cgroup that climbs",
            |t| {
                std::fs::write(
                    t.handshake(),
                    handshake_bytes(UNIT, &format!("/a/../{UNIT}")),
                )
                .unwrap()
            },
            "was not placed",
        ),
        (
            "no cgroup v2 mount",
            |t| std::fs::remove_file(t.mount().join("cgroup.controllers")).unwrap(),
            "not a readable cgroup v2 mount",
        ),
        (
            "malformed events",
            |t| std::fs::write(t.scope().join("cgroup.events"), "frozen 0\n").unwrap(),
            "cgroup.events is malformed",
        ),
        (
            "unreadable events",
            |t| std::fs::create_dir_all(t.scope().join("cgroup.events")).unwrap(),
            "cgroup.events is unreadable",
        ),
    ];
    for (what, edit, want) in cases {
        let t = Tree::new();
        t.events("", "0");
        if what == "unreadable events" {
            std::fs::remove_file(t.scope().join("cgroup.events")).unwrap();
        }
        edit(&t);
        let r = t.judge();
        assert_eq!(r.verdict, Verdict::Unknown, "{what}: {r:?}");
        assert!(r.observation.contains(want), "{what}: {}", r.observation);
        assert!(r.proves_clean(UNIT).is_err(), "{what}");
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn the_kernel_probe_refuses_a_mount_that_is_not_cgroup_v2() {
    // The real probe checks statfs: a temporary directory with the right
    // file names is not a cgroup v2 mount.
    let t = Tree::new();
    t.events("", "0");
    let mut k = client_scope::Kernel::new(&t.handshake());
    k.mount = t.mount();
    let r = judge_now(&k);
    assert_eq!(r.verdict, Verdict::Unknown, "{r:?}");
    assert!(
        r.observation.contains("is not a cgroup v2 mount"),
        "{}",
        r.observation
    );
}

// ---------------------------------------------------------------------------
// The one deadline, on a paused clock: a stalled read is cut at once in real
// time, and never at more than the judgment's five seconds.
// ---------------------------------------------------------------------------

/// A probe whose `stall` step never answers.
struct Stalled {
    stall: &'static str,
}

impl Stalled {
    async fn at<T>(&self, step: &str, v: T) -> T {
        if self.stall == step {
            std::future::pending::<()>().await;
        }
        v
    }
}

impl Probe for Stalled {
    async fn handshake(&self) -> io::Result<Vec<u8>> {
        Ok(self.at("handshake", handshake_bytes(UNIT, CGROUP)).await)
    }
    async fn mount(&self) -> Result<(), String> {
        self.at("mount", Ok(())).await
    }
    async fn events(&self, _: &str) -> io::Result<String> {
        Ok(self.at("events", "populated 1\n".to_owned()).await)
    }
    async fn procs(&self, _: &str) -> io::Result<String> {
        Ok(self.at("procs", "4000000000\n".to_owned()).await)
    }
    async fn children(&self, _: &str) -> io::Result<Vec<String>> {
        Ok(self.at("children", vec![]).await)
    }
    async fn comm(&self, _: u32) -> io::Result<Vec<u8>> {
        Ok(self.at("comm", b"adapter\n".to_vec()).await)
    }
}

#[tokio::test(start_paused = true)]
async fn a_stalled_observation_is_unknown_at_the_deadline_and_never_later() {
    for (stall, want) in [
        ("handshake", "the handshake read was cut"),
        ("mount", "the mount check was cut"),
        ("events", "the first observation was cut"),
    ] {
        let t0 = tokio::time::Instant::now();
        let r = client_scope::judge(&Stalled { stall }, UNIT, Duration::from_secs(3)).await;
        assert_eq!(t0.elapsed(), Duration::from_secs(3), "{stall}");
        assert_eq!(r.verdict, Verdict::Unknown, "{stall}");
        assert!(r.observation.contains(want), "{stall}: {}", r.observation);
    }
    // A budget above the cap is held to five seconds.
    let t0 = tokio::time::Instant::now();
    let r = client_scope::judge(&Stalled { stall: "events" }, UNIT, Duration::from_secs(60)).await;
    assert_eq!(t0.elapsed(), client_scope::JUDGE_MAX);
    assert_eq!(r.verdict, Verdict::Unknown);
}

/// A probe whose client cgroup is removed and whose parent check fails
/// with `parent`, or stalls when it is `None` (the parent vanishing or
/// turning unreadable between the two reads is a race no file tree holds).
struct Removed {
    parent: Option<io::ErrorKind>,
}

impl Probe for Removed {
    async fn handshake(&self) -> io::Result<Vec<u8>> {
        Ok(handshake_bytes(UNIT, CGROUP))
    }
    async fn mount(&self) -> Result<(), String> {
        Ok(())
    }
    async fn events(&self, _: &str) -> io::Result<String> {
        Err(io::ErrorKind::NotFound.into())
    }
    async fn procs(&self, _: &str) -> io::Result<String> {
        unreachable!("a removed scope is not walked")
    }
    async fn children(&self, _: &str) -> io::Result<Vec<String>> {
        match self.parent {
            Some(kind) => Err(kind.into()),
            None => std::future::pending().await,
        }
    }
    async fn comm(&self, _: u32) -> io::Result<Vec<u8>> {
        unreachable!("a removed scope is not walked")
    }
}

#[tokio::test(start_paused = true)]
async fn a_removed_scope_whose_parent_check_fails_or_stalls_is_unknown_within_the_deadline() {
    let t0 = tokio::time::Instant::now();
    let r = client_scope::judge(&Removed { parent: None }, UNIT, Duration::from_secs(3)).await;
    assert_eq!(t0.elapsed(), Duration::from_secs(3));
    assert_eq!(r.verdict, Verdict::Unknown, "{r:?}");
    assert!(
        r.observation
            .contains("the removed scope's parent check was cut"),
        "{}",
        r.observation
    );
    for (kind, want) in [
        (
            io::ErrorKind::PermissionDenied,
            "the removed scope's parent is unreadable",
        ),
        (
            io::ErrorKind::NotFound,
            "observer cannot see the client scope",
        ),
    ] {
        let t0 = tokio::time::Instant::now();
        let r = client_scope::judge(
            &Removed { parent: Some(kind) },
            UNIT,
            Duration::from_secs(3),
        )
        .await;
        assert_eq!(t0.elapsed(), Duration::ZERO, "{kind:?}");
        assert_eq!(r.verdict, Verdict::Unknown, "{kind:?}: {r:?}");
        assert!(r.observation.contains(want), "{kind:?}: {}", r.observation);
    }
}

#[tokio::test(start_paused = true)]
async fn a_survivor_whose_diagnostics_stall_is_reported_at_the_deadline() {
    for stall in ["procs", "comm", "children"] {
        let t0 = tokio::time::Instant::now();
        let r = client_scope::judge(&Stalled { stall }, UNIT, Duration::from_millis(1500)).await;
        assert_eq!(t0.elapsed(), Duration::from_millis(1500), "{stall}");
        // The first observation decided; the cut diagnostics only say so.
        assert_eq!(r.verdict, Verdict::Survivor, "{stall}");
        assert!(!r.complete, "{stall}");
        assert!(
            r.notes.iter().any(|n| n.contains("diagnostics was cut")),
            "{stall}: {r:?}"
        );
    }
    // Unstalled, the whole judgment takes no time at all.
    let t0 = tokio::time::Instant::now();
    let r = client_scope::judge(&Stalled { stall: "" }, UNIT, Duration::from_secs(3)).await;
    assert_eq!(t0.elapsed(), Duration::ZERO);
    assert_eq!(r.members[0].name.as_deref(), Some("adapter"));
    assert!(r.complete);
}

// ---------------------------------------------------------------------------
// The handshake, the arguments and the platform.
// ---------------------------------------------------------------------------

#[test]
fn every_other_platform_refuses_before_a_workload_is_spawned() {
    // Portable: the decision itself, no process launched.
    client_scope::platform("linux").unwrap();
    for os in ["macos", "freebsd", "windows"] {
        let e = client_scope::platform(os).unwrap_err();
        assert!(e.contains("(Linux)") && e.contains(os), "{e}");
    }
    #[cfg(not(target_os = "linux"))]
    assert!(client_scope::platform(std::env::consts::OS).is_err());
}

#[test]
fn the_handshake_is_the_kernels_unified_line_inside_the_unit() {
    let text = format!("0::{CGROUP}\n");
    let hs = client_scope::own_handshake(UNIT, &text).unwrap();
    assert_eq!((hs.unit.as_str(), hs.cgroup.as_str()), (UNIT, CGROUP));
    assert_eq!(hs.pid, std::process::id());
    let e = client_scope::own_handshake(UNIT, "0::/user.slice/session-2.scope\n").unwrap_err();
    assert!(e.contains("not in its client scope"), "{e}");
    for v1 in ["", "1:name=systemd:/x\n", "0::/a\n0::/b\n"] {
        assert!(client_scope::own_handshake(UNIT, v1).is_err(), "{v1:?}");
    }
    // Exclusive: a second write to the same path refuses.
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("hs");
    client_scope::write_exclusive(&f, b"{}").unwrap();
    let e = client_scope::write_exclusive(&f, b"{}").unwrap_err();
    assert!(e.contains("it must be new"), "{e}");
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn names_are_escaped_and_units_are_plain() {
    assert_eq!(
        client_scope::escape(b"a\"b\\c\n\xff"),
        "a\"b\\x5cc\\x0a\\xff"
    );
    assert!(client_scope::valid_unit(UNIT));
    for bad in ["x", ".scope", "a b.scope", "a/b.scope", "a.service"] {
        assert!(!client_scope::valid_unit(bad), "{bad}");
    }
    assert!(client_scope::cgroup_names_unit(CGROUP, UNIT));
    for bad in ["", "relative/x", "/a//x", "/a/./x", &format!("/a\n/{UNIT}")] {
        assert!(!client_scope::cgroup_names_unit(bad, UNIT), "{bad:?}");
    }
}

#[test]
fn the_modes_take_exactly_their_arguments() {
    let v = |s: &[&str]| s.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    let a = client_scope::parse_args(
        &v(&["--unit", UNIT, "--handshake", "h", "--", "api-mcp", "--x"]),
        true,
    )
    .unwrap();
    assert_eq!(a.command, ["api-mcp", "--x"]);
    let j = client_scope::parse_args(
        &v(&[
            "--unit",
            UNIT,
            "--handshake",
            "h",
            "--out",
            "o",
            "--budget-ms",
            "2500",
        ]),
        false,
    )
    .unwrap();
    assert_eq!(j.budget, Duration::from_millis(2500));
    assert_eq!(
        client_scope::parse_args(
            &v(&["--unit", UNIT, "--handshake", "h", "--out", "o"]),
            false
        )
        .unwrap()
        .budget,
        client_scope::JUDGE_DEFAULT
    );
    for (args, command, want) in [
        (
            v(&["--unit", UNIT, "--handshake", "h"]),
            true,
            "a command is required",
        ),
        (
            v(&["--unit", UNIT, "--handshake", "h", "--", "x"]),
            false,
            "takes no command",
        ),
        (
            v(&["--unit", UNIT, "--handshake", "h"]),
            false,
            "--out is required",
        ),
        (
            v(&["--handshake", "h", "--out", "o"]),
            false,
            "--unit is required",
        ),
        (
            v(&["--unit", UNIT, "--out", "o"]),
            false,
            "--handshake is required",
        ),
        (
            v(&["--unit", "a b.scope", "--handshake", "h", "--out", "o"]),
            false,
            "not a plain",
        ),
        (
            v(&[
                "--unit",
                UNIT,
                "--handshake",
                "h",
                "--out",
                "o",
                "--budget-ms",
                "5001",
            ]),
            false,
            "above",
        ),
        (
            v(&[
                "--unit",
                UNIT,
                "--handshake",
                "h",
                "--out",
                "o",
                "--budget-ms",
                "x",
            ]),
            false,
            "--budget-ms x",
        ),
        (
            v(&[
                "--unit",
                UNIT,
                "--handshake",
                "h",
                "--out",
                "o",
                "--other",
                "1",
            ]),
            false,
            "unknown flag",
        ),
        (
            v(&["--unit", UNIT, "--handshake", "h", "--out", "o", "--", "x"]),
            true,
            "unknown flag --out",
        ),
        (v(&["--unit"]), false, "needs a value"),
    ] {
        let e = client_scope::parse_args(&args, command).unwrap_err();
        assert!(e.contains(want), "{args:?}: {e}");
    }
}
