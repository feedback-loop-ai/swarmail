//! The one sequential schedule (spec R3): for each repetition, targets in
//! the protocol's counterbalanced order, every profile on a fresh owned
//! instance. Rows stream as they happen; cleanup that is not proved stops
//! all later work.

use crate::mailpit_mcp::{Frame, Scan, ToolError, poll_wait};
use crate::protocol::{
    Connection, Observation, Protocol, TargetKind, ThroughputProfile, WaitProfile,
};
use crate::records::{Manifest, Phase, Planned, ROW_VERSION, Row, RunDir};
use crate::targets::{CleanupFacts, LaunchError, LaunchFacts, LaunchSpec, Limits, Owned};
use crate::wire::{Clock, Disposition, Message, Smtp, authored, same_body};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Config {
    pub protocol: Protocol,
    pub manifest: Manifest,
    pub specs: BTreeMap<TargetKind, LaunchSpec>,
    pub output: PathBuf,
    pub scratch: PathBuf,
}

/// How a run ended. `Infrastructure` dominates: a run that could not
/// prove its own lifecycle reports nothing as measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    WorkloadFailures(Vec<String>),
    Infrastructure(String),
}

impl Verdict {
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Clean => 0,
            Verdict::WorkloadFailures(_) => 1,
            Verdict::Infrastructure(_) => 2,
        }
    }
}

pub struct Lifecycle {
    pub launches: Vec<LaunchFacts>,
    pub cleanups: Vec<CleanupFacts>,
}

/// Every coordinate the run will produce, declared before launch.
pub fn plan(p: &Protocol) -> Vec<Planned> {
    let mut out = Vec::new();
    let reps = p
        .throughput
        .iter()
        .map(|t| t.repetitions)
        .chain(p.waits.iter().map(|w| w.repetitions))
        .max()
        .unwrap_or(0);
    for rep in 0..reps {
        for &target in p.order(rep) {
            for t in p.throughput.iter().filter(|t| t.repetitions > rep) {
                let mk = |phase, rows| Planned {
                    target,
                    profile: t.name.clone(),
                    repetition: rep,
                    phase,
                    rows,
                };
                out.push(mk(Phase::Send, Some(t.messages)));
                out.push(mk(Phase::Visible, None));
                out.push(mk(Phase::Verified, Some(1)));
            }
            for w in p.waits.iter().filter(|w| w.repetitions > rep) {
                let mk = |phase, rows| Planned {
                    target,
                    profile: w.name.clone(),
                    repetition: rep,
                    phase,
                    rows,
                };
                out.push(mk(Phase::Present, Some(w.present)));
                out.push(mk(Phase::Future, Some(w.future)));
                out.push(mk(Phase::Empty, Some(w.empty)));
            }
        }
    }
    out
}

/// Tokens are `mc-<tag>-<index:06>`; the tag is fixed width per run so all
/// tokens of a run have one length.
fn tag(run_id: &str, target: TargetKind, profile: usize, rep: u32) -> String {
    let h = crate::targets::sha256_hex(run_id.as_bytes());
    let t = TargetKind::ALL
        .iter()
        .position(|k| *k == target)
        .unwrap_or(9);
    format!("{}t{t}p{profile:02}r{rep:02}", &h[..8])
}

pub fn row(
    run: &str,
    target: TargetKind,
    profile: &str,
    rep: u32,
    phase: Phase,
    sample: u32,
    outcome: &str,
) -> Row {
    Row {
        row_version: ROW_VERSION,
        run_id: run.to_owned(),
        target,
        profile: profile.to_owned(),
        repetition: rep,
        phase,
        sample,
        token: None,
        outcome: outcome.to_owned(),
        reason: None,
        code: None,
        invoked_ns: None,
        start_ns: None,
        ack_ns: None,
        observed_ns: None,
        occurrences: None,
        fidelity_ok: None,
        requests: None,
    }
}

type Sink = Arc<Mutex<RunDir>>;

fn emit(sink: &Sink, r: &Row) -> Result<(), String> {
    sink.lock()
        .expect("sink lock")
        .append(r)
        .map_err(|e| format!("sample write failed: {e}"))
}

