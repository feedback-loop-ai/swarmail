//! The eval recipe's two exec scripts (decision 0119), run directly under
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
        slot_row(&fx, &id);
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

/// What `trial.sh` leaves beside a retained leaf: the slot's `row.json` is the
/// row the trial printed, which is the leaf's `result.json`.
fn slot_row(fx: &Fixture, id: &str) {
    std::fs::copy(
        fx.trials.join(id).join("result.json"),
        fx.slot.join("row.json"),
    )
    .unwrap();
}

/// Leave the slot the way the runner's precedence legs would have: the row,
/// the retained `result.json` and its digest in `metadata.json` are rewritten
/// together, as `provenance.rs`'s `rebind` does (the deadline and
/// infrastructure legs cannot be raced offline, 0020 section 5).
fn rewrite_row(fx: &Fixture, id: &str, outcome: &str, class: Value) {
    let leaf = fx.trials.join(id);
    let mut row: Value =
        serde_json::from_slice(&std::fs::read(leaf.join("result.json")).unwrap()).unwrap();
    row["outcome"] = json!(outcome);
    row["failure_class"] = class;
    let bytes = serde_json::to_vec(&row).unwrap();
    std::fs::write(leaf.join("result.json"), &bytes).unwrap();
    let mut meta: Value =
        serde_json::from_slice(&std::fs::read(leaf.join("metadata.json")).unwrap()).unwrap();
    meta["result"]["length"] = json!(bytes.len());
    meta["result"]["sha256"] = json!(swarmail_evals::provenance::sha256(&bytes));
    std::fs::write(
        leaf.join("metadata.json"),
        serde_json::to_vec(&meta).unwrap(),
    )
    .unwrap();
    slot_row(fx, id);
}

fn truncate_state(fx: &Fixture, id: &str) {
    let state = fx.trials.join(id).join("state.json");
    let bytes = std::fs::read(&state).unwrap();
    std::fs::write(&state, &bytes[..bytes.len() / 2]).unwrap();
}

/// Run `score.sh` on a slot and return the seat's result word and the
/// `score.json` it left (if any).
fn score(fx: &Fixture, id: &str) -> (Value, Option<Value>) {
    let result = run_script(
        fx,
        "score.sh",
        &request(fx, id, json!([])).to_string(),
        true,
    );
    let notes = result["notes"].as_str().unwrap();
    assert!(!notes.contains(fx.dir.path().to_str().unwrap()), "{notes}");
    let score = std::fs::read(fx.slot.join("score.json"))
        .ok()
        .map(|b| serde_json::from_slice(&b).unwrap());
    (result, score)
}

#[test]
fn the_score_script_maps_check_exit_codes() {
    let _trial_scope = trial_scope();
    for (agent, expected, business, code) in
        [("oracle", "pass", "pass", 0), ("null", "fail", "fail", 1)]
    {
        let fx = fixture();
        let id = retained_leaf(&fx, agent);
        slot_row(&fx, &id);
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
            json!({"schema_version": 1, "trial_id": id, "business": business, "check_exit": code}),
            "the business replay is named `business`, never `outcome`"
        );
        let notes = result["notes"].as_str().unwrap();
        assert!(!notes.contains(fx.dir.path().to_str().unwrap()), "{notes}");
    }

    // A leaf whose state was truncated after the trial is unscorable, and the
    // verdict says so rather than passing or failing it.
    let fx = fixture();
    let id = retained_leaf(&fx, "oracle");
    slot_row(&fx, &id);
    truncate_state(&fx, &id);
    let (result, score) = score(&fx, &id);
    assert_contract(&result, "unscorable");
    let score = score.expect("check ran, so the verdict is recorded");
    assert_eq!(score["business"], "unscorable");
    assert_eq!(score["check_exit"], 2);
    assert!(score.get("outcome").is_none());

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
    assert_eq!(score["business"], "unscorable");
    assert_eq!(score["check_exit"], 2);

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

