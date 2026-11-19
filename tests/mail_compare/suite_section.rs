//! Story D of accepted decision 0132: protocol 4's test-execution section
//! as the driver sees it. The seeded plan against an independently computed
//! golden, the address plan's disjointness, the coordinate and its owner,
//! the protocol section, the holder's control lines, the record's checks
//! (one corrupted field at a time), the exit verdict and the figures
//! recomputed from the record and the suite window. The suite itself runs in
//! the eval crate (`evals/tests/testexec.rs`); its held, enforced chunks are
//! the host proofs `api_section::testexec_real_swarmail` and
//! `api_section::testexec_real_mailpit`.

use super::*;
use crate::suite::{
    self, Cleanup, ClientCpu, Coord, FinalCleanup, Outcome, Policy, SuiteRecord, TrialRow, Trip,
    Unstarted, WaitMethod, WaitTiming,
};
use crate::suite_chunk::{self, Control, ControlState};
use serde_json::json;
use std::collections::BTreeSet;

const DRAFT_V4: &[u8] = include_bytes!("../../docs/evals/draft/protocol.v4.json");

fn draft() -> Protocol {
    Protocol::parse(DRAFT_V4).unwrap()
}

/// The claim order of seed 163, computed outside this code (Python's
/// hashlib over `testexec/163/<id>`, sorted by digest then id).
const GOLDEN: [u32; 100] = [
    129, 137, 197, 134, 131, 189, 188, 163, 166, 118, 183, 103, 167, 195, 117, 107, 164, 181, 168,
    176, 143, 108, 133, 110, 120, 174, 159, 132, 177, 102, 139, 171, 180, 138, 116, 185, 142, 152,
    191, 184, 148, 114, 160, 169, 165, 193, 157, 175, 127, 187, 123, 161, 150, 135, 105, 100, 173,
    172, 153, 149, 141, 190, 179, 126, 151, 144, 146, 147, 109, 182, 186, 198, 145, 101, 111, 170,
    125, 156, 158, 194, 140, 121, 124, 119, 112, 154, 128, 122, 155, 130, 113, 115, 104, 106, 199,
    178, 196, 136, 162, 192,
];

#[test]
fn the_seeded_plan_is_the_golden_order_with_twenty_trials_per_flow() {
    let plan = suite::plan(163);
    assert_eq!(plan.iter().map(|t| t.id).collect::<Vec<_>>(), GOLDEN);
    // Public seeds: the digest's first eight bytes, big-endian (golden).
    assert_eq!(
        (plan[0].id, plan[0].flow.as_str(), plan[0].seed),
        (129, "magic-login", 130517112964538392)
    );
    assert_eq!(
        (plan[1].id, plan[1].flow.as_str(), plan[1].seed),
        (137, "digest-lookalike", 713257238957937428)
    );
    assert_eq!((plan[99].id, plan[99].seed), (192, 18000945344956096377));
    assert!(plan.iter().all(|t| t.fixture_seed.len() == 64));
    assert_eq!(
        &plan[0].fixture_seed[..16],
        format!("{:016x}", plan[0].seed)
    );
    // Flows cycle by id in the spec's order, twenty each.
    for t in &plan {
        assert_eq!(t.flow, suite::FLOWS[((t.id - 100) % 5) as usize]);
    }
    for f in suite::FLOWS {
        assert_eq!(plan.iter().filter(|t| t.flow == f).count(), 20, "{f}");
    }
    // The same plan for every target, parallelism and repetition: it is a
    // function of the seed alone.
    assert_eq!(plan, suite::plan(163));
    assert_ne!(plan, suite::plan(164));
    let ids: BTreeSet<u32> = plan.iter().map(|t| t.id).collect();
    assert_eq!(ids, (100..200).collect());
}

