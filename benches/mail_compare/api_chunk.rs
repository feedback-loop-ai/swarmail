//! Protocol 4's API chunk on the native surface (accepted decision 0132 §6,
//! §11, story B): one fresh instance of one target, under its cgroup leaf,
//! filled to one level, then every native cell in the fixed order, each
//! measured operation inside a resource window. Launch, sampling, the idle
//! windows and the sealed directory are the cost chunk's own (story A); this
//! module adds only the API section's steps between readiness and teardown.
//!
//! Exit classes follow 0132 §11.3: a target's own failure (a fill not
//! reached, an OOM, an exit, a wrong answer, a call left unresolved) is a
//! measured row and the chunk exits 1; an infrastructure, cleanup or
//! accounting failure exits 2 and leaves the directory unsealed.

use crate::api::{self, ApiRecord, CellRecord, CellStatus, PlannedCell};
use crate::clients::{self, Operation, Surface};
use crate::protocol::TargetKind;
use crate::records::{PlannedWindow, ResourceRow, WindowKind, WindowOutcome, WindowRow};
use crate::resources::{Leaf, Snap};
use crate::runner::{
    Chunk, Closing, Config, Lifecycle, Sink, Verdict, bound, emit_window, idle_ready,
    never_opened_outcome, unopened_window, window_outcome,
};
use crate::wire::Clock;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

/// The windows' profile of a launch: `api.f<fill>`.
pub fn fill_profile(fill: u32) -> String {
    format!("api.f{fill}")
}

/// The window profile of one cell: `api.<cell>`.
pub fn cell_profile(cell: &PlannedCell) -> String {
    format!("api.{}", cell.name())
}

/// The cells a chunk plans: the surface's capability table, in order.
pub fn cells(target: TargetKind, surface: Surface) -> Vec<PlannedCell> {
    api::plan_cells(surface, |op| {
        clients::capability(target.name(), surface, op)
    })
    .0
}

/// Every window an API chunk produces, each exactly once: the launch's
/// `ready` and `idle_ready`, the fill level's `idle_fill`, and one window per
/// planned cell (opened, or never opened with its cause).
pub fn planned_windows(
    target: TargetKind,
    repetition: u32,
    surface: Surface,
    fill: u32,
) -> Vec<PlannedWindow> {
    let w = |profile: String, window| PlannedWindow {
        target,
        profile,
        repetition,
        window,
    };
    let mut out = vec![
        w(fill_profile(fill), WindowKind::Ready),
        w(fill_profile(fill), WindowKind::IdleReady),
        w(fill_profile(fill), WindowKind::IdleFill),
    ];
    for c in cells(target, surface) {
        out.push(w(cell_profile(&c), WindowKind::ApiCell));
    }
    out
}

/// The leaf meter: each cell's window opens with the counters read and a
/// sample written at its first bound, and closes the same way at its last.
struct LeafMeter<'a> {
    leaf: &'a Leaf,
    run: &'a str,
    launch: &'a str,
    target: TargetKind,
    repetition: u32,
    clock: Clock,
    sink: &'a Sink,
    target_cpus: &'a [u32],
    exited: &'a (dyn Fn() -> bool + Sync),
    open: Mutex<BTreeMap<String, Snap>>,
    cells: &'a [PlannedCell],
}

impl LeafMeter<'_> {
    fn open_now(&self, cell: &str) -> Result<(), String> {
        let snap = bound(self.leaf, self.run, self.target, &self.clock, self.sink)?;
        self.open
            .lock()
            .expect("meter lock")
            .insert(cell.to_owned(), snap);
        Ok(())
    }

    fn close_now(&self, cell: &str) -> Result<(), String> {
        let b = bound(self.leaf, self.run, self.target, &self.clock, self.sink)?;
        let a = self
            .open
            .lock()
            .expect("meter lock")
            .remove(cell)
            .ok_or_else(|| format!("window {cell} closed without opening"))?;
        let planned = self
            .cells
            .iter()
            .find(|c| c.name() == cell)
            .ok_or_else(|| format!("window {cell} is not planned"))?;
        let profile = cell_profile(planned);
        let closing = Closing {
            run: self.run,
            launch: self.launch,
            key: (self.target, &profile, self.repetition),
            leaf: self.leaf,
            proc_root: std::path::Path::new("/proc"),
            target_cpus: self.target_cpus,
        };
        let outcome = window_outcome(&a.events, &b.events, (self.exited)(), false);
        emit_window(
            self.sink,
            closing.window(WindowKind::ApiCell, (&a, &b), None, outcome),
        )
    }
}

impl api::Meter for LeafMeter<'_> {
    fn open<'a>(&'a self, cell: &'a str) -> api::MeterFuture<'a> {
        Box::pin(async move { self.open_now(cell) })
    }

    fn close<'a>(&'a self, cell: &'a str) -> api::MeterFuture<'a> {
        Box::pin(async move { self.close_now(cell) })
    }
}

/// One unopened window of the chunk's plan.
fn unopened(
    sink: &Sink,
    (run, launch): (&str, &str),
    (target, profile, rep): (TargetKind, &str, u32),
    kind: WindowKind,
    clock: &Clock,
    outcome: WindowOutcome,
) -> Result<(), String> {
    emit_window(
        sink,
        unopened_window(
            run,
            launch,
            (target, profile, rep),
            kind,
            clock.now_ns(),
            outcome,
        ),
    )
}

