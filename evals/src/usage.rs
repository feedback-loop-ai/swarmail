//! Nested-driver usage: the six token facts of one paid session, read from
//! the session-final checkpoint the pinned brokkr driver authors (route (b) of
//! decision 0119's 2026-11-23 addendum; accepted 0136; proposed 0138).
//!
//! The harness, never the agent, authors this record: the wrapper folds the
//! driver's own stdout, sends the projection to the runner on the launch pipe
//! the CLI never holds, and the runner publishes it as the trial leaf's usage
//! companion, bound by digest in the leaf's metadata commit beside the
//! unchanged row, so the join is the row's `trial_id` (proposed 0138). Each of the six facts stands on
//! its own: a value, or one fixed reason it is unavailable. A missing figure
//! is never `0`, and an explicit `0` is kept.
//!
//! Only the family's session-final step counts. Per-turn rows (`seat-turn`,
//! `turn-completed`) and the result object (which the agent's own file
//! populates) carry no accounting authority here.

use crate::runner::TrialResult;
use crate::tasks::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// The source every nested usage record names.
pub const SOURCE: &str = "via_brokkr_driver";
/// The largest integer a count may hold: JSON's exact-integer range.
pub const COUNT_LIMIT: u64 = 9_007_199_254_740_991;
/// The six facts, in record order.
pub const FIELDS: [&str; 6] = [
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "num_turns",
    "total_cost_usd",
];

/// The checkpoint step that carries a family's session-final totals.
pub fn final_step(family: &str) -> Option<&'static str> {
    match family {
        "claude" => Some("claude-code-session-finished"),
        "codex" => Some("codex-session-finished"),
        "dsh" => Some("deepseek-harness-session-finished"),
        _ => None,
    }
}

/// Why one fact is not a number. A closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The exchange carried no session-final checkpoint.
    NoSessionFinal,
    /// The session-final checkpoint does not carry the field.
    FieldAbsent,
    /// The field is not a representable nonnegative value.
    FieldMalformed,
    /// Two session-final checkpoints disagree on the field.
    FinalConflicting,
    /// The wrapper's receipt never reached the runner (a kill, a crash).
    ReceiptMissing,
    /// The sessions' checked sum left the exact range.
    SumOverflow,
}

/// One fact: a value or why there is none.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Fact<T> {
    Present { value: T },
    Unavailable { reason: Reason },
}

impl<T: Copy> Fact<T> {
    pub fn value(&self) -> Option<T> {
        match self {
            Self::Present { value } => Some(*value),
            Self::Unavailable { .. } => None,
        }
    }
}

/// The six facts of one session.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    pub input_tokens: Fact<u64>,
    pub output_tokens: Fact<u64>,
    pub cache_read_tokens: Fact<u64>,
    pub cache_write_tokens: Fact<u64>,
    pub num_turns: Fact<u64>,
    pub total_cost_usd: Fact<f64>,
}

impl Facts {
    /// Every fact unavailable for one reason.
    pub fn unavailable(reason: Reason) -> Self {
        let u = Fact::Unavailable { reason };
        Self {
            input_tokens: u,
            output_tokens: u,
            cache_read_tokens: u,
            cache_write_tokens: u,
            num_turns: u,
            total_cost_usd: Fact::Unavailable { reason },
        }
    }
    fn validate(&self) -> Result<(), String> {
        let counts = [
            self.input_tokens,
            self.output_tokens,
            self.cache_read_tokens,
            self.cache_write_tokens,
            self.num_turns,
        ];
        if counts
            .iter()
            .any(|c| c.value().is_some_and(|v| v > COUNT_LIMIT))
        {
            return Err("usage_count_invalid".into());
        }
        if self
            .total_cost_usd
            .value()
            .is_some_and(|v| !v.is_finite() || v < 0.0)
        {
            return Err("usage_cost_invalid".into());
        }
        Ok(())
    }
}

/// What the wrapper sends the runner after the exchange: the facts and the
/// attempt they belong to. The runner adds the trial it owns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// `completed` for a launched session that returned its one result, or
    /// the harness-error code the exchange ended with.
    pub disposition: String,
    pub attempt_id: String,
    pub family: String,
    pub facts: Facts,
}

/// The usage companion's file name in a retained trial leaf (proposed
/// 0138): the harness's own record beside `result.json`, bound by digest in
/// the same `metadata.json` commit. The schema-2 row does not change.
pub const COMPANION_FILE: &str = "usage.json";
/// The companion's format version.
pub const COMPANION_VERSION: u32 = 1;
/// The disposition of a session whose wrapper said `ready` and then sent no
/// receipt (a kill, a crash, a deadline).
pub const RECEIPT_MISSING: &str = "receipt_missing";
/// The disposition of an agent whose wrapper never launched its session.
pub const UNSTARTED: &str = "agent_unstarted";

