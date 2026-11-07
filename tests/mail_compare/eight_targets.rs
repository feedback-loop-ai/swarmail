//! Eight comparison targets (issue #340, proposed decision 0130): source
//! lock 4 and protocol 3 against authored captures, the history bytes they
//! must not disturb, the listener, environment and config rules, and the
//! version pairing. Every fixture here is authored by the test from the
//! contract, never read back from a decoder's or a validator's own output.
//! The real-binary tests at the end are `#[ignore]`d and fail without their
//! pinned bytes; they never skip.

use super::*;
use serde_json::{Value, json};

/// SHA-256 of the bytes at `c95386e`, taken before the change.
const V2_PROTOCOL_SHA: &str = "735da43a0cbaaa85cd0fd6ab55765cbf3f71ba176d58702dd71bc4444a82231d";
/// The freeze (2026-11-07): sha256 of `docs/evals/protocol.json` and
/// `docs/evals/sources.lock.json` as frozen; decision 0130's addendum records
/// the same values. Set when the protocol is frozen, never edited after.
const FROZEN_PROTOCOL_V3_SHA: &str =
    "229babaac849ae832ecb1a9f8ec742c970d0f36edc9d26d4ff9bb9c3c0d7224e";
const FROZEN_LOCK_V4_SHA: &str = "ed201599dd57441cbcf0c7e66e7c82a56dd217ef683b7bf4e72ae7ce9d3a04b1";
const V3_LOCK_SHA: &str = "6523abf4df890d7aca8ea7e5112ae3e32ab35f2260e2528b070eefa70007b03c";

/// The rows the pending-entry tests make pending in an authored copy of the
/// committed lock: the vocabulary stays (a pending entry never launches and
/// never collects) though no committed row is pending any more.
const PENDING: [TargetKind; 4] = [
    TargetKind::Mailslurper,
    TargetKind::Smtp4dev,
    TargetKind::Greenmail,
    TargetKind::AlkemioMailslurper,
];

const NEW_TARGETS: [TargetKind; 6] = [
    TargetKind::Mailslurper,
    TargetKind::MailslurperSourceBuild,
    TargetKind::Smtp4dev,
    TargetKind::Inbucket,
    TargetKind::Greenmail,
    TargetKind::AlkemioMailslurper,
];

fn sha(bytes: &[u8]) -> String {
    targets::sha256_hex(bytes)
}

fn v4() -> Value {
    json(SOURCES_V4)
}

fn v3() -> Value {
    json(SOURCES)
}

fn lock4(v: Value) -> SourceLock {
    lock_from(v).expect("parses")
}

/// The committed lock 4 with `PENDING`'s rows made pending, each authored from
/// what a pending entry states, never read back from a validator.
fn pending_v4() -> Value {
    let mut l = v4();
    for t in PENDING {
        let at = entry_of(&mut l, t.name());
        let (tag, image) = match t {
            TargetKind::AlkemioMailslurper => (Value::Null, json!(protocol::ALKEMIO_IMAGE)),
            TargetKind::Smtp4dev => (json!("3.16.1"), Value::Null),
            TargetKind::Greenmail => (json!("release-2.1.15"), Value::Null),
            _ => (json!("release-1.15.0"), Value::Null),
        };
        l["entries"][at] = json!({
            "target": t.name(), "role": "competitor", "pin": null,
            "pending": {"release_tag": tag, "image": image,
                "listeners": ["smtp", "http"], "launch": [], "launch_env": [],
                "config_file": null, "reason": "authored: the bytes are not available"}
        });
    }
    l
}

fn entry_of(v: &mut Value, t: &str) -> usize {
    v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .position(|e| e["target"] == t)
        .unwrap()
}

// ---- history bytes -------------------------------------------------------

#[test]
fn history_v2_v3_bytes_are_the_c95386e_bytes() {
    assert_eq!(sha(PROTOCOL), V2_PROTOCOL_SHA);
    assert_eq!(sha(SOURCES), V3_LOCK_SHA);
    // Each copy is named for the day its bytes were first committed.
    let history = evals_dir().join("history");
    for (name, want) in [
        ("2026-10-30-protocol.v2.json", V2_PROTOCOL_SHA),
        ("2026-10-30-sources.lock.v3.json", V3_LOCK_SHA),
    ] {
        assert_eq!(
            sha(&std::fs::read(history.join(name)).unwrap()),
            want,
            "{name}"
        );
    }
}

#[test]
fn history_v3_lock_still_validates_verifies_and_pairs_with_v2() {
    let lock = SourceLock::parse(SOURCES).unwrap();
    lock.validate().expect("lock 3 still validates");
    lock.verify_files(&evals_dir())
        .expect("its documents verify");
    let p = Protocol::parse(PROTOCOL).unwrap();
    protocol::pairing(&p, &lock).expect("protocol 2 pairs with lock 3");
    protocol::collection_ready(&p, &lock).expect("the collected pair still admits collection");
    // Both still launch: the specs a v3 lock builds carry no declared
    // listeners, so the lock 3 launch path is the one that runs.
    for t in [TargetKind::Mailpit, TargetKind::Mailhog] {
        let pin = lock.pin(t).unwrap();
        assert!(pin.listeners.is_none() && pin.runtime.is_none());
        assert!(protocol::launch_template_ok(&pin.launch), "{t}");
    }
}

#[test]
fn mailpit_mailhog_v3_documents_byte_unchanged() {
    // The v4 lock binds the very same capture bytes the v3 lock did.
    let old = v3();
    let new = v4();
    for t in ["mailpit", "mailhog"] {
        let docs = |v: &Value| {
            v["entries"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["target"] == t)
                .unwrap()["pin"]["documents"]
                .clone()
        };
        assert_eq!(docs(&old), docs(&new), "{t}");
    }
}

// ---- targets -------------------------------------------------------------

#[test]
fn target_names_round_trip_and_unknown_refused() {
    assert_eq!(TargetKind::ALL.len(), 9);
    for t in TargetKind::ALL {
        assert_eq!(TargetKind::parse(t.name()), Some(t));
        assert_eq!(serde_json::to_value(t).unwrap(), t.name());
        assert_eq!(
            serde_json::from_value::<TargetKind>(json!(t.name())).unwrap(),
            t
        );
    }
    for name in [
        "latest",
        "MailSlurper",
        "alkemiomailslurper",
        "",
        "greenmail ",
    ] {
        assert_eq!(TargetKind::parse(name), None, "{name:?}");
        assert!(serde_json::from_value::<TargetKind>(json!(name)).is_err());
    }
    assert_eq!(
        TargetKind::V2,
        [TargetKind::ALL[0], TargetKind::ALL[1], TargetKind::ALL[2]]
    );
    assert_eq!(
        TargetKind::ALL
            .iter()
            .filter(|t| t.name().starts_with("alkemio"))
            .count(),
        1
    );
}

#[test]
fn alkemio_name_is_not_mailslurper() {
    let a = TargetKind::AlkemioMailslurper;
    let m = TargetKind::Mailslurper;
    assert_ne!(a.name(), m.name());
    assert_ne!(a.provenance(), m.provenance());
    assert!(
        matches!(a.provenance(), protocol::Provenance::ThirdPartyImage(r) if r.contains("alkem-io"))
    );
    assert!(
        matches!(m.provenance(), protocol::Provenance::Official(r) if r.ends_with("/mailslurper"))
    );
    // Source directories are keyed by the target name, so the rows never
    // share one, and neither is Swarmail's own MailSlurper shim's test
    // file (decision 0118).
    assert_ne!(format!("sources/{a}/"), format!("sources/{m}/"));
    for t in [a, m] {
        assert_ne!(t.name(), "mailslurper-shim");
    }
    let shim = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mailslurper.rs");
    assert!(
        shim.exists(),
        "the shim's own test file is the one decision 0118 names"
    );
    assert!(
        !std::fs::read_to_string(shim)
            .unwrap()
            .contains("alkemio-mailslurper")
    );
}

#[test]
fn v2_protocol_and_v3_lock_refuse_each_new_target() {
    for t in NEW_TARGETS {
        // Protocol 2 declaring the target.
        let mut p = json(PROTOCOL);
        p["targets"].as_array_mut().unwrap().push(t.name().into());
        let why = refusals(protocol_from(p).unwrap().validate());
        assert!(why.contains(&format!("refuses target {t}")), "{t}: {why}");
        // Lock 3 locking it.
        let mut l = v3();
        l["entries"].as_array_mut().unwrap().push(json!({
            "target": t.name(), "role": "competitor", "pin": null
        }));
        let why = refusals(lock4(l).validate());
        assert!(
            why.contains(&format!("lock_version 3 refuses target {t}")),
            "{t}: {why}"
        );
    }
}

// ---- version gates and pairing --------------------------------------------

#[test]
fn lock_v3_refuses_each_v4_field() {
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Edit)> = vec![
        (
            "ref_kind",
            Box::new(|p| p["tagged_source"]["observed"]["ref_kind"] = "annotated".into()),
        ),
        (
            "image",
            Box::new(|p| {
                p["image"] = json!({"reference": "x", "index_digest": "sha256:0", "manifest_digest": "sha256:0",
                    "platform": "linux/amd64", "layer_digest": "sha256:0", "member": "m"})
            }),
        ),
        (
            "runtime",
            Box::new(|p| {
                p["runtime"] = json!({"name": "jre", "download": "https://x", "sha256": "0",
                    "digest_basis": "locally_computed", "checked_members": [], "version_output": "v",
                    "license": {"spdx": "MIT", "holder": "h"}})
            }),
        ),
        (
            "listeners",
            Box::new(|p| p["listeners"] = json!(["smtp", "http"])),
        ),
        (
            "launch_env",
            Box::new(|p| p["launch_env"] = json!([["A", "b"]])),
        ),
        (
            "config_file",
            Box::new(|p| p["config_file"] = json!({"name": "c", "lines": ["x"]})),
        ),
        (
            "checked_members",
            Box::new(|p| p["artifact"]["checked_members"] = json!([])),
        ),
    ];
    for (field, edit) in cases {
        let mut l = v3();
        let at = entry_of(&mut l, "mailpit");
        edit(&mut l["entries"][at]["pin"]);
        // Parsing is not the gate (the struct is shared); validation is,
        // and it names the field.
        let parsed = lock_from(l);
        let why = match parsed {
            Ok(lock) => refusals(lock.validate()),
            Err(e) => e.0.join("\n"),
        };
        assert!(
            why.contains(&format!("lock_version 3 refuses {field}")),
            "{field}: {why}"
        );
    }
    // A pending entry at lock 3.
    let mut l = v3();
    l["entries"].as_array_mut().unwrap().push(json!({
        "target": "mailslurper", "role": "competitor", "pin": null,
        "pending": {"release_tag": "release-1.15.0", "image": null, "listeners": ["smtp", "http"],
                    "launch": [], "config_file": null, "reason": "r"}
    }));
    let why = refusals(lock4(l).validate());
    assert!(why.contains("lock_version 3 refuses pending"), "{why}");
}

#[test]
fn lock_v4_requires_ref_kind() {
    let mut l = v4();
    let at = entry_of(&mut l, "mailpit");
    l["entries"][at]["pin"]["tagged_source"]["observed"]
        .as_object_mut()
        .unwrap()
        .remove("ref_kind");
    let why = refusals(lock4(l).validate());
    assert!(
        why.contains("lock_version 4 needs ref_kind on a tagged pin"),
        "{why}"
    );
    // And it needs its listeners, every target and a version it knows.
    let mut l = v4();
    let at = entry_of(&mut l, "mailhog");
    l["entries"][at]["pin"]
        .as_object_mut()
        .unwrap()
        .remove("listeners");
    assert!(refusals(lock4(l).validate()).contains("needs the declared listeners"));
    let mut l = v4();
    l["entries"].as_array_mut().unwrap().pop();
    assert!(
        refusals(lock4(l).validate())
            .contains("lock_version 4 needs an entry for alkemio-mailslurper")
    );
}

#[test]
fn v4_tag_shapes_accept_release_bare_v_and_refuse_latest_branch_rc() {
    for ok in ["release-1.15.0", "3.16.1", "v3.2.0", "v1.32.0", "2.1.15"] {
        assert!(protocol::is_version_tag(ok), "{ok}");
    }
    for bad in [
        "latest",
        "main",
        "release-",
        "v1",
        "1.15.0-rc",
        "",
        "v",
        "release-1",
        "vv1.0",
        "1..2",
        "master",
    ] {
        assert!(!protocol::is_version_tag(bad), "{bad:?}");
    }
    // The v3 shape is not loosened: only `v1.2.3` is a lock 3 / protocol 2 tag.
    assert!(protocol::is_release_tag("v1.32.0"));
    assert!(!protocol::is_release_tag("3.16.1"));
    assert!(!protocol::is_release_tag("release-1.15.0"));
}