#[test]
fn the_address_plan_is_disjoint_and_the_one_twelve_collision_refuses_before_send() {
    let plan = suite::plan(163);
    suite::check_disjoint(&plan).unwrap();
    assert_eq!(suite::primary(100), "test-100@suite.invalid");
    assert_eq!(suite::inbox(100), "test-100");
    assert_eq!(
        suite::owned("recipient-crowd", 101),
        [
            "test-101@suite.invalid",
            "test-101.team@suite.invalid",
            "test-101-old@suite.invalid",
            "test-1012@suite.invalid",
            "old.test-101@suite.invalid",
        ]
    );
    assert_eq!(suite::owned("magic-login", 104), ["test-104@suite.invalid"]);
    // Numbered from 1, crowd trial 1's `…2` alias is trial 12's primary.
    let at = |id: u32, flow: &str| suite::PlannedTrial {
        id,
        flow: flow.into(),
        seed: 0,
        fixture_seed: String::new(),
    };
    let low = [at(1, "recipient-crowd"), at(12, "magic-login")];
    let e = suite::check_disjoint(&low).unwrap_err();
    assert!(
        e.contains("trials 1 and 12") || e.contains("trials 12 and 1"),
        "{e}"
    );
    assert!(e.contains("test-12@suite.invalid"), "{e}");
    // A planned id twice is refused too.
    let twice = suite::plan_ids(163, [100, 100]);
    assert!(suite::check_disjoint(&twice).unwrap_err().contains("twice"));
    assert_eq!(suite::owner_of(&plan, "TEST-1012@suite.invalid"), Some(101));
    assert_eq!(suite::owner_of(&plan, "test-99@suite.invalid"), None);
}

#[test]
fn a_coordinate_is_parsed_against_the_section_and_owned_by_testexec_alone() {
    let ok = Coord::parse("testexec.mailpit.p8.r0").unwrap();
    assert_eq!(
        (ok.target.as_str(), ok.parallelism, ok.repetition),
        ("mailpit", 8, 0)
    );
    assert_eq!(ok.name(), "testexec.mailpit.p8.r0");
    for (bad, why) in [
        ("testexec.mailslurper.p1.r0", "does not start"),
        ("testexec.mailpit.p4.r0", "not one of 1, 8, 32"),
        ("testexec.mailpit.p8.r3", "outside 0..3"),
        ("testexec.mailpit.p08.r0", "is not p<n>"),
        ("testexec.nope.p8.r0", "unknown target"),
        ("testexec.mailpit.p8", "is not a testexec"),
    ] {
        let e = Coord::parse(bad).unwrap_err();
        assert!(e.contains(why), "{bad}: {e}");
    }
    let p = draft();
    let all = runner::plan_testexec(&p);
    // 0132 §11.2: 72 test-execution chunks, beside (not inside) the plan
    // collect-v4 runs today, which story E extends.
    assert_eq!(all.len(), 72);
    assert!(
        !runner::plan_v4(&p)
            .iter()
            .any(|c| matches!(c, runner::Chunk::Testexec { .. }))
    );
    let c = runner::Chunk::parse("testexec.mailpit.p8.r0", &p).unwrap();
    assert_eq!(c.name(), "testexec.mailpit.p8.r0");
    assert!(all.contains(&c));
    assert!(runner::Chunk::parse("testexec.mailslurper.p8.r0", &p).is_err());
    // run refuses the suite, api-v1 refuses it, testexec-v1 holds only it.
    use runner::Owner;
    let pre = |o, c| runner::v4_preflight_owner(o, &p, "smoke", Some(c), None, None);
    let e = pre(Owner::Run, "testexec.mailpit.p8.r0").unwrap_err();
    assert!(e.contains("not by run"), "{e}");
    let e = pre(Owner::HoldApi, "testexec.mailpit.p8.r0").unwrap_err();
    assert!(e.contains("api.mcp chunks only"), "{e}");
    let e = pre(Owner::HoldSuite, "api.mcp.mailpit.r0.f0").unwrap_err();
    assert!(e.contains("testexec chunks only"), "{e}");
    let e = pre(Owner::HoldSuite, "cost.mailpit.r0").unwrap_err();
    assert!(e.contains("testexec chunks only"), "{e}");
    assert_eq!(
        pre(Owner::HoldSuite, "testexec.mailpit.p8.r0").unwrap(),
        Some(c.clone())
    );
    // Its windows: ready, idle_ready and the suite, under testexec.p8.
    let (rows, windows) = runner::plan_chunk(&p, &c);
    assert!(rows.is_empty());
    let kinds: Vec<_> = windows
        .iter()
        .map(|w| (w.profile.as_str(), w.window))
        .collect();
    use records::WindowKind as K;
    assert_eq!(
        kinds,
        [
            ("testexec.p8", K::Ready),
            ("testexec.p8", K::IdleReady),
            ("testexec.p8", K::Suite)
        ]
    );
}

