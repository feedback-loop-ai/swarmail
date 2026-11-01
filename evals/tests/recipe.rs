//! The eval recipe's two exec scripts (decision 0116), run directly under
//! bash with a synthesized prompt file, exactly as the engine's exec driver
//! would run them: `trial.sh` runs `swarmail-evals trial` from the matrix's
//! request, `score.sh` re-applies the check to the retained leaf. The engine
//! itself is not present; these tests prove everything the scripts do.

mod common;

use common::*;
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn helper_agent() {
    maybe_run_helper();
}

/// A slot directory and the pieces a request names.
struct Fixture {
    dir: tempfile::TempDir,
    slot: PathBuf,
    trials: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempdir();
    let slot = dir.path().join("slot");
    let trials = dir.path().join("trials");
    std::fs::create_dir(&slot).unwrap();
    std::fs::create_dir(&trials).unwrap();
    Fixture { dir, slot, trials }
}

fn request(fx: &Fixture, trial_id: &str, args: Value) -> Value {
    json!({
        "schema_version": 1,
        "trial_id": trial_id,
        "slot": fx.slot,
        "trials_root": fx.trials,
        "evals_bin": exe(),
        "swarmail_bin": swarmail_bin(),
        "args": args,
    })
}

/// Write the request and a prompt carrying it; run one script; return the
/// parsed result file.
fn run_script(fx: &Fixture, script: &str, request: &str, with_request_line: bool) -> Value {
    let request_path = fx.slot.join("request.json");
    std::fs::write(&request_path, request).unwrap();
    let result = fx.dir.path().join("result.json");
    let _ = std::fs::remove_file(&result);
    let prompt = fx.dir.path().join("prompt.txt");
    let mut text = String::from("A seat prompt.\n");
    if with_request_line {
        text.push_str(&format!(
            "swarmail-eval-request: {}\n",
            request_path.display()
        ));
    }
    text.push_str(&format!("{}\n", result.display()));
    std::fs::write(&prompt, text).unwrap();
    let out = std::process::Command::new("bash")
        .arg(recipe_dir().join("scripts").join(script))
        .arg(&prompt)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{script} wrote a result and exited 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&std::fs::read(&result).expect("the script wrote its result")).unwrap()
}

fn assert_contract(result: &Value, expected: &str) {
    let keys: std::collections::BTreeSet<_> = result.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        ["inputs", "notes", "result"].map(String::from).into(),
        "exactly result, inputs and notes: {result}"
    );
    assert_eq!(result["result"], expected, "{result}");
}

fn trial_args(agent: &str) -> Value {
    json!(["--task", "signup", "--agent", agent])
}

#[test]
fn the_trial_script_runs_the_trial_and_reports_ran() {
    let _trial_scope = trial_scope();
    for (agent, code, outcome) in [("oracle", "0", "pass"), ("null", "1", "fail")] {
        let fx = fixture();
        let id = format!("{agent}-slot-1");
        let result = run_script(
            &fx,
            "trial.sh",
            &request(&fx, &id, trial_args(agent)).to_string(),
            true,
        );
        assert_contract(&result, "ran");
        assert_eq!(result["inputs"], json!({"trial_id": id}));
        let row_text = std::fs::read_to_string(fx.slot.join("row.json")).unwrap();
        assert_eq!(row_text.lines().count(), 1, "exactly one row line");
        let row: Value = serde_json::from_str(&row_text).unwrap();
        assert_eq!(row["outcome"], outcome, "{agent}");
        assert_eq!(row["confinement"], "unconfined");
        assert_eq!(
            std::fs::read_to_string(fx.slot.join("exit"))
                .unwrap()
                .trim(),
            code
        );
        assert!(fx.slot.join("trial.stderr").is_file());
        let notes = result["notes"].as_str().unwrap();
        assert!(
            !notes.contains(fx.dir.path().to_str().unwrap()),
            "no path under the root reaches a result: {notes}"
        );
    }
}

