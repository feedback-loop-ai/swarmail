//! Protocol 4's test-execution section (accepted decision 0132 §8, §9,
//! §12, story D): the reference suite's plan, address scheme, cleanup
//! policy, record and the figures recomputed from it.
//!
//! One module, compiled by `#[path]` into the wire driver (which holds the
//! target and seals the chunk), its tests and the eval crate (which runs the
//! suite), so the plan the runner executes and the record the holder checks
//! are one definition. It uses only packages both manifests already name:
//! `serde`, `serde_json` and `ring`.
//!
//! The plan is a pure function of the protocol's corpus seed: trial `i` of
//! `0..100` has id `100 + i` and flow `FLOWS[i % 5]`; trials are claimed in
//! ascending order of SHA-256 over the ASCII `testexec/<seed>/<id>` (ties by
//! id), and the first eight bytes of that digest, big-endian, are the
//! trial's public seed. Worker scheduling never changes a trial's inputs.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The section's name in the protocol, the chunk coordinate and the report.
pub const SECTION: &str = "testexec";
/// The record's schema version.
pub const RECORD_VERSION: u32 = 1;
/// Tests per run (0132 §8.2).
pub const TRIALS: u32 = 100;
/// The first trial id. Numbering from 100 keeps every crowd alias of one
/// trial (one ends in a literal `2`) clear of every other trial's primary:
/// at id 1, `test-12@` would be both trial 1's alias and trial 12's primary.
pub const FIRST_ID: u32 = 100;
/// The five reference flows (0132 §8.1), in the order trial ids cycle them.
pub const FLOWS: [&str; 5] = [
    "otp-backlog",
    "recipient-crowd",
    "digest-lookalike",
    "reset-login",
    "magic-login",
];
/// Workers drawing from the one queue (0132 §8.2); 32 is the operator's
/// ceiling, never a test's default.
pub const PARALLELISM: [u32; 3] = [1, 8, 32];
/// Repetitions per (target, parallelism).
pub const REPETITIONS: u32 = 3;
/// One test's whole budget, claim to cleanup (0132 §8.2).
pub const TEST_DEADLINE_MS: u64 = 30_000;
/// No claim at or after this, counted from the first claim (0132 §8.2).
pub const SUITE_DEADLINE_MS: u64 = 540_000;
/// The shared-store recipient domain (0132 §8.3).
pub const DOMAIN: &str = "suite.invalid";
/// The frozen recipient scheme, `n` unpadded.
pub const RECIPIENT_SCHEME: &str = "test-<n>@suite.invalid";
/// The claim order, as the protocol states it.
pub const ORDER: &str = "ascending sha256(\"testexec/<seed>/<id>\"), ties by id";
/// The fixture time every trial's public inputs carry (whole-second UTC).
pub const FIXTURE_TIME: &str = "2026-11-16T00:00:00Z";

/// Protocol 4's `testexec` section, value for value 0132's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub trials: u32,
    pub first_id: u32,
    pub flows: Vec<String>,
    pub parallelism: Vec<u32>,
    pub repetitions: u32,
    pub test_deadline_ms: u64,
    pub suite_deadline_ms: u64,
    pub recipient_scheme: String,
    pub order: String,
    pub fixture_time: String,
    /// The wait profile each test waits with (0132 §8.1).
    pub wait_profile: String,
}

impl Section {
    pub fn specified() -> Section {
        Section {
            trials: TRIALS,
            first_id: FIRST_ID,
            flows: FLOWS.iter().map(|f| (*f).to_owned()).collect(),
            parallelism: PARALLELISM.to_vec(),
            repetitions: REPETITIONS,
            test_deadline_ms: TEST_DEADLINE_MS,
            suite_deadline_ms: SUITE_DEADLINE_MS,
            recipient_scheme: RECIPIENT_SCHEME.to_owned(),
            order: ORDER.to_owned(),
            fixture_time: FIXTURE_TIME.to_owned(),
            wait_profile: "wait-best".to_owned(),
        }
    }
}