#[test]
fn the_protocol_carries_0132s_section_and_nothing_else() {
    let p = draft();
    p.validate().unwrap();
    assert_eq!(p.testexec, Some(suite::Section::specified()));
    // A tuned value is refused by name.
    let mut v: serde_json::Value = serde_json::from_slice(DRAFT_V4).unwrap();
    v["testexec"]["test_deadline_ms"] = json!(60_000);
    let e = Protocol::parse(v.to_string().as_bytes())
        .unwrap()
        .validate()
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("testexec: protocol 4's testexec section is 0132's"),
        "{e}"
    );
    // A frozen protocol 4 needs it.
    let mut v: serde_json::Value = serde_json::from_slice(DRAFT_V4).unwrap();
    v.as_object_mut().unwrap().remove("testexec");
    v["status"] = json!("frozen");
    let e = Protocol::parse(v.to_string().as_bytes())
        .unwrap()
        .validate()
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("a frozen protocol 4 needs its testexec section"),
        "{e}"
    );
    // The section stays owed until its host proofs land (story E freezes).
    assert_eq!(protocol::OWED_SECTIONS, ["testexec"]);
    // An earlier protocol refuses the field by name.
    let mut v3: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../docs/evals/history/2026-11-07-protocol.v3.json"
    ))
    .unwrap();
    v3["testexec"] = serde_json::to_value(suite::Section::specified()).unwrap();
    let e = Protocol::parse(v3.to_string().as_bytes())
        .unwrap()
        .validate()
        .unwrap_err()
        .to_string();
    assert!(e.contains("refuses testexec"), "{e}");
}

#[test]
fn every_target_has_its_narrowest_documented_cleanup_and_its_wait_best() {
    let p = draft();
    for (t, p1, p8) in [
        ("swarmail", Policy::InboxClear, Policy::InboxClear),
        ("mailpit", Policy::DeleteById, Policy::DeleteById),
        ("smtp4dev", Policy::DeleteById, Policy::DeleteById),
        ("inbucket", Policy::DeleteById, Policy::DeleteById),
        (
            "greenmail",
            Policy::GlobalClearPerTest,
            Policy::GlobalClearAfterSuite,
        ),
        ("mailhog", Policy::Unavailable, Policy::Unavailable),
        (
            "mailslurper-source-build",
            Policy::Unavailable,
            Policy::Unavailable,
        ),
        (
            "alkemio-mailslurper",
            Policy::Unavailable,
            Policy::Unavailable,
        ),
    ] {
        assert_eq!(Policy::of(t, 1), p1, "{t}");
        assert_eq!(Policy::of(t, 8), p8, "{t}");
        assert_eq!(Policy::of(t, 32), p8, "{t}");
    }
    let m = |t| suite_chunk::wait_method(&p, TargetKind::parse(t).unwrap()).unwrap();
    assert_eq!(m("swarmail"), WaitMethod::Native);
    assert_eq!(m("mailhog"), WaitMethod::Stream { backstop_ms: 1000 });
    for t in [
        "mailpit",
        "smtp4dev",
        "inbucket",
        "greenmail",
        "mailslurper-source-build",
        "alkemio-mailslurper",
    ] {
        assert_eq!(m(t), WaitMethod::Poll { interval_ms: 100 }, "{t}");
    }
}