#[test]
fn the_trial_script_refuses_malformed_requests_without_running_anything() {
    let fx = fixture();
    let good = request(&fx, "refused-1", trial_args("oracle"));
    let mut missing_args = good.clone();
    missing_args.as_object_mut().unwrap().remove("args");
    let mut relative_bin = good.clone();
    relative_bin["evals_bin"] = json!("swarmail-evals");
    let mut bad_id = good.clone();
    bad_id["trial_id"] = json!("../escape");
    let mut bad_version = good.clone();
    bad_version["schema_version"] = json!(2);
    let mut oversize = good.clone();
    oversize["args"] = json!(["x".repeat(70 * 1024)]);
    let mut absent_swarmail = good.clone();
    absent_swarmail["swarmail_bin"] = json!("/nonexistent/swarmail");
    let mut absent_slot = good.clone();
    absent_slot["slot"] = json!("/nonexistent/slot");
    for (label, text, with_line) in [
        ("no request line", good.to_string(), false),
        ("missing args", missing_args.to_string(), true),
        ("relative evals_bin", relative_bin.to_string(), true),
        ("path-like trial id", bad_id.to_string(), true),
        ("wrong schema version", bad_version.to_string(), true),
        ("over 64 KiB", oversize.to_string(), true),
        ("absent swarmail", absent_swarmail.to_string(), true),
        ("absent slot", absent_slot.to_string(), true),
        ("not JSON", "{".to_string(), true),
    ] {
        let result = run_script(&fx, "trial.sh", &text, with_line);
        assert_contract(&result, "harness_error");
        assert_eq!(result["inputs"], json!({}), "{label}");
        assert!(
            !fx.slot.join("row.json").exists(),
            "{label}: no row was created"
        );
        let notes = result["notes"].as_str().unwrap();
        assert!(
            !notes.contains(fx.dir.path().to_str().unwrap()),
            "{label}: {notes}"
        );
    }
}

#[test]
fn the_trial_script_never_overwrites_a_row_or_reports_a_missing_one() {
    let _trial_scope = trial_scope();
    let fx = fixture();
    let text = request(&fx, "once-1", trial_args("oracle")).to_string();
    assert_contract(&run_script(&fx, "trial.sh", &text, true), "ran");
    let first = std::fs::read(fx.slot.join("row.json")).unwrap();
    let again = run_script(&fx, "trial.sh", &text, true);
    assert_contract(&again, "harness_error");
    assert_eq!(
        std::fs::read(fx.slot.join("row.json")).unwrap(),
        first,
        "a second run never clobbers the first row"
    );
    // An unknown task yields a harness-error row and exit 2: that is a trial
    // that ran (the matrix and the gate then rule on it), not a missing row.
    let fx = fixture();
    let bad = request(
        &fx,
        "badtask-1",
        json!(["--task", "no-such-task", "--agent", "null"]),
    );
    let result = run_script(&fx, "trial.sh", &bad.to_string(), true);
    assert_contract(&result, "ran");
    assert_eq!(
        std::fs::read_to_string(fx.slot.join("exit"))
            .unwrap()
            .trim(),
        "2"
    );
    // A command line the CLI refuses prints no row at all: never reported as ran.
    let fx = fixture();
    let bad = request(&fx, "norow-1", json!(["--no-such-flag"]));
    let result = run_script(&fx, "trial.sh", &bad.to_string(), true);
    assert_contract(&result, "harness_error");
    assert_eq!(result["inputs"], json!({}));
    assert_eq!(
        std::fs::read_to_string(fx.slot.join("exit"))
            .unwrap()
            .trim(),
        "2"
    );
}

/// Retain a leaf the way the matrix's trial does and return its id.
fn retained_leaf(fx: &Fixture, agent: &str) -> String {
    let (row, out) = trial(&[
        "--task",
        "signup",
        "--agent",
        agent,
        "--artifacts-dir",
        fx.trials.to_str().unwrap(),
    ]);
    assert!(fx.trials.join(&row.trial_id).is_dir(), "{}", stderr(&out));
    row.trial_id
}

