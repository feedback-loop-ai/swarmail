//! Offline product and control admission proofs. The controls use real servers;
//! no provider executable or external-network agent is invoked.
mod common;
use common::*;
use std::{io::Write, path::Path, process::Command};
use swarmail_evals::{
    artifacts, baseline,
    provenance::*,
    report,
    tasks::{self, Outcome},
};

#[test]
fn dry_run_uses_the_exact_approved_product_without_reading_config_or_creating_output() {
    let dir = tempdir();
    let output = dir.path().join("never-created");
    // A directory cannot be parsed as a configuration file. dry-run must
    // return without opening it; no test-only behavior is required.
    let config = dir.path().join("not-a-config");
    std::fs::create_dir(&config).unwrap();
    let result = Command::new(exe())
        .args(["baseline", "--dry-run", "--config"])
        .arg(&config)
        .arg("--output")
        .arg(&output)
        .env("ANTHROPIC_API_KEY", "private-canary-never-a-public-input")
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result);
    assert!(!output.exists());
    let plan: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    // The approved product is the sixteen default-tier tasks, in catalogue
    // order; the hard and expert tiers are separate, operator-selected
    // products. The filter is a positive list of the three default tiers, so
    // a tier added later never joins the approved product.
    let default_tier: Vec<_> = tasks::catalogue()
        .into_iter()
        .filter(|t| tasks::DEFAULT_TIERS.contains(&t.tier))
        .map(|t| t.id)
        .collect();
    assert_eq!(default_tier.len(), 16);
    assert_eq!(plan["tasks"], serde_json::to_value(&default_tier).unwrap());
    assert_eq!(plan["control_rows"], 32);
    assert_eq!(plan["paid_rows"], 480);
    assert_eq!(plan["deadline_ms"], baseline::DEADLINE_MS);
    assert_eq!(plan["concurrency"], 1);
    assert_eq!(plan["control_repetitions"], 1);
    assert_eq!(plan["paid_repetitions"], 5);
    assert_eq!(plan["steps"][0], "fresh free control matrix");
    assert_eq!(
        plan["steps"][1],
        "validate every control and retained matrix"
    );
    assert_eq!(
        plan["steps"][3],
        "task-major paid chunks, each started only while its credential has the margin left"
    );
    assert_eq!(
        plan["report_argv"],
        serde_json::json!([
            "swarmail-evals",
            "report",
            "--input",
            output.join("paid.jsonl")
        ])
    );
    assert!(
        !String::from_utf8(result.stdout)
            .unwrap()
            .contains("private-canary")
    );
    assert!(
        !String::from_utf8(result.stderr)
            .unwrap()
            .contains("private-canary")
    );
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 1);
}

#[test]
fn declarations_have_the_exact_six_models_and_private_literal_config_handoff() {
    let plan = baseline::Plan::new(Path::new("/tmp/result"));
    let expected = [
        ("claude-opus", "agent-claude", "claude-opus-5-5"),
        ("claude-sonnet", "agent-claude", "claude-sonnet-5-5"),
        ("codex-sol", "agent-codex", "gpt-6.1-sol"),
        ("codex-luna", "agent-codex", "gpt-6-luna"),
        ("glm-dashscope", "agent-dsh", "dashscope/glm-5.3"),
        ("glm-spark", "agent-dsh", "spark-glm/GLM-5.3-Flash-EXL3"),
    ];
    assert_eq!(plan.agents.len(), expected.len());
    for (a, (label, command, model)) in plan.agents.iter().zip(expected) {
        assert_eq!(a.label, label);
        assert_eq!(a.model.as_deref(), Some(model));
        assert_eq!(a.argv, ["swarmail-evals", command, "--model", model]);
        assert!(!a.public_argv);
        assert!(a.config_id.is_none());
    }
    let config = Path::new("/private/config with spaces;$(unexpanded)`literal`.json");
    let effective = plan
        .effective_agents(Path::new("/private/eval executable"), config)
        .unwrap();
    for a in effective {
        assert_eq!(a.argv[0], "/private/eval executable");
        assert_eq!(&a.argv[4..], ["--config", config.to_str().unwrap()]);
        assert!(!a.public_argv);
    }
    assert!(!serde_json::to_string(&plan).unwrap().contains("unexpanded"));
    assert_eq!(report::parse_agents(baseline::AGENTS).unwrap(), plan.agents);
}