/// A clean, complete record of `coord`: a hundred scored passes, each
/// cleaned under its policy, inside the suite window.
fn clean(coord: &Coord) -> SuiteRecord {
    let p = draft();
    let method = suite_chunk::wait_method(&p, TargetKind::parse(&coord.target).unwrap()).unwrap();
    let mut rec = SuiteRecord::new(coord, 163, method);
    rec.window_start = Some(Trip {
        sent_ns: 0,
        received_ns: 50,
        owner_t_ns: 25,
    });
    let cleanup = match rec.policy {
        Policy::GlobalClearAfterSuite => Cleanup::Deferred,
        Policy::Unavailable => Cleanup::NotAvailable,
        _ => Cleanup::Done { removed: 1 },
    };
    for (i, t) in rec.plan.clone().iter().enumerate() {
        let claim = 100 + i as u64 * 1_000;
        rec.rows.push(TrialRow {
            id: t.id,
            flow: t.flow.clone(),
            seed: t.seed,
            worker: i as u32 % coord.parallelism,
            claim_ns: claim,
            end_ns: claim + 900,
            outcome: Outcome::Pass,
            cause: None,
            evidence: Vec::new(),
            scored: Some(true),
            foreign_seen: (i % 3) as u64,
            owned_seen: 1,
            wait: Some(WaitTiming {
                call_ns: claim + 10,
                return_ns: claim + 500,
                count: 1,
                ack_ns: Some(claim + 400),
                ack_to_return_ns: Some(100),
                ack_absent: None,
                cause: "poll".into(),
            }),
            wait_absent: None,
            cleanup: cleanup.clone(),
        });
    }
    rec.first_claim_ns = Some(100);
    rec.last_end_ns = Some(100 + 99 * 1_000 + 900);
    let last = rec.last_end_ns.unwrap();
    rec.window_end = Some(Trip {
        sent_ns: last + 1,
        received_ns: last + 60,
        owner_t_ns: last + 30,
    });
    rec.client_cpu = Some(ClientCpu {
        user_usec: 7,
        system_usec: 3,
    });
    if rec.policy == Policy::GlobalClearAfterSuite {
        rec.final_cleanup = Some(FinalCleanup {
            start_ns: last + 100,
            end_ns: last + 200,
            cleanup: Cleanup::Done { removed: 180 },
        });
    }
    rec
}

fn coord(t: &str, par: u32) -> Coord {
    Coord {
        target: t.into(),
        parallelism: par,
        repetition: 0,
    }
}