#[test]
fn annotated_mailpit_mailhog_pins_unchanged_under_v4() {
    let lock = lock4(v4());
    lock.validate().expect("the current lock validates");
    lock.verify_files(&evals_dir()).expect("and verifies");
    for (t, annotated) in [(TargetKind::Mailpit, true), (TargetKind::Mailhog, true)] {
        let pin = lock.pin(t).unwrap();
        let protocol::Fact::Observed(ts) = &pin.tagged_source else {
            panic!("{t}")
        };
        assert_eq!(ts.ref_kind == Some(protocol::RefKind::Annotated), annotated);
        assert!(ts.tag_object.is_some() && ts.tag_signature.as_deref() == Some("unsigned"));
    }
    // The v3 artifact, release and storage facts are the same values.
    let old = v3();
    let new = v4();
    for t in ["mailpit", "mailhog"] {
        for k in [
            "release", "artifact", "storage", "wait", "launch", "license",
        ] {
            let at = |v: &Value| {
                v["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["target"] == t)
                    .unwrap()["pin"][k]
                    .clone()
            };
            assert_eq!(at(&old), at(&new), "{t}.{k}");
        }
    }
}

#[test]
fn protocol_lock_pairing_refused_across_versions() {
    let p2 = Protocol::parse(PROTOCOL).unwrap();
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let l3 = SourceLock::parse(SOURCES).unwrap();
    let l4 = SourceLock::parse(SOURCES_V4).unwrap();
    protocol::pairing(&p2, &l3).unwrap();
    protocol::pairing(&p3, &l4).unwrap();
    let why = refusals(protocol::pairing(&p2, &l4));
    assert!(
        why.contains("protocol_version 2 pairs only with lock_version 3, not 4"),
        "{why}"
    );
    let why = refusals(protocol::pairing(&p3, &l3));
    assert!(
        why.contains("protocol_version 3 pairs only with lock_version 4, not 3"),
        "{why}"
    );
    // collection_ready lists it too, among every other refusal.
    assert!(
        refusals(protocol::collection_ready(&p2, &l4)).contains("pairs only with lock_version 3")
    );
    assert!(
        refusals(protocol::collection_ready(&p3, &l3)).contains("pairs only with lock_version 4")
    );
}

// ---- authored captures ---------------------------------------------------

const C: &str = "4fa1c2e3b5d6978a0b1c2d3e4f5061728394a5b6";
const TAG: &str = "release-1.15.0";

/// What an authored lightweight capture set can be made to say.
#[derive(Clone)]
struct Knobs {
    target: TargetKind,
    tag: String,
    ref_name: String,
    ref_type: String,
    ref_sha: String,
    truncated: bool,
    omit_from_tree: Option<&'static str>,
    tag_object: bool,
    tag_signature: bool,
    git_tag_doc: bool,
}

impl Knobs {
    fn new(target: TargetKind) -> Knobs {
        Knobs {
            target,
            tag: TAG.into(),
            ref_name: TAG.into(),
            ref_type: "commit".into(),
            ref_sha: C.into(),
            truncated: false,
            omit_from_tree: None,
            tag_object: false,
            tag_signature: false,
            git_tag_doc: false,
        }
    }
}

/// Write one capture and describe it as a lock document.
fn capture(dir: &Path, name: &str, kind: &str, bytes: &[u8], attribution: Value) -> Value {
    std::fs::write(dir.join(name), bytes).unwrap();
    let owner = dir.file_name().unwrap().to_str().unwrap();
    json!({"kind": kind, "path": format!("sources/{owner}/{name}"),
           "size": bytes.len(), "sha256": sha(bytes), "attribution": {"observed": attribution}})
}

/// A lightweight-tag release pin with its captures written under a fresh
/// copy of the evals directory. Nothing is read from the validators: the
/// shapes are the GitHub REST documents the contract names.
fn release_pin(root: &Path, k: &Knobs) -> (PathBuf, Value) {
    let t = k.target;
    let evals = evals_copy(root);
    let sd = evals.join("sources").join(t.name());
    std::fs::create_dir_all(&sd).unwrap();
    let repo = t.official_home().to_owned();
    let home = repo.trim_start_matches("https://github.com/");
    let api = format!("https://api.github.com/repos/{home}");
    let greenmail = t == TargetKind::Greenmail;
    let exe = if greenmail {
        "greenmail-standalone-2.1.15.jar"
    } else {
        "authored-linux-amd64"
    };
    let exe_sha = "5".repeat(64);
    let release = json!({"url": format!("{api}/releases/42"), "tag_name": k.tag,
        "html_url": format!("{repo}/releases/tag/{}", k.tag), "target_commitish": "master",
        "published_at": "2026-05-01T00:00:00Z",
        "assets": [{"name": exe, "browser_download_url": format!("{repo}/releases/download/{}/{exe}", k.tag),
                    "size": 1234, "digest": null}]});
    let license = b"MIT License\n\nCopyright (c) authored\n".to_vec();
    let api_doc = b"# Authored API\n\nGET /mail lists messages.\n".to_vec();
    let setup = b"# Authored setup\n\nRun the binary with a config.\n".to_vec();
    let files: [(&str, &str, &Vec<u8>, &str); 3] = [
        ("authored-license.txt", "license", &license, "LICENSE"),
        ("authored-api.md", "api", &api_doc, "docs/api.md"),
        ("authored-setup.md", "setup", &setup, "docs/setup.md"),
    ];
    let mut docs = vec![capture(
        &sd,
        "authored-release.json",
        "release_metadata",
        &serde_json::to_vec(&release).unwrap(),
        json!({"from": "api", "url": format!("{api}/releases/42")}),
    )];
    let mut tree = Vec::new();
    for (name, kind, bytes, upstream) in files {
        docs.push(capture(
            &sd,
            name,
            kind,
            bytes,
            json!({"from": "commit",
            "url": format!("{repo}/blob/{C}/{upstream}"), "upstream_path": upstream,
            "git_blob": protocol::git_blob_id(bytes)}),
        ));
        if k.omit_from_tree != Some(name) {
            tree.push(json!({"path": upstream, "type": "blob", "sha": protocol::git_blob_id(bytes), "size": bytes.len()}));
        }
    }
    let ref_url = format!("{api}/git/refs/tags/{}", k.tag);
    let ref_body = json!({"ref": format!("refs/tags/{}", k.ref_name), "url": ref_url,
        "object": {"type": k.ref_type, "sha": k.ref_sha}});
    docs.push(capture(
        &sd,
        "authored-ref.json",
        "git_ref",
        &serde_json::to_vec(&ref_body).unwrap(),
        json!({"from": "api", "url": ref_url}),
    ));
    let tree_url = format!("{api}/git/trees/{C}");
    let tree_body = json!({"sha": C, "url": tree_url, "truncated": k.truncated, "tree": tree});
    docs.push(capture(
        &sd,
        "authored-tree.json",
        "git_tree",
        &serde_json::to_vec(&tree_body).unwrap(),
        json!({"from": "api", "url": tree_url}),
    ));
    if k.git_tag_doc {
        let tag_url = format!("{api}/git/tags/{}", "a".repeat(40));
        let body = json!({"sha": "a".repeat(40), "url": tag_url, "tag": k.tag,
            "object": {"type": "commit", "sha": C}, "verification": {"reason": "unsigned"}});
        docs.push(capture(
            &sd,
            "authored-tag.json",
            "git_tag",
            &serde_json::to_vec(&body).unwrap(),
            json!({"from": "api", "url": tag_url}),
        ));
    }
    let mut tagged = json!({"ref_kind": "lightweight", "commit": C});
    if k.tag_object {
        tagged["tag_object"] = "a".repeat(40).into();
    }
    if k.tag_signature {
        tagged["tag_signature"] = "unsigned".into();
    }
    let config = json!({"name": "config.json", "lines": [
        "{{", "  \"smtpPort\": {smtp_port}, \"servicePort\": {http_port}, \"wwwPort\": {ui_port}", "}}"]});
    let mut pin = json!({
        "release": {"repository": repo, "tag": k.tag, "html_url": format!("{repo}/releases/tag/{}", k.tag),
                    "target_commitish": "master", "published_at": "2026-05-01T00:00:00Z"},
        "acquisition": {"basis": "host_supplied_capture", "captured_on": "2026-11-06"},
        "tagged_source": {"observed": tagged},
        "binary_build_provenance": {"unresolved": "no build record ties the executable to the tag"},
        "artifact": {"kind": "executable", "file_name": exe,
            "download": format!("{repo}/releases/download/{}/{exe}", k.tag), "size": 1234, "sha256": exe_sha,
            "digest_basis": "locally_computed", "upstream_digest": null, "member": null,
            "executable_sha256": exe_sha, "os": "linux", "arch": "amd64",
            "version_output": format!("authored {}", k.tag.trim_start_matches("release-"))},
        "license": {"spdx": "MIT", "holder": "Authored"},
        "documents": docs,
        "launch": ["-config", "{config_path}", "--max", "10000"],
        "storage": {"backend": "scratch_file", "max_messages": 10000, "cap_flag": "--max", "basis": "authored"},
        "wait": {"reviewed": [], "finding": "no wait source was reviewed", "best_wait": {"unresolved": "none"}},
        "listeners": ["smtp", "http", "ui"],
        "config_file": config,
    });
    if greenmail {
        pin["launch"] = json!([
            "-Dp={smtp_port}",
            "-Dq={pop3_port}",
            "-jar",
            "{artifact}",
            "--max",
            "10000"
        ]);
        pin["listeners"] = json!(["smtp", "pop3"]);
        pin.as_object_mut().unwrap().remove("config_file");
        pin["runtime"] = json!({"name": "jre", "download": "https://example.invalid/jre.tar.gz",
            "sha256": "6".repeat(64), "digest_basis": "locally_computed",
            "checked_members": [{"path": "bin/java", "sha256": "7".repeat(64)},
                                {"path": "lib/server/libjvm.so", "sha256": "8".repeat(64)}],
            "version_output": "openjdk 21",
            "license": {"spdx": "GPL-2.0-only WITH Classpath-exception-2.0", "holder": "Oracle"}});
        pin["license"]["spdx"] = "Apache-2.0".into();
    }
    (
        evals,
        json!({"target": t.name(), "role": "competitor", "pin": pin}),
    )
}

/// The committed lock 4 with `t`'s entry replaced.
fn lock_with(t: TargetKind, entry: Value) -> SourceLock {
    let mut l = v4();
    let at = entry_of(&mut l, t.name());
    l["entries"][at] = entry;
    lock4(l)
}

fn authored(k: &Knobs) -> (tempfile::TempDir, PathBuf, SourceLock) {
    let root = tempfile::tempdir().unwrap();
    let (dir, entry) = release_pin(root.path(), k);
    let lock = lock_with(k.target, entry);
    (root, dir, lock)
}

#[test]
fn lightweight_pin_validates_from_authored_captures() {
    let (_root, dir, lock) = authored(&Knobs::new(TargetKind::Mailslurper));
    lock.validate().expect("a lightweight pin validates");
    lock.verify_files(&dir)
        .expect("the ref names the commit; the tree holds every document");
    let pin = lock.pin(TargetKind::Mailslurper).unwrap();
    let protocol::Fact::Observed(ts) = &pin.tagged_source else {
        panic!()
    };
    assert_eq!(ts.ref_kind, Some(protocol::RefKind::Lightweight));
    assert!(
        ts.tag_object.is_none() && ts.tag_signature.is_none(),
        "absent, not unsigned"
    );
    assert!(
        pin.documents
            .iter()
            .all(|d| d.kind != protocol::DocKind::GitTag)
    );
    for tag in ["release-1.15.0", "3.16.1", "v3.2.0"] {
        let mut k = Knobs::new(TargetKind::Mailslurper);
        k.tag = tag.into();
        k.ref_name = tag.into();
        let (_r, dir, lock) = authored(&k);
        lock.validate().unwrap_or_else(|e| panic!("{tag}: {e}"));
        lock.verify_files(&dir)
            .unwrap_or_else(|e| panic!("{tag}: {e}"));
    }
}

#[test]
fn lightweight_pin_refuses_tag_object_signature_or_git_tag_doc() {
    for (name, edit, want) in [
        (
            "tag object",
            (|k: &mut Knobs| k.tag_object = true) as fn(&mut Knobs),
            "no tag object and no signature",
        ),
        (
            "signature",
            |k| k.tag_signature = true,
            "no tag object and no signature",
        ),
        (
            "git_tag document",
            |k| k.git_tag_doc = true,
            "no git_tag document",
        ),
    ] {
        let mut k = Knobs::new(TargetKind::Mailslurper);
        edit(&mut k);
        let (_r, _d, lock) = authored(&k);
        let why = refusals(lock.validate());
        assert!(why.contains(want), "{name}: {why}");
    }
}

#[test]
fn lightweight_ref_of_type_tag_or_other_commit_refused() {
    for (name, edit) in [
        (
            "ref of type tag",
            (|k: &mut Knobs| k.ref_type = "tag".into()) as fn(&mut Knobs),
        ),
        ("another commit", |k| k.ref_sha = "9".repeat(40)),
        ("another tag", |k| k.ref_name = "release-9.9.9".into()),
    ] {
        let mut k = Knobs::new(TargetKind::Mailslurper);
        edit(&mut k);
        let (_r, dir, lock) = authored(&k);
        lock.validate()
            .expect("structure is fine; the captured bytes are what refuse");
        let why = refusals(lock.verify_files(&dir));
        assert!(
            why.contains("does not name commit") && why.contains("directly as refs/tags/"),
            "{name}: {why}"
        );
    }
}

#[test]
fn lightweight_truncated_tree_or_missing_document_refused() {
    let mut k = Knobs::new(TargetKind::Mailslurper);
    k.truncated = true;
    let (_r, dir, lock) = authored(&k);
    assert!(refusals(lock.verify_files(&dir)).contains("not the complete listing of commit"));
    let mut k = Knobs::new(TargetKind::Mailslurper);
    k.omit_from_tree = Some("authored-setup.md");
    let (_r, dir, lock) = authored(&k);
    let why = refusals(lock.verify_files(&dir));
    assert!(
        why.contains("authored-setup.md is not docs/setup.md in the tagged tree"),
        "{why}"
    );
    // A tag that is not a version is refused whatever the captures say.
    let mut k = Knobs::new(TargetKind::Mailslurper);
    k.tag = "latest".into();
    k.ref_name = "latest".into();
    let (_r, _d, lock) = authored(&k);
    assert!(refusals(lock.validate()).contains("is not a version tag"));
}

// ---- image provenance ----------------------------------------------------

#[derive(Clone, Default)]
struct ImageKnobs {
    wrong_index_digest: bool,
    wrong_manifest_digest: bool,
    index_names_other_platform: bool,
    manifest_omits_layer: bool,
    omit_manifest_doc: bool,
    observed_tagged_source: bool,
    observed_build_provenance: bool,
    manifest_omits_runtime_layer: bool,
}

fn image_pin(root: &Path, k: &ImageKnobs) -> (PathBuf, Value) {
    let t = TargetKind::AlkemioMailslurper;
    let evals = evals_copy(root);
    let sd = evals.join("sources").join(t.name());
    std::fs::create_dir_all(&sd).unwrap();
    let layer = "c".repeat(64);
    let listed = if k.manifest_omits_layer {
        "d".repeat(64)
    } else {
        layer.clone()
    };
    let loader = "f".repeat(64);
    let loader_listed = if k.manifest_omits_runtime_layer {
        "a".repeat(64)
    } else {
        loader.clone()
    };
    let manifest = json!({"schemaVersion": 2, "layers": [
        {"digest": format!("sha256:{listed}")}, {"digest": format!("sha256:{loader_listed}")}]});
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let m = sha(&manifest_bytes);
    let arch = if k.index_names_other_platform {
        "arm64"
    } else {
        "amd64"
    };
    let index = json!({"schemaVersion": 2, "manifests": [
        {"digest": format!("sha256:{m}"), "platform": {"os": "linux", "architecture": arch}},
        {"digest": format!("sha256:{}", "e".repeat(64)), "platform": {"os": "linux", "architecture": "s390x"}}]});
    let index_bytes = serde_json::to_vec(&index).unwrap();
    let i = sha(&index_bytes);
    let reference = protocol::ALKEMIO_IMAGE;
    let attr = json!({"from": "image", "reference": reference});
    let mut docs = vec![capture(
        &sd,
        "alkemio-index.json",
        "image_index",
        &index_bytes,
        attr.clone(),
    )];
    if !k.omit_manifest_doc {
        docs.push(capture(
            &sd,
            "alkemio-manifest.json",
            "image_manifest",
            &manifest_bytes,
            attr.clone(),
        ));
    }
    docs.push(capture(
        &sd,
        "alkemio-license.txt",
        "license",
        b"MIT License\n\nauthored\n",
        attr.clone(),
    ));
    docs.push(capture(
        &sd,
        "alkemio-api.md",
        "api",
        b"# authored api\n",
        attr,
    ));
    let exe_sha = "5".repeat(64);
    let bad = "b".repeat(64);
    let tagged = if k.observed_tagged_source {
        json!({"observed": {"ref_kind": "lightweight", "commit": C}})
    } else {
        json!({"unresolved": "an image has no tag"})
    };
    let build = if k.observed_build_provenance {
        json!({"observed": {"commit": C, "evidence": "sources/alkemio-mailslurper/alkemio-api.md"}})
    } else {
        json!({"unresolved": "the image states no build record"})
    };
    let pin = json!({
        "image": {"reference": reference,
                  "index_digest": format!("sha256:{}", if k.wrong_index_digest { &bad } else { &i }),
                  "manifest_digest": format!("sha256:{}", if k.wrong_manifest_digest { &bad } else { &m }),
                  "platform": "linux/amd64", "layer_digest": format!("sha256:{layer}"),
                  "member": "usr/local/bin/mailslurper"},
        "acquisition": {"basis": "host_supplied_capture", "captured_on": "2026-11-06"},
        "tagged_source": tagged,
        "binary_build_provenance": build,
        "artifact": {"kind": "executable", "file_name": "mailslurper", "download": reference, "size": 1234,
                     "sha256": exe_sha, "digest_basis": "locally_computed", "upstream_digest": null,
                     "member": "usr/local/bin/mailslurper", "executable_sha256": exe_sha,
                     "os": "linux", "arch": "amd64", "version_output": "mailslurper authored-patched"},
        "license": {"spdx": "MIT", "holder": "Authored"},
        "documents": docs,
        "launch": ["{artifact}", "--max", "10000"],
        "storage": {"backend": "scratch_file", "max_messages": 10000, "cap_flag": "--max", "basis": "authored"},
        "wait": {"reviewed": [], "finding": "an image has no tagged tree", "best_wait": {"unresolved": "none"}},
        "listeners": ["smtp", "http", "ui"],
        "config_file": {"name": "config.json", "lines": ["{{", "\"p\": {smtp_port} {http_port} {ui_port}", "}}"]},
        "runtime": {"name": "musl-authored",
            "download": format!("https://ghcr.io/v2/authored/blobs/sha256:{loader}"),
            "sha256": loader, "digest_basis": "locally_computed",
            "checked_members": [{"path": "lib/ld-musl-x86_64.so.1", "sha256": "9".repeat(64)}],
            "version_output": "musl libc authored",
            "license": {"spdx": "MIT", "holder": "authored"}},
    });
    (
        evals,
        json!({"target": t.name(), "role": "competitor", "pin": pin}),
    )
}

fn image_authored(k: &ImageKnobs) -> (tempfile::TempDir, PathBuf, SourceLock) {
    let root = tempfile::tempdir().unwrap();
    let (dir, entry) = image_pin(root.path(), k);
    (root, dir, lock_with(TargetKind::AlkemioMailslurper, entry))
}

#[test]
fn image_pin_validates_content_addressed_chain() {
    let (_r, dir, lock) = image_authored(&ImageKnobs::default());
    lock.validate().expect("an image pin validates");
    lock.verify_files(&dir)
        .expect("index -> manifest -> layer is content addressed");
    let pin = lock.pin(TargetKind::AlkemioMailslurper).unwrap();
    assert!(pin.release.is_none() && pin.image.is_some());
    assert!(pin.tagged_source.gap().is_some() && pin.binary_build_provenance.gap().is_some());
}

#[test]
fn image_pin_refuses_each_digest_mismatch_and_missing_manifest() {
    let cases: Vec<(&str, ImageKnobs, &str)> = vec![
        (
            "index digest",
            ImageKnobs {
                wrong_index_digest: true,
                ..Default::default()
            },
            "no captured ImageIndex document with digest sha256:",
        ),
        (
            "manifest digest",
            ImageKnobs {
                wrong_manifest_digest: true,
                ..Default::default()
            },
            "no captured ImageManifest document with digest sha256:",
        ),
        (
            "platform",
            ImageKnobs {
                index_names_other_platform: true,
                ..Default::default()
            },
            "does not name manifest",
        ),
        (
            "layer",
            ImageKnobs {
                manifest_omits_layer: true,
                ..Default::default()
            },
            "does not name layer",
        ),
    ];
    for (name, k, want) in cases {
        let (_r, dir, lock) = image_authored(&k);
        let why = match lock.validate() {
            Err(e) => e.0.join("\n"),
            Ok(()) => refusals(lock.verify_files(&dir)),
        };
        assert!(why.contains(want), "{name}: {why}");
    }
    let (_r, _d, lock) = image_authored(&ImageKnobs {
        omit_manifest_doc: true,
        ..Default::default()
    });
    assert!(refusals(lock.validate()).contains("no captured ImageManifest document"));
    // The captured bytes are what the digest names: altering either on disk
    // is refused whatever the lock says.
    for name in ["alkemio-index.json", "alkemio-manifest.json"] {
        let (_r, dir, lock) = image_authored(&ImageKnobs::default());
        let path = dir.join("sources/alkemio-mailslurper").join(name);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[5] ^= 1;
        std::fs::write(&path, bytes).unwrap();
        let why = refusals(lock.verify_files(&dir));
        assert!(
            why.contains(&format!("{name} does not match its digest")),
            "{name}: {why}"
        );
    }
}

#[test]
fn image_pin_refuses_observed_tagged_source_or_build_provenance() {
    let (_r, _d, lock) = image_authored(&ImageKnobs {
        observed_tagged_source: true,
        ..Default::default()
    });
    assert!(refusals(lock.validate()).contains("an image pin has no tagged source to observe"));
    let (_r, _d, lock) = image_authored(&ImageKnobs {
        observed_build_provenance: true,
        ..Default::default()
    });
    assert!(refusals(lock.validate()).contains("no binary build provenance to observe"));
    // The image must be the labelled source, not any image.
    let root = tempfile::tempdir().unwrap();
    let (_dir, mut entry) = image_pin(root.path(), &ImageKnobs::default());
    entry["pin"]["image"]["reference"] = "ghcr.io/other/mailslurper:latest".into();
    let why = refusals(lock_with(TargetKind::AlkemioMailslurper, entry).validate());
    assert!(why.contains("is not the labelled source"), "{why}");
}

// ---- runtime pin and licenses -------------------------------------------

#[test]
fn runtime_license_checked_against_runtime_list_only() {
    let jre = "GPL-2.0-only WITH Classpath-exception-2.0";
    let (_r, dir, lock) = authored(&Knobs::new(TargetKind::Greenmail));
    lock.validate()
        .expect("a greenmail pin with its JRE validates");
    lock.verify_files(&dir).expect("and verifies");
    // The runtime's own license must be on the runtime list.
    for bad in ["GPL-3.0-only", "GPL-2.0-only", "Apache-2.0", ""] {
        let root = tempfile::tempdir().unwrap();
        let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
        entry["pin"]["runtime"]["license"]["spdx"] = bad.into();
        let why = refusals(lock_with(TargetKind::Greenmail, entry).validate());
        assert!(why.contains("is not on the runtime list"), "{bad:?}: {why}");
    }
    // The runtime expression never stands in for a measured product's
    // license: the permissive list is not consulted for the JRE, and the
    // JRE's expression is not admitted on it.
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
    entry["pin"]["license"]["spdx"] = jre.into();
    let why = refusals(lock_with(TargetKind::Greenmail, entry).validate());
    assert!(
        why.contains(&format!("license \"{jre}\" is not on the permissive list")),
        "{why}"
    );
}

#[test]
fn permissive_list_is_unchanged() {
    assert_eq!(
        protocol::PERMISSIVE,
        ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"]
    );
}

#[test]
fn runtime_pin_refused_on_target_other_than_greenmail() {
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Mailslurper));
    let other = tempfile::tempdir().unwrap();
    let (_g, greenmail) = release_pin(other.path(), &Knobs::new(TargetKind::Greenmail));
    entry["pin"]["runtime"] = greenmail["pin"]["runtime"].clone();
    entry["pin"]["launch"]
        .as_array_mut()
        .unwrap()
        .push("{artifact}".into());
    let why = refusals(lock_with(TargetKind::Mailslurper, entry).validate());
    assert!(
        why.contains(
            "only greenmail (a JRE) and alkemio-mailslurper (a musl loader) pin a runtime"
        ),
        "{why}"
    );
    // And greenmail without one, or with its checked members short.
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
    entry["pin"].as_object_mut().unwrap().remove("runtime");
    assert!(
        refusals(lock_with(TargetKind::Greenmail, entry).validate())
            .contains("needs a pinned runtime")
    );
    for drop in ["bin/java", "lib/server/libjvm.so"] {
        let root = tempfile::tempdir().unwrap();
        let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
        let members = entry["pin"]["runtime"]["checked_members"]
            .as_array_mut()
            .unwrap();
        members.retain(|m| m["path"] != drop);
        let why = refusals(lock_with(TargetKind::Greenmail, entry).validate());
        assert!(
            why.contains(&format!("the runtime does not check {drop}")),
            "{why}"
        );
    }
    // More than eight members, a repeated member and an unconfined path.
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
    let members = entry["pin"]["runtime"]["checked_members"]
        .as_array_mut()
        .unwrap();
    for n in 0..7 {
        members.push(json!({"path": format!("lib/m{n}.so"), "sha256": "1".repeat(64)}));
    }
    members.push(json!({"path": "bin/java", "sha256": "1".repeat(64)}));
    members.push(json!({"path": "../escape", "sha256": "1".repeat(64)}));
    let why = refusals(lock_with(TargetKind::Greenmail, entry).validate());
    assert!(
        why.contains("more than 8") && why.contains("listed twice") && why.contains("is malformed"),
        "{why}"
    );
}

// ---- listeners, environment and config -----------------------------------

fn mailslurper_pin_with(edit: impl Fn(&mut Value)) -> String {
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Mailslurper));
    edit(&mut entry["pin"]);
    match lock_from(json!({"lock_version": 4, "checked_on": "2026-11-07", "entries": [entry]})) {
        Ok(lock) => lock
            .validate()
            .err()
            .map(|r| r.0.join("\n"))
            .unwrap_or_default(),
        Err(e) => e.0.join("\n"),
    }
}

#[test]
fn listener_roles_closed_unique_and_template_complete() {
    // The pin as authored validates, so each refusal below is its edit's.
    let ok = mailslurper_pin_with(|_| {});
    assert!(!ok.contains("listener") && !ok.contains("slot"), "{ok}");
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["smtp", "http", "http", "ui"]));
    assert!(why.contains("listener role http is listed twice"), "{why}");
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["smtp", "http", "ui2"]));
    assert!(
        why.contains("unknown variant"),
        "a role outside the closed set never parses: {why}"
    );
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["http", "ui"]));
    assert!(why.contains("lack the smtp role"), "{why}");
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["smtp", "ui"]));
    assert!(
        why.contains("lack the http role its read path uses"),
        "{why}"
    );
    // An undeclared slot, and a declared listener the launch never gets.
    let why = mailslurper_pin_with(|p| {
        p["config_file"]["lines"]
            .as_array_mut()
            .unwrap()
            .push("{imap_port}".into())
    });
    assert!(why.contains("names a slot the declared listeners"), "{why}");
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["smtp", "http", "ui", "imap"]));
    assert!(
        why.contains("the declared imap listener is never given to the launch"),
        "{why}"
    );
    let why = mailslurper_pin_with(|p| p["launch"].as_array_mut().unwrap().push("{nope}".into()));
    assert!(why.contains("names a slot"), "{why}");
}

#[test]
fn launch_env_refuses_owned_and_malformed_names() {
    for name in ["PATH", "HOME", "TMPDIR", "LANG", "XDG_CONFIG_HOME", "XDG_X"] {
        let why = mailslurper_pin_with(|p| p["launch_env"] = json!([[name, "x"]]));
        assert!(
            why.contains(&format!("environment name {name} is owned by the launcher")),
            "{name}: {why}"
        );
    }
    for name in ["lower", "A-B", "1A", "", "A B", &"A".repeat(65)] {
        let why = mailslurper_pin_with(|p| p["launch_env"] = json!([[name, "x"]]));
        assert!(why.contains("is not upper snake case"), "{name:?}: {why}");
    }
    let why = mailslurper_pin_with(|p| p["launch_env"] = json!([["A_B", "x"], ["A_B", "y"]]));
    assert!(why.contains("is listed twice"), "{why}");
    let why =
        mailslurper_pin_with(|p| p["launch_env"] = json!([["INBUCKET_SMTP_ADDR", "{smtp_addr}"]]));
    assert!(
        !why.contains("environment"),
        "an upper-snake name with a declared slot is accepted: {why}"
    );
    let why = mailslurper_pin_with(|p| p["launch_env"] = json!([["X", "{pop3_addr}"]]));
    assert!(why.contains("names a slot"), "{why}");
}

#[test]
fn config_file_name_confined() {
    for name in ["a/b", "..", "../x", "", "/etc/passwd", "a..b"] {
        let why = mailslurper_pin_with(|p| p["config_file"]["name"] = name.into());
        assert!(
            why.contains("must be one plain leaf name"),
            "{name:?}: {why}"
        );
    }
    let why = mailslurper_pin_with(|p| p["config_file"]["lines"] = json!([]));
    assert!(why.contains("1 to 64 lines"), "{why}");
    // A launch naming {config_path} needs a config file to name.
    let why = mailslurper_pin_with(|p| {
        p.as_object_mut().unwrap().remove("config_file");
    });
    assert!(why.contains("states no config file"), "{why}");
    // The brace escapes are the only way to write a brace.
    let why = mailslurper_pin_with(|p| {
        p["config_file"]["lines"] = json!(["{", "}"]);
    });
    assert!(why.contains("names a slot"), "{why}");
}

// ---- pending entries ----------------------------------------------------

#[test]
fn pending_entry_validates_but_refuses_collection_and_launch() {
    let lock = lock4(pending_v4());
    lock.validate()
        .expect("pending entries are valid structure");
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    p.validate().expect("the draft protocol is valid");
    for t in PENDING {
        let e = lock.entry(t).unwrap();
        assert!(e.pin.is_none() && e.pending.is_some(), "{t}");
        assert!(lock.pin(t).is_none());
    }
    // `verify_files` has no bytes to check for them and is not a gate.
    lock.verify_files(&evals_dir())
        .expect("pending entries have no bytes");
    // Collection and launch both refuse each one by name.
    let mut frozen = p.clone();
    frozen.status = protocol::Status::Frozen;
    frozen.unresolved.clear();
    let why = refusals(protocol::collection_ready(&frozen, &lock));
    let launch = refusals(protocol::launchable(&p, &lock));
    for t in PENDING {
        assert!(
            why.contains(&format!("{t}: pending, no verified pin")),
            "{t}: {why}"
        );
        assert!(
            launch.contains(&format!(
                "{t} has no source pin; it cannot be launched (pending: "
            )),
            "{t}: {launch}"
        );
    }
    // Both and neither are refused.
    let mut l = pending_v4();
    let at = entry_of(&mut l, "mailslurper");
    let root = tempfile::tempdir().unwrap();
    let (_dir, pinned) = release_pin(root.path(), &Knobs::new(TargetKind::Mailslurper));
    l["entries"][at]["pin"] = pinned["pin"].clone();
    assert!(refusals(lock4(l).validate()).contains("a pin or pending, never both"));
    let mut l = pending_v4();
    let at = entry_of(&mut l, "mailslurper");
    l["entries"][at].as_object_mut().unwrap().remove("pending");
    assert!(refusals(lock4(l).validate()).contains("mailslurper: a competitor needs a pin"));
    // A pending entry states what kind of source it is and why.
    let mut l = pending_v4();
    let at = entry_of(&mut l, "smtp4dev");
    l["entries"][at]["pending"]["release_tag"] = "latest".into();
    l["entries"][at]["pending"]["reason"] = " ".into();
    let why = refusals(lock4(l).validate());
    assert!(
        why.contains("is not a version tag") && why.contains("states no reason"),
        "{why}"
    );
    let mut l = pending_v4();
    let at = entry_of(&mut l, "alkemio-mailslurper");
    l["entries"][at]["pending"]["image"] = "ghcr.io/other/x:1".into();
    assert!(refusals(lock4(l).validate()).contains("is not the labelled"));
}

// ---- protocol 3 -----------------------------------------------------------

fn p3() -> Value {
    json(PROTOCOL_V3)
}

/// The position of `target`'s record in the protocol's `key` list.
fn idx(v: &Value, key: &str, target: &str) -> usize {
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .position(|x| x["target"] == target)
        .unwrap()
}

fn v3_refusals(f: impl Fn(&mut Value)) -> String {
    let mut v = p3();
    f(&mut v);
    match protocol_from(v) {
        Ok(p) => p
            .validate()
            .err()
            .map(|r| r.0.join("\n"))
            .unwrap_or_default(),
        Err(e) => e.0.join("\n"),
    }
}

#[test]
fn current_protocol_v3_shape_and_v2_workloads_equal() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    p.validate().expect("the committed protocol 3 validates");
    assert_eq!(p.protocol_version, 3);
    assert_eq!(p.id, "fair-mail-comparison-v3");
    assert_eq!(p.supersedes.as_deref(), Some("fair-mail-comparison-v2"));
    assert!(p.change_reason.as_deref().unwrap().contains("0130"));
    assert_eq!(p.targets, TargetKind::ALL.to_vec());
    // Every target has one retention row, one product, one capability record
    // and one observation per wait profile.
    for t in TargetKind::ALL {
        assert_eq!(
            p.storage.retention.iter().filter(|r| r.target == t).count(),
            1,
            "{t}"
        );
        assert_eq!(
            p.products
                .iter()
                .flatten()
                .filter(|x| x.target == t)
                .count(),
            1,
            "{t}"
        );
        assert_eq!(
            p.capabilities
                .iter()
                .flatten()
                .filter(|x| x.target == t)
                .count(),
            1,
            "{t}"
        );
        for w in &p.waits {
            assert_eq!(
                w.observation.iter().filter(|o| o.target == t).count(),
                1,
                "{t}"
            );
        }
    }
    // The throughput and wait profiles, corpus, budgets and resources are
    // the history v2 bytes, field by field.
    let (old, new) = (json(PROTOCOL), p3());
    for key in ["corpus", "throughput", "budgets", "resources"] {
        assert_eq!(old[key], new[key], "{key}");
    }
    assert_eq!(
        old["statistics"]["quantile_method"],
        new["statistics"]["quantile_method"]
    );
    assert_eq!(
        old["statistics"]["quantiles"],
        new["statistics"]["quantiles"]
    );
    let (ow, nw) = (&old["waits"][0], &new["waits"][0]);
    for key in [
        "name",
        "repetitions",
        "present",
        "future",
        "empty",
        "deadline_ms",
        "empty_deadline_ms",
    ] {
        assert_eq!(ow[key], nw[key], "wait {key}");
    }
    for i in 0..3 {
        assert_eq!(
            ow["observation"][i], nw["observation"][i],
            "observation {i}"
        );
    }
    // The orders are the nine cyclic rotations of the target list.
    let orders = p.statistics.orders.clone();
    assert_eq!(orders.len(), 9);
    for (k, order) in orders.iter().enumerate() {
        let want: Vec<TargetKind> = (0..9).map(|i| p.targets[(i + k) % 9]).collect();
        assert_eq!(*order, want);
    }
    // Every cap is the value set at launch, above every residency, or none.
    for r in &p.storage.retention {
        assert!(r.max_messages.is_none_or(|c| c >= 3100), "{}", r.target);
    }
}

