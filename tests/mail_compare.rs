//! The fair wire comparison driver (issue #163, proposed decision 0091),
//! proved against the real Swarmail binary it launches and owns: validator
//! refusals, independent arithmetic, real throughput and wait runs,
//! lifecycle and privacy facts read from the kernel, sealing and admission;
//! the source lock against its captured documents; and the Mailpit MCP
//! adapter's protocol and refusal paths against a bounded local responder.
//! The pinned Mailpit and MailHog executables are not in CI, so the tests
//! that run them (decoders, the adapter over real SMTP → Mailpit HTTP → MCP,
//! a three-target run) are ignored by default and fail without them.

// The driver's own modules, compiled into this test crate. Some items serve
// only the driver binary (its CLI), so dead-code analysis here is partial.
#[allow(dead_code)]
#[path = "../benches/mail_compare/mailpit_mcp.rs"]
mod mailpit_mcp;
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
        .expect("the committed lock is structurally valid, its gaps named");
    s.verify_files(&evals_dir())
        .expect("every captured document matches its digest and the release metadata");
    protocol::retention_matches(&p, &s).expect("retention is stated as launched");
    let why = refusals(protocol::collection_ready(&p, &s));
    for expected in [
        "protocol is a draft, not frozen",
        "mailpit: source revision unresolved: the release metadata's target_commitish is the branch 'develop'",
        "mailhog: source revision unresolved: the release metadata's target_commitish is the branch 'master'",
        "sources/mailpit/mailpit-swagger.json attribution unresolved",
        "mailpit: no captured setup document",
        "mailhog: wait source unresolved",
        "mailpit does not store in memory",
        "resource budget is not enforced",
        "unresolved: primary attribution of MailHog's /api/v1/messages/{id} read",
    ] {
        assert!(
            why.contains(expected),
            "missing refusal {expected:?} in:\n{why}"
        );
    }
    assert!(
        !why.contains("mailhog does not store in memory"),
        "MailHog's memory store is not refused: {why}"
    );
    assert_eq!(p.targets, TargetKind::ALL.to_vec());
    assert_eq!(p.statistics.orders.len(), 6, "every target order once");
    assert!(
        p.throughput.iter().any(|t| t.sessions == 1000),
        "the bounded 1,000-session profile is declared"
    );
    // Mailpit keeps 500 messages unless launched otherwise; the committed
    // method states the cap it launches with, above every profile.
    let cap = s.pin(TargetKind::Mailpit).unwrap().storage.max_messages;
    assert_eq!(cap, Some(10_000));
    for t in &p.throughput {
        assert!(u64::from(t.messages) < cap.unwrap(), "{}", t.name);
    }
}

#[test]
fn the_provisioned_artifacts_match_the_pins_and_their_basis() {
    let s = SourceLock::parse(SOURCES).unwrap();
    let mp = &s.pin(TargetKind::Mailpit).unwrap().artifact;
    assert_eq!(
        mp.sha256,
        "4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c"
    );
    assert_eq!(
        mp.digest_basis,
        protocol::DigestBasis::UpstreamReleaseMetadata
    );
    assert_eq!(mp.upstream_digest.as_deref(), Some(mp.sha256.as_str()));
    assert_eq!(
        mp.executable_sha256,
        "262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8"
    );
    let mh = &s.pin(TargetKind::Mailhog).unwrap().artifact;
    assert_eq!(
        mh.sha256,
        "e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7"
    );
    assert_eq!(mh.digest_basis, protocol::DigestBasis::LocallyComputed);
    assert_eq!(mh.upstream_digest, None, "MailHog publishes no digest");
    assert_eq!(mh.executable_sha256, mh.sha256);
    // The historical v1 bytes are kept and still read by their own
    // reader; the current reader refuses them outright.
    let v1 = include_bytes!("../docs/evals/history/2026-10-28-sources.lock.v1.json");
    protocol::v1::SourceLock::parse(v1)
        .unwrap()
        .validate()
        .expect("the historical lock still validates under v1");
    let why = SourceLock::parse(v1)
        .expect_err("v1 is not a current lock")
        .0
        .join("");
    assert!(why.contains("historical"), "{why}");
}

fn evals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/evals")
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

fn committed_entry(t: TargetKind) -> serde_json::Value {
    json(SOURCES)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["target"] == t.name())
        .unwrap()
        .clone()
}

fn lock_of(entries: Vec<serde_json::Value>) -> SourceLock {
    lock_from(
        serde_json::json!({"lock_version": 2, "checked_on": "2026-10-28", "entries": entries}),
    )
    .expect("parses")
}

