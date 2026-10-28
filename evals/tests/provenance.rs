//! Version migration and retained evidence against actual server/agent children.
mod common;
use common::*;
use serde_json::{Value, json};
use std::path::Path;
use swarmail_evals::{artifacts as a, provenance as p, results, runner, tasks::Outcome};
const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const TIME: &str = "2026-10-24T12:00:00Z";
fn inputs() -> p::FixtureInputs {
    p::FixtureInputs::resolve(Some(ZERO), Some(TIME)).unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn load(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn save(path: &Path, v: &Value) {
    std::fs::write(path, bytes(v)).unwrap();
}
fn bind(path: &Path, name: &str) -> Value {
    let b = std::fs::read(path.join(name)).unwrap();
    json!({"path":name,"length":b.len(),"sha256":p::sha256(&b)})
}
fn rebind(root: &Path, name: &str) {
    let path = root.join("metadata.json");
    let mut meta = load(&path);
    if name == "state.json" {
        meta["state"]["file"] = bind(root, name);
    } else if name == "result.json" {
        meta["result"] = bind(root, name);
    }
    save(&path, &meta);
}
fn retained(task: &str, agent: &str) -> (tempfile::TempDir, runner::TrialResult) {
    let dir = tempfile::tempdir().unwrap();
    let out = cli()
        .args([
            "trial",
            "--task",
            task,
            "--agent",
            agent,
            "--seed",
            ZERO,
            "--fixture-time",
            TIME,
            "--artifacts-dir",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_ne!(row.outcome, Outcome::HarnessError, "{}", stderr(&out));
    (dir, row)
}
fn check(root: &Path) -> std::process::Output {
    cli()
        .args(["check", "--artifacts-dir"])
        .arg(root)
        .env("SWARMAIL_BIN", "/does-not-exist")
        .output()
        .unwrap()
}
fn matrix_helper(mode: &str, env: &[(&str, &str)]) -> (tempfile::TempDir, Vec<String>) {
    let (command, owner) = helper_command(mode, &[], env);
    let argv = command
        .get_args()
        .skip_while(|a| *a != "--")
        .skip(1)
        .map(|a| a.to_str().unwrap().to_string())
        .collect();
    (owner, argv)
}
#[test]
fn independent_length_framing_cell_choice_and_public_argv_vectors() {
    assert_eq!(
        p::hex(&p::framed(&[b"a", b"bc"])),
        "3fafa1cf2f19a7c1129beb20cf0983f73a489a221fc0dd2f16d1be292d089205"
    );
    assert_eq!(
        p::hex(&p::framed(&[b"ab", b"c"])),
        "601d5476e2ccfe2c87a2bba7a322659734a05749d5b5aa781f513e4912db0d5f"
    );
    assert_eq!(
        p::hex(&p::framed(&[b"", b""])),
        "374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb"
    );
    let c = p::MatrixCoordinates {
        matrix_id: "0".repeat(32),
        master_seed: ZERO.into(),
        task_index: 1,
        agent_index: 2,
        repetition_index: 3,
    };
    assert_eq!(
        c.cell_seed("signup", "λ agent").unwrap(),
        "cf9e798d5bf4852d2c64917c7a820f3433ee0d927c0adb76f54fc20096471b7c"
    );
    assert_eq!(
        p::hex(&inputs().choice("digest", 0, "digest", 0, "target-position")),
        "d5ac35bf56bbb079f62d40b59813ce00c8f49b3288a4cc88c08e0fe63c96c356"
    );
    for (args, expected) in [
        (
            vec![],
            "9a7f2529b8c7193e0a1e4a7b3a6bef5e66f3a5fccd1467df11ef61ce8e687e4d",
        ),
        (
            vec![""],
            "0503c0d82aa95082f02a2df8d69a857a1ea0e18ca47fc3cd9407758c3c8c5adb",
        ),
        (
            vec!["ab", "c"],
            "3335f181fb1d42c097ae74b2fb6c959eb35727446b6ea821bb10e1c4fc25f830",
        ),
        (
            vec!["a", "bc"],
            "e5c258e3fcad03f8c97b73d14a6ee61c14959003a1449d602570afe11a7df060",
        ),
        (
            vec!["λ", ""],
            "614dee3feb7854b67e90b541adc770705c4f70dcbf6383f6f6ba125e009a63a6",
        ),
    ] {
        assert_eq!(
            p::public_argv(&args.into_iter().map(str::to_string).collect::<Vec<_>>()),
            expected
        );
    }
}
#[test]
fn input_calendar_hex_depth_and_serialization_bounds() {
    for seed in ["", "00", &"A".repeat(64), &"g".repeat(64)] {
        assert!(p::FixtureInputs::resolve(Some(seed), Some(TIME)).is_err());
    }
    for t in [
        "2026-02-29T00:00:00Z",
        "2024-13-01T00:00:00Z",
        "1969-12-31T23:59:59Z",
        "9998-12-02T00:00:00Z",
        "2026-10-24T12:00:00+00:00",
        "2026-10-24T12:00:00.0Z",
    ] {
        assert!(p::fixture_time(t).is_err(), "{t}");
    }
    for t in [
        "1970-01-01T00:00:00Z",
        "2024-02-29T00:00:00Z",
        "9998-12-01T00:00:00Z",
        "9998-12-01T23:59:59Z",
    ] {
        assert!(p::fixture_time(t).is_ok());
    }
    assert!(p::depth(format!("{}0{}", "[".repeat(64), "]".repeat(64)).as_bytes()).is_ok());
    assert!(p::depth(format!("{}0{}", "[".repeat(65), "]".repeat(65)).as_bytes()).is_err());
    assert!(a::encode(&"x".repeat(61), 64).is_ok()); // quotes + 61 + newline == 64
    assert!(a::encode(&"x".repeat(62), 64).is_ok());
    assert!(a::encode(&"x".repeat(63), 64).is_err());
}
#[test]
fn historical_rows_refuse_and_current_rows_keep_report_accounting() {
    let historical = include_str!("data/results-v1.jsonl");
    assert!(swarmail_evals::report::parse_rows(historical).is_err());
    for line in historical.lines() {
        assert!(results::read(line.as_bytes()).is_err());
    }
    // Preserve report arithmetic with provenance-bearing v2 records.
    let rows: Vec<_> = historical
        .lines()
        .map(|line| {
            let mut value: Value = serde_json::from_str(line).unwrap();
            let req = runner::TrialRequest {
                task: value["task"].as_str().map(str::to_string),
                agent: value["agent"].as_str().map(str::to_string),
                argv: vec!["/bin/true".into()],
                fixture: inputs(),
                ..Default::default()
            };
            value["schema_version"] = json!(2);
            value["concurrency"] = json!(1);
            value["trial_id"] = json!(runner::new_trial_id());
            value["provenance"] =
                serde_json::to_value(runner::make_provenance(&req, None, None)).unwrap();
            results::read(&bytes(&value)).unwrap()
        })
        .collect();
    let mut current = swarmail_evals::report::report(&rows);
    assert!(
        current["tool_surface_note"]
            .as_str()
            .unwrap()
            .contains("asymmetric")
    );
    current.as_object_mut().unwrap().remove("tool_surface_note");
    assert_eq!(
        current["groups"][0]
            .as_object_mut()
            .unwrap()
            .remove("tool_surfaces"),
        Some(json!([{"kind":"external_unknown"}]))
    );
    assert_eq!(
        current["groups"][1]
            .as_object_mut()
            .unwrap()
            .remove("tool_surfaces"),
        Some(json!([{"kind":"invalid_request"}]))
    );
    // Remove exactly the newly asserted metadata. Every historical accounting
    // field, including intervals and quantiles, still matches byte-for-byte.
    assert_eq!(
        current.to_string(),
        include_str!("data/report-v1.json").trim()
    );
    let req = runner::TrialRequest {
        task: Some("signup".into()),
        agent: Some("null".into()),
        fixture: inputs(),
        ..Default::default()
    };
    let row = runner::refused_row(&req, &"0".repeat(32));
    let mixed = format!("{historical}{}\n", serde_json::to_string(&row).unwrap());
    assert!(swarmail_evals::report::parse_rows(&mixed).is_err());
}
#[test]
fn strict_version_dispatch_required_nullable_fields_duplicates_and_hybrids() {
    let req = runner::TrialRequest {
        task: Some("signup".into()),
        agent: Some("null".into()),
        fixture: inputs(),
        ..Default::default()
    };
    let row = runner::refused_row(&req, &"0".repeat(32));
    let mut valid = serde_json::to_value(&row).unwrap();
    // This is a synthetic wire fixture; its boolean duplicate vector must
    // not depend on whether the checkout happened to be dirty at build time.
    valid["provenance"]["harness"]["source_dirty"] =
        json!({"status":"present","source":"build_git","value":true});
    assert!(results::read(&bytes(&valid)).is_ok());
    fn required(v: &Value, path: &mut Vec<String>, root: &Value) {
        if let Value::Object(map) = v {
            for (k, child) in map {
                let mut missing = root.clone();
                let mut at = &mut missing;
                for p in path.iter() {
                    at = at.get_mut(p).unwrap();
                }
                at.as_object_mut().unwrap().remove(k);
                assert!(
                    results::read(&bytes(&missing)).is_err(),
                    "missing {path:?}/{k}"
                );
                path.push(k.clone());
                required(child, path, root);
                path.pop();
            }
        }
    }
    required(&valid, &mut vec![], &valid);
    for from in [
        "\"schema_version\":2",
        "\"trial_id\":",
        "\"status\":\"present\"",
        "\"source\":\"build_git\"",
        "\"value\":true",
    ] {
        let text = serde_json::to_string(&valid).unwrap();
        let duplicate = if from.ends_with(':') {
            text.replacen(from, "\"trial_id\":\"duplicate\",\"trial_id\":", 1)
        } else {
            text.replacen(from, &format!("{from},{from}"), 1)
        };
        assert_ne!(duplicate, text);
        assert!(results::read(duplicate.as_bytes()).is_err(), "{duplicate}");
    }
    for (key, value) in [("schema_version", json!(3)), ("unexpected", json!(true))] {
        let mut v = valid.clone();
        v[key] = value;
        assert!(results::read(&bytes(&v)).is_err());
    }
    let mut v = valid.clone();
    v["schema_version"] = json!(1);
    assert!(results::read(&bytes(&v)).is_err());
    let mut old: Value = serde_json::from_str(
        include_str!("data/results-v1.jsonl")
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    required(&old, &mut vec![], &old);
    let old_bytes = serde_json::to_string(&old).unwrap();
    let duplicate = old_bytes.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(results::read(duplicate.as_bytes()).is_err());
    old["schema_version"] = json!(2);
    assert!(results::read(&bytes(&old)).is_err());
}
#[test]
fn trial_retention_replay_preserves_original_and_refuses_tampered_bytes_and_symlinks() {
    let _trial_scope = trial_scope();
    let (dir, row) = retained("signup", "oracle");
    let root = dir.path().join(&row.trial_id);
    let replay = check(&root);
    assert_eq!(replay.status.code(), Some(0), "{}", stderr(&replay));
    let value: Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(
        value["original_result"],
        serde_json::to_value(&row).unwrap()
    );
    assert_eq!(value["business_replay"]["verdict"]["outcome"], "pass");
    let result = root.join("result.json");
    let original = std::fs::read(&result).unwrap();
    let original_meta = std::fs::read(root.join("metadata.json")).unwrap();
    for concurrency in [
        None,
        Some(json!(0)),
        Some(json!(2)),
        Some(json!(1.0)),
        Some(json!("1")),
        Some(Value::Null),
    ] {
        let mut invalid: Value = serde_json::from_slice(&original).unwrap();
        match concurrency {
            None => {
                invalid.as_object_mut().unwrap().remove("concurrency");
            }
            Some(value) => invalid["concurrency"] = value,
        }
        save(&result, &invalid);
        rebind(&root, "result.json");
        let refused = check(&root);
        assert_eq!(refused.status.code(), Some(2));
        assert!(refused.stdout.is_empty());
    }
    std::fs::write(root.join("metadata.json"), original_meta).unwrap();
    std::fs::write(&result, b"{}").unwrap();
    let refused = check(&root);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    std::fs::write(&result, &original).unwrap();
    let outside = dir.path().join("result-copy");
    std::fs::rename(&result, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &result).unwrap();
    assert!(a::trial(&root).is_err());
    std::fs::remove_file(&result).unwrap();
    std::fs::rename(outside, &result).unwrap();
    let marker = root.join("metadata.json");
    let meta = std::fs::read(&marker).unwrap();
    std::fs::remove_file(&marker).unwrap();
    assert!(a::trial(&root).is_err());
    std::fs::write(marker, meta).unwrap();
    let mut v = load(&result);
    v["provenance"]["task"]["scorer_id"] = json!("future-scorer");
    save(&result, &v);
    rebind(&root, "result.json");
    assert!(a::trial(&root).is_err());
}
#[test]
fn snapshot_semantics_and_historical_version_cannot_be_laundered_by_new_hashes() {
    let _trial_scope = trial_scope();
    let (dir, row) = retained("signup", "oracle");
    let root = dir.path().join(&row.trial_id);
    let state = root.join("state.json");
    let original = load(&state);
    let mut wrong = original.clone();
    wrong["trial_id"] = json!("f".repeat(32));
    save(&state, &wrong);
    rebind(&root, "state.json");
    assert!(a::trial(&root).is_err());
    wrong = original.clone();
    wrong["events"] = json!([]);
    save(&state, &wrong);
    rebind(&root, "state.json");
    assert!(a::trial(&root).is_err());
    save(&state, &original);
    rebind(&root, "state.json");
    let result = root.join("result.json");
    let mut old = load(&result);
    old.as_object_mut().unwrap().remove("provenance");
    old["schema_version"] = json!(1);
    save(&result, &old);
    rebind(&root, "result.json");
    assert!(a::trial(&root).unwrap_err().contains("version"));
}
#[test]
fn early_errors_and_impossible_retention_are_truthful_without_private_diagnostics() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let out = cli()
        .env("SWARMAIL_BIN", "/missing-target")
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "null",
            "--artifacts-dir",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    let row = one_row(&out);
    let root = dir.path().join(&row.trial_id);
    assert!(root.join("metadata.json").exists());
    let replay = check(&root);
    assert_eq!(replay.status.code(), Some(2));
    let r: Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(r["business_replay"]["status"], "unscorable");
    let bad = dir.path().join("not-a-directory");
    std::fs::write(&bad, "sentinel").unwrap();
    let out = cli()
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "oracle",
            "--artifacts-dir",
        ])
        .arg(&bad)
        .output()
        .unwrap();
    assert_eq!(one_row(&out).outcome, Outcome::HarnessError);
    assert_eq!(std::fs::read_to_string(bad).unwrap(), "sentinel");
    let marker = "CANARY-private-credential-38f7";
    let out = cli()
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "external",
            "--artifacts-dir",
        ])
        .arg(dir.path())
        .args(["--", "/bin/false", marker])
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(
        row.provenance.agent.command,
        p::Availability::unavailable(p::Reason::PrivateArgv)
    );
    for entry in std::fs::read_dir(dir.path().join(&row.trial_id)).unwrap() {
        let b = std::fs::read(entry.unwrap().path()).unwrap();
        let s = String::from_utf8_lossy(&b);
        assert!(!s.contains(marker));
        assert!(!s.contains(&p::sha256(marker.as_bytes())));
    }
}
#[test]
fn a_real_eight_cell_retained_matrix_replays_offline_in_order_and_checks_all_before_output() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents.json");
    save(
        &agents,
        &json!([{"label":"oracle","argv":[]},{"label":"null","argv":[]}]),
    );
    let out = cli()
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--trials",
            "2",
            "--agents",
        ])
        .arg(agents)
        .args(["--seed", ZERO, "--fixture-time", TIME, "--artifacts-dir"])
        .arg(dir.path())
        .output()
        .unwrap();
    let rows = swarmail_evals::report::parse_rows(&String::from_utf8_lossy(&out.stdout)).unwrap();
    assert_eq!(rows.len(), 8);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let runs: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(runs.len(), 1);
    let root = runs[0].path();
    let validated = a::matrix(&root).unwrap();
    assert_eq!(validated.len(), 8);
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&root)
        .env("SWARMAIL_BIN", "/gone")
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(1), "{}", stderr(&replay));
    assert_eq!(String::from_utf8_lossy(&replay.stdout).lines().count(), 8);
    let index = load(&root.join("matrix.json"));
    let path = index["metadata"][7]["path"].as_str().unwrap();
    std::fs::remove_file(root.join(path)).unwrap();
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(2));
    assert!(
        replay.stdout.is_empty(),
        "validate every cell before any output"
    );
}
#[test]
fn supplied_invalid_seeds_times_and_builtin_declarations_launch_nothing() {
    let _trial_scope = trial_scope();
    for args in [
        vec!["--seed", "bad"],
        vec!["--fixture-time", "tomorrow"],
        vec!["--agent-model", "claimed"],
        vec!["--public-argv"],
    ] {
        let out = cli()
            .args(["trial", "--task", "signup", "--agent", "null"])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn helper_agent() {
    if std::env::var(HELPER_ENV).as_deref() == Ok("replace_target") {
        let target =
            std::path::PathBuf::from(std::env::var_os("EVAL_TEST_REPLACE_TARGET").unwrap());
        let replacement = target.with_extension("replacement");
        std::fs::copy("/bin/false", &replacement).unwrap();
        std::fs::rename(replacement, target).unwrap();
        let marker = std::path::PathBuf::from(std::env::var_os(MARKER_ENV).unwrap());
        use std::io::Write;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(marker)
            .unwrap()
            .write_all(b"launched\n")
            .unwrap();
        std::process::exit(0);
    }
    if let Ok(mode) = std::env::var(HELPER_ENV)
        && ["index_failure", "partial_evidence", "published_conflict"].contains(&mode.as_str())
    {
        let root = std::path::PathBuf::from(std::env::var_os(MARKER_ENV).unwrap());
        let run = std::fs::read_dir(&root)
            .unwrap()
            .map(Result::unwrap)
            .find(|e| e.path().is_dir())
            .unwrap()
            .path();
        let plan: a::MatrixPlan =
            serde_json::from_slice(&std::fs::read(run.join("matrix-plan.json")).unwrap()).unwrap();
        let cell = &plan.cells[0];
        match mode.as_str() {
            "index_failure" => std::fs::create_dir(run.join("matrix.json")).unwrap(),
            "partial_evidence" => {
                let trial = run.join("trials").join(&cell.trial_id);
                std::fs::create_dir(&trial).unwrap();
                std::fs::write(trial.join("sentinel"), b"preserve partial evidence").unwrap();
            }
            "published_conflict" => {
                let req = runner::TrialRequest {
                    task: Some(cell.task.clone()),
                    agent: Some(cell.agent.clone()),
                    argv: vec!["/bin/true".into()],
                    deadline_ms: Some(cell.deadline_ms),
                    fixture: cell.fixture.clone(),
                    matrix: Some(cell.coordinates.clone()),
                    trial_id: Some(cell.trial_id.clone()),
                    ..Default::default()
                };
                let row = runner::refused_row(&req, &cell.trial_id);
                a::retain_trial(&run.join("trials"), &row, None).unwrap();
                std::fs::write(
                    root.join("original-metadata"),
                    std::fs::read(
                        run.join("trials")
                            .join(&cell.trial_id)
                            .join("metadata.json"),
                    )
                    .unwrap(),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        std::process::exit(0);
    }
    if std::env::var(HELPER_ENV).as_deref() != Ok("projection") {
        maybe_run_helper();
        return;
    }
    let projection = tokio::runtime::Runtime::new().unwrap().block_on(async {
        use swarmail_evals::{
            mcp::McpClient,
            oracle::{AgentEnv, App, ics_property, labelled_url},
        };
        let env = AgentEnv::from_env().unwrap();
        let mut mcp = McpClient::connect(&env.mcp_url).await.unwrap();
        let count = if env.task == "stale-otp" { 2 } else { 1 };
        let mails = mcp.wait_emails(&env.inbox, None, count).await.unwrap();
        let mut projection = vec![];
        for mail in mails {
            let id = mail["id"].as_str().unwrap();
            let base = env.mcp_url.trim_end_matches("/mcp");
            let raw = reqwest::get(format!("{base}/api/v1/messages/{id}/raw"))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            let headers = raw
                .lines()
                .filter(|l| {
                    l.starts_with("Date:")
                        || l.starts_with("Message-ID:")
                        || l.starts_with("Content-Type: multipart/")
                })
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut value = json!({"headers":headers,"subject":mail["subject"]});
            if env.task == "invite-time" {
                use base64::Engine;
                let part = raw.split("filename=\"invite.ics\"").nth(1).unwrap();
                let b64 = part
                    .split("\r\n\r\n")
                    .nth(1)
                    .unwrap()
                    .lines()
                    .take_while(|l| !l.starts_with("--"))
                    .collect::<String>();
                let attachment = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .unwrap();
                let app = App::new(&env.app_url, &env.identity).unwrap();
                let link =
                    labelled_url(mail["text"].as_str().unwrap(), "Open the invitation:").unwrap();
                let (_, page) = app.get(&link).await.unwrap();
                let (_, download) = app
                    .get_raw(page["ics_url"].as_str().unwrap())
                    .await
                    .unwrap();
                assert_eq!(attachment, download);
                let ics = String::from_utf8(download).unwrap();
                value["calendar"] = json!([
                    ics_property(&ics, "DTSTART"),
                    ics_property(&ics, "DTEND"),
                    ics_property(&ics, "DTSTAMP")
                ]);
            }
            if env.task == "digest-forty" {
                value["target_position"] = json!(
                    mail["text"]
                        .as_str()
                        .unwrap()
                        .lines()
                        .position(|l| l.contains(swarmail_evals::tasks::DIGEST_ITEM))
                        .unwrap()
                );
            }
            projection.push(value);
        }
        projection.sort_by_key(|v| v.to_string());
        projection
    });
    std::fs::write(
        std::env::var_os(MARKER_ENV).unwrap(),
        serde_json::to_vec(&projection).unwrap(),
    )
    .unwrap();
    std::process::exit(0);
}
#[test]
fn fresh_fixture_processes_repeat_only_the_declared_projection() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    for task in ["invite-time", "digest-forty", "stale-otp"] {
        let mut projections = vec![];
        for (n, seed) in [ZERO, ZERO, &"ab".repeat(32)].iter().enumerate() {
            let path = dir.path().join(format!("{task}-{n}.json"));
            let (row, out) = helper_trial(
                "projection",
                &[
                    "--task",
                    task,
                    "--agent",
                    "capture",
                    "--seed",
                    seed,
                    "--fixture-time",
                    TIME,
                ],
                &[(MARKER_ENV, path.to_str().unwrap())],
            );
            assert_ne!(row.outcome, Outcome::HarnessError, "{}", stderr(&out));
            assert_eq!(
                row.provenance.fixture,
                p::FixtureInputs::resolve(Some(seed), Some(TIME)).unwrap()
            );
            projections.push(load(&path));
        }
        assert_eq!(projections[0], projections[1], "{task}");
        if task == "invite-time" {
            assert_eq!(projections[0][0]["calendar"][0], "20261109T173000Z");
            assert_eq!(projections[2][0]["calendar"][0], "20261105T113000Z");
            assert_eq!(projections[0][0]["calendar"][2], "20261024T120000Z");
            assert_eq!(projections[2][0]["calendar"][2], "20261024T120000Z");
        }
    }
}

#[test]
fn catalogue_encoding_and_scorer_definition_have_independent_golden_hashes() {
    let encoded = serde_json::to_vec(&swarmail_evals::tasks::Catalogue {
        schema_version: 2,
        tasks: swarmail_evals::tasks::catalogue(),
    })
    .unwrap();
    assert_eq!(encoded, include_bytes!("data/catalogue-v2.json"));
    assert_eq!(
        p::catalogue_hash(),
        "6068b687475db4be0000fa325de9f2b610b98470526085010ebd648e04862a94"
    );
    assert_eq!(
        p::task_hash("confirm-decoys").unwrap(),
        "2b375c2612263e94edad691de80fa6dc98e8c5f0564c85d97c8db2ff72334fcd"
    );
    assert_eq!(
        p::task_hash("digest-forty").unwrap(),
        "c82c90e57c55e69680b023497d86b2d7900396f369ee9e80f4b67049dcbe0548"
    );
    assert_eq!(
        p::task_hash("signup").unwrap(),
        "6374e444edb41e3f05d98931459199624981d175f7814b1e6242e3f3525ae446"
    );
}

#[test]
fn declared_model_public_command_and_legacy_private_configuration_stay_distinct() {
    let _trial_scope = trial_scope();
    let out = cli()
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "external",
            "--agent-model",
            "declared-model-v1",
            "--agent-config-id",
            "public-config-a",
            "--public-argv",
            "--",
            "/bin/true",
            "literal",
            "λ",
            "",
        ])
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(
        row.provenance.agent.model,
        p::Availability::present("declared-model-v1".into(), p::Source::OperatorDeclared)
    );
    assert_eq!(
        row.provenance.agent.config_id,
        p::Availability::present("public-config-a".into(), p::Source::OperatorDeclared)
    );
    assert_eq!(
        row.provenance.agent.command,
        p::Availability::present(
            p::CommandIdentity {
                algorithm: "public-argv-sha256-v1".into(),
                sha256: p::public_argv(&["/bin/true", "literal", "λ", ""].map(str::to_string))
            },
            p::Source::DeclaredPublicArgv
        )
    );
    let legacy =
        swarmail_evals::report::parse_agents(r#"[{"label":"old-label","argv":["/bin/true"]}]"#)
            .unwrap();
    assert_eq!(
        (
            legacy[0].model.clone(),
            legacy[0].config_id.clone(),
            legacy[0].public_argv
        ),
        (None, None, false)
    );
    let (row, _) = trial(&["--task", "signup", "--agent", "null"]);
    assert_eq!(
        row.provenance.agent.model,
        p::Availability::NotApplicable {
            reason: p::Reason::Builtin
        }
    );
}
#[test]
fn sequential_retained_matrices_preserve_duplicate_occurrences_and_exclusive_ownership() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents.json");
    save(&agents, &json!([{"label":"null","argv":[]}]));
    let root = dir.path().join("fresh-shared-root");
    let start = || {
        cli()
            .args([
                "matrix",
                "--tasks",
                "signup,signup",
                "--trials",
                "1",
                "--agents",
            ])
            .arg(&agents)
            .args(["--seed", ZERO, "--fixture-time", TIME, "--artifacts-dir"])
            .arg(&root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    // #199 admits sequential trials; retain both matrices' full provenance
    // and exclusive artifact ownership assertions on that supported path.
    for _ in 0..2 {
        let out = start().wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
        let text = String::from_utf8(out.stdout).unwrap();
        let rows = text
            .lines()
            .map(|l| results::read(l.as_bytes()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        let a = &rows[0];
        let b = &rows[1];
        assert_eq!(a.provenance.matrix.as_ref().unwrap().task_index, 0);
        assert_eq!(b.provenance.matrix.as_ref().unwrap().task_index, 1);
        assert_eq!(a.provenance.fixture.fixture_time, TIME);
        assert_ne!(
            a.provenance.fixture.seed, b.provenance.fixture.seed,
            "fixed unequal occurrence vectors"
        );
    }
    let runs = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(runs.len(), 2);
    for run in runs {
        assert_eq!(a::matrix(&run.path()).unwrap().len(), 2);
    }
}
#[test]
fn business_replay_never_rewrites_budget_or_infrastructure_history() {
    let _trial_scope = trial_scope();
    let (dir, row) = retained("signup", "oracle");
    let root = dir.path().join(&row.trial_id);
    let result = root.join("result.json");
    for (outcome, class) in [("fail", "budget"), ("harness_error", "harness_error")] {
        let mut original = serde_json::to_value(&row).unwrap();
        original["outcome"] = json!(outcome);
        original["failure_class"] = json!(class);
        save(&result, &original);
        rebind(&root, "result.json");
        let replay = check(&root);
        assert_eq!(replay.status.code(), Some(0), "business replay exit");
        let v: Value = serde_json::from_slice(&replay.stdout).unwrap();
        assert_eq!(v["original_result"], original);
        assert_eq!(v["business_replay"]["verdict"]["outcome"], "pass");
    }
    let mut state = load(&root.join("state.json"));
    state["deliveries"][0]["status"] = json!("parked");
    save(&root.join("state.json"), &state);
    rebind(&root, "state.json");
    let replay = check(&root);
    assert_eq!(replay.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(v["business_replay"]["status"], "unscorable");
    assert_eq!(v["original_result"]["outcome"], "harness_error");
}

#[test]
fn retained_reader_enforces_fixed_paths_declared_lengths_and_all_size_boundaries() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bound.json");
    for (limit, kind) in [
        (a::SMALL_LIMIT, "small"),
        (a::STATE_LIMIT, "state"),
        (a::PLAN_LIMIT, "plan"),
    ] {
        std::fs::write(&file, vec![b' '; limit]).unwrap();
        assert_eq!(
            a::read_file(dir.path(), "bound.json", limit).unwrap().len(),
            limit,
            "{kind}"
        );
        std::fs::write(&file, vec![b' '; limit + 1]).unwrap();
        assert!(
            a::read_file(dir.path(), "bound.json", limit).is_err(),
            "{kind}"
        );
    }
    let (dir, row) = retained("signup", "oracle");
    let root = dir.path().join(&row.trial_id);
    let meta = root.join("metadata.json");
    let original = load(&meta);
    for (key, value) in [
        ("path", json!("../result.json")),
        ("path", json!("/absolute")),
        ("length", json!(a::SMALL_LIMIT + 1)),
        ("sha256", json!("0".repeat(64))),
    ] {
        let mut m = original.clone();
        m["result"][key] = value;
        save(&meta, &m);
        let out = check(&root);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
    save(&meta, &original);
    let mut m = original.clone();
    m["schema_version"] = json!(2);
    save(&meta, &m);
    assert!(a::trial(&root).is_err());
    save(
        &meta,
        &json!({"harness":"swarmail-evals","harness_version":"0.1.0","swarmail_bin":"/old/path","confinement":"unconfined"}),
    );
    assert!(
        a::trial(&root).is_err(),
        "old unversioned metadata is not a bound envelope"
    );
    save(&meta, &original);
    let mut m = original.clone();
    m["state"]["file"]["length"] = json!(u64::MAX);
    save(&meta, &m);
    assert!(a::trial(&root).is_err());
}

#[test]
fn matrix_publication_failures_and_conflicting_child_evidence_never_overwrite_or_complete() {
    let _trial_scope = trial_scope();
    for mode in ["index_failure", "partial_evidence", "published_conflict"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("runs");
        let agents = dir.path().join("agents.json");
        let (_helper_owner, argv) = matrix_helper(mode, &[(MARKER_ENV, root.to_str().unwrap())]);
        save(&agents, &json!([{"label":"fault-helper","argv":argv}]));
        let out = cli()
            .env(HELPER_ENV, mode)
            .env(MARKER_ENV, &root)
            .args(["matrix", "--tasks", "signup", "--trials", "1", "--agents"])
            .arg(agents)
            .arg("--artifacts-dir")
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        let row: runner::TrialResult = serde_json::from_slice(&out.stdout).unwrap();
        row.validate().unwrap();
        assert_eq!(
            row.outcome,
            if mode == "index_failure" {
                Outcome::Fail
            } else {
                Outcome::HarnessError
            },
            "{mode}: {}",
            stderr(&out)
        );
        let run = std::fs::read_dir(&root)
            .unwrap()
            .map(Result::unwrap)
            .find(|e| e.path().is_dir())
            .unwrap()
            .path();
        assert!(a::matrix(&run).is_err());
        let trial = run.join("trials").join(&row.trial_id);
        match mode {
            "partial_evidence" => {
                assert_eq!(
                    std::fs::read(trial.join("sentinel")).unwrap(),
                    b"preserve partial evidence"
                );
                assert!(!trial.join("metadata.json").exists());
            }
            "published_conflict" => {
                assert_eq!(
                    std::fs::read(trial.join("metadata.json")).unwrap(),
                    std::fs::read(root.join("original-metadata")).unwrap()
                );
            }
            "index_failure" => assert!(run.join("matrix.json").is_dir()),
            _ => unreachable!(),
        }
    }
}
#[test]
fn invalid_requests_and_missing_targets_retain_explicit_unscorable_error_evidence() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec!["--agent", "oracle"],
        vec!["--task", "unknown", "--agent", "null"],
        vec!["--task", "signup", "--agent", ""],
    ] {
        let out = cli()
            .arg("trial")
            .args(args)
            .arg("--artifacts-dir")
            .arg(dir.path())
            .output()
            .unwrap();
        let row = one_row(&out);
        assert_eq!(row.outcome, Outcome::HarnessError);
        let replay = check(&dir.path().join(&row.trial_id));
        assert_eq!(replay.status.code(), Some(2));
        assert!(!replay.stdout.is_empty());
    }
    let agents = dir.path().join("agents.json");
    save(
        &agents,
        &json!([{"label":"oracle","argv":[]},{"label":"null","argv":[]} ]),
    );
    let root = dir.path().join("matrix");
    let out = cli()
        .env("SWARMAIL_BIN", "/unavailable-target")
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--trials",
            "2",
            "--agents",
        ])
        .arg(agents)
        .arg("--artifacts-dir")
        .arg(&root)
        .output()
        .unwrap();
    let rows = swarmail_evals::report::parse_rows(&String::from_utf8_lossy(&out.stdout)).unwrap();
    assert_eq!(rows.len(), 8);
    assert!(rows.iter().all(|r| r.outcome == Outcome::HarnessError));
    assert_eq!(out.status.code(), Some(2));
    let run = std::fs::read_dir(root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let replay = a::matrix(&run).unwrap();
    assert_eq!(replay.len(), 8);
    assert_eq!(a::replay_exit(&replay), 2);
}

#[test]
fn replacing_a_measured_target_fails_affected_cells_without_relaunch_or_invented_source() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("swarmail");
    std::fs::copy(swarmail_bin(), &target).unwrap();
    let digest = swarmail_evals::identity::file_hash(&target).unwrap();
    let marker = dir.path().join("launched");
    let agents = dir.path().join("agents.json");
    let (_helper_owner, argv) = matrix_helper(
        "replace_target",
        &[
            (MARKER_ENV, marker.to_str().unwrap()),
            ("EVAL_TEST_REPLACE_TARGET", target.to_str().unwrap()),
        ],
    );
    save(&agents, &json!([{"label":"replacer","argv":argv}]));
    let out = cli()
        .env("SWARMAIL_BIN", &target)
        .env(HELPER_ENV, "replace_target")
        .env(MARKER_ENV, &marker)
        .args(["matrix", "--tasks", "signup", "--trials", "2", "--agents"])
        .arg(agents)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    let text = String::from_utf8(out.stdout).unwrap();
    let rows = swarmail_evals::report::parse_rows(&text).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.outcome == Outcome::HarnessError));
    assert_eq!(
        std::fs::read_to_string(marker).unwrap(),
        "launched\n",
        "second cell refuses before launch"
    );
    for line in text.lines() {
        let row = results::read(line.as_bytes()).unwrap();
        assert_eq!(
            row.provenance.target.executable_sha256,
            p::Availability::present(digest.clone(), p::Source::MeasuredBytes)
        );
        assert_eq!(
            row.provenance.target.build_commit,
            p::Availability::unavailable(p::Reason::ExternalSourceUnknown)
        );
    }
}

#[test]
fn a_retained_snapshot_requires_nullable_keys_without_changing_legacy_bare_state() {
    let _trial_scope = trial_scope();
    let (dir, row) = retained("signup", "oracle");
    let root = dir.path().join(&row.trial_id);
    let path = root.join("state.json");
    let original = load(&path);
    for key in ["claimed", "flow", "category"] {
        let mut state = original.clone();
        state["events"][0].as_object_mut().unwrap().remove(key);
        save(&path, &state);
        rebind(&root, "state.json");
        assert!(a::trial(&root).is_err(), "required event {key}");
        assert!(
            serde_json::from_value::<swarmail_evals::state::Snapshot>(state).is_ok(),
            "bare-state shape preserved"
        );
    }
    let (other, other_row) = retained("signup", "oracle");
    std::fs::copy(
        other.path().join(other_row.trial_id).join("state.json"),
        &path,
    )
    .unwrap();
    rebind(&root, "state.json");
    assert!(
        a::trial(&root).is_err(),
        "actual swapped snapshot refuses even with recomputed hashes"
    );
}

fn no_private_bytes(bytes: &[u8], canaries: &[String]) {
    let text = String::from_utf8_lossy(bytes);
    assert!(
        canaries.iter().all(|c| !text.contains(c)),
        "private claimant excluded from public bytes"
    );
}
fn scan_tree(root: &Path, canaries: &[String]) {
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scan_tree(&path, canaries);
        } else {
            no_private_bytes(&std::fs::read(path).unwrap(), canaries);
        }
    }
}
fn canaries(root: &Path) -> Vec<String> {
    std::fs::read_dir(root)
        .unwrap()
        .flat_map(|e| {
            serde_json::from_slice::<Vec<String>>(&std::fs::read(e.unwrap().path()).unwrap())
                .unwrap()
        })
        .collect()
}
#[test]
fn external_private_claims_are_absent_from_trial_matrix_replay_and_failure_publication() {
    let _trial_scope = trial_scope();
    use swarmail_evals::state::FailureClass;
    for mode in [
        "private_claims",
        "private_claims_then_oracle",
        "private_claims_budget",
        "private_claims_retention_failure",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        let root = dir.path().join("published");
        let (row, out) = helper_trial(
            mode,
            &[
                "--task",
                "otp",
                "--agent",
                "claim-proof",
                "--deadline-ms",
                if mode == "private_claims_budget" {
                    "5000"
                } else {
                    "10000"
                },
                "--artifacts-dir",
                root.to_str().unwrap(),
            ],
            &[
                (MARKER_ENV, private.path().to_str().unwrap()),
                ("EVAL_TEST_RETENTION_PATH", root.to_str().unwrap()),
            ],
        );
        let canaries = canaries(private.path());
        assert_eq!(canaries.len(), 2);
        let expected = match mode {
            "private_claims_then_oracle" => None,
            "private_claims_budget" => Some(FailureClass::Budget),
            "private_claims_retention_failure" => Some(FailureClass::HarnessError),
            _ => Some(FailureClass::ToolMisuse),
        };
        assert_eq!(row.failure_class, expected);
        no_private_bytes(&out.stdout, &canaries);
        no_private_bytes(&out.stderr, &canaries);
        if root.is_dir() {
            scan_tree(&root, &canaries);
            let trial = root.join(&row.trial_id);
            let replay = check(&trial);
            no_private_bytes(&replay.stdout, &canaries);
            no_private_bytes(&replay.stderr, &canaries);
            assert_eq!(
                replay.status.code(),
                Some(if mode == "private_claims_then_oracle" {
                    0
                } else {
                    1
                })
            );
            let state = load(&trial.join("state.json"));
            assert!(
                state["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .take(2)
                    .all(|e| e["claimed"].is_null())
            );
        } else {
            assert!(!root.join(&row.trial_id).join("metadata.json").exists());
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let (_helper_owner, argv) = matrix_helper(
        "private_claims",
        &[(MARKER_ENV, private.path().to_str().unwrap())],
    );
    let agents = dir.path().join("agents.json");
    save(&agents, &json!([{"label":"claim-proof", "argv":argv}]));
    let root = dir.path().join("published");
    let out = cli()
        .env(HELPER_ENV, "private_claims")
        .env(MARKER_ENV, private.path())
        .args(["matrix", "--tasks", "otp,otp", "--trials", "1", "--agents"])
        .arg(agents)
        .arg("--artifacts-dir")
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let canaries = canaries(private.path());
    assert_eq!(canaries.len(), 4);
    scan_tree(&root, &canaries);
    no_private_bytes(&out.stdout, &canaries);
    no_private_bytes(&out.stderr, &canaries);
    let run = std::fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&run)
        .env("SWARMAIL_BIN", "/gone")
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(1));
    no_private_bytes(&replay.stdout, &canaries);
    no_private_bytes(&replay.stderr, &canaries);
    for line in String::from_utf8_lossy(&replay.stdout).lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["original_result"]["failure_class"], "tool_misuse");
        assert_eq!(
            v["business_replay"]["verdict"]["failure_class"],
            "tool_misuse"
        );
    }
}

