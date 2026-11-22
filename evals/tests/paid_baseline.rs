//! One-command paid baseline proofs (proposed 0137) over fake direct CLIs:
//! stand-in executables answer the bounded version probe and log each
//! launch; no provider, credential or network beyond loopback is involved.
mod common;
use common::paid::*;
use common::*;
use serde_json::{Value, json};
use std::{io::Write, path::Path, process::Command};
use swarmail_evals::{
    artifacts, baseline,
    provenance::*,
    report,
    runner::TrialResult,
    tasks::{self, Outcome},
};

#[test]
fn dry_run_prints_the_whole_product_and_reads_creates_and_launches_nothing() {
    let dir = tempdir();
    // Neither file exists: a dry run never opens a selection or credential.
    let config = dir.path().join("absent-selection.json");
    let plan_for = |tier: &[&str], output: &Path, margin: &[&str]| {
        let out = Command::new(exe())
            .args(["baseline", "--dry-run", "--config"])
            .arg(&config)
            .arg("--output")
            .arg(output)
            .args(tier)
            .args(margin)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let a = plan_for(&[], &dir.path().join("a"), &[]);
    let b = plan_for(
        &[],
        &dir.path().join("elsewhere"),
        &["--freshness-minutes", "90"],
    );
    let del = plan_for(&["--tier", "deliverability"], &dir.path().join("a"), &[]);
    let id = a["product_id"].as_str().unwrap();
    assert!(id.starts_with("baseline-v1-") && id.len() == "baseline-v1-".len() + 64);
    // Output location and margin never change what an authorization covers.
    assert_eq!(a["product_id"], b["product_id"]);
    assert_ne!(a["product_id"], del["product_id"]);
    assert_eq!(
        a["product_id"],
        baseline::Plan::new(Path::new("/x")).product_id
    );
    assert_eq!(a["authorization"], format!("--authorized-product {id}"));
    assert_eq!(b["freshness"]["margin_minutes"], 90);
    assert_eq!(a["freshness"]["margin_minutes"], 70);
    for (plan, controls, paid, tasks) in [(&a, 32, 480, 16), (&del, 2, 30, 1)] {
        assert_eq!(plan["control_slots"].as_array().unwrap().len(), controls);
        let chunks = plan["chunks"].as_array().unwrap();
        assert_eq!(chunks.len(), tasks * 6);
        assert!(
            chunks
                .iter()
                .all(|c| c["repetitions"] == json!([0, 1, 2, 3, 4]))
        );
        assert_eq!(chunks.len() * 5, paid);
        // Task-major: each task's six declarations, in declaration order.
        let labels: Vec<_> = chunks[..6].iter().map(|c| c["agent"].clone()).collect();
        assert_eq!(
            labels,
            [
                "claude-opus",
                "claude-sonnet",
                "codex-sol",
                "codex-luna",
                "glm-dashscope",
                "glm-spark"
            ]
        );
        assert!(chunks[..6].iter().all(|c| c["task"] == chunks[0]["task"]));
        assert_eq!(
            plan["estimate"]["nominal_minutes"],
            (controls + paid) as u64 * 10
        );
        assert!(
            plan["estimate"]["calculation"]
                .as_str()
                .unwrap()
                .contains(&format!("({controls} control + {paid} paid slots)"))
        );
        let rules: Vec<_> = plan["freshness"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["credential"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(rules, ["claude", "codex", "dsh:dashscope", "dsh:spark-glm"]);
        assert_eq!(plan["agents"].as_array().unwrap().len(), 6);
        assert_eq!(plan["concurrency"], 1);
    }
    assert_eq!(del["chunks"][4]["credential"], "dsh:dashscope");
    assert_eq!(del["chunks"][0]["model"], "claude-opus-5-5");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn missing_and_wrong_authorization_refuse_by_name_after_the_plan() {
    let p = Paid::new();
    let (code, out, err) = run(&mut p.command(&[]));
    assert_eq!(code, Some(2));
    assert!(
        err.contains("baseline refused: authorization_missing"),
        "{err}"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&out).unwrap()["product_id"],
        p.plan().product_id
    );
    let wrong = baseline::Plan::new(&p.output).product_id;
    for extra in [&[][..], &["--resume"][..]] {
        let mut c = p.command(&["--authorized-product", &wrong]);
        let (code, _, err) = run(c.args(extra));
        assert_eq!(code, Some(2));
        assert!(
            err.contains("baseline refused: authorization_mismatch"),
            "{err}"
        );
        let mut c = p.command(&["--resume"]);
        let (code, _, err) = run(&mut c);
        assert_eq!(code, Some(2));
        assert!(err.contains("authorization_missing"), "{err}");
    }
    assert!(!p.output.exists());
    assert!(p.launches().is_empty());
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn every_contract_refusal_names_its_family_and_cause_before_any_spend() {
    let p = Paid::new();
    let refuse = |expected: &str| {
        let (code, _, err) = run(&mut p.authorized(&[]));
        assert_eq!(code, Some(2), "{expected}: {err}");
        assert!(err.contains(expected), "{expected}: {err}");
        assert!(!contains(err.as_bytes(), CANARY), "{err}");
        assert!(!p.output.exists(), "{expected}");
        assert!(p.launches().is_empty(), "{expected}");
    };
    // Only the last family fails: nothing spends for the others either.
    p.select(|v| {
        v["families"].as_object_mut().unwrap().remove("dsh");
    });
    refuse("contract dsh: contract_missing");
    p.select(|v| v["families"]["codex"]["cli_version"] = json!("0.165.0"));
    refuse("contract codex: version_unsupported");
    p.select(|v| v["families"]["dsh"]["cli_sha256"] = json!("0".repeat(64)));
    refuse("contract dsh: digest_mismatch");
    p.select(|v| v["families"]["claude"]["auth_shape"] = json!("codex_chatgpt_v1"));
    refuse("contract claude: auth_shape_unsupported");
    p.select(|v| v["families"]["claude"]["profile"] = json!("claude_2_1_310_via_brokkr"));
    refuse("contract claude: profile_mismatch");
    // An opaque forwarding path cannot establish identity.
    let link = p.dir.path().join("codex-link");
    std::os::unix::fs::symlink(p.bin("codex"), &link).unwrap();
    p.select(|v| v["families"]["codex"]["executable"] = json!(link));
    refuse("contract codex: executable_unsafe");
    // The probe's banner must be the profile's exact version; the banner
    // itself (here carrying a canary) is never echoed.
    p.write_cli("dsh", &format!("0.1.6 {CANARY}"));
    p.select(|_| {});
    refuse("contract dsh: version_mismatch");
    p.write_cli("dsh", banner("dsh"));
    // Credential shapes with unknown validity refuse by family and route.
    p.select(|_| {});
    private(
        &p.cred("codex"),
        json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":CANARY,"access_token":CANARY,"refresh_token":CANARY,"account_id":"a"},"last_refresh":"2026-11-22T00:00:00Z"})
            .to_string()
            .as_bytes(),
    );
    refuse("credential codex: credential_expiry_unknown");
    p.codex(now() + 24 * HOUR);
    // The old two-key dsh file carries no validity evidence.
    private(
        &p.cred("dsh"),
        json!({"DASHSCOPE_API_KEY":CANARY,"SPARK_API_KEY":CANARY})
            .to_string()
            .as_bytes(),
    );
    refuse("credential dsh:dashscope: private_input_invalid");
    // The version-1 selection has no contract at all.
    private(
        &p.config,
        json!({"version":1,"families":{"claude":{"executable":p.bin("claude"),"credential_file":p.cred("claude")},"codex":{"executable":p.bin("codex"),"credential_file":p.cred("codex")},"dsh":{"executable":p.bin("dsh"),"credential_file":p.cred("dsh")}}})
            .to_string()
            .as_bytes(),
    );
    refuse("contract claude: private_input_invalid");
}

#[test]
fn freshness_is_inclusive_and_never_overflows() {
    assert!(baseline::fresh_enough(1000 + 70 * 60, 1000, 70));
    assert!(!baseline::fresh_enough(1000 + 70 * 60 - 1, 1000, 70));
    assert!(!baseline::fresh_enough(u64::MAX, u64::MAX, 1));
    assert!(!baseline::fresh_enough(u64::MAX, 0, u64::MAX));
    let plan = baseline::Plan::for_tier(Some(tasks::Tier::Deliverability), Path::new("/x"));
    let pick = |blocked: &[&str]| {
        baseline::next_chunk(&plan.chunks, |k| !blocked.contains(&k)).map(|c| c.agent.clone())
    };
    assert_eq!(pick(&[]).as_deref(), Some("claude-opus"));
    assert_eq!(pick(&["claude"]).as_deref(), Some("codex-sol"));
    assert_eq!(
        pick(&["claude", "codex", "dsh:dashscope"]).as_deref(),
        Some("glm-spark")
    );
    assert_eq!(
        pick(&["claude", "codex", "dsh:dashscope", "dsh:spark-glm"]),
        None
    );
}

fn attempt_id(lines: &[String]) -> String {
    let line = lines
        .iter()
        .find(|l| l.contains("--confirm-cleanup "))
        .unwrap_or_else(|| panic!("{lines:?}"));
    line.rsplit("--confirm-cleanup ").next().unwrap()[..32].to_string()
}
fn paid_rows(p: &Paid) -> Vec<Value> {
    std::fs::read_to_string(p.output.join("paid.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
/// Every retained paid result.json, by trial id: committed score bytes.
fn retained_results(p: &Paid) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let paid = p.output.join("paid");
    for attempt in std::fs::read_dir(&paid).into_iter().flatten() {
        let trials = attempt.unwrap().path().join("trials");
        for t in std::fs::read_dir(&trials).into_iter().flatten() {
            let t = t.unwrap().path();
            if let Ok(bytes) = std::fs::read(t.join("result.json")) {
                out.insert(t.file_name().unwrap().to_string_lossy().into(), bytes);
            }
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn one_command_gates_on_controls_defers_stale_logins_and_resumes_by_slot() {
    let _scope = trial_scope();
    let p = Paid::new();
    // Claude's login is valid but inside the 70-minute margin: its chunks
    // defer while the other families run.
    p.claude(now() + 30 * 60);
    std::fs::write(&p.hold, b"codex gpt-6.1-sol 3").unwrap();
    let first = Running::start(p.authorized(&[]));
    // Mid-chunk: the third codex-sol trial has launched and is holding.
    wait("three codex-sol launches", || {
        p.count("codex gpt-6.1-sol") == 3
    });
    let launches = p.launches();
    assert_eq!(launches.len(), 3, "controls launch no CLI: {launches:?}");
    // One writer owns the run: a competing resume refuses and starts nothing.
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("run_writer_busy"), "{err}");
    first.signal(libc::SIGKILL);
    let (_, lines) = first.finish();
    assert!(
        lines.iter().any(|l| l.contains("deferring claude")),
        "{lines:?}"
    );
    let scored_before = retained_results(&p);
    assert_eq!(scored_before.len(), 2, "two codex-sol scores committed");
    // The coordinator died: cleanup of its attempt is unproved.
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    let attempt = attempt_id(&err.lines().map(str::to_string).collect::<Vec<_>>());
    // The orphaned trial still holds admission: confirmation cannot override it.
    let (code, _, err) = run(&mut p.authorized(&["--resume", "--confirm-cleanup", &attempt]));
    assert_eq!(code, Some(2));
    assert!(err.contains("trial admission is still held"), "{err}");
    std::fs::remove_file(&p.hold).unwrap();
    wait("the orphaned trial to end", swarmail_evals::admission::idle);
    assert_eq!(p.launches().len(), 3, "refusals launched nothing");
    // The orphaned worker may still have committed its own hash-bound score
    // after its owned cleanup; such a slot is scored and never re-run.
    let scored = |p: &Paid| {
        retained_results(p)
            .values()
            .filter(|b| !contains(b, "\"outcome\":\"harness_error\""))
            .count()
    };
    let orphan_scored = scored(&p);
    assert!((2..=3).contains(&orphan_scored));
    // Resume with the confirmation; interrupt between chunks, after codex-luna.
    let second = Running::start(p.authorized(&["--resume", "--confirm-cleanup", &attempt]));
    second.until("codex-luna closed", |l| {
        l.contains("chunk closed") && l.contains("agent=codex-luna")
    });
    second.signal(libc::SIGTERM);
    let (code, lines) = second.finish();
    assert_eq!(code, Some(2), "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("operator-confirmed cleanup"))
    );
    // codex-sol: one launch per committed slot plus the killed one only if
    // its score was lost; codex-luna: five.
    let codex_sol = 3 + 5 - orphan_scored;
    assert_eq!(p.count("codex gpt-6.1-sol"), codex_sol);
    assert_eq!(p.count("codex gpt-6-luna"), 5);
    let mid = retained_results(&p);
    for (id, bytes) in &scored_before {
        assert_eq!(mid.get(id), Some(bytes), "committed score bytes unchanged");
    }
    // Resume: dsh runs, then every remaining chunk is Claude's and the run
    // waits with its reason until the operator renews that login.
    let third = Running::start(p.authorized(&["--resume"]));
    third.until("the renewal wait", |l| {
        l.contains("baseline waiting") && l.contains("blocked: claude")
    });
    assert_eq!(p.count("claude claude-opus-5-5"), 0);
    p.claude(now() + 24 * HOUR);
    let (code, lines) = third.finish();
    assert_eq!(code, Some(1), "fake CLIs are measured failures: {lines:?}");
    let starts: Vec<_> = lines
        .iter()
        .filter(|l| l.contains("chunk start"))
        .filter_map(|l| l.split("agent=").nth(1)?.split(' ').next())
        .collect();
    // The remaining dsh chunks ran first; the deferred Claude chunks ran last,
    // once the renewed login qualified.
    let tail = starts.len() - 2;
    assert_eq!(
        starts[tail..],
        ["claude-opus", "claude-sonnet"],
        "{lines:?}"
    );
    assert!(
        starts[..tail].iter().all(|a| a.starts_with("glm-")),
        "{lines:?}"
    );
    let launches = p.launches();
    assert!(!launches.contains(&"LEAK".to_string()));
    for (line, n) in [
        ("claude claude-opus-5-5", 5),
        ("claude claude-sonnet-5-5", 5),
        ("codex gpt-6.1-sol", codex_sol),
        ("codex gpt-6-luna", 5),
        ("dsh spark-glm", 5),
    ] {
        assert_eq!(p.count(line), n, "{line}: {launches:?}");
    }
    // An interrupt that reached a started dashscope trial leaves one unscored
    // launch; otherwise there are exactly five.
    let dashscope = p.count("dsh dashscope");
    assert!((5..=6).contains(&dashscope), "{launches:?}");
    // Claude ran last: every claude launch follows every other family's.
    let first_claude = launches
        .iter()
        .position(|l| l.starts_with("claude"))
        .unwrap();
    assert!(
        launches[first_claude..]
            .iter()
            .all(|l| l.starts_with("claude"))
    );
    let rows = paid_rows(&p);
    assert_eq!(rows.len(), 30);
    let plan = p.plan();
    for (i, row) in rows.iter().enumerate() {
        let spec = &plan.agents[i / 5];
        assert_eq!(row["agent"], spec.label);
        assert_eq!(row["outcome"], "fail");
        let family = spec.wrapper_profile.unwrap().family();
        assert_eq!(
            row["provenance"]["agent"]["cli"],
            json!({
                "profile": profile(family),
                "version": profile(family).cli_version(),
                "sha256": swarmail_evals::identity::file_hash(&p.bin(family)).unwrap(),
            })
        );
    }
    let controls = std::fs::read_to_string(p.output.join("controls.jsonl")).unwrap();
    let controls: Vec<Value> = controls
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        controls
            .iter()
            .map(|r| (r["agent"].clone(), r["outcome"].clone()))
            .collect::<Vec<_>>(),
        [
            (json!("oracle"), json!("pass")),
            (json!("null"), json!("fail"))
        ]
    );
    let report: Value =
        serde_json::from_slice(&std::fs::read(p.output.join("report.json")).unwrap()).unwrap();
    let attempts: u64 = report["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["attempts"].as_u64().unwrap())
        .sum();
    assert_eq!(attempts, 30);
    let end = retained_results(&p);
    for (id, bytes) in &scored_before {
        assert_eq!(end.get(id), Some(bytes));
    }
    // No credential, claim or banner byte reaches any retained or public file.
    let everything = all_bytes(&p.output);
    assert!(!contains(&everything, CANARY));
    assert!(!contains(&everything, SPARK_CANARY));
    assert!(!lines.iter().any(|l| l.contains(CANARY)));
    bad_controls_never_admit(&p);
    // Reopening a complete product launches nothing and reproduces its bytes.
    let before = (
        std::fs::read(p.output.join("paid.jsonl")).unwrap(),
        std::fs::read(p.output.join("report.json")).unwrap(),
    );
    std::fs::remove_file(p.output.join("report.json")).unwrap();
    let n = p.launches().len();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(p.launches().len(), n);
    assert_eq!(
        std::fs::read(p.output.join("paid.jsonl")).unwrap(),
        before.0
    );
    assert_eq!(
        std::fs::read(p.output.join("report.json")).unwrap(),
        before.1
    );
    // Conflicting derived output refuses rather than being overwritten.
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(p.output.join("paid.jsonl"))
        .unwrap();
    f.write_all(b"{}\n").unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("output_conflict paid.jsonl"), "{err}");
    // A replaced CLI (re-pinned or not) is identity drift for this run.
    p.write_cli("codex", &format!("{} ", banner("codex")));
    p.write_cli("codex", banner("codex"));
    std::fs::OpenOptions::new()
        .append(true)
        .open(p.bin("codex"))
        .unwrap()
        .write_all(b"# replaced\n")
        .unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("contract codex: digest_mismatch"), "{err}");
    p.select(|_| {});
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("manifest_drift"), "{err}");
    assert_eq!(p.launches().len(), n);
}

fn bytes(rows: &[TrialResult]) -> Vec<u8> {
    rows.iter()
        .flat_map(|r| {
            let mut b = serde_json::to_vec(r).unwrap();
            b.push(b'\n');
            b
        })
        .collect()
}
/// The run's real retained controls: admission refuses every mutation of
/// them, a fresh start never reuses the run's output, and synthetic paid
/// accounting covers the default product's exact 480 coordinates.
fn bad_controls_never_admit(paid: &Paid) {
    let output = &paid.output;
    let stream = std::fs::read(output.join("controls.jsonl")).unwrap();
    let rows = baseline::rows(&stream).unwrap();
    assert_eq!(rows.len(), 2);
    let root = std::fs::read_dir(output.join("controls"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let matrix: artifacts::MatrixPlan =
        decode(&artifacts::read_file(&root, "matrix-plan.json", artifacts::PLAN_LIMIT).unwrap())
            .unwrap();
    let plan = paid.plan();
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
    let again = paid.authorized(&[]).output().unwrap();
    assert_eq!(again.status.code(), Some(2));
    assert_eq!(
        std::fs::read(output.join("controls.jsonl")).unwrap(),
        before
    );
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(output).unwrap().permissions().mode() & 0o777,
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
    paid_accounting(&rows[0], &baseline::Plan::new(output), &harness, &target);
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