/// One planned trial: its immutable number, flow and public inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTrial {
    pub id: u32,
    pub flow: String,
    /// The first eight bytes of the claim digest, big-endian.
    pub seed: u64,
    /// The fixture's public seed: the whole claim digest, as hex.
    pub fixture_seed: String,
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .try_into()
        .expect("a SHA-256 digest is 32 bytes")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The claim digest of trial `id` under `seed`.
pub fn claim_digest(seed: u64, id: u32) -> [u8; 32] {
    sha256(format!("testexec/{seed}/{id}").as_bytes())
}

/// The suite's plan under the protocol's corpus seed, in claim order.
pub fn plan(seed: u64) -> Vec<PlannedTrial> {
    plan_ids(seed, (0..TRIALS).map(|i| FIRST_ID + i))
}

/// The plan of any id set, flows cycling from [`FIRST_ID`]: [`plan`]'s rule,
/// open to the tests' collision cases.
pub fn plan_ids(seed: u64, ids: impl IntoIterator<Item = u32>) -> Vec<PlannedTrial> {
    let mut trials: Vec<([u8; 32], PlannedTrial)> = ids
        .into_iter()
        .map(|id| {
            let d = claim_digest(seed, id);
            let flow = FLOWS[(id.wrapping_sub(FIRST_ID) % FLOWS.len() as u32) as usize];
            let seed = u64::from_be_bytes(d[..8].try_into().expect("eight bytes"));
            (
                d,
                PlannedTrial {
                    id,
                    flow: flow.to_owned(),
                    seed,
                    fixture_seed: hex(&d),
                },
            )
        })
        .collect();
    trials.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
    trials.into_iter().map(|(_, t)| t).collect()
}

/// A trial's primary address, `test-<n>@suite.invalid`.
pub fn primary(id: u32) -> String {
    format!("test-{id}@{DOMAIN}")
}

/// A trial's Swarmail inbox: its own SMTP AUTH identity (0004).
pub fn inbox(id: u32) -> String {
    format!("test-{id}")
}

/// Every address the fixture sends a trial's mail to: the primary, and for
/// `recipient-crowd` its four aliases, in the fixture's own forms
/// (`fixture.rs`, `Scenario::SignupCrowd`; a test holds them equal to the
/// addresses the fixture really sends to).
pub fn owned(flow: &str, id: u32) -> Vec<String> {
    let p = primary(id);
    if flow != "recipient-crowd" {
        return vec![p];
    }
    let local = format!("test-{id}");
    vec![
        p,
        format!("{local}.team@{DOMAIN}"),
        format!("{local}-old@{DOMAIN}"),
        format!("{local}2@{DOMAIN}"),
        format!("old.{local}@{DOMAIN}"),
    ]
}

/// Refuse a plan whose trials are not pairwise disjoint under any target's
/// mailbox rules, before anything is sent: no address, mailbox (the local
/// part, case-folded, as a per-mailbox store keys it) or inbox may belong to
/// two trials.
pub fn check_disjoint(trials: &[PlannedTrial]) -> Result<(), String> {
    let mut owner: BTreeMap<String, u32> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for t in trials {
        if !ids.insert(t.id) {
            return Err(format!("trial {} is planned twice", t.id));
        }
        let mut keys = BTreeSet::new();
        for a in owned(&t.flow, t.id) {
            let a = a.to_ascii_lowercase();
            let local = a.split('@').next().unwrap_or_default().to_owned();
            keys.insert(format!("address {a}"));
            keys.insert(format!("mailbox {local}"));
        }
        keys.insert(format!("inbox {}", inbox(t.id).to_ascii_lowercase()));
        for k in keys {
            if let Some(other) = owner.insert(k.clone(), t.id)
                && other != t.id
            {
                return Err(format!(
                    "trials {other} and {} both own {k}: the address plan is not disjoint",
                    t.id
                ));
            }
        }
    }
    Ok(())
}

