//! The frozen experiment (spec R1) and the competitor source lock: closed,
//! versioned records whose validators refuse before any collection. A
//! draft protocol or an unverified source pin may drive the labelled smoke
//! run; it can never become measured evidence.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The current method's version. Version 1 is the unfrozen drafts (draft-2
/// and draft-3), whose bytes stay under `docs/evals/history/` and still
/// read here with none of the version 2 sections; only version 2 can
/// collect (proposed decision 0093 §3).
pub const PROTOCOL_VERSION: u32 = 2;
pub const DRAFT_PROTOCOL_VERSION: u32 = 1;

/// Upper bounds the validator holds every count and deadline to: a
/// protocol is an input file, and an absurd value must refuse rather than
/// schedule unbounded work.
pub const MAX_MESSAGES: u32 = 100_000;
pub const MAX_SESSIONS: u32 = 1_000;
pub const MAX_DEADLINE_MS: u64 = 600_000;
pub const MAX_REPETITIONS: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Swarmail,
    Mailpit,
    Mailhog,
    /// Added with lock version 4 (decision 0129 §7): an agent-comparison
    /// target, never a wire-bench one.
    Mailslurper,
}

/// The three targets lock versions 1 to 3 know. Each historical reader
/// keeps this expectation, so adding a target to [`TargetKind::ALL`] never
/// changes what an archived lock means (decision 0093 §1).
pub const HISTORICAL_TARGETS: [TargetKind; 3] = [
    TargetKind::Swarmail,
    TargetKind::Mailpit,
    TargetKind::Mailhog,
];

impl TargetKind {
    pub const ALL: [TargetKind; 4] = [
        TargetKind::Swarmail,
        TargetKind::Mailpit,
        TargetKind::Mailhog,
        TargetKind::Mailslurper,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TargetKind::Swarmail => "swarmail",
            TargetKind::Mailpit => "mailpit",
            TargetKind::Mailhog => "mailhog",
            TargetKind::Mailslurper => "mailslurper",
        }
    }

    pub fn parse(name: &str) -> Option<TargetKind> {
        TargetKind::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The upstream home a competitor's official source must come from.
    /// Third-party mirrors, recipes and images cannot stand in (spec R1).
    /// Swarmail is the candidate itself and has no upstream. The captured
    /// release metadata under `docs/evals/sources/` names these homes as
    /// the releases' own; [`SourceLock::verify_files`] rechecks that.
    pub fn official_repository(self) -> Option<&'static str> {
        match self {
            TargetKind::Swarmail => None,
            TargetKind::Mailpit => Some("https://github.com/axllent/mailpit"),
            TargetKind::Mailhog => Some("https://github.com/mailhog/MailHog"),
            TargetKind::Mailslurper => Some("https://github.com/mailslurper/mailslurper"),
        }
    }
}

impl fmt::Display for TargetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    Fresh,
    Reused,
}

/// How a wait profile observes a target. `Native` is a server's own
/// blocking wait (Swarmail's await); `Poll` is the common client path at a
/// frozen interval, the only shape a target without a source-verified wait
/// facility gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Observation {
    Native,
    Poll,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Protocol {
    pub protocol_version: u32,
    pub id: String,
    pub status: Status,
    /// Why this protocol differs from its predecessor; required whenever a
    /// predecessor is named (a changed method is a new run, never a rerun).
    pub supersedes: Option<String>,
    pub change_reason: Option<String>,
    pub targets: Vec<TargetKind>,
    pub corpus: Corpus,
    pub throughput: Vec<ThroughputProfile>,
    pub waits: Vec<WaitProfile>,
    pub resources: Resources,
    pub storage: Storage,
    pub statistics: Statistics,
    /// Version 2: what the measurement may claim about the executables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
    /// Version 2: the common poll, the driver's wait operation bound and
    /// the adapter's bounds and schema identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budgets: Option<Budgets>,
    /// Version 2: the immutable product each target is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<Product>>,
    /// Every value the method still lacks, by name. Collection refuses
    /// while one remains; the list is how a draft says what it is missing
    /// instead of carrying a placeholder that looks like a value.
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub seed: u64,
    /// Authored kinds, cycled in this order across message indices.
    pub kinds: Vec<MessageKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Plain,
    Html,
    Multipart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThroughputProfile {
    pub name: String,
    pub repetitions: u32,
    pub warmup_messages: u32,
    pub messages: u32,
    pub sessions: u32,
    pub connection: Connection,
    pub send_deadline_ms: u64,
    pub verify_deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitProfile {
    pub name: String,
    pub repetitions: u32,
    pub present: u32,
    pub future: u32,
    pub empty: u32,
    pub deadline_ms: u64,
    pub empty_deadline_ms: u64,
    pub observation: Vec<TargetObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetObservation {
    pub target: TargetKind,
    pub mode: Observation,
    /// Poll interval; required for `poll`, refused for `native`.
    pub interval_ms: Option<u64>,
    /// The source-backed reason this mode was chosen for this target.
    pub justification: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    /// Whether the selected runtime actually enforces the budget. An
    /// unenforced budget is recorded as such and refuses collection: spec
    /// R3 forbids describing unenforced limits as enforced.
    pub enforced: bool,
    pub cpus: Option<u32>,
    pub memory_mib: Option<u32>,
    pub open_files: u32,
    pub startup_deadline_ms: u64,
    pub cleanup_deadline_ms: u64,
    pub request_deadline_ms: u64,
    pub max_response_bytes: u64,
    /// Version 2: the profile this budget is, under its own name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ResourceProfile>,
}

/// An unenforced budget is a separately named local profile, never the
/// requested enforced one (proposed decision 0093 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceProfile {
    Enforced,
    UnenforcedLocal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    /// `memory` is the matched primary profile; it is valid only when every
    /// target's lock entry records that it supports memory-only storage.
    pub mode: String,
    /// Version 2: whether every target stores the same way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<StorageProfile>,
    /// Per-target retention settings, stated as launched. An exact-count
    /// workload may never knowingly overflow one (spec R3).
    pub retention: Vec<Retention>,
}

/// `Matched` claims one backend and one cap for every target; anything
/// else is the explicitly `Unmatched` profile, whose per-target facts are
/// the comparison's stated difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageProfile {
    MatchedMemory,
    Unmatched,
}

/// What a cap does when it is reached. Only what a source or a probe
/// established may be stated; `Unestablished` is the honest default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prune {
    /// The launch sets no cap, the executable's help lists no cap flag and
    /// a bounded probe saw none applied: not proof that none exists.
    NoCapObserved,
    /// The cap evicts the oldest mail only (Swarmail, decision 0003).
    OldestOnly,
    /// A cap exists; what it evicts is not established.
    Unestablished,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    pub target: TargetKind,
    /// `None` = unlimited, stated explicitly.
    pub max_messages: Option<u64>,
    /// Version 2: where the target keeps mail, what the cap does and what
    /// is established about compression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prune: Option<Prune>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
}

/// The limited claim a measurement of official artifacts makes (proposed
/// decision 0093 §2). `OfficialArtifact` measures the digest-checked
/// executable and attributes only its tagged source context; it makes no
/// built-from claim and cannot satisfy `BuiltFrom`, which needs resolved
/// binary build provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimScope {
    OfficialArtifact,
    BuiltFrom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub scope: ClaimScope,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// The common-client poll interval every poll observation uses.
    pub poll_interval_ms: u64,
    /// The wire driver's bound on one wait operation: the longest wait
    /// deadline plus one request bound (spec clarification Q3).
    pub wait_operation_ms: u64,
    pub adapter: AdapterBudgets,
}

/// The MCP adapter's declared bounds and schema identity; each must equal
/// what the adapter is built with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterBudgets {
    pub operation_ms: u64,
    pub request_ms: u64,
    /// The admitted `timeout_ms` maximum: `operation_ms - request_ms`.
    pub max_wait_ms: u64,
    pub max_pages: u32,
    pub max_requests: u32,
    pub max_operation_bytes: u64,
    pub wait_method: String,
    /// SHA-256 of the `tools/list` array the adapter serves.
    pub tools_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub target: TargetKind,
    /// A competitor's official release tag; the candidate has none.
    pub release_tag: Option<String>,
    /// A competitor's executable digest; the candidate's is recorded per
    /// run in the manifest.
    pub executable_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statistics {
    pub quantile_method: String,
    pub quantiles: Vec<u32>,
    /// One target order per repetition; each must be a permutation of
    /// `targets` so no target always runs first.
    pub orders: Vec<Vec<TargetKind>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub Vec<String>);

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("; "))
    }
}

pub fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

/// `v` and a dotted number: a version tag, never a branch or `latest`.
pub fn is_release_tag(s: &str) -> bool {
    s.strip_prefix('v').is_some_and(|n| {
        !n.is_empty()
            && n.split('.').count() >= 2
            && n.split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    })
}

pub fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn bounded(out: &mut Vec<String>, owner: &str, field: &str, value: u64, min: u64, max: u64) {
    if value < min || value > max {
        out.push(format!("{owner}: {field} {value} outside {min}..={max}"));
    }
}

impl Protocol {
    pub fn parse(bytes: &[u8]) -> Result<Protocol, Refusal> {
        serde_json::from_slice(bytes)
            .map_err(|e| Refusal(vec![format!("protocol does not parse: {e}")]))
    }

