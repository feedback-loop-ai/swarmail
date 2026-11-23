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

pub const FRESHNESS_MINUTES: u64 = 70;

#[derive(Debug, Serialize)]
pub struct Plan {
    /// `baseline-v1-<sha256>` over the fixed-field product record: the id an
    /// operator names in `--authorized-product` (proposed 0137).
    pub product_id: String,
    pub tier: Option<tasks::Tier>,
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
    pub control_slots: Vec<ControlSlot>,
    pub chunks: Vec<Chunk>,
    pub estimate: Estimate,
    pub freshness: Freshness,
    pub steps: Vec<&'static str>,
    pub files: Vec<&'static str>,
    pub report_argv: Vec<String>,
    pub wrapper_contracts: &'static str,
    pub authorization: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ControlSlot {
    pub task: String,
    pub agent: String,
}
/// One task × model scheduling unit with its nominal repetition indices.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chunk {
    pub task: String,
    pub agent: String,
    pub model: String,
    pub family: &'static str,
    pub credential: String,
    pub repetitions: Vec<u32>,
}
#[derive(Debug, Serialize)]
pub struct Estimate {
    pub nominal_minutes: u64,
    pub calculation: String,
    pub additional: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Freshness {
    pub margin_minutes: u64,
    pub rules: Vec<FreshnessRule>,
}
#[derive(Debug, Serialize)]
pub struct FreshnessRule {
    pub credential: String,
    pub agents: Vec<String>,
    pub evidence: &'static str,
}
/// The fields an authorization covers, in a fixed order. Output location,
/// private inputs, seeds, the margin and credential state stay outside it.
#[derive(Serialize)]
struct ProductRecord<'a> {
    schema: &'static str,
    tier: Option<tasks::Tier>,
    catalogue_schema: u32,
    catalogue_sha256: &'a str,
    tasks: &'a [String],
    control_agents: &'a [report::AgentSpec],
    agents: &'a [report::AgentSpec],
    control_repetitions: u32,
    paid_repetitions: u32,
    deadline_ms: u64,
    concurrency: u32,
}
fn evidence(family: &str) -> &'static str {
    match family {
        "claude" => "claudeAiOauth.expiresAt (epoch milliseconds) of the selected login",
        "codex" => "integer exp claim of the selected access token; last_refresh must parse",
        _ => "operator-attested valid_through (epoch seconds) beside the selected route key",
    }
}

impl Plan {
    /// The product the operator approved on 2026-10-26: the sixteen
    /// default-tier tasks, selected by a positive list of tiers so a tier
    /// added later never joins it.
    pub fn new(output: &Path) -> Self {
        Self::for_tier(None, output)
    }

    /// `None` is the approved product (the default-tier tasks); `Some(tier)`
    /// is that tier alone, a separate operator-selected product (proposed
    /// 0117).
    pub fn for_tier(tier: Option<tasks::Tier>, output: &Path) -> Self {
        Self::with_margin(tier, output, FRESHNESS_MINUTES)
    }

    pub fn with_margin(tier: Option<tasks::Tier>, output: &Path, margin_minutes: u64) -> Self {
        Self::with_route(tier, output, margin_minutes, None)
    }

