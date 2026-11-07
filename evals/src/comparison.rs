//! The agent-suite comparison's records and verifier (proposed decision 0129
//! §8–§9, spec R9–R10): the frozen product record, the inputs `matrix
//! --target` binds before it spawns anything, the rows it writes, the
//! controls' index, and `comparison-check`, which names the first
//! prerequisite that does not hold.
//!
//! Nothing here starts a trial or reads a wrapper configuration. The checks
//! read bytes and compare them: the product against the lock, the controls
//! against the product, the receipts against the repository.

use crate::identity::{COMPARISON_SCHEMA, COMPARISON_TARGETS, ComparisonRow, SMTP_AUTH_VALUES};
use crate::provenance::sha256;
use crate::target::{self, Target};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// The most any input record may be: a product record or a lock is a few
/// kilobytes, and a larger file is not one.
pub const READ_BOUND: u64 = 1 << 20;

/// The version of the product record this build reads.
pub const PRODUCT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ruling {
    pub date: String,
    pub quote: String,
    pub issue: String,
    pub records: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    /// Whether an SMTP `250` means the mail is queryable. Swarmail promises
    /// it (decision 0001); a competitor's is `not-promised`, declared and
    /// never measured per trial.
    pub accept_implies_queryable: String,
    pub native_wait: String,
}

/// One target's frozen tuple: everything about how the target is reached
/// that is not the fixture or the scorer. Each fact is copied from a result,
/// never written ahead of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetTuple {
    pub target: String,
    pub comparison_name: String,
    pub connection: String,
    pub lock_entry: Option<String>,
    pub adapter_mode: Option<String>,
    pub artifact_kind: Option<String>,
    pub version_report: Option<String>,
    pub executable_sha256: Option<String>,
    pub tools_sha256: String,
    pub smtp_auth: String,
    pub isolation: String,
    pub wait_method: String,
    pub delivery: Delivery,
    pub recipient_filter: String,
    pub prompt_header_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTuple {
    pub label: String,
    pub wrapper_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlsPlan {
    pub agents: Vec<String>,
    pub trials: u32,
    pub cells: u32,
}

/// The frozen product record (0091 §1, 0129 §8): what a comparison is, fixed
/// before any control or paid session runs. A changed method is a new id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub id: String,
    pub version: u32,
    pub ruling: Ruling,
    pub tier: String,
    pub tasks: Vec<String>,
    /// For each task, whether each target is requested for it.
    pub applicability: BTreeMap<String, BTreeMap<String, String>>,
    pub targets: Vec<TargetTuple>,
    pub agents: Vec<AgentTuple>,
    pub trials: u32,
    pub order: String,
    pub seed_policy: String,
    pub planned_cells: u32,
    pub controls: ControlsPlan,
    pub statistics: String,
    pub cost_ref: String,
    pub batch_semantics: String,
}

/// The header digest a target's prompt freezes: everything above the goal
/// line (for Swarmail, `PROMPT_TEMPLATE`'s; for a competitor, its own).
pub fn header_sha256(target: Target) -> String {
    let header = if target.is_competitor() {
        crate::tasks::competitor_header(target.display())
    } else {
        let t = crate::tasks::PROMPT_TEMPLATE;
        t[..t.find("Task: {goal}").expect("the template names the goal")].to_string()
    };
    sha256(header.as_bytes())
}

