//! The fair wire comparison driver (issue #163, proposed decision 0091),
//! proved against the real Swarmail binary it launches and owns: validator
//! refusals, independent arithmetic, real throughput and wait runs,
//! lifecycle and privacy facts read from the kernel, sealing and admission.
//! Mailpit and MailHog are not exercised here: their official pins are not
//! captured (docs/evals/sources.lock.json), and the driver refuses to launch
//! an unverified target — that refusal is what these tests prove for them.

// The driver's own modules, compiled into this test crate. Some items serve
// only the driver binary (its CLI), so dead-code analysis here is partial.
#[allow(dead_code)]
#[path = "../benches/mail_compare/protocol.rs"]
mod protocol;
#[allow(dead_code)]
#[path = "../benches/mail_compare/records.rs"]
mod records;
#[allow(dead_code)]
#[path = "../benches/mail_compare/report.rs"]
mod report;
#[allow(dead_code)]
#[path = "../benches/mail_compare/runner.rs"]
mod runner;
#[allow(dead_code)]
#[path = "../benches/mail_compare/stats.rs"]
mod stats;
#[allow(dead_code)]
#[path = "../benches/mail_compare/targets.rs"]
mod targets;
#[allow(dead_code)]
#[path = "../benches/mail_compare/wire.rs"]
mod wire;

use protocol::{Protocol, SourceLock, TargetKind};
use records::{Manifest, Phase, Row};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SMOKE: &[u8] = include_bytes!("mail_compare/swarmail-smoke.json");
const SMOKE_LOCK: &[u8] = include_bytes!("mail_compare/swarmail-only.lock.json");
const PROTOCOL: &[u8] = include_bytes!("../docs/evals/protocol.json");
const SOURCES: &[u8] = include_bytes!("../docs/evals/sources.lock.json");

fn smoke() -> Protocol {
    Protocol::parse(SMOKE).expect("the smoke fixture parses")
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).expect("fixture json")
}

fn protocol_from(v: serde_json::Value) -> Result<Protocol, protocol::Refusal> {
    Protocol::parse(&serde_json::to_vec(&v).unwrap())
}

fn lock_from(v: serde_json::Value) -> Result<SourceLock, protocol::Refusal> {
    SourceLock::parse(&serde_json::to_vec(&v).unwrap())
}

fn refusals(r: Result<(), protocol::Refusal>) -> String {
    r.expect_err("must refuse").0.join("\n")
}

fn limits() -> targets::Limits {
    runner::limits(&smoke())
}

fn swarmail_spec(template: Vec<String>) -> targets::LaunchSpec {
    targets::LaunchSpec {
        kind: TargetKind::Swarmail,
        binary: PathBuf::from(env!("CARGO_BIN_EXE_swarmail")),
        expected_sha256: None,
        template,
    }
}

fn manifest(p: &Protocol, run_id: &str) -> Manifest {
    Manifest {
        manifest_version: records::MANIFEST_VERSION,
        run_id: run_id.to_owned(),
        mode: "smoke".into(),
        protocol_id: p.id.clone(),
        protocol_sha256: targets::sha256_hex(&serde_json::to_vec(p).unwrap()),
        sources_sha256: targets::sha256_hex(SMOKE_LOCK),
        candidate_commit: None,
        candidate_dirty: None,
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        kernel: "test".into(),
        cpus: 1,
        prelaunch_utc: "2026-10-28T00:00:00Z".into(),
        planned: runner::plan(p),
    }
}

/// Run `p` against the real binary into `root/out`; scratch is `root/s`.
async fn run(
    p: Protocol,
    root: &Path,
    run_id: &str,
) -> (runner::Verdict, runner::Lifecycle, Option<PathBuf>) {
    let cfg = runner::Config {
        manifest: manifest(&p, run_id),
        specs: [(
            TargetKind::Swarmail,
            swarmail_spec(targets::swarmail_template(0)),
        )]
        .into(),
        output: root.join("out"),
        scratch: root.join("s"),
        protocol: p,
    };
    std::fs::create_dir(root.join("s")).unwrap();
    runner::run(cfg, wire::Clock(Instant::now())).await
}

fn rows(dir: &Path) -> Vec<Row> {
    let m = records::read_manifest(dir).unwrap();
    records::read_rows(dir, &m).unwrap()
}

// ---- arithmetic, against hand-computed vectors -------------------------