#[test]
fn the_source_lock_refuses_mutable_foreign_false_and_incomplete_pins() {
    let lock = |e: serde_json::Value| lock_of(vec![e]);
    lock(committed_entry(TargetKind::Mailpit))
        .validate()
        .expect("the committed pin validates");
    let cases: Vec<(&str, serde_json::Value, &str)> = vec![
        (
            "/pin/source_revision",
            serde_json::json!({"observed": "v1.32.0"}),
            "not a full commit id",
        ),
        (
            "/pin/source_revision",
            serde_json::json!({"unresolved": " "}),
            "unresolved without a reason",
        ),
        (
            "/pin/release/repository",
            "https://github.com/someone/mailpit-fork".into(),
            "is not the official",
        ),
        (
            "/pin/release/html_url",
            "https://example.invalid/v1.32.0".into(),
            "not the official tag page",
        ),
        (
            "/pin/release/published_at",
            "recently".into(),
            "is not a date",
        ),
        (
            "/pin/license/spdx",
            "GPL-3.0-only".into(),
            "not on the permissive list",
        ),
        ("/pin/license/holder", "".into(), "holder is not recorded"),
        (
            "/pin/artifact/download",
            "https://example.invalid/mailpit.tgz".into(),
            "not the official release asset",
        ),
        (
            "/pin/artifact/executable_sha256",
            "abc".into(),
            "64 lowercase hex",
        ),
        ("/pin/artifact/os", "".into(), "platform is incomplete"),
        (
            "/pin/artifact/digest_basis",
            "locally_computed".into(),
            "cannot carry an upstream digest",
        ),
        (
            "/pin/artifact/upstream_digest",
            serde_json::Value::Null,
            "needs the published digest",
        ),
        (
            "/pin/artifact/upstream_digest",
            "c".repeat(64).into(),
            "needs the published digest",
        ),
        (
            "/pin/artifact/member",
            serde_json::Value::Null,
            "names its member",
        ),
        (
            "/pin/artifact/kind",
            "executable".into(),
            "no member and one digest",
        ),
        (
            "/pin/artifact/version_output",
            "mailpit v1.31.0".into(),
            "does not name v1.32.0",
        ),
        (
            "/pin/documents/0/path",
            "../../../etc/passwd".into(),
            "is not under sources/mailpit/",
        ),
        (
            "/pin/documents/1/path",
            "sources/mailpit/mailpit-release.json".into(),
            "listed twice",
        ),
        ("/pin/documents/2/sha256", "nope".into(), "digest is not 64"),
        (
            "/pin/documents/2/attribution",
            serde_json::json!({"observed": {"url": "https://example.invalid/swagger.json", "revision": "main"}}),
            "not attributed to the official repository",
        ),
        (
            "/pin/documents/2/attribution",
            serde_json::json!({"observed": {"url": "https://github.com/axllent/mailpit/blob/v1.32.0/swagger.json", "revision": "v1.32.0"}}),
            "no full revision",
        ),
        (
            "/pin/documents/2/kind",
            "setup".into(),
            "no captured Api document",
        ),
        (
            "/pin/launch",
            serde_json::json!(["--smtp", "{smtp_addr}"]),
            "launch template",
        ),
        (
            "/pin/storage/max_messages",
            500.into(),
            "does not set --max 500",
        ),
        (
            "/pin/storage/cap_flag",
            serde_json::Value::Null,
            "go together",
        ),
        ("/pin/storage/basis", " ".into(), "no basis"),
        (
            "/pin/wait",
            serde_json::json!({"unresolved": ""}),
            "wait is unresolved without a reason",
        ),
        ("/role", "candidate".into(), "cannot be the candidate"),
        ("/pin", serde_json::Value::Null, "a competitor needs a pin"),
    ];
    for (path, value, want) in cases {
        let mut e = committed_entry(TargetKind::Mailpit);
        *e.pointer_mut(path).unwrap() = value;
        let why = refusals(lock(e).validate());
        assert!(why.contains(want), "{path}: expected {want:?} in:\n{why}");
    }
    let mut pinned = committed_entry(TargetKind::Swarmail);
    pinned["pin"] = committed_entry(TargetKind::Mailpit)["pin"].clone();
    assert!(refusals(lock(pinned).validate()).contains("recorded per run"));
    let mut competitor = committed_entry(TargetKind::Swarmail);
    competitor["role"] = "competitor".into();
    assert!(refusals(lock(competitor).validate()).contains("must be the candidate"));
    let twice = lock_of(vec![
        committed_entry(TargetKind::Mailpit),
        committed_entry(TargetKind::Mailpit),
    ]);
    assert!(refusals(twice.validate()).contains("2 lock entries"));
    let mut extra = committed_entry(TargetKind::Mailhog);
    extra["pin"]["verified"] = true.into();
    assert!(
        lock_from(
            serde_json::json!({"lock_version": 2, "checked_on": "2026-10-28", "entries": [extra]})
        )
        .is_err(),
        "an unknown field never parses"
    );
    let mut v = json(SOURCES);
    v["lock_version"] = 3.into();
    v["checked_on"] = "today".into();
    let why = refusals(lock_from(v).unwrap().validate());
    assert!(
        why.contains("lock_version 3 is not 2") && why.contains("checked_on"),
        "{why}"
    );
    // A complete pin alone does not admit a draft protocol.
    let p = Protocol::parse(PROTOCOL).unwrap();
    let why = refusals(protocol::collection_ready(
        &p,
        &lock(committed_entry(TargetKind::Mailpit)),
    ));
    assert!(
        why.contains("swarmail has no source lock entry")
            && why.contains("mailhog has no source lock entry"),
        "{why}"
    );
}

/// Copy the committed lock's documents into a scratch tree.
fn evals_copy(root: &Path) -> PathBuf {
    let dir = root.join("evals");
    std::fs::create_dir_all(dir.join("sources")).unwrap();
    for t in ["mailpit", "mailhog"] {
        copy_dir(
            &evals_dir().join("sources").join(t),
            &dir.join("sources").join(t),
        );
    }
    dir
}

#[test]
fn captured_documents_are_rechecked_and_a_false_digest_basis_refuses() {
    let root = tempfile::tempdir().unwrap();
    let dir = evals_copy(root.path());
    let committed = SourceLock::parse(SOURCES).unwrap();
    committed.verify_files(&dir).expect("the copy verifies");

    let check = |entry: serde_json::Value| {
        let l = lock_of(vec![entry]);
        l.validate().expect("structurally valid");
        refusals(l.verify_files(&dir))
    };
    // MailHog's release publishes no digest: claiming an upstream basis
    // with the locally computed value is refused against the metadata.
    let mut e = committed_entry(TargetKind::Mailhog);
    e["pin"]["artifact"]["digest_basis"] = "upstream_release_metadata".into();
    e["pin"]["artifact"]["upstream_digest"] = e["pin"]["artifact"]["sha256"].clone();
    assert!(check(e).contains("published digest disagrees"));
    // Mailpit's release does publish one: hiding it is refused too.
    let mut e = committed_entry(TargetKind::Mailpit);
    e["pin"]["artifact"]["digest_basis"] = "locally_computed".into();
    e["pin"]["artifact"]["upstream_digest"] = serde_json::Value::Null;
    assert!(check(e).contains("published digest disagrees"));
    let mut e = committed_entry(TargetKind::Mailpit);
    e["pin"]["artifact"]["size"] = 1.into();
    assert!(check(e).contains("URL or size disagrees"));
    let mut e = committed_entry(TargetKind::Mailpit);
    e["pin"]["release"]["target_commitish"] = "0123456789abcdef0123456789abcdef01234567".into();
    assert!(check(e).contains("target_commitish disagrees"));
    let mut e = committed_entry(TargetKind::Mailpit);
    e["pin"]["artifact"]["file_name"] = "mailpit-linux-arm64.tar.gz".into();
    e["pin"]["artifact"]["download"] =
        "https://github.com/axllent/mailpit/releases/download/v1.32.0/mailpit-linux-arm64.tar.gz"
            .into();
    let why = check(e);
    assert!(
        why.contains("URL or size disagrees") || why.contains("published digest"),
        "{why}"
    );

    // Changed or missing bytes refuse; the lock's digests are not re-derived.
    let swagger = dir.join("sources/mailpit/mailpit-swagger.json");
    let mut bytes = std::fs::read(&swagger).unwrap();
    bytes[10] ^= 1;
    std::fs::write(&swagger, &bytes).unwrap();
    std::fs::remove_file(dir.join("sources/mailhog/mailhog-license.txt")).unwrap();
    let why = refusals(committed.verify_files(&dir));
    assert!(
        why.contains("mailpit-swagger.json does not match its digest"),
        "{why}"
    );
    assert!(why.contains("mailhog-license.txt is missing"), "{why}");
}

