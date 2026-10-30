//! The derived report: pure arithmetic over admitted rows, so anyone can
//! recompute it from `samples.jsonl` and the protocol. Losses, timeouts,
//! failures and unstarted work are counted beside every rate.

use crate::protocol::{Claim, Protocol, ResourceProfile, StorageProfile, TargetKind};
use crate::records::{Manifest, Phase, Row};
use crate::runner::Lifecycle;
use crate::stats::{Quantiles, Reconciliation, quantiles};
use serde::Serialize;
use std::collections::BTreeMap;

pub const REPORT_VERSION: u32 = 2;

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
    pub throughput: Vec<ThroughputCell>,
    pub waits: Vec<WaitCell>,
    pub launches: Vec<crate::targets::LaunchFacts>,
    pub cleanups: Vec<crate::targets::CleanupFacts>,
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
        throughput,
        waits,
        launches: life.launches.clone(),
        cleanups: life.cleanups.clone(),
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
    for key in ["throughput", "waits"] {
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
