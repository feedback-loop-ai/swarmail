//! Matrix inputs and the report (decision 0055 §§9–10).
//!
//! Success is passes over passes plus fails, with a 95% Wilson interval;
//! harness errors are counted and shown, never folded into either side.
//! Median and p95 are nearest-rank over non-harness-error wall times.
//! Empty denominators give `null`, never NaN or an invented 100%.

use crate::runner::{BUILTINS, TrialResult};
use crate::state::FailureClass;
use crate::tasks::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashSet};

/// The two-sided 95% normal quantile.
pub const Z95: f64 = 1.959963984540054;

/// One matrix agent: a label and its literal argv (empty for built-ins).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSpec {
    pub label: String,
    pub argv: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub config_id: Option<String>,
    #[serde(default)]
    pub public_argv: bool,
    #[serde(default)]
    pub wrapper_profile: Option<crate::provenance::WrapperProfile>,
}

/// Parse and validate a matrix agent file.
pub fn parse_agents(text: &str) -> Result<Vec<AgentSpec>, String> {
    let agents: Vec<AgentSpec> =
        serde_json::from_str(text).map_err(|e| format!("agent file: {e}"))?;
    if agents.is_empty() {
        return Err("agent file names no agents".to_string());
    }
    let mut seen = HashSet::new();
    for a in &agents {
        if a.label.is_empty() {
            return Err("an agent label is empty".to_string());
        }
        if !seen.insert(a.label.as_str()) {
            return Err(format!("duplicate agent label {}", a.label));
        }
        let builtin = BUILTINS.contains(&a.label.as_str());
        if a.label.len() > 1024
            || a.model
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 1024)
            || a.config_id
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("public agent declaration exceeds bounds".into());
        }
        if builtin
            && (a.model.is_some()
                || a.config_id.is_some()
                || a.public_argv
                || a.wrapper_profile.is_some())
        {
            return Err("builtin declarations cannot be overridden".into());
        }
        if let Some(profile) = a.wrapper_profile {
            profile.validate_model(a.model.as_deref())?;
        }
        if builtin && !a.argv.is_empty() {
            return Err(format!("built-in agent {} takes an empty argv", a.label));
        }
        if !builtin && a.argv.is_empty() {
            return Err(format!("external agent {} needs a nonempty argv", a.label));
        }
    }
    Ok(agents)
}

/// The 95% Wilson score interval for `passes` of `n`; `None` when n = 0.
pub fn wilson(passes: u64, n: u64) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let (n, p, z2) = (n as f64, passes as f64 / n as f64, Z95 * Z95);
    let denom = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let half = Z95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    Some((
        ((centre - half) / denom).max(0.0),
        ((centre + half) / denom).min(1.0),
    ))
}

/// Nearest-rank quantile over sorted values: index `ceil(p*n) - 1`.
pub fn nearest_rank(sorted: &[u64], p: f64) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (p * sorted.len() as f64).ceil() as usize;
    Some(sorted[rank.max(1) - 1])
}

/// Parse JSONL result rows strictly; a malformed row refuses the input.
pub fn parse_rows(text: &str) -> Result<Vec<TrialResult>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            let row =
                crate::results::read(l.as_bytes()).map_err(|e| format!("line {}: {e}", i + 1))?;
            row.validate().map_err(|e| format!("line {}: {e}", i + 1))?;
            Ok(row)
        })
        .collect()
}

fn ratio(a: u64, b: u64) -> Value {
    if b == 0 {
        Value::Null
    } else {
        json!(a as f64 / b as f64)
    }
}

