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
const V3_LOCK_SHA: &str = "6523abf4df890d7aca8ea7e5112ae3e32ab35f2260e2528b070eefa70007b03c";

const NEW_TARGETS: [TargetKind; 5] = [
    TargetKind::Mailslurper,
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
        assert_eq!(sha(&std::fs::read(history.join(name)).unwrap()), want, "{name}");
    }
}

#[test]
fn history_v3_lock_still_validates_verifies_and_pairs_with_v2() {
    let lock = SourceLock::parse(SOURCES).unwrap();
    lock.validate().expect("lock 3 still validates");
    lock.verify_files(&evals_dir()).expect("its documents verify");
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
    assert_eq!(TargetKind::ALL.len(), 8);
    for t in TargetKind::ALL {
        assert_eq!(TargetKind::parse(t.name()), Some(t));
        assert_eq!(serde_json::to_value(t).unwrap(), t.name());
        assert_eq!(
            serde_json::from_value::<TargetKind>(json!(t.name())).unwrap(),
            t
        );
    }
    for name in ["latest", "MailSlurper", "alkemiomailslurper", "", "greenmail "] {
        assert_eq!(TargetKind::parse(name), None, "{name:?}");
        assert!(serde_json::from_value::<TargetKind>(json!(name)).is_err());
    }
    assert_eq!(TargetKind::V2, [TargetKind::ALL[0], TargetKind::ALL[1], TargetKind::ALL[2]]);
    assert_eq!(TargetKind::ALL.iter().filter(|t| t.name().starts_with("alkemio")).count(), 1);
}

#[test]
fn alkemio_name_is_not_mailslurper() {
    let a = TargetKind::AlkemioMailslurper;
    let m = TargetKind::Mailslurper;
    assert_ne!(a.name(), m.name());
    assert_ne!(a.provenance(), m.provenance());
    assert!(matches!(a.provenance(), protocol::Provenance::ThirdPartyImage(r) if r.contains("alkem-io")));
    assert!(matches!(m.provenance(), protocol::Provenance::Official(r) if r.ends_with("/mailslurper")));
    // Source directories are keyed by the target name, so the rows never
    // share one, and neither is Swarmail's own MailSlurper shim's test
    // file (decision 0118).
    assert_ne!(format!("sources/{a}/"), format!("sources/{m}/"));
    for t in [a, m] {
        assert_ne!(t.name(), "mailslurper-shim");
    }
    let shim = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mailslurper.rs");
    assert!(shim.exists(), "the shim's own test file is the one decision 0118 names");
    assert!(!std::fs::read_to_string(shim).unwrap().contains("alkemio-mailslurper"));
}

#[test]
fn v2_protocol_and_v3_lock_refuse_each_new_target() {
    for t in NEW_TARGETS {
        // Protocol 2 declaring the target.
        let mut p = json(PROTOCOL);
        p["targets"].as_array_mut().unwrap().push(t.name().into());
        let why = refusals(protocol_from(p).unwrap().validate());
        assert!(
            why.contains(&format!("refuses target {t}")),
            "{t}: {why}"
        );
        // Lock 3 locking it.
        let mut l = v3();
        l["entries"].as_array_mut().unwrap().push(json!({
            "target": t.name(), "role": "competitor", "pin": null
        }));
        let why = refusals(lock4(l).validate());
        assert!(why.contains(&format!("lock_version 3 refuses target {t}")), "{t}: {why}");
    }
}

// ---- version gates and pairing --------------------------------------------

#[test]
fn lock_v3_refuses_each_v4_field() {
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("ref_kind", Box::new(|p| p["tagged_source"]["observed"]["ref_kind"] = "annotated".into())),
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
        ("listeners", Box::new(|p| p["listeners"] = json!(["smtp", "http"]))),
        ("launch_env", Box::new(|p| p["launch_env"] = json!([["A", "b"]]))),
        ("config_file", Box::new(|p| p["config_file"] = json!({"name": "c", "lines": ["x"]}))),
        ("checked_members", Box::new(|p| p["artifact"]["checked_members"] = json!([]))),
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
        assert!(why.contains(&format!("lock_version 3 refuses {field}")), "{field}: {why}");
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
    assert!(why.contains("lock_version 4 needs ref_kind on a tagged pin"), "{why}");
    // And it needs its listeners, every target and a version it knows.
    let mut l = v4();
    let at = entry_of(&mut l, "mailhog");
    l["entries"][at]["pin"].as_object_mut().unwrap().remove("listeners");
    assert!(refusals(lock4(l).validate()).contains("needs the declared listeners"));
    let mut l = v4();
    l["entries"].as_array_mut().unwrap().pop();
    assert!(refusals(lock4(l).validate()).contains("lock_version 4 needs an entry for alkemio-mailslurper"));
}

