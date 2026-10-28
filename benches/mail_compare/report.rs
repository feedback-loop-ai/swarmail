//! The derived report: pure arithmetic over admitted rows, so anyone can
//! recompute it from `samples.jsonl` and the protocol. Losses, timeouts,
//! failures and unstarted work are counted beside every rate.

use crate::protocol::{Protocol, TargetKind};
use crate::records::{Manifest, Phase, Row};
use crate::runner::Lifecycle;
use crate::stats::{Quantiles, Reconciliation, quantiles};
use serde::Serialize;
use std::collections::BTreeMap;

pub const REPORT_VERSION: u32 = 1;

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
    pub unstarted: u64,
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
            throughput_cell(target, profile, repetition, &rs, qs)
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
        unstarted,
        exact: unstarted == 0 && rec.exact(),
        accepted_per_second: rec.accepted_per_second(),
        verified_per_second: rec.verified_per_second(),
        scheduling_delay: quantiles(sched, 0, qs),
        send_latency: quantiles(lat, 0, qs),
        reconciliation: rec,
    }
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
            Phase::Present => present_censored += usize::from(r.outcome == "timeout"),
            Phase::Future if observed => {
                future.extend(diff(r.observed_ns, r.start_ns));
                offset.extend(diff(r.observed_ns, r.ack_ns));
            }
            Phase::Future => future_censored += usize::from(r.outcome == "timeout"),
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
