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
#[path = "../benches/mail_compare/mailhog_mcp.rs"]
mod mailhog_mcp;
#[allow(dead_code)]
#[path = "../benches/mail_compare/mailpit_mcp.rs"]
mod mailpit_mcp;
#[allow(dead_code)]
#[path = "../benches/mail_compare/mailslurper_mcp.rs"]
mod mailslurper_mcp;
#[allow(dead_code)]
#[path = "../benches/mail_compare/mcp.rs"]
mod mcp;
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

#[path = "mail_compare/eight_targets.rs"]
mod eight_targets;

const SMOKE: &[u8] = include_bytes!("mail_compare/swarmail-smoke.json");
const SMOKE_LOCK: &[u8] = include_bytes!("mail_compare/swarmail-only.lock.json");
/// Protocol 2 and lock 3 as they were collected under on 2026-10-30; their
/// bytes live under `docs/evals/history/` and the existing tests keep
/// proving them (proposed decision 0130 keeps both readable and launchable).
const PROTOCOL: &[u8] = include_bytes!("../docs/evals/history/2026-10-30-protocol.v2.json");
const SOURCES: &[u8] = include_bytes!("../docs/evals/history/2026-10-30-sources.lock.v3.json");
/// The current records: protocol 3 and lock 4.
const PROTOCOL_V3: &[u8] = include_bytes!("../docs/evals/protocol.json");
const SOURCES_V4: &[u8] = include_bytes!("../docs/evals/sources.lock.json");

/// Characters beyond `[A-Za-z0-9._-]` that MailHog's own ids carry in the wire
/// corpus: `=` and `@` (`<base64url, padded>@mailhog.example`). Only the
/// corpus check uses it: the adapter offers no tool that takes an id.
const MAILHOG_CORPUS_ID_EXTRA: &[u8] = b"=@";

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
    targets::LaunchSpec::new(
        TargetKind::Swarmail,
        PathBuf::from(env!("CARGO_BIN_EXE_swarmail")),
        None,
        template,
    )
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
        keepers: targets::Keepers::default(),
    };
    std::fs::create_dir(root.join("s")).unwrap();
    runner::run(cfg, wire::Clock(Instant::now())).await
}

fn rows(dir: &Path) -> Vec<Row> {
    let m = records::read_manifest(dir).unwrap();
    records::read_rows(dir, &m).unwrap()
}

