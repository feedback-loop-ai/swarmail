//! The comparison's records and verifier (proposed decision 0129 §8–§9, spec
//! R2, R9): the product record, the inputs `matrix --target` binds before it
//! spawns anything, and `comparison-check`'s refusals, one per prerequisite
//! in the fixed order. Every set below is generated, bound by real digests
//! and a real git repository; nothing is a stub file.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use swarmail_evals::comparison::{self, CheckInputs, Mode};
use swarmail_evals::provenance::sha256;
use swarmail_evals::target;

const PRODUCT: &str = include_str!("../../docs/evals/agent-comparison.json");
const LOCK: &str = include_str!("../../docs/evals/sources.lock.json");

fn product_value() -> Value {
    serde_json::from_str(PRODUCT).unwrap()
}

#[test]
fn the_committed_product_validates_against_the_committed_lock() {
    let product = comparison::Product::parse(PRODUCT.as_bytes()).unwrap();
    let lock: Value = serde_json::from_str(LOCK).unwrap();
    product
        .validate(Some(&lock))
        .expect("the committed record is valid");
    assert_eq!(product.planned_cells, 240);
    assert_eq!(product.controls.cells, 120);
    let slurper = product.tuple(target::Target::Mailslurper).unwrap();
    // The source build, as accepted 0130's lock names it (its own row,
    // `mailslurper-source-build`, an `executable`), and the version it reports.
    assert_eq!(
        slurper.lock_entry.as_deref(),
        Some("mailslurper-source-build")
    );
    assert_eq!(slurper.artifact_kind.as_deref(), Some("executable"));
    assert_eq!(slurper.version_report.as_deref(), Some("v1.14.1"));
    assert_eq!(slurper.smtp_auth, "none");
    assert_eq!(slurper.delivery.accept_implies_queryable, "not-promised");
    assert_eq!(
        slurper.executable_sha256.as_deref(),
        Some("eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff")
    );
    assert!(slurper.recipient_filter.starts_with("upstream-substring"));
    // The header digests are the ones the prompt tests freeze.
    for t in target::ALL {
        assert_eq!(
            product.tuple(t).unwrap().prompt_header_sha256,
            comparison::header_sha256(t)
        );
    }
}

#[test]
fn a_product_that_miscounts_its_cells_or_disagrees_with_the_lock_or_the_code_is_refused() {
    let lock: Value = serde_json::from_str(LOCK).unwrap();
    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut v = product_value();
        edit(&mut v);
        let p = comparison::Product::parse(v.to_string().as_bytes()).unwrap();
        p.validate(Some(&lock)).expect_err("must refuse")
    };
    let why = refused(&|v| v["planned_cells"] = 241.into());
    assert!(
        why.contains("planned_cells 241 is not tasks × targets × agents × trials = 240"),
        "{why}"
    );
    let why = refused(&|v| v["trials"] = 4.into());
    assert!(why.contains("planned_cells 240 is not"), "{why}");
    let why = refused(&|v| v["controls"]["cells"] = 100.into());
    assert!(why.contains("controls are not"), "{why}");
    let why = refused(&|v| v["targets"][3]["artifact_kind"] = "archive".into());
    assert!(why.contains("mailslurper artifact_kind"), "{why}");
    let why = refused(&|v| v["targets"][3]["version_report"] = "v1.15.0".into());
    assert!(why.contains("mailslurper version_report"), "{why}");
    let why = refused(&|v| v["targets"][1]["executable_sha256"] = "0".repeat(64).into());
    assert!(why.contains("mailpit executable_sha256"), "{why}");
    let why = refused(&|v| v["targets"][2]["prompt_header_sha256"] = "1".repeat(64).into());
    assert!(why.contains("mailhog: prompt header digest"), "{why}");
    let why = refused(&|v| v["targets"][0]["tools_sha256"] = "nope".into());
    assert!(
        why.contains("swarmail: tools_sha256 is not a SHA-256"),
        "{why}"
    );
    let why = refused(&|v| v["targets"][3]["smtp_auth"] = "login".into());
    assert!(why.contains("smtp_auth is not plain or none"), "{why}");
    let why = refused(&|v| v["targets"].as_array_mut().unwrap().swap(1, 2));
    assert!(why.contains("targets"), "{why}");
    let why = refused(&|v| v["targets"][0]["executable_sha256"] = "2".repeat(64).into());
    assert!(why.contains("measured per run"), "{why}");
    let why = refused(&|v| v["tasks"] = json!(["otp-backlog"]));
    assert!(why.contains("hard tier"), "{why}");
    let why = refused(&|v| v["ruling"]["quote"] = "something else".into());
    assert!(why.contains("operator's"), "{why}");
}

