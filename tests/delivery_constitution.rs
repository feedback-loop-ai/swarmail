//! Structural pins on the delivery constitution (`.brokkr/recipes/self/`), accepted
//! decision 0050. These read the committed JSON and scripts; they are
//! regression guards on the shape the repairs gave the constitution, not a
//! substitute for the pinned engine's own evaluation (0050 §5) — the verify
//! seat compiles the bundle with the pinned brokkr and refuses its
//! policy-sweep warning.

use std::path::Path;
use std::process::Command;

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

fn sha256_file(path: &str) -> String {
    let absolute = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let output = Command::new("sha256sum")
        .arg(&absolute)
        .output()
        .or_else(|_| {
            Command::new("shasum")
                .arg("-a")
                .arg("256")
                .arg(&absolute)
                .output()
        })
        .unwrap_or_else(|e| panic!("cannot run a SHA-256 utility for {path}: {e}"));
    assert!(output.status.success(), "SHA-256 failed for {path}");
    String::from_utf8(output.stdout)
        .expect("SHA-256 output is UTF-8")
        .split_whitespace()
        .next()
        .expect("SHA-256 output contains a digest")
        .to_owned()
}

/// #125, amended by decision 0102 and its 2026-10-31 addendum: a spec defect
/// with every safety fact explicit returns to design (beside a security
/// residual too), and the sixth review that still finds one parks; a residual
/// above low, or any security residual, returns to implement with the panel's
/// findings, and the sixth review that still holds one parks for the
/// operator; only explicit non-security residuals at or below low ship, and a
/// final unconditional hard stop still catches spec defects beside applied
/// fixes and absent, null or malformed facts alike. At 2a325c9 the allow rule
/// had no `when` at all and nothing followed it.
#[test]
fn residual_admission_requires_explicit_safe_facts_and_unknowns_stop() {
    let policy = read_json(".brokkr/recipes/self/policy.json");
    assert_eq!(policy["schema"], "forge.phase-machine/v2", "park is v2");
    let residual = rules_from(&policy, "review", "residual");
    let ids: Vec<&str> = residual.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "REVIEW-RESIDUAL-SPEC-EXHAUSTED",
            "REVIEW-RESIDUAL-SPEC-SECURITY-EXHAUSTED",
            "REVIEW-RESIDUAL-SPEC-REDESIGN",
            "REVIEW-RESIDUAL-SPEC-SECURITY-REDESIGN",
            "REVIEW-RESIDUAL-EXHAUSTED",
            "REVIEW-RESIDUAL-SECURITY-EXHAUSTED",
            "REVIEW-RESIDUAL-REWORK",
            "REVIEW-RESIDUAL-SECURITY-REWORK",
            "REVIEW-RESIDUAL-OK",
            "REVIEW-RESIDUAL-LOW-FIXED",
            "REVIEW-RESIDUAL-UNSAFE"
        ]
    );
    // A spec defect needs every residual fact present and well-formed: a
    // severity bound that admits every severity word still refuses an absent,
    // null or malformed one, and security is split, never left unread.
    let spec = |security: bool| json!({"spec_defect": true, "fixes_applied": false, "has_security_residual": security, "max_residual_severity_at_most": "critical"});
    let above_low = json!({"max_residual_severity_above": "low", "spec_defect": false});
    let security = json!({"has_security_residual": true, "spec_defect": false});
    for (park, rework, facts, next) in [
        (0, 2, &spec(false), "design"),
        (1, 3, &spec(true), "design"),
        (4, 6, &above_low, "implement"),
        (5, 7, &security, "implement"),
    ] {
        let mut exhausted = facts.clone();
        exhausted["visits_review_gte"] = json!(6);
        assert_eq!(residual[park]["when"], exhausted, "five returns, then park");
        assert_eq!(residual[park]["park"], true);
        assert_eq!(residual[park].get("next"), None);
        assert_eq!(&residual[rework]["when"], facts);
        assert_eq!(residual[rework]["next"], next);
        assert_eq!(residual[rework].get("severity"), None);
    }
    let low = |fixes: bool| json!({"has_security_residual": false, "max_residual_severity_at_most": "low", "spec_defect": false, "fixes_applied": fixes});
    assert_eq!(residual[8]["when"], low(false));
    assert_eq!(residual[8]["next"], "ship");
    assert_eq!(residual[9]["when"], low(true));
    assert_eq!(residual[9]["next"], "verify");
    assert_eq!(residual[10].get("when"), None);
    assert_eq!(
        (
            residual[10]["next"].as_str(),
            residual[10]["severity"].as_str()
        ),
        (Some("stop"), Some("hard"))
    );
    let hold = rules_from(&policy, "review", "security-hold");
    assert_eq!(hold.len(), 1);
    assert_eq!(
        (hold[0]["next"].as_str(), hold[0]["severity"].as_str()),
        (Some("stop"), Some("hard"))
    );
}

/// Decision 0102's 2026-10-31 addendum: a clean verdict that finds the spec
/// defective, from a read-only panel, returns to design and parks at the
/// sixth review; its ship and reverify rows still demand `spec_defect: false`,
/// and the final unconditional stop still catches a spec defect beside
/// applied fixes and every unknown fact.
#[test]
fn clean_spec_defects_return_to_design_and_unknowns_stop() {
    let policy = read_json(".brokkr/recipes/self/policy.json");
    let clean = rules_from(&policy, "review", "clean");
    let ids: Vec<&str> = clean.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "REVIEW-CLEAN-SPEC-EXHAUSTED",
            "REVIEW-CLEAN-SPEC-REDESIGN",
            "REVIEW-CLEAN-NO-FIXES",
            "REVIEW-CLEAN",
            "REVIEW-CLEAN-UNSAFE"
        ]
    );
    let spec = json!({"spec_defect": true, "fixes_applied": false});
    let mut exhausted = spec.clone();
    exhausted["visits_review_gte"] = json!(6);
    assert_eq!(clean[0]["when"], exhausted);
    assert_eq!(clean[0]["park"], true);
    assert_eq!(clean[0].get("next"), None);
    assert_eq!(clean[1]["when"], spec);
    assert_eq!(clean[1]["next"], "design");
    assert_eq!(
        clean[2]["when"],
        json!({"fixes_applied": false, "spec_defect": false})
    );
    assert_eq!(clean[2]["next"], "ship");
    assert_eq!(
        clean[3]["when"],
        json!({"fixes_applied": true, "spec_defect": false})
    );
    assert_eq!(clean[3]["next"], "verify");
    assert_eq!(clean[4].get("when"), None);
    assert_eq!(
        (clean[4]["next"].as_str(), clean[4]["severity"].as_str()),
        (Some("stop"), Some("hard"))
    );
}

/// Decision 0102's 2026-10-31 addendum: across the whole table, a rule that
/// reads `spec_defect: true` either returns to design or parks for the
/// operator, and every route to ship or verify requires `spec_defect: false`.
/// The design return shares 0102's budget: every review return, to implement
/// or to design, has a parking twin at the same sixth review.
#[test]
fn a_spec_defect_returns_to_design_or_parks_and_never_ships() {
    let policy = read_json(".brokkr/recipes/self/policy.json");
    let rules = policy["rules"].as_array().unwrap();
    let defect: Vec<&Value> = rules
        .iter()
        .filter(|rule| rule["when"]["spec_defect"] == true)
        .collect();
    assert_eq!(defect.len(), 6);
    for rule in &defect {
        assert_eq!(rule["from"], "review", "{rule}");
        assert_eq!(rule["when"]["fixes_applied"], false, "{rule}");
        match rule.get("next") {
            Some(next) => assert_eq!(next, "design", "{rule}"),
            None => {
                assert_eq!(rule["park"], true, "{rule}");
                assert_eq!(rule["when"]["visits_review_gte"], 6, "{rule}");
            }
        }
    }
    for rule in rules.iter().filter(|rule| {
        rule["from"] == "review" && matches!(rule["next"].as_str(), Some("ship" | "verify"))
    }) {
        assert_eq!(rule["when"]["spec_defect"], false, "{rule}");
    }
    let to_design: Vec<&str> = rules
        .iter()
        .filter(|rule| rule["from"] == "review" && rule["next"] == "design")
        .map(|rule| rule["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        to_design,
        [
            "REVIEW-RESIDUAL-SPEC-REDESIGN",
            "REVIEW-RESIDUAL-SPEC-SECURITY-REDESIGN",
            "REVIEW-CLEAN-SPEC-REDESIGN"
        ]
    );
    for rule in rules.iter().filter(|rule| {
        rule["from"] == "review" && matches!(rule["next"].as_str(), Some("design" | "implement"))
    }) {
        let mut parked = rule["when"].clone();
        parked["visits_review_gte"] = json!(6);
        let twin = rules
            .iter()
            .find(|park| park["park"] == true && park["when"] == parked)
            .unwrap_or_else(|| panic!("{} has no parking twin", rule["id"]));
        let at = |id: &Value| rules.iter().position(|r| &r["id"] == id).unwrap();
        assert!(
            at(&twin["id"]) < at(&rule["id"]),
            "park before {}",
            rule["id"]
        );
    }
}

/// #124: only the deterministic check's `designed` advances design; a
/// phase-level `drafted` is a hard stop (at 2a325c9 DESIGN-OK-DRAFTED sent it
/// to implement); one failure retries and the second stops.
#[test]
fn only_the_design_check_advances_to_implement() {
    let policy = read_json(".brokkr/recipes/self/policy.json");
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
    let bundle = read_json(".brokkr/recipes/self/bundle.json");
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
            .join(".brokkr/recipes/self/scripts/design-check.sh")
            .is_file()
    );
}

