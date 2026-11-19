//! `hold --control testexec-v1`: the test-execution chunk's resource owner
//! (accepted decision 0132 §8, §11, story D). It launches one fresh target
//! under its cgroup leaf with the sampler, takes `ready` and `idle_ready`,
//! prints its endpoints and the chunk's client CPUs, then obeys the eval
//! crate's control lines: `window_start` and `window_end` of the one
//! `suite` window, on this process's clock, and the `record` last. Then it
//! tears down, checks the record against the plan and the window, and seals
//! only when cleanup is proved. Any refused line ends the hold: teardown, no
//! seal (exit 2).
//!
//! Exit classes follow 0132 §11.3: every trial that did not pass, every
//! unproved per-test cleanup, every unstarted trial and an OOM or exit in
//! the suite window are the target's measured losses (sealed, exit 1);
//! infrastructure, interruption or an unproved teardown exits 2 unsealed.

use crate::records::{ResourceRow, WindowKind, WindowOutcome, WindowRow};
use crate::resources::Snap;
use crate::runner::{
    Chunk, Closing, Config, Lifecycle, Sink, Verdict, bound, emit_window, idle_ready,
    never_opened_outcome, unopened_window, window_outcome,
};
use crate::suite::{self, Coord, SuiteRecord};
use crate::wire::Clock;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The one window a suite opens.
pub const SUITE_WINDOW: &str = "suite";
/// The longest control line but the record's.
pub const MAX_CONTROL_LINE: usize = 1024;

/// One control command, closed: an unknown key or command refuses.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    WindowStart { id: String },
    WindowEnd { id: String },
    Record { testexec: Box<SuiteRecord> },
}

/// What the loop has seen: the suite window's bounds and the record.
#[derive(Debug, Default)]
pub struct ControlState {
    pub opened: Option<Snap>,
    pub closed: Option<(Snap, Snap)>,
    pub record: Option<SuiteRecord>,
}

/// Parse one control line: bounded and closed. The record is bounded by the
/// reader's [`crate::api_chunk::MAX_RECORD_LINE`]; every other command by
/// [`MAX_CONTROL_LINE`], judged from the parsed command.
pub fn parse_control(line: &[u8]) -> Result<Control, String> {
    if line.len() > crate::api_chunk::MAX_RECORD_LINE {
        return Err(format!("a control line of {} bytes", line.len()));
    }
    let c: Control = serde_json::from_slice(line).map_err(|e| format!("control line: {e}"))?;
    if !matches!(c, Control::Record { .. }) && line.len() > MAX_CONTROL_LINE {
        return Err(format!(
            "a control line of {} bytes exceeds {MAX_CONTROL_LINE}",
            line.len()
        ));
    }
    Ok(c)
}

/// Check one command against the state before acting on it: one suite
/// window, opened once, closed once, and the record after it.
pub fn admit_control(c: &Control, st: &ControlState) -> Result<(), String> {
    if st.record.is_some() {
        return Err("a control command after the record".into());
    }
    match c {
        Control::WindowStart { id } | Control::WindowEnd { id } if id != SUITE_WINDOW => {
            Err(format!("window {id:?} is not the suite window"))
        }
        Control::WindowStart { .. } if st.opened.is_some() || st.closed.is_some() => {
            Err("the suite window was already opened".into())
        }
        Control::WindowEnd { .. } if st.opened.is_none() => {
            Err("the suite window was not opened".into())
        }
        Control::Record { .. } if st.opened.is_some() => {
            Err("the record came with the suite window still open".into())
        }
        _ => Ok(()),
    }
}

/// The record against the windows: a suite that started a trial opened and
/// closed its window; one that started nothing never opened it.
pub fn windows_agree(st: &ControlState, rec: &SuiteRecord) -> Result<(), String> {
    match (rec.rows.is_empty(), st.closed.is_some()) {
        (false, true) | (true, false) => {}
        (started, closed) => {
            return Err(format!(
                "the record started {} trial(s) but the suite window closed {closed}",
                if started { 0 } else { rec.rows.len() }
            ));
        }
    }
    if let (Some((a, b)), Some(s), Some(e)) = (&st.closed, &rec.window_start, &rec.window_end)
        && (s.owner_t_ns != a.t_ns || e.owner_t_ns != b.t_ns)
    {
        return Err("the record's window instants are not the holder's".into());
    }
    Ok(())
}

