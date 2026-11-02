//! Journal facts v1, their join to result rows, and the retained tool-call
//! record (decision 0116).
//!
//! Rows do not change. brokkr's journal owns tokens, turns, wall time and
//! tool rows per seat; `scripts/journal-facts.sh` turns the pinned brokkr's
//! export into the closed shape read here, and `report --journal` joins it to
//! the rows by the `trial_id` the matrix assigned before it spawned the run
//! and the trial seat echoed in its typed result. Nothing is discovered by
//! timestamp or "newest run".
//!
//! Three rules keep the figures honest:
//! * a missing, ambiguous or accounting-free fact is a typed reason, never `0`
//!   or `false` or `[]`: every one of the five facts per run (tokens, turns,
//!   wall time, deadline kill, tool calls) is `{status, value | reason}`;
//! * `tokens_per_success`, the tool-call figure and the deadline-kill count
//!   are each `null` whenever any scored joined trial in the group lacks the
//!   fact they need, so a partial join can never bias them;
//! * the retained tool-call record carries names, classes, lengths and
//!   digests, never an argument or a result byte (accepted 0075 §4).

use crate::artifacts;
use crate::provenance::{AgentKind, is_hash, sha256};
use crate::report::nearest_rank;
use crate::runner::TrialResult;
use crate::tasks::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub const FACTS_SCHEMA: u64 = 1;
/// The largest facts file the reader will take.
pub const FACTS_LIMIT: usize = 8 * 1024 * 1024;
/// One retained record's cap, the artifact small-file limit.
pub const EVIDENCE_LIMIT: usize = artifacts::SMALL_LIMIT;
const NAME_LIMIT: usize = 128;
const COUNTER_LIMIT: u64 = 9_007_199_254_740_991;

/// Why a fact is not a number. A closed set; `0` is never a stand-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    JournalMissing,
    JournalAmbiguous,
    JournalHasNoAccounting,
    BuiltinAgent,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JournalMissing => "journal_missing",
            Self::JournalAmbiguous => "journal_ambiguous",
            Self::JournalHasNoAccounting => "journal_has_no_accounting",
            Self::BuiltinAgent => "builtin_agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Fact<T> {
    Present { value: T },
    Unavailable { reason: Reason },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cached: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Ok,
    Error,
    InvalidArgument,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub ordinal: u64,
    pub tool: String,
    pub outcome: ToolOutcome,
    pub argument_bytes: u64,
    pub argument_sha256: String,
    pub result_bytes: u64,
    pub result_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub brokkr_version: String,
    pub binary_sha256: String,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialFacts {
    pub trial_id: String,
    pub run_id: String,
    pub seat: String,
    pub tokens: Fact<Tokens>,
    pub turns: Fact<u64>,
    pub wall_time_ms: Fact<u64>,
    pub deadline_killed: Fact<bool>,
    pub tool_calls: Fact<Vec<ToolCall>>,
}