/// The trial an address belongs to under `trials`' plan, if any.
pub fn owner_of(trials: &[PlannedTrial], address: &str) -> Option<u32> {
    trials.iter().find_map(|t| {
        owned(&t.flow, t.id)
            .iter()
            .any(|a| a.eq_ignore_ascii_case(address))
            .then_some(t.id)
    })
}

/// The targets a suite launches: every protocol 4 target but the official
/// MailSlurper, whose row does not start (0132 §2.2).
pub const TARGETS: [&str; 8] = [
    "swarmail",
    "mailpit",
    "mailhog",
    "mailslurper-source-build",
    "smtp4dev",
    "inbucket",
    "greenmail",
    "alkemio-mailslurper",
];

/// One `(target, parallelism, repetition)` coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coord {
    pub target: String,
    pub parallelism: u32,
    pub repetition: u32,
}

impl Coord {
    /// `testexec.<target>.p<parallelism>.r<repetition>`.
    pub fn name(&self) -> String {
        format!(
            "{SECTION}.{}.p{}.r{}",
            self.target, self.parallelism, self.repetition
        )
    }

    /// Parse and validate a coordinate: an unknown or non-launching target,
    /// a parallelism other than 1, 8 or 32, or a repetition outside 0–2 is
    /// refused by name.
    pub fn parse(text: &str) -> Result<Coord, String> {
        let parts: Vec<&str> = text.split('.').collect();
        let num = |s: &str, p: char| {
            s.strip_prefix(p)
                .filter(|n| !n.is_empty() && (!n.starts_with('0') || *n == "0"))
                .and_then(|n| n.parse::<u32>().ok())
                .ok_or_else(|| format!("chunk {text:?}: {s:?} is not {p}<n>"))
        };
        let [SECTION, target, par, rep] = parts.as_slice() else {
            return Err(format!(
                "{text:?} is not a testexec.<target>.p<parallelism>.r<repetition> chunk"
            ));
        };
        if *target == "mailslurper" {
            return Err(format!(
                "chunk {text:?}: the official mailslurper does not start (0132 §2.2)"
            ));
        }
        if !TARGETS.contains(target) {
            return Err(format!("chunk {text:?}: unknown target {target:?}"));
        }
        let parallelism = num(par, 'p')?;
        if !PARALLELISM.contains(&parallelism) {
            return Err(format!(
                "chunk {text:?}: parallelism {parallelism} is not one of 1, 8, 32"
            ));
        }
        let repetition = num(rep, 'r')?;
        if repetition >= REPETITIONS {
            return Err(format!(
                "chunk {text:?}: repetition {repetition} is outside 0..{REPETITIONS}"
            ));
        }
        Ok(Coord {
            target: (*target).to_owned(),
            parallelism,
            repetition,
        })
    }
}

/// The narrowest documented cleanup, chosen from the capability table
/// before work starts (0132 §8.4, acceptance answer 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    /// Swarmail clears the trial's own inbox.
    InboxClear,
    /// Delete every message the trial owns, by id, then verify it gone.
    DeleteById,
    /// Only a global clear, run after each test (parallelism 1).
    GlobalClearPerTest,
    /// Only a global clear, run once after every worker settled and outside
    /// the suite window (parallelism 8 and 32).
    GlobalClearAfterSuite,
    /// No delete or clear the captured documents establish: nothing is
    /// removed, and every row says so.
    Unavailable,
}

impl Policy {
    pub fn of(target: &str, parallelism: u32) -> Policy {
        use crate::clients::{ClearScope, Operation, native_capability};
        let delete = native_capability(target, Operation::Delete);
        let clear = native_capability(target, Operation::Clear);
        match (clear.scope(), delete.supported()) {
            (Some(ClearScope::Recipient), _) if target == "swarmail" => Policy::InboxClear,
            (_, true) => Policy::DeleteById,
            (Some(_), false) if parallelism == 1 => Policy::GlobalClearPerTest,
            (Some(_), false) => Policy::GlobalClearAfterSuite,
            (None, false) => Policy::Unavailable,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Policy::InboxClear => "inbox_clear",
            Policy::DeleteById => "delete_by_id",
            Policy::GlobalClearPerTest => "global_clear_per_test",
            Policy::GlobalClearAfterSuite => "global_clear_after_suite",
            Policy::Unavailable => "unavailable",
        }
    }
}

