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
        assert!(w.contains(expect), "{field}={value}: {w}");
        // And absent: the field omitted refuses too.
        let mut v = draft();
        v["resources"].as_object_mut().unwrap().remove(field);
        // A field the type requires is refused at parse, the rest by name.
        let w = match protocol_from(v) {
            Ok(p) => refusals(p.validate()),
            Err(e) => e.to_string(),
        };
        assert!(
            w.contains(expect) || w.contains("missing field") || w.contains("resources"),
            "{field} absent: {w}"
        );
    }
    let mut v = draft();
    v["resources"]["chunk_deadline_ms"] = json!(0);
    assert!(why(v).contains("chunk_deadline_ms Some(0) is outside 1..=540000"));
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
            ("chunk_deadline_ms", json!(540_000)),
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
    assert_eq!(
        after(&d["claim"].clone().into_wrapped()),
        after(&v3["claim"].clone().into_wrapped())
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
    let j = resources::read_join(proc_root.path(), 4321).unwrap();
    assert_eq!(j.cgroup, "target-0002");
    assert_eq!(j.cpus_allowed, vec![4, 6]);
    resources::check_join(&j, "target-0002", &[4, 6]).unwrap();
    assert!(resources::check_join(&j, "target-0001", &[4, 6]).is_err());
    assert!(resources::read_join(proc_root.path(), 1).is_err());
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
    assert!(resources::affinity_held(
        proc_root.path(),
        &[10, 11],
        &[0, 1]
    ));
    // One widened thread makes it false.
    mk(10, 12, "0-3");
    assert!(!resources::affinity_held(
        proc_root.path(),
        &[10, 11],
        &[0, 1]
    ));
    // A process that vanished is skipped; a status that cannot be parsed is not.
    assert!(resources::affinity_held(
        proc_root.path(),
        &[11, 99],
        &[0, 1]
    ));
    put(&proc_root.path().join("11/task/11"), "status", "garbage\n");
    assert!(!resources::affinity_held(proc_root.path(), &[11], &[0, 1]));
}