#[test]
fn a_protocol_whose_retention_differs_from_the_launch_refuses() {
    let s = SourceLock::parse(SOURCES).unwrap();
    let mut v = json(PROTOCOL);
    for r in v["storage"]["retention"].as_array_mut().unwrap() {
        r["max_messages"] = serde_json::Value::Null;
    }
    let p = protocol_from(v).unwrap();
    let why = refusals(protocol::retention_matches(&p, &s));
    assert!(
        why.contains("mailpit: protocol retention None is not the launched cap Some(10000)"),
        "{why}"
    );
    assert!(refusals(protocol::collection_ready(&p, &s)).contains("launched cap"));
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

// ---- the Mailpit MCP adapter: protocol and refusal paths ------------------

type Answer = dyn Fn(&str) -> Option<(u16, String)> + Send + Sync;

/// A bounded local HTTP responder, for refusal paths only: `answer` maps a
/// request target to `(status, body)`, or `None` to hold the connection
/// without answering (a hung upstream). Real Mailpit proves the rest.
async fn responder(answer: Box<Answer>) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let answer: std::sync::Arc<Answer> = answer.into();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            let answer = answer.clone();
            tokio::spawn(async move {
                let mut head = Vec::new();
                let mut b = [0u8; 1024];
                while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16384 {
                    match s.read(&mut b).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => head.extend_from_slice(&b[..n]),
                    }
                }
                let target = String::from_utf8_lossy(&head)
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                match answer(&target) {
                    Some((status, body)) => {
                        let resp = format!(
                            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = s.write_all(resp.as_bytes()).await;
                    }
                    None => std::future::pending::<()>().await,
                }
            });
        }
    });
    addr
}

struct Mcp {
    w: Option<tokio::io::WriteHalf<tokio::io::DuplexStream>>,
    r: tokio::io::BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    served: tokio::task::JoinHandle<mailpit_mcp::End>,
}

impl Mcp {
    fn start(addr: std::net::SocketAddr, bounds: mailpit_mcp::Bounds, out_buf: usize) -> Mcp {
        let (client_in, server_in) = tokio::io::duplex(1 << 20);
        let (server_out, client_out) = tokio::io::duplex(out_buf);
        let http = wire::Http::new(addr, bounds.request, bounds.max_body);
        let served = tokio::spawn(mailpit_mcp::serve(server_in, server_out, http, bounds));
        let (_, w) = tokio::io::split(client_in);
        let (r, _) = tokio::io::split(client_out);
        Mcp {
            w: Some(w),
            r: tokio::io::BufReader::new(r),
            served,
        }
    }

    async fn raw(&mut self, line: &[u8]) {
        use tokio::io::AsyncWriteExt;
        let w = self.w.as_mut().unwrap();
        w.write_all(line).await.unwrap();
        w.write_all(b"\n").await.unwrap();
    }

    async fn send(&mut self, v: serde_json::Value) {
        self.raw(v.to_string().as_bytes()).await;
    }