/// The figures of an API chunk's report (`report.json`'s `api` key), each a
/// pure function of `api.json` and the window and sample rows, so admission
/// recomputes them.
pub fn api_report(rec: &ApiRecord, resources: &[ResourceRow]) -> serde_json::Value {
    let window = |kind: WindowKind, profile: &str| {
        resources.iter().find_map(|r| match r {
            ResourceRow::Window(w) if w.window == kind && w.profile == profile => Some(w),
            _ => None,
        })
    };
    let cpu = |w: Option<&WindowRow>| {
        w.and_then(|w| w.counters())
            .map(|(a, b, _, _)| b.usage_usec.saturating_sub(a.usage_usec))
    };
    let cells: Vec<serde_json::Value> = rec
        .cells
        .iter()
        .map(|c| {
            let profile = format!("api.{}", c.cell);
            let w = window(WindowKind::ApiCell, &profile);
            let s = cell_stats(c, cpu(w));
            serde_json::json!({
                "cell": c.cell,
                "operation": c.operation,
                "concurrency": c.concurrency,
                "status": c.status,
                "blocked": c.blocked,
                "method": c.method,
                "scope": c.scope,
                "stats": s,
                // The global clear: one invocation, published as its own
                // raw sample with no quantiles (spec AC9, R13).
                "sample_ns": (c.concurrency.is_none())
                    .then(|| c.measured.rows.first().map(|r| r.end_ns.saturating_sub(r.start_ns)))
                    .flatten(),
                "resource_window": w.map(|w| serde_json::json!({
                    "start_ns": w.start_ns,
                    "end_ns": w.end_ns,
                    "outcome": w.outcome,
                    "cpu_usec": cpu(Some(w)),
                })),
                "at_equal_results": at_equal_results(c, &s, w),
                "postcheck": c.postcheck,
            })
        })
        .collect();
    let idle = window(WindowKind::IdleFill, &fill_profile(rec.fill.fill));
    let idle_fill = idle.filter(|w| w.counters().is_some()).map(|w| {
        let tail = w.end_ns.saturating_sub(crate::runner::IDLE_TAIL_NS);
        let samples: Vec<_> = resources
            .iter()
            .filter_map(|r| match r {
                ResourceRow::Sample(s)
                    if s.launch == w.launch && s.t_ns >= tail && s.t_ns <= w.end_ns =>
                {
                    Some(s)
                }
                _ => None,
            })
            .collect();
        let mean = |f: &dyn Fn(&crate::records::SampleRow) -> u64| {
            (!samples.is_empty())
                .then(|| samples.iter().map(|s| f(s) as f64).sum::<f64>() / samples.len() as f64)
        };
        serde_json::json!({
            "label": format!("idle_fill_{}", rec.fill.fill),
            "memory_current": mean(&|s| s.memory_current),
            "anon": mean(&|s| s.anon),
            "samples": samples.len(),
            "scratch_bytes": w.scratch_bytes,
        })
    });
    serde_json::json!({
        "surface": rec.surface,
        "target": rec.target,
        "repetition": rec.repetition,
        "fill": rec.fill,
        "store": rec.store,
        "fixtures": rec.fixtures,
        "growth": rec.growth,
        "capabilities": rec.capabilities,
        "abandoned": rec.abandoned,
        "idle_fill": idle_fill,
        "cells": cells,
    })
}

/// A cell's statistics as published: the global clear's single invocation
/// carries no quantiles — one sample is not a distribution (R13).
pub fn cell_stats(c: &CellRecord, cpu_usec: Option<u64>) -> api::CellStats {
    let mut s = api::stats(&c.measured, cpu_usec);
    if c.concurrency.is_none() {
        (s.p50_ns, s.p95_ns, s.p99_ns) = (None, None, None);
    }
    s
}

/// The control brackets bound to the holder's own windows (R14): each
/// pair's holder instants are its cell window's start and end, and the
/// idle-fill bracket's is the idle-fill window's end — the same clock, so
/// equality is checkable; the runner's instants are never compared to it.
pub fn check_bracket_windows(rec: &ApiRecord, resources: &[ResourceRow]) -> Result<(), String> {
    let window = |kind: WindowKind, profile: &str| {
        resources.iter().find_map(|r| match r {
            ResourceRow::Window(w) if w.window == kind && w.profile == profile => Some(w),
            _ => None,
        })
    };
    for b in &rec.control {
        let bound = match (b.cmd.as_str(), b.id.as_deref()) {
            ("idle_fill", None) => window(WindowKind::IdleFill, &fill_profile(rec.fill.fill))
                .is_some_and(|w| w.end_ns == b.owner_t_ns),
            ("window_start", Some(id)) => window(WindowKind::ApiCell, &format!("api.{id}"))
                .is_some_and(|w| w.start_ns == b.owner_t_ns),
            ("window_end", Some(id)) => window(WindowKind::ApiCell, &format!("api.{id}"))
                .is_some_and(|w| w.end_ns == b.owner_t_ns),
            _ => false,
        };
        if !bound {
            return Err(format!(
                "the {} bracket {:?} names no window of the holder's",
                b.cmd, b.id
            ));
        }
    }
    Ok(())
}

/// Whether a cell stands at equal results (0132 §9.6, spec AC8, R4): it
/// measured, every planned operation started and was exact, and its
/// resource window closed clean with its counters — a failed or missing
/// window never ranks, whatever the operations answered.
pub fn at_equal_results(c: &CellRecord, s: &api::CellStats, w: Option<&WindowRow>) -> bool {
    c.status == CellStatus::Measured
        && s.started == s.planned
        && s.exact == s.started
        && w.is_some_and(|w| w.outcome == WindowOutcome::Measured && w.counters().is_some())
}

/// The target's own failures a record shows (spec AC8, R4), the same on
/// both surfaces: a measured cell whose started operations include a
/// `wrong_result`, `failed` or `timeout` row cannot exit clean. Inexact
/// fidelity is documented, unranked evidence and capability rows are not
/// executions: neither is a failure here.
pub fn row_failures(rec: &ApiRecord) -> Vec<String> {
    let mut out = Vec::new();
    for c in rec
        .cells
        .iter()
        .filter(|c| c.status == CellStatus::Measured)
    {
        let bad = |class: &str| {
            c.measured
                .rows
                .iter()
                .filter(|r| r.outcome.class() == class)
                .count()
        };
        let (wrong, failed, timeout) = (bad("wrong_result"), bad("failed"), bad("timeout"));
        if wrong + failed + timeout > 0 {
            out.push(format!(
                "{} {}: {wrong} wrong_result, {failed} failed, {timeout} timeout",
                rec.target, c.cell
            ));
        }
    }
    out
}

/// The target's own failures its windows show (spec AC8, R4), the same on
/// both surfaces: any readiness, idle, idle-fill or cell window that closed
/// on an OOM kill or the target's exit. A blocked or deadline window is a
/// cause recorded elsewhere, not a second failure.
pub fn window_failures(resources: &[ResourceRow]) -> Vec<String> {
    resources
        .iter()
        .filter_map(|r| match r {
            ResourceRow::Window(w)
                if matches!(
                    w.outcome,
                    WindowOutcome::OomKilled | WindowOutcome::TargetExited
                ) =>
            {
                Some(format!(
                    "{} {:?} {}: {:?}",
                    w.target, w.window, w.profile, w.outcome
                ))
            }
            _ => None,
        })
        .collect()
}

/// The last look at a target before teardown (spec AC8, R4): it exited, or
/// the kernel OOM-killed in its leaf, after its last reply.
pub fn final_state_failure(exited: bool, oom_kills: Option<u64>) -> Option<String> {
    (exited || oom_kills.is_some_and(|n| n >= 1))
        .then(|| "the target exited or was OOM-killed".to_owned())
}

