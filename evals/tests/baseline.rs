//! Offline product and control admission proofs. The controls use real servers;
//! no provider executable or external-network agent is invoked.
mod common;
use common::*;
use std::{path::Path, process::Command};
use swarmail_evals::{
    artifacts, baseline,
    provenance::*,
    report,
    runner::TrialResult,
    tasks::{self, Outcome},
};

fn bytes(rows: &[TrialResult]) -> Vec<u8> {
    rows.iter()
        .flat_map(|r| {
            let mut b = serde_json::to_vec(r).unwrap();
            b.push(b'\n');
            b
        })
        .collect()
}
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
    assert_eq!(
        plan["tasks"],
        serde_json::to_value(tasks::catalogue().iter().map(|t| &t.id).collect::<Vec<_>>()).unwrap()
    );
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
    assert_eq!(plan["steps"][3], "paid matrix");
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

#[test]
fn real_controls_are_retained_and_bad_controls_never_authorize_paid_launch() {
    let _scope = trial_scope();
    let dir = tempdir();
    let output = dir.path().join("baseline");
    let config = dir.path().join("private.json");
    // Invalid manifest contents fail the post-control selection check.
    std::fs::write(&config, "synthetic-secret-canary").unwrap();
    let result = cli()
        .args(["baseline", "--config"])
        .arg(&config)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    // The credential-free selection is invalid: fail after real controls,
    // before admitting any paid slot. Credential files are never opened here.
    assert!(!output.join("paid.jsonl").exists());
    assert!(!output.join("paid").exists());
    assert!(!output.join("report.json").exists());
    assert!(!String::from_utf8_lossy(&result.stderr).contains("synthetic-secret-canary"));
    let stream = std::fs::read(output.join("controls.jsonl")).unwrap();
    assert!(!String::from_utf8_lossy(&stream).contains("synthetic-secret-canary"));
    let rows = baseline::rows(&stream).unwrap();
    assert_eq!(rows.len(), 32);
    let root = std::fs::read_dir(output.join("controls"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let matrix: artifacts::MatrixPlan =
        decode(&artifacts::read_file(&root, "matrix-plan.json", artifacts::PLAN_LIMIT).unwrap())
            .unwrap();
    let plan = baseline::Plan::new(&output);
    let harness = rows[0].provenance.harness.clone();
    let p = &rows[0].provenance.target;
    let target = ExecutableIdentity {
        executable_sha256: p.executable_sha256.clone(),
        build_commit: p.build_commit.clone(),
        source_dirty: p.source_dirty.clone(),
    };
    baseline::validate_controls(&rows, &matrix, &plan, 1, &harness, &target).unwrap();
    let replay = artifacts::matrix(&root).unwrap();
    assert_eq!(replay.len(), rows.len());
    assert!(
        replay
            .iter()
            .zip(&rows)
            .all(|(a, b)| a.original_result == *b)
    );
    let original = stream.clone();
    let value: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();
    let cases = [
        "missing",
        "duplicate",
        "extra",
        "reorder",
        "malformed",
        "truncated",
        "blank",
        "wrong_class",
        "oracle_fail",
        "null_pass",
        "infra",
        "foreign_trial",
        "foreign_matrix",
        "wrong_task",
        "wrong_agent",
        "wrong_catalogue",
        "wrong_model",
        "wrong_config",
        "wrong_deadline",
        "wrong_limits",
        "wrong_target",
        "wrong_harness",
        "wrong_concurrency",
        "duplicate_key",
    ];
    for case in cases {
        let mut v = value.clone();
        match case {
            "missing" => {
                v.pop();
            }
            "duplicate" => v[1] = v[0].clone(),
            "extra" => v.push(v[0].clone()),
            "reorder" => v.swap(0, 1),
            "wrong_class" => v[1]["failure_class"] = serde_json::json!("wrong_message"),
            "oracle_fail" => {
                v[0]["outcome"] = serde_json::json!("fail");
                v[0]["failure_class"] = serde_json::json!("gave_up");
            }
            "null_pass" => {
                v[1]["outcome"] = serde_json::json!("pass");
                v[1]["failure_class"] = serde_json::Value::Null;
            }
            "infra" => {
                v[0]["outcome"] = serde_json::json!("harness_error");
                v[0]["failure_class"] = serde_json::json!("harness_error");
            }
            "foreign_trial" => v[0]["trial_id"] = serde_json::json!("a".repeat(32)),
            "foreign_matrix" => {
                v[0]["provenance"]["matrix"]["matrix_id"] = serde_json::json!("b".repeat(32))
            }
            "wrong_task" => v[0]["task"] = serde_json::json!("otp"),
            "wrong_agent" => v[0]["agent"] = serde_json::json!("null"),
            "wrong_catalogue" => {
                v[0]["provenance"]["task"]["catalogue_sha256"] = serde_json::json!("a".repeat(64))
            }
            "wrong_model" => {
                v[0]["provenance"]["agent"]["model"] = serde_json::json!({"status":"present","value":"wrong","source":"operator_declared"})
            }
            "wrong_config" => {
                v[0]["provenance"]["agent"]["config_id"]["value"] = serde_json::json!("wrong")
            }
            "wrong_deadline" => {
                v[0]["provenance"]["limits"]["agent_deadline_ms"]["value"] =
                    serde_json::json!(60_000)
            }
            "wrong_limits" => v[0]["provenance"]["limits"]["startup_ms"] = serde_json::json!(1),
            "wrong_target" => {
                v[0]["provenance"]["target"]["executable_sha256"]["value"] =
                    serde_json::json!("a".repeat(64))
            }
            "wrong_harness" => {
                v[0]["provenance"]["harness"]["executable_sha256"]["value"] =
                    serde_json::json!("a".repeat(64))
            }
            "wrong_concurrency" => v[0]["concurrency"] = serde_json::json!(2),
            _ => {}
        }
        let mut stream: Vec<u8> = v
            .iter()
            .flat_map(|r| {
                let mut b = serde_json::to_vec(r).unwrap();
                b.push(b'\n');
                b
            })
            .collect();
        match case {
            "malformed" => stream.extend_from_slice(b"not-json\n"),
            "truncated" => {
                stream.pop();
            }
            "blank" => stream.extend_from_slice(b"\n"),
            "duplicate_key" => stream
                .splice(1..1, b"\"schema_version\":2,".iter().copied())
                .for_each(drop),
            _ => {}
        }
        let mut paid_launches = 0;
        let admission = baseline::rows(&stream)
            .and_then(|r| baseline::validate_controls(&r, &matrix, &plan, 1, &harness, &target));
        if admission.is_ok() {
            paid_launches += 1;
        }
        assert_eq!(paid_launches, 0, "{case}");
        assert!(admission.is_err(), "{case}");
    }
    for status in [0, 2, 3, 127, 130, 143, 255] {
        assert!(
            baseline::validate_controls(&rows, &matrix, &plan, status, &harness, &target).is_err()
        );
    }
    assert_eq!(
        std::fs::read(output.join("controls.jsonl")).unwrap(),
        original
    );
    assert_eq!(bytes(&rows), original);
    // Never overwrite a prior or partial result, including existing modes.
    let before = std::fs::read(output.join("controls.jsonl")).unwrap();
    let again = cli()
        .args(["baseline", "--config"])
        .arg(&config)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(again.status.code(), Some(2));
    assert_eq!(
        std::fs::read(output.join("controls.jsonl")).unwrap(),
        before
    );
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(output.join("controls.jsonl"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    // Full synthetic paid accounting exercises the exact 480 coordinates and
    // declared models without invoking any provider. Reports retain all rows.
    paid_accounting(&rows[0], &plan, &harness, &target);
}

fn paid_accounting(
    template: &TrialResult,
    plan: &baseline::Plan,
    harness: &HarnessIdentity,
    target: &ExecutableIdentity,
) {
    let matrix_id = "f".repeat(32);
    let master = template.provenance.fixture.clone();
    let mut matrix = artifacts::MatrixPlan {
        schema_version: 1,
        matrix_id: matrix_id.clone(),
        master: master.clone(),
        tasks: plan.tasks.clone(),
        agents: plan.agents.iter().map(|a| a.label.clone()).collect(),
        repetitions: 5,
        cells: Vec::new(),
    };
    let mut paid = Vec::new();
    for (ti, task) in plan.tasks.iter().enumerate() {
        for (ai, agent) in plan.agents.iter().enumerate() {
            for ri in 0..5 {
                let coordinates = MatrixCoordinates {
                    matrix_id: matrix_id.clone(),
                    master_seed: master.seed.clone(),
                    task_index: ti as u64,
                    agent_index: ai as u64,
                    repetition_index: ri,
                };
                let mut fixture = master.clone();
                fixture.seed = coordinates.cell_seed(task, &agent.label).unwrap();
                let trial_id = format!("{:032x}", paid.len() + 1);
                let cell = artifacts::PlannedCell {
                    trial_id: trial_id.clone(),
                    task: task.clone(),
                    agent: agent.label.clone(),
                    fixture: fixture.clone(),
                    coordinates: coordinates.clone(),
                    deadline_ms: baseline::DEADLINE_MS,
                };
                let mut row = template.clone();
                row.trial_id = trial_id;
                row.task = Some(task.clone());
                row.agent = Some(agent.label.clone());
                row.provenance.fixture = fixture;
                row.provenance.matrix = Some(coordinates);
                row.provenance.task.definition_sha256 =
                    Availability::present(task_hash(task).unwrap(), Source::CatalogueDefinition);
                row.provenance.agent.kind = AgentKind::External;
                row.provenance.agent.tool_surface =
                    ToolSurface::declared(agent.wrapper_profile.unwrap());
                row.provenance.agent.command = Availability::unavailable(Reason::PrivateArgv);
                row.provenance.agent.model =
                    Availability::present(agent.model.clone().unwrap(), Source::OperatorDeclared);
                row.provenance.agent.config_id = Availability::unavailable(Reason::NotDeclared);
                matrix.cells.push(cell);
                paid.push(row);
            }
        }
    }
    assert_eq!(paid.len(), 480);
    baseline::validate_accounting(
        &paid,
        &matrix,
        &plan.agents,
        &plan.tasks,
        5,
        harness,
        target,
    )
    .unwrap();
    assert_eq!(baseline::status(0, true, true, &paid), 0);
    paid[0].outcome = Outcome::Fail;
    paid[0].failure_class = Some(swarmail_evals::state::FailureClass::GaveUp);
    assert_eq!(baseline::status(1, true, true, &paid), 1);
    paid[1].outcome = Outcome::HarnessError;
    paid[1].failure_class = Some(swarmail_evals::state::FailureClass::HarnessError);
    let summary = report::report(&paid);
    assert_eq!(
        serde_json::json!(
            summary["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["attempts"].as_u64().unwrap())
                .sum::<u64>()
        ),
        480
    );
    assert_eq!(
        serde_json::json!(
            summary["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["harness_errors"].as_u64().unwrap())
                .sum::<u64>()
        ),
        1
    );
    assert_eq!(baseline::status(2, true, true, &paid), 2);
    assert_eq!(baseline::status(0, false, true, &paid), 2);
    assert_eq!(baseline::status(0, true, false, &paid), 2);
    paid.pop();
    assert!(
        baseline::validate_accounting(
            &paid,
            &matrix,
            &plan.agents,
            &plan.tasks,
            5,
            harness,
            target
        )
        .is_err()
    );
    assert_eq!(
        serde_json::json!(
            report::report(&paid)["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["attempts"].as_u64().unwrap())
                .sum::<u64>()
        ),
        479
    );
}
