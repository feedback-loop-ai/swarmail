//! The frozen experiment (spec R1) and the competitor source lock: closed,
//! versioned records whose validators refuse before any collection. A
//! draft protocol or an unverified source pin may drive the labelled smoke
//! run; it can never become measured evidence.

use serde::{Deserialize, Serialize};
use std::fmt;

pub const PROTOCOL_VERSION: u32 = 1;
pub const LOCK_VERSION: u32 = 1;

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
}

impl TargetKind {
    pub const ALL: [TargetKind; 3] = [
        TargetKind::Swarmail,
        TargetKind::Mailpit,
        TargetKind::Mailhog,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TargetKind::Swarmail => "swarmail",
            TargetKind::Mailpit => "mailpit",
            TargetKind::Mailhog => "mailhog",
        }
    }

    pub fn parse(name: &str) -> Option<TargetKind> {
        TargetKind::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The upstream home a competitor's official source must come from.
    /// Third-party mirrors, recipes and images cannot stand in (spec R1).
    /// Swarmail is the candidate itself and has no upstream. These homes
    /// were written from the author's knowledge, not captured: the first
    /// official capture confirms them (docs/evals/README.md).
    pub fn official_repository(self) -> Option<&'static str> {
        match self {
            TargetKind::Swarmail => None,
            TargetKind::Mailpit => Some("https://github.com/axllent/mailpit"),
            TargetKind::Mailhog => Some("https://github.com/mailhog/MailHog"),
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    /// `memory` is the matched primary profile; it is valid only when every
    /// target's lock entry records that it supports memory-only storage.
    pub mode: String,
    /// Per-target retention settings, stated as launched. An exact-count
    /// workload may never knowingly overflow one (spec R3).
    pub retention: Vec<Retention>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    pub target: TargetKind,
    /// `None` = unlimited, stated explicitly.
    pub max_messages: Option<u64>,
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
        if self.protocol_version != PROTOCOL_VERSION {
            out.push(format!(
                "protocol_version {} is not {PROTOCOL_VERSION}",
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
        if !matches!(self.storage.mode.as_str(), "memory" | "durable") {
            out.push(format!(
                "storage mode {:?} is neither memory nor durable",
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
        if out.is_empty() {
            Ok(())
        } else {
            Err(Refusal(out))
        }
    }

    /// The target order of repetition `rep` (0-based), counterbalanced by
    /// the protocol's own list.
    pub fn order(&self, rep: u32) -> &[TargetKind] {
        &self.statistics.orders[rep as usize % self.statistics.orders.len()]
    }
}

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

pub const LAUNCH_SLOTS: [&str; 3] = ["{smtp_addr}", "{http_addr}", "{data_dir}"];

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

/// The permissive licenses a pinned experiment input may carry: the
/// subset of `deny.toml`'s allowlist a server release could use. The
/// binary is not linked, but its terms still have to be checkable.
const PERMISSIVE: [&str; 5] = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"];

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
        if self.lock_version != LOCK_VERSION {
            out.push(format!(
                "lock_version {} is not {LOCK_VERSION}",
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

/// Collection admission: a frozen, structurally valid protocol with nothing
/// unresolved, an enforced resource budget, and a verified pin (or the
/// candidate) for every declared target. Every refusal is listed, so the
/// one command can say exactly which phase is blocked.
pub fn collection_ready(p: &Protocol, lock: &SourceLock) -> Result<(), Refusal> {
    let mut out = Vec::new();
    if let Err(Refusal(r)) = p.validate() {
        out.extend(r);
    }
    if let Err(Refusal(r)) = lock.validate() {
        out.extend(r);
    }
    if p.status != Status::Frozen {
        out.push("protocol is a draft, not frozen".into());
    }
    for u in &p.unresolved {
        out.push(format!("unresolved: {u}"));
    }
    if !p.resources.enforced {
        out.push("resource budget is not enforced by the selected runtime".into());
    }
    for t in &p.targets {
        match lock.entry(*t) {
            None => out.push(format!("{t} has no source lock entry")),
            Some(e) if e.status == PinStatus::Unverified => out.push(format!(
                "{t} source is unverified: {}",
                e.unverified_reason.as_deref().unwrap_or_default()
            )),
            Some(e)
                if e.status == PinStatus::Verified
                    && p.storage.mode == "memory"
                    && e.supports_memory_storage != Some(true) =>
            {
                out.push(format!("{t} does not record memory-only storage support"))
            }
            Some(_) => {}
        }
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(Refusal(out))
    }
}