#[test]
fn nearest_rank_matches_hand_computed_ranks() {
    let ten: Vec<i64> = (1..=10).collect();
    // rank = ceil(p·n/100): p50 → 5, p95 → 10, p99 → 10, p10 → 1, p11 → 2.
    for (p, want) in [
        (50, 5),
        (95, 10),
        (99, 10),
        (10, 1),
        (11, 2),
        (100, 10),
        (1, 1),
    ] {
        assert_eq!(stats::nearest_rank(&ten, p), Some(want), "p{p}");
    }
    assert_eq!(stats::nearest_rank(&[7], 99), Some(7));
    assert_eq!(
        stats::nearest_rank(&[], 50),
        None,
        "an empty set has no quantile, not zero"
    );
    assert_eq!(stats::nearest_rank(&ten, 0), None);
    assert_eq!(stats::nearest_rank(&ten, 101), None);
    // Signed values sort as signed: an observation before the ack stays negative.
    let q = stats::quantiles(vec![30, -20, 10, -5], 2, &[50, 99]);
    assert_eq!(q.samples, 4);
    assert_eq!(
        q.censored, 2,
        "censored samples are counted beside, never folded in"
    );
    assert_eq!(q.values, vec![(50, Some(-5)), (99, Some(30))]);
    let none = stats::quantiles(Vec::new(), 3, &[50]);
    assert_eq!(none.values, vec![(50, None)]);
}

#[test]
fn reconciliation_rates_use_their_own_windows() {
    let r = stats::Reconciliation {
        attempted: 10,
        accepted: 8,
        rejected: 1,
        failed_before_terminator: 0,
        ambiguous: 1,
        visible_unique: 6,
        missing_accepted: 2,
        send_window_ns: 2_000_000_000,
        verified_window_ns: 3_000_000_000,
        ..Default::default()
    };
    assert!(r.balanced());
    assert!(!r.exact());
    assert_eq!(
        r.accepted_per_second(),
        Some(4.0),
        "8 accepted / 2 s send window"
    );
    assert_eq!(
        r.verified_per_second(),
        Some(2.0),
        "6 verified / 3 s verified window"
    );
    let zero = stats::Reconciliation::default();
    assert_eq!(zero.accepted_per_second(), None, "no window, no rate");
    let unbalanced = stats::Reconciliation {
        attempted: 2,
        accepted: 1,
        ..Default::default()
    };
    assert!(!unbalanced.balanced());
}

#[test]
fn the_corpus_is_deterministic_equal_width_and_dot_stuffed() {
    let kinds = smoke().corpus.kinds;
    let a = wire::authored(163, "tag", 7, &kinds);
    assert_eq!(
        a,
        wire::authored(163, "tag", 7, &kinds),
        "same inputs, same bytes"
    );
    assert_ne!(
        a.bytes,
        wire::authored(164, "tag", 7, &kinds).bytes,
        "the seed matters"
    );
    let tokens: Vec<_> = (0..300)
        .map(|i| wire::authored(163, "tag", i, &kinds).token)
        .collect();
    assert!(
        tokens.iter().all(|t| t.len() == tokens[0].len()),
        "no token can be a substring of another"
    );
    assert_eq!(targets::token_of(&a.subject), a.token);
    assert_eq!(targets::token_of("unrelated subject"), "");
    assert_eq!(
        wire::dot_stuff(b".a\r\n..b\r\nc.\r\n"),
        b"..a\r\n...b\r\nc.\r\n"
    );
    assert!(wire::same_body(Some("x\r\ny"), Some("x\ny\n")));
    assert!(!wire::same_body(Some("x"), Some("y")));
    assert!(!wire::same_body(Some("x"), None));
    assert!(wire::same_body(None, Some("  ")));
}

// ---- the frozen method and the source lock ------------------------------

#[test]
fn the_committed_protocol_and_lock_refuse_collection_and_say_why() {
    let p = Protocol::parse(PROTOCOL).expect("the committed protocol parses");
    let s = SourceLock::parse(SOURCES).expect("the committed lock parses");
    s.validate()
        .expect("an honest unverified lock is structurally valid");
    let why = refusals(protocol::collection_ready(&p, &s));
    for expected in [
        "protocol is a draft, not frozen",
        "mailpit source is unverified",
        "mailhog source is unverified",
        "resource budget is not enforced",
        "mailpit poll has no frozen interval",
        "unresolved: ",
    ] {
        assert!(
            why.contains(expected),
            "missing refusal {expected:?} in:\n{why}"
        );
    }
    assert_eq!(p.targets, TargetKind::ALL.to_vec());
    assert_eq!(p.statistics.orders.len(), 6, "every target order once");
    assert!(
        p.throughput.iter().any(|t| t.sessions == 1000),
        "the bounded 1,000-session profile is declared"
    );
}