#[test]
fn every_target_maps_to_one_lock_entry_and_a_tuple_bound_to_another_is_refused_by_name() {
    use target::Target;
    assert_eq!(Target::Swarmail.lock_entry(), None);
    assert_eq!(Target::Mailpit.lock_entry(), Some("mailpit"));
    assert_eq!(Target::Mailhog.lock_entry(), Some("mailhog"));
    // The agent `mailslurper` runs main's source-built row, never the
    // official `mailslurper` row, which does not start.
    assert_eq!(
        Target::Mailslurper.lock_entry(),
        Some("mailslurper-source-build")
    );
    let lock: Value = serde_json::from_str(LOCK).unwrap();
    // Every mapped entry exists in the committed lock.
    for t in target::ALL.into_iter().filter(|t| t.is_competitor()) {
        let row = t.lock_entry().unwrap();
        assert!(
            lock["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["target"] == row),
            "{row}"
        );
    }
    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut v = product_value();
        edit(&mut v);
        let p = comparison::Product::parse(v.to_string().as_bytes()).unwrap();
        p.validate(Some(&lock)).expect_err("must refuse")
    };
    // The official row, by name.
    let why = refused(&|v| v["targets"][3]["lock_entry"] = "mailslurper".into());
    assert!(
        why.contains("mailslurper: lock_entry Some(\"mailslurper\") is not the target's Some(\"mailslurper-source-build\")"),
        "{why}"
    );
    // Any other entry, by name: MailHog's tuple bound to Mailpit's row, and
    // the candidate bound to a row at all.
    let why = refused(&|v| v["targets"][2]["lock_entry"] = "mailpit".into());
    assert!(
        why.contains("mailhog: lock_entry Some(\"mailpit\") is not the target's Some(\"mailhog\")"),
        "{why}"
    );
    let why = refused(&|v| v["targets"][3]["lock_entry"] = serde_json::Value::Null);
    assert!(
        why.contains("mailslurper: lock_entry None is not the target's"),
        "{why}"
    );
    // The lock-side check refuses the same binding before it compares a digest.
    let mut v = product_value();
    v["targets"][3]["lock_entry"] = "mailslurper".into();
    let p = comparison::Product::parse(v.to_string().as_bytes()).unwrap();
    let why =
        comparison::tuple_agrees_with_lock(p.tuple(target::Target::Mailslurper).unwrap(), &lock)
            .unwrap_err();
    assert!(
        why.contains("product_lock_mismatch") && why.contains("lock_entry"),
        "{why}"
    );
}

// ---- inputs `matrix --target` binds ----------------------------------------

