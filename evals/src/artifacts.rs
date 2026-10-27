//! Detectable completion: metadata binds bytes and is published last.
use crate::{
    provenance::*,
<<<<<<< HEAD
    results,
=======
    results::{self, ResultRecord},
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
    runner::TrialResult,
    state::Snapshot,
    tasks::{self, Outcome},
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub const PLAN_LIMIT: usize = 8 * 1024 * 1024;
pub const STATE_LIMIT: usize = 16 * 1024 * 1024;
pub const SMALL_LIMIT: usize = 64 * 1024;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub path: String,
    pub length: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateBinding {
    Present { file: Binding },
    Absent { reason: AbsentReason },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbsentReason {
    NotCaptured,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub schema_version: u32,
    pub trial_id: String,
    pub result: Binding,
    pub state: StateBinding,
    pub diagnostics: Binding,
}
pub fn generated_id(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn encode<T: Serialize>(v: &T, limit: usize) -> Result<Vec<u8>, String> {
    struct Bounded {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other("encoded size limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut out, v).map_err(|_| "serialization size limit")?;
    Ok(out.bytes)
}
/// Check each existing component before descending or creating a child.
pub fn ensure_root(root: &Path) -> Result<(), String> {
    let mut path = PathBuf::new();
    for c in root.components() {
        if matches!(c, std::path::Component::ParentDir) {
            return Err("parent path refused".into());
        }
        path.push(c);
        match std::fs::symlink_metadata(&path) {
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
            Ok(_) => return Err("root component is not a regular directory".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Err(e) = std::fs::create_dir(&path)
                    && e.kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err("directory creation failed".into());
                }
                let m = std::fs::symlink_metadata(&path).map_err(|_| "directory unavailable")?;
                if !m.is_dir() || m.file_type().is_symlink() {
                    return Err("root component is not a regular directory".into());
                }
            }
            Err(_) => return Err("root unavailable".into()),
        }
    }
    safe_root(root)
}
pub fn safe_root(root: &Path) -> Result<(), String> {
    let mut path = PathBuf::new();
    for c in root.components() {
        if matches!(c, std::path::Component::ParentDir) {
            return Err("parent path refused".into());
        }
        path.push(c);
        if std::fs::symlink_metadata(&path)
            .map_err(|_| "path unavailable")?
            .file_type()
            .is_symlink()
        {
            return Err("symlink refused".into());
        }
    }
    if !root.is_dir() {
        return Err("root not a directory".into());
    }
    Ok(())
}
pub fn read_file(root: &Path, name: &str, limit: usize) -> Result<Vec<u8>, String> {
    safe_root(root)?;
    if name.contains('/') || matches!(name, "." | "..") {
        return Err("invalid fixed filename".into());
    }
    use std::os::unix::fs::OpenOptionsExt;
    let path = root.join(name);
    if std::fs::symlink_metadata(&path)
        .map_err(|_| "file missing")?
        .file_type()
        .is_symlink()
    {
        return Err("symlink refused".into());
    }
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "file unavailable")?;
    let meta = f.metadata().map_err(|_| "file metadata")?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err("file type/size refused".into());
    }
    let mut bytes = Vec::new();
    (&mut f)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "file read failed")?;
    if bytes.len() > limit {
        return Err("file grew beyond limit".into());
    }
    Ok(bytes)
}
pub fn publish(root: &Path, name: &str, bytes: &[u8]) -> Result<Binding, String> {
    safe_root(root)?;
    crate::runner::write_artifact_with(root, name, bytes, |f, b| f.write_all(b))?;
    Ok(Binding {
        path: name.into(),
        length: bytes.len() as u64,
        sha256: sha256(bytes),
    })
}
pub fn retain_trial(root: &Path, row: &TrialResult, snap: Option<&Snapshot>) -> Result<(), String> {
    retain_with(root, row, snap, publish)
}
fn retain_with(
    root: &Path,
    row: &TrialResult,
    snap: Option<&Snapshot>,
    mut publish: impl FnMut(&Path, &str, &[u8]) -> Result<Binding, String>,
) -> Result<(), String> {
    if !generated_id(&row.trial_id) {
        return Err("invalid generated identity".into());
    }
    ensure_root(root)?;
    let out = crate::runner::create_private_artifact_dir(root, &row.trial_id)?;
<<<<<<< HEAD
    row.validate().map_err(|e| {
        if e.starts_with("schema_version ") {
            "result-version mismatch"
        } else {
            "invalid result"
        }
    })?;
=======
    row.validate().map_err(|_| "invalid result")?;
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
    if let Some(s) = snap {
        crate::state::validate_publication(s)?;
        if s.trial_id != row.trial_id || row.task.as_deref() != Some(&s.task) {
            return Err("snapshot association mismatch".into());
        }
        tasks::validate_snapshot_structure(
            &tasks::find(&s.task).ok_or("unknown snapshot task")?,
            s,
        )
        .map_err(|_| "invalid snapshot structure")?;
    } else if row.outcome != Outcome::HarnessError {
        return Err("business result requires state".into());
    }
    let result = publish(&out, "result.json", &encode(row, SMALL_LIMIT - 1)?)?;
    let state = match snap {
        Some(s) => StateBinding::Present {
            file: publish(&out, "state.json", &encode(s, STATE_LIMIT)?)?,
        },
        None => StateBinding::Absent {
            reason: AbsentReason::NotCaptured,
        },
    };
    // No child logs, arbitrary errors, paths, prompt data or panic text.
    let diagnostics = publish(
        &out,
        "diagnostics.log",
        if row.outcome == Outcome::HarnessError {
            b"harness_error\n"
        } else {
            b"trial_finished\n"
        },
    )?;
    let meta = Metadata {
        schema_version: 1,
        trial_id: row.trial_id.clone(),
        result,
        state,
        diagnostics,
    };
    publish(&out, "metadata.json", &encode(&meta, SMALL_LIMIT)?)?;
    Ok(())
}
fn bound(root: &Path, b: &Binding, name: &str, limit: usize) -> Result<Vec<u8>, String> {
    if b.path != name || b.length > limit as u64 || !is_hash(&b.sha256) {
        return Err("invalid binding".into());
    }
    let bytes = read_file(root, name, limit)?;
    if bytes.len() as u64 != b.length || sha256(&bytes) != b.sha256 {
        return Err("byte binding mismatch".into());
    }
    Ok(bytes)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedCell {
    pub trial_id: String,
    pub task: String,
    pub agent: String,
    pub fixture: FixtureInputs,
    pub coordinates: MatrixCoordinates,
    pub deadline_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixPlan {
    pub schema_version: u32,
    pub matrix_id: String,
    pub master: FixtureInputs,
    #[serde(deserialize_with = "crate::provenance::cells")]
    pub tasks: Vec<String>,
    #[serde(deserialize_with = "crate::provenance::cells")]
    pub agents: Vec<String>,
    pub repetitions: u32,
    #[serde(deserialize_with = "crate::provenance::cells")]
    pub cells: Vec<PlannedCell>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixIndex {
    pub schema_version: u32,
    pub matrix_id: String,
    pub plan: Binding,
    #[serde(deserialize_with = "crate::provenance::cells")]
    pub metadata: Vec<Binding>,
}

#[derive(Debug)]
pub struct ValidatedTrial {
    pub row: TrialResult,
    pub state: Option<Snapshot>,
}
pub fn trial(root: &Path) -> Result<ValidatedTrial, String> {
    let meta: Metadata = decode(&read_file(root, "metadata.json", SMALL_LIMIT)?)
        .map_err(|_| "invalid metadata schema")?;
    if meta.schema_version != 1 || !generated_id(&meta.trial_id) {
        return Err("metadata version/identity".into());
    }
<<<<<<< HEAD
    let record =
        results::read(&bound(root, &meta.result, "result.json", SMALL_LIMIT)?).map_err(|e| {
            if e.starts_with("schema_version ") {
                "result-version mismatch"
            } else {
                "invalid result"
            }
        })?;
    let row = record;
=======
    let record = results::read(&bound(root, &meta.result, "result.json", SMALL_LIMIT)?)
        .map_err(|_| "invalid result")?;
    let ResultRecord::V2(row) = record else {
        return Err("result-version mismatch".into());
    };
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
    if meta.trial_id != row.trial_id {
        return Err("result/metadata identity mismatch".into());
    }
    if row.provenance.task.catalogue_sha256 != catalogue_hash() {
        return Err("incompatible catalogue".into());
    }
    match (&row.task, &row.provenance.task.definition_sha256) {
        (Some(id), Availability::Present { value, .. }) if Some(value.clone()) == task_hash(id) => {
        }
        (
            _,
            Availability::Unavailable {
                reason: Reason::RequestInvalid,
            },
        ) if row.outcome == Outcome::HarnessError => {}
        _ => return Err("task definition mismatch".into()),
    }
    let state = match &meta.state {
        StateBinding::Present { file } => {
            let s: Snapshot = strict_snapshot(&bound(root, file, "state.json", STATE_LIMIT)?)
                .map_err(|_| "invalid snapshot")?;
            if s.trial_id != row.trial_id || row.task.as_deref() != Some(s.task.as_str()) {
                return Err("snapshot identity mismatch".into());
            }
            let task = tasks::find(&s.task).ok_or("unknown snapshot task")?;
            tasks::validate_snapshot_structure(&task, &s)
                .map_err(|_| "invalid snapshot structure")?;
            Some(s)
        }
        StateBinding::Absent { .. } => {
            if row.outcome != Outcome::HarnessError {
                return Err("missing state on business result".into());
            }
            None
        }
    };
    bound(root, &meta.diagnostics, "diagnostics.log", SMALL_LIMIT)?;
<<<<<<< HEAD
    Ok(ValidatedTrial { row, state })
=======
    Ok(ValidatedTrial { row: *row, state })
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BusinessReplay {
    Scored { verdict: tasks::Verdict },
    Unscorable { reason: &'static str },
}
#[derive(Debug, Clone, Serialize)]
pub struct ReplayRow {
    pub schema_version: u32,
    pub trial_id: String,
    pub coordinates: Option<MatrixCoordinates>,
    pub original_result: TrialResult,
    pub business_replay: BusinessReplay,
}
impl ValidatedTrial {
    pub fn replay(self) -> ReplayRow {
        let verdict = self
            .row
            .task
            .as_deref()
            .and_then(tasks::find)
            .zip(self.state.as_ref())
            .and_then(|(t, s)| tasks::score(&t, s).ok());
        let business_replay = match verdict {
            Some(verdict) => BusinessReplay::Scored { verdict },
            None => BusinessReplay::Unscorable {
                reason: if self.state.is_none() {
                    "state_absent"
                } else {
                    "state_ineligible"
                },
            },
        };
        ReplayRow {
            schema_version: 1,
            trial_id: self.row.trial_id.clone(),
            coordinates: self.row.provenance.matrix.clone(),
            original_result: self.row,
            business_replay,
        }
    }
}

pub fn validate_plan(plan: &MatrixPlan) -> Result<(), String> {
    plan.master.validate()?;
    let count = plan
        .tasks
        .len()
        .checked_mul(plan.agents.len())
        .and_then(|n| n.checked_mul(plan.repetitions as usize))
        .ok_or("product overflow")?;
    if plan.schema_version != 1
        || !generated_id(&plan.matrix_id)
        || plan.tasks.is_empty()
        || plan.agents.is_empty()
        || plan.repetitions == 0
        || count > 4096
        || count != plan.cells.len()
    {
        return Err("invalid matrix dimensions/version".into());
    }
    let mut labels = std::collections::HashSet::new();
    for label in &plan.agents {
        if label.is_empty() || label.len() > 1024 || !labels.insert(label) {
            return Err("invalid agent labels".into());
        }
    }
    let mut ids = std::collections::HashSet::new();
    for (i, c) in plan.cells.iter().enumerate() {
        let ti = i / (plan.agents.len() * plan.repetitions as usize);
        let ai = (i / plan.repetitions as usize) % plan.agents.len();
        let ri = i % plan.repetitions as usize;
        let expected = MatrixCoordinates {
            matrix_id: plan.matrix_id.clone(),
            master_seed: plan.master.seed.clone(),
            task_index: ti as u64,
            agent_index: ai as u64,
            repetition_index: ri as u64,
        };
        if c.task != plan.tasks[ti]
            || tasks::find(&c.task).is_none()
            || c.agent != plan.agents[ai]
            || c.coordinates != expected
            || c.fixture.seed != expected.cell_seed(&c.task, &c.agent)?
            || c.fixture.fixture_time != plan.master.fixture_time
            || !generated_id(&c.trial_id)
            || !ids.insert(&c.trial_id)
            || c.deadline_ms == 0
            || c.deadline_ms > tasks::MAX_DEADLINE_MS
        {
            return Err("invalid cell identity/inputs/order".into());
        }
        c.fixture.validate()?;
    }
    Ok(())
}
pub fn matches_cell(row: &TrialResult, c: &PlannedCell) -> Result<(), String> {
<<<<<<< HEAD
    row.validate().map_err(|e| {
        if e.starts_with("schema_version ") {
            "result-version mismatch"
        } else {
            "invalid result"
        }
    })?;
=======
    row.validate().map_err(|_| "invalid result")?;
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
    if row.trial_id != c.trial_id
        || row.task.as_deref() != Some(&c.task)
        || row.agent.as_deref() != Some(&c.agent)
        || row.provenance.fixture != c.fixture
        || row.provenance.matrix.as_ref() != Some(&c.coordinates)
        || row.provenance.limits.agent_deadline_ms
            != Availability::present(c.deadline_ms, Source::ResolvedInput)
    {
        return Err("wrong cell result".into());
    }
    Ok(())
}
pub fn matrix(root: &Path) -> Result<Vec<ReplayRow>, String> {
    let index: MatrixIndex =
        decode(&read_file(root, "matrix.json", PLAN_LIMIT)?).map_err(|_| "invalid matrix index")?;
    let plan: MatrixPlan = decode(&bound(root, &index.plan, "matrix-plan.json", PLAN_LIMIT)?)
        .map_err(|_| "invalid matrix plan")?;
    validate_plan(&plan)?;
    if index.schema_version != 1
        || index.matrix_id != plan.matrix_id
        || index.metadata.len() != plan.cells.len()
    {
        return Err("invalid index dimensions/identity".into());
    }
    let trials = root.join("trials");
    safe_root(&trials)?;
    let mut out = Vec::new();
    for (c, b) in plan.cells.iter().zip(&index.metadata) {
        if b.path != format!("trials/{}/metadata.json", c.trial_id) {
            return Err("invalid metadata path".into());
        }
        let trial_root = trials.join(&c.trial_id);
        let mut relative = b.clone();
        relative.path = "metadata.json".into();
        bound(&trial_root, &relative, "metadata.json", SMALL_LIMIT)?;
        let v = trial(&trial_root)?;
        matches_cell(&v.row, c)?;
        out.push(v.replay());
    }
    Ok(out)
}
pub fn metadata_binding(root: &Path, id: &str) -> Result<Binding, String> {
    if !generated_id(id) {
        return Err("invalid identity".into());
    }
    let bytes = read_file(&root.join("trials").join(id), "metadata.json", SMALL_LIMIT)?;
    Ok(Binding {
        path: format!("trials/{id}/metadata.json"),
        length: bytes.len() as u64,
        sha256: sha256(&bytes),
    })
}
pub fn replay_exit(rows: &[ReplayRow]) -> i32 {
    if rows.iter().any(|r|matches!(r.business_replay,BusinessReplay::Unscorable{..})) {2}
    else if rows.iter().any(|r|matches!(&r.business_replay,BusinessReplay::Scored{verdict} if verdict.outcome!=Outcome::Pass)) {1}
    else {0}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_publication_stage_refuses_without_a_completion_marker() {
        let req = crate::runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("null".into()),
            ..Default::default()
        };
        let row = crate::runner::refused_row(&req, &"0".repeat(32));
        let snap = Snapshot {
<<<<<<< HEAD
            schema_version: crate::state::SNAPSHOT_SCHEMA,
=======
            schema_version: 1,
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
            trial_id: row.trial_id.clone(),
            task: "signup".into(),
            frozen: true,
            identities: vec![crate::state::IdentityState {
                identity: "a@example.test".into(),
                inbox: "a".into(),
                role: crate::state::Role::Agent,
                facts: Default::default(),
            }],
            deliveries: vec![],
            events: vec![],
        };
        for stage in [
            "result.json",
            "state.json",
            "diagnostics.log",
            "metadata.json",
        ] {
            let root = tempfile::tempdir().unwrap();
<<<<<<< HEAD
            let mut injected = false;
            let r = retain_with(root.path(), &row, Some(&snap), |root, name, bytes| {
                if name == stage {
                    injected = true;
=======
            let r = retain_with(root.path(), &row, Some(&snap), |root, name, bytes| {
                if name == stage {
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
                    return Err("injected write failure".into());
                }
                publish(root, name, bytes)
            });
            assert!(r.is_err());
<<<<<<< HEAD
            assert!(injected, "{stage} injection was not reached");
            assert_eq!(r.unwrap_err(), "injected write failure");
=======
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
            assert!(
                !root
                    .path()
                    .join(&row.trial_id)
                    .join("metadata.json")
                    .exists()
            );
        }
    }
    #[test]
    fn exclusive_destinations_and_symlinked_roots_are_never_reused() {
        let dir = tempfile::tempdir().unwrap();
        let req = crate::runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("null".into()),
            ..Default::default()
        };
        let row = crate::runner::refused_row(&req, &"0".repeat(32));
        retain_trial(dir.path(), &row, None).unwrap();
        let before = read_file(
            &dir.path().join(&row.trial_id),
            "metadata.json",
            SMALL_LIMIT,
        )
        .unwrap();
        assert!(retain_trial(dir.path(), &row, None).is_err());
        assert_eq!(
            read_file(
                &dir.path().join(&row.trial_id),
                "metadata.json",
                SMALL_LIMIT
            )
            .unwrap(),
            before
        );
        let link = dir.path().join("linked");
        std::os::unix::fs::symlink(dir.path(), &link).unwrap();
        assert!(ensure_root(&link.join("must-not-create")).is_err());
        assert!(!dir.path().join("must-not-create").exists());
        assert!(read_file(dir.path(), "../metadata.json", SMALL_LIMIT).is_err());
    }
}

// The historical bare-state reader retains its nullable-field tolerance.
// New envelopes require every key while constructing the same Snapshot type.
#[derive(Deserialize)]
#[serde(remote = "crate::state::Event", deny_unknown_fields)]
struct EventWire {
    seq: u64,
    #[serde(deserialize_with = "nullable")]
    claimed: Option<String>,
    #[serde(deserialize_with = "nullable")]
    flow: Option<crate::state::Flow>,
    action: String,
    accepted: bool,
    #[serde(deserialize_with = "nullable")]
    category: Option<crate::state::FailureClass>,
}
#[derive(Deserialize)]
struct EventItem(#[serde(with = "EventWire")] crate::state::Event);
fn strict_events<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<crate::state::Event>, D::Error> {
    Ok(Vec::<EventItem>::deserialize(d)?
        .into_iter()
        .map(|e| e.0)
        .collect())
}
#[derive(Deserialize)]
#[serde(remote = "Snapshot", deny_unknown_fields)]
struct SnapshotWire {
    schema_version: u32,
    trial_id: String,
    task: String,
    frozen: bool,
    identities: Vec<crate::state::IdentityState>,
    deliveries: Vec<crate::state::Delivery>,
    #[serde(deserialize_with = "strict_events")]
    events: Vec<crate::state::Event>,
}
fn strict_snapshot(bytes: &[u8]) -> Result<Snapshot, String> {
    depth(bytes)?;
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let s = SnapshotWire::deserialize(&mut d).map_err(|_| "invalid retained snapshot")?;
    d.end().map_err(|_| "trailing snapshot data")?;
    Ok(s)
}

/// All generated matrix/trial leaves share the existing private publication primitive.
pub fn create_private_dir(root: &Path, name: &str) -> Result<PathBuf, String> {
    safe_root(root)?;
    if name != "trials" && !generated_id(name) {
        return Err("invalid_generated_identity".into());
    }
    crate::runner::create_private_artifact_dir(root, name)
}