/// Recompute every throughput cell's accounting from the raw rows alone,
/// independently of report.rs, and compare it with the report: the two
/// sums `requested = attempted + unstarted` and `attempted = accepted +
/// rejected + failed_before_terminator + ambiguous`, with each disposition
/// exclusive and `unstarted` never merged with a connect failure.
fn assert_cells_match_rows(dir: &Path) {
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    let all = rows(dir);
    let cells = report["throughput"].as_array().unwrap();
    assert!(!cells.is_empty());
    for cell in cells {
        let mine: Vec<&Row> = all
            .iter()
            .filter(|r| {
                r.phase == Phase::Send
                    && serde_json::json!(r.target) == cell["target"]
                    && r.profile == cell["profile"].as_str().unwrap()
                    && u64::from(r.repetition) == cell["repetition"].as_u64().unwrap()
            })
            .collect();
        let count = |o: &str| mine.iter().filter(|r| r.outcome == o).count() as u64;
        let (accepted, rejected, failed, unstarted) = (
            count("accepted"),
            count("rejected"),
            count("failed_before_terminator"),
            count("unstarted"),
        );
        let ambiguous = mine.len() as u64 - accepted - rejected - failed - unstarted;
        let attempted = accepted + rejected + failed + ambiguous;
        let rec = &cell["reconciliation"];
        let reported = |v: &serde_json::Value| v.as_u64().unwrap();
        assert_eq!(
            reported(&cell["requested"]),
            attempted + unstarted,
            "{cell}"
        );
        assert_eq!(reported(&cell["requested"]), mine.len() as u64, "{cell}");
        assert_eq!(reported(&cell["unstarted"]), unstarted, "{cell}");
        assert_eq!(reported(&rec["attempted"]), attempted, "{cell}");
        assert_eq!(reported(&rec["accepted"]), accepted, "{cell}");
        assert_eq!(reported(&rec["rejected"]), rejected, "{cell}");
        assert_eq!(reported(&rec["failed_before_terminator"]), failed, "{cell}");
        assert_eq!(reported(&rec["ambiguous"]), ambiguous, "{cell}");
        assert_eq!(reported(&cell["admitted"]), accepted, "{cell}");
        assert_eq!(reported(&cell["refused"]), rejected + failed, "{cell}");
        let session_cap = cell["sessions"].as_u64().unwrap();
        assert!(
            session_cap >= 1,
            "the scheduled sessions are reported: {cell}"
        );
    }
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

#[test]
fn late_observations_are_retained_but_censored_like_timeouts() {
    let p = smoke();
    let m = manifest(&p, "late");
    let wait = |phase, sample, outcome: &str, observed: Option<u64>| {
        let mut r = runner::row("late", TargetKind::Swarmail, "w", 0, phase, sample, outcome);
        (r.invoked_ns, r.start_ns, r.ack_ns, r.observed_ns) =
            (Some(100), Some(100), Some(120), observed);
        r
    };
    let rows = vec![
        wait(Phase::Present, 0, "observed", Some(150)),
        wait(Phase::Present, 1, "late", Some(9_000)),
        wait(Phase::Present, 2, "timeout", None),
        wait(Phase::Future, 0, "late", Some(9_500)),
        wait(Phase::Future, 1, "observed", Some(110)),
    ];
    let life = runner::Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let r = report::report(&p, &m, &rows, &life);
    let w = &r.waits[0];
    assert_eq!(
        (w.present_latency.samples, w.present_latency.censored),
        (1, 2)
    );
    assert_eq!(
        (w.future_latency.samples, w.future_latency.censored),
        (1, 1)
    );
    // The late observation's offset is not folded into the quantiles,
    // and the signed offset of the on-time one stays negative.
    assert_eq!(w.future_offset_from_ack.values[0].1, Some(-10));
    assert_eq!(w.outcomes["present"]["late"], 1);
    assert_eq!(w.outcomes["future"]["late"], 1);
}

// ---- the frozen method and the source lock ------------------------------

#[test]
fn the_committed_protocol_is_frozen_and_collects_only_its_declared_limited_claim() {
    let p = Protocol::parse(PROTOCOL).expect("the committed protocol parses");
    // Its predecessors' bytes are kept and still read as what they were:
    // version 1 drafts that carry none of the version 2 sections.
    let draft2 = include_bytes!("../docs/evals/history/2026-10-28-protocol.draft-2.json");
    let old = Protocol::parse(draft2).expect("draft-2 still parses");
    old.validate().expect("draft-2 is still structurally valid");
    let draft3 = include_bytes!("../docs/evals/history/2026-10-29-protocol.draft-3.json");
    let mid = Protocol::parse(draft3).expect("draft-3 still parses");
    mid.validate().expect("draft-3 is still structurally valid");
    assert_eq!(mid.supersedes.as_deref(), Some(old.id.as_str()));
    assert_eq!(p.supersedes.as_deref(), Some(mid.id.as_str()));
    assert_eq!(
        (
            old.protocol_version,
            mid.protocol_version,
            p.protocol_version
        ),
        (1, 1, 2)
    );
    assert!(p.change_reason.is_some());
    let s = SourceLock::parse(SOURCES).expect("the committed lock parses");
    s.validate()
        .expect("the committed lock is structurally valid, its gaps named");
    s.verify_files(&evals_dir())
        .expect("every captured document matches its digest and the release metadata");
    protocol::retention_matches(&p, &s).expect("retention is stated as launched");
    protocol::products_match(&p, &s).expect("the products are the locked executables");
    p.validate().expect("the committed protocol is valid");
    protocol::collection_ready(&p, &s).expect("the frozen protocol admits collection");
    // What it admits is limited, and says so in the protocol itself.
    assert_eq!(p.status, protocol::Status::Frozen);
    assert!(p.unresolved.is_empty());
    let claim = p.claim.as_ref().unwrap();
    assert_eq!(claim.scope, protocol::ClaimScope::OfficialArtifact);
    assert!(claim.statement.contains("no built-from"), "{claim:?}");
    assert!(
        s.pin(TargetKind::Mailpit)
            .unwrap()
            .binary_build_provenance
            .gap()
            .is_some()
    );
    assert!(
        s.pin(TargetKind::Mailhog)
            .unwrap()
            .binary_build_provenance
            .gap()
            .is_some()
    );
    assert_eq!(p.storage.profile, Some(protocol::StorageProfile::Unmatched));
    assert_eq!(
        p.resources.profile,
        Some(protocol::ResourceProfile::UnenforcedLocal)
    );
    assert!(!p.resources.enforced);
    // Best wait stays blocked: it is not a declared profile, and the lock
    // still names why.
    assert!(p.waits.iter().all(|w| w.name != protocol::BEST_WAIT));
    for t in [TargetKind::Mailpit, TargetKind::Mailhog] {
        assert!(s.pin(t).unwrap().wait.best_wait.gap().is_some(), "{t}");
    }
    assert_eq!(p.targets, TargetKind::V2.to_vec());
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
    // Swarmail is launched with its shipped cap and oldest-only prune.
    let swarmail = &p.storage.retention[0];
    assert_eq!(swarmail.target, TargetKind::Swarmail);
    assert_eq!(
        (swarmail.max_messages, swarmail.prune),
        (Some(100_000), Some(protocol::Prune::OldestOnly))
    );
}

/// The committed protocol with one JSON edit applied.
fn edited(f: impl Fn(&mut serde_json::Value)) -> Protocol {
    let mut v = json(PROTOCOL);
    f(&mut v);
    protocol_from(v).expect("the edit keeps the shape")
}

#[test]
fn a_changed_method_schema_product_or_parity_never_validates_or_collects() {
    let s = SourceLock::parse(SOURCES).unwrap();
    let invalid = |p: Protocol| refusals(p.validate());
    // Method: a poll at another interval is not the common method, and a
    // wait method or operation bound that is not the built one refuses.
    let why = invalid(edited(|v| {
        v["waits"][0]["observation"][1]["interval_ms"] = 250.into();
    }));
    assert!(why.contains("not the common 100 ms"), "{why}");
    let why = invalid(edited(|v| {
        v["budgets"]["wait_operation_ms"] = 30_000.into();
    }));
    assert!(
        why.contains("longest wait 30000 plus one request bound"),
        "{why}"
    );
    let why = invalid(edited(|v| {
        v["budgets"]["adapter"]["wait_method"] = "websocket subscription".into();
    }));
    assert!(why.contains("adapter wait method"), "{why}");
    let why = invalid(edited(|v| {
        v["budgets"]["adapter"]["max_wait_ms"] = 30_000.into();
    }));
    assert!(why.contains("not what the adapter is built with"), "{why}");
    let why = invalid(edited(|v| {
        v["budgets"]["adapter"]["max_pages"] = 101.into();
    }));
    assert!(why.contains("page, request and byte bounds"), "{why}");
    // Schema: the adapter's served tools are bound by digest.
    let why = invalid(edited(|v| {
        v["budgets"]["adapter"]["tools_sha256"] = "0".repeat(64).into();
    }));
    assert!(why.contains("not the schema the adapter serves"), "{why}");
    // Product: a missing, extra, mutable or unverifiable identity refuses.
    let why = invalid(edited(|v| {
        v["products"][1]["release_tag"] = "latest".into();
    }));
    assert!(why.contains("not an immutable version tag"), "{why}");
    let why = invalid(edited(|v| {
        v["products"][2]["executable_sha256"] = "abc".into();
    }));
    assert!(why.contains("not 64 hex digits"), "{why}");
    let why = invalid(edited(|v| {
        v["products"].as_array_mut().unwrap().pop();
    }));
    assert!(
        why.contains("mailhog needs exactly one product entry"),
        "{why}"
    );
    let why = invalid(edited(|v| {
        v["products"][0]["release_tag"] = "v1.0.0".into();
    }));
    assert!(why.contains("swarmail is the candidate"), "{why}");
    let swapped = edited(|v| {
        v["products"][1]["release_tag"] = "v9.9.9".into();
        v["products"][2]["executable_sha256"] = "1".repeat(64).into();
    });
    swapped
        .validate()
        .expect("well formed, yet not the locked bytes");
    let why = refusals(protocol::collection_ready(&swapped, &s));
    assert!(
        why.contains("mailpit: protocol release Some(\"v9.9.9\") is not the locked v1.32.0")
            && why.contains("mailhog: protocol executable digest is not the locked one"),
        "{why}"
    );
    // False parity: matched storage over different backends or caps, an
    // enforced claim over an unenforced budget, or a retention fact that
    // is not what the lock establishes.
    let why = invalid(edited(|v| {
        v["storage"]["mode"] = "memory".into();
        v["storage"]["profile"] = "matched_memory".into();
    }));
    assert!(
        why.contains("matched_memory claims one memory backend"),
        "{why}"
    );
    let why = invalid(edited(|v| {
        v["storage"]["profile"] = "matched_memory".into()
    }));
    assert!(
        why.contains("mode \"mixed\" and profile MatchedMemory disagree"),
        "{why}"
    );
    let why = invalid(edited(|v| v["resources"]["profile"] = "enforced".into()));
    assert!(
        why.contains("enforced false cannot be profile Enforced"),
        "{why}"
    );
    let why = invalid(edited(|v| {
        v["storage"]["retention"][1]["prune"] = "no_cap_observed".into();
    }));
    assert!(
        why.contains("mailpit cap and prune behavior disagree"),
        "{why}"
    );
    let why = invalid(edited(|v| {
        v["storage"]["retention"][2]
            .as_object_mut()
            .unwrap()
            .remove("compression");
    }));
    assert!(
        why.contains("mailhog states no backend, prune behavior and compression"),
        "{why}"
    );
    let why = invalid(edited(|v| v["claim"]["statement"] = " ".into()));
    assert!(
        why.contains("no statement of what is and is not claimed"),
        "{why}"
    );
    let lying = edited(|v| v["storage"]["retention"][2]["backend"] = "scratch_file".into());
    let why = refusals(protocol::retention_matches(&lying, &s));
    assert!(
        why.contains("mailhog: protocol backend ScratchFile is not the locked Memory"),
        "{why}"
    );
    // A version 2 protocol without its sections, or a draft with them.
    let why = invalid(edited(|v| {
        v.as_object_mut().unwrap().remove("claim");
    }));
    assert!(why.contains("needs claim, budgets, products"), "{why}");
    let why = invalid(edited(|v| v["protocol_version"] = 1.into()));
    assert!(why.contains("carries version 2 sections"), "{why}");
}

#[test]
fn a_built_from_claim_or_a_draft_or_an_unlisted_enforcement_refuses_collection() {
    let s = SourceLock::parse(SOURCES).unwrap();
    // A profile that requires built-from provenance rejects the explicit
    // unresolved state: the official-artifact scope is the only way past it.
    let why = refusals(protocol::collection_ready(
        &edited(|v| v["claim"]["scope"] = "built_from".into()),
        &s,
    ));
    for t in ["mailpit", "mailhog"] {
        assert!(
            why.contains(&format!(
                "{t}: binary build provenance unresolved (no build attestation"
            )),
            "{why}"
        );
    }
    assert!(why.contains("only a protocol declaring the official-artifact claim scope"));
    // A historical draft never collects, even structurally valid.
    let draft = Protocol::parse(include_bytes!(
        "../docs/evals/history/2026-10-29-protocol.draft-3.json"
    ))
    .unwrap();
    let why = refusals(protocol::collection_ready(&draft, &s));
    for expected in [
        "protocol is a draft, not frozen",
        "protocol version 1 is a historical draft; only versions 2 and 3 collect",
        "unresolved: binary build provenance",
        "mailpit: best wait unresolved: no route, producer",
        "mailpit does not store in memory",
        "resource budget is not enforced",
        "unresolved: primary attribution of MailHog's /api/v1/messages/{id} read",
    ] {
        assert!(why.contains(expected), "missing {expected:?} in:\n{why}");
    }
    // An unenforced budget collects only as its named profile.
    let why = refusals(protocol::collection_ready(
        &edited(|v| {
            v["resources"].as_object_mut().unwrap().remove("profile");
        }),
        &s,
    ));
    assert!(
        why.contains("needs claim, budgets, products, resources.profile"),
        "{why}"
    );
}

#[test]
fn cumulative_residency_beyond_a_declared_cap_refuses_before_any_launch() {
    // A throughput instance's warmup and measured mail, and a wait
    // instance's present and future sends, must all fit every cap.
    let why = refusals(
        edited(|v| {
            v["storage"]["retention"][1]["max_messages"] = 3000.into();
        })
        .validate(),
    );
    assert!(
        why.contains("3100 messages overflow mailpit's retention"),
        "{why}"
    );
    let why = refusals(
        edited(|v| {
            v["storage"]["retention"][1]["max_messages"] = 150.into();
            for t in v["throughput"].as_array_mut().unwrap() {
                t["messages"] = 40.into();
                t["warmup_messages"] = 0.into();
                t["sessions"] = 1.into();
            }
        })
        .validate(),
    );
    assert!(
        why.contains("200 wait sends overflow mailpit's retention"),
        "{why}"
    );
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
    // The v2 bytes are kept as committed, still verify under their own
    // reader with source_revision meaning built-from, and the current
    // reader refuses them rather than reinterpreting that field.
    let v2 = include_bytes!("../docs/evals/history/2026-10-28-sources.lock.v2.json");
    let old = protocol::v2::SourceLock::parse(v2).unwrap();
    old.validate()
        .expect("the historical lock still validates under v2");
    old.verify_files(&evals_dir())
        .expect("its documents are still the committed bytes");
    let why = SourceLock::parse(v2)
        .expect_err("v2 is not a current lock")
        .0
        .join("");
    assert!(why.contains("source_revision means built-from"), "{why}");
    // The tag chains the captures record: source identity, not the
    // executable's build.
    for (t, tag_object, commit) in [
        (
            TargetKind::Mailpit,
            "c00f93d0a78967e9f84c291eb34ab73386fdbe0c",
            "ce798cfd781ef07b321427fc5082b3eebdcae460",
        ),
        (
            TargetKind::Mailhog,
            "056259cd4133e22624340741776709bb46eb9ae9",
            "0441dd494b03c9255a9b8e90e3458ebb115eacff",
        ),
    ] {
        let pin = s.pin(t).unwrap();
        let protocol::Fact::Observed(ts) = &pin.tagged_source else {
            panic!("{t}: tagged source unresolved");
        };
        assert_eq!(
            (ts.tag_object.as_deref().unwrap(), ts.commit.as_str()),
            (tag_object, commit)
        );
        assert_eq!(ts.tag_signature.as_deref(), Some("unsigned"));
        assert!(pin.binary_build_provenance.gap().is_some(), "{t}");
        assert!(pin.wait.best_wait.gap().is_some(), "{t}");
        assert_eq!(pin.wait.reviewed.len(), 2, "{t}");
    }
}

#[test]
fn sha1_and_git_blob_ids_match_their_published_vectors() {
    let hex = |b: [u8; 20]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    for (msg, want) in [
        (&b""[..], "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
        (b"abc", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        (
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
        ),
    ] {
        assert_eq!(hex(protocol::sha1(msg)), want);
    }
    assert_eq!(
        hex(protocol::sha1(&vec![b'a'; 1_000_000])),
        "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
    );
    // `printf 'hello\n' | git hash-object --stdin`
    assert_eq!(
        protocol::git_blob_id(b"hello\n"),
        "ce013625030ba8dba906f756967f9e9ca394464a"
    );
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
            mutate(&|v| v["protocol_version"] = 3.into()),
            "protocol_version 3",
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
            "not memory, durable or mixed",
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
        serde_json::json!({"lock_version": 3, "checked_on": "2026-10-29", "entries": entries}),
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
            "/pin/tagged_source/observed/commit",
            "v1.32.0".into(),
            "needs a full tag object and commit id",
        ),
        (
            "/pin/tagged_source/observed/tag_signature",
            " ".into(),
            "signature status is not recorded",
        ),
        (
            "/pin/tagged_source",
            serde_json::json!({"unresolved": " "}),
            "tagged_source is unresolved without a reason",
        ),
        (
            "/pin/tagged_source",
            serde_json::json!({"unresolved": "not captured"}),
            "cites a commit the lock does not resolve",
        ),
        // A tag's commit is not build evidence: a built-from claim that
        // cites the tag capture is refused, as is a branch for a commit.
        (
            "/pin/binary_build_provenance",
            serde_json::json!({"observed": {"commit": "ce798cfd781ef07b321427fc5082b3eebdcae460", "evidence": "sources/mailpit/mailpit-tag.json"}}),
            "cites no captured build evidence",
        ),
        (
            "/pin/binary_build_provenance",
            serde_json::json!({"observed": {"commit": "develop", "evidence": "sources/mailpit/build.json"}}),
            "is not a full commit id",
        ),
        (
            "/pin/binary_build_provenance",
            serde_json::json!({"unresolved": ""}),
            "binary_build_provenance is unresolved without a reason",
        ),
        (
            "/pin/acquisition/captured_on",
            "yesterday".into(),
            "captured_on \"yesterday\"",
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
            serde_json::json!({"observed": {"from": "api", "url": "https://example.invalid/swagger.json"}}),
            "an API object cannot attribute it",
        ),
        (
            "/pin/documents/2/attribution",
            serde_json::json!({"observed": {"from": "api", "url": "https://example.invalid/swagger.json"}}),
            "not attributed to the official repository's API",
        ),
        (
            "/pin/documents/2/attribution/observed/url",
            "https://github.com/axllent/mailpit/blob/v1.32.0/server/ui/api/v1/swagger.json".into(),
            "not attributed to its path at the tagged commit",
        ),
        (
            "/pin/documents/2/attribution/observed/git_blob",
            "main".into(),
            "a repository path and a full git blob id",
        ),
        (
            "/pin/documents/5/attribution",
            serde_json::json!({"observed": {"from": "commit", "url": "x", "upstream_path": "x", "git_blob": "x"}}),
            "is an API object, not a file of the tagged commit",
        ),
        ("/pin/documents/0/size", 0.into(), "size 0 is out of bounds"),
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
            "/pin/wait/best_wait",
            serde_json::json!({"unresolved": ""}),
            "best_wait is unresolved without a reason",
        ),
        (
            "/pin/wait/best_wait",
            serde_json::json!({"observed": " "}),
            "names no mechanism",
        ),
        ("/pin/wait/finding", " ".into(), "states no finding"),
        (
            "/pin/wait/reviewed/0/retained_as",
            serde_json::json!({"observed": "sources/mailpit/mailpit-readme.md"}),
            "names no retained source document with its digest",
        ),
        (
            "/pin/wait/reviewed/1/retained_as",
            serde_json::json!({"unresolved": " "}),
            "server/websockets/client.go is unresolved without a reason",
        ),
        (
            "/pin/wait/reviewed/1/upstream_path",
            "server/websockets/hub.go".into(),
            "listed twice",
        ),
        (
            "/pin/wait/reviewed/0/upstream_path",
            "../hub.go".into(),
            "is malformed",
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
            serde_json::json!({"lock_version": 3, "checked_on": "2026-10-29", "entries": [extra]})
        )
        .is_err(),
        "an unknown field never parses"
    );
    let mut v = json(SOURCES);
    v["lock_version"] = 5.into();
    v["checked_on"] = "today".into();
    let why = refusals(lock_from(v).unwrap().validate());
    assert!(
        why.contains("lock_version 5 is not 3 or 4") && why.contains("checked_on"),
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
    // Every pinned row of the committed lock keeps its captured documents.
    for t in [
        "mailpit",
        "mailhog",
        "inbucket",
        "mailslurper",
        "mailslurper-source-build",
        "smtp4dev",
        "greenmail",
        "alkemio-mailslurper",
    ] {
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
fn the_tag_chain_and_every_blob_are_rechecked_against_the_captures() {
    let root = tempfile::tempdir().unwrap();
    let dir = evals_copy(root.path());
    // Byte checks only: several of these edits are refused structurally
    // too, and this proves the captures refuse them on their own.
    let check = |entry: serde_json::Value| refusals(lock_of(vec![entry]).verify_files(&dir));
    for t in [TargetKind::Mailpit, TargetKind::Mailhog] {
        lock_of(vec![committed_entry(t)])
            .verify_files(&dir)
            .unwrap_or_else(|r| panic!("{t} verifies: {r}"));
    }
    let other = "0123456789abcdef0123456789abcdef01234567";
    let cases: Vec<(&str, serde_json::Value, &str)> = vec![
        // The tag chain: a different commit or tag object, a claimed
        // signature, all refused against the captured objects.
        (
            "/pin/tagged_source/observed/commit",
            other.into(),
            "the captured tag object does not name v1.32.0, commit 0123",
        ),
        (
            "/pin/tagged_source/observed/commit",
            other.into(),
            "the captured tree is not the complete listing of commit 0123",
        ),
        (
            "/pin/tagged_source/observed/tag_object",
            other.into(),
            "the captured ref does not name tag object 0123",
        ),
        (
            "/pin/tagged_source/observed/tag_signature",
            "valid".into(),
            "signature \"valid\"",
        ),
        // A document is the blob its attribution names, at that path in
        // the tagged tree, with its stated size.
        (
            "/pin/documents/2/attribution/observed/git_blob",
            other.into(),
            "mailpit-swagger.json is not the git blob its attribution names",
        ),
        (
            "/pin/documents/1/attribution/observed/upstream_path",
            "LICENSE.md".into(),
            "mailpit-license.txt is not LICENSE.md in the tagged tree",
        ),
        (
            "/pin/documents/3/size",
            7.into(),
            "mailpit-readme.md is not 7 bytes",
        ),
        (
            "/pin/wait/reviewed/1/git_blob",
            other.into(),
            "reviewed source server/websockets/client.go is not in the tagged tree",
        ),
        // An API capture names itself; another object's URL refuses.
        (
            "/pin/documents/5/attribution/observed/url",
            "https://api.github.com/repos/axllent/mailpit/git/refs/tags/v1.31.0".into(),
            "mailpit-ref.json does not name itself",
        ),
    ];
    for (path, value, want) in cases {
        let mut e = committed_entry(TargetKind::Mailpit);
        *e.pointer_mut(path).unwrap() = value;
        let why = check(e);
        assert!(why.contains(want), "{path}: expected {want:?} in:\n{why}");
    }
    // A truncated tree listing proves nothing about absent paths, even
    // when the lock is rewritten to its new digest.
    let tree = dir.join("sources/mailpit/mailpit-tree.json");
    let mut v = json(&std::fs::read(&tree).unwrap());
    v["truncated"] = true.into();
    let bytes = serde_json::to_vec(&v).unwrap();
    std::fs::write(&tree, &bytes).unwrap();
    let mut e = committed_entry(TargetKind::Mailpit);
    e["pin"]["documents"][7]["sha256"] = targets::sha256_hex(&bytes).into();
    e["pin"]["documents"][7]["size"] = bytes.len().into();
    assert!(check(e).contains("the captured tree is not the complete listing"));
    // A changed source body refuses by digest before anything else.
    let hub = dir.join("sources/mailpit/mailpit-websockets-hub.go");
    let mut bytes = std::fs::read(&hub).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&hub, &bytes).unwrap();
    let why = check(committed_entry(TargetKind::Mailpit));
    assert!(
        why.contains("mailpit-websockets-hub.go does not match its digest"),
        "{why}"
    );
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
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
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
    assert_cells_match_rows(&dir);

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
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
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
    assert_cells_match_rows(&dir);
    let cell = &report["throughput"][0];
    assert_eq!(
        cell["sessions"], 1000,
        "the scheduled sessions are reported"
    );
    assert_eq!(cell["requested"], 1000);
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
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
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
        &targets::Keepers::default(),
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
        targets::Owned::launch(
            &wrong,
            &leaf,
            &limits(),
            &clock,
            &targets::Keepers::default()
        )
        .await
        .err(),
        Some(targets::LaunchError::DigestMismatch)
    );
    assert!(!leaf.exists(), "a refused digest creates nothing");
    let mut missing = swarmail_spec(targets::swarmail_template(0));
    missing.binary = root.path().join("no-such-server");
    assert_eq!(
        targets::Owned::launch(
            &missing,
            &leaf,
            &limits(),
            &clock,
            &targets::Keepers::default()
        )
        .await
        .err(),
        Some(targets::LaunchError::Binary("unreadable"))
    );
    let mut plain = swarmail_spec(targets::swarmail_template(0));
    plain.binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert_eq!(
        targets::Owned::launch(
            &plain,
            &leaf,
            &limits(),
            &clock,
            &targets::Keepers::default()
        )
        .await
        .err(),
        Some(targets::LaunchError::Binary("not executable"))
    );
    std::fs::create_dir(&leaf).unwrap();
    let ok = swarmail_spec(targets::swarmail_template(0));
    assert_eq!(
        targets::Owned::launch(&ok, &leaf, &limits(), &clock, &targets::Keepers::default())
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
    match targets::Owned::launch(
        &broken,
        &leaf,
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await
    .err()
    {
        Some(targets::LaunchError::NotReady(_, facts)) => assert!(facts.proved(), "{facts:?}"),
        other => panic!("expected a readiness refusal, got {other:?}"),
    }
    assert!(!leaf.exists());
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn a_dropped_owner_leaves_its_worker_to_finish_and_prove_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("leaf");
    let clock = wire::Clock(Instant::now());
    let keepers = targets::Keepers::default();
    let owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &scratch,
        &limits(),
        &clock,
        &keepers,
    )
    .await
    .unwrap();
    let pid = owned.pid();
    // Cancellation at active work: the owner is dropped with a request in
    // flight, never told to stop.
    let work = async move {
        let _ = owned.list_all(4).await;
        std::future::pending::<()>().await;
    };
    tokio::select! {
        biased;
        _ = work => unreachable!("the work never ends by itself"),
        _ = async {} => {}
    }
    let settled = keepers.settle(limits().cleanup * 2).await;
    assert_eq!(settled.len(), 1);
    assert!(settled[0].proved(), "{:?}", settled[0]);
    assert!(stops_running(pid).await);
    assert!(
        !scratch.exists(),
        "the surviving worker disposed of the scratch"
    );
    assert!(
        keepers.settle(limits().cleanup).await.is_empty(),
        "a settled worker is not counted twice"
    );
}

/// A stand-in server: `/bin/sh -c body`, which never becomes ready, so
/// every launch of it ends in the readiness refusal and its cleanup. The
/// shell is an existing executable: a freshly written script could be
/// "text file busy" while a parallel test forks.
fn fake_spec(kind: TargetKind, body: &str) -> targets::LaunchSpec {
    targets::LaunchSpec::new(
        kind,
        PathBuf::from("/bin/sh"),
        None,
        vec!["-c".into(), body.into()],
    )
}

fn short_limits() -> targets::Limits {
    targets::Limits {
        startup: Duration::from_millis(400),
        cleanup: Duration::from_secs(2),
        request: Duration::from_millis(200),
        max_response_bytes: 1 << 20,
        open_files: 256,
    }
}

/// The pid a fake server wrote into its private HOME, once it has.
async fn started(scratch: &Path) -> i32 {
    let marker = scratch.join("home/pid");
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(text) = std::fs::read_to_string(&marker)
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        assert!(Instant::now() < until, "the fake server never started");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn a_launch_cancelled_during_readiness_is_still_cleaned_by_its_worker() {
    // The fake server never binds its reserved slots, so a slot reads as
    // closed only while no parallel test has taken it: a stranger's
    // listener fails the proof by design (`refused`). An attempt whose only
    // unproved fact is a listener is tried again, a bounded number of
    // times; every other fact is asserted on every attempt.
    let root = tempfile::tempdir().unwrap();
    let body = "echo $$ > \"$HOME/pid.tmp\"; mv \"$HOME/pid.tmp\" \"$HOME/pid\"; exec sleep 600";
    for attempt in 1..=3 {
        let scratch = root.path().join(format!("leaf{attempt}"));
        let keepers = targets::Keepers::default();
        let mut lim = short_limits();
        lim.startup = Duration::from_secs(60);
        let clock = wire::Clock(Instant::now());
        let spec = fake_spec(TargetKind::Mailpit, body);
        let launch = targets::Owned::launch(&spec, &scratch, &lim, &clock, &keepers);
        let pid = tokio::select! {
            r = launch => panic!("a never-ready server became ready: {:?}", r.err()),
            pid = started(&scratch) => pid,
        };
        let settled = keepers.settle(lim.cleanup * 2).await;
        assert_eq!(settled.len(), 1);
        let f = &settled[0];
        assert!(stops_running(pid).await);
        assert!(!scratch.exists());
        assert!(
            f.reaped && f.group_gone && f.scratch_removed && !f.forced_kill,
            "{f:?}"
        );
        if f.proved() {
            return;
        }
        assert!(!f.listeners_closed && attempt < 3, "{f:?}");
    }
}

#[tokio::test]
async fn a_term_resistant_child_is_killed_within_one_composed_cleanup_budget() {
    // The fake server never binds its reserved slots, so a slot reads as
    // closed only while no parallel test has taken it. An attempt whose only
    // unproved fact is a listener is tried again, a bounded number of times;
    // every other fact and the time bound are asserted on every attempt.
    let root = tempfile::tempdir().unwrap();
    // The ignored TERM is inherited by the sleeps, so only the group's
    // forced kill ends them.
    let body = "trap '' TERM; while :; do sleep 1; done";
    let lim = short_limits();
    for attempt in 1..=3 {
        let clock = wire::Clock(Instant::now());
        let scratch = root.path().join(format!("leaf{attempt}"));
        let begun = Instant::now();
        let r = targets::Owned::launch(
            &fake_spec(TargetKind::Mailhog, body),
            &scratch,
            &lim,
            &clock,
            &targets::Keepers::default(),
        )
        .await;
        let took = begun.elapsed();
        let facts = match r.err() {
            Some(targets::LaunchError::NotReady(why, facts)) => {
                assert_eq!(why, "readiness deadline");
                facts
            }
            other => panic!("expected a readiness refusal, got {other:?}"),
        };
        // Startup, plus one readiness probe past it, plus one cleanup
        // budget: the grace, the kill, the group, the drain and the listener
        // probes share that budget rather than each taking their own.
        let bound = lim.startup + lim.request * 2 + lim.cleanup;
        assert!(took < bound + Duration::from_millis(500), "{took:?}");
        assert!(!scratch.exists());
        assert!(
            facts.forced_kill && facts.reaped && facts.group_gone && facts.scratch_removed,
            "{facts:?}"
        );
        if facts.proved() {
            return;
        }
        assert!(!facts.listeners_closed && attempt < 3, "{facts:?}");
    }
}

#[tokio::test]
async fn output_pressure_never_stalls_the_child_or_its_cleanup() {
    let root = tempfile::tempdir().unwrap();
    // A "Swarmail" whose startup line names closed ports and that then
    // floods its stdout: the drain keeps the pipe empty, and its handle
    // is awaited at cleanup.
    let body = "echo 'swarmail smtp=127.0.0.1:1 http=127.0.0.1:1'; exec yes flood";
    let r = targets::Owned::launch(
        &fake_spec(TargetKind::Swarmail, body),
        &root.path().join("leaf"),
        &short_limits(),
        &wire::Clock(Instant::now()),
        &targets::Keepers::default(),
    )
    .await;
    match r.err() {
        Some(targets::LaunchError::NotReady(why, facts)) => {
            assert_eq!(why, "readiness deadline");
            assert!(facts.proved() && !facts.forced_kill, "{facts:?}");
        }
        other => panic!("expected a readiness refusal, got {other:?}"),
    }
    assert!(!root.path().join("leaf").exists());
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn an_acknowledged_cleanup_hold_keeps_the_parent_waiting_even_across_cancellation() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("leaf");
    let keepers = targets::Keepers::default();
    let mut owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &scratch,
        &limits(),
        &wire::Clock(Instant::now()),
        &keepers,
    )
    .await
    .unwrap();
    let pid = owned.pid();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    owned.on_cleanup(async move {
        let _ = entered_tx.send(());
        let _ = release_rx.await;
    });
    {
        let shutdown = owned.shutdown();
        tokio::pin!(shutdown);
        // One poll sends the stop; the worker acknowledges the hold.
        assert!(
            tokio::time::timeout(Duration::ZERO, &mut shutdown)
                .await
                .is_err()
        );
        entered_rx.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::ZERO, &mut shutdown)
                .await
                .is_err(),
            "the parent is still waiting while cleanup is held"
        );
        assert!(
            matches!(proc_state(pid), Some('S') | Some('R')),
            "nothing is cleaned while the hold stands"
        );
        // Cancellation during cleanup: the waiting shutdown is dropped.
    }
    release_tx.send(()).unwrap();
    let facts = owned.shutdown().await;
    assert!(facts.proved(), "{facts:?}");
    assert!(!scratch.exists());
    let settled = keepers.settle(limits().cleanup).await;
    assert_eq!(settled, vec![facts]);
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn a_worker_that_fails_proves_nothing_and_still_kills_its_group() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("leaf");
    let keepers = targets::Keepers::default();
    let mut owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &scratch,
        &limits(),
        &wire::Clock(Instant::now()),
        &keepers,
    )
    .await
    .unwrap();
    let pid = owned.pid();
    // A nested failure inside the worker: its join fails, its report
    // never arrives, and its group guard is the only cleanup left.
    owned.on_cleanup(async { panic!("injected worker failure (expected)") });
    let facts = owned.shutdown().await;
    assert_eq!(facts, targets::CleanupFacts::unproved(TargetKind::Swarmail));
    assert_eq!(
        keepers.settle(limits().cleanup).await,
        vec![targets::CleanupFacts::unproved(TargetKind::Swarmail)]
    );
    assert!(stops_running(pid).await, "the guard killed the group");
    assert!(
        scratch.exists(),
        "forced death proves nothing, so the scratch stays for the operator"
    );
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn a_drain_that_fails_to_join_leaves_cleanup_unproved_and_the_run_unindexed() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("leaf");
    let keepers = targets::Keepers::default();
    let mut owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &scratch,
        &limits(),
        &wire::Clock(Instant::now()),
        &keepers,
    )
    .await
    .unwrap();
    let pid = owned.pid();
    // The worker itself survives: the nested join of the drain fails, and
    // that alone leaves its disposal unproved.
    owned.fail_drain().await;
    let facts = owned.shutdown().await;
    assert!(!facts.reaped, "a failed drain join is not a reaped child");
    assert!(!facts.proved(), "{facts:?}");
    assert!(
        facts.group_gone && facts.listeners_closed && facts.scratch_removed,
        "every other step still ran and was observed: {facts:?}"
    );
    assert!(stops_running(pid).await);
    assert_eq!(keepers.settle(limits().cleanup).await, vec![facts.clone()]);
    // Admission stops there: the run directory takes no index.
    let p = tiny();
    let dir =
        records::RunDir::create(&root.path().join("run"), &manifest(&p, "drain-run")).unwrap();
    dir.write_artifact("report.json", b"{}").unwrap();
    assert!(
        dir.finalize(facts.proved())
            .unwrap_err()
            .contains("cleanup is not proved")
    );
    assert!(!root.path().join("run/index.json").exists());
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn bind_ownership_is_read_from_the_kernel_and_refuses_foreign_or_ambiguous_listeners() {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    let root = tempfile::tempdir().unwrap();
    let keepers = targets::Keepers::default();
    let mut owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &root.path().join("leaf"),
        &limits(),
        &wire::Clock(Instant::now()),
        &keepers,
    )
    .await
    .unwrap();
    let pid = owned.pid();
    assert_eq!(targets::binds(pid, owned.smtp), targets::Bind::Owned);
    assert_eq!(targets::binds(pid, owned.http_addr), targets::Bind::Owned);
    let (smtp, http) = (owned.smtp, owned.http_addr);
    assert!(owned.shutdown().await.proved());
    assert_eq!(targets::binds(pid, smtp), targets::Bind::Absent);
    assert_eq!(targets::binds(pid, http), targets::Bind::Absent);

    let reuse = |addr: &str| {
        let s = tokio::net::TcpSocket::new_v4().unwrap();
        s.set_reuseport(true).unwrap();
        s.bind(addr.parse().unwrap()).unwrap();
        s.listen(8).unwrap().into_std().unwrap()
    };
    // Another group holds one listener on a port; this process holds a
    // second on the same port, and a wildcard on another.
    let theirs = reuse("127.0.0.1:0");
    let port = theirs.local_addr().unwrap().port();
    let fd = theirs.as_raw_fd();
    let mut other = std::process::Command::new("sleep");
    other.arg("60").process_group(0);
    // SAFETY: dup2 and fcntl are async-signal-safe. They give the child the
    // listener as fd 3 without close-on-exec; dup2 onto itself would keep
    // the flag, so an fd that already is 3 has the flag cleared instead.
    unsafe {
        other.pre_exec(move || {
            let r = if fd == 3 {
                libc::fcntl(3, libc::F_SETFD, 0)
            } else {
                libc::dup2(fd, 3)
            };
            if r < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut other = other.spawn().unwrap();
    let group = other.id() as i32;
    drop(theirs);
    let at = |p: u16| std::net::SocketAddr::from(([127, 0, 0, 1], p));
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        let seen = targets::binds(group, at(port));
        if seen == targets::Bind::Owned {
            break;
        }
        assert!(
            Instant::now() < until,
            "the other group never held its listener: {seen:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let ours = reuse(&format!("127.0.0.1:{port}"));
    assert_eq!(
        targets::binds(group, at(port)),
        targets::Bind::Ambiguous,
        "a second holder outside the group"
    );
    let wildcard = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    let wport = wildcard.local_addr().unwrap().port();
    assert_eq!(
        targets::binds(group, at(wport)),
        targets::Bind::Foreign,
        "a wildcard listener accepts at the loopback address too"
    );
    other.kill().unwrap();
    other.wait().unwrap();
    assert_eq!(targets::binds(group, at(port)), targets::Bind::Foreign);
    drop(ours);
    drop(wildcard);
    assert!(keepers.settle(limits().cleanup).await[0].proved());
}

#[tokio::test]
#[cfg(not(target_os = "linux"))]
async fn without_proc_bind_ownership_is_unknown_and_the_launch_refuses_after_cleaning() {
    let root = tempfile::tempdir().unwrap();
    let r = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &root.path().join("leaf"),
        &limits(),
        &wire::Clock(Instant::now()),
        &targets::Keepers::default(),
    )
    .await;
    match r.err() {
        Some(targets::LaunchError::NotReady(why, facts)) => {
            assert_eq!(why, "smtp bind ownership unknown");
            assert!(facts.proved(), "{facts:?}");
        }
        other => panic!("expected a bind refusal, got {other:?}"),
    }
}

#[test]
fn the_executable_is_rechecked_against_its_launch_digest() {
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("server");
    std::fs::write(&bin, b"first bytes").unwrap();
    let digest = targets::sha256_hex(b"first bytes");
    assert!(targets::unchanged(&bin, &digest));
    std::fs::write(&bin, b"other bytes").unwrap();
    assert!(!targets::unchanged(&bin, &digest), "replaced bytes refuse");
    std::fs::remove_file(&bin).unwrap();
    assert!(
        !targets::unchanged(&bin, &digest),
        "a vanished executable refuses"
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
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
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

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn admission_recomputes_the_plan_the_receipts_and_the_report() {
    let root = tempfile::tempdir().unwrap();
    let p = tiny();
    let bytes = serde_json::to_vec(&p).unwrap();
    let (verdict, _, sealed) = run(p.clone(), root.path(), "plan-run").await;
    assert_eq!(verdict, runner::Verdict::Clean);
    let good = sealed.unwrap();
    report::admit_run(&good, &bytes).expect("the real run admits against its protocol");
    let m = records::read_manifest(&good).unwrap();
    let all = rows(&good);
    let on_disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(good.join("report.json")).unwrap()).unwrap();

    // A sealed copy with chosen planned coordinates, rows and report: a
    // directory the storage layer accepts, so only the protocol can refuse.
    let forge = |name: &str, keep: usize, report: &serde_json::Value| {
        let mut manifest = m.clone();
        manifest.planned.truncate(keep);
        let mut dir = records::RunDir::create(&root.path().join(name), &manifest).unwrap();
        for r in &all {
            if manifest
                .planned
                .iter()
                .any(|p| p.target == r.target && p.profile == r.profile && p.phase == r.phase)
            {
                dir.append(r).unwrap();
            }
        }
        dir.write_artifact("report.json", &serde_json::to_vec(report).unwrap())
            .unwrap();
        let path = dir.finalize(true).expect("storage accepts it");
        records::admit(&path).expect("and so does the directory-level admission");
        path
    };
    let full = m.planned.len();
    assert!(full >= 2);

    // A strict prefix of the plan is a smaller denominator, never a run.
    let why = report::admit_run(&forge("prefix", full - 1, &on_disk), &bytes).unwrap_err();
    assert!(why.contains("a smaller denominator never admits"), "{why}");

    // An index that arrived while a cleanup was unproved, or with a launch
    // that has no receipt, never admits.
    let mut unproved = on_disk.clone();
    unproved["cleanups"][0]["reaped"] = false.into();
    let why = report::admit_run(&forge("unproved", full, &unproved), &bytes).unwrap_err();
    assert!(why.contains("proved cleanup receipt"), "{why}");
    let mut missing = on_disk.clone();
    missing["cleanups"] = serde_json::json!([]);
    let why = report::admit_run(&forge("missing", full, &missing), &bytes).unwrap_err();
    assert!(why.contains("proved cleanup receipt"), "{why}");

    // A report that is not the arithmetic of its rows never admits.
    let mut lying = on_disk.clone();
    lying["throughput"][0]["reconciliation"]["accepted"] = 999.into();
    let why = report::admit_run(&forge("lying", full, &lying), &bytes).unwrap_err();
    assert!(why.contains("disagrees with the rows"), "{why}");
    assert!(
        why.contains("throughput[0].reconciliation.accepted")
            && why.contains("report 999")
            && why.contains("rows 4"),
        "the refusal names the first difference: {why}"
    );

    // A report from another version of the driver is refused by name,
    // before any of its cells are compared.
    let mut other_version = on_disk.clone();
    other_version["report_version"] = 999.into();
    other_version["throughput"][0]["reconciliation"]["accepted"] = 999.into();
    let why = report::admit_run(&forge("version", full, &other_version), &bytes).unwrap_err();
    assert!(
        why.contains("999")
            && why.contains(&format!("version {}", report::REPORT_VERSION))
            && why.contains("re-collect with the admitting driver")
            && !why.contains("disagrees"),
        "{why}"
    );

    // Another protocol's bytes, or none at all, never admit.
    let other = serde_json::to_vec(&smoke()).unwrap();
    let why = report::admit_run(&good, &other).unwrap_err();
    assert!(why.contains("different protocol"), "{why}");
    assert!(report::admit_run(&good, b"{").is_err());

    // A killed run: the first leaves exist, nothing seals, and the plan
    // stays the full denominator with no index.
    let killed = root.path().join("killed");
    let mut dir = records::RunDir::create(&killed, &m).unwrap();
    dir.append(&all[0]).unwrap();
    drop(dir);
    assert!(!killed.join("index.json").exists());
    let why = report::admit_run(&killed, &bytes).unwrap_err();
    assert!(why.contains("incomplete"), "{why}");
}

/// A sealed copy of a real run with edited rows: the report is regenerated
/// from them by the driver's own arithmetic and serializer, the real run's
/// proved launch and cleanup receipts are kept, and storage accepts the
/// directory, so only the admission arithmetic can refuse it.
fn sew(
    root: &Path,
    name: &str,
    real: &Path,
    p: &Protocol,
    edit: impl FnOnce(&mut Vec<Row>),
) -> PathBuf {
    let m = records::read_manifest(real).unwrap();
    let mut all = rows(real);
    edit(&mut all);
    let on_disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(real.join("report.json")).unwrap()).unwrap();
    let life = runner::Lifecycle {
        launches: Vec::new(),
        cleanups: Vec::new(),
    };
    let mut fresh = serde_json::to_value(report::report(p, &m, &all, &life)).unwrap();
    fresh["launches"] = on_disk["launches"].clone();
    fresh["cleanups"] = on_disk["cleanups"].clone();
    let mut dir = records::RunDir::create(&root.join(name), &m).unwrap();
    for r in &all {
        dir.append(r).unwrap();
    }
    dir.write_artifact("report.json", &serde_json::to_vec_pretty(&fresh).unwrap())
        .unwrap();
    dir.finalize(true).expect("storage accepts the sewn run")
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn admission_survives_rates_that_do_not_round_trip() {
    // 4 accepted sends over a 1 000 000 002 ns window: 4e9 / 1 000 000 002 is
    // 3.9999999920000002, one of the values serde_json's default float
    // parser reads back one unit in the last place away.
    let rate = 4.0 * 1e9 / 1_000_000_002.0;

    let root = tempfile::tempdir().unwrap();
    let p = tiny();
    let bytes = serde_json::to_vec(&p).unwrap();
    let (verdict, _, sealed) = run(p.clone(), root.path(), "float-run").await;
    assert_eq!(verdict, runner::Verdict::Clean);
    let real = sealed.unwrap();
    let sewn = sew(root.path(), "float-sewn", &real, &p, |all| {
        let v = all.iter_mut().find(|r| r.phase == Phase::Verified).unwrap();
        v.start_ns = Some(0);
        v.ack_ns = Some(1_000_000_002);
    });
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(sewn.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        written["throughput"][0]["reconciliation"]["accepted"], 4,
        "the sewn cell holds the fixture's four accepted sends"
    );
    assert_eq!(
        written["throughput"][0]["accepted_per_second"].as_f64(),
        Some(rate),
        "the sewn cell carries the rate that does not round-trip by default"
    );
    report::admit_run(&sewn, &bytes).expect("a rate that does not misparse admits");
    // The property admission relies on: a finite f64 survives the report's
    // own serialize-then-parse.
    let echoed: serde_json::Value =
        serde_json::from_slice(&serde_json::to_vec(&serde_json::json!({ "r": rate })).unwrap())
            .unwrap();
    assert_eq!(
        echoed["r"].as_f64(),
        Some(rate),
        "floats round-trip exactly"
    );
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn a_run_with_competitor_refusals_admits() {
    let root = tempfile::tempdir().unwrap();
    let p = tiny();
    let bytes = serde_json::to_vec(&p).unwrap();
    let (verdict, _, sealed) = run(p.clone(), root.path(), "refusal-run").await;
    assert_eq!(verdict, runner::Verdict::Clean);
    let real = sealed.unwrap();
    let sewn = sew(root.path(), "refusal-sewn", &real, &p, |all| {
        let sends: Vec<usize> = (0..all.len())
            .filter(|i| all[*i].phase == Phase::Send)
            .collect();
        let tokens: Vec<_> = sends[..2].iter().map(|i| all[*i].token.clone()).collect();
        // One connect refusal with its reply code, as the runner writes it,
        // and one send whose outcome the client could not establish.
        let refused = &mut all[sends[0]];
        refused.outcome = "failed_before_terminator".into();
        refused.reason = Some("connect: refused".into());
        refused.code = Some(421);
        refused.start_ns = None;
        refused.ack_ns = None;
        let unknown = &mut all[sends[1]];
        unknown.outcome = "ambiguous".into();
        unknown.ack_ns = None;
        // A refused or unknown send is not an accepted one: whatever of it
        // is visible afterwards is unexpected, never a delivery.
        for r in all.iter_mut().filter(|r| r.phase == Phase::Visible) {
            if tokens.contains(&r.token) {
                r.outcome = "unexpected".into();
            }
        }
    });
    report::admit_run(&sewn, &bytes).expect("a run carrying refusals and ambiguity admits");
    assert_cells_match_rows(&sewn);
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(sewn.join("report.json")).unwrap()).unwrap();
    let cell = &written["throughput"][0];
    assert_eq!(cell["refused"], 1, "{cell}");
    assert_eq!(cell["exact"], false, "{cell}");
    assert_eq!(cell["reconciliation"]["failed_before_terminator"], 1);
    assert_eq!(cell["reconciliation"]["ambiguous"], 1);
    let n = |k: &str| cell[k].as_u64().unwrap();
    let rec = |k: &str| cell["reconciliation"][k].as_u64().unwrap();
    assert_eq!(
        n("requested"),
        n("admitted") + n("refused") + rec("ambiguous") + n("unstarted")
    );
    assert_eq!(
        rec("attempted"),
        rec("accepted") + rec("rejected") + rec("failed_before_terminator") + rec("ambiguous")
    );
}

#[test]
fn connect_refusal_is_a_coded_failed_before_terminator_row() {
    let blank = || Row {
        row_version: records::ROW_VERSION,
        run_id: "r".into(),
        target: TargetKind::Mailpit,
        profile: "p".into(),
        repetition: 0,
        phase: Phase::Send,
        sample: 0,
        token: None,
        outcome: "unstarted".into(),
        reason: None,
        code: None,
        invoked_ns: None,
        start_ns: None,
        ack_ns: None,
        observed_ns: None,
        occurrences: None,
        fidelity_ok: None,
        requests: None,
    };
    // A peer that answers with a reply code refused the session itself.
    let mut refused = blank();
    runner::connect_failure(&mut refused, 421, wire::WireError::Malformed);
    assert_eq!(refused.outcome, "failed_before_terminator");
    assert_eq!(refused.code, Some(421));
    assert_eq!(refused.reason.as_deref(), Some("connect: refused"));
    // A failure with no reply keeps the wire's own reason and no code.
    let mut silent = blank();
    runner::connect_failure(&mut silent, 0, wire::WireError::Timeout);
    assert_eq!(silent.outcome, "failed_before_terminator");
    assert_eq!(silent.code, None);
    assert_eq!(silent.reason.as_deref(), Some("connect: deadline"));
}

#[tokio::test]
async fn a_421_greeting_is_a_coded_connect_refusal() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let (mut s, _) = listener.accept().await.unwrap();
        s.write_all(b"421 4.3.2 Too many connections, please try again later\r\n")
            .await
            .unwrap();
    });
    let refused = wire::Smtp::open(addr, Duration::from_secs(5)).await;
    peer.await.unwrap();
    match refused {
        Err((421, wire::WireError::Malformed)) => {}
        Err((code, e)) => panic!("expected a coded refusal, got {code} {}", e.reason()),
        Ok(_) => panic!("a 421 greeting must not open a session"),
    }
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
    compare_mail_env(args, witness, &[])
}