#[test]
fn the_score_script_maps_check_exit_codes() {
    let _trial_scope = trial_scope();
    for (agent, expected, outcome, code) in
        [("oracle", "pass", "pass", 0), ("null", "fail", "fail", 1)]
    {
        let fx = fixture();
        let id = retained_leaf(&fx, agent);
        let result = run_script(
            &fx,
            "score.sh",
            &request(&fx, &id, json!([])).to_string(),
            true,
        );
        assert_contract(&result, expected);
        let score: Value =
            serde_json::from_slice(&std::fs::read(fx.slot.join("score.json")).unwrap()).unwrap();
        assert_eq!(
            score,
            json!({"schema_version": 1, "trial_id": id, "outcome": outcome, "exit": code})
        );
        let notes = result["notes"].as_str().unwrap();
        assert!(!notes.contains(fx.dir.path().to_str().unwrap()), "{notes}");
    }

    // A leaf whose state was truncated after the trial is unscorable, and the
    // verdict says so rather than passing or failing it.
    let fx = fixture();
    let id = retained_leaf(&fx, "oracle");
    let state = fx.trials.join(&id).join("state.json");
    let bytes = std::fs::read(&state).unwrap();
    std::fs::write(&state, &bytes[..bytes.len() / 2]).unwrap();
    let result = run_script(
        &fx,
        "score.sh",
        &request(&fx, &id, json!([])).to_string(),
        true,
    );
    assert_contract(&result, "unscorable");
    let score: Value =
        serde_json::from_slice(&std::fs::read(fx.slot.join("score.json")).unwrap()).unwrap();
    assert_eq!(score["outcome"], "harness_error");
    assert_eq!(score["exit"], 2);

    // A leaf that does not exist, and a malformed request, are unscorable.
    let fx = fixture();
    let result = run_script(
        &fx,
        "score.sh",
        &request(&fx, "no-leaf-1", json!([])).to_string(),
        true,
    );
    assert_contract(&result, "unscorable");
    let result = run_script(&fx, "score.sh", "{", true);
    assert_contract(&result, "unscorable");
    let result = run_script(&fx, "score.sh", "{}", false);
    assert_contract(&result, "unscorable");
}

#[test]
fn the_scripts_read_only_their_prompt_and_name_no_secret() {
    // The scripts are bash plus jq and the candidate's own executables: no
    // network tool, no model, no read of an agent's account.
    for script in ["trial.sh", "score.sh"] {
        let text = std::fs::read_to_string(recipe_dir().join("scripts").join(script)).unwrap();
        for forbidden in [
            "curl",
            "wget",
            "ssh",
            "nc ",
            "git push",
            "ANTHROPIC",
            "OPENAI",
            "claude ",
            "codex ",
        ] {
            assert!(!text.contains(forbidden), "{script} mentions {forbidden}");
        }
        assert!(text.contains("set -u"), "{script}");
    }
}

// ---- evals/scripts/journal-facts.sh (decision 0116, F3) -------------------
//
// brokkr was not reachable when this was written, so the export below is a
// STAND-IN in the shape the script documents, not a captured real export. The
// operator's first engine run replaces it with a capture.

fn journal_script() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/journal-facts.sh")
}

