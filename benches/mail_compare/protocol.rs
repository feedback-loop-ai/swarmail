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
/// The eight-target method (proposed decision 0130 §3). It starts a
/// separate run: protocol 2's collection stays as recorded, and no table or
/// statistic merges a version 2 cell with a version 3 one.
pub const PROTOCOL_VERSION_3: u32 = 3;
/// Every protocol id the method has used; a new protocol never reuses one.
pub const HISTORICAL_IDS: [&str; 3] = [
    "fair-mail-comparison-draft-2",
    "fair-mail-comparison-draft-3",
    "fair-mail-comparison-v2",
];
pub const PROTOCOL_V3_ID: &str = "fair-mail-comparison-v3";

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
    Mailslurper,
    /// MailSlurper release-1.15.0 built from its pinned source with cgo
    /// (operator ruling 2026-11-07). A different executable from the
    /// official release artifact: its own row, never a relabelled
    /// `mailslurper`, Alkemio's image or Swarmail's shim (decision 0118).
    #[serde(rename = "mailslurper-source-build")]
    MailslurperSourceBuild,
    Smtp4dev,
    Inbucket,
    Greenmail,
    /// Alkemio's patched MailSlurper image, extracted and run natively. It
    /// is its own target and never the upstream `mailslurper` row, nor
    /// Swarmail's own MailSlurper shim (decision 0118).
    #[serde(rename = "alkemio-mailslurper")]
    AlkemioMailslurper,
}