/// Structural admission of `api.json` against the chunk's plan: the cells
/// are the plan's, in order; every cell's counts add up (planned is started
/// plus unstarted, and started is its rows); every row index is distinct and
/// below the plan; a blocked cell started nothing.
pub fn check_record(rec: &ApiRecord, chunk: &Chunk) -> Result<(), String> {
    let Chunk::Api {
        surface,
        target,
        repetition,
        fill,
    } = chunk
    else {
        return Err("not an api chunk".into());
    };
    if rec.surface != *surface
        || rec.target != target.name()
        || rec.repetition != *repetition
        || rec.fill.fill != *fill
    {
        return Err("api.json names another chunk".into());
    }
    api::check_brackets(rec).map_err(|e| format!("api.json: {e}"))?;
    let want = cells(*target, *surface);
    let names: Vec<String> = rec.cells.iter().map(|c| c.cell.clone()).collect();
    let planned: Vec<String> = want.iter().map(PlannedCell::name).collect();
    if names != planned {
        return Err(format!(
            "api.json's cells {names:?} are not the plan's {planned:?}"
        ));
    }
    for (c, p) in rec.cells.iter().zip(&want) {
        // The cell is the plan's (R10): its operation, concurrency and
        // scope, and the method, routes and citations of its capability.
        let cap = clients::capability(target.name(), *surface, p.operation);
        let planned = CellRecord::new(p, &cap);
        if (
            c.operation,
            c.concurrency,
            c.scope,
            c.method,
            &c.routes,
            &c.cites,
        ) != (
            planned.operation,
            planned.concurrency,
            planned.scope,
            planned.method,
            &planned.routes,
            &planned.cites,
        ) {
            return Err(format!("api.json: cell {} is not the planned cell", c.cell));
        }
        let m = &c.measured;
        let ok = adds_up(m, p.planned, api::Phase::Measured)
            && (c.status == CellStatus::Measured) == c.blocked.is_none()
            && (c.status == CellStatus::Measured || m.started == 0);
        if !ok {
            return Err(format!("api.json: cell {} does not add up", c.cell));
        }
        // Its warmup: none planned, none run; one run adds up on its own;
        // a measured cell's warmup started every slot and none failed.
        let warm_ok = match (&c.warmup, p.warmup) {
            (None, 0) => true,
            (Some(_), 0) => false,
            (None, _) => c.status == CellStatus::Blocked,
            (Some(w), n) => {
                adds_up(w, n, api::Phase::Warmup)
                    && (c.status == CellStatus::Blocked
                        || (w.unstarted == 0
                            && w.rows.iter().all(|r| {
                                matches!(
                                    r.outcome,
                                    api::Outcome::Exact | api::Outcome::InexactFidelity { .. }
                                )
                            })))
            }
        };
        if !warm_ok {
            return Err(format!(
                "api.json: cell {}'s warmup does not add up",
                c.cell
            ));
        }
    }
    Ok(())
}

/// One phase's counts against its plan: planned is started plus
/// unstarted, started is its rows, every row of this phase with a distinct
/// index below the plan.
fn adds_up(r: &api::Ran, planned: u32, phase: api::Phase) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    r.planned == planned
        && r.started.checked_add(r.unstarted) == Some(r.planned)
        && r.started as usize == r.rows.len()
        && r.rows
            .iter()
            .all(|x| x.index < planned && x.phase == phase && seen.insert(x.index))
}

/// The seed and run tag the chunk's fill corpus derives from.
fn run_tag(run: &str, target: TargetKind, rep: u32, fill: u32) -> String {
    format!("{}-{target}-r{rep}-f{fill}", &run[..run.len().min(16)])
}