#[test]
fn protocol_v3_requires_all_eight_and_each_section() {
    for t in &NEW_TARGETS {
        let why = v3_refusals(|v| {
            v["targets"]
                .as_array_mut()
                .unwrap()
                .retain(|x| x != t.name());
        });
        assert!(
            why.contains(&format!("protocol_version 3 needs target {t}")),
            "{t}: {why}"
        );
    }
    let why = v3_refusals(|v| {
        v["capabilities"]
            .as_array_mut()
            .unwrap()
            .retain(|c| c["target"] != "inbucket");
    });
    assert!(
        why.contains("capabilities: inbucket needs exactly one record"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["capabilities"][0]
            .as_object_mut()
            .unwrap()
            .remove("native_wait");
    });
    assert!(
        why.contains("native_wait"),
        "a missing capability key never parses: {why}"
    );
    let why = v3_refusals(|v| {
        let at = idx(v, "products", "smtp4dev");
        v["products"][at].as_object_mut().unwrap().remove("scope");
    });
    assert!(why.contains("products: smtp4dev states no scope"), "{why}");
    let why = v3_refusals(|v| {
        v.as_object_mut().unwrap().remove("capabilities");
    });
    assert!(
        why.contains("capabilities: swarmail needs exactly one record"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["storage"]["retention"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["target"] != "greenmail");
    });
    assert!(
        why.contains("storage: greenmail needs exactly one retention entry"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["waits"][0]["observation"]
            .as_array_mut()
            .unwrap()
            .retain(|o| o["target"] != "smtp4dev");
    });
    assert!(
        why.contains("smtp4dev needs exactly one observation mode"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["statistics"]["orders"].as_array_mut().unwrap().pop();
    });
    assert!(
        why.contains("not the 9 cyclic rotations") && why.contains("rotation 8 is missing"),
        "{why}"
    );
}

#[test]
fn capability_without_or_with_foreign_citation_refused() {
    let why = v3_refusals(|v| v["capabilities"][1]["server_search"]["cite"] = Value::Null);
    assert!(
        why.contains("mailpit server_search is Supported without a citation"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["capabilities"][1]["native_wait"] = json!({"status": "not_supported", "cite": ""});
    });
    assert!(
        why.contains("mailpit native_wait is NotSupported without a citation"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["capabilities"][3]["server_search"] = json!({"status": "not_inspected", "cite": "x"});
    });
    assert!(
        why.contains("mailslurper server_search is not_inspected, so it cites nothing"),
        "{why}"
    );
    // A citation names a document or reviewed source of that target's pin.
    let p = |f: &dyn Fn(&mut Value)| {
        let mut v = p3();
        f(&mut v);
        protocol_from(v).unwrap()
    };
    let lock = lock4(v4());
    protocol::capabilities_match(&p(&|_| {}), &lock).expect("the committed citations all resolve");
    let foreign = p(&|v| {
        v["capabilities"][2]["server_search"]["cite"] =
            "sources/mailpit/mailpit-swagger.json".into();
    });
    let why = refusals(protocol::capabilities_match(&foreign, &lock));
    assert!(why.contains("mailhog server_search cites \"sources/mailpit/mailpit-swagger.json\", which is no document"), "{why}");
    let pending = p(&|v| {
        let at = idx(v, "capabilities", "greenmail");
        v["capabilities"][at]["raw_source"] =
            json!({"status": "supported", "cite": "sources/greenmail/x.md"});
    });
    // A row with no pin has nothing to cite (an authored lock without it).
    let mut without = v4();
    let at = entry_of(&mut without, "greenmail");
    without["entries"][at]["pin"] = Value::Null;
    let why = refusals(protocol::capabilities_match(&pending, &lock4(without)));
    assert!(
        why.contains("greenmail raw_source cites") && why.contains("has no pin to cite"),
        "{why}"
    );
    let repo = p(&|v| {
        v["capabilities"][0]["native_wait"]["cite"] =
            "docs/decisions/0002-awaits-subscribe-before-checking.md".into();
    });
    assert!(refusals(protocol::capabilities_match(&repo, &lock)).contains("not a repo: path"));
    // A stated cap agrees with the flag the launch sets.
    let capped = p(&|v| {
        v["capabilities"][2]["retention_cap"] =
            json!({"status": "supported", "cite": "sources/mailhog/mailhog-readme.md"});
    });
    assert!(
        refusals(protocol::capabilities_match(&capped, &lock))
            .contains("mailhog retention_cap is supported but its launch sets no cap")
    );
}

#[test]
fn product_scope_must_match_provenance() {
    let why = v3_refusals(|v| {
        let at = idx(v, "products", "alkemio-mailslurper");
        v["products"][at]["scope"] = "official_artifact".into();
    });
    assert!(
        why.contains(
            "alkemio-mailslurper scope OfficialArtifact is not its provenance's ThirdPartyImage"
        ),
        "{why}"
    );
    let why = v3_refusals(|v| v["products"][1]["scope"] = "third_party_image".into());
    assert!(
        why.contains("mailpit scope ThirdPartyImage is not its provenance's OfficialArtifact"),
        "{why}"
    );
    let why = v3_refusals(|v| v["products"][0]["scope"] = "official_artifact".into());
    assert!(
        why.contains("swarmail scope OfficialArtifact is not its provenance's Candidate"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        let at = idx(v, "products", "alkemio-mailslurper");
        v["products"][at]["release_tag"] = "v1.0.0".into();
    });
    assert!(
        why.contains("alkemio-mailslurper is a third-party image: it has no release tag"),
        "{why}"
    );
    // The lock's pin source must agree with the scope, once pinned.
    let root = tempfile::tempdir().unwrap();
    let (_dir, entry) = image_pin(root.path(), &ImageKnobs::default());
    let lock = lock_with(TargetKind::AlkemioMailslurper, entry);
    let mut v = p3();
    let at = idx(&v, "products", "alkemio-mailslurper");
    v["products"][at]["executable_sha256"] = "5".repeat(64).into();
    let ok = protocol_from(v.clone()).unwrap();
    protocol::products_match(&ok, &lock).expect("an image pin matches its image scope");
    v["products"][at]["scope"] = "official_artifact".into();
    let why = refusals(protocol::products_match(&protocol_from(v).unwrap(), &lock));
    assert!(
        why.contains("protocol scope Some(OfficialArtifact) is not what the lock's pin is"),
        "{why}"
    );
}

#[test]
fn v3_reusing_v2_id_refused() {
    for id in [
        "fair-mail-comparison-v2",
        "fair-mail-comparison-draft-3",
        "fair-mail-comparison-draft-2",
    ] {
        let why = v3_refusals(|v| v["id"] = id.into());
        assert!(
            why.contains("reuses a historical protocol id"),
            "{id}: {why}"
        );
    }
    let why = v3_refusals(|v| v["supersedes"] = "fair-mail-comparison-draft-3".into());
    assert!(
        why.contains("protocol 3 supersedes fair-mail-comparison-v2"),
        "{why}"
    );
    let why = v3_refusals(|v| v["change_reason"] = "wider".into());
    assert!(why.contains("must cite decision 0130"), "{why}");
}

#[test]
fn v3_cap_below_workload_refused() {
    let why = v3_refusals(|v| {
        let at = v["storage"]["retention"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r["target"] == "smtp4dev")
            .unwrap();
        v["storage"]["retention"][at]["max_messages"] = 100.into();
    });
    assert!(
        why.contains("3100 messages overflow smtp4dev's retention"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        let at = v["storage"]["retention"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r["target"] == "inbucket")
            .unwrap();
        v["storage"]["retention"][at]["max_messages"] = 3000.into();
    });
    assert!(
        why.contains("3100 messages overflow inbucket's retention"),
        "{why}"
    );
}

#[test]
fn v2_protocol_refuses_v3_sections() {
    let mut v = json(PROTOCOL);
    v["capabilities"] = p3()["capabilities"].clone();
    let why = refusals(protocol_from(v).unwrap().validate());
    assert!(
        why.contains("protocol_version 2 carries version 3 sections"),
        "{why}"
    );
    let mut v = json(PROTOCOL);
    v["products"][1]["scope"] = "official_artifact".into();
    let why = refusals(protocol_from(v).unwrap().validate());
    assert!(why.contains("carries version 3 sections"), "{why}");
}

#[test]
fn frozen_only_when_nothing_unresolved_and_no_pending() {
    let lock = lock4(v4());
    let pending = lock4(pending_v4());
    let frozen = Protocol::parse(PROTOCOL_V3).unwrap();
    // The committed protocol is frozen with nothing unresolved.
    assert_eq!(frozen.status, protocol::Status::Frozen);
    assert!(frozen.unresolved.is_empty());
    protocol::collection_ready(&frozen, &lock).expect("the committed pair is ready");
    // A draft says what it lacks, and a lock that still has pending rows
    // refuses each by name even once the protocol is frozen.
    let mut draft = frozen.clone();
    draft.status = protocol::Status::Draft;
    draft.unresolved = vec!["mailslurper: authored gap".into()];
    let why = refusals(protocol::collection_ready(&draft, &lock));
    assert!(why.contains("protocol is a draft, not frozen"), "{why}");
    assert!(
        why.contains("unresolved: mailslurper: authored gap"),
        "{why}"
    );
    let why = refusals(protocol::collection_ready(&frozen, &pending));
    for t in PENDING {
        assert!(
            why.contains(&format!("{t}: pending, no verified pin")),
            "{t}: {why}"
        );
    }
    // Freezing it with entries still unresolved is itself refused.
    let why = v3_refusals(|v| v["unresolved"] = json!(["authored gap"]));
    assert!(
        why.contains("a frozen protocol has nothing unresolved"),
        "{why}"
    );
    assert!(!why.contains("pending, no verified pin"));
    // The unenforced profile and unmatched storage stay labelled.
    assert_eq!(
        frozen.storage.profile,
        Some(protocol::StorageProfile::Unmatched)
    );
    assert_eq!(
        frozen.resources.profile,
        Some(protocol::ResourceProfile::UnenforcedLocal)
    );
}

#[test]
fn current_lock_v4_validates_and_verifies() {
    let lock = lock4(v4());
    lock.validate().expect("the committed lock validates");
    lock.verify_files(&evals_dir())
        .expect("and its captured documents verify");
    assert_eq!(lock.lock_version, 4);
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    protocol::pairing(&p, &lock).unwrap();
    protocol::retention_matches(&p, &lock).expect("retention is as launched");
    protocol::products_match(&p, &lock).expect("products are the locked executables");
    protocol::capabilities_match(&p, &lock).expect("citations resolve");
    protocol::does_not_start_matches(&p, &lock).expect("the published result cites its pin");
    // Every competitor is pinned, none is pending, and each has a capture
    // directory of its own.
    for t in protocol::TargetKind::ALL.into_iter().skip(1) {
        let e = lock.entry(t).unwrap();
        assert!(e.pin.is_some() && e.pending.is_none(), "{t}");
        assert!(evals_dir().join("sources").join(t.name()).is_dir(), "{t}");
    }
    // Every tagged source is observed, every document attribution too.
    for t in [
        TargetKind::Mailslurper,
        TargetKind::MailslurperSourceBuild,
        TargetKind::Smtp4dev,
        TargetKind::Inbucket,
        TargetKind::Greenmail,
    ] {
        let pin = lock.pin(t).unwrap();
        assert!(pin.tagged_source.gap().is_none(), "{t}");
        assert!(
            pin.documents.iter().all(|d| d.attribution.gap().is_none()),
            "{t}"
        );
    }
}

#[test]
fn committed_protocol_v3_is_ready_for_collection() {
    // The same two checks `mail_compare check` runs over the committed files.
    let lock = lock4(v4());
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    protocol::collection_ready(&p, &lock).expect("ready for collection");
    lock.verify_files(&evals_dir())
        .expect("and its documents verify");
    assert_eq!(p.status, protocol::Status::Frozen);
    assert!(p.unresolved.is_empty());
    // Every capability is stated with a citation or not_inspected.
    for c in p.capabilities.iter().flatten() {
        for (key, cap) in c.each() {
            match cap.status {
                protocol::CapabilityStatus::NotInspected => assert!(cap.cite.is_none()),
                _ => assert!(cap.cite.is_some(), "{} {key}", c.target),
            }
        }
    }
    assert_eq!(
        p.claim.as_ref().unwrap().scope,
        protocol::ClaimScope::OfficialArtifact
    );
}

/// What `mail_compare check` prints while gaps remain: it refuses, and it
/// names each one (a pending row, an unresolved tagged source, and every line
/// of the protocol's own unresolved list). Authored from the committed pair.
#[test]
fn committed_check_lists_every_open_gap() {
    let mut l = pending_v4();
    let at = entry_of(&mut l, "inbucket");
    l["entries"][at]["pin"]["tagged_source"] = json!({"unresolved": "authored gap"});
    let lock = lock4(l);
    let mut p = Protocol::parse(PROTOCOL_V3).unwrap();
    p.status = protocol::Status::Draft;
    p.unresolved = vec!["inbucket: authored protocol gap".into()];
    let why = refusals(protocol::collection_ready(&p, &lock));
    assert!(why.contains("protocol is a draft, not frozen"), "{why}");
    for t in PENDING {
        assert!(
            why.contains(&format!("{t}: pending, no verified pin; it cannot collect")),
            "{t}: {why}"
        );
    }
    assert!(
        why.contains("inbucket: tagged source unresolved: authored gap"),
        "the pin's own gap is named: {why}"
    );
    assert!(!why.contains("inbucket: pending"), "{why}");
    for gap in &p.unresolved {
        assert!(why.contains(&format!("unresolved: {gap}")), "{gap}: {why}");
    }
}

#[test]
fn declared_competitors_come_from_the_protocol() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let names: Vec<&str> = protocol::declared_competitors(&p)
        .iter()
        .map(|t| t.name())
        .collect();
    assert_eq!(
        names,
        [
            "mailpit",
            "mailhog",
            "mailslurper-source-build",
            "smtp4dev",
            "inbucket",
            "greenmail",
            "alkemio-mailslurper"
        ]
    );
    let old = Protocol::parse(PROTOCOL).unwrap();
    assert_eq!(
        protocol::declared_competitors(&old),
        [TargetKind::Mailpit, TargetKind::Mailhog]
    );
}

// ---- the report ---------------------------------------------------------

#[test]
fn report_refuses_merged_v2_v3_runs() {
    let p2 = Protocol::parse(PROTOCOL).unwrap();
    let p3 = Protocol::parse(PROTOCOL_V3).unwrap();
    let m2 = manifest(&p2, "run-2");
    let m3 = manifest(&p3, "run-3");
    report::one_denominator(&[&m2, &m2]).expect("one protocol is one denominator");
    let why = report::one_denominator(&[&m2, &m3]).unwrap_err();
    assert!(
        why.contains("fair-mail-comparison-v2 and fair-mail-comparison-v3")
            && why.contains("separate denominators"),
        "{why}"
    );
}

#[test]
fn row_labels_present_for_each_target() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let labels = report::row_labels(&p);
    assert_eq!(labels.len(), 9);
    let of = |t: TargetKind| labels.iter().find(|l| l.target == t).unwrap();
    assert_eq!(
        of(TargetKind::AlkemioMailslurper).source,
        "third-party image (extracted binary)"
    );
    assert_eq!(of(TargetKind::Mailslurper).source, "official release");
    assert_eq!(of(TargetKind::Swarmail).source, "candidate");
    assert_eq!(of(TargetKind::Greenmail).read_path, "pop3");
    assert_eq!(of(TargetKind::Inbucket).read_path, "http-json");
    for t in NEW_TARGETS {
        assert_eq!(of(t).search, "client-side-filter", "{t}");
    }
    for t in TargetKind::V2 {
        assert_eq!(of(t).search, "server", "{t}");
    }
    assert!(labels.iter().all(|l| l.storage_backend.is_some()));
    assert_ne!(
        of(TargetKind::Mailslurper).source,
        of(TargetKind::AlkemioMailslurper).source
    );
}

// ---- lifecycle: listeners, environment, config file, checked members -----

use protocol::ListenerRole as Role;
use targets::{LaunchError, LaunchSpec};

/// A scripted child: `/bin/sh` runs the rendered config file, whose lines are
/// the script (argv elements cannot hold spaces; config lines can). The shell
/// is an existing executable, so no freshly written script is ever "text file
/// busy" while a parallel test forks.
fn declared(kind: TargetKind, roles: &[Role], script: &[&str]) -> LaunchSpec {
    let mut spec = LaunchSpec::new(
        kind,
        PathBuf::from("/bin/sh"),
        None,
        vec!["{config_path}".into()],
    );
    spec.extra.listeners = roles.to_vec();
    spec.extra.config = Some(protocol::ConfigFile {
        name: "run.sh".into(),
        lines: script.iter().map(|l| (*l).to_owned()).collect(),
        cwd: false,
    });
    spec
}

/// The pid marker a scripted child ends with, written atomically.
const MARK_STARTED: &str =
    "echo $$ > \"$HOME/pid.tmp\"; mv \"$HOME/pid.tmp\" \"$HOME/pid\"; exec sleep 600";

fn candidate_with_pop3_slot() -> Vec<String> {
    let mut t = targets::swarmail_template(0);
    let at = t.iter().position(|a| a == "--pop3-listen").unwrap();
    t[at + 1] = "{pop3_addr}".into();
    t
}

async fn launch_err(spec: &LaunchSpec, root: &Path) -> LaunchError {
    let clock = wire::Clock(Instant::now());
    targets::Owned::launch(
        spec,
        &root.join("leaf"),
        &short_limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await
    .err()
    .expect("the launch must refuse")
}

#[tokio::test]
async fn launch_template_must_use_exactly_the_declared_slots() {
    let root = tempfile::tempdir().unwrap();
    let all = [Role::Smtp, Role::Http];
    // An undeclared slot, an omitted declared listener, a repeated role and
    // a roleless launch are refused before anything starts.
    for (roles, body) in [
        (&all[..], "echo {smtp_addr} {http_addr} {imap_port}"),
        (&all[..], "echo {smtp_addr}"),
        (
            &[Role::Smtp, Role::Http, Role::Imap][..],
            "echo {smtp_addr} {http_addr}",
        ),
        (
            &[Role::Smtp, Role::Smtp, Role::Http][..],
            "echo {smtp_addr} {http_addr}",
        ),
        (
            &[Role::Smtp, Role::Pop3][..],
            "echo {smtp_addr} {pop3_addr}",
        ),
    ] {
        // The scripted child is the one that would have run; it only has
        // a Mailpit-shaped read path, so the pop3 case is Greenmail's.
        let kind = if roles.contains(&Role::Pop3) {
            TargetKind::Greenmail
        } else {
            TargetKind::Mailpit
        };
        let spec = declared(kind, roles, &[body]);
        let spec = if roles == [Role::Smtp, Role::Pop3] {
            // Declared right, but the pop3 slot is used and the http one is not
            // declared: the only refusal is the undeclared `{http_addr}`.
            declared(kind, roles, &["echo {smtp_addr} {pop3_addr} {http_addr}"])
        } else {
            spec
        };
        assert_eq!(
            launch_err(&spec, root.path()).await,
            LaunchError::Template,
            "{roles:?} {body}"
        );
        assert!(!root.path().join("leaf").exists(), "nothing was created");
    }
    // Greenmail reads over POP3 and so needs that role.
    let spec = declared(
        TargetKind::Greenmail,
        &all,
        &["echo {smtp_addr} {http_addr}"],
    );
    assert_eq!(launch_err(&spec, root.path()).await, LaunchError::Template);
}

#[tokio::test]
async fn checked_member_or_jar_mismatch_refuses_launch() {
    let root = tempfile::tempdir().unwrap();
    let member = root.path().join("libjvm.so");
    let jar = root.path().join("greenmail.jar");
    std::fs::write(&member, b"authored member bytes").unwrap();
    std::fs::write(&jar, b"authored jar bytes").unwrap();
    let good_member = sha(b"authored member bytes");
    let good_jar = sha(b"authored jar bytes");
    let mk = |member_sha: &str, jar_sha: &str| {
        let mut s = declared(
            TargetKind::Mailpit,
            &[Role::Smtp, Role::Http],
            &["echo {smtp_addr} {http_addr} {artifact}"],
        );
        s.expected_sha256 = Some(jar_sha.to_owned());
        s.extra.artifact = Some(jar.clone());
        s.extra.members = vec![(member.clone(), member_sha.to_owned())];
        s
    };
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        launch_err(&mk(&"0".repeat(64), &good_jar), dir.path()).await,
        LaunchError::MemberDigestMismatch
    );
    assert_eq!(
        launch_err(&mk(&good_member, &"0".repeat(64)), dir.path()).await,
        LaunchError::DigestMismatch
    );
    assert!(
        !dir.path().join("leaf").exists(),
        "a refused launch creates no scratch"
    );
    // Both right: the launch starts and only readiness fails (nothing listens).
    let why = launch_err(&mk(&good_member, &good_jar), dir.path()).await;
    assert!(matches!(why, LaunchError::NotReady(..)), "{why:?}");
    // The seal recheck: replacing either file after launch is detected.
    let launched = targets::LaunchFacts {
        target: TargetKind::Mailpit,
        executable_sha256: good_jar.clone(),
        argv_template: vec![],
        ready_ns: 0,
        members: vec![(member.to_string_lossy().into_owned(), good_member.clone())],
    };
    let spec = mk(&good_member, &good_jar);
    assert!(targets::spec_unchanged(&spec, &launched));
    std::fs::write(&member, b"swapped").unwrap();
    assert!(
        !targets::spec_unchanged(&spec, &launched),
        "a swapped member refuses"
    );
    std::fs::write(&member, b"authored member bytes").unwrap();
    std::fs::write(&jar, b"swapped").unwrap();
    assert!(
        !targets::spec_unchanged(&spec, &launched),
        "a swapped jar refuses"
    );
}

