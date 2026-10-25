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

#[test]
fn sequential_contracts_and_decision_links_are_honest() {
    for relative in [
        "evals/README.md",
        "evals/run-plan.md",
        "specs/agent-eval-harness/spec.md",
        "docs/decisions/0055-agent-eval-harness.md",
    ] {
        let path = root().join(relative);
        let text = std::fs::read_to_string(&path).unwrap();
        for phrase in [
            "baseline runs sequentially",
            "fresh HOME",
            "concurrency",
            "Concurrent trials are not isolated",
            "same-uid",
            "shared TMPDIR",
            "do-nothing agent passes",
            "MCP URL, app URL, identity and inbox context",
            "#201",
            "#221",
        ] {
            assert!(text.contains(phrase), "{relative}: missing {phrase}");
        }
        // Check bounded local Markdown links; anchors and external links need no filesystem lookup.
        for suffix in text.split("](").skip(1) {
            let target = suffix.split(')').next().unwrap().split('#').next().unwrap();
            if !target.is_empty() && !target.contains("://") {
                assert!(
                    path.parent().unwrap().join(target).exists(),
                    "{relative}: broken link {target}"
                );
            }
        }
    }
    let spec = std::fs::read_to_string(root().join("specs/agent-eval-harness/spec.md")).unwrap();
    assert!(spec.contains("AC4 — Positive control"));
    assert!(!spec.contains("Two simultaneously active isolation trials"));
    let index = std::fs::read_to_string(root().join("docs/decisions/README.md")).unwrap();
    let rows: Vec<_> = index
        .lines()
        .filter(|line| line.starts_with("| [0076]"))
        .collect();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].contains("proposed"));
    let decision =
        std::fs::read_to_string(root().join("docs/decisions/0076-sequential-eval-trials.md"))
            .unwrap();
    assert!(decision.contains("**Status:** proposed"));
    assert!(
        index
            .lines()
            .any(|line| line.starts_with("| [0063]") && line.contains("proposed"))
    );
    assert!(
        index
            .lines()
            .any(|line| line.starts_with("| [0071]") && line.contains("accepted"))
    );
    assert_eq!(
        index
            .lines()
            .filter(|line| line.starts_with("| [0077]"))
            .count(),
        1
    );
    let plan = std::fs::read_to_string(root().join("specs/agent-eval-harness/plan.md")).unwrap();
    assert!(!plan.contains("two concurrently active isolation trials"));
}