impl TrialFacts {
    /// Whether the journal accounted anything for this run at all: any one of
    /// the five typed facts is present. A present empty tool list is a fact
    /// ("zero calls"); an unavailable one is absence.
    pub fn accounted(&self) -> bool {
        matches!(self.tokens, Fact::Present { .. })
            || matches!(self.turns, Fact::Present { .. })
            || matches!(self.wall_time_ms, Fact::Present { .. })
            || matches!(self.deadline_killed, Fact::Present { .. })
            || matches!(self.tool_calls, Fact::Present { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    pub schema_version: u64,
    pub source: Source,
    pub trials: Vec<TrialFacts>,
}

fn safe_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= NAME_LIMIT
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn bounded_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 1024 && !s.contains('\0')
}

fn counter(n: u64) -> bool {
    n <= COUNTER_LIMIT
}

fn fact_ok(fact: &Fact<u64>) -> bool {
    match fact {
        Fact::Present { value } => counter(*value),
        Fact::Unavailable { .. } => true,
    }
}

impl Facts {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != FACTS_SCHEMA
            || !bounded_text(&self.source.brokkr_version)
            || !is_hash(&self.source.binary_sha256)
            || !bounded_text(&self.source.query)
        {
            return Err("journal_invalid".into());
        }
        for trial in &self.trials {
            let tokens_ok = match &trial.tokens {
                Fact::Present { value } => {
                    counter(value.input) && counter(value.output) && counter(value.cached)
                }
                Fact::Unavailable { .. } => true,
            };
            let calls_ok = match &trial.tool_calls {
                Fact::Present { value } => value.iter().enumerate().all(|(i, call)| {
                    call.ordinal == i as u64 + 1
                        && !call.tool.is_empty()
                        && call.tool.len() <= 1024
                        && is_hash(&call.argument_sha256)
                        && is_hash(&call.result_sha256)
                        && counter(call.argument_bytes)
                        && counter(call.result_bytes)
                }),
                Fact::Unavailable { .. } => true,
            };
            if !safe_id(&trial.trial_id)
                || !safe_id(&trial.run_id)
                || trial.seat != "trial"
                || !tokens_ok
                || !fact_ok(&trial.turns)
                || !fact_ok(&trial.wall_time_ms)
                || !calls_ok
            {
                return Err("journal_invalid".into());
            }
        }
        Ok(())
    }
}

/// Read a facts file strictly: unknown fields, a wrong schema, an invalid id,
/// digest or ordinal refuses the whole input.
pub fn read(bytes: &[u8]) -> Result<Facts, String> {
    if bytes.len() > FACTS_LIMIT {
        return Err("journal_invalid".into());
    }
    let facts: Facts =
        crate::provenance::decode(bytes).map_err(|_| "journal_invalid".to_string())?;
    facts.validate()?;
    Ok(facts)
}

#[derive(Debug, Clone, PartialEq)]
enum JoinedTrial {
    Absent(Reason),
    Entry(Box<TrialFacts>),
}

/// Facts joined to the rows by `trial_id`, plus the evidence index once written.
#[derive(Debug, Clone)]
pub struct Joined {
    source: Source,
    trials: BTreeMap<String, JoinedTrial>,
    evidence: BTreeMap<String, Value>,
}

/// Join facts to rows. A facts entry for a trial id in no row, or two entries
/// for one id, refuses the input so a file from another matrix can never be
/// joined by mistake. A row with no entry is `journal_missing`; a built-in
/// agent launches no driver and is `builtin_agent`.
pub fn join(rows: &[TrialResult], facts: &Facts) -> Result<Joined, String> {
    let mut entries: BTreeMap<&str, &TrialFacts> = BTreeMap::new();
    for entry in &facts.trials {
        if entries.insert(&entry.trial_id, entry).is_some() {
            return Err("journal_duplicate_trial".into());
        }
    }
    let mut trials = BTreeMap::new();
    for row in rows {
        let joined = if row.provenance.agent.kind == AgentKind::Builtin {
            JoinedTrial::Absent(Reason::BuiltinAgent)
        } else if let Some(entry) = entries.get(row.trial_id.as_str()) {
            JoinedTrial::Entry(Box::new((*entry).clone()))
        } else {
            JoinedTrial::Absent(Reason::JournalMissing)
        };
        if trials.insert(row.trial_id.clone(), joined).is_some() {
            return Err("journal_duplicate_trial".into());
        }
    }
    if entries.keys().any(|id| !trials.contains_key(*id)) {
        return Err("journal_unmatched_trial".into());
    }
    Ok(Joined {
        source: facts.source.clone(),
        trials,
        evidence: BTreeMap::new(),
    })
}

impl Joined {
    pub fn source(&self) -> &Source {
        &self.source
    }
}

/// A tool name safe to print and to use as a key: at most 128 bytes of
/// printable ASCII, else the SHA-256 of the name.
pub fn bounded_tool_name(name: &str) -> String {
    if name.len() <= NAME_LIMIT && name.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        name.to_string()
    } else {
        format!("sha256:{}", sha256(name.as_bytes()))
    }
}

#[derive(Serialize)]
struct EvidenceCall<'a> {
    ordinal: u64,
    tool: String,
    outcome: ToolOutcome,
    argument_bytes: u64,
    argument_sha256: &'a str,
    result_bytes: u64,
    result_sha256: &'a str,
}

#[derive(Serialize)]
struct Evidence<'a> {
    schema_version: u64,
    trial_id: &'a str,
    run_id: &'a str,
    truncated: bool,
    retained: u64,
    tool_calls: Vec<EvidenceCall<'a>>,
}

