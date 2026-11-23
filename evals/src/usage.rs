//! Nested-driver usage: the six token facts of one paid session, read from
//! the session-final checkpoint the pinned brokkr driver authors (route (b) of
//! decision 0119's 2026-11-23 addendum; accepted 0136; proposed 0138).
//!
//! The harness, never the agent, authors this record: the wrapper folds the
//! driver's own stdout, sends the projection to the runner on the launch pipe
//! the CLI never holds, and the runner puts it on the trial row it already
//! owns, so the join is the row's `trial_id`. Each of the six facts stands on
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

/// The record a nested trial's row carries (harness-authored, 0075 §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NestedUsage {
    pub source: String,
    pub trial_id: String,
    /// The driver exchange's attempt; empty when no receipt arrived.
    pub attempt_id: String,
    pub family: String,
    pub facts: Facts,
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

impl NestedUsage {
    /// The record for a receipt, bound to the runner's trial.
    pub fn from_receipt(trial_id: &str, r: &Receipt) -> Self {
        Self {
            source: SOURCE.into(),
            trial_id: trial_id.into(),
            attempt_id: r.attempt_id.clone(),
            family: r.family.clone(),
            facts: r.facts,
        }
    }
    /// The record for a launched nested session whose receipt never came.
    pub fn missing(trial_id: &str, family: &str) -> Self {
        Self {
            source: SOURCE.into(),
            trial_id: trial_id.into(),
            attempt_id: String::new(),
            family: family.into(),
            facts: Facts::unavailable(Reason::ReceiptMissing),
        }
    }
    /// The record's own invariants, checked on every row read.
    pub fn validate(&self, trial_id: &str) -> Result<(), String> {
        if self.source != SOURCE || self.trial_id != trial_id {
            return Err("usage_binding_mismatch".into());
        }
        if final_step(&self.family).is_none() {
            return Err("usage_family_invalid".into());
        }
        let receipt_missing = self.facts == Facts::unavailable(Reason::ReceiptMissing);
        if receipt_missing != self.attempt_id.is_empty()
            || (!self.attempt_id.is_empty() && !plain_id(&self.attempt_id))
        {
            return Err("usage_attempt_invalid".into());
        }
        self.facts.validate()
    }
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

fn sum_field(
    rows: &[&TrialResult],
    get: impl Fn(&Facts) -> Option<u64>,
) -> Result<u64, &'static str> {
    let mut sum = 0u64;
    for row in rows {
        let Some(u) = &row.nested_usage else {
            return Err("incomplete_attribution");
        };
        let v = get(&u.facts).ok_or("missing_required_count")?;
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
/// the group's passes. Cache reads and writes, turns and cost are reported
/// beside it, each on its own availability. `None` when no row of the group
/// ran through the nested driver: journal figures are then the only ones, and
/// a nested gap is never filled from the journal.
pub fn summarize(group: &[&TrialResult]) -> Option<Value> {
    if !group.iter().any(|r| r.nested_usage.is_some()) {
        return None;
    }
    let scored: Vec<&TrialResult> = group.iter().copied().filter(|r| scored(r)).collect();
    let passes = scored.iter().filter(|r| r.outcome == Outcome::Pass).count() as u64;
    let total = sum_field(&scored, |f| f.input_tokens.value())
        .and_then(|i| sum_field(&scored, |f| f.output_tokens.value()).map(|o| (i, o)))
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
        cost = match (cost, row.nested_usage.as_ref()) {
            (Ok(sum), Some(u)) => u
                .facts
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
        "total_definition": "input_tokens + output_tokens, every scored trial",
        "total_tokens": figure(total),
        "tokens_per_success": per_success,
        "cache_read_tokens": figure(sum_field(&scored, |f| f.cache_read_tokens.value())),
        "cache_write_tokens": figure(sum_field(&scored, |f| f.cache_write_tokens.value())),
        "num_turns": figure(sum_field(&scored, |f| f.num_turns.value())),
        "total_cost_usd": match cost {
            Ok(v) if v.is_finite() => json!({"status": "present", "value": v}),
            Ok(_) => json!({"status": "unavailable", "reason": "sum_overflow"}),
            Err(reason) => json!({"status": "unavailable", "reason": reason}),
        },
    }))
}

#[cfg(test)]
mod tests;