fn dry_run(tier: &[&str]) -> (Option<i32>, serde_json::Value, String) {
    let dir = tempdir();
    let config = dir.path().join("not-a-config");
    std::fs::create_dir(&config).unwrap();
    let out = Command::new(exe())
        .args(["baseline", "--dry-run", "--config"])
        .arg(&config)
        .arg("--output")
        .arg(dir.path().join("never-created"))
        .args(tier)
        .output()
        .unwrap();
    (
        out.status.code(),
        serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// AC4: `--tier hard` is its own product with the same discipline, and the
/// default product is untouched by its existence.
#[test]
fn the_hard_tier_is_a_separate_baseline_product_with_the_same_discipline() {
    let (code, default, _) = dry_run(&[]);
    let (hard_code, hard, _) = dry_run(&["--tier", "hard"]);
    assert_eq!((code, hard_code), (Some(0), Some(0)));
    assert_eq!(
        hard["tasks"],
        serde_json::json!(["otp-backlog", "recipient-crowd", "digest-lookalike"])
    );
    assert_eq!(hard["control_rows"], 6);
    assert_eq!(hard["paid_rows"], 90);
    assert_eq!(default["tasks"].as_array().unwrap().len(), 16);
    assert_eq!(
        (
            default["control_rows"].clone(),
            default["paid_rows"].clone()
        ),
        (32.into(), 480.into())
    );
    // Everything but the task list and its cardinalities is shared.
    for key in [
        "deadline_ms",
        "concurrency",
        "control_repetitions",
        "paid_repetitions",
        "steps",
        "files",
        "agents",
        "control_agents",
        "catalogue_schema",
        "catalogue_sha256",
        "wrapper_contracts",
    ] {
        assert_eq!(hard[key], default[key], "{key}");
    }
    // A default-tier tier selection plans exactly that tier.
    let (_, straight, _) = dry_run(&["--tier", "straight"]);
    assert_eq!(
        straight["tasks"],
        serde_json::json!(["signup", "otp", "magic-login"])
    );
    // Hashes: the plan carries the full catalogue's, not the historical one.
    assert_eq!(hard["catalogue_sha256"], p_hash());
}

fn p_hash() -> String {
    catalogue_hash()
}

#[test]
fn an_unknown_baseline_tier_is_a_usage_error_with_no_plan() {
    for bad in ["harder", "Hard", "multi-step", ""] {
        let (code, plan, err) = dry_run(&["--tier", bad]);
        assert_eq!(code, Some(2), "{bad:?}");
        assert!(plan.is_null(), "{bad:?}: no plan on stdout");
        assert!(err.contains("arguments_invalid"), "{bad:?}: {err}");
    }
}

/// Controls come first for the hard product too: six real control trials
/// (oracle passes, null fails with its idle class) are retained and admitted
/// before anything paid, and a bad manifest still stops short of a paid slot.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn hard_tier_controls_run_for_real_before_any_paid_launch() {
    let _scope = trial_scope();
    // Controls run for real; every login is inside the margin, so the run
    // waits with its reason and is interrupted before any paid slot.
    let paid = common::paid::Paid::for_tier(tasks::Tier::Hard);
    paid.controls_then_interrupted_wait();
    let output = paid.output.clone();
    assert!(!output.join("paid.jsonl").exists());
    assert!(!output.join("report.json").exists());
    let stream = std::fs::read(output.join("controls.jsonl")).unwrap();
    let rows = baseline::rows(&stream).unwrap();
    assert_eq!(rows.len(), 6);
    let tasks_run: Vec<_> = rows.iter().map(|r| r.task.clone().unwrap()).collect();
    assert_eq!(
        tasks_run,
        [
            "otp-backlog",
            "otp-backlog",
            "recipient-crowd",
            "recipient-crowd",
            "digest-lookalike",
            "digest-lookalike"
        ]
    );
    for pair in rows.chunks(2) {
        assert_eq!(pair[0].agent.as_deref(), Some("oracle"));
        assert_eq!(pair[0].outcome, Outcome::Pass);
        assert_eq!(pair[1].agent.as_deref(), Some("null"));
        assert_eq!(pair[1].outcome, Outcome::Fail);
        assert_eq!(
            pair[1].failure_class,
            Some(swarmail_evals::state::FailureClass::GaveUp)
        );
    }
    let root = std::fs::read_dir(output.join("controls"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let matrix: artifacts::MatrixPlan =
        decode(&artifacts::read_file(&root, "matrix-plan.json", artifacts::PLAN_LIMIT).unwrap())
            .unwrap();
    assert_eq!(
        matrix.tasks,
        tasks_run.iter().step_by(2).cloned().collect::<Vec<_>>()
    );
    let plan = baseline::Plan::for_tier(Some(tasks::Tier::Hard), &output);
    let harness = rows[0].provenance.harness.clone();
    let p = &rows[0].provenance.target;
    let target = ExecutableIdentity {
        executable_sha256: p.executable_sha256.clone(),
        build_commit: p.build_commit.clone(),
        source_dirty: p.source_dirty.clone(),
    };
    baseline::validate_controls(&rows, &matrix, &plan, 1, &harness, &target).unwrap();
    // The default product's plan refuses these controls: wrong task list.
    let default_plan = baseline::Plan::new(&output);
    assert!(
        baseline::validate_controls(&rows, &matrix, &default_plan, 1, &harness, &target).is_err()
    );
    // Corrupt retained control evidence never authorizes a paid slot: the
    // resume refuses the gate instead of rerunning or trusting it.
    let trial = std::fs::read_dir(root.join("trials"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let result = trial.join("result.json");
    let mut bytes = std::fs::read(&result).unwrap();
    bytes[10] ^= 1;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&result)
        .unwrap()
        .write_all(&bytes)
        .unwrap();
    paid.claude(common::paid::now() + 24 * common::paid::HOUR);
    let out = paid.authorized(&["--resume"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("baseline_control_retention_invalid"),
        "{out:?}"
    );
    assert!(paid.launches().is_empty());
}

/// AC12: `--tier expert` is its own product, 5 tasks, 10 controls and 150
/// paid slots, sharing the approved deadline, concurrency, steps and files;
/// the default product and the hard product are untouched by it.
#[test]
fn the_expert_tier_is_a_separate_baseline_product_with_the_same_discipline() {
    let (code, default, _) = dry_run(&[]);
    let (hard_code, hard, _) = dry_run(&["--tier", "hard"]);
    let (expert_code, expert, _) = dry_run(&["--tier", "expert"]);
    assert_eq!((code, hard_code, expert_code), (Some(0), Some(0), Some(0)));
    assert_eq!(
        expert["tasks"],
        serde_json::json!([
            "otp-correction",
            "digest-correction",
            "otp-late",
            "confirm-attachment",
            "confirm-injection"
        ])
    );
    assert_eq!(expert["control_rows"], 10);
    assert_eq!(expert["paid_rows"], 150);
    // The approved product is still the sixteen: 32 controls, 480 slots.
    assert_eq!(default["tasks"].as_array().unwrap().len(), 16);
    assert_eq!(
        (
            default["control_rows"].clone(),
            default["paid_rows"].clone()
        ),
        (32.into(), 480.into())
    );
    assert_eq!(hard["control_rows"], 6);
    assert_eq!(hard["paid_rows"], 90);
    for key in [
        "deadline_ms",
        "concurrency",
        "control_repetitions",
        "paid_repetitions",
        "steps",
        "files",
        "agents",
        "control_agents",
        "catalogue_schema",
        "catalogue_sha256",
        "wrapper_contracts",
    ] {
        assert_eq!(expert[key], default[key], "{key}");
    }
    assert_eq!(expert["catalogue_sha256"], p_hash());
    // The tier flag stays strict about its spelling.
    for bad in ["Expert", "experts", "expert-tier"] {
        let (code, plan, err) = dry_run(&["--tier", bad]);
        assert_eq!(code, Some(2), "{bad:?}");
        assert!(plan.is_null(), "{bad:?}: no plan on stdout");
        assert!(err.contains("arguments_invalid"), "{bad:?}: {err}");
    }
}

/// Controls come first for the expert product too: ten real control trials
/// are retained and admitted before anything paid, and the default product's
/// plan refuses them.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn expert_tier_controls_run_for_real_before_any_paid_launch() {
    let _scope = trial_scope();
    // Controls run for real; every login is inside the margin, so the run
    // waits with its reason and is interrupted before any paid slot.
    let paid = common::paid::Paid::for_tier(tasks::Tier::Expert);
    paid.controls_then_interrupted_wait();
    let output = paid.output.clone();
    assert!(!output.join("paid.jsonl").exists());
    assert!(!output.join("report.json").exists());
    let stream = std::fs::read(output.join("controls.jsonl")).unwrap();
    let rows = baseline::rows(&stream).unwrap();
    assert_eq!(rows.len(), 10);
    let tasks_run: Vec<_> = rows.iter().map(|r| r.task.clone().unwrap()).collect();
    let expert = tasks::tier_ids(tasks::Tier::Expert);
    let doubled: Vec<String> = expert
        .iter()
        .flat_map(|id| [id.clone(), id.clone()])
        .collect();
    assert_eq!(tasks_run, doubled);
    for pair in rows.chunks(2) {
        assert_eq!(pair[0].agent.as_deref(), Some("oracle"));
        assert_eq!(pair[0].outcome, Outcome::Pass);
        assert_eq!(pair[1].agent.as_deref(), Some("null"));
        assert_eq!(pair[1].outcome, Outcome::Fail);
        assert_eq!(
            pair[1].failure_class,
            Some(swarmail_evals::state::FailureClass::GaveUp)
        );
    }
    let root = std::fs::read_dir(output.join("controls"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let matrix: artifacts::MatrixPlan =
        decode(&artifacts::read_file(&root, "matrix-plan.json", artifacts::PLAN_LIMIT).unwrap())
            .unwrap();
    assert_eq!(matrix.tasks, expert);
    let plan = baseline::Plan::for_tier(Some(tasks::Tier::Expert), &output);
    let harness = rows[0].provenance.harness.clone();
    let p = &rows[0].provenance.target;
    let target = ExecutableIdentity {
        executable_sha256: p.executable_sha256.clone(),
        build_commit: p.build_commit.clone(),
        source_dirty: p.source_dirty.clone(),
    };
    baseline::validate_controls(&rows, &matrix, &plan, 1, &harness, &target).unwrap();
    let default_plan = baseline::Plan::new(&output);
    assert!(
        baseline::validate_controls(&rows, &matrix, &default_plan, 1, &harness, &target).is_err()
    );
}

/// Proposed 0131: `--tier deliverability` is its own product, 1 task, 2
/// controls and 30 paid slots, sharing the approved deadline, concurrency,
/// steps and files; the default, hard and expert products are untouched.
#[test]
fn the_deliverability_tier_is_a_separate_baseline_product_with_the_same_discipline() {
    let (code, default, _) = dry_run(&[]);
    let (hard_code, hard, _) = dry_run(&["--tier", "hard"]);
    let (expert_code, expert, _) = dry_run(&["--tier", "expert"]);
    let (del_code, del, _) = dry_run(&["--tier", "deliverability"]);
    assert_eq!(
        (code, hard_code, expert_code, del_code),
        (Some(0), Some(0), Some(0), Some(0))
    );
    assert_eq!(del["tasks"], serde_json::json!(["newsletter-findings"]));
    assert_eq!(del["control_rows"], 2);
    assert_eq!(del["paid_rows"], 30);
    assert_eq!(
        (
            default["control_rows"].clone(),
            default["paid_rows"].clone()
        ),
        (32.into(), 480.into())
    );
    assert_eq!(default["tasks"].as_array().unwrap().len(), 16);
    assert_eq!(
        (hard["control_rows"].clone(), hard["paid_rows"].clone()),
        (6.into(), 90.into())
    );
    assert_eq!(
        (expert["control_rows"].clone(), expert["paid_rows"].clone()),
        (10.into(), 150.into())
    );
    for key in [
        "deadline_ms",
        "concurrency",
        "control_repetitions",
        "paid_repetitions",
        "steps",
        "files",
        "agents",
        "control_agents",
        "catalogue_schema",
        "catalogue_sha256",
        "wrapper_contracts",
    ] {
        assert_eq!(del[key], default[key], "{key}");
    }
    assert_eq!(del["catalogue_sha256"], p_hash());
    for bad in ["Deliverability", "deliverable", "deliverability-tier"] {
        let (code, plan, err) = dry_run(&["--tier", bad]);
        assert_eq!(code, Some(2), "{bad:?}");
        assert!(plan.is_null(), "{bad:?}: no plan on stdout");
        assert!(err.contains("arguments_invalid"), "{bad:?}: {err}");
    }
}
