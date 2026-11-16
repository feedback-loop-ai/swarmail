//! The one sequential schedule (spec R3): for each repetition, targets in
//! the protocol's counterbalanced order, every profile on a fresh owned
//! instance. Rows stream as they happen; cleanup that is not proved stops
//! all later work.

use crate::mailpit_mcp::{Bounds, Frame, Scan, ToolError, poll_wait};
use crate::protocol::{
    Connection, Observation, Protocol, SourceLock, TargetKind, ThroughputProfile, WaitProfile,
};
use crate::records::{
    Manifest, Phase, Planned, PlannedWindow, ROW_VERSION, ResourceRow, Row, RunDir, WindowKind,
    WindowOutcome, WindowRow,
};
use crate::resources::{Leaf, MemEvents, Sampler, Snap};
use crate::targets::{
    CleanupFacts, Keepers, LaunchError, LaunchFacts, LaunchSpec, Limits, Owned, swarmail_template,
};
use crate::wire::{Clock, Disposition, Message, Smtp, WireError, authored, same_body};
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
    /// Every launch's cleanup worker; the caller settles them, which is
    /// what proves an interrupted run's cleanup.
    pub keepers: Keepers,
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

/// `--flag value` pairs; a repeated flag other than `--bin` and `--runtime`
/// (one of each per target, so two targets may each need a runtime: GreenMail's
/// JRE and the Alkemio row's musl loader), a flag without a value or a stray
/// word refuses.
pub fn flags(args: &[String]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let name = flag
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {flag:?}"))?;
        let value = it.next().ok_or_else(|| format!("--{name} needs a value"))?;
        let slot = out.entry(name.to_owned()).or_default();
        if !slot.is_empty() && name != "bin" && name != "runtime" {
            return Err(format!("--{name} given twice"));
        }
        slot.push(value.clone());
    }
    Ok(out)
}

/// Bind the wrapper's `--bin TARGET=PATH` and `--runtime TARGET=DIR` values
/// to launch specs, with no file or process I/O. A target is given at most
/// one of each; a published does-not-start target takes neither and has no
/// spec; every launched target needs its `--bin`; a runtime needs a pin that
/// states one (and a pin that states one needs it).
pub fn launch_specs(
    p: &Protocol,
    s: &SourceLock,
    bins: &[String],
    runtimes: &[String],
) -> Result<BTreeMap<TargetKind, LaunchSpec>, String> {
    launch_specs_of(p, s, bins, runtimes, &p.launch_order(0).collect::<Vec<_>>())
}

