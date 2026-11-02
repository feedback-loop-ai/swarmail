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

/// How the prompt file renders the request and the result path.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rendering {
    /// No request line at all.
    NoRequest,
    /// The round-1 shape: the request and the result path each on a line of
    /// their own, flush left.
    Bare,
    /// The engine's rendering, observed 2026-11-02: the feature text follows
    /// `Feature: ` and the result path is an indented line of its own.
    Engine,
    /// The engine's rendering with every line ending `\r\n`.
    EngineCrlf,
}

/// Write the request and a prompt carrying it; run one script; return the
/// parsed result file.
fn run_script(fx: &Fixture, script: &str, request: &str, with_request_line: bool) -> Value {
    let rendering = if with_request_line {
        Rendering::Bare
    } else {
        Rendering::NoRequest
    };
    run_script_rendered(fx, script, request, rendering)
}

fn render_prompt(
    request_path: &std::path::Path,
    result: &std::path::Path,
    how: Rendering,
) -> String {
    let (eol, request_line, result_line) = match how {
        Rendering::NoRequest => ("\n", None, result.display().to_string()),
        Rendering::Bare => (
            "\n",
            Some(format!("swarmail-eval-request: {}", request_path.display())),
            result.display().to_string(),
        ),
        Rendering::Engine | Rendering::EngineCrlf => (
            if how == Rendering::EngineCrlf {
                "\r\n"
            } else {
                "\n"
            },
            Some(format!(
                "Feature: swarmail-eval-request: {}",
                request_path.display()
            )),
            format!("    {}", result.display()),
        ),
    };
    let mut lines = vec!["A seat prompt.".to_string(), String::new()];
    lines.extend(request_line);
    lines.push("Phase: trial".to_string());
    lines.push("Write your result to exactly this file:".to_string());
    lines.push(result_line);
    lines.join(eol) + eol
}