#[test]
fn the_protocol_validator_refuses_every_malformed_method() {
    let base = json(SMOKE);
    let mutate = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v = base.clone();
        f(&mut v);
        v
    };
    // Shape: unknown, missing and duplicated fields never parse.
    assert!(protocol_from(mutate(&|v| v["extra"] = 1.into())).is_err());
    assert!(
        protocol_from(mutate(&|v| {
            v.as_object_mut().unwrap().remove("statistics");
        }))
        .is_err()
    );
    let dup =
        String::from_utf8(SMOKE.to_vec())
            .unwrap()
            .replacen("\"id\":", "\"id\": \"x\", \"id\":", 1);
    assert!(
        Protocol::parse(dup.as_bytes()).is_err(),
        "a duplicate key refuses"
    );
    smoke()
        .validate()
        .expect("the fixture is structurally valid");
    let cases: Vec<(serde_json::Value, &str)> = vec![
        (
            mutate(&|v| v["protocol_version"] = 2.into()),
            "protocol_version 2",
        ),
        (
            mutate(&|v| v["supersedes"] = "older".into()),
            "change_reason",
        ),
        (
            mutate(&|v| v["throughput"][0]["messages"] = 0.into()),
            "messages 0 outside",
        ),
        (
            mutate(&|v| v["throughput"][0]["sessions"] = 1001.into()),
            "sessions 1001 outside",
        ),
        (
            mutate(&|v| v["throughput"][1]["name"] = "smoke-c3-reused".into()),
            "duplicated",
        ),
        (
            mutate(&|v| v["storage"]["retention"][0]["max_messages"] = 10.into()),
            "overflow swarmail's retention",
        ),
        (
            mutate(&|v| v["statistics"]["orders"] = serde_json::json!([["mailpit"]])),
            "not a permutation",
        ),
        (
            mutate(&|v| v["statistics"]["quantile_method"] = "linear".into()),
            "not nearest-rank",
        ),
        (
            mutate(&|v| v["waits"][1]["observation"][0]["interval_ms"] = serde_json::Value::Null),
            "no frozen interval",
        ),
        (
            mutate(&|v| v["waits"][0]["observation"][0]["interval_ms"] = 5.into()),
            "native wait carries a poll interval",
        ),
        (
            mutate(&|v| v["waits"][0]["observation"][0]["justification"] = " ".into()),
            "no justification",
        ),
        (
            mutate(&|v| v["resources"]["enforced"] = true.into()),
            "must state cpus and memory_mib",
        ),
        (
            mutate(&|v| v["storage"]["mode"] = "disk".into()),
            "neither memory nor durable",
        ),
        (
            mutate(&|v| {
                v["targets"] = serde_json::json!(["swarmail", "mailpit"]);
                v["statistics"]["orders"] = serde_json::json!([["swarmail", "mailpit"]]);
                v["storage"]["retention"] = serde_json::json!([{"target": "swarmail", "max_messages": null}, {"target": "mailpit", "max_messages": null}]);
                v["waits"][0]["observation"] = serde_json::json!([
                    {"target": "swarmail", "mode": "native", "interval_ms": null, "justification": "x"},
                    {"target": "mailpit", "mode": "native", "interval_ms": null, "justification": "x"}]);
            }),
            "no source-verified native wait is implemented for mailpit",
        ),
    ];
    for (v, want) in cases {
        let why = refusals(protocol_from(v).expect("parses").validate());
        assert!(why.contains(want), "expected {want:?} in:\n{why}");
    }
}

fn verified_mailpit() -> serde_json::Value {
    let rev = "0123456789abcdef0123456789abcdef01234567";
    let home = "https://github.com/axllent/mailpit";
    serde_json::json!({
        "target": "mailpit", "status": "verified", "unverified_reason": null,
        "source": {
            "repository": home, "revision": rev, "release": "vX.Y.Z (illustrative test value)",
            "license_spdx": "MIT", "license_document": format!("{home}/blob/{rev}/LICENSE"),
            "api_document": format!("{home}/blob/{rev}/docs/api.md"),
            "setup_document": format!("{home}/blob/{rev}/README.md"), "captured_on": "2026-10-28"
        },
        "artifact": {
            "download": format!("{home}/releases/download/vX/archive.tar.gz"),
            "file_name": "archive.tar.gz", "sha256": "a".repeat(64), "executable_sha256": "b".repeat(64),
            "os": "linux", "arch": "x86_64"
        },
        "supports_memory_storage": true,
        "launch": ["--smtp", "{smtp_addr}", "--listen", "{http_addr}"]
    })
}

