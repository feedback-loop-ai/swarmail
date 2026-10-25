//! Build context belongs to this executable, never its runtime directory.
use std::path::{Path, PathBuf};
mod git_context;
fn git(root: &Path, args: &[&str]) -> Option<String> {
    String::from_utf8(git_context::git(root, args)?)
        .ok()
        .map(|s| s.trim().to_string())
}
fn watch(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir()
        && let Ok(entries) = std::fs::read_dir(path)
    {
        for e in entries.flatten() {
            let p = e.path();
            if !matches!(
                e.file_name().to_str(),
                Some("target" | ".forge" | ".brokkr" | ".git")
            ) {
                watch(&p);
            }
        }
    }
}
fn main() {
    let dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let root = dir.parent().expect("evals parent");
    for name in [
        "src",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "git_context.rs",
    ] {
        watch(&dir.join(name));
    }
    for name in ["src", "Cargo.toml", "Cargo.lock"] {
        watch(&root.join(name));
    }
    println!("cargo:rerun-if-changed={}", root.join(".git").display());
    if let Some(files) = git(
        root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    ) {
        let mut directories = std::collections::BTreeSet::new();
        for f in files.split('\0').filter(|f| !f.is_empty()) {
            let p = root.join(f);
            println!("cargo:rerun-if-changed={}", p.display());
            let mut parent = p.parent();
            while let Some(p) = parent {
                if !p.starts_with(root) {
                    break;
                }
                directories.insert(p.to_path_buf());
                parent = p.parent();
            }
        }
        for p in directories {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
    if let Some(path) = git(root, &["rev-parse", "--absolute-git-dir"]) {
        let path = PathBuf::from(path);
        for name in ["HEAD", "index", "packed-refs", "refs"] {
            watch(&path.join(name));
        }
    }
    if let Some(path) = git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ) {
        let path = PathBuf::from(path);
        for name in ["HEAD", "packed-refs", "refs"] {
            watch(&path.join(name));
        }
    }
    let commit = git(root, &["rev-parse", "--verify", "HEAD"]).unwrap_or_default();
    let dirty = git(root, &["status", "--porcelain", "--untracked-files=all"])
        .map(|s| (!s.is_empty()).to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=EVAL_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=EVAL_BUILD_DIRTY={dirty}");
    println!("cargo:rerun-if-env-changed=PATH");
}
