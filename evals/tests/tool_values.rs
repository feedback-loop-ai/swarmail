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

/// A sentinel file the capture admits: every sentinel is at least
/// `toolvalues::SENTINEL_MIN_BYTES` long.
const LONG_SENTINELS: &str = r#"["a sentinel long enough to register 0123456789"]"#;

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

/// The encoded spellings of each literal the canary search also looks for:
/// base64 (padded and not), base64url, hex, decimal and hex HTML entities,
/// quoted-printable, and hex joined by `:`, ` `, `/`, `,` and a newline.
/// Case-folded spellings are caught by the search itself, which ignores
/// ASCII case.
fn spellings_of(literals: &[&str]) -> Vec<String> {
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD};
    let mut out = Vec::new();
    for l in literals {
        out.push(STANDARD.encode(l));
        out.push(STANDARD_NO_PAD.encode(l));
        out.push(URL_SAFE_NO_PAD.encode(l));
        out.push(l.bytes().map(|b| format!("{b:02x}")).collect());
        out.push(l.bytes().map(|b| format!("&#{b};")).collect());
        out.push(l.bytes().map(|b| format!("&#x{b:x};")).collect());
        out.push(l.bytes().map(|b| format!("={b:02X}")).collect());
        for sep in [":", " ", "/", ",", "\n"] {
            out.push(
                l.bytes()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(sep),
            );
        }
    }
    out
}