#[test]
fn publication_guard_refuses_supplied_state_before_any_result_or_state_bytes() {
    let _trial_scope = trial_scope();
    let (dir, row) = retained("signup", "oracle");
    let mut snap: swarmail_evals::state::Snapshot =
        serde_json::from_value(load(&dir.path().join(&row.trial_id).join("state.json"))).unwrap();
    snap.events[0].claimed = Some("private-writer-canary".into());
    let root = tempfile::tempdir().unwrap();
    let req = runner::TrialRequest {
        artifacts_dir: Some(root.path().into()),
        ..Default::default()
    };
    assert_eq!(
        a::retain_trial(root.path(), &row, Some(&snap)).unwrap_err(),
        "noncanonical snapshot claim"
    );
    let dest = root.path().join(&row.trial_id);
    assert_eq!(std::fs::read_dir(&dest).unwrap().count(), 0);
    // Preserve exclusive partial ownership, including the finalizer's error path.
    let root = tempfile::tempdir().unwrap();
    let req = runner::TrialRequest {
        artifacts_dir: Some(root.path().into()),
        ..req
    };
    let refused = runner::finalize(&req, row.clone(), Some(&snap));
    assert_eq!(refused.outcome, Outcome::HarnessError);
    assert_eq!(refused.exit_code(), 2);
    assert_eq!(
        std::fs::read_dir(root.path().join(&row.trial_id))
            .unwrap()
            .count(),
        0
    );
    assert!(a::trial(&root.path().join(&row.trial_id)).is_err());
}

