//! Private, exclusive, append-only run output (spec R4). A run directory
//! is created fresh (0700), every artifact is created new (0600) and never
//! rewritten, the manifest lands before any launch, sample rows stream as
//! they happen, and the completion index is written only after cleanup is
//! proved and every row validates. A directory without an index is an
//! incomplete run, and nothing here can turn it into a complete one.

use crate::protocol::{TargetKind, is_hex, is_name};
use crate::targets::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub const ROW_VERSION: u32 = 1;
/// The resource rows of protocol 4 (`sample` and `window`).
pub const RESOURCE_ROW_VERSION: u32 = 1;
pub const MANIFEST_VERSION: u32 = 1;
pub const INDEX_VERSION: u32 = 1;
/// A sample row longer than this is malformed on read.
pub const MAX_ROW_BYTES: usize = 4096;
/// The frozen protocol's full collection writes about 270 MiB of samples
/// (757,464 rows on 2026-10-29), so the bound leaves room above that.
pub const MAX_ARTIFACT_BYTES: u64 = 512 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Send,
    Visible,
    Verified,
    Present,
    Future,
    Empty,
}

/// One typed measurement. Absent events stay absent (`null`) with a
/// finite reason; nothing is filled in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub row_version: u32,
    pub run_id: String,
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub phase: Phase,
    pub sample: u32,
    pub token: Option<String>,
    /// accepted | rejected | failed_before_terminator | ambiguous for sends;
    /// observed | late | timeout | error | unrelated for waits; visible | verified
    /// for verification.
    pub outcome: String,
    pub reason: Option<String>,
    pub code: Option<u16>,
    pub invoked_ns: Option<u64>,
    pub start_ns: Option<u64>,
    pub ack_ns: Option<u64>,
    pub observed_ns: Option<u64>,
    pub occurrences: Option<u64>,
    pub fidelity_ok: Option<bool>,
    pub requests: Option<u64>,
}

impl Row {
    pub fn coordinate(&self) -> (TargetKind, String, u32, Phase, u32) {
        (
            self.target,
            self.profile.clone(),
            self.repetition,
            self.phase,
            self.sample,
        )
    }
}

/// The plan a manifest declares before launch: how many rows of each
/// coordinate family a complete run holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Planned {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub phase: Phase,
    /// Exact row count, or `None` where the count is the observation
    /// itself (visible rows: one per id the server showed).
    pub rows: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub manifest_version: u32,
    pub run_id: String,
    /// `smoke` runs are labelled as such everywhere and never evidence.
    pub mode: String,
    pub protocol_id: String,
    pub protocol_sha256: String,
    pub sources_sha256: String,
    pub candidate_commit: Option<String>,
    pub candidate_dirty: Option<bool>,
    pub os: String,
    pub arch: String,
    pub kernel: String,
    pub cpus: u32,
    pub prelaunch_utc: String,
    pub planned: Vec<Planned>,
    /// Protocol 4 only (absent from every earlier manifest's bytes): the
    /// chunk's coordinate, `cost.<target>.r<repetition>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk: Option<String>,
    /// Protocol 4 only: the host the chunk ran on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<HostFacts>,
    /// Protocol 4 only: every window the chunk will produce. Its presence is
    /// what makes a run directory a resource run: only then are resource
    /// rows read, and a protocol 1 to 3 directory refuses them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planned_windows: Option<Vec<PlannedWindow>>,
}

/// What the run records about its host (0132 §12, `host.json`): enough to
/// see that every chunk ran on the same machine and the same CPUs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostFacts {
    pub kernel: String,
    /// `systemctl --version`'s first line, as the wrapper passed it.
    pub systemd: String,
    pub cpu_model: String,
    pub nproc: u32,
    /// The thread siblings of each target core, as the kernel lists them.
    pub siblings: Vec<Vec<u32>>,
    pub cpu_layout: crate::resources::Layout,
    pub governor: String,
}

/// What a window measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowKind {
    /// Leaf creation to readiness: the CPU time to ready.
    Ready,
    /// Five seconds without traffic after readiness.
    IdleReady,
    /// The profile's send-and-verify window.
    Measure,
}

/// How a window ended. `OomKilled` is a rise of `oom_kill` over the window
/// (`max` alone is pressure, not an OOM); both it and `TargetExited` are
/// measured outcomes of the cell, never retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowOutcome {
    Measured,
    OomKilled,
    TargetExited,
    ChunkDeadline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedWindow {
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub window: WindowKind,
}