/// #123: the verify seat runs every gate from its own pinned directory and
/// never invokes a candidate copy; the public entry points are only the fixed
/// shims the pinned digests name.
#[test]
fn verify_runs_the_pinned_gates_and_checks_the_public_shims() {
    let seat = read_text(".brokkr/recipes/self/scripts/verify-seat.sh");
    for gate in [
        "\"bash $pin_q/coverage-gate.sh\"",
        "\"bash $pin_q/coverage-gate.sh selftest\"",
        "\"bash $pin_q/design-check.sh selftest\"",
        "\"bash $pin_q/delivery-proof-test.sh\"",
        "\"bash $pin_q/publish-guard-test.sh\"",
        "\"bash $pin_q/seal-branch-test.sh\"",
        "'cd evals && cargo fmt --all -- --check'",
        "'cd evals && cargo clippy --all-targets --locked -- -D warnings'",
        "cargo build --locked --bin swarmail --message-format=json",
        "cd evals && SWARMAIL_BIN=\"$bin\" cargo test --locked",
    ] {
        assert!(seat.contains(gate), "verify-seat.sh does not run {gate}");
    }
    // The eval gates sit between the root suite and the bundle compile.
    let at = |needle: &str| seat.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("run \"cargo test\"") < at("run \"evals cargo fmt\""));
    assert!(at("run \"evals cargo test\"") < at("run \"brokkr compile"));
    let preflight = seat
        .split("for resource in")
        .nth(1)
        .and_then(|tail| tail.split("; do").next())
        .expect("verify-seat has a pinned-resource preflight");
    for resource in [
        "coverage-gate.sh",
        "publish-guard.sh",
        "publish-guard-test.sh",
        "design-check.sh",
        "delivery-proof-test.sh",
        "seal-branch.sh",
        "seal-branch-test.sh",
        "speckit/check-prerequisites.sh",
        "speckit/common.sh",
        "coverage-fn-scan/Cargo.toml",
        "coverage-fn-scan/Cargo.lock",
        "coverage-fn-scan/src/main.rs",
        "shims.sha256",
        "../tooling.json",
    ] {
        assert!(
            preflight.contains(resource),
            "verify-seat does not preflight {resource}"
        );
    }
    assert!(seat.contains("bash -c \"$command\""));
    assert!(!seat.contains("bash -lc \"$command\""));
    for candidate_copy in ["bash scripts/", "bash ./scripts/", "tools/coverage-fn-scan"] {
        assert!(
            !seat.contains(candidate_copy),
            "verify-seat.sh still runs {candidate_copy}"
        );
    }
    assert!(seat.contains("sha256sum --check --strict \"$pin_dir/shims.sha256\""));
    let manifest = read_text(".brokkr/recipes/self/scripts/shims.sha256");
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
            ".specify/scripts/bash/check-prerequisites.sh",
            "scripts/seal-branch.sh",
            "scripts/seal-branch-test.sh"
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
            text.contains(&format!(
                "exec bash .brokkr/recipes/self/scripts/{target} \"$@\""
            )),
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

/// Fixed reviewed digests make the tracked bundle, shim manifest and canonical
/// boundary checks fail closed when their bytes change. The manifest itself is
/// pinned independently, then every listed public shim is checked against it.
#[test]
fn pinned_bundle_shims_and_proof_dispatch_are_digest_bound() {
    for (path, expected) in [
        (
            ".brokkr/recipes/self/bundle.json",
            "2a5fa3696dac122822102316041022850d1630641987b96156a2f4f784bdb265",
        ),
        (
            ".brokkr/recipes/self/policy.json",
            "2f07ed7d734f67ae43134942fcea028e41904efcab1bdd9839f4f3cceb02ea90",
        ),
        (
            ".brokkr/recipes/self/tooling.json",
            "dbd9b411de49e182d719e23eb6adb6aa18f5dd19e32b05cc9613259421e2b2ff",
        ),
        (
            ".brokkr/recipes/self/scripts/shims.sha256",
            "e3acc3f2220b266f5ecdf06e1d4c5ba594892514fc2535134f7d504aa83fc30e",
        ),
        (
            ".brokkr/recipes/self/scripts/coverage-gate.sh",
            "f96b60bef6582efbce6166a6a8da71ab73aad81e545908e2bf6238e5b36c1fbf",
        ),
        (
            ".brokkr/recipes/self/scripts/design-check.sh",
            "05285f7c0c0b1ee89e00daafefcf4d082c3057f07dcbe8b16c060f517e92134e",
        ),
        (
            ".brokkr/recipes/self/scripts/verify-seat.sh",
            "b42a7928fac7287824baa2b04672e0539085fdec4ffcbe1539cecb95fd7ed81a",
        ),
        (
            ".brokkr/recipes/self/scripts/delivery-proof-test.sh",
            "9a048b395f5a73360127b8e20455a70ed2c89aa1d82d67974b8882c6903d08a3",
        ),
    ] {
        assert_eq!(
            sha256_file(path),
            expected,
            "{path} changed without a reviewed pin"
        );
    }

    let manifest = read_text(".brokkr/recipes/self/scripts/shims.sha256");
    let mut manifest_paths = manifest
        .lines()
        .map(|line| line.split_once("  ").expect("sha256 manifest row").1)
        .collect::<Vec<_>>();
    manifest_paths.sort_unstable();
    assert_eq!(
        manifest_paths,
        [
            ".specify/scripts/bash/check-prerequisites.sh",
            "scripts/coverage-gate.sh",
            "scripts/design-check.sh",
            "scripts/publish-guard-test.sh",
            "scripts/publish-guard.sh",
            "scripts/seal-branch-test.sh",
            "scripts/seal-branch.sh",
        ],
        "the pinned public shim manifest must contain exactly seven reviewed members"
    );
    for line in manifest.lines() {
        let (expected, path) = line.split_once("  ").expect("sha256 manifest row");
        assert_eq!(
            sha256_file(path),
            expected,
            "{path} differs from the pinned shim manifest"
        );
    }

    let bundle = read_json(".brokkr/recipes/self/bundle.json");
    let design = bundle["seats"]["design"]["sequence"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(design["driver"]["command"][4], "bash");
    assert_eq!(design["driver"]["command"][5], "./scripts/design-check.sh");

    let seat = read_text(".brokkr/recipes/self/scripts/verify-seat.sh");
    let active = seat
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "recorded_sha=",
        "launched_digest=",
        "recorded_digests=",
        "grep -qx \"$launched_digest\"",
        "fail_now \"verify-seat: tooling.json records no binary_sha256",
        "does not match the recorded binary_sha256",
        "run \"delivery tamper-boundary proof\"",
        "\"bash $pin_q/delivery-proof-test.sh\"",
    ] {
        assert!(
            active.contains(needle),
            "verify-seat lost required pinned behavior: {needle}"
        );
    }
    for candidate_dispatch in ["bash scripts/", "bash ./scripts/"] {
        assert!(
            !active.contains(candidate_dispatch),
            "verify-seat dispatches a candidate-controlled gate: {candidate_dispatch}"
        );
    }
}