/// One case per cell of the agreement table (spec "Revision 3"): the seat
/// reports the trial's class when the business replay agrees with the row, and
/// `unscorable` for a `harness_error` row, a contradiction or an unreadable row.
/// `score.json` always names the business replay and `check_exit`.
#[test]
fn the_score_script_follows_the_agreement_table() {
    let _trial_scope = trial_scope();
    type Prepare = fn(&Fixture, &str);
    let cells: [(&str, &str, Prepare, &str, &str); 7] = [
        (
            "pass row, passing state",
            "oracle",
            |_, _| {},
            "pass",
            "pass",
        ),
        (
            "business fail row, failing state",
            "null",
            |_, _| {},
            "fail",
            "fail",
        ),
        (
            "budget row over a passing state",
            "oracle",
            |fx, id| rewrite_row(fx, id, "fail", json!("budget")),
            "fail",
            "pass",
        ),
        (
            "harness_error row over a passing state",
            "oracle",
            |fx, id| rewrite_row(fx, id, "harness_error", json!("harness_error")),
            "unscorable",
            "pass",
        ),
        (
            "pass row over a failing state",
            "null",
            |fx, id| rewrite_row(fx, id, "pass", Value::Null),
            "unscorable",
            "fail",
        ),
        (
            "budget row over a truncated state",
            "oracle",
            |fx, id| {
                rewrite_row(fx, id, "fail", json!("budget"));
                truncate_state(fx, id);
            },
            "unscorable",
            "unscorable",
        ),
        (
            "business fail row over a passing state",
            "oracle",
            |fx, id| rewrite_row(fx, id, "fail", json!("wrong_message")),
            "unscorable",
            "pass",
        ),
    ];
    for (cell, agent, prepare, seat, business) in cells {
        let fx = fixture();
        let id = retained_leaf(&fx, agent);
        slot_row(&fx, &id);
        prepare(&fx, &id);
        let (result, score) = score(&fx, &id);
        assert_contract(&result, seat);
        let score = score.unwrap_or_else(|| panic!("{cell}: check ran, so a verdict is recorded"));
        assert_eq!(score["business"], business, "{cell}");
        assert!(score["check_exit"].is_u64(), "{cell}");
        assert!(score.get("outcome").is_none(), "{cell}");
    }

    // A row the gate cannot read leaves the seat unscorable, with the verdict
    // still written: the matrix then reports `trial_row_missing` from its own read.
    type Spoil = fn(&Fixture);
    let spoils: [(&str, Spoil); 3] = [
        ("row.json missing", |fx| {
            std::fs::remove_file(fx.slot.join("row.json")).unwrap()
        }),
        ("row.json a symlink", |fx| {
            let real = fx.slot.join("row.real");
            std::fs::rename(fx.slot.join("row.json"), &real).unwrap();
            std::os::unix::fs::symlink(&real, fx.slot.join("row.json")).unwrap();
        }),
        ("row.json two lines", |fx| {
            let mut text = std::fs::read_to_string(fx.slot.join("row.json")).unwrap();
            text.push_str(&text.clone());
            std::fs::write(fx.slot.join("row.json"), text).unwrap();
        }),
    ];
    for (cell, spoil) in spoils {
        let fx = fixture();
        let id = retained_leaf(&fx, "oracle");
        slot_row(&fx, &id);
        spoil(&fx);
        let (result, score) = score(&fx, &id);
        assert_contract(&result, "unscorable");
        let score = score.unwrap_or_else(|| panic!("{cell}: the verdict is still written"));
        assert_eq!(score["business"], "pass", "{cell}");
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

// ---- evals/scripts/journal-facts.sh (decision 0119, F3) -------------------
//
// The real export shape: `brokkr export --run RUN --out DIR` writes
// DIR/RUN.ndjson. tests/data/journal-export-smoke.ndjson is the sanitized
// export of the 2026-11-23 direct claude smoke (run smoke-4a42634a), with
// only the trial seat's `inputs.trial_id` set to fixed_id(1); the multi-seat
// run below is built in the same event shape.

fn journal_script() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/journal-facts.sh")
}

fn smoke_export() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/journal-export-smoke.ndjson"),
    )
    .unwrap()
}