#[test]
fn v4_tag_shapes_accept_release_bare_v_and_refuse_latest_branch_rc() {
    for ok in ["release-1.15.0", "3.16.1", "v3.2.0", "v1.32.0", "2.1.15"] {
        assert!(protocol::is_version_tag(ok), "{ok}");
    }
    for bad in ["latest", "main", "release-", "v1", "1.15.0-rc", "", "v", "release-1", "vv1.0", "1..2", "master"] {
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
        let protocol::Fact::Observed(ts) = &pin.tagged_source else { panic!("{t}") };
        assert_eq!(ts.ref_kind == Some(protocol::RefKind::Annotated), annotated);
        assert!(ts.tag_object.is_some() && ts.tag_signature.as_deref() == Some("unsigned"));
    }
    // The v3 artifact, release and storage facts are the same values.
    let old = v3();
    let new = v4();
    for t in ["mailpit", "mailhog"] {
        for k in ["release", "artifact", "storage", "wait", "launch", "license"] {
            let at = |v: &Value| {
                v["entries"].as_array().unwrap().iter().find(|e| e["target"] == t).unwrap()["pin"][k].clone()
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
    assert!(why.contains("protocol_version 2 pairs only with lock_version 3, not 4"), "{why}");
    let why = refusals(protocol::pairing(&p3, &l3));
    assert!(why.contains("protocol_version 3 pairs only with lock_version 4, not 3"), "{why}");
    // collection_ready lists it too, among every other refusal.
    assert!(refusals(protocol::collection_ready(&p2, &l4)).contains("pairs only with lock_version 3"));
    assert!(refusals(protocol::collection_ready(&p3, &l3)).contains("pairs only with lock_version 4"));
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
    let exe = if greenmail { "greenmail-standalone-2.1.15.jar" } else { "authored-linux-amd64" };
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
    let mut docs = vec![capture(&sd, "authored-release.json", "release_metadata",
        &serde_json::to_vec(&release).unwrap(),
        json!({"from": "api", "url": format!("{api}/releases/42")}))];
    let mut tree = Vec::new();
    for (name, kind, bytes, upstream) in files {
        docs.push(capture(&sd, name, kind, bytes, json!({"from": "commit",
            "url": format!("{repo}/blob/{C}/{upstream}"), "upstream_path": upstream,
            "git_blob": protocol::git_blob_id(bytes)})));
        if k.omit_from_tree != Some(name) {
            tree.push(json!({"path": upstream, "type": "blob", "sha": protocol::git_blob_id(bytes), "size": bytes.len()}));
        }
    }
    let ref_url = format!("{api}/git/refs/tags/{}", k.tag);
    let ref_body = json!({"ref": format!("refs/tags/{}", k.ref_name), "url": ref_url,
        "object": {"type": k.ref_type, "sha": k.ref_sha}});
    docs.push(capture(&sd, "authored-ref.json", "git_ref", &serde_json::to_vec(&ref_body).unwrap(),
        json!({"from": "api", "url": ref_url})));
    let tree_url = format!("{api}/git/trees/{C}");
    let tree_body = json!({"sha": C, "url": tree_url, "truncated": k.truncated, "tree": tree});
    docs.push(capture(&sd, "authored-tree.json", "git_tree", &serde_json::to_vec(&tree_body).unwrap(),
        json!({"from": "api", "url": tree_url})));
    if k.git_tag_doc {
        let tag_url = format!("{api}/git/tags/{}", "a".repeat(40));
        let body = json!({"sha": "a".repeat(40), "url": tag_url, "tag": k.tag,
            "object": {"type": "commit", "sha": C}, "verification": {"reason": "unsigned"}});
        docs.push(capture(&sd, "authored-tag.json", "git_tag", &serde_json::to_vec(&body).unwrap(),
            json!({"from": "api", "url": tag_url})));
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
        pin["launch"] = json!(["-Dp={smtp_port}", "-Dq={pop3_port}", "-jar", "{artifact}", "--max", "10000"]);
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
    (evals, json!({"target": t.name(), "role": "competitor", "pin": pin}))
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
    lock.verify_files(&dir).expect("the ref names the commit; the tree holds every document");
    let pin = lock.pin(TargetKind::Mailslurper).unwrap();
    let protocol::Fact::Observed(ts) = &pin.tagged_source else { panic!() };
    assert_eq!(ts.ref_kind, Some(protocol::RefKind::Lightweight));
    assert!(ts.tag_object.is_none() && ts.tag_signature.is_none(), "absent, not unsigned");
    assert!(pin.documents.iter().all(|d| d.kind != protocol::DocKind::GitTag));
    for tag in ["release-1.15.0", "3.16.1", "v3.2.0"] {
        let mut k = Knobs::new(TargetKind::Mailslurper);
        k.tag = tag.into();
        k.ref_name = tag.into();
        let (_r, dir, lock) = authored(&k);
        lock.validate().unwrap_or_else(|e| panic!("{tag}: {e}"));
        lock.verify_files(&dir).unwrap_or_else(|e| panic!("{tag}: {e}"));
    }
}

#[test]
fn lightweight_pin_refuses_tag_object_signature_or_git_tag_doc() {
    for (name, edit, want) in [
        ("tag object", (|k: &mut Knobs| k.tag_object = true) as fn(&mut Knobs), "no tag object and no signature"),
        ("signature", |k| k.tag_signature = true, "no tag object and no signature"),
        ("git_tag document", |k| k.git_tag_doc = true, "no git_tag document"),
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
        ("ref of type tag", (|k: &mut Knobs| k.ref_type = "tag".into()) as fn(&mut Knobs)),
        ("another commit", |k| k.ref_sha = "9".repeat(40)),
        ("another tag", |k| k.ref_name = "release-9.9.9".into()),
    ] {
        let mut k = Knobs::new(TargetKind::Mailslurper);
        edit(&mut k);
        let (_r, dir, lock) = authored(&k);
        lock.validate().expect("structure is fine; the captured bytes are what refuse");
        let why = refusals(lock.verify_files(&dir));
        assert!(why.contains("does not name commit") && why.contains("directly as refs/tags/"), "{name}: {why}");
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
    assert!(why.contains("authored-setup.md is not docs/setup.md in the tagged tree"), "{why}");
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
}

fn image_pin(root: &Path, k: &ImageKnobs) -> (PathBuf, Value) {
    let t = TargetKind::AlkemioMailslurper;
    let evals = evals_copy(root);
    let sd = evals.join("sources").join(t.name());
    std::fs::create_dir_all(&sd).unwrap();
    let layer = "c".repeat(64);
    let listed = if k.manifest_omits_layer { "d".repeat(64) } else { layer.clone() };
    let manifest = json!({"schemaVersion": 2, "layers": [{"digest": format!("sha256:{listed}")}]});
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let m = sha(&manifest_bytes);
    let arch = if k.index_names_other_platform { "arm64" } else { "amd64" };
    let index = json!({"schemaVersion": 2, "manifests": [
        {"digest": format!("sha256:{m}"), "platform": {"os": "linux", "architecture": arch}},
        {"digest": format!("sha256:{}", "e".repeat(64)), "platform": {"os": "linux", "architecture": "s390x"}}]});
    let index_bytes = serde_json::to_vec(&index).unwrap();
    let i = sha(&index_bytes);
    let reference = protocol::ALKEMIO_IMAGE;
    let attr = json!({"from": "image", "reference": reference});
    let mut docs = vec![capture(&sd, "alkemio-index.json", "image_index", &index_bytes, attr.clone())];
    if !k.omit_manifest_doc {
        docs.push(capture(&sd, "alkemio-manifest.json", "image_manifest", &manifest_bytes, attr.clone()));
    }
    docs.push(capture(&sd, "alkemio-license.txt", "license", b"MIT License\n\nauthored\n", attr.clone()));
    docs.push(capture(&sd, "alkemio-api.md", "api", b"# authored api\n", attr));
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
        "launch": ["-config", "{config_path}", "--max", "10000"],
        "storage": {"backend": "scratch_file", "max_messages": 10000, "cap_flag": "--max", "basis": "authored"},
        "wait": {"reviewed": [], "finding": "an image has no tagged tree", "best_wait": {"unresolved": "none"}},
        "listeners": ["smtp", "http", "ui"],
        "config_file": {"name": "config.json", "lines": ["{{", "\"p\": {smtp_port} {http_port} {ui_port}", "}}"]},
    });
    (evals, json!({"target": t.name(), "role": "competitor", "pin": pin}))
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
    lock.verify_files(&dir).expect("index -> manifest -> layer is content addressed");
    let pin = lock.pin(TargetKind::AlkemioMailslurper).unwrap();
    assert!(pin.release.is_none() && pin.image.is_some());
    assert!(pin.tagged_source.gap().is_some() && pin.binary_build_provenance.gap().is_some());
}

#[test]
fn image_pin_refuses_each_digest_mismatch_and_missing_manifest() {
    let cases: Vec<(&str, ImageKnobs, &str)> = vec![
        ("index digest", ImageKnobs { wrong_index_digest: true, ..Default::default() }, "no captured ImageIndex document with digest sha256:"),
        ("manifest digest", ImageKnobs { wrong_manifest_digest: true, ..Default::default() }, "no captured ImageManifest document with digest sha256:"),
        ("platform", ImageKnobs { index_names_other_platform: true, ..Default::default() }, "does not name manifest"),
        ("layer", ImageKnobs { manifest_omits_layer: true, ..Default::default() }, "does not name layer"),
    ];
    for (name, k, want) in cases {
        let (_r, dir, lock) = image_authored(&k);
        let why = match lock.validate() {
            Err(e) => e.0.join("\n"),
            Ok(()) => refusals(lock.verify_files(&dir)),
        };
        assert!(why.contains(want), "{name}: {why}");
    }
    let (_r, _d, lock) = image_authored(&ImageKnobs { omit_manifest_doc: true, ..Default::default() });
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
        assert!(why.contains(&format!("{name} does not match its digest")), "{name}: {why}");
    }
}

#[test]
fn image_pin_refuses_observed_tagged_source_or_build_provenance() {
    let (_r, _d, lock) = image_authored(&ImageKnobs { observed_tagged_source: true, ..Default::default() });
    assert!(refusals(lock.validate()).contains("an image pin has no tagged source to observe"));
    let (_r, _d, lock) = image_authored(&ImageKnobs { observed_build_provenance: true, ..Default::default() });
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
    lock.validate().expect("a greenmail pin with its JRE validates");
    lock.verify_files(&dir).expect("and verifies");
    // The runtime's own license must be on the runtime list.
    for bad in ["MIT", "GPL-3.0-only", "GPL-2.0-only", ""] {
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
    assert!(why.contains(&format!("license \"{jre}\" is not on the permissive list")), "{why}");
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
    entry["pin"]["launch"].as_array_mut().unwrap().push("{artifact}".into());
    let why = refusals(lock_with(TargetKind::Mailslurper, entry).validate());
    assert!(why.contains("only greenmail pins a runtime"), "{why}");
    // And greenmail without one, or with its checked members short.
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
    entry["pin"].as_object_mut().unwrap().remove("runtime");
    assert!(refusals(lock_with(TargetKind::Greenmail, entry).validate()).contains("needs a pinned runtime"));
    for drop in ["bin/java", "lib/server/libjvm.so"] {
        let root = tempfile::tempdir().unwrap();
        let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
        let members = entry["pin"]["runtime"]["checked_members"].as_array_mut().unwrap();
        members.retain(|m| m["path"] != drop);
        let why = refusals(lock_with(TargetKind::Greenmail, entry).validate());
        assert!(why.contains(&format!("the runtime does not check {drop}")), "{why}");
    }
    // More than eight members, a repeated member and an unconfined path.
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Greenmail));
    let members = entry["pin"]["runtime"]["checked_members"].as_array_mut().unwrap();
    for n in 0..7 {
        members.push(json!({"path": format!("lib/m{n}.so"), "sha256": "1".repeat(64)}));
    }
    members.push(json!({"path": "bin/java", "sha256": "1".repeat(64)}));
    members.push(json!({"path": "../escape", "sha256": "1".repeat(64)}));
    let why = refusals(lock_with(TargetKind::Greenmail, entry).validate());
    assert!(why.contains("more than 8") && why.contains("listed twice") && why.contains("is malformed"), "{why}");
}

// ---- listeners, environment and config -----------------------------------

fn mailslurper_pin_with(edit: impl Fn(&mut Value)) -> String {
    let root = tempfile::tempdir().unwrap();
    let (_d, mut entry) = release_pin(root.path(), &Knobs::new(TargetKind::Mailslurper));
    edit(&mut entry["pin"]);
    match lock_from(json!({"lock_version": 4, "checked_on": "2026-11-07", "entries": [entry]})) {
        Ok(lock) => lock.validate().err().map(|r| r.0.join("\n")).unwrap_or_default(),
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
    assert!(why.contains("unknown variant"), "a role outside the closed set never parses: {why}");
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["http", "ui"]));
    assert!(why.contains("lack the smtp role"), "{why}");
    let why = mailslurper_pin_with(|p| p["listeners"] = json!(["smtp", "ui"]));
    assert!(why.contains("lack the http role its read path uses"), "{why}");
    // An undeclared slot, and a declared listener the launch never gets.
    let why = mailslurper_pin_with(|p| {
        p["config_file"]["lines"].as_array_mut().unwrap().push("{imap_port}".into())
    });
    assert!(why.contains("names a slot the declared listeners"), "{why}");
    let why = mailslurper_pin_with(|p| {
        p["listeners"] = json!(["smtp", "http", "ui", "imap"])
    });
    assert!(why.contains("the declared imap listener is never given to the launch"), "{why}");
    let why = mailslurper_pin_with(|p| p["launch"].as_array_mut().unwrap().push("{nope}".into()));
    assert!(why.contains("names a slot"), "{why}");
}