fn scratch() -> tempfile::TempDir {
    // Under the build directory on /home, never the RAM-backed /tmp.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/comparison-tests");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::Builder::new()
        .prefix("t-")
        .tempdir_in(root)
        .unwrap()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

fn executable(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = write(dir, name, bytes);
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

/// A product and lock whose competitor digests are the digests of files made
/// here, so a binary the product pins can exist without being an upstream's.
struct Synthetic {
    dir: tempfile::TempDir,
    product: PathBuf,
    sources: PathBuf,
    bins: [(&'static str, PathBuf); 3],
}

fn synthetic() -> Synthetic {
    let dir = scratch();
    let mut product = product_value();
    let mut lock: Value = serde_json::from_str(LOCK).unwrap();
    let mut bins = Vec::new();
    for (i, name, row) in [
        (1, "mailpit", "mailpit"),
        (2, "mailhog", "mailhog"),
        (3, "mailslurper", "mailslurper-source-build"),
    ] {
        let bin = executable(dir.path(), name, format!("a stand-in {name}").as_bytes());
        let digest = sha256(&std::fs::read(&bin).unwrap());
        product["targets"][i]["executable_sha256"] = digest.clone().into();
        for e in lock["entries"].as_array_mut().unwrap() {
            if e["target"] == row {
                e["pin"]["artifact"]["executable_sha256"] = digest.clone().into();
            }
        }
        bins.push((name, bin));
    }
    let product_path = write(dir.path(), "product.json", product.to_string().as_bytes());
    let sources = write(dir.path(), "sources.lock.json", lock.to_string().as_bytes());
    Synthetic {
        dir,
        product: product_path,
        sources,
        bins: bins.try_into().unwrap(),
    }
}

#[test]
fn every_matrix_input_refusal_names_itself_before_anything_is_spawned() {
    let s = synthetic();
    let adapter = executable(s.dir.path(), "adapter", b"#!/bin/sh\nexit 0\n");
    let bin = &s.bins[0].1;
    let ok = |t: &str| {
        comparison::bind_inputs(
            t,
            Some(&adapter),
            Some(bin),
            Some(&s.sources),
            Some(&s.product),
        )
    };
    let bound = ok("mailpit").expect("the synthetic inputs bind");
    assert_eq!(bound.competitor.as_ref().unwrap().target, "mailpit");
    // An unknown target.
    let why = comparison::bind_inputs("bogus", None, None, None, Some(&s.product)).unwrap_err();
    assert!(why.contains("unknown --target \"bogus\""), "{why}");
    // A bare backend name or a native one is not a target name either.
    assert!(comparison::bind_inputs("mailpit-mcp", None, None, None, Some(&s.product)).is_err());
    // A product is required, and a competitor needs the rest.
    let why = comparison::bind_inputs("swarmail", None, None, None, None).unwrap_err();
    assert!(why.starts_with("product_missing"), "{why}");
    let why = comparison::bind_inputs(
        "mailpit",
        None,
        Some(bin),
        Some(&s.sources),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(why.starts_with("competitor_inputs_missing"), "{why}");
    // Swarmail takes none of them.
    let why = comparison::bind_inputs("swarmail", Some(&adapter), None, None, Some(&s.product))
        .unwrap_err();
    assert!(why.starts_with("swarmail_takes_no_adapter"), "{why}");
    comparison::bind_inputs("swarmail", None, None, None, Some(&s.product))
        .expect("swarmail binds");
    // A FIFO and a directory are not regular files, and neither blocks.
    let fifo = s.dir.path().join("fifo");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&adapter),
        Some(bin),
        Some(&fifo),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(why.starts_with("sources_not_regular_file"), "{why}");
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&adapter),
        Some(bin),
        Some(s.dir.path()),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(why.starts_with("sources_not_regular_file"), "{why}");
    let why = comparison::bind_inputs("swarmail", None, None, None, Some(&fifo)).unwrap_err();
    assert!(why.starts_with("product_not_regular_file"), "{why}");
    // An over-bound product.
    let big = write(
        s.dir.path(),
        "big.json",
        &vec![b' '; comparison::READ_BOUND as usize + 1],
    );
    let why = comparison::bind_inputs("swarmail", None, None, None, Some(&big)).unwrap_err();
    assert!(why.starts_with("product_too_large"), "{why}");
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&adapter),
        Some(bin),
        Some(&big),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(why.starts_with("sources_too_large"), "{why}");
    // A wrong-digest --target-bin names both digests.
    let other = executable(s.dir.path(), "other", b"not the pinned bytes");
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&adapter),
        Some(&other),
        Some(&s.sources),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(why.starts_with("target_bin_digest_mismatch"), "{why}");
    assert!(why.contains(&sha256(b"not the pinned bytes")), "{why}");
    assert!(why.contains(&sha256(b"a stand-in mailpit")), "{why}");
    // A product that disagrees with the lock.
    let mut lock: Value =
        serde_json::from_str(&std::fs::read_to_string(&s.sources).unwrap()).unwrap();
    for e in lock["entries"].as_array_mut().unwrap() {
        if e["target"] == "mailpit" {
            e["pin"]["artifact"]["executable_sha256"] = "0".repeat(64).into();
        }
    }
    let skewed = write(
        s.dir.path(),
        "skewed.lock.json",
        lock.to_string().as_bytes(),
    );
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&adapter),
        Some(bin),
        Some(&skewed),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(
        why.starts_with("product_invalid: ") && why.contains("product_lock_mismatch"),
        "{why}"
    );
    // An adapter that is not an executable file, or a relative path.
    let plain = write(s.dir.path(), "plain", b"x");
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&plain),
        Some(bin),
        Some(&s.sources),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(why.starts_with("adapter_not_executable"), "{why}");
    let why = comparison::bind_inputs(
        "mailpit",
        Some(&adapter),
        Some(Path::new("relative")),
        Some(&s.sources),
        Some(&s.product),
    )
    .unwrap_err();
    assert!(
        why.starts_with("target_bin_unreadable") || why.starts_with("competitor_paths_relative"),
        "{why}"
    );
    // A task the product does not request, or one a competitor cannot take.
    let mut v = product_value();
    v["applicability"]["otp-backlog"]["mailhog"] = "not-requested".into();
    let p = comparison::Product::parse(v.to_string().as_bytes()).unwrap();
    let mut b = bound.clone();
    b.target = target::Target::Mailhog;
    b.product = p;
    let why = b.check_tasks(&["otp-backlog".into()]).unwrap_err();
    assert!(why.starts_with("task_not_applicable"), "{why}");
    let why = bound.check_tasks(&["signup".into()]).unwrap_err();
    assert!(
        why.starts_with("task_not_applicable") || why.contains("not requested"),
        "{why}"
    );
    assert!(
        bound
            .check_tasks(&["otp-backlog".into(), "digest-lookalike".into()])
            .is_ok()
    );
    assert!(bound.check_tasks(&["no-such-task".into()]).is_err());
}

