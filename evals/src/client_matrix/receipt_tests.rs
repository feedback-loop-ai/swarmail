//! The receipt contract's portable proofs (issue #392): every case runs on
//! a temporary copy through the same scanner the checked-in directory and
//! the runner's drafts use. File-only: no subprocess, socket or wait.

use super::receipt::*;
use serde_json::{Value, json};

// ---------------------------------------------------------------- fixtures

const NAME: &str = "2026-11-25-claude-code.json";

fn steps_with(fail: &[&str]) -> Value {
    let mut steps = serde_json::Map::new();
    for step in STEPS {
        let outcome = if fail.contains(&step) {
            json!({"status": "fail", "reason": format!("{step} not observed")})
        } else {
            json!({"status": "pass"})
        };
        steps.insert(step.into(), outcome);
    }
    Value::Object(steps)
}

/// Every step passes on 2025-11-25, and the reply carries an extension
/// member, which survives.
fn modern_pass() -> Value {
    json!({
        "date": "2026-11-25",
        "client": "claude-code",
        "client_version": "synthetic 1.0",
        "os": "synthetic OS",
        "swarmail_commit": "0123456789abcdef0123456789abcdef01234567",
        "executable_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "transport": "http",
        "negotiated_revision": "2025-11-25",
        "initialize_response": {
            "jsonrpc": "2.0",
            "id": 0,
            "result": {
                "protocolVersion": "2025-11-25",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "swarmail", "version": "0.5.0"},
                "_meta": {"x-extension": true}
            }
        },
        "steps": steps_with(&[]),
        "recorded_by": "operator",
        "mode": "interactive"
    })
}

/// March: initialize passes, structured output legitimately fails.
fn march_partial() -> Value {
    let mut receipt = modern_pass();
    receipt["negotiated_revision"] = json!("2025-03-26");
    receipt["initialize_response"]["result"]["protocolVersion"] = json!("2025-03-26");
    receipt["steps"] = steps_with(&["structured_content"]);
    receipt["transport"] = json!("stdio");
    receipt["client"] = json!("claude-desktop");
    receipt
}

/// A captured initialize error: no revision, every step failed.
fn initialize_error() -> Value {
    let mut receipt = modern_pass();
    receipt["negotiated_revision"] = Value::Null;
    receipt["initialize_response"] =
        json!({"jsonrpc": "2.0", "id": 0, "error": {"code": -32602, "message": "bad"}});
    receipt["steps"] = steps_with(&STEPS);
    receipt["recorded_by"] = json!("dispatcher");
    receipt["mode"] = json!("headless");
    receipt
}

/// No reply was captured at all: both nullable keys present as `null`.
fn missing_reply() -> Value {
    let mut receipt = initialize_error();
    receipt["initialize_response"] = Value::Null;
    receipt
}

/// The verdict of a fresh directory holding exactly one file.
fn verdict(name: &str, bytes: &[u8]) -> Result<String, String> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(name), bytes).unwrap();
    scan(dir.path())
}

fn verdict_of(name: &str, receipt: &Value) -> Result<String, String> {
    verdict(
        name,
        serde_json::to_string_pretty(receipt).unwrap().as_bytes(),
    )
}

fn name_of(receipt: &Value) -> String {
    format!(
        "{}-{}.json",
        receipt["date"].as_str().unwrap(),
        receipt["client"].as_str().unwrap()
    )
}

#[track_caller]
fn refused(result: Result<String, String>, needle: &str) {
    match result {
        Ok(status) => panic!("accepted ({status}); expected a refusal naming {needle:?}"),
        Err(error) => assert!(
            error.contains(needle),
            "refusal {error:?} does not name {needle:?}"
        ),
    }
}

#[track_caller]
fn refused_value(receipt: &Value, needle: &str) {
    refused(verdict_of(NAME, receipt), needle);
}

/// `receipt` with the key at `pointer` removed.
fn without(receipt: &Value, pointer: &str) -> Value {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let mut copy = receipt.clone();
    let removed = copy
        .pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .and_then(|object| object.remove(key));
    assert!(removed.is_some(), "{pointer} is absent from the fixture");
    copy
}

fn with(receipt: &Value, pointer: &str, value: Value) -> Value {
    let mut copy = receipt.clone();
    *copy
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer}")) = value;
    copy
}

/// Insert `extra` right after the one occurrence of `anchor`, at the byte
/// level, so a duplicate key reaches the decoder.
fn spliced(receipt: &Value, anchor: &str, extra: &str) -> Vec<u8> {
    let text = serde_json::to_string(receipt).unwrap();
    assert_eq!(text.matches(anchor).count(), 1, "{anchor} must occur once");
    text.replacen(anchor, &format!("{anchor}{extra}"), 1)
        .into_bytes()
}