fn tree_bytes(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = vec![];
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(tree_bytes(&path));
        } else {
            out.push((path.clone(), std::fs::read(path).unwrap()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
fn bare(task: &str, path: &Path) -> std::process::Output {
    cli()
        .args(["check", "--task", task, "--state"])
        .arg(path)
        .env("SWARMAIL_BIN", "/gone")
        .output()
        .unwrap()
}
#[test]
fn historical_unknown_claims_score_without_mutation_and_reader_errors_do_not_reflect_input() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let (row, _) = helper_trial(
        "private_claims",
        &[
            "--task",
            "otp",
            "--agent",
            "claim-proof",
            "--artifacts-dir",
            dir.path().to_str().unwrap(),
        ],
        &[(MARKER_ENV, private.path().to_str().unwrap())],
    );
    let root = dir.path().join(&row.trial_id);
    let path = root.join("state.json");
    let original_state = load(&path);
    let canary = "private-historical-reader-canary".to_string();
    let canaries = [canary.clone()];
    let original_result = load(&root.join("result.json"));
    for claimed in [None, Some(Value::Null), Some(json!(canary))] {
        let mut historical = original_state.clone();
        match claimed {
            None => {
                historical["events"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("claimed");
            }
            Some(v) => {
                historical["events"][0]["claimed"] = v;
            }
        }
        save(&path, &historical);
        rebind(&root, "state.json");
        let before = tree_bytes(&root);
        let legacy = bare("otp", &path);
        assert_eq!(legacy.status.code(), Some(1));
        let verdict: Value = serde_json::from_slice(&legacy.stdout).unwrap();
        assert_eq!(
            verdict,
            json!({"task":"otp","outcome":"fail","failure_class":"tool_misuse"})
        );
        no_private_bytes(&legacy.stdout, &canaries);
        no_private_bytes(&legacy.stderr, &canaries);
        if historical["events"][0].get("claimed").is_some() {
            let replay = check(&root);
            assert_eq!(replay.status.code(), Some(1));
            let value: Value = serde_json::from_slice(&replay.stdout).unwrap();
            assert_eq!(value["original_result"], original_result);
            assert_eq!(
                value["business_replay"]["verdict"]["failure_class"],
                "tool_misuse"
            );
            no_private_bytes(&replay.stdout, &canaries);
            no_private_bytes(&replay.stderr, &canaries);
            assert!(a::trial(&root).is_ok());
        } else {
            assert!(a::trial(&root).is_err());
        }
        assert!(
            before == tree_bytes(&root),
            "readers leave historical evidence untouched"
        );
    }
    let mut cases = vec![];
    let mut v = original_state.clone();
    v[&canary] = json!(true);
    cases.push(v);
    let mut v = original_state.clone();
    v["events"][0]["claimed"] = json!({&canary:true});
    cases.push(v);
    let mut v = original_state.clone();
    v["identities"][0]["identity"] = json!(canary);
    let duplicate = v["identities"][0].clone();
    v["identities"].as_array_mut().unwrap().push(duplicate);
    cases.push(v);
    let mut v = original_state.clone();
    v["identities"][0]["identity"] = json!(canary);
    v["identities"][0]["facts"]["confirmed"] = json!(true);
    cases.push(v);
    let mut v = original_state.clone();
    v["task"] = json!(canary);
    cases.push(v);
    let mut v = original_state.clone();
    v["identities"][0]["facts"]["reset_count"] = json!(canary);
    cases.push(v);
    for malformed in cases {
        save(&path, &malformed);
        rebind(&root, "state.json");
        let before = tree_bytes(&root);
        let legacy = bare("otp", &path);
        assert_eq!(legacy.status.code(), Some(2));
        let v: Value = serde_json::from_slice(&legacy.stdout).unwrap();
        assert_eq!(v["outcome"], "harness_error");
        no_private_bytes(&legacy.stdout, &canaries);
        no_private_bytes(&legacy.stderr, &canaries);
        let replay = check(&root);
        assert_eq!(replay.status.code(), Some(2));
        assert!(replay.stdout.is_empty());
        no_private_bytes(&replay.stderr, &canaries);
        let err = a::trial(&root).unwrap_err();
        no_private_bytes(err.as_bytes(), &canaries);
        assert!(
            before == tree_bytes(&root),
            "invalid evidence remains untouched"
        );
    }
    save(&path, &original_state);
    rebind(&root, "state.json");
    let result_path = root.join("result.json");
    for key in ["extra", "schema_version"] {
        let mut bad = original_result.clone();
        if key == "extra" {
            bad[&canary] = json!(true);
        } else {
            bad[key] = json!(canary);
        }
        save(&result_path, &bad);
        rebind(&root, "result.json");
        let err = a::trial(&root).unwrap_err();
        no_private_bytes(err.as_bytes(), &canaries);
        let replay = check(&root);
        assert_eq!(replay.status.code(), Some(2));
        assert!(replay.stdout.is_empty());
        no_private_bytes(&replay.stderr, &canaries);
    }
    for output in [bare(&canary, &path), bare("otp", &dir.path().join(&canary))] {
        assert_eq!(output.status.code(), Some(2));
        no_private_bytes(&output.stdout, &canaries);
        no_private_bytes(&output.stderr, &canaries);
    }
}

#[test]
fn retained_matrix_historical_claims_remain_scoreable_and_a_bad_later_cell_emits_no_rows() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents.json");
    save(&agents, &json!([{"label":"null","argv":[]}]));
    let out = cli()
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--trials",
            "1",
            "--agents",
        ])
        .arg(&agents)
        .arg("--artifacts-dir")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let root = std::fs::read_dir(dir.path())
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.path().is_dir())
        .unwrap()
        .path();
    let mut index = load(&root.join("matrix.json"));
    let canary = "private-matrix-history-canary".to_string();
    let mut states = vec![];
    for metadata in index["metadata"].as_array_mut().unwrap() {
        let meta_path = root.join(metadata["path"].as_str().unwrap());
        let trial = meta_path.parent().unwrap();
        let path = trial.join("state.json");
        let mut state = load(&path);
        let seq = state["deliveries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["seq"].as_u64().unwrap())
            .max()
            .unwrap()
            + 1;
        state["events"] = json!([{"seq":seq,"claimed":canary,"flow":null,"action":"unknown_action","accepted":false,"category":"tool_misuse"}]);
        save(&path, &state);
        rebind(trial, "state.json");
        let mut binding = bind(trial, "metadata.json");
        binding["path"] = metadata["path"].clone();
        *metadata = binding;
        states.push(path);
    }
    save(&root.join("matrix.json"), &index);
    let before = tree_bytes(&root);
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&root)
        .env("SWARMAIL_BIN", "/gone")
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&replay.stdout).lines().count(), 2);
    no_private_bytes(&replay.stdout, std::slice::from_ref(&canary));
    no_private_bytes(&replay.stderr, std::slice::from_ref(&canary));
    assert!(
        before == tree_bytes(&root),
        "matrix replay leaves history untouched"
    );
    for line in String::from_utf8_lossy(&replay.stdout).lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["original_result"]["failure_class"], "gave_up");
        assert_eq!(
            v["business_replay"]["verdict"]["failure_class"],
            "tool_misuse"
        );
    }
    let path = &states[1];
    let trial = path.parent().unwrap();
    let mut malformed = load(path);
    malformed[&canary] = json!(true);
    save(path, &malformed);
    rebind(trial, "state.json");
    let mut binding = bind(trial, "metadata.json");
    binding["path"] = index["metadata"][1]["path"].clone();
    index["metadata"][1] = binding;
    save(&root.join("matrix.json"), &index);
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(2));
    assert!(replay.stdout.is_empty());
    no_private_bytes(&replay.stderr, std::slice::from_ref(&canary));
    no_private_bytes(
        a::matrix(&root).unwrap_err().as_bytes(),
        std::slice::from_ref(&canary),
    );
}