/// The lines of `text` that open a fenced command with `prefix`, each joined
/// with its `\`-continued lines.
fn commands(text: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if !line.starts_with(prefix) {
            continue;
        }
        let mut command = line.trim_end_matches('\\').to_string();
        let mut open = line.ends_with('\\');
        while open {
            let next = lines.next().expect("a continued command has a next line");
            command.push_str(next.trim_end_matches('\\'));
            open = next.ends_with('\\');
        }
        out.push(command);
    }
    out
}

/// #141: every documented `docker run` cleans up its anonymous `/data`
/// volume (`--rm`) and gives the 10 s drain room (`--stop-timeout 30`); the
/// `:main` Helm line pulls on every start; the stale "until the next
/// `:main`" workaround is gone; the build context excludes `target/` at any
/// depth; and the Dockerfile no longer calls SIGKILL Docker's default. At
/// 2a325c9 the README's runs had no `--rm`, the Helm line kept
/// `IfNotPresent` and `.dockerignore` matched only the root `target/`. The
/// real-Docker proof of the same contract is `scripts/docker-smoke.sh` (CI).
#[test]
fn the_install_examples_clean_up_and_pull_what_they_name() {
    let readme = read_text("README.md");
    let runs = commands(&readme, "docker run ");
    assert!(
        runs.len() >= 3,
        "the README documents its docker runs: {runs:#?}"
    );
    for run in &runs {
        assert!(
            run.contains(" --rm "),
            "a documented run leaks its volume: {run}"
        );
        assert!(
            run.contains(" --stop-timeout 30 "),
            "a documented run keeps Docker's 10 s grace: {run}"
        );
    }
    let helm = commands(&readme, "helm install swarmail ./charts/swarmail ");
    assert!(!helm.is_empty(), "the README documents its helm install");
    for line in &helm {
        if line.contains("--set image.tag=main") {
            assert!(
                line.contains("--set image.pullPolicy=Always"),
                "a :main install must pull on every start: {line}"
            );
        }
    }
    assert!(
        !readme.contains("until the next `:main`"),
        "the README keeps the expired next-:main workaround"
    );

    let ignore = read_text(".dockerignore");
    assert!(
        ignore.lines().any(|l| l.trim() == "**/target/"),
        ".dockerignore must exclude target/ at every depth"
    );
    assert!(
        !ignore.lines().any(|l| l.trim() == "target/"),
        "a root-only target/ pattern is the #141 leak"
    );
    let dockerfile = read_text("Dockerfile");
    assert!(
        !dockerfile.contains("the default is SIGKILL"),
        "the Dockerfile still calls SIGKILL Docker's default stop signal"
    );
}

/// #141: the workflow runs the chart-bump check, its selftest and the real
/// Docker smoke, and the smoke job is read-only. The job ordering itself is
/// proved by the pinned publish-guard proof (its release-wiring mutants).
#[test]
fn ci_runs_the_chart_history_check_and_the_docker_smoke() {
    let ci = read_text(".github/workflows/ci.yml");
    for needle in [
        "run: bash scripts/chart-verify.sh selftest",
        "run: bash scripts/chart-verify.sh history",
        "CHART_PR_BASE: ${{ github.event.pull_request.base.sha }}",
        "CHART_PUSH_BEFORE: ${{ github.event.before }}",
        "fetch-depth: 0",
        "run: bash scripts/docker-smoke.sh",
    ] {
        assert!(ci.contains(needle), "ci.yml must carry {needle:?}");
    }
    let smoke = ci
        .split("\n  docker-smoke:\n")
        .nth(1)
        .and_then(|rest| rest.split("\n  chart-publish:\n").next())
        .expect("ci.yml has a docker-smoke job before chart-publish");
    assert!(
        !smoke.contains("packages: write"),
        "the smoke job publishes nothing"
    );
}

/// #201: the `test` matrix job (ubuntu and macOS) runs the standalone eval
/// workspace's fmt, clippy and tests against the candidate's own binary. No
/// eval step is conditional or allowed to fail, and no new job (hence no new
/// status context) carries them.
#[test]
fn ci_gates_the_eval_harness_on_both_operating_systems() {
    let ci = read_text(".github/workflows/ci.yml");
    let test_job = ci
        .split("\n  test:\n")
        .nth(1)
        .and_then(|rest| rest.split("\n  coverage:\n").next())
        .expect("ci.yml has a test job before coverage");
    assert!(
        test_job.contains("os: [ubuntu-latest, macos-latest]"),
        "the eval gates ride the two-OS test matrix"
    );
    for needle in [
        "components: rustfmt, clippy",
        "evals -> target",
        "cargo build --locked --bin swarmail --message-format=json",
        "echo \"SWARMAIL_BIN=$bin\" >> \"$GITHUB_ENV\"",
        "test -x \"$bin\"",
    ] {
        assert!(
            test_job.contains(needle),
            "the test job must carry {needle:?}"
        );
    }
    let steps: Vec<&str> = test_job.split("\n      - ").collect();
    let step = |name: &str| -> &str {
        steps
            .iter()
            .copied()
            .find(|step| step.starts_with(&format!("name: {name}")))
            .unwrap_or_else(|| panic!("the test job has no step named {name:?}"))
    };
    let resolve = step("Resolve the candidate Swarmail binary");
    for (name, command) in [
        ("evals fmt", "cargo fmt --all -- --check"),
        (
            "evals clippy",
            "cargo clippy --all-targets --locked -- -D warnings",
        ),
        ("evals test", "cargo test --locked"),
    ] {
        let body = step(name);
        assert!(body.contains(&format!("run: {command}")), "{name}: {body}");
        assert!(body.contains("working-directory: evals"), "{name}: {body}");
        for forbidden in ["if:", "continue-on-error", "|| true"] {
            assert!(
                !body.contains(forbidden),
                "{name} must not carry {forbidden}"
            );
        }
    }
    for forbidden in ["if:", "continue-on-error", "|| true"] {
        assert!(
            !resolve.contains(forbidden),
            "the resolve step must not carry {forbidden}"
        );
    }
    // Order: the root suite builds first, the binary is resolved, the three
    // gates follow, and the privileged port-80 steps stay last.
    let position = |needle: &str| test_job.find(needle).unwrap_or_else(|| panic!("{needle}"));
    let order = [
        position("run: cargo test --workspace --locked"),
        position("name: Resolve the candidate Swarmail binary"),
        position("name: evals fmt"),
        position("name: evals clippy"),
        position("name: evals test"),
        position("portless_bridge_pings_port_eighty"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    // The harness's build-when-unset fallback must never be what a gate runs.
    assert_eq!(
        ci.matches("working-directory: evals").count(),
        3,
        "only the three eval gates run from evals/"
    );
}

#[test]
fn codex_correctness_charter_provenance_matches_the_tracked_library() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = read_text(".brokkr/agents/README.md");
    let overlay = read_json(".brokkr/agents/codex-review-correctness.json");
    let base = read_json(".brokkr/agents/review-correctness.json");
    assert_eq!(overlay["extends"], "review-correctness");
    assert_eq!(overlay["models"], json!(["fable"]));
    assert_eq!(base["charter"], "charters/review-correctness.md");
    assert!(readme.contains("brokkr compile --bundle .brokkr/recipes/self"));
    assert!(readme.contains(".brokkr/state/bin/brokkr agents show codex-review-correctness"));
    assert!(readme.contains("historical host copy"));
    assert!(readme.contains("actual review launch"));

    let charter = root.join(".brokkr/agents/charters/review-correctness.md");
    let digest_output = std::process::Command::new("sha256sum")
        .arg(charter)
        .output()
        .expect("sha256sum is available");
    assert!(digest_output.status.success());
    let digest = String::from_utf8(digest_output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    assert!(
        readme.contains(&digest),
        "README omits the tracked charter digest"
    );
}

// Runtime fixtures for the pinned verify seat. The gate resources are
// controlled test scripts, while verify-seat.sh itself is copied byte-for-byte
// from the tracked pinned bundle.
const VERIFY_RESOURCE_PATHS: &[&str] = &[
    "coverage-gate.sh",
    "publish-guard.sh",
    "publish-guard-test.sh",
    "design-check.sh",
    "delivery-proof-test.sh",
    "seal-branch.sh",
    "seal-branch-test.sh",
    "speckit/check-prerequisites.sh",
    "speckit/common.sh",
    "coverage-fn-scan/Cargo.toml",
    "coverage-fn-scan/Cargo.lock",
    "coverage-fn-scan/src/main.rs",
    "shims.sha256",
    "../tooling.json",
];

const VERIFY_SHIMS: &[&str] = &[
    "scripts/coverage-gate.sh",
    "scripts/publish-guard.sh",
    "scripts/publish-guard-test.sh",
    "scripts/design-check.sh",
    ".specify/scripts/bash/check-prerequisites.sh",
    "scripts/seal-branch.sh",
    "scripts/seal-branch-test.sh",
];

struct VerifySeatFixture {
    _root: tempfile::TempDir,
    candidate: std::path::PathBuf,
    pin: std::path::PathBuf,
    prompt: std::path::PathBuf,
    result: std::path::PathBuf,
    engine: std::path::PathBuf,
    bin: std::path::PathBuf,
    gate_order: std::path::PathBuf,
    gate_report: std::path::PathBuf,
    evals_report: std::path::PathBuf,
    swarmail: std::path::PathBuf,
    argv_marker: std::path::PathBuf,
    later_marker: std::path::PathBuf,
}

impl VerifySeatFixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::Builder::new()
            .prefix("verify seat ; [fixture] ")
            .tempdir()
            .expect("temporary verify fixture");
        // The gate stubs report bash's $PWD, which bash derives from getcwd()
        // because Command::current_dir sets no PWD. macOS temp dirs sit behind
        // the /var -> /private/var symlink, so every fixture path is built on
        // the physical root.
        let base = root.path().canonicalize().expect("canonical fixture root");
        let candidate = base.join("candidate ; [workspace]");
        let bundle = base.join("bundle ; [pinned]");
        let pin = bundle.join("scripts");
        let result_dir = base.join("result files ; [entry]");
        let bin = base.join("fake commands");
        let engine_dir = base.join("engine path ; [literal]");
        let engine = engine_dir.join("brokkr");
        let gate_order = base.join("gate order.txt");
        let gate_report = base.join("gate report.txt");
        let evals_report = base.join("evals report.txt");
        let swarmail = base.join("built swarmail ; [bin]");
        let argv_marker = base.join("engine argv.txt");
        let later_marker = base.join("later gate.txt");
        let prompt = base.join("verify prompt.md");
        let result = result_dir.join("seat result.json");

        std::fs::create_dir_all(&candidate).unwrap();
        // The standalone eval workspace the seat's evals gates `cd` into.
        std::fs::create_dir_all(candidate.join("evals")).unwrap();
        std::fs::create_dir_all(&pin).unwrap();
        std::fs::create_dir_all(&result_dir).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&engine_dir).unwrap();

        for shim in VERIFY_SHIMS {
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(shim);
            let destination = candidate.join(shim);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::copy(source, destination).unwrap();
        }
        let verify_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".brokkr/recipes/self/scripts/verify-seat.sh");
        std::fs::copy(verify_source, pin.join("verify-seat.sh")).unwrap();

        let engine_script = r#"#!/usr/bin/env bash