    /// `driver` runs every paid agent through that pinned brokkr driver
    /// (route (b)): each takes its amended `*_via_brokkr` profile and the
    /// `--via-brokkr` flag, so the product id covers the route and the path.
    pub fn with_route(
        tier: Option<tasks::Tier>,
        output: &Path,
        margin_minutes: u64,
        driver: Option<&Path>,
    ) -> Self {
        let tasks: Vec<_> = tasks::catalogue()
            .into_iter()
            .filter(|t| match tier {
                None => tasks::DEFAULT_TIERS.contains(&t.tier),
                Some(tier) => t.tier == tier,
            })
            .map(|t| t.id)
            .collect();
        let mut agents = report::parse_agents(AGENTS).expect("reviewed agent declarations");
        if let Some(driver) = driver {
            for a in &mut agents {
                a.wrapper_profile = a.wrapper_profile.map(WrapperProfile::via);
                a.argv.push("--via-brokkr".into());
                a.argv.push(driver.to_string_lossy().into_owned());
            }
        }
        let control_agents =
            report::parse_agents(r#"[{"label":"oracle","argv":[]},{"label":"null","argv":[]}]"#)
                .expect("builtins");
        let catalogue_sha256 = catalogue_hash();
        let product_id = format!(
            "baseline-v1-{}",
            sha256(
                &serde_json::to_vec(&ProductRecord {
                    schema: "baseline-product-v1",
                    tier,
                    catalogue_schema: tasks::CATALOGUE_SCHEMA,
                    catalogue_sha256: &catalogue_sha256,
                    tasks: &tasks,
                    control_agents: &control_agents,
                    agents: &agents,
                    control_repetitions: 1,
                    paid_repetitions: PAID_TRIALS,
                    deadline_ms: DEADLINE_MS,
                    concurrency: 1,
                })
                .expect("typed product")
            )
        );
        let control_slots: Vec<_> = tasks
            .iter()
            .flat_map(|t| {
                control_agents.iter().map(|a| ControlSlot {
                    task: t.clone(),
                    agent: a.label.clone(),
                })
            })
            .collect();
        let chunks: Vec<_> = tasks
            .iter()
            .flat_map(|t| {
                agents.iter().map(|a| {
                    let family = a.wrapper_profile.expect("declared profile").family();
                    let model = a.model.clone().expect("declared model");
                    Chunk {
                        task: t.clone(),
                        agent: a.label.clone(),
                        credential: crate::wrappers::credential_key(family, &model),
                        model,
                        family,
                        repetitions: (0..PAID_TRIALS).collect(),
                    }
                })
            })
            .collect();
        let mut rules: Vec<FreshnessRule> = Vec::new();
        for a in &agents {
            let family = a.wrapper_profile.expect("declared profile").family();
            let key = crate::wrappers::credential_key(family, a.model.as_deref().unwrap_or(""));
            match rules.iter_mut().find(|r| r.credential == key) {
                Some(rule) => rule.agents.push(a.label.clone()),
                None => rules.push(FreshnessRule {
                    credential: key,
                    agents: vec![a.label.clone()],
                    evidence: evidence(family),
                }),
            }
        }
        let control_rows = tasks.len() * 2;
        let paid_rows = tasks.len() * agents.len() * PAID_TRIALS as usize;
        let slots = (control_rows + paid_rows) as u64;
        Self {
            authorization: format!("--authorized-product {product_id}"),
            product_id,
            tier,
            catalogue_schema: tasks::CATALOGUE_SCHEMA,
            catalogue_sha256,
            control_rows,
            paid_rows,
            tasks,
            agents,
            control_agents,
            control_repetitions: 1,
            paid_repetitions: PAID_TRIALS,
            concurrency: 1,
            deadline_ms: DEADLINE_MS,
            output: output.into(),
            control_slots,
            chunks,
            estimate: Estimate {
                nominal_minutes: slots * DEADLINE_MS / 60_000,
                calculation: format!(
                    "({control_rows} control + {paid_rows} paid slots) x {DEADLINE_MS} ms / 60000, every slot at its full deadline, sequentially"
                ),
                additional: "preparation, owned cleanup and credential-renewal waits add unpredictable time",
            },
            freshness: Freshness {
                margin_minutes,
                rules,
            },
            steps: vec![
                "verify every family's pinned CLI contract and credential shape",
                "fresh free control matrix",
                "validate every control and retained matrix",
                "task-major paid chunks, each started only while its credential has the margin left",
                "validate every retained paid slot",
                "report every scored paid row",
            ],
            files: vec![
                "manifest.json",
                "controls.jsonl",
                "controls/<matrix-id>/matrix-plan.json",
                "controls/<matrix-id>/trials/<trial-id>/",
                "controls/<matrix-id>/matrix.json",
                "attempts/<attempt-id>.json",
                "attempts/<attempt-id>.closed",
                "paid/<attempt-id>/matrix-plan.json",
                "paid/<attempt-id>/trials/<trial-id>/",
                "paid/<attempt-id>/matrix.json",
                "paid.jsonl",
                "report.json",
            ],
            report_argv: vec![
                "swarmail-evals".into(),
                "report".into(),
                "--input".into(),
                output.join("paid.jsonl").to_string_lossy().into_owned(),
            ],
            wrapper_contracts: "a version-2 private selection pins each family's CLI path, version, sha256 and auth shape (see evals/wrappers.md); authenticated six-model and HTTP MCP smoke remains operator-owned",
        }
    }
    /// The families the product spends, in declaration order.
    pub fn families(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for c in &self.chunks {
            if !out.contains(&c.family) {
                out.push(c.family);
            }
        }
        out
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
        let spec = &specs[(i / repetitions as usize) % specs.len()];
        if !trial_ids.insert(&row.trial_id) {
            return Err("baseline_provenance_invalid".into());
        }
        validate_row(row, cell, spec, harness, target, None)?;
    }
    Ok(())
}

/// One retained row against its planned cell, declaration, the run's harness
/// and target, and (for a paid slot) the verified CLI binding it must carry.
pub fn validate_row(
    row: &TrialResult,
    cell: &artifacts::PlannedCell,
    spec: &report::AgentSpec,
    harness: &HarnessIdentity,
    target: &ExecutableIdentity,
    cli: Option<&CliIdentity>,
) -> Result<(), String> {
    artifacts::matches_cell(row, cell).map_err(|_| "baseline_cell_invalid")?;
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
    if row.concurrency != 1
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
        || p.agent.cli.as_ref() != cli
        || p.limits.startup_ms != 30_000
        || p.limits.io_ms != 10_000
        || p.limits.shutdown_ms != 5_000
        || p.limits.matrix_child_envelope_ms != Some(DEADLINE_MS + 300_000)
    {
        return Err("baseline_provenance_invalid".into());
    }
    Ok(())
}

/// A chunk may start only while its credential has at least the margin
/// left: equality qualifies, one second less defers, overflow never does.
pub fn fresh_enough(valid_until: u64, now: u64, margin_minutes: u64) -> bool {
    margin_minutes
        .checked_mul(60)
        .and_then(|margin| now.checked_add(margin))
        .is_some_and(|needed| valid_until >= needed)
}

/// The earliest outstanding chunk whose credential is eligible, in plan
/// order, so deferral never reorders the rest.
pub fn next_chunk(outstanding: &[Chunk], eligible: impl Fn(&str) -> bool) -> Option<&Chunk> {
    outstanding.iter().find(|c| eligible(&c.credential))
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