// ---- a generated, genuine set of prerequisites ---------------------------------

struct Set {
    _s: Synthetic,
    ev: PathBuf,
    repo: PathBuf,
    inputs: CheckInputs,
    swarmail: PathBuf,
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.test",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn commit_file(repo: &Path, rel: &str, body: &str) -> String {
    let path = repo.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, body).unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", rel]);
    git(repo, &["rev-parse", "HEAD"])
}

fn put_json(path: &Path, v: &Value) {
    std::fs::write(path, serde_json::to_string_pretty(v).unwrap() + "\n").unwrap();
}

fn get_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// Re-bind a set after its controls changed: the receipts name the index.
fn rebind(set: &Set) {
    let index = std::fs::read(set.ev.join("controls/index.json")).unwrap();
    let mut r = get_json(&set.inputs.receipts);
    r["controls_index_sha256"] = sha256(&index).into();
    put_json(&set.inputs.receipts, &r);
}

fn control_row(
    product_sha: &str,
    t: &comparison::TargetTuple,
    fallback_exe: &str,
    (task, agent, attempt): (&str, &str, u32),
) -> swarmail_evals::identity::ComparisonRow {
    use swarmail_evals::identity::{ComparisonCoordinate, ComparisonRow, ComparisonSettings};
    let pass = agent == "oracle";
    ComparisonRow {
        schema_version: 3,
        kind: "comparison".into(),
        protocol_sha256: product_sha.into(),
        target: t.comparison_name.clone(),
        executable_sha256: t.executable_sha256.clone().unwrap_or(fallback_exe.into()),
        settings: ComparisonSettings {
            connection: t.connection.clone(),
            tools_sha256: t.tools_sha256.clone(),
            smtp_auth: t.smtp_auth.clone(),
        },
        coordinate: ComparisonCoordinate {
            task: task.into(),
            agent: agent.into(),
            attempt,
        },
        outcome: if pass {
            swarmail_evals::tasks::Outcome::Pass
        } else {
            swarmail_evals::tasks::Outcome::Fail
        },
        hash: String::new(),
    }
    .seal()
}