/// Where a target's bytes come from (proposed decision 0130 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// The candidate checkout itself.
    Candidate,
    /// An official upstream repository's release.
    Official(&'static str),
    /// A third party's OCI image reference: never an official artifact.
    ThirdPartyImage(&'static str),
    /// An executable built from an official repository's tagged commit by
    /// the operator's supply, never a release asset (ruling 2026-11-07).
    SourceBuild(&'static str),
}

impl Provenance {
    /// Whether the target has a tagged commit of an official repository to
    /// observe. Only a third-party image has none: its identity is its
    /// index -> manifest -> layer digest chain, and `validate_image` refuses
    /// an observed `tagged_source` on it (proposed decision 0130 §2). The
    /// answer is keyed on this closed provenance, never on whether the lock
    /// data happens to carry an `image` field.
    pub fn has_tagged_source(self) -> bool {
        !matches!(self, Provenance::ThirdPartyImage(_))
    }
}

/// The Alkemio image the third-party row is extracted from.
pub const ALKEMIO_IMAGE: &str = "ghcr.io/alkem-io/test-suites/mailslurper:smtps-4c6aecc-p2";

impl TargetKind {
    /// The three targets protocol 2 and lock 3 know, as they were recorded.
    pub const V2: [TargetKind; 3] = [
        TargetKind::Swarmail,
        TargetKind::Mailpit,
        TargetKind::Mailhog,
    ];
    /// The nine targets protocol 3 and lock 4 require.
    pub const ALL: [TargetKind; 9] = [
        TargetKind::Swarmail,
        TargetKind::Mailpit,
        TargetKind::Mailhog,
        TargetKind::Mailslurper,
        TargetKind::MailslurperSourceBuild,
        TargetKind::Smtp4dev,
        TargetKind::Inbucket,
        TargetKind::Greenmail,
        TargetKind::AlkemioMailslurper,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TargetKind::Swarmail => "swarmail",
            TargetKind::Mailpit => "mailpit",
            TargetKind::Mailhog => "mailhog",
            TargetKind::Mailslurper => "mailslurper",
            TargetKind::MailslurperSourceBuild => "mailslurper-source-build",
            TargetKind::Smtp4dev => "smtp4dev",
            TargetKind::Inbucket => "inbucket",
            TargetKind::Greenmail => "greenmail",
            TargetKind::AlkemioMailslurper => "alkemio-mailslurper",
        }
    }

    pub fn parse(name: &str) -> Option<TargetKind> {
        TargetKind::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Where a competitor's bytes must come from. An official release must
    /// come from the upstream home; a third-party image is never counted as
    /// one (spec R1). Swarmail is the candidate itself and has no upstream.
    /// The captured release metadata under `docs/evals/sources/` names the
    /// homes as the releases' own; [`SourceLock::verify_files`] rechecks it.
    pub fn provenance(self) -> Provenance {
        match self {
            TargetKind::Swarmail => Provenance::Candidate,
            TargetKind::Mailpit => Provenance::Official("https://github.com/axllent/mailpit"),
            TargetKind::Mailhog => Provenance::Official("https://github.com/mailhog/MailHog"),
            TargetKind::Mailslurper => {
                Provenance::Official("https://github.com/mailslurper/mailslurper")
            }
            TargetKind::MailslurperSourceBuild => {
                Provenance::SourceBuild("https://github.com/mailslurper/mailslurper")
            }
            TargetKind::Smtp4dev => Provenance::Official("https://github.com/rnwood/smtp4dev"),
            TargetKind::Inbucket => Provenance::Official("https://github.com/inbucket/inbucket"),
            TargetKind::Greenmail => {
                Provenance::Official("https://github.com/greenmail-mail-test/greenmail")
            }
            TargetKind::AlkemioMailslurper => Provenance::ThirdPartyImage(ALKEMIO_IMAGE),
        }
    }

    /// The upstream repository of an official or source-built target, else
    /// `""`.
    pub fn official_home(self) -> &'static str {
        match self.provenance() {
            Provenance::Official(home) | Provenance::SourceBuild(home) => home,
            _ => "",
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
    /// Version 3: what each target's pin establishes about search, waits,
    /// retention and raw reads, each with its citation or `not_inspected`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<Capabilities>>,
    /// Version 3 (operator ruling 2026-11-07): targets whose published
    /// result is that the pinned executable does not start. Each keeps its
    /// row, pin, product, capabilities and rotation slot; the runner never
    /// launches one and its report row carries no number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub does_not_start: Option<Vec<DoesNotStart>>,
    /// Every value the method still lacks, by name. Collection refuses
    /// while one remains; the list is how a draft says what it is missing
    /// instead of carrying a placeholder that looks like a value.
    pub unresolved: Vec<String>,
}

/// A target that does not start, with the document of its pin that records
/// the failure (proposed decision 0130, 2026-11-07 addendum).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoesNotStart {
    pub target: TargetKind,
    /// A document path of that target's pin: the captured failure output.
    pub cite: String,
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
    /// Version 3: what kind of product this is. It must agree with the
    /// target's provenance and, once pinned, the lock's pin source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ProductScope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductScope {
    Candidate,
    OfficialArtifact,
    ThirdPartyImage,
    /// Built from an official repository's tagged commit; never a release
    /// asset.
    SourceBuild,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Supported,
    NotSupported,
    NotInspected,
}

/// One fact about a target. `supported` and `not_supported` cite a document
/// or reviewed source of the target's pin; a fact nobody reviewed is
/// `not_inspected` and carries no citation. Nothing is assumed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub status: CapabilityStatus,
    pub cite: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub target: TargetKind,
    pub server_search: Capability,
    pub native_wait: Capability,
    pub retention_cap: Capability,
    pub raw_source: Capability,
}

impl Capabilities {
    pub fn each(&self) -> [(&'static str, &Capability); 4] {
        [
            ("server_search", &self.server_search),
            ("native_wait", &self.native_wait),
            ("retention_cap", &self.retention_cap),
            ("raw_source", &self.raw_source),
        ]
    }
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

/// Lock version 4's closed tag shapes: `v1.2.3`, `release-1.2.3` or a bare
/// `1.2.3`. A tag is evidence, not a pattern: `latest`, branch names,
/// prereleases and a bare major are not versions.
pub fn is_version_tag(s: &str) -> bool {
    let n = s
        .strip_prefix("release-")
        .or_else(|| s.strip_prefix('v'))
        .unwrap_or(s);
    n.split('.').count() >= 2
        && n.split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
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
            DRAFT_PROTOCOL_VERSION | PROTOCOL_VERSION | PROTOCOL_VERSION_3
        ) {
            out.push(format!(
                "protocol_version {} is not {DRAFT_PROTOCOL_VERSION}, {PROTOCOL_VERSION} or {PROTOCOL_VERSION_3}",
                self.protocol_version
            ));
        }
        // Target sets are scoped by version: protocol 2 and the drafts know
        // the original three, protocol 3 requires all eight.
        let v3 = self.protocol_version == PROTOCOL_VERSION_3;
        for t in &self.targets {
            if !v3 && !TargetKind::V2.contains(t) {
                out.push(format!(
                    "protocol_version {} refuses target {t}; only protocol_version {PROTOCOL_VERSION_3} declares it",
                    self.protocol_version
                ));
            }
        }
        if v3 {
            for t in TargetKind::ALL {
                if !self.targets.contains(&t) {
                    out.push(format!(
                        "protocol_version {PROTOCOL_VERSION_3} needs target {t}"
                    ));
                }
            }
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
        if v3 && self.status == Status::Frozen && !self.unresolved.is_empty() {
            out.push(format!(
                "a frozen protocol has nothing unresolved, but {} entries remain",
                self.unresolved.len()
            ));
        }
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
            self.capabilities.is_some(),
            self.products.iter().flatten().any(|p| p.scope.is_some()),
            self.does_not_start.is_some(),
        ];
        if self.protocol_version == DRAFT_PROTOCOL_VERSION {
            if has.iter().any(|h| *h) {
                out.push(format!(
                    "protocol_version {DRAFT_PROTOCOL_VERSION} carries version {PROTOCOL_VERSION} sections"
                ));
            }
            return;
        }
        if self.protocol_version == PROTOCOL_VERSION && (has[6] || has[7] || has[8]) {
            out.push(format!(
                "protocol_version {PROTOCOL_VERSION} carries version {PROTOCOL_VERSION_3} sections (capabilities, product scope, does_not_start)"
            ));
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
        if self.protocol_version == PROTOCOL_VERSION_3 {
            self.validate_v3(out);
        }
    }

    /// Protocol 3's own rules (proposed decision 0130 §3): a new id that
    /// supersedes protocol 2 and cites the record, the eight cyclic
    /// rotations, one capability record per target and a product scope that
    /// agrees with each target's provenance.
    fn validate_v3(&self, out: &mut Vec<String>) {
        if HISTORICAL_IDS.contains(&self.id.as_str()) {
            out.push(format!("id {:?} reuses a historical protocol id", self.id));
        }
        if self.id != PROTOCOL_V3_ID {
            out.push(format!(
                "protocol 3's id is {PROTOCOL_V3_ID}, not {:?}",
                self.id
            ));
        }
        if self.supersedes.as_deref() != Some(HISTORICAL_IDS[2]) {
            out.push(format!("protocol 3 supersedes {}", HISTORICAL_IDS[2]));
        }
        if !self
            .change_reason
            .as_deref()
            .is_some_and(|r| r.contains("0130"))
        {
            out.push("protocol 3's change_reason must cite decision 0130".into());
        }
        let n = self.targets.len();
        if self.statistics.orders.len() != n {
            out.push(format!(
                "protocol 3 has {} target orders, not the {n} cyclic rotations",
                self.statistics.orders.len()
            ));
        }
        for k in 0..n {
            let rotation: Vec<TargetKind> = (0..n).map(|i| self.targets[(i + k) % n]).collect();
            if !self.statistics.orders.contains(&rotation) {
                out.push(format!("target order rotation {k} is missing"));
            }
        }
        let caps = self.capabilities.as_deref().unwrap_or_default();
        for t in &self.targets {
            let mine: Vec<&Capabilities> = caps.iter().filter(|c| c.target == *t).collect();
            let [c] = mine.as_slice() else {
                out.push(format!("capabilities: {t} needs exactly one record"));
                continue;
            };
            for (key, cap) in c.each() {
                let cited = cap.cite.as_deref().is_some_and(|s| !s.trim().is_empty());
                match (cap.status, cited) {
                    (CapabilityStatus::NotInspected, false) => {}
                    (CapabilityStatus::NotInspected, true) => out.push(format!(
                        "capabilities: {t} {key} is not_inspected, so it cites nothing"
                    )),
                    (_, true) => {}
                    (_, false) => out.push(format!(
                        "capabilities: {t} {key} is {:?} without a citation",
                        cap.status
                    )),
                }
            }
        }
        for c in caps {
            if !self.targets.contains(&c.target) {
                out.push(format!(
                    "capabilities: record for undeclared target {}",
                    c.target
                ));
            }
        }
        self.validate_does_not_start(out);
        for p in self.products.iter().flatten() {
            let want = match p.target.provenance() {
                Provenance::Candidate => ProductScope::Candidate,
                Provenance::Official(_) => ProductScope::OfficialArtifact,
                Provenance::ThirdPartyImage(_) => ProductScope::ThirdPartyImage,
                Provenance::SourceBuild(_) => ProductScope::SourceBuild,
            };
            match p.scope {
                None => out.push(format!("products: {} states no scope", p.target)),
                Some(s) if s != want => out.push(format!(
                    "products: {} scope {s:?} is not its provenance's {want:?}",
                    p.target
                )),
                Some(_) => {}
            }
        }
    }

    /// `does_not_start` entries: declared competitors, listed once, each
    /// citing something. That the cite is a document of the target's pin is
    /// [`does_not_start_matches`]'s, which needs the lock.
    fn validate_does_not_start(&self, out: &mut Vec<String>) {
        for (i, d) in self.does_not_start.iter().flatten().enumerate() {
            let t = d.target;
            if !self.targets.contains(&t) || t == TargetKind::Swarmail {
                out.push(format!("does_not_start: {t} is not a declared competitor"));
            }
            if self
                .does_not_start
                .iter()
                .flatten()
                .take(i)
                .any(|o| o.target == t)
            {
                out.push(format!("does_not_start: {t} is listed twice"));
            }
            if d.cite.trim().is_empty() {
                out.push(format!("does_not_start: {t} cites no document"));
            }
        }
    }

    /// Whether this protocol publishes `t` as a target that does not start.
    pub fn does_not_start(&self, t: TargetKind) -> bool {
        self.does_not_start.iter().flatten().any(|d| d.target == t)
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
            let v3 = self.protocol_version == PROTOCOL_VERSION_3;
            let tag_ok = |tag: &str| {
                if v3 {
                    is_version_tag(tag)
                } else {
                    is_release_tag(tag)
                }
            };
            match (t.provenance(), &p.release_tag, &p.executable_sha256) {
                (Provenance::Candidate, None, None) => {}
                (Provenance::Candidate, ..) => out.push(format!(
                    "products: {t} is the candidate; its identity is recorded per run, not pinned here"
                )),
                // Protocol 3 may name a product whose bytes are not
                // available yet (a pending lock entry): its digest is then
                // absent, and collection refuses the entry by name.
                (Provenance::Official(_) | Provenance::SourceBuild(_), Some(tag), sha)
                    if sha.is_some() || v3 =>
                {
                    if !tag_ok(tag) {
                        out.push(format!(
                            "products: {t} release tag {tag:?} is not an immutable version tag"
                        ));
                    }
                    if sha.as_deref().is_some_and(|s| !is_hex(s, 64)) {
                        out.push(format!("products: {t} executable digest is not 64 hex digits"));
                    }
                }
                (Provenance::Official(_) | Provenance::SourceBuild(_), ..) => out.push(format!(
                    "products: {t} needs a release tag and an executable digest"
                )),
                (Provenance::ThirdPartyImage(_), None, sha) if v3 => {
                    if sha.as_deref().is_some_and(|s| !is_hex(s, 64)) {
                        out.push(format!("products: {t} executable digest is not 64 hex digits"));
                    }
                }
                (Provenance::ThirdPartyImage(_), ..) => out.push(format!(
                    "products: {t} is a third-party image: it has no release tag, and only protocol {PROTOCOL_VERSION_3} names one"
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

/// A launch template is short printable argv with every brace a known
/// slot, and both listener slots present (the driver must own the ports).
/// This is lock version 3's rule; lock version 4 states its listeners and
/// is held by [`launch_checks`].
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

/// The role a listener plays (proposed decision 0130 §2). A closed set:
/// each role at most once per launch, every one a reserved loopback port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListenerRole {
    Smtp,
    Http,
    Pop3,
    Imap,
    Ui,
}

impl ListenerRole {
    pub fn name(self) -> &'static str {
        match self {
            ListenerRole::Smtp => "smtp",
            ListenerRole::Http => "http",
            ListenerRole::Pop3 => "pop3",
            ListenerRole::Imap => "imap",
            ListenerRole::Ui => "ui",
        }
    }

    pub fn addr_slot(self) -> String {
        format!("{{{}_addr}}", self.name())
    }

    pub fn port_slot(self) -> String {
        format!("{{{}_port}}", self.name())
    }
}

/// Names the launcher owns in the child's closed environment.
pub fn env_name_owned(n: &str) -> bool {
    matches!(n, "PATH" | "HOME" | "TMPDIR" | "LANG") || n.starts_with("XDG_")
}

fn env_name_ok(n: &str) -> bool {
    n.len() <= 64
        && n.starts_with(|c: char| c.is_ascii_uppercase())
        && n.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// A rendered config file: a leaf name in the owned scratch and its lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    pub name: String,
    pub lines: Vec<String>,
    /// The child's working directory is the directory the file is rendered
    /// in: for a program that reads a fixed relative name and has no flag for
    /// it (the Alkemio fork reads `./config.json`). Default false: the
    /// working directory is the owned scratch.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cwd: bool,
}

/// The slots a version 4 text may name: one address and one port per
/// declared role, plus the owned data directory and scratch, the launched
/// artifact and the rendered config file.
pub fn slots_of(roles: &[ListenerRole]) -> Vec<String> {
    let mut v: Vec<String> = roles
        .iter()
        .flat_map(|r| [r.addr_slot(), r.port_slot()])
        .collect();
    for s in ["{data_dir}", "{scratch}", "{artifact}", "{config_path}"] {
        v.push(s.into());
    }
    v
}

/// A literal brace is written doubled (`{{` and `}}`), so a JSON config
/// line is expressible. The escapes are removed before slots are looked for.
pub const BRACE_ESCAPES: [(&str, &str); 2] = [("{{", "\u{e000}"), ("}}", "\u{e001}")];

/// The texts (argv, env values, config lines) with every escape and known
/// slot removed; any brace left is an undeclared slot.
fn without_slots(text: &str, slots: &[String]) -> String {
    let mut rest = text.to_owned();
    for (escape, _) in BRACE_ESCAPES {
        rest = rest.replace(escape, "");
    }
    for s in slots {
        rest = rest.replace(s.as_str(), "");
    }
    rest
}

/// Version 4's launch rules, shared by a pin and a pending entry: the
/// listener set, the template, the environment and the config file.
pub fn launch_checks(
    out: &mut Vec<String>,
    t: TargetKind,
    roles: &[ListenerRole],
    launch: &[String],
    env: &[[String; 2]],
    config: Option<&ConfigFile>,
) {
    for (i, r) in roles.iter().enumerate() {
        if roles[..i].contains(r) {
            out.push(format!("{t}: listener role {} is listed twice", r.name()));
        }
    }
    if !roles.contains(&ListenerRole::Smtp) {
        out.push(format!("{t}: the listeners lack the smtp role"));
    }
    let read = if t == TargetKind::Greenmail {
        ListenerRole::Pop3
    } else {
        ListenerRole::Http
    };
    if !roles.contains(&read) {
        out.push(format!(
            "{t}: the listeners lack the {} role its read path uses",
            read.name()
        ));
    }
    let slots = slots_of(roles);
    let mut texts: Vec<(&str, &str)> = Vec::new();
    if launch.len() > 32 {
        out.push(format!("{t}: the launch has more than 32 arguments"));
    }
    for a in launch {
        if a.len() > 256
            || !without_slots(a, &slots)
                .chars()
                .all(|c| c.is_ascii_graphic())
        {
            out.push(format!(
                "{t}: launch argument {a:?} is not short printable text"
            ));
        }
        texts.push(("launch", a));
    }
    if env.len() > 16 {
        out.push(format!(
            "{t}: the launch environment has more than 16 entries"
        ));
    }
    for (i, [name, value]) in env.iter().enumerate() {
        if !env_name_ok(name) {
            out.push(format!(
                "{t}: environment name {name:?} is not upper snake case"
            ));
        }
        if env_name_owned(name) {
            out.push(format!(
                "{t}: environment name {name} is owned by the launcher"
            ));
        }
        if env[..i].iter().any(|[n, _]| n == name) {
            out.push(format!("{t}: environment name {name} is listed twice"));
        }
        if value.len() > 256
            || !without_slots(value, &slots)
                .chars()
                .all(|c| c.is_ascii_graphic())
        {
            out.push(format!(
                "{t}: environment value of {name} is not short printable text"
            ));
        }
        texts.push(("environment", value));
    }
    match config {
        Some(c) => {
            if !is_name(&c.name) || c.name.contains("..") || c.name.contains('/') {
                out.push(format!(
                    "{t}: config file name {:?} must be one plain leaf name",
                    c.name
                ));
            }
            if c.lines.is_empty() || c.lines.len() > 64 {
                out.push(format!("{t}: a config file has 1 to 64 lines"));
            }
            for l in &c.lines {
                if l.len() > 1024 || !l.chars().all(|ch| ch == ' ' || ch.is_ascii_graphic()) {
                    out.push(format!("{t}: a config line is not short printable text"));
                }
                texts.push(("config", l));
            }
        }
        None => {
            if launch.iter().any(|a| a.contains("{config_path}")) {
                out.push(format!(
                    "{t}: the launch names {{config_path}} but states no config file"
                ));
            }
        }
    }
    for (kind, text) in &texts {
        if without_slots(text, &slots).contains(['{', '}']) {
            out.push(format!(
                "{t}: {kind} text {text:?} names a slot the declared listeners and the owned directory do not provide"
            ));
        }
    }
    for r in roles {
        let (a, p) = (r.addr_slot(), r.port_slot());
        if !texts.iter().any(|(_, x)| x.contains(&a) || x.contains(&p)) {
            out.push(format!(
                "{t}: the declared {} listener is never given to the launch",
                r.name()
            ));
        }
    }
}

/// The permissive licenses a pinned experiment input may carry: the
/// subset of `deny.toml`'s allowlist a server release could use. The
/// binary is not linked, but its terms still have to be checkable.
pub const PERMISSIVE: [&str; 5] = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"];

/// The licenses a pinned *runtime* may carry (proposed decision 0130 §6,
/// accepted by the operator on 2026-11-07): a separate closed list that
/// holds the JRE's expression and musl's. A runtime is neither linked nor
/// measured, so [`PERMISSIVE`] is never widened for it and `deny.toml` does
/// not change.
const RUNTIME_LICENSES: [&str; 2] = ["GPL-2.0-only WITH Classpath-exception-2.0", "MIT"];

/// One runtime a target may pin (operator ruling 2026-11-07): the program the
/// launcher execs, the members that must be digested, and whether the
/// runtime is a layer of the target's own image (checked against the
/// captured manifest). Any other target's runtime is refused.
pub struct RuntimeRule {
    pub target: TargetKind,
    pub entry: &'static str,
    pub members: &'static [&'static str],
    pub image_layer: bool,
}

pub const RUNTIMES: [RuntimeRule; 2] = [
    RuntimeRule {
        target: TargetKind::Greenmail,
        entry: "bin/java",
        members: &["bin/java", "lib/server/libjvm.so"],
        image_layer: false,
    },
    RuntimeRule {
        target: TargetKind::AlkemioMailslurper,
        entry: "lib/ld-musl-x86_64.so.1",
        members: &["lib/ld-musl-x86_64.so.1"],
        image_layer: true,
    },
];

/// The runtime rule of `t`, if it may pin a runtime.
pub fn runtime_rule(t: TargetKind) -> Option<&'static RuntimeRule> {
    RUNTIMES.iter().find(|r| r.target == t)
}

/// A version an executable reports that differs from its release tag,
/// recorded as a closed fact (operator ruling 2026-11-07): `(target, tag,
/// reported version)`. The release-1.15.0 source hard-codes
/// `SERVER_VERSION = "1.14.1"`, so both MailSlurper executables log 1.14.1.
/// Data cannot excuse a version check: only this list can.
pub const REPORTED_VERSIONS: [(TargetKind, &str, &str); 2] = [
    (TargetKind::Mailslurper, "release-1.15.0", "1.14.1"),
    (
        TargetKind::MailslurperSourceBuild,
        "release-1.15.0",
        "1.14.1",
    ),
];

/// The registries a pin's digest may be bound to instead of an asset of the
/// official release (operator ruling 2026-11-07): `(target, base URL)`. The
/// artifact's download is `{base}{version}/{file_name}`.
pub const REGISTRY: [(TargetKind, &str); 1] = [(
    TargetKind::Greenmail,
    "https://repo1.maven.org/maven2/com/icegreen/greenmail-standalone/",
)];

/// A source archive whose members may attribute a document without a git
/// tree listing: `(targets, archive sha256, the commit the archive is of)`.
/// The commit is the archive's own pax comment (`git get-tar-commit-id`).
pub const SOURCE_ARCHIVES: [(&[TargetKind], &str, &str); 1] = [(
    &[TargetKind::Mailslurper, TargetKind::MailslurperSourceBuild],
    "83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f",
    "068acffdcd8960f248ea438c3b7ee51ead601bd2",
)];

/// Source lock version 3 (proposed decision 0093 §1). It keeps version 2's
/// separate artifact, digest-basis and document facts, and replaces v2's
/// `source_revision` — a built-from claim — with two facts that never fill
/// each other: `tagged_source`, the commit the official tag names through
/// its captured ref → tag → commit chain, and `binary_build_provenance`,
/// what the executable was built from, which only captured build evidence
/// resolves. Tag identity is source identity, never executable identity.
pub const LOCK_VERSION: u32 = 3;
/// Lock version 4 (proposed decision 0130 §2) is version 3 plus fields only
/// version 4 may carry: lightweight tag pins, third-party image pins, a
/// pinned runtime, declared listeners, a launch environment, a config file
/// and pending entries. Version 3 refuses each of them.
pub const LOCK_VERSION_4: u32 = 4;

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
    /// A competitor has a pin or (version 4) a pending entry; the
    /// candidate has neither.
    pub pin: Option<Pin>,
    /// Version 4: a declared competitor whose bytes are not available. It
    /// never launches, and collection refuses it by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
}

/// What can be stated without the bytes (proposed decision 0130 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    /// An official target's release tag, as the issue or release page
    /// names it; absent for the third-party image.
    pub release_tag: Option<String>,
    /// The third-party image reference; absent for an official target.
    pub image: Option<String>,
    pub listeners: Vec<ListenerRole>,
    /// The drafted launch. It is not verified against any executable.
    pub launch: Vec<String>,
    #[serde(default)]
    pub launch_env: Vec<[String; 2]>,
    pub config_file: Option<ConfigFile>,
    /// Why the entry is pending and what it still lacks.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    /// An official release's metadata. Lock version 3 requires it; a
    /// version 4 image pin has `image` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Release>,
    /// Version 4: a third-party OCI image the executable is extracted from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageSource>,
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
    pub storage: StorageFacts,
    pub wait: WaitFacts,
    /// Version 4: the interpreter the target needs but that is not a target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<Runtime>,
    /// Version 4: the listeners the launch declares. After readiness the
    /// owned group must listen on exactly this set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listeners: Option<Vec<ListenerRole>>,
    /// Version 4: `[NAME, value]` environment entries added to the closed
    /// environment. Slots in values are substituted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_env: Option<Vec<[String; 2]>>,
    /// Version 4: a config file rendered into the owned scratch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_file: Option<ConfigFile>,
}

/// A third-party image pin: the index, the platform manifest and the layer
/// are bound by content digest, and the executable is a member of the layer
/// extracted once into its own empty directory (decision 0130 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSource {
    pub reference: String,
    /// `sha256:<64 hex>` of the captured index bytes.
    pub index_digest: String,
    /// The platform manifest the index names.
    pub manifest_digest: String,
    /// `os/arch`, as the index states it.
    pub platform: String,
    /// The layer the manifest names that holds the executable.
    pub layer_digest: String,
    /// The executable's path inside the layer.
    pub member: String,
}