pub fn limits(p: &Protocol) -> Limits {
    Limits {
        startup: Duration::from_millis(p.resources.startup_deadline_ms),
        cleanup: Duration::from_millis(p.resources.cleanup_deadline_ms),
        request: Duration::from_millis(p.resources.request_deadline_ms),
        max_response_bytes: p.resources.max_response_bytes,
        open_files: u64::from(p.resources.open_files),
    }
}

/// Run the whole schedule. The returned run directory is sealed only on
/// `Clean` or `WorkloadFailures`, after every cleanup is proved.
pub async fn run(cfg: Config, clock: Clock) -> (Verdict, Lifecycle, Option<PathBuf>) {
    let mut life = Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let dir = match RunDir::create(&cfg.output, &cfg.manifest) {
        Ok(d) => d,
        Err(e) => return (Verdict::Infrastructure(format!("output: {e}")), life, None),
    };
    let sink: Sink = Arc::new(Mutex::new(dir));
    let limits = limits(&cfg.protocol);
    let p = &cfg.protocol;
    let run_id = cfg.manifest.run_id.clone();
    let mut failures = Vec::new();
    let reps = p
        .throughput
        .iter()
        .map(|t| t.repetitions)
        .chain(p.waits.iter().map(|w| w.repetitions))
        .max()
        .unwrap_or(0);
    let mut leaf = 0u32;
    for rep in 0..reps {
        for &target in p.order(rep) {
            let Some(spec) = cfg.specs.get(&target) else {
                return (
                    Verdict::Infrastructure(format!("{target}: no launch spec")),
                    life,
                    None,
                );
            };
            let jobs: Vec<Job> = p
                .throughput
                .iter()
                .enumerate()
                .filter(|(_, t)| t.repetitions > rep)
                .map(|(i, t)| Job::Throughput(i, t))
                .chain(
                    p.waits
                        .iter()
                        .enumerate()
                        .filter(|(_, w)| w.repetitions > rep)
                        .map(|(i, w)| Job::Wait(p.throughput.len() + i, w)),
                )
                .collect();
            for job in jobs {
                if let Job::Throughput(_, t) = job
                    && t.warmup_messages > 0
                {
                    leaf += 1;
                    if let Err(e) = warmup(spec, &cfg, &limits, &clock, t, leaf, &mut life).await {
                        return (Verdict::Infrastructure(e), life, None);
                    }
                }
                leaf += 1;
                let scratch = cfg.scratch.join(format!("leaf-{leaf:04}"));
                let mut owned = match Owned::launch(spec, &scratch, &limits, &clock).await {
                    Ok(o) => o,
                    Err(LaunchError::NotReady(why, facts)) => {
                        life.cleanups.push(facts);
                        return (
                            Verdict::Infrastructure(format!("{target}: not ready ({why})")),
                            life,
                            None,
                        );
                    }
                    Err(e) => {
                        return (
                            Verdict::Infrastructure(format!("{target}: launch refused ({e:?})")),
                            life,
                            None,
                        );
                    }
                };
                life.launches.push(owned.facts.clone());
                let result = match job {
                    Job::Throughput(i, t) => {
                        throughput(
                            &owned,
                            p,
                            t,
                            &tag(&run_id, target, i, rep),
                            rep,
                            &run_id,
                            &sink,
                            &clock,
                        )
                        .await
                    }
                    Job::Wait(i, w) => {
                        waits(
                            &owned,
                            p,
                            w,
                            &tag(&run_id, target, i, rep),
                            rep,
                            &run_id,
                            &sink,
                            &clock,
                        )
                        .await
                    }
                };
                let facts = owned.shutdown().await;
                let proved = facts.proved();
                life.cleanups.push(facts);
                if !proved {
                    return (
                        Verdict::Infrastructure(format!("{target}: cleanup not proved")),
                        life,
                        None,
                    );
                }
                match result {
                    Ok(mut f) => failures.append(&mut f),
                    Err(e) => return (Verdict::Infrastructure(e), life, None),
                }
            }
        }
    }
    let dir = Arc::try_unwrap(sink)
        .ok()
        .expect("no sink clone outlives the run")
        .into_inner()
        .expect("sink lock");
    let rows = match crate::records::read_rows(dir.path(), &cfg.manifest) {
        Ok(r) => r,
        Err(e) => {
            return (
                Verdict::Infrastructure(format!("accounting: {e}")),
                life,
                None,
            );
        }
    };
    let report = crate::report::report(p, &cfg.manifest, &rows, &life);
    if let Err(e) = dir.write_artifact(
        "report.json",
        &serde_json::to_vec_pretty(&report).expect("report serializes"),
    ) {
        return (Verdict::Infrastructure(format!("report: {e}")), life, None);
    }
    match dir.finalize(life.cleanups.iter().all(CleanupFacts::proved)) {
        Ok(path) if failures.is_empty() => (Verdict::Clean, life, Some(path)),
        Ok(path) => (Verdict::WorkloadFailures(failures), life, Some(path)),
        Err(e) => (Verdict::Infrastructure(e), life, None),
    }
}