fn build_set() -> Set {
    let s = synthetic();
    let ev = s.dir.path().join("ev");
    std::fs::create_dir_all(ev.join("controls")).unwrap();
    std::fs::create_dir_all(ev.join("contracts")).unwrap();
    let repo = s.dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    let head = commit_file(&repo, "README.md", "the candidate");
    let product_bytes = std::fs::read(&s.product).unwrap();
    let product_sha = sha256(&product_bytes);
    let product = comparison::Product::parse(&product_bytes).unwrap();
    let swarmail = executable(s.dir.path(), "swarmail", b"a stand-in swarmail");
    let swarmail_sha = sha256(b"a stand-in swarmail");
    let mut rows = String::new();
    let mut index_rows = Vec::new();
    for t in &product.targets {
        let tools: Vec<String> = ["list", "read"]
            .iter()
            .map(|n| format!("{}_{n}", t.target))
            .collect();
        put_json(
            &ev.join(format!("contracts/{}.json", t.target)),
            &json!({"target": t.target, "tools_sha256": t.tools_sha256,
                "negotiation": {"protocol_version": "2025-03-26"},
                "batch": {"answered": true, "reply_is_array": true},
                "tool_names": tools, "cleanup_receipt": true,
                "calls": tools.iter().map(|n| json!({"tool": n, "outcome": "ok"})).collect::<Vec<_>>()}),
        );
        for task in &product.tasks {
            for agent in &product.controls.agents {
                for attempt in 1..=product.controls.trials {
                    let row = control_row(&product_sha, t, &swarmail_sha, (task, agent, attempt));
                    rows.push_str(&(serde_json::to_string(&row).unwrap() + "\n"));
                    index_rows.push(json!({"target": t.target, "task": task, "agent": agent, "attempt": attempt,
                        "outcome": row.outcome, "row_sha256": row.hash,
                        "failure_class": if agent == "oracle" { Value::Null } else { json!("gave_up") },
                        "fixture_sha256": sha256(format!("{task}/{agent}/{attempt}").as_bytes())}));
                }
            }
        }
    }
    std::fs::write(ev.join("controls/rows.jsonl"), &rows).unwrap();
    put_json(
        &ev.join("controls/index.json"),
        &json!({"schema": 1, "product_id": product.id, "product_sha256": product_sha,
            "master_seed": "0".repeat(64), "fixture_time": "2026-11-07T00:00:00Z",
            "rows_sha256": sha256(rows.as_bytes()), "cleanup_receipts_sha256": sha256(b"receipts"),
            "rows": index_rows}),
    );
    let index_sha = sha256(&std::fs::read(ev.join("controls/index.json")).unwrap());
    put_json(
        &ev.join("receipts.json"),
        &json!({"schema": 1, "head": head, "controls_index_sha256": index_sha,
            "cleanup_receipts_sha256": sha256(b"receipts"),
            "eval_suite": {"command": "cargo test --locked", "head": head, "status": "pass"},
            "ci": {"status": "pending"}}),
    );
    put_json(
        &ev.join("cost.json"),
        &json!({"basis": "unavailable", "reason": "no usage capture"}),
    );
    let mut entries = vec![json!({"role": "swarmail", "path": swarmail, "sha256": swarmail_sha})];
    for (name, bin) in &s.bins {
        entries.push(
            json!({"role": name, "path": bin, "sha256": sha256(&std::fs::read(bin).unwrap())}),
        );
    }
    for role in ["adapter", "harness"] {
        let p = executable(s.dir.path(), role, role.as_bytes());
        entries.push(json!({"role": role, "path": p, "sha256": sha256(role.as_bytes())}));
    }
    put_json(
        &ev.join("identity.json"),
        &json!({"schema": 1, "product_sha256": product_sha, "entries": entries}),
    );
    let baseline = write(
        &ev,
        "baseline.md",
        b"# Baseline\n\nThe #151 paid baseline.\n",
    );
    let inputs = CheckInputs {
        baseline,
        contracts: ev.join("contracts"),
        controls_dir: ev.join("controls"),
        receipts: ev.join("receipts.json"),
        cost: ev.join("cost.json"),
        identity: ev.join("identity.json"),
        product: s.product.clone(),
        sources: s.sources.clone(),
        repo: repo.clone(),
    };
    Set {
        _s: s,
        ev,
        repo,
        inputs,
        swarmail,
    }
}

fn why(set: &Set, mode: Mode) -> String {
    comparison::check(&set.inputs, mode).expect_err("must refuse")
}