/// One run's events: interleaved effects, a retried trial attempt, a review
/// seat with its own bill, ambiguous and unaccounted trials.
fn multi_seat_export() -> String {
    let mut seq = 0;
    let mut lines = Vec::new();
    let mut ev = |kind: &str, attempt: Option<&str>, payload: Value| {
        seq += 1;
        let mut e = json!({"type": kind, "run_id": "run-multi", "seq": seq, "payload": payload});
        if let Some(a) = attempt {
            e["attempt_id"] = json!(a);
        }
        lines.push(e.to_string());
    };
    let final_step = |i: u64, o: u64, c: u64| json!({"step": "claude-code-session-finished", "input_tokens": i, "output_tokens": o, "cache_read_tokens": c, "num_turns": 3, "exit_code": 0});
    let cp = |effect: &str, attempt: &str, data: Value| {
        (
            effect.to_string(),
            attempt.to_string(),
            json!({"effect_id": effect, "attempt_id": attempt, "checkpoint": data}),
        )
    };
    let req = |effect: &str, seat: &str| json!({"effect_id": effect, "seat": seat, "phase": seat});
    let ok = |effect: &str, attempt: &str, trial: Option<String>| {
        let inputs = trial.map_or(json!({}), |t| json!({"trial_id": t}));
        json!({"effect_id": effect, "attempt_id": attempt, "result": {"result": "ran", "inputs": inputs}})
    };
    for (effect, seat) in [
        ("t2", "trial"),
        ("rv", "review"),
        ("t3a", "trial"),
        ("t3b", "trial"),
        ("t4", "trial"),
        ("t5", "trial"),
        ("t6", "trial"),
        ("t7", "trial"),
        ("t8", "review"),
    ] {
        ev("effect/requested", None, req(effect, seat));
    }
    // Trial 2: attempt a1 fails after a big final; a2 succeeds. Review
    // checkpoints interleave with a2's.
    let script = [
        cp("t2", "a1", final_step(9000, 9000, 1)),
        cp(
            "t2",
            "a2",
            json!({"step": "seat-turn", "input_tokens": 70, "output_tokens": 7}),
        ),
        cp("rv", "r1", final_step(5555, 555, 5)),
        cp("t2", "a2", final_step(100, 10, 50)),
        cp(
            "t4",
            "c1",
            json!({"step": "exec-session-finished", "exit_code": 0}),
        ),
        cp("t5", "e1", final_step(40, 4, 0)),
        cp("t5", "e1", final_step(40, 4, 0)),
        cp("t6", "f1", final_step(40, 4, 0)),
        cp("t6", "f1", final_step(41, 4, 0)),
        cp(
            "t7",
            "g1",
            json!({"step": "codex-session-finished", "input_tokens": 8, "output_tokens": 2, "num_turns": 1}),
        ),
    ];
    for (_, attempt, payload) in script {
        ev("effect/checkpointed", Some(&attempt), payload);
    }
    ev(
        "effect/failed",
        Some("a1"),
        json!({"effect_id": "t2", "attempt_id": "a1", "error": "x"}),
    );
    for (effect, attempt, trial) in [
        ("t2", "a2", 2),
        ("rv", "r1", 2),
        ("t3a", "b1", 3),
        ("t3b", "b2", 3),
        ("t4", "c1", 4),
        ("t5", "e1", 5),
        ("t6", "f1", 6),
        ("t7", "g1", 7),
        ("t8", "h1", 8),
    ] {
        ev(
            "effect/succeeded",
            Some(attempt),
            ok(effect, attempt, Some(fixed_id(trial))),
        );
    }
    lines.join("\n") + "\n"
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
/// empty); `export --db DB --run RUN --out DIR` copies `exports/RUN.ndjson`
/// into DIR, or does `export` instead of copying when it is given.
fn fake_brokkr(fx: &Fixture, name: &str, version: &str, export: Option<&str>) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = fx.dir.path().join(name);
    let exports = fx.dir.path().join("exports");
    let version_line = if version.is_empty() {
        "exit 1".to_string()
    } else {
        format!("echo 'brokkr {version}'; exit 0")
    };
    let export_line = export.map_or_else(
        || {
            format!(
                "cp \"{}/$5.ndjson\" \"$7/$5.ndjson\"; exit $?",
                exports.display()
            )
        },
        str::to_string,
    );
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then {version_line}; fi\nif [ \"$1\" = export ] && [ \"$2\" = --db ] && [ \"$4\" = --run ] && [ \"$6\" = --out ] && [ \"$#\" = 7 ]; then {export_line}; fi\nexit 64\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn saved_exports(fx: &Fixture) -> PathBuf {
    let dir = fx.dir.path().join("exports");
    if !dir.exists() {
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("smoke-4a42634a.ndjson"), smoke_export()).unwrap();
        std::fs::write(dir.join("run-multi.ndjson"), multi_seat_export()).unwrap();
    }
    dir
}