#[test]
fn retained_plan_coordinates_versions_and_cross_cell_results_refuse_even_with_new_bindings() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents.json");
    save(&agents, &json!([{"label":"null","argv":[]}]));
    let out = cli()
        .args([
            "matrix",
            "--tasks",
            "signup,otp",
            "--trials",
            "1",
            "--agents",
        ])
        .arg(&agents)
        .arg("--artifacts-dir")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let root = std::fs::read_dir(dir.path())
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.path().is_dir())
        .unwrap()
        .path();
    let plan_path = root.join("matrix-plan.json");
    let index_path = root.join("matrix.json");
    let plan = load(&plan_path);
    let index = load(&index_path);
    let mut cases = vec![];
    let mut v = plan.clone();
    v["schema_version"] = json!(2);
    cases.push(v);
    let mut v = plan.clone();
    v["cells"][1]["coordinates"] = v["cells"][0]["coordinates"].clone();
    cases.push(v);
    let mut v = plan.clone();
    v["cells"][1]["fixture"]["seed"] = json!("f".repeat(64));
    cases.push(v);
    let mut v = plan.clone();
    v["cells"].as_array_mut().unwrap().pop();
    cases.push(v);
    let mut v = plan.clone();
    v["cells"][1]["trial_id"] = v["cells"][0]["trial_id"].clone();
    cases.push(v);
    for bad in cases {
        save(&plan_path, &bad);
        let mut bound = index.clone();
        bound["plan"] = bind(&root, "matrix-plan.json");
        save(&index_path, &bound);
        let out = cli()
            .args(["check", "--matrix"])
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
    save(&plan_path, &plan);
    for missing in [false, true] {
        let mut bad = index.clone();
        if missing {
            bad["metadata"].as_array_mut().unwrap().pop();
        } else {
            bad["metadata"][1] = bad["metadata"][0].clone();
        }
        save(&index_path, &bad);
        let out = cli()
            .args(["check", "--matrix"])
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
    let trial = root
        .join(index["metadata"][0]["path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf();
    let other = root
        .join(index["metadata"][1]["path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf();
    let result_path = trial.join("result.json");
    let original = load(&result_path);
    for kind in ["catalogue", "scorer", "cross_cell"] {
        let mut bad = original.clone();
        match kind {
            "catalogue" => bad["provenance"]["task"]["catalogue_sha256"] = json!("f".repeat(64)),
            "scorer" => bad["provenance"]["task"]["scorer_id"] = json!("business-score-v999"),
            _ => bad = load(&other.join("result.json")),
        }
        save(&result_path, &bad);
        rebind(&trial, "result.json");
        let mut bound = index.clone();
        let mut meta = bind(&trial, "metadata.json");
        meta["path"] = bound["metadata"][0]["path"].clone();
        bound["metadata"][0] = meta;
        save(&index_path, &bound);
        let out = cli()
            .args(["check", "--matrix"])
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn public_native_argv_hashes_the_original_declaration_and_non_utf8_opt_in_refuses() {
    let _trial_scope = trial_scope();
    use std::{
        ffi::OsString,
        os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    };
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("agent");
    let marker = dir.path().join("launched");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ntouch '{}'\nexec '{}' agent oracle\n",
            marker.display(),
            exe()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let argv = ["./agent", "λ", "--seed", "literal"];
    let out = cli()
        .current_dir(dir.path())
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "external",
            "--public-argv",
            "--",
        ])
        .args(argv)
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Pass);
    assert_eq!(
        row.provenance.agent.command,
        p::Availability::present(
            p::CommandIdentity {
                algorithm: "public-argv-sha256-v1".into(),
                sha256: p::public_argv(&argv.map(str::to_string))
            },
            p::Source::DeclaredPublicArgv
        )
    );
    std::fs::remove_file(&marker).unwrap();
    let out = cli()
        .current_dir(dir.path())
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "external",
            "--public-argv",
            "--",
            "./agent",
        ])
        .arg(OsString::from_vec(b"private-native-canary-\xff".to_vec()))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!marker.exists());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("private-native-canary"));
}

#[test]
fn retained_agent_input_accepts_the_exact_bound_and_refuses_one_extra_byte_before_admission() {
    let _trial_scope = trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents.json");
    let mut bytes = br#"[{"label":"null","argv":[]}]"#.to_vec();
    bytes.resize(1024 * 1024, b' ');
    std::fs::write(&agents, &bytes).unwrap();
    let root = dir.path().join("evidence");
    let out = cli()
        .env("SWARMAIL_BIN", "/unavailable-bound-proof")
        .args(["matrix", "--tasks", "signup", "--trials", "1", "--agents"])
        .arg(&agents)
        .arg("--artifacts-dir")
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(one_row(&out).outcome, Outcome::HarnessError);
    let run = std::fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(a::matrix(&run).unwrap().len(), 1);
    bytes.push(b' ');
    std::fs::write(&agents, &bytes).unwrap();
    let untouched = dir.path().join("must-not-create");
    let out = cli()
        .args(["matrix", "--tasks", "signup", "--trials", "1", "--agents"])
        .arg(&agents)
        .arg("--artifacts-dir")
        .arg(&untouched)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!untouched.exists());
}