/// One agent session of a nested trial, in the trial's assignment order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    /// The agent's index in the trial's assignment (0 for a one-agent task).
    pub agent: u8,
    /// The driver exchange's attempt; empty when no receipt arrived.
    pub attempt_id: String,
    /// `completed`, the harness-error code the exchange ended with,
    /// [`RECEIPT_MISSING`] or [`UNSTARTED`].
    pub disposition: String,
    pub facts: Facts,
}

impl Session {
    /// The session an agent's receipt reports.
    pub fn from_receipt(agent: u8, r: &Receipt) -> Self {
        Self {
            agent,
            attempt_id: r.attempt_id.clone(),
            disposition: r.disposition.clone(),
            facts: r.facts,
        }
    }
    /// A session with no receipt: launched and lost, or never launched.
    pub fn without_receipt(agent: u8, disposition: &str) -> Self {
        Self {
            agent,
            attempt_id: String::new(),
            disposition: disposition.into(),
            facts: Facts::unavailable(Reason::ReceiptMissing),
        }
    }
    fn validate(&self) -> Result<(), String> {
        let lost = self.facts == Facts::unavailable(Reason::ReceiptMissing)
            && (self.disposition == RECEIPT_MISSING || self.disposition == UNSTARTED);
        let ok = if self.attempt_id.is_empty() {
            lost
        } else {
            plain_id(&self.attempt_id)
                && plain_code(&self.disposition)
                && self.disposition != RECEIPT_MISSING
                && self.disposition != UNSTARTED
        };
        if !ok {
            return Err("usage_attempt_invalid".into());
        }
        self.facts.validate()
    }
}

/// The harness-authored usage companion of one nested trial (0075 §4,
/// proposed 0138): every expected agent's session, listed by assignment.
/// The trial's figures are derived, never stored: [`Companion::totals`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Companion {
    pub version: u32,
    pub source: String,
    pub trial_id: String,
    /// The matrix slot the trial filled, when it ran in one.
    pub slot: Option<crate::provenance::MatrixCoordinates>,
    pub profile: crate::provenance::WrapperProfile,
    pub sessions: Vec<Session>,
}

impl Companion {
    /// The companion for a trial's settled sessions, bound to its row.
    pub fn new(row: &TrialResult, sessions: Vec<Session>) -> Option<Self> {
        let profile = nested_profile(row)?;
        Some(Self {
            version: COMPANION_VERSION,
            source: SOURCE.into(),
            trial_id: row.trial_id.clone(),
            slot: row.provenance.matrix.clone(),
            profile,
            sessions,
        })
    }
    /// The companion's ownership against the row it claims: version, source,
    /// trial, slot, profile, and a full roster of the task's agents in
    /// assignment order, each driver attempt owned once.
    pub fn validate(&self, row: &TrialResult) -> Result<(), String> {
        if self.version != COMPANION_VERSION || self.source != SOURCE {
            return Err("usage_version_unsupported".into());
        }
        if self.trial_id != row.trial_id || self.slot != row.provenance.matrix {
            return Err("usage_binding_mismatch".into());
        }
        if nested_profile(row) != Some(self.profile) {
            return Err("usage_profile_mismatch".into());
        }
        let agents = row
            .task
            .as_deref()
            .and_then(crate::tasks::find)
            .map_or(0, |t| t.agent_count as usize);
        if self.sessions.len() != agents
            || self
                .sessions
                .iter()
                .enumerate()
                .any(|(i, s)| s.agent as usize != i)
        {
            return Err("usage_roster_mismatch".into());
        }
        let mut owners = std::collections::BTreeSet::new();
        for session in &self.sessions {
            session.validate()?;
            if !session.attempt_id.is_empty() && !owners.insert(session.attempt_id.as_str()) {
                return Err("usage_attempt_duplicate".into());
            }
        }
        Ok(())
    }
    /// The driver attempts this companion owns.
    pub fn attempts(&self) -> impl Iterator<Item = &str> {
        self.sessions
            .iter()
            .map(|s| s.attempt_id.as_str())
            .filter(|a| !a.is_empty())
    }
    /// The trial's six figures: the checked per-field sum over every
    /// session. A session without the field makes it unavailable (its
    /// reason, never zero); a sum past the exact range is `sum_overflow`.
    pub fn totals(&self) -> Facts {
        let count = |get: fn(&Facts) -> Fact<u64>| {
            let mut sum = 0u64;
            for s in &self.sessions {
                match get(&s.facts) {
                    Fact::Present { value } => match sum.checked_add(value) {
                        Some(n) if n <= COUNT_LIMIT => sum = n,
                        _ => {
                            return Fact::Unavailable {
                                reason: Reason::SumOverflow,
                            };
                        }
                    },
                    unavailable => return unavailable,
                }
            }
            Fact::Present { value: sum }
        };
        let mut usd = 0f64;
        let mut cost = None;
        for s in &self.sessions {
            match s.facts.total_cost_usd {
                Fact::Present { value } => usd += value,
                unavailable => {
                    cost = Some(unavailable);
                    break;
                }
            }
        }
        Facts {
            input_tokens: count(|f| f.input_tokens),
            output_tokens: count(|f| f.output_tokens),
            cache_read_tokens: count(|f| f.cache_read_tokens),
            cache_write_tokens: count(|f| f.cache_write_tokens),
            num_turns: count(|f| f.num_turns),
            total_cost_usd: cost.unwrap_or(if usd.is_finite() {
                Fact::Present { value: usd }
            } else {
                Fact::Unavailable {
                    reason: Reason::SumOverflow,
                }
            }),
        }
    }
}