#[derive(Clone, Copy)]
enum Job<'a> {
    Throughput(usize, &'a ThroughputProfile),
    Wait(usize, &'a WaitProfile),
}

/// Warmup traffic on its own instance, discarded with it: it warms the
/// controller and host, never the measured server's state.
async fn warmup(
    spec: &LaunchSpec,
    cfg: &Config,
    limits: &Limits,
    clock: &Clock,
    t: &ThroughputProfile,
    leaf: u32,
    life: &mut Lifecycle,
) -> Result<(), String> {
    let scratch = cfg.scratch.join(format!("leaf-{leaf:04}"));
    let mut owned = match Owned::launch(spec, &scratch, limits, clock).await {
        Ok(o) => o,
        Err(LaunchError::NotReady(why, facts)) => {
            life.cleanups.push(facts);
            return Err(format!("{}: warmup not ready ({why})", spec.kind));
        }
        Err(e) => return Err(format!("{}: warmup launch refused ({e:?})", spec.kind)),
    };
    life.launches.push(owned.facts.clone());
    let step = Duration::from_millis(cfg.protocol.resources.request_deadline_ms);
    for i in 0..t.warmup_messages {
        let msg = authored(
            cfg.protocol.corpus.seed,
            "warmup00000000",
            i,
            &cfg.protocol.corpus.kinds,
        );
        if let Ok(mut s) = Smtp::open(owned.smtp, step).await {
            let _ = s.send(&msg, clock).await;
            s.quit().await;
        }
    }
    let facts = owned.shutdown().await;
    let proved = facts.proved();
    life.cleanups.push(facts);
    if proved {
        Ok(())
    } else {
        Err(format!("{}: warmup cleanup not proved", spec.kind))
    }
}

/// Closed-loop workers, one outstanding delivery each. Every index gets
/// exactly one row; an index the send deadline cut off is `unstarted`, not
/// dropped. Nothing is retried.
#[allow(clippy::too_many_arguments)]
async fn throughput(
    owned: &Owned,
    p: &Protocol,
    t: &ThroughputProfile,
    tag: &str,
    rep: u32,
    run: &str,
    sink: &Sink,
    clock: &Clock,
) -> Result<Vec<String>, String> {
    let next = Arc::new(AtomicU32::new(0));
    let accepted = Arc::new(Mutex::new(BTreeSet::new()));
    let send_deadline = Instant::now() + Duration::from_millis(t.send_deadline_ms);
    let step = Duration::from_millis(p.resources.request_deadline_ms);
    let window_start = clock.now_ns();
    let mut workers = Vec::new();
    for _ in 0..t.sessions {
        let (next, accepted, sink, clock) = (next.clone(), accepted.clone(), sink.clone(), *clock);
        let (addr, target, tag, run, profile) = (
            owned.smtp,
            owned.kind,
            tag.to_owned(),
            run.to_owned(),
            t.name.clone(),
        );
        let (seed, kinds, messages, connection) = (
            p.corpus.seed,
            p.corpus.kinds.clone(),
            t.messages,
            t.connection,
        );
        workers.push(tokio::spawn(async move {
            let mut session: Option<Smtp> = None;
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= messages {
                    break;
                }
                let invoked = clock.now_ns();
                let msg = authored(seed, &tag, i, &kinds);
                let mut r = row(&run, target, &profile, rep, Phase::Send, i, "unstarted");
                r.token = Some(msg.token.clone());
                r.invoked_ns = Some(invoked);
                if Instant::now() >= send_deadline {
                    r.reason = Some("send deadline".into());
                    emit(&sink, &r)?;
                    continue;
                }
                if session.is_none() {
                    match Smtp::open(addr, step).await {
                        Ok(s) => session = Some(s),
                        Err((code, e)) => {
                            r.outcome = "failed_before_terminator".into();
                            r.reason = Some(format!("connect: {}", e.reason()));
                            r.code = (code > 0).then_some(code);
                            emit(&sink, &r)?;
                            continue;
                        }
                    }
                }
                let s = session.as_mut().expect("opened above");
                let rec = s.send(&msg, &clock).await;
                r.start_ns = Some(rec.start_ns);
                r.ack_ns = rec.ack_ns;
                let ok = fill(&mut r, &rec.disposition);
                if ok {
                    accepted
                        .lock()
                        .expect("accepted lock")
                        .insert(msg.token.clone());
                }
                if (connection == Connection::Fresh || !ok)
                    && let Some(s) = session.take()
                {
                    s.quit().await;
                }
                emit(&sink, &r)?;
            }
            if let Some(s) = session.take() {
                s.quit().await;
            }
            Ok::<(), String>(())
        }));
    }
    for w in workers {
        w.await.map_err(|_| "a send worker panicked".to_owned())??;
    }
    let send_end = clock.now_ns();
    let accepted = std::mem::take(&mut *accepted.lock().expect("accepted lock"));

    // Verification: list until every accepted id is visible or the
    // deadline passes. Competitor acceptance is not presumed to be storage.
    let verify_deadline = Instant::now() + Duration::from_millis(t.verify_deadline_ms);
    let max_pages = t.messages / 250 + 4;
    let (listing, reason) = loop {
        let attempt = owned.list_all(max_pages).await;
        let done = match &attempt {
            Ok(l) => {
                let seen: BTreeSet<&String> = l.iter().map(|(tok, _)| tok).collect();
                accepted.iter().all(|a| seen.contains(a))
            }
            Err(_) => false,
        };
        if done || Instant::now() >= verify_deadline {
            break match attempt {
                Ok(l) => (l, (!done).then_some("verify deadline")),
                Err(e) => (Vec::new(), Some(e.reason())),
            };
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    let verified_ns = clock.now_ns();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (tok, id) in listing {
        groups.entry(tok).or_default().push(id);
    }
    let mut mismatches = 0;
    for (k, (tok, ids)) in groups.iter().enumerate() {
        let mut r = row(
            run,
            owned.kind,
            &t.name,
            rep,
            Phase::Visible,
            k as u32,
            "visible",
        );
        r.occurrences = Some(ids.len() as u64);
        if tok.is_empty() {
            r.outcome = "foreign".into();
        } else {
            r.token = Some(tok.clone());
            if !accepted.contains(tok) {
                r.outcome = "unexpected".into();
            }
            // Untimed fidelity: outside both windows.
            let index = tok.rsplit('-').next().and_then(|n| n.parse().ok());
            let ok = match (index, owned.fetch(&ids[0]).await) {
                (Some(i), Ok(f)) => fidelity(&authored(p.corpus.seed, tag, i, &p.corpus.kinds), &f),
                _ => false,
            };
            mismatches += u32::from(!ok);
            r.fidelity_ok = Some(ok);
        }
        emit(sink, &r)?;
    }
    let mut v = row(
        run,
        owned.kind,
        &t.name,
        rep,
        Phase::Verified,
        0,
        if reason.is_none() {
            "verified"
        } else {
            "incomplete"
        },
    );
    v.reason = reason.map(str::to_owned);
    v.start_ns = Some(window_start);
    v.ack_ns = Some(send_end);
    v.observed_ns = Some(verified_ns);
    v.requests = Some(owned.http.requests());
    emit(sink, &v)?;
    let visible = groups.keys().filter(|t| accepted.contains(*t)).count();
    let exact = reason.is_none()
        && accepted.len() as u32 == t.messages
        && visible == accepted.len()
        && groups.len() == accepted.len()
        && groups.values().all(|ids| ids.len() == 1)
        && mismatches == 0;
    Ok(if exact {
        Vec::new()
    } else {
        vec![format!(
            "{} {} rep {rep}: not an exact count",
            owned.kind, t.name
        )]
    })
}

/// Map a disposition onto a row; `true` when accepted.
fn fill(r: &mut Row, d: &Disposition) -> bool {
    match d {
        Disposition::Accepted { code } => {
            r.outcome = "accepted".into();
            r.code = Some(*code);
            true
        }
        Disposition::Rejected { phase, code } => {
            r.outcome = "rejected".into();
            r.code = Some(*code);
            r.reason = Some((*phase).into());
            false
        }
        Disposition::FailedBeforeTerminator { phase, reason } => {
            r.outcome = "failed_before_terminator".into();
            r.reason = Some(format!("{phase}: {reason}"));
            false
        }
        Disposition::Ambiguous { reason } => {
            r.outcome = "ambiguous".into();
            r.reason = Some((*reason).into());
            false
        }
    }
}

pub fn fidelity(m: &Message, f: &crate::targets::Fetched) -> bool {
    f.subject.as_deref() == Some(m.subject.as_str())
        && same_body(m.text.as_deref(), f.text.as_deref())
        && same_body(m.html.as_deref(), f.html.as_deref())
}

/// Present, future and empty waits. Every attempt is one row whatever its
/// outcome; nothing is clamped, and a timeout is a censored sample.
#[allow(clippy::too_many_arguments)]
async fn waits(
    owned: &Owned,
    p: &Protocol,
    w: &WaitProfile,
    tag: &str,
    rep: u32,
    run: &str,
    sink: &Sink,
    clock: &Clock,
) -> Result<Vec<String>, String> {
    let obs = w
        .observation
        .iter()
        .find(|o| o.target == owned.kind)
        .ok_or("no observation mode")?;
    let (mode, interval) = (
        obs.mode,
        Duration::from_millis(obs.interval_ms.unwrap_or(0)),
    );
    let deadline = Duration::from_millis(w.deadline_ms);
    let step = Duration::from_millis(p.resources.request_deadline_ms);
    let mut failures = Vec::new();
    let mut fail = |r: &Row, want: &str| {
        if r.outcome != want {
            failures.push(format!(
                "{} {} rep {rep} {:?} {}: {}",
                owned.kind, w.name, r.phase, r.sample, r.outcome
            ));
        }
    };
    for i in 0..w.present {
        let msg = authored(p.corpus.seed, tag, i, &p.corpus.kinds);
        let mut r = row(
            run,
            owned.kind,
            &w.name,
            rep,
            Phase::Present,
            i,
            "send_failed",
        );
        r.token = Some(msg.token.clone());
        if send_one(owned, &msg, step, clock, &mut r).await {
            r.invoked_ns = Some(clock.now_ns());
            observe(
                owned,
                mode,
                (interval, step),
                &msg.token,
                deadline,
                clock,
                &mut r,
            )
            .await;
        }
        fail(&r, "observed");
        emit(sink, &r)?;
    }
    for i in 0..w.future {
        let msg = authored(p.corpus.seed, tag, 200_000 + i, &p.corpus.kinds);
        let mut r = row(
            run,
            owned.kind,
            &w.name,
            rep,
            Phase::Future,
            i,
            "send_failed",
        );
        r.token = Some(msg.token.clone());
        r.invoked_ns = Some(clock.now_ns());
        // The observer is polled first, so its request is issued before the
        // send starts; issuing is not server subscription readiness.
        let mut send_row = r.clone();
        let mut obs_row = r.clone();
        let (_, sent) = tokio::join!(
            observe(
                owned,
                mode,
                (interval, step),
                &msg.token,
                deadline,
                clock,
                &mut obs_row
            ),
            send_one(owned, &msg, step, clock, &mut send_row),
        );
        r.start_ns = send_row.start_ns;
        r.ack_ns = send_row.ack_ns;
        r.code = send_row.code;
        if sent {
            r.outcome = obs_row.outcome;
            r.reason = obs_row.reason;
            r.observed_ns = obs_row.observed_ns;
        } else {
            r.reason = send_row.reason;
        }
        r.requests = obs_row.requests;
        fail(&r, "observed");
        emit(sink, &r)?;
    }
    let empty_deadline = Duration::from_millis(w.empty_deadline_ms);
    for i in 0..w.empty {
        let token = authored(p.corpus.seed, tag, 400_000 + i, &p.corpus.kinds).token;
        let mut r = row(run, owned.kind, &w.name, rep, Phase::Empty, i, "timeout");
        r.token = Some(token.clone());
        r.invoked_ns = Some(clock.now_ns());
        observe(
            owned,
            mode,
            (interval, step),
            &token,
            empty_deadline,
            clock,
            &mut r,
        )
        .await;
        fail(&r, "timeout");
        emit(sink, &r)?;
    }
    Ok(failures)
}

/// One fresh session, one message; records start/ack/code on `r`.
async fn send_one(
    owned: &Owned,
    msg: &Message,
    step: Duration,
    clock: &Clock,
    r: &mut Row,
) -> bool {
    match Smtp::open(owned.smtp, step).await {
        Ok(mut s) => {
            let rec = s.send(msg, clock).await;
            s.quit().await;
            r.start_ns = Some(rec.start_ns);
            r.ack_ns = rec.ack_ns;
            let ok = fill(r, &rec.disposition);
            if !ok {
                r.reason = Some(format!(
                    "{}: {}",
                    r.outcome,
                    r.reason.take().unwrap_or_default()
                ));
                r.outcome = "send_failed".into();
            }
            ok
        }
        Err((_, e)) => {
            r.outcome = "send_failed".into();
            r.reason = Some(format!("connect: {}", e.reason()));
            false
        }
    }
}

/// Observe `token` within `deadline` of now; sets outcome, observed_ns and
/// the request count on `r`. An answer carrying any other message is
/// `unrelated`, whatever else it carries.
#[allow(clippy::too_many_arguments)]
async fn observe(
    owned: &Owned,
    mode: Observation,
    (interval, step): (Duration, Duration),
    token: &str,
    deadline: Duration,
    clock: &Clock,
    r: &mut Row,
) {
    let before = owned.http.requests();
    let until = Instant::now() + deadline;
    let judge = |tokens: &[String]| {
        if tokens.iter().any(|t| t != token) {
            Some("unrelated")
        } else if tokens.is_empty() {
            None
        } else {
            Some("observed")
        }
    };
    let result: Result<Option<&'static str>, &'static str> = match mode {
        Observation::Native => owned
            .native_wait(token, deadline)
            .await
            .map(|a| a.and_then(|t| judge(&t)))
            .map_err(|e| e.reason()),
        // The adapter's poll, so both observe under one tested rule
        // (proposed decision 0093 §5): a scan at once and `interval` after
        // each completed one; only a scan that ended by the cutoff can
        // observe in time, a later match is late (censored), and a scan
        // still running one request bound past the cutoff is an error.
        Observation::Poll => {
            let cutoff = tokio::time::Instant::from_std(until);
            let frame = Frame {
                ceiling: cutoff + step,
                cutoff,
                response: cutoff + step,
            };
            let mut s = FindScan {
                owned,
                token,
                last: Vec::new(),
            };
            poll_wait(&mut s, frame, interval)
                .await
                .map(|v| match (v["status"].as_str(), judge(&s.last)) {
                    (_, Some("unrelated")) => Some("unrelated"),
                    (Some("found"), o) => o,
                    (Some("late"), Some(_)) => Some("late"),
                    _ => None,
                })
                .map_err(|e| e.reason())
        }
    };
    let at = clock.now_ns();
    r.requests = Some(owned.http.requests() - before);
    match result {
        Ok(Some(o)) => {
            r.outcome = o.into();
            r.observed_ns = Some(at);
        }
        Ok(None) => r.outcome = "timeout".into(),
        Err(e) => {
            r.outcome = "error".into();
            r.reason = Some(e.into());
        }
    }
}

/// One complete `find` per scan, keeping the last answer to judge.
struct FindScan<'a> {
    owned: &'a Owned,
    token: &'a str,
    last: Vec<String>,
}

impl Scan for FindScan<'_> {
    async fn scan(&mut self) -> Result<(u64, Vec<serde_json::Value>), ToolError> {
        self.last = self
            .owned
            .find(self.token)
            .await
            .map_err(|e| ToolError::Transport(e.reason()))?;
        Ok((self.last.len() as u64, Vec::new()))
    }

    fn requests(&self) -> u32 {
        0
    }
}