    /// The next message, within a real deadline that fails the test.
    async fn recv(&mut self) -> serde_json::Value {
        use tokio::io::AsyncBufReadExt;
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(30), self.r.read_line(&mut line))
            .await
            .expect("a reply within the deadline")
            .unwrap();
        serde_json::from_str(&line).expect("one JSON object per line")
    }

    async fn request(
        &mut self,
        id: i64,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        self.send(
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        )
        .await;
        let r = self.recv().await;
        assert_eq!(r["id"], id, "{r}");
        r
    }

    async fn init(&mut self) {
        let r = self
            .request(
                0,
                "initialize",
                serde_json::json!({"protocolVersion": "2025-03-26",
                "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}),
            )
            .await;
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26", "{r}");
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .await;
    }

    /// A tool call's `(isError, parsed text)`.
    async fn tool(
        &mut self,
        id: i64,
        name: &str,
        args: serde_json::Value,
    ) -> (bool, serde_json::Value) {
        let r = self
            .request(
                id,
                "tools/call",
                serde_json::json!({"name": name, "arguments": args}),
            )
            .await;
        tool_result(&r)
    }
}

fn tool_result(r: &serde_json::Value) -> (bool, serde_json::Value) {
    let res = &r["result"];
    assert_eq!(res["content"][0]["type"], "text", "{r}");
    let text = res["content"][0]["text"].as_str().unwrap();
    (
        res["isError"].as_bool().unwrap(),
        serde_json::from_str(text).unwrap(),
    )
}

fn small_bounds() -> mailpit_mcp::Bounds {
    mailpit_mcp::Bounds {
        max_line: 1024,
        request: Duration::from_secs(5),
        max_body: 4096,
        max_operation_bytes: 16 << 10,
        cleanup: Duration::from_millis(500),
        poll_interval: Duration::from_millis(50),
        ..mailpit_mcp::Bounds::default()
    }
}

fn page_json(matching: u64, start: u64, ids: &[&str]) -> String {
    let msgs: Vec<_> = ids
        .iter()
        .map(|i| serde_json::json!({"ID": i, "Subject": format!("s {i}")}))
        .collect();
    serde_json::json!({"total": 99, "unread": 0, "count": ids.len(), "messages_count": matching,
        "messages_unread": 0, "start": start, "tags": [], "messages": msgs})
    .to_string()
}

#[test]
fn the_adapter_endpoint_is_a_literal_loopback_socket() {
    for ok in ["http://127.0.0.1:8025", "http://[::1]:8025"] {
        assert!(mailpit_mcp::parse_endpoint(ok).is_ok(), "{ok}");
    }
    for bad in [
        "https://127.0.0.1:8025",
        "http://localhost:8025",
        "http://10.0.0.1:8025",
        "http://127.0.0.1:8025/",
        "http://127.0.0.1:8025/api",
        "http://u:p@127.0.0.1:8025",
        "http://127.0.0.1:0",
        "127.0.0.1:8025",
    ] {
        assert!(mailpit_mcp::parse_endpoint(bad).is_err(), "{bad}");
    }
}

#[tokio::test]
async fn the_adapter_speaks_closed_bounded_jsonrpc_and_refuses_safely() {
    let addr = responder(Box::new(|_| Some((200, page_json(0, 0, &[]))))).await;
    let bounds = mailpit_mcp::Bounds {
        max_line: 8192,
        ..small_bounds()
    };
    let mut c = Mcp::start(addr, bounds, 1 << 20);
    let r = c.request(1, "tools/list", serde_json::json!({})).await;
    assert_eq!(r["error"]["code"], -32600, "tools before initialize: {r}");
    let r = c.request(2, "ping", serde_json::json!({})).await;
    assert_eq!(r["result"], serde_json::json!({}));
    let r = c
        .request(
            3,
            "initialize",
            serde_json::json!({"protocolVersion": "2024-11-05"}),
        )
        .await;
    assert_eq!(r["error"]["code"], -32602, "{r}");
    assert_eq!(
        r["error"]["data"]["supported"],
        serde_json::json!(["2025-03-26"])
    );
    c.raw(b"{not json").await;
    let r = c.recv().await;
    assert_eq!(
        (r["id"].clone(), r["error"]["code"].clone()),
        (serde_json::Value::Null, (-32700).into())
    );
    c.raw(b"[]").await;
    assert_eq!(c.recv().await["error"]["code"], -32600, "batches refuse");
    c.raw(&[b'x'; 9000]).await;
    let r = c.recv().await;
    assert_eq!(r["error"]["code"], -32600, "an overlong line refuses: {r}");
    assert!(r["id"].is_null());
    c.send(serde_json::json!({"jsonrpc": "2.0", "id": null, "method": "ping"}))
        .await;
    assert_eq!(c.recv().await["error"]["code"], -32600, "a null id refuses");
    c.send(serde_json::json!({"jsonrpc": "1.0", "id": 4, "method": "ping"}))
        .await;
    assert_eq!(c.recv().await["error"]["code"], -32600);
    c.init().await;
    let r = c
        .request(
            5,
            "initialize",
            serde_json::json!({"protocolVersion": "2025-03-26"}),
        )
        .await;
    assert_eq!(r["error"]["code"], -32600, "initialize twice: {r}");
    let r = c.request(6, "resources/list", serde_json::json!({})).await;
    assert_eq!(r["error"]["code"], -32601);
    // Notifications and client responses never get a reply: the next line
    // read is the ping's.
    c.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/progress"}))
        .await;
    c.send(serde_json::json!({"jsonrpc": "2.0", "id": 77, "result": {}}))
        .await;
    let r = c.request(7, "ping", serde_json::json!({})).await;
    assert!(r.get("result").is_some());

    let r = c.request(8, "tools/list", serde_json::json!({})).await;
    let tools = r["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "mailpit_list",
            "mailpit_search",
            "mailpit_read",
            "mailpit_wait"
        ],
        "general list/search/read/wait only: no admin, send, delete, release, tag, chaos, extraction or inbox tools"
    );
    for t in tools {
        assert_eq!(t["inputSchema"]["additionalProperties"], false, "{t}");
    }
    assert!(
        tools[2]["description"]
            .as_str()
            .unwrap()
            .contains("marks the message read")
    );
    assert!(
        tools[3]["description"]
            .as_str()
            .unwrap()
            .contains("not a Mailpit wait facility")
    );

    let secret = "zzz-private-input-zzz";
    let bad: Vec<serde_json::Value> = vec![
        serde_json::json!({"name": "mailpit_list", "arguments": {secret: 1}}),
        serde_json::json!({"name": "mailpit_list", "arguments": {"limit": 0}}),
        serde_json::json!({"name": "mailpit_list", "arguments": {"limit": 251}}),
        serde_json::json!({"name": "mailpit_list", "arguments": {"limit": "5"}}),
        serde_json::json!({"name": "mailpit_list", "arguments": {"start": 100_001}}),
        serde_json::json!({"name": "mailpit_list", "arguments": {"start": -1}}),
        serde_json::json!({"name": "mailpit_search", "arguments": {"query": ""}}),
        serde_json::json!({"name": "mailpit_search", "arguments": {"query": "q".repeat(4097)}}),
        serde_json::json!({"name": "mailpit_search", "arguments": {}}),
        serde_json::json!({"name": "mailpit_read", "arguments": {"id": "i".repeat(513)}}),
        serde_json::json!({"name": "mailpit_wait", "arguments": {"query": "q", "timeout_ms": 30_001}}),
        serde_json::json!({"name": "mailpit_wait", "arguments": {"query": "q"}}),
        serde_json::json!({"name": "mailpit_delete", "arguments": {}}),
        serde_json::json!({"name": "mailpit_list", "arguments": [], "extra": secret}),
    ];
    for (i, params) in bad.into_iter().enumerate() {
        let r = c
            .request(100 + i as i64, "tools/call", params.clone())
            .await;
        assert_eq!(r["error"]["code"], -32602, "{params}: {r}");
        assert!(!r.to_string().contains(secret), "input echoed: {r}");
    }
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
}