#[test]
fn launch_env_refuses_owned_and_malformed_names() {
    for name in ["PATH", "HOME", "TMPDIR", "LANG", "XDG_CONFIG_HOME", "XDG_X"] {
        let why = mailslurper_pin_with(|p| p["launch_env"] = json!([[name, "x"]]));
        assert!(why.contains(&format!("environment name {name} is owned by the launcher")), "{name}: {why}");
    }
    for name in ["lower", "A-B", "1A", "", "A B", &"A".repeat(65)] {
        let why = mailslurper_pin_with(|p| p["launch_env"] = json!([[name, "x"]]));
        assert!(why.contains("is not upper snake case"), "{name:?}: {why}");
    }
    let why = mailslurper_pin_with(|p| p["launch_env"] = json!([["A_B", "x"], ["A_B", "y"]]));
    assert!(why.contains("is listed twice"), "{why}");
    let why = mailslurper_pin_with(|p| p["launch_env"] = json!([["INBUCKET_SMTP_ADDR", "{smtp_addr}"]]));
    assert!(!why.contains("environment"), "an upper-snake name with a declared slot is accepted: {why}");
    let why = mailslurper_pin_with(|p| p["launch_env"] = json!([["X", "{pop3_addr}"]]));
    assert!(why.contains("names a slot"), "{why}");
}