/// A checked member of an unpacked tree: its path relative to the tree's
/// root and its sha256, rechecked at launch and again at seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckedMember {
    pub path: String,
    pub sha256: String,
}

/// An interpreter pin (GreenMail's JRE). Only the code-bearing members it
/// lists are digested; the rest of the tree is not rechecked, and the
/// README labels it so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub name: String,
    pub download: String,
    pub sha256: String,
    pub digest_basis: DigestBasis,
    pub checked_members: Vec<CheckedMember>,
    pub version_output: String,
    pub license: License,
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
    /// Version 4: how the ref names the commit. Required at version 4,
    /// refused at version 3 (whose tags are all annotated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_kind: Option<RefKind>,
    /// The annotated tag object; absent for a lightweight tag, which has
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_object: Option<String>,
    pub commit: String,
    /// The tag's signature status as the captured tag object reports it.
    /// A lightweight tag's signature is absent, not "unsigned": there is no
    /// object that could be signed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_signature: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Annotated,
    Lightweight,
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
}

/// Where the asset digest comes from. Only `upstream_release_metadata`
/// is a digest the publisher stated; `locally_computed` is integrity of
/// the bytes the host supplied, not an upstream checksum or signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigestBasis {
    UpstreamReleaseMetadata,
    LocallyComputed,
    /// The digest is the one a package registry publishes beside the
    /// artifact (a `.sha256` document), for an official artifact the GitHub
    /// release does not list. Integrity against the registry's own checksum
    /// only: nothing here authenticates the publisher.
    RegistryChecksum,
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
    /// Version 4: further code-bearing members of an unpacked tree that are
    /// digested at launch and at seal (at most 8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_members: Option<Vec<CheckedMember>>,
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
    BuildEvidence,
    /// Version 4: the captured OCI image index, whose sha256 is its digest.
    ImageIndex,
    /// Version 4: the captured platform manifest.
    ImageManifest,
    /// Version 4: an API commit object, linking the tagged commit to its
    /// root tree (the supplied trees are fetched by tree sha).
    GitCommit,
    /// Version 4: a registry's checksum document for the artifact.
    Checksum,
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
    /// A document of the third-party image `reference`.
    Image { reference: String },
    /// A member of a digested archive: the pin's artifact, its runtime
    /// archive or a closed source archive. Proved by a real test that
    /// re-extracts it; the bench has no archive reader.
    Member {
        archive_sha256: String,
        member: String,
    },
    /// The pinned executable's own output for `argv`, proved by a real test
    /// that re-runs it.
    Output { argv: Vec<String> },
    /// Operator-supplied evidence: `record` names the supply path and
    /// carries the document's sha256.
    Supplied { record: String },
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
        let mut out = Vec::new();
        let v4 = self.lock_version == LOCK_VERSION_4;
        if !matches!(self.lock_version, LOCK_VERSION | LOCK_VERSION_4) {
            out.push(format!(
                "lock_version {} is not {LOCK_VERSION} or {LOCK_VERSION_4}",
                self.lock_version
            ));
        }
        validate_entries(&mut out, &self.checked_on, &self.entries, |e| {
            (e.target, e.role, e.pin.is_some() || e.pending.is_some())
        });
        for e in &self.entries {
            let t = e.target;
            if !v4 && !TargetKind::V2.contains(&t) {
                out.push(format!(
                    "lock_version {LOCK_VERSION} refuses target {t}; only lock_version {LOCK_VERSION_4} locks it"
                ));
            }
            if !v4 && e.pending.is_some() {
                out.push(format!("{t}: lock_version {LOCK_VERSION} refuses pending"));
            }
            if e.pin.is_some() && e.pending.is_some() {
                out.push(format!("{t}: an entry is a pin or pending, never both"));
            }
        }
        if v4 {
            for t in TargetKind::ALL {
                if self.entry(t).is_none() {
                    out.push(format!(
                        "lock_version {LOCK_VERSION_4} needs an entry for {t}"
                    ));
                }
            }
        }
        for e in &self.entries {
            if e.role != Role::Competitor || e.target == TargetKind::Swarmail {
                continue;
            }
            if let Some(pin) = &e.pin {
                validate_pin(&mut out, e.target, pin, v4);
            }
            if let Some(pending) = &e.pending {
                validate_pending(&mut out, e.target, pending);
            }
        }
        refusal(out)
    }

    /// Recheck every captured document against its size and digest; the
    /// release metadata against the pin; each API capture against the URL
    /// it names itself by; the ref → (tag →) commit chain; an image pin's
    /// index → manifest → layer chain; and every commit-attributed document
    /// and reviewed source against its git blob id in the captured tree.
    /// `dir` is the lock's own directory. Local consistency only, not
    /// authentication. A pending entry has no bytes and is skipped; the
    /// collection gate refuses it.
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
                if let Fact::Observed(Attribution::Commit { git_blob, .. }) = &d.attribution
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
                if d.kind == DocKind::Checksum
                    && pin.artifact.upstream_digest.as_deref()
                        != Some(String::from_utf8_lossy(&bytes).trim())
                {
                    out.push(format!(
                        "{t}: the checksum document {} disagrees with the pin's upstream digest",
                        d.path
                    ));
                }
                if let Some(img) = &pin.image {
                    let layer = pin.runtime.as_ref().map(|r| format!("sha256:{}", r.sha256));
                    image_document(&mut out, t, (img, layer), d.kind, &d.path, &bytes);
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
                if d.kind == DocKind::ReleaseMetadata
                    && let Some(release) = &pin.release
                {
                    release_agrees(&mut out, t, release, &pin.artifact, &v);
                }
                json.insert(d.kind, v);
            }
            if let (Fact::Observed(ts), Some(release)) = (&pin.tagged_source, &pin.release) {
                tag_chain(&mut out, t, pin, &release.tag, ts, &json);
            }
        }
        refusal(out)
    }
}