#[test]
fn a_record_is_checked_whole_and_refused_one_corrupted_field_at_a_time() {
    let c = coord("mailpit", 8);
    let rec = clean(&c);
    suite::check_record(&rec, &c).unwrap();
    assert!(suite::failures(&rec).is_empty());
    let chunk = runner::Chunk::parse(&c.name(), &draft()).unwrap();
    suite_chunk::check_against(&rec, &chunk, &draft()).unwrap();
    let refuse = |what: &str, edit: &dyn Fn(&mut SuiteRecord)| {
        let mut r = rec.clone();
        edit(&mut r);
        let e = suite::check_record(&r, &c).unwrap_err();
        assert!(e.contains(what), "{what}: {e}");
    };
    refuse("is not of", &|r| r.parallelism = 32);
    refuse("the plan is not seed", &|r| r.plan.swap(0, 1));
    refuse("policy", &|r| r.policy = Policy::Unavailable);
    refuse("accounted for twice", &|r| {
        r.unstarted.push(Unstarted {
            id: 129,
            reason: "x".into(),
        })
    });
    refuse("not accounted for", &|r| {
        r.rows.pop();
    });
    refuse("is not planned", &|r| r.rows[0].id = 7);
    refuse("flow or seed", &|r| r.rows[0].seed ^= 1);
    refuse("worker 8 of 8", &|r| r.rows[0].worker = 8);
    refuse("ends before its claim", &|r| r.rows[0].end_ns = 0);
    refuse("a pass the scorer did not pass", &|r| {
        r.rows[0].scored = Some(false)
    });
    refuse("a pass without its delete_by_id cleanup", &|r| {
        r.rows[0].cleanup = Cleanup::NotRun { reason: "x".into() }
    });
    refuse("without a cause", &|r| r.rows[0].outcome = Outcome::Timeout);
    refuse("interference without causal evidence", &|r| {
        r.rows[0].outcome = Outcome::Interference;
        r.rows[0].cause = Some("x".into());
    });
    refuse("evidence on a timeout row", &|r| {
        r.rows[0].outcome = Outcome::Timeout;
        r.rows[0].cause = Some("x".into());
        r.rows[0].evidence = vec!["x".into()];
    });
    refuse("cleanup Deferred under delete_by_id", &|r| {
        r.rows[0].outcome = Outcome::Timeout;
        r.rows[0].cause = Some("x".into());
        r.rows[0].cleanup = Cleanup::Deferred;
    });
    refuse("ack_to_return disagrees", &|r| {
        r.rows[0].wait.as_mut().unwrap().ack_to_return_ns = Some(1)
    });
    refuse("ack_to_return disagrees", &|r| {
        r.rows[0].wait.as_mut().unwrap().ack_absent = Some("x".into())
    });
    refuse("outside the trial", &|r| {
        r.rows[0].wait.as_mut().unwrap().return_ns = u64::MAX
    });
    refuse("neither a wait nor its absence", &|r| {
        r.rows[0].wait_absent = Some("x".into())
    });
    refuse("first claim and last end", &|r| r.last_end_ns = Some(1));
    refuse("without a suite window", &|r| r.window_end = None);
    refuse("does not enclose", &|r| {
        r.window_start.as_mut().unwrap().received_ns = 10_000
    });
    refuse("without the client's CPU", &|r| r.client_cpu = None);
    // The budgets are absolute: 30 s from a claim, and no claim or end at
    // or after the suite cutoff, 540 s from the first claim.
    let ms = 1_000_000;
    refuse("trial 129: runs past its test deadline", &|r| {
        r.rows[0].end_ns = r.rows[0].claim_ns + suite::TEST_DEADLINE_MS * ms + 1;
        r.last_end_ns = r.rows.iter().map(|x| x.end_ns).max();
    });
    // The last row moved, its wait with it, to a claim `at` lasting `len`.
    let late = |r: &mut SuiteRecord, at: u64, len: u64| {
        let row = r.rows.last_mut().unwrap();
        let shift = at - row.claim_ns;
        let w = row.wait.as_mut().unwrap();
        w.call_ns = at;
        w.return_ns = at + len;
        w.ack_ns = Some(at);
        w.ack_to_return_ns = Some(len as i64);
        (row.claim_ns, row.end_ns) = (at, at + len);
        r.last_end_ns = Some(at + len);
        r.window_end.as_mut().unwrap().sent_ns += shift;
    };
    let cutoff = 100 + suite::SUITE_DEADLINE_MS * ms;
    refuse("trial 192: runs past the suite cutoff", &|r| {
        late(r, cutoff, 900)
    });
    refuse("trial 192: runs past the suite cutoff", &|r| {
        late(r, cutoff - 10, 20)
    });
    let mut r = rec.clone();
    late(&mut r, cutoff - 20, 20);
    suite::check_record(&r, &c).unwrap();
    // A trial ending exactly on its bound is inside it.
    let mut r = rec.clone();
    r.rows[0].end_ns = r.rows[0].claim_ns + suite::TEST_DEADLINE_MS * ms;
    r.last_end_ns = r.rows.iter().map(|x| x.end_ns).max();
    r.window_end.as_mut().unwrap().sent_ns = r.last_end_ns.unwrap();
    suite::check_record(&r, &c).unwrap();
    // A suite that claimed nothing never opened its window.
    refuse("started nothing opened its window", &|r| {
        let ids: Vec<u32> = r.rows.drain(..).map(|x| x.id).collect();
        r.unstarted = ids
            .into_iter()
            .map(|id| Unstarted {
                id,
                reason: "chunk_deadline".into(),
            })
            .collect();
        (r.first_claim_ns, r.last_end_ns, r.window_end) = (None, None, None);
    });
    refuse("a final clear under delete_by_id", &|r| {
        r.final_cleanup = Some(FinalCleanup {
            start_ns: 0,
            end_ns: 0,
            cleanup: Cleanup::Deferred,
        })
    });
    refuse("a blocked suite started", &|r| r.blocked = Some("x".into()));
    refuse("without its reason", &|r| {
        let row = r.rows.pop().unwrap();
        r.unstarted.push(Unstarted {
            id: row.id,
            reason: String::new(),
        });
        r.last_end_ns = r.rows.iter().map(|x| x.end_ns).max();
    });
    // The protocol: its seed and the target's wait-best.
    let mut r = rec.clone();
    r.method = WaitMethod::Poll { interval_ms: 10 };
    assert!(
        suite_chunk::check_against(&r, &chunk, &draft())
            .unwrap_err()
            .contains("wait-best")
    );
    // A global-only suite owes its one clear after the window.
    let g = coord("greenmail", 8);
    let rec = clean(&g);
    suite::check_record(&rec, &g).unwrap();
    let mut r = rec.clone();
    r.final_cleanup = None;
    assert!(
        suite::check_record(&r, &g)
            .unwrap_err()
            .contains("global clear is missing")
    );
    let mut r = rec.clone();
    r.final_cleanup.as_mut().unwrap().start_ns = 10;
    assert!(
        suite::check_record(&r, &g)
            .unwrap_err()
            .contains("did not follow the suite window")
    );
}