/// How a test waits: the target's `wait-best` row (0132 §7.2, §8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum WaitMethod {
    /// Swarmail's blocking REST await (0002).
    Native,
    /// A cited push channel waking an authoritative scan.
    Stream { backstop_ms: u64 },
    /// The common complete-scan poll.
    Poll { interval_ms: u64 },
}

/// One trial's outcome (0132 §8.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Timeout,
    WrongMessage,
    Interference,
    Harness,
}

impl Outcome {
    pub const ALL: [Outcome; 5] = [
        Outcome::Pass,
        Outcome::Timeout,
        Outcome::WrongMessage,
        Outcome::Interference,
        Outcome::Harness,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Timeout => "timeout",
            Outcome::WrongMessage => "wrong_message",
            Outcome::Interference => "interference",
            Outcome::Harness => "harness",
        }
    }
}

/// What a trial's own cleanup did, apart from its outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Cleanup {
    /// The policy's mutation ran and its scope was verified empty.
    Done { removed: u64 },
    /// The mutation or its verification failed; nothing broader ran.
    Failed { reason: String },
    /// The suite's one global clear owns it (`global_clear_after_suite`).
    Deferred,
    /// The target documents no cleanup (`unavailable`).
    NotAvailable,
    /// Never attempted: the trial ended before it could clean, or the suite
    /// cut it. Never a pass.
    NotRun { reason: String },
}

impl Cleanup {
    /// Whether the cleanup meets its declared policy.
    pub fn proved(&self, policy: Policy) -> bool {
        matches!(
            (self, policy),
            (
                Cleanup::Done { .. },
                Policy::InboxClear | Policy::DeleteById | Policy::GlobalClearPerTest
            ) | (Cleanup::Deferred, Policy::GlobalClearAfterSuite)
                | (Cleanup::NotAvailable, Policy::Unavailable)
        )
    }
}

/// A wait's timing on the suite's one monotonic clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitTiming {
    pub call_ns: u64,
    pub return_ns: u64,
    /// How many messages the wait required.
    pub count: u32,
    /// The final DATA 250 that completed the required set, when observed.
    pub ack_ns: Option<u64>,
    /// `return_ns - ack_ns`, signed and never clamped; absent with a reason.
    pub ack_to_return_ns: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ack_absent: Option<String>,
    /// How the answer was seen: `native`, `poll`, or a stream scan's cause.
    pub cause: String,
}

/// One claimed trial, start to end (0132 §8.5, §8.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialRow {
    pub id: u32,
    pub flow: String,
    pub seed: u64,
    pub worker: u32,
    pub claim_ns: u64,
    pub end_ns: u64,
    pub outcome: Outcome,
    /// Why a trial that did not pass ended as it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// Causal evidence behind `interference` (never empty for it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    /// The fixture's app-state scorer, when it judged (0055).
    pub scored: Option<bool>,
    /// Distinct other-trial messages the target's filter returned and the
    /// exact match dropped (an observation, not a failure).
    pub foreign_seen: u64,
    /// Distinct messages of this trial the source saw (accounting).
    pub owned_seen: u64,
    pub wait: Option<WaitTiming>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_absent: Option<String>,
    pub cleanup: Cleanup,
}

/// A planned trial no worker claimed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unstarted {
    pub id: u32,
    pub reason: String,
}

/// One control round trip to the holder: sent and received on the runner's
/// clock, and the holder's own instant (never subtracted from the runner's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trip {
    pub sent_ns: u64,
    pub received_ns: u64,
    pub owner_t_ns: u64,
}