/// One reading of a leaf: `memory.current`, the `memory.stat` split and
/// `pids.current`, at `t_ns` on the run's clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleRow {
    pub row_version: u32,
    pub run_id: String,
    /// The leaf's name: one per launch.
    pub launch: String,
    pub target: TargetKind,
    pub t_ns: u64,
    pub memory_current: u64,
    pub anon: u64,
    pub file: u64,
    pub kernel: u64,
    pub pids: u64,
}

/// One measurement window: `cpu.stat` and `memory.events` at both bounds, and
/// whether every task in the leaf still had exactly the two target CPUs at
/// the end. Peak and mean memory are not stored: the report derives them
/// from the `sample` rows inside the bounds, so there is one source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowRow {
    pub row_version: u32,
    pub run_id: String,
    pub launch: String,
    pub target: TargetKind,
    pub profile: String,
    pub repetition: u32,
    pub window: WindowKind,
    pub start_ns: u64,
    pub end_ns: u64,
    pub cpu_start: crate::resources::CpuStat,
    pub cpu_end: crate::resources::CpuStat,
    pub events_start: crate::resources::MemEvents,
    pub events_end: crate::resources::MemEvents,
    pub affinity_held: bool,
    /// The scratch tree's allocated bytes (`st_blocks × 512`), at the end of
    /// an `idle_ready` window.
    pub scratch_bytes: Option<u64>,
    pub outcome: WindowOutcome,
}

/// The second row family of a protocol 4 `samples.jsonl`, told apart from a
/// message row by its `kind` key, which no message row has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResourceRow {
    Sample(SampleRow),
    Window(WindowRow),
}

pub struct RunDir {
    path: PathBuf,
    samples: File,
    run_id: String,
}