/// [`launch_specs`] for an explicit set of targets: a protocol 4 chunk runs
/// one target and so needs only that target's `--bin` (and `--runtime`).
pub fn launch_specs_of(
    p: &Protocol,
    s: &SourceLock,
    bins: &[String],
    runtimes: &[String],
    wanted: &[TargetKind],
) -> Result<BTreeMap<TargetKind, LaunchSpec>, String> {
    let mut bin_of = BTreeMap::new();
    for b in bins {
        let (name, path) = b.split_once('=').ok_or("--bin takes TARGET=PATH")?;
        let t = TargetKind::parse(name).ok_or_else(|| format!("unknown target {name:?}"))?;
        if bin_of.insert(t, PathBuf::from(path)).is_some() {
            return Err(format!("--bin {t} given twice"));
        }
    }
    let mut runtime_of = BTreeMap::new();
    for r in runtimes {
        let (name, dir) = r.split_once('=').ok_or("--runtime takes TARGET=DIR")?;
        let t = TargetKind::parse(name).ok_or_else(|| format!("unknown target {name:?}"))?;
        if runtime_of.insert(t, PathBuf::from(dir)).is_some() {
            return Err(format!("--runtime {t} given twice"));
        }
    }
    let mut specs = BTreeMap::new();
    // A published does-not-start target is never launched: it takes no
    // --bin and has no spec (a --bin for it is refused below). The rotation
    // is the same for every repetition's membership, so the first one's
    // launch order is the launched set.
    for t in wanted.iter().copied() {
        let binary = bin_of
            .remove(&t)
            .ok_or_else(|| format!("--bin {t}=PATH is required"))?;
        let spec = match t {
            TargetKind::Swarmail => {
                let cap = p
                    .storage
                    .retention
                    .iter()
                    .find(|r| r.target == t)
                    .and_then(|r| r.max_messages);
                LaunchSpec::new(t, binary, None, swarmail_template(cap.unwrap_or(0)))
            }
            _ => {
                let pin = s.pin(t).ok_or_else(|| format!("{t}: no source pin"))?;
                // The protocol's cap is what lock 5's placeholder resolves
                // to; `retention_matches` holds it equal to the pin's own.
                let cap = p
                    .storage
                    .retention
                    .iter()
                    .find(|r| r.target == t)
                    .and_then(|r| r.max_messages);
                LaunchSpec::from_pin(t, binary, runtime_of.remove(&t), pin, cap)?
            }
        };
        specs.insert(t, spec);
    }
    if let Some(t) = bin_of.keys().next() {
        return Err(format!(
            "--bin {t} names a target the protocol does not declare or publishes as not starting"
        ));
    }
    if let Some(t) = runtime_of.keys().next() {
        return Err(format!(
            "--runtime {t} names a target that pins no runtime or is not declared"
        ));
    }
    Ok(specs)
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
        // A published does-not-start target is never planned: it has no
        // coordinate to produce (its report row is blocked, not measured).
        for target in p.launch_order(rep) {
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

pub type Sink = Arc<Mutex<RunDir>>;

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
        for target in p.launch_order(rep) {
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
                let mut owned = match Owned::launch(spec, &scratch, &limits, &clock, &cfg.keepers)
                    .await
                {
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
                            None,
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
    // Every executable must still be the bytes it was launched from: a
    // replacement during the run would leave the record standing behind
    // a binary nobody measured.
    for launched in &life.launches {
        let same = cfg
            .specs
            .get(&launched.target)
            .is_some_and(|spec| crate::targets::spec_unchanged(spec, launched));
        if !same {
            return (
                Verdict::Infrastructure(format!(
                    "{}: executable changed during the run",
                    launched.target
                )),
                life,
                None,
            );
        }
    }
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
    let mut owned = match Owned::launch(spec, &scratch, limits, clock, &cfg.keepers).await {
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
    probe: Option<&Probe>,
) -> Result<Vec<String>, String> {
    let next = Arc::new(AtomicU32::new(0));
    let accepted = Arc::new(Mutex::new(BTreeSet::new()));
    let send_deadline = Instant::now() + Duration::from_millis(t.send_deadline_ms);
    let step = Duration::from_millis(p.resources.request_deadline_ms);
    let window_start = clock.now_ns();
    if let Some(probe) = probe {
        probe.mark(Mark::Start);
    }
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
                            connect_failure(&mut r, code, e);
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
        // A target that has exited (an OOM kill, a crash) will not list
        // again: protocol 4 records the outcome instead of spinning to the
        // deadline. Earlier protocols keep their loop as it was.
        let gone = probe.is_some() && owned.has_exited();
        if done || gone || Instant::now() >= verify_deadline {
            break match attempt {
                Ok(l) => (
                    l,
                    (!done).then_some(if gone {
                        "target exited"
                    } else {
                        "verify deadline"
                    }),
                ),
                Err(e) => (Vec::new(), Some(e.reason())),
            };
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    let verified_ns = clock.now_ns();
    if let Some(probe) = probe {
        probe.mark(Mark::End);
    }
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
    v.requests = Some(owned.requests());
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

/// Record a session that could not be opened: a failure before the
/// terminator. A peer that answered with a reply code refused the session
/// itself, and the code is kept as measured; anything else names the wire
/// failure.
pub fn connect_failure(r: &mut Row, code: u16, e: WireError) {
    r.outcome = "failed_before_terminator".into();
    r.code = (code > 0).then_some(code);
    r.reason = Some(match r.code {
        Some(_) => "connect: refused".to_owned(),
        None => format!("connect: {}", e.reason()),
    });
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
    let before = owned.requests();
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
            // The driver's operation is its wait plus one request bound,
            // declared in the protocol; the one constructor composes it.
            let bounds = Bounds {
                operation: deadline + step,
                request: step,
                poll_interval: interval,
                ..Bounds::default()
            };
            let frame = Frame::new(tokio::time::Instant::now(), &bounds, deadline);
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
    r.requests = Some(owned.requests() - before);
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

// ============================================================================
// Protocol 4: the cost chunk (accepted decision 0132 §4, §11)
// ============================================================================

/// Five seconds without traffic after readiness (0132 §4.3).
pub const IDLE_READY: Duration = Duration::from_secs(5);
/// The figure of an idle point is the mean over the last two seconds of it.
pub const IDLE_TAIL_NS: u64 = 2_000_000_000;

/// One bounded unit of a v4 collection: one driver process for one
/// coordinate. Story A builds the cost section's; later stories add the
/// others' to the same enum and the same plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chunk {
    /// All of one target's cost profiles of one repetition (0-based, as the
    /// rows number them).
    Cost { target: TargetKind, repetition: u32 },
}

impl Chunk {
    /// `cost.<target>.r<repetition>`.
    pub fn name(&self) -> String {
        match self {
            Chunk::Cost { target, repetition } => format!("cost.{target}.r{repetition}"),
        }
    }

    /// Parse a coordinate against the protocol: an unknown section, a target
    /// the protocol does not launch or a repetition it does not have is
    /// refused by name.
    pub fn parse(text: &str, p: &Protocol) -> Result<Chunk, String> {
        let parts: Vec<&str> = text.split('.').collect();
        let [section, target, rep] = parts.as_slice() else {
            return Err(format!("{text:?} is not a chunk coordinate"));
        };
        if *section != "cost" {
            return Err(format!(
                "chunk {text:?}: section {section:?} is not implemented (only cost)"
            ));
        }
        let target = TargetKind::parse(target)
            .ok_or_else(|| format!("chunk {text:?}: unknown target {target:?}"))?;
        let repetition: u32 = rep
            .strip_prefix('r')
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| format!("chunk {text:?}: {rep:?} is not r<repetition>"))?;
        let chunk = Chunk::Cost { target, repetition };
        if !plan_v4(p).contains(&chunk) {
            return Err(format!("chunk {text:?} is not in the protocol's plan"));
        }
        Ok(chunk)
    }

    pub fn target(&self) -> TargetKind {
        match self {
            Chunk::Cost { target, .. } => *target,
        }
    }

    pub fn repetition(&self) -> u32 {
        match self {
            Chunk::Cost { repetition, .. } => *repetition,
        }
    }
}

/// The chunk plan, derived from the protocol and never computed by the
/// wrapper: for each repetition, for each launched target in that
/// repetition's rotation, one cost chunk.
pub fn plan_v4(p: &Protocol) -> Vec<Chunk> {
    let reps = p
        .throughput
        .iter()
        .map(|t| t.repetitions)
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for repetition in 0..reps {
        for target in p.launch_order(repetition) {
            out.push(Chunk::Cost { target, repetition });
        }
    }
    out
}

/// What one chunk will produce: the message rows of its target's profiles
/// of its repetition, and the three windows of each.
pub fn plan_chunk(p: &Protocol, chunk: &Chunk) -> (Vec<Planned>, Vec<PlannedWindow>) {
    let (target, rep) = (chunk.target(), chunk.repetition());
    let mut planned = Vec::new();
    let mut windows = Vec::new();
    for t in p.throughput.iter().filter(|t| t.repetitions > rep) {
        let mk = |phase, rows| Planned {
            target,
            profile: t.name.clone(),
            repetition: rep,
            phase,
            rows,
        };
        planned.push(mk(Phase::Send, Some(t.messages)));
        planned.push(mk(Phase::Visible, None));
        planned.push(mk(Phase::Verified, Some(1)));
        for window in [
            WindowKind::Ready,
            WindowKind::IdleReady,
            WindowKind::Measure,
        ] {
            windows.push(PlannedWindow {
                target,
                profile: t.name.clone(),
                repetition: rep,
                window,
            });
        }
    }
    (planned, windows)
}

/// What `run` checks about protocol 4's chunk and candidate before anything
/// launches. Protocols 1 to 3 have no chunks and refuse `--chunk`; protocol 4
/// needs one, and a v4 collection refuses a candidate with no stated commit or
/// a dirty tree, by name (0132 §2.3: the 2026-10-30 v2 collection was admitted
/// as raw measurement only because its tree was dirty; v4 makes that a
/// refusal).
pub fn v4_preflight(
    p: &Protocol,
    mode: &str,
    chunk: Option<&str>,
    commit: Option<&str>,
    dirty: Option<bool>,
) -> Result<Option<Chunk>, String> {
    if p.protocol_version != crate::protocol::PROTOCOL_VERSION_4 {
        return match chunk {
            Some(c) => Err(format!(
                "protocol {} has no chunks: --chunk {c} is for protocol 4",
                p.protocol_version
            )),
            None => Ok(None),
        };
    }
    let Some(text) = chunk else {
        return Err("protocol 4 needs --chunk cost.<target>.r<repetition>".into());
    };
    let chunk = Chunk::parse(text, p)?;
    if mode == "collect" {
        if commit.is_none() {
            return Err("a v4 collection needs the candidate's commit: --commit SHA".into());
        }
        if dirty != Some(false) {
            return Err(
                "a v4 collection refuses a candidate tree that is dirty or not stated clean: --dirty false"
                    .into(),
            );
        }
    }
    Ok(Some(chunk))
}

/// How a window ended, from the leaf's `memory.events` at its bounds and
/// whether the target exited meanwhile. Only a rise of `oom_kill` is an OOM;
/// `max` alone is pressure at the limit.
pub fn window_outcome(start: &MemEvents, end: &MemEvents, exited: bool) -> WindowOutcome {
    if end.oom_kill > start.oom_kill {
        WindowOutcome::OomKilled
    } else if exited {
        WindowOutcome::TargetExited
    } else {
        WindowOutcome::Measured
    }
}

#[derive(Clone, Copy)]
enum Mark {
    Start,
    End,
}

/// The readings taken at the bounds of one window, and what went wrong
/// taking them.
#[derive(Default)]
struct Marks {
    start: Option<Snap>,
    end: Option<Snap>,
    affinity_held: Option<bool>,
    error: Option<String>,
}

/// Takes the leaf's counters at the bounds of the profile's send-and-verify
/// window, from inside `throughput`, so the end bound is the instant
/// verification completed and the untimed fidelity reads fall outside it.
struct Probe {
    leaf: Leaf,
    run: String,
    target: TargetKind,
    clock: Clock,
    sink: Sink,
    target_cpus: Vec<u32>,
    marks: Mutex<Marks>,
}

impl Probe {
    fn mark(&self, which: Mark) {
        let taken = bound(&self.leaf, &self.run, self.target, &self.clock, &self.sink);
        let held = match which {
            Mark::End => self.leaf.read_procs().map(|pids| {
                crate::resources::affinity_held(
                    std::path::Path::new("/proc"),
                    &pids,
                    &self.target_cpus,
                )
            }),
            Mark::Start => Ok(true),
        };
        let mut m = self.marks.lock().expect("marks lock");
        match (taken, held) {
            (Ok(snap), Ok(held)) => match which {
                Mark::Start => m.start = Some(snap),
                Mark::End => {
                    m.end = Some(snap);
                    m.affinity_held = Some(held);
                }
            },
            (Err(e), _) | (_, Err(e)) => {
                m.error.get_or_insert(e);
            }
        }
    }
}

/// One bound of a window: the counters, then a `sample` row, so that the
/// sample's time is the bound and no window is empty of samples. The
/// returned snapshot carries the sample's time.
fn bound(
    leaf: &Leaf,
    run: &str,
    target: TargetKind,
    clock: &Clock,
    sink: &Sink,
) -> Result<Snap, String> {
    let mut snap = leaf.snap(clock)?;
    let row = leaf.sample(run, target, clock)?;
    snap.t_ns = row.t_ns;
    sink.lock()
        .expect("sink lock")
        .append_resource(&ResourceRow::Sample(row))
        .map_err(|e| format!("sample write failed: {e}"))?;
    Ok(snap)
}

#[allow(clippy::too_many_arguments)]
pub fn window_row(
    run: &str,
    launch: &str,
    (target, profile, rep): (TargetKind, &str, u32),
    window: WindowKind,
    (a, b): (&Snap, &Snap),
    affinity_held: bool,
    scratch_bytes: Option<u64>,
    outcome: WindowOutcome,
) -> WindowRow {
    WindowRow {
        row_version: crate::records::RESOURCE_ROW_VERSION,
        run_id: run.to_owned(),
        launch: launch.to_owned(),
        target,
        profile: profile.to_owned(),
        repetition: rep,
        window,
        start_ns: a.t_ns,
        end_ns: b.t_ns,
        cpu_start: a.cpu,
        cpu_end: b.cpu,
        events_start: a.events,
        events_end: b.events,
        affinity_held,
        scratch_bytes,
        outcome,
    }
}

fn emit_window(sink: &Sink, w: WindowRow) -> Result<(), String> {
    sink.lock()
        .expect("sink lock")
        .append_resource(&ResourceRow::Window(w))
        .map_err(|e| format!("sample write failed: {e}"))
}

/// A profile the chunk deadline stopped before it began: every send is an
/// `unstarted` row, the verification row says why, and each window is
/// `chunk_deadline` with nothing measured. Nothing was launched.
pub fn skip_profile(
    run: &str,
    t: &ThroughputProfile,
    (target, rep): (TargetKind, u32),
    sink: &Sink,
    clock: &Clock,
) -> Result<(), String> {
    for i in 0..t.messages {
        let mut r = row(run, target, &t.name, rep, Phase::Send, i, "unstarted");
        r.reason = Some("chunk deadline".into());
        emit(sink, &r)?;
    }
    let now = clock.now_ns();
    let mut v = row(run, target, &t.name, rep, Phase::Verified, 0, "incomplete");
    v.reason = Some("chunk deadline".into());
    v.start_ns = Some(now);
    v.ack_ns = Some(now);
    v.observed_ns = Some(now);
    v.requests = Some(0);
    emit(sink, &v)?;
    let none = Snap {
        t_ns: now,
        cpu: Default::default(),
        events: Default::default(),
    };
    for kind in [
        WindowKind::Ready,
        WindowKind::IdleReady,
        WindowKind::Measure,
    ] {
        emit_window(
            sink,
            window_row(
                run,
                "none",
                (target, &t.name, rep),
                kind,
                (&none, &none),
                true,
                None,
                WindowOutcome::ChunkDeadline,
            ),
        )?;
    }
    Ok(())
}

/// Run one cost chunk: that target's cost profiles of that repetition, in
/// order, each as protocol 3 does it (a warmup instance, then a fresh measured
/// one), both in leaves. A target that exits in a window is a measured outcome
/// and the chunk goes on; the chunk deadline stops profiles that have not
/// begun. The directory is sealed on every outcome but an infrastructure
/// failure.
pub async fn run_chunk(
    cfg: Config,
    chunk: Chunk,
    clock: Clock,
) -> (Verdict, Lifecycle, Option<PathBuf>) {
    let mut life = Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let infra = |e: String, life: Lifecycle| (Verdict::Infrastructure(e), life, None);
    let dir = match RunDir::create(&cfg.output, &cfg.manifest) {
        Ok(d) => d,
        Err(e) => return infra(format!("output: {e}"), life),
    };
    let sink: Sink = Arc::new(Mutex::new(dir));
    let limits = limits(&cfg.protocol);
    let p = &cfg.protocol;
    let (target, rep) = (chunk.target(), chunk.repetition());
    let Some(spec) = cfg.specs.get(&target) else {
        return infra(format!("{target}: no launch spec"), life);
    };
    let Some(scope) = spec.extra.scope.clone() else {
        return infra(format!("{target}: a protocol 4 launch has no scope"), life);
    };
    let deadline = Duration::from_millis(p.resources.chunk_deadline_ms.unwrap_or(540_000));
    let started = Instant::now();
    let mut failures = Vec::new();
    let mut leaf_no = 0u32;
    for (i, t) in p
        .throughput
        .iter()
        .enumerate()
        .filter(|(_, t)| t.repetitions > rep)
    {
        if started.elapsed() >= deadline {
            if let Err(e) = skip_profile(&cfg.manifest.run_id, t, (target, rep), &sink, &clock) {
                return infra(e, life);
            }
            failures.push(format!("{target} {} rep {rep}: chunk deadline", t.name));
            continue;
        }
        if t.warmup_messages > 0 {
            leaf_no += 1;
            if let Err(e) = warmup(spec, &cfg, &limits, &clock, t, leaf_no, &mut life).await {
                return infra(e, life);
            }
        }
        leaf_no += 1;
        match measured_instance(
            (spec, &scope.layout.target_cpus),
            &cfg,
            &limits,
            &clock,
            (i, t, rep),
            leaf_no,
            &sink,
            &mut life,
        )
        .await
        {
            Ok(mut f) => failures.append(&mut f),
            Err(e) => return infra(e, life),
        }
    }
    let dir = Arc::try_unwrap(sink)
        .ok()
        .expect("no sink clone outlives the chunk")
        .into_inner()
        .expect("sink lock");
    let (rows, resources) = match crate::records::read_run(dir.path(), &cfg.manifest) {
        Ok(r) => r,
        Err(e) => return infra(format!("accounting: {e}"), life),
    };
    for launched in &life.launches {
        let same = cfg
            .specs
            .get(&launched.target)
            .is_some_and(|spec| crate::targets::spec_unchanged(spec, launched));
        if !same {
            return infra(
                format!("{}: executable changed during the run", launched.target),
                life,
            );
        }
    }
    let report = crate::report::report_v4(p, &cfg.manifest, &rows, &[&resources[..]], &life);
    if let Err(e) = dir.write_artifact(
        "report.json",
        &serde_json::to_vec_pretty(&report).expect("report serializes"),
    ) {
        return infra(format!("report: {e}"), life);
    }
    match dir.finalize(life.cleanups.iter().all(CleanupFacts::proved)) {
        Ok(path) if failures.is_empty() => (Verdict::Clean, life, Some(path)),
        Ok(path) => (Verdict::WorkloadFailures(failures), life, Some(path)),
        Err(e) => infra(e, life),
    }
}

/// One fresh, sampled instance of a target: ready, idle, measure, then its
/// whole cleanup, which must be proved. Returns the workload failures it
/// saw; an error is an infrastructure failure.
#[allow(clippy::too_many_arguments)]
async fn measured_instance(
    (spec, target_cpus): (&LaunchSpec, &[u32]),
    cfg: &Config,
    limits: &Limits,
    clock: &Clock,
    (i, t, rep): (usize, &ThroughputProfile, u32),
    leaf_no: u32,
    sink: &Sink,
    life: &mut Lifecycle,
) -> Result<Vec<String>, String> {
    let p = &cfg.protocol;
    let run = cfg.manifest.run_id.clone();
    let target = spec.kind;
    let scratch = cfg.scratch.join(format!("leaf-{leaf_no:04}"));
    let interval = Duration::from_millis(p.resources.sample_interval_ms.unwrap_or(100));
    // The sampler starts when the leaf is created, from inside the launch.
    let sampler: Arc<Mutex<Option<Sampler>>> = Arc::default();
    let created: Arc<Mutex<Option<u64>>> = Arc::default();
    let hook = {
        let (sampler, created, sink, run, clock) = (
            sampler.clone(),
            created.clone(),
            sink.clone(),
            run.clone(),
            *clock,
        );
        crate::targets::LeafHook(Arc::new(move |leaf: &Leaf| {
            *created.lock().expect("created lock") = Some(clock.now_ns());
            let emit: crate::resources::Emit = {
                let sink = sink.clone();
                Arc::new(move |row| {
                    sink.lock()
                        .expect("sink lock")
                        .append_resource(&ResourceRow::Sample(row))
                        .map_err(|e| format!("sample write failed: {e}"))
                })
            };
            let s = Sampler::spawn(leaf.clone(), run.clone(), target, clock, interval, emit);
            *sampler.lock().expect("sampler lock") = Some(s);
        }))
    };
    let mut spec = spec.clone();
    spec.extra.on_leaf = Some(hook);
    let stop_sampler = || async {
        let s = sampler.lock().expect("sampler lock").take();
        match s {
            Some(s) => s.stop().await.map(|_| ()),
            None => Ok(()),
        }
    };
    let mut owned = match Owned::launch(&spec, &scratch, limits, clock, &cfg.keepers).await {
        Ok(o) => o,
        Err(e) => {
            let _ = stop_sampler().await;
            return Err(match e {
                LaunchError::NotReady(why, facts) => {
                    life.cleanups.push(facts);
                    format!("{target}: not ready ({why})")
                }
                LaunchError::Leaf(why, facts) => {
                    life.cleanups.extend(facts);
                    format!("{target}: {why}")
                }
                other => format!("{target}: launch refused ({other:?})"),
            });
        }
    };
    life.launches.push(owned.facts.clone());
    let leaf = owned.leaf.clone().ok_or("a leaf launch has no leaf")?;
    let launch = leaf.name().to_owned();
    let created_ns = created
        .lock()
        .expect("created lock")
        .unwrap_or_else(|| clock.now_ns());
    let key = (target, t.name.as_str(), rep);
    let mut failures = Vec::new();
    let mut note = |label: WindowKind, outcome: WindowOutcome| {
        if outcome != WindowOutcome::Measured {
            failures.push(format!(
                "{target} {} rep {rep}: {label:?} {outcome:?}",
                t.name
            ));
        }
    };
    // ready: leaf creation to readiness. A new leaf's counters start at zero.
    let ready = bound(&leaf, &run, target, clock, sink)?;
    let zero = Snap {
        t_ns: created_ns,
        cpu: Default::default(),
        events: Default::default(),
    };
    let outcome = window_outcome(&zero.events, &ready.events, owned.has_exited());
    note(WindowKind::Ready, outcome);
    let row = window_row(
        &run,
        &launch,
        key,
        WindowKind::Ready,
        (&zero, &ready),
        true,
        None,
        outcome,
    );
    emit_window(sink, row)?;
    // idle_ready: five seconds without traffic.
    let idle_a = bound(&leaf, &run, target, clock, sink)?;
    tokio::time::sleep(IDLE_READY).await;
    let idle_b = bound(&leaf, &run, target, clock, sink)?;
    let scratch_bytes = crate::resources::allocated_bytes(&scratch).ok();
    let outcome = window_outcome(&idle_a.events, &idle_b.events, owned.has_exited());
    note(WindowKind::IdleReady, outcome);
    let row = window_row(
        &run,
        &launch,
        key,
        WindowKind::IdleReady,
        (&idle_a, &idle_b),
        true,
        scratch_bytes,
        outcome,
    );
    emit_window(sink, row)?;
    // measure: the profile's send-and-verify window.
    let probe = Probe {
        leaf: leaf.clone(),
        run: run.clone(),
        target,
        clock: *clock,
        sink: sink.clone(),
        target_cpus: target_cpus.to_vec(),
        marks: Mutex::new(Marks::default()),
    };
    let result = throughput(
        &owned,
        p,
        t,
        &tag(&run, target, i, rep),
        rep,
        &run,
        sink,
        clock,
        Some(&probe),
    )
    .await;
    let marks = probe.marks.into_inner().expect("marks lock");
    let measured = match (marks.start, marks.end, marks.error) {
        (Some(a), Some(b), None) => {
            let outcome = window_outcome(&a.events, &b.events, owned.has_exited());
            note(WindowKind::Measure, outcome);
            let row = window_row(
                &run,
                &launch,
                key,
                WindowKind::Measure,
                (&a, &b),
                marks.affinity_held.unwrap_or(false),
                None,
                outcome,
            );
            emit_window(sink, row)
        }
        (_, _, Some(e)) => Err(format!("{target}: window bounds: {e}")),
        _ => Err(format!("{target}: the measurement window never closed")),
    };
    let sampled = stop_sampler().await;
    let facts = owned.shutdown().await;
    let proved = facts.proved();
    life.cleanups.push(facts);
    if !proved {
        return Err(format!("{target}: cleanup not proved"));
    }
    sampled?;
    measured?;
    let mut workload = result?;
    failures.append(&mut workload);
    Ok(failures)
}
