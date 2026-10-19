//! Structural pins on the delivery constitution (`bundles/self/`), proposed
//! decision 0050. These read the committed JSON and scripts; they are
//! regression guards on the shape the repairs gave the constitution, not a
//! substitute for the pinned engine's own evaluation (0050 §5) — the verify
//! seat compiles the bundle with the pinned brokkr and refuses its
//! policy-sweep warning.

use std::path::Path;

use serde_json::{Value, json};

fn read_json(path: &str) -> Value {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|e| panic!("{path} is unreadable: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} is not JSON: {e}"))
}

fn read_text(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|e| panic!("{path} is unreadable: {e}"))
}

fn rules_from(policy: &Value, phase: &str, result: &str) -> Vec<Value> {
    policy["rules"]
        .as_array()
        .expect("policy.rules is an array")
        .iter()
        .filter(|rule| rule["from"] == phase && rule["result"] == result)
        .cloned()
        .collect()
}

/// #125: the residual allow rule requires both explicit safe facts, the
/// above-medium hard stop precedes it, and a final unconditional hard stop
/// catches security residuals and absent, null or malformed facts alike. At
/// 2a325c9 the allow rule had no `when` at all and nothing followed it. A
/// separate security rule before the allow would leave the final stop firing
/// on no present valuation, which the pinned engine refuses as dead.
#[test]
fn residual_admission_requires_explicit_safe_facts_and_unknowns_stop() {
    let policy = read_json("bundles/self/policy.json");
    let residual = rules_from(&policy, "review", "residual");
    let ids: Vec<&str> = residual.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "REVIEW-RESIDUAL-ABOVE-MEDIUM",
            "REVIEW-RESIDUAL-OK",
            "REVIEW-RESIDUAL-UNSAFE"
        ]
    );
    assert_eq!(
        residual[0]["when"],
        json!({"max_residual_severity_above": "medium"})
    );
    assert_eq!(
        residual[1]["when"],
        json!({"has_security_residual": false, "max_residual_severity_at_most": "medium"})
    );
    assert_eq!(residual[1]["next"], "ship");
    assert_eq!(residual[2].get("when"), None);
    for stop in [&residual[0], &residual[2]] {
        assert_eq!(
            (stop["next"].as_str(), stop["severity"].as_str()),
            (Some("stop"), Some("hard"))
        );
    }
    let hold = rules_from(&policy, "review", "security-hold");
    assert_eq!(hold.len(), 1);
    assert_eq!(
        (hold[0]["next"].as_str(), hold[0]["severity"].as_str()),
        (Some("stop"), Some("hard"))
    );
}

/// #124: only the deterministic check's `designed` advances design; a
/// phase-level `drafted` is a hard stop (at 2a325c9 DESIGN-OK-DRAFTED sent it
/// to implement); one failure retries and the second stops.
#[test]
fn only_the_design_check_advances_to_implement() {
    let policy = read_json("bundles/self/policy.json");
    let to_implement: Vec<String> = policy["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|rule| rule["from"] == "design" && rule["next"] == "implement")
        .map(|rule| format!("{} on {}", rule["id"], rule["result"]))
        .collect();
    assert_eq!(to_implement, ["\"DESIGN-OK\" on \"designed\""]);
    let drafted = rules_from(&policy, "design", "drafted");
    assert_eq!(drafted.len(), 1);
    assert_eq!(
        (drafted[0]["next"].as_str(), drafted[0]["severity"].as_str()),
        (Some("stop"), Some("hard"))
    );
    let fail = rules_from(&policy, "design", "fail");
    let fail_ids: Vec<&str> = fail.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(fail_ids, ["DESIGN-FAIL-TWICE", "DESIGN-RETRY"]);
    assert_eq!(fail[0]["when"], json!({"consecutive_failures_gte": 2}));
    assert_eq!(fail[0]["next"], "stop");
    assert_eq!(fail[1]["next"], "design");
}

/// #124: the design sequence ends positions -> chief -> check, the chief
/// returns only `drafted`, and the final step is the pinned exec validator.
#[test]
fn the_design_sequence_ends_in_the_pinned_exec_check() {
    let bundle = read_json("bundles/self/bundle.json");
    let sequence = bundle["seats"]["design"]["sequence"].as_array().unwrap();
    let names: Vec<&str> = sequence
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["positions", "chief", "check"]);
    assert_eq!(sequence[1]["results"], json!(["drafted"]));
    let check = &sequence[2];
    assert_eq!(check["class"], "gate");
    assert_eq!(
        check.get("results"),
        None,
        "the final step receives the seat's results"
    );
    assert_eq!(
        check["driver"]["command"],
        json!([
            "{brokkr}",
            "driver",
            "exec",
            "--",
            "bash",
            "./scripts/design-check.sh",
            "{prompt_file}"
        ])
    );
    assert_eq!(check["hands"]["network"], false);
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bundles/self/scripts/design-check.sh")
            .is_file()
    );
}

/// #123: the verify seat runs every gate from its own pinned directory and
/// never invokes a candidate copy; the public entry points are only the fixed
/// shims the pinned digests name.
#[test]
fn verify_runs_the_pinned_gates_and_checks_the_public_shims() {
    let seat = read_text("bundles/self/scripts/verify-seat.sh");
    for gate in [
        "\"bash $pin_q/coverage-gate.sh\"",
        "\"bash $pin_q/coverage-gate.sh selftest\"",
        "\"bash $pin_q/design-check.sh selftest\"",
        "\"bash $pin_q/publish-guard-test.sh\"",
    ] {
        assert!(seat.contains(gate), "verify-seat.sh does not run {gate}");
    }
    for candidate_copy in ["bash scripts/", "tools/coverage-fn-scan"] {
        assert!(
            !seat.contains(candidate_copy),
            "verify-seat.sh still runs {candidate_copy}"
        );
    }
    assert!(seat.contains("sha256sum --check --strict \"$pin_dir/shims.sha256\""));
    let manifest = read_text("bundles/self/scripts/shims.sha256");
    let listed: Vec<&str> = manifest
        .lines()
        .map(|l| l.split_once("  ").unwrap().1)
        .collect();
    assert_eq!(
        listed,
        [
            "scripts/coverage-gate.sh",
            "scripts/publish-guard.sh",
            "scripts/publish-guard-test.sh",
            "scripts/design-check.sh",
            ".specify/scripts/bash/check-prerequisites.sh"
        ]
    );
    for shim in &listed {
        let text = read_text(shim);
        let target = shim
            .rsplit('/')
            .next()
            .map(|name| {
                if name == "check-prerequisites.sh" {
                    "speckit/check-prerequisites.sh".to_owned()
                } else {
                    name.to_owned()
                }
            })
            .unwrap();
        assert!(
            text.contains(&format!("exec bash bundles/self/scripts/{target} \"$@\"")),
            "{shim} is not the fixed dispatch to the pinned {target}"
        );
    }
    // One canonical scanner: the old candidate copy's sources are gone (its
    // gitignored build output may linger locally and is never executed).
    for old in [
        "tools/coverage-fn-scan/Cargo.toml",
        "tools/coverage-fn-scan/src/main.rs",
    ] {
        assert!(
            !Path::new(env!("CARGO_MANIFEST_DIR")).join(old).exists(),
            "{old} still exists"
        );
    }
}