/// The client process's CPU over the suite interval (`RUSAGE_SELF`, which
/// includes the co-resident fixture apps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientCpu {
    pub user_usec: u64,
    pub system_usec: u64,
}

/// The suite's one deferred global clear, outside the suite window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalCleanup {
    pub start_ns: u64,
    pub end_ns: u64,
    pub cleanup: Cleanup,
}

/// Everything a test-execution chunk keeps: enough to recompute the
/// section's figures (0132 §12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteRecord {
    pub record_version: u32,
    pub target: String,
    pub parallelism: u32,
    pub repetition: u32,
    pub seed: u64,
    pub policy: Policy,
    pub method: WaitMethod,
    pub plan: Vec<PlannedTrial>,
    pub rows: Vec<TrialRow>,
    pub unstarted: Vec<Unstarted>,
    /// First claim to last trial end; absent when nothing started.
    pub first_claim_ns: Option<u64>,
    pub last_end_ns: Option<u64>,
    /// The holder's suite window, opened before the first claim and closed
    /// after the last end; both round trips are kept, apart from wall time.
    pub window_start: Option<Trip>,
    pub window_end: Option<Trip>,
    pub client_cpu: Option<ClientCpu>,
    pub final_cleanup: Option<FinalCleanup>,
    /// A documented setup refusal: nothing started, every trial unstarted.
    pub blocked: Option<String>,
}

impl SuiteRecord {
    /// An empty record of `coord` under `seed`'s plan.
    pub fn new(coord: &Coord, seed: u64, method: WaitMethod) -> SuiteRecord {
        SuiteRecord {
            record_version: RECORD_VERSION,
            target: coord.target.clone(),
            parallelism: coord.parallelism,
            repetition: coord.repetition,
            seed,
            policy: Policy::of(&coord.target, coord.parallelism),
            method,
            plan: plan(seed),
            rows: Vec::new(),
            unstarted: Vec::new(),
            first_claim_ns: None,
            last_end_ns: None,
            window_start: None,
            window_end: None,
            client_cpu: None,
            final_cleanup: None,
            blocked: None,
        }
    }

    /// Every planned trial unstarted for `reason`, nothing started.
    pub fn block(&mut self, reason: &str) {
        self.blocked = Some(reason.to_owned());
        self.rows.clear();
        self.unstarted = self
            .plan
            .iter()
            .map(|t| Unstarted {
                id: t.id,
                reason: reason.to_owned(),
            })
            .collect();
        (self.first_claim_ns, self.last_end_ns) = (None, None);
    }
}