    /// Structural validity: every count bounded, every name and reference
    /// well formed. A draft can be structurally valid; collection needs
    /// [`collection_ready`] on top.
    pub fn validate(&self) -> Result<(), Refusal> {
        let mut out = Vec::new();
        if !matches!(
            self.protocol_version,
            DRAFT_PROTOCOL_VERSION | PROTOCOL_VERSION
        ) {
            out.push(format!(
                "protocol_version {} is neither {DRAFT_PROTOCOL_VERSION} nor {PROTOCOL_VERSION}",
                self.protocol_version
            ));
        }
        if !is_name(&self.id) {
            out.push("id must be 1-64 [A-Za-z0-9._-]".into());
        }
        if self.supersedes.is_some()
            && self
                .change_reason
                .as_deref()
                .is_none_or(|r| r.trim().is_empty())
        {
            out.push("a protocol that supersedes another must state its change_reason".into());
        }
        if self.targets.is_empty() {
            out.push("no targets".into());
        }
        for (i, t) in self.targets.iter().enumerate() {
            if self.targets[..i].contains(t) {
                out.push(format!("target {t} listed twice"));
            }
        }
        if self.corpus.kinds.is_empty() {
            out.push("corpus has no message kinds".into());
        }
        if self.throughput.is_empty() && self.waits.is_empty() {
            out.push("no profiles".into());
        }
        let mut names: Vec<&str> = Vec::new();
        for p in &self.throughput {
            if !is_name(&p.name) || names.contains(&p.name.as_str()) {
                out.push(format!(
                    "profile name {:?} is malformed or duplicated",
                    p.name
                ));
            }
            names.push(&p.name);
            let n = &p.name;
            bounded(
                &mut out,
                n,
                "repetitions",
                p.repetitions.into(),
                1,
                MAX_REPETITIONS.into(),
            );
            bounded(
                &mut out,
                n,
                "messages",
                p.messages.into(),
                1,
                MAX_MESSAGES.into(),
            );
            bounded(
                &mut out,
                n,
                "warmup_messages",
                p.warmup_messages.into(),
                0,
                MAX_MESSAGES.into(),
            );
            bounded(
                &mut out,
                n,
                "sessions",
                p.sessions.into(),
                1,
                MAX_SESSIONS.into(),
            );
            bounded(
                &mut out,
                n,
                "send_deadline_ms",
                p.send_deadline_ms,
                1,
                MAX_DEADLINE_MS,
            );
            bounded(
                &mut out,
                n,
                "verify_deadline_ms",
                p.verify_deadline_ms,
                1,
                MAX_DEADLINE_MS,
            );
            if p.sessions > p.messages {
                out.push(format!("{n}: more sessions than messages"));
            }
            for r in &self.storage.retention {
                let total = u64::from(p.messages) + u64::from(p.warmup_messages);
                if r.max_messages.is_some_and(|cap| total > cap) {
                    out.push(format!(
                        "{n}: {total} messages overflow {}'s retention; an exact-count workload must fit",
                        r.target
                    ));
                }
            }
        }
        for w in &self.waits {
            if !is_name(&w.name) || names.contains(&w.name.as_str()) {
                out.push(format!(
                    "profile name {:?} is malformed or duplicated",
                    w.name
                ));
            }
            names.push(&w.name);
            let n = &w.name;
            bounded(
                &mut out,
                n,
                "repetitions",
                w.repetitions.into(),
                1,
                MAX_REPETITIONS.into(),
            );
            bounded(
                &mut out,
                n,
                "present",
                w.present.into(),
                0,
                MAX_MESSAGES.into(),
            );
            bounded(
                &mut out,
                n,
                "future",
                w.future.into(),
                0,
                MAX_MESSAGES.into(),
            );
            bounded(&mut out, n, "empty", w.empty.into(), 0, MAX_MESSAGES.into());
            bounded(
                &mut out,
                n,
                "deadline_ms",
                w.deadline_ms,
                1,
                MAX_DEADLINE_MS,
            );
            bounded(
                &mut out,
                n,
                "empty_deadline_ms",
                w.empty_deadline_ms,
                1,
                MAX_DEADLINE_MS,
            );
            for t in &self.targets {
                if w.observation.iter().filter(|o| o.target == *t).count() != 1 {
                    out.push(format!("{n}: {t} needs exactly one observation mode"));
                }
            }
            for o in &w.observation {
                let t = o.target;
                if !self.targets.contains(&t) {
                    out.push(format!("{n}: observation for undeclared target {t}"));
                }
                match (o.mode, o.interval_ms) {
                    (Observation::Poll, None) => {
                        out.push(format!("{n}: {t} poll has no frozen interval"))
                    }
                    (Observation::Poll, Some(i)) => {
                        bounded(&mut out, n, "interval_ms", i, 1, MAX_DEADLINE_MS)
                    }
                    (Observation::Native, Some(_)) => {
                        out.push(format!("{n}: {t} native wait carries a poll interval"))
                    }
                    (Observation::Native, None) if t != TargetKind::Swarmail => out.push(format!(
                        "{n}: no source-verified native wait is implemented for {t}"
                    )),
                    (Observation::Native, None) => {}
                }
                if o.justification.trim().is_empty() {
                    out.push(format!("{n}: {t} observation has no justification"));
                }
            }
            // A wait instance holds every present and future send at once.
            let sends = u64::from(w.present) + u64::from(w.future);
            for r in &self.storage.retention {
                if r.max_messages.is_some_and(|cap| sends > cap) {
                    out.push(format!(
                        "{n}: {sends} wait sends overflow {}'s retention; an exact-count workload must fit",
                        r.target
                    ));
                }
            }
        }
        let r = &self.resources;
        bounded(
            &mut out,
            "resources",
            "open_files",
            r.open_files.into(),
            64,
            1 << 20,
        );
        bounded(
            &mut out,
            "resources",
            "startup_deadline_ms",
            r.startup_deadline_ms,
            1,
            MAX_DEADLINE_MS,
        );
        bounded(
            &mut out,
            "resources",
            "cleanup_deadline_ms",
            r.cleanup_deadline_ms,
            1,
            MAX_DEADLINE_MS,
        );
        bounded(
            &mut out,
            "resources",
            "request_deadline_ms",
            r.request_deadline_ms,
            1,
            MAX_DEADLINE_MS,
        );
        bounded(
            &mut out,
            "resources",
            "max_response_bytes",
            r.max_response_bytes,
            1024,
            256 << 20,
        );
        if r.enforced && (r.cpus.is_none() || r.memory_mib.is_none()) {
            out.push("resources: an enforced budget must state cpus and memory_mib".into());
        }
        if !matches!(self.storage.mode.as_str(), "memory" | "durable" | "mixed") {
            out.push(format!(
                "storage mode {:?} is not memory, durable or mixed",
                self.storage.mode
            ));
        }
        for t in &self.targets {
            if self
                .storage
                .retention
                .iter()
                .filter(|r| r.target == *t)
                .count()
                != 1
            {
                out.push(format!("storage: {t} needs exactly one retention entry"));
            }
        }
        let s = &self.statistics;
        if s.quantile_method != "nearest-rank" {
            out.push(format!(
                "quantile method {:?} is not nearest-rank",
                s.quantile_method
            ));
        }
        if s.quantiles.is_empty() || s.quantiles.iter().any(|q| *q == 0 || *q > 100) {
            out.push("quantiles must be non-empty percentiles in 1..=100".into());
        }
        let reps = self
            .throughput
            .iter()
            .map(|p| p.repetitions)
            .chain(self.waits.iter().map(|w| w.repetitions))
            .max()
            .unwrap_or(0);
        if (s.orders.len() as u64) < u64::from(reps) {
            out.push(format!(
                "{} target orders for {reps} repetitions",
                s.orders.len()
            ));
        }
        let mut want = self.targets.clone();
        want.sort();
        for order in &s.orders {
            let mut sorted = order.clone();
            sorted.sort();
            if sorted != want {
                out.push(format!(
                    "order {order:?} is not a permutation of the targets"
                ));
            }
        }
        self.validate_sections(&mut out);
        if out.is_empty() {
            Ok(())
        } else {
            Err(Refusal(out))
        }
    }

    /// The version-specific sections. Version 1 carries none of them; a
    /// version 2 protocol carries every one, each held to what the code
    /// and the other sections say.
    fn validate_sections(&self, out: &mut Vec<String>) {
        let r = &self.resources;
        let has = [
            self.claim.is_some(),
            self.budgets.is_some(),
            self.products.is_some(),
            r.profile.is_some(),
            self.storage.profile.is_some(),
            self.storage
                .retention
                .iter()
                .any(|x| x.backend.is_some() || x.prune.is_some() || x.compression.is_some()),
        ];
        if self.protocol_version == DRAFT_PROTOCOL_VERSION {
            if has.iter().any(|h| *h) {
                out.push(format!(
                    "protocol_version {DRAFT_PROTOCOL_VERSION} carries version {PROTOCOL_VERSION} sections"
                ));
            }
            return;
        }
        let (Some(claim), Some(budgets), Some(products), Some(profile), Some(storage)) = (
            &self.claim,
            &self.budgets,
            &self.products,
            r.profile,
            self.storage.profile,
        ) else {
            out.push(format!(
                "protocol_version {PROTOCOL_VERSION} needs claim, budgets, products, resources.profile and storage.profile"
            ));
            return;
        };
        if claim.statement.trim().is_empty() {
            out.push("claim: no statement of what is and is not claimed".into());
        }
        match (r.enforced, profile) {
            (true, ResourceProfile::Enforced) | (false, ResourceProfile::UnenforcedLocal) => {}
            _ => out.push(format!(
                "resources: enforced {} cannot be profile {profile:?}",
                r.enforced
            )),
        }
        self.validate_retention(storage, out);
        self.validate_budgets(budgets, out);
        self.validate_products(products, out);
    }

    fn validate_retention(&self, profile: StorageProfile, out: &mut Vec<String>) {
        // `mixed` is the unmatched profile's mode: each target's own
        // backend is the declared difference.
        match (self.storage.mode.as_str(), profile) {
            ("memory", StorageProfile::MatchedMemory) | ("mixed", StorageProfile::Unmatched) => {}
            (mode, _) => out.push(format!(
                "storage: mode {mode:?} and profile {profile:?} disagree"
            )),
        }
        for e in &self.storage.retention {
            let t = e.target;
            let (Some(_), Some(prune), Some(compression)) =
                (e.backend, e.prune, e.compression.as_deref())
            else {
                out.push(format!(
                    "storage: {t} states no backend, prune behavior and compression"
                ));
                continue;
            };
            if compression.trim().is_empty() {
                out.push(format!("storage: {t} compression is blank"));
            }
            match (e.max_messages, prune) {
                (None, Prune::NoCapObserved) => {}
                (Some(_), Prune::OldestOnly | Prune::Unestablished) => {}
                _ => out.push(format!("storage: {t} cap and prune behavior disagree")),
            }
        }
        // False parity: a profile claiming one storage for all targets
        // while their backends or caps differ.
        let first = self.storage.retention.first();
        let same = self.storage.retention.iter().all(|e| {
            e.backend == Some(Backend::Memory)
                && first.is_some_and(|f| f.max_messages == e.max_messages)
        });
        if profile == StorageProfile::MatchedMemory && !same {
            out.push(
                "storage: matched_memory claims one memory backend and one cap, but the targets differ"
                    .into(),
            );
        }
    }

    fn validate_budgets(&self, b: &Budgets, out: &mut Vec<String>) {
        for w in &self.waits {
            for o in &w.observation {
                if o.mode == Observation::Poll && o.interval_ms != Some(b.poll_interval_ms) {
                    out.push(format!(
                        "{}: {} polls at {:?} ms, not the common {} ms; unequal polls are not a common method",
                        w.name, o.target, o.interval_ms, b.poll_interval_ms
                    ));
                }
            }
        }
        let longest = self
            .waits
            .iter()
            .map(|w| w.deadline_ms.max(w.empty_deadline_ms))
            .max()
            .unwrap_or(0);
        if b.wait_operation_ms != longest + self.resources.request_deadline_ms {
            out.push(format!(
                "budgets: wait_operation_ms {} is not the longest wait {longest} plus one request bound",
                b.wait_operation_ms
            ));
        }
        let built = crate::mailpit_mcp::Bounds::default();
        let a = &b.adapter;
        let ms = |d: std::time::Duration| d.as_millis() as u64;
        if (a.operation_ms, a.request_ms, a.max_wait_ms)
            != (ms(built.operation), ms(built.request), ms(built.max_wait()))
            || a.request_ms != self.resources.request_deadline_ms
        {
            out.push(
                "budgets: adapter operation, request and maximum wait are not what the adapter is built with"
                    .into(),
            );
        }
        if (a.max_pages, a.max_requests, a.max_operation_bytes)
            != (
                built.max_pages,
                built.max_requests,
                built.max_operation_bytes,
            )
        {
            out.push("budgets: adapter page, request and byte bounds are not what the adapter is built with".into());
        }
        if a.wait_method != crate::mailpit_mcp::WAIT_METHOD {
            out.push(format!(
                "budgets: adapter wait method {:?} is not {:?}",
                a.wait_method,
                crate::mailpit_mcp::WAIT_METHOD
            ));
        }
        let tools = serde_json::to_vec(&crate::mailpit_mcp::tools(&built)).unwrap_or_default();
        if a.tools_sha256 != sha256_hex(&tools) {
            out.push("budgets: adapter tools_sha256 is not the schema the adapter serves".into());
        }
    }

