//! The operator-approved product and strict admission, independent of CLI support.
use crate::{
    artifacts,
    provenance::*,
    report,
    runner::TrialResult,
    tasks::{self, Outcome},
};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const DEADLINE_MS: u64 = 600_000;
pub const PAID_TRIALS: u32 = 5;
pub const AGENTS: &str = include_str!("../agents-baseline.json");

#[derive(Debug, Serialize)]
pub struct Plan {
    pub catalogue_schema: u32,
    pub catalogue_sha256: String,
    pub tasks: Vec<String>,
    pub agents: Vec<report::AgentSpec>,
    pub control_agents: Vec<report::AgentSpec>,
    pub control_repetitions: u32,
    pub paid_repetitions: u32,
    pub control_rows: usize,
    pub paid_rows: usize,
    pub concurrency: u32,
    pub deadline_ms: u64,
    pub output: PathBuf,
    pub steps: Vec<&'static str>,
    pub files: Vec<&'static str>,
    pub report_argv: Vec<String>,
    pub wrapper_contracts: &'static str,
}
impl Plan {
    pub fn new(output: &Path) -> Self {
        let tasks: Vec<_> = tasks::catalogue().into_iter().map(|t| t.id).collect();
        let agents = report::parse_agents(AGENTS).expect("reviewed agent declarations");
        Self {
            catalogue_schema: tasks::CATALOGUE_SCHEMA,
            catalogue_sha256: catalogue_hash(),
            control_rows: tasks.len() * 2,
            paid_rows: tasks.len() * agents.len() * PAID_TRIALS as usize,
            tasks,
            agents,
            control_agents: report::parse_agents(
                r#"[{"label":"oracle","argv":[]},{"label":"null","argv":[]}]"#,
            )
            .expect("builtins"),
            control_repetitions: 1,
            paid_repetitions: PAID_TRIALS,
            concurrency: 1,
            deadline_ms: DEADLINE_MS,
            output: output.into(),
            steps: vec![
                "fresh free control matrix",
                "validate every control and retained matrix",
                "launch declared wrapper recipes",
                "paid matrix",
                "validate paid accounting",
                "report every parseable paid row",
            ],
            files: vec![
                "controls.jsonl",
                "controls/<matrix-id>/matrix-plan.json",
                "controls/<matrix-id>/trials/<trial-id>/",
                "controls/<matrix-id>/matrix.json",
                "paid.jsonl",
                "paid/<matrix-id>/matrix-plan.json",
                "paid/<matrix-id>/trials/<trial-id>/",
                "paid/<matrix-id>/matrix.json",
                "report.json",
            ],
            report_argv: vec![
                "swarmail-evals".into(),
                "report".into(),
                "--input".into(),
                output.join("paid.jsonl").to_string_lossy().into_owned(),
            ],
            wrapper_contracts: "offline launch assembly supported; authenticated six-model and HTTP MCP smoke remains operator-owned (see evals/wrappers.md)",
        }
    }
    /// Resolve only public executable/argv and append an opaque private locator.
    /// The coordinator never reads credentials or opts this argv into hashing.
    pub fn effective_agents(
        &self,
        executable: &Path,
        config: &Path,
    ) -> Result<Vec<report::AgentSpec>, String> {
        let executable = executable.to_str().ok_or("executable_path_invalid")?;
        let config = config.to_str().ok_or("config_path_invalid")?;
        let mut agents = self.agents.clone();
        for a in &mut agents {
            a.argv[0] = executable.into();
            a.argv.extend(["--config".into(), config.into()]);
        }
        Ok(agents)
    }
}

/// JSONL here is a complete stream, so even a syntactically valid unterminated
/// final row or an empty line refuses admission. The original bytes stay intact.
pub fn rows(bytes: &[u8]) -> Result<Vec<TrialResult>, String> {
    if bytes.is_empty() || !bytes.ends_with(b"\n") || bytes.len() > 4096 * RESULT_LIMIT {
        return Err("baseline_rows_incomplete".into());
    }
    bytes[..bytes.len() - 1]
        .split(|b| *b == b'\n')
        .map(|line| {
            if line.is_empty() {
                return Err("baseline_row_empty".into());
            }
            crate::results::read(line).map_err(|_| "baseline_row_invalid".into())
        })
        .collect()
}