#[test]
fn an_unavailable_cleanup_policy_seals_rows_that_record_no_cleanup() {
    // MailHog documents no narrow delete and no clear the suite may use:
    // every row records the capability as unavailable, which is its
    // policy's proof, and no suite clear is owed.
    for par in [1, 8, 32] {
        let c = coord("mailhog", par);
        let rec = clean(&c);
        assert_eq!(rec.policy, Policy::Unavailable);
        assert!(rec.rows.iter().all(|r| r.cleanup == Cleanup::NotAvailable));
        suite::check_record(&rec, &c).unwrap();
        suite::cleanup_proved(&rec).unwrap();
        assert!(suite::failures(&rec).is_empty());
        let chunk = runner::Chunk::parse(&c.name(), &draft()).unwrap();
        suite_chunk::check_against(&rec, &chunk, &draft()).unwrap();
        // Any other disposition is refused under it.
        for other in [
            Cleanup::Deferred,
            Cleanup::Done { removed: 1 },
            Cleanup::Failed { reason: "x".into() },
        ] {
            let mut r = rec.clone();
            r.rows[0].outcome = Outcome::Timeout;
            r.rows[0].cause = Some("timeout: waited".into());
            r.rows[0].cleanup = other.clone();
            let e = suite::check_record(&r, &c).unwrap_err();
            assert!(e.contains("under unavailable"), "{other:?}: {e}");
        }
    }
}

#[test]
fn only_a_hundred_scored_passes_with_proved_cleanup_is_a_clean_suite() {
    let c = coord("swarmail", 1);
    let rec = clean(&c);
    assert!(suite::failures(&rec).is_empty());
    // A timeout, an unstarted trial, a failed cleanup and a failed suite
    // clear are each the target's named loss (exit 1, sealed).
    let mut r = rec.clone();
    r.rows[3].outcome = Outcome::Timeout;
    r.rows[3].cause = Some("timeout: waited".into());
    let f = suite::failures(&r);
    assert_eq!(f.len(), 1);
    assert!(f[0].contains("timeout: timeout: waited"), "{f:?}");
    let mut r = rec.clone();
    let row = r.rows.pop().unwrap();
    r.unstarted.push(Unstarted {
        id: row.id,
        reason: "suite_deadline".into(),
    });
    r.last_end_ns = r.rows.iter().map(|x| x.end_ns).max();
    suite::check_record(&r, &c).unwrap();
    assert_eq!(
        suite::failures(&r),
        ["testexec.swarmail.p1.r0: 1 trial(s) unstarted: suite_deadline"]
    );
    let mut r = rec.clone();
    r.rows[0].outcome = Outcome::Harness;
    r.rows[0].cause = Some("scored pass, cleanup not proved".into());
    r.rows[0].cleanup = Cleanup::Failed {
        reason: "status_500".into(),
    };
    suite::check_record(&r, &c).unwrap();
    assert_eq!(suite::failures(&r).len(), 2);
    // A blocked suite is a loss, never an empty success.
    let mut r = SuiteRecord::new(&c, 163, WaitMethod::Native);
    r.block("target_exited");
    suite::check_record(&r, &c).unwrap();
    assert_eq!(
        suite::failures(&r),
        ["testexec.swarmail.p1.r0: blocked: target_exited"]
    );
    let g = coord("greenmail", 32);
    let mut r = clean(&g);
    r.final_cleanup.as_mut().unwrap().cleanup = Cleanup::Failed { reason: "x".into() };
    assert_eq!(suite::failures(&r).len(), 1);
    assert!(suite::cleanup_proved(&r).is_err());
}