// ---------------------------------------------------------------- the scan

#[test]
fn receipt_an_empty_directory_passes_with_no_receipts_yet() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(scan(dir.path()).unwrap(), EMPTY);
    std::fs::write(dir.path().join(README), "# receipts\n").unwrap();
    assert_eq!(scan(dir.path()).unwrap(), EMPTY);
}

#[test]
fn receipt_a_missing_directory_or_a_foreign_entry_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    refused(scan(&dir.path().join("absent")), "cannot enumerate");

    let sub = tempfile::tempdir().unwrap();
    std::fs::create_dir(sub.path().join(NAME)).unwrap();
    refused(scan(sub.path()), "not a regular file");

    for name in [
        "notes.txt",
        "2026-11-25-claude-code.JSON",
        "2026-11-25-cursor.json",
        "2026-11-25claude-code.json",
        "2026-11-2-claude-code.json",
        "26-11-25-claude-code.json",
        "readme.md",
    ] {
        refused(verdict(name, b"{}"), "not a receipt name");
    }
    refused(
        verdict("2026-02-30-codex.json", b"{}"),
        "not a real YYYY-MM-DD date",
    );
    refused(verdict(NAME, b"{not json"), NAME);
    refused(verdict(NAME, b"\xff\xfe"), "not UTF-8");
}

#[cfg(unix)]
#[test]
fn receipt_a_symlinked_receipt_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.json");
    std::fs::write(&target, serde_json::to_vec(&modern_pass()).unwrap()).unwrap();
    let receipts = dir.path().join("receipts");
    std::fs::create_dir(&receipts).unwrap();
    std::os::unix::fs::symlink(&target, receipts.join(NAME)).unwrap();
    refused(scan(&receipts), "not a regular file");
}