pub fn is_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Product {
    pub fn parse(bytes: &[u8]) -> Result<Product, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("product_invalid: {e}"))
    }

    pub fn tuple(&self, target: Target) -> Option<&TargetTuple> {
        self.targets.iter().find(|t| t.target == target.name())
    }

    /// Whether `task` is requested for `target`.
    pub fn applicable(&self, task: &str, target: Target) -> bool {
        self.applicability
            .get(task)
            .and_then(|r| r.get(target.name()))
            .is_some_and(|v| v == "requested")
    }

    /// Structural validity: the closed target set in order, the tuple facts
    /// well formed and equal to what the code freezes, and the denominator
    /// equal to tasks × targets × agents × trials. `lock` is the source lock
    /// as JSON; when given, each competitor tuple must agree with its entry.
    pub fn validate(&self, lock: Option<&Value>) -> Result<(), String> {
        let mut out: Vec<String> = Vec::new();
        if self.version != PRODUCT_VERSION {
            out.push(format!("version {} is not {PRODUCT_VERSION}", self.version));
        }
        if self.id.is_empty() || self.id.len() > 64 {
            out.push("id is empty or too long".into());
        }
        if self.ruling.quote != "compare with all that we have shims for" {
            out.push("ruling quote is not the operator's".into());
        }
        let hard: Vec<String> = crate::tasks::catalogue()
            .into_iter()
            .filter(|t| t.tier == crate::tasks::Tier::Hard)
            .map(|t| t.id)
            .collect();
        if self.tier != "hard" || self.tasks != hard {
            out.push("tasks are not the hard tier in catalogue order".into());
        }
        let names: Vec<&str> = self.targets.iter().map(|t| t.target.as_str()).collect();
        let want: Vec<&str> = target::ALL.iter().map(|t| t.name()).collect();
        if names != want {
            out.push(format!("targets {names:?} are not {want:?}"));
        }
        for tuple in &self.targets {
            if let Err(why) = self.validate_tuple(tuple, lock) {
                out.extend(why);
            }
        }
        let (n_tasks, n_targets, n_agents) = (
            self.tasks.len() as u32,
            self.targets.len() as u32,
            self.agents.len() as u32,
        );
        let planned = n_tasks * n_targets * n_agents * self.trials;
        if self.trials == 0 || self.planned_cells != planned {
            out.push(format!(
                "planned_cells {} is not tasks × targets × agents × trials = {planned}",
                self.planned_cells
            ));
        }
        if self.controls.agents != ["oracle", "null"]
            || self.controls.trials == 0
            || self.controls.cells != n_tasks * n_targets * 2 * self.controls.trials
        {
            out.push(
                "controls are not the oracle and null agents over every task and target".into(),
            );
        }
        for task in &self.tasks {
            let all = self.applicability.get(task).is_some_and(|r| {
                r.len() == target::ALL.len()
                    && r.values().all(|v| v == "requested" || v == "not-requested")
            });
            if !all {
                out.push(format!("{task}: applicability does not name every target"));
            }
        }
        if self
            .agents
            .iter()
            .any(|a| a.label.is_empty() || a.wrapper_profile.is_empty())
        {
            out.push("an agent has no label or wrapper profile".into());
        }
        if self.cost_ref != "agent-comparison-cost.json" {
            out.push("cost_ref is not the cost record".into());
        }
        if out.is_empty() {
            Ok(())
        } else {
            Err(format!("product_invalid: {}", out.join("; ")))
        }
    }

    fn validate_tuple(&self, tuple: &TargetTuple, lock: Option<&Value>) -> Result<(), Vec<String>> {
        let mut out = Vec::new();
        let Ok(t) = Target::parse(&tuple.target) else {
            return Ok(());
        };
        let n = t.name();
        let connection = COMPARISON_TARGETS
            .iter()
            .find(|(c, _)| *c == t.comparison_name())
            .map(|(_, c)| *c);
        if tuple.comparison_name != t.comparison_name()
            || Some(tuple.connection.as_str()) != connection
        {
            out.push(format!("{n}: name or connection is not the target's"));
        }
        if tuple.adapter_mode.as_deref() != t.adapter_mode() {
            out.push(format!("{n}: adapter mode is not the target's"));
        }
        if !is_hash(&tuple.tools_sha256) {
            out.push(format!("{n}: tools_sha256 is not a SHA-256"));
        }
        if !SMTP_AUTH_VALUES.contains(&tuple.smtp_auth.as_str()) {
            out.push(format!("{n}: smtp_auth is not plain or none"));
        }
        if tuple.prompt_header_sha256 != header_sha256(t) {
            out.push(format!(
                "{n}: prompt header digest is not the frozen text's {}",
                header_sha256(t)
            ));
        }
        if !["promised", "not-promised"].contains(&tuple.delivery.accept_implies_queryable.as_str())
        {
            out.push(format!("{n}: delivery semantics are not declared"));
        }
        if t.is_competitor() {
            let bound = tuple.lock_entry.as_deref() == Some(n)
                && tuple.executable_sha256.as_deref().is_some_and(is_hash)
                && tuple.artifact_kind.is_some()
                && tuple
                    .version_report
                    .as_deref()
                    .is_some_and(|v| !v.is_empty());
            if !bound {
                out.push(format!(
                    "{n}: a competitor tuple binds its lock entry and executable"
                ));
            }
            if let Some(lock) = lock
                && let Err(why) = tuple_agrees_with_lock(tuple, lock)
            {
                out.push(why);
            }
        } else if tuple.lock_entry.is_some() || tuple.executable_sha256.is_some() {
            out.push("swarmail: the candidate is measured per run, never pinned".into());
        }
        if out.is_empty() { Ok(()) } else { Err(out) }
    }
}