#[test]
fn config_file_name_confined() {
    for name in ["a/b", "..", "../x", "", "/etc/passwd", "a..b"] {
        let why = mailslurper_pin_with(|p| p["config_file"]["name"] = name.into());
        assert!(why.contains("must be one plain leaf name"), "{name:?}: {why}");
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
    let lock = lock4(v4());
    lock.validate().expect("pending entries are valid structure");
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    p.validate().expect("the draft protocol is valid");
    for t in NEW_TARGETS {
        let e = lock.entry(t).unwrap();
        assert!(e.pin.is_none() && e.pending.is_some(), "{t}");
        assert!(lock.pin(t).is_none());
    }
    // `verify_files` has no bytes to check for them and is not a gate.
    lock.verify_files(&evals_dir()).expect("pending entries have no bytes");
    // Collection and launch both refuse each one by name.
    let mut frozen = p.clone();
    frozen.status = protocol::Status::Frozen;
    frozen.unresolved.clear();
    let why = refusals(protocol::collection_ready(&frozen, &lock));
    let launch = refusals(protocol::launchable(&p, &lock));
    for t in NEW_TARGETS {
        assert!(why.contains(&format!("{t}: pending, no verified pin")), "{t}: {why}");
        assert!(launch.contains(&format!("{t} has no source pin; it cannot be launched (pending: ")), "{t}: {launch}");
    }
    // Both and neither are refused.
    let mut l = v4();
    let at = entry_of(&mut l, "mailslurper");
    let root = tempfile::tempdir().unwrap();
    let (_dir, pinned) = release_pin(root.path(), &Knobs::new(TargetKind::Mailslurper));
    l["entries"][at]["pin"] = pinned["pin"].clone();
    assert!(refusals(lock4(l).validate()).contains("a pin or pending, never both"));
    let mut l = v4();
    let at = entry_of(&mut l, "mailslurper");
    l["entries"][at].as_object_mut().unwrap().remove("pending");
    assert!(refusals(lock4(l).validate()).contains("mailslurper: a competitor needs a pin"));
    // A pending entry states what kind of source it is and why.
    let mut l = v4();
    let at = entry_of(&mut l, "smtp4dev");
    l["entries"][at]["pending"]["release_tag"] = "latest".into();
    l["entries"][at]["pending"]["reason"] = " ".into();
    let why = refusals(lock4(l).validate());
    assert!(why.contains("is not a version tag") && why.contains("states no reason"), "{why}");
    let mut l = v4();
    let at = entry_of(&mut l, "alkemio-mailslurper");
    l["entries"][at]["pending"]["image"] = "ghcr.io/other/x:1".into();
    assert!(refusals(lock4(l).validate()).contains("is not the labelled"));
}

// ---- protocol 3 -----------------------------------------------------------

fn p3() -> Value {
    json(PROTOCOL_V3)
}

fn v3_refusals(f: impl Fn(&mut Value)) -> String {
    let mut v = p3();
    f(&mut v);
    match protocol_from(v) {
        Ok(p) => p.validate().err().map(|r| r.0.join("\n")).unwrap_or_default(),
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
        assert_eq!(p.storage.retention.iter().filter(|r| r.target == t).count(), 1, "{t}");
        assert_eq!(p.products.iter().flatten().filter(|x| x.target == t).count(), 1, "{t}");
        assert_eq!(p.capabilities.iter().flatten().filter(|x| x.target == t).count(), 1, "{t}");
        for w in &p.waits {
            assert_eq!(w.observation.iter().filter(|o| o.target == t).count(), 1, "{t}");
        }
    }
    // The throughput and wait profiles, corpus, budgets and resources are
    // the history v2 bytes, field by field.
    let (old, new) = (json(PROTOCOL), p3());
    for key in ["corpus", "throughput", "budgets", "resources"] {
        assert_eq!(old[key], new[key], "{key}");
    }
    assert_eq!(old["statistics"]["quantile_method"], new["statistics"]["quantile_method"]);
    assert_eq!(old["statistics"]["quantiles"], new["statistics"]["quantiles"]);
    let (ow, nw) = (&old["waits"][0], &new["waits"][0]);
    for key in ["name", "repetitions", "present", "future", "empty", "deadline_ms", "empty_deadline_ms"] {
        assert_eq!(ow[key], nw[key], "wait {key}");
    }
    for i in 0..3 {
        assert_eq!(ow["observation"][i], nw["observation"][i], "observation {i}");
    }
    // The orders are the eight cyclic rotations of the target list.
    let orders = p.statistics.orders.clone();
    assert_eq!(orders.len(), 8);
    for (k, order) in orders.iter().enumerate() {
        let want: Vec<TargetKind> = (0..8).map(|i| p.targets[(i + k) % 8]).collect();
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
            v["targets"].as_array_mut().unwrap().retain(|x| x != t.name());
        });
        assert!(why.contains(&format!("protocol_version 3 needs target {t}")), "{t}: {why}");
    }
    let why = v3_refusals(|v| {
        v["capabilities"].as_array_mut().unwrap().retain(|c| c["target"] != "inbucket");
    });
    assert!(why.contains("capabilities: inbucket needs exactly one record"), "{why}");
    let why = v3_refusals(|v| {
        v["capabilities"][0].as_object_mut().unwrap().remove("native_wait");
    });
    assert!(why.contains("native_wait"), "a missing capability key never parses: {why}");
    let why = v3_refusals(|v| {
        v["products"][4].as_object_mut().unwrap().remove("scope");
    });
    assert!(why.contains("products: smtp4dev states no scope"), "{why}");
    let why = v3_refusals(|v| {
        v.as_object_mut().unwrap().remove("capabilities");
    });
    assert!(why.contains("capabilities: swarmail needs exactly one record"), "{why}");
    let why = v3_refusals(|v| {
        v["storage"]["retention"].as_array_mut().unwrap().retain(|r| r["target"] != "greenmail");
    });
    assert!(why.contains("storage: greenmail needs exactly one retention entry"), "{why}");
    let why = v3_refusals(|v| {
        v["waits"][0]["observation"].as_array_mut().unwrap().retain(|o| o["target"] != "smtp4dev");
    });
    assert!(why.contains("smtp4dev needs exactly one observation mode"), "{why}");
    let why = v3_refusals(|v| {
        v["statistics"]["orders"].as_array_mut().unwrap().pop();
    });
    assert!(why.contains("not the 8 cyclic rotations") && why.contains("rotation 7 is missing"), "{why}");
}