#[test]
fn receipt_the_four_evidence_states_are_accepted() {
    for receipt in [
        modern_pass(),
        march_partial(),
        initialize_error(),
        missing_reply(),
    ] {
        let name = name_of(&receipt);
        let status = verdict_of(&name, &receipt).unwrap_or_else(|e| panic!("{e}"));
        assert!(status.starts_with("1 receipt(s)"), "{status}");
        assert!(status.ends_with("no compatibility verdict"), "{status}");
    }
    // Several receipts, one per client, in one directory.
    let dir = tempfile::tempdir().unwrap();
    for (client, receipt) in CLIENTS.iter().zip([
        modern_pass(),
        initialize_error(),
        missing_reply(),
        march_partial(),
    ]) {
        let receipt = with(&receipt, "/client", json!(client));
        std::fs::write(
            dir.path().join(name_of(&receipt)),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
    }
    assert!(scan(dir.path()).unwrap().starts_with("4 receipt(s)"));
}

#[test]
fn receipt_removing_any_required_field_is_refused() {
    let pass = modern_pass();
    for field in pass.as_object().unwrap().keys() {
        refused_value(
            &without(&pass, &format!("/{field}")),
            &format!("missing field `{field}`"),
        );
    }
    for step in STEPS {
        refused_value(
            &without(&pass, &format!("/steps/{step}")),
            &format!("missing field `{step}`"),
        );
        refused_value(
            &without(&pass, &format!("/steps/{step}/status")),
            "missing field `status`",
        );
    }
    // Every conditional reason, on a receipt where every step failed.
    let failed = missing_reply();
    for step in STEPS {
        refused_value(
            &without(&failed, &format!("/steps/{step}/reason")),
            &format!("steps.{step}: a fail needs a nonblank reason"),
        );
    }
    // The handshake evidence a passing initialize needs.
    for (pointer, needle) in [
        ("/initialize_response/jsonrpc", "jsonrpc is not \"2.0\""),
        ("/initialize_response/id", "missing id"),
        (
            "/initialize_response/result",
            "exactly one of result and error",
        ),
        (
            "/initialize_response/result/protocolVersion",
            "does not match",
        ),
    ] {
        refused_value(&without(&pass, pointer), needle);
    }
}

#[test]
fn receipt_unknown_and_duplicate_fields_are_refused() {
    let pass = modern_pass();
    let mut extra = pass.clone();
    extra["verdict"] = json!("compatible");
    refused_value(&extra, "unknown field `verdict`");
    refused_value(
        &with_or_insert(&pass, "/steps/tools_call", json!({"status": "pass"})),
        "unknown field `tools_call`",
    );
    refused_value(
        &with_or_insert(&pass, "/steps/read/note", json!("x")),
        "unknown field `note`",
    );

    // serde_json sorts keys, so each anchor below is the start of an object.
    for (anchor, extra, field) in [
        (
            r#""client":"claude-code","#,
            r#""client":"claude-code","#,
            "client",
        ),
        (r#""steps":{"#, r#""read":{"status":"pass"},"#, "read"),
        (r#""initialize":{"#, r#""status":"fail","#, "status"),
    ] {
        refused(
            verdict(NAME, &spliced(&pass, anchor, extra)),
            &format!("duplicate field `{field}`"),
        );
    }
    let failed = missing_reply();
    refused(
        verdict(
            NAME,
            &spliced(&failed, r#""read":{"#, r#""reason":"once","#),
        ),
        "duplicate field `reason`",
    );
}

#[test]
fn receipt_wrong_types_enums_and_blanks_are_refused() {
    let pass = modern_pass();
    for (pointer, value, needle) in [
        ("/client", json!("cursor"), "unknown variant `cursor`"),
        ("/transport", json!("sse"), "unknown variant `sse`"),
        ("/recorded_by", json!("ci"), "unknown variant `ci`"),
        ("/mode", json!("scripted"), "unknown variant `scripted`"),
        (
            "/steps/read/status",
            json!("skipped"),
            "unknown variant `skipped`",
        ),
        ("/client_version", json!(1), "invalid type"),
        ("/os", Value::Null, "invalid type"),
        ("/date", json!(20261125), "invalid type"),
        ("/steps", json!("all"), "invalid type"),
        ("/steps/read", json!("pass"), "invalid type"),
        ("/negotiated_revision", json!(20251125), "invalid type"),
        ("/client_version", json!("  "), "client_version: blank"),
        ("/os", json!(""), "os: blank"),
        (
            "/steps/read/reason",
            json!("why"),
            "steps.read: a pass has no reason",
        ),
        ("/steps/read/reason", Value::Null, "invalid type: null"),
    ] {
        refused_value(&with_or_insert(&pass, pointer, value), needle);
    }
    let failed = missing_reply();
    refused_value(
        &with(&failed, "/steps/read/reason", json!(" ")),
        "steps.read: a fail needs a nonblank reason",
    );
    refused_value(
        &with(&failed, "/steps/read/reason", json!(3)),
        "invalid type",
    );

    // The array form serde would otherwise decode as a struct.
    let in_order = Value::Array(FIELDS.iter().map(|key| pass[key].clone()).collect());
    refused(
        verdict(NAME, &serde_json::to_vec(&in_order).unwrap()),
        "the receipt is not a JSON object",
    );
    let mut array_steps = pass.clone();
    array_steps["steps"] = Value::Array(STEPS.iter().map(|s| pass["steps"][s].clone()).collect());
    refused_value(&array_steps, "steps: not a JSON object");
    refused_value(
        &with(&pass, "/steps/read", json!(["pass"])),
        "steps.read: not a JSON object",
    );
}

/// Set `pointer`, creating its last key when absent.
fn with_or_insert(receipt: &Value, pointer: &str, value: Value) -> Value {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let mut copy = receipt.clone();
    copy.pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .unwrap()
        .insert(key.into(), value);
    copy
}

#[test]
fn receipt_bad_hashes_dates_and_names_are_refused() {
    let pass = modern_pass();
    let commit = pass["swarmail_commit"].as_str().unwrap().to_owned();
    let sha = pass["executable_sha256"].as_str().unwrap().to_owned();
    for (field, good, digits) in [
        ("swarmail_commit", &commit, 40),
        ("executable_sha256", &sha, 64),
    ] {
        let needle = format!("{field}: not {digits} lowercase hex digits");
        for bad in [
            format!("{}g", &good[1..]),
            good.to_uppercase(),
            good[1..].to_owned(),
            format!("{good}0"),
            String::new(),
            format!("{} ", &good[1..]),
        ] {
            refused_value(&with(&pass, &format!("/{field}"), json!(bad)), &needle);
        }
    }
    for bad in [
        "2026-02-30",
        "2026-13-01",
        "2026-1-05",
        "+2026-11-25",
        "2026/11/25",
    ] {
        refused_value(
            &with(&pass, "/date", json!(bad)),
            "is not a real YYYY-MM-DD date",
        );
    }
    refused_value(
        &with(&pass, "/date", json!("2026-11-24")),
        "does not match the file name",
    );
    refused_value(
        &with(&pass, "/client", json!("codex")),
        "does not match the file name",
    );
    refused(
        verdict_of("2026-11-25-codex.json", &pass),
        "does not match the file name",
    );
}

#[test]
fn receipt_contradictory_handshake_evidence_is_refused() {
    let pass = modern_pass();
    let failed = initialize_error();
    for (receipt, needle) in [
        (
            with(&pass, "/negotiated_revision", json!("2025-06-18")),
            "does not match",
        ),
        (
            with(&pass, "/negotiated_revision", Value::Null),
            "does not match",
        ),
        (
            with(
                &missing_reply(),
                "/negotiated_revision",
                json!("2025-11-25"),
            ),
            "does not match",
        ),
        (
            with(&failed, "/negotiated_revision", json!("2025-11-25")),
            "does not match",
        ),
        (
            with(
                &with(&pass, "/initialize_response", Value::Null),
                "/negotiated_revision",
                Value::Null,
            ),
            "a pass needs the captured initialize_response",
        ),
        (
            with(
                &with(&failed, "/steps/initialize", json!({"status": "pass"})),
                "/steps/list_tools",
                json!({"status": "fail", "reason": "x"}),
            ),
            "a pass needs a success response with result",
        ),
        (
            with(&pass, "/initialize_response/id", Value::Null),
            "non-null response id",
        ),
        (
            with(
                &with(
                    &pass,
                    "/initialize_response/result/protocolVersion",
                    json!(" "),
                ),
                "/negotiated_revision",
                json!(" "),
            ),
            "nonblank result.protocolVersion",
        ),
        (
            with(
                &without(&pass, "/initialize_response/result/protocolVersion"),
                "/negotiated_revision",
                Value::Null,
            ),
            "nonblank result.protocolVersion",
        ),
        (
            with(&pass, "/initialize_response", json!([])),
            "not a JSON object",
        ),
        (
            with(&pass, "/initialize_response/jsonrpc", json!("1.0")),
            "jsonrpc is not \"2.0\"",
        ),
        (
            with(&pass, "/initialize_response/result", json!("ok")),
            "result: not a JSON object",
        ),
        (
            with_or_insert(&pass, "/initialize_response/error", json!({"code": 1})),
            "exactly one of result and error",
        ),
        (
            with(&failed, "/steps/read", json!({"status": "pass"})),
            "steps.read: cannot pass after initialize failed",
        ),
        (
            with(
                &missing_reply(),
                "/steps/list_attachments",
                json!({"status": "pass"}),
            ),
            "steps.list_attachments: cannot pass",
        ),
    ] {
        refused_value(&receipt, needle);
    }
}


#[test]
fn receipt_mistyping_any_required_field_is_refused() {
    let pass = modern_pass();
    assert_eq!(
        pass.as_object().unwrap().keys().count(),
        FIELDS.len(),
        "the fixture carries exactly the twelve fields"
    );
    // One wrong JSON type per field; a nullable field refuses a number.
    for field in FIELDS {
        let wrong = match field {
            "steps" | "initialize_response" => json!(7),
            _ => json!(["x"]),
        };
        let refusal = verdict_of(NAME, &with(&pass, &format!("/{field}"), wrong));
        let refusal = refusal.expect_err(field);
        let named = refusal.contains("invalid type")
            || refusal.contains("not a JSON object")
            || refusal.contains("initialize_response");
        assert!(named, "{field}: {refusal}");
    }
    for step in STEPS {
        for (pointer, value) in [
            (format!("/steps/{step}"), json!(true)),
            (format!("/steps/{step}/status"), json!(1)),
        ] {
            refused_value(&with(&pass, &pointer, value), "invalid type");
        }
    }
    let failed = missing_reply();
    for step in STEPS {
        refused_value(
            &with(&failed, &format!("/steps/{step}/reason"), json!(false)),
            "invalid type",
        );
    }
}

#[test]
fn receipt_a_typed_draft_serializes_to_an_accepted_file() {
    for fixture in [modern_pass(), march_partial(), initialize_error(), missing_reply()] {
        let name = name_of(&fixture);
        let bytes = serde_json::to_vec(&fixture).unwrap();
        let receipt = validate(&bytes, &name).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(receipt.file_name(), name);
        let again = serde_json::to_vec_pretty(&receipt).unwrap();
        assert_eq!(validate(&again, &name).unwrap(), receipt);
        let reread: Value = serde_json::from_slice(&again).unwrap();
        assert_eq!(reread, fixture, "the typed form loses and adds nothing");
    }
    assert_eq!(Outcome::pass().reason, None);
    assert_eq!(Outcome::fail("x").status, Status::Fail);
    // validate_file refuses a path with no usable name and a missing file.
    refused(
        validate_file(std::path::Path::new("/")).map(|_| String::new()),
        "no UTF-8 file name",
    );
    let dir = tempfile::tempdir().unwrap();
    refused(
        validate_file(&dir.path().join(NAME)).map(|_| String::new()),
        "cannot stat",
    );
}