#[test]
fn the_source_lock_refuses_mutable_foreign_and_incomplete_pins() {
    let lock = |entry: serde_json::Value| {
        lock_from(serde_json::json!({"lock_version": 1, "entries": [entry]})).expect("parses")
    };
    // The shape a verified pin must have validates (values are synthetic,
    // used only to prove the validator; nothing about Mailpit is asserted).
    lock(verified_mailpit())
        .validate()
        .expect("a complete pin validates");
    let cases: Vec<(&str, serde_json::Value, &str)> = vec![
        ("/source/revision", "v1.2.3".into(), "not a full commit id"),
        (
            "/source/repository",
            "https://github.com/someone/mailpit-fork".into(),
            "is not the official",
        ),
        (
            "/source/license_spdx",
            "GPL-3.0-only".into(),
            "not on the permissive list",
        ),
        ("/source/captured_on", "yesterday".into(), "not YYYY-MM-DD"),
        (
            "/artifact/download",
            "https://example.invalid/mailpit.tgz".into(),
            "not under the official repository",
        ),
        (
            "/artifact/executable_sha256",
            "abc".into(),
            "64 lowercase hex",
        ),
        ("/artifact/os", "".into(), "platform is incomplete"),
        (
            "/supports_memory_storage",
            serde_json::Value::Null,
            "storage capability is not recorded",
        ),
        (
            "/launch",
            serde_json::json!(["--smtp", "{smtp_addr}"]),
            "launch template",
        ),
        (
            "/launch",
            serde_json::json!(["{smtp_addr}", "{http_addr}", "{home}"]),
            "launch template",
        ),
        (
            "/source",
            serde_json::Value::Null,
            "both an official source and a pinned artifact",
        ),
        (
            "/unverified_reason",
            "but".into(),
            "carries an unverified_reason",
        ),
    ];
    for (path, value, want) in cases {
        let mut e = verified_mailpit();
        *e.pointer_mut(path).unwrap() = value;
        let why = refusals(lock(e).validate());
        assert!(why.contains(want), "{path}: expected {want:?} in:\n{why}");
    }
    let swarmail_pinned = serde_json::json!({"target": "swarmail", "status": "verified", "unverified_reason": null,
        "source": null, "artifact": null, "supports_memory_storage": true, "launch": null});
    assert!(refusals(lock(swarmail_pinned).validate()).contains("must be the candidate"));
    let mut silent = verified_mailpit();
    silent["status"] = "unverified".into();
    assert!(refusals(lock(silent).validate()).contains("unverified without a reason"));
    let mut candidate = verified_mailpit();
    candidate["status"] = "candidate".into();
    assert!(refusals(lock(candidate).validate()).contains("cannot be the candidate"));
    let twice = lock_from(
        serde_json::json!({"lock_version": 1, "entries": [verified_mailpit(), verified_mailpit()]}),
    )
    .unwrap();
    assert!(refusals(twice.validate()).contains("2 lock entries"));
    // A verified pin alone does not admit a draft protocol.
    let p = Protocol::parse(PROTOCOL).unwrap();
    let why = refusals(protocol::collection_ready(&p, &lock(verified_mailpit())));
    assert!(
        why.contains("swarmail has no source lock entry")
            && why.contains("mailhog has no source lock entry"),
        "{why}"
    );
}

