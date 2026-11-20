//! Value-bearing tool evidence end to end (decision 0119 §4), offline: the
//! brokkr stand-in runs the recipe's real trial and score seats, the oracle
//! and null controls run beside a scripted external agent that leaks the
//! fixture's admin bearer and the run's sentinels through both
//! harness-owned tools, and the report joins the redacted captures to a
//! journal stand-in built from the agent's own call log. No real agent.

mod common;

use common::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use swarmail_evals::runner::TrialResult;
use swarmail_evals::tasks::Outcome;

#[test]
fn helper_agent() {
    maybe_run_helper();
}

fn files_under(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        if meta.is_dir() {
            files_under(&path, out);
        } else if meta.is_file() {
            out.push(path);
        }
    }
}

/// Every written byte under `roots`, plus `streams`, holds none of `canaries`.
fn assert_clean(roots: &[&Path], streams: &[&[u8]], canaries: &[&str]) -> usize {
    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            files.push(root.to_path_buf());
        } else {
            files_under(root, &mut files);
        }
    }
    let found = |bytes: &[u8], c: &str| bytes.windows(c.len()).any(|w| w == c.as_bytes());
    for file in &files {
        let bytes = std::fs::read(file).unwrap();
        for c in canaries {
            assert!(!found(&bytes, c), "a canary in {}", file.display());
        }
    }
    for (i, s) in streams.iter().enumerate() {
        for c in canaries {
            assert!(!found(s, c), "a canary in stream {i}");
        }
    }
    files.len()
}

struct Agent {
    label: &'static str,
    marker: PathBuf,
    _wrapper_dir: tempfile::TempDir,
    wrapper: PathBuf,
}

fn agent(label: &'static str, mode: &str, private: &Path, env: &[(&str, &str)]) -> Agent {
    let marker = private.join(label);
    let marker_text = marker.display().to_string();
    let mut all: Vec<(&str, &str)> = env.to_vec();
    all.push((MARKER_ENV, &marker_text));
    let (_cmd, dir) = helper_command(mode, &[], &all);
    let wrapper = dir.path().join("agent-wrapper.sh");
    Agent {
        label,
        marker,
        _wrapper_dir: dir,
        wrapper,
    }
}

fn facts_for(row: &TrialResult, calls: &Value) -> Value {
    let tool_calls: Vec<Value> = calls
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({
                "ordinal": i + 1,
                "tool": c["tool"],
                "outcome": "ok",
                "argument_bytes": c["bytes"],
                "argument_sha256": c["sha256"],
                "result_bytes": 2,
                "result_sha256": swarmail_evals::provenance::sha256(b"{}"),
            })
        })
        .collect();
    json!({
        "trial_id": row.trial_id,
        "run_id": format!("run-{}", &row.trial_id[..8]),
        "seat": "trial",
        "tokens": {"status": "present", "value": {"input": 100, "output": 20, "cached": 5}},
        "turns": {"status": "present", "value": 3},
        "wall_time_ms": {"status": "present", "value": 1000},
        "deadline_killed": {"status": "present", "value": false},
        "tool_calls": {"status": "present", "value": tool_calls},
    })
}

fn facts_file(dir: &Path, name: &str, trials: Vec<Value>) -> PathBuf {
    let path = dir.join(name);
    let doc = json!({
        "schema_version": 1,
        "source": {
            "brokkr_version": "0.12.0",
            "binary_sha256": swarmail_evals::provenance::sha256(b"stand-in"),
            "query": "test journal stand-in built from the probe agent's own call log",
        },
        "trials": trials,
    });
    std::fs::write(&path, doc.to_string()).unwrap();
    path
}

fn report(rows: &Path, extra: &[&std::ffi::OsStr]) -> std::process::Output {
    cli()
        .args(["report", "--input"])
        .arg(rows)
        .args(extra)
        .output()
        .unwrap()
}

/// The report's value-mode additions removed: what is left must equal the
/// default report's bytes, so the mode can change no figure.
fn without_values(mut v: Value) -> Value {
    for g in v["groups"].as_array_mut().unwrap() {
        g["journal"].as_object_mut().unwrap().remove("tool_values");
    }
    v
}