    fn validate_products(&self, products: &[Product], out: &mut Vec<String>) {
        for t in &self.targets {
            let mine: Vec<&Product> = products.iter().filter(|p| p.target == *t).collect();
            let [p] = mine.as_slice() else {
                out.push(format!("products: {t} needs exactly one product entry"));
                continue;
            };
            match (t.official_repository(), &p.release_tag, &p.executable_sha256) {
                (None, None, None) => {}
                (None, ..) => out.push(format!(
                    "products: {t} is the candidate; its identity is recorded per run, not pinned here"
                )),
                (Some(_), Some(tag), Some(sha)) => {
                    if !is_release_tag(tag) {
                        out.push(format!(
                            "products: {t} release tag {tag:?} is not an immutable version tag"
                        ));
                    }
                    if !is_hex(sha, 64) {
                        out.push(format!("products: {t} executable digest is not 64 hex digits"));
                    }
                }
                (Some(_), ..) => out.push(format!(
                    "products: {t} needs a release tag and an executable digest"
                )),
            }
        }
        for p in products {
            if !self.targets.contains(&p.target) {
                out.push(format!(
                    "products: entry for undeclared target {}",
                    p.target
                ));
            }
        }
    }

    /// The target order of repetition `rep` (0-based), counterbalanced by
    /// the protocol's own list.
    pub fn order(&self, rep: u32) -> &[TargetKind] {
        &self.statistics.orders[rep as usize % self.statistics.orders.len()]
    }
}

pub const LAUNCH_SLOTS: [&str; 3] = ["{smtp_addr}", "{http_addr}", "{data_dir}"];

/// The slots a launch config template may name (lock version 4): each
/// listener's `host:port`, host and port, plus the private data directory.
/// `www` is the admin listener a target may start whether asked or not
/// (MailSlurper); the launcher reserves its port only when a template names
/// it, so that listener is owned and its release is part of the receipt.
pub const CONFIG_SLOTS: [&str; 10] = [
    "{smtp_addr}",
    "{smtp_host}",
    "{smtp_port}",
    "{http_addr}",
    "{http_host}",
    "{http_port}",
    "{www_addr}",
    "{www_host}",
    "{www_port}",
    "{data_dir}",
];

/// The argv slot that stands for the config file the launcher writes.
pub const CONFIG_FILE_SLOT: &str = "{config_file}";

/// A target that reads its listeners from a file (decision 0129 §7): the
/// template is written into the trial's private scratch with every slot
/// filled, and the argv names the file through `{config_file}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchConfig {
    pub file_name: String,
    pub template: String,
}

/// A v4 launch is either the v3 argv template alone, or an argv that names
/// `{config_file}` beside a config template that carries the listener slots.
pub fn launch_ok(argv: &[String], config: Option<&LaunchConfig>) -> bool {
    let Some(c) = config else {
        return launch_template_ok(argv);
    };
    // A config template is JSON, so braces are expected; what is refused is
    // a `{name}` that is not a known slot.
    let clean = |s: &str| {
        let mut rest = s;
        while let Some(open) = rest.find('{') {
            rest = &rest[open..];
            let Some(close) = rest.find('}') else {
                return true;
            };
            let word = &rest[1..close];
            if !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
                if !CONFIG_SLOTS.contains(&&rest[..=close]) {
                    return false;
                }
                rest = &rest[close + 1..];
            } else {
                rest = &rest[1..];
            }
        }
        true
    };
    argv.len() <= 32
        && argv.iter().any(|a| a.contains(CONFIG_FILE_SLOT))
        && argv.iter().all(|a| {
            let rest = a.replace(CONFIG_FILE_SLOT, "");
            a.len() <= 256
                && !rest.contains(['{', '}'])
                && rest.chars().all(|c| c.is_ascii_graphic())
        })
        && is_name(&c.file_name)
        && c.template.len() <= 4096
        && clean(&c.template)
        && ["{smtp_", "{http_", "{www_"]
            .iter()
            .all(|slot| c.template.contains(slot))
        && c.template.contains("{data_dir}")
}

/// A launch template is short printable argv with every brace a known
/// slot, and both listener slots present (the driver must own the ports).
pub fn launch_template_ok(argv: &[String]) -> bool {
    let has = |slot: &str| argv.iter().any(|a| a.contains(slot));
    argv.len() <= 32
        && has("{smtp_addr}")
        && has("{http_addr}")
        && argv.iter().all(|a| {
            let mut rest = a.clone();
            for slot in LAUNCH_SLOTS {
                rest = rest.replace(slot, "");
            }
            a.len() <= 256
                && !rest.contains(['{', '}'])
                && rest.chars().all(|c| c.is_ascii_graphic())
        })
}

/// The permissive licenses a pinned experiment input may carry: the
/// subset of `deny.toml`'s allowlist a server release could use. The
/// binary is not linked, but its terms still have to be checkable.
const PERMISSIVE: [&str; 5] = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"];

/// Source lock version 3 (proposed decision 0093 §1). It keeps version 2's
/// separate artifact, digest-basis and document facts, and replaces v2's
/// `source_revision` — a built-from claim — with two facts that never fill
/// each other: `tagged_source`, the commit the official tag names through
/// its captured ref → tag → commit chain, and `binary_build_provenance`,
/// what the executable was built from, which only captured build evidence
/// resolves. Tag identity is source identity, never executable identity.
pub const LOCK_VERSION: u32 = 4;

/// A fact the lock either observed or names as missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fact<T> {
    Observed(T),
    Unresolved(String),
}

impl<T> Fact<T> {
    pub fn gap(&self) -> Option<&str> {
        match self {
            Fact::Observed(_) => None,
            Fact::Unresolved(r) => Some(r),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLock {
    pub lock_version: u32,
    /// The day the bytes below were checked, `YYYY-MM-DD`.
    pub checked_on: String,
    pub entries: Vec<SourceEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The candidate checkout itself: identity is the commit and the
    /// built binary's digest, both recorded per run in the manifest.
    Candidate,
    Competitor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEntry {
    pub target: TargetKind,
    pub role: Role,
    /// Required for a competitor, refused for the candidate.
    pub pin: Option<Pin>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub release: Release,
    pub acquisition: Acquisition,
    /// The commit the official tag names. Says nothing about the executable.
    pub tagged_source: Fact<TaggedSource>,
    /// What the executable was built from, with its captured evidence. No
    /// tag, branch, release `target_commitish` or source capture fills it.
    pub binary_build_provenance: Fact<BuildProvenance>,
    pub artifact: Artifact,
    pub license: License,
    pub documents: Vec<Document>,
    /// The public, fixed launch argv. `{smtp_addr}`, `{http_addr}` and
    /// `{data_dir}` are the only substitutions; the driver owns them.
    pub launch: Vec<String>,
    /// Lock version 4: the config file a target reads its listeners from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<LaunchConfig>,
    /// Lock version 4: whether the target takes the eval fixture's SMTP
    /// AUTH PLAIN, answered from its reviewed source and, separately, by a
    /// probe against the pinned executable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smtp_auth: Option<SmtpAuthFacts>,
    pub storage: StorageFacts,
    pub wait: WaitFacts,
}

/// How the captured bytes reached this lock. Captures are bound by digest,
/// not authenticated: nothing here proves who served them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acquisition {
    pub basis: AcquisitionBasis,
    /// The day the captures were taken, `YYYY-MM-DD`, as supplied.
    pub captured_on: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionBasis {
    /// Read-only GitHub API captures the host supplied; this lock's author
    /// made no network observation of its own.
    HostSuppliedCapture,
}

/// The official tag's chain, each link a captured document: the ref names
/// the annotated tag object, the tag object names the commit, and the
/// tree listing is that commit's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaggedSource {
    /// Absent (v3 and before) means an annotated tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_kind: Option<TagKind>,
    /// The annotated tag object; `null` exactly for a lightweight tag,
    /// which has none.
    pub tag_object: Option<String>,
    pub commit: String,
    /// The tag's signature status as the captured tag object reports it;
    /// `null` for a lightweight tag.
    pub tag_signature: Option<String>,
    /// A lightweight tag's tree: no commit object or tree listing is
    /// captured, so it stays unresolved with its reason and is never
    /// synthesised from a source archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<Fact<String>>,
    /// The source archive the tagged files were read from, when no tree
    /// listing attributes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_archive: Option<SourceArchive>,
}

/// The answer to one question, from two independent readings (decision
/// 0129 §2): the reviewed source, and a probe of the pinned executable. A
/// probe that could not run is `Unresolved` with its reason, never assumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmtpAuthFacts {
    pub answer: SmtpAuth,
    pub source_finding: String,
    pub probe: Fact<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtpAuth {
    /// The target accepts AUTH PLAIN.
    Plain,
    /// The target has no AUTH: the fixture leaves the step out.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    Annotated,
    Lightweight,
}

/// A source archive as the host supplied it: locally hashed, and carrying
/// the commit its pax comment names. The comment does not prove the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArchive {
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
    pub commit_comment: String,
}

/// Build evidence: the commit, and the `build_evidence` document saying so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildProvenance {
    pub commit: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub repository: String,
    pub tag: String,
    pub html_url: String,
    /// As the release metadata states it: a branch name, not a revision.
    pub target_commitish: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Archive,
    Executable,
    /// An executable the operator had built from the pinned, tag-named
    /// source (the 2026-11-07 ruling recorded in decision 0129): not the
    /// publisher's release artifact, so it carries its `build` and the
    /// `official_release` it stands in for.
    BuiltFromSource,
}

/// Where the asset digest comes from. Only `upstream_release_metadata`
/// is a digest the publisher stated; `locally_computed` is integrity of
/// the bytes the host supplied, not an upstream checksum or signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigestBasis {
    UpstreamReleaseMetadata,
    LocallyComputed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub kind: ArtifactKind,
    pub file_name: String,
    pub download: String,
    pub size: u64,
    /// Digest of the release asset as downloaded.
    pub sha256: String,
    pub digest_basis: DigestBasis,
    /// The digest the release metadata publishes; `None` exactly when the
    /// metadata publishes none.
    pub upstream_digest: Option<String>,
    /// The archive member that is the executable; `None` for a bare one.
    pub member: Option<String>,
    /// Digest of the executable launched; checked at every launch.
    pub executable_sha256: String,
    pub os: String,
    pub arch: String,
    /// The executable's own version report, as observed.
    pub version_output: String,
    /// Said when the executable's own report does not name the release tag
    /// (MailSlurper 1.15.0 logs v1.14.1), instead of fitting the report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_note: Option<String>,
    /// How a `built_from_source` executable was made; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<Build>,
    /// The publisher's own release artifact that a `built_from_source`
    /// executable stands in for, and why it cannot be measured; absent
    /// otherwise. No launcher reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub official_release: Option<OfficialRelease>,
}

/// The inputs of one build, as recorded. It is one build and not a
/// reproducible one: a rebuild that differs is a new pin, never a digest to
/// force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// The source archive built; equal to the tagged source archive's.
    pub source_sha256: String,
    /// The builder image, pinned by digest (`name@sha256:<64 hex>`).
    pub builder_image: String,
    pub toolchain: String,
    pub flags: Vec<String>,
}