/// Every check a sealed record must pass: the coordinate, the plan, the
/// policy, exactly one row or unstarted entry per planned id, each row
/// consistent with its class and its cleanup, the suite interval derived
/// from the rows, and the deferred clear where the policy needs one.
pub fn check_record(rec: &SuiteRecord, coord: &Coord) -> Result<(), String> {
    if rec.record_version != RECORD_VERSION {
        return Err(format!("record_version {}", rec.record_version));
    }
    if (rec.target.as_str(), rec.parallelism, rec.repetition)
        != (coord.target.as_str(), coord.parallelism, coord.repetition)
    {
        return Err(format!("the record is not of {}", coord.name()));
    }
    if rec.plan != plan(rec.seed) {
        return Err(format!("the plan is not seed {}'s", rec.seed));
    }
    let policy = Policy::of(&rec.target, rec.parallelism);
    if rec.policy != policy {
        return Err(format!(
            "policy {} is not {}'s {}",
            rec.policy.name(),
            rec.target,
            policy.name()
        ));
    }
    let planned: BTreeMap<u32, &PlannedTrial> = rec.plan.iter().map(|t| (t.id, t)).collect();
    let mut seen = BTreeSet::new();
    for id in rec
        .rows
        .iter()
        .map(|r| r.id)
        .chain(rec.unstarted.iter().map(|u| u.id))
    {
        if !planned.contains_key(&id) {
            return Err(format!("trial {id} is not planned"));
        }
        if !seen.insert(id) {
            return Err(format!("trial {id} is accounted for twice"));
        }
    }
    if seen.len() != planned.len() {
        let missing: Vec<u32> = planned
            .keys()
            .filter(|i| !seen.contains(i))
            .copied()
            .collect();
        return Err(format!("trials {missing:?} are not accounted for"));
    }
    if rec.blocked.is_some() && !rec.rows.is_empty() {
        return Err("a blocked suite started trials".into());
    }
    if rec.unstarted.iter().any(|u| u.reason.is_empty()) {
        return Err("an unstarted trial without its reason".into());
    }
    for r in &rec.rows {
        check_row(rec, planned[&r.id], r)?;
    }
    let first = rec.rows.iter().map(|r| r.claim_ns).min();
    let last = rec.rows.iter().map(|r| r.end_ns).max();
    if (rec.first_claim_ns, rec.last_end_ns) != (first, last) {
        return Err("the suite interval is not the rows' first claim and last end".into());
    }
    if first.is_none() && (rec.window_start.is_some() || rec.window_end.is_some()) {
        return Err("a suite that started nothing opened its window".into());
    }
    if let Some(first) = first {
        // The budgets are absolute (0132 §8.2): no claim at or after the
        // suite cutoff, and no test past the earlier of that cutoff and its
        // own 30 s.
        let cutoff = first + SUITE_DEADLINE_MS * 1_000_000;
        for r in &rec.rows {
            if r.claim_ns >= cutoff || r.end_ns > cutoff {
                return Err(format!("trial {}: runs past the suite cutoff", r.id));
            }
            if r.end_ns - r.claim_ns > TEST_DEADLINE_MS * 1_000_000 {
                return Err(format!("trial {}: runs past its test deadline", r.id));
            }
        }
    }
    if let (Some(first), Some(last)) = (first, last) {
        let (Some(a), Some(b)) = (&rec.window_start, &rec.window_end) else {
            return Err("trials ran without a suite window".into());
        };
        if a.received_ns > first || b.sent_ns < last {
            return Err("the suite window does not enclose every trial".into());
        }
        if rec.client_cpu.is_none() {
            return Err("trials ran without the client's CPU".into());
        }
    }
    match (&rec.final_cleanup, rec.policy) {
        (Some(f), Policy::GlobalClearAfterSuite) => {
            if rec
                .window_end
                .as_ref()
                .is_none_or(|b| f.start_ns < b.received_ns)
            {
                return Err("the global clear did not follow the suite window".into());
            }
            if f.end_ns < f.start_ns {
                return Err("the global clear ends before it starts".into());
            }
            if !matches!(f.cleanup, Cleanup::Done { .. } | Cleanup::Failed { .. }) {
                return Err(format!("the global clear is {:?}", f.cleanup));
            }
        }
        (None, Policy::GlobalClearAfterSuite) if !rec.rows.is_empty() => {
            return Err("the suite's global clear is missing".into());
        }
        (Some(_), p) => return Err(format!("a final clear under {}", p.name())),
        _ => {}
    }
    Ok(())
}