fn create_new(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

impl RunDir {
    /// Create `path` (its parent must exist; `path` must not) and write the
    /// manifest before anything launches.
    pub fn create(path: &Path, manifest: &Manifest) -> std::io::Result<RunDir> {
        std::fs::DirBuilder::new().mode(0o700).create(path)?;
        let mut m = create_new(&path.join("manifest.json"))?;
        m.write_all(&serde_json::to_vec_pretty(manifest).expect("manifest serializes"))?;
        m.write_all(b"\n")?;
        let samples = create_new(&path.join("samples.jsonl"))?;
        Ok(RunDir {
            path: path.to_owned(),
            samples,
            run_id: manifest.run_id.clone(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Stream one row; a write failure is an operational failure the
    /// caller must stop on.
    pub fn append(&mut self, row: &Row) -> std::io::Result<()> {
        let mut line = serde_json::to_vec(row).expect("row serializes");
        line.push(b'\n');
        self.samples.write_all(&line)?;
        Ok(())
    }

    /// Stream one resource row (protocol 4).
    pub fn append_resource(&mut self, row: &ResourceRow) -> std::io::Result<()> {
        let mut line = serde_json::to_vec(row).expect("row serializes");
        line.push(b'\n');
        self.samples.write_all(&line)?;
        Ok(())
    }

    /// Write a derived artifact (the report) as a new file.
    pub fn write_artifact(&self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        let mut f = create_new(&self.path.join(name))?;
        f.write_all(bytes)
    }

    /// Seal: only when cleanup is proved and the directory admits as
    /// complete does the index appear. Otherwise the directory stays as
    /// written — an incomplete run with its prefix intact.
    pub fn finalize(mut self, cleanup_proved: bool) -> Result<PathBuf, String> {
        self.samples
            .flush()
            .map_err(|_| "samples flush failed".to_owned())?;
        if !cleanup_proved {
            return Err("cleanup is not proved; the run stays incomplete".into());
        }
        let files = artifact_listing(&self.path)?;
        let index = Index {
            index_version: INDEX_VERSION,
            run_id: self.run_id.clone(),
            artifacts: files,
        };
        validate_contents(&self.path, &index)?;
        let mut f = create_new(&self.path.join("index.json")).map_err(|e| format!("index: {e}"))?;
        f.write_all(&serde_json::to_vec_pretty(&index).expect("index serializes"))
            .map_err(|e| format!("index: {e}"))?;
        Ok(self.path.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub index_version: u32,
    pub run_id: String,
    /// name → (sha256, length)
    pub artifacts: BTreeMap<String, (String, u64)>,
}

fn artifact_listing(dir: &Path) -> Result<BTreeMap<String, (String, u64)>, String> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read run dir: {e}"))? {
        let entry = entry.map_err(|e| format!("read run dir: {e}"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 artifact name".to_owned())?;
        if name == "index.json" {
            continue;
        }
        let meta = std::fs::symlink_metadata(entry.path()).map_err(|e| format!("{name}: {e}"))?;
        if !meta.is_file() || !is_name(&name) {
            return Err(format!("{name}: not a plain, well-named file"));
        }
        if meta.len() > MAX_ARTIFACT_BYTES {
            return Err(format!("{name}: larger than the artifact bound"));
        }
        let bytes = std::fs::read(entry.path()).map_err(|e| format!("{name}: {e}"))?;
        out.insert(name, (sha256_hex(&bytes), bytes.len() as u64));
    }
    Ok(out)
}

/// What [`admit`] proves about a sealed run.
#[derive(Debug)]
pub struct Admitted {
    pub manifest: Manifest,
    pub rows: Vec<Row>,
    /// Protocol 4's `sample` and `window` rows; empty for every earlier run.
    #[allow(dead_code)] // read by admit-v4, which the next group adds
    pub resources: Vec<ResourceRow>,
}

pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let bytes = std::fs::read(dir.join("manifest.json")).map_err(|e| format!("manifest: {e}"))?;
    let m: Manifest = serde_json::from_slice(&bytes).map_err(|e| format!("manifest: {e}"))?;
    if m.manifest_version != MANIFEST_VERSION || !is_name(&m.run_id) {
        return Err("manifest: unknown version or malformed run id".into());
    }
    if !is_hex(&m.protocol_sha256, 64) || !is_hex(&m.sources_sha256, 64) {
        return Err("manifest: protocol and sources digests must be SHA-256".into());
    }
    if !matches!(m.mode.as_str(), "smoke" | "collect") {
        return Err(format!("manifest: unknown mode {:?}", m.mode));
    }
    Ok(m)
}

/// Strictly read every row of `samples.jsonl` against its manifest:
/// truncated, oversized, malformed, foreign-run, duplicate, unplanned and
/// missing rows all refuse. Used before sealing and again on admission.
/// A directory whose manifest plans no windows carries message rows only,
/// and a resource row in it refuses.
pub fn read_rows(dir: &Path, m: &Manifest) -> Result<Vec<Row>, String> {
    read_run(dir, m).map(|(rows, _)| rows)
}

/// [`read_rows`], and the resource rows of a protocol 4 directory: the
/// `sample` rows of its leaves and the `window` rows it planned, each
/// planned window exactly once.
pub fn read_run(dir: &Path, m: &Manifest) -> Result<(Vec<Row>, Vec<ResourceRow>), String> {
    let bytes = std::fs::read(dir.join("samples.jsonl")).map_err(|e| format!("samples: {e}"))?;
    if !bytes.is_empty() && bytes.last() != Some(&b'\n') {
        return Err("samples: truncated final row".into());
    }
    let mut rows = Vec::new();
    let mut resources = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (n, line) in bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        if line.len() > MAX_ROW_BYTES {
            return Err(format!("samples: row {n} exceeds {MAX_ROW_BYTES} bytes"));
        }
        if m.planned_windows.is_some() && has_kind(line) {
            let row: ResourceRow =
                serde_json::from_slice(line).map_err(|e| format!("samples: row {n}: {e}"))?;
            let (version, run) = match &row {
                ResourceRow::Sample(s) => (s.row_version, &s.run_id),
                ResourceRow::Window(w) => (w.row_version, &w.run_id),
            };
            if version != RESOURCE_ROW_VERSION {
                return Err(format!("samples: row {n} has version {version}"));
            }
            if *run != m.run_id {
                return Err(format!("samples: row {n} belongs to another run"));
            }
            resources.push(row);
            continue;
        }
        let row: Row =
            serde_json::from_slice(line).map_err(|e| format!("samples: row {n}: {e}"))?;
        if row.row_version != ROW_VERSION {
            return Err(format!("samples: row {n} has version {}", row.row_version));
        }
        if row.run_id != m.run_id {
            return Err(format!("samples: row {n} belongs to another run"));
        }
        if !seen.insert(row.coordinate()) {
            return Err(format!("samples: row {n} duplicates a coordinate"));
        }
        rows.push(row);
    }
    let mut counts: BTreeMap<(TargetKind, &str, u32, Phase), u32> = BTreeMap::new();
    for r in &rows {
        let planned = m.planned.iter().any(|p| {
            p.target == r.target
                && p.profile == r.profile
                && p.repetition == r.repetition
                && p.phase == r.phase
        });
        if !planned {
            return Err(format!(
                "samples: unplanned row {} {} rep {} {:?}",
                r.target, r.profile, r.repetition, r.phase
            ));
        }
        *counts
            .entry((r.target, &r.profile, r.repetition, r.phase))
            .or_default() += 1;
    }
    for p in &m.planned {
        let got = counts
            .get(&(p.target, p.profile.as_str(), p.repetition, p.phase))
            .copied()
            .unwrap_or(0);
        if p.rows.is_some_and(|want| want != got) {
            return Err(format!(
                "samples: {} {} rep {} {:?} has {got} rows, {} planned",
                p.target,
                p.profile,
                p.repetition,
                p.phase,
                p.rows.unwrap_or_default()
            ));
        }
    }
    if let Some(planned) = &m.planned_windows {
        check_windows(planned, &resources)?;
    }
    Ok((rows, resources))
}

/// Whether a row line carries the `kind` key of a resource row.
fn has_kind(line: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(line)
        .map(|v| v.get("kind").is_some())
        .unwrap_or(false)
}

/// Each planned window exactly once, and none that was not planned.
fn check_windows(planned: &[PlannedWindow], rows: &[ResourceRow]) -> Result<(), String> {
    let mut got: BTreeMap<(TargetKind, String, u32, WindowKind), u32> = BTreeMap::new();
    for r in rows {
        if let ResourceRow::Window(w) = r {
            let known = planned.iter().any(|p| {
                p.target == w.target
                    && p.profile == w.profile
                    && p.repetition == w.repetition
                    && p.window == w.window
            });
            if !known {
                return Err(format!(
                    "samples: unplanned window {} {} rep {} {:?}",
                    w.target, w.profile, w.repetition, w.window
                ));
            }
            *got.entry((w.target, w.profile.clone(), w.repetition, w.window))
                .or_default() += 1;
        }
    }
    for p in planned {
        match got
            .get(&(p.target, p.profile.clone(), p.repetition, p.window))
            .copied()
            .unwrap_or(0)
        {
            1 => {}
            0 => {
                return Err(format!(
                    "samples: window {} {} rep {} {:?} is missing",
                    p.target, p.profile, p.repetition, p.window
                ));
            }
            n => {
                return Err(format!(
                    "samples: window {} {} rep {} {:?} appears {n} times",
                    p.target, p.profile, p.repetition, p.window
                ));
            }
        }
    }
    Ok(())
}

fn validate_contents(dir: &Path, index: &Index) -> Result<(), String> {
    for required in ["manifest.json", "samples.jsonl", "report.json"] {
        if !index.artifacts.contains_key(required) {
            return Err(format!("{required} is missing"));
        }
    }
    let m = read_manifest(dir)?;
    if m.run_id != index.run_id {
        return Err("index and manifest name different runs".into());
    }
    read_run(dir, &m).map(|_| ())
}

/// Admit a sealed run: the index must exist, list exactly the files
/// present with their digests and lengths, and every row must validate.
/// A directory without an index is incomplete and refuses.
pub fn admit(dir: &Path) -> Result<Admitted, String> {
    let bytes = std::fs::read(dir.join("index.json"))
        .map_err(|_| "no completion index: the run is incomplete".to_owned())?;
    let index: Index = serde_json::from_slice(&bytes).map_err(|e| format!("index: {e}"))?;
    if index.index_version != INDEX_VERSION {
        return Err("index: unknown version".into());
    }
    let present = artifact_listing(dir)?;
    if present != index.artifacts {
        return Err(
            "index does not match the files present (missing, extra or altered artifact)".into(),
        );
    }
    validate_contents(dir, &index)?;
    let manifest = read_manifest(dir)?;
    let (rows, resources) = read_run(dir, &manifest)?;
    Ok(Admitted {
        manifest,
        rows,
        resources,
    })
}
