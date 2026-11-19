//! Protocol 4's test execution (accepted decision 0132 §8, §9, §12, story
//! D): the reference suite of 100 scored fixture trials at parallelism 1, 8
//! or 32 against one freshly launched target.
//!
//! A test is the existing fixture flow, end to end: the fixture app sends
//! (its existing `prepare` trigger), the oracle's existing scripted solution
//! waits through [`NativeSource`] with the target's `wait-best`, reads,
//! asserts and acts on the app's public plane, the fixture's app-state
//! scorer judges the real snapshot (0055), and the test cleans up under the
//! narrowest documented policy. No scenario code is new.
//!
//! The queue is the plan's claim order (`suite::plan`): one next index,
//! shared by every worker, one active trial per worker. Each worker owns a
//! [`Station`] bound before the suite clock and serves each claim a fresh
//! private generation, drained before the next. Every claim yields exactly
//! one row and one outcome; every planned id unclaimed when claims stop is
//! `unstarted` with its reason. Budgets are absolute: no claim at or after
//! the suite cutoff (540 s from the suite's start, which precedes the first
//! claim, or the chunk's own end), and every test, cleanup included, ends
//! inside the earlier of that cutoff and 30 s from its claim: its work stops
//! [`CLEANUP_RESERVE`] before its cleanup deadline, which is itself
//! [`SETTLE_GRACE`] before the trial's cancellation, [`CANCEL_MARGIN`]
//! inside the bound. A suite that can claim nothing never opens its window
//! (the holder's rule, `benches/mail_compare/suite_chunk.rs`).

use crate::api::Clock;
use crate::clients::{self, Client};
use crate::fixture::{Ack, Station, TrialConfig};
use crate::native_source::{Failure, Mutations, NativeSource};
use crate::suite::{
    self, Cleanup, ClientCpu, Coord, FinalCleanup, Outcome, PlannedTrial, Policy, SuiteRecord,
    TrialRow, Trip, Unstarted, WaitMethod, WaitTiming,
};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

/// How long a retired fixture generation gets to finish its handlers
/// before the station is cut (and the suite with it).
pub const DRAIN_BOUND: Duration = Duration::from_secs(5);
/// How far inside a test's bound (the earlier of 30 s from its claim and the
/// suite cutoff) a trial still running is cancelled, its row a timeout: the
/// row's end then lies inside the bound however late the timer fires.
pub const CANCEL_MARGIN: Duration = Duration::from_secs(1);
/// What a trial gets between its cleanup deadline and its cancellation to
/// come back with its row.
pub const SETTLE_GRACE: Duration = Duration::from_millis(500);
/// The tail of every test kept for its own cleanup: the work (delivery,
/// wait, solution, scoring) stops this long before the cleanup deadline, so
/// a test that timed out still recovers and removes its leftovers.
pub const CLEANUP_RESERVE: Duration = Duration::from_secs(3);
/// No claim with less than this left before the suite cutoff: a claim
/// always has its margins and some work before the bound.
pub const CLAIM_FLOOR: Duration = Duration::from_secs(6);
/// The chunk's tail kept for the suite's one global clear and its check.
pub const FINAL_RESERVE: Duration = Duration::from_secs(30);
/// The holder's one window id (`benches/mail_compare/suite_chunk.rs`).
pub const SUITE_WINDOW: &str = "suite";

/// The holder's suite window, on its own clock.
#[allow(async_fn_in_trait)]
pub trait SuiteMeter {
    /// `window_start` or `window_end`: one round trip, both instants kept.
    async fn window(&self, cmd: &str) -> Result<Trip, String>;
}

/// A meter with no holder: the window is this process's own instant (the
/// eval crate's tests against an unheld target).
pub struct LocalMeter(pub Clock);

impl SuiteMeter for LocalMeter {
    async fn window(&self, _cmd: &str) -> Result<Trip, String> {
        let t = self.0.now_ns();
        Ok(Trip {
            sent_ns: t,
            received_ns: t,
            owner_t_ns: t,
        })
    }
}

/// Everything one suite needs, fixed before the first claim.
#[derive(Clone)]
pub struct SuiteConfig {
    pub coord: Coord,
    pub seed: u64,
    pub method: WaitMethod,
    pub smtp: std::net::SocketAddr,
    pub http: std::net::SocketAddr,
    pub smtp_auth: crate::mail::SmtpAuth,
    /// The trials, in claim order: `suite::plan(seed)` in a real chunk.
    pub plan: Vec<PlannedTrial>,
    pub clock: Clock,
    /// One test's budget and the suite's, from the protocol.
    pub test_deadline: Duration,
    pub suite_deadline: Duration,
    /// The chunk's own end for work: nothing runs past it.
    pub work_end: Instant,
}

/// This process's CPU so far (`RUSAGE_SELF`).
pub fn rusage() -> Option<ClientCpu> {
    // SAFETY: getrusage fills a caller-owned struct.
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) } != 0 {
        return None;
    }
    let usec = |t: libc::timeval| t.tv_sec as u64 * 1_000_000 + t.tv_usec as u64;
    Some(ClientCpu {
        user_usec: usec(u.ru_utime),
        system_usec: usec(u.ru_stime),
    })
}