/// As [`compare_mail`], with extra environment (the wrapper's prebuilt-binary
/// overrides and a fake verifier's controls).
fn compare_mail_env(args: &[&str], witness: &Path, env: &[(&str, &str)]) -> (i32, String) {
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
        .envs(env.iter().copied())
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
        (
            vec!["collect", "--output", o],
            "pass --bin mailpit=PATH with its pinned executable",
        ),
        (vec!["evidence", "--output", o], "--final is required"),
        (
            vec!["evidence", "--output", o, "--final", "/nonexistent/final"],
            "does not exist",
        ),
        (vec!["measure", "--output", o], "--config is required"),
        (
            vec!["measure", "--output", o, "--config", "/nonexistent/config"],
            "does not exist",
        ),
    ];
    for (args, want) in cases {
        let (code, text) = compare_mail(&args, &w);
        assert_eq!(code, 2, "{args:?}: {text}");
        assert!(text.contains(want), "{args:?}: expected {want:?} in {text}");
    }

    // T10's prerequisites, in order: removing or emptying any one refuses
    // by its name, and an earlier one is always named first.
    let prerequisites = [
        ("baseline", "--baseline"),
        ("final-contracts", "--contracts"),
        ("controls", "--controls-dir"),
        ("coordination-receipts", "--receipts"),
        ("cost-basis", "--cost"),
        ("identity", "--identity"),
        ("full-product", "--product"),
    ];
    let empty = root.path().join("empty");
    std::fs::write(&empty, b"").unwrap();
    let e = empty.to_str().unwrap();
    for mode_args in [
        vec!["evidence", "--output", o, "--final", i],
        vec!["measure", "--output", o, "--config", i],
    ] {
        for (n, (name, _)) in prerequisites.iter().enumerate() {
            for corrupt in [false, true] {
                let mut args = mode_args.clone();
                for (m, (_, flag)) in prerequisites.iter().enumerate() {
                    if m != n {
                        args.extend([*flag, i]);
                    } else if corrupt {
                        args.extend([*flag, e]);
                    }
                }
                let (code, text) = compare_mail(&args, &w);
                assert_eq!(code, 2, "{args:?}: {text}");
                assert!(
                    text.contains(&format!("blocked at phase \"prerequisite-{name}\"")),
                    "{args:?} should name {name}: {text}"
                );
            }
        }
        // Everything missing: the first in order is the one named.
        let (code, text) = compare_mail(&mode_args, &w);
        assert_eq!(code, 2);
        assert!(text.contains("\"prerequisite-baseline\""), "{text}");
    }
    // `controls` needs only the product and the competitors' executables.
    // With no product it has no input, and main's refusal stands (the first
    // prerequisite of the chain), with a hint on what it does need.
    let (code, text) = compare_mail(&["controls", "--output", o], &w);
    assert_eq!(code, 2);
    assert!(
        text.contains("blocked at phase \"prerequisite-baseline\": --baseline is not supplied")
            && text.contains("controls needs --product P"),
        "{text}"
    );
    let product = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/evals/agent-comparison.json");
    let p = product.to_str().unwrap();
    let (code, text) = compare_mail(&["controls", "--output", o, "--product", p], &w);
    assert_eq!(code, 2);
    assert!(
        text.contains("\"competitor-binaries\"")
            && text.contains("--bin mailpit=PATH with its pinned executable"),
        "{text}"
    );
    // Only the lock entries of the product record are accepted, and the
    // MailSlurper built from source is `mailslurper-source-build`.
    let exe = script(&root.path().join("exe"), "exit 0\n");
    let (code, text) = compare_mail(
        &[
            "controls",
            "--output",
            o,
            "--product",
            p,
            "--bin",
            &format!("mailpit={exe}"),
            "--bin",
            &format!("mailhog={exe}"),
            "--bin",
            &format!("mailslurper={exe}"),
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(
        text.contains("\"competitor-binaries\"")
            && text.contains("--bin mailslurper-source-build=PATH"),
        "{text}"
    );
    let (code, text) = compare_mail(
        &[
            "controls",
            "--output",
            o,
            "--product",
            p,
            "--mailpit",
            &exe,
            "--mailhog",
            &exe,
            "--bin",
            &format!("mailslurper-source-build={i}"),
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(text.contains("is not an executable file"), "{text}");
    // `--mailslurper` is main's grammar's unknown flag: the official row does
    // not start, and no flag stands for the source build but its lock entry.
    let (code, text) = compare_mail(&["controls", "--output", o, "--mailslurper", i], &w);
    assert_eq!(code, 2);
    assert!(text.contains("unknown argument: --mailslurper"), "{text}");

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

// ---- the wrapper's integrated modes (decision 0129 §9) ----------------------

/// A scratch directory on /home (under the build directory), never the
/// RAM-backed /tmp.
fn home_scratch() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/wrapper-tests");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::Builder::new()
        .prefix("w-")
        .tempdir_in(root)
        .unwrap()
}

fn script(path: &Path, body: &str) -> String {
    std::fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.display().to_string()
}

/// Everything a non-dry or dry measure needs, with a fake `swarmail-evals`
/// that logs its arguments, refuses as `FAKE_REFUSE` says, and otherwise
/// succeeds. The wrapper's own logic is what is proved here: the verifier's
/// checks are proved on real bytes in the eval crate's tests.
struct Rig {
    dir: tempfile::TempDir,
    evals: String,
    swarmail: String,
    adapter: String,
    log: PathBuf,
    flags: Vec<String>,
}

fn wrapper_rig(trials: u32) -> Rig {
    let dir = home_scratch();
    let p = dir.path();
    let log = p.join("evals.log");
    let evals = script(
        &p.join("swarmail-evals"),
        r#"echo "$@" >> "$FAKE_LOG"
case "$1" in
  comparison-check)
    if [ -n "$FAKE_REFUSE" ]; then echo "swarmail-evals: comparison refused: $FAKE_REFUSE" >&2; exit 2; fi
    echo "comparison-check: the fake verifier ran in mode $3"; exit 0 ;;
  matrix) echo '{"fake":"row"}'; exit 0 ;;
esac
exit 0
"#,
    );
    let swarmail = script(&p.join("swarmail"), "exit 0\n");
    let adapter = script(&p.join("adapter"), "exit 0\n");
    let product = p.join("product.json");
    std::fs::write(
        &product,
        format!(
            "{{\n  \"id\": \"synthetic-one-trial\",\n  \"trials\": {trials},\n  \"planned_cells\": 48,\n  \"targets\": [\n    {{\n      \"lock_entry\": null\n    }},\n    {{\n      \"lock_entry\": \"mailpit\"\n    }},\n    {{\n      \"lock_entry\": \"mailhog\"\n    }},\n    {{\n      \"lock_entry\": \"mailslurper-source-build\"\n    }}\n  ]\n}}\n"
        ),
    )
    .unwrap();
    let mut flags = vec![];
    for (flag, name) in [
        ("--baseline", "baseline"),
        ("--contracts", "contracts"),
        ("--controls-dir", "controls"),
        ("--receipts", "receipts"),
        ("--cost", "cost"),
        ("--identity", "identity"),
    ] {
        let f = p.join(name);
        std::fs::write(&f, b"x").unwrap();
        flags.extend([flag.to_owned(), f.display().to_string()]);
    }
    flags.extend(["--product".into(), product.display().to_string()]);
    // main's grammar: one `--bin LOCK-ENTRY=PATH` per competitor lock entry of
    // the product record (the MailSlurper built from source is its own row).
    for name in ["mailpit", "mailhog", "mailslurper-source-build"] {
        flags.extend([
            "--bin".into(),
            format!("{name}={}", script(&p.join(name), "exit 0\n")),
        ]);
    }
    std::fs::write(p.join("config.json"), r#"[{"label":"null","argv":[]}]"#).unwrap();
    Rig {
        dir,
        evals,
        swarmail,
        adapter,
        log,
        flags,
    }
}

impl Rig {
    fn run(&self, mode_args: &[&str], extra: &[&str], refuse: &str) -> (i32, String) {
        let out = self.dir.path().join(format!(
            "out-{}",
            std::fs::read_dir(self.dir.path()).unwrap().count()
        ));
        let o = out.display().to_string();
        let config = self.dir.path().join("config.json").display().to_string();
        let mut args: Vec<&str> = mode_args.to_vec();
        args.extend(["--output", &o]);
        if mode_args[0] == "measure" {
            args.extend(["--config", &config]);
        }
        args.extend(extra);
        let flags: Vec<&str> = self.flags.iter().map(String::as_str).collect();
        args.extend(&flags);
        compare_mail_env(
            &args,
            self.dir.path(),
            &[
                ("COMPARE_MAIL_EVALS_BIN", &self.evals),
                ("COMPARE_MAIL_SWARMAIL_BIN", &self.swarmail),
                ("COMPARE_MAIL_ADAPTER_BIN", &self.adapter),
                ("COMPARE_MAIL_EVAL_SUITE", &self.swarmail),
                ("FAKE_LOG", &self.log.display().to_string()),
                ("FAKE_REFUSE", refuse),
            ],
        )
    }

    fn log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }
}

#[test]
fn the_comparison_integration_refusal_is_gone_and_a_dry_run_runs_only_the_verifier() {
    let script_text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/compare-mail.sh"),
    )
    .unwrap();
    assert_eq!(script_text.matches("comparison-integration").count(), 0);
    let rig = wrapper_rig(1);
    // The configuration is unreadable and holds a canary that must never appear.
    let config = rig.dir.path().join("config.json");
    std::fs::write(&config, b"CANARY-credential-zzz").unwrap();
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (code, text) = rig.run(&["measure", "--dry-run"], &[], "");
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("the fake verifier ran in mode dry-run"),
        "{text}"
    );
    assert!(
        !text.contains("CANARY") && !text.contains("Permission denied"),
        "{text}"
    );
    // The only process the wrapper started is the verifier: no matrix, no
    // contract probe, no index, no agent.
    let log = rig.log();
    assert_eq!(log.lines().count(), 1, "{log}");
    assert!(log.starts_with("comparison-check --mode dry-run "), "{log}");
    assert!(!rig.dir.path().join("launched").exists());
    // A refusal by the verifier is relayed by the prerequisite it names.
    let (code, text) = rig.run(
        &["measure", "--dry-run"],
        &[],
        "prerequisite-controls: the controls hold no rows for mailhog",
    );
    assert_eq!(code, 2);
    assert!(
        text.contains("blocked at phase \"prerequisite-controls\"")
            && text.contains("no rows for mailhog"),
        "{text}"
    );
}

#[test]
fn an_eval_crate_that_does_not_build_is_refused_as_eval_build() {
    let rig = wrapper_rig(1);
    // A `cargo` that fails, first on PATH, and no prebuilt verifier named.
    let bin = rig.dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    script(
        &bin.join("cargo"),
        "echo 'error: could not compile' >&2\nexit 101\n",
    );
    let out = rig.dir.path().join("out-build");
    let o = out.display().to_string();
    let config = rig.dir.path().join("config.json").display().to_string();
    let flags: Vec<&str> = rig.flags.iter().map(String::as_str).collect();
    let mut args = vec!["measure", "--dry-run", "--output", &o, "--config", &config];
    args.extend(&flags);
    let (code, text) = compare_mail_env(
        &args,
        rig.dir.path(),
        &[("FAKE_LOG", &rig.log.display().to_string())],
    );
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("blocked at phase \"eval-build\"") && text.contains("does not build"),
        "{text}"
    );
    assert!(
        rig.log().is_empty(),
        "the verifier never ran on a crate that does not build"
    );
}

#[test]
fn a_paid_measure_needs_the_verified_prerequisites_and_the_authorized_product() {
    let rig = wrapper_rig(1);
    // Without --authorized-product, with a wrong one: refused by name, and no
    // matrix, no agent.
    let (code, text) = rig.run(&["measure"], &[], "");
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("blocked at phase \"authorization\"") && text.contains("synthetic-one-trial"),
        "{text}"
    );
    let (code, text) = rig.run(
        &["measure"],
        &["--authorized-product", "another-product"],
        "",
    );
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("blocked at phase \"authorization\"") && text.contains("is not the product"),
        "{text}"
    );
    // A pending item (the verifier refuses it in measure mode) is refused by name.
    let (code, text) = rig.run(
        &["measure"],
        &["--authorized-product", "synthetic-one-trial"],
        "prerequisite-coordination-receipts: ci: the required CI run has not been supplied",
    );
    assert_eq!(code, 2);
    assert!(
        text.contains("\"prerequisite-coordination-receipts\""),
        "{text}"
    );
    assert!(
        !rig.log().contains("matrix "),
        "no agent matrix was delegated: {}",
        rig.log()
    );
    assert!(!rig.dir.path().join("launched").exists());
    // All three hold: the delegation runs the four targets in product order,
    // one matrix each, with the inert null config and the product's trials.
    let (code, text) = rig.run(
        &["measure"],
        &["--authorized-product", "synthetic-one-trial"],
        "",
    );
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    let matrices: Vec<&str> = log.lines().filter(|l| l.starts_with("matrix ")).collect();
    assert_eq!(matrices.len(), 4, "{log}");
    for (line, target) in matrices
        .iter()
        .zip(["swarmail", "mailpit", "mailhog", "mailslurper"])
    {
        assert!(
            line.starts_with(&format!("matrix --target {target} --product ")),
            "{line}"
        );
        assert!(
            line.contains("--tier hard") && line.contains("--trials 1 "),
            "{line}"
        );
        assert_eq!(line.contains("--adapter "), target != "swarmail", "{line}");
        assert_eq!(
            line.contains("--target-bin "),
            target != "swarmail",
            "{line}"
        );
    }
    assert!(
        log.lines()
            .next()
            .unwrap()
            .starts_with("comparison-check --mode measure "),
        "{log}"
    );
    assert!(
        !rig.dir.path().join("launched").exists(),
        "no agent CLI ran"
    );
}

