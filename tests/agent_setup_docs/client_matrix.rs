//! The MCP client matrix's receipts (issue #392, epic #164): one typed
//! reader for `docs/evals/clients/`, run on the checked-in directory and on
//! temporary copies.
//!
//! The reader checks file shape only. A well-formed receipt, a passing
//! test, a fake client or an empty directory is never a compatibility
//! verdict (decisions 0079 and 0080): that verdict is the operator's, read
//! from receipts recorded through the named clients.
//!
//! Every test here is file-only: no subprocess, no socket, no wait.
//! Receipt-owned JSON is decoded straight from the bytes into strict types,
//! so serde refuses a duplicate key instead of a map keeping the last one.
//! The captured `initialize` reply is protocol-owned and stays a `Value`.

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use std::path::Path;

pub(crate) const MATRIX: &str = "docs/agents/client-matrix.md";
pub(crate) const RECEIPTS: &str = "docs/evals/clients";
/// The one file the scan skips.
const README: &str = "README.md";
/// The status of a successfully read, empty directory.
pub(crate) const EMPTY: &str = "no receipts yet";

/// The six checklist steps, in checklist order.
pub(crate) const STEPS: [&str; 6] = [
    "initialize",
    "list_tools",
    "read",
    "wait_for_code",
    "structured_content",
    "list_attachments",
];
/// The four tools the checklist calls.
pub(crate) const TOOLS: [&str; 4] = [
    "swarmail_get_latest_email",
    "swarmail_wait_for_code",
    "swarmail_search_emails",
    "swarmail_list_attachments",
];
/// The four client slugs, in the guide's order.
pub(crate) const CLIENTS: [&str; 4] = ["claude-code", "codex", "mcp-inspector", "claude-desktop"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Client {
    ClaudeCode,
    Codex,
    McpInspector,
    ClaudeDesktop,
}

impl Client {
    fn slug(self) -> &'static str {
        match self {
            Client::ClaudeCode => CLIENTS[0],
            Client::Codex => CLIENTS[1],
            Client::McpInspector => CLIENTS[2],
            Client::ClaudeDesktop => CLIENTS[3],
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Transport {
    Http,
    Stdio,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RecordedBy {
    Operator,
    Dispatcher,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Interactive,
    Headless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Pass,
    Fail,
}

/// A required key whose value may be `null`: `deserialize_with` makes serde
/// report a missing key instead of defaulting an `Option` to `None`.
fn nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

/// An optional key that, when present, must be a string (`null` refuses).
fn present_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    status: Status,
    #[serde(default, deserialize_with = "present_string")]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Steps {
    initialize: Outcome,
    list_tools: Outcome,
    read: Outcome,
    wait_for_code: Outcome,
    structured_content: Outcome,
    list_attachments: Outcome,
}

impl Steps {
    fn in_order(&self) -> [(&'static str, &Outcome); 6] {
        [
            (STEPS[0], &self.initialize),
            (STEPS[1], &self.list_tools),
            (STEPS[2], &self.read),
            (STEPS[3], &self.wait_for_code),
            (STEPS[4], &self.structured_content),
            (STEPS[5], &self.list_attachments),
        ]
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // transport, recorded_by and mode are checked by decoding
struct Receipt {
    date: String,
    client: Client,
    client_version: String,
    os: String,
    swarmail_commit: String,
    executable_sha256: String,
    transport: Transport,
    #[serde(deserialize_with = "nullable")]
    negotiated_revision: Option<String>,
    #[serde(deserialize_with = "nullable")]
    initialize_response: Option<Value>,
    steps: Steps,
    recorded_by: RecordedBy,
    mode: Mode,
}

fn blank(text: &str) -> bool {
    text.trim().is_empty()
}

fn lower_hex(text: &str, digits: usize) -> bool {
    text.len() == digits && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A canonical, real calendar date: it parses and prints back unchanged.
fn calendar(text: &str) -> bool {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .is_ok_and(|date| date.format("%Y-%m-%d").to_string() == text)
}

/// `YYYY-MM-DD-<client>.json` → (date, client slug).
fn split_name(name: &str) -> Result<(&str, &str), String> {
    let shape = || {
        format!(
            "not a receipt name: expected YYYY-MM-DD-<client>.json with <client> one of {CLIENTS:?}"
        )
    };
    let stem = name.strip_suffix(".json").ok_or_else(shape)?;
    let (date, client) = match (stem.get(..10), stem.get(10..11), stem.get(11..)) {
        (Some(date), Some("-"), Some(client)) => (date, client),
        _ => return Err(shape()),
    };
    if !CLIENTS.contains(&client) {
        return Err(shape());
    }
    if !calendar(date) {
        return Err(format!(
            "file name date {date:?} is not a real YYYY-MM-DD date"
        ));
    }
    Ok((date, client))
}

/// The revision the captured reply carries, after checking it is a
/// JSON-RPC 2.0 response object.
fn observed_revision(response: &Value) -> Result<Option<&str>, String> {
    let object = response
        .as_object()
        .ok_or("initialize_response: not a JSON object")?;
    if object.get("jsonrpc") != Some(&json!("2.0")) {
        return Err("initialize_response: jsonrpc is not \"2.0\"".into());
    }
    if !object.contains_key("id") {
        return Err("initialize_response: missing id".into());
    }
    match (object.get("result"), object.get("error")) {
        (Some(result), None) => {
            let result = result
                .as_object()
                .ok_or("initialize_response.result: not a JSON object")?;
            Ok(result.get("protocolVersion").and_then(Value::as_str))
        }
        (None, Some(_)) => Ok(None),
        _ => Err("initialize_response: needs exactly one of result and error".into()),
    }
}

/// serde also decodes a struct from a JSON array of its fields in order;
/// a receipt, its `steps` and each outcome must be objects with named keys.
fn objects_only(bytes: &[u8]) -> Result<(), String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if !value.is_object() {
        return Err("the receipt is not a JSON object".into());
    }
    if !value["steps"].is_object() {
        return Err("steps: not a JSON object".into());
    }
    for step in STEPS {
        if !value["steps"][step].is_object() {
            return Err(format!("steps.{step}: not a JSON object"));
        }
    }
    Ok(())
}

fn check(receipt: &Receipt, name: &str) -> Result<(), String> {
    let (date, client) = split_name(name)?;
    if !calendar(&receipt.date) {
        return Err(format!(
            "date {:?} is not a real YYYY-MM-DD date",
            receipt.date
        ));
    }
    if receipt.date != date {
        return Err(format!(
            "date {:?} does not match the file name",
            receipt.date
        ));
    }
    if receipt.client.slug() != client {
        return Err(format!(
            "client {:?} does not match the file name",
            receipt.client.slug()
        ));
    }
    for (field, value) in [
        ("client_version", &receipt.client_version),
        ("os", &receipt.os),
    ] {
        if blank(value) {
            return Err(format!("{field}: blank"));
        }
    }
    if !lower_hex(&receipt.swarmail_commit, 40) {
        return Err("swarmail_commit: not 40 lowercase hex digits".into());
    }
    if !lower_hex(&receipt.executable_sha256, 64) {
        return Err("executable_sha256: not 64 lowercase hex digits".into());
    }
    for (step, outcome) in receipt.steps.in_order() {
        match (outcome.status, &outcome.reason) {
            (Status::Pass, None) => {}
            (Status::Pass, Some(_)) => return Err(format!("steps.{step}: a pass has no reason")),
            (Status::Fail, Some(reason)) if !blank(reason) => {}
            (Status::Fail, _) => {
                return Err(format!("steps.{step}: a fail needs a nonblank reason"));
            }
        }
    }
    let observed = match &receipt.initialize_response {
        None => None,
        Some(response) => observed_revision(response)?,
    };
    if receipt.negotiated_revision.as_deref() != observed {
        return Err(format!(
            "negotiated_revision {:?} does not match the captured result.protocolVersion {observed:?}",
            receipt.negotiated_revision
        ));
    }
    if receipt.steps.initialize.status == Status::Pass {
        let response = receipt
            .initialize_response
            .as_ref()
            .ok_or("steps.initialize: a pass needs the captured initialize_response")?;
        if response.get("result").is_none() {
            return Err("steps.initialize: a pass needs a success response with result".into());
        }
        if response["id"].is_null() {
            return Err("steps.initialize: a pass needs a non-null response id".into());
        }
        if observed.is_none_or(blank) {
            return Err("steps.initialize: a pass needs a nonblank result.protocolVersion".into());
        }
    } else if let Some((step, _)) = receipt
        .steps
        .in_order()
        .into_iter()
        .find(|(_, outcome)| outcome.status == Status::Pass)
    {
        return Err(format!(
            "steps.{step}: cannot pass after initialize failed (no usable session)"
        ));
    }
    Ok(())
}

/// Read every receipt of `dir`. `Ok` carries the status line; every error
/// names the offending path and the field or rule.
pub(crate) fn scan(dir: &Path) -> Result<String, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("{}: cannot enumerate: {e}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: cannot enumerate: {e}", dir.display()))?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| format!("{}: a file name is not UTF-8: {name:?}", dir.display()))?
            .to_owned();
        if name != README {
            names.push(name);
        }
    }
    names.sort();
    for name in &names {
        let path = dir.join(name);
        let at = |rule: String| format!("{}: {rule}", path.display());
        let meta = std::fs::symlink_metadata(&path).map_err(|e| at(format!("cannot stat: {e}")))?;
        if !meta.is_file() {
            return Err(at("not a regular file".into()));
        }
        split_name(name).map_err(at)?;
        let bytes = std::fs::read(&path).map_err(|e| at(format!("cannot read: {e}")))?;
        std::str::from_utf8(&bytes).map_err(|e| at(format!("not UTF-8: {e}")))?;
        let receipt: Receipt = serde_json::from_slice(&bytes).map_err(|e| at(e.to_string()))?;
        objects_only(&bytes).map_err(at)?;
        check(&receipt, name).map_err(at)?;
    }
    Ok(match names.len() {
        0 => EMPTY.to_owned(),
        n => format!("{n} receipt(s) well formed; file shape only, no compatibility verdict"),
    })
}

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
fn client_matrix_checked_in_receipts_are_well_formed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join(RECEIPTS);
    assert!(
        dir.join(README).is_file(),
        "{RECEIPTS}/README.md is tracked so a path typo cannot pass as empty"
    );
    let status = scan(&dir).unwrap_or_else(|e| panic!("{e}"));
    println!("client matrix: {status}");
    assert!(status == EMPTY || status.ends_with("no compatibility verdict"));
}

#[test]
fn client_matrix_an_empty_directory_passes_with_no_receipts_yet() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(scan(dir.path()).unwrap(), EMPTY);
    std::fs::write(dir.path().join(README), "# receipts\n").unwrap();
    assert_eq!(scan(dir.path()).unwrap(), EMPTY);
}