#[test]
fn the_figures_are_recomputed_from_the_record_and_the_suite_window() {
    let c = coord("mailpit", 8);
    let mut rec = clean(&c);
    rec.rows[1].outcome = Outcome::WrongMessage;
    rec.rows[1].cause = Some("oracle: x".into());
    let r = suite::report(&rec);
    assert_eq!(r["coordinate"], json!("testexec.mailpit.p8.r0"));
    assert_eq!(r["planned"], json!(100));
    assert_eq!(
        r["by_class"],
        json!({"pass": 99, "timeout": 0, "wrong_message": 1, "interference": 0, "harness": 0})
    );
    assert_eq!(r["foreign_seen"], json!(99));
    assert_eq!(r["suite_wall_ns"], json!(99_900));
    assert_eq!(r["trial_wall_ns"]["p50"], json!(900));
    assert_eq!(r["wait_ns"]["n"], json!(100));
    assert_eq!(r["ack_to_return_ns"]["p95"], json!(100));
    assert_eq!(r["client_cpu_usec"], json!(10));
    assert_eq!(r["window_round_trips_ns"], json!([50, 59]));
    assert_eq!(r["at_equal_results"], json!(false));
    // The target's side: CPU and memory inside the window only.
    let leaf = "target-0001";
    let w = records::WindowRow {
        row_version: records::RESOURCE_ROW_VERSION,
        run_id: "r".into(),
        launch: leaf.into(),
        target: TargetKind::Mailpit,
        profile: "testexec.p8".into(),
        repetition: 0,
        window: records::WindowKind::Suite,
        start_ns: 1_000,
        end_ns: 2_000,
        cpu_start: Some(resources::CpuStat {
            usage_usec: 10,
            user_usec: 6,
            system_usec: 4,
            nr_throttled: 0,
            throttled_usec: 0,
        }),
        cpu_end: Some(resources::CpuStat {
            usage_usec: 310,
            user_usec: 206,
            system_usec: 104,
            nr_throttled: 2,
            throttled_usec: 9,
        }),
        events_start: Some(resources::MemEvents::default()),
        events_end: Some(resources::MemEvents::default()),
        affinity_held: Some(true),
        scratch_bytes: None,
        outcome: records::WindowOutcome::Measured,
    };
    let sample = |t_ns, mem| {
        records::ResourceRow::Sample(records::SampleRow {
            row_version: records::RESOURCE_ROW_VERSION,
            run_id: "r".into(),
            launch: leaf.into(),
            target: TargetKind::Mailpit,
            t_ns,
            memory_current: mem,
            anon: mem / 2,
            file: 0,
            kernel: 0,
            pids: 3,
        })
    };
    let rows = vec![
        sample(500, 9_000),
        sample(1_000, 100),
        sample(1_500, 300),
        sample(2_000, 200),
        records::ResourceRow::Window(w),
    ];
    let r = suite_chunk::resource_report(8, &rows);
    assert_eq!(r["target"]["usage_usec"], json!(300));
    assert_eq!(r["target"]["nr_throttled"], json!(2));
    assert_eq!(r["peak_memory_current"], json!(300));
    assert_eq!(r["samples"], json!(3));
    assert_eq!(suite_chunk::suite_report(&rec, &rows)["resources"], r);
    assert_eq!(
        suite_chunk::resource_report(32, &rows),
        serde_json::Value::Null
    );
}