/// The amended via-brokkr profile a row's agent ran under, if any: the rows
/// whose usage only a companion may carry. A superseded via-brokkr profile
/// never launched the nested driver (it refuses `driver_contract_unknown`),
/// so its historical rows stay journal-route rows, readable as before.
pub fn nested_profile(row: &TrialResult) -> Option<crate::provenance::WrapperProfile> {
    match &row.provenance.agent.tool_surface {
        crate::provenance::ToolSurface::Wrapper { profile, .. }
            if profile.via_brokkr() && profile.approved() =>
        {
            Some(*profile)
        }
        _ => None,
    }
}

/// The largest receipt line the runner reads.
pub const RECEIPT_LIMIT: usize = 4096;
/// The receipt line's prefix on the launch pipe, after `ready\n`.
pub const RECEIPT_PREFIX: &[u8] = b"nested ";

fn plain_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

/// Parse the receipt line the wrapper wrote after `ready\n`, without its
/// newline. Anything else is no receipt.
pub fn parse_receipt(line: &[u8]) -> Option<Receipt> {
    let body = line.strip_prefix(RECEIPT_PREFIX)?;
    let r: Receipt = serde_json::from_slice(body).ok()?;
    (plain_id(&r.attempt_id)
        && final_step(&r.family).is_some()
        && plain_code(&r.disposition)
        && r.facts.validate().is_ok())
    .then_some(r)
}
fn plain_code(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

/// The receipt line the wrapper writes, newline included.
pub fn receipt_line(r: &Receipt) -> Vec<u8> {
    let mut out = RECEIPT_PREFIX.to_vec();
    out.extend(serde_json::to_vec(r).expect("a typed receipt always encodes"));
    out.push(b'\n');
    out
}

/// One field of one final checkpoint.
#[derive(Clone, Copy, PartialEq)]
enum Seen {
    Absent,
    Malformed,
    Count(u64),
    Usd(f64),
}

fn seen(data: &Map<String, Value>, field: &str) -> Seen {
    match data.get(field) {
        None => Seen::Absent,
        Some(v) if field == "total_cost_usd" => match v.as_f64() {
            Some(x) if v.is_number() && x.is_finite() && x >= 0.0 => Seen::Usd(x),
            _ => Seen::Malformed,
        },
        Some(v) => match v.as_u64() {
            Some(n) if n <= COUNT_LIMIT => Seen::Count(n),
            _ => Seen::Malformed,
        },
    }
}

fn fold(values: &[Seen]) -> Result<Seen, Reason> {
    let Some(first) = values.first() else {
        return Err(Reason::NoSessionFinal);
    };
    if values.contains(&Seen::Malformed) {
        return Err(Reason::FieldMalformed);
    }
    if values.iter().any(|v| v != first) {
        return Err(Reason::FinalConflicting);
    }
    match first {
        Seen::Absent => Err(Reason::FieldAbsent),
        v => Ok(*v),
    }
}

/// Project the family's session-final checkpoints (each a checkpoint frame's
/// `data`) into the six facts. Identical repeated finals count once.
pub fn project<'a>(family: &str, data: impl IntoIterator<Item = &'a Map<String, Value>>) -> Facts {
    let step = final_step(family);
    let finals: Vec<_> = data
        .into_iter()
        .filter(|d| step.is_some() && d.get("step").and_then(Value::as_str) == step)
        .collect();
    let field = |name: &str| fold(&finals.iter().map(|d| seen(d, name)).collect::<Vec<_>>());
    let count = |name: &str| match field(name) {
        Ok(Seen::Count(n)) => Fact::Present { value: n },
        Ok(_) => Fact::Unavailable {
            reason: Reason::FieldMalformed,
        },
        Err(reason) => Fact::Unavailable { reason },
    };
    Facts {
        input_tokens: count("input_tokens"),
        output_tokens: count("output_tokens"),
        cache_read_tokens: count("cache_read_tokens"),
        cache_write_tokens: count("cache_write_tokens"),
        num_turns: count("num_turns"),
        total_cost_usd: match field("total_cost_usd") {
            Ok(Seen::Usd(x)) => Fact::Present { value: x },
            Ok(_) => Fact::Unavailable {
                reason: Reason::FieldMalformed,
            },
            Err(reason) => Fact::Unavailable { reason },
        },
    }
}