/// A competitor tuple's facts against its entry in the source lock: the
/// executable digest, the artifact kind and the version report. A product
/// that disagrees with the lock names a binary the lock does not pin.
pub fn tuple_agrees_with_lock(tuple: &TargetTuple, lock: &Value) -> Result<(), String> {
    let entry = lock["entries"]
        .as_array()
        .and_then(|e| e.iter().find(|e| e["target"] == json!(tuple.target)))
        .ok_or_else(|| {
            format!(
                "product_lock_mismatch: the lock has no {} entry",
                tuple.target
            )
        })?;
    let artifact = &entry["pin"]["artifact"];
    let fact = |field: &str, product: Option<&str>, locked: &Value| {
        if locked.as_str() == product {
            Ok(())
        } else {
            Err(format!(
                "product_lock_mismatch: {} {field} is {product:?} in the product and {locked} in the lock",
                tuple.target
            ))
        }
    };
    fact(
        "executable_sha256",
        tuple.executable_sha256.as_deref(),
        &artifact["executable_sha256"],
    )?;
    fact(
        "artifact_kind",
        tuple.artifact_kind.as_deref(),
        &artifact["kind"],
    )?;
    let reported = artifact["version_output"].as_str().unwrap_or_default();
    match tuple.version_report.as_deref() {
        Some(v) if !v.is_empty() && reported.contains(v) => Ok(()),
        other => Err(format!(
            "product_lock_mismatch: {} version_report {other:?} is not in the lock's version output {reported:?}",
            tuple.target
        )),
    }
}

/// Read one input record: a regular file, at most [`READ_BOUND`] bytes. The
/// file is opened without blocking and checked on the opened descriptor, so
/// a FIFO or a device is refused without waiting on it.
pub fn read_regular(path: &Path, what: &str) -> Result<Vec<u8>, String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| format!("{what}_unreadable"))?;
    if !file
        .metadata()
        .map_err(|_| format!("{what}_unreadable"))?
        .is_file()
    {
        return Err(format!("{what}_not_regular_file"));
    }
    let mut bytes = Vec::new();
    file.take(READ_BOUND + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("{what}_unreadable"))?;
    if bytes.len() as u64 > READ_BOUND {
        return Err(format!("{what}_too_large"));
    }
    Ok(bytes)
}

/// The inputs `matrix --target` has checked before it spawns anything.
#[derive(Debug, Clone)]
pub struct MatrixTarget {
    pub target: Target,
    pub product: Product,
    pub product_sha256: String,
    /// The competitor's executables, `None` for Swarmail (whose own
    /// executable the runner resolves as ever).
    pub competitor: Option<CompetitorContext>,
    pub tuple: TargetTuple,
}

/// What a competitor trial worker is handed, beside the matrix cell
/// (decision 0129 §4): never a flag a worker could be given on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitorContext {
    pub target: String,
    pub adapter: PathBuf,
    pub target_bin: PathBuf,
    pub sources: PathBuf,
    pub tools_sha256: String,
    pub smtp_auth: String,
}

impl CompetitorContext {
    pub fn plan(&self) -> Result<crate::runner::CompetitorPlan, String> {
        Ok(crate::runner::CompetitorPlan {
            target: Target::parse(&self.target)?,
            adapter: self.adapter.clone(),
            target_bin: self.target_bin.clone(),
            sources: self.sources.clone(),
            tools_sha256: self.tools_sha256.clone(),
            smtp_auth: crate::mail::SmtpAuth::parse(&self.smtp_auth)
                .ok_or("smtp_auth is not plain or none")?,
        })
    }
}