printf 'brokkr\n' >> "$GATE_ORDER"
printf '%s\n' "$@" > "$BROKKR_ARGV_MARKER"
exit 0
"#;
        std::fs::write(&engine, engine_script).unwrap();
        let mut engine_permissions = std::fs::metadata(&engine).unwrap().permissions();
        engine_permissions.set_mode(0o755);
        std::fs::set_permissions(&engine, engine_permissions).unwrap();

        // `cargo build --message-format=json` answers with the records the
        // seat's filter must tell apart: a decoy bin, the lib of the same
        // name (no executable) and the real swarmail bin.
        let cargo_script = r#"#!/usr/bin/env bash
printf 'cargo:%s\n' "$*" >> "$GATE_ORDER"
if [ "$1" = tree ]; then
  printf 'error: package ID specification native-tls did not match any packages\n' >&2
  exit 101
fi
if [ "$1" = build ]; then
  jq -cn '{reason: "compiler-artifact", target: {name: "decoy", kind: ["bin"]}, executable: "/nonexistent/decoy"}'
  jq -cn '{reason: "compiler-artifact", target: {name: "swarmail", kind: ["lib"]}, executable: null}'
  jq -cn --arg exe "$FAKE_SWARMAIL" '{reason: "compiler-artifact", target: {name: "swarmail", kind: ["bin"]}, executable: $exe}'
  jq -cn '{reason: "build-finished", success: true}'
  exit 0
fi
case "$PWD" in
  */evals) printf '%s\tcwd=%s\tswarmail_bin=%s\n' "$1" "$PWD" "${SWARMAIL_BIN-unset}" >> "$EVALS_REPORT" ;;
esac
exit 0
"#;
        let cargo = bin.join("cargo");
        std::fs::write(&cargo, cargo_script).unwrap();
        let mut cargo_permissions = std::fs::metadata(&cargo).unwrap().permissions();
        cargo_permissions.set_mode(0o755);
        std::fs::set_permissions(&cargo, cargo_permissions).unwrap();

        std::fs::write(&swarmail, "#!/bin/sh\nexit 0\n").unwrap();
        let mut swarmail_permissions = std::fs::metadata(&swarmail).unwrap().permissions();
        swarmail_permissions.set_mode(0o755);
        std::fs::set_permissions(&swarmail, swarmail_permissions).unwrap();

        std::fs::write(
            &prompt,
            format!("Result file\n\n    {}\n", result.display()),
        )
        .unwrap();

        let fixture = Self {
            _root: root,
            candidate,
            pin,
            prompt,
            result,
            engine,
            bin,
            gate_order,
            gate_report,
            evals_report,
            swarmail,
            argv_marker,
            later_marker,
        };
        fixture.install_resources();
        fixture.write_tooling(Some(&fixture.engine_digest()));
        fixture
    }

    fn engine_digest(&self) -> String {
        let output = std::process::Command::new("sha256sum")
            .arg(&self.engine)
            .output()
            .expect("sha256sum is available to the verify fixture");
        assert!(output.status.success(), "sha256sum failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned()
    }

    fn write_tooling(&self, digest: Option<&str>) {
        let broker = match digest {
            Some(digest) => serde_json::json!({
                "sha": "fixture-engine",
                "binary_sha256": digest,
            }),
            None => serde_json::json!({ "sha": "fixture-engine" }),
        };
        let path = self.pin.parent().unwrap().join("tooling.json");
        std::fs::write(
            path,
            serde_json::to_vec(&serde_json::json!({ "brokkr": broker })).unwrap(),
        )
        .unwrap();
    }

    fn install_resources(&self) {
        for resource in VERIFY_RESOURCE_PATHS {
            self.restore_resource(resource);
        }
    }

    fn restore_resource(&self, resource: &str) {
        let path = self.pin.join(resource);
        if resource == "../tooling.json" {
            self.write_tooling(Some(&self.engine_digest()));
            return;
        }
        if resource == "shims.sha256" {
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".brokkr/recipes/self/scripts/shims.sha256");
            std::fs::copy(source, path).unwrap();
            return;
        }
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if resource == "coverage-gate.sh" {
            let coverage = r#"#!/usr/bin/env bash
printf '%s\n' coverage-gate.sh >> "$GATE_ORDER"
if IFS= read -r input; then input_state=available; else input_state=eof; fi
printf 'cwd=%s\noffline=%s\nstdin=%s\n' "$PWD" "${CARGO_NET_OFFLINE-unset}" "$input_state" > "$GATE_REPORT"
printf 'forced coverage failure\n' >&2
exit 17
"#;
            std::fs::write(path, coverage).unwrap();
        } else if resource.ends_with(".sh") {
            let later = r#"#!/usr/bin/env bash
printf '%s\n' "${0##*/}" >> "$GATE_ORDER"
printf '%s\n' invoked > "$LATER_MARKER"
exit 0
"#;
            std::fs::write(path, later).unwrap();
        } else {
            std::fs::write(path, "fixture resource\n").unwrap();
        }
    }

    fn base_command(&self) -> std::process::Command {
        let mut command = std::process::Command::new("bash");
        let verify_script = self.pin.join("verify-seat.sh");
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let path = std::env::join_paths(
            std::iter::once(self.bin.clone()).chain(std::env::split_paths(&current_path)),
        )
        .unwrap();
        command
            .arg(verify_script)
            .arg(&self.prompt)
            .arg(&self.engine)
            .current_dir(&self.candidate)
            .env("PATH", path)
            .env("GATE_ORDER", &self.gate_order)
            .env("GATE_REPORT", &self.gate_report)
            .env("EVALS_REPORT", &self.evals_report)
            .env("FAKE_SWARMAIL", &self.swarmail)
            // An inherited value must not satisfy the seat: it resolves its own.
            .env("SWARMAIL_BIN", "/inherited/must-not-be-used")
            .env("BROKKR_ARGV_MARKER", &self.argv_marker)
            .env("LATER_MARKER", &self.later_marker);
        command
    }

    fn read_result(&self) -> serde_json::Value {
        let text = std::fs::read_to_string(&self.result).expect("verify seat wrote its result");
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("verify seat result is JSON");
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(keys, ["inputs", "notes", "result"]);
        value
    }
}