/// Confine every thread of this process to `cpus` (the chunk's client
/// CPUs, which the holder states), before any worker starts; threads made
/// later inherit it. Linux only, where the holder runs.
pub fn pin_process(cpus: &[u32]) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if cpus.is_empty() {
            return Err("the holder named no client CPUs".into());
        }
        // SAFETY: a zeroed cpu_set_t is the empty set; CPU_SET writes into it.
        let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
        for &c in cpus {
            unsafe { libc::CPU_SET(c as usize, &mut set) };
        }
        let tasks = std::fs::read_dir("/proc/self/task").map_err(|e| format!("threads: {e}"))?;
        for t in tasks.flatten() {
            let Some(tid) = t.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else {
                continue;
            };
            // SAFETY: the set outlives the call; a thread that exited since
            // the listing answers ESRCH, which is not a failure.
            let rc = unsafe {
                libc::sched_setaffinity(tid, std::mem::size_of::<libc::cpu_set_t>(), &set)
            };
            if rc != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(format!(
                    "pinning thread {tid}: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cpus;
        Err("client CPU placement needs Linux".into())
    }
}

/// The SMTP dialogue a target gets: AUTH PLAIN for Swarmail only, whose
/// username names the test's inbox (0004); the others take no AUTH.
pub fn smtp_auth(target: &str) -> crate::mail::SmtpAuth {
    if target == "swarmail" {
        crate::mail::SmtpAuth::Plain
    } else {
        crate::mail::SmtpAuth::None
    }
}

/// The final DATA reply that completed a wait's required set: the
/// `count`-th acceptance, in the order the replies were read. Every mail of
/// the five reference flows carries the subject its flow waits for, so the
/// trial's acceptances are the matching set (`tests/testexec.rs` holds the
/// fixture to that).
pub fn count_completing_ack(acks: &[Ack], count: u32, clock: Clock) -> Result<u64, String> {
    let mut delivered: Vec<std::time::Instant> =
        acks.iter().filter(|a| a.delivered).map(|a| a.at).collect();
    delivered.sort();
    let n = count.max(1) as usize;
    delivered
        .get(n - 1)
        .map(|at| at.saturating_duration_since(clock.0).as_nanos() as u64)
        .ok_or_else(|| {
            format!(
                "{} of {n} required deliveries acknowledged",
                delivered.len()
            )
        })
}

/// One worker's equipment: its fixture station and its keep-alive client.
pub struct WorkerKit {
    pub station: Station,
    pub client: Client,
    pub admin: reqwest::Client,
}

impl WorkerKit {
    /// A worker's fixture, bound before the suite clock, and its client.
    pub async fn bind(cfg: &SuiteConfig) -> Result<WorkerKit, String> {
        let station = Station::bind(cfg.smtp, cfg.smtp_auth)
            .await
            .map_err(|e| format!("fixture bind: {e}"))?;
        let client = Client::new(&cfg.coord.target, cfg.http)
            .ok_or_else(|| format!("{} has no client", cfg.coord.target))?;
        // No connection outlives its request: a pooled connection would be
        // pinned to the generation it was opened in.
        let admin = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(WorkerKit {
            station,
            client,
            admin,
        })
    }
}

/// A trial's end before the oracle could be judged: the fixture, its admin
/// plane or the snapshot failed (a `harness` row).
struct Harness(String);

async fn admin_call(
    kit: &WorkerKit,
    secret: &str,
    (post, path, body): (bool, &str, Option<Value>),
    deadline: Instant,
) -> Result<Value, Harness> {
    let url = format!("http://{}{path}", kit.station.admin);
    let req = if post {
        kit.admin.post(&url)
    } else {
        kit.admin.get(&url)
    };
    let mut req = req.header("authorization", format!("Bearer {secret}"));
    if let Some(b) = body {
        req = req
            .header("content-type", "application/json")
            .body(b.to_string());
    }
    let call = async {
        let resp = req.send().await.map_err(|e| format!("{path}: {e}"))?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|e| format!("{path}: {e}"))?;
        let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        if status == 200 {
            Ok(v)
        } else {
            Err(format!("{path}: status {status}: {v}"))
        }
    };
    tokio::time::timeout_at(deadline, call)
        .await
        .map_err(|_| Harness(format!("{path}: the test's deadline passed")))?
        .map_err(Harness)
}

/// What a trial left before classification.
struct Ran {
    harness: Option<String>,
    oracle: Option<Result<(), String>>,
    failure: Option<Failure>,
    scored: Option<bool>,
    verdict: Option<String>,
    timing: Option<crate::native_source::Timing>,
    foreign_seen: u64,
    owned_seen: u64,
    vanished: Vec<(String, u64, u64)>,
    owned: Vec<String>,
    cleanup: Cleanup,
}