#[test]
fn controls_run_each_target_as_a_named_timed_command_and_index_them() {
    let rig = wrapper_rig(1);
    let (code, text) = rig.run(&["controls"], &[], "");
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("120 rows, 4 targets"), "{text}");
    let log = rig.log();
    for t in ["swarmail", "mailpit", "mailhog", "mailslurper"] {
        assert!(
            log.contains(&format!("comparison-contract --target {t} ")),
            "{t}: {log}"
        );
        assert!(log.contains(&format!("matrix --target {t} ")), "{t}: {log}");
    }
    for line in log.lines().filter(|l| l.starts_with("matrix ")) {
        assert!(
            line.contains("--tier hard") && line.contains("--trials 5 "),
            "{line}"
        );
    }
    // The index is written once, after every target, with the suite's status.
    let index: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with("comparison-index "))
        .collect();
    assert_eq!(index.len(), 1, "{log}");
    assert!(
        index[0].contains("--eval-suite-status pass"),
        "{}",
        index[0]
    );
    assert!(!rig.dir.path().join("launched").exists());
    // A suite that did not pass is recorded as failed, and controls refuse it.
    let failing = script(&rig.dir.path().join("failing-suite"), "exit 1\n");
    let out = rig.dir.path().join("out-failing");
    let o = out.display().to_string();
    let flags: Vec<&str> = rig.flags.iter().map(String::as_str).collect();
    let mut args = vec!["controls", "--output", &o];
    args.extend(&flags);
    let (code, text) = compare_mail_env(
        &args,
        rig.dir.path(),
        &[
            ("COMPARE_MAIL_EVALS_BIN", &rig.evals),
            ("COMPARE_MAIL_SWARMAIL_BIN", &rig.swarmail),
            ("COMPARE_MAIL_ADAPTER_BIN", &rig.adapter),
            ("COMPARE_MAIL_EVAL_SUITE", &failing),
            ("FAKE_LOG", &rig.log.display().to_string()),
            ("FAKE_REFUSE", ""),
        ],
    );
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("blocked at phase \"eval-suite\""), "{text}");
    assert!(
        rig.log().contains("--eval-suite-status fail"),
        "{}",
        rig.log()
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
        Mcp::start_backend::<mailpit_mcp::Mailpit>(addr, bounds, out_buf)
    }

    fn start_backend<B: mcp::Backend>(
        addr: std::net::SocketAddr,
        bounds: mailpit_mcp::Bounds,
        out_buf: usize,
    ) -> Mcp {
        let (client_in, server_in) = tokio::io::duplex(1 << 20);
        let (server_out, client_out) = tokio::io::duplex(out_buf);
        let http = wire::Http::new(addr, bounds.request, bounds.max_body);
        let served = tokio::spawn(mcp::serve::<B, _, _>(server_in, server_out, http, bounds));
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
    assert_eq!(
        c.recv().await["error"]["code"],
        -32600,
        "an empty batch refuses"
    );
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
async fn the_advertised_wait_maximum_is_the_derived_one_and_nothing_longer_is_admitted() {
    let addr = responder(Box::new(|_| Some((200, page_json(0, 0, &[]))))).await;
    let bounds = mailpit_mcp::Bounds::default();
    let max = bounds.max_wait().as_millis() as u64;
    assert_eq!(max, 20_000);
    let mut c = Mcp::start(addr, bounds, 1 << 20);
    let r = c
        .request(
            1,
            "initialize",
            serde_json::json!({"protocolVersion": "2025-03-26"}),
        )
        .await;
    assert!(r.get("result").is_some(), "{r}");
    let r = c.request(2, "tools/list", serde_json::json!({})).await;
    let wait = &r["result"]["tools"][3];
    assert_eq!(wait["name"], "mailpit_wait");
    assert_eq!(
        wait["inputSchema"]["properties"]["timeout_ms"]["maximum"], 20_000,
        "the schema carries the derived maximum"
    );
    let refused = c
        .request(
            3,
            "tools/call",
            serde_json::json!({"name": "mailpit_wait",
                "arguments": {"query": "q", "timeout_ms": max + 1}}),
        )
        .await;
    assert_eq!(refused["error"]["code"], -32602, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        "invalid tool name or arguments"
    );
    let admitted = c
        .request(
            4,
            "tools/call",
            serde_json::json!({"name": "mailpit_wait",
                "arguments": {"query": "q", "timeout_ms": 1}}),
        )
        .await;
    assert!(admitted.get("result").is_some(), "{admitted}");
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

/// `n` distinct summaries `prefix0..`, enough to fill a scan page.
fn many(prefix: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}{i}")).collect()
}

#[tokio::test]
async fn full_pages_isolate_duplicate_moving_short_and_overlong_scans() {
    let addr = responder(Box::new(|t: &str| {
        let q = |s: &str| t.contains(&format!("query={s}&"));
        let at = |s: u64| t.contains(&format!("&start={s}&"));
        let page = |matching: u64, start: u64, ids: Vec<String>| {
            let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
            (200, page_json(matching, start, &ids))
        };
        Some(match t {
            // A full first page, then an ID already seen.
            _ if q("dup") && at(0) => page(251, 0, many("a", 250)),
            _ if q("dup") && at(250) => page(251, 250, vec!["a7".into()]),
            // A full first page, then the matching count moves.
            _ if q("moving") && at(0) => page(300, 0, many("m", 250)),
            _ if q("moving") && at(250) => page(301, 250, many("n", 50)),
            // A short page before the last: the old fixed stride would
            // have asked for start=250 next and skipped two matches.
            _ if q("short") && at(0) => page(300, 0, many("s", 248)),
            _ if q("short") && at(250) => page(300, 250, many("t", 50)),
            // An empty page while matches remain makes no progress.
            _ if q("stall") && at(0) => page(300, 0, many("x", 250)),
            _ if q("stall") => page(300, 250, Vec::new()),
            // More full pages than one scan may read.
            _ if q("long") => {
                let start: u64 = t
                    .split("&start=")
                    .nth(1)
                    .unwrap()
                    .split('&')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                page(1000, start, many(&format!("l{start}-"), 250))
            }
            // Exactly two full pages: complete.
            _ if q("exact") && at(0) => page(500, 0, many("e", 250)),
            _ if q("exact") && at(250) => page(500, 250, many("f", 250)),
            _ => page(0, 0, Vec::new()),
        })
    }))
    .await;
    let bounds = mailpit_mcp::Bounds {
        max_body: 1 << 20,
        max_operation_bytes: 8 << 20,
        max_pages: 3,
        ..small_bounds()
    };
    let http = wire::Http::new(addr, bounds.request, bounds.max_body);
    let wait = |query: &str| mailpit_mcp::Call::Wait {
        query: query.into(),
        timeout_ms: 2000,
    };
    for (query, want) in [
        ("dup", mailpit_mcp::ToolError::Incomplete),
        ("moving", mailpit_mcp::ToolError::Incomplete),
        ("short", mailpit_mcp::ToolError::Incomplete),
        ("stall", mailpit_mcp::ToolError::Incomplete),
        ("long", mailpit_mcp::ToolError::Budget("pages per scan")),
    ] {
        let got =
            mailpit_mcp::run_call(&http, &bounds, &wait(query), tokio::time::Instant::now()).await;
        assert_eq!(got, Err(want), "{query}");
    }
    let v = mailpit_mcp::run_call(&http, &bounds, &wait("exact"), tokio::time::Instant::now())
        .await
        .unwrap();
    assert_eq!(v["status"], "found", "{v}");
    assert_eq!(v["messages"].as_array().unwrap().len(), 500);
    assert_eq!(
        (v["scans"].clone(), v["requests"].clone()),
        (1.into(), 2.into())
    );
}

#[tokio::test(start_paused = true)]
async fn a_call_that_outlived_its_ceiling_in_the_queue_sends_nothing() {
    let bounds = mailpit_mcp::Bounds {
        operation: Duration::from_secs(1),
        request: Duration::from_millis(200),
        ..small_bounds()
    };
    // Nothing listens here: the refusal must come before any request.
    let http = wire::Http::new(
        "127.0.0.1:9".parse().unwrap(),
        bounds.request,
        bounds.max_body,
    );
    let admitted = tokio::time::Instant::now();
    tokio::time::advance(bounds.operation).await;
    for call in [
        mailpit_mcp::Call::List { start: 0, limit: 1 },
        mailpit_mcp::Call::Read { id: "x".into() },
        mailpit_mcp::Call::Wait {
            query: "q".into(),
            timeout_ms: 500,
        },
    ] {
        let got = mailpit_mcp::run_call(&http, &bounds, &call, admitted).await;
        assert_eq!(
            got,
            Err(mailpit_mcp::ToolError::Budget("operation deadline")),
            "{call:?}"
        );
    }
    assert_eq!(http.requests(), 0, "an expired queued call issued HTTP");
}

/// A scripted scan for the controlled clock: each step takes its virtual
/// duration and then answers its matching count, or its error.
struct Scripted {
    steps: std::collections::VecDeque<(u64, Result<u64, mailpit_mcp::ToolError>)>,
    t0: tokio::time::Instant,
    starts: Vec<u64>,
}

impl mailpit_mcp::Scan for Scripted {
    async fn scan(&mut self) -> Result<(u64, Vec<serde_json::Value>), mailpit_mcp::ToolError> {
        self.starts
            .push((tokio::time::Instant::now() - self.t0).as_millis() as u64);
        let (ms, answer) = self.steps.pop_front().unwrap_or((0, Ok(0)));
        tokio::time::sleep(Duration::from_millis(ms)).await;
        answer.map(|n| {
            (
                n,
                (0..n)
                    .map(|i| serde_json::json!({"ID": format!("m{i}")}))
                    .collect(),
            )
        })
    }
    fn requests(&self) -> u32 {
        self.starts.len() as u32
    }
}

/// `(answer, elapsed ms, scan start offsets ms)` of one controlled wait.
async fn scripted_wait(
    wait_ms: u64,
    steps: &[(u64, Result<u64, mailpit_mcp::ToolError>)],
) -> (
    Result<serde_json::Value, mailpit_mcp::ToolError>,
    u64,
    Vec<u64>,
) {
    let bounds = mailpit_mcp::Bounds::default();
    let t0 = tokio::time::Instant::now();
    let frame = mailpit_mcp::Frame::new(t0, &bounds, Duration::from_millis(wait_ms));
    let mut s = Scripted {
        steps: steps.iter().copied().collect(),
        t0,
        starts: Vec::new(),
    };
    let got = mailpit_mcp::poll_wait(&mut s, frame, bounds.poll_interval).await;
    let elapsed = (tokio::time::Instant::now() - t0).as_millis() as u64;
    (got, elapsed, s.starts)
}

#[tokio::test(start_paused = true)]
async fn the_wait_frame_composes_one_ceiling_cutoff_and_response_allowance() {
    let b = mailpit_mcp::Bounds::default();
    let t0 = tokio::time::Instant::now();
    let s = Duration::from_secs;
    let frame = |w| mailpit_mcp::Frame::new(t0, &b, s(w));
    // The derived maximum is operation - request: the last scan always
    // gets the full request bound, and the ceiling is exactly met.
    assert_eq!(b.max_wait(), s(20));
    assert_eq!(
        frame(20),
        mailpit_mcp::Frame {
            ceiling: t0 + s(30),
            cutoff: t0 + s(20),
            response: t0 + s(30)
        }
    );
    assert_eq!(frame(1).response, t0 + s(11));
    assert_eq!(frame(0).ceiling, t0 + b.operation);
    // Every admitted millisecond keeps the full allowance, never a clamp.
    for ms in 1..=b.max_wait().as_millis() as u64 {
        let f = mailpit_mcp::Frame::new(t0, &b, Duration::from_millis(ms));
        assert!(f.cutoff <= f.ceiling && f.response <= f.ceiling, "{ms}");
        assert_eq!(f.response, f.cutoff + b.request, "{ms}");
    }
}

#[tokio::test(start_paused = true)]
#[should_panic(expected = "cannot keep a 10s response allowance inside 30s")]
async fn a_wait_beyond_the_derived_maximum_has_no_frame() {
    let b = mailpit_mcp::Bounds::default();
    let past = b.max_wait() + Duration::from_millis(1);
    mailpit_mcp::Frame::new(tokio::time::Instant::now(), &b, past);
}

#[test]
fn the_wire_driver_builds_no_frame_of_its_own() {
    // One constructor: `Frame::new` is the only place a frame's instants
    // are composed (spec clarification Q3), driver and adapter alike.
    for src in [
        "runner.rs",
        "stats.rs",
        "targets.rs",
        "wire.rs",
        "records.rs",
        "mailpit_mcp.rs",
        "mailhog_mcp.rs",
        "mailslurper_mcp.rs",
    ] {
        let text = std::fs::read_to_string(format!("benches/mail_compare/{src}")).unwrap();
        assert!(!text.contains("Frame {"), "{src} builds a frame by hand");
    }
    // The struct, its impl, the constructor's signature and its body:
    // all in the generic half, none in a backend.
    let text = std::fs::read_to_string("benches/mail_compare/mcp.rs").unwrap();
    assert_eq!(text.matches("Frame {").count(), 4);
}

#[tokio::test(start_paused = true)]
async fn the_poll_observes_until_the_exact_cutoff_and_never_past_the_ceiling() {
    use mailpit_mcp::ToolError;
    let status = |r: &Result<serde_json::Value, ToolError>| r.as_ref().unwrap()["status"].clone();

    // Present: the immediate scan answers.
    let (r, at, starts) = scripted_wait(1000, &[(0, Ok(2))]).await;
    assert_eq!((status(&r), at, starts), ("found".into(), 0, vec![0]));
    assert_eq!(r.unwrap()["messages_count"], 2);

    // Empty: a scan at once and 100 ms after each, timeout exactly at the
    // cutoff, never at the last scan.
    let (r, at, starts) = scripted_wait(1000, &[]).await;
    assert_eq!(status(&r), "timeout");
    assert_eq!(at, 1000);
    assert_eq!(starts, (0..10).map(|i| i * 100).collect::<Vec<_>>());
    let v = r.unwrap();
    assert_eq!(
        (v["scans"].clone(), v["requests"].clone()),
        (10.into(), 10.into())
    );

    // The last partial interval: 30 ms scans start every 130 ms; the
    // last ends at 940, the next would start after the cutoff, and the
    // answer still waits for the cutoff instead of returning early.
    let (r, at, starts) = scripted_wait(1000, &[(30, Ok(0)); 8]).await;
    assert_eq!((status(&r), at), ("timeout".into(), 1000));
    assert_eq!(starts, (0..8).map(|i| i * 130).collect::<Vec<_>>());

    // A scan that ends exactly at the cutoff counts; one that ends a
    // millisecond later is late, kept and censored.
    let (r, at, _) = scripted_wait(1000, &[(0, Ok(0)), (900, Ok(1))]).await;
    assert_eq!((status(&r), at), ("found".into(), 1000));
    let (r, at, _) = scripted_wait(1000, &[(0, Ok(0)), (901, Ok(1))]).await;
    assert_eq!((status(&r), at), ("late".into(), 1001));
    assert_eq!(r.unwrap()["messages"].as_array().unwrap().len(), 1);

    // A scan that started in time and ended later with no match is a
    // complete observation: timeout, answered when it ended.
    let (r, at, starts) = scripted_wait(1000, &[(0, Ok(0)), (950, Ok(0))]).await;
    assert_eq!(
        (status(&r), at, starts),
        ("timeout".into(), 1050, vec![0, 100])
    );

    // No scan starts at or after the cutoff.
    let (r, at, starts) = scripted_wait(250, &[]).await;
    assert_eq!(
        (status(&r), at, starts),
        ("timeout".into(), 250, vec![0, 100, 200])
    );

    // A scan still running when the response allowance ends is
    // incomplete: an error at cutoff + request bound, not a timeout.
    let (r, at, _) = scripted_wait(1000, &[(0, Ok(0)), (20_000, Ok(1))]).await;
    assert_eq!(
        (r, at),
        (Err(ToolError::Budget("operation deadline")), 11_000)
    );

    // At the derived 20 s maximum the last scan still gets its full
    // request bound: a match first seen after the cutoff is `late`, and
    // a scan still running at the 30 s ceiling is an incomplete error.
    let (r, at, starts) = scripted_wait(20_000, &[(0, Ok(0)), (19_950, Ok(1))]).await;
    assert_eq!(
        (status(&r), at, starts),
        ("late".into(), 20_050, vec![0, 100])
    );
    let (r, at, starts) = scripted_wait(20_000, &[(0, Ok(0)), (30_000, Ok(1))]).await;
    assert_eq!(
        (r, at, starts),
        (
            Err(ToolError::Budget("operation deadline")),
            30_000,
            vec![0, 100]
        )
    );

    // A failed scan is an error at once, never an early timeout.
    let (r, at, _) = scripted_wait(1000, &[(0, Ok(0)), (5, Err(ToolError::Incomplete))]).await;
    assert_eq!((r, at), (Err(ToolError::Incomplete), 105));
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
    // The same name main's real-binary tests use: the source-built
    // MailSlurper is `MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD`, and
    // `MAIL_COMPARE_MAILSLURPER` stays the official release's.
    let var = format!("MAIL_COMPARE_{}", t.name().to_uppercase().replace('-', "_"));
    let binary = std::env::var_os(&var)
        .unwrap_or_else(|| panic!("{var} must name the pinned {t} executable"));
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    let pin = lock.pin(t).unwrap();
    targets::LaunchSpec::from_pin(t, binary.into(), None, pin).unwrap()
}

async fn launch_pinned(t: TargetKind, scratch: &Path) -> targets::Owned {
    let clock = wire::Clock(Instant::now());
    targets::Owned::launch(
        &pinned(t),
        scratch,
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
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
    let r = targets::Owned::launch(
        &spec,
        &root.path().join("a"),
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await;
    assert!(matches!(r, Err(targets::LaunchError::DigestMismatch)));
    spec.binary = root.path().join("absent");
    let r = targets::Owned::launch(
        &spec,
        &root.path().join("b"),
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await;
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

/// The protocol's storage facts (version 2) read from the pinned
/// executables: a finite cap that is really applied and what it evicts, and
/// the absence of one within a bounded probe. A probe states what it saw,
/// never that no limit exists.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs MAIL_COMPARE_MAILPIT and MAIL_COMPARE_MAILHOG (pinned executables)"]
async fn real_competitor_caps_are_probed_against_the_declared_storage_facts() {
    use protocol::MessageKind;
    let kinds = [
        MessageKind::Plain,
        MessageKind::Html,
        MessageKind::Multipart,
    ];
    let p = Protocol::parse(PROTOCOL).unwrap();
    let declared = |t: TargetKind| {
        p.storage
            .retention
            .iter()
            .find(|r| r.target == t)
            .unwrap()
            .clone()
    };

    // Mailpit: the launch's cap flag is `--max`; launch with a small one
    // and send more than it.
    let mailpit = declared(TargetKind::Mailpit);
    let mut spec = pinned(TargetKind::Mailpit);
    let at = spec.template.iter().position(|a| a == "--max").unwrap();
    assert_eq!(
        spec.template[at + 1],
        mailpit.max_messages.unwrap().to_string()
    );
    spec.template[at + 1] = "5".into();
    let root = tempfile::tempdir().unwrap();
    let clock = wire::Clock(Instant::now());
    let mut owned = targets::Owned::launch(
        &spec,
        &root.path().join("s"),
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await
    .unwrap();
    let corpus: Vec<wire::Message> = (0..8)
        .map(|i| wire::authored(163, "capprobe0001", i, &kinds))
        .collect();
    for m in &corpus {
        send_authored(&owned, m).await;
    }
    // The pinned Mailpit did not evict at once: for a bounded window the
    // listing is whatever it holds, and the fact stated is that eviction
    // is not synchronous with acceptance, so a workload must never reach
    // the cap, never that eviction timing or order is established. If it
    // does evict inside the window, only a prefix of the oldest may go.
    let window = Instant::now();
    let kept = loop {
        let kept = owned.list_all(8).await.unwrap();
        if kept.len() < 8 || window.elapsed() > Duration::from_secs(20) {
            break kept;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    let tokens: Vec<&str> = kept.iter().map(|(t, _)| t.as_str()).collect();
    assert!(tokens.len() >= 5, "never below the cap: {tokens:?}");
    for m in &corpus[3..] {
        assert!(
            tokens.contains(&m.token.as_str()),
            "newest kept: {tokens:?}"
        );
    }
    let evicted = 8 - tokens.len();
    for m in &corpus[..evicted] {
        assert!(
            !tokens.contains(&m.token.as_str()),
            "oldest evicted: {tokens:?}"
        );
    }
    for m in &corpus[evicted..] {
        assert!(tokens.contains(&m.token.as_str()), "no gap: {tokens:?}");
    }
    assert!(owned.shutdown().await.proved());

    // MailHog: no cap flag exists, and 1,200 messages (more than any
    // default seen in the help or the setup document) all stay listed.
    // This bounds what was observed; it does not prove unlimited storage.
    let hog = declared(TargetKind::Mailhog);
    assert_eq!(hog.max_messages, None);
    let owned = launch_pinned(TargetKind::Mailhog, &root.path().join("h")).await;
    let mut owned = owned;
    for i in 0..1200 {
        send_authored(&owned, &wire::authored(163, "capprobe0002", i, &kinds)).await;
    }
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let n = owned.list_all(8).await.map(|l| l.len());
        if matches!(n, Ok(1200)) {
            break;
        }
        assert!(Instant::now() < until, "MailHog never listed 1200: {n:?}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(owned.shutdown().await.proved());
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
        keepers: targets::Keepers::default(),
    };
    std::fs::create_dir(root.path().join("s")).unwrap();
    let (verdict, life, sealed) = runner::run(cfg, wire::Clock(Instant::now())).await;
    assert_eq!(verdict, runner::Verdict::Clean, "{verdict:?}");
    assert!(life.cleanups.iter().all(targets::CleanupFacts::proved));
    let launched: std::collections::BTreeSet<_> = life.launches.iter().map(|l| l.target).collect();
    assert_eq!(launched, TargetKind::V2.into());
    let dir = sealed.expect("a clean run is sealed");
    records::admit(&dir).expect("the sealed run is admitted");
    report::admit_run(&dir, &serde_json::to_vec(&p).unwrap())
        .expect("it admits against the protocol that planned it");
    assert_cells_match_rows(&dir);
    let rows = rows(&dir);
    for t in TargetKind::V2 {
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

// ---- the generic adapter half: batches (proposed decision 0129 §6) ---------

/// An upstream that counts every request it receives and answers each with
/// an empty page; a target holding `hang` is never answered.
async fn counting_upstream(
    hang: &'static str,
) -> (
    std::net::SocketAddr,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = seen.clone();
    let addr = responder(Box::new(move |t: &str| {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if !hang.is_empty() && t.contains(hang) {
            return None;
        }
        Some((200, page_json(0, 0, &[])))
    }))
    .await;
    (addr, seen)
}

fn upstream_requests(seen: &std::sync::atomic::AtomicUsize) -> usize {
    seen.load(std::sync::atomic::Ordering::SeqCst)
}

fn list_member(id: i64) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": "mailpit_list", "arguments": {"limit": 1}}})
}

async fn initialized(addr: std::net::SocketAddr, bounds: mailpit_mcp::Bounds) -> Mcp {
    let mut c = Mcp::start(addr, bounds, 1 << 20);
    c.init().await;
    c
}

#[tokio::test]
async fn a_batch_of_one_and_of_sixteen_is_answered_with_one_ordered_array() {
    let (addr, seen) = counting_upstream("").await;
    let bounds = mailpit_mcp::Bounds {
        max_line: 8192,
        ..small_bounds()
    };
    let mut c = initialized(addr, bounds).await;
    for n in [1usize, 16] {
        let before = upstream_requests(&seen);
        let batch: Vec<_> = (0..n as i64)
            .map(|i| match i % 3 {
                0 => list_member(100 + i),
                1 => serde_json::json!({"jsonrpc": "2.0", "id": 100 + i, "method": "ping"}),
                _ => serde_json::json!({"jsonrpc": "2.0", "id": format!("s{i}"),
                    "method": "tools/list"}),
            })
            .collect();
        c.send(serde_json::Value::Array(batch)).await;
        let r = c.recv().await;
        let replies = r.as_array().unwrap_or_else(|| panic!("one array, got {r}"));
        assert_eq!(replies.len(), n, "{r}");
        for (i, reply) in replies.iter().enumerate() {
            let want = match i % 3 {
                2 => serde_json::json!(format!("s{i}")),
                _ => serde_json::json!(100 + i as i64),
            };
            assert_eq!(reply["id"], want, "member order is kept: {r}");
        }
        let calls = (0..n).filter(|i| i % 3 == 0).count();
        assert_eq!(
            upstream_requests(&seen) - before,
            calls,
            "one request per call"
        );
    }
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
}

#[tokio::test]
async fn an_empty_or_oversized_batch_is_one_null_id_error_and_runs_nothing() {
    let (addr, seen) = counting_upstream("").await;
    let mut c = initialized(addr, small_bounds()).await;
    let seventeen: Vec<_> = (0..17).map(list_member).collect();
    for batch in [serde_json::json!([]), serde_json::Value::Array(seventeen)] {
        c.send(batch).await;
        let r = c.recv().await;
        assert!(r.is_object(), "one error object, not an array: {r}");
        assert!(r["id"].is_null(), "{r}");
        assert_eq!(r["error"]["code"], -32600, "{r}");
    }
    assert_eq!(upstream_requests(&seen), 0, "a refusal executes nothing");
    // The cap is the one Swarmail's envelope holds.
    assert_eq!(mcp::MAX_BATCH, swarmail::mcp::MCP_BATCH_CAP);
    let r = c.request(7, "ping", serde_json::json!({})).await;
    assert_eq!(r["result"], serde_json::json!({}));
}

#[tokio::test]
async fn a_notifications_only_batch_writes_nothing() {
    let (addr, seen) = counting_upstream("").await;
    let mut c = initialized(addr, small_bounds()).await;
    c.send(serde_json::json!([
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 5}}
    ]))
    .await;
    // The very next message written is the ping's: nothing came before it.
    let r = c.request(9, "ping", serde_json::json!({})).await;
    assert_eq!(r["result"], serde_json::json!({}));
    assert_eq!(upstream_requests(&seen), 0);
}

#[tokio::test]
async fn initialize_inside_a_batch_is_refused_and_duplicate_ids_are_answered_as_sent() {
    let (addr, seen) = counting_upstream("").await;
    let mut c = Mcp::start(addr, small_bounds(), 1 << 20);
    c.send(
        serde_json::json!([{"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-03-26"}}]),
    )
    .await;
    let r = c.recv().await;
    assert_eq!(r[0]["id"], 1, "{r}");
    assert_eq!(r[0]["error"]["code"], -32600, "{r}");
    // It did not initialize the session.
    let r = c.request(2, "tools/list", serde_json::json!({})).await;
    assert_eq!(r["error"]["code"], -32600, "{r}");
    c.init().await;
    c.send(serde_json::json!([list_member(5), list_member(5), 17, {"id": 6}]))
        .await;
    let r = c.recv().await;
    let r = r.as_array().unwrap();
    assert_eq!(r.len(), 4, "{r:?}");
    assert_eq!((&r[0]["id"], &r[1]["id"]), (&5.into(), &5.into()));
    assert!(r[0]["result"].is_object() && r[1]["result"].is_object());
    // Invalid members are answered where they stood, with a null id.
    assert_eq!(r[2]["error"]["code"], -32600);
    assert!(r[2]["id"].is_null());
    assert_eq!(r[3]["error"]["code"], -32600);
    assert_eq!(upstream_requests(&seen), 2, "both duplicates ran");
}

#[tokio::test]
async fn a_member_whose_reply_would_cross_the_reply_bound_gets_its_own_error() {
    // Each page is about 100 bytes of JSON per message; the bound leaves
    // room for one small reply and one large one is refused alone.
    let big: Vec<String> = (0..40).map(|i| format!("id{i:04}")).collect();
    let big_refs: Vec<&str> = big.iter().map(String::as_str).collect();
    let big_page = page_json(40, 0, &big_refs);
    let addr = responder(Box::new(move |t: &str| {
        Some(if t.ends_with("limit=2") {
            (200, big_page.clone())
        } else {
            (200, page_json(0, 0, &[]))
        })
    }))
    .await;
    let bounds = mailpit_mcp::Bounds {
        max_reply: 2600,
        ..small_bounds()
    };
    let mut c = initialized(addr, bounds).await;
    let large = serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "mailpit_list", "arguments": {"limit": 2}}});
    c.send(serde_json::json!([list_member(1), large, list_member(3)]))
        .await;
    let r = c.recv().await;
    let r = r.as_array().unwrap();
    assert_eq!(r.len(), 3, "{r:?}");
    let (e1, v1) = tool_result(&r[0]);
    let (e2, v2) = tool_result(&r[1]);
    assert!(!e1, "{v1}");
    assert!(e2, "the over-bound member alone is refused: {v2}");
    assert!(v2["error"].as_str().unwrap().starts_with("reply_too_large"));
    assert_eq!(r[1]["id"], 2);
    assert!(r[2]["result"].is_object(), "later members still run");
}

#[tokio::test]
async fn a_full_queue_refuses_every_request_member_and_runs_nothing() {
    let (addr, seen) = counting_upstream("start=0&limit=1").await;
    let bounds = mailpit_mcp::Bounds {
        max_pending: 1,
        ..small_bounds()
    };
    let mut c = initialized(addr, bounds).await;
    // One call holds the only slot: its upstream never answers.
    c.send(list_member(1)).await;
    let before = loop {
        let n = upstream_requests(&seen);
        if n > 0 {
            break n;
        }
        tokio::task::yield_now().await;
    };
    c.send(
        serde_json::json!([list_member(2), {"jsonrpc": "2.0", "id": 3, "method": "ping"},
        {"jsonrpc": "2.0", "method": "notifications/initialized"}]),
    )
    .await;
    let r = c.recv().await;
    let r = r.as_array().unwrap();
    assert_eq!(r.len(), 2, "{r:?}");
    for (reply, id) in r.iter().zip([2, 3]) {
        assert_eq!(reply["id"], id, "{r:?}");
        assert_eq!(reply["error"]["code"], -32000, "{r:?}");
    }
    assert_eq!(upstream_requests(&seen), before, "nothing ran");
    // A single call over the cap is refused the same way.
    c.send(list_member(4)).await;
    assert_eq!(c.recv().await["error"]["code"], -32000);
}

#[tokio::test]
async fn a_batch_line_over_the_line_bound_is_refused_and_the_next_line_still_works() {
    let (addr, seen) = counting_upstream("").await;
    // The default bound is 64 KiB.
    let mut c = initialized(addr, mailpit_mcp::Bounds::default()).await;
    let pad = " ".repeat(64 << 10);
    c.raw(format!("[{pad}{}]", list_member(1)).as_bytes()).await;
    let r = c.recv().await;
    assert_eq!(r["error"]["code"], -32600, "{r}");
    assert!(r["id"].is_null());
    let r = c.request(2, "ping", serde_json::json!({})).await;
    assert_eq!(r["result"], serde_json::json!({}));
    assert_eq!(upstream_requests(&seen), 0);
}

#[tokio::test]
async fn eof_in_the_middle_of_a_batch_ends_cleanly_without_a_reply() {
    let (addr, seen) = counting_upstream("limit=1").await;
    let mut c = initialized(addr, small_bounds()).await;
    c.send(serde_json::json!([list_member(1), list_member(2)]))
        .await;
    while upstream_requests(&seen) == 0 {
        tokio::task::yield_now().await;
    }
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    let mut rest = String::new();
    use tokio::io::AsyncReadExt;
    c.r.read_to_string(&mut rest).await.unwrap();
    assert!(rest.is_empty(), "no partial array was written: {rest:?}");
    assert_eq!(upstream_requests(&seen), 1, "the second member never ran");
}

#[tokio::test]
async fn cancelling_one_batch_member_cancels_only_that_member() {
    // Member 10's upstream request never answers; member 11's does.
    let addr = responder(Box::new(|t: &str| {
        if t.contains("start=7&") {
            None
        } else {
            Some((200, page_json(0, 0, &[])))
        }
    }))
    .await;
    let mut c = initialized(addr, small_bounds()).await;
    let held = serde_json::json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call",
        "params": {"name": "mailpit_list", "arguments": {"start": 7, "limit": 1}}});
    c.send(serde_json::json!([held, list_member(11), list_member(12)]))
        .await;
    // A member waiting its turn can be cancelled too.
    c.send(
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": {"requestId": 12}}),
    )
    .await;
    c.send(
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": {"requestId": 10}}),
    )
    .await;
    let r = c.recv().await;
    let r = r.as_array().unwrap();
    assert_eq!(r.len(), 1, "only the uncancelled member answers: {r:?}");
    assert_eq!(r[0]["id"], 11);
    assert!(r[0]["result"].is_object());
}

#[test]
fn the_mailpit_tool_list_digest_is_the_one_protocol_v2_froze() {
    let protocol: serde_json::Value =
        serde_json::from_slice(&std::fs::read("docs/evals/protocol.json").unwrap()).unwrap();
    let frozen = protocol["budgets"]["adapter"]["tools_sha256"]
        .as_str()
        .unwrap();
    let built = mcp::tools_sha256::<mailpit_mcp::Mailpit>(&mailpit_mcp::Bounds::default());
    assert_eq!(built, frozen, "the split changed Mailpit's tool list");
}

// ---- the MailHog adapter (proposed decision 0129 §5) -----------------------

/// MailHog's v2 page shape, as the pinned executable sends it.
fn hog_page(total: u64, start: u64, ids: &[&str]) -> String {
    let items: Vec<_> = ids
        .iter()
        .map(|i| {
            serde_json::json!({"ID": i, "Content": {"Headers": {"Subject": [format!("s {i}")]},
                "Body": "raw body"}, "Raw": {"Data": "Subject: x\r\n\r\nraw body"}})
        })
        .collect();
    serde_json::json!({"total": total, "count": ids.len(), "start": start, "items": items})
        .to_string()
}

fn no_swarmail_only_capability(tools: &serde_json::Value, upstream: &str) {
    for t in tools.as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        assert!(name.starts_with(upstream), "{name}");
        let words = format!("{name} {}", t["description"].as_str().unwrap()).to_lowercase();
        for banned in [
            "inbox", "await", "extract", "delete", "admin", "release", "send", "hint",
        ] {
            assert!(!words.contains(banned), "{name} offers {banned}: {words}");
        }
    }
}

