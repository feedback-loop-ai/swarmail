//! Measured executable bytes and runner-owned source associations.
use crate::provenance::*;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub fn file_hash(path: &Path) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|_| "executable not readable")?;
    if !f.metadata().map_err(|_| "executable metadata")?.is_file() {
        return Err("executable not regular".into());
    }
    let mut h = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buf = [0; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|_| "executable read")?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(h.finish().as_ref()))
}
pub fn measured(path: &Path) -> Availability<String> {
    file_hash(path)
        .map(|h| Availability::present(h, Source::MeasuredBytes))
        .unwrap_or_else(|_| Availability::unavailable(Reason::NotReadable))
}
pub fn harness() -> HarnessIdentity {
    HarnessIdentity {
        package: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        executable_sha256: std::env::current_exe()
            .map(|p| measured(&p))
            .unwrap_or_else(|_| Availability::unavailable(Reason::NotReadable)),
        build_commit: if env!("EVAL_BUILD_COMMIT").is_empty() {
            Availability::unavailable(Reason::NoGit)
        } else {
            Availability::present(env!("EVAL_BUILD_COMMIT").into(), Source::BuildGit)
        },
        source_dirty: env!("EVAL_BUILD_DIRTY")
            .parse()
            .map(|v| Availability::present(v, Source::BuildGit))
            .unwrap_or_else(|_| Availability::unavailable(Reason::NoGit)),
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TargetBuild {
    pub path: PathBuf,
    pub identity: ExecutableIdentity,
}
impl TargetBuild {
    pub fn external(path: PathBuf) -> Self {
        Self {
            identity: ExecutableIdentity {
                executable_sha256: measured(&path),
                build_commit: Availability::unavailable(Reason::ExternalSourceUnknown),
                source_dirty: Availability::unavailable(Reason::ExternalSourceUnknown),
            },
            path,
        }
    }
    pub fn recheck(&self) -> Result<(), String> {
        match &self.identity.executable_sha256 {
            Availability::Present { value, .. } if file_hash(&self.path)? == *value => Ok(()),
            _ => Err("executable identity changed/unavailable".into()),
        }
    }
}
#[derive(PartialEq)]
pub struct SourceContext {
    commit: Option<String>,
    dirty: Option<bool>,
    fingerprint: String,
}
impl SourceContext {
    pub fn capture(root: &Path) -> Result<Self, String> {
        crate::runner::inspect_source(root)
    }
    pub(crate) fn with_git(
        root: &Path,
        commit: Option<&[u8]>,
        dirty: Option<&[u8]>,
    ) -> Result<Self, String> {
        let commit = commit
            .and_then(|b| std::str::from_utf8(b).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| {
                (s.len() == 40 || s.len() == 64)
                    && s.bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            });
        let dirty = dirty.map(|b| !b.is_empty());
        fn collect(
            root: &Path,
            path: &Path,
            out: &mut Vec<(String, String)>,
        ) -> Result<(), String> {
            if path.is_dir() {
                let entries =
                    std::fs::read_dir(path).map_err(|_| "source directory unavailable")?;
                for e in entries {
                    collect(
                        root,
                        &e.map_err(|_| "source entry unavailable")?.path(),
                        out,
                    )?;
                }
            } else {
                out.push((
                    path.strip_prefix(root)
                        .map_err(|_| "source path")?
                        .to_string_lossy()
                        .into_owned(),
                    file_hash(path)?,
                ));
            }
            Ok(())
        }
        let mut entries = Vec::new();
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            "src",
            "build.rs",
            ".cargo",
            "rust-toolchain.toml",
        ] {
            if !root.join(name).exists() {
                continue;
            }
            collect(root, &root.join(name), &mut entries)?;
        }
        entries.sort();
        Ok(Self {
            commit,
            dirty,
            fingerprint: sha256(&serde_json::to_vec(&entries).expect("source entries")),
        })
    }
    pub fn associate(&self, path: PathBuf) -> TargetBuild {
        TargetBuild {
            identity: ExecutableIdentity {
                executable_sha256: measured(&path),
                build_commit: self
                    .commit
                    .clone()
                    .map(|v| Availability::present(v, Source::OwnedBuildContext))
                    .unwrap_or_else(|| Availability::unavailable(Reason::NoGit)),
                source_dirty: self
                    .dirty
                    .map(|v| Availability::present(v, Source::OwnedBuildContext))
                    .unwrap_or_else(|| Availability::unavailable(Reason::NoGit)),
            },
            path,
        }
    }
}

/// The comparison variant's schema number. Native rows are schema 2, the
/// legacy row is 1, and no reader accepts a number it does not own.
pub const COMPARISON_SCHEMA: u32 = 3;
const COMPARISON_KIND: &str = "comparison";
/// The agent targets that exist. MailHog has no agent adapter, so its cells
/// are refused by name and never invented.
const COMPARISON_TARGETS: [(&str, &str); 2] = [
    ("swarmail-mcp", "native-mcp"),
    ("mailpit-mcp", "stdio-adapter"),
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonSettings {
    /// How the agent reaches the target; it must be the one the target has.
    pub connection: String,
    /// SHA-256 of the tool list the agent was offered.
    pub tools_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonCoordinate {
    pub task: String,
    pub attempt: u32,
}

/// One agent-side comparison trial: identity (target and measured
/// executable), settings, coordinate and a hash over all of them. It is a
/// distinct schema, so the native reader refuses it and this reader refuses
/// every native or legacy row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRow {
    pub schema_version: u32,
    pub kind: String,
    pub protocol_sha256: String,
    pub target: String,
    pub executable_sha256: String,
    pub settings: ComparisonSettings,
    pub coordinate: ComparisonCoordinate,
    pub outcome: crate::tasks::Outcome,
    pub hash: String,
}
impl ComparisonRow {
    /// The hash is over the canonical serialization with the hash field empty.
    pub fn digest(&self) -> String {
        let mut body = self.clone();
        body.hash = String::new();
        sha256(&serde_json::to_vec(&body).expect("comparison row"))
    }
    pub fn seal(mut self) -> Self {
        self.hash = self.digest();
        self
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMPARISON_SCHEMA || self.kind != COMPARISON_KIND {
            return Err(format!("not a schema {COMPARISON_SCHEMA} comparison row"));
        }
        if self.target == "mailhog" {
            return Err("mailhog agent cells are unsupported".into());
        }
        let Some((_, connection)) = COMPARISON_TARGETS.iter().find(|(t, _)| *t == self.target)
        else {
            return Err("unknown comparison target".into());
        };
        if self.settings.connection != *connection {
            return Err("settings connection does not match the target".into());
        }
        if !is_hash(&self.protocol_sha256)
            || !is_hash(&self.executable_sha256)
            || !is_hash(&self.settings.tools_sha256)
        {
            return Err("identity or settings digest is not a SHA-256".into());
        }
        if self.coordinate.task.is_empty() || self.coordinate.attempt == 0 {
            return Err("coordinate needs a task and an attempt from 1".into());
        }
        if self.hash != self.digest() {
            return Err("row hash does not match its contents".into());
        }
        Ok(())
    }
}