/// Bind ordered unique rows to this invocation's prelaunch matrix plan and
/// declarations, never infer success from matrix's exit code alone.
#[allow(clippy::too_many_arguments)]
pub fn validate_accounting(
    rows: &[TrialResult],
    plan: &artifacts::MatrixPlan,
    specs: &[report::AgentSpec],
    tasks: &[String],
    repetitions: u32,
    harness: &HarnessIdentity,
    target: &ExecutableIdentity,
) -> Result<(), String> {
    artifacts::validate_plan(plan).map_err(|_| "baseline_plan_invalid")?;
    if plan.tasks != tasks
        || plan.agents != specs.iter().map(|a| a.label.clone()).collect::<Vec<_>>()
        || plan.repetitions != repetitions
        || rows.len() != plan.cells.len()
    {
        return Err("baseline_accounting_invalid".into());
    }
    let mut trial_ids = std::collections::HashSet::new();
    for (i, (row, cell)) in rows.iter().zip(&plan.cells).enumerate() {
        artifacts::matches_cell(row, cell).map_err(|_| "baseline_cell_invalid")?;
        let spec = &specs[(i / repetitions as usize) % specs.len()];
        let p = &row.provenance;
        let builtin = crate::runner::BUILTINS.contains(&spec.label.as_str());
        let expected_model = if builtin {
            Availability::NotApplicable {
                reason: Reason::Builtin,
            }
        } else {
            Availability::present(
                spec.model.clone().ok_or("baseline_model_missing")?,
                Source::OperatorDeclared,
            )
        };
        let expected_config = if builtin {
            Availability::present(spec.label.clone(), Source::Builtin)
        } else {
            spec.config_id
                .clone()
                .map(|v| Availability::present(v, Source::OperatorDeclared))
                .unwrap_or_else(|| Availability::unavailable(Reason::NotDeclared))
        };
        let expected_command = if builtin {
            Availability::present(
                CommandIdentity {
                    algorithm: "public-argv-sha256-v1".into(),
                    sha256: public_argv(std::slice::from_ref(&spec.label)),
                },
                Source::Builtin,
            )
        } else {
            Availability::unavailable(Reason::PrivateArgv)
        };
        if !trial_ids.insert(&row.trial_id)
            || row.concurrency != 1
            || cell.deadline_ms != DEADLINE_MS
            || p.task.catalogue_sha256 != catalogue_hash()
            || p.task.definition_sha256
                != Availability::present(
                    task_hash(&cell.task).ok_or("baseline_task_invalid")?,
                    Source::CatalogueDefinition,
                )
            || &p.harness != harness
            || p.target.executable_sha256 != target.executable_sha256
            || p.target.build_commit != target.build_commit
            || p.target.source_dirty != target.source_dirty
            || p.agent.kind
                != if builtin {
                    AgentKind::Builtin
                } else {
                    AgentKind::External
                }
            || p.agent.tool_surface != ToolSurface::for_request(true, builtin, spec.wrapper_profile)
            || p.agent.model != expected_model
            || p.agent.config_id != expected_config
            || p.agent.command != expected_command
            || p.agent.executable_sha256 != harness.executable_sha256
            || p.limits.startup_ms != 30_000
            || p.limits.io_ms != 10_000
            || p.limits.shutdown_ms != 5_000
            || p.limits.matrix_child_envelope_ms != Some(DEADLINE_MS + 300_000)
        {
            return Err("baseline_provenance_invalid".into());
        }
    }
    Ok(())
}

pub fn validate_controls(
    rows: &[TrialResult],
    matrix: &artifacts::MatrixPlan,
    plan: &Plan,
    status: u8,
    harness: &HarnessIdentity,
    target: &ExecutableIdentity,
) -> Result<(), String> {
    validate_accounting(
        rows,
        matrix,
        &plan.control_agents,
        &plan.tasks,
        1,
        harness,
        target,
    )?;
    if status != 1 {
        return Err("baseline_control_exit_invalid".into());
    }
    for row in rows {
        let task = tasks::find(row.task.as_deref().ok_or("baseline_task_missing")?)
            .ok_or("baseline_task_invalid")?;
        let valid = match row.agent.as_deref() {
            Some("oracle") => row.outcome == Outcome::Pass && row.failure_class.is_none(),
            Some("null") => {
                row.outcome == Outcome::Fail && row.failure_class == Some(task.idle_class)
            }
            _ => false,
        };
        if !valid {
            return Err("baseline_controls_failed".into());
        }
    }
    Ok(())
}

/// Operational errors dominate valid measured business failures and report success.
pub fn status(matrix_status: u8, accounting_ok: bool, report_ok: bool, rows: &[TrialResult]) -> u8 {
    if matrix_status > 1
        || !accounting_ok
        || !report_ok
        || rows.iter().any(|r| r.outcome == Outcome::HarnessError)
    {
        2
    } else if matrix_status == 1 || rows.iter().any(|r| r.outcome == Outcome::Fail) {
        1
    } else {
        0
    }
}