fn standin_export() -> Value {
    let digest = |s: &str| swarmail_evals::provenance::sha256(s.as_bytes());
    let usage = json!({
        "input_tokens": 1200, "output_tokens": 300, "cached_tokens": 4000,
        "turns": 7, "wall_time_ms": 18400, "deadline_killed": false,
        "tool_calls": [
            {"tool": "mcp__swarmail__wait", "outcome": "ok", "argument_bytes": 61,
             "argument_sha256": digest("a"), "result_bytes": 2048, "result_sha256": digest("r")},
            {"tool": "mcp__fixture__eval_http", "outcome": "invalid_argument", "argument_bytes": 9,
             "argument_sha256": digest("b"), "result_bytes": 40, "result_sha256": digest("s")}
        ]
    });
    let mut bad_tool = usage.clone();
    bad_tool["tool_calls"][1]["outcome"] = json!("exploded");
    let seat = |trial: &str, usage: Value| json!({"seat": "trial", "result": "ran", "inputs": {"trial_id": trial}, "usage": usage});
    json!({"runs": [
        {"run_id": "run-a", "seats": [seat(&fixed_id(1), usage), {"seat": "score", "result": "pass", "inputs": {}}]},
        {"run_id": "run-b", "seats": [seat(&fixed_id(2), Value::Null)]},
        {"run_id": "run-c", "seats": [seat(&fixed_id(3), json!({"turns": 1}))]},
        {"run_id": "run-d", "seats": [seat(&fixed_id(3), json!({"turns": 2}))]},
        {"run_id": "run-e", "seats": [seat(&fixed_id(5), bad_tool)]},
        {"run_id": "run-f", "seats": [{"seat": "implement", "result": "complete", "inputs": {"trial_id": fixed_id(6)}}]}
    ]})
}