/// The publisher's release artifact that does not start, kept as bytes: its
/// digests and the document holding its failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficialRelease {
    pub file_name: String,
    pub download: String,
    pub size: u64,
    pub sha256: String,
    pub member: String,
    pub member_sha256: String,
    pub exit_status: i32,
    /// The `build_evidence` document holding the failure's stderr.
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct License {
    pub spdx: String,
    pub holder: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    ReleaseMetadata,
    License,
    Api,
    Setup,
    /// A reviewed source body.
    Source,
    GitRef,
    GitTag,
    GitTree,
    /// A GitHub commit object: the link from a lightweight tag to its tree.
    GitCommit,
    BuildEvidence,
}

impl DocKind {
    /// A GitHub API object rather than a file of the tagged commit.
    fn is_api_object(self) -> bool {
        matches!(
            self,
            DocKind::ReleaseMetadata
                | DocKind::GitRef
                | DocKind::GitTag
                | DocKind::GitTree
                | DocKind::GitCommit
        )
    }
}

/// A captured document, committed beside the lock. `path` is relative to
/// the lock's own directory and confined to `sources/<target>/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub kind: DocKind,
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub attribution: Fact<Attribution>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
pub enum Attribution {
    /// `upstream_path` of the tagged commit, whose git blob id is `git_blob`.
    Commit {
        url: String,
        upstream_path: String,
        git_blob: String,
    },
    /// A GitHub REST object as it names itself at `url`.
    Api { url: String },
    /// `upstream_path` of the source archive whose digest is
    /// `archive_sha256`; `git_blob` is computed locally from the bytes, so
    /// it ties the capture to the archive and proves nothing about a tree.
    Archive {
        archive_sha256: String,
        upstream_path: String,
        git_blob: String,
    },
}

/// What was read to characterise a server-side wait, and what it shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitFacts {
    pub reviewed: Vec<ReviewedSource>,
    /// What the reviewed bodies establish and what they leave open.
    pub finding: String,
    /// The mechanism a best-available wait would use, with evidence that
    /// it misses nothing between registration and scan. While unresolved,
    /// only the labelled common-client poll observes this target.
    pub best_wait: Fact<String>,
}

/// A source body read for the wait finding, bound to the tagged tree by
/// its git blob id. `retained_as` is its committed `source` document, or
/// why its bytes are not retained here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSource {
    pub upstream_path: String,
    pub size: u64,
    pub sha256: String,
    pub git_blob: String,
    pub retained_as: Fact<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Memory,
    /// A file store inside the owned scratch: not memory, and nothing
    /// about its durability is claimed.
    ScratchFile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageFacts {
    pub backend: Backend,
    /// The retention cap as launched; `None` = no cap exists.
    pub max_messages: Option<u64>,
    /// The launch flag that sets the cap, whose value must equal it.
    pub cap_flag: Option<String>,
    /// How the backend and cap were established.
    pub basis: String,
}

impl SourceLock {
    pub fn parse(bytes: &[u8]) -> Result<SourceLock, Refusal> {
        #[derive(Deserialize)]
        struct Version {
            lock_version: u32,
        }
        let historical = match serde_json::from_slice::<Version>(bytes) {
            Ok(Version { lock_version: 1 }) => {
                "lock_version 1 is the historical unverified draft; only the v1 reader reads it, and nothing launches from it"
            }
            Ok(Version { lock_version: 2 }) => {
                "lock_version 2 is historical: its source_revision means built-from, so only the v2 reader reads it, and nothing launches from it"
            }
            _ => {
                return serde_json::from_slice(bytes)
                    .map_err(|e| Refusal(vec![format!("source lock does not parse: {e}")]));
            }
        };
        Err(Refusal(vec![historical.into()]))
    }

    pub fn entry(&self, target: TargetKind) -> Option<&SourceEntry> {
        self.entries.iter().find(|e| e.target == target)
    }

    /// The competitor pin a launch may use; `None` for the candidate or a
    /// target without one.
    pub fn pin(&self, target: TargetKind) -> Option<&Pin> {
        self.entry(target).and_then(|e| e.pin.as_ref())
    }

    /// Structural validity: every stated fact is well formed and every
    /// missing one carries a reason. Says nothing about the bytes on disk
    /// ([`SourceLock::verify_files`]) or collection ([`collection_ready`]).
    pub fn validate(&self) -> Result<(), Refusal> {
        self.validate_as(LOCK_VERSION, &TargetKind::ALL)
    }

    /// The one reader of lock versions 3 and 4: the version it holds the
    /// lock to and the targets that version knows. Version 4 is
    /// [`LOCK_VERSION`] with every [`TargetKind`]; [`v3::validate`] is the
    /// same reader with [`HISTORICAL_TARGETS`], so archived v3 bytes keep
    /// their meaning.
    pub fn validate_as(&self, version: u32, expected: &[TargetKind]) -> Result<(), Refusal> {
        let mut out = Vec::new();
        if self.lock_version != version {
            out.push(format!(
                "lock_version {} is not {version}",
                self.lock_version
            ));
        }
        validate_entries(&mut out, &self.checked_on, &self.entries, expected, |e| {
            (e.target, e.role, e.pin.is_some())
        });
        for e in &self.entries {
            if let (Role::Competitor, Some(pin)) = (e.role, &e.pin)
                && e.target != TargetKind::Swarmail
            {
                validate_pin(&mut out, e.target, pin);
            }
        }
        refusal(out)
    }

    /// Recheck every captured document against its size and digest; the
    /// release metadata against the pin; each API capture against the URL
    /// it names itself by; the ref → tag → commit chain; and every
    /// commit-attributed document and reviewed source against its git
    /// blob id in the captured tree. `dir` is the lock's own directory.
    /// Local consistency only, not authentication.
    pub fn verify_files(&self, dir: &std::path::Path) -> Result<(), Refusal> {
        let mut out = Vec::new();
        for e in &self.entries {
            let Some(pin) = &e.pin else { continue };
            let t = e.target;
            let mut json = std::collections::HashMap::new();
            for d in &pin.documents {
                let Some(bytes) = read_document(&mut out, t, dir, &d.path, &d.sha256) else {
                    continue;
                };
                if bytes.len() as u64 != d.size {
                    out.push(format!("{t}: {} is not {} bytes", d.path, d.size));
                }
                if let Fact::Observed(
                    Attribution::Commit { git_blob, .. } | Attribution::Archive { git_blob, .. },
                ) = &d.attribution
                    && git_blob_id(&bytes) != *git_blob
                {
                    out.push(format!(
                        "{t}: {} is not the git blob its attribution names",
                        d.path
                    ));
                }
                if d.kind == DocKind::License {
                    license_text(&mut out, t, &pin.license, &d.path, &bytes);
                }
                if !d.kind.is_api_object() {
                    continue;
                }
                let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                    out.push(format!("{t}: {} does not parse", d.path));
                    continue;
                };
                if let Fact::Observed(Attribution::Api { url }) = &d.attribution
                    && v.get("url").and_then(|u| u.as_str()) != Some(url.as_str())
                {
                    out.push(format!("{t}: {} does not name itself {url}", d.path));
                }
                if d.kind == DocKind::ReleaseMetadata {
                    release_agrees(&mut out, t, &pin.release, &pin.artifact, &v);
                }
                json.insert(d.kind, v);
            }
            if let Fact::Observed(ts) = &pin.tagged_source {
                tag_chain(&mut out, t, pin, ts, &json);
            }
        }
        refusal(out)
    }
}

fn refusal(out: Vec<String>) -> Result<(), Refusal> {
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}

/// The checks lock versions 2 and 3 share: the date, one entry per
/// target, and the roles. `view` is `(target, role, has a pin)`.
fn validate_entries<E>(
    out: &mut Vec<String>,
    checked_on: &str,
    entries: &[E],
    expected: &[TargetKind],
    view: impl Fn(&E) -> (TargetKind, Role, bool),
) {
    if !is_date(checked_on) {
        out.push(format!("checked_on {checked_on:?} is not YYYY-MM-DD"));
    }
    for t in TargetKind::ALL {
        let n = entries.iter().filter(|e| view(e).0 == t).count();
        if n > 1 {
            out.push(format!("{t} has {n} lock entries"));
        }
        if n > 0 && !expected.contains(&t) {
            out.push(format!("{t} is not a target of this lock version"));
        }
    }
    for e in entries {
        match view(e) {
            (TargetKind::Swarmail, Role::Candidate, false) => {}
            (TargetKind::Swarmail, Role::Candidate, true) => out.push(
                "swarmail is the candidate; its identity is recorded per run, not pinned".into(),
            ),
            (t, Role::Candidate, _) => out.push(format!("{t} cannot be the candidate")),
            (TargetKind::Swarmail, Role::Competitor, _) => {
                out.push("swarmail's entry must be the candidate".into())
            }
            (t, Role::Competitor, false) => out.push(format!("{t}: a competitor needs a pin")),
            (_, Role::Competitor, true) => {}
        }
    }
}

/// A captured document's bytes, if present, bounded and matching `sha256`.
fn read_document(
    out: &mut Vec<String>,
    t: TargetKind,
    dir: &std::path::Path,
    path: &str,
    sha256: &str,
) -> Option<Vec<u8>> {
    match std::fs::read(dir.join(path)) {
        Ok(b) if b.len() as u64 > MAX_DOCUMENT_BYTES => {
            out.push(format!("{t}: {path} is oversized"));
            None
        }
        Ok(b) if sha256_hex(&b) != sha256 => {
            out.push(format!("{t}: {path} does not match its digest"));
            None
        }
        Ok(b) => Some(b),
        Err(_) => {
            out.push(format!("{t}: {path} is missing"));
            None
        }
    }
}

fn license_text(out: &mut Vec<String>, t: TargetKind, l: &License, path: &str, bytes: &[u8]) {
    if l.spdx == "MIT" && !String::from_utf8_lossy(bytes).contains("MIT License") {
        out.push(format!("{t}: {path} is not an MIT license text"));
    }
}

