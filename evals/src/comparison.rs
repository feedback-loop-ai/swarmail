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
            // The lock entry the target maps to (`Target::lock_entry`); a
            // tuple that names any other entry, the official `mailslurper`
            // row included, is refused by name.
            if tuple.lock_entry.as_deref() != t.lock_entry() {
                out.push(format!(
                    "{n}: lock_entry {:?} is not the target's {:?}",
                    tuple.lock_entry,
                    t.lock_entry()
                ));
            }
            let bound = tuple.lock_entry.as_deref() == t.lock_entry()
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
    // The entry the target maps to, not the entry the tuple claims: a tuple
    // bound to another row is refused before any digest is compared.
    let want = Target::parse(&tuple.target)?.lock_entry();
    if tuple.lock_entry.as_deref() != want {
        return Err(format!(
            "product_lock_mismatch: {} lock_entry is {:?}, the target's is {want:?}",
            tuple.target, tuple.lock_entry
        ));
    }
    let entry = lock["entries"]
        .as_array()
        .and_then(|e| {
            e.iter()
                .find(|e| want.is_some_and(|w| e["target"] == json!(w)))
        })
        .ok_or_else(|| {
            format!(
                "product_lock_mismatch: the lock has no {} entry",
                want.unwrap_or(&tuple.target)
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
                && (task.scenario.uses_chaos()
                    || task.agent_count != 1
                    || task.scenario.reads_reports());
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

// ---- the controls' records ---------------------------------------------------

/// One control row as the index records it: the comparison row's own hash,
/// and what the row cannot carry (the failure class, and a digest of the
/// fixture inputs the cell was generated from, which is what must be equal
/// across targets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexRow {
    pub target: String,
    pub task: String,
    pub agent: String,
    pub attempt: u32,
    pub outcome: crate::tasks::Outcome,
    pub failure_class: Option<crate::state::FailureClass>,
    pub row_sha256: String,
    pub fixture_sha256: String,
}

/// The controls directory's index, bound to the product it ran under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlsIndex {
    pub schema: u32,
    pub product_id: String,
    pub product_sha256: String,
    pub master_seed: String,
    pub fixture_time: String,
    pub rows_sha256: String,
    /// A digest of each target's retained matrix index: the matrix writes it
    /// only when every slot's row was committed, which needs every slot's
    /// cleanup to have been proved or refused by name.
    pub cleanup_receipts_sha256: String,
    pub rows: Vec<IndexRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityEntry {
    pub role: String,
    pub path: String,
    pub sha256: String,
}

/// Every measured executable: the candidate, each competitor, the adapter
/// driver and the harness. The verifier re-measures each at its path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub schema: u32,
    pub product_sha256: String,
    pub entries: Vec<IdentityEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalSuite {
    pub command: String,
    pub head: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ci {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipts {
    pub schema: u32,
    pub head: String,
    pub controls_index_sha256: String,
    pub cleanup_receipts_sha256: String,
    pub eval_suite: EvalSuite,
    pub ci: Ci,
}

/// Retained trial results by `(task, agent, attempt)`.
type Retained = BTreeMap<(String, String, u32), Value>;

/// The digest of the fixture inputs a retained result was generated from:
/// the task's definition and the cell's seed and time. The fixture never
/// sees the target, so for one (task, agent, attempt) it is the same on all.
fn fixture_digest(result: &Value) -> String {
    let p = &result["provenance"];
    sha256(
        &serde_json::to_vec(&json!({
            "task_definition_sha256": p["task"]["definition_sha256"]["value"],
            "fixture": p["fixture"],
        }))
        .expect("fixture digest input"),
    )
}

/// Every retained trial result under `artifacts` (a `--artifacts-dir` root),
/// as `(task, agent, attempt) -> result.json`, and the digest of the matrix
/// index that proves the matrix finished.
fn retained_results(artifacts: &Path) -> Result<(Retained, String), String> {
    let matrix = std::fs::read_dir(artifacts)
        .map_err(|e| format!("artifacts: {e}"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("matrix.json").is_file())
        .collect::<Vec<_>>();
    let [matrix] = matrix.as_slice() else {
        return Err(format!(
            "{} holds {} finished matrices, not one",
            artifacts.display(),
            matrix.len()
        ));
    };
    let index = read_regular(&matrix.join("matrix.json"), "matrix_index")?;
    let mut results = BTreeMap::new();
    for entry in std::fs::read_dir(matrix.join("trials")).map_err(|e| format!("trials: {e}"))? {
        let path = entry.map_err(|e| e.to_string())?.path().join("result.json");
        let bytes = read_regular(&path, "result")?;
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("result: {e}"))?;
        let key = (
            v["task"].as_str().unwrap_or_default().to_owned(),
            v["agent"].as_str().unwrap_or_default().to_owned(),
            v["provenance"]["matrix"]["repetition_index"]
                .as_u64()
                .unwrap_or(u64::MAX) as u32
                + 1,
        );
        if results.insert(key.clone(), v).is_some() {
            return Err(format!("two retained results for {key:?}"));
        }
    }
    Ok((results, sha256(&index)))
}

/// Build the controls index from each target's rows and retained artifacts.
/// `rows` is the concatenation in product order; `per_target` gives each
/// target's retained-artifacts root.
pub fn build_index(
    product: &Product,
    product_sha256: &str,
    rows_bytes: &[u8],
    per_target: &[(String, PathBuf)],
) -> Result<ControlsIndex, String> {
    let rows: Vec<ComparisonRow> = std::str::from_utf8(rows_bytes)
        .map_err(|_| "rows are not UTF-8")?
        .lines()
        .map(|l| serde_json::from_str(l).map_err(|e| format!("a row does not parse: {e}")))
        .collect::<Result<_, _>>()?;
    let mut retained = BTreeMap::new();
    let mut receipts = Vec::new();
    for (target, dir) in per_target {
        let (results, matrix_digest) = retained_results(dir)?;
        receipts.push(format!("{target}:{matrix_digest}"));
        retained.insert(target.clone(), results);
    }
    let mut out = Vec::new();
    let (mut seed, mut time) = (String::new(), String::new());
    for row in &rows {
        row.validate()?;
        let target = target::ALL
            .iter()
            .find(|t| t.comparison_name() == row.target)
            .ok_or("a row names an unknown target")?;
        let key = (
            row.coordinate.task.clone(),
            row.coordinate.agent.clone(),
            row.coordinate.attempt,
        );
        let result = retained
            .get(target.name())
            .and_then(|r| r.get(&key))
            .ok_or_else(|| format!("no retained result for {} {key:?}", target.name()))?;
        seed = result["provenance"]["matrix"]["master_seed"]
            .as_str()
            .unwrap_or_default()
            .into();
        time = result["provenance"]["fixture"]["fixture_time"]
            .as_str()
            .unwrap_or_default()
            .into();
        out.push(IndexRow {
            target: target.name().into(),
            task: key.0,
            agent: key.1,
            attempt: key.2,
            outcome: row.outcome,
            failure_class: serde_json::from_value(result["failure_class"].clone()).unwrap_or(None),
            row_sha256: row.hash.clone(),
            fixture_sha256: fixture_digest(result),
        });
    }
    Ok(ControlsIndex {
        schema: 1,
        product_id: product.id.clone(),
        product_sha256: product_sha256.into(),
        master_seed: seed,
        fixture_time: time,
        rows_sha256: sha256(rows_bytes),
        cleanup_receipts_sha256: sha256(receipts.join("\n").as_bytes()),
        rows: out,
    })
}

// ---- the verifier ---------------------------------------------------------------

/// What `comparison-check` is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Verify, print the plan and the pending items, launch nothing.
    DryRun,
    /// Verify for the evidence step: a pending CI run is allowed.
    Evidence,
    /// Verify for the paid start: a pending item is refused by name.
    Measure,
}

/// The paths a check reads. Nothing here is a wrapper configuration.
#[derive(Debug, Clone)]
pub struct CheckInputs {
    pub baseline: PathBuf,
    pub contracts: PathBuf,
    pub controls_dir: PathBuf,
    pub receipts: PathBuf,
    pub cost: PathBuf,
    pub identity: PathBuf,
    pub product: PathBuf,
    pub sources: PathBuf,
    /// The repository the receipts' head is checked in.
    pub repo: PathBuf,
}

/// The paths a change may touch between the receipts' head and the
/// candidate's: the evidence itself. Any code, lock, product or script change
/// makes the receipts stale.
const EVIDENCE_PATHS: [&str; 3] = [
    "docs/evals/agent-controls-",
    "docs/evals/README.md",
    "specs/",
];

/// What a passing check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub product: Product,
    pub controls_rows: usize,
    pub pending: Vec<String>,
}

fn refuse<T>(name: &str, why: impl std::fmt::Display) -> Result<T, String> {
    Err(format!("prerequisite-{name}: {why}"))
}

fn json_file(path: &Path, what: &str, name: &str) -> Result<(Value, Vec<u8>), String> {
    let bytes = read_regular(path, what).or_else(|e| refuse(name, e))?;
    let v = serde_json::from_slice(&bytes)
        .or_else(|e| refuse(name, format!("{what} does not parse: {e}")))?;
    Ok((v, bytes))
}

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    git_raw(repo, args).map(|out| out.trim().to_owned())
}

/// `git`'s output exactly as written: `status --porcelain` starts a line
/// with a space for an unstaged change, which trimming would eat.
fn git_raw(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Verify every prerequisite in the fixed order and name the first that does
/// not hold as `prerequisite-<name>: <reason>`.
pub fn check(inputs: &CheckInputs, mode: Mode) -> Result<Verified, String> {
    let product_bytes =
        read_regular(&inputs.product, "product").or_else(|e| refuse("full-product", e))?;
    let product_sha = sha256(&product_bytes);
    let product = Product::parse(&product_bytes).or_else(|e| refuse("full-product", e))?;
    // 1. the baseline: the published #151 report.
    let baseline = std::fs::metadata(&inputs.baseline).or_else(|e| refuse("baseline", e))?;
    let baseline_ok = if baseline.is_dir() {
        std::fs::read_dir(&inputs.baseline).is_ok_and(|mut d| d.next().is_some())
    } else {
        read_regular(&inputs.baseline, "baseline")
            .is_ok_and(|b| String::from_utf8_lossy(&b).contains("#151"))
    };
    if !baseline_ok {
        return refuse("baseline", "it is not the published #151 report");
    }
    check_contracts(inputs, &product)?;
    let (index, rows) = check_controls(inputs, &product, &product_sha)?;
    let pending = check_receipts(inputs, &index, mode)?;
    check_cost(inputs)?;
    check_identity(inputs, &product, &product_sha, &rows)?;
    let (lock, _) = json_file(&inputs.sources, "sources", "full-product")?;
    product
        .validate(Some(&lock))
        .or_else(|e| refuse("full-product", e))?;
    Ok(Verified {
        product,
        controls_rows: rows.len(),
        pending,
    })
}

fn check_contracts(inputs: &CheckInputs, product: &Product) -> Result<(), String> {
    const NAME: &str = "final-contracts";
    for tuple in &product.targets {
        let path = inputs.contracts.join(format!("{}.json", tuple.target));
        let (c, _) = json_file(&path, "contract", NAME)?;
        let want = |ok: bool, why: &str| {
            if ok {
                Ok(())
            } else {
                refuse(NAME, format!("{}: {why}", tuple.target))
            }
        };
        want(
            c["target"] == json!(tuple.target),
            "the contract is for another target",
        )?;
        want(
            c["tools_sha256"] == json!(tuple.tools_sha256),
            "its tool list is not the product's",
        )?;
        want(
            c["negotiation"]["protocol_version"] == json!(crate::mcp::PROTOCOL_VERSION),
            "it did not negotiate the protocol version",
        )?;
        want(
            c["batch"]["answered"] == json!(true) && c["batch"]["reply_is_array"] == json!(true),
            "its batch probe was not answered",
        )?;
        want(
            c["cleanup_receipt"] == json!(true),
            "its probe left no cleanup receipt",
        )?;
        let strings = |key: &str, field: Option<&str>| -> Vec<String> {
            c[key]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| match field {
                            Some(f) => x[f].as_str(),
                            None => x.as_str(),
                        })
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        let (names, called) = (strings("tool_names", None), strings("calls", Some("tool")));
        want(
            !names.is_empty() && names == called,
            "it lists one outcome per announced tool",
        )?;
        let skipped = c["calls"]
            .as_array()
            .is_some_and(|a| a.iter().any(|x| x["outcome"] == "skipped_mutating"));
        want(
            !(tuple.target != "swarmail" && skipped),
            "a competitor tool was skipped",
        )?;
    }
    Ok(())
}

fn check_controls(
    inputs: &CheckInputs,
    product: &Product,
    product_sha: &str,
) -> Result<(ControlsIndex, Vec<ComparisonRow>), String> {
    const NAME: &str = "controls";
    let rows_bytes = read_regular(&inputs.controls_dir.join("rows.jsonl"), "rows")
        .or_else(|e| refuse(NAME, e))?;
    if rows_bytes.last() != Some(&b'\n') {
        return refuse(NAME, "rows.jsonl ends in a torn line");
    }
    let (index_value, _) = json_file(&inputs.controls_dir.join("index.json"), "index", NAME)?;
    let index: ControlsIndex =
        serde_json::from_value(index_value).or_else(|e| refuse(NAME, format!("index: {e}")))?;
    if index.product_sha256 != product_sha || index.product_id != product.id {
        return refuse(NAME, "the controls are not bound to this product");
    }
    if index.rows_sha256 != sha256(&rows_bytes) {
        return refuse(NAME, "rows.jsonl is not the file the index binds");
    }
    let rows: Vec<ComparisonRow> = std::str::from_utf8(&rows_bytes)
        .or_else(|_| refuse(NAME, "rows are not UTF-8"))?
        .lines()
        .map(|l| {
            serde_json::from_str(l).or_else(|e| refuse(NAME, format!("a row does not parse: {e}")))
        })
        .collect::<Result<_, _>>()?;
    let per_target = (product.tasks.len() as u32 * 2 * product.controls.trials) as usize;
    for tuple in &product.targets {
        let t = Target::parse(&tuple.target)?;
        let own: Vec<&ComparisonRow> = rows
            .iter()
            .filter(|r| r.target == t.comparison_name())
            .collect();
        if own.is_empty() {
            return refuse(NAME, format!("the controls hold no rows for {}", t.name()));
        }
        if own.len() != per_target {
            return refuse(
                NAME,
                format!(
                    "{} has {} control rows, the product plans {per_target}",
                    t.name(),
                    own.len()
                ),
            );
        }
        for task in &product.tasks {
            for agent in &product.controls.agents {
                for attempt in 1..=product.controls.trials {
                    let n = own
                        .iter()
                        .filter(|r| {
                            r.coordinate.task == *task
                                && r.coordinate.agent == *agent
                                && r.coordinate.attempt == attempt
                        })
                        .count();
                    if n != 1 {
                        return refuse(
                            NAME,
                            format!(
                                "{}: control row {task} {agent} {attempt} is missing or repeated",
                                t.name()
                            ),
                        );
                    }
                }
            }
        }
    }
    if rows.len() != product.controls.cells as usize || rows.len() != index.rows.len() {
        return refuse(NAME, "the control rows are not the product's planned cells");
    }
    for (row, entry) in rows.iter().zip(&index.rows) {
        row.validate().or_else(|e| refuse(NAME, e))?;
        let tuple = product
            .targets
            .iter()
            .find(|t| t.comparison_name == row.target)
            .ok_or_else(|| format!("prerequisite-{NAME}: a row names an unknown target"))?;
        let bound = row.protocol_sha256 == product_sha
            && row.settings.connection == tuple.connection
            && row.settings.tools_sha256 == tuple.tools_sha256
            && row.settings.smtp_auth == tuple.smtp_auth
            && tuple
                .executable_sha256
                .as_ref()
                .is_none_or(|e| *e == row.executable_sha256)
            && entry.row_sha256 == row.hash
            && entry.target == tuple.target
            && (entry.task.as_str(), entry.agent.as_str(), entry.attempt)
                == (
                    row.coordinate.task.as_str(),
                    row.coordinate.agent.as_str(),
                    row.coordinate.attempt,
                );
        if !bound {
            return refuse(
                NAME,
                format!(
                    "{} {:?} is not bound to the product",
                    row.target, row.coordinate
                ),
            );
        }
        let idle = crate::tasks::find(&row.coordinate.task).map(|t| t.idle_class);
        let ok = match (row.coordinate.agent.as_str(), row.outcome) {
            ("oracle", crate::tasks::Outcome::Pass) => entry.failure_class.is_none(),
            ("null", crate::tasks::Outcome::Fail) => entry.failure_class == idle,
            _ => false,
        };
        if !ok {
            return refuse(
                NAME,
                format!(
                    "{} {:?}: {:?} is not what the {} control must give",
                    row.target, row.coordinate, row.outcome, row.coordinate.agent
                ),
            );
        }
    }
    // The fixture never sees the target: one (task, agent, attempt) is the
    // same fixture on every target.
    let mut seen: BTreeMap<(&str, &str, u32), &str> = BTreeMap::new();
    for e in &index.rows {
        let want = *seen
            .entry((&e.task, &e.agent, e.attempt))
            .or_insert(&e.fixture_sha256);
        if want != e.fixture_sha256 {
            return refuse(
                NAME,
                format!(
                    "{} {} {}: the fixture differs between targets",
                    e.task, e.agent, e.attempt
                ),
            );
        }
    }
    Ok((index, rows))
}

fn check_receipts(
    inputs: &CheckInputs,
    index: &ControlsIndex,
    mode: Mode,
) -> Result<Vec<String>, String> {
    const NAME: &str = "coordination-receipts";
    let (v, _) = json_file(&inputs.receipts, "receipts", NAME)?;
    let r: Receipts =
        serde_json::from_value(v).or_else(|e| refuse(NAME, format!("receipts: {e}")))?;
    let index_bytes = read_regular(&inputs.controls_dir.join("index.json"), "index")
        .or_else(|e| refuse(NAME, e))?;
    if r.controls_index_sha256 != sha256(&index_bytes)
        || r.cleanup_receipts_sha256 != index.cleanup_receipts_sha256
    {
        return refuse(NAME, "the receipts are not bound to this controls index");
    }
    if r.eval_suite.status != "pass"
        || r.eval_suite.head != r.head
        || !r.eval_suite.command.contains("--locked")
    {
        return refuse(
            NAME,
            "the locked eval suite did not pass at the receipts' head",
        );
    }
    if r.head.len() < 7 || !r.head.bytes().all(|b| b.is_ascii_hexdigit()) {
        return refuse(NAME, "the receipts' head is not a commit");
    }
    let candidate = git(&inputs.repo, &["rev-parse", "HEAD"]).or_else(|e| refuse(NAME, e))?;
    if git(
        &inputs.repo,
        &["merge-base", "--is-ancestor", &r.head, &candidate],
    )
    .is_err()
    {
        return refuse(
            NAME,
            format!(
                "the receipts' head {} is not an ancestor of the candidate {candidate}",
                r.head
            ),
        );
    }
    let changed = git(&inputs.repo, &["diff", "--name-only", &r.head, &candidate])
        .or_else(|e| refuse(NAME, e))?;
    let dirty = git_raw(
        &inputs.repo,
        &["status", "--porcelain", "--untracked-files=all"],
    )
    .or_else(|e| refuse(NAME, e))?;
    let paths = changed
        .lines()
        .map(str::to_owned)
        .chain(dirty.lines().map(|l| l.get(3..).unwrap_or(l).to_owned()));
    for p in paths {
        if !EVIDENCE_PATHS.iter().any(|ok| p.starts_with(ok)) {
            return refuse(
                NAME,
                format!(
                    "{p} changed since the receipts' head {}: the receipts are stale",
                    r.head
                ),
            );
        }
    }
    let mut pending = Vec::new();
    match (r.ci.status.as_str(), &r.ci.run) {
        ("pending", None) => pending.push(
            "ci: the required CI run has not been supplied (receipts.ci.status is pending)"
                .to_owned(),
        ),
        ("run", Some(run)) if !run.trim().is_empty() => {}
        _ => return refuse(NAME, "receipts.ci is neither pending nor a run reference"),
    }
    if mode == Mode::Measure && !pending.is_empty() {
        return refuse(NAME, pending.join("; "));
    }
    Ok(pending)
}

fn check_cost(inputs: &CheckInputs) -> Result<(), String> {
    const NAME: &str = "cost-basis";
    let (v, _) = json_file(&inputs.cost, "cost", NAME)?;
    match v["basis"].as_str() {
        Some("unavailable") if v["reason"].as_str().is_some_and(|r| !r.trim().is_empty()) => Ok(()),
        Some("measured") if v["total_usd"].as_f64().is_some_and(|c| c > 0.0) => Ok(()),
        Some("unavailable") => refuse(NAME, "an unavailable basis says why"),
        Some("measured") => refuse(NAME, "a measured cost is never 0"),
        _ => refuse(NAME, "the basis is neither unavailable nor measured"),
    }
}

fn check_identity(
    inputs: &CheckInputs,
    product: &Product,
    product_sha: &str,
    rows: &[ComparisonRow],
) -> Result<(), String> {
    const NAME: &str = "identity";
    let (v, _) = json_file(&inputs.identity, "identity", NAME)?;
    let id: Identity =
        serde_json::from_value(v).or_else(|e| refuse(NAME, format!("identity: {e}")))?;
    if id.product_sha256 != product_sha {
        return refuse(NAME, "the identity is not bound to this product");
    }
    for role in [
        "swarmail",
        "mailpit",
        "mailhog",
        "mailslurper",
        "adapter",
        "harness",
    ] {
        if id.entries.iter().filter(|e| e.role == role).count() != 1 {
            return refuse(
                NAME,
                format!("the identity does not hold exactly one {role}"),
            );
        }
    }
    for e in &id.entries {
        // Re-measure: the executable at the recorded path is the one measured.
        let now = crate::identity::file_hash(Path::new(&e.path)).or_else(|_| {
            refuse(
                NAME,
                format!("{} at {} cannot be re-measured", e.role, e.path),
            )
        })?;
        if now != e.sha256 {
            return refuse(
                NAME,
                format!(
                    "{} at {} is {now} now, and was {} when measured",
                    e.role, e.path, e.sha256
                ),
            );
        }
        if let Some(tuple) = product.targets.iter().find(|t| t.target == e.role)
            && tuple
                .executable_sha256
                .as_ref()
                .is_some_and(|want| *want != e.sha256)
        {
            return refuse(
                NAME,
                format!("{} is not the executable the product pins", e.role),
            );
        }
    }
    let candidate = id
        .entries
        .iter()
        .find(|e| e.role == "swarmail")
        .map(|e| e.sha256.as_str());
    if rows
        .iter()
        .any(|r| r.target == "swarmail-mcp" && Some(r.executable_sha256.as_str()) != candidate)
    {
        return refuse(
            NAME,
            "the swarmail rows name another executable than the identity's",
        );
    }
    Ok(())
}

impl Verified {
    /// The dry run's transcript: what was verified, the plan, the pending.
    pub fn dry_run_report(&self) -> String {
        let p = &self.product;
        let mut out = format!(
            "comparison-check dry run: every prerequisite holds for product {} ({} control rows, {} contracts)\n",
            p.id,
            self.controls_rows,
            p.targets.len()
        );
        out.push_str(&format!(
            "plan: {} slots = {} targets x {} tasks x {} agents x {} trials, one slot at a time, in this order (one matrix per target, as `compare-mail.sh measure` runs them):\n",
            p.planned_cells,
            p.targets.len(),
            p.tasks.len(),
            p.agents.len(),
            p.trials
        ));
        let block = p.agents.len() as u32 * p.trials;
        let labels = p
            .agents
            .iter()
            .map(|a| a.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let mut slot = 1;
        for t in &p.targets {
            for task in &p.tasks {
                out.push_str(&format!(
                    "  slots {:>3}-{:>3}: {task} on {} for {labels}, {} trials each\n",
                    slot,
                    slot + block - 1,
                    t.target,
                    p.trials
                ));
                slot += block;
            }
        }
        out.push_str("pending (operator): --authorized-product with the product id, and the operator's confirmation of the spend\n");
        for item in &self.pending {
            out.push_str(&format!("pending: {item}\n"));
        }
        out.push_str("nothing was launched and no wrapper configuration was opened\n");
        out
    }
}
