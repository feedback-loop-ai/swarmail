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

/// One repetition of one cost profile of one target.
#[derive(Debug, Clone, Serialize)]
pub struct CostRep {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    /// The v2 reconciliation is exact: requested = accepted = visible,
    /// seen once, fidelity intact.
    pub exact: bool,
    /// Distinct accepted messages observed on the server.
    pub visible: u64,
    pub window_outcome: WindowOutcome,
    pub cpu_user_usec: u64,
    pub cpu_system_usec: u64,
    pub cpu_total_usec: u64,
    pub peak_memory_bytes: u64,
    pub mean_memory_bytes: f64,
    pub peak_anon_bytes: u64,
    pub mean_anon_bytes: f64,
    pub oom_kills: u64,
    pub nr_throttled: u64,
    pub throttled_usec: u64,
    pub affinity_held: bool,
    pub visible_per_cpu_second: Option<f64>,
    pub cpu_ms_per_1k_visible: Option<f64>,
    pub shortfalls: Vec<&'static str>,
}

/// One target and cost profile over its repetitions: the median with the
/// minimum and maximum beside it. A cell that is not at equal results keeps
/// its figures, is not ranked, and names its shortfalls.
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
    pub memory_mib: f64,
    pub anon_mib: f64,
    pub scratch_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdleCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetitions: u32,
    pub memory_mib: Option<Spread>,
    pub anon_mib: Option<Spread>,
    pub scratch_bytes: Option<Spread>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupRep {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub ready_ms: f64,
    pub cpu_ms_to_ready: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupCell {
    pub target: TargetKind,
    pub profile: String,
    pub repetitions: u32,
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

fn mean(values: impl Iterator<Item = u64>) -> f64 {
    let (sum, n) = values.fold((0u128, 0u64), |(s, n), v| (s + u128::from(v), n + 1));
    if n == 0 { 0.0 } else { sum as f64 / n as f64 }
}

/// Derive one chunk's figures. Samples are matched to a window by launch name
/// and bounds, so one chunk's rows are never read against another's clock.
pub fn figures(cells: &[ThroughputCell], res: &[ResourceRow]) -> Figures {
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
        if let Some(w) = window(c, WindowKind::Measure) {
            let s = within(w, w.start_ns);
            let user = w.cpu_end.user_usec.saturating_sub(w.cpu_start.user_usec);
            let system = w
                .cpu_end
                .system_usec
                .saturating_sub(w.cpu_start.system_usec);
            let total = user + system;
            let visible = c.reconciliation.visible_unique;
            let r = &c.reconciliation;
            let mut shortfalls = Vec::new();
            for (name, hit) in [
                ("refused", c.refused > 0),
                ("lost", r.missing_accepted > 0),
                ("fidelity", r.fidelity_mismatches > 0),
                ("oom", w.outcome == WindowOutcome::OomKilled),
                (
                    "timeout",
                    w.outcome == WindowOutcome::ChunkDeadline || c.unstarted > 0,
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
            out.cost.push(CostRep {
                target: c.target,
                profile: c.profile.clone(),
                repetition: c.repetition,
                exact: c.exact,
                visible,
                window_outcome: w.outcome,
                cpu_user_usec: user,
                cpu_system_usec: system,
                cpu_total_usec: total,
                peak_memory_bytes: s.iter().map(|x| x.memory_current).max().unwrap_or(0),
                mean_memory_bytes: mean(s.iter().map(|x| x.memory_current)),
                peak_anon_bytes: s.iter().map(|x| x.anon).max().unwrap_or(0),
                mean_anon_bytes: mean(s.iter().map(|x| x.anon)),
                oom_kills: w
                    .events_end
                    .oom_kill
                    .saturating_sub(w.events_start.oom_kill),
                nr_throttled: w
                    .cpu_end
                    .nr_throttled
                    .saturating_sub(w.cpu_start.nr_throttled),
                throttled_usec: w
                    .cpu_end
                    .throttled_usec
                    .saturating_sub(w.cpu_start.throttled_usec),
                affinity_held: w.affinity_held,
                // A zero divisor is null, never zero or infinite.
                visible_per_cpu_second: (total > 0).then(|| visible as f64 / (total as f64 / 1e6)),
                cpu_ms_per_1k_visible: (visible > 0)
                    .then(|| (total as f64 / 1e3) / (visible as f64 / 1e3)),
                shortfalls,
            });
        }
        if let Some(w) = window(c, WindowKind::IdleReady) {
            let tail = w.end_ns.saturating_sub(crate::runner::IDLE_TAIL_NS);
            let s = within(w, tail);
            out.idle.push(IdleRep {
                target: c.target,
                profile: c.profile.clone(),
                repetition: c.repetition,
                memory_mib: mean(s.iter().map(|x| x.memory_current)) / MIB,
                anon_mib: mean(s.iter().map(|x| x.anon)) / MIB,
                scratch_bytes: w.scratch_bytes,
            });
        }
        if let Some(w) = window(c, WindowKind::Ready) {
            let cpu = w.cpu_end.usage_usec.saturating_sub(w.cpu_start.usage_usec);
            out.startup.push(StartupRep {
                target: c.target,
                profile: c.profile.clone(),
                repetition: c.repetition,
                ready_ms: w.end_ns.saturating_sub(w.start_ns) as f64 / 1e6,
                cpu_ms_to_ready: cpu as f64 / 1e3,
            });
        }
    }
    out
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
                labels: if rs.iter().any(|r| !r.affinity_held) {
                    vec!["affinity_widened"]
                } else {
                    Vec::new()
                },
                visible_per_cpu_second: pick(&|r| r.visible_per_cpu_second),
                cpu_ms_per_1k_visible: pick(&|r| r.cpu_ms_per_1k_visible),
                peak_memory_mib: pick(&|r| Some(r.peak_memory_bytes as f64 / MIB)),
                peak_anon_mib: pick(&|r| Some(r.peak_anon_bytes as f64 / MIB)),
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
            memory_mib: spread(&rs.iter().map(|r| r.memory_mib).collect::<Vec<_>>()),
            anon_mib: spread(&rs.iter().map(|r| r.anon_mib).collect::<Vec<_>>()),
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
            ready_ms: spread(&rs.iter().map(|r| r.ready_ms).collect::<Vec<_>>()),
            cpu_ms_to_ready: spread(&rs.iter().map(|r| r.cpu_ms_to_ready).collect::<Vec<_>>()),
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
        let mut f = figures(&r.throughput, g);
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