#[test]
fn journal_facts_script_reads_the_real_export_shape() {
    use swarmail_evals::journal;
    let fx = fixture();
    let saved = saved_exports(&fx);
    let fake = fake_brokkr(&fx, "brokkr", "0.15.2", None);
    let db = fx.dir.path().join("forge.db");
    std::fs::write(&db, b"not a real database; the stand-in ignores it").unwrap();
    let ids: Vec<String> = (1..=9).map(fixed_id).collect();
    let none = json!({"status": "unavailable", "reason": "journal_has_no_accounting"});
    for live in [true, false] {
        let out_path = fx
            .dir
            .path()
            .join(if live { "live.json" } else { "saved.json" });
        let mut args = vec![
            fake.to_str().unwrap(),
            db.to_str().unwrap(),
            out_path.to_str().unwrap(),
        ];
        args.extend(["--run", "smoke-4a42634a", "--run", "run-multi"]);
        args.extend(ids.iter().map(String::as_str));
        let env = if live {
            vec![]
        } else {
            vec![("JOURNAL_EXPORT_DIR", saved.to_str().unwrap())]
        };
        let out = run_facts(&args, &env);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let facts = journal::read(&std::fs::read(&out_path).unwrap()).expect("reads as facts v1");
        assert_eq!(facts.source.brokkr_version, "0.15.2");
        assert_eq!(
            facts.source.binary_sha256,
            swarmail_evals::provenance::sha256(&std::fs::read(&fake).unwrap())
        );
        assert!(facts.source.query.starts_with(if live {
            "brokkr export --db forge.db --run smoke-4a42634a run-multi"
        } else {
            "saved export smoke-4a42634a sha256:"
        }));
        let value: Value = serde_json::from_slice(&std::fs::read(&out_path).unwrap()).unwrap();
        let trials = value["trials"].as_array().unwrap();
        let by_id = |n: u32| {
            trials
                .iter()
                .find(|t| t["trial_id"] == fixed_id(n))
                .cloned()
        };
        // 8 is owned by a review seat and 9 by no run: absent, not invented.
        assert_eq!(trials.len(), 7);
        assert!(by_id(8).is_none() && by_id(9).is_none());
        // The real smoke: the trial seat's session-final checkpoint.
        let smoke = by_id(1).unwrap();
        assert_eq!(smoke["run_id"], "smoke-4a42634a");
        assert_eq!(
            smoke["tokens"],
            json!({"status": "present", "value": {"input": 34366, "output": 202, "cached": 34362}})
        );
        assert_eq!(smoke["turns"], json!({"status": "present", "value": 2}));
        assert_eq!(
            smoke["deadline_killed"],
            json!({"status": "present", "value": false})
        );
        assert_eq!(
            (&smoke["wall_time_ms"], &smoke["tool_calls"]),
            (&none, &none)
        );
        // Only the owning attempt counts: not the failed retry, the per-turn
        // row, nor the interleaved review seat's bill.
        let retried = by_id(2).unwrap();
        assert_eq!(retried["run_id"], "run-multi");
        assert_eq!(
            retried["tokens"]["value"],
            json!({"input": 100, "output": 10, "cached": 50})
        );
        for fact in [
            "tokens",
            "turns",
            "wall_time_ms",
            "deadline_killed",
            "tool_calls",
        ] {
            assert_eq!(
                by_id(3).unwrap()[fact],
                json!({"status": "unavailable", "reason": "journal_ambiguous"}),
                "{fact}"
            );
        }
        // The nested drop: the outer exec seat carries no counts.
        let dropped = by_id(4).unwrap();
        assert_eq!((&dropped["tokens"], &dropped["turns"]), (&none, &none));
        // A repeated identical final counts once; a differing one is ambiguous.
        assert_eq!(
            by_id(5).unwrap()["tokens"]["value"],
            json!({"input": 40, "output": 4, "cached": 0})
        );
        assert_eq!(
            by_id(6).unwrap()["tokens"],
            json!({"status": "unavailable", "reason": "journal_ambiguous"})
        );
        assert_eq!(by_id(6).unwrap()["turns"]["value"], 3);
        // A final without the cache read cannot fill the v1 token triple.
        assert_eq!(by_id(7).unwrap()["tokens"], none);
        assert_eq!(by_id(7).unwrap()["turns"]["value"], 1);
    }
}

