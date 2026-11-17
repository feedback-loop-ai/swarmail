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

/// `--bin TARGET=PATH` or `--runtime TARGET=DIR` values as a map, each target
/// at most once.
pub fn pairs(values: &[String], flag: &str) -> Result<BTreeMap<TargetKind, PathBuf>, String> {
    let mut out = BTreeMap::new();
    for v in values {
        let (name, path) = v
            .split_once('=')
            .ok_or_else(|| format!("--{flag} takes TARGET=PATH"))?;
        let t = TargetKind::parse(name).ok_or_else(|| format!("unknown target {name:?}"))?;
        if out.insert(t, PathBuf::from(path)).is_some() {
            return Err(format!("--{flag} {t} given twice"));
        }
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
                // Protocol 4 always launches Swarmail capped: no cap is an
                // error, never `--max-per-inbox 0` (which is uncapped).
                let cap = match (p.protocol_version, cap) {
                    (crate::protocol::PROTOCOL_VERSION_4, None | Some(0)) => {
                        return Err(
                            "swarmail: a protocol 4 launch needs the retention cap its entry states, and it states none (0 is uncapped)"
                                .into(),
                        );
                    }
                    (_, cap) => cap.unwrap_or(0),
                };
                LaunchSpec::new(t, binary, None, swarmail_template(cap))
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

/// Whether `end` (the chunk's one instant, protocol 4 only) has come.
pub fn past(end: Option<Instant>) -> bool {
    end.is_some_and(|e| Instant::now() >= e)
}

/// Race `work` against the chunk's instant: `None` is a cut. Dropping the
/// future is the cancellation. Without an instant (protocols 1 to 3) the
/// work is simply awaited.
pub async fn until<F: std::future::Future>(end: Option<Instant>, work: F) -> Option<F::Output> {
    match end {
        None => Some(work.await),
        Some(end) => tokio::time::timeout_at(tokio::time::Instant::from_std(end), work)
            .await
            .ok(),
    }
}

/// How a send session ends (0132 §10, V16): with `QUIT` before the chunk's
/// instant. At or after it no `QUIT` is begun and the session is dropped, and
/// a `QUIT` in flight is cut at it, so a server that never answers one costs
/// nothing past the instant. Protocols 1 to 3 pass no instant and end a
/// session as they always did.
async fn end_session(s: Smtp, chunk_end: Option<Instant>) {
    if past(chunk_end) {
        return;
    }
    let _ = until(chunk_end, s.quit()).await;
}

/// Why a profile was not measured, written in one vocabulary whatever the
/// cause (0132 §11.3): the chunk's deadline came, or the target was killed
/// for memory or exited on its own before it could be measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    ChunkDeadline,
    OomKilled,
    TargetExited,
}

impl Cause {
    pub fn outcome(self) -> WindowOutcome {
        match self {
            Cause::ChunkDeadline => WindowOutcome::ChunkDeadline,
            Cause::OomKilled => WindowOutcome::OomKilled,
            Cause::TargetExited => WindowOutcome::TargetExited,
        }
    }

    /// The reason a send row and the verification row carry.
    pub fn reason(self) -> &'static str {
        match self {
            Cause::ChunkDeadline => "chunk deadline",
            Cause::OomKilled => "oom killed",
            Cause::TargetExited => "target exited",
        }
    }
}