#[test]
fn capability_without_or_with_foreign_citation_refused() {
    let why = v3_refusals(|v| v["capabilities"][1]["server_search"]["cite"] = Value::Null);
    assert!(why.contains("mailpit server_search is Supported without a citation"), "{why}");
    let why = v3_refusals(|v| {
        v["capabilities"][1]["native_wait"] = json!({"status": "not_supported", "cite": ""});
    });
    assert!(why.contains("mailpit native_wait is NotSupported without a citation"), "{why}");
    let why = v3_refusals(|v| {
        v["capabilities"][3]["server_search"] = json!({"status": "not_inspected", "cite": "x"});
    });
    assert!(why.contains("mailslurper server_search is not_inspected, so it cites nothing"), "{why}");
    // A citation names a document or reviewed source of that target's pin.
    let p = |f: &dyn Fn(&mut Value)| {
        let mut v = p3();
        f(&mut v);
        protocol_from(v).unwrap()
    };
    let lock = lock4(v4());
    protocol::capabilities_match(&p(&|_| {}), &lock).expect("the committed citations all resolve");
    let foreign = p(&|v| {
        v["capabilities"][2]["server_search"]["cite"] = "sources/mailpit/mailpit-swagger.json".into();
    });
    let why = refusals(protocol::capabilities_match(&foreign, &lock));
    assert!(why.contains("mailhog server_search cites \"sources/mailpit/mailpit-swagger.json\", which is no document"), "{why}");
    let pending = p(&|v| {
        v["capabilities"][6]["raw_source"] = json!({"status": "supported", "cite": "sources/greenmail/x.md"});
    });
    let why = refusals(protocol::capabilities_match(&pending, &lock));
    assert!(why.contains("greenmail raw_source cites") && why.contains("has no pin to cite"), "{why}");
    let repo = p(&|v| {
        v["capabilities"][0]["native_wait"]["cite"] = "docs/decisions/0002-awaits-subscribe-before-checking.md".into();
    });
    assert!(refusals(protocol::capabilities_match(&repo, &lock)).contains("not a repo: path"));
    // A stated cap agrees with the flag the launch sets.
    let capped = p(&|v| {
        v["capabilities"][2]["retention_cap"] = json!({"status": "supported", "cite": "sources/mailhog/mailhog-readme.md"});
    });
    assert!(refusals(protocol::capabilities_match(&capped, &lock)).contains("mailhog retention_cap is supported but its launch sets no cap"));
}

