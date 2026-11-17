//! Protocol 4, lock 5 and the enforced cost section (accepted decision
//! 0132, story A, issue #359). Fixtures are authored by the test from the
//! contract. What needs a delegated cgroup v2 hierarchy (the join, the
//! limits, `cgroup.kill`) is proved by the `#[ignore]`d real proofs at the
//! end, which fail rather than skip; everything else runs on fixture trees
//! of plain files, on any host.

use super::*;
use serde_json::{Value, json};

const DRAFT_PROTOCOL: &[u8] = include_bytes!("../../docs/evals/draft/protocol.v4.json");
const DRAFT_LOCK: &[u8] = include_bytes!("../../docs/evals/draft/sources.lock.v5.json");

fn draft() -> Value {
    json(DRAFT_PROTOCOL)
}

fn lock5() -> Value {
    json(DRAFT_LOCK)
}

fn p4(v: Value) -> Protocol {
    protocol_from(v).expect("parses")
}

fn why(v: Value) -> String {
    refusals(p4(v).validate())
}

/// The draft as a frozen protocol that carries what a freeze needs, to be
/// validated with nothing owed.
fn frozen_ready() -> Value {
    let mut v = draft();
    v["status"] = json!("frozen");
    v["unresolved"] = json!([]);
    v["clients_sha256"] = json!({"mailpit": "a".repeat(64)});
    for c in v["capabilities"].as_array_mut().unwrap() {
        for k in [
            "server_search",
            "native_wait",
            "retention_cap",
            "raw_source",
        ] {
            if c[k]["status"] == "not_inspected" {
                c[k] = json!({"status": "not_supported", "cite": "authored: closed for the test"});
            }
        }
    }
    v
}

// ---- T1.1 ------------------------------------------------------------------

#[test]
fn v4_protocol_validates_with_its_id_predecessor_and_reason() {
    let p = p4(draft());
    p.validate().expect("the draft validates");
    assert_eq!(p.protocol_version, protocol::PROTOCOL_VERSION_4);
    assert_eq!(p.id, "fair-mail-comparison-v4");
    assert_eq!(p.supersedes.as_deref(), Some("fair-mail-comparison-v3"));
    assert!(p.change_reason.as_deref().unwrap().contains("0132"));
}

#[test]
fn v4_refuses_a_wrong_id_predecessor_or_reason() {
    let mut v = draft();
    v["id"] = json!("fair-mail-comparison-v3");
    assert!(why(v).contains("protocol 4's id is fair-mail-comparison-v4"));
    let mut v = draft();
    v["supersedes"] = json!("fair-mail-comparison-v2");
    assert!(why(v).contains("protocol 4 supersedes fair-mail-comparison-v3"));
    let mut v = draft();
    v["change_reason"] = json!("a reason that cites nothing");
    assert!(why(v).contains("change_reason must cite decision 0132"));
    let mut v = draft();
    v["change_reason"] = json!("   ");
    assert!(why(v).contains("must state its change_reason"));
}

#[test]
fn v4_keeps_every_v3_rule() {
    // A missing target.
    let mut v = draft();
    v["targets"].as_array_mut().unwrap().pop();
    assert!(why(v).contains("protocol_version 4 needs target alkemio-mailslurper"));
    // A missing rotation.
    let mut v = draft();
    v["statistics"]["orders"].as_array_mut().unwrap().pop();
    let w = why(v);
    assert!(w.contains("target orders") || w.contains("rotation"), "{w}");
    // A missing capability record.
    let mut v = draft();
    v["capabilities"].as_array_mut().unwrap().pop();
    assert!(why(v).contains("capabilities: alkemio-mailslurper needs exactly one record"));
    // A product scope that is not the provenance's.
    let mut v = draft();
    v["products"][0]["scope"] = json!("official_artifact");
    assert!(why(v).contains("products: swarmail scope"));
    // A product with no scope.
    let mut v = draft();
    v["products"][0].as_object_mut().unwrap().remove("scope");
    assert!(why(v).contains("products: swarmail states no scope"));
    // The official MailSlurper row must stay does_not_start.
    let mut v = draft();
    v["does_not_start"] = json!([]);
    assert!(why(v).contains("publishes the official mailslurper row as does_not_start"));
    let mut v = draft();
    v.as_object_mut().unwrap().remove("does_not_start");
    assert!(why(v).contains("publishes the official mailslurper row as does_not_start"));
}

#[test]
fn every_cross_pairing_of_protocols_and_locks_refuses() {
    let p2 = Protocol::parse(PROTOCOL).unwrap();
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let p4 = p4(draft());
    let l3 = SourceLock::parse(SOURCES).unwrap();
    let l4 = SourceLock::parse(SOURCES_V4).unwrap();
    let l5 = lock_from(lock5()).unwrap();
    for (name, p) in [("2", &p2), ("3", &p3), ("4", &p4)] {
        for (lname, l) in [("3", &l3), ("4", &l4), ("5", &l5)] {
            let right = matches!((name, lname), ("2", "3") | ("3", "4") | ("4", "5"));
            let got = protocol::pairing(p, l);
            assert_eq!(got.is_ok(), right, "protocol {name} with lock {lname}");
            if !right {
                let w = refusals(got);
                assert!(
                    w.contains(&format!(
                        "protocol_version {name} pairs only with lock_version"
                    )),
                    "{w}"
                );
            }
        }
    }
}

// ---- T1.2 ------------------------------------------------------------------

#[test]
fn v4_resources_block_refuses_each_absent_or_different_value() {
    let cases: [(&str, Value, &str); 10] = [
        ("enforced", json!(false), "protocol 4's budget is enforced"),
        (
            "profile",
            json!("unenforced_local"),
            "protocol 4's profile is enforced",
        ),
        (
            "mechanism",
            json!("docker"),
            "mechanism is cgroup_v2_user_scope",
        ),
        (
            "cpu_placement",
            json!("quota_only"),
            "cpu_placement is affinity_plus_quota",
        ),
        ("cpus", json!(4), "cpus is 2"),
        ("memory_mib", json!(4096), "memory_mib is 2048"),
        ("swap_mib", json!(512), "swap_mib is 0"),
        ("tasks", json!(8192), "tasks is 4096"),
        (
            "sample_interval_ms",
            json!(250),
            "sample_interval_ms is 100",
        ),
        ("chunk_deadline_ms", json!(600_000), "chunk_deadline_ms"),
    ];
    for (field, value, expect) in cases {
        let mut v = draft();
        v["resources"][field] = value.clone();
        let w = why(v);
        assert!(
            w.contains(expect) && w.contains(field),
            "{field}={value}: {w}"
        );
        // And absent: the field omitted refuses too.
        let mut v = draft();
        v["resources"].as_object_mut().unwrap().remove(field);
        // A field the type requires is refused at parse, the rest by name.
        let w = match protocol_from(v) {
            Ok(p) => refusals(p.validate()),
            Err(e) => e.to_string(),
        };
        // The refusal names the field, whether the type or the rule refuses.
        assert!(w.contains(field), "{field} absent: {w}");
    }
    let mut v = draft();
    v["resources"]["chunk_deadline_ms"] = json!(0);
    assert!(why(v).contains("chunk_deadline_ms Some(0) is outside 1..=480000"));
}

// ---- R1.1 ------------------------------------------------------------------

fn with_deadline(ms: u64, startup: u64, cleanup: u64) -> Value {
    let mut v = draft();
    v["resources"]["chunk_deadline_ms"] = json!(ms);
    v["resources"]["startup_deadline_ms"] = json!(startup);
    v["resources"]["cleanup_deadline_ms"] = json!(cleanup);
    v
}

#[test]
fn v4_chunk_deadline_at_its_ceiling_passes_and_one_above_refuses() {
    // The draft's budgets: 560000 - 30000 - 30000 - 2 x 10000 = 480000.
    let ceiling = protocol::CHUNK_KILL_MS - protocol::CHUNK_MARGIN_MS - 30_000 - 2 * 10_000;
    assert_eq!(ceiling, 480_000);
    p4(with_deadline(ceiling, 30_000, 10_000))
        .validate()
        .expect("a deadline exactly at the ceiling passes");
    let w = why(with_deadline(ceiling + 1, 30_000, 10_000));
    assert!(w.contains("chunk_deadline_ms Some(480001)"), "{w}");
    let w = why(with_deadline(0, 30_000, 10_000));
    assert!(w.contains("chunk_deadline_ms Some(0)"), "{w}");
}

#[test]
fn v4_chunk_deadline_ceiling_moves_with_the_startup_and_cleanup_budgets() {
    // The draft's 480000 passes under the draft's budgets and refuses once a
    // larger startup or cleanup budget lowers the ceiling beneath it.
    p4(with_deadline(480_000, 30_000, 10_000))
        .validate()
        .unwrap();
    let ceiling = |startup: u64, cleanup: u64| {
        protocol::CHUNK_KILL_MS - protocol::CHUNK_MARGIN_MS - startup - 2 * cleanup
    };
    // startup 90 s: ceiling 560000 - 30000 - 90000 - 20000 = 420000.
    assert_eq!(ceiling(90_000, 10_000), 420_000);
    let w = why(with_deadline(480_000, 90_000, 10_000));
    assert!(w.contains("is outside 1..=420000"), "{w}");
    p4(with_deadline(420_000, 90_000, 10_000))
        .validate()
        .unwrap();
    // cleanup 30 s: ceiling 560000 - 30000 - 30000 - 60000 = 440000.
    assert_eq!(ceiling(30_000, 30_000), 440_000);
    p4(with_deadline(440_000, 30_000, 30_000))
        .validate()
        .unwrap();
    let w = why(with_deadline(440_001, 30_000, 30_000));
    assert!(w.contains("is outside 1..=440000"), "{w}");
    // A shorter startup budget raises it.
    assert_eq!(ceiling(10_000, 10_000), 500_000);
    p4(with_deadline(500_000, 10_000, 10_000))
        .validate()
        .unwrap();
}

#[test]
fn v4_chunk_deadline_refusal_names_the_ceiling() {
    let w = why(with_deadline(protocol::CHUNK_KILL_MS, 30_000, 10_000));
    assert!(w.contains("1..=480000"), "{w}");
    for part in [
        "CHUNK_KILL_MS 560000",
        "CHUNK_MARGIN_MS 30000",
        "startup_deadline_ms 30000",
        "cleanup_deadline_ms 10000",
    ] {
        assert!(w.contains(part), "{part}: {w}");
    }
}

#[test]
fn v4_chunk_deadline_ceiling_is_480000_for_the_draft_budgets() {
    // The wrapper's SIGTERM instant is the constant, not a round number.
    assert_eq!(protocol::CHUNK_KILL_MS, 560_000);
    let d = p4(draft());
    let r = &d.resources;
    let ceiling = protocol::CHUNK_KILL_MS
        - protocol::CHUNK_MARGIN_MS
        - r.startup_deadline_ms
        - 2 * r.cleanup_deadline_ms;
    assert_eq!(ceiling, 480_000);
    assert_eq!(r.chunk_deadline_ms, Some(ceiling));
}

#[test]
fn the_v4_draft_and_the_smoke_fixture_sit_at_their_ceiling_and_validate() {
    let smoke: Value = json(include_bytes!("v4-smoke.json"));
    for (name, v) in [("the draft", draft()), ("the smoke fixture", smoke)] {
        assert_eq!(
            v["resources"]["chunk_deadline_ms"],
            json!(480_000),
            "{name}"
        );
        assert_eq!(
            v["resources"]["startup_deadline_ms"],
            json!(30_000),
            "{name}"
        );
        assert_eq!(
            v["resources"]["cleanup_deadline_ms"],
            json!(10_000),
            "{name}"
        );
        p4(v).validate().unwrap_or_else(|r| panic!("{name}: {r}"));
    }
}

#[test]
fn the_v4_draft_states_a_chunk_deadline_of_480000() {
    assert_eq!(draft()["resources"]["chunk_deadline_ms"], json!(480_000));
    assert_eq!(p4(draft()).resources.chunk_deadline_ms, Some(480_000));
}

#[test]
fn earlier_protocols_refuse_each_v4_resources_field() {
    for (base, version) in [(PROTOCOL, 2), (PROTOCOL_V3, 3)] {
        for (field, value) in [
            ("mechanism", json!("cgroup_v2_user_scope")),
            ("cpu_placement", json!("affinity_plus_quota")),
            ("swap_mib", json!(0)),
            ("tasks", json!(4096)),
            ("sample_interval_ms", json!(100)),
            ("chunk_deadline_ms", json!(480_000)),
        ] {
            let mut v = json(base);
            v["resources"][field] = value;
            let w = refusals(protocol_from(v).unwrap().validate());
            assert!(
                w.contains(&format!(
                    "protocol_version {version} refuses resources.{field}; only protocol_version 4 carries it"
                )),
                "{version} {field}: {w}"
            );
        }
        let mut v = json(base);
        v["headline"] = json!({"profile": "cost-c8-reused"});
        let w = refusals(protocol_from(v).unwrap().validate());
        assert!(w.contains("refuses headline"), "{w}");
    }
}

#[test]
fn v2_and_v3_history_bytes_round_trip_unchanged() {
    for bytes in [PROTOCOL, PROTOCOL_V3] {
        let p = Protocol::parse(bytes).unwrap();
        p.validate().expect("the frozen record still validates");
        let again = serde_json::to_value(&p).unwrap();
        assert_eq!(again, json(bytes), "no new field leaks into an old record");
        // The v4 keys are absent from the bytes, not null.
        let text = serde_json::to_string(&p).unwrap();
        for key in [
            "mechanism",
            "cpu_placement",
            "swap_mib",
            "headline",
            "clients_sha256",
        ] {
            assert!(!text.contains(key), "{key}");
        }
    }
}

// ---- T1.3 ------------------------------------------------------------------

#[test]
fn frozen_v4_cost_table_is_exact() {
    p4(frozen_ready())
        .validate_owing(&[])
        .expect("a ready frozen v4 validates");
    let check = |v: Value, expect: &str| {
        let w = refusals(p4(v).validate_owing(&[]));
        assert!(w.contains(expect), "{expect}: {w}");
    };
    // Removed.
    let mut v = frozen_ready();
    v["throughput"].as_array_mut().unwrap().pop();
    check(v, "carries exactly the 4 cost profiles");
    // Added.
    let mut v = frozen_ready();
    let mut extra = v["throughput"][0].clone();
    extra["name"] = json!("cost-extra");
    v["throughput"].as_array_mut().unwrap().push(extra);
    check(v, "carries exactly the 4 cost profiles");
    // Renamed.
    let mut v = frozen_ready();
    v["throughput"][3]["name"] = json!("cost-c32-other");
    check(v, "needs the profile cost-c32-fresh");
    // Each field changed.
    for (field, value) in [
        ("repetitions", json!(2)),
        ("warmup_messages", json!(0)),
        ("messages", json!(2999)),
        ("sessions", json!(7)),
        ("connection", json!("fresh")),
        ("send_deadline_ms", json!(299_999)),
        ("verify_deadline_ms", json!(59_999)),
    ] {
        let mut v = frozen_ready();
        v["throughput"][1][field] = value;
        check(
            v,
            "cost-c8-reused: a frozen protocol 4's cost table is exact",
        );
    }
}

#[test]
fn v4_refuses_thirty_three_sessions() {
    for status in ["draft", "frozen"] {
        let mut v = draft();
        v["status"] = json!(status);
        v["throughput"][3]["sessions"] = json!(33);
        assert!(why(v).contains("33 sessions is above protocol 4's bound of 32"));
    }
    // 32 is the bound itself.
    p4(draft()).validate().unwrap();
}

#[test]
fn v4_refuses_a_wait_profile() {
    let mut v = draft();
    let w = json(PROTOCOL_V3)["waits"][0].clone();
    v["waits"] = json!([w]);
    assert!(why(v).contains("protocol 4 refuses wait profiles"));
}

#[test]
fn v4_headline_is_required_and_is_cost_c8() {
    let mut v = draft();
    v.as_object_mut().unwrap().remove("headline");
    assert!(why(v).contains("protocol 4 needs a headline profile"));
    let mut v = draft();
    v["headline"]["profile"] = json!("cost-c1-reused");
    assert!(why(v).contains("headline profile is cost-c8-reused, not \"cost-c1-reused\""));
    // Frozen: it names a declared profile.
    let mut v = frozen_ready();
    v["throughput"][1]["name"] = json!("cost-renamed");
    let w = refusals(p4(v).validate_owing(&[]));
    assert!(
        w.contains("headline profile \"cost-c8-reused\" is not a declared profile"),
        "{w}"
    );
}

// ---- T1.4 ------------------------------------------------------------------

#[test]
fn v4_refuses_a_cap_other_than_150000() {
    for target in ["swarmail", "mailpit"] {
        let mut v = draft();
        let r = v["storage"]["retention"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["target"] == target)
            .unwrap();
        r["max_messages"] = json!(100_000);
        assert!(why(v).contains(&format!(
            "protocol 4 launches {target} with the one cap 150000"
        )));
    }
    // A target with no cap flag stays uncapped.
    let p = p4(draft());
    let e = p
        .storage
        .retention
        .iter()
        .find(|r| r.target == TargetKind::Mailhog)
        .unwrap();
    assert_eq!(e.max_messages, None);
}

#[test]
fn v4_refuses_a_null_swarmail_cap() {
    let mut v = draft();
    let r = v["storage"]["retention"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["target"] == "swarmail")
        .unwrap();
    r["max_messages"] = Value::Null;
    let w = why(v);
    assert!(
        w.contains("protocol 4 launches swarmail with a cap") && w.contains("not null"),
        "{w}"
    );
    // The same entry removed altogether is the same refusal.
    let mut v = draft();
    v["storage"]["retention"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["target"] != "swarmail");
    assert!(why(v).contains("protocol 4 launches swarmail with a cap"));
}

#[test]
fn a_v4_swarmail_spec_is_never_built_uncapped() {
    let lock = lock_from(lock5()).unwrap();
    let bins = vec!["swarmail=/never/swarmail".to_owned()];
    let build = |p: &Protocol, l: &SourceLock| {
        runner::launch_specs_of(p, l, &bins, &[], &[TargetKind::Swarmail])
    };
    // Protocol 4 with no cap stated: an error, not `--max-per-inbox 0`.
    let mut v = draft();
    for r in v["storage"]["retention"].as_array_mut().unwrap() {
        if r["target"] == "swarmail" {
            r["max_messages"] = Value::Null;
        }
    }
    let w = build(&p4(v), &lock).unwrap_err();
    assert!(w.contains("needs the retention cap"), "{w}");
    // With its cap it carries 150000.
    let specs = build(&p4(draft()), &lock).unwrap();
    let t = &specs[&TargetKind::Swarmail].template;
    assert!(
        t.windows(2).any(|w| w == ["--max-per-inbox", "150000"]),
        "{t:?}"
    );
    // Protocol 3's Swarmail spec is what it was, null cap and all.
    let (p3, l4) = (
        Protocol::parse(PROTOCOL_V3).unwrap(),
        SourceLock::parse(SOURCES_V4).unwrap(),
    );
    let t = &build(&p3, &l4).unwrap()[&TargetKind::Swarmail].template;
    let cap = p3
        .storage
        .retention
        .iter()
        .find(|r| r.target == TargetKind::Swarmail)
        .and_then(|r| r.max_messages)
        .unwrap_or(0);
    assert_eq!(*t, targets::swarmail_template(cap));
    let mut v = json(PROTOCOL_V3);
    for r in v["storage"]["retention"].as_array_mut().unwrap() {
        if r["target"] == "swarmail" {
            r["max_messages"] = Value::Null;
        }
    }
    let t = &build(&protocol_from(v).unwrap(), &l4).unwrap()[&TargetKind::Swarmail].template;
    assert_eq!(*t, targets::swarmail_template(0), "protocol 3 is unchanged");
}

// ---- T1.5 ------------------------------------------------------------------

#[test]
fn this_driver_refuses_a_frozen_v4_naming_the_owed_sections() {
    let w = refusals(p4(frozen_ready()).validate());
    assert!(
        w.contains(
            "this driver cannot freeze protocol 4: the api, waits, testexec section(s) are owed"
        ),
        "{w}"
    );
    assert_eq!(protocol::OWED_SECTIONS, ["api", "waits", "testexec"]);
    // A draft is not refused for it.
    p4(draft()).validate().unwrap();
}

#[test]
fn with_nothing_owed_a_frozen_v4_still_needs_resolved_clients_and_inspected_capabilities() {
    let check = |v: Value, expect: &str| {
        let w = refusals(p4(v).validate_owing(&[]));
        assert!(w.contains(expect), "{expect}: {w}");
    };
    let mut v = frozen_ready();
    v["unresolved"] = json!(["story B"]);
    check(v, "a frozen protocol has nothing unresolved");
    let mut v = frozen_ready();
    v.as_object_mut().unwrap().remove("clients_sha256");
    check(v, "needs clients_sha256");
    let mut v = frozen_ready();
    v["clients_sha256"] = json!({});
    check(v, "needs clients_sha256");
    let mut v = frozen_ready();
    v["clients_sha256"] = json!({"mailpit": "not-hex"});
    check(v, "needs clients_sha256");
    let mut v = frozen_ready();
    v["capabilities"][1]["native_wait"] = json!({"status": "not_inspected", "cite": null});
    check(
        v,
        "has no not_inspected capability, but mailpit native_wait is",
    );
}

#[test]
fn earlier_protocols_refuse_clients_sha256() {
    let mut v = json(PROTOCOL_V3);
    v["clients_sha256"] = json!({"mailpit": "a".repeat(64)});
    let w = refusals(protocol_from(v).unwrap().validate());
    assert!(
        w.contains("protocol_version 3 refuses clients_sha256"),
        "{w}"
    );
    let mut v = json(PROTOCOL);
    v["clients_sha256"] = json!({});
    assert!(refusals(protocol_from(v).unwrap().validate()).contains("refuses clients_sha256"));
}

// ---- T1.6 ------------------------------------------------------------------

/// A lock value's pin for `target`.
fn pin_of<'a>(l: &'a mut Value, target: &str) -> &'a mut Value {
    let at = l["entries"]
        .as_array()
        .unwrap()
        .iter()
        .position(|e| e["target"] == target)
        .unwrap();
    &mut l["entries"][at]["pin"]
}

fn lock_why(v: Value) -> String {
    refusals(lock_from(v).unwrap().validate())
}

#[test]
fn lock_v5_requires_the_placeholder_at_each_cap_flag() {
    lock_from(lock5())
        .unwrap()
        .validate()
        .expect("the draft lock validates");
    let mut l = lock5();
    assert!(
        pin_of(&mut l, "mailpit")["launch"]
            .as_array()
            .unwrap()
            .contains(&json!("{max_messages}"))
    );
    assert!(
        pin_of(&mut l, "smtp4dev")["launch"]
            .as_array()
            .unwrap()
            .contains(&json!("{max_messages}"))
    );
    assert_eq!(
        pin_of(&mut l, "inbucket")["launch_env"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|kv| kv[1] == "{max_messages}")
            .count(),
        1
    );
    // The `flag=value` form, authored on a copy of the Mailpit pin.
    let mut l = lock5();
    let argv = pin_of(&mut l, "mailpit")["launch"].as_array_mut().unwrap();
    let at = argv.iter().position(|a| a == "--max").unwrap();
    argv.remove(at + 1);
    argv[at] = json!("--max={max_messages}");
    lock_from(l)
        .unwrap()
        .validate()
        .expect("flag=placeholder is the cap flag too");
    // Absent at the flag.
    let mut l = lock5();
    let argv = pin_of(&mut l, "mailpit")["launch"].as_array_mut().unwrap();
    let at = argv.iter().position(|a| a == "--max").unwrap();
    argv.drain(at..=at + 1);
    let w = lock_why(l);
    assert!(
        w.contains("mailpit: lock_version 5 sets --max through {max_messages} exactly once"),
        "{w}"
    );
    // The env form, removed.
    let mut l = lock5();
    let env = pin_of(&mut l, "inbucket")["launch_env"]
        .as_array_mut()
        .unwrap();
    env.retain(|kv| kv[0] != "INBUCKET_STORAGE_MAILBOXMSGCAP");
    let w = lock_why(l);
    assert!(
        w.contains("inbucket: lock_version 5 sets env:INBUCKET_STORAGE_MAILBOXMSGCAP"),
        "{w}"
    );
}

#[test]
fn lock_v5_refuses_a_literal_cap() {
    for (target, form) in [
        ("mailpit", "argv"),
        ("smtp4dev", "argv"),
        ("inbucket", "env"),
    ] {
        let mut l = lock5();
        let pin = pin_of(&mut l, target);
        if form == "argv" {
            let argv = pin["launch"].as_array_mut().unwrap();
            let at = argv.iter().position(|a| a == "{max_messages}").unwrap();
            argv[at] = json!("150000");
        } else {
            for kv in pin["launch_env"].as_array_mut().unwrap() {
                if kv[1] == "{max_messages}" {
                    kv[1] = json!("150000");
                }
            }
        }
        let w = lock_why(l);
        assert!(w.contains("a literal cap is refused"), "{target}: {w}");
    }
}

#[test]
fn lock_v5_refuses_the_placeholder_elsewhere() {
    // In the launch of an uncapped target.
    let mut l = lock5();
    pin_of(&mut l, "mailhog")["launch"]
        .as_array_mut()
        .unwrap()
        .push(json!("{max_messages}"));
    let w = lock_why(l);
    assert!(w.contains("names a slot the declared listeners"), "{w}");
    // Twice in a capped launch.
    let mut l = lock5();
    pin_of(&mut l, "mailpit")["launch"]
        .as_array_mut()
        .unwrap()
        .push(json!("{max_messages}"));
    let w = lock_why(l);
    assert!(w.contains("names a slot"), "{w}");
    // In another environment value.
    let mut l = lock5();
    pin_of(&mut l, "smtp4dev")["launch_env"]
        .as_array_mut()
        .unwrap()
        .push(json!(["OTHER_VALUE", "{max_messages}"]));
    assert!(lock_why(l).contains("names a slot the declared listeners"));
}

#[test]
fn lock_v5_refuses_a_second_cap_flag() {
    let twice = "occurs 2 times in the launch";
    // As an argv pair, the placeholder then a literal.
    let mut l = lock5();
    let argv = pin_of(&mut l, "mailpit")["launch"].as_array_mut().unwrap();
    argv.extend([json!("--max"), json!("500")]);
    let w = lock_why(l);
    assert!(
        w.contains("mailpit: the cap flag --max") && w.contains(twice),
        "{w}"
    );
    // As `flag=`, after the placeholder in either of its forms.
    for first_is_joined in [false, true] {
        let mut l = lock5();
        let argv = pin_of(&mut l, "mailpit")["launch"].as_array_mut().unwrap();
        if first_is_joined {
            let at = argv.iter().position(|a| a == "--max").unwrap();
            argv.remove(at + 1);
            argv[at] = json!("--max={max_messages}");
        }
        argv.push(json!("--max=500"));
        let w = lock_why(l);
        assert!(w.contains(twice), "{first_is_joined}: {w}");
    }
    // As the environment entry the cap flag names.
    let mut l = lock5();
    pin_of(&mut l, "inbucket")["launch_env"]
        .as_array_mut()
        .unwrap()
        .push(json!(["INBUCKET_STORAGE_MAILBOXMSGCAP", "500"]));
    let w = lock_why(l);
    assert!(
        w.contains("inbucket: the cap flag env:INBUCKET_STORAGE_MAILBOXMSGCAP")
            && w.contains(twice),
        "{w}"
    );
    // smtp4dev's flag, twice, with the literal first.
    let mut l = lock5();
    let argv = pin_of(&mut l, "smtp4dev")["launch"].as_array_mut().unwrap();
    argv.splice(0..0, [json!("--messagestokeep"), json!("500")]);
    assert!(lock_why(l).contains(twice));
}

#[test]
fn lock_v4_and_v3_refuse_the_placeholder() {
    let mut l = json(SOURCES_V4);
    let argv = pin_of(&mut l, "mailpit")["launch"].as_array_mut().unwrap();
    let at = argv.iter().position(|a| a == "10000").unwrap();
    argv[at] = json!("{max_messages}");
    let w = lock_why(l);
    assert!(
        w.contains("mailpit: lock_version 4 refuses the {max_messages} placeholder"),
        "{w}"
    );
    let mut l = json(SOURCES);
    pin_of(&mut l, "mailpit")["launch"]
        .as_array_mut()
        .unwrap()
        .push(json!("{max_messages}"));
    let w = lock_why(l);
    assert!(
        w.contains("mailpit: lock_version 3 refuses the {max_messages} placeholder"),
        "{w}"
    );
}

// ---- T1.7 ------------------------------------------------------------------

#[test]
fn the_active_records_are_still_v3_until_the_v4_freeze() {
    assert_eq!(ACTIVE_PROTOCOL, PROTOCOL_V3);
    assert_eq!(ACTIVE_SOURCES, SOURCES_V4);
    assert_eq!(
        Protocol::parse(ACTIVE_PROTOCOL).unwrap().protocol_version,
        3
    );
    assert_eq!(SourceLock::parse(ACTIVE_SOURCES).unwrap().lock_version, 4);
}

// ---- T1.8 ------------------------------------------------------------------

#[test]
fn the_v4_draft_validates_and_is_not_ready_for_collection() {
    let p = p4(draft());
    p.validate().unwrap();
    assert_eq!(p.status, protocol::Status::Draft);
    let l = lock_from(lock5()).unwrap();
    let w = refusals(protocol::collection_ready(&p, &l));
    assert!(w.contains("protocol is a draft, not frozen"), "{w}");
    assert!(w.contains("unresolved: story B"), "{w}");
    // Nothing else is wrong with it: the refusals are the draft's own.
    for line in w.lines() {
        assert!(
            line.contains("draft, not frozen") || line.starts_with("unresolved:"),
            "unexpected refusal: {line}"
        );
    }
}

#[test]
fn the_v4_draft_differs_from_v3_only_where_0132_says() {
    let (d, v3) = (draft(), json(PROTOCOL_V3));
    let (d, v3) = (d.as_object().unwrap(), v3.as_object().unwrap());
    let allowed = [
        "protocol_version",
        "id",
        "status",
        "supersedes",
        "change_reason",
        "throughput",
        "waits",
        "resources",
        "storage",
        "claim",
        "budgets",
        "headline",
        "unresolved",
    ];
    let mut keys: Vec<&String> = d.keys().chain(v3.keys()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        if !allowed.contains(&k.as_str()) {
            assert_eq!(d.get(k), v3.get(k), "{k} must be v3's, unchanged");
        }
    }
    assert!(d.contains_key("headline") && !v3.contains_key("headline"));
    // Resources: v3's fields kept, two changed, six added.
    let (dr, vr) = (
        d["resources"].as_object().unwrap(),
        v3["resources"].as_object().unwrap(),
    );
    for (k, v) in vr {
        if !["enforced", "profile"].contains(&k.as_str()) {
            assert_eq!(dr.get(k), Some(v), "resources.{k}");
        }
    }
    assert_eq!(dr.len(), vr.len() + 6);
    // Storage: the same targets, backends and prune behaviour; only caps move.
    assert_eq!(d["storage"]["mode"], v3["storage"]["mode"]);
    assert_eq!(d["storage"]["profile"], v3["storage"]["profile"]);
    let (da, va) = (
        d["storage"]["retention"].as_array().unwrap(),
        v3["storage"]["retention"].as_array().unwrap(),
    );
    assert_eq!(da.len(), va.len());
    for (a, b) in da.iter().zip(va) {
        for k in ["target", "backend", "prune", "compression"] {
            assert_eq!(a[k], b[k], "{} {k}", b["target"]);
        }
        let want = if b["max_messages"].is_null() {
            Value::Null
        } else {
            json!(150_000)
        };
        assert_eq!(a["max_messages"], want, "{}", b["target"]);
    }
    // Budgets: the adapter's bounds are v3's; only the wait bound moves.
    assert_eq!(d["budgets"]["adapter"], v3["budgets"]["adapter"]);
    assert_eq!(
        d["budgets"]["poll_interval_ms"],
        v3["budgets"]["poll_interval_ms"]
    );
    // The claim keeps v3's text from the target-search sentence on.
    let tail = "Targets without a verified server search";
    let after = |v: &Value| {
        v["claim"]["statement"]
            .as_str()
            .unwrap()
            .split(tail)
            .nth(1)
            .unwrap()
            .to_owned()
    };
    // The one change in it: GreenMail's POP3 read is this draft's, not v4's.
    assert_eq!(
        after(&d["claim"].clone().into_wrapped()).replace(" in this draft", ""),
        after(&v3["claim"].clone().into_wrapped())
    );
}

#[test]
fn the_v4_draft_names_greenmail_rest_and_the_wait_budget_as_unresolved() {
    let d = draft();
    let owed: Vec<&str> = d["unresolved"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    let rest = owed
        .iter()
        .find(|x| x.contains("GreenMail"))
        .expect("an entry for GreenMail's read path");
    assert!(
        rest.contains("REST") && rest.contains("POP3") && rest.contains("story B"),
        "{rest}"
    );
    assert!(rest.contains("answer 8"), "{rest}");
    let wait = owed
        .iter()
        .find(|x| x.contains("wait_operation_ms"))
        .expect("an entry for the wait budget");
    assert!(
        wait.contains("story C")
            && wait.contains("10000")
            && wait.contains("40000")
            && wait.contains("no wait profile"),
        "{wait}"
    );
    // The budget it names is the draft's, and v3's is what it says.
    assert_eq!(d["budgets"]["wait_operation_ms"], 10_000);
    assert_eq!(json(PROTOCOL_V3)["budgets"]["wait_operation_ms"], 40_000);
    // The claim says the POP3 read is the draft's.
    let claim = d["claim"]["statement"].as_str().unwrap();
    assert!(
        claim.contains("GreenMail is read over POP3 in this draft"),
        "{claim}"
    );
    // A draft still validates, and the frozen rule keeps it from freezing.
    p4(d.clone()).validate().unwrap();
    let mut frozen = frozen_ready();
    frozen["unresolved"] = d["unresolved"].clone();
    let w = refusals(p4(frozen).validate_owing(&[]));
    assert!(
        w.contains("a frozen protocol has nothing unresolved"),
        "{w}"
    );
}

trait Wrapped {
    fn into_wrapped(self) -> Value;
}

impl Wrapped for Value {
    fn into_wrapped(self) -> Value {
        json!({ "claim": self })
    }
}

#[test]
fn lock_v5_differs_from_lock_v4_only_in_version_caps_and_three_launches() {
    let (l5, l4) = (lock5(), json(SOURCES_V4));
    assert_eq!(l5["lock_version"], 5);
    assert_eq!(l5["checked_on"], l4["checked_on"]);
    let (e5, e4) = (
        l5["entries"].as_array().unwrap(),
        l4["entries"].as_array().unwrap(),
    );
    assert_eq!(e5.len(), e4.len());
    for (a, b) in e5.iter().zip(e4) {
        let t = b["target"].as_str().unwrap();
        if !["mailpit", "smtp4dev", "inbucket"].contains(&t) {
            assert_eq!(a, b, "{t} is lock 4's, unchanged");
            continue;
        }
        let (mut a, mut b) = (a.clone(), b.clone());
        for e in [&mut a, &mut b] {
            let pin = e["pin"].as_object_mut().unwrap();
            pin.insert("launch".into(), Value::Null);
            pin.insert("launch_env".into(), Value::Null);
            let st = pin["storage"].as_object_mut().unwrap();
            st.remove("max_messages");
            st.remove("basis");
        }
        assert_eq!(a, b, "{t}: only its launch, cap and basis moved");
    }
    let mut l = l5.clone();
    for t in ["mailpit", "smtp4dev", "inbucket"] {
        let pin = pin_of(&mut l, t);
        assert_eq!(pin["storage"]["max_messages"], 150_000, "{t}");
        let text = serde_json::to_string(&pin["launch"]).unwrap()
            + &serde_json::to_string(&pin["launch_env"]).unwrap();
        assert!(!text.contains("\"10000\""), "{t} keeps no literal cap");
        assert!(text.contains("{max_messages}"), "{t}");
    }
}

#[test]
fn lock_v5_verifies_its_documents_and_pairs_with_the_draft() {
    let l = lock_from(lock5()).unwrap();
    l.validate().unwrap();
    l.verify_files(Path::new("docs/evals"))
        .expect("the draft lock verifies against docs/evals");
    let p = p4(draft());
    protocol::pairing(&p, &l).unwrap();
    protocol::retention_matches(&p, &l).expect("the protocol's caps are the lock's stated caps");
    protocol::products_match(&p, &l).unwrap();
    protocol::capabilities_match(&p, &l).unwrap();
    protocol::does_not_start_matches(&p, &l).unwrap();
    protocol::launchable(&p, &l).unwrap();
}

// ============================================================================
// T2: resources.rs on fixture trees
// ============================================================================

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

/// A cgroupfs imitation: a directory of plain files the test authors.
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir().expect("fixture directory")
}

fn put(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).expect("fixture file");
}

/// A scope directory as a delegated systemd scope shows it.
fn scope_fixture(controllers: &str, procs: &str) -> tempfile::TempDir {
    let d = fixture();
    put(d.path(), "cgroup.controllers", controllers);
    put(d.path(), "cgroup.procs", procs);
    put(d.path(), "cgroup.subtree_control", "");
    d
}

/// Every file a real leaf has, with plausible content.
fn leaf_fixture(parent: &Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir(&dir).unwrap();
    for (f, text) in [
        ("memory.max", "max\n"),
        ("memory.swap.max", "max\n"),
        ("memory.oom.group", "0\n"),
        ("cpu.max", "max 100000\n"),
        ("pids.max", "max\n"),
        ("cgroup.kill", ""),
        ("memory.peak", "1048576\n"),
        ("cgroup.procs", ""),
        ("memory.current", "4096\n"),
        ("memory.stat", "anon 1\nfile 2\nkernel 3\n"),
        ("memory.events", "low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\n"),
        ("cpu.stat", CPU_STAT),
        ("pids.current", "1\n"),
    ] {
        put(&dir, f, text);
    }
    dir
}

const CPU_STAT: &str = "usage_usec 8123456\nuser_usec 6000000\nsystem_usec 2123456\nnr_periods 400\nnr_throttled 12\nthrottled_usec 345678\nnr_bursts 0\nburst_usec 0\n";
const MEM_STAT: &str = "anon 104857600\nfile 52428800\nkernel 3145728\nkernel_stack 163840\npagetables 409600\nslab 2000000\nshmem 0\n";
const MEM_EVENTS: &str = "low 0\nhigh 0\nmax 3\noom 1\noom_kill 1\noom_group_kill 1\n";

/// A tree that records every operation and can lie about one read.
struct Spy {
    log: Mutex<Vec<String>>,
    lie: Option<(&'static str, &'static str)>,
}

impl Spy {
    fn new(lie: Option<(&'static str, &'static str)>) -> Arc<Spy> {
        Arc::new(Spy {
            log: Mutex::new(Vec::new()),
            lie,
        })
    }

    fn ops(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }

    fn note(&self, op: &str, p: &Path) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{op} {}", p.file_name().unwrap().to_string_lossy()));
    }
}

impl resources::Tree for Spy {
    fn read(&self, p: &Path) -> std::io::Result<String> {
        self.note("read", p);
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        match self.lie {
            Some((file, text)) if file == name => Ok(text.to_owned()),
            _ => resources::Host.read(p),
        }
    }

    fn write(&self, p: &Path, text: &str) -> std::io::Result<()> {
        self.note(&format!("write[{text}]"), p);
        resources::Host.write(p, text)
    }

    fn exists(&self, p: &Path) -> bool {
        resources::Host.exists(p)
    }

    /// Like the kernel, a new cgroup comes with its `cgroup.procs`.
    fn mkdir(&self, p: &Path) -> std::io::Result<()> {
        self.note("mkdir", p);
        resources::Host.mkdir(p)?;
        std::fs::write(p.join("cgroup.procs"), "")
    }

    fn rmdir(&self, p: &Path) -> std::io::Result<()> {
        self.note("rmdir", p);
        std::fs::remove_dir_all(p)
    }
}

// ---- T2.1 ------------------------------------------------------------------

#[test]
fn detect_refuses_exactly_without_a_cgroup2_root() {
    let scope = scope_fixture("cpu memory pids\n", "");
    let root = scope.path();
    // The cgroup's own path is the root itself: `0::/`.
    let found = resources::detect(&resources::Host, root, "0::/\n", true).expect("delegated");
    assert_eq!(found, root.join(""));
    for (own, is2) in [("0::/\n", false), ("1:name=x:/\n", true), ("", true)] {
        assert_eq!(
            resources::detect(&resources::Host, root, own, is2).unwrap_err(),
            resources::NO_DELEGATION,
            "own {own:?}, cgroup2 {is2}"
        );
    }
    assert_eq!(
        resources::NO_DELEGATION,
        "resources: no delegated cgroup v2 (cpu memory pids)"
    );
    // A cgroup whose controllers file does not exist.
    let gone = root.join("nowhere");
    assert_eq!(
        resources::detect(&resources::Host, &gone, "0::/\n", true).unwrap_err(),
        resources::NO_DELEGATION
    );
}

#[test]
fn detect_refuses_exactly_without_each_controller() {
    for lacking in resources::CONTROLLERS {
        let have: Vec<&str> = resources::CONTROLLERS
            .iter()
            .copied()
            .filter(|c| *c != lacking)
            .chain(["io"])
            .collect();
        let scope = scope_fixture(&have.join(" "), "");
        assert_eq!(
            resources::detect(&resources::Host, scope.path(), "0::/\n", true).unwrap_err(),
            resources::NO_DELEGATION,
            "without {lacking}"
        );
    }
}

#[test]
fn adoption_refuses_exactly_when_the_read_back_lacks_a_controller() {
    let scope = scope_fixture("cpu memory pids\n", "77\n");
    std::fs::write(scope.path().join("cgroup.subtree_control"), "").unwrap();
    let spy = Spy::new(Some(("cgroup.subtree_control", "cpu memory\n")));
    assert_eq!(
        resources::adopt(&*spy, scope.path(), 77).unwrap_err(),
        resources::NO_DELEGATION
    );
    // Told the truth, it adopts: the tokens may carry a leading plus.
    let ok = scope_fixture("cpu memory pids\n", "77\n");
    let spy = Spy::new(Some(("cgroup.subtree_control", "+cpu +memory +pids\n")));
    resources::adopt(&*spy, ok.path(), 77).expect("adopts");
    let spy = Spy::new(None);
    let again = scope_fixture("cpu memory pids\n", "77\n");
    resources::adopt(&*spy, again.path(), 77).expect("adopts on plain files too");
}

#[test]
fn adoption_refuses_an_occupied_scope_and_an_existing_driver_leaf() {
    let busy = scope_fixture("cpu memory pids\n", "77\n78\n79\n");
    let w = resources::adopt(&resources::Host, busy.path(), 77).unwrap_err();
    assert!(w.contains("holds 2 other process(es)"), "{w}");
    assert!(!busy.path().join("driver").exists(), "nothing was made");
    let taken = scope_fixture("cpu memory pids\n", "77\n");
    std::fs::create_dir(taken.path().join("driver")).unwrap();
    let w = resources::adopt(&resources::Host, taken.path(), 77).unwrap_err();
    assert!(w.contains("already holds a driver leaf"), "{w}");
    // A scope whose procs file is not pids.
    let bad = scope_fixture("cpu memory pids\n", "x\n");
    assert!(resources::adopt(&resources::Host, bad.path(), 77).is_err());
}

#[test]
fn adoption_makes_the_driver_leaf_before_it_enables_controllers() {
    let scope = scope_fixture("cpu memory pids\n", "4242\n");
    let spy = Spy::new(None);
    resources::adopt(&*spy, scope.path(), 4242).expect("adopts");
    assert_eq!(
        spy.ops(),
        [
            "read cgroup.procs",
            "mkdir driver",
            "write[4242] cgroup.procs",
            "write[+cpu +memory +pids] cgroup.subtree_control",
            "read cgroup.subtree_control",
        ]
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn off_linux_detection_is_the_exact_refusal() {
    let scope = scope_fixture("cpu memory pids\n", "");
    assert_eq!(
        resources::detect(&resources::Host, scope.path(), "0::/\n", true).unwrap_err(),
        resources::NO_DELEGATION
    );
    assert_eq!(
        resources::detect_host().unwrap_err(),
        resources::NO_DELEGATION
    );
    assert_eq!(
        resources::prepare_host(Path::new(".")).err().unwrap(),
        resources::NO_DELEGATION
    );
}

// ---- T2.2 ------------------------------------------------------------------

#[test]
fn a_leaf_records_the_five_limits_as_read_back() {
    let scope = fixture();
    let dir = leaf_fixture(scope.path(), "target-0001");
    let (leaf, limits) = resources::Leaf::prepare(resources::host(), dir.clone(), "target-0001")
        .expect("a complete leaf prepares");
    assert_eq!(
        limits,
        resources::LimitsRead {
            memory_max: "2147483648".into(),
            memory_swap_max: "0".into(),
            memory_oom_group: "1".into(),
            cpu_max: "200000 100000".into(),
            pids_max: "4096".into(),
        }
    );
    // What was written is what the files hold now.
    for (file, want) in [
        ("memory.max", "2147483648"),
        ("memory.swap.max", "0"),
        ("memory.oom.group", "1"),
        ("cpu.max", "200000 100000"),
        ("pids.max", "4096"),
    ] {
        assert_eq!(std::fs::read_to_string(dir.join(file)).unwrap(), want);
    }
    assert_eq!(leaf.name(), "target-0001");
    // The values are the decision's, not the code's own constants restated.
    assert_eq!(2048u64 << 20, 2_147_483_648);
    assert_eq!(limits.cpu_max, format!("{} {}", 2 * 100_000, 100_000));
}

#[test]
fn a_differing_read_back_refuses_naming_the_file() {
    for (file, lie) in [
        ("memory.max", "1073741824\n"),
        ("memory.swap.max", "max\n"),
        ("memory.oom.group", "0\n"),
        ("cpu.max", "max 100000\n"),
        ("pids.max", "max\n"),
    ] {
        let scope = fixture();
        let dir = leaf_fixture(scope.path(), "target-0001");
        let spy: Arc<dyn resources::Tree> = Spy::new(Some((file, lie)));
        let w = match resources::Leaf::prepare(spy, dir, "target-0001") {
            Err(e) => e,
            Ok(_) => panic!("{file}: a differing read-back must refuse"),
        };
        assert!(w.contains(&format!("{file} reads back")), "{w}");
    }
}

#[test]
fn a_leaf_without_each_required_file_refuses_naming_it() {
    for file in [
        "memory.max",
        "memory.swap.max",
        "memory.oom.group",
        "cpu.max",
        "pids.max",
        "cgroup.kill",
        "memory.peak",
    ] {
        let scope = fixture();
        let dir = leaf_fixture(scope.path(), "target-0001");
        std::fs::remove_file(dir.join(file)).unwrap();
        let w = match resources::Leaf::prepare(resources::host(), dir, "target-0001") {
            Err(e) => e,
            Ok(_) => panic!("{file}: a missing file must refuse"),
        };
        assert!(w.contains(&format!("lacks {file}")), "{w}");
    }
    // `create` makes the directory and, on a refusal, removes it again: a
    // plain directory has none of the files a cgroup brings.
    let scope = fixture();
    let w = match resources::Leaf::create(resources::host(), scope.path(), "target-0009") {
        Err(e) => e,
        Ok(_) => panic!("a bare directory is no leaf"),
    };
    assert!(w.contains("lacks"), "{w}");
    assert!(!scope.path().join("target-0009").exists());
}

// ---- T2.3 ------------------------------------------------------------------

#[test]
fn cgroup_files_parse_from_captured_text() {
    let c = resources::CpuStat::parse(CPU_STAT).unwrap();
    assert_eq!(
        c,
        resources::CpuStat {
            usage_usec: 8_123_456,
            user_usec: 6_000_000,
            system_usec: 2_123_456,
            nr_throttled: 12,
            throttled_usec: 345_678,
        }
    );
    let m = resources::MemStat::parse(MEM_STAT).unwrap();
    assert_eq!(
        (m.anon, m.file, m.kernel),
        (104_857_600, 52_428_800, 3_145_728)
    );
    // Kernels before 6.5 have no `kernel` key: its components are summed.
    let old = resources::MemStat::parse("anon 10\nfile 20\nkernel_stack 1\npagetables 2\nslab 4\n")
        .unwrap();
    assert_eq!(old.kernel, 7);
    let e = resources::MemEvents::parse(MEM_EVENTS).unwrap();
    assert_eq!((e.max, e.oom, e.oom_kill), (3, 1, 1));
    assert_eq!(
        resources::parse_single("memory.current", "123456789\n"),
        Ok(123_456_789)
    );
    assert_eq!(resources::parse_single("pids.current", " 7 "), Ok(7));
    assert_eq!(resources::parse_procs("123\n456\n"), Ok(vec![123, 456]));
    assert_eq!(resources::parse_procs(""), Ok(vec![]));
}

#[test]
fn malformed_cgroup_text_refuses() {
    for bad in [
        "",
        "usage_usec",
        "usage_usec x\n",
        "a 1 2\n",
        "a 1\na 2\n",
        "a -1\n",
    ] {
        assert!(resources::parse_flat("f", bad).is_err(), "{bad:?}");
    }
    // A well-formed file that lacks a required key.
    assert!(
        resources::CpuStat::parse("usage_usec 1\n")
            .unwrap_err()
            .contains("lacks user_usec")
    );
    assert!(
        resources::MemEvents::parse("max 1\noom 0\n")
            .unwrap_err()
            .contains("lacks oom_kill")
    );
    assert!(
        resources::MemStat::parse("anon 1\n")
            .unwrap_err()
            .contains("lacks file")
    );
    for bad in ["max\n", "", "abc", "-1", "1.5"] {
        assert!(
            resources::parse_single("memory.peak", bad).is_err(),
            "{bad:?}"
        );
    }
    for bad in ["abc\n", "0\n", "-5\n", "1 2\n"] {
        assert!(resources::parse_procs(bad).is_err(), "{bad:?}");
    }
    for bad in ["", "1:name=x:/a\n", "no colon here"] {
        assert!(resources::parse_proc_cgroup(bad).is_err(), "{bad:?}");
    }
    for bad in ["", "3-1", "a", "1-b", "0-99999999", ","] {
        assert!(resources::parse_cpu_list(bad).is_err(), "{bad:?}");
    }
    assert!(resources::parse_cpus_allowed("Name:\tx\n").is_err());
}

#[test]
fn proc_cgroup_and_cpus_allowed_parse() {
    assert_eq!(
        resources::parse_proc_cgroup(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/run-r1.scope/target-0003\n"
        ),
        Ok(
            "/user.slice/user-1000.slice/user@1000.service/app.slice/run-r1.scope/target-0003"
                .into()
        )
    );
    let set = |v: &[u32]| v.iter().copied().collect::<BTreeSet<u32>>();
    assert_eq!(resources::parse_cpu_list("0-1\n"), Ok(set(&[0, 1])));
    assert_eq!(resources::parse_cpu_list("0,2"), Ok(set(&[0, 2])));
    assert_eq!(
        resources::parse_cpu_list("0-3,8,10-11"),
        Ok(set(&[0, 1, 2, 3, 8, 10, 11]))
    );
    let status =
        "Name:\tmailpit\nCpus_allowed:\t3\nCpus_allowed_list:\t0-1\nMems_allowed_list:\t0\n";
    assert_eq!(resources::parse_cpus_allowed(status), Ok(set(&[0, 1])));
    assert_eq!(resources::format_cpu_list(&[0, 1, 4, 6, 7, 8]), "0-1,4,6-8");
    assert_eq!(resources::format_cpu_list(&[]), "");
    // The join read-back compares the leaf and exactly the two target CPUs.
    let joined = resources::Joined {
        cgroup: "target-0003".into(),
        cpus_allowed: vec![0, 1],
    };
    resources::check_join(&joined, "target-0003", &[0, 1]).unwrap();
    let w = resources::check_join(&joined, "target-0004", &[0, 1]).unwrap_err();
    assert!(w.contains("not its leaf"), "{w}");
    let w = resources::check_join(&joined, "target-0003", &[0, 2]).unwrap_err();
    assert!(w.contains("not exactly 0,2"), "{w}");
    let wide = resources::Joined {
        cgroup: "target-0003".into(),
        cpus_allowed: vec![0, 1, 2],
    };
    assert!(resources::check_join(&wide, "target-0003", &[0, 1]).is_err());
}

#[test]
fn a_join_read_back_mismatch_refuses_the_launch() {
    // The comparison the launcher runs after the spawn, on a fixture /proc.
    let proc_root = fixture();
    let pid_dir = proc_root.path().join("4321");
    std::fs::create_dir(&pid_dir).unwrap();
    put(&pid_dir, "cgroup", "0::/a/b/scope/target-0002\n");
    put(&pid_dir, "status", "Name:\tx\nCpus_allowed_list:\t4,6\n");
    let j = resources::read_join_whole(proc_root.path(), 4321)
        .unwrap()
        .joined;
    assert_eq!(j.cgroup, "target-0002");
    assert_eq!(j.cpus_allowed, vec![4, 6]);
    resources::check_join(&j, "target-0002", &[4, 6]).unwrap();
    assert!(resources::check_join(&j, "target-0001", &[4, 6]).is_err());
    assert!(resources::read_join_whole(proc_root.path(), 1).is_err());
}

// ---- T2.4 ------------------------------------------------------------------

fn smt(c: u32) -> BTreeSet<u32> {
    BTreeSet::from([c % 4, c % 4 + 4])
}

fn alone(c: u32) -> BTreeSet<u32> {
    BTreeSet::from([c])
}

fn cpus(v: &[u32]) -> BTreeSet<u32> {
    v.iter().copied().collect()
}

#[test]
fn layout_takes_two_distinct_cores_and_idles_their_siblings() {
    // Eight CPUs, four cores: cpu n and n+4 are siblings.
    let l = resources::layout(&cpus(&[0, 1, 2, 3, 4, 5, 6, 7]), &smt).unwrap();
    assert_eq!(l.target_cpus, vec![0, 1]);
    assert_eq!(l.idle_siblings, vec![4, 5]);
    assert_eq!(l.client_cpus, vec![2, 3, 6, 7]);
    assert!(l.siblings_outside_set.is_empty());
    // A target is never its own sibling, and the two never share a core.
    assert!(!smt(l.target_cpus[0]).contains(&l.target_cpus[1]));
    // No SMT: nothing to idle.
    let l = resources::layout(&cpus(&[0, 1, 2, 3]), &alone).unwrap();
    assert_eq!(l.target_cpus, vec![0, 1]);
    assert!(l.idle_siblings.is_empty());
    assert_eq!(l.client_cpus, vec![2, 3]);
    // The lowest allowed CPU, not CPU 0: a set that starts elsewhere.
    let l = resources::layout(&cpus(&[8, 9, 10, 11]), &alone).unwrap();
    assert_eq!(l.target_cpus, vec![8, 9]);
    // Siblings that are adjacent numbers (0,1 one core): the second target
    // is the next CPU that is not a sibling of the first.
    let pairs = |c: u32| BTreeSet::from([c & !1, c | 1]);
    let l = resources::layout(&cpus(&[0, 1, 2, 3, 4, 5]), &pairs).unwrap();
    assert_eq!(l.target_cpus, vec![0, 2]);
    assert_eq!(l.idle_siblings, vec![1, 3]);
    assert_eq!(l.client_cpus, vec![4, 5]);
}

#[test]
fn layout_refuses_one_core() {
    let w = resources::layout(&cpus(&[0]), &alone).unwrap_err();
    assert!(w.contains("fewer than two distinct cores"), "{w}");
    // Two CPUs of one core are one core.
    let w = resources::layout(&cpus(&[0, 4]), &smt).unwrap_err();
    assert!(w.contains("fewer than two distinct cores"), "{w}");
    assert!(resources::layout(&cpus(&[]), &alone).is_err());
}

#[test]
fn layout_refuses_when_no_client_cpu_is_left() {
    let w = resources::layout(&cpus(&[0, 1]), &alone).unwrap_err();
    assert!(w.contains("no client CPU is left"), "{w}");
    // Siblings of the targets are idle, so they are not clients either.
    let w = resources::layout(&cpus(&[0, 1, 4, 5]), &smt).unwrap_err();
    assert!(w.contains("no client CPU is left"), "{w}");
}

#[test]
fn layout_records_siblings_outside_the_set() {
    let l = resources::layout(&cpus(&[0, 1, 2, 3]), &smt).unwrap();
    assert_eq!(l.target_cpus, vec![0, 1]);
    assert!(l.idle_siblings.is_empty());
    assert_eq!(l.siblings_outside_set, vec![4, 5]);
    assert_eq!(l.client_cpus, vec![2, 3]);
    // The sibling list is read from the kernel's file, and an unreadable
    // CPU is its own only sibling.
    let sys = fixture();
    let topo = sys.path().join("devices/system/cpu/cpu3/topology");
    std::fs::create_dir_all(&topo).unwrap();
    put(&topo, "thread_siblings_list", "3,67\n");
    assert_eq!(resources::read_siblings(sys.path(), 3), cpus(&[3, 67]));
    assert_eq!(resources::read_siblings(sys.path(), 9), cpus(&[9]));
}

// ---- T2.5 ------------------------------------------------------------------

#[test]
fn tmpfs_magic_refuses_and_disk_magic_passes() {
    for tmpfs in [0x0102_1994u64, 0x8584_58f6] {
        let w = resources::scratch_backing(tmpfs).unwrap_err();
        assert!(w.contains("scratch directory is on tmpfs"), "{w}");
    }
    for disk in [
        0xEF53u64,
        0x5846_5342,
        0x9123_683E,
        0x6969,
        resources::CGROUP2_SUPER_MAGIC,
    ] {
        resources::scratch_backing(disk).expect("not a RAM filesystem");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_tmpfs_scratch_refuses() {
    let w = resources::check_scratch(Path::new("/dev/shm")).unwrap_err();
    assert!(w.contains("tmpfs"), "{w}");
    assert!(resources::check_scratch(Path::new("/no/such/dir")).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn a_target_dir_scratch_passes() {
    resources::check_scratch(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("the target directory is disk backed");
}

#[test]
fn allocated_bytes_sums_blocks_of_a_tree() {
    use std::os::unix::fs::MetadataExt;
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("alloc-{}", std::process::id()));
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("big"), vec![7u8; 300_000]).unwrap();
    std::fs::write(base.join("sub/small"), b"x").unwrap();
    std::os::unix::fs::symlink("/etc", base.join("link")).unwrap();
    let blocks = |p: &Path| std::fs::symlink_metadata(p).unwrap().blocks() * 512;
    let want = blocks(&base)
        + blocks(&base.join("big"))
        + blocks(&base.join("sub"))
        + blocks(&base.join("sub/small"))
        + blocks(&base.join("link"));
    let got = resources::allocated_bytes(&base).unwrap();
    assert_eq!(got, want, "a symlink counts as itself, not its target");
    assert!(got >= 300_000, "the 300 kB file is allocated: {got}");
    assert!(resources::allocated_bytes(&base.join("missing")).is_err());
    std::fs::remove_dir_all(&base).unwrap();
}

/// The old boolean reading: a task that cannot be read is not a widening.
fn held(root: &Path, pids: &[i32], cpus: &[u32]) -> bool {
    resources::affinity_read(root, pids, cpus).unwrap_or(true)
}

#[test]
fn affinity_held_reads_every_task_of_every_process() {
    let proc_root = fixture();
    let mk = |pid: u32, tid: u32, list: &str| {
        let d = proc_root.path().join(format!("{pid}/task/{tid}"));
        std::fs::create_dir_all(&d).unwrap();
        put(
            &d,
            "status",
            &format!("Name:\tx\nCpus_allowed_list:\t{list}\n"),
        );
    };
    mk(10, 10, "0-1");
    mk(10, 11, "0-1");
    mk(11, 11, "0-1");
    assert!(held(proc_root.path(), &[10, 11], &[0, 1]));
    // One widened thread makes it false.
    mk(10, 12, "0-3");
    assert!(!held(proc_root.path(), &[10, 11], &[0, 1]));
    // A process that vanished is skipped; a status that cannot be parsed is not.
    assert!(held(proc_root.path(), &[11, 99], &[0, 1]));
    put(&proc_root.path().join("11/task/11"), "status", "garbage\n");
    assert!(!held(proc_root.path(), &[11], &[0, 1]));
}

// ============================================================================
// T3: the launcher join, teardown and receipts
// ============================================================================

fn draft_specs() -> std::collections::BTreeMap<TargetKind, targets::LaunchSpec> {
    let (p, l) = (p4(draft()), lock_from(lock5()).unwrap());
    let bins: Vec<String> = p
        .launch_order(0)
        .map(|t| format!("{t}=/never/launched/{t}"))
        .collect();
    let runtimes = vec![
        "greenmail=/never/launched/jre".to_owned(),
        "alkemio-mailslurper=/never/launched/musl".to_owned(),
    ];
    runner::launch_specs(&p, &l, &bins, &runtimes).expect("the draft builds its specs")
}

// ---- T3.1 ------------------------------------------------------------------

#[test]
fn the_v4_spec_carries_150000_at_each_cap_flag() {
    let specs = draft_specs();
    let pair = |t: TargetKind, flag: &str| {
        let a = &specs[&t].template;
        let at = a
            .iter()
            .position(|x| x == flag)
            .unwrap_or_else(|| panic!("{t}: no {flag}"));
        a[at + 1].clone()
    };
    assert_eq!(pair(TargetKind::Mailpit, "--max"), "150000");
    assert_eq!(pair(TargetKind::Smtp4dev, "--messagestokeep"), "150000");
    assert_eq!(pair(TargetKind::Swarmail, "--max-per-inbox"), "150000");
    let inbucket = &specs[&TargetKind::Inbucket].extra.env;
    assert!(inbucket.contains(&["INBUCKET_STORAGE_MAILBOXMSGCAP".into(), "150000".into()]));
    // No placeholder survives anywhere, and an uncapped target has none.
    for (t, spec) in &specs {
        let all = spec.template.iter().chain(spec.extra.env.iter().flatten());
        for text in all {
            assert!(!text.contains("{max_messages}"), "{t}: {text}");
        }
    }
    assert!(
        !specs[&TargetKind::Mailhog]
            .template
            .iter()
            .any(|a| a == "150000")
    );
}

#[tokio::test]
async fn an_unresolved_placeholder_refuses_before_spawn() {
    let lock = lock_from(lock5()).unwrap();
    let pin = lock.pin(TargetKind::Mailpit).unwrap();
    let spec = targets::LaunchSpec::from_pin(
        TargetKind::Mailpit,
        PathBuf::from("/bin/true"),
        None,
        pin,
        None,
    )
    .unwrap();
    assert!(spec.template.iter().any(|a| a == "{max_messages}"));
    let root = tempfile::tempdir().unwrap();
    let leaf = root.path().join("leaf");
    let r = targets::Owned::launch(
        &spec,
        &leaf,
        &limits(),
        &wire::Clock(Instant::now()),
        &targets::Keepers::default(),
    )
    .await;
    assert_eq!(r.err(), Some(targets::LaunchError::Template));
    assert!(!leaf.exists(), "no scratch, so no process, was made");
}

#[test]
fn a_lock_v4_spec_is_unchanged_by_the_cap_argument() {
    let lock = SourceLock::parse(SOURCES_V4).unwrap();
    for t in [
        TargetKind::Mailpit,
        TargetKind::Smtp4dev,
        TargetKind::Inbucket,
        TargetKind::Mailhog,
    ] {
        let pin = lock.pin(t).unwrap();
        let make =
            |cap| targets::LaunchSpec::from_pin(t, PathBuf::from("/x"), None, pin, cap).unwrap();
        let (none, some) = (make(None), make(Some(777)));
        assert_eq!(none.template, some.template, "{t}");
        assert_eq!(none.extra.env, some.extra.env, "{t}");
        assert_eq!(none.template, pin.launch, "{t}: the lock's own launch");
    }
    let pin = lock.pin(TargetKind::Mailpit).unwrap();
    assert!(
        pin.launch.contains(&"10000".to_owned()),
        "lock 4 keeps its literal cap"
    );
}

// ---- T3.2 ------------------------------------------------------------------

#[tokio::test]
async fn a_launch_without_a_leaf_never_builds_a_resources_context() {
    let spec = swarmail_spec(targets::swarmail_template(0));
    assert!(
        spec.extra.scope.is_none(),
        "protocols 2 and 3 carry no scope"
    );
    let root = tempfile::tempdir().unwrap();
    let keepers = targets::Keepers::default();
    let mut owned = targets::Owned::launch(
        &spec,
        &root.path().join("leaf"),
        &limits(),
        &wire::Clock(Instant::now()),
        &keepers,
    )
    .await
    .expect("a real Swarmail launches");
    assert!(owned.leaf.is_none());
    let f = &owned.facts;
    assert!(f.leaf.is_none() && f.limits.is_none() && f.cpu_layout.is_none());
    assert!(f.joined.is_none() && f.env_template.is_none());
    let cleanup = owned.shutdown().await;
    assert!(cleanup.proved(), "{cleanup:?}");
    assert_eq!(cleanup.cgroup_removed, None);
    assert_eq!(cleanup.memory_peak_bytes, None);
    let text = serde_json::to_string(&cleanup).unwrap();
    assert!(
        !text.contains("cgroup_removed") && !text.contains("memory_peak"),
        "{text}"
    );
}

// ---- T3.3 ------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn a_leaf_that_never_empties_ends_at_the_budget_unproved_and_in_place() {
    let scope = fixture();
    let dir = leaf_fixture(scope.path(), "target-0001");
    put(&dir, "cgroup.procs", "4242\n");
    let (leaf, _) =
        resources::Leaf::prepare(resources::host(), dir.clone(), "target-0001").unwrap();
    let start = tokio::time::Instant::now();
    let t = leaf.teardown(start + Duration::from_millis(200)).await;
    assert!(t.killed, "the kill is always written");
    assert!(!t.removed, "a leaf that is not empty is never removed");
    assert_eq!(t.memory_peak_bytes, Some(1_048_576));
    assert!(dir.exists(), "the leaf stays in place for the operator");
    assert_eq!(
        std::fs::read_to_string(dir.join("cgroup.kill")).unwrap(),
        "1"
    );
    assert!(
        start.elapsed() >= Duration::from_millis(200),
        "it waited the budget out"
    );
    assert!(start.elapsed() < Duration::from_secs(5), "and no longer");
}

#[tokio::test(start_paused = true)]
async fn the_kill_is_written_and_procs_read_once_even_past_the_budget() {
    let scope = fixture();
    let dir = leaf_fixture(scope.path(), "target-0001");
    put(&dir, "cgroup.procs", "4242\n");
    let spy = Spy::new(None);
    let (leaf, _) = resources::Leaf::prepare(spy.clone(), dir.clone(), "target-0001").unwrap();
    let before = spy.ops().len();
    // A budget that ended before teardown began.
    let spent = tokio::time::Instant::now();
    tokio::time::advance(Duration::from_secs(5)).await;
    let t = leaf.teardown(spent).await;
    assert_eq!(
        spy.ops()[before..],
        [
            "write[1] cgroup.kill",
            "read cgroup.procs",
            "read memory.peak",
            "read memory.events"
        ],
        "one kill, one read of the procs, then the two receipt readings"
    );
    assert!(t.killed && !t.removed);
    // The same leaf once the kernel has emptied it: one read, then removal.
    put(&dir, "cgroup.procs", "");
    let before = spy.ops().len();
    let t = leaf.teardown(spent).await;
    assert!(t.removed);
    assert_eq!(
        spy.ops()[before..],
        [
            "write[1] cgroup.kill",
            "read cgroup.procs",
            "read memory.peak",
            "read memory.events",
            "rmdir target-0001"
        ]
    );
}

#[test]
fn a_leaf_launch_without_cgroup_removed_is_not_proved() {
    let mut f = targets::CleanupFacts::unproved(TargetKind::Mailpit);
    f.reaped = true;
    f.group_gone = true;
    f.listeners_closed = true;
    f.scratch_removed = true;
    assert!(
        f.proved(),
        "a launch without a leaf is proved as it always was"
    );
    f.leaf = Some("target-0001".into());
    assert!(
        !f.proved(),
        "a leaf launch with no cgroup_removed is not proved"
    );
    f.cgroup_removed = Some(false);
    assert!(!f.proved());
    f.cgroup_removed = Some(true);
    assert!(f.proved());
    f.reaped = false;
    assert!(!f.proved(), "the older proofs still all hold");
}

// ---- T3.4 ------------------------------------------------------------------

#[test]
fn v3_launch_and_cleanup_receipts_serialize_byte_for_byte() {
    // Literal JSON as the serializers wrote it before protocol 4 existed.
    let launch = targets::LaunchFacts {
        target: TargetKind::Mailpit,
        executable_sha256: "ab".repeat(32),
        argv_template: vec!["--smtp".into(), "{smtp_addr}".into()],
        ready_ns: 42,
        members: vec![("member".into(), "cd".repeat(32))],
        leaf: None,
        limits: None,
        cpu_layout: None,
        joined: None,
        env_template: None,
    };
    assert_eq!(
        serde_json::to_string(&launch).unwrap(),
        format!(
            "{{\"target\":\"mailpit\",\"executable_sha256\":\"{}\",\"argv_template\":[\"--smtp\",\"{{smtp_addr}}\"],\"ready_ns\":42,\"members\":[[\"member\",\"{}\"]]}}",
            "ab".repeat(32),
            "cd".repeat(32)
        )
    );
    let mut cleanup = targets::CleanupFacts::unproved(TargetKind::Greenmail);
    cleanup.forced_kill = true;
    cleanup.reaped = true;
    cleanup.group_gone = true;
    cleanup.listeners_closed = true;
    cleanup.scratch_removed = true;
    cleanup.listeners = vec![targets::ListenerFact {
        role: protocol::ListenerRole::Smtp,
        closed: true,
    }];
    assert_eq!(
        serde_json::to_string(&cleanup).unwrap(),
        "{\"target\":\"greenmail\",\"forced_kill\":true,\"reaped\":true,\"group_gone\":true,\"listeners_closed\":true,\"listeners\":[{\"role\":\"smtp\",\"closed\":true}],\"scratch_removed\":true}"
    );
}

#[tokio::test]
async fn a_v4_launch_receipt_shows_the_cap_and_keeps_ports_and_paths_as_slots() {
    // The receipt's template is the spec's, with the cap resolved when the
    // spec was built; ports and private paths stay slots (0093's privacy rule).
    let spec = swarmail_spec(targets::swarmail_template(protocol::V4_CAP));
    let root = tempfile::tempdir().unwrap();
    let mut owned = targets::Owned::launch(
        &spec,
        &root.path().join("leaf"),
        &limits(),
        &wire::Clock(Instant::now()),
        &targets::Keepers::default(),
    )
    .await
    .unwrap();
    let t = owned.facts.argv_template.clone();
    assert!(owned.shutdown().await.proved());
    assert_eq!(t, targets::swarmail_template(150_000));
    assert!(t.windows(2).any(|w| w == ["--max-per-inbox", "150000"]));
    assert!(t.contains(&"{smtp_addr}".to_owned()) && t.contains(&"{http_addr}".to_owned()));
    let text = t.join(" ");
    assert!(!text.contains(root.path().to_str().unwrap()), "{text}");
    // And a competitor's, from the draft lock.
    let specs = draft_specs();
    let m = specs[&TargetKind::Mailpit].template.join(" ");
    assert!(
        m.contains("--max 150000") && m.contains("{data_dir}/mailpit.db"),
        "{m}"
    );
}

// ============================================================================
// T4: rows, the cost chunk and the figures
// ============================================================================

/// A protocol 4 draft cut down to one small cost profile, for hand-built rows.
fn tiny4() -> Protocol {
    let mut v = draft();
    let mut prof = v["throughput"][1].clone();
    prof["messages"] = json!(4);
    prof["warmup_messages"] = json!(0);
    prof["sessions"] = json!(2);
    v["throughput"] = json!([prof]);
    p4(v)
}

fn tiny_manifest(p: &Protocol, run: &str, chunk: &runner::Chunk) -> Manifest {
    let (planned, windows) = runner::plan_chunk(p, chunk);
    let mut m = manifest(p, run);
    m.planned = planned;
    m.chunk = Some(chunk.name());
    m.planned_windows = Some(windows);
    m
}

fn mailpit_chunk(rep: u32) -> runner::Chunk {
    runner::Chunk::Cost {
        target: TargetKind::Mailpit,
        repetition: rep,
    }
}

fn zero_snap(t_ns: u64) -> resources::Snap {
    resources::Snap {
        t_ns,
        cpu: Default::default(),
        events: Default::default(),
    }
}

fn win(
    run: &str,
    launch: &str,
    (profile, rep): (&str, u32),
    kind: records::WindowKind,
    (start_ns, end_ns): (u64, u64),
    outcome: records::WindowOutcome,
) -> records::WindowRow {
    runner::window_row(
        run,
        launch,
        (TargetKind::Mailpit, profile, rep),
        kind,
        (&zero_snap(start_ns), &zero_snap(end_ns)),
        Some(true),
        None,
        outcome,
    )
}

fn sample_at(run: &str, launch: &str, t_ns: u64, memory: u64, anon: u64) -> records::SampleRow {
    records::SampleRow {
        row_version: records::RESOURCE_ROW_VERSION,
        run_id: run.into(),
        launch: launch.into(),
        target: TargetKind::Mailpit,
        t_ns,
        memory_current: memory,
        anon,
        file: memory - anon.min(memory),
        kernel: 0,
        pids: 3,
    }
}

fn write_samples(dir: &Path, rows: &[String]) {
    let mut text = rows.join("\n");
    text.push('\n');
    std::fs::write(dir.join("samples.jsonl"), text).unwrap();
}

fn all_windows(run: &str, rep: u32) -> Vec<records::WindowRow> {
    use records::WindowKind::*;
    let o = records::WindowOutcome::Measured;
    [(Ready, 0, 1), (IdleReady, 2, 7), (Measure, 8, 20)]
        .into_iter()
        .map(|(k, a, b)| {
            let at = (a * 1_000_000_000, b * 1_000_000_000);
            win(run, "target-0001", ("cost-c8-reused", rep), k, at, o)
        })
        .collect()
}

fn message_lines(run: &str, rep: u32) -> Vec<String> {
    let mut lines = Vec::new();
    for (phase, n, outcome) in [
        (Phase::Send, 4, "accepted"),
        (Phase::Visible, 4, "visible"),
        (Phase::Verified, 1, "verified"),
    ] {
        for i in 0..n {
            let mut r = runner::row(
                run,
                TargetKind::Mailpit,
                "cost-c8-reused",
                rep,
                phase,
                i,
                outcome,
            );
            r.token = Some(format!("t{i}"));
            lines.push(serde_json::to_string(&r).unwrap());
        }
    }
    lines
}

fn window_line(w: &records::WindowRow) -> String {
    serde_json::to_string(&records::ResourceRow::Window(w.clone())).unwrap()
}

// ---- T4.1 ------------------------------------------------------------------

#[test]
fn a_v4_run_directory_reads_both_row_families() {
    let p = tiny4();
    let m = tiny_manifest(&p, "run-a", &mailpit_chunk(0));
    let dir = fixture();
    let mut lines = message_lines("run-a", 0);
    let sample = sample_at("run-a", "target-0001", 5, 10, 4);
    lines.push(serde_json::to_string(&records::ResourceRow::Sample(sample)).unwrap());
    lines.extend(all_windows("run-a", 0).iter().map(window_line));
    write_samples(dir.path(), &lines);
    let (rows, res) = records::read_run(dir.path(), &m).expect("both families read");
    assert_eq!((rows.len(), res.len()), (9, 4));
    assert!(lines[9].contains("\"kind\":\"sample\""), "{}", lines[9]);
    assert!(lines[10].contains("\"kind\":\"window\""));
    // `read_rows` is the message rows alone.
    assert_eq!(records::read_rows(dir.path(), &m).unwrap().len(), 9);
    // A resource row of another run refuses.
    let mut foreign = lines.clone();
    let other = sample_at("other", "t", 5, 10, 4);
    foreign[9] = serde_json::to_string(&records::ResourceRow::Sample(other)).unwrap();
    write_samples(dir.path(), &foreign);
    assert!(
        records::read_run(dir.path(), &m)
            .unwrap_err()
            .contains("another run")
    );
    // An unknown key in a resource row refuses (the type is closed).
    let mut extra = lines.clone();
    extra[9] = extra[9].replace("\"pids\":3", "\"pids\":3,\"secret\":1");
    write_samples(dir.path(), &extra);
    assert!(records::read_run(dir.path(), &m).is_err());
}

#[test]
fn a_resource_row_in_a_v3_run_directory_refuses() {
    let p = smoke();
    let m = manifest(&p, "run-b");
    assert!(m.planned_windows.is_none());
    let dir = fixture();
    let sample = records::ResourceRow::Sample(sample_at("run-b", "target-0001", 5, 10, 4));
    write_samples(dir.path(), &[serde_json::to_string(&sample).unwrap()]);
    let w = records::read_rows(dir.path(), &m).unwrap_err();
    assert!(w.contains("row 0"), "{w}");
    assert!(records::read_run(dir.path(), &m).is_err());
}

#[test]
fn a_duplicate_an_unplanned_and_a_missing_window_each_refuse() {
    let p = tiny4();
    let m = tiny_manifest(&p, "run-c", &mailpit_chunk(0));
    let dir = fixture();
    let ws = all_windows("run-c", 0);
    // The complete set reads.
    let mut ok = message_lines("run-c", 0);
    ok.extend(ws.iter().map(window_line));
    write_samples(dir.path(), &ok);
    records::read_run(dir.path(), &m).expect("a complete chunk reads");
    // A duplicate.
    let mut dup = ok.clone();
    dup.push(window_line(&ws[2]));
    write_samples(dir.path(), &dup);
    let w = records::read_run(dir.path(), &m).unwrap_err();
    assert!(
        w.contains("Measure") && w.contains("appears 2 times"),
        "{w}"
    );
    // An unplanned one: another repetition, and another profile.
    for (profile, rep) in [("cost-c8-reused", 1), ("cost-c1-reused", 0)] {
        let mut extra = ok.clone();
        let kind = records::WindowKind::Measure;
        let o = records::WindowOutcome::Measured;
        extra.push(window_line(&win(
            "run-c",
            "target-0001",
            (profile, rep),
            kind,
            (0, 1),
            o,
        )));
        write_samples(dir.path(), &extra);
        let w = records::read_run(dir.path(), &m).unwrap_err();
        assert!(w.contains("unplanned window"), "{w}");
    }
    // A missing one.
    let mut gone = message_lines("run-c", 0);
    gone.extend(ws[..2].iter().map(window_line));
    write_samples(dir.path(), &gone);
    let w = records::read_run(dir.path(), &m).unwrap_err();
    assert!(w.contains("Measure is missing"), "{w}");
}

#[test]
fn a_v3_manifest_serializes_byte_for_byte() {
    // The manifest as it was written before protocol 4 existed: none of the
    // three new keys appears.
    let p = smoke();
    let m = manifest(&p, "run-d");
    let text = serde_json::to_string(&m).unwrap();
    for key in ["chunk", "host", "planned_windows"] {
        assert!(!text.contains(key), "{key} in {text}");
    }
    let back: Manifest = serde_json::from_str(&text).expect("old bytes parse");
    assert_eq!(back, m);
    assert!(back.chunk.is_none() && back.host.is_none() && back.planned_windows.is_none());
}

// ---- T4.2 ------------------------------------------------------------------

#[test]
fn a_window_outcome_is_oom_killed_only_when_oom_kill_rose() {
    let at = |max, oom, oom_kill| resources::MemEvents { max, oom, oom_kill };
    use records::WindowOutcome::*;
    // `max` alone is pressure, not an OOM.
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(900, 0, 0), false, false),
        Measured
    );
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(900, 1, 0), false, false),
        Measured
    );
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(900, 1, 1), false, false),
        OomKilled
    );
    // It is the rise over the window, not the total.
    assert_eq!(
        runner::window_outcome(&at(0, 1, 2), &at(0, 1, 2), false, false),
        Measured
    );
    assert_eq!(
        runner::window_outcome(&at(0, 1, 2), &at(0, 2, 3), true, false),
        OomKilled
    );
    // A target that exited without an OOM is its own outcome.
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(0, 0, 0), true, false),
        TargetExited
    );
    // The chunk deadline is the last of the three: an OOM and an exit are
    // kernel and process facts and are never relabelled as a deadline.
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(0, 0, 0), false, true),
        ChunkDeadline
    );
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(0, 1, 1), false, true),
        OomKilled
    );
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(0, 0, 0), true, true),
        TargetExited
    );
}

#[test]
fn affinity_held_is_false_when_any_task_differs() {
    let proc_root = fixture();
    for (pid, tid, list) in [(5, 5, "2-3"), (5, 6, "2-3"), (6, 6, "2-3")] {
        let d = proc_root.path().join(format!("{pid}/task/{tid}"));
        std::fs::create_dir_all(&d).unwrap();
        put(&d, "status", &format!("Cpus_allowed_list:\t{list}\n"));
    }
    assert!(held(proc_root.path(), &[5, 6], &[2, 3]));
    put(
        &proc_root.path().join("6/task/6"),
        "status",
        "Cpus_allowed_list:\t2-4\n",
    );
    assert!(!held(proc_root.path(), &[5, 6], &[2, 3]));
}

#[tokio::test(start_paused = true)]
async fn the_sampler_writes_a_row_per_tick_and_one_at_each_bound() {
    let scope = fixture();
    let dir = leaf_fixture(scope.path(), "target-0001");
    put(&dir, "memory.current", "8192\n");
    put(&dir, "memory.stat", MEM_STAT);
    let current = dir.join("memory.current");
    let (leaf, _) = resources::Leaf::prepare(resources::host(), dir, "target-0001").unwrap();
    let clock = wire::Clock(Instant::now());
    let rows: Arc<Mutex<Vec<records::SampleRow>>> = Arc::default();
    let sink = rows.clone();
    let emit: resources::Emit = Arc::new(move |r| {
        sink.lock().unwrap().push(r);
        Ok(())
    });
    let sampler = resources::Sampler::spawn(
        leaf.clone(),
        "run-e".into(),
        TargetKind::Mailpit,
        clock,
        Duration::from_millis(100),
        emit,
    );
    // The paused clock advances by hand: no sleep stands in for a sync.
    for _ in 0..3 {
        tokio::time::advance(Duration::from_millis(100)).await;
        tokio::task::yield_now().await;
    }
    let n = sampler.stop().await.expect("the sampler ends cleanly");
    let got = rows.lock().unwrap().clone();
    assert_eq!(n as usize, got.len());
    assert!(
        got.len() >= 3,
        "a row at the start and one per tick: {}",
        got.len()
    );
    assert!(
        got.windows(2).all(|w| w[1].t_ns >= w[0].t_ns),
        "time moves forward"
    );
    let first = &got[0];
    assert_eq!(first.memory_current, 8192);
    assert_eq!(
        (first.anon, first.file, first.kernel, first.pids),
        (104_857_600, 52_428_800, 3_145_728, 1)
    );
    assert_eq!(first.launch, "target-0001");
    // A bound is one sample of the same kind, taken on demand.
    let at_bound = leaf.sample("run-e", TargetKind::Mailpit, &clock).unwrap();
    assert!(at_bound.t_ns >= got[got.len() - 1].t_ns);
    let snap = leaf.snap(&clock).unwrap();
    assert_eq!(snap.cpu.usage_usec, 8_123_456);
    // A reading that fails stops the sampler with the reason, not silently.
    std::fs::write(&current, "max\n").unwrap();
    let broken = resources::Sampler::spawn(
        leaf,
        "run-e".into(),
        TargetKind::Mailpit,
        clock,
        Duration::from_millis(100),
        Arc::new(|_| Ok(())),
    );
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;
    let w = broken.stop().await.unwrap_err();
    assert!(w.contains("memory.current"), "{w}");
}

// ---- T4.3 ------------------------------------------------------------------

#[test]
fn the_draft_plans_24_cost_chunks_in_rotation_order() {
    let p = p4(draft());
    let plan = runner::plan_v4(&p);
    assert_eq!(plan.len(), 24);
    let mut want = Vec::new();
    for rep in 0..3u32 {
        for t in p.order(rep) {
            if *t != TargetKind::Mailslurper {
                want.push(runner::Chunk::Cost {
                    target: *t,
                    repetition: rep,
                });
            }
        }
    }
    assert_eq!(plan, want);
    assert!(!plan.iter().any(|c| c.target() == TargetKind::Mailslurper));
    assert_eq!(plan[0].name(), "cost.swarmail.r0");
    assert_eq!(plan[8].name(), "cost.mailpit.r1");
    assert_eq!(plan[16].name(), "cost.mailhog.r2");
    assert_eq!(plan[23].name(), "cost.mailpit.r2");
    for rep in 0..3 {
        assert_eq!(plan.iter().filter(|c| c.repetition() == rep).count(), 8);
    }
    // Each chunk plans its profiles' rows and three windows apiece.
    let (planned, windows) = runner::plan_chunk(&p, &plan[0]);
    assert_eq!(
        planned.len(),
        12,
        "three row families for each of four profiles"
    );
    assert_eq!(windows.len(), 12);
    assert!(
        planned
            .iter()
            .all(|x| x.target == TargetKind::Swarmail && x.repetition == 0)
    );
}

#[test]
fn chunk_names_round_trip_and_unknown_ones_refuse() {
    let p = p4(draft());
    for c in runner::plan_v4(&p) {
        assert_eq!(runner::Chunk::parse(&c.name(), &p).unwrap(), c);
    }
    for (text, expect) in [
        ("cost.mailslurper.r0", "not in the protocol's plan"),
        ("cost.mailpit.r3", "not in the protocol's plan"),
        ("cost.nobody.r0", "unknown target"),
        ("api.mailpit.r0", "is not implemented"),
        ("cost.mailpit", "not a chunk coordinate"),
        ("cost.mailpit.0", "not r<repetition>"),
        ("", "not a chunk coordinate"),
        ("cost.mailpit.r0.x", "not a chunk coordinate"),
    ] {
        let w = runner::Chunk::parse(text, &p).unwrap_err();
        assert!(w.contains(expect), "{text:?}: {w}");
    }
}

#[test]
fn protocol_4_needs_a_chunk_and_protocol_3_refuses_one() {
    let p = p4(draft());
    let w = runner::v4_preflight(&p, "smoke", None, None, None).unwrap_err();
    assert!(w.contains("protocol 4 needs --chunk"), "{w}");
    let ok = runner::v4_preflight(&p, "smoke", Some("cost.mailpit.r1"), None, None).unwrap();
    assert_eq!(ok, Some(mailpit_chunk(1)));
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let w = runner::v4_preflight(&p3, "collect", Some("cost.mailpit.r1"), None, None).unwrap_err();
    assert!(w.contains("protocol 3 has no chunks"), "{w}");
    assert_eq!(
        runner::v4_preflight(&p3, "collect", None, None, None),
        Ok(None)
    );
    let p2 = Protocol::parse(PROTOCOL).unwrap();
    assert!(runner::v4_preflight(&p2, "smoke", Some("cost.mailpit.r0"), None, None).is_err());
    assert!(runner::v4_preflight(&p, "smoke", Some("bogus"), None, None).is_err());
}

#[test]
fn a_v4_collect_refuses_a_missing_or_dirty_commit() {
    let p = p4(draft());
    let chunk = Some("cost.mailpit.r0");
    let sha = "a".repeat(40);
    let w = runner::v4_preflight(&p, "collect", chunk, None, Some(false)).unwrap_err();
    assert!(w.contains("needs the candidate's commit"), "{w}");
    let w = runner::v4_preflight(&p, "collect", chunk, Some(&sha), Some(true)).unwrap_err();
    assert!(w.contains("dirty or not stated clean"), "{w}");
    let w = runner::v4_preflight(&p, "collect", chunk, Some(&sha), None).unwrap_err();
    assert!(w.contains("dirty or not stated clean"), "{w}");
    runner::v4_preflight(&p, "collect", chunk, Some(&sha), Some(false))
        .expect("a clean commit passes");
    // A smoke run is labelled and never evidence: it needs neither.
    runner::v4_preflight(&p, "smoke", chunk, None, None).unwrap();
}

// ---- T4.4 ------------------------------------------------------------------

#[test]
fn a_not_measured_profile_writes_its_rows_for_each_cause() {
    // One path for the three causes: its sends are `unstarted`, its
    // verification `incomplete`, each with the cause as reason, and its three
    // windows were never opened (no counters), with the matching outcome.
    use runner::Cause;
    for (cause, reason, outcome, launch) in [
        (
            Cause::ChunkDeadline,
            "chunk deadline",
            records::WindowOutcome::ChunkDeadline,
            "none",
        ),
        (
            Cause::OomKilled,
            "oom killed",
            records::WindowOutcome::OomKilled,
            "target-0003",
        ),
        (
            Cause::TargetExited,
            "target exited",
            records::WindowOutcome::TargetExited,
            "target-0004",
        ),
    ] {
        let p = tiny4();
        let m = tiny_manifest(&p, "run-f", &mailpit_chunk(0));
        let dir = fixture();
        let out = dir.path().join("run");
        let sink: runner::Sink = Arc::new(Mutex::new(records::RunDir::create(&out, &m).unwrap()));
        let clock = wire::Clock(Instant::now());
        runner::skip_profile(
            "run-f",
            &p.throughput[0],
            (TargetKind::Mailpit, 0),
            (cause, launch),
            &sink,
            &clock,
        )
        .expect("rows are written");
        drop(sink);
        let (rows, res) =
            records::read_run(&out, &m).expect("the planned rows and windows are all there");
        let sends: Vec<&Row> = rows.iter().filter(|r| r.phase == Phase::Send).collect();
        assert_eq!(sends.len(), 4);
        assert!(
            sends
                .iter()
                .all(|r| r.outcome == "unstarted" && r.reason.as_deref() == Some(reason)),
            "{reason}"
        );
        let verified = rows.iter().find(|r| r.phase == Phase::Verified).unwrap();
        assert_eq!(verified.outcome, "incomplete");
        assert_eq!(verified.reason.as_deref(), Some(reason));
        assert_eq!(res.len(), 3);
        for r in &res {
            let records::ResourceRow::Window(w) = r else {
                panic!("only windows are written")
            };
            assert_eq!(w.outcome, outcome);
            assert_eq!(w.launch, launch);
            assert!(w.counters().is_none(), "nothing was measured");
            assert_eq!((w.affinity_held, w.scratch_bytes), (None, None));
        }
    }
}

#[test]
fn a_chunk_with_any_unmeasured_window_exits_one() {
    use runner::Verdict;
    assert_eq!(Verdict::Clean.exit_code(), 0);
    assert_eq!(Verdict::WorkloadFailures(vec!["oom".into()]).exit_code(), 1);
    assert_eq!(Verdict::Infrastructure("x".into()).exit_code(), 2);
}

// ---- T4.5 ------------------------------------------------------------------

#[test]
fn median_min_max_over_one_two_and_three_values() {
    assert_eq!(stats::spread(&[]), None);
    let one = stats::spread(&[7.5]).unwrap();
    assert_eq!((one.median, one.min, one.max, one.n), (7.5, 7.5, 7.5, 1));
    // Nearest rank: ceil(50 * 2 / 100) = 1, the lower of two.
    let two = stats::spread(&[9.0, 3.0]).unwrap();
    assert_eq!((two.median, two.min, two.max, two.n), (3.0, 3.0, 9.0, 2));
    // ceil(50 * 3 / 100) = 2, the middle of three, whatever the order given.
    let three = stats::spread(&[4.0, 1.0, 2.0]).unwrap();
    assert_eq!(
        (three.median, three.min, three.max, three.n),
        (2.0, 1.0, 4.0, 3)
    );
    // A value that is not a number is not a figure.
    let nan = stats::spread(&[f64::NAN, 5.0, f64::INFINITY]).unwrap();
    assert_eq!((nan.median, nan.min, nan.max, nan.n), (5.0, 5.0, 5.0, 1));
    assert_eq!(stats::spread(&[f64::NAN]), None);
}

// ---- T4.6 ------------------------------------------------------------------

const MIB: u64 = 1 << 20;
const SEC: u64 = 1_000_000_000;

/// One repetition's measurements, all hand-chosen so the expected figures
/// below are worked out on paper, not read back from the code.
struct Rep {
    target: TargetKind,
    rep: u32,
    profile: &'static str,
    /// `cpu.stat` user and system µs over the measure window.
    cpu: (u64, u64),
    /// The memory (MiB) sampled mid-window; the bounds sample 100 MiB.
    peak_mib: u64,
    outcome: records::WindowOutcome,
    held: bool,
    oom_kill: u64,
    /// The three windows were never opened (outcome as `outcome`): no
    /// counters and no samples.
    unopened: bool,
    /// How the `ready` and `idle_ready` windows ended.
    startup: records::WindowOutcome,
    idle: records::WindowOutcome,
}

impl Rep {
    fn new(rep: u32, cpu: (u64, u64), peak_mib: u64) -> Rep {
        Rep {
            target: TargetKind::Mailpit,
            rep,
            profile: "cost-c8-reused",
            cpu,
            peak_mib,
            outcome: records::WindowOutcome::Measured,
            held: true,
            oom_kill: 0,
            unopened: false,
            startup: records::WindowOutcome::Measured,
            idle: records::WindowOutcome::Measured,
        }
    }
}

fn snap(t: u64, cpu: resources::CpuStat, oom_kill: u64) -> resources::Snap {
    resources::Snap {
        t_ns: t,
        cpu,
        events: resources::MemEvents {
            max: 0,
            oom: oom_kill,
            oom_kill,
        },
    }
}

fn cpu(user: u64, system: u64, throttled: u64) -> resources::CpuStat {
    resources::CpuStat {
        usage_usec: user + system,
        user_usec: user,
        system_usec: system,
        nr_throttled: throttled,
        throttled_usec: throttled * 1000,
    }
}

/// The resource rows of one repetition on a fresh clock of its own.
fn res_rows(run: &str, r: &Rep) -> Vec<records::ResourceRow> {
    use records::WindowKind::*;
    let launch = format!("target-{:04}", r.rep + 1);
    let key = (r.target, r.profile, r.rep);
    let mut out = Vec::new();
    let w = |kind, (a, b): (resources::Snap, resources::Snap), held: Option<bool>, scratch, o| {
        records::ResourceRow::Window(runner::window_row(
            run,
            &launch,
            key,
            kind,
            (&a, &b),
            held,
            scratch,
            o,
        ))
    };
    if r.unopened {
        for kind in [Ready, IdleReady, Measure] {
            out.push(records::ResourceRow::Window(runner::unopened_window(
                run, "none", key, kind, SEC, r.outcome,
            )));
        }
        return out;
    }
    out.push(w(
        Ready,
        (
            snap(0, cpu(0, 0, 0), 0),
            snap(SEC, cpu(100_000, 150_000, 0), 0),
        ),
        Some(true),
        None,
        r.startup,
    ));
    out.push(w(
        IdleReady,
        (
            snap(2 * SEC, cpu(0, 0, 0), 0),
            snap(7 * SEC, cpu(0, 0, 0), 0),
        ),
        Some(true),
        Some(4096),
        r.idle,
    ));
    out.push(w(
        Measure,
        (
            snap(8 * SEC, cpu(0, 0, 0), 0),
            snap(20 * SEC, cpu(r.cpu.0, r.cpu.1, 7), r.oom_kill),
        ),
        Some(r.held),
        None,
        r.outcome,
    ));
    let mut s = |t: u64, mib: u64, anon_mib: u64| {
        let mut row = sample_at(run, &launch, t, mib * MIB, anon_mib * MIB);
        row.target = r.target;
        out.push(records::ResourceRow::Sample(row));
    };
    // The idle window: only the last two seconds (5 s to 7 s) count.
    for (t, mib) in [
        (2 * SEC, 900),
        (4 * SEC, 900),
        (5 * SEC + SEC / 2, 100),
        (6 * SEC, 200),
        (7 * SEC, 300),
    ] {
        s(t, mib, mib / 2);
    }
    // The measure window, with samples outside it that must not count.
    s(8 * SEC - 1, 999, 999);
    s(8 * SEC, 100, 50);
    s(14 * SEC, r.peak_mib, r.peak_mib / 2);
    s(20 * SEC, 100, 50);
    s(20 * SEC + 1, 998, 998);
    out
}

/// Message rows of one exact repetition: four accepted, four visible once,
/// fidelity intact, one verification.
fn msg_rows(run: &str, r: &Rep) -> Vec<Row> {
    let mut rows = Vec::new();
    for i in 0..4 {
        let mut s = runner::row(run, r.target, r.profile, r.rep, Phase::Send, i, "accepted");
        s.token = Some(format!("t{i}"));
        s.invoked_ns = Some(8 * SEC + u64::from(i));
        s.start_ns = Some(8 * SEC + u64::from(i) + 1);
        s.ack_ns = Some(8 * SEC + u64::from(i) + 1_000_001);
        rows.push(s);
        let mut v = runner::row(
            run,
            r.target,
            r.profile,
            r.rep,
            Phase::Visible,
            i,
            "visible",
        );
        v.token = Some(format!("t{i}"));
        v.occurrences = Some(1);
        v.fidelity_ok = Some(true);
        rows.push(v);
    }
    let mut v = runner::row(
        run,
        r.target,
        r.profile,
        r.rep,
        Phase::Verified,
        0,
        "verified",
    );
    v.start_ns = Some(8 * SEC);
    v.ack_ns = Some(9 * SEC);
    v.observed_ns = Some(10 * SEC);
    rows.push(v);
    rows
}

fn life() -> runner::Lifecycle {
    runner::Lifecycle {
        launches: vec![],
        cleanups: vec![],
    }
}

type Editor<'a> = Option<&'a dyn Fn(&mut Vec<Row>)>;

fn v4_report(reps: &[(Rep, Editor)]) -> report::Report {
    let mut names: Vec<&str> = reps.iter().map(|(r, _)| r.profile).collect();
    names.dedup();
    let mut v = draft();
    let proto = v["throughput"][1].clone();
    v["throughput"] = json!(
        names
            .iter()
            .map(|n| {
                let mut t = proto.clone();
                t["name"] = json!(n);
                t["messages"] = json!(4);
                t["warmup_messages"] = json!(0);
                t["sessions"] = json!(2);
                t
            })
            .collect::<Vec<_>>()
    );
    let p = p4(v);
    let m = tiny_manifest(&p, "run-g", &mailpit_chunk(0));
    let (mut rows, mut groups) = (Vec::new(), Vec::new());
    for (r, edit) in reps {
        let mut mine = msg_rows("run-g", r);
        if let Some(edit) = edit {
            edit(&mut mine);
        }
        rows.append(&mut mine);
        groups.push(res_rows("run-g", r));
    }
    let g: Vec<&[records::ResourceRow]> = groups.iter().map(Vec::as_slice).collect();
    report::report_v4(&p, &m, &rows, &g, &life())
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{a} is not {b}");
}

#[test]
fn cost_figures_equal_independent_arithmetic() {
    // CPU totals 2 s, 4 s and 1 s over 4 visible mails each.
    let r = v4_report(&[
        (Rep::new(0, (1_500_000, 500_000), 300), None),
        (Rep::new(1, (3_000_000, 1_000_000), 400), None),
        (Rep::new(2, (600_000, 400_000), 200), None),
    ]);
    let cost = &r.v4.as_ref().unwrap().cost;
    assert_eq!(cost.repetitions.len(), 3);
    let first = &cost.repetitions[0];
    assert_eq!(
        (
            first.cpu_user_usec,
            first.cpu_system_usec,
            first.cpu_total_usec
        ),
        (Some(1_500_000), Some(500_000), Some(2_000_000))
    );
    assert_eq!(first.visible, Some(4));
    near(first.visible_per_cpu_second.unwrap(), 2.0); // 4 mails / 2 s
    near(first.cpu_ms_per_1k_visible.unwrap(), 500_000.0); // 2000 ms / 0.004 k
    assert_eq!(
        (first.nr_throttled, first.throttled_usec),
        (Some(7), Some(7000))
    );
    let [cell] = cost.cells.as_slice() else {
        panic!("one cell")
    };
    assert_eq!(
        (cell.target, cell.profile.as_str(), cell.repetitions),
        (TargetKind::Mailpit, "cost-c8-reused", 3)
    );
    // Per second: 2, 1 and 4 mails; the median is the middle one.
    let s = cell.visible_per_cpu_second.unwrap();
    assert_eq!((s.median, s.min, s.max), (2.0, 1.0, 4.0));
    // Per 1k: 500,000, 1,000,000 and 250,000 ms.
    let s = cell.cpu_ms_per_1k_visible.unwrap();
    assert_eq!(
        (s.median, s.min, s.max),
        (500_000.0, 250_000.0, 1_000_000.0)
    );
    // Peak memory 300, 400 and 200 MiB; anon half of each.
    let s = cell.peak_memory_mib.unwrap();
    assert_eq!((s.median, s.min, s.max), (300.0, 200.0, 400.0));
    let s = cell.peak_anon_mib.unwrap();
    assert_eq!((s.median, s.min, s.max), (150.0, 100.0, 200.0));
    assert!(cell.ranked && cell.shortfalls.is_empty() && cell.labels.is_empty());
}

#[test]
fn a_zero_divisor_is_null_never_zero_or_infinite() {
    // No CPU time at all: a per-second figure has no divisor.
    let r = v4_report(&[(Rep::new(0, (0, 0), 300), None)]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    assert_eq!(rep.visible_per_cpu_second, None);
    // Its divisor is the visible count, which is 4: no CPU is zero ms.
    assert_eq!(rep.cpu_ms_per_1k_visible, Some(0.0));
    // Nothing visible: a per-1k figure has no divisor, though CPU was spent.
    let gone: &dyn Fn(&mut Vec<Row>) = &|rows| rows.retain(|r| r.phase != Phase::Visible);
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), Some(gone))]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    assert_eq!(rep.visible, Some(0));
    assert_eq!(rep.cpu_ms_per_1k_visible, None);
    near(rep.visible_per_cpu_second.unwrap(), 0.0);
    // Both zero: neither is a number, and the serialized report says null.
    let r = v4_report(&[(Rep::new(0, (0, 0), 300), Some(gone))]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    assert_eq!(
        (rep.visible_per_cpu_second, rep.cpu_ms_per_1k_visible),
        (None, None)
    );
    let text = serde_json::to_string(&r).unwrap();
    assert!(!text.contains("inf") && !text.contains("NaN"), "{text}");
    let cell = &r.v4.as_ref().unwrap().cost.cells[0];
    assert!(cell.visible_per_cpu_second.is_none() && cell.cpu_ms_per_1k_visible.is_none());
}

fn only_cell(r: &report::Report) -> &report::CostCell {
    let cells = &r.v4.as_ref().unwrap().cost.cells;
    assert_eq!(cells.len(), 1);
    &cells[0]
}

#[test]
fn an_inexact_repetition_unranks_the_cell_with_its_shortfall() {
    use records::WindowOutcome as O;
    type Edit = Box<dyn Fn(&mut Vec<Row>)>;
    fn drop_visible(rows: &mut Vec<Row>, token: &str) {
        rows.retain(|r| !(r.phase == Phase::Visible && r.token.as_deref() == Some(token)));
    }
    fn set_send(rows: &mut [Row], outcome: &str) {
        let r = rows
            .iter_mut()
            .find(|r| r.phase == Phase::Send && r.sample == 3)
            .unwrap();
        r.outcome = outcome.into();
    }
    let cases: Vec<(&str, Edit, O)> = vec![
        (
            "refused",
            Box::new(|rows| {
                set_send(rows, "rejected");
                drop_visible(rows, "t3");
            }),
            O::Measured,
        ),
        (
            "lost",
            Box::new(|rows| drop_visible(rows, "t3")),
            O::Measured,
        ),
        (
            "fidelity",
            Box::new(|rows| {
                rows.iter_mut()
                    .find(|r| r.phase == Phase::Visible)
                    .unwrap()
                    .fidelity_ok = Some(false);
            }),
            O::Measured,
        ),
        ("oom", Box::new(|_| {}), O::OomKilled),
        ("timeout", Box::new(|_| {}), O::ChunkDeadline),
        ("target_exited", Box::new(|_| {}), O::TargetExited),
        (
            "duplicate",
            Box::new(|rows| {
                rows.iter_mut()
                    .find(|r| r.phase == Phase::Visible)
                    .unwrap()
                    .occurrences = Some(2);
            }),
            O::Measured,
        ),
        (
            "unexpected",
            Box::new(|rows| {
                let mut extra = runner::row(
                    "run-g",
                    TargetKind::Mailpit,
                    "cost-c8-reused",
                    0,
                    Phase::Visible,
                    9,
                    "unexpected",
                );
                extra.token = Some("stranger".into());
                extra.occurrences = Some(1);
                rows.push(extra);
            }),
            O::Measured,
        ),
        (
            "ambiguous",
            Box::new(|rows| {
                set_send(rows, "ambiguous");
                drop_visible(rows, "t3");
            }),
            O::Measured,
        ),
    ];
    for (name, edit, outcome) in &cases {
        let mut rep = Rep::new(0, (1_000_000, 0), 300);
        rep.outcome = *outcome;
        let r = v4_report(&[(rep, Some(&**edit))]);
        let cell = only_cell(&r);
        assert!(!cell.ranked, "{name} must not be ranked");
        assert_eq!(cell.shortfalls, vec![*name], "{name}");
        // Its figures are published all the same.
        assert!(
            cell.peak_memory_mib.is_some(),
            "{name}: a shortfall keeps its figures"
        );
        assert!(cell.visible_per_cpu_second.is_some(), "{name}");
    }
    // One inexact repetition among exact ones unranks the whole cell, and
    // the other two keep their own figures.
    let bad: &dyn Fn(&mut Vec<Row>) = &*cases[1].1;
    let r = v4_report(&[
        (Rep::new(0, (1_000_000, 0), 300), None),
        (Rep::new(1, (1_000_000, 0), 300), Some(bad)),
        (Rep::new(2, (1_000_000, 0), 300), None),
    ]);
    let cell = only_cell(&r);
    assert!(!cell.ranked);
    assert_eq!(cell.shortfalls, vec!["lost"]);
    assert_eq!(cell.repetitions, 3);
    let reps = &r.v4.as_ref().unwrap().cost.repetitions;
    assert!(reps[0].exact && !reps[1].exact && reps[2].exact);
    assert!(reps[0].shortfalls.is_empty());
}

#[test]
fn an_unmeasured_window_unranks_the_cell() {
    // Every message exact, but the window was not measured.
    for outcome in [
        records::WindowOutcome::OomKilled,
        records::WindowOutcome::TargetExited,
        records::WindowOutcome::ChunkDeadline,
    ] {
        let mut rep = Rep::new(0, (1_000_000, 0), 300);
        rep.outcome = outcome;
        let r = v4_report(&[(rep, None)]);
        let reps = &r.v4.as_ref().unwrap().cost.repetitions;
        assert!(reps[0].exact, "the rows themselves are exact");
        assert_eq!(reps[0].window_outcome, outcome);
        assert!(!only_cell(&r).ranked, "{outcome:?}");
    }
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), None)]);
    assert!(only_cell(&r).ranked);
}

#[test]
fn affinity_widened_is_a_label_and_does_not_change_ranking() {
    let mut rep = Rep::new(0, (1_000_000, 0), 300);
    rep.held = false;
    let r = v4_report(&[(rep, None)]);
    let cell = only_cell(&r);
    assert_eq!(cell.labels, vec!["affinity_widened"]);
    assert!(cell.ranked, "it is a label: ranking stays 0132 §5.2's");
    assert!(cell.shortfalls.is_empty());
    assert_eq!(
        r.v4.as_ref().unwrap().cost.repetitions[0].affinity_held,
        Some(false)
    );
}

#[test]
fn peak_and_mean_come_from_the_samples_inside_the_window() {
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), None)]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    // Inside [8 s, 20 s]: 100, 300 and 100 MiB (anon 50, 150, 50). The
    // samples just outside, at 999 and 998 MiB, are not read.
    assert_eq!(rep.peak_memory_bytes, Some(300 * MIB));
    assert_eq!(rep.peak_anon_bytes, Some(150 * MIB));
    near(
        rep.mean_memory_bytes.unwrap(),
        (100.0 + 300.0 + 100.0) / 3.0 * MIB as f64,
    );
    near(
        rep.mean_anon_bytes.unwrap(),
        (50.0 + 150.0 + 50.0) / 3.0 * MIB as f64,
    );
    // A second launch's samples on the same clock are not mixed in.
    let mut both = res_rows("run-g", &Rep::new(0, (1_000_000, 0), 300));
    let mut other = Rep::new(1, (1_000_000, 0), 900);
    other.profile = "cost-c8-reused";
    let mut noise = res_rows("run-g", &other);
    noise.retain(|r| matches!(r, records::ResourceRow::Sample(_)));
    for r in &mut noise {
        if let records::ResourceRow::Sample(s) = r {
            s.launch = "target-0099".into();
        }
    }
    both.extend(noise);
    let p = tiny4();
    let m = tiny_manifest(&p, "run-g", &mailpit_chunk(0));
    let rows = msg_rows("run-g", &Rep::new(0, (1_000_000, 0), 300));
    let r = report::report_v4(&p, &m, &rows, &[&both[..]], &life());
    assert_eq!(
        r.v4.as_ref().unwrap().cost.repetitions[0].peak_memory_bytes,
        Some(300 * MIB)
    );
}

#[test]
fn idle_ready_is_the_mean_of_its_last_two_seconds() {
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), None)]);
    let v4 = r.v4.as_ref().unwrap();
    // Samples at 5.5 s, 6 s and 7 s (100, 200, 300 MiB) are in the last two
    // seconds of a window that ends at 7 s; the 900 MiB ones are not.
    let cell = &v4.idle[0];
    assert_eq!(
        (cell.target, cell.profile.as_str(), cell.repetitions),
        (TargetKind::Mailpit, "cost-c8-reused", 1)
    );
    near(cell.memory_mib.unwrap().median, 200.0);
    near(cell.anon_mib.unwrap().median, 100.0);
    assert_eq!(cell.scratch_bytes.unwrap().median, 4096.0);
    // Startup: ready at 1 s, with 100 ms of user and 150 ms of system CPU.
    let up = &v4.startup[0];
    assert_eq!(up.ready_ms.unwrap().median, 1000.0);
    assert_eq!(up.cpu_ms_to_ready.unwrap().median, 250.0);
}

#[test]
fn the_headline_is_the_cells_of_cost_c8_and_owes_mib_per_1k_retained() {
    let mut other = Rep::new(0, (4_000_000, 0), 500);
    other.profile = "cost-c1-reused";
    let r = v4_report(&[(other, None), (Rep::new(0, (1_000_000, 0), 300), None)]);
    let v4 = r.v4.as_ref().unwrap();
    assert_eq!(v4.cost.cells.len(), 2);
    assert_eq!(v4.headline.profile, "cost-c8-reused");
    assert_eq!(v4.headline.cost.len(), 1);
    assert_eq!(v4.headline.cost[0].profile, "cost-c8-reused");
    assert!(
        v4.headline
            .idle
            .iter()
            .chain(&[])
            .all(|c| c.profile == "cost-c8-reused")
    );
    assert_eq!(v4.headline.idle.len(), 1);
    assert_eq!(v4.headline.startup.len(), 1);
    // Fill levels belong to story B: owed, and no number appears.
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["mib_per_1k_retained"]["status"], "owed");
    assert!(
        j["mib_per_1k_retained"]["owed_by"]
            .as_str()
            .unwrap()
            .contains("story B")
    );
    for key in ["cost", "idle", "startup", "headline", "mib_per_1k_retained"] {
        assert!(j.get(key).is_some(), "{key} is a key of the report");
    }
}

#[test]
fn the_blocked_row_has_no_number() {
    let p = p4(draft());
    let m = tiny_manifest(&p, "run-h", &mailpit_chunk(0));
    let r = report::report_v4(&p, &m, &[], &[], &life());
    assert_eq!(r.blocked.len(), 1);
    let b = &r.blocked[0];
    assert_eq!(b.target, TargetKind::Mailslurper);
    assert!(b.result.contains("does not start"));
    assert_eq!(b.cells.len(), 4, "every cost profile is blocked");
    assert!(b.cells.iter().all(|c| c.outcome == "blocked"));
    let v4 = r.v4.as_ref().unwrap();
    assert!(
        !v4.cost
            .cells
            .iter()
            .any(|c| c.target == TargetKind::Mailslurper)
    );
    assert!(v4.cost.repetitions.is_empty() && v4.idle.is_empty() && v4.startup.is_empty());
    // The label row says so too.
    let label = r
        .labels
        .iter()
        .find(|l| l.target == TargetKind::Mailslurper)
        .unwrap();
    assert!(label.does_not_start.is_some());
}

#[test]
fn a_v4_report_is_version_3_and_a_v3_report_is_still_version_2() {
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), None)]);
    assert_eq!(r.report_version, 3);
    assert_eq!(report::REPORT_VERSION_V4, 3);
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let m3 = manifest(&p3, "run-i");
    let r3 = report::report(&p3, &m3, &[], &life());
    assert_eq!(r3.report_version, 2);
    assert_eq!(report::REPORT_VERSION, 2);
    let j = serde_json::to_value(&r3).unwrap();
    for key in ["cost", "idle", "startup", "headline", "mib_per_1k_retained"] {
        assert!(
            j.get(key).is_none(),
            "{key} must not appear in a version 2 report"
        );
    }
}

// ============================================================================
// R3: no observation, no number
// ============================================================================

/// Every send `unstarted` and the verification `incomplete`, as a profile
/// that was not measured leaves them.
fn unstarted_rows(run: &str, r: &Rep, reason: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for i in 0..4 {
        let mut s = runner::row(run, r.target, r.profile, r.rep, Phase::Send, i, "unstarted");
        s.reason = Some(reason.into());
        rows.push(s);
    }
    let mut v = runner::row(
        run,
        r.target,
        r.profile,
        r.rep,
        Phase::Verified,
        0,
        "incomplete",
    );
    v.reason = Some(reason.into());
    (v.start_ns, v.ack_ns, v.observed_ns) = (Some(8 * SEC), Some(8 * SEC), Some(8 * SEC));
    v.requests = Some(0);
    rows.push(v);
    rows
}

/// A repetition whose three windows were never opened, for `outcome`.
fn dead(rep: u32, outcome: records::WindowOutcome) -> Rep {
    let mut r = Rep::new(rep, (0, 0), 0);
    r.unopened = true;
    r.outcome = outcome;
    r
}

fn dead_rows(reason: &'static str) -> impl Fn(&mut Vec<Row>) {
    move |rows: &mut Vec<Row>| {
        let r = Rep::new(rows[0].repetition, (0, 0), 0);
        *rows = unstarted_rows("run-g", &r, reason);
    }
}

/// Every number under `v` that is zero, skipping the coordinates that name a
/// repetition (an index is not a figure).
fn zeros(v: &Value, path: &str, out: &mut Vec<String>) {
    match v {
        Value::Number(n) if n.as_f64() == Some(0.0) => out.push(path.to_owned()),
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                zeros(x, &format!("{path}[{i}]"), out);
            }
        }
        Value::Object(m) => {
            for (k, x) in m {
                if k != "repetition" {
                    zeros(x, &format!("{path}.{k}"), out);
                }
            }
        }
        _ => {}
    }
}

#[test]
fn a_never_opened_window_serializes_null_counters_and_never_zero() {
    let w = runner::unopened_window(
        "run-h",
        "none",
        (TargetKind::Mailpit, "cost-c8-reused", 0),
        records::WindowKind::Measure,
        7,
        records::WindowOutcome::ChunkDeadline,
    );
    let v = serde_json::to_value(&w).unwrap();
    for k in [
        "cpu_start",
        "cpu_end",
        "events_start",
        "events_end",
        "affinity_held",
        "scratch_bytes",
    ] {
        assert!(v[k].is_null(), "{k}: {v}");
    }
    let text = serde_json::to_string(&w).unwrap();
    assert!(
        !text.contains("usage_usec") && !text.contains("oom_kill"),
        "{text}"
    );
    assert_eq!(
        (w.start_ns, w.end_ns),
        (7, 7),
        "both bounds are the instant it was given up"
    );
    assert!(w.counters().is_none());
}

#[test]
fn the_reader_refuses_a_measured_window_without_counters() {
    let p = tiny4();
    let m = tiny_manifest(&p, "run-i", &mailpit_chunk(0));
    let dir = fixture();
    let mut ws = all_windows("run-i", 0);
    let write = |ws: &[records::WindowRow]| {
        let mut lines = message_lines("run-i", 0);
        lines.extend(ws.iter().map(window_line));
        write_samples(dir.path(), &lines);
    };
    write(&ws);
    records::read_run(dir.path(), &m).expect("the whole set reads");
    ws[2].cpu_start = None;
    ws[2].cpu_end = None;
    ws[2].events_start = None;
    ws[2].events_end = None;
    ws[2].affinity_held = None;
    write(&ws);
    let w = records::read_run(dir.path(), &m).unwrap_err();
    assert!(w.contains("is measured but has no counters"), "{w}");
    // The same window that was never opened reads, with its own outcome.
    ws[2].outcome = records::WindowOutcome::ChunkDeadline;
    write(&ws);
    records::read_run(dir.path(), &m).expect("a never-opened window reads");
    // And it may not carry an affinity or a scratch size it never read.
    ws[2].affinity_held = Some(true);
    write(&ws);
    let w = records::read_run(dir.path(), &m).unwrap_err();
    assert!(w.contains("never opened but carries"), "{w}");
}

#[test]
fn the_reader_refuses_a_window_with_partial_counters() {
    let p = tiny4();
    let m = tiny_manifest(&p, "run-i", &mailpit_chunk(0));
    let dir = fixture();
    for field in ["cpu_start", "cpu_end", "events_start", "events_end"] {
        let mut ws = all_windows("run-i", 0);
        match field {
            "cpu_start" => ws[0].cpu_start = None,
            "cpu_end" => ws[0].cpu_end = None,
            "events_start" => ws[0].events_start = None,
            _ => ws[0].events_end = None,
        }
        let mut lines = message_lines("run-i", 0);
        lines.extend(ws.iter().map(window_line));
        write_samples(dir.path(), &lines);
        let w = records::read_run(dir.path(), &m).unwrap_err();
        assert!(w.contains("some counters and not others"), "{field}: {w}");
    }
}

#[test]
fn the_reader_refuses_a_sample_of_launch_none() {
    let p = tiny4();
    let m = tiny_manifest(&p, "run-i", &mailpit_chunk(0));
    let dir = fixture();
    let mut lines = message_lines("run-i", 0);
    let sample = sample_at("run-i", "none", 5, 10, 4);
    lines.push(serde_json::to_string(&records::ResourceRow::Sample(sample)).unwrap());
    lines.extend(all_windows("run-i", 0).iter().map(window_line));
    write_samples(dir.path(), &lines);
    let w = records::read_run(dir.path(), &m).unwrap_err();
    assert!(w.contains("sample of launch none"), "{w}");
}

#[test]
fn affinity_is_read_at_the_end_of_each_window_kind() {
    use records::WindowKind::*;
    let scope = fixture();
    let dir = leaf_fixture(scope.path(), "target-0001");
    let (leaf, _) =
        resources::Leaf::prepare(resources::host(), dir.clone(), "target-0001").unwrap();
    let proc_root = fixture();
    let task = |pid: u32, tid: u32, list: &str| {
        let d = proc_root.path().join(format!("{pid}/task/{tid}"));
        std::fs::create_dir_all(&d).unwrap();
        put(&d, "status", &format!("Cpus_allowed_list:\t{list}\n"));
    };
    let closing = |procs: &str| {
        put(&dir, "cgroup.procs", procs);
        runner::Closing {
            run: "run-j",
            launch: "target-0001",
            key: (TargetKind::Mailpit, "cost-c8-reused", 0),
            leaf: &leaf,
            proc_root: proc_root.path(),
            target_cpus: &[2, 3],
        }
    };
    let snaps = (zero_snap(0), zero_snap(SEC));
    let m = records::WindowOutcome::Measured;
    for kind in [Ready, IdleReady, Measure] {
        // No task at all: no observation, not a pass.
        let w = closing("").window(kind, (&snaps.0, &snaps.1), None, m);
        assert_eq!(w.affinity_held, None, "{kind:?}: an empty leaf");
        // Every task on the pair.
        task(5, 5, "2-3");
        task(5, 6, "2-3");
        let w = closing("5\n").window(kind, (&snaps.0, &snaps.1), None, m);
        assert_eq!(w.affinity_held, Some(true), "{kind:?}: the pair");
        // One task differing.
        task(5, 6, "2-4");
        let w = closing("5\n").window(kind, (&snaps.0, &snaps.1), None, m);
        assert_eq!(w.affinity_held, Some(false), "{kind:?}: one widened");
        task(5, 6, "2-3");
        // A pid listed that has no /proc entry: still no observation.
        let w = closing("99\n").window(kind, (&snaps.0, &snaps.1), None, m);
        assert_eq!(w.affinity_held, None, "{kind:?}: a vanished process");
    }
}

#[test]
fn a_repetition_that_was_not_measured_has_null_figures() {
    use records::WindowOutcome::ChunkDeadline;
    let gone = dead_rows("chunk deadline");
    let r = v4_report(&[(dead(0, ChunkDeadline), Some(&gone))]);
    let v = serde_json::to_value(&r).unwrap();
    let rep = &v["cost"]["repetitions"][0];
    for k in [
        "visible",
        "cpu_user_usec",
        "cpu_system_usec",
        "cpu_total_usec",
        "peak_memory_bytes",
        "mean_memory_bytes",
        "peak_anon_bytes",
        "mean_anon_bytes",
        "oom_kills",
        "nr_throttled",
        "throttled_usec",
        "affinity_held",
        "visible_per_cpu_second",
        "cpu_ms_per_1k_visible",
    ] {
        assert!(rep[k].is_null(), "{k}: {rep}");
    }
    let cell = only_cell(&r);
    assert!(!cell.ranked);
    assert_eq!(cell.shortfalls, vec!["timeout"]);
    assert!(cell.cpu_ms_per_1k_visible.is_none() && cell.peak_memory_mib.is_none());
    assert_eq!(rep["window_outcome"], "chunk_deadline");
}

#[test]
fn one_of_three_not_measured_gives_a_median_over_two_with_n_two() {
    use records::WindowOutcome::ChunkDeadline;
    let gone = dead_rows("chunk deadline");
    let r = v4_report(&[
        (Rep::new(0, (1_500_000, 500_000), 300), None),
        (Rep::new(1, (3_000_000, 1_000_000), 400), None),
        (dead(2, ChunkDeadline), Some(&gone)),
    ]);
    let cell = only_cell(&r);
    assert_eq!(cell.repetitions, 3);
    // 2 and 1 mails per CPU second: the lower of two is the median.
    let s = cell.visible_per_cpu_second.unwrap();
    assert_eq!((s.median, s.min, s.max, s.n), (1.0, 1.0, 2.0, 2));
    let s = cell.cpu_ms_per_1k_visible.unwrap();
    assert_eq!(
        (s.median, s.min, s.max, s.n),
        (500_000.0, 500_000.0, 1_000_000.0, 2)
    );
    let s = cell.peak_memory_mib.unwrap();
    assert_eq!((s.median, s.min, s.max, s.n), (300.0, 300.0, 400.0, 2));
    // The idle and startup cells are over the two as well, and unranked.
    let v4 = r.v4.as_ref().unwrap();
    assert_eq!(v4.idle[0].memory_mib.unwrap().n, 2);
    assert_eq!(v4.startup[0].ready_ms.unwrap().n, 2);
    assert!(!v4.idle[0].ranked && !v4.startup[0].ranked);
}

#[test]
fn three_not_measured_give_null_figures_and_no_zero() {
    use records::WindowOutcome::ChunkDeadline;
    let gone = dead_rows("chunk deadline");
    let r = v4_report(&[
        (dead(0, ChunkDeadline), Some(&gone)),
        (dead(1, ChunkDeadline), Some(&gone)),
        (dead(2, ChunkDeadline), Some(&gone)),
    ]);
    let v = serde_json::to_value(&r.v4).unwrap();
    let mut found = Vec::new();
    for key in ["cost", "idle", "startup", "headline"] {
        zeros(&v[key], key, &mut found);
    }
    assert!(
        found.is_empty(),
        "a zero where a figure is meant: {found:?}"
    );
    let cell = only_cell(&r);
    assert!(
        cell.visible_per_cpu_second.is_none()
            && cell.cpu_ms_per_1k_visible.is_none()
            && cell.peak_memory_mib.is_none()
            && cell.peak_anon_mib.is_none()
    );
    let v4 = r.v4.as_ref().unwrap();
    assert!(v4.idle[0].memory_mib.is_none() && v4.idle[0].scratch_bytes.is_none());
    assert!(v4.startup[0].ready_ms.is_none() && v4.startup[0].cpu_ms_to_ready.is_none());
}

#[test]
fn idle_and_startup_cells_are_unranked_and_name_the_window_outcome() {
    use records::WindowOutcome as O;
    for (outcome, name) in [
        (O::ChunkDeadline, "timeout"),
        (O::OomKilled, "oom"),
        (O::TargetExited, "target_exited"),
    ] {
        let mut rep = Rep::new(0, (1_000_000, 0), 300);
        (rep.startup, rep.idle) = (outcome, outcome);
        let r = v4_report(&[(rep, None)]);
        let v4 = r.v4.as_ref().unwrap();
        for (ranked, shortfalls) in [
            (v4.idle[0].ranked, &v4.idle[0].shortfalls),
            (v4.startup[0].ranked, &v4.startup[0].shortfalls),
        ] {
            assert!(!ranked, "{outcome:?}");
            assert_eq!(shortfalls, &vec![name], "{outcome:?}");
        }
        assert!(v4.idle[0].memory_mib.is_none() && v4.startup[0].ready_ms.is_none());
    }
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), None)]);
    let v4 = r.v4.as_ref().unwrap();
    assert!(v4.idle[0].ranked && v4.startup[0].ranked);
    assert!(v4.idle[0].shortfalls.is_empty() && v4.startup[0].shortfalls.is_empty());
}

#[test]
fn a_target_that_exited_in_its_idle_window_has_no_idle_figure() {
    let mut rep = Rep::new(0, (1_000_000, 0), 300);
    rep.idle = records::WindowOutcome::TargetExited;
    let r = v4_report(&[(rep, None)]);
    let v4 = r.v4.as_ref().unwrap();
    let idle = &v4.idle[0];
    assert!(idle.memory_mib.is_none() && idle.anon_mib.is_none() && idle.scratch_bytes.is_none());
    assert_eq!(idle.shortfalls, vec!["target_exited"]);
    // It was ready, so its startup figure stands.
    assert!(v4.startup[0].ready_ms.is_some() && v4.startup[0].ranked);
}

#[test]
fn an_opened_oom_killed_measure_window_keeps_its_cpu_and_peak_unranked() {
    let mut rep = Rep::new(0, (1_500_000, 500_000), 300);
    (rep.outcome, rep.oom_kill) = (records::WindowOutcome::OomKilled, 2);
    let r = v4_report(&[(rep, None)]);
    let c = &r.v4.as_ref().unwrap().cost.repetitions[0];
    assert_eq!(c.cpu_total_usec, Some(2_000_000));
    assert_eq!(c.peak_memory_bytes, Some(300 * MIB));
    assert_eq!(c.oom_kills, Some(2));
    let cell = only_cell(&r);
    assert!(!cell.ranked);
    assert_eq!(cell.shortfalls, vec!["oom"]);
    assert!(cell.peak_memory_mib.is_some() && cell.cpu_ms_per_1k_visible.is_some());
}

#[test]
fn a_verification_cut_by_the_chunk_deadline_names_timeout_not_lost() {
    // Four accepted, nothing listed because the deadline came: not lost.
    let cut = |reason: &'static str| {
        move |rows: &mut Vec<Row>| {
            rows.retain(|r| r.phase != Phase::Visible);
            let v = rows
                .iter_mut()
                .find(|r| r.phase == Phase::Verified)
                .unwrap();
            v.outcome = "incomplete".into();
            v.reason = Some(reason.into());
        }
    };
    let by_deadline = cut("chunk deadline");
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), Some(&by_deadline))]);
    let cell = only_cell(&r);
    assert!(!cell.ranked);
    assert!(
        cell.shortfalls.contains(&"timeout"),
        "{:?}",
        cell.shortfalls
    );
    assert!(!cell.shortfalls.contains(&"lost"), "{:?}", cell.shortfalls);
    // The same rows ended by the profile's own verify deadline are lost.
    let by_verify = cut("verify deadline");
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), Some(&by_verify))]);
    let cell = only_cell(&r);
    assert!(cell.shortfalls.contains(&"lost"), "{:?}", cell.shortfalls);
    assert!(
        !cell.shortfalls.contains(&"timeout"),
        "{:?}",
        cell.shortfalls
    );
}

#[test]
fn unstarted_rows_of_a_dead_target_do_not_name_timeout() {
    use records::WindowOutcome as O;
    for (outcome, reason, name) in [
        (O::TargetExited, "target exited", "target_exited"),
        (O::OomKilled, "oom killed", "oom"),
    ] {
        let gone = dead_rows(reason);
        let r = v4_report(&[(dead(0, outcome), Some(&gone))]);
        let cell = only_cell(&r);
        assert_eq!(cell.shortfalls, vec![name], "{reason}");
    }
    // Whereas rows left unstarted by a deadline do.
    let gone = dead_rows("send deadline");
    let r = v4_report(&[(dead(0, O::Measured), Some(&gone))]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    assert!(rep.shortfalls.contains(&"timeout"), "{:?}", rep.shortfalls);
}

#[test]
fn an_unread_fidelity_is_not_exact_and_not_a_fidelity_mismatch() {
    let unread: &dyn Fn(&mut Vec<Row>) = &|rows| {
        let v = rows.iter_mut().find(|r| r.phase == Phase::Visible).unwrap();
        v.fidelity_ok = None;
    };
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), Some(unread))]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    assert!(
        !rep.exact,
        "a visible row whose fidelity was never read is not exact"
    );
    assert!(
        !rep.shortfalls.contains(&"fidelity"),
        "{:?}",
        rep.shortfalls
    );
    assert!(!only_cell(&r).ranked);
    // A read that failed is a mismatch, as before.
    let wrong: &dyn Fn(&mut Vec<Row>) = &|rows| {
        let v = rows.iter_mut().find(|r| r.phase == Phase::Visible).unwrap();
        v.fidelity_ok = Some(false);
    };
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), Some(wrong))]);
    assert!(only_cell(&r).shortfalls.contains(&"fidelity"));
}

// ============================================================================
// T5: footprint, hold and admit-v4
// ============================================================================

/// Files for every launched target, and a lock whose digests are theirs:
/// authored from what the footprint reads, never from its own output.
struct Footprints {
    _root: tempfile::TempDir,
    /// The directory every target's files live under.
    dir: PathBuf,
    /// Bytes written beside each archive row's executable, in its root.
    archive_extra: std::collections::BTreeMap<TargetKind, u64>,
    lock: SourceLock,
    bins: std::collections::BTreeMap<TargetKind, PathBuf>,
    runtimes: std::collections::BTreeMap<TargetKind, PathBuf>,
    /// Bytes written under each runtime tree.
    tree: std::collections::BTreeMap<TargetKind, u64>,
    /// Bytes of each checked member beside a tree pin's executable.
    members: std::collections::BTreeMap<TargetKind, u64>,
}

fn footprints() -> Footprints {
    let root = fixture();
    let p = p4(draft());
    let mut l = lock5();
    let mut out = Footprints {
        lock: lock_from(l.clone()).unwrap(),
        dir: root.path().to_path_buf(),
        archive_extra: Default::default(),
        _root: fixture(),
        bins: Default::default(),
        runtimes: Default::default(),
        tree: Default::default(),
        members: Default::default(),
    };
    for t in p.launch_order(0) {
        let dir = root.path().join(t.name());
        std::fs::create_dir_all(&dir).unwrap();
        let content = format!("executable of {t}");
        if t == TargetKind::Swarmail {
            let bin = dir.join("executable");
            std::fs::write(&bin, &content).unwrap();
            out.bins.insert(t, bin);
            continue;
        }
        let pin = pin_of(&mut l, t.name());
        // An archive's executable sits at its pinned member path inside the
        // extraction root; any other row's is a file of its own.
        let bin = if pin["artifact"]["kind"] == "archive" {
            let member = pin["artifact"]["member"].as_str().unwrap();
            let bin = dir.join(member);
            std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
            std::fs::create_dir_all(dir.join("docs")).unwrap();
            std::fs::create_dir_all(dir.join("lib/deep")).unwrap();
            std::fs::write(dir.join("docs/readme.txt"), vec![2u8; 500]).unwrap();
            std::fs::write(dir.join("lib/deep/data.bin"), vec![3u8; 700]).unwrap();
            std::os::unix::fs::symlink("/etc/hostname", dir.join("link")).unwrap();
            out.archive_extra.insert(t, 1200);
            bin
        } else {
            dir.join("executable")
        };
        std::fs::write(&bin, &content).unwrap();
        out.bins.insert(t, bin);
        pin["artifact"]["executable_sha256"] = json!(targets::sha256_hex(content.as_bytes()));
        let mut member_bytes = 0;
        if let Some(ms) = pin["artifact"]["checked_members"].as_array_mut() {
            for m in ms {
                let path = dir.join(m["path"].as_str().unwrap());
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                let body = format!("member {}", m["path"]);
                std::fs::write(&path, &body).unwrap();
                m["sha256"] = json!(targets::sha256_hex(body.as_bytes()));
                member_bytes += body.len() as u64;
            }
            out.members.insert(t, member_bytes);
        }
        if pin["runtime"].is_object() {
            let rt = root.path().join(format!("runtime-{t}"));
            let mut written = 0;
            for m in pin["runtime"]["checked_members"].as_array_mut().unwrap() {
                let path = rt.join(m["path"].as_str().unwrap());
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                let body = format!("runtime member {}", m["path"]);
                std::fs::write(&path, &body).unwrap();
                m["sha256"] = json!(targets::sha256_hex(body.as_bytes()));
                written += body.len() as u64;
            }
            // Files the lock does not name still count toward the tree.
            std::fs::create_dir_all(rt.join("lib/extra")).unwrap();
            std::fs::write(rt.join("lib/extra/data.bin"), vec![1u8; 1000]).unwrap();
            std::os::unix::fs::symlink("/etc/hostname", rt.join("link")).unwrap();
            out.tree.insert(t, written + 1000);
            out.runtimes.insert(t, rt);
        }
    }
    out.lock = lock_from(l).unwrap();
    out._root = root;
    out
}

fn run_footprint(f: &Footprints) -> Result<Value, String> {
    resources::footprint(
        &p4(draft()),
        &f.lock,
        Path::new("docs/evals"),
        &f.bins,
        &f.runtimes,
    )
}

fn entry<'a>(v: &'a Value, t: &str) -> &'a Value {
    v["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["target"] == t)
        .unwrap_or_else(|| panic!("no footprint for {t}"))
}

#[test]
fn footprint_reads_archive_layer_and_candidate_sizes() {
    let f = footprints();
    let v = run_footprint(&f).expect("a complete fixture");
    assert_eq!(
        v["targets"].as_array().unwrap().len(),
        8,
        "the eight launched targets"
    );
    // The candidate: the release binary itself.
    let own = std::fs::metadata(&f.bins[&TargetKind::Swarmail])
        .unwrap()
        .len();
    assert_eq!(entry(&v, "swarmail")["download_bytes"], own);
    assert_eq!(entry(&v, "swarmail")["installed_bytes"], own);
    // An archive: the lock's own pinned size, read from the committed JSON.
    let committed = lock5();
    let size = |t: &str| {
        committed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["target"] == t)
            .unwrap()["pin"]["artifact"]["size"]
            .as_u64()
            .unwrap()
    };
    assert_eq!(entry(&v, "mailpit")["download_bytes"], size("mailpit"));
    assert_eq!(entry(&v, "greenmail")["download_bytes"], size("greenmail"));
    // The image row: the sum of the compressed layers of the captured
    // manifest, summed here from the document itself.
    let manifest: Value = json(
        &std::fs::read("docs/evals/sources/alkemio-mailslurper/alkemio-manifest.json").unwrap(),
    );
    let layers = manifest["layers"].as_array().unwrap();
    let sum: u64 = layers.iter().map(|l| l["size"].as_u64().unwrap()).sum();
    assert!(layers.len() > 1 && sum > 0);
    assert_eq!(entry(&v, "alkemio-mailslurper")["download_bytes"], sum);
}

#[test]
fn footprint_installed_bytes_include_the_runtime_tree() {
    let f = footprints();
    let v = run_footprint(&f).unwrap();
    let exe = |t: TargetKind| std::fs::metadata(&f.bins[&t]).unwrap().len();
    for t in [TargetKind::Greenmail, TargetKind::AlkemioMailslurper] {
        let e = entry(&v, t.name());
        assert_eq!(
            e["installed_bytes"],
            exe(t) + f.tree[&t],
            "{t}: executable plus runtime tree"
        );
        // The files are counted too: the executable, the named members and
        // the one extra file; a symlink is not followed.
        assert!(e["installed_files"].as_u64().unwrap() >= 3, "{e}");
    }
    // A tree pin's checked members beside its executable count too.
    for (t, bytes) in &f.members {
        let e = entry(&v, t.name());
        let around = f.archive_extra.get(t).copied().unwrap_or(0);
        assert_eq!(e["installed_bytes"], exe(*t) + bytes + around, "{t}");
    }
    // A plain executable is itself.
    assert_eq!(
        entry(&v, "mailhog")["installed_bytes"],
        exe(TargetKind::Mailhog)
    );
}

/// The targets whose lock artifact is an archive: Mailpit, smtp4dev and
/// Inbucket among the launched ones.
fn archive_targets() -> Vec<&'static str> {
    let l = lock5();
    l["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["pin"]["artifact"]["kind"] == "archive")
        .map(|e| e["target"].as_str().unwrap())
        .filter(|t| *t != "mailslurper")
        .map(|t| &*Box::leak(t.to_owned().into_boxed_str()))
        .collect()
}

#[test]
fn footprint_never_publishes_an_executable_alone_as_installed() {
    let f = footprints();
    let v = run_footprint(&f).unwrap();
    let archives = archive_targets();
    assert_eq!(archives, ["mailpit", "smtp4dev", "inbucket"]);
    for e in v["targets"].as_array().unwrap() {
        let t = e["target"].as_str().unwrap();
        let exe = std::fs::metadata(&f.bins[&TargetKind::parse(t).unwrap()])
            .unwrap()
            .len();
        if archives.contains(&t) {
            // The extracted archive: more than the executable it holds.
            assert!(e["installed_bytes"].as_u64().unwrap() > exe, "{t}");
        } else {
            // A row whose artifact is the executable keeps its sum.
            assert!(e["installed_bytes"].as_u64().is_some(), "{t}: {e}");
        }
    }
}

#[test]
fn footprint_labels_an_unpinned_runtime_and_never_counts_it_as_zero() {
    let f = footprints();
    let v = run_footprint(&f).unwrap();
    for (t, what) in [
        ("mailslurper-source-build", "host glibc"),
        ("smtp4dev", "host libraries"),
    ] {
        let e = entry(&v, t);
        let label = e["unpinned_runtime"]
            .as_str()
            .expect("a label, not a number");
        assert!(
            label.starts_with("unpinned, not counted") && label.contains(what),
            "{label}"
        );
        assert!(
            e.get("runtime_bytes").is_none(),
            "{t}: an unpinned runtime has no byte count"
        );
        // smtp4dev's .NET runtime is inside its archive, which is walked
        // whole: it has a size, and the label above stands beside it.
        assert!(e["installed_bytes"].as_u64().unwrap() > 0, "{e}");
    }
    // A target whose runtime is pinned or absent carries no such label.
    for t in ["mailpit", "greenmail", "swarmail"] {
        assert!(entry(&v, t).get("unpinned_runtime").is_none(), "{t}");
    }
}

#[test]
fn footprint_blocks_the_does_not_start_row() {
    let f = footprints();
    let v = run_footprint(&f).unwrap();
    assert!(
        v["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["target"] != "mailslurper")
    );
    let [b] = v["blocked"].as_array().unwrap().as_slice() else {
        panic!("one blocked row")
    };
    assert_eq!(b["target"], "mailslurper");
    assert!(b["result"].as_str().unwrap().starts_with("does not start"));
    assert!(
        b.as_object()
            .unwrap()
            .keys()
            .all(|k| k == "target" || k == "result"),
        "no number: {b}"
    );
}

#[test]
fn footprint_refuses_a_changed_executable() {
    let f = footprints();
    std::fs::write(&f.bins[&TargetKind::Mailpit], "replaced bytes").unwrap();
    let w = run_footprint(&f).unwrap_err();
    assert!(
        w.contains("mailpit") && w.contains("not the pinned bytes"),
        "{w}"
    );
    // A changed runtime member refuses too, and a missing --bin is named.
    let f = footprints();
    let java = f.runtimes[&TargetKind::Greenmail].join("bin/java");
    std::fs::write(java, "tampered").unwrap();
    assert!(run_footprint(&f).unwrap_err().contains("checked member"));
    let mut f = footprints();
    f.bins.remove(&TargetKind::Inbucket);
    assert!(
        run_footprint(&f)
            .unwrap_err()
            .contains("--bin inbucket=PATH is required")
    );
}

// ---- S1 --------------------------------------------------------------------

/// Every regular file under `root` by hand, as `sha256sum` prints it, sorted
/// by the path's bytes, hashed: the expected digest, built from V18's format
/// and never from the function under test.
fn listing_by_hand(root: &Path) -> (u64, u64, String) {
    fn files(dir: &Path, rel: &str, out: &mut Vec<(String, PathBuf)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let rel = format!("{rel}/{}", e.file_name().to_str().unwrap());
            let t = e.file_type().unwrap();
            if t.is_dir() {
                files(&e.path(), &rel, out);
            } else if t.is_file() {
                out.push((rel, e.path()));
            }
        }
    }
    let mut all = Vec::new();
    files(root, "", &mut all);
    all.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut text = String::new();
    let mut bytes = 0;
    for (rel, path) in &all {
        let body = std::fs::read(path).unwrap();
        bytes += body.len() as u64;
        text.push_str(&format!("{}  .{rel}\n", targets::sha256_hex(&body)));
    }
    (
        bytes,
        all.len() as u64,
        targets::sha256_hex(text.as_bytes()),
    )
}

#[test]
fn a_tree_walk_counts_each_regular_file_once_and_no_link() {
    let d = fixture();
    let r = d.path();
    std::fs::create_dir_all(r.join("a/b")).unwrap();
    std::fs::create_dir_all(r.join("empty")).unwrap();
    std::fs::write(r.join("top"), vec![1u8; 10]).unwrap();
    std::fs::write(r.join("a/mid"), vec![1u8; 20]).unwrap();
    std::fs::write(r.join("a/b/deep"), vec![1u8; 30]).unwrap();
    std::os::unix::fs::symlink(r.join("top"), r.join("link-to-file")).unwrap();
    std::os::unix::fs::symlink(r.join("a"), r.join("link-to-dir")).unwrap();
    std::os::unix::fs::symlink("/nonexistent/target", r.join("dangling")).unwrap();
    let w = resources::walk_tree(r).unwrap();
    assert_eq!((w.bytes, w.files), (60, 3));
    // A root reached through a link is walked as the directory it is.
    let l = fixture();
    std::os::unix::fs::symlink(r, l.path().join("via")).unwrap();
    assert_eq!(resources::walk_tree(&l.path().join("via")).unwrap(), w);
    // A file or a missing path is not a tree.
    assert!(
        resources::walk_tree(&r.join("top"))
            .unwrap_err()
            .contains("not a directory")
    );
    assert!(resources::walk_tree(&r.join("missing")).is_err());
}

#[test]
fn a_tree_digest_equals_the_listing_the_test_builds_by_hand() {
    let d = fixture();
    let r = d.path();
    std::fs::create_dir_all(r.join("lib/x")).unwrap();
    std::fs::create_dir_all(r.join("bin")).unwrap();
    std::fs::write(r.join("lib/x/b.so"), b"second").unwrap();
    std::fs::write(r.join("lib/a.so"), b"first").unwrap();
    std::fs::write(r.join("bin/run"), b"#!run").unwrap();
    std::fs::write(r.join("B upper"), b"upper-case sorts before lower").unwrap();
    std::fs::write(r.join("a lower"), b"lower").unwrap();
    std::fs::write(r.join("z"), b"").unwrap();
    let (bytes, files, digest) = listing_by_hand(r);
    let w = resources::walk_tree(r).unwrap();
    assert_eq!(
        (w.bytes, w.files, w.sha256.as_str()),
        (bytes, files, digest.as_str())
    );
    // The same bytes through V18's own command line.
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(
            "cd \"$1\" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum",
        )
        .arg("sh")
        .arg(r)
        .output()
        .expect("sh runs");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout)
            .unwrap()
            .split_whitespace()
            .next(),
        Some(digest.as_str()),
        "the documented command prints the same digest"
    );
}

#[test]
fn a_tree_digest_changes_when_a_file_is_added_removed_or_changed() {
    let d = fixture();
    let r = d.path();
    std::fs::write(r.join("one"), b"1").unwrap();
    std::fs::write(r.join("two"), b"2").unwrap();
    let base = resources::walk_tree(r).unwrap();
    std::fs::write(r.join("three"), b"3").unwrap();
    let added = resources::walk_tree(r).unwrap();
    assert_ne!(added.sha256, base.sha256);
    std::fs::remove_file(r.join("three")).unwrap();
    assert_eq!(resources::walk_tree(r).unwrap(), base, "removed again");
    std::fs::write(r.join("two"), b"X").unwrap();
    let changed = resources::walk_tree(r).unwrap();
    assert_eq!((changed.bytes, changed.files), (base.bytes, base.files));
    assert_ne!(changed.sha256, base.sha256, "same size, other bytes");
    // A rename is a change of the listing too.
    std::fs::rename(r.join("two"), r.join("2")).unwrap();
    assert_ne!(resources::walk_tree(r).unwrap().sha256, changed.sha256);
}

#[test]
fn a_tree_walk_refuses_a_name_sha256sum_would_escape() {
    for name in ["new\nline", "back\\slash"] {
        let d = fixture();
        std::fs::write(d.path().join(name), b"x").unwrap();
        let e = resources::walk_tree(d.path()).unwrap_err();
        assert!(e.contains("newline or a backslash"), "{e}");
        // The same name on a directory on the way to a file.
        let d = fixture();
        std::fs::create_dir(d.path().join(name)).unwrap();
        std::fs::write(d.path().join(name).join("f"), b"x").unwrap();
        assert!(resources::walk_tree(d.path()).is_err());
    }
    // A link with such a name is not listed, so it is not refused.
    let d = fixture();
    std::fs::write(d.path().join("ok"), b"x").unwrap();
    std::os::unix::fs::symlink("ok", d.path().join("new\nline")).unwrap();
    assert_eq!(resources::walk_tree(d.path()).unwrap().files, 1);
}

#[cfg(target_os = "linux")]
#[test]
fn a_tree_walk_refuses_a_name_that_is_not_utf8() {
    use std::os::unix::ffi::OsStrExt;
    let d = fixture();
    let name = std::ffi::OsStr::from_bytes(b"bad\xff\xfename");
    std::fs::write(d.path().join(name), b"x").unwrap();
    let e = resources::walk_tree(d.path()).unwrap_err();
    assert!(e.contains("not UTF-8"), "{e}");
}

fn tree_row(e: &Value, kind: &str) -> Value {
    let [t] = e["installed_trees"].as_array().unwrap().as_slice() else {
        panic!("one walked tree: {e}")
    };
    assert_eq!(t["kind"], kind);
    t.clone()
}

#[test]
fn footprint_sizes_an_archive_row_from_its_extraction_root() {
    let f = footprints();
    let bin = f.bins[&TargetKind::Mailpit].clone();
    let root = bin.parent().unwrap().to_path_buf();
    // A root the test builds whole: the member, two more files, a nested
    // directory and a link.
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::create_dir_all(root.join("share/deep")).unwrap();
    std::fs::write(&bin, "executable of mailpit").unwrap();
    std::fs::write(root.join("share/a.txt"), vec![0u8; 11]).unwrap();
    std::fs::write(root.join("share/deep/b.bin"), vec![0u8; 13]).unwrap();
    std::os::unix::fs::symlink(&bin, root.join("alias")).unwrap();
    let v = run_footprint(&f).unwrap();
    let e = entry(&v, "mailpit");
    let exe = "executable of mailpit".len() as u64;
    assert_eq!(e["installed_bytes"], exe + 11 + 13, "{e}");
    assert_eq!(e["installed_files"], 3, "the executable is counted once");
    // The archive's download is a different number and stays.
    assert!(e["download_bytes"].as_u64().unwrap() > 0);
    // The rule that applied is stated as fixed text.
    assert!(
        e["installed_basis"]
            .as_str()
            .unwrap()
            .contains("extraction root"),
        "{e}"
    );
}

#[test]
fn footprint_refuses_an_archive_bin_reached_through_a_link() {
    // The member is a link to the pinned bytes outside the root: the digest
    // holds, but the walk would not count the executable.
    let f = footprints();
    let bin = f.bins[&TargetKind::Mailpit].clone();
    let root = bin.parent().unwrap().to_path_buf();
    let outside = root.parent().unwrap().join("mailpit-real");
    std::fs::rename(&bin, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &bin).unwrap();
    let w = run_footprint(&f).unwrap_err();
    assert!(
        w.contains("mailpit") && w.contains("is not the member mailpit of its extraction root"),
        "{w}"
    );
}

#[test]
fn footprint_lists_each_walked_tree_with_its_kind_and_digest() {
    let f = footprints();
    let v = run_footprint(&f).unwrap();
    // An archive row: its extraction root.
    let root = f.bins[&TargetKind::Smtp4dev]
        .parent()
        .unwrap()
        .to_path_buf();
    let (bytes, files, digest) = listing_by_hand(&root);
    let t = tree_row(entry(&v, "smtp4dev"), "archive");
    assert_eq!(
        (
            t["bytes"].as_u64(),
            t["files"].as_u64(),
            t["sha256"].as_str()
        ),
        (Some(bytes), Some(files), Some(digest.as_str()))
    );
    // An executable row with a runtime: that runtime tree.
    let (bytes, files, digest) = listing_by_hand(&f.runtimes[&TargetKind::Greenmail]);
    let t = tree_row(entry(&v, "greenmail"), "runtime");
    assert_eq!(
        (
            t["bytes"].as_u64(),
            t["files"].as_u64(),
            t["sha256"].as_str()
        ),
        (Some(bytes), Some(files), Some(digest.as_str()))
    );
    // A row that walks none has an empty list.
    for t in ["swarmail", "mailhog", "mailslurper-source-build"] {
        assert_eq!(entry(&v, t)["installed_trees"], json!([]), "{t}");
    }
}

#[test]
fn footprint_refuses_a_bin_that_does_not_end_with_the_pinned_member() {
    let mut f = footprints();
    let bin = f.bins[&TargetKind::Mailpit].clone();
    let other = bin.with_file_name("not-the-member");
    std::fs::rename(&bin, &other).unwrap();
    f.bins.insert(TargetKind::Mailpit, other);
    let w = run_footprint(&f).unwrap_err();
    assert!(
        w.contains("mailpit") && w.contains("pinned member mailpit"),
        "{w}"
    );
    // The pure rule, for a nested member and a bare file name.
    assert_eq!(
        resources::extraction_root(Path::new("/x/y/a/b/exe"), "a/b/exe").unwrap(),
        Path::new("/x/y")
    );
    assert_eq!(
        resources::extraction_root(Path::new("exe"), "exe").unwrap(),
        Path::new(".")
    );
    assert!(resources::extraction_root(Path::new("/x/bexe"), "exe").is_err());
}

#[test]
fn footprint_never_publishes_null_zero_owed_or_a_host_path() {
    fn walk(v: &Value, dir: &str, at: &str) {
        match v {
            Value::Object(m) => {
                for (k, v) in m {
                    assert_ne!(k, "executable_bytes", "{at}");
                    if k == "installed_bytes" || k == "installed_files" {
                        assert!(v.as_u64().unwrap() > 0, "{at}.{k}: {v}");
                    }
                    walk(v, dir, &format!("{at}.{k}"));
                }
            }
            Value::Array(a) => {
                for (i, v) in a.iter().enumerate() {
                    walk(v, dir, &format!("{at}[{i}]"));
                }
            }
            Value::String(s) => {
                assert!(!s.contains("owed"), "{at}: {s}");
                assert!(!s.contains(dir), "{at}: a host path: {s}");
            }
            _ => {}
        }
    }
    let f = footprints();
    let v = run_footprint(&f).unwrap();
    // Every artifact kind is in the document: an archive, an executable
    // with a runtime, an executable alone, the candidate and the image row.
    for t in [
        "mailpit",
        "greenmail",
        "mailhog",
        "swarmail",
        "alkemio-mailslurper",
    ] {
        assert!(entry(&v, t)["installed_bytes"].as_u64().unwrap() > 0, "{t}");
    }
    walk(&v, f.dir.to_str().unwrap(), "$");
}

#[test]
fn footprint_refuses_an_empty_or_non_regular_candidate() {
    // V18: every launched row's installed size is positive, the candidate's
    // too; a zero-length or directory --bin refuses and prints no document.
    let mut f = footprints();
    let empty = f.dir.join("empty-swarmail");
    std::fs::write(&empty, "").unwrap();
    f.bins.insert(TargetKind::Swarmail, empty);
    let w = run_footprint(&f).unwrap_err();
    assert!(
        w.contains("swarmail") && w.contains("nonempty regular file"),
        "{w}"
    );

    let mut f = footprints();
    let dir = f.dir.join("swarmail-dir");
    std::fs::create_dir(&dir).unwrap();
    f.bins.insert(TargetKind::Swarmail, dir);
    let w = run_footprint(&f).unwrap_err();
    assert!(w.contains("nonempty regular file"), "{w}");

    let mut f = footprints();
    f.bins
        .insert(TargetKind::Swarmail, f.dir.join("no-such-swarmail"));
    let w = run_footprint(&f).unwrap_err();
    assert!(
        w.contains("swarmail") && w.contains("no-such-swarmail"),
        "{w}"
    );
}

#[test]
fn footprint_refuses_two_targets_that_share_a_tree() {
    // Two separate directories pass.
    run_footprint(&footprints()).expect("separate trees");

    // Another competitor's executable inside an archive root.
    let mut f = footprints();
    let root = f.bins[&TargetKind::Mailpit].parent().unwrap().to_path_buf();
    let inside = root.join("mailhog-inside");
    std::fs::write(&inside, "executable of mailhog").unwrap();
    f.bins.insert(TargetKind::Mailhog, inside);
    let w = run_footprint(&f).unwrap_err();
    assert!(
        w.contains("mailpit") && w.contains("mailhog") && w.contains("share a tree"),
        "{w}"
    );

    // The candidate inside an archive root.
    let mut f = footprints();
    let root = f.bins[&TargetKind::Inbucket]
        .parent()
        .unwrap()
        .to_path_buf();
    let inside = root.join("swarmail");
    std::fs::write(&inside, "the candidate").unwrap();
    f.bins.insert(TargetKind::Swarmail, inside);
    let w = run_footprint(&f).unwrap_err();
    assert!(w.contains("inbucket") && w.contains("swarmail"), "{w}");

    // A runtime tree inside an archive root.
    let mut f = footprints();
    let root = f.bins[&TargetKind::Mailpit].parent().unwrap().to_path_buf();
    let moved = root.join("jre");
    std::fs::rename(&f.runtimes[&TargetKind::Greenmail], &moved).unwrap();
    f.runtimes.insert(TargetKind::Greenmail, moved);
    let w = run_footprint(&f).unwrap_err();
    assert!(w.contains("mailpit") && w.contains("greenmail"), "{w}");

    // An archive root inside a runtime tree.
    let mut f = footprints();
    let rt = f.runtimes[&TargetKind::Greenmail].clone();
    let old = f.bins[&TargetKind::Mailpit].parent().unwrap().to_path_buf();
    let moved = rt.join("mailpit-root");
    std::fs::rename(&old, &moved).unwrap();
    f.bins.insert(TargetKind::Mailpit, moved.join("mailpit"));
    let w = run_footprint(&f).unwrap_err();
    assert!(w.contains("mailpit") && w.contains("greenmail"), "{w}");

    // The same directory reached through a symbolic link.
    let mut f = footprints();
    let mailpit_root = f.bins[&TargetKind::Mailpit].parent().unwrap().to_path_buf();
    let via = f.dir.join("via-link");
    std::os::unix::fs::symlink(&mailpit_root, &via).unwrap();
    let body = std::fs::read(&f.bins[&TargetKind::Inbucket]).unwrap();
    std::fs::write(mailpit_root.join("inbucket"), body).unwrap();
    f.bins.insert(TargetKind::Inbucket, via.join("inbucket"));
    let w = run_footprint(&f).unwrap_err();
    assert!(w.contains("mailpit") && w.contains("inbucket"), "{w}");
}

// ---- T5.2 ------------------------------------------------------------------

#[test]
fn hold_refuses_missing_flags_and_protocol_3() {
    let args = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let full = args(&[
        "--protocol",
        "p.json",
        "--sources",
        "s.json",
        "--bin",
        "mailpit=/x/mailpit",
        "--scratch",
        "/s",
        "--samples",
        "/f",
    ]);
    let ok = runner::hold_args(&full).unwrap();
    assert_eq!(
        (ok.target, ok.runtime.is_none()),
        (TargetKind::Mailpit, true)
    );
    assert_eq!(ok.samples, PathBuf::from("/f"));
    for (drop, expect) in [
        ("--protocol", "--protocol is required"),
        ("--sources", "--sources is required"),
        ("--scratch", "--scratch is required"),
        ("--samples", "--samples is required"),
        ("--bin", "exactly one --bin"),
    ] {
        let mut a = full.clone();
        let at = a.iter().position(|x| x == drop).unwrap();
        a.drain(at..at + 2);
        let w = runner::hold_args(&a).unwrap_err();
        assert!(w.contains(expect), "{drop}: {w}");
    }
    let mut two = full.clone();
    two.extend(args(&["--bin", "mailhog=/x/mailhog"]));
    assert!(
        runner::hold_args(&two)
            .unwrap_err()
            .contains("exactly one --bin")
    );
    let mut other = full.clone();
    other.extend(args(&["--runtime", "greenmail=/jre"]));
    assert!(
        runner::hold_args(&other)
            .unwrap_err()
            .contains("must name mailpit")
    );
    let mut unknown = full.clone();
    unknown.extend(args(&["--wat", "1"]));
    assert!(
        runner::hold_args(&unknown)
            .unwrap_err()
            .contains("unknown flag --wat")
    );
    // Protocol 3 has no budget to hold a target under.
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let w = runner::hold_protocol(&p3).unwrap_err();
    assert!(
        w.contains("hold needs protocol 4") && w.contains("protocol 3"),
        "{w}"
    );
    runner::hold_protocol(&p4(draft())).unwrap();
}

#[tokio::test]
async fn hold_refuses_bytes_on_stdin_by_name() {
    // Without a scope the launch has no leaf, so this proves the teardown, the
    // receipts and the refusal on any host; the budget is real proof 4.
    let spec = swarmail_spec(targets::swarmail_template(0));
    let root = tempfile::tempdir().unwrap();
    let clock = wire::Clock(Instant::now());
    let mut printed = Vec::new();
    let receipt = runner::hold(
        &spec,
        (
            &root.path().join("hold"),
            &root.path().join("samples.jsonl"),
        ),
        &limits(),
        &clock,
        Duration::from_millis(100),
        &b"hello"[..],
        &mut printed,
        std::future::pending(),
    )
    .await
    .expect("it launches and tears down");
    assert_eq!(receipt.stdin_bytes, 5);
    assert!(receipt.cleanup.proved(), "teardown ran before the refusal");
    let w = runner::hold_verdict(&receipt).unwrap_err();
    assert!(w.contains("stdin carried 5 byte(s) before EOF"), "{w}");
    // Two lines: the endpoints, then the receipts.
    let text = String::from_utf8(printed).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    let up: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(up["target"], "swarmail");
    assert!(up["smtp"].as_str().unwrap().starts_with("127.0.0.1:"));
    let done: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(done["stdin_bytes"], 5);
    assert_eq!(done["cleanup"]["reaped"], true);
    // Silence, then EOF, is the clean end.
    let mut printed = Vec::new();
    let receipt = runner::hold(
        &spec,
        (
            &root.path().join("hold2"),
            &root.path().join("samples2.jsonl"),
        ),
        &limits(),
        &clock,
        Duration::from_millis(100),
        &b""[..],
        &mut printed,
        std::future::pending(),
    )
    .await
    .unwrap();
    runner::hold_verdict(&receipt).expect("EOF with no byte exits cleanly");
    // An interrupt ends it too, with the same teardown.
    let (_keep, reader) = tokio::io::duplex(8);
    let mut printed = Vec::new();
    let receipt = runner::hold(
        &spec,
        (
            &root.path().join("hold3"),
            &root.path().join("samples3.jsonl"),
        ),
        &limits(),
        &clock,
        Duration::from_millis(100),
        reader,
        &mut printed,
        async {},
    )
    .await
    .unwrap();
    assert!(receipt.cleanup.proved());
    // An existing samples file is never overwritten.
    let w = runner::hold(
        &spec,
        (
            &root.path().join("hold4"),
            &root.path().join("samples.jsonl"),
        ),
        &limits(),
        &clock,
        Duration::from_millis(100),
        &b""[..],
        &mut Vec::new(),
        std::future::pending(),
    )
    .await
    .unwrap_err();
    assert!(w.contains("--samples"), "{w}");
}

/// A writer the test can read while `hold` still holds the other end.
#[derive(Clone, Default)]
struct HeldOut(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for HeldOut {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl HeldOut {
    /// The first `n` complete lines, once they have been printed.
    async fn lines(&self, n: usize) -> Vec<String> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let text = String::from_utf8(self.0.lock().unwrap().clone()).unwrap();
            let lines: Vec<String> = text.lines().map(str::to_owned).collect();
            if lines.len() >= n && text.ends_with('\n') {
                return lines;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "no line {n}: {text:?}"
            );
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

#[tokio::test]
async fn hold_prints_each_endpoint_as_the_address_it_listens_on() {
    // A launch without a leaf, held open by a pipe whose far end stays up.
    let spec = swarmail_spec(targets::swarmail_template(0));
    let root = tempfile::tempdir().unwrap();
    let (keep, reader) = tokio::io::duplex(8);
    let out = HeldOut::default();
    let (o, r) = (out.clone(), root.path().to_owned());
    let held = tokio::spawn(async move {
        let mut o = o;
        runner::hold(
            &spec,
            (&r.join("hold"), &r.join("samples.jsonl")),
            &limits(),
            &wire::Clock(Instant::now()),
            Duration::from_millis(100),
            reader,
            &mut o,
            std::future::pending(),
        )
        .await
    });
    let first: Value = serde_json::from_str(&out.lines(1).await[0]).unwrap();
    for key in ["smtp", "http"] {
        let addr: std::net::SocketAddr = first[key]
            .as_str()
            .unwrap_or_else(|| panic!("{key} is an address: {first}"))
            .parse()
            .unwrap_or_else(|e| panic!("{key} is host:port: {e}: {first}"));
        tokio::net::TcpStream::connect(addr)
            .await
            .unwrap_or_else(|e| panic!("{key} {addr} accepts a connect: {e}"));
    }
    drop(keep);
    let receipt = held.await.unwrap().expect("EOF ends the hold");
    runner::hold_verdict(&receipt).unwrap();
    // A POP3 listener prints its address, not a word.
    let pop3: std::net::SocketAddr = "127.0.0.1:2110".parse().unwrap();
    let http: std::net::SocketAddr = "127.0.0.1:8025".parse().unwrap();
    let owned = targets::Owned::fixture(TargetKind::Greenmail, http, Some(pop3), &limits());
    let e = runner::hold_endpoints(&owned);
    assert_eq!(e.pop3.as_deref(), Some("127.0.0.1:2110"));
    assert_eq!(e.http.as_deref(), Some("127.0.0.1:8025"));
    let line = serde_json::to_value(&e).unwrap();
    assert_eq!(line["pop3"], "127.0.0.1:2110");
    // And a target with no POP3 listener prints none.
    let owned = targets::Owned::fixture(TargetKind::Mailpit, http, None, &limits());
    assert!(runner::hold_endpoints(&owned).pop3.is_none());
}

#[tokio::test]
async fn a_hold_ended_by_its_interrupt_tears_down_and_prints_a_proved_receipt() {
    let spec = swarmail_spec(targets::swarmail_template(0));
    let root = tempfile::tempdir().unwrap();
    // The pipe stays open and silent: only the interrupt can end it, and it
    // fires after the endpoints line has been printed.
    let (_keep, reader) = tokio::io::duplex(8);
    let (fire, interrupt) = tokio::sync::oneshot::channel::<()>();
    let out = HeldOut::default();
    let (o, r) = (out.clone(), root.path().to_owned());
    let held = tokio::spawn(async move {
        let mut o = o;
        runner::hold(
            &spec,
            (&r.join("hold"), &r.join("samples.jsonl")),
            &limits(),
            &wire::Clock(Instant::now()),
            Duration::from_millis(100),
            reader,
            &mut o,
            async {
                let _ = interrupt.await;
            },
        )
        .await
    });
    let lines = out.lines(1).await;
    assert_eq!(
        lines.len(),
        1,
        "the receipt is not printed before the interrupt"
    );
    assert!(!held.is_finished(), "it holds until it is interrupted");
    fire.send(()).unwrap();
    let receipt = held.await.unwrap().expect("the interrupt ends the hold");
    assert!(receipt.cleanup.proved(), "{:?}", receipt.cleanup);
    assert_eq!(receipt.stdin_bytes, 0);
    runner::hold_verdict(&receipt).expect("an interrupt is a clean end");
    let lines = out.lines(2).await;
    let done: Value = serde_json::from_str(&lines[1]).unwrap();
    assert_eq!(done["cleanup"]["reaped"], true);
    assert_eq!(done["cleanup"]["scratch_removed"], true);
    assert!(
        done["cleanup"].get("cgroup_removed").is_none(),
        "no leaf, so no leaf receipt: {done}"
    );
}

// ---- T5.3 ------------------------------------------------------------------

/// A protocol whose plan is eight small chunks: one repetition of one tiny
/// cost profile, so a whole run is cheap to author and to admit.
fn adm_protocol() -> (Protocol, Vec<u8>) {
    adm_protocol_n(1)
}

/// The admission fixture's protocol over `reps` repetitions.
fn adm_protocol_n(reps: u32) -> (Protocol, Vec<u8>) {
    let mut v = draft();
    let mut prof = v["throughput"][1].clone();
    prof["repetitions"] = json!(reps);
    prof["messages"] = json!(4);
    prof["warmup_messages"] = json!(0);
    prof["sessions"] = json!(2);
    v["throughput"] = json!([prof]);
    let bytes = serde_json::to_vec(&v).unwrap();
    (p4(v), bytes)
}

fn adm_host(nproc: u32) -> records::HostFacts {
    records::HostFacts {
        kernel: "Linux 7.0.0".into(),
        systemd: "systemd 259".into(),
        cpu_model: "Test CPU".into(),
        nproc,
        siblings: vec![vec![2, 66], vec![3, 67]],
        cpu_layout: resources::Layout {
            target_cpus: vec![2, 3],
            idle_siblings: vec![66, 67],
            client_cpus: vec![4, 5, 6, 7],
            siblings_outside_set: vec![],
        },
        governor: "performance".into(),
    }
}

type Edit = fn(&mut Value);
type LedgerEdit = fn(&mut report::LedgerEntry);

#[derive(Default)]
struct Build {
    skip: Option<&'static str>,
    extra_chunk: bool,
    unsealed: Option<&'static str>,
    other_run: Option<&'static str>,
    other_host: Option<&'static str>,
    dirty: bool,
    /// One chunk, by coordinate, whose candidate tree was dirty.
    dirty_chunk: Option<&'static str>,
    /// Repetitions of the protocol (0 is 1).
    reps: u32,
    /// One chunk, by coordinate, whose launch ran a different executable.
    other_digest: Option<&'static str>,
    no_footprint: bool,
    /// An edit of the footprint document the run carries.
    footprint: Option<Edit>,
    /// One chunk, by coordinate, whose instant came before it could launch
    /// anything: no receipts, every row `unstarted`, no window opened.
    no_launch: Option<&'static str>,
    edit: Option<(&'static str, Edit)>,
    ledger: Option<(&'static str, LedgerEdit)>,
}

struct Fixture {
    root: tempfile::TempDir,
    p: Protocol,
    pbytes: Vec<u8>,
    lock: SourceLock,
    sbytes: Vec<u8>,
}

impl Fixture {
    fn dir(&self) -> PathBuf {
        self.root.path().join("run")
    }

    fn admit(&self, seal: bool) -> Result<report::RunAdmitted, String> {
        let bins = Default::default();
        let a = report::Admission {
            protocol: &self.p,
            protocol_bytes: &self.pbytes,
            lock: &self.lock,
            lock_bytes: &self.sbytes,
            bins: &bins,
        };
        report::admit_v4(&self.dir(), &a, seal)
    }
}

fn build_run(b: Build) -> Fixture {
    let root = fixture();
    let (p, pbytes) = adm_protocol_n(b.reps.max(1));
    let sbytes = serde_json::to_vec(&lock5()).unwrap();
    let lock = SourceLock::parse(&sbytes).unwrap();
    let bins: Vec<String> = p
        .launch_order(0)
        .map(|t| format!("{t}=/never/{t}"))
        .collect();
    let runtimes = vec![
        "greenmail=/never/jre".to_owned(),
        "alkemio-mailslurper=/never/musl".to_owned(),
    ];
    let specs = runner::launch_specs(&p, &lock, &bins, &runtimes).unwrap();
    let run = root.path().join("run");
    std::fs::create_dir_all(run.join("chunks")).unwrap();
    std::fs::create_dir_all(run.join("ledger")).unwrap();
    if !b.no_footprint {
        let mut doc = json!({
            "footprint_version": 1,
            "protocol_id": p.id,
            "targets": p
                .launch_order(0)
                .enumerate()
                .map(|(i, t)| json!({
                    "target": t,
                    "download_bytes": 1000 + i,
                    "installed_bytes": 5000 + i,
                    "installed_files": 3 + i,
                    "installed_basis": "authored for the fixture run",
                    "installed_trees": [],
                }))
                .collect::<Vec<_>>(),
            "blocked": [],
        });
        if let Some(edit) = b.footprint {
            edit(&mut doc);
        }
        std::fs::write(
            run.join("footprint.json"),
            serde_json::to_vec(&doc).unwrap(),
        )
        .unwrap();
    }
    for c in runner::plan_v4(&p) {
        let name = c.name();
        let is = |x: Option<&str>| x == Some(name.as_str());
        let run_id = if is(b.other_run) {
            "run-other"
        } else {
            "run-adm"
        };
        let spec = &specs[&c.target()];
        let layout = adm_host(128).cpu_layout;
        // The leaf the chunk's rows name (`res_rows`), so its receipt is theirs.
        let leaf = format!("target-{:04}", c.repetition() + 1);
        let launch = targets::LaunchFacts {
            target: c.target(),
            executable_sha256: if is(b.other_digest) {
                "cd".repeat(32)
            } else {
                spec.expected_sha256
                    .clone()
                    .unwrap_or_else(|| "ab".repeat(32))
            },
            argv_template: spec.template.clone(),
            ready_ns: 1,
            members: spec
                .extra
                .members
                .iter()
                .map(|(p, s)| (p.to_string_lossy().into_owned(), s.clone()))
                .collect(),
            leaf: Some(leaf.clone()),
            limits: Some(resources::LimitsRead {
                memory_max: "2147483648".into(),
                memory_swap_max: "0".into(),
                memory_oom_group: "1".into(),
                cpu_max: "200000 100000".into(),
                pids_max: "4096".into(),
            }),
            cpu_layout: Some(layout.clone()),
            joined: Some(resources::Joined {
                cgroup: leaf.clone(),
                cpus_allowed: layout.target_cpus.clone(),
            }),
            env_template: Some(spec.extra.env.clone()),
        };
        let mut cleanup = targets::CleanupFacts::unproved(c.target());
        (
            cleanup.reaped,
            cleanup.group_gone,
            cleanup.listeners_closed,
            cleanup.scratch_removed,
        ) = (true, true, true, true);
        (cleanup.cgroup_removed, cleanup.leaf) = (Some(true), Some(leaf.clone()));
        let nothing = is(b.no_launch);
        let life = if nothing {
            runner::Lifecycle {
                launches: vec![],
                cleanups: vec![],
            }
        } else {
            runner::Lifecycle {
                launches: vec![launch],
                cleanups: vec![cleanup],
            }
        };
        let mut m = manifest(&p, run_id);
        m.mode = "collect".into();
        m.protocol_sha256 = targets::sha256_hex(&pbytes);
        m.sources_sha256 = targets::sha256_hex(&sbytes);
        m.candidate_commit = Some("a".repeat(40));
        m.candidate_dirty = Some(b.dirty || is(b.dirty_chunk));
        let (planned, windows) = runner::plan_chunk(&p, &c);
        (m.planned, m.planned_windows, m.chunk) = (planned, Some(windows), Some(name.clone()));
        m.host = Some(adm_host(if is(b.other_host) { 64 } else { 128 }));
        let skipped = is(b.skip);
        if !skipped {
            let dir = run.join("chunks").join(&name);
            let mut rd = records::RunDir::create(&dir, &m).unwrap();
            let mut rep = Rep::new(c.repetition(), (1_000_000, 500_000), 300);
            rep.target = c.target();
            if nothing {
                (rep.unopened, rep.outcome) = (true, records::WindowOutcome::ChunkDeadline);
            }
            let rows = if nothing {
                unstarted_rows(run_id, &rep, "chunk deadline")
            } else {
                msg_rows(run_id, &rep)
            };
            for r in &rows {
                rd.append(r).unwrap();
            }
            let res = res_rows(run_id, &rep);
            for r in &res {
                rd.append_resource(r).unwrap();
            }
            let mut report =
                serde_json::to_value(report::report_v4(&p, &m, &rows, &[&res[..]], &life)).unwrap();
            if let Some((who, edit)) = b.edit
                && who == name
            {
                edit(&mut report);
            }
            rd.write_artifact("report.json", &serde_json::to_vec_pretty(&report).unwrap())
                .unwrap();
            if !is(b.unsealed) {
                rd.finalize(true).unwrap();
            }
        }
        let mut entry = report::LedgerEntry {
            coordinate: name.clone(),
            command: format!("timeout --kill-after=30 560 taskset -c 0-7 systemd-run … {name}"),
            exit: i32::from(is(b.no_launch)),
            wall_ms: 1234,
            load_before: "0.10 0.20 0.30 1/200 99".into(),
            load_after: "0.15 0.20 0.30 1/200 99".into(),
            scope_clean: true,
            driver: "/driver/mail_compare".into(),
        };
        if let Some((who, edit)) = b.ledger
            && who == name
        {
            edit(&mut entry);
        }
        std::fs::write(
            run.join("ledger").join(format!("{name}.json")),
            serde_json::to_vec(&entry).unwrap(),
        )
        .unwrap();
    }
    if b.extra_chunk {
        std::fs::create_dir(run.join("chunks/cost.extra.r0")).unwrap();
    }
    Fixture {
        root,
        p,
        pbytes,
        lock,
        sbytes,
    }
}

/// Rewrite a run-level file the way a tamperer who also fixes the index would.
fn rewrite_and_reindex(dir: &Path, file: &str, edit: impl Fn(&mut Value)) {
    let mut v: Value = json(&std::fs::read(dir.join(file)).unwrap());
    edit(&mut v);
    std::fs::write(dir.join(file), serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    let mut artifacts = std::collections::BTreeMap::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let name = e.file_name().into_string().unwrap();
        if name != "index.json" && e.path().is_file() {
            let bytes = std::fs::read(e.path()).unwrap();
            artifacts.insert(name, (targets::sha256_hex(&bytes), bytes.len() as u64));
        }
    }
    let old: Value = json(&std::fs::read(dir.join("index.json")).unwrap());
    let index = records::Index {
        index_version: records::INDEX_VERSION,
        run_id: old["run_id"].as_str().unwrap().into(),
        artifacts,
    };
    std::fs::write(
        dir.join("index.json"),
        serde_json::to_vec_pretty(&index).unwrap(),
    )
    .unwrap();
}

#[test]
fn a_complete_fixture_run_seals_and_then_verifies() {
    let f = build_run(Build::default());
    let sealed = f.admit(true).expect("a complete run seals");
    assert_eq!((sealed.run_id.as_str(), sealed.chunks), ("run-adm", 8));
    for name in [
        "manifest.json",
        "host.json",
        "chunks.json",
        "launches.json",
        "report.json",
        "report-cost.json",
        "index.json",
    ] {
        assert!(f.dir().join(name).is_file(), "{name}");
    }
    let verified = f.admit(false).expect("a sealed run verifies");
    assert_eq!(verified.files.keys().len(), 6);
    let manifest = &verified.files["manifest.json"];
    assert_eq!(manifest["chunks"].as_array().unwrap().len(), 8);
    assert_eq!(manifest["candidate_dirty"], false);
    assert_eq!(verified.files["chunks.json"].as_array().unwrap().len(), 8);
    assert_eq!(
        verified.files["launches.json"][0]["chunk"],
        "cost.swarmail.r0"
    );
    assert_eq!(verified.files["report.json"]["report_version"], 3);
    assert!(verified.files["report-cost.json"]["headline"]["profile"] == "cost-c8-reused");
    assert_eq!(
        verified.files["report.json"]["cost"]["cells"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    // Verifying before sealing says to seal first.
    let fresh = build_run(Build::default());
    assert!(
        fresh
            .admit(false)
            .unwrap_err()
            .contains("run admit-v4 --seal first")
    );
}

fn refuses(b: Build, expect: &str) {
    let f = build_run(b);
    let w = f.admit(true).expect_err("must refuse");
    assert!(w.contains(expect), "wanted {expect:?} in {w}");
}

#[test]
fn admit_v4_refuses_a_missing_chunk() {
    refuses(
        Build {
            skip: Some("cost.inbucket.r0"),
            ..Default::default()
        },
        "chunk cost.inbucket.r0 is missing",
    );
}

#[test]
fn admit_v4_refuses_an_extra_chunk() {
    refuses(
        Build {
            extra_chunk: true,
            ..Default::default()
        },
        "chunk cost.extra.r0 is not in the protocol's plan",
    );
}

#[test]
fn admit_v4_refuses_an_unsealed_chunk() {
    refuses(
        Build {
            unsealed: Some("cost.mailhog.r0"),
            ..Default::default()
        },
        "chunk cost.mailhog.r0: no completion index",
    );
}

#[test]
fn admit_v4_refuses_two_run_ids() {
    refuses(
        Build {
            other_run: Some("cost.smtp4dev.r0"),
            ..Default::default()
        },
        "the run id differs from the first chunk's",
    );
}

#[test]
fn admit_v4_refuses_a_dirty_candidate() {
    refuses(
        Build {
            dirty: true,
            ..Default::default()
        },
        "its tree was dirty",
    );
}

#[test]
fn admit_v4_refuses_a_later_dirty_chunk() {
    // Not the first chunk of the plan, and the first is clean.
    let (p, _) = adm_protocol();
    let last = runner::plan_v4(&p).last().unwrap().name();
    assert_ne!(last, runner::plan_v4(&p)[0].name());
    let last: &'static str = Box::leak(last.into_boxed_str());
    refuses(
        Build {
            dirty_chunk: Some(last),
            ..Default::default()
        },
        &format!("chunk {last}: its tree was dirty"),
    );
}

#[test]
fn admit_v4_refuses_a_target_whose_digest_differs_between_chunks() {
    let (a, b) = ("cost.swarmail.r0", "cost.swarmail.r1");
    // Two repetitions, so Swarmail has two chunks; the second ran other bytes.
    let f = build_run(Build {
        reps: 2,
        other_digest: Some(b),
        ..Default::default()
    });
    let w = f.admit(true).expect_err("must refuse");
    assert!(
        w.contains("swarmail's executable digest differs")
            && w.contains(&format!("chunk {b}"))
            && w.contains(a),
        "{w}"
    );
    // The same two chunks on one executable admit.
    build_run(Build {
        reps: 2,
        ..Default::default()
    })
    .admit(true)
    .expect("one executable per target admits");
}

#[test]
fn admit_v4_refuses_a_cleanup_without_its_launch_receipt() {
    refuses(
        Build {
            edit: Some(("cost.greenmail.r0", |r| r["launches"] = json!([]))),
            ..Default::default()
        },
        "chunk cost.greenmail.r0: launch and cleanup receipts must pair",
    );
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |r| {
                let c = r["cleanups"][0].clone();
                r["cleanups"].as_array_mut().unwrap().push(c);
            })),
            ..Default::default()
        },
        "1 launch receipts, 2 cleanup receipts",
    );
}

#[test]
fn admit_v4_refuses_a_row_whose_launch_has_no_receipt() {
    // Both receipt lists emptied: the counts still pair, but the chunk's
    // windows and samples name a launch nothing proves.
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |r| {
                r["launches"] = json!([]);
                r["cleanups"] = json!([]);
            })),
            ..Default::default()
        },
        "which has no launch receipt",
    );
}

#[test]
fn admit_v4_refuses_an_empty_plan_without_panicking() {
    let mut v = draft();
    v["throughput"] = json!([]);
    let p = p4(v);
    assert!(runner::plan_v4(&p).is_empty());
    let (root, sbytes) = (fixture(), serde_json::to_vec(&lock5()).unwrap());
    let lock = SourceLock::parse(&sbytes).unwrap();
    let pbytes = serde_json::to_vec(&p).unwrap();
    let bins = Default::default();
    let a = report::Admission {
        protocol: &p,
        protocol_bytes: &pbytes,
        lock: &lock,
        lock_bytes: &sbytes,
        bins: &bins,
    };
    // The directory is empty and is never read.
    for seal in [true, false] {
        let w = report::admit_v4(root.path(), &a, seal).unwrap_err();
        assert!(w.contains("chunk plan is empty"), "{w}");
    }
}

#[test]
fn the_sealed_manifest_names_a_draft_as_a_draft() {
    let f = build_run(Build::default());
    assert_eq!(f.p.status, protocol::Status::Draft);
    let sealed = f.admit(true).unwrap();
    assert_eq!(sealed.files["manifest.json"]["protocol_status"], "draft");
    // It is in the bytes on disk, and verification recomputes it.
    let on_disk: Value = json(&std::fs::read(f.dir().join("manifest.json")).unwrap());
    assert_eq!(on_disk["protocol_status"], "draft");
    rewrite_and_reindex(&f.dir(), "manifest.json", |m| {
        m["protocol_status"] = json!("frozen");
    });
    let w = f.admit(false).unwrap_err();
    assert!(
        w.contains("manifest.json") && w.contains("protocol_status"),
        "{w}"
    );
}

#[test]
fn admit_v4_refuses_differing_host_facts() {
    refuses(
        Build {
            other_host: Some("cost.greenmail.r0"),
            ..Default::default()
        },
        "the host facts (kernel, CPUs and layout) differs",
    );
}

#[test]
fn admit_v4_refuses_a_wrong_limit() {
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |v| {
                v["launches"][0]["limits"]["memory_max"] = json!("1073741824")
            })),
            ..Default::default()
        },
        "chunk cost.mailpit.r0: launch 0: the limits read back are",
    );
}

#[test]
fn admit_v4_refuses_a_missing_join_read_back() {
    refuses(
        Build {
            edit: Some(("cost.mailhog.r0", |v| {
                v["launches"][0].as_object_mut().unwrap().remove("joined");
            })),
            ..Default::default()
        },
        "the join read-back is missing",
    );
    // A leaf that is not the one the receipt names refuses too.
    refuses(
        Build {
            edit: Some(("cost.mailhog.r0", |v| {
                v["launches"][0]["joined"]["cgroup"] = json!("target-0099")
            })),
            ..Default::default()
        },
        "the join read-back is missing or is not the leaf",
    );
}

#[test]
fn admit_v4_refuses_a_wrong_cap() {
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |v| {
                for a in v["launches"][0]["argv_template"].as_array_mut().unwrap() {
                    if a == "150000" {
                        *a = json!("10000");
                    }
                }
            })),
            ..Default::default()
        },
        "a cap other than the protocol's",
    );
    // The candidate's own cap is the protocol's too.
    refuses(
        Build {
            edit: Some(("cost.swarmail.r0", |v| {
                for a in v["launches"][0]["argv_template"].as_array_mut().unwrap() {
                    if a == "150000" {
                        *a = json!("100000");
                    }
                }
            })),
            ..Default::default()
        },
        "a cap other than the protocol's",
    );
}

#[test]
fn admit_v4_refuses_cgroup_removed_false_or_absent() {
    refuses(
        Build {
            edit: Some(("cost.inbucket.r0", |v| {
                v["cleanups"][0]["cgroup_removed"] = json!(false)
            })),
            ..Default::default()
        },
        "cgroup_removed is false, not true",
    );
    refuses(
        Build {
            edit: Some(("cost.inbucket.r0", |v| {
                v["cleanups"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("cgroup_removed");
            })),
            ..Default::default()
        },
        "cgroup_removed is null, not true",
    );
}

#[test]
fn admit_v4_refuses_a_ledger_exit_two() {
    refuses(
        Build {
            ledger: Some(("cost.mailpit.r0", |e| e.exit = 2)),
            ..Default::default()
        },
        "ledger exit 2: the run is incomplete",
    );
    // Exit 1 is a measured outcome: the run still admits.
    let f = build_run(Build {
        ledger: Some(("cost.mailpit.r0", |e| e.exit = 1)),
        ..Default::default()
    });
    f.admit(true).expect("a chunk that exited 1 is admitted");
}

#[test]
fn admit_v4_refuses_an_unclean_scope() {
    refuses(
        Build {
            ledger: Some(("cost.greenmail.r0", |e| e.scope_clean = false)),
            ..Default::default()
        },
        "its scope was not clean",
    );
}

#[test]
fn admit_v4_refuses_a_second_seal() {
    let f = build_run(Build::default());
    f.admit(true).unwrap();
    let w = f.admit(true).unwrap_err();
    assert!(
        w.contains("already exists") && w.contains("never overwrites"),
        "{w}"
    );
    // The first seal is untouched and still verifies.
    f.admit(false).unwrap();
}

#[test]
fn admit_v4_names_the_path_of_an_edited_figure() {
    let f = build_run(Build::default());
    f.admit(true).unwrap();
    rewrite_and_reindex(&f.dir(), "report.json", |v| {
        v["cost"]["cells"][0]["visible_per_cpu_second"]["median"] = json!(99_999.0);
    });
    let w = f.admit(false).unwrap_err();
    assert!(
        w.contains("report.json.cost.cells[0].visible_per_cpu_second.median"),
        "the first differing path is named: {w}"
    );
    // An altered file with no new index is the index's refusal.
    let g = build_run(Build::default());
    g.admit(true).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(g.dir().join("host.json"))
        .unwrap();
    std::fs::write(g.dir().join("reproduce.txt"), "extra").unwrap();
    assert!(
        g.admit(false)
            .unwrap_err()
            .contains("index does not match the files present")
    );
    // And a missing footprint is named before anything is derived.
    let h = build_run(Build {
        no_footprint: true,
        ..Default::default()
    });
    assert!(
        h.admit(true)
            .unwrap_err()
            .contains("footprint.json is missing")
    );
}

// ---- S1.4 ------------------------------------------------------------------

#[test]
fn the_run_headline_carries_each_targets_installed_size() {
    let f = build_run(Build::default());
    let sealed = f.admit(true).unwrap();
    let want: Vec<Value> =
        f.p.launch_order(0)
            .enumerate()
            .map(|(i, t)| {
                json!({
                    "target": t,
                    "installed_bytes": 5000 + i,
                    "installed_files": 3 + i,
                    "installed_basis": "authored for the fixture run",
                })
            })
            .collect();
    assert_eq!(want.len(), 8, "every launched target, none blocked");
    for file in ["report.json", "report-cost.json"] {
        assert_eq!(
            sealed.files[file]["headline"]["installed"],
            json!(want),
            "{file}"
        );
    }
    // It is in the bytes on disk and verifies.
    let disk: Value = json(&std::fs::read(f.dir().join("report-cost.json")).unwrap());
    assert_eq!(disk["headline"]["installed"], json!(want));
    f.admit(false).unwrap();
    assert!(
        want.iter().all(|c| c["target"] != "mailslurper"),
        "the does-not-start row has no number"
    );
}

#[test]
fn a_chunk_report_has_no_installed_key() {
    let f = build_run(Build::default());
    f.admit(true).unwrap();
    for c in runner::plan_v4(&f.p) {
        let chunk: Value = json(
            &std::fs::read(f.dir().join("chunks").join(c.name()).join("report.json")).unwrap(),
        );
        assert!(
            chunk["headline"].get("installed").is_none(),
            "{}: {}",
            c.name(),
            chunk["headline"]
        );
    }
}

#[test]
fn admit_v4_refuses_a_footprint_without_a_launched_targets_row() {
    let f = build_run(Build {
        footprint: Some(|d| {
            d["targets"]
                .as_array_mut()
                .unwrap()
                .retain(|r| r["target"] != "inbucket");
        }),
        ..Default::default()
    });
    let w = f.admit(true).unwrap_err();
    assert!(w.contains("inbucket") && w.contains("0 rows"), "{w}");
    // Two rows for one target are as wrong as none.
    let f = build_run(Build {
        footprint: Some(|d| {
            let row = d["targets"][0].clone();
            d["targets"].as_array_mut().unwrap().push(row);
        }),
        ..Default::default()
    });
    let w = f.admit(true).unwrap_err();
    assert!(w.contains("swarmail") && w.contains("2 rows"), "{w}");
    // A list that is not a list.
    let f = build_run(Build {
        footprint: Some(|d| d["targets"] = json!("none")),
        ..Default::default()
    });
    assert!(f.admit(true).unwrap_err().contains("no list of targets"));
}

#[test]
fn admit_v4_refuses_a_null_or_zero_installed_size() {
    for (bad, shown) in [(json!(null), "null"), (json!(0), "0")] {
        let f = build_run(Build {
            footprint: Some(match shown {
                "null" => |d| d["targets"][3]["installed_bytes"] = json!(null),
                _ => |d| d["targets"][3]["installed_bytes"] = json!(0),
            }),
            ..Default::default()
        });
        let target = f.p.launch_order(0).nth(3).unwrap();
        let w = f.admit(true).unwrap_err();
        assert!(
            w.contains(target.name()) && w.contains("installed_bytes") && w.contains(shown),
            "{bad}: {w}"
        );
    }
    // A missing file count or basis is refused by name too.
    let f = build_run(Build {
        footprint: Some(|d| {
            d["targets"][1]
                .as_object_mut()
                .unwrap()
                .remove("installed_files");
        }),
        ..Default::default()
    });
    assert!(f.admit(true).unwrap_err().contains("installed_files"));
    let f = build_run(Build {
        footprint: Some(|d| {
            d["targets"][2]
                .as_object_mut()
                .unwrap()
                .remove("installed_basis");
        }),
        ..Default::default()
    });
    assert!(f.admit(true).unwrap_err().contains("installed_basis"));
}

#[test]
fn admit_v4_refuses_a_footprint_of_another_protocol() {
    let f = build_run(Build {
        footprint: Some(|d| d["protocol_id"] = json!("fair-mail-comparison-v3")),
        ..Default::default()
    });
    let w = f.admit(true).unwrap_err();
    assert!(
        w.contains("fair-mail-comparison-v3") && w.contains("fair-mail-comparison-v4"),
        "{w}"
    );
    let f = build_run(Build {
        footprint: Some(|d| {
            d.as_object_mut().unwrap().remove("protocol_id");
        }),
        ..Default::default()
    });
    assert!(
        f.admit(true)
            .unwrap_err()
            .contains("footprint.json is for protocol")
    );
}

#[test]
fn admit_v4_names_the_path_of_an_edited_installed_cell() {
    let f = build_run(Build::default());
    f.admit(true).unwrap();
    rewrite_and_reindex(&f.dir(), "report.json", |v| {
        v["headline"]["installed"][2]["installed_bytes"] = json!(1);
    });
    let w = f.admit(false).unwrap_err();
    assert!(
        w.contains("report.json.headline.installed[2].installed_bytes"),
        "the first differing path is named: {w}"
    );
}

#[test]
fn a_footprint_changed_after_the_seal_fails_the_verify() {
    let f = build_run(Build::default());
    f.admit(true).unwrap();
    // Changed without a new index: the index's refusal.
    let path = f.dir().join("footprint.json");
    let mut doc: Value = json(&std::fs::read(&path).unwrap());
    doc["targets"][0]["installed_bytes"] = json!(1);
    std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    let w = f.admit(false).unwrap_err();
    assert!(
        w.contains("index does not match the files present") || w.contains("headline.installed"),
        "{w}"
    );
    // Changed with the index remade: the headline no longer agrees.
    let g = build_run(Build::default());
    g.admit(true).unwrap();
    rewrite_and_reindex(&g.dir(), "footprint.json", |d| {
        d["targets"][0]["installed_bytes"] = json!(1);
    });
    let w = g.admit(false).unwrap_err();
    assert!(w.contains("headline.installed[0].installed_bytes"), "{w}");
}

// ---- S5 --------------------------------------------------------------------

#[test]
fn admit_v4_admits_a_sealed_chunk_that_launched_nothing() {
    // The chunk's instant came before it could launch: no receipts, rows
    // that say why, a ledger exit of 1. It seals, and it verifies.
    let f = build_run(Build {
        no_launch: Some("cost.inbucket.r0"),
        ..Default::default()
    });
    let chunk: Value =
        json(&std::fs::read(f.dir().join("chunks/cost.inbucket.r0/report.json")).unwrap());
    assert_eq!(chunk["launches"], json!([]), "nothing was launched");
    assert_eq!(chunk["cleanups"], json!([]));
    f.admit(true)
        .expect("a sealed zero-launch chunk is admitted");
    f.admit(false).expect("and verifies");
    let ledger: Value = json(&std::fs::read(f.dir().join("ledger/cost.inbucket.r0.json")).unwrap());
    assert_eq!(ledger["exit"], 1);
}

#[test]
fn a_run_with_a_zero_launch_chunk_still_refuses_a_missing_cleanup_elsewhere() {
    refuses(
        Build {
            no_launch: Some("cost.inbucket.r0"),
            edit: Some(("cost.greenmail.r0", |r| r["cleanups"] = json!([]))),
            ..Default::default()
        },
        "chunk cost.greenmail.r0: launch and cleanup receipts must pair",
    );
    // And an extra cleanup in the chunk that launched nothing is not a pair.
    refuses(
        Build {
            no_launch: Some("cost.inbucket.r0"),
            edit: Some(("cost.inbucket.r0", |r| {
                r["cleanups"] = json!([{"target": "inbucket", "leaf": "target-0001"}]);
            })),
            ..Default::default()
        },
        "chunk cost.inbucket.r0: launch and cleanup receipts must pair",
    );
}

#[test]
fn admit_v4_refuses_a_cleanup_that_names_another_launchs_leaf() {
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |r| {
                r["cleanups"][0]["leaf"] = json!("target-0009");
            })),
            ..Default::default()
        },
        "its cleanup receipt names the leaf \"target-0009\", not the launch's \"target-0001\"",
    );
}

#[test]
fn admit_v4_refuses_two_cleanups_of_one_leaf() {
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |r| {
                let (l, c) = (r["launches"][0].clone(), r["cleanups"][0].clone());
                r["launches"].as_array_mut().unwrap().push(l);
                r["cleanups"].as_array_mut().unwrap().push(c);
            })),
            ..Default::default()
        },
        "two cleanup receipts name the leaf \"target-0001\"",
    );
}

#[test]
fn admit_v4_refuses_a_leaf_cleanup_without_a_leaf_name() {
    refuses(
        Build {
            edit: Some(("cost.mailpit.r0", |r| {
                r["cleanups"][0].as_object_mut().unwrap().remove("leaf");
            })),
            ..Default::default()
        },
        "its cleanup receipt names no leaf",
    );
}

#[test]
fn an_idle_cell_ranks_only_on_all_three_values() {
    let rep = |memory: Option<f64>, anon: Option<f64>, scratch: Option<u64>| report::IdleRep {
        target: TargetKind::Mailpit,
        profile: "cost-c8-reused".into(),
        repetition: 0,
        memory_mib: memory,
        anon_mib: anon,
        scratch_bytes: scratch,
        shortfall: None,
    };
    let ranked = |r: report::IdleRep| report::idle_cells(&[r])[0].ranked;
    assert!(ranked(rep(Some(10.0), Some(5.0), Some(4096))), "all three");
    assert!(!ranked(rep(None, Some(5.0), Some(4096))), "no memory");
    assert!(!ranked(rep(Some(10.0), None, Some(4096))), "no anon");
    assert!(!ranked(rep(Some(10.0), Some(5.0), None)), "no scratch");
    // One repetition short is enough: the cell is not ranked.
    let cells = report::idle_cells(&[
        rep(Some(10.0), Some(5.0), Some(4096)),
        report::IdleRep {
            repetition: 1,
            ..rep(Some(10.0), Some(5.0), None)
        },
    ]);
    assert!(!cells[0].ranked);
}

#[test]
fn admit_run_still_refuses_a_protocol_4_directory() {
    let f = build_run(Build::default());
    let chunk = f.dir().join("chunks/cost.mailpit.r0");
    let w = report::admit_run(&chunk, &f.pbytes).unwrap_err();
    assert!(w.contains("admit-v4"), "{w}");
    // The v3 admission is not reachable from admit_v4 either.
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let l4 = SourceLock::parse(SOURCES_V4).unwrap();
    let bins = Default::default();
    let a = report::Admission {
        protocol: &p3,
        protocol_bytes: PROTOCOL_V3,
        lock: &l4,
        lock_bytes: SOURCES_V4,
        bins: &bins,
    };
    assert!(
        report::admit_v4(&f.dir(), &a, false)
            .unwrap_err()
            .contains("needs protocol_version 4")
    );
}

// ============================================================================
// T6: collect-v4, with a fake driver and fake systemd-run, systemctl,
// taskset and timeout on PATH. The script runs from a copy in a scratch git
// repository, so the candidate's tree is the test's to make clean or dirty.
// ============================================================================

const FAKE_UNAME: &str = "#!/bin/sh\nif [ \"$1\" = -s ]; then echo \"${FAKE_UNAME:-Linux}\"; else echo \"${FAKE_UNAME:-Linux}\"; fi\n";
const FAKE_NPROC: &str = "#!/bin/sh\nif [ \"$1\" = --all ]; then echo \"${FAKE_NPROC_ALL:-${FAKE_NPROC:-32}}\"; else echo \"${FAKE_NPROC:-32}\"; fi\n";
/// A `sleep` that returns at once: the wrapper's polls take no time.
const FAKE_SLEEP: &str = "#!/bin/sh\nexit 0\n";
/// A `cargo` that logs, and leaves what the wrapper looks for.
const FAKE_CARGO: &str = r#"#!/bin/bash
echo "cargo $*" >> "$FAKE_LOG"
# A build that is running: it says so, with its pid, and would run on.
if [ "${FAKE_STALL:-}" = "cargo-$1" ]; then
  echo $$ > "$FAKE_MARKER"
  exec @SLEEP@ 120
fi
case "$1" in
  build) mkdir -p target/release && cp "$FAKE_DIR/candidate" target/release/swarmail ;;
  bench) echo "{\"executable\":\"$FAKE_DIR/mail_compare-fake\"}" ;;
esac
exit 0
"#;
const FAKE_TASKSET: &str =
    "#!/bin/bash\necho \"taskset $*\" >> \"$FAKE_LOG\"\nshift 2\nexec \"$@\"\n";
const FAKE_SYSTEMD_RUN: &str = "#!/bin/bash\necho \"systemd-run $*\" >> \"$FAKE_LOG\"\nwhile [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\"\n";
const FAKE_TIMEOUT: &str = r#"#!/bin/bash
echo "timeout $*" >> "$FAKE_LOG"
case "$1" in --kill-after=*) shift 2 ;; *) shift 1 ;; esac
if [ -s "${FAKE_TIMEOUTS:-/dev/null}" ]; then
  while read -r coord; do
    case "$*" in *"--chunk $coord "*|*"--chunk $coord") exit 124 ;; esac
  done < "$FAKE_TIMEOUTS"
fi
exec "$@"
"#;
const FAKE_SYSTEMCTL: &str = r#"#!/bin/bash
echo "systemctl $*" >> "$FAKE_LOG"
case "$*" in
  --version) echo "systemd 259 (fake)"; exit 0 ;;
  *is-active*)
    unit="${@: -1}"
    # FAKE_SCOPE: lines `<coordinate> <answer>...`, one answer per call, the
    # last one repeating. An answer is a state word, `-` (nothing printed and
    # a non-zero status) or `wait` (it writes FAKE_SCOPE_MARKER and waits for
    # FAKE_SCOPE_MARKER.release, then says `active`).
    answer=''
    if [ -s "${FAKE_SCOPE:-/dev/null}" ]; then
      while read -r coord rest; do
        case "$unit" in *"$coord"*)
          n=$(cat "$FAKE_LOG.n.$coord" 2>/dev/null || echo 0)
          echo $((n + 1)) > "$FAKE_LOG.n.$coord"
          answers=($rest)
          [ "$n" -lt "${#answers[@]}" ] || n=$((${#answers[@]} - 1))
          answer="${answers[$n]}" ;;
        esac
      done < "$FAKE_SCOPE"
    fi
    case "$answer" in
      '') ;;
      -) exit 3 ;;
      wait)
        echo "$unit" > "$FAKE_SCOPE_MARKER"
        until [ -e "$FAKE_SCOPE_MARKER.release" ]; do @SLEEP@ 0.02; done
        echo active; exit 0 ;;
      active) echo active; exit 0 ;;
      *) echo "$answer"; exit 3 ;;
    esac
    if [ -n "${FAKE_ACTIVE_ONCE:-}" ]; then
      case "$unit" in *"$FAKE_ACTIVE_ONCE"*)
        if [ ! -e "$FAKE_LOG.once" ]; then : > "$FAKE_LOG.once"; echo active; exit 0; fi ;; esac
    fi
    if [ -s "${FAKE_ACTIVE:-/dev/null}" ]; then
      while read -r coord; do
        case "$unit" in *"$coord"*) echo active; exit 0 ;; esac
      done < "$FAKE_ACTIVE"
    fi
    echo inactive; exit 3 ;;
esac
exit 0
"#;
const FAKE_DRIVER: &str = r#"#!/bin/bash
echo "driver $*" >> "$FAKE_LOG"
# A step that is running: it says so, with its pid, and would run on.
if [ "${FAKE_STALL:-}" = "$1" ]; then
  echo $$ > "$FAKE_MARKER"
  exec @SLEEP@ 120
fi
case "$1" in
  check) echo "mail_compare check: ready"; exit 0 ;;
  targets)
    case " $* " in *" --plan "*) cat "$FAKE_PLAN" ;; *) cat "$FAKE_TARGETS" ;; esac
    exit 0 ;;
  footprint) echo '{"footprint_version":1}'; exit 0 ;;
  admit-v4)
    case " $* " in
      *" --seal "*) exit "${FAKE_SEAL_EXIT:-${FAKE_ADMIT_EXIT:-0}}" ;;
      *) exit "${FAKE_VERIFY_EXIT:-${FAKE_ADMIT_EXIT:-0}}" ;;
    esac ;;
  run)
    out='' chunk=''
    while [ $# -gt 0 ]; do
      case "$1" in --output) out="$2" ;; --chunk) chunk="$2" ;; esac
      shift
    done
    how=$(grep "^$chunk " "${FAKE_BEHAVE:-/dev/null}" 2>/dev/null | cut -d' ' -f2)
    # Whether the run's reproduce.txt existed when this chunk began.
    root=$(dirname "$(dirname "$out")")
    [ ! -e "$root/reproduce.txt" ] || echo "present when $chunk began" >> "$FAKE_LOG.repro"
    mkdir -p "$out"
    if [ "$how" = wait ]; then
      # A driver that is running: it says so, and ends as the real one does
      # on a signal (exit 2, nothing sealed).
      echo "$chunk" > "$FAKE_MARKER"
      trap 'echo terminated > "$FAKE_MARKER.done"; exit 2' TERM
      while :; do command -p sleep 0.05; done
    fi
    [ "$how" = noindex ] || echo '{}' > "$out/index.json"
    case "$how" in noindex|'') exit 0 ;; *) exit "$how" ;; esac ;;
esac
exit 2
"#;

struct V4Rig {
    root: tempfile::TempDir,
    plan: Vec<String>,
}

impl V4Rig {
    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn out(&self) -> PathBuf {
        self.root.path().join("out")
    }

    fn file(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    fn log(&self) -> Vec<String> {
        std::fs::read_to_string(self.file("log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The competitors the draft declares, as the driver would list them.
    fn competitors() -> Vec<String> {
        let p = p4(draft());
        protocol::declared_competitors(&p)
            .iter()
            .map(|t| t.name().to_owned())
            .collect()
    }

    /// `collect-v4` with the draft route, every competitor given, and the
    /// extra environment. Returns (exit code, output).
    fn run(&self, extra: &[&str], env: &[(&str, &str)]) -> (i32, String) {
        let mut args: Vec<String> = ["collect-v4", "--output"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        args.push(self.out().display().to_string());
        for t in Self::competitors() {
            args.extend(["--bin".to_owned(), format!("{t}=/bin/sh")]);
        }
        args.extend(
            [
                "--runtime",
                "greenmail=/never/jre",
                "--runtime",
                "alkemio-mailslurper=/never/musl",
            ]
            .map(str::to_owned),
        );
        args.extend(extra.iter().map(|s| (*s).to_owned()));
        self.exec(&args, env)
    }

    /// The wrapper's command, in a cleared environment of the rig's own: its
    /// git sees no ambient configuration.
    fn command(&self, args: &[String], env: &[(&str, &str)]) -> std::process::Command {
        let path = format!(
            "{}:{}",
            self.file("fake").display(),
            self.file("tools").display()
        );
        let mut cmd = std::process::Command::new("bash");
        cmd.arg(self.repo().join("scripts/compare-mail.sh"))
            .args(args)
            .env_clear()
            .env("PATH", path)
            .env("HOME", self.root.path())
            .env("FAKE_LOG", self.file("log"))
            .env("FAKE_DIR", self.file("fake"))
            .env("FAKE_PLAN", self.file("plan"))
            .env("FAKE_TARGETS", self.file("targets"))
            .env("COMPARE_MAIL_DRIVER_BIN", self.file("fake/driver"))
            .env("COMPARE_MAIL_SWARMAIL_BIN", self.file("fake/candidate"))
            .env("COMPARE_MAIL_BIN_DIR", self.file("frozen"))
            .envs(env.iter().copied());
        own_git_environment(&mut cmd);
        cmd
    }

    fn exec(&self, args: &[String], env: &[(&str, &str)]) -> (i32, String) {
        let out = self.command(args, env).output().unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.code().unwrap_or(-1), text)
    }

    /// The ledger entries that exist, in plan order.
    fn ledger(&self) -> Vec<Value> {
        self.plan
            .iter()
            .filter_map(|c| std::fs::read(self.out().join("ledger").join(format!("{c}.json"))).ok())
            .map(|b| json(&b))
            .collect()
    }
}

/// The rig owns its git environment: no system or global configuration, and
/// none of the `GIT_CONFIG_COUNT` family that a caller's shell may carry
/// (a count with no key makes git refuse to run at all).
fn own_git_environment(cmd: &mut std::process::Command) {
    for (name, _) in std::env::vars_os().filter_map(|(k, v)| k.into_string().ok().map(|k| (k, v))) {
        if name == "GIT_CONFIG_COUNT"
            || name.starts_with("GIT_CONFIG_KEY_")
            || name.starts_with("GIT_CONFIG_VALUE_")
        {
            cmd.env_remove(&name);
        }
    }
    cmd.env_remove("GIT_CONFIG_COUNT")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
}

fn v4_rig() -> V4Rig {
    v4_rig_with(&[])
}

/// The host's own `sleep`, which the fakes `exec` so that a signal sent to
/// the process they stand for reaches the sleeping process itself.
fn real_sleep() -> String {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|d| d.join("sleep"))
        .find(|p| p.is_file())
        .expect("a sleep on PATH")
        .display()
        .to_string()
}

/// The rig, built while `ambient` is in the environment of the commands it
/// runs (so a test can show that an ambient git configuration is ignored).
fn v4_rig_with(ambient: &[(&str, &str)]) -> V4Rig {
    let root = fixture();
    let r = root.path();
    let plan: Vec<String> = runner::plan_v4(&p4(draft()))
        .iter()
        .map(runner::Chunk::name)
        .collect();
    for d in ["repo/scripts", "repo/docs/evals/draft", "fake", "tools"] {
        std::fs::create_dir_all(r.join(d)).unwrap();
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::copy(
        here.join("scripts/compare-mail.sh"),
        r.join("repo/scripts/compare-mail.sh"),
    )
    .unwrap();
    for (from, to) in [
        ("docs/evals/protocol.json", "docs/evals/protocol.json"),
        (
            "docs/evals/sources.lock.json",
            "docs/evals/sources.lock.json",
        ),
        (
            "docs/evals/draft/protocol.v4.json",
            "docs/evals/draft/protocol.v4.json",
        ),
        (
            "docs/evals/draft/sources.lock.v5.json",
            "docs/evals/draft/sources.lock.v5.json",
        ),
    ] {
        std::fs::copy(here.join(from), r.join("repo").join(to)).unwrap();
    }
    std::fs::write(r.join("plan"), plan.join("\n") + "\n").unwrap();
    std::fs::write(r.join("targets"), V4Rig::competitors().join("\n") + "\n").unwrap();
    for (name, body) in [
        ("uname", FAKE_UNAME),
        ("nproc", FAKE_NPROC),
        ("sleep", FAKE_SLEEP),
        ("cargo", FAKE_CARGO),
        ("mail_compare-fake", FAKE_DRIVER),
        ("taskset", FAKE_TASKSET),
        ("systemd-run", FAKE_SYSTEMD_RUN),
        ("timeout", FAKE_TIMEOUT),
        ("systemctl", FAKE_SYSTEMCTL),
        ("driver", FAKE_DRIVER),
        ("candidate", "#!/bin/sh\nexit 0\n"),
    ] {
        let p = r.join("fake").join(name);
        std::fs::write(&p, body.replace("@SLEEP@", &real_sleep())).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    // The real tools the script itself needs, and nothing else: a tool the
    // test leaves out is genuinely missing.
    for tool in [
        "bash",
        "sh",
        "sed",
        "grep",
        "tr",
        "cut",
        "git",
        "date",
        "cat",
        "head",
        "tail",
        "wc",
        "mkdir",
        "rmdir",
        "dirname",
        "basename",
        "realpath",
        "sha256sum",
        "cp",
        "chmod",
        "mv",
        "rm",
        "awk",
        "sort",
        "env",
    ] {
        if let Some(found) = std::env::var_os("PATH")
            .into_iter()
            .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
            .map(|d| d.join(tool))
            .find(|p| p.exists())
        {
            let _ = std::os::unix::fs::symlink(found, r.join("tools").join(tool));
        }
    }
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(r.join("repo"))
            .args([
                "-c",
                "user.email=t@example.invalid",
                "-c",
                "user.name=t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .envs(ambient.iter().copied());
        own_git_environment(&mut cmd);
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    // The candidate is on main (0132 §2.3): a rig's default fixture is.
    git(&["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "fixture"]);
    V4Rig { root, plan }
}

#[test]
fn collect_v4_dry_run_prints_24_commands_in_plan_order_and_starts_nothing() {
    let rig = v4_rig();
    let draft_route = [
        "--protocol",
        "docs/evals/draft/protocol.v4.json",
        "--sources",
        "docs/evals/draft/sources.lock.v5.json",
        "--documents",
        "docs/evals",
    ];
    let mut extra = draft_route.to_vec();
    extra.push("--dry-run");
    let (code, text) = rig.run(&extra, &[]);
    assert_eq!(code, 0, "{text}");
    let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("cost.")).collect();
    assert_eq!(lines.len(), 24, "{text}");
    for (line, coord) in lines.iter().zip(&rig.plan) {
        assert!(
            line.starts_with(&format!("{coord}\t")),
            "{line} should be {coord}"
        );
        assert!(line.contains("timeout --kill-after=30 560 taskset -c 0-7 systemd-run --user --scope -p Delegate=yes"), "{line}");
        assert!(line.contains(&format!("--chunk {coord} ")), "{line}");
    }
    // Nothing was launched, written or built.
    assert!(!rig.out().exists());
    let log = rig.log();
    // The plan and the version, each under its bound and its own line.
    let reads =
        |l: &String| l.contains(" targets --protocol") || l.ends_with("systemctl --version");
    assert!(
        log.iter().all(reads),
        "only the plan and the version were read: {log:?}"
    );
    // Not even the clean tree is demanded of a dry run.
    std::fs::write(rig.repo().join("scratch.txt"), "dirty").unwrap();
    let (code, text) = rig.run(&extra, &[]);
    assert_eq!(code, 0, "{text}");
}

#[test]
fn collect_v4_refuses_the_default_protocol_and_names_the_draft_route() {
    let rig = v4_rig();
    let (code, text) = rig.run(&[], &[]);
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("blocked at phase \"protocol\""), "{text}");
    assert!(
        text.contains("collect-v4 needs a protocol_version 4 protocol"),
        "{text}"
    );
    assert!(text.contains("--protocol docs/evals/draft/protocol.v4.json --sources docs/evals/draft/sources.lock.v5.json --documents docs/evals"), "{text}");
    assert!(rig.log().is_empty() && !rig.out().exists());
}

#[test]
fn collect_v4_names_each_preflight_refusal() {
    let route = [
        "--protocol",
        "docs/evals/draft/protocol.v4.json",
        "--sources",
        "docs/evals/draft/sources.lock.v5.json",
        "--documents",
        "docs/evals",
    ];
    // A tool that is not on PATH.
    for tool in ["systemd-run", "systemctl", "taskset", "timeout"] {
        let rig = v4_rig();
        std::fs::remove_file(rig.file("fake").join(tool)).unwrap();
        let (code, text) = rig.run(&route, &[]);
        assert_eq!(code, 2, "{tool}: {text}");
        assert!(
            text.contains(&format!("collect-v4 needs {tool} on PATH")),
            "{text}"
        );
        assert!(rig.log().is_empty(), "nothing ran: {tool}");
    }
    // An existing --output.
    let rig = v4_rig();
    std::fs::create_dir(rig.out()).unwrap();
    let (code, text) = rig.run(&route, &[]);
    assert_eq!(code, 2);
    assert!(
        text.contains("--output exists; runs are never overwritten or resumed"),
        "{text}"
    );
    // Too few CPUs in min(nproc/4, 32): 8 gives 2.
    let rig = v4_rig();
    let (code, text) = rig.run(&route, &[("FAKE_NPROC", "8")]);
    assert_eq!(code, 2);
    assert!(
        text.contains("at least 3 CPUs in min(nproc/4, 32), and nproc 8 gives 2"),
        "{text}"
    );
    // 12 is the least that is enough; 4096 is capped at 32 CPUs.
    let rig = v4_rig();
    let mut dry = route.to_vec();
    dry.push("--dry-run");
    let (code, text) = rig.run(&dry, &[("FAKE_NPROC", "12")]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("taskset -c 0-2 "), "{text}");
    let (_, text) = rig.run(&dry, &[("FAKE_NPROC", "4096")]);
    assert!(
        text.contains("taskset -c 0-31 ") && !text.contains("0-32"),
        "{text}"
    );
    // A dirty candidate, in a real run only.
    let rig = v4_rig();
    std::fs::write(rig.repo().join("untracked.txt"), "x").unwrap();
    let (code, text) = rig.run(&route, &[]);
    assert_eq!(code, 2);
    assert!(text.contains("the candidate tree is dirty"), "{text}");
    assert!(
        !rig.out().exists() && rig.log().is_empty(),
        "{:?}",
        rig.log()
    );
    // An uncommitted candidate.
    let rig = v4_rig();
    std::fs::remove_dir_all(rig.repo().join(".git")).unwrap();
    let (code, text) = rig.run(&route, &[]);
    assert_eq!(code, 2);
    assert!(text.contains("not a committed git checkout"), "{text}");
    // A competitor with no --bin is the existing refusal, by name.
    let rig = v4_rig();
    let args: Vec<String> = ["collect-v4", "--output", rig.out().to_str().unwrap()]
        .iter()
        .chain(route.iter())
        .map(|s| (*s).to_owned())
        .collect();
    let (code, text) = rig.exec(&args, &[]);
    assert_eq!(code, 2);
    assert!(
        text.contains("the protocol declares mailpit; pass --bin mailpit=PATH"),
        "{text}"
    );
    // Not Linux: the one mode that refuses by platform.
    let rig = v4_rig();
    let (code, text) = rig.run(&route, &[("FAKE_UNAME", "Darwin")]);
    assert_eq!(code, 2);
    assert!(
        text.contains("collect-v4 runs on Linux with delegated cgroup v2 only"),
        "{text}"
    );
}

const DRAFT_ROUTE: [&str; 6] = [
    "--protocol",
    "docs/evals/draft/protocol.v4.json",
    "--sources",
    "docs/evals/draft/sources.lock.v5.json",
    "--documents",
    "docs/evals",
];

fn chunk_lines(log: &[String], prefix: &str) -> Vec<String> {
    log.iter()
        .filter(|l| l.starts_with(prefix))
        .cloned()
        .collect()
}

#[test]
fn collect_v4_nests_timeout_taskset_then_the_scope() {
    let rig = v4_rig();
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    // The first chunk's four commands, in nesting order.
    let first = log
        .iter()
        .position(|l| l.starts_with("timeout ") && l.contains("--chunk cost.swarmail.r0"))
        .unwrap();
    let kinds: Vec<&str> = log[first..first + 4]
        .iter()
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["timeout", "taskset", "systemd-run", "driver"],
        "{:?}",
        &log[first..first + 4]
    );
    assert!(log[first].starts_with("timeout --kill-after=30 560 taskset -c 0-7 systemd-run --user --scope -p Delegate=yes --unit=mail-compare-"), "{}", log[first]);
    assert!(
        log[first + 1].starts_with("taskset -c 0-7 systemd-run --user --scope"),
        "{}",
        log[first + 1]
    );
    assert!(
        log[first + 2]
            .starts_with("systemd-run --user --scope -p Delegate=yes --unit=mail-compare-"),
        "{}",
        log[first + 2]
    );
    let run = &log[first + 3];
    assert!(
        run.starts_with("driver run --mode collect --protocol docs/evals/draft/protocol.v4.json"),
        "{run}"
    );
    assert!(
        run.contains("--run-id collect-v4-")
            && run.contains("--commit ")
            && run.contains("--dirty false"),
        "{run}"
    );
    assert!(run.contains("--systemd systemd 259 (fake)"), "{run}");
    // A chunk is given its own target's bytes and no other's.
    assert!(
        run.contains("--bin swarmail=") && !run.contains("--bin mailpit="),
        "{run}"
    );
    let mailpit = log
        .iter()
        .find(|l| l.starts_with("driver run") && l.contains("--chunk cost.mailpit.r0"))
        .unwrap();
    assert!(
        mailpit.contains("--bin mailpit=/bin/sh") && !mailpit.contains("--bin swarmail="),
        "{mailpit}"
    );
    let jre = log
        .iter()
        .find(|l| l.starts_with("driver run") && l.contains("--chunk cost.greenmail.r0"))
        .unwrap();
    assert!(jre.contains("--runtime greenmail=/never/jre"), "{jre}");
    // Every chunk is bounded the same way, in its own scope.
    let units: std::collections::BTreeSet<&str> = log
        .iter()
        .filter(|l| l.starts_with("systemd-run "))
        .map(|l| l.split(' ').find(|w| w.starts_with("--unit=")).unwrap())
        .collect();
    assert_eq!(units.len(), 24, "one scope per chunk");
    assert!(chunk_lines(&log, "timeout --kill-after=30 560").len() >= 24);
}

#[test]
fn collect_v4_stops_on_a_chunk_exit_two() {
    let rig = v4_rig();
    std::fs::write(rig.file("behave"), "cost.mailhog.r0 2\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_BEHAVE", rig.file("behave").to_str().unwrap())],
    );
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("blocked at phase \"chunk-cost.mailhog.r0\""),
        "{text}"
    );
    assert!(
        text.contains("is incomplete and is never resumed"),
        "{text}"
    );
    let ledger = rig.ledger();
    assert_eq!(
        ledger.len(),
        3,
        "swarmail, mailpit and the chunk that failed"
    );
    assert_eq!(ledger[2]["coordinate"], "cost.mailhog.r0");
    assert_eq!(ledger[2]["exit"], 2);
    let log = rig.log();
    assert!(
        chunk_lines(&log, "driver run").len() == 3,
        "no later chunk started: {log:?}"
    );
    assert!(
        chunk_lines(&log, "driver admit-v4").is_empty(),
        "an incomplete run is never admitted"
    );
    // Nothing was removed: the sealed chunks stay where they are.
    assert!(
        rig.out()
            .join("chunks/cost.swarmail.r0/index.json")
            .is_file()
    );
}

#[test]
fn collect_v4_counts_a_timeout_as_two() {
    let rig = v4_rig();
    std::fs::write(rig.file("timeouts"), "cost.mailpit.r0\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_TIMEOUTS", rig.file("timeouts").to_str().unwrap())],
    );
    assert_eq!(code, 2, "{text}");
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 2);
    assert_eq!(
        (ledger[1]["coordinate"].as_str(), ledger[1]["exit"].as_i64()),
        (Some("cost.mailpit.r0"), Some(2))
    );
    assert!(
        chunk_lines(&rig.log(), "driver run").len() == 1,
        "the timed-out chunk never ran its driver"
    );
}

#[test]
fn collect_v4_counts_a_sealless_exit_zero_as_two() {
    let rig = v4_rig();
    std::fs::write(rig.file("behave"), "cost.swarmail.r0 noindex\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_BEHAVE", rig.file("behave").to_str().unwrap())],
    );
    assert_eq!(code, 2, "{text}");
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 1);
    assert_eq!(
        ledger[0]["exit"], 2,
        "an exit 0 with no completion index was never sealed"
    );
    assert_eq!(ledger[0]["scope_clean"], true);
}

#[test]
fn collect_v4_full_fake_run_writes_24_ledgers_seals_and_verifies() {
    let rig = v4_rig();
    std::fs::write(rig.file("behave"), "cost.smtp4dev.r1 1\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_BEHAVE", rig.file("behave").to_str().unwrap())],
    );
    assert_eq!(
        code, 1,
        "a chunk that exited 1 makes the run exit 1: {text}"
    );
    assert!(
        text.contains("wrote") && text.contains("(24 chunks)"),
        "{text}"
    );
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 24);
    let coords: std::collections::BTreeSet<&str> = ledger
        .iter()
        .map(|e| e["coordinate"].as_str().unwrap())
        .collect();
    assert_eq!(coords, rig.plan.iter().map(String::as_str).collect());
    for e in &ledger {
        assert!(
            matches!(e["exit"].as_i64(), Some(0 | 1)) && e["scope_clean"] == true,
            "{e}"
        );
        assert!(
            e["command"].as_str().unwrap().contains("systemd-run"),
            "{e}"
        );
        assert!(
            e["load_before"].is_string() && e["load_after"].is_string() && e["wall_ms"].is_u64()
        );
    }
    assert_eq!(ledger.iter().filter(|e| e["exit"] == 1).count(), 1);
    // The plan's order is the order they ran in.
    let ran: Vec<String> = chunk_lines(&rig.log(), "driver run")
        .iter()
        .map(|l| {
            l.split("--chunk ")
                .nth(1)
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(ran, rig.plan);
    let repro = std::fs::read_to_string(rig.out().join("reproduce.txt")).unwrap();
    assert!(
        repro.starts_with("scripts/compare-mail.sh collect-v4 --output "),
        "{repro}"
    );
    assert!(
        repro.contains("--protocol docs/evals/draft/protocol.v4.json"),
        "{repro}"
    );
    assert!(rig.out().join("footprint.json").is_file());
    // Sealed, then verified, in that order and after every chunk.
    let log = rig.log();
    let admits = chunk_lines(&log, "driver admit-v4");
    assert_eq!(admits.len(), 2, "{admits:?}");
    assert!(
        admits[0].starts_with("driver admit-v4 --seal ") && !admits[1].contains("--seal"),
        "{admits:?}"
    );
    let last_run = log
        .iter()
        .rposition(|l| l.starts_with("driver run"))
        .unwrap();
    let first_admit = log
        .iter()
        .position(|l| l.starts_with("driver admit-v4"))
        .unwrap();
    assert!(last_run < first_admit);
    assert!(
        admits[0].contains("--bin swarmail=") && admits[0].contains("--bin mailpit=/bin/sh"),
        "{admits:?}"
    );
    // The scratch root is gone.
    assert!(!rig.file("out.scratch").exists());
    // All chunks exiting 0 is exit 0; an admission that refuses is exit 2.
    let rig = v4_rig();
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 0, "{text}");
    let rig = v4_rig();
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_ADMIT_EXIT", "2")]);
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("blocked at phase \"admission\""), "{text}");
}

#[test]
fn collect_v4_usage_names_the_mode() {
    let root = fixture();
    let (code, text) = compare_mail(&[], &root.path().join("w"));
    assert_eq!(code, 2);
    assert!(text.contains("compare-mail.sh collect-v4"), "{text}");
    assert!(text.contains("Linux only"), "{text}");
}

// ---- R6: the wrapper's waits, signal, bound and git environment --------------

fn behave(rig: &V4Rig, lines: &str) -> String {
    std::fs::write(rig.file("behave"), lines).unwrap();
    rig.file("behave").to_str().unwrap().to_owned()
}

fn is_active_calls(log: &[String], coord: &str) -> usize {
    log.iter()
        .filter(|l| l.starts_with("systemctl --user is-active") && l.contains(coord))
        .count()
}

#[test]
fn collect_v4_waits_for_a_scope_that_is_active_once_then_inactive() {
    let rig = v4_rig();
    // systemd has not yet learned that the scope is empty at the first look.
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_ACTIVE_ONCE", "cost.mailpit.r0")]);
    assert_eq!(code, 0, "a scope that goes inactive is clean: {text}");
    let log = rig.log();
    assert_eq!(is_active_calls(&log, "cost.mailpit.r0"), 2, "{log:?}");
    assert!(
        !log.iter().any(|l| l.starts_with("systemctl --user kill")),
        "nothing was killed: {log:?}"
    );
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 24, "the run went on");
    assert!(ledger.iter().all(|e| e["scope_clean"] == true));
}

#[test]
fn collect_v4_stops_a_scope_that_stays_active_and_counts_two() {
    let rig = v4_rig();
    std::fs::write(rig.file("active"), "cost.mailpit.r0\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_ACTIVE", rig.file("active").to_str().unwrap())],
    );
    assert_eq!(code, 2, "{text}");
    let log = rig.log();
    // Twenty queries, and no more, before it is called alive (S3.3).
    assert_eq!(is_active_calls(&log, "cost.mailpit.r0"), 20, "{log:?}");
    let last_look = log
        .iter()
        .rposition(|l| l.starts_with("systemctl --user is-active") && l.contains("cost.mailpit.r0"))
        .unwrap();
    let kill = log
        .iter()
        .position(|l| {
            l.starts_with("systemctl --user kill --signal=KILL mail-compare-")
                && l.contains("cost.mailpit.r0")
        })
        .expect("the scope was killed");
    let stop = log
        .iter()
        .position(|l| {
            l.starts_with("systemctl --user stop mail-compare-") && l.contains("cost.mailpit.r0")
        })
        .expect("and stopped");
    assert!(last_look < kill && kill < stop, "after the wait: {log:?}");
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 2);
    assert_eq!(
        (ledger[1]["exit"].as_i64(), &ledger[1]["scope_clean"]),
        (Some(2), &json!(false))
    );
    assert_eq!(chunk_lines(&log, "driver run").len(), 2, "no later chunk");
}

#[test]
fn collect_v4_writes_reproduce_txt_only_after_the_last_chunk() {
    let rig = v4_rig();
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 0, "{text}");
    // No chunk's driver ever found it.
    let seen = std::fs::read_to_string(rig.file("log.repro")).unwrap_or_default();
    assert!(seen.is_empty(), "present while a chunk ran: {seen}");
    assert!(rig.out().join("reproduce.txt").is_file());
    // And the seal and the verification come after it.
    let log = rig.log();
    assert_eq!(chunk_lines(&log, "driver admit-v4").len(), 2);
}

#[test]
fn an_incomplete_collect_v4_has_no_reproduce_txt() {
    let rig = v4_rig();
    let b = behave(&rig, "cost.mailhog.r0 2\n");
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_BEHAVE", &b)]);
    assert_eq!(code, 2, "{text}");
    assert!(!rig.out().join("reproduce.txt").exists());
    assert!(rig.out().join("ledger").is_dir(), "the ledger is there");
}

#[test]
fn collect_v4_on_sigterm_ends_the_chunk_writes_its_ledger_and_exits_two() {
    let rig = v4_rig();
    let b = behave(&rig, "cost.swarmail.r0 wait\n");
    let marker = rig.file("marker");
    let mut args: Vec<String> = vec![
        "collect-v4".into(),
        "--output".into(),
        rig.out().display().to_string(),
    ];
    args.extend(DRAFT_ROUTE.map(str::to_owned));
    for t in V4Rig::competitors() {
        args.extend(["--bin".to_owned(), format!("{t}=/bin/sh")]);
    }
    args.extend(
        [
            "--runtime",
            "greenmail=/never/jre",
            "--runtime",
            "alkemio-mailslurper=/never/musl",
        ]
        .map(str::to_owned),
    );
    let mut child = rig
        .command(
            &args,
            &[
                ("FAKE_BEHAVE", &b),
                ("FAKE_MARKER", marker.to_str().unwrap()),
            ],
        )
        .spawn()
        .unwrap();
    // The fake driver says it is running; then, and only then, the signal.
    let deadline = Instant::now() + Duration::from_secs(60);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "the driver never started");
        assert!(
            child.try_wait().unwrap().is_none(),
            "the wrapper ended early"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // SAFETY: signalling a child process this test owns.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        assert!(Instant::now() < deadline, "the wrapper did not end");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(2), "{status:?}");
    // The chunk was ended by the signal, not abandoned: the driver ran its
    // own interrupt path, and its entry is in the ledger with exit 2.
    assert!(rig.file("marker.done").exists(), "the driver was signalled");
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 1, "{ledger:?}");
    assert_eq!(
        (ledger[0]["coordinate"].as_str(), ledger[0]["exit"].as_i64()),
        (Some("cost.swarmail.r0"), Some(2))
    );
    assert_eq!(ledger[0]["scope_clean"], true);
    assert!(!rig.out().join("index.json").exists());
    assert!(!rig.out().join("reproduce.txt").exists());
    assert!(
        !rig.out()
            .join("chunks/cost.swarmail.r0/index.json")
            .exists()
    );
    let log = rig.log();
    assert_eq!(chunk_lines(&log, "driver run").len(), 1, "no later chunk");
    assert!(chunk_lines(&log, "driver admit-v4").is_empty());
}

#[test]
fn collect_v4_runs_every_process_under_the_bound() {
    let rig = v4_rig();
    // No overrides: the wrapper builds the candidate and the driver itself.
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[
            ("COMPARE_MAIL_DRIVER_BIN", ""),
            ("COMPARE_MAIL_SWARMAIL_BIN", ""),
        ],
    );
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    let bound = "timeout --kill-after=30 560 taskset -c 0-7 ";
    // The timeout line that carries each kind of process, and the process's
    // own line: one for each, every time.
    let kinds = [
        ("cargo build --release --locked", "cargo build"),
        ("cargo bench --locked", "cargo bench"),
        ("targets --protocol", "driver targets"),
        ("check --protocol", "driver check"),
        ("footprint --protocol", "driver footprint"),
        ("run --mode collect", "driver run"),
        ("admit-v4 --seal", "driver admit-v4 --seal"),
        ("admit-v4 /", "driver admit-v4 /"),
    ];
    for (needle, own) in kinds {
        let started = log.iter().filter(|l| l.starts_with(own)).count();
        let bounded = log
            .iter()
            .filter(|l| l.starts_with(bound) && l.contains(needle))
            .count();
        assert!(started >= 1, "{own} ran: {log:?}");
        assert_eq!(
            bounded, started,
            "{own}: each run is under the bound: {log:?}"
        );
    }
    assert_eq!(chunk_lines(&log, "driver run").len(), 24);
}

#[test]
fn collect_v4_refuses_a_restricted_shell_by_name() {
    let rig = v4_rig();
    // A shell narrowed to 32 of 64 CPUs would be pinned outside its own set.
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_NPROC", "32"), ("FAKE_NPROC_ALL", "64")],
    );
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("needs a shell that may use all of the host's CPUs")
            && text.contains("nproc is 32, nproc --all is 64"),
        "{text}"
    );
    assert!(
        rig.log().is_empty(),
        "before anything was built or run: {:?}",
        rig.log()
    );
    assert!(!rig.out().exists());
}

#[test]
fn collect_v4_chunk_timeout_is_chunk_kill_ms() {
    let rig = v4_rig();
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    let chunk = log
        .iter()
        .find(|l| l.starts_with("timeout ") && l.contains("--chunk cost.swarmail.r0"))
        .unwrap();
    let seconds: u64 = chunk
        .split(' ')
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("a number of seconds in {chunk}"));
    assert_eq!(seconds * 1000, protocol::CHUNK_KILL_MS);
    // And the protocol's own bound is what that number is for.
    assert!(
        chunk.starts_with("timeout --kill-after=30 560 taskset"),
        "{chunk}"
    );
}

#[test]
fn collect_v4_counts_exit_three_and_a_kill_as_two() {
    for (status, coord) in [("3", "cost.mailpit.r0"), ("137", "cost.swarmail.r0")] {
        let rig = v4_rig();
        let b = behave(&rig, &format!("{coord} {status}\n"));
        let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_BEHAVE", &b)]);
        assert_eq!(code, 2, "{status}: {text}");
        let ledger = rig.ledger();
        let last = ledger.last().unwrap();
        assert_eq!(
            (last["coordinate"].as_str(), last["exit"].as_i64()),
            (Some(coord), Some(2)),
            "{status}"
        );
        assert!(!rig.out().join("reproduce.txt").exists(), "{status}");
        assert!(chunk_lines(&rig.log(), "driver admit-v4").is_empty());
    }
}

#[test]
fn collect_v4_a_timeout_with_a_live_scope_is_killed_and_counted() {
    let rig = v4_rig();
    std::fs::write(rig.file("timeouts"), "cost.mailpit.r0\n").unwrap();
    std::fs::write(rig.file("active"), "cost.mailpit.r0\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[
            ("FAKE_TIMEOUTS", rig.file("timeouts").to_str().unwrap()),
            ("FAKE_ACTIVE", rig.file("active").to_str().unwrap()),
        ],
    );
    assert_eq!(code, 2, "{text}");
    let ledger = rig.ledger();
    let last = ledger.last().unwrap();
    assert_eq!(
        (last["exit"].as_i64(), &last["scope_clean"]),
        (Some(2), &json!(false))
    );
    let log = rig.log();
    assert!(
        log.iter()
            .any(|l| l.starts_with("systemctl --user kill") && l.contains("cost.mailpit.r0")),
        "{log:?}"
    );
    assert!(
        log.iter()
            .any(|l| l.starts_with("systemctl --user stop") && l.contains("cost.mailpit.r0")),
        "{log:?}"
    );
}

#[test]
fn collect_v4_a_seal_followed_by_a_refused_verify_exits_two() {
    let rig = v4_rig();
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_VERIFY_EXIT", "1")]);
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("blocked at phase \"admission\"") && text.contains("not admitted"),
        "{text}"
    );
    let log = rig.log();
    assert_eq!(
        chunk_lines(&log, "driver admit-v4 --seal").len(),
        1,
        "it sealed first"
    );
    assert_eq!(chunk_lines(&log, "driver admit-v4").len(), 2);
    // A refused seal never reaches the verification.
    let rig = v4_rig();
    let (code, _) = rig.run(&DRAFT_ROUTE, &[("FAKE_SEAL_EXIT", "1")]);
    assert_eq!(code, 2);
    assert_eq!(chunk_lines(&rig.log(), "driver admit-v4").len(), 1);
}

#[test]
fn the_v4_rig_ignores_an_ambient_git_config_count() {
    // A count with no key makes git refuse to run at all: the rig owns its
    // git environment, for the fixture and for the wrapper alike.
    let rig = v4_rig_with(&[("GIT_CONFIG_COUNT", "1")]);
    assert!(rig.repo().join(".git").is_dir(), "git init and commit ran");
    let mut dry = DRAFT_ROUTE.to_vec();
    dry.push("--dry-run");
    let (code, text) = rig.run(&dry, &[("GIT_CONFIG_COUNT", "1")]);
    assert_eq!(code, 0, "{text}");
    assert_eq!(text.lines().filter(|l| l.starts_with("cost.")).count(), 24);
    // A real run reads the tree's state with git, and still does.
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("GIT_CONFIG_COUNT", "1")]);
    assert_eq!(code, 0, "{text}");
}

// ---- S3: the bound, the scope's judgement, the signal path and main ----------

impl V4Rig {
    /// `collect-v4`'s arguments, as `run` builds them.
    fn collect_args(&self, extra: &[&str]) -> Vec<String> {
        let mut args: Vec<String> = ["collect-v4", "--output"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        args.push(self.out().display().to_string());
        for t in Self::competitors() {
            args.extend(["--bin".to_owned(), format!("{t}=/bin/sh")]);
        }
        args.extend(
            [
                "--runtime",
                "greenmail=/never/jre",
                "--runtime",
                "alkemio-mailslurper=/never/musl",
            ]
            .map(str::to_owned),
        );
        args.extend(extra.iter().map(|s| (*s).to_owned()));
        args
    }

    /// A git command in the rig's repository, in the rig's own environment.
    fn git(&self, args: &[&str]) -> String {
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(self.repo())
            .args([
                "-c",
                "user.email=t@example.invalid",
                "-c",
                "user.name=t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args);
        own_git_environment(&mut cmd);
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }

    /// One more commit on the current branch.
    fn commit(&self, name: &str) {
        std::fs::write(self.repo().join(name), name).unwrap();
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", name]);
    }
}

/// Start `collect-v4` with the draft route and wait until `marker` appears,
/// which is the fake process saying that it is running; then signal the
/// wrapper with SIGTERM, run `after`, and return how the wrapper ended and
/// how long it took from the signal. No sleep places the signal.
fn signalled_at(
    rig: &V4Rig,
    env: &[(&str, &str)],
    marker: &Path,
    after: impl FnOnce(),
) -> (std::process::ExitStatus, Duration) {
    let mut child = rig
        .command(&rig.collect_args(&DRAFT_ROUTE), env)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "the fake never started");
        assert!(
            child.try_wait().unwrap().is_none(),
            "the wrapper ended before the fake was running"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let signalled = Instant::now();
    // SAFETY: signalling a child process this test owns.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    after();
    loop {
        if let Some(s) = child.try_wait().unwrap() {
            return (s, signalled.elapsed());
        }
        assert!(Instant::now() < deadline, "the wrapper did not end");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn process_is_gone(pid: i32) -> bool {
    // SAFETY: signal 0 only asks whether the process exists.
    unsafe { libc::kill(pid, 0) == -1 }
}

#[test]
fn no_collect_v4_timeout_line_carries_600() {
    let rig = v4_rig();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[
            ("COMPARE_MAIL_DRIVER_BIN", ""),
            ("COMPARE_MAIL_SWARMAIL_BIN", ""),
        ],
    );
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    let timeouts: Vec<&String> = log.iter().filter(|l| l.starts_with("timeout ")).collect();
    assert!(
        timeouts.len() >= 50,
        "every step and every query is bounded"
    );
    for l in timeouts {
        let words: Vec<&str> = l.split(' ').collect();
        assert!(
            !words[1..3].contains(&"600") && !l.contains("--kill-after=30 600"),
            "{l}"
        );
    }
}

#[test]
fn collect_v4_bounds_every_systemctl_call() {
    let rig = v4_rig();
    std::fs::write(rig.file("scope"), "cost.swarmail.r0 active\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_SCOPE", rig.file("scope").to_str().unwrap())],
    );
    assert_eq!(code, 2, "an unclean scope stops the run: {text}");
    let log = rig.log();
    let mut seen = std::collections::BTreeMap::new();
    for (i, l) in log.iter().enumerate() {
        let Some(call) = l.strip_prefix("systemctl ") else {
            continue;
        };
        let bounded = &log[i - 1];
        let want = if call.starts_with("--version") {
            3
        } else if call.starts_with("--user is-active") {
            1
        } else if call.starts_with("--user kill") || call.starts_with("--user stop") {
            3
        } else {
            panic!("a systemctl call nobody expected: {l}")
        };
        assert_eq!(
            bounded,
            &format!("timeout {want} systemctl {call}"),
            "line {i}: {log:?}"
        );
        seen.insert(call.split(' ').nth(1).unwrap_or("").to_owned(), want);
    }
    // All four kinds of call were made, so all four were checked.
    let kinds: Vec<&str> = seen.keys().map(String::as_str).collect();
    assert_eq!(kinds, ["", "is-active", "kill", "stop"], "{seen:?}");
}

#[test]
fn collect_v4_sums_stay_within_ten_minutes() {
    let rig = v4_rig();
    std::fs::write(rig.file("scope"), "cost.swarmail.r0 active\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_SCOPE", rig.file("scope").to_str().unwrap())],
    );
    assert_eq!(code, 2, "{text}");
    let log = rig.log();
    // The prefix's two numbers, from the line of the first chunk.
    let chunk = log
        .iter()
        .find(|l| l.starts_with("timeout --kill-after=") && l.contains("--chunk "))
        .unwrap();
    let words: Vec<&str> = chunk.split(' ').collect();
    let after: f64 = words[1]
        .trim_start_matches("--kill-after=")
        .parse()
        .unwrap();
    let term: f64 = words[2].parse().unwrap();
    // The three systemctl bounds, from the log.
    let bound_of = |what: &str| -> f64 {
        let l = log
            .iter()
            .find(|l| l.starts_with("timeout ") && l.contains(&format!("systemctl --user {what}")))
            .unwrap_or_else(|| panic!("no bounded {what} in {log:?}"));
        l.split(' ').nth(1).unwrap().parse().unwrap()
    };
    let (ask, kill, stop) = (bound_of("is-active"), bound_of("kill"), bound_of("stop"));
    // The poll count and interval, from the script.
    let script = std::fs::read_to_string(rig.repo().join("scripts/compare-mail.sh")).unwrap();
    let after_word = |key: &str| -> f64 {
        let at = script
            .find(key)
            .unwrap_or_else(|| panic!("no {key} in the script"));
        script[at + key.len()..]
            .split(|c: char| c != '.' && !c.is_ascii_digit())
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let (polls, interval) = (
        after_word("scope_polls="),
        after_word("scope_poll_interval="),
    );
    assert_eq!(
        (term, after),
        (protocol::CHUNK_KILL_MS as f64 / 1000.0, 30.0)
    );
    // 0132 section 11.1: ten minutes, for every kind of step.
    assert!(term + after <= 600.0, "a step");
    assert!(term + after + ask + kill + stop <= 600.0, "a failed chunk");
    assert!(
        term + polls * (ask + interval) + kill + stop <= 600.0,
        "a sealed chunk"
    );
    assert_eq!(term + after + ask + kill + stop, 597.0);
    assert_eq!(term + polls * (ask + interval) + kill + stop, 596.0);
}

fn scope_script(rig: &V4Rig, line: &str) -> String {
    std::fs::write(rig.file("scope"), format!("{line}\n")).unwrap();
    rig.file("scope").to_str().unwrap().to_owned()
}

#[test]
fn collect_v4_reads_a_failed_scope_as_gone() {
    let rig = v4_rig();
    // A scope whose leaf saw an OOM kill may end as `failed`: that is gone.
    let s = scope_script(&rig, "cost.mailpit.r0 failed");
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_SCOPE", &s)]);
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    assert_eq!(is_active_calls(&log, "cost.mailpit.r0"), 1, "{log:?}");
    assert!(!log.iter().any(|l| l.starts_with("systemctl --user kill")));
    assert!(rig.ledger().iter().all(|e| e["scope_clean"] == true));
}

#[test]
fn collect_v4_asks_again_after_a_query_that_says_nothing() {
    let rig = v4_rig();
    // A failed query, a silent one, then the answer: not a dirty scope.
    let s = scope_script(&rig, "cost.mailpit.r0 - - inactive");
    let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_SCOPE", &s)]);
    assert_eq!(code, 0, "{text}");
    let log = rig.log();
    assert_eq!(is_active_calls(&log, "cost.mailpit.r0"), 3, "{log:?}");
    assert!(!log.iter().any(|l| l.starts_with("systemctl --user kill")));
    assert_eq!(rig.ledger().len(), 24);
}

#[test]
fn collect_v4_never_calls_an_unknown_scope_clean() {
    for answer in ["active", "-", "reloading"] {
        let rig = v4_rig();
        let s = scope_script(&rig, &format!("cost.mailpit.r0 {answer}"));
        let (code, text) = rig.run(&DRAFT_ROUTE, &[("FAKE_SCOPE", &s)]);
        assert_eq!(code, 2, "{answer}: {text}");
        let log = rig.log();
        assert_eq!(
            is_active_calls(&log, "cost.mailpit.r0"),
            20,
            "{answer}: twenty queries and no more"
        );
        let ends: Vec<&str> = log
            .iter()
            .filter(|l| {
                l.starts_with("systemctl --user kill") || l.starts_with("systemctl --user stop")
            })
            .map(|l| l.split(' ').nth(2).unwrap())
            .collect();
        assert_eq!(ends, ["kill", "stop"], "{answer}: {log:?}");
        // The kill is SIGKILL: systemctl's default SIGTERM would leave a target
        // that ignores it alive until systemd's stop timeout, past the sums.
        let kills: Vec<&String> = log
            .iter()
            .filter(|l| l.starts_with("systemctl --user kill"))
            .collect();
        assert!(
            kills
                .iter()
                .all(|l| l.starts_with("systemctl --user kill --signal=KILL ")),
            "{answer}: {kills:?}"
        );
        let last = rig.ledger().pop().unwrap();
        assert_eq!(
            (
                last["coordinate"].as_str(),
                last["exit"].as_i64(),
                &last["scope_clean"]
            ),
            (Some("cost.mailpit.r0"), Some(2), &json!(false)),
            "{answer}"
        );
        assert_eq!(
            chunk_lines(&log, "driver run").len(),
            2,
            "{answer}: no later chunk"
        );
    }
}

#[test]
fn collect_v4_does_not_wait_for_a_failed_chunks_scope() {
    // A chunk that timed out and whose scope is still active: one query,
    // then the kill and the stop, in that order.
    let rig = v4_rig();
    std::fs::write(rig.file("timeouts"), "cost.mailpit.r0\n").unwrap();
    let s = scope_script(&rig, "cost.mailpit.r0 active");
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[
            ("FAKE_TIMEOUTS", rig.file("timeouts").to_str().unwrap()),
            ("FAKE_SCOPE", &s),
        ],
    );
    assert_eq!(code, 2, "{text}");
    let log = rig.log();
    let of_scope: Vec<&str> = log
        .iter()
        .filter(|l| l.starts_with("systemctl --user") && l.contains("cost.mailpit.r0"))
        .map(|l| l.split(' ').nth(2).unwrap())
        .collect();
    assert_eq!(of_scope, ["is-active", "kill", "stop"], "{log:?}");
    assert_eq!(rig.ledger().pop().unwrap()["scope_clean"], false);
    // Gone at the one query: no kill, no stop, and still a failed chunk.
    let rig = v4_rig();
    std::fs::write(rig.file("timeouts"), "cost.mailpit.r0\n").unwrap();
    let (code, text) = rig.run(
        &DRAFT_ROUTE,
        &[("FAKE_TIMEOUTS", rig.file("timeouts").to_str().unwrap())],
    );
    assert_eq!(code, 2, "{text}");
    let log = rig.log();
    assert_eq!(is_active_calls(&log, "cost.mailpit.r0"), 1, "{log:?}");
    assert!(
        !log.iter()
            .any(|l| l.starts_with("systemctl --user kill")
                || l.starts_with("systemctl --user stop"))
    );
    let last = rig.ledger().pop().unwrap();
    assert_eq!(
        (last["exit"].as_i64(), &last["scope_clean"]),
        (Some(2), &json!(true))
    );
}

/// What a signal does to a step the wrapper is waiting for: the mode ends
/// with exit 2 at once, not when the step would have ended, and the process
/// the fake stands for is gone.
fn signalled_step(stall: &str, bare: bool) -> (V4Rig, Duration) {
    let rig = v4_rig();
    let marker = rig.file("marker");
    let mut env = vec![
        ("FAKE_STALL", stall),
        ("FAKE_MARKER", marker.to_str().unwrap()),
    ];
    if bare {
        env.extend([
            ("COMPARE_MAIL_DRIVER_BIN", ""),
            ("COMPARE_MAIL_SWARMAIL_BIN", ""),
        ]);
    }
    let (status, took) = signalled_at(&rig, &env, &marker, || {});
    assert_eq!(status.code(), Some(2), "{stall}: {status:?}");
    let pid: i32 = std::fs::read_to_string(&marker)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(process_is_gone(pid), "{stall}: the fake outlived the mode");
    // The fake would have run for 120 s.
    assert!(took < Duration::from_secs(30), "{stall}: {took:?}");
    (rig, took)
}

#[test]
fn collect_v4_on_sigterm_during_a_build_exits_two_at_once() {
    for stall in ["cargo-build", "cargo-bench"] {
        let (rig, _) = signalled_step(stall, true);
        assert!(!rig.out().exists(), "{stall}: nothing was made");
        assert!(chunk_lines(&rig.log(), "driver run").is_empty(), "{stall}");
    }
}

#[test]
fn collect_v4_on_sigterm_during_check_footprint_and_admission_exits_two_at_once() {
    for stall in ["targets", "check", "footprint", "admit-v4"] {
        let (rig, _) = signalled_step(stall, false);
        let log = rig.log();
        let ran = chunk_lines(&log, "driver run").len();
        match stall {
            "targets" | "check" | "footprint" => assert_eq!(ran, 0, "{stall}: {log:?}"),
            _ => {
                // The admission is after every chunk, and the run's own
                // files were not removed by the interrupt.
                assert_eq!(ran, 24, "{stall}");
                assert!(rig.out().join("reproduce.txt").is_file());
            }
        }
        assert!(
            !rig.out().join("index.json").exists(),
            "{stall}: nothing was sealed"
        );
    }
}

#[test]
fn collect_v4_on_sigterm_during_the_scope_judgement_still_writes_the_ledger() {
    let rig = v4_rig();
    let marker = rig.file("scope-marker");
    // The chunk has ended and is sealed: the first question about its scope
    // does not answer until the test has signalled the wrapper.
    let s = scope_script(&rig, "cost.swarmail.r0 wait");
    let release = rig.file("scope-marker.release");
    let (status, _) = signalled_at(
        &rig,
        &[
            ("FAKE_SCOPE", &s),
            ("FAKE_SCOPE_MARKER", marker.to_str().unwrap()),
        ],
        &marker,
        || std::fs::write(&release, "go").unwrap(),
    );
    assert_eq!(status.code(), Some(2), "{status:?}");
    let ledger = rig.ledger();
    assert_eq!(ledger.len(), 1, "{ledger:?}");
    assert_eq!(
        (
            ledger[0]["coordinate"].as_str(),
            ledger[0]["exit"].as_i64(),
            &ledger[0]["scope_clean"]
        ),
        (Some("cost.swarmail.r0"), Some(2), &json!(false)),
        "the chunk was not abandoned: it is ledgered as failed"
    );
    let log = rig.log();
    assert!(
        log.iter()
            .any(|l| l.starts_with("systemctl --user kill") && l.contains("cost.swarmail.r0")),
        "a scope that was not gone is killed: {log:?}"
    );
    assert!(
        log.iter()
            .any(|l| l.starts_with("systemctl --user stop") && l.contains("cost.swarmail.r0")),
        "and stopped: {log:?}"
    );
    assert_eq!(chunk_lines(&log, "driver run").len(), 1, "no later chunk");
    assert!(chunk_lines(&log, "driver admit-v4").is_empty());
}

#[test]
fn collect_v4_refuses_a_clean_branch_that_is_not_main() {
    let rig = v4_rig();
    rig.git(&["checkout", "-q", "-b", "feature"]);
    rig.commit("feature.txt");
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("blocked at phase \"candidate\"")
            && text.contains("refs/heads/main")
            && text.contains("a commit of main only"),
        "{text}"
    );
    assert!(rig.log().is_empty(), "before anything was built or run");
    assert!(!rig.out().exists());
}

#[test]
fn collect_v4_refuses_a_branch_with_main_merged_in() {
    let rig = v4_rig();
    rig.git(&["checkout", "-q", "-b", "feature"]);
    rig.commit("feature.txt");
    rig.git(&["checkout", "-q", "main"]);
    rig.commit("later-on-main.txt");
    rig.git(&["checkout", "-q", "feature"]);
    rig.git(&["merge", "-q", "--no-edit", "main"]);
    // The branch has everything main has, and is still not in its history.
    assert_eq!(
        rig.git(&["merge-base", "main", "HEAD"]),
        rig.git(&["rev-parse", "main"])
    );
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("not in the history of refs/heads/main"),
        "{text}"
    );
}

#[test]
fn collect_v4_accepts_an_earlier_commit_of_main_and_its_tip() {
    let rig = v4_rig();
    rig.commit("second.txt");
    rig.commit("third.txt");
    // Detached at the tip, and at an earlier commit: main has moved on, and
    // a published run must still be repeatable.
    for at in ["main", "main~2"] {
        rig.git(&["checkout", "-q", "--detach", at]);
        let rig_out = rig.out();
        let _ = std::fs::remove_dir_all(&rig_out);
        let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
        assert_eq!(code, 0, "{at}: {text}");
    }
}

#[test]
fn collect_v4_refuses_a_head_origin_main_does_not_have() {
    let rig = v4_rig();
    rig.commit("second.txt");
    // The remote's main is one commit behind the local one.
    rig.git(&["update-ref", "refs/remotes/origin/main", "main~1"]);
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("not in the history of refs/remotes/origin/main"),
        "{text}"
    );
    // At the same commit, both refs agree and the run goes on.
    rig.git(&["update-ref", "refs/remotes/origin/main", "main"]);
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 0, "{text}");
}

#[test]
fn collect_v4_refuses_when_no_main_ref_exists() {
    let rig = v4_rig();
    rig.git(&["branch", "-q", "-m", "main", "trunk"]);
    let (code, text) = rig.run(&DRAFT_ROUTE, &[]);
    assert_eq!(code, 2, "{text}");
    assert!(
        text.contains("neither refs/heads/main nor refs/remotes/origin/main exists"),
        "{text}"
    );
    assert!(rig.log().is_empty());
}

#[test]
fn collect_v4_dry_run_does_not_check_main() {
    let rig = v4_rig();
    rig.git(&["checkout", "-q", "-b", "feature"]);
    rig.commit("feature.txt");
    let mut dry = DRAFT_ROUTE.to_vec();
    dry.push("--dry-run");
    let (code, text) = rig.run(&dry, &[]);
    assert_eq!(code, 0, "{text}");
    assert_eq!(text.lines().filter(|l| l.starts_with("cost.")).count(), 24);
}

// ============================================================================
// R1 to R3: the chunk's own code, against a real target on a fixture scope
//
// A fixture scope is a directory of plain files that a fake kernel keeps like
// cgroupfs does for what the driver reads: a new leaf appears with its control
// files, `cgroup.kill` empties `cgroup.procs`, and a leaf can be removed with
// what is in it. It cannot move a real process, so the scope trusts the join's
// cgroup read-back (a test-only seam); the CPUs the target may run on are
// still read from `/proc`, where the child has set them.
// ============================================================================

#[cfg(target_os = "linux")]
mod chunk {
    use super::*;
    use records::{ResourceRow, WindowKind, WindowOutcome};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    pub struct FakeKernel {
        killed: Mutex<BTreeSet<PathBuf>>,
        oom_kill: u64,
    }

    impl FakeKernel {
        pub fn new(oom_kill: u64) -> Arc<FakeKernel> {
            Arc::new(FakeKernel {
                killed: Mutex::new(BTreeSet::new()),
                oom_kill,
            })
        }
    }

    impl resources::Tree for FakeKernel {
        fn read(&self, p: &Path) -> std::io::Result<String> {
            let emptied = p.file_name().is_some_and(|n| n == "cgroup.procs")
                && p.parent()
                    .is_some_and(|d| self.killed.lock().unwrap().contains(d));
            if emptied {
                return Ok(String::new());
            }
            std::fs::read_to_string(p)
        }

        fn write(&self, p: &Path, text: &str) -> std::io::Result<()> {
            if p.file_name().is_some_and(|n| n == "cgroup.kill")
                && let Some(d) = p.parent()
            {
                self.killed.lock().unwrap().insert(d.to_owned());
            }
            resources::host().write(p, text)
        }

        fn exists(&self, p: &Path) -> bool {
            resources::host().exists(p)
        }

        fn mkdir(&self, p: &Path) -> std::io::Result<()> {
            std::fs::create_dir(p)?;
            let events = format!(
                "low 0\nhigh 0\nmax 0\noom {k}\noom_kill {k}\n",
                k = self.oom_kill
            );
            for (f, text) in [
                ("memory.max", "max\n"),
                ("memory.swap.max", "max\n"),
                ("memory.oom.group", "0\n"),
                ("cpu.max", "max 100000\n"),
                ("pids.max", "max\n"),
                ("cgroup.kill", ""),
                ("memory.peak", "1048576\n"),
                ("cgroup.procs", ""),
                ("memory.current", "4096\n"),
                ("memory.stat", "anon 1\nfile 2\nkernel 3\n"),
                ("memory.events", events.as_str()),
                ("cpu.stat", CPU_STAT),
                ("pids.current", "1\n"),
            ] {
                std::fs::write(p.join(f), text)?;
            }
            Ok(())
        }

        fn rmdir(&self, p: &Path) -> std::io::Result<()> {
            std::fs::remove_dir_all(p)
        }
    }

    fn layout() -> resources::Layout {
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let allowed: Vec<u32> = resources::parse_cpus_allowed(&status)
            .unwrap()
            .into_iter()
            .collect();
        assert!(allowed.len() >= 2, "a fixture scope needs two allowed CPUs");
        resources::Layout {
            target_cpus: allowed[..2].to_vec(),
            idle_siblings: Vec::new(),
            client_cpus: allowed[2..].to_vec(),
            siblings_outside_set: Vec::new(),
        }
    }

    pub struct Rig {
        pub dir: tempfile::TempDir,
        pub scope: Arc<resources::Scope>,
        pub keepers: targets::Keepers,
    }

    impl Rig {
        /// A fixture scope; `oom_kill` is what each new leaf's
        /// `memory.events` already counts, `trusting` whether the join's
        /// cgroup read-back is taken as the leaf.
        pub fn new(oom_kill: u64, trusting: bool) -> Rig {
            let dir = fixture();
            let scope_dir = dir.path().join("scope");
            std::fs::create_dir(&scope_dir).unwrap();
            let mut scope = resources::Scope::new(FakeKernel::new(oom_kill), scope_dir, layout());
            if trusting {
                scope = scope.trusting_the_join();
            }
            Rig {
                dir,
                scope: Arc::new(scope),
                keepers: targets::Keepers::default(),
            }
        }

        pub fn target_cpus(&self) -> Vec<u32> {
            self.scope.layout.target_cpus.clone()
        }

        pub fn scope_dir(&self) -> PathBuf {
            self.dir.path().join("scope")
        }

        /// A launch spec for `binary` as the Swarmail candidate, in a leaf
        /// of this scope.
        pub fn spec(&self, p: &Protocol, binary: &Path) -> targets::LaunchSpec {
            let sbytes = serde_json::to_vec(&lock5()).unwrap();
            let lock = SourceLock::parse(&sbytes).unwrap();
            let bins = vec![format!("swarmail={}", binary.display())];
            let mut specs =
                runner::launch_specs_of(p, &lock, &bins, &[], &[TargetKind::Swarmail]).unwrap();
            let mut spec = specs.remove(&TargetKind::Swarmail).unwrap();
            spec.extra.scope = Some(self.scope.clone());
            spec
        }

        /// The chunk `cost.swarmail.r0` of `p` against `binary`.
        pub fn config(
            &self,
            p: &Protocol,
            binary: &Path,
            run: &str,
        ) -> (runner::Config, runner::Chunk) {
            let chunk = runner::Chunk::Cost {
                target: TargetKind::Swarmail,
                repetition: 0,
            };
            let mut m = manifest(p, run);
            let (planned, windows) = runner::plan_chunk(p, &chunk);
            (m.planned, m.planned_windows, m.chunk) = (planned, Some(windows), Some(chunk.name()));
            let scratch = self.dir.path().join("scratch");
            std::fs::create_dir(&scratch).unwrap();
            let cfg = runner::Config {
                protocol: p.clone(),
                manifest: m,
                specs: [(TargetKind::Swarmail, self.spec(p, binary))].into(),
                output: self.dir.path().join("out"),
                scratch,
                keepers: self.keepers.clone(),
            };
            (cfg, chunk)
        }
    }

    pub fn profile(name: &str, warmup: u32, messages: u32, sessions: u32) -> Value {
        json!({
            "name": name, "repetitions": 1, "warmup_messages": warmup,
            "messages": messages, "sessions": sessions, "connection": "reused",
            "send_deadline_ms": 300000, "verify_deadline_ms": 60000
        })
    }

    /// The smoke draft with these profiles and this chunk deadline. It is
    /// parsed, not validated: the deadline is the test's to place.
    pub fn protocol(profiles: Vec<Value>, deadline_ms: u64) -> Protocol {
        let mut v = json(V4_SMOKE);
        v["throughput"] = json!(profiles);
        v["resources"]["chunk_deadline_ms"] = json!(deadline_ms);
        Protocol::parse(&serde_json::to_vec(&v).unwrap()).unwrap()
    }

    /// A candidate that stays up briefly and exits: it never announces a
    /// listener, so it is never ready, and it has exited when the launch is
    /// torn down.
    pub fn exits_soon(dir: &Path) -> PathBuf {
        let path = dir.join("exits-soon");
        std::fs::write(&path, "#!/bin/sh\nsleep 0.3\nexit 1\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    pub fn windows(res: &[ResourceRow]) -> Vec<&records::WindowRow> {
        res.iter()
            .filter_map(|r| match r {
                ResourceRow::Window(w) => Some(w),
                ResourceRow::Sample(_) => None,
            })
            .collect()
    }

    pub fn window<'a>(
        res: &'a [ResourceRow],
        profile: &str,
        kind: WindowKind,
    ) -> &'a records::WindowRow {
        windows(res)
            .into_iter()
            .find(|w| w.profile == profile && w.window == kind)
            .unwrap_or_else(|| panic!("no {kind:?} window of {profile}"))
    }

    pub fn sends(rows: &[Row]) -> Vec<&Row> {
        rows.iter().filter(|r| r.phase == Phase::Send).collect()
    }

    /// A scripted SMTP server that accepts every message, or stops answering
    /// at the first command that begins with `hang_on` (empty: never).
    pub async fn smtp_stub(hang_on: &'static str) -> std::net::SocketAddr {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((s, _)) = l.accept().await {
                tokio::spawn(async move {
                    let (r, mut w) = s.into_split();
                    let mut r = tokio::io::BufReader::new(r);
                    let _ = w.write_all(b"220 stub\r\n").await;
                    let mut line = String::new();
                    loop {
                        line.clear();
                        if r.read_line(&mut line).await.unwrap_or(0) == 0 {
                            return;
                        }
                        let up = line.to_ascii_uppercase();
                        if !hang_on.is_empty() && up.starts_with(hang_on) {
                            std::future::pending::<()>().await;
                        }
                        let reply: &[u8] = if up.starts_with("DATA") {
                            let _ = w.write_all(b"354 go\r\n").await;
                            loop {
                                line.clear();
                                if r.read_line(&mut line).await.unwrap_or(0) == 0 {
                                    return;
                                }
                                if line == ".\r\n" {
                                    break;
                                }
                            }
                            b"250 queued\r\n"
                        } else if up.starts_with("QUIT") {
                            b"221 bye\r\n"
                        } else {
                            b"250 ok\r\n"
                        };
                        let _ = w.write_all(reply).await;
                    }
                });
            }
        });
        addr
    }

    /// A Mailpit read path listing `messages` (id, subject) at once, and
    /// holding every raw fetch without an answer when `hang_fetch`.
    pub async fn mailpit_http(
        messages: Vec<(String, String)>,
        hang_fetch: bool,
    ) -> std::net::SocketAddr {
        super::super::responder(Box::new(move |target: &str| {
            if target.contains("/raw") && hang_fetch {
                return None;
            }
            let items: Vec<Value> = messages
                .iter()
                .map(|(id, subject)| json!({"ID": id, "Subject": subject}))
                .collect();
            Some((
                200,
                json!({"start": 0, "messages_count": items.len(), "messages": items}).to_string(),
            ))
        }))
        .await
    }

    /// A detached Mailpit whose SMTP side is `smtp` and read path `http`.
    pub fn detached(
        smtp: std::net::SocketAddr,
        http: std::net::SocketAddr,
        p: &Protocol,
    ) -> targets::Owned {
        let mut o = targets::Owned::fixture(TargetKind::Mailpit, http, None, &runner::limits(p));
        o.smtp = smtp;
        o
    }

    /// A manifest that plans the rows of one Mailpit chunk and no windows,
    /// for the stages that write rows and no windows.
    pub fn rows_only(p: &Protocol, run: &str) -> Manifest {
        let mut m = tiny_manifest(p, run, &mailpit_chunk(0));
        m.planned_windows = None;
        m
    }

    pub fn sink_for(m: &Manifest, out: &Path) -> runner::Sink {
        Arc::new(Mutex::new(records::RunDir::create(out, m).unwrap()))
    }

    pub fn read_rows(out: &Path, m: &Manifest, sink: runner::Sink) -> Vec<Row> {
        drop(sink);
        records::read_rows(out, m).expect("the rows read back")
    }

    pub fn subject_of(p: &Protocol, tag: &str, i: u32) -> String {
        wire::authored(p.corpus.seed, tag, i, &p.corpus.kinds).subject
    }

    // ---- R1.2 ----------------------------------------------------------------

    #[tokio::test]
    async fn a_chunk_whose_deadline_has_passed_launches_nothing_and_seals_with_exit_one() {
        let rig = Rig::new(0, true);
        let p = protocol(
            vec![profile("cost-a", 3, 6, 2), profile("cost-b", 0, 6, 2)],
            1,
        );
        let (cfg, chunk) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-d1");
        let manifest = cfg.manifest.clone();
        // A clock whose origin is older than the deadline: the instant has
        // come before the chunk starts.
        let old = Instant::now()
            .checked_sub(Duration::from_secs(5))
            .expect("the host has been up five seconds");
        let (verdict, life, sealed) = runner::run_chunk(cfg, chunk, wire::Clock(old)).await;
        assert_eq!(verdict.exit_code(), 1, "{verdict:?}");
        let runner::Verdict::WorkloadFailures(failures) = verdict else {
            panic!("a chunk with unmeasured profiles is a workload failure")
        };
        assert_eq!(failures.len(), 2);
        assert!(failures.iter().all(|f| f.contains("chunk deadline")));
        assert!(
            life.launches.is_empty() && life.cleanups.is_empty(),
            "nothing was launched"
        );
        let sealed = sealed.expect("the chunk is sealed");
        assert!(sealed.join("index.json").exists());
        let (rows, res) =
            records::read_run(&sealed, &manifest).expect("every planned row and window");
        assert_eq!(sends(&rows).len(), 12);
        assert!(sends(&rows).iter().all(|r| {
            r.outcome == "unstarted" && r.reason.as_deref() == Some("chunk deadline")
        }));
        assert_eq!(windows(&res).len(), 6, "every planned window once");
        for w in windows(&res) {
            assert_eq!(w.outcome, WindowOutcome::ChunkDeadline);
            assert_eq!((w.launch.as_str(), w.counters().is_none()), ("none", true));
        }
        assert!(
            std::fs::read_dir(rig.scope_dir()).unwrap().next().is_none(),
            "no leaf was made"
        );
    }

    #[tokio::test]
    async fn a_send_window_past_chunk_end_is_all_unstarted_and_lists_nothing() {
        let rig = Rig::new(0, true);
        let p = protocol(vec![profile("cost-a", 0, 6, 2)], 480_000);
        let (cfg, _) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-d2");
        let (manifest, out) = (cfg.manifest.clone(), cfg.output.clone());
        let sink = sink_for(&manifest, &out);
        let mut life = runner::Lifecycle {
            launches: vec![],
            cleanups: vec![],
        };
        let spec = &cfg.specs[&TargetKind::Swarmail];
        let clock = wire::Clock(Instant::now());
        // The instant is already here when the stage is entered.
        let failures = runner::measured_instance(
            (spec, &rig.target_cpus()),
            &cfg,
            &runner::limits(&p),
            &clock,
            (0, &p.throughput[0], 0),
            1,
            &sink,
            &mut life,
            Instant::now(),
        )
        .await
        .expect("a cut profile is a workload failure, not an infrastructure one");
        assert!(!failures.is_empty());
        drop(sink);
        let (rows, res) = records::read_run(&out, &manifest).unwrap();
        assert_eq!(sends(&rows).len(), 6, "every index has exactly one row");
        assert!(sends(&rows).iter().all(|r| {
            r.outcome == "unstarted" && r.reason.as_deref() == Some("chunk deadline")
        }));
        assert!(
            rows.iter().all(|r| r.phase != Phase::Visible),
            "nothing was listed"
        );
        let v = rows.iter().find(|r| r.phase == Phase::Verified).unwrap();
        assert_eq!(
            (v.outcome.as_str(), v.reason.as_deref()),
            ("incomplete", Some("chunk deadline"))
        );
        assert_eq!(v.requests, Some(0), "the target was sent no request");
        let ready = window(&res, "cost-a", WindowKind::Ready);
        assert_eq!(ready.outcome, WindowOutcome::Measured);
        assert!(ready.counters().is_some());
        for kind in [WindowKind::IdleReady, WindowKind::Measure] {
            let w = window(&res, "cost-a", kind);
            assert_eq!(w.outcome, WindowOutcome::ChunkDeadline, "{kind:?}");
            assert!(w.counters().is_none(), "{kind:?} was never opened");
            assert_eq!(
                w.launch, ready.launch,
                "{kind:?} is the launched instance's"
            );
        }
        assert_eq!(life.launches.len(), 1);
        assert!(
            life.cleanups
                .iter()
                .all(|c| c.proved() && c.cgroup_removed == Some(true))
        );
    }

    // ---- S4.1: no QUIT after the chunk's instant ------------------------------

    /// What a test-owned SMTP listener has received, line by line, with
    /// `<eof>` where a connection ended.
    pub type Heard = Arc<Mutex<Vec<String>>>;

    /// A server that accepts every message and records every line it is
    /// sent. With `hang_quit` it never answers `QUIT`. With `hold_until`, the
    /// first time it has answered a message's terminator it blocks its
    /// (single) runtime thread until that instant: the client cannot run in
    /// between, so the instant falls after the acceptance and before the
    /// client's next step, whatever the scheduler does.
    pub async fn smtp_recording(
        hang_quit: bool,
        hold_until: Option<Instant>,
    ) -> (std::net::SocketAddr, Heard) {
        let heard: Heard = Arc::default();
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        let (log, held) = (
            heard.clone(),
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        );
        tokio::spawn(async move {
            while let Ok((s, _)) = l.accept().await {
                let (log, held) = (log.clone(), held.clone());
                tokio::spawn(async move {
                    let (r, mut w) = s.into_split();
                    let mut r = tokio::io::BufReader::new(r);
                    let _ = w.write_all(b"220 stub\r\n").await;
                    let mut line = String::new();
                    loop {
                        line.clear();
                        if r.read_line(&mut line).await.unwrap_or(0) == 0 {
                            log.lock().unwrap().push("<eof>".into());
                            return;
                        }
                        log.lock().unwrap().push(line.trim_end().to_owned());
                        let up = line.to_ascii_uppercase();
                        if up.starts_with("QUIT") && hang_quit {
                            std::future::pending::<()>().await;
                        }
                        let reply: &[u8] = if up.starts_with("DATA") {
                            let _ = w.write_all(b"354 go\r\n").await;
                            loop {
                                line.clear();
                                if r.read_line(&mut line).await.unwrap_or(0) == 0 {
                                    return;
                                }
                                if line == ".\r\n" {
                                    break;
                                }
                            }
                            b"250 queued\r\n"
                        } else if up.starts_with("QUIT") {
                            b"221 bye\r\n"
                        } else {
                            b"250 ok\r\n"
                        };
                        let _ = w.write_all(reply).await;
                        if let Some(t) = hold_until
                            && reply == b"250 queued\r\n"
                            && !held.swap(true, std::sync::atomic::Ordering::SeqCst)
                        {
                            while Instant::now() < t {
                                std::thread::sleep(Duration::from_millis(5));
                            }
                        }
                    }
                });
            }
        });
        (addr, heard)
    }

    /// Wait until the listener has seen `n` connections end.
    pub async fn ended(heard: &Heard, n: usize) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let lines = heard.lock().unwrap().clone();
            if lines.iter().filter(|l| *l == "<eof>").count() >= n {
                return lines;
            }
            assert!(Instant::now() < deadline, "no end of connection: {lines:?}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn quits(lines: &[String]) -> usize {
        lines.iter().filter(|l| l.starts_with("QUIT")).count()
    }

    /// One send of a profile against `smtp`, with the chunk's instant `end`,
    /// returning the rows and how long it took.
    async fn one_send(
        connection: &str,
        smtp: std::net::SocketAddr,
        end: Option<Instant>,
        run: &str,
    ) -> (Vec<Row>, Duration) {
        let mut pr = profile("cost-a", 0, 1, 1);
        pr["connection"] = json!(connection);
        pr["verify_deadline_ms"] = json!(200);
        let p = protocol(vec![pr], 480_000);
        let owned = detached(smtp, mailpit_http(vec![], false).await, &p);
        let m = rows_only(&p, run);
        let dir = fixture();
        let out = dir.path().join("run");
        let sink = sink_for(&m, &out);
        let clock = wire::Clock(Instant::now());
        let started = Instant::now();
        runner::throughput(
            &owned,
            &p,
            &p.throughput[0],
            "quittest",
            0,
            run,
            &sink,
            &clock,
            (None, end),
        )
        .await
        .unwrap();
        let took = started.elapsed();
        (read_rows(&out, &m, sink), took)
    }

    #[tokio::test]
    async fn a_reused_session_past_chunk_end_is_dropped_without_quit() {
        // The acceptance comes before the instant; the instant is here by the
        // time the worker would end its session. The 5 s window only has to
        // outlast setup: at 400 ms an instrumented coverage build had not yet
        // started the send (verify, 2026-11-17); the server's hold still puts
        // the instant between acceptance and the client's next step.
        let end = Instant::now() + Duration::from_secs(5);
        let (smtp, heard) = smtp_recording(false, Some(end + Duration::from_millis(100))).await;
        let (rows, _) = one_send("reused", smtp, Some(end), "run-q1").await;
        assert_eq!(sends(&rows)[0].outcome, "accepted");
        let lines = ended(&heard, 1).await;
        assert_eq!(quits(&lines), 0, "{lines:?}");
    }

    #[tokio::test]
    async fn a_fresh_session_past_chunk_end_sends_no_quit_and_keeps_its_accepted_row() {
        // 5 s only has to outlast setup (see the reused-session case).
        let end = Instant::now() + Duration::from_secs(5);
        let (smtp, heard) = smtp_recording(false, Some(end + Duration::from_millis(100))).await;
        let (rows, _) = one_send("fresh", smtp, Some(end), "run-q2").await;
        let s = sends(&rows);
        assert_eq!(s.len(), 1);
        assert_eq!(
            (s[0].outcome.as_str(), s[0].reason.as_deref()),
            ("accepted", None),
            "the row keeps what the server said"
        );
        let lines = ended(&heard, 1).await;
        assert_eq!(quits(&lines), 0, "{lines:?}");
    }

    #[tokio::test]
    async fn a_quit_in_flight_is_cut_at_chunk_end() {
        let (smtp, heard) = smtp_recording(true, None).await;
        // 5 s only has to outlast setup and the send before QUIT begins.
        let end = Instant::now() + Duration::from_secs(5);
        let (rows, took) = one_send("reused", smtp, Some(end), "run-q3").await;
        assert_eq!(sends(&rows)[0].outcome, "accepted");
        // The listener never answers QUIT: the call returned at the instant,
        // not one request deadline (10 s) after the QUIT.
        assert!(took < Duration::from_secs(9), "{took:?}");
        let lines = heard.lock().unwrap().clone();
        assert_eq!(
            quits(&lines),
            1,
            "it was begun before the instant: {lines:?}"
        );
    }

    #[tokio::test]
    async fn before_chunk_end_a_session_still_ends_with_quit() {
        let (smtp, heard) = smtp_recording(false, None).await;
        let end = Instant::now() + Duration::from_secs(60);
        let (rows, _) = one_send("reused", smtp, Some(end), "run-q4").await;
        assert_eq!(sends(&rows)[0].outcome, "accepted");
        let lines = ended(&heard, 1).await;
        assert_eq!(quits(&lines), 1, "{lines:?}");
    }

    #[tokio::test]
    async fn a_protocol_3_session_still_ends_with_quit() {
        // No instant: protocols 1 to 3 end a session as they always did.
        for connection in ["reused", "fresh"] {
            let (smtp, heard) = smtp_recording(false, None).await;
            let (rows, _) = one_send(connection, smtp, None, "run-q5").await;
            assert_eq!(sends(&rows)[0].outcome, "accepted", "{connection}");
            let lines = ended(&heard, 1).await;
            assert_eq!(quits(&lines), 1, "{connection}: {lines:?}");
        }
    }

    /// The warmup's sessions keep V16's rule too: an acceptance before the
    /// instant, the instant here before the client's next step, no `QUIT`.
    #[tokio::test]
    async fn a_warmup_session_past_chunk_end_sends_no_quit() {
        // 5 s only has to outlast setup (see the reused-session case).
        let end = Instant::now() + Duration::from_secs(5);
        let (smtp, heard) = smtp_recording(false, Some(end + Duration::from_millis(100))).await;
        let p = protocol(vec![profile("cost-a", 1, 1, 1)], 480_000);
        let clock = wire::Clock(Instant::now());
        let cut =
            runner::warmup_sends(smtp, &p, 2, Duration::from_secs(10), &clock, Some(end)).await;
        assert!(cut, "the second send never began");
        let lines = ended(&heard, 1).await;
        assert_eq!(quits(&lines), 0, "{lines:?}");
        assert_eq!(
            lines.iter().filter(|l| *l == "DATA").count(),
            1,
            "one send, accepted before the instant: {lines:?}"
        );
    }

    #[tokio::test]
    async fn before_chunk_end_a_warmup_session_still_ends_with_quit() {
        for end in [None, Some(Instant::now() + Duration::from_secs(60))] {
            let (smtp, heard) = smtp_recording(false, None).await;
            let p = protocol(vec![profile("cost-a", 1, 1, 1)], 480_000);
            let clock = wire::Clock(Instant::now());
            let cut = runner::warmup_sends(smtp, &p, 2, Duration::from_secs(10), &clock, end).await;
            assert!(!cut, "{end:?}");
            let lines = ended(&heard, 2).await;
            assert_eq!(quits(&lines), 2, "{end:?}: {lines:?}");
        }
    }

    // ---- S4.2, S4.3 ------------------------------------------------------------

    #[test]
    fn a_never_opened_window_keeps_the_order_of_outcomes() {
        for (oom_kill, exited, want) in [
            (1, false, WindowOutcome::OomKilled),
            (1, true, WindowOutcome::OomKilled),
            (0, true, WindowOutcome::TargetExited),
            (0, false, WindowOutcome::ChunkDeadline),
        ] {
            let rig = Rig::new(oom_kill, true);
            let (leaf, _) = rig.scope.leaf().unwrap();
            assert_eq!(
                runner::never_opened_outcome(&leaf, exited, WindowOutcome::ChunkDeadline),
                want,
                "oom_kill {oom_kill}, exited {exited}"
            );
        }
    }

    #[test]
    fn a_join_under_another_parent_with_the_leafs_name_is_a_mismatch() {
        let rig = Rig::new(0, true);
        let (leaf, _) = rig.scope.leaf().unwrap();
        let proc_root = fixture();
        let pid = proc_root.path().join("77");
        std::fs::create_dir(&pid).unwrap();
        std::fs::write(pid.join("status"), "Cpus_allowed_list:\t2,3\n").unwrap();
        // The same last component, under another parent.
        std::fs::write(
            pid.join("cgroup"),
            format!("0::/user.slice/another.scope/{}\n", leaf.name()),
        )
        .unwrap();
        let read = resources::read_join_whole(proc_root.path(), 77).unwrap();
        assert_eq!(read.joined.cgroup, leaf.name(), "the name alone would pass");
        let w = resources::check_join_whole(&read, (leaf.name(), &leaf.cgroup_path()), &[2, 3])
            .unwrap_err();
        assert!(
            w.contains("not its leaf") && w.contains("another.scope"),
            "{w}"
        );
    }

    #[test]
    fn a_join_at_the_leafs_own_path_passes_and_the_receipt_keeps_the_name() {
        let rig = Rig::new(0, true);
        let (leaf, _) = rig.scope.leaf().unwrap();
        let proc_root = fixture();
        let pid = proc_root.path().join("78");
        std::fs::create_dir(&pid).unwrap();
        std::fs::write(pid.join("status"), "Cpus_allowed_list:\t2,3\n").unwrap();
        std::fs::write(pid.join("cgroup"), format!("0::{}\n", leaf.cgroup_path())).unwrap();
        let read = resources::read_join_whole(proc_root.path(), 78).unwrap();
        resources::check_join_whole(&read, (leaf.name(), &leaf.cgroup_path()), &[2, 3]).unwrap();
        // The receipt carries the leaf's name and nothing of the path.
        assert_eq!(read.joined.cgroup, "target-0001");
        let receipt = serde_json::to_value(&read.joined).unwrap();
        assert_eq!(
            receipt,
            json!({"cgroup": "target-0001", "cpus_allowed": [2, 3]})
        );
        // The path as /proc writes it: below the cgroup mount, from a slash.
        assert_eq!(
            resources::cgroup_path_of(
                Path::new("/sys/fs/cgroup/user.slice/mail-compare-x.scope/target-0001"),
                Path::new("/sys/fs/cgroup")
            ),
            "/user.slice/mail-compare-x.scope/target-0001"
        );
        assert_eq!(
            resources::cgroup_path_of(
                Path::new("/tmp/fixture/target-0001"),
                Path::new("/sys/fs/cgroup")
            ),
            "/tmp/fixture/target-0001"
        );
    }

    // ---- R1.3 ----------------------------------------------------------------

    #[tokio::test]
    async fn a_send_in_flight_is_cancelled_at_chunk_end() {
        let p = protocol(vec![profile("cost-a", 0, 3, 1)], 480_000);
        // Accepts, greets, answers EHLO, and never answers MAIL FROM.
        let owned = detached(
            smtp_stub("MAIL").await,
            mailpit_http(vec![], false).await,
            &p,
        );
        let m = rows_only(&p, "run-d3");
        let dir = fixture();
        let out = dir.path().join("run");
        let sink = sink_for(&m, &out);
        let clock = wire::Clock(Instant::now());
        let started = Instant::now();
        let end = started + Duration::from_millis(1500);
        let failures = runner::throughput(
            &owned,
            &p,
            &p.throughput[0],
            "cuttest",
            0,
            "run-d3",
            &sink,
            &clock,
            (None, Some(end)),
        )
        .await
        .unwrap();
        let request = Duration::from_millis(p.resources.request_deadline_ms);
        assert!(
            started.elapsed() < request / 2,
            "returned at the instant, not a request deadline later: {:?}",
            started.elapsed()
        );
        assert!(!failures.is_empty());
        let rows = read_rows(&out, &m, sink);
        let s = sends(&rows);
        assert_eq!(s.len(), 3, "the rows count the profile's messages exactly");
        let cut: Vec<&&Row> = s.iter().filter(|r| r.outcome == "ambiguous").collect();
        assert_eq!(cut.len(), 1, "one send was in flight");
        assert!(cut[0].reason.as_deref().unwrap().contains("chunk deadline"));
        assert!(s.iter().filter(|r| r.outcome != "ambiguous").all(|r| {
            r.outcome == "unstarted" && r.reason.as_deref() == Some("chunk deadline")
        }));
        let v = rows.iter().find(|r| r.phase == Phase::Verified).unwrap();
        assert_eq!(
            (v.outcome.as_str(), v.reason.as_deref()),
            ("incomplete", Some("chunk deadline"))
        );
        assert!(
            rows.iter().all(|r| r.phase != Phase::Visible),
            "no listing after the instant"
        );
    }

    #[tokio::test]
    async fn a_connect_in_flight_is_cancelled_with_a_failed_connects_disposition() {
        let p = protocol(vec![profile("cost-a", 0, 2, 1)], 480_000);
        // Never greets: the connect cannot finish.
        let silent = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = silent.local_addr().unwrap();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((s, _)) = silent.accept().await {
                held.push(s);
            }
        });
        let owned = detached(addr, mailpit_http(vec![], false).await, &p);
        let m = rows_only(&p, "run-d3b");
        let dir = fixture();
        let out = dir.path().join("run");
        let sink = sink_for(&m, &out);
        let clock = wire::Clock(Instant::now());
        // The connect never finishes, so the instant always cuts it in flight;
        // 3 s only has to let the connect begin on a loaded host.
        let end = Instant::now() + Duration::from_secs(3);
        runner::throughput(
            &owned,
            &p,
            &p.throughput[0],
            "cuttest",
            0,
            "run-d3b",
            &sink,
            &clock,
            (None, Some(end)),
        )
        .await
        .unwrap();
        let rows = read_rows(&out, &m, sink);
        let s = sends(&rows);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].outcome, "failed_before_terminator");
        assert!(s[0].reason.as_deref().unwrap().contains("chunk deadline"));
        assert_eq!(
            (s[1].outcome.as_str(), s[1].reason.as_deref()),
            ("unstarted", Some("chunk deadline"))
        );
    }

    #[tokio::test]
    async fn a_warmup_cut_in_a_leaf_with_an_oom_is_oom_killed_not_a_deadline() {
        let rig = Rig::new(1, true);
        let p = protocol(vec![profile("cost-a", 50, 6, 2)], 480_000);
        let (cfg, _) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-d4o");
        let spec = &cfg.specs[&TargetKind::Swarmail];
        let mut life = runner::Lifecycle {
            launches: vec![],
            cleanups: vec![],
        };
        let clock = wire::Clock(Instant::now());
        let warm = runner::warmup(
            spec,
            &cfg,
            &runner::limits(&p),
            &clock,
            &p.throughput[0],
            1,
            &mut life,
            Some(Instant::now()),
        )
        .await
        .unwrap();
        let runner::Warm::NotMeasured(cause, _) = warm else {
            panic!("the warmup was cut")
        };
        // V13: the leaf's OOM comes before the chunk deadline.
        assert_eq!(cause, runner::Cause::OomKilled);
    }

    #[tokio::test]
    async fn an_idle_window_never_opened_in_a_leaf_with_an_oom_is_oom_killed() {
        let rig = Rig::new(1, true);
        let p = protocol(vec![profile("cost-a", 0, 6, 2)], 480_000);
        let (cfg, _) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-d6o");
        let spec = &cfg.specs[&TargetKind::Swarmail];
        let clock = wire::Clock(Instant::now());
        let scratch = rig.dir.path().join("scratch").join("leaf");
        let mut owned =
            targets::Owned::launch(spec, &scratch, &runner::limits(&p), &clock, &rig.keepers)
                .await
                .unwrap();
        let leaf = owned.leaf.clone().unwrap();
        let sink = sink_for(&cfg.manifest, &cfg.output);
        let closing = runner::Closing {
            run: "run-d6o",
            launch: leaf.name(),
            key: (TargetKind::Swarmail, "cost-a", 0),
            leaf: &leaf,
            proc_root: Path::new("/proc"),
            target_cpus: &rig.target_cpus(),
        };
        let never = runner::idle_ready(
            &closing,
            (&owned, &scratch),
            &sink,
            &clock,
            Some(Instant::now()),
        )
        .await
        .unwrap();
        // V13: the leaf's OOM comes before the chunk deadline.
        assert_eq!(never.outcome, WindowOutcome::OomKilled);
        assert!(never.counters().is_none(), "never opened");
        assert!(owned.shutdown().await.proved());
    }

    #[tokio::test]
    async fn a_warmup_cut_by_chunk_end_launches_no_measured_instance() {
        let rig = Rig::new(0, true);
        let p = protocol(vec![profile("cost-a", 50, 6, 2)], 480_000);
        let (cfg, _) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-d4");
        let spec = &cfg.specs[&TargetKind::Swarmail];
        let mut life = runner::Lifecycle {
            launches: vec![],
            cleanups: vec![],
        };
        let clock = wire::Clock(Instant::now());
        // The instant is here once the warmup instance is up.
        let warm = runner::warmup(
            spec,
            &cfg,
            &runner::limits(&p),
            &clock,
            &p.throughput[0],
            1,
            &mut life,
            Some(Instant::now()),
        )
        .await
        .unwrap();
        let runner::Warm::NotMeasured(cause, launch) = warm else {
            panic!("the warmup was cut")
        };
        assert_eq!(cause, runner::Cause::ChunkDeadline);
        assert_eq!(launch, "target-0001");
        assert_eq!(life.launches.len(), 1, "only the warmup's launch receipt");
        assert!(
            life.cleanups
                .iter()
                .all(|c| c.proved() && c.cgroup_removed == Some(true))
        );
    }

    #[tokio::test]
    async fn a_fidelity_read_cut_by_chunk_end_is_unread_not_a_mismatch() {
        let p = protocol(vec![profile("cost-a", 0, 1, 1)], 480_000);
        let subject = subject_of(&p, "fidtest", 0);
        let owned = detached(
            smtp_stub("").await,
            mailpit_http(vec![("m1".into(), subject)], true).await,
            &p,
        );
        let m = rows_only(&p, "run-d5");
        let dir = fixture();
        let out = dir.path().join("run");
        let sink = sink_for(&m, &out);
        let clock = wire::Clock(Instant::now());
        // The send and the listing finish at once; the fetch is never answered.
        let end = Instant::now() + Duration::from_secs(3);
        let failures = runner::throughput(
            &owned,
            &p,
            &p.throughput[0],
            "fidtest",
            0,
            "run-d5",
            &sink,
            &clock,
            (None, Some(end)),
        )
        .await
        .unwrap();
        assert!(!failures.is_empty(), "the repetition is not exact");
        let rows = read_rows(&out, &m, sink);
        assert_eq!(sends(&rows)[0].outcome, "accepted");
        let visible: Vec<&Row> = rows.iter().filter(|r| r.phase == Phase::Visible).collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(
            visible[0].fidelity_ok, None,
            "unread, not intact and not a mismatch"
        );
        let v = rows.iter().find(|r| r.phase == Phase::Verified).unwrap();
        assert_eq!(
            (v.outcome.as_str(), v.reason.as_deref()),
            ("incomplete", Some("chunk deadline"))
        );
    }

    #[tokio::test]
    async fn an_idle_wait_cut_by_chunk_end_keeps_its_counters_as_chunk_deadline() {
        let rig = Rig::new(0, true);
        let p = protocol(vec![profile("cost-a", 0, 6, 2)], 480_000);
        let (cfg, _) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-d6");
        let spec = &cfg.specs[&TargetKind::Swarmail];
        let clock = wire::Clock(Instant::now());
        let scratch = rig.dir.path().join("scratch").join("leaf");
        let mut owned =
            targets::Owned::launch(spec, &scratch, &runner::limits(&p), &clock, &rig.keepers)
                .await
                .unwrap();
        let leaf = owned.leaf.clone().unwrap();
        let sink = sink_for(&cfg.manifest, &cfg.output);
        let closing = runner::Closing {
            run: "run-d6",
            launch: leaf.name(),
            key: (TargetKind::Swarmail, "cost-a", 0),
            leaf: &leaf,
            proc_root: Path::new("/proc"),
            target_cpus: &rig.target_cpus(),
        };
        // The instant comes one second into the five of the idle window.
        let started = Instant::now();
        let idle = runner::idle_ready(
            &closing,
            (&owned, &scratch),
            &sink,
            &clock,
            Some(started + Duration::from_secs(1)),
        )
        .await
        .unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "cut, not awaited"
        );
        assert_eq!(idle.outcome, WindowOutcome::ChunkDeadline);
        assert!(
            idle.counters().is_some(),
            "it was opened, so it keeps its counters"
        );
        assert!(idle.end_ns - idle.start_ns < 5_000_000_000);
        // At or after the instant the window is not opened at all.
        let never = runner::idle_ready(&closing, (&owned, &scratch), &sink, &clock, Some(started))
            .await
            .unwrap();
        assert_eq!(never.outcome, WindowOutcome::ChunkDeadline);
        assert!(never.counters().is_none());
        // Without an instant it is the full five seconds, measured.
        let full = runner::idle_ready(&closing, (&owned, &scratch), &sink, &clock, None)
            .await
            .unwrap();
        assert_eq!(full.outcome, WindowOutcome::Measured);
        assert!(full.end_ns - full.start_ns >= 5_000_000_000);
        assert!(owned.shutdown().await.proved());
    }

    #[tokio::test]
    async fn the_profiles_own_send_deadline_still_writes_send_deadline() {
        let mut t = profile("cost-a", 0, 3, 1);
        t["send_deadline_ms"] = json!(0);
        let p = protocol(vec![t], 480_000);
        let owned = detached(smtp_stub("").await, mailpit_http(vec![], false).await, &p);
        let m = rows_only(&p, "run-d7");
        let dir = fixture();
        let out = dir.path().join("run");
        let sink = sink_for(&m, &out);
        let clock = wire::Clock(Instant::now());
        // The chunk's instant is far off: the profile's own deadline comes first.
        let end = Instant::now() + Duration::from_secs(600);
        runner::throughput(
            &owned,
            &p,
            &p.throughput[0],
            "t",
            0,
            "run-d7",
            &sink,
            &clock,
            (None, Some(end)),
        )
        .await
        .unwrap();
        let rows = read_rows(&out, &m, sink);
        assert_eq!(sends(&rows).len(), 3);
        assert!(
            sends(&rows).iter().all(|r| {
                r.outcome == "unstarted" && r.reason.as_deref() == Some("send deadline")
            })
        );
    }

    #[tokio::test]
    async fn a_protocol_3_run_has_no_chunk_end() {
        // The same hung server, with no instant: the send ends at the request
        // deadline and its row says what it always said.
        let mut v = json(V4_SMOKE);
        v["throughput"] = json!([profile("cost-a", 0, 2, 1)]);
        v["resources"]["request_deadline_ms"] = json!(300);
        let p = Protocol::parse(&serde_json::to_vec(&v).unwrap()).unwrap();
        let owned = detached(
            smtp_stub("MAIL").await,
            mailpit_http(vec![], false).await,
            &p,
        );
        let m = rows_only(&p, "run-d8");
        let dir = fixture();
        let out = dir.path().join("run");
        let sink = sink_for(&m, &out);
        let clock = wire::Clock(Instant::now());
        let started = Instant::now();
        runner::throughput(
            &owned,
            &p,
            &p.throughput[0],
            "t",
            0,
            "run-d8",
            &sink,
            &clock,
            (None, None),
        )
        .await
        .unwrap();
        assert!(
            started.elapsed() >= Duration::from_millis(300),
            "the request deadline was awaited"
        );
        let rows = read_rows(&out, &m, sink);
        assert!(
            rows.iter()
                .all(|r| { !r.reason.as_deref().unwrap_or_default().contains("chunk") })
        );
        assert!(
            sends(&rows)
                .iter()
                .all(|r| r.outcome == "failed_before_terminator")
        );
    }

    // ---- R2.1, R2.3 -----------------------------------------------------------

    async fn launch_exits_soon(rig: &Rig, root: &Path) -> targets::LaunchError {
        let p = protocol(vec![profile("cost-a", 0, 2, 1)], 480_000);
        let spec = rig.spec(&p, &exits_soon(root));
        let scratch = root.join("s");
        std::fs::create_dir(&scratch).unwrap();
        match targets::Owned::launch(
            &spec,
            &scratch.join("leaf"),
            &runner::limits(&p),
            &wire::Clock(Instant::now()),
            &rig.keepers,
        )
        .await
        {
            Err(e) => e,
            Ok(_) => panic!("a program that exits is never ready"),
        }
    }

    #[tokio::test]
    async fn a_leaf_cleanup_receipt_carries_oom_kills_and_exited_early() {
        // A leaf that counted one OOM kill and a leader that was gone.
        let rig = Rig::new(1, true);
        let root = fixture();
        let targets::LaunchError::NotReady(_, facts, _) =
            launch_exits_soon(&rig, root.path()).await
        else {
            panic!("not ready")
        };
        assert_eq!((facts.oom_kills, facts.exited_early), (Some(1), Some(true)));
        assert!(facts.proved());
        // The same leader, no OOM.
        let rig = Rig::new(0, true);
        let root = fixture();
        let targets::LaunchError::NotReady(_, facts, _) =
            launch_exits_soon(&rig, root.path()).await
        else {
            panic!("not ready")
        };
        assert_eq!((facts.oom_kills, facts.exited_early), (Some(0), Some(true)));
        // A leader the driver had to stop is not an early exit.
        let rig = Rig::new(0, true);
        let p = protocol(vec![profile("cost-a", 0, 2, 1)], 480_000);
        let spec = rig.spec(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")));
        let root = fixture();
        let mut owned = targets::Owned::launch(
            &spec,
            &root.path().join("leaf"),
            &runner::limits(&p),
            &wire::Clock(Instant::now()),
            &rig.keepers,
        )
        .await
        .unwrap();
        let facts = owned.shutdown().await;
        assert_eq!(
            (facts.oom_kills, facts.exited_early),
            (Some(0), Some(false))
        );
        assert!(facts.proved());
        // And a launch without a leaf carries neither, in its bytes.
        let none = targets::CleanupFacts::unproved(TargetKind::Mailpit);
        let text = serde_json::to_string(&none).unwrap();
        assert!(
            !text.contains("oom_kills") && !text.contains("exited_early"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn a_leaf_cleanup_receipt_names_its_leaf() {
        let rig = Rig::new(0, true);
        let root = fixture();
        let targets::LaunchError::NotReady(_, facts, launch) =
            launch_exits_soon(&rig, root.path()).await
        else {
            panic!("not ready")
        };
        let launch = launch.expect("a leaf launch keeps its receipt");
        assert_eq!(facts.leaf.as_deref(), Some("target-0001"));
        assert_eq!(facts.leaf, launch.leaf, "the name in its launch receipt");
        let cleanup = serde_json::to_value(&facts).unwrap();
        assert_eq!(cleanup["leaf"], "target-0001");
        // A launch without a leaf has none to name: the key is absent.
        let none = targets::CleanupFacts::unproved(TargetKind::Mailpit);
        assert!(none.leaf.is_none());
        assert!(serde_json::to_value(&none).unwrap().get("leaf").is_none());
    }

    #[tokio::test]
    async fn a_not_ready_leaf_launch_keeps_its_launch_receipt() {
        let rig = Rig::new(0, true);
        let root = fixture();
        let targets::LaunchError::NotReady(_, facts, launch) =
            launch_exits_soon(&rig, root.path()).await
        else {
            panic!("not ready")
        };
        let launch = launch.expect("a leaf launch keeps its receipt");
        assert_eq!(launch.leaf.as_deref(), Some("target-0001"));
        assert!(launch.limits.is_some() && launch.joined.is_some() && launch.cpu_layout.is_some());
        assert!(facts.proved());
    }

    fn chunk_of_two_profiles() -> Protocol {
        // The first profile has a warmup, the second does not: a target that
        // exits at once is met on each of the two paths.
        protocol(
            vec![profile("cost-a", 3, 6, 2), profile("cost-b", 0, 6, 2)],
            480_000,
        )
    }

    type Ran = (
        runner::Verdict,
        runner::Lifecycle,
        Option<PathBuf>,
        Manifest,
        Rig,
    );

    async fn run_exits_soon() -> Ran {
        let rig = Rig::new(0, true);
        let p = chunk_of_two_profiles();
        let candidate = exits_soon(rig.dir.path());
        let (cfg, chunk) = rig.config(&p, &candidate, "run-e1");
        let manifest = cfg.manifest.clone();
        let (v, life, sealed) = runner::run_chunk(cfg, chunk, wire::Clock(Instant::now())).await;
        (v, life, sealed, manifest, rig)
    }

    #[tokio::test]
    async fn a_chunk_whose_target_exits_at_once_seals_with_exit_one_and_goes_on() {
        let (verdict, life, sealed, manifest, _rig) = run_exits_soon().await;
        assert_eq!(verdict.exit_code(), 1, "{verdict:?}");
        let sealed = sealed.expect("a failed target is a measured outcome: the chunk seals");
        assert_eq!(
            life.launches.len(),
            life.cleanups.len(),
            "a launch receipt for each cleanup"
        );
        assert_eq!(life.launches.len(), 2, "each profile was attempted");
        assert!(
            life.cleanups
                .iter()
                .all(|c| c.proved() && c.cgroup_removed == Some(true))
        );
        let (rows, res) = records::read_run(&sealed, &manifest).unwrap();
        assert_eq!(sends(&rows).len(), 12);
        assert!(
            sends(&rows).iter().all(|r| {
                r.outcome == "unstarted" && r.reason.as_deref() == Some("target exited")
            })
        );
        assert_eq!(windows(&res).len(), 6);
        for w in windows(&res) {
            assert_eq!(w.outcome, WindowOutcome::TargetExited);
            assert!(w.counters().is_none());
        }
        assert_eq!(
            window(&res, "cost-a", WindowKind::Measure).launch,
            "target-0001"
        );
        assert_eq!(
            window(&res, "cost-b", WindowKind::Measure).launch,
            "target-0002"
        );
        // And the figures say so: no number, and the reason.
        let report: Value =
            serde_json::from_slice(&std::fs::read(sealed.join("report.json")).unwrap()).unwrap();
        for cell in report["cost"]["cells"].as_array().unwrap() {
            assert_eq!(cell["ranked"], false);
            assert_eq!(cell["shortfalls"], json!(["target_exited"]));
        }
    }

    #[tokio::test]
    async fn a_target_that_exits_before_its_join_is_read_is_still_target_exited() {
        // `/bin/false` is gone in well under a millisecond: the join's
        // read-back must not race it, whatever the schedule.
        for n in 0..8 {
            let rig = Rig::new(0, true);
            let p = protocol(vec![profile("cost-a", 0, 6, 2)], 480_000);
            let (cfg, chunk) = rig.config(&p, Path::new("/bin/false"), "run-e4");
            let (verdict, life, sealed) =
                runner::run_chunk(cfg, chunk, wire::Clock(Instant::now())).await;
            assert_eq!(verdict.exit_code(), 1, "round {n}: {verdict:?}");
            assert!(sealed.is_some(), "round {n}");
            assert_eq!(life.launches.len(), 1);
            assert!(life.launches[0].joined.is_some(), "its join was read back");
            assert!(life.cleanups.iter().all(targets::CleanupFacts::proved));
        }
    }

    #[tokio::test]
    async fn the_chunk_goes_on_after_a_profile_that_was_not_measured() {
        let (_, life, _, _, _rig) = run_exits_soon().await;
        let names: Vec<&str> = life
            .launches
            .iter()
            .map(|l| l.leaf.as_deref().unwrap())
            .collect();
        assert_eq!(
            names,
            ["target-0001", "target-0002"],
            "the second profile was still attempted"
        );
    }

    #[tokio::test]
    async fn a_join_mismatch_is_still_exit_two_and_unsealed() {
        // A scope that does not trust the join: the real process is not in
        // the fixture leaf, and the read-back says so.
        let rig = Rig::new(0, false);
        let p = protocol(vec![profile("cost-a", 0, 6, 2)], 480_000);
        let (cfg, chunk) = rig.config(&p, Path::new(env!("CARGO_BIN_EXE_swarmail")), "run-e2");
        let (verdict, life, sealed) =
            runner::run_chunk(cfg, chunk, wire::Clock(Instant::now())).await;
        assert_eq!(verdict.exit_code(), 2, "{verdict:?}");
        assert!(sealed.is_none(), "no seal on a launch refusal");
        let runner::Verdict::Infrastructure(why) = verdict else {
            panic!()
        };
        assert!(why.contains("not its leaf"), "{why}");
        assert!(life.cleanups.iter().all(targets::CleanupFacts::proved));
    }

    #[tokio::test]
    async fn a_protocol_3_target_that_exits_at_once_is_still_infrastructure() {
        let root = fixture();
        let p = smoke();
        let mut spec = swarmail_spec(targets::swarmail_template(0));
        spec.binary = exits_soon(root.path());
        let cfg = runner::Config {
            manifest: manifest(&p, "run-e3"),
            specs: [(TargetKind::Swarmail, spec)].into(),
            output: root.path().join("out"),
            scratch: root.path().join("s"),
            protocol: p,
            keepers: targets::Keepers::default(),
        };
        std::fs::create_dir(root.path().join("s")).unwrap();
        let (verdict, _, sealed) = runner::run(cfg, wire::Clock(Instant::now())).await;
        assert!(sealed.is_none());
        let runner::Verdict::Infrastructure(why) = verdict else {
            panic!("protocol 3 keeps its behaviour: {verdict:?}")
        };
        assert!(why.contains("not ready"), "{why}");
    }
}

// ---- R2.2 ------------------------------------------------------------------

#[test]
fn a_not_ready_launch_is_classified_from_its_receipts() {
    use runner::Cause::*;
    let facts = |oom: Option<u64>, early: Option<bool>, proved: bool| {
        let mut f = targets::CleanupFacts::unproved(TargetKind::Swarmail);
        (
            f.reaped,
            f.group_gone,
            f.listeners_closed,
            f.scratch_removed,
        ) = (proved, proved, proved, proved);
        f.leaf = Some("target-0001".into());
        f.cgroup_removed = Some(proved);
        (f.oom_kills, f.exited_early) = (oom, early);
        f
    };
    let c = |f: &targets::CleanupFacts, receipt| runner::classify_not_ready(f, receipt);
    // An OOM kill is an OOM whatever else is true.
    assert_eq!(c(&facts(Some(1), Some(true), true), true), Some(OomKilled));
    assert_eq!(c(&facts(Some(2), Some(false), true), true), Some(OomKilled));
    // A leader that was gone, with no OOM, exited on its own.
    assert_eq!(
        c(&facts(Some(0), Some(true), true), true),
        Some(TargetExited)
    );
    assert_eq!(c(&facts(None, Some(true), true), true), Some(TargetExited));
    // Alive and never ready is not something the driver can call a result.
    assert_eq!(c(&facts(Some(0), Some(false), true), true), None);
    assert_eq!(c(&facts(None, None, true), true), None);
    // No launch receipt: nothing evidences the launch.
    assert_eq!(c(&facts(Some(1), Some(true), true), false), None);
    // An unproved cleanup proves nothing, whatever it read.
    assert_eq!(c(&facts(Some(1), Some(true), false), true), None);
    assert_eq!(c(&facts(Some(0), Some(true), false), true), None);
}

// ============================================================================
// T7: the real proofs. `#[ignore]`d and Linux-only; each fails, never skips,
// without a delegated cgroup v2 hierarchy, with the refusal's exact text.
// Run one at a time, as the only process of a delegated scope:
//
//   bin=$(cargo test --locked --test mail_compare --no-run --message-format=json \
//     | grep -o '"executable":"[^"]*mail_compare-[^"]*"' | tail -n1 | cut -d'"' -f4)
//   cat /proc/loadavg
//   timeout 600 taskset -c 0-31 systemd-run --user --scope --quiet -p Delegate=yes -- \
//     "$bin" --ignored --exact v4::<name> --test-threads=1
//   cat /proc/loadavg
//
// cargo must not be the process inside the scope: it would stay in the
// scope's own cgroup, and the test could then not enable controllers.
// ============================================================================

const V4_SMOKE: &[u8] = include_bytes!("v4-smoke.json");

#[test]
fn the_v4_readme_states_the_revised_sentences() {
    let readme = std::fs::read_to_string("docs/evals/README.md").unwrap();
    let start = readme
        .find("## Protocol v4: cost to operate")
        .expect("the v4 section");
    let section = &readme[start..];
    let section = &section[..section[3..].find("\n## ").map_or(section.len(), |n| n + 3)];
    // The markdown is wrapped: compare the words, not the line breaks.
    let section = section.split_whitespace().collect::<Vec<_>>().join(" ");
    for needle in [
        // SMT siblings: idle where they are inside the set, recorded otherwise.
        "siblings_outside_set",
        // A figure that was not observed is null, never zero.
        "`null`, never zero",
        // Every row has an installed size, identified by a listing digest.
        "Every row has an installed size",
        "`installed_trees`",
        "`sha256sum`",
        "identified by its digest and not pinned",
        // The ten minutes hold as a sum, and a scope is read from systemd.
        "`timeout --kill-after=30 560 taskset",
        "= 597 s",
        "= 596 s",
        "merge-base --is-ancestor",
        "`inactive` or `failed`",
        // A startup that is alive but not ready stops the run.
        "alive but not ready",
        // It is a frozen v4 that the driver refuses; a draft run is labelled.
        "refuses a *frozen* protocol 4",
        "`protocol_status`",
        // Affinity is read at the end of every opened window.
        "end of every window that was opened",
        // The chunk deadline reaches the work in flight.
        "cancelled, not awaited",
        // The seal proves the state at the seal.
        "power loss",
        // The host run is what the merge waits for.
        "The host run gates the merge",
        "is not merged",
    ] {
        assert!(section.contains(needle), "the v4 section lacks {needle:?}");
    }
    assert!(
        !section.contains("can never be admitted"),
        "admit-v4 does admit a draft run, and labels it"
    );
    for gone in [
        "owed: the extracted archive",
        "--kill-after=30 600",
        "wrapper's 600 s kill",
        "`executable_bytes`",
    ] {
        assert!(
            !section.contains(gone),
            "the v4 section still says {gone:?}"
        );
    }
    assert!(
        !section.contains("**Owed.**"),
        "the owed paragraph is replaced by the merge gate"
    );
    // v3 still collects from the active paths.
    assert!(section.contains("v3 still collects from those paths"));
}

#[test]
fn the_v4_smoke_fixture_validates_as_a_draft() {
    let p = Protocol::parse(V4_SMOKE).unwrap();
    p.validate().expect("the smoke fixture validates");
    assert_eq!(p.status, protocol::Status::Draft);
    assert_eq!(p.throughput.len(), 1);
    assert_eq!(
        (p.throughput[0].messages, p.throughput[0].warmup_messages),
        (20, 10)
    );
    let l = lock_from(lock5()).unwrap();
    protocol::pairing(&p, &l).unwrap();
    protocol::retention_matches(&p, &l).unwrap();
    protocol::launchable(&p, &l).unwrap();
    assert_eq!(
        runner::plan_v4(&p).len(),
        8,
        "one repetition of eight targets"
    );
}

/// Detect, check the scratch, pin this thread to the client CPUs and adopt
/// the scope, or fail with the refusal's exact text. Call it before any
/// runtime is built, so every worker thread inherits the pin.
#[cfg(target_os = "linux")]
fn proof_scope() -> resources::Scope {
    resources::prepare_host(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .unwrap_or_else(|e| panic!("{e}"))
}

#[cfg(target_os = "linux")]
fn proof_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// A child that joins `leaf` and the target CPUs between fork and exec, by
/// the same call the launcher makes.
#[cfg(target_os = "linux")]
fn spawn_in_leaf(
    leaf: &resources::Leaf,
    layout: &resources::Layout,
    mut cmd: std::process::Command,
) -> std::process::Child {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    let procs = std::fs::OpenOptions::new()
        .write(true)
        .open(leaf.procs_path())
        .unwrap();
    let fd = procs.as_raw_fd();
    let affinity = resources::Affinity::of(&layout.target_cpus).unwrap();
    // SAFETY: the closure only writes to an open descriptor and sets an
    // affinity mask built before the fork.
    unsafe {
        cmd.pre_exec(move || resources::join_in_child(fd, &affinity));
    }
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .expect("spawn into the leaf")
}

/// Wait for `check` for at most `secs`, polling: a real deadline, never a
/// sleep a result depends on.
#[cfg(target_os = "linux")]
fn within(secs: u64, what: &str, mut check: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    while !check() {
        assert!(Instant::now() < end, "{what}: not within {secs} s");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(target_os = "linux")]
fn teardown(rt: &tokio::runtime::Runtime, leaf: &resources::Leaf) -> resources::Teardown {
    rt.block_on(async {
        leaf.teardown(tokio::time::Instant::now() + Duration::from_secs(10))
            .await
    })
}

#[cfg(target_os = "linux")]
fn reap(children: &mut [std::process::Child]) {
    for c in children {
        let _ = c.kill();
        let _ = c.wait();
    }
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs a delegated cgroup v2 scope; see the header of this section"]
fn v4_real_oom_at_two_gib() {
    let scope = proof_scope();
    let rt = proof_runtime();
    let (leaf, limits) = scope.leaf().expect("a leaf with the budget");
    assert_eq!(limits.memory_max, "2147483648");
    let before = leaf.mem_events().unwrap();
    // A hog that allocates and writes 64 MiB at a time until the kernel
    // kills it. Not `tail /dev/zero`: GNU tail buffers the endless line,
    // but the uutils tail of Ubuntu 26.04 streams it to stdout and never
    // grows (the proof timed out on this host, 2026-11-17).
    let mut cmd = std::process::Command::new("python3");
    cmd.args([
        "-c",
        "b = []\nwhile True:\n    b.append(bytearray(b'\\x01') * (64 << 20))",
    ])
    .stdout(std::process::Stdio::null());
    let mut hog = spawn_in_leaf(&leaf, &scope.layout, cmd);
    within(120, "the hog is OOM-killed", || {
        hog.try_wait().unwrap().is_some()
    });
    let after = leaf.mem_events().unwrap();
    assert!(after.oom_kill >= 1, "the kernel killed it: {after:?}");
    // The cell's outcome is `oom_killed`, and nothing is retried: there is
    // exactly one process, and it is gone.
    assert_eq!(
        runner::window_outcome(&before, &after, true, false),
        records::WindowOutcome::OomKilled
    );
    let peak = leaf.mem_peak().unwrap();
    let (half, limit) = (1u64 << 30, 2u64 << 30);
    assert!(
        peak >= half && peak <= limit + (64 << 20),
        "peak {peak} sits at the 2 GiB limit"
    );
    let t = teardown(&rt, &leaf);
    assert!(
        t.killed && t.removed,
        "the leaf was emptied and removed: {t:?}"
    );
    assert!(t.memory_peak_bytes.is_some());
    // The teardown reads the OOM it is the receipt of, before the leaf goes.
    assert!(
        t.oom_kills.is_some_and(|n| n >= 1),
        "the teardown's oom_kills is the kernel's: {t:?}"
    );
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs a delegated cgroup v2 scope; see the header of this section"]
fn v4_real_cpu_held_within_two() {
    let scope = proof_scope();
    let rt = proof_runtime();
    let clock = wire::Clock(Instant::now());
    for widen in [false, true] {
        let (leaf, _) = scope.leaf().unwrap();
        let mut spinners: Vec<std::process::Child> = (0..4)
            .map(|_| {
                let mut cmd = std::process::Command::new("sh");
                cmd.arg("-c").arg(if widen {
                    "taskset -pc 0-31 $$ >/dev/null 2>&1; while :; do :; done"
                } else {
                    "while :; do :; done"
                });
                spawn_in_leaf(&leaf, &scope.layout, cmd)
            })
            .collect();
        let held = |leaf: &resources::Leaf| {
            let pids = leaf.read_procs().unwrap();
            held(Path::new("/proc"), &pids, &scope.layout.target_cpus)
        };
        if widen {
            // The spinners widen their own affinity; wait for all four to.
            within(20, "every spinner widened its affinity", || !held(&leaf));
        } else {
            assert!(held(&leaf), "every task has exactly the two target CPUs");
        }
        let a = leaf.snap(&clock).unwrap();
        let start = Instant::now();
        // The fixed five-second window is the measurement itself.
        std::thread::sleep(Duration::from_secs(5));
        let b = leaf.snap(&clock).unwrap();
        let wall_us = start.elapsed().as_micros() as f64;
        let used = (b.cpu.usage_usec - a.cpu.usage_usec) as f64;
        let bound = 2.0 * wall_us * 1.05 + 100_000.0;
        assert!(
            used <= bound,
            "widen={widen}: {used} µs of CPU in {wall_us} µs of wall time exceeds {bound}"
        );
        assert!(
            used >= wall_us,
            "widen={widen}: the spinners did use the budget ({used} µs)"
        );
        if widen {
            assert!(
                b.cpu.nr_throttled > a.cpu.nr_throttled,
                "the quota throttled: {:?} -> {:?}",
                a.cpu,
                b.cpu
            );
            assert!(
                !held(&leaf),
                "affinity_held is false once a task widened it"
            );
        } else {
            assert!(held(&leaf), "affinity held through the window");
        }
        let t = teardown(&rt, &leaf);
        reap(&mut spinners);
        assert!(t.killed && t.removed, "widen={widen}: {t:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs a delegated cgroup v2 scope; see the header of this section"]
fn v4_real_setsid_escapee_removed() {
    let scope = proof_scope();
    let rt = proof_runtime();
    let (leaf, _) = scope.leaf().unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("setsid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pidfile = dir.join("escapee.pid");
    // The child starts a sleeper in a new session and exits.
    let script = format!(
        "setsid sh -c 'echo $$ > {}; exec sleep 300' & exit 0",
        pidfile.display()
    );
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(script);
    let mut outer = spawn_in_leaf(&leaf, &scope.layout, cmd);
    let outer_pid = outer.id() as i32;
    outer.wait().unwrap();
    within(10, "the escapee wrote its pid", || {
        std::fs::read_to_string(&pidfile).is_ok_and(|t| !t.trim().is_empty())
    });
    let escapee: i32 = std::fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // The process-group kill of 0092 §6 does not reach it: it is in its own
    // session and group, but still in the leaf.
    // SAFETY: signalling a process group the test created.
    unsafe { libc::killpg(outer_pid, libc::SIGKILL) };
    assert!(
        unsafe { libc::kill(escapee, 0) } == 0,
        "the escapee survived the group kill"
    );
    let procs = leaf.read_procs().unwrap();
    assert!(
        procs.contains(&escapee),
        "cgroup.procs is not empty: {procs:?}"
    );
    let t = teardown(&rt, &leaf);
    assert!(
        t.killed && t.removed,
        "after cgroup.kill the leaf is empty and removed: {t:?}"
    );
    within(
        10,
        "the escapee is gone",
        || unsafe { libc::kill(escapee, 0) } != 0,
    );
    assert!(!leaf.read_procs().is_ok_and(|p| !p.is_empty()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(target_os = "linux")]
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

#[cfg(target_os = "linux")]
impl std::io::Write for Shared {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn proof_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs a delegated cgroup v2 scope; see the header of this section"]
fn v4_real_hold_swarmail_under_budget() {
    let scope = Arc::new(proof_scope());
    let rt = proof_runtime();
    let mut spec = swarmail_spec(targets::swarmail_template(protocol::V4_CAP));
    spec.extra.scope = Some(scope.clone());
    let dir = proof_dir("hold");
    let (hold_dir, samples) = (dir.join("hold"), dir.join("samples.jsonl"));
    let out = Shared::default();
    let (stdin, reader) = tokio::io::duplex(64);
    let receipt = rt.block_on(async {
        let task = {
            let (out, samples) = (out.clone(), samples.clone());
            tokio::spawn(async move {
                let mut w = out;
                runner::hold(
                    &spec,
                    (&hold_dir, &samples),
                    &limits(),
                    &wire::Clock(Instant::now()),
                    Duration::from_millis(100),
                    reader,
                    &mut w,
                    std::future::pending(),
                )
                .await
            })
        };
        let first = loop {
            let text = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
            if let Some(line) = text.lines().next() {
                break line.to_owned();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        let up: Value = serde_json::from_str(&first).unwrap();
        assert!(up["leaf"].as_str().unwrap().starts_with("target-"), "{up}");
        let smtp = up["smtp"].as_str().unwrap().parse().unwrap();
        let http = up["http"].as_str().unwrap().parse().unwrap();
        // One mail, accepted, and then visible: the held target is the real one.
        let clock = wire::Clock(Instant::now());
        let step = Duration::from_secs(10);
        let kinds = [protocol::MessageKind::Plain];
        let msg = wire::authored(163, "holdproof0001", 0, &kinds);
        let mut s = wire::Smtp::open(smtp, step)
            .await
            .expect("the held target accepts a session");
        let rec = s.send(&msg, &clock).await;
        assert!(
            matches!(rec.disposition, wire::Disposition::Accepted { .. }),
            "{:?}",
            rec.disposition
        );
        s.quit().await;
        let reader_side = targets::Owned::fixture(TargetKind::Swarmail, http, None, &limits());
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            let seen = reader_side.list_all(4).await.unwrap_or_default();
            if seen.iter().any(|(tok, _)| *tok == msg.token) {
                break;
            }
            assert!(
                Instant::now() < end,
                "the accepted mail never became visible"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        // Sampled while it is held.
        let end = Instant::now() + Duration::from_secs(10);
        while std::fs::read_to_string(&samples).map_or(0, |t| t.lines().count()) < 3 {
            assert!(Instant::now() < end, "the sampler wrote no rows");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        drop(stdin);
        task.await
            .unwrap()
            .expect("it tears down and prints its receipts")
    });
    runner::hold_verdict(&receipt).expect("EOF with no byte is a clean end");
    let l = &receipt.launch;
    // All five limits read back as V9 sets them, not only two of them.
    let read = l.limits.as_ref().expect("the limits read back");
    assert_eq!(read.memory_max, "2147483648");
    assert_eq!(read.memory_swap_max, "0");
    assert_eq!(read.memory_oom_group, "1");
    assert_eq!(read.cpu_max, "200000 100000");
    assert_eq!(read.pids_max, "4096");
    // The cleanup receipt names the leaf of its launch receipt (revision 2).
    assert!(l.leaf.is_some(), "{l:?}");
    assert_eq!(receipt.cleanup.leaf, l.leaf, "{:?}", receipt.cleanup);
    let joined = l.joined.as_ref().expect("the join read-back");
    assert_eq!(Some(&joined.cgroup), l.leaf.as_ref());
    assert_eq!(joined.cpus_allowed, scope.layout.target_cpus);
    assert_eq!(receipt.cleanup.cgroup_removed, Some(true));
    assert!(receipt.cleanup.proved(), "{:?}", receipt.cleanup);
    let lines = std::fs::read_to_string(&samples).unwrap();
    assert!(lines.lines().all(|l| l.contains("\"kind\":\"sample\"")));
    let printed = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
    assert_eq!(printed.lines().count(), 2, "{printed}");
    // A second hold, ended by its interrupt instead of EOF, gives the same
    // receipt: the interrupt fires after the endpoints line, and the pipe
    // stays open and silent throughout.
    let mut spec = swarmail_spec(targets::swarmail_template(protocol::V4_CAP));
    spec.extra.scope = Some(scope.clone());
    let (hold_dir, samples2) = (dir.join("hold2"), dir.join("samples2.jsonl"));
    let out2 = Shared::default();
    let (stdin2, reader2) = tokio::io::duplex(64);
    let (fire, interrupt) = tokio::sync::oneshot::channel::<()>();
    let by_interrupt = rt.block_on(async {
        let task = {
            let (out, samples2) = (out2.clone(), samples2.clone());
            tokio::spawn(async move {
                let mut w = out;
                runner::hold(
                    &spec,
                    (&hold_dir, &samples2),
                    &limits(),
                    &wire::Clock(Instant::now()),
                    Duration::from_millis(100),
                    reader2,
                    &mut w,
                    async {
                        let _ = interrupt.await;
                    },
                )
                .await
            })
        };
        while out2.0.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        fire.send(()).unwrap();
        task.await
            .unwrap()
            .expect("the interrupt tears it down and prints its receipts")
    });
    drop(stdin2);
    runner::hold_verdict(&by_interrupt).expect("an interrupt is a clean end");
    assert_eq!(by_interrupt.cleanup.cgroup_removed, Some(true));
    assert!(by_interrupt.cleanup.proved(), "{:?}", by_interrupt.cleanup);
    assert_eq!(by_interrupt.launch.limits, receipt.launch.limits);
    assert!(by_interrupt.launch.leaf.is_some());
    assert_eq!(by_interrupt.cleanup.leaf, by_interrupt.launch.leaf);
    assert_eq!(by_interrupt.cleanup.exited_early, Some(false));
    let printed = String::from_utf8(out2.0.lock().unwrap().clone()).unwrap();
    assert_eq!(printed.lines().count(), 2, "{printed}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs a delegated cgroup v2 scope; see the header of this section"]
fn v4_real_cost_chunk_seals_and_admits() {
    let scope = Arc::new(proof_scope());
    let rt = proof_runtime();
    let p = Protocol::parse(V4_SMOKE).unwrap();
    let sbytes = serde_json::to_vec(&lock5()).unwrap();
    let lock = SourceLock::parse(&sbytes).unwrap();
    let chunk = runner::Chunk::Cost {
        target: TargetKind::Swarmail,
        repetition: 0,
    };
    let bins = vec![format!("swarmail={}", env!("CARGO_BIN_EXE_swarmail"))];
    let mut specs =
        runner::launch_specs_of(&p, &lock, &bins, &[], &[TargetKind::Swarmail]).unwrap();
    specs.get_mut(&TargetKind::Swarmail).unwrap().extra.scope = Some(scope.clone());
    let dir = proof_dir("chunk");
    let (output, scratch) = (dir.join("chunk"), dir.join("scratch"));
    std::fs::create_dir(&scratch).unwrap();
    let mut m = manifest(&p, "proof5");
    m.protocol_sha256 = targets::sha256_hex(V4_SMOKE);
    m.sources_sha256 = targets::sha256_hex(&sbytes);
    let (planned, windows) = runner::plan_chunk(&p, &chunk);
    (m.planned, m.planned_windows, m.chunk) = (planned, Some(windows), Some(chunk.name()));
    m.host = Some(resources::host_facts(
        "Linux (proof)",
        "systemd (proof)",
        &scope.layout,
    ));
    let keepers = targets::Keepers::default();
    let cfg = runner::Config {
        protocol: p.clone(),
        manifest: m,
        specs,
        output: output.clone(),
        scratch: scratch.clone(),
        keepers: keepers.clone(),
    };
    let clock = wire::Clock(Instant::now());
    let (verdict, life, sealed) = rt.block_on(runner::run_chunk(cfg, chunk.clone(), clock));
    let settled = rt.block_on(keepers.settle(Duration::from_secs(60)));
    assert!(
        settled.iter().all(targets::CleanupFacts::proved),
        "{settled:?}"
    );
    assert_eq!(
        verdict,
        runner::Verdict::Clean,
        "the candidate is exact under the budget"
    );
    let sealed = sealed.expect("the chunk sealed");
    assert!(std::fs::remove_dir(&scratch).is_ok(), "the scratch is gone");
    // Two launches: the warmup instance and the measured one, both in leaves.
    assert_eq!(life.launches.len(), 2);
    assert!(
        life.cleanups
            .iter()
            .all(|c| c.proved() && c.cgroup_removed == Some(true))
    );
    // Each cleanup receipt names the leaf of its launch receipt (revision 2).
    for (launch, cleanup) in life.launches.iter().zip(&life.cleanups) {
        assert!(launch.leaf.is_some(), "{launch:?}");
        assert_eq!(cleanup.leaf, launch.leaf, "{cleanup:?}");
    }
    let bins = Default::default();
    let a = report::Admission {
        protocol: &p,
        protocol_bytes: V4_SMOKE,
        lock: &lock,
        lock_bytes: &sbytes,
        bins: &bins,
    };
    let c = report::admit_chunk(&sealed, &chunk, &a).expect("windows, samples and figures admit");
    let windows: Vec<&records::WindowRow> = c
        .resources
        .iter()
        .filter_map(|r| match r {
            records::ResourceRow::Window(w) => Some(w),
            records::ResourceRow::Sample(_) => None,
        })
        .collect();
    assert_eq!(windows.len(), 3);
    assert!(
        windows
            .iter()
            .all(|w| w.outcome == records::WindowOutcome::Measured
                && w.affinity_held == Some(true))
    );
    let idle = windows
        .iter()
        .find(|w| w.window == records::WindowKind::IdleReady)
        .unwrap();
    assert!(
        idle.end_ns - idle.start_ns >= 5_000_000_000,
        "five seconds without traffic"
    );
    assert!(idle.scratch_bytes.is_some_and(|b| b > 0));
    let measure = windows
        .iter()
        .find(|w| w.window == records::WindowKind::Measure)
        .unwrap();
    assert!(
        measure.cpu_end.unwrap().usage_usec > measure.cpu_start.unwrap().usage_usec,
        "the target used CPU"
    );
    assert!(
        c.resources
            .iter()
            .filter(|r| matches!(r, records::ResourceRow::Sample(_)))
            .count()
            >= 10
    );
    let cell = &c.report["cost"]["cells"][0];
    assert_eq!(cell["ranked"], true, "{cell}");
    assert!(cell["visible_per_cpu_second"]["median"].as_f64().unwrap() > 0.0);
    assert!(cell["peak_memory_mib"]["median"].as_f64().unwrap() > 0.0);
    // A second chunk whose "candidate" is a program that exits at once seals
    // with exit 1: the profile is not measured with `target_exited`, each
    // launch has its receipt and a proved cleanup, and no figure is a number.
    let bins = vec!["swarmail=/bin/false".to_owned()];
    let mut specs =
        runner::launch_specs_of(&p, &lock, &bins, &[], &[TargetKind::Swarmail]).unwrap();
    specs.get_mut(&TargetKind::Swarmail).unwrap().extra.scope = Some(scope.clone());
    let (output, scratch) = (dir.join("chunk-false"), dir.join("scratch-false"));
    std::fs::create_dir(&scratch).unwrap();
    let mut m = manifest(&p, "proof5b");
    m.protocol_sha256 = targets::sha256_hex(V4_SMOKE);
    m.sources_sha256 = targets::sha256_hex(&sbytes);
    let (planned, windows) = runner::plan_chunk(&p, &chunk);
    (m.planned, m.planned_windows, m.chunk) = (planned, Some(windows), Some(chunk.name()));
    m.host = Some(resources::host_facts(
        "Linux (proof)",
        "systemd (proof)",
        &scope.layout,
    ));
    let keepers = targets::Keepers::default();
    let cfg = runner::Config {
        protocol: p.clone(),
        manifest: m,
        specs,
        output,
        scratch: scratch.clone(),
        keepers: keepers.clone(),
    };
    let (verdict, life, sealed) = rt.block_on(runner::run_chunk(
        cfg,
        chunk.clone(),
        wire::Clock(Instant::now()),
    ));
    let settled = rt.block_on(keepers.settle(Duration::from_secs(60)));
    assert!(
        settled.iter().all(targets::CleanupFacts::proved),
        "{settled:?}"
    );
    assert_eq!(verdict.exit_code(), 1, "{verdict:?}");
    let sealed = sealed.expect("a target that failed on its own is sealed");
    assert!(!life.launches.is_empty() && life.launches.len() == life.cleanups.len());
    assert!(
        life.cleanups
            .iter()
            .all(|c| c.proved() && c.cgroup_removed == Some(true))
    );
    for (launch, cleanup) in life.launches.iter().zip(&life.cleanups) {
        assert!(
            cleanup.leaf.is_some() && cleanup.leaf == launch.leaf,
            "{cleanup:?}"
        );
    }
    let bins = Default::default();
    let a = report::Admission {
        protocol: &p,
        protocol_bytes: V4_SMOKE,
        lock: &lock,
        lock_bytes: &sbytes,
        bins: &bins,
    };
    let c = report::admit_chunk(&sealed, &chunk, &a).expect("the not-measured chunk admits");
    let failed: Vec<&records::WindowRow> = c
        .resources
        .iter()
        .filter_map(|r| match r {
            records::ResourceRow::Window(w) => Some(w),
            records::ResourceRow::Sample(_) => None,
        })
        .collect();
    assert_eq!(failed.len(), 3);
    assert!(
        failed
            .iter()
            .all(|w| w.outcome == records::WindowOutcome::TargetExited && w.counters().is_none())
    );
    let cell = &c.report["cost"]["cells"][0];
    assert_eq!(cell["ranked"], false, "{cell}");
    assert_eq!(cell["shortfalls"], json!(["target_exited"]));
    for figure in [
        "visible_per_cpu_second",
        "cpu_ms_per_1k_visible",
        "peak_memory_mib",
        "peak_anon_mib",
    ] {
        assert!(cell[figure].is_null(), "{figure}: {cell}");
    }
    // No idle or startup figure of it is a number either: one cell each,
    // unranked, every value null.
    for (section, figures) in [
        ("idle", &["memory_mib", "anon_mib", "scratch_bytes"][..]),
        ("startup", &["ready_ms", "cpu_ms_to_ready"][..]),
    ] {
        let cells = c.report[section]
            .as_array()
            .unwrap_or_else(|| panic!("{section}: {}", c.report));
        assert_eq!(cells.len(), 1, "{section}: {cells:?}");
        let cell = &cells[0];
        assert_eq!(cell["ranked"], false, "{section}: {cell}");
        for figure in figures {
            assert!(cell[*figure].is_null(), "{section}.{figure}: {cell}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The five real proofs exist, are `#[ignore]`d and are Linux-only: the file
/// says so, and each is the whole of its function.
#[test]
fn the_five_real_proofs_are_ignored_and_linux_only() {
    let text = std::fs::read_to_string("tests/mail_compare/v4.rs").unwrap();
    for name in [
        "v4_real_oom_at_two_gib",
        "v4_real_cpu_held_within_two",
        "v4_real_setsid_escapee_removed",
        "v4_real_hold_swarmail_under_budget",
        "v4_real_cost_chunk_seals_and_admits",
    ] {
        let at = text
            .find(&format!("\nfn {name}()"))
            .unwrap_or_else(|| panic!("the proof {name} exists"));
        let head: Vec<&str> = text[..at].lines().rev().take(4).collect();
        let head = head.join("\n");
        assert!(
            head.contains("#[cfg(target_os = \"linux\")]"),
            "{name}: {head}"
        );
        assert!(head.contains("#[test]"), "{name}: {head}");
        assert!(
            head.contains("#[ignore = \"needs a delegated cgroup v2 scope"),
            "{name}: {head}"
        );
        // It fails, never skips: no early return on a missing hierarchy.
        let body_end = text[at..].find("\n}\n").map_or(text.len(), |n| at + n);
        let body = &text[at..body_end];
        assert!(!body.contains("return;"), "{name} must not skip");
    }
}