#[tokio::test]
async fn the_adapter_refuses_incomplete_malformed_and_overbudget_upstream_answers() {
    let addr = responder(Box::new(|t: &str| {
        let q = |s: &str| t.contains(&format!("query={s}&"));
        let at = |s: u64| t.contains(&format!("&start={s}&"));
        Some(match t {
            _ if t.starts_with("/api/v1/messages?") => (200, "{".into()),
            _ if q("dupq7") && at(0) => (200, page_json(3, 0, &["a", "b"])),
            _ if q("dupq7") => (200, page_json(3, 250, &["a"])),
            _ if q("shortq7") && at(0) => (200, page_json(5, 0, &["a", "b"])),
            _ if q("shortq7") => (200, page_json(5, 250, &[])),
            _ if q("movingq7") && at(0) => (200, page_json(3, 0, &["a"])),
            _ if q("movingq7") => (200, page_json(4, 250, &["b"])),
            _ if q("echoq7") => (200, page_json(1, 7, &["a"])),
            _ if q("countq7") => (
                200,
                page_json(1, 0, &["a"]).replace("\"count\":1", "\"count\":2"),
            ),
            _ if q("bigq7") => (200, page_json(1, 0, &[&"x".repeat(5000)])),
            _ if q("neverq7") => (200, page_json(0, 0, &[])),
            _ if t.ends_with("/message/goneq7") => (404, "404 page not found".into()),
            _ if t.ends_with("/message/otherq7") => (200, r#"{"ID":"different"}"#.into()),
            _ if t.ends_with("/message/boomq7") => (500, "{}".into()),
            _ => (200, page_json(0, 0, &[])),
        })
    }))
    .await;
    let mut c = Mcp::start(addr, small_bounds(), 1 << 20);
    c.init().await;
    let mut id = 10;
    let mut expect = async |c: &mut Mcp, name: &str, args: serde_json::Value, want: &str| {
        id += 1;
        let (is_error, v) = c.tool(id, name, args.clone()).await;
        assert!(is_error, "{name} {args}: {v}");
        let text = v["error"].as_str().unwrap();
        assert!(
            text.starts_with(want),
            "{name} {args}: expected {want}, got {text}"
        );
        assert!(!text.contains("q7"), "input echoed in {text}");
    };
    expect(&mut c, "mailpit_list", serde_json::json!({}), "malformed").await;
    for (query, want) in [
        ("dupq7", "incomplete"),
        ("shortq7", "incomplete"),
        ("movingq7", "incomplete"),
        ("echoq7", "malformed"),
        ("countq7", "malformed"),
        ("bigq7", "budget_exhausted: response bytes"),
    ] {
        expect(
            &mut c,
            "mailpit_wait",
            serde_json::json!({"query": query, "timeout_ms": 1000}),
            want,
        )
        .await;
    }
    expect(
        &mut c,
        "mailpit_search",
        serde_json::json!({"query": "echoq7"}),
        "malformed",
    )
    .await;
    expect(
        &mut c,
        "mailpit_read",
        serde_json::json!({"id": "goneq7"}),
        "not_found",
    )
    .await;
    expect(
        &mut c,
        "mailpit_read",
        serde_json::json!({"id": "otherq7"}),
        "malformed",
    )
    .await;
    expect(
        &mut c,
        "mailpit_read",
        serde_json::json!({"id": "boomq7"}),
        "upstream_status: Mailpit answered HTTP 500",
    )
    .await;

    // An empty wait answers timeout only after complete scans, and counts
    // every request it made.
    let (is_error, v) = c
        .tool(
            50,
            "mailpit_wait",
            serde_json::json!({"query": "neverq7", "timeout_ms": 300}),
        )
        .await;
    assert!(!is_error, "{v}");
    assert_eq!(v["status"], "timeout");
    assert_eq!(v["messages"], serde_json::json!([]));
    let scans = v["scans"].as_u64().unwrap();
    assert!(scans >= 1, "{v}");
    assert_eq!(
        v["requests"].as_u64().unwrap(),
        scans,
        "one page per empty scan"
    );
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);

    // Aggregate request and byte budgets end a long wait with an error.
    let tight = mailpit_mcp::Bounds {
        max_requests: 3,
        ..small_bounds()
    };
    let mut c = Mcp::start(addr, tight, 1 << 20);
    c.init().await;
    let (is_error, v) = c
        .tool(
            1,
            "mailpit_wait",
            serde_json::json!({"query": "neverq7", "timeout_ms": 5000}),
        )
        .await;
    assert!(
        is_error && v["error"] == "budget_exhausted: request count",
        "{v}"
    );
    let bytes = mailpit_mcp::Bounds {
        max_operation_bytes: 600,
        ..small_bounds()
    };
    let mut c = Mcp::start(addr, bytes, 1 << 20);
    c.init().await;
    let (is_error, v) = c
        .tool(
            1,
            "mailpit_wait",
            serde_json::json!({"query": "neverq7", "timeout_ms": 5000}),
        )
        .await;
    assert!(
        is_error && v["error"] == "budget_exhausted: operation bytes",
        "{v}"
    );
}

#[tokio::test]
async fn cancellation_eof_queue_and_output_bounds_hold() {
    let addr = responder(Box::new(|t: &str| {
        if t.contains("query=hang") {
            None
        } else if t.contains("query=wide") {
            Some((200, page_json(1, 0, &[&"w".repeat(1500)])))
        } else {
            Some((200, page_json(0, 0, &[])))
        }
    }))
    .await;
    let mut c = Mcp::start(addr, small_bounds(), 1 << 20);
    c.init().await;
    let hang = |id: i64| {
        serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "mailpit_search", "arguments": {"query": "hang"}}})
    };
    // One runs, seven queue, the ninth is refused at once; the reader stays
    // responsive while the first is stuck upstream.
    for id in 1..=9 {
        c.send(hang(id)).await;
    }
    let r = c.recv().await;
    assert_eq!(
        (r["id"].clone(), r["error"]["code"].clone()),
        (9.into(), (-32000).into()),
        "{r}"
    );
    let r = c.request(20, "ping", serde_json::json!({})).await;
    assert!(r.get("result").is_some());
    c.send(hang(1)).await;
    let r = c.recv().await;
    assert_eq!(
        (r["id"].clone(), r["error"]["code"].clone()),
        (1.into(), (-32600).into()),
        "a duplicate in-flight id refuses: {r}"
    );
    // Cancel the running call and every queued one: none is ever answered.
    for id in 1..=8 {
        c.send(
            serde_json::json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
            "params": {"requestId": id, "reason": "test"}}),
        )
        .await;
    }
    let (is_error, v) = c.tool(30, "mailpit_list", serde_json::json!({})).await;
    assert!(!is_error, "{v}");
    assert_eq!(v["count"], 0);
    // A reply beyond the reply bound is a finite error, never truncated.
    let mut wide = Mcp::start(
        addr,
        mailpit_mcp::Bounds {
            max_reply: 2048,
            ..small_bounds()
        },
        1 << 20,
    );
    wide.init().await;
    let (is_error, v) = wide
        .tool(1, "mailpit_search", serde_json::json!({"query": "wide"}))
        .await;
    assert!(
        is_error && v["error"].as_str().unwrap().starts_with("reply_too_large"),
        "{v}"
    );

    // EOF while a call is stuck upstream: the adapter cancels it and ends.
    c.send(hang(40)).await;
    let r = c.request(41, "ping", serde_json::json!({})).await;
    assert!(r.get("result").is_some());
    drop(c.w.take());
    let end = tokio::time::timeout(Duration::from_secs(10), c.served)
        .await
        .expect("EOF ends serving within the cleanup bound")
        .unwrap();
    assert_eq!(end, mailpit_mcp::End::Eof);

    // An output nobody reads: serving ends within the cleanup bound.
    let mut blocked = Mcp::start(addr, small_bounds(), 64);
    for id in 0..64 {
        blocked
            .send(serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "ping"}))
            .await;
    }
    let end = tokio::time::timeout(Duration::from_secs(10), blocked.served)
        .await
        .expect("a blocked output ends serving")
        .unwrap();
    assert_eq!(end, mailpit_mcp::End::OutputBlocked);
}

