//! The derived report: pure arithmetic over admitted rows, so anyone can
//! recompute it from `samples.jsonl` and the protocol. Losses, timeouts,
//! failures and unstarted work are counted beside every rate.

use crate::protocol::{
    Backend, Claim, Protocol, Provenance, ResourceProfile, StorageProfile, TargetKind,
};
use crate::records::{
    Manifest, Phase, ResourceRow, Row, SampleRow, WindowKind, WindowOutcome, WindowRow,
};
use crate::runner::Lifecycle;
use crate::stats::{Quantiles, Reconciliation, Spread, quantiles, spread};
use serde::Serialize;
use std::collections::BTreeMap;

pub const REPORT_VERSION: u32 = 2;
/// Protocol 4's report: protocols 2 and 3 keep version 2 and their own
/// admission terms, so no collection already admitted stops admitting.
pub const REPORT_VERSION_V4: u32 = 3;

#[derive(Debug, Serialize)]
pub struct Report {
    pub report_version: u32,
    pub run_id: String,
    pub mode: String,
    /// Always `false`: a driver report is raw measurement. Only evidence
    /// admission (task T10, not built) could make one evidence, and a
    /// smoke report never.
    pub evidence: bool,
    pub protocol_id: String,
    pub protocol_sha256: String,
    /// What the measurement claims and the profiles it ran under, copied
    /// from the protocol so no report is read without its limits. Absent
    /// only for a version 1 draft, which can never be evidence.
    pub claim: Option<Claim>,
    pub storage_profile: Option<StorageProfile>,
    pub resource_profile: Option<ResourceProfile>,
    pub units: Units,
    /// What each row is, stated beside its cells: source kind, read path,
    /// search method and storage, so no cell is read without its label
    /// (proposed decision 0130 §3).
    pub labels: Vec<RowLabel>,
    /// Rows the protocol publishes as not starting: every profile's cell
    /// is blocked, with what it scheduled and no number.
    pub blocked: Vec<BlockedRow>,
    pub throughput: Vec<ThroughputCell>,
    pub waits: Vec<WaitCell>,
    pub launches: Vec<crate::targets::LaunchFacts>,
    pub cleanups: Vec<crate::targets::CleanupFacts>,
    /// Protocol 4 only: the cost section's figures (0132 §5).
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub v4: Option<V4Report>,
}

/// One row's labels. `source` separates the candidate, an official release
/// and a third-party image: the Alkemio row is never an official competitor.
#[derive(Debug, Serialize)]
pub struct RowLabel {
    pub target: TargetKind,
    pub source: &'static str,
    /// `http-json`, or `pop3` for GreenMail (a fresh session per scan).
    pub read_path: &'static str,
    /// `server` search, or a complete scan `client-side-filter`ed by token.
    pub search: &'static str,
    pub storage_backend: Option<Backend>,
    pub retention_cap: Option<u64>,
    /// The version the executable reports when it is not its tag's, derived
    /// from the closed `REPORTED_VERSIONS` constant.
    pub version: Option<String>,
    /// What the row runs on besides its own bytes: a pinned JRE or musl
    /// loader, the host's glibc (a source build, unpinned) or nothing.
    pub runtime: Option<&'static str>,
    /// What the artifact's digest is bound to when that is weaker than a
    /// publisher's own statement.
    pub integrity: Option<&'static str>,
    /// `Some` when the protocol publishes that the pinned executable does
    /// not start: the row carries this label and no number.
    pub does_not_start: Option<String>,
}

/// A published does-not-start row: it keeps its place in the full suite,
/// and each cell says what it would have been asked and that it was blocked.
#[derive(Debug, Serialize)]
pub struct BlockedRow {
    pub target: TargetKind,
    pub result: String,
    pub cells: Vec<BlockedCell>,
}

#[derive(Debug, Serialize)]
pub struct BlockedCell {
    pub profile: String,
    pub repetitions: u32,
    /// Messages one repetition of the profile schedules.
    pub requested: u64,
    pub outcome: &'static str,
}

pub fn blocked_rows(p: &Protocol) -> Vec<BlockedRow> {
    p.does_not_start
        .iter()
        .flatten()
        .map(|d| BlockedRow {
            target: d.target,
            result: format!("does not start (published result: {})", d.cite),
            cells: p
                .throughput
                .iter()
                .map(|t| BlockedCell {
                    profile: t.name.clone(),
                    repetitions: t.repetitions,
                    requested: u64::from(t.messages),
                    outcome: "blocked",
                })
                .chain(p.waits.iter().map(|w| BlockedCell {
                    profile: w.name.clone(),
                    repetitions: w.repetitions,
                    requested: u64::from(w.present + w.future + w.empty),
                    outcome: "blocked",
                }))
                .collect(),
        })
        .collect()
}

pub fn row_labels(p: &Protocol) -> Vec<RowLabel> {
    p.targets
        .iter()
        .map(|t| {
            let retention = p.storage.retention.iter().find(|r| r.target == *t);
            RowLabel {
                target: *t,
                source: match t.provenance() {
                    Provenance::Candidate => "candidate",
                    Provenance::Official(_) => "official release",
                    Provenance::ThirdPartyImage(_) => "third-party image (extracted binary)",
                    Provenance::SourceBuild(_) => "built from source (cgo; host glibc, unpinned)",
                },
                read_path: if *t == TargetKind::Greenmail {
                    "pop3"
                } else {
                    "http-json"
                },
                search: if crate::targets::has_server_search(*t) {
                    "server"
                } else {
                    "client-side-filter"
                },
                storage_backend: retention.and_then(|r| r.backend),
                retention_cap: retention.and_then(|r| r.max_messages),
                version: crate::protocol::REPORTED_VERSIONS
                    .iter()
                    .find(|(rt, _, _)| rt == t)
                    .map(|(_, tag, v)| format!("{v} reported / tag {tag}")),
                runtime: match (crate::protocol::runtime_rule(*t), t.provenance()) {
                    (Some(rule), _) if rule.image_layer => {
                        Some("musl loader (pinned, from the image's own layer)")
                    }
                    (Some(_), _) => Some("JRE (pinned)"),
                    (None, Provenance::SourceBuild(_)) => Some("host glibc (unpinned)"),
                    // smtp4dev ships self-contained .NET but resolves its
                    // native dependencies on the host: no runtime is pinned.
                    (None, _) if *t == TargetKind::Smtp4dev => {
                        Some("self-contained .NET on host libraries (unpinned)")
                    }
                    (None, _) => None,
                },
                integrity: crate::protocol::REGISTRY
                    .iter()
                    .any(|(rt, _)| rt == t)
                    .then_some(
                        "integrity against the registry's own checksum; authenticity not verified",
                    ),
                does_not_start: p
                    .does_not_start
                    .iter()
                    .flatten()
                    .find(|d| d.target == *t)
                    .map(|d| format!("does not start (published result: {})", d.cite)),
            }
        })
        .collect()
}

