//! Owned targets (spec R4): one owner launches each server in its own
//! process group with a closed environment and private scratch, proves
//! protocol readiness within a deadline, and on every exit path signals,
//! reaps and verifies that the group and its listeners are gone. It never
//! touches a process or endpoint it did not create.

use crate::protocol::TargetKind;
use crate::wire::{Http, WireError};
use serde::Serialize;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};

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
    pub binary: PathBuf,
    /// The digest the bytes must have; `None` only for the candidate,
    /// whose digest is recorded rather than pinned.
    pub expected_sha256: Option<String>,
    pub template: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaunchFacts {
    pub target: TargetKind,
    pub executable_sha256: String,
    /// The template, not the expanded argv: expanded values carry private
    /// scratch paths and ports, which stay out of public records.
    pub argv_template: Vec<String>,
    pub ready_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleanupFacts {
    pub target: TargetKind,
    pub forced_kill: bool,
    pub reaped: bool,
    pub group_gone: bool,
    pub listeners_closed: bool,
    pub scratch_removed: bool,
}

impl CleanupFacts {
    pub fn proved(&self) -> bool {
        self.reaped && self.group_gone && self.listeners_closed && self.scratch_removed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    Binary(&'static str),
    DigestMismatch,
    Scratch,
    Spawn,
    /// Readiness failed; the partial start was still cleaned up, and the
    /// facts say whether that cleanup is proved.
    NotReady(&'static str, CleanupFacts),
}

/// Where a target's listing keeps a message's subject.
type SubjectOf = fn(&serde_json::Value) -> Option<&str>;

pub struct Owned {
    pub kind: TargetKind,
    pub smtp: SocketAddr,
    pub http_addr: SocketAddr,
    pub http: Http,
    pub facts: LaunchFacts,
    child: Option<Child>,
    pgid: i32,
    scratch: PathBuf,
    limits: Limits,
    drain: Option<tokio::task::JoinHandle<()>>,
}

pub use crate::protocol::sha256_hex;

/// A loopback port the OS just handed out. The listener is dropped before
/// the child binds it, so a foreign bind can win the gap; that loss shows
/// up as a readiness failure, never as measuring someone else's server,
/// because readiness also requires the owned child to be alive.
fn reserve_port() -> Option<SocketAddr> {
    TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok()
}

fn private_dir(path: &Path) -> bool {
    std::fs::DirBuilder::new().mode(0o700).create(path).is_ok()
}

impl Owned {
    /// Launch `spec` with scratch at `scratch` (which must not exist).
    pub async fn launch(
        spec: &LaunchSpec,
        scratch: &Path,
        limits: &Limits,
        clock: &crate::wire::Clock,
    ) -> Result<Owned, LaunchError> {
        // Absolute paths only past this point: a relative program or scratch
        // path would resolve against the child's working directory.
        let binary =
            std::fs::canonicalize(&spec.binary).map_err(|_| LaunchError::Binary("unreadable"))?;
        let bytes = std::fs::read(&binary).map_err(|_| LaunchError::Binary("unreadable"))?;
        let mode = std::fs::metadata(&binary).map_err(|_| LaunchError::Binary("unreadable"))?;
        if mode.permissions().mode() & 0o111 == 0 {
            return Err(LaunchError::Binary("not executable"));
        }
        let digest = sha256_hex(&bytes);
        drop(bytes);
        if spec
            .expected_sha256
            .as_deref()
            .is_some_and(|want| want != digest)
        {
            return Err(LaunchError::DigestMismatch);
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
        let (smtp_slot, http_slot) = match spec.kind {
            TargetKind::Swarmail => ("127.0.0.1:0".to_owned(), "127.0.0.1:0".to_owned()),
            _ => match (reserve_port(), reserve_port()) {
                (Some(s), Some(h)) => (s.to_string(), h.to_string()),
                _ => {
                    let _ = std::fs::remove_dir_all(scratch);
                    return Err(LaunchError::Spawn);
                }
            },
        };
        let data_dir = scratch.join("data");
        let argv: Vec<String> = spec
            .template
            .iter()
            .map(|a| {
                a.replace("{smtp_addr}", &smtp_slot)
                    .replace("{http_addr}", &http_slot)
                    .replace("{data_dir}", &data_dir.to_string_lossy())
            })
            .collect();
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
            .env("XDG_STATE_HOME", scratch.join("state"))
            .current_dir(scratch)
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
        let mut owned = Owned {
            kind: spec.kind,
            smtp: "127.0.0.1:0".parse().expect("literal"),
            http_addr: "127.0.0.1:0".parse().expect("literal"),
            http: Http::new(
                "127.0.0.1:0".parse().expect("literal"),
                limits.request,
                limits.max_response_bytes,
            ),
            facts: LaunchFacts {
                target: spec.kind,
                executable_sha256: digest,
                argv_template: spec.template.clone(),
                ready_ns: 0,
            },
            child: None,
            pgid,
            scratch: scratch.to_owned(),
            limits: limits.clone(),
            drain: None,
        };
        let stdout = child.stdout.take();
        owned.child = Some(child);
        let deadline = Instant::now() + limits.startup;
        let addrs = match (spec.kind, stdout) {
            (TargetKind::Swarmail, Some(out)) => {
                let (addrs, drain) = read_startup(out, deadline).await;
                owned.drain = drain;
                addrs
            }
            _ => smtp_slot.parse().ok().zip(http_slot.parse().ok()),
        };
        let Some((smtp, http)) = addrs else {
            let facts = owned.shutdown().await;
            return Err(LaunchError::NotReady("no listener addresses", facts));
        };
        owned.smtp = smtp;
        owned.http_addr = http;
        owned.http = Http::new(http, limits.request, limits.max_response_bytes);
        if let Err(why) = owned.ready(deadline).await {
            let facts = owned.shutdown().await;
            return Err(LaunchError::NotReady(why, facts));
        }
        owned.facts.ready_ns = clock.now_ns();
        Ok(owned)
    }

    /// The leader's pid, which is also its process group id. Read by the
    /// lifecycle tests; the driver itself never needs it.
    #[allow(dead_code)]
    pub fn pid(&self) -> i32 {
        self.pgid
    }

    /// Protocol readiness: the owned child still runs, the SMTP banner is
    /// `220` and the HTTP listing answers `200`, all before `deadline`.
    async fn ready(&mut self, deadline: Instant) -> Result<(), &'static str> {
        loop {
            if let Some(child) = self.child.as_mut()
                && !matches!(child.try_wait(), Ok(None))
            {
                return Err("exited before ready");
            }
            let banner = crate::wire::Smtp::open(self.smtp, self.limits.request).await;
            let listing = self.http.get(&self.list_path(0, 1)).await;
            if let (Ok(s), Ok((200, _))) = (banner, listing) {
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

    // The competitor mappings below follow the captured API documents in
    // docs/evals/sources/ where those agree with the pinned executables,
    // and the executables where they do not; every disagreement is listed
    // in docs/evals/README.md and protocol.json's unresolved list.

    fn list_path(&self, start: u32, limit: u32) -> String {
        match self.kind {
            TargetKind::Swarmail => format!(
                "/api/v1/inboxes/default/messages?subject=mail-compare&limit={limit}&offset={start}"
            ),
            TargetKind::Mailpit => format!("/api/v1/messages?start={start}&limit={limit}"),
            TargetKind::Mailhog => format!("/api/v2/messages?start={start}&limit={limit}"),
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
        }
    }

    /// One page: the count of everything the request matches (not the
    /// mailbox size) and `[(server id, subject)]`. Mailpit's search keeps
    /// the mailbox size in `total` and the matches in `messages_count`;
    /// MailHog's `total` is the match count whatever its Swagger says.
    /// Where the target echoes `start`, it must be the offset asked for.
    fn page(&self, v: &serde_json::Value, start: u32) -> Option<(u64, Vec<(String, String)>)> {
        let (items, id, matching, subject): (&str, &str, &str, SubjectOf) = match self.kind {
            TargetKind::Swarmail => ("emails", "id", "total", |m| m.get("subject")?.as_str()),
            TargetKind::Mailpit => ("messages", "ID", "messages_count", |m| {
                m.get("Subject")?.as_str()
            }),
            TargetKind::Mailhog => ("items", "ID", "total", |m| {
                m.get("Content")?
                    .get("Headers")?
                    .get("Subject")?
                    .get(0)?
                    .as_str()
            }),
        };
        let total = v.get(matching)?.as_u64()?;
        if self.kind != TargetKind::Swarmail && v.get("start")?.as_u64()? != u64::from(start) {
            return None;
        }
        let list = v.get(items)?.as_array()?;
        let mut out = Vec::with_capacity(list.len());
        for m in list {
            out.push((
                m.get(id)?.as_str()?.to_owned(),
                subject(m).unwrap_or_default().to_owned(),
            ));
        }
        Some((total, out))
    }

    /// Page `path(start, PAGE)` to completion: `[(server id, subject)]`.
    /// Complete means the pages reached the matching count with every id
    /// distinct and the count unchanged. A page that does not parse, a
    /// repeated id, a moving count, an empty page short of the count or
    /// more pages than `max_pages` is an error, never a shorter success.
    async fn scan(
        &self,
        path: impl Fn(u32, u32) -> String,
        max_pages: u32,
    ) -> Result<Vec<(String, String)>, WireError> {
        const PAGE: u32 = 250;
        let mut out: Vec<(String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut expected = None;
        for n in 0..max_pages {
            let start = n * PAGE;
            let (status, v) = self.http.get_json(&path(start, PAGE)).await?;
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
                std::cmp::Ordering::Less if got == 0 => return Err(WireError::Incomplete),
                std::cmp::Ordering::Less => {}
            }
        }
        Err(WireError::Oversized)
    }

    /// Every message the server shows, paged to completion, as
    /// `(token or "", server id)`.
    pub async fn list_all(&self, max_pages: u32) -> Result<Vec<(String, String)>, WireError> {
        let all = self.scan(|s, l| self.list_path(s, l), max_pages).await?;
        Ok(all
            .into_iter()
            .map(|(id, subject)| (token_of(&subject), id))
            .collect())
    }

    /// One complete search for `token`: every hit, related or not, so a
    /// target that answers with unrelated mail is recorded doing so.
    pub async fn find(&self, token: &str) -> Result<Vec<String>, WireError> {
        const FIND_MAX_PAGES: u32 = 4;
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
        }
    }

    /// Signal the group, reap the leader, prove the group and listeners are
    /// gone and remove the owned scratch. Idempotent; every exit path of
    /// the driver ends here.
    pub async fn shutdown(&mut self) -> CleanupFacts {
        let mut facts = CleanupFacts {
            target: self.kind,
            forced_kill: false,
            reaped: self.child.is_none(),
            group_gone: false,
            listeners_closed: false,
            scratch_removed: false,
        };
        if let Some(mut child) = self.child.take() {
            signal_group(self.pgid, libc::SIGTERM);
            if let Ok(Ok(_)) = tokio::time::timeout(self.limits.cleanup, child.wait()).await {
                facts.reaped = true;
            } else {
                facts.forced_kill = true;
                signal_group(self.pgid, libc::SIGKILL);
                facts.reaped = matches!(
                    tokio::time::timeout(self.limits.cleanup, child.wait()).await,
                    Ok(Ok(_))
                );
            }
        }
        // A member that outlived the leader still holds the group id, so
        // the id cannot have been reused: it is still ours to kill. With no
        // member left the kill finds nothing (ESRCH); reuse in between would
        // need the pid space to wrap inside that instant.
        signal_group(self.pgid, libc::SIGKILL);
        let until = Instant::now() + self.limits.cleanup;
        loop {
            facts.group_gone = !group_exists(self.pgid);
            if facts.group_gone || Instant::now() >= until {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        if let Some(drain) = self.drain.take() {
            facts.reaped &= tokio::time::timeout(self.limits.cleanup, drain)
                .await
                .is_ok();
        }
        facts.listeners_closed = refused(self.smtp).await && refused(self.http_addr).await;
        facts.scratch_removed =
            std::fs::remove_dir_all(&self.scratch).is_ok() || !self.scratch.exists();
        facts
    }
}

impl Drop for Owned {
    /// Cancellation: a dropped owner kills its group synchronously. Drop
    /// cannot await the reap, so a dropped run proves nothing about
    /// cleanup and its output never receives a completion index.
    fn drop(&mut self) {
        if self.child.is_some() {
            signal_group(self.pgid, libc::SIGKILL);
        }
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

/// A listener counts as closed when a connect is refused — a bound port
/// someone else grabbed in the meantime reads as still open, which fails
/// cleanup rather than passing it.
async fn refused(addr: SocketAddr) -> bool {
    addr.port() == 0
        || matches!(
            tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr))
                .await,
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
        return (None, Some(drain));
    }
    let field = |name: &str| -> Option<SocketAddr> {
        line.split(name)
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    (field("smtp=").zip(field("http=")), Some(drain))
}

/// `"mail-compare <token> …"` → `<token>`; anything else → `""`.
pub fn token_of(subject: &str) -> String {
    let mut words = subject.split_whitespace();
    match (words.next(), words.next()) {
        (Some("mail-compare"), Some(t)) if t.starts_with("mc-") => t.to_owned(),
        _ => String::new(),
    }
}

/// Only parts that are text/plain or text/html as sent: the parser's
/// converted alternatives are not what was stored.
fn parse_raw(raw: &[u8]) -> Result<Fetched, WireError> {
    let parsed = mail_parser::MessageParser::default()
        .parse(raw)
        .ok_or(WireError::Malformed)?;
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
/// the protocol's cap (`0` = unlimited).
pub fn swarmail_template(cap: u64) -> Vec<String> {
    [
        "serve",
        "--smtp-listen",
        "{smtp_addr}",
        "--pop3-listen",
        "127.0.0.1:0",
        "--http-listen",
        "{http_addr}",
        "--max-per-inbox",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .chain([cap.to_string()])
    .collect()
}
