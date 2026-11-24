//! The MCP client matrix's receipt contract (issue #392, epic #164): one
//! typed reader for `docs/evals/clients/`, its drafts and the handbook's
//! example. The evals harness owns it; the root `agent_setup_docs` target
//! compiles this same file by path, so there is exactly one checker.
//!
//! The reader checks file shape only. A well-formed receipt, a passing
//! test, a fake client or an empty directory is never a compatibility
//! verdict (decisions 0079 and 0080): that verdict is the operator's, read
//! from receipts recorded through the named clients.
//!
//! Receipt-owned JSON is decoded straight from the bytes into strict types,
//! so serde refuses a duplicate key instead of a map keeping the last one.
//! The captured `initialize` reply is protocol-owned and stays a `Value`.
//! This file names only `serde`, `serde_json`, `chrono` and `std`: both
//! crates that compile it already depend on each.

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// The checklist and the receipt format.
pub const MATRIX: &str = "docs/agents/client-matrix.md";
/// Where receipts are committed, from the repository root.
pub const RECEIPTS: &str = "docs/evals/clients";
/// The one file the scan skips.
pub const README: &str = "README.md";
/// The status of a successfully read, empty directory.
pub const EMPTY: &str = "no receipts yet";

/// The six checklist steps, in checklist order.
pub const STEPS: [&str; 6] = [
    "initialize",
    "list_tools",
    "read",
    "wait_for_code",
    "structured_content",
    "list_attachments",
];
/// The four tools the checklist calls.
pub const TOOLS: [&str; 4] = [
    "swarmail_get_latest_email",
    "swarmail_wait_for_code",
    "swarmail_search_emails",
    "swarmail_list_attachments",
];
/// The four client slugs, in the guide's order.
pub const CLIENTS: [&str; 4] = ["claude-code", "codex", "mcp-inspector", "claude-desktop"];
/// The twelve receipt fields, in the handbook's order.
pub const FIELDS: [&str; 12] = [
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

/// The checklist's synthetic fixture message, exactly as the handbook's
/// fenced block shows it (LF line ends, no indentation). The root docs
/// target checks the handbook against it; the runner delivers it.
pub const FIXTURE: &str = "\
From: Matrix Sender <sender@matrix.test>
To: operator@matrix.test
Subject: Swarmail client matrix fixture
MIME-Version: 1.0
Content-Type: multipart/mixed; boundary=\"matrix-boundary\"

--matrix-boundary
Content-Type: text/plain; charset=utf-8

Your verification code is 482913.

--matrix-boundary
Content-Type: text/plain; name=\"matrix.txt\"
Content-Disposition: attachment; filename=\"matrix.txt\"
Content-Transfer-Encoding: base64

aGVsbG8gbWF0cml4Cg==
--matrix-boundary--
";

/// The fenced fixture block that follows `` `matrix-fixture.eml`: `` in
/// `text` (the handbook), unindented, with LF line ends.
pub fn fenced_fixture(text: &str) -> Result<String, String> {
    let missing = || "the checklist's fixture block is missing".to_string();
    let after = text
        .split_once("`matrix-fixture.eml`:")
        .ok_or_else(missing)?
        .1;
    let fence = after.split_once("```text\n").ok_or_else(missing)?.1;
    let mut message = String::new();
    for line in fence.lines() {
        let line = line.strip_prefix("   ").unwrap_or(line);
        if line.starts_with("```") {
            return Ok(message);
        }
        message.push_str(line);
        message.push('\n');
    }
    Err(missing())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Client {
    ClaudeCode,
    Codex,
    McpInspector,
    ClaudeDesktop,
}

impl Client {
    pub fn slug(self) -> &'static str {
        match self {
            Client::ClaudeCode => CLIENTS[0],
            Client::Codex => CLIENTS[1],
            Client::McpInspector => CLIENTS[2],
            Client::ClaudeDesktop => CLIENTS[3],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Http,
    Stdio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordedBy {
    Operator,
    Dispatcher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Interactive,
    Headless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub status: Status,
    #[serde(
        default,
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub reason: Option<String>,
}

impl Outcome {
    pub fn pass() -> Self {
        Outcome {
            status: Status::Pass,
            reason: None,
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Outcome {
            status: Status::Fail,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    pub initialize: Outcome,
    pub list_tools: Outcome,
    pub read: Outcome,
    pub wait_for_code: Outcome,
    pub structured_content: Outcome,
    pub list_attachments: Outcome,
}

impl Steps {
    pub fn in_order(&self) -> [(&'static str, &Outcome); 6] {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub date: String,
    pub client: Client,
    pub client_version: String,
    pub os: String,
    pub swarmail_commit: String,
    pub executable_sha256: String,
    pub transport: Transport,
    #[serde(deserialize_with = "nullable")]
    pub negotiated_revision: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub initialize_response: Option<Value>,
    pub steps: Steps,
    pub recorded_by: RecordedBy,
    pub mode: Mode,
}

impl Receipt {
    /// The file name this receipt must carry: `YYYY-MM-DD-<client>.json`.
    pub fn file_name(&self) -> String {
        format!("{}-{}.json", self.date, self.client.slug())
    }
}

fn blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// `digits` lowercase hex digits, exactly.
pub fn lower_hex(text: &str, digits: usize) -> bool {
    text.len() == digits && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A canonical, real calendar date: it parses and prints back unchanged.
fn calendar(text: &str) -> bool {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .is_ok_and(|date| date.format("%Y-%m-%d").to_string() == text)
}

/// `YYYY-MM-DD-<client>.json` → (date, client slug).
pub fn split_name(name: &str) -> Result<(&str, &str), String> {
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

/// Validate one receipt's bytes against the file name it is (or will be)
/// stored under. Every error names the field or the rule.
pub fn validate(bytes: &[u8], name: &str) -> Result<Receipt, String> {
    split_name(name)?;
    std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    let receipt: Receipt = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    objects_only(bytes)?;
    check(&receipt, name)?;
    Ok(receipt)
}

/// Validate one receipt file: a regular file (never a link) whose basename
/// is the receipt name. Errors name the path.
pub fn validate_file(path: &Path) -> Result<Receipt, String> {
    let at = |rule: String| format!("{}: {rule}", path.display());
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| at("no UTF-8 file name".into()))?;
    let meta = std::fs::symlink_metadata(path).map_err(|e| at(format!("cannot stat: {e}")))?;
    if !meta.is_file() {
        return Err(at("not a regular file".into()));
    }
    split_name(name).map_err(at)?;
    let bytes = std::fs::read(path).map_err(|e| at(format!("cannot read: {e}")))?;
    validate(&bytes, name).map_err(at)
}

/// Read every receipt of `dir`. `Ok` carries the status line; every error
/// names the offending path and the field or rule.
pub fn scan(dir: &Path) -> Result<String, String> {
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
        validate_file(&dir.join(name))?;
    }
    Ok(match names.len() {
        0 => EMPTY.to_owned(),
        n => format!("{n} receipt(s) well formed; file shape only, no compatibility verdict"),
    })
}