/// The record's bytes: whole calls only, under the cap. Truncation is typed
/// (`"truncated": true` with the `retained` count), never silent. Only
/// names, classes, lengths and digests are ever copied.
fn evidence_bytes(facts: &TrialFacts, all: &[ToolCall]) -> Result<Vec<u8>, String> {
    let mut calls: Vec<EvidenceCall> = all
        .iter()
        .map(|call| EvidenceCall {
            ordinal: call.ordinal,
            tool: bounded_tool_name(&call.tool),
            outcome: call.outcome,
            argument_bytes: call.argument_bytes,
            argument_sha256: &call.argument_sha256,
            result_bytes: call.result_bytes,
            result_sha256: &call.result_sha256,
        })
        .collect();
    fn frame<'a>(
        facts: &'a TrialFacts,
        calls: Vec<EvidenceCall<'a>>,
        truncated: bool,
        retained: u64,
    ) -> Evidence<'a> {
        Evidence {
            schema_version: 1,
            trial_id: &facts.trial_id,
            run_id: &facts.run_id,
            truncated,
            retained,
            tool_calls: calls,
        }
    }
    // Sized with the longer flag and the widest count so the final record is
    // always under the cap.
    let mut used = serde_json::to_vec(&frame(facts, Vec::new(), true, u64::MAX))
        .map_err(|_| "evidence_write_failed")?
        .len();
    let mut keep = 0;
    for call in &calls {
        let size = serde_json::to_vec(call)
            .map_err(|_| "evidence_write_failed")?
            .len()
            + 1;
        if used + size > EVIDENCE_LIMIT {
            break;
        }
        used += size;
        keep += 1;
    }
    let truncated = keep < calls.len();
    calls.truncate(keep);
    artifacts::encode(&frame(facts, calls, truncated, keep as u64), EVIDENCE_LIMIT)
        .map_err(|_| "evidence_write_failed".into())
}

/// Write `DIR/<trial_id>/tool-calls.json` for every joined trial whose
/// `tool_calls` fact is present through the artifact primitives (exclusive
/// 0700 directory, exclusive 0600 file, no clobber) and index each in the
/// joined facts. A trial that cannot be written is recorded as
/// `absent: evidence_write_failed` and the report still completes; a trial
/// whose tool rows the journal did not give is `absent: <its reason>` and no
/// file or directory is created for it. Nothing is skipped silently.
pub fn write_evidence(dir: &Path, joined: &mut Joined) {
    // A root this call creates is private like every leaf below it; a root
    // the operator already made keeps the operator's own mode.
    let created = std::fs::symlink_metadata(dir).is_err();
    let root = artifacts::resolve_root(dir).and_then(|real| {
        artifacts::ensure_root(&real)?;
        if created {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "directory permissions failed".to_string())?;
        }
        Ok(real)
    });
    let mut index = BTreeMap::new();
    for (id, trial) in &joined.trials {
        let entry = match trial {
            JoinedTrial::Absent(reason) => json!({"absent": reason.as_str()}),
            JoinedTrial::Entry(facts) => match &facts.tool_calls {
                Fact::Unavailable { reason } => json!({"absent": reason.as_str()}),
                Fact::Present { value } => {
                    let written = root.as_ref().map_err(|e| e.clone()).and_then(|root| {
                        let bytes = evidence_bytes(facts, value)?;
                        let leaf = artifacts::create_private_dir(root, id)?;
                        artifacts::publish(&leaf, "tool-calls.json", &bytes)
                    });
                    match written {
                        Ok(binding) => json!({
                            "path": format!("{id}/{}", binding.path),
                            "length": binding.length,
                            "sha256": binding.sha256,
                        }),
                        Err(_) => json!({"absent": "evidence_write_failed"}),
                    }
                }
            },
        };
        index.insert(id.clone(), entry);
    }
    joined.evidence = index;
}

fn distribution(values: &mut [u64], with_sum: bool) -> Value {
    values.sort_unstable();
    let mut out = Map::new();
    out.insert("n".into(), json!(values.len()));
    out.insert("median".into(), json!(nearest_rank(values, 0.5)));
    out.insert("p95".into(), json!(nearest_rank(values, 0.95)));
    out.insert("max".into(), json!(values.last()));
    if with_sum {
        out.insert("sum".into(), json!(values.iter().sum::<u64>()));
    }
    Value::Object(out)
}

fn distribution_or_null(mut values: Vec<u64>, with_sum: bool) -> Value {
    if values.is_empty() {
        Value::Null
    } else {
        distribution(&mut values, with_sum)
    }
}