/// What a launch that did not become ready was, from its receipts and only
/// when its cleanup is proved (V15, V16): the leaf's `oom_kill` count says
/// the kernel ended it for memory (whatever else happened), a leader that
/// had exited before the teardown says it failed on its own, and anything
/// else (alive but never ready, no launch receipt, an unproved cleanup)
/// is not something the driver can call a result.
pub fn classify_not_ready(cleanup: &CleanupFacts, has_launch_receipt: bool) -> Option<Cause> {
    if !has_launch_receipt || !cleanup.proved() {
        return None;
    }
    if cleanup.oom_kills.is_some_and(|n| n >= 1) {
        Some(Cause::OomKilled)
    } else if cleanup.exited_early == Some(true) {
        Some(Cause::TargetExited)
    } else {
        None
    }
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
                    if let Err(e) = warmup(
                        spec,
                        &cfg,
                        &limits,
                        &clock,
                        t,
                        leaf,
                        &mut life,
                        (None, None),
                    )
                    .await
                    {
                        return (Verdict::Infrastructure(e), life, None);
                    }
                }
                leaf += 1;
                let scratch = cfg.scratch.join(format!("leaf-{leaf:04}"));
                let mut owned = match Owned::launch(spec, &scratch, &limits, &clock, &cfg.keepers)
                    .await
                {
                    Ok(o) => o,
                    Err(LaunchError::NotReady(why, facts, _)) => {
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
                            (None, None),
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

/// What a warmup left behind.
pub enum Warm {
    /// Its traffic was sent and its instance torn down and proved.
    Done,
    /// The profile cannot be measured: the instance was cut by the chunk
    /// deadline, or failed on its own before readiness. The instance is
    /// torn down and proved; `launch` names its leaf (`none` if it never
    /// had a receipt).
    NotMeasured(Cause, String),
}

/// Warmup traffic on its own instance, discarded with it: it warms the
/// controller and host, never the measured server's state. In a chunk,
/// the chunk's instant cancels what is in flight and no later message is
/// sent. With a sink (a chunk), its leaf is sampled from creation to
/// teardown like every other launch's (V13).
#[allow(clippy::too_many_arguments)]
pub async fn warmup(
    spec: &LaunchSpec,
    cfg: &Config,
    limits: &Limits,
    clock: &Clock,
    t: &ThroughputProfile,
    leaf: u32,
    life: &mut Lifecycle,
    (chunk_end, sink): (Option<Instant>, Option<&Sink>),
) -> Result<Warm, String> {
    let scratch = cfg.scratch.join(format!("leaf-{leaf:04}"));
    let sampler: Arc<Mutex<Option<Sampler>>> = Arc::default();
    let mut spec = spec.clone();
    if let Some(sink) = sink {
        let interval =
            Duration::from_millis(cfg.protocol.resources.sample_interval_ms.unwrap_or(100));
        spec.extra.on_leaf = Some(sampler_hook(
            (sampler.clone(), Arc::default()),
            sink_emit(sink),
            (cfg.manifest.run_id.clone(), spec.kind, *clock, interval),
        ));
    }
    let spec = &spec;
    let launched = Owned::launch(spec, &scratch, limits, clock, &cfg.keepers).await;
    if launched.is_err() {
        // The launch failed after its leaf was made: the sampler stops with
        // it, and the launch's own error is the one reported.
        let _ = stop_sampling(&sampler).await;
    }
    let mut owned = match launched {
        Ok(o) => o,
        Err(LaunchError::NotReady(why, facts, launch)) => {
            let name = launch.as_ref().and_then(|l| l.leaf.clone());
            let cause = classify_not_ready(&facts, launch.is_some());
            life.cleanups.push(facts);
            life.launches.extend(launch.map(|l| *l));
            return match cause {
                Some(cause) => Ok(Warm::NotMeasured(
                    cause,
                    name.unwrap_or_else(|| "none".into()),
                )),
                None => Err(format!("{}: warmup not ready ({why})", spec.kind)),
            };
        }
        Err(LaunchError::Leaf(why, facts)) => {
            life.cleanups.extend(facts);
            return Err(format!("{}: warmup {why}", spec.kind));
        }
        Err(e) => return Err(format!("{}: warmup launch refused ({e:?})", spec.kind)),
    };
    life.launches.push(owned.facts.clone());
    let step = Duration::from_millis(cfg.protocol.resources.request_deadline_ms);
    let cut = warmup_sends(
        owned.smtp,
        &cfg.protocol,
        t.warmup_messages,
        step,
        clock,
        chunk_end,
    )
    .await;
    let name = owned.leaf.as_ref().map(|l| l.name().to_owned());
    let sampled = stop_sampling(&sampler).await;
    let facts = owned.shutdown().await;
    let proved = facts.proved();
    // A cut warmup keeps V13's order from its cleanup receipt: an OOM in the
    // leaf, then a leader that had exited, and only then the chunk deadline.
    let cause = classify_not_ready(&facts, true).unwrap_or(Cause::ChunkDeadline);
    life.cleanups.push(facts);
    if !proved {
        return Err(format!("{}: warmup cleanup not proved", spec.kind));
    }
    sampled.map_err(|e| format!("{}: warmup {e}", spec.kind))?;
    Ok(if cut {
        Warm::NotMeasured(cause, name.unwrap_or_else(|| "none".into()))
    } else {
        Warm::Done
    })
}

/// The warmup's sends, one fresh session each, nothing recorded; `true` when
/// the chunk's instant cut them. Each session ends through `end_session`, so
/// the warmup keeps V16's rule as the measure path does: at or after the
/// instant no `QUIT` is begun and a `QUIT` in flight is cut at it.
pub async fn warmup_sends(
    smtp: std::net::SocketAddr,
    p: &Protocol,
    messages: u32,
    step: Duration,
    clock: &Clock,
    chunk_end: Option<Instant>,
) -> bool {
    for i in 0..messages {
        if past(chunk_end) {
            return true;
        }
        let msg = authored(p.corpus.seed, "warmup00000000", i, &p.corpus.kinds);
        let sent = until(chunk_end, async {
            if let Ok(mut s) = Smtp::open(smtp, step).await {
                let _ = s.send(&msg, clock).await;
                end_session(s, chunk_end).await;
            }
        })
        .await;
        if sent.is_none() {
            return true;
        }
    }
    false
}

/// Closed-loop workers, one outstanding delivery each. Every index gets
/// exactly one row; an index the send deadline cut off is `unstarted`, not
/// dropped. Nothing is retried.
///
/// In a chunk (`chunk_end` is `Some`) the chunk's instant is one more
/// deadline, and it reaches what is in flight: a connect or a send still
/// running at it is cancelled, never awaited (a connect cut that way keeps
/// the disposition of a failed connect, a send is `ambiguous`, each with a
/// reason that names the chunk deadline); no listing and no fidelity read is
/// begun at or after it. Protocols 1 to 3 pass none and are as they were.
#[allow(clippy::too_many_arguments)]
pub async fn throughput(
    owned: &Owned,
    p: &Protocol,
    t: &ThroughputProfile,
    tag: &str,
    rep: u32,
    run: &str,
    sink: &Sink,
    clock: &Clock,
    (probe, chunk_end): (Option<&Probe>, Option<Instant>),
) -> Result<Vec<String>, String> {
    if past(chunk_end) {
        // Nothing begins at or after the instant: not a send, a listing or
        // a read, and the window is not opened.
        if let Some(probe) = probe {
            probe.cut();
        }
        write_unstarted(run, t, (owned.kind, rep), Cause::ChunkDeadline, sink, clock)?;
        return Ok(vec![format!(
            "{} {} rep {rep}: chunk deadline",
            owned.kind, t.name
        )]);
    }
    let next = Arc::new(AtomicU32::new(0));
    let accepted = Arc::new(Mutex::new(BTreeSet::new()));
    let send_deadline = Instant::now() + Duration::from_millis(t.send_deadline_ms);
    // The earlier of the profile's own deadline and the chunk's; the reason
    // a row carries says which one ended the sending.
    let (stop_at, stop_reason) = match chunk_end {
        Some(end) if end < send_deadline => (end, "chunk deadline"),
        _ => (send_deadline, "send deadline"),
    };
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
                if Instant::now() >= stop_at {
                    r.reason = Some(stop_reason.into());
                    emit(&sink, &r)?;
                    continue;
                }
                if session.is_none() {
                    match until(chunk_end, Smtp::open(addr, step)).await {
                        Some(Ok(s)) => session = Some(s),
                        Some(Err((code, e))) => {
                            connect_failure(&mut r, code, e);
                            emit(&sink, &r)?;
                            continue;
                        }
                        None => {
                            r.outcome = "failed_before_terminator".into();
                            r.reason = Some("connect: chunk deadline".into());
                            emit(&sink, &r)?;
                            continue;
                        }
                    }
                }
                let s = session.as_mut().expect("opened above");
                let began = clock.now_ns();
                let Some(rec) = until(chunk_end, s.send(&msg, &clock)).await else {
                    // Cancelled in flight: what the server did with it is not
                    // known, and the session is dropped, never reused.
                    session = None;
                    r.start_ns = Some(began);
                    r.outcome = "ambiguous".into();
                    r.reason = Some("chunk deadline".into());
                    emit(&sink, &r)?;
                    continue;
                };
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
                    end_session(s, chunk_end).await;
                }
                emit(&sink, &r)?;
            }
            if let Some(s) = session.take() {
                end_session(s, chunk_end).await;
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
    let (verify_end, verify_reason) = match chunk_end {
        Some(end) if end < verify_deadline => (end, "chunk deadline"),
        _ => (verify_deadline, "verify deadline"),
    };
    let max_pages = t.messages / 250 + 4;
    let mut cut = false;
    let (listing, reason) = loop {
        if past(chunk_end) {
            // Not looked for is not lost: nothing is listed once the
            // instant has come.
            cut = true;
            break (Vec::new(), Some("chunk deadline"));
        }
        let Some(attempt) = until(chunk_end, owned.list_all(max_pages)).await else {
            cut = true;
            break (Vec::new(), Some("chunk deadline"));
        };
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
        if done || gone || Instant::now() >= verify_end {
            break match attempt {
                Ok(l) => (
                    l,
                    (!done).then_some(if gone { "target exited" } else { verify_reason }),
                ),
                Err(e) => (Vec::new(), Some(e.reason())),
            };
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    cut |= reason == Some("chunk deadline");
    let verified_ns = clock.now_ns();
    if let Some(probe) = probe {
        probe.mark(Mark::End);
    }
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (tok, id) in listing {
        groups.entry(tok).or_default().push(id);
    }
    let mut mismatches = 0;
    let mut unread = 0;
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
            // Untimed fidelity: outside both windows. At the chunk's instant
            // the read is cancelled and the row carries no verdict: unread
            // is neither intact nor a mismatch.
            let index = tok.rsplit('-').next().and_then(|n| n.parse().ok());
            let read = if past(chunk_end) {
                None
            } else {
                until(chunk_end, owned.fetch(&ids[0])).await
            };
            match read {
                Some(fetched) => {
                    let ok = match (index, fetched) {
                        (Some(i), Ok(f)) => {
                            fidelity(&authored(p.corpus.seed, tag, i, &p.corpus.kinds), &f)
                        }
                        _ => false,
                    };
                    mismatches += u32::from(!ok);
                    r.fidelity_ok = Some(ok);
                }
                None => unread += 1,
            }
        }
        emit(sink, &r)?;
    }
    let reason = match (reason, unread > 0) {
        (None, true) => {
            cut = true;
            Some("chunk deadline")
        }
        (r, _) => r,
    };
    if cut && let Some(probe) = probe {
        probe.cut();
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

/// How a window ended, from the leaf's `memory.events` at its bounds, whether
/// the target exited meanwhile and whether the chunk's deadline ended its
/// work. Only a rise of `oom_kill` is an OOM; `max` alone is pressure at the
/// limit. Where more than one applies the first of OOM, exit, deadline wins:
/// an OOM is a kernel fact and is never relabelled as a deadline.
pub fn window_outcome(
    start: &MemEvents,
    end: &MemEvents,
    exited: bool,
    cut: bool,
) -> WindowOutcome {
    if end.oom_kill > start.oom_kill {
        WindowOutcome::OomKilled
    } else if exited {
        WindowOutcome::TargetExited
    } else if cut {
        WindowOutcome::ChunkDeadline
    } else {
        WindowOutcome::Measured
    }
}

/// The outcome of a window that was never opened, in V13's order: an OOM kill
/// the instance's leaf shows, then the leader having exited, then the cause
/// in hand. An OOM is a kernel fact and is never relabelled, whether or not
/// the window was opened (V13).
pub fn never_opened_outcome(leaf: &Leaf, exited: bool, cause: WindowOutcome) -> WindowOutcome {
    if leaf.mem_events().is_ok_and(|e| e.oom_kill >= 1) {
        WindowOutcome::OomKilled
    } else if exited {
        WindowOutcome::TargetExited
    } else {
        cause
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
pub struct Marks {
    start: Option<Snap>,
    end: Option<Snap>,
    /// Read at the end bound: `None` when no task could be read.
    affinity_held: Option<bool>,
    error: Option<String>,
    /// The chunk's deadline ended the window's work (a send, a listing or a
    /// fidelity read), or kept it from beginning.
    cut: bool,
}

/// Whether every task in `leaf` has exactly `target_cpus`, read from
/// `proc_root`: one function for the end of every window kind. `None` is no
/// task to read (or no list of them), which is no observation.
pub fn read_affinity(
    leaf: &Leaf,
    proc_root: &std::path::Path,
    target_cpus: &[u32],
) -> Option<bool> {
    let pids = leaf.read_procs().ok()?;
    crate::resources::affinity_read(proc_root, &pids, target_cpus)
}

/// Takes the leaf's counters at the bounds of the profile's send-and-verify
/// window, from inside `throughput`, so the end bound is the instant
/// verification completed and the untimed fidelity reads fall outside it.
pub struct Probe {
    leaf: Leaf,
    run: String,
    target: TargetKind,
    clock: Clock,
    sink: Sink,
    target_cpus: Vec<u32>,
    marks: Mutex<Marks>,
}

impl Probe {
    pub fn new(
        (leaf, target_cpus): (Leaf, Vec<u32>),
        run: String,
        target: TargetKind,
        clock: Clock,
        sink: Sink,
    ) -> Probe {
        Probe {
            leaf,
            run,
            target,
            clock,
            sink,
            target_cpus,
            marks: Mutex::new(Marks::default()),
        }
    }

    fn mark(&self, which: Mark) {
        let taken = bound(&self.leaf, &self.run, self.target, &self.clock, &self.sink);
        let held = match which {
            Mark::End => {
                read_affinity(&self.leaf, std::path::Path::new("/proc"), &self.target_cpus)
            }
            Mark::Start => None,
        };
        let mut m = self.marks.lock().expect("marks lock");
        match taken {
            Ok(snap) => match which {
                Mark::Start => m.start = Some(snap),
                Mark::End => {
                    m.end = Some(snap);
                    m.affinity_held = held;
                }
            },
            Err(e) => {
                m.error.get_or_insert(e);
            }
        }
    }

    /// The chunk's deadline ended this window's work.
    fn cut(&self) {
        self.marks.lock().expect("marks lock").cut = true;
    }

    pub fn into_marks(self) -> Marks {
        self.marks.into_inner().expect("marks lock")
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

/// An opened window: its counters at both bounds and what the end read of
/// the affinity (`None` when no task could be read).
#[allow(clippy::too_many_arguments)]
pub fn window_row(
    run: &str,
    launch: &str,
    (target, profile, rep): (TargetKind, &str, u32),
    window: WindowKind,
    (a, b): (&Snap, &Snap),
    affinity_held: Option<bool>,
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
        cpu_start: Some(a.cpu),
        cpu_end: Some(b.cpu),
        events_start: Some(a.events),
        events_end: Some(b.events),
        affinity_held,
        scratch_bytes,
        outcome,
    }
}

/// A window that was never opened: no counters, no affinity, no scratch
/// size, both bounds the instant it was given up. `launch` is the leaf of
/// the instance that failed, or `none` when none was launched.
pub fn unopened_window(
    run: &str,
    launch: &str,
    (target, profile, rep): (TargetKind, &str, u32),
    window: WindowKind,
    at_ns: u64,
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
        start_ns: at_ns,
        end_ns: at_ns,
        cpu_start: None,
        cpu_end: None,
        events_start: None,
        events_end: None,
        affinity_held: None,
        scratch_bytes: None,
        outcome,
    }
}

/// What a window needs to close: whose it is, and where its end reads the
/// affinity from.
pub struct Closing<'a> {
    pub run: &'a str,
    pub launch: &'a str,
    pub key: (TargetKind, &'a str, u32),
    pub leaf: &'a Leaf,
    pub proc_root: &'a std::path::Path,
    pub target_cpus: &'a [u32],
}

impl Closing<'_> {
    /// An opened window of any kind, closed now: the affinity is read here,
    /// at its end, the same way for `ready`, `idle_ready` and `measure`.
    pub fn window(
        &self,
        kind: WindowKind,
        bounds: (&Snap, &Snap),
        scratch_bytes: Option<u64>,
        outcome: WindowOutcome,
    ) -> WindowRow {
        self.with_affinity(
            kind,
            bounds,
            read_affinity(self.leaf, self.proc_root, self.target_cpus),
            scratch_bytes,
            outcome,
        )
    }

    /// The same, for a window whose end was read earlier.
    pub fn with_affinity(
        &self,
        kind: WindowKind,
        bounds: (&Snap, &Snap),
        affinity_held: Option<bool>,
        scratch_bytes: Option<u64>,
        outcome: WindowOutcome,
    ) -> WindowRow {
        window_row(
            self.run,
            self.launch,
            self.key,
            kind,
            bounds,
            affinity_held,
            scratch_bytes,
            outcome,
        )
    }
}

fn emit_window(sink: &Sink, w: WindowRow) -> Result<(), String> {
    sink.lock()
        .expect("sink lock")
        .append_resource(&ResourceRow::Window(w))
        .map_err(|e| format!("sample write failed: {e}"))
}

/// Every send of a profile as an `unstarted` row and the verification row as
/// `incomplete`, both with the cause as their reason. Nothing was sent and
/// nothing was listed.
pub fn write_unstarted(
    run: &str,
    t: &ThroughputProfile,
    (target, rep): (TargetKind, u32),
    cause: Cause,
    sink: &Sink,
    clock: &Clock,
) -> Result<(), String> {
    for i in 0..t.messages {
        let mut r = row(run, target, &t.name, rep, Phase::Send, i, "unstarted");
        r.reason = Some(cause.reason().into());
        emit(sink, &r)?;
    }
    let now = clock.now_ns();
    let mut v = row(run, target, &t.name, rep, Phase::Verified, 0, "incomplete");
    v.reason = Some(cause.reason().into());
    v.start_ns = Some(now);
    v.ack_ns = Some(now);
    v.observed_ns = Some(now);
    v.requests = Some(0);
    emit(sink, &v)
}

/// A profile that was not measured, by one path whatever the cause: its
/// sends are `unstarted`, its verification `incomplete`, and its three
/// windows are never opened (no counters), each with the cause as outcome.
/// `launch` is the leaf of the instance that failed, or `none`.
pub fn skip_profile(
    run: &str,
    t: &ThroughputProfile,
    (target, rep): (TargetKind, u32),
    (cause, launch): (Cause, &str),
    sink: &Sink,
    clock: &Clock,
) -> Result<(), String> {
    write_unstarted(run, t, (target, rep), cause, sink, clock)?;
    let now = clock.now_ns();
    for kind in [
        WindowKind::Ready,
        WindowKind::IdleReady,
        WindowKind::Measure,
    ] {
        emit_window(
            sink,
            unopened_window(
                run,
                launch,
                (target, &t.name, rep),
                kind,
                now,
                cause.outcome(),
            ),
        )?;
    }
    Ok(())
}

/// Run one cost chunk: that target's cost profiles of that repetition, in
/// order, each as protocol 3 does it (a warmup instance, then a fresh measured
/// one), both in leaves. A target that exits in a window, or before it is
/// ready, is a measured outcome and the chunk goes on. The directory is
/// sealed on every outcome but an infrastructure failure.
///
/// The chunk deadline is one instant, `chunk_end`, taken from the run
/// clock's origin (which the driver took before it made the output
/// directory). At or after it nothing new starts: no instance is launched,
/// no window opened, no send, listing or read begun; and what is in flight on
/// the wire is cancelled, not awaited. Only a launch already in flight and a
/// teardown are never cut, and `protocol.rs` bounds the sum of those.
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
    let chunk_end = clock.0 + Duration::from_millis(p.resources.chunk_deadline_ms.unwrap_or(0));
    let mut failures = Vec::new();
    let mut leaf_no = 0u32;
    // A profile that was not measured: one path for every cause.
    let not_measured =
        |t: &ThroughputProfile, (cause, launch): (Cause, &str), failures: &mut Vec<String>| {
            skip_profile(
                &cfg.manifest.run_id,
                t,
                (target, rep),
                (cause, launch),
                &sink,
                &clock,
            )?;
            failures.push(format!("{target} {} rep {rep}: {}", t.name, cause.reason()));
            Ok::<(), String>(())
        };
    for (i, t) in p
        .throughput
        .iter()
        .enumerate()
        .filter(|(_, t)| t.repetitions > rep)
    {
        if past(Some(chunk_end)) {
            if let Err(e) = not_measured(t, (Cause::ChunkDeadline, "none"), &mut failures) {
                return infra(e, life);
            }
            continue;
        }
        if t.warmup_messages > 0 {
            leaf_no += 1;
            match warmup(
                spec,
                &cfg,
                &limits,
                &clock,
                t,
                leaf_no,
                &mut life,
                (Some(chunk_end), Some(&sink)),
            )
            .await
            {
                Ok(Warm::Done) => {}
                Ok(Warm::NotMeasured(cause, launch)) => {
                    if let Err(e) = not_measured(t, (cause, &launch), &mut failures) {
                        return infra(e, life);
                    }
                    continue;
                }
                Err(e) => return infra(e, life),
            }
        }
        if past(Some(chunk_end)) {
            if let Err(e) = not_measured(t, (Cause::ChunkDeadline, "none"), &mut failures) {
                return infra(e, life);
            }
            continue;
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
            chunk_end,
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
/// saw; an error is an infrastructure failure. An instance that did not
/// become ready because the target itself failed (killed for memory, or
/// exited) makes the profile not measured, which is a workload failure.
#[allow(clippy::too_many_arguments)]
pub async fn measured_instance(
    (spec, target_cpus): (&LaunchSpec, &[u32]),
    cfg: &Config,
    limits: &Limits,
    clock: &Clock,
    (i, t, rep): (usize, &ThroughputProfile, u32),
    leaf_no: u32,
    sink: &Sink,
    life: &mut Lifecycle,
    chunk_end: Instant,
) -> Result<Vec<String>, String> {
    let end = Some(chunk_end);
    let p = &cfg.protocol;
    let run = cfg.manifest.run_id.clone();
    let target = spec.kind;
    let scratch = cfg.scratch.join(format!("leaf-{leaf_no:04}"));
    let interval = Duration::from_millis(p.resources.sample_interval_ms.unwrap_or(100));
    // The sampler starts when the leaf is created, from inside the launch.
    let sampler: Arc<Mutex<Option<Sampler>>> = Arc::default();
    let created: Arc<Mutex<Option<u64>>> = Arc::default();
    let hook = sampler_hook(
        (sampler.clone(), created.clone()),
        sink_emit(sink),
        (run.clone(), target, *clock, interval),
    );
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
            return match e {
                LaunchError::NotReady(why, facts, launch) => {
                    let name = launch.as_ref().and_then(|l| l.leaf.clone());
                    let cause = classify_not_ready(&facts, launch.is_some());
                    life.cleanups.push(facts);
                    life.launches.extend(launch.map(|l| *l));
                    match cause {
                        Some(cause) => {
                            skip_profile(
                                &run,
                                t,
                                (target, rep),
                                (cause, name.as_deref().unwrap_or("none")),
                                sink,
                                clock,
                            )?;
                            Ok(vec![format!(
                                "{target} {} rep {rep}: {}",
                                t.name,
                                cause.reason()
                            )])
                        }
                        None => Err(format!("{target}: not ready ({why})")),
                    }
                }
                LaunchError::Leaf(why, facts) => {
                    life.cleanups.extend(facts);
                    Err(format!("{target}: {why}"))
                }
                other => Err(format!("{target}: launch refused ({other:?})")),
            };
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
    let closing = Closing {
        run: &run,
        launch: &launch,
        key,
        leaf: &leaf,
        proc_root: std::path::Path::new("/proc"),
        target_cpus,
    };
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
    let outcome = window_outcome(&zero.events, &ready.events, owned.has_exited(), false);
    note(WindowKind::Ready, outcome);
    emit_window(
        sink,
        closing.window(WindowKind::Ready, (&zero, &ready), None, outcome),
    )?;
    // idle_ready: five seconds without traffic.
    let idle = idle_ready(&closing, (&owned, &scratch), sink, clock, end).await?;
    note(WindowKind::IdleReady, idle.outcome);
    emit_window(sink, idle)?;
    // measure: the profile's send-and-verify window.
    let probe = Probe::new(
        (leaf.clone(), target_cpus.to_vec()),
        run.clone(),
        target,
        *clock,
        sink.clone(),
    );
    let result = throughput(
        &owned,
        p,
        t,
        &tag(&run, target, i, rep),
        rep,
        &run,
        sink,
        clock,
        (Some(&probe), end),
    )
    .await;
    let marks = probe.into_marks();
    let measured = match (marks.start, marks.end, marks.error) {
        (Some(a), Some(b), None) => {
            let outcome = window_outcome(&a.events, &b.events, owned.has_exited(), marks.cut);
            note(WindowKind::Measure, outcome);
            emit_window(
                sink,
                closing.with_affinity(
                    WindowKind::Measure,
                    (&a, &b),
                    marks.affinity_held,
                    None,
                    outcome,
                ),
            )
        }
        (_, _, Some(e)) => Err(format!("{target}: window bounds: {e}")),
        (None, None, None) if marks.cut => {
            // The instant came before the window could open.
            let outcome =
                never_opened_outcome(&leaf, owned.has_exited(), WindowOutcome::ChunkDeadline);
            note(WindowKind::Measure, outcome);
            emit_window(
                sink,
                unopened_window(
                    &run,
                    &launch,
                    key,
                    WindowKind::Measure,
                    clock.now_ns(),
                    outcome,
                ),
            )
        }
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

/// The `idle_ready` window: five seconds without traffic after readiness. At
/// or after the chunk's instant it is not opened; the instant ends it early
/// when it comes first, and the window then keeps its counters with outcome
/// `chunk_deadline`.
pub async fn idle_ready(
    closing: &Closing<'_>,
    (owned, scratch): (&Owned, &std::path::Path),
    sink: &Sink,
    clock: &Clock,
    end: Option<Instant>,
) -> Result<WindowRow, String> {
    let (run, target) = (closing.run, closing.key.0);
    if past(end) {
        // Never opened: V13's order, as for the measure window.
        let outcome = never_opened_outcome(
            closing.leaf,
            owned.has_exited(),
            WindowOutcome::ChunkDeadline,
        );
        return Ok(unopened_window(
            run,
            closing.launch,
            closing.key,
            WindowKind::IdleReady,
            clock.now_ns(),
            outcome,
        ));
    }
    let a = bound(closing.leaf, run, target, clock, sink)?;
    let cut = until(end, tokio::time::sleep(IDLE_READY)).await.is_none();
    let b = bound(closing.leaf, run, target, clock, sink)?;
    let scratch_bytes = crate::resources::allocated_bytes(scratch).ok();
    let outcome = window_outcome(&a.events, &b.events, owned.has_exited(), cut);
    Ok(closing.window(WindowKind::IdleReady, (&a, &b), scratch_bytes, outcome))
}

/// Where a sampler is parked, and when its leaf was made.
pub type SamplerSlot = (Arc<Mutex<Option<Sampler>>>, Arc<Mutex<Option<u64>>>);

/// A leaf hook that starts a sampler on each new leaf, parks it in `slot` and
/// notes when the leaf was made. Used by the measured instance and by `hold`.
/// Where a chunk's sampler writes: its sample rows, in the chunk's resource
/// file.
fn sink_emit(sink: &Sink) -> crate::resources::Emit {
    let sink = sink.clone();
    Arc::new(move |row| {
        sink.lock()
            .expect("sink lock")
            .append_resource(&ResourceRow::Sample(row))
            .map_err(|e| format!("sample write failed: {e}"))
    })
}

/// Take a launch's sampler, if its leaf started one, and stop it.
async fn stop_sampling(slot: &Mutex<Option<Sampler>>) -> Result<(), String> {
    let s = slot.lock().expect("sampler lock").take();
    match s {
        Some(s) => s.stop().await.map(|_| ()),
        None => Ok(()),
    }
}

pub fn sampler_hook(
    (slot, created): SamplerSlot,
    emit: crate::resources::Emit,
    (run, target, clock, interval): (String, TargetKind, Clock, Duration),
) -> crate::targets::LeafHook {
    crate::targets::LeafHook(Arc::new(move |leaf: &Leaf| {
        *created.lock().expect("created lock") = Some(clock.now_ns());
        let s = Sampler::spawn(
            leaf.clone(),
            run.clone(),
            target,
            clock,
            interval,
            emit.clone(),
        );
        *slot.lock().expect("sampler lock") = Some(s);
    }))
}

// ---- hold (0132 §11, story A: V19) -------------------------------------------

/// What `hold` was asked: one target, its bytes and where to put its samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldArgs {
    pub protocol: PathBuf,
    pub sources: PathBuf,
    pub documents: Option<PathBuf>,
    pub target: TargetKind,
    pub bin: PathBuf,
    pub runtime: Option<PathBuf>,
    pub scratch: PathBuf,
    pub samples: PathBuf,
}

/// `hold`'s flags: a missing one, an unknown one or a second target refuses
/// by name.
pub fn hold_args(args: &[String]) -> Result<HoldArgs, String> {
    let f = flags(args)?;
    let known = [
        "protocol",
        "sources",
        "documents",
        "bin",
        "runtime",
        "scratch",
        "samples",
    ];
    if let Some(k) = f.keys().find(|k| !known.contains(&k.as_str())) {
        return Err(format!("unknown flag --{k}"));
    }
    let one = |name: &str| -> Result<PathBuf, String> {
        f.get(name)
            .and_then(|v| v.first())
            .map(PathBuf::from)
            .ok_or_else(|| format!("--{name} is required"))
    };
    let bins = pairs(f.get("bin").map_or(&[][..], Vec::as_slice), "bin")?;
    let runtimes = pairs(f.get("runtime").map_or(&[][..], Vec::as_slice), "runtime")?;
    let mut it = bins.into_iter();
    let (Some((target, bin)), None) = (it.next(), it.next()) else {
        return Err("hold takes exactly one --bin TARGET=PATH".into());
    };
    if runtimes.keys().any(|t| *t != target) {
        return Err(format!("--runtime must name {target}, the one target held"));
    }
    Ok(HoldArgs {
        protocol: one("protocol")?,
        sources: one("sources")?,
        documents: f
            .get("documents")
            .and_then(|v| v.first())
            .map(PathBuf::from),
        target,
        bin,
        runtime: runtimes.get(&target).cloned(),
        scratch: one("scratch")?,
        samples: one("samples")?,
    })
}

/// `hold` runs the enforced budget: protocols 2 and 3 have none.
pub fn hold_protocol(p: &Protocol) -> Result<(), String> {
    if p.protocol_version != crate::protocol::PROTOCOL_VERSION_4 {
        return Err(format!(
            "hold needs protocol {}: protocol {} has no enforced budget to hold a target under",
            crate::protocol::PROTOCOL_VERSION_4,
            p.protocol_version
        ));
    }
    Ok(())
}

/// What a held target printed when it was ready.
#[derive(Debug, serde::Serialize)]
pub struct HoldEndpoints {
    pub target: TargetKind,
    pub leaf: Option<String>,
    pub smtp: String,
    pub http: Option<String>,
    pub pop3: Option<String>,
}

/// The endpoints of a launched target, each the `host:port` it listens on:
/// the POP3 listener's included, so a reader connects to what is printed.
pub fn hold_endpoints(owned: &Owned) -> HoldEndpoints {
    HoldEndpoints {
        target: owned.kind,
        leaf: owned.leaf.as_ref().map(|l| l.name().to_owned()),
        smtp: owned.smtp.to_string(),
        http: (owned.http_addr.port() != 0).then(|| owned.http_addr.to_string()),
        pop3: owned.pop3_listen.map(|p| p.to_string()),
    }
}

/// What it printed when it was done.
#[derive(Debug, serde::Serialize)]
pub struct HoldReceipt {
    pub launch: LaunchFacts,
    pub cleanup: CleanupFacts,
    /// Bytes read from the input before it ended. Silence is what `hold`
    /// listens for, so any byte is refused, after teardown.
    pub stdin_bytes: u64,
}

/// A held run's verdict: it exits 0 only when cleanup is proved and the input
/// said nothing.
pub fn hold_verdict(r: &HoldReceipt) -> Result<(), String> {
    if !r.cleanup.proved() {
        return Err("cleanup is not proved".into());
    }
    if r.stdin_bytes > 0 {
        return Err(format!(
            "stdin carried {} byte(s) before EOF: hold reads nothing from it, so a later story can add a line protocol without changing what silence means",
            r.stdin_bytes
        ));
    }
    Ok(())
}

/// Launch one target (under its leaf when the spec has a scope), print its
/// endpoints as one JSON line, sample it, and wait for the input to end or
/// `interrupt` to fire. Then run the whole teardown and print the launch and
/// cleanup receipts as one JSON line.
#[allow(clippy::too_many_arguments)]
pub async fn hold<R, W>(
    spec: &LaunchSpec,
    (scratch, samples): (&std::path::Path, &std::path::Path),
    limits: &Limits,
    clock: &Clock,
    interval: Duration,
    mut input: R,
    out: &mut W,
    interrupt: impl std::future::Future<Output = ()>,
) -> Result<HoldReceipt, String>
where
    R: tokio::io::AsyncRead + Unpin,
    W: std::io::Write,
{
    use std::os::unix::fs::OpenOptionsExt;
    use tokio::io::AsyncReadExt;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(samples)
        .map_err(|e| format!("--samples {}: {e}", samples.display()))?;
    let file = Arc::new(Mutex::new(file));
    let emit: crate::resources::Emit = Arc::new(move |row| {
        use std::io::Write;
        let mut line = serde_json::to_vec(&ResourceRow::Sample(row)).expect("row serializes");
        line.push(b'\n');
        file.lock()
            .expect("samples lock")
            .write_all(&line)
            .map_err(|e| format!("sample write failed: {e}"))
    });
    let sampler: Arc<Mutex<Option<Sampler>>> = Arc::default();
    let created: Arc<Mutex<Option<u64>>> = Arc::default();
    let mut spec = spec.clone();
    spec.extra.on_leaf = Some(sampler_hook(
        (sampler.clone(), created),
        emit,
        ("hold".to_owned(), spec.kind, *clock, interval),
    ));
    let keepers = Keepers::default();
    let stop_sampler = || async {
        let s = sampler.lock().expect("sampler lock").take();
        match s {
            Some(s) => s.stop().await.map(|_| ()),
            None => Ok(()),
        }
    };
    let mut owned = match Owned::launch(&spec, scratch, limits, clock, &keepers).await {
        Ok(o) => o,
        Err(e) => {
            let _ = stop_sampler().await;
            keepers.settle(limits.cleanup * 2).await;
            return Err(format!("{} did not launch: {e:?}", spec.kind));
        }
    };
    let line = serde_json::to_string(&hold_endpoints(&owned)).expect("endpoints serialize");
    let announced = writeln!(out, "{line}").and_then(|()| out.flush());
    let mut bytes = 0u64;
    if announced.is_ok() {
        let mut buf = [0u8; 4096];
        tokio::pin!(interrupt);
        loop {
            tokio::select! {
                n = input.read(&mut buf) => match n {
                    Ok(0) | Err(_) => break,
                    Ok(k) => bytes += k as u64,
                },
                _ = &mut interrupt => break,
            }
        }
    }
    let sampled = stop_sampler().await;
    let facts = owned.shutdown().await;
    let launch = owned.facts.clone();
    keepers.settle(limits.cleanup * 2).await;
    let receipt = HoldReceipt {
        launch,
        cleanup: facts,
        stdin_bytes: bytes,
    };
    let line = serde_json::to_string(&receipt).expect("receipt serializes");
    writeln!(out, "{line}")
        .and_then(|()| out.flush())
        .map_err(|e| format!("could not print the receipt: {e}"))?;
    announced.map_err(|e| format!("could not print the endpoints: {e}"))?;
    sampled?;
    Ok(receipt)
}