/// A report is one run of one protocol. Version 2 and version 3 are
/// separate runs with separate denominators (proposed decision 0130 §3), so
/// manifests of different protocol ids are never read as one table or one
/// statistic.
#[allow(dead_code)] // the guard for any reader of several runs; tests drive it
pub fn one_denominator(manifests: &[&Manifest]) -> Result<(), String> {
    let mut ids: Vec<&str> = manifests.iter().map(|m| m.protocol_id.as_str()).collect();
    ids.dedup();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > 1 {
        return Err(format!(
            "runs of protocols {} are separate denominators; no report merges them",
            ids.join(" and ")
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Units {
    pub time: &'static str,
    pub rates: &'static str,
    pub quantiles: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ThroughputCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    /// Messages the profile scheduled: `attempted + unstarted`.
    pub requested: u64,
    /// Rows the send deadline cut off before any connection was tried;
    /// never merged with a connect or terminator failure.
    pub unstarted: u64,
    /// Sessions the profile scheduled, and the retention cap this target
    /// ran under (`None` = none declared).
    pub sessions: u32,
    pub retention_cap: Option<u64>,
    /// Accepted by the server: the `250`s received.
    pub admitted: u64,
    /// Refused at a command (`rejected`) or never completed before the
    /// terminator (`failed_before_terminator`); ambiguous sends are
    /// neither admitted nor refused.
    pub refused: u64,
    pub reconciliation: Reconciliation,
    pub exact: bool,
    pub accepted_per_second: Option<f64>,
    pub verified_per_second: Option<f64>,
    /// `start - invoked` per started send: controller scheduling delay
    /// (including session open on fresh connections).
    pub scheduling_delay: Quantiles,
    /// `ack - start` per accepted send.
    pub send_latency: Quantiles,
}

#[derive(Debug, Serialize)]
pub struct WaitCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetitions: u32,
    /// `observed - invoked`, present mail.
    pub present_latency: Quantiles,
    /// `observed - send start`, future mail.
    pub future_latency: Quantiles,
    /// `observed - ack`, signed: negative when the observer saw the mail
    /// before the client received the final 250.
    pub future_offset_from_ack: Quantiles,
    pub outcomes: BTreeMap<String, BTreeMap<String, u64>>,
    pub requests: u64,
}

fn diff(a: Option<u64>, b: Option<u64>) -> Option<i64> {
    Some(a? as i64 - b? as i64)
}

pub fn report(p: &Protocol, m: &Manifest, rows: &[Row], life: &Lifecycle) -> Report {
    let qs = &p.statistics.quantiles;
    let mut cells: BTreeMap<(TargetKind, String, u32), Vec<&Row>> = BTreeMap::new();
    let mut wcells: BTreeMap<(TargetKind, String), Vec<&Row>> = BTreeMap::new();
    for r in rows {
        match r.phase {
            Phase::Send | Phase::Visible | Phase::Verified => cells
                .entry((r.target, r.profile.clone(), r.repetition))
                .or_default()
                .push(r),
            Phase::Present | Phase::Future | Phase::Empty => wcells
                .entry((r.target, r.profile.clone()))
                .or_default()
                .push(r),
        }
    }
    let throughput = cells
        .into_iter()
        .map(|((target, profile, repetition), rs)| {
            let sessions = p
                .throughput
                .iter()
                .find(|t| t.name == profile)
                .map_or(0, |t| t.sessions);
            let cap = p
                .storage
                .retention
                .iter()
                .find(|r| r.target == target)
                .and_then(|r| r.max_messages);
            throughput_cell(target, profile, repetition, (sessions, cap), &rs, qs)
        })
        .collect();
    let waits = wcells
        .into_iter()
        .map(|((target, profile), rs)| wait_cell(target, profile, &rs, qs))
        .collect();
    Report {
        report_version: REPORT_VERSION,
        run_id: m.run_id.clone(),
        mode: m.mode.clone(),
        evidence: false,
        protocol_id: m.protocol_id.clone(),
        protocol_sha256: m.protocol_sha256.clone(),
        claim: p.claim.clone(),
        storage_profile: p.storage.profile,
        resource_profile: p.resources.profile,
        units: Units {
            time: "nanoseconds of the controller's monotonic clock",
            rates: "messages per second of the stated window",
            quantiles: "nearest-rank over successful samples; censored counted separately",
        },
        labels: row_labels(p),
        blocked: blocked_rows(p),
        throughput,
        waits,
        launches: life.launches.clone(),
        cleanups: life.cleanups.clone(),
        v4: None,
    }
}

fn throughput_cell(
    target: TargetKind,
    profile: String,
    repetition: u32,
    (sessions, retention_cap): (u32, Option<u64>),
    rs: &[&Row],
    qs: &[u32],
) -> ThroughputCell {
    let mut rec = Reconciliation::default();
    let mut unstarted = 0;
    let (mut sched, mut lat) = (Vec::new(), Vec::new());
    for r in rs {
        match (r.phase, r.outcome.as_str()) {
            (Phase::Send, "unstarted") => unstarted += 1,
            (Phase::Send, o) => {
                rec.attempted += 1;
                match o {
                    "accepted" => {
                        rec.accepted += 1;
                        lat.extend(diff(r.ack_ns, r.start_ns));
                    }
                    "rejected" => rec.rejected += 1,
                    "failed_before_terminator" => rec.failed_before_terminator += 1,
                    _ => rec.ambiguous += 1,
                }
                sched.extend(diff(r.start_ns, r.invoked_ns));
            }
            (Phase::Visible, o) => {
                let n = r.occurrences.unwrap_or(0);
                match o {
                    "visible" => {
                        rec.visible_unique += 1;
                        rec.duplicates += n.saturating_sub(1);
                    }
                    _ => rec.unexpected_visible += n,
                }
                rec.fidelity_mismatches += u64::from(r.fidelity_ok == Some(false));
            }
            (Phase::Verified, _) => {
                rec.send_window_ns = diff(r.ack_ns, r.start_ns).unwrap_or(0).max(0) as u64;
                rec.verified_window_ns = diff(r.observed_ns, r.start_ns).unwrap_or(0).max(0) as u64;
            }
            _ => {}
        }
    }
    rec.missing_accepted = rec.accepted - rec.visible_unique.min(rec.accepted);
    ThroughputCell {
        target,
        profile,
        repetition,
        requested: rec.attempted + unstarted,
        unstarted,
        sessions,
        retention_cap,
        admitted: rec.accepted,
        refused: rec.rejected + rec.failed_before_terminator,
        exact: unstarted == 0 && rec.exact(),
        accepted_per_second: rec.accepted_per_second(),
        verified_per_second: rec.verified_per_second(),
        scheduling_delay: quantiles(sched, 0, qs),
        send_latency: quantiles(lat, 0, qs),
        reconciliation: rec,
    }
}

fn censored(outcome: &str) -> bool {
    matches!(outcome, "timeout" | "late")
}

fn wait_cell(target: TargetKind, profile: String, rs: &[&Row], qs: &[u32]) -> WaitCell {
    let (mut present, mut future, mut offset) = (Vec::new(), Vec::new(), Vec::new());
    let (mut present_censored, mut future_censored) = (0, 0);
    let mut outcomes: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut requests = 0;
    let mut reps = std::collections::BTreeSet::new();
    for r in rs {
        reps.insert(r.repetition);
        requests += r.requests.unwrap_or(0);
        let phase = format!("{:?}", r.phase).to_lowercase();
        *outcomes
            .entry(phase)
            .or_default()
            .entry(r.outcome.clone())
            .or_default() += 1;
        let observed = r.outcome == "observed";
        match r.phase {
            Phase::Present if observed => present.extend(diff(r.observed_ns, r.invoked_ns)),
            // A late observation is retained but censored for the
            // requested interval, like a timeout.
            Phase::Present => present_censored += usize::from(censored(&r.outcome)),
            Phase::Future if observed => {
                future.extend(diff(r.observed_ns, r.start_ns));
                offset.extend(diff(r.observed_ns, r.ack_ns));
            }
            Phase::Future => future_censored += usize::from(censored(&r.outcome)),
            _ => {}
        }
    }
    WaitCell {
        target,
        profile,
        repetitions: reps.len() as u32,
        present_latency: quantiles(present, present_censored, qs),
        future_latency: quantiles(future, future_censored, qs),
        future_offset_from_ack: quantiles(offset, future_censored, qs),
        outcomes,
        requests,
    }
}

/// Admit a sealed run against the protocol that claims to have produced
/// it (spec clarification Q6). Beyond [`crate::records::admit`]'s checks of
/// the directory itself, the plan is recomputed from the protocol, so a
/// manifest that declares a smaller denominator cannot admit; every launch
/// has a proved cleanup receipt in the report; and the report's cells are
/// the arithmetic of the rows, not a separate claim.
pub fn admit_run(
    dir: &std::path::Path,
    protocol_bytes: &[u8],
) -> Result<crate::records::Admitted, String> {
    let p = Protocol::parse(protocol_bytes).map_err(|r| r.to_string())?;
    p.validate().map_err(|r| r.to_string())?;
    if p.protocol_version == crate::protocol::PROTOCOL_VERSION_4 {
        return Err(
            "a protocol 4 run is admitted by admit-v4, chunk by chunk and as one run".into(),
        );
    }
    let admitted = crate::records::admit(dir)?;
    let m = &admitted.manifest;
    if m.protocol_sha256 != crate::targets::sha256_hex(protocol_bytes) {
        return Err("the manifest names a different protocol".into());
    }
    if m.planned != crate::runner::plan(&p) {
        return Err(
            "the manifest's plan is not the protocol's full plan: a smaller denominator never admits"
                .into(),
        );
    }
    let bytes = std::fs::read(dir.join("report.json")).map_err(|e| format!("report: {e}"))?;
    let on_disk: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("report: {e}"))?;
    if on_disk["report_version"] != REPORT_VERSION {
        return Err(format!(
            "the report is version {}, this driver admits version {REPORT_VERSION}: re-collect with the admitting driver",
            on_disk["report_version"]
        ));
    }
    let listed = |key: &str| on_disk[key].as_array().cloned().unwrap_or_default();
    let (launches, cleanups) = (listed("launches"), listed("cleanups"));
    let proved = |c: &serde_json::Value| {
        [
            "reaped",
            "group_gone",
            "listeners_closed",
            "scratch_removed",
        ]
        .iter()
        .all(|k| c[*k] == true)
    };
    if launches.is_empty() || cleanups.len() != launches.len() || !cleanups.iter().all(proved) {
        return Err(
            "every launch needs a proved cleanup receipt before the run can be admitted".into(),
        );
    }
    let life = Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let fresh = serde_json::to_value(report(&p, m, &admitted.rows, &life))
        .map_err(|e| format!("report: {e}"))?;
    // Protocol 3's report also carries the row labels and the blocked rows
    // the reader sees beside every number; both are pure functions of the
    // protocol, so a sealed report cannot drop a does-not-start row or edit a
    // label. Protocol 2's admission keeps its terms (the 2026-10-30
    // collection predates both keys).
    let keys: &[&str] = if p.protocol_version == crate::protocol::PROTOCOL_VERSION_3 {
        &["throughput", "waits", "blocked", "labels"]
    } else {
        &["throughput", "waits"]
    };
    for &key in keys {
        if let Some((at, report, rows)) = first_difference(key, &on_disk[key], &fresh[key]) {
            return Err(format!(
                "the report disagrees with the rows it was derived from at {at}: report {report}, rows {rows}"
            ));
        }
    }
    Ok(admitted)
}

/// The first place two JSON values differ, as a path with both sides.
fn first_difference(
    at: &str,
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Option<(String, serde_json::Value, serde_json::Value)> {
    use serde_json::Value;
    match (a, b) {
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .enumerate()
            .find_map(|(i, (x, y))| first_difference(&format!("{at}[{i}]"), x, y)),
        (Value::Object(x), Value::Object(y)) if x.len() == y.len() && x.keys().eq(y.keys()) => x
            .iter()
            .find_map(|(k, x)| first_difference(&format!("{at}.{k}"), x, &y[k])),
        _ => (a != b).then(|| (at.to_owned(), a.clone(), b.clone())),
    }
}

// ============================================================================
// Protocol 4: resources per visible mail, at equal results (0132 §5)
// ============================================================================

/// The figures protocol 4 adds to the report. Every one is derived from
/// message rows and resource rows, so anyone can recompute it.
#[derive(Debug, Serialize)]
pub struct V4Report {
    pub cost: CostSection,
    pub idle: Vec<IdleCell>,
    pub startup: Vec<StartupCell>,
    pub headline: Headline,
    /// Needs the API section's fill levels (story B): owed, never a number.
    pub mib_per_1k_retained: Owed,
}

#[derive(Debug, Serialize)]
pub struct Owed {
    pub status: &'static str,
    pub owed_by: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CostSection {
    pub repetitions: Vec<CostRep>,
    pub cells: Vec<CostCell>,
}

/// One repetition of one cost profile of one target. A figure that the
/// repetition has no observation for is `null`, never zero: a window that was
/// never opened has no counters, and nothing is derived from it.
#[derive(Debug, Clone, Serialize)]
pub struct CostRep {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    /// The v2 reconciliation is exact: requested = accepted = visible,
    /// seen once, and every visible message's fidelity was read and intact.
    pub exact: bool,
    /// Distinct accepted messages observed on the server; `null` when the
    /// `measure` window was never opened.
    pub visible: Option<u64>,
    pub window_outcome: WindowOutcome,
    pub cpu_user_usec: Option<u64>,
    pub cpu_system_usec: Option<u64>,
    pub cpu_total_usec: Option<u64>,
    pub peak_memory_bytes: Option<u64>,
    pub mean_memory_bytes: Option<f64>,
    pub peak_anon_bytes: Option<u64>,
    pub mean_anon_bytes: Option<f64>,
    pub oom_kills: Option<u64>,
    pub nr_throttled: Option<u64>,
    pub throttled_usec: Option<u64>,
    /// `null` when no task could be read at the window's end.
    pub affinity_held: Option<bool>,
    pub visible_per_cpu_second: Option<f64>,
    pub cpu_ms_per_1k_visible: Option<f64>,
    pub shortfalls: Vec<&'static str>,
}

/// One target and cost profile over its repetitions: the median with the
/// minimum and maximum beside it, over the repetitions that have a value
/// (`n` says how many). A cell that is not at equal results keeps its
/// figures, is not ranked, and names its shortfalls.
#[derive(Debug, Clone, Serialize)]
pub struct CostCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetitions: u32,
    pub ranked: bool,
    pub shortfalls: Vec<&'static str>,
    pub labels: Vec<&'static str>,
    pub visible_per_cpu_second: Option<Spread>,
    pub cpu_ms_per_1k_visible: Option<Spread>,
    pub peak_memory_mib: Option<Spread>,
    pub peak_anon_mib: Option<Spread>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdleRep {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub memory_mib: Option<f64>,
    pub anon_mib: Option<f64>,
    pub scratch_bytes: Option<u64>,
    /// Why this repetition has no value, from the window's outcome.
    pub shortfall: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdleCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetitions: u32,
    /// True only when every repetition has a value.
    pub ranked: bool,
    pub shortfalls: Vec<&'static str>,
    pub memory_mib: Option<Spread>,
    pub anon_mib: Option<Spread>,
    pub scratch_bytes: Option<Spread>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupRep {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub ready_ms: Option<f64>,
    pub cpu_ms_to_ready: Option<f64>,
    pub shortfall: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetitions: u32,
    pub ranked: bool,
    pub shortfalls: Vec<&'static str>,
    pub ready_ms: Option<Spread>,
    pub cpu_ms_to_ready: Option<Spread>,
}

/// The headline row: the cells of the protocol's headline profile, with the
/// idle and startup figures of the same profile.
#[derive(Debug, Serialize)]
pub struct Headline {
    pub profile: String,
    pub cost: Vec<CostCell>,
    pub idle: Vec<IdleCell>,
    pub startup: Vec<StartupCell>,
    /// The installed size of every launched target, copied from the run's
    /// `footprint.json` (0132 §4.5, §5.6). Only a whole run has one; a
    /// chunk's report carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<Vec<InstalledCell>>,
}

/// One launched target's installed size, as `footprint` published it.
#[derive(Debug, Clone, Serialize)]
pub struct InstalledCell {
    pub target: TargetKind,
    pub installed_bytes: u64,
    pub installed_files: u64,
    pub installed_basis: String,
}

const MIB: f64 = 1_048_576.0;

/// What a chunk's resource rows say about each of its cells, before the
/// cells over repetitions are formed.
#[derive(Debug, Default)]
pub struct Figures {
    pub cost: Vec<CostRep>,
    pub idle: Vec<IdleRep>,
    pub startup: Vec<StartupRep>,
}

/// The mean of the values, or `None` when there are none: no sample, no
/// mean.
fn mean(values: impl Iterator<Item = u64>) -> Option<f64> {
    let (sum, n) = values.fold((0u128, 0u64), |(s, n), v| (s + u128::from(v), n + 1));
    (n > 0).then(|| sum as f64 / n as f64)
}

/// Why a window that was not `measured` gives no figure, in the closed
/// shortfall vocabulary.
fn outcome_shortfall(o: WindowOutcome) -> Option<&'static str> {
    match o {
        WindowOutcome::Measured => None,
        WindowOutcome::OomKilled => Some("oom"),
        WindowOutcome::TargetExited => Some("target_exited"),
        WindowOutcome::ChunkDeadline => Some("timeout"),
    }
}

/// What one cell's message rows say beyond the reconciliation: the cuts the
/// chunk deadline made, and the fidelity reads that were not made.
#[derive(Default)]
struct RowFacts {
    /// A send row ended at a deadline (the profile's or the chunk's).
    deadline_rows: bool,
    /// The verification was ended by the chunk deadline: what was not looked
    /// for was not lost.
    verification_cut: bool,
    /// Visible rows whose fidelity was never read.
    unread: u64,
}

impl RowFacts {
    fn of<'a>(rows: impl Iterator<Item = &'a Row>) -> RowFacts {
        let mut f = RowFacts::default();
        for r in rows {
            let reason = r.reason.as_deref();
            match r.phase {
                Phase::Send => {
                    f.deadline_rows |= reason
                        .is_some_and(|x| x == "send deadline" || x.contains("chunk deadline"));
                }
                Phase::Verified => {
                    f.verification_cut |= reason == Some("chunk deadline");
                }
                Phase::Visible => {
                    f.unread += u64::from(r.token.is_some() && r.fidelity_ok.is_none());
                }
                _ => {}
            }
        }
        f
    }
}

/// Derive one chunk's figures. Samples are matched to a window by launch name
/// and bounds, so one chunk's rows are never read against another's clock.
///
/// No observation, no number: a window that was never opened has no counters
/// and gives `null`; the cost figures come from a `measure` window with
/// counters whatever its outcome (0132 §5.2 keeps them for an unranked cell),
/// idle memory and the scratch footprint from an `idle_ready` window that was
/// `measured`, and the startup figures from a `ready` window that was
/// `measured`.
pub fn figures(cells: &[ThroughputCell], rows: &[Row], res: &[ResourceRow]) -> Figures {
    let windows: Vec<&WindowRow> = res
        .iter()
        .filter_map(|r| match r {
            ResourceRow::Window(w) => Some(w),
            ResourceRow::Sample(_) => None,
        })
        .collect();
    let samples: Vec<&SampleRow> = res
        .iter()
        .filter_map(|r| match r {
            ResourceRow::Sample(s) => Some(s),
            ResourceRow::Window(_) => None,
        })
        .collect();
    let window = |c: &ThroughputCell, kind: WindowKind| {
        windows.iter().copied().find(|w| {
            w.target == c.target
                && w.profile == c.profile
                && w.repetition == c.repetition
                && w.window == kind
        })
    };
    let within = |w: &WindowRow, from: u64| -> Vec<&SampleRow> {
        samples
            .iter()
            .copied()
            .filter(|s| s.launch == w.launch && s.target == w.target)
            .filter(|s| s.t_ns >= from && s.t_ns <= w.end_ns)
            .collect()
    };
    let mut out = Figures::default();
    for c in cells {
        let facts = RowFacts::of(rows.iter().filter(|r| {
            r.target == c.target && r.profile == c.profile && r.repetition == c.repetition
        }));
        if let Some(w) = window(c, WindowKind::Measure) {
            out.cost
                .push(cost_rep(c, w, &within(w, w.start_ns), &facts));
        }
        if let Some(w) = window(c, WindowKind::IdleReady) {
            let measured = w.outcome == WindowOutcome::Measured;
            let tail = w.end_ns.saturating_sub(crate::runner::IDLE_TAIL_NS);
            let s = within(w, tail);
            out.idle.push(IdleRep {
                target: c.target,
                profile: c.profile.clone(),
                repetition: c.repetition,
                memory_mib: mean(s.iter().map(|x| x.memory_current))
                    .filter(|_| measured)
                    .map(|m| m / MIB),
                anon_mib: mean(s.iter().map(|x| x.anon))
                    .filter(|_| measured)
                    .map(|m| m / MIB),
                scratch_bytes: w.scratch_bytes.filter(|_| measured),
                shortfall: outcome_shortfall(w.outcome),
            });
        }
        if let Some(w) = window(c, WindowKind::Ready) {
            let measured = w.outcome == WindowOutcome::Measured;
            let cpu = w
                .counters()
                .filter(|_| measured)
                .map(|(a, b, ..)| b.usage_usec.saturating_sub(a.usage_usec));
            out.startup.push(StartupRep {
                target: c.target,
                profile: c.profile.clone(),
                repetition: c.repetition,
                ready_ms: measured.then(|| w.end_ns.saturating_sub(w.start_ns) as f64 / 1e6),
                cpu_ms_to_ready: cpu.map(|c| c as f64 / 1e3),
                shortfall: outcome_shortfall(w.outcome),
            });
        }
    }
    out
}

/// One repetition's cost figures from its `measure` window and the samples
/// inside it. Each is `None` when the window carries no counters (never
/// opened) or no sample fell inside it.
fn cost_rep(c: &ThroughputCell, w: &WindowRow, s: &[&SampleRow], facts: &RowFacts) -> CostRep {
    let counters = w.counters();
    let user = counters.map(|(a, b, ..)| b.user_usec.saturating_sub(a.user_usec));
    let system = counters.map(|(a, b, ..)| b.system_usec.saturating_sub(a.system_usec));
    let total = user.zip(system).map(|(u, y)| u + y);
    let visible = counters.map(|_| c.reconciliation.visible_unique);
    let r = &c.reconciliation;
    let mut shortfalls = Vec::new();
    for (name, hit) in [
        ("refused", c.refused > 0),
        ("lost", r.missing_accepted > 0 && !facts.verification_cut),
        ("fidelity", r.fidelity_mismatches > 0),
        ("oom", w.outcome == WindowOutcome::OomKilled),
        (
            "timeout",
            w.outcome == WindowOutcome::ChunkDeadline
                || facts.deadline_rows
                || facts.verification_cut,
        ),
        ("target_exited", w.outcome == WindowOutcome::TargetExited),
        ("duplicate", r.duplicates > 0),
        ("unexpected", r.unexpected_visible > 0),
        ("ambiguous", r.ambiguous > 0),
    ] {
        if hit {
            shortfalls.push(name);
        }
    }
    let seen = |f: fn(&&SampleRow) -> u64| counters.and(s.iter().map(f).max());
    let average = |f: fn(&&SampleRow) -> u64| counters.and(mean(s.iter().map(f)));
    CostRep {
        target: c.target,
        profile: c.profile.clone(),
        repetition: c.repetition,
        exact: c.exact && facts.unread == 0,
        visible,
        window_outcome: w.outcome,
        cpu_user_usec: user,
        cpu_system_usec: system,
        cpu_total_usec: total,
        peak_memory_bytes: seen(|x| x.memory_current),
        mean_memory_bytes: average(|x| x.memory_current),
        peak_anon_bytes: seen(|x| x.anon),
        mean_anon_bytes: average(|x| x.anon),
        oom_kills: counters.map(|(_, _, a, b)| b.oom_kill.saturating_sub(a.oom_kill)),
        nr_throttled: counters.map(|(a, b, ..)| b.nr_throttled.saturating_sub(a.nr_throttled)),
        throttled_usec: counters
            .map(|(a, b, ..)| b.throttled_usec.saturating_sub(a.throttled_usec)),
        affinity_held: w.affinity_held,
        // A zero divisor is null, never zero or infinite.
        visible_per_cpu_second: total
            .zip(visible)
            .filter(|(total, _)| *total > 0)
            .map(|(total, visible)| visible as f64 / (total as f64 / 1e6)),
        cpu_ms_per_1k_visible: total
            .zip(visible)
            .filter(|(_, visible)| *visible > 0)
            .map(|(total, visible)| (total as f64 / 1e3) / (visible as f64 / 1e3)),
        shortfalls,
    }
}

/// Group `items` by (target, profile), in the order of first appearance.
fn grouped<T>(
    items: &[T],
    key: impl Fn(&T) -> (TargetKind, String),
) -> Vec<((TargetKind, String), Vec<&T>)> {
    let mut out: Vec<((TargetKind, String), Vec<&T>)> = Vec::new();
    for it in items {
        let k = key(it);
        match out.iter_mut().find(|(g, _)| *g == k) {
            Some((_, v)) => v.push(it),
            None => out.push((k, vec![it])),
        }
    }
    out
}

/// The shortfall names the idle and startup cells may carry, in order.
const WINDOW_SHORTFALLS: [&str; 3] = ["oom", "timeout", "target_exited"];

fn named(shortfalls: impl Iterator<Item = Option<&'static str>>) -> Vec<&'static str> {
    let have: Vec<&'static str> = shortfalls.flatten().collect();
    WINDOW_SHORTFALLS
        .iter()
        .copied()
        .filter(|n| have.contains(n))
        .collect()
}

/// The cells over repetitions, ranked only at equal results (0132 §5.2).
pub fn cost_cells(reps: &[CostRep]) -> Vec<CostCell> {
    let order = [
        "refused",
        "lost",
        "fidelity",
        "oom",
        "timeout",
        "target_exited",
        "duplicate",
        "unexpected",
        "ambiguous",
    ];
    grouped(reps, |r| (r.target, r.profile.clone()))
        .into_iter()
        .map(|((target, profile), rs)| {
            let pick = |f: &dyn Fn(&CostRep) -> Option<f64>| {
                spread(&rs.iter().filter_map(|r| f(r)).collect::<Vec<_>>())
            };
            let shortfalls: Vec<&'static str> = order
                .iter()
                .copied()
                .filter(|n| rs.iter().any(|r| r.shortfalls.contains(n)))
                .collect();
            let ranked = rs
                .iter()
                .all(|r| r.exact && r.window_outcome == WindowOutcome::Measured);
            CostCell {
                target,
                profile,
                repetitions: rs.len() as u32,
                ranked,
                shortfalls,
                labels: if rs.iter().any(|r| r.affinity_held == Some(false)) {
                    vec!["affinity_widened"]
                } else {
                    Vec::new()
                },
                visible_per_cpu_second: pick(&|r| r.visible_per_cpu_second),
                cpu_ms_per_1k_visible: pick(&|r| r.cpu_ms_per_1k_visible),
                peak_memory_mib: pick(&|r| r.peak_memory_bytes.map(|b| b as f64 / MIB)),
                peak_anon_mib: pick(&|r| r.peak_anon_bytes.map(|b| b as f64 / MIB)),
            }
        })
        .collect()
}

pub fn idle_cells(reps: &[IdleRep]) -> Vec<IdleCell> {
    grouped(reps, |r| (r.target, r.profile.clone()))
        .into_iter()
        .map(|((target, profile), rs)| IdleCell {
            target,
            profile,
            repetitions: rs.len() as u32,
            // Every published value of a cell is needed to rank it.
            ranked: rs.iter().all(|r| {
                r.memory_mib.is_some() && r.anon_mib.is_some() && r.scratch_bytes.is_some()
            }),
            shortfalls: named(rs.iter().map(|r| r.shortfall)),
            memory_mib: spread(&rs.iter().filter_map(|r| r.memory_mib).collect::<Vec<_>>()),
            anon_mib: spread(&rs.iter().filter_map(|r| r.anon_mib).collect::<Vec<_>>()),
            scratch_bytes: spread(
                &rs.iter()
                    .filter_map(|r| r.scratch_bytes.map(|b| b as f64))
                    .collect::<Vec<_>>(),
            ),
        })
        .collect()
}

pub fn startup_cells(reps: &[StartupRep]) -> Vec<StartupCell> {
    grouped(reps, |r| (r.target, r.profile.clone()))
        .into_iter()
        .map(|((target, profile), rs)| StartupCell {
            target,
            profile,
            repetitions: rs.len() as u32,
            ranked: rs
                .iter()
                .all(|r| r.ready_ms.is_some() && r.cpu_ms_to_ready.is_some()),
            shortfalls: named(rs.iter().map(|r| r.shortfall)),
            ready_ms: spread(&rs.iter().filter_map(|r| r.ready_ms).collect::<Vec<_>>()),
            cpu_ms_to_ready: spread(
                &rs.iter()
                    .filter_map(|r| r.cpu_ms_to_ready)
                    .collect::<Vec<_>>(),
            ),
        })
        .collect()
}

/// Protocol 4's report: protocol 3's cells, and the cost section's figures
/// derived from each chunk's own resource rows (`groups`: one slice per
/// chunk, because each chunk's leaves and clock are its own).
pub fn report_v4(
    p: &Protocol,
    m: &Manifest,
    rows: &[Row],
    groups: &[&[ResourceRow]],
    life: &Lifecycle,
) -> Report {
    let mut r = report(p, m, rows, life);
    r.report_version = REPORT_VERSION_V4;
    let mut all = Figures::default();
    for g in groups {
        let mut f = figures(&r.throughput, rows, g);
        all.cost.append(&mut f.cost);
        all.idle.append(&mut f.idle);
        all.startup.append(&mut f.startup);
    }
    let cells = cost_cells(&all.cost);
    let idle = idle_cells(&all.idle);
    let startup = startup_cells(&all.startup);
    let lead = p
        .headline
        .as_ref()
        .map(|h| h.profile.clone())
        .unwrap_or_default();
    r.v4 = Some(V4Report {
        headline: Headline {
            cost: cells
                .iter()
                .filter(|c| c.profile == lead)
                .cloned()
                .collect(),
            idle: idle.iter().filter(|c| c.profile == lead).cloned().collect(),
            startup: startup
                .iter()
                .filter(|c| c.profile == lead)
                .cloned()
                .collect(),
            installed: None,
            profile: lead,
        },
        cost: CostSection {
            repetitions: all.cost,
            cells,
        },
        idle,
        startup,
        mib_per_1k_retained: Owed {
            status: "owed",
            owed_by: "story B: the API section's fill levels (0132 §6.4)",
        },
    });
    r
}

// ============================================================================
// Protocol 4: admission of a whole run (0132 §11.4)
// ============================================================================

/// The files `admit-v4 --seal` writes at the top of a run directory, each a
/// pure function of the chunks, so verifying is recomputing and comparing.
pub const SEALED_FILES: [&str; 6] = [
    "manifest.json",
    "host.json",
    "chunks.json",
    "launches.json",
    "report.json",
    "report-cost.json",
];
pub const ADMISSION_VERSION: u32 = 1;

/// One line of the wrapper's ledger: what it ran for one chunk and how that
/// ended. `exit` is the driver's own (0 or 1; 2 stops the run), after the
/// wrapper mapped a timeout or a kill to 2.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    pub coordinate: String,
    pub command: String,
    pub exit: i32,
    pub wall_ms: u64,
    pub load_before: String,
    pub load_after: String,
    /// The chunk's scope held nothing when the chunk ended.
    pub scope_clean: bool,
    pub driver: String,
}

/// What admission checks a run against.
pub struct Admission<'a> {
    pub protocol: &'a Protocol,
    pub protocol_bytes: &'a [u8],
    pub lock: &'a crate::protocol::SourceLock,
    pub lock_bytes: &'a [u8],
    /// Executables to recheck by digest from their files, when given.
    pub bins: &'a std::collections::BTreeMap<TargetKind, std::path::PathBuf>,
}

/// One chunk directory, admitted.
pub struct ChunkAdmitted {
    pub name: String,
    pub manifest: Manifest,
    pub rows: Vec<Row>,
    pub resources: Vec<ResourceRow>,
    pub report: serde_json::Value,
}

fn expected_limits() -> serde_json::Value {
    serde_json::json!({
        "memory_max": crate::resources::MEMORY_MAX,
        "memory_swap_max": crate::resources::MEMORY_SWAP_MAX,
        "memory_oom_group": crate::resources::MEMORY_OOM_GROUP,
        "cpu_max": crate::resources::CPU_MAX,
        "pids_max": crate::resources::PIDS_MAX,
    })
}

/// The launch the protocol and lock say a target is run with: its template
/// with the protocol's cap resolved, its environment, its checked-member
/// digests and its pinned executable digest.
type Expected = (Vec<String>, Vec<[String; 2]>, Vec<String>, Option<String>);

fn expected_launch(a: &Admission, t: TargetKind) -> Result<Expected, String> {
    let cap = a
        .protocol
        .storage
        .retention
        .iter()
        .find(|r| r.target == t)
        .and_then(|r| r.max_messages);
    if t == TargetKind::Swarmail {
        let template = crate::targets::swarmail_template(cap.unwrap_or(0));
        return Ok((template, Vec::new(), Vec::new(), None));
    }
    let pin = a
        .lock
        .pin(t)
        .ok_or_else(|| format!("{t}: the source lock pins no executable"))?;
    let runtime = pin
        .runtime
        .as_ref()
        .map(|_| std::path::PathBuf::from("/runtime"));
    let spec = crate::targets::LaunchSpec::from_pin(
        t,
        std::path::PathBuf::from("/executable"),
        runtime,
        pin,
        cap,
    )?;
    let mut members: Vec<String> = spec.extra.members.iter().map(|(_, s)| s.clone()).collect();
    members.sort();
    Ok((spec.template, spec.extra.env, members, spec.expected_sha256))
}

/// Each launch of a chunk, against the lock, the protocol and its own
/// cleanup receipt (0132 §11.4).
fn check_launch(
    a: &Admission,
    chunk: &str,
    (l, c): (&serde_json::Value, &serde_json::Value),
    n: usize,
    target_cpus: &[u32],
) -> Result<(), String> {
    let at = |what: &str| format!("chunk {chunk}: launch {n}: {what}");
    let target = l["target"]
        .as_str()
        .and_then(TargetKind::parse)
        .ok_or_else(|| at("names no known target"))?;
    if c["target"] != l["target"] {
        return Err(at("its cleanup receipt is for another target"));
    }
    if l["limits"] != expected_limits() {
        return Err(at(&format!(
            "the limits read back are {}, not the budget's {}",
            l["limits"],
            expected_limits()
        )));
    }
    let leaf = l["leaf"].as_str().unwrap_or_default();
    // The cleanup is the launch's by the leaf it names, not only by its
    // place in a list.
    match c["leaf"].as_str() {
        None => return Err(at("its cleanup receipt names no leaf")),
        Some(named) if named != leaf => {
            return Err(at(&format!(
                "its cleanup receipt names the leaf {named:?}, not the launch's {leaf:?}"
            )));
        }
        Some(_) => {}
    }
    let joined = &l["joined"];
    if leaf.is_empty()
        || joined["cgroup"].as_str() != Some(leaf)
        || joined["cpus_allowed"] != serde_json::json!(target_cpus)
    {
        return Err(at(&format!(
            "the join read-back is missing or is not the leaf {leaf:?} on CPUs {target_cpus:?}: {joined}"
        )));
    }
    if l["cpu_layout"]["target_cpus"] != serde_json::json!(target_cpus) {
        return Err(at("its target CPUs are not the run's pair"));
    }
    let (template, env, members, pinned) = expected_launch(a, target)?;
    let same = l["argv_template"] == serde_json::json!(template)
        && l["env_template"] == serde_json::json!(env);
    if !same {
        return Err(at(
            "the launch template is not the protocol's (a cap other than the protocol's, or another argument)",
        ));
    }
    let digest = l["executable_sha256"].as_str().unwrap_or_default();
    if pinned.as_deref().is_some_and(|want| digest != want) {
        return Err(at("its executable digest is not the lock's"));
    }
    if let Some(path) = a.bins.get(&target) {
        let have = std::fs::read(path)
            .map(|b| crate::targets::sha256_hex(&b))
            .map_err(|e| at(&format!("{}: {e}", path.display())))?;
        if have != digest {
            return Err(at(&format!(
                "the executable at {} is not the bytes that ran",
                path.display()
            )));
        }
    }
    let mut ran: Vec<String> = l["members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m[1].as_str().map(str::to_owned))
        .collect();
    ran.sort();
    if ran != members {
        return Err(at("its checked members are not the lock's"));
    }
    for k in [
        "reaped",
        "group_gone",
        "listeners_closed",
        "scratch_removed",
    ] {
        if c[k] != true {
            return Err(at(&format!("cleanup {k} is not proved")));
        }
    }
    if c["cgroup_removed"] != true {
        return Err(at(&format!(
            "cgroup_removed is {}, not true: the leaf was not proved empty and removed",
            c["cgroup_removed"]
        )));
    }
    Ok(())
}

/// The keys of a chunk or run report that are recomputed from rows.
const DERIVED: [&str; 9] = [
    "throughput",
    "waits",
    "blocked",
    "labels",
    "cost",
    "idle",
    "startup",
    "headline",
    "mib_per_1k_retained",
];

fn no_life() -> Lifecycle {
    Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    }
}

/// Admit one sealed chunk directory: its rows, its plan, its receipts and its
/// figures. The run-level checks (one run id, one host, a clean commit) are
/// [`admit_v4`]'s.
pub fn admit_chunk(
    dir: &std::path::Path,
    chunk: &crate::runner::Chunk,
    a: &Admission,
) -> Result<ChunkAdmitted, String> {
    let name = chunk.name();
    let admitted = crate::records::admit(dir).map_err(|e| format!("chunk {name}: {e}"))?;
    let m = &admitted.manifest;
    let refuse = |why: &str| Err(format!("chunk {name}: {why}"));
    if m.chunk.as_deref() != Some(name.as_str()) {
        return refuse("its manifest names another chunk");
    }
    if m.protocol_sha256 != crate::targets::sha256_hex(a.protocol_bytes) {
        return refuse("its manifest names a different protocol");
    }
    if m.sources_sha256 != crate::targets::sha256_hex(a.lock_bytes) {
        return refuse("its manifest names a different source lock");
    }
    let (planned, windows) = crate::runner::plan_chunk(a.protocol, chunk);
    if m.planned != planned || m.planned_windows.as_ref() != Some(&windows) {
        return refuse(
            "its plan is not the protocol's plan for this chunk: a smaller denominator never admits",
        );
    }
    let Some(host) = &m.host else {
        return refuse("its manifest states no host");
    };
    let bytes =
        std::fs::read(dir.join("report.json")).map_err(|e| format!("chunk {name}: report: {e}"))?;
    let on_disk: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("chunk {name}: report: {e}"))?;
    if on_disk["report_version"] != REPORT_VERSION_V4 {
        return refuse(&format!(
            "its report is version {}, this driver admits version {REPORT_VERSION_V4}",
            on_disk["report_version"]
        ));
    }
    let listed = |key: &str| on_disk[key].as_array().cloned().unwrap_or_default();
    let (launches, cleanups) = (listed("launches"), listed("cleanups"));
    // A chunk whose instant came before it could launch anything has no
    // launch and no cleanup receipt, and is admitted: it is sealed, and every
    // one of its rows says why it was not measured (V16).
    if launches.len() != cleanups.len() {
        return refuse(&format!(
            "launch and cleanup receipts must pair, one proved cleanup for each launch: {} launch receipts, {} cleanup receipts",
            launches.len(),
            cleanups.len()
        ));
    }
    for (n, pair) in launches.iter().zip(&cleanups).enumerate() {
        if pair.0["target"].as_str() != Some(chunk.target().name()) {
            return refuse(&format!("launch {n} is not of the chunk's target"));
        }
        check_launch(a, &name, pair, n, &host.cpu_layout.target_cpus)?;
    }
    let mut named = std::collections::BTreeSet::new();
    for c in &cleanups {
        let leaf = c["leaf"].as_str().unwrap_or_default();
        if !named.insert(leaf) {
            return refuse(&format!(
                "two cleanup receipts name the leaf {leaf:?}: one proved cleanup for each launch"
            ));
        }
    }
    // Every launch a window or a sample names has its receipt here, so its
    // limits, join, cap and removal were checked above (V20).
    let receipted: std::collections::BTreeSet<&str> =
        launches.iter().filter_map(|l| l["leaf"].as_str()).collect();
    for r in &admitted.resources {
        let (launch, what) = match r {
            crate::records::ResourceRow::Sample(s) => (s.launch.as_str(), "a sample"),
            crate::records::ResourceRow::Window(w) => (w.launch.as_str(), "a window"),
        };
        if launch != "none" && !receipted.contains(launch) {
            return refuse(&format!(
                "{what} names the launch {launch:?}, which has no launch receipt"
            ));
        }
    }
    let groups = [&admitted.resources[..]];
    let fresh = serde_json::to_value(report_v4(
        a.protocol,
        m,
        &admitted.rows,
        &groups,
        &no_life(),
    ))
    .map_err(|e| format!("chunk {name}: report: {e}"))?;
    for key in DERIVED {
        if let Some((at, rep, rows)) = first_difference(key, &on_disk[key], &fresh[key]) {
            return Err(format!(
                "chunk {name}: the report disagrees with the rows it was derived from at {at}: report {rep}, rows {rows}"
            ));
        }
    }
    Ok(ChunkAdmitted {
        name,
        manifest: admitted.manifest,
        rows: admitted.rows,
        resources: admitted.resources,
        report: on_disk,
    })
}

type Artifacts = std::collections::BTreeMap<String, (String, u64)>;

fn top_files(dir: &std::path::Path) -> Result<Artifacts, String> {
    let mut out = Artifacts::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read run dir: {e}"))? {
        let entry = entry.map_err(|e| format!("read run dir: {e}"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 name in the run directory".to_owned())?;
        let meta = std::fs::symlink_metadata(entry.path()).map_err(|e| format!("{name}: {e}"))?;
        if name == "index.json" || !meta.is_file() {
            continue;
        }
        let bytes = std::fs::read(entry.path()).map_err(|e| format!("{name}: {e}"))?;
        out.insert(
            name,
            (crate::targets::sha256_hex(&bytes), bytes.len() as u64),
        );
    }
    Ok(out)
}

/// The result of admitting a run.
#[derive(Debug)]
pub struct RunAdmitted {
    pub run_id: String,
    pub chunks: usize,
    /// The run-level files, as recomputed from the chunks.
    pub files: std::collections::BTreeMap<String, serde_json::Value>,
}

fn names_in(dir: &std::path::Path, ext: Option<&str>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let name = entry
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 name".to_owned())?;
        out.push(match ext {
            Some(x) => name.strip_suffix(x).unwrap_or(&name).to_owned(),
            None => name,
        });
    }
    out.sort();
    Ok(out)
}

fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The installed size of each launched target, read from the run's
/// `footprint.json`: the file must be the run's protocol's, and every
/// launched target must have exactly one row with a positive size. A hole is
/// a refusal naming the target, never a `null` in the headline.
fn installed_cells(fp: &serde_json::Value, p: &Protocol) -> Result<Vec<InstalledCell>, String> {
    if fp["protocol_id"].as_str() != Some(p.id.as_str()) {
        return Err(format!(
            "footprint.json is for protocol {}, not {}",
            fp["protocol_id"], p.id
        ));
    }
    let rows = fp["targets"]
        .as_array()
        .ok_or("footprint.json has no list of targets")?;
    let mut out = Vec::new();
    for t in p.launch_order(0) {
        let mine: Vec<&serde_json::Value> =
            rows.iter().filter(|r| r["target"] == t.name()).collect();
        let [row] = mine.as_slice() else {
            return Err(format!(
                "footprint.json has {} rows for {t}: a launched target has exactly one",
                mine.len()
            ));
        };
        let positive = |key: &str| {
            row[key].as_u64().filter(|n| *n > 0).ok_or_else(|| {
                format!(
                    "footprint.json: {t}'s {key} is {}, not a positive integer",
                    row[key]
                )
            })
        };
        out.push(InstalledCell {
            target: t,
            installed_bytes: positive("installed_bytes")?,
            installed_files: positive("installed_files")?,
            installed_basis: row["installed_basis"]
                .as_str()
                .ok_or_else(|| format!("footprint.json: {t} states no installed_basis"))?
                .to_owned(),
        });
    }
    Ok(out)
}

/// Admit a whole v4 run directory: recompute the plan from the protocol,
/// require exactly one admitted chunk and one ledger entry per coordinate,
/// require one run, one host and one clean candidate, and recompute every
/// run-level file from the chunks. With `seal`, write them as new files and
/// then the run's index, refusing if any already exists; without it, compare
/// each with its recomputation and name the first path that differs.
pub fn admit_v4(dir: &std::path::Path, a: &Admission, seal: bool) -> Result<RunAdmitted, String> {
    if a.protocol.protocol_version != crate::protocol::PROTOCOL_VERSION_4 {
        return Err(format!(
            "admit-v4 needs protocol_version 4, not {}",
            a.protocol.protocol_version
        ));
    }
    // Before anything is indexed: a protocol with no chunk has nothing to
    // admit, and a run of nothing is not a run.
    if crate::runner::plan_v4(a.protocol).is_empty() {
        return Err("the protocol's chunk plan is empty: there is no chunk to admit".into());
    }
    a.protocol.validate().map_err(|r| r.to_string())?;
    if seal {
        for f in SEALED_FILES.iter().chain(&["index.json"]) {
            if dir.join(f).exists() {
                return Err(format!(
                    "{f} already exists: the run is sealed, and admit-v4 --seal never overwrites"
                ));
            }
        }
    }
    let plan = crate::runner::plan_v4(a.protocol);
    let mut want: Vec<String> = plan.iter().map(crate::runner::Chunk::name).collect();
    want.sort();
    let have = names_in(&dir.join("chunks"), None)?;
    if let Some(n) = want.iter().find(|n| !have.contains(n)) {
        return Err(format!(
            "chunk {n} is missing: every planned coordinate needs one sealed chunk"
        ));
    }
    if let Some(n) = have.iter().find(|n| !want.contains(n)) {
        return Err(format!("chunk {n} is not in the protocol's plan"));
    }
    let ledger_names = names_in(&dir.join("ledger"), Some(".json"))?;
    if let Some(n) = ledger_names.iter().find(|n| !want.contains(n)) {
        return Err(format!("ledger entry {n} is not in the protocol's plan"));
    }
    let mut admitted = Vec::new();
    let mut ledger = Vec::new();
    for c in &plan {
        let name = c.name();
        let raw = std::fs::read(dir.join("ledger").join(format!("{name}.json")))
            .map_err(|_| format!("chunk {name}: no ledger entry"))?;
        let entry: LedgerEntry =
            serde_json::from_slice(&raw).map_err(|e| format!("chunk {name}: ledger entry: {e}"))?;
        if entry.coordinate != name {
            return Err(format!(
                "chunk {name}: its ledger entry is for {}",
                entry.coordinate
            ));
        }
        if !matches!(entry.exit, 0 | 1) {
            return Err(format!(
                "chunk {name}: ledger exit {}: the run is incomplete and is never admitted",
                entry.exit
            ));
        }
        if !entry.scope_clean {
            return Err(format!(
                "chunk {name}: its scope was not clean when the chunk ended"
            ));
        }
        ledger.push(entry);
        admitted.push(admit_chunk(&dir.join("chunks").join(&name), c, a)?);
    }
    let first = admitted[0].manifest.clone();
    let mut digests: std::collections::BTreeMap<String, (String, String)> =
        std::collections::BTreeMap::new();
    for c in &admitted {
        let m = &c.manifest;
        // Every chunk, not only the first, ran on a clean candidate.
        if m.candidate_dirty != Some(false) {
            return Err(format!(
                "chunk {}: its tree was dirty or not stated clean: a v4 run is never admitted from a dirty candidate",
                c.name
            ));
        }
        // One executable per target across the whole run.
        for l in c.report["launches"].as_array().into_iter().flatten() {
            let (t, d) = (
                l["target"].as_str().unwrap_or_default().to_owned(),
                l["executable_sha256"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            );
            if let Some((was, earlier)) = digests.get(&t)
                && *was != d
            {
                return Err(format!(
                    "chunk {}: {t}'s executable digest differs from the one chunk {earlier} ran: one executable per target across the run",
                    c.name
                ));
            }
            digests.entry(t).or_insert((d, c.name.clone()));
        }
        let same = [
            ("the run id", m.run_id == first.run_id),
            ("the mode", m.mode == first.mode),
            (
                "the candidate commit",
                m.candidate_commit == first.candidate_commit,
            ),
            (
                "the host facts (kernel, CPUs and layout)",
                m.host == first.host,
            ),
        ];
        if let Some((what, _)) = same.iter().find(|(_, ok)| !ok) {
            return Err(format!(
                "chunk {}: {what} differs from the first chunk's",
                c.name
            ));
        }
    }
    if first.candidate_commit.is_none() || first.candidate_dirty != Some(false) {
        return Err("the candidate commit is missing or its tree was dirty: a v4 run is never admitted from either".into());
    }
    let footprint = std::fs::read(dir.join("footprint.json")).map_err(|_| {
        "footprint.json is missing: collect-v4 writes it before the first chunk".to_owned()
    })?;
    let footprint = serde_json::from_slice::<serde_json::Value>(&footprint)
        .map_err(|e| format!("footprint.json: {e}"))?;
    let installed = installed_cells(&footprint, a.protocol)?;
    // The run-level files, recomputed from the chunks alone.
    let rows: Vec<Row> = admitted.iter().flat_map(|c| c.rows.clone()).collect();
    let groups: Vec<&[ResourceRow]> = admitted.iter().map(|c| &c.resources[..]).collect();
    let mut run_manifest = first.clone();
    run_manifest.chunk = None;
    let mut whole = report_v4(a.protocol, &run_manifest, &rows, &groups, &no_life());
    if let Some(v4) = whole.v4.as_mut() {
        v4.headline.installed = Some(installed);
    }
    let run_report = serde_json::to_value(whole).map_err(|e| format!("report: {e}"))?;
    let pick = |keys: &[&str]| {
        let mut o = serde_json::Map::new();
        for k in keys {
            o.insert((*k).to_owned(), run_report[*k].clone());
        }
        serde_json::Value::Object(o)
    };
    let mut files = std::collections::BTreeMap::new();
    files.insert(
        "manifest.json".to_owned(),
        serde_json::json!({
            "admission_version": ADMISSION_VERSION,
            "run_id": first.run_id,
            "mode": first.mode,
            "protocol_id": first.protocol_id,
            "protocol_sha256": first.protocol_sha256,
            "sources_sha256": first.sources_sha256,
            "candidate_commit": first.candidate_commit,
            "candidate_dirty": false,
            // A draft run is labelled a draft in its own bytes.
            "protocol_status": a.protocol.status,
            "chunks": plan.iter().map(crate::runner::Chunk::name).collect::<Vec<_>>(),
        }),
    );
    files.insert(
        "host.json".to_owned(),
        serde_json::to_value(&first.host).map_err(|e| e.to_string())?,
    );
    files.insert(
        "chunks.json".to_owned(),
        serde_json::to_value(&ledger).map_err(|e| e.to_string())?,
    );
    let per_chunk = admitted.iter().map(|c| {
        serde_json::json!({
            "chunk": c.name,
            "launches": c.report["launches"],
            "cleanups": c.report["cleanups"],
        })
    });
    files.insert(
        "launches.json".to_owned(),
        serde_json::Value::Array(per_chunk.collect()),
    );
    files.insert("report.json".to_owned(), run_report.clone());
    files.insert(
        "report-cost.json".to_owned(),
        pick(&[
            "report_version",
            "run_id",
            "protocol_id",
            "protocol_sha256",
            "cost",
            "idle",
            "startup",
            "headline",
            "mib_per_1k_retained",
            "blocked",
        ]),
    );
    let out = RunAdmitted {
        run_id: first.run_id.clone(),
        chunks: admitted.len(),
        files,
    };
    if seal {
        for (name, value) in &out.files {
            let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            write_new(&dir.join(name), &bytes)?;
        }
        let index = crate::records::Index {
            index_version: crate::records::INDEX_VERSION,
            run_id: out.run_id.clone(),
            artifacts: top_files(dir)?,
        };
        let mut bytes = serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        write_new(&dir.join("index.json"), &bytes)?;
        return Ok(out);
    }
    for (name, value) in &out.files {
        let on_disk: serde_json::Value = std::fs::read(dir.join(name))
            .map_err(|_| format!("{name} is missing: run admit-v4 --seal first"))
            .and_then(|b| serde_json::from_slice(&b).map_err(|e| format!("{name}: {e}")))?;
        if let Some((at, found, derived)) = first_difference(name, &on_disk, value) {
            return Err(format!(
                "{name} disagrees with the chunks it was derived from at {at}: file {found}, chunks {derived}"
            ));
        }
    }
    let raw = std::fs::read(dir.join("index.json"))
        .map_err(|_| "index.json is missing: run admit-v4 --seal first".to_owned())?;
    let index: crate::records::Index =
        serde_json::from_slice(&raw).map_err(|e| format!("index: {e}"))?;
    if index.run_id != out.run_id || index.artifacts != top_files(dir)? {
        return Err(
            "index does not match the files present (missing, extra or altered artifact)".into(),
        );
    }
    Ok(out)
}