// ---- real pinned competitors -----------------------------------------------
//
// These need the pinned Mailpit and MailHog executables, which CI does not
// have, so they are ignored by default. Run them with
// `MAIL_COMPARE_MAILPIT=… MAIL_COMPARE_MAILHOG=… cargo test --test
// mail_compare -- --ignored`; without the variables they fail, never skip.

const THREE: &[u8] = include_bytes!("mail_compare/three-target-smoke.json");

/// The pinned executable named by `MAIL_COMPARE_<TARGET>`, launched with the
/// committed lock's digest and argv.
fn pinned(t: TargetKind) -> targets::LaunchSpec {
    let var = format!("MAIL_COMPARE_{}", t.name().to_uppercase());
    let binary = std::env::var_os(&var)
        .unwrap_or_else(|| panic!("{var} must name the pinned {t} executable"));
    let lock = SourceLock::parse(SOURCES).unwrap();
    let pin = lock.pin(t).unwrap();
    targets::LaunchSpec {
        kind: t,
        binary: binary.into(),
        expected_sha256: Some(pin.artifact.executable_sha256.clone()),
        template: pin.launch.clone(),
    }
}

async fn launch_pinned(t: TargetKind, scratch: &Path) -> targets::Owned {
    let clock = wire::Clock(Instant::now());
    targets::Owned::launch(&pinned(t), scratch, &limits(), &clock)
        .await
        .unwrap_or_else(|e| panic!("{t} did not launch: {e:?}"))
}

async fn smtp_code(r: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>) -> u16 {
    use tokio::io::AsyncBufReadExt;
    loop {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(10), r.read_line(&mut line))
            .await
            .expect("an SMTP reply within the deadline")
            .unwrap();
        if line.as_bytes().get(3) != Some(&b'-') {
            return line[..3].parse().unwrap();
        }
    }
}

/// One message from outside the corpus (its own addresses, so a search
/// for the corpus's words cannot match it), optionally after AUTH PLAIN
/// as `user`; panics unless every step is accepted.
async fn send_as(smtp: std::net::SocketAddr, user: Option<&str>, subject: &str, body: &str) {
    use base64::Engine;
    use tokio::io::AsyncWriteExt;
    let (r, mut w) = tokio::net::TcpStream::connect(smtp)
        .await
        .unwrap()
        .into_split();
    let mut r = tokio::io::BufReader::new(r);
    assert_eq!(smtp_code(&mut r).await, 220);
    let mut steps = vec!["EHLO identity.invalid\r\n".to_owned()];
    if let Some(u) = user {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("\0{u}\0pw"));
        steps.push(format!("AUTH PLAIN {token}\r\n"));
    }
    steps.extend([
        "MAIL FROM:<other@identity.invalid>\r\n".to_owned(),
        "RCPT TO:<inbox@identity.invalid>\r\n".to_owned(),
        "DATA\r\n".to_owned(),
    ]);
    for step in steps {
        w.write_all(step.as_bytes()).await.unwrap();
        let code = smtp_code(&mut r).await;
        assert!((200..400).contains(&code), "{step:?} answered {code}");
    }
    let msg = format!(
        "From: <other@identity.invalid>\r\nTo: <inbox@identity.invalid>\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n.\r\n"
    );
    w.write_all(msg.as_bytes()).await.unwrap();
    assert_eq!(smtp_code(&mut r).await, 250);
    w.write_all(b"QUIT\r\n").await.unwrap();
}