fn run_script_rendered(fx: &Fixture, script: &str, request: &str, how: Rendering) -> Value {
    let request_path = fx.slot.join("request.json");
    std::fs::write(&request_path, request).unwrap();
    let result = fx.dir.path().join("result.json");
    let _ = std::fs::remove_file(&result);
    let prompt = fx.dir.path().join("prompt.txt");
    std::fs::write(&prompt, render_prompt(&request_path, &result, how)).unwrap();
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
fn trial_script_parses_the_engine_rendered_prompt() {
    let _trial_scope = trial_scope();
    for (label, how) in [
        ("engine", Rendering::Engine),
        ("engine, CRLF", Rendering::EngineCrlf),
        ("bare", Rendering::Bare),
    ] {
        let fx = fixture();
        let id = format!("render-{}", label.replace([',', ' '], ""));
        let text = request(&fx, &id, trial_args("oracle")).to_string();
        let result = run_script_rendered(&fx, "trial.sh", &text, how);
        assert_contract(&result, "ran");
        assert_eq!(result["inputs"], json!({"trial_id": id}), "{label}");
        let row: Value =
            serde_json::from_str(&std::fs::read_to_string(fx.slot.join("row.json")).unwrap())
                .unwrap();
        assert_eq!(row["outcome"], "pass", "{label}");
    }
}

#[test]
fn the_scripts_still_refuse_a_relative_request_path_in_the_engine_rendering() {
    for script in ["trial.sh", "score.sh"] {
        let fx = fixture();
        let result_path = fx.dir.path().join("result.json");
        let prompt = fx.dir.path().join("prompt.txt");
        std::fs::write(
            &prompt,
            format!(
                "Feature: swarmail-eval-request: relative/request.json\n    {}\n",
                result_path.display()
            ),
        )
        .unwrap();
        let out = std::process::Command::new("bash")
            .arg(recipe_dir().join("scripts").join(script))
            .arg(&prompt)
            .output()
            .unwrap();
        assert!(out.status.success());
        let result: Value = serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap();
        let expected = if script == "trial.sh" {
            "harness_error"
        } else {
            "unscorable"
        };
        assert_contract(&result, expected);
        assert!(!fx.slot.join("row.json").exists());
        assert!(!fx.slot.join("score.json").exists());
    }
}

#[test]
fn score_script_parses_the_engine_rendered_prompt() {
    let _trial_scope = trial_scope();
    for how in [Rendering::Engine, Rendering::EngineCrlf, Rendering::Bare] {
        let fx = fixture();
        let id = retained_leaf(&fx, "oracle");
        let text = request(&fx, &id, json!([])).to_string();
        let result = run_script_rendered(&fx, "score.sh", &text, how);
        assert_contract(&result, "pass");
        assert!(fx.slot.join("score.json").is_file());
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
        let score_text = std::fs::read_to_string(fx.slot.join("score.json")).unwrap();
        assert_eq!(score_text.lines().count(), 1, "the verdict is one line");
        let score: Value = serde_json::from_str(&score_text).unwrap();
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
    // check ran and could not score the missing leaf: the verdict is recorded.
    let score: Value =
        serde_json::from_slice(&std::fs::read(fx.slot.join("score.json")).unwrap()).unwrap();
    assert_eq!(score["outcome"], "harness_error");
    assert_eq!(score["exit"], 2);

    // A request the gate cannot read means check never ran: no verdict file,
    // so the matrix sees `score_missing`, never a verdict nobody reached.
    for (text, with_line) in [("{", true), ("{}", false)] {
        let fx = fixture();
        let result = run_script(&fx, "score.sh", text, with_line);
        assert_contract(&result, "unscorable");
        assert!(
            !fx.slot.join("score.json").exists(),
            "an unrun gate writes no verdict"
        );
    }
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

fn digest(s: &str) -> String {
    swarmail_evals::provenance::sha256(s.as_bytes())
}

fn tool_row(tool: &str, outcome: &str) -> Value {
    json!({"tool": tool, "outcome": outcome, "argument_bytes": 61,
           "argument_sha256": digest(tool), "result_bytes": 2048, "result_sha256": digest(outcome)})
}

fn full_usage() -> Value {
    json!({
        "input_tokens": 1200, "output_tokens": 300, "cached_tokens": 4000,
        "turns": 7, "wall_time_ms": 18400, "deadline_killed": false,
        "tool_calls": [
            tool_row("mcp__swarmail__wait", "ok"),
            tool_row("mcp__fixture__eval_http", "invalid_argument"),
        ]
    })
}

fn standin_export() -> Value {
    let mut bad_tool = full_usage();
    bad_tool["tool_calls"][1]["outcome"] = json!("exploded");
    // An empty tool list is a real fact; a non-boolean kill flag is not one.
    let mut empty_list = full_usage();
    empty_list["tool_calls"] = json!([]);
    empty_list["deadline_killed"] = json!("no");
    // Tool rows the journal did not give: absent, never an empty list.
    let mut null_rows = full_usage();
    null_rows["tool_calls"] = Value::Null;
    null_rows.as_object_mut().unwrap().remove("deadline_killed");
    let seat = |trial: &str, usage: Value| json!({"seat": "trial", "result": "ran", "inputs": {"trial_id": trial}, "usage": usage});
    json!({"runs": [
        {"run_id": "run-a", "seats": [seat(&fixed_id(1), full_usage()), {"seat": "score", "result": "pass", "inputs": {}}]},
        {"run_id": "run-b", "seats": [seat(&fixed_id(2), Value::Null)]},
        {"run_id": "run-c", "seats": [seat(&fixed_id(3), json!({"turns": 1}))]},
        {"run_id": "run-d", "seats": [seat(&fixed_id(3), json!({"turns": 2}))]},
        {"run_id": "run-e", "seats": [seat(&fixed_id(5), bad_tool)]},
        {"run_id": "run-f", "seats": [{"seat": "implement", "result": "complete", "inputs": {"trial_id": fixed_id(6)}}]},
        {"run_id": "run-g", "seats": [seat(&fixed_id(7), empty_list)]},
        {"run_id": "run-h", "seats": [seat(&fixed_id(8), null_rows)]}
    ]})
}

fn run_facts(args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = std::process::Command::new("bash");
    cmd.arg(journal_script())
        .args(args)
        .stdin(std::process::Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

/// A stand-in brokkr: `--version` prints `version` (and fails when it is
/// empty); `export` prints the file at `export` or fails when that is `None`.
fn fake_brokkr(
    fx: &Fixture,
    name: &str,
    version: &str,
    export: Option<&std::path::Path>,
) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = fx.dir.path().join(name);
    let version_line = if version.is_empty() {
        "exit 1".to_string()
    } else {
        format!("echo 'brokkr {version}'; exit 0")
    };
    let export_line = match export {
        Some(file) => format!("[ \"$1\" = export ] && exec cat {}", file.display()),
        None => "[ \"$1\" = export ] && exit 7".to_string(),
    };
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then {version_line}; fi\n{export_line}\nexit 9\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn journal_facts_script_output_reads_as_facts_v1() {
    use swarmail_evals::journal;
    let fx = fixture();
    let export = fx.dir.path().join("export.json");
    std::fs::write(&export, standin_export().to_string()).unwrap();
    let fake = fake_brokkr(&fx, "brokkr", "0.12.0", Some(&export));
    let db = fx.dir.path().join("forge.db");
    std::fs::write(&db, b"not a real database; the stand-in ignores it").unwrap();
    let ids: Vec<String> = (1..=8).map(fixed_id).collect();
    let id_args: Vec<&str> = ids.iter().map(String::as_str).collect();
    let facts_names = [
        "tokens",
        "turns",
        "wall_time_ms",
        "deadline_killed",
        "tool_calls",
    ];

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
        let out = run_facts(&args, &env);
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
        // Six of eight ids are carried; 4 is in no run and 6 only in a seat
        // that is not the trial seat.
        assert_eq!(trials.len(), 6);
        assert!(
            by_id(4).is_none(),
            "an absent trial is omitted, not invented"
        );
        assert!(by_id(6).is_none(), "only the trial seat is read");
        let first = by_id(1).unwrap();
        assert_eq!(first["run_id"], "run-a");
        assert_eq!(
            first["tokens"],
            json!({"status": "present", "value": {"input": 1200, "output": 300, "cached": 4000}})
        );
        assert_eq!(first["turns"], json!({"status": "present", "value": 7}));
        assert_eq!(
            first["deadline_killed"],
            json!({"status": "present", "value": false})
        );
        assert_eq!(first["tool_calls"]["status"], "present");
        assert_eq!(first["tool_calls"]["value"].as_array().unwrap().len(), 2);
        assert_eq!(first["tool_calls"]["value"][1]["ordinal"], 2);
        assert_eq!(
            first["tool_calls"]["value"][1]["outcome"],
            "invalid_argument"
        );
        let none = json!({"status": "unavailable", "reason": "journal_has_no_accounting"});
        for fact in facts_names {
            assert_eq!(by_id(2).unwrap()[fact], none, "a run with no usage: {fact}");
        }
        // Each fact is typed on its own: an unrecognised tool row loses the
        // tool rows and nothing else.
        let bad = by_id(5).unwrap();
        assert_eq!(bad["tool_calls"], none);
        assert_eq!(bad["tokens"]["status"], "present");
        assert_eq!(
            bad["deadline_killed"],
            json!({"status": "present", "value": false})
        );
        // A present empty list is "zero calls"; a non-boolean flag is absence.
        let empty = by_id(7).unwrap();
        assert_eq!(
            empty["tool_calls"],
            json!({"status": "present", "value": []})
        );
        assert_eq!(empty["deadline_killed"], none);
        assert_eq!(empty["tokens"]["status"], "present");
        // Absent and null tool rows are unavailable, never an empty list.
        let null_rows = by_id(8).unwrap();
        assert_eq!(null_rows["tool_calls"], none);
        assert_eq!(null_rows["deadline_killed"], none);
        assert_eq!(null_rows["turns"]["status"], "present");
        let ambiguous = by_id(3).unwrap();
        for fact in facts_names {
            assert_eq!(
                ambiguous[fact],
                json!({"status": "unavailable", "reason": "journal_ambiguous"}),
                "{fact}"
            );
        }

        // Joined to rows, the absent trials are journal_missing and the
        // ambiguous one is typed, never a number.
        let rows: Vec<_> = (1..=8)
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
        assert_eq!(out["joined"], 4, "trials 1, 5, 7 and 8 have tokens");
        assert_eq!(
            out["missing"],
            json!({"journal_ambiguous": 1, "journal_has_no_accounting": 1, "journal_missing": 2})
        );
        assert_eq!(out["tokens_per_success"], Value::Null);
        assert_eq!(out["tokens_per_success_reason"], "incomplete_join");
        assert_eq!(out["tool_calls"], Value::Null);
        assert_eq!(out["tool_calls_reason"], "incomplete_join");
        assert_eq!(out["deadline_killed"], Value::Null);
    }
}

#[test]
fn journal_facts_script_refuses_without_writing_a_file() {
    let fx = fixture();
    let export = fx.dir.path().join("export.json");
    std::fs::write(&export, standin_export().to_string()).unwrap();
    let fake = fake_brokkr(&fx, "brokkr", "0.12.0", Some(&export));
    let out_path = fx.dir.path().join("out.json");
    let fake_s = fake.to_str().unwrap();
    let out_s = out_path.to_str().unwrap();
    let id = fixed_id(1);
    let saved = [("JOURNAL_EXPORT_FILE", export.to_str().unwrap())];
    let not_json = fx.dir.path().join("garbage.json");
    std::fs::write(&not_json, "not json at all").unwrap();
    let garbage = [("JOURNAL_EXPORT_FILE", not_json.to_str().unwrap())];
    // Documents that are JSON but not the shape the script reads.
    let mut shapes = Vec::new();
    for (name, body) in [
        ("runs-is-a-string.json", r#"{"runs": "nope"}"#),
        ("unrelated.json", r#"{"unrelated": 1}"#),
        (
            "seat-without-name.json",
            r#"{"runs": [{"run_id": "r", "seats": [{}]}]}"#,
        ),
        ("run-without-id.json", r#"{"runs": [{"seats": []}]}"#),
        ("empty.json", ""),
    ] {
        let path = fx.dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        shapes.push((name, path));
    }
    let versionless = fake_brokkr(&fx, "brokkr-no-version", "", Some(&export));
    let failing_export = fake_brokkr(&fx, "brokkr-export-fails", "0.12.0", None);
    let db = fx.dir.path().join("forge.db");
    std::fs::write(&db, b"db").unwrap();
    let db_s = db.to_str().unwrap();
    type Case<'a> = (
        String,
        Vec<&'a str>,
        Vec<(&'a str, &'a str)>,
        Option<&'a str>,
    );
    let mut cases: Vec<Case> = vec![
        (
            "relative brokkr".into(),
            vec!["brokkr", "/db", out_s, id.as_str()],
            saved.to_vec(),
            None,
        ),
        (
            "relative output".into(),
            vec![fake_s, "/db", "out.json", id.as_str()],
            saved.to_vec(),
            None,
        ),
        (
            "no trial ids".into(),
            vec![fake_s, "/db", out_s],
            saved.to_vec(),
            None,
        ),
        (
            "path-like id".into(),
            vec![fake_s, "/db", out_s, "../x"],
            saved.to_vec(),
            None,
        ),
        (
            "export is not JSON".into(),
            vec![fake_s, "/db", out_s, id.as_str()],
            garbage.to_vec(),
            Some("export_not_json"),
        ),
        (
            "no database for a live export".into(),
            vec![fake_s, "/nonexistent.db", out_s, id.as_str()],
            vec![],
            None,
        ),
        (
            "brokkr export fails".into(),
            vec![failing_export.to_str().unwrap(), db_s, out_s, id.as_str()],
            vec![],
            Some("export_failed"),
        ),
        (
            "the version is unreadable".into(),
            vec![versionless.to_str().unwrap(), db_s, out_s, id.as_str()],
            saved.to_vec(),
            Some("export_version_unreadable"),
        ),
    ];
    for (name, path) in &shapes {
        cases.push((
            format!("shape mismatch: {name}"),
            vec![fake_s, "/db", out_s, id.as_str()],
            vec![("JOURNAL_EXPORT_FILE", path.to_str().unwrap())],
            Some("export_shape_mismatch"),
        ));
    }
    for (label, args, env, class) in &cases {
        let out = run_facts(args, env);
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(2), "{label}: {err}");
        if let Some(class) = class {
            assert!(err.contains(class), "{label}: wanted {class} in {err}");
        }
        assert!(!out_path.exists(), "{label}: no output file");
    }
    // An existing output is never overwritten.
    std::fs::write(&out_path, "precious").unwrap();
    let out = run_facts(&[fake_s, "/db", out_s, id.as_str()], &saved);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(std::fs::read_to_string(&out_path).unwrap(), "precious");
}

#[test]
fn a_matching_export_with_no_run_for_a_trial_yields_journal_missing_by_absence() {
    use swarmail_evals::journal;
    let fx = fixture();
    let export = fx.dir.path().join("empty-runs.json");
    std::fs::write(&export, r#"{"runs": []}"#).unwrap();
    let fake = fake_brokkr(&fx, "brokkr", "0.12.0", Some(&export));
    let out_path = fx.dir.path().join("out.json");
    let id = fixed_id(1);
    let out = run_facts(
        &[
            fake.to_str().unwrap(),
            "/db",
            out_path.to_str().unwrap(),
            id.as_str(),
        ],
        &[("JOURNAL_EXPORT_FILE", export.to_str().unwrap())],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let facts = journal::read(&std::fs::read(&out_path).unwrap()).unwrap();
    assert!(facts.trials.is_empty(), "absence, not an invented entry");
    let rows = vec![synthetic_row(
        "signup",
        "agent",
        true,
        swarmail_evals::tasks::Outcome::Pass,
        10,
        &id,
    )];
    let joined = journal::join(&rows, &facts).unwrap();
    let group: Vec<_> = rows.iter().collect();
    let summary = journal::summarize(&group, &joined);
    assert_eq!(summary["missing"], json!({"journal_missing": 1}));
}

// ---- the matrix environment never crosses the seats (decision 0116, F1) ----

#[test]
fn the_trial_seat_strips_the_matrix_environment() {
    // Seam 1 of the four that sever any accounting channel (0116 section 6):
    // trial.sh runs the trial under `env -i`. The sentinel is set in the
    // environment of the process that runs the script, as the matrix's would
    // be, and must not reach the agent the trial launches. The agent here is a
    // probe that records its environment keys. The follow-up that forwards the
    // observed channel flips this test to the observed name.
    let _trial_scope = trial_scope();
    let fx = fixture();
    let record = fx.dir.path().join("agent-env.txt");
    let probe = fx.dir.path().join("probe.sh");
    std::fs::write(
        &probe,
        format!(
            "#!/bin/sh\nenv | sed 's/=.*//' | sort > {}\nexit 0\n",
            record.display()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let args = json!(["--task", "signup", "--agent", "probe", "--", probe]);
    let request_path = fx.slot.join("request.json");
    std::fs::write(&request_path, request(&fx, "seam-1", args).to_string()).unwrap();
    let result = fx.dir.path().join("result.json");
    let prompt = fx.dir.path().join("prompt.txt");
    std::fs::write(
        &prompt,
        render_prompt(&request_path, &result, Rendering::Engine),
    )
    .unwrap();
    let out = std::process::Command::new("bash")
        .arg(recipe_dir().join("scripts/trial.sh"))
        .arg(&prompt)
        .env("SWARMAIL_EVAL_SEAM_SENTINEL", "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let keys = std::fs::read_to_string(&record)
        .expect("the probe agent ran and recorded its environment keys");
    assert!(
        !keys.contains("SWARMAIL_EVAL_SEAM_SENTINEL"),
        "the sentinel crossed the seat: {keys}"
    );
    assert!(
        keys.lines().all(|k| !k.contains("SENTINEL")),
        "no inherited key survives: {keys}"
    );
}