#[test]
fn no_adapter_tool_offers_a_swarmail_only_capability() {
    let b = mailpit_mcp::Bounds::default();
    no_swarmail_only_capability(&mailpit_mcp::tools(&b), "mailpit_");
    no_swarmail_only_capability(&mailhog_mcp::tools(&b), "mailhog_");
    no_swarmail_only_capability(&mailslurper_mcp::tools(&b), "mailslurper_");
    let names = |v: serde_json::Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().replace("mailhog", "mailpit"))
            .collect()
    };
    // MailHog's read-by-id route binds to no source the lock holds, so its
    // adapter offers the other three; each of those matches Mailpit's.
    let mut mailpit_names = names(mailpit_mcp::tools(&b));
    mailpit_names.retain(|n| n != "mailpit_read");
    assert_eq!(
        names(mailhog_mcp::tools(&b)),
        mailpit_names,
        "the same operations as Mailpit's: list, search, wait (no read)"
    );
    let slurper: Vec<String> = mailslurper_mcp::tools(&b)
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            t["name"]
                .as_str()
                .unwrap()
                .replace("mailslurper", "mailpit")
        })
        .collect();
    assert_eq!(slurper, names(mailpit_mcp::tools(&b)));
}

#[test]
fn the_mailhog_adapter_validates_its_calls_closed() {
    let b = mailpit_mcp::Bounds::default();
    let call = |name: &str, args: serde_json::Value| {
        mailhog_mcp::parse_call(&serde_json::json!({"name": name, "arguments": args}), &b)
    };
    assert_eq!(
        call("mailhog_list", serde_json::json!({})),
        Some(mailhog_mcp::Call::List {
            start: 0,
            limit: 50
        })
    );
    assert!(call("mailhog_search", serde_json::json!({"query": "q"})).is_none());
    assert!(
        call(
            "mailhog_search",
            serde_json::json!({"kind": "all", "query": "q"})
        )
        .is_none()
    );
    assert!(
        call(
            "mailhog_search",
            serde_json::json!({"kind": "to", "query": ""})
        )
        .is_none()
    );
    assert!(call("mailhog_list", serde_json::json!({"limit": 251})).is_none());
    assert!(call("mailhog_list", serde_json::json!({"extra": 1})).is_none());
    // No read-by-id tool: MailHog's `/api/v1/messages/{id}` binds to no source
    // the lock holds (0092 §3), so the tool is neither offered nor parsed.
    assert!(call("mailhog_read", serde_json::json!({"id": "a"})).is_none());
    let tools = mailhog_mcp::tools(&b);
    let offered: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        offered,
        ["mailhog_list", "mailhog_search", "mailhog_wait"],
        "only operations a bound document or reviewed source states"
    );
    assert!(call("mailhog_delete", serde_json::json!({})).is_none());
    assert!(
        call(
            "mailhog_wait",
            serde_json::json!({"kind": "to", "query": "q"})
        )
        .is_none()
    );
    let too_long = b.max_wait_ms() + 1;
    assert!(
        call(
            "mailhog_wait",
            serde_json::json!({"kind": "to", "query": "q", "timeout_ms": too_long})
        )
        .is_none()
    );
    let wait = call(
        "mailhog_wait",
        serde_json::json!({"kind": "containing", "query": "q", "timeout_ms": 5}),
    );
    assert!(matches!(
        wait,
        Some(mailhog_mcp::Call::Wait { timeout_ms: 5, .. })
    ));
}

#[tokio::test]
async fn the_mailhog_adapter_reads_pages_and_refuses_wrong_shapes() {
    let addr = responder(Box::new(|t: &str| {
        Some(match t {
            "/api/v2/messages?start=0&limit=2" => (200, hog_page(2, 0, &["a", "b"])),
            "/api/v2/messages?start=0&limit=3" => (200, hog_page(2, 7, &["a"])),
            "/api/v2/messages?start=0&limit=4" => {
                (200, "{\"total\":0,\"start\":0,\"items\":null}".into())
            }
            "/api/v2/messages?start=0&limit=5" => {
                (200, "{\"total\":1,\"start\":0,\"items\":[{}]}".into())
            }
            "/api/v2/messages?start=0&limit=6" => (500, "x".into()),
            t if t.starts_with("/api/v2/search?kind=to&query=a%40b&") => {
                (200, hog_page(1, 0, &["a"]))
            }
            _ => (404, "{}".into()),
        })
    }))
    .await;
    let mut c = Mcp::start_backend::<mailhog_mcp::Mailhog>(addr, small_bounds(), 1 << 20);
    c.init().await;
    let (e, v) = c
        .tool(1, "mailhog_list", serde_json::json!({"limit": 2}))
        .await;
    assert!(!e, "{v}");
    assert_eq!(
        (v["total"].clone(), v["count"].clone()),
        (2.into(), 2.into())
    );
    assert_eq!(
        v["items"][1]["Raw"]["Data"], "Subject: x\r\n\r\nraw body",
        "bodies pass through"
    );
    for (limit, why) in [(3, "echoed start"), (5, "item without an ID")] {
        let (e, v) = c
            .tool(2, "mailhog_list", serde_json::json!({"limit": limit}))
            .await;
        assert!(
            e && v["error"].as_str().unwrap().starts_with("malformed"),
            "{why}: {v}"
        );
    }
    let (e, v) = c
        .tool(3, "mailhog_list", serde_json::json!({"limit": 4}))
        .await;
    assert!(
        !e && v["items"] == serde_json::json!([]),
        "a null item list is empty: {v}"
    );
    let (e, v) = c
        .tool(4, "mailhog_list", serde_json::json!({"limit": 6}))
        .await;
    assert!(
        e && v["error"].as_str().unwrap().starts_with("upstream_status"),
        "{v}"
    );
    let (e, v) = c
        .tool(
            5,
            "mailhog_search",
            serde_json::json!({"kind": "to", "query": "a@b"}),
        )
        .await;
    assert!(!e && v["total"] == 1, "{v}");
    // The route that is not offered draws the generic unknown-tool refusal.
    c.send(
        serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
        "params": {"name": "mailhog_read", "arguments": {"id": "a"}}}),
    )
    .await;
    let r = c.recv().await;
    assert_eq!(r["error"]["code"], -32602, "{r}");
    // The generic half answers a batch for this backend too.
    c.send(serde_json::json!([
        {"jsonrpc": "2.0", "id": 20, "method": "tools/call",
            "params": {"name": "mailhog_list", "arguments": {"limit": 2}}},
        {"jsonrpc": "2.0", "id": 21, "method": "tools/call",
            "params": {"name": "mailhog_list", "arguments": {"limit": 6}}}
    ]))
    .await;
    let r = c.recv().await;
    assert_eq!(r.as_array().unwrap().len(), 2, "{r}");
    assert_eq!(
        (r[0]["id"].clone(), r[1]["id"].clone()),
        (20.into(), 21.into())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_mailhog_adapter_lists_searches_and_waits_over_real_smtp_into_swarmails_shim() {
    // Mechanics only: Swarmail's own MailHog v2 shim is not MailHog, so
    // this proves the adapter's paging, filters and poll, never MailHog's
    // behaviour. The pinned executable's own test is ignored below.
    let root = tempfile::tempdir().unwrap();
    let clock = wire::Clock(Instant::now());
    let keepers = targets::Keepers::default();
    let mut owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &root.path().join("s"),
        &limits(),
        &clock,
        &keepers,
    )
    .await
    .expect("swarmail launches");
    for (i, subject) in ["hog-one", "hog-two", "hog-three"].iter().enumerate() {
        send_as(owned.smtp, None, subject, &format!("body {i}")).await;
    }
    let mut c = Mcp::start_backend::<mailhog_mcp::Mailhog>(
        owned.http_addr,
        mailpit_mcp::Bounds::default(),
        1 << 20,
    );
    c.init().await;
    let (e, listed) = c
        .tool(1, "mailhog_list", serde_json::json!({"limit": 2}))
        .await;
    assert!(!e, "{listed}");
    assert_eq!(
        (listed["total"].clone(), listed["count"].clone()),
        (3.into(), 2.into())
    );
    let (e, found) = c
        .tool(
            2,
            "mailhog_search",
            serde_json::json!({"kind": "containing", "query": "hog-two"}),
        )
        .await;
    assert!(!e && found["total"] == 1, "{found}");
    let (e, all) = c
        .tool(
            3,
            "mailhog_search",
            serde_json::json!({"kind": "to", "query": "inbox@identity.invalid"}),
        )
        .await;
    assert!(!e && all["total"] == 3, "{all}");
    let (e, hit) = c
        .tool(
            4,
            "mailhog_wait",
            serde_json::json!({"kind": "containing", "query": "hog-one", "timeout_ms": 5000}),
        )
        .await;
    assert!(!e, "{hit}");
    assert_eq!(hit["status"], "found");
    assert_eq!(hit["method"], mailhog_mcp::WAIT_METHOD);
    assert_eq!(hit["messages_count"], 1);
    let (e, miss) = c
        .tool(
            5,
            "mailhog_wait",
            serde_json::json!({"kind": "containing", "query": "never-sent", "timeout_ms": 300}),
        )
        .await;
    assert!(
        !e && miss["status"] == "timeout" && miss["messages_count"] == 0,
        "{miss}"
    );
    assert!(miss["scans"].as_u64().unwrap() >= 2, "it polled: {miss}");
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    assert!(owned.shutdown().await.proved());
}

// ---- launch mode: the adapter owns its backend (proposed 0129 §4) ----------

async fn launched_swarmail(root: &Path) -> (targets::Owned, targets::Keepers) {
    let keepers = targets::Keepers::default();
    let owned = targets::Owned::launch(
        &swarmail_spec(targets::swarmail_template(0)),
        &root.join("s"),
        &limits(),
        &wire::Clock(Instant::now()),
        &keepers,
    )
    .await
    .expect("swarmail launches");
    (owned, keepers)
}

/// Drive `serve_owned` over duplex pipes: the announcement line and the
/// reply to one ping come back, then the input closes.
async fn serve_launched(
    owned: &mut targets::Owned,
) -> (String, serde_json::Value, mcp::End, targets::CleanupFacts) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let (mut client_in, server_in) = tokio::io::duplex(1 << 16);
    let (server_out, client_out) = tokio::io::duplex(1 << 16);
    let serving = mcp::serve_owned::<mailpit_mcp::Mailpit, _, _>(
        owned,
        server_in,
        server_out,
        mailpit_mcp::Bounds::default(),
    );
    let client = async {
        let mut r = tokio::io::BufReader::new(client_out);
        let mut announce = String::new();
        r.read_line(&mut announce).await.unwrap();
        client_in
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
            .await
            .unwrap();
        let mut reply = String::new();
        r.read_line(&mut reply).await.unwrap();
        drop(client_in);
        (announce, serde_json::from_str(&reply).unwrap())
    };
    let ((end, facts), (announce, reply)) = tokio::join!(serving, client);
    (announce, reply, end, facts)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn launch_mode_announces_serves_and_gives_a_receipt_only_after_full_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let (mut owned, keepers) = launched_swarmail(root.path()).await;
    let smtp = owned.smtp;
    let (announce, reply, end, facts) = serve_launched(&mut owned).await;
    let want = format!(
        "smtp={smtp} tools_sha256={}\n",
        mcp::tools_sha256::<mailpit_mcp::Mailpit>(&mailpit_mcp::Bounds::default())
    );
    assert_eq!(announce, want, "one announcement line first");
    assert_eq!(reply["result"], serde_json::json!({}));
    assert_eq!(end, mcp::End::Eof);
    assert!(facts.proved(), "{facts:?}");
    assert!(mcp::receipt(end, &facts));
    let settled = keepers.settle(Duration::from_secs(10)).await;
    assert!(
        settled.iter().all(targets::CleanupFacts::proved),
        "{settled:?}"
    );
    assert!(!root.path().join("s").exists(), "the owned scratch is gone");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn launch_mode_is_not_a_receipt_when_cleanup_is_forced_to_fail() {
    let root = tempfile::tempdir().unwrap();
    let (mut owned, keepers) = launched_swarmail(root.path()).await;
    // The existing cleanup seam: the drain's join fails during cleanup.
    owned.fail_drain().await;
    let (_, _, end, facts) = serve_launched(&mut owned).await;
    assert_eq!(end, mcp::End::Eof, "the serving itself ended in order");
    assert!(!facts.proved(), "{facts:?}");
    assert!(
        !mcp::receipt(end, &facts),
        "no receipt without full cleanup"
    );
    drop(keepers);
}

