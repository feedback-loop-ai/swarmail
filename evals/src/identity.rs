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