/// The ref names the tag object; the tag object names the tag, the commit
/// and its stated signature status; and the untruncated tree listing of
/// that commit holds every commit-attributed document and reviewed source
/// at its path with its git blob id and size. The tree capture is asked
/// for by commit, so its root tree object id is not a captured fact.
fn tag_chain(
    out: &mut Vec<String>,
    t: TargetKind,
    pin: &Pin,
    ts: &TaggedSource,
    json: &std::collections::HashMap<DocKind, serde_json::Value>,
) {
    let s = |v: &serde_json::Value, p: &str| {
        v.pointer(p)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let tag = &pin.release.tag;
    if ts.tag_kind == Some(TagKind::Lightweight) {
        // A lightweight tag is a ref straight to a commit: there is no tag
        // object to capture, and no tree listing was captured either.
        let ref_ok = json.get(&DocKind::GitRef).is_some_and(|r| {
            s(r, "/ref") == format!("refs/tags/{tag}")
                && s(r, "/object/type") == "commit"
                && s(r, "/object/sha") == ts.commit
        });
        if !ref_ok {
            out.push(format!(
                "{t}: the captured ref does not name commit {} for a lightweight tag",
                ts.commit
            ));
        }
        if let Some(Fact::Observed(tree)) = &ts.tree {
            lightweight_tree(out, t, pin, ts, tree, json);
        }
        return;
    }
    let tag_object = ts.tag_object.clone().unwrap_or_default();
    let tag_signature = ts.tag_signature.clone().unwrap_or_default();
    let ref_ok = json.get(&DocKind::GitRef).is_some_and(|r| {
        s(r, "/ref") == format!("refs/tags/{tag}")
            && s(r, "/object/type") == "tag"
            && s(r, "/object/sha") == tag_object
    });
    if !ref_ok {
        out.push(format!(
            "{t}: the captured ref does not name tag object {tag_object}"
        ));
    }
    let tag_ok = json.get(&DocKind::GitTag).is_some_and(|g| {
        s(g, "/sha") == tag_object
            && s(g, "/tag") == *tag
            && s(g, "/object/type") == "commit"
            && s(g, "/object/sha") == ts.commit
            && s(g, "/verification/reason") == tag_signature
    });
    if !tag_ok {
        out.push(format!(
            "{t}: the captured tag object does not name {tag}, commit {} and signature {:?}",
            ts.commit, tag_signature
        ));
    }
    let tree = json
        .get(&DocKind::GitTree)
        .filter(|tr| {
            s(tr, "/sha") == ts.commit
                && tr.get("truncated").and_then(|x| x.as_bool()) == Some(false)
        })
        .and_then(|tr| tr.get("tree").and_then(|x| x.as_array()));
    let Some(tree) = tree else {
        out.push(format!(
            "{t}: the captured tree is not the complete listing of commit {}",
            ts.commit
        ));
        return;
    };
    let holds = |path: &str, blob: &str, size: u64| {
        tree.iter().any(|e| {
            s(e, "/path") == path
                && s(e, "/type") == "blob"
                && s(e, "/sha") == blob
                && e.get("size").and_then(|x| x.as_u64()) == Some(size)
        })
    };
    for d in &pin.documents {
        if let Fact::Observed(Attribution::Commit {
            upstream_path,
            git_blob,
            ..
        }) = &d.attribution
            && !holds(upstream_path, git_blob, d.size)
        {
            out.push(format!(
                "{t}: {} is not {upstream_path} in the tagged tree",
                d.path
            ));
        }
    }
    for r in &pin.wait.reviewed {
        if !holds(&r.upstream_path, &r.git_blob, r.size) {
            out.push(format!(
                "{t}: reviewed source {} is not in the tagged tree",
                r.upstream_path
            ));
        }
    }
}

/// A lightweight tag's observed root tree: the captured commit object names
/// the commit and the tree, the captured listing is that tree and is
/// complete, the tree object rebuilt from the listing's own entries has that
/// id (so the listing is exact, not merely labelled), and every root-level
/// capture has the blob the tree names for it.
fn lightweight_tree(
    out: &mut Vec<String>,
    t: TargetKind,
    pin: &Pin,
    ts: &TaggedSource,
    tree: &str,
    json: &std::collections::HashMap<DocKind, serde_json::Value>,
) {
    let s = |v: &serde_json::Value, p: &str| {
        v.pointer(p)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let commit_ok = json
        .get(&DocKind::GitCommit)
        .is_some_and(|c| s(c, "/sha") == ts.commit && s(c, "/tree/sha") == tree);
    if !commit_ok {
        out.push(format!(
            "{t}: the captured commit object does not name commit {} and tree {tree}",
            ts.commit
        ));
    }
    let listing = json
        .get(&DocKind::GitTree)
        .filter(|l| {
            s(l, "/sha") == tree && l.get("truncated").and_then(|x| x.as_bool()) == Some(false)
        })
        .and_then(|l| l.get("tree").and_then(|x| x.as_array()));
    let Some(listing) = listing else {
        out.push(format!(
            "{t}: the captured tree is not the complete listing of tree {tree}"
        ));
        return;
    };
    match git_tree_id(listing) {
        Some(id) if id == tree => {}
        Some(id) => out.push(format!(
            "{t}: the captured listing hashes to tree {id}, not {tree}"
        )),
        None => out.push(format!("{t}: the captured listing is not a tree's entries")),
    }
    for d in &pin.documents {
        if let Fact::Observed(Attribution::Archive {
            upstream_path,
            git_blob,
            ..
        }) = &d.attribution
            && !upstream_path.contains('/')
        {
            let named = listing.iter().any(|e| {
                s(e, "/path") == *upstream_path
                    && s(e, "/type") == "blob"
                    && s(e, "/sha") == *git_blob
            });
            if !named {
                out.push(format!(
                    "{t}: {} is not the blob the root tree names for {upstream_path}",
                    d.path
                ));
            }
        }
    }
}

/// Git's tree object id from a GitHub tree listing's entries (in the order
/// the listing gives them, which is git's): SHA-1 over
/// `tree <len>\0` and, per entry, `<mode> <name>\0<20 raw bytes>`. `None`
/// when an entry is not shaped like one.
pub fn git_tree_id(entries: &[serde_json::Value]) -> Option<String> {
    let mut body = Vec::new();
    for e in entries {
        let mode = e.get("mode")?.as_str()?.trim_start_matches('0');
        let path = e.get("path")?.as_str()?;
        let sha = e.get("sha")?.as_str()?;
        if !is_hex(sha, 40) || mode.is_empty() || path.is_empty() {
            return None;
        }
        body.extend_from_slice(mode.as_bytes());
        body.push(b' ');
        body.extend_from_slice(path.as_bytes());
        body.push(0);
        for i in 0..20 {
            body.push(u8::from_str_radix(&sha[2 * i..2 * i + 2], 16).ok()?);
        }
    }
    let mut m = format!("tree {}\0", body.len()).into_bytes();
    m.extend_from_slice(&body);
    Some(sha1(&m).iter().map(|b| format!("{b:02x}")).collect())
}

/// The captured documents are small text; a larger file is not one.
pub const MAX_DOCUMENT_BYTES: u64 = 1 << 20;

/// SHA-256 through the rustls/ring provider already in the tree — the
/// bench adds no hashing crate.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let rustls::SupportedCipherSuite::Tls13(suite) =
        rustls::crypto::ring::cipher_suite::TLS13_AES_128_GCM_SHA256
    else {
        unreachable!("a TLS 1.3 suite")
    };
    let out = suite.common.hash_provider.hash(bytes);
    out.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Git's blob id of `bytes`: SHA-1 over `blob <len>\0<bytes>`. It ties a
/// captured body to the captured tree listing. SHA-1 is git's object
/// naming here, an integrity cross-check; SHA-256 is the digest of record.
pub fn git_blob_id(bytes: &[u8]) -> String {
    let mut m = format!("blob {}\0", bytes.len()).into_bytes();
    m.extend_from_slice(bytes);
    sha1(&m).iter().map(|b| format!("{b:02x}")).collect()
}

/// FIPS 180-4 SHA-1, only for git object ids (see [`git_blob_id`]); the
/// tree has no SHA-1 crate and this adds none.
pub fn sha1(msg: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut data = msg.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&((msg.len() as u64) * 8).to_be_bytes());
    for block in data.as_chunks::<64>().0 {
        let mut w = [0u32; 80];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            (e, d, c, b, a) = (d, c, b.rotate_left(30), a, next);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 20];
    for (chunk, x) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
        chunk.copy_from_slice(&x.to_be_bytes());
    }
    out
}

fn release_agrees(
    out: &mut Vec<String>,
    t: TargetKind,
    r: &Release,
    a: &Artifact,
    v: &serde_json::Value,
) {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str());
    for (field, want) in [
        ("tag_name", &r.tag),
        ("html_url", &r.html_url),
        ("target_commitish", &r.target_commitish),
        ("published_at", &r.published_at),
    ] {
        if s(field) != Some(want.as_str()) {
            out.push(format!(
                "{t}: release metadata {field} disagrees with the pin"
            ));
        }
    }
    // A source build has no published binary: the release lists the
    // official asset it stands in for.
    let (file_name, download, size) = match &a.official_release {
        Some(o) if a.kind == ArtifactKind::BuiltFromSource => (&o.file_name, &o.download, o.size),
        _ => (&a.file_name, &a.download, a.size),
    };
    let asset = v
        .get("assets")
        .and_then(|x| x.as_array())
        .into_iter()
        .flatten()
        .find(|x| x.get("name").and_then(|n| n.as_str()) == Some(file_name.as_str()));
    let Some(asset) = asset else {
        out.push(format!("{t}: release metadata lists no asset {file_name}"));
        return;
    };
    if asset.get("browser_download_url").and_then(|x| x.as_str()) != Some(download.as_str())
        || asset.get("size").and_then(|x| x.as_u64()) != Some(size)
    {
        out.push(format!(
            "{t}: release asset URL or size disagrees with the pin"
        ));
    }
    // A missing key and an explicit null both mean "published none".
    let published = match asset.get("digest") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(d)) => Ok(Some(d.as_str())),
        Some(_) => Err(()),
    };
    let want = a.upstream_digest.as_ref().map(|d| format!("sha256:{d}"));
    if published != Ok(want.as_deref()) {
        out.push(format!(
            "{t}: the release metadata's published digest disagrees with the pin's digest basis"
        ));
    }
}

/// A repository-relative path: no root, no `.`/`..` segment, printable.
fn is_upstream_path(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 256
        && p.split('/').all(|s| {
            !s.is_empty() && s != "." && s != ".." && s.bytes().all(|b| b.is_ascii_graphic())
        })
}