#[tokio::test]
async fn config_file_rendered_in_scratch_and_removed_with_env_slots_substituted() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = declared(
        TargetKind::Mailpit,
        &[Role::Smtp, Role::Http],
        &[
            "cp \"{config_path}\" \"$HOME/config.copy\"",
            "echo \"$MC_SMTP|$MC_HTTP|$MC_NAME\" > \"$HOME/env.copy\"",
            "# braces {{kept}} smtp={smtp_port} http={http_port}",
            MARK_STARTED,
        ],
    );
    spec.extra.env = vec![
        ["MC_SMTP".into(), "{smtp_addr}".into()],
        ["MC_HTTP".into(), "{http_addr}".into()],
        ["MC_NAME".into(), "plain".into()],
    ];
    let scratch = root.path().join("leaf");
    let keepers = targets::Keepers::default();
    let mut lim = short_limits();
    lim.startup = Duration::from_secs(60);
    let clock = wire::Clock(Instant::now());
    let launch = targets::Owned::launch(&spec, &scratch, &lim, &clock, &keepers);
    tokio::select! {
        r = launch => panic!("a never-ready server became ready: {:?}", r.err()),
        _ = started(&scratch) => {}
    }
    let copy = std::fs::read_to_string(scratch.join("home/config.copy")).unwrap();
    let env = std::fs::read_to_string(scratch.join("home/env.copy")).unwrap();
    // The config was rendered into the owned scratch before the child ran,
    // privately, with the braces unescaped and every port substituted.
    let line = copy.lines().find(|l| l.starts_with("# braces")).unwrap();
    assert!(line.starts_with("# braces {kept} smtp="), "{line}");
    let num = |key: &str| -> u16 {
        line.split(key)
            .nth(1)
            .unwrap()
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let (smtp_port, http_port) = (num("smtp="), num("http="));
    assert_ne!(smtp_port, http_port, "every role has its own reserved port");
    let parts: Vec<&str> = env.trim().split('|').collect();
    assert_eq!(
        parts,
        [
            format!("127.0.0.1:{smtp_port}"),
            format!("127.0.0.1:{http_port}"),
            "plain".into()
        ]
    );
    let mode = std::fs::metadata(scratch.join("config/run.sh"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    // A reserved port is released before the child could bind it, so a
    // parallel test may take it afterwards: `listeners_closed` is not this
    // test's claim, the reaping and the scratch are.
    let settled = keepers.settle(lim.cleanup * 2).await;
    let f = &settled[0];
    assert!(
        f.reaped && f.group_gone && f.scratch_removed && !f.forced_kill,
        "{f:?}"
    );
    assert!(!scratch.exists(), "the config file went with its scratch");
}

#[tokio::test]
async fn cleanup_reports_each_listener_and_conjunction() {
    let root = tempfile::tempdir().unwrap();
    let spec = declared(
        TargetKind::Mailpit,
        &[Role::Smtp, Role::Http],
        &[
            "echo {smtp_addr} > \"$HOME/smtp.tmp\"; mv \"$HOME/smtp.tmp\" \"$HOME/smtp\"",
            "echo {http_addr} > \"$HOME/http\"",
            MARK_STARTED,
        ],
    );
    let scratch = root.path().join("leaf");
    let clock = wire::Clock(Instant::now());
    let lim = short_limits();
    let keepers = targets::Keepers::default();
    let task = {
        let (spec, scratch, lim, keepers) =
            (spec.clone(), scratch.clone(), lim.clone(), keepers.clone());
        tokio::spawn(async move {
            targets::Owned::launch(&spec, &scratch, &lim, &clock, &keepers)
                .await
                .err()
        })
    };
    started(&scratch).await;
    let smtp: std::net::SocketAddr = std::fs::read_to_string(scratch.join("home/smtp"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // A foreign listener takes one declared address; the other stays free.
    let foreign = std::net::TcpListener::bind(smtp).expect("the reserved port was released");
    let Some(LaunchError::NotReady(_, facts)) = task.await.unwrap() else {
        panic!("not ready expected")
    };
    drop(foreign);
    let by_role = |r: Role| {
        facts
            .listeners
            .iter()
            .find(|l| l.role == r)
            .map(|l| l.closed)
    };
    assert_eq!(
        by_role(Role::Smtp),
        Some(false),
        "an address someone else holds is not closed"
    );
    assert!(
        by_role(Role::Http).is_some(),
        "every declared listener has its own proof"
    );
    assert!(
        !facts.listeners_closed,
        "the conjunction fails when any one stays open: {facts:?}"
    );
    assert!(facts.reaped && facts.group_gone && facts.scratch_removed && !facts.proved());
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the listening set is read from /proc (Linux)"
)]
async fn undeclared_listener_refused_and_cleaned() {
    let root = tempfile::tempdir().unwrap();
    let clock = wire::Clock(Instant::now());
    let keepers = targets::Keepers::default();
    // The candidate also listens for POP3, which this launch does not
    // declare: the group's listening set is not the declared one.
    let mut spec = swarmail_spec(targets::swarmail_template(0));
    spec.extra.listeners = vec![Role::Smtp, Role::Http];
    let scratch = root.path().join("leaf");
    let err = targets::Owned::launch(&spec, &scratch, &limits(), &clock, &keepers)
        .await
        .err();
    let Some(LaunchError::UndeclaredListener(facts)) = err else {
        panic!("{err:?}")
    };
    assert!(
        facts.proved(),
        "the refused launch was cleaned up: {facts:?}"
    );
    assert!(!scratch.exists());
    let settled = keepers.settle(limits().cleanup * 2).await;
    assert!(settled.iter().all(targets::CleanupFacts::proved));

    // Declaring all three makes the same launch exact, and cleanup then
    // proves each of the three closed.
    let mut spec = swarmail_spec(candidate_with_pop3_slot());
    spec.extra.listeners = vec![Role::Smtp, Role::Http, Role::Pop3];
    let mut owned = targets::Owned::launch(
        &spec,
        &root.path().join("leaf2"),
        &limits(),
        &clock,
        &keepers,
    )
    .await
    .expect("an exactly declared set launches");
    let facts = owned.shutdown().await;
    assert!(facts.proved() && facts.listeners_closed, "{facts:?}");
    let roles: Vec<Role> = facts.listeners.iter().map(|l| l.role).collect();
    assert_eq!(roles, [Role::Smtp, Role::Http, Role::Pop3]);
    assert!(facts.listeners.iter().all(|l| l.closed));
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the listening set is read from /proc (Linux)"
)]
async fn group_listeners_reads_the_kernel_for_the_owned_group_only() {
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
    // The candidate listens for SMTP, HTTP and POP3, and nothing else.
    let set = targets::group_listeners(owned.pid()).expect("readable on linux");
    assert!(
        set.contains(&owned.smtp) && set.contains(&owned.http_addr),
        "{set:?}"
    );
    assert_eq!(set.len(), 3, "{set:?}");
    assert!(set.iter().all(|a| a.ip().is_loopback()), "{set:?}");
    let pid = owned.pid();
    assert!(owned.shutdown().await.proved());
    assert_eq!(
        targets::group_listeners(pid),
        Some(Default::default()),
        "a gone group listens on nothing"
    );
}

// ---- the POP3 reader against a scripted responder ------------------------

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

enum Reply {
    Bytes(Vec<u8>),
    /// Say nothing, and keep the connection open until the peer leaves.
    Stall,
}

type Handler = Arc<dyn Fn(&str) -> Reply + Send + Sync>;

struct Responder {
    addr: std::net::SocketAddr,
    seen: Arc<Mutex<Vec<String>>>,
    connections: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
}

/// A local POP3 server that follows a script and records every command.
async fn responder(greeting: Option<&'static str>, handler: Handler) -> Responder {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let r = Responder {
        addr: listener.local_addr().unwrap(),
        seen: Arc::default(),
        connections: Arc::default(),
        closed: Arc::default(),
    };
    let (seen, connections, closed) = (r.seen.clone(), r.connections.clone(), r.closed.clone());
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            connections.fetch_add(1, Ordering::SeqCst);
            let (seen, closed, handler) = (seen.clone(), closed.clone(), handler.clone());
            tokio::spawn(async move {
                let (rd, mut wr) = stream.into_split();
                let mut rd = tokio::io::BufReader::new(rd);
                if let Some(g) = greeting {
                    let _ = wr.write_all(g.as_bytes()).await;
                }
                loop {
                    let mut line = String::new();
                    match rd.read_line(&mut line).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let cmd = line.trim_end().to_owned();
                    seen.lock().unwrap().push(cmd.clone());
                    if let Reply::Bytes(b) = handler(&cmd) {
                        let _ = wr.write_all(&b).await;
                    }
                }
                closed.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    r
}

/// RFC 1939 §3 byte-stuffing, written out here on its own.
fn stuffed(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in raw.split_inclusive(|b| *b == b'\n') {
        if line.first() == Some(&b'.') {
            out.push(b'.');
        }
        out.extend_from_slice(line);
    }
    out
}

fn multi(body: &[u8]) -> Reply {
    let mut v = b"+OK\r\n".to_vec();
    v.extend(stuffed(body));
    v.extend_from_slice(b".\r\n");
    Reply::Bytes(v)
}

/// A faithful server holding `msgs` as `(unique id, raw message)`.
fn standard(msgs: Vec<(&'static str, Vec<u8>)>) -> Handler {
    Arc::new(move |cmd: &str| {
        let n = |prefix: &str| {
            cmd.strip_prefix(prefix)
                .and_then(|r| r.split(' ').next()?.parse::<usize>().ok())
        };
        match cmd.split(' ').next().unwrap() {
            "USER" | "PASS" => Reply::Bytes(b"+OK\r\n".to_vec()),
            "STAT" => Reply::Bytes(format!("+OK {} 1000\r\n", msgs.len()).into_bytes()),
            "UIDL" => {
                let body: String = msgs
                    .iter()
                    .enumerate()
                    .map(|(i, (u, _))| format!("{} {u}\r\n", i + 1))
                    .collect();
                multi(body.as_bytes())
            }
            "TOP" => {
                let raw = &msgs[n("TOP ").unwrap() - 1].1;
                let head_end = raw
                    .windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .map_or(raw.len(), |i| i + 4);
                multi(&raw[..head_end])
            }
            "RETR" => multi(&msgs[n("RETR ").unwrap() - 1].1),
            "QUIT" => Reply::Bytes(b"+OK bye\r\n".to_vec()),
            _ => Reply::Bytes(b"-ERR unknown\r\n".to_vec()),
        }
    })
}

fn pop3(addr: std::net::SocketAddr, step: Duration, max: u64) -> wire::Pop3 {
    wire::Pop3::new(addr, step, max)
}

const GREETING: Option<&str> = Some("+OK authored pop3 ready\r\n");

fn authored_mail(subject: &str, body: &str) -> Vec<u8> {
    format!("From: <a@b.invalid>\r\nSubject: {subject}\r\n\r\n{body}").into_bytes()
}

#[tokio::test]
async fn pop3_dot_unstuffing() {
    // A body line `..lead` and a lone `.` line are stuffed on the wire as
    // `...lead` and `..`; neither is the terminator, and both come back as
    // authored.
    let raw = authored_mail("dots", "..lead\r\n.\r\nplain\r\n");
    let r = responder(GREETING, standard(vec![("u1", raw.clone())])).await;
    let got = pop3(r.addr, Duration::from_secs(5), 1 << 20)
        .retrieve("u1")
        .await
        .unwrap();
    assert_eq!(got, raw, "{}", String::from_utf8_lossy(&got));
    // The wire really did carry the stuffed forms.
    assert!(String::from_utf8_lossy(&stuffed(&raw)).contains("\r\n...lead\r\n..\r\n"));
    // A listing returns the header block, blank line included.
    let heads = pop3(r.addr, Duration::from_secs(5), 1 << 20)
        .list()
        .await
        .unwrap();
    assert_eq!(
        heads,
        vec![(
            "u1".to_owned(),
            b"From: <a@b.invalid>\r\nSubject: dots\r\n\r\n".to_vec()
        )]
    );
    assert_eq!(targets::subject_of_headers(&heads[0].1).unwrap(), "dots");
}

#[tokio::test]
async fn pop3_err_each_step() {
    let raw = authored_mail("one", "body\r\n");
    for step in ["USER", "PASS", "STAT", "UIDL", "TOP", "RETR"] {
        let inner = standard(vec![("u1", raw.clone())]);
        let handler: Handler = Arc::new(move |cmd: &str| {
            if cmd.starts_with(step) {
                Reply::Bytes(b"-ERR refused\r\n".to_vec())
            } else {
                inner(cmd)
            }
        });
        let r = responder(GREETING, handler).await;
        let client = pop3(r.addr, Duration::from_secs(5), 1 << 20);
        let result = if step == "RETR" {
            client.retrieve("u1").await.map(|_| ())
        } else {
            client.list().await.map(|_| ())
        };
        assert_eq!(result, Err(wire::WireError::Refused), "{step}");
    }
    // An error greeting is not a session at all.
    let r = responder(Some("-ERR busy\r\n"), standard(vec![])).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .list()
            .await
            .map(|_| ()),
        Err(wire::WireError::Malformed)
    );
    // A status line that is neither +OK nor -ERR is malformed.
    let handler: Handler = Arc::new(|_: &str| Reply::Bytes(b"OK?\r\n".to_vec()));
    let r = responder(GREETING, handler).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .list()
            .await
            .map(|_| ()),
        Err(wire::WireError::Malformed)
    );
}

#[tokio::test]
async fn pop3_long_line_and_oversized_message() {
    let long = "x".repeat(wire::MAX_POP3_LINE + 10);
    let handler: Handler = Arc::new(move |cmd: &str| {
        if cmd.starts_with("USER") {
            Reply::Bytes(format!("+OK {long}\r\n").into_bytes())
        } else {
            Reply::Bytes(b"+OK\r\n".to_vec())
        }
    });
    let r = responder(GREETING, handler).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .list()
            .await
            .map(|_| ()),
        Err(wire::WireError::Oversized)
    );
    // A body line over the cap, in a multi-line answer.
    let raw = authored_mail(
        "long",
        &format!("{}\r\n", "y".repeat(wire::MAX_POP3_LINE + 10)),
    );
    let r = responder(GREETING, standard(vec![("u1", raw.clone())])).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .retrieve("u1")
            .await,
        Err(wire::WireError::Oversized)
    );
    // A message over the byte cap, though every line is short.
    let raw = authored_mail(
        "big",
        &"z".repeat(300)
            .repeat(4)
            .chars()
            .collect::<Vec<_>>()
            .chunks(60)
            .map(|c| c.iter().collect::<String>() + "\r\n")
            .collect::<String>(),
    );
    let r = responder(GREETING, standard(vec![("u1", raw.clone())])).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 256)
            .retrieve("u1")
            .await,
        Err(wire::WireError::Oversized)
    );
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .retrieve("u1")
            .await
            .unwrap()
            .len(),
        raw.len()
    );
}

#[tokio::test]
async fn pop3_stall_hits_deadline() {
    // The deadline under test is a real one, kept short. (The paused clock
    // was tried first: tokio auto-advances it whenever the runtime waits on
    // a real loopback socket, so a successful greeting alone read as 28
    // virtual seconds and a timeout could fire early. A deadline is a
    // deadline; no test here sleeps to synchronize.)
    let step = Duration::from_millis(150);
    for stalled in ["STAT", "UIDL", "TOP", "USER"] {
        let inner = standard(vec![("u1", authored_mail("one", "b\r\n"))]);
        let handler: Handler = Arc::new(move |cmd: &str| {
            if cmd.starts_with(stalled) {
                Reply::Stall
            } else {
                inner(cmd)
            }
        });
        let r = responder(GREETING, handler).await;
        let started = Instant::now();
        let result = pop3(r.addr, step, 1 << 20).list().await.map(|_| ());
        assert_eq!(result, Err(wire::WireError::Timeout), "{stalled}");
        let took = started.elapsed();
        assert!(
            took >= step && took < Duration::from_secs(10),
            "{stalled}: one exchange's deadline: {took:?}"
        );
    }
    // No greeting at all is readiness that never arrives.
    let r = responder(None, standard(vec![])).await;
    assert_eq!(
        pop3(r.addr, step, 1 << 20).greeting().await,
        Err(wire::WireError::Timeout)
    );
}

#[tokio::test]
async fn pop3_cancel_mid_retr_leaves_no_task_behind() {
    // The server sends half a message and goes quiet; the caller gives up
    // the moment the server reports having sent it.
    let reached = Arc::new(tokio::sync::Notify::new());
    let signal = reached.clone();
    let handler: Handler = Arc::new(move |cmd: &str| match cmd.split(' ').next().unwrap() {
        "USER" | "PASS" => Reply::Bytes(b"+OK\r\n".to_vec()),
        "STAT" => Reply::Bytes(b"+OK 1 100\r\n".to_vec()),
        "UIDL" => multi(b"1 u1\r\n"),
        "RETR" => {
            signal.notify_one();
            Reply::Bytes(b"+OK\r\nSubject: half\r\n".to_vec())
        }
        _ => Reply::Stall,
    });
    let r = responder(GREETING, handler).await;
    let alive = || {
        tokio::runtime::Handle::current()
            .metrics()
            .num_alive_tasks()
    };
    let before = alive();
    let client = pop3(r.addr, Duration::from_secs(60), 1 << 20);
    tokio::select! {
        biased;
        _ = reached.notified() => {}
        got = client.retrieve("u1") => panic!("the read cannot finish: {got:?}"),
    }
    // The dropped future closed its socket: the server sees the peer leave
    // (a real deadline bounds the wait), and nothing of the client's runs.
    tokio::time::timeout(Duration::from_secs(30), async {
        while r.closed.load(Ordering::SeqCst) < 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the server saw the connection close");
    assert_eq!(r.connections.load(Ordering::SeqCst), 1);
    assert_eq!(alive(), before, "no task outlived the cancelled read");
}

#[tokio::test]
async fn pop3_never_sends_dele() {
    let mail = vec![
        ("u1", authored_mail("one", "1\r\n")),
        ("u2", authored_mail("two", "2\r\n")),
    ];
    let r = responder(GREETING, standard(mail)).await;
    let client = pop3(r.addr, Duration::from_secs(5), 1 << 20);
    assert_eq!(client.list().await.unwrap().len(), 2);
    client.retrieve("u2").await.unwrap();
    assert_eq!(
        client.list().await.unwrap().len(),
        2,
        "reading deleted nothing"
    );
    let seen = r.seen.lock().unwrap().clone();
    assert!(!seen.iter().any(|c| c.starts_with("DELE")), "{seen:?}");
    for c in &seen {
        let word = c.split(' ').next().unwrap();
        assert!(
            ["USER", "PASS", "STAT", "UIDL", "TOP", "RETR", "QUIT"].contains(&word),
            "{c}"
        );
    }
    // A fresh session per scan: three calls, three logins, three QUITs, and
    // the request counter counts every command.
    assert_eq!(r.connections.load(Ordering::SeqCst), 3);
    for word in ["USER", "PASS", "STAT", "UIDL", "QUIT"] {
        assert_eq!(
            seen.iter().filter(|c| c.starts_with(word)).count(),
            3,
            "{word}"
        );
    }
    assert_eq!(client.requests(), seen.len() as u64);
}

#[tokio::test]
async fn pop3_stat_uidl_mismatch_incomplete() {
    let two = |count: usize, listing: &'static str| -> Handler {
        Arc::new(move |cmd: &str| match cmd.split(' ').next().unwrap() {
            "STAT" => Reply::Bytes(format!("+OK {count} 10\r\n").into_bytes()),
            "UIDL" => multi(listing.as_bytes()),
            "QUIT" => Reply::Bytes(b"+OK\r\n".to_vec()),
            _ => Reply::Bytes(b"+OK\r\n".to_vec()),
        })
    };
    for (name, count, listing) in [
        ("STAT counts more", 3, "1 a\r\n2 b\r\n"),
        ("STAT counts fewer", 1, "1 a\r\n2 b\r\n"),
        ("a number skipped", 2, "1 a\r\n3 b\r\n"),
        ("a repeated unique id", 2, "1 a\r\n2 a\r\n"),
    ] {
        let r = responder(GREETING, two(count, listing)).await;
        let got = pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .list()
            .await
            .map(|_| ());
        assert_eq!(got, Err(wire::WireError::Incomplete), "{name}");
    }
    // A malformed UIDL line is malformed, not incomplete.
    let r = responder(GREETING, two(1, "just-one-word\r\n")).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .list()
            .await
            .map(|_| ()),
        Err(wire::WireError::Malformed)
    );
    // An id the server does not hold is not retrievable.
    let r = responder(
        GREETING,
        standard(vec![("u1", authored_mail("one", "b\r\n"))]),
    )
    .await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .retrieve("nope")
            .await,
        Err(wire::WireError::Malformed)
    );
}

#[tokio::test]
async fn pop3_readiness_needs_greeting() {
    let r = responder(GREETING, standard(vec![])).await;
    pop3(r.addr, Duration::from_secs(5), 1 << 20)
        .greeting()
        .await
        .expect("a +OK greeting is ready");
    let r = responder(Some("-ERR starting\r\n"), standard(vec![])).await;
    assert_eq!(
        pop3(r.addr, Duration::from_secs(5), 1 << 20)
            .greeting()
            .await,
        Err(wire::WireError::Malformed)
    );
    // Nothing listening is io, not readiness.
    let dead = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap()
    };
    assert_eq!(
        pop3(dead, Duration::from_secs(5), 1 << 20).greeting().await,
        Err(wire::WireError::Io)
    );
}

// ---- decoders ------------------------------------------------------------
//
// The MailSlurper, smtp4dev, Inbucket and Alkemio listing shapes below are
// authored here from the upstream documentation as recalled. No captured
// document or pinned executable stood behind them when this was written, so
// these tests prove the decoders' refusals and arithmetic, not that the
// shapes are the real tools': the `real_*` tests at the end are that proof.

const PAGED_HTTP: [TargetKind; 3] = [
    TargetKind::Mailslurper,
    TargetKind::AlkemioMailslurper,
    TargetKind::Smtp4dev,
];
const HTTP_NEW: [TargetKind; 4] = [
    TargetKind::Mailslurper,
    TargetKind::AlkemioMailslurper,
    TargetKind::Smtp4dev,
    TargetKind::Inbucket,
];

fn subject_of(token: &str) -> String {
    format!("mail-compare {token} alpha bravo charlie")
}

fn token_n(prefix: &str, n: usize) -> String {
    format!("mc-{prefix}-{n:06}")
}

/// One listing item as `kind`'s API states it.
fn item(kind: TargetKind, id: &str, token: &str) -> Value {
    let _ = kind;
    json!({"id": id, "subject": subject_of(token), "from": "sender@mail-compare.invalid"})
}

/// One authored page of `all` for page number `page` (1-based).
fn page_body(kind: TargetKind, all: &[(String, String)], page: usize, total: usize) -> Value {
    let size = targets::page_size(kind) as usize;
    let slice: Vec<Value> = all
        .iter()
        .skip((page - 1) * size)
        .take(size)
        .map(|(id, token)| item(kind, id, token))
        .collect();
    match kind {
        TargetKind::Mailslurper | TargetKind::AlkemioMailslurper => {
            let mut items = slice;
            for i in &mut items {
                i["fromAddress"] = i["from"].take();
            }
            json!({"mailItems": items, "totalPages": total.div_ceil(size), "totalRecords": total})
        }
        TargetKind::Smtp4dev => {
            json!({"results": slice, "currentPage": page, "pageCount": total.div_ceil(size),
                                        "rowCount": total, "pageSize": size})
        }
        TargetKind::Inbucket => json!(slice),
        _ => unreachable!("not an authored HTTP listing"),
    }
}

fn mailbox(n: usize, prefix: &str) -> Vec<(String, String)> {
    (0..n)
        .map(|i| (format!("id-{prefix}-{i}"), token_n(prefix, i)))
        .collect()
}

fn page_of(target: &str) -> usize {
    // MailSlurper asks for `pageNumber`, smtp4dev (its API XML names the
    // parameter) for `page`.
    let key = if target.contains("pageNumber=") {
        "pageNumber="
    } else {
        "page="
    };
    target
        .split(key)
        .nth(1)
        .and_then(|r| r.split('&').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1)
}

/// Limits for scans served by a local responder: a request deadline that a
/// loaded host cannot trip.
fn big_limits() -> targets::Limits {
    targets::Limits {
        request: Duration::from_secs(10),
        ..short_limits()
    }
}

#[test]
fn decode_page_refuses_incomplete_wrong_and_malformed_bodies() {
    let all = mailbox(3, "d");
    for kind in HTTP_NEW {
        let complete = page_body(kind, &all, 1, 3);
        let want: Vec<(String, String)> = all
            .iter()
            .map(|(id, t)| (id.clone(), subject_of(t)))
            .collect();
        assert_eq!(
            targets::decode_page(kind, &complete, 0),
            Some((3, want)),
            "{kind}"
        );
        // An empty page of an empty mailbox is complete and empty.
        assert_eq!(
            targets::decode_page(kind, &page_body(kind, &[], 1, 0), 0),
            Some((0, vec![])),
            "{kind}"
        );
        // A body missing the count or the list, an item without an id, an
        // empty id, a list that is not a list and a non-object are refused.
        let mut cases: Vec<Value> = vec![json!(null), json!("x"), json!(7)];
        for key in [
            "mailItems",
            "results",
            "totalRecords",
            "rowCount",
            "currentPage",
        ] {
            if complete.get(key).is_some() {
                let mut v = complete.clone();
                v.as_object_mut().unwrap().remove(key);
                cases.push(v);
                let mut v = complete.clone();
                v[key] = json!("not-a-list-or-count");
                cases.push(v);
            }
        }
        let mut v = complete.clone();
        for key in ["mailItems", "results"] {
            if v.get(key).is_some() {
                v[key][0].as_object_mut().unwrap().remove("id");
            }
        }
        if kind == TargetKind::Inbucket {
            v[0].as_object_mut().unwrap().remove("id");
        }
        cases.push(v);
        let mut v = complete.clone();
        for key in ["mailItems", "results"] {
            if v.get(key).is_some() {
                v[key][0]["id"] = "".into();
            }
        }
        if kind == TargetKind::Inbucket {
            v[0]["id"] = "".into();
        }
        cases.push(v);
        for (n, bad) in cases.iter().enumerate() {
            assert_eq!(
                targets::decode_page(kind, bad, 0),
                None,
                "{kind} case {n}: {bad}"
            );
        }
    }
    // Position: a page the target echoes must be the page asked for, a page
    // offset that is not a page boundary does not exist, and a per-mailbox
    // listing has only offset 0.
    let smtp4dev = page_body(TargetKind::Smtp4dev, &all, 1, 3);
    assert!(
        targets::decode_page(TargetKind::Smtp4dev, &smtp4dev, 250).is_none(),
        "page 1 answered for page 2"
    );
    assert!(targets::decode_page(TargetKind::Smtp4dev, &smtp4dev, 100).is_none());
    let ms = page_body(TargetKind::Mailslurper, &all, 1, 3);
    assert!(targets::decode_page(TargetKind::Mailslurper, &ms, 25).is_none());
    assert!(
        targets::decode_page(
            TargetKind::Inbucket,
            &page_body(TargetKind::Inbucket, &all, 1, 3),
            3
        )
        .is_none()
    );
    // GreenMail has no JSON listing, ever.
    assert!(targets::decode_page(TargetKind::Greenmail, &json!([]), 0).is_none());
    // The original three still decode exactly as before.
    let mailpit =
        json!({"messages_count": 1, "start": 0, "messages": [{"ID": "m1", "Subject": "s"}]});
    assert_eq!(
        targets::decode_page(TargetKind::Mailpit, &mailpit, 0),
        Some((1, vec![("m1".into(), "s".into())]))
    );
    assert!(targets::decode_page(TargetKind::Mailpit, &mailpit, 250).is_none());
}