fn run_facts(fx: &Fixture, args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = std::process::Command::new("bash");
    cmd.arg(journal_script())
        .args(args)
        .stdin(std::process::Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let _ = fx;
    cmd.output().unwrap()
}

#[test]
fn journal_facts_script_output_reads_as_facts_v1() {
    use swarmail_evals::journal;
    let fx = fixture();
    let export = fx.dir.path().join("export.json");
    std::fs::write(&export, standin_export().to_string()).unwrap();
    // A stand-in brokkr: --version and `export --db`.
    let fake = fx.dir.path().join("brokkr");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'brokkr 0.12.0'; exit 0; fi\n[ \"$1\" = export ] && exec cat {}\nexit 9\n",
            export.display()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let db = fx.dir.path().join("forge.db");
    std::fs::write(&db, b"not a real database; the stand-in ignores it").unwrap();
    let ids: Vec<String> = [1, 2, 3, 4, 5].into_iter().map(fixed_id).collect();
    let id_args: Vec<&str> = ids.iter().map(String::as_str).collect();

    for saved in [false, true] {
        let out_path = fx
            .dir
            .path()
            .join(if saved { "saved.json" } else { "live.json" });
        let mut args = vec![
            fake.to_str().unwrap(),
            db.to_str().unwrap(),
            out_path.to_str().unwrap(),
        ];
        args.extend(&id_args);
        let env: Vec<(&str, &str)> = if saved {
            vec![("JOURNAL_EXPORT_FILE", export.to_str().unwrap())]
        } else {
            vec![]
        };
        let out = run_facts(&fx, &args, &env);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let facts = journal::read(&std::fs::read(&out_path).unwrap()).expect("reads as facts v1");
        assert_eq!(facts.source.brokkr_version, "0.12.0");
        assert_eq!(
            facts.source.binary_sha256,
            swarmail_evals::provenance::sha256(&std::fs::read(&fake).unwrap())
        );
        assert!(facts.source.query.starts_with(if saved {
            "saved export sha256:"
        } else {
            "brokkr export"
        }));
        let value: Value = serde_json::from_slice(&std::fs::read(&out_path).unwrap()).unwrap();
        let trials = value["trials"].as_array().unwrap();
        let by_id = |n: u32| trials.iter().find(|t| t["trial_id"] == fixed_id(n));
        // Four of five ids are carried; id 4 is absent from the export.
        assert_eq!(trials.len(), 4);
        assert!(
            by_id(4).is_none(),
            "an absent trial is omitted, not invented"
        );
        let first = by_id(1).unwrap();
        assert_eq!(first["run_id"], "run-a");
        assert_eq!(
            first["tokens"],
            json!({"status": "present", "value": {"input": 1200, "output": 300, "cached": 4000}})
        );
        assert_eq!(first["turns"], json!({"status": "present", "value": 7}));
        assert_eq!(first["tool_calls"].as_array().unwrap().len(), 2);
        assert_eq!(first["tool_calls"][1]["ordinal"], 2);
        assert_eq!(first["tool_calls"][1]["outcome"], "invalid_argument");
        let none = json!({"status": "unavailable", "reason": "journal_has_no_accounting"});
        assert_eq!(by_id(2).unwrap()["tokens"], none, "a run with no usage");
        assert_eq!(
            by_id(5).unwrap()["tokens"],
            none,
            "an unrecognised tool row withholds the whole entry"
        );
        assert_eq!(by_id(5).unwrap()["tool_calls"], json!([]));
        let ambiguous = by_id(3).unwrap();
        assert_eq!(
            ambiguous["tokens"],
            json!({"status": "unavailable", "reason": "journal_ambiguous"})
        );
        assert_eq!(ambiguous["turns"]["reason"], "journal_ambiguous");

        // Joined to rows, the absent trial is journal_missing and the
        // ambiguous one is typed, never a number.
        let rows: Vec<_> = (1..=5)
            .map(|n| {
                synthetic_row(
                    "signup",
                    "agent",
                    true,
                    swarmail_evals::tasks::Outcome::Pass,
                    10,
                    &fixed_id(n),
                )
            })
            .collect();
        let summary = journal::join(&rows, &facts).unwrap();
        let group: Vec<_> = rows.iter().collect();
        let out = journal::summarize(&group, &summary);
        assert_eq!(out["joined"], 1);
        assert_eq!(
            out["missing"],
            json!({"journal_ambiguous": 1, "journal_has_no_accounting": 2, "journal_missing": 1})
        );
        assert_eq!(out["tokens_per_success"], Value::Null);
        assert_eq!(out["tokens_per_success_reason"], "incomplete_join");
    }
}

#[test]
fn journal_facts_script_refuses_without_writing_a_file() {
    let fx = fixture();
    let export = fx.dir.path().join("export.json");
    std::fs::write(&export, standin_export().to_string()).unwrap();
    let fake = fx.dir.path().join("brokkr");
    std::fs::write(
        &fake,
        "#!/bin/sh\n[ \"$1\" = --version ] && echo 'brokkr 0.12.0'\nexit 0\n",
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out_path = fx.dir.path().join("out.json");
    let fake_s = fake.to_str().unwrap();
    let out_s = out_path.to_str().unwrap();
    let id = fixed_id(1);
    let saved = [("JOURNAL_EXPORT_FILE", export.to_str().unwrap())];
    let not_json = fx.dir.path().join("garbage.json");
    std::fs::write(&not_json, "not json at all").unwrap();
    let garbage = [("JOURNAL_EXPORT_FILE", not_json.to_str().unwrap())];
    for (label, args, env) in [
        (
            "relative brokkr",
            vec!["brokkr", "/db", out_s, id.as_str()],
            &saved[..],
        ),
        (
            "relative output",
            vec![fake_s, "/db", "out.json", id.as_str()],
            &saved[..],
        ),
        ("no trial ids", vec![fake_s, "/db", out_s], &saved[..]),
        (
            "path-like id",
            vec![fake_s, "/db", out_s, "../x"],
            &saved[..],
        ),
        (
            "export is not JSON",
            vec![fake_s, "/db", out_s, id.as_str()],
            &garbage[..],
        ),
        (
            "no database for a live export",
            vec![fake_s, "/nonexistent.db", out_s, id.as_str()],
            &[][..],
        ),
    ] {
        let out = run_facts(&fx, &args, env);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{label}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out_path.exists(), "{label}: no output file");
    }
    // An existing output is never overwritten.
    std::fs::write(&out_path, "precious").unwrap();
    let out = run_facts(&fx, &[fake_s, "/db", out_s, id.as_str()], &saved);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(std::fs::read_to_string(&out_path).unwrap(), "precious");
}
