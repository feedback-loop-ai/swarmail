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
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
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
    // The superseded profiles left the approved set (accepted 0137's
    // 2026-11-23 amendment), even beside their own version.
    for (family, old, version) in [
        ("claude", "claude_2_1_310", "2.1.310"),
        ("codex", "codex_0_164_0", "0.164.0"),
        ("dsh", "dsh_0_1_5_rc_1", "0.1.5-rc.1"),
    ] {
        p.select(|v| {
            v["families"][family]["profile"] = json!(old);
            v["families"][family]["cli_version"] = json!(version);
        });
        refuse(&format!("contract {family}: profile_superseded"));
    }
    // dsh runs only as an interpreted package under its full pin (accepted
    // 0137's 2026-11-23 amendment): every mutation refuses before spend.
    dsh_pin_refusals(&p, &refuse);
    // An opaque forwarding path cannot establish identity.
    let link = p.dir.path().join("codex-link");
    std::os::unix::fs::symlink(p.bin("codex"), &link).unwrap();
    p.select(|v| v["families"]["codex"]["executable"] = json!(link));
    refuse("contract codex: executable_unsafe");
    // So cannot a launcher script: its digest never covers what it runs.
    let shim = p.dir.path().join("codex-shim");
    std::fs::write(
        &shim,
        format!("#!/bin/sh\nexec '{}' \"$@\"\n", p.bin("codex").display()),
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::os::unix::fs::PermissionsExt::from_mode(0o700)).unwrap();
    p.select(|v| {
        v["families"]["codex"]["executable"] = json!(shim);
        v["families"]["codex"]["cli_sha256"] =
            json!(swarmail_evals::identity::file_hash(&shim).unwrap());
    });
    refuse("contract codex: executable_not_native");
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
/// Every committed paid result.json (its leaf has the metadata marker), by
/// trial id: committed score bytes.
fn retained_results(p: &Paid) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let paid = p.output.join("paid");
    for attempt in std::fs::read_dir(&paid).into_iter().flatten() {
        let trials = attempt.unwrap().path().join("trials");
        for t in std::fs::read_dir(&trials).into_iter().flatten() {
            let t = t.unwrap().path();
            // A score commits at its metadata marker; a leaf without one
            // holds no committed row.
            if !t.join("metadata.json").exists() {
                continue;
            }
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
    let orphan = first.session.live_trial();
    // One writer owns the run: a competing resume refuses and starts nothing.
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("run_writer_busy"), "{err}");
    first.signal(libc::SIGKILL);
    // The worker outlives its coordinator and still owns its trial.
    let (_, lines, first_family) = first.finish_owned();
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
    // The released worker finishes its owned cleanup (0076): its whole
    // family, servers included, exits before anyone confirms anything.
    first_family.wait_clear(std::time::Duration::from_secs(30));
    // The trial's scratch went with it; the worker's runtime root belonged
    // to the coordinator the test killed.
    assert!(!orphan.scratch.exists(), "{}", orphan.scratch.display());
    first_family.dispose_runtime(&orphan);
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
    // Derived output beside outstanding slots conflicts: the resume refuses
    // before launching anything.
    std::fs::write(p.output.join("report.json"), b"{}").unwrap();
    let n = p.launches().len();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("output_conflict report.json"), "{err}");
    assert_eq!(p.launches().len(), n);
    std::fs::remove_file(p.output.join("report.json")).unwrap();
    // A trial worker killed before it reports proves no owned cleanup: its
    // agent may survive, so the attempt stays open (no closure) and only the
    // operator's confirmation reopens the run.
    let k = p.count("dsh dashscope") + 1;
    std::fs::write(&p.hold, format!("dsh dashscope {k}")).unwrap();
    let killed = Running::start(p.authorized(&["--resume"]));
    wait("a held dashscope launch", || p.count("dsh dashscope") == k);
    // The held launch shows the worker and both live servers in one private
    // scratch root; only the worker is killed, pinned by its identity.
    let trial = killed.session.live_trial();
    killed.session.kill_worker(&trial);
    let (code, lines, family) = killed.finish_owned();
    assert_eq!(code, Some(2), "{lines:?}");
    let open = lines
        .iter()
        .find(|l| l.contains("has no proof of owned cleanup"))
        .unwrap_or_else(|| panic!("{lines:?}"));
    let killed_attempt = open
        .split("attempt ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_string();
    assert!(
        !p.output
            .join("attempts")
            .join(format!("{killed_attempt}.closed"))
            .exists(),
        "no closure without proof"
    );
    std::fs::remove_file(&p.hold).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(
        err.contains(&format!(
            "attempt {killed_attempt} has no proof of owned cleanup"
        )),
        "{err}"
    );
    assert_eq!(p.count("dsh dashscope"), k, "the refusal launched nothing");
    // The test killed the worker, so the cleanup it could not run is the
    // test's: end the trial's surviving service and fixture (0076), check
    // that nothing it owned is left, and clear the abandoned scratch root
    // before acting as the operator who confirms the cleanup (0137).
    family.terminate();
    family.assert_clear();
    family.dispose_scratch(&trial);
    // A leaf retention never finished (a worker killed mid-retention, a full
    // disk) holds no metadata marker: an uncommitted attempt, not tampering.
    // Strip the marker from the killed attempt's unscored leaf; the confirmed
    // resume keeps the leaf as found and its slot stays outstanding.
    let killed_dir = p.output.join("paid").join(&killed_attempt);
    let partial = std::fs::read_dir(killed_dir.join("trials"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|t| {
            std::fs::read(t.join("result.json"))
                .is_ok_and(|b| contains(&b, "\"outcome\":\"harness_error\""))
        })
        .expect("the killed attempt's unscored leaf");
    let partial_bytes = std::fs::read(partial.join("result.json")).unwrap();
    std::fs::remove_file(partial.join("metadata.json")).unwrap();
    let partial_dir = partial.clone();
    let partial = partial.file_name().unwrap().to_string_lossy().to_string();
    // Mid-chunk, the operator rotates the dsh CLI and its pin. The held
    // second glm-spark trial ran on the run's binary; the wrapper of every
    // later trial refuses before exec, so no model runs and those slots stay
    // unscored: harness errors, never measured failures.
    std::fs::write(&p.hold, b"dsh spark-glm 2").unwrap();
    let third = Running::start(p.authorized(&["--resume", "--confirm-cleanup", &killed_attempt]));
    wait("two glm-spark launches", || p.count("dsh spark-glm") == 2);
    rotate(&p, "dsh");
    p.select(|_| {});
    std::fs::remove_file(&p.hold).unwrap();
    let (code, lines) = third.finish();
    assert_eq!(code, Some(2), "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|l| l.contains(&format!("trial {partial} has no committed marker"))),
        "{lines:?}"
    );
    assert_eq!(
        std::fs::read(partial_dir.join("result.json")).unwrap(),
        partial_bytes,
        "the uncommitted leaf is kept as found"
    );
    assert_eq!(
        p.count("dsh spark-glm"),
        2,
        "refused trials launched no CLI"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("agent=glm-spark") && l.contains("scored=2/5")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("ended with a harness error")),
        "{lines:?}"
    );
    let unstarted = retained_results(&p)
        .values()
        .filter(|b| contains(b, "\"outcome\":\"harness_error\""))
        .count();
    assert!(
        unstarted >= 3,
        "three unstarted glm-spark attempts retained"
    );
    // Between chunks: the re-pinned selection is drift for this run, and the
    // restored binary under the rotated pin is a digest refusal.
    let n = p.launches().len();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("baseline refused: contract_drift"), "{err}");
    p.write_cli("dsh", banner("dsh"));
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(
        err.contains("contract dsh: runtime_digest_mismatch"),
        "{err}"
    );
    p.select(|_| {});
    // A different margin is a different run.
    let (code, _, err) = run(&mut p.authorized(&["--resume", "--freshness-minutes", "71"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("manifest_drift"), "{err}");
    assert_eq!(p.launches().len(), n);
    // Resume: the three outstanding glm-spark slots run, then every remaining
    // chunk is Claude's and the run waits with its reason until the operator
    // renews that login.
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
        lines
            .iter()
            .any(|l| l.contains("agent=glm-spark repetitions=[2, 3, 4]")),
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
    // Five scored dashscope launches plus the one whose worker was killed;
    // an interrupt that reached a started dashscope trial adds one more.
    let dashscope = p.count("dsh dashscope");
    assert!((6..=7).contains(&dashscope), "{launches:?}");
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
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(p.output.join("manifest.json")).unwrap()).unwrap();
    for (i, row) in rows.iter().enumerate() {
        let spec = &plan.agents[i / 5];
        assert_eq!(row["agent"], spec.label);
        assert_eq!(row["outcome"], "fail");
        // Each row stands at its nominal slot, whichever attempt scored it,
        // with the seed the run's master gives that slot.
        let m: MatrixCoordinates =
            serde_json::from_value(row["provenance"]["matrix"].clone()).unwrap();
        assert_eq!(
            (m.task_index, m.agent_index, m.repetition_index),
            (0, (i / 5) as u64, (i % 5) as u64),
            "{row}"
        );
        assert_eq!(json!(m.master_seed), manifest["master"]["seed"]);
        assert_eq!(
            row["provenance"]["fixture"]["seed"],
            json!(m.cell_seed(&plan.tasks[0], &spec.label).unwrap())
        );
        let family = spec.wrapper_profile.unwrap().family();
        assert_eq!(
            row["provenance"]["agent"]["cli"],
            json!({
                "profile": profile(family),
                "version": profile(family).cli_version(),
                // dsh's identity is its interpreted pin's, runtime and package.
                "sha256": if family == "dsh" {
                    p.dsh_pin().identity()
                } else {
                    swarmail_evals::identity::file_hash(&p.bin(family)).unwrap()
                },
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
    let end = retained_results(&p);
    for (id, bytes) in &scored_before {
        assert_eq!(end.get(id), Some(bytes));
    }
    // Unscored rows (refused, interrupted, killed) carry the CLI binding too.
    let unscored: Vec<_> = end
        .values()
        .filter(|b| contains(b, "\"outcome\":\"harness_error\""))
        .collect();
    assert!(unscored.len() >= 4, "{}", unscored.len());
    // The report counts every attempt: the 30 scored slots plus each
    // retained unscored attempt as an infrastructure error.
    let report: Value =
        serde_json::from_slice(&std::fs::read(p.output.join("report.json")).unwrap()).unwrap();
    let sum = |field: &str| -> u64 {
        report["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g[field].as_u64().unwrap())
            .sum()
    };
    assert_eq!(sum("attempts"), 30 + unscored.len() as u64);
    assert_eq!(sum("harness_errors"), unscored.len() as u64);
    assert_eq!(sum("scored"), 30);
    // The printed report command reads the retained companions with the
    // same verified reader as report.json (part of #404): it runs as
    // printed, over the same scored slots.
    let printed = cli()
        .args(["report", "--input"])
        .arg(p.output.join("paid.jsonl"))
        .arg("--usage-from")
        .arg(p.output.join("paid"))
        .output()
        .unwrap();
    assert_eq!(printed.status.code(), Some(0), "{printed:?}");
    let printed: Value = serde_json::from_slice(&printed.stdout).unwrap();
    assert_eq!(
        printed["groups"].as_array().unwrap().len(),
        report["groups"].as_array().unwrap().len()
    );
    let scored = |r: &Value| -> u64 {
        r["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["scored"].as_u64().unwrap())
            .sum()
    };
    assert_eq!(scored(&printed), 30);
    for bytes in unscored {
        assert!(
            contains(bytes, "\"cli\":{\"profile\":"),
            "{}",
            String::from_utf8_lossy(bytes)
        );
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
    // Beside a control matrix that would have to run again, derived output
    // conflicts before the controls launch.
    let control = std::fs::read_dir(p.output.join("controls"))
        .unwrap()
        .map(|e| e.unwrap().path().join("matrix.json"))
        .find(|m| m.exists())
        .unwrap();
    std::fs::rename(&control, control.with_extension("aside")).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("output_conflict paid.jsonl"), "{err}");
    std::fs::rename(control.with_extension("aside"), &control).unwrap();
    std::fs::remove_file(p.output.join("paid.jsonl")).unwrap();
    // Unexpected or edited slot evidence refuses before any launch.
    let stray = p.output.join("attempts").join("stray");
    std::fs::write(&stray, b"{}").unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("attempt_evidence_unexpected"), "{err}");
    std::fs::remove_file(&stray).unwrap();
    let assignment = std::fs::read_dir(p.output.join("attempts"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|f| {
            f.extension().is_some_and(|x| x == "json")
                && p.output
                    .join("paid")
                    .join(f.file_stem().unwrap())
                    .join("matrix-plan.json")
                    .exists()
        })
        .unwrap();
    let original = std::fs::read(&assignment).unwrap();
    let mut edited: Value = serde_json::from_slice(&original).unwrap();
    edited["agent"] = json!(if edited["agent"] == "glm-spark" {
        "glm-dashscope"
    } else {
        "glm-spark"
    });
    std::fs::write(&assignment, edited.to_string()).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("matrix_plan_inconsistent"), "{err}");
    std::fs::write(&assignment, &original).unwrap();
    // A resumed attempt's slots cannot be re-labelled: [2, 3, 4] edited to
    // the sorted, in-range [1, 3, 4] no longer matches its cells.
    let resumed = std::fs::read_dir(p.output.join("attempts"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|f| {
            std::fs::read_to_string(f)
                .unwrap()
                .contains("\"repetitions\":[2,3,4]")
        })
        .unwrap();
    let original = std::fs::read(&resumed).unwrap();
    let edited = String::from_utf8(original.clone())
        .unwrap()
        .replace("\"repetitions\":[2,3,4]", "\"repetitions\":[1,3,4]");
    std::fs::write(&resumed, edited).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("matrix_plan_inconsistent"), "{err}");
    std::fs::write(&resumed, &original).unwrap();
    // A closure must name how cleanup was proved.
    let closure = resumed.with_extension("closed");
    let original = std::fs::read(&closure).unwrap();
    let forged = String::from_utf8(original.clone())
        .unwrap()
        .replace("owned_cleanup", "assumed");
    std::fs::write(&closure, forged).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("closure_invalid"), "{err}");
    std::fs::write(&closure, &original).unwrap();
    // Restored evidence regenerates the same canonical rows.
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(
        std::fs::read(p.output.join("paid.jsonl")).unwrap(),
        before.0
    );
    assert_eq!(p.launches().len(), n);
    // Recovery needs no private input: with the selection, every credential
    // and every CLI gone, a complete run still reopens and regenerates its
    // derived output from retained rows alone.
    std::fs::remove_file(p.output.join("report.json")).unwrap();
    std::fs::remove_file(&p.config).unwrap();
    for family in FAMILIES {
        std::fs::remove_file(p.bin(family)).unwrap();
        std::fs::remove_file(p.cred(family)).unwrap();
    }
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(
        std::fs::read(p.output.join("report.json")).unwrap(),
        before.1
    );
    assert_eq!(p.launches().len(), n);
}

/// Replace `family`'s CLI with different bytes that still answer its banner.
fn dsh_pin_refusals(p: &Paid, refuse: &dyn Fn(&str)) {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = |path: &std::path::Path, m: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(m)).unwrap()
    };
    // A native family never carries a pin, and dsh never runs without one.
    p.select(|v| {
        v["families"]["claude"]["interpreted"] = v["families"]["dsh"]["interpreted"].clone()
    });
    refuse("contract claude: interpreted_pin_unexpected");
    p.select(|v| {
        v["families"]["dsh"]
            .as_object_mut()
            .unwrap()
            .remove("interpreted");
    });
    refuse("contract dsh: interpreted_pin_missing");
    // A shim in place of the entry, or of the runtime, refuses by name.
    p.select(|v| v["families"]["dsh"]["executable"] = json!(p.bin("dsh")));
    refuse("contract dsh: executable_not_entry");
    let shim = p.dir.path().join("node-shim");
    std::os::unix::fs::symlink(p.bin("dsh"), &shim).unwrap();
    let pin =
        swarmail_evals::interpreted::pin(&shim, NODE_BANNER, &p.package(), DSH_ENTRY).unwrap();
    p.select(|v| {
        v["families"]["dsh"]["cli_sha256"] = json!(pin.identity());
        v["families"]["dsh"]["interpreted"] = json!(pin);
    });
    refuse("contract dsh: runtime_executable_unsafe");
    // A changed runtime binary or banner.
    p.select(|v| v["families"]["dsh"]["interpreted"]["runtime_version"] = json!("v22.11.1"));
    refuse("contract dsh: digest_mismatch");
    let mut pin = p.dsh_pin();
    pin.runtime_version = "v22.11.1".into();
    p.select(|v| {
        v["families"]["dsh"]["cli_sha256"] = json!(pin.identity());
        v["families"]["dsh"]["interpreted"] = json!(pin);
    });
    refuse("contract dsh: runtime_version_mismatch");
    p.select(|_| {});
    rotate(p, "dsh");
    refuse("contract dsh: runtime_digest_mismatch");
    p.write_cli("dsh", banner("dsh"));
    // A changed, an extra and a missing package file, and a link leaving
    // the package root.
    let package = p.package();
    let entry = package.join(DSH_ENTRY);
    std::fs::write(&entry, "// changed\n").unwrap();
    refuse("contract dsh: package_file_changed");
    std::fs::write(&entry, "// dsh entry\n").unwrap();
    let extra = package.join("extra.js");
    std::fs::write(&extra, "").unwrap();
    mode(&extra, 0o644);
    refuse("contract dsh: package_file_extra");
    std::fs::remove_file(&extra).unwrap();
    let manifest = package.join("package.json");
    let kept = std::fs::read(&manifest).unwrap();
    std::fs::remove_file(&manifest).unwrap();
    refuse("contract dsh: package_file_missing");
    std::fs::write(&manifest, kept).unwrap();
    mode(&manifest, 0o644);
    std::os::unix::fs::symlink(p.cred("dsh"), package.join("escape")).unwrap();
    refuse("contract dsh: package_symlink_escapes");
    std::fs::remove_file(package.join("escape")).unwrap();
    p.select(|_| {});
}

fn rotate(p: &Paid, family: &str) {
    p.write_cli(family, banner(family));
    let next = p.bin(family).with_extension("rotated");
    let mut bytes = std::fs::read(p.bin(family)).unwrap();
    bytes.extend_from_slice(b"# rotated\n");
    std::fs::write(&next, bytes).unwrap();
    std::fs::set_permissions(&next, std::os::unix::fs::PermissionsExt::from_mode(0o700)).unwrap();
    std::fs::rename(&next, p.bin(family)).unwrap();
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
        nominal: None,
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

/// The operator's helper prints exactly the dsh selection's pin members the
/// harness verifies, and refuses an unpinnable package by its finite reason.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn pin_interpreted_prints_the_selection_members_the_contract_verifies() {
    let p = Paid::new();
    let pin_command = || {
        let mut c = cli();
        c.arg("pin-interpreted")
            .arg("--runtime")
            .arg(p.bin("dsh"))
            .args(["--runtime-version", NODE_BANNER, "--package-root"])
            .arg(p.package())
            .args(["--entry", DSH_ENTRY]);
        run(&mut c)
    };
    let (code, out, err) = pin_command();
    assert_eq!(code, Some(0), "{err}");
    let pin = p.dsh_pin();
    let printed: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(
        printed,
        json!({"executable": pin.entry_path(), "cli_sha256": pin.identity(), "interpreted": pin})
    );
    std::os::unix::fs::symlink(p.cred("dsh"), p.package().join("escape")).unwrap();
    let (code, out, err) = pin_command();
    assert_eq!((code, out.as_str()), (Some(2), ""));
    assert!(
        err.contains("pin refused: package_symlink_escapes"),
        "{err}"
    );
    assert!(!contains(err.as_bytes(), CANARY));
}

/// Route (b) through the real `baseline` entry point (part of #404, proposed
/// 0138): every paid slot runs through one offline stand-in driver, the
/// baseline's own report reads each committed usage companion, its printed
/// `report_argv` replays to the same report, and a resume refuses a
/// companion that claims another leaf's driver attempt or slot and a codex selection
/// whose sandbox class differs from the retained rows'.
///
/// Host-gated for time, not privilege: the smallest product is 32 slots and
/// takes about 100 s, past the CI test job's budget. It is Linux-only: the
/// paid launch grant needs Linux's socket peer evidence and `/proc`
/// (`paid_grant_unsupported` elsewhere). The dispatcher runs it:
/// `cd evals && cargo test --locked --test paid_baseline -- --ignored a_nested_baseline`.
#[test]
#[ignore = "host-gated: a whole nested baseline (~100 s, Linux only: grant endpoints and /proc); run with --ignored"]
fn a_nested_baseline_reports_its_companions_and_resume_refuses_what_they_contradict() {
    let _scope = trial_scope();
    let p = Paid::nested();
    let finals = json!({
        "claude": {"step": "claude-code-session-finished", "input_tokens": 41871, "output_tokens": 180, "cache_read_tokens": 41867, "cache_write_tokens": 18455, "num_turns": 2, "total_cost_usd": 0.0798147, "exit_code": 0},
        "codex": {"step": "codex-session-finished", "input_tokens": 45729, "output_tokens": 244, "cache_read_tokens": 41600, "num_turns": 1, "exit_code": 0},
        "dsh": {"step": "deepseek-harness-session-finished", "input_tokens": 11497, "output_tokens": 323, "cache_read_tokens": 4096, "num_turns": 2, "exit_code": 0},
    });
    let mut by_family = json!({});
    for f in FAMILIES {
        by_family[f] = json!([finals[f]]);
    }
    p.driver_mode(json!({"mode": "ok", "finals_by_family": by_family}));
    let (code, out, err) = run(&mut p.authorized(&[]));
    assert_eq!(
        code,
        Some(1),
        "every stand-in trial is a measured failure: {err}"
    );
    // The printed plan's report command reads the companions.
    let plan: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    let argv: Vec<String> = serde_json::from_value(plan["report_argv"].clone()).unwrap();
    assert_eq!(
        argv[1..],
        [
            "report".to_string(),
            "--input".into(),
            p.output.join("paid.jsonl").display().to_string(),
            "--usage-from".into(),
            p.output.join("paid").display().to_string(),
        ]
    );
    // The baseline's own report reads every nested companion: each group's
    // figures are its five sessions' sums, never `incomplete_attribution`.
    let bytes = std::fs::read(p.output.join("report.json")).unwrap();
    assert!(!contains(&bytes, "incomplete_attribution"));
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    let groups = report["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 6);
    for g in groups {
        let family = p
            .plan()
            .agents
            .iter()
            .find(|a| json!(a.label) == g["agent"])
            .unwrap()
            .wrapper_profile
            .unwrap()
            .family();
        let usage = g
            .as_object()
            .unwrap()
            .values()
            .find(|v| v.get("total_definition").is_some())
            .unwrap_or_else(|| panic!("{g}"));
        let f = &finals[family];
        let total =
            5 * (f["input_tokens"].as_u64().unwrap() + f["output_tokens"].as_u64().unwrap());
        assert_eq!(usage["scored_trials"], 5, "{usage}");
        assert_eq!(
            usage["total_tokens"],
            json!({"status": "present", "value": total}),
            "{usage}"
        );
        assert_eq!(
            usage["cache_read_tokens"],
            json!({"status": "present", "value": 5 * f["cache_read_tokens"].as_u64().unwrap()})
        );
        // Every stand-in trial fails: per-success is typed, never zero.
        assert_eq!(
            usage["tokens_per_success"],
            json!({"status": "unavailable", "reason": "no_passes"})
        );
    }
    // The printed command, run as printed, reproduces the same report.
    let replay = cli().args(&argv[1..]).output().unwrap();
    assert_eq!(replay.status.code(), Some(0), "{replay:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&replay.stdout).unwrap(),
        report
    );
    // Two committed companions in different attempts' leaves.
    let mut leaves = Vec::new();
    for attempt in std::fs::read_dir(p.output.join("paid")).unwrap() {
        let trials = attempt.unwrap().path().join("trials");
        if let Some(leaf) = std::fs::read_dir(trials)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|l| l.join("usage.json").exists())
        {
            leaves.push(leaf);
        }
    }
    assert_eq!(leaves.len(), 6, "{leaves:?}");
    let (a, b) = (&leaves[0], &leaves[1]);
    let read = |leaf: &Path, name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(leaf.join(name)).unwrap()).unwrap()
    };
    let original = (
        std::fs::read(b.join("usage.json")).unwrap(),
        std::fs::read(b.join("metadata.json")).unwrap(),
    );
    // Leaf B's companion claims leaf A's driver attempt; its own binding,
    // slot and profile stay intact and its marker re-binds the new bytes, so
    // only the ledger-wide owner check can see it.
    let mut usage = read(b, "usage.json");
    usage["sessions"][0]["attempt_id"] = read(a, "usage.json")["sessions"][0]["attempt_id"].clone();
    let forged = serde_json::to_vec(&usage).unwrap();
    let mut meta = read(b, "metadata.json");
    meta["usage"]["length"] = json!(forged.len());
    meta["usage"]["sha256"] = json!(swarmail_evals::provenance::sha256(&forged));
    std::fs::write(b.join("usage.json"), &forged).unwrap();
    std::fs::write(b.join("metadata.json"), serde_json::to_vec(&meta).unwrap()).unwrap();
    // Each leaf still verifies alone: only the shared owner set refuses.
    artifacts::trial(a).unwrap();
    artifacts::trial(b).unwrap();
    std::fs::remove_file(p.output.join("report.json")).unwrap();
    let n = std::fs::read(p.dir.path().join("driver-record.json")).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("duplicate usage owner"), "{err}");
    assert!(!p.output.join("report.json").exists());
    std::fs::write(b.join("usage.json"), &original.0).unwrap();
    std::fs::write(b.join("metadata.json"), &original.1).unwrap();
    // The version-1 assignment chain (tasks.md, the panel's low on
    // 9f28d4e8): leaf B's companion names leaf A's slot, every other field
    // intact and the marker re-bound to the new bytes, so the companion is
    // hash-valid and only its ownership of B's row is wrong.
    let mut usage = read(b, "usage.json");
    assert_ne!(usage["slot"], read(a, "usage.json")["slot"]);
    usage["slot"] = read(a, "usage.json")["slot"].clone();
    let forged = serde_json::to_vec(&usage).unwrap();
    let mut meta = read(b, "metadata.json");
    meta["usage"]["length"] = json!(forged.len());
    meta["usage"]["sha256"] = json!(swarmail_evals::provenance::sha256(&forged));
    std::fs::write(b.join("usage.json"), &forged).unwrap();
    std::fs::write(b.join("metadata.json"), serde_json::to_vec(&meta).unwrap()).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("usage_binding_mismatch"), "{err}");
    assert!(!p.output.join("report.json").exists());
    std::fs::write(b.join("usage.json"), &original.0).unwrap();
    std::fs::write(b.join("metadata.json"), &original.1).unwrap();
    // Each retained codex row records the class its selection named.
    let rows = paid_rows(&p);
    let codex: Vec<_> = rows
        .iter()
        .filter(|r| r["agent"].as_str().unwrap().starts_with("codex-"))
        .collect();
    assert_eq!(codex.len(), 10);
    for row in &codex {
        let declared = row["provenance"]["agent"]["tool_surface"].to_string();
        assert!(
            declared.contains("workspace-write sandbox (driver-selected)"),
            "{declared}"
        );
        assert!(!declared.contains("\"read-only sandbox"), "{declared}");
    }
    // A codex selection whose sandbox class differs from the retained rows'
    // refuses the resume at the first codex attempt, writing no report.
    p.select(|v| v["families"]["codex"]["nested"]["codex_sandbox"] = json!("read-only"));
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("baseline_provenance_invalid"), "{err}");
    assert!(!p.output.join("report.json").exists());
    // The same selection with the retained class reopens: only the class
    // differed.
    p.select(|v| v["families"]["codex"]["nested"]["codex_sandbox"] = json!("workspace-write"));
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(1), "{err}");
    std::fs::remove_file(p.output.join("report.json")).unwrap();
    // The other side: the selection stays fixed and one retained codex row
    // records the other admitted class, its digest repaired in the marker.
    let leaf = leaves
        .iter()
        .find(|l| {
            read(l, "result.json")["agent"]
                .as_str()
                .unwrap()
                .starts_with("codex-")
        })
        .unwrap();
    let kept = (
        std::fs::read(leaf.join("result.json")).unwrap(),
        std::fs::read(leaf.join("metadata.json")).unwrap(),
    );
    let text = String::from_utf8(kept.0.clone()).unwrap();
    assert!(text.contains("workspace-write sandbox (driver-selected)"));
    let swapped = text
        .replace(
            "workspace-write sandbox (driver-selected)",
            "read-only sandbox (driver-selected)",
        )
        .into_bytes();
    let mut meta = read(leaf, "metadata.json");
    meta["result"]["length"] = json!(swapped.len());
    meta["result"]["sha256"] = json!(swarmail_evals::provenance::sha256(&swapped));
    std::fs::write(leaf.join("result.json"), &swapped).unwrap();
    std::fs::write(
        leaf.join("metadata.json"),
        serde_json::to_vec(&meta).unwrap(),
    )
    .unwrap();
    artifacts::trial(leaf).unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("baseline_provenance_invalid"), "{err}");
    assert!(!p.output.join("report.json").exists());
    std::fs::write(leaf.join("result.json"), &kept.0).unwrap();
    std::fs::write(leaf.join("metadata.json"), &kept.1).unwrap();
    // Restored, the complete product reopens to its own bytes, having
    // launched nothing in any of the refusals above.
    p.select(|_| {});
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(std::fs::read(p.output.join("report.json")).unwrap(), bytes);
    assert_eq!(
        std::fs::read(p.dir.path().join("driver-record.json")).unwrap(),
        n
    );
}