/// Run one claimed trial on `kit`, every step inside `deadline`.
async fn run_trial(
    cfg: &SuiteConfig,
    kit: &mut WorkerKit,
    trial: &PlannedTrial,
    policy: Policy,
    log: &Mutations,
    (deadline, cleanup_by): (Instant, Instant),
    acks: Arc<Mutex<Vec<Ack>>>,
) -> Ran {
    let mut ran = Ran {
        harness: None,
        oracle: None,
        failure: None,
        scored: None,
        verdict: None,
        timing: None,
        foreign_seen: 0,
        owned_seen: 0,
        vanished: Vec::new(),
        owned: suite::owned(&trial.flow, trial.id),
        cleanup: Cleanup::NotRun {
            reason: "the trial ended before its cleanup".into(),
        },
    };
    let Some(task) = crate::tasks::find(&trial.flow) else {
        ran.harness = Some(format!("{} is not a fixture task", trial.flow));
        return ran;
    };
    let inputs = crate::provenance::FixtureInputs {
        seed: trial.fixture_seed.clone(),
        fixture_time: suite::FIXTURE_TIME.into(),
        seed_algorithm: crate::provenance::ALGORITHM.into(),
        projection_version: 1,
    };
    let secret = {
        use rand::Rng;
        crate::provenance::hex(&rand::rng().random::<[u8; 32]>())
    };
    let hook_acks = acks.clone();
    let installed = kit.station.install(TrialConfig {
        task: task.clone(),
        trial_id: format!("testexec-{}", trial.id),
        admin_secret: secret.clone(),
        inputs,
        ack: Some(Arc::new(move |a| {
            hook_acks.lock().unwrap_or_else(|e| e.into_inner()).push(a)
        })),
    });
    if let Err(e) = installed {
        ran.harness = Some(format!("fixture: {e}"));
        return ran;
    }
    let primary = suite::primary(trial.id);
    let inbox = suite::inbox(trial.id);
    let steps = async {
        let provision = json!({"identities": [{
            "identity": primary,
            "inbox": inbox,
            "smtp_identity": inbox,
        }]});
        admin_call(
            kit,
            &secret,
            (true, "/admin/provision", Some(provision)),
            deadline,
        )
        .await?;
        let prepared = admin_call(kit, &secret, (true, "/admin/prepare", None), deadline).await?;
        let list = prepared["deliveries"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if list.is_empty() || list.iter().any(|d| d["status"] != json!("delivered")) {
            return Err(Harness(format!("fixture delivery failed: {prepared}")));
        }
        Ok(())
    };
    let mut source = NativeSource::new(
        &kit.client,
        cfg.http,
        cfg.method,
        trial,
        (deadline, cfg.clock),
    )
    .beside_neighbours(cfg.coord.parallelism > 1);
    match steps.await {
        Err(Harness(e)) => ran.harness = Some(e),
        Ok(()) => {
            solve(
                kit,
                &secret,
                (trial, &task),
                &mut source,
                deadline,
                &mut ran,
            )
            .await
        }
    }
    // Cleanup runs whatever happened after the install, in the test's
    // reserved tail: delete-by-id recovers leftovers of a failed flow too.
    source.extend_deadline(cleanup_by);
    ran.cleanup = if Instant::now() < cleanup_by {
        source.cleanup(policy, log).await
    } else {
        Cleanup::NotRun {
            reason: "the test's deadline passed before its cleanup".into(),
        }
    };
    ran
}

/// The oracle's solution and the scorer's judgement of the snapshot.
async fn solve(
    kit: &WorkerKit,
    secret: &str,
    (trial, task): (&PlannedTrial, &crate::tasks::Task),
    source: &mut NativeSource<'_>,
    deadline: Instant,
    ran: &mut Ran,
) {
    let primary = suite::primary(trial.id);
    let app_url = kit.station.app_url();
    let left = deadline.saturating_duration_since(Instant::now());
    let env = crate::oracle::AgentEnv {
        task: trial.flow.clone(),
        app_url: app_url.clone(),
        mcp_url: String::new(),
        identity: primary.clone(),
        inbox: suite::inbox(trial.id),
        goal: String::new(),
        prompt_file: String::new(),
        deadline_ms: left.as_millis() as u64,
    };
    let app = match crate::oracle::App::confined(
        &app_url,
        &primary,
        left.max(Duration::from_millis(1)),
    ) {
        Ok(a) => a,
        Err(e) => {
            ran.harness = Some(format!("app client: {e}"));
            return;
        }
    };
    let solved = tokio::time::timeout_at(deadline, crate::oracle::solve_suite(&env, &app, source))
        .await
        .unwrap_or_else(|_| Err("timeout: the test's deadline passed in the solution".into()));
    ran.oracle = Some(solved);
    ran.failure = source.failure.clone();
    ran.timing = source.timing.clone();
    ran.foreign_seen = source.foreign_seen();
    ran.owned_seen = source.owned_seen();
    ran.vanished = source.vanished();
    // The app-state scorer judges the real snapshot (0055).
    let judged = async {
        admin_call(kit, secret, (true, "/admin/freeze", None), deadline).await?;
        let snap = admin_call(kit, secret, (false, "/admin/snapshot", None), deadline).await?;
        let snap: crate::state::Snapshot =
            serde_json::from_value(snap).map_err(|e| Harness(format!("snapshot: {e}")))?;
        crate::tasks::score(task, &snap).map_err(|e| Harness(format!("score: {e}")))
    };
    match judged.await {
        Ok(v) => {
            ran.scored = Some(v.outcome == crate::tasks::Outcome::Pass);
            ran.verdict = v.failure_class.map(|c| format!("{c:?}"));
        }
        // A solution that already failed is classified by its own typed
        // failure: a scoring step that then ran out of time is no harness.
        Err(Harness(e)) if matches!(ran.oracle, Some(Ok(()))) => ran.harness = Some(e),
        Err(Harness(e)) => ran.verdict = Some(format!("not judged: {e}")),
    }
}

/// A trial's one outcome, its cause and, for `interference` only, its
/// causal evidence (0132 §8.5). Missing mail alone is a `timeout`: only own
/// mail that was seen and then vanished while another test's mutation could
/// reach it is `interference`.
fn classify(
    r: &Ran,
    trial: u32,
    policy: Policy,
    log: &Mutations,
) -> (Outcome, Option<String>, Vec<String>) {
    if let Some(h) = &r.harness {
        return (Outcome::Harness, Some(h.clone()), Vec::new());
    }
    match &r.oracle {
        None => (
            Outcome::Harness,
            Some("the oracle did not run".into()),
            Vec::new(),
        ),
        Some(Err(e)) => match &r.failure {
            Some(Failure::Timeout(w)) => {
                let mut evidence = Vec::new();
                for (key, seen, missed) in &r.vanished {
                    for m in log.foreign_reaching(trial, &r.owned, *seen, *missed) {
                        evidence.push(format!(
                            "own message {key} seen at {seen} ns and missing at {missed} ns, while trial {} mutated {} ({}..{} ns)",
                            m.trial, m.scope, m.start_ns, m.end_ns
                        ));
                    }
                }
                if evidence.is_empty() {
                    (Outcome::Timeout, Some(format!("timeout: {w}")), Vec::new())
                } else {
                    (
                        Outcome::Interference,
                        Some(format!("own mail removed by another test's mutation: {w}")),
                        evidence,
                    )
                }
            }
            Some(f @ Failure::Identity(_)) => (Outcome::WrongMessage, Some(f.text()), Vec::new()),
            Some(f @ Failure::Operation(_)) => (Outcome::Harness, Some(f.text()), Vec::new()),
            None if e.starts_with("timeout:") => (Outcome::Timeout, Some(e.clone()), Vec::new()),
            None if e.starts_with("GET ") || e.starts_with("POST ") => {
                (Outcome::Harness, Some(format!("app: {e}")), Vec::new())
            }
            None => (
                Outcome::WrongMessage,
                Some(format!("oracle: {e}")),
                Vec::new(),
            ),
        },
        Some(Ok(())) => match r.scored {
            Some(true) if r.cleanup.proved(policy) => (Outcome::Pass, None, Vec::new()),
            Some(true) => (
                Outcome::Harness,
                Some(format!("scored pass, cleanup not proved: {:?}", r.cleanup)),
                Vec::new(),
            ),
            Some(false) => (
                Outcome::WrongMessage,
                Some(format!(
                    "scorer: {}",
                    r.verdict.clone().unwrap_or_else(|| "fail".into())
                )),
                Vec::new(),
            ),
            None => (
                Outcome::Harness,
                Some("the scorer did not judge".into()),
                Vec::new(),
            ),
        },
    }
}

fn row_of(
    cfg: &SuiteConfig,
    trial: &PlannedTrial,
    (worker, claim_ns, end_ns): (u32, u64, u64),
    ran: Ran,
    acks: &[Ack],
    (policy, log): (Policy, &Mutations),
) -> TrialRow {
    let (outcome, cause, evidence) = classify(&ran, trial.id, policy, log);
    let wait = ran.timing.as_ref().map(|t| {
        let ack = count_completing_ack(acks, t.count, cfg.clock);
        WaitTiming {
            call_ns: t.call_ns,
            return_ns: t.return_ns,
            count: t.count,
            ack_ns: ack.as_ref().ok().copied(),
            ack_to_return_ns: ack.as_ref().ok().map(|a| t.return_ns as i64 - *a as i64),
            ack_absent: ack.err(),
            cause: t.cause.clone(),
        }
    });
    let wait_absent = wait.is_none().then(|| {
        ran.failure
            .as_ref()
            .map(Failure::text)
            .or_else(|| ran.harness.clone())
            .unwrap_or_else(|| "the solution returned before its wait".into())
    });
    TrialRow {
        id: trial.id,
        flow: trial.flow.clone(),
        seed: trial.seed,
        worker,
        claim_ns,
        end_ns,
        outcome,
        cause,
        evidence,
        scored: ran.scored,
        foreign_seen: ran.foreign_seen,
        owned_seen: ran.owned_seen,
        wait,
        wait_absent,
        cleanup: ran.cleanup,
    }
}

/// A claimed trial that never came back: a timeout at its deadline, or a
/// harness row when its worker was lost.
fn lost_row(
    trial: &PlannedTrial,
    (worker, claim_ns): (u32, u64),
    end_ns: u64,
    why: &str,
    timeout: bool,
) -> TrialRow {
    TrialRow {
        id: trial.id,
        flow: trial.flow.clone(),
        seed: trial.seed,
        worker,
        claim_ns,
        end_ns: end_ns.max(claim_ns),
        outcome: if timeout {
            Outcome::Timeout
        } else {
            Outcome::Harness
        },
        cause: Some(why.into()),
        evidence: Vec::new(),
        scored: None,
        foreign_seen: 0,
        owned_seen: 0,
        wait: None,
        wait_absent: Some(why.into()),
        cleanup: Cleanup::NotRun { reason: why.into() },
    }
}

/// The suite's shared state between workers.
struct Board {
    next: AtomicUsize,
    claims: Mutex<Vec<Option<(u32, u64)>>>,
    rows: Mutex<Vec<Option<TrialRow>>>,
    cut: Mutex<Option<String>>,
    log: Mutations,
}

impl Board {
    fn cut_reason(&self) -> Option<String> {
        self.cut.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// A claim's bounds from `now`: its cancellation, its cleanup deadline and
/// its work deadline, each inside the earlier of the test deadline and the
/// suite cutoff (0132 §8.2).
fn trial_bounds(now: Instant, cfg: &SuiteConfig, cut_at: Instant) -> (Instant, Instant, Instant) {
    let bound = (now + cfg.test_deadline).min(cut_at);
    let back = |t: Instant, d: Duration| t.checked_sub(d).unwrap_or(now).max(now);
    let cancel_at = back(bound, CANCEL_MARGIN);
    let cleanup_by = back(cancel_at, SETTLE_GRACE);
    (cancel_at, cleanup_by, back(cleanup_by, CLEANUP_RESERVE))
}

/// One worker: claim the next index until the queue is empty, the suite
/// cutoff comes, or the suite is cut; one active trial at a time.
async fn worker(
    cfg: Arc<SuiteConfig>,
    board: Arc<Board>,
    mut kit: WorkerKit,
    w: u32,
    (cut_at, policy): (Instant, Policy),
) -> WorkerKit {
    loop {
        if board.cut_reason().is_some() {
            break;
        }
        let i = board.next.fetch_add(1, Ordering::SeqCst);
        // The first claim was admitted by `run_suite` before it opened the
        // window: a window once opened always holds a trial.
        if i >= cfg.plan.len() || (i > 0 && Instant::now() + CLAIM_FLOOR >= cut_at) {
            break;
        }
        let trial = &cfg.plan[i];
        // The claim and its started row are one step: whatever happens to
        // this worker, the trial is accounted for as started.
        let claim_ns = cfg.clock.now_ns();
        board.claims.lock().unwrap_or_else(|e| e.into_inner())[i] = Some((w, claim_ns));
        let (cancel_at, cleanup_by, work_by) = trial_bounds(Instant::now(), &cfg, cut_at);
        let acks: Arc<Mutex<Vec<Ack>>> = Arc::default();
        let ran = tokio::time::timeout_at(
            cancel_at,
            run_trial(
                &cfg,
                &mut kit,
                trial,
                policy,
                &board.log,
                (work_by, cleanup_by),
                acks.clone(),
            ),
        )
        .await;
        // The test ends here: the fixture's drain is the worker's, after it.
        let end_ns = cfg.clock.now_ns();
        let retired = kit.station.retire(DRAIN_BOUND).await;
        let acks = acks.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut row = match ran {
            Ok(ran) => row_of(
                &cfg,
                trial,
                (w, claim_ns, end_ns),
                ran,
                &acks,
                (policy, &board.log),
            ),
            Err(_) => lost_row(
                trial,
                (w, claim_ns),
                end_ns,
                "the trial did not return by its deadline",
                true,
            ),
        };
        if let Err(e) = retired {
            // A live handler could still act: no further trial may share
            // this fixture or the store with it (the suite is cut).
            row.outcome = Outcome::Harness;
            row.cause = Some(e.clone());
            row.evidence.clear();
            *board.cut.lock().unwrap_or_else(|p| p.into_inner()) = Some(format!("worker {w}: {e}"));
        }
        board.rows.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(row);
    }
    kit
}

/// Run the suite: the window opens before the first claim and closes after
/// the last trial ended; the client's CPU is read at both bookends; the one
/// deferred global clear (8 and 32 workers on a global-only target) runs
/// after the window, and never while a worker is unsettled. `kits` are the
/// workers' fixtures and clients, bound before this is called.
pub async fn run_suite<M: SuiteMeter>(
    cfg: SuiteConfig,
    kits: Vec<WorkerKit>,
    meter: &M,
) -> Result<SuiteRecord, String> {
    suite::check_disjoint(&cfg.plan)?;
    if kits.len() != cfg.coord.parallelism as usize {
        return Err(format!(
            "{} worker(s) bound for parallelism {}",
            kits.len(),
            cfg.coord.parallelism
        ));
    }
    if cfg.test_deadline <= CLAIM_FLOOR {
        return Err(format!(
            "a test deadline of {:?} leaves no work past the claim floor",
            cfg.test_deadline
        ));
    }
    let mut rec = SuiteRecord::new(&cfg.coord, cfg.seed, cfg.method);
    rec.plan = cfg.plan.clone();
    let policy = rec.policy;
    let work_end = cfg.work_end;
    let cutoff_reason = move |by_suite: Instant| {
        if by_suite <= work_end {
            "suite_deadline"
        } else {
            "chunk_deadline"
        }
    };
    // The first claim is admitted before the window opens: a suite that can
    // claim nothing never opens it, and every planned id is unstarted.
    let now = Instant::now();
    if cfg.plan.is_empty() || now + CLAIM_FLOOR >= (now + cfg.suite_deadline).min(cfg.work_end) {
        let reason = cutoff_reason(now + cfg.suite_deadline);
        rec.unstarted = cfg
            .plan
            .iter()
            .map(|t| Unstarted {
                id: t.id,
                reason: reason.into(),
            })
            .collect();
        return Ok(rec);
    }
    let cpu0 = rusage();
    rec.window_start = Some(meter.window("window_start").await?);
    let start = Instant::now();
    let by_suite = start + cfg.suite_deadline;
    let cut_at = by_suite.min(cfg.work_end);
    if start + CLAIM_FLOOR >= cut_at {
        // The holder's own round trip ate the admitted claim's margin: the
        // window is open with nothing to hold, which only it can explain.
        return Err("the suite window opened too late for its first claim".into());
    }
    let n = cfg.plan.len();
    let board = Arc::new(Board {
        next: AtomicUsize::new(0),
        claims: Mutex::new(vec![None; n]),
        rows: Mutex::new(vec![None; n]),
        cut: Mutex::new(None),
        log: Mutations::default(),
    });
    let cfg = Arc::new(cfg);
    let mut set = tokio::task::JoinSet::new();
    for (w, kit) in kits.into_iter().enumerate() {
        set.spawn(worker(
            cfg.clone(),
            board.clone(),
            kit,
            w as u32,
            (cut_at, policy),
        ));
    }
    // Every trial is cancelled inside its bound (at most the cutoff), and
    // its fixture drains within the drain bound; a worker still running
    // then is lost.
    let settle_by = cut_at + DRAIN_BOUND + Duration::from_secs(1);
    let mut kits = Vec::new();
    let mut lost = 0;
    loop {
        match tokio::time::timeout_at(settle_by, set.join_next()).await {
            Ok(Some(Ok(kit))) => kits.push(kit),
            Ok(Some(Err(_))) => lost += 1,
            Ok(None) => break,
            Err(_) => {
                lost += set.len();
                set.abort_all();
                while set.join_next().await.is_some() {}
                break;
            }
        }
    }
    let end_ns = cfg.clock.now_ns();
    let claims = board
        .claims
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut rows = std::mem::take(&mut *board.rows.lock().unwrap_or_else(|e| e.into_inner()));
    let cut = board.cut_reason();
    let reason = cut
        .clone()
        .map(|c| format!("suite_cut: {c}"))
        .unwrap_or_else(|| cutoff_reason(by_suite).into());
    for (i, t) in cfg.plan.iter().enumerate() {
        match (claims[i], rows[i].take()) {
            (_, Some(row)) => rec.rows.push(row),
            (Some(claim), None) => rec.rows.push(lost_row(
                t,
                claim,
                end_ns,
                "the worker did not settle",
                false,
            )),
            (None, None) => rec.unstarted.push(Unstarted {
                id: t.id,
                reason: reason.clone(),
            }),
        }
    }
    rec.first_claim_ns = rec.rows.iter().map(|r| r.claim_ns).min();
    rec.last_end_ns = rec.rows.iter().map(|r| r.end_ns).max();
    rec.window_end = Some(meter.window("window_end").await?);
    rec.client_cpu = cpu0.zip(rusage()).map(|(a, b)| ClientCpu {
        user_usec: b.user_usec.saturating_sub(a.user_usec),
        system_usec: b.system_usec.saturating_sub(a.system_usec),
    });
    for k in kits {
        k.station.stop().await;
    }
    if policy == Policy::GlobalClearAfterSuite && !rec.rows.is_empty() {
        let start_ns = cfg.clock.now_ns();
        let cleanup = if lost > 0 || cut.is_some() {
            Cleanup::Failed {
                reason: "not run: a worker did not settle, and the store-wide clear would reach its work".into(),
            }
        } else {
            final_clear(&cfg, cfg.work_end + FINAL_RESERVE).await
        };
        rec.final_cleanup = Some(FinalCleanup {
            start_ns,
            end_ns: cfg.clock.now_ns(),
            cleanup,
        });
    }
    Ok(rec)
}

/// Every planned test's mail at its own addresses, counted by a complete
/// exact listing of each.
async fn planned_mail(
    client: &Client,
    plan: &[PlannedTrial],
    deadline: Instant,
) -> Result<u64, String> {
    let mut n = 0;
    for t in plan {
        for a in suite::owned(&t.flow, t.id) {
            let scope = clients::Scope {
                inbox: a.split('@').next().unwrap_or_default().to_owned(),
                address: a,
            };
            let mut spend = clients::Spend::default();
            n += client
                .list(&scope, deadline, &mut spend)
                .await
                .map_err(|e| e.reason())?
                .len() as u64;
        }
    }
    Ok(n)
}

/// The suite's one global clear, with the planned tests' mail counted
/// before it and checked gone after it.
async fn final_clear(cfg: &SuiteConfig, deadline: Instant) -> Cleanup {
    let failed = |reason: String| Cleanup::Failed { reason };
    let Some(client) = Client::new(&cfg.coord.target, cfg.http) else {
        return failed("no client".into());
    };
    let before = match planned_mail(&client, &cfg.plan, deadline).await {
        Ok(n) => n,
        Err(e) => return failed(format!("recover: {e}")),
    };
    let any = clients::Scope {
        address: suite::primary(suite::FIRST_ID),
        inbox: suite::inbox(suite::FIRST_ID),
    };
    let mut spend = clients::Spend::default();
    if let Err(e) = client.clear(&any, deadline, &mut spend).await {
        return failed(format!("clear: {}", e.reason()));
    }
    match planned_mail(&client, &cfg.plan, deadline).await {
        Ok(0) => Cleanup::Done { removed: before },
        Ok(left) => failed(format!("{left} message(s) remain after the global clear")),
        Err(e) => failed(format!("verify: {e}")),
    }
}

/// What the chunk's flags and protocol fix before anything is spawned.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub flags: std::collections::BTreeMap<String, String>,
    pub coord: Coord,
    pub seed: u64,
    pub method: WaitMethod,
    pub protocol: Value,
}

/// The target's `wait-best` row of a protocol, as JSON.
pub fn wait_best(protocol: &Value, target: &str) -> Result<WaitMethod, String> {
    let row = protocol["waits"]
        .as_array()
        .and_then(|w| w.iter().find(|p| p["name"] == json!("wait-best")))
        .and_then(|p| p["observation"].as_array())
        .and_then(|o| o.iter().find(|r| r["target"] == json!(target)))
        .ok_or_else(|| format!("--protocol has no wait-best row for {target}"))?;
    match row["mode"].as_str() {
        Some("native") => Ok(WaitMethod::Native),
        Some("stream") => Ok(WaitMethod::Stream {
            backstop_ms: row["backstop_ms"]
                .as_u64()
                .ok_or("a stream row without its backstop")?,
        }),
        Some("poll") => Ok(WaitMethod::Poll {
            interval_ms: row["interval_ms"]
                .as_u64()
                .ok_or("a poll row without its interval")?,
        }),
        other => Err(format!("wait-best mode {other:?} for {target}")),
    }
}

/// Every refusal that needs no process: the flags, the coordinate, the
/// protocol's section, seed and wait row, the pinned executable's name and
/// presence, the output and the address plan.
pub fn preflight(args: &[String]) -> Result<Prepared, String> {
    let f = crate::apibench::flags(args)?;
    let coord = Coord::parse(&f["chunk"])?;
    if !matches!(f["mode"].as_str(), "smoke" | "collect") {
        return Err(format!("--mode {:?} is not smoke or collect", f["mode"]));
    }
    let protocol: Value = serde_json::from_slice(
        &std::fs::read(&f["protocol"]).map_err(|e| format!("--protocol: {e}"))?,
    )
    .map_err(|e| format!("--protocol: {e}"))?;
    if protocol["protocol_version"] != json!(4) {
        return Err("--protocol is not protocol 4".into());
    }
    let section: suite::Section = serde_json::from_value(protocol[suite::SECTION].clone())
        .map_err(|e| format!("--protocol has no testexec section: {e}"))?;
    if section != suite::Section::specified() {
        return Err("--protocol's testexec section is not 0132's".into());
    }
    let seed = protocol["corpus"]["seed"]
        .as_u64()
        .ok_or("--protocol has no corpus seed")?;
    let method = wait_best(&protocol, &coord.target)?;
    let (pinned, path) = f["bin"]
        .split_once('=')
        .ok_or("--bin must be TARGET=PATH")?;
    if pinned != coord.target {
        return Err(format!(
            "--bin names {pinned}, but the chunk is {}'s",
            coord.target
        ));
    }
    if !std::path::Path::new(path).is_file() {
        return Err(format!("--bin {path} is not a file"));
    }
    if std::path::Path::new(&f["output"]).exists() {
        return Err("--output already exists; runs are never overwritten or resumed".into());
    }
    suite::check_disjoint(&suite::plan(seed))?;
    Ok(Prepared {
        flags: f,
        coord,
        seed,
        method,
        protocol,
    })
}

/// The held target's control pipe, as the suite's window meter.
pub struct HolderMeter {
    pipe: tokio::sync::Mutex<(
        tokio::process::ChildStdin,
        tokio::io::BufReader<tokio::process::ChildStdout>,
    )>,
    clock: Clock,
}

impl HolderMeter {
    async fn command(&self, line: &Value) -> Result<(Value, u64, u64), String> {
        use tokio::io::AsyncWriteExt;
        let mut p = self.pipe.lock().await;
        let sent = self.clock.now_ns();
        let mut bytes = line.to_string().into_bytes();
        bytes.push(b'\n');
        p.0.write_all(&bytes)
            .await
            .map_err(|e| format!("control: {e}"))?;
        p.0.flush().await.map_err(|e| format!("control: {e}"))?;
        let reply = crate::apibench::holder_line(&mut p.1, Duration::from_secs(60))
            .await
            .map_err(|e| format!("control: {e}"))?
            .ok_or("control: the holder ended")?;
        let got = self.clock.now_ns();
        let v: Value = serde_json::from_str(&reply).map_err(|e| format!("control reply: {e}"))?;
        if v.get("ok").is_none() {
            return Err(format!("control refused: {v}"));
        }
        Ok((v, sent, got))
    }
}

impl SuiteMeter for HolderMeter {
    async fn window(&self, cmd: &str) -> Result<Trip, String> {
        let (v, sent_ns, received_ns) = self
            .command(&json!({"cmd": cmd, "id": SUITE_WINDOW}))
            .await?;
        let owner_t_ns = v["t_ns"]
            .as_u64()
            .ok_or_else(|| format!("control reply to {cmd} has no t_ns"))?;
        Ok(Trip {
            sent_ns,
            received_ns,
            owner_t_ns,
        })
    }
}

/// Run one test-execution chunk; the exit is the holder's (0 a clean suite,
/// 1 the target's measured losses, sealed; 2 an infrastructure, control or
/// teardown failure, unsealed). Every refusal before the holder exists
/// spawns nothing.
pub async fn testexec(args: &[String]) -> i32 {
    let started = std::time::Instant::now();
    let prepared = match preflight(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("swarmail-evals testexec: refused before launch: {e}");
            return 2;
        }
    };
    match held(prepared, started).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("swarmail-evals testexec: {e}");
            2
        }
    }
}