/// The pin facts lock versions 2 and 3 share: release, artifact and its
/// digest basis, license, launch and storage.
fn validate_common(
    out: &mut Vec<String>,
    t: TargetKind,
    (r, a, license): (&Release, &Artifact, &License),
    (launch, launch_ok): (&[String], bool),
    st: &StorageFacts,
) {
    let official = t.official_repository().unwrap_or_default();
    if r.repository != official {
        out.push(format!(
            "{t}: source {} is not the official {official}",
            r.repository
        ));
    }
    if r.tag.is_empty() || r.tag.len() > 64 || !r.tag.chars().all(|c| c.is_ascii_graphic()) {
        out.push(format!("{t}: release tag {:?} is malformed", r.tag));
    }
    if r.html_url != format!("{official}/releases/tag/{}", r.tag) {
        out.push(format!(
            "{t}: release page {:?} is not the official tag page",
            r.html_url
        ));
    }
    if r.target_commitish.trim().is_empty() {
        out.push(format!("{t}: release target_commitish is not recorded"));
    }
    if !r.published_at.get(..10).is_some_and(is_date) {
        out.push(format!(
            "{t}: published_at {:?} is not a date",
            r.published_at
        ));
    }
    // A source build has no published binary: its download is the release page.
    let official_download = match a.kind {
        ArtifactKind::BuiltFromSource => r.html_url.clone(),
        _ => format!("{official}/releases/download/{}/{}", r.tag, a.file_name),
    };
    if a.download != official_download {
        out.push(format!(
            "{t}: download {:?} is not the official release asset",
            a.download
        ));
    }
    if !is_name(&a.file_name) {
        out.push(format!("{t}: artifact file name is malformed"));
    }
    if !is_hex(&a.sha256, 64) || !is_hex(&a.executable_sha256, 64) {
        out.push(format!(
            "{t}: artifact digests must be 64 lowercase hex digits"
        ));
    }
    match (a.digest_basis, &a.upstream_digest) {
        (DigestBasis::UpstreamReleaseMetadata, Some(d)) if *d == a.sha256 => {}
        (DigestBasis::UpstreamReleaseMetadata, _) => out.push(format!(
            "{t}: an upstream digest basis needs the published digest, equal to the asset's"
        )),
        (DigestBasis::LocallyComputed, None) => {}
        (DigestBasis::LocallyComputed, Some(_)) => out.push(format!(
            "{t}: a locally computed digest cannot carry an upstream digest"
        )),
    }
    match (a.kind, &a.member) {
        (ArtifactKind::Executable, None) if a.executable_sha256 == a.sha256 => {}
        (ArtifactKind::Executable, _) => out.push(format!(
            "{t}: a bare executable asset has no member and one digest"
        )),
        (ArtifactKind::Archive, Some(m)) if is_name(m) => {}
        (ArtifactKind::Archive, _) => out.push(format!("{t}: an archive pin names its member")),
        (ArtifactKind::BuiltFromSource, None)
            if a.executable_sha256 == a.sha256
                && a.digest_basis == DigestBasis::LocallyComputed
                && a.upstream_digest.is_none() => {}
        (ArtifactKind::BuiltFromSource, _) => out.push(format!(
            "{t}: a source-built executable has no member, one digest, a locally computed basis and no upstream digest"
        )),
    }
    match (a.kind, &a.build, &a.official_release) {
        (ArtifactKind::BuiltFromSource, Some(_), Some(_)) | (_, None, None) => {}
        (ArtifactKind::BuiltFromSource, _, _) => out.push(format!(
            "{t}: a source-built executable needs its build record and the official release it stands in for"
        )),
        _ => out.push(format!(
            "{t}: a build record and an official release belong to a source-built executable only"
        )),
    }
    if a.os.trim().is_empty() || a.arch.trim().is_empty() {
        out.push(format!("{t}: artifact platform is incomplete"));
    }
    let named = a.version_output.contains(r.tag.trim_start_matches('v'));
    match &a.version_note {
        // The report disagrees with the tag, and the lock says so.
        Some(note) if !named => {
            if note.trim().is_empty() || a.version_output.trim().is_empty() {
                out.push(format!(
                    "{t}: a version report that does not name {} needs its note and the observed text",
                    r.tag
                ));
            }
        }
        Some(_) => out.push(format!(
            "{t}: a version note is for a report that does not name {}",
            r.tag
        )),
        None if !named => out.push(format!(
            "{t}: the observed version output does not name {}",
            r.tag
        )),
        None => {}
    }
    if !PERMISSIVE.contains(&license.spdx.as_str()) {
        out.push(format!(
            "{t}: license {:?} is not on the permissive list",
            license.spdx
        ));
    }
    if license.holder.trim().is_empty() {
        out.push(format!("{t}: license holder is not recorded"));
    }
    if !launch_ok {
        out.push(format!("{t}: launch template is missing or malformed"));
    }
    match (st.max_messages, &st.cap_flag) {
        (Some(n), Some(flag)) => {
            if !launch
                .windows(2)
                .any(|w| w[0] == *flag && w[1] == n.to_string())
            {
                out.push(format!("{t}: the launch does not set {flag} {n}"));
            }
        }
        (None, None) => {}
        _ => out.push(format!(
            "{t}: a retention cap and its launch flag go together"
        )),
    }
    if st.basis.trim().is_empty() {
        out.push(format!("{t}: storage facts state no basis"));
    }
}

/// A document path confined to `sources/<target>/`, listed once, with a
/// well-formed digest.
fn document_path(out: &mut Vec<String>, t: TargetKind, earlier: &[&str], path: &str, sha: &str) {
    let prefix = format!("sources/{t}/");
    // A flat name, or one directory (`build/`, `git/`) holding a flat name.
    let confined = |rest: &str| {
        let parts: Vec<&str> = rest.split('/').collect();
        matches!(parts.len(), 1 | 2)
            && parts
                .iter()
                .all(|p| is_name(p) && !p.bytes().all(|b| b == b'.'))
    };
    if !path.strip_prefix(&prefix).is_some_and(confined) {
        out.push(format!("{t}: document path {path:?} is not under {prefix}"));
    }
    if earlier.contains(&path) {
        out.push(format!("{t}: document {path} is listed twice"));
    }
    if !is_hex(sha, 64) {
        out.push(format!("{t}: {path} digest is not 64 lowercase hex digits"));
    }
}

fn validate_pin(out: &mut Vec<String>, t: TargetKind, pin: &Pin) {
    validate_common(
        out,
        t,
        (&pin.release, &pin.artifact, &pin.license),
        (&pin.launch, launch_ok(&pin.launch, pin.config.as_ref())),
        &pin.storage,
    );
    let official = t.official_repository().unwrap_or_default();
    if !is_date(&pin.acquisition.captured_on) {
        out.push(format!(
            "{t}: captured_on {:?} is not YYYY-MM-DD",
            pin.acquisition.captured_on
        ));
    }
    let has = |kind: DocKind| pin.documents.iter().any(|d| d.kind == kind);
    if let Some(a) = &pin.smtp_auth {
        if a.source_finding.trim().is_empty() {
            out.push(format!("{t}: the SMTP AUTH answer cites no source finding"));
        }
        match &a.probe {
            Fact::Observed(p) if p.trim().is_empty() => {
                out.push(format!("{t}: the SMTP AUTH probe result is empty"))
            }
            Fact::Observed(_) => {}
            Fact::Unresolved(why) => gap(out, t, "smtp_auth probe", why),
        }
    }
    let commit = match &pin.tagged_source {
        Fact::Observed(ts) if ts.tag_kind == Some(TagKind::Lightweight) => {
            validate_lightweight(out, t, pin, ts);
            // No tree is captured, so no file can be attributed by commit.
            None
        }
        Fact::Observed(ts) => {
            let object = ts.tag_object.as_deref().unwrap_or_default();
            if !is_hex(object, 40) || !is_hex(&ts.commit, 40) {
                out.push(format!(
                    "{t}: tagged source needs a full tag object and commit id (a tag name is mutable)"
                ));
            }
            if ts
                .tag_signature
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                out.push(format!("{t}: the tag's signature status is not recorded"));
            }
            if ts.tree.is_some() || ts.source_archive.is_some() {
                out.push(format!(
                    "{t}: an annotated tag's tree is its captured listing, not an archive"
                ));
            }
            for kind in [DocKind::GitRef, DocKind::GitTag, DocKind::GitTree] {
                if !has(kind) {
                    out.push(format!(
                        "{t}: tagged source has no captured {kind:?} document"
                    ));
                }
            }
            Some(ts.commit.as_str())
        }
        Fact::Unresolved(why) => {
            gap(out, t, "tagged_source", why);
            None
        }
    };
    match &pin.binary_build_provenance {
        Fact::Observed(bp) => {
            if !is_hex(&bp.commit, 40) {
                out.push(format!(
                    "{t}: build provenance {:?} is not a full commit id",
                    bp.commit
                ));
            }
            if !pin
                .documents
                .iter()
                .any(|d| d.kind == DocKind::BuildEvidence && d.path == bp.evidence)
            {
                out.push(format!(
                    "{t}: build provenance cites no captured build evidence; a tag, branch or source capture is not one"
                ));
            }
        }
        Fact::Unresolved(why) if pin.artifact.kind == ArtifactKind::BuiltFromSource => {
            out.push(format!(
                "{t}: a source-built executable's build provenance must be observed, not unresolved ({why})"
            ))
        }
        Fact::Unresolved(why) => gap(out, t, "binary_build_provenance", why),
    }
    validate_source_build(out, t, pin);
    let api_home = format!(
        "https://api.github.com/repos/{}/",
        official.trim_start_matches("https://github.com/")
    );
    let mut earlier = Vec::new();
    for d in &pin.documents {
        document_path(out, t, &earlier, &d.path, &d.sha256);
        earlier.push(d.path.as_str());
        if d.size == 0 || d.size > MAX_DOCUMENT_BYTES {
            out.push(format!("{t}: {} size {} is out of bounds", d.path, d.size));
        }
        match &d.attribution {
            Fact::Observed(Attribution::Api { url }) => {
                if !d.kind.is_api_object() {
                    out.push(format!(
                        "{t}: {} is a repository file; an API object cannot attribute it",
                        d.path
                    ));
                }
                if !url.starts_with(&api_home) {
                    out.push(format!(
                        "{t}: {} is not attributed to the official repository's API",
                        d.path
                    ));
                }
            }
            Fact::Observed(Attribution::Commit {
                url,
                upstream_path,
                git_blob,
            }) => {
                if d.kind.is_api_object() {
                    out.push(format!(
                        "{t}: {} is an API object, not a file of the tagged commit",
                        d.path
                    ));
                }
                if !is_upstream_path(upstream_path) || !is_hex(git_blob, 40) {
                    out.push(format!(
                        "{t}: {} attribution needs a repository path and a full git blob id",
                        d.path
                    ));
                }
                match commit {
                    Some(c) if *url == format!("{official}/blob/{c}/{upstream_path}") => {}
                    Some(_) => out.push(format!(
                        "{t}: {} is not attributed to its path at the tagged commit",
                        d.path
                    )),
                    None => out.push(format!(
                        "{t}: {} cites a commit the lock does not resolve",
                        d.path
                    )),
                }
            }
            Fact::Observed(Attribution::Archive {
                archive_sha256,
                upstream_path,
                git_blob,
            }) => {
                if d.kind.is_api_object() {
                    out.push(format!(
                        "{t}: {} is an API object, not a file of the source archive",
                        d.path
                    ));
                }
                if !is_upstream_path(upstream_path) || !is_hex(git_blob, 40) {
                    out.push(format!(
                        "{t}: {} attribution needs an archive path and a full git blob id",
                        d.path
                    ));
                }
                let archive = match &pin.tagged_source {
                    Fact::Observed(ts) => ts.source_archive.as_ref(),
                    Fact::Unresolved(_) => None,
                };
                if archive.is_none_or(|a| a.sha256 != *archive_sha256) {
                    out.push(format!(
                        "{t}: {} cites a source archive the lock does not carry",
                        d.path
                    ));
                }
            }
            Fact::Unresolved(why) => gap(out, t, &d.path, why),
        }
    }
    for kind in [DocKind::ReleaseMetadata, DocKind::License, DocKind::Api] {
        if !has(kind) {
            out.push(format!("{t}: no captured {kind:?} document"));
        }
    }
    let w = &pin.wait;
    if w.finding.trim().is_empty() {
        out.push(format!("{t}: the wait review states no finding"));
    }
    for (i, r) in w.reviewed.iter().enumerate() {
        let p = &r.upstream_path;
        if !is_upstream_path(p) || !is_hex(&r.sha256, 64) || !is_hex(&r.git_blob, 40) {
            out.push(format!("{t}: reviewed source {p:?} is malformed"));
        }
        if w.reviewed[..i].iter().any(|o| o.upstream_path == *p) {
            out.push(format!("{t}: reviewed source {p} is listed twice"));
        }
        match &r.retained_as {
            Fact::Observed(path) => {
                if !pin.documents.iter().any(|d| {
                    d.kind == DocKind::Source
                        && d.path == *path
                        && d.sha256 == r.sha256
                        && d.size == r.size
                }) {
                    out.push(format!(
                        "{t}: reviewed source {p} names no retained source document with its digest"
                    ));
                }
            }
            Fact::Unresolved(why) => gap(out, t, p, why),
        }
    }
    match &w.best_wait {
        Fact::Observed(m) if m.trim().is_empty() => {
            out.push(format!("{t}: the best wait names no mechanism"))
        }
        Fact::Observed(_) => {}
        Fact::Unresolved(why) => gap(out, t, "best_wait", why),
    }
}