fn check_row(rec: &SuiteRecord, t: &PlannedTrial, r: &TrialRow) -> Result<(), String> {
    let id = r.id;
    if (r.flow.as_str(), r.seed) != (t.flow.as_str(), t.seed) {
        return Err(format!("trial {id}: flow or seed differs from the plan"));
    }
    if r.worker >= rec.parallelism {
        return Err(format!(
            "trial {id}: worker {} of {}",
            r.worker, rec.parallelism
        ));
    }
    if r.end_ns < r.claim_ns {
        return Err(format!("trial {id}: ends before its claim"));
    }
    match r.outcome {
        Outcome::Pass => {
            if r.scored != Some(true) || r.cause.is_some() {
                return Err(format!("trial {id}: a pass the scorer did not pass"));
            }
            if !r.cleanup.proved(rec.policy) {
                return Err(format!(
                    "trial {id}: a pass without its {} cleanup",
                    rec.policy.name()
                ));
            }
            if r.wait.is_none() {
                return Err(format!("trial {id}: a pass without its wait"));
            }
        }
        o if r.cause.as_deref().is_none_or(str::is_empty) => {
            return Err(format!("trial {id}: {} without a cause", o.name()));
        }
        Outcome::Interference if r.evidence.is_empty() => {
            return Err(format!("trial {id}: interference without causal evidence"));
        }
        _ => {}
    }
    if r.outcome != Outcome::Interference && !r.evidence.is_empty() {
        return Err(format!(
            "trial {id}: evidence on a {} row",
            r.outcome.name()
        ));
    }
    let allowed = match &r.cleanup {
        Cleanup::Deferred => rec.policy == Policy::GlobalClearAfterSuite,
        Cleanup::NotAvailable => rec.policy == Policy::Unavailable,
        Cleanup::Done { .. } | Cleanup::Failed { .. } => !matches!(
            rec.policy,
            Policy::GlobalClearAfterSuite | Policy::Unavailable
        ),
        Cleanup::NotRun { reason } => !reason.is_empty(),
    };
    if !allowed {
        return Err(format!(
            "trial {id}: cleanup {:?} under {}",
            r.cleanup,
            rec.policy.name()
        ));
    }
    match (&r.wait, &r.wait_absent) {
        (Some(w), None) => {
            if w.return_ns < w.call_ns || w.call_ns < r.claim_ns || w.return_ns > r.end_ns {
                return Err(format!("trial {id}: its wait lies outside the trial"));
            }
            let want = w.ack_ns.map(|a| w.return_ns as i64 - a as i64);
            if w.ack_to_return_ns != want || w.ack_ns.is_none() != w.ack_absent.is_some() {
                return Err(format!("trial {id}: ack_to_return disagrees with its ack"));
            }
        }
        (None, Some(why)) if !why.is_empty() => {}
        _ => return Err(format!("trial {id}: neither a wait nor its absence")),
    }
    Ok(())
}

/// Whether a checked record's cleanup is proved: every row's own cleanup
/// met its policy and, where the suite's one global clear owns cleanup, it
/// was done. A suite whose cleanup is not proved is never a successful
/// measurement.
pub fn cleanup_proved(rec: &SuiteRecord) -> Result<(), String> {
    for r in &rec.rows {
        if !r.cleanup.proved(rec.policy) {
            return Err(format!("trial {}: cleanup {:?}", r.id, r.cleanup));
        }
    }
    if rec.policy == Policy::GlobalClearAfterSuite
        && !rec.rows.is_empty()
        && !matches!(
            rec.final_cleanup.as_ref().map(|f| &f.cleanup),
            Some(Cleanup::Done { .. })
        )
    {
        return Err("the suite's global clear is not proved".into());
    }
    Ok(())
}

/// The target's own losses a checked record states, each named: every
/// trial that did not pass, every cleanup that is not proved under its
/// policy, every unstarted trial and a failed suite clear. An empty list is
/// the only successful measurement: a hundred scored passes, cleanup proved.
pub fn failures(rec: &SuiteRecord) -> Vec<String> {
    let name = Coord {
        target: rec.target.clone(),
        parallelism: rec.parallelism,
        repetition: rec.repetition,
    }
    .name();
    let mut out = Vec::new();
    if let Some(b) = &rec.blocked {
        out.push(format!("{name}: blocked: {b}"));
    }
    for r in &rec.rows {
        if r.outcome != Outcome::Pass {
            out.push(format!(
                "{name} trial {} ({}): {}: {}",
                r.id,
                r.flow,
                r.outcome.name(),
                r.cause.clone().unwrap_or_default()
            ));
        }
        if !r.cleanup.proved(rec.policy) {
            out.push(format!("{name} trial {}: cleanup {:?}", r.id, r.cleanup));
        }
    }
    if rec.blocked.is_none() {
        let mut why: BTreeMap<&str, u32> = BTreeMap::new();
        for u in &rec.unstarted {
            *why.entry(u.reason.as_str()).or_default() += 1;
        }
        for (reason, n) in why {
            out.push(format!("{name}: {n} trial(s) unstarted: {reason}"));
        }
    }
    if let Some(f) = &rec.final_cleanup
        && !matches!(f.cleanup, Cleanup::Done { .. })
    {
        out.push(format!("{name}: the suite's global clear: {:?}", f.cleanup));
    }
    out
}