/// Whether a row is scored: a pass or a measured failure.
fn scored(row: &TrialResult) -> bool {
    row.outcome != Outcome::HarnessError
}

/// The committed companions a report reads, by trial id.
pub type Companions = std::collections::BTreeMap<String, Companion>;

fn sum_field(
    rows: &[&TrialResult],
    usage: &Companions,
    get: impl Fn(&Facts) -> Option<u64>,
) -> Result<u64, &'static str> {
    let mut sum = 0u64;
    for row in rows {
        let Some(u) = usage.get(&row.trial_id) else {
            return Err("incomplete_attribution");
        };
        let v = get(&u.totals()).ok_or("missing_required_count")?;
        sum = sum.checked_add(v).ok_or("sum_overflow")?;
    }
    if sum > COUNT_LIMIT {
        return Err("sum_overflow");
    }
    Ok(sum)
}

fn figure(r: Result<u64, &'static str>) -> Value {
    match r {
        Ok(v) => json!({"status": "present", "value": v}),
        Err(reason) => json!({"status": "unavailable", "reason": reason}),
    }
}

/// The group's nested-driver token figures under accepted 0136: the checked
/// sum of `input_tokens + output_tokens` over every scored trial, divided by
/// the group's passes, from the committed companions alone. Cache reads and
/// writes, turns and cost are reported beside it, each on its own
/// availability. `None` when no row of the group ran through the nested
/// driver: journal figures are then the only ones, and a nested gap is never
/// filled from the journal.
pub fn summarize(group: &[&TrialResult], usage: &Companions) -> Option<Value> {
    if !group.iter().any(|r| nested_profile(r).is_some()) {
        return None;
    }
    let scored: Vec<&TrialResult> = group.iter().copied().filter(|r| scored(r)).collect();
    let passes = scored.iter().filter(|r| r.outcome == Outcome::Pass).count() as u64;
    let total = sum_field(&scored, usage, |f| f.input_tokens.value())
        .and_then(|i| sum_field(&scored, usage, |f| f.output_tokens.value()).map(|o| (i, o)))
        .and_then(|(i, o)| {
            i.checked_add(o)
                .filter(|t| *t <= COUNT_LIMIT)
                .ok_or("sum_overflow")
        });
    let per_success = match (total, passes) {
        (Err(reason), _) => json!({"status": "unavailable", "reason": reason}),
        (Ok(_), 0) => json!({"status": "unavailable", "reason": "no_passes"}),
        (Ok(t), p) => json!({"status": "present", "value": t as f64 / p as f64}),
    };
    let mut cost = Ok(0f64);
    for row in &scored {
        cost = match (cost, usage.get(&row.trial_id)) {
            (Ok(sum), Some(u)) => u
                .totals()
                .total_cost_usd
                .value()
                .map(|v| sum + v)
                .ok_or("missing_required_count"),
            (Ok(_), None) => Err("incomplete_attribution"),
            (e, _) => e,
        };
    }
    Some(json!({
        "source": SOURCE,
        "scored_trials": scored.len(),
        "passes": passes,
        "total_definition": "input_tokens + output_tokens, every session of every scored trial",
        "total_tokens": figure(total),
        "tokens_per_success": per_success,
        "cache_read_tokens": figure(sum_field(&scored, usage, |f| f.cache_read_tokens.value())),
        "cache_write_tokens": figure(sum_field(&scored, usage, |f| f.cache_write_tokens.value())),
        "num_turns": figure(sum_field(&scored, usage, |f| f.num_turns.value())),
        "total_cost_usd": match cost {
            Ok(v) if v.is_finite() => json!({"status": "present", "value": v}),
            Ok(_) => json!({"status": "unavailable", "reason": "sum_overflow"}),
            Err(reason) => json!({"status": "unavailable", "reason": reason}),
        },
    }))
}

#[cfg(test)]
mod tests;