/// The suite's resource figures from the window and sample rows: target
/// CPU (user, system, total), the sampled peak and mean of `memory.current`
/// and `anon` within the window, its OOM and throttle counts, and the
/// leaf's lifetime peak beside them.
pub fn resource_report(parallelism: u32, resources: &[ResourceRow]) -> serde_json::Value {
    let profile = crate::runner::suite_profile(parallelism);
    let w = resources.iter().find_map(|r| match r {
        ResourceRow::Window(w) if w.window == WindowKind::Suite && w.profile == profile => Some(w),
        _ => None,
    });
    let Some(w) = w else {
        return serde_json::Value::Null;
    };
    let samples: Vec<&crate::records::SampleRow> = resources
        .iter()
        .filter_map(|r| match r {
            ResourceRow::Sample(s)
                if s.launch == w.launch && s.t_ns >= w.start_ns && s.t_ns <= w.end_ns =>
            {
                Some(s)
            }
            _ => None,
        })
        .collect();
    let peak = |f: &dyn Fn(&crate::records::SampleRow) -> u64| samples.iter().map(|s| f(s)).max();
    let mean = |f: &dyn Fn(&crate::records::SampleRow) -> u64| {
        (!samples.is_empty())
            .then(|| samples.iter().map(|s| f(s) as f64).sum::<f64>() / samples.len() as f64)
    };
    let counters = w.counters().map(|(a, b, c, d)| {
        serde_json::json!({
            "user_usec": b.user_usec.saturating_sub(a.user_usec),
            "system_usec": b.system_usec.saturating_sub(a.system_usec),
            "usage_usec": b.usage_usec.saturating_sub(a.usage_usec),
            "nr_throttled": b.nr_throttled.saturating_sub(a.nr_throttled),
            "throttled_usec": b.throttled_usec.saturating_sub(a.throttled_usec),
            "oom": d.oom.saturating_sub(c.oom),
            "oom_kill": d.oom_kill.saturating_sub(c.oom_kill),
        })
    });
    serde_json::json!({
        "start_ns": w.start_ns,
        "end_ns": w.end_ns,
        "outcome": w.outcome,
        "affinity_held": w.affinity_held,
        "target": counters,
        "samples": samples.len(),
        "peak_memory_current": peak(&|s| s.memory_current),
        "peak_anon": peak(&|s| s.anon),
        "mean_memory_current": mean(&|s| s.memory_current),
        "mean_anon": mean(&|s| s.anon),
    })
}

/// The chunk report's `testexec` key: the record's figures and the suite
/// window's, both recomputed from the files on admission.
pub fn suite_report(rec: &SuiteRecord, resources: &[ResourceRow]) -> serde_json::Value {
    let mut v = suite::report(rec);
    v["resources"] = resource_report(rec.parallelism, resources);
    v
}

/// The chunk's coordinate as the shared module states it.
pub fn coord_of(chunk: &Chunk) -> Option<Coord> {
    match chunk {
        Chunk::Testexec {
            target,
            parallelism,
            repetition,
        } => Some(Coord {
            target: target.name().to_owned(),
            parallelism: *parallelism,
            repetition: *repetition,
        }),
        _ => None,
    }
}

/// The record against the protocol: its seed is the corpus seed and its
/// wait method the target's `wait-best` row.
pub fn check_against(
    rec: &SuiteRecord,
    chunk: &Chunk,
    p: &crate::protocol::Protocol,
) -> Result<(), String> {
    let coord = coord_of(chunk).ok_or("not a testexec chunk")?;
    suite::check_record(rec, &coord)?;
    if rec.seed != p.corpus.seed {
        return Err(format!(
            "seed {} is not the corpus seed {}",
            rec.seed, p.corpus.seed
        ));
    }
    let want = wait_method(p, chunk.target())?;
    if rec.method != want {
        return Err(format!(
            "wait method {:?} is not the target's wait-best {want:?}",
            rec.method
        ));
    }
    Ok(())
}

/// The target's `wait-best` row, as the suite waits with it (0132 §8.1).
pub fn wait_method(
    p: &crate::protocol::Protocol,
    target: crate::protocol::TargetKind,
) -> Result<suite::WaitMethod, String> {
    use crate::protocol::Observation;
    let o = p
        .waits
        .iter()
        .find(|w| w.name == crate::protocol::BEST_WAIT)
        .and_then(|w| w.observation.iter().find(|o| o.target == target))
        .ok_or_else(|| format!("the protocol has no wait-best row for {target}"))?;
    Ok(match o.mode {
        Observation::Native => suite::WaitMethod::Native,
        Observation::Stream => suite::WaitMethod::Stream {
            backstop_ms: o.backstop_ms.ok_or("a stream row without its backstop")?,
        },
        Observation::Poll => suite::WaitMethod::Poll {
            interval_ms: o.interval_ms.ok_or("a poll row without its interval")?,
        },
    })
}