/// The source-built artifact's own records (decision 0129, 2026-11-07
/// addendum): the build's inputs, tied to the tagged source archive, and the
/// official release it stands in for, with its failure held as a captured
/// document.
fn validate_source_build(out: &mut Vec<String>, t: TargetKind, pin: &Pin) {
    let a = &pin.artifact;
    let evidence = |path: &str| {
        pin.documents
            .iter()
            .any(|d| d.kind == DocKind::BuildEvidence && d.path == path)
    };
    if let Some(b) = &a.build {
        let archive = match &pin.tagged_source {
            Fact::Observed(ts) => ts.source_archive.as_ref(),
            Fact::Unresolved(_) => None,
        };
        if !is_hex(&b.source_sha256, 64) || archive.is_none_or(|x| x.sha256 != b.source_sha256) {
            out.push(format!(
                "{t}: the build's source_sha256 is not the tagged source archive's digest"
            ));
        }
        let pinned_image = b
            .builder_image
            .split_once("@sha256:")
            .is_some_and(|(n, d)| {
                !n.is_empty() && !n.contains(char::is_whitespace) && is_hex(d, 64)
            });
        if !pinned_image {
            out.push(format!(
                "{t}: the builder image is not pinned by an @sha256: digest"
            ));
        }
        if b.toolchain.trim().is_empty() || b.flags.is_empty() {
            out.push(format!("{t}: the build records no toolchain or flags"));
        }
    }
    if let Some(o) = &a.official_release {
        if !is_name(&o.file_name)
            || !is_name(&o.member)
            || o.size == 0
            || !is_hex(&o.sha256, 64)
            || !is_hex(&o.member_sha256, 64)
            || o.exit_status == 0
        {
            out.push(format!(
                "{t}: the official release record needs names, a size, two digests and a failing exit status"
            ));
        }
        let official = t.official_repository().unwrap_or_default();
        if o.download
            != format!(
                "{official}/releases/download/{}/{}",
                pin.release.tag, o.file_name
            )
        {
            out.push(format!(
                "{t}: the official release download is not the official release asset"
            ));
        }
        if !evidence(&o.evidence) {
            out.push(format!(
                "{t}: the official release's failure cites no captured build evidence"
            ));
        }
    }
}

fn gap(out: &mut Vec<String>, t: TargetKind, what: &str, why: &str) {
    if why.trim().is_empty() {
        out.push(format!("{t}: {what} is unresolved without a reason"));
    }
}

/// A lightweight tag read from a source archive (lock version 4): the ref
/// names the commit directly, the archive is bound by digest, and the tree
/// stays unresolved with its reason. A tree synthesised from the archive,
/// a tag object or a captured tag/tree document for this shape is refused.
fn validate_lightweight(out: &mut Vec<String>, t: TargetKind, pin: &Pin, ts: &TaggedSource) {
    if ts.tag_object.is_some() || ts.tag_signature.is_some() {
        out.push(format!(
            "{t}: a lightweight tag has no tag object and no signature status"
        ));
    }
    if !is_hex(&ts.commit, 40) {
        out.push(format!(
            "{t}: tagged source needs a full commit id (a tag name is mutable)"
        ));
    }
    // The tree is observed only when a captured commit object names it and
    // a captured, complete listing is it; otherwise it stays unresolved with
    // its reason. It is never synthesised from a source archive.
    let docs = |kind: DocKind| pin.documents.iter().any(|d| d.kind == kind);
    let observed = matches!(&ts.tree, Some(Fact::Observed(_)));
    match &ts.tree {
        Some(Fact::Unresolved(why)) => gap(out, t, "tree", why),
        Some(Fact::Observed(tree)) => {
            if !is_hex(tree, 40) {
                out.push(format!("{t}: the observed tree is not a full tree id"));
            }
            if !docs(DocKind::GitCommit) || !docs(DocKind::GitTree) {
                out.push(format!(
                    "{t}: an observed tree needs the git_commit and git_tree documents that bind it; no tree is synthesised from an archive"
                ));
            }
        }
        None => out.push(format!(
            "{t}: a lightweight tag's tree is observed with its git_commit and git_tree documents, or unresolved with its reason"
        )),
    }
    match &ts.source_archive {
        Some(a) => {
            if !is_name(&a.file_name)
                || !is_hex(&a.sha256, 64)
                || a.size == 0
                || a.commit_comment != ts.commit
            {
                out.push(format!(
                    "{t}: the source archive needs a name, a size, a digest and the comment naming the tagged commit"
                ));
            }
        }
        None => out.push(format!("{t}: a lightweight tag needs its source archive")),
    }
    for d in &pin.documents {
        let tree_doc = matches!(d.kind, DocKind::GitTree | DocKind::GitCommit);
        if d.kind == DocKind::GitTag || (tree_doc && !observed) {
            out.push(format!(
                "{t}: {} cannot exist for a lightweight tag whose tree is unresolved",
                d.path
            ));
        }
        // Files are read from the archive; a commit attribution would claim
        // a recursive listing that was never captured.
        if matches!(d.attribution, Fact::Observed(Attribution::Commit { .. })) {
            out.push(format!(
                "{t}: {} cannot be attributed by commit while the tree is unresolved",
                d.path
            ));
        }
    }
    if !pin.documents.iter().any(|d| d.kind == DocKind::GitRef) {
        out.push(format!(
            "{t}: tagged source has no captured GitRef document"
        ));
    }
}