async fn send_authored(owned: &targets::Owned, m: &wire::Message) {
    let clock = wire::Clock(Instant::now());
    let mut s = wire::Smtp::open(owned.smtp, Duration::from_secs(10))
        .await
        .unwrap();
    let rec = s.send(m, &clock).await;
    assert!(
        matches!(rec.disposition, wire::Disposition::Accepted { .. }),
        "{rec:?}"
    );
    s.quit().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs MAIL_COMPARE_MAILPIT and MAIL_COMPARE_MAILHOG (pinned executables)"]
async fn a_wrong_or_missing_competitor_binary_refuses_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let clock = wire::Clock(Instant::now());
    let mut spec = pinned(TargetKind::Mailpit);
    spec.binary = pinned(TargetKind::Mailhog).binary;
    let r = targets::Owned::launch(&spec, &root.path().join("a"), &limits(), &clock).await;
    assert!(matches!(r, Err(targets::LaunchError::DigestMismatch)));
    spec.binary = root.path().join("absent");
    let r = targets::Owned::launch(&spec, &root.path().join("b"), &limits(), &clock).await;
    assert!(matches!(r, Err(targets::LaunchError::Binary(_))));
    assert!(!root.path().join("a").exists() && !root.path().join("b").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs MAIL_COMPARE_MAILPIT (the pinned Mailpit executable)"]
async fn real_mailpit_through_the_adapter_lists_searches_reads_and_waits_faithfully() {
    use protocol::MessageKind;
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_pinned(TargetKind::Mailpit, &root.path().join("s")).await;
    let kinds = [
        MessageKind::Plain,
        MessageKind::Html,
        MessageKind::Multipart,
    ];
    let corpus: Vec<wire::Message> = (0..6)
        .map(|i| wire::authored(163, "adapterfid01", i, &kinds))
        .collect();
    for m in &corpus {
        send_authored(&owned, m).await;
    }
    send_as(owned.smtp, Some("alice"), "identity alice", "for alice").await;
    send_as(owned.smtp, Some("bob"), "identity bob", "for bob").await;
    for i in 0..3 {
        send_as(
            owned.smtp,
            None,
            &format!("unrelated {i}"),
            "nothing to see",
        )
        .await;
    }
    let mut c = Mcp::start(owned.http_addr, mailpit_mcp::Bounds::default(), 1 << 20);
    c.init().await;
    let mut id = 100;
    let mut call = async |c: &mut Mcp, name: &str, args: serde_json::Value| {
        id += 1;
        let (is_error, v) = c.tool(id, name, args.clone()).await;
        assert!(!is_error, "{name} {args}: {v}");
        v
    };
    // Accept is not presumed to be storage: a finite rescan budget waits
    // for all eleven to be listed before anything is asserted.
    let until = Instant::now() + Duration::from_secs(10);
    while call(&mut c, "mailpit_list", serde_json::json!({"limit": 1})).await["total"] != 11 {
        assert!(Instant::now() < until, "Mailpit never listed all eleven");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Pagination: newest first, distinct ids, pages that cover the mailbox.
    let mut ids = Vec::new();
    for start in [0, 4, 8] {
        let v = call(
            &mut c,
            "mailpit_list",
            serde_json::json!({"start": start, "limit": 4}),
        )
        .await;
        assert_eq!(
            (
                v["total"].clone(),
                v["messages_count"].clone(),
                v["start"].clone()
            ),
            (11.into(), 11.into(), start.into())
        );
        assert_eq!(v["count"], if start == 8 { 3 } else { 4 });
        for m in v["messages"].as_array().unwrap() {
            ids.push(m["ID"].as_str().unwrap().to_owned());
        }
        if start == 0 {
            assert_eq!(v["messages"][0]["Subject"], "unrelated 2", "newest first");
        }
    }
    let distinct: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!((ids.len(), distinct.len()), (11, 11));

    // Search: the match count is not the mailbox total.
    let v = call(
        &mut c,
        "mailpit_search",
        serde_json::json!({"query": corpus[1].token}),
    )
    .await;
    assert_eq!(
        (
            v["messages_count"].clone(),
            v["total"].clone(),
            v["count"].clone()
        ),
        (1.into(), 11.into(), 1.into())
    );
    assert_eq!(v["messages"][0]["Subject"], corpus[1].subject.as_str());
    let p1 = call(
        &mut c,
        "mailpit_search",
        serde_json::json!({"query": "mail-compare", "limit": 2}),
    )
    .await;
    let p2 = call(
        &mut c,
        "mailpit_search",
        serde_json::json!({"query": "mail-compare", "start": 2, "limit": 2}),
    )
    .await;
    assert_eq!(
        (p1["messages_count"].clone(), p1["count"].clone()),
        (6.into(), 2.into())
    );
    assert_ne!(p1["messages"][0]["ID"], p2["messages"][0]["ID"]);
    let unread = call(
        &mut c,
        "mailpit_search",
        serde_json::json!({"query": "is:unread"}),
    )
    .await;
    assert_eq!(unread["messages_count"], 11);

    // Read: ordinary bodies as Mailpit serves them, and the read side
    // effect. For an HTML-only message Mailpit serves a Text it derived
    // from the HTML; the adapter passes it through rather than hides it.
    for m in &corpus {
        let hit = call(
            &mut c,
            "mailpit_search",
            serde_json::json!({"query": m.token}),
        )
        .await;
        let mid = hit["messages"][0]["ID"].as_str().unwrap().to_owned();
        let v = call(&mut c, "mailpit_read", serde_json::json!({"id": mid})).await;
        assert_eq!(v["Subject"], m.subject.as_str());
        let html = v["HTML"].as_str().filter(|h| !h.is_empty());
        assert!(wire::same_body(m.html.as_deref(), html), "{m:?} {v}");
        match m.kind {
            MessageKind::Html => {
                let derived = v["Text"].as_str().unwrap();
                assert!(
                    derived.contains(&m.token) && !derived.contains("<p>"),
                    "derived text: {derived:?}"
                );
            }
            _ => assert!(
                wire::same_body(m.text.as_deref(), v["Text"].as_str()),
                "{m:?} {v}"
            ),
        }
    }
    let unread = call(
        &mut c,
        "mailpit_search",
        serde_json::json!({"query": "is:unread"}),
    )
    .await;
    assert_eq!(
        unread["messages_count"], 5,
        "reading marked six messages read"
    );

    // Identity: Mailpit records the AUTH user but keeps one mailbox, so
    // both identities' mail is listed together. Isolation is unsupported.
    let all = call(&mut c, "mailpit_list", serde_json::json!({"limit": 50})).await;
    let users: std::collections::BTreeSet<&str> = all["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["Username"].as_str().filter(|u| !u.is_empty()))
        .collect();
    assert_eq!(users, ["alice", "bob"].into());

    let (is_error, v) = c
        .tool(1, "mailpit_read", serde_json::json!({"id": "no-such-id"}))
        .await;
    assert!(
        is_error && v["error"].as_str().unwrap().starts_with("not_found"),
        "{v}"
    );

    // Waits: present, future and empty, each answered after complete scans.
    let v = call(
        &mut c,
        "mailpit_wait",
        serde_json::json!({"query": corpus[2].token, "timeout_ms": 5000}),
    )
    .await;
    assert_eq!(
        (
            v["status"].clone(),
            v["messages_count"].clone(),
            v["scans"].clone()
        ),
        ("found".into(), 1.into(), 1.into())
    );
    let future = wire::authored(163, "adapterfid01", 100, &kinds);
    c.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "mailpit_wait", "arguments": {"query": future.token, "timeout_ms": 10000}}}))
        .await;
    send_authored(&owned, &future).await;
    let r = c.recv().await;
    assert_eq!(r["id"], 2);
    let (is_error, v) = tool_result(&r);
    assert!(!is_error, "{v}");
    assert_eq!(v["status"], "found");
    assert_eq!(v["messages"][0]["Subject"], future.subject.as_str());
    let never = wire::authored(163, "adapterfid01", 200, &kinds);
    let v = call(
        &mut c,
        "mailpit_wait",
        serde_json::json!({"query": never.token, "timeout_ms": 300}),
    )
    .await;
    assert_eq!(
        (v["status"].clone(), v["messages_count"].clone()),
        ("timeout".into(), 0.into())
    );
    assert!(v["scans"].as_u64().unwrap() >= 1);

    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    let facts = owned.shutdown().await;
    assert!(facts.proved() && !facts.forced_kill, "{facts:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs MAIL_COMPARE_MAILPIT and MAIL_COMPARE_MAILHOG (pinned executables)"]
async fn real_competitors_decode_complete_scans_searches_and_stored_bodies() {
    use protocol::MessageKind;
    let kinds = [
        MessageKind::Plain,
        MessageKind::Html,
        MessageKind::Multipart,
    ];
    for t in [TargetKind::Mailpit, TargetKind::Mailhog] {
        let root = tempfile::tempdir().unwrap();
        let mut owned = launch_pinned(t, &root.path().join("s")).await;
        // More than one 250-message page, plus mail that matches nothing.
        let corpus: Vec<wire::Message> = (0..260)
            .map(|i| wire::authored(163, "decodescan01", i, &kinds))
            .collect();
        for m in &corpus {
            send_authored(&owned, m).await;
        }
        for i in 0..2 {
            send_as(owned.smtp, None, &format!("unrelated {i}"), "nothing").await;
        }
        let until = Instant::now() + Duration::from_secs(30);
        let listing = loop {
            match owned.list_all(8).await {
                Ok(l) if l.len() == 262 => break l,
                other => {
                    let seen = other.map(|l| l.len());
                    assert!(Instant::now() < until, "{t}: never listed 262: {seen:?}");
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            }
        };
        let ids: std::collections::BTreeSet<_> = listing.iter().map(|(_, id)| id).collect();
        assert_eq!(ids.len(), 262, "{t}: distinct ids");
        let tokens: std::collections::BTreeSet<_> = listing
            .iter()
            .map(|(tok, _)| tok.as_str())
            .filter(|t| !t.is_empty())
            .collect();
        assert_eq!(tokens.len(), 260, "{t}: every authored token once");
        for i in [0usize, 1, 2, 259] {
            let m = &corpus[i];
            assert_eq!(
                owned.find(&m.token).await.unwrap(),
                vec![m.token.clone()],
                "{t} find {i}"
            );
            let id = &listing.iter().find(|(tok, _)| *tok == m.token).unwrap().1;
            let f = owned.fetch(id).await.unwrap();
            assert!(runner::fidelity(m, &f), "{t} {:?}: {f:?}", m.kind);
        }
        let absent = wire::authored(163, "decodescan01", 999_999, &kinds).token;
        assert_eq!(
            owned.find(&absent).await.unwrap(),
            Vec::<String>::new(),
            "{t}: a complete empty search"
        );
        assert!(
            owned.fetch("no-such-id").await.is_err(),
            "{t}: an unknown id is no message"
        );
        let facts = owned.shutdown().await;
        assert!(facts.proved() && !facts.forced_kill, "{t}: {facts:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs MAIL_COMPARE_MAILPIT and MAIL_COMPARE_MAILHOG (pinned executables)"]
async fn a_real_three_target_smoke_run_reconciles_every_message_and_wait_and_seals() {
    let p = Protocol::parse(THREE).unwrap();
    p.validate().unwrap();
    let lock = SourceLock::parse(SOURCES).unwrap();
    protocol::retention_matches(&p, &lock).unwrap();
    let root = tempfile::tempdir().unwrap();
    let cfg = runner::Config {
        manifest: manifest(&p, "three-target-smoke"),
        specs: [
            (
                TargetKind::Swarmail,
                swarmail_spec(targets::swarmail_template(0)),
            ),
            (TargetKind::Mailpit, pinned(TargetKind::Mailpit)),
            (TargetKind::Mailhog, pinned(TargetKind::Mailhog)),
        ]
        .into(),
        output: root.path().join("out"),
        scratch: root.path().join("s"),
        protocol: p.clone(),
    };
    std::fs::create_dir(root.path().join("s")).unwrap();
    let (verdict, life, sealed) = runner::run(cfg, wire::Clock(Instant::now())).await;
    assert_eq!(verdict, runner::Verdict::Clean, "{verdict:?}");
    assert!(life.cleanups.iter().all(targets::CleanupFacts::proved));
    let launched: std::collections::BTreeSet<_> = life.launches.iter().map(|l| l.target).collect();
    assert_eq!(launched, TargetKind::ALL.into());
    let dir = sealed.expect("a clean run is sealed");
    records::admit(&dir).expect("the sealed run is admitted");
    let rows = rows(&dir);
    for t in TargetKind::ALL {
        let mine = |phase: Phase| {
            rows.iter()
                .filter(move |r| r.target == t && r.phase == phase)
        };
        let sends: Vec<_> = mine(Phase::Send).collect();
        assert_eq!(sends.len(), 312, "{t}: one row per planned send");
        assert!(sends.iter().all(|r| r.outcome == "accepted"), "{t}");
        let visible: Vec<_> = mine(Phase::Visible).collect();
        assert_eq!(
            visible.len(),
            312,
            "{t}: every accepted message visible once"
        );
        assert!(
            visible.iter().all(|r| r.outcome == "visible"
                && r.occurrences == Some(1)
                && r.fidelity_ok == Some(true)),
            "{t}"
        );
        assert!(
            mine(Phase::Present)
                .chain(mine(Phase::Future))
                .all(|r| r.outcome == "observed"),
            "{t}"
        );
        assert!(
            mine(Phase::Empty).all(|r| r.outcome == "timeout" && r.observed_ns.is_none()),
            "{t}"
        );
    }
}