#[tokio::test]
async fn scans_of_the_new_targets_are_complete_or_an_error() {
    for kind in PAGED_HTTP {
        let size = targets::page_size(kind) as usize;
        let n = size * 2 + 20;
        let all = mailbox(n, "s");
        let served = all.clone();
        let addr = super::responder(Box::new(move |t: &str| {
            Some((200, page_body(kind, &served, page_of(t), n).to_string()))
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        let got = owned
            .list_all(10)
            .await
            .unwrap_or_else(|e| panic!("{kind}: {e:?}"));
        let want: Vec<(String, String)> =
            all.iter().map(|(id, t)| (t.clone(), id.clone())).collect();
        assert_eq!(got, want, "{kind}: every message once, in order");
        assert_eq!(owned.requests(), 3, "{kind}: three pages, one request each");
        assert_eq!(
            owned.list_all(2).await,
            Err(wire::WireError::Oversized),
            "{kind}: more pages than allowed"
        );

        // Page two repeats an id from page one.
        let (a, dup) = (all.clone(), all[1].clone());
        let addr = super::responder(Box::new(move |t: &str| {
            let mut v = a.clone();
            if page_of(t) == 2 {
                v[size] = dup.clone();
            }
            Some((200, page_body(kind, &v, page_of(t), n).to_string()))
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        assert_eq!(
            owned.list_all(10).await,
            Err(wire::WireError::Incomplete),
            "{kind}: repeated id"
        );

        // The count moves between pages.
        let a = all.clone();
        let addr = super::responder(Box::new(move |t: &str| {
            let total = if page_of(t) == 1 { n } else { n + 1 };
            Some((200, page_body(kind, &a, page_of(t), total).to_string()))
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        assert_eq!(
            owned.list_all(10).await,
            Err(wire::WireError::Incomplete),
            "{kind}: moving total"
        );

        // A page that comes up short before the count is reached.
        let a = all.clone();
        let addr = super::responder(Box::new(move |t: &str| {
            let mut body = page_body(kind, &a, page_of(t), n);
            let key = if kind == TargetKind::Smtp4dev {
                "results"
            } else {
                "mailItems"
            };
            if page_of(t) == 1 {
                body[key].as_array_mut().unwrap().pop();
            }
            Some((200, body.to_string()))
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        assert_eq!(
            owned.list_all(10).await,
            Err(wire::WireError::Incomplete),
            "{kind}: short page"
        );
    }
}

#[tokio::test]
async fn http_bodies_that_are_truncated_oversized_or_malformed_are_errors() {
    for kind in HTTP_NEW {
        let all = mailbox(3, "t");
        let whole = page_body(kind, &all, 1, 3).to_string();
        let cut = whole[..whole.len() / 2].to_owned();
        let huge = whole.clone();
        let serve = |body: String, status: u16| async move {
            super::responder(Box::new(move |_: &str| Some((status, body.clone())))).await
        };
        let owned = |addr| targets::Owned::fixture(kind, addr, None, &big_limits());
        assert_eq!(
            owned(serve(cut, 200).await).list_all(5).await,
            Err(wire::WireError::Malformed),
            "{kind}: truncated"
        );
        assert_eq!(
            owned(serve("not json".into(), 200).await).list_all(5).await,
            Err(wire::WireError::Malformed)
        );
        assert_eq!(
            owned(serve(whole.clone(), 500).await).list_all(5).await,
            Err(wire::WireError::Malformed),
            "{kind}: status"
        );
        assert_eq!(
            owned(serve(whole.clone(), 200).await)
                .list_all(5)
                .await
                .unwrap()
                .len(),
            3,
            "{kind}: whole"
        );
        let mut small = big_limits();
        small.max_response_bytes = 64;
        let addr = serve(huge, 200).await;
        let tiny = targets::Owned::fixture(kind, addr, None, &small);
        assert_eq!(
            tiny.list_all(5).await,
            Err(wire::WireError::Oversized),
            "{kind}: oversized"
        );
    }
}

#[tokio::test]
async fn client_side_find_labelled_and_complete() {
    for kind in HTTP_NEW {
        assert!(
            !targets::has_server_search(kind),
            "{kind} has no verified search"
        );
        let mut all = mailbox(4, "f");
        all.insert(2, ("id-noise".into(), "not-a-token".into()));
        let served = all.clone();
        let n = all.len();
        let addr = super::responder(Box::new(move |t: &str| {
            Some((200, page_body(kind, &served, page_of(t), n).to_string()))
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        // The unrelated message is never a hit; the wanted one is exactly one.
        assert_eq!(
            owned.find(&token_n("f", 3)).await.unwrap(),
            vec![token_n("f", 3)],
            "{kind}"
        );
        assert!(
            owned.find("mc-absent-000000").await.unwrap().is_empty(),
            "{kind}"
        );
        // A scan that cannot be completed is an error, never a short answer.
        let bad = super::responder(Box::new(|_: &str| Some((200, "{".into())))).await;
        let owned = targets::Owned::fixture(kind, bad, None, &big_limits());
        assert_eq!(
            owned.find("mc-f-000000").await,
            Err(wire::WireError::Malformed),
            "{kind}"
        );
    }
    for t in TargetKind::V2 {
        assert!(targets::has_server_search(t), "{t}");
    }
}

#[tokio::test]
async fn greenmail_lists_finds_and_fetches_over_pop3() {
    let msgs: Vec<(&'static str, Vec<u8>)> = ["g0", "g1", "g2"]
        .iter()
        .map(|u| {
            (
                *u,
                authored_mail(&subject_of(&token_n(u, 0)), &format!("body {u}\r\n")),
            )
        })
        .collect();
    let r = responder(GREETING, standard(msgs.clone())).await;
    // GreenMail's HTTP address is never read: it has none.
    let owned = targets::Owned::fixture(
        TargetKind::Greenmail,
        "127.0.0.1:1".parse().unwrap(),
        Some(r.addr),
        &big_limits(),
    );
    let listed = owned.list_all(1).await.unwrap();
    assert_eq!(
        listed,
        vec![
            (token_n("g0", 0), "g0".into()),
            (token_n("g1", 0), "g1".into()),
            (token_n("g2", 0), "g2".into())
        ]
    );
    assert_eq!(
        owned.find(&token_n("g1", 0)).await.unwrap(),
        vec![token_n("g1", 0)]
    );
    let fetched = owned.fetch("g2").await.unwrap();
    assert_eq!(
        fetched.subject.as_deref(),
        Some(subject_of(&token_n("g2", 0)).as_str())
    );
    assert!(wire::same_body(Some("body g2"), fetched.text.as_deref()));
    assert!(
        owned.requests() > 0,
        "POP3 commands are counted as requests"
    );
    assert!(!targets::has_server_search(TargetKind::Greenmail));
}

fn corpus(n: u32) -> Vec<wire::Message> {
    let kinds = [
        protocol::MessageKind::Plain,
        protocol::MessageKind::Html,
        protocol::MessageKind::Multipart,
    ];
    (0..n)
        .map(|i| wire::authored(163, "unit0000", i, &kinds))
        .collect()
}

/// MailSlurper stores decoded fields, not raw bytes: authored here from the
/// message the corpus sent, split the way a mail store splits it.
fn slurper_json(m: &wire::Message) -> String {
    let raw = String::from_utf8(m.bytes.clone()).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let ct_line = head
        .lines()
        .find(|l| l.starts_with("Content-Type:"))
        .unwrap();
    let ct = ct_line["Content-Type:".len()..].trim();
    let (content_type, boundary) = match ct.split_once(';') {
        Some((t, rest)) => (
            t.trim(),
            rest.trim()
                .strip_prefix("boundary=")
                .map(|b| b.trim_matches('"'))
                .unwrap_or(""),
        ),
        None => (ct, ""),
    };
    json!({"id": "x", "subject": m.subject, "contentType": content_type, "boundary": boundary, "body": body}).to_string()
}

#[tokio::test]
async fn each_native_api_reads_back_subject_text_and_html_as_sent() {
    for m in corpus(6) {
        for kind in [
            TargetKind::Smtp4dev,
            TargetKind::Inbucket,
            TargetKind::Mailslurper,
            TargetKind::AlkemioMailslurper,
        ] {
            let (raw, slurper) = (m.bytes.clone(), slurper_json(&m));
            let addr = super::responder(Box::new(move |t: &str| match kind {
                TargetKind::Smtp4dev if t == "/api/messages/theid/raw" => {
                    Some((200, String::from_utf8(raw.clone()).unwrap()))
                }
                TargetKind::Inbucket if t == "/api/v1/mailbox/rcpt/theid/source" => {
                    Some((200, String::from_utf8(raw.clone()).unwrap()))
                }
                TargetKind::Mailslurper | TargetKind::AlkemioMailslurper if t == "/mail/theid" => {
                    Some((200, slurper.clone()))
                }
                _ => Some((404, "{}".into())),
            }))
            .await;
            let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
            let got = owned
                .fetch("theid")
                .await
                .unwrap_or_else(|e| panic!("{kind} {:?}: {e:?}", m.kind));
            assert_eq!(
                got.subject.as_deref(),
                Some(m.subject.as_str()),
                "{kind} {:?}",
                m.kind
            );
            assert!(
                wire::same_body(m.text.as_deref(), got.text.as_deref()),
                "{kind} {:?} text: {got:?}",
                m.kind
            );
            assert!(
                wire::same_body(m.html.as_deref(), got.html.as_deref()),
                "{kind} {:?} html: {got:?}",
                m.kind
            );
            // An id the server does not know is an error, not an empty message.
            assert_eq!(
                owned.fetch("other").await.map(|_| ()),
                Err(wire::WireError::Malformed),
                "{kind}"
            );
        }
    }
    // A MailSlurper answer with no body or subject is malformed.
    let addr = super::responder(Box::new(|_: &str| {
        Some((200, json!({"id": "x"}).to_string()))
    }))
    .await;
    let owned = targets::Owned::fixture(TargetKind::Mailslurper, addr, None, &big_limits());
    assert_eq!(
        owned.fetch("x").await.map(|_| ()),
        Err(wire::WireError::Malformed)
    );
}

#[tokio::test]
async fn hostile_bodies_round_trip() {
    // Dot-leading lines, an 8-bit (RFC 2047) subject, a 998-octet header and
    // a boundary string that appears inside a part: the reader that one
    // corpus uses for every competitor returns each as authored.
    let long_header = format!("X-Long: {}", "a".repeat(998 - "X-Long: ".len()));
    assert_eq!(long_header.len(), 998);
    let plain = format!(
        "From: <a@b.invalid>\r\nTo: <c@d.invalid>\r\nSubject: =?UTF-8?B?aMOpbGxv?=\r\n{long_header}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n.leading dot\r\n..two dots\r\n.\r\nlast line\r\n"
    );
    let multipart = "From: <a@b.invalid>\r\nSubject: plain ascii\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"b-mc-x\"\r\n\r\n--b-mc-x\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nsee the boundary below\r\n--other is not a boundary\r\n.dotted\r\n--b-mc-x\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>html part</p>\r\n--b-mc-x--\r\n";
    // The boundary string inside a part, not at a line start, is legal
    // content that the MIME parser reads as a delimiter and cuts the part at.
    let mid_line = multipart.replace("see the boundary below", "see --b-mc-x for details");
    for kind in [TargetKind::Smtp4dev, TargetKind::Inbucket] {
        let path = if kind == TargetKind::Smtp4dev {
            "/api/messages/p/raw"
        } else {
            "/api/v1/mailbox/rcpt/p/source"
        };
        let a = plain.clone();
        let addr = super::responder(Box::new(move |t: &str| {
            (t == path).then(|| (200, a.clone()))
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        let got = owned.fetch("p").await.unwrap();
        assert_eq!(got.subject.as_deref(), Some("héllo"), "{kind}");
        assert_eq!(
            got.text.as_deref(),
            Some(".leading dot\r\n..two dots\r\n.\r\nlast line\r\n"),
            "{kind}: {got:?}"
        );
    }
    // The multipart body, served at the raw path: a line that merely starts
    // like a boundary is content, and a mid-line boundary string is refused
    // rather than returned truncated.
    for kind in [TargetKind::Smtp4dev, TargetKind::Inbucket] {
        let path = if kind == TargetKind::Smtp4dev {
            "/api/messages/m/raw"
        } else {
            "/api/v1/mailbox/rcpt/m/source"
        };
        let (ok, hostile) = (multipart.to_owned(), mid_line.clone());
        let addr = super::responder(Box::new(move |t: &str| {
            (t == path).then(|| (200, ok.clone())).or_else(|| {
                (t.ends_with("/h/raw") || t.ends_with("/h/source")).then(|| (200, hostile.clone()))
            })
        }))
        .await;
        let owned = targets::Owned::fixture(kind, addr, None, &big_limits());
        let got = owned.fetch("m").await.unwrap();
        assert_eq!(got.subject.as_deref(), Some("plain ascii"));
        let text = got.text.unwrap();
        assert!(
            text.contains("see the boundary below")
                && text.contains("--other is not a boundary")
                && text.contains(".dotted"),
            "{text:?}"
        );
        assert!(
            !text.contains("<p>"),
            "the html part is not in the text part: {text:?}"
        );
        assert!(
            wire::same_body(Some("<p>html part</p>"), got.html.as_deref()),
            "{:?}",
            got.html
        );
        assert_eq!(
            owned.fetch("h").await.map(|_| ()),
            Err(wire::WireError::Malformed),
            "{kind}: a cut part is not a success"
        );
    }
}

// ---- the wrapper ---------------------------------------------------------
//
// Free: these run the real script with no paid input, and every case refuses
// in its preflight, before a build, a launch or an agent could start.

fn wrapper(args: &[&str], w: &Path) -> (i32, String) {
    let r = compare_mail(args, w);
    assert!(
        !w.join("launched").exists(),
        "no agent ever started: {args:?}"
    );
    r
}

#[test]
fn wrapper_requires_every_declared_competitor() {
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    let declared = [
        "mailpit",
        "mailhog",
        "mailslurper-source-build",
        "smtp4dev",
        "inbucket",
        "greenmail",
        "alkemio-mailslurper",
    ];
    for mode in ["local", "collect"] {
        // The first declared competitor with no binary is the one named.
        for have in 0..declared.len() {
            let mut args: Vec<String> = vec![mode.into(), "--output".into(), o.into()];
            for t in &declared[..have] {
                args.extend(["--bin".into(), format!("{t}=/bin/sh")]);
            }
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let (code, text) = wrapper(&refs, &w);
            assert_eq!(code, 2, "{mode} with {have}: {text}");
            assert!(
                text.contains(&format!("blocked at phase \"competitor-binaries\": the protocol declares {}; pass --bin {}=PATH", declared[have], declared[have])),
                "{mode} with {have}: {text}"
            );
        }
    }
    // A binary that is not executable is refused by name.
    let (code, text) = wrapper(
        &[
            "local",
            "--output",
            o,
            "--bin",
            "mailpit=/nonexistent/mailpit",
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(
        text.contains("--bin mailpit=/nonexistent/mailpit is not an executable file"),
        "{text}"
    );
}

#[test]
fn wrapper_reads_a_runtime_run_file_without_exec() {
    // A file a pinned runtime runs (GreenMail's jar) is read, never exec'd, so
    // it need not be an executable; without a runtime the same file is refused.
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    let jar = root.path().join("greenmail.jar");
    std::fs::write(&jar, b"not exec").unwrap();
    let jar = jar.to_str().unwrap().to_owned();
    let mut base: Vec<String> = vec!["local".into(), "--output".into(), o.into()];
    for t in [
        "mailpit",
        "mailhog",
        "mailslurper-source-build",
        "smtp4dev",
        "inbucket",
    ] {
        base.extend(["--bin".into(), format!("{t}=/bin/sh")]);
    }
    base.extend(["--bin".into(), format!("greenmail={jar}")]);
    let run = |extra: &[&str]| {
        let mut args = base.clone();
        args.extend(extra.iter().map(|a| (*a).to_owned()));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        wrapper(&refs, &w)
    };
    let (code, text) = run(&[]);
    assert_eq!(code, 2);
    assert!(
        text.contains(&format!("--bin greenmail={jar} is not an executable file")),
        "{text}"
    );
    let (code, text) = run(&["--runtime", "greenmail=/runtime/root"]);
    assert_eq!(code, 2);
    assert!(
        text.contains(
            "the protocol declares alkemio-mailslurper; pass --bin alkemio-mailslurper=PATH"
        ),
        "the jar passed, the next declared row is the one missing: {text}"
    );
    // A runtime needs a file, not a missing path.
    let (code, text) = run(&[
        "--runtime",
        "greenmail=/runtime/root",
        "--bin",
        "greenmail=/nonexistent/x.jar",
    ]);
    assert_eq!(code, 2);
    assert!(
        text.contains("--bin greenmail=/nonexistent/x.jar is not a file"),
        "{text}"
    );
}

#[test]
fn wrapper_refuses_unknown_bin() {
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    for (args, want) in [
        (
            vec!["--bin", "bogus=/bin/sh"],
            "--bin bogus names a target the protocol does not declare",
        ),
        // The candidate is built by the wrapper, never supplied.
        (
            vec!["--bin", "swarmail=/bin/sh"],
            "--bin swarmail names a target the protocol does not declare",
        ),
        (
            vec!["--runtime", "mailpit2=/tmp"],
            "--runtime mailpit2 names a target the protocol does not declare",
        ),
        (
            vec!["--bin", "mailpit"],
            "--bin takes NAME=VALUE, not mailpit",
        ),
        (vec!["--bin", "=/bin/sh"], "--bin takes NAME=VALUE"),
        (vec!["--bin"], "--bin needs a value"),
    ] {
        let mut full = vec!["local", "--output", o];
        full.extend(args.iter().copied());
        let (code, text) = wrapper(&full, &w);
        assert_eq!(code, 2, "{args:?}: {text}");
        assert!(text.contains(want), "{args:?}: expected {want:?} in {text}");
    }
    // The v2 protocol declares only the original two, so a new name is unknown there.
    let v2 = evals_dir().join("history/2026-10-30-protocol.v2.json");
    let (code, text) = wrapper(
        &[
            "local",
            "--output",
            o,
            "--protocol",
            v2.to_str().unwrap(),
            "--bin",
            "mailslurper=/bin/sh",
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(text.contains("--bin mailslurper names a target the protocol does not declare (it declares: mailpit mailhog)"), "{text}");
}

#[test]
fn wrapper_aliases_still_work() {
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    // The aliases count as --bin mailpit= and --bin mailhog=: with both, the
    // next declared competitor is the one missing.
    let (code, text) = wrapper(
        &[
            "local",
            "--output",
            o,
            "--mailpit",
            "/bin/sh",
            "--mailhog",
            "/bin/sh",
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(
        text.contains("the protocol declares mailslurper-source-build; pass --bin mailslurper-source-build=PATH"),
        "{text}"
    );
    let (code, text) = wrapper(
        &[
            "local",
            "--output",
            o,
            "--mailhog",
            "/bin/sh",
            "--bin",
            "mailpit=/bin/sh",
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(text.contains("the protocol declares mailslurper"), "{text}");
    let (code, text) = wrapper(&["local", "--output", o, "--mailpit", "/nonexistent"], &w);
    assert_eq!(code, 2);
    assert!(
        text.contains("--bin mailpit=/nonexistent is not an executable file"),
        "{text}"
    );
    // Against protocol 2 the two aliases are the whole requirement: with only
    // one, the other is named, exactly as before.
    let v2 = evals_dir().join("history/2026-10-30-protocol.v2.json");
    let v2 = v2.to_str().unwrap();
    let (code, text) = wrapper(
        &[
            "local",
            "--output",
            o,
            "--protocol",
            v2,
            "--mailpit",
            "/bin/sh",
        ],
        &w,
    );
    assert_eq!(code, 2);
    assert!(
        text.contains("the protocol declares mailhog; pass --bin mailhog=PATH"),
        "{text}"
    );
}

#[test]
fn wrapper_paid_modes_refuse_as_before() {
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    for (args, want) in [
        (
            vec!["controls", "--output", o],
            "blocked at phase \"prerequisite-baseline\": --baseline is not supplied",
        ),
        (
            vec!["evidence", "--output", o],
            "--final is required in evidence mode",
        ),
        (
            vec!["measure", "--output", o],
            "--config is required in measure mode",
        ),
        (
            vec!["measure", "--output", o, "--config", "/nonexistent/config"],
            "--config does not exist",
        ),
    ] {
        let (code, text) = wrapper(&args, &w);
        assert_eq!(code, 2, "{args:?}: {text}");
        assert!(text.contains(want), "{args:?}: expected {want:?} in {text}");
    }
}

/// GreenMail answers POP3 before its REST API listener binds (found by the
/// real launch, which refused with "http bind is absent"). A declared bind
/// that is absent is waited for until the startup deadline; a bind someone
/// holds is answered at once, and an absent one stays absent at the deadline.
#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn a_declared_bind_that_arrives_after_readiness_is_waited_for() {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    let loopback: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    // A socket bound but not yet listening, held by a child in its own
    // group: nothing listens there until the parent calls listen().
    let sock = tokio::net::TcpSocket::new_v4().unwrap();
    sock.bind(loopback).unwrap();
    let addr = sock.local_addr().unwrap();
    let fd = sock.as_raw_fd();
    let mut child = std::process::Command::new("sleep");
    child.arg("60").process_group(0);
    // SAFETY: dup2 and fcntl are async-signal-safe; the child gets the
    // socket as fd 3 without close-on-exec.
    unsafe {
        child.pre_exec(move || {
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
    let mut child = child.spawn().unwrap();
    let group = child.id() as i32;
    assert_eq!(targets::binds(group, addr), targets::Bind::Absent);
    // The server is slow to bind; the wait is bounded by a real deadline.
    let late = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        sock.listen(8).unwrap()
    });
    let seen = targets::bind_within(group, addr, Instant::now() + Duration::from_secs(20)).await;
    assert_eq!(
        seen,
        targets::Bind::Owned,
        "the late listener was waited for"
    );
    let _listener = late.await.unwrap();
    // A port nothing ever listens on is still absent when the deadline passes.
    let never = tokio::net::TcpSocket::new_v4().unwrap();
    never.bind(loopback).unwrap();
    let never_addr = never.local_addr().unwrap();
    let seen = targets::bind_within(
        group,
        never_addr,
        Instant::now() + Duration::from_millis(100),
    )
    .await;
    assert_eq!(seen, targets::Bind::Absent);
    // A foreign holder is reported at once, not waited out.
    let foreign = std::net::TcpListener::bind(loopback).unwrap();
    let t0 = Instant::now();
    let seen = targets::bind_within(
        group,
        foreign.local_addr().unwrap(),
        Instant::now() + Duration::from_secs(20),
    )
    .await;
    assert_eq!(seen, targets::Bind::Foreign);
    assert!(t0.elapsed() < Duration::from_secs(5));
    child.kill().unwrap();
    child.wait().unwrap();
}

// ---- ruling-scoped vocabulary: refusal tests (operator rulings 2026-11-07) -

/// The committed lock's pin of `t`, edited, as a lock (never verified).
fn lock_edited(t: &str, f: impl Fn(&mut Value)) -> SourceLock {
    let mut l = v4();
    let at = entry_of(&mut l, t);
    f(&mut l["entries"][at]["pin"]);
    lock4(l)
}

fn why_pin(t: &str, f: impl Fn(&mut Value)) -> String {
    refusals(lock_edited(t, f).validate())
}

/// Edit one document of a pin in place (by its path).
fn edit_doc(p: &mut Value, path: &str, f: impl Fn(&mut Value)) {
    let mut hit = false;
    for d in p["documents"].as_array_mut().unwrap() {
        if d["path"] == path {
            f(d);
            hit = true;
        }
    }
    assert!(hit, "{path}");
}

/// A copy of the committed documents with one document's bytes edited, and
/// the lock's size and digest for it kept in step (so only the content's own
/// meaning can refuse it).
fn tampered(
    t: &str,
    doc: &str,
    edit: impl Fn(Vec<u8>) -> Vec<u8>,
) -> (tempfile::TempDir, PathBuf, SourceLock) {
    let root = tempfile::tempdir().unwrap();
    let dir = evals_copy(root.path());
    let path = format!("sources/{t}/{doc}");
    let bytes = edit(std::fs::read(dir.join(&path)).unwrap());
    std::fs::write(dir.join(&path), &bytes).unwrap();
    let mut l = v4();
    let at = entry_of(&mut l, t);
    edit_doc(&mut l["entries"][at]["pin"], &path, |d| {
        d["size"] = bytes.len().into();
        d["sha256"] = sha(&bytes).into();
    });
    (root, dir, lock4(l))
}

fn replace_bytes(bytes: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains(from), "{from}");
    text.replace(from, to).into_bytes()
}

#[test]
fn mailslurper_source_build_is_neither_mailslurper_nor_the_shim() {
    let b = TargetKind::MailslurperSourceBuild;
    assert_eq!(b.name(), "mailslurper-source-build");
    assert_eq!(TargetKind::parse("mailslurper-source-build"), Some(b));
    for other in [
        TargetKind::Mailslurper,
        TargetKind::AlkemioMailslurper,
        TargetKind::Smtp4dev,
    ] {
        assert_ne!(b, other);
        assert_ne!(b.name(), other.name());
    }
    assert_ne!(b.name(), "mailslurper-shim");
    assert_ne!(
        b.provenance(),
        TargetKind::Mailslurper.provenance(),
        "a source build is not the official release"
    );
    assert!(
        matches!(b.provenance(), protocol::Provenance::SourceBuild(h) if h.ends_with("/mailslurper"))
    );
    assert_eq!(b.official_home(), TargetKind::Mailslurper.official_home());
    assert_ne!(format!("sources/{b}/"), "sources/mailslurper/");
}

#[test]
fn source_build_refuses_unresolved_build_provenance() {
    let why = why_pin("mailslurper-source-build", |p| {
        p["binary_build_provenance"] = json!({"unresolved": "no record"});
    });
    assert!(why.contains("needs observed build provenance"), "{why}");
}

#[test]
fn source_build_refuses_provenance_on_another_commit() {
    let why = why_pin("mailslurper-source-build", |p| {
        p["binary_build_provenance"]["observed"]["commit"] = "9".repeat(40).into();
    });
    assert!(
        why.contains("names a commit other than the tagged"),
        "{why}"
    );
}

#[test]
fn source_build_refuses_missing_build_evidence() {
    let why = why_pin("mailslurper-source-build", |p| {
        p["binary_build_provenance"]["observed"]["evidence"] =
            "sources/mailslurper-source-build/mailslurper-license.txt".into();
    });
    assert!(why.contains("cites no captured build evidence"), "{why}");
    // A build record that is not supplied evidence is refused too.
    let why = why_pin("mailslurper-source-build", |p| {
        for d in p["documents"].as_array_mut().unwrap() {
            if d["kind"] == "build_evidence" {
                d["attribution"] = json!({"observed": {"from": "api",
                    "url": "https://api.github.com/repos/mailslurper/mailslurper/x"}});
            }
        }
    });
    assert!(why.contains("must be attributed as supplied"), "{why}");
}

#[test]
fn source_build_refuses_other_download() {
    let why = why_pin("mailslurper-source-build", |p| {
        p["artifact"]["download"] = "https://github.com/mailslurper/mailslurper/releases/download/release-1.15.0/mailslurper".into();
    });
    assert!(
        why.contains("download is the tagged commit's tree"),
        "{why}"
    );
    let why = why_pin("mailslurper-source-build", |p| {
        p["artifact"]["digest_basis"] = "upstream_release_metadata".into();
        p["artifact"]["upstream_digest"] = p["artifact"]["sha256"].clone();
    });
    assert!(why.contains("a bare executable digested locally"), "{why}");
}

#[test]
fn reported_version_accepted_for_mailslurper_rows_at_release_1_15_0() {
    let lock = lock4(v4());
    lock.validate().unwrap();
    for t in [TargetKind::Mailslurper, TargetKind::MailslurperSourceBuild] {
        let pin = lock.pin(t).unwrap();
        assert_eq!(pin.release.as_ref().unwrap().tag, "release-1.15.0");
        assert!(pin.artifact.version_output.contains("1.14.1"), "{t}");
        assert!(!pin.artifact.version_output.contains("1.15.0"), "{t}");
    }
    // The report's version cell is derived from the closed constant.
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let labels = report::row_labels(&p);
    for t in [TargetKind::Mailslurper, TargetKind::MailslurperSourceBuild] {
        let l = labels.iter().find(|l| l.target == t).unwrap();
        assert_eq!(
            l.version.as_deref(),
            Some("1.14.1 reported / tag release-1.15.0")
        );
    }
    assert_eq!(labels.iter().filter(|l| l.version.is_some()).count(), 2);
}

#[test]
fn reported_version_refused_for_another_target() {
    for t in ["smtp4dev", "inbucket"] {
        let why = why_pin(t, |p| {
            p["artifact"]["version_output"] = "Starting MailSlurper Server v1.14.1".into();
        });
        assert!(
            why.contains("the observed version output does not name"),
            "{t}: {why}"
        );
    }
}

#[test]
fn reported_version_refused_for_another_tag() {
    let why = why_pin("mailslurper", |p| {
        p["release"]["tag"] = "release-1.15.1".into();
        p["release"]["html_url"] =
            "https://github.com/mailslurper/mailslurper/releases/tag/release-1.15.1".into();
    });
    assert!(
        why.contains("the observed version output does not name release-1.15.1"),
        "{why}"
    );
}

#[test]
fn version_output_naming_neither_version_is_refused() {
    for t in ["mailslurper", "mailslurper-source-build"] {
        let why = why_pin(t, |p| {
            p["artifact"]["version_output"] = "Starting MailSlurper Server v9.9.9".into();
        });
        assert!(
            why.contains("the observed version output does not name release-1.15.0"),
            "{t}: {why}"
        );
    }
}

#[test]
fn does_not_start_refuses_undeclared_target() {
    let why = v3_refusals(|v| v["does_not_start"] = json!([{"target": "swarmail", "cite": "x"}]));
    assert!(
        why.contains("does_not_start: swarmail is not a declared competitor"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        v["targets"]
            .as_array_mut()
            .unwrap()
            .retain(|t| t != "greenmail");
        v["does_not_start"] = json!([{"target": "greenmail", "cite": "x"}]);
    });
    assert!(
        why.contains("does_not_start: greenmail is not a declared competitor"),
        "{why}"
    );
    let why = v3_refusals(|v| {
        let d = v["does_not_start"][0].clone();
        v["does_not_start"].as_array_mut().unwrap().push(d);
    });
    assert!(
        why.contains("does_not_start: mailslurper is listed twice"),
        "{why}"
    );
    let why = v3_refusals(|v| v["does_not_start"][0]["cite"] = " ".into());
    assert!(
        why.contains("does_not_start: mailslurper cites no document"),
        "{why}"
    );
}

#[test]
fn does_not_start_refuses_target_without_pin() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let mut l = v4();
    let at = entry_of(&mut l, "mailslurper");
    l["entries"][at]["pin"] = Value::Null;
    let why = refusals(protocol::does_not_start_matches(&p, &lock4(l)));
    assert!(
        why.contains("does_not_start: mailslurper has no pin"),
        "{why}"
    );
}

#[test]
fn does_not_start_refuses_cite_outside_pin() {
    let lock = lock4(v4());
    for cite in [
        "sources/mailslurper/not-a-document.txt",
        "sources/mailslurper-source-build/mailslurper-license.txt",
    ] {
        let mut p = Protocol::parse(PROTOCOL_V3).unwrap();
        p.does_not_start.as_mut().unwrap()[0].cite = cite.into();
        let why = refusals(protocol::does_not_start_matches(&p, &lock));
        assert!(
            why.contains("which is no document of its pin"),
            "{cite}: {why}"
        );
        assert!(refusals(protocol::collection_ready(&p, &lock)).contains("does_not_start"));
    }
}

#[test]
fn does_not_start_cite_must_be_failure_evidence() {
    let lock = lock4(v4());
    // The committed pair: the cite is the supplied failure output.
    let committed = Protocol::parse(PROTOCOL_V3).unwrap();
    protocol::does_not_start_matches(&committed, &lock).expect("the committed cite is evidence");
    // A document of the pin that is not failure evidence (a Commit-attributed
    // README, an Api object, a Member) is refused.
    for cite in [
        "sources/mailslurper/mailslurper-readme.md",
        "sources/mailslurper/mailslurper-license.txt",
        "sources/mailslurper/mailslurper-release.json",
        "sources/mailslurper/mailslurper-config.json",
    ] {
        let mut p = committed.clone();
        p.does_not_start.as_mut().unwrap()[0].cite = cite.into();
        let why = refusals(protocol::does_not_start_matches(&p, &lock));
        assert!(
            why.contains("which is not failure evidence"),
            "{cite}: {why}"
        );
        assert!(
            refusals(protocol::collection_ready(&p, &lock)).contains("not failure evidence"),
            "{cite}"
        );
    }
    // The executable's own output is evidence too.
    let mut l = v4();
    let at = entry_of(&mut l, "mailslurper");
    for d in l["entries"][at]["pin"]["documents"].as_array_mut().unwrap() {
        if d["path"] == "sources/mailslurper/mailslurper-official-failure.txt" {
            d["attribution"] = json!({"observed": {"from": "output", "argv": ["--version"]}});
        }
    }
    protocol::does_not_start_matches(&committed, &lock4(l)).expect("an Output cite is evidence");
}

#[test]
fn does_not_start_refused_on_v2() {
    let mut v = json(PROTOCOL);
    v["does_not_start"] = json!([{"target": "mailpit", "cite": "x"}]);
    let why = refusals(protocol_from(v).unwrap().validate());
    assert!(why.contains("carries version 3 sections"), "{why}");
}

#[test]
fn plan_yields_no_coordinate_for_a_does_not_start_target() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let plan = runner::plan(&p);
    assert!(plan.iter().all(|c| c.target != TargetKind::Mailslurper));
    // The exact count, from the protocol's own profiles: every launched
    // target gets three coordinates (send, visible, verified) per throughput
    // profile and three (present, future, empty) per wait profile, for each
    // repetition that profile runs. Nine targets, one of them not starting.
    let per_target: usize = (0..6u32)
        .map(|rep| {
            3 * p.throughput.iter().filter(|t| t.repetitions > rep).count()
                + 3 * p.waits.iter().filter(|w| w.repetitions > rep).count()
        })
        .sum();
    assert!(per_target > 0);
    assert_eq!(
        p.throughput
            .iter()
            .map(|t| t.repetitions)
            .chain(p.waits.iter().map(|w| w.repetitions))
            .max(),
        Some(6),
        "the count above is over the protocol's six repetitions"
    );
    assert_eq!(p.targets.len(), 9);
    assert_eq!(plan.len(), (p.targets.len() - 1) * per_target);
    for t in protocol::declared_competitors(&p) {
        assert_eq!(plan.iter().filter(|c| c.target == t).count(), per_target);
    }
    assert_eq!(
        plan.iter()
            .filter(|c| c.target == TargetKind::Swarmail)
            .count(),
        per_target
    );
    // The row keeps its slot in every rotation: the denominator is the suite.
    assert!(
        p.statistics
            .orders
            .iter()
            .all(|o| o.contains(&TargetKind::Mailslurper))
    );
    // launch_order is the rotation minus the published row, in rotation order.
    for rep in 0..p.statistics.orders.len() as u32 {
        let want: Vec<_> = p
            .order(rep)
            .iter()
            .copied()
            .filter(|t| *t != TargetKind::Mailslurper)
            .collect();
        assert_eq!(p.launch_order(rep).collect::<Vec<_>>(), want);
        assert_eq!(want.len(), 8);
    }
}

#[test]
fn report_row_for_does_not_start_has_label_and_no_number() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let labels = report::row_labels(&p);
    let l = labels
        .iter()
        .find(|l| l.target == TargetKind::Mailslurper)
        .unwrap();
    assert_eq!(
        l.does_not_start.as_deref(),
        Some(
            "does not start (published result: sources/mailslurper/mailslurper-official-failure.txt)"
        )
    );
    assert!(
        labels
            .iter()
            .filter(|l| l.target != TargetKind::Mailslurper)
            .all(|l| l.does_not_start.is_none())
    );
    let blocked = report::blocked_rows(&p);
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].target, TargetKind::Mailslurper);
    assert_eq!(blocked[0].cells.len(), p.throughput.len() + p.waits.len());
    assert!(
        blocked[0]
            .cells
            .iter()
            .all(|c| c.outcome == "blocked" && c.requested > 0)
    );
    let text = serde_json::to_string(&blocked).unwrap();
    assert!(
        !text.contains("per_second") && !text.contains("latency"),
        "{text}"
    );
    // The built-from-source row is labelled as such, with its runtime.
    let sb = labels
        .iter()
        .find(|l| l.target == TargetKind::MailslurperSourceBuild)
        .unwrap();
    assert_eq!(sb.source, "built from source (cgo; host glibc, unpinned)");
    assert_eq!(sb.runtime, Some("host glibc (unpinned)"));
    let gm = labels
        .iter()
        .find(|l| l.target == TargetKind::Greenmail)
        .unwrap();
    assert!(gm.integrity.unwrap().contains("authenticity not verified"));
    assert_eq!(gm.runtime, Some("JRE (pinned)"));
    let ak = labels
        .iter()
        .find(|l| l.target == TargetKind::AlkemioMailslurper)
        .unwrap();
    assert!(ak.runtime.unwrap().starts_with("musl loader (pinned"));
}

#[test]
fn runtime_refused_on_a_third_target() {
    let jre = lock4(v4())
        .pin(TargetKind::Greenmail)
        .unwrap()
        .runtime
        .clone()
        .unwrap();
    for t in ["smtp4dev", "mailslurper", "inbucket"] {
        let why = why_pin(t, |p| {
            p["runtime"] = serde_json::to_value(&jre).unwrap();
            p["launch"]
                .as_array_mut()
                .unwrap()
                .push("{artifact}".into());
        });
        assert!(
            why.contains(
                "only greenmail (a JRE) and alkemio-mailslurper (a musl loader) pin a runtime"
            ),
            "{t}: {why}"
        );
    }
}

#[test]
fn loader_runtime_refuses_missing_member() {
    let why = why_pin("alkemio-mailslurper", |p| {
        p["runtime"]["checked_members"] = json!([])
    });
    assert!(
        why.contains("the runtime does not check lib/ld-musl-x86_64.so.1"),
        "{why}"
    );
    let why = why_pin("alkemio-mailslurper", |p| {
        p["runtime"]["checked_members"][0]["path"] = "lib/ld-musl-aarch64.so.1".into();
    });
    assert!(
        why.contains("the runtime does not check lib/ld-musl-x86_64.so.1"),
        "{why}"
    );
    let why = why_pin("alkemio-mailslurper", |p| {
        p.as_object_mut().unwrap().remove("runtime");
    });
    assert!(why.contains("needs a pinned runtime"), "{why}");
}

#[test]
fn loader_runtime_refuses_layer_absent_from_manifest() {
    let other = "1".repeat(64);
    let lock = lock_edited("alkemio-mailslurper", |p| {
        p["runtime"]["sha256"] = other.clone().into();
        p["runtime"]["download"] =
            format!("https://ghcr.io/v2/alkem-io/test-suites/mailslurper/blobs/sha256:{other}")
                .into();
    });
    lock.validate().expect("structurally fine");
    let why = refusals(lock.verify_files(&evals_dir()));
    assert!(
        why.contains("does not list the runtime's layer sha256:"),
        "{why}"
    );
    // The download must name the blob of the runtime's own digest.
    let why = why_pin("alkemio-mailslurper", |p| {
        p["runtime"]["download"] =
            "https://ghcr.io/v2/alkem-io/test-suites/mailslurper/blobs/sha256:".into();
    });
    assert!(
        why.contains("is not the image blob of its own sha256"),
        "{why}"
    );
    // The committed loader layer is listed by the committed manifest.
    lock4(v4()).verify_files(&evals_dir()).unwrap();
}

#[test]
fn runtime_license_off_the_list_is_refused() {
    for t in ["alkemio-mailslurper", "greenmail"] {
        for bad in [
            "GPL-3.0-only",
            "GPL-2.0-only",
            "Apache-2.0",
            "LGPL-2.1-or-later",
            "",
        ] {
            let why = why_pin(t, |p| p["runtime"]["license"]["spdx"] = bad.into());
            assert!(
                why.contains("is not on the runtime list"),
                "{t} {bad:?}: {why}"
            );
        }
        for ok in ["MIT", "GPL-2.0-only WITH Classpath-exception-2.0"] {
            let r = lock_edited(t, |p| p["runtime"]["license"]["spdx"] = ok.into()).validate();
            let why = r.err().map(|e| e.0.join("\n")).unwrap_or_default();
            assert!(!why.contains("runtime list"), "{t} {ok}: {why}");
        }
    }
}

#[test]
fn permissive_list_is_exactly_the_five() {
    assert_eq!(
        protocol::PERMISSIVE,
        ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"]
    );
    // MIT on the runtime list widens nothing for a measured product: a
    // product license outside the five is still refused.
    for bad in [
        "GPL-2.0-only WITH Classpath-exception-2.0",
        "MPL-2.0",
        "Zlib",
    ] {
        let why = why_pin("inbucket", |p| p["license"]["spdx"] = bad.into());
        assert!(
            why.contains("is not on the permissive list"),
            "{bad}: {why}"
        );
    }
    // And deny.toml's own allowlist is untouched by the runtime ruling.
    let deny =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml")).unwrap();
    assert!(!deny.contains("Classpath"));
}

#[test]
fn loader_launch_execs_the_pinned_loader_with_the_binary_as_artifact() {
    let lock = lock4(v4());
    let pin = lock.pin(TargetKind::AlkemioMailslurper).unwrap();
    let root = PathBuf::from("/runtime/root");
    let exe = PathBuf::from("/bin/extracted/mailslurper");
    let spec = LaunchSpec::from_pin(
        TargetKind::AlkemioMailslurper,
        exe.clone(),
        Some(root.clone()),
        pin,
    )
    .unwrap();
    assert_eq!(spec.binary, root.join("lib/ld-musl-x86_64.so.1"));
    assert_eq!(spec.extra.artifact, Some(exe));
    assert_eq!(
        spec.template.first().map(String::as_str),
        Some("{artifact}")
    );
    assert_eq!(
        spec.expected_sha256.as_deref(),
        Some(pin.artifact.executable_sha256.as_str())
    );
    let rt = pin.runtime.as_ref().unwrap();
    assert_eq!(
        spec.extra.members,
        vec![(
            root.join("lib/ld-musl-x86_64.so.1"),
            rt.checked_members[0].sha256.clone()
        )]
    );
    // No host loader and no library path anywhere in the launch.
    assert!(spec.extra.env.iter().all(|[n, _]| n != "LD_LIBRARY_PATH"));
    assert!(!spec.binary.starts_with("/lib"));
    // The config is rendered where the program, which has no flag for it, reads it.
    assert!(spec.extra.config.as_ref().unwrap().cwd);
    // GreenMail still execs bin/java; a pin with no runtime takes none.
    let g = lock.pin(TargetKind::Greenmail).unwrap();
    let gs = LaunchSpec::from_pin(
        TargetKind::Greenmail,
        PathBuf::from("/j.jar"),
        Some(root.clone()),
        g,
    )
    .unwrap();
    assert_eq!(gs.binary, root.join("bin/java"));
    let s4 = lock.pin(TargetKind::Smtp4dev).unwrap();
    let err = LaunchSpec::from_pin(TargetKind::Smtp4dev, PathBuf::from("/x"), Some(root), s4);
    assert!(err.unwrap_err().contains("its pin has no runtime"));
}

#[test]
fn registry_download_outside_home_is_refused() {
    let why = why_pin("greenmail", |p| {
        p["artifact"]["download"] =
            "https://example.invalid/greenmail-standalone-2.1.15.jar".into();
    });
    assert!(
        why.contains("is not under the registry https://repo1.maven.org/maven2/com/icegreen/greenmail-standalone/"),
        "{why}"
    );
    let why = why_pin("greenmail", |p| {
        p["artifact"]["download"] = "https://repo1.maven.org/maven2/com/icegreen/greenmail-standalone/2.1.14/greenmail-standalone-2.1.15.jar".into();
    });
    assert!(why.contains("is not under the registry"), "{why}");
}

#[test]
fn registry_basis_on_another_target_is_refused() {
    let why = why_pin("smtp4dev", |p| {
        p["artifact"]["digest_basis"] = "registry_checksum".into()
    });
    assert!(why.contains("no registry is recorded for it"), "{why}");
}

#[test]
fn registry_checksum_document_disagreeing_is_refused() {
    let (_r, dir, lock) = tampered("greenmail", "greenmail-jar.sha256", |_| {
        "2".repeat(64).into_bytes()
    });
    let why = refusals(lock.verify_files(&dir));
    assert!(
        why.contains("disagrees with the pin's upstream digest"),
        "{why}"
    );
    // The honest bytes agree.
    lock4(v4()).verify_files(&evals_dir()).unwrap();
}

#[test]
fn registry_pin_without_checksum_document_is_refused() {
    let why = why_pin("greenmail", |p| {
        p["documents"]
            .as_array_mut()
            .unwrap()
            .retain(|d| d["kind"] != "checksum");
    });
    assert!(
        why.contains("needs its captured Checksum document"),
        "{why}"
    );
}

#[test]
fn registry_checksum_record_must_name_the_sha256_url() {
    let why = why_pin("greenmail", |p| {
        edit_doc(p, "sources/greenmail/greenmail-jar.sha256", |d| {
            let sha = d["sha256"].as_str().unwrap().to_owned();
            d["attribution"]["observed"]["record"] =
                format!(".compare-bins/2026-11-07-supply/greenmail-maven/x sha256 {sha}").into();
        });
    });
    assert!(
        why.contains("the checksum record must name https://repo1.maven.org/"),
        "{why}"
    );
}

#[test]
fn upstream_release_pin_with_unlisted_asset_is_still_refused() {
    let lock = lock_edited("inbucket", |p| {
        p["artifact"]["file_name"] = "inbucket_9.9.9_linux_amd64.tar.gz".into();
        p["artifact"]["download"] = "https://github.com/inbucket/inbucket/releases/download/v3.2.0/inbucket_9.9.9_linux_amd64.tar.gz".into();
    });
    let why = refusals(lock.verify_files(&evals_dir()));
    assert!(
        why.contains("release metadata lists no asset inbucket_9.9.9_linux_amd64.tar.gz"),
        "{why}"
    );
    // The registry basis skips only the asset list: tag, page, commitish and
    // date still apply to GreenMail's release metadata.
    let lock = lock_edited("greenmail", |p| {
        p["release"]["target_commitish"] = "main".into()
    });
    let why = refusals(lock.verify_files(&evals_dir()));
    assert!(
        why.contains("release metadata target_commitish disagrees with the pin"),
        "{why}"
    );
}

#[test]
fn skipped_asset_list_still_checks_tag_commitish_and_date() {
    // Two pins skip the release asset list: GreenMail (registry basis) and
    // the source build. Each must still refuse an edited release field.
    for (t, field, edited, named) in [
        ("greenmail", "tag", "release-9.9.9", "tag_name"),
        ("greenmail", "target_commitish", "main", "target_commitish"),
        (
            "greenmail",
            "published_at",
            "2000-01-01T00:00:00Z",
            "published_at",
        ),
        (
            "mailslurper-source-build",
            "tag",
            "release-9.9.9",
            "tag_name",
        ),
        (
            "mailslurper-source-build",
            "target_commitish",
            "main",
            "target_commitish",
        ),
        (
            "mailslurper-source-build",
            "published_at",
            "2000-01-01T00:00:00Z",
            "published_at",
        ),
    ] {
        let lock = lock_edited(t, |p| p["release"][field] = edited.into());
        let why = refusals(lock.verify_files(&evals_dir()));
        assert!(
            why.contains(&format!(
                "{t}: release metadata {named} disagrees with the pin"
            )),
            "{t} {field}: {why}"
        );
    }
    // The untouched committed lock still verifies, asset list skipped.
    lock4(v4()).verify_files(&evals_dir()).unwrap();
}

#[test]
fn tree_rebuild_matches_authored_git_mktree_fixture() {
    // Authored with `git mktree` (a blob, an executable, a symlink and a
    // subtree), committed as bytes; the id is git's own answer.
    let fixture = json(include_bytes!("mktree-fixture.json"));
    assert_eq!(fixture["sha"], "7eecf41bf68c97fedabeb2d6727c78d5686810fd");
    let entries = fixture["tree"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(
        protocol::tree_object_id(entries).as_deref(),
        fixture["sha"].as_str()
    );
    // An edited sha, path, mode or order is a different object.
    for edit in [
        (|e: &mut Vec<Value>| e[0]["sha"] = "0".repeat(40).into()) as fn(&mut Vec<Value>),
        |e| e[1]["path"] = "bb".into(),
        |e| e[1]["mode"] = "100644".into(),
        |e| e.swap(0, 1),
        |e| e[3]["mode"] = "100644".into(),
    ] {
        let mut e = entries.clone();
        edit(&mut e);
        assert_ne!(
            protocol::tree_object_id(&e).as_deref(),
            fixture["sha"].as_str()
        );
    }
    // An entry that is not one has no id.
    let mut bad = entries.clone();
    bad[0]["mode"] = "100664".into();
    assert_eq!(protocol::tree_object_id(&bad), None);
    let mut bad = entries.clone();
    bad[0]["sha"] = "xyz".into();
    assert_eq!(protocol::tree_object_id(&bad), None);
}

#[test]
fn edited_tree_entry_fails_the_rebuild() {
    let (_r, dir, lock) = tampered("mailslurper", "mailslurper-tree.json", |b| {
        replace_bytes(
            b,
            "678bb2b4e7ee4195a0acfad0295d19c4ac1dc96f",
            "778bb2b4e7ee4195a0acfad0295d19c4ac1dc96f",
        )
    });
    let why = refusals(lock.verify_files(&dir));
    assert!(why.contains("does not rebuild to its own id"), "{why}");
    let (_r, dir, lock) = tampered("smtp4dev", "smtp4dev-tree.json", |b| {
        replace_bytes(b, "\"LICENSE.md\"", "\"LICENSE.mx\"")
    });
    let why = refusals(lock.verify_files(&dir));
    assert!(why.contains("does not rebuild to its own id"), "{why}");
}

#[test]
fn git_commit_naming_another_tree_is_refused() {
    let (_r, dir, lock) = tampered("mailslurper", "mailslurper-commit.json", |b| {
        replace_bytes(
            b,
            "5f29f08a747241992f5e8ffc25ca8fb35d550729",
            "6f29f08a747241992f5e8ffc25ca8fb35d550729",
        )
    });
    let why = refusals(lock.verify_files(&dir));
    assert!(
        why.contains("names a tree other than the captured one"),
        "{why}"
    );
}

#[test]
fn git_commit_for_another_commit_is_refused() {
    let (_r, dir, lock) = tampered("inbucket", "inbucket-commit.json", |b| {
        // Only the object's own id changes (its url keeps the real one).
        String::from_utf8(b)
            .unwrap()
            .replacen(
                "\"sha\":\"742249691d2f8f40253411394a8bbb8e50561849\"",
                "\"sha\":\"842249691d2f8f40253411394a8bbb8e50561849\"",
                1,
            )
            .into_bytes()
    });
    let why = refusals(lock.verify_files(&dir));
    assert!(
        why.contains("the captured commit object is not the tagged commit"),
        "{why}"
    );
}

#[test]
fn commit_attributed_blob_absent_from_tree_is_refused() {
    let (_r, dir, lock) = tampered("mailslurper", "mailslurper-tree.json", |b| {
        replace_bytes(b, "\"path\":\"LICENSE\"", "\"path\":\"LICENSX\"")
    });
    let why = refusals(lock.verify_files(&dir));
    assert!(
        why.contains("mailslurper-license.txt is not LICENSE in the tagged tree"),
        "{why}"
    );
}

#[test]
fn member_refuses_escaping_path() {
    for bad in ["../etc/passwd", "/abs/path", "a/../../b", "a//b", ""] {
        let why = why_pin("smtp4dev", |p| {
            edit_doc(p, "sources/smtp4dev/smtp4dev-api.xml", |d| {
                d["attribution"]["observed"]["member"] = bad.into();
            });
        });
        assert!(
            why.contains("needs an archive digest and a confined member path"),
            "{bad:?}: {why}"
        );
    }
}

#[test]
fn member_refuses_bad_digest() {
    for bad in ["xyz", "", &"A".repeat(64), &"1".repeat(63)] {
        let why = why_pin("smtp4dev", |p| {
            edit_doc(p, "sources/smtp4dev/smtp4dev-api.xml", |d| {
                d["attribution"]["observed"]["archive_sha256"] = bad.into();
            });
        });
        assert!(
            why.contains("needs an archive digest and a confined member path"),
            "{bad:?}: {why}"
        );
    }
}

#[test]
fn member_refuses_archive_outside_the_allowed_set() {
    let why = why_pin("smtp4dev", |p| {
        edit_doc(p, "sources/smtp4dev/smtp4dev-api.xml", |d| {
            d["attribution"]["observed"]["archive_sha256"] = "7".repeat(64).into();
        });
    });
    assert!(
        why.contains("neither the pin's artifact, its runtime nor a source archive"),
        "{why}"
    );
    // The MailSlurper source archive belongs to the two MailSlurper rows only.
    let why = why_pin("smtp4dev", |p| {
        edit_doc(p, "sources/smtp4dev/smtp4dev-api.xml", |d| {
            d["attribution"]["observed"]["archive_sha256"] = protocol::SOURCE_ARCHIVES[0].1.into();
        });
    });
    assert!(why.contains("neither the pin's artifact"), "{why}");
    // And only while the pin's tagged commit is the archive's own commit.
    let why = why_pin("mailslurper", |p| {
        p["tagged_source"]["observed"]["commit"] = "9".repeat(40).into();
    });
    assert!(why.contains("neither the pin's artifact"), "{why}");
}

#[test]
fn output_refuses_empty_argv() {
    for bad in [
        json!([]),
        json!([""]),
        json!(["a b"]),
        json!(["1", "2", "3", "4", "5", "6", "7", "8", "9"]),
    ] {
        let why = why_pin("inbucket", |p| {
            edit_doc(p, "sources/inbucket/inbucket-help.txt", |d| {
                d["attribution"]["observed"]["argv"] = bad.clone();
            });
        });
        assert!(
            why.contains("output attribution needs a short, non-empty, printable argv"),
            "{bad}: {why}"
        );
    }
}

#[test]
fn supplied_refuses_empty_record() {
    for bad in ["", "   "] {
        let why = why_pin("mailslurper", |p| {
            edit_doc(
                p,
                "sources/mailslurper/mailslurper-official-failure.txt",
                |d| {
                    d["attribution"]["observed"]["record"] = bad.into();
                },
            );
        });
        assert!(
            why.contains("supplied attribution must name the supply path"),
            "{bad:?}: {why}"
        );
    }
}

#[test]
fn supplied_refuses_record_without_document_digest() {
    let why = why_pin("mailslurper", |p| {
        edit_doc(
            p,
            "sources/mailslurper/mailslurper-official-failure.txt",
            |d| {
                d["attribution"]["observed"]["record"] = ".compare-bins/2026-11-07-supply/x".into();
            },
        );
    });
    assert!(
        why.contains("supplied attribution must name the supply path"),
        "{why}"
    );
    // Nor a record that does not name the supply at all.
    let why = why_pin("mailslurper", |p| {
        edit_doc(
            p,
            "sources/mailslurper/mailslurper-official-failure.txt",
            |d| {
                let sha = d["sha256"].as_str().unwrap().to_owned();
                d["attribution"]["observed"]["record"] = format!("somewhere sha256 {sha}").into();
            },
        );
    });
    assert!(
        why.contains("supplied attribution must name the supply path"),
        "{why}"
    );
}

#[test]
fn official_pin_still_needs_an_observed_tagged_source_but_an_image_pin_does_not() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let lock = lock_edited("inbucket", |pin| {
        pin["tagged_source"] = json!({"unresolved": "authored gap"});
    });
    let why = refusals(protocol::collection_ready(&p, &lock));
    assert!(
        why.contains("inbucket: tagged source unresolved: authored gap"),
        "{why}"
    );
    // The image pin's tagged-source gap is structural (validate_image refuses
    // an observed one), so readiness does not count it.
    let lock = lock4(v4());
    assert!(
        lock.pin(TargetKind::AlkemioMailslurper)
            .unwrap()
            .tagged_source
            .gap()
            .is_some()
    );
    protocol::collection_ready(&p, &lock).expect("an image pin is ready without a tag");
}

#[test]
fn has_tagged_source_is_false_only_for_the_image_provenance() {
    let with: Vec<_> = TargetKind::ALL
        .into_iter()
        .filter(|t| t.provenance().has_tagged_source())
        .collect();
    let without: Vec<_> = TargetKind::ALL
        .into_iter()
        .filter(|t| !t.provenance().has_tagged_source())
        .collect();
    assert_eq!(TargetKind::ALL.len(), 9);
    assert_eq!(without, [TargetKind::AlkemioMailslurper]);
    assert_eq!(with.len(), 8);
    // Only the image provenance is exempt; a lock that gives another target
    // an `image` field does not inherit the exemption.
    assert!(matches!(
        TargetKind::AlkemioMailslurper.provenance(),
        protocol::Provenance::ThirdPartyImage(_)
    ));
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let mut l = v4();
    let from = entry_of(&mut l, "alkemio-mailslurper");
    let image = l["entries"][from]["pin"]["image"].clone();
    assert!(image.is_object(), "the image pin carries its `image` field");
    let at = entry_of(&mut l, "smtp4dev");
    l["entries"][at]["pin"]["tagged_source"] = json!({"unresolved": "authored gap"});
    l["entries"][at]["pin"]["image"] = image;
    let why = refusals(protocol::collection_ready(&p, &lock4(l)));
    assert!(
        why.contains("smtp4dev: tagged source unresolved: authored gap"),
        "{why}"
    );
}

#[tokio::test]
async fn config_file_cwd_runs_where_rendered() {
    for (cwd, want) in [(false, ""), (true, "config")] {
        let root = tempfile::tempdir().unwrap();
        let mut spec = declared(
            TargetKind::Mailpit,
            &[Role::Smtp, Role::Http],
            &[
                "# {smtp_addr} {http_addr}",
                "pwd > \"$HOME/cwd.copy\"",
                MARK_STARTED,
            ],
        );
        spec.extra.config.as_mut().unwrap().cwd = cwd;
        let scratch = root.path().join("leaf");
        let keepers = targets::Keepers::default();
        let mut lim = short_limits();
        lim.startup = Duration::from_secs(60);
        let clock = wire::Clock(Instant::now());
        let launch = targets::Owned::launch(&spec, &scratch, &lim, &clock, &keepers);
        tokio::select! {
            r = launch => panic!("a never-ready server became ready: {:?}", r.err()),
            _ = started(&scratch) => {}
        }
        let got = std::fs::read_to_string(scratch.join("home/cwd.copy")).unwrap();
        let mut expect = scratch.canonicalize().unwrap();
        if !want.is_empty() {
            expect.push(want);
        }
        assert_eq!(got.trim(), expect.to_string_lossy(), "cwd={cwd}");
        let settled = keepers.settle(lim.cleanup * 2).await;
        assert!(settled[0].reaped && settled[0].group_gone && settled[0].scratch_removed);
    }
}

#[test]
fn frozen_v3_and_lock_v4_bytes_are_the_recorded_ones() {
    // The freeze is identified by content hash (decision 0130's addendum
    // records the same two values). After it neither file's bytes change:
    // a change is a new protocol version, never an edit.
    assert_eq!(sha(PROTOCOL_V3), FROZEN_PROTOCOL_V3_SHA);
    assert_eq!(sha(SOURCES_V4), FROZEN_LOCK_V4_SHA);
    // The history bytes keep their digests.
    assert_eq!(sha(PROTOCOL), V2_PROTOCOL_SHA);
    assert_eq!(sha(SOURCES), V3_LOCK_SHA);
}

// ---- real executables (ignored; they fail without their bytes) -----------

fn env_name(t: TargetKind) -> String {
    format!("MAIL_COMPARE_{}", t.name().to_uppercase().replace('-', "_"))
}

/// The root directory of the runtime a pin needs, from the environment:
/// `MAIL_COMPARE_JRE` for GreenMail's JRE, `MAIL_COMPARE_MUSL` for the root
/// holding Alkemio's `lib/ld-musl-x86_64.so.1`.
fn runtime_env(t: TargetKind) -> &'static str {
    match t {
        TargetKind::AlkemioMailslurper => "MAIL_COMPARE_MUSL",
        _ => "MAIL_COMPARE_JRE",
    }
}

/// The launch of the pinned `t`, from the committed lock 4 and the bytes the
/// environment names. A target without a pin fails naming why.
fn real_spec(t: TargetKind) -> LaunchSpec {
    let var = env_name(t);
    let binary = std::env::var_os(&var)
        .unwrap_or_else(|| panic!("{var} must name the pinned {t} executable"));
    let lock = lock4(v4());
    let pin = lock
        .pin(t)
        .unwrap_or_else(|| panic!("{t} has no verified pin in the committed lock"));
    let runtime = pin.runtime.as_ref().map(|_| {
        let var = runtime_env(t);
        std::path::PathBuf::from(
            std::env::var_os(var)
                .unwrap_or_else(|| panic!("{var} must name the pinned runtime's root directory")),
        )
    });
    LaunchSpec::from_pin(t, binary.into(), runtime, pin).expect("a launch from the pin")
}

async fn real_launch(t: TargetKind, root: &Path) -> targets::Owned {
    let spec = real_spec(t);
    let clock = wire::Clock(Instant::now());
    targets::Owned::launch(
        &spec,
        &root.join("leaf"),
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await
    .unwrap_or_else(|e| panic!("{t} did not launch: {e:?}"))
}

async fn send_all(owned: &targets::Owned, msgs: &[wire::Message]) {
    let clock = wire::Clock(Instant::now());
    let mut s = wire::Smtp::open(owned.smtp, Duration::from_secs(10))
        .await
        .expect("an SMTP session");
    for m in msgs {
        let rec = s.send(m, &clock).await;
        assert!(
            matches!(rec.disposition, wire::Disposition::Accepted { .. }),
            "{}: {rec:?}",
            m.token
        );
    }
    s.quit().await;
}

/// Everything sent is listed (a real deadline, not a sleep), by token.
async fn listed_all(owned: &targets::Owned, want: usize, max_pages: u32) -> Vec<(String, String)> {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        match owned.list_all(max_pages).await {
            Ok(v) if v.len() == want => return v,
            other => assert!(
                Instant::now() < until,
                "{} never listed {want} messages: {:?}",
                owned.kind,
                other.map(|v| v.len())
            ),
        }
        tokio::task::yield_now().await;
    }
}

/// MailSlurper (and the Alkemio fork) keep one decoded body per message and
/// no raw message: the HTML part when there is one, else the plain text with
/// each newline turned into `<br />` (`DataCommandExecutor.decodeBody` in the
/// pinned source; observed on the real binaries). A multipart message's plain
/// part is dropped. So their body fidelity is not exact by construction, and
/// a collection reports it per message through `fidelity_ok`; these proofs
/// assert the contract they do keep.
fn loses_under_burst(t: TargetKind) -> bool {
    stores_decoded_body(t)
}

fn stores_decoded_body(t: TargetKind) -> bool {
    matches!(
        t,
        TargetKind::Mailslurper
            | TargetKind::MailslurperSourceBuild
            | TargetKind::AlkemioMailslurper
    )
}

/// What a decoded-body target returns for one authored message: an HTML
/// message's HTML as sent, a plain message's text with only `<br />` markers
/// added, and for a multipart message nothing but its subject (the one stored
/// body is not readable as a MIME part).
fn decoded_body_contract(t: TargetKind, m: &wire::Message, got: &targets::Fetched) {
    match m.kind {
        protocol::MessageKind::Plain => {
            let stripped = got
                .text
                .as_deref()
                .map(|x| x.replace("<br />", "").replace("<br/>", ""));
            assert!(
                wire::same_body(m.text.as_deref(), stripped.as_deref()),
                "{t} plain text {got:?}"
            );
        }
        protocol::MessageKind::Html => assert!(
            wire::same_body(m.html.as_deref(), got.html.as_deref()),
            "{t} html {got:?}"
        ),
        _ => {}
    }
}

async fn real_round_trip(t: TargetKind) {
    let root = tempfile::tempdir().unwrap();
    let mut owned = real_launch(t, root.path()).await;
    if t == TargetKind::Greenmail {
        // The recipient's POP3 login must work before anything is sent, and
        // the store must read as empty: a rejected login on an empty store
        // would otherwise look like a competitor that lost mail.
        assert_eq!(
            owned.list_all(1).await,
            Ok(Vec::new()),
            "greenmail: login as the workload recipient and STAT +OK 0 before any delivery"
        );
    }
    let msgs = corpus(6);
    send_all(&owned, &msgs).await;
    let listed = listed_all(&owned, msgs.len(), 20).await;
    for m in &msgs {
        let (_, id) = listed
            .iter()
            .find(|(tok, _)| *tok == m.token)
            .unwrap_or_else(|| panic!("{t}: {} is not listed", m.token));
        let got = owned
            .fetch(id)
            .await
            .unwrap_or_else(|e| panic!("{t}: fetch {}: {e:?}", m.token));
        assert_eq!(
            got.subject.as_deref(),
            Some(m.subject.as_str()),
            "{t} {:?}",
            m.kind
        );
        if stores_decoded_body(t) {
            decoded_body_contract(t, m, &got);
        } else {
            assert!(
                wire::same_body(m.text.as_deref(), got.text.as_deref()),
                "{t} {:?} text {got:?}",
                m.kind
            );
            assert!(
                wire::same_body(m.html.as_deref(), got.html.as_deref()),
                "{t} {:?} html {got:?}",
                m.kind
            );
        }
        assert_eq!(
            owned.find(&m.token).await.unwrap(),
            vec![m.token.clone()],
            "{t}: client-side find"
        );
    }
    let facts = owned.shutdown().await;
    assert!(facts.proved(), "{t}: {facts:?}");
}

/// The hostile set `hostile_bodies_round_trip` proves on authored fixtures,
/// authored again as whole messages to send: dot-leading lines and a lone `.`
/// line in a body that SMTP must stuff, an RFC 2047 (8-bit) subject, a header
/// line of 998 octets, and a boundary string in the body, at a line start
/// (content) and mid-line (a legal body that cuts the part for a MIME parser).
/// Every subject carries the corpus token so a listing can name the message.
fn hostile_messages() -> Vec<wire::Message> {
    let long_header = format!("X-Long: {}", "a".repeat(998 - "X-Long: ".len()));
    assert_eq!(long_header.len(), 998);
    let message = |n: u32, kind, subject: &str, ct: &str, body: &str| {
        let token = format!("mc-host-{n:06}");
        let head = format!(
            "From: <{}>\r\nTo: <{}>\r\nSubject: {subject}\r\nMessage-ID: <{token}@mail-compare.invalid>\r\nDate: Wed, 28 Oct 2026 00:00:00 +0000\r\nMIME-Version: 1.0\r\n{long_header}\r\nContent-Type: {ct}\r\n",
            wire::FROM,
            wire::TO
        );
        (token, kind, format!("{head}\r\n{body}").into_bytes())
    };
    let multipart = |n: u32, line: &str| {
        let b = format!("b-mc-host-{n:06}");
        message(
            n,
            protocol::MessageKind::Multipart,
            &format!("mail-compare mc-host-{n:06} plain ascii"),
            &format!("multipart/alternative; boundary=\"{b}\""),
            &format!(
                "--{b}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{line}\r\n--other is not a boundary\r\n.dotted\r\n--{b}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>html part</p>\r\n--{b}--\r\n"
            ),
        )
    };
    let plain = message(
        0,
        protocol::MessageKind::Plain,
        "mail-compare mc-host-000000 =?UTF-8?B?aMOpbGxv?=",
        "text/plain; charset=utf-8",
        ".leading dot\r\n..two dots\r\n.\r\nlast line\r\n",
    );
    let at_line_start = multipart(1, "see the boundary below");
    let mid_line = multipart(2, "see --b-mc-host-000002 for details");
    let expected = [
        (
            "mail-compare mc-host-000000 héllo".to_owned(),
            ".leading dot\r\n..two dots\r\n.\r\nlast line\r\n".to_owned(),
            None,
        ),
        (
            "mail-compare mc-host-000001 plain ascii".to_owned(),
            "see the boundary below\r\n--other is not a boundary\r\n.dotted".to_owned(),
            Some("<p>html part</p>".to_owned()),
        ),
        (
            "mail-compare mc-host-000002 plain ascii".to_owned(),
            "see --b-mc-host-000002 for details\r\n--other is not a boundary\r\n.dotted".to_owned(),
            Some("<p>html part</p>".to_owned()),
        ),
    ];
    [plain, at_line_start, mid_line]
        .into_iter()
        .zip(expected)
        .map(
            |((token, kind, bytes), (subject, text, html))| wire::Message {
                token,
                kind,
                subject,
                text: Some(text),
                html,
                bytes,
            },
        )
        .collect()
}

/// Send the hostile set to a real server and read each message back through
/// its native read path: the subject, text and HTML must equal what was sent.
/// The mid-line boundary message may be refused as malformed by the driver's
/// own MIME reader (a cut part is not a success) and may never come back
/// truncated.
async fn real_hostile(t: TargetKind) {
    let root = tempfile::tempdir().unwrap();
    let mut owned = real_launch(t, root.path()).await;
    let msgs = hostile_messages();
    send_all(&owned, &msgs).await;
    let listed = listed_all(&owned, msgs.len(), 20).await;
    for m in &msgs {
        let (_, id) = listed
            .iter()
            .find(|(tok, _)| *tok == m.token)
            .unwrap_or_else(|| panic!("{t}: hostile {} is not listed: {listed:?}", m.token));
        match owned.fetch(id).await {
            Ok(got) if stores_decoded_body(t) => {
                // The decoded-body targets keep one body and do not undo
                // SMTP dot-stuffing (observed: ".leading dot" comes back as
                // "..leading dot"), so only the subject is asserted; the
                // observation is printed for the delivery notes.
                assert_eq!(got.subject.as_deref(), Some(m.subject.as_str()), "{t}");
                eprintln!(
                    "OBSERVED {t} {}: text {:?} html {:?}",
                    m.token, got.text, got.html
                );
            }
            Ok(got) => {
                assert_eq!(got.subject.as_deref(), Some(m.subject.as_str()), "{t}");
                assert!(
                    wire::same_body(m.text.as_deref(), got.text.as_deref()),
                    "{t} {}: text sent {:?} got {:?}",
                    m.token,
                    m.text,
                    got.text
                );
                assert!(
                    wire::same_body(m.html.as_deref(), got.html.as_deref()),
                    "{t} {}: html sent {:?} got {:?}",
                    m.token,
                    m.html,
                    got.html
                );
            }
            Err(wire::WireError::Malformed) if m.token.ends_with("000002") => {}
            Err(e) => panic!("{t}: fetch {}: {e:?}", m.token),
        }
    }
    let facts = owned.shutdown().await;
    assert!(facts.proved(), "{t}: {facts:?}");
}

/// Each tool's default retention (smtp4dev 100 per mailbox, Inbucket 500 per
/// mailbox) is overridden at launch: 600 messages all stay.
async fn real_caps(t: TargetKind) {
    let root = tempfile::tempdir().unwrap();
    let mut owned = real_launch(t, root.path()).await;
    let msgs = corpus(600);
    if loses_under_burst(t) {
        // MailSlurper acknowledges a mail before it stores it and loses
        // some under SQLite lock contention ("database is locked", 27 of 600
        // in a probe sent in 0.23 s). That is an observation about the tool,
        // recorded in the delivery notes, not a retention cap: the cap proof
        // sends in small batches and waits for each to be listed, so only
        // eviction could lower the count.
        for (n, batch) in msgs.chunks(5).enumerate() {
            send_all(&owned, batch).await;
            listed_all(&owned, (n + 1) * 5, 100).await;
        }
    } else {
        send_all(&owned, &msgs).await;
    }
    assert_eq!(
        listed_all(&owned, 600, 100).await.len(),
        600,
        "{t}: nothing was evicted"
    );
    assert!(owned.shutdown().await.proved(), "{t}");
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_SMTP4DEV (the pinned smtp4dev executable)"]
async fn real_smtp4dev_smtp_to_native_api_round_trip() {
    real_round_trip(TargetKind::Smtp4dev).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_INBUCKET (the pinned Inbucket executable)"]
async fn real_inbucket_smtp_to_native_api_round_trip() {
    real_round_trip(TargetKind::Inbucket).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD (the pinned cgo build of MailSlurper)"]
async fn real_mailslurper_source_build_smtp_to_native_api_round_trip() {
    real_round_trip(TargetKind::MailslurperSourceBuild).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_ALKEMIO_MAILSLURPER; MAIL_COMPARE_MUSL (the extracted Alkemio binary and its pinned loader root)"]
async fn real_alkemio_mailslurper_smtp_to_native_api_round_trip() {
    real_round_trip(TargetKind::AlkemioMailslurper).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_GREENMAIL; MAIL_COMPARE_JRE (the pinned jar and the JRE root)"]
async fn real_greenmail_smtp_to_native_api_round_trip() {
    real_round_trip(TargetKind::Greenmail).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_SMTP4DEV (the pinned smtp4dev executable)"]
async fn real_smtp4dev_hostile_round_trip() {
    real_hostile(TargetKind::Smtp4dev).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_INBUCKET (the pinned Inbucket executable)"]
async fn real_inbucket_hostile_round_trip() {
    real_hostile(TargetKind::Inbucket).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD (the pinned cgo build of MailSlurper)"]
async fn real_mailslurper_source_build_hostile_round_trip() {
    real_hostile(TargetKind::MailslurperSourceBuild).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_ALKEMIO_MAILSLURPER; MAIL_COMPARE_MUSL (the extracted Alkemio binary and its pinned loader root)"]
async fn real_alkemio_mailslurper_hostile_round_trip() {
    real_hostile(TargetKind::AlkemioMailslurper).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_GREENMAIL; MAIL_COMPARE_JRE (the pinned jar and the JRE root)"]
async fn real_greenmail_hostile_round_trip() {
    real_hostile(TargetKind::Greenmail).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_SMTP4DEV (the pinned smtp4dev executable)"]
async fn real_caps_hold_smtp4dev() {
    real_caps(TargetKind::Smtp4dev).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_INBUCKET (the pinned Inbucket executable)"]
async fn real_caps_hold_inbucket() {
    real_caps(TargetKind::Inbucket).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD (the pinned cgo build of MailSlurper)"]
async fn real_caps_hold_mailslurper_source_build() {
    real_caps(TargetKind::MailslurperSourceBuild).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_ALKEMIO_MAILSLURPER; MAIL_COMPARE_MUSL (the extracted Alkemio binary and its pinned loader root)"]
async fn real_caps_hold_alkemio_mailslurper() {
    real_caps(TargetKind::AlkemioMailslurper).await;
}

#[tokio::test]
#[ignore = "needs MAIL_COMPARE_GREENMAIL; MAIL_COMPARE_JRE (the pinned jar and the JRE root)"]
async fn real_caps_hold_greenmail() {
    real_caps(TargetKind::Greenmail).await;
}

/// The pinned official MailSlurper executable does not start: it exits
/// non-zero within the startup deadline with go-sqlite3's cgo stub line,
/// binds none of its declared listeners, and the launcher's own cleanup is
/// proved. The executable runs as its pin launches it (`-config` on the
/// rendered config), its stderr read directly because the launcher discards
/// a child's output.
#[tokio::test]
#[ignore = "needs MAIL_COMPARE_MAILSLURPER (the pinned official MailSlurper executable)"]
async fn real_mailslurper_official_does_not_start() {
    let t = TargetKind::Mailslurper;
    let lock = lock4(v4());
    let pin = lock.pin(t).expect("mailslurper is pinned");
    let var = env_name(t);
    let exe = PathBuf::from(
        std::env::var_os(&var).unwrap_or_else(|| panic!("{var} must name the pinned executable")),
    );
    assert_eq!(
        sha(&std::fs::read(&exe).expect("readable")),
        pin.artifact.executable_sha256,
        "the executable is not the pinned bytes"
    );
    // The published result is in the committed protocol, citing a document
    // of this pin.
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    assert!(p.does_not_start(t));
    // Ports for the rendered config: reserved, released, never connected to.
    let ports: Vec<u16> = (0..3)
        .map(|_| {
            std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port()
        })
        .collect();
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let roles = [
        (
            protocol::ListenerRole::Smtp,
            format!("127.0.0.1:{}", ports[0]),
        ),
        (
            protocol::ListenerRole::Http,
            format!("127.0.0.1:{}", ports[1]),
        ),
        (
            protocol::ListenerRole::Ui,
            format!("127.0.0.1:{}", ports[2]),
        ),
    ];
    let cfg = pin.config_file.as_ref().expect("a config file");
    let text = cfg
        .lines
        .iter()
        .map(|l| targets::substitute(l, &roles, (&data, root.path()), None, None))
        .collect::<Vec<_>>()
        .join("\n");
    let cfg_path = root.path().join(&cfg.name);
    std::fs::write(&cfg_path, text).unwrap();
    let started = Instant::now();
    let out = tokio::process::Command::new(&exe)
        .args(["-config", cfg_path.to_str().unwrap()])
        .current_dir(root.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(limits().startup, out)
        .await
        .expect("the executable exits within the startup deadline")
        .expect("it runs");
    assert!(started.elapsed() < limits().startup);
    assert!(!out.status.success(), "it must not start: {:?}", out.status);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Binary was compiled with 'CGO_ENABLED=0'"),
        "{stderr}"
    );
    // The recorded failure output names the same stub line.
    let recorded = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/evals/sources/mailslurper/mailslurper-official-failure.txt"),
    )
    .unwrap();
    assert!(recorded.contains("Binary was compiled with 'CGO_ENABLED=0'"));
    // No declared listener is bound.
    for port in &ports {
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", *port))
                .await
                .is_err(),
            "port {port} must not be bound"
        );
    }
    // The launcher's own path refuses it and proves the cleanup.
    let spec = real_spec(t);
    let clock = wire::Clock(Instant::now());
    match targets::Owned::launch(
        &spec,
        &root.path().join("leaf"),
        &limits(),
        &clock,
        &targets::Keepers::default(),
    )
    .await
    {
        Err(LaunchError::NotReady(_, facts)) => assert!(facts.proved(), "{facts:?}"),
        other => panic!(
            "the official executable must not launch: {:?}",
            other.map(|_| ())
        ),
    }
}

/// A real directory for a test's own files, under `target/` and never the
/// RAM-backed temporary directory (the archives here are large).
fn fresh_dir(label: &str) -> tempfile::TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/compare-dl");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::Builder::new()
        .prefix(&format!("{label}-"))
        .tempdir_in(base)
        .unwrap()
}

/// Every file (one directory level) of the colon-separated directories in
/// `MAIL_COMPARE_ARCHIVE_DIRS` whose sha256 is `want`.
fn archive_by_digest(want: &str) -> PathBuf {
    let dirs = std::env::var("MAIL_COMPARE_ARCHIVE_DIRS")
        .expect("MAIL_COMPARE_ARCHIVE_DIRS must list the directories holding the pinned archives");
    for d in dirs.split(':') {
        for entry in std::fs::read_dir(d).unwrap_or_else(|e| panic!("{d}: {e}")) {
            let path = entry.unwrap().path();
            if path.is_file()
                && path.metadata().unwrap().len() > 0
                && sha(&std::fs::read(&path).unwrap()) == want
            {
                return path;
            }
        }
    }
    panic!("no archive with sha256 {want} in {dirs}");
}

fn extract_member(archive: &Path, member: &str, into: &Path) -> Vec<u8> {
    let name = archive.to_string_lossy();
    let status = if name.ends_with(".tar.gz") {
        std::process::Command::new("tar")
            .args(["-xzf", &name, "-C"])
            .arg(into)
            .arg(member)
            .status()
    } else {
        std::process::Command::new("unzip")
            .args(["-q", "-o", &name, member, "-d"])
            .arg(into)
            .status()
    }
    .expect("tar or unzip runs");
    assert!(status.success(), "extracting {member} from {name}");
    std::fs::read(into.join(member)).unwrap_or_else(|e| panic!("{member}: {e}"))
}

/// Every `Member` document is re-extracted from its archive with the system
/// `unzip`/`tar` into a fresh directory, every `Output` document is
/// reproduced by running the pinned executable bounded, and each committed
/// document's sha256 must match. The MailSlurper source tarball's own pax
/// comment names the tagged commit.
#[test]
#[ignore = "needs MAIL_COMPARE_ARCHIVE_DIRS and each pinned executable (MAIL_COMPARE_<TARGET>, MAIL_COMPARE_JRE)"]
fn real_member_and_output_documents_match() {
    let lock = lock4(v4());
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/evals");
    let (mut members, mut outputs) = (0, 0);
    for e in &lock.entries {
        let Some(pin) = &e.pin else { continue };
        let t = e.target;
        for d in &pin.documents {
            let committed = std::fs::read(docs.join(&d.path)).unwrap();
            assert_eq!(sha(&committed), d.sha256, "{t}: {}", d.path);
            match &d.attribution {
                protocol::Fact::Observed(protocol::Attribution::Member {
                    archive_sha256,
                    member,
                }) => {
                    let archive = archive_by_digest(archive_sha256);
                    let dir = fresh_dir("member");
                    let got = extract_member(&archive, member, dir.path());
                    assert_eq!(sha(&got), d.sha256, "{t}: member {member} of {archive:?}");
                    members += 1;
                }
                protocol::Fact::Observed(protocol::Attribution::Output { argv }) => {
                    let var = env_name(t);
                    let binary = std::env::var_os(&var)
                        .unwrap_or_else(|| panic!("{var} must name the pinned executable"));
                    assert_eq!(
                        sha(&std::fs::read(&binary).unwrap()),
                        pin.artifact.executable_sha256,
                        "{t}: not the pinned bytes"
                    );
                    let program = match (&pin.runtime, protocol::runtime_rule(t)) {
                        (Some(_), Some(rule)) => PathBuf::from(
                            std::env::var_os(runtime_env(t)).expect("the runtime root"),
                        )
                        .join(rule.entry),
                        _ => PathBuf::from(&binary),
                    };
                    let args: Vec<String> = argv
                        .iter()
                        .map(|a| a.replace("{artifact}", &binary.to_string_lossy()))
                        .collect();
                    let home = fresh_dir("output");
                    let got = std::process::Command::new("timeout")
                        .args(["60", "/bin/sh", "-c", "exec \"$0\" \"$@\" 2>&1"])
                        .arg(&program)
                        .args(&args)
                        .env_clear()
                        .env("PATH", "/usr/bin:/bin")
                        .env("LANG", "C")
                        .env("HOME", home.path())
                        .env("DOTNET_BUNDLE_EXTRACT_BASE_DIR", home.path().join("x"))
                        .current_dir(home.path())
                        .output()
                        .expect("the program runs");
                    assert!(got.status.success(), "{t}: {argv:?}: {:?}", got.status);
                    assert_eq!(sha(&got.stdout), d.sha256, "{t}: output of {argv:?}");
                    outputs += 1;
                }
                _ => {}
            }
        }
    }
    assert!(
        members > 0 && outputs > 0,
        "{members} members, {outputs} outputs"
    );
    // The MailSlurper source tarball is of the tagged commit: its pax header
    // comment is that commit's id.
    let (_, sha_hex, commit) = protocol::SOURCE_ARCHIVES[0];
    let tarball = archive_by_digest(sha_hex);
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg("gzip -dc \"$0\" | git get-tar-commit-id")
        .arg(&tarball)
        .output()
        .expect("git get-tar-commit-id runs");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), commit);
}

// ---- the labelled smoke fixture ------------------------------------------

#[test]
fn eight_target_smoke_fixture_is_a_valid_draft_that_cannot_launch_yet() {
    let smoke = include_bytes!("eight-target-smoke.json");
    let p = Protocol::parse(smoke).unwrap();
    p.validate()
        .expect("the smoke fixture is a valid protocol 3 draft");
    assert_eq!(p.status, protocol::Status::Draft);
    assert_eq!(p.targets, TargetKind::ALL.to_vec());
    let committed = Protocol::parse(PROTOCOL_V3).unwrap();
    assert_eq!(p.corpus, committed.corpus);
    assert_eq!(p.storage, committed.storage);
    assert_eq!(p.capabilities, committed.capabilities);
    assert!(
        p.throughput[0].messages < 100 && p.waits[0].present < 10,
        "tiny profiles only"
    );
    let lock = lock4(v4());
    protocol::pairing(&p, &lock).unwrap();
    // It never collects (it is a draft), and its smoke run launches every
    // declared row: each has a pin.
    let why = refusals(protocol::collection_ready(&p, &lock));
    assert!(why.contains("protocol is a draft, not frozen"), "{why}");
    protocol::launchable(&p, &lock).expect("every declared row has a pin");
    // The official MailSlurper row's published result rides in the draft too.
    assert!(p.does_not_start(TargetKind::Mailslurper));
    assert_eq!(
        protocol::declared_competitors(&p).len(),
        TargetKind::ALL.len() - 2
    );
}

fn strings(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| (*s).to_owned()).collect()
}

/// `--bin` values for every launched target of the committed protocol 3.
fn v3_bins(p: &Protocol) -> Vec<String> {
    p.launch_order(0).map(|t| format!("{t}=/x/{t}")).collect()
}

#[test]
fn flags_repeat_bin_and_runtime_only() {
    let f = runner::flags(&strings(&[
        "--runtime",
        "greenmail=/jre",
        "--runtime",
        "alkemio-mailslurper=/musl",
        "--bin",
        "swarmail=/s",
        "--bin",
        "mailpit=/m",
        "--mode",
        "smoke",
    ]))
    .unwrap();
    assert_eq!(
        f["runtime"],
        ["greenmail=/jre", "alkemio-mailslurper=/musl"]
    );
    assert_eq!(f["bin"], ["swarmail=/s", "mailpit=/m"]);
    assert_eq!(f["mode"], ["smoke"]);
    for twice in ["protocol", "sources", "output", "mode"] {
        let why = runner::flags(&strings(&[
            &format!("--{twice}"),
            "a",
            &format!("--{twice}"),
            "b",
        ]))
        .unwrap_err();
        assert_eq!(why, format!("--{twice} given twice"));
    }
    assert!(
        runner::flags(&strings(&["stray"]))
            .unwrap_err()
            .contains("unexpected")
    );
    assert!(
        runner::flags(&strings(&["--bin"]))
            .unwrap_err()
            .contains("needs a value")
    );
}

#[test]
fn launch_specs_take_two_runtimes() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let lock = lock4(v4());
    let specs = runner::launch_specs(
        &p,
        &lock,
        &v3_bins(&p),
        &strings(&["greenmail=/jre", "alkemio-mailslurper=/musl"]),
    )
    .unwrap();
    // The official MailSlurper row is published as not starting: no spec.
    assert_eq!(specs.len(), TargetKind::ALL.len() - 1);
    assert!(!specs.contains_key(&TargetKind::Mailslurper));
    // GreenMail execs its JRE with the jar as the artifact; the Alkemio row
    // execs its musl loader with the extracted binary as the artifact.
    let g = &specs[&TargetKind::Greenmail];
    assert_eq!(g.binary, Path::new("/jre").join("bin/java"));
    assert_eq!(g.extra.artifact.as_deref(), Some(Path::new("/x/greenmail")));
    let a = &specs[&TargetKind::AlkemioMailslurper];
    assert_eq!(a.binary, Path::new("/musl").join("lib/ld-musl-x86_64.so.1"));
    assert_eq!(
        a.extra.artifact.as_deref(),
        Some(Path::new("/x/alkemio-mailslurper"))
    );
    // The others run their own binary directly.
    assert_eq!(
        specs[&TargetKind::Inbucket].binary,
        Path::new("/x/inbucket")
    );
    assert!(specs[&TargetKind::Inbucket].extra.artifact.is_none());
    assert_eq!(
        specs[&TargetKind::Swarmail].binary,
        Path::new("/x/swarmail")
    );
    // A pin that needs a runtime refuses without one, and one that has none
    // refuses one, and a runtime for a row the protocol does not launch.
    let why =
        runner::launch_specs(&p, &lock, &v3_bins(&p), &strings(&["greenmail=/jre"])).unwrap_err();
    assert!(
        why.contains("--runtime alkemio-mailslurper=DIR is required"),
        "{why}"
    );
    let why = runner::launch_specs(
        &p,
        &lock,
        &v3_bins(&p),
        &strings(&["greenmail=/jre", "alkemio-mailslurper=/m", "inbucket=/r"]),
    )
    .unwrap_err();
    assert!(
        why.contains("--runtime inbucket is given but its pin has no runtime"),
        "{why}"
    );
    let why = runner::launch_specs(
        &p,
        &lock,
        &v3_bins(&p),
        &strings(&["greenmail=/jre", "alkemio-mailslurper=/m", "mailslurper=/r"]),
    )
    .unwrap_err();
    assert!(
        why.contains("--runtime mailslurper names a target"),
        "{why}"
    );
}

#[test]
fn launch_specs_refuse_a_bin_for_a_does_not_start_target() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let lock = lock4(v4());
    let mut bins = v3_bins(&p);
    bins.push("mailslurper=/x/mailslurper".into());
    let why = runner::launch_specs(
        &p,
        &lock,
        &bins,
        &strings(&["greenmail=/jre", "alkemio-mailslurper=/musl"]),
    )
    .unwrap_err();
    assert!(
        why.contains("--bin mailslurper names a target the protocol does not declare or publishes as not starting"),
        "{why}"
    );
    // A launched target without its --bin refuses by name.
    let few: Vec<String> = v3_bins(&p)
        .into_iter()
        .filter(|b| !b.starts_with("inbucket="))
        .collect();
    let why = runner::launch_specs(
        &p,
        &lock,
        &few,
        &strings(&["greenmail=/jre", "alkemio-mailslurper=/musl"]),
    )
    .unwrap_err();
    assert!(why.contains("--bin inbucket=PATH is required"), "{why}");
    // Malformed values and unknown targets refuse.
    let why = runner::launch_specs(&p, &lock, &strings(&["nopath"]), &[]).unwrap_err();
    assert!(why.contains("--bin takes TARGET=PATH"), "{why}");
    let why = runner::launch_specs(&p, &lock, &strings(&["nobody=/x"]), &[]).unwrap_err();
    assert!(why.contains("unknown target"), "{why}");
}

#[test]
fn launch_specs_refuse_a_runtime_given_twice_for_one_target() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let lock = lock4(v4());
    let why = runner::launch_specs(
        &p,
        &lock,
        &v3_bins(&p),
        &strings(&[
            "greenmail=/jre",
            "greenmail=/other",
            "alkemio-mailslurper=/musl",
        ]),
    )
    .unwrap_err();
    assert_eq!(why, "--runtime greenmail given twice");
    let mut bins = v3_bins(&p);
    bins.push("swarmail=/again".into());
    let why = runner::launch_specs(&p, &lock, &bins, &[]).unwrap_err();
    assert_eq!(why, "--bin swarmail given twice");
    let why = runner::launch_specs(&p, &lock, &v3_bins(&p), &strings(&["greenmail"])).unwrap_err();
    assert!(why.contains("--runtime takes TARGET=DIR"), "{why}");
}

/// The nine-target smoke fixture with every competitor published as not
/// starting: the candidate is the only launched target, so a real run needs
/// one launch spec and no competitor binary. The cites are never read (only
/// `collection_ready` reads them, and this is a smoke).
fn candidate_alone_v3() -> Protocol {
    let mut v = json(include_bytes!("eight-target-smoke.json"));
    let skipped: Vec<Value> = TargetKind::ALL
        .into_iter()
        .filter(|t| *t != TargetKind::Swarmail)
        .map(|t| json!({"target": t.name(), "cite": format!("sources/{t}/failure.txt")}))
        .collect();
    v["does_not_start"] = Value::Array(skipped);
    let p = protocol_from(v).unwrap();
    p.validate().expect("a valid protocol 3 smoke");
    assert!(protocol::declared_competitors(&p).is_empty());
    p
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn run_never_launches_a_does_not_start_target() {
    let p = candidate_alone_v3();
    // A competitor with no launch spec is Infrastructure the moment the run
    // reaches it; a does_not_start target is never reached.
    let root = tempfile::tempdir().unwrap();
    let (verdict, life, sealed) = run(p.clone(), root.path(), "never-launches").await;
    assert_eq!(verdict, runner::Verdict::Clean, "{verdict:?}");
    assert!(!life.launches.is_empty());
    assert!(
        life.launches
            .iter()
            .all(|l| l.target == TargetKind::Swarmail),
        "only the candidate launched"
    );
    assert!(life.cleanups.iter().all(targets::CleanupFacts::proved));
    let dir = sealed.expect("a clean run is sealed");
    let rows = rows(&dir);
    assert!(rows.iter().all(|r| r.target == TargetKind::Swarmail));
    let report: Value =
        serde_json::from_slice(&std::fs::read(dir.join("report.json")).unwrap()).unwrap();
    let blocked = report["blocked"].as_array().unwrap();
    assert_eq!(blocked.len(), TargetKind::ALL.len() - 1);
    for b in blocked {
        assert!(
            b["result"]
                .as_str()
                .unwrap()
                .starts_with("does not start (published result: sources/"),
            "{b}"
        );
        assert!(
            b["cells"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["outcome"] == "blocked"),
            "{b}"
        );
    }
    report::admit_run(&dir, &serde_json::to_vec(&p).unwrap()).expect("it admits");
}

#[tokio::test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
async fn admit_run_refuses_a_v3_report_with_an_edited_blocked_row() {
    let p = candidate_alone_v3();
    let bytes = serde_json::to_vec(&p).unwrap();
    let root = tempfile::tempdir().unwrap();
    let (verdict, _, sealed) = run(p.clone(), root.path(), "sealed-v3").await;
    assert_eq!(verdict, runner::Verdict::Clean, "{verdict:?}");
    let good = sealed.unwrap();
    report::admit_run(&good, &bytes).expect("the untampered v3 report admits");
    let m = records::read_manifest(&good).unwrap();
    let all = rows(&good);
    let on_disk: Value =
        serde_json::from_slice(&std::fs::read(good.join("report.json")).unwrap()).unwrap();
    // A sealed copy with an edited report: storage accepts it, so only the
    // protocol can refuse.
    let forge = |name: &str, report: &Value| {
        let mut dir = records::RunDir::create(&root.path().join(name), &m).unwrap();
        for r in &all {
            dir.append(r).unwrap();
        }
        dir.write_artifact("report.json", &serde_json::to_vec(report).unwrap())
            .unwrap();
        let path = dir.finalize(true).expect("storage accepts it");
        records::admit(&path).expect("and so does the directory-level admission");
        path
    };
    let refused =
        |name: &str, report: &Value| report::admit_run(&forge(name, report), &bytes).unwrap_err();
    forge("same", &on_disk);
    report::admit_run(&root.path().join("same"), &bytes).expect("an unedited copy admits");

    // A dropped blocked row.
    let mut dropped = on_disk.clone();
    dropped["blocked"].as_array_mut().unwrap().pop();
    let why = refused("dropped", &dropped);
    assert!(
        why.contains("disagrees with the rows") && why.contains("blocked"),
        "{why}"
    );
    // A blocked cell given a number.
    let mut numbered = on_disk.clone();
    numbered["blocked"][0]["cells"][0]["rate"] = 12.5.into();
    let why = refused("numbered", &numbered);
    assert!(why.contains("blocked[0].cells[0]"), "{why}");
    let mut outcome = on_disk.clone();
    outcome["blocked"][0]["cells"][0]["outcome"] = "measured".into();
    let why = refused("outcome", &outcome);
    assert!(why.contains("blocked[0].cells[0].outcome"), "{why}");
    // An edited label: a runtime claimed for a row that pins none.
    let mut labelled = on_disk.clone();
    let at = TargetKind::ALL
        .iter()
        .position(|t| *t == TargetKind::Inbucket)
        .unwrap();
    labelled["labels"][at]["runtime"] = "JRE (pinned)".into();
    let why = refused("labelled", &labelled);
    assert!(why.contains("labels[") && why.contains("runtime"), "{why}");
    // A dropped label key set.
    let mut unlabelled = on_disk.clone();
    unlabelled.as_object_mut().unwrap().remove("labels");
    let why = refused("unlabelled", &unlabelled);
    assert!(why.contains("labels"), "{why}");
}

#[test]
fn report_labels_name_every_unpinned_runtime() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let labels = report::row_labels(&p);
    let runtime = |t: TargetKind| labels.iter().find(|l| l.target == t).unwrap().runtime;
    assert_eq!(
        runtime(TargetKind::Smtp4dev),
        Some("self-contained .NET on host libraries (unpinned)")
    );
    assert_eq!(
        runtime(TargetKind::MailslurperSourceBuild),
        Some("host glibc (unpinned)")
    );
    // The pinned runtimes keep their labels, and the rest carry none.
    assert_eq!(runtime(TargetKind::Greenmail), Some("JRE (pinned)"));
    assert_eq!(
        runtime(TargetKind::AlkemioMailslurper),
        Some("musl loader (pinned, from the image's own layer)")
    );
    for t in [
        TargetKind::Swarmail,
        TargetKind::Mailpit,
        TargetKind::Mailhog,
        TargetKind::Mailslurper,
        TargetKind::Inbucket,
    ] {
        assert_eq!(runtime(t), None, "{t}");
    }
}

#[test]
fn wrapper_collect_needs_a_declared_competitor() {
    let root = tempfile::tempdir().unwrap();
    let w = root.path().join("w");
    let out = root.path().join("out");
    let o = out.to_str().unwrap();
    let smoke =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mail_compare/swarmail-smoke.json");
    let s = smoke.to_str().unwrap();
    // `collect` measures competitors: a Swarmail-only protocol cannot collect.
    // (`local` over it asks for no competitor binary; the bounded smoke run
    // in the delivery notes covers that path, since it builds the candidate.)
    let (_, text) = wrapper(&["collect", "--output", o, "--protocol", s], &w);
    assert!(
        text.contains("collect measures competitors, and the protocol")
            && text.contains("declares none"),
        "{text}"
    );
    assert!(!out.exists());
}