/// How long the holder has to print its receipt once its control input
/// closes, and then to exit.
const RECEIPT_BOUND: Duration = Duration::from_secs(120);

async fn held(p: Prepared, started: std::time::Instant) -> Result<i32, String> {
    let f = &p.flags;
    let ends = crate::apibench::mcp_chunk_end(started, &p.protocol)?;
    let work_end = ends.checked_sub(FINAL_RESERVE).unwrap_or(ends);
    let mut hold = tokio::process::Command::new(&f["driver"]);
    hold.arg("hold").args(["--control", "testexec-v1"]);
    for (k, v) in f {
        if k != "driver" {
            hold.arg(format!("--{k}")).arg(v);
        }
    }
    hold.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut holder = hold.spawn().map_err(|e| format!("hold: {e}"))?;
    let stdin = holder.stdin.take().ok_or("hold: no stdin")?;
    let stdout = tokio::io::BufReader::new(holder.stdout.take().ok_or("hold: no stdout")?);
    let clock = Clock(std::time::Instant::now());
    let meter = HolderMeter {
        pipe: tokio::sync::Mutex::new((stdin, stdout)),
        clock,
    };
    let target = p.coord.target.clone();
    let mut first_line = None;
    let mut leaf = None;
    let ours: Result<(), String> = async {
        let read = {
            let mut g = meter.pipe.lock().await;
            crate::apibench::holder_line(&mut g.1, RECEIPT_BOUND).await
        };
        let endpoints: Value = match read {
            Ok(Some(line)) => {
                serde_json::from_str(&line).map_err(|e| format!("hold endpoints: {e}"))?
            }
            Ok(None) => return Err("the holder ended before it announced".into()),
            Err(e) => return Err(format!("the holder did not announce: {e}")),
        };
        if endpoints.get("cleanup").is_some() {
            // It refused or its target did not come up: its first line is
            // already its receipt.
            first_line = Some(endpoints);
            return Ok(());
        }
        leaf = endpoints["leaf"].as_str().map(str::to_owned);
        let addr = |k: &str| -> Result<std::net::SocketAddr, String> {
            endpoints[k]
                .as_str()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| format!("hold: no {k} endpoint"))
        };
        let (smtp, http) = (addr("smtp")?, addr("http")?);
        let cpus: Vec<u32> = endpoints["client_cpus"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| c.as_u64().map(|c| c as u32))
                    .collect()
            })
            .unwrap_or_default();
        // Every worker thread on the client CPUs before any worker starts.
        pin_process(&cpus)?;
        let cfg = SuiteConfig {
            coord: p.coord.clone(),
            seed: p.seed,
            method: p.method,
            smtp,
            http,
            smtp_auth: smtp_auth(&target),
            plan: suite::plan(p.seed),
            clock,
            test_deadline: Duration::from_millis(suite::TEST_DEADLINE_MS),
            suite_deadline: Duration::from_millis(suite::SUITE_DEADLINE_MS),
            work_end,
        };
        // Each worker's fixture is bound before the suite clock starts.
        let mut kits = Vec::new();
        for _ in 0..p.coord.parallelism {
            kits.push(WorkerKit::bind(&cfg).await?);
        }
        let rec = run_suite(cfg, kits, &meter).await?;
        meter
            .command(&json!({"cmd": "record", "testexec": rec}))
            .await
            .map(|_| ())
    }
    .await;
    let (stdin, mut stdout) = meter.pipe.into_inner();
    drop(stdin);
    let receipt = match first_line {
        Some(v) => Some(v),
        None => {
            let mut found = None;
            for _ in 0..4 {
                match crate::apibench::holder_line(&mut stdout, RECEIPT_BOUND).await {
                    Ok(Some(line)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&line)
                            && v.get("cleanup").is_some()
                        {
                            found = Some(v);
                            break;
                        }
                    }
                    _ => break,
                }
            }
            found
        }
    };
    let status = match tokio::time::timeout(RECEIPT_BOUND, holder.wait()).await {
        Ok(Ok(s)) => s.code(),
        _ => {
            let _ = holder.kill().await;
            None
        }
    };
    let receipt = receipt
        .ok_or_else(|| "missing".to_owned())
        .and_then(|v| crate::apibench::validate_receipt(&v, &target, leaf.as_deref()));
    let (code, why) = crate::apibench::chunk_exit(status, receipt, ours);
    if let Some(e) = why {
        eprintln!("swarmail-evals testexec: {e}");
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_source::Mutation;

    fn ran(oracle: Result<(), String>, failure: Option<Failure>) -> Ran {
        Ran {
            harness: None,
            oracle: Some(oracle),
            failure,
            scored: Some(false),
            verdict: None,
            timing: None,
            foreign_seen: 0,
            owned_seen: 0,
            vanished: Vec::new(),
            owned: suite::owned("magic-login", 104),
            cleanup: Cleanup::Done { removed: 1 },
        }
    }

    #[test]
    fn missing_mail_alone_is_a_timeout_and_interference_needs_its_cause() {
        let log = Mutations::default();
        let to = Failure::Timeout("waited".into());
        let mut r = ran(Err("timeout: waited".into()), Some(to));
        let (o, c, e) = classify(&r, 104, Policy::DeleteById, &log);
        assert_eq!((o, e.is_empty()), (Outcome::Timeout, true), "{c:?}");
        // Own mail that vanished with no foreign mutation is still the
        // target's loss, never interference.
        r.vanished = vec![("mp-1".into(), 10, 20)];
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::Timeout
        );
        // A neighbour's delete of other ids does not reach this test.
        log.push(Mutation {
            trial: 105,
            start_ns: 12,
            end_ns: 15,
            scope: suite::primary(105),
        });
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::Timeout
        );
        // A store-wide clear by another test inside the interval does.
        log.push(Mutation {
            trial: 106,
            start_ns: 15,
            end_ns: 18,
            scope: "all".into(),
        });
        let (o, _, e) = classify(&r, 104, Policy::DeleteById, &log);
        assert_eq!(o, Outcome::Interference);
        assert_eq!(e.len(), 1);
        assert!(e[0].contains("trial 106 mutated all"), "{e:?}");
        // Its own mutation is never evidence.
        let own = Mutations::default();
        own.push(Mutation {
            trial: 104,
            start_ns: 15,
            end_ns: 18,
            scope: "all".into(),
        });
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &own).0,
            Outcome::Timeout
        );
    }

    #[test]
    fn every_class_has_its_cause() {
        let log = Mutations::default();
        let mut r = ran(Ok(()), None);
        r.scored = Some(true);
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log),
            (Outcome::Pass, None, Vec::new())
        );
        // A scored pass whose cleanup failed is not a pass.
        r.cleanup = Cleanup::Failed { reason: "x".into() };
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::Harness
        );
        r.scored = Some(false);
        r.verdict = Some("WrongLink".into());
        let (o, c, _) = classify(&r, 104, Policy::DeleteById, &log);
        assert_eq!(
            (o, c.as_deref()),
            (Outcome::WrongMessage, Some("scorer: WrongLink"))
        );
        let r = ran(Err("expected five confirmations, saw 6".into()), None);
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::WrongMessage
        );
        let r = ran(Err("GET http://x: refused".into()), None);
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::Harness
        );
        let r = ran(
            Err("x".into()),
            Some(Failure::Operation("scan: incomplete".into())),
        );
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::Harness
        );
        let r = ran(Err("x".into()), Some(Failure::Identity("listed".into())));
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::WrongMessage
        );
        let mut r = ran(Ok(()), None);
        r.harness = Some("fixture delivery failed".into());
        assert_eq!(
            classify(&r, 104, Policy::DeleteById, &log).0,
            Outcome::Harness
        );
    }

    #[test]
    fn the_completing_ack_is_the_count_th_acceptance_in_reply_order() {
        let clock = Clock(std::time::Instant::now());
        let at = |ms: u64| clock.0 + Duration::from_millis(ms);
        let ack = |seq, ms, delivered| Ack {
            seq,
            to: "a".into(),
            code: if delivered { 250 } else { 550 },
            delivered,
            at: at(ms),
        };
        // Out of order on the hook, a refusal among them.
        let acks = vec![
            ack(3, 30, true),
            ack(1, 10, true),
            ack(2, 20, false),
            ack(4, 40, true),
        ];
        assert_eq!(count_completing_ack(&acks, 1, clock), Ok(10_000_000));
        assert_eq!(count_completing_ack(&acks, 3, clock), Ok(40_000_000));
        let e = count_completing_ack(&acks, 4, clock).unwrap_err();
        assert_eq!(e, "3 of 4 required deliveries acknowledged");
    }
}