#[test]
fn product_scope_must_match_provenance() {
    let why = v3_refusals(|v| v["products"][7]["scope"] = "official_artifact".into());
    assert!(why.contains("alkemio-mailslurper scope OfficialArtifact is not its provenance's ThirdPartyImage"), "{why}");
    let why = v3_refusals(|v| v["products"][1]["scope"] = "third_party_image".into());
    assert!(why.contains("mailpit scope ThirdPartyImage is not its provenance's OfficialArtifact"), "{why}");
    let why = v3_refusals(|v| v["products"][0]["scope"] = "official_artifact".into());
    assert!(why.contains("swarmail scope OfficialArtifact is not its provenance's Candidate"), "{why}");
    let why = v3_refusals(|v| v["products"][7]["release_tag"] = "v1.0.0".into());
    assert!(why.contains("alkemio-mailslurper is a third-party image: it has no release tag"), "{why}");
    // The lock's pin source must agree with the scope, once pinned.
    let root = tempfile::tempdir().unwrap();
    let (_dir, entry) = image_pin(root.path(), &ImageKnobs::default());
    let lock = lock_with(TargetKind::AlkemioMailslurper, entry);
    let mut v = p3();
    v["products"][7]["executable_sha256"] = "5".repeat(64).into();
    let ok = protocol_from(v.clone()).unwrap();
    protocol::products_match(&ok, &lock).expect("an image pin matches its image scope");
    v["products"][7]["scope"] = "official_artifact".into();
    let why = refusals(protocol::products_match(&protocol_from(v).unwrap(), &lock));
    assert!(why.contains("protocol scope Some(OfficialArtifact) is not what the lock's pin is"), "{why}");
}