// ---- real runs against the real binary ---------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_real_smoke_run_reconciles_every_message_and_wait_and_seals() {
    let root = tempfile::tempdir().unwrap();
    let (verdict, life, sealed) = run(smoke(), root.path(), "smoke-run").await;
    assert_eq!(verdict, runner::Verdict::Clean);
    let dir = sealed.expect("a clean run is sealed");
    assert!(
        life.cleanups.iter().all(|c| c.proved() && !c.forced_kill),
        "{:?}",
        life.cleanups
    );
    assert_eq!(
        life.cleanups.len(),
        5,
        "one warmup and four measured instances, each torn down"
    );
    assert!(
        std::fs::read_dir(root.path().join("s"))
            .unwrap()
            .next()
            .is_none(),
        "no scratch leaf survives"
    );

    // Private, exclusive output.
    assert_eq!(
        std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for f in [
        "manifest.json",
        "samples.jsonl",
        "report.json",
        "index.json",
    ] {
        let mode = std::fs::metadata(dir.join(f)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{f}");
    }
    let admitted = records::admit(&dir).expect("the sealed run admits");

    // Recompute the accounting from the raw rows, independently of report.rs.
    let rows = admitted.rows;
    for profile in ["smoke-c3-reused", "smoke-c4-fresh"] {
        let sends: Vec<_> = rows
            .iter()
            .filter(|r| r.profile == profile && r.phase == Phase::Send)
            .collect();
        assert_eq!(sends.len(), 12);
        assert!(
            sends
                .iter()
                .all(|r| r.outcome == "accepted" && r.code == Some(250)),
            "{sends:?}"
        );
        let mut samples: Vec<_> = sends.iter().map(|r| r.sample).collect();
        samples.sort();
        assert_eq!(
            samples,
            (0..12).collect::<Vec<_>>(),
            "every planned index exactly once"
        );
        let visible: Vec<_> = rows
            .iter()
            .filter(|r| r.profile == profile && r.phase == Phase::Visible)
            .collect();
        assert_eq!(visible.len(), 12);
        assert!(visible.iter().all(|r| r.outcome == "visible"
            && r.occurrences == Some(1)
            && r.fidelity_ok == Some(true)));
        let tokens: std::collections::BTreeSet<_> = sends.iter().map(|r| r.token.clone()).collect();
        assert_eq!(
            tokens,
            visible.iter().map(|r| r.token.clone()).collect(),
            "the accepted ids are the visible ids"
        );
        let v = rows
            .iter()
            .find(|r| r.profile == profile && r.phase == Phase::Verified)
            .unwrap();
        assert_eq!(v.outcome, "verified");
        assert!(
            v.start_ns <= v.ack_ns && v.ack_ns <= v.observed_ns,
            "send window inside verified window"
        );
    }
    for profile in ["smoke-wait-native", "smoke-wait-poll"] {
        let of = |phase| {
            rows.iter()
                .filter(move |r: &&Row| r.profile == profile && r.phase == phase)
        };
        assert!(of(Phase::Present).all(|r| r.outcome == "observed"
            && r.invoked_ns >= r.ack_ns
            && r.observed_ns >= r.invoked_ns));
        assert!(
            of(Phase::Future).all(|r| r.outcome == "observed"
                && r.invoked_ns <= r.start_ns
                && r.ack_ns.is_some())
        );
        assert!(of(Phase::Empty).all(|r| r.outcome == "timeout" && r.observed_ns.is_none()));
        assert!(
            of(Phase::Empty).all(|r| r.requests >= Some(1)),
            "every observation records its requests"
        );
        assert_eq!(
            of(Phase::Present).count() + of(Phase::Future).count() + of(Phase::Empty).count(),
            8
        );
    }

    // The report is arithmetic over those rows.
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        report["evidence"], false,
        "a smoke report is never evidence"
    );
    for cell in report["throughput"].as_array().unwrap() {
        assert_eq!(cell["exact"], true, "{cell}");
        let rec = &cell["reconciliation"];
        let want =
            rec["accepted"].as_f64().unwrap() * 1e9 / rec["send_window_ns"].as_f64().unwrap();
        assert!((cell["accepted_per_second"].as_f64().unwrap() - want).abs() < 1e-6 * want);
    }
    let native = report["waits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["profile"] == "smoke-wait-native")
        .unwrap();
    assert_eq!(native["present_latency"]["samples"], 3);
    assert_eq!(native["outcomes"]["empty"]["timeout"], 2);
    // The signed offset is recomputed from raw rows and must match exactly.
    let mut offsets: Vec<i64> = rows
        .iter()
        .filter(|r| r.profile == "smoke-wait-native" && r.phase == Phase::Future)
        .map(|r| r.observed_ns.unwrap() as i64 - r.ack_ns.unwrap() as i64)
        .collect();
    offsets.sort();
    assert_eq!(
        native["future_offset_from_ack"]["values"][0][1],
        serde_json::json!(stats::nearest_rank(&offsets, 50))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_bounded_thousand_session_profile_accounts_for_every_attempt() {
    let mut v = json(SMOKE);
    v["throughput"] = serde_json::json!([{
        "name": "stress-c1000-fresh", "repetitions": 1, "warmup_messages": 0, "messages": 1000,
        "sessions": 1000, "connection": "fresh", "send_deadline_ms": 120000, "verify_deadline_ms": 60000
    }]);
    v["waits"] = serde_json::json!([]);
    let p = protocol_from(v).unwrap();
    p.validate().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (verdict, life, sealed) = run(p, root.path(), "stress-run").await;
    // Saturation or refusal would be data, not an infrastructure failure:
    // the run must seal either way, with every attempt in one disposition.
    assert!(
        !matches!(verdict, runner::Verdict::Infrastructure(_)),
        "{verdict:?}"
    );
    assert!(life.cleanups.iter().all(|c| c.proved()));
    let dir = sealed.expect("sealed");
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    let cell = &report["throughput"][0];
    let rec = &cell["reconciliation"];
    let n = |k: &str| rec[k].as_u64().unwrap();
    assert_eq!(n("attempted") + cell["unstarted"].as_u64().unwrap(), 1000);
    assert_eq!(
        n("accepted") + n("rejected") + n("failed_before_terminator") + n("ambiguous"),
        n("attempted")
    );
    assert_eq!(
        rows(&dir).iter().filter(|r| r.phase == Phase::Send).count(),
        1000
    );
}

// ---- ownership, lifecycle and privacy -----------------------------------

fn proc_state(pid: i32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit(')').next()?.trim().chars().next()
}

/// Gone, or a zombie awaiting its reaper: either way not running.
async fn stops_running(pid: i32) -> bool {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if matches!(proc_state(pid), None | Some('Z') | Some('X')) {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn a_launched_target_is_closed_private_bounded_and_fully_reaped() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("leaf");
    // Unowned neighbours: a process and a listener this test holds.
    let mut neighbour = std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .unwrap();
    let neighbour_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let clock = wire::Clock(Instant::now());
    let mut owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &scratch,
        &limits(),
        &clock,
    )
    .await
    .expect("the real binary becomes ready");
    let pid = owned.pid();

    // Closed environment: exactly the keys the owner sets, none inherited.
    let environ = std::fs::read(format!("/proc/{pid}/environ")).unwrap();
    let mut keys: Vec<String> = environ
        .split(|b| *b == 0)
        .filter(|kv| !kv.is_empty())
        .map(|kv| {
            String::from_utf8_lossy(kv)
                .split('=')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect();
    keys.sort();
    let want = [
        "HOME",
        "LANG",
        "PATH",
        "TMPDIR",
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
    ];
    assert_eq!(keys, want, "the child environment is closed");
    assert!(
        std::env::vars().any(|(k, _)| !want.contains(&k.as_str())),
        "the canary: this process has keys the child must not"
    );
    let environ = String::from_utf8_lossy(&environ);
    let canon = std::fs::canonicalize(&scratch).unwrap();
    assert!(
        environ.contains(&format!("HOME={}", canon.join("home").display())),
        "a private HOME"
    );

    // Diagnostics discarded, stdin closed, scratch private, the fd ceiling enforced.
    for fd in [0, 2] {
        assert_eq!(
            std::fs::read_link(format!("/proc/{pid}/fd/{fd}")).unwrap(),
            Path::new("/dev/null"),
            "fd {fd}"
        );
    }
    for leaf in ["", "home", "tmp", "config", "cache", "data", "state"] {
        let mode = std::fs::metadata(scratch.join(leaf))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "scratch/{leaf}");
    }
    let limits_text = std::fs::read_to_string(format!("/proc/{pid}/limits")).unwrap();
    let open_files = limits_text
        .lines()
        .find(|l| l.starts_with("Max open files"))
        .unwrap();
    assert!(
        open_files
            .split_whitespace()
            .filter(|w| *w == "4096")
            .count()
            == 2,
        "{open_files}"
    );
    // Its own process group, so signals reach it and nothing else.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let pgrp: i32 = stat
        .rsplit(')')
        .next()
        .unwrap()
        .split_whitespace()
        .nth(2)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(pgrp, pid);

    let facts = owned.shutdown().await;
    assert!(facts.proved() && !facts.forced_kill, "{facts:?}");
    assert!(!scratch.exists());
    assert!(stops_running(pid).await);
    // Unowned resources survive the owner's cleanup.
    assert_eq!(
        neighbour.try_wait().unwrap(),
        None,
        "the unowned process still runs"
    );
    let neighbour_addr = neighbour_listener.local_addr().unwrap();
    assert!(
        std::net::TcpStream::connect(neighbour_addr).is_ok(),
        "the unowned listener still accepts"
    );
    neighbour.kill().unwrap();
    neighbour.wait().unwrap();
    // Idempotent: a second shutdown proves the same facts and harms nothing.
    assert!(owned.shutdown().await.reaped);
}

#[tokio::test]
async fn launch_refusals_happen_before_spawn_and_partial_starts_are_cleaned() {
    let root = tempfile::tempdir().unwrap();
    let clock = wire::Clock(Instant::now());
    let mut wrong = swarmail_spec(targets::swarmail_template(0));
    wrong.expected_sha256 = Some("0".repeat(64));
    let leaf = root.path().join("a");
    assert_eq!(
        targets::Owned::launch(&wrong, &leaf, &limits(), &clock)
            .await
            .err(),
        Some(targets::LaunchError::DigestMismatch)
    );
    assert!(!leaf.exists(), "a refused digest creates nothing");
    let mut missing = swarmail_spec(targets::swarmail_template(0));
    missing.binary = root.path().join("no-such-server");
    assert_eq!(
        targets::Owned::launch(&missing, &leaf, &limits(), &clock)
            .await
            .err(),
        Some(targets::LaunchError::Binary("unreadable"))
    );
    let mut plain = swarmail_spec(targets::swarmail_template(0));
    plain.binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert_eq!(
        targets::Owned::launch(&plain, &leaf, &limits(), &clock)
            .await
            .err(),
        Some(targets::LaunchError::Binary("not executable"))
    );
    std::fs::create_dir(&leaf).unwrap();
    let ok = swarmail_spec(targets::swarmail_template(0));
    assert_eq!(
        targets::Owned::launch(&ok, &leaf, &limits(), &clock)
            .await
            .err(),
        Some(targets::LaunchError::Scratch),
        "scratch is never reused"
    );
    // A server that exits before readiness: refused, and its partial start
    // is still reaped and its scratch removed.
    let mut broken = swarmail_spec(targets::swarmail_template(0));
    broken.template.push("--no-such-flag".into());
    let leaf = root.path().join("b");
    match targets::Owned::launch(&broken, &leaf, &limits(), &clock)
        .await
        .err()
    {
        Some(targets::LaunchError::NotReady(_, facts)) => assert!(facts.proved(), "{facts:?}"),
        other => panic!("expected a readiness refusal, got {other:?}"),
    }
    assert!(!leaf.exists());
}

#[tokio::test]
async fn a_dropped_owner_kills_its_group_but_proves_no_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("leaf");
    let clock = wire::Clock(Instant::now());
    let owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &scratch,
        &limits(),
        &clock,
    )
    .await
    .unwrap();
    let pid = owned.pid();
    drop(owned); // cancellation
    assert!(
        stops_running(pid).await,
        "the dropped owner's group is killed"
    );
    assert!(
        scratch.exists(),
        "nothing was proved, so nothing was cleaned: the leaf stays for the operator"
    );
}

// ---- sealing and admission ----------------------------------------------

fn tiny() -> Protocol {
    let mut v = json(SMOKE);
    v["throughput"] = serde_json::json!([v["throughput"][1].clone()]);
    v["throughput"][0]["messages"] = 4.into();
    v["waits"] = serde_json::json!([]);
    protocol_from(v).unwrap()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
    }
}

fn tamper(dir: &Path, f: impl FnOnce(&mut Vec<u8>)) {
    let path = dir.join("samples.jsonl");
    let mut bytes = std::fs::read(&path).unwrap();
    f(&mut bytes);
    std::fs::write(path, bytes).unwrap();
}

#[tokio::test]
async fn admission_refuses_truncated_foreign_duplicate_missing_and_altered_runs() {
    let root = tempfile::tempdir().unwrap();
    let (verdict, _, sealed) = run(tiny(), root.path(), "admit-run").await;
    assert_eq!(verdict, runner::Verdict::Clean);
    let good = sealed.unwrap();
    records::admit(&good).expect("the untouched run admits");
    let lines = || String::from_utf8(std::fs::read(good.join("samples.jsonl")).unwrap()).unwrap();
    let first = lines().lines().next().unwrap().to_owned();
    type Tamper = Box<dyn FnOnce(&mut Vec<u8>)>;
    let cases: Vec<(&str, Tamper)> = vec![
        (
            "truncated final row",
            Box::new(|b: &mut Vec<u8>| {
                b.pop();
            }),
        ),
        (
            "belongs to another run",
            Box::new({
                let row = first.replace("\"run_id\":\"admit-run\"", "\"run_id\":\"other-run\"");
                move |b: &mut Vec<u8>| b.extend_from_slice(format!("{row}\n").as_bytes())
            }),
        ),
        (
            "duplicates a coordinate",
            Box::new({
                let row = first.clone();
                move |b: &mut Vec<u8>| b.extend_from_slice(format!("{row}\n").as_bytes())
            }),
        ),
        (
            "unplanned row",
            Box::new({
                let row = first.replace(
                    "\"profile\":\"smoke-c4-fresh\"",
                    "\"profile\":\"best-rerun\"",
                );
                move |b: &mut Vec<u8>| b.extend_from_slice(format!("{row}\n").as_bytes())
            }),
        ),
        (
            "4 planned",
            Box::new(|b: &mut Vec<u8>| {
                // Drop one send row: a selectively retained run.
                let text = String::from_utf8(b.clone()).unwrap();
                let mut kept: Vec<&str> = text.lines().collect();
                let i = kept
                    .iter()
                    .position(|l| l.contains("\"phase\":\"send\""))
                    .unwrap();
                kept.remove(i);
                *b = format!("{}\n", kept.join("\n")).into_bytes();
            }),
        ),
        (
            "unknown field",
            Box::new({
                let row = first.replacen('{', "{\"extra\":1,", 1);
                move |b: &mut Vec<u8>| *b = format!("{row}\n").into_bytes()
            }),
        ),
    ];
    for (i, (want, f)) in cases.into_iter().enumerate() {
        let dir = root.path().join(format!("case-{i}"));
        copy_dir(&good, &dir);
        tamper(&dir, f);
        let m = records::read_manifest(&dir).unwrap();
        let why = records::read_rows(&dir, &m).expect_err(want);
        assert!(why.contains(want), "expected {want:?}, got {why}");
        let why = records::admit(&dir).expect_err("altered bytes never admit");
        assert!(why.contains("index does not match"), "{why}");
    }
    // An extra file, or a missing one, breaks the index too.
    let dir = root.path().join("extra");
    copy_dir(&good, &dir);
    std::fs::write(dir.join("best.json"), b"{}").unwrap();
    assert!(
        records::admit(&dir)
            .unwrap_err()
            .contains("index does not match")
    );
    let dir = root.path().join("missing");
    copy_dir(&good, &dir);
    std::fs::remove_file(dir.join("report.json")).unwrap();
    assert!(
        records::admit(&dir)
            .unwrap_err()
            .contains("index does not match")
    );
}

#[test]
fn an_unproved_or_colliding_run_is_never_sealed_or_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let p = tiny();
    let m = manifest(&p, "partial-run");
    let path = root.path().join("out");
    let mut dir = records::RunDir::create(&path, &m).unwrap();
    assert!(
        records::RunDir::create(&path, &m).is_err(),
        "an existing run directory is never reused"
    );
    let mut row = Row {
        row_version: records::ROW_VERSION,
        run_id: "partial-run".into(),
        target: TargetKind::Swarmail,
        profile: "smoke-c4-fresh".into(),
        repetition: 0,
        phase: Phase::Send,
        sample: 0,
        token: None,
        outcome: "accepted".into(),
        reason: None,
        code: Some(250),
        invoked_ns: Some(1),
        start_ns: Some(2),
        ack_ns: Some(3),
        observed_ns: None,
        occurrences: None,
        fidelity_ok: None,
        requests: None,
    };
    dir.append(&row).unwrap();
    row.sample = 1;
    dir.append(&row).unwrap();
    dir.write_artifact("report.json", b"{}").unwrap();
    assert!(
        dir.write_artifact("report.json", b"{}").is_err(),
        "artifacts are created once"
    );
    assert!(
        dir.finalize(false)
            .unwrap_err()
            .contains("cleanup is not proved")
    );
    assert!(!path.join("index.json").exists());
    assert!(records::admit(&path).unwrap_err().contains("incomplete"));
    // The streamed prefix survives untouched.
    assert_eq!(rows_prefix(&path), 2);
    // Proved cleanup is not enough when the rows are short of the plan.
    let dir = records::RunDir::create(&root.path().join("short"), &m).unwrap();
    dir.write_artifact("report.json", b"{}").unwrap();
    assert!(dir.finalize(true).unwrap_err().contains("planned"));
    assert!(!root.path().join("short/index.json").exists());
}