#[test]
fn the_holders_control_lines_are_closed_ordered_and_bound_to_its_window() {
    use suite_chunk::{admit_control, parse_control, windows_agree};
    assert_eq!(
        parse_control(br#"{"cmd":"window_start","id":"suite"}"#).unwrap(),
        Control::WindowStart { id: "suite".into() }
    );
    for bad in [
        &br#"{"cmd":"idle_fill"}"#[..],
        br#"{"cmd":"window_end"}"#,
        br#"{"cmd":"window_start","id":"suite","x":1}"#,
        b"nope",
    ] {
        assert!(
            parse_control(bad).is_err(),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
    let long = format!(
        "{{\"cmd\":\"window_start\",\"id\":\"{}\"}}",
        "x".repeat(2000)
    );
    assert!(
        parse_control(long.as_bytes())
            .unwrap_err()
            .contains("exceeds")
    );
    let start = Control::WindowStart { id: "suite".into() };
    let end = Control::WindowEnd { id: "suite".into() };
    let mut st = ControlState::default();
    assert!(
        admit_control(
            &Control::WindowStart {
                id: "list.c1".into()
            },
            &st
        )
        .unwrap_err()
        .contains("not the suite window")
    );
    assert!(admit_control(&end, &st).unwrap_err().contains("not opened"));
    admit_control(&start, &st).unwrap();
    let snap = |t| resources::Snap {
        t_ns: t,
        cpu: Default::default(),
        events: Default::default(),
    };
    st.opened = Some(snap(25));
    assert!(
        admit_control(&start, &st)
            .unwrap_err()
            .contains("already opened")
    );
    let c = coord("mailpit", 8);
    let rec = clean(&c);
    let record = Control::Record {
        testexec: Box::new(rec.clone()),
    };
    assert!(
        admit_control(&record, &st)
            .unwrap_err()
            .contains("still open")
    );
    st.opened = None;
    st.closed = Some((snap(25), snap(rec.window_end.as_ref().unwrap().owner_t_ns)));
    windows_agree(&st, &rec).unwrap();
    let mut moved = rec.clone();
    moved.window_start.as_mut().unwrap().owner_t_ns = 26;
    assert!(
        windows_agree(&st, &moved)
            .unwrap_err()
            .contains("not the holder's")
    );
    // Nothing started: the window must not have been opened, and vice versa.
    let mut blocked = SuiteRecord::new(&c, 163, WaitMethod::Poll { interval_ms: 100 });
    blocked.block("target_exited");
    assert!(windows_agree(&st, &blocked).is_err());
    windows_agree(&ControlState::default(), &blocked).unwrap();
    assert!(windows_agree(&ControlState::default(), &rec).is_err());
    // So does a suite whose cutoff came before its first claim: every id
    // unstarted with its reason, no window, a sealed loss (exit 1), never
    // a refusal (exit 2).
    let mut none = SuiteRecord::new(&c, 163, WaitMethod::Poll { interval_ms: 100 });
    none.unstarted = none
        .plan
        .iter()
        .map(|t| Unstarted {
            id: t.id,
            reason: "chunk_deadline".into(),
        })
        .collect();
    windows_agree(&ControlState::default(), &none).unwrap();
    let chunk = runner::Chunk::parse(&c.name(), &draft()).unwrap();
    suite_chunk::check_against(&none, &chunk, &draft()).unwrap();
    suite::cleanup_proved(&none).unwrap();
    assert_eq!(
        suite::failures(&none),
        ["testexec.mailpit.p8.r0: 100 trial(s) unstarted: chunk_deadline"]
    );
    st.record = Some(rec);
    assert!(
        admit_control(&end, &st)
            .unwrap_err()
            .contains("after the record")
    );
    // The record line is the one long line; it parses with its record.
    let line = serde_json::to_vec(&json!({"cmd": "record", "testexec": clean(&c)})).unwrap();
    assert!(line.len() > suite_chunk::MAX_CONTROL_LINE);
    assert!(matches!(
        parse_control(&line).unwrap(),
        Control::Record { .. }
    ));
}