/// The nearest-rank quantile of sorted values.
fn rank(sorted: &[i64], q: f64) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let n = (q * sorted.len() as f64).ceil().max(1.0) as usize;
    Some(sorted[n.min(sorted.len()) - 1])
}

fn quantiles(mut v: Vec<i64>) -> serde_json::Value {
    v.sort_unstable();
    serde_json::json!({
        "n": v.len(),
        "p50": rank(&v, 0.50),
        "p95": rank(&v, 0.95),
        "max": v.last(),
    })
}

/// The section's row-derived figures, each a pure function of the record,
/// so the holder and every later reader recompute them (0132 §8.6, §12).
/// The target's CPU and memory over the suite window are the holder's
/// window rows, added beside these by the driver.
pub fn report(rec: &SuiteRecord) -> serde_json::Value {
    let mut by_class: BTreeMap<&str, u64> = Outcome::ALL.iter().map(|o| (o.name(), 0)).collect();
    let mut by_flow: BTreeMap<&str, BTreeMap<&str, u64>> = BTreeMap::new();
    for r in &rec.rows {
        *by_class.entry(r.outcome.name()).or_default() += 1;
        *by_flow
            .entry(r.flow.as_str())
            .or_default()
            .entry(r.outcome.name())
            .or_default() += 1;
    }
    let walls = rec
        .rows
        .iter()
        .map(|r| (r.end_ns - r.claim_ns) as i64)
        .collect();
    let waits = rec
        .rows
        .iter()
        .filter_map(|r| r.wait.as_ref())
        .map(|w| (w.return_ns - w.call_ns) as i64)
        .collect();
    let acks = rec
        .rows
        .iter()
        .filter_map(|r| r.wait.as_ref()?.ack_to_return_ns)
        .collect();
    let coordinate = Coord {
        target: rec.target.clone(),
        parallelism: rec.parallelism,
        repetition: rec.repetition,
    }
    .name();
    serde_json::json!({
        "coordinate": coordinate,
        "seed": rec.seed,
        "policy": rec.policy.name(),
        "method": rec.method,
        "planned": rec.plan.len(),
        "started": rec.rows.len(),
        "unstarted": rec.unstarted.len(),
        "blocked": rec.blocked,
        "by_class": by_class,
        "by_flow": by_flow,
        "foreign_seen": rec.rows.iter().map(|r| r.foreign_seen).sum::<u64>(),
        "trials_with_foreign_seen": rec.rows.iter().filter(|r| r.foreign_seen > 0).count(),
        "suite_wall_ns": rec.first_claim_ns.zip(rec.last_end_ns).map(|(a, b)| b - a),
        "trial_wall_ns": quantiles(walls),
        "wait_ns": quantiles(waits),
        "ack_to_return_ns": quantiles(acks),
        "waits_absent": rec.rows.iter().filter(|r| r.wait.is_none()).count(),
        "client_cpu_usec": rec.client_cpu.map(|c| c.user_usec + c.system_usec),
        "cleanup_failures": rec.rows.iter().filter(|r| matches!(r.cleanup, Cleanup::Failed { .. })).count(),
        "final_cleanup": rec.final_cleanup.as_ref().map(|f| &f.cleanup),
        "window_round_trips_ns": [
            rec.window_start.as_ref().map(|t| t.received_ns.saturating_sub(t.sent_ns)),
            rec.window_end.as_ref().map(|t| t.received_ns.saturating_sub(t.sent_ns)),
        ],
        "at_equal_results": failures(rec).is_empty(),
    })
}