#[test]
fn journal_facts_script_refuses_without_writing_a_file() {
    let fx = fixture();
    let saved = saved_exports(&fx);
    let fake = fake_brokkr(&fx, "brokkr", "0.15.2", None);
    let fake_s = fake.to_str().unwrap();
    let out_path = fx.dir.path().join("out.json");
    let out_s = out_path.to_str().unwrap();
    let id = fixed_id(1);
    let db = fx.dir.path().join("forge.db");
    std::fs::write(&db, b"db").unwrap();
    let db_s = db.to_str().unwrap();
    let bad_dir = fx.dir.path().join("bad");
    std::fs::create_dir(&bad_dir).unwrap();
    for (name, body) in [
        ("not-json", "not json at all\n"),
        ("empty", ""),
        ("untyped", "{\"payload\": {}}\n"),
        (
            "no-seat",
            "{\"type\": \"effect/requested\", \"payload\": {\"effect_id\": \"e\"}}\n",
        ),
        (
            "no-attempt",
            "{\"type\": \"effect/checkpointed\", \"payload\": {\"effect_id\": \"e\", \"checkpoint\": {}}}\n",
        ),
        (
            "no-payload",
            "{\"type\": \"effect/succeeded\", \"attempt_id\": \"a\"}\n",
        ),
    ] {
        std::fs::write(bad_dir.join(format!("{name}.ndjson")), body).unwrap();
    }
    let bad = bad_dir.to_str().unwrap();
    let good = saved.to_str().unwrap();
    let versionless = fake_brokkr(&fx, "brokkr-no-version", "", None);
    let failing = fake_brokkr(&fx, "brokkr-export-fails", "0.15.2", Some("exit 7"));
    let silent = fake_brokkr(&fx, "brokkr-export-silent", "0.15.2", Some("exit 0"));
    let run = |run: &'static str| ["--run", run];
    type Case<'a> = (&'a str, Vec<&'a str>, Option<&'a str>, &'a str);
    let cases: Vec<Case> = vec![
        (
            "relative brokkr",
            [
                vec!["brokkr", db_s, out_s],
                run("run-multi").to_vec(),
                vec![&id],
            ]
            .concat(),
            Some(good),
            "absolute",
        ),
        (
            "relative output",
            [
                vec![fake_s, db_s, "out.json"],
                run("run-multi").to_vec(),
                vec![&id],
            ]
            .concat(),
            Some(good),
            "absolute",
        ),
        (
            "no run",
            vec![fake_s, db_s, out_s, &id, &id, &id],
            Some(good),
            "--run",
        ),
        (
            "no trial ids",
            [
                vec![fake_s, db_s, out_s],
                run("run-multi").to_vec(),
                run("smoke-4a42634a").to_vec(),
            ]
            .concat(),
            Some(good),
            "TRIAL_ID",
        ),
        (
            "path-like id",
            [
                vec![fake_s, db_s, out_s],
                run("run-multi").to_vec(),
                vec!["../x"],
            ]
            .concat(),
            Some(good),
            "plain",
        ),
        (
            "path-like run",
            [vec![fake_s, db_s, out_s], run("../run").to_vec(), vec![&id]].concat(),
            Some(good),
            "plain",
        ),
        (
            "missing saved run",
            [vec![fake_s, db_s, out_s], run("absent").to_vec(), vec![&id]].concat(),
            Some(good),
            "export_failed",
        ),
        (
            "export fails",
            [
                vec![failing.to_str().unwrap(), db_s, out_s],
                run("run-multi").to_vec(),
                vec![&id],
            ]
            .concat(),
            None,
            "export_failed",
        ),
        (
            "export writes nothing",
            [
                vec![silent.to_str().unwrap(), db_s, out_s],
                run("run-multi").to_vec(),
                vec![&id],
            ]
            .concat(),
            None,
            "export_failed",
        ),
        (
            "no version",
            [
                vec![versionless.to_str().unwrap(), db_s, out_s],
                run("run-multi").to_vec(),
                vec![&id],
            ]
            .concat(),
            Some(good),
            "export_version_unreadable",
        ),
        (
            "not json",
            [
                vec![fake_s, db_s, out_s],
                run("not-json").to_vec(),
                vec![&id],
            ]
            .concat(),
            Some(bad),
            "export_not_json",
        ),
    ];
    let mut cases = cases;
    for name in ["empty", "untyped", "no-seat", "no-attempt", "no-payload"] {
        cases.push((
            name,
            [vec![fake_s, db_s, out_s], run(name).to_vec(), vec![&id]].concat(),
            Some(bad),
            "export_shape_mismatch",
        ));
    }
    for (label, args, dir, want) in cases {
        let env: Vec<(&str, &str)> = dir
            .map(|d| vec![("JOURNAL_EXPORT_DIR", d)])
            .unwrap_or_default();
        let out = run_facts(&args, &env);
        assert_eq!(out.status.code(), Some(2), "{label}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(want), "{label}: {err}");
        assert!(!out_path.exists(), "{label}: wrote a file");
    }
    // An existing OUT is never overwritten.
    std::fs::write(&out_path, b"keep").unwrap();
    let out = run_facts(
        &[fake_s, db_s, out_s, "--run", "run-multi", &id],
        &[("JOURNAL_EXPORT_DIR", good)],
    );
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(std::fs::read(&out_path).unwrap(), b"keep");
}

#[test]
fn a_matching_export_with_no_run_for_a_trial_yields_journal_missing_by_absence() {
    use swarmail_evals::journal;
    let fx = fixture();
    let saved = saved_exports(&fx);
    let fake = fake_brokkr(&fx, "brokkr", "0.15.2", None);
    let out_path = fx.dir.path().join("out.json");
    let id = fixed_id(42);
    let args = [
        fake.to_str().unwrap(),
        "/db",
        out_path.to_str().unwrap(),
        "--run",
        "run-multi",
        id.as_str(),
    ];
    let out = run_facts(&args, &[("JOURNAL_EXPORT_DIR", saved.to_str().unwrap())]);
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

// ---- the matrix environment never crosses the seats (decision 0119, F1) ----

/// An executable shell script that records its own environment keys.
fn env_probe(dir: &std::path::Path, name: &str, record: &std::path::Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let probe = dir.join(name);
    std::fs::write(
        &probe,
        format!(
            "#!/bin/sh\nenv | sed 's/=.*//' | sort > {}\nexit 2\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o755)).unwrap();
    probe
}

#[test]
fn the_trial_script_strips_the_matrix_environment() {
    // Seam 1 of the four that sever any accounting channel (0119 section 6):
    // trial.sh runs the trial under `env -i`. The request's `evals_bin` is a
    // probe that records *its own* environment through the real script, so the
    // proof can go red: delete `env -i` from trial.sh and the sentinel, set in
    // the environment of the process that runs the script as the matrix's
    // would be, appears in the record. The follow-up that forwards the
    // observed channel flips this test to the observed name.
    let _trial_scope = trial_scope();
    let fx = fixture();
    let record = fx.dir.path().join("trial-env.txt");
    let probe = env_probe(fx.dir.path(), "evals-probe.sh", &record);
    let mut req = request(&fx, "seam-1", json!(["--task", "signup"]));
    req["evals_bin"] = json!(probe);
    let request_path = fx.slot.join("request.json");
    std::fs::write(&request_path, req.to_string()).unwrap();
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
        .expect("the probe ran under trial.sh and recorded its environment keys");
    assert!(
        keys.lines().any(|k| k == "SWARMAIL_BIN"),
        "the probe ran as trial.sh launches the trial: {keys}"
    );
    assert!(
        keys.lines().all(|k| !k.contains("SENTINEL")),
        "an inherited key survived env -i: {keys}"
    );
}

#[test]
fn the_runner_rebuilds_the_agents_environment() {
    // Seam 2: `swarmail-evals trial` builds the agent's environment itself.
    // The sentinel is in the trial's own environment, so only the runner's
    // rebuild keeps it from the probe agent. No claim is made for seams 3 and
    // 4 (`through_driver`): no production path crosses them while the driver
    // is refused, and their proof is the follow-up's.
    let _trial_scope = trial_scope();
    let dir = tempdir();
    let record = dir.path().join("agent-env.txt");
    let probe = env_probe(dir.path(), "agent-probe.sh", &record);
    let out = cli()
        .env("SWARMAIL_EVAL_SEAM_SENTINEL", "1")
        .args(["trial", "--task", "signup", "--agent", "probe", "--"])
        .arg(&probe)
        .output()
        .unwrap();
    let row = one_row(&out);
    assert_ne!(row.outcome, swarmail_evals::tasks::Outcome::HarnessError);
    let keys = std::fs::read_to_string(&record)
        .expect("the probe agent ran and recorded its environment keys");
    assert!(
        keys.lines().count() > 0 && keys.lines().all(|k| !k.contains("SENTINEL")),
        "the trial's own environment reached the agent: {keys}"
    );
}