#[test]
fn a_genuine_set_passes_and_only_a_pending_ci_item_stops_the_paid_start() {
    let set = build_set();
    let v = comparison::check(&set.inputs, Mode::DryRun).expect("a genuine set verifies");
    assert_eq!(v.controls_rows, 120);
    assert_eq!(v.pending.len(), 1, "{:?}", v.pending);
    let report = v.dry_run_report();
    assert!(
        report.contains("plan: 240 slots = 4 targets x 3 tasks x 4 agents x 5 trials"),
        "{report}"
    );
    assert!(
        report.contains("slots   1- 20: otp-backlog on swarmail"),
        "{report}"
    );
    // One matrix per target: a target's tasks are consecutive.
    assert!(
        report.contains("slots  21- 40: recipient-crowd on swarmail"),
        "{report}"
    );
    assert!(
        report.contains("slots  61- 80: otp-backlog on mailpit"),
        "{report}"
    );
    assert!(
        report.contains("slots 221-240: digest-lookalike on mailslurper"),
        "{report}"
    );
    assert!(report.contains("pending: ci:"), "{report}");
    assert!(
        report.contains("pending (operator): --authorized-product"),
        "{report}"
    );
    comparison::check(&set.inputs, Mode::Evidence).expect("evidence allows a pending run");
    let w = why(&set, Mode::Measure);
    assert!(
        w.starts_with("prerequisite-coordination-receipts: ci:"),
        "{w}"
    );
    let mut r = get_json(&set.inputs.receipts);
    r["ci"] = json!({"status": "run", "run": "https://example.test/actions/runs/1"});
    put_json(&set.inputs.receipts, &r);
    let v = comparison::check(&set.inputs, Mode::Measure).expect("a supplied run lifts it");
    assert!(v.pending.is_empty());
}

/// Rewrite the controls rows (and the index that binds them) with `f`.
fn rewrite_rows(
    set: &Set,
    f: &dyn Fn(Vec<&str>) -> Vec<String>,
    drop_index: &dyn Fn(&Value) -> bool,
) {
    let rows = set.ev.join("controls/rows.jsonl");
    let text = std::fs::read_to_string(&rows).unwrap();
    let body = f(text.lines().collect()).join("\n") + "\n";
    std::fs::write(&rows, &body).unwrap();
    let ip = set.ev.join("controls/index.json");
    let mut i = get_json(&ip);
    i["rows_sha256"] = sha256(body.as_bytes()).into();
    i["rows"].as_array_mut().unwrap().retain(|r| !drop_index(r));
    put_json(&ip, &i);
    rebind(set);
}

fn mutate_index(set: &Set, f: impl Fn(&mut Value)) {
    let ip = set.ev.join("controls/index.json");
    let mut i = get_json(&ip);
    f(&mut i);
    put_json(&ip, &i);
    rebind(set);
}