#[test]
fn v3_reusing_v2_id_refused() {
    for id in ["fair-mail-comparison-v2", "fair-mail-comparison-draft-3", "fair-mail-comparison-draft-2"] {
        let why = v3_refusals(|v| v["id"] = id.into());
        assert!(why.contains("reuses a historical protocol id"), "{id}: {why}");
    }
    let why = v3_refusals(|v| v["supersedes"] = "fair-mail-comparison-draft-3".into());
    assert!(why.contains("protocol 3 supersedes fair-mail-comparison-v2"), "{why}");
    let why = v3_refusals(|v| v["change_reason"] = "wider".into());
    assert!(why.contains("must cite decision 0130"), "{why}");
}

#[test]
fn v3_cap_below_workload_refused() {
    let why = v3_refusals(|v| {
        let at = v["storage"]["retention"].as_array().unwrap().iter().position(|r| r["target"] == "smtp4dev").unwrap();
        v["storage"]["retention"][at]["max_messages"] = 100.into();
    });
    assert!(why.contains("3100 messages overflow smtp4dev's retention"), "{why}");
    let why = v3_refusals(|v| {
        let at = v["storage"]["retention"].as_array().unwrap().iter().position(|r| r["target"] == "inbucket").unwrap();
        v["storage"]["retention"][at]["max_messages"] = 3000.into();
    });
    assert!(why.contains("3100 messages overflow inbucket's retention"), "{why}");
}