/// The `journal` object of one (task, agent) group. Aggregates run over the
/// group's scored (non-harness-error) trials; harness-error trials are
/// counted apart, not averaged in.
pub fn summarize(group: &[&TrialResult], joined: &Joined) -> Value {
    let scored: Vec<&&TrialResult> = group
        .iter()
        .filter(|r| r.outcome != Outcome::HarnessError)
        .collect();
    let mut missing: BTreeMap<&'static str, u64> = BTreeMap::new();
    let (mut input, mut output, mut cached, mut total) = (vec![], vec![], vec![], vec![]);
    let (mut turns, mut wall) = (vec![], vec![]);
    let mut by_tool: BTreeMap<String, u64> = BTreeMap::new();
    let (mut calls, mut invalid, mut with_calls, mut killed, mut with_kill) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    // Set when a scored trial in the group lacks the fact (no journal entry,
    // or an entry whose fact is unavailable): the aggregate is then withheld.
    let (mut calls_incomplete, mut kill_incomplete) = (false, false);
    for row in &scored {
        match &joined.trials[&row.trial_id] {
            JoinedTrial::Absent(reason) => {
                *missing.entry(reason.as_str()).or_default() += 1;
                // A scored trial with no journal entry lacks every fact: the
                // tool-call and kill aggregates are withheld like the token
                // figure, never a total that silently omits it.
                calls_incomplete = true;
                kill_incomplete = true;
            }
            JoinedTrial::Entry(facts) => {
                match facts.tokens {
                    Fact::Present { value } => {
                        input.push(value.input);
                        output.push(value.output);
                        cached.push(value.cached);
                        total.push(value.input + value.output);
                    }
                    Fact::Unavailable { reason } => {
                        *missing.entry(reason.as_str()).or_default() += 1
                    }
                }
                if let Fact::Present { value } = facts.turns {
                    turns.push(value);
                }
                if let Fact::Present { value } = facts.wall_time_ms {
                    wall.push(value);
                }
                match &facts.tool_calls {
                    Fact::Present { value } => {
                        with_calls += 1;
                        for call in value {
                            calls += 1;
                            *by_tool.entry(bounded_tool_name(&call.tool)).or_default() += 1;
                            invalid += u64::from(call.outcome == ToolOutcome::InvalidArgument);
                        }
                    }
                    Fact::Unavailable { .. } => calls_incomplete = true,
                }
                match facts.deadline_killed {
                    Fact::Present { value } => {
                        with_kill += 1;
                        killed += u64::from(value);
                    }
                    Fact::Unavailable { .. } => kill_incomplete = true,
                }
            }
        }
    }
    let passes = scored.iter().filter(|r| r.outcome == Outcome::Pass).count() as u64;
    let joined_count = total.len();
    let sum_total: u64 = total.iter().sum();
    let (per_success, reason) = if !missing.is_empty() {
        (Value::Null, json!("incomplete_join"))
    } else if passes == 0 {
        (Value::Null, json!("no_passes"))
    } else {
        (json!(sum_total as f64 / passes as f64), Value::Null)
    };
    let tokens = if joined_count == 0 {
        Value::Null
    } else {
        json!({
            "input": distribution(&mut input, true),
            "output": distribution(&mut output, true),
            "cached": distribution(&mut cached, true),
            "total": distribution(&mut total, true),
            "total_definition": "input + output; cached is reported beside and never added, because providers disagree on whether input already includes it",
        })
    };
    let mut runs = Map::new();
    let mut evidence = Map::new();
    for row in group {
        // Only a run the journal accounted names a run id: an ambiguous or
        // accounting-free entry has no run worth citing.
        if let JoinedTrial::Entry(facts) = &joined.trials[&row.trial_id]
            && facts.accounted()
        {
            runs.insert(row.trial_id.clone(), json!(facts.run_id));
        }
        if let Some(entry) = joined.evidence.get(&row.trial_id) {
            evidence.insert(row.trial_id.clone(), entry.clone());
        }
    }
    let (tool_calls, tool_calls_reason) = if calls_incomplete {
        (Value::Null, json!("incomplete_join"))
    } else if with_calls == 0 {
        (Value::Null, Value::Null)
    } else {
        (
            json!({"total": calls, "invalid_argument": invalid, "by_tool": by_tool, "trials": with_calls}),
            Value::Null,
        )
    };
    let (deadline_killed, deadline_killed_reason) = if kill_incomplete {
        (Value::Null, json!("incomplete_join"))
    } else if with_kill == 0 {
        (Value::Null, Value::Null)
    } else {
        (json!(killed), Value::Null)
    };
    json!({
        "joined": joined_count,
        "scored": scored.len(),
        "excluded_harness_errors": group.len() - scored.len(),
        "missing": missing,
        "tokens": tokens,
        "tokens_per_success": per_success,
        "tokens_per_success_reason": reason,
        "turns": distribution_or_null(turns, false),
        "seat_wall_time_ms": distribution_or_null(wall, false),
        "tool_calls": tool_calls,
        "tool_calls_reason": tool_calls_reason,
        "deadline_killed": deadline_killed,
        "deadline_killed_reason": deadline_killed_reason,
        "runs": runs,
        "evidence": evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::WrapperProfile;
    use crate::runner::{self, TrialRequest};
    use crate::state::FailureClass;

    fn id(n: u32) -> String {
        format!("{n:032x}")
    }

    fn row(n: u32, agent: &str, external: bool, outcome: Outcome) -> TrialResult {
        let req = TrialRequest {
            task: Some("signup".into()),
            agent: Some(agent.into()),
            argv: if external {
                vec!["/bin/agent".into()]
            } else {
                Vec::new()
            },
            agent_model: external.then(|| "claude-sonnet-5-5".to_string()),
            wrapper_profile: external.then_some(WrapperProfile::Claude2_1_310ViaBrokkr),
            ..Default::default()
        };
        let mut row = runner::refused_row(&req, &id(n));
        row.outcome = outcome;
        match outcome {
            Outcome::Pass => {
                row.failure_class = None;
                row.agent_exit_status = Some(0);
            }
            Outcome::Fail => {
                row.failure_class = Some(FailureClass::GaveUp);
                row.agent_exit_status = Some(0);
            }
            Outcome::HarnessError => {}
        }
        row.validate().unwrap();
        row
    }

    fn call(i: u64, tool: &str, outcome: ToolOutcome) -> ToolCall {
        ToolCall {
            ordinal: i,
            tool: tool.into(),
            outcome,
            argument_bytes: 3,
            argument_sha256: sha256(b"a"),
            result_bytes: 4,
            result_sha256: sha256(b"r"),
        }
    }

    fn facts_for(n: u32, tokens: Fact<Tokens>, calls: Vec<ToolCall>) -> TrialFacts {
        TrialFacts {
            trial_id: id(n),
            run_id: format!("run-{n}"),
            seat: "trial".into(),
            tokens,
            turns: Fact::Present { value: 4 },
            wall_time_ms: Fact::Present { value: 100 },
            deadline_killed: Fact::Present { value: false },
            tool_calls: Fact::Present { value: calls },
        }
    }

    /// A run the journal holds but with nothing accounted: every fact typed
    /// unavailable for one reason.
    fn no_accounting(n: u32, reason: Reason) -> TrialFacts {
        TrialFacts {
            trial_id: id(n),
            run_id: format!("run-{n}"),
            seat: "trial".into(),
            tokens: unavailable(reason),
            turns: unavailable(reason),
            wall_time_ms: unavailable(reason),
            deadline_killed: unavailable(reason),
            tool_calls: unavailable(reason),
        }
    }

    fn unavailable<T>(reason: Reason) -> Fact<T> {
        Fact::Unavailable { reason }
    }

    fn tokens(input: u64, output: u64) -> Fact<Tokens> {
        Fact::Present {
            value: Tokens {
                input,
                output,
                cached: 7,
            },
        }
    }

    fn file(trials: Vec<TrialFacts>) -> Facts {
        Facts {
            schema_version: 1,
            source: Source {
                brokkr_version: "0.12.0".into(),
                binary_sha256: sha256(b"brokkr"),
                query: "brokkr export (unit test)".into(),
            },
            trials,
        }
    }

    fn summary(rows: &[TrialResult], facts: &Facts) -> Value {
        let joined = join(rows, facts).unwrap();
        let group: Vec<&TrialResult> = rows.iter().collect();
        summarize(&group, &joined)
    }

    #[test]
    fn the_reader_is_strict() {
        let good = serde_json::to_value(file(vec![facts_for(1, tokens(1, 2), vec![])])).unwrap();
        assert!(read(good.to_string().as_bytes()).is_ok());
        let mutate = |f: &dyn Fn(&mut Value)| {
            let mut v = good.clone();
            f(&mut v);
            read(v.to_string().as_bytes())
        };
        assert_eq!(
            mutate(&|v| v["x"] = json!(1)).unwrap_err(),
            "journal_invalid"
        );
        assert!(mutate(&|v| v["schema_version"] = json!(2)).is_err());
        assert!(mutate(&|v| v["source"]["binary_sha256"] = json!("nothex")).is_err());
        assert!(mutate(&|v| v["source"]["query"] = json!("")).is_err());
        assert!(mutate(&|v| v["trials"][0]["trial_id"] = json!("../x")).is_err());
        assert!(mutate(&|v| v["trials"][0]["run_id"] = json!("")).is_err());
        assert!(mutate(&|v| v["trials"][0]["seat"] = json!("score")).is_err());
        assert!(mutate(&|v| v["trials"][0]["extra"] = json!(1)).is_err());
        assert!(
            mutate(&|v| v["trials"][0]["tokens"]["value"]["input"] = json!(COUNTER_LIMIT + 1))
                .is_err()
        );
        assert!(mutate(&|v| v["trials"][0]["turns"]["value"] = json!(COUNTER_LIMIT + 1)).is_err());
        assert!(
            mutate(&|v| v["trials"][0]["wall_time_ms"]["value"] = json!(COUNTER_LIMIT + 1))
                .is_err()
        );
        assert!(
            mutate(
                &|v| v["trials"][0]["tokens"] = json!({"status": "unavailable", "reason": "zero"})
            )
            .is_err(),
            "reasons are a closed set"
        );
        assert!(
            mutate(&|v| v["trials"][0]["tokens"] = json!({"status": "present", "value": -1}))
                .is_err()
        );
        let with_calls = |f: &dyn Fn(&mut Value)| {
            let mut v = serde_json::to_value(file(vec![facts_for(
                1,
                tokens(1, 2),
                vec![call(1, "t", ToolOutcome::Ok), call(2, "t", ToolOutcome::Ok)],
            )]))
            .unwrap();
            f(&mut v);
            read(v.to_string().as_bytes())
        };
        assert!(with_calls(&|_| {}).is_ok());
        assert!(
            with_calls(&|v| v["trials"][0]["tool_calls"]["value"][1]["ordinal"] = json!(5))
                .is_err()
        );
        assert!(
            with_calls(&|v| v["trials"][0]["tool_calls"]["value"][0]["tool"] = json!("")).is_err()
        );
        assert!(
            with_calls(
                &|v| v["trials"][0]["tool_calls"]["value"][0]["argument_sha256"] = json!("abc")
            )
            .is_err()
        );
        assert!(
            with_calls(&|v| v["trials"][0]["tool_calls"]["value"][0]["outcome"] = json!("fine"))
                .is_err()
        );
        // An untyped fact is refused, never read as "zero calls" or "not
        // killed": absence and a real zero must stay distinguishable.
        for (field, untyped) in [
            ("tool_calls", json!([])),
            ("tool_calls", Value::Null),
            ("deadline_killed", json!(false)),
            ("deadline_killed", json!(true)),
            ("deadline_killed", Value::Null),
        ] {
            assert_eq!(
                mutate(&|v| v["trials"][0][field] = untyped.clone()).unwrap_err(),
                "journal_invalid",
                "{field}: {untyped}"
            );
        }
        // Typed present-empty and typed unavailable both read.
        assert!(
            mutate(&|v| v["trials"][0]["tool_calls"] = json!({"status": "present", "value": []}))
                .is_ok()
        );
        assert!(
            mutate(&|v| v["trials"][0]["tool_calls"] =
                json!({"status": "unavailable", "reason": "journal_has_no_accounting"}))
            .is_ok()
        );
        assert!(
            mutate(&|v| v["trials"][0]["deadline_killed"] =
                json!({"status": "present", "value": "false"}))
            .is_err(),
            "deadline_killed is a JSON boolean"
        );
        assert!(read(&vec![b' '; FACTS_LIMIT + 1]).is_err());
        assert!(read(b"not json").is_err());
    }

    #[test]
    fn join_types_every_absence() {
        let rows = vec![
            row(1, "oracle", false, Outcome::Pass),
            row(2, "agent", true, Outcome::Pass),
            row(3, "agent", true, Outcome::Fail),
        ];
        // A built-in row ignores any entry that claims it; a row with no entry
        // is journal_missing; a duplicate or unmatched id refuses.
        let facts = file(vec![
            facts_for(1, tokens(9, 9), vec![]),
            facts_for(2, tokens(1, 1), vec![]),
        ]);
        let joined = join(&rows, &facts).unwrap();
        assert_eq!(
            joined.trials[&id(1)],
            JoinedTrial::Absent(Reason::BuiltinAgent)
        );
        assert!(matches!(joined.trials[&id(2)], JoinedTrial::Entry(_)));
        assert_eq!(
            joined.trials[&id(3)],
            JoinedTrial::Absent(Reason::JournalMissing)
        );
        assert_eq!(joined.source().brokkr_version, "0.12.0");

        let dup = file(vec![
            facts_for(2, tokens(1, 1), vec![]),
            facts_for(2, tokens(2, 2), vec![]),
        ]);
        assert_eq!(join(&rows, &dup).unwrap_err(), "journal_duplicate_trial");
        let stray = file(vec![facts_for(42, tokens(1, 1), vec![])]);
        assert_eq!(join(&rows, &stray).unwrap_err(), "journal_unmatched_trial");
        let twice = vec![rows[1].clone(), rows[1].clone()];
        assert_eq!(
            join(&twice, &file(vec![])).unwrap_err(),
            "journal_duplicate_trial"
        );
    }

    #[test]
    fn tokens_per_success_is_withheld_unless_the_join_is_whole() {
        let rows = vec![
            row(1, "agent", true, Outcome::Pass),
            row(2, "agent", true, Outcome::Pass),
        ];
        // Whole: (3 + 7) / 2 passes.
        let whole = summary(
            &rows,
            &file(vec![
                facts_for(1, tokens(1, 2), vec![]),
                facts_for(2, tokens(3, 4), vec![]),
            ]),
        );
        assert_eq!(whole["tokens_per_success"], json!(5.0));
        assert_eq!(whole["tokens"]["total"]["sum"], 10);
        assert!(
            whole["tokens"]["total_definition"]
                .as_str()
                .unwrap()
                .contains("cached")
        );

        // One entry says the run exists but its journal has no accounting; one
        // is ambiguous. Neither is zero; both withhold the figure.
        let a = no_accounting(1, Reason::JournalHasNoAccounting);
        let b = no_accounting(2, Reason::JournalAmbiguous);
        let partial = summary(&rows, &file(vec![a.clone(), b]));
        assert_eq!(
            partial["runs"],
            json!({}),
            "no run is cited for an unaccounted entry"
        );
        assert_eq!(partial["tokens_per_success"], Value::Null);
        assert_eq!(partial["tokens_per_success_reason"], "incomplete_join");
        assert_eq!(
            partial["missing"],
            json!({"journal_ambiguous": 1, "journal_has_no_accounting": 1})
        );
        assert_eq!(partial["tokens"], Value::Null);
        assert_eq!(partial["joined"], 0);
        assert_eq!(partial["turns"], Value::Null);

        // No passes: nothing to divide by, and the reason says so.
        let failing = vec![row(1, "agent", true, Outcome::Fail)];
        let none = summary(&failing, &file(vec![facts_for(1, tokens(1, 2), vec![])]));
        assert_eq!(none["tokens_per_success"], Value::Null);
        assert_eq!(none["tokens_per_success_reason"], "no_passes");
        assert_eq!(none["tokens"]["total"]["sum"], 3);

        // An entry that accounts nothing at all has no tool-call figure, and
        // says why rather than reporting zero calls and zero kills.
        let nothing = summary(&rows[..1], &file(vec![a]));
        assert_eq!(nothing["tool_calls"], Value::Null);
        assert_eq!(nothing["tool_calls_reason"], "incomplete_join");
        assert_eq!(nothing["deadline_killed"], Value::Null);
        assert_eq!(nothing["deadline_killed_reason"], "incomplete_join");
        assert_eq!(nothing["turns"], Value::Null);
    }

    #[test]
    fn every_fact_is_typed_and_a_partial_one_withholds_only_its_own_aggregate() {
        let rows = [
            row(1, "agent", true, Outcome::Pass),
            row(2, "agent", true, Outcome::Pass),
            row(3, "agent", true, Outcome::Fail),
        ];
        // Trial 1 made no tool calls and was not killed: a real zero. Trial 2
        // has tokens but no tool rows and no kill fact: absence, not zero.
        let zero = facts_for(1, tokens(1, 2), vec![]);
        let mut partial = facts_for(2, tokens(3, 4), vec![]);
        partial.tool_calls = unavailable(Reason::JournalHasNoAccounting);
        partial.deadline_killed = unavailable(Reason::JournalHasNoAccounting);
        let mut killed = facts_for(3, tokens(5, 6), vec![call(1, "t", ToolOutcome::Error)]);
        killed.deadline_killed = Fact::Present { value: true };

        // Present zeros are numbers.
        let whole = summary(
            &[rows[0].clone(), rows[2].clone()],
            &file(vec![zero.clone(), killed]),
        );
        assert_eq!(
            whole["tool_calls"],
            json!({"total": 1, "invalid_argument": 0, "by_tool": {"t": 1}, "trials": 2})
        );
        assert_eq!(whole["tool_calls_reason"], Value::Null);
        assert_eq!(whole["deadline_killed"], 1);
        assert_eq!(whole["deadline_killed_reason"], Value::Null);
        let only_zero = summary(&rows[..1], &file(vec![zero.clone()]));
        assert_eq!(
            only_zero["tool_calls"],
            json!({"total": 0, "invalid_argument": 0, "by_tool": {}, "trials": 1}),
            "a present empty list is zero calls, not absence"
        );
        assert_eq!(only_zero["deadline_killed"], 0);

        // One unavailable fact withholds the tool-call and kill aggregates but
        // leaves every token figure a number.
        let mixed = summary(&rows[..2], &file(vec![zero, partial]));
        assert_eq!(mixed["tool_calls"], Value::Null);
        assert_eq!(mixed["tool_calls_reason"], "incomplete_join");
        assert_eq!(mixed["deadline_killed"], Value::Null);
        assert_eq!(mixed["deadline_killed_reason"], "incomplete_join");
        assert_eq!(mixed["tokens_per_success"], json!(5.0));
        assert_eq!(mixed["tokens"]["total"]["sum"], 10);
        assert_eq!(mixed["runs"].as_object().unwrap().len(), 2);
    }

    #[test]
    fn harness_errors_are_counted_apart_never_averaged_in() {
        let rows = vec![
            row(1, "agent", true, Outcome::Pass),
            row(2, "agent", true, Outcome::HarnessError),
        ];
        let out = summary(&rows, &file(vec![facts_for(1, tokens(5, 5), vec![])]));
        assert_eq!(out["scored"], 1);
        assert_eq!(out["excluded_harness_errors"], 1);
        assert_eq!(out["missing"], json!({}), "the harness error needs no fact");
        assert_eq!(out["tokens_per_success"], json!(10.0));
    }

    #[test]
    fn tool_counts_and_names_are_bounded() {
        let long = "x".repeat(300);
        assert_eq!(
            bounded_tool_name("mcp__swarmail__wait"),
            "mcp__swarmail__wait"
        );
        for odd in [long.as_str(), "naïve", "tab\there"] {
            let name = bounded_tool_name(odd);
            assert_eq!(name, format!("sha256:{}", sha256(odd.as_bytes())));
            assert!(name.len() <= NAME_LIMIT);
        }
        let rows = vec![row(1, "agent", true, Outcome::Pass)];
        let calls = vec![
            call(1, &long, ToolOutcome::InvalidArgument),
            call(2, &long, ToolOutcome::Error),
        ];
        let out = summary(&rows, &file(vec![facts_for(1, tokens(1, 1), calls)]));
        let by_tool = out["tool_calls"]["by_tool"].as_object().unwrap();
        assert_eq!(by_tool.len(), 1);
        assert!(by_tool.keys().all(|k| k.len() <= NAME_LIMIT));
        assert_eq!(out["tool_calls"]["invalid_argument"], 1);
    }

    #[test]
    fn evidence_is_cut_at_whole_calls_and_says_so() {
        let calls: Vec<ToolCall> = (1..=3000)
            .map(|i| call(i, "mcp__swarmail__wait", ToolOutcome::Ok))
            .collect();
        let facts = facts_for(1, tokens(1, 1), calls);
        let Fact::Present { value: all } = &facts.tool_calls else {
            unreachable!()
        };
        let bytes = evidence_bytes(&facts, all).unwrap();
        assert!(bytes.len() <= EVIDENCE_LIMIT);
        let record: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(record["truncated"], true);
        let kept = record["tool_calls"].as_array().unwrap().len();
        assert!(kept > 0 && kept < 3000);
        assert_eq!(record["retained"], kept, "the count says how much was kept");
        let small_calls = vec![call(1, "t", ToolOutcome::Ok)];
        let small = facts_for(1, tokens(1, 1), small_calls.clone());
        let record: Value =
            serde_json::from_slice(&evidence_bytes(&small, &small_calls).unwrap()).unwrap();
        assert_eq!(record["truncated"], false);
        assert_eq!(record["retained"], 1);
        assert_eq!(record["tool_calls"].as_array().unwrap().len(), 1);
        // A present empty list is a real fact: a record that says zero calls.
        let none = facts_for(1, tokens(1, 1), vec![]);
        let record: Value = serde_json::from_slice(&evidence_bytes(&none, &[]).unwrap()).unwrap();
        assert_eq!(
            (record["truncated"].clone(), record["retained"].clone()),
            (json!(false), json!(0))
        );
        assert_eq!(record["tool_calls"], json!([]));
    }

    #[test]
    fn an_unusable_evidence_root_is_a_typed_absence_for_every_trial() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        std::fs::write(&blocker, b"x").unwrap();
        let rows = vec![
            row(1, "agent", true, Outcome::Pass),
            row(2, "agent", true, Outcome::Pass),
        ];
        let empty = no_accounting(2, Reason::JournalHasNoAccounting);
        let facts = file(vec![
            facts_for(1, tokens(1, 1), vec![call(1, "t", ToolOutcome::Ok)]),
            empty,
        ]);
        let mut joined = join(&rows, &facts).unwrap();
        write_evidence(&blocker.join("below"), &mut joined);
        assert_eq!(
            joined.evidence[&id(1)],
            json!({"absent": "evidence_write_failed"})
        );
        assert_eq!(
            joined.evidence[&id(2)],
            json!({"absent": "journal_has_no_accounting"})
        );
    }
}