#[test]
fn each_prerequisite_refuses_by_name_when_missing_wrong_or_misbound() {
    type Break = Box<dyn Fn(&Set)>;
    let cases: Vec<(&str, Break, &str)> = vec![
        (
            "baseline",
            Box::new(|s| std::fs::remove_file(&s.inputs.baseline).unwrap()),
            "No such file",
        ),
        (
            "baseline",
            Box::new(|s| {
                write(&s.ev, "baseline.md", b"something else");
            }),
            "published #151 report",
        ),
        (
            "final-contracts",
            Box::new(|s| std::fs::remove_file(s.ev.join("contracts/mailhog.json")).unwrap()),
            "contract_unreadable",
        ),
        (
            "final-contracts",
            edit_json_at("contracts/mailpit.json", |c| {
                c["tools_sha256"] = "1".repeat(64).into()
            }),
            "mailpit: its tool list is not the product's",
        ),
        (
            "final-contracts",
            edit_json_at("contracts/swarmail.json", |c| {
                c["calls"].as_array_mut().unwrap().pop();
            }),
            "one outcome per announced tool",
        ),
        (
            "final-contracts",
            edit_json_at("contracts/mailslurper.json", |c| {
                c["batch"]["answered"] = false.into()
            }),
            "batch probe was not answered",
        ),
        (
            "controls",
            Box::new(|s| std::fs::remove_file(s.ev.join("controls/rows.jsonl")).unwrap()),
            "rows_unreadable",
        ),
        (
            "controls",
            Box::new(|s| {
                let p = s.ev.join("controls/rows.jsonl");
                let mut b = std::fs::read(&p).unwrap();
                b.pop();
                std::fs::write(&p, b).unwrap();
            }),
            "torn line",
        ),
        (
            "controls",
            Box::new(|s| {
                let first = std::fs::read_to_string(s.ev.join("controls/rows.jsonl"))
                    .unwrap()
                    .lines()
                    .next()
                    .unwrap()
                    .to_owned();
                let hash = first
                    .split("\"hash\":\"")
                    .nth(1)
                    .unwrap()
                    .trim_end_matches("\"}")
                    .to_owned();
                rewrite_rows(
                    s,
                    &|l| l.into_iter().skip(1).map(str::to_owned).collect(),
                    &|r| r["row_sha256"] == hash.as_str(),
                );
            }),
            "swarmail has 29 control rows, the product plans 30",
        ),
        (
            "controls",
            Box::new(|s| {
                rewrite_rows(
                    s,
                    &|l| {
                        l.into_iter()
                            .filter(|x| !x.contains("mailhog-mcp"))
                            .map(str::to_owned)
                            .collect()
                    },
                    &|r| r["target"] == "mailhog",
                )
            }),
            "the controls hold no rows for mailhog",
        ),
        (
            "controls",
            Box::new(|s| mutate_index(s, |i| i["product_sha256"] = "3".repeat(64).into())),
            "not bound to this product",
        ),
        (
            "controls",
            Box::new(|s| {
                mutate_index(s, |i| {
                    let row = i["rows"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|r| r["agent"] == "null")
                        .unwrap();
                    row["failure_class"] = "wrong_message".into();
                })
            }),
            "is not what the null control must give",
        ),
        (
            "controls",
            Box::new(|s| {
                mutate_index(s, |i| {
                    i["rows"][0]["fixture_sha256"] = "4".repeat(64).into()
                })
            }),
            "the fixture differs between targets",
        ),
        (
            "coordination-receipts",
            Box::new(|s| std::fs::remove_file(&s.inputs.receipts).unwrap()),
            "receipts_unreadable",
        ),
        (
            "coordination-receipts",
            Box::new(|s| {
                let mut r = get_json(&s.inputs.receipts);
                r["eval_suite"]["status"] = "fail".into();
                put_json(&s.inputs.receipts, &r);
            }),
            "locked eval suite did not pass",
        ),
        (
            "coordination-receipts",
            Box::new(|s| {
                let mut r = get_json(&s.inputs.receipts);
                r["controls_index_sha256"] = "5".repeat(64).into();
                put_json(&s.inputs.receipts, &r);
            }),
            "not bound to this controls index",
        ),
        (
            "cost-basis",
            Box::new(|s| {
                put_json(
                    &s.inputs.cost,
                    &json!({"basis": "measured", "total_usd": 0.0}),
                )
            }),
            "never 0",
        ),
        (
            "cost-basis",
            Box::new(|s| {
                put_json(
                    &s.inputs.cost,
                    &json!({"basis": "unavailable", "reason": " "}),
                )
            }),
            "says why",
        ),
        (
            "cost-basis",
            Box::new(|s| put_json(&s.inputs.cost, &json!({"basis": "free"}))),
            "neither unavailable nor measured",
        ),
        (
            "cost-basis",
            Box::new(|s| std::fs::remove_file(&s.inputs.cost).unwrap()),
            "cost_unreadable",
        ),
        (
            "identity",
            Box::new(|s| std::fs::write(&s.swarmail, b"another candidate").unwrap()),
            "now, and was",
        ),
        (
            "identity",
            Box::new(|s| {
                let mut i = get_json(&s.inputs.identity);
                i["entries"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|e| e["role"] != "adapter");
                put_json(&s.inputs.identity, &i);
            }),
            "exactly one adapter",
        ),
        (
            "identity",
            Box::new(|s| {
                let mut i = get_json(&s.inputs.identity);
                i["product_sha256"] = "6".repeat(64).into();
                put_json(&s.inputs.identity, &i);
            }),
            "not bound to this product",
        ),
        (
            "full-product",
            Box::new(|s| std::fs::remove_file(&s.inputs.product).unwrap()),
            "product_unreadable",
        ),
    ];
    for (name, breakage, words) in cases {
        let set = build_set();
        breakage(&set);
        let got = why(&set, Mode::DryRun);
        assert!(
            got.starts_with(&format!("prerequisite-{name}: ")) && got.contains(words),
            "{name} / {words}: {got}"
        );
    }
}

fn edit_json_at(rel: &'static str, f: impl Fn(&mut Value) + 'static) -> Box<dyn Fn(&Set)> {
    Box::new(move |s| {
        let path = s.ev.join(rel);
        let mut v = get_json(&path);
        f(&mut v);
        put_json(&path, &v);
    })
}