#[test]
fn client_matrix_a_missing_directory_or_a_foreign_entry_is_refused() {
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
fn client_matrix_a_symlinked_receipt_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.json");
    std::fs::write(&target, serde_json::to_vec(&modern_pass()).unwrap()).unwrap();
    let receipts = dir.path().join("receipts");
    std::fs::create_dir(&receipts).unwrap();
    std::os::unix::fs::symlink(&target, receipts.join(NAME)).unwrap();
    refused(scan(&receipts), "not a regular file");
}

#[test]
fn client_matrix_the_four_evidence_states_are_accepted() {
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
fn client_matrix_removing_any_required_field_is_refused() {
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
fn client_matrix_unknown_and_duplicate_fields_are_refused() {
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
fn client_matrix_wrong_types_enums_and_blanks_are_refused() {
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
    const ORDER: [&str; 12] = [
        "date",
        "client",
        "client_version",
        "os",
        "swarmail_commit",
        "executable_sha256",
        "transport",
        "negotiated_revision",
        "initialize_response",
        "steps",
        "recorded_by",
        "mode",
    ];
    let in_order = Value::Array(ORDER.iter().map(|key| pass[key].clone()).collect());
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
fn client_matrix_bad_hashes_dates_and_names_are_refused() {
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
fn client_matrix_contradictory_handshake_evidence_is_refused() {
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

// ---------------------------------------------------------------- the docs

fn doc(path: &str) -> String {
    super::squash(&super::raw(path))
}

/// Relative link destinations of `text` resolve to files beside `page`.
fn dangling(page: &str, text: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = root.join(page).parent().unwrap().to_owned();
    text.split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .filter(|target| !target.starts_with("http"))
        .map(|target| target.split('#').next().unwrap())
        .filter(|path| !path.is_empty() && !base.join(path).exists())
        .map(str::to_owned)
        .collect()
}

#[test]
fn client_matrix_the_guide_names_every_route_step_tool_and_field() {
    let guide = doc(MATRIX);
    for heading in [
        "### Claude Code",
        "### Codex",
        "### MCP Inspector",
        "### Claude Desktop",
    ] {
        assert!(guide.contains(heading), "{MATRIX} lacks {heading}");
    }
    for needle in [
        "(setup.md#1-start-the-server)",
        "(setup.md#2-claude-code)",
        "(setup.md#3-codex)",
        "(setup.md#where-mail-lands)",
        "headless (`claude -p`)",
        "headless (`codex exec`)",
        "This route needs a person.",
        "`482913`",
        "`matrix.txt`",
        "decoded size `13`",
        "stored as <id>",
        "\"timeout_ms\": 0}`",
        "structuredContent",
        "**Status, 2026-11-22: no receipts yet.**",
        "never says that a client is compatible",
    ] {
        assert!(guide.contains(needle), "{MATRIX} lacks {needle:?}");
    }
    for step in STEPS {
        assert!(
            guide.contains(&format!("| `{step}` |")),
            "{MATRIX} has no row for {step}"
        );
    }
    for tool in TOOLS {
        assert!(guide.contains(tool), "{MATRIX} never names {tool}");
    }
    for client in CLIENTS {
        assert!(
            guide.contains(&format!("`{client}`")),
            "{MATRIX} never names {client}"
        );
    }
    for field in modern_pass().as_object().unwrap().keys() {
        assert!(
            guide.contains(&format!("| `{field}` |")),
            "{MATRIX} has no row for {field}"
        );
    }
    let readme = format!("{RECEIPTS}/{README}");
    for page in [MATRIX, readme.as_str()] {
        assert_eq!(
            dangling(page, &super::raw(page)),
            Vec::<String>::new(),
            "{page}"
        );
    }
}

#[test]
fn client_matrix_the_guides_example_receipt_is_well_formed() {
    let raw = super::raw(MATRIX);
    let examples: Vec<&str> = raw
        .split("\n```json\n")
        .skip(1)
        .map(|rest| rest.split("\n```\n").next().unwrap())
        .collect();
    assert_eq!(examples.len(), 1, "{MATRIX} has exactly one json fence");
    let name = "2026-11-25-claude-code.json";
    assert!(raw.contains(&format!("file name `{name}`")));
    let status = verdict(name, examples[0].as_bytes()).unwrap_or_else(|e| panic!("{e}"));
    assert!(status.starts_with("1 receipt(s)"));
}

#[test]
fn client_matrix_every_status_page_links_the_matrix_and_dates_the_empty_state() {
    for (page, link) in [
        ("docs/agents/setup.md", "](client-matrix.md)"),
        ("docs/guide/mcp.md", "](../agents/client-matrix.md)"),
        ("docs/evals/README.md", "](../agents/client-matrix.md)"),
        (
            "specs/mcp-version-negotiation/implementation.md",
            "](../../docs/agents/client-matrix.md)",
        ),
        (
            "docs/evals/clients/README.md",
            "](../../agents/client-matrix.md)",
        ),
    ] {
        let text = doc(page);
        assert!(
            text.contains(link),
            "{page} does not link the matrix with {link}"
        );
        assert!(
            text.contains("2026-11-22"),
            "{page} carries no dated status"
        );
        assert!(
            text.contains("no receipts yet") || text.contains("no receipt exists"),
            "{page}"
        );
        assert!(
            !text.contains("Closes #392") && !text.contains("Fixes #392"),
            "{page}"
        );
    }
}