#[test]
fn launch_mode_uses_the_deadlines_protocol_v2_froze() {
    let protocol: serde_json::Value =
        serde_json::from_slice(&std::fs::read("docs/evals/protocol.json").unwrap()).unwrap();
    let r = &protocol["resources"];
    let l = mcp::launch_limits();
    assert_eq!(l.startup.as_millis() as u64, r["startup_deadline_ms"]);
    assert_eq!(l.cleanup.as_millis() as u64, r["cleanup_deadline_ms"]);
    assert_eq!(l.request.as_millis() as u64, r["request_deadline_ms"]);
    assert_eq!(l.max_response_bytes, r["max_response_bytes"]);
    assert_eq!(l.open_files, r["open_files"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs MAIL_COMPARE_MAILHOG (the pinned MailHog executable)"]
async fn real_mailhog_through_the_adapter_lists_searches_and_waits() {
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_pinned(TargetKind::Mailhog, &root.path().join("s")).await;
    for (i, subject) in ["hogtoken-one", "hogtoken-two", "other-three"]
        .iter()
        .enumerate()
    {
        send_as(owned.smtp, None, subject, &format!("body {i}")).await;
    }
    let mut c = Mcp::start_backend::<mailhog_mcp::Mailhog>(
        owned.http_addr,
        mailpit_mcp::Bounds::default(),
        1 << 20,
    );
    c.init().await;
    let mut id = 100;
    let mut call = async |c: &mut Mcp, name: &str, args: serde_json::Value| {
        id += 1;
        let (is_error, v) = c.tool(id, name, args.clone()).await;
        assert!(!is_error, "{name} {args}: {v}");
        v
    };
    // Accept is not presumed to be storage: wait for all three to be listed.
    let until = Instant::now() + Duration::from_secs(10);
    while call(&mut c, "mailhog_list", serde_json::json!({"limit": 1})).await["total"] != 3 {
        assert!(Instant::now() < until, "MailHog never listed all three");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let listed = call(&mut c, "mailhog_list", serde_json::json!({"limit": 10})).await;
    assert_eq!(listed["count"], 3, "{listed}");
    assert!(
        listed["items"][0]["ID"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "{listed}"
    );
    let hits = call(
        &mut c,
        "mailhog_search",
        serde_json::json!({"kind": "containing", "query": "hogtoken"}),
    )
    .await;
    assert_eq!(hits["total"], 2, "{hits}");
    assert!(
        listed["items"][0]["Raw"]["Data"]
            .as_str()
            .unwrap()
            .contains("Subject: "),
        "raw MIME stays raw in a listed item"
    );
    let found = call(
        &mut c,
        "mailhog_wait",
        serde_json::json!({"kind": "containing", "query": "hogtoken-one", "timeout_ms": 5000}),
    )
    .await;
    assert_eq!(found["status"], "found", "{found}");
    let none = call(
        &mut c,
        "mailhog_wait",
        serde_json::json!({"kind": "containing", "query": "never-sent", "timeout_ms": 300}),
    )
    .await;
    assert_eq!(none["status"], "timeout", "{none}");
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    assert!(owned.shutdown().await.proved());
}

// ---- the adapter sources accepted 0130's lock does not bind (proposed 0129 §5) ----

/// The MailSlurper sources the adapter cites that accepted 0130's lock does
/// not bind, plus `go.mod` and `go.sum`: the committed path under
/// `docs/evals/agent-sources/mailslurper/`, the member of the release-1.15.0
/// source archive (`83f66f25…359f`) they were read from, and the sha256 of
/// both. `ServiceController.go` and `setupServicesListener.go` are main's
/// documents (`sources/mailslurper-source-build/`), so they are not here.
const AGENT_SOURCES: [(&str, &str, &str); 14] = [
    (
        "CommonQueries.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/CommonQueries.go",
        "090458bd89bb8767c755e140b22797ea8a7596b9e5a0770614cb285ec63d0db8",
    ),
    (
        "DataCommandExecutor.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/DataCommandExecutor.go",
        "bc6a64b05f5c64e2fcbd1ffac787eb74fb176089b577e520077657044c367929",
    ),
    (
        "DatabaseReceiver.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/DatabaseReceiver.go",
        "cc8e3a13abbbfec85c22ad80ff06800adc88f3269e4b541ba87fc63d8868f91f",
    ),
    (
        "HelloCommandExecutor.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/HelloCommandExecutor.go",
        "dc28f653b2ee41c09aaa6bd9a8c3fa5d2c98de672921ef7e443f6f4aaf95957d",
    ),
    (
        "MailCollectionResponse.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/MailCollectionResponse.go",
        "e9a110af272463ba22196a746ee3e0b1ade45abe1f79365ba01a4e9799613d83",
    ),
    (
        "MailItem.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/MailItem.go",
        "ad1d0b1ce06a2f315d93281612a8f64433e9ce211e0663fcc77308fbc9040ae0",
    ),
    (
        "MailSearch.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/MailSearch.go",
        "9d99312158e5b84f254c1162f0fbd26a5c98e4a363db460d97f1ddcd7b7bdd66",
    ),
    (
        "SMTPWorker.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/SMTPWorker.go",
        "1f4595b737e84c03d6fddf29f2bf3b85bd79c35ca4caec005e21739cfd8ff040",
    ),
    (
        "SMTPWorkerResponses.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/SMTPWorkerResponses.go",
        "fb8252f9be5a2c8564f1c5c194bc4ab2b00b465bc6796278ce8a3f8ca1d79029",
    ),
    (
        "SmtpCommands.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/SmtpCommands.go",
        "f1cff9896e72254a87facdf06bd6fc83903f0ec1e803b169937bc3ed168140b3",
    ),
    (
        "Writer.go",
        "mailslurper-release-1.15.0/pkg/mailslurper/Writer.go",
        "b856bf145795443fa47df467655d3e3641b7cb75fb35686de90a61597320f8ea",
    ),
    (
        "go.mod",
        "mailslurper-release-1.15.0/go.mod",
        "2d59cd3c66115ad2c5e4f84f9776cad156b70dcf0cfe967f21e543f06e9321a2",
    ),
    (
        "go.sum",
        "mailslurper-release-1.15.0/go.sum",
        "ca08ec830cad16bd373f5d66178827462fd26db321f7ad3e1b040a66024d140d",
    ),
    (
        "mailslurper.go",
        "mailslurper-release-1.15.0/cmd/mailslurper/mailslurper.go",
        "8978fb4a7ec7786dd3e6da8a6e5cec596c5a6b347188895fbaef2ef0fa487482",
    ),
];

/// The source archive the lock pins for both MailSlurper rows.
const MAILSLURPER_SOURCE_ARCHIVE: &str =
    "83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f";

fn agent_sources_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/evals/agent-sources/mailslurper")
}

#[test]
fn the_adapter_sources_main_does_not_bind_are_rehashed_against_the_table() {
    let dir = agent_sources_dir();
    // The directory holds exactly the table's files: nothing unbound sits
    // beside them and none is missing.
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = AGENT_SOURCES.iter().map(|r| r.0.to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
    for (path, member, want) in AGENT_SOURCES {
        let bytes = std::fs::read(dir.join(path)).unwrap();
        assert_eq!(targets::sha256_hex(&bytes), want, "{path} ({member})");
    }
    // The table's archive is the one the lock binds for both MailSlurper
    // rows, and none of its files is a document main already binds.
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    for t in [TargetKind::Mailslurper, TargetKind::MailslurperSourceBuild] {
        let pin = lock.pin(t).unwrap();
        assert!(
            pin.documents.iter().any(|d| matches!(
                &d.attribution,
                protocol::Fact::Observed(protocol::Attribution::Member { archive_sha256, .. })
                    if archive_sha256 == MAILSLURPER_SOURCE_ARCHIVE
            )),
            "{t} binds the source archive"
        );
        for (path, _, want) in AGENT_SOURCES {
            assert!(
                pin.documents.iter().all(|d| d.sha256 != want),
                "{path} is already bound by {t}'s lock row: delete the copy"
            );
        }
    }
}

#[test]
#[ignore = "needs MAIL_COMPARE_ARCHIVE_DIRS (a directory holding the pinned MailSlurper source archive)"]
fn the_adapter_sources_main_does_not_bind_are_members_of_the_pinned_archive() {
    // Main's own archive helpers: the archive is found by its digest, and
    // each member is re-extracted with the system `tar` into a fresh directory.
    let archive = eight_targets::archive_by_digest(MAILSLURPER_SOURCE_ARCHIVE);
    let dir = eight_targets::fresh_dir("agent-sources");
    for (path, member, want) in AGENT_SOURCES {
        let got = eight_targets::extract_member(&archive, member, dir.path());
        assert_eq!(targets::sha256_hex(&got), want, "{member} of {archive:?}");
        assert_eq!(
            std::fs::read(agent_sources_dir().join(path)).unwrap(),
            got,
            "{path} is the archive member's bytes"
        );
    }
}

// ---- the MailSlurper adapter (proposed decisions 0092 §3, 0129 §5) --------

fn slurper_item(id: &str, to: &str, subject: &str, body: &str) -> serde_json::Value {
    serde_json::json!({"id": id, "dateSent": "2026-11-07 10:00:00", "fromAddress": "a@b.invalid",
        "toAddresses": [to], "subject": subject, "xmailer": "", "mimeVersion": "1.0",
        "body": body, "contentType": "text/plain", "boundary": "", "attachments": [],
        "transferEncoding": ""})
}

fn slurper_page(total: u64, ids: &[&str]) -> String {
    let items: Vec<_> = ids
        .iter()
        .map(|i| slurper_item(i, "inbox@identity.invalid", &format!("s {i}"), "b"))
        .collect();
    serde_json::json!({"mailItems": items, "totalPages": total.div_ceil(50), "totalRecords": total})
        .to_string()
}

async fn initialized_for<B: mcp::Backend>(addr: std::net::SocketAddr) -> Mcp {
    let mut c = Mcp::start_backend::<B>(addr, mailpit_mcp::Bounds::default(), 1 << 20);
    c.init().await;
    c
}

#[test]
fn the_mailslurper_adapter_validates_its_calls_closed() {
    let b = mailpit_mcp::Bounds::default();
    let call = |name: &str, args: serde_json::Value| {
        mailslurper_mcp::parse_call(&serde_json::json!({"name": name, "arguments": args}), &b)
    };
    assert_eq!(
        call("mailslurper_list", serde_json::json!({})),
        Some(mailslurper_mcp::Call::List { page: 1 })
    );
    assert_eq!(
        call("mailslurper_list", serde_json::json!({"page": 3})),
        Some(mailslurper_mcp::Call::List { page: 3 })
    );
    // Pages are 1-based and bounded; nothing is clamped.
    assert!(call("mailslurper_list", serde_json::json!({"page": 0})).is_none());
    assert!(call("mailslurper_list", serde_json::json!({"page": 2001})).is_none());
    assert!(call("mailslurper_list", serde_json::json!({"limit": 10})).is_none());
    // A search needs one filter, and a blank term would silently stop
    // filtering upstream, so it refuses.
    assert!(call("mailslurper_search", serde_json::json!({})).is_none());
    assert!(call("mailslurper_search", serde_json::json!({"to": "  "})).is_none());
    assert!(call("mailslurper_search", serde_json::json!({"to": 7})).is_none());
    assert!(
        call(
            "mailslurper_search",
            serde_json::json!({"kind": "to", "to": "a"})
        )
        .is_none()
    );
    let over = "x".repeat(4097);
    assert!(call("mailslurper_search", serde_json::json!({"message": over})).is_none());
    let both = call(
        "mailslurper_search",
        serde_json::json!({"to": "a@b", "message": "m", "page": 2}),
    );
    assert!(matches!(
        both,
        Some(mailslurper_mcp::Call::Search { page: 2, ref filter })
            if filter.to.as_deref() == Some("a@b") && filter.from.is_none()
    ));
    assert!(call("mailslurper_read", serde_json::json!({"id": "a1-B_2.c"})).is_some());
    assert!(call("mailslurper_read", serde_json::json!({})).is_none());
    // What upstream serves that this adapter does not offer.
    for gone in [
        "mailslurper_delete",
        "mailslurper_prune",
        "mailslurper_attachment",
        "mailslurper_login",
        "mailslurper_version",
    ] {
        assert!(call(gone, serde_json::json!({})).is_none(), "{gone}");
    }
    assert!(call("mailslurper_wait", serde_json::json!({"to": "q"})).is_none());
    let late = serde_json::json!({"to": "q", "timeout_ms": b.max_wait_ms() + 1});
    assert!(call("mailslurper_wait", late).is_none());
    assert!(matches!(
        call(
            "mailslurper_wait",
            serde_json::json!({"message": "q", "timeout_ms": 5})
        ),
        Some(mailslurper_mcp::Call::Wait { timeout_ms: 5, .. })
    ));
}

#[test]
fn the_mailslurper_descriptions_state_the_like_substring_semantics_and_cite_their_routes() {
    let tools = mailslurper_mcp::tools(&mailpit_mcp::Bounds::default());
    let by = |n: &str| {
        tools
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == n)
            .unwrap_or_else(|| panic!("{n}"))
            .clone()
    };
    let search = by("mailslurper_search");
    let text = search["description"].as_str().unwrap();
    for must in [
        "SQL LIKE substring match",
        "wraps the term in % on both sides",
        "does not escape it",
        "a longer address that contains the term also matches",
        "wildcard",
    ] {
        assert!(text.contains(must), "{must}: {text}");
    }
    for filter in ["to", "from", "message"] {
        let d = search["inputSchema"]["properties"][filter]["description"]
            .as_str()
            .unwrap();
        assert!(d.contains("SQL LIKE substring match"), "{filter}: {d}");
    }
    for (name, route) in [
        ("mailslurper_list", "GET /mail"),
        ("mailslurper_search", "GET /mail"),
        ("mailslurper_read", "GET /mail/{id}"),
        ("mailslurper_wait", "GET /mail"),
    ] {
        let d = by(name)["description"].as_str().unwrap().to_owned();
        assert!(d.contains(route), "{name} names its route: {d}");
    }
    let wait = by("mailslurper_wait")["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(wait.contains("not a MailSlurper wait facility"));
}

#[tokio::test]
async fn the_mailslurper_adapter_reads_pages_and_refuses_wrong_shapes() {
    let big: Vec<String> = (0..51).map(|i| format!("m{i:02}")).collect();
    let first: Vec<&str> = big[..50].iter().map(String::as_str).collect();
    let page_one = slurper_page(51, &first);
    let addr = responder(Box::new(move |t: &str| {
        Some(match t {
            "/mail?pageNumber=1" => (200, page_one.clone()),
            "/mail?pageNumber=3" => (200, "{\"totalPages\":0,\"totalRecords\":0}".into()),
            "/mail?pageNumber=4" => (
                200,
                "{\"mailItems\":null,\"totalPages\":0,\"totalRecords\":0}".into(),
            ),
            "/mail?pageNumber=5" => (
                200,
                "{\"mailItems\":[{}],\"totalPages\":1,\"totalRecords\":1}".into(),
            ),
            "/mail?pageNumber=6" => (200, slurper_page(0, &["x"])),
            "/mail?pageNumber=7" => (200, "[]".into()),
            "/mail?pageNumber=8" => (500, "x".into()),
            "/mail?pageNumber=9" => (
                200,
                "{\"mailItems\":\"no\",\"totalPages\":1,\"totalRecords\":1}".into(),
            ),
            "/mail?pageNumber=1&to=ann" => (200, slurper_page(2, &["a", "b"])),
            "/mail?pageNumber=1&message=gone" => (200, slurper_page(0, &[])),
            "/mail/a1" => (
                200,
                slurper_item("a1", "inbox@identity.invalid", "s", "raw body").to_string(),
            ),
            "/mail/b2" => (500, "Problem getting mail item".into()),
            "/mail/c3" => (200, slurper_item("other", "x@y", "s", "b").to_string()),
            _ => (404, "{}".into()),
        })
    }))
    .await;
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(addr).await;
    let (e, v) = c.tool(1, "mailslurper_list", serde_json::json!({})).await;
    assert!(!e && v["totalRecords"] == 51 && v["mailItems"].as_array().unwrap().len() == 50);
    for (id, page) in [(2, 3), (3, 4)] {
        let (e, v) = c
            .tool(id, "mailslurper_list", serde_json::json!({"page": page}))
            .await;
        assert!(!e && v["mailItems"] == serde_json::json!([]), "{page}: {v}");
    }
    // Wrong shapes are typed errors, never panics or shorter successes.
    for (id, page) in [(4, 5), (5, 6), (6, 7), (7, 9)] {
        let (e, v) = c
            .tool(id, "mailslurper_list", serde_json::json!({"page": page}))
            .await;
        assert!(
            e && v["error"]
                .as_str()
                .is_some_and(|t| t.starts_with("malformed")),
            "{page}: {v}"
        );
    }
    let (e, v) = c
        .tool(8, "mailslurper_list", serde_json::json!({"page": 8}))
        .await;
    assert!(
        e && v["error"].as_str().unwrap().starts_with("upstream_status"),
        "{v}"
    );
    let (e, v) = c
        .tool(9, "mailslurper_search", serde_json::json!({"to": "ann"}))
        .await;
    assert!(!e && v["totalRecords"] == 2, "{v}");
    let (e, v) = c
        .tool(10, "mailslurper_read", serde_json::json!({"id": "a1"}))
        .await;
    assert!(
        !e && v["body"] == "raw body",
        "the body passes through: {v}"
    );
    let (e, v) = c
        .tool(11, "mailslurper_read", serde_json::json!({"id": "b2"}))
        .await;
    assert!(
        e && v["error"].as_str().unwrap().contains("HTTP 500"),
        "{v}"
    );
    let (e, v) = c
        .tool(12, "mailslurper_read", serde_json::json!({"id": "c3"}))
        .await;
    assert!(
        e && v["error"].as_str().unwrap().starts_with("malformed"),
        "{v}"
    );
    let (e, v) = c
        .tool(
            13,
            "mailslurper_wait",
            serde_json::json!({"message": "gone", "timeout_ms": 250}),
        )
        .await;
    assert!(!e && v["status"] == "timeout", "{v}");
    assert_eq!(v["method"], mailslurper_mcp::WAIT_METHOD);
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
}

#[tokio::test]
async fn a_mailslurper_scan_walks_every_page_of_a_filtered_search_before_it_answers() {
    let ids: Vec<String> = (0..51).map(|i| format!("s{i:02}")).collect();
    let first: Vec<&str> = ids[..50].iter().map(String::as_str).collect();
    let page_one = slurper_page(51, &first);
    let addr = responder(Box::new(move |t: &str| {
        Some(match t {
            "/mail?pageNumber=1&to=ann%40x.invalid" => (200, page_one.clone()),
            "/mail?pageNumber=2&to=ann%40x.invalid" => (200, slurper_page(51, &["s50"])),
            _ => (404, "{}".into()),
        })
    }))
    .await;
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(addr).await;
    let args = serde_json::json!({"to": "ann@x.invalid", "timeout_ms": 2000});
    let (e, v) = c.tool(1, "mailslurper_wait", args).await;
    assert!(!e, "{v}");
    assert_eq!(v["status"], "found");
    assert_eq!(v["messages_count"], 51);
    assert_eq!(v["messages"].as_array().unwrap().len(), 51);
    assert_eq!(v["requests"], 2, "one request per page: {v}");
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
}

/// A slot nobody holds: reserve a loopback port and release it at once. It
/// is a candidate, not a promise; between the release and the child's bind a
/// parallel test can take it.
fn reserve_and_release() -> std::net::SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

/// Launch Swarmail with its MailSlurper shim on a slot `choose` offers.
///
/// A slot that is reserved and released before the child binds it can be
/// taken in between by a test running in parallel (the coverage gate's
/// instrumented run did exactly that); Swarmail then exits at startup with
/// `NotReady(NO_LISTENER_ADDRESSES)`. A fresh slot is tried at most five
/// times, each refused launch must have proved its cleanup, and any other
/// launch error fails at once. Returns the slot, the owned launch and the
/// number of attempts it took. `scratch` holds one leaf per attempt.
async fn launch_shim_with_fresh_slot(
    scratch: &Path,
    mut choose: impl FnMut() -> std::net::SocketAddr,
) -> (std::net::SocketAddr, targets::Owned, u32) {
    const ATTEMPTS: u32 = 5;
    let keepers = targets::Keepers::default();
    let mut attempt = 0;
    loop {
        attempt += 1;
        let slot = choose();
        let mut template = targets::swarmail_template(0);
        template.extend(["--mailslurper-listen".into(), slot.to_string()]);
        match targets::Owned::launch(
            &swarmail_spec(template),
            &scratch.join(format!("a{attempt}")),
            &limits(),
            &wire::Clock(Instant::now()),
            &keepers,
        )
        .await
        {
            Ok(owned) => return (slot, owned, attempt),
            Err(targets::LaunchError::NotReady(targets::NO_LISTENER_ADDRESSES, facts)) => {
                // Every refused launch proves its cleanup, the last one too.
                assert!(
                    facts.proved(),
                    "a refused launch still cleans up: {facts:?}"
                );
                assert!(
                    attempt < ATTEMPTS,
                    "swarmail launches: {ATTEMPTS} slots refused in a row"
                );
            }
            Err(e) => panic!("swarmail launches: {e:?}"),
        }
    }
}

/// The first candidate is held by a live listener across the launch, as a
/// parallel test would hold it: the launch is refused with proved cleanup,
/// a later, fresh slot launches, and the holder is untouched.
#[tokio::test]
async fn a_slot_taken_by_another_listener_is_refused_cleanly_and_the_next_slot_launches() {
    let root = tempfile::tempdir().unwrap();
    let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let taken = held.local_addr().unwrap();
    let mut first = true;
    let (slot, mut owned, attempts) = launch_shim_with_fresh_slot(root.path(), || {
        if std::mem::take(&mut first) {
            taken
        } else {
            reserve_and_release()
        }
    })
    .await;
    // At least one refusal (the held slot) before a slot launched. The fresh
    // candidate is itself reserved and released, so a parallel test may
    // refuse it too: the count is a floor, never exact.
    assert!(attempts >= 2, "the held slot is refused first: {attempts}");
    assert_ne!(slot, taken);
    // The refused attempt's scratch is gone and the winner's shim is up.
    assert!(!root.path().join("a1").exists());
    assert!(owned.shutdown().await.proved());
    // The holder never lost its port to the refused child.
    let (_peer, _) = {
        let c = std::net::TcpStream::connect(taken).expect("the held listener still accepts");
        let accepted = held.accept().expect("and it accepted");
        (c, accepted)
    };
}

/// All five candidates held: five launches are refused, each with proved
/// cleanup, and the helper then fails loudly instead of retrying forever.
#[tokio::test]
#[should_panic(expected = "slots refused in a row")]
async fn five_taken_slots_exhaust_the_retry_and_fail_loudly() {
    let root = tempfile::tempdir().unwrap();
    let held: Vec<std::net::TcpListener> = (0..5)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    let drawn = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&drawn);
    let scratch = root.path().to_owned();
    let addrs: Vec<_> = held.iter().map(|l| l.local_addr().unwrap()).collect();
    // The helper runs in its own task so its panic can be inspected after
    // the count is read, then re-raised for `should_panic` to match.
    let ended = tokio::spawn(async move {
        launch_shim_with_fresh_slot(&scratch, || {
            addrs[counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)]
        })
        .await;
    })
    .await;
    assert_eq!(
        drawn.load(std::sync::atomic::Ordering::SeqCst),
        5,
        "exactly five candidates were offered"
    );
    std::panic::resume_unwind(ended.expect_err("the helper must not succeed").into_panic());
}

#[tokio::test]
async fn the_mailslurper_adapter_lists_and_reads_over_real_smtp_into_swarmails_shim() {
    // Mechanics only: Swarmail's own MailSlurper shim is not MailSlurper
    // (and refuses MailSlurper's filters), so this proves list paging and
    // read over a real exchange, never MailSlurper's behaviour.
    let root = tempfile::tempdir().unwrap();
    let (slot, mut owned, _attempts) =
        launch_shim_with_fresh_slot(root.path(), reserve_and_release).await;
    for (i, subject) in ["slurp-one", "slurp-two", "slurp-three"].iter().enumerate() {
        send_as(owned.smtp, None, subject, &format!("body {i}")).await;
    }
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(slot).await;
    let (e, listed) = c.tool(1, "mailslurper_list", serde_json::json!({})).await;
    assert!(!e, "{listed}");
    assert_eq!(listed["totalRecords"], 3, "{listed}");
    let items = listed["mailItems"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    let id = items[0]["id"].as_str().unwrap().to_owned();
    let (e, read) = c
        .tool(2, "mailslurper_read", serde_json::json!({"id": id}))
        .await;
    assert!(!e && read["id"] == serde_json::json!(id), "{read}");
    let to = read["toAddresses"].as_array().unwrap();
    assert!(to.iter().any(|a| a == "inbox@identity.invalid"), "{read}");
    let (e, empty) = c
        .tool(3, "mailslurper_list", serde_json::json!({"page": 2}))
        .await;
    assert!(!e && empty["totalRecords"] == 3, "{empty}");
    assert_eq!(empty["mailItems"], serde_json::json!([]), "{empty}");
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    assert!(owned.shutdown().await.proved());
}

/// The MailSlurper adapter drives main's `mailslurper-source-build` row: its
/// launch comes from the lock's pin through the builder the wire run uses.
#[test]
fn the_mailslurper_adapter_launches_main_s_source_build_pin() {
    assert_eq!(
        <mailslurper_mcp::Mailslurper as mcp::Backend>::TARGET,
        TargetKind::MailslurperSourceBuild
    );
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    let spec = mcp::launch_spec_for(
        <mailslurper_mcp::Mailslurper as mcp::Backend>::TARGET,
        PathBuf::from("/never/launched/mailslurper"),
        &lock,
    )
    .unwrap();
    assert_eq!(spec.kind, TargetKind::MailslurperSourceBuild);
    assert_eq!(
        spec.expected_sha256.as_deref(),
        Some("eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff")
    );
    assert_eq!(
        spec.extra.listeners,
        vec![
            protocol::ListenerRole::Smtp,
            protocol::ListenerRole::Http,
            protocol::ListenerRole::Ui
        ]
    );
    assert_eq!(spec.extra.config.as_ref().unwrap().name, "config.json");
    // It is the spec the wire run builds for the same row.
    let pin = lock.pin(TargetKind::MailslurperSourceBuild).unwrap();
    let wire_run = targets::LaunchSpec::from_pin(
        TargetKind::MailslurperSourceBuild,
        PathBuf::from("/never/launched/mailslurper"),
        None,
        pin,
    )
    .unwrap();
    assert_eq!(spec.template, wire_run.template);
    assert_eq!(spec.extra.env, wire_run.extra.env);
    // Mailpit and MailHog serve their own rows, unchanged.
    for (t, row) in [
        (
            <mailpit_mcp::Mailpit as mcp::Backend>::TARGET,
            TargetKind::Mailpit,
        ),
        (
            <mailhog_mcp::Mailhog as mcp::Backend>::TARGET,
            TargetKind::Mailhog,
        ),
    ] {
        assert_eq!(t, row);
        let spec = mcp::launch_spec_for(t, PathBuf::from("/never/launched"), &lock).unwrap();
        assert_eq!(spec.kind, row);
        assert!(spec.extra.config.is_none(), "{row}");
    }
}

/// An adapter entry point refuses every target but its three, by name,
/// before the lock is read: the official `mailslurper` row, which does not
/// start, is among the refused.
#[test]
fn an_adapter_refuses_a_target_that_is_not_its_own_by_name_and_spawns_nothing() {
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    for t in [
        TargetKind::Swarmail,
        TargetKind::Mailslurper,
        TargetKind::Smtp4dev,
        TargetKind::Inbucket,
        TargetKind::Greenmail,
        TargetKind::AlkemioMailslurper,
    ] {
        let why = mcp::adapter_target(t).unwrap_err();
        assert!(why.contains(t.name()), "{why}");
        let why = mcp::launch_spec_for(t, PathBuf::from("/never/launched"), &lock).unwrap_err();
        assert!(
            why.contains(t.name()) && why.contains("no MCP adapter"),
            "{why}"
        );
    }
    for t in [
        TargetKind::Mailpit,
        TargetKind::Mailhog,
        TargetKind::MailslurperSourceBuild,
    ] {
        mcp::adapter_target(t).unwrap();
    }
    // A lock that pins no row for an adapter's target refuses by name too.
    let mut bare = lock.clone();
    bare.entries
        .retain(|e| e.target != TargetKind::MailslurperSourceBuild);
    let why = mcp::launch_spec_for(
        TargetKind::MailslurperSourceBuild,
        PathBuf::from("/never/launched"),
        &bare,
    )
    .unwrap_err();
    assert!(why.contains("mailslurper-source-build"), "{why}");
}

/// Every hostile argument a tool refuses, per adapter: none makes a request.
#[tokio::test]
async fn hostile_arguments_never_reach_an_upstream_on_any_adapter() {
    let long_id = "a".repeat(129);
    let bad_ids: Vec<String> = [
        "a/b", "..", "a?x=1", "a%2e", "a b", "a\\b", "a\u{e9}", "a\nb", "",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .chain([long_id])
    .collect();
    let over = "x".repeat(4097);
    type Calls = Vec<(&'static str, serde_json::Value)>;
    let cases: Vec<(&str, Calls)> = vec![
        (
            "mailpit",
            vec![
                ("mailpit_list", serde_json::json!({"limit": 251})),
                ("mailpit_list", serde_json::json!({"start": 100001})),
                ("mailpit_search", serde_json::json!({"query": over})),
            ],
        ),
        (
            "mailhog",
            vec![
                ("mailhog_list", serde_json::json!({"limit": 251})),
                ("mailhog_list", serde_json::json!({"start": 100001})),
                (
                    "mailhog_search",
                    serde_json::json!({"kind": "to", "query": over}),
                ),
            ],
        ),
        (
            "mailslurper",
            vec![
                ("mailslurper_list", serde_json::json!({"page": 2001})),
                ("mailslurper_search", serde_json::json!({"to": over})),
                (
                    "mailslurper_wait",
                    serde_json::json!({"to": "x", "timeout_ms": 10_000_000}),
                ),
            ],
        ),
    ];
    for (adapter, over_cap) in cases {
        let (addr, seen) = counting_upstream("").await;
        let mut c = match adapter {
            "mailpit" => initialized_for::<mailpit_mcp::Mailpit>(addr).await,
            "mailhog" => initialized_for::<mailhog_mcp::Mailhog>(addr).await,
            _ => initialized_for::<mailslurper_mcp::Mailslurper>(addr).await,
        };
        let mut id = 1;
        let mut calls: Vec<(String, serde_json::Value)> = bad_ids
            .iter()
            .map(|b| (format!("{adapter}_read"), serde_json::json!({"id": b})))
            .collect();
        calls.extend(over_cap.into_iter().map(|(n, a)| (n.to_owned(), a)));
        for (name, args) in calls {
            id += 1;
            let params = serde_json::json!({"name": name, "arguments": args});
            let r = c.request(id, "tools/call", params).await;
            assert_eq!(r["error"]["code"], -32602, "{adapter} {name} {args}: {r}");
        }
        assert_eq!(upstream_requests(&seen), 0, "{adapter} made a request");
        // An accepted id reaches upstream once, as one path segment. MailHog's
        // adapter has no tool that takes an id: even a good id is refused.
        id += 1;
        let params = serde_json::json!({"name": format!("{adapter}_read"),
            "arguments": {"id": "ok-1_2.3"}});
        let r = c.request(id, "tools/call", params).await;
        if adapter == "mailhog" {
            assert_eq!(r["error"]["code"], -32602, "{r}");
            assert_eq!(upstream_requests(&seen), 0, "{adapter}");
        } else {
            assert_eq!(upstream_requests(&seen), 1, "{adapter}");
        }
        drop(c.w.take());
        assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    }
}

#[test]
fn an_accepted_id_is_path_encoded_and_the_alphabet_is_the_observed_one() {
    for ok in ["abc", "a-b_c.d", "A1", &"a".repeat(128)] {
        assert!(mcp::valid_id(ok, b""), "{ok}");
        assert_eq!(wire::query_value(ok), ok, "unreserved characters stay");
    }
    for bad in ["", "a/b", "a b", "a%2f", "é", &"a".repeat(129)] {
        assert!(!mcp::valid_id(bad, b""), "{bad}");
    }
    // A backend whose observed ids carry more says so, and is encoded.
    assert!(mcp::valid_id("a@b.example", b"@"));
    assert!(!mcp::valid_id("a@b.example", b""));
    assert_eq!(wire::query_value("a@b/c d"), "a%40b%2Fc%20d");
}

#[tokio::test]
async fn no_adapter_follows_a_redirect_and_an_unknown_shape_is_a_typed_error() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    // A loopback 302 to a second upstream that counts what reaches it.
    let (target, seen) = counting_upstream("").await;
    let redirect = format!("http://{target}/mail?pageNumber=1");
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            let redirect = redirect.clone();
            tokio::spawn(async move {
                let mut b = [0u8; 1024];
                let _ = s.read(&mut b).await;
                let resp = format!(
                    "HTTP/1.1 302 Found\r\nLocation: {redirect}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = s.write_all(resp.as_bytes()).await;
            });
        }
    });
    let mut c = initialized_for::<mailpit_mcp::Mailpit>(addr).await;
    let (e, v) = c.tool(1, "mailpit_list", serde_json::json!({})).await;
    assert!(
        e && v["error"].as_str().unwrap().contains("HTTP 302"),
        "{v}"
    );
    let mut c = initialized_for::<mailhog_mcp::Mailhog>(addr).await;
    let (e, v) = c.tool(1, "mailhog_list", serde_json::json!({})).await;
    assert!(
        e && v["error"].as_str().unwrap().contains("HTTP 302"),
        "{v}"
    );
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(addr).await;
    let (e, v) = c.tool(1, "mailslurper_list", serde_json::json!({})).await;
    assert!(
        e && v["error"].as_str().unwrap().contains("HTTP 302"),
        "{v}"
    );
    assert_eq!(upstream_requests(&seen), 0, "a redirect was followed");
    // An unknown upstream shape is a typed error on each.
    let addr = responder(Box::new(|_| Some((200, "{\"unexpected\":true}".into())))).await;
    let mut c = initialized_for::<mailpit_mcp::Mailpit>(addr).await;
    let r = c.tool(1, "mailpit_list", serde_json::json!({})).await;
    assert!(r.0 && r.1["error"].as_str().unwrap().starts_with("malformed"));
    let mut c = initialized_for::<mailhog_mcp::Mailhog>(addr).await;
    let r = c.tool(1, "mailhog_list", serde_json::json!({})).await;
    assert!(r.0 && r.1["error"].as_str().unwrap().starts_with("malformed"));
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(addr).await;
    let r = c.tool(1, "mailslurper_list", serde_json::json!({})).await;
    assert!(r.0 && r.1["error"].as_str().unwrap().starts_with("malformed"));
    // An oversized one is a budget error, never a panic.
    let huge = responder(Box::new(|_| Some((200, "x".repeat(5000))))).await;
    let bounds = mailpit_mcp::Bounds {
        max_body: 4096,
        ..mailpit_mcp::Bounds::default()
    };
    let mut c = Mcp::start_backend::<mailslurper_mcp::Mailslurper>(huge, bounds, 1 << 20);
    c.init().await;
    let (e, v) = c.tool(1, "mailslurper_list", serde_json::json!({})).await;
    assert!(
        e && v["error"].as_str().unwrap().starts_with("budget_exhausted"),
        "{v}"
    );
}

// ---- the wire corpus: real upstream responses, decoded in CI (0129 §5) -----
//
// `capture_wire_corpus` is the only writer. It runs the pinned executable
// (`MAIL_COMPARE_<TARGET>`), sends synthetic mail shaped like the eval
// fixture's (a lookalike recipient pair and a multipart message with
// quoted-printable and base64 parts: no agent body, 0075), and records each
// response body the adapter's decoders and the oracle's normalizer read.
// It never overwrites: a moved pin is recaptured by removing the directory.

const CORPUS_FILES: [&str; 6] = [
    "list.json",
    "read.json",
    "read-multipart.json",
    "search-filtered.json",
    "search-empty.json",
    "read-not-found.json",
];

/// The lock row an agent-suite target's bytes come from. The agent target
/// `mailslurper` names the service API the adapter drives; its executable is
/// accepted 0130's `mailslurper-source-build` row, never the official
/// `mailslurper` row, which does not start. Every other target is refused by
/// name, so a new `TargetKind` is a compile error here.
fn lock_row(t: TargetKind) -> TargetKind {
    match t {
        TargetKind::Mailpit => TargetKind::Mailpit,
        TargetKind::Mailhog => TargetKind::Mailhog,
        TargetKind::Mailslurper => TargetKind::MailslurperSourceBuild,
        TargetKind::Swarmail
        | TargetKind::MailslurperSourceBuild
        | TargetKind::Smtp4dev
        | TargetKind::Inbucket
        | TargetKind::Greenmail
        | TargetKind::AlkemioMailslurper => panic!("{t} is not an agent-suite wire-corpus target"),
    }
}

/// The digest a corpus index binds for a pin's launch config: the text the
/// launcher renders, before its slots are filled (`None` for a pin with no
/// config file).
fn config_template_sha256(pin: &protocol::Pin) -> Option<String> {
    pin.config_file
        .as_ref()
        .map(|c| targets::sha256_hex(c.lines.join("\n").as_bytes()))
}

fn corpus_dir(t: TargetKind) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs/evals/wire-corpus")
        .join(t.name())
}

/// One message over plain SMTP, as the fixture's dialogue sends it (EHLO,
/// AUTH PLAIN when `auth`, MAIL, RCPT, DATA).
async fn send_raw(smtp: std::net::SocketAddr, auth: bool, rcpt: &str, raw: &str) {
    use base64::Engine;
    use tokio::io::AsyncWriteExt;
    let (r, mut w) = tokio::net::TcpStream::connect(smtp)
        .await
        .unwrap()
        .into_split();
    let mut r = tokio::io::BufReader::new(r);
    assert_eq!(smtp_code(&mut r).await, 220);
    let mut steps = vec!["EHLO identity.invalid\r\n".to_owned()];
    if auth {
        let token = base64::engine::general_purpose::STANDARD.encode("\0corpus\0pw");
        steps.push(format!("AUTH PLAIN {token}\r\n"));
    }
    steps.extend([
        "MAIL FROM:<sender@corpus.invalid>\r\n".to_owned(),
        format!("RCPT TO:<{rcpt}>\r\n"),
        "DATA\r\n".to_owned(),
    ]);
    for step in steps {
        w.write_all(step.as_bytes()).await.unwrap();
        let code = smtp_code(&mut r).await;
        assert!((200..400).contains(&code), "{step:?} answered {code}");
    }
    w.write_all(format!("{raw}\r\n.\r\n").as_bytes())
        .await
        .unwrap();
    assert_eq!(smtp_code(&mut r).await, 250);
    w.write_all(b"QUIT\r\n").await.unwrap();
}

/// The corpus mail: `(recipient, subject, raw message)`.
fn corpus_mail() -> Vec<(&'static str, &'static str, String)> {
    let plain = |to: &str, subject: &str, body: &str| {
        format!(
            "From: sender@corpus.invalid\r\nTo: {to}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}"
        )
    };
    let multipart = "From: sender@corpus.invalid\r\nTo: ann@corpus.invalid\r\nSubject: corpus-multipart\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"b1\"\r\n\r\n--b1\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\ncode =3D 123456 caf=C3=A9\r\n--b1\r\nContent-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nPHA+Y29kZSAxMjM0NTY8L3A+\r\n--b1--";
    vec![
        (
            "ann@corpus.invalid",
            "corpus-ann",
            plain("ann@corpus.invalid", "corpus-ann", "code 111111"),
        ),
        (
            "joann@corpus.invalid",
            "corpus-joann",
            plain("joann@corpus.invalid", "corpus-joann", "code 222222"),
        ),
        (
            "ann@corpus.invalid",
            "corpus-multipart",
            multipart.to_owned(),
        ),
    ]
}

#[tokio::test]
#[ignore = "needs the pinned executables: MAIL_COMPARE_MAILPIT and MAIL_COMPARE_MAILHOG"]
async fn capture_wire_corpus() {
    for t in [TargetKind::Mailpit, TargetKind::Mailhog] {
        let dir = corpus_dir(t);
        assert!(
            !dir.exists(),
            "{dir:?} exists: remove it to recapture, a corpus is never overwritten"
        );
        let root = tempfile::tempdir().unwrap();
        let mut owned = launch_pinned(t, &root.path().join("s")).await;
        for (rcpt, _, raw) in corpus_mail() {
            send_raw(owned.smtp, true, rcpt, &raw).await;
        }
        let (list, listing, one, many, none, read): (String, &str, String, String, String, &str) =
            match t {
                TargetKind::Mailpit => (
                    "/api/v1/messages?start=0&limit=50".into(),
                    "messages",
                    "/api/v1/search?query=to%3Aann%40corpus%2Einvalid&start=0&limit=50".into(),
                    String::new(),
                    "/api/v1/search?query=to%3Anobody%40corpus%2Einvalid&start=0&limit=50".into(),
                    "/api/v1/message/",
                ),
                _ => (
                    "/api/v2/messages?start=0&limit=50".into(),
                    "items",
                    "/api/v2/search?kind=to&query=ann%40corpus%2Einvalid&start=0&limit=50".into(),
                    String::new(),
                    "/api/v2/search?kind=to&query=nobody%40corpus%2Einvalid&start=0&limit=50"
                        .into(),
                    "/api/v1/messages/",
                ),
            };
        let _ = many;
        // Accept is not presumed to be storage: wait for all three.
        let until = Instant::now() + Duration::from_secs(10);
        let listed = loop {
            let (s, body) = owned.http.get(&list).await.unwrap();
            let v = json(&body);
            if s == 200 && v[listing].as_array().is_some_and(|a| a.len() == 3) {
                break (body, v);
            }
            assert!(Instant::now() < until, "{t} never listed all three");
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        let id_of = |subject: &str| -> String {
            listed.1[listing]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m.to_string().contains(subject))
                .and_then(|m| m["ID"].as_str())
                .unwrap_or_else(|| panic!("{t}: no id for {subject}"))
                .to_owned()
        };
        let mut out: Vec<(&str, String, u16, Vec<u8>)> =
            vec![("list.json", list.clone(), 200, listed.0.clone())];
        for (name, path) in [
            ("read.json", format!("{read}{}", id_of("corpus-ann"))),
            (
                "read-multipart.json",
                format!("{read}{}", id_of("corpus-multipart")),
            ),
            ("search-filtered.json", one),
            ("search-empty.json", none),
            ("read-not-found.json", format!("{read}not-an-id")),
        ] {
            let (s, body) = owned.http.get(&path).await.unwrap();
            out.push((name, path, s, body));
        }
        assert!(owned.shutdown().await.proved());
        std::fs::create_dir_all(&dir).unwrap();
        let lock = SourceLock::parse(SOURCES_V4).unwrap();
        let mut files = Vec::new();
        for (name, request, status, body) in out {
            assert_eq!(name, CORPUS_FILES[files.len()]);
            std::fs::write(dir.join(name), &body).unwrap();
            files.push(
                serde_json::json!({"name": name, "request": request, "status": status,
                "size": body.len(), "sha256": targets::sha256_hex(&body)}),
            );
        }
        let index = serde_json::json!({
            "target": t.name(),
            "executable_sha256": lock.pin(t).unwrap().artifact.executable_sha256,
            "mail": corpus_mail().iter().map(|(r, s, _)| serde_json::json!({"to": r, "subject": s})).collect::<Vec<_>>(),
            "files": files,
        });
        std::fs::write(
            dir.join("index.json"),
            serde_json::to_string_pretty(&index).unwrap() + "\n",
        )
        .unwrap();
    }
}

const MAILSLURPER_CORPUS_FILES: [&str; 8] = [
    "list.json",
    "read.json",
    "read-multipart.json",
    "read-base64.json",
    "read-lines.json",
    "search-filtered.json",
    "search-empty.json",
    "read-not-found.json",
];

/// A multi-line plain body with a URL and a blank line: what MailSlurper does
/// to line breaks decides how the oracle reads the fixture's text mails.
fn slurper_lines_mail() -> (&'static str, &'static str, String) {
    (
        "ann@corpus.invalid",
        "corpus-lines",
        "From: sender@corpus.invalid\r\nTo: ann@corpus.invalid\r\nSubject: corpus-lines\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 7bit\r\n\r\nalpha\r\nConfirm your account: http://x.invalid/a?b=1&c=2\r\n\r\ngamma\r\n".to_owned(),
    )
}

/// The mail MailSlurper's corpus adds to `corpus_mail()`: a plain body under a
/// base64 transfer encoding, which MailSlurper decodes at ingest.
fn slurper_base64_mail() -> (&'static str, &'static str, String) {
    (
        "ann@corpus.invalid",
        "corpus-base64",
        "From: sender@corpus.invalid\r\nTo: ann@corpus.invalid\r\nSubject: corpus-base64\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nY29kZSA0NDQ0NDQNCnNlY29uZCBsaW5lIGh0dHBzOi8veC5pbnZhbGlkL2E/Yj0xJmM9Mg0K".to_owned(),
    )
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD (the pinned executable built from source)"]
async fn capture_wire_corpus_mailslurper() {
    let t = TargetKind::Mailslurper;
    let dir = corpus_dir(t);
    assert!(
        !dir.exists(),
        "{dir:?} exists: remove it to recapture, a corpus is never overwritten"
    );
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_pinned(lock_row(t), &root.path().join("s")).await;
    let mut mail = corpus_mail();
    mail.push(slurper_base64_mail());
    mail.push(slurper_lines_mail());
    // MailSlurper has no AUTH: the fixture's dialogue leaves the step out.
    for (rcpt, _, raw) in &mail {
        send_raw(owned.smtp, false, rcpt, raw).await;
    }
    let list = "/mail?pageNumber=1".to_owned();
    // Accept is not storage for MailSlurper: wait for all of them.
    let until = Instant::now() + Duration::from_secs(10);
    let listed = loop {
        let (s, body) = owned.http.get(&list).await.unwrap();
        let v = json(&body);
        if s == 200 && v["totalRecords"] == mail.len() {
            break (body, v);
        }
        assert!(Instant::now() < until, "{t} never listed all of them");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let id_of = |subject: &str| -> String {
        listed.1["mailItems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["subject"] == subject)
            .and_then(|m| m["id"].as_str())
            .unwrap_or_else(|| panic!("{t}: no id for {subject}"))
            .to_owned()
    };
    let to = |addr: &str| format!("/mail?pageNumber=1&to={}", wire::query_value(addr));
    let mut out: Vec<(&str, String, u16, Vec<u8>)> =
        vec![("list.json", list.clone(), 200, listed.0.clone())];
    for (name, path) in [
        ("read.json", format!("/mail/{}", id_of("corpus-ann"))),
        (
            "read-multipart.json",
            format!("/mail/{}", id_of("corpus-multipart")),
        ),
        (
            "read-base64.json",
            format!("/mail/{}", id_of("corpus-base64")),
        ),
        (
            "read-lines.json",
            format!("/mail/{}", id_of("corpus-lines")),
        ),
        ("search-filtered.json", to("ann@corpus.invalid")),
        ("search-empty.json", to("nobody@corpus.invalid")),
        ("read-not-found.json", "/mail/not-an-id".to_owned()),
    ] {
        let (s, body) = owned.http.get(&path).await.unwrap();
        out.push((name, path, s, body));
    }
    assert!(owned.shutdown().await.proved());
    std::fs::create_dir_all(&dir).unwrap();
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    let pin = lock.pin(lock_row(t)).unwrap();
    let mut files = Vec::new();
    for (name, request, status, body) in out {
        assert_eq!(name, MAILSLURPER_CORPUS_FILES[files.len()]);
        std::fs::write(dir.join(name), &body).unwrap();
        files.push(
            serde_json::json!({"name": name, "request": request, "status": status,
            "size": body.len(), "sha256": targets::sha256_hex(&body)}),
        );
    }
    let index = serde_json::json!({
        "target": t.name(),
        "executable_sha256": pin.artifact.executable_sha256,
        "config_template_sha256": config_template_sha256(pin),
        "mail": mail.iter().map(|(r, s, _)| serde_json::json!({"to": r, "subject": s})).collect::<Vec<_>>(),
        "files": files,
    });
    std::fs::write(
        dir.join("index.json"),
        serde_json::to_string_pretty(&index).unwrap() + "\n",
    )
    .unwrap();
}

/// Check one target's corpus against its index and the lock, and decode every
/// file through the adapter's own decoders. The first failure names itself.
fn check_corpus(t: TargetKind, dir: &Path, lock: &SourceLock) -> Result<(), String> {
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("index.json")).map_err(|e| e.to_string())?)
            .map_err(|e| format!("index: {e}"))?;
    let want = &lock
        .pin(lock_row(t))
        .ok_or("no pin")?
        .artifact
        .executable_sha256;
    if index["target"] != t.name() || index["executable_sha256"] != *want {
        return Err(format!(
            "{t}: the index's executable digest is not the lock's, recapture the corpus"
        ));
    }
    // Every target's index binds its launch config template: MailSlurper's is
    // the lock's `mailslurper-source-build` config file (accepted 0130), the
    // other two have none and say `null`.
    let template = lock.pin(lock_row(t)).and_then(config_template_sha256);
    if index.get("config_template_sha256") != Some(&serde_json::json!(template)) {
        return Err(format!(
            "{t}: the index's config template digest is not the lock's, recapture the corpus"
        ));
    }
    let wanted: &[&str] = if t == TargetKind::Mailslurper {
        &MAILSLURPER_CORPUS_FILES
    } else {
        &CORPUS_FILES
    };
    let files = index["files"].as_array().ok_or("files")?;
    let names: Vec<&str> = files.iter().filter_map(|f| f["name"].as_str()).collect();
    if names != wanted {
        return Err(format!("{t}: the index lists {names:?}"));
    }
    let mut body = std::collections::HashMap::new();
    for f in files {
        let name = f["name"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(name)).map_err(|e| format!("{name}: {e}"))?;
        if bytes.len() as u64 != f["size"] || targets::sha256_hex(&bytes) != f["sha256"] {
            return Err(format!("{t}: {name} is not the file the index binds"));
        }
        body.insert(name, (f["status"].as_u64().unwrap_or(0), bytes));
    }
    let v = |n: &str| -> Result<serde_json::Value, String> {
        serde_json::from_slice(&body[n].1).map_err(|e| format!("{n}: {e}"))
    };
    if t == TargetKind::Mailslurper {
        return check_slurper_corpus(&body, &v);
    }
    let decode = |n: &str| -> Result<(u64, Vec<String>), String> {
        let (total, ids): (u64, Vec<String>) = match t {
            TargetKind::Mailpit => {
                let p = mailpit_mcp::summary_page(v(n)?, 0).map_err(|e| format!("{n}: {e:?}"))?;
                (
                    p["messages_count"].as_u64().unwrap(),
                    p["messages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|m| m["ID"].as_str().unwrap().to_owned())
                        .collect(),
                )
            }
            TargetKind::Mailhog => {
                let (total, ids, _) =
                    mailhog_mcp::page(v(n)?, 0).map_err(|e| format!("{n}: {e:?}"))?;
                (total, ids.into_iter().map(|(i, _)| i).collect())
            }
            // MailSlurper's corpus returned above; no other target has one.
            TargetKind::Mailslurper
            | TargetKind::Swarmail
            | TargetKind::MailslurperSourceBuild
            | TargetKind::Smtp4dev
            | TargetKind::Inbucket
            | TargetKind::Greenmail
            | TargetKind::AlkemioMailslurper => {
                return Err(format!("{t} has no decoder for this corpus"));
            }
        };
        Ok((total, ids))
    };
    let extra: &[u8] = if t == TargetKind::Mailhog {
        MAILHOG_CORPUS_ID_EXTRA
    } else {
        mailpit_mcp::MAILPIT_ID_EXTRA
    };
    let (total, ids) = decode("list.json")?;
    if (total, ids.len()) != (3, 3) || !ids.iter().all(|i| mcp::valid_id(i, extra)) {
        return Err(format!("{t}: list.json is not three ids in the alphabet"));
    }
    let (found, hits) = decode("search-filtered.json")?;
    // `ann@corpus.invalid` is inside `joann@corpus.invalid`: upstream's own
    // recipient filter returns the lookalike too (all three mails), which is
    // why the oracle matches the recipient exactly after any pre-filter.
    let raw = String::from_utf8_lossy(&body["search-filtered.json"].1).into_owned();
    if found != 3 || hits.len() != 3 || !raw.contains("joann@corpus.invalid") {
        return Err(format!(
            "{t}: the filtered search is not the substring match the corpus records"
        ));
    }
    let (found, hits) = decode("search-empty.json")?;
    if found != 0 || !hits.is_empty() {
        return Err(format!("{t}: the empty search is not empty"));
    }
    for n in ["read.json", "read-multipart.json"] {
        if body[n].0 != 200 || !mcp::valid_id(v(n)?["ID"].as_str().unwrap_or(""), extra) {
            return Err(format!("{t}: {n} is not a read of one stored message"));
        }
    }
    // What an unknown id looks like is what the adapters map to not_found:
    // Mailpit answers 404, MailHog `200 null`.
    let unknown = &body["read-not-found.json"];
    let expected = if t == TargetKind::Mailpit { 404 } else { 200 };
    if unknown.0 != expected
        || (t == TargetKind::Mailhog && v("read-not-found.json")? != serde_json::Value::Null)
    {
        return Err(format!(
            "{t}: read-not-found.json is not the upstream's not-found shape"
        ));
    }
    Ok(())
}

/// MailSlurper's corpus through the adapter's own decoder: every page and
/// read in the one id alphabet; the two body branches `GetMail` has (a body it
/// base64-decodes and one it leaves alone); the filter's lookalike; the empty
/// search; and the unknown id's shape, an empty item at HTTP 200.
fn check_slurper_corpus(
    body: &std::collections::HashMap<&str, (u64, Vec<u8>)>,
    v: &dyn Fn(&str) -> Result<serde_json::Value, String>,
) -> Result<(), String> {
    let t = TargetKind::Mailslurper;
    let page = |n: &str| -> Result<(u64, Vec<String>), String> {
        let (total, ids, _) = mailslurper_mcp::page(v(n)?).map_err(|e| format!("{n}: {e:?}"))?;
        Ok((total, ids.into_iter().map(|(i, _)| i).collect()))
    };
    let (total, ids) = page("list.json")?;
    if (total, ids.len()) != (5, 5)
        || !ids
            .iter()
            .all(|i| mcp::valid_id(i, mailslurper_mcp::MAILSLURPER_ID_EXTRA))
    {
        return Err(format!("{t}: list.json is not five ids in the alphabet"));
    }
    let (found, hits) = page("search-filtered.json")?;
    let raw = String::from_utf8_lossy(&body["search-filtered.json"].1).into_owned();
    if found != 5 || hits.len() != 5 || !raw.contains("joann@corpus.invalid") {
        return Err(format!(
            "{t}: the filtered search is not the substring match the corpus records"
        ));
    }
    let (found, hits) = page("search-empty.json")?;
    if found != 0 || !hits.is_empty() {
        return Err(format!("{t}: the empty search is not empty"));
    }
    let listed = v("list.json")?;
    let listed_body = |subject: &str| -> String {
        listed["mailItems"]
            .as_array()
            .and_then(|a| a.iter().find(|m| m["subject"] == subject))
            .and_then(|m| m["body"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    for n in [
        "read.json",
        "read-multipart.json",
        "read-base64.json",
        "read-lines.json",
    ] {
        let id = v(n)?["id"].as_str().unwrap_or("").to_owned();
        if body[n].0 != 200 || !mcp::valid_id(&id, mailslurper_mcp::MAILSLURPER_ID_EXTRA) {
            return Err(format!("{t}: {n} is not a read of one stored message"));
        }
        if !ids.contains(&id) {
            return Err(format!("{t}: {n} reads an id the listing does not carry"));
        }
    }
    // The branch GetMail leaves alone: a plain body is the listing's body.
    if v("read.json")?["body"].as_str() != Some(listed_body("corpus-ann").as_str()) {
        return Err(format!("{t}: a plain body changed between list and read"));
    }
    // The branch GetMail decodes: the multipart's HTML part is base64 in the
    // listing and markup on read.
    let encoded = listed_body("corpus-multipart");
    let decoded = v("read-multipart.json")?["body"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    use base64::Engine;
    if base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .ok()
        != Some(decoded.clone().into_bytes())
        || !decoded.starts_with('<')
    {
        return Err(format!(
            "{t}: GetMail's base64 branch is not what the corpus records"
        ));
    }
    // An unknown id is an empty item at 200, which the adapter reports as not
    // found (a real 404 never comes).
    let unknown = v("read-not-found.json")?;
    if body["read-not-found.json"].0 != 200 || unknown["id"] != "" || unknown["subject"] != "" {
        return Err(format!(
            "{t}: read-not-found.json is not the upstream's not-found shape"
        ));
    }
    Ok(())
}

#[test]
fn the_wire_corpus_decodes_through_the_adapter_decoders_and_binds_its_executable() {
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    for t in [
        TargetKind::Mailpit,
        TargetKind::Mailhog,
        TargetKind::Mailslurper,
    ] {
        check_corpus(t, &corpus_dir(t), &lock).unwrap_or_else(|e| panic!("{e}"));
    }
    // MailHog's real ids are why its alphabet is wider than Mailpit's.
    let list = json(&std::fs::read(corpus_dir(TargetKind::Mailhog).join("list.json")).unwrap());
    for m in list["items"].as_array().unwrap() {
        let id = m["ID"].as_str().unwrap();
        assert!(mcp::valid_id(id, MAILHOG_CORPUS_ID_EXTRA), "{id}");
        assert!(!mcp::valid_id(id, b""), "{id} needs the widened alphabet");
        assert!(id.ends_with("@mailhog.example"), "{id}");
    }
    let list = json(&std::fs::read(corpus_dir(TargetKind::Mailpit).join("list.json")).unwrap());
    for m in list["messages"].as_array().unwrap() {
        let id = m["ID"].as_str().unwrap();
        assert!(id.bytes().all(|b| b.is_ascii_alphanumeric()), "{id}");
    }
}

#[tokio::test]
async fn the_mailslurper_adapter_serves_the_corpus_and_maps_the_empty_item_to_not_found() {
    let dir = corpus_dir(TargetKind::Mailslurper);
    let file = |n: &str| String::from_utf8(std::fs::read(dir.join(n)).unwrap()).unwrap();
    let index = json(&std::fs::read(dir.join("index.json")).unwrap());
    let request = |name: &str| -> String {
        index["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == name)
            .unwrap()["request"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let routes: Vec<(String, String)> = [
        "list.json",
        "read.json",
        "read-multipart.json",
        "read-not-found.json",
        "search-filtered.json",
        "search-empty.json",
    ]
    .iter()
    .map(|n| (request(n), file(n)))
    .collect();
    let addr = responder(Box::new(move |t: &str| {
        routes
            .iter()
            .find(|(path, _)| path == t)
            .map(|(_, body)| (200, body.clone()))
    }))
    .await;
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(addr).await;
    let (e, v) = c.tool(1, "mailslurper_list", serde_json::json!({})).await;
    assert!(!e && v["totalRecords"] == 5, "{v}");
    let (e, v) = c
        .tool(
            2,
            "mailslurper_search",
            serde_json::json!({"to": "ann@corpus.invalid"}),
        )
        .await;
    assert!(!e && v["totalRecords"] == 5, "{v}");
    let (e, v) = c
        .tool(
            3,
            "mailslurper_search",
            serde_json::json!({"to": "nobody@corpus.invalid"}),
        )
        .await;
    assert!(!e && v["totalRecords"] == 0, "{v}");
    let id = |n: &str| json(file(n).as_bytes())["id"].as_str().unwrap().to_owned();
    for (n, name) in [(4, "read.json"), (5, "read-multipart.json")] {
        let (e, v) = c
            .tool(n, "mailslurper_read", serde_json::json!({"id": id(name)}))
            .await;
        assert!(!e && v["id"] == id(name).as_str(), "{name}: {v}");
    }
    // The unknown id's request is `/mail/not-an-id`: an empty item at 200.
    let (e, v) = c
        .tool(
            6,
            "mailslurper_read",
            serde_json::json!({"id": "not-an-id"}),
        )
        .await;
    assert!(
        e && v["error"].as_str().unwrap().starts_with("not_found"),
        "{v}"
    );
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
}

#[test]
fn a_mailslurper_corpus_that_drifts_from_its_pin_or_its_shapes_refuses() {
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    let t = TargetKind::Mailslurper;
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("c");
    copy_dir(&corpus_dir(t), &dir);
    check_corpus(t, &dir, &lock).expect("the copy checks");
    // A moved config template: the index's digest is not the lock's.
    let mut other = lock.clone();
    other.entries.iter_mut().for_each(|e| {
        if let Some(c) = e
            .pin
            .as_mut()
            .filter(|_| e.target == lock_row(t))
            .and_then(|p| p.config_file.as_mut())
        {
            c.lines.push(" ".into());
        }
    });
    let why = check_corpus(t, &dir, &other).expect_err("must refuse");
    assert!(
        why.contains("config template digest is not the lock's"),
        "{why}"
    );
    // A moved executable.
    let mut other = lock.clone();
    other.entries.iter_mut().for_each(|e| {
        if let Some(p) = e.pin.as_mut().filter(|_| e.target == lock_row(t)) {
            p.artifact.executable_sha256 = "0".repeat(64);
        }
    });
    let why = check_corpus(t, &dir, &other).expect_err("must refuse");
    assert!(why.contains("executable digest is not the lock's"), "{why}");
    // A changed file, and a corpus that no longer records GetMail's branches.
    let rebind = |name: &str, edit: &dyn Fn(&mut serde_json::Value)| {
        let path = dir.join(name);
        let mut v = json(&std::fs::read(&path).unwrap());
        edit(&mut v);
        let bytes = serde_json::to_vec(&v).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let mut index = json(&std::fs::read(dir.join("index.json")).unwrap());
        for f in index["files"].as_array_mut().unwrap() {
            if f["name"] == name {
                f["size"] = bytes.len().into();
                f["sha256"] = targets::sha256_hex(&bytes).into();
            }
        }
        std::fs::write(dir.join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();
    };
    rebind("read-multipart.json", &|v| {
        v["body"] = "<p>other</p>".into()
    });
    let why = check_corpus(t, &dir, &lock).expect_err("must refuse");
    assert!(why.contains("GetMail's base64 branch"), "{why}");
    let file = dir.join("read.json");
    let mut bytes = std::fs::read(&file).unwrap();
    bytes[3] ^= 1;
    std::fs::write(&file, bytes).unwrap();
    let why = check_corpus(t, &dir, &lock).expect_err("must refuse");
    assert!(
        why.contains("read.json is not the file the index binds"),
        "{why}"
    );
}

#[test]
fn a_corpus_whose_index_disagrees_with_the_lock_or_its_files_refuses() {
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    let t = TargetKind::Mailhog;
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("c");
    copy_dir(&corpus_dir(t), &dir);
    check_corpus(t, &dir, &lock).expect("the copy checks");
    // A moved pin: the index's executable digest is not the lock's.
    let mut other = lock.clone();
    other.entries.iter_mut().for_each(|e| {
        if let Some(p) = e.pin.as_mut().filter(|_| e.target == lock_row(t)) {
            p.artifact.executable_sha256 = "0".repeat(64);
        }
    });
    let why = check_corpus(t, &dir, &other).expect_err("must refuse");
    assert!(why.contains("executable digest is not the lock's"), "{why}");
    // A changed file is not the file the index binds.
    let file = dir.join("list.json");
    let mut bytes = std::fs::read(&file).unwrap();
    bytes[3] ^= 1;
    std::fs::write(&file, bytes).unwrap();
    let why = check_corpus(t, &dir, &lock).expect_err("must refuse");
    assert!(
        why.contains("list.json is not the file the index binds"),
        "{why}"
    );
}

// ---- the AUTH PLAIN probe (decision 0129 §2, spec R8) ----------------------

/// The unscored setup probe: does the target take the fixture's AUTH PLAIN?
/// MailSlurper's reviewed source says it has no AUTH: EHLO advertises no
/// extension and an `AUTH PLAIN …` line draws `221 Bye` and a close. The probe
/// reads that from the wire. `Ok` is the transcript of exactly that outcome;
/// any other answer (an advertised AUTH, a `334`, a `235`, a reply that is not
/// `221`, a connection that stays open) is an error naming it, so the product
/// record cannot freeze on a guess.
async fn auth_plain_probe(smtp: std::net::SocketAddr) -> Result<String, String> {
    use base64::Engine;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let step = Duration::from_secs(10);
    let (r, mut w) = tokio::time::timeout(step, tokio::net::TcpStream::connect(smtp))
        .await
        .map_err(|_| "connect timed out")?
        .map_err(|e| format!("connect: {e}"))?
        .into_split();
    let mut r = tokio::io::BufReader::new(r);
    let mut transcript = String::new();
    let line = async |r: &mut tokio::io::BufReader<_>| -> Result<Option<String>, String> {
        let mut s = String::new();
        let n = tokio::time::timeout(step, r.read_line(&mut s))
            .await
            .map_err(|_| "a reply timed out".to_owned())?
            .map_err(|e| format!("read: {e}"))?;
        Ok((n > 0).then(|| s.trim_end().to_owned()))
    };
    let banner = line(&mut r).await?.ok_or("closed before the banner")?;
    transcript.push_str(&format!("S: {banner}\n"));
    w.write_all(b"EHLO identity.invalid\r\n")
        .await
        .map_err(|e| e.to_string())?;
    transcript.push_str("C: EHLO identity.invalid\n");
    loop {
        let l = line(&mut r).await?.ok_or("closed during EHLO")?;
        transcript.push_str(&format!("S: {l}\n"));
        if l.to_ascii_uppercase().contains("AUTH") {
            return Err(format!("EHLO advertises AUTH: {l}"));
        }
        if !l.starts_with("250") {
            return Err(format!("EHLO was not answered 250: {l}"));
        }
        if l.as_bytes().get(3) != Some(&b'-') {
            break;
        }
    }
    let token = base64::engine::general_purpose::STANDARD.encode("\0probe\0pw");
    w.write_all(format!("AUTH PLAIN {token}\r\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    transcript.push_str("C: AUTH PLAIN <base64 of \\0probe\\0pw>\n");
    let mut replies = Vec::new();
    while let Some(l) = line(&mut r).await? {
        transcript.push_str(&format!("S: {l}\n"));
        replies.push(l);
        if replies.len() > 8 {
            return Err("more replies than a closing line".into());
        }
    }
    transcript.push_str("(connection closed by the server)\n");
    if replies
        .iter()
        .any(|l| l.starts_with("235") || l.starts_with("334"))
    {
        return Err(format!("AUTH PLAIN was taken: {replies:?}"));
    }
    // The reviewed source says one goodbye per worker error: the unknown
    // command's, and (the command goroutine's `break` only leaves its
    // `select`, so it reads again) the read error on the connection the
    // first goodbye closed. Either is `221 Bye`, and nothing else is.
    if !(1..=2).contains(&replies.len()) || replies.iter().any(|l| l != "221 Bye") {
        return Err(format!("AUTH PLAIN did not draw only 221 Bye: {replies:?}"));
    }
    Ok(transcript)
}

/// A scripted SMTP server: each connection gets `banner`, then one reply
/// block per client line from `replies`, closing after the last (or at once
/// when `close` is set after the last reply).
async fn scripted_smtp(
    banner: &'static str,
    replies: Vec<&'static str>,
    close: bool,
) -> std::net::SocketAddr {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((s, _)) = l.accept().await {
            let replies = replies.clone();
            tokio::spawn(async move {
                let (r, mut w) = s.into_split();
                let mut r = tokio::io::BufReader::new(r);
                let _ = w.write_all(format!("{banner}\r\n").as_bytes()).await;
                for reply in replies {
                    let mut l = String::new();
                    if r.read_line(&mut l).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let _ = w.write_all(format!("{reply}\r\n").as_bytes()).await;
                }
                if close {
                    return;
                }
                std::future::pending::<()>().await;
            });
        }
    });
    addr
}

#[tokio::test]
async fn the_auth_probe_accepts_only_a_no_auth_221_close_and_names_every_other_answer() {
    let ehlo = "250 Hello. How very nice to meet you!";
    let no_auth = scripted_smtp("220 Welcome", vec![ehlo, "221 Bye"], true).await;
    let t = auth_plain_probe(no_auth)
        .await
        .expect("the expected outcome");
    assert!(
        t.contains("S: 221 Bye") && t.contains("connection closed"),
        "{t}"
    );
    assert!(!t.contains("S: 235") && !t.contains("S: 334"), "{t}");
    // The source's race: a second goodbye for the read error on the closed
    // connection. Two lines of `221 Bye` are the same answer.
    let two = scripted_smtp("220 Welcome", vec![ehlo, "221 Bye\r\n221 Bye"], true).await;
    let t = auth_plain_probe(two).await.expect("two goodbyes");
    assert_eq!(t.matches("S: 221 Bye").count(), 2, "{t}");
    for (replies, want) in [
        (
            vec![ehlo, "221 Bye\r\n221 Bye\r\n221 Bye"],
            "did not draw only 221 Bye",
        ),
        (
            vec![ehlo, "221 Bye\r\n221 later"],
            "did not draw only 221 Bye",
        ),
        (
            vec!["250-hi\r\n250 AUTH PLAIN", "235 ok"],
            "EHLO advertises AUTH",
        ),
        (
            vec![ehlo, "235 Authentication successful"],
            "AUTH PLAIN was taken",
        ),
        (vec![ehlo, "334 "], "AUTH PLAIN was taken"),
        (vec![ehlo, "500 unknown"], "did not draw only 221 Bye"),
        (vec!["421 down"], "EHLO was not answered 250"),
    ] {
        let addr = scripted_smtp("220 Welcome", replies, true).await;
        let why = auth_plain_probe(addr).await.expect_err("must refuse");
        assert!(why.contains(want), "{want}: {why}");
    }
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD (the pinned executable built from source)"]
async fn mailslurper_auth_plain_probe() {
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_pinned(TargetKind::MailslurperSourceBuild, &root.path().join("s")).await;
    let outcome = auth_plain_probe(owned.smtp).await;
    assert!(owned.shutdown().await.proved());
    let transcript = outcome.unwrap_or_else(|e| panic!("the probe disagrees with the source: {e}"));
    // The subject is the digest the launch just checked: the source build,
    // never the published executable, which does not start.
    let subject = SourceLock::parse(SOURCES_V4)
        .unwrap()
        .pin(TargetKind::MailslurperSourceBuild)
        .unwrap()
        .artifact
        .executable_sha256
        .clone();
    assert_eq!(
        subject,
        "eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff"
    );
    let transcript = format!(
        "# subject: the executable built from source, sha256 {subject} (checked by the launcher)\n\
         # the published linux executable (zip 48f2f9ba…55f6, member 0cd9416b…2627) does not start and was not probed\n\
         {transcript}"
    );
    if let Some(dir) = std::env::var_os("MAIL_COMPARE_TRANSCRIPTS") {
        std::fs::write(
            Path::new(&dir).join("mailslurper-auth-probe.txt"),
            transcript,
        )
        .unwrap();
    }
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD (the pinned executable built from source)"]
async fn real_mailslurper_through_the_adapter_lists_searches_reads_and_waits() {
    let root = tempfile::tempdir().unwrap();
    let mut owned = launch_pinned(TargetKind::MailslurperSourceBuild, &root.path().join("s")).await;
    // MailSlurper has no AUTH: the fixture leaves the step out.
    for (i, subject) in ["slurptoken-one", "slurptoken-two", "other-three"]
        .iter()
        .enumerate()
    {
        send_as(owned.smtp, None, subject, &format!("body {i}")).await;
    }
    let mut c = initialized_for::<mailslurper_mcp::Mailslurper>(owned.http_addr).await;
    let mut id = 100;
    let mut call = async |c: &mut Mcp, name: &str, args: serde_json::Value| {
        id += 1;
        let (is_error, v) = c.tool(id, name, args.clone()).await;
        assert!(!is_error, "{name} {args}: {v}");
        v
    };
    // MailSlurper sends its 250 before it stores the mail: wait for all three.
    let until = Instant::now() + Duration::from_secs(10);
    while call(&mut c, "mailslurper_list", serde_json::json!({})).await["totalRecords"] != 3 {
        assert!(Instant::now() < until, "MailSlurper never listed all three");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let listed = call(&mut c, "mailslurper_list", serde_json::json!({})).await;
    let first = listed["mailItems"][0]["id"].as_str().unwrap().to_owned();
    let hits = call(
        &mut c,
        "mailslurper_search",
        serde_json::json!({"message": "slurptoken"}),
    )
    .await;
    assert_eq!(hits["totalRecords"], 2, "{hits}");
    let read = call(&mut c, "mailslurper_read", serde_json::json!({"id": first})).await;
    assert_eq!(read["id"], serde_json::json!(first));
    let found = call(
        &mut c,
        "mailslurper_wait",
        serde_json::json!({"message": "slurptoken-one", "timeout_ms": 5000}),
    )
    .await;
    assert_eq!(found["status"], "found", "{found}");
    let none = call(
        &mut c,
        "mailslurper_wait",
        serde_json::json!({"message": "never-sent", "timeout_ms": 300}),
    )
    .await;
    assert_eq!(none["status"], "timeout", "{none}");
    drop(c.w.take());
    assert_eq!(c.served.await.unwrap(), mailpit_mcp::End::Eof);
    assert!(owned.shutdown().await.proved());
}

/// The frozen product record's `tools_sha256` for each adapter is the digest
/// of the tool list the adapter serves, so a tool added or removed (or a
/// description changed) cannot leave the record naming other tools.
#[test]
fn the_product_record_pins_the_tool_lists_the_adapters_serve() {
    let product = json(include_bytes!("../docs/evals/agent-comparison.json"));
    let b = mailpit_mcp::Bounds::default();
    let built = [
        ("mailpit", mcp::tools_sha256::<mailpit_mcp::Mailpit>(&b)),
        ("mailhog", mcp::tools_sha256::<mailhog_mcp::Mailhog>(&b)),
        (
            "mailslurper",
            mcp::tools_sha256::<mailslurper_mcp::Mailslurper>(&b),
        ),
    ];
    let tuples = product["targets"].as_array().unwrap();
    for (name, digest) in built {
        let tuple = tuples
            .iter()
            .find(|t| t["target"] == name)
            .unwrap_or_else(|| panic!("no tuple for {name}"));
        assert_eq!(tuple["tools_sha256"], digest, "{name}");
    }
}