#[test]
fn the_first_failing_prerequisite_is_the_one_named() {
    let set = build_set();
    put_json(
        &set.inputs.cost,
        &json!({"basis": "measured", "total_usd": 0}),
    );
    std::fs::write(&set.swarmail, b"another candidate").unwrap();
    assert!(why(&set, Mode::DryRun).starts_with("prerequisite-cost-basis"));
    put_json(
        &set.inputs.cost,
        &json!({"basis": "unavailable", "reason": "r"}),
    );
    assert!(why(&set, Mode::DryRun).starts_with("prerequisite-identity"));
}

#[test]
fn receipts_are_refused_unless_their_head_is_an_ancestor_whose_diff_touches_only_evidence() {
    let set = build_set();
    commit_file(
        &set.repo,
        "docs/evals/agent-controls-2026-11-08/rows.jsonl",
        "evidence",
    );
    commit_file(&set.repo, "docs/evals/README.md", "the readme");
    commit_file(&set.repo, "specs/agent-suite-comparison/tasks.md", "tasks");
    comparison::check(&set.inputs, Mode::DryRun).expect("an evidence-only diff is accepted");
    for path in [
        "evals/src/runner.rs",
        "benches/mail_compare.rs",
        "scripts/compare-mail.sh",
        "src/main.rs",
        "docs/evals/sources.lock.json",
        "docs/evals/agent-comparison.json",
    ] {
        let set = build_set();
        commit_file(&set.repo, path, "a change");
        let got = why(&set, Mode::DryRun);
        assert!(
            got.starts_with("prerequisite-coordination-receipts: ")
                && got.contains(path)
                && got.contains("stale"),
            "{path}: {got}"
        );
    }
    let set = build_set();
    std::fs::write(set.repo.join("evals.rs"), "dirty").unwrap();
    assert!(why(&set, Mode::DryRun).contains("evals.rs changed since"));
    // An unstaged edit and a new file under the evidence paths are the
    // evidence being written, whatever git's leading space looks like.
    let set = build_set();
    commit_file(
        &set.repo,
        "docs/evals/agent-controls-2026-11-08/gates.txt",
        "first",
    );
    std::fs::write(
        set.repo
            .join("docs/evals/agent-controls-2026-11-08/gates.txt"),
        "edited",
    )
    .unwrap();
    std::fs::write(
        set.repo
            .join("docs/evals/agent-controls-2026-11-08/dry-run.txt"),
        "new",
    )
    .unwrap();
    comparison::check(&set.inputs, Mode::DryRun)
        .expect("unstaged and untracked evidence is allowed");
    // An unstaged edit elsewhere is named by its whole path.
    commit_file(&set.repo, "evals/src/lib.rs", "x");
    std::fs::write(set.repo.join("evals/src/lib.rs"), "y").unwrap();
    let got = why(&set, Mode::DryRun);
    assert!(got.contains(" evals/src/lib.rs changed since"), "{got}");
    let set = build_set();
    git(&set.repo, &["checkout", "-q", "--orphan", "other"]);
    commit_file(&set.repo, "other.txt", "elsewhere");
    let got = why(&set, Mode::DryRun);
    assert!(got.contains("is not an ancestor of the candidate"), "{got}");
}

#[test]
fn a_product_whose_tuple_disagrees_with_the_lock_is_refused_naming_the_lock_fact() {
    let lock: Value = serde_json::from_str(LOCK).unwrap();
    for (field, value, words) in [
        ("artifact_kind", "archive", "mailslurper artifact_kind"),
        ("version_report", "v1.15.0", "mailslurper version_report"),
    ] {
        let mut p = product_value();
        p["targets"][3][field] = value.into();
        let parsed = comparison::Product::parse(p.to_string().as_bytes()).unwrap();
        let e = parsed.validate(Some(&lock)).unwrap_err();
        assert!(
            e.contains("product_lock_mismatch") && e.contains(words),
            "{e}"
        );
    }
    // The wrapper's verifier reads the same refusal under the product's name.
    let set = build_set();
    let mut p = get_json(&set.inputs.product);
    p["targets"][3]["version_report"] = "v1.15.0".into();
    put_json(&set.inputs.product, &p);
    let got = why(&set, Mode::DryRun);
    assert!(got.starts_with("prerequisite-"), "{got}");
}