/// An image pin's content-addressed chain. The index's sha256 is its
/// digest; it names the declared platform's manifest; the manifest's sha256
/// is that digest and it names the layer the executable is extracted from.
fn image_document(
    out: &mut Vec<String>,
    t: TargetKind,
    (img, runtime_layer): (&ImageSource, Option<String>),
    kind: DocKind,
    path: &str,
    bytes: &[u8],
) {
    let hex = |d: &str| d.strip_prefix("sha256:").unwrap_or_default().to_owned();
    let doc = || serde_json::from_slice::<serde_json::Value>(bytes).ok();
    match kind {
        DocKind::ImageIndex => {
            if sha256_hex(bytes) != hex(&img.index_digest) {
                out.push(format!("{t}: {path} is not the index {}", img.index_digest));
            }
            let named = doc().is_some_and(|v| {
                v.get("manifests")
                    .and_then(|m| m.as_array())
                    .into_iter()
                    .flatten()
                    .any(|m| {
                        let platform = format!(
                            "{}/{}",
                            m.pointer("/platform/os")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default(),
                            m.pointer("/platform/architecture")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                        );
                        m.get("digest").and_then(|d| d.as_str())
                            == Some(img.manifest_digest.as_str())
                            && platform == img.platform
                    })
            });
            if !named {
                out.push(format!(
                    "{t}: the captured index does not name manifest {} for {}",
                    img.manifest_digest, img.platform
                ));
            }
        }
        DocKind::ImageManifest => {
            if sha256_hex(bytes) != hex(&img.manifest_digest) {
                out.push(format!(
                    "{t}: {path} is not the manifest {}",
                    img.manifest_digest
                ));
            }
            let names_layer = doc().is_some_and(|v| {
                v.get("layers")
                    .and_then(|l| l.as_array())
                    .into_iter()
                    .flatten()
                    .any(|l| {
                        l.get("digest").and_then(|d| d.as_str()) == Some(img.layer_digest.as_str())
                    })
            });
            if !names_layer {
                out.push(format!(
                    "{t}: the captured manifest does not name layer {}",
                    img.layer_digest
                ));
            }
            if let Some(layer) = runtime_layer {
                let listed = doc().is_some_and(|v| {
                    v.get("layers")
                        .and_then(|l| l.as_array())
                        .into_iter()
                        .flatten()
                        .any(|l| l.get("digest").and_then(|d| d.as_str()) == Some(layer.as_str()))
                });
                if !listed {
                    out.push(format!(
                        "{t}: the captured manifest does not list the runtime's layer {layer}"
                    ));
                }
            }
        }
        _ => {}
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

/// The ref names the commit: through the annotated tag object, which names
/// the tag, the commit and its stated signature status, or (version 4,
/// lightweight) directly, with no tag object at all. The untruncated tree
/// listing of that commit holds every commit-attributed document and
/// reviewed source at its path with its git blob id and size. The tree
/// capture is asked for by commit, so its root tree object id is not a
/// captured fact.
fn tag_chain(
    out: &mut Vec<String>,
    t: TargetKind,
    pin: &Pin,
    tag: &str,
    ts: &TaggedSource,
    json: &std::collections::HashMap<DocKind, serde_json::Value>,
) {
    let s = |v: &serde_json::Value, p: &str| {
        v.pointer(p)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let kind = ts.ref_kind.unwrap_or(RefKind::Annotated);
    let tag_object = ts.tag_object.clone().unwrap_or_default();
    let ref_ok = json.get(&DocKind::GitRef).is_some_and(|r| {
        s(r, "/ref") == format!("refs/tags/{tag}")
            && match kind {
                RefKind::Annotated => {
                    s(r, "/object/type") == "tag" && s(r, "/object/sha") == tag_object
                }
                RefKind::Lightweight => {
                    s(r, "/object/type") == "commit" && s(r, "/object/sha") == ts.commit
                }
            }
    });
    if !ref_ok {
        match kind {
            RefKind::Annotated => out.push(format!(
                "{t}: the captured ref does not name tag object {tag_object}"
            )),
            RefKind::Lightweight => out.push(format!(
                "{t}: the captured ref does not name commit {} directly as refs/tags/{tag}",
                ts.commit
            )),
        }
    }
    if kind == RefKind::Annotated {
        let signature = ts.tag_signature.clone().unwrap_or_default();
        let tag_ok = json.get(&DocKind::GitTag).is_some_and(|g| {
            s(g, "/sha") == tag_object
                && s(g, "/tag") == *tag
                && s(g, "/object/type") == "commit"
                && s(g, "/object/sha") == ts.commit
                && s(g, "/verification/reason") == signature
        });
        if !tag_ok {
            out.push(format!(
                "{t}: the captured tag object does not name {tag}, commit {} and signature {signature:?}",
                ts.commit
            ));
        }
    }
    // The tree is asked for by commit (the tree's own id is then the commit,
    // Mailpit's and MailHog's captures) or by tree id, linked to the tagged
    // commit by a captured commit object whose `tree` names it. On the second
    // route the listing is rebuilt into git's tree object and hashed, so an
    // edited entry cannot vouch for a blob (ruling 2026-11-07).
    let commit_object = json.get(&DocKind::GitCommit);
    let route_ok = |tr: &serde_json::Value| match commit_object {
        None => s(tr, "/sha") == ts.commit,
        Some(c) => s(c, "/sha") == ts.commit && s(c, "/tree/sha") == s(tr, "/sha"),
    };
    if let Some(c) = commit_object {
        if s(c, "/sha") != ts.commit {
            out.push(format!(
                "{t}: the captured commit object is not the tagged commit {}",
                ts.commit
            ));
        } else if json
            .get(&DocKind::GitTree)
            .is_some_and(|tr| s(c, "/tree/sha") != s(tr, "/sha"))
        {
            out.push(format!(
                "{t}: the captured commit object names a tree other than the captured one"
            ));
        }
    }
    let tree = json
        .get(&DocKind::GitTree)
        .filter(|tr| route_ok(tr) && tr.get("truncated").and_then(|x| x.as_bool()) == Some(false))
        .and_then(|tr| tr.get("tree").and_then(|x| x.as_array()));
    let Some(tree) = tree else {
        out.push(format!(
            "{t}: the captured tree is not the complete listing of commit {}",
            ts.commit
        ));
        return;
    };
    if commit_object.is_some()
        && let Some(listing) = json.get(&DocKind::GitTree)
        && tree_object_id(tree).as_deref() != Some(s(listing, "/sha").as_str())
    {
        out.push(format!(
            "{t}: the captured tree listing does not rebuild to its own id"
        ));
    }
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

/// Git's tree object id of a non-recursive tree listing: SHA-1 over
/// `tree <len>\0` and, per entry in listed order, `<mode> <path>\0` and the
/// entry's 20 raw id bytes (`040000` is written `40000`). `None` for an
/// entry that is not one.
pub fn tree_object_id(entries: &[serde_json::Value]) -> Option<String> {
    let mut body = Vec::new();
    for e in entries {
        let mode = match e.get("mode")?.as_str()? {
            "040000" => "40000",
            m @ ("100644" | "100755" | "120000" | "160000") => m,
            _ => return None,
        };
        let path = e.get("path")?.as_str()?;
        let sha = e.get("sha")?.as_str()?;
        if !is_hex(sha, 40) {
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
    // A registry-checksum pin and a source build are artifacts the GitHub
    // release does not list; only the asset-list check is skipped, never
    // the tag, page, commitish or date above (ruling 2026-11-07).
    if a.digest_basis == DigestBasis::RegistryChecksum
        || matches!(t.provenance(), Provenance::SourceBuild(_))
    {
        return;
    }
    let asset = v
        .get("assets")
        .and_then(|x| x.as_array())
        .into_iter()
        .flatten()
        .find(|x| x.get("name").and_then(|n| n.as_str()) == Some(a.file_name.as_str()));
    let Some(asset) = asset else {
        out.push(format!(
            "{t}: release metadata lists no asset {}",
            a.file_name
        ));
        return;
    };
    if asset.get("browser_download_url").and_then(|x| x.as_str()) != Some(a.download.as_str())
        || asset.get("size").and_then(|x| x.as_u64()) != Some(a.size)
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

/// The version a release tag stands for: `release-1.2.3` and `v1.2.3` both
/// report `1.2.3`.
fn version_of(tag: &str) -> &str {
    let n = tag.strip_prefix("release-").unwrap_or(tag);
    n.strip_prefix('v').unwrap_or(n)
}

/// The pin facts lock versions 2, 3 and 4 share: release, artifact and its
/// digest basis, license, launch and storage. `launch` is `Some` for the
/// versions whose template is the fixed three-slot one; lock 4 checks its
/// launch and cap itself.
fn validate_common(
    out: &mut Vec<String>,
    t: TargetKind,
    (r, a, license): (&Release, &Artifact, &License),
    launch: Option<&[String]>,
    st: &StorageFacts,
) {
    let official = t.official_home();
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
    // Where the artifact comes from: an asset of the official release, a
    // package registry's file under a closed base URL (ruling 2026-11-07), or
    // (a source build) the tagged commit, which validate_pin checks because
    // it needs the resolved commit.
    match (a.digest_basis, t.provenance()) {
        (DigestBasis::RegistryChecksum, _) => match REGISTRY.iter().find(|(rt, _)| *rt == t) {
            Some((_, base)) => {
                if a.download != format!("{base}{}/{}", version_of(&r.tag), a.file_name) {
                    out.push(format!(
                        "{t}: download {:?} is not under the registry {base}",
                        a.download
                    ));
                }
            }
            None => out.push(format!(
                "{t}: no registry is recorded for it; the registry checksum basis is refused"
            )),
        },
        (_, Provenance::SourceBuild(_)) => {}
        _ => {
            if a.download != format!("{official}/releases/download/{}/{}", r.tag, a.file_name) {
                out.push(format!(
                    "{t}: download {:?} is not the official release asset",
                    a.download
                ));
            }
        }
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
        (DigestBasis::RegistryChecksum, Some(d)) if *d == a.sha256 => {}
        (DigestBasis::RegistryChecksum, _) => out.push(format!(
            "{t}: a registry checksum basis needs the published checksum, equal to the asset's"
        )),
    }
    match (a.kind, &a.member) {
        (ArtifactKind::Executable, None) if a.executable_sha256 == a.sha256 => {}
        (ArtifactKind::Executable, _) => out.push(format!(
            "{t}: a bare executable asset has no member and one digest"
        )),
        (ArtifactKind::Archive, Some(m)) if is_name(m) => {}
        (ArtifactKind::Archive, _) => out.push(format!("{t}: an archive pin names its member")),
    }
    if a.os.trim().is_empty() || a.arch.trim().is_empty() {
        out.push(format!("{t}: artifact platform is incomplete"));
    }
    // The version the tag stands for, or exactly the version the closed
    // REPORTED_VERSIONS list records for this target and tag.
    let reported = REPORTED_VERSIONS
        .iter()
        .any(|(rt, tag, v)| *rt == t && *tag == r.tag && a.version_output.contains(v));
    if !a.version_output.contains(version_of(&r.tag)) && !reported {
        out.push(format!(
            "{t}: the observed version output does not name {}",
            r.tag
        ));
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
    if let Some(launch) = launch {
        if !launch_template_ok(launch) {
            out.push(format!("{t}: launch template is missing or malformed"));
        }
        cap_check(out, t, st, launch, &[]);
    }
    if st.basis.trim().is_empty() {
        out.push(format!("{t}: storage facts state no basis"));
    }
}

/// The retention cap and the flag that sets it go together, and the launch
/// really sets it: an argv `FLAG N` pair or `FLAG=N`, or (`env:NAME`) an
/// environment entry. A default the launch does not override never counts.
fn cap_check(
    out: &mut Vec<String>,
    t: TargetKind,
    st: &StorageFacts,
    launch: &[String],
    env: &[[String; 2]],
) {
    match (st.max_messages, &st.cap_flag) {
        (Some(n), Some(flag)) => {
            let n = n.to_string();
            let set = match flag.strip_prefix("env:") {
                Some(name) => env.iter().any(|[k, v]| k == name && *v == n),
                None => {
                    launch.windows(2).any(|w| w[0] == *flag && w[1] == n)
                        || launch.iter().any(|a| *a == format!("{flag}={n}"))
                }
            };
            if !set {
                out.push(format!("{t}: the launch does not set {flag} {n}"));
            }
        }
        (None, None) => {}
        _ => out.push(format!(
            "{t}: a retention cap and its launch flag go together"
        )),
    }
}

/// A document path confined to `sources/<target>/`, listed once, with a
/// well-formed digest.
fn document_path(out: &mut Vec<String>, t: TargetKind, earlier: &[&str], path: &str, sha: &str) {
    let prefix = format!("sources/{t}/");
    if !path.strip_prefix(&prefix).is_some_and(is_name) {
        out.push(format!("{t}: document path {path:?} is not under {prefix}"));
    }
    if earlier.contains(&path) {
        out.push(format!("{t}: document {path} is listed twice"));
    }
    if !is_hex(sha, 64) {
        out.push(format!("{t}: {path} digest is not 64 lowercase hex digits"));
    }
}

/// `sha256:` and 64 lowercase hex digits.
fn is_digest(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|h| is_hex(h, 64))
}

/// Checked members: at most 8, each a confined relative path with a digest,
/// each listed once.
fn checked_members(out: &mut Vec<String>, t: TargetKind, whose: &str, m: &[CheckedMember]) {
    if m.len() > 8 {
        out.push(format!("{t}: {whose} lists more than 8 checked members"));
    }
    for (i, c) in m.iter().enumerate() {
        if !is_upstream_path(&c.path) || !is_hex(&c.sha256, 64) {
            out.push(format!("{t}: {whose} member {:?} is malformed", c.path));
        }
        if m[..i].iter().any(|o| o.path == c.path) {
            out.push(format!("{t}: {whose} member {} is listed twice", c.path));
        }
    }
}

fn validate_runtime(out: &mut Vec<String>, t: TargetKind, pin: &Pin) {
    let rule = runtime_rule(t);
    let Some(rt) = &pin.runtime else {
        if let Some(rule) = rule {
            out.push(format!(
                "{t}: {t} needs a pinned runtime (its entry program {})",
                rule.entry
            ));
        }
        return;
    };
    let Some(rule) = rule else {
        out.push(format!(
            "{t}: only greenmail (a JRE) and alkemio-mailslurper (a musl loader) pin a runtime"
        ));
        return;
    };
    if !is_name(&rt.name) || !rt.download.starts_with("https://") || !is_hex(&rt.sha256, 64) {
        out.push(format!(
            "{t}: the runtime needs a name, an https download and a sha256"
        ));
    }
    checked_members(out, t, "runtime", &rt.checked_members);
    for must in rule.members {
        if !rt.checked_members.iter().any(|m| m.path == *must) {
            out.push(format!("{t}: the runtime does not check {must}"));
        }
    }
    if rule.image_layer {
        // Taken from the image's own layer: the runtime's archive digest is
        // a layer of the pin's captured manifest (verify_files checks the
        // manifest), and its download names that blob.
        if pin.image.is_none() {
            out.push(format!(
                "{t}: a loader runtime is a layer of the pin's image"
            ));
        }
        if !rt
            .download
            .ends_with(&format!("/blobs/sha256:{}", rt.sha256))
        {
            out.push(format!(
                "{t}: the loader runtime's download is not the image blob of its own sha256"
            ));
        }
    }
    if rt.version_output.trim().is_empty() {
        out.push(format!("{t}: the runtime's version output is not recorded"));
    }
    if !RUNTIME_LICENSES.contains(&rt.license.spdx.as_str()) {
        out.push(format!(
            "{t}: runtime license {:?} is not on the runtime list",
            rt.license.spdx
        ));
    }
    if rt.license.holder.trim().is_empty() {
        out.push(format!("{t}: the runtime license holder is not recorded"));
    }
    if !pin.launch.iter().any(|a| a.contains("{artifact}")) {
        out.push(format!(
            "{t}: a runtime launch names the jar as {{artifact}}"
        ));
    }
}

fn validate_image(out: &mut Vec<String>, t: TargetKind, pin: &Pin, img: &ImageSource, want: &str) {
    if img.reference != want {
        out.push(format!(
            "{t}: image reference {:?} is not the labelled source {want}",
            img.reference
        ));
    }
    for (name, d) in [
        ("index_digest", &img.index_digest),
        ("manifest_digest", &img.manifest_digest),
        ("layer_digest", &img.layer_digest),
    ] {
        if !is_digest(d) {
            out.push(format!("{t}: image {name} {d:?} is not sha256:<64 hex>"));
        }
    }
    if img.platform.split('/').count() != 2 || img.platform.split('/').any(str::is_empty) {
        out.push(format!(
            "{t}: image platform {:?} is not os/arch",
            img.platform
        ));
    }
    if !is_upstream_path(&img.member) {
        out.push(format!(
            "{t}: image member {:?} is not a confined path",
            img.member
        ));
    }
    let a = &pin.artifact;
    let base = img.member.rsplit('/').next().unwrap_or_default();
    if a.kind != ArtifactKind::Executable
        || a.member.as_deref() != Some(img.member.as_str())
        || a.file_name != base
        || a.download != img.reference
        || a.digest_basis != DigestBasis::LocallyComputed
        || a.upstream_digest.is_some()
        || a.sha256 != a.executable_sha256
        || !is_hex(&a.sha256, 64)
    {
        out.push(format!(
            "{t}: an image pin's artifact is the extracted member: an executable of that member, named by its leaf, locally digested once"
        ));
    }
    if !is_name(&a.file_name) || a.os.trim().is_empty() || a.arch.trim().is_empty() {
        out.push(format!("{t}: artifact name or platform is incomplete"));
    }
    if a.version_output.trim().is_empty() {
        out.push(format!(
            "{t}: the extracted executable's version output is not recorded"
        ));
    }
    if !PERMISSIVE.contains(&pin.license.spdx.as_str()) {
        out.push(format!(
            "{t}: license {:?} is not on the permissive list",
            pin.license.spdx
        ));
    }
    if pin.license.holder.trim().is_empty() {
        out.push(format!("{t}: license holder is not recorded"));
    }
    // An image has no tag and no build record: nothing may be claimed.
    if matches!(pin.tagged_source, Fact::Observed(_)) {
        out.push(format!("{t}: an image pin has no tagged source to observe"));
    }
    if matches!(pin.binary_build_provenance, Fact::Observed(_)) {
        out.push(format!(
            "{t}: an image pin has no binary build provenance to observe"
        ));
    }
    if !pin.wait.reviewed.is_empty() {
        out.push(format!(
            "{t}: an image has no tagged tree to review sources in"
        ));
    }
    for (kind, digest) in [
        (DocKind::ImageIndex, &img.index_digest),
        (DocKind::ImageManifest, &img.manifest_digest),
    ] {
        let want = digest.strip_prefix("sha256:").unwrap_or_default();
        if !pin
            .documents
            .iter()
            .any(|d| d.kind == kind && d.sha256 == want)
        {
            out.push(format!(
                "{t}: no captured {kind:?} document with digest {digest}"
            ));
        }
    }
}

fn validate_pending(out: &mut Vec<String>, t: TargetKind, p: &Pending) {
    match (t.provenance(), &p.release_tag, &p.image) {
        (Provenance::Official(_), Some(tag), None) => {
            if !is_version_tag(tag) {
                out.push(format!(
                    "{t}: pending release tag {tag:?} is not a version tag"
                ));
            }
        }
        (Provenance::ThirdPartyImage(r), None, Some(img)) => {
            if img != r {
                out.push(format!(
                    "{t}: pending image {img:?} is not the labelled {r}"
                ));
            }
        }
        _ => out.push(format!(
            "{t}: pending names the wrong kind of source: an official target has a release tag, the third-party image an image reference"
        )),
    }
    if p.reason.trim().is_empty() {
        out.push(format!("{t}: a pending entry states no reason"));
    }
    if p.launch.is_empty() && p.launch_env.is_empty() && p.config_file.is_none() {
        if !p.listeners.contains(&ListenerRole::Smtp) {
            out.push(format!("{t}: pending listeners lack the smtp role"));
        }
    } else {
        launch_checks(
            out,
            t,
            &p.listeners,
            &p.launch,
            &p.launch_env,
            p.config_file.as_ref(),
        );
    }
}

fn validate_pin(out: &mut Vec<String>, t: TargetKind, pin: &Pin, v4: bool) {
    if !v4 {
        for (field, present) in [
            ("image", pin.image.is_some()),
            ("runtime", pin.runtime.is_some()),
            ("listeners", pin.listeners.is_some()),
            ("launch_env", pin.launch_env.is_some()),
            ("config_file", pin.config_file.is_some()),
            ("checked_members", pin.artifact.checked_members.is_some()),
            (
                "ref_kind",
                matches!(&pin.tagged_source, Fact::Observed(ts) if ts.ref_kind.is_some()),
            ),
        ] {
            if present {
                out.push(format!("{t}: lock_version {LOCK_VERSION} refuses {field}"));
            }
        }
    }
    match (&pin.release, &pin.image, t.provenance()) {
        (Some(rel), None, Provenance::Official(_) | Provenance::SourceBuild(_)) => {
            validate_common(
                out,
                t,
                (rel, &pin.artifact, &pin.license),
                (!v4).then_some(&pin.launch[..]),
                &pin.storage,
            );
            if v4 && !is_version_tag(&rel.tag) {
                out.push(format!(
                    "{t}: release tag {:?} is not a version tag (v1.2.3, release-1.2.3 or 1.2.3)",
                    rel.tag
                ));
            }
        }
        (None, Some(img), Provenance::ThirdPartyImage(want)) if v4 => {
            validate_image(out, t, pin, img, want);
        }
        (None, Some(_), _) if !v4 => out.push(format!("{t}: a lock 3 pin needs a release")),
        (None, None, _) => out.push(format!(
            "{t}: a pin needs a release (or, at lock_version {LOCK_VERSION_4}, an image)"
        )),
        (Some(_), Some(_), _) => out.push(format!("{t}: a pin is a release or an image, not both")),
        (Some(_), None, _) => out.push(format!(
            "{t}: its provenance is not an official release; a release cannot stand for it"
        )),
        (None, Some(_), _) => out.push(format!(
            "{t}: its provenance is not a third-party image; an image cannot stand for it"
        )),
    }
    if v4 {
        let env = pin.launch_env.as_deref().unwrap_or_default();
        match &pin.listeners {
            Some(roles) => launch_checks(out, t, roles, &pin.launch, env, pin.config_file.as_ref()),
            None => out.push(format!(
                "{t}: lock_version {LOCK_VERSION_4} needs the declared listeners"
            )),
        }
        cap_check(out, t, &pin.storage, &pin.launch, env);
        validate_runtime(out, t, pin);
        if let Some(m) = &pin.artifact.checked_members {
            checked_members(out, t, "artifact", m);
        }
    }
    let official = t.official_home();
    if !is_date(&pin.acquisition.captured_on) {
        out.push(format!(
            "{t}: captured_on {:?} is not YYYY-MM-DD",
            pin.acquisition.captured_on
        ));
    }
    let has = |kind: DocKind| pin.documents.iter().any(|d| d.kind == kind);
    let commit = match &pin.tagged_source {
        Fact::Observed(ts) if pin.image.is_none() => {
            let kind = match (v4, ts.ref_kind) {
                (true, None) => {
                    out.push(format!(
                        "{t}: lock_version {LOCK_VERSION_4} needs ref_kind on a tagged pin"
                    ));
                    RefKind::Annotated
                }
                (true, Some(k)) => k,
                (false, _) => RefKind::Annotated,
            };
            match kind {
                RefKind::Annotated => {
                    if !is_hex(&ts.commit, 40)
                        || !ts.tag_object.as_deref().is_some_and(|o| is_hex(o, 40))
                    {
                        out.push(format!(
                            "{t}: tagged source needs a full tag object and commit id (a tag name is mutable)"
                        ));
                    }
                    if ts
                        .tag_signature
                        .as_deref()
                        .is_none_or(|sig| sig.trim().is_empty())
                    {
                        out.push(format!("{t}: the tag's signature status is not recorded"));
                    }
                    for k in [DocKind::GitRef, DocKind::GitTag, DocKind::GitTree] {
                        if !has(k) {
                            out.push(format!("{t}: tagged source has no captured {k:?} document"));
                        }
                    }
                }
                RefKind::Lightweight => {
                    if !is_hex(&ts.commit, 40) {
                        out.push(format!(
                            "{t}: tagged source needs a full commit id (a tag name is mutable)"
                        ));
                    }
                    if ts.tag_object.is_some() || ts.tag_signature.is_some() {
                        out.push(format!(
                            "{t}: a lightweight tag has no tag object and no signature to record"
                        ));
                    }
                    if has(DocKind::GitTag) {
                        out.push(format!("{t}: a lightweight tag has no git_tag document"));
                    }
                    for k in [DocKind::GitRef, DocKind::GitTree] {
                        if !has(k) {
                            out.push(format!("{t}: tagged source has no captured {k:?} document"));
                        }
                    }
                }
            }
            Some(ts.commit.as_str())
        }
        Fact::Observed(_) => None,
        Fact::Unresolved(why) => {
            gap(out, t, "tagged_source", why);
            None
        }
    };
    if let Provenance::SourceBuild(home) = t.provenance() {
        // A source build is the operator's supply, built from the validated
        // tagged commit: its download is that commit's tree, its digest is
        // computed locally, and its build record is observed (ruling
        // 2026-11-07), never unresolved.
        let a = &pin.artifact;
        match commit {
            Some(c) => {
                if a.download != format!("{home}/tree/{c}") {
                    out.push(format!(
                        "{t}: a source build's download is the tagged commit's tree {home}/tree/{c}"
                    ));
                }
                match &pin.binary_build_provenance {
                    Fact::Observed(bp) if bp.commit == c => {}
                    Fact::Observed(_) => out.push(format!(
                        "{t}: the build provenance names a commit other than the tagged {c}"
                    )),
                    Fact::Unresolved(_) => out.push(format!(
                        "{t}: a source build needs observed build provenance, not an unresolved one"
                    )),
                }
            }
            None => out.push(format!(
                "{t}: a source build needs an observed tagged source to be built from"
            )),
        }
        if a.digest_basis != DigestBasis::LocallyComputed
            || a.kind != ArtifactKind::Executable
            || a.member.is_some()
        {
            out.push(format!(
                "{t}: a source build is a bare executable digested locally"
            ));
        }
    }
    match &pin.binary_build_provenance {
        Fact::Observed(bp) if pin.image.is_none() => {
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
        Fact::Observed(_) => {}
        Fact::Unresolved(why) => gap(out, t, "binary_build_provenance", why),
    }
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
        if matches!(d.kind, DocKind::ImageIndex | DocKind::ImageManifest) && pin.image.is_none() {
            out.push(format!(
                "{t}: {} is an image document on a pin with no image",
                d.path
            ));
        }
        match &d.attribution {
            Fact::Observed(Attribution::Api { url }) => {
                if !d.kind.is_api_object() {
                    out.push(format!(
                        "{t}: {} is a repository file; an API object cannot attribute it",
                        d.path
                    ));
                }
                if pin.image.is_some() || !url.starts_with(&api_home) {
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
            Fact::Observed(Attribution::Image { reference }) => {
                if pin.image.as_ref().map(|i| &i.reference) != Some(reference) {
                    out.push(format!(
                        "{t}: {} is attributed to an image the pin does not name",
                        d.path
                    ));
                }
            }
            Fact::Observed(Attribution::Member {
                archive_sha256,
                member,
            }) => {
                if d.kind.is_api_object() {
                    out.push(format!(
                        "{t}: {} is an API object, not a member of an archive",
                        d.path
                    ));
                }
                if !is_hex(archive_sha256, 64) || !is_upstream_path(member) {
                    out.push(format!(
                        "{t}: {} attribution needs an archive digest and a confined member path",
                        d.path
                    ));
                }
                let allowed = *archive_sha256 == pin.artifact.sha256
                    || pin
                        .runtime
                        .as_ref()
                        .is_some_and(|r| r.sha256 == *archive_sha256)
                    || SOURCE_ARCHIVES.iter().any(|(targets, sha, c)| {
                        targets.contains(&t) && sha == archive_sha256 && commit == Some(c)
                    });
                if !allowed {
                    out.push(format!(
                        "{t}: {} is attributed to an archive that is neither the pin's artifact, its runtime nor a source archive of its tagged commit",
                        d.path
                    ));
                }
            }
            Fact::Observed(Attribution::Output { argv }) => {
                if d.kind.is_api_object()
                    || argv.is_empty()
                    || argv.len() > 8
                    || argv.iter().any(|x| {
                        x.is_empty() || x.len() > 64 || !x.chars().all(|c| c.is_ascii_graphic())
                    })
                {
                    out.push(format!(
                        "{t}: {} output attribution needs a short, non-empty, printable argv",
                        d.path
                    ));
                }
            }
            Fact::Observed(Attribution::Supplied { record }) => {
                if d.kind.is_api_object()
                    || record.trim().is_empty()
                    || record.len() > 1024
                    || !record.contains(".compare-bins/")
                    || !record.contains(&d.sha256)
                {
                    out.push(format!(
                        "{t}: {} supplied attribution must name the supply path and carry the document's sha256",
                        d.path
                    ));
                }
                if d.kind == DocKind::Checksum
                    && !record.contains(&format!("{}.sha256", pin.artifact.download))
                {
                    out.push(format!(
                        "{t}: the checksum record must name {}.sha256",
                        pin.artifact.download
                    ));
                }
            }
            Fact::Unresolved(why) => gap(out, t, &d.path, why),
        }
    }
    let required: &[DocKind] = if pin.image.is_some() {
        &[DocKind::License, DocKind::Api]
    } else {
        &[DocKind::ReleaseMetadata, DocKind::License, DocKind::Api]
    };
    for kind in required {
        if !has(*kind) {
            out.push(format!("{t}: no captured {kind:?} document"));
        }
    }
    if pin.artifact.digest_basis == DigestBasis::RegistryChecksum && !has(DocKind::Checksum) {
        out.push(format!(
            "{t}: a registry checksum pin needs its captured Checksum document"
        ));
    }
    // A registry's checksum and a build record are operator-supplied
    // evidence; no other attribution would be true of them.
    for d in &pin.documents {
        let supplied = matches!(d.attribution, Fact::Observed(Attribution::Supplied { .. }));
        if matches!(d.kind, DocKind::Checksum | DocKind::BuildEvidence)
            && !supplied
            && d.attribution.gap().is_none()
        {
            out.push(format!(
                "{t}: {} is supplied evidence and must be attributed as supplied",
                d.path
            ));
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

fn gap(out: &mut Vec<String>, t: TargetKind, what: &str, why: &str) {
    if why.trim().is_empty() {
        out.push(format!("{t}: {what} is unresolved without a reason"));
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
        let locked = pin.release.as_ref().map(|r| r.tag.as_str());
        if product.release_tag.as_deref() != locked {
            out.push(format!(
                "{}: protocol release {:?} is not the locked {}",
                product.target,
                product.release_tag,
                locked.unwrap_or("(no release: an image)")
            ));
        }
        let kind = match (&pin.release, &pin.image) {
            (Some(_), None)
                if matches!(product.target.provenance(), Provenance::SourceBuild(_)) =>
            {
                Some(ProductScope::SourceBuild)
            }
            (Some(_), None) => Some(ProductScope::OfficialArtifact),
            (None, Some(_)) => Some(ProductScope::ThirdPartyImage),
            _ => None,
        };
        if product.scope.is_some() && product.scope != kind {
            out.push(format!(
                "{}: protocol scope {:?} is not what the lock's pin is",
                product.target, product.scope
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

/// The competitors a protocol declares, in its own order: every declared
/// target but the candidate, which the wrapper builds itself. The wrapper
/// derives its required binaries from this, never from a fixed list.
pub fn declared_competitors(p: &Protocol) -> Vec<TargetKind> {
    p.targets
        .iter()
        .copied()
        .filter(|t| *t != TargetKind::Swarmail && !p.does_not_start(*t))
        .collect()
}

/// A published does-not-start result needs the lock behind it: the target
/// has a pin, and the cite is a document of that pin (the captured failure
/// output). The row keeps its pin, product and denominator; it is only
/// never launched.
pub fn does_not_start_matches(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    for d in p.does_not_start.iter().flatten() {
        let t = d.target;
        match lock.pin(t) {
            None => out.push(format!(
                "does_not_start: {t} has no pin, so there is no result to publish for it"
            )),
            Some(pin) => {
                if !pin.documents.iter().any(|x| x.path == d.cite) {
                    out.push(format!(
                        "does_not_start: {t} cites {:?}, which is no document of its pin",
                        d.cite
                    ));
                }
            }
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}

/// Whether every declared target can launch: the candidate by its role, a
/// competitor by a pin. A pending entry never launches, and the refusal
/// names it and says why it is pending.
pub fn launchable(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    for t in &p.targets {
        let ok = match lock.entry(*t) {
            Some(e) if *t == TargetKind::Swarmail => e.role == Role::Candidate,
            Some(e) => e.pin.is_some(),
            None => false,
        };
        if !ok {
            let why = lock
                .entry(*t)
                .and_then(|e| e.pending.as_ref())
                .map(|p| format!(" (pending: {})", p.reason))
                .unwrap_or_default();
            out.push(format!("{t} has no source pin; it cannot be launched{why}"));
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}

/// Protocol 2 (and the version 1 drafts) pair only with lock 3, protocol 3
/// only with lock 4: neither reads the other's records (decision 0130 §3).
pub fn pairing(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let want = if p.protocol_version == PROTOCOL_VERSION_3 {
        LOCK_VERSION_4
    } else {
        LOCK_VERSION
    };
    if lock.lock_version == want {
        Ok(())
    } else {
        Err(Refusal(vec![format!(
            "protocol_version {} pairs only with lock_version {want}, not {}",
            p.protocol_version, lock.lock_version
        )]))
    }
}

/// Every capability citation names a document or reviewed source of that
/// target's pin (the candidate's names a repository path with `repo:`), a
/// pending target states nothing it cannot cite, and a stated retention cap
/// agrees with the flag the launch sets.
pub fn capabilities_match(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    for c in p.capabilities.iter().flatten() {
        let t = c.target;
        let entry = lock.entry(t);
        for (key, cap) in c.each() {
            let Some(cite) = cap.cite.as_deref() else {
                continue;
            };
            match (t.provenance(), entry.and_then(|e| e.pin.as_ref())) {
                (Provenance::Candidate, _) => {
                    if !cite.starts_with("repo:") {
                        out.push(format!(
                            "capabilities: {t} {key} cites {cite:?}, not a repo: path"
                        ));
                    }
                }
                (_, Some(pin)) => {
                    let known = pin.documents.iter().any(|d| d.path == cite)
                        || pin.wait.reviewed.iter().any(|r| r.upstream_path == cite);
                    if !known {
                        out.push(format!(
                            "capabilities: {t} {key} cites {cite:?}, which is no document or reviewed source of its pin"
                        ));
                    }
                }
                (_, None) => out.push(format!(
                    "capabilities: {t} {key} cites {cite:?}, but {t} has no pin to cite"
                )),
            }
        }
        if let Some(pin) = entry.and_then(|e| e.pin.as_ref()) {
            let capped = pin.storage.cap_flag.is_some();
            match (c.retention_cap.status, capped) {
                (CapabilityStatus::Supported, false) => out.push(format!(
                    "capabilities: {t} retention_cap is supported but its launch sets no cap"
                )),
                (CapabilityStatus::NotSupported, true) => out.push(format!(
                    "capabilities: {t} retention_cap is not_supported but its launch sets one"
                )),
                _ => {}
            }
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
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
    if let Err(Refusal(r)) = pairing(p, lock) {
        out.extend(r);
    }
    if let Err(Refusal(r)) = capabilities_match(p, lock) {
        out.extend(r);
    }
    if let Err(Refusal(r)) = does_not_start_matches(p, lock) {
        out.extend(r);
    }
    if p.status != Status::Frozen {
        out.push("protocol is a draft, not frozen".into());
    }
    if !matches!(p.protocol_version, PROTOCOL_VERSION | PROTOCOL_VERSION_3) {
        out.push(format!(
            "protocol version {} is a historical draft; only versions {PROTOCOL_VERSION} and {PROTOCOL_VERSION_3} collect",
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
        if let Some(pending) = &e.pending {
            out.push(format!(
                "{t}: pending, no verified pin; it cannot collect: {}",
                pending.reason
            ));
            continue;
        }
        let Some(pin) = &e.pin else { continue };
        // An image pin has no tag of an official repository to observe
        // (validate_image refuses an observed one), so the third-party
        // image's identity is its index -> manifest -> layer digest chain,
        // and its tagged-source gap is structural, not missing. The
        // exemption is the closed provenance's, never the lock data's:
        // every other pin still needs an observed tagged source.
        if let Some(why) = pin.tagged_source.gap()
            && t.provenance().has_tagged_source()
        {
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
            for t in TargetKind::ALL {
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
        let official = t.official_home();
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
            validate_entries(&mut out, &self.checked_on, &self.entries, |e| {
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
            Some(&pin.launch),
            &pin.storage,
        );
        let official = t.official_home();
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