fn rows_prefix(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("samples.jsonl"))
        .unwrap()
        .lines()
        .count()
}

// ---- the one command -----------------------------------------------------

/// Run `scripts/compare-mail.sh` with inert agent witnesses first on PATH:
/// any of them executing leaves a mark. Returns (exit code, stderr+stdout).
fn compare_mail(args: &[&str], witness: &Path) -> (i32, String) {
    let bin = witness.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for agent in ["claude", "codex", "dsh", "gemini", "opencode", "brokkr"] {
        let path = bin.join(agent);
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\necho {agent} >> '{}'\n",
                witness.join("launched").display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = std::process::Command::new("bash")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/compare-mail.sh"))
        .args(args)
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn every_delivery_mode_refuses_with_its_blocked_phase_and_launches_no_agent() {
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    let input = root.path().join("input");
    std::fs::write(&input, b"{}").unwrap();
    let i = input.to_str().unwrap();

    let (code, text) = compare_mail(&[], &w);
    assert_eq!(code, 2);
    assert!(
        text.contains("compare-mail.sh local"),
        "no arguments prints usage: {text}"
    );

    let eval_blocked = |text: &str| {
        text.contains("blocked at phase \"eval-build\"")
            || text.contains("blocked at phase \"comparison-integration\"")
    };
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["bogus", "--output", o], "\"arguments\""),
        (vec!["local"], "--output DIR is required"),
        (
            vec!["local", "--output", o, "--surprise", "x"],
            "unknown argument",
        ),
        (vec!["local", "--output", o], "\"competitor-binaries\""),
        (
            vec!["local", "--output", o, "--mailpit", i, "--mailhog", i],
            "is not an executable file",
        ),
        (vec!["evidence", "--output", o], "--baseline is required"),
        (
            vec![
                "evidence",
                "--output",
                o,
                "--baseline",
                i,
                "--final",
                i,
                "--receipts",
                "/nonexistent/receipts",
            ],
            "does not exist",
        ),
        (
            vec!["measure", "--output", o, "--baseline", i, "--receipts", i],
            "--config is required",
        ),
    ];
    for (args, want) in cases {
        let (code, text) = compare_mail(&args, &w);
        assert_eq!(code, 2, "{args:?}: {text}");
        assert!(text.contains(want), "{args:?}: expected {want:?} in {text}");
    }
    for args in [
        vec!["controls", "--output", o],
        vec![
            "evidence",
            "--output",
            o,
            "--baseline",
            i,
            "--final",
            i,
            "--receipts",
            i,
        ],
        vec![
            "measure",
            "--output",
            o,
            "--baseline",
            i,
            "--config",
            i,
            "--receipts",
            i,
            "--dry-run",
        ],
    ] {
        let (code, text) = compare_mail(&args, &w);
        assert_eq!(code, 2, "{args:?}: {text}");
        assert!(eval_blocked(&text), "{args:?}: {text}");
    }
    std::fs::create_dir(&out).unwrap();
    let (code, text) = compare_mail(&["local", "--output", o], &w);
    assert_eq!(code, 2);
    assert!(text.contains("never overwritten"), "{text}");
    assert!(
        !w.join("launched").exists(),
        "a delivery mode launched an agent witness"
    );
    assert!(
        std::fs::read_dir(&out).unwrap().next().is_none(),
        "a refusal writes no output"
    );
}