/// Every declared target's retention, as the protocol states it, equals
/// the cap its pin launches with, and (version 2) the backend the lock
/// establishes. A default the protocol did not choose (Mailpit keeps 500
/// unless told otherwise) can never slip in.
pub fn retention_matches(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    for r in &p.storage.retention {
        if let Some(pin) = lock.pin(r.target) {
            if pin.storage.max_messages != r.max_messages {
                out.push(format!(
                    "{}: protocol retention {:?} is not the launched cap {:?}",
                    r.target, r.max_messages, pin.storage.max_messages
                ));
            }
            if r.backend.is_some_and(|b| b != pin.storage.backend) {
                out.push(format!(
                    "{}: protocol backend {:?} is not the locked {:?}",
                    r.target,
                    r.backend.unwrap_or(pin.storage.backend),
                    pin.storage.backend
                ));
            }
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}

/// Every declared competitor product is the lock's release tag and
/// executable digest: a protocol cannot name a different product than the
/// bytes the lock verifies.
pub fn products_match(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    for product in p.products.iter().flatten() {
        let Some(pin) = lock.pin(product.target) else {
            continue;
        };
        if product.release_tag.as_deref() != Some(pin.release.tag.as_str()) {
            out.push(format!(
                "{}: protocol release {:?} is not the locked {}",
                product.target, product.release_tag, pin.release.tag
            ));
        }
        if product.executable_sha256.as_deref() != Some(pin.artifact.executable_sha256.as_str()) {
            out.push(format!(
                "{}: protocol executable digest is not the locked one",
                product.target
            ));
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// The wait profile that claims each target's best available mechanism.
/// It needs a resolved best wait per competitor; the common-client
/// profile needs only the labelled poll (proposed decision 0093 §4).
pub const BEST_WAIT: &str = "wait-best";

/// Collection admission: a frozen, structurally valid protocol with nothing
/// unresolved, an enforced resource budget, and for every declared target
/// the candidate or a pin with every fact observed. Every refusal is
/// listed, so the one command can say exactly which phase is blocked.
pub fn collection_ready(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    if let Err(Refusal(r)) = p.validate() {
        out.extend(r);
    }
    if let Err(Refusal(r)) = lock.validate() {
        out.extend(r);
    }
    if let Err(Refusal(r)) = retention_matches(p, lock) {
        out.extend(r);
    }
    if let Err(Refusal(r)) = products_match(p, lock) {
        out.extend(r);
    }
    if p.status != Status::Frozen {
        out.push("protocol is a draft, not frozen".into());
    }
    if p.protocol_version != PROTOCOL_VERSION {
        out.push(format!(
            "protocol version {} is a historical draft; only version {PROTOCOL_VERSION} collects",
            p.protocol_version
        ));
    }
    for u in &p.unresolved {
        out.push(format!("unresolved: {u}"));
    }
    // An unenforced budget collects only as the separately named local
    // profile, and every report says so (proposed decision 0093 §3).
    if !p.resources.enforced && p.resources.profile != Some(ResourceProfile::UnenforcedLocal) {
        out.push("resource budget is not enforced by the selected runtime".into());
    }
    for t in &p.targets {
        let Some(e) = lock.entry(*t) else {
            out.push(format!("{t} has no source lock entry"));
            continue;
        };
        let Some(pin) = &e.pin else { continue };
        if let Some(why) = pin.tagged_source.gap() {
            out.push(format!("{t}: tagged source unresolved: {why}"));
        }
        // Proposed 0093 §2: the checked executable may be measured while
        // its build provenance is unresolved only under a frozen method
        // that declares the limited official-artifact claim; a protocol
        // that needs a built-from claim refuses it.
        if let Some(why) = pin.binary_build_provenance.gap()
            && p.claim.as_ref().map(|c| c.scope) != Some(ClaimScope::OfficialArtifact)
        {
            out.push(format!(
                "{t}: binary build provenance unresolved ({why}); only a protocol declaring the official-artifact claim scope may measure without it"
            ));
        }
        for d in &pin.documents {
            if let Some(why) = d.attribution.gap() {
                out.push(format!("{t}: {} attribution unresolved: {why}", d.path));
            }
        }
        if !pin.documents.iter().any(|d| d.kind == DocKind::Setup) {
            out.push(format!("{t}: no captured setup document"));
        }
        let best = p.waits.iter().any(|w| w.name == BEST_WAIT);
        if let (true, Some(why)) = (best, pin.wait.best_wait.gap()) {
            out.push(format!("{t}: best wait unresolved: {why}"));
        }
        let matched = p.storage.profile == Some(StorageProfile::MatchedMemory);
        if (matched || p.storage.mode == "memory") && pin.storage.backend != Backend::Memory {
            out.push(format!(
                "{t} does not store in memory; the matched memory profile cannot run it"
            ));
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}

/// The historical reader of lock version 1 (stopped candidate 4a116db).
/// Its bytes stay committed under `docs/evals/history/` and still
/// validate here; the driver never launches or collects from one, because
/// v1 cannot say whose digest a pin carries (decision 0092 §1). Only the
/// tests read it, so the driver binary sees it as dead code.
#[allow(dead_code)]
pub mod v1 {
    use super::{PERMISSIVE, Refusal, TargetKind, is_date, is_hex, is_name, launch_template_ok};
    use serde::{Deserialize, Serialize};

    pub const V1_LOCK_VERSION: u32 = 1;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SourceLock {
        pub lock_version: u32,
        pub entries: Vec<SourceEntry>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum PinStatus {
        /// The candidate checkout itself: identity is the commit and the
        /// built binary's digest, both recorded per run in the manifest.
        Candidate,
        Verified,
        Unverified,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SourceEntry {
        pub target: TargetKind,
        pub status: PinStatus,
        /// Why the entry is not verified; required while unverified.
        pub unverified_reason: Option<String>,
        pub source: Option<OfficialSource>,
        pub artifact: Option<Artifact>,
        pub supports_memory_storage: Option<bool>,
        /// The public, fixed launch argv from the official setup document.
        /// `{smtp_addr}`, `{http_addr}` and `{data_dir}` are the only
        /// substitutions; the driver owns those values.
        pub launch: Option<Vec<String>>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct OfficialSource {
        pub repository: String,
        /// Full commit id; a tag alone is mutable and refuses.
        pub revision: String,
        pub release: String,
        pub license_spdx: String,
        pub license_document: String,
        pub api_document: String,
        pub setup_document: String,
        pub captured_on: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Artifact {
        pub download: String,
        pub file_name: String,
        /// Digest of the downloaded release archive as published.
        pub sha256: String,
        /// Digest of the executable launched; checked at every launch.
        pub executable_sha256: String,
        pub os: String,
        pub arch: String,
    }

    impl SourceLock {
        pub fn parse(bytes: &[u8]) -> Result<SourceLock, Refusal> {
            serde_json::from_slice(bytes)
                .map_err(|e| Refusal(vec![format!("source lock does not parse: {e}")]))
        }

        pub fn entry(&self, target: TargetKind) -> Option<&SourceEntry> {
            self.entries.iter().find(|e| e.target == target)
        }

        pub fn validate(&self) -> Result<(), Refusal> {
            let mut out = Vec::new();
            if self.lock_version != V1_LOCK_VERSION {
                out.push(format!(
                    "lock_version {} is not {V1_LOCK_VERSION}",
                    self.lock_version
                ));
            }
            for t in super::HISTORICAL_TARGETS {
                let n = self.entries.iter().filter(|e| e.target == t).count();
                if n > 1 {
                    out.push(format!("{t} has {n} lock entries"));
                }
            }
            for e in &self.entries {
                let t = e.target;
                match (e.status, t) {
                    (PinStatus::Candidate, TargetKind::Swarmail) => {
                        if e.source.is_some()
                            || e.artifact.is_some()
                            || e.unverified_reason.is_some()
                            || e.launch.is_some()
                        {
                            out.push(
                                "swarmail is the candidate; its identity is recorded per run, not pinned"
                                    .into(),
                            );
                        }
                    }
                    (PinStatus::Candidate, _) => out.push(format!("{t} cannot be the candidate")),
                    (_, TargetKind::Swarmail) => {
                        out.push("swarmail's entry must be the candidate".into())
                    }
                    (PinStatus::Unverified, _) => {
                        if e.unverified_reason
                            .as_deref()
                            .is_none_or(|r| r.trim().is_empty())
                        {
                            out.push(format!("{t} is unverified without a reason"));
                        }
                    }
                    (PinStatus::Verified, _) => {
                        if e.unverified_reason.is_some() {
                            out.push(format!("{t} is verified yet carries an unverified_reason"));
                        }
                        verify_pins(&mut out, e);
                    }
                }
            }
            if out.is_empty() {
                Ok(())
            } else {
                Err(Refusal(out))
            }
        }
    }

    fn verify_pins(out: &mut Vec<String>, e: &SourceEntry) {
        let t = e.target;
        let (Some(src), Some(art)) = (&e.source, &e.artifact) else {
            out.push(format!(
                "{t}: verified without both an official source and a pinned artifact"
            ));
            return;
        };
        let official = t.official_repository().unwrap_or_default();
        if src.repository != official {
            out.push(format!(
                "{t}: source {} is not the official {official}",
                src.repository
            ));
        }
        if !is_hex(&src.revision, 40) {
            out.push(format!(
                "{t}: revision {:?} is not a full commit id (a tag alone is mutable)",
                src.revision
            ));
        }
        if src.release.trim().is_empty() {
            out.push(format!("{t}: no release"));
        }
        if !PERMISSIVE.contains(&src.license_spdx.as_str()) {
            out.push(format!(
                "{t}: license {:?} is not on the permissive list",
                src.license_spdx
            ));
        }
        for (field, url) in [
            ("license_document", &src.license_document),
            ("api_document", &src.api_document),
            ("setup_document", &src.setup_document),
            ("download", &art.download),
        ] {
            if !url.starts_with(&format!("{official}/")) {
                out.push(format!(
                    "{t}: {field} {url:?} is not under the official repository"
                ));
            }
        }
        if !is_date(&src.captured_on) {
            out.push(format!(
                "{t}: captured_on {:?} is not YYYY-MM-DD",
                src.captured_on
            ));
        }
        if !is_hex(&art.sha256, 64) || !is_hex(&art.executable_sha256, 64) {
            out.push(format!(
                "{t}: artifact digests must be 64 lowercase hex digits"
            ));
        }
        if !is_name(&art.file_name) {
            out.push(format!("{t}: artifact file name is malformed"));
        }
        if art.os.trim().is_empty() || art.arch.trim().is_empty() {
            out.push(format!("{t}: artifact platform is incomplete"));
        }
        if e.supports_memory_storage.is_none() {
            out.push(format!("{t}: storage capability is not recorded"));
        }
        if !e.launch.as_deref().is_some_and(launch_template_ok) {
            out.push(format!("{t}: launch template is missing or malformed"));
        }
    }
}

/// The historical reader of lock version 2 (stopped candidate 14217f7).
/// Its bytes stay committed under `docs/evals/history/` and still verify
/// against the documents beside the current lock. Its `source_revision`
/// keeps its recorded meaning — the commit the release was built from —
/// which is why version 3 replaced it rather than filling it from a tag.
/// The driver never launches or collects from a v2 lock; only the tests
/// read one, so the driver binary sees this as dead code.
#[allow(dead_code)]
pub mod v2 {
    use super::{
        Artifact, Fact, License, Refusal, Release, Role, StorageFacts, TargetKind, document_path,
        gap, is_hex, license_text, read_document, refusal, release_agrees, validate_common,
        validate_entries,
    };
    use serde::{Deserialize, Serialize};

    pub const V2_LOCK_VERSION: u32 = 2;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SourceLock {
        pub lock_version: u32,
        pub checked_on: String,
        pub entries: Vec<SourceEntry>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SourceEntry {
        pub target: TargetKind,
        pub role: Role,
        pub pin: Option<Pin>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Pin {
        pub release: Release,
        /// The full commit the release was built from.
        pub source_revision: Fact<String>,
        pub artifact: Artifact,
        pub license: License,
        pub documents: Vec<Document>,
        pub launch: Vec<String>,
        pub storage: StorageFacts,
        /// The reviewed source of a server-side wait or stream facility.
        pub wait: Fact<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum DocKind {
        ReleaseMetadata,
        License,
        Api,
        Setup,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Document {
        pub kind: DocKind,
        pub path: String,
        pub sha256: String,
        pub attribution: Fact<Attribution>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Attribution {
        pub url: String,
        pub revision: String,
    }

    impl SourceLock {
        pub fn parse(bytes: &[u8]) -> Result<SourceLock, Refusal> {
            serde_json::from_slice(bytes)
                .map_err(|e| Refusal(vec![format!("source lock does not parse: {e}")]))
        }

        pub fn validate(&self) -> Result<(), Refusal> {
            let mut out = Vec::new();
            if self.lock_version != V2_LOCK_VERSION {
                out.push(format!(
                    "lock_version {} is not {V2_LOCK_VERSION}",
                    self.lock_version
                ));
            }
            validate_entries(
                &mut out,
                &self.checked_on,
                &self.entries,
                &super::HISTORICAL_TARGETS,
                |e| (e.target, e.role, e.pin.is_some()),
            );
            for e in &self.entries {
                if let (Role::Competitor, Some(pin)) = (e.role, &e.pin)
                    && e.target != TargetKind::Swarmail
                {
                    validate_pin(&mut out, e.target, pin);
                }
            }
            refusal(out)
        }

        /// Version 2's byte checks: each document's digest, the release
        /// metadata against the pin, and the license text.
        pub fn verify_files(&self, dir: &std::path::Path) -> Result<(), Refusal> {
            let mut out = Vec::new();
            for e in &self.entries {
                let Some(pin) = &e.pin else { continue };
                let t = e.target;
                for d in &pin.documents {
                    let Some(bytes) = read_document(&mut out, t, dir, &d.path, &d.sha256) else {
                        continue;
                    };
                    match d.kind {
                        DocKind::ReleaseMetadata => {
                            match serde_json::from_slice::<serde_json::Value>(&bytes) {
                                Ok(v) => {
                                    release_agrees(&mut out, t, &pin.release, &pin.artifact, &v)
                                }
                                Err(_) => out.push(format!("{t}: release metadata does not parse")),
                            }
                        }
                        DocKind::License => {
                            license_text(&mut out, t, &pin.license, &d.path, &bytes)
                        }
                        _ => {}
                    }
                }
            }
            refusal(out)
        }
    }

    fn validate_pin(out: &mut Vec<String>, t: TargetKind, pin: &Pin) {
        validate_common(
            out,
            t,
            (&pin.release, &pin.artifact, &pin.license),
            (&pin.launch, super::launch_template_ok(&pin.launch)),
            &pin.storage,
        );
        let official = t.official_repository().unwrap_or_default();
        match &pin.source_revision {
            Fact::Observed(rev) if !is_hex(rev, 40) => out.push(format!(
                "{t}: revision {rev:?} is not a full commit id (a tag or branch is mutable)"
            )),
            Fact::Observed(_) => {}
            Fact::Unresolved(why) => gap(out, t, "source_revision", why),
        }
        let mut earlier = Vec::new();
        for d in &pin.documents {
            document_path(out, t, &earlier, &d.path, &d.sha256);
            earlier.push(d.path.as_str());
            match &d.attribution {
                Fact::Observed(at) => {
                    if !at.url.starts_with(&format!("{official}/")) {
                        out.push(format!(
                            "{t}: {} is not attributed to the official repository",
                            d.path
                        ));
                    }
                    if !is_hex(&at.revision, 40) {
                        out.push(format!(
                            "{t}: {} attribution names no full revision",
                            d.path
                        ));
                    }
                }
                Fact::Unresolved(why) => gap(out, t, &d.path, why),
            }
        }
        for kind in [DocKind::ReleaseMetadata, DocKind::License, DocKind::Api] {
            if !pin.documents.iter().any(|d| d.kind == kind) {
                out.push(format!("{t}: no captured {kind:?} document"));
            }
        }
        match &pin.wait {
            Fact::Observed(src) if src.trim().is_empty() => {
                out.push(format!("{t}: wait source is empty"))
            }
            Fact::Observed(_) => {}
            Fact::Unresolved(why) => gap(out, t, "wait", why),
        }
    }
}

/// The historical reader of lock version 3 (candidate c95386e's
/// `docs/evals/sources.lock.json`, archived byte-identical under
/// `docs/evals/history/`). It is the current reader given the three targets
/// version 3 knew: no reader code is copied, so the two cannot drift.
#[allow(dead_code)]
pub mod v3 {
    use super::{HISTORICAL_TARGETS, Refusal, SourceLock};

    pub const V3_LOCK_VERSION: u32 = 3;

    pub fn validate(lock: &SourceLock) -> Result<(), Refusal> {
        lock.validate_as(V3_LOCK_VERSION, &HISTORICAL_TARGETS)
    }
}