/// Every written byte under `roots`, plus `streams`, holds none of
/// `canaries`, compared ignoring ASCII case.
fn assert_clean(roots: &[&Path], streams: &[&[u8]], canaries: &[&str]) -> usize {
    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            files.push(root.to_path_buf());
        } else {
            files_under(root, &mut files);
        }
    }
    let found = |bytes: &[u8], c: &str| {
        let (bytes, c) = (bytes.to_ascii_lowercase(), c.to_ascii_lowercase());
        bytes.windows(c.len()).any(|w| w == c.as_bytes())
    };
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
    // A numeric sentinel of the minimum length: 32 digits drawn from the
    // nonce, never starting with a zero.
    let number: String = std::iter::once('9')
        .chain(
            nonce[1..]
                .chars()
                .map(|c| char::from_digit(c.to_digit(16).unwrap() % 10, 10).unwrap()),
        )
        .collect();
    let sentinels = private.join("sentinels.json");
    std::fs::write(&sentinels, json!([sentinel, number]).to_string()).unwrap();
    let env = [
        ("EVAL_TEST_SENTINEL", sentinel.as_str()),
        ("EVAL_TEST_SENTINEL_NUMBER", number.as_str()),
    ];
    // The probe runs last and, mid-trial, rewrites the sentinel file to
    // hold only a late sentinel: the one registered at start must still be
    // redacted and the late one honoured.
    let late = format!("late-{}", swarmail_evals::runner::new_trial_id());
    let sentinels_text = sentinels.display().to_string();
    let mut probe_env = env.to_vec();
    probe_env.push(("EVAL_TEST_SENTINELS_FILE", sentinels_text.as_str()));
    probe_env.push(("EVAL_TEST_LATE_SENTINEL", late.as_str()));
    let probe = agent("probe", "tool_values", &private, &probe_env);
    let planted = agent("planted", "tool_values_planted", &private, &env);
    let encoded = agent("encoded", "tool_values_encoded", &private, &env);
    let joined = agent("joined", "tool_values_joined", &private, &env);
    let agents = private.join("agents.json");
    std::fs::write(
        &agents,
        json!([
            {"label": "oracle", "argv": []},
            {"label": "null", "argv": []},
            {"label": planted.label, "argv": [planted.wrapper]},
            {"label": encoded.label, "argv": [encoded.wrapper]},
            {"label": joined.label, "argv": [joined.wrapper]},
            {"label": probe.label, "argv": [probe.wrapper]},
        ])
        .to_string(),
    )
    .unwrap();
    let matrix = brokkr_matrix("ok", &realm)
        .args(["--tasks", "signup", "--agents"])
        .arg(&agents)
        .args(["--trials", "1", "--artifacts-dir"])
        .arg(&root)
        // Relative to the matrix's working directory (the realm): every
        // slot's request must carry it absolute, since a real trial seat
        // runs elsewhere.
        .args(["--capture-tool-values", "captures"])
        .arg("--tool-sentinels")
        .arg(&sentinels)
        .output()
        .unwrap();
    let real_captures = std::fs::canonicalize(&captures).unwrap();
    let mut requests = 0;
    for matrix_dir in std::fs::read_dir(&root).unwrap() {
        let slots = matrix_dir.unwrap().path().join("slots");
        for slot in std::fs::read_dir(&slots).into_iter().flatten() {
            let request: Value = serde_json::from_slice(
                &std::fs::read(slot.unwrap().path().join("request.json")).unwrap(),
            )
            .unwrap();
            let args = request["args"].as_array().unwrap();
            let at = args
                .iter()
                .position(|a| a == "--capture-tool-values")
                .expect("the capture flag");
            assert_eq!(args[at + 1], json!(real_captures.to_str().unwrap()));
            requests += 1;
        }
    }
    let rows = swarmail_evals::report::parse_rows(&String::from_utf8_lossy(&matrix.stdout))
        .unwrap_or_else(|e| panic!("{e}\n{}", stderr(&matrix)));
    assert_eq!(rows.len(), 6, "{}", stderr(&matrix));
    assert_eq!(requests, 6, "one request per brokkr slot");
    let by = |label: &str| {
        rows.iter()
            .find(|r| r.agent.as_deref() == Some(label))
            .unwrap()
    };
    // The controls keep their verdicts with capture on; the probe makes
    // calls and solves nothing.
    assert_eq!(by("oracle").outcome, Outcome::Pass);
    for label in ["null", "probe", "planted", "encoded", "joined"] {
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
    let (probe_bearer, planted_bearer, encoded_bearer, joined_bearer) = (
        bearer(&probe),
        bearer(&planted),
        bearer(&encoded),
        bearer(&joined),
    );
    assert_ne!(
        probe_bearer, planted_bearer,
        "each trial has its own bearer"
    );
    // Deny by default still yields evidence: the oracle's own synthetic
    // arguments (its real MCP calls) pass the plain-shape check whole.
    let oracle_capture =
        swarmail_evals::toolvalues::read_capture(&captures, &by("oracle").trial_id).unwrap();
    match &oracle_capture.outcome {
        swarmail_evals::toolvalues::CaptureOutcome::Complete { calls } => {
            assert!(!calls.is_empty(), "the oracle made calls");
            for c in calls {
                assert!(swarmail_evals::toolvalues::plain(&c.value), "{}", c.value);
            }
        }
        other => panic!("the oracle's arguments were withheld: {other:?}"),
    }
    let rows_path = out.join("rows.jsonl");
    std::fs::write(&rows_path, &matrix.stdout).unwrap();
    let facts = facts_file(
        &out,
        "facts.json",
        vec![
            facts_for(by("probe"), &calls(&probe)),
            facts_for(by("planted"), &calls(&planted)),
            facts_for(by("encoded"), &calls(&encoded)),
            facts_for(by("joined"), &calls(&joined)),
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
    assert_eq!(
        index[&id("encoded")],
        json!({"absent": "invalid_argument_encoding"})
    );
    assert_eq!(
        index[&id("joined")],
        json!({"absent": "invalid_argument_encoding"})
    );
    for label in ["planted", "encoded", "joined"] {
        assert!(!e2.join(id(label)).join("tool-values.json").exists());
    }

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
        "cafe ok [REDACTED] [REDACTED]"
    );
    assert_eq!(
        entries[1]["value"]["subject"], "\"quoted\" [REDACTED] [REDACTED]",
        "a removed sentinel and a late one are both redacted"
    );
    assert_eq!(
        std::fs::read_to_string(&sentinels).unwrap(),
        json!([late]).to_string(),
        "the probe did rewrite the file"
    );
    assert_eq!(
        entries[2]["value"]["path"],
        "/?t=[REDACTED]&e=[REDACTED]%zz&d=[REDACTED]"
    );
    assert_eq!(entries[3]["value"]["body"]["password"], "[REDACTED]");
    assert_eq!(entries[3]["value"]["body"]["note"], "ok!");
    assert_eq!(entries[3]["value"]["body"]["email"], "cafe@x");

    // Transcripts: failing rows per class, the probe's verified sequence.
    let tindex: Value =
        serde_json::from_slice(&std::fs::read(e2.join("transcripts/index.json")).unwrap()).unwrap();
    let classes = tindex["classes"].as_array().unwrap();
    let total: u64 = classes.iter().map(|c| c["total"].as_u64().unwrap()).sum();
    assert_eq!(
        total, 5,
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
        6
    );
    assert!(
        page_text.contains("| probe | 1 | 0 | 1 | 0 | null (no\\_passes) | 5 |"),
        "{page_text}"
    );

    // No canary in any written byte or stream, private intermediates
    // included; each literal's tail catches its encoded spellings too.
    let literals = [
        probe_bearer.as_str(),
        planted_bearer.as_str(),
        encoded_bearer.as_str(),
        joined_bearer.as_str(),
        sentinel.as_str(),
        number.as_str(),
        late.as_str(),
    ];
    let mut spellings = spellings_of(&literals);
    let (spaced, dotted, slashed) = chunked_spellings(&encoded_bearer, &sentinel);
    spellings.extend([spaced, dotted, slashed]);
    let mut canaries: Vec<&str> = literals.to_vec();
    canaries.extend([
        &probe_bearer[1..],
        &planted_bearer[1..],
        &encoded_bearer[1..],
        &joined_bearer[1..],
        &sentinel[1..],
    ]);
    canaries.extend(spellings.iter().map(String::as_str));
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
    assert_eq!(String::from_utf8_lossy(&replay.stdout).lines().count(), 6);

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
    // A group whose input plus output total overflows a u64 (1025 passes
    // at the per-fact cap, as the journal's own overflow test), with a
    // small cached count that does not.
    let overflow_template =
        synthetic_row("signup", "overflow", true, Outcome::Pass, 1000, &id(1000));
    let rows: Vec<TrialResult> = rows
        .into_iter()
        .chain((1000..2025).map(|n| {
            let mut r = overflow_template.clone();
            r.trial_id = id(n);
            r
        }))
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
    // The cached cell carries the same reason: without a journal nothing
    // was joined, so no cached count either.
    assert!(
        line(&page, "claude-a")
            .ends_with("| null (incomplete\\_join) | null (incomplete\\_join) |")
    );
    assert!(
        line(&page, "only-errors")
            .ends_with("| 0 | 0 | 1 | null (no\\_passes) | null (no\\_passes) |")
    );
    assert!(line(&page, "oracle").ends_with("| null (builtin\\_agent) | null (builtin\\_agent) |"));
    // The stdout report is the same bytes as without the page.
    assert_eq!(out.stdout, report(&rows_path, &[]).stdout);
    // With the golden journal: a number with its cached count, and reasons.
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/journal-facts-golden.json");
    let mut doc: Value = serde_json::from_slice(&std::fs::read(&golden).unwrap()).unwrap();
    let cap = 9_007_199_254_740_991u64;
    let trials = doc["trials"].as_array_mut().unwrap();
    for n in 1000..2025u32 {
        trials.push(json!({
            "trial_id": id(n),
            "run_id": format!("run-{n}"),
            "seat": "trial",
            "tokens": {"status": "present", "value": {"input": cap, "output": cap, "cached": 1}},
            "turns": {"status": "present", "value": 1},
            "wall_time_ms": {"status": "present", "value": 1000},
            "deadline_killed": {"status": "present", "value": false},
            "tool_calls": {"status": "present", "value": []},
        }));
    }
    let facts = dir.path().join("facts.json");
    std::fs::write(&facts, doc.to_string()).unwrap();
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
        line(&page, "codex-b").ends_with("| null (incomplete\\_join) | null (incomplete\\_join) |"),
        "{page}"
    );
    // Only harness errors: no token facts were joined, so the cached cell
    // carries the figure's own reason.
    assert!(
        line(&page, "only-errors").ends_with("| null (no\\_passes) | null (no\\_passes) |"),
        "{page}"
    );
    // The overflowed total keeps its separate, whole cached count.
    assert!(
        line(&page, "overflow").ends_with("| null (sum\\_overflow) | 1025 |"),
        "{page}"
    );
    assert!(
        line(&page, "only-fails").contains("null (incomplete\\_join)"),
        "{page}"
    );
    // A built-in group keeps its own reason with a journal joined too.
    assert!(
        line(&page, "oracle").ends_with("| null (builtin\\_agent) | null (builtin\\_agent) |"),
        "{page}"
    );
    for l in page.lines().filter(|l| l.starts_with("| signup |")) {
        assert!(!l.contains("| - |") && !l.contains("|  |"), "{l}");
    }
    // A page name that is not UTF-8 cannot be published: refused whole.
    use std::os::unix::ffi::OsStrExt;
    let odd = dir
        .path()
        .join(std::ffi::OsStr::from_bytes(b"page-\xff.md"));
    let out = report(&rows_path, &["--page".as_ref(), odd.as_os_str()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("page_write_failed"),
        "{}",
        stderr(&out)
    );
    assert!(out.stdout.is_empty());
    assert!(std::fs::symlink_metadata(&odd).is_err());
    // A transcript section that cannot be written fails the value report
    // by name: here one is already there.
    let evidence = dir.path().join("evidence");
    std::fs::create_dir(&evidence).unwrap();
    std::fs::create_dir(evidence.join("transcripts")).unwrap();
    let captures = dir.path().join("no-captures");
    std::fs::create_dir(&captures).unwrap();
    let out = report(
        &rows_path,
        &[
            "--journal".as_ref(),
            facts.as_os_str(),
            "--evidence-dir".as_ref(),
            evidence.as_os_str(),
            "--tool-values-from".as_ref(),
            captures.as_os_str(),
        ],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("harness error: transcripts_write_failed"),
        "{}",
        stderr(&out)
    );
    assert!(out.stdout.is_empty());
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
    std::fs::write(&sentinels, LONG_SENTINELS).unwrap();
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
    let orphan = dir.path().join("absent").join("fresh");
    for (out, category) in [
        (run(&existing, &sentinels, &[]), "capture_root_exists"),
        (run(&orphan, &sentinels, &[]), "capture_root_unavailable"),
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
    assert!(!orphan.exists());
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
    // A matrix refused by a later admission (here brokkr mode without an
    // artifacts root) leaves no capture root, so the same path is retried.
    let refused = run(&fresh, &sentinels, &["--brokkr", "/nonexistent/brokkr"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(
        stderr(&refused).contains("brokkr_requires_artifacts"),
        "{}",
        stderr(&refused)
    );
    assert!(!fresh.exists(), "a later refusal leaves no root behind");
    // Refusals after the root is made, by the matrix's own validation:
    // zero trials, and an agents file that does not parse. Each leaves no
    // root either.
    let bad_agents = dir.path().join("bad-agents.json");
    std::fs::write(&bad_agents, "not an agents file").unwrap();
    for (trials, agents_file) in [("0", agents.as_path()), ("1", bad_agents.as_path())] {
        let out = cli()
            .args([
                "matrix", "--tasks", "signup", "--trials", trials, "--agents",
            ])
            .arg(agents_file)
            .arg("--capture-tool-values")
            .arg(&fresh)
            .arg("--tool-sentinels")
            .arg(&sentinels)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
        assert!(out.stdout.is_empty());
        assert!(
            !fresh.exists(),
            "trials {trials}: a refused matrix leaves no root"
        );
    }
    let _trial_scope = trial_scope();
    let retried = run(&fresh, &sentinels, &[]);
    assert!(retried.status.success(), "{}", stderr(&retried));
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(&fresh).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
    let leaves: Vec<_> = std::fs::read_dir(&fresh).unwrap().collect();
    assert_eq!(leaves.len(), 1, "the oracle's capture record");
}

#[test]
fn a_capture_that_cannot_be_written_leaves_the_row_and_says_so() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    // An existing root that is not a directory: the trial runs, its
    // record cannot be written.
    let root = dir.path().join("not-a-directory");
    std::fs::write(&root, b"x").unwrap();
    let sentinels = dir.path().join("s.json");
    std::fs::write(&sentinels, LONG_SENTINELS).unwrap();
    let mut command = cli();
    command
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .arg("--capture-tool-values")
        .arg(&root)
        .arg("--tool-sentinels")
        .arg(&sentinels);
    let out = bounded(command, ISOLATED_CLI_BOUND).wait_with_output();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("swarmail-evals: harness error: tool_capture_write_failed"),
        "{}",
        stderr(&out)
    );
    assert_eq!(std::fs::read(&root).unwrap(), b"x");
}

#[test]
fn a_trial_reads_its_capture_paths_relative_to_its_working_directory() {
    let _trial_scope = trial_scope();
    let dir = tempdir();
    std::fs::create_dir(dir.path().join("captures")).unwrap();
    std::fs::write(dir.path().join("sentinels.json"), LONG_SENTINELS).unwrap();
    // Bare names: the sentinel file's parent is the empty path.
    let mut command = cli();
    command
        .current_dir(dir.path())
        .args(["trial", "--task", "signup", "--agent", "oracle"])
        .args(["--capture-tool-values", "captures"])
        .args(["--tool-sentinels", "sentinels.json"]);
    let out = bounded(command, ISOLATED_CLI_BOUND).wait_with_output();
    let row = one_row(&out);
    assert_eq!(row.outcome, Outcome::Pass, "{}", stderr(&out));
    let record =
        swarmail_evals::toolvalues::read_capture(&dir.path().join("captures"), &row.trial_id)
            .expect("the capture record");
    assert!(
        matches!(
            record.outcome,
            swarmail_evals::toolvalues::CaptureOutcome::Complete { .. }
        ),
        "the sentinel file was read: {:?}",
        record.outcome
    );
    assert_eq!(record.registry_literals, 2, "the bearer and the sentinel");
}

#[test]
fn a_page_cut_short_is_removed() {
    use std::os::unix::process::CommandExt;
    let dir = tempdir();
    let rows = dir.path().join("rows.jsonl");
    let row = synthetic_row(
        "signup",
        "claude-a",
        true,
        Outcome::Pass,
        1000,
        &"3".repeat(32),
    );
    std::fs::write(&rows, format!("{}\n", serde_json::to_string(&row).unwrap())).unwrap();
    let page = dir.path().join("report.md");
    let mut command = cli();
    command
        .args(["report", "--input"])
        .arg(&rows)
        .arg("--page")
        .arg(&page);
    // The child may write 64 bytes to any file: the page is created, cut
    // part-way, and its write fails (EFBIG, the signal ignored).
    unsafe {
        command.pre_exec(|| {
            if libc::signal(libc::SIGXFSZ, libc::SIG_IGN) == libc::SIG_ERR {
                return Err(std::io::Error::last_os_error());
            }
            let limit = libc::rlimit {
                rlim_cur: 64,
                rlim_max: 64,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let out = command.output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("page_write_failed"),
        "{}",
        stderr(&out)
    );
    assert!(out.stdout.is_empty());
    assert!(
        std::fs::symlink_metadata(&page).is_err(),
        "the partial page is removed"
    );
    // Nothing else is left either: the page was staged in a private
    // temporary, which is gone with the failed write.
    let names = |dir: &Path| -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    };
    assert_eq!(names(dir.path()), ["rows.jsonl"]);
    // Killed mid-write (the kernel's SIGXFSZ, left at its default), the
    // writer has no chance to clean up: the page's name still never holds
    // a partial page. Only the private temporary may remain.
    let mut killed = cli();
    killed
        .args(["report", "--input"])
        .arg(&rows)
        .arg("--page")
        .arg(&page);
    unsafe {
        killed.pre_exec(|| {
            if libc::signal(libc::SIGXFSZ, libc::SIG_DFL) == libc::SIG_ERR {
                return Err(std::io::Error::last_os_error());
            }
            let limit = libc::rlimit {
                rlim_cur: 64,
                rlim_max: 64,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let out = killed.output().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(out.status.signal(), Some(libc::SIGXFSZ), "{}", stderr(&out));
    assert!(
        std::fs::symlink_metadata(&page).is_err(),
        "a killed write leaves no page"
    );
    for name in names(dir.path()) {
        if name != "rows.jsonl" {
            std::fs::remove_file(dir.path().join(name)).unwrap();
        }
    }
    // The same command without the limit writes the page whole, private.
    let whole = report(&rows, &["--page".as_ref(), page.as_os_str()]);
    assert!(whole.status.success(), "{}", stderr(&whole));
    assert!(std::fs::metadata(&page).unwrap().len() > 64);
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&page).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(names(dir.path()), ["report.md", "rows.jsonl"]);
}
