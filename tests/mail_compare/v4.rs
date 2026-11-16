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
            "read memory.peak"
        ],
        "one kill, one read of the procs, nothing more"
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
    f.leaf = true;
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
        true,
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
        runner::window_outcome(&at(0, 0, 0), &at(900, 0, 0), false),
        Measured
    );
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(900, 1, 0), false),
        Measured
    );
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(900, 1, 1), false),
        OomKilled
    );
    // It is the rise over the window, not the total.
    assert_eq!(
        runner::window_outcome(&at(0, 1, 2), &at(0, 1, 2), false),
        Measured
    );
    assert_eq!(
        runner::window_outcome(&at(0, 1, 2), &at(0, 2, 3), true),
        OomKilled
    );
    // A target that exited without an OOM is its own outcome.
    assert_eq!(
        runner::window_outcome(&at(0, 0, 0), &at(0, 0, 0), true),
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
    assert!(resources::affinity_held(proc_root.path(), &[5, 6], &[2, 3]));
    put(
        &proc_root.path().join("6/task/6"),
        "status",
        "Cpus_allowed_list:\t2-4\n",
    );
    assert!(!resources::affinity_held(
        proc_root.path(),
        &[5, 6],
        &[2, 3]
    ));
}

#[tokio::test(start_paused = true)]
async fn the_sampler_writes_a_row_per_tick_and_one_at_each_bound() {
    let scope = fixture();
    let dir = leaf_fixture(scope.path(), "target-0001");
    put(&dir, "memory.current", "8192\n");
    put(&dir, "memory.stat", MEM_STAT);
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
    std::fs::write(leaf.dir().join("memory.current"), "max\n").unwrap();
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
fn a_profile_past_the_chunk_deadline_is_unstarted_rows_and_a_chunk_deadline_window() {
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
        &sink,
        &clock,
    )
    .expect("rows are written");
    drop(sink);
    let (rows, res) =
        records::read_run(&out, &m).expect("the planned rows and windows are all there");
    let sends: Vec<&Row> = rows.iter().filter(|r| r.phase == Phase::Send).collect();
    assert_eq!(sends.len(), 4);
    let why = Some("chunk deadline");
    assert!(
        sends
            .iter()
            .all(|r| r.outcome == "unstarted" && r.reason.as_deref() == why)
    );
    let verified = rows.iter().find(|r| r.phase == Phase::Verified).unwrap();
    assert_eq!(verified.outcome, "incomplete");
    assert_eq!(res.len(), 3);
    for r in &res {
        let records::ResourceRow::Window(w) = r else {
            panic!("only windows are written")
        };
        assert_eq!(w.outcome, records::WindowOutcome::ChunkDeadline);
        assert_eq!(w.cpu_start, w.cpu_end, "nothing was measured");
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
    assert_eq!((one.median, one.min, one.max), (7.5, 7.5, 7.5));
    // Nearest rank: ceil(50 * 2 / 100) = 1, the lower of two.
    let two = stats::spread(&[9.0, 3.0]).unwrap();
    assert_eq!((two.median, two.min, two.max), (3.0, 3.0, 9.0));
    // ceil(50 * 3 / 100) = 2, the middle of three, whatever the order given.
    let three = stats::spread(&[4.0, 1.0, 2.0]).unwrap();
    assert_eq!((three.median, three.min, three.max), (2.0, 1.0, 4.0));
    // A value that is not a number is not a figure.
    let nan = stats::spread(&[f64::NAN, 5.0, f64::INFINITY]).unwrap();
    assert_eq!((nan.median, nan.min, nan.max), (5.0, 5.0, 5.0));
    assert_eq!(stats::spread(&[f64::NAN]), None);
}

// ---- T4.6 ------------------------------------------------------------------

const MIB: u64 = 1 << 20;
const SEC: u64 = 1_000_000_000;

/// One repetition's measurements, all hand-chosen so the expected figures
/// below are worked out on paper, not read back from the code.
struct Rep {
    rep: u32,
    profile: &'static str,
    /// `cpu.stat` user and system µs over the measure window.
    cpu: (u64, u64),
    /// The memory (MiB) sampled mid-window; the bounds sample 100 MiB.
    peak_mib: u64,
    outcome: records::WindowOutcome,
    held: bool,
    oom_kill: u64,
}

impl Rep {
    fn new(rep: u32, cpu: (u64, u64), peak_mib: u64) -> Rep {
        Rep {
            rep,
            profile: "cost-c8-reused",
            cpu,
            peak_mib,
            outcome: records::WindowOutcome::Measured,
            held: true,
            oom_kill: 0,
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
    let key = (TargetKind::Mailpit, r.profile, r.rep);
    let mut out = Vec::new();
    let w = |kind, (a, b): (resources::Snap, resources::Snap), held, scratch, o| {
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
    let m = records::WindowOutcome::Measured;
    out.push(w(
        Ready,
        (
            snap(0, cpu(0, 0, 0), 0),
            snap(SEC, cpu(100_000, 150_000, 0), 0),
        ),
        true,
        None,
        m,
    ));
    out.push(w(
        IdleReady,
        (
            snap(2 * SEC, cpu(0, 0, 0), 0),
            snap(7 * SEC, cpu(0, 0, 0), 0),
        ),
        true,
        Some(4096),
        m,
    ));
    out.push(w(
        Measure,
        (
            snap(8 * SEC, cpu(0, 0, 0), 0),
            snap(20 * SEC, cpu(r.cpu.0, r.cpu.1, 7), r.oom_kill),
        ),
        r.held,
        None,
        r.outcome,
    ));
    let mut s = |t: u64, mib: u64, anon_mib: u64| {
        out.push(records::ResourceRow::Sample(sample_at(
            run,
            &launch,
            t,
            mib * MIB,
            anon_mib * MIB,
        )));
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
        let mut s = runner::row(
            run,
            TargetKind::Mailpit,
            r.profile,
            r.rep,
            Phase::Send,
            i,
            "accepted",
        );
        s.token = Some(format!("t{i}"));
        s.invoked_ns = Some(8 * SEC + u64::from(i));
        s.start_ns = Some(8 * SEC + u64::from(i) + 1);
        s.ack_ns = Some(8 * SEC + u64::from(i) + 1_000_001);
        rows.push(s);
        let mut v = runner::row(
            run,
            TargetKind::Mailpit,
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
        TargetKind::Mailpit,
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
        (1_500_000, 500_000, 2_000_000)
    );
    assert_eq!(first.visible, 4);
    near(first.visible_per_cpu_second.unwrap(), 2.0); // 4 mails / 2 s
    near(first.cpu_ms_per_1k_visible.unwrap(), 500_000.0); // 2000 ms / 0.004 k
    assert_eq!((first.nr_throttled, first.throttled_usec), (7, 7000));
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
    assert_eq!(rep.visible, 0);
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
    assert!(!r.v4.as_ref().unwrap().cost.repetitions[0].affinity_held);
}

#[test]
fn peak_and_mean_come_from_the_samples_inside_the_window() {
    let r = v4_report(&[(Rep::new(0, (1_000_000, 0), 300), None)]);
    let rep = &r.v4.as_ref().unwrap().cost.repetitions[0];
    // Inside [8 s, 20 s]: 100, 300 and 100 MiB (anon 50, 150, 50). The
    // samples just outside, at 999 and 998 MiB, are not read.
    assert_eq!(rep.peak_memory_bytes, 300 * MIB);
    assert_eq!(rep.peak_anon_bytes, 150 * MIB);
    near(
        rep.mean_memory_bytes,
        (100.0 + 300.0 + 100.0) / 3.0 * MIB as f64,
    );
    near(
        rep.mean_anon_bytes,
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
        300 * MIB
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