/// What the holder prints once the target is ready: its endpoints, its
/// leaf, and the chunk's client CPUs, where the eval side places its
/// worker threads before any of them starts (0132 §3.4).
#[derive(Debug, serde::Serialize)]
pub struct SuiteEndpoints {
    #[serde(flatten)]
    pub endpoints: crate::runner::HoldEndpoints,
    pub client_cpus: Vec<u32>,
}

/// The test-execution chunk's owner. See the module documentation.
pub async fn hold_suite<R, W>(
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
    let Some(coord) = coord_of(&chunk) else {
        return infra(
            "hold --control testexec-v1 holds testexec chunks only".into(),
            life,
        );
    };
    let target = chunk.target();
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
    let profile = crate::runner::suite_profile(coord.parallelism);
    let scratch = cfg.scratch.join("leaf-0001");
    let interval = Duration::from_millis(p.resources.sample_interval_ms.unwrap_or(100));
    let sampler: Arc<Mutex<Option<crate::resources::Sampler>>> = Arc::default();
    let created: Arc<Mutex<Option<u64>>> = Arc::default();
    let stop_sampler = || async {
        let s = sampler.lock().expect("sampler lock").take();
        match s {
            Some(s) => s.stop().await.map(|_| ()),
            None => Ok(()),
        }
    };
    let launched = crate::api_chunk::launch_sampled(
        spec,
        &sink,
        (sampler.clone(), created.clone()),
        (&run, target, clock, interval),
        (&scratch, &limits, &cfg.keepers),
    )
    .await;
    let mut owned = match launched {
        Ok(o) => o,
        Err(e) => {
            let _ = stop_sampler().await;
            // A target that crashed or was OOM-killed before readiness is
            // its own measured failure: every window unopened, every trial
            // unstarted, sealed, exit 1. Anything else is infrastructure.
            let step = match e {
                crate::targets::LaunchError::NotReady(why, facts, launch) => {
                    let name = launch.as_ref().and_then(|l| l.leaf.clone());
                    let cause = crate::runner::classify_not_ready(&facts, launch.is_some());
                    life.cleanups.push(facts);
                    life.launches.extend(launch.map(|l| *l));
                    match cause {
                        Some(cause) => {
                            let at = clock.now_ns();
                            let leaf = name.as_deref().unwrap_or("none");
                            let mut ok = Ok(());
                            for kind in
                                [WindowKind::Ready, WindowKind::IdleReady, WindowKind::Suite]
                            {
                                let w = unopened_window(
                                    &run,
                                    leaf,
                                    (target, &profile, coord.repetition),
                                    kind,
                                    at,
                                    cause.outcome(),
                                );
                                ok = ok.and(emit_window(&sink, w));
                            }
                            let method =
                                wait_method(p, target).unwrap_or(suite::WaitMethod::Native);
                            let mut rec = SuiteRecord::new(&coord, p.corpus.seed, method);
                            rec.block(cause.reason());
                            ok.map(|()| (rec, vec![format!("{target}: {}", cause.reason())]))
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
                Ok((rec, failures)) => seal(sink, &cfg, &chunk, rec, life, failures),
                Err(e) => infra(e, life),
            };
        }
    };
    life.launches.push(owned.facts.clone());
    // The holder enforces the chunk's lifetime itself: a caller that hangs
    // or disappears cannot keep the target past it.
    let chunk_end = clock.0 + Duration::from_millis(p.resources.chunk_deadline_ms.unwrap_or(0));
    let result = control_loop(
        &owned,
        (&run, &profile, &scratch, &sink, &clock),
        (&chunk, &scope.layout, coord.repetition),
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
    let (rec, failures) = match sampled.and(result) {
        Ok(v) => v,
        Err(e) => return infra(e, life),
    };
    seal(sink, &cfg, &chunk, rec, life, failures)
}

type Held<'a> = (&'a str, &'a str, &'a std::path::Path, &'a Sink, &'a Clock);

/// Readiness windows, the endpoints line and the control lines; returns the
/// record and the target's own failures, or an infrastructure error.
async fn control_loop<R, W>(
    owned: &crate::targets::Owned,
    (run, profile, scratch, sink, clock): Held<'_>,
    (chunk, layout, rep): (&Chunk, &crate::resources::Layout, u32),
    (created, chunk_end): (&Arc<Mutex<Option<u64>>>, std::time::Instant),
    input: &mut R,
    out: &mut W,
) -> Result<(SuiteRecord, Vec<String>), String>
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
        target_cpus: &layout.target_cpus,
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
    let endpoints = SuiteEndpoints {
        endpoints: crate::runner::hold_endpoints(owned),
        client_cpus: layout.client_cpus.clone(),
    };
    let line = serde_json::to_string(&endpoints).expect("endpoints serialize");
    writeln!(out, "{line}")
        .and_then(|()| out.flush())
        .map_err(|e| format!("endpoints: {e}"))?;
    let mut st = ControlState::default();
    while let Some(line) = crate::api_chunk::next_control_line(input, chunk_end).await? {
        let c = parse_control(&line)?;
        admit_control(&c, &st)?;
        let reply = match c {
            Control::WindowStart { id } => {
                let snap = bound(&leaf, run, target, clock, sink)?;
                let t = snap.t_ns;
                st.opened = Some(snap);
                serde_json::json!({"ok": "window_start", "id": id, "t_ns": t})
            }
            Control::WindowEnd { id } => {
                let b = bound(&leaf, run, target, clock, sink)?;
                let a = st.opened.take().expect("admitted above");
                let outcome = window_outcome(&a.events, &b.events, owned.has_exited(), false);
                emit_window(
                    sink,
                    closing.window(WindowKind::Suite, (&a, &b), None, outcome),
                )?;
                let t = b.t_ns;
                st.closed = Some((a, b));
                serde_json::json!({"ok": "window_end", "id": id, "t_ns": t, "outcome": outcome})
            }
            Control::Record { testexec } => {
                windows_agree(&st, &testexec)?;
                st.record = Some(*testexec);
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
    if st.closed.is_none() {
        let outcome = never_opened_outcome(&leaf, owned.has_exited(), WindowOutcome::Blocked);
        emit_window(
            sink,
            unopened_window(
                run,
                &launch,
                (target, profile, rep),
                WindowKind::Suite,
                clock.now_ns(),
                outcome,
            ),
        )?;
    }
    let mut failures = Vec::new();
    if let Some(why) = crate::api_chunk::final_state_failure(
        owned.has_exited(),
        leaf.mem_events().ok().map(|e| e.oom_kill),
    ) {
        failures.push(format!("{target}: {why}"));
    }
    Ok((rec, failures))
}

/// Window outcomes that are the target's own failure.
fn window_failures(resources: &[ResourceRow]) -> Vec<String> {
    resources
        .iter()
        .filter_map(|r| match r {
            ResourceRow::Window(w @ WindowRow { outcome, .. })
                if matches!(
                    outcome,
                    WindowOutcome::OomKilled | WindowOutcome::TargetExited
                ) =>
            {
                Some(format!("{} {:?}: {:?}", w.target, w.window, outcome))
            }
            _ => None,
        })
        .collect()
}

/// Write `testexec.json` and the report, then seal: only a checked record
/// and a proved teardown seal.
fn seal(
    sink: Sink,
    cfg: &Config,
    chunk: &Chunk,
    rec: SuiteRecord,
    life: Lifecycle,
    failures: Vec<String>,
) -> (Verdict, Lifecycle, Option<std::path::PathBuf>) {
    let infra = |e: String, life: Lifecycle| (Verdict::Infrastructure(e), life, None);
    let dir = Arc::try_unwrap(sink)
        .ok()
        .expect("no sink clone outlives the chunk")
        .into_inner()
        .expect("sink lock");
    if let Err(e) = check_against(&rec, chunk, &cfg.protocol) {
        return infra(format!("accounting: {e}"), life);
    }
    let bytes = serde_json::to_vec_pretty(&rec).expect("the record serializes");
    if let Err(e) = dir.write_artifact("testexec.json", &bytes) {
        return infra(format!("testexec.json: {e}"), life);
    }
    let (rows, resources) = match crate::records::read_run(dir.path(), &cfg.manifest) {
        Ok(r) => r,
        Err(e) => return infra(format!("accounting: {e}"), life),
    };
    let mut failures = failures;
    failures.extend(suite::failures(&rec));
    failures.extend(window_failures(&resources));
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
    v[suite::SECTION] = suite_report(&rec, &resources);
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