/// Aggregate rows per (task, agent).
pub fn report(rows: &[TrialResult]) -> Value {
    let mut groups: BTreeMap<(Option<String>, Option<String>), Vec<&TrialResult>> = BTreeMap::new();
    for row in rows {
        groups
            .entry((row.task.clone(), row.agent.clone()))
            .or_default()
            .push(row);
    }
    let out: Vec<Value> = groups
        .into_iter()
        .map(|((task, agent), rows)| {
            let count = |o: Outcome| rows.iter().filter(|r| r.outcome == o).count() as u64;
            let (passes, fails, errors) = (
                count(Outcome::Pass),
                count(Outcome::Fail),
                count(Outcome::HarnessError),
            );
            let attempts = rows.len() as u64;
            let mut times: Vec<u64> = rows
                .iter()
                .filter(|r| r.outcome != Outcome::HarnessError)
                .map(|r| r.wall_time_ms)
                .collect();
            times.sort_unstable();
            let mut classes = Map::new();
            for class in FailureClass::ALL {
                let n = rows
                    .iter()
                    .filter(|r| r.failure_class == Some(class))
                    .count();
                classes.insert(class.as_str().to_string(), json!(n));
            }
            let mut surfaces = Vec::new();
            for row in &rows {
                let surface = &row.provenance.agent.tool_surface;
                if !surfaces.contains(surface) {
                    surfaces.push(surface.clone());
                }
            }
            json!({
                "tool_surfaces": surfaces,
                "task": task,
                "agent": agent,
                "attempts": attempts,
                "passes": passes,
                "fails": fails,
                "harness_errors": errors,
                "harness_error_rate": ratio(errors, attempts),
                "scored": passes + fails,
                "success_rate": ratio(passes, passes + fails),
                "success_ci95": wilson(passes, passes + fails).map(|(lo, hi)| json!([lo, hi])),
                "passes_over_attempts": ratio(passes, attempts),
                "wall_time_ms": {
                    "n": times.len(),
                    "median": nearest_rank(&times, 0.5),
                    "p95": nearest_rank(&times, 0.95),
                },
                "failure_classes": classes,
            })
        })
        .collect();
    json!({ "groups": out, "tool_surface_note": "Per-family tool surfaces are asymmetric declared configurations: Claude disables builtins; Codex retains native reading/edit tools, code-mode and ChatGPT connectors; dsh retains native tools whose removal is unmeasured. All trials are unconfined." })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_matches_known_values() {
        assert_eq!(wilson(0, 0), None);
        let (lo, hi) = wilson(10, 10).unwrap();
        // At p = 1 the lower bound is n / (n + z²) and the upper bound 1.
        assert!((lo - 10.0 / (10.0 + Z95 * Z95)).abs() < 1e-12, "{lo}");
        assert!((hi - 1.0).abs() < 1e-12, "{hi}");
        let (lo, hi) = wilson(0, 10).unwrap();
        // The mirror image: 0 of 10 has upper bound z² / (n + z²).
        assert!(
            lo.abs() < 1e-12 && (hi - 0.2775328).abs() < 1e-6,
            "{lo} {hi}"
        );
        let (lo, hi) = wilson(5, 10).unwrap();
        assert!((lo - 0.2365931).abs() < 1e-6 && (hi - 0.7634069).abs() < 1e-6);
    }

    #[test]
    fn nearest_rank_uses_ceil_minus_one() {
        let v: Vec<u64> = (1..=20).collect();
        assert_eq!(nearest_rank(&v, 0.5), Some(10));
        assert_eq!(nearest_rank(&v, 0.95), Some(19));
        assert_eq!(nearest_rank(&[7], 0.95), Some(7));
        assert_eq!(nearest_rank(&[], 0.5), None);
        assert_eq!(nearest_rank(&[1, 2, 3], 0.5), Some(2));
    }

    #[test]
    fn agent_files_refuse_bad_shapes() {
        assert!(
            parse_agents(r#"[{"label":"oracle","argv":[]},{"label":"x","argv":["/bin/x"]}]"#)
                .is_ok()
        );
        for bad in [
            r#"[]"#,
            r#"[{"label":"oracle","argv":["x"]}]"#,
            r#"[{"label":"x","argv":[]}]"#,
            r#"[{"label":"x","argv":["a"]},{"label":"x","argv":["b"]}]"#,
            r#"[{"label":"x","argv":["a"],"shell":true}]"#,
            r#"[{"label":"","argv":["a"]}]"#,
        ] {
            assert!(parse_agents(bad).is_err(), "{bad}");
        }
    }
}