/// Run one native API chunk. See the module documentation.
pub async fn run(
    cfg: Config,
    chunk: Chunk,
    clock: Clock,
) -> (Verdict, Lifecycle, Option<std::path::PathBuf>) {
    let mut life = Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let infra = |e: String, life: Lifecycle| (Verdict::Infrastructure(e), life, None);
    let Chunk::Api {
        surface,
        target,
        repetition: rep,
        fill,
    } = chunk.clone()
    else {
        return infra("not an api chunk".into(), life);
    };
    if surface != Surface::Native {
        return infra(
            "api.mcp chunks are measured by the eval crate's api-mcp against a held target".into(),
            life,
        );
    }
    let p = &cfg.protocol;
    let seed = p.corpus.seed;
    let cells = cells(target, surface);
    let (_, capability_rows) = api::plan_cells(surface, |op| {
        clients::capability(target.name(), surface, op)
    });
    let growth = api::fixture_growth(seed, &cells);
    if let Err(e) = api::within_cap(fill, growth) {
        return infra(e, life);
    }
    let dir = match crate::records::RunDir::create(&cfg.output, &cfg.manifest) {
        Ok(d) => d,
        Err(e) => return infra(format!("output: {e}"), life),
    };
    let sink: Sink = Arc::new(Mutex::new(dir));
    let limits = crate::runner::limits(p);
    let Some(spec) = cfg.specs.get(&target) else {
        return infra(format!("{target}: no launch spec"), life);
    };
    let Some(scope) = spec.extra.scope.clone() else {
        return infra(format!("{target}: a protocol 4 launch has no scope"), life);
    };
    let chunk_end = clock.0 + Duration::from_millis(p.resources.chunk_deadline_ms.unwrap_or(0));
    let chunk_end_tokio = Instant::from_std(chunk_end);
    let run = cfg.manifest.run_id.clone();
    let profile = fill_profile(fill);
    let api_clock = api::Clock(clock.0);
    let mut failures: Vec<String> = Vec::new();
    let mut record = ApiRecord {
        surface,
        target: target.name().to_owned(),
        repetition: rep,
        fill: api::FillRecord {
            fill,
            ..Default::default()
        },
        fixtures: 0,
        growth,
        capabilities: capability_rows,
        cells: Vec::new(),
        abandoned: None,
        store: api::store_label(clients::Family::of(target.name()).expect("a launched family"))
            .to_owned(),
        ledger: Vec::new(),
        control: Vec::new(),
    };
    let caps = |op: Operation| clients::capability(target.name(), surface, op);
    let not_measured = |why: &str,
                        outcome: WindowOutcome,
                        launch: &str,
                        from: WindowKind,
                        record: &mut ApiRecord|
     -> Result<(), String> {
        never_measured(
            (&sink, &run, launch, &clock),
            (target, &profile, rep),
            (&cells, &caps),
            (why, outcome, from),
            record,
        )
    };
    // Launch, as the cost chunk's measured instance does: the sampler starts
    // when the leaf is created, from inside the launch.
    let scratch = cfg.scratch.join("leaf-0001");
    let interval = Duration::from_millis(p.resources.sample_interval_ms.unwrap_or(100));
    let sampler: Arc<Mutex<Option<crate::resources::Sampler>>> = Arc::default();
    let created: Arc<Mutex<Option<u64>>> = Arc::default();
    let hook = crate::runner::sampler_hook(
        (sampler.clone(), created.clone()),
        crate::runner::sink_emit(&sink),
        (run.clone(), target, clock, interval),
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
    let launched =
        crate::targets::Owned::launch(&spec, &scratch, &limits, &clock, &cfg.keepers).await;
    let mut owned = match launched {
        Ok(o) => o,
        Err(e) => {
            let _ = stop_sampler().await;
            let step = match e {
                crate::targets::LaunchError::NotReady(why, facts, launch) => {
                    let name = launch.as_ref().and_then(|l| l.leaf.clone());
                    let cause = crate::runner::classify_not_ready(&facts, launch.is_some());
                    life.cleanups.push(facts);
                    life.launches.extend(launch.map(|l| *l));
                    match cause {
                        Some(cause) => not_measured(
                            cause.reason(),
                            cause.outcome(),
                            name.as_deref().unwrap_or("none"),
                            WindowKind::Ready,
                            &mut record,
                        )
                        .map(|()| failures.push(format!("{target}: {}", cause.reason()))),
                        None => Err(format!("{target}: not ready ({why})")),
                    }
                }
                crate::targets::LaunchError::Leaf(why, facts) => {
                    life.cleanups.extend(facts);
                    Err(format!("{target}: {why}"))
                }
                other => Err(format!("{target}: launch refused ({other:?})")),
            };
            if let Err(e) = step {
                return infra(e, life);
            }
            return seal(sink, &cfg, &chunk, record, life, failures);
        }
    };
    life.launches.push(owned.facts.clone());
    let outcome = measure(
        &mut owned,
        (&cfg, &chunk, &clock, api_clock),
        (&sink, &run, &profile, &scratch),
        (&cells, &caps, chunk_end_tokio, &scope.layout.target_cpus),
        &created,
        &mut record,
        &mut failures,
    )
    .await;
    let sampled = stop_sampler().await;
    let facts = owned.shutdown().await;
    let proved = facts.proved();
    life.cleanups.push(facts);
    if !proved {
        return infra(format!("{target}: cleanup not proved"), life);
    }
    if let Err(e) = sampled.and(outcome) {
        return infra(e, life);
    }
    seal(sink, &cfg, &chunk, record, life, failures)
}

type Caps<'a> = &'a dyn Fn(Operation) -> clients::Capability;

/// Readiness to the last cell, on a launched instance. An `Err` is an
/// infrastructure failure; the target's own failures are rows and land in
/// `failures`.
#[allow(clippy::too_many_arguments)]
async fn measure(
    owned: &mut crate::targets::Owned,
    (cfg, chunk, clock, api_clock): (&Config, &Chunk, &Clock, api::Clock),
    (sink, run, profile, scratch): (&Sink, &str, &str, &std::path::Path),
    (cells, caps, chunk_end, target_cpus): (&[PlannedCell], Caps<'_>, Instant, &[u32]),
    created: &Arc<Mutex<Option<u64>>>,
    record: &mut ApiRecord,
    failures: &mut Vec<String>,
) -> Result<(), String> {
    let (target, rep, fill) = (chunk.target(), chunk.repetition(), record.fill.fill);
    let leaf = owned.leaf.clone().ok_or("a leaf launch has no leaf")?;
    let launch = leaf.name().to_owned();
    let created_ns = created
        .lock()
        .expect("created lock")
        .unwrap_or_else(|| clock.now_ns());
    let closing = Closing {
        run,
        launch: &launch,
        key: (target, profile, rep),
        leaf: &leaf,
        proc_root: std::path::Path::new("/proc"),
        target_cpus,
    };
    let block_rest = |why: &str,
                      outcome: WindowOutcome,
                      from_idle_fill: bool,
                      record: &mut ApiRecord|
     -> Result<(), String> {
        if from_idle_fill {
            unopened(
                sink,
                (run, &launch),
                (target, profile, rep),
                WindowKind::IdleFill,
                clock,
                outcome,
            )?;
        }
        for c in cells {
            record.cells.push(CellRecord::blocked(
                c,
                &caps(c.operation),
                why,
                clock.now_ns(),
            ));
            unopened(
                sink,
                (run, &launch),
                (target, &cell_profile(c), rep),
                WindowKind::ApiCell,
                clock,
                outcome,
            )?;
        }
        Ok(())
    };
    // ready: leaf creation to readiness; idle_ready: five quiet seconds.
    let ready = bound(&leaf, run, target, clock, sink)?;
    let zero = Snap {
        t_ns: created_ns,
        cpu: Default::default(),
        events: Default::default(),
    };
    let r = window_outcome(&zero.events, &ready.events, owned.has_exited(), false);
    emit_window(
        sink,
        closing.window(WindowKind::Ready, (&zero, &ready), None, r),
    )?;
    let idle = idle_ready(
        &closing,
        (owned, scratch),
        sink,
        clock,
        Some(chunk_end.into_std()),
    )
    .await?;
    let i = idle.outcome;
    emit_window(sink, idle)?;
    if r != WindowOutcome::Measured || i != WindowOutcome::Measured {
        failures.push(format!(
            "{target}: before the fill: ready {r:?}, idle {i:?}"
        ));
        return block_rest("target_failed_before_fill", i.max_cause(r), true, record);
    }
    if owned.http_addr.port() == 0 {
        return Err(format!("{target}: no HTTP listener to read its API on"));
    }
    let client = clients::Client::new(target.name(), owned.http_addr)
        .ok_or_else(|| format!("{target}: no API family"))?;
    // fill: untimed, outside every window, within its own bound.
    let tag = run_tag(run, target, rep, fill);
    let (filled, background) = api::fill_ledger(
        owned.smtp,
        &client,
        (cfg.protocol.corpus.seed, &tag),
        fill,
        api_clock,
    )
    .await;
    record.fill = filled;
    if !record.fill.reached {
        failures.push(format!(
            "{target} f{fill}: fill_not_reached ({})",
            record.fill.detail.clone().unwrap_or_default()
        ));
        let outcome =
            never_opened_outcome(&leaf, owned.has_exited(), WindowOutcome::FillNotReached);
        return block_rest("fill_not_reached", outcome, true, record);
    }
    // idle_fill_N: five quiet seconds with the fill in place, before any
    // fixture (0132 §4.3).
    let mut idle = idle_ready(
        &closing,
        (owned, scratch),
        sink,
        clock,
        Some(chunk_end.into_std()),
    )
    .await?;
    idle.window = WindowKind::IdleFill;
    emit_window(sink, idle)?;
    // fixtures: authored before the server sees them, resolved untimed.
    let mut fixtures = api::plan_fixtures(cfg.protocol.corpus.seed, cells);
    if let Err(e) = api::prepare(owned.smtp, &client, &mut fixtures).await {
        failures.push(format!("{target} f{fill}: fixtures not prepared: {e}"));
        let outcome = never_opened_outcome(&leaf, owned.has_exited(), WindowOutcome::Blocked);
        return block_rest(
            &format!("fixtures_not_prepared: {e}"),
            outcome,
            false,
            record,
        );
    }
    record.fixtures = fixtures.len() as u64;
    let workers = cells
        .iter()
        .filter_map(|c| c.concurrency)
        .max()
        .unwrap_or(1);
    let pool: Vec<clients::Client> = (0..workers)
        .map(|_| clients::Client::new(target.name(), owned.http_addr).expect("a family"))
        .collect();
    let native = api::NativeSurface {
        clients: Arc::new(pool),
        fixtures: Arc::new(fixtures),
    };
    let exited = || owned.has_exited();
    let meter = LeafMeter {
        leaf: &leaf,
        run,
        launch: &launch,
        target,
        repetition: rep,
        clock: *clock,
        sink,
        target_cpus,
        exited: &exited,
        open: Mutex::new(BTreeMap::new()),
        cells,
    };
    let split = cells
        .iter()
        .position(|c| c.concurrency.is_none())
        .unwrap_or(cells.len());
    let (head, tail) = cells.split_at(split);
    let (mut recs, mut abandoned) =
        api::run_cells(head, caps, &native, &meter, api_clock, Some(chunk_end)).await?;
    // The complete batch ledger, before the global clear or at batch end
    // (R6). An abandoned launch runs no ledger and no final clear: what an
    // unknown mutation did cannot be told apart.
    if abandoned.is_none() {
        let background = background.unwrap_or_default();
        let problems = api::final_ledger(
            &native.clients[0],
            (cfg.protocol.corpus.seed, &background),
            &native.fixtures,
            head,
            &mut recs,
        )
        .await;
        record.ledger = problems;
    }
    if let Some(why) = &abandoned {
        for c in tail {
            recs.push(CellRecord::blocked(
                c,
                &caps(c.operation),
                why,
                clock.now_ns(),
            ));
        }
    } else {
        let (more, ab) =
            api::run_cells(tail, caps, &native, &meter, api_clock, Some(chunk_end)).await?;
        recs.extend(more);
        abandoned = ab;
    }
    for r in recs.iter().filter(|r| r.status == CellStatus::Blocked) {
        let cause = if r.blocked.as_deref() == Some("chunk_deadline") {
            WindowOutcome::ChunkDeadline
        } else {
            WindowOutcome::Blocked
        };
        let outcome = never_opened_outcome(&leaf, owned.has_exited(), cause);
        let c = cells
            .iter()
            .find(|c| c.name() == r.cell)
            .expect("a planned cell");
        unopened(
            sink,
            (run, &launch),
            (target, &cell_profile(c), rep),
            WindowKind::ApiCell,
            clock,
            outcome,
        )?;
        if r.blocked.as_deref() != Some("chunk_deadline") {
            failures.push(format!(
                "{target} f{fill} {}: {}",
                r.cell,
                r.blocked.clone().unwrap_or_default()
            ));
        }
    }
    if let Some(why) = &abandoned {
        failures.push(format!("{target} f{fill}: launch abandoned: {why}"));
    }
    if let Some(why) = final_state_failure(
        owned.has_exited(),
        leaf.mem_events().ok().map(|e| e.oom_kill),
    ) {
        failures.push(format!("{target} f{fill}: {why}"));
    }
    record.cells = recs;
    record.abandoned = abandoned;
    Ok(())
}

/// Every window and cell of a launch that never measured, on either
/// surface (R4): the windows from `from` on are written unopened with
/// `outcome`, and every planned cell is blocked by `why`.
fn never_measured(
    (sink, run, launch, clock): (&Sink, &str, &str, &Clock),
    (target, profile, rep): (TargetKind, &str, u32),
    (cells, caps): (&[PlannedCell], Caps<'_>),
    (why, outcome, from): (&str, WindowOutcome, WindowKind),
    record: &mut ApiRecord,
) -> Result<(), String> {
    let kinds = [
        WindowKind::Ready,
        WindowKind::IdleReady,
        WindowKind::IdleFill,
    ];
    let skip = kinds.iter().position(|k| *k == from).unwrap_or(kinds.len());
    for k in &kinds[skip..] {
        unopened(
            sink,
            (run, launch),
            (target, profile, rep),
            *k,
            clock,
            outcome,
        )?;
    }
    for c in cells {
        record.cells.push(CellRecord::blocked(
            c,
            &caps(c.operation),
            why,
            clock.now_ns(),
        ));
        unopened(
            sink,
            (run, launch),
            (target, &cell_profile(c), rep),
            WindowKind::ApiCell,
            clock,
            outcome,
        )?;
    }
    Ok(())
}

/// A chunk's record before anything measured: every planned cell to come.
fn blank_record(target: TargetKind, surface: Surface, rep: u32, fill: u32, seed: u64) -> ApiRecord {
    let cells = cells(target, surface);
    let (_, capabilities) = api::plan_cells(surface, |op| {
        clients::capability(target.name(), surface, op)
    });
    ApiRecord {
        surface,
        target: target.name().to_owned(),
        repetition: rep,
        fill: api::FillRecord {
            fill,
            ..Default::default()
        },
        fixtures: 0,
        growth: api::fixture_growth(seed, &cells),
        capabilities,
        cells: Vec::new(),
        abandoned: None,
        store: api::store_label(clients::Family::of(target.name()).expect("a launched family"))
            .to_owned(),
        ledger: Vec::new(),
        control: Vec::new(),
    }
}

/// Write `api.json` and the report, then seal: only a proved cleanup seals.
fn seal(
    sink: Sink,
    cfg: &Config,
    chunk: &Chunk,
    record: ApiRecord,
    life: Lifecycle,
    failures: Vec<String>,
) -> (Verdict, Lifecycle, Option<std::path::PathBuf>) {
    let infra = |e: String, life: Lifecycle| (Verdict::Infrastructure(e), life, None);
    let dir = Arc::try_unwrap(sink)
        .ok()
        .expect("no sink clone outlives the chunk")
        .into_inner()
        .expect("sink lock");
    if let Err(e) = check_record(&record, chunk) {
        return infra(format!("accounting: {e}"), life);
    }
    let bytes = serde_json::to_vec_pretty(&record).expect("the record serializes");
    if let Err(e) = dir.write_artifact("api.json", &bytes) {
        return infra(format!("api.json: {e}"), life);
    }
    let (rows, resources) = match crate::records::read_run(dir.path(), &cfg.manifest) {
        Ok(r) => r,
        Err(e) => return infra(format!("accounting: {e}"), life),
    };
    // One verdict for both surfaces (R4): the record's failed rows and the
    // windows' OOM or exit outcomes are the target's own failures, whoever
    // measured them.
    let mut failures = failures;
    if let Err(e) = check_bracket_windows(&record, &resources) {
        return infra(format!("accounting: {e}"), life);
    }
    failures.extend(row_failures(&record));
    failures.extend(window_failures(&resources));
    if !record.ledger.is_empty() {
        failures.push(format!(
            "{} f{}: ledger: {}",
            record.target,
            record.fill.fill,
            record.ledger.join("; ")
        ));
    }
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
    let report = crate::report::report_v4(
        &cfg.protocol,
        &cfg.manifest,
        &rows,
        &[&resources[..]],
        &life,
    );
    let mut v = serde_json::to_value(&report).expect("report serializes");
    v["api"] = api_report(&record, &resources);
    if let Err(e) = dir.write_artifact("report.json", &serde_json::to_vec_pretty(&v).expect("json"))
    {
        return infra(format!("report: {e}"), life);
    }
    match dir.finalize(
        life.cleanups
            .iter()
            .all(crate::targets::CleanupFacts::proved),
    ) {
        Ok(path) if failures.is_empty() => (Verdict::Clean, life, Some(path)),
        Ok(path) => (Verdict::WorkloadFailures(failures), life, Some(path)),
        Err(e) => infra(e, life),
    }
}

trait Cause {
    fn max_cause(self, other: WindowOutcome) -> WindowOutcome;
}

impl Cause for WindowOutcome {
    /// The first of OOM, exit, deadline between two outcomes: an OOM is a
    /// kernel fact and is never relabelled.
    fn max_cause(self, other: WindowOutcome) -> WindowOutcome {
        for o in [
            WindowOutcome::OomKilled,
            WindowOutcome::TargetExited,
            WindowOutcome::ChunkDeadline,
        ] {
            if self == o || other == o {
                return o;
            }
        }
        WindowOutcome::Blocked
    }
}

/// Recheck every citation of `target`'s capability tables, on both surfaces,
/// before anything launches (spec AC1): a document citation must be a
/// document of the target's pin in `lock` (whose path, size and digest
/// `SourceLock::verify_files` has just rechecked) and must contain its
/// needle; a `repo:` citation must name a file of the candidate's tree under
/// `repo` that contains it. Missing evidence refuses; it never becomes an
/// unsupported capability.
pub fn check_citations(
    target: TargetKind,
    lock: &crate::protocol::SourceLock,
    documents: &std::path::Path,
    repo: &std::path::Path,
) -> Result<(), Vec<String>> {
    let mut out = Vec::new();
    let pin_docs: Vec<String> = lock
        .pin(target)
        .map(|p| p.documents.iter().map(|d| d.path.clone()).collect())
        .unwrap_or_default();
    for surface in [Surface::Native, Surface::Mcp] {
        for op in Operation::ALL {
            let cap = clients::capability(target.name(), surface, op);
            for cite in cap.cites() {
                let (path, bytes) = match cite.doc.strip_prefix("repo:") {
                    Some(rel) => (rel.to_owned(), std::fs::read(repo.join(rel)).ok()),
                    None => {
                        let path = format!("sources/{target}/{}", cite.doc);
                        if !pin_docs.contains(&path) {
                            out.push(format!(
                                "{target} {} {}: {path} is no document of its lock pin",
                                surface.name(),
                                op.name()
                            ));
                            continue;
                        }
                        (path.clone(), std::fs::read(documents.join(&path)).ok())
                    }
                };
                match bytes {
                    None => out.push(format!("{target}: {path} cannot be read")),
                    Some(b)
                        if !b
                            .windows(cite.needle.len())
                            .any(|w| w == cite.needle.as_bytes()) =>
                    {
                        out.push(format!(
                            "{target} {} {}: {path} does not state {:?}",
                            surface.name(),
                            op.name(),
                            cite.needle
                        ))
                    }
                    Some(_) => {}
                }
            }
        }
    }
    out.sort();
    out.dedup();
    if out.is_empty() { Ok(()) } else { Err(out) }
}

// ============================================================================
// `hold --control api-v1`: the MCP chunk's resource owner (spec AC8, T7)
// ============================================================================

/// The longest control line but the record's.
pub const MAX_CONTROL_LINE: usize = 1024;
/// The longest `record` line: every row of every cell of one chunk.
pub const MAX_RECORD_LINE: usize = 64 << 20;

/// One control command, closed: an unknown key or command refuses.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    WindowStart { id: String },
    WindowEnd { id: String },
    IdleFill {},
    Record { api: Box<ApiRecord> },
}

/// What the control loop has seen, so a duplicate, an unmatched end, a
/// command after the record or a foreign cell refuses by name.
#[derive(Debug, Default)]
pub struct ControlState {
    pub opened: BTreeMap<String, Snap>,
    pub closed: std::collections::BTreeSet<String>,
    pub idle_fill: bool,
    pub record: Option<ApiRecord>,
}

/// Parse one control line: bounded, closed and well-formed.
pub fn parse_control(line: &[u8]) -> Result<Control, String> {
    let is_record = line.starts_with(b"{\"cmd\":\"record\"");
    let bound = if is_record {
        MAX_RECORD_LINE
    } else {
        MAX_CONTROL_LINE
    };
    if line.len() > bound {
        return Err(format!(
            "a control line of {} bytes exceeds {bound}",
            line.len()
        ));
    }
    serde_json::from_slice(line).map_err(|e| format!("control line: {e}"))
}

/// Check one command against the plan and the state before acting on it.
pub fn admit_control(c: &Control, st: &ControlState, cells: &[PlannedCell]) -> Result<(), String> {
    if st.record.is_some() {
        return Err("a control command after the record".into());
    }
    let planned = |id: &str| cells.iter().any(|c| c.name() == id);
    match c {
        Control::WindowStart { id } if !planned(id) => {
            Err(format!("window {id:?} is not a planned cell"))
        }
        Control::WindowStart { id } if st.opened.contains_key(id) || st.closed.contains(id) => {
            Err(format!("window {id:?} was already opened"))
        }
        Control::WindowStart { .. } if !st.opened.is_empty() => {
            Err("a window is already open".into())
        }
        Control::WindowEnd { id } if !st.opened.contains_key(id) => {
            Err(format!("window {id:?} was not opened"))
        }
        Control::IdleFill {} if st.idle_fill => Err("idle_fill was already taken".into()),
        Control::IdleFill {} if !st.opened.is_empty() || !st.closed.is_empty() => {
            Err("idle_fill comes before every cell".into())
        }
        Control::Record { .. } if !st.opened.is_empty() => {
            Err("the record came with a window still open".into())
        }
        _ => Ok(()),
    }
}

/// After the record: every planned cell window is either one the control
/// opened and closed, or a cell the record says was blocked; never both,
/// never neither.
pub fn windows_agree(st: &ControlState, rec: &ApiRecord) -> Result<(), String> {
    for c in &rec.cells {
        let opened = st.closed.contains(&c.cell);
        match (c.status, opened) {
            (CellStatus::Measured, true) | (CellStatus::Blocked, false) => {}
            (s, o) => {
                return Err(format!(
                    "cell {}: status {s:?} but window opened {o}",
                    c.cell
                ));
            }
        }
    }
    Ok(())
}

/// The next control line, unless the chunk's deadline passes first: then
/// the hold ends as an infrastructure failure and tears down (R12).
pub async fn next_control_line<R: tokio::io::AsyncBufRead + Unpin>(
    r: &mut R,
    chunk_end: std::time::Instant,
) -> Result<Option<Vec<u8>>, String> {
    tokio::time::timeout_at(Instant::from_std(chunk_end), read_line(r))
        .await
        .map_err(|_| "the chunk deadline passed with the control input still open".to_owned())?
}

/// Read one line of at most `MAX_RECORD_LINE` bytes; `None` at EOF.
async fn read_line<R: tokio::io::AsyncBufRead + Unpin>(
    r: &mut R,
) -> Result<Option<Vec<u8>>, String> {
    use tokio::io::AsyncBufReadExt;
    let mut line = Vec::new();
    let n = tokio::io::AsyncReadExt::take(&mut *r, MAX_RECORD_LINE as u64 + 1)
        .read_until(b'\n', &mut line)
        .await
        .map_err(|e| format!("control input: {e}"))?;
    if n == 0 {
        return Ok(None);
    }
    if line.last() != Some(&b'\n') {
        return Err("a control line without its end, or over the bound".into());
    }
    line.pop();
    Ok(Some(line))
}

/// The MCP chunk's owner (spec AC8): launch the target under its leaf with
/// the sampler, take `ready` and `idle_ready`, print the endpoints, then obey
/// `api-v1` control lines on `input` until EOF: open and close each cell's
/// window on this process's clock, take `idle_fill` once, and accept the
/// record last. Then tear down, write every window the record says was never
/// opened, and seal only a record that agrees with the windows and a
/// cleanup that is proved. Any refused line ends the hold: teardown, no seal.
#[allow(clippy::too_many_arguments)]
pub async fn hold_api<R, W>(
    cfg: Config,
    chunk: Chunk,
    clock: Clock,
    mut input: R,
    out: &mut W,
) -> (Verdict, Lifecycle, Option<std::path::PathBuf>)
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: std::io::Write,
{
    let mut life = Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let infra = |e: String, life: Lifecycle| (Verdict::Infrastructure(e), life, None);
    let (target, rep, fill) = (
        chunk.target(),
        chunk.repetition(),
        match chunk {
            Chunk::Api { fill, .. } => fill,
            _ => 0,
        },
    );
    let cells = cells(target, Surface::Mcp);
    let p = &cfg.protocol;
    let dir = match crate::records::RunDir::create(&cfg.output, &cfg.manifest) {
        Ok(d) => d,
        Err(e) => return infra(format!("output: {e}"), life),
    };
    let sink: Sink = Arc::new(Mutex::new(dir));
    let limits = crate::runner::limits(p);
    let Some(spec) = cfg.specs.get(&target) else {
        return infra(format!("{target}: no launch spec"), life);
    };
    let Some(scope) = spec.extra.scope.clone() else {
        return infra(format!("{target}: a protocol 4 launch has no scope"), life);
    };
    let run = cfg.manifest.run_id.clone();
    let profile = fill_profile(fill);
    let scratch = cfg.scratch.join("leaf-0001");
    let interval = Duration::from_millis(p.resources.sample_interval_ms.unwrap_or(100));
    let sampler: Arc<Mutex<Option<crate::resources::Sampler>>> = Arc::default();
    let created: Arc<Mutex<Option<u64>>> = Arc::default();
    let mut spec = spec.clone();
    spec.extra.on_leaf = Some(crate::runner::sampler_hook(
        (sampler.clone(), created.clone()),
        crate::runner::sink_emit(&sink),
        (run.clone(), target, clock, interval),
    ));
    let stop_sampler = || async {
        let s = sampler.lock().expect("sampler lock").take();
        match s {
            Some(s) => s.stop().await.map(|_| ()),
            None => Ok(()),
        }
    };
    let mut owned =
        match crate::targets::Owned::launch(&spec, &scratch, &limits, &clock, &cfg.keepers).await {
            Ok(o) => o,
            Err(e) => {
                let _ = stop_sampler().await;
                // As on the native surface (R4): a target that crashed or was
                // OOM-killed before readiness is its own measured failure —
                // every window and cell blocked, sealed, exit 1 — and anything
                // else (no delegation, a refused leaf) is infrastructure.
                let step = match e {
                    crate::targets::LaunchError::NotReady(why, facts, launch) => {
                        let name = launch.as_ref().and_then(|l| l.leaf.clone());
                        let cause = crate::runner::classify_not_ready(&facts, launch.is_some());
                        life.cleanups.push(facts);
                        life.launches.extend(launch.map(|l| *l));
                        match cause {
                            Some(cause) => {
                                let mut record =
                                    blank_record(target, Surface::Mcp, rep, fill, p.corpus.seed);
                                let caps = |op: Operation| {
                                    clients::capability(target.name(), Surface::Mcp, op)
                                };
                                never_measured(
                                    (&sink, &run, name.as_deref().unwrap_or("none"), &clock),
                                    (target, &profile, rep),
                                    (&cells, &caps),
                                    (cause.reason(), cause.outcome(), WindowKind::Ready),
                                    &mut record,
                                )
                                .map(|()| (record, vec![format!("{target}: {}", cause.reason())]))
                            }
                            None => Err(format!("{target}: not ready ({why})")),
                        }
                    }
                    crate::targets::LaunchError::Leaf(why, facts) => {
                        life.cleanups.extend(facts);
                        Err(format!("{target}: {why}"))
                    }
                    other => Err(format!("{target} did not launch: {other:?}")),
                };
                let receipt = serde_json::json!({
                    "launch": life.launches.last(),
                    "cleanup": life.cleanups.last(),
                });
                let _ = writeln!(out, "{receipt}").and_then(|()| out.flush());
                return match step {
                    Ok((record, failures)) => seal(sink, &cfg, &chunk, record, life, failures),
                    Err(e) => infra(e, life),
                };
            }
        };
    life.launches.push(owned.facts.clone());
    // The holder enforces the chunk's lifetime itself (R12): a caller that
    // hangs or disappears cannot keep the target past it.
    let chunk_end = clock.0 + Duration::from_millis(p.resources.chunk_deadline_ms.unwrap_or(0));
    let result = control_loop(
        &owned,
        (&run, &profile, &scratch, &sink, &clock),
        (&cells, &chunk, &scope.layout.target_cpus, rep),
        (&created, chunk_end),
        &mut input,
        out,
    )
    .await;
    let sampled = stop_sampler().await;
    let facts = owned.shutdown().await;
    let proved = facts.proved();
    life.cleanups.push(facts);
    let receipt = serde_json::json!({
        "launch": life.launches.last(),
        "cleanup": life.cleanups.last(),
    });
    let _ = writeln!(out, "{receipt}").and_then(|()| out.flush());
    if !proved {
        return infra(format!("{target}: cleanup not proved"), life);
    }
    let (record, failures) = match sampled.and(result) {
        Ok(v) => v,
        Err(e) => return infra(e, life),
    };
    seal(sink, &cfg, &chunk, record, life, failures)
}

type Held<'a> = (&'a str, &'a str, &'a std::path::Path, &'a Sink, &'a Clock);

/// Readiness windows, the endpoints line and the control lines; returns the
/// record and the target's own failures, or an infrastructure error.
async fn control_loop<R, W>(
    owned: &crate::targets::Owned,
    (run, profile, scratch, sink, clock): Held<'_>,
    (cells, chunk, target_cpus, rep): (&[PlannedCell], &Chunk, &[u32], u32),
    (created, chunk_end): (&Arc<Mutex<Option<u64>>>, std::time::Instant),
    input: &mut R,
    out: &mut W,
) -> Result<(ApiRecord, Vec<String>), String>
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: std::io::Write,
{
    let target = chunk.target();
    let leaf = owned.leaf.clone().ok_or("a leaf launch has no leaf")?;
    let launch = leaf.name().to_owned();
    let created_ns = created
        .lock()
        .expect("created lock")
        .unwrap_or_else(|| clock.now_ns());
    let closing = Closing {
        run,
        launch: &launch,
        key: (target, profile, rep),
        leaf: &leaf,
        proc_root: std::path::Path::new("/proc"),
        target_cpus,
    };
    let ready = bound(&leaf, run, target, clock, sink)?;
    let zero = Snap {
        t_ns: created_ns,
        cpu: Default::default(),
        events: Default::default(),
    };
    let r = window_outcome(&zero.events, &ready.events, owned.has_exited(), false);
    emit_window(
        sink,
        closing.window(WindowKind::Ready, (&zero, &ready), None, r),
    )?;
    let idle = idle_ready(&closing, (owned, scratch), sink, clock, Some(chunk_end)).await?;
    emit_window(sink, idle)?;
    let endpoints =
        serde_json::to_string(&crate::runner::hold_endpoints(owned)).expect("endpoints serialize");
    writeln!(out, "{endpoints}")
        .and_then(|()| out.flush())
        .map_err(|e| format!("endpoints: {e}"))?;
    let mut st = ControlState::default();
    while let Some(line) = next_control_line(input, chunk_end).await? {
        let c = parse_control(&line)?;
        admit_control(&c, &st, cells)?;
        let reply = match c {
            Control::WindowStart { id } => {
                let snap = bound(&leaf, run, target, clock, sink)?;
                let t = snap.t_ns;
                st.opened.insert(id.clone(), snap);
                serde_json::json!({"ok": "window_start", "id": id, "t_ns": t})
            }
            Control::WindowEnd { id } => {
                let b = bound(&leaf, run, target, clock, sink)?;
                let a = st.opened.remove(&id).expect("admitted above");
                let cell = cells
                    .iter()
                    .find(|c| c.name() == id)
                    .expect("admitted above");
                let key_profile = cell_profile(cell);
                let k = Closing {
                    key: (target, &key_profile, rep),
                    ..closing_copy(&closing)
                };
                let outcome = window_outcome(&a.events, &b.events, owned.has_exited(), false);
                emit_window(sink, k.window(WindowKind::ApiCell, (&a, &b), None, outcome))?;
                st.closed.insert(id.clone());
                serde_json::json!({"ok": "window_end", "id": id, "t_ns": b.t_ns, "outcome": outcome})
            }
            Control::IdleFill {} => {
                let mut w =
                    idle_ready(&closing, (owned, scratch), sink, clock, Some(chunk_end)).await?;
                w.window = WindowKind::IdleFill;
                let t = w.end_ns;
                emit_window(sink, w)?;
                st.idle_fill = true;
                serde_json::json!({"ok": "idle_fill", "t_ns": t})
            }
            Control::Record { api } => {
                check_record(&api, chunk)?;
                windows_agree(&st, &api)?;
                st.record = Some(*api);
                serde_json::json!({"ok": "record"})
            }
        };
        writeln!(out, "{reply}")
            .and_then(|()| out.flush())
            .map_err(|e| format!("control reply: {e}"))?;
    }
    let rec = st
        .record
        .take()
        .ok_or("the control input ended without a record")?;
    let mut failures = Vec::new();
    if !st.idle_fill {
        let outcome = if rec.fill.reached {
            WindowOutcome::Blocked
        } else {
            WindowOutcome::FillNotReached
        };
        unopened(
            sink,
            (run, &launch),
            (target, profile, rep),
            WindowKind::IdleFill,
            clock,
            outcome,
        )?;
    }
    if !rec.fill.reached {
        failures.push(format!("{target}: fill_not_reached"));
    }
    for c in rec.cells.iter().filter(|c| c.status == CellStatus::Blocked) {
        let cell = cells
            .iter()
            .find(|p| p.name() == c.cell)
            .expect("checked against the plan");
        let cause = match c.blocked.as_deref() {
            Some("chunk_deadline") => WindowOutcome::ChunkDeadline,
            Some("fill_not_reached") => WindowOutcome::FillNotReached,
            _ => WindowOutcome::Blocked,
        };
        let outcome = never_opened_outcome(&leaf, owned.has_exited(), cause);
        unopened(
            sink,
            (run, &launch),
            (target, &cell_profile(cell), rep),
            WindowKind::ApiCell,
            clock,
            outcome,
        )?;
        failures.push(format!(
            "{target} {}: {}",
            c.cell,
            c.blocked.clone().unwrap_or_default()
        ));
    }
    if let Some(why) = &rec.abandoned {
        failures.push(format!("{target}: launch abandoned: {why}"));
    }
    if let Some(why) = final_state_failure(
        owned.has_exited(),
        leaf.mem_events().ok().map(|e| e.oom_kill),
    ) {
        failures.push(format!("{target}: {why}"));
    }
    Ok((rec, failures))
}

fn closing_copy<'a>(c: &Closing<'a>) -> Closing<'a> {
    Closing {
        run: c.run,
        launch: c.launch,
        key: c.key,
        leaf: c.leaf,
        proc_root: c.proc_root,
        target_cpus: c.target_cpus,
    }
}