fn value_index(report: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for g in report["groups"].as_array().unwrap() {
        for (id, entry) in g["journal"]["tool_values"].as_object().unwrap() {
            out.insert(id.clone(), entry.clone());
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn values_are_redacted_bound_and_never_scored() {
    let _trial_scope = trial_scope();
    let started = std::time::Instant::now();
    let dir = tempdir();
    let private = dir.path().join("agent-private");
    let out = dir.path().join("out");
    std::fs::create_dir(&private).unwrap();
    std::fs::create_dir(&out).unwrap();
    let realm = brokkr_realm(dir.path());
    let root = realm.join("artifacts");
    let captures = realm.join("captures");
    let nonce = swarmail_evals::runner::new_trial_id();
    let sentinel = format!("sentinel-{nonce}");
    let number = (u64::from_str_radix(&nonce[..12], 16).unwrap() % 900_000_000_000
        + 100_000_000_000)
        .to_string();
    let sentinels = private.join("sentinels.json");
    std::fs::write(&sentinels, json!([sentinel, number]).to_string()).unwrap();
    let env = [
        ("EVAL_TEST_SENTINEL", sentinel.as_str()),
        ("EVAL_TEST_SENTINEL_NUMBER", number.as_str()),
    ];
    let probe = agent("probe", "tool_values", &private, &env);
    let planted = agent("planted", "tool_values_planted", &private, &env);
    let agents = private.join("agents.json");
    std::fs::write(
        &agents,
        json!([
            {"label": "oracle", "argv": []},
            {"label": "null", "argv": []},
            {"label": probe.label, "argv": [probe.wrapper]},
            {"label": planted.label, "argv": [planted.wrapper]},
        ])
        .to_string(),
    )
    .unwrap();
    let matrix = brokkr_matrix("ok", &realm)
        .args(["--tasks", "signup", "--agents"])
        .arg(&agents)
        .args(["--trials", "1", "--artifacts-dir"])
        .arg(&root)
        .arg("--capture-tool-values")
        .arg(&captures)
        .arg("--tool-sentinels")
        .arg(&sentinels)
        .output()
        .unwrap();
    let rows = swarmail_evals::report::parse_rows(&String::from_utf8_lossy(&matrix.stdout))
        .unwrap_or_else(|e| panic!("{e}\n{}", stderr(&matrix)));
    assert_eq!(rows.len(), 4, "{}", stderr(&matrix));
    let by = |label: &str| {
        rows.iter()
            .find(|r| r.agent.as_deref() == Some(label))
            .unwrap()
    };
    // The controls keep their verdicts with capture on; the probe makes
    // calls and solves nothing.
    assert_eq!(by("oracle").outcome, Outcome::Pass);
    for label in ["null", "probe", "planted"] {
        assert_eq!(
            by(label).outcome,
            Outcome::Fail,
            "{label}: {}",
            stderr(&matrix)
        );
    }
    let calls = |a: &Agent| -> Value {
        let failure = std::fs::read_to_string(a.marker.with_extension("failure"));
        assert!(failure.is_err(), "{}: {failure:?}", a.label);
        serde_json::from_slice(&std::fs::read(a.marker.with_extension("calls")).unwrap()).unwrap()
    };
    let bearer = |a: &Agent| std::fs::read_to_string(a.marker.with_extension("bearer")).unwrap();
    let (probe_bearer, planted_bearer) = (bearer(&probe), bearer(&planted));
    assert_ne!(
        probe_bearer, planted_bearer,
        "each trial has its own bearer"
    );
    let rows_path = out.join("rows.jsonl");
    std::fs::write(&rows_path, &matrix.stdout).unwrap();
    let facts = facts_file(
        &out,
        "facts.json",
        vec![
            facts_for(by("probe"), &calls(&probe)),
            facts_for(by("planted"), &calls(&planted)),
        ],
    );

    // The default report and the value-bearing one over the same inputs.
    let e1 = out.join("e1");
    let e2 = out.join("e2");
    let page = out.join("report.md");
    let plain = report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts.as_os_str(),
            "--evidence-dir".as_ref(),
            e1.as_os_str(),
        ],
    );
    let valued = report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts.as_os_str(),
            "--evidence-dir".as_ref(),
            e2.as_os_str(),
            "--tool-values-from".as_ref(),
            captures.as_os_str(),
            "--page".as_ref(),
            page.as_os_str(),
        ],
    );
    assert!(plain.status.success(), "{}", stderr(&plain));
    assert!(valued.status.success(), "{}", stderr(&valued));
    let plain_json: Value = serde_json::from_slice(&plain.stdout).unwrap();
    let valued_json: Value = serde_json::from_slice(&valued.stdout).unwrap();
    assert_eq!(
        without_values(valued_json.clone()),
        plain_json,
        "the mode changes no figure"
    );
    let index = value_index(&valued_json);
    let id = |label: &str| by(label).trial_id.clone();
    assert_eq!(index[&id("oracle")], json!({"absent": "builtin_agent"}));
    assert_eq!(index[&id("null")], json!({"absent": "builtin_agent"}));
    assert_eq!(index[&id("planted")], json!({"absent": "residual_secret"}));
    assert!(!e2.join(id("planted")).join("tool-values.json").exists());

    // The probe's companion: whole, ordered, redacted and bound.
    let leaf = e2.join(id("probe"));
    let bytes = std::fs::read(leaf.join("tool-values.json")).unwrap();
    assert_eq!(
        index[&id("probe")]["sha256"],
        swarmail_evals::provenance::sha256(&bytes)
    );
    assert_eq!(index[&id("probe")]["length"], bytes.len());
    let companion: Value = serde_json::from_slice(&bytes).unwrap();
    let structural = std::fs::read(leaf.join("tool-calls.json")).unwrap();
    assert_eq!(
        companion["structural"]["sha256"],
        swarmail_evals::provenance::sha256(&structural)
    );
    assert_eq!(companion["model_text"], "not_retained");
    let entries = companion["tool_calls"].as_array().unwrap();
    let logged = calls(&probe);
    assert_eq!(entries.len(), 4);
    for (e, c) in entries.iter().zip(logged.as_array().unwrap()) {
        assert_eq!(e["argument_sha256"], c["sha256"]);
        assert_eq!(e["argument_bytes"], c["bytes"]);
        assert_eq!(e["owned"], true);
    }
    assert_eq!(
        entries[0]["value"]["subject"],
        "caf\u{e9} \u{2713} [REDACTED] [REDACTED]"
    );
    assert_eq!(entries[1]["value"]["subject"], "\"quoted\" [REDACTED]");
    assert_eq!(
        entries[2]["value"]["path"],
        "/?t=[REDACTED]&e=[REDACTED]%zz&d=[REDACTED]"
    );
    assert_eq!(entries[3]["value"]["body"]["password"], "[REDACTED]");
    assert_eq!(entries[3]["value"]["body"]["note"], "\u{2713}\t");

    // Transcripts: failing rows per class, the probe's verified sequence.
    let tindex: Value =
        serde_json::from_slice(&std::fs::read(e2.join("transcripts/index.json")).unwrap()).unwrap();
    let classes = tindex["classes"].as_array().unwrap();
    let total: u64 = classes.iter().map(|c| c["total"].as_u64().unwrap()).sum();
    assert_eq!(
        total, 3,
        "every failing row is counted before the evidence filter"
    );
    let picked: Vec<&Value> = classes
        .iter()
        .flat_map(|c| c["transcripts"].as_array().unwrap())
        .collect();
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0]["trial_id"], id("probe"));
    let transcript: Value = serde_json::from_slice(
        &std::fs::read(e2.join(picked[0]["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(
        transcript["row"],
        serde_json::to_value(by("probe")).unwrap()
    );
    assert_eq!(transcript["tool_calls"], companion["tool_calls"]);

    // The page names a figure or a reason for every group.
    let page_text = std::fs::read_to_string(&page).unwrap();
    assert_eq!(
        page_text
            .lines()
            .filter(|l| l.starts_with("| signup |"))
            .count(),
        4
    );
    assert!(
        page_text.contains("| probe | 1 | 0 | 1 | 0 | null (no\\_passes) | 5 |"),
        "{page_text}"
    );

    // No canary in any written byte or stream, private intermediates
    // included; each literal's tail catches its encoded spellings too.
    let canaries = [
        probe_bearer.as_str(),
        planted_bearer.as_str(),
        sentinel.as_str(),
        number.as_str(),
        &probe_bearer[1..],
        &planted_bearer[1..],
        &sentinel[1..],
    ];
    let searched = assert_clean(
        &[&realm, &out],
        &[
            &matrix.stdout,
            &matrix.stderr,
            &plain.stdout,
            &plain.stderr,
            &valued.stdout,
            &valued.stderr,
        ],
        &canaries,
    );
    assert!(
        searched > 20,
        "the search saw the artifacts ({searched} files)"
    );

    // Scoring is independent: the retained matrix replays as before.
    let matrix_dir = std::fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let replay = cli()
        .args(["check", "--matrix"])
        .arg(&matrix_dir)
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(1), "{}", stderr(&replay));
    assert_eq!(String::from_utf8_lossy(&replay.stdout).lines().count(), 4);

    // Corrupt, missing and mismatched inputs change only the value index.
    let rerun = |name: &str, facts: &Path| {
        let ev = out.join(name);
        let o = report(
            &rows_path,
            &[
                "--journal".as_ref(),
                facts.as_os_str(),
                "--evidence-dir".as_ref(),
                ev.as_os_str(),
                "--tool-values-from".as_ref(),
                captures.as_os_str(),
            ],
        );
        assert!(o.status.success(), "{name}: {}", stderr(&o));
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        (value_index(&v), without_values(v))
    };
    let mut wrong = calls(&probe);
    wrong[1]["sha256"] = json!(swarmail_evals::provenance::sha256(b"other"));
    let mismatched = facts_file(&out, "mismatch.json", vec![facts_for(by("probe"), &wrong)]);
    let (idx, _) = rerun("e3", &mismatched);
    assert_eq!(idx[&id("probe")], json!({"absent": "binding_mismatch"}));
    assert_eq!(idx[&id("planted")], json!({"absent": "journal_missing"}));
    let mut unaccounted = facts_for(by("probe"), &calls(&probe));
    unaccounted["tool_calls"] =
        json!({"status": "unavailable", "reason": "journal_has_no_accounting"});
    let mut ambiguous = facts_for(by("planted"), &calls(&planted));
    ambiguous["tool_calls"] = json!({"status": "unavailable", "reason": "journal_ambiguous"});
    let upstream = facts_file(&out, "upstream.json", vec![unaccounted, ambiguous]);
    let (idx, _) = rerun("e4", &upstream);
    assert_eq!(
        idx[&id("probe")],
        json!({"absent": "journal_has_no_accounting"})
    );
    assert_eq!(idx[&id("planted")], json!({"absent": "journal_ambiguous"}));
    // A substituted capture byte, then a missing capture.
    let capture = captures.join(id("probe")).join("capture.json");
    let mut cbytes = std::fs::read(&capture).unwrap();
    let at = cbytes.len() / 2;
    cbytes[at] ^= 1;
    std::fs::remove_file(&capture).unwrap();
    std::fs::write(&capture, &cbytes).unwrap();
    let (idx, scores) = rerun("e5", &facts);
    assert_eq!(idx[&id("probe")], json!({"absent": "capture_missing"}));
    assert_eq!(scores, plain_json);
    std::fs::remove_dir_all(captures.join(id("probe"))).unwrap();
    let (idx, scores) = rerun("e6", &facts);
    assert_eq!(idx[&id("probe")], json!({"absent": "capture_missing"}));
    assert_eq!(scores, plain_json);
    assert_clean(&[&out], &[], &canaries);
    eprintln!(
        "tool-values proof: {:.1} s",
        started.elapsed().as_secs_f64()
    );
}

#[test]
fn the_page_gives_every_paid_group_a_figure_or_its_reason() {
    let dir = tempdir();
    let id = |n: u32| format!("{n:032x}");
    let rows: Vec<TrialResult> = [
        ("claude-a", true, Outcome::Pass, 3),
        ("claude-a", true, Outcome::Pass, 4),
        ("claude-a", true, Outcome::Fail, 5),
        ("codex-b", true, Outcome::Pass, 6),
        ("codex-b", true, Outcome::Fail, 7),
        ("oracle", false, Outcome::Pass, 1),
        ("only-errors", true, Outcome::HarnessError, 20),
        ("only-fails", true, Outcome::Fail, 21),
    ]
    .into_iter()
    .map(|(agent, external, outcome, n)| {
        synthetic_row("signup", agent, external, outcome, 1000, &id(n))
    })
    .chain([synthetic_row(
        "otp",
        "codex-b",
        true,
        Outcome::Pass,
        1000,
        &id(8),
    )])
    .collect();
    let rows_path = dir.path().join("rows.jsonl");
    let text: String = rows
        .iter()
        .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
        .collect();
    std::fs::write(&rows_path, text).unwrap();
    let line = |page: &str, agent: &str| -> String {
        page.lines()
            .find(|l| l.starts_with(&format!("| signup | {} |", agent.replace('-', "\\-"))))
            .unwrap_or_else(|| panic!("{agent}\n{page}"))
            .to_string()
    };
    // Without a journal: no fact was joined, so every paid figure is null
    // with incomplete_join, an empty scored population is no_passes.
    let bare = dir.path().join("bare.md");
    let out = report(&rows_path, &["--page".as_ref(), bare.as_os_str()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let page = std::fs::read_to_string(&bare).unwrap();
    assert!(line(&page, "claude-a").ends_with("| null (incomplete\\_join) | null |"));
    assert!(line(&page, "only-errors").ends_with("| 0 | 0 | 1 | null (no\\_passes) | null |"));
    assert!(line(&page, "oracle").ends_with("| null (builtin\\_agent) | null |"));
    // The stdout report is the same bytes as without the page.
    assert_eq!(out.stdout, report(&rows_path, &[]).stdout);
    // With the golden journal: a number with its cached count, and reasons.
    let facts = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/journal-facts-golden.json");
    let joined = dir.path().join("joined.md");
    let out = report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts.as_os_str(),
            "--page".as_ref(),
            joined.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let page = std::fs::read_to_string(&joined).unwrap();
    assert!(
        line(&page, "claude-a").ends_with("| 3600 | 12000 |"),
        "{page}"
    );
    assert!(
        line(&page, "codex-b").ends_with("| null (incomplete\\_join) | 0 |"),
        "{page}"
    );
    assert!(
        line(&page, "only-errors").contains("null (no\\_passes)"),
        "{page}"
    );
    assert!(
        line(&page, "only-fails").contains("null (incomplete\\_join)"),
        "{page}"
    );
    for l in page.lines().filter(|l| l.starts_with("| signup |")) {
        assert!(!l.contains("| - |") && !l.contains("|  |"), "{l}");
    }
    // A page is never written over an existing file.
    let again = report(&rows_path, &["--page".as_ref(), joined.as_os_str()]);
    assert_eq!(again.status.code(), Some(2));
    assert!(stderr(&again).contains("page_write_failed"));
    assert!(again.stdout.is_empty());
    // The value mode needs its journal and its evidence directory.
    let refused = report(
        &rows_path,
        &["--tool-values-from".as_ref(), dir.path().as_os_str()],
    );
    assert_eq!(refused.status.code(), Some(2));
}

#[test]
fn matrix_capture_refuses_before_any_trial() {
    let dir = tempdir();
    let agents = dir.path().join("agents.json");
    std::fs::write(&agents, r#"[{"label":"oracle","argv":[]}]"#).unwrap();
    let sentinels = dir.path().join("s.json");
    std::fs::write(&sentinels, r#"["canary"]"#).unwrap();
    let existing = dir.path().join("exists");
    std::fs::create_dir(&existing).unwrap();
    let run = |root: &Path, sentinels: &Path, extra: &[&str]| {
        cli()
            .args(["matrix", "--tasks", "signup", "--trials", "1", "--agents"])
            .arg(&agents)
            .arg("--capture-tool-values")
            .arg(root)
            .arg("--tool-sentinels")
            .arg(sentinels)
            .args(extra)
            .output()
            .unwrap()
    };
    let fresh = dir.path().join("fresh");
    for (out, category) in [
        (run(&existing, &sentinels, &[]), "capture_root_exists"),
        (
            run(&fresh, &dir.path().join("absent.json"), &[]),
            "tool_sentinels_unreadable",
        ),
        (run(&fresh, &agents, &[]), "tool_sentinels_unreadable"),
    ] {
        assert_eq!(out.status.code(), Some(2), "{category}");
        assert!(out.stdout.is_empty());
        assert!(stderr(&out).contains(category), "{}", stderr(&out));
    }
    assert!(!fresh.exists(), "a refusal creates no root");
    // The capture flags come as a pair.
    let lone = cli()
        .args(["matrix", "--tasks", "signup", "--trials", "1", "--agents"])
        .arg(&agents)
        .arg("--capture-tool-values")
        .arg(&fresh)
        .output()
        .unwrap();
    assert_eq!(lone.status.code(), Some(2));
    assert!(!fresh.exists());
}
