//! Detectable completion: metadata binds bytes and is published last.
use crate::{
    provenance::*,
    results,
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
/// Owner and permission bits of one directory entry, as the admission rule
/// for a symlinked ancestor sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ownership {
    pub(crate) uid: u32,
    pub(crate) mode: u32,
}
impl Ownership {
    fn of(m: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        Self {
            uid: m.uid(),
            mode: m.mode(),
        }
    }
    /// A group or other write bit that a sticky bit does not neutralise: in a
    /// root-owned sticky directory (`/private/tmp`) no other user can unlink
    /// or rename an entry they do not own, which is what the write-bit test
    /// stands for.
    fn replaceable(self) -> bool {
        self.mode & 0o022 != 0 && self.mode & 0o1000 == 0
    }
}
/// Whether a symlinked ancestor may be followed: the link, the directory that
/// holds it and the directory it resolves to must all be root's, and neither
/// directory may let group or other replace entries. Mode reasons come first
/// so an unprivileged test on a real filesystem reaches them.
pub(crate) fn admit_ancestor(
    link: Ownership,
    holder: Ownership,
    target: Ownership,
) -> Result<(), &'static str> {
    if holder.replaceable() {
        return Err("symlinked ancestor holder writable by group or other");
    }
    if target.replaceable() {
        return Err("symlinked ancestor target writable by group or other");
    }
    if link.uid != 0 {
        return Err("symlinked ancestor not owned by root");
    }
    if holder.uid != 0 {
        return Err("symlinked ancestor holder not owned by root");
    }
    if target.uid != 0 {
        return Err("symlinked ancestor target not owned by root");
    }
    Ok(())
}
const MAX_LINK_EXPANSIONS: usize = 40;
/// Resolve a supplied artifact or output root once, to its real path, without
/// creating anything. A symlinked ancestor is followed only when
/// `admit_ancestor` admits it (root-owned, not replaceable by group or other);
/// the root's own leaf, a `..` component and any other link refuse. Callers
/// bind the returned path and use only it afterwards; `ensure_root` and
/// `safe_root` still refuse every link they meet below it.
pub fn resolve_root(root: &Path) -> Result<PathBuf, String> {
    resolve_with(root, admit_ancestor)
}
fn resolve_with(
    root: &Path,
    admit: impl Fn(Ownership, Ownership, Ownership) -> Result<(), &'static str>,
) -> Result<PathBuf, String> {
    use std::{collections::VecDeque, ffi::OsString};
    fn names(p: &Path) -> Vec<OsString> {
        p.components().map(|c| c.as_os_str().to_owned()).collect()
    }
    // A `..` anywhere in what was supplied refuses before any link is judged.
    if root
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("parent path refused".into());
    }
    let mut queue: VecDeque<OsString> = names(root).into();
    let mut real = PathBuf::new();
    let mut expansions = 0;
    while let Some(name) = queue.pop_front() {
        if name == ".." {
            return Err("parent path refused".into());
        }
        if name == "." && !real.as_os_str().is_empty() {
            continue;
        }
        // An absolute name resets `real` to the root, as `push` does.
        let next = real.join(&name);
        match std::fs::symlink_metadata(&next) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => real = next,
            Err(_) => return Err("root unavailable".into()),
            Ok(m) if m.file_type().is_symlink() => {
                if queue.is_empty() {
                    return Err("symlink refused".into());
                }
                let holder_path = if real.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    real.as_path()
                };
                let holder = Ownership::of(
                    &std::fs::symlink_metadata(holder_path).map_err(|_| "root unavailable")?,
                );
                if holder.replaceable() {
                    return Err("symlinked ancestor holder writable by group or other".into());
                }
                let target = std::fs::metadata(&next)
                    .map_err(|_| "symlinked ancestor target unavailable")?;
                if !target.is_dir() {
                    return Err("symlinked ancestor target not a directory".into());
                }
                admit(Ownership::of(&m), holder, Ownership::of(&target))?;
                expansions += 1;
                if expansions > MAX_LINK_EXPANSIONS {
                    return Err("symlink loop refused".into());
                }
                let to = std::fs::read_link(&next)
                    .map_err(|_| "symlinked ancestor target unavailable")?;
                // The link is replaced by its target's components, each of
                // which re-enters this walk and is checked by the same rule.
                for n in names(&to).into_iter().rev() {
                    queue.push_front(n);
                }
            }
            Ok(m) if m.is_dir() => real = next,
            Ok(_) => return Err("root component is not a regular directory".into()),
        }
    }
    Ok(real)
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
    row.validate().map_err(|e| {
        if e.starts_with("schema_version ") {
            "result-version mismatch"
        } else {
            "invalid result"
        }
    })?;
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
    let record =
        results::read(&bound(root, &meta.result, "result.json", SMALL_LIMIT)?).map_err(|e| {
            if e.starts_with("schema_version ") {
                "result-version mismatch"
            } else {
                "invalid result"
            }
        })?;
    let row = record;
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
    Ok(ValidatedTrial { row, state })
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
    row.validate().map_err(|e| {
        if e.starts_with("schema_version ") {
            "result-version mismatch"
        } else {
            "invalid result"
        }
    })?;
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
    /// A temporary artifact root by its real path. These tests name real
    /// paths directly so they do not depend on the symlinked-ancestor rule
    /// they are not about (macOS's temporary directory lives under /var, a
    /// symlink to /private/var); below the root every link still refuses.
    /// It is also made 0700: `tempdir` follows the umask, and a link held in a
    /// group-writable directory would refuse for the holder's mode before the
    /// reason the test is about.
    fn artifact_root(dir: &tempfile::TempDir) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::canonicalize(dir.path()).unwrap()
    }
    #[test]
    fn every_publication_stage_refuses_without_a_completion_marker() {
        let req = crate::runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("null".into()),
            ..Default::default()
        };
        let row = crate::runner::refused_row(&req, &"0".repeat(32));
        let snap = Snapshot {
            schema_version: crate::state::SNAPSHOT_SCHEMA,
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
            let dir = tempfile::tempdir().unwrap();
            let root = artifact_root(&dir);
            let mut injected = false;
            let r = retain_with(&root, &row, Some(&snap), |root, name, bytes| {
                if name == stage {
                    injected = true;
                    return Err("injected write failure".into());
                }
                publish(root, name, bytes)
            });
            assert!(r.is_err());
            assert!(injected, "{stage} injection was not reached");
            assert_eq!(r.unwrap_err(), "injected write failure");
            assert!(!root.join(&row.trial_id).join("metadata.json").exists());
        }
    }
    #[test]
    fn exclusive_destinations_and_symlinked_roots_are_never_reused() {
        let owner = tempfile::tempdir().unwrap();
        let dir = artifact_root(&owner);
        let req = crate::runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("null".into()),
            ..Default::default()
        };
        let row = crate::runner::refused_row(&req, &"0".repeat(32));
        retain_trial(&dir, &row, None).unwrap();
        let before = read_file(&dir.join(&row.trial_id), "metadata.json", SMALL_LIMIT).unwrap();
        assert!(retain_trial(&dir, &row, None).is_err());
        assert_eq!(
            read_file(&dir.join(&row.trial_id), "metadata.json", SMALL_LIMIT).unwrap(),
            before
        );
        let link = dir.join("linked");
        std::os::unix::fs::symlink(&dir, &link).unwrap();
        assert_eq!(
            resolve_root(&link.join("must-not-create")).unwrap_err(),
            "symlinked ancestor not owned by root"
        );
        assert!(ensure_root(&link.join("must-not-create")).is_err());
        assert!(!dir.join("must-not-create").exists());
        assert!(read_file(&dir, "../metadata.json", SMALL_LIMIT).is_err());
    }
    fn own(uid: u32, mode: u32) -> Ownership {
        Ownership { uid, mode }
    }
    #[test]
    fn ancestor_admission_is_a_pure_rule_over_owner_and_mode() {
        let dir = own(0, 0o40755);
        let link = own(0, 0o120777);
        // Both accepting arms: a plain root-owned target, and a root-owned
        // sticky 1777 target such as macOS's /private/tmp.
        assert_eq!(admit_ancestor(link, dir, dir), Ok(()));
        assert_eq!(admit_ancestor(link, dir, own(0, 0o41777)), Ok(()));
        for (link, holder, target, reason) in [
            (
                link,
                own(0, 0o40775),
                dir,
                "symlinked ancestor holder writable by group or other",
            ),
            (
                link,
                own(0, 0o40757),
                dir,
                "symlinked ancestor holder writable by group or other",
            ),
            (
                link,
                dir,
                own(0, 0o40777),
                "symlinked ancestor target writable by group or other",
            ),
            (
                link,
                dir,
                own(1000, 0o40775),
                "symlinked ancestor target writable by group or other",
            ),
            (
                own(1000, 0o120777),
                dir,
                dir,
                "symlinked ancestor not owned by root",
            ),
            (
                link,
                own(1000, 0o40755),
                dir,
                "symlinked ancestor holder not owned by root",
            ),
            (
                link,
                dir,
                own(1000, 0o40755),
                "symlinked ancestor target not owned by root",
            ),
            // The sticky bit exempts only the write-bit test, never ownership.
            (
                link,
                own(1000, 0o41777),
                dir,
                "symlinked ancestor holder not owned by root",
            ),
            (
                link,
                dir,
                own(1000, 0o41777),
                "symlinked ancestor target not owned by root",
            ),
        ] {
            assert_eq!(admit_ancestor(link, holder, target), Err(reason));
        }
    }
    #[test]
    fn symlink_free_roots_resolve_to_themselves_and_create_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let base = artifact_root(&dir);
        let absent = base.join("a").join("b");
        let first = resolve_root(&absent).unwrap();
        assert_eq!(first, absent);
        assert_eq!(resolve_root(&absent).unwrap(), first);
        assert_eq!(resolve_root(&base).unwrap(), base);
        assert!(!base.join("a").exists(), "resolution creates nothing");
        assert_eq!(
            resolve_root(Path::new("relative/absent")).unwrap(),
            Path::new("relative/absent")
        );
        assert_eq!(resolve_root(Path::new(".")).unwrap(), Path::new("."));
        for refused in [
            Path::new("../x").to_path_buf(),
            base.join("..").join("x"),
            base.join("a").join("..").join("x"),
        ] {
            assert_eq!(resolve_root(&refused).unwrap_err(), "parent path refused");
        }
        let file = base.join("file");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(
            resolve_root(&file.join("child")).unwrap_err(),
            "root component is not a regular directory"
        );
        assert!(!base.join("a").exists());
    }
    fn mkdir(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir(path).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    #[test]
    fn symlinked_ancestors_resolve_only_when_root_owned_and_unswappable() {
        use std::os::unix::fs::symlink;
        let owner = tempfile::tempdir().unwrap();
        let base = artifact_root(&owner);
        let real = base.join("real");
        mkdir(&real, 0o700);
        // A same-uid link is never admitted: its owner could swap it.
        let held = base.join("held");
        symlink(&real, &held).unwrap();
        for rest in [Path::new("out"), Path::new("out/deeper")] {
            assert_eq!(
                resolve_root(&held.join(rest)).unwrap_err(),
                "symlinked ancestor not owned by root"
            );
        }
        assert!(!real.join("out").exists(), "nothing under the target");
        // A two-hop chain refuses at its first link.
        let hop = base.join("hop");
        symlink(&held, &hop).unwrap();
        assert_eq!(
            resolve_root(&hop.join("out")).unwrap_err(),
            "symlinked ancestor not owned by root"
        );
        // A link held in a directory group or other can write to.
        let shared = base.join("shared");
        mkdir(&shared, 0o770);
        symlink(&real, shared.join("link")).unwrap();
        assert_eq!(
            resolve_root(&shared.join("link").join("out")).unwrap_err(),
            "symlinked ancestor holder writable by group or other"
        );
        // A link whose target directory group or other can write to.
        let open_target = base.join("open-target");
        mkdir(&open_target, 0o770);
        symlink(&open_target, base.join("to-open")).unwrap();
        assert_eq!(
            resolve_root(&base.join("to-open").join("out")).unwrap_err(),
            "symlinked ancestor target writable by group or other"
        );
        assert!(!open_target.join("out").exists());
        // Dangling, and not a directory.
        symlink(base.join("absent"), base.join("dangling")).unwrap();
        assert_eq!(
            resolve_root(&base.join("dangling").join("out")).unwrap_err(),
            "symlinked ancestor target unavailable"
        );
        let file = base.join("file");
        std::fs::write(&file, b"x").unwrap();
        symlink(&file, base.join("to-file")).unwrap();
        assert_eq!(
            resolve_root(&base.join("to-file").join("out")).unwrap_err(),
            "symlinked ancestor target not a directory"
        );
        // The root's own leaf is never a link, whoever owns it.
        assert_eq!(resolve_root(&held).unwrap_err(), "symlink refused");
        assert_eq!(
            resolve_root(&base.join("held").join("..").join("x")).unwrap_err(),
            "parent path refused"
        );
        assert!(!base.join("x").exists());
        // Root-owned acceptance on this host, where it has such a link.
        let unique = format!(
            "swarmail-evals-probe-{}",
            base.file_name().unwrap().to_string_lossy()
        );
        let candidate = ["/tmp", "/var", "/etc", "/bin", "/sbin", "/lib", "/lib64"]
            .into_iter()
            .map(Path::new)
            .find(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()));
        let Some(link) = candidate else {
            eprintln!(
                "no root-owned symlinked ancestor exists on this host; acceptance \
                 is proven by the predicate table and the macOS tests"
            );
            return;
        };
        let probe = link.join(&unique);
        match resolve_root(&probe) {
            Ok(real) => {
                eprintln!("root-owned symlinked ancestor {link:?} accepted");
                assert_eq!(real, std::fs::canonicalize(link).unwrap().join(&unique));
            }
            Err(reason) => {
                eprintln!("symlinked ancestor {link:?} refused: {reason}");
                assert!(reason.starts_with("symlinked ancestor "), "{reason}");
            }
        }
        assert!(!probe.exists(), "resolution creates nothing");
    }
    #[test]
    fn an_admitted_link_is_replaced_by_its_target_and_checked_again() {
        use std::os::unix::fs::symlink;
        let permissive = |_: Ownership, _: Ownership, _: Ownership| Ok(());
        let owner = tempfile::tempdir().unwrap();
        let base = artifact_root(&owner);
        mkdir(&base.join("real"), 0o700);
        // Relative targets resolve against the directory holding the link.
        symlink("real", base.join("rel")).unwrap();
        assert_eq!(
            resolve_with(&base.join("rel").join("out"), permissive).unwrap(),
            base.join("real").join("out")
        );
        // Absolute targets restart at the root.
        symlink(base.join("real"), base.join("abs")).unwrap();
        assert_eq!(
            resolve_with(&base.join("abs").join("out"), permissive).unwrap(),
            base.join("real").join("out")
        );
        // A link inside a target is judged by the same rule.
        mkdir(&base.join("real").join("sub"), 0o700);
        symlink("sub", base.join("real").join("inner")).unwrap();
        symlink("real/inner", base.join("chain")).unwrap();
        assert_eq!(
            resolve_with(&base.join("chain").join("out"), permissive).unwrap(),
            base.join("real").join("sub").join("out")
        );
        let strict = |l, h, t| admit_ancestor(l, h, t);
        assert_eq!(
            resolve_with(&base.join("chain").join("out"), strict).unwrap_err(),
            "symlinked ancestor not owned by root"
        );
        // `..` inside a target refuses, and so does a leaf link.
        symlink("real/../real", base.join("dots")).unwrap();
        assert_eq!(
            resolve_with(&base.join("dots").join("out"), permissive).unwrap_err(),
            "parent path refused"
        );
        assert_eq!(
            resolve_with(&base.join("rel"), permissive).unwrap_err(),
            "symlink refused"
        );
        // Relative supplied paths resolve against the working directory.
        let held = base.join("rel-held");
        symlink("real", &held).unwrap();
        assert_eq!(
            resolve_with(Path::new("./nowhere/out"), permissive).unwrap(),
            Path::new("./nowhere/out")
        );
        // The expansion bound: 40 links resolve, 41 refuse.
        for (levels, ok) in [(40, true), (41, false)] {
            let mut at = base.join(format!("deep-{levels}"));
            mkdir(&at, 0o700);
            let mut supplied = at.clone();
            for i in 0..levels {
                let next = at.join(format!("r{i}"));
                mkdir(&next, 0o700);
                symlink(format!("r{i}"), at.join(format!("l{i}"))).unwrap();
                supplied.push(format!("l{i}"));
                at = next;
            }
            supplied.push("out");
            let r = resolve_with(&supplied, permissive);
            if ok {
                assert!(r.unwrap().ends_with("out"));
            } else {
                assert_eq!(r.unwrap_err(), "symlink loop refused");
            }
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_tmp_resolves_through_its_root_owned_link_and_retains_there() {
        use std::os::unix::fs::MetadataExt;
        let unique = format!("swarmail-evals-{}", generated_name());
        let named = Path::new("/tmp").join(&unique);
        let real = resolve_root(&named).unwrap();
        assert_eq!(real, Path::new("/private/tmp").join(&unique));
        let req = crate::runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("null".into()),
            ..Default::default()
        };
        let id = "1".repeat(32);
        let row = crate::runner::refused_row(&req, &id);
        let result = retain_trial(&real, &row, None);
        let seen = real.join(&id).join("metadata.json");
        let through = named.join(&id).join("metadata.json");
        let same = std::fs::metadata(&seen).and_then(|a| {
            std::fs::metadata(&through).map(|b| (a.dev(), a.ino()) == (b.dev(), b.ino()))
        });
        let _ = std::fs::remove_dir_all(&real);
        assert!(result.is_ok(), "retain under {real:?}: {result:?}");
        assert!(
            same.unwrap(),
            "same file through both spellings of {real:?}"
        );
    }
    #[cfg(target_os = "macos")]
    fn generated_name() -> String {
        let dir = tempfile::tempdir().unwrap();
        dir.path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
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
