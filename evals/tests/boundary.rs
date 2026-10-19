//! The crate boundary (operator ruling 2026-10-18; decision 0055): evals
//! is its own workspace, outside the published crate, and every package in
//! its lock is already in the root lock at the same version and source.

use std::collections::HashSet;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// (name, version, source) for every `[[package]]` in a Cargo.lock.
fn packages(lock: &str) -> HashSet<(String, String, String)> {
    lock.split("[[package]]")
        .skip(1)
        .map(|block| {
            let field = |k: &str| {
                block
                    .lines()
                    .find_map(|l| l.strip_prefix(&format!("{k} = ")))
                    .map(|v| v.trim_matches('"').to_string())
                    .unwrap_or_default()
            };
            (field("name"), field("version"), field("source"))
        })
        .collect()
}

#[test]
fn every_evals_package_is_already_in_the_root_lock() {
    let ours = packages(&std::fs::read_to_string(root().join("evals/Cargo.lock")).unwrap());
    let theirs = packages(&std::fs::read_to_string(root().join("Cargo.lock")).unwrap());
    let foreign: Vec<_> = ours
        .iter()
        .filter(|p| p.0 != "swarmail-evals" && !theirs.contains(*p))
        .collect();
    assert!(
        foreign.is_empty(),
        "packages outside the root lock: {foreign:?}"
    );
    assert!(ours.iter().all(|p| p.0 != "native-tls" && p.0 != "openssl"));
    assert!(ours.iter().any(|p| p.0 == "swarmail-evals"));
}

#[test]
fn evals_is_its_own_unpublished_workspace_and_the_root_does_not_package_it() {
    let manifest = std::fs::read_to_string(root().join("evals/Cargo.toml")).unwrap();
    assert!(manifest.lines().any(|l| l.trim() == "[workspace]"));
    assert!(manifest.contains("publish = false"));
    let root_manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(
        !root_manifest.contains("evals"),
        "the root manifest names nothing of evals"
    );
}