#[test]
fn v2_protocol_refuses_v3_sections() {
    let mut v = json(PROTOCOL);
    v["capabilities"] = p3()["capabilities"].clone();
    let why = refusals(protocol_from(v).unwrap().validate());
    assert!(why.contains("protocol_version 2 carries version 3 sections"), "{why}");
    let mut v = json(PROTOCOL);
    v["products"][1]["scope"] = "official_artifact".into();
    let why = refusals(protocol_from(v).unwrap().validate());
    assert!(why.contains("carries version 3 sections"), "{why}");
}

#[test]
fn frozen_only_when_nothing_unresolved_and_no_pending() {
    let lock = lock4(v4());
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    // The committed protocol is a draft and says what it lacks.
    assert_eq!(p.status, protocol::Status::Draft);
    assert!(!p.unresolved.is_empty());
    let why = refusals(protocol::collection_ready(&p, &lock));
    assert!(why.contains("protocol is a draft, not frozen"), "{why}");
    assert!(why.contains("unresolved: mailslurper: no verified pin"), "{why}");
    for t in NEW_TARGETS {
        assert!(why.contains(&format!("{t}: pending, no verified pin")), "{t}: {why}");
    }
    // Freezing it with entries still unresolved is itself refused.
    let why = v3_refusals(|v| v["status"] = "frozen".into());
    assert!(why.contains("a frozen protocol has nothing unresolved"), "{why}");
    // Emptying `unresolved` alone does not admit it: the pending rows do.
    let mut frozen = p.clone();
    frozen.status = protocol::Status::Frozen;
    frozen.unresolved.clear();
    frozen.validate().expect("structurally a frozen v3 can stand");
    let why = refusals(protocol::collection_ready(&frozen, &lock));
    assert!(why.contains("pending, no verified pin"), "{why}");
    assert!(!why.contains("a frozen protocol has nothing unresolved"));
    // The unenforced profile and unmatched storage stay labelled.
    assert_eq!(p.storage.profile, Some(protocol::StorageProfile::Unmatched));
    assert_eq!(p.resources.profile, Some(protocol::ResourceProfile::UnenforcedLocal));
}

#[test]
fn current_lock_v4_validates_and_verifies() {
    let lock = lock4(v4());
    lock.validate().expect("the committed lock validates");
    lock.verify_files(&evals_dir()).expect("and its captured documents verify");
    assert_eq!(lock.lock_version, 4);
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    protocol::pairing(&p, &lock).unwrap();
    protocol::retention_matches(&p, &lock).expect("retention is as launched where a pin exists");
    protocol::products_match(&p, &lock).expect("products are the locked executables");
    protocol::capabilities_match(&p, &lock).expect("citations resolve");
    // The new capture directories do not exist for rows with no bytes, and
    // no document of an official row moved.
    for t in NEW_TARGETS {
        assert!(!evals_dir().join("sources").join(t.name()).exists(), "{t}");
    }
}

#[test]
fn declared_competitors_come_from_the_protocol() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let names: Vec<&str> = protocol::declared_competitors(&p).iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        ["mailpit", "mailhog", "mailslurper", "smtp4dev", "inbucket", "greenmail", "alkemio-mailslurper"]
    );
    let old = Protocol::parse(PROTOCOL).unwrap();
    assert_eq!(protocol::declared_competitors(&old), [TargetKind::Mailpit, TargetKind::Mailhog]);
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
        why.contains("fair-mail-comparison-v2 and fair-mail-comparison-v3") && why.contains("separate denominators"),
        "{why}"
    );
}

#[test]
fn row_labels_present_for_each_target() {
    let p = Protocol::parse(PROTOCOL_V3).unwrap();
    let labels = report::row_labels(&p);
    assert_eq!(labels.len(), 8);
    let of = |t: TargetKind| labels.iter().find(|l| l.target == t).unwrap();
    assert_eq!(of(TargetKind::AlkemioMailslurper).source, "third-party image (extracted binary)");
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
    assert_ne!(of(TargetKind::Mailslurper).source, of(TargetKind::AlkemioMailslurper).source);
}