#[test]
fn verify_seat_runs_gates_without_login_and_preserves_failure_evidence() {
    let fixture = VerifySeatFixture::new();
    let hook = fixture._root.path().join("bash startup hook.sh");
    let startup_log = fixture._root.path().join("startup observations.txt");
    let home = fixture._root.path().join("isolated login home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        &hook,
        r#"if [[ -n ${BASH_EXECUTION_STRING-} ]]; then
  if shopt -q login_shell; then shell_mode=login; else shell_mode=nonlogin; fi
  printf '%s\t%s\n' "$shell_mode" "$BASH_EXECUTION_STRING" >> "$STARTUP_LOG"
fi
"#,
    )
    .unwrap();

    let positive = std::process::Command::new("bash")
        .args(["-lc", "true"])
        .current_dir(&fixture.candidate)
        .env("HOME", &home)
        .env("BASH_ENV", &hook)
        .env("STARTUP_LOG", &startup_log)
        .output()
        .expect("login-shell positive control started");
    assert!(positive.status.success());
    let positive_log =
        std::fs::read_to_string(&startup_log).expect("login positive control reached BASH_ENV");
    assert!(
        positive_log
            .lines()
            .any(|line| line.starts_with("login\ttrue")),
        "the positive control did not observe login-shell startup: {positive_log}"
    );
    std::fs::write(&startup_log, "").unwrap();

    let output = fixture
        .base_command()
        .env("BASH_ENV", &hook)
        .env("STARTUP_LOG", &startup_log)
        .env("HOME", &home)
        .output()
        .expect("actual verify seat fixture ran");
    assert!(
        output.status.success(),
        "verify seat reports through JSON, not its process status: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result = fixture.read_result();
    assert_eq!(result["result"], "fail");
    let notes = result["notes"].as_str().unwrap();
    assert!(notes.contains("forced coverage failure"), "{notes}");

    let order = std::fs::read_to_string(&fixture.gate_order).unwrap();
    assert_eq!(
        order.lines().collect::<Vec<_>>(),
        [
            "cargo:fmt --all -- --check",
            "cargo:clippy --workspace --all-targets --locked -- -D warnings",
            "cargo:test --workspace --locked",
            "cargo:fmt --all -- --check",
            "cargo:clippy --all-targets --locked -- -D warnings",
            "cargo:build --locked --bin swarmail --message-format=json",
            "cargo:test --locked",
            "brokkr",
            "coverage-gate.sh",
        ]
    );
    // The eval gates ran inside evals/, and the suite ran against the binary
    // the build record named — not the inherited value, not a decoy.
    let evals = std::fs::read_to_string(&fixture.evals_report).unwrap();
    let evals_dir = fixture.candidate.join("evals");
    let swarmail = fixture.swarmail.display();
    let observed: Vec<_> = evals.lines().collect();
    assert_eq!(observed.len(), 3, "{evals}");
    assert!(
        observed[0].starts_with(&format!("fmt\tcwd={}\t", evals_dir.display())),
        "{evals}"
    );
    assert!(
        observed[1].starts_with(&format!("clippy\tcwd={}\t", evals_dir.display())),
        "{evals}"
    );
    assert_eq!(
        observed[2],
        format!("test\tcwd={}\tswarmail_bin={swarmail}", evals_dir.display())
    );
    let report = std::fs::read_to_string(&fixture.gate_report).unwrap();
    assert!(report.contains(&format!("cwd={}", fixture.candidate.display())));
    assert!(report.contains("offline=true"));
    assert!(report.contains("stdin=eof"));
    assert!(
        !fixture.later_marker.exists(),
        "a later gate ran after failure"
    );

    let argv = std::fs::read_to_string(&fixture.argv_marker).unwrap();
    assert_eq!(
        argv.lines().collect::<Vec<_>>(),
        ["compile", "--bundle", ".brokkr/recipes/self"]
    );
    let startup = std::fs::read_to_string(&startup_log).unwrap();
    let observed: Vec<_> = startup.lines().collect();
    assert!(
        observed.len() >= 5,
        "the startup hook did not observe each gate shell: {startup}"
    );
    assert!(
        observed.iter().all(|line| line.starts_with("nonlogin\t")),
        "a gate runner used login startup: {startup}"
    );
}

/// #201: the eval suite never runs against a binary the seat could not
/// resolve. A build record naming no executable fails the seat closed, with the
/// reason in the notes, before the harness's own build-when-unset path could.
#[test]
fn verify_seat_refuses_the_eval_suite_without_an_executable_candidate_binary() {
    let fixture = VerifySeatFixture::new();
    let output = fixture
        .base_command()
        .env("FAKE_SWARMAIL", fixture.candidate.join("not built"))
        .output()
        .expect("actual verify seat fixture ran");
    assert!(output.status.success());
    let result = fixture.read_result();
    assert_eq!(result["result"], "fail");
    let notes = result["notes"].as_str().unwrap();
    assert!(notes.contains("evals cargo test failed"), "{notes}");
    assert!(notes.contains("no executable swarmail binary"), "{notes}");
    let order = std::fs::read_to_string(&fixture.gate_order).unwrap();
    assert_eq!(
        order.lines().last(),
        Some("cargo:build --locked --bin swarmail --message-format=json"),
        "the eval suite ran, or a later gate did, without a binary: {order}"
    );
    assert!(!fixture.later_marker.exists());
}

#[test]
fn verify_seat_preflights_each_static_resource_before_any_gate() {
    let fixture = VerifySeatFixture::new();
    for resource in VERIFY_RESOURCE_PATHS {
        let missing = fixture.pin.join(resource);
        std::fs::remove_file(&missing).unwrap_or_else(|error| {
            panic!("could not remove preflight fixture resource {resource}: {error}")
        });
        std::fs::write(
            &fixture.result,
            r#"{"result":"pass","inputs":{},"notes":"stale result"}"#,
        )
        .unwrap();
        let _ = std::fs::remove_file(&fixture.gate_order);
        let output = fixture
            .base_command()
            .output()
            .expect("actual verify seat missing-resource fixture ran");
        assert!(
            output.status.success(),
            "verify seat writes fail verdicts as JSON: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = fixture.read_result();
        assert_eq!(result["result"], "fail", "missing {resource}");
        assert!(
            result["notes"].as_str().unwrap().contains(resource),
            "missing-resource diagnostic omitted {resource}: {}",
            result["notes"]
        );
        assert!(
            !fixture.gate_order.exists(),
            "a gate ran before missing resource {resource} was refused"
        );
        fixture.restore_resource(resource);
    }
}

#[test]
fn verify_seat_refuses_missing_or_mismatched_engine_identity_before_gates() {
    let fixture = VerifySeatFixture::new();
    for (digest, expected) in [
        (None, "records no binary_sha256"),
        (
            Some("0000000000000000000000000000000000000000000000000000000000000000"),
            "does not match the recorded binary_sha256",
        ),
    ] {
        fixture.write_tooling(digest);
        let _ = std::fs::remove_file(&fixture.gate_order);
        let output = fixture
            .base_command()
            .output()
            .expect("actual verify seat identity fixture ran");
        assert!(output.status.success());
        let result = fixture.read_result();
        assert_eq!(result["result"], "fail");
        assert!(
            result["notes"].as_str().unwrap().contains(expected),
            "identity refusal did not name {expected}: {}",
            result["notes"]
        );
        assert!(
            !fixture.gate_order.exists(),
            "a gate ran before identity refusal"
        );
        fixture.write_tooling(Some(&fixture.engine_digest()));
    }

    // A matched identity reaches an actual pass verdict, not just a different
    // early failure. Only the fixture's controlled gates are made successful.
    std::fs::write(
        fixture.pin.join("coverage-gate.sh"),
        "#!/usr/bin/env bash\nprintf '%s\\n' coverage-gate.sh >> \"$GATE_ORDER\"\n",
    )
    .unwrap();
    let output = fixture.base_command().output().unwrap();
    assert!(output.status.success());
    assert_eq!(fixture.read_result()["result"], "pass");
    let order = std::fs::read_to_string(&fixture.gate_order).unwrap();
    assert!(order.contains("brokkr\ncoverage-gate.sh\n"));
    assert!(order.contains("cargo:tree -i native-tls"));

    // The same oracle detects a disabled comparison in a scratch script:
    // admitting the wrong digest then yields pass and runs the gates.
    let script = fixture.pin.join("verify-seat.sh");
    let original = std::fs::read_to_string(&script).unwrap();
    let mutant = original.replace(
        "printf '%s\\n' \"$recorded_digests\" | grep -qx \"$launched_digest\" ||",
        "true ||",
    );
    assert_ne!(original, mutant);
    std::fs::write(&script, mutant).unwrap();
    fixture.write_tooling(Some(PUBLISHED_ENGINE_DIGEST));
    std::fs::remove_file(&fixture.gate_order).unwrap();
    let output = fixture.base_command().output().unwrap();
    assert!(output.status.success());
    assert_eq!(fixture.read_result()["result"], "pass");
    assert!(fixture.gate_order.exists());
    std::fs::write(script, original).unwrap();
}

const PUBLISHED_ENGINE_URL: &str = "https://github.com/feedback-loop-ai/brokkr/releases/download/swarmail-engine-a9893b4-p4/brokkr-0.12.0-a9893b4-p4-x86_64-unknown-linux-gnu";
const PUBLISHED_ENGINE_DIGEST: &str =
    "29dc3309925a2556b9713b7a9b5fb8089b8e4610e86215fc89554e75634e7cad";
const READ_TOOLING_STEP: &str = "Read the pinned brokkr record (fail closed)";
const ADMIT_ENGINE_STEP: &str = "Admit the published brokkr and compile every recipe";

#[test]
fn published_engine_record_matches_the_operator_release() {
    let record = read_json(".brokkr/recipes/self/tooling.json");
    let engine = &record["brokkr"];
    assert_eq!(engine["sha"], "a9893b4dfb67417edc99d91f941a9bac3856f138");
    assert_eq!(engine["version"], "0.12.0");
    assert_eq!(engine["binary_url"], PUBLISHED_ENGINE_URL);
    assert_eq!(engine["binary_sha256"], PUBLISHED_ENGINE_DIGEST);
    assert_eq!(
        engine["patches"],
        json!([
            {"path": "tooling-patches/0001-preserve-review-residual-facts.patch", "sha256": "5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5"},
            {"path": "tooling-patches/0002-preserve-review-scope-and-fixes.patch", "sha256": "361272eff344a5d5e05b832281407a99c3f1a77fc1fbc5216e19fb4ce7487173"},
            {"path": "tooling-patches/0003-bind-verified-candidate.patch", "sha256": "4098228e7649d1bda8961bb491aa29d982b462c420470e265f8b9acf0a148d96"},
            {"path": "tooling-patches/0004-raise-hands-command-timeout.patch", "sha256": "73bea06793febaed05edb23338e8963087ef6227888fe28cdd29337a418c15d6"}
        ])
    );
    for patch in engine["patches"].as_array().unwrap() {
        assert_eq!(
            sha256_file(&format!(
                ".brokkr/recipes/self/{}",
                patch["path"].as_str().unwrap()
            )),
            patch["sha256"].as_str().unwrap()
        );
    }
    let provenance = engine["binary_provenance"].as_str().unwrap();
    assert!(provenance.contains("source base a9893b4dfb67417edc99d91f941a9bac3856f138 plus"));
    assert!(provenance.contains("four checksum-pinned patches"));
    assert!(provenance.contains(
        "https://github.com/feedback-loop-ai/brokkr/releases/tag/swarmail-engine-a9893b4-p4"
    ));
    assert!(!provenance.contains(".forge/"));
}

// Execute the actual uniquely named literal workflow body. Missing or ambiguous
// boundaries refuse instead of silently testing a copy of the algorithm.
fn brokkr_ci_job() -> String {
    let ci = read_text(".github/workflows/ci.yml");
    let marker = "\n  brokkr-compile:\n";
    assert_eq!(ci.matches(marker).count(), 1);
    ci.split_once(marker)
        .unwrap()
        .1
        .lines()
        .take_while(|line| {
            !(line.starts_with("  ")
                && !line.starts_with("   ")
                && !line.trim_start().starts_with('#')
                && line.ends_with(':'))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn brokkr_ci_step(name: &str) -> String {
    let job = brokkr_ci_job();
    let marker = format!("      - name: {name}\n");
    assert_eq!(job.matches(&marker).count(), 1, "unique step {name}");
    job.split_once(&marker)
        .unwrap()
        .1
        .lines()
        .take_while(|line| !line.starts_with("      - "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn brokkr_ci_body(name: &str) -> String {
    let step = brokkr_ci_step(name);
    let marker = "        run: |\n";
    assert_eq!(step.matches(marker).count(), 1, "literal body for {name}");
    let body = step
        .split_once(marker)
        .unwrap()
        .1
        .lines()
        .take_while(|line| line.is_empty() || line.starts_with("          "))
        .map(|line| line.strip_prefix("          ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!body.is_empty(), "empty workflow body for {name}");
    assert!(!body.contains("${{"), "expression spliced into run body");
    body
}

#[test]
fn ci_transports_artifact_fields_through_env_and_retains_source_tests() {
    let step = brokkr_ci_step(ADMIT_ENGINE_STEP);
    for line in [
        "          BINARY_URL: ${{ steps.tooling.outputs.binary_url }}",
        "          BINARY_SHA256: ${{ steps.tooling.outputs.binary_sha256 }}",
        "          RECORDED_VERSION: ${{ steps.tooling.outputs.version }}",
    ] {
        assert!(step.lines().any(|actual| actual == line), "{line}");
    }
    brokkr_ci_body(READ_TOOLING_STEP);
    brokkr_ci_body(ADMIT_ENGINE_STEP);
    let job = brokkr_ci_job();
    for proof in [
        "python3 scripts/apply-brokkr-patches.py brokkr-src",
        "cargo test --locked -p brokkr-core policy::tests",
        "cargo test --locked -p brokkr-runtime --lib aggregate",
        "cargo test --locked -p brokkr-runtime --lib candidate_binding_tests",
    ] {
        assert!(job.contains(proof), "lost auxiliary source proof {proof}");
    }
    assert!(!job.contains("target/release/brokkr"));
    assert!(!job.contains("cargo build --release"));
}

struct BrokkrCiFixture {
    root: tempfile::TempDir,
    bin: std::path::PathBuf,
    marker: std::path::PathBuf,
    github_output: std::path::PathBuf,
    engine: std::path::PathBuf,
    runner_temp: std::path::PathBuf,
}

impl BrokkrCiFixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("brokkr CI ; [fixture] ")
            .tempdir()
            .unwrap();
        let bin = root.path().join("transport commands");
        let marker = root.path().join("engine execution.txt");
        let github_output = root.path().join("github output.txt");
        let engine = root.path().join("published bytes");
        let runner_temp = root.path().join("runner temp");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&runner_temp).unwrap();
        std::fs::create_dir_all(root.path().join(".brokkr/recipes/self")).unwrap();
        let fixture = Self {
            root,
            bin,
            marker,
            github_output,
            engine,
            runner_temp,
        };
        fixture.write_record(&read_json(".brokkr/recipes/self/tooling.json"));
        fixture.write_command(
            &fixture.engine,
            r#"#!/usr/bin/env bash
printf '%s\n' "$*" >> "$ENGINE_MARKER"
if [ "$*" = --version ]; then
  printf 'brokkr %s\n' "${FIXTURE_VERSION:-0.12.0}"
  exit 0
fi
[ "$1" = compile ] && [ "$2" = --bundle ] || exit 93
case "$3" in .brokkr/recipes/self|.brokkr/recipes/codex|.brokkr/recipes/codex-careful|.brokkr/recipes/codex-fast|.brokkr/recipes/mixed-opus-terra|.brokkr/recipes/claude-careful) ;; *) exit 94 ;; esac
if [ "$3" = "${FAIL_RECIPE:-bundles/codex-careful}" ]; then
  case "${COMPILE_MODE:-clean}" in
    stdout-warning) printf 'lets the run go on without reading fixture\n' ;;
    stderr-warning) printf 'lets the run go on without reading fixture\n' >&2 ;;
    failure) printf 'fixture compile failure\n' >&2; exit 23 ;;
  esac
fi
printf 'compiled %s\n' "$3"
"#,
        );
        fixture.write_command(
            &fixture.bin.join("curl"),
            r#"#!/usr/bin/env bash
printf '%s\n' "$@" > "$CURL_ARGS"
[ "${TRANSPORT_FAIL:-0}" = 0 ] || exit 22
while [ "$#" -gt 0 ]; do
  if [ "$1" = --output ]; then
    shift
    destination="$1"
  fi
  shift
done
cp "$FIXTURE_ASSET" "$destination"
"#,
        );
        fixture
    }

    fn write_command(&self, path: &Path, script: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, script).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_record(&self, record: &Value) {
        std::fs::write(
            self.root.path().join(".brokkr/recipes/self/tooling.json"),
            serde_json::to_vec(record).unwrap(),
        )
        .unwrap();
    }

    fn digest(&self) -> String {
        sha256_file(self.engine.to_str().unwrap())
    }

    fn command(&self, body: &str) -> Command {
        let mut command = Command::new("bash");
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let path = std::env::join_paths(
            std::iter::once(self.bin.clone()).chain(std::env::split_paths(&current_path)),
        )
        .unwrap();
        command
            .args(["-c", body])
            .current_dir(self.root.path())
            .env("PATH", path)
            .env("GITHUB_OUTPUT", &self.github_output)
            .env("RUNNER_TEMP", &self.runner_temp)
            .env("BINARY_URL", PUBLISHED_ENGINE_URL)
            .env("BINARY_SHA256", self.digest())
            .env("RECORDED_VERSION", "0.12.0")
            .env("FIXTURE_ASSET", &self.engine)
            .env("ENGINE_MARKER", &self.marker)
            .env("CURL_ARGS", self.root.path().join("curl args.txt"))
            .env_remove("TRANSPORT_FAIL")
            .env_remove("COMPILE_MODE")
            .env_remove("FIXTURE_VERSION")
            .env_remove("FAIL_RECIPE");
        command
    }

    fn assert_cleaned(&self) {
        assert_eq!(std::fs::read_dir(&self.runner_temp).unwrap().count(), 0);
    }
}

fn ci_diagnostics(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn ci_record_reader_refuses_bad_fields_before_exporting_any_output() {
    let body = brokkr_ci_body(READ_TOOLING_STEP);
    let fixture = BrokkrCiFixture::new();
    let valid = read_json(".brokkr/recipes/self/tooling.json");
    for (field, invalids) in [
        ("sha", vec![json!("0".repeat(40) + "\n"), json!(7)]),
        ("version", vec![json!("0.12.0\n"), json!("v0.12.0")]),
        (
            "binary_sha256",
            vec![
                Value::Null,
                json!([]),
                json!(7),
                json!(""),
                json!("a".repeat(63)),
                json!("A".repeat(64)),
                json!(format!("{PUBLISHED_ENGINE_DIGEST}\n")),
                json!(format!("{PUBLISHED_ENGINE_DIGEST}\0")),
            ],
        ),
        (
            "binary_url",
            vec![
                Value::Null,
                json!([]),
                json!(7),
                json!(""),
                json!(PUBLISHED_ENGINE_URL.replace("https:", "http:")),
                json!(PUBLISHED_ENGINE_URL.replace("github.com", "github.com.evil.invalid")),
                json!(format!("{PUBLISHED_ENGINE_URL}\n")),
                json!(format!("{PUBLISHED_ENGINE_URL}\r")),
                json!(format!("{PUBLISHED_ENGINE_URL}\0")),
            ],
        ),
    ] {
        for invalid in std::iter::once(None).chain(invalids.into_iter().map(Some)) {
            let mut record = valid.clone();
            if let Some(value) = invalid {
                record["brokkr"][field] = value;
            } else {
                record["brokkr"].as_object_mut().unwrap().remove(field);
            }
            fixture.write_record(&record);
            let output = fixture.command(&body).output().unwrap();
            assert!(!output.status.success(), "admitted bad {field}: {record}");
            assert!(ci_diagnostics(&output).contains(&format!("brokkr.{field}")));
            assert!(!fixture.github_output.exists(), "exported invalid metadata");
        }
    }
    let record_path = fixture
        .root
        .path()
        .join(".brokkr/recipes/self/tooling.json");
    for malformed in ["{", "{}", "null", r#"{"brokkr":[]}"#] {
        std::fs::write(&record_path, malformed).unwrap();
        let output = fixture.command(&body).output().unwrap();
        assert!(!output.status.success());
        assert!(ci_diagnostics(&output).contains("REFUSAL:"));
        assert!(!fixture.github_output.exists());
    }
    // jq normally accepts a stream of documents; a final valid document
    // must not mask earlier malformed fields or create multiline outputs.
    for concatenated in [format!("{{}}\n{valid}"), format!("{valid}\n{valid}")] {
        std::fs::write(&record_path, concatenated).unwrap();
        let output = fixture.command(&body).output().unwrap();
        assert!(!output.status.success());
        assert!(ci_diagnostics(&output).contains("REFUSAL:"));
        assert!(!fixture.github_output.exists());
    }
    std::fs::remove_file(record_path).unwrap();
    let output = fixture.command(&body).output().unwrap();
    assert!(!output.status.success());
    assert!(ci_diagnostics(&output).contains("tooling.json is absent"));
    assert!(!fixture.github_output.exists());

    fixture.write_record(&valid);
    let output = fixture.command(&body).output().unwrap();
    assert!(output.status.success(), "{}", ci_diagnostics(&output));
    assert_eq!(
        std::fs::read_to_string(&fixture.github_output).unwrap(),
        format!(
            "sha=a9893b4dfb67417edc99d91f941a9bac3856f138\nversion=0.12.0\nbinary_sha256={PUBLISHED_ENGINE_DIGEST}\nbinary_url={PUBLISHED_ENGINE_URL}\n"
        )
    );
    assert!(!fixture.marker.exists());
}

#[test]
fn ci_admits_digest_before_executing_and_compiles_all_six_recipes() {
    let body = brokkr_ci_body(ADMIT_ENGINE_STEP);
    let fixture = BrokkrCiFixture::new();
    let mismatch = fixture
        .command(&body)
        .env("BINARY_SHA256", PUBLISHED_ENGINE_DIGEST)
        .output()
        .unwrap();
    assert!(!mismatch.status.success());
    let diagnostics = ci_diagnostics(&mismatch);
    assert!(diagnostics.contains("REFUSAL: brokkr binary_sha256 mismatch"));
    assert!(diagnostics.contains(&format!("expected {PUBLISHED_ENGINE_DIGEST}")));
    assert!(diagnostics.contains(&format!("observed {}", fixture.digest())));
    assert!(!fixture.marker.exists(), "mismatched engine executed");
    fixture.assert_cleaned();

    // Removing admission in a scratch body executes the wrong bytes: the
    // no-execution oracle discriminates this broken control flow.
    let mutant = body.replace(
        "if [ \"$observed\" != \"$BINARY_SHA256\" ]; then",
        "if false; then",
    );
    assert_ne!(body, mutant);
    let output = fixture
        .command(&mutant)
        .env("BINARY_SHA256", PUBLISHED_ENGINE_DIGEST)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", ci_diagnostics(&output));
    assert!(fixture.marker.exists(), "mutant did not expose execution");
    std::fs::remove_file(&fixture.marker).unwrap();

    let six = "--version\ncompile --bundle .brokkr/recipes/self\ncompile --bundle .brokkr/recipes/codex\ncompile --bundle .brokkr/recipes/codex-careful\ncompile --bundle .brokkr/recipes/codex-fast\ncompile --bundle .brokkr/recipes/mixed-opus-terra\ncompile --bundle .brokkr/recipes/claude-careful\n";
    // Dropping the final recipe from the loop still exits zero: only the exact
    // trace below catches the omission.
    let omitted = body.replace(
        "mixed-opus-terra claude-careful; do",
        "mixed-opus-terra; do",
    );
    assert_ne!(body, omitted);
    let output = fixture.command(&omitted).output().unwrap();
    assert!(output.status.success(), "{}", ci_diagnostics(&output));
    assert_ne!(std::fs::read_to_string(&fixture.marker).unwrap(), six);
    std::fs::remove_file(&fixture.marker).unwrap();

    let output = fixture.command(&body).output().unwrap();
    assert!(output.status.success(), "{}", ci_diagnostics(&output));
    assert_eq!(std::fs::read_to_string(&fixture.marker).unwrap(), six);
    let args = std::fs::read_to_string(fixture.root.path().join("curl args.txt")).unwrap();
    for required in [
        "--fail\n",
        "--location\n",
        "--proto\n=https\n",
        "--proto-redir\n=https\n",
        "--connect-timeout\n10\n",
        "--max-time\n120\n",
        PUBLISHED_ENGINE_URL,
    ] {
        assert!(args.contains(required), "curl omitted {required:?}: {args}");
    }
    fixture.assert_cleaned();
}

#[test]
fn ci_refuses_failed_download_or_hash_before_execution() {
    let body = brokkr_ci_body(ADMIT_ENGINE_STEP);
    for failure in ["download", "hash"] {
        let fixture = BrokkrCiFixture::new();
        let mut command = fixture.command(&body);
        if failure == "download" {
            command.env("TRANSPORT_FAIL", "1");
        } else {
            fixture.write_command(
                &fixture.bin.join("sha256sum"),
                "#!/usr/bin/env bash\nexit 19\n",
            );
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        let expected = if failure == "download" {
            "binary_url download failed"
        } else {
            "binary_sha256 could not be checked"
        };
        assert!(
            ci_diagnostics(&output).contains(expected),
            "{}",
            ci_diagnostics(&output)
        );
        assert!(
            !fixture.marker.exists(),
            "engine executed after {failure} failure"
        );
        fixture.assert_cleaned();
    }
}

#[test]
fn ci_refuses_warning_on_either_stream_and_preserves_compile_failure_status() {
    let body = brokkr_ci_body(ADMIT_ENGINE_STEP);
    // The final recipe gets its own failures: nothing after it could mask them,
    // and every earlier recipe compiled clean first.
    for (recipe, later) in [
        (
            ".brokkr/recipes/codex-careful",
            Some(".brokkr/recipes/codex-fast"),
        ),
        (
            ".brokkr/recipes/mixed-opus-terra",
            Some(".brokkr/recipes/claude-careful"),
        ),
        (".brokkr/recipes/claude-careful", None),
    ] {
        for mode in ["stdout-warning", "stderr-warning", "failure"] {
            let fixture = BrokkrCiFixture::new();
            let output = fixture
                .command(&body)
                .env("FAIL_RECIPE", recipe)
                .env("COMPILE_MODE", mode)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(if mode == "failure" { 23 } else { 1 })
            );
            let diagnostics = ci_diagnostics(&output);
            assert!(diagnostics.contains(recipe), "{diagnostics}");
            let expected = if mode == "failure" {
                "fixture compile failure"
            } else {
                "policy-sweep warning"
            };
            assert!(diagnostics.contains(expected), "{diagnostics}");
            let marker = std::fs::read_to_string(&fixture.marker).unwrap();
            assert!(marker.ends_with(&format!("compile --bundle {recipe}\n")));
            if let Some(later) = later {
                assert!(!marker.contains(later));
            }
            fixture.assert_cleaned();
        }
    }
    let fixture = BrokkrCiFixture::new();
    let output = fixture
        .command(&body)
        .env("FIXTURE_VERSION", "0.0.0")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(ci_diagnostics(&output).contains("admitted brokkr version"));
    assert_eq!(
        std::fs::read_to_string(&fixture.marker).unwrap(),
        "--version\n"
    );
    fixture.assert_cleaned();
}

/// The step of a CI job whose `name:` is `name`, from its `- name:` line to
/// the next step.
fn ci_step<'a>(job: &'a str, name: &str) -> &'a str {
    let marker = format!("      - name: {name}\n");
    let start = job
        .find(&marker)
        .unwrap_or_else(|| panic!("the test job has no step named {name:?}"));
    let rest = &job[start + marker.len()..];
    &rest[..rest.find("\n      - ").unwrap_or(rest.len())]
}

/// Issue #162, proposed 0110 §6: each client fixture's example suite is a
/// named Linux step of the required `test` job (a new job would not be a
/// required context on main), built `--locked`, against a server this step
/// starts, with the CI-only framework installed scripts-off and
/// binaries-only. Red when a needle goes, when a step leaves the job or
/// loses its Linux guard, and when the two suites stop being the same six
/// examples.
#[test]
fn ci_runs_both_client_example_suites_in_the_linux_test_job() {
    let ci = read_text(".github/workflows/ci.yml");
    let job = ci
        .split("\n  test:\n")
        .nth(1)
        .and_then(|rest| rest.split("\n  coverage:\n").next())
        .expect("ci.yml has a test job before coverage");
    for (name, needles) in [
        (
            "Playwright fixture example suite",
            &[
                "cargo build --locked",
                "target/debug/swarmail serve",
                "http://127.0.0.1:8025/healthz",
                "npm install --prefix clients/playwright --ignore-scripts",
                "--no-package-lock",
                "playwright test -c clients/playwright/examples",
            ][..],
        ),
        (
            "pytest fixture example suite",
            &[
                "cargo build --locked",
                "target/debug/swarmail serve",
                "http://127.0.0.1:8025/healthz",
                "pip install --no-deps --only-binary :all: -r clients/pytest/requirements-ci.txt",
                "-m pytest -p pytest_swarmail.plugin",
                "clients/pytest/examples",
            ][..],
        ),
    ] {
        let step = ci_step(job, name);
        assert!(
            step.contains("if: runner.os == 'Linux'"),
            "{name} is Linux-only"
        );
        for needle in needles {
            assert!(step.contains(needle), "{name} must carry {needle:?}");
        }
        assert!(
            step.contains("set -euo pipefail") && step.contains("trap "),
            "{name} fails closed and stops its server"
        );
    }

    let spec = read_text("clients/playwright/examples/example.spec.ts");
    let suite = read_text("clients/pytest/examples/test_example.py");
    let numbered = |text: &str, prefix: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| l.strip_prefix(prefix))
            .map(|l| l.chars().take(1).collect())
            .collect()
    };
    let expected: Vec<String> = (1..=6).map(|n| n.to_string()).collect();
    assert_eq!(
        numbered(&spec, "test('"),
        expected[..5],
        "spec.ts examples 1-5"
    );
    assert_eq!(
        numbered(&spec, "test(\""),
        expected[5..],
        "spec.ts example 6"
    );
    assert_eq!(numbered(&suite, "def test_"), expected, "pytest examples");
}
