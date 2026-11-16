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