/// Check every input of `matrix --target` and return what the run needs
/// (spec R2). Each refusal is a name followed by its reason, and nothing has
/// been spawned when one is returned: an unknown target, a flag the target
/// does not take, a record that is not a bounded regular file, a product
/// that is not valid or disagrees with the lock, and a `--target-bin` whose
/// digest is not the product tuple's (both digests are named).
pub fn bind_inputs(
    target: &str,
    adapter: Option<&Path>,
    target_bin: Option<&Path>,
    sources: Option<&Path>,
    product: Option<&Path>,
) -> Result<MatrixTarget, String> {
    let target = Target::parse(target)?;
    let product_path = product.ok_or("product_missing: --target needs --product")?;
    let bytes = read_regular(product_path, "product")?;
    let product_sha256 = sha256(&bytes);
    let product = Product::parse(&bytes)?;
    let lock: Option<Value> = sources
        .map(|p| read_regular(p, "sources"))
        .transpose()?
        .map(|b| serde_json::from_slice(&b).map_err(|e| format!("sources_invalid: {e}")))
        .transpose()?;
    product.validate(lock.as_ref())?;
    let tuple = product
        .tuple(target)
        .ok_or("product_invalid: the product has no tuple for the target")?
        .clone();
    if !target.is_competitor() {
        if adapter.is_some() || target_bin.is_some() || sources.is_some() {
            return Err("swarmail_takes_no_adapter: --adapter, --target-bin and --sources are for a competitor".into());
        }
        return Ok(MatrixTarget {
            target,
            product,
            product_sha256,
            competitor: None,
            tuple,
        });
    }
    let (Some(adapter), Some(target_bin), Some(sources)) = (adapter, target_bin, sources) else {
        return Err(format!(
            "competitor_inputs_missing: {} needs --adapter, --target-bin and --sources",
            target.name()
        ));
    };
    let executable = std::fs::metadata(adapter).is_ok_and(|m| {
        use std::os::unix::fs::PermissionsExt;
        m.is_file() && m.permissions().mode() & 0o111 != 0
    });
    if !executable || !adapter.is_absolute() {
        return Err(
            "adapter_not_executable: --adapter is an absolute path to an executable file".into(),
        );
    }
    if !target_bin.is_absolute() || !sources.is_absolute() {
        return Err(
            "competitor_paths_relative: --target-bin and --sources are absolute paths".into(),
        );
    }
    let bin = crate::identity::file_hash(target_bin)
        .map_err(|e| format!("target_bin_unreadable: {e}"))?;
    let want = tuple.executable_sha256.clone().unwrap_or_default();
    if bin != want {
        return Err(format!(
            "target_bin_digest_mismatch: --target-bin hashes to {bin}, the product tuple pins {want}"
        ));
    }
    let competitor = CompetitorContext {
        target: target.name().into(),
        adapter: adapter.into(),
        target_bin: target_bin.into(),
        sources: sources.into(),
        tools_sha256: tuple.tools_sha256.clone(),
        smtp_auth: tuple.smtp_auth.clone(),
    };
    Ok(MatrixTarget {
        target,
        product,
        product_sha256,
        competitor: Some(competitor),
        tuple,
    })
}

impl MatrixTarget {
    /// Refuse a task the product does not request for this target, or one a
    /// competitor cannot take (several agents, or chaos).
    pub fn check_tasks(&self, ids: &[String]) -> Result<(), String> {
        for id in ids {
            let task = crate::tasks::find(id).ok_or_else(|| format!("unknown task {id}"))?;
            let structural = self.target.is_competitor()
                && (task.scenario.uses_chaos() || task.agent_count != 1);
            if structural || !self.product.applicable(id, self.target) {
                return Err(format!(
                    "task_not_applicable: {id} is not requested for {} by product {}",
                    self.target.name(),
                    self.product.id
                ));
            }
        }
        Ok(())
    }
}

/// Builds the schema-3 row a comparison worker's result becomes.
#[derive(Debug, Clone)]
pub struct RowConverter {
    pub protocol_sha256: String,
    pub target: &'static str,
    pub connection: String,
    pub executable_sha256: String,
    pub tools_sha256: String,
    pub smtp_auth: String,
}

impl RowConverter {
    pub fn new(m: &MatrixTarget, executable_sha256: String) -> RowConverter {
        RowConverter {
            protocol_sha256: m.product_sha256.clone(),
            target: m.target.comparison_name(),
            connection: m.tuple.connection.clone(),
            executable_sha256,
            tools_sha256: m.tuple.tools_sha256.clone(),
            smtp_auth: m.tuple.smtp_auth.clone(),
        }
    }

    pub fn convert(&self, row: &crate::runner::TrialResult) -> Result<ComparisonRow, String> {
        let (Some(task), Some(agent), Some(matrix)) =
            (&row.task, &row.agent, &row.provenance.matrix)
        else {
            return Err("a comparison row needs a task, an agent and matrix coordinates".into());
        };
        let comparison = ComparisonRow {
            schema_version: COMPARISON_SCHEMA,
            kind: "comparison".into(),
            protocol_sha256: self.protocol_sha256.clone(),
            target: self.target.into(),
            executable_sha256: self.executable_sha256.clone(),
            settings: crate::identity::ComparisonSettings {
                connection: self.connection.clone(),
                tools_sha256: self.tools_sha256.clone(),
                smtp_auth: self.smtp_auth.clone(),
            },
            coordinate: crate::identity::ComparisonCoordinate {
                task: task.clone(),
                agent: agent.clone(),
                attempt: matrix.repetition_index as u32 + 1,
            },
            outcome: row.outcome,
            hash: String::new(),
        }
        .seal();
        comparison.validate()?;
        Ok(comparison)
    }
}
